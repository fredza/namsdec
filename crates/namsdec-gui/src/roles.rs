//! À quoi sert chaque registre, selon la convention.
//!
//! Le processeur, lui, n'en sait rien : RBX et R12 lui sont interchangeables.
//! Tout ce qui les distingue est un accord entre les programmes — l'ABI
//! System V — et c'est un accord qu'on ne devine pas en lisant le code.
//! Il faut l'avoir appris, ou l'avoir sous la main.
//!
//! Deux conventions se superposent, et c'est là qu'on se trompe : celle
//! des appels de fonction et celle des appels système. Le quatrième
//! argument passe par RCX dans l'une et par R10 dans l'autre, parce que
//! `syscall` écrase RCX. Les deux rôles sont donc dits ensemble.

use crate::i18n::Lang;

/// Le rôle conventionnel d'un registre, ou une chaîne vide s'il n'en a
/// pas — auquel cas il est libre, ce qui est aussi une information.
pub fn of(lang: Lang, name: &str) -> &'static str {
    let fr = lang == Lang::Fr;
    match name {
        "RAX" if fr => "Valeur de retour d'une fonction ; numéro de l'appel système.",
        "RAX" => "A function's return value; the system call number.",
        "RBX" if fr => "Libre, mais à rendre intact : l'appelant compte dessus.",
        "RBX" => "Free to use, but must be given back unchanged: the caller relies on it.",
        "RCX" if fr => "4e argument d'une fonction — mais `syscall` l'écrase.",
        "RCX" => "A function's 4th argument — but `syscall` clobbers it.",
        "RDX" if fr => "3e argument, d'une fonction comme d'un appel système.",
        "RDX" => "3rd argument, of a function as well as of a system call.",
        "RSI" if fr => "2e argument, d'une fonction comme d'un appel système.",
        "RSI" => "2nd argument, of a function as well as of a system call.",
        "RDI" if fr => "1er argument, d'une fonction comme d'un appel système.",
        "RDI" => "1st argument, of a function as well as of a system call.",
        "RSP" if fr => "Sommet de la pile. `push`, `pop` et `call` le déplacent pour vous.",
        "RSP" => "Top of the stack. `push`, `pop` and `call` move it for you.",
        "RBP" if fr => "Base du cadre d'appel, quand on en tient un ; à rendre intact.",
        "RBP" => "Base of the call frame, when one is kept; must be given back unchanged.",
        "R8" if fr => "5e argument, d'une fonction comme d'un appel système.",
        "R8" => "5th argument, of a function as well as of a system call.",
        "R9" if fr => "6e argument, d'une fonction comme d'un appel système.",
        "R9" => "6th argument, of a function as well as of a system call.",
        "R10" if fr => "4e argument d'un appel système — là où une fonction prend RCX.",
        "R10" => "A system call's 4th argument — where a function takes RCX.",
        "R11" if fr => "Libre ; `syscall` s'en sert pour garder les indicateurs.",
        "R11" => "Free to use; `syscall` uses it to keep the flags.",
        "R12" | "R13" | "R14" | "R15" if fr => {
            "Libre, mais à rendre intact : l'appelant compte dessus."
        }
        "R12" | "R13" | "R14" | "R15" => {
            "Free to use, but must be given back unchanged: the caller relies on it."
        }
        "RIP" if fr => "L'instruction en cours. Seuls les sauts et les appels l'écrivent.",
        "RIP" => "The current instruction. Only jumps and calls write to it.",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux conventions sont dites là où elles divergent : c'est la
    /// seule chose qui distingue RCX de R10.
    #[test]
    fn the_two_calling_conventions_are_told_apart() {
        assert!(of(Lang::Fr, "RCX").contains("syscall"));
        assert!(of(Lang::Fr, "R10").contains("appel système"));
        assert!(of(Lang::En, "R10").contains("system call"));
        assert_ne!(of(Lang::Fr, "RCX"), of(Lang::Fr, "R10"));
    }

    /// Ceux qu'une sous-routine doit rendre intacts le disent tous, y
    /// compris les quatre qui se ressemblent.
    #[test]
    fn the_callee_saved_ones_say_so() {
        for name in ["RBX", "R12", "R13", "R14", "R15"] {
            assert!(
                of(Lang::Fr, name).contains("rendre intact"),
                "{name} est à rendre intact"
            );
        }
        assert!(!of(Lang::Fr, "RAX").contains("rendre intact"));
    }

    /// Chaque registre du bandeau a son rôle, dans les deux langues :
    /// un trou se verrait à l'écran comme une infobulle vide.
    #[test]
    fn every_register_of_the_banner_has_a_role() {
        for (name, _) in crate::REGISTERS
            .iter()
            .chain(&[("RIP", namsdec_core::Reg64::Rax)])
        {
            for lang in [Lang::Fr, Lang::En] {
                assert!(!of(lang, name).is_empty(), "{name} n'a pas de rôle");
            }
        }
    }
}
