//! Le coprocesseur 8087 : une pile de huit cases, et des comparaisons
//! qui répondent dans leur propre mot d'état.
//!
//! C'est la machine que décrivent les cours d'assembleur écrits avant
//! SSE, et elle ne ressemble à rien d'autre dans le processeur : pas de
//! registres nommés, pas d'indicateurs partagés, et un format de 80 bits
//! qu'aucun type de Rust ne porte — d'où les deux conversions au bas de
//! ce fichier.

use iced_x86::{Instruction, MemorySize, Mnemonic, OpKind, Register};
use namsdec_core::{RFlags, Width, X87};

use crate::vector::FpOp;
use crate::{EmuError, Emulator, OperandRole, Simplification, round_even, to_integer};

/// L'indice de la dernière opérande, s'il y en a une. Le décodeur rend
/// explicites les opérandes implicites du x87 — `fcom st1` devient
/// `fcom st0, st1` —, et c'est toujours la dernière qui porte
/// l'information.
fn last_operand(instr: &Instruction) -> Option<u32> {
    instr.op_count().checked_sub(1)
}

/// L'indice de la case de pile qu'une opérande désigne, si c'en est une :
/// `st0` rend 0, `st3` rend 3.
fn st_operand(instr: &Instruction, op_index: u32) -> Option<usize> {
    if op_index >= instr.op_count() || instr.op_kind(op_index) != OpKind::Register {
        return None;
    }
    let reg = instr.op_register(op_index);
    (Register::ST0..=Register::ST7)
        .contains(&reg)
        .then(|| reg.number())
}

/// Le format étendu du x87 : signe, exposant sur quinze bits biaisé de
/// 16383, et une mantisse de 64 bits dont le bit de poids fort est
/// *explicite* — contrairement aux formats 32 et 64 bits de l'IEEE, où il
/// est sous-entendu. C'est cette différence qui rend la conversion moins
/// évidente qu'un simple décalage.
fn f80_to_f64(bytes: &[u8]) -> f64 {
    let mut mantissa_bytes = [0u8; 8];
    mantissa_bytes.copy_from_slice(&bytes[0..8]);
    let mantissa = u64::from_le_bytes(mantissa_bytes);
    let sign_exp = u16::from_le_bytes([bytes[8], bytes[9]]);
    let sign = if sign_exp & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from(sign_exp & 0x7fff);
    if exponent == 0x7fff {
        return if mantissa << 1 == 0 {
            sign * f64::INFINITY
        } else {
            f64::NAN
        };
    }
    // La mantisse vaut son entier multiplié par 2 puissance (exposant
    // moins le biais moins les 63 bits de partie fractionnaire).
    sign * (mantissa as f64) * f64::from(exponent - 16383 - 63).exp2()
}

/// L'écriture inverse.
fn f64_to_f80(value: f64) -> [u8; 10] {
    let mut out = [0u8; 10];
    if value == 0.0 {
        if value.is_sign_negative() {
            out[9] = 0x80;
        }
        return out;
    }
    let sign: u16 = if value.is_sign_negative() { 0x8000 } else { 0 };
    if value.is_nan() || value.is_infinite() {
        out[0..8].copy_from_slice(&if value.is_nan() {
            0xc000_0000_0000_0000u64.to_le_bytes()
        } else {
            0x8000_0000_0000_0000u64.to_le_bytes()
        });
        out[8..10].copy_from_slice(&(sign | 0x7fff).to_le_bytes());
        return out;
    }
    let bits = value.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if raw_exponent == 0 {
        // Dénormalisé : on le normalise en décalant, et l'exposant suit.
        let shift = fraction.leading_zeros();
        (fraction << shift, -1022 - (shift as i32 - 11))
    } else {
        ((1u64 << 63) | (fraction << 11), raw_exponent - 1023)
    };
    out[0..8].copy_from_slice(&mantissa.to_le_bytes());
    out[8..10].copy_from_slice(&(sign | ((exponent + 16383) as u16 & 0x7fff)).to_le_bytes());
    out
}

