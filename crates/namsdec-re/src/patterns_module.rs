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

use crate::patterns::{Finding, Pattern, scan};
use crate::{ReContext, ReModule};

#[derive(Default)]
pub struct PatternsModule {
    found: Vec<Finding>,
}

/// Combien d'endroits on montre par famille.
///
/// Au-delà, la liste cesse d'être une piste pour devenir un annuaire —
/// et le compte total, lui, reste affiché.
const PAR_FAMILLE: usize = 12;

impl ReModule for PatternsModule {
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
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.found.is_empty() {
            ui.label(ctx.t(
                "Aucune figure reconnue : ce code ne compare rien de remarquable.",
                "No shape recognised: this code compares nothing remarkable.",
            ));
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
                            ui.monospace(format!("{:#010x}", finding.addr));
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
    }
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
