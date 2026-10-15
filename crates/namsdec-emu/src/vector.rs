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

use crate::regmap::{map_mask, map_vector};
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

/// La largeur d'un ÉLÉMENT, pour les instructions qui acceptent un
/// masque.
///
/// Un masque d'AVX-512 s'applique élément par élément : `vpaddd
/// zmm0{k1}, …` n'additionne que les doubles mots dont le bit est à
/// un. Savoir ce qu'est un élément est donc la condition pour
/// appliquer le masque, et c'est le NOM de l'instruction qui le dit —
/// le `d` de `vpaddd`, le `64` de `vmovdqu64`.
///
/// `None` pour tout ce dont on ne sait pas le dire. Le masque y est
/// alors refusé plutôt que deviné : se tromper d'élément décalerait
/// les voies retenues, et rendrait un résultat faux sans rien
/// signaler. C'est la même règle que partout ici — dire, ou s'arrêter.
fn largeur_élément(m: Mnemonic) -> Option<usize> {
    use Mnemonic as M;
    Some(match m {
        // Les déplacements, dont le suffixe EVEX ne dit QUE cela.
        M::Vmovdqu8 => 1,
        M::Vmovdqu16 => 2,
        M::Vmovdqu32 | M::Vmovdqa32 => 4,
        M::Vmovdqu64 | M::Vmovdqa64 => 8,
        // Le calcul entier, par la lettre finale.
        M::Vpaddb
        | M::Vpsubb
        | M::Vpminub
        | M::Vpmaxub
        | M::Vpminsb
        | M::Vpmaxsb
        | M::Vpavgb
        | M::Vpabsb
        | M::Vpshufb
        | M::Vpcmpeqb
        | M::Vpcmpgtb => 1,
        M::Vpaddw
        | M::Vpsubw
        | M::Vpminuw
        | M::Vpmaxuw
        | M::Vpminsw
        | M::Vpmaxsw
        | M::Vpavgw
        | M::Vpabsw
        | M::Vpmullw
        | M::Vpcmpeqw
        | M::Vpcmpgtw => 2,
        M::Vpaddd
        | M::Vpsubd
        | M::Vpminud
        | M::Vpmaxud
        | M::Vpminsd
        | M::Vpmaxsd
        | M::Vpabsd
        | M::Vpmulld
        | M::Vpandd
        | M::Vpord
        | M::Vpxord
        | M::Vpandnd
        | M::Vpcmpeqd
        | M::Vpcmpgtd => 4,
        M::Vpaddq
        | M::Vpsubq
        | M::Vpandq
        | M::Vporq
        | M::Vpxorq
        | M::Vpandnq
        | M::Vpcmpeqq
        | M::Vpcmpgtq => 8,
        // Les flottants paquetés : simple ou double précision.
        M::Vaddps
        | M::Vsubps
        | M::Vmulps
        | M::Vdivps
        | M::Vminps
        | M::Vmaxps
        | M::Vsqrtps
        | M::Vandps
        | M::Vorps
        | M::Vxorps
        | M::Vandnps => 4,
        M::Vaddpd
        | M::Vsubpd
        | M::Vmulpd
        | M::Vdivpd
        | M::Vminpd
        | M::Vmaxpd
        | M::Vsqrtpd
        | M::Vandpd
        | M::Vorpd
        | M::Vxorpd
        | M::Vandnpd => 8,
        _ => return None,
    })
}

/// Compose le résultat d'une opération masquée.
///
/// Les deux formes du masquage d'AVX-512, et elles ne font pas la même
/// chose des voies écartées :
///   - FUSION (`{k1}`) : elles gardent ce qu'elles avaient. C'est le
///     défaut, et c'est ce qui permet d'écrire « ajoute 1 aux éléments
///     positifs » sans toucher aux autres.
///   - MISE À ZÉRO (`{k1}{z}`) : elles tombent à zéro.
///
/// `ancien` est ce que la destination portait AVANT — d'où la lecture
/// du registre dans `write_vec_op`, qui n'aurait aucun sens sans
/// masque.
fn appliquer_masque(
    calculé: Vector,
    ancien: Vector,
    masque: u64,
    bytes: usize,
    à_zéro: bool,
) -> Vector {
    let bits = bytes * 8;
    let par_tranche = 128 / bits.max(1);
    let mut out = ancien;
    for tranche in 0..4 {
        for voie in 0..par_tranche {
            let rang = tranche * par_tranche + voie;
            let décalage = voie * bits;
            let champ = if bits >= 128 {
                u128::MAX
            } else {
                ((1u128 << bits) - 1) << décalage
            };
            let retenue = rang < 64 && (masque >> rang) & 1 == 1;
            let valeur = if retenue {
                calculé[tranche] & champ
            } else if à_zéro {
                0
            } else {
                ancien[tranche] & champ
            };
            out[tranche] = (out[tranche] & !champ) | valeur;
        }
    }
    out
}

/// Un registre vectoriel lu ou écrit : QUATRE tranches de 128 bits.
///
/// Quatre et non deux : `zmm` en fait 512. Les instructions SSE n'en
/// emploient qu'une, celles d'AVX deux, et les tranches qu'elles ne
/// touchent pas valent zéro — ce qui n'est pas la même chose que « ce
/// qui y était », et c'est `write_vec_op` qui tranche.
type Vector = [u128; 4];

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

/// `pmin*`/`pmax*` : voie par voie, le plus petit — ou le plus grand —
/// des deux. Le jeu en donne une forme par largeur **et** par signe,
/// parce qu'un octet à `0x80` vaut 128 pour l'un et −128 pour l'autre :
/// c'est tout ce qui sépare `pmaxub` de `pmaxsb`.
///
/// `pminub` est la pièce maîtresse du `strlen` de la glibc — seize
/// octets comparés d'un coup au vecteur nul —, et c'est à ce titre qu'on
/// la croise dans à peu près tout binaire compilé.
fn minmax_lanes(a: u128, b: u128, bytes: usize, signed: bool, max: bool) -> u128 {
    let Some(w) = width_from_bytes(bytes) else {
        return 0;
    };
    int_lanes(a, b, bytes, move |x, y| {
        let garde_x = if signed {
            let (sx, sy) = (sign_extend(x, w) as i64, sign_extend(y, w) as i64);
            if max { sx > sy } else { sx < sy }
        } else if max {
            x > y
        } else {
            x < y
        };
        if garde_x { x } else { y }
    })
}

/// L'arithmétique **saturée** : au lieu de repartir de l'autre bout
/// quand elle déborde, la voie s'arrête à sa borne.
///
/// C'est ce qu'il faut quand les éléments sont des échantillons ou des
/// pixels : deux clairs additionnés doivent donner du blanc, pas du
/// noir. L'arithmétique ordinaire — `paddb` — fait l'inverse, et c'est
/// pour ça que les deux existent.
fn saturating_lanes(a: u128, b: u128, bytes: usize, signed: bool, sub: bool) -> u128 {
    let Some(w) = width_from_bytes(bytes) else {
        return 0;
    };
    let bits = bytes * 8;
    let mask = lane_mask(bits);
    int_lanes(a, b, bytes, move |x, y| {
        if signed {
            let (sx, sy) = (sign_extend(x, w) as i64, sign_extend(y, w) as i64);
            let brut = if sub { sx - sy } else { sx + sy };
            let haut = (1i64 << (bits - 1)) - 1;
            brut.clamp(-haut - 1, haut) as u64
        } else if sub {
            x.saturating_sub(y)
        } else {
            (x + y).min(mask)
        }
    })
}

/// `pshufb` : chaque octet du résultat est l'octet de la source que
/// désigne l'octet de même rang du contrôle — et il est **nul** si le
/// bit de poids fort de ce contrôle-là est mis.
///
/// C'est la table de correspondance du pauvre : seize recherches en une
/// instruction. Les routines de conversion (hexadécimal, base64,
/// UTF-8) ne font pratiquement que ça.
fn shuffle_bytes(src: u128, control: u128) -> u128 {
    let mut out = 0u128;
    for lane in 0..16 {
        let choix = (control >> (lane * 8)) as u8;
        if choix & 0x80 != 0 {
            continue;
        }
        let octet = (src >> (usize::from(choix & 0x0f) * 8)) as u8;
        out |= u128::from(octet) << (lane * 8);
    }
    out
}

/// `palignr` : les deux sources bout à bout — la première en haut —, le
/// tout décalé de `bytes` octets vers le bas, et l'on garde les seize du
/// bas.
///
/// Elle donne une fenêtre de seize octets à cheval sur deux lectures
/// alignées : c'est ainsi qu'un `memcpy` traite une adresse qui ne
/// tombe pas rond sans jamais faire de lecture non alignée.
fn align_right(a: u128, b: u128, bytes: u32) -> u128 {
    match bytes {
        0 => b,
        1..=15 => (b >> (bytes * 8)) | (a << (128 - bytes * 8)),
        16 => a,
        17..=31 => a >> ((bytes - 16) * 8),
        _ => 0,
    }
}

