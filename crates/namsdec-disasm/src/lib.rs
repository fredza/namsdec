//! Désassemblage x86-64 via `iced-x86`, formaté en syntaxe NASM — cohérent
//! avec le reste de l'outil. Réutilisé tel quel en phase 2 pour le futur
//! désassembleur ELF (seule la source des octets change).

use iced_x86::{Decoder, DecoderOptions, Formatter, GasFormatter, Instruction, NasmFormatter};

/// La syntaxe dans laquelle on lit l'assembleur.
///
/// Les deux disent la même chose et ne se ressemblent pas : `mov rax,
/// [rbx+8]` d'un côté, `movq 8(%rbx), %rax` de l'autre. Ce n'est pas un
/// détail de goût — c'est la première marche devant un rapport
/// d'`objdump`, une session `gdb` ou un tutoriel trouvé en ligne, où
/// tout est écrit dans l'autre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Syntax {
    /// Celle de NASM, dite aussi Intel : la destination d'abord, les
    /// adresses entre crochets. C'est celle qu'on écrit dans l'éditeur,
    /// et la seule que nasm sait assembler.
    #[default]
    Nasm,
    /// Celle du système : `gdb`, `objdump`, `as`. La source d'abord, les
    /// registres préfixés d'un `%`, la taille suffixée au mnémonique.
    ///
    /// **Une vue, non une écriture** : namsdec assemble avec nasm, qui
    /// ne la lit pas. Un listing montré ainsi se lit, se copie dans un
    /// rapport, et ne se réassemble pas.
    System,
}

impl Syntax {
    /// L'identifiant stable, pour les réglages.
    pub fn id(self) -> &'static str {
        match self {
            Syntax::Nasm => "nasm",
            Syntax::System => "system",
        }
    }

    /// Celle que cet identifiant désigne, si on la connaît.
    pub fn by_id(id: &str) -> Option<Self> {
        match id {
            "nasm" => Some(Syntax::Nasm),
            "system" => Some(Syntax::System),
            _ => None,
        }
    }

    /// Met en forme une instruction dans cette syntaxe.
    fn format(self, instr: &Instruction, espacé: bool, out: &mut String) {
        match self {
            Syntax::Nasm => {
                let mut f = NasmFormatter::new();
                f.options_mut().set_space_after_operand_separator(espacé);
                f.format(instr, out);
            }
            Syntax::System => {
                let mut f = GasFormatter::new();
                f.options_mut().set_space_after_operand_separator(espacé);
                f.format(instr, out);
            }
        }
    }
}

/// L'instruction décodée, telle que la rend [`decode_one`].
///
/// Réexportée pour que l'interface puisse la nommer dans ses signatures
/// sans dépendre elle-même du décodeur : ce qu'elle en fait, elle le
/// fait toujours en passant par cette crate.
pub mod install;
pub mod known;
pub mod lanceur;
pub mod plugin;
pub mod pseudo;

pub use iced_x86::Instruction as Decoded;
/// Les types du décodeur dont les autres crates ont besoin pour lire une
/// instruction : la nature d'un opérande, et les registres.
pub use iced_x86::{FlowControl, Mnemonic, OpKind, Register};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    pub address: u64,
    pub bytes: Vec<u8>,
    pub text: String,
}

/// Décode une seule instruction en tête de `bytes`, positionnée à `ip`.
/// Réutilisé par `namsdec-emu` pour le fetch/decode de l'interpréteur —
/// seule la sémantique d'exécution y est réécrite à la main.
pub fn decode_one(bytes: &[u8], ip: u64) -> Option<Instruction> {
    let mut decoder = Decoder::with_ip(64, bytes, ip, DecoderOptions::NONE);
    if !decoder.can_decode() {
        return None;
    }
    let instr = decoder.decode();
    if instr.is_invalid() {
        None
    } else {
        Some(instr)
    }
}

