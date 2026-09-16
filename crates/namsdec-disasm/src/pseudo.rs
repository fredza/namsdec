//! L'assembleur dit en français — ou en anglais.
//!
//! `add rax, rcx` se lit « RAX = RAX + RCX » ; `jne boucle » se lit « si
//! différent, aller à boucle ». Rien de nouveau n'est calculé ici : la
//! même instruction, écrite comme on la lirait à voix haute. C'est le
//! chaînon que tout le monde franchit de tête après quelques semaines,
//! et qui manque les premiers jours.
//!
//! Ce module ne devine rien. Une instruction qu'il ne sait pas dire est
//! rendue telle quelle, en syntaxe NASM : une paraphrase approximative
//! serait pire que la forme d'origine, qui au moins est exacte.

use iced_x86::{ConditionCode, Formatter, Instruction, Mnemonic, NasmFormatter, OpKind, Register};

/// L'instruction, dite en clair.
///
/// `lang` suit la convention des autres crates : « fr » ou « en ».
pub fn of(instr: &Instruction, lang: &str) -> String {
    let fr = lang.starts_with("fr");
    let op = |index: u32| operand(instr, index);
    let m = instr.mnemonic();

    // Les cas où l'idiome ne se lit pas dans les opérandes : `xor rax,
    // rax` ne veut pas dire « RAX = RAX ou exclusif RAX », il veut dire
    // « RAX = 0 », et c'est ce que tout le monde y lit.
    if matches!(m, Mnemonic::Xor | Mnemonic::Sub)
        && instr.op_count() == 2
        && instr.op0_kind() == OpKind::Register
        && instr.op1_kind() == OpKind::Register
        && instr.op0_register() == instr.op1_register()
    {
        return format!("{} = 0", op(0));
    }

    match m {
        Mnemonic::Mov | Mnemonic::Movzx | Mnemonic::Movsx | Mnemonic::Movsxd => {
            format!("{} = {}", op(0), op(1))
        }
        Mnemonic::Lea => {
            let quoi = if fr { "adresse de" } else { "address of" };
            format!("{} = {quoi} {}", op(0), inner(instr))
        }
        Mnemonic::Add => format!("{} = {} + {}", op(0), op(0), op(1)),
        Mnemonic::Sub => format!("{} = {} − {}", op(0), op(0), op(1)),
        Mnemonic::Imul | Mnemonic::Mul if instr.op_count() >= 2 => {
            format!("{} = {} × {}", op(0), op(0), op(1))
        }
        Mnemonic::Mul => format!("RDX:RAX = RAX × {}", op(0)),
        Mnemonic::Div | Mnemonic::Idiv => {
            if fr {
                format!("RAX = RDX:RAX ÷ {}, reste dans RDX", op(0))
            } else {
                format!("RAX = RDX:RAX ÷ {}, remainder in RDX", op(0))
            }
        }
        Mnemonic::Inc => format!("{} = {} + 1", op(0), op(0)),
        Mnemonic::Dec => format!("{} = {} − 1", op(0), op(0)),
        Mnemonic::Neg => format!("{} = −{}", op(0), op(0)),
        Mnemonic::Not => {
            if fr {
                format!("{} = tous les bits inversés de {}", op(0), op(0))
            } else {
                format!("{} = every bit of {} flipped", op(0), op(0))
            }
        }
        Mnemonic::And => format!("{} = {} ET {}", op(0), op(0), op(1)),
        Mnemonic::Or => format!("{} = {} OU {}", op(0), op(0), op(1)),
        Mnemonic::Xor => format!("{} = {} OU-exclusif {}", op(0), op(0), op(1)),
        Mnemonic::Shl | Mnemonic::Sal => shift(instr, op(0), op(1), true, fr),
        Mnemonic::Shr | Mnemonic::Sar => shift(instr, op(0), op(1), false, fr),
        Mnemonic::Cmp => {
            if fr {
                format!("comparer {} et {}", op(0), op(1))
            } else {
                format!("compare {} and {}", op(0), op(1))
            }
        }
        Mnemonic::Test => {
            if fr {
                format!("tester les bits de {} contre {}", op(0), op(1))
            } else {
                format!("test the bits of {} against {}", op(0), op(1))
            }
        }
        Mnemonic::Push => {
            if fr {
                format!("empiler {}", op(0))
            } else {
                format!("push {}", op(0))
            }
        }
        Mnemonic::Pop => {
            if fr {
                format!("dépiler dans {}", op(0))
            } else {
                format!("pop into {}", op(0))
            }
        }
        Mnemonic::Call => {
            if fr {
                format!("appeler {}", op(0))
            } else {
                format!("call {}", op(0))
            }
        }
        Mnemonic::Ret => {
            if fr {
                "revenir à l'appelant".to_owned()
            } else {
                "return to the caller".to_owned()
            }
        }
        Mnemonic::Jmp => {
            if fr {
                format!("aller à {}", op(0))
            } else {
                format!("go to {}", op(0))
            }
        }
        Mnemonic::Syscall => {
            if fr {
                "appel système (le numéro est dans RAX)".to_owned()
            } else {
                "system call (the number is in RAX)".to_owned()
            }
        }
        Mnemonic::Nop => {
            if fr {
                "ne rien faire".to_owned()
            } else {
                "do nothing".to_owned()
            }
        }
        Mnemonic::Hlt => {
            if fr {
                "arrêter la machine".to_owned()
            } else {
                "halt the machine".to_owned()
            }
        }
        Mnemonic::Leave => {
            if fr {
                "défaire le cadre d'appel (RSP = RBP, dépiler RBP)".to_owned()
            } else {
                "undo the call frame (RSP = RBP, pop RBP)".to_owned()
            }
        }
        _ => {
            // Un saut conditionnel, un `setcc`, un `cmovcc` : leur
            // condition est dans le mnémonique, pas dans les opérandes.
            if let Some(texte) = conditional(instr, fr) {
                return texte;
            }
            // Ce qu'on ne sait pas dire, on le rend tel quel : une
            // paraphrase approximative serait pire que la forme
            // d'origine, qui au moins est exacte.
            nasm(instr)
        }
    }
}