/// `pack*` : deux vecteurs d'éléments larges rendus dans un seul, deux
/// fois plus étroit — ceux de `a` en bas, ceux de `b` en haut.
///
/// Ce qui ne tient pas est **saturé**, et c'est la seule façon de
/// redescendre de seize à huit bits sans que 300 devienne 44. La source
/// est toujours lue comme signée ; `signed` ne dit que ce qu'il advient
/// de la destination — d'où `packuswb`, qui ramène tout négatif à zéro.
fn pack_lanes(a: u128, b: u128, bytes: usize, signed: bool) -> u128 {
    let Some(w) = width_from_bytes(bytes) else {
        return 0;
    };
    let étroit = bytes * 4;
    let voies = 16 / bytes;
    let mut out = 0u128;
    for lane in 0..voies {
        for (moitié, src) in [a, b].into_iter().enumerate() {
            let brut = (src >> (lane * bytes * 8)) as u64 & lane_mask(bytes * 8);
            let valeur = sign_extend(brut, w) as i64;
            let serré = if signed {
                let haut = (1i64 << (étroit - 1)) - 1;
                (valeur.clamp(-haut - 1, haut) as u64) & lane_mask(étroit)
            } else {
                valeur.clamp(0, (1i64 << étroit) - 1) as u64
            };
            out |= u128::from(serré) << ((moitié * voies + lane) * étroit);
        }
    }
    out
}

/// `pavgb`/`pavgw` : la moyenne arrondie au supérieur. L'addition se
/// fait sur une voie plus large que les éléments, donc rien ne déborde.
fn average_lanes(a: u128, b: u128, bytes: usize) -> u128 {
    int_lanes(a, b, bytes, |x, y| (x + y + 1) >> 1)
}

/// `psadbw` : la somme des écarts absolus, par moitiés de huit octets,
/// rangée dans le mot du bas de chaque quadruple mot.
///
/// Une mesure de ressemblance entre deux blocs en une instruction —
/// elle vient de la compression vidéo, et sert aussi à compter des
/// octets (somme d'un masque d'octets à 1).
fn sum_abs_diff(a: u128, b: u128) -> u128 {
    let mut out = 0u128;
    for moitié in 0..2 {
        let mut total = 0u64;
        for octet in 0..8 {
            let shift = (moitié * 8 + octet) * 8;
            total += u64::from(((a >> shift) as u8).abs_diff((b >> shift) as u8));
        }
        out |= u128::from(total) << (moitié * 64);
    }
    out
}

/// `pabs*` : la valeur absolue, voie par voie. La plus petite valeur
/// négative n'a pas d'opposé qui tienne dans sa voie, et reste
/// elle-même — c'est ce que fait le processeur, pas une approximation.
fn abs_lanes(v: u128, bytes: usize) -> u128 {
    let Some(w) = width_from_bytes(bytes) else {
        return 0;
    };
    int_lanes(v, v, bytes, move |x, _| {
        (sign_extend(x, w) as i64).unsigned_abs()
    })
}

/// Ce qu'une multiplication vectorielle garde du produit : un produit
/// de deux voies de `n` bits en fait `2n`, et il faut bien en jeter la
/// moitié.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MulPart {
    /// `pmullw`, `pmulld` : les bits du bas.
    Low,
    /// `pmulhw` : ceux du haut, les sources étant signées.
    HighSigned,
    /// `pmulhuw` : ceux du haut, les sources ne l'étant pas.
    HighUnsigned,
}

/// La multiplication voie par voie. Les deux moitiés du produit se
/// demandent en deux instructions, qui se suivent dans le code quand
/// c'est le produit entier qu'on veut.
fn mul_lanes(a: u128, b: u128, bytes: usize, part: MulPart) -> u128 {
    let Some(w) = width_from_bytes(bytes) else {
        return 0;
    };
    let bits = bytes * 8;
    int_lanes(a, b, bytes, move |x, y| match part {
        MulPart::Low => x.wrapping_mul(y),
        MulPart::HighUnsigned => ((u128::from(x) * u128::from(y)) >> bits) as u64,
        MulPart::HighSigned => {
            let produit =
                i128::from(sign_extend(x, w) as i64) * i128::from(sign_extend(y, w) as i64);
            (produit >> bits) as u64
        }
    })
}

/// `pmuludq`/`pmuldq` : deux multiplications 32 × 32 → 64, sur les voies
/// paires seulement. Le produit tient en entier — rien n'est jeté —, et
/// c'est la brique de l'arithmétique en grands nombres, celle de la
/// cryptographie.
fn mul_even_dwords(a: u128, b: u128, signed: bool) -> u128 {
    let mut out = 0u128;
    for lane in 0..2 {
        let shift = lane * 64;
        let produit = if signed {
            (i64::from((a >> shift) as u32 as i32) * i64::from((b >> shift) as u32 as i32)) as u64
        } else {
            u64::from((a >> shift) as u32) * u64::from((b >> shift) as u32)
        };
        out |= u128::from(produit) << shift;
    }
    out
}

/// `pmaddwd` : huit produits signés de seize bits, additionnés deux par
/// deux en quatre résultats de trente-deux. La largeur double au
/// passage, ce qui laisse la place à la somme : c'est la brique du
/// produit scalaire, et donc de presque tout le calcul matriciel.
fn madd_words(a: u128, b: u128) -> u128 {
    let mut out = 0u128;
    for lane in 0..4 {
        let lire = |v: u128, i: usize| i32::from((v >> ((lane * 2 + i) * 16)) as u16 as i16);
        let somme = lire(a, 0) * lire(b, 0) + lire(a, 1) * lire(b, 1);
        out |= u128::from(somme as u32) << (lane * 32);
    }
    out
}

/// `pmaddubsw` : la même idée sur des octets, et avec une asymétrie que
/// rien n'annonce dans le nom — la *première* source est non signée, la
/// seconde signée. C'est ce qu'il faut pour multiplier des pixels (0 à
/// 255) par des coefficients qui, eux, peuvent être négatifs.
fn madd_bytes(a: u128, b: u128) -> u128 {
    let mut out = 0u128;
    for lane in 0..8 {
        let sans_signe = |i: usize| i32::from((a >> ((lane * 2 + i) * 8)) as u8);
        let avec_signe = |i: usize| i32::from((b >> ((lane * 2 + i) * 8)) as u8 as i8);
        let somme =
            (sans_signe(0) * avec_signe(0) + sans_signe(1) * avec_signe(1)).clamp(-32768, 32767);
        out |= u128::from(somme as u16) << (lane * 16);
    }
    out
}

/// `phadd*`/`phsub*` : additionner — ou soustraire — les éléments **côte
/// à côte dans le même registre**, au lieu d'un registre sur l'autre.
///
/// C'est le geste qui manquait pour totaliser un vecteur : quatre
/// nombres, deux `phaddd`, et la somme est en bas.
fn horizontal_lanes(a: u128, b: u128, bytes: usize, sub: bool, saturate: bool) -> u128 {
    let Some(w) = width_from_bytes(bytes) else {
        return 0;
    };
    let bits = bytes * 8;
    let voies = 16 / bytes;
    let mut out = 0u128;
    for (moitié, src) in [a, b].into_iter().enumerate() {
        for paire in 0..(voies / 2) {
            let lire = |i: usize| {
                sign_extend(
                    (src >> ((paire * 2 + i) * bits)) as u64 & lane_mask(bits),
                    w,
                ) as i64
            };
            let brut = if sub {
                lire(0) - lire(1)
            } else {
                lire(0) + lire(1)
            };
            let valeur = if saturate {
                let haut = (1i64 << (bits - 1)) - 1;
                brut.clamp(-haut - 1, haut) as u64
            } else {
                brut as u64
            };
            out |= u128::from(valeur & lane_mask(bits)) << ((moitié * voies / 2 + paire) * bits);
        }
    }
    out
}

/// Les instructions de CHAÎNE de SSE4.2 : `pcmpistri` et ses trois
/// sœurs.
///
/// Elles font en une instruction ce qu'une boucle fait en vingt :
/// comparer seize octets à seize autres, selon l'une de quatre
/// questions, et dire où. La bibliothèque C s'en sert pour `strlen`,
/// `strchr`, `strcmp` et `strstr` — d'où les quatre cent et quelques
/// occurrences qu'on en compte dans les binaires d'épreuve, et l'arrêt
/// qu'elles provoquaient.
///
/// Leur immédiat porte tout : le format des éléments, la question
/// posée, la polarité de la réponse, et la forme de la sortie. C'est
/// l'instruction la plus paramétrée du jeu x86, et la lire est la
/// moitié du travail.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Contrôle {
    /// Éléments de 8 bits, ou de 16.
    octets: bool,
    /// Lus comme signés.
    signé: bool,
    /// La question posée — les bits 3 et 2 de l'immédiat.
    mode: Agrégation,
    /// Le résultat est-il retourné ? Et l'est-il seulement là où les
    /// éléments sont valides (« masqué ») ?
    négatif: bool,
    masqué: bool,
    /// Pour un index : le bit de poids fort plutôt que le faible. Pour
    /// un masque : un élément entier par réponse plutôt qu'un bit.
    haut: bool,
}