impl Emulator {
    /// Les instructions du coprocesseur. Dernière famille de la chaîne :
    /// ce qu'elle ne connaît pas n'est simulé par personne.
    pub(crate) fn execute_x87(&mut self, instr: &Instruction) -> Result<(), EmuError> {
        use Mnemonic as M;
        match instr.mnemonic() {
            // --- x87 : la pile du 8087 -------------------------------
            // Huit registres en anneau, désignés par rapport au sommet.
            // C'est la machine à calculer d'avant SSE, et c'est encore
            // par elle que la plupart des cours font leur premier calcul
            // à virgule — d'où sa présence ici.
            M::Fld | M::Fild => {
                let value = self.read_fp_operand(instr, 0)?;
                self.cpu.regs.x87.push(value);
            }
            M::Fld1 => self.cpu.regs.x87.push(1.0),
            M::Fldz => self.cpu.regs.x87.push(0.0),
            M::Fldpi => self.cpu.regs.x87.push(std::f64::consts::PI),
            M::Fldl2e => self.cpu.regs.x87.push(std::f64::consts::LOG2_E),
            M::Fldl2t => self.cpu.regs.x87.push(std::f64::consts::LOG2_10),
            M::Fldlg2 => self.cpu.regs.x87.push(std::f64::consts::LOG10_2),
            M::Fldln2 => self.cpu.regs.x87.push(std::f64::consts::LN_2),

            M::Fst | M::Fstp | M::Fist | M::Fistp | M::Fisttp => {
                let value = self.cpu.regs.x87.get(0);
                match st_operand(instr, 0) {
                    Some(i) => self.cpu.regs.x87.set(i, value),
                    None => {
                        let truncate = instr.mnemonic() == M::Fisttp;
                        self.write_fp_operand(instr, 0, value, truncate)?;
                    }
                }
                if matches!(instr.mnemonic(), M::Fstp | M::Fistp | M::Fisttp) {
                    self.cpu.regs.x87.pop();
                }
            }
            M::Fxch => {
                let other = last_operand(instr)
                    .and_then(|index| st_operand(instr, index))
                    .unwrap_or(1);
                let top = self.cpu.regs.x87.get(0);
                let swapped = self.cpu.regs.x87.get(other);
                self.cpu.regs.x87.set(0, swapped);
                self.cpu.regs.x87.set(other, top);
            }

            M::Fadd | M::Fiadd => self.x87_arith(instr, FpOp::Add, false, false)?,
            M::Faddp => self.x87_arith(instr, FpOp::Add, false, true)?,
            M::Fmul | M::Fimul => self.x87_arith(instr, FpOp::Mul, false, false)?,
            M::Fmulp => self.x87_arith(instr, FpOp::Mul, false, true)?,
            M::Fsub | M::Fisub => self.x87_arith(instr, FpOp::Sub, false, false)?,
            M::Fsubp => self.x87_arith(instr, FpOp::Sub, false, true)?,
            // Les formes « r » soustraient et divisent dans l'autre
            // sens : `fsubr` fait source moins destination.
            M::Fsubr | M::Fisubr => self.x87_arith(instr, FpOp::Sub, true, false)?,
            M::Fsubrp => self.x87_arith(instr, FpOp::Sub, true, true)?,
            M::Fdiv | M::Fidiv => self.x87_arith(instr, FpOp::Div, false, false)?,
            M::Fdivp => self.x87_arith(instr, FpOp::Div, false, true)?,
            M::Fdivr | M::Fidivr => self.x87_arith(instr, FpOp::Div, true, false)?,
            M::Fdivrp => self.x87_arith(instr, FpOp::Div, true, true)?,

            M::Fchs => self.x87_map(|v| -v),
            M::Fabs => self.x87_map(f64::abs),
            M::Fsqrt => {
                let v = self.cpu.regs.x87.get(0).sqrt();
                let v = self.shorten_x87(v);
                self.cpu.regs.x87.set(0, v);
            }
            // Les transcendantes : le x87 les calcule sur son format
            // étendu et avec son propre polynôme, namsdec les confie à
            // la bibliothèque de Rust. Les derniers bits diffèrent, et
            // le contrôle de précision ne s'y applique pas même sur le
            // vrai processeur.
            M::Fsin => {
                self.note_simplification(Simplification::X87Transcendantes);
                self.x87_map(f64::sin)
            }
            M::Fcos => {
                self.note_simplification(Simplification::X87Transcendantes);
                self.x87_map(f64::cos)
            }
            M::Frndint => {
                let mode = self.cpu.regs.x87;
                self.x87_map(|v| mode.round_to_integer(v));
            }
            M::F2xm1 => {
                self.note_simplification(Simplification::X87Transcendantes);
                self.x87_map(|v| v.exp2() - 1.0)
            }
            M::Fsincos => {
                self.note_simplification(Simplification::X87Transcendantes);
                let v = self.cpu.regs.x87.get(0);
                self.cpu.regs.x87.set(0, v.sin());
                self.cpu.regs.x87.push(v.cos());
            }
            M::Fptan => {
                self.note_simplification(Simplification::X87Transcendantes);
                let v = self.cpu.regs.x87.get(0);
                self.cpu.regs.x87.set(0, v.tan());
                // La constante 1 empilée derrière : un héritage du
                // 8087, qui rendait ainsi un rapport plutôt qu'un
                // nombre.
                self.cpu.regs.x87.push(1.0);
            }
            M::Fpatan => {
                self.note_simplification(Simplification::X87Transcendantes);
                let x = self.cpu.regs.x87.get(0);
                let y = self.cpu.regs.x87.get(1);
                self.cpu.regs.x87.set(1, y.atan2(x));
                self.cpu.regs.x87.pop();
            }
            M::Fyl2x => {
                self.note_simplification(Simplification::X87Transcendantes);
                let x = self.cpu.regs.x87.get(0);
                let y = self.cpu.regs.x87.get(1);
                self.cpu.regs.x87.set(1, y * x.log2());
                self.cpu.regs.x87.pop();
            }
            M::Fscale => {
                let v = self.cpu.regs.x87.get(0);
                let by = self.cpu.regs.x87.get(1).trunc();
                self.cpu.regs.x87.set(0, v * by.exp2());
            }
            M::Fprem | M::Fprem1 => {
                let a = self.cpu.regs.x87.get(0);
                let b = self.cpu.regs.x87.get(1);
                // `fprem1` est le reste de l'IEEE : son quotient
                // s'arrondit au plus proche, quel que soit le mode
                // réglé dans le mot de contrôle.
                let r = if instr.mnemonic() == M::Fprem {
                    a % b
                } else {
                    a - b * round_even(a / b)
                };
                self.cpu.regs.x87.set(0, r);
                // C2 dirait « reste partiel, recommencez » ; ici le
                // calcul est complet du premier coup.
                self.cpu.regs.x87.status &= !X87::C2;
                self.note_simplification(Simplification::X87FpremEnUnPas);
            }

            M::Fcom | M::Fcomp | M::Fcompp | M::Fucom | M::Fucomp | M::Fucompp | M::Ftst => {
                let a = self.cpu.regs.x87.get(0);
                let b = if instr.mnemonic() == M::Ftst {
                    0.0
                } else {
                    self.x87_comparand(instr)?
                };
                // Le x87 ne touche pas aux indicateurs du processeur : il
                // répond dans son propre mot d'état, qu'il faut aller
                // chercher avec `fstsw`. C'est la grande différence avec
                // `comisd`, et la raison du `fstsw ax` / `sahf` qu'on
                // voit partout dans le code d'époque.
                let status = &mut self.cpu.regs.x87.status;
                *status &= !(X87::C0 | X87::C2 | X87::C3);
                if a.is_nan() || b.is_nan() {
                    *status |= X87::C0 | X87::C2 | X87::C3;
                } else if a < b {
                    *status |= X87::C0;
                } else if a == b {
                    *status |= X87::C3;
                }
                match instr.mnemonic() {
                    M::Fcomp | M::Fucomp => {
                        self.cpu.regs.x87.pop();
                    }
                    M::Fcompp | M::Fucompp => {
                        self.cpu.regs.x87.pop();
                        self.cpu.regs.x87.pop();
                    }
                    _ => {}
                }
            }
            // Les formes « i » écrivent directement dans les indicateurs
            // du processeur : le Pentium Pro a enfin permis de sauter sur
            // une comparaison flottante sans passer par AX.
            M::Fcomi | M::Fcomip | M::Fucomi | M::Fucomip => {
                let a = self.cpu.regs.x87.get(0);
                let b = self.x87_comparand(instr)?;
                let mut flags = RFlags::empty();
                if a.is_nan() || b.is_nan() {
                    flags |= RFlags::ZF | RFlags::PF | RFlags::CF;
                } else if a < b {
                    flags |= RFlags::CF;
                } else if a == b {
                    flags |= RFlags::ZF;
                }
                self.cpu.flags.apply_arith(flags);
                if matches!(instr.mnemonic(), M::Fcomip | M::Fucomip) {
                    self.cpu.regs.x87.pop();
                }
            }
            M::Fxam => {
                let v = self.cpu.regs.x87.get(0);
                let empty = !self.cpu.regs.x87.is_occupied(0);
                let status = &mut self.cpu.regs.x87.status;
                *status &= !(X87::C0 | X87::C1 | X87::C2 | X87::C3);
                if v.is_sign_negative() {
                    *status |= X87::C1;
                }
                if empty {
                    *status |= X87::C0 | X87::C3;
                } else if v.is_nan() {
                    *status |= X87::C0;
                } else if v.is_infinite() {
                    *status |= X87::C0 | X87::C2;
                } else if v == 0.0 {
                    *status |= X87::C3;
                } else {
                    *status |= X87::C2;
                }
            }

            M::Fstsw | M::Fnstsw => {
                self.note_simplification(Simplification::X87MotDEtatPartiel);
                let word = self.cpu.regs.x87.status_word();
                match instr.op_kind(0) {
                    OpKind::Register => self.set_reg(instr.op_register(0), u64::from(word))?,
                    _ => {
                        let addr = self.effective_address(instr, 0)?;
                        self.record_memory(addr, 2);
                        self.cpu.mem.write_u16(addr, word)?;
                    }
                }
            }
            M::Fstcw | M::Fnstcw => {
                let addr = self.effective_address(instr, 0)?;
                self.record_memory(addr, 2);
                self.cpu.mem.write_u16(addr, self.cpu.regs.x87.control)?;
            }
            // Le mot de contrôle est un vrai registre : ce qu'un
            // programme y pose gouverne l'arrondi des conversions et la
            // précision des opérations de base, et se relit tel quel.
            //
            // Ce qu'il ne gouverne pas ici : les six bits de masquage
            // d'exception (0 à 5). Aucune exception n'est levée, donc
            // les démasquer ne change rien — c'est recensé sous
            // `X87ExceptionsMasquees`.
            M::Fldcw => {
                let addr = self.effective_address(instr, 0)?;
                let word = self.cpu.mem.read_u16(addr)?;
                self.cpu.regs.x87.control = word;
                if word & 0x003f != 0x003f {
                    self.note_simplification(Simplification::X87ExceptionsMasquees);
                }
            }
            M::Finit | M::Fninit => self.cpu.regs.x87.reset(),
            M::Ffree | M::Ffreep => {
                let i = last_operand(instr)
                    .and_then(|index| st_operand(instr, index))
                    .unwrap_or(0);
                self.cpu.regs.x87.free(i);
                if instr.mnemonic() == M::Ffreep {
                    self.cpu.regs.x87.pop();
                }
            }
            M::Fincstp => self.cpu.regs.x87.rotate(true),
            M::Fdecstp => self.cpu.regs.x87.rotate(false),
            // Rien à attendre : il n'y a pas de coprocesseur séparé.
            M::Fnop | M::Wait => {}
            other => {
                return Err(EmuError::UnsupportedInstruction {
                    backend: Self::NAME,
                    mnemonic: format!("{other:?}"),
                });
            }
        }
        Ok(())
    }

