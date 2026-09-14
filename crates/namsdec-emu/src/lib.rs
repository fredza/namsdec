//! Interpréteur x86-64 "maison" : le décodage octets -> instruction
//! structurée est délégué à `namsdec-disasm` (donc à `iced-x86`), mais toute
//! la sémantique d'exécution (effets sur registres/flags/mémoire) est
//! écrite ici, à la main, pour rester transparente et pédagogique.

mod regmap;

use std::collections::BTreeSet;

use iced_x86::{Instruction, Mnemonic, OpKind, Register};
use namsdec_core::{add_with_flags, logic_flags, sub_with_flags, Cpu, Memory, RFlags, Width};
use thiserror::Error;

use regmap::map_register;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Error)]
pub enum EmuError {
    #[error("instruction non supportée par l'interpréteur : {0}")]
    Unsupported(String),
    #[error("erreur mémoire : {0}")]
    Mem(#[from] namsdec_core::MemError),
    #[error("décodage invalide à l'adresse {addr:#x}")]
    InvalidInstruction { addr: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Continued,
    Halted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStop {
    Halted,
    Breakpoint(u64),
    StepLimit,
}

pub struct Emulator {
    pub cpu: Cpu,
    pub breakpoints: BTreeSet<u64>,
}

impl Emulator {
    pub fn new(cpu: Cpu) -> Self {
        Self {
            cpu,
            breakpoints: BTreeSet::new(),
        }
    }

    /// Décode puis exécute l'instruction à RIP.
    pub fn step(&mut self) -> Result<StepResult, EmuError> {
        if self.cpu.halted {
            return Ok(StepResult::Halted);
        }

        let rip = self.cpu.regs.rip;
        let window = fetch_window(&self.cpu.mem, rip)?.to_vec();
        let instr = namsdec_disasm::decode_one(&window, rip)
            .ok_or(EmuError::InvalidInstruction { addr: rip })?;

        // Par défaut RIP avance à l'instruction suivante ; les branchements
        // écrasent cette valeur pendant `execute`.
        self.cpu.regs.rip = instr.next_ip();
        self.execute(&instr)?;

        Ok(if self.cpu.halted {
            StepResult::Halted
        } else {
            StepResult::Continued
        })
    }

    /// Exécute jusqu'à `max_steps` instructions, jusqu'à `hlt`, un point
    /// d'arrêt (autre que celui de départ) ou la limite de pas.
    pub fn run(&mut self, max_steps: u32) -> Result<RunStop, EmuError> {
        for _ in 0..max_steps {
            if self.cpu.halted {
                return Ok(RunStop::Halted);
            }
            self.step()?;
            if self.cpu.halted {
                return Ok(RunStop::Halted);
            }
            if self.breakpoints.contains(&self.cpu.regs.rip) {
                return Ok(RunStop::Breakpoint(self.cpu.regs.rip));
            }
        }
        Ok(RunStop::StepLimit)
    }

    fn execute(&mut self, instr: &Instruction) -> Result<(), EmuError> {
        use Mnemonic as M;
        match instr.mnemonic() {
            M::Mov => {
                let v = self.read_op(instr, 1)?;
                self.write_op(instr, 0, v)?;
            }
            M::Add => self.binary_arith(instr, add_with_flags, true)?,
            M::Sub => self.binary_arith(instr, sub_with_flags, true)?,
            M::Cmp => self.binary_arith(instr, sub_with_flags, false)?,
            M::And => self.binary_logic(instr, |a, b| a & b, true)?,
            M::Or => self.binary_logic(instr, |a, b| a | b, true)?,
            M::Xor => self.binary_logic(instr, |a, b| a ^ b, true)?,
            M::Test => self.binary_logic(instr, |a, b| a & b, false)?,
            M::Inc => self.inc_dec(instr, true)?,
            M::Dec => self.inc_dec(instr, false)?,
            M::Push => {
                let v = self.read_op(instr, 0)?;
                let new_rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp).wrapping_sub(8);
                self.cpu.mem.write_u64(new_rsp, v)?;
                self.cpu.regs.set64(namsdec_core::Reg64::Rsp, new_rsp);
            }
            M::Pop => {
                let rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);
                let v = self.cpu.mem.read_u64(rsp)?;
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rsp.wrapping_add(8));
                self.write_op(instr, 0, v)?;
            }
            M::Lea => {
                let addr = self.effective_address(instr, 1)?;
                self.write_op(instr, 0, addr)?;
            }
            M::Nop => {}
            M::Jmp => {
                self.cpu.regs.rip = instr.near_branch_target();
            }
            M::Je | M::Jne | M::Jl | M::Jle | M::Jg | M::Jge | M::Jb | M::Jbe | M::Ja
            | M::Jae => {
                if self.condition_met(instr.mnemonic()) {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }
            M::Hlt => {
                self.cpu.halted = true;
            }
            other => {
                return Err(EmuError::Unsupported(format!("{other:?}")));
            }
        }
        Ok(())
    }

    fn condition_met(&self, mnemonic: Mnemonic) -> bool {
        let f = &self.cpu.flags;
        let zf = f.contains(RFlags::ZF);
        let sf = f.contains(RFlags::SF);
        let cf = f.contains(RFlags::CF);
        let of = f.contains(RFlags::OF);
        match mnemonic {
            Mnemonic::Je => zf,
            Mnemonic::Jne => !zf,
            Mnemonic::Jb => cf,
            Mnemonic::Jae => !cf,
            Mnemonic::Jbe => cf || zf,
            Mnemonic::Ja => !cf && !zf,
            Mnemonic::Jl => sf != of,
            Mnemonic::Jge => sf == of,
            Mnemonic::Jle => zf || (sf != of),
            Mnemonic::Jg => !zf && (sf == of),
            _ => unreachable!("condition_met appelé pour un mnémonique non conditionnel"),
        }
    }

    fn binary_arith(
        &mut self,
        instr: &Instruction,
        op: impl Fn(u64, u64, Width) -> (u64, RFlags),
        write_result: bool,
    ) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let a = self.read_op(instr, 0)?;
        let b = self.read_op(instr, 1)?;
        let (result, flags) = op(a, b, w);
        self.cpu.flags.apply_arith(flags);
        if write_result {
            self.write_op(instr, 0, result)?;
        }
        Ok(())
    }

