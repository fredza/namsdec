//! Interpréteur x86-64 "maison" : le décodage octets -> instruction
//! structurée est délégué à `namsdec-disasm` (donc à `iced-x86`), mais toute
//! la sémantique d'exécution (effets sur registres/flags/mémoire) est
//! écrite ici, à la main, pour rester transparente et pédagogique.

mod regmap;

use std::collections::{BTreeSet, VecDeque};

use iced_x86::{ConditionCode, Instruction, MemorySize, Mnemonic, OpKind, Register};
use namsdec_core::{
    Cpu, Memory, RFlags, Reg64, Width, X87, add_with_flags, logic_flags, sub_with_flags,
};
use thiserror::Error;

use regmap::{is_high_byte, map_register, map_vector};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Là où une opérande n'a pas pu être traitée — le sens de la phrase
/// change en français (« opérande source/destination »), la variante le
/// garde donc plutôt que de figer une formulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandRole {
    Destination,
    Read,
    Write,
}

/// Erreurs d'exécution. `Display` est en anglais et les valeurs restent
/// accessibles : c'est `namsdec-gui` qui rend le message dans la langue de
/// l'utilisateur (voir `MemError` pour le pourquoi).
///
/// « Non supporté » désigne toujours l'interpréteur *courant* : un autre
/// moteur d'exécution branché derrière `Backend` peut parfaitement
/// connaître l'instruction que celui-ci refuse, d'où le nom du backend
/// dans le message.
#[derive(Debug, Error)]
pub enum EmuError {
    #[error("{backend}: unsupported instruction: {mnemonic}")]
    UnsupportedInstruction {
        backend: &'static str,
        mnemonic: String,
    },
    #[error("{backend}: unsupported register: {register}")]
    UnsupportedRegister {
        backend: &'static str,
        register: String,
    },
    #[error("{backend}: unsupported {role:?} operand: {kind}")]
    UnsupportedOperand {
        backend: &'static str,
        role: OperandRole,
        kind: String,
    },
    #[error("{backend}: unsupported memory operand size: {bytes} bytes")]
    UnsupportedOperandSize { backend: &'static str, bytes: usize },
    #[error("{backend}: unsupported syscall: {number}")]
    UnsupportedSyscall { backend: &'static str, number: u64 },
    #[error("memory error: {0}")]
    Mem(#[from] namsdec_core::MemError),
    #[error("invalid instruction at {addr:#x}")]
    InvalidInstruction { addr: u64 },
    /// Ce que le processeur signale par l'exception #DE : une division
    /// par zéro, ou un quotient trop grand pour le registre qui doit le
    /// recevoir. Deux variantes distinctes parce que ce ne sont pas la
    /// même erreur de programmation.
    #[error("division by zero at {addr:#x}")]
    DivideByZero { addr: u64 },
    #[error("quotient too large at {addr:#x}")]
    DivideOverflow { addr: u64 },
}

/// L'adresse arrondie au début de la page suivante — elle-même si elle
/// est déjà alignée.
fn page_up(addr: u64) -> u64 {
    addr.div_ceil(PAGE).saturating_mul(PAGE)
}

/// Une zone de mémoire surveillée : on s'arrête quand elle change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watchpoint {
    pub addr: u64,
    /// Un à huit octets — la taille de ce qu'on regarde d'habitude : un
    /// octet, un mot, un pointeur.
    pub len: usize,
    /// La valeur vue la dernière fois. C'est la comparaison avec elle qui
    /// déclenche l'arrêt, et non l'écriture : un `mov [x], 0` sur une
    /// case déjà nulle n'apprend rien et n'arrête rien.
    previous: u64,
}

impl Watchpoint {
    /// La valeur retenue au dernier armement.
    pub fn value(&self) -> u64 {
        self.previous
    }
}

/// Un appel en cours : ce qu'il faut pour remonter la chaîne.
///
/// L'émulateur *voit* les `call` et les `ret` passer, il n'a donc pas à
/// deviner les cadres en suivant RBP comme le fait un débogueur sans
/// symboles — ce qui échoue justement sur le code qu'on écrit en
/// apprenant, où personne ne monte de cadre de pile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// L'adresse de l'instruction `call` elle-même : c'est elle qui se
    /// traduit en « appelé depuis la ligne … ».
    pub call_site: u64,
    /// Où l'appel est parti — le début de la sous-routine.
    pub entry: u64,
    /// Où il reviendra.
    pub return_addr: u64,
    /// RSP juste après l'empilement de l'adresse de retour.
    pub rsp: u64,
}

/// Ce qu'un pas a fait à la pile des appels, pour pouvoir le défaire.
#[derive(Clone, Copy, Debug)]
enum FrameChange {
    /// Ni `call` ni `ret` : rien à défaire.
    None,
    /// Un `call` : le cadre empilé est à retirer.
    Pushed,
    /// Un `ret` : le cadre dépilé est à remettre.
    Popped(Frame),
}

/// Ce qu'il faut retenir d'un pas pour pouvoir le défaire.
///
/// Un instantané complet de la machine coûterait la mémoire entière à
/// chaque instruction ; on ne garde donc que ce qui est petit (registres,
/// flags, état d'arrêt) et, pour la mémoire, les seuls octets que le pas
/// s'apprête à écraser. Un pas ordinaire n'en touche aucun.
#[derive(Clone, Debug)]
struct Undo {
    regs: namsdec_core::Registers,
    flags: RFlags,
    halted: bool,
    exit_code: Option<i64>,
    /// Longueur de la sortie avant le pas : ce qu'il a écrit se tronque.
    output_len: usize,
    /// Les octets d'entrée qu'un `read` a consommés, à remettre en tête.
    consumed_input: Vec<u8>,
    input_closed: bool,
    /// Les octets écrasés, avec leur adresse, dans l'ordre des écritures.
    memory: Vec<(u64, Vec<u8>)>,
    /// Ce que le pas a fait à la pile des appels.
    frames: FrameChange,
    /// Le tas d'avant : un `brk` se défait comme le reste.
    heap: Heap,
    /// Le compteur d'instructions d'avant : `rdtsc` doit rendre deux fois
    /// la même valeur si on rejoue deux fois le même pas.
    ticks: u64,
}

/// Combien de pas on peut défaire. Au-delà, les plus anciens sont
/// oubliés : revenir mille instructions en arrière n'a pas de sens
/// pédagogique, et la mémoire n'est pas extensible.
const HISTORY_LIMIT: usize = 5_000;

/// Ce que fait une instruction de chaîne d'un élément.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StringOp {
    /// `movs` : recopie de `[rsi]` vers `[rdi]`.
    Move,
    /// `stos` : écrit l'accumulateur en `[rdi]`.
    Store,
    /// `lods` : lit `[rsi]` dans l'accumulateur.
    Load,
    /// `scas` : compare l'accumulateur à `[rdi]`.
    Scan,
    /// `cmps` : compare `[rsi]` à `[rdi]`.
    Compare,
}

/// La largeur d'un élément, lue sur le mnémonique.
fn string_width(mnemonic: Mnemonic) -> Option<Width> {
    use Mnemonic as M;
    Some(match mnemonic {
        M::Movsb | M::Stosb | M::Lodsb | M::Scasb | M::Cmpsb => Width::W8,
        M::Movsw | M::Stosw | M::Lodsw | M::Scasw | M::Cmpsw => Width::W16,
        M::Movsd | M::Stosd | M::Lodsd | M::Scasd | M::Cmpsd => Width::W32,
        M::Movsq | M::Stosq | M::Lodsq | M::Scasq | M::Cmpsq => Width::W64,
        _ => return None,
    })
}

/// `movsd` et `cmpsd` nomment deux instructions sans rapport : celle qui
/// travaille sur une chaîne, et celle du jeu SSE qui manipule un flottant
/// dans un registre `xmm`. Seule la première désigne ses opérandes par
/// RSI/RDI, et c'est à ça qu'on les distingue.
fn is_string_operation(instr: &Instruction) -> bool {
    (0..instr.op_count()).any(|op| {
        matches!(
            instr.op_kind(op),
            OpKind::MemorySegRSI
                | OpKind::MemorySegESI
                | OpKind::MemorySegSI
                | OpKind::MemoryESRDI
                | OpKind::MemoryESEDI
                | OpKind::MemoryESDI
        )
    })
}

/// Le sens d'un décalage vectoriel.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PackedShift {
    Left,
    Right,
    /// Celui qui recopie le bit de signe.
    Arithmetic,
}

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
enum FpOp {
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
    fn apply64(self, a: f64, b: f64) -> f64 {
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

/// L'arrondi au plus proche, l'impair tranché vers le pair — le mode par
/// défaut du processeur, celui que `cvtsd2si` applique.
fn round_even(value: f64) -> f64 {
    let rounded = value.round();
    if (value - value.trunc()).abs() == 0.5 && rounded % 2.0 != 0.0 {
        rounded - value.signum()
    } else {
        rounded
    }
}

/// Le résultat entier d'une conversion. Hors bornes ou NaN, le
/// processeur rend « l'entier indéfini » : le plus petit négatif de la
/// largeur demandée. Une valeur reconnaissable, plutôt qu'un repli
/// silencieux sur zéro.
fn to_integer(value: f64, w: Width) -> u64 {
    let (min, max, indefinite) = match w {
        Width::W64 => (i64::MIN as f64, i64::MAX as f64, i64::MIN as u64),
        _ => (
            f64::from(i32::MIN),
            f64::from(i32::MAX),
            u64::from(i32::MIN as u32),
        ),
    };
    if value.is_nan() || value < min || value > max {
        indefinite
    } else {
        value as i64 as u64
    }
}

/// Le sens d'une rotation, et si elle passe par la retenue — auquel cas
/// le registre tourne sur un bit de plus que sa largeur.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RotateKind {
    Left,
    Right,
    CarryLeft,
    CarryRight,
}

/// Ce qu'on fait au bit désigné, une fois qu'on l'a lu dans CF.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BitOp {
    /// `bt` : le lire, et rien d'autre.
    Test,
    Set,
    Reset,
    Complement,
}

impl BitOp {
    /// La valeur à réécrire, ou `None` quand il n'y a rien à écrire.
    fn apply(self, value: u64, bit: u32) -> Option<u64> {
        let one = 1u64 << bit;
        match self {
            BitOp::Test => None,
            BitOp::Set => Some(value | one),
            BitOp::Reset => Some(value & !one),
            BitOp::Complement => Some(value ^ one),
        }
    }
}

/// Le sens d'un décalage, et s'il recopie le bit de signe.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ShiftKind {
    Left,
    Right,
    /// `sar` : décalage à droite qui recopie le bit de signe, donc une
    /// division entière par une puissance de deux.
    Arithmetic,
}

/// Lit `len` octets (1 à 8) en petit-boutien. Hors de la fenêtre
/// simulée, rend zéro : une zone surveillée qu'on ne peut pas lire ne
/// change jamais, plutôt que de faire échouer l'exécution.
fn read_bytes(mem: &namsdec_core::Memory, addr: u64, len: usize) -> u64 {
    let mut value = 0u64;
    for i in (0..len).rev() {
        let byte = mem.read_u8(addr.wrapping_add(i as u64)).unwrap_or(0);
        value = (value << 8) | u64::from(byte);
    }
    value
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

/// Étend `value`, large de `w`, sur 64 bits en recopiant son bit de signe.
fn sign_extend(value: u64, w: Width) -> u64 {
    let bits = w.bits();
    if bits >= 64 {
        return value;
    }
    let shift = 64 - bits;
    (((value << shift) as i64) >> shift) as u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Continued,
    Halted,
    /// Le programme a demandé à lire son entrée standard, et il n'y a rien
    /// à lui donner. L'instruction n'a **pas** été exécutée et RIP est
    /// resté dessus : dès que l'entrée arrive, le même `syscall` repart
    /// comme si de rien n'était.
    NeedsInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStop {
    Halted,
    Breakpoint(u64),
    /// Un point de surveillance a vu la mémoire changer sous lui.
    ///
    /// Là où un point d'arrêt demande « arrête-toi *là* », celui-ci
    /// demande « arrête-toi quand *ça* change » — et l'on ne sait pas
    /// d'avance quelle instruction le fera. C'est ce qui le rend utile,
    /// et coûteux dans un vrai débogueur ; ici, l'interpréteur passe déjà
    /// par un seul endroit pour écrire en mémoire.
    Watchpoint {
        addr: u64,
        old: u64,
        new: u64,
    },
    StepLimit,
    /// Suspendu en attendant l'entrée standard (voir
    /// `StepResult::NeedsInput`). L'exécution reprend d'elle-même une fois
    /// l'entrée fournie.
    NeedsInput,
}

/// Résultat d'un `step_over` : distinct de `StepResult`/`RunStop` parce que
/// "j'ai enjambé un appel entier" n'est ni un simple pas, ni une fin de
/// `run()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOverResult {
    /// Comme `RunStop::Watchpoint` : la sous-routine enjambée a touché à
    /// une zone surveillée.
    Watchpoint {
        addr: u64,
        old: u64,
        new: u64,
    },
    /// L'instruction courante n'était pas un `call` : un pas normal a été
    /// fait, exactement comme `step()`.
    Stepped,
    /// C'était un `call` : toute la sous-routine a été exécutée d'un bloc,
    /// RIP est revenu juste après l'appel.
    SteppedOverCall,
    Halted,
    /// Un point d'arrêt a été atteint à l'intérieur de la sous-routine.
    Breakpoint(u64),
    StepLimit,
    /// Suspendu en attendant l'entrée standard (voir
    /// `StepResult::NeedsInput`).
    NeedsInput,
}

/// Un moteur d'exécution x86-64, vu par l'interface.
///
/// L'interpréteur maison (`Emulator`) en est le premier, et le seul pour
/// l'instant : il exécute pas à pas et *explique*, ce qui est tout l'objet
/// pédagogique de namsdec. Un moteur de production (Unicorn, par exemple)
/// couvre en revanche bien plus d'instructions et va bien plus vite — deux
/// qualités qui n'ont de sens qu'à côté de la première, pas à sa place.
/// D'où ce trait plutôt qu'un remplacement : le jour où un second moteur
/// arrive, il se branche ici et l'interface ne change pas d'une ligne.
///
/// `cpu()` fait partie du contrat parce que toute la barre latérale
/// (registres, flags, pile, mémoire) lit cet état-là. Un moteur qui tient
/// son état ailleurs — dans un processus, dans une VM — le recopie donc
/// dans ce `Cpu` après chaque pas : c'est la vue que l'interface sait
/// afficher, et c'est le prix d'entrée pour s'y brancher.
pub trait Backend {
    /// Nom court du moteur, tel qu'il apparaît dans les messages
    /// d'erreur : « non supporté » ne veut rien dire sans dire *par qui*.
    fn name(&self) -> &'static str;

    fn cpu(&self) -> &Cpu;
    fn cpu_mut(&mut self) -> &mut Cpu;

    fn breakpoints(&self) -> &BTreeSet<u64>;
    fn set_breakpoints(&mut self, breakpoints: BTreeSet<u64>);

    /// Les zones à surveiller — adresse et longueur (1 à 8 octets).
    /// Poser un point de surveillance l'arme sur l'état courant.
    fn set_watchpoints(&mut self, points: &[(u64, usize)]);

    /// Les zones surveillées, avec la valeur retenue.
    fn watchpoints(&self) -> &[Watchpoint];

    fn step(&mut self) -> Result<StepResult, EmuError>;
    fn step_over(&mut self, max_steps: u32) -> Result<StepOverResult, EmuError>;
    fn run(&mut self, max_steps: u32) -> Result<RunStop, EmuError>;

    /// Défait le dernier pas, s'il en reste un à défaire.
    ///
    /// Un moteur qui ne sait pas revenir en arrière rend `false` sans
    /// rien faire : c'est à l'interface de griser son bouton, pas au
    /// programme de se retrouver dans un état à moitié restauré.
    fn step_back(&mut self) -> bool;

    /// Reste-t-il un pas à défaire ?
    fn can_step_back(&self) -> bool;

    /// Les appels en cours, du plus ancien au plus récent. Vide quand
    /// l'exécution est au niveau du programme principal.
    fn frames(&self) -> &[Frame];

    /// Le tas : où il commence, où `brk` l'a laissé, jusqu'où il peut
    /// monter. Un moteur qui n'en simule pas rend un tas vide.
    fn heap(&self) -> Heap;

    /// Ce que le programme a écrit sur sa sortie standard depuis le
    /// démarrage.
    fn output(&self) -> &[u8];

    /// Ajoute des octets à l'entrée standard du programme.
    fn provide_input(&mut self, bytes: &[u8]);

    /// Signale la fin de l'entrée : les `read` suivants rendront 0 (fin de
    /// fichier) au lieu d'attendre. L'équivalent d'un Ctrl+D.
    fn close_input(&mut self);

    /// Reste-t-il de l'entrée non lue ? Sert à dire à l'utilisateur ce que
    /// le programme va trouver.
    fn pending_input(&self) -> usize;

    /// Le code rendu par `exit`, si le programme s'est terminé de
    /// lui-même. `None` tant qu'il tourne, ou s'il s'est arrêté sur `hlt`
    /// — qui arrête la machine sans rien rendre.
    fn exit_code(&self) -> Option<i64>;
}

/// La taille d'une page. `brk` travaille par pages sous Linux, et
/// aligner la rupture de départ dessus, c'est montrer la même chose ici.
const PAGE: u64 = 0x1000;

/// Le tas : le segment que `brk` fait grandir, entre la fin de l'image du
/// programme et la pile.
///
/// Il n'y a pas de pagination ici, ni de noyau qui cherche de la place :
/// la mémoire simulée existe déjà tout entière. Ce que `brk` déplace est
/// donc une simple frontière — mais c'est exactement ce qu'un programme
/// voit d'elle, et c'est ce qu'il faut comprendre pour écrire un
/// allocateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Heap {
    start: u64,
    current: u64,
    limit: u64,
}

impl Heap {
    /// La première adresse du tas. `brk` ne descend jamais en dessous :
    /// ce qui est là, c'est le programme lui-même.
    pub fn start(&self) -> u64 {
        self.start
    }

    /// La rupture courante : la première adresse **hors** du tas. C'est
    /// ce que rend `brk(0)`.
    pub fn current(&self) -> u64 {
        self.current
    }

    /// Le plafond : au-delà, `brk` échoue. Il laisse de quoi vivre à la
    /// pile, qui descend depuis le haut de la mémoire.
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Combien d'octets le programme s'est fait donner.
    pub fn len(&self) -> u64 {
        self.current - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.current == self.start
    }
}

pub struct Emulator {
    pub cpu: Cpu,
    pub breakpoints: BTreeSet<u64>,
    /// Ce que le programme a écrit via `write` — des octets bruts, pas du
    /// texte : un programme peut parfaitement écrire ce qu'il veut sur sa
    /// sortie, y compris de l'UTF-8 incomplet ou du binaire, et ce n'est
    /// pas à l'émulateur d'en décider.
    output: Vec<u8>,
    /// Ce que l'utilisateur a fourni et que le programme n'a pas encore
    /// lu. Une file : `read` consomme par le début, l'interface ajoute
    /// par la fin.
    input: VecDeque<u8>,
    /// L'entrée est-elle terminée ? Tant qu'elle ne l'est pas, un `read`
    /// sur un tampon vide *attend* au lieu de rendre une fin de fichier —
    /// c'est ce que fait un vrai terminal, et c'est ce qui distingue
    /// « rien pour l'instant » de « plus rien à lire ».
    input_closed: bool,
    exit_code: Option<i64>,
    /// De quoi défaire les derniers pas — voir `Undo` et `step_back`.
    history: VecDeque<Undo>,
    /// Les appels en cours, du plus ancien au plus récent.
    frames: Vec<Frame>,
    /// Les zones surveillées.
    watchpoints: Vec<Watchpoint>,
    /// Le tas, tel que `brk` l'a laissé.
    heap: Heap,
    /// Combien d'instructions ont été exécutées depuis le chargement.
    /// C'est ce que rend `rdtsc` : le temps qu'on peut mesurer ici.
    ticks: u64,
}

