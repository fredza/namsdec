//! L'autocomplétion de l'éditeur.
//!
//! Ce module ne connaît ni egui ni l'application : il prend une source,
//! une position de curseur, et rend ce qu'on pourrait vouloir écrire là.
//! Tout ce qui décide est ici, et se teste sans écran.
//!
//! Ce qu'il propose dépend de *l'endroit* : au début d'une ligne on écrit
//! une instruction ou une directive, dans une opérande un registre ou une
//! étiquette du programme. Proposer les seize registres généraux après
//! `section` ne serait pas une aide, seulement du bruit à écarter.

use crate::syntax::{self, DIRECTIVES, PREPROCESSOR, REGISTERS, Token};

/// Les mnémoniques proposés. Contrairement à la coloration — qui
/// reconnaît un mnémonique à sa place et n'a donc pas de liste —, il faut
/// bien *nommer* ce qu'on propose. La liste couvre ce que l'interpréteur
/// de namsdec sait exécuter, plus ce qu'un cours écrit tôt : proposer une
/// instruction que le moteur refusera serait un mauvais service.
pub const MNEMONICS: &[&str] = &[
    "adc",
    "add",
    "addpd",
    "addps",
    "addsd",
    "addss",
    "and",
    "andn",
    "andnpd",
    "andnps",
    "andpd",
    "andps",
    "bextr",
    "blsi",
    "blsmsk",
    "blsr",
    "bsf",
    "bsr",
    "bswap",
    "bt",
    "btc",
    "btr",
    "bts",
    "bzhi",
    "call",
    "cbw",
    "cdq",
    "cdqe",
    "clc",
    "cld",
    "cmc",
    "cmova",
    "cmovae",
    "cmovb",
    "cmovbe",
    "cmove",
    "cmovg",
    "cmovge",
    "cmovl",
    "cmovle",
    "cmovne",
    "cmovno",
    "cmovnp",
    "cmovns",
    "cmovnz",
    "cmovo",
    "cmovp",
    "cmovs",
    "cmovz",
    "cmp",
    "cmpsb",
    "cmpsd",
    "cmpsq",
    "cmpsw",
    "cmpxchg",
    "comisd",
    "comiss",
    "cpuid",
    "cqo",
    "cvtsd2si",
    "cvtsd2ss",
    "cvtsi2sd",
    "cvtsi2ss",
    "cvtss2sd",
    "cvtss2si",
    "cvttsd2si",
    "cvttss2si",
    "cwd",
    "cwde",
    "dec",
    "div",
    "divpd",
    "divps",
    "divsd",
    "divss",
    "endbr64",
    "enter",
    "f2xm1",
    "fabs",
    "fadd",
    "faddp",
    "fchs",
    "fcom",
    "fcomi",
    "fcomip",
    "fcomp",
    "fcompp",
    "fcos",
    "fdecstp",
    "fdiv",
    "fdivp",
    "fdivr",
    "fdivrp",
    "ffree",
    "ffreep",
    "fiadd",
    "fidiv",
    "fidivr",
    "fild",
    "fimul",
    "fincstp",
    "finit",
    "fist",
    "fistp",
    "fisttp",
    "fisub",
    "fisubr",
    "fld",
    "fld1",
    "fldcw",
    "fldl2e",
    "fldl2t",
    "fldlg2",
    "fldln2",
    "fldpi",
    "fldz",
    "fmul",
    "fmulp",
    "fninit",
    "fnop",
    "fnstcw",
    "fnstsw",
    "fpatan",
    "fprem",
    "fprem1",
    "fptan",
    "frndint",
    "fscale",
    "fsin",
    "fsincos",
    "fsqrt",
    "fst",
    "fstcw",
    "fstp",
    "fstsw",
    "fsub",
    "fsubp",
    "fsubr",
    "fsubrp",
    "ftst",
    "fucom",
    "fucomi",
    "fucomip",
    "fucomp",
    "fucompp",
    "fwait",
    "fxam",
    "fxch",
    "fyl2x",
    "hlt",
    "idiv",
    "imul",
    "inc",
    "int3",
    "ja",
    "jae",
    "jb",
    "jbe",
    "jc",
    "je",
    "jecxz",
    "jg",
    "jge",
    "jl",
    "jle",
    "jmp",
    "jna",
    "jnae",
    "jnb",
    "jnbe",
    "jnc",
    "jne",
    "jng",
    "jnge",
    "jnl",
    "jnle",
    "jno",
    "jnp",
    "jns",
    "jnz",
    "jo",
    "jp",
    "jrcxz",
    "js",
    "jz",
    "lahf",
    "lea",
    "leave",
    "lfence",
    "lodsb",
    "lodsd",
    "lodsq",
    "lodsw",
    "loop",
    "loope",
    "loopne",
    "loopnz",
    "loopz",
    "lzcnt",
    "maxpd",
    "maxps",
    "maxsd",
    "maxss",
    "mfence",
    "minpd",
    "minps",
    "minsd",
    "minss",
    "mov",
    "movapd",
    "movaps",
    "movbe",
    "movd",
    "movdqa",
    "movdqu",
    "movhlps",
    "movlhps",
    "movmskpd",
    "movmskps",
    "movq",
    "movsb",
    "movsd",
    "movsq",
    "movss",
    "movsw",
    "movsx",
    "movsxd",
    "movupd",
    "movups",
    "movzx",
    "mul",
    "mulpd",
    "mulps",
    "mulsd",
    "mulss",
    "mulx",
    "neg",
    "nop",
    "not",
    "or",
    "orpd",
    "orps",
    "paddb",
    "paddd",
    "paddq",
    "paddw",
    "pand",
    "pandn",
    "pause",
    "pcmpeqb",
    "pcmpeqd",
    "pcmpeqw",
    "pcmpgtb",
    "pcmpgtd",
    "pcmpgtw",
    "pmovmskb",
    "pop",
    "popcnt",
    "popf",
    "popfq",
    "por",
    "pshufd",
    "pslld",
    "pslldq",
    "psllq",
    "psllw",
    "psrad",
    "psraw",
    "psrld",
    "psrldq",
    "psrlq",
    "psrlw",
    "psubb",
    "psubd",
    "psubq",
    "psubw",
    "punpckhbw",
    "punpckhdq",
    "punpckhqdq",
    "punpckhwd",
    "punpcklbw",
    "punpckldq",
    "punpcklqdq",
    "punpcklwd",
    "push",
    "pushf",
    "pushfq",
    "pxor",
    "rcl",
    "rcr",
    "rdtsc",
    "rdtscp",
    "rep",
    "repe",
    "repne",
    "repnz",
    "repz",
    "ret",
    "rol",
    "ror",
    "rorx",
    "sahf",
    "sal",
    "sar",
    "sarx",
    "sbb",
    "scasb",
    "scasd",
    "scasq",
    "scasw",
    "seta",
    "setae",
    "setb",
    "setbe",
    "setc",
    "sete",
    "setg",
    "setge",
    "setl",
    "setle",
    "setnc",
    "setne",
    "setno",
    "setnp",
    "setns",
    "setnz",
    "seto",
    "setp",
    "sets",
    "setz",
    "sfence",
    "shl",
    "shld",
    "shlx",
    "shr",
    "shrd",
    "shrx",
    "shufpd",
    "shufps",
    "sqrtpd",
    "sqrtps",
    "sqrtsd",
    "sqrtss",
    "stc",
    "std",
    "stosb",
    "stosd",
    "stosq",
    "stosw",
    "sub",
    "subpd",
    "subps",
    "subsd",
    "subss",
    "syscall",
    "test",
    "tzcnt",
    "ucomisd",
    "ucomiss",
    "ud2",
    "unpckhpd",
    "unpckhps",
    "unpcklpd",
    "unpcklps",
    "xadd",
    "xchg",
    "xlatb",
    "xor",
    "xorpd",
    "xorps",
];

