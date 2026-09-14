/// Les 16 registres généraux x86-64, dans l'ordre d'encodage ModRM/REX
/// (0=RAX .. 7=RDI, 8=R8 .. 15=R15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum Reg64 {
    Rax,
    Rcx,
    Rdx,
    Rbx,
    Rsp,
    Rbp,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

#[derive(Clone, Debug, Default)]
pub struct Registers {
    gpr: [u64; 16],
    pub rip: u64,
}

impl Registers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get64(&self, r: Reg64) -> u64 {
        self.gpr[r as usize]
    }

    pub fn get32(&self, r: Reg64) -> u32 {
        self.gpr[r as usize] as u32
    }

    pub fn get16(&self, r: Reg64) -> u16 {
        self.gpr[r as usize] as u16
    }

    pub fn get8l(&self, r: Reg64) -> u8 {
        self.gpr[r as usize] as u8
    }

    pub fn set64(&mut self, r: Reg64, v: u64) {
        self.gpr[r as usize] = v;
    }

    /// Écrire un registre 32 bits met à zéro les 32 bits hauts du registre
    /// 64 bits qui le contient — c'est le comportement réel de x86-64,
    /// source classique de confusion pour qui apprend l'assembleur.
    pub fn set32(&mut self, r: Reg64, v: u32) {
        self.gpr[r as usize] = v as u64;
    }

    /// Écrire un registre 16 bits préserve le reste du registre 64 bits
    /// (comportement historique hérité du x86 16 bits).
    pub fn set16(&mut self, r: Reg64, v: u16) {
        let slot = &mut self.gpr[r as usize];
        *slot = (*slot & !0xFFFF) | v as u64;
    }

    /// Écrire l'octet bas préserve le reste du registre 64 bits.
    pub fn set8l(&mut self, r: Reg64, v: u8) {
        let slot = &mut self.gpr[r as usize];
        *slot = (*slot & !0xFF) | v as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set32_zero_extends_upper_bits() {
        let mut regs = Registers::new();
        regs.set64(Reg64::Rax, 0xDEAD_BEEF_0000_0001);
        regs.set32(Reg64::Rax, 0x1234_5678);
        assert_eq!(regs.get64(Reg64::Rax), 0x0000_0000_1234_5678);
    }

    #[test]
    fn set16_preserves_upper_bits() {
        let mut regs = Registers::new();
        regs.set64(Reg64::Rax, 0x1122_3344_5566_7788);
        regs.set16(Reg64::Rax, 0xBEEF);
        assert_eq!(regs.get64(Reg64::Rax), 0x1122_3344_5566_BEEF);
    }

    #[test]
    fn set8l_preserves_upper_bits() {
        let mut regs = Registers::new();
        regs.set64(Reg64::Rax, 0x1122_3344_5566_7788);
        regs.set8l(Reg64::Rax, 0xAB);
        assert_eq!(regs.get64(Reg64::Rax), 0x1122_3344_5566_77AB);
    }

    #[test]
    fn rax_and_r8_are_independent_slots() {
        let mut regs = Registers::new();
        regs.set64(Reg64::Rax, 1);
        regs.set64(Reg64::R8, 2);
        assert_eq!(regs.get64(Reg64::Rax), 1);
        assert_eq!(regs.get64(Reg64::R8), 2);
    }
}