    /// La valeur d'un opérande flottant : une case de la pile, ou ce que
    /// la mémoire contient — un flottant simple, double ou étendu, ou un
    /// entier de 16, 32 ou 64 bits pour les formes `fi…`.
    fn read_fp_operand(&mut self, instr: &Instruction, op_index: u32) -> Result<f64, EmuError> {
        if let Some(i) = st_operand(instr, op_index) {
            return Ok(self.cpu.regs.x87.get(i));
        }
        let addr = self.effective_address(instr, op_index)?;
        Ok(match instr.memory_size() {
            MemorySize::Float32 => f64::from(f32::from_bits(self.cpu.mem.read_u32(addr)?)),
            MemorySize::Float64 => f64::from_bits(self.cpu.mem.read_u64(addr)?),
            MemorySize::Float80 => {
                self.note_simplification(Simplification::X87FormatEtenduEnMemoire);
                f80_to_f64(self.cpu.mem.bytes_at(addr, 10)?)
            }
            MemorySize::Int16 => f64::from(self.cpu.mem.read_u16(addr)? as i16),
            MemorySize::Int32 => f64::from(self.cpu.mem.read_u32(addr)? as i32),
            MemorySize::Int64 => self.cpu.mem.read_u64(addr)? as i64 as f64,
            other => {
                return Err(EmuError::UnsupportedOperandSize {
                    backend: Self::NAME,
                    bytes: other.size(),
                });
            }
        })
    }