/// Met en forme une instruction déjà décodée, en syntaxe NASM et espacée
/// comme on l'écrit à la main : `mov rax, rcx` plutôt que `mov rax,rcx`.
///
/// C'est la forme que montre l'interface à côté du code de
/// l'utilisateur ; les vidages (onglet Octets, listing) gardent la forme
/// compacte de [`disassemble`], alignée colonne par colonne.
pub fn format_one(instr: &Instruction) -> String {
    format_one_in(instr, Syntax::Nasm)
}

/// La même chose, dans la syntaxe demandée.
pub fn format_one_in(instr: &Instruction, syntax: Syntax) -> String {
    let mut text = String::new();
    syntax.format(instr, true, &mut text);
    text
}

/// Désassemble `bytes` en supposant qu'ils sont chargés à partir de
/// `base_addr` (mode 64 bits).
pub fn disassemble(bytes: &[u8], base_addr: u64) -> Vec<DecodedInstruction> {
    disassemble_in(bytes, base_addr, Syntax::Nasm)
}

/// Le même désassemblage, dans la syntaxe demandée.
pub fn disassemble_in(bytes: &[u8], base_addr: u64, syntax: Syntax) -> Vec<DecodedInstruction> {
    let mut decoder = Decoder::with_ip(64, bytes, base_addr, DecoderOptions::NONE);
    let mut instr = Instruction::default();
    let mut result = Vec::new();
    let mut text = String::new();

    while decoder.can_decode() {
        decoder.decode_out(&mut instr);
        text.clear();
        syntax.format(&instr, false, &mut text);

        let start = (instr.ip() - base_addr) as usize;
        let end = start + instr.len();
        result.push(DecodedInstruction {
            address: instr.ip(),
            bytes: bytes[start..end].to_vec(),
            text: text.clone(),
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_mov_eax_ebx() {
        let out = disassemble(&[0x89, 0xD8], 0x400000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].address, 0x400000);
        assert_eq!(out[0].bytes, vec![0x89, 0xD8]);
        assert_eq!(out[0].text, "mov eax,ebx");
    }

    #[test]
    fn decodes_sum_loop_program() {
        // mov eax,0 / mov ecx,5 / add rax,rcx / dec rcx / jnz $-6 / hlt
        // (exactement les octets produits par nasm pour le programme de
        // somme du plan — voir namsdec-asm::tests::assembles_sum_loop_*)
        let bytes: &[u8] = &[
            0xB8, 0x00, 0x00, 0x00, 0x00, // mov eax,0
            0xB9, 0x05, 0x00, 0x00, 0x00, // mov ecx,5
            0x48, 0x01, 0xC8, // add rax,rcx
            0x48, 0xFF, 0xC9, // dec rcx
            0x75, 0xF8, // jnz sum_loop
            0xF4, // hlt
        ];
        let out = disassemble(bytes, 0x400000);
        assert_eq!(out.len(), 6);
        assert_eq!(out[0].address, 0x400000);
        assert_eq!(out[2].address, 0x40000A); // add rax,rcx
        assert_eq!(out[2].text, "add rax,rcx");
        assert_eq!(out[5].address, 0x400012); // hlt
        assert_eq!(out[5].text, "hlt");
    }
}

/// Les champs d'une instruction déjà décodée, dans les octets qui la
/// portent.
///
/// C'est la forme qu'appelle l'interface : elle tient une `Instruction`
/// rendue par [`decode_one`] et la fenêtre d'octets d'où elle vient, et
/// n'a pas à savoir compter les immédiats.
pub fn fields_of(instr: &Instruction, window: &[u8]) -> Vec<(std::ops::Range<usize>, Field)> {
    let len = instr.len().min(window.len());
    fields(&window[..len], immediate_len(instr))
}

/// La taille, en octets, de la valeur écrite en clair dans
/// l'instruction. Zéro quand il n'y en a pas.
fn immediate_len(instr: &Instruction) -> usize {
    use iced_x86::OpKind;
    (0..instr.op_count())
        .map(|index| match instr.op_kind(index) {
            OpKind::Immediate8
            | OpKind::Immediate8_2nd
            | OpKind::Immediate8to16
            | OpKind::Immediate8to32
            | OpKind::Immediate8to64 => 1,
            OpKind::Immediate16 => 2,
            OpKind::Immediate32 | OpKind::Immediate32to64 => 4,
            OpKind::Immediate64 => 8,
            _ => 0,
        })
        .sum()
}

/// Le rôle d'un groupe d'octets dans une instruction encodée.
///
/// Une instruction x86 n'est pas un bloc : c'est une suite de champs,
/// chacun avec son travail. Les distinguer, c'est passer de « 48 89 d8 »
/// à « opérandes 64 bits, `mov` registre vers registre, RBX vers RAX ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Un préfixe, avec ce qu'il change.
    Prefix(Prefix),
    /// L'opcode : l'instruction elle-même, sur un à trois octets.
    OpCode,
    /// ModRM, SIB et le déplacement : *où* l'instruction va chercher ses
    /// opérandes.
    Addressing,
    /// La valeur écrite en clair dans l'instruction.
    Immediate,
}

/// Les préfixes qu'on rencontre en lisant du code 64 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefix {
    /// `REX.W` : l'instruction travaille sur 64 bits.
    RexW,
    /// Un REX sans W : il donne accès aux registres R8 à R15, et aux
    /// octets bas de RSI, RDI, RSP, RBP.
    Rex,
    /// `66` : opérandes de 16 bits.
    OperandSize,
    /// `67` : adresses de 32 bits.
    AddressSize,
    /// `F3` : `rep`, ou une instruction SSE à part entière.
    Rep,
    /// `F2` : `repne`, même remarque.
    Repne,
    /// `F0` : `lock`, l'opération devient atomique.
    Lock,
    /// Un préfixe de segment (`fs:`, `gs:`…).
    Segment,
    /// `C4`/`C5` : l'encodage VEX des instructions AVX.
    Vex,
}

