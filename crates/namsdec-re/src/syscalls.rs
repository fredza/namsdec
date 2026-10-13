use crate::behaviour::{self, Family};
use crate::{ReContext, ReModule, ReSection};
use iced_x86::{Mnemonic, OpKind, Register};

/// Les appels système que le programme contient, avec leur numéro et leur
/// nom.
///
/// Un `syscall` tout seul ne dit rien : c'est le RAX qui le précède qui
/// dit ce qu'il demande au noyau. Le module remonte donc les quelques
/// instructions d'avant pour trouver ce numéro — exactement ce qu'on fait
/// à la main en lisant un désassemblage, et ce qui raconte en une liste
/// ce que le programme fait du monde extérieur.
#[derive(Default)]
pub struct SyscallsModule {
    found: Vec<Call>,
}

/// Un `syscall` du programme : où il est, et ce qu'il demande — quand
/// le numéro se lit d'ici.
pub struct Call {
    pub addr: u64,
    pub number: Option<u64>,
}

impl Call {
    /// La famille de comportement de cet appel, quand son numéro se lit
    /// et que namsdec sait ce qu'il fait.
    pub fn family(&self) -> Option<Family> {
        behaviour::family_of_syscall(self.number?)
    }
}

/// Tous les appels système d'un désassemblage, avec leur numéro quand il
/// est écrit en clair juste avant.
///
/// Sorti du module pour servir ailleurs : la console a besoin de savoir
/// si le programme peut lire quelque chose avant de proposer un champ de
/// saisie à un programme qui ne lira jamais.
pub fn scan(disasm: &[namsdec_disasm::DecodedInstruction]) -> Vec<Call> {
    disasm
        .iter()
        .enumerate()
        .filter(|(_, instr)| instr.text.trim() == "syscall")
        .map(|(index, instr)| Call {
            addr: instr.address,
            number: number_before(disasm, index),
        })
        .collect()
}

/// Le programme peut-il lire son entrée ?
///
/// Vrai s'il contient un `read` (appel numéro zéro), et vrai aussi si un
/// appel a un numéro qu'on ne sait pas lire d'ici : mieux vaut proposer
/// une saisie inutile que la refuser à un programme qui l'attend.
pub fn reads_input(disasm: &[namsdec_disasm::DecodedInstruction]) -> bool {
    scan(disasm)
        .iter()
        .any(|call| matches!(call.number, None | Some(0)))
}

/// Combien d'instructions on remonte pour trouver le numéro. Au-delà,
/// c'est que RAX vient d'ailleurs — d'un calcul, d'un argument — et
/// deviner serait mentir.
const LOOK_BACK: usize = 12;

use crate::table::{is_simulated, syscall_name};

/// Le numéro chargé dans RAX juste avant `index`, s'il est écrit en clair.
fn number_before(disasm: &[namsdec_disasm::DecodedInstruction], index: usize) -> Option<u64> {
    let first = index.saturating_sub(LOOK_BACK);
    for earlier in disasm[first..index].iter().rev() {
        let instr = namsdec_disasm::decode_one(&earlier.bytes, earlier.address)?;
        let writes_rax = instr.op_count() > 0
            && instr.op_kind(0) == OpKind::Register
            && matches!(
                instr.op_register(0),
                Register::RAX | Register::EAX | Register::AX
            );
        if !writes_rax {
            continue;
        }
        // La dernière écriture de RAX avant l'appel décide. Si ce n'est
        // pas un immédiat — un calcul, une lecture mémoire —, le numéro
        // n'est pas lisible ici, et on le dit plutôt que de remonter plus
        // haut vers une valeur périmée.
        if instr.mnemonic() == Mnemonic::Mov && instr.op_count() > 1 {
            return match instr.op_kind(1) {
                OpKind::Immediate8
                | OpKind::Immediate16
                | OpKind::Immediate32
                | OpKind::Immediate64
                | OpKind::Immediate8to16
                | OpKind::Immediate8to32
                | OpKind::Immediate8to64
                | OpKind::Immediate32to64 => Some(instr.immediate(1)),
                _ => None,
            };
        }
        // `xor rax, rax` : l'idiome pour l'appel numéro zéro.
        if instr.mnemonic() == Mnemonic::Xor
            && instr.op_count() > 1
            && instr.op_kind(1) == OpKind::Register
            && instr.op_register(0) == instr.op_register(1)
        {
            return Some(0);
        }
        return None;
    }
    None
}