    fn binary_logic(
        &mut self,
        instr: &Instruction,
        op: impl Fn(u64, u64) -> u64,
        write_result: bool,
    ) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let a = self.read_op(instr, 0)?;
        let b = self.read_op(instr, 1)?;
        let result = op(a, b);
        self.cpu.flags.apply_arith(logic_flags(result, w));
        if write_result {
            self.write_op(instr, 0, result)?;
        }
        Ok(())
    }

    /// INC/DEC affectent OF/SF/ZF/AF/PF comme add/sub de 1, mais
    /// préservent CF (comportement x86 réel).
    fn inc_dec(&mut self, instr: &Instruction, increment: bool) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let a = self.read_op(instr, 0)?;
        let (result, flags) = if increment {
            add_with_flags(a, 1, w)
        } else {
            sub_with_flags(a, 1, w)
        };
        let cf = self.cpu.flags.contains(RFlags::CF);
        self.cpu.flags.apply_arith(flags);
        self.cpu.flags.set(RFlags::CF, cf);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    fn dest_width(&self, instr: &Instruction) -> Result<Width, EmuError> {
        match instr.op_kind(0) {
            OpKind::Register => Ok(map_register(instr.op_register(0))?.1),
            OpKind::Memory => self.mem_operand_width(instr),
            other => Err(EmuError::Unsupported(format!(
                "opérande destination non supportée : {other:?}"
            ))),
        }
    }

    fn mem_operand_width(&self, instr: &Instruction) -> Result<Width, EmuError> {
        width_from_bytes(instr.memory_size().size()).ok_or_else(|| {
            EmuError::Unsupported(format!(
                "taille d'opérande mémoire non supportée : {} octets",
                instr.memory_size().size()
            ))
        })
    }

    fn read_op(&self, instr: &Instruction, op_index: u32) -> Result<u64, EmuError> {
        match instr.op_kind(op_index) {
            OpKind::Register => self.get_reg(instr.op_register(op_index)),
            OpKind::Memory => {
                let addr = self.effective_address(instr, op_index)?;
                let w = self.mem_operand_width(instr)?;
                self.read_mem(addr, w)
            }
            OpKind::Immediate8
            | OpKind::Immediate16
            | OpKind::Immediate32
            | OpKind::Immediate64
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64
            | OpKind::Immediate32to64 => Ok(instr.immediate(op_index)),
            other => Err(EmuError::Unsupported(format!(
                "lecture d'un opérande {other:?} non supportée"
            ))),
        }
    }

    fn write_op(&mut self, instr: &Instruction, op_index: u32, value: u64) -> Result<(), EmuError> {
        match instr.op_kind(op_index) {
            OpKind::Register => self.set_reg(instr.op_register(op_index), value),
            OpKind::Memory => {
                let addr = self.effective_address(instr, op_index)?;
                let w = self.mem_operand_width(instr)?;
                self.write_mem(addr, w, value)
            }
            other => Err(EmuError::Unsupported(format!(
                "écriture vers un opérande {other:?} non supportée"
            ))),
        }
    }

    fn get_reg(&self, reg: Register) -> Result<u64, EmuError> {
        let (r, w) = map_register(reg)?;
        Ok(match w {
            Width::W64 => self.cpu.regs.get64(r),
            Width::W32 => self.cpu.regs.get32(r) as u64,
            Width::W16 => self.cpu.regs.get16(r) as u64,
            Width::W8 => self.cpu.regs.get8l(r) as u64,
        })
    }

    fn set_reg(&mut self, reg: Register, value: u64) -> Result<(), EmuError> {
        let (r, w) = map_register(reg)?;
        match w {
            Width::W64 => self.cpu.regs.set64(r, value),
            Width::W32 => self.cpu.regs.set32(r, value as u32),
            Width::W16 => self.cpu.regs.set16(r, value as u16),
            Width::W8 => self.cpu.regs.set8l(r, value as u8),
        }
        Ok(())
    }

    fn effective_address(&self, instr: &Instruction, op_index: u32) -> Result<u64, EmuError> {
        let next_ip = instr.next_ip();
        instr
            .virtual_address(op_index, 0, |register, _idx, _size| {
                if register == Register::None {
                    Some(0)
                } else if register == Register::RIP {
                    Some(next_ip)
                } else {
                    map_register(register).ok().map(|(r, _)| self.cpu.regs.get64(r))
                }
            })
            .ok_or(EmuError::InvalidInstruction { addr: instr.ip() })
    }

    fn read_mem(&self, addr: u64, w: Width) -> Result<u64, EmuError> {
        Ok(match w {
            Width::W8 => self.cpu.mem.read_u8(addr)? as u64,
            Width::W16 => self.cpu.mem.read_u16(addr)? as u64,
            Width::W32 => self.cpu.mem.read_u32(addr)? as u64,
            Width::W64 => self.cpu.mem.read_u64(addr)?,
        })
    }

    fn write_mem(&mut self, addr: u64, w: Width, value: u64) -> Result<(), EmuError> {
        match w {
            Width::W8 => self.cpu.mem.write_u8(addr, value as u8)?,
            Width::W16 => self.cpu.mem.write_u16(addr, value as u16)?,
            Width::W32 => self.cpu.mem.write_u32(addr, value as u32)?,
            Width::W64 => self.cpu.mem.write_u64(addr, value)?,
        }
        Ok(())
    }
}

