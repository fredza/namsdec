//! « Arrête-toi là, mais seulement si… »
//!
//! Un point d'arrêt dans une boucle de mille tours s'atteint mille fois,
//! et l'on cherche le tour où quelque chose va de travers. Sans
//! condition, il ne reste qu'à cliquer neuf cent quatre-vingt-dix-neuf
//! fois sur « Exécuter » — ce qui fait passer le débogage pour une
//! punition.
//!
//! La forme est minuscule : une valeur, une comparaison, une valeur.
//! Elle se lit sans documentation (`rcx == 0`, `rax > 0x100`), et ce
//! qu'elle refuse est refusé clairement plutôt qu'interprété de travers.
//!
//! Les deux côtés sont des expressions de la **calculatrice** : les
//! mêmes qu'on tape dans Outils › Calculatrice, avec les parenthèses,
//! les opérateurs binaires, les dix-huit fonctions, et la lecture
//! mémoire entre crochets. `(rax & 0xff) == 0x41` et `[rsp+8] != 0`
//! s'écrivent donc tels quels. Une grammaire de plus à apprendre serait
//! une grammaire de trop, et c'est aussi pour cela qu'elle est celle de
//! NASM là où les deux se recouvrent — `byte [rdi]`, `0x41`, `'A'`.

use namsdec_core::Registers;

use crate::calc;

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
    /// La condition tient-elle, avec ces registres et cette mémoire ?
    ///
    /// Une condition qu'on ne peut pas évaluer — un registre mal
    /// orthographié, une adresse hors des pages du programme — rend
    /// `false` : on ne s'arrête pas sur ce qu'on ne comprend pas, et le
    /// champ dira que l'expression est refusée.
    ///
    /// `mem` peut manquer : la condition ne lit alors la mémoire de
    /// personne, et toute lecture entre crochets la rend fausse.
    pub fn holds(&self, regs: &Registers, mem: Option<&dyn calc::Memory>) -> bool {
        let (Ok(left), Ok(right)) = (
            calc::eval_with(&self.left, Some(regs), mem),
            calc::eval_with(&self.right, Some(regs), mem),
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

    /// Ce qu'on compare, tel qu'il est écrit.
    ///
    /// Utile pour *dire* pourquoi une condition ne tient pas : « rax »
    /// se relit à l'écran à côté de la valeur qu'il porte.
    pub fn left(&self) -> &str {
        &self.left
    }

    /// Ce que les deux côtés valent en ce moment.
    ///
    /// `None` quand l'un des deux ne se calcule pas encore — un registre
    /// sans programme, une adresse hors des pages chargées. C'est un cas
    /// normal, et non une faute : on le dit, on ne l'invente pas.
    pub fn sides(&self, regs: &Registers, mem: Option<&dyn calc::Memory>) -> Option<(u64, u64)> {
        let left = calc::eval_with(&self.left, Some(regs), mem).ok()?;
        let right = calc::eval_with(&self.right, Some(regs), mem).ok()?;
        Some((left, right))
    }
}

/// Lit une condition. Rend `None` sur ce qui n'en est pas une.
///
/// Ce qui est refusé, ce sont les expressions **mal écrites** — un mot
/// inconnu, une parenthèse en trop. Pas celles qu'on ne sait pas encore
/// calculer : `[rsp+8] != 0` s'écrit avant de lancer le programme, et
/// c'est même le seul moment où l'on écrit une condition d'arrêt. Voir
/// `CalcError::is_syntax`.
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
        bien_écrit(left)?;
        bien_écrit(right)?;
        return Some(Condition {
            left: left.to_owned(),
            comparison,
            right: right.to_owned(),
        });
    }
    None
}

