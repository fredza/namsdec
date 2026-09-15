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
        // Les deux fautes de division. Le message dit *pourquoi* le
        // processeur s'arrête là : c'est une exception matérielle (#DE),
        // pas une limite de l'interpréteur.
        EmuError::DivideByZero { addr } => {
            if fr {
                format!("division par zéro à l'adresse {addr:#x}")
            } else {
                format!("division by zero at {addr:#x}")
            }
        }
        EmuError::DivideOverflow { addr } => {
            if fr {
                format!(
                    "le quotient ne tient pas dans le registre, à l'adresse {addr:#x} \
                     (pensez à mettre à zéro la moitié haute du dividende)"
                )
            } else {
                format!(
                    "the quotient does not fit in its register, at {addr:#x} \
                     (remember to clear the high half of the dividend)"
                )
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

/// Les refus à l'installation d'un thème. Ils doivent dire quoi corriger
/// dans le fichier : c'est tout ce que l'utilisateur peut faire.
pub fn plugin(lang: Lang, error: &namsdec_theme::PluginError) -> String {
    use namsdec_theme::PluginError as E;
    let fr = lang == Lang::Fr;
    match error {
        E::Io(e) => {
            if fr {
                format!("fichier illisible : {e}")
            } else {
                format!("cannot read the file: {e}")
            }
        }
        E::Format(e) => {
            if fr {
                format!("ce n'est pas un fichier de thème valide : {e}")
            } else {
                format!("this is not a valid theme file: {e}")
            }
        }
        E::Colour { field, value } => {
            if fr {
                format!("« {field} » n'est pas une couleur comme « #1e1e1e » : {value}")
            } else {
                format!("\"{field}\" is not a colour like \"#1e1e1e\": {value}")
            }
        }
        E::DuplicateId(id) => {
            if fr {
                format!("l'identifiant « {id} » est déjà pris par un autre thème")
            } else {
                format!("the identifier \"{id}\" is already taken by another theme")
            }
        }
        E::BadId(id) => {
            if fr {
                format!("l'identifiant ne peut contenir que lettres, chiffres et tirets : « {id} »")
            } else {
                format!("the identifier may only contain letters, digits and dashes: \"{id}\"")
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
