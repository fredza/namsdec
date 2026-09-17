//! Le calcul vectoriel : SSE et AVX.
//!
//! Les deux jeux partagent tout sauf l'encodage. SSE écrit dans sa
//! première source, AVX nomme une destination à part et va jusqu'à
//! 256 bits ; ce qui se passe entre les deux est la même opération sur
//! les mêmes voies. Les fonctions de ce fichier travaillent donc sur une
//! moitié de 128 bits, et `vector_binary` les applique à une moitié ou
//! aux deux.

use std::cmp::Ordering;

use iced_x86::{Instruction, Mnemonic, OpKind};
use namsdec_core::RFlags;

use crate::regmap::map_vector;
use crate::{
    EmuError, Emulator, OperandRole, round_even, sign_extend, to_integer, width_from_bytes,
};

/// Le sens d'un décalage vectoriel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PackedShift {
    Left,
    Right,
    /// Celui qui recopie le bit de signe.
    Arithmetic,
}

/// Le masque d'un élément de `bits` bits.
fn lane_mask(bits: usize) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

/// Ce qu'une instruction flottante calcule.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FpOp {
    Add,
    Sub,
    Mul,
    Div,
    Min,
    Max,
    /// Le seul unaire : il ne lit que la source.
    Sqrt,
}

/// Simple précision (32 bits) ou double (64) : la même opération, sur
/// deux formats dont l'un perd plus que l'autre.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FpWidth {
    Single,
    Double,
}

impl FpOp {
    pub(crate) fn apply64(self, a: f64, b: f64) -> f64 {
        match self {
            FpOp::Add => a + b,
            FpOp::Sub => a - b,
            FpOp::Mul => a * b,
            FpOp::Div => a / b,
            // `minsd`/`maxsd` rendent la *source* dès que la comparaison
            // est fausse — un NaN, une égalité — et c'est exactement ce
            // que fait cette écriture-là.
            FpOp::Min => {
                if a < b {
                    a
                } else {
                    b
                }
            }
            FpOp::Max => {
                if a > b {
                    a
                } else {
                    b
                }
            }
            FpOp::Sqrt => b.sqrt(),
        }
    }

    /// La même chose en simple précision. Calculée en `f32`, et non en
    /// `f64` puis arrondie : l'arrondi intermédiaire n'est pas le même,
    /// et c'est précisément ce qu'on vient apprendre ici.
    fn apply32(self, a: f32, b: f32) -> f32 {
        match self {
            FpOp::Add => a + b,
            FpOp::Sub => a - b,
            FpOp::Mul => a * b,
            FpOp::Div => a / b,
            FpOp::Min => {
                if a < b {
                    a
                } else {
                    b
                }
            }
            FpOp::Max => {
                if a > b {
                    a
                } else {
                    b
                }
            }
            FpOp::Sqrt => b.sqrt(),
        }
    }
}

/// Une valeur vectorielle : deux moitiés de 128 bits, la basse d'abord.
/// La haute n'existe que pour AVX — une instruction SSE ne la voit pas.
type Vector = [u128; 2];

/// L'instruction laisse-t-elle la moitié haute du registre de
/// destination où elle est ? Oui pour SSE, non pour AVX, et c'est
/// l'encodage qui tranche, pas la largeur écrite.
fn keeps_upper_half(instr: &Instruction) -> bool {
    instr.encoding() == iced_x86::EncodingKind::Legacy
}

/// Les indices des deux sources d'une instruction vectorielle.
///
/// SSE écrit dans sa première source — `addpd xmm0, xmm1` fait
/// `xmm0 += xmm1` —, AVX ajoute une destination devant : `vaddpd ymm0,
/// ymm1, ymm2`. `extra` compte les opérandes qui suivent les sources,
/// c'est-à-dire l'immédiat des instructions qui en portent un.
fn sources(instr: &Instruction, extra: u32) -> (u32, u32) {
    let count = instr.op_count().saturating_sub(extra);
    if count >= 3 {
        (count - 2, count - 1)
    } else {
        (0, count.saturating_sub(1))
    }
}

/// Une opération flottante sur le seul élément du bas ; le reste du
/// registre vient de la première source.
fn fp_scalar(a: u128, b: u128, op: FpOp, w: FpWidth) -> u128 {
    let (mask, low) = match w {
        FpWidth::Single => (
            scalar_mask(4),
            u128::from(
                op.apply32(f32::from_bits(a as u32), f32::from_bits(b as u32))
                    .to_bits(),
            ),
        ),
        FpWidth::Double => (
            scalar_mask(8),
            u128::from(
                op.apply64(f64::from_bits(a as u64), f64::from_bits(b as u64))
                    .to_bits(),
            ),
        ),
    };
    (a & !mask) | low
}

/// La même opération sur tout le registre à la fois : quatre flottants
/// simples ou deux doubles par moitié. C'est là tout l'intérêt de ces
/// registres — un seul ordre, plusieurs calculs.
fn fp_packed(a: u128, b: u128, op: FpOp, w: FpWidth) -> u128 {
    let mut out = 0u128;
    match w {
        FpWidth::Single => {
            for lane in 0..4 {
                let x = f32::from_bits((a >> (lane * 32)) as u32);
                let y = f32::from_bits((b >> (lane * 32)) as u32);
                out |= u128::from(op.apply32(x, y).to_bits()) << (lane * 32);
            }
        }
        FpWidth::Double => {
            for lane in 0..2 {
                let x = f64::from_bits((a >> (lane * 64)) as u64);
                let y = f64::from_bits((b >> (lane * 64)) as u64);
                out |= u128::from(op.apply64(x, y).to_bits()) << (lane * 64);
            }
        }
    }
    out
}