impl Backend for Emulator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    fn breakpoints(&self) -> &BTreeSet<u64> {
        &self.breakpoints
    }

    fn set_watchpoints(&mut self, points: &[(u64, usize)]) {
        Emulator::set_watchpoints(self, points);
    }

    fn watchpoints(&self) -> &[Watchpoint] {
        Emulator::watchpoints(self)
    }

    fn set_breakpoints(&mut self, breakpoints: BTreeSet<u64>) {
        self.breakpoints = breakpoints;
    }

    fn step(&mut self) -> Result<StepResult, EmuError> {
        Emulator::step(self)
    }

    fn step_over(&mut self, max_steps: u32) -> Result<StepOverResult, EmuError> {
        Emulator::step_over(self, max_steps)
    }

    fn run(&mut self, max_steps: u32) -> Result<RunStop, EmuError> {
        Emulator::run(self, max_steps)
    }

    fn step_back(&mut self) -> bool {
        Emulator::step_back(self)
    }

    fn can_step_back(&self) -> bool {
        Emulator::can_step_back(self)
    }

    fn frames(&self) -> &[Frame] {
        Emulator::frames(self)
    }

    fn heap(&self) -> Heap {
        Emulator::heap(self)
    }

    fn output(&self) -> &[u8] {
        &self.output
    }

    fn provide_input(&mut self, bytes: &[u8]) {
        self.input.extend(bytes);
    }

    fn close_input(&mut self) {
        self.input_closed = true;
    }

    fn pending_input(&self) -> usize {
        self.input.len()
    }

    fn exit_code(&self) -> Option<i64> {
        self.exit_code
    }
}

impl Emulator {
    /// Le nom de ce moteur-ci dans les messages d'erreur. Un nom propre,
    /// sans mot traduisible autour : c'est l'interface qui l'enchâsse dans
    /// une phrase française ou anglaise.
    pub const NAME: &'static str = "namsdec";

    pub fn new(cpu: Cpu) -> Self {
        let end = cpu.mem.range().end;
        Self {
            breakpoints: BTreeSet::new(),
            output: Vec::new(),
            input: VecDeque::new(),
            input_closed: false,
            exit_code: None,
            history: VecDeque::new(),
            frames: Vec::new(),
            watchpoints: Vec::new(),
            // Pas de tas tant que personne n'a dit où le mettre : seul
            // celui qui a chargé le programme sait où son image finit.
            // `brk` se comporte alors comme sous un système à court de
            // mémoire — il rend la rupture sans la déplacer.
            heap: Heap {
                start: end,
                current: end,
                limit: end,
            },
            ticks: 0,
            cpu,
        }
    }

    /// Installe le tas : `start` est la fin de l'image du programme (la
    /// rupture part de la page suivante, comme sous Linux), `limit` le
    /// plus haut qu'on le laisse monter.
    ///
    /// C'est à l'appelant de le dire parce que lui seul sait où le
    /// programme finit et où sa pile commence ; l'émulateur ne voit qu'un
    /// bloc d'octets.
    pub fn set_heap(&mut self, start: u64, limit: u64) {
        let range = self.cpu.mem.range();
        let start = page_up(start).clamp(range.start, range.end);
        // Un plafond plus bas que le départ n'est pas une erreur : c'est
        // un tas de taille nulle, et `brk` refusera tout.
        let limit = limit.clamp(start, range.end);
        self.heap = Heap {
            start,
            current: start,
            limit,
        };
    }

    /// Le tas, tel que `brk` l'a laissé.
    pub fn heap(&self) -> Heap {
        self.heap
    }

    /// Déplace la rupture et rend celle qui vaut désormais.
    ///
    /// Linux rend **toujours** la rupture effective, pas un code
    /// d'erreur : inchangée, elle veut dire « refusé ». D'où l'usage de
    /// `brk(0)`, qui ne demande rien et sert donc à la lire.
    fn set_break(&mut self, requested: u64) -> u64 {
        if (self.heap.start..=self.heap.limit).contains(&requested) {
            if requested < self.heap.current {
                // Ce qu'on rend au système est perdu : le noyau reprend
                // les pages et n'en rendra que des vierges. Les remettre
                // à zéro ici, c'est rendre cet oubli visible plutôt que
                // de laisser un programme fautif retrouver ses données.
                let len = (self.heap.current - requested) as usize;
                self.record_memory(requested, len);
                let _ = self.cpu.mem.write_bytes(requested, &vec![0u8; len]);
            }
            self.heap.current = requested;
        }
        self.heap.current
    }

    /// L'appel système Linux x86-64 : le numéro est dans RAX, les
    /// arguments dans RDI, RSI, RDX (puis R10, R8, R9 — dont aucun appel
    /// géré ici n'a besoin), et la valeur de retour repart dans RAX.
    ///
    /// Quatre appels seulement sont implémentés : ceux dont un premier
    /// programme en assembleur a besoin pour faire quelque chose de
    /// visible — `read`, `write`, `exit` — et `brk`, qui lui donne de la
    /// mémoire. namsdec n'est pas un noyau, et ne prétend pas l'être : un
    /// appel non géré s'arrête en le disant, ce qui apprend davantage
    /// qu'un code d'erreur silencieux.
    fn syscall(&mut self) -> Result<(), EmuError> {
        const SYS_READ: u64 = 0;
        const SYS_WRITE: u64 = 1;
        const SYS_EXIT: u64 = 60;
        /// `exit_group` termine tous les fils d'exécution. Sans threads,
        /// c'est un `exit` — et c'est celui que produisent la plupart des
        /// bibliothèques C.
        const SYS_EXIT_GROUP: u64 = 231;
        /// `brk` déplace la fin du tas. C'est sur lui que repose tout
        /// `malloc`, et c'est le seul appel « mémoire » qu'un programme
        /// écrit à la main utilise sans bibliothèque.
        const SYS_BRK: u64 = 12;

        let number = self.cpu.regs.get64(Reg64::Rax);
        match number {
            SYS_READ => {
                let fd = self.cpu.regs.get64(Reg64::Rdi);
                let buf = self.cpu.regs.get64(Reg64::Rsi);
                let count = self.cpu.regs.get64(Reg64::Rdx);
                let read = self.read(fd, buf, count)?;
                self.cpu.regs.set64(Reg64::Rax, read as u64);
            }
            SYS_WRITE => {
                let fd = self.cpu.regs.get64(Reg64::Rdi);
                let buf = self.cpu.regs.get64(Reg64::Rsi);
                let count = self.cpu.regs.get64(Reg64::Rdx);
                let written = self.write(fd, buf, count)?;
                self.cpu.regs.set64(Reg64::Rax, written as u64);
            }
            SYS_BRK => {
                let requested = self.cpu.regs.get64(Reg64::Rdi);
                let now = self.set_break(requested);
                self.cpu.regs.set64(Reg64::Rax, now);
            }
            SYS_EXIT | SYS_EXIT_GROUP => {
                // Le code de sortie est un octet : `exit(256)` rend 0, et
                // le tronquer ici évite de laisser croire le contraire.
                let code = self.cpu.regs.get64(Reg64::Rdi) as u8;
                self.exit_code = Some(code as i64);
                self.cpu.halted = true;
            }
            number => {
                return Err(EmuError::UnsupportedSyscall {
                    backend: Self::NAME,
                    number,
                });
            }
        }
        Ok(())
    }

    /// `write(fd, buf, count)`. Rend le nombre d'octets écrits, ou un code
    /// d'erreur négatif — la convention du noyau, qu'un programme en
    /// assembleur est censé savoir lire.
    fn write(&mut self, fd: u64, buf: u64, count: u64) -> Result<i64, EmuError> {
        /// Sortie standard et sortie d'erreur sont les seules ouvertes :
        /// il n'y a pas de système de fichiers derrière cet émulateur.
        /// Tout autre descripteur reçoit `-EBADF`, exactement comme le
        /// noyau répondrait à un descripteur jamais ouvert.
        const EBADF: i64 = -9;
        if fd != 1 && fd != 2 {
            return Ok(EBADF);
        }
        // Rien à lire, et surtout rien à vérifier en mémoire : un
        // `write` de zéro octet est valide et ne touche pas au tampon.
        if count == 0 {
            return Ok(0);
        }

        // La lecture passe par la mémoire simulée et ses bornes : un
        // pointeur qui sort de la fenêtre est une erreur d'exécution, pas
        // un moyen de lire ce qui traîne à côté.
        let bytes = self.cpu.mem.bytes_at(buf, count as usize)?;
        self.output.extend_from_slice(bytes);
        Ok(count as i64)
    }

    /// `read(fd, buf, count)`. Rend le nombre d'octets lus, 0 en fin
    /// d'entrée, ou un code d'erreur négatif.
    ///
    /// N'est jamais appelée quand il n'y a rien à lire et que l'entrée
    /// reste ouverte : `step` suspend l'exécution avant d'en arriver là
    /// (voir `blocks_on_input`). Ce qui suit n'a donc à traiter que les
    /// cas où `read` peut répondre tout de suite.
    fn read(&mut self, fd: u64, buf: u64, count: u64) -> Result<i64, EmuError> {
        /// L'entrée standard est le seul descripteur en lecture.
        const EBADF: i64 = -9;
        if fd != 0 {
            return Ok(EBADF);
        }
        if count == 0 {
            return Ok(0);
        }

        // Un `read` rend ce qu'il a sous la main, sans attendre d'avoir de
        // quoi remplir tout le tampon — c'est pour ça qu'un programme
        // sérieux boucle sur `read` au lieu de croire qu'un seul appel
        // suffit, et cette leçon-là mérite d'être apprise ici aussi.
        let take = (count as usize).min(self.input.len());
        if take == 0 {
            // Entrée close : fin de fichier.
            return Ok(0);
        }

        // L'entrée n'est retirée de la file qu'une fois écrite. Drainée
        // d'abord, elle était perdue quand le tampon de destination
        // tombait hors de la fenêtre mémoire : `step` rembobine RIP pour
        // rejouer le `syscall`, mais l'utilisateur devait retaper ce
        // qu'il avait déjà envoyé.
        let bytes: Vec<u8> = self.input.iter().take(take).copied().collect();
        self.record_memory(buf, bytes.len());
        self.cpu.mem.write_bytes(buf, &bytes)?;
        self.input.drain(..take);
        // L'entrée consommée est notée pour pouvoir être rendue : sans
        // ça, défaire un `read` laisserait le programme sans ce qu'il
        // vient de lire, et le rejeu se bloquerait sur une file vide.
        if let Some(undo) = self.history.back_mut() {
            undo.consumed_input.extend(bytes.iter().copied());
        }
        Ok(take as i64)
    }

    /// Cette instruction est-elle un `read` qui n'a rien à lire ?
    ///
    /// La question se pose *avant* d'exécuter quoi que ce soit : le
    /// programme doit être suspendu sans que RIP ni RAX ne bougent, pour
    /// que le même `syscall` se rejoue à l'identique quand l'entrée
    /// arrive. C'est ce qu'un vrai `read` bloquant fait du point de vue du
    /// programme, sans qu'on ait à bloquer le fil d'exécution de
    /// l'interface.
    fn blocks_on_input(&self, instr: &Instruction) -> bool {
        const SYS_READ: u64 = 0;
        instr.mnemonic() == Mnemonic::Syscall
            && self.cpu.regs.get64(Reg64::Rax) == SYS_READ
            && self.cpu.regs.get64(Reg64::Rdi) == 0
            && self.cpu.regs.get64(Reg64::Rdx) > 0
            && self.input.is_empty()
            && !self.input_closed
    }

    /// Décode l'instruction à RIP sans l'exécuter — partagé par `step()` et
    /// `step_over()` (qui doit inspecter l'instruction avant de décider
    /// comment avancer).
    fn decode_at_rip(&self) -> Result<Instruction, EmuError> {
        let rip = self.cpu.regs.rip;
        let window = fetch_window(&self.cpu.mem, rip)?.to_vec();
        namsdec_disasm::decode_one(&window, rip).ok_or(EmuError::InvalidInstruction { addr: rip })
    }

    /// Décode puis exécute l'instruction à RIP.
    pub fn step(&mut self) -> Result<StepResult, EmuError> {
        if self.cpu.halted {
            return Ok(StepResult::Halted);
        }

        let instr = self.decode_at_rip()?;

        if self.blocks_on_input(&instr) {
            return Ok(StepResult::NeedsInput);
        }

        // Par défaut RIP avance à l'instruction suivante ; les branchements
        // écrasent cette valeur pendant `execute`.
        let rip_before = self.cpu.regs.rip;
        self.begin_undo();
        self.cpu.regs.rip = instr.next_ip();
        self.ticks = self.ticks.wrapping_add(1);
        if let Err(e) = self.execute(&instr) {
            // L'instruction n'a pas eu lieu : RIP doit rester dessus.
            // Sinon chaque tentative suivante avançait d'un cran — et
            // relancer le pas à pas après une erreur finissait par sortir
            // du code pour « exécuter » les données qui le suivent.
            //
            // Elle n'a pas non plus à figurer dans l'historique : on
            // défait tout de suite ce qu'elle a pu écrire avant d'échouer,
            // et sa page de journal disparaît avec.
            self.step_back();
            self.cpu.regs.rip = rip_before;
            return Err(e);
        }

        Ok(if self.cpu.halted {
            StepResult::Halted
        } else {
            StepResult::Continued
        })
    }

    /// Comme `step()`, sauf que si l'instruction courante est un `call`,
    /// toute la sous-routine appelée est exécutée d'un bloc plutôt que d'y
    /// entrer — le comportement "Step Over" standard des débogueurs.
    pub fn step_over(&mut self, max_steps: u32) -> Result<StepOverResult, EmuError> {
        if self.cpu.halted {
            return Ok(StepOverResult::Halted);
        }

        let instr = self.decode_at_rip()?;
        if instr.mnemonic() != Mnemonic::Call {
            return match self.step()? {
                StepResult::Halted => Ok(StepOverResult::Halted),
                StepResult::NeedsInput => Ok(StepOverResult::NeedsInput),
                StepResult::Continued => Ok(StepOverResult::Stepped),
            };
        }

        let return_addr = instr.next_ip();
        let start_rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);

