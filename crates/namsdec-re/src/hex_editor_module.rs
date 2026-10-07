use crate::{ReContext, ReModule, ReRequest};

/// Un éditeur binaire : le fichier chargé en grille hexadécimale, un octet
/// qu'on choisit, une valeur qu'on pose à la place.
///
/// C'est la suite naturelle de l'analyse : on a compris qu'un octet décide
/// d'un saut, d'une longueur, d'une constante, et l'on veut le voir
/// autrement — remplacer `75` (jne) par `74` (je), pour lire ce que le
/// programme ferait alors. Un geste qu'on apprend en le faisant, et qui
/// **se défait** : chaque écriture entre dans la liste des correctifs
/// (Outils › Correctifs), avec l'octet d'avant, prête à revenir en
/// arrière. Rien n'est écrit sur le disque tant qu'on ne l'enregistre pas
/// soi-même, vers un fichier qu'on choisit — jamais par-dessus l'original.
///
/// Première version : on navigue, on choisit un octet, on en pose un
/// autre, et ce qu'on a touché se repère au coup d'œil. L'édition de
/// plusieurs octets d'un coup et l'insertion viendront ensuite — insérer
/// décale tout ce qui suit, et chaque adresse du fichier deviendrait
/// fausse, donc elle demande sa propre prudence.
#[derive(Default)]
pub struct HexEditorModule {
    /// L'octet du début de la page affichée.
    page: usize,
    /// Ce qu'on a tapé dans « aller à ».
    aller: String,
    /// L'octet choisi, s'il y en a un.
    choisi: Option<usize>,
    /// La valeur qu'on pose à la place, en hexadécimal.
    valeur: String,
    /// Les octets qu'on a modifiés depuis l'ouverture de ce binaire, pour
    /// les montrer autrement. Remis à zéro au chargement d'un fichier.
    touchés: std::collections::BTreeSet<usize>,
    /// Ce qu'on cherche, et comment : en octets hexadécimaux, ou en texte.
    recherche: String,
    recherche_hex: bool,
    /// La frappe en place : le demi-octet de poids fort déjà tapé, en
    /// attente du second. `None` quand on n'a rien commencé.
    nibble: Option<u8>,
    /// La dernière trouvaille, surlignée dans la grille.
    trouvé: Option<std::ops::Range<usize>>,
    demande: Option<ReRequest>,
}

/// Seize octets par ligne, et seize lignes par page : la forme de tous les
/// éditeurs hexadécimaux, et de `xxd`. Assez pour lire un bloc, pas assez
/// pour noyer la fenêtre.
const PAR_LIGNE: usize = 16;
const LIGNES: usize = 16;
const PAR_PAGE: usize = PAR_LIGNE * LIGNES;

/// Lit un nombre écrit en hexadécimal (`0x…`) ou en décimal.
fn nombre(texte: &str) -> Option<usize> {
    let t = texte.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16).ok()
    } else {
        t.parse().ok()
    }
}

/// Lit un octet : un ou deux chiffres hexadécimaux, de `0` à `ff`.
fn octet(texte: &str) -> Option<u8> {
    let t = texte.trim();
    if t.is_empty() || t.len() > 2 {
        return None;
    }
    u8::from_str_radix(t, 16).ok()
}

