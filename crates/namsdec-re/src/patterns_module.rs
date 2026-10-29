//! Le module « Figures » : ce que le code fait de remarquable.
//!
//! Le classement suit une règle, et une seule : **ce qui est rare est ce
//! qui parle**. Dans un binaire de soixante mille instructions, mille
//! sept cents tests de nullité n'apprennent rien — c'est la ponctuation
//! du langage machine. Quatre comparaisons par XOR, en revanche,
//! désignent quatre endroits, et dans un programme qui demande un mot de
//! passe, l'un d'eux est la réponse.
//!
//! Les familles sont donc rangées de la plus rare à la plus commune, et
//! chacune dit son compte : c'est le compte qui permet de juger si la
//! piste vaut d'être suivie.

use std::collections::BTreeMap;

use crate::misalign::{self, Misalignment};
use crate::patterns::{Finding, Pattern, scan};
use crate::vue::{Bloc, Fragment, Ton, Vue};
use crate::{ReContext, ReModule, ReRequest, ReSection};

#[derive(Default)]
pub struct PatternsModule {
    /// Ce que le panneau demande à l'application : aller voir une
    /// adresse. Relevé après le dessin, jamais pendant.
    demande: Option<ReRequest>,
    found: Vec<Finding>,
    /// Les endroits où le flot lit d'autres octets que le listing.
    ///
    /// Rangés ici et non dans `patterns` : une figure de `patterns` se
    /// résume à une ligne de détail, celle-ci demande les **deux
    /// lectures** côte à côte, et c'est leur désaccord qui est
    /// l'information.
    misalignments: Vec<Misalignment>,
}

impl PatternsModule {
    /// Les désalignements relevés, pour qui veut les lire sans
    /// l'interface — le panneau des signaux s'en sert.
    pub fn misalignments(&self) -> &[Misalignment] {
        &self.misalignments
    }
}

/// Combien d'endroits on montre par famille.
///
/// Au-delà, la liste cesse d'être une piste pour devenir un annuaire —
/// et le compte total, lui, reste affiché.
const PAR_FAMILLE: usize = 12;

impl ReModule for PatternsModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Code
    }

    fn id(&self) -> &'static str {
        "patterns"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Figures du code"
        } else {
            "Code shapes"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.found = scan(ctx.disasm);
        // Borné aux sections exécutables quand on sait où elles sont :
        // une chaîne de format décodée comme du code rend des sauts qui
        // tombent au hasard, et le panneau crierait au loup sur le
        // premier binaire venu.
        let base = ctx.disasm.first().map(|i| i.address).unwrap_or(0);
        self.misalignments = misalign::scan_in(ctx.disasm, &misalign::code_zones(ctx.binary, base));
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        let mut vue = Vue::neuve();
        // Le désalignement d'abord, et sans compromis : c'est la seule
        // figure de ce panneau qui dise que **le listing lui-même ment**.
        // Toutes les autres se lisent dans un listing juste.
        if !self.misalignments.is_empty() {
            for bloc in désalignements(ctx, &self.misalignments) {
                vue = vue.bloc(bloc);
            }
        }
        if self.found.is_empty() {
            if self.misalignments.is_empty() {
                vue = vue.texte(ctx.t(
                    "Aucune figure reconnue : ce code ne compare rien de remarquable.",
                    "No shape recognised: this code compares nothing remarkable.",
                ));
            }
            return Some(vue);
        }
        vue = vue.texte(
            Fragment::dit(ctx.t(
                "Rangées de la plus rare à la plus commune : dans un gros binaire, mille \
                 tests de nullité sont de la ponctuation, et quatre comparaisons par XOR \
                 sont une piste.",
                "Sorted from rarest to commonest: in a large binary, a thousand null tests \
                 are punctuation, and four XOR comparisons are a lead.",
            ))
            .faible(),
        );

        let mut familles: BTreeMap<Pattern, Vec<&Finding>> = BTreeMap::new();
        for finding in &self.found {
            familles.entry(finding.pattern).or_default().push(finding);
        }
        let mut rangées: Vec<(Pattern, Vec<&Finding>)> = familles.into_iter().collect();
        rangées.sort_by_key(|(_, occurrences)| occurrences.len());

        for (pattern, occurrences) in rangées {
            let mut dedans = vec![
                Bloc::Texte(Fragment::dit(pattern.explain(ctx.lang)).faible().into()),
                Bloc::Tableau {
                    colonnes: Vec::new(),
                    rangées: occurrences
                        .iter()
                        .take(PAR_FAMILLE)
                        .map(|finding| {
                            vec![
                                Fragment::dit(format!("{:#010x}", finding.addr))
                                    .mène_à(finding.addr)
                                    .into(),
                                Fragment::dit(&finding.detail).code().into(),
                            ]
                        })
                        .collect(),
                    rayé: true,
                },
            ];
            if occurrences.len() > PAR_FAMILLE {
                dedans.push(Bloc::Texte(
                    Fragment::dit(format!(
                        "… {} {}",
                        occurrences.len() - PAR_FAMILLE,
                        ctx.t("de plus", "more")
                    ))
                    .faible()
                    .into(),
                ));
            }
            vue = vue.bloc(Bloc::Repliable {
                titre: format!(
                    "{} — {} {}",
                    pattern.title(ctx.lang),
                    occurrences.len(),
                    ctx.t("endroits", "places")
                )
                .into(),
                // Ouvertes tant qu'elles restent lisibles : une famille
                // de douze endroits se parcourt, une de mille se
                // demande.
                ouvert: occurrences.len() <= PAR_FAMILLE,
                blocs: dedans,
            });
        }
        Some(vue)
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if let Some(vue) = self.vue(ctx)
            && let Some(demande) = crate::vue_peinte::peindre(ui, ctx, &vue)
        {
            // Relevé pendant le dessin, posé après : c'est la règle de
            // tous les panneaux, et la vue ne la change pas.
            self.demande = Some(demande);
        }
    }
}