/// Ce qu'on propose, et d'où ça vient — la provenance sert à colorer la
/// liste comme le code, pour qu'on la lise sans la déchiffrer.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Candidate {
    pub text: String,
    pub kind: Token,
}

/// Ce que l'éditeur doit savoir pour proposer quelque chose.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Suggestions {
    /// Le mot en cours de frappe, tel qu'il est écrit.
    pub prefix: String,
    /// Où il commence, en octets depuis le début de la source.
    pub start: usize,
    pub candidates: Vec<Candidate>,
}

impl Suggestions {
    /// Le candidat qu'`Alt` accepte.
    pub fn accepted(&self) -> Option<&str> {
        self.candidates.first().map(|first| first.text.as_str())
    }

    /// L'étendue à remplacer, en octets : le mot commencé, en entier.
    ///
    /// Remplacer et non compléter : une étiquette vient de la source
    /// telle qu'elle y est écrite, et NASM distingue les casses. Après
    /// avoir tapé `bo`, accepter `Boucle` doit écrire `Boucle` — ajouter
    /// « ucle » derrière « bo » donnerait `boucle`, une étiquette qui
    /// n'existe pas.
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.prefix.len()
    }
}

/// Combien de propositions au plus. Au-delà, la liste cesse d'aider :
/// personne ne lit vingt lignes pour choisir un registre.
pub const MAX_CANDIDATES: usize = 6;

