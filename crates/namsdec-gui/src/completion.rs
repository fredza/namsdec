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
    "adc", "add", "and", "call", "cld", "cmova", "cmovae", "cmovb", "cmovbe", "cmove", "cmovg",
    "cmovge", "cmovl", "cmovle", "cmovne", "cmovnz", "cmovz", "cmp", "cmpsb", "cmpsd", "cmpsq",
    "cmpsw", "dec", "div", "hlt", "idiv", "imul", "inc", "ja", "jae", "jb", "jbe", "je", "jg",
    "jge", "jl", "jle", "jmp", "jna", "jnae", "jnb", "jnbe", "jne", "jng", "jnge", "jnl", "jnle",
    "jno", "jnp", "jns", "jnz", "jo", "jp", "js", "jz", "lea", "leave", "lodsb", "lodsd", "lodsq",
    "lodsw", "loop", "mov", "movsb", "movsd", "movsq", "movsw", "movsx", "movsxd", "movzx", "mul",
    "neg", "nop", "not", "or", "pop", "push", "rep", "repe", "repne", "repnz", "repz", "ret",
    "sal", "sar", "sbb", "scasb", "scasd", "scasq", "scasw", "seta", "setae", "setb", "setbe",
    "sete", "setg", "setge", "setl", "setle", "setne", "setnz", "setz", "shl", "shr", "std",
    "stosb", "stosd", "stosq", "stosw", "sub", "syscall", "test", "xchg", "xor",
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
    /// Ce qu'il reste à écrire pour accepter le premier candidat.
    pub fn completion(&self) -> Option<&str> {
        let first = self.candidates.first()?;
        first.text.get(self.prefix.len()..)
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

/// Les étiquettes définies dans la source : c'est ce qu'on écrit après un
/// `jmp` ou un `call`, et personne ne peut le deviner à notre place.
fn labels_of(source: &str) -> Vec<String> {
    let mut labels = Vec::new();
    for line in source.lines() {
        for (piece, token) in syntax::tokenize(line) {
            if token == Token::Label && !labels.iter().any(|known| known == piece) {
                labels.push(piece.to_owned());
            }
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
fn is_first_word(line: &str, word_start: usize) -> bool {
    let before = &line[..word_start];
    // Une étiquette et ses deux-points ne comptent pas : après
    // `boucle:`, on écrit bien une instruction.
    let before = before.rsplit(':').next().unwrap_or(before);
    before.trim().is_empty()
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
        assert!(proposed.contains(&"movsb".to_owned()));
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
        assert_eq!(&source[suggestions.start..], "mo");
        assert_eq!(suggestions.completion(), Some("v"), "« mo » + « v »");

        // La casse de ce qui est déjà tapé est respectée : on ne réécrit
        // pas ce que l'utilisateur a saisi.
        let suggestions = suggest("_start:\n    MO", 14).expect("des propositions");
        assert_eq!(suggestions.prefix, "MO");
        assert_eq!(suggestions.completion(), Some("v"));
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
