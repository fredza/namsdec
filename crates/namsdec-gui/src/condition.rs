//! « Arrête-toi là, mais seulement si… »
//!
//! Un point d'arrêt dans une boucle de mille tours s'atteint mille fois,
//! et l'on cherche le tour où quelque chose va de travers. Sans
//! condition, il ne reste qu'à cliquer neuf cent quatre-vingt-dix-neuf
//! fois sur « Exécuter » — ce qui fait passer le débogage pour une
//! punition.
//!
//! La grammaire est volontairement minuscule : une valeur, une
//! comparaison, une valeur. Elle se lit sans documentation (`rcx == 0`,
//! `rax > 0x100`), et ce qu'elle refuse est refusé clairement plutôt
//! qu'interprété de travers.

use namsdec_core::Registers;

use crate::sidebar::eval_address;

/// Ce que deux valeurs peuvent se dire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

/// Une condition d'arrêt, prête à être évaluée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    left: String,
    comparison: Comparison,
    right: String,
}

impl Condition {
    /// La condition tient-elle, avec ces registres ?
    ///
    /// Une condition qu'on ne peut pas évaluer — un registre mal
    /// orthographié — rend `false` : on ne s'arrête pas sur ce qu'on ne
    /// comprend pas, et le champ dira que l'expression est refusée.
    pub fn holds(&self, regs: &Registers) -> bool {
        let (Some(left), Some(right)) = (
            eval_address(&self.left, Some(regs)),
            eval_address(&self.right, Some(regs)),
        ) else {
            return false;
        };
        match self.comparison {
            Comparison::Equal => left == right,
            Comparison::NotEqual => left != right,
            // Non signé : une adresse et un compteur sont des nombres
            // sans signe, et c'est ce qu'on compare ici neuf fois sur
            // dix. Pour le signé, on écrit la condition autrement.
            Comparison::Less => left < right,
            Comparison::LessOrEqual => left <= right,
            Comparison::Greater => left > right,
            Comparison::GreaterOrEqual => left >= right,
        }
    }
}

/// Lit une condition. Rend `None` sur ce qui n'en est pas une.
///
/// Les deux côtés sont des expressions d'adresse — un registre, un
/// nombre, une somme —, les mêmes que celles du champ de l'onglet
/// Mémoire : une grammaire de plus à apprendre serait une grammaire de
/// trop.
pub fn parse(text: &str) -> Option<Condition> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    // Les opérateurs à deux caractères d'abord : sinon `<=` se lirait
    // comme `<` suivi d'un `=` égaré.
    for (token, comparison) in [
        ("==", Comparison::Equal),
        ("!=", Comparison::NotEqual),
        ("<=", Comparison::LessOrEqual),
        (">=", Comparison::GreaterOrEqual),
        ("=", Comparison::Equal),
        ("<", Comparison::Less),
        (">", Comparison::Greater),
    ] {
        let Some((left, right)) = text.split_once(token) else {
            continue;
        };
        let (left, right) = (left.trim(), right.trim());
        if left.is_empty() || right.is_empty() {
            return None;
        }
        // Les deux côtés doivent se lire — sinon la condition serait
        // toujours fausse, sans qu'on sache pourquoi.
        eval_address(left, None).or_else(|| eval_address(left, Some(&Registers::default())))?;
        eval_address(right, None).or_else(|| eval_address(right, Some(&Registers::default())))?;
        return Some(Condition {
            left: left.to_owned(),
            comparison,
            right: right.to_owned(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::Reg64;

    fn regs(rcx: u64, rax: u64) -> Registers {
        let mut regs = Registers::default();
        regs.set64(Reg64::Rcx, rcx);
        regs.set64(Reg64::Rax, rax);
        regs
    }

    /// La condition qu'on écrit pour trouver le dernier tour d'une
    /// boucle.
    #[test]
    fn the_last_turn_of_a_loop_can_be_caught() {
        let condition = parse("rcx == 0").expect("une condition");
        assert!(condition.holds(&regs(0, 0)));
        assert!(!condition.holds(&regs(5, 0)));
    }

    /// Les six comparaisons, et les deux façons d'écrire l'égalité.
    #[test]
    fn every_comparison_reads() {
        let cas = [
            ("rax == 3", true),
            ("rax = 3", true),
            ("rax != 3", false),
            ("rax < 4", true),
            ("rax <= 3", true),
            ("rax > 2", true),
            ("rax >= 3", true),
            ("rax > 3", false),
        ];
        for (texte, attendu) in cas {
            let condition = parse(texte).unwrap_or_else(|| panic!("« {texte} » doit se lire"));
            assert_eq!(condition.holds(&regs(0, 3)), attendu, "{texte}");
        }
    }

    /// Les deux côtés sont des expressions, pas seulement des registres :
    /// on compare aussi une somme à une adresse.
    #[test]
    fn both_sides_are_expressions() {
        let condition = parse("rax + 2 == 0x10").expect("une condition");
        assert!(condition.holds(&regs(0, 0xe)));
        assert!(!condition.holds(&regs(0, 0xf)));
    }

    /// Ce qui n'est pas une condition est refusé, plutôt qu'interprété
    /// de travers : une condition toujours fausse ferait croire à un
    /// point d'arrêt cassé.
    #[test]
    fn what_is_not_a_condition_is_refused() {
        assert!(parse("").is_none());
        assert!(parse("rcx").is_none(), "il manque la comparaison");
        assert!(parse("rcx ==").is_none());
        assert!(parse("== 0").is_none());
        assert!(parse("rxc == 0").is_none(), "registre mal orthographié");
        assert!(parse("rcx == bonjour").is_none());
    }
}