/// Ce côté-là de la comparaison s'écrit-il ?
///
/// Évalué à blanc, avec des registres à zéro et sans mémoire : ce qui
/// échoue pour une faute de frappe est refusé, ce qui échoue seulement
/// faute de programme est accepté.
fn bien_écrit(expr: &str) -> Option<()> {
    match calc::eval_with(expr, Some(&Registers::default()), None) {
        Ok(_) => Some(()),
        Err(e) if e.is_syntax() => None,
        Err(_) => Some(()),
    }
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

    /// Une mémoire d'essai : une tranche d'octets posée à une adresse.
    struct Tranche {
        base: u64,
        octets: Vec<u8>,
    }

    impl calc::Memory for Tranche {
        fn peek(&self, addr: u64, taille: usize) -> Option<u64> {
            let début = addr.checked_sub(self.base)? as usize;
            let morceau = self.octets.get(début..début + taille)?;
            Some(
                morceau
                    .iter()
                    .enumerate()
                    .fold(0u64, |acc, (rang, o)| acc | u64::from(*o) << (8 * rang)),
            )
        }
    }

    /// Une condition sur ce que la mémoire contient.
    ///
    /// C'est celle qu'on veut devant un contrôle de mot de passe :
    /// l'intéressant n'est pas dans un registre, il est à l'adresse
    /// qu'un registre désigne.
    #[test]
    fn a_condition_can_look_into_memory() {
        let mut regs = Registers::default();
        regs.set64(Reg64::Rsp, 0x1000);
        let mem = Tranche {
            base: 0x1000,
            octets: vec![0, 0, 0, 0, 0, 0, 0, 0, 0x41, 0, 0, 0, 0, 0, 0, 0],
        };
        let mem = Some(&mem as &dyn calc::Memory);

        let condition = parse("[rsp+8] != 0").expect("une condition");
        assert!(condition.holds(&regs, mem));
        assert!(
            !condition.holds(&regs, None),
            "sans mémoire, on ne s'arrête pas sur ce qu'on ne peut pas lire"
        );

        let condition = parse("byte [rsp + 8] == 'A'").expect("une condition");
        assert!(condition.holds(&regs, mem));
    }

    /// Toute la grammaire de la calculatrice est disponible.
    ///
    /// Les deux côtés en sont des expressions : parenthèses, opérateurs
    /// binaires, fonctions. Une grammaire de plus à apprendre serait une
    /// grammaire de trop.
    #[test]
    fn both_sides_are_calculator_expressions() {
        let mut regs = Registers::default();
        regs.set64(Reg64::Rax, 0x1234_5641);
        assert!(parse("(rax & 0xff) == 'A'").unwrap().holds(&regs, None));
        assert!(parse("rax >> 24 == 0x12").unwrap().holds(&regs, None));
        assert!(
            parse("bswap32(rax) == 0x41563412")
                .unwrap()
                .holds(&regs, None)
        );
    }

    /// Une condition s'écrit **avant** de lancer le programme.
    ///
    /// C'est même le seul moment où l'on en écrit une. Refuser le champ
    /// parce qu'il n'y a encore ni registre ni mémoire à lire
    /// reviendrait à interdire la fonction au moment où elle sert.
    #[test]
    fn a_condition_is_written_before_the_program_runs() {
        for texte in [
            "rcx == 0",
            "[rsp+8] != 0",
            "byte [rdi] == 'A'",
            "(rax & 0xff) == 0x41",
        ] {
            assert!(parse(texte).is_some(), "« {texte} » doit être accepté");
        }
        // Mal écrit, en revanche, reste refusé.
        for texte in ["totomatic == 0", "rax == ", "rax == (1", "rax == 1 2"] {
            assert!(parse(texte).is_none(), "« {texte} » doit être refusé");
        }
    }

    /// La condition qu'on écrit pour trouver le dernier tour d'une
    /// boucle.
    #[test]
    fn the_last_turn_of_a_loop_can_be_caught() {
        let condition = parse("rcx == 0").expect("une condition");
        assert!(condition.holds(&regs(0, 0), None));
        assert!(!condition.holds(&regs(5, 0), None));
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
            assert_eq!(condition.holds(&regs(0, 3), None), attendu, "{texte}");
        }
    }

    /// Les deux côtés sont des expressions, pas seulement des registres :
    /// on compare aussi une somme à une adresse.
    #[test]
    fn both_sides_are_expressions() {
        let condition = parse("rax + 2 == 0x10").expect("une condition");
        assert!(condition.holds(&regs(0, 0xe), None));
        assert!(!condition.holds(&regs(0, 0xf), None));
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
