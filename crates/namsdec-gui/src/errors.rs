//! Les erreurs des crates métier, rendues dans la langue de l'utilisateur.
//!
//! Sous l'interface, aucune crate ne connaît la langue choisie : leurs
//! erreurs portent un `Display` anglais — celui qui part dans une trace ou
//! un rapport de bug — et gardent leurs valeurs dans des variantes
//! structurées. C'est ici, et seulement ici, que ces variantes deviennent
//! une phrase française ou anglaise.
//!
//! Ces messages vivent à part de `i18n.rs`, qui ne tient que des chaînes
//! *statiques* vérifiées par le compilateur. Ceux-là interpolent tous
//! quelque chose (une adresse, un registre, une raison système) : leur
//! place est dans une fonction qui reçoit l'erreur, pas dans une table.

use namsdec_asm::AsmError;
use namsdec_core::MemError;
use namsdec_emu::{EmuError, OperandRole};

use crate::i18n::Lang;

/// Le message affiché à l'utilisateur pour une erreur d'exécution.
pub fn emu(lang: Lang, error: &EmuError) -> String {
    let fr = lang == Lang::Fr;
    match error {
        // « Non supporté » appelle immédiatement « par quoi ? » : le nom du
        // moteur répond dans la phrase même, d'autant qu'un autre backend
        // branché derrière `namsdec_emu::Backend` peut connaître cette
        // instruction-là.
        EmuError::UnsupportedInstruction { backend, mnemonic } => {
            if fr {
                format!("l'interpréteur {backend} ne gère pas l'instruction {mnemonic}")
            } else {
                format!("the {backend} interpreter does not support the {mnemonic} instruction")
            }
        }
        EmuError::UnsupportedRegister { backend, register } => {
            if fr {
                format!("l'interpréteur {backend} ne gère pas le registre {register}")
            } else {
                format!("the {backend} interpreter does not support the {register} register")
            }
        }
        EmuError::UnsupportedOperand {
            backend,
            role,
            kind,
        } => {
            let role = operand_role(lang, *role);
            if fr {
                format!("l'interpréteur {backend} ne gère pas cette opérande {role} ({kind})")
            } else {
                format!("the {backend} interpreter does not support this {role} operand ({kind})")
            }
        }
        EmuError::UnsupportedOperandSize { backend, bytes } => {
            if fr {
                format!(
                    "l'interpréteur {backend} ne gère pas les opérandes mémoire de {bytes} octets"
                )
            } else {
                format!("the {backend} interpreter does not support {bytes}-byte memory operands")
            }
        }
        // Le numéro brut plutôt qu'un nom : c'est lui que l'utilisateur a
        // mis dans RAX, et c'est sous ce numéro qu'il le retrouvera dans
        // la table des appels système.
        EmuError::UnsupportedSyscall { backend, number } => {
            if fr {
                format!("l'interpréteur {backend} ne simule pas l'appel système {number}")
            } else {
                format!("the {backend} interpreter does not simulate syscall {number}")
            }
        }
        EmuError::Mem(e) => mem(lang, e),
        EmuError::InvalidInstruction { addr } => {
            if fr {
                format!("instruction indécodable à l'adresse {addr:#x}")
            } else {
                format!("could not decode an instruction at {addr:#x}")
            }
        }
    }
}

fn operand_role(lang: Lang, role: OperandRole) -> &'static str {
    match (role, lang) {
        (OperandRole::Destination, Lang::Fr) => "de destination",
        (OperandRole::Destination, Lang::En) => "destination",
        (OperandRole::Read, Lang::Fr) => "en lecture",
        (OperandRole::Read, Lang::En) => "read",
        (OperandRole::Write, Lang::Fr) => "en écriture",
        (OperandRole::Write, Lang::En) => "write",
    }
}

pub fn mem(lang: Lang, error: &MemError) -> String {
    match error {
        MemError::OutOfBounds { addr } => {
            if lang == Lang::Fr {
                format!("accès mémoire hors de la fenêtre simulée, à l'adresse {addr:#x}")
            } else {
                format!("memory access outside the simulated window, at {addr:#x}")
            }
        }
    }
}

/// Les erreurs d'assemblage dont le message *est* le contenu affiché.
///
/// `Failed` n'y figure pas volontairement : ses diagnostics sont montrés
/// un par un sous l'éditeur, reliés à leur ligne — un message global les
/// résumerait moins bien.
pub fn asm(lang: Lang, error: &AsmError) -> String {
    let fr = lang == Lang::Fr;
    match error {
        AsmError::NasmNotFound => {
            if fr {
                "nasm est introuvable dans le PATH (ex : `sudo dnf install nasm`).".to_owned()
            } else {
                "nasm was not found in PATH (e.g.: `sudo dnf install nasm`).".to_owned()
            }
        }
        AsmError::Failed { diagnostics } => {
            if fr {
                format!(
                    "échec de l'assemblage — {} diagnostic(s)",
                    diagnostics.len()
                )
            } else {
                format!("assembly failed — {} diagnostic(s)", diagnostics.len())
            }
        }
        AsmError::LinkerNotFound => {
            if fr {
                "ld est introuvable dans le PATH (ex : `sudo dnf install binutils`).".to_owned()
            } else {
                "ld was not found in PATH (e.g.: `sudo dnf install binutils`).".to_owned()
            }
        }
        // Le message de `ld` est repris tel quel : il nomme le symbole
        // manquant, ce qu'aucune reformulation ne ferait mieux.
        AsmError::LinkFailed { message } => {
            if fr {
                format!("échec de l'édition de liens : {message}")
            } else {
                format!("linking failed: {message}")
            }
        }
        AsmError::UnreadableProgram => {
            if fr {
                "le programme lié n'a pas pu être relu (ELF inattendu).".to_owned()
            } else {
                "the linked program could not be read back (unexpected ELF).".to_owned()
            }
        }
        AsmError::Io(e) => {
            if fr {
                format!("erreur d'entrée/sortie : {e}")
            } else {
                format!("I/O error: {e}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le nom du moteur doit apparaître dans le message : c'est la seule
    /// chose qui dit à l'utilisateur *qui* refuse l'instruction, et donc
    /// qu'un autre moteur pourrait l'accepter.
    #[test]
    fn an_unsupported_instruction_names_the_backend_in_both_languages() {
        let error = EmuError::UnsupportedInstruction {
            backend: "namsdec",
            mnemonic: "Cpuid".to_owned(),
        };

        let fr = emu(Lang::Fr, &error);
        assert!(fr.contains("namsdec") && fr.contains("Cpuid"), "{fr}");
        let en = emu(Lang::En, &error);
        assert!(en.contains("namsdec") && en.contains("Cpuid"), "{en}");
        assert_ne!(fr, en, "les deux langues doivent donner deux phrases");
    }

    /// Le `Display` des crates métier reste anglais quoi qu'il arrive :
    /// c'est lui qui part dans les traces, indépendamment de la langue
    /// choisie dans l'interface.
    #[test]
    fn the_underlying_display_stays_english() {
        let error = MemError::OutOfBounds { addr: 0x4000 };
        assert_eq!(error.to_string(), "memory access out of bounds at 0x4000");
        assert!(mem(Lang::Fr, &error).contains("hors de la fenêtre simulée"));
    }
}