/// Ce qu'on peut vouloir écrire à `cursor` (un décalage en octets).
///
/// Rend `None` quand il n'y a rien à proposer : pas de mot commencé,
/// curseur dans un commentaire ou dans une chaîne, mot déjà complet et
/// seul de son espèce.
pub fn suggest(source: &str, cursor: usize) -> Option<Suggestions> {
    let cursor = cursor.min(source.len());
    let line_start = source[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let line = &source[line_start..cursor];

    // Dans un commentaire ou une chaîne, on écrit du français, pas de
    // l'assembleur.
    if in_comment_or_string(line) {
        return None;
    }

    let prefix_start = line_start + start_of_word(line);
    let prefix = &source[prefix_start..cursor];
    if prefix.is_empty() {
        return None;
    }

    let first_word = is_first_word(line, start_of_word(line));
    let lower = prefix.to_ascii_lowercase();

    let mut candidates = Vec::new();
    if let Some(rest) = lower.strip_prefix('%') {
        // Le préprocesseur ne se propose qu'après son `%`.
        push_matching(
            &mut candidates,
            PREPROCESSOR,
            rest,
            Token::Preprocessor,
            "%",
        );
    } else if first_word {
        // En tête d'instruction : une instruction, ou un mot de NASM.
        push_matching(&mut candidates, MNEMONICS, &lower, Token::Mnemonic, "");
        push_matching(&mut candidates, DIRECTIVES, &lower, Token::Directive, "");
    } else {
        // Dans une opérande : un registre, ou une étiquette du programme.
        push_matching(&mut candidates, REGISTERS, &lower, Token::Register, "");
        let labels = labels_of(source);
        let borrowed: Vec<&str> = labels.iter().map(String::as_str).collect();
        push_matching(&mut candidates, &borrowed, &lower, Token::Label, "");
        // Les tailles d'opérande s'écrivent aussi là : `mov qword [rsi]`.
        push_matching(&mut candidates, DIRECTIVES, &lower, Token::Directive, "");
    }

    // L'ordre est celui des listes, et c'est **voulu** : les registres y
    // sont rangés comme on les enseigne — RAX, RBX, RCX, RDX, RSI, RDI
    // avant R8 à R15 —, les mnémoniques par ordre alphabétique, où le
    // mot nu précède ses variantes (`mov` avant `movsb`). Trier par
    // longueur paraissait plus malin et remontait `r10` devant `rax`.
    candidates.dedup_by(|a, b| a.text == b.text);
    candidates.truncate(MAX_CANDIDATES);

    // Rien à ajouter : le mot est déjà écrit en entier et rien d'autre ne
    // commence pareil. Proposer « mov » à quelqu'un qui vient de taper
    // « mov » n'aide personne.
    if candidates.len() == 1 && candidates[0].text.len() == prefix.len() {
        return None;
    }
    if candidates.is_empty() {
        return None;
    }

    Some(Suggestions {
        prefix: prefix.to_owned(),
        start: prefix_start,
        candidates,
    })
}

fn push_matching(out: &mut Vec<Candidate>, words: &[&str], prefix: &str, kind: Token, lead: &str) {
    for word in words {
        if word.len() >= prefix.len() && word.to_ascii_lowercase().starts_with(prefix) {
            out.push(Candidate {
                text: format!("{lead}{word}"),
                kind,
            });
        }
    }
}

/// Les étiquettes **définies** dans la source : c'est ce qu'on écrit
/// après un `jmp` ou un `call`, et personne ne peut le deviner à notre
/// place.
///
/// Définies, donc en tête de ligne et suivies de leurs deux-points. Tout
/// ce qui *ressemble* à une étiquette ne se saute pas : le `.text` de
/// `section .text` nomme une section, et proposer d'y sauter n'aiderait
/// personne.
fn labels_of(source: &str) -> Vec<String> {
    let mut labels = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let Some(colon) = trimmed.find(':') else {
            continue;
        };
        let name = &trimmed[..colon];
        let looks_like_a_label = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '$' | '@' | '?'));
        if looks_like_a_label && !labels.iter().any(|known| known == name) {
            labels.push(name.to_owned());
        }
    }
    labels
}

