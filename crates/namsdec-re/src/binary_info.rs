use crate::{ReContext, ReModule};

/// Module de preuve de concept : taille du binaire, adresse d'entrée et
/// nombre d'instructions décodées. Sert à valider que le mécanisme
/// `ReModule` fonctionne de bout en bout avant que la phase 2 (CFG,
/// xrefs...) ne s'appuie dessus.
#[derive(Default)]
pub struct BinaryInfoModule {
    size: usize,
    entry: u64,
    instruction_count: usize,
}

impl ReModule for BinaryInfoModule {
    fn id(&self) -> &'static str {
        "binary_info"
    }

    fn title(&self) -> &'static str {
        "BinaryInfo"
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.size = ctx.disasm.iter().map(|i| i.bytes.len()).sum();
        self.entry = ctx.disasm.first().map(|i| i.address).unwrap_or(0);
        self.instruction_count = ctx.disasm.len();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _ctx: &ReContext) {
        ui.label(format!("Taille : {} octets", self.size));
        ui.label(format!("Adresse d'entrée : {:#x}", self.entry));
        ui.label(format!("Instructions : {}", self.instruction_count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    #[test]
    fn on_binary_loaded_summarizes_disassembly() {
        let bytes: &[u8] = &[0xB8, 0x00, 0x00, 0x00, 0x00, 0xF4]; // mov eax,0 / hlt
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let cpu = Cpu::new(Memory::new(0x400000, 0x10));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
        };

        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.size, 6);
        assert_eq!(module.entry, 0x400000);
        assert_eq!(module.instruction_count, 2);
    }
}