impl ReModule for SyscallsModule {
    fn section(&self) -> ReSection {
        ReSection::Borrows
    }

    fn id(&self) -> &'static str {
        "syscalls"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Appels système"
        } else {
            "System calls"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.found = scan(ctx.disasm);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.found.is_empty() {
            ui.label(ctx.t(
                "Ce programme ne demande rien au système.",
                "This program asks nothing of the system.",
            ));
            return;
        }
        // Le portrait d'abord, comme pour les imports : la liste répond
        // « quels appels », le portrait répond « pour quoi faire ». Les
        // deux panneaux emploient la même ligne et les mêmes couleurs —
        // un `socket` est un `socket`, qu'il vienne du noyau ou de la
        // table de liaison.
        let comptes: Vec<(Family, usize)> = behaviour::ALL
            .into_iter()
            .filter_map(|famille| {
                let combien = self
                    .found
                    .iter()
                    .filter(|call| call.family() == Some(famille))
                    .count();
                (combien > 0).then_some((famille, combien))
            })
            .collect();
        if !comptes.is_empty() {
            crate::imports_module::portrait(ui, ctx, &comptes);
            ui.add_space(4.0);
        }
        for call in &self.found {
            ui.horizontal(|ui| {
                ui.monospace(format!("{:08x}", call.addr));
                match call.number {
                    None => {
                        ui.label(ctx.t(
                            "numéro calculé — illisible d'ici",
                            "number computed — not readable from here",
                        ));
                    }
                    Some(number) => {
                        ui.monospace(format!("{number}"));
                        ui.label(syscall_name(number).unwrap_or(ctx.t("inconnu", "unknown")));
                        if !is_simulated(number) {
                            ui.weak(ctx.t("(non simulé)", "(not simulated)"));
                        }
                        if let Some(famille) = call.family() {
                            ui.label(
                                egui::RichText::new(famille.label(ctx.lang)).color(famille.color()),
                            )
                            .on_hover_text(famille.why(ctx.lang));
                        }
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn context_of(bytes: &[u8]) -> Vec<namsdec_disasm::DecodedInstruction> {
        namsdec_disasm::disassemble(bytes, 0x400000)
    }

    #[test]
    fn the_number_comes_from_the_rax_just_before() {
        // mov eax, 1 / mov edi, 1 / syscall   puis   xor eax, eax / syscall
        let bytes: &[u8] = &[
            0xB8, 0x01, 0x00, 0x00, 0x00, 0xBF, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x05, 0x31, 0xC0,
            0x0F, 0x05,
        ];
        let disasm = context_of(bytes);
        let cpu = Cpu::new(Memory::new(0x400000, 0x10));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };
        let mut module = SyscallsModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.found.len(), 2);
        assert_eq!(module.found[0].number, Some(1), "write");
        assert_eq!(module.found[1].number, Some(0), "xor rax, rax vaut zéro");
        assert_eq!(syscall_name(1), Some("write"));
        assert!(is_simulated(12) && !is_simulated(59));
    }

    /// Un numéro qui vient d'un calcul ne se devine pas : le module le dit
    /// plutôt que de remonter jusqu'à une valeur qui n'a plus cours.
    #[test]
    fn a_computed_number_is_reported_as_unreadable() {
        // mov eax, 1 / add eax, ebx / syscall
        let bytes: &[u8] = &[0xB8, 0x01, 0x00, 0x00, 0x00, 0x01, 0xD8, 0x0F, 0x05];
        let disasm = context_of(bytes);
        let cpu = Cpu::new(Memory::new(0x400000, 0x10));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };
        let mut module = SyscallsModule::default();
        module.on_binary_loaded(&ctx);
        assert_eq!(module.found[0].number, None);
    }
}