impl ReModule for HexEditorModule {
    fn id(&self) -> &'static str {
        "hex_editor"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Éditeur binaire"
        } else {
            "Binary editor"
        }
    }

    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn on_binary_loaded(&mut self, _ctx: &ReContext) {
        // Un autre fichier : ce qu'on avait touché ne le concerne plus.
        self.touchés.clear();
        self.choisi = None;
        self.page = 0;
        self.trouvé = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let octets = ctx.binary;
        if octets.is_empty() {
            ui.weak(ctx.t(
                "Rien de chargé : il n'y a pas de fichier à éditer.",
                "Nothing loaded: there is no file to edit.",
            ));
            return;
        }

        ui.weak(ctx.t(
            "Choisissez un octet, posez-en un autre. Chaque écriture se défait depuis \
             Outils › Correctifs, et rien ne touche le disque tant que vous n'enregistrez \
             pas vous-même, vers un fichier que vous choisissez.",
            "Pick a byte, put another in its place. Each write can be undone from \
             Tools › Patches, and nothing touches the disk until you save it yourself, to \
             a file you choose.",
        ));
        ui.add_space(6.0);

        // La barre de navigation : aller à un décalage, page précédente et
        // suivante. La page est bornée au fichier des deux côtés.
        let dernière_page = octets.len().saturating_sub(1) / PAR_PAGE * PAR_PAGE;
        self.page = self.page.min(dernière_page);
        ui.horizontal_wrapped(|ui| {
            ui.weak(ctx.t("Aller à", "Go to"));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut self.aller)
                    .hint_text("0x1000")
                    .font(egui::TextStyle::Monospace)
                    .desired_width(90.0),
            );
            let validé = champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.button(ctx.t("Aller", "Go")).clicked() || validé)
                && let Some(n) = nombre(&self.aller)
            {
                let cible = n.min(octets.len().saturating_sub(1));
                self.page = cible / PAR_PAGE * PAR_PAGE;
                self.choisi = Some(cible);
            }
            ui.separator();
            if ui
                .add_enabled(self.page > 0, egui::Button::new("◀"))
                .clicked()
            {
                self.page = self.page.saturating_sub(PAR_PAGE);
            }
            ui.monospace(format!(
                "{:#x} … {:#x}",
                self.page,
                (self.page + PAR_PAGE).min(octets.len())
            ));
            if ui
                .add_enabled(self.page < dernière_page, egui::Button::new("▶"))
                .clicked()
            {
                self.page = (self.page + PAR_PAGE).min(dernière_page);
            }
            ui.separator();
            ui.weak(format!("{} {}", octets.len(), ctx.t("octets", "bytes")));
        });

        // La recherche : un motif en octets ou en texte, trouvé en avant
        // ou en arrière, et la trouvaille surlignée.
        ui.add_space(4.0);
        let mut cherche: Option<bool> = None; // Some(précédent)
        ui.horizontal_wrapped(|ui| {
            ui.weak(ctx.t("Chercher", "Find"));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut self.recherche)
                    .hint_text(if self.recherche_hex {
                        "7f 45 4c 46"
                    } else {
                        "ELF"
                    })
                    .font(egui::TextStyle::Monospace)
                    .desired_width(160.0),
            );
            let entrée = champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.selectable_value(&mut self.recherche_hex, false, ctx.t("texte", "text"));
            ui.selectable_value(&mut self.recherche_hex, true, "hex");
            if ui
                .button("▶")
                .on_hover_text(ctx.t("Suivant", "Next"))
                .clicked()
                || entrée
            {
                cherche = Some(false);
            }
            if ui
                .button("◀")
                .on_hover_text(ctx.t("Précédent", "Previous"))
                .clicked()
            {
                cherche = Some(true);
            }
        });
        if let Some(précédent) = cherche {
            match motif(&self.recherche, self.recherche_hex) {
                None => {
                    self.trouvé = None;
                    ui.weak(ctx.t(
                        "Rien à chercher : un texte, ou des octets hexadécimaux par paires.",
                        "Nothing to search: a text, or hexadecimal bytes in pairs.",
                    ));
                }
                Some(m) => {
                    // D'où l'on repart : après l'octet choisi pour « suivant »,
                    // depuis lui pour « précédent ».
                    let depuis = match self.choisi {
                        Some(c) if !précédent => c + 1,
                        Some(c) => c,
                        None => 0,
                    };
                    match trouver(octets, &m, depuis, précédent) {
                        Some(pos) => {
                            self.trouvé = Some(pos..pos + m.len());
                            self.choisi = Some(pos);
                            self.valeur = format!("{:02x}", octets[pos]);
                            self.page = pos / PAR_PAGE * PAR_PAGE;
                        }
                        None => {
                            self.trouvé = None;
                            ui.weak(ctx.t("Introuvable.", "Not found."));
                        }
                    }
                }
            }
        }

        ui.add_space(6.0);
        let visuals = ui.visuals().clone();
        let fond_choisi = visuals.selection.bg_fill;
        let fond_touché = visuals.warn_fg_color.gamma_multiply(0.30);
        let fond_trouvé = visuals.hyperlink_color.gamma_multiply(0.35);
        let mut nouveau_choix: Option<usize> = None;
        let glyphe = egui::TextStyle::Monospace.resolve(ui.style());

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                let fin = (self.page + PAR_PAGE).min(octets.len());
                let mut ligne = self.page;
                while ligne < fin {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        // Le décalage de la ligne, en tête.
                        ui.monospace(
                            egui::RichText::new(format!("{ligne:08x}"))
                                .color(visuals.weak_text_color()),
                        );
                        ui.add_space(2.0);
                        // Les octets, cliquables.
                        for col in 0..PAR_LIGNE {
                            let off = ligne + col;
                            if off >= fin {
                                // Aligner l'ASCII même sur une ligne
                                // incomplète : deux espaces par octet absent.
                                ui.monospace("  ");
                            } else {
                                let mut texte =
                                    egui::RichText::new(format!("{:02x}", octets[off])).monospace();
                                if Some(off) == self.choisi {
                                    texte = texte.background_color(fond_choisi);
                                } else if self.trouvé.as_ref().is_some_and(|r| r.contains(&off)) {
                                    texte = texte.background_color(fond_trouvé);
                                } else if self.touchés.contains(&off) {
                                    texte = texte.background_color(fond_touché);
                                }
                                if ui
                                    .add(egui::Label::new(texte).sense(egui::Sense::click()))
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    nouveau_choix = Some(off);
                                }
                            }
                            // Un demi-espace entre les octets, un cran de
                            // plus au milieu : l'œil retrouve la 8e colonne.
                            if col == 7 {
                                ui.add_space(6.0);
                            }
                        }
                        ui.add_space(10.0);
                        // La colonne de texte : ce qui est imprimable, le
                        // reste en point médian.
                        let mut ascii = String::with_capacity(PAR_LIGNE);
                        for col in 0..PAR_LIGNE {
                            let off = ligne + col;
                            if off >= fin {
                                break;
                            }
                            let b = octets[off];
                            ascii.push(if (0x20..0x7f).contains(&b) {
                                b as char
                            } else {
                                '·'
                            });
                        }
                        ui.monospace(egui::RichText::new(ascii).color(visuals.weak_text_color()));
                    });
                    ligne += PAR_LIGNE;
                }
                let _ = glyphe;
            });

        if let Some(off) = nouveau_choix {
            self.choisi = Some(off);
            self.valeur = format!("{:02x}", octets[off]);
            self.nibble = None;
        }

        // Le clavier, quand un octet est choisi et qu'aucun champ de texte
        // n'a le focus (sinon la frappe irait à « Chercher » ou « Aller
        // à »). Les flèches déplacent le curseur ; les chiffres hexa le
        // changent en place, par quartets, comme dans un éditeur hexa.
        if let Some(off) = self.choisi
            && ui.memory(|m| m.focused()).is_none()
        {
            let dernier = octets.len().saturating_sub(1);
            let mut bascule: Option<usize> = None;
            let mut chiffre: Option<u8> = None;
            let mut annule = false;
            ui.input_mut(|i| {
                use egui::Key;
                let rien = egui::Modifiers::NONE;
                if i.consume_key(rien, Key::ArrowRight) {
                    bascule = Some((off + 1).min(dernier));
                } else if i.consume_key(rien, Key::ArrowLeft) {
                    bascule = Some(off.saturating_sub(1));
                } else if i.consume_key(rien, Key::ArrowDown) {
                    bascule = Some((off + PAR_LIGNE).min(dernier));
                } else if i.consume_key(rien, Key::ArrowUp) {
                    bascule = Some(off.saturating_sub(PAR_LIGNE));
                } else if i.consume_key(rien, Key::PageDown) {
                    bascule = Some((off + PAR_PAGE).min(dernier));
                } else if i.consume_key(rien, Key::PageUp) {
                    bascule = Some(off.saturating_sub(PAR_PAGE));
                } else if i.consume_key(rien, Key::Escape) {
                    annule = true;
                } else {
                    // Un chiffre hexadécimal ?
                    for (k, v) in [
                        (Key::Num0, 0u8),
                        (Key::Num1, 1),
                        (Key::Num2, 2),
                        (Key::Num3, 3),
                        (Key::Num4, 4),
                        (Key::Num5, 5),
                        (Key::Num6, 6),
                        (Key::Num7, 7),
                        (Key::Num8, 8),
                        (Key::Num9, 9),
                        (Key::A, 10),
                        (Key::B, 11),
                        (Key::C, 12),
                        (Key::D, 13),
                        (Key::E, 14),
                        (Key::F, 15),
                    ] {
                        if i.consume_key(rien, k) {
                            chiffre = Some(v);
                            break;
                        }
                    }
                }
            });
            if annule {
                self.nibble = None;
            }
            if let Some(cible) = bascule {
                self.choisi = Some(cible);
                self.valeur = format!("{:02x}", octets[cible]);
                self.nibble = None;
                // Suivre le curseur : s'il sort de la page, on tourne.
                if cible < self.page || cible >= self.page + PAR_PAGE {
                    self.page = cible / PAR_PAGE * PAR_PAGE;
                }
            }
            if let Some(d) = chiffre {
                let (attente, complet) = frapper(self.nibble, d);
                self.nibble = attente;
                // L'octet est complet : on l'écrit et l'on avance — la
                // frappe continue toute seule, quartet après quartet.
                if let Some(valeur) = complet {
                    if octets.get(off).copied() != Some(valeur) {
                        self.demande = Some(ReRequest::PatchBytes {
                            offset: off,
                            bytes: vec![valeur],
                        });
                        self.touchés.insert(off);
                    }
                    let suivant = (off + 1).min(dernier);
                    self.choisi = Some(suivant);
                    self.valeur = format!("{:02x}", octets.get(suivant).copied().unwrap_or(0));
                    if suivant >= self.page + PAR_PAGE {
                        self.page = suivant / PAR_PAGE * PAR_PAGE;
                    }
                }
            }
        }

        // L'octet choisi, et de quoi le remplacer.
        if let Some(off) = self.choisi {
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.monospace(
                    egui::RichText::new(format!(
                        "{} {off:#x}  =  {:02x}",
                        ctx.t("Octet", "Byte"),
                        octets.get(off).copied().unwrap_or(0)
                    ))
                    .strong(),
                );
                ui.add_space(8.0);
                ui.weak(ctx.t("→", "→"));
                let champ = ui.add(
                    egui::TextEdit::singleline(&mut self.valeur)
                        .hint_text("90")
                        .font(egui::TextStyle::Monospace)
                        .desired_width(44.0),
                );
                let nouvel = octet(&self.valeur);
                let change = nouvel.is_some_and(|b| Some(b) != octets.get(off).copied());
                let validé = champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.add_enabled_ui(change, |ui| {
                    if (ui.button(ctx.t("Écrire", "Write")).clicked() || (validé && change))
                        && let Some(b) = nouvel
                    {
                        self.demande = Some(ReRequest::PatchBytes {
                            offset: off,
                            bytes: vec![b],
                        });
                        self.touchés.insert(off);
                    }
                });
            });
            if !self.valeur.trim().is_empty() && octet(&self.valeur).is_none() {
                ui.weak(ctx.t(
                    "Un octet en hexadécimal : de « 0 » à « ff ».",
                    "One byte in hexadecimal: from \"0\" to \"ff\".",
                ));
            }
            // La frappe en place : le quartet déjà tapé, en attente du
            // second, et l'indice pour qui ne connaît pas le geste.
            if let Some(haut) = self.nibble {
                ui.weak(
                    egui::RichText::new(format!("{}{haut:x}_", ctx.t("frappe : ", "typing: ")))
                        .monospace(),
                );
            } else {
                ui.weak(ctx.t(
                    "Flèches pour se déplacer, chiffres hexa pour écrire en place.",
                    "Arrows to move, hex digits to type in place.",
                ));
            }

            // L'inspecteur : les octets à partir d'ici, lus dans chaque
            // format. Petit-boutien, celui du x86.
            let reste = &octets[off..];
            let lectures = inspecter(reste);
            if !lectures.is_empty() {
                ui.add_space(6.0);
                ui.weak(ctx.t(
                    "Lu à partir d'ici (petit-boutien) :",
                    "Read from here (little-endian):",
                ));
                egui::Grid::new("hex_inspecteur")
                    .num_columns(2)
                    .min_col_width(0.0)
                    .spacing([16.0, 2.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (nom, valeur) in lectures {
                            ui.weak(nom);
                            if ui
                                .add(
                                    egui::Label::new(egui::RichText::new(&valeur).monospace())
                                        .sense(egui::Sense::click()),
                                )
                                .on_hover_text(ctx.t("Cliquer pour copier", "Click to copy"))
                                .clicked()
                            {
                                ui.ctx().copy_text(valeur);
                            }
                            ui.end_row();
                        }
                    });
            }
        } else {
            ui.add_space(4.0);
            ui.weak(ctx.t(
                "Cliquez un octet pour le choisir.",
                "Click a byte to pick it.",
            ));
        }
    }
}

