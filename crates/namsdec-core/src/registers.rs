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

/// Les seize registres vectoriels de SSE, larges de 128 bits.
///
/// C'est là que se fait tout le calcul flottant du x86-64 moderne : la
/// pile du 8087 n'est plus utilisée que par du code ancien, et l'ABI
/// System V passe les flottants dans XMM0 à XMM7.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum RegXmm {
    Xmm0,
    Xmm1,
    Xmm2,
    Xmm3,
    Xmm4,
    Xmm5,
    Xmm6,
    Xmm7,
    Xmm8,
    Xmm9,
    Xmm10,
    Xmm11,
    Xmm12,
    Xmm13,
    Xmm14,
    Xmm15,
}

impl RegXmm {
    pub const ALL: [RegXmm; 16] = [
        RegXmm::Xmm0,
        RegXmm::Xmm1,
        RegXmm::Xmm2,
        RegXmm::Xmm3,
        RegXmm::Xmm4,
        RegXmm::Xmm5,
        RegXmm::Xmm6,
        RegXmm::Xmm7,
        RegXmm::Xmm8,
        RegXmm::Xmm9,
        RegXmm::Xmm10,
        RegXmm::Xmm11,
        RegXmm::Xmm12,
        RegXmm::Xmm13,
        RegXmm::Xmm14,
        RegXmm::Xmm15,
    ];

    /// Le numéro du registre, tel qu'il s'écrit : `xmm5` pour 5.
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, Default)]
pub struct Registers {
    gpr: [u64; 16],
    /// Les registres SSE. Ils ne se mélangent pas aux généraux : ni les
    /// mêmes instructions, ni les mêmes tailles, ni le même usage.
    xmm: [u128; 16],
    pub rip: u64,
}

impl Registers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Un registre SSE entier — 128 bits, deux flottants doubles ou
    /// quatre simples selon ce qu'on y a mis.
    pub fn get_xmm(&self, r: RegXmm) -> u128 {
        self.xmm[r as usize]
    }

    pub fn set_xmm(&mut self, r: RegXmm, v: u128) {
        self.xmm[r as usize] = v;
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

    /// L'octet *haut* du registre bas — AH, BH, CH, DH : les bits 8 à 15.
    ///
    /// Un vestige du 8086, qu'une instruction ne peut atteindre que sans
    /// préfixe REX : `mov ah, 1` s'assemble, `mov ah, r8b` non.
    pub fn get8h(&self, r: Reg64) -> u8 {
        (self.gpr[r as usize] >> 8) as u8
    }

    /// Écrire l'octet haut préserve le reste du registre 64 bits.
    pub fn set8h(&mut self, r: Reg64, v: u8) {
        let slot = &mut self.gpr[r as usize];
        *slot = (*slot & !0xFF00) | ((v as u64) << 8);
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
