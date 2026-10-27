use crate::{ReContext, ReModule, ReRequest, ReSection, digest, entropy, framed_button};

/// Le vidage hexadécimal du fichier, et l'empreinte de ce qu'on y
/// sélectionne.
///
/// L'onglet Mémoire montre la mémoire **du programme qui tourne** ;
/// celui-ci montre le **fichier**, tel qu'il est sur le disque. Les
/// deux diffèrent plus qu'on ne croit : ce qui n'est pas chargé n'a pas
/// d'adresse, ce qui est relogé n'a plus la même valeur, et un binaire
/// empaqueté ne ressemble à rien une fois déballé.
///
/// Ce qu'on vient y chercher tient en une question : *ces octets-là,
/// qu'est-ce que c'est ?* D'où l'empreinte et l'entropie d'une
/// sélection — un bloc à 7,9 bits par octet est chiffré ou compressé, un
/// bloc à 4,5 est du code, et un SHA-256 se colle dans un compte rendu
/// ou se compare à celui d'un fichier connu.
pub struct DumpModule {
    /// Le début de la sélection, tel qu'on l'a écrit.
    début: String,
    /// Sa longueur, telle qu'on l'a écrite.
    longueur: String,
    /// Où le vidage doit se rendre à la prochaine image, s'il le doit.
    aller_à: Option<usize>,
    /// Les octets qu'on veut écrire à la place de la tranche, tels qu'on
    /// les a tapés.
    remplacement: String,
    /// Ce qu'on demande à l'application, relevé à l'image suivante.
    demande: Option<ReRequest>,
}

impl Default for DumpModule {
    fn default() -> Self {
        Self {
            début: "0".to_owned(),
            // Seize octets : une ligne du vidage. C'est la sélection
            // qu'on refait le plus souvent, et elle donne une réponse
            // tout de suite plutôt qu'un champ vide.
            longueur: "0x10".to_owned(),
            aller_à: None,
            remplacement: String::new(),
            demande: None,
        }
    }
}

/// Combien d'octets par ligne. Seize, comme tous les vidages depuis
/// quarante ans : c'est ce qui fait qu'une adresse ronde tombe en début
/// de ligne.
const PAR_LIGNE: usize = 16;

/// Le vidage en texte, tel qu'on l'enregistre ou qu'on le colle.
///
/// La même forme que celle de l'écran — décalage, seize octets, puis la
/// colonne de texte —, parce qu'un vidage qu'on enregistre sert à être
/// comparé à celui qu'on a sous les yeux. C'est aussi la forme que
/// `xxd` et `hexdump -C` donnent, donc celle qu'un `diff` sait lire.
///
/// Une seule fonction pour l'écran et pour le fichier : deux qui
/// formateraient chacune finiraient par ne plus donner le même texte, et
/// la comparaison ne dirait plus rien.
pub fn hex_dump(octets: &[u8]) -> String {
    let mut out = String::with_capacity(octets.len() * 4);
    for (ligne, tranche) in octets.chunks(PAR_LIGNE).enumerate() {
        out.push_str(&format!("{:08x}  ", ligne * PAR_LIGNE));
        for (rang, octet) in tranche.iter().enumerate() {
            out.push_str(&format!("{octet:02x} "));
            if rang == PAR_LIGNE / 2 - 1 {
                out.push(' ');
            }
        }
        // Les colonnes restent alignées sur la dernière ligne, qui est
        // souvent incomplète : sans ce remplissage, la colonne de texte
        // remonterait de quelques caractères et le fichier ne se
        // comparerait plus au précédent.
        for rang in tranche.len()..PAR_LIGNE {
            out.push_str("   ");
            if rang == PAR_LIGNE / 2 - 1 {
                out.push(' ');
            }
        }
        out.push_str(&format!(" |{}|\n", lisible(tranche, PAR_LIGNE)));
    }
    out
}

/// Lit un nombre écrit à la main, dans la base que son préfixe annonce.
///
/// `0x` hexadécimal, sinon décimal. Rendre `None` sur ce qui n'en est
/// pas un plutôt que zéro : un champ qui répond « 0 » à une faute de
/// frappe fait croire qu'on regarde le début du fichier.
fn nombre(texte: &str) -> Option<usize> {
    let texte = texte.trim();
    if let Some(hexa) = texte
        .strip_prefix("0x")
        .or_else(|| texte.strip_prefix("0X"))
    {
        usize::from_str_radix(hexa, 16).ok()
    } else {
        texte.parse().ok()
    }
}