/// Découpe les octets d'une instruction en champs.
///
/// Approximatif par construction sur un point : le milieu — ModRM, SIB,
/// déplacement — n'est pas décomposé plus finement, parce qu'il faudrait
/// pour cela la table des opcodes complète. Les préfixes, l'opcode et
/// l'immédiat, eux, se lisent sans table, et ce sont ceux qui expliquent
/// ce qu'on voit.
pub fn fields(bytes: &[u8], immediate_len: usize) -> Vec<(std::ops::Range<usize>, Field)> {
    let mut out = Vec::new();
    let mut at = 0usize;

    // Les préfixes viennent en premier, en nombre quelconque ; REX doit
    // être le dernier avant l'opcode.
    while at < bytes.len() {
        let prefix = match bytes[at] {
            0xf0 => Prefix::Lock,
            0xf2 => Prefix::Repne,
            0xf3 => Prefix::Rep,
            0x66 => Prefix::OperandSize,
            0x67 => Prefix::AddressSize,
            0x2e | 0x36 | 0x3e | 0x26 | 0x64 | 0x65 => Prefix::Segment,
            b @ 0x40..=0x4f => {
                // Le bit W est celui de poids 3 : c'est lui qui fait
                // passer l'instruction en 64 bits.
                let kind = if b & 0x08 != 0 {
                    Prefix::RexW
                } else {
                    Prefix::Rex
                };
                out.push((at..at + 1, Field::Prefix(kind)));
                at += 1;
                break;
            }
            0xc5 => {
                // VEX court : deux octets, opcode compris dans la suite.
                out.push((at..at + 2, Field::Prefix(Prefix::Vex)));
                at += 2;
                break;
            }
            0xc4 => {
                out.push((at..at + 3, Field::Prefix(Prefix::Vex)));
                at += 3;
                break;
            }
            _ => break,
        };
        out.push((at..at + 1, Field::Prefix(prefix)));
        at += 1;
    }

    // L'opcode : un octet, ou deux à trois quand il commence par 0F.
    let opcode_start = at;
    if bytes.get(at) == Some(&0x0f) {
        at += 1;
        if matches!(bytes.get(at), Some(0x38) | Some(0x3a)) {
            at += 1;
        }
    }
    if at < bytes.len() {
        at += 1;
    }
    if at > opcode_start {
        out.push((opcode_start..at, Field::OpCode));
    }

    // L'immédiat se prend par la fin : sa longueur est connue, et ce qui
    // reste entre l'opcode et lui est l'adressage.
    let immediate_start = bytes.len().saturating_sub(immediate_len);
    if immediate_len > 0 && immediate_start >= at {
        if immediate_start > at {
            out.push((at..immediate_start, Field::Addressing));
        }
        out.push((immediate_start..bytes.len(), Field::Immediate));
    } else if at < bytes.len() {
        out.push((at..bytes.len(), Field::Addressing));
    }
    out
}