        for _ in 0..max_steps {
            match self.step()? {
                StepResult::Halted => {
                    if let Some((addr, old, new)) = self.triggered_watchpoint() {
                        return Ok(StepOverResult::Watchpoint { addr, old, new });
                    }
                    return Ok(StepOverResult::Halted);
                }
                // Le `read` est au milieu de la sous-routine : on rend la
                // main, et `step_over` sera relancé quand l'entrée sera
                // là. La sous-routine reprendra donc là où elle en est,
                // et non depuis son début.
                StepResult::NeedsInput => return Ok(StepOverResult::NeedsInput),
                StepResult::Continued => {}
            }
            if let Some((addr, old, new)) = self.triggered_watchpoint() {
                return Ok(StepOverResult::Watchpoint { addr, old, new });
            }
            // La pile doit être revenue au moins à son niveau de départ pour
            // qu'on considère être "revenu" — protège contre un `ret`
            // orphelin qui atterrirait par coïncidence sur `return_addr` à
            // une profondeur de pile différente.
            if self.cpu.regs.rip == return_addr
                && self.cpu.regs.get64(namsdec_core::Reg64::Rsp) >= start_rsp
            {
                return Ok(StepOverResult::SteppedOverCall);
            }
            if self.breakpoints.contains(&self.cpu.regs.rip) {
                return Ok(StepOverResult::Breakpoint(self.cpu.regs.rip));
            }
        }
        Ok(StepOverResult::StepLimit)
    }

    /// Exécute jusqu'à `max_steps` instructions, jusqu'à `hlt`, un point
    /// d'arrêt (autre que celui de départ) ou la limite de pas.
    pub fn run(&mut self, max_steps: u32) -> Result<RunStop, EmuError> {
        for _ in 0..max_steps {
            if self.cpu.halted {
                return Ok(RunStop::Halted);
            }
            if self.step()? == StepResult::NeedsInput {
                return Ok(RunStop::NeedsInput);
            }
            // Avant même l'arrêt de la machine : si la dernière
            // instruction a touché une zone surveillée, c'est elle qu'on
            // vient voir, fût-elle la dernière du programme.
            if let Some((addr, old, new)) = self.triggered_watchpoint() {
                return Ok(RunStop::Watchpoint { addr, old, new });
            }
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
                let new_rsp = self
                    .cpu
                    .regs
                    .get64(namsdec_core::Reg64::Rsp)
                    .wrapping_sub(8);
                self.record_memory(new_rsp, 8);
                self.cpu.mem.write_u64(new_rsp, v)?;
                self.cpu.regs.set64(namsdec_core::Reg64::Rsp, new_rsp);
            }
            M::Pop => {
                let rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);
                let v = self.cpu.mem.read_u64(rsp)?;
                // RSP avance *avant* l'écriture, comme sur un vrai
                // processeur : `pop rsp` charge RSP depuis la pile (la
                // valeur dépilée l'emporte), et `pop [rsp+8]` calcule son
                // adresse après l'incrément.
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rsp.wrapping_add(8));
                if let Err(e) = self.write_op(instr, 0, v) {
                    // Mais si l'écriture échoue, l'instruction n'a pas eu
                    // lieu : RSP retourne où il était, sans quoi chaque
                    // rejeu (que `step` provoque en rembobinant RIP)
                    // dépilerait un mot de plus.
                    self.cpu.regs.set64(namsdec_core::Reg64::Rsp, rsp);
                    return Err(e);
                }
            }
            M::Lea => {
                let addr = self.effective_address(instr, 1)?;
                self.write_op(instr, 0, addr)?;
            }

            // Extensions de taille : la source est plus étroite que la
            // destination, c'est tout l'intérêt de l'instruction — d'où
            // une largeur lue sur l'opérande source et non sur la
            // destination comme partout ailleurs.
            M::Movzx => {
                let v = self.read_op(instr, 1)?;
                self.write_op(instr, 0, v)?;
            }
            M::Movsx | M::Movsxd => {
                let v = self.read_op(instr, 1)?;
                let w = self.src_width(instr, 1)?;
                self.write_op(instr, 0, sign_extend(v, w))?;
            }

            M::Neg => {
                let w = self.dest_width(instr)?;
                let a = self.read_op(instr, 0)?;
                let (result, flags) = sub_with_flags(0, a, w);
                self.cpu.flags.apply_arith(flags);
                self.write_op(instr, 0, result)?;
            }
            M::Not => {
                // La seule opération logique qui ne touche aucun flag.
                let w = self.dest_width(instr)?;
                let a = self.read_op(instr, 0)?;
                self.write_op(instr, 0, !a & w.mask())?;
            }

            M::Adc => self.binary_carry(instr, true)?,
            M::Sbb => self.binary_carry(instr, false)?,

            M::Shl | M::Sal => self.shift(instr, ShiftKind::Left)?,
            M::Shr => self.shift(instr, ShiftKind::Right)?,
            M::Sar => self.shift(instr, ShiftKind::Arithmetic)?,

            M::Mul => self.multiply_accumulator(instr, false)?,
            M::Imul => self.imul(instr)?,
            M::Div => self.divide(instr, false)?,
            M::Idiv => self.divide(instr, true)?,

            M::Xchg => {
                let a = self.read_op(instr, 0)?;
                let b = self.read_op(instr, 1)?;
                self.write_op(instr, 0, b)?;
                self.write_op(instr, 1, a)?;
            }

            // `leave` défait ce que le prologue d'une fonction a monté :
            // RSP revient sur le cadre, puis RBP est dépilé.
            M::Leave => {
                let rbp = self.cpu.regs.get64(namsdec_core::Reg64::Rbp);
                let saved = self.cpu.mem.read_u64(rbp)?;
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rbp.wrapping_add(8));
                self.cpu.regs.set64(namsdec_core::Reg64::Rbp, saved);
            }

            // `loop` : RCX décrémenté *sans toucher aux flags*, puis saut
            // tant qu'il n'est pas nul.
            M::Loop => {
                let rcx = self
                    .cpu
                    .regs
                    .get64(namsdec_core::Reg64::Rcx)
                    .wrapping_sub(1);
                self.cpu.regs.set64(namsdec_core::Reg64::Rcx, rcx);
                if rcx != 0 {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }

            M::Sete
            | M::Setne
            | M::Setb
            | M::Setae
            | M::Setbe
            | M::Seta
            | M::Setl
            | M::Setge
            | M::Setle
            | M::Setg
            | M::Sets
            | M::Setns
            | M::Seto
            | M::Setno
            | M::Setp
            | M::Setnp => {
                let v = u64::from(self.condition_met(instr));
                self.write_op(instr, 0, v)?;
            }

            M::Cmove
            | M::Cmovne
            | M::Cmovb
            | M::Cmovae
            | M::Cmovbe
            | M::Cmova
            | M::Cmovl
            | M::Cmovge
            | M::Cmovle
            | M::Cmovg
            | M::Cmovs
            | M::Cmovns
            | M::Cmovo
            | M::Cmovno
            | M::Cmovp
            | M::Cmovnp => {
                if self.condition_met(instr) {
                    let v = self.read_op(instr, 1)?;
                    self.write_op(instr, 0, v)?;
                }
            }
            M::Nop => {}

            // Le drapeau de direction : vers les adresses croissantes
            // (`cld`, l'état normal) ou décroissantes (`std`).
            M::Cld => self.cpu.flags.remove(RFlags::DF),
            M::Std => self.cpu.flags.insert(RFlags::DF),

            // Les instructions de chaînes. Le garde `is_string_operation`
            // écarte l'homonyme SSE : `movsd xmm0, [rax]` porte le même
            // mnémonique que `movsd` sur chaîne, et n'a rien à voir.
            M::Movsb | M::Movsw | M::Movsd | M::Movsq if is_string_operation(instr) => {
                self.string_step(instr, StringOp::Move)?
            }
            M::Stosb | M::Stosw | M::Stosd | M::Stosq => {
                self.string_step(instr, StringOp::Store)?
            }
            M::Lodsb | M::Lodsw | M::Lodsd | M::Lodsq => self.string_step(instr, StringOp::Load)?,
            M::Scasb | M::Scasw | M::Scasd | M::Scasq => self.string_step(instr, StringOp::Scan)?,
            M::Cmpsb | M::Cmpsw | M::Cmpsd | M::Cmpsq if is_string_operation(instr) => {
                self.string_step(instr, StringOp::Compare)?
            }
            // --- Changer une valeur signée de largeur -----------------
            // `cbw`/`cwde`/`cdqe` recopient le bit de signe dans la
            // moitié haute ; `cwd`/`cdq`/`cqo` le recopient dans le
            // registre voisin, ce qu'il faut faire avant chaque `idiv`
            // sous peine de diviser par un reste oublié.
            M::Cbw => {
                let v = sign_extend(u64::from(self.cpu.regs.get8l(Reg64::Rax)), Width::W8);
                self.cpu.regs.set16(Reg64::Rax, v as u16);
            }
            M::Cwde => {
                let v = sign_extend(u64::from(self.cpu.regs.get16(Reg64::Rax)), Width::W16);
                self.cpu.regs.set32(Reg64::Rax, v as u32);
            }
            M::Cdqe => {
                let v = sign_extend(u64::from(self.cpu.regs.get32(Reg64::Rax)), Width::W32);
                self.cpu.regs.set64(Reg64::Rax, v);
            }
            M::Cwd => {
                let high = if (self.cpu.regs.get16(Reg64::Rax) as i16) < 0 {
                    u16::MAX
                } else {
                    0
                };
                self.cpu.regs.set16(Reg64::Rdx, high);
            }
            M::Cdq => {
                let high = if (self.cpu.regs.get32(Reg64::Rax) as i32) < 0 {
                    u32::MAX
                } else {
                    0
                };
                self.cpu.regs.set32(Reg64::Rdx, high);
            }
            M::Cqo => {
                let high = if (self.cpu.regs.get64(Reg64::Rax) as i64) < 0 {
                    u64::MAX
                } else {
                    0
                };
                self.cpu.regs.set64(Reg64::Rdx, high);
            }

            // --- Rotations -------------------------------------------
            M::Rol => self.rotate(instr, RotateKind::Left)?,
            M::Ror => self.rotate(instr, RotateKind::Right)?,
            M::Rcl => self.rotate(instr, RotateKind::CarryLeft)?,
            M::Rcr => self.rotate(instr, RotateKind::CarryRight)?,

            // --- Décalages doubles -----------------------------------
            M::Shld => self.double_shift(instr, true)?,
            M::Shrd => self.double_shift(instr, false)?,

            // --- Le bit désigné --------------------------------------
            M::Bt => self.bit_test(instr, BitOp::Test)?,
            M::Bts => self.bit_test(instr, BitOp::Set)?,
            M::Btr => self.bit_test(instr, BitOp::Reset)?,
            M::Btc => self.bit_test(instr, BitOp::Complement)?,

            // --- Chercher et compter des bits ------------------------
            M::Bsf | M::Bsr => {
                let w = self.dest_width(instr)?;
                let src = self.read_op(instr, 1)? & w.mask();
                let mut flags = self.cpu.flags;
                flags.set(RFlags::ZF, src == 0);
                self.cpu.flags = flags;
                // Source nulle : la destination garde sa valeur — c'est
                // « indéfini » chez Intel, et ne rien faire est la seule
                // lecture qui ne mente pas.
                if src != 0 {
                    let index = if instr.mnemonic() == M::Bsf {
                        src.trailing_zeros()
                    } else {
                        63 - src.leading_zeros()
                    };
                    self.write_op(instr, 0, u64::from(index))?;
                }
            }
            M::Tzcnt | M::Lzcnt => {
                let w = self.dest_width(instr)?;
                let bits = w.bits();
                let src = self.read_op(instr, 1)? & w.mask();
                // Là où `bsf`/`bsr` laissent la destination en l'air,
                // `tzcnt`/`lzcnt` rendent la largeur de l'opérande : une
                // réponse, et non un cas particulier à traiter.
                let count = if src == 0 {
                    bits
                } else if instr.mnemonic() == M::Tzcnt {
                    src.trailing_zeros()
                } else {
                    src.leading_zeros() - (64 - bits)
                };
                let mut flags = self.cpu.flags;
                flags.set(RFlags::CF, src == 0);
                flags.set(RFlags::ZF, count == 0);
                self.cpu.flags = flags;
                self.write_op(instr, 0, u64::from(count))?;
            }
            M::Popcnt => {
                let w = self.dest_width(instr)?;
                let src = self.read_op(instr, 1)? & w.mask();
                let count = u64::from(src.count_ones());
                let mut flags = RFlags::empty();
                flags.set(RFlags::ZF, count == 0);
                self.cpu.flags.apply_arith(flags);
                self.write_op(instr, 0, count)?;
            }

            // --- Échanges -------------------------------------------
            M::Xadd => {
                let w = self.dest_width(instr)?;
                let dest = self.read_op(instr, 0)?;
                let src = self.read_op(instr, 1)?;
                let (result, flags) = add_with_flags(dest, src, w);
                self.cpu.flags.apply_arith(flags);
                // L'ancienne destination part dans la source : c'est ce
                // qui fait de `xadd` un compteur atomique.
                self.write_op(instr, 1, dest)?;
                self.write_op(instr, 0, result)?;
            }
            M::Cmpxchg => {
                let w = self.dest_width(instr)?;
                let dest = self.read_op(instr, 0)?;
                let acc = self.accumulator(w);
                let (_, flags) = sub_with_flags(acc & w.mask(), dest & w.mask(), w);
                self.cpu.flags.apply_arith(flags);
                if acc & w.mask() == dest & w.mask() {
                    let src = self.read_op(instr, 1)?;
                    self.write_op(instr, 0, src)?;
                } else {
                    // Échec : l'accumulateur reçoit ce qu'il y avait, de
                    // quoi réessayer avec la bonne valeur de départ.
                    self.set_accumulator(w, dest);
                }
            }
            M::Bswap => {
                let w = self.dest_width(instr)?;
                let v = self.read_op(instr, 0)?;
                let swapped = match w {
                    Width::W64 => v.swap_bytes(),
                    _ => u64::from((v as u32).swap_bytes()),
                };
                self.write_op(instr, 0, swapped)?;
            }
            M::Movbe => {
                let w = self.dest_width(instr)?;
                let v = self.read_op(instr, 1)? & w.mask();
                let swapped = match w {
                    Width::W16 => u64::from((v as u16).swap_bytes()),
                    Width::W32 => u64::from((v as u32).swap_bytes()),
                    _ => v.swap_bytes(),
                };
                self.write_op(instr, 0, swapped)?;
            }

            // --- Les indicateurs à la main ---------------------------
            M::Clc => self.cpu.flags.remove(RFlags::CF),
            M::Stc => self.cpu.flags.insert(RFlags::CF),
            M::Cmc => self.cpu.flags.toggle(RFlags::CF),
            M::Lahf => {
                // Le bit 1 vaut toujours 1 sur un vrai processeur : le
                // recopier ici évite qu'un programme qui compare AH à une
                // constante trouve autre chose que sur la machine.
                let ah = (self.cpu.flags.bits() & 0xd5) as u8 | 0x02;
                self.cpu.regs.set8h(Reg64::Rax, ah);
            }
            M::Sahf => {
                let ah = u64::from(self.cpu.regs.get8h(Reg64::Rax));
                let kept = self.cpu.flags.bits() & !0xd5;
                self.cpu.flags = RFlags::from_bits_truncate(kept | (ah & 0xd5));
            }
            M::Pushfq | M::Pushfd | M::Pushf => {
                let value = self.cpu.flags.bits() | 0x02;
                self.push(value)?;
            }
            M::Popfq | M::Popfd | M::Popf => {
                let value = self.pop()?;
                self.cpu.flags = RFlags::from_bits_truncate(value);
            }

            // --- Boucles et sauts sur RCX ----------------------------
            M::Loope | M::Loopne => {
                let rcx = self.cpu.regs.get64(Reg64::Rcx).wrapping_sub(1);
                self.cpu.regs.set64(Reg64::Rcx, rcx);
                let zf = self.cpu.flags.contains(RFlags::ZF);
                let wanted = instr.mnemonic() == M::Loope;
                if rcx != 0 && zf == wanted {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }
            M::Jrcxz | M::Jecxz => {
                let counter = if instr.mnemonic() == M::Jrcxz {
                    self.cpu.regs.get64(Reg64::Rcx)
                } else {
                    u64::from(self.cpu.regs.get32(Reg64::Rcx))
                };
                if counter == 0 {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }

            // --- Cadre de pile, table, et le reste -------------------
            M::Enter => {
                // Le niveau d'imbrication (le second opérande) sert aux
                // langages à fonctions imbriquées, que personne n'écrit
                // en assembleur : seul le niveau 0 est simulé, et les
                // autres se signalent plutôt que de mentir.
                if self.read_op(instr, 1)? & 0x1f != 0 {
                    return Err(EmuError::UnsupportedInstruction {
                        backend: Self::NAME,
                        mnemonic: "enter (niveau ≠ 0)".to_owned(),
                    });
                }
                let rbp = self.cpu.regs.get64(Reg64::Rbp);
                self.push(rbp)?;
                let rsp = self.cpu.regs.get64(Reg64::Rsp);
                self.cpu.regs.set64(Reg64::Rbp, rsp);
                let frame = self.read_op(instr, 0)?;
                self.cpu.regs.set64(Reg64::Rsp, rsp.wrapping_sub(frame));
            }
            M::Xlatb => {
                let addr = self
                    .cpu
                    .regs
                    .get64(Reg64::Rbx)
                    .wrapping_add(u64::from(self.cpu.regs.get8l(Reg64::Rax)));
                let byte = self.cpu.mem.read_u8(addr)?;
                self.cpu.regs.set8l(Reg64::Rax, byte);
            }
            M::Rdtsc => {
                // Un vrai compteur de cycles n'a pas de sens ici ; celui
                // des instructions exécutées en a un : il mesure ce que
                // le programme fait, et non ce que la machine met à le
                // faire. Il avance donc de 1 par pas, toujours.
                let ticks = self.ticks;
                self.cpu.regs.set32(Reg64::Rax, ticks as u32);
                self.cpu.regs.set32(Reg64::Rdx, (ticks >> 32) as u32);
            }
            // Un point d'arrêt logiciel : l'exécution s'arrête, comme
            // devant un débogueur — sauf qu'ici, le débogueur, c'est
            // l'application elle-même.
            M::Int3 => {
                self.cpu.halted = true;
            }
            // `ud2` existe pour planter : c'est ce qu'un compilateur pose
            // là où le programme ne doit jamais passer.
            M::Ud2 => {
                return Err(EmuError::InvalidInstruction { addr: instr.ip() });
            }
            // --- Les instructions de bits « modernes » (BMI) ---------
            // Elles font en une instruction ce qui en demandait trois, et
            // surtout : elles ne touchent pas aux indicateurs, ce qui
            // permet de les glisser au milieu d'un calcul.
            M::Shlx | M::Shrx | M::Sarx => {
                let w = self.dest_width(instr)?;
                let bits = u64::from(w.bits());
                let value = self.read_op(instr, 1)? & w.mask();
                let count = self.read_op(instr, 2)? % bits;
                let result = match instr.mnemonic() {
                    M::Shlx => (value << count) & w.mask(),
                    M::Shrx => value >> count,
                    _ => ((sign_extend(value, w) as i64) >> count) as u64 & w.mask(),
                };
                self.write_op(instr, 0, result)?;
            }
            M::Rorx => {
                let w = self.dest_width(instr)?;
                let bits = w.bits();
                let value = self.read_op(instr, 1)? & w.mask();
                let count = (self.read_op(instr, 2)? % u64::from(bits)) as u32;
                let result = if count == 0 {
                    value
                } else {
                    ((value >> count) | (value << (bits - count))) & w.mask()
                };
                self.write_op(instr, 0, result)?;
            }
            M::Andn => {
                let w = self.dest_width(instr)?;
                let result = !self.read_op(instr, 1)? & self.read_op(instr, 2)? & w.mask();
                self.cpu.flags.apply_arith(logic_flags(result, w));
                self.write_op(instr, 0, result)?;
            }
            M::Blsi | M::Blsr | M::Blsmsk => {
                let w = self.dest_width(instr)?;
                let src = self.read_op(instr, 1)? & w.mask();
                // `src & -src` isole le bit bas posé ; les trois
                // instructions en dérivent.
                let lowest = src.isolate_lowest_one();
                let result = match instr.mnemonic() {
                    M::Blsi => lowest,
                    M::Blsr => src & src.wrapping_sub(1),
                    _ => src ^ src.wrapping_sub(1),
                } & w.mask();
                let mut flags = logic_flags(result, w);
                // `blsi` dit « il y avait un bit » ; les deux autres
                // disent « il n'y en avait pas ». Une différence de
                // convention, pas de calcul.
                flags.set(RFlags::CF, (src == 0) != (instr.mnemonic() == M::Blsi));
                if instr.mnemonic() == M::Blsmsk {
                    flags.remove(RFlags::ZF);
                }
                self.cpu.flags.apply_arith(flags);
                self.write_op(instr, 0, result)?;
            }
            M::Bextr | M::Bzhi => {
                let w = self.dest_width(instr)?;
                let bits = w.bits();
                let src = self.read_op(instr, 1)? & w.mask();
                let control = self.read_op(instr, 2)?;
                let result = if instr.mnemonic() == M::Bextr {
                    let start = (control & 0xff) as u32;
                    let len = ((control >> 8) & 0xff) as u32;
                    if start >= bits || len == 0 {
                        0
                    } else {
                        let taken = len.min(bits - start);
                        let mask = if taken >= 64 {
                            u64::MAX
                        } else {
                            (1u64 << taken) - 1
                        };
                        (src >> start) & mask
                    }
                } else {
                    let index = (control & 0xff) as u32;
                    if index >= bits {
                        src
                    } else {
                        src & ((1u64 << index) - 1)
                    }
                };
                let mut flags = logic_flags(result, w);
                flags.remove(RFlags::SF);
                self.cpu.flags.apply_arith(flags);
                self.write_op(instr, 0, result)?;
            }
            M::Mulx => {
                // Le seul produit qui ne touche à aucun indicateur, et qui
                // dit lui-même où ranger ses deux moitiés.
                let w = self.dest_width(instr)?;
                let src = self.read_op(instr, 2)? & w.mask();
                let acc = match w {
                    Width::W32 => u64::from(self.cpu.regs.get32(Reg64::Rdx)),
                    _ => self.cpu.regs.get64(Reg64::Rdx),
                };
                let full = u128::from(acc) * u128::from(src);
                let low = (full & u128::from(w.mask())) as u64;
                let high = ((full >> w.bits()) & u128::from(w.mask())) as u64;
                self.write_op(instr, 1, low)?;
                self.write_op(instr, 0, high)?;
            }

            // --- Ce que la machine dit d'elle-même -------------------
            M::Cpuid => {
                // namsdec n'est pas un processeur : il ne peut annoncer
                // aucune extension sans mentir. Il se nomme, et déclare
                // n'avoir rien de plus — un programme qui interroge
                // `cpuid` prendra donc son chemin de repli, ce qui est la
                // seule réponse honnête.
                let leaf = self.cpu.regs.get32(Reg64::Rax);
                let (a, b, c, d) = if leaf == 0 {
                    // « namsdec emu », dans l'ordre EBX:EDX:ECX du manuel.
                    (0u32, 0x736d_616e, 0x0075_6d65, 0x2063_6564)
                } else {
                    (0, 0, 0, 0)
                };
                self.cpu.regs.set32(Reg64::Rax, a);
                self.cpu.regs.set32(Reg64::Rbx, b);
                self.cpu.regs.set32(Reg64::Rcx, c);
                self.cpu.regs.set32(Reg64::Rdx, d);
            }
            M::Rdtscp => {
                let ticks = self.ticks;
                self.cpu.regs.set32(Reg64::Rax, ticks as u32);
                self.cpu.regs.set32(Reg64::Rdx, (ticks >> 32) as u32);
                // L'identifiant du cœur : il n'y en a qu'un.
                self.cpu.regs.set32(Reg64::Rcx, 0);
            }

            // Rien à simuler : ni cache, ni pipeline, ni exécution dans
            // le désordre. Elles restent légales, et ne font rien.
            M::Pause | M::Mfence | M::Lfence | M::Sfence | M::Endbr64 | M::Endbr32 => {}

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
            M::Fsqrt => self.x87_map(f64::sqrt),
            M::Fsin => self.x87_map(f64::sin),
            M::Fcos => self.x87_map(f64::cos),
            M::Frndint => self.x87_map(round_even),
            M::F2xm1 => self.x87_map(|v| v.exp2() - 1.0),
            M::Fsincos => {
                let v = self.cpu.regs.x87.get(0);
                self.cpu.regs.x87.set(0, v.sin());
                self.cpu.regs.x87.push(v.cos());
            }
            M::Fptan => {
                let v = self.cpu.regs.x87.get(0);
                self.cpu.regs.x87.set(0, v.tan());
                // La constante 1 empilée derrière : un héritage du
                // 8087, qui rendait ainsi un rapport plutôt qu'un
                // nombre.
                self.cpu.regs.x87.push(1.0);
            }
            M::Fpatan => {
                let x = self.cpu.regs.x87.get(0);
                let y = self.cpu.regs.x87.get(1);
                self.cpu.regs.x87.set(1, y.atan2(x));
                self.cpu.regs.x87.pop();
            }
            M::Fyl2x => {
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
                let r = if instr.mnemonic() == M::Fprem {
                    a % b
                } else {
                    a - b * round_even(a / b)
                };
                self.cpu.regs.x87.set(0, r);
                // C2 dirait « reste partiel, recommencez » ; ici le
                // calcul est complet du premier coup.
                self.cpu.regs.x87.status &= !X87::C2;
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
                // Le mot de contrôle au repos : précision étendue,
                // arrondi au plus proche, toutes les exceptions masquées.
                self.cpu.mem.write_u16(addr, 0x037f)?;
            }
            // Le mode d'arrondi ne change rien ici : tout est calculé en
            // double précision, et l'arrondi au plus proche est le seul
            // simulé. L'instruction est acceptée pour que le programme
            // qui l'écrit continue de tourner.
            M::Fldcw => {}
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

            M::Jmp => {
                self.cpu.regs.rip = self.branch_target(instr)?;
            }
            M::Je
            | M::Jne
            | M::Jl
            | M::Jle
            | M::Jg
            | M::Jge
            | M::Jb
            | M::Jbe
            | M::Ja
            | M::Jae
            | M::Js
            | M::Jns
            | M::Jo
            | M::Jno
            | M::Jp
            | M::Jnp => {
                if self.condition_met(instr) {
                    self.cpu.regs.rip = instr.near_branch_target();
                }
            }
            M::Call => {
                let target = self.branch_target(instr)?;
                // step() a déjà avancé RIP à next_ip avant d'appeler execute() :
                // c'est exactement l'adresse de retour à empiler.
                let ret_addr = self.cpu.regs.rip;
                let new_rsp = self
                    .cpu
                    .regs
                    .get64(namsdec_core::Reg64::Rsp)
                    .wrapping_sub(8);
                self.record_memory(new_rsp, 8);
                self.cpu.mem.write_u64(new_rsp, ret_addr)?;
                self.cpu.regs.set64(namsdec_core::Reg64::Rsp, new_rsp);
                self.cpu.regs.rip = target;
                self.frames.push(Frame {
                    // RIP a déjà avancé : l'adresse du `call` lui-même
                    // est celle d'où l'instruction a été lue.
                    call_site: instr.ip(),
                    entry: target,
                    return_addr: ret_addr,
                    rsp: new_rsp,
                });
                if let Some(undo) = self.history.back_mut() {
                    undo.frames = FrameChange::Pushed;
                }
            }
            M::Ret => {
                let rsp = self.cpu.regs.get64(namsdec_core::Reg64::Rsp);
                let ret_addr = self.cpu.mem.read_u64(rsp)?;
                self.cpu
                    .regs
                    .set64(namsdec_core::Reg64::Rsp, rsp.wrapping_add(8));
                self.cpu.regs.rip = ret_addr;
                // Un `ret` sans `call` — du code qui saute dans une
                // sous-routine, ou une pile bricolée à la main — ne
                // dépile rien : la chaîne ne prétend pas savoir ce
                // qu'elle n'a pas vu.
                if let Some(frame) = self.frames.pop()
                    && let Some(undo) = self.history.back_mut()
                {
                    undo.frames = FrameChange::Popped(frame);
                }
            }
            M::Hlt => {
                self.cpu.halted = true;
            }
            M::Syscall => self.syscall()?,
            other => {
                return Err(EmuError::UnsupportedInstruction {
                    backend: Self::NAME,
                    mnemonic: format!("{other:?}"),
                });
            }
        }
        Ok(())
    }

    /// La condition d'un `jcc`, d'un `setcc` ou d'un `cmovcc`, lue sur
    /// l'instruction elle-même plutôt que déduite de son mnémonique :
    /// iced-x86 les ramène toutes à un même code, et les trois familles
    /// se partagent donc ce seul test.
    fn condition_met(&self, instr: &Instruction) -> bool {
        let f = &self.cpu.flags;
        let zf = f.contains(RFlags::ZF);
        let sf = f.contains(RFlags::SF);
        let cf = f.contains(RFlags::CF);
        let of = f.contains(RFlags::OF);
        let pf = f.contains(RFlags::PF);
        match instr.condition_code() {
            ConditionCode::e => zf,
            ConditionCode::ne => !zf,
            ConditionCode::b => cf,
            ConditionCode::ae => !cf,
            ConditionCode::be => cf || zf,
            ConditionCode::a => !cf && !zf,
            ConditionCode::l => sf != of,
            ConditionCode::ge => sf == of,
            ConditionCode::le => zf || (sf != of),
            ConditionCode::g => !zf && (sf == of),
            ConditionCode::s => sf,
            ConditionCode::ns => !sf,
            ConditionCode::o => of,
            ConditionCode::no => !of,
            ConditionCode::p => pf,
            ConditionCode::np => !pf,
            ConditionCode::None => unreachable!("instruction sans condition"),
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

    /// **Une seule itération** d'une instruction de chaîne, préfixe `rep`
    /// compris.
    ///
    /// C'est le choix de conception de ces instructions-là. Un vrai
    /// processeur les rend interruptibles : `rep movsb` n'est pas une
    /// boucle atomique, il avance d'un élément et se rejoue tant que RCX
    /// n'est pas nul. On fait pareil, pour deux raisons qui vont dans le
    /// même sens : le pas à pas montre RSI, RDI et RCX avancer un à un —
    /// c'est ce qu'on vient voir en apprenant —, et une copie d'un
    /// million d'octets ne fige pas l'interface le temps d'une frame.
    ///
    /// RIP a déjà avancé quand on arrive ici (voir `step`) : le rejeu
    /// consiste à le remettre sur l'instruction.
    fn string_step(&mut self, instr: &Instruction, op: StringOp) -> Result<(), EmuError> {
        use namsdec_core::Reg64::{Rcx, Rdi, Rsi};
        let w = string_width(instr.mnemonic()).ok_or_else(|| EmuError::UnsupportedInstruction {
            backend: Self::NAME,
            mnemonic: format!("{:?}", instr.mnemonic()),
        })?;
        let repeat = instr.has_rep_prefix() || instr.has_repe_prefix() || instr.has_repne_prefix();

        // `rep` avec RCX nul ne fait rien du tout — pas même une
        // itération. C'est la nuance qui distingue un compteur à zéro
        // d'un compteur à un.
        if repeat && self.cpu.regs.get64(Rcx) == 0 {
            return Ok(());
        }

        let bytes = u64::from(w.bits() / 8);
        let step = if self.cpu.flags.contains(RFlags::DF) {
            bytes.wrapping_neg()
        } else {
            bytes
        };
        let (rsi, rdi) = (self.cpu.regs.get64(Rsi), self.cpu.regs.get64(Rdi));

        match op {
            StringOp::Move => {
                let v = self.read_mem(rsi, w)?;
                self.write_mem(rdi, w, v)?;
                self.cpu.regs.set64(Rsi, rsi.wrapping_add(step));
                self.cpu.regs.set64(Rdi, rdi.wrapping_add(step));
            }
            StringOp::Store => {
                let v = self.accumulator(w);
                self.write_mem(rdi, w, v)?;
                self.cpu.regs.set64(Rdi, rdi.wrapping_add(step));
            }
            StringOp::Load => {
                let v = self.read_mem(rsi, w)?;
                self.set_accumulator(w, v);
                self.cpu.regs.set64(Rsi, rsi.wrapping_add(step));
            }
            StringOp::Scan => {
                let v = self.read_mem(rdi, w)?;
                let (_, flags) = sub_with_flags(self.accumulator(w), v, w);
                self.cpu.flags.apply_arith(flags);
                self.cpu.regs.set64(Rdi, rdi.wrapping_add(step));
            }
            StringOp::Compare => {
                let left = self.read_mem(rsi, w)?;
                let right = self.read_mem(rdi, w)?;
                let (_, flags) = sub_with_flags(left, right, w);
                self.cpu.flags.apply_arith(flags);
                self.cpu.regs.set64(Rsi, rsi.wrapping_add(step));
                self.cpu.regs.set64(Rdi, rdi.wrapping_add(step));
            }
        }

        if repeat {
            let remaining = self.cpu.regs.get64(Rcx).wrapping_sub(1);
            self.cpu.regs.set64(Rcx, remaining);
            // `repe`/`repne` s'arrêtent aussi sur la comparaison : c'est
            // ce qui fait de `repne scasb` un « cherche cet octet » et de
            // `repe cmpsb` un « compare jusqu'à la différence ».
            let zf = self.cpu.flags.contains(RFlags::ZF);
            let again = remaining != 0
                && match op {
                    StringOp::Scan | StringOp::Compare => {
                        if instr.has_repne_prefix() {
                            !zf
                        } else {
                            zf
                        }
                    }
                    _ => true,
                };
            if again {
                self.cpu.regs.rip = instr.ip();
            }
        }
        Ok(())
    }

    fn set_accumulator(&mut self, w: Width, value: u64) {
        use namsdec_core::Reg64::Rax;
        match w {
            Width::W8 => self.cpu.regs.set8l(Rax, value as u8),
            Width::W16 => self.cpu.regs.set16(Rax, value as u16),
            Width::W32 => self.cpu.regs.set32(Rax, value as u32),
            Width::W64 => self.cpu.regs.set64(Rax, value),
        }
    }

    /// `adc`/`sbb` : l'addition et la soustraction qui tiennent compte de
    /// la retenue précédente — c'est par elles qu'on additionne des
    /// nombres plus larges qu'un registre.
    fn binary_carry(&mut self, instr: &Instruction, add: bool) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let a = self.read_op(instr, 0)?;
        let b = self.read_op(instr, 1)?;
        let carry = u64::from(self.cpu.flags.contains(RFlags::CF));

        // Deux opérations enchaînées plutôt qu'une : chacune peut
        // déborder, et `a + 0xff.. + 1` déborde deux fois.
        let (partial, first) = if add {
            add_with_flags(a, b, w)
        } else {
            sub_with_flags(a, b, w)
        };
        let (result, second) = if add {
            add_with_flags(partial, carry, w)
        } else {
            sub_with_flags(partial, carry, w)
        };
        let mut flags = second;
        for flag in [RFlags::CF, RFlags::OF, RFlags::AF] {
            if first.contains(flag) {
                flags.insert(flag);
            }
        }
        self.cpu.flags.apply_arith(flags);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    /// Décalages : `shl`/`sal`, `shr` et `sar`.
    ///
    /// Le compte est masqué comme sur le processeur (5 bits, 6 en 64
    /// bits), et un compte nul ne touche à aucun flag — c'est ce qui
    /// distingue `shl reg, cl` avec CL à zéro d'un décalage de 1. CF
    /// reçoit le dernier bit sorti ; OF n'a de sens que pour un décalage
    /// de 1, et AF reste indéfini (laissé à zéro).
    fn shift(&mut self, instr: &Instruction, kind: ShiftKind) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let bits = u64::from(w.bits());
        let a = self.read_op(instr, 0)? & w.mask();
        let count = self.read_op(instr, 1)? & if bits == 64 { 0x3f } else { 0x1f };
        if count == 0 {
            return Ok(());
        }

        let msb = 1u64 << (bits - 1);
        let (result, carry) = match kind {
            ShiftKind::Left => {
                let carry = count <= bits && (a >> (bits - count)) & 1 == 1;
                (a.wrapping_shl(count as u32) & w.mask(), carry)
            }
            ShiftKind::Right => {
                let carry = count <= bits && (a >> (count - 1)) & 1 == 1;
                (a >> count.min(63), carry)
            }
            ShiftKind::Arithmetic => {
                let signed = sign_extend(a, w) as i64;
                let carry = (signed >> (count - 1).min(63)) & 1 == 1;
                ((signed >> count.min(63)) as u64 & w.mask(), carry)
            }
        };

        let mut flags = logic_flags(result, w);
        flags.set(RFlags::CF, carry);
        if count == 1 {
            let overflow = match kind {
                ShiftKind::Left => ((result & msb) != 0) != carry,
                ShiftKind::Right => (a & msb) != 0,
                ShiftKind::Arithmetic => false,
            };
            flags.set(RFlags::OF, overflow);
        }
        self.cpu.flags.apply_arith(flags);
        self.write_op(instr, 0, result)?;
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

    /// La valeur d'un opérande flottant : une case de la pile, ou ce que
    /// la mémoire contient — un flottant simple, double ou étendu, ou un
    /// entier de 16, 32 ou 64 bits pour les formes `fi…`.
    fn read_fp_operand(&self, instr: &Instruction, op_index: u32) -> Result<f64, EmuError> {
        if let Some(i) = st_operand(instr, op_index) {
            return Ok(self.cpu.regs.x87.get(i));
        }
        let addr = self.effective_address(instr, op_index)?;
        Ok(match instr.memory_size() {
            MemorySize::Float32 => f64::from(f32::from_bits(self.cpu.mem.read_u32(addr)?)),
            MemorySize::Float64 => f64::from_bits(self.cpu.mem.read_u64(addr)?),
            MemorySize::Float80 => f80_to_f64(self.cpu.mem.bytes_at(addr, 10)?),
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
        let rounded = if truncate {
            value.trunc()
        } else {
            round_even(value)
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
        self.cpu.regs.x87.set(dest, result);
        if pop {
            self.cpu.regs.x87.pop();
        }
        Ok(())
    }

    /// Ce à quoi `fcom` et ses variantes comparent le sommet : `st1` par
    /// défaut, la case ou la valeur nommée sinon.
    fn x87_comparand(&self, instr: &Instruction) -> Result<f64, EmuError> {
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

    /// Empile huit octets : RSP descend, puis la valeur s'écrit.
    fn push(&mut self, value: u64) -> Result<(), EmuError> {
        let new_rsp = self.cpu.regs.get64(Reg64::Rsp).wrapping_sub(8);
        self.record_memory(new_rsp, 8);
        self.cpu.mem.write_u64(new_rsp, value)?;
        self.cpu.regs.set64(Reg64::Rsp, new_rsp);
        Ok(())
    }

    /// Dépile huit octets. RSP n'avance que si la lecture a réussi : un
    /// pas qui échoue doit pouvoir se rejouer à l'identique.
    fn pop(&mut self) -> Result<u64, EmuError> {
        let rsp = self.cpu.regs.get64(Reg64::Rsp);
        let value = self.cpu.mem.read_u64(rsp)?;
        self.cpu.regs.set64(Reg64::Rsp, rsp.wrapping_add(8));
        Ok(value)
    }

    /// Les quatre rotations. Contrairement aux décalages, elles ne
    /// touchent ni ZF, ni SF, ni PF : seule la retenue en sort, et le
    /// débordement n'a de sens que pour une rotation d'un cran.
    fn rotate(&mut self, instr: &Instruction, kind: RotateKind) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let bits = w.bits();
        let mask = w.mask();
        let a = self.read_op(instr, 0)? & mask;
        let raw = self.read_op(instr, 1)? & if bits == 64 { 0x3f } else { 0x1f };
        if raw == 0 {
            // Compte nul : pas même un indicateur ne bouge.
            return Ok(());
        }

        let through_carry = matches!(kind, RotateKind::CarryLeft | RotateKind::CarryRight);
        let count = if through_carry {
            // La retenue fait un bit de plus dans le tour.
            match bits {
                8 => raw % 9,
                16 => raw % 17,
                _ => raw,
            }
        } else {
            raw % u64::from(bits)
        };

        let msb = 1u64 << (bits - 1);
        let cf_before = self.cpu.flags.contains(RFlags::CF);
        let mut cf = cf_before;
        let mut v = a;
        // Un cran à la fois : c'est lent, et c'est mot pour mot ce que
        // dit le manuel — ce qui compte davantage ici.
        for _ in 0..count {
            match kind {
                RotateKind::Left => {
                    let top = v & msb != 0;
                    v = ((v << 1) & mask) | u64::from(top);
                    cf = top;
                }
                RotateKind::Right => {
                    let bottom = v & 1;
                    v = (v >> 1) | (bottom << (bits - 1));
                    cf = bottom == 1;
                }
                RotateKind::CarryLeft => {
                    let top = v & msb != 0;
                    v = ((v << 1) & mask) | u64::from(cf);
                    cf = top;
                }
                RotateKind::CarryRight => {
                    let bottom = v & 1 == 1;
                    v = (v >> 1) | (u64::from(cf) << (bits - 1));
                    cf = bottom;
                }
            }
        }
        // `rol al, 8` ne change rien à AL mais renseigne tout de même la
        // retenue : elle vaut le bit qui serait sorti.
        match kind {
            RotateKind::Left => cf = v & 1 == 1,
            RotateKind::Right => cf = v & msb != 0,
            _ => {}
        }

        let mut flags = self.cpu.flags;
        flags.set(RFlags::CF, cf);
        if raw == 1 {
            let of = match kind {
                RotateKind::Left | RotateKind::CarryLeft => (v & msb != 0) != cf,
                RotateKind::Right => (v & msb != 0) != (v & (msb >> 1) != 0),
                RotateKind::CarryRight => (a & msb != 0) != cf_before,
            };
            flags.set(RFlags::OF, of);
        }
        self.cpu.flags = flags;
        self.write_op(instr, 0, v)?;
        Ok(())
    }

    /// `shld`/`shrd` : la destination se décale, et les bits qui entrent
    /// viennent de la source au lieu d'être des zéros. C'est ainsi qu'on
    /// décale une valeur plus large qu'un registre.
    fn double_shift(&mut self, instr: &Instruction, left: bool) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let bits = w.bits();
        let mask = w.mask();
        let dest = self.read_op(instr, 0)? & mask;
        let src = self.read_op(instr, 1)? & mask;
        let count = (self.read_op(instr, 2)? & if bits == 64 { 0x3f } else { 0x1f }) as u32;
        if count == 0 {
            return Ok(());
        }
        if count >= bits {
            // Intel laisse le résultat indéfini ; ne rien faire est la
            // seule réponse qui ne raconte pas d'histoire.
            return Ok(());
        }

        let (result, cf) = if left {
            (
                ((dest << count) | (src >> (bits - count))) & mask,
                (dest >> (bits - count)) & 1 == 1,
            )
        } else {
            (
                ((dest >> count) | (src << (bits - count))) & mask,
                (dest >> (count - 1)) & 1 == 1,
            )
        };

        let mut flags = logic_flags(result, w);
        flags.set(RFlags::CF, cf);
        if count == 1 {
            // Le signe a-t-il changé ? C'est tout ce que dit OF ici.
            flags.set(RFlags::OF, (dest ^ result) & (1 << (bits - 1)) != 0);
        }
        self.cpu.flags.apply_arith(flags);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    /// `bt`, `bts`, `btr`, `btc` : le bit désigné part dans CF, et
    /// l'instruction le laisse, le pose, l'efface ou le retourne.
    fn bit_test(&mut self, instr: &Instruction, op: BitOp) -> Result<(), EmuError> {
        let w = self.dest_width(instr)?;
        let raw = self.read_op(instr, 1)?;

        // Décalage dans un registre et destination en mémoire : le bit
        // désigné n'est pas borné par l'opérande, il compte dans une
        // suite d'octets. `bt [drapeaux], rax` avec RAX = 100 va donc
        // chercher le treizième octet — ce qui permet de tester un
        // tableau de bits sans calculer soi-même l'octet.
        if instr.op_kind(0) == OpKind::Memory && instr.op_kind(1) == OpKind::Register {
            let base = self.effective_address(instr, 0)?;
            let offset = sign_extend(raw, self.src_width(instr, 1)?) as i64;
            let addr = base.wrapping_add_signed(offset.div_euclid(8));
            let bit = offset.rem_euclid(8) as u32;
            let byte = u64::from(self.cpu.mem.read_u8(addr)?);
            self.cpu.flags.set(RFlags::CF, (byte >> bit) & 1 == 1);
            if let Some(new) = op.apply(byte, bit) {
                self.record_memory(addr, 1);
                self.cpu.mem.write_u8(addr, new as u8)?;
            }
            return Ok(());
        }

        let value = self.read_op(instr, 0)? & w.mask();
        let bit = (raw & u64::from(w.bits() - 1)) as u32;
        self.cpu.flags.set(RFlags::CF, (value >> bit) & 1 == 1);
        if let Some(new) = op.apply(value, bit) {
            self.write_op(instr, 0, new)?;
        }
        Ok(())
    }

    /// `mul`, et la forme à un opérande d'`imul` : le produit est deux
    /// fois plus large que les opérandes, il déborde donc sur un second
    /// registre (AH, DX, EDX ou RDX selon la taille).
    fn multiply_accumulator(&mut self, instr: &Instruction, signed: bool) -> Result<(), EmuError> {
        let w = self.src_width(instr, 0)?;
        let src = self.read_op(instr, 0)?;
        let acc = self.accumulator(w);
        let mask = u128::from(w.mask());

        let full = if signed {
            ((sign_extend(acc, w) as i64 as i128) * (sign_extend(src, w) as i64 as i128)) as u128
        } else {
            u128::from(acc & w.mask()) * u128::from(src & w.mask())
        };
        let low = (full & mask) as u64;
        let high = ((full >> w.bits()) & mask) as u64;
        self.write_wide_result(w, low, high);

        // CF et OF disent la seule chose qui compte : le produit
        // tenait-il dans la largeur de départ ?
        let overflowed = if signed {
            sign_extend(low, w) != ((high << (w.bits() - 1)) | low)
                || (high != 0 && high != w.mask())
        } else {
            high != 0
        };
        self.cpu.flags.set(RFlags::CF, overflowed);
        self.cpu.flags.set(RFlags::OF, overflowed);
        Ok(())
    }

    /// `imul` a trois formes : un opérande (comme `mul`, en signé), deux
    /// (`dst *= src`) et trois (`dst = src * immédiat`). Les deux
    /// dernières gardent le produit dans la largeur de la destination.
    fn imul(&mut self, instr: &Instruction) -> Result<(), EmuError> {
        if instr.op_count() == 1 {
            return self.multiply_accumulator(instr, true);
        }
        let w = self.dest_width(instr)?;
        let (a, b) = if instr.op_count() == 2 {
            (self.read_op(instr, 0)?, self.read_op(instr, 1)?)
        } else {
            (self.read_op(instr, 1)?, self.read_op(instr, 2)?)
        };
        let full = (sign_extend(a, w) as i64 as i128) * (sign_extend(b, w) as i64 as i128);
        let result = (full as u64) & w.mask();
        // Le produit a-t-il été tronqué ? C'est tout ce que disent CF et
        // OF sur ces formes-là.
        let truncated = i128::from(sign_extend(result, w) as i64) != full;
        self.cpu.flags.set(RFlags::CF, truncated);
        self.cpu.flags.set(RFlags::OF, truncated);
        self.write_op(instr, 0, result)?;
        Ok(())
    }

    /// `div` et `idiv` : le dividende est deux fois plus large que le
    /// diviseur (AX, DX:AX, EDX:EAX, RDX:RAX), le quotient va dans
    /// l'accumulateur et le reste dans RDX (AH en 8 bits).
    ///
    /// Une division par zéro, comme un quotient trop grand pour le
    /// registre qui doit le recevoir, lève l'exception du processeur —
    /// ici une erreur qui arrête l'exécution en le disant, plutôt qu'un
    /// résultat inventé.
    fn divide(&mut self, instr: &Instruction, signed: bool) -> Result<(), EmuError> {
        use namsdec_core::Reg64::{Rax, Rdx};
        let w = self.src_width(instr, 0)?;
        let divisor = self.read_op(instr, 0)? & w.mask();
        if divisor == 0 {
            return Err(EmuError::DivideByZero {
                addr: self.cpu.regs.rip,
            });
        }

        let bits = w.bits();
        let low = self.accumulator(w) & w.mask();
        let high = match w {
            // En 8 bits le dividende est AX en entier : sa moitié haute
            // est AH, pas DX.
            Width::W8 => u64::from(self.cpu.regs.get16(Rax) >> 8),
            Width::W16 => u64::from(self.cpu.regs.get16(Rdx)),
            Width::W32 => u64::from(self.cpu.regs.get32(Rdx)),
            Width::W64 => self.cpu.regs.get64(Rdx),
        };
        let raw = (u128::from(high) << bits) | u128::from(low);

        let (quotient, remainder) = if signed {
            // Le dividende double largeur porte son signe sur son bit de
            // poids fort : il faut l'étendre avant de diviser.
            let shift = 128 - 2 * bits;
            let dividend = ((raw << shift) as i128) >> shift;
            let divisor = sign_extend(divisor, w) as i64 as i128;
            let quotient = dividend / divisor;
            if i128::from(sign_extend((quotient as u64) & w.mask(), w) as i64) != quotient {
                return Err(EmuError::DivideOverflow {
                    addr: self.cpu.regs.rip,
                });
            }
            (
                (quotient as u64) & w.mask(),
                ((dividend % divisor) as u64) & w.mask(),
            )
        } else {
            let quotient = raw / u128::from(divisor);
            if quotient > u128::from(w.mask()) {
                return Err(EmuError::DivideOverflow {
                    addr: self.cpu.regs.rip,
                });
            }
            (quotient as u64, (raw % u128::from(divisor)) as u64)
        };

        self.write_wide_result(w, quotient, remainder);
        Ok(())
    }

    /// Range une paire (bas, haut) là où le processeur la met : AX en 8
    /// bits, sinon l'accumulateur et RDX à la bonne largeur.
    fn write_wide_result(&mut self, w: Width, low: u64, high: u64) {
        use namsdec_core::Reg64::{Rax, Rdx};
        match w {
            Width::W8 => self
                .cpu
                .regs
                .set16(Rax, ((low & 0xff) | ((high & 0xff) << 8)) as u16),
            Width::W16 => {
                self.cpu.regs.set16(Rax, low as u16);
                self.cpu.regs.set16(Rdx, high as u16);
            }
            Width::W32 => {
                self.cpu.regs.set32(Rax, low as u32);
                self.cpu.regs.set32(Rdx, high as u32);
            }
            Width::W64 => {
                self.cpu.regs.set64(Rax, low);
                self.cpu.regs.set64(Rdx, high);
            }
        }
    }

    /// AL, AX, EAX ou RAX selon la largeur.
    fn accumulator(&self, w: Width) -> u64 {
        use namsdec_core::Reg64::Rax;
        match w {
            Width::W8 => u64::from(self.cpu.regs.get8l(Rax)),
            Width::W16 => u64::from(self.cpu.regs.get16(Rax)),
            Width::W32 => u64::from(self.cpu.regs.get32(Rax)),
            Width::W64 => self.cpu.regs.get64(Rax),
        }
    }

    /// La largeur d'un opérande donné — celle de la *source* quand elle
    /// diffère de la destination (`movzx`, `movsx`, `mul`, `div`).
    fn src_width(&self, instr: &Instruction, op_index: u32) -> Result<Width, EmuError> {
        match instr.op_kind(op_index) {
            OpKind::Register => Ok(map_register(instr.op_register(op_index))?.1),
            OpKind::Memory => self.mem_operand_width(instr),
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Read,
                kind: format!("{other:?}"),
            }),
        }
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

    /// Adresse cible d'un `jmp`/`call` : `near_branch_target()` pour les
    /// formes relatives (rel8/rel32, l'immense majorité en pratique), sinon
    /// l'opérande générique (registre ou mémoire indirects). Sans ce
    /// dispatch, `near_branch_target()` sur une forme indirecte renvoie
    /// silencieusement 0 plutôt que d'échouer — un bug latent qu'avait
    /// `Jmp` avant l'ajout de `Call`.
    fn branch_target(&self, instr: &Instruction) -> Result<u64, EmuError> {
        match instr.op_kind(0) {
            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                Ok(instr.near_branch_target())
            }
            _ => self.read_op(instr, 0),
        }
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
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Destination,
                kind: format!("{other:?}"),
            }),
        }
    }

    fn mem_operand_width(&self, instr: &Instruction) -> Result<Width, EmuError> {
        width_from_bytes(instr.memory_size().size()).ok_or_else(|| {
            EmuError::UnsupportedOperandSize {
                backend: Self::NAME,
                bytes: instr.memory_size().size(),
            }
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
            | OpKind::Immediate32to64
            // Le second immédiat d'`enter` : la seule instruction du jeu
            // qui en porte deux.
            | OpKind::Immediate8_2nd => Ok(instr.immediate(op_index)),
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Read,
                kind: format!("{other:?}"),
            }),
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
            other => Err(EmuError::UnsupportedOperand {
                backend: Self::NAME,
                role: OperandRole::Write,
                kind: format!("{other:?}"),
            }),
        }
    }

    /// Ouvre une page de journal pour le pas qui commence.
    fn begin_undo(&mut self) {
        if self.history.len() >= HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.history.push_back(Undo {
            regs: self.cpu.regs.clone(),
            flags: self.cpu.flags,
            halted: self.cpu.halted,
            exit_code: self.exit_code,
            output_len: self.output.len(),
            consumed_input: Vec::new(),
            input_closed: self.input_closed,
            memory: Vec::new(),
            frames: FrameChange::None,
            heap: self.heap,
            ticks: self.ticks,
        });
    }

    /// Note les octets sur le point d'être écrasés. Lus *avant*
    /// l'écriture, évidemment — c'est toute l'idée.
    fn record_memory(&mut self, addr: u64, len: usize) {
        if self.history.is_empty() {
            return;
        }
        let Ok(old) = self.cpu.mem.bytes_at(addr, len) else {
            return;
        };
        let old = old.to_vec();
        if let Some(undo) = self.history.back_mut() {
            undo.memory.push((addr, old));
        }
    }

    /// Défait le dernier pas : les registres, les flags, l'arrêt, la
    /// sortie écrite, l'entrée consommée et les octets écrasés reviennent
    /// à ce qu'ils étaient.
    ///
    /// C'est ce qu'un émulateur peut offrir et qu'un débogueur ordinaire
    /// peine à rendre : on rejoue une instruction autant de fois qu'il le
    /// faut pour la comprendre, au lieu de tout relancer depuis le début.
    pub fn step_back(&mut self) -> bool {
        let Some(undo) = self.history.pop_back() else {
            return false;
        };
        // Dans l'ordre inverse des écritures : deux écritures au même
        // endroit doivent se défaire en commençant par la dernière.
        for (addr, bytes) in undo.memory.iter().rev() {
            let _ = self.cpu.mem.write_bytes(*addr, bytes);
        }
        self.cpu.regs = undo.regs;
        self.cpu.flags = undo.flags;
        self.cpu.halted = undo.halted;
        self.exit_code = undo.exit_code;
        self.output.truncate(undo.output_len);
        for byte in undo.consumed_input.iter().rev() {
            self.input.push_front(*byte);
        }
        self.input_closed = undo.input_closed;
        self.heap = undo.heap;
        self.ticks = undo.ticks;
        match undo.frames {
            FrameChange::None => {}
            FrameChange::Pushed => {
                self.frames.pop();
            }
            FrameChange::Popped(frame) => self.frames.push(frame),
        }
        // Revenir en arrière ne déclenche pas les points de
        // surveillance : ils regardent ce qui vient, pas ce qu'on défait.
        self.rearm_watchpoints();
        true
    }

    /// Les appels en cours, du plus ancien au plus récent.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Remplace les zones surveillées, et les arme sur l'état courant :
    /// poser un point de surveillance, c'est dire « préviens-moi si ça
    /// change **à partir de maintenant** ».
    pub fn set_watchpoints(&mut self, points: &[(u64, usize)]) {
        self.watchpoints = points
            .iter()
            .map(|&(addr, len)| Watchpoint {
                addr,
                len: len.clamp(1, 8),
                previous: self.peek(addr, len.clamp(1, 8)),
            })
            .collect();
    }

    pub fn watchpoints(&self) -> &[Watchpoint] {
        &self.watchpoints
    }

    /// Réarme les zones surveillées sur l'état courant, sans rien
    /// signaler. Après un retour en arrière, c'est d'ici qu'on repart.
    fn rearm_watchpoints(&mut self) {
        for point in &mut self.watchpoints {
            let (addr, len) = (point.addr, point.len);
            // Emprunt : la lecture passe par `peek`, qui ne touche à rien.
            let value = read_bytes(&self.cpu.mem, addr, len);
            point.previous = value;
        }
    }

    /// La première zone surveillée dont la valeur a changé.
    fn triggered_watchpoint(&mut self) -> Option<(u64, u64, u64)> {
        for index in 0..self.watchpoints.len() {
            let (addr, len, old) = {
                let point = &self.watchpoints[index];
                (point.addr, point.len, point.previous)
            };
            let new = read_bytes(&self.cpu.mem, addr, len);
            if new != old {
                self.watchpoints[index].previous = new;
                return Some((addr, old, new));
            }
        }
        None
    }

    fn peek(&self, addr: u64, len: usize) -> u64 {
        read_bytes(&self.cpu.mem, addr, len)
    }

    /// Y a-t-il quelque chose à défaire ?
    pub fn can_step_back(&self) -> bool {
        !self.history.is_empty()
    }

    fn get_reg(&self, reg: Register) -> Result<u64, EmuError> {
        let (r, w) = map_register(reg)?;
        if is_high_byte(reg) {
            return Ok(u64::from(self.cpu.regs.get8h(r)));
        }
        Ok(match w {
            Width::W64 => self.cpu.regs.get64(r),
            Width::W32 => self.cpu.regs.get32(r) as u64,
            Width::W16 => self.cpu.regs.get16(r) as u64,
            Width::W8 => self.cpu.regs.get8l(r) as u64,
        })
    }

    fn set_reg(&mut self, reg: Register, value: u64) -> Result<(), EmuError> {
        let (r, w) = map_register(reg)?;
        if is_high_byte(reg) {
            self.cpu.regs.set8h(r, value as u8);
            return Ok(());
        }
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
            .virtual_address(op_index, 0, |register, _idx, _size| match register {
                Register::None => Some(0),
                Register::RIP | Register::EIP => Some(next_ip),
                // iced réclame aussi le registre de segment : une adresse
                // virtuelle, c'est la base du segment plus le décalage.
                // En 64 bits, CS/DS/ES/SS ont une base nulle — c'est le
                // modèle plat, et c'est ce que Linux donne à un
                // programme. Sans ce cas, *aucun* opérande mémoire ne se
                // calculait : `mov [case], rax` échouait avant même de
                // lire la mémoire.
                Register::CS | Register::DS | Register::ES | Register::SS => Some(0),
                // FS et GS, eux, ont une base que le noyau installe (TLS,
                // canari de pile). namsdec ne la simule pas : mieux vaut
                // refuser l'instruction que calculer une adresse fausse.
                Register::FS | Register::GS => None,
                other => map_register(other)
                    .ok()
                    .map(|(r, _)| self.cpu.regs.get64(r)),
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
        self.record_memory(addr, (w.bits() / 8) as usize);
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

    /// Une instruction que l'interpréteur ne sait pas exécuter laisse RIP
    /// où il est. Sans ça, réessayer avançait d'un cran à chaque fois et
    /// l'exécution finissait par décoder les données comme du code.
    #[test]
    fn a_failed_step_leaves_rip_on_the_offending_instruction() {
        let mut mem = namsdec_core::Memory::new(0x400000, 0x1000);
        // `in al, dx` (EC) : parfaitement décodable, et jamais exécutée
        // ici — il n'y a pas de ports d'entrée-sortie dans une machine
        // qui n'a pas de matériel.
        mem.load(0x400000, &[0xec]).unwrap();
        let mut cpu = namsdec_core::Cpu::new(mem);
        cpu.regs.rip = 0x400000;
        let mut emu = Emulator::new(cpu);

        for attempt in 1..=3 {
            let err = emu.step().expect_err("`in` n'est pas supportée");
            assert!(
                matches!(err, EmuError::UnsupportedInstruction { .. }),
                "obtenu {err:?}"
            );
            assert_eq!(
                emu.cpu.regs.rip, 0x400000,
                "RIP a bougé à la tentative {attempt}"
            );
        }
    }
    use namsdec_core::Reg64;

    const BASE: u64 = 0x400000;

    /// Assemble `asm` avec `namsdec-asm`, le charge en mémoire et exécute
    /// jusqu'à l'arrêt (halt, point d'arrêt ou limite de pas). Test
    /// d'intégration "golden" : il passe par le vrai `nasm` externe.
    fn run_program(asm: &str, breakpoints: &[u64]) -> Emulator {
        let mut emu = load_only(asm, breakpoints);
        emu.run(10_000).expect("exécution");
        emu
    }

    /// Le programme que tout le monde écrit en premier : il parle, puis
    /// il sort. C'est exactement ce que `write` et `exit` servent à faire.
    #[test]
    fn a_program_can_write_to_stdout_and_exit_with_a_code() {
        let emu = run_program(
            "\
section .data
    message db \"salut\", 10

section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, message
    mov rdx, 6
    syscall

    mov rax, 60
    mov rdi, 3
    syscall
",
            &[],
        );

        assert_eq!(emu.output(), b"salut\n");
        assert_eq!(emu.exit_code(), Some(3));
        assert!(emu.cpu.halted, "`exit` arrête la machine");
    }

    /// `write` rend le nombre d'octets écrits, comme le noyau — un
    /// programme peut donc le vérifier, et le `hlt` juste après laisse la
    /// valeur observable dans RAX.
    #[test]
    fn write_returns_the_number_of_bytes_written() {
        let emu = run_program(
            "\
section .data
    message db \"salut\", 10

section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, message
    mov rdx, 6
    syscall
    hlt
",
            &[],
        );

        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
        assert_eq!(emu.output(), b"salut\n");
    }

    /// Un programme qui lit son entrée : tant qu'on ne lui a rien donné,
    /// il attend — sans avancer d'un pouce, et sans bloquer l'interface.
    /// Dès que l'entrée arrive, le même `syscall` repart et la lit.
    #[test]
    fn read_waits_for_input_then_consumes_it() {
        const PROGRAM: &str = "\
section .bss
    tampon resb 32

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 32
    syscall
    hlt
";
        let mut emu = load_only(PROGRAM, &[]);

        // Le programme court jusqu'au `read`, puis s'arrête là.
        assert_eq!(emu.run(100).unwrap(), RunStop::NeedsInput);
        let waiting_at = emu.cpu.regs.rip;
        assert!(!emu.cpu.halted, "attendre n'est pas finir");

        // Insister ne sert à rien et ne casse rien : RIP ne bouge pas.
        assert_eq!(emu.step().unwrap(), StepResult::NeedsInput);
        assert_eq!(emu.cpu.regs.rip, waiting_at);

        emu.provide_input(b"bonjour\n");
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);

        // `read` a rendu le nombre d'octets lus...
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 8);
        // ...et les a bien déposés dans le tampon du programme.
        let buf = emu.cpu.regs.get64(Reg64::Rsi);
        assert_eq!(emu.cpu.mem.bytes_at(buf, 8).unwrap(), b"bonjour\n");
        assert_eq!(emu.pending_input(), 0, "tout a été consommé");
    }

    /// Une entrée close rend une fin de fichier (0) au lieu d'attendre —
    /// l'équivalent d'un Ctrl+D. Sans ça, un programme qui lit jusqu'à la
    /// fin de son entrée attendrait pour toujours.
    #[test]
    fn a_closed_input_reads_as_end_of_file() {
        let mut emu = load_only(
            "\
section .bss
    tampon resb 8

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 8
    syscall
    hlt
",
            &[],
        );

        emu.close_input();
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rax),
            0,
            "attendu une fin de fichier"
        );
    }

    /// `read` rend ce qu'il a sous la main plutôt que d'attendre de quoi
    /// remplir tout le tampon — c'est pourquoi un programme sérieux boucle
    /// sur `read`.
    #[test]
    fn read_returns_what_is_available_without_filling_the_buffer() {
        let mut emu = load_only(
            "\
section .bss
    tampon resb 64

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 64
    syscall
    hlt
",
            &[],
        );

        emu.provide_input(b"court");
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 5);
    }

    /// Lire depuis autre chose que l'entrée standard n'a pas de sens ici :
    /// même réponse que le noyau pour un descripteur jamais ouvert.
    #[test]
    fn reading_from_an_unknown_file_descriptor_returns_ebadf() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 7
    mov rsi, 0
    mov rdx, 8
    syscall
    hlt