/// La question que l'instruction pose à chaque paire d'éléments.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Agrégation {
    /// « Cet élément est-il dans l'ensemble ? » — `strchr`, `strspn`.
    EqualAny,
    /// « Est-il dans l'un de ces intervalles ? » — les classes de
    /// caractères, `isalpha` vectorisé.
    Ranges,
    /// « Les deux chaînes sont-elles égales, élément par élément ? » —
    /// `strcmp`, et `strlen` en comparant à elle-même.
    EqualEach,
    /// « La première apparaît-elle ICI dans la seconde ? » — `strstr`.
    EqualOrdered,
}

impl Contrôle {
    fn lire(imm: u8) -> Self {
        Self {
            octets: imm & 0b01 == 0,
            signé: imm & 0b10 != 0,
            mode: match (imm >> 2) & 0b11 {
                0 => Agrégation::EqualAny,
                1 => Agrégation::Ranges,
                2 => Agrégation::EqualEach,
                _ => Agrégation::EqualOrdered,
            },
            négatif: (imm >> 4) & 1 == 1,
            masqué: (imm >> 5) & 1 == 1,
            haut: (imm >> 6) & 1 == 1,
        }
    }

    /// Combien d'éléments tiennent dans les 128 bits : seize octets, ou
    /// huit mots.
    fn voies(self) -> usize {
        if self.octets { 16 } else { 8 }
    }

    /// Un élément du vecteur, lu à sa largeur et à son signe.
    fn élément(self, v: u128, i: usize) -> i32 {
        if self.octets {
            let b = (v >> (i * 8)) as u8;
            if self.signé {
                i32::from(b as i8)
            } else {
                i32::from(b)
            }
        } else {
            let w = (v >> (i * 16)) as u16;
            if self.signé {
                i32::from(w as i16)
            } else {
                i32::from(w)
            }
        }
    }
}

/// Combien d'éléments d'un vecteur comptent, une chaîne s'arrêtant à son
/// premier élément NUL.
///
/// C'est le « i » d'`istri` : la longueur est *implicite*, lue dans les
/// données. Les formes `estri` la reçoivent dans un registre, et c'est
/// toute leur différence.
fn longueur_implicite(ctrl: Contrôle, v: u128) -> usize {
    (0..ctrl.voies())
        .position(|i| ctrl.élément(v, i) == 0)
        .unwrap_or(ctrl.voies())
}

/// Le résultat brut de la comparaison : un bit par position de la
/// SECONDE source, avant la polarité.
///
/// `a` est l'ensemble ou le motif, `b` la chaîne où l'on cherche —
/// l'ordre du manuel, et celui qu'on écrit : `pcmpistri xmm1, xmm2`
/// cherche ce que porte xmm1 dans ce que porte xmm2.
fn comparer_chaînes(ctrl: Contrôle, a: u128, la: usize, b: u128, lb: usize) -> u16 {
    let n = ctrl.voies();
    let mut res: u16 = 0;
    for j in 0..n {
        let valide_b = j < lb;
        let vrai = match ctrl.mode {
            // Un élément invalide ne correspond à rien.
            Agrégation::EqualAny => {
                (0..n).any(|i| i < la && valide_b && ctrl.élément(a, i) == ctrl.élément(b, j))
            }
            // Les intervalles vont par paires : bas, haut, bas, haut.
            Agrégation::Ranges => (0..n / 2).any(|paire| {
                let (bas, haut) = (paire * 2, paire * 2 + 1);
                bas < la
                    && valide_b
                    && ctrl.élément(a, bas) <= ctrl.élément(b, j)
                    && ctrl.élément(b, j) <= ctrl.élément(a, haut.min(n - 1))
            }),
            // DEUX invalides se valent : c'est ce qui fait que deux
            // chaînes plus courtes que seize octets se disent égales
            // au-delà de leur fin. Un seul invalide, jamais.
            Agrégation::EqualEach => match (j < la, valide_b) {
                (true, true) => ctrl.élément(a, j) == ctrl.élément(b, j),
                (false, false) => true,
                _ => false,
            },
            // Le motif, cherché À PARTIR de j. Un élément du motif
            // au-delà de sa fin vaut vrai — le motif est trouvé ; un
            // élément de la chaîne au-delà de la sienne, faux.
            Agrégation::EqualOrdered => (0..n - j).all(|k| {
                let i = k;
                if i >= la {
                    true
                } else if j + k >= lb {
                    false
                } else {
                    ctrl.élément(a, i) == ctrl.élément(b, j + k)
                }
            }),
        };
        if vrai {
            res |= 1 << j;
        }
    }
    res
}

/// La polarité : rien, tout retourné, ou retourné seulement là où la
/// chaîne est valide.
///
/// La troisième est celle qui compte dans la vraie vie : `strlen`
/// demande « où est le premier élément qui n'est PAS là », et retourner
/// aussi le remplissage au-delà de la fin donnerait toujours seize.
fn polarité(ctrl: Contrôle, res: u16, lb: usize) -> u16 {
    let n = ctrl.voies();
    let tous = if n >= 16 { u16::MAX } else { (1u16 << n) - 1 };
    match (ctrl.négatif, ctrl.masqué) {
        (false, _) => res,
        (true, false) => res ^ tous,
        (true, true) => {
            let valides = if lb >= 16 { u16::MAX } else { (1u16 << lb) - 1 };
            (res ^ valides) & tous
        }
    }
}