fn width_from_bytes(bytes: usize) -> Option<Width> {
    match bytes {
        1 => Some(Width::W8),
        2 => Some(Width::W16),
        4 => Some(Width::W32),
        8 => Some(Width::W64),
        _ => None,
    }
}

fn fetch_window(mem: &Memory, addr: u64) -> Result<&[u8], EmuError> {
    let range = mem.range();
    if addr < range.start || addr >= range.end {
        return Err(EmuError::InvalidInstruction { addr });
    }
    let avail = (range.end - addr).min(16) as usize;
    mem.bytes_at(addr, avail)
        .map_err(|_| EmuError::InvalidInstruction { addr })
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::Reg64;

    const BASE: u64 = 0x400000;

    /// Assemble `asm` avec `namsdec-asm`, le charge en mémoire et exécute
    /// jusqu'à l'arrêt (halt, point d'arrêt ou limite de pas). Test
    /// d'intégration "golden" : il passe par le vrai `nasm` externe.
    fn run_program(asm: &str, breakpoints: &[u64]) -> Emulator {
        let out = namsdec_asm::assemble(asm, BASE).expect("assemblage (nasm requis)");
        let mut mem = Memory::new(BASE, 0x10000);
        mem.load(BASE, &out.bytes).unwrap();

        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = BASE;
        cpu.regs.set64(Reg64::Rsp, BASE + 0x8000);

        let mut emu = Emulator::new(cpu);
        emu.breakpoints.extend(breakpoints.iter().copied());
        emu.run(10_000).expect("exécution");
        emu
    }

    const SUM_LOOP_ASM: &str = "\
BITS 64
org 0x400000
    mov rax, 0
    mov rcx, 5
sum_loop:
    add rax, rcx
    dec rcx
    jnz sum_loop
    hlt
";

    #[test]
    fn sum_loop_computes_15_and_halts() {
        let emu = run_program(SUM_LOOP_ASM, &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 15);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0);
        assert!(emu.cpu.flags.contains(RFlags::ZF));
    }

    #[test]
    fn breakpoint_stops_before_halt() {
        // sum_loop: est à 0x40000A (voir namsdec-asm::tests::assembles_sum_loop_*)
        let emu = run_program(SUM_LOOP_ASM, &[0x40000A]);
        assert!(!emu.cpu.halted);
        assert_eq!(emu.cpu.regs.rip, 0x40000A);
    }

    #[test]
    fn push_pop_round_trip() {
        let asm = "\
BITS 64
org 0x400000
    mov rax, 0x1234
    push rax
    mov rax, 0
    pop rbx
    hlt
";
        let emu = run_program(asm, &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 0x1234);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
    }

    #[test]
    fn lea_computes_address_without_touching_memory() {
        let asm = "\
BITS 64
org 0x400000
    mov rbx, 0x400000
    lea rax, [rbx + 0x10]
    hlt
";
        let emu = run_program(asm, &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0x400010);
    }

    #[test]
    fn inc_dec_preserve_carry_flag() {
        let asm = "\
BITS 64
org 0x400000
    mov rax, 0xFFFFFFFFFFFFFFFF
    add rax, 2       ; positionne CF=1
    inc rbx          ; ne doit pas modifier CF
    hlt
";
        let emu = run_program(asm, &[]);
        assert!(emu.cpu.halted);
        assert!(emu.cpu.flags.contains(RFlags::CF));
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 1);
    }
}