",
            &[],
        );

        assert_eq!(emu.cpu.regs.get64(Reg64::Rax) as i64, -9);
    }

    /// `hlt` arrête la machine sans que le programme ait rendu quoi que ce
    /// soit : il n'y a pas de code de sortie à inventer.
    #[test]
    fn halting_is_not_exiting() {
        let emu = run_program("section .text\n    mov rax, 1\n    hlt\n", &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.exit_code(), None);
    }

    /// Il n'y a pas de système de fichiers derrière cet émulateur : tout
    /// descripteur autre que 1 et 2 reçoit `-EBADF`, la réponse qu'aurait
    /// donnée le noyau. Un programme en assembleur est censé savoir lire
    /// ce retour négatif — c'est la convention, pas un détail d'émulation.
    #[test]
    fn writing_to_an_unknown_file_descriptor_returns_ebadf() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 7
    mov rsi, 0
    mov rdx, 1
    syscall
    hlt
",
            &[],
        );

        assert_eq!(emu.cpu.regs.get64(Reg64::Rax) as i64, -9, "attendu -EBADF");
        assert!(emu.output().is_empty(), "rien ne doit avoir été écrit");
    }

    /// Un appel système que namsdec ne simule pas s'arrête en le disant.
    /// Mieux vaut cet aveu qu'un code d'erreur inventé, qui laisserait
    /// croire que l'appel a eu lieu.
    #[test]
    fn an_unimplemented_syscall_says_so() {
        // 39 = `getpid`.
        let mut emu = load_only(
            "section .text\n    global _start\n_start:\n    mov rax, 39\n    syscall\n    hlt\n",
            &[],
        );
        let entry = emu.cpu.regs.rip;

        let err = emu.run(100).expect_err("getpid n'est pas simulé");
        match err {
            EmuError::UnsupportedSyscall { number, .. } => assert_eq!(number, 39),
            other => panic!("attendu UnsupportedSyscall, obtenu {other:?}"),
        }
        assert!(
            emu.cpu.regs.rip > entry,
            "les instructions avant le syscall se sont bien exécutées"
        );
        assert!(!emu.cpu.halted, "un appel refusé n'arrête pas la machine");
    }

    /// Un pointeur qui sort de la fenêtre mémoire simulée est une erreur,
    /// pas une occasion de lire ce qui traîne à côté.
    #[test]
    fn writing_from_an_address_outside_memory_fails() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, 0xdeadbeef
    mov rdx, 4
    syscall
    hlt