/// Le résultat d'une comparaison flottante SSE, élément par élément :
/// que des uns quand c'est vrai, que des zéros sinon.
///
/// Le prédicat est le même pour toutes les largeurs — c'est l'octet
/// immédiat de `cmpps xmm0, xmm1, 2`, et les formes nommées comme
/// `cmplepd` ne sont que des raccourcis d'assembleur pour cet octet-là.
/// Les quatre prédicats en `n` ne sont pas la négation logique de leur
/// jumeau : ils rendent vrai quand la comparaison est fausse **ou** que
/// l'un des deux est un NaN, ce qui n'est pas la même chose.
fn fp_compare(x: f64, y: f64, predicate: u8) -> bool {
    let unordered = x.is_nan() || y.is_nan();
    match predicate & 7 {
        0 => x == y,
        1 => x < y,
        2 => x <= y,
        3 => unordered,
        4 => x != y,
        // « Non plus petit » et « non plus petit ou égal » s'écrivent
        // avec `partial_cmp` : devant un NaN il n'y a pas d'ordre du
        // tout, et c'est justement ce qui distingue ces deux prédicats
        // de `ge` et `gt`.
        5 => !matches!(x.partial_cmp(&y), Some(Ordering::Less)),
        6 => !matches!(x.partial_cmp(&y), Some(Ordering::Less | Ordering::Equal)),
        _ => !unordered,
    }
}

/// `cmpps`/`cmppd` : la comparaison qui écrit un masque dans le registre
/// au lieu de poser des indicateurs.
fn fp_compare_packed(a: u128, b: u128, predicate: u8, w: FpWidth) -> u128 {
    let mut out = 0u128;
    match w {
        FpWidth::Single => {
            for lane in 0..4 {
                let x = f32::from_bits((a >> (lane * 32)) as u32);
                let y = f32::from_bits((b >> (lane * 32)) as u32);
                if fp_compare(x as f64, y as f64, predicate) {
                    out |= u128::from(u32::MAX) << (lane * 32);
                }
            }
        }
        FpWidth::Double => {
            for lane in 0..2 {
                let x = f64::from_bits((a >> (lane * 64)) as u64);
                let y = f64::from_bits((b >> (lane * 64)) as u64);
                if fp_compare(x, y, predicate) {
                    out |= u128::from(u64::MAX) << (lane * 64);
                }
            }
        }
    }
    out
}

/// `cmpss`/`cmpsd` : le même masque, mais sur le seul élément du bas —
/// le reste du registre est celui de la destination, intact.
fn fp_compare_scalar(a: u128, b: u128, predicate: u8, w: FpWidth) -> u128 {
    match w {
        FpWidth::Single => {
            let (x, y) = (f32::from_bits(a as u32), f32::from_bits(b as u32));
            let mask = if fp_compare(x as f64, y as f64, predicate) {
                u128::from(u32::MAX)
            } else {
                0
            };
            (a & !u128::from(u32::MAX)) | mask
        }
        FpWidth::Double => {
            let (x, y) = (f64::from_bits(a as u64), f64::from_bits(b as u64));
            let mask = if fp_compare(x, y, predicate) {
                u128::from(u64::MAX)
            } else {
                0
            };
            (a & !u128::from(u64::MAX)) | mask
        }
    }
}

/// Une opération entière élément par élément, sur des morceaux de
/// `bytes` octets : seize additions d'un seul ordre pour `paddb`.
fn int_lanes(a: u128, b: u128, bytes: usize, op: impl Fn(u64, u64) -> u64) -> u128 {
    let bits = bytes * 8;
    let mask = lane_mask(bits);
    let mut out = 0u128;
    for lane in 0..(16 / bytes) {
        let shift = lane * bits;
        let x = (a >> shift) as u64 & mask;
        let y = (b >> shift) as u64 & mask;
        out |= u128::from(op(x, y) & mask) << shift;
    }
    out
}

/// `pcmpgt*` : la comparaison est *signée*, contrairement à `pcmpeq*`
/// pour qui la question ne se pose pas.
fn signed_gt_lanes(a: u128, b: u128, bytes: usize) -> u128 {
    let Some(w) = width_from_bytes(bytes) else {
        return 0;
    };
    int_lanes(a, b, bytes, move |x, y| {
        if (sign_extend(x, w) as i64) > (sign_extend(y, w) as i64) {
            !0
        } else {
            0
        }
    })
}

/// Un décalage vectoriel. Au-delà de la largeur d'un élément le résultat
/// est nul — ou tout à signe pour `psra*` —, et non « ce qui reste »
/// comme sur un registre général.
fn shift_lanes(a: u128, count: u64, bytes: usize, kind: PackedShift) -> u128 {
    let bits = (bytes * 8) as u64;
    let Some(w) = width_from_bytes(bytes) else {
        return 0;
    };
    let mask = lane_mask(bits as usize);
    let mut out = 0u128;
    for lane in 0..(16 / bytes) {
        let shift = lane * bits as usize;
        let x = (a >> shift) as u64 & mask;
        let value = match kind {
            PackedShift::Left if count < bits => (x << count) & mask,
            PackedShift::Right if count < bits => x >> count,
            PackedShift::Arithmetic => {
                let signed = sign_extend(x, w) as i64;
                (signed >> count.min(bits - 1)) as u64 & mask
            }
            _ => 0,
        };
        out |= u128::from(value) << shift;
    }
    out
}

