use iced_x86::Register;
use namsdec_core::{Reg64, Width};

use crate::EmuError;

/// Associe un registre `iced_x86` (dans une quelconque de ses largeurs) au
/// registre 64 bits `namsdec-core` qui le contient et à sa largeur.
pub fn map_register(reg: Register) -> Result<(Reg64, Width), EmuError> {
    use Register::*;
    let mapped = match reg {
        RAX => (Reg64::Rax, Width::W64),
        EAX => (Reg64::Rax, Width::W32),
        AX => (Reg64::Rax, Width::W16),
        AL => (Reg64::Rax, Width::W8),

        RCX => (Reg64::Rcx, Width::W64),
        ECX => (Reg64::Rcx, Width::W32),
        CX => (Reg64::Rcx, Width::W16),
        CL => (Reg64::Rcx, Width::W8),

        RDX => (Reg64::Rdx, Width::W64),
        EDX => (Reg64::Rdx, Width::W32),
        DX => (Reg64::Rdx, Width::W16),
        DL => (Reg64::Rdx, Width::W8),

        RBX => (Reg64::Rbx, Width::W64),
        EBX => (Reg64::Rbx, Width::W32),
        BX => (Reg64::Rbx, Width::W16),
        BL => (Reg64::Rbx, Width::W8),

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