/// Les deux lectures, côte à côte.
///
/// C'est la forme qui apprend quelque chose : « voici ce que le listing
/// affiche, voici ce que le processeur lit, et voici l'octet qui les
/// sépare ». Une ligne qui dirait seulement « désalignement en 0x401234 »
/// laisserait tout le travail à faire.
fn désalignements(ctx: &ReContext, trouvés: &[Misalignment]) -> Vec<Bloc> {
    let mut blocs = vec![
        Bloc::Texte(
            vec![
                // Le rouge est un VERDICT depuis P7 : le module dit que
                // c'est grave, l'interface choisit la teinte.
                Fragment::dit(ctx.t("Le listing est trompé", "The listing is being fooled"))
                    .ton(Ton::Mauvais),
                Fragment::dit(format!(
                    " {} {}",
                    trouvés.len(),
                    if trouvés.len() > 1 {
                        ctx.t("endroits", "places")
                    } else {
                        ctx.t("endroit", "place")
                    }
                ))
                .faible(),
            ]
            .into(),
        ),
        Bloc::Texte(
            Fragment::dit(ctx.t(
                "Un saut atterrit au MILIEU d'une instruction du listing. Le listing lit ligne à ligne ; le processeur, lui, va où les sauts l'envoient — et lit donc d'autres octets. C'est l'anti-désassemblage classique : un seul octet-piège suffit à faire mentir tout ce qui suit. Le découpage en instructions n'est pas une propriété des octets, c'est une propriété du point où l'on commence à lire.",
                "A jump lands in the MIDDLE of an instruction of the listing. The listing reads line by line; the processor goes where jumps send it — and therefore reads other bytes. This is classic anti-disassembly: a single trap byte is enough to make everything that follows lie. Where instructions begin is not a property of the bytes, it is a property of where one starts reading.",
            ))
            .faible()
            .into(),
        ),
    ];

    for m in trouvés {
        blocs.push(Bloc::Texte(
            vec![
                Fragment::dit(format!("{:#010x}", m.from)).mène_à(m.from),
                Fragment::dit(ctx.t(" envoie à ", " sends to ")),
                Fragment::dit(format!("{:#010x}", m.target)).code(),
                Fragment::dit(format!(
                    " — {} {} {} {:#x}",
                    m.offset,
                    if m.offset > 1 {
                        ctx.t("octets", "bytes")
                    } else {
                        ctx.t("octet", "byte")
                    },
                    ctx.t("après le début de", "past the start of"),
                    m.inside
                ))
                .faible(),
            ]
            .into(),
        ));
        // Les deux lectures, côte à côte. C'est la forme qui apprend
        // quelque chose : « voici ce que le listing affiche, voici ce
        // que le processeur lit, et voici l'octet qui les sépare ». Une
        // ligne qui dirait seulement « désalignement en 0x401234 »
        // laisserait tout le travail à faire.
        let mut rangées = vec![vec![
            ctx.t("le listing lit", "the listing reads").into(),
            Fragment::dit(&m.linear).code().into(),
        ]];
        rangées.extend(m.flow.iter().enumerate().map(|(rang, lue)| {
            let texte = Fragment::dit(&lue.text).code();
            vec![
                if rang == 0 {
                    ctx.t("le processeur lit", "the processor reads").into()
                } else {
                    crate::vue::Ligne::default()
                },
                vec![
                    Fragment::dit(format!("{:#010x} ", lue.addr)).code(),
                    // La PREMIÈRE ligne lue par le processeur est celle
                    // qui contredit le listing : c'est elle qu'on
                    // cherche, et les suivantes ne font que remettre le
                    // flot d'aplomb.
                    if rang == 0 {
                        texte.ton(Ton::Mauvais)
                    } else {
                        texte
                    },
                ]
                .into(),
            ]
        }));
        blocs.push(Bloc::Retrait(vec![Bloc::Tableau {
            colonnes: Vec::new(),
            rangées,
            rayé: true,
        }]));
    }
    blocs
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// La famille la plus rare passe devant : c'est elle qu'on est venu
    /// chercher.
    #[test]
    fn the_rarest_family_comes_first() {
        // Beaucoup de tests de nullité, une seule comparaison par XOR.
        let mut bytes = Vec::new();
        for _ in 0..5 {
            bytes.extend_from_slice(&[0x48, 0x85, 0xC0, 0x74, 0x00]); // test rax,rax ; je
        }
        bytes.extend_from_slice(&[0xB9, 0x48, 0x56, 0x55, 0x48]); // mov ecx, "HVUH"
        bytes.extend_from_slice(&[0x33, 0x0C, 0x03]); // xor ecx, [rbx+rax]

        let disasm = namsdec_disasm::disassemble(&bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = PatternsModule::default();
        module.on_binary_loaded(&ctx);

        let mut familles: BTreeMap<Pattern, usize> = BTreeMap::new();
        for finding in &module.found {
            *familles.entry(finding.pattern).or_default() += 1;
        }
        assert_eq!(familles.get(&Pattern::XorCompare), Some(&1));
        assert!(
            familles.get(&Pattern::NullCheck).copied().unwrap_or(0) > 1,
            "les tests de nullité sont nombreux, comme dans un vrai binaire"
        );
        // Et la figure qui compte porte la valeur attendue.
        let xor = module
            .found
            .iter()
            .find(|f| f.pattern == Pattern::XorCompare)
            .expect("la comparaison par XOR");
        assert!(xor.detail.contains("HVUH"), "{}", xor.detail);
    }

    /// Le panneau PLEIN se lit sans être peint — et c'est la seule
    /// façon de le vérifier ici.
    ///
    /// La scène du banc ne montre que le cas vide : le programme
    /// d'épreuve de l'interface ne contient aucune figure, si bien que
    /// la comparaison au pixel ne prouve rien de ce panneau-ci. La
    /// frontière de P7 répond exactement à ce trou — on construit un
    /// code qui porte des figures, et l'on lit ce que le panneau
    /// montrerait.
    #[test]
    fn le_panneau_plein_se_lit_sans_etre_peint() {
        let mut bytes = Vec::new();
        for _ in 0..5 {
            bytes.extend_from_slice(&[0x48, 0x85, 0xC0, 0x74, 0x00]); // test rax,rax ; je
        }
        bytes.extend_from_slice(&[0xB9, 0x48, 0x56, 0x55, 0x48]); // mov ecx, "HVUH"
        bytes.extend_from_slice(&[0x33, 0x0C, 0x03]); // xor ecx, [rbx+rax]

        let disasm = namsdec_disasm::disassemble(&bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = PatternsModule::default();
        module.on_binary_loaded(&ctx);
        let vue = module.vue(&ctx).expect("une vue");

        // Une famille par repli, et le plus rare devant — ce que le
        // panneau promet dans sa première phrase.
        let replis: Vec<(&str, bool, usize)> = vue
            .blocs
            .iter()
            .filter_map(|bloc| match bloc {
                Bloc::Repliable {
                    titre,
                    ouvert,
                    blocs,
                } => Some((titre.fragments[0].texte.as_str(), *ouvert, blocs.len())),
                _ => None,
            })
            .collect();
        // Trois familles, et non deux : ce code porte aussi une
        // constante en texte — c'est le `mov ecx, "HVUH"` qui sert à la
        // comparaison par XOR, et le panneau a raison de la nommer.
        assert_eq!(replis.len(), 3, "les familles trouvées : {replis:?}");
        assert!(
            replis.last().expect("la dernière").0.contains("nullité"),
            "la plus COMMUNE ferme la marche — c'est la règle du panneau, \
             et la seule raison d'être de son classement : {replis:?}"
        );
        assert!(
            replis.iter().all(|(_, ouvert, _)| *ouvert),
            "des familles d'un ou cinq endroits s'ouvrent d'elles-mêmes : {replis:?}"
        );

        // Et les adresses mènent quelque part : une figure qu'on ne
        // peut pas atteindre ne sert à rien.
        let adresses: Vec<u64> = vue
            .blocs
            .iter()
            .filter_map(|bloc| match bloc {
                Bloc::Repliable { blocs, .. } => Some(blocs),
                _ => None,
            })
            .flatten()
            .filter_map(|bloc| match bloc {
                Bloc::Tableau { rangées, .. } => Some(rangées),
                _ => None,
            })
            .flatten()
            .filter_map(|rangée| rangée.first()?.fragments.first()?.mène_à)
            .collect();
        assert!(
            adresses.iter().all(|a| (0x1000..0x1100).contains(a)),
            "les adresses sont celles du code éprouvé : {adresses:#x?}"
        );
        assert!(!adresses.is_empty(), "et il y en a");
    }
}