",
            &[],
        );

        let err = emu
            .run(100)
            .expect_err("l'adresse est hors de la mémoire simulée");
        assert!(matches!(err, EmuError::Mem(_)), "obtenu {err:?}");
        assert!(emu.output().is_empty());
    }

    /// Un `write` de zéro octet est valide et ne regarde même pas le
    /// pointeur — y compris quand celui-ci ne pointe nulle part.
    #[test]
    fn writing_zero_bytes_is_allowed_and_writes_nothing() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, 0xdeadbeef
    mov rdx, 0
    syscall
    hlt
",
            &[],
        );

        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
        assert!(emu.output().is_empty());
    }

    /// Charge sans exécuter — pour les programmes dont on veut observer
    /// l'échec plutôt que le résultat.
    /// `rep movsb` recopie une chaîne, un octet par pas — et le pas à
    /// pas montre RSI, RDI et RCX avancer, ce qu'une boucle atomique
    /// aurait caché.
    #[test]
    fn rep_movsb_copies_one_byte_per_step() {
        let mut emu = load_only(
            "\
section .data
    source db \"salut\", 0

section .bss
    cible resb 8

section .text
    global _start
_start:
    mov rsi, source
    mov rdi, cible
    mov rcx, 6
    rep movsb
    hlt
",
            &[],
        );
        // Les trois `mov`, puis la copie.
        for _ in 0..3 {
            emu.step().unwrap();
        }
        let (rsi, rdi) = (
            emu.cpu.regs.get64(Reg64::Rsi),
            emu.cpu.regs.get64(Reg64::Rdi),
        );
        let rip = emu.cpu.regs.rip;

        emu.step().unwrap();
        assert_eq!(
            emu.cpu.regs.rip, rip,
            "l'instruction se rejoue tant que RCX reste"
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 5, "un élément par pas");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), rsi + 1);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdi), rdi + 1);
        assert_eq!(
            emu.cpu.mem.read_u8(rdi).unwrap(),
            b's',
            "le premier octet est passé"
        );

        // Jusqu'au bout.
        emu.run(100).unwrap();
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0);
        assert_eq!(
            emu.cpu.mem.bytes_at(rdi, 6).unwrap(),
            b"salut\0",
            "la chaîne entière, terminateur compris"
        );
        assert!(emu.cpu.halted);
    }

    /// `rep` avec RCX à zéro ne fait *rien* — pas même une itération.
    /// C'est la nuance qui distingue un compteur nul d'un compteur à un,
    /// et le piège de tous les premiers `rep`.
    #[test]
    fn rep_with_a_zero_count_does_nothing_at_all() {
        let mut emu = load_only(
            "\
section .bss
    cible resb 4

section .text
    global _start
_start:
    mov rdi, cible
    mov rax, 0x41
    xor rcx, rcx
    rep stosb
    hlt
",
            &[],
        );
        emu.run(100).unwrap();
        let cible = emu.cpu.regs.get64(Reg64::Rdi);
        assert_eq!(emu.cpu.mem.read_u8(cible).unwrap(), 0, "rien n'a été écrit");
        assert!(emu.cpu.halted);
    }

    /// `std` fait travailler les chaînes à reculons, `cld` les remet à
    /// l'endroit — c'est tout ce que le drapeau de direction fait, et on
    /// ne le voit que sur une instruction de chaîne.
    #[test]
    fn the_direction_flag_reverses_a_string_operation() {
        let mut emu = load_only(
            "\
section .bss
    cible resb 8

section .text
    global _start
_start:
    mov rdi, cible
    add rdi, 3          ; on part de la fin
    mov rax, 0x41
    mov rcx, 4
    std
    rep stosb           ; écrit vers les adresses décroissantes
    cld
    hlt
",
            &[],
        );
        emu.run(200).unwrap();
        let base = emu.cpu.regs.get64(Reg64::Rdi).wrapping_add(1);
        assert_eq!(
            emu.cpu.mem.bytes_at(base, 4).unwrap(),
            b"AAAA",
            "les quatre octets sont remplis, de la fin vers le début"
        );
        assert!(
            !emu.cpu.flags.contains(RFlags::DF),
            "`cld` a remis la direction à l'endroit"
        );
    }

    /// `repne scasb` cherche un octet : il s'arrête sur l'égalité, sans
    /// attendre que RCX s'épuise. C'est ainsi qu'on mesure une chaîne
    /// terminée par un zéro.
    #[test]
    fn repne_scasb_stops_on_the_byte_it_was_looking_for() {
        let mut emu = load_only(
            "\
section .data
    texte db \"abc\", 0

section .text
    global _start
_start:
    mov rdi, texte
    xor rax, rax        ; on cherche l'octet nul
    mov rcx, 16
    repne scasb
    hlt
",
            &[],
        );
        emu.run(200).unwrap();
        // RDI s'est arrêté juste après le zéro, et RCX porte ce qui
        // restait : la longueur s'en déduit.
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rcx),
            12,
            "quatre octets parcourus"
        );
        assert!(
            emu.cpu.flags.contains(RFlags::ZF),
            "arrêté sur une égalité, pas sur l'épuisement du compteur"
        );
    }

    /// Le pas arrière défait une itération à la fois, la mémoire copiée
    /// comprise : chaque élément est un pas, donc chaque élément se
    /// défait.
    #[test]
    fn stepping_back_undoes_one_string_element_at_a_time() {
        let mut emu = load_only(
            "\
section .data
    source db \"ab\"

section .bss
    cible resb 4

section .text
    global _start
_start:
    mov rsi, source
    mov rdi, cible
    mov rcx, 2
    rep movsb
    hlt
",
            &[],
        );
        for _ in 0..3 {
            emu.step().unwrap();
        }
        let cible = emu.cpu.regs.get64(Reg64::Rdi);
        emu.step().unwrap();
        emu.step().unwrap();
        assert_eq!(emu.cpu.mem.bytes_at(cible, 2).unwrap(), b"ab");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0);

        emu.step_back();
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 1, "une itération défaite");
        assert_eq!(
            emu.cpu.mem.bytes_at(cible, 2).unwrap(),
            b"a\0",
            "et l'octet qu'elle avait écrit est repris"
        );
    }

    /// Un point de surveillance arrête l'exécution quand la mémoire
    /// change sous lui — sans qu'on ait eu à deviner quelle instruction
    /// le ferait, ce qui est tout l'intérêt.
    #[test]
    fn a_watchpoint_stops_the_run_when_the_value_changes() {
        let mut emu = load_only(
            "\
section .data
    compteur dq 0

section .text
    global _start
_start:
    mov rax, 1
    mov rbx, 2
    mov rcx, 3
    mov [compteur], rcx     ; c'est ici que ça change
    mov rdx, 4
    hlt
",
            &[],
        );
        let compteur = namsdec_asm::assemble(
            "section .data\n    compteur dq 0\nsection .text\n    global _start\n_start:\n    hlt\n",
            BASE,
        )
        .unwrap()
        .sections
        .iter()
        .find(|s| s.name == ".data")
        .expect("une section .data")
        .addr;

        emu.set_watchpoints(&[(compteur, 8)]);
        assert_eq!(emu.watchpoints().len(), 1);
        assert_eq!(
            emu.watchpoints()[0].value(),
            0,
            "armé sur la valeur du moment"
        );

        let stop = emu.run(100).expect("exécution");
        match stop {
            RunStop::Watchpoint { addr, old, new } => {
                assert_eq!(addr, compteur);
                assert_eq!((old, new), (0, 3), "la valeur d'avant et celle d'après");
            }
            other => panic!("attendu un arrêt sur surveillance, obtenu {other:?}"),
        }
        // Arrêté *après* l'instruction fautive, pas à la fin du programme.
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rdx),
            0,
            "`mov rdx, 4` n'a pas eu lieu"
        );
        assert!(!emu.cpu.halted);

        // Relancé, il va jusqu'au bout : la zone ne change plus.
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
    }

    /// Une écriture qui ne change rien n'arrête rien : `mov [x], 0` sur
    /// une case déjà nulle n'apprend rien à personne.
    #[test]
    fn writing_the_same_value_does_not_trigger_a_watchpoint() {
        let mut emu = load_only(
            "\
section .bss
    case resq 1

section .text
    global _start
_start:
    xor rax, rax
    mov [case], rax
    hlt
",
            &[],
        );
        let case = BASE + 0x1000; // .bss, la seule donnée
        emu.set_watchpoints(&[(case, 8)]);
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted, "rien n'a changé");
    }

    /// Le retour en arrière réarme les zones surveillées : il défait, il
    /// ne déclenche pas.
    #[test]
    fn stepping_back_rearms_the_watchpoints_instead_of_firing_them() {
        let mut emu = load_only(
            "\
section .bss
    case resq 1

section .text
    global _start
_start:
    mov rax, 5
    mov [case], rax
    hlt
",
            &[],
        );
        let case = BASE + 0x1000;
        emu.set_watchpoints(&[(case, 8)]);
        assert!(matches!(
            emu.run(100).unwrap(),
            RunStop::Watchpoint { old: 0, new: 5, .. }
        ));

        // On revient avant l'écriture : la zone est réarmée sur zéro, et
        // rien ne s'est déclenché au passage.
        assert!(emu.step_back());
        assert_eq!(emu.watchpoints()[0].value(), 0);

        // Et rejouer le même pas la redéclenche.
        assert!(matches!(
            emu.run(100).unwrap(),
            RunStop::Watchpoint { old: 0, new: 5, .. }
        ));
    }

    /// La chaîne des appels se lit directement : l'émulateur voit passer
    /// les `call` et les `ret`, il n'a pas à deviner les cadres en
    /// suivant RBP — ce que le code d'un débutant ne monte pas.
    #[test]
    fn the_call_chain_follows_calls_and_returns() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    call externe
    hlt