    /// Écrit une valeur de la pile en mémoire, au format que
    /// l'instruction demande. `truncate` distingue `fisttp` — qui tranche
    /// vers zéro — des `fist`, qui arrondissent.
    fn write_fp_operand(
        &mut self,
        instr: &Instruction,
        op_index: u32,
        value: f64,
        truncate: bool,
    ) -> Result<(), EmuError> {
        let addr = self.effective_address(instr, op_index)?;
        // `fisttp` tranche vers zéro quoi qu'il arrive ; les autres
        // suivent le mode réglé dans le mot de contrôle.
        let rounded = if truncate {
            value.trunc()
        } else {
            self.cpu.regs.x87.round_to_integer(value)
        };
        match instr.memory_size() {
            MemorySize::Float32 => {
                self.record_memory(addr, 4);
                self.cpu.mem.write_u32(addr, (value as f32).to_bits())?;
            }
            MemorySize::Float64 => {
                self.record_memory(addr, 8);
                self.cpu.mem.write_u64(addr, value.to_bits())?;
            }
            MemorySize::Float80 => {
                self.note_simplification(Simplification::X87FormatEtenduEnMemoire);
                self.record_memory(addr, 10);
                self.cpu.mem.write_bytes(addr, &f64_to_f80(value))?;
            }
            MemorySize::Int16 => {
                self.record_memory(addr, 2);
                self.cpu
                    .mem
                    .write_u16(addr, to_integer(rounded, Width::W16) as u16)?;
            }
            MemorySize::Int32 => {
                self.record_memory(addr, 4);
                self.cpu
                    .mem
                    .write_u32(addr, to_integer(rounded, Width::W32) as u32)?;
            }
            MemorySize::Int64 => {
                self.record_memory(addr, 8);
                self.cpu
                    .mem
                    .write_u64(addr, to_integer(rounded, Width::W64))?;
            }
            other => {
                return Err(EmuError::UnsupportedOperandSize {
                    backend: Self::NAME,
                    bytes: other.size(),
                });
            }
        }
        Ok(())
    }

