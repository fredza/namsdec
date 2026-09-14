use crate::flags::RFlags;
use crate::memory::Memory;
use crate::registers::Registers;

/// État complet du CPU simulé : registres, indicateurs, mémoire et
/// statut d'arrêt (`hlt` ou erreur d'exécution).
pub struct Cpu {
    pub regs: Registers,
    pub flags: RFlags,
    pub mem: Memory,
    pub halted: bool,
}

impl Cpu {
    pub fn new(mem: Memory) -> Self {
        Self {
            regs: Registers::new(),
            flags: RFlags::empty(),
            mem,
            halted: false,
        }
    }
}
