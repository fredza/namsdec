use crate::vue::{Bloc, Champ, Fragment, Ligne, Vue};
use crate::{ReContext, ReModule, ReRequest, ReSection, digest, entropy};

/// Ce que le panneau demande, par son nom.
///
/// `ALLER` porte LE MÊME NOM que le champ du début, et c'est voulu :
/// un bouton qui porte l'action de son champ le valide, comme la
/// touche Entrée. Sans cela, taper une adresse puis cliquer « Aller »
/// aurait perdu ce qu'on vient d'écrire — le champ perd le clavier
/// sans qu'Entrée ait été pressée.
const ACTION_DÉBUT: &str = "vidage:début";
const ACTION_LONGUEUR: &str = "vidage:longueur";
const ACTION_TOUT: &str = "vidage:tout";
const ACTION_COPIER: &str = "vidage:copier";
const ACTION_ÉCRIRE: &str = "vidage:écrire";

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
    /// Le texte à poser dans le presse-papier, quand on l'a demandé.
    ///
    /// Posé par `agit`, consommé par `ui` : copier demande le contexte
    /// egui, que le comportement n'a pas — et c'est tant mieux, car
    /// c'est ce qui permet d'éprouver « copier le vidage » sans
    /// peindre, en lisant ce qui a été mis là.
    à_copier: Option<String>,
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
            à_copier: None,
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

impl DumpModule {
    /// La tranche que les deux champs désignent, s'ils en désignent une.
    fn choisie(&self, octets: &[u8]) -> Option<(usize, usize)> {
        nombre(&self.début)
            .zip(nombre(&self.longueur))
            .and_then(|(d, l)| tranche(octets.len(), d, l))
    }

