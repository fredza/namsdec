//! Reconnaître ce que le code *fait*, et non ce qu'il dit.
//!
//! Une suite d'instructions ne se lit pas une par une : on y reconnaît
//! des figures. « Charger une constante, la mettre en XOR avec ce que
//! l'utilisateur a tapé, sauter si le résultat est nul » n'est pas trois
//! instructions, c'est **une comparaison déguisée** — et dans un
//! programme qui demande un mot de passe, c'est le seul endroit qui
//! compte.
//!
//! Ce module relève les figures les plus parlantes. Il en dit ce qu'il
//! voit, jamais plus : un motif reconnu est une piste, pas une preuve,
//! et l'endroit est donné pour qu'on aille lire soi-même.
//!
//! La figure la plus payante est aussi la plus simple : **une constante
//! qui est du texte**. `mov ecx, 0x48555648` ne dit rien ; les mêmes
//! octets lus comme des caractères disent `HVUH`, et l'on vient de lire
//! le début d'un mot de passe.

use namsdec_disasm::{DecodedInstruction, FlowControl, Mnemonic, OpKind, Register};

/// Ce qu'on a reconnu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pattern {
    /// Un nombre écrit dans le code qui, lu comme des octets, est du
    /// texte lisible.
    AsciiConstant,
    /// Une comparaison de longueur : `cmp reg, n` suivi d'un saut.
    LengthCheck,
    /// Une comparaison par XOR : on met en XOR avec une constante, et
    /// l'on saute si le résultat est nul. Nul veut dire égal.
    XorCompare,
    /// Une comparaison de chaîne par instruction de chaîne.
    StringCompare,
    /// `test reg, reg` suivi d'un saut : « est-ce nul ? ».
    NullCheck,
    /// Un saut dont la cible sort d'un tableau : un `switch`.
    JumpTable,
}

impl Pattern {
    /// Ce que le motif veut dire, en une phrase.
    pub fn explain(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match self {
            Pattern::AsciiConstant if fr => {
                "Ce nombre est du texte : les octets qui le composent sont des caractères \
                 lisibles. Un mot de passe comparé d'un bloc se cache souvent ainsi."
            }
            Pattern::AsciiConstant => {
                "This number is text: the bytes making it up are readable characters. A \
                 password compared in one go often hides this way."
            }
            Pattern::LengthCheck if fr => {
                "Une longueur est comparée à un nombre fixe. C'est le premier filtre d'une \
                 vérification : ce qui n'a pas la bonne taille est refusé sans être lu."
            }
            Pattern::LengthCheck => {
                "A length is compared against a fixed number. This is a check's first \
                 filter: what has the wrong size is refused without being read."
            }
            Pattern::XorCompare if fr => {
                "Une comparaison déguisée : `a XOR b` ne vaut zéro que si a et b sont \
                 égaux. La constante est donc la valeur attendue, telle quelle."
            }
            Pattern::XorCompare => {
                "A disguised comparison: `a XOR b` is zero only when a and b are equal. \
                 The constant is therefore the expected value, as is."
            }
            Pattern::StringCompare if fr => {
                "Deux suites d'octets sont comparées par le processeur lui-même, sans \
                 boucle écrite."
            }
            Pattern::StringCompare => {
                "Two runs of bytes are compared by the processor itself, with no written \
                 loop."
            }
            Pattern::NullCheck if fr => {
                "« Est-ce nul ? », en deux octets. C'est ainsi qu'on teste un pointeur, un \
                 code de retour, un drapeau."
            }
            Pattern::NullCheck => {
                "« Is this zero? », in two bytes. That is how one tests a pointer, a return \
                 code, a flag."
            }
            Pattern::JumpTable if fr => {
                "La cible du saut sort d'un tableau : c'est un `switch`, et les adresses de \
                 ses branches sont dans les données."
            }
            Pattern::JumpTable => {
                "The jump target comes out of an array: this is a `switch`, and its \
                 branches' addresses live in the data."
            }
        }
    }

    pub fn title(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match self {
            Pattern::AsciiConstant => {
                if fr {
                    "constante en texte"
                } else {
                    "text constant"
                }
            }
            Pattern::LengthCheck => {
                if fr {
                    "contrôle de longueur"
                } else {
                    "length check"
                }
            }
            Pattern::XorCompare => {
                if fr {
                    "comparaison par XOR"
                } else {
                    "XOR comparison"
                }
            }
            Pattern::StringCompare => {
                if fr {
                    "comparaison de chaîne"
                } else {
                    "string compare"
                }
            }
            Pattern::NullCheck => {
                if fr {
                    "test de nullité"
                } else {
                    "null test"
                }
            }
            Pattern::JumpTable => {
                if fr {
                    "table de sauts"
                } else {
                    "jump table"
                }
            }
        }
    }
}

