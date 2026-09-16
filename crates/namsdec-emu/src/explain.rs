//! Ce que l'instruction suivante va manipuler, avec les valeurs du moment.
//!
//! Un débutant lit `mov rax, [rsi + rcx*8 + 16]` et ne sait pas *où* cela
//! pointe : il faut tenir trois registres de tête et faire le calcul. Le
//! processeur, lui, l'a déjà fait. Ce module refait ce calcul avant
//! l'exécution et rend, opérande par opérande, la valeur que
//! l'instruction va lire — l'adresse effective comprise, et ce qui s'y
//! trouve.
//!
//! Aucune langue ici : les lignes sont des couples (ce qui est écrit dans
//! le code, ce que ça vaut maintenant). Seul l'en-tête du panneau est
//! traduit.

use iced_x86::{Formatter, Instruction, NasmFormatter, OpKind, Register};
use namsdec_core::Cpu;

/// Une ligne du panneau : l'opérande tel qu'il est écrit, et sa valeur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// L'opérande, en syntaxe NASM : `rsi`, `[rsi+8]`, `60`.
    pub text: String,
    /// Sa valeur maintenant : `0x402000`, `96`, ou l'adresse calculée
    /// suivie de ce qu'elle contient.
    pub value: String,
    /// L'adresse que cette ligne désigne, quand elle en désigne une :
    /// l'adresse effective d'un opérande mémoire, la cible d'un saut.
    /// L'interface la rapproche des étiquettes du programme.
    pub addr: Option<u64>,
}

/// Les opérandes de `instr`, résolus sur l'état actuel du processeur.
///
/// Les opérandes implicites — le `rax` de `mul rcx`, le `rcx` d'un `loop`
/// — n'y figurent pas : `iced` ne les compte pas comme opérandes, et le
/// panneau montre ce qui est *écrit*, pas la sémantique complète. Celle-ci
/// est dans la fiche de l'instruction, juste au-dessus.
pub fn operands(cpu: &Cpu, instr: &Instruction) -> Vec<Line> {
    let mut formatter = NasmFormatter::new();
    let mut lines = Vec::new();
    for index in 0..instr.op_count() {
        let mut text = String::new();
        // Un opérande que le formateur refuse (numéro hors de l'encodage)
        // n'a rien à montrer : on passe au suivant.
        if formatter.format_operand(instr, &mut text, index).is_err() {
            continue;
        }
        let mut addr = None;
        let value = match instr.op_kind(index) {
            OpKind::Register => register_value(cpu, instr.op_register(index)),
            OpKind::Memory => {
                addr = crate::effective_address(&cpu.regs, instr, index);
                memory_value(cpu, instr, index)
            }
            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                addr = Some(instr.near_branch_target());
                Some(format!("{:#x}", instr.near_branch_target()))
            }
            // Une immédiate est déjà sous les yeux ; sa forme décimale ne
            // l'est pas toujours (`0x1b6`, c'est 438 — les droits d'un
            // fichier). On ne la répète que si elle se lit autrement.
            OpKind::Immediate8
            | OpKind::Immediate8_2nd
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate64
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate32to64 => immediate_value(instr, index, &text),
            _ => None,
        };
        if let Some(value) = value {
            lines.push(Line { text, value, addr });
        }
    }
    lines
}

/// La valeur d'un registre, sur sa propre largeur : `eax` vaut huit
/// chiffres, pas seize. Montrer seize donnerait à croire qu'écrire dans
/// `eax` touche la moitié haute — c'est exactement l'inverse qui est
/// vrai, et le panneau ne doit pas suggérer le contraire.
fn register_value(cpu: &Cpu, reg: Register) -> Option<String> {
    if reg.is_xmm() || reg.is_ymm() || reg.is_zmm() {
        // Les vectoriels ont leur propre bandeau, avec leur lecture
        // flottante : le répéter ici en hexadécimal n'apprendrait rien.
        return None;
    }
    let value = crate::register_value(&cpu.regs, reg)?;
    let digits = (reg.size() * 2).max(2);
    let mut shown = format!("{value:#0width$x}", width = digits + 2);
    // La lecture décimale à côté, quand elle dit autre chose : `0x3c`
    // c'est 60, le numéro de `exit`, et c'est ce numéro-là qu'on cherche.
    if value > 9 {
        shown.push_str(&format!(" ({value})"));
    }
    Some(shown)
}