/// Un décalage, avec ce qu'il fait au nombre : multiplier ou diviser par
/// une puissance de deux, ce qui est *la* raison de s'en servir.
fn shift(instr: &Instruction, dest: String, count: String, gauche: bool, fr: bool) -> String {
    let facteur = (instr.op_count() > 1 && instr.op1_kind() != OpKind::Register)
        .then(|| instr.try_immediate(1).ok())
        .flatten()
        .filter(|n| *n < 63)
        .map(|n| 1u64 << n);
    let opération = if gauche { "<<" } else { ">>" };
    match (facteur, gauche, fr) {
        (Some(f), true, true) => format!("{dest} = {dest} {opération} {count}  (× {f})"),
        (Some(f), true, false) => format!("{dest} = {dest} {opération} {count}  (× {f})"),
        (Some(f), false, true) => format!("{dest} = {dest} {opération} {count}  (÷ {f})"),
        (Some(f), false, false) => format!("{dest} = {dest} {opération} {count}  (÷ {f})"),
        _ => format!("{dest} = {dest} {opération} {count}"),
    }
}

/// Les instructions dont la condition est dans le nom.
fn conditional(instr: &Instruction, fr: bool) -> Option<String> {
    let condition = describe_condition(instr.condition_code(), fr)?;
    let m = instr.mnemonic();
    let nom = format!("{m:?}").to_ascii_lowercase();
    let cible = operand(instr, 0);
    if nom.starts_with("set") {
        return Some(if fr {
            format!("{cible} = 1 si {condition}, sinon 0")
        } else {
            format!("{cible} = 1 if {condition}, else 0")
        });
    }
    if nom.starts_with("cmov") {
        let source = operand(instr, 1);
        return Some(if fr {
            format!("{cible} = {source} si {condition}")
        } else {
            format!("{cible} = {source} if {condition}")
        });
    }
    if nom.starts_with('j') {
        return Some(if fr {
            format!("si {condition}, aller à {cible}")
        } else {
            format!("if {condition}, go to {cible}")
        });
    }
    None
}