#[cfg(test)]
mod encoding_tests {
    use super::*;

    /// Les cas qu'on rencontre dès le premier programme.
    #[test]
    fn the_first_instructions_split_into_their_fields() {
        // 48 89 d8 : REX.W, opcode 89, ModRM d8.
        assert_eq!(
            fields(&[0x48, 0x89, 0xd8], 0),
            vec![
                (0..1, Field::Prefix(Prefix::RexW)),
                (1..2, Field::OpCode),
                (2..3, Field::Addressing),
            ]
        );
        // b8 2a 00 00 00 : opcode seul, puis quatre octets d'immédiat.
        assert_eq!(
            fields(&[0xb8, 0x2a, 0x00, 0x00, 0x00], 4),
            vec![(0..1, Field::OpCode), (1..5, Field::Immediate)]
        );
        // 0f 05 : `syscall`, opcode sur deux octets, rien d'autre.
        assert_eq!(fields(&[0x0f, 0x05], 0), vec![(0..2, Field::OpCode)]);
    }

    /// Un REX sans W ne dit pas la même chose qu'un REX.W : le premier
    /// ouvre les registres R8 à R15, le second passe en 64 bits.
    #[test]
    fn rex_with_and_without_w_are_different_things() {
        assert_eq!(
            fields(&[0x41, 0x89, 0xc0], 0)[0],
            (0..1, Field::Prefix(Prefix::Rex))
        );
        assert_eq!(
            fields(&[0x49, 0x89, 0xc0], 0)[0],
            (0..1, Field::Prefix(Prefix::RexW))
        );
    }

    /// Un préfixe de répétition, puis l'opcode : `rep movsb`.
    #[test]
    fn a_repeat_prefix_is_told_from_the_opcode() {
        assert_eq!(
            fields(&[0xf3, 0xa4], 0),
            vec![(0..1, Field::Prefix(Prefix::Rep)), (1..2, Field::OpCode)]
        );
    }

    /// Des octets vides ne font pas paniquer : le découpage d'une
    /// instruction tronquée rend ce qu'il peut.
    #[test]
    fn nothing_in_nothing_out() {
        assert!(fields(&[], 0).is_empty());
        assert_eq!(
            fields(&[0x48], 0),
            vec![(0..1, Field::Prefix(Prefix::RexW))]
        );
    }
}

