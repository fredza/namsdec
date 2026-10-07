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

        ui.add_space(6.0);
        let visuals = ui.visuals().clone();
        let fond_choisi = visuals.selection.bg_fill;
        let fond_touché = visuals.warn_fg_color.gamma_multiply(0.30);
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
        } else {
            ui.add_space(4.0);
            ui.weak(ctx.t(
                "Cliquez un octet pour le choisir.",
                "Click a byte to pick it.",
            ));
        }
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
