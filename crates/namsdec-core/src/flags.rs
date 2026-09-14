bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct RFlags: u64 {
        const CF = 1 << 0;
        const PF = 1 << 2;
        const AF = 1 << 4;
        const ZF = 1 << 6;
        const SF = 1 << 7;
        const TF = 1 << 8;
        const IF = 1 << 9;
        const DF = 1 << 10;
        const OF = 1 << 11;
    }
}

/// Les six indicateurs recalculés par les instructions arithmétiques et
/// logiques (TF/IF/DF ne sont jamais touchés par add/sub/cmp/and/or/xor...).
const ARITH_FLAGS: RFlags = RFlags::CF
    .union(RFlags::PF)
    .union(RFlags::AF)
    .union(RFlags::ZF)
    .union(RFlags::SF)
    .union(RFlags::OF);

impl RFlags {
    /// Remplace CF/PF/AF/ZF/SF/OF par les valeurs fraîchement calculées,
    /// sans toucher TF/IF/DF.
    pub fn apply_arith(&mut self, computed: RFlags) {
        self.remove(ARITH_FLAGS);
        self.insert(computed);
    }
}

/// Largeur d'une opération arithmétique/logique, pour savoir où se
/// trouvent le bit de signe et le bit de retenue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Width {
    W8,
    W16,
    W32,
    W64,
}

impl Width {
    fn bits(self) -> u32 {
        match self {
            Width::W8 => 8,
            Width::W16 => 16,
            Width::W32 => 32,
            Width::W64 => 64,
        }
    }

    fn mask(self) -> u64 {
        if self.bits() == 64 {
            u64::MAX
        } else {
            (1u64 << self.bits()) - 1
        }
    }

    fn sign_bit(self) -> u64 {
        1u64 << (self.bits() - 1)
    }
}

fn parity_even(result: u64) -> bool {
    (result as u8).count_ones().is_multiple_of(2)
}

fn common_flags(result: u64, width: Width) -> RFlags {
    let mut flags = RFlags::empty();
    flags.set(RFlags::ZF, result & width.mask() == 0);
    flags.set(RFlags::SF, result & width.sign_bit() != 0);
    flags.set(RFlags::PF, parity_even(result));
    flags
}

/// `a + b` tronqué à `width`, avec les indicateurs qui en résultent.
pub fn add_with_flags(a: u64, b: u64, width: Width) -> (u64, RFlags) {
    let mask = width.mask();
    let a = a & mask;
    let b = b & mask;
    let full = a as u128 + b as u128;
    let result = (full as u64) & mask;

    let mut flags = common_flags(result, width);
    flags.set(RFlags::CF, full > mask as u128);
    flags.set(RFlags::AF, (a ^ b ^ result) & 0x10 != 0);

    let sign_a = a & width.sign_bit() != 0;
    let sign_b = b & width.sign_bit() != 0;
    let sign_r = result & width.sign_bit() != 0;
    flags.set(RFlags::OF, sign_a == sign_b && sign_r != sign_a);

    (result, flags)
}

/// `a - b` tronqué à `width`, avec les indicateurs qui en résultent
/// (utilisé aussi pour `cmp`, qui calcule les mêmes indicateurs sans
/// écrire le résultat).
pub fn sub_with_flags(a: u64, b: u64, width: Width) -> (u64, RFlags) {
    let mask = width.mask();
    let a = a & mask;
    let b = b & mask;
    let result = a.wrapping_sub(b) & mask;

    let mut flags = common_flags(result, width);
    flags.set(RFlags::CF, a < b);
    flags.set(RFlags::AF, (a ^ b ^ result) & 0x10 != 0);

    let sign_a = a & width.sign_bit() != 0;
    let sign_b = b & width.sign_bit() != 0;
    let sign_r = result & width.sign_bit() != 0;
    flags.set(RFlags::OF, sign_a != sign_b && sign_r != sign_a);

    (result, flags)
}

/// Indicateurs pour un résultat logique (`and`/`or`/`xor`/`test`) :
/// CF et OF sont toujours à zéro, AF est indéfini côté matériel et donc
/// mis à zéro ici par simplification pédagogique.
pub fn logic_flags(result: u64, width: Width) -> RFlags {
    let mut flags = common_flags(result, width);
    flags.remove(RFlags::CF | RFlags::OF | RFlags::AF);
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sets_zero_flag() {
        let (result, flags) = add_with_flags(1, u64::MAX, Width::W8); // 1 + 0xFF (=0xFF) tronqué 8 bits -> 0
        assert_eq!(result, 0);
        assert!(flags.contains(RFlags::ZF));
        assert!(flags.contains(RFlags::CF));
    }

    #[test]
    fn add_signed_overflow_32bit() {
        // 0x7FFFFFFF + 1 : dépassement de capacité signée sur 32 bits.
        let (result, flags) = add_with_flags(0x7FFF_FFFF, 1, Width::W32);
        assert_eq!(result, 0x8000_0000);
        assert!(flags.contains(RFlags::OF));
        assert!(flags.contains(RFlags::SF));
        assert!(!flags.contains(RFlags::CF));
        assert!(!flags.contains(RFlags::ZF));
    }

    #[test]
    fn add_unsigned_carry_8bit() {
        let (result, flags) = add_with_flags(0xFF, 0x02, Width::W8);
        assert_eq!(result, 0x01);
        assert!(flags.contains(RFlags::CF));
        assert!(!flags.contains(RFlags::OF));
    }

    #[test]
    fn sub_sets_carry_on_borrow() {
        let (result, flags) = sub_with_flags(0, 1, Width::W8);
        assert_eq!(result, 0xFF);
        assert!(flags.contains(RFlags::CF));
        assert!(flags.contains(RFlags::SF));
        assert!(!flags.contains(RFlags::ZF));
    }

    #[test]
    fn sub_equal_operands_sets_zero_flag() {
        let (result, flags) = sub_with_flags(5, 5, Width::W64);
        assert_eq!(result, 0);
        assert!(flags.contains(RFlags::ZF));
        assert!(!flags.contains(RFlags::CF));
        assert!(!flags.contains(RFlags::SF));
    }

    #[test]
    fn logic_flags_clear_carry_and_overflow() {
        let flags = logic_flags(0x0F, Width::W8);
        assert!(!flags.contains(RFlags::CF));
        assert!(!flags.contains(RFlags::OF));
        assert!(flags.contains(RFlags::PF)); // 0x0F = 4 bits à 1 -> parité paire
    }

    #[test]
    fn apply_arith_preserves_direction_and_interrupt_flags() {
        let mut flags = RFlags::DF | RFlags::IF;
        let (_, computed) = add_with_flags(1, 1, Width::W8);
        flags.apply_arith(computed);
        assert!(flags.contains(RFlags::DF));
        assert!(flags.contains(RFlags::IF));
    }
}