/// Une figure reconnue, à un endroit précis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub addr: u64,
    pub pattern: Pattern,
    /// Ce qu'on a lu de concret : la constante, sa lecture en texte, la
    /// longueur attendue.
    pub detail: String,
}

/// Les octets d'un nombre, lus comme du texte — s'ils en sont.
///
/// Quatre caractères au moins : en deçà, deux lettres au milieu d'un
/// nombre ordinaire suffiraient à crier au trésor. Et tous imprimables :
/// un seul octet de contrôle, et ce n'est plus du texte.
pub fn as_text(value: u64) -> Option<String> {
    for largeur in [8usize, 4] {
        let octets = &value.to_le_bytes()[..largeur];
        // Un nombre de huit octets dont le haut est nul n'est pas une
        // chaîne de huit caractères : on retombera sur quatre.
        if octets.contains(&0) {
            continue;
        }
        if octets
            .iter()
            .all(|b| (0x20..=0x7e).contains(b) || *b == b'\n' || *b == b'\t')
        {
            return String::from_utf8(octets.to_vec()).ok();
        }
    }
    None
}

/// Relève les figures d'un désassemblage.
pub fn scan(disasm: &[DecodedInstruction]) -> Vec<Finding> {
    let décodées: Vec<Option<namsdec_disasm::Decoded>> = disasm
        .iter()
        .map(|instr| namsdec_disasm::decode_one(&instr.bytes, instr.address))
        .collect();
    let mut out = Vec::new();

    for (index, decoded) in décodées.iter().enumerate() {
        let Some(instr) = decoded else { continue };
        let addr = disasm[index].address;

        // --- Une constante qui est du texte -------------------------
        if matches!(
            instr.op1_kind(),
            OpKind::Immediate8
                | OpKind::Immediate16
                | OpKind::Immediate32
                | OpKind::Immediate32to64
                | OpKind::Immediate64
        ) && let Some(texte) = as_text(instr.immediate(1))
        {
            out.push(Finding {
                addr,
                pattern: Pattern::AsciiConstant,
                detail: format!("{:#x} = {texte:?}", instr.immediate(1)),
            });
        }

        // --- Une comparaison de longueur ----------------------------
        // `cmp reg, n` suivi d'un saut : la valeur comparée est petite,
        // sinon c'est un nombre quelconque et non une taille.
        if instr.mnemonic() == Mnemonic::Cmp
            && instr.op0_kind() == OpKind::Register
            && matches!(
                instr.op1_kind(),
                OpKind::Immediate8
                    | OpKind::Immediate8to32
                    | OpKind::Immediate8to64
                    | OpKind::Immediate32
                    | OpKind::Immediate32to64
            )
            && suit_un_saut(&décodées, index)
        {
            let valeur = instr.immediate(1);
            if (1..=4096).contains(&valeur) {
                out.push(Finding {
                    addr,
                    pattern: Pattern::LengthCheck,
                    detail: format!("{valeur}"),
                });
            }
        }

        // --- Une comparaison par XOR --------------------------------
        // `mov reg, constante` puis `xor reg, [mémoire]` : le résultat
        // ne vaut zéro que si la mémoire contient la constante.
        if instr.mnemonic() == Mnemonic::Mov
            && instr.op0_kind() == OpKind::Register
            && matches!(
                instr.op1_kind(),
                OpKind::Immediate32 | OpKind::Immediate32to64 | OpKind::Immediate64
            )
        {
            let registre = instr.op0_register();
            let constante = instr.immediate(1);
            for suivant in décodées.iter().skip(index + 1).take(3).flatten() {
                if suivant.mnemonic() == Mnemonic::Xor
                    && suivant.op0_kind() == OpKind::Register
                    && même_registre(suivant.op0_register(), registre)
                    && suivant.op1_kind() == OpKind::Memory
                {
                    let lecture = as_text(constante)
                        .map(|t| format!(" = {t:?}"))
                        .unwrap_or_default();
                    out.push(Finding {
                        addr,
                        pattern: Pattern::XorCompare,
                        detail: format!("{constante:#x}{lecture}"),
                    });
                    break;
                }
                // Le registre réécrit : ce n'est plus la même valeur.
                if suivant.op0_kind() == OpKind::Register
                    && même_registre(suivant.op0_register(), registre)
                {
                    break;
                }
            }
        }

        // --- Une comparaison de chaîne ------------------------------
        if matches!(
            instr.mnemonic(),
            Mnemonic::Cmpsb | Mnemonic::Cmpsw | Mnemonic::Cmpsd | Mnemonic::Cmpsq | Mnemonic::Scasb
        ) && (instr.has_rep_prefix() || instr.has_repe_prefix() || instr.has_repne_prefix())
        {
            out.push(Finding {
                addr,
                pattern: Pattern::StringCompare,
                detail: disasm[index].text.clone(),
            });
        }

        // --- Un test de nullité -------------------------------------
        if instr.mnemonic() == Mnemonic::Test
            && instr.op0_kind() == OpKind::Register
            && instr.op1_kind() == OpKind::Register
            && instr.op0_register() == instr.op1_register()
            && suit_un_saut(&décodées, index)
        {
            out.push(Finding {
                addr,
                pattern: Pattern::NullCheck,
                detail: format!("{:?}", instr.op0_register()).to_lowercase(),
            });
        }

        // --- Une table de sauts -------------------------------------
        if instr.flow_control() == FlowControl::IndirectBranch
            && instr.op0_kind() == OpKind::Memory
            && instr.memory_index() != Register::None
        {
            out.push(Finding {
                addr,
                pattern: Pattern::JumpTable,
                detail: disasm[index].text.clone(),
            });
        }
    }
    out
}