/// L'adresse que désigne l'opérande, puis ce qu'on y trouve.
fn memory_value(cpu: &Cpu, instr: &Instruction, index: u32) -> Option<String> {
    let addr = crate::effective_address(&cpu.regs, instr, index)?;
    let size = instr.memory_size().size();
    let mut shown = format!("{addr:#x}");
    // `peek_bytes` : montrer ce qu'il y a là ne doit pas dépendre du
    // droit de lecture — le panneau observe, il n'exécute pas.
    if matches!(size, 1 | 2 | 4 | 8)
        && let Ok(bytes) = cpu.mem.peek_bytes(addr, size)
    {
        let mut buf = [0u8; 8];
        buf[..size].copy_from_slice(bytes);
        let value = u64::from_le_bytes(buf);
        shown.push_str(&format!(" -> {:#0width$x}", value, width = size * 2 + 2));
    }
    Some(shown)
}

/// L'immédiate, en décimal — et seulement si le code l'a écrite
/// autrement : redire « 5 » sous un `5` ne sert à rien.
fn immediate_value(instr: &Instruction, index: u32, text: &str) -> Option<String> {
    let raw = instr.try_immediate(index).ok()?;
    let signed = raw as i64;
    let decimal = signed.to_string();
    if text.trim() == decimal {
        return None;
    }
    Some(decimal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Memory, Reg64};

    fn cpu_with(program: &[u8]) -> Cpu {
        let mut mem = Memory::new(0x400000, 0x2000);
        mem.load(0x400000, program).unwrap();
        Cpu::new(mem)
    }

    #[test]
    fn a_register_operand_shows_its_current_value() {
        // mov rax, rcx
        let mut cpu = cpu_with(&[0x48, 0x89, 0xC8]);
        cpu.regs.set64(Reg64::Rax, 0x11);
        cpu.regs.set64(Reg64::Rcx, 0x3c);
        let instr = namsdec_disasm::decode_one(&[0x48, 0x89, 0xC8], 0x400000).unwrap();
        let lines = operands(&cpu, &instr);
        assert_eq!(lines[0].text, "rax");
        assert_eq!(lines[0].value, "0x0000000000000011 (17)");
        assert_eq!(lines[1].text, "rcx");
        assert!(lines[1].value.ends_with("(60)"), "{:?}", lines[1]);
    }

    /// Le cœur du panneau : l'adresse calculée, que personne ne veut
    /// faire de tête, et l'octet qui s'y trouve.
    #[test]
    fn a_memory_operand_shows_where_it_points_and_what_is_there() {
        // mov al, [rsi+rcx*4+8]
        let bytes = [0x8A, 0x44, 0x8E, 0x08];
        let mut cpu = cpu_with(&bytes);
        cpu.regs.set64(Reg64::Rsi, 0x401000);
        cpu.regs.set64(Reg64::Rcx, 2);
        cpu.mem.poke(0x401010, &[0x7f]).unwrap();
        let instr = namsdec_disasm::decode_one(&bytes, 0x400000).unwrap();
        let lines = operands(&cpu, &instr);
        assert_eq!(lines[1].value, "0x401010 -> 0x7f", "{:?}", lines[1]);
    }

    /// Une immédiate écrite en hexadécimal se relit en décimal ; une
    /// immédiate déjà décimale ne se répète pas.
    #[test]
    fn an_immediate_is_repeated_only_when_it_reads_differently() {
        // mov eax, 60
        let bytes = [0xB8, 0x3C, 0x00, 0x00, 0x00];
        let cpu = cpu_with(&bytes);
        let instr = namsdec_disasm::decode_one(&bytes, 0x400000).unwrap();
        let lines = operands(&cpu, &instr);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[1].text, "3Ch");
        assert_eq!(lines[1].value, "60");
    }

    /// Un saut dit où il mène, sans qu'on aille chercher l'étiquette.
    #[test]
    fn a_jump_shows_its_target() {
        // jnz $-6
        let bytes = [0x75, 0xF8];
        let cpu = cpu_with(&bytes);
        let instr = namsdec_disasm::decode_one(&bytes, 0x400010).unwrap();
        let lines = operands(&cpu, &instr);
        assert_eq!(lines[0].value, "0x40000a");
    }
}