/// Entrelacer les éléments de deux registres, en partant du bas ou du
/// haut. C'est ce qui sert à élargir des octets en mots, et à transposer.
fn unpack_lanes(a: u128, b: u128, bytes: usize, high: bool) -> u128 {
    let bits = bytes * 8;
    let mask = u128::from(lane_mask(bits));
    let lanes = 16 / bytes;
    let first = if high { lanes / 2 } else { 0 };
    let mut out = 0u128;
    for step in 0..(lanes / 2) {
        let from = (first + step) * bits;
        out |= ((a >> from) & mask) << (step * 2 * bits);
        out |= ((b >> from) & mask) << ((step * 2 + 1) * bits);
    }
    out
}

/// `pshufd` : les quatre mots de 32 bits remis dans l'ordre que
/// l'immédiat désigne, deux bits par élément.
fn shuffle_dwords(src: u128, control: u64) -> u128 {
    let mut out = 0u128;
    for lane in 0..4 {
        let from = (control >> (lane * 2)) & 3;
        out |= ((src >> (from * 32)) & u128::from(u32::MAX)) << (lane * 32);
    }
    out
}

/// `shufps` : les deux éléments bas viennent de la première source, les
/// deux hauts de la seconde.
fn shuffle_singles(a: u128, b: u128, control: u64) -> u128 {
    let mut out = 0u128;
    for lane in 0..4 {
        let from = (control >> (lane * 2)) & 3;
        let taken = if lane < 2 { a } else { b };
        out |= ((taken >> (from * 32)) & u128::from(u32::MAX)) << (lane * 32);
    }
    out
}

/// `shufpd` : un bit par élément, et le même partage entre les sources.
fn shuffle_doubles(a: u128, b: u128, control: u64) -> u128 {
    let low = (a >> ((control & 1) * 64)) & u128::from(u64::MAX);
    let high = (b >> (((control >> 1) & 1) * 64)) & u128::from(u64::MAX);
    low | (high << 64)
}

/// Le masque des `bytes` octets du bas d'un registre XMM.
fn scalar_mask(bytes: usize) -> u128 {
    if bytes >= 16 {
        u128::MAX
    } else {
        (1u128 << (bytes * 8)) - 1
    }
}

