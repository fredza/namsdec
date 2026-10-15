use iced_x86::Register;
use namsdec_core::{Reg64, RegXmm, Width};

use crate::EmuError;

/// Associe un registre `iced_x86` (dans une quelconque de ses largeurs) au
/// registre 64 bits `namsdec-core` qui le contient et à sa largeur.
/// AH, BH, CH, DH désignent l'octet *haut* du registre, et non le bas :
/// `map_register` les range avec lui — c'est bien le même registre 64
/// bits, à la même largeur — et c'est cette question-ci qui dit lequel
/// des deux octets lire.
pub fn is_high_byte(reg: Register) -> bool {
    matches!(
        reg,
        Register::AH | Register::BH | Register::CH | Register::DH
    )
}

/// Le registre vectoriel correspondant, s'il en est un.
///
/// `xmm5` et `ymm5` rendent le même : ce sont les mêmes bits, vus sur
/// 128 ou sur 256. Ce qui les distingue — ce qu'il advient de la moitié
/// haute — se décide à l'écriture, pas ici.
///
/// Séparé de `map_register` parce que ce n'est pas la même machine : un
/// registre vectoriel ne se range pas dans un registre général, ne porte
/// pas de largeur au sens de `Width`, et ne sert à aucune des
/// instructions entières.
pub fn map_vector(reg: Register) -> Option<RegXmm> {
    if !reg.is_xmm() && !reg.is_ymm() {
        return None;
    }
    RegXmm::ALL.get(reg.number()).copied()
}

/// Le registre de masque qu'iced nomme, s'il en nomme un.
///
/// `k0` en est un comme les autres ici : c'est seulement employé
/// COMME MASQUE d'une autre instruction qu'il veut dire « aucun
/// masque », et cette lecture-là se fait ailleurs.
pub fn map_mask(reg: Register) -> Option<namsdec_core::RegMask> {
    if !reg.is_k() {
        return None;
    }
    namsdec_core::RegMask::ALL.get(reg.number()).copied()
}

pub fn map_register(reg: Register) -> Result<(Reg64, Width), EmuError> {
    use Register::*;
    let mapped = match reg {
        RAX => (Reg64::Rax, Width::W64),
        EAX => (Reg64::Rax, Width::W32),
        AX => (Reg64::Rax, Width::W16),
        AL => (Reg64::Rax, Width::W8),
        AH => (Reg64::Rax, Width::W8),

        RCX => (Reg64::Rcx, Width::W64),
        ECX => (Reg64::Rcx, Width::W32),
        CX => (Reg64::Rcx, Width::W16),
        CL => (Reg64::Rcx, Width::W8),
        CH => (Reg64::Rcx, Width::W8),

        RDX => (Reg64::Rdx, Width::W64),
        EDX => (Reg64::Rdx, Width::W32),
        DX => (Reg64::Rdx, Width::W16),
        DL => (Reg64::Rdx, Width::W8),
        DH => (Reg64::Rdx, Width::W8),

        RBX => (Reg64::Rbx, Width::W64),
        EBX => (Reg64::Rbx, Width::W32),
        BX => (Reg64::Rbx, Width::W16),
        BL => (Reg64::Rbx, Width::W8),
        BH => (Reg64::Rbx, Width::W8),

        RSP => (Reg64::Rsp, Width::W64),
        ESP => (Reg64::Rsp, Width::W32),
        SP => (Reg64::Rsp, Width::W16),
        SPL => (Reg64::Rsp, Width::W8),

        RBP => (Reg64::Rbp, Width::W64),
        EBP => (Reg64::Rbp, Width::W32),
        BP => (Reg64::Rbp, Width::W16),
        BPL => (Reg64::Rbp, Width::W8),

        RSI => (Reg64::Rsi, Width::W64),
        ESI => (Reg64::Rsi, Width::W32),
        SI => (Reg64::Rsi, Width::W16),
        SIL => (Reg64::Rsi, Width::W8),

        RDI => (Reg64::Rdi, Width::W64),
        EDI => (Reg64::Rdi, Width::W32),
        DI => (Reg64::Rdi, Width::W16),
        DIL => (Reg64::Rdi, Width::W8),

        R8 => (Reg64::R8, Width::W64),
        R8D => (Reg64::R8, Width::W32),
        R8W => (Reg64::R8, Width::W16),
        R8L => (Reg64::R8, Width::W8),

        R9 => (Reg64::R9, Width::W64),
        R9D => (Reg64::R9, Width::W32),
        R9W => (Reg64::R9, Width::W16),
        R9L => (Reg64::R9, Width::W8),

        R10 => (Reg64::R10, Width::W64),
        R10D => (Reg64::R10, Width::W32),
        R10W => (Reg64::R10, Width::W16),
        R10L => (Reg64::R10, Width::W8),

        R11 => (Reg64::R11, Width::W64),
        R11D => (Reg64::R11, Width::W32),
        R11W => (Reg64::R11, Width::W16),
        R11L => (Reg64::R11, Width::W8),

        R12 => (Reg64::R12, Width::W64),
        R12D => (Reg64::R12, Width::W32),
        R12W => (Reg64::R12, Width::W16),
        R12L => (Reg64::R12, Width::W8),

        R13 => (Reg64::R13, Width::W64),
        R13D => (Reg64::R13, Width::W32),
        R13W => (Reg64::R13, Width::W16),
        R13L => (Reg64::R13, Width::W8),

        R14 => (Reg64::R14, Width::W64),
        R14D => (Reg64::R14, Width::W32),
        R14W => (Reg64::R14, Width::W16),
        R14L => (Reg64::R14, Width::W8),

        R15 => (Reg64::R15, Width::W64),
        R15D => (Reg64::R15, Width::W32),
        R15W => (Reg64::R15, Width::W16),
        R15L => (Reg64::R15, Width::W8),

        other => {
            return Err(EmuError::UnsupportedRegister {
                backend: crate::Emulator::NAME,
                register: format!("{other:?}"),
            });
        }
    };
    Ok(mapped)
}