/// Lit ce qu'on cherche : une suite d'octets hexadécimaux (« 7f 45 4c »
/// ou « 7f454c »), ou un texte pris tel quel. Rien pour une saisie vide
/// ou un hexadécimal impair — on ne devine pas un demi-octet.
fn motif(query: &str, hex: bool) -> Option<Vec<u8>> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    if !hex {
        return Some(q.as_bytes().to_vec());
    }
    let nettoyé: String = q.chars().filter(|c| !c.is_whitespace()).collect();
    if nettoyé.is_empty() || !nettoyé.len().is_multiple_of(2) {
        return None;
    }
    let octets = nettoyé.as_bytes();
    let mut out = Vec::with_capacity(octets.len() / 2);
    let mut i = 0;
    while i < octets.len() {
        let paire = std::str::from_utf8(&octets[i..i + 2]).ok()?;
        out.push(u8::from_str_radix(paire, 16).ok()?);
        i += 2;
    }
    Some(out)
}

/// Cherche le motif dans les octets, à partir de `depuis`, dans un sens ou
/// l'autre. Revient au début (ou à la fin) quand il n'y a plus rien devant
/// — chercher à nouveau fait le tour, comme dans un éditeur.
fn trouver(octets: &[u8], motif: &[u8], depuis: usize, précédent: bool) -> Option<usize> {
    if motif.is_empty() || motif.len() > octets.len() {
        return None;
    }
    let dernier = octets.len() - motif.len();
    let occupe = |i: usize| &octets[i..i + motif.len()] == motif;
    if précédent {
        (0..=dernier)
            .rev()
            .find(|&i| i < depuis && occupe(i))
            .or_else(|| (0..=dernier).rev().find(|&i| occupe(i)))
    } else {
        (depuis..=dernier)
            .find(|&i| occupe(i))
            .or_else(|| (0..=dernier).find(|&i| occupe(i)))
    }
}