/// Le début du mot en cours, en octets depuis le début de la ligne.
fn start_of_word(line: &str) -> usize {
    let mut start = line.len();
    for (i, c) in line.char_indices().rev() {
        if c.is_alphanumeric() || matches!(c, '_' | '.' | '%' | '@' | '?') {
            start = i;
        } else {
            break;
        }
    }
    start
}

/// Le mot commencé est-il le premier de la ligne — hors étiquette ?
///
/// « Hors étiquette » veut dire : une définition d'étiquette en tête de
/// ligne, `boucle:`, après quoi on écrit bien une instruction. Pas
/// n'importe quels deux-points : ceux de `[fs:0x28]` n'ouvrent rien, et
/// s'en servir comme repère proposait des instructions au milieu d'une
/// opérande.
fn is_first_word(line: &str, word_start: usize) -> bool {
    let before = line[..word_start].trim_start();
    let Some(colon) = before.find(':') else {
        return before.is_empty();
    };
    let (name, rest) = before.split_at(colon);
    let is_label_definition = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '$' | '@' | '?'));
    is_label_definition && rest[1..].trim().is_empty()
}

/// Le curseur est-il dans un commentaire ou une chaîne ? Les morceaux de
/// la ligne le disent : le dernier d'entre eux contient le curseur.
fn in_comment_or_string(line: &str) -> bool {
    matches!(
        syntax::tokenize(line).last().map(|(_, token)| *token),
        Some(Token::Comment) | Some(Token::Str)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le curseur est à la fin de la source : c'est le cas courant, on
    /// tape.
    fn at_end(source: &str) -> Option<Suggestions> {
        suggest(source, source.len())
    }

    fn texts(source: &str) -> Vec<String> {
        at_end(source)
            .map(|s| s.candidates.into_iter().map(|c| c.text).collect())
            .unwrap_or_default()
    }

    /// En tête d'instruction, on propose des instructions — pas les seize
    /// registres, qui n'ont rien à y faire.
    #[test]
    fn the_first_word_of_a_line_suggests_instructions() {
        let proposed = texts("section .text\n_start:\n    mo");
        assert_eq!(
            proposed.first().map(String::as_str),
            Some("mov"),
            "le plus court d'abord"
        );
        // Le jeu SSE a beaucoup enrichi le préfixe « mo » : ce qui
        // compte est que la liste ne propose que des instructions, et
        // qu'elle tienne dans sa borne.
        assert!(
            proposed.iter().all(|c| MNEMONICS.contains(&c.as_str())),
            "{proposed:?}"
        );
        assert!(proposed.len() > 1, "{proposed:?}");
        assert!(
            !proposed.iter().any(|c| c == "rax"),
            "aucun registre en tête d'instruction : {proposed:?}"
        );
    }

    /// Dans une opérande, on propose des registres et les étiquettes que
    /// le programme définit — ce que personne ne peut deviner à notre
    /// place.
    #[test]
    fn an_operand_suggests_registers_and_the_program_s_own_labels() {
        let proposed = texts("_start:\n    mov r");
        assert!(proposed.contains(&"rax".to_owned()));
        assert!(proposed.contains(&"rsi".to_owned()));
        assert!(!proposed.iter().any(|c| c == "ret"), "{proposed:?}");

        let source = "boucle:\n    dec rcx\n    jnz bo";
        assert!(texts(source).contains(&"boucle".to_owned()));
        // Y compris les étiquettes locales.
        let source = ".suite:\n    jmp .su";
        assert!(texts(source).contains(&".suite".to_owned()));
    }

    /// Le préprocesseur ne se propose qu'après son `%` : sinon `define`
    /// polluerait chaque ligne commençant par un `d`.
    #[test]
    fn the_preprocessor_only_appears_after_a_percent_sign() {
        assert!(texts("%inc").contains(&"%include".to_owned()));
        assert!(texts("%def").contains(&"%define".to_owned()));
        let proposed = texts("    de");
        assert!(!proposed.iter().any(|c| c.starts_with('%')), "{proposed:?}");
    }

    /// On n'écrit pas de l'assembleur dans un commentaire ni dans une
    /// chaîne : rien ne doit y être proposé.
    #[test]
    fn nothing_is_suggested_inside_a_comment_or_a_string() {
        assert_eq!(at_end("    mov rax, 1  ; on met ra"), None);
        assert_eq!(at_end("msg db \"salut mo"), None);
        // Mais après la chaîne refermée, si.
        assert!(at_end("msg db \"salut\", 10\n    mo").is_some());
    }

    /// Ce qu'il reste à taper, et où : c'est tout ce que l'éditeur a
    /// besoin de savoir pour accepter une proposition.
    #[test]
    fn a_suggestion_says_what_is_left_to_type_and_where_it_starts() {
        let source = "_start:\n    mo";
        let suggestions = at_end(source).expect("des propositions");
        assert_eq!(suggestions.prefix, "mo");
        assert_eq!(&source[suggestions.range()], "mo", "le mot commencé");
        assert_eq!(suggestions.accepted(), Some("mov"));

        // C'est bien le mot *entier* qui est remplacé : le candidat porte
        // sa casse, et NASM distingue `Boucle` de `boucle`.
        let source = "Boucle:\n    dec rcx\n    jnz bo";
        let suggestions = at_end(source).expect("des propositions");
        assert_eq!(suggestions.accepted(), Some("Boucle"));
        assert_eq!(&source[suggestions.range()], "bo");
    }

    /// Un préfixe de segment n'ouvre pas une nouvelle instruction : les
    /// deux-points de `[fs:0x28]` ne sont pas ceux d'une étiquette, et
    /// s'en servir comme repère proposait `rep` et `ret` au milieu d'une
    /// opérande.
    #[test]
    fn a_segment_override_does_not_start_a_new_instruction() {
        let proposed = texts("    mov rax, [fs:r");
        assert!(proposed.contains(&"rax".to_owned()), "{proposed:?}");
        assert!(!proposed.iter().any(|c| c == "ret"), "{proposed:?}");
    }

    /// Seules les étiquettes *définies* se proposent comme cibles : le
    /// `.text` de `section .text` nomme une section, on n'y saute pas.
    #[test]
    fn only_defined_labels_are_offered_as_jump_targets() {
        let source = "section .text\n_start:\n    mov rax, [fs:0]\n.fin:\n    jmp ";
        let labels = labels_of(source);
        assert_eq!(
            labels,
            vec!["_start".to_owned(), ".fin".to_owned()],
            "{labels:?}"
        );
    }

    /// Rien à proposer : pas de mot commencé, ou un mot déjà complet et
    /// seul de son espèce.
    #[test]
    fn nothing_is_suggested_when_there_is_nothing_to_add() {
        assert_eq!(at_end(""), None);
        assert_eq!(at_end("    "), None);
        assert_eq!(at_end("    mov rax, "), None);
        // « syscall » est complet et unique : la liste n'apporterait rien.
        assert_eq!(at_end("    syscall"), None);
        // « mov », lui, reste proposé : « movzx » et « movsb » existent.
        assert!(at_end("    mov").is_some());
    }

    /// La liste reste courte : personne ne lit vingt lignes pour choisir
    /// un registre.
    #[test]
    fn the_list_stays_short_enough_to_read() {
        let suggestions = at_end("    mov r").expect("des propositions");
        assert!(suggestions.candidates.len() <= MAX_CANDIDATES);
    }
}