/// La tranche que ces deux champs désignent dans ce fichier.
///
/// Bornée au fichier : une longueur trop grande donne ce qui reste, et
/// non une erreur — on tape « tout le reste » en écrivant un grand
/// nombre, et refuser serait pédant. Un début hors du fichier, en
/// revanche, ne désigne rien.
pub fn tranche(taille_du_fichier: usize, début: usize, longueur: usize) -> Option<(usize, usize)> {
    if début >= taille_du_fichier || longueur == 0 {
        return None;
    }
    let fin = début.saturating_add(longueur).min(taille_du_fichier);
    Some((début, fin))
}

impl ReModule for DumpModule {
    fn section(&self) -> ReSection {
        ReSection::Bytes
    }

    fn id(&self) -> &'static str {
        "dump"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Vidage du fichier"
        } else {
            "File dump"
        }
    }

    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let octets = ctx.binary;
        if octets.is_empty() {
            ui.weak(ctx.t(
                "Rien de chargé : il n'y a pas de fichier à vider.",
                "Nothing loaded: there is no file to dump.",
            ));
            return;
        }

        // La sélection, écrite à la main ou posée d'un clic.
        let mut aller = false;
        ui.horizontal_wrapped(|ui| {
            ui.weak(ctx.t("De", "From"));
            ui.add(
                egui::TextEdit::singleline(&mut self.début)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(90.0),
            );
            ui.weak(ctx.t("longueur", "length"));
            ui.add(
                egui::TextEdit::singleline(&mut self.longueur)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(90.0),
            );
            aller |= framed_button(ui, ctx.t("Aller", "Go"))
                .on_hover_text(ctx.t(
                    "Amener le vidage sur cette tranche",
                    "Bring the dump to this range",
                ))
                .clicked();
            if framed_button(ui, ctx.t("Tout", "Everything")).clicked() {
                self.début = "0".to_owned();
                self.longueur = format!("{:#x}", octets.len());
            }
            ui.separator();
            // Le vidage entier dans le presse-papier : on le colle dans
            // un compte rendu, ou on le compare à celui d'un autre
            // outil. C'est la forme de `xxd`, donc celle qu'un `diff`
            // sait lire.
            if framed_button(ui, ctx.t("Copier le vidage", "Copy the dump"))
                .on_hover_text(ctx.t(
                    "Le fichier entier, au format de xxd",
                    "The whole file, in xxd's format",
                ))
                .clicked()
            {
                ui.ctx().copy_text(hex_dump(octets));
            }
        });

        let choisie = nombre(&self.début)
            .zip(nombre(&self.longueur))
            .and_then(|(d, l)| tranche(octets.len(), d, l));
        match choisie {
            Some((début, fin)) => {
                if aller {
                    self.aller_à = Some(début);
                }
                let morceau = &octets[début..fin];
                ui.add_space(4.0);
                egui::Grid::new("dump_selection")
                    .num_columns(2)
                    .min_col_width(0.0)
                    .striped(true)
                    .spacing([14.0, 3.0])
                    .show(ui, |ui| {
                        ui.weak(ctx.t("Tranche", "Range"));
                        ui.monospace(format!(
                            "{:#x} … {:#x} ({} {})",
                            début,
                            fin,
                            morceau.len(),
                            ctx.t("octets", "bytes")
                        ));
                        ui.end_row();

                        ui.weak("SHA-256");
                        let empreinte = digest::hex(&digest::sha256(morceau));
                        if crate::clickable_label(
                            ui,
                            egui::Label::new(
                                egui::RichText::new(&empreinte).monospace().size(11.0),
                            ),
                        )
                        .on_hover_text(ctx.t("Cliquer pour copier", "Click to copy"))
                        .clicked()
                        {
                            ui.ctx().copy_text(empreinte);
                        }
                        ui.end_row();

                        ui.weak("CRC-32");
                        ui.monospace(format!("{:08x}", digest::crc32(morceau)));
                        ui.end_row();

                        ui.weak(ctx.t("Entropie", "Entropy"));
                        let valeur = entropy::shannon(morceau);
                        ui.monospace(format!(
                            "{valeur:.2} — {}",
                            entropy::Verdict::of(valeur).label(ctx.lang.starts_with("fr"))
                        ));
                        ui.end_row();

                        ui.weak(ctx.t("Texte", "Text"));
                        ui.monospace(lisible(morceau, 48));
                        ui.end_row();
                    });

                // Et l'écriture. Le vidage montrait ; il laisse
                // maintenant changer, ce qui est la suite naturelle : on
                // a repéré la constante qu'on veut voir autrement, et
                // l'on n'avait jusqu'ici aucun moyen de la poser.
                //
                // Autant d'octets que la tranche en compte, exactement.
                // C'est la même règle que pour l'inversion d'un saut :
                // un octet de plus ou de moins décalerait tout ce qui
                // suit, et chaque adresse du fichier deviendrait fausse.
                ui.add_space(6.0);
                let attendus = morceau.len();
                let écrits = octets_écrits(&self.remplacement);
                ui.horizontal(|ui| {
                    ui.weak(ctx.t("Remplacer par", "Replace with"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.remplacement)
                            .hint_text(ctx.t("41 42 43…", "41 42 43…"))
                            .font(egui::TextStyle::Monospace)
                            .desired_width(220.0),
                    );
                    let prêt = écrits.as_ref().is_some_and(|o| o.len() == attendus);
                    ui.add_enabled_ui(prêt, |ui| {
                        if framed_button(ui, ctx.t("Écrire", "Write"))
                            .on_hover_text(ctx.t(
                                "Le changement se défait depuis Outils > Patcher le binaire.",
                                "The change can be undone from Tools > Patch the binary.",
                            ))
                            .clicked()
                            && let Some(bytes) = écrits.clone()
                        {
                            self.demande = Some(ReRequest::PatchBytes {
                                offset: début,
                                // Autant retirés qu'écrits : le vidage
                                // du panneau latéral remplace sur
                                // place, et ne déplace jamais rien.
                                retirés: bytes.len(),
                                bytes,
                            });
                            self.remplacement.clear();
                        }
                    });
                });
                match &écrits {
                    _ if self.remplacement.trim().is_empty() => {}
                    None => {
                        ui.weak(ctx.t(
                            "Des octets en hexadécimal, par paires : « 90 90 » ou « 9090 ».",
                            "Bytes in hexadecimal, in pairs: \"90 90\" or \"9090\".",
                        ));
                    }
                    Some(o) if o.len() != attendus => {
                        ui.weak(format!(
                            "{} {attendus} {} {} {}",
                            ctx.t("La tranche en compte", "The range holds"),
                            ctx.t("octets", "bytes"),
                            ctx.t("; vous en donnez", "; you give"),
                            o.len()
                        ));
                    }
                    Some(_) => {}
                }
            }
            None => {
                ui.add_space(4.0);
                ui.weak(ctx.t(
                    "Cette tranche ne désigne rien : un début dans le fichier, une longueur \
                     non nulle. Cliquez un octet du vidage pour poser le début.",
                    "That range points at nothing: a start inside the file, a non-zero \
                     length. Click a byte in the dump to set the start.",
                ));
            }
        }

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        // Le vidage lui-même, une ligne à la fois et seulement celles
        // qu'on voit : un fichier de trois cents kilo-octets fait vingt
        // mille lignes, et les disposer toutes coûterait une seconde
        // par image.
        let hauteur = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
        let lignes = octets.len().div_ceil(PAR_LIGNE);
        let mut zone = egui::ScrollArea::vertical()
            .id_salt("dump_rows")
            .auto_shrink([false, false])
            // Ce que la place offre, et non un chiffre — même défaut
            // que la liste des chaînes : trois cent vingt pixels en dur
            // laissaient le reste de la fenêtre vide. Le plancher garde
            // l'ancien comportement là où la place manque.
            .max_height(ui.available_height().max(320.0));
        if let Some(cible) = self.aller_à.take() {
            zone = zone.vertical_scroll_offset((cible / PAR_LIGNE) as f32 * hauteur);
        }
        let mut cliqué: Option<usize> = None;
        zone.show_rows(ui, hauteur, lignes, |ui, plage| {
            for ligne in plage {
                let base = ligne * PAR_LIGNE;
                let fin = (base + PAR_LIGNE).min(octets.len());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.monospace(
                        egui::RichText::new(format!("{base:08x}"))
                            .color(ui.visuals().weak_text_color()),
                    );
                    for (rang, octet) in octets[base..fin].iter().enumerate() {
                        let position = base + rang;
                        let dans = choisie.is_some_and(|(d, f)| position >= d && position < f);
                        let texte = egui::RichText::new(format!("{octet:02x}")).monospace();
                        let texte = if dans {
                            texte.background_color(
                                ui.visuals().selection.bg_fill.gamma_multiply(0.55),
                            )
                        } else {
                            texte
                        };
                        if crate::clickable_label(ui, egui::Label::new(texte)).clicked() {
                            cliqué = Some(position);
                        }
                        // Un blanc au milieu : c'est ce qui fait qu'on
                        // compte les octets par huit sans les compter.
                        if rang == PAR_LIGNE / 2 - 1 {
                            ui.add_space(6.0);
                        }
                    }
                    ui.add_space(8.0);
                    ui.monospace(lisible(&octets[base..fin], PAR_LIGNE));
                });
            }
        });
        if let Some(position) = cliqué {
            self.début = format!("{position:#x}");
        }
    }
}