externe:
    call interne
    ret

interne:
    mov rax, 1
    ret
",
            &[],
        );
        assert!(emu.frames().is_empty(), "au départ, personne n'est appelé");

        emu.step().unwrap(); // call externe
        assert_eq!(emu.frames().len(), 1);
        assert_eq!(
            emu.frames()[0].call_site,
            BASE,
            "le `call` est la première instruction"
        );

        emu.step().unwrap(); // call interne
        assert_eq!(emu.frames().len(), 2, "deux appels imbriqués");
        let inner = emu.frames()[1];
        assert_eq!(inner.entry, emu.cpu.regs.rip, "on est entré dans `interne`");
        assert!(inner.call_site > emu.frames()[0].call_site);

        emu.step().unwrap(); // mov rax, 1
        emu.step().unwrap(); // ret
        assert_eq!(emu.frames().len(), 1, "le retour dépile son cadre");
        emu.step().unwrap(); // ret
        assert!(emu.frames().is_empty(), "de retour au programme principal");

        // Un `ret` de trop ne dépile rien qui n'existe pas.
        emu.cpu.regs.rip = inner.return_addr;
        assert!(emu.frames().is_empty());
    }

    /// Et le pas arrière remet la chaîne comme elle était : défaire un
    /// `call` retire son cadre, défaire un `ret` le remet.
    #[test]
    fn stepping_back_restores_the_call_chain() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    call externe
    hlt

externe:
    mov rax, 1
    ret
",
            &[],
        );
        emu.step().unwrap(); // call
        let called = emu.frames().to_vec();
        assert_eq!(called.len(), 1);

        emu.step().unwrap(); // mov
        emu.step().unwrap(); // ret
        assert!(emu.frames().is_empty());

        emu.step_back(); // défait le ret
        assert_eq!(emu.frames(), called.as_slice(), "le cadre est revenu");

        emu.step_back(); // défait le mov
        emu.step_back(); // défait le call
        assert!(emu.frames().is_empty(), "et il repart avec son `call`");
    }

    /// Lire et écrire *en mémoire* par un opérande — le pain quotidien
    /// dès qu'on manipule un tableau.
    ///
    /// Aucune de ces formes ne fonctionnait : iced réclame le registre de
    /// segment pour calculer une adresse virtuelle, et l'interpréteur ne
    /// savait pas quoi en faire. Il refusait donc l'instruction avant
    /// même de toucher à la mémoire — et rien ne le disait, faute d'un
    /// test qui déréférence quoi que ce soit.
    #[test]
    fn memory_operands_are_addressable_in_every_usual_form() {
        let emu = run_program(
            "\
section .data
    table dq 10, 20, 30, 40

section .bss
    case resq 1

section .text
    global _start
_start:
    mov rax, [table]            ; adresse absolue
    mov rsi, table
    mov rbx, [rsi + 8]          ; base + décalage
    mov rcx, 2
    mov rdx, [rsi + rcx*8]      ; base + index mis à l'échelle
    mov r8, 99
    mov [case], r8              ; écriture absolue
    mov r9, [case]              ; relecture
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 10, "[table]");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 20, "[rsi + 8]");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 30, "[rsi + rcx*8]");
        assert_eq!(emu.cpu.regs.get64(Reg64::R9), 99, "ce qu'on vient d'écrire");
    }

    /// Le pas arrière défait ce que le dernier pas a fait : registres,
    /// flags, et la mémoire écrite au passage.
    #[test]
    fn stepping_back_undoes_registers_flags_and_memory() {
        let mut emu = load_only(
            "\
section .bss
    case resq 1

section .text
    global _start
_start:
    mov rax, 7
    mov [case], rax
    cmp rax, 7          ; ZF = 1
    hlt
",
            &[],
        );
        for _ in 0..3 {
            emu.step().unwrap();
        }
        let case = emu.cpu.regs.rip;
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 7);
        assert!(emu.cpu.flags.contains(RFlags::ZF), "le cmp a levé ZF");

        // On défait le `cmp` : ZF retombe, RIP recule.
        assert!(emu.step_back());
        assert!(
            !emu.cpu.flags.contains(RFlags::ZF),
            "ZF est revenu comme avant"
        );
        assert!(emu.cpu.regs.rip < case);

        // Puis l'écriture en mémoire, et enfin le `mov rax, 7` : RAX
        // redevient nul.
        assert!(emu.step_back());
        assert!(emu.step_back());
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0, "RAX est revenu à zéro");

        // Plus rien à défaire au départ du programme.
        assert!(!emu.can_step_back());
        assert!(!emu.step_back());
    }

    /// Ce qu'un `mov [adresse], valeur` a écrit est bien remis en place,
    /// et pas seulement les registres.
    #[test]
    fn stepping_back_restores_the_bytes_that_were_overwritten() {
        let mut emu = load_only(
            "\
section .data
    case dq 0x1122334455667788

section .text
    global _start
_start:
    mov rax, 0
    mov [case], rax
    hlt
",
            &[],
        );
        let addr = {
            // La case est la seule donnée : elle est au début de .data.
            let out = namsdec_asm::assemble(
                "section .data\n    case dq 0x1122334455667788\nsection .text\n    global _start\n_start:\n    hlt\n",
                BASE,
            )
            .unwrap();
            out.sections
                .iter()
                .find(|s| s.name == ".data")
                .expect("une section .data")
                .addr
        };

        emu.step().unwrap(); // mov rax, 0
        emu.step().unwrap(); // mov [case], rax
        assert_eq!(
            emu.cpu.mem.read_u64(addr).unwrap(),
            0,
            "la case est écrasée"
        );

        assert!(emu.step_back());
        assert_eq!(
            emu.cpu.mem.read_u64(addr).unwrap(),
            0x1122334455667788,
            "les octets d'origine sont revenus"
        );
    }

    /// Défaire un `write` reprend ce que le programme avait affiché, et
    /// défaire un `read` lui rend ce qu'il avait consommé — sans quoi le
    /// rejeu se bloquerait sur une entrée vide.
    #[test]
    fn stepping_back_rewinds_the_console_too() {
        let mut emu = load_only(
            "\
section .bss
    tampon resb 8

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 3
    syscall             ; read
    mov rax, 1
    mov rdi, 1
    mov rsi, tampon
    mov rdx, 3
    syscall             ; write
    hlt
",
            &[],
        );
        emu.provide_input(b"oui");
        emu.run(100).unwrap();
        assert_eq!(emu.output(), b"oui");
        assert_eq!(emu.pending_input(), 0);

        // Défaire le `write` : la sortie se vide.
        while emu.output() == b"oui" {
            assert!(emu.step_back());
        }
        assert!(emu.output().is_empty(), "la sortie est reprise");

        // Continuer jusqu'à défaire le `read` : l'entrée revient.
        while emu.pending_input() == 0 && emu.can_step_back() {
            emu.step_back();
        }
        assert_eq!(emu.pending_input(), 3, "les octets lus sont rendus");
    }

    /// Le lot d'instructions qui manquait pour aller au-delà des
    /// premiers exercices : multiplication, division, décalages,
    /// extensions de taille, `setcc`. Chacune est vérifiée sur son
    /// résultat *et* sur les flags qu'un cours fait lire juste après.
    #[test]
    fn multiplication_and_division_land_in_the_right_registers() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 7
    mov rbx, 6
    mul rbx             ; RDX:RAX = 42
    mov rcx, rax

    mov rax, 100
    xor rdx, rdx        ; la moitié haute du dividende doit être nette
    mov rbx, 7
    div rbx             ; RAX = 14, RDX = 2
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 42, "7 * 6");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 14, "100 / 7");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 2, "et son reste");
    }

    /// `imul` à deux opérandes garde le produit dans la destination, et
    /// signale par CF/OF qu'il a dû le tronquer.
    #[test]
    fn imul_reports_a_truncated_product() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov eax, -3
    imul eax, 5         ; -15, tient dans 32 bits
    mov ebx, eax

    mov eax, 0x40000000
    imul eax, 4         ; 2^32 : ne tient pas
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get32(Reg64::Rbx) as i32, -15);
        assert!(
            emu.cpu.flags.contains(RFlags::CF) && emu.cpu.flags.contains(RFlags::OF),
            "le produit tronqué doit lever CF et OF"
        );
    }

    /// Une division par zéro est l'exception du processeur, pas un
    /// résultat inventé — et l'adresse fautive vient avec.
    #[test]
    fn dividing_by_zero_is_reported_as_the_processor_would() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rax, 5
    xor rdx, rdx
    xor rbx, rbx
    div rbx
    hlt
",
            &[],
        );
        let err = emu.run(50).expect_err("division par zéro");
        assert!(
            matches!(err, EmuError::DivideByZero { .. }),
            "obtenu {err:?}"
        );
        // Et le quotient qui déborde est une faute distincte.
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rdx, 1          ; dividende = 2^64, quotient trop grand
    xor rax, rax
    mov rbx, 1
    div rbx
    hlt
",
            &[],
        );
        let err = emu.run(50).expect_err("quotient hors bornes");
        assert!(
            matches!(err, EmuError::DivideOverflow { .. }),
            "obtenu {err:?}"
        );
    }

    /// Décalages : le résultat, et le dernier bit sorti dans CF.
    #[test]
    fn shifts_move_the_bits_and_the_carry() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 5
    shl rax, 3          ; 40
    mov rbx, rax

    mov rcx, 0x81
    shr rcx, 1          ; 0x40, CF = 1 (le bit sorti)
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 40, "5 << 3");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0x40);
        assert!(emu.cpu.flags.contains(RFlags::CF), "le bit 0 sort dans CF");
    }

    /// `sar` recopie le bit de signe : c'est ce qui en fait une division
    /// signée par une puissance de deux, là où `shr` ferait entrer des
    /// zéros et changerait le signe.
    #[test]
    fn sar_keeps_the_sign_where_shr_loses_it() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, -16
    sar rax, 2          ; -4
    mov rbx, rax

    mov rcx, -16
    shr rcx, 2          ; un très grand nombre positif
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx) as i64, -4);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), (-16i64 as u64) >> 2);
    }

    /// `movzx` complète par des zéros, `movsx` par le bit de signe : la
    /// même valeur d'octet donne deux nombres différents, et c'est
    /// exactement la leçon.
    #[test]
    fn movzx_and_movsx_disagree_on_purpose() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov al, 0xf0
    movzx rbx, al       ; 240
    mov al, 0xf0
    movsx rcx, al       ; -16
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 240);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx) as i64, -16);
    }

    /// `setcc` met un booléen dans un octet, sans branchement — et
    /// `neg`/`not`, que le complément à deux réclame dès le premier cours.
    #[test]
    fn setcc_neg_and_not_do_their_arithmetic() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 3
    cmp rax, 3
    sete bl             ; 1
    cmp rax, 4
    setg cl             ; 0

    mov rdx, 5
    neg rdx             ; -5
    mov rsi, 0
    not rsi             ; tous les bits à 1
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rbx), 1, "3 == 3");
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rcx), 0, "3 > 4 est faux");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx) as i64, -5);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), u64::MAX);
    }

    /// `loop`, `xchg` et `leave` : la boucle idiomatique, l'échange, et
    /// le démontage du cadre de pile qu'un prologue a monté.
    #[test]
    fn loop_xchg_and_leave_behave_as_taught() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rcx, 4
    xor rax, rax
somme:
    add rax, rcx
    loop somme          ; 4+3+2+1 = 10

    mov rbx, 1
    mov rdx, 2
    xchg rbx, rdx

    push rbp
    mov rbp, rsp
    sub rsp, 32
    leave               ; RSP et RBP reviennent où ils étaient
    hlt