/// La condition, dite comme on la lit — et non comme elle s'écrit.
fn describe_condition(code: ConditionCode, fr: bool) -> Option<&'static str> {
    Some(match (code, fr) {
        (ConditionCode::None, _) => return None,
        (ConditionCode::e, true) => "égal",
        (ConditionCode::e, false) => "equal",
        (ConditionCode::ne, true) => "différent",
        (ConditionCode::ne, false) => "not equal",
        (ConditionCode::b, true) => "plus petit (non signé)",
        (ConditionCode::b, false) => "below (unsigned)",
        (ConditionCode::ae, true) => "plus grand ou égal (non signé)",
        (ConditionCode::ae, false) => "above or equal (unsigned)",
        (ConditionCode::be, true) => "plus petit ou égal (non signé)",
        (ConditionCode::be, false) => "below or equal (unsigned)",
        (ConditionCode::a, true) => "plus grand (non signé)",
        (ConditionCode::a, false) => "above (unsigned)",
        (ConditionCode::l, true) => "plus petit (signé)",
        (ConditionCode::l, false) => "less (signed)",
        (ConditionCode::ge, true) => "plus grand ou égal (signé)",
        (ConditionCode::ge, false) => "greater or equal (signed)",
        (ConditionCode::le, true) => "plus petit ou égal (signé)",
        (ConditionCode::le, false) => "less or equal (signed)",
        (ConditionCode::g, true) => "plus grand (signé)",
        (ConditionCode::g, false) => "greater (signed)",
        (ConditionCode::s, true) => "négatif",
        (ConditionCode::s, false) => "negative",
        (ConditionCode::ns, true) => "positif ou nul",
        (ConditionCode::ns, false) => "positive or zero",
        (ConditionCode::o, true) => "débordement",
        (ConditionCode::o, false) => "overflow",
        (ConditionCode::no, true) => "pas de débordement",
        (ConditionCode::no, false) => "no overflow",
        (ConditionCode::p, true) => "parité paire",
        (ConditionCode::p, false) => "even parity",
        (ConditionCode::np, true) => "parité impaire",
        (ConditionCode::np, false) => "odd parity",
    })
}

/// Un opérande, écrit comme dans le code — mais la mémoire dite en
/// toutes lettres : `[rsi+8]` devient `mémoire[RSI + 8]`, parce que les
/// crochets sont justement ce qu'on ne lit pas au début.
fn operand(instr: &Instruction, index: u32) -> String {
    let mut formatter = NasmFormatter::new();
    formatter
        .options_mut()
        .set_space_after_operand_separator(true);
    formatter.options_mut().set_uppercase_registers(true);
    let mut texte = String::new();
    if formatter.format_operand(instr, &mut texte, index).is_err() {
        return String::new();
    }
    if instr.op_kind(index) == OpKind::Memory {
        return format!("mémoire{}", spaced(&texte));
    }
    // Une cible de saut : l'adresse nue. `short 0000000000400000h` dit
    // comment elle est encodée, ce qui n'a rien à faire dans une phrase.
    if matches!(
        instr.op_kind(index),
        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64
    ) {
        return format!("{:#x}", instr.near_branch_target());
    }
    texte
}

/// Ce qu'il y a entre les crochets, pour `lea` : l'adresse elle-même,
/// pas ce qu'elle contient.
fn inner(instr: &Instruction) -> String {
    let texte = operand(instr, 1);
    texte
        .trim_start_matches("mémoire")
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned()
}

/// Des espaces autour des signes : `[RSI+RCX*8+16]` ne se lit pas,
/// `[RSI + RCX*8 + 16]` si.
fn spaced(texte: &str) -> String {
    texte.replace('+', " + ").replace("  ", " ")
}