/// Ce mot est-il une instruction que le processeur connaît ?
///
/// La liste vient d'iced-x86 — le décodeur que namsdec emploie déjà —,
/// et non d'un tableau écrit à la main. C'est ce qui empêche les deux de
/// diverger : ce qui se décode se colore, et réciproquement.
///
/// S'y ajoutent les préfixes, que NASM écrit comme un premier mot
/// (`rep movsb`, `lock xchg`) alors que le décodeur les porte sur
/// l'instruction elle-même.
pub fn is_mnemonic(word: &str) -> bool {
    use std::collections::HashSet;
    use std::sync::OnceLock;

    static CONNUS: OnceLock<HashSet<String>> = OnceLock::new();
    let connus = CONNUS.get_or_init(|| {
        let mut set: HashSet<String> = Mnemonic::values()
            .filter(|m| *m != Mnemonic::INVALID)
            .map(|m| format!("{m:?}").to_ascii_lowercase())
            .collect();
        // Les préfixes : le décodeur les range à part, l'assembleur les
        // écrit devant. Sans eux, `rep movsb` verrait son premier mot
        // signalé comme inconnu.
        for préfixe in [
            "rep", "repe", "repz", "repne", "repnz", "lock", "xacquire", "xrelease", "bnd",
            "notrack",
        ] {
            set.insert(préfixe.to_owned());
        }
        // Les synonymes de condition. `jne` et `jnz` sont le **même**
        // octet : la première orthographe parle du résultat d'un `cmp`
        // (« pas égal »), la seconde du drapeau qui le porte (« zéro pas
        // mis »). iced n'en nomme qu'une, NASM accepte les deux, et
        // signaler `jnz` reviendrait à reprocher à quelqu'un d'écrire
        // l'assembleur qu'on lui a appris.
        //
        // Chaque groupe désigne une seule condition. Dès qu'une de ses
        // orthographes est connue du décodeur, toutes le deviennent.
        const CONDITIONS: &[&[&str]] = &[
            &["e", "z"],
            &["ne", "nz"],
            &["b", "c", "nae"],
            &["ae", "nb", "nc"],
            &["be", "na"],
            &["a", "nbe"],
            &["l", "nge"],
            &["ge", "nl"],
            &["le", "ng"],
            &["g", "nle"],
            &["p", "pe"],
            &["np", "po"],
            &["s"],
            &["ns"],
            &["o"],
            &["no"],
        ];
        for racine in ["j", "set", "cmov", "fcmov", "loope", "loopne"] {
            for groupe in CONDITIONS {
                let connu = groupe
                    .iter()
                    .any(|suffixe| set.contains(&format!("{racine}{suffixe}")));
                if connu {
                    for suffixe in *groupe {
                        set.insert(format!("{racine}{suffixe}"));
                    }
                }
            }
        }
        // Les comparaisons vectorielles, que NASM nomme par leur
        // condition. `cmpltpd` est un `cmppd` dont l'immédiat vaut 1 :
        // le décodeur ne connaît que la forme générale, l'assembleur
        // accepte les deux, et c'est la forme nommée qu'on écrit.
        for suffixe in ["ps", "pd", "ss", "sd"] {
            if !set.contains(&format!("cmp{suffixe}")) {
                continue;
            }
            for condition in ["eq", "lt", "le", "unord", "neq", "nlt", "nle", "ord"] {
                set.insert(format!("cmp{condition}{suffixe}"));
                set.insert(format!("vcmp{condition}{suffixe}"));
            }
        }
        set
    });
    connus.contains(&word.to_ascii_lowercase())
}

#[cfg(test)]
mod mnemonic_tests {
    use super::*;

    /// La coloration distinguait `mov` de `mav` : elle ne le fait plus.
    ///
    /// Peindre un mot inventé comme une instruction n'apprend rien et
    /// rassure à tort — c'est précisément la faute de frappe qu'on
    /// cherche des yeux avant de lancer l'assembleur.
    #[test]
    fn a_made_up_word_is_not_an_instruction() {
        assert!(is_mnemonic("mov"), "mov existe");
        assert!(!is_mnemonic("mav"), "mav n'existe pas");
        assert!(is_mnemonic("MOV"), "la casse ne compte pas");
    }

    /// Les préfixes comptent : NASM les écrit en premier mot.
    #[test]
    fn prefixes_count_as_a_first_word() {
        assert!(is_mnemonic("rep"), "`rep movsb` commence par `rep`");
        assert!(is_mnemonic("lock"));
    }

