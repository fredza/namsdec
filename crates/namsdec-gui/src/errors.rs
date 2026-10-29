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
use namsdec_core::{Access, MemError};
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
        // Un appel qui sort du programme : ce n'est pas une instruction
        // manquante, c'est une fonction qui vit dans une bibliothèque.
        EmuError::UnsupportedLibrary { name, .. } => {
            if fr {
                format!("namsdec ne fournit pas la fonction de bibliothèque « {name} »")
            } else {
                format!("namsdec does not provide the {name} library function")
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
        // Le numéro d'abord : c'est lui que l'utilisateur a mis dans RAX,
        // et c'est sous ce numéro qu'il le retrouvera dans la table des
        // appels système. Mais le numéro SEUL n'apprend rien — « l'appel
        // système 8 » laisse chercher, « lseek » dit tout de suite de
        // quoi il s'agit et donne la page de manuel à lire. Les deux,
        // donc, quand la table connaît le nom.
        EmuError::UnsupportedSyscall { backend, number } => {
            let nom = namsdec_re::table::syscall(*number)
                .map(|call| format!(" ({})", call.name))
                .unwrap_or_default();
            if fr {
                format!("l'interpréteur {backend} ne simule pas l'appel système {number}{nom}")
            } else {
                format!("the {backend} interpreter does not simulate syscall {number}{nom}")
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

/// Ce qu'il y a à faire, quand l'erreur appelle un geste précis.
///
/// Le message dit ce qui s'est passé ; celui-ci dit par où reprendre.
/// C'est la différence entre « accès interdit à 0x401000 » et « `.text`
/// n'est pas inscriptible : vos données vont dans `section .data` » —
/// la première phrase constate, la seconde apprend.
///
/// `None` quand l'erreur se suffit à elle-même : répéter le message
/// d'une autre façon n'aide personne.
pub fn advice(lang: Lang, error: &EmuError) -> Option<&'static str> {
    let fr = lang == Lang::Fr;
    Some(match error {
        // Une fonction qui vit dans une bibliothèque : ce n'est pas une
        // faute du programme, c'est une limite de namsdec — et elle se
        // dit comme telle, avec ce qui marche malgré tout.
        EmuError::UnsupportedLibrary { .. } => {
            if fr {
                "namsdec n'est pas un éditeur de liens dynamique : aucune bibliothèque \
                 n'est chargée. Il fournit lui-même une poignée de fonctions — afficher, \
                 lire, comparer des chaînes, demander de la mémoire — ce qui suffit à la \
                 plupart des programmes ; celle-ci n'en fait pas partie. Un binaire lié \
                 statiquement (`gcc -static`), lui, n'emprunte rien et s'exécute entier."
            } else {
                "namsdec is not a dynamic linker: no library is loaded. It provides a \
                 handful of functions itself — printing, reading, comparing strings, \
                 asking for memory — which is enough for most programs; this one is not \
                 among them. A statically linked binary (`gcc -static`) borrows nothing \
                 and runs in full."
            }
        }
        EmuError::Mem(MemError::Protection { needed, .. }) => match (needed, fr) {
            (Access::Write, true) => {
                "Cette zone n'est pas inscriptible. Le code (`section .text`) et les \
                 constantes (`.rodata`) sont en lecture seule : ce qui doit changer \
                 pendant l'exécution va dans `section .data` ou `section .bss`."
            }
            (Access::Write, false) => {
                "This region is not writable. Code (`section .text`) and constants \
                 (`.rodata`) are read-only: whatever changes while running belongs in \
                 `section .data` or `section .bss`."
            }
            (Access::Execute, true) => {
                "On ne saute pas dans des données. Vérifiez l'adresse du saut ou du \
                 `ret` : une adresse de retour écrasée sur la pile mène exactement ici."
            }
            (Access::Execute, false) => {
                "One does not jump into data. Check the jump or `ret` target: a return \
                 address smashed on the stack lands exactly here."
            }
            (Access::Read, true) => {
                "Cette zone ne se lit pas. C'est rare — vérifiez le registre qui a \
                 servi d'adresse."
            }
            (Access::Read, false) => {
                "This region cannot be read. That is rare — check the register used as \
                 an address."
            }
        },
        EmuError::Mem(MemError::Unmapped { .. }) => {
            if fr {
                "Rien n'est projeté à cette adresse. Le plus souvent, le registre qui \
                 sert d'adresse vaut 0 ou n'a jamais été chargé : regardez sa valeur \
                 dans le bandeau, juste au-dessus."
            } else {
                "Nothing is mapped at this address. Most often the register used as an \
                 address is 0 or was never loaded: check its value in the banner above."
            }
        }
        EmuError::Mem(MemError::OutOfBounds { .. }) => {
            if fr {
                "L'adresse sort de la mémoire simulée. Un décalage de tableau trop \
                 grand, ou une adresse calculée à partir d'un registre non initialisé."
            } else {
                "The address falls outside the simulated memory. An array index gone \
                 too far, or an address computed from an uninitialised register."
            }
        }
        EmuError::DivideByZero { .. } => {
            if fr {
                "Le diviseur vaut zéro. Le processeur lève une exception matérielle : \
                 c'est au programme de tester le diviseur avant `div` ou `idiv`."
            } else {
                "The divisor is zero. The processor raises a hardware exception: it is \
                 the program's job to test the divisor before `div` or `idiv`."
            }
        }
        EmuError::DivideOverflow { .. } => {
            if fr {
                "`div` divise RDX:RAX, pas RAX seul. Mettez RDX à zéro (`xor rdx, rdx`) \
                 avant une division non signée sur 64 bits — ou utilisez `cqo` pour une \
                 division signée."
            } else {
                "`div` divides RDX:RAX, not RAX alone. Zero RDX (`xor rdx, rdx`) before \
                 an unsigned 64-bit division — or use `cqo` for a signed one."
            }
        }
        // Cette aide a nommé quatre appels — `read`, `write`, `brk`,
        // `exit` — pendant tout le temps où l'interpréteur en a simulé
        // quarante-huit. Une liste écrite dans une phrase ne suit pas le
        // code qui la dément : elle n'énumère donc plus rien, elle dit
        // la LIGNE DE PARTAGE, qui, elle, ne bouge pas.
        EmuError::UnsupportedSyscall { .. } => {
            if fr {
                "namsdec ne simule pas cet appel système : il n'y a pas de vrai noyau \
                 derrière. Ce qu'il sait rendre, c'est ce qu'un programme demande pour \
                 tourner seul — lire, écrire, prendre de la mémoire, se renseigner sur \
                 lui-même. Ce qui manque touche au dehors : les fichiers, le réseau, \
                 les autres processus. La liste des appels système, dans l'analyse du \
                 binaire, dit pour chacun s'il est simulé."
            } else {
                "namsdec does not simulate this system call: there is no real kernel \
                 behind it. What it can answer is what a program needs to run on its \
                 own — reading, writing, taking memory, asking about itself. What is \
                 missing reaches outside: files, the network, other processes. The \
                 system call list, in the binary analysis, says for each one whether \
                 it is simulated."
            }
        }
        // Le piège du vieux tutoriel : `int 0x80` est l'interface des
        // programmes 32 bits. Elle existe encore, mais ce n'est pas
        // celle d'un programme 64 bits — et les numéros d'appels n'y
        // sont même pas les mêmes.
        EmuError::UnsupportedInstruction { mnemonic, .. } if mnemonic == "Int" => {
            if fr {
                "`int 0x80` est l'ancienne interface, celle des programmes 32 bits. \
                 En 64 bits, on écrit `syscall` — et attention, les numéros ne sont pas \
                 les mêmes d'une interface à l'autre : `write` vaut 4 en 32 bits, 1 en \
                 64 bits."
            } else {
                "`int 0x80` is the old interface, the one 32-bit programs use. In 64-bit \
                 code one writes `syscall` — and beware, the call numbers differ between \
                 the two: `write` is 4 on 32-bit, 1 on 64-bit."
            }
        }
        // Une instruction que namsdec ne simule pas : dire où lire ce
        // qu'elle fait vaut mieux que de s'arrêter sur un refus. La
        // documentation, elle, la couvre — c'est tout l'intérêt de
        // documenter ce qu'on n'exécute pas.
        EmuError::UnsupportedInstruction { mnemonic, .. }
            if namsdec_doc::lookup(mnemonic).is_some() =>
        {
            if fr {
                "namsdec ne simule pas cette instruction, mais il sait ce qu'elle fait : \
                 posez le curseur dessus et appuyez sur F1. L'interpréteur couvre ce qu'on \
                 écrit pour apprendre, la documentation couvre aussi ce qu'on lit dans les \
                 binaires des autres."
            } else {
                "namsdec does not simulate this instruction, but it knows what it does: \
                 put the cursor on it and press F1. The interpreter covers what one writes \
                 to learn; the documentation also covers what one reads in other people's \
                 binaries."
            }
        }
        EmuError::InvalidInstruction { .. } => {
            if fr {
                "Ces octets ne forment pas une instruction. On y arrive en sautant au \
                 milieu d'une instruction, ou dans des données."
            } else {
                "These bytes do not form an instruction. One gets here by jumping into \
                 the middle of an instruction, or into data."
            }
        }
        _ => return None,
    })
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
        // Le défaut de segmentation, dit en clair : l'adresse existe dans
        // la fenêtre, mais rien n'y est projeté — ni programme, ni pile,
        // ni tas.
        MemError::Unmapped { addr } => {
            if lang == Lang::Fr {
                format!(
                    "rien n'est projeté à l'adresse {addr:#x} : ni le programme, ni la pile, ni le tas"
                )
            } else {
                format!(
                    "nothing is mapped at {addr:#x}: not the program, not the stack, not the heap"
                )
            }
        }
        MemError::Protection {
            addr,
            needed,
            region,
        } => {
            let fr = lang == Lang::Fr;
            let verbe = match (needed, fr) {
                (Access::Read, true) => "lire",
                (Access::Read, false) => "read",
                (Access::Write, true) => "écrire à",
                (Access::Write, false) => "write to",
                (Access::Execute, true) => "exécuter",
                (Access::Execute, false) => "execute",
            };
            if fr {
                format!("interdit d'{verbe} l'adresse {addr:#x} : elle est dans {region}")
            } else {
                format!("not allowed to {verbe} {addr:#x}: it lives in {region}")
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
        // Le délai, et ce qu'on en fait. Nommer l'outil ET le temps
        // accordé, parce que les deux répondent à la question qu'on se
        // pose : qui a été arrêté, et à partir de quand.
        AsmError::TooLong { tool, seconds } => {
            if fr {
                format!(
                    "{tool} a dépassé les {seconds} secondes accordées et a été \
                     arrêté. Une directive comme `times` peut décrire un programme \
                     énorme en une ligne."
                )
            } else {
                format!(
                    "{tool} went past the {seconds} seconds it was given and was \
                     stopped. A directive such as `times` can describe a huge program \
                     in one line."
                )
            }
        }
        AsmError::Cancelled => {
            if fr {
                "assemblage arrêté à la demande".to_owned()
            } else {
                "assembly stopped on request".to_owned()
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
/// Les refus à l'installation d'une série d'exercices, dans la langue de
/// l'interface.
pub fn exercise_pack(lang: Lang, error: &namsdec_exercises::pack::PackError) -> String {
    use namsdec_exercises::pack::PackError as E;
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
                format!("ce n'est pas un fichier d'exercices valide : {e}")
            } else {
                format!("this is not a valid exercise file: {e}")
            }
        }
        E::BadId(id) => {
            if fr {
                format!(
                    "l'identifiant ne doit contenir que des lettres, des chiffres ou des tirets : {id}"
                )
            } else {
                format!("the identifier must be made of letters, digits or dashes: {id}")
            }
        }
        E::Empty => {
            if fr {
                "cette série ne contient aucun exercice".to_owned()
            } else {
                "this pack contains no exercise".to_owned()
            }
        }
        E::NoCode(id) => {
            if fr {
                format!("l'exercice « {id} » n'a pas de programme de départ")
            } else {
                format!("the exercise \"{id}\" has no starting program")
            }
        }
        E::DuplicateId(id) => {
            if fr {
                format!("deux exercices portent l'identifiant « {id} »")
            } else {
                format!("two exercises share the identifier \"{id}\"")
            }
        }
    }
}

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

    /// Un appel système refusé se nomme, et pas seulement par son
    /// numéro.
    ///
    /// « l'appel système 8 » laisse chercher dans une table ; « lseek »
    /// dit tout de suite de quoi il s'agit, et donne la page de manuel
    /// à lire. Le numéro reste, parce que c'est lui qu'on a mis dans
    /// RAX.
    #[test]
    fn un_appel_systeme_refuse_se_nomme() {
        let error = EmuError::UnsupportedSyscall {
            backend: "namsdec",
            number: 2,
        };

        let fr = emu(Lang::Fr, &error);
        assert!(fr.contains('2') && fr.contains("open"), "{fr}");
        let en = emu(Lang::En, &error);
        assert!(en.contains('2') && en.contains("open"), "{en}");

        // Un numéro que la table ignore ne doit pas inventer de nom, ni
        // laisser une parenthèse vide derrière lui.
        let inconnu = EmuError::UnsupportedSyscall {
            backend: "namsdec",
            number: 999,
        };
        let fr = emu(Lang::Fr, &inconnu);
        assert!(fr.contains("999") && !fr.contains('('), "{fr}");
    }

    /// L'aide d'un appel système refusé n'énumère pas à moitié.
    ///
    /// Elle a nommé quatre appels — `read`, `write`, `brk` et `exit` —
    /// pendant tout le temps où l'interpréteur en simulait quarante-
    /// huit. C'est la façon dont un texte ment sans que personne mente :
    /// il était vrai le jour où il fut écrit, et le quarante-neuvième
    /// appel s'ajoutera sans que son auteur revienne relire la phrase.
    ///
    /// Donc : ou bien l'aide n'énumère rien, ou bien elle les cite TOUS.
    /// La première branche est celle qu'on a choisie ; l'invariant
    /// n'interdit pas l'autre, il interdit l'entre-deux.
    #[test]
    fn l_aide_d_un_appel_refuse_n_enumere_pas_a_moitie() {
        let error = EmuError::UnsupportedSyscall {
            backend: "namsdec",
            number: 2,
        };
        for lang in [Lang::Fr, Lang::En] {
            let aide = advice(lang, &error).expect("un appel refusé appelle un conseil");
            let simulés: Vec<&str> = (0..=400)
                .filter_map(namsdec_re::table::syscall)
                .filter(|call| call.simulated)
                .map(|call| call.name)
                .collect();
            let cités: Vec<&str> = simulés
                .iter()
                .copied()
                .filter(|nom| aide.contains(&format!("`{nom}`")))
                .collect();
            assert!(
                cités.is_empty() || cités.len() == simulés.len(),
                "l'aide cite {} appels simulés sur {} : {cités:?}",
                cités.len(),
                simulés.len(),
            );
        }
    }

    /// Une erreur d'exécution ne se contente pas de constater : elle dit
    /// par où reprendre, et ce conseil dépend de ce qui manquait.
    #[test]
    fn a_protection_error_says_where_the_data_belongs() {
        let write = EmuError::Mem(MemError::Protection {
            addr: 0x401000,
            needed: Access::Write,
            region: ".text",
        });
        let fr = advice(Lang::Fr, &write).expect("écrire dans du code appelle un conseil");
        assert!(fr.contains("section .data"), "{fr}");
        assert!(advice(Lang::En, &write).unwrap().contains("section .data"));

        let exec = EmuError::Mem(MemError::Protection {
            addr: 0x402000,
            needed: Access::Execute,
            region: ".data",
        });
        assert!(advice(Lang::Fr, &exec).unwrap().contains("ret"));

        // La division : l'erreur classique est d'oublier RDX.
        assert!(
            advice(Lang::Fr, &EmuError::DivideOverflow { addr: 0 })
                .unwrap()
                .contains("RDX"),
        );
        // Une instruction documentée mais non simulée : on renvoie à sa
        // fiche plutôt que de laisser sur un refus.
        let documentée = EmuError::UnsupportedInstruction {
            backend: "namsdec",
            mnemonic: "Movlps".to_owned(),
        };
        assert!(advice(Lang::Fr, &documentée).unwrap().contains("F1"));
        // Et une instruction qu'on ne documente pas non plus n'invente
        // rien.
        //
        // L'exemple était `Vpconflictd` jusqu'au 15 octobre 2026, date
        // à laquelle elle a été simulée ET documentée — ce qui a fait
        // échouer ce contrôle, à juste titre. On prend donc une
        // instruction de prélecture pour la collecte, que namsdec ne
        // simule pas et dont la fiche n'apprendrait rien de plus que
        // « namsdec ne la fait pas ».
        assert!(
            advice(
                Lang::Fr,
                &EmuError::UnsupportedInstruction {
                    backend: "namsdec",
                    mnemonic: "Vgatherpf0dps".to_owned(),
                }
            )
            .is_none()
        );

        // Le vieux tutoriel 32 bits, reconnu par son instruction.
        let old_school = EmuError::UnsupportedInstruction {
            backend: "namsdec",
            mnemonic: "Int".to_owned(),
        };
        assert!(advice(Lang::Fr, &old_school).unwrap().contains("syscall"));
        assert!(advice(Lang::En, &old_school).unwrap().contains("32-bit"));

        // Et ce qui n'appelle aucun geste n'invente pas de conseil.
        assert!(
            advice(
                Lang::Fr,
                &EmuError::UnsupportedRegister {
                    backend: "namsdec",
                    register: "CR0".to_owned(),
                }
            )
            .is_none()
        );
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