/// L'inspecteur de données : comment lire les octets à partir du curseur,
/// dans les formats qu'un désassembleur rencontre. C'est l'outil phare de
/// 010 Editor, et le plus pédagogique — « ces quatre octets, c'est ce
/// nombre-là ». En petit-boutien (`LE`), celui du x86 : l'octet de poids
/// faible d'abord, ce qui déroute tant qu'on ne l'a pas vu une fois.
///
/// Fonction libre, pour l'éprouver sans interface.
fn inspecter(octets: &[u8]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let Some(&b) = octets.first() else {
        return out;
    };
    out.push(("int8", (b as i8).to_string()));
    out.push(("uint8", b.to_string()));
    out.push(("binaire", format!("{b:08b}")));
    out.push((
        "caractère",
        if (0x20..0x7f).contains(&b) {
            (b as char).to_string()
        } else {
            "·".to_owned()
        },
    ));
    if let Ok(deux) = <[u8; 2]>::try_from(&octets[..2.min(octets.len())]) {
        out.push(("int16 LE", i16::from_le_bytes(deux).to_string()));
        out.push(("uint16 LE", u16::from_le_bytes(deux).to_string()));
    }
    if octets.len() >= 4 {
        let quatre = <[u8; 4]>::try_from(&octets[..4]).unwrap();
        out.push(("int32 LE", i32::from_le_bytes(quatre).to_string()));
        out.push(("uint32 LE", u32::from_le_bytes(quatre).to_string()));
        out.push(("float32 LE", format!("{}", f32::from_le_bytes(quatre))));
    }
    if octets.len() >= 8 {
        let huit = <[u8; 8]>::try_from(&octets[..8]).unwrap();
        out.push(("int64 LE", i64::from_le_bytes(huit).to_string()));
        out.push(("uint64 LE", u64::from_le_bytes(huit).to_string()));
        out.push(("float64 LE", format!("{}", f64::from_le_bytes(huit))));
    }
    out
}