",
            &[],
        );
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rax),
            10,
            "la boucle a bien tourné"
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 2, "échangés");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 1);
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rsp),
            BASE + 0x8000,
            "`leave` défait le prologue en entier, `push rbp` compris"
        );
    }

    /// `adc` additionne la retenue : c'est ainsi qu'on additionne plus
    /// large qu'un registre.
    #[test]
    fn adc_carries_across_registers() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, -1         ; 0xffff_ffff_ffff_ffff
    mov rbx, 1
    add rax, rbx        ; déborde : CF = 1
    mov rcx, 0
    adc rcx, 0          ; la retenue remonte
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 1, "la retenue est passée");
    }

    /// Un `read` dont le tampon tombe hors de la fenêtre mémoire échoue —
    /// et l'entrée déjà fournie doit rester dans la file. `step` rembobine
    /// RIP pour rejouer le `syscall` à l'identique ; si les octets avaient
    /// été consommés au passage, l'utilisateur devait retaper ce qu'il
    /// venait d'envoyer, et le rejeu se bloquait sur une file vide.
    #[test]
    fn a_read_that_faults_keeps_the_input_it_could_not_write() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, 0xdeadbeef     ; tampon hors de la fenêtre simulée
    mov rdx, 4
    syscall
    hlt
",
            &[],
        );
        emu.provide_input(b"oui\n");

        let err = emu.run(100).expect_err("le tampon est hors bornes");
        assert!(matches!(err, EmuError::Mem(_)), "obtenu {err:?}");
        assert_eq!(
            emu.pending_input(),
            4,
            "l'entrée doit être intacte pour le rejeu"
        );
        // Et le `syscall` est toujours sous RIP, prêt à être rejoué.
        assert_eq!(
            emu.cpu.mem.bytes_at(emu.cpu.regs.rip, 2).unwrap(),
            &[0x0f, 0x05],
            "RIP doit être resté sur le syscall"
        );
    }

    /// Un `pop` dont l'écriture échoue laisse RSP là où il était. Il
    /// avançait avant l'écriture : chaque rejeu — que `step` provoque en
    /// rembobinant RIP — dépilait un mot de plus, et la pile se dévidait
    /// silencieusement, huit octets par tentative.
    #[test]
    fn a_failed_pop_leaves_rsp_where_it_was() {
        let mut mem = namsdec_core::Memory::new(0x400000, 0x1000);
        // `pop qword [rax]` (8F 00), avec RAX hors de la fenêtre.
        mem.load(0x400000, &[0x8f, 0x00]).unwrap();
        let mut cpu = namsdec_core::Cpu::new(mem);
        cpu.regs.rip = 0x400000;
        cpu.regs.set64(Reg64::Rsp, 0x400800);
        cpu.regs.set64(Reg64::Rax, 0xdeadbeef);
        let mut emu = Emulator::new(cpu);

        for attempt in 1..=3 {
            emu.step().expect_err("la destination est hors bornes");
            assert_eq!(
                emu.cpu.regs.get64(Reg64::Rsp),
                0x400800,
                "RSP a bougé à la tentative {attempt}"
            );
            assert_eq!(emu.cpu.regs.rip, 0x400000, "et RIP aussi");
        }
    }

    fn load_only(asm: &str, breakpoints: &[u64]) -> Emulator {
        let out = namsdec_asm::assemble(asm, BASE).expect("assemblage (nasm et ld requis)");
        let mut mem = Memory::new(BASE, 0x10000);
        // L'ELF dit où il veut être chargé et par où commencer : sur un
        // programme à plusieurs sections, ce ne sont pas la même adresse.
        mem.load(out.load_addr, &out.bytes).unwrap();

        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = out.entry;
        cpu.regs.set64(Reg64::Rsp, BASE + 0x8000);

        let mut emu = Emulator::new(cpu);
        emu.set_heap(out.load_addr + out.bytes.len() as u64, BASE + 0x7000);
        emu.breakpoints.extend(breakpoints.iter().copied());
        emu
    }

    /// Comme `run_program`, mais sans exécuter quoi que ce soit — pour les
    /// tests qui doivent piloter `step`/`step_over` à la main. Renvoie
    /// aussi la sortie d'assemblage pour retrouver les adresses par ligne.
    fn load_program(asm: &str) -> (Emulator, namsdec_asm::AssembleOutput) {
        let out = namsdec_asm::assemble(asm, BASE).expect("assemblage (nasm et ld requis)");
        let mut mem = Memory::new(BASE, 0x10000);
        mem.load(out.load_addr, &out.bytes).unwrap();

        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = out.entry;
        cpu.regs.set64(Reg64::Rsp, BASE + 0x8000);

        let mut emu = Emulator::new(cpu);
        // Le tas entre l'image du programme et la pile, comme le fait
        // l'interface quand elle charge un programme.
        emu.set_heap(out.load_addr + out.bytes.len() as u64, BASE + 0x7000);
        (emu, out)
    }

    const SUM_LOOP_ASM: &str = "\
BITS 64
section .text
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
section .text
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
section .text
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
section .text
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

    const CALL_RET_ASM: &str = "\
BITS 64
section .text
    mov rax, 1
    call add_five
    hlt
add_five:
    add rax, 5
    ret
";

    #[test]
    fn call_then_ret_round_trips_and_updates_rax() {
        let emu = run_program(CALL_RET_ASM, &[]);
        assert!(emu.cpu.halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
    }

    #[test]
    fn step_into_a_call_enters_the_subroutine() {
        let (mut emu, out) = load_program(CALL_RET_ASM);
        let call_addr = out.line_to_addr[&4];
        while emu.cpu.regs.rip != call_addr {
            emu.step().unwrap();
        }
        let subroutine_addr = out.line_to_addr[&7]; // "add rax, 5"
        emu.step().unwrap();
        assert_eq!(emu.cpu.regs.rip, subroutine_addr);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 1); // pas encore exécuté
    }

    #[test]
    fn step_over_a_call_runs_the_whole_subroutine() {
        let (mut emu, out) = load_program(CALL_RET_ASM);
        let call_addr = out.line_to_addr[&4];
        while emu.cpu.regs.rip != call_addr {
            emu.step().unwrap();
        }
        let after_call_addr = out.line_to_addr[&5]; // hlt
        let result = emu.step_over(1000).unwrap();
        assert_eq!(result, StepOverResult::SteppedOverCall);
        assert_eq!(emu.cpu.regs.rip, after_call_addr);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
    }

    #[test]
    fn step_over_a_non_call_instruction_behaves_like_step() {
        let (mut emu, _out) = load_program(SUM_LOOP_ASM);
        let result = emu.step_over(1000).unwrap();
        assert_eq!(result, StepOverResult::Stepped);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0); // "mov rax, 0" tout juste exécuté
    }
    /// Un tas neuf : le programme demande où il commence, le fait
    /// grandir, et écrit dans ce qu'il vient d'obtenir. C'est le cœur de
    /// ce qu'un `malloc` fait pour vous.
    #[test]
    fn brk_grows_the_heap_and_the_new_memory_is_usable() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 12         ; brk(0) : ne demande rien, lit la rupture
    xor rdi, rdi
    syscall
    mov r12, rax        ; le début du tas

    mov rdi, rax
    add rdi, 4096
    mov rax, 12         ; brk(fin + 4096) : 4 Kio de plus
    syscall
    mov r13, rax

    mov byte [r12], 42  ; la mémoire obtenue s'écrit
    movzx r14, byte [r12]
    hlt
",
            &[],
        );

        let start = emu.cpu.regs.get64(Reg64::R12);
        assert_eq!(start % PAGE, 0, "la rupture part sur une page");
        assert_eq!(start, emu.heap().start());
        assert_eq!(
            emu.cpu.regs.get64(Reg64::R13),
            start + 4096,
            "brk rend la rupture obtenue, pas un code d'erreur"
        );
        assert_eq!(emu.heap().len(), 4096);
        assert_eq!(emu.cpu.regs.get64(Reg64::R14), 42, "le tas est utilisable");
    }

    /// Le tas ne mord ni sur le programme, ni sur la pile : hors de ces
    /// bornes, `brk` refuse — et refuser, pour lui, c'est rendre la
    /// rupture inchangée.
    #[test]
    fn a_brk_outside_the_heap_is_refused_and_leaves_the_break_alone() {
        let mut emu = Emulator::new(Cpu::new(Memory::new(BASE, 0x10000)));
        emu.set_heap(BASE + 0x1000, BASE + 0x2000);

        assert_eq!(emu.set_break(0), BASE + 0x1000, "brk(0) ne déplace rien");
        assert_eq!(
            emu.set_break(BASE + 0x2001),
            BASE + 0x1000,
            "au-dessus du plafond : refusé"
        );
        assert_eq!(
            emu.set_break(BASE + 0x0800),
            BASE + 0x1000,
            "sous le programme : refusé"
        );
        assert_eq!(
            emu.set_break(BASE + 0x2000),
            BASE + 0x2000,
            "le plafond lui-même s'atteint"
        );
    }

    /// Rendre de la mémoire, c'est la perdre : le noyau reprend les pages
    /// et n'en rendra que des vierges. Un programme qui rétrécit son tas
    /// puis le rouvre ne doit pas y retrouver ses données.
    #[test]
    fn giving_the_heap_back_forgets_what_was_in_it() {
        let mut emu = Emulator::new(Cpu::new(Memory::new(BASE, 0x10000)));
        emu.set_heap(BASE + 0x1000, BASE + 0x3000);
        emu.set_break(BASE + 0x2000);
        emu.cpu.mem.write_u64(BASE + 0x1000, 0x1122_3344).unwrap();

        emu.set_break(BASE + 0x1000); // rendu
        emu.set_break(BASE + 0x2000); // repris
        assert_eq!(emu.cpu.mem.read_u64(BASE + 0x1000).unwrap(), 0);
    }

    /// Sans tas installé, `brk` se comporte comme sous un système à court
    /// de mémoire : il rend la rupture, il ne la déplace pas.
    #[test]
    fn without_a_heap_brk_hands_out_nothing() {
        let mut emu = Emulator::new(Cpu::new(Memory::new(BASE, 0x10000)));
        let top = BASE + 0x10000;
        assert_eq!(emu.set_break(0), top);
        assert_eq!(emu.set_break(top + 0x1000), top);
        assert_eq!(emu.heap().len(), 0);
    }

    /// Revenir en arrière sur un `brk` rend la mémoire au système.
    #[test]
    fn stepping_back_undoes_a_brk() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rax, 12
    xor rdi, rdi
    syscall
    mov rdi, rax
    add rdi, 4096
    mov rax, 12
    syscall
    hlt
",
            &[],
        );

        let start = emu.heap().current();
        for _ in 0..20 {
            if emu.heap().current() != start {
                break;
            }
            emu.step().unwrap();
        }
        assert_eq!(emu.heap().current(), start + 4096, "le tas a grandi");

        assert!(emu.step_back());
        assert_eq!(
            emu.heap().current(),
            start,
            "défaire le pas rend aussi la mémoire"
        );
    }
    /// Diviser un nombre négatif demande d'abord d'étendre son signe dans
    /// RDX : sans `cqo`, `idiv` divise par un reste oublié.
    #[test]
    fn sign_extension_prepares_a_signed_division() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, -17
    cqo                 ; RDX:RAX = -17 sur 128 bits
    mov rbx, 5
    idiv rbx            ; RAX = -3, RDX = -2

    mov r13, rax        ; le quotient, mis à l'abri
    mov r14, rdx        ; et le reste

    mov r8b, -3
    movsx r9, r8b       ; comparaison : la même idée, sur un registre
    mov al, -5
    cbw                 ; AL -> AX
    mov r10w, ax
    mov eax, -6
    cdqe                ; EAX -> RAX
    mov r11, rax
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::R13) as i64, -3, "quotient");
        assert_eq!(emu.cpu.regs.get64(Reg64::R14) as i64, -2, "reste");
        assert_eq!(emu.cpu.regs.get64(Reg64::R9) as i64, -3);
        assert_eq!(emu.cpu.regs.get16(Reg64::R10) as i16, -5, "cbw");
        assert_eq!(emu.cpu.regs.get64(Reg64::R11) as i64, -6, "cdqe");
    }

    /// Les rotations font ressortir par le bas ce qui sort par le haut —
    /// et `rcl`/`rcr` font passer le bit par la retenue.
    #[test]
    fn rotations_bring_back_what_a_shift_would_lose() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov al, 10000001b
    rol al, 1           ; 00000011b, CF = 1
    mov bl, al

    mov cl, 00000011b
    ror cl, 1           ; 10000001b, CF = 1

    stc
    mov dl, 00000001b
    rcl dl, 1           ; la retenue entre par le bas : 00000011b

    mov sil, 11111111b
    rol sil, 8          ; un tour complet : rien ne change
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rbx), 0b0000_0011, "rol");
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rcx), 0b1000_0001, "ror");
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rdx), 0b0000_0011, "rcl");
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rsi) & 0xff,
            0xff,
            "un tour entier"
        );
        assert!(
            emu.cpu.flags.contains(RFlags::CF),
            "le bit sorti reste dans la retenue"
        );
    }

    /// Le bit désigné : `bt` le lit, `bts` le pose, et sur une zone
    /// mémoire le décalage n'est pas borné par l'opérande — c'est ce qui
    /// permet de traiter un tableau de bits.
    #[test]
    fn the_bit_instructions_find_set_and_count_bits() {
        let emu = run_program(
            "\
section .bss
    drapeaux resb 32

section .text
    global _start
_start:
    mov rax, 1000b
    bt rax, 3           ; CF = 1
    setc r8b
    bts rax, 4          ; rax = 11000b
    btr rax, 3          ; rax = 10000b
    btc rax, 0          ; rax = 10001b
    mov r9, rax

    mov rcx, 100        ; le centième bit : treizième octet, bit 4
    bts [drapeaux], rcx
    movzx r10, byte [drapeaux + 12]

    mov rax, 0x100
    bsf rbx, rax        ; 8
    bsr rdx, rax        ; 8
    mov rax, 0xff00
    popcnt rsi, rax     ; 8
    tzcnt rdi, rax      ; 8
    lzcnt r11, rax      ; 48
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1, "bt a mis CF");
        assert_eq!(emu.cpu.regs.get64(Reg64::R9), 0b1_0001);
        assert_eq!(emu.cpu.regs.get64(Reg64::R10), 1 << 4, "le centième bit");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 8, "bsf");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 8, "bsr");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), 8, "popcnt");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdi), 8, "tzcnt");
        assert_eq!(emu.cpu.regs.get64(Reg64::R11), 48, "lzcnt");
    }

    /// `xadd` et `cmpxchg` sont les deux instructions sur lesquelles
    /// reposent les compteurs et les verrous : elles lisent et écrivent
    /// dans le même souffle.
    #[test]
    fn exchange_instructions_swap_and_compare() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 10
    mov rbx, 3
    xadd rax, rbx       ; rax = 13, rbx = 10
    mov r11, rax        ; la somme, avant que cmpxchg ne reprenne RAX

    mov rcx, 5
    mov rax, 5
    mov rdx, 42
    cmpxchg rcx, rdx    ; rax == rcx : rcx prend 42, ZF = 1
    setz r8b

    mov rax, 0          ; cette fois la comparaison échoue
    mov rsi, 7
    cmpxchg rsi, rdx    ; rax reçoit 7, rsi ne bouge pas
    setz r9b

    mov r10d, 0x11223344
    bswap r10d          ; 0x44332211
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::R11), 13, "xadd : la somme");
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rbx),
            10,
            "xadd : l'ancienne valeur"
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 42, "l'échange a eu lieu");
        assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), 7, "l'échange a échoué");
        assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 0);
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rax),
            7,
            "un cmpxchg qui échoue rend la valeur trouvée"
        );
        assert_eq!(emu.cpu.regs.get32(Reg64::R10), 0x4433_2211, "bswap");
    }

    /// Les indicateurs se posent aussi à la main, et se rangent sur la
    /// pile — ce qu'il faut pour écrire une routine qui ne perturbe pas
    /// l'état de celle qui l'appelle.
    #[test]
    fn flags_can_be_set_saved_and_restored() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    stc
    pushfq              ; la retenue part sur la pile
    clc
    setc r8b            ; 0
    popfq
    setc r9b            ; 1 : elle est revenue

    cmc                 ; et elle se retourne
    setc r10b           ; 0

    stc
    lahf                ; les indicateurs dans AH
    mov dl, ah          ; (AH ne se lit pas dans une instruction à REX)
    movzx r11, dl
    clc
    sahf                ; et retour
    setc r12b           ; 1
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 0);
        assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 1, "popfq a rendu la retenue");
        assert_eq!(emu.cpu.regs.get8l(Reg64::R10), 0, "cmc l'a retournée");
        assert_eq!(
            emu.cpu.regs.get64(Reg64::R11) & 0x03,
            0x03,
            "lahf : CF posée, et le bit 1 toujours à 1"
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::R12), 1, "sahf l'a rendue");
    }

    /// Boucles conditionnelles, saut sur RCX nul, cadre de pile par
    /// `enter`, et la table de `xlatb`.
    #[test]
    fn loops_frames_and_tables() {
        let emu = run_program(
            "\
section .data
    table db 'a', 'b', 'c', 'd'

section .text
    global _start
_start:
    mov rcx, 10
    xor rax, rax
compte:
    inc rax
    cmp rax, 4
    loopne compte       ; s'arrête sur l'égalité, RCX non nul
    mov r13, rax        ; mis à l'abri : xlatb reprendra RAX
    mov r14, rcx        ; de même pour RCX, remis à zéro plus bas

    xor rcx, rcx
    jrcxz vide          ; RCX est nul : on saute
    mov rbx, 1          ; sautée
vide:
    mov rbx, 2

    mov rbp, rsp
    enter 32, 0         ; cadre de 32 octets
    mov r8, rsp
    mov r9, rbp
    leave

    mov rbx, table
    mov al, 2
    xlatb               ; AL = table[2] = 'c'
    mov r10b, al
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::R13), 4, "loopne s'arrête sur ZF");
        assert_eq!(
            emu.cpu.regs.get64(Reg64::R14),
            6,
            "et RCX a compté les tours"
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::R10), b'c', "xlatb a lu la table");
        assert_eq!(
            emu.cpu.regs.get64(Reg64::R9) - emu.cpu.regs.get64(Reg64::R8),
            32,
            "enter a réservé 32 octets sous RBP"
        );
    }

    /// `shld`/`shrd` décalent une valeur plus large qu'un registre : les
    /// bits qui entrent viennent de l'autre moitié, et non de zéros.
    #[test]
    fn double_shifts_carry_bits_across_registers() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 0xff00000000000000
    mov rbx, 0x8000000000000000
    shld rax, rbx, 4    ; rax = 0xf000...008
    mov r8, rax

    mov rcx, 0x0f
    mov rdx, 0xf0
    shrd rcx, rdx, 4    ; rcx = 0x0000...00 | (0xf0 << 60)
    mov r9, rcx
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::R8), 0xf000_0000_0000_0008, "shld");
        assert_eq!(emu.cpu.regs.get64(Reg64::R9), 0x0000_0000_0000_0000, "shrd");
    }

    /// `rdtsc` compte les instructions exécutées : un temps qui a un sens
    /// ici, et qui revient en arrière avec le pas arrière.
    #[test]
    fn rdtsc_counts_the_instructions_and_steps_back() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    rdtsc
    mov r8, rax
    rdtsc
    mov r9, rax
    hlt
