use crate::{ReContext, ReModule, digest};

/// L'identité du fichier : sa taille et ses empreintes.
///
/// C'est la première ligne d'un compte rendu, avant même de dire ce que
/// le programme fait : *de quel fichier parle-t-on ?* Deux personnes qui
/// lisent « le crackme » n'ont pas forcément le même octet sous les
/// yeux, et l'empreinte est ce qui tranche. Elle sert aussi à soi-même,
/// trois semaines plus tard, devant deux fichiers du même nom.
///
/// Calculées une fois, au chargement : une empreinte de trois cents
/// kilo-octets coûte une milliseconde, et la refaire à chaque image
/// serait la payer soixante fois par seconde pour rien.
#[derive(Default)]
pub struct DigestModule {
    taille: usize,
    sha256: String,
    crc32: u32,
}

impl ReModule for DigestModule {
    fn id(&self) -> &'static str {
        "digest"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Empreintes"
        } else {
            "Fingerprints"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.taille = ctx.binary.len();
        self.sha256 = digest::hex(&digest::sha256(ctx.binary));
        self.crc32 = digest::crc32(ctx.binary);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.taille == 0 {
            ui.weak(ctx.t(
                "Rien de chargé : il n'y a pas de fichier à empreindre.",
                "Nothing loaded: there is no file to fingerprint.",
            ));
            return;
        }
        let mut à_copier: Option<String> = None;
        egui::Grid::new("digest_grid")
            .num_columns(2)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                ui.label(ctx.t("Taille", "Size"));
                ui.monospace(format!("{} {}", self.taille, ctx.t("octets", "bytes")));
                ui.end_row();

                // Cliquables, les deux : une empreinte sert à être
                // collée quelque part, pas à être recopiée à la main.
                ui.label("SHA-256");
                if ui
                    .add(
                        egui::Label::new(egui::RichText::new(&self.sha256).monospace())
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_text(ctx.t("Cliquer pour copier", "Click to copy"))
                    .clicked()
                {
                    à_copier = Some(self.sha256.clone());
                }
                ui.end_row();

                ui.label("CRC-32");
                let crc = format!("{:08x}", self.crc32);
                if ui
                    .add(
                        egui::Label::new(egui::RichText::new(&crc).monospace())
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_text(ctx.t("Cliquer pour copier", "Click to copy"))
                    .clicked()
                {
                    à_copier = Some(crc);
                }
                ui.end_row();
            });
        if let Some(texte) = à_copier {
            ui.ctx().copy_text(texte);
        }

        ui.add_space(4.0);
        ui.weak(ctx.t(
            "SHA-256 identifie le fichier : deux contenus différents ne la partagent pas. \
             CRC-32 ne fait que dire s'il est arrivé entier — on en fabrique deux identiques \
             sans peine, et elle ne prouve donc rien.",
            "SHA-256 identifies the file: two different contents never share one. CRC-32 only \
             says whether it arrived intact — two identical ones are easy to build, so it \
             proves nothing.",
        ));
        ui.add_space(2.0);
        ui.weak(ctx.t(
            "Ni MD5 ni SHA-1 : ce sont des empreintes cassées, et les afficher à côté d'une \
             bonne sans le dire apprendrait le faux.",
            "No MD5 or SHA-1: those are broken, and showing them beside a sound one without \
             saying so would teach something false.",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// L'empreinte porte sur le **fichier**, non sur ce qui en est
    /// désassemblé.
    ///
    /// C'est ce qui permet de la comparer à celle qu'un autre outil
    /// donne : `sha256sum` lit le fichier entier, en-têtes compris.
    #[test]
    fn the_fingerprint_covers_the_whole_file() {
        let binaire = b"\x7fELF et la suite du fichier".to_vec();
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &binaire,
            lang: "fr",
        };
        let mut module = DigestModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.taille, binaire.len());
        assert_eq!(module.sha256, digest::hex(&digest::sha256(&binaire)));
        assert_eq!(module.crc32, digest::crc32(&binaire));
        // Soixante-quatre caractères hexadécimaux, la forme qu'on colle.
        assert_eq!(module.sha256.len(), 64);
        assert!(module.sha256.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Rien de chargé : rien à empreindre, et le module le dit au lieu
    /// de montrer l'empreinte du vide, qui ressemble à une vraie.
    #[test]
    fn nothing_loaded_is_said_rather_than_fingerprinted() {
        let module = DigestModule::default();
        assert_eq!(module.taille, 0);
        assert!(module.sha256.is_empty());
    }
}