    /// Les deux orthographes d'une même condition sont acceptées.
    ///
    /// `jne` et `jnz` sont le même octet : l'une nomme le résultat d'un
    /// `cmp`, l'autre le drapeau qui le porte. Les exercices livrés
    /// emploient les deux, et ils ont raison.
    #[test]
    fn both_spellings_of_a_condition_are_known() {
        for paire in [
            ("jne", "jnz"),
            ("je", "jz"),
            ("setne", "setnz"),
            ("cmovae", "cmovnb"),
        ] {
            assert!(is_mnemonic(paire.0), "{} devrait être connu", paire.0);
            assert!(
                is_mnemonic(paire.1),
                "{} aussi : c'est le même octet",
                paire.1
            );
        }
        // Mais le synonyme d'une condition qui n'existe pas n'en devient
        // pas une pour autant.
        assert!(!is_mnemonic("jzz"));
    }

    /// Et la liste est bien celle du décodeur, pas un échantillon.
    #[test]
    fn the_list_is_the_decoders_own() {
        for instruction in ["syscall", "endbr64", "vpxor", "cmovne", "pshufb", "sarx"] {
            assert!(is_mnemonic(instruction), "{instruction} devrait être connu");
        }
    }
}

#[cfg(test)]
mod tests_syntaxe {
    use super::*;

    /// Les deux syntaxes disent la même chose, et ne se ressemblent pas.
    ///
    /// C'est tout l'intérêt du choix : devant un rapport d'`objdump` ou
    /// une session `gdb`, tout est écrit dans l'autre, et la traduction
    /// de tête est la première marche — celle où l'on abandonne.
    #[test]
    fn the_two_syntaxes_say_the_same_thing_differently() {
        // mov rax, [rbx+8]
        let octets: &[u8] = &[0x48, 0x8B, 0x43, 0x08];
        let nasm = &disassemble_in(octets, 0x1000, Syntax::Nasm)[0].text;
        let système = &disassemble_in(octets, 0x1000, Syntax::System)[0].text;

        assert!(nasm.contains("rax"), "{nasm}");
        assert!(
            nasm.contains('['),
            "NASM met l'adresse entre crochets : {nasm}"
        );
        assert!(
            système.contains("%rax"),
            "le système préfixe les registres : {système}"
        );
        assert!(
            système.contains('('),
            "et met l'adresse entre parenthèses : {système}"
        );
        assert_ne!(nasm, système, "les deux formes doivent différer");
    }

    /// L'ordre des opérandes s'inverse : c'est le piège qui coûte le plus
    /// cher quand on passe de l'une à l'autre.
    #[test]
    fn the_operands_swap_sides() {
        // mov eax, 1 — la destination est EAX dans les deux cas.
        let octets: &[u8] = &[0xB8, 0x01, 0x00, 0x00, 0x00];
        let nasm = &disassemble_in(octets, 0x1000, Syntax::Nasm)[0].text;
        let système = &disassemble_in(octets, 0x1000, Syntax::System)[0].text;
        // En NASM la destination vient en premier, dans le système en
        // dernier : la position de « eax » dans le texte le dit.
        let place = |t: &str| t.find("eax").unwrap_or(usize::MAX);
        assert!(
            place(nasm) < place(système),
            "NASM : {nasm}\nsystème : {système}"
        );
    }

    /// L'identifiant fait l'aller-retour : c'est ce qui part dans les
    /// réglages.
    #[test]
    fn the_identifier_survives_a_round_trip() {
        for syntaxe in [Syntax::Nasm, Syntax::System] {
            assert_eq!(Syntax::by_id(syntaxe.id()), Some(syntaxe));
        }
        assert_eq!(Syntax::by_id("att"), None, "un nom inconnu ne devine pas");
    }

    /// Le défaut est NASM, et ce n'est pas un hasard : c'est la seule que
    /// nasm sait assembler, donc la seule dans laquelle on peut écrire.
    #[test]
    fn the_default_is_the_one_we_can_write_in() {
        assert_eq!(Syntax::default(), Syntax::Nasm);
        let octets: &[u8] = &[0x48, 0x8B, 0x43, 0x08];
        assert_eq!(
            disassemble(octets, 0x1000)[0].text,
            disassemble_in(octets, 0x1000, Syntax::Nasm)[0].text
        );
    }
}