/// L'instruction qui suit est-elle un saut conditionnel ?
fn suit_un_saut(décodées: &[Option<namsdec_disasm::Decoded>], index: usize) -> bool {
    décodées
        .iter()
        .skip(index + 1)
        .take(3)
        .flatten()
        .any(|i| i.flow_control() == FlowControl::ConditionalBranch)
}

/// Deux registres désignent-ils le même, quelle que soit leur largeur ?
fn même_registre(a: Register, b: Register) -> bool {
    a == b || a.full_register() == b.full_register()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La figure qui résout un crackme : une constante mise en XOR avec
    /// ce que l'utilisateur a tapé.
    #[test]
    fn a_xor_comparison_hands_over_the_expected_value() {
        // mov ecx, 0x48555648 ; xor ecx, [rbx+rax] — pris tel quel dans
        // un vrai crackme.
        let bytes: &[u8] = &[
            0xB9, 0x48, 0x56, 0x55, 0x48, // mov ecx, 0x48555648
            0x33, 0x0C, 0x03, // xor ecx, [rbx+rax]
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let trouvés = scan(&disasm);

        let xor = trouvés
            .iter()
            .find(|f| f.pattern == Pattern::XorCompare)
            .expect("la comparaison par XOR est reconnue");
        assert!(
            xor.detail.contains("HVUH"),
            "la constante doit se lire en texte : {}",
            xor.detail
        );
        // Et la constante est aussi relevée pour elle-même.
        assert!(
            trouvés
                .iter()
                .any(|f| f.pattern == Pattern::AsciiConstant && f.detail.contains("HVUH"))
        );
    }

    /// Un nombre ordinaire n'est pas du texte, et l'on ne prétend pas le
    /// contraire.
    #[test]
    fn an_ordinary_number_is_not_text() {
        assert_eq!(as_text(0), None);
        assert_eq!(as_text(1), None);
        assert_eq!(as_text(0x400000), None);
        // Quatre caractères lisibles : c'en est.
        assert_eq!(as_text(0x48555648).as_deref(), Some("HVUH"));
        // Huit aussi, quand les huit octets sont lisibles.
        assert_eq!(as_text(0x4142434445464748).as_deref(), Some("HGFEDCBA"));
    }

    /// Le premier filtre d'une vérification : la longueur.
    #[test]
    fn a_length_check_is_recognised() {
        // cmp rsi, 7 ; jne +2
        let bytes: &[u8] = &[0x48, 0x83, 0xFE, 0x07, 0x75, 0x02];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let trouvé = scan(&disasm)
            .into_iter()
            .find(|f| f.pattern == Pattern::LengthCheck)
            .expect("le contrôle de longueur est reconnu");
        assert_eq!(trouvé.detail, "7");
        assert_eq!(trouvé.addr, 0x1000);
    }

    /// Une comparaison sans saut derrière n'est pas un contrôle : on ne
    /// relève pas tout ce qui ressemble de loin à quelque chose.
    #[test]
    fn a_comparison_without_a_branch_is_not_a_check() {
        // cmp rsi, 7 ; nop
        let bytes: &[u8] = &[0x48, 0x83, 0xFE, 0x07, 0x90];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        assert!(
            !scan(&disasm)
                .iter()
                .any(|f| f.pattern == Pattern::LengthCheck)
        );
    }
}