",
            &[],
        );
        emu.run(100).expect("exécution");
        let first = emu.cpu.regs.get64(Reg64::R8);
        let second = emu.cpu.regs.get64(Reg64::R9);
        assert_eq!(first, 1, "le premier rdtsc est la première instruction");
        assert_eq!(second, 3, "deux instructions plus loin");
        while emu.can_step_back() {
            emu.step_back();
        }
        assert_eq!(emu.ticks, 0, "revenir au départ remet le compteur à zéro");
    }
    /// AH, BH, CH, DH : l'octet haut des quatre registres historiques.
    /// Un vestige du 8086, mais tous les cours d'assembleur passent par
    /// lui — et il ne se confond pas avec l'octet bas.
    #[test]
    fn the_high_byte_registers_are_not_the_low_ones() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 0x1122
    mov ah, 0xff        ; seul l'octet haut change
    mov bl, al          ; l'octet bas est intact
    mov cl, ah
    mov dx, ax
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0xff22, "ah seul a changé");
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rbx), 0x22, "al n'a pas bougé");
        assert_eq!(emu.cpu.regs.get8l(Reg64::Rcx), 0xff, "ah se lit");
        assert_eq!(emu.cpu.regs.get16(Reg64::Rdx), 0xff22);
    }

    /// Ce qui ne simule rien mais doit tout de même passer : les barrières
    /// mémoire, `pause`, et le `endbr64` que tout compilateur pose en tête
    /// de fonction depuis que les processeurs vérifient les sauts.
    #[test]
    fn the_instructions_that_do_nothing_still_run() {
        let emu = run_program(
            "\
section .data
    mot dd 0x11223344

section .text
    global _start
_start:
    endbr64
    pause
    mfence
    lfence
    sfence
    movbe eax, [mot]    ; 0x44332211 : les octets à l'envers
    mov r8d, eax
    xchg r8, r9
    hlt
",
            &[],
        );
        assert_eq!(
            emu.cpu.regs.get64(Reg64::R9),
            0x4433_2211,
            "movbe, puis xchg"
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::R8), 0);
    }

    /// `int3` arrête le programme là où il est — c'est le point d'arrêt
    /// qu'un programme se pose à lui-même — et `ud2` échoue exprès.
    #[test]
    fn int3_stops_and_ud2_faults() {
        let mut emu = load_only(
            "\
section .text
    global _start
_start:
    mov rax, 1
    int3
    mov rax, 2
    ud2
",
            &[],
        );
        assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 1, "arrêté sur l'int3");

        // On repart : l'arrêt n'est pas la fin du programme.
        emu.cpu.halted = false;
        let err = emu.run(100).expect_err("ud2 doit échouer");
        assert!(
            matches!(err, EmuError::InvalidInstruction { .. }),
            "obtenu {err:?}"
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 2);
    }
    /// Les instructions de bits récentes : elles font en une fois ce qui
    /// demandait un masque et deux décalages, sans toucher aux
    /// indicateurs qu'un calcul en cours pourrait attendre.
    #[test]
    fn the_modern_bit_instructions_shift_extract_and_multiply() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    mov rax, 0xff
    mov rcx, 4
    shlx rbx, rax, rcx      ; 0xff0
    shrx rdx, rax, rcx      ; 0x0f
    mov rsi, -16
    sarx rdi, rsi, rcx      ; -1 : le signe est recopié
    rorx r8, rax, 4         ; 0xf00000000000000f

    mov r9, 0b1011000
    blsi r10, r9            ; le bit bas posé : 0b1000
    blsr r11, r9            ; le même, effacé : 0b1010000
    andn r12, r9, rax       ; ~r9 & 0xff = 0b10100111

    mov rax, 0x1234
    mov rcx, 0x0804         ; huit bits (0x08) à partir du quatrième (0x04)
    bextr r13, rax, rcx     ; 0x23
    mov rcx, 8
    bzhi r14, rax, rcx      ; on ne garde que les huit bas : 0x34

    mov rdx, 0x1_0000_0000
    mov r15, 4
    mulx rbx, rsi, r15      ; rdx * r15 = 0x4_0000_0000
    hlt
",
            &[],
        );
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rbx) as i64,
            0,
            "mulx : la moitié haute"
        );
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rsi),
            0x4_0000_0000,
            "mulx : la basse"
        );
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rdi) as i64,
            -1,
            "sarx recopie le signe"
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::R8), 0xf000_0000_0000_000f, "rorx");
        assert_eq!(emu.cpu.regs.get64(Reg64::R10), 0b1000, "blsi");
        assert_eq!(emu.cpu.regs.get64(Reg64::R11), 0b101_0000, "blsr");
        assert_eq!(emu.cpu.regs.get64(Reg64::R12), 0b1010_0111, "andn");
        assert_eq!(emu.cpu.regs.get64(Reg64::R13), 0x23, "bextr");
        assert_eq!(emu.cpu.regs.get64(Reg64::R14), 0x34, "bzhi");
    }

    /// `cpuid` ne peut annoncer aucune extension sans mentir : il se nomme
    /// et s'arrête là.
    #[test]
    fn cpuid_says_who_it_is_and_promises_nothing() {
        let emu = run_program(
            "\
section .text
    global _start
_start:
    xor eax, eax
    cpuid
    mov r8, rbx
    mov r9, rcx
    mov r10, rdx
    mov r11, rax

    mov eax, 1
    cpuid                   ; aucune extension annoncée
    mov r12, rdx
    hlt
",
            &[],
        );
        let mut vendor = Vec::new();
        for part in [Reg64::R8, Reg64::R10, Reg64::R9] {
            vendor.extend_from_slice(&(emu.cpu.regs.get32(part)).to_le_bytes());
        }
        assert_eq!(&vendor, b"namsdec emu\0", "obtenu {vendor:?}");
        assert_eq!(emu.cpu.regs.get64(Reg64::R11), 0, "aucune feuille au-delà");
        assert_eq!(emu.cpu.regs.get64(Reg64::R12), 0, "aucun bit de capacité");
    }
    /// Le calcul flottant du x86-64 : il se fait dans les registres XMM,
    /// un nombre à la fois, et les résultats se comparent avec des tests
    /// *non signés* parce que `comisd` écrit dans CF et ZF.
    #[test]
    fn scalar_floating_point_computes_and_compares() {
        let emu = run_program(
            "\
section .data
    deux    dq 2.0
    trois   dq 3.0
    demi    dd 0.5

section .text
    global _start
_start:
    movsd xmm0, [deux]
    movsd xmm1, [trois]
    addsd xmm0, xmm1        ; 5.0
    movq rax, xmm0

    movsd xmm2, [trois]
    mulsd xmm2, xmm2        ; 9.0
    sqrtsd xmm2, xmm2       ; 3.0
    movq rbx, xmm2

    movss xmm3, [demi]
    addss xmm3, xmm3        ; 1.0 en simple précision
    movd ecx, xmm3

    movsd xmm4, [deux]
    comisd xmm4, [trois]    ; 2 < 3 : CF = 1
    setb r8b
    comisd xmm4, [deux]     ; égal : ZF = 1
    sete r9b

    mov r11, 5
    cvtsi2sd xmm5, r11      ; 5 -> 5.0
    cvttsd2si rdx, xmm5     ; et retour
    pxor xmm6, xmm6         ; la façon habituelle de remettre à zéro
    movq rsi, xmm6
    hlt
",
            &[],
        );
        assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rax)), 5.0, "addsd");
        assert_eq!(
            f64::from_bits(emu.cpu.regs.get64(Reg64::Rbx)),
            3.0,
            "sqrtsd"
        );
        assert_eq!(
            f32::from_bits(emu.cpu.regs.get32(Reg64::Rcx)),
            1.0,
            "addss en simple précision"
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1, "comisd : plus petit");
        assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 1, "comisd : égal");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 5, "cvttsd2si");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), 0, "pxor remet à zéro");
    }

    /// Un NaN n'est ni plus petit, ni égal, ni plus grand : `comisd` le
    /// dit en posant les trois indicateurs d'un coup — « non ordonné ».
    #[test]
    fn a_nan_comparison_is_unordered() {
        let emu = run_program(
            "\
section .data
    pasnombre dq 0x7ff8000000000000
    un        dq 1.0

section .text
    global _start
_start:
    movsd xmm0, [pasnombre]
    movsd xmm1, [un]
    comisd xmm0, xmm1
    setp r8b                ; PF : non ordonné
    setz r9b
    setc r10b
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1, "PF signale le NaN");
        assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 1);
        assert_eq!(emu.cpu.regs.get8l(Reg64::R10), 1);
    }

    /// `addps` fait quatre additions d'un seul ordre — c'est toute la
    /// raison d'être des registres larges.
    #[test]
    fn packed_arithmetic_computes_four_at_once() {
        let emu = run_program(
            "\
section .data
    align 16
    quatre dd 1.0, 2.0, 3.0, 4.0
    unites dd 1.0, 1.0, 1.0, 1.0
    sortie times 16 db 0

section .text
    global _start
_start:
    movaps xmm0, [quatre]
    movaps xmm1, [unites]
    addps xmm0, xmm1        ; 2, 3, 4, 5
    movaps [sortie], xmm0

    movss xmm2, [sortie]
    movd eax, xmm2
    movss xmm3, [sortie + 12]
    movd ebx, xmm3
    hlt
",
            &[],
        );
        assert_eq!(f32::from_bits(emu.cpu.regs.get32(Reg64::Rax)), 2.0);
        assert_eq!(f32::from_bits(emu.cpu.regs.get32(Reg64::Rbx)), 5.0);
    }

    /// Le haut d'un registre XMM ne bouge pas quand on n'écrit que son
    /// bas — sauf en venant de la mémoire, où `movsd` met le reste à
    /// zéro. Une asymétrie du jeu d'instructions, et une source d'erreurs
    /// classique.
    #[test]
    fn a_scalar_move_only_touches_what_it_must() {
        let emu = run_program(
            "\
section .data
    align 16
    motifs dq 0x1111111111111111, 0x2222222222222222
    un     dq 1.0
    sortie times 16 db 0

section .text
    global _start
_start:
    movaps xmm0, [motifs]
    movsd xmm0, [un]        ; depuis la mémoire : le haut passe à zéro
    movaps [sortie], xmm0
    mov rax, [sortie]
    mov rbx, [sortie + 8]

    movaps xmm1, [motifs]
    movsd xmm2, [un]
    movsd xmm1, xmm2        ; entre registres : le haut est préservé
    movaps [sortie], xmm1
    mov rcx, [sortie]
    mov rdx, [sortie + 8]
    hlt
",
            &[],
        );
        assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rax)), 1.0);
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rbx),
            0,
            "la mémoire efface le haut"
        );
        assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rcx)), 1.0);
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rdx),
            0x2222_2222_2222_2222,
            "un registre le laisse en place"
        );
    }
    /// SSE entier : seize additions d'un ordre, une comparaison qui rend
    /// un masque, et `pmovmskb` qui ramène le résultat dans un registre
    /// général — la chaîne complète d'un `memchr` vectoriel.
    #[test]
    fn packed_integers_add_compare_and_report() {
        let emu = run_program(
            "\
section .data
    align 16
    a       times 16 db 1
    b       times 16 db 2
    cible   db 'n','a','m','s','d','e','c',0,0,0,0,0,0,0,0,0
    lettres times 16 db 'm'
    sortie  times 16 db 0

section .text
    global _start
_start:
    movdqa xmm0, [a]
    movdqa xmm1, [b]
    paddb xmm0, xmm1        ; seize fois 3
    movdqa [sortie], xmm0
    movzx rax, byte [sortie]

    movdqa xmm2, [cible]
    movdqa xmm3, [lettres]
    pcmpeqb xmm2, xmm3      ; le 'm' de « nams » : troisième octet
    pmovmskb ebx, xmm2

    movdqa xmm4, [a]
    psllq xmm4, 4           ; chaque moitié décalée de quatre bits
    movq rcx, xmm4

    movdqa xmm5, [a]
    punpcklbw xmm5, xmm5    ; des octets entrelacés avec eux-mêmes
    movq rdx, xmm5
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 3, "paddb");
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rbx),
            0b100,
            "pcmpeqb puis pmovmskb"
        );
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rcx),
            0x0101_0101_0101_0101u64 << 4,
            "psllq décale les deux moitiés"
        );
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rdx),
            0x0101_0101_0101_0101,
            "punpcklbw"
        );
    }

    /// `pshufd` remet les quatre éléments dans l'ordre qu'on veut, et
    /// `movhlps` ramène le haut d'un registre en bas — deux façons
    /// d'atteindre ce que les instructions scalaires ne voient pas.
    #[test]
    fn shuffles_reach_the_upper_half() {
        let emu = run_program(
            "\
section .data
    align 16
    quatre dd 10, 20, 30, 40
    sortie times 16 db 0

section .text
    global _start
_start:
    movdqa xmm0, [quatre]
    pshufd xmm1, xmm0, 00_01_10_11b   ; ordre inversé : 40, 30, 20, 10
    movdqa [sortie], xmm1
    mov eax, [sortie]
    mov ebx, [sortie + 12]

    movhlps xmm2, xmm0      ; le haut de xmm0 (30, 40) passe en bas
    movq rcx, xmm2
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get32(Reg64::Rax), 40, "pshufd a inversé");
        assert_eq!(emu.cpu.regs.get32(Reg64::Rbx), 10);
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rcx),
            (40u64 << 32) | 30,
            "movhlps"
        );
    }
    /// Le x87 : une pile, et non des registres nommés. `fld` empile, les
    /// opérations consomment le sommet, et le résultat redescend en
    /// mémoire. C'est la machine que décrivent les cours d'assembleur
    /// écrits avant SSE.
    #[test]
    fn the_x87_stack_computes_and_stores() {
        let emu = run_program(
            "\
section .data
    deux    dq 2.0
    trois   dq 3.0
    sept    dd 7
    sortie  dq 0.0
    entier  dq 0
    etendu  dt 0.0

section .text
    global _start
_start:
    fld qword [deux]        ; st0 = 2
    fld qword [trois]       ; st0 = 3, st1 = 2
    faddp                   ; st1 += st0, puis dépile : st0 = 5
    fstp qword [sortie]     ; la pile est vide
    mov rax, [sortie]

    fild dword [sept]       ; 7, converti depuis un entier
    fld qword [deux]
    fdivp                   ; st1 / st0 = 3.5
    fistp qword [entier]    ; arrondi au plus proche pair : 4
    mov rbx, [entier]

    fld qword [trois]
    fstp tword [etendu]     ; aller-retour par le format 80 bits
    fld tword [etendu]
    fstp qword [sortie]
    mov rcx, [sortie]
    hlt
",
            &[],
        );
        assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rax)), 5.0, "faddp");
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rbx),
            4,
            "fistp arrondit 3,5 vers 4"
        );
        assert_eq!(
            f64::from_bits(emu.cpu.regs.get64(Reg64::Rcx)),
            3.0,
            "le format étendu se relit"
        );
        assert_eq!(emu.cpu.regs.x87.depth(), 0, "tout a été dépilé");
    }

    /// Le x87 ne touche pas aux indicateurs du processeur : il répond
    /// dans son mot d'état, qu'il faut aller chercher. D'où le
    /// `fstsw ax` / `sahf` du code d'époque — et les `fcomi`, venus plus
    /// tard, qui s'en passent.
    #[test]
    fn an_x87_comparison_answers_in_its_own_status_word() {
        let emu = run_program(
            "\
section .data
    deux  dq 2.0
    trois dq 3.0

section .text
    global _start
_start:
    fld qword [deux]
    fld qword [trois]       ; st0 = 3, st1 = 2
    fcom st1                ; 3 > 2 : ni C0 ni C3
    fstsw ax
    sahf                    ; C0 -> CF, C3 -> ZF
    seta r8b                ; strictement plus grand

    fcomip st1              ; la même comparaison, dans les indicateurs
    seta r9b

    fldz
    ftst                    ; zéro contre zéro : C3
    fstsw ax
    sahf
    sete r10b
    hlt
",
            &[],
        );
        assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1, "fstsw puis sahf");
        assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 1, "fcomip directement");
        assert_eq!(emu.cpu.regs.get8l(Reg64::R10), 1, "ftst voit le zéro");
    }

    /// Les constantes et les fonctions du coprocesseur : c'est avec
    /// elles qu'on écrit un calcul trigonométrique sans table.
    #[test]
    fn the_coprocessor_knows_its_constants() {
        let emu = run_program(
            "\
section .data
    sortie dq 0.0

section .text
    global _start
_start:
    fldpi
    fstp qword [sortie]
    mov rax, [sortie]

    fldpi
    fld1
    fld1
    faddp                   ; 2
    fdivp                   ; pi / 2
    fsin                    ; sin(pi/2) = 1
    fstp qword [sortie]
    mov rbx, [sortie]

    fld1
    fchs                    ; -1
    fabs                    ; 1
    fstp qword [sortie]
    mov rcx, [sortie]
    hlt
",
            &[],
        );
        assert_eq!(
            f64::from_bits(emu.cpu.regs.get64(Reg64::Rax)),
            std::f64::consts::PI
        );
        assert!(
            (f64::from_bits(emu.cpu.regs.get64(Reg64::Rbx)) - 1.0).abs() < 1e-12,
            "sin(pi/2)"
        );
        assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rcx)), 1.0);
    }
    /// AVX : les mêmes registres, deux fois plus larges. Huit additions
    /// flottantes d'un seul ordre.
    #[test]
    fn avx_computes_eight_at_a_time() {
        let emu = run_program(
            "\
section .data
    align 32
    huit   dd 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0
    uns    dd 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0
    sortie times 32 db 0

section .text
    global _start
_start:
    vmovaps ymm0, [huit]
    vmovaps ymm1, [uns]
    vaddps ymm2, ymm0, ymm1     ; 2, 3, 4, 5, 6, 7, 8, 9
    vmovaps [sortie], ymm2
    mov eax, [sortie]           ; le premier
    mov ebx, [sortie + 28]      ; le dernier

    ; AVX ne détruit pas ses sources : la destination est à part.
    vmovaps [sortie], ymm0
    mov ecx, [sortie]
    hlt
",
            &[],
        );
        assert_eq!(f32::from_bits(emu.cpu.regs.get32(Reg64::Rax)), 2.0);
        assert_eq!(f32::from_bits(emu.cpu.regs.get32(Reg64::Rbx)), 9.0);
        assert_eq!(
            f32::from_bits(emu.cpu.regs.get32(Reg64::Rcx)),
            1.0,
            "la source est intacte"
        );
    }

    /// La règle qui surprend tout le monde : écrire dans `xmm3` avec une
    /// instruction AVX remet à zéro la moitié haute de `ymm3`, alors que
    /// la même écriture en SSE la laisse intacte. Ce sont pourtant les
    /// mêmes bits.
    #[test]
    fn a_vex_write_clears_the_upper_half_where_sse_leaves_it() {
        let emu = run_program(
            "\
section .data
    align 32
    huit   dd 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0
    uns    dd 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0
    sortie times 32 db 0

section .text
    global _start
_start:
    vmovaps ymm3, [huit]
    movaps xmm3, [uns]          ; SSE : le haut de ymm3 survit
    vextractf128 xmm4, ymm3, 1
    vmovaps [sortie], xmm4
    mov eax, [sortie]           ; 5.0

    vmovaps ymm3, [huit]
    vmovaps xmm3, [uns]         ; AVX 128 bits : le haut passe à zéro
    vextractf128 xmm5, ymm3, 1
    vmovaps [sortie], xmm5
    mov ebx, [sortie]           ; 0

    vmovaps ymm6, [huit]
    vzeroupper                  ; et voilà comment on nettoie avant du SSE
    vextractf128 xmm7, ymm6, 1
    vmovaps [sortie], xmm7
    mov ecx, [sortie]
    hlt
",
            &[],
        );
        assert_eq!(
            f32::from_bits(emu.cpu.regs.get32(Reg64::Rax)),
            5.0,
            "une écriture SSE laisse le haut"
        );
        assert_eq!(
            emu.cpu.regs.get32(Reg64::Rbx),
            0,
            "une écriture VEX l'efface"
        );
        assert_eq!(emu.cpu.regs.get32(Reg64::Rcx), 0, "vzeroupper aussi");
    }

    /// Les instructions propres à AVX : recopier un élément partout, et
    /// déplacer les moitiés de 128 bits, qui ne se parlent pas d'
    /// elles-mêmes.
    #[test]
    fn avx_broadcasts_and_moves_its_halves() {
        let emu = run_program(
            "\
section .data
    align 32
    sept   dd 7.0
    huit   dd 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0
    quatre dq 10, 20, 30, 40
    sortie times 32 db 0

section .text
    global _start
_start:
    vbroadcastss ymm0, [sept]   ; huit fois 7
    vmovaps [sortie], ymm0
    mov eax, [sortie + 16]

    ; Les deux moitiés échangées : 5,6,7,8 en bas.
    vmovaps ymm1, [huit]
    vperm2f128 ymm2, ymm1, ymm1, 0x01
    vmovaps [sortie], ymm2
    mov ebx, [sortie]

    ; Et les quatre quadruples mots remis dans l'ordre inverse.
    vmovdqa ymm3, [quatre]
    vpermq ymm4, ymm3, 00_01_10_11b
    vmovdqa [sortie], ymm4
    mov rcx, [sortie]

    vpaddq ymm5, ymm3, ymm3     ; l'entier marche aussi sur 256 bits
    vmovdqa [sortie], ymm5
    mov rdx, [sortie + 24]
    hlt
",
            &[],
        );
        assert_eq!(
            f32::from_bits(emu.cpu.regs.get32(Reg64::Rax)),
            7.0,
            "diffusé jusque dans la moitié haute"
        );
        assert_eq!(
            f32::from_bits(emu.cpu.regs.get32(Reg64::Rbx)),
            5.0,
            "vperm2f128 a échangé les moitiés"
        );
        assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 40, "vpermq a inversé");
        assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 80, "vpaddq sur 256 bits");
    }
}