/// Les octets qu'on a tapés, ou `None` si ce n'en sont pas.
///
/// Par paires, espacées ou collées : « 90 90 » et « 9090 » donnent la
/// même chose. Un nombre impair de chiffres est refusé plutôt que
/// complété — un octet fait deux chiffres, et deviner le second
/// reviendrait à écrire autre chose que ce qu'on a demandé.
fn octets_écrits(texte: &str) -> Option<Vec<u8>> {
    let chiffres: String = texte.chars().filter(|c| !c.is_whitespace()).collect();
    if chiffres.is_empty() || !chiffres.len().is_multiple_of(2) {
        return None;
    }
    let paires: Vec<char> = chiffres.chars().collect();
    paires
        .chunks(2)
        .map(|paire| {
            let haut = paire[0].to_digit(16)?;
            let bas = paire[1].to_digit(16)?;
            Some((haut * 16 + bas) as u8)
        })
        .collect()
}

/// Les octets lus comme du texte, le reste en points — comme dans tout
/// vidage hexadécimal depuis quarante ans.
fn lisible(octets: &[u8], combien: usize) -> String {
    octets
        .iter()
        .take(combien)
        .map(|o| {
            if o.is_ascii_graphic() || *o == b' ' {
                *o as char
            } else {
                '.'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un nombre se lit dans la base que son préfixe annonce, et ce qui
    /// n'en est pas un ne devient pas zéro.
    ///
    /// Un champ qui répond « 0 » à une faute de frappe fait croire qu'on
    /// regarde le début du fichier, ce qui est le plus sûr moyen de
    /// conclure de travers.
    #[test]
    fn a_number_reads_in_the_base_its_prefix_announces() {
        assert_eq!(nombre("0x10"), Some(16));
        assert_eq!(nombre("0X10"), Some(16));
        assert_eq!(nombre("16"), Some(16));
        assert_eq!(nombre("  0x1f  "), Some(31));
        assert_eq!(nombre(""), None);
        assert_eq!(nombre("zz"), None);
        assert_eq!(nombre("0xzz"), None);
    }

    /// La tranche est bornée au fichier, et un début dehors ne désigne
    /// rien.
    #[test]
    fn a_range_is_bounded_by_the_file() {
        // Une longueur trop grande donne ce qui reste : on tape « tout
        // le reste » en écrivant un grand nombre, et refuser serait
        // pédant.
        assert_eq!(tranche(100, 90, 1000), Some((90, 100)));
        assert_eq!(tranche(100, 0, 16), Some((0, 16)));
        // Un début hors du fichier, en revanche, ne désigne rien.
        assert_eq!(tranche(100, 100, 16), None);
        assert_eq!(tranche(100, 200, 16), None);
        // Et une longueur nulle non plus : l'empreinte du vide
        // ressemble à une vraie.
        assert_eq!(tranche(100, 0, 0), None);
        assert_eq!(tranche(0, 0, 16), None);
        // Le débordement ne fait rien tomber.
        assert_eq!(tranche(100, 1, usize::MAX), Some((1, 100)));
    }

    /// Le vidage en texte a la forme de `xxd`, colonnes comprises.
    ///
    /// C'est ce qui permet de le comparer : un vidage enregistré sert à
    /// être mis en regard de celui qu'on a sous les yeux, ou de celui
    /// d'un autre outil.
    #[test]
    fn the_text_dump_has_the_shape_one_can_diff() {
        let texte = hex_dump(b"ELF\x01\x02");
        let lignes: Vec<&str> = texte.lines().collect();
        assert_eq!(lignes.len(), 1, "cinq octets tiennent sur une ligne");
        assert!(
            lignes[0].starts_with("00000000  45 4c 46 01 02"),
            "{}",
            lignes[0]
        );
        assert!(lignes[0].ends_with("|ELF..|"), "{}", lignes[0]);

        // Deux lignes pleines, et les décalages qui vont avec.
        let texte = hex_dump(&[0x41u8; 20]);
        let lignes: Vec<&str> = texte.lines().collect();
        assert_eq!(lignes.len(), 2);
        assert!(
            lignes[1].starts_with("00000010  41 41 41 41 "),
            "{}",
            lignes[1]
        );
        // La colonne de texte reste à la même abscisse sur une ligne
        // incomplète : sans le remplissage, un `diff` verrait une
        // différence là où il n'y en a pas.
        let barre = |l: &str| l.find('|').expect("la colonne de texte");
        assert_eq!(barre(lignes[0]), barre(lignes[1]), "{lignes:?}");

        assert!(hex_dump(&[]).is_empty(), "rien à vider");
    }

    /// Les octets qu'on tape se lisent par paires, espacées ou collées.
    #[test]
    fn typed_bytes_read_in_pairs() {
        assert_eq!(octets_écrits("90 90"), Some(vec![0x90, 0x90]));
        assert_eq!(octets_écrits("9090"), Some(vec![0x90, 0x90]));
        assert_eq!(octets_écrits("  41  42  "), Some(vec![0x41, 0x42]));
        assert_eq!(octets_écrits("EB"), Some(vec![0xeb]));
        // Un nombre impair de chiffres est refusé plutôt que complété :
        // un octet fait deux chiffres, et deviner le second reviendrait
        // à écrire autre chose que ce qu'on a demandé.
        assert_eq!(octets_écrits("909"), None);
        assert_eq!(octets_écrits("9z"), None);
        assert_eq!(octets_écrits(""), None);
        assert_eq!(octets_écrits("   "), None);
    }

    /// Le texte d'un vidage : ce qui se lit, et des points pour le
    /// reste.
    #[test]
    fn the_text_column_shows_what_reads() {
        assert_eq!(lisible(b"ELF\x01\x02 ok", 16), "ELF.. ok");
        assert_eq!(lisible(b"abcdef", 3), "abc");
        assert_eq!(lisible(b"", 16), "");
    }

    /// L'empreinte porte sur la tranche choisie, non sur le fichier.
    ///
    /// C'est tout l'intérêt : « ces octets-là, qu'est-ce que c'est ? »
    #[test]
    fn the_digest_covers_the_selected_range_only() {
        let fichier = b"aaaabbbbcccc";
        let (début, fin) = tranche(fichier.len(), 4, 4).expect("une tranche");
        assert_eq!(&fichier[début..fin], b"bbbb");
        assert_eq!(
            digest::hex(&digest::sha256(&fichier[début..fin])),
            digest::hex(&digest::sha256(b"bbbb"))
        );
        assert_ne!(
            digest::hex(&digest::sha256(&fichier[début..fin])),
            digest::hex(&digest::sha256(fichier))
        );
    }

    /// Les valeurs de départ répondent tout de suite.
    ///
    /// Seize octets depuis le début : la sélection qu'on refait le plus
    /// souvent, et qui donne une réponse plutôt qu'un champ vide.
    #[test]
    fn the_first_selection_already_says_something() {
        let module = DumpModule::default();
        assert_eq!(nombre(&module.début), Some(0));
        assert_eq!(nombre(&module.longueur), Some(16));
    }
}