    /// Le calcul d'une instruction arithmétique du x87, dans toutes ses
    /// formes : sans opérande (`faddp`), avec une (mémoire ou pile), ou
    /// avec deux cases de la pile.
    fn x87_arith(
        &mut self,
        instr: &Instruction,
        op: FpOp,
        reverse: bool,
        pop: bool,
    ) -> Result<(), EmuError> {
        let (dest, src) = match instr.op_count() {
            // `faddp` tout court : st1 += st0, puis on dépile.
            0 => (1usize, self.cpu.regs.x87.get(0)),
            1 => (0usize, self.read_fp_operand(instr, 0)?),
            _ => {
                let dest = st_operand(instr, 0).ok_or_else(|| EmuError::UnsupportedOperand {
                    backend: Self::NAME,
                    role: OperandRole::Destination,
                    kind: format!("{:?}", instr.op_kind(0)),
                })?;
                (dest, self.read_fp_operand(instr, 1)?)
            }
        };
        let a = self.cpu.regs.x87.get(dest);
        let result = if reverse {
            op.apply64(src, a)
        } else {
            op.apply64(a, src)
        };
        let result = self.shorten_x87(result);
        self.cpu.regs.x87.set(dest, result);
        if pop {
            self.cpu.regs.x87.pop();
        }
        Ok(())
    }

    /// Ce à quoi `fcom` et ses variantes comparent le sommet : `st1` par
    /// défaut, la case ou la valeur nommée sinon.
    fn x87_comparand(&mut self, instr: &Instruction) -> Result<f64, EmuError> {
        match last_operand(instr) {
            // `fcom` tout court compare le sommet à celui d'en dessous.
            None => Ok(self.cpu.regs.x87.get(1)),
            // Le décodeur explicite l'opérande implicite : `fcom st1`
            // arrive ici en `fcom st0, st1`, et c'est la *seconde* qui
            // dit à quoi comparer.
            Some(index) => self.read_fp_operand(instr, index),
        }
    }

    /// Les instructions qui transforment le sommet sans rien empiler.
    fn x87_map(&mut self, f: impl Fn(f64) -> f64) {
        let value = self.cpu.regs.x87.get(0);
        self.cpu.regs.x87.set(0, f(value));
    }
}
