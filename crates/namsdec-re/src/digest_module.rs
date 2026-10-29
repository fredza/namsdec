use crate::vue::{Bloc, Fragment, Vue};
use crate::{ReContext, ReModule, ReSection, digest};

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
    fn section(&self) -> ReSection {
        ReSection::File
    }

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

    /// Ce que ce panneau montre — premier module passé par la
    /// frontière de P7, et choisi pour cela : un tableau, du texte, et
    /// une action. Rien de ce qu'il décrit ici ne suppose egui.
    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        if self.taille == 0 {
            return Some(
                Vue::neuve().texte(
                    Fragment::dit(ctx.t(
                        "Rien de chargé : il n'y a pas de fichier à empreindre.",
                        "Nothing loaded: there is no file to fingerprint.",
                    ))
                    .faible(),
                ),
            );
        }
        let copier = ctx.t("Cliquer pour copier", "Click to copy");
        Some(
            Vue::neuve()
                .bloc(Bloc::Tableau {
                    colonnes: Vec::new(),
                    rangées: vec![
                        vec![
                            ctx.t("Taille", "Size").into(),
                            Fragment::dit(format!("{} {}", self.taille, ctx.t("octets", "bytes")))
                                .code()
                                .into(),
                        ],
                        vec![
                            "SHA-256".into(),
                            // Une empreinte sert à être collée quelque
                            // part, pas à être recopiée à la main.
                            Fragment::dit(&self.sha256)
                                .code()
                                .copiable()
                                .survol(copier)
                                .into(),
                        ],
                        vec![
                            "CRC-32".into(),
                            Fragment::dit(format!("{:08x}", self.crc32))
                                .code()
                                .copiable()
                                .survol(copier)
                                .into(),
                        ],
                    ],
                    rayé: true,
                })
                .texte(
                    Fragment::dit(ctx.t(
                        "SHA-256 identifie le fichier : deux contenus différents ne la \
                         partagent pas. CRC-32 ne fait que dire s'il est arrivé entier — on \
                         en fabrique deux identiques sans peine, et elle ne prouve donc rien.",
                        "SHA-256 identifies the file: two different contents never share one. \
                         CRC-32 only says whether it arrived intact — two identical ones are \
                         easy to build, so it proves nothing.",
                    ))
                    .faible(),
                )
                .texte(
                    Fragment::dit(ctx.t(
                        "Ni MD5 ni SHA-1 : ce sont des empreintes cassées, et les afficher à \
                         côté d'une bonne sans le dire apprendrait le faux.",
                        "No MD5 or SHA-1: those are broken, and showing them beside a sound \
                         one without saying so would teach something false.",
                    ))
                    .faible(),
                ),
        )
    }

    /// Dérivé de la vue, et c'est tout l'objet de P7 : une seule
    /// description du panneau, peinte par un traducteur commun.
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if let Some(vue) = self.vue(ctx) {
            crate::vue_peinte::peindre(ui, ctx, &vue);
        }
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
            names: None,
            patches: &[],
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

    /// Ce que le panneau montre se lit **sans le peindre**, et c'est
    /// toute la raison d'être de la frontière de P7.
    ///
    /// Avant, cette vérification demandait un contexte egui, une
    /// surface, un thème, et finissait par comparer des rectangles.
    /// Elle lit maintenant du texte — et le même texte part dans un
    /// rapport ou dans un export.
    #[test]
    fn ce_que_le_panneau_montre_se_lit_sans_le_peindre() {
        let binaire = b"\x7fELF et la suite du fichier".to_vec();
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &binaire,
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = DigestModule::default();

        // Rien de chargé : une seule ligne, et elle le dit.
        let vide = module.vue(&ctx).expect("une vue, même vide");
        assert_eq!(vide.blocs.len(), 1);
        assert!(
            matches!(&vide.blocs[0], Bloc::Texte(ligne) if ligne.texte_nu().contains("Rien de chargé")),
            "{:?}",
            vide.blocs[0]
        );

        module.on_binary_loaded(&ctx);
        let vue = module.vue(&ctx).expect("une vue");
        let Some(Bloc::Tableau { rangées, .. }) = vue.blocs.first() else {
            panic!("le panneau commence par son tableau : {:?}", vue.blocs);
        };
        assert_eq!(rangées.len(), 3, "taille, SHA-256, CRC-32");
        assert_eq!(rangées[1][0].texte_nu(), "SHA-256");
        assert_eq!(
            rangées[1][1].texte_nu(),
            digest::hex(&digest::sha256(&binaire)),
            "l'empreinte montrée est celle du fichier"
        );
        assert!(
            rangées[1][1].fragments[0].copiable,
            "et elle se copie d'un clic"
        );

        // Et la vue entière se sérialise : c'est ce qui la rend
        // utilisable par autre chose que namsdec.
        let json = serde_json::to_string(&vue).expect("la vue sort de l'outil");
        assert!(json.contains("SHA-256"), "{json}");
    }
}