/// Applique un chiffre hexadécimal à la frappe en place.
///
/// Rend le nouvel état d'attente et, quand l'octet est complet, la valeur
/// à écrire. Premier quartet : on le garde (poids fort), rien à écrire.
/// Second : l'octet est `haut<<4 | bas`, et l'attente retombe.
fn frapper(nibble: Option<u8>, chiffre: u8) -> (Option<u8>, Option<u8>) {
    match nibble {
        None => (Some(chiffre), None),
        Some(haut) => (None, Some((haut << 4) | chiffre)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_nombre_se_lit_en_hex_ou_en_decimal() {
        assert_eq!(nombre("0x10"), Some(16));
        assert_eq!(nombre("16"), Some(16));
        assert_eq!(nombre("  0X1f "), Some(31));
        assert_eq!(nombre("zz"), None);
        assert_eq!(nombre(""), None);
    }

    #[test]
    fn un_octet_tient_sur_deux_chiffres_hex() {
        assert_eq!(octet("90"), Some(0x90));
        assert_eq!(octet("f"), Some(0x0f));
        assert_eq!(octet("ff"), Some(0xff));
        assert_eq!(octet("100"), None); // trois chiffres
        assert_eq!(octet("gg"), None);
        assert_eq!(octet(""), None);
    }

    /// Changer d'octet oublie ce qu'on avait touché : un nouveau fichier
    /// n'hérite pas des marques de l'ancien.
    /// L'inspecteur lit les octets dans chaque format, en petit-boutien,
    /// et ne propose que les largeurs qui tiennent dans ce qui reste.
    /// La frappe en place assemble l'octet quartet par quartet.
    #[test]
    fn la_frappe_assemble_l_octet_par_quartets() {
        // Rien tapé : le premier chiffre devient le quartet de poids fort,
        // rien n'est encore écrit.
        assert_eq!(frapper(None, 0x9), (Some(0x9), None));
        // Un quartet en attente : le second complète l'octet, l'attente
        // retombe. 0x9 puis 0x0 -> 0x90.
        assert_eq!(frapper(Some(0x9), 0x0), (None, Some(0x90)));
        // 0x7 puis 0x4 -> 0x74 (je).
        assert_eq!(frapper(Some(0x7), 0x4), (None, Some(0x74)));
        // 0x0 puis 0xf -> 0x0f.
        assert_eq!(frapper(Some(0x0), 0xf), (None, Some(0x0f)));
    }

    #[test]
    fn un_motif_se_lit_en_hex_ou_en_texte() {
        assert_eq!(motif("ELF", false), Some(b"ELF".to_vec()));
        assert_eq!(
            motif("7f 45 4c 46", true),
            Some(vec![0x7f, 0x45, 0x4c, 0x46])
        );
        assert_eq!(motif("7f454c46", true), Some(vec![0x7f, 0x45, 0x4c, 0x46]));
        // Un demi-octet ne se devine pas.
        assert_eq!(motif("7f4", true), None);
        assert_eq!(motif("zz", true), None);
        assert_eq!(motif("   ", false), None);
    }

    /// La recherche va en avant, en arrière, et fait le tour quand il n'y
    /// a plus rien devant — comme dans un éditeur.
    #[test]
    fn la_recherche_trouve_et_fait_le_tour() {
        let octets = b"ab_ab_ab"; // 'ab' en 0, 3, 6
        let m = b"ab";
        // En avant depuis 0 : la première.
        assert_eq!(trouver(octets, m, 0, false), Some(0));
        // En avant depuis 1 : la suivante.
        assert_eq!(trouver(octets, m, 1, false), Some(3));
        // En avant après la dernière : on fait le tour, retour à 0.
        assert_eq!(trouver(octets, m, 7, false), Some(0));
        // En arrière depuis 6 : la précédente.
        assert_eq!(trouver(octets, m, 6, true), Some(3));
        // En arrière depuis 0 : le tour, la dernière.
        assert_eq!(trouver(octets, m, 0, true), Some(6));
        // Un motif plus long que le fichier : rien.
        assert_eq!(trouver(b"x", m, 0, false), None);
        // Absent.
        assert_eq!(trouver(octets, b"zz", 0, false), None);
    }

    #[test]
    fn l_inspecteur_lit_chaque_format() {
        let trouve = |lectures: &[(&str, String)], nom: &str| {
            lectures
                .iter()
                .find(|(n, _)| *n == nom)
                .map(|(_, v)| v.clone())
        };

        // 0x01 suivi de zéros : 1 dans toutes les largeurs non signées.
        let l = inspecter(&[0x01, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(trouve(&l, "uint8").as_deref(), Some("1"));
        assert_eq!(trouve(&l, "uint16 LE").as_deref(), Some("1"));
        assert_eq!(trouve(&l, "uint32 LE").as_deref(), Some("1"));
        assert_eq!(trouve(&l, "uint64 LE").as_deref(), Some("1"));

        // 0xff seul : -1 signé, 255 non signé, et les bits.
        let l = inspecter(&[0xff]);
        assert_eq!(trouve(&l, "int8").as_deref(), Some("-1"));
        assert_eq!(trouve(&l, "uint8").as_deref(), Some("255"));
        assert_eq!(trouve(&l, "binaire").as_deref(), Some("11111111"));
        // Un seul octet : pas de largeur de 16 bits ou plus.
        assert!(trouve(&l, "int16 LE").is_none());
        assert!(trouve(&l, "uint32 LE").is_none());

        // Petit-boutien : 0x00 0x01 = 256, et non 1.
        let l = inspecter(&[0x00, 0x01]);
        assert_eq!(trouve(&l, "uint16 LE").as_deref(), Some("256"));

        // Rien à lire sur rien.
        assert!(inspecter(&[]).is_empty());
    }

    #[test]
    fn un_nouveau_binaire_efface_les_marques() {
        use namsdec_core::{Cpu, Memory};
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &[1, 2, 3],
            lang: "fr",
            names: None,
        };
        let mut module = HexEditorModule::default();
        module.touchés.insert(2);
        module.choisi = Some(2);
        module.page = 256;
        module.on_binary_loaded(&ctx);
        assert!(module.touchés.is_empty());
        assert_eq!(module.choisi, None);
        assert_eq!(module.page, 0);
    }
}
