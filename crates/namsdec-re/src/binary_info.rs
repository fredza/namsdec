use crate::{ReContext, ReModule, elf_scan, strings, xrefs};

/// Le résumé : de quoi on parle, avant d'ouvrir les autres panneaux.
///
/// Six modules détaillent chacun une chose ; celui-ci répond à la
/// question qu'on pose en premier — qu'est-ce que c'est, et qu'est-ce
/// qu'il y a dedans ? Tout ce qu'il montre est mesuré, pas déclaré :
/// les fonctions viennent des `call` relevés, les chaînes des octets
/// lisibles, la nature du fichier de ses en-têtes.
#[derive(Default)]
pub struct BinaryInfoModule {
    size: usize,
    entry: u64,
    instructions: usize,
    functions: usize,
    strings: usize,
    /// Ce que l'en-tête dit du fichier, quand on l'a.
    kind: Option<&'static str>,
    linking: Option<&'static str>,
}

impl ReModule for BinaryInfoModule {
    fn id(&self) -> &'static str {
        "binary_info"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "En bref"
        } else {
            "At a glance"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.size = ctx.disasm.iter().map(|i| i.bytes.len()).sum();
        self.entry = ctx.disasm.first().map(|i| i.address).unwrap_or(0);
        self.instructions = ctx.disasm.len();

        let range = ctx.cpu.mem.range();
        self.functions = xrefs::scan(ctx.disasm, range.clone()).functions().len();
        self.strings = (range.end - range.start)
            .try_into()
            .ok()
            .and_then(|len: usize| ctx.cpu.mem.peek_bytes(range.start, len).ok())
            .map(|bytes| strings::scan(bytes, range.start).len())
            .unwrap_or(0);

        let scan = elf_scan::scan(ctx.binary);
        self.kind = scan
            .as_ref()
            .map(|scan| if scan.pie() { "PIE" } else { "EXEC" });
        self.linking = scan
            .as_ref()
            .map(|scan| if scan.dynamic() { "dyn" } else { "static" });
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        egui::Grid::new("binary_info_grid")
            .num_columns(2)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                let mut ligne = |label: &str, valeur: String| {
                    ui.label(label);
                    ui.monospace(valeur);
                    ui.end_row();
                };
                ligne(
                    ctx.t("Code", "Code"),
                    format!(
                        "{} {} · {} {}",
                        self.size,
                        ctx.t("octets", "bytes"),
                        self.instructions,
                        ctx.t("instructions", "instructions")
                    ),
                );
                ligne(
                    ctx.t("Point d'entrée", "Entry point"),
                    format!("{:#x}", self.entry),
                );
                ligne(
                    ctx.t("Fonctions", "Functions"),
                    format!(
                        "{} ({})",
                        self.functions,
                        ctx.t("cibles de `call`", "`call` targets")
                    ),
                );
                ligne(ctx.t("Chaînes", "Strings"), format!("{}", self.strings));
                if let (Some(kind), Some(linking)) = (self.kind, self.linking) {
                    ligne(ctx.t("Fichier", "File"), format!("{kind} · {linking}"));
                }
            });
        if self.kind.is_none() {
            ui.weak(ctx.t(
                "Pas d'en-tête ELF : ce sont des octets bruts, désassemblés depuis le premier.",
                "No ELF header: these are raw bytes, disassembled from the first one.",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    #[test]
    fn the_summary_counts_what_it_sees() {
        let bytes: &[u8] = &[
            0xe8, 0x01, 0x00, 0x00, 0x00, // call +1
            0x90, // nop  (la cible)
            0xf4, // hlt
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
        };

        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.size, 7);
        assert_eq!(module.entry, 0x400000);
        assert_eq!(module.instructions, 3);
        // Le `call` désigne une adresse : c'est une fonction.
        assert_eq!(module.functions, 1);
        // Pas d'en-tête : on ne prétend rien sur la nature du fichier.
        assert!(module.kind.is_none() && module.linking.is_none());
    }
}
