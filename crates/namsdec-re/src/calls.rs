use std::collections::BTreeMap;

use crate::{ReContext, ReModule};
use iced_x86::{FlowControl, OpKind};

/// Les sous-routines du programme, et d'où on les appelle.
///
/// Un désassemblage est une suite plate d'instructions ; ce qui en fait un
/// programme lisible, c'est de savoir où commencent les sous-routines et
/// qui les appelle. Les `call` le disent : leur cible est une entrée, leur
/// adresse une référence.
#[derive(Default)]
pub struct CallsModule {
    routines: Vec<Routine>,
    /// Les cibles de saut qui ne sont pas des appels : le début des
    /// boucles et des branches, ce qu'un désassembleur nomme des
    /// étiquettes.
    labels: usize,
}

struct Routine {
    entry: u64,
    callers: Vec<u64>,
}

/// Relève les cibles d'appel et de saut d'un désassemblage.
fn analyse(disasm: &[namsdec_disasm::DecodedInstruction]) -> (Vec<Routine>, usize) {
    let mut calls: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut labels: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    for decoded in disasm {
        let Some(instr) = namsdec_disasm::decode_one(&decoded.bytes, decoded.address) else {
            continue;
        };
        // Une cible immédiate, et non un registre : `call rax` ne dit pas
        // où il va, et personne ne peut le savoir sans exécuter.
        let target = match instr.op0_kind() {
            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                instr.near_branch_target()
            }
            _ => continue,
        };
        match instr.flow_control() {
            FlowControl::Call => calls.entry(target).or_default().push(decoded.address),
            FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch => {
                labels.insert(target);
            }
            _ => {}
        }
    }
    let routines = calls
        .into_iter()
        .map(|(entry, callers)| Routine { entry, callers })
        .collect();
    (routines, labels.len())
}

impl ReModule for CallsModule {
    fn id(&self) -> &'static str {
        "calls"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Sous-routines et références"
        } else {
            "Subroutines and references"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        let (routines, labels) = analyse(ctx.disasm);
        self.routines = routines;
        self.labels = labels;
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.routines.is_empty() {
            ui.label(ctx.t(
                "Aucun appel : tout le programme est d'un seul tenant.",
                "No call: the whole program is one straight line.",
            ));
        }
        for routine in &self.routines {
            ui.horizontal_wrapped(|ui| {
                ui.monospace(format!("{:08x}", routine.entry));
                ui.label(ctx.t("appelée depuis", "called from"));
                for caller in &routine.callers {
                    ui.monospace(format!("{caller:08x}"));
                }
            });
        }
        ui.label(format!(
            "{} {}",
            self.labels,
            ctx.t(
                "cibles de saut (boucles, branches)",
                "jump targets (loops, branches)"
            )
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    #[test]
    fn calls_name_the_subroutines_and_their_callers() {
        // call +5 ; jmp +2 ; nop ; ret  — puis un second appel au même endroit
        let bytes: &[u8] = &[
            0xE8, 0x06, 0x00, 0x00, 0x00, // call 0x40000b
            0xE8, 0x01, 0x00, 0x00, 0x00, // call 0x40000b
            0x90, // nop
            0xC3, // ret (à 0x40000b)
            0xEB, 0xFE, // jmp sur soi-même
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let (routines, labels) = analyse(&disasm);

        assert_eq!(routines.len(), 1, "un seul point d'entrée appelé");
        assert_eq!(routines[0].entry, 0x40000b);
        assert_eq!(
            routines[0].callers,
            vec![0x400000, 0x400005],
            "les deux appelants, dans l'ordre"
        );
        assert_eq!(labels, 1, "le saut sur soi-même est une cible");
    }

    #[test]
    fn an_indirect_call_is_not_invented() {
        // call rax : la cible n'est pas dans l'instruction.
        let bytes: &[u8] = &[0xFF, 0xD0];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let (routines, _) = analyse(&disasm);
        assert!(routines.is_empty());

        let cpu = Cpu::new(Memory::new(0x400000, 0x10));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "en",
        };
        let mut module = CallsModule::default();
        module.on_binary_loaded(&ctx);
        assert!(module.routines.is_empty());
    }
}
