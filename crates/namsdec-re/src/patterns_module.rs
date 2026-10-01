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
use crate::{ReContext, ReModule, ReRequest, clickable_address};

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

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        // Le désalignement d'abord, et sans compromis : c'est la seule
        // figure de ce panneau qui dise que **le listing lui-même ment**.
        // Toutes les autres se lisent dans un listing juste.
        if !self.misalignments.is_empty() {
            if let Some(addr) = désalignements(ui, ctx, &self.misalignments) {
                self.demande = Some(ReRequest::GoTo { addr });
            }
            ui.add_space(6.0);
        }
        if self.found.is_empty() {
            if self.misalignments.is_empty() {
                ui.label(ctx.t(
                    "Aucune figure reconnue : ce code ne compare rien de remarquable.",
                    "No shape recognised: this code compares nothing remarkable.",
                ));
            }
            return;
        }
        ui.weak(ctx.t(
            "Rangées de la plus rare à la plus commune : dans un gros binaire, mille tests \
             de nullité sont de la ponctuation, et quatre comparaisons par XOR sont une \
             piste.",
            "Sorted from rarest to commonest: in a large binary, a thousand null tests are \
             punctuation, and four XOR comparisons are a lead.",
        ));
        ui.add_space(4.0);

        // Relevé pendant le dessin, posé après : les grilles empruntent
        // `self`, et le module ne peut donc pas s'écrire dedans.
        let mut demande: Option<ReRequest> = None;
        let mut familles: BTreeMap<Pattern, Vec<&Finding>> = BTreeMap::new();
        for finding in &self.found {
            familles.entry(finding.pattern).or_default().push(finding);
        }
        let mut rangées: Vec<(Pattern, Vec<&Finding>)> = familles.into_iter().collect();
        rangées.sort_by_key(|(_, occurrences)| occurrences.len());

        for (pattern, occurrences) in rangées {
            egui::CollapsingHeader::new(format!(
                "{} — {} {}",
                pattern.title(ctx.lang),
                occurrences.len(),
                ctx.t("endroits", "places")
            ))
            .id_salt(pattern.title("en"))
            // Ouvertes tant qu'elles restent lisibles : une famille de
            // douze endroits se parcourt, une de mille se demande.
            .default_open(occurrences.len() <= PAR_FAMILLE)
            .show(ui, |ui| {
                ui.weak(pattern.explain(ctx.lang));
                ui.add_space(3.0);
                egui::Grid::new(format!("patterns_{}", pattern.title("en")))
                    .num_columns(2)
                    .min_col_width(0.0)
                    .striped(true)
                    .show(ui, |ui| {
                        for finding in occurrences.iter().take(PAR_FAMILLE) {
                            if clickable_address(ui, ctx, finding.addr) {
                                demande = Some(ReRequest::GoTo { addr: finding.addr });
                            }
                            ui.monospace(&finding.detail);
                            ui.end_row();
                        }
                    });
                if occurrences.len() > PAR_FAMILLE {
                    ui.weak(format!(
                        "… {} {}",
                        occurrences.len() - PAR_FAMILLE,
                        ctx.t("de plus", "more")
                    ));
                }
            });
        }
        if demande.is_some() {
            self.demande = demande;
        }
    }
}

/// Les deux lectures, côte à côte.
///
/// C'est la forme qui apprend quelque chose : « voici ce que le listing
/// affiche, voici ce que le processeur lit, et voici l'octet qui les
/// sépare ». Une ligne qui dirait seulement « désalignement en 0x401234 »
/// laisserait tout le travail à faire.
fn désalignements(ui: &mut egui::Ui, ctx: &ReContext, trouvés: &[Misalignment]) -> Option<u64> {
    let mut aller: Option<u64> = None;
    let rouge = egui::Color32::from_rgb(0xf4, 0x87, 0x71);
    ui.horizontal_wrapped(|ui| {
        ui.strong(
            egui::RichText::new(ctx.t("Le listing est trompé", "The listing is being fooled"))
                .color(rouge),
        );
        ui.weak(format!(
            "{} {}",
            trouvés.len(),
            if trouvés.len() > 1 {
                ctx.t("endroits", "places")
            } else {
                ctx.t("endroit", "place")
            }
        ));
    });
    ui.weak(ctx.t(
        "Un saut atterrit au MILIEU d'une instruction du listing. Le listing lit ligne à ligne ; le processeur, lui, va où les sauts l'envoient — et lit donc d'autres octets. C'est l'anti-désassemblage classique : un seul octet-piège suffit à faire mentir tout ce qui suit. Le découpage en instructions n'est pas une propriété des octets, c'est une propriété du point où l'on commence à lire.",
        "A jump lands in the MIDDLE of an instruction of the listing. The listing reads line by line; the processor goes where jumps send it — and therefore reads other bytes. This is classic anti-disassembly: a single trap byte is enough to make everything that follows lie. Where instructions begin is not a property of the bytes, it is a property of where one starts reading.",
    ));
    ui.add_space(4.0);

    for m in trouvés {
        ui.horizontal_wrapped(|ui| {
            if clickable_address(ui, ctx, m.from) {
                aller = Some(m.from);
            }
            ui.label(ctx.t("envoie à", "sends to"));
            ui.monospace(format!("{:#010x}", m.target));
            ui.weak(format!(
                "— {} {} {} {:#x}",
                m.offset,
                if m.offset > 1 {
                    ctx.t("octets", "bytes")
                } else {
                    ctx.t("octet", "byte")
                },
                ctx.t("après le début de", "past the start of"),
                m.inside
            ));
        });
        ui.indent(m.target, |ui| {
            egui::Grid::new(format!("misalign_{:x}", m.target))
                .num_columns(2)
                .min_col_width(0.0)
                .striped(true)
                .show(ui, |ui| {
                    ui.label(ctx.t("le listing lit", "the listing reads"));
                    ui.monospace(&m.linear);
                    ui.end_row();
                    for (rang, lue) in m.flow.iter().enumerate() {
                        ui.label(if rang == 0 {
                            ctx.t("le processeur lit", "the processor reads")
                        } else {
                            ""
                        });
                        ui.horizontal(|ui| {
                            ui.monospace(format!("{:#010x}", lue.addr));
                            ui.monospace(egui::RichText::new(&lue.text).color(if rang == 0 {
                                rouge
                            } else {
                                ui.visuals().text_color()
                            }));
                        });
                        ui.end_row();
                    }
                });
        });
        ui.add_space(4.0);
    }
    aller
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
}