/// `pmovzx*`/`pmovsx*` : les éléments du bas de la source, élargis — par
/// des zéros ou par leur bit de signe. Rien ne se perd : c'est l'inverse
/// exact de `pack*`.
fn extend_lanes(src: u128, from: usize, to: usize, signed: bool) -> u128 {
    let Some(w) = width_from_bytes(from) else {
        return 0;
    };
    let mut out = 0u128;
    for lane in 0..(16 / to) {
        let valeur = (src >> (lane * from * 8)) as u64 & lane_mask(from * 8);
        let large = if signed {
            sign_extend(valeur, w)
        } else {
            valeur
        };
        out |= u128::from(large & lane_mask(to * 8)) << (lane * to * 8);
    }
    out
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

/// `pshuflw`/`pshufhw` : le brassage des mots de seize bits, dans une
/// moitié du registre seulement — l'autre est recopiée telle quelle.
///
/// C'est l'outil des routines de chaînes : `strlen` et ses semblables
/// s'en servent pour rapprocher les octets qu'elles viennent de comparer.
fn shuffle_words(src: u128, control: u64, high: bool) -> u128 {
    let décalage = if high { 64 } else { 0 };
    let moitié = (src >> décalage) & u128::from(u64::MAX);
    let mut brassée = 0u128;
    for lane in 0..4 {
        let from = (control >> (lane * 2)) & 3;
        brassée |= ((moitié >> (from * 16)) & u128::from(u16::MAX)) << (lane * 16);
    }
    let autre = if high {
        src & u128::from(u64::MAX)
    } else {
        (src >> 64) << 64
    };
    autre | (brassée << décalage)
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
        // UN MASQUE D'OPÉRANDE FAIT REFUSER. L'encodage EVEX permet
        // d'écrire `vmovdqu64 ymm0{k1}{z}, [rsi]` : l'opération ne
        // s'applique qu'aux voies que le registre de masque désigne, et
        // les autres gardent leur valeur ou tombent à zéro. L'ignorer
        // donnerait un résultat faux sans le dire — le pire des trois
        // comportements possibles.
        //
        // Les registres de masque eux-mêmes n'existent pas encore ici,
        // et c'est un chantier de structure : tant qu'ils manquent, on
        // s'arrête en nommant ce qui manque.
        // UN MASQUE D'OPÉRANDE s'applique élément par élément, et
        // `write_vec_op` sait le faire — à condition de savoir ce
        // qu'est un élément. Pour ce dont on ne sait pas le dire, on
        // s'arrête : se tromper d'élément décalerait les voies
        // retenues et rendrait un résultat faux sans rien signaler.
        if instr.op_mask() != iced_x86::Register::None
            && largeur_élément(instr.mnemonic()).is_none()
        {
            return Err(EmuError::UnsupportedInstruction {
                backend: Self::NAME,
                mnemonic: format!(
                    "{:?} (masque AVX-512 sur un élément inconnu)",
                    instr.mnemonic()
                ),
            });
        }
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
            | M::Vmovdqu
            // LES MÊMES, ÉCRITS EN EVEX. AVX-512 donne un nom par
            // largeur d'élément — `vmovdqu8`, `vmovdqu64` — parce que
            // son masque s'applique élément par élément et qu'il faut
            // donc savoir ce qu'est un élément. SANS masque, et sur un
            // registre de 128 ou 256 bits, elles déplacent exactement
            // ce que déplace `vmovdqu` : à elles seules, elles font
            // plus de mille deux cents des occurrences AVX-512 des
            // binaires d'épreuve.
            | M::Vmovdqu8
            | M::Vmovdqu16
            | M::Vmovdqu32
            | M::Vmovdqu64
            | M::Vmovdqa32
            | M::Vmovdqa64
            // Les déplacements « non temporels » : au processeur, ils
            // disent de ne pas garder la donnée en cache — on écrit un
            // tampon qu'on ne relira pas. À l'émulation, un cache n'est
            // rien : ce sont des déplacements comme les autres. De même
            // `lddqu`, qui n'est qu'un `movdqu` plus adroit à cheval sur
            // deux lignes de cache.
            | M::Movntps
            | M::Movntpd
            | M::Movntdq
            | M::Movntdqa
            | M::Lddqu
            | M::Vmovntps
            | M::Vmovntpd
            | M::Vmovntdq
            | M::Vmovntdqa
            | M::Vlddqu => {
                let src = self.read_vec_op(instr, instr.op_count() - 1)?;
                // Autant de tranches que la destination en porte : une
                // pour `xmm`, deux pour `ymm`, QUATRE pour `zmm`. En
                // copier deux quoi qu'il arrive perdait la moitié haute
                // d'un registre de 512 sans rien dire.
                let tranches = self.tranches(instr);
                let mut out = [0u128; 4];
                out[..tranches].copy_from_slice(&src[..tranches]);
                self.write_vec_op(instr, 0, out)?;
            }
            // Les demi-déplacements : huit octets qui vont dans une
            // moitié du registre et laissent l'autre intacte. C'est ainsi
            // qu'une bibliothèque C recopie une chaîne de moins de seize
            // octets, et on les croise donc partout dans un vrai binaire.
            M::Movlps | M::Movlpd | M::Vmovlps | M::Vmovlpd => self.move_half(instr, false)?,
            M::Movhps | M::Movhpd | M::Vmovhps | M::Vmovhpd => self.move_half(instr, true)?,
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
                self.write_vec_op(instr, 0, [(upper & !mask) | low, 0, 0, 0])?;
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
                self.write_vec_op(instr, 0, [(upper & !scalar_mask(8)) | low, 0, 0, 0])?;
            }
            M::Cvtsd2ss | M::Vcvtsd2ss => {
                let last = instr.op_count() - 1;
                let src = f64::from_bits(self.read_vec_op(instr, last)?[0] as u64);
                let upper = self.read_vec_op(instr, if instr.op_count() >= 3 { 1 } else { 0 })?[0];
                let low = u128::from((src as f32).to_bits());
                self.write_vec_op(instr, 0, [(upper & !scalar_mask(4)) | low, 0, 0, 0])?;
            }

            // Les opérations bit à bit. `xorps xmm0, xmm0` et
            // `pxor xmm0, xmm0` sont la façon habituelle de remettre un
            // registre à zéro.
            M::Xorps
            | M::Xorpd
            | M::Pxor
            | M::Vxorps
            | M::Vxorpd
            | M::Vpxor
            // Les formes EVEX : le `d` et le `q` final ne disent que la
            // largeur d'élément que le masque emploierait, et sans
            // masque un OU exclusif est un OU exclusif.
            | M::Vpxord
            | M::Vpxorq => {
                self.vector_binary(instr, |a, b| a ^ b)?
            }
            M::Andps
            | M::Andpd
            | M::Pand
            | M::Vandps
            | M::Vandpd
            | M::Vpand
            | M::Vpandd
            | M::Vpandq => {
                self.vector_binary(instr, |a, b| a & b)?
            }
            M::Orps
            | M::Orpd
            | M::Por
            | M::Vorps
            | M::Vorpd
            | M::Vpor
            | M::Vpord
            | M::Vporq => {
                self.vector_binary(instr, |a, b| a | b)?
            }
            M::Andnps
            | M::Andnpd
            | M::Pandn
            | M::Vandnps
            | M::Vandnpd
            | M::Vpandn
            | M::Vpandnd
            | M::Vpandnq => {
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

            M::Pcmpeqq | M::Vpcmpeqq => self.vector_binary(instr, |a, b| {
                int_lanes(a, b, 8, |x, y| if x == y { !0 } else { 0 })
            })?,
            M::Pcmpgtq | M::Vpcmpgtq => {
                self.vector_binary(instr, |a, b| signed_gt_lanes(a, b, 8))?
            }

            // Le minimum et le maximum, voie par voie. SSE2 n'en donnait
            // que deux formes — l'octet non signé et le mot signé, les
            // deux dont les chaînes et le son avaient besoin ; SSE4.1 a
            // complété le tableau.
            M::Pminub | M::Vpminub => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 1, false, false))?
            }
            M::Pmaxub | M::Vpmaxub => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 1, false, true))?
            }
            M::Pminsb | M::Vpminsb => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 1, true, false))?
            }
            M::Pmaxsb | M::Vpmaxsb => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 1, true, true))?
            }
            M::Pminuw | M::Vpminuw => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 2, false, false))?
            }
            M::Pmaxuw | M::Vpmaxuw => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 2, false, true))?
            }
            M::Pminsw | M::Vpminsw => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 2, true, false))?
            }
            M::Pmaxsw | M::Vpmaxsw => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 2, true, true))?
            }
            M::Pminud | M::Vpminud => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 4, false, false))?
            }
            M::Pmaxud | M::Vpmaxud => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 4, false, true))?
            }
            M::Pminsd | M::Vpminsd => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 4, true, false))?
            }
            M::Pmaxsd | M::Vpmaxsd => {
                self.vector_binary(instr, |a, b| minmax_lanes(a, b, 4, true, true))?
            }

            // L'addition et la soustraction qui s'arrêtent à la borne.
            M::Paddusb | M::Vpaddusb => {
                self.vector_binary(instr, |a, b| saturating_lanes(a, b, 1, false, false))?
            }
            M::Paddusw | M::Vpaddusw => {
                self.vector_binary(instr, |a, b| saturating_lanes(a, b, 2, false, false))?
            }
            M::Paddsb | M::Vpaddsb => {
                self.vector_binary(instr, |a, b| saturating_lanes(a, b, 1, true, false))?
            }
            M::Paddsw | M::Vpaddsw => {
                self.vector_binary(instr, |a, b| saturating_lanes(a, b, 2, true, false))?
            }
            M::Psubusb | M::Vpsubusb => {
                self.vector_binary(instr, |a, b| saturating_lanes(a, b, 1, false, true))?
            }
            M::Psubusw | M::Vpsubusw => {
                self.vector_binary(instr, |a, b| saturating_lanes(a, b, 2, false, true))?
            }
            M::Psubsb | M::Vpsubsb => {
                self.vector_binary(instr, |a, b| saturating_lanes(a, b, 1, true, true))?
            }
            M::Psubsw | M::Vpsubsw => {
                self.vector_binary(instr, |a, b| saturating_lanes(a, b, 2, true, true))?
            }

            // La moyenne, l'écart absolu et la valeur absolue : trois
            // mesures que le jeu rend en une instruction parce que le
            // traitement d'image et le son les demandent partout.
            M::Pavgb | M::Vpavgb => self.vector_binary(instr, |a, b| average_lanes(a, b, 1))?,
            M::Pavgw | M::Vpavgw => self.vector_binary(instr, |a, b| average_lanes(a, b, 2))?,
            M::Psadbw | M::Vpsadbw => self.vector_binary(instr, sum_abs_diff)?,
            M::Pabsb | M::Vpabsb => self.vector_binary(instr, |_, b| abs_lanes(b, 1))?,
            M::Pabsw | M::Vpabsw => self.vector_binary(instr, |_, b| abs_lanes(b, 2))?,
            M::Pabsd | M::Vpabsd => self.vector_binary(instr, |_, b| abs_lanes(b, 4))?,

            // La multiplication. Un produit de deux voies de n bits en
            // fait 2n : il faut dire laquelle des deux moitiés on garde.
            M::Pmullw | M::Vpmullw => {
                self.vector_binary(instr, |a, b| mul_lanes(a, b, 2, MulPart::Low))?
            }
            M::Pmulhw | M::Vpmulhw => {
                self.vector_binary(instr, |a, b| mul_lanes(a, b, 2, MulPart::HighSigned))?
            }
            M::Pmulhuw | M::Vpmulhuw => {
                self.vector_binary(instr, |a, b| mul_lanes(a, b, 2, MulPart::HighUnsigned))?
            }
            M::Pmulld | M::Vpmulld => {
                self.vector_binary(instr, |a, b| mul_lanes(a, b, 4, MulPart::Low))?
            }
            M::Pmuludq | M::Vpmuludq => {
                self.vector_binary(instr, |a, b| mul_even_dwords(a, b, false))?
            }
            M::Pmuldq | M::Vpmuldq => {
                self.vector_binary(instr, |a, b| mul_even_dwords(a, b, true))?
            }

            M::Pmaddwd | M::Vpmaddwd => self.vector_binary(instr, madd_words)?,
            M::Pmaddubsw | M::Vpmaddubsw => self.vector_binary(instr, madd_bytes)?,
            M::Phaddw | M::Vphaddw => {
                self.vector_binary(instr, |a, b| horizontal_lanes(a, b, 2, false, false))?
            }
            M::Phaddd | M::Vphaddd => {
                self.vector_binary(instr, |a, b| horizontal_lanes(a, b, 4, false, false))?
            }
            M::Phaddsw | M::Vphaddsw => {
                self.vector_binary(instr, |a, b| horizontal_lanes(a, b, 2, false, true))?
            }
            M::Phsubw | M::Vphsubw => {
                self.vector_binary(instr, |a, b| horizontal_lanes(a, b, 2, true, false))?
            }
            M::Phsubd | M::Vphsubd => {
                self.vector_binary(instr, |a, b| horizontal_lanes(a, b, 4, true, false))?
            }
            M::Phsubsw | M::Vphsubsw => {
                self.vector_binary(instr, |a, b| horizontal_lanes(a, b, 2, true, true))?
            }

            // Réordonner, élargir, rétrécir : le travail d'emballage,
            // qui occupe la moitié d'une routine vectorielle réelle.
            M::Pshufb | M::Vpshufb => self.vector_binary(instr, shuffle_bytes)?,
            M::Palignr | M::Vpalignr => {
                let octets = self.read_op(instr, instr.op_count() - 1)? as u32;
                self.vector_binary_extra(instr, 1, move |a, b| align_right(a, b, octets))?;
            }
            M::Packsswb | M::Vpacksswb => {
                self.vector_binary(instr, |a, b| pack_lanes(a, b, 2, true))?
            }
            M::Packssdw | M::Vpackssdw => {
                self.vector_binary(instr, |a, b| pack_lanes(a, b, 4, true))?
            }
            M::Packuswb | M::Vpackuswb => {
                self.vector_binary(instr, |a, b| pack_lanes(a, b, 2, false))?
            }
            M::Packusdw | M::Vpackusdw => {
                self.vector_binary(instr, |a, b| pack_lanes(a, b, 4, false))?
            }
            M::Pmovzxbw | M::Vpmovzxbw => self.extend_packed(instr, 1, 2, false)?,
            M::Pmovzxbd | M::Vpmovzxbd => self.extend_packed(instr, 1, 4, false)?,
            M::Pmovzxbq | M::Vpmovzxbq => self.extend_packed(instr, 1, 8, false)?,
            M::Pmovzxwd | M::Vpmovzxwd => self.extend_packed(instr, 2, 4, false)?,
            M::Pmovzxwq | M::Vpmovzxwq => self.extend_packed(instr, 2, 8, false)?,
            M::Pmovzxdq | M::Vpmovzxdq => self.extend_packed(instr, 4, 8, false)?,
            M::Pmovsxbw | M::Vpmovsxbw => self.extend_packed(instr, 1, 2, true)?,
            M::Pmovsxbd | M::Vpmovsxbd => self.extend_packed(instr, 1, 4, true)?,
            M::Pmovsxbq | M::Vpmovsxbq => self.extend_packed(instr, 1, 8, true)?,
            M::Pmovsxwd | M::Vpmovsxwd => self.extend_packed(instr, 2, 4, true)?,
            M::Pmovsxwq | M::Vpmovsxwq => self.extend_packed(instr, 2, 8, true)?,
            M::Pmovsxdq | M::Vpmovsxdq => self.extend_packed(instr, 4, 8, true)?,

            // Une seule voie, nommée par un immédiat : le pont entre un
            // registre vectoriel et un registre général.
            M::Pinsrb | M::Vpinsrb => self.insert_element(instr, 1)?,
            M::Pinsrw | M::Vpinsrw => self.insert_element(instr, 2)?,
            M::Pinsrd | M::Vpinsrd => self.insert_element(instr, 4)?,
            M::Pinsrq | M::Vpinsrq => self.insert_element(instr, 8)?,
            M::Pextrb | M::Vpextrb => self.extract_element(instr, 1)?,
            M::Pextrw | M::Vpextrw => self.extract_element(instr, 2)?,
            M::Pextrd | M::Vpextrd => self.extract_element(instr, 4)?,
            M::Pextrq | M::Vpextrq => self.extract_element(instr, 8)?,

            M::Ptest | M::Vptest => self.packed_test(instr)?,

            // --- Les registres de MASQUE d'AVX-512 -------------------
            // Un bit par voie, là où les jeux précédents écrivaient un
            // vecteur de zéros et de uns. C'est le seul morceau
            // d'AVX-512 qui ne demande pas d'élargir les registres
            // vectoriels : huit entiers de soixante-quatre bits.
            M::Kmovb => self.move_mask(instr, 8)?,
            M::Kmovw => self.move_mask(instr, 16)?,
            M::Kmovd => self.move_mask(instr, 32)?,
            M::Kmovq => self.move_mask(instr, 64)?,

            M::Kandb => self.mask_binary(instr, 8, |a, b| a & b)?,
            M::Kandw => self.mask_binary(instr, 16, |a, b| a & b)?,
            M::Kandd => self.mask_binary(instr, 32, |a, b| a & b)?,
            M::Kandq => self.mask_binary(instr, 64, |a, b| a & b)?,
            M::Korb => self.mask_binary(instr, 8, |a, b| a | b)?,
            M::Korw => self.mask_binary(instr, 16, |a, b| a | b)?,
            M::Kord => self.mask_binary(instr, 32, |a, b| a | b)?,
            M::Korq => self.mask_binary(instr, 64, |a, b| a | b)?,
            M::Kxorb => self.mask_binary(instr, 8, |a, b| a ^ b)?,
            M::Kxorw => self.mask_binary(instr, 16, |a, b| a ^ b)?,
            M::Kxord => self.mask_binary(instr, 32, |a, b| a ^ b)?,
            M::Kxorq => self.mask_binary(instr, 64, |a, b| a ^ b)?,
            M::Kandnb => self.mask_binary(instr, 8, |a, b| !a & b)?,
            M::Kandnw => self.mask_binary(instr, 16, |a, b| !a & b)?,
            M::Kandnd => self.mask_binary(instr, 32, |a, b| !a & b)?,
            M::Kandnq => self.mask_binary(instr, 64, |a, b| !a & b)?,
            M::Kxnorb => self.mask_binary(instr, 8, |a, b| !(a ^ b))?,
            M::Kxnorw => self.mask_binary(instr, 16, |a, b| !(a ^ b))?,
            M::Kxnord => self.mask_binary(instr, 32, |a, b| !(a ^ b))?,
            M::Kxnorq => self.mask_binary(instr, 64, |a, b| !(a ^ b))?,
            M::Knotb => self.mask_binary(instr, 8, |a, _| !a)?,
            M::Knotw => self.mask_binary(instr, 16, |a, _| !a)?,
            M::Knotd => self.mask_binary(instr, 32, |a, _| !a)?,
            M::Knotq => self.mask_binary(instr, 64, |a, _| !a)?,
            // `kadd*` additionne deux masques comme des nombres : on
            // s'en sert pour avancer un compteur de voies.
            M::Kaddb => self.mask_binary(instr, 8, |a, b| a.wrapping_add(b))?,
            M::Kaddw => self.mask_binary(instr, 16, |a, b| a.wrapping_add(b))?,
            M::Kaddd => self.mask_binary(instr, 32, |a, b| a.wrapping_add(b))?,
            M::Kaddq => self.mask_binary(instr, 64, |a, b| a.wrapping_add(b))?,

            M::Kshiftlb => self.mask_shift(instr, 8, true)?,
            M::Kshiftlw => self.mask_shift(instr, 16, true)?,
            M::Kshiftld => self.mask_shift(instr, 32, true)?,
            M::Kshiftlq => self.mask_shift(instr, 64, true)?,
            M::Kshiftrb => self.mask_shift(instr, 8, false)?,
            M::Kshiftrw => self.mask_shift(instr, 16, false)?,
            M::Kshiftrd => self.mask_shift(instr, 32, false)?,
            M::Kshiftrq => self.mask_shift(instr, 64, false)?,

            M::Kortestb => self.mask_test(instr, 8, true)?,
            M::Kortestw => self.mask_test(instr, 16, true)?,
            M::Kortestd => self.mask_test(instr, 32, true)?,
            M::Kortestq => self.mask_test(instr, 64, true)?,
            M::Ktestb => self.mask_test(instr, 8, false)?,
            M::Ktestw => self.mask_test(instr, 16, false)?,
            M::Ktestd => self.mask_test(instr, 32, false)?,
            M::Ktestq => self.mask_test(instr, 64, false)?,

            // `kunpck` colle deux masques bout à bout : la moitié
            // basse du résultat vient du second, la haute du premier.
            M::Kunpckbw => self.mask_binary(instr, 16, |a, b| (a << 8) | (b & 0xff))?,
            M::Kunpckwd => self.mask_binary(instr, 32, |a, b| (a << 16) | (b & 0xffff))?,
            M::Kunpckdq => {
                self.mask_binary(instr, 64, |a, b| (a << 32) | (b & 0xffff_ffff))?
            }

            // Ce qui ÉCRIT dans un masque : la comparaison voie par
            // voie, et le test voie par voie.
            M::Vptestmb => self.test_into_mask(instr, 1, false)?,
            M::Vptestmw => self.test_into_mask(instr, 2, false)?,
            M::Vptestmd => self.test_into_mask(instr, 4, false)?,
            M::Vptestmq => self.test_into_mask(instr, 8, false)?,
            M::Vptestnmb => self.test_into_mask(instr, 1, true)?,
            M::Vptestnmw => self.test_into_mask(instr, 2, true)?,
            M::Vptestnmd => self.test_into_mask(instr, 4, true)?,
            M::Vptestnmq => self.test_into_mask(instr, 8, true)?,

            M::Vpcmpb => self.compare_into_mask(instr, 1, true)?,
            M::Vpcmpw => self.compare_into_mask(instr, 2, true)?,
            M::Vpcmpd => self.compare_into_mask(instr, 4, true)?,
            M::Vpcmpq => self.compare_into_mask(instr, 8, true)?,
            M::Vpcmpub => self.compare_into_mask(instr, 1, false)?,
            M::Vpcmpuw => self.compare_into_mask(instr, 2, false)?,
            M::Vpcmpud => self.compare_into_mask(instr, 4, false)?,
            M::Vpcmpuq => self.compare_into_mask(instr, 8, false)?,

            // Les instructions de chaîne de SSE4.2 : une boucle de
            // `strlen` en une instruction, et c'est pour cela que la
            // bibliothèque C en est pleine.
            M::Pcmpistri | M::Vpcmpistri => self.packed_compare_strings(instr, false, true)?,
            M::Pcmpistrm | M::Vpcmpistrm => self.packed_compare_strings(instr, false, false)?,
            M::Pcmpestri | M::Vpcmpestri => self.packed_compare_strings(instr, true, true)?,
            M::Pcmpestrm | M::Vpcmpestrm => self.packed_compare_strings(instr, true, false)?,

            // Demander au cache de préparer une ligne : une *indication*,
            // qui ne change aucun registre ni aucun octet de mémoire.
            // Ici, où il n'y a pas de cache, elle ne fait rien — mais
            // elle doit passer, sans quoi un binaire optimisé s'arrête
            // sur une instruction qui, sur la vraie machine, n'a aucun
            // effet observable.
            M::Prefetch
            | M::Prefetchw
            | M::Prefetchwt1
            | M::Prefetchnta
            | M::Prefetcht0
            | M::Prefetcht1
            | M::Prefetcht2 => {}

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
            M::Pshuflw | M::Vpshuflw => {
                let control = self.read_op(instr, instr.op_count() - 1)?;
                self.vector_binary_extra(instr, 1, move |_, b| shuffle_words(b, control, false))?;
            }
            M::Pshufhw | M::Vpshufhw => {
                let control = self.read_op(instr, instr.op_count() - 1)?;
                self.vector_binary_extra(instr, 1, move |_, b| shuffle_words(b, control, true))?;
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
                self.write_vec_op(instr, 0, [half, if wide { half } else { 0 }, 0, 0])?;
            }
            // Les deux moitiés d'un registre 256 bits ne se parlent pas
            // d'elles-mêmes : il faut les instructions qui les nomment.
            M::Vinsertf128 | M::Vinserti128 => {
                let dest = self.read_vec_op(instr, 1)?;
                let src = self.read_vec_op(instr, 2)?[0];
                let high = self.read_op(instr, 3)? & 1 == 1;
                let out = if high {
                    [dest[0], src, 0, 0]
                } else {
                    [src, dest[1], 0, 0]
                };
                self.write_vec_op(instr, 0, out)?;
            }
            M::Vextractf128 | M::Vextracti128 => {
                let src = self.read_vec_op(instr, 1)?;
                let high = self.read_op(instr, 2)? & 1 == 1;
                let taken = if high { src[1] } else { src[0] };
                self.write_vec_op(instr, 0, [taken, 0, 0, 0])?;
            }
            // `vpblendw` choisit, mot de seize bits par mot de seize
            // bits, entre deux sources — l'immédiat dit lequel prendre.
            //
            // Huit bits pour seize mots sur 256 : le motif se RÉPÈTE sur
            // chaque moitié de 128 bits. C'est la règle qu'on oublie, et
            // elle vient de l'origine en SSE, où il n'y avait qu'une
            // moitié ; AVX a élargi le registre sans élargir l'immédiat.
            //
            // Un bit à zéro prend la première source, à un la seconde —
            // l'inverse de ce que l'intuition souffle, et le manuel est
            // formel.
            M::Pblendw | M::Vpblendw => {
                let a = self.read_vec_op(instr, 1)?;
                let b = self.read_vec_op(instr, 2)?;
                let motif = self.read_op(instr, 3)? as u8;
                let mut out = [0u128; 4];
                for (moitié, case) in out.iter_mut().enumerate() {
                    let (gauche, droite) = (a[moitié], b[moitié]);
                    let mut assemblé = 0u128;
                    for mot in 0..8u32 {
                        let décalage = mot * 16;
                        let pris = if motif & (1 << mot) != 0 {
                            droite
                        } else {
                            gauche
                        };
                        assemblé |= ((pris >> décalage) & 0xffff) << décalage;
                    }
                    *case = assemblé;
                }
                self.write_vec_op(instr, 0, out)?;
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
                let out = [pick(control & 0xf), pick((control >> 4) & 0xf), 0, 0];
                self.write_vec_op(instr, 0, out)?;
            }
            M::Vpermq | M::Vpermpd => {
                let src = self.read_vec_op(instr, 1)?;
                let control = self.read_op(instr, 2)?;
                let quad = |index: u64| -> u128 {
                    let half = src[(index / 2) as usize];
                    (half >> ((index % 2) * 64)) & u128::from(u64::MAX)
                };
                let mut out = [0u128; 4];
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
                    .map(|r| self.cpu.regs.get_zmm(r))
                    .ok_or_else(|| EmuError::UnsupportedRegister {
                        backend: Self::NAME,
                        register: format!("{reg:?}"),
                    })
            }
            OpKind::Memory => {
                let addr = self.effective_address(instr, op_index)?;
                let bytes = instr.memory_size().size().min(64);
                let raw = self.cpu.mem.bytes_at(addr, bytes)?;
                let mut buf = [0u8; 64];
                buf[..bytes].copy_from_slice(raw);
                let (chunks, _) = buf.as_chunks::<16>();
                let mut tranches = [0u128; 4];
                for (tranche, chunk) in tranches.iter_mut().zip(chunks) {
                    *tranche = u128::from_le_bytes(*chunk);
                }
                Ok(tranches)
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
        // LE MASQUE, s'il y en a un : la valeur calculée se compose
        // avec ce que la destination portait, voie par voie. Fait ICI
        // et non dans chaque instruction — toutes passent par là, et
        // une règle écrite cent fois est une règle qu'on applique
        // quatre-vingt-dix-neuf fois.
        let value = match map_mask(instr.op_mask()) {
            Some(k) => {
                let bytes = largeur_élément(instr.mnemonic()).unwrap_or(8);
                let ancien = self.read_vec_op(instr, op_index).unwrap_or([0; 4]);
                appliquer_masque(
                    value,
                    ancien,
                    self.cpu.regs.get_mask(k),
                    bytes,
                    instr.zeroing_masking(),
                )
            }
            None => value,
        };
        match instr.op_kind(op_index) {
            OpKind::Register => {
                let reg = instr.op_register(op_index);
                match map_vector(reg) {
                    Some(r) => {
                        if keeps_upper_half(instr) {
                            // SSE : seuls les 128 bits de bas changent,
                            // tout le reste est laissé où il est.
                            self.cpu.regs.set_xmm(r, value[0]);
                        } else {
                            // VEX et EVEX remettent à zéro ce qu'ils
                            // n'écrivent pas : une instruction qui
                            // nomme `ymm0` efface les bits 256 à 511,
                            // et c'est la règle qu'on oublie en mêlant
                            // les jeux.
                            let mut out = [0u128; 4];
                            let tranches = if reg.is_zmm() {
                                4
                            } else if reg.is_ymm() {
                                2
                            } else {
                                1
                            };
                            out[..tranches].copy_from_slice(&value[..tranches]);
                            self.cpu.regs.set_zmm(r, out);
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
                let bytes = instr.memory_size().size().min(64);
                let mut buf = [0u8; 64];
                for (i, tranche) in value.iter().enumerate() {
                    buf[i * 16..(i + 1) * 16].copy_from_slice(&tranche.to_le_bytes());
                }
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
        self.tranches(instr) >= 2
    }

    /// Combien de tranches de 128 bits l'instruction écrit : une pour
    /// `xmm`, deux pour `ymm`, quatre pour `zmm`.
    ///
    /// Lue sur la DESTINATION, qui est ce qui décide : `vmovdqu64 ymm0,
    /// [rsi]` écrit 256 bits quelle que soit la taille de la zone lue.
    fn tranches(&self, instr: &Instruction) -> usize {
        match instr.op_kind(0) {
            OpKind::Register => {
                let reg = instr.op_register(0);
                if reg.is_zmm() {
                    4
                } else if reg.is_ymm() {
                    2
                } else {
                    1
                }
            }
            OpKind::Memory => instr.memory_size().size().div_ceil(16).clamp(1, 4),
            _ => 1,
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
        let tranches = self.tranches(instr);
        let mut out = [0u128; 4];
        for (i, place) in out.iter_mut().enumerate().take(tranches) {
            *place = f(a[i], b[i]);
        }
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
            return self.write_vec_op(instr, 0, [src[0] & mask, 0, 0, 0]);
        }
        if instr.op_count() >= 3 {
            // `vmovss xmm0, xmm1, xmm2` : le haut vient de la première
            // source, l'élément du bas de la seconde.
            let upper = self.read_vec_op(instr, 1)?[0] & !mask;
            let low = self.read_vec_op(instr, 2)?[0] & mask;
            return self.write_vec_op(instr, 0, [upper | low, 0, 0, 0]);
        }
        let src = self.read_vec_op(instr, last)?[0] & mask;
        let upper = if instr.op_kind(last) == OpKind::Memory {
            0
        } else {
            self.read_vec_op(instr, 0)?[0] & !mask
        };
        self.write_vec_op(instr, 0, [upper | src, 0, 0, 0])
    }

    /// `movlpd`/`movhpd` et leurs cousins : huit octets déplacés dans
    /// **une moitié** du registre, l'autre gardée telle quelle.
    ///
    /// C'est ce qui les distingue de `movsd`, qui écrase le haut quand il
    /// lit la mémoire. Deux d'entre elles mises bout à bout recopient
    /// seize octets sans exiger d'alignement — la recette de tous les
    /// `memcpy` courts du monde.
    fn move_half(&mut self, instr: &Instruction, high: bool) -> Result<(), EmuError> {
        let moitié = if high { 64 } else { 0 };
        let mask: u128 = 0xffff_ffff_ffff_ffff << moitié;
        if instr.op_kind(0) == OpKind::Memory {
            // Vers la mémoire : la moitié choisie, et rien d'autre.
            let src = self.read_vec_op(instr, instr.op_count() - 1)?[0];
            return self.write_vec_op(instr, 0, [(src & mask) >> moitié, 0, 0, 0]);
        }
        let last = instr.op_count() - 1;
        let src = self.read_vec_op(instr, last)?[0] & 0xffff_ffff_ffff_ffff;
        // La forme AVX à trois opérandes prend l'autre moitié de sa
        // première source ; la forme SSE la garde de la destination.
        let reste = if instr.op_count() >= 3 {
            self.read_vec_op(instr, 1)?[0] & !mask
        } else {
            self.read_vec_op(instr, 0)?[0] & !mask
        };
        self.write_vec_op(instr, 0, [reste | (src << moitié), 0, 0, 0])
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
            return self.write_vec_op(instr, 0, [value & mask, 0, 0, 0]);
        }
        let value = (self.read_vec_op(instr, 1)?[0] & mask) as u64;
        if instr.op_kind(0) == OpKind::Memory {
            self.write_vec_op(instr, 0, [u128::from(value), 0, 0, 0])
        } else {
            self.write_op(instr, 0, value)
        }
    }

    /// `pmovzx*`/`pmovsx*` : la source élargie, chaque élément gardant
    /// sa valeur.
    ///
    /// Elle ne passe pas par `vector_binary`, qui traite les deux
    /// moitiés séparément : ici la source fait la moitié de la
    /// destination, et c'est précisément d'une moitié à l'autre que les
    /// éléments traversent.
    fn extend_packed(
        &mut self,
        instr: &Instruction,
        from: usize,
        to: usize,
        signed: bool,
    ) -> Result<(), EmuError> {
        let src = self.read_vec_op(instr, instr.op_count() - 1)?[0];
        if self.is_wide(instr) {
            // La destination fait 256 bits : la source entière y passe,
            // et la part qui nourrit la moitié haute est celle qui se
            // trouve au-delà de 128 / (to / from) bits.
            let coupe = 128 / (to / from);
            let out = [
                extend_lanes(src, from, to, signed),
                extend_lanes(src >> coupe, from, to, signed),
                0,
                0,
            ];
            self.write_vec_op(instr, 0, out)
        } else {
            self.write_vec_op(instr, 0, [extend_lanes(src, from, to, signed), 0, 0, 0])
        }
    }

    /// `pinsr*` : une voie écrite à l'index que donne l'immédiat, le
    /// reste du registre intact. La valeur vient d'un registre général
    /// ou de la mémoire — c'est par là qu'un scalaire entre dans un
    /// vecteur.
    fn insert_element(&mut self, instr: &Instruction, bytes: usize) -> Result<(), EmuError> {
        let last = instr.op_count() - 1;
        let index = self.read_op(instr, last)? as usize % (16 / bytes);
        // AVX nomme la destination à part : `vpinsrw xmm0, xmm1, eax, 2`
        // laisse `xmm1` intact, là où `pinsrw xmm0, eax, 2` écrit dans sa
        // propre source.
        let gardé = self.read_vec_op(instr, if instr.op_count() >= 4 { 1 } else { 0 })?[0];
        let valeur = u128::from(self.read_op(instr, last - 1)? & lane_mask(bytes * 8));
        let shift = index * bytes * 8;
        let trou = u128::from(lane_mask(bytes * 8)) << shift;
        self.write_vec_op(instr, 0, [(gardé & !trou) | (valeur << shift), 0, 0, 0])
    }

    /// `pextr*` : le chemin inverse. Vers un registre général, la voie
    /// est étendue par des zéros, quelle que soit sa largeur d'origine.
    fn extract_element(&mut self, instr: &Instruction, bytes: usize) -> Result<(), EmuError> {
        let last = instr.op_count() - 1;
        let index = self.read_op(instr, last)? as usize % (16 / bytes);
        let src = self.read_vec_op(instr, last - 1)?[0];
        let valeur = (src >> (index * bytes * 8)) as u64 & lane_mask(bytes * 8);
        self.write_op(instr, 0, valeur)
    }

    /// `pcmpistri`, `pcmpistrm`, `pcmpestri`, `pcmpestrm`.
    ///
    /// Une comparaison de seize octets contre seize autres, selon la
    /// question que porte l'immédiat, et la réponse rendue soit en
    /// INDEX (dans ECX) soit en MASQUE (dans XMM0).
    ///
    /// Les quatre indicateurs qu'elles posent valent d'être lus, parce
    /// que c'est sur eux que branche la bibliothèque C :
    ///   - CF : on a trouvé quelque chose ;
    ///   - ZF : la chaîne cherchée dans (`b`) se termine dans ces seize
    ///     octets — autrement dit, on tient sa fin ;
    ///   - SF : celle qu'on cherche (`a`) s'y termine aussi ;
    ///   - OF : le premier élément de la réponse.
    ///
    /// AF et PF sont remis à zéro, toujours.
    fn packed_compare_strings(
        &mut self,
        instr: &Instruction,
        explicite: bool,
        index: bool,
    ) -> Result<(), EmuError> {
        let a = self.read_vec_op(instr, 0)?[0];
        let b = self.read_vec_op(instr, 1)?[0];
        let ctrl = Contrôle::lire(self.read_op(instr, 2)? as u8);
        let n = ctrl.voies();
        // Les longueurs. Explicites, elles viennent d'EAX et EDX, et
        // c'est leur VALEUR ABSOLUE qui compte, bornée au vecteur :
        // une longueur négative ne veut rien dire, et le processeur la
        // lit ainsi.
        let (la, lb) = if explicite {
            let borne = |v: u64| ((v as i32).unsigned_abs() as usize).min(n);
            (
                borne(self.cpu.regs.get64(namsdec_core::Reg64::Rax)),
                borne(self.cpu.regs.get64(namsdec_core::Reg64::Rdx)),
            )
        } else {
            (longueur_implicite(ctrl, a), longueur_implicite(ctrl, b))
        };

        let brut = comparer_chaînes(ctrl, a, la, b, lb);
        let res = polarité(ctrl, brut, lb);

        if index {
            // L'index du premier bit à 1 — ou du dernier, selon le bit
            // 6 —, et `n` quand il n'y en a aucun. C'est ce `n` qui
            // fait sortir des boucles de la glibc : « rien ici, passe
            // aux seize suivants ».
            let trouvé = if res == 0 {
                n as u64
            } else if ctrl.haut {
                (15 - res.leading_zeros()) as u64
            } else {
                res.trailing_zeros() as u64
            };
            self.cpu.regs.set64(namsdec_core::Reg64::Rcx, trouvé);
        } else {
            // Le masque, dans XMM0 — un registre IMPLICITE, que
            // l'instruction n'écrit nulle part et qu'elle écrase quand
            // même. Un bit par élément, ou l'élément entier à 1.
            let masque = if ctrl.haut {
                let large = 128 / n;
                (0..n).fold(0u128, |acc, i| {
                    if res >> i & 1 == 1 {
                        acc | (((1u128 << large) - 1) << (i * large))
                    } else {
                        acc
                    }
                })
            } else {
                u128::from(res)
            };
            self.cpu.regs.set_xmm(namsdec_core::RegXmm::Xmm0, masque);
        }

        let mut flags = RFlags::empty();
        if res != 0 {
            flags |= RFlags::CF;
        }
        if lb < n {
            flags |= RFlags::ZF;
        }
        if la < n {
            flags |= RFlags::SF;
        }
        if res & 1 == 1 {
            flags |= RFlags::OF;
        }
        self.cpu.flags.apply_arith(flags);
        Ok(())
    }

    /// Lit un opérande qui est un registre de masque.
    fn read_mask_op(&self, instr: &Instruction, op: u32) -> Result<u64, EmuError> {
        let reg = instr.op_register(op);
        map_mask(reg)
            .map(|k| self.cpu.regs.get_mask(k))
            .ok_or_else(|| EmuError::UnsupportedRegister {
                backend: Self::NAME,
                register: format!("{reg:?}"),
            })
    }

    /// Écrit un registre de masque.
    fn write_mask_op(&mut self, instr: &Instruction, op: u32, v: u64) -> Result<(), EmuError> {
        let reg = instr.op_register(op);
        let k = map_mask(reg).ok_or_else(|| EmuError::UnsupportedRegister {
            backend: Self::NAME,
            register: format!("{reg:?}"),
        })?;
        self.cpu.regs.set_mask(k, v);
        Ok(())
    }

    /// `kmovb/w/d/q` : déplacer un masque — d'un registre de masque à
    /// l'autre, vers un registre général, ou depuis la mémoire.
    ///
    /// C'est par elle qu'un masque sort du monde d'AVX-512 : une
    /// comparaison écrit `k1`, `kmovd eax, k1` le ramène dans un
    /// registre ordinaire, et le code branche dessus. On la trouve
    /// sept cent trois fois dans les binaires d'épreuve, soit plus que
    /// toute autre instruction AVX-512.
    fn move_mask(&mut self, instr: &Instruction, bits: u32) -> Result<(), EmuError> {
        let masque = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let source = if instr.op_kind(1) == OpKind::Register && instr.op_register(1).is_k() {
            self.read_mask_op(instr, 1)?
        } else {
            // Un registre général ou la mémoire : `read_op` sait lire
            // les deux, à la largeur que l'instruction déclare.
            self.read_op(instr, 1)?
        } & masque;
        if instr.op_kind(0) == OpKind::Register && instr.op_register(0).is_k() {
            self.write_mask_op(instr, 0, source)
        } else {
            self.write_op(instr, 0, source)
        }
    }

    /// Les opérations bit à bit entre deux masques : `kand`, `kor`,
    /// `kxor`, `kandn`, `kxnor`, `knot`, et les décalages.
    ///
    /// Elles servent à combiner des conditions : « les octets qui sont
    /// des lettres ET qui ne sont pas déjà pris ». C'est le même
    /// travail que `pand` sur un vecteur de masques, en huit fois moins
    /// de bits.
    fn mask_binary(
        &mut self,
        instr: &Instruction,
        bits: u32,
        f: impl Fn(u64, u64) -> u64,
    ) -> Result<(), EmuError> {
        let masque = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        // Deux sources nommées, ou une seule pour `knot`.
        let (a, b) = if instr.op_count() >= 3 {
            (self.read_mask_op(instr, 1)?, self.read_mask_op(instr, 2)?)
        } else {
            (self.read_mask_op(instr, 1)?, 0)
        };
        self.write_mask_op(instr, 0, f(a, b) & masque)
    }

    /// Un décalage de masque, dont le compte est un immédiat.
    fn mask_shift(&mut self, instr: &Instruction, bits: u32, gauche: bool) -> Result<(), EmuError> {
        let masque = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let a = self.read_mask_op(instr, 1)?;
        let n = self.read_op(instr, 2)?;
        let out = if n >= 64 {
            0
        } else if gauche {
            a << n
        } else {
            a >> n
        };
        self.write_mask_op(instr, 0, out & masque)
    }

    /// `kortest` et `ktest` : poser les indicateurs sans rien écrire.
    ///
    /// C'est là que le masque redevient une décision. `kortestd k1, k1`
    /// suivi d'un `jz` est la façon dont une boucle AVX-512 demande
    /// « rien trouvé dans ces soixante-quatre octets ? ».
    ///
    /// `kortest` : ZF dit que le OU des deux est nul, CF qu'il est
    /// plein. `ktest` : ZF dit que le ET est nul, CF que le ET-NON
    /// l'est.
    fn mask_test(&mut self, instr: &Instruction, bits: u32, ou: bool) -> Result<(), EmuError> {
        let masque = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let a = self.read_mask_op(instr, 0)? & masque;
        let b = self.read_mask_op(instr, 1)? & masque;
        let mut flags = RFlags::empty();
        let (zéro, retenue) = if ou {
            ((a | b) == 0, (a | b) == masque)
        } else {
            ((a & b) == 0, (b & !a) & masque == 0)
        };
        if zéro {
            flags |= RFlags::ZF;
        }
        if retenue {
            flags |= RFlags::CF;
        }
        self.cpu.flags.apply_arith(flags);
        Ok(())
    }

    /// `vptestm*` et `vptestnm*` : la question de `ptest`, posée voie
    /// par voie, et dont la réponse va dans un registre de masque.
    ///
    /// `vptestmb k1, ymm2, ymm3` pose un bit par octet, là où le ET des
    /// deux n'est pas nul ; la forme `nm` fait l'inverse. C'est
    /// l'instruction qui remplace le couple `pcmpeqb` + `pmovmskb` des
    /// jeux précédents — et l'on en compte quatre cents dans les
    /// binaires d'épreuve.
    fn test_into_mask(
        &mut self,
        instr: &Instruction,
        bytes: usize,
        nul: bool,
    ) -> Result<(), EmuError> {
        let a = self.read_vec_op(instr, 1)?;
        let b = self.read_vec_op(instr, 2)?;
        let large = matches!(instr.op_kind(1), OpKind::Register) && instr.op_register(1).is_ymm();
        let voies = if large { 32 / bytes } else { 16 / bytes };
        let bits = bytes * 8;
        let mut out = 0u64;
        for voie in 0..voies {
            let (moitié, dans) = (voie / (16 / bytes), voie % (16 / bytes));
            let mot = (a[moitié] >> (dans * bits)) & u128::from(lane_mask(bits));
            let autre = (b[moitié] >> (dans * bits)) & u128::from(lane_mask(bits));
            if ((mot & autre) != 0) != nul {
                out |= 1u64 << voie;
            }
        }
        self.write_mask_op(instr, 0, out)
    }

    /// `vpcmp*` : comparer voie par voie selon un PRÉDICAT, et poser un
    /// bit par voie dans un registre de masque.
    ///
    /// Les jeux précédents n'avaient que « égal » et « plus grand », et
    /// l'on composait le reste. AVX-512 porte le prédicat dans un
    /// immédiat : 0 égal, 1 plus petit, 2 plus petit ou égal, 4
    /// différent, 5 pas plus petit, 6 pas plus petit ou égal. Les 3 et
    /// 7 sont « jamais » et « toujours ».
    fn compare_into_mask(
        &mut self,
        instr: &Instruction,
        bytes: usize,
        signé: bool,
    ) -> Result<(), EmuError> {
        let a = self.read_vec_op(instr, 1)?;
        let b = self.read_vec_op(instr, 2)?;
        let prédicat = self.read_op(instr, 3)? & 7;
        let large = matches!(instr.op_kind(1), OpKind::Register) && instr.op_register(1).is_ymm();
        let voies = if large { 32 / bytes } else { 16 / bytes };
        let bits = bytes * 8;
        let Some(w) = width_from_bytes(bytes) else {
            return Ok(());
        };
        let mut out = 0u64;
        for voie in 0..voies {
            let (moitié, dans) = (voie / (16 / bytes), voie % (16 / bytes));
            let x = ((a[moitié] >> (dans * bits)) as u64) & lane_mask(bits);
            let y = ((b[moitié] >> (dans * bits)) as u64) & lane_mask(bits);
            let ordre = if signé {
                (sign_extend(x, w) as i64).cmp(&(sign_extend(y, w) as i64))
            } else {
                x.cmp(&y)
            };
            let vrai = match prédicat {
                0 => ordre == Ordering::Equal,
                1 => ordre == Ordering::Less,
                2 => ordre != Ordering::Greater,
                3 => false,
                4 => ordre != Ordering::Equal,
                5 => ordre != Ordering::Less,
                6 => ordre == Ordering::Greater,
                _ => true,
            };
            if vrai {
                out |= 1u64 << voie;
            }
        }
        self.write_mask_op(instr, 0, out)
    }

    /// `ptest` : deux questions posées d'un coup sur 128 ou 256 bits.
    ///
    /// ZF dit que la seconde source n'a aucun bit en commun avec la
    /// première ; CF, qu'elle n'en a aucun *hors* de la première. Après
    /// une comparaison vectorielle, c'est ainsi qu'on demande « aucune
    /// voie ne correspond » et « toutes correspondent » sans jamais
    /// ramener le vecteur dans un registre général.
    fn packed_test(&mut self, instr: &Instruction) -> Result<(), EmuError> {
        let masque = self.read_vec_op(instr, 0)?;
        let valeur = self.read_vec_op(instr, 1)?;
        let large = matches!(instr.op_kind(0), OpKind::Register) && instr.op_register(0).is_ymm();
        let moitiés = if large { 2 } else { 1 };
        let mut flags = RFlags::empty();
        if (0..moitiés).all(|i| valeur[i] & masque[i] == 0) {
            flags |= RFlags::ZF;
        }
        if (0..moitiés).all(|i| valeur[i] & !masque[i] == 0) {
            flags |= RFlags::CF;
        }
        self.cpu.flags.apply_arith(flags);
        Ok(())
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
        let tranches = self.tranches(instr);
        let mut out = [0u128; 4];
        for (i, place) in out.iter_mut().enumerate().take(tranches) {
            *place = shift_lanes(a[i], count, bytes, kind);
        }
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