impl Emulator {
    /// Les instructions vectorielles, entières comme flottantes. Appelée
    /// quand le jeu entier n'a pas reconnu le mnémonique ; celles qu'elle
    /// ne connaît pas non plus passent au x87.
    pub(crate) fn execute_vector(&mut self, instr: &Instruction) -> Result<(), EmuError> {
        use Mnemonic as M;
        match instr.mnemonic() {
            // --- SSE et AVX : le calcul vectoriel --------------------
            // Le x86-64 moderne ne se sert plus de la pile du 8087 : les
            // flottants vivent dans les registres vectoriels, un par un
            // (« scalaire ») ou plusieurs à la fois (« paqueté »).
            //
            // SSE et AVX partagent tout sauf l'encodage : AVX nomme une
            // destination séparée, va jusqu'à 256 bits, et remet à zéro
            // ce qu'il n'écrit pas. Les deux passent donc par les mêmes
            // fonctions, et `sources` fait la différence.
            M::Movss | M::Vmovss => self.move_scalar(instr, 4)?,
            M::Movsd | M::Vmovsd => self.move_scalar(instr, 8)?,
            M::Movaps
            | M::Movapd
            | M::Movups
            | M::Movupd
            | M::Movdqa
            | M::Movdqu
            | M::Vmovaps
            | M::Vmovapd
            | M::Vmovups
            | M::Vmovupd
            | M::Vmovdqa
            | M::Vmovdqu => {
                let src = self.read_vec_op(instr, instr.op_count() - 1)?;
                let wide = self.is_wide(instr);
                self.write_vec_op(instr, 0, [src[0], if wide { src[1] } else { 0 }])?;
            }
            M::Movd | M::Vmovd => self.move_between_files(instr, 4)?,
            M::Movq | M::Vmovq => self.move_between_files(instr, 8)?,

            M::Addss | M::Vaddss => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Add, FpWidth::Single))?
            }
            M::Addsd | M::Vaddsd => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Add, FpWidth::Double))?
            }
            M::Subss | M::Vsubss => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Sub, FpWidth::Single))?
            }
            M::Subsd | M::Vsubsd => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Sub, FpWidth::Double))?
            }
            M::Mulss | M::Vmulss => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Mul, FpWidth::Single))?
            }
            M::Mulsd | M::Vmulsd => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Mul, FpWidth::Double))?
            }
            M::Divss | M::Vdivss => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Div, FpWidth::Single))?
            }
            M::Divsd | M::Vdivsd => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Div, FpWidth::Double))?
            }
            M::Minss | M::Vminss => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Min, FpWidth::Single))?
            }
            M::Minsd | M::Vminsd => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Min, FpWidth::Double))?
            }
            M::Maxss | M::Vmaxss => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Max, FpWidth::Single))?
            }
            M::Maxsd | M::Vmaxsd => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Max, FpWidth::Double))?
            }
            M::Sqrtss | M::Vsqrtss => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Sqrt, FpWidth::Single))?
            }
            M::Sqrtsd | M::Vsqrtsd => {
                self.vector_binary(instr, |a, b| fp_scalar(a, b, FpOp::Sqrt, FpWidth::Double))?
            }

            M::Addps | M::Vaddps => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Add, FpWidth::Single))?
            }
            M::Addpd | M::Vaddpd => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Add, FpWidth::Double))?
            }
            M::Subps | M::Vsubps => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Sub, FpWidth::Single))?
            }
            M::Subpd | M::Vsubpd => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Sub, FpWidth::Double))?
            }
            M::Mulps | M::Vmulps => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Mul, FpWidth::Single))?
            }
            M::Mulpd | M::Vmulpd => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Mul, FpWidth::Double))?
            }
            M::Divps | M::Vdivps => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Div, FpWidth::Single))?
            }
            M::Divpd | M::Vdivpd => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Div, FpWidth::Double))?
            }
            M::Minps | M::Vminps => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Min, FpWidth::Single))?
            }
            M::Minpd | M::Vminpd => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Min, FpWidth::Double))?
            }
            M::Maxps | M::Vmaxps => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Max, FpWidth::Single))?
            }
            M::Maxpd | M::Vmaxpd => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Max, FpWidth::Double))?
            }
            M::Sqrtps | M::Vsqrtps => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Sqrt, FpWidth::Single))?
            }
            M::Sqrtpd | M::Vsqrtpd => {
                self.vector_binary(instr, |a, b| fp_packed(a, b, FpOp::Sqrt, FpWidth::Double))?
            }

            // `comiss` et `ucomiss` ne diffèrent que par l'exception
            // qu'un NaN signalant déclenche — et il n'y a pas
            // d'exception flottante ici.
            M::Comiss | M::Ucomiss | M::Vcomiss | M::Vucomiss => {
                self.compare_fp(instr, FpWidth::Single)?
            }
            M::Comisd | M::Ucomisd | M::Vcomisd | M::Vucomisd => {
                self.compare_fp(instr, FpWidth::Double)?
            }

            M::Cvtsi2ss | M::Cvtsi2sd | M::Vcvtsi2ss | M::Vcvtsi2sd => {
                let last = instr.op_count() - 1;
                let w = self.src_width(instr, last)?;
                let value = sign_extend(self.read_op(instr, last)?, w) as i64;
                let single = matches!(instr.mnemonic(), M::Cvtsi2ss | M::Vcvtsi2ss);
                let (mask, low) = if single {
                    (scalar_mask(4), u128::from((value as f32).to_bits()))
                } else {
                    (scalar_mask(8), u128::from((value as f64).to_bits()))
                };
                // Le haut vient de la première source : la destination
                // elle-même en SSE, l'opérande du milieu en AVX.
                let upper = self.read_vec_op(instr, if instr.op_count() >= 3 { 1 } else { 0 })?[0];
                self.write_vec_op(instr, 0, [(upper & !mask) | low, 0])?;
            }
            M::Cvttss2si
            | M::Cvtss2si
            | M::Cvttsd2si
            | M::Cvtsd2si
            | M::Vcvttss2si
            | M::Vcvtss2si
            | M::Vcvttsd2si
            | M::Vcvtsd2si => {
                let src = self.read_vec_op(instr, 1)?[0];
                let single = matches!(
                    instr.mnemonic(),
                    M::Cvttss2si | M::Cvtss2si | M::Vcvttss2si | M::Vcvtss2si
                );
                let value = if single {
                    f64::from(f32::from_bits(src as u32))
                } else {
                    f64::from_bits(src as u64)
                };
                let truncate = matches!(
                    instr.mnemonic(),
                    M::Cvttss2si | M::Cvttsd2si | M::Vcvttss2si | M::Vcvttsd2si
                );
                let rounded = if truncate {
                    value.trunc()
                } else {
                    round_even(value)
                };
                let w = self.dest_width(instr)?;
                self.write_op(instr, 0, to_integer(rounded, w))?;
            }
            M::Cvtss2sd | M::Vcvtss2sd => {
                let last = instr.op_count() - 1;
                let src = f32::from_bits(self.read_vec_op(instr, last)?[0] as u32);
                let upper = self.read_vec_op(instr, if instr.op_count() >= 3 { 1 } else { 0 })?[0];
                let low = u128::from(f64::from(src).to_bits());
                self.write_vec_op(instr, 0, [(upper & !scalar_mask(8)) | low, 0])?;
            }
            M::Cvtsd2ss | M::Vcvtsd2ss => {
                let last = instr.op_count() - 1;
                let src = f64::from_bits(self.read_vec_op(instr, last)?[0] as u64);
                let upper = self.read_vec_op(instr, if instr.op_count() >= 3 { 1 } else { 0 })?[0];
                let low = u128::from((src as f32).to_bits());
                self.write_vec_op(instr, 0, [(upper & !scalar_mask(4)) | low, 0])?;
            }

            // Les opérations bit à bit. `xorps xmm0, xmm0` et
            // `pxor xmm0, xmm0` sont la façon habituelle de remettre un
            // registre à zéro.
            M::Xorps | M::Xorpd | M::Pxor | M::Vxorps | M::Vxorpd | M::Vpxor => {
                self.vector_binary(instr, |a, b| a ^ b)?
            }
            M::Andps | M::Andpd | M::Pand | M::Vandps | M::Vandpd | M::Vpand => {
                self.vector_binary(instr, |a, b| a & b)?
            }
            M::Orps | M::Orpd | M::Por | M::Vorps | M::Vorpd | M::Vpor => {
                self.vector_binary(instr, |a, b| a | b)?
            }
            M::Andnps | M::Andnpd | M::Pandn | M::Vandnps | M::Vandnpd | M::Vpandn => {
                self.vector_binary(instr, |a, b| !a & b)?
            }

            // --- Le vectoriel entier ---------------------------------
            M::Paddb | M::Vpaddb => {
                self.vector_binary(instr, |a, b| int_lanes(a, b, 1, u64::wrapping_add))?
            }
            M::Paddw | M::Vpaddw => {
                self.vector_binary(instr, |a, b| int_lanes(a, b, 2, u64::wrapping_add))?
            }
            M::Paddd | M::Vpaddd => {
                self.vector_binary(instr, |a, b| int_lanes(a, b, 4, u64::wrapping_add))?
            }
            M::Paddq | M::Vpaddq => {
                self.vector_binary(instr, |a, b| int_lanes(a, b, 8, u64::wrapping_add))?
            }
            M::Psubb | M::Vpsubb => {
                self.vector_binary(instr, |a, b| int_lanes(a, b, 1, u64::wrapping_sub))?
            }
            M::Psubw | M::Vpsubw => {
                self.vector_binary(instr, |a, b| int_lanes(a, b, 2, u64::wrapping_sub))?
            }
            M::Psubd | M::Vpsubd => {
                self.vector_binary(instr, |a, b| int_lanes(a, b, 4, u64::wrapping_sub))?
            }
            M::Psubq | M::Vpsubq => {
                self.vector_binary(instr, |a, b| int_lanes(a, b, 8, u64::wrapping_sub))?
            }
            // Une comparaison qui ne touche aucun indicateur : elle rend
            // un masque, tout à 1 là où c'est vrai. C'est ce masque qu'on
            // combine ensuite avec `pand`.
            M::Pcmpeqb | M::Vpcmpeqb => self.vector_binary(instr, |a, b| {
                int_lanes(a, b, 1, |x, y| if x == y { !0 } else { 0 })
            })?,
            M::Pcmpeqw | M::Vpcmpeqw => self.vector_binary(instr, |a, b| {
                int_lanes(a, b, 2, |x, y| if x == y { !0 } else { 0 })
            })?,
            M::Pcmpeqd | M::Vpcmpeqd => self.vector_binary(instr, |a, b| {
                int_lanes(a, b, 4, |x, y| if x == y { !0 } else { 0 })
            })?,
            M::Pcmpgtb | M::Vpcmpgtb => {
                self.vector_binary(instr, |a, b| signed_gt_lanes(a, b, 1))?
            }
            M::Pcmpgtw | M::Vpcmpgtw => {
                self.vector_binary(instr, |a, b| signed_gt_lanes(a, b, 2))?
            }
            M::Pcmpgtd | M::Vpcmpgtd => {
                self.vector_binary(instr, |a, b| signed_gt_lanes(a, b, 4))?
            }

            M::Psllw | M::Vpsllw => self.packed_shift(instr, 2, PackedShift::Left)?,
            M::Pslld | M::Vpslld => self.packed_shift(instr, 4, PackedShift::Left)?,
            M::Psllq | M::Vpsllq => self.packed_shift(instr, 8, PackedShift::Left)?,
            M::Psrlw | M::Vpsrlw => self.packed_shift(instr, 2, PackedShift::Right)?,
            M::Psrld | M::Vpsrld => self.packed_shift(instr, 4, PackedShift::Right)?,
            M::Psrlq | M::Vpsrlq => self.packed_shift(instr, 8, PackedShift::Right)?,
            M::Psraw | M::Vpsraw => self.packed_shift(instr, 2, PackedShift::Arithmetic)?,
            M::Psrad | M::Vpsrad => self.packed_shift(instr, 4, PackedShift::Arithmetic)?,
            // Celles-ci décalent par octets, et chaque moitié de 128 bits
            // pour son compte : AVX n'a pas élargi le décalage, il l'a
            // dédoublé.
            M::Pslldq | M::Psrldq | M::Vpslldq | M::Vpsrldq => {
                let last = instr.op_count() - 1;
                let bits = (self.read_op(instr, last)? as u32).min(16) * 8;
                let left = matches!(instr.mnemonic(), M::Pslldq | M::Vpslldq);
                self.vector_binary_extra(instr, 1, move |_, b| {
                    if bits >= 128 {
                        0
                    } else if left {
                        b << bits
                    } else {
                        b >> bits
                    }
                })?;
            }

            M::Pmovmskb | M::Vpmovmskb => self.move_sign_mask(instr, 8)?,
            M::Movmskps | M::Vmovmskps => self.move_sign_mask(instr, 32)?,
            M::Movmskpd | M::Vmovmskpd => self.move_sign_mask(instr, 64)?,

            // Comparer en écrivant un masque, et non des indicateurs :
            // l'autre façon de comparer des flottants, celle qui sert à
            // choisir sans brancher. iced rend le prédicat sous forme
            // d'immédiat, y compris pour les formes nommées — `cmpltpd`
            // est décodé comme `cmppd …, 1`.
            M::Cmpps | M::Vcmpps => {
                let predicate = self.read_op(instr, instr.op_count() - 1)? as u8;
                self.vector_binary_extra(instr, 1, move |a, b| {
                    fp_compare_packed(a, b, predicate, FpWidth::Single)
                })?;
            }
            M::Cmppd | M::Vcmppd => {
                let predicate = self.read_op(instr, instr.op_count() - 1)? as u8;
                self.vector_binary_extra(instr, 1, move |a, b| {
                    fp_compare_packed(a, b, predicate, FpWidth::Double)
                })?;
            }
            M::Cmpss | M::Vcmpss => {
                let predicate = self.read_op(instr, instr.op_count() - 1)? as u8;
                self.vector_binary_extra(instr, 1, move |a, b| {
                    fp_compare_scalar(a, b, predicate, FpWidth::Single)
                })?;
            }
            M::Cmpsd | M::Vcmpsd => {
                let predicate = self.read_op(instr, instr.op_count() - 1)? as u8;
                self.vector_binary_extra(instr, 1, move |a, b| {
                    fp_compare_scalar(a, b, predicate, FpWidth::Double)
                })?;
            }

            // Remettre les éléments dans un autre ordre.
            M::Pshufd | M::Vpshufd => {
                let control = self.read_op(instr, instr.op_count() - 1)?;
                self.vector_binary_extra(instr, 1, move |_, b| shuffle_dwords(b, control))?;
            }
            M::Shufps | M::Vshufps => {
                let control = self.read_op(instr, instr.op_count() - 1)?;
                self.vector_binary_extra(instr, 1, move |a, b| shuffle_singles(a, b, control))?;
            }
            M::Shufpd | M::Vshufpd => {
                let control = self.read_op(instr, instr.op_count() - 1)?;
                self.vector_binary_extra(instr, 1, move |a, b| shuffle_doubles(a, b, control))?;
            }
            M::Movhlps | M::Vmovhlps => {
                self.vector_binary(instr, |a, b| (a & !u128::from(u64::MAX)) | (b >> 64))?
            }
            M::Movlhps | M::Vmovlhps => {
                self.vector_binary(instr, |a, b| (a & u128::from(u64::MAX)) | (b << 64))?
            }
            M::Punpcklbw | M::Vpunpcklbw => {
                self.vector_binary(instr, |a, b| unpack_lanes(a, b, 1, false))?
            }
            M::Punpcklwd | M::Vpunpcklwd => {
                self.vector_binary(instr, |a, b| unpack_lanes(a, b, 2, false))?
            }
            M::Punpckldq | M::Unpcklps | M::Vpunpckldq | M::Vunpcklps => {
                self.vector_binary(instr, |a, b| unpack_lanes(a, b, 4, false))?
            }
            M::Punpcklqdq | M::Unpcklpd | M::Vpunpcklqdq | M::Vunpcklpd => {
                self.vector_binary(instr, |a, b| unpack_lanes(a, b, 8, false))?
            }
            M::Punpckhbw | M::Vpunpckhbw => {
                self.vector_binary(instr, |a, b| unpack_lanes(a, b, 1, true))?
            }
            M::Punpckhwd | M::Vpunpckhwd => {
                self.vector_binary(instr, |a, b| unpack_lanes(a, b, 2, true))?
            }
            M::Punpckhdq | M::Unpckhps | M::Vpunpckhdq | M::Vunpckhps => {
                self.vector_binary(instr, |a, b| unpack_lanes(a, b, 4, true))?
            }
            M::Punpckhqdq | M::Unpckhpd | M::Vpunpckhqdq | M::Vunpckhpd => {
                self.vector_binary(instr, |a, b| unpack_lanes(a, b, 8, true))?
            }

            // --- Ce qu'AVX ajoute, et qui n'a pas d'équivalent SSE ----
            // Un élément recopié partout : c'est ainsi qu'on multiplie un
            // vecteur entier par un scalaire.
            M::Vbroadcastss
            | M::Vbroadcastsd
            | M::Vpbroadcastb
            | M::Vpbroadcastw
            | M::Vpbroadcastd
            | M::Vpbroadcastq => {
                let src = self.read_vec_op(instr, 1)?[0];
                let bytes = match instr.mnemonic() {
                    M::Vpbroadcastb => 1,
                    M::Vpbroadcastw => 2,
                    M::Vbroadcastss | M::Vpbroadcastd => 4,
                    _ => 8,
                };
                let element = src & scalar_mask(bytes);
                let mut half = 0u128;
                for lane in 0..(16 / bytes) {
                    half |= element << (lane * bytes * 8);
                }
                let wide = self.is_wide(instr);
                self.write_vec_op(instr, 0, [half, if wide { half } else { 0 }])?;
            }
            // Les deux moitiés d'un registre 256 bits ne se parlent pas
            // d'elles-mêmes : il faut les instructions qui les nomment.
            M::Vinsertf128 | M::Vinserti128 => {
                let dest = self.read_vec_op(instr, 1)?;
                let src = self.read_vec_op(instr, 2)?[0];
                let high = self.read_op(instr, 3)? & 1 == 1;
                let out = if high { [dest[0], src] } else { [src, dest[1]] };
                self.write_vec_op(instr, 0, out)?;
            }
            M::Vextractf128 | M::Vextracti128 => {
                let src = self.read_vec_op(instr, 1)?;
                let high = self.read_op(instr, 2)? & 1 == 1;
                let taken = if high { src[1] } else { src[0] };
                self.write_vec_op(instr, 0, [taken, 0])?;
            }
            M::Vperm2f128 | M::Vperm2i128 => {
                let a = self.read_vec_op(instr, 1)?;
                let b = self.read_vec_op(instr, 2)?;
                let control = self.read_op(instr, 3)?;
                let pick = |selector: u64| -> u128 {
                    if selector & 0b1000 != 0 {
                        return 0;
                    }
                    match selector & 3 {
                        0 => a[0],
                        1 => a[1],
                        2 => b[0],
                        _ => b[1],
                    }
                };
                let out = [pick(control & 0xf), pick((control >> 4) & 0xf)];
                self.write_vec_op(instr, 0, out)?;
            }
            M::Vpermq | M::Vpermpd => {
                let src = self.read_vec_op(instr, 1)?;
                let control = self.read_op(instr, 2)?;
                let quad = |index: u64| -> u128 {
                    let half = src[(index / 2) as usize];
                    (half >> ((index % 2) * 64)) & u128::from(u64::MAX)
                };
                let mut out = [0u128; 2];
                for lane in 0..4 {
                    let from = (control >> (lane * 2)) & 3;
                    out[lane / 2] |= quad(from) << ((lane % 2) * 64);
                }
                self.write_vec_op(instr, 0, out)?;
            }
            // Remettre les moitiés hautes à zéro avant de repasser à du
            // code SSE : sans elle, le processeur paie une pénalité à
            // chaque transition. Ici, elle ne coûte rien — mais elle doit
            // faire ce qu'elle dit.
            M::Vzeroupper | M::Vzeroall => {
                let all = instr.mnemonic() == M::Vzeroall;
                for reg in namsdec_core::RegXmm::ALL {
                    let value = self.cpu.regs.get_ymm(reg);
                    self.cpu
                        .regs
                        .set_ymm(reg, if all { [0, 0] } else { [value[0], 0] });
                }
            }
            _ => return self.execute_x87(instr),
        }
        Ok(())
    }

    /// Lit un opérande vectoriel : un registre (les 256 bits, dont la
    /// moitié haute ne vaut que pour AVX) ou ce que l'instruction va
    /// chercher en mémoire, de quatre à trente-deux octets.
    fn read_vec_op(&self, instr: &Instruction, op_index: u32) -> Result<Vector, EmuError> {
        match instr.op_kind(op_index) {
            OpKind::Register => {
                let reg = instr.op_register(op_index);
                map_vector(reg)
                    .map(|r| self.cpu.regs.get_ymm(r))
                    .ok_or_else(|| EmuError::UnsupportedRegister {
                        backend: Self::NAME,
                        register: format!("{reg:?}"),
                    })
            }
            OpKind::Memory => {
                let addr = self.effective_address(instr, op_index)?;
                let bytes = instr.memory_size().size().min(32);
                let raw = self.cpu.mem.bytes_at(addr, bytes)?;
                let mut buf = [0u8; 32];
                buf[..bytes].copy_from_slice(raw);
                let (chunks, _) = buf.as_chunks::<16>();
                let mut halves = [0u128; 2];
                for (half, chunk) in halves.iter_mut().zip(chunks) {
                    *half = u128::from_le_bytes(*chunk);
                }
                Ok(halves)
            }
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Read,
                kind: format!("{other:?}"),
            }),
        }
    }

    /// Écrit un opérande vectoriel.
    ///
    /// Sur un registre, ce qu'il advient de la moitié haute ne dépend pas
    /// de la valeur écrite mais de l'*encodage* : une instruction SSE la
    /// laisse où elle est, une instruction AVX la remet à zéro. C'est la
    /// règle qu'on oublie en mélangeant les deux jeux.
    fn write_vec_op(
        &mut self,
        instr: &Instruction,
        op_index: u32,
        value: Vector,
    ) -> Result<(), EmuError> {
        match instr.op_kind(op_index) {
            OpKind::Register => {
                let reg = instr.op_register(op_index);
                match map_vector(reg) {
                    Some(r) => {
                        if keeps_upper_half(instr) {
                            self.cpu.regs.set_xmm(r, value[0]);
                        } else {
                            let high = if reg.is_ymm() { value[1] } else { 0 };
                            self.cpu.regs.set_ymm(r, [value[0], high]);
                        }
                        Ok(())
                    }
                    None => Err(EmuError::UnsupportedRegister {
                        backend: Self::NAME,
                        register: format!("{reg:?}"),
                    }),
                }
            }
            OpKind::Memory => {
                let addr = self.effective_address(instr, op_index)?;
                let bytes = instr.memory_size().size().min(32);
                let mut buf = [0u8; 32];
                buf[..16].copy_from_slice(&value[0].to_le_bytes());
                buf[16..].copy_from_slice(&value[1].to_le_bytes());
                self.record_memory(addr, bytes);
                self.cpu.mem.write_bytes(addr, &buf[..bytes])?;
                Ok(())
            }
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Write,
                kind: format!("{other:?}"),
            }),
        }
    }

    /// L'opération travaille-t-elle sur 256 bits ? La destination le dit.
    fn is_wide(&self, instr: &Instruction) -> bool {
        match instr.op_kind(0) {
            OpKind::Register => instr.op_register(0).is_ymm(),
            OpKind::Memory => instr.memory_size().size() > 16,
            _ => false,
        }
    }

    /// Le cœur de toutes les instructions vectorielles à deux sources :
    /// la même opération appliquée à chaque moitié de 128 bits.
    ///
    /// Elle sert aux deux jeux. SSE écrit dans sa première source (`addpd
    /// xmm0, xmm1`), AVX nomme une destination à part (`vaddpd ymm0,
    /// ymm1, ymm2`) : `sources` dit lesquelles lire, et le reste est
    /// identique — c'est bien la même machine, avec un encodage plus
    /// large.
    fn vector_binary(
        &mut self,
        instr: &Instruction,
        f: impl Fn(u128, u128) -> u128,
    ) -> Result<(), EmuError> {
        self.vector_binary_extra(instr, 0, f)
    }

    /// La même chose quand l'instruction porte en plus un immédiat, qui
    /// ne compte pas parmi les sources.
    fn vector_binary_extra(
        &mut self,
        instr: &Instruction,
        extra: u32,
        f: impl Fn(u128, u128) -> u128,
    ) -> Result<(), EmuError> {
        let (first, second) = sources(instr, extra);
        let a = self.read_vec_op(instr, first)?;
        let b = self.read_vec_op(instr, second)?;
        let wide = self.is_wide(instr);
        let out = [f(a[0], b[0]), if wide { f(a[1], b[1]) } else { 0 }];
        self.write_vec_op(instr, 0, out)
    }

    /// `movss`/`movsd` : un seul flottant se déplace, et ce qui arrive au
    /// reste du registre dépend d'où il vient. Depuis la mémoire, tout le
    /// reste est mis à zéro ; depuis un autre registre, le haut est
    /// préservé — une asymétrie du jeu d'instructions, pas un choix
    /// d'ici. AVX en fait une troisième opérande explicite.
    fn move_scalar(&mut self, instr: &Instruction, bytes: usize) -> Result<(), EmuError> {
        let mask = scalar_mask(bytes);
        let last = instr.op_count() - 1;
        if instr.op_kind(0) == OpKind::Memory {
            let src = self.read_vec_op(instr, last)?;
            return self.write_vec_op(instr, 0, [src[0] & mask, 0]);
        }
        if instr.op_count() >= 3 {
            // `vmovss xmm0, xmm1, xmm2` : le haut vient de la première
            // source, l'élément du bas de la seconde.
            let upper = self.read_vec_op(instr, 1)?[0] & !mask;
            let low = self.read_vec_op(instr, 2)?[0] & mask;
            return self.write_vec_op(instr, 0, [upper | low, 0]);
        }
        let src = self.read_vec_op(instr, last)?[0] & mask;
        let upper = if instr.op_kind(last) == OpKind::Memory {
            0
        } else {
            self.read_vec_op(instr, 0)?[0] & !mask
        };
        self.write_vec_op(instr, 0, [upper | src, 0])
    }

    /// `movd`/`movq` : le passage entre les deux familles de registres.
    /// Ce qui entre dans un registre vectoriel en efface le reste ; ce qui
    /// en sort est tronqué à la largeur demandée.
    fn move_between_files(&mut self, instr: &Instruction, bytes: usize) -> Result<(), EmuError> {
        let mask = scalar_mask(bytes);
        let dest_is_vector =
            instr.op_kind(0) == OpKind::Register && map_vector(instr.op_register(0)).is_some();
        let src_is_vector =
            instr.op_kind(1) == OpKind::Register && map_vector(instr.op_register(1)).is_some();

        if dest_is_vector {
            let value = if src_is_vector {
                self.read_vec_op(instr, 1)?[0]
            } else {
                u128::from(self.read_op(instr, 1)?)
            };
            return self.write_vec_op(instr, 0, [value & mask, 0]);
        }
        let value = (self.read_vec_op(instr, 1)?[0] & mask) as u64;
        if instr.op_kind(0) == OpKind::Memory {
            self.write_vec_op(instr, 0, [u128::from(value), 0])
        } else {
            self.write_op(instr, 0, value)
        }
    }

    /// `comisd` et ses variantes rangent le résultat de la comparaison
    /// dans ZF, PF et CF — et non dans les indicateurs de signe, qu'un
    /// `jg` irait lire pour rien. D'où les `ja`/`jb` qu'on voit après du
    /// code flottant : ce sont les tests *non signés* qui conviennent.
    fn compare_fp(&mut self, instr: &Instruction, w: FpWidth) -> Result<(), EmuError> {
        let a = self.read_vec_op(instr, 0)?[0];
        let b = self.read_vec_op(instr, 1)?[0];
        let (x, y) = match w {
            FpWidth::Single => (
                f64::from(f32::from_bits(a as u32)),
                f64::from(f32::from_bits(b as u32)),
            ),
            FpWidth::Double => (f64::from_bits(a as u64), f64::from_bits(b as u64)),
        };
        let mut flags = RFlags::empty();
        if x.is_nan() || y.is_nan() {
            // « Non ordonné » : les trois indicateurs à 1 d'un coup, ce
            // qu'aucune comparaison entière ne produit jamais.
            flags |= RFlags::ZF | RFlags::PF | RFlags::CF;
        } else if x < y {
            flags |= RFlags::CF;
        } else if x == y {
            flags |= RFlags::ZF;
        }
        self.cpu.flags.apply_arith(flags);
        Ok(())
    }

    /// Les décalages vectoriels : un seul compte — lu dans un immédiat ou
    /// dans le bas d'un registre — appliqué à tous les éléments.
    fn packed_shift(
        &mut self,
        instr: &Instruction,
        bytes: usize,
        kind: PackedShift,
    ) -> Result<(), EmuError> {
        let last = instr.op_count() - 1;
        let count = if matches!(instr.op_kind(last), OpKind::Immediate8) {
            self.read_op(instr, last)?
        } else {
            self.read_vec_op(instr, last)?[0] as u64
        };
        let (_, source) = sources(instr, 1);
        let a = self.read_vec_op(instr, source)?;
        let wide = self.is_wide(instr);
        let out = [
            shift_lanes(a[0], count, bytes, kind),
            if wide {
                shift_lanes(a[1], count, bytes, kind)
            } else {
                0
            },
        ];
        self.write_vec_op(instr, 0, out)
    }

    /// Rassembler les bits de signe dans un registre général : c'est
    /// ainsi qu'on ramène le résultat d'une comparaison vectorielle
    /// là où un saut conditionnel peut le lire.
    fn move_sign_mask(&mut self, instr: &Instruction, step: usize) -> Result<(), EmuError> {
        let src = self.read_vec_op(instr, 1)?;
        let wide = instr.op_kind(1) == OpKind::Register && instr.op_register(1).is_ymm();
        let per_half = 128 / step;
        let mut mask = 0u64;
        for (index, half) in src.iter().enumerate() {
            if index == 1 && !wide {
                break;
            }
            for lane in 0..per_half {
                let sign = (half >> (lane * step + step - 1)) & 1;
                mask |= (sign as u64) << (index * per_half + lane);
            }
        }
        self.write_op(instr, 0, mask)
    }
}