    /// Ce que la tranche VAUT : son étendue, son empreinte, son
    /// entropie, le texte qu'on y lit — et de quoi la remplacer.
    ///
    /// C'est la part de ce panneau qui est de la compréhension, et
    /// c'est elle qui passe la frontière. La grille d'octets reste
    /// peinte : voir le commentaire de `ui`.
    fn vue_de_la_tranche(&self, ctx: &ReContext, octets: &[u8]) -> Vue {
        // LA SÉLECTION SE VALIDE, ELLE NE SE DEVINE PLUS À CHAQUE
        // LETTRE. Le panneau relisait les deux champs à chaque image,
        // de sorte que la tranche bougeait sous les doigts pendant
        // qu'on tapait « 0x1 », « 0x10 », « 0x100 ». Le tampon de
        // frappe vit maintenant chez celui qui peint, et le module
        // n'apprend ce qui est écrit qu'à la validation — par Entrée,
        // ou par le bouton qui porte la même action.
        let mut blocs = vec![
            Bloc::Champ(
                Champ::pour(ACTION_DÉBUT)
                    .étiquette(Fragment::dit(ctx.t("De", "From")).faible())
                    .valeur(&self.début)
                    .code()
                    .caractères(12)
                    .bouton(
                        Fragment::dit(ctx.t("Aller", "Go"))
                            .bouton(ACTION_DÉBUT)
                            .survol(ctx.t(
                                "Amener le vidage sur cette tranche",
                                "Bring the dump to this range",
                            )),
                    ),
            ),
            Bloc::Champ(
                Champ::pour(ACTION_LONGUEUR)
                    .étiquette(Fragment::dit(ctx.t("longueur", "length")).faible())
                    .valeur(&self.longueur)
                    .code()
                    .caractères(12)
                    .bouton(Fragment::dit(ctx.t("Tout", "Everything")).bouton(ACTION_TOUT))
                    // Le vidage entier dans le presse-papier : on le
                    // colle dans un compte rendu, ou on le compare à
                    // celui d'un autre outil. C'est la forme de `xxd`,
                    // donc celle qu'un `diff` sait lire.
                    .bouton(
                        Fragment::dit(ctx.t("Copier le vidage", "Copy the dump"))
                            .bouton(ACTION_COPIER)
                            .survol(ctx.t(
                                "Le fichier entier, au format de xxd",
                                "The whole file, in xxd's format",
                            )),
                    ),
            ),
        ];

        let Some((début, fin)) = self.choisie(octets) else {
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Cette tranche ne désigne rien : un début dans le fichier, une longueur \
                     non nulle. Cliquez un octet du vidage pour poser le début.",
                    "That range points at nothing: a start inside the file, a non-zero \
                     length. Click a byte in the dump to set the start.",
                ))
                .faible()
                .into(),
            ));
            return Vue { blocs };
        };

        let morceau = &octets[début..fin];
        let nom = |texte: &str| -> Ligne { Fragment::dit(texte).faible().into() };
        let valeur = entropy::shannon(morceau);
        blocs.push(Bloc::Tableau {
            colonnes: Vec::new(),
            rangées: vec![
                vec![
                    nom(ctx.t("Tranche", "Range")),
                    Fragment::dit(format!(
                        "{:#x} … {:#x} ({} {})",
                        début,
                        fin,
                        morceau.len(),
                        ctx.t("octets", "bytes")
                    ))
                    .code()
                    .into(),
                ],
                vec![
                    nom("SHA-256"),
                    // Une empreinte sert à être collée quelque part,
                    // pas à être recopiée à la main.
                    Fragment::dit(digest::hex(&digest::sha256(morceau)))
                        .code()
                        .copiable()
                        .into(),
                ],
                vec![
                    nom("CRC-32"),
                    Fragment::dit(format!("{:08x}", digest::crc32(morceau)))
                        .code()
                        .into(),
                ],
                vec![
                    nom(ctx.t("Entropie", "Entropy")),
                    Fragment::dit(format!(
                        "{valeur:.2} — {}",
                        entropy::Verdict::of(valeur).label(ctx.lang.starts_with("fr"))
                    ))
                    .code()
                    .into(),
                ],
                vec![
                    nom(ctx.t("Texte", "Text")),
                    Fragment::dit(lisible(morceau, 48)).code().into(),
                ],
            ],
            rayé: true,
        });

        // Et l'écriture. Le vidage montrait ; il laisse maintenant
        // changer, ce qui est la suite naturelle : on a repéré la
        // constante qu'on veut voir autrement, et l'on n'avait jusqu'ici
        // aucun moyen de la poser.
        //
        // Autant d'octets que la tranche en compte, exactement. C'est la
        // même règle que pour l'inversion d'un saut : un octet de plus
        // ou de moins décalerait tout ce qui suit, et chaque adresse du
        // fichier deviendrait fausse.
        let attendus = morceau.len();
        let écrits = octets_écrits(&self.remplacement);
        let prêt = écrits.as_ref().is_some_and(|o| o.len() == attendus);
        blocs.push(Bloc::Champ(
            Champ::pour(ACTION_ÉCRIRE)
                .étiquette(Fragment::dit(ctx.t("Remplacer par", "Replace with")).faible())
                .valeur(&self.remplacement)
                .indice("41 42 43…")
                .code()
                .caractères(26)
                .bouton(
                    Fragment::dit(ctx.t("Écrire", "Write"))
                        .bouton(ACTION_ÉCRIRE)
                        .inactif_si(!prêt)
                        .survol(ctx.t(
                            "Le changement se défait depuis Outils > Patcher le binaire.",
                            "The change can be undone from Tools > Patch the binary.",
                        )),
                ),
        ));
        match &écrits {
            _ if self.remplacement.trim().is_empty() => {}
            None => blocs.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Des octets en hexadécimal, par paires : « 90 90 » ou « 9090 ».",
                    "Bytes in hexadecimal, in pairs: \"90 90\" or \"9090\".",
                ))
                .faible()
                .into(),
            )),
            Some(o) if o.len() != attendus => blocs.push(Bloc::Texte(
                Fragment::dit(format!(
                    "{} {attendus} {} {} {}",
                    ctx.t("La tranche en compte", "The range holds"),
                    ctx.t("octets", "bytes"),
                    ctx.t("; vous en donnez", "; you give"),
                    o.len()
                ))
                .faible()
                .into(),
            )),
            Some(_) => {}
        }
        Vue { blocs }
    }

    /// Ce que le panneau fait de ce qu'on lui demande.
    ///
    /// À part du dessin pour être éprouvé sans peindre : poser une
    /// tranche, copier, écrire sont des comportements.
    fn agit(&mut self, action: &str, octets: &[u8]) {
        if let Some(valeur) = action.strip_prefix(&format!("{ACTION_DÉBUT}:")) {
            self.début = valeur.to_owned();
            // Valider le début, c'est demander à voir : « Aller » porte
            // la même action, et Entrée vaut « Aller ».
            if let Some((début, _)) = self.choisie(octets) {
                self.aller_à = Some(début);
            }
            return;
        }
        if let Some(valeur) = action.strip_prefix(&format!("{ACTION_LONGUEUR}:")) {
            self.longueur = valeur.to_owned();
            return;
        }
        if let Some(valeur) = action.strip_prefix(&format!("{ACTION_ÉCRIRE}:")) {
            self.remplacement = valeur.to_owned();
            let Some((début, fin)) = self.choisie(octets) else {
                return;
            };
            let Some(bytes) = octets_écrits(&self.remplacement) else {
                return;
            };
            if bytes.len() != fin - début {
                return;
            }
            self.demande = Some(ReRequest::PatchBytes {
                offset: début,
                // Autant retirés qu'écrits : le vidage du panneau
                // latéral remplace sur place, et ne déplace jamais rien.
                retirés: bytes.len(),
                bytes,
            });
            self.remplacement.clear();
            return;
        }
        match action {
            ACTION_TOUT => {
                self.début = "0".to_owned();
                self.longueur = format!("{:#x}", octets.len());
            }
            ACTION_COPIER => self.à_copier = Some(hex_dump(octets)),
            _ => {}
        }
    }
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

    /// Ce que la tranche vaut — et rien de la grille.
    ///
    /// ATTENTION, CE PANNEAU EST PARTAGÉ : `ui` ne peint pas cette vue
    /// et s'arrête là, il la peint PUIS dessine la grille d'octets, qui
    /// ne se dit pas. Un rapport, lui, ne reçoit que ce qui se dit —
    /// et c'est bien ce qu'il veut : l'empreinte d'une tranche, son
    /// entropie, le texte qu'on y lit.
    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        (!ctx.binary.is_empty()).then(|| self.vue_de_la_tranche(ctx, ctx.binary))
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let octets = ctx.binary;
        if octets.is_empty() {
            crate::vue_peinte::peindre(
                ui,
                ctx,
                &Vue::neuve().texte(
                    Fragment::dit(ctx.t(
                        "Rien de chargé : il n'y a pas de fichier à vider.",
                        "Nothing loaded: there is no file to dump.",
                    ))
                    .faible(),
                ),
            );
            return;
        }

        // LA TÊTE PASSE LA FRONTIÈRE, LA GRILLE RESTE PEINTE — et c'est
        // la même décision que pour l'éditeur binaire, pour les mêmes
        // raisons plus une.
        //
        // Ce qu'un rapport veut de ce panneau, c'est ce que la tranche
        // VAUT : son empreinte SHA-256, son CRC, son entropie, le texte
        // qu'on y lit. C'est de la compréhension, et cela passe. La
        // grille d'octets, elle, est un instrument : un curseur, une
        // sélection, un clic par octet.
        //
        // Et ici une mesure s'ajoute à l'argument : la grille est
        // VIRTUALISÉE (`show_rows`), parce qu'un fichier de trois cents
        // kilo-octets fait vingt mille lignes. Une vue se rebâtit à
        // chaque image ; la décrire entière, ce serait composer
        // 320 000 fragments soixante fois par seconde pour en peindre
        // quarante. La frontière coûterait alors ce qu'elle doit faire
        // gagner.
        let gestes = crate::vue_peinte::peindre_tout(ui, ctx, &self.vue_de_la_tranche(ctx, octets));
        for action in &gestes.actions {
            self.agit(action, octets);
        }
        if let Some(texte) = self.à_copier.take() {
            ui.ctx().copy_text(texte);
        }

        let choisie = self.choisie(octets);
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        // Le vidage lui-même, une ligne à la fois et seulement celles
        // qu'on voit.
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

    /// La tête du panneau se lit sans être peinte — et la grille non.
    ///
    /// **C'est la frontière telle qu'elle a été tranchée pour ce
    /// panneau** : ce que la tranche VAUT (son étendue, son empreinte,
    /// son CRC, son entropie, le texte qu'on y lit) est de la
    /// compréhension et passe ; la grille d'octets est un instrument,
    /// et de surcroît virtualisée sur vingt mille lignes — une vue se
    /// rebâtit à chaque image.
    ///
    /// Le contrôle suit le chemin entier SANS PEINDRE : la vue propose
    /// les actions, `agit` les applique, la vue suivante porte la
    /// nouvelle tranche.
    #[test]
    fn la_tête_du_vidage_se_lit_sans_être_peinte() {
        use namsdec_core::{Cpu, Memory};
        let octets: Vec<u8> = (0..=255u8).collect();
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &octets,
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = DumpModule::default();

        /// Les paires nom/valeur du tableau de mesures.
        fn mesures(module: &DumpModule, ctx: &ReContext, octets: &[u8]) -> Vec<(String, String)> {
            module
                .vue_de_la_tranche(ctx, octets)
                .blocs
                .iter()
                .filter_map(|bloc| match bloc {
                    Bloc::Tableau { rangées, .. } => Some(rangées),
                    _ => None,
                })
                .flatten()
                .filter_map(|rangée| match rangée.as_slice() {
                    [nom, valeur] => Some((nom.texte_nu(), valeur.texte_nu())),
                    _ => None,
                })
                .collect()
        }

        // Au départ, seize octets depuis zéro : une réponse tout de
        // suite plutôt qu'un champ vide.
        let départ = mesures(&module, &ctx, &octets);
        let tranche = &départ
            .iter()
            .find(|(nom, _)| nom == "Tranche")
            .expect("la tranche est dite")
            .1;
        assert!(tranche.contains("0x10"), "seize octets : {tranche}");
        let empreinte_16 = départ
            .iter()
            .find(|(nom, _)| nom == "SHA-256")
            .expect("l'empreinte est dite")
            .1
            .clone();

        // « TOUT », par l'action que la vue propose.
        let vue = module.vue_de_la_tranche(&ctx, &octets);
        let tout = vue
            .blocs
            .iter()
            .filter_map(|bloc| match bloc {
                Bloc::Champ(champ) => Some(&champ.boutons),
                _ => None,
            })
            .flatten()
            .find(|f| f.texte == "Tout")
            .and_then(|f| f.action.clone())
            .expect("« Tout » se demande");
        module.agit(&tout, &octets);

        let après = mesures(&module, &ctx, &octets);
        let tranche = &après
            .iter()
            .find(|(nom, _)| nom == "Tranche")
            .expect("la tranche est dite")
            .1;
        assert!(tranche.contains("256"), "le fichier entier : {tranche}");
        let empreinte_256 = &après
            .iter()
            .find(|(nom, _)| nom == "SHA-256")
            .expect("l'empreinte est dite")
            .1;
        // L'EMPREINTE A CHANGÉ : c'est ce qui prouve qu'elle porte sur
        // la tranche et non sur le fichier, et c'est la moitié du
        // contrôle — sans elle, on ne vérifierait que l'affichage d'un
        // nombre.
        assert_ne!(
            &empreinte_16, empreinte_256,
            "l'empreinte de seize octets n'est pas celle de deux cent cinquante-six"
        );

        // ET VALIDER LE DÉBUT, C'EST DEMANDER À VOIR. « Aller » porte
        // l'action du champ : sans cela, taper une adresse puis cliquer
        // aurait perdu ce qu'on vient d'écrire.
        module.agit(&format!("{ACTION_DÉBUT}:0x20"), &octets);
        assert_eq!(module.aller_à, Some(0x20), "le vidage s'y rend");

        // COPIER LE VIDAGE ne peint pas : le texte attend d'être pris.
        assert!(module.à_copier.is_none());
        module.agit(ACTION_COPIER, &octets);
        let copié = module
            .à_copier
            .take()
            .expect("le vidage attend d'être pris");
        assert!(
            copié.starts_with("00000000  00 01 02 03"),
            "la forme de xxd, celle qu'un `diff` sait lire : {}",
            &copié[..copié.len().min(40)]
        );
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