/// La forme NASM, pour ce qu'on ne sait pas dire autrement.
fn nasm(instr: &Instruction) -> String {
    let mut formatter = NasmFormatter::new();
    formatter
        .options_mut()
        .set_space_after_operand_separator(true);
    let mut texte = String::new();
    formatter.format(instr, &mut texte);
    texte
}

/// Le registre, en majuscules — comme le bandeau les montre.
pub fn register_name(register: Register) -> String {
    format!("{register:?}").to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dire(bytes: &[u8]) -> String {
        let instr = crate::decode_one(bytes, 0x400000).expect("décodable");
        of(&instr, "fr")
    }

    #[test]
    fn the_everyday_instructions_read_as_sentences() {
        // 48 89 d8 : mov rax, rbx
        assert_eq!(dire(&[0x48, 0x89, 0xd8]), "RAX = RBX");
        // 48 01 c8 : add rax, rcx
        assert_eq!(dire(&[0x48, 0x01, 0xc8]), "RAX = RAX + RCX");
        // 48 ff c9 : dec rcx
        assert_eq!(dire(&[0x48, 0xff, 0xc9]), "RCX = RCX − 1");
        // c3 : ret
        assert_eq!(dire(&[0xc3]), "revenir à l'appelant");
    }

    /// L'idiome qu'on lit sans le calculer : `xor rax, rax` veut dire
    /// zéro, et le dire autrement serait le traduire mot à mot.
    #[test]
    fn the_zeroing_idiom_is_read_as_a_zero() {
        // 48 31 c0 : xor rax, rax
        assert_eq!(dire(&[0x48, 0x31, 0xc0]), "RAX = 0");
        // Mais un xor entre deux registres différents reste un xor.
        // 48 31 c8 : xor rax, rcx
        assert_eq!(dire(&[0x48, 0x31, 0xc8]), "RAX = RAX OU-exclusif RCX");
    }

    /// La condition est dite comme on la lit, et le signe avec : c'est
    /// là que l'on se trompe.
    #[test]
    fn a_conditional_jump_says_its_condition() {
        // 75 fe : jne $-0
        assert_eq!(dire(&[0x75, 0xfe]), "si différent, aller à 0x400000");
        // 77 fe : ja
        assert_eq!(
            dire(&[0x77, 0xfe]),
            "si plus grand (non signé), aller à 0x400000"
        );
        // 7f fe : jg
        assert_eq!(
            dire(&[0x7f, 0xfe]),
            "si plus grand (signé), aller à 0x400000"
        );
    }

    /// La mémoire est dite en toutes lettres : les crochets sont
    /// justement ce qu'on ne lit pas au début.
    #[test]
    fn memory_is_spelled_out() {
        // 8a 44 8e 08 : mov al, [rsi+rcx*4+8]
        let texte = dire(&[0x8a, 0x44, 0x8e, 0x08]);
        assert!(texte.starts_with("AL = mémoire["), "{texte}");
        assert!(texte.contains("RSI + "), "{texte}");
    }

    /// Un décalage dit ce qu'il fait au nombre : c'est la raison de s'en
    /// servir.
    #[test]
    fn a_shift_says_what_it_multiplies_by() {
        // 48 c1 e0 03 : shl rax, 3
        assert_eq!(dire(&[0x48, 0xc1, 0xe0, 0x03]), "RAX = RAX << 3  (× 8)");
    }

    /// Ce qu'on ne sait pas dire est rendu tel quel, en NASM : une
    /// paraphrase approximative serait pire que la forme exacte.
    #[test]
    fn what_cannot_be_said_plainly_is_left_alone() {
        // 0f ae f0 : mfence
        assert_eq!(dire(&[0x0f, 0xae, 0xf0]), "mfence");
    }

    /// Et les deux langues disent la même chose autrement.
    #[test]
    fn both_languages_answer() {
        let instr = crate::decode_one(&[0xc3], 0x400000).unwrap();
        assert_eq!(of(&instr, "en"), "return to the caller");
        assert_ne!(of(&instr, "fr"), of(&instr, "en"));
    }
}
