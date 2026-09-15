//! Coloration syntaxique NASM.
//!
//! Un tokenizer par ligne, pas une grammaire : il n'y a rien à analyser
//! ici, seulement à *reconnaître* — et reconnaître suffit à colorer.
//!
//! Ce qu'il distingue, et pourquoi : les registres (la moitié d'une ligne
//! d'assembleur, autrefois de la couleur du texte ordinaire), les chaînes
//! avec leurs échappements, les nombres dans toutes les formes que NASM
//! accepte, les directives et le préprocesseur — ce qui s'adresse à
//! l'assembleur et non au processeur —, les étiquettes, locales
//! comprises, et les commentaires.
//!
//! Les mnémoniques ne sont volontairement **pas** une liste fermée. Le
//! premier mot d'une instruction est un mnémonique par position : un jeu
//! d'instructions s'allonge, et une liste finit toujours par mentir. Ce
//! qui est fermé, ce sont les listes qui le *peuvent* — les registres et
//! les mots de NASM — parce qu'elles sont finies par nature.

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, TextStyle, Ui};
use namsdec_theme::Theme;

/// Les registres x86-64. Liste close, et complète : c'est le processeur
/// qui la fixe, pas nous.
pub const REGISTERS: &[&str] = &[
    // 64 bits
    "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rsp", "rbp", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15", "rip", "rflags", // 32
    "eax", "ebx", "ecx", "edx", "esi", "edi", "esp", "ebp", "r8d", "r9d", "r10d", "r11d", "r12d",
    "r13d", "r14d", "r15d", "eip", // 16
    "ax", "bx", "cx", "dx", "si", "di", "sp", "bp", "r8w", "r9w", "r10w", "r11w", "r12w", "r13w",
    "r14w", "r15w", // 8
    "al", "bl", "cl", "dl", "ah", "bh", "ch", "dh", "sil", "dil", "spl", "bpl", "r8b", "r9b",
    "r10b", "r11b", "r12b", "r13b", "r14b", "r15b", // segments
    "cs", "ds", "es", "fs", "gs", "ss", // flottants et vectoriels
    "st0", "st1", "st2", "st3", "st4", "st5", "st6", "st7", "xmm0", "xmm1", "xmm2", "xmm3", "xmm4",
    "xmm5", "xmm6", "xmm7", "xmm8", "xmm9", "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15",
    "ymm0", "ymm1", "ymm2", "ymm3", "ymm4", "ymm5", "ymm6", "ymm7",
];

/// Les mots que NASM comprend lui-même : directives, tailles, opérateurs
/// de section. Ils ne s'adressent pas au processeur.
pub const DIRECTIVES: &[&str] = &[
    "bits", "section", "segment", "global", "extern", "common", "static", "default", "org",
    "align", "alignb", "absolute", "group", "import", "export", "cpu", "float", "use16", "use32",
    "use64", // définition de données
    "db", "dw", "dd", "dq", "dt", "do", "dy", "dz", "resb", "resw", "resd", "resq", "rest", "reso",
    "resy", "resz", "incbin", "equ", "times", // tailles et modificateurs d'opérande
    "byte", "word", "dword", "qword", "tword", "oword", "yword", "zword", "ptr", "near", "far",
    "short", "strict", "nosplit", "rel", "abs", "wrt",
];

/// Les mots du préprocesseur, sans leur `%`.
pub const PREPROCESSOR: &[&str] = &[
    "include", "define", "undef", "assign", "xdefine", "idefine", "macro", "endmacro", "imacro",
    "rmacro", "unmacro", "if", "ifdef", "ifndef", "ifmacro", "ifidn", "ifidni", "ifnum", "ifstr",
    "ifid", "elif", "elifdef", "elifndef", "else", "endif", "rep", "endrep", "exitrep", "error",
    "warning", "fatal", "line", "push", "pop", "repl", "use", "pragma", "strlen", "substr",
    "defstr", "deftok",
];

/// Ce qu'un mot désigne. Sert à la coloration, et sert aussi à
/// l'autocomplétion, qui propose ces mêmes familles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Token {
    Mnemonic,
    Register,
    Directive,
    Preprocessor,
    Number,
    Str,
    Label,
    Comment,
    Plain,
}

impl Token {
    pub fn colour(self, theme: &Theme) -> Color32 {
        match self {
            Token::Mnemonic => theme.mnemonic,
            Token::Register => theme.register,
            Token::Directive | Token::Preprocessor => theme.directive,
            Token::Number => theme.number,
            Token::Str => theme.string,
            Token::Label => theme.label,
            Token::Comment => theme.comment,
            Token::Plain => theme.text,
        }
    }
}

/// Les couleurs viennent du thème actif (`namsdec-theme`) et non du
/// `dark_mode` d'egui : un thème installé plus tard apporte sa propre
/// palette de code sans qu'il y ait ici deux branches à rallonger.
pub fn highlight(ui: &Ui, theme: &Theme, text: &str, wrap_width: f32) -> LayoutJob {
    let font_id = TextStyle::Monospace.resolve(ui.style());

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            append(&mut job, "\n", &font_id, theme.text, false);
        }
        for (piece, token) in tokenize(line) {
            append(
                &mut job,
                piece,
                &font_id,
                token.colour(theme),
                token == Token::Comment,
            );
        }
    }

    job
}

/// Découpe une ligne en morceaux colorables, dans l'ordre. La
/// concaténation des morceaux redonne la ligne, exactement — c'est ce
/// qui garantit que la coloration ne déplace jamais un caractère.
pub fn tokenize(line: &str) -> Vec<(&str, Token)> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    // Le premier mot d'une instruction est un mnémonique — sauf s'il est
    // un mot de NASM. Une étiquette en tête de ligne ne consomme pas ce
    // droit : `boucle: add rax, 1` a bien un mnémonique.
    let mut first_word = true;

    while i < line.len() {
        let c = line[i..].chars().next().expect("i est sur une frontière");
        match c {
            // Le commentaire prend tout le reste de la ligne.
            ';' => {
                out.push((&line[i..], Token::Comment));
                break;
            }
            '"' | '\'' | '`' => {
                let start = i;
                i = end_of_string(bytes, i);
                out.push((&line[start..i], Token::Str));
                first_word = false;
            }
            c if c.is_whitespace() => {
                let start = i;
                i = scan(line, i, |c| c.is_whitespace());
                out.push((&line[start..i], Token::Plain));
            }
            // Le préprocesseur : `%include`, `%%étiquette`, `%1`.
            '%' => {
                let start = i;
                i = scan(line, i + 1, is_word_char);
                out.push((&line[start..i], Token::Preprocessor));
                first_word = false;
            }
            '$' => {
                // `$` est l'adresse courante, `$$` le début de section :
                // des nombres, pas des noms.
                let start = i;
                i = scan(line, i, |c| c == '$' || is_word_char(c));
                out.push((&line[start..i], Token::Number));
                first_word = false;
            }
            c if c.is_ascii_digit() => {
                let start = i;
                i = scan(line, i, is_number_char);
                out.push((&line[start..i], Token::Number));
                first_word = false;
            }
            c if is_word_start(c) => {
                let start = i;
                i = scan(line, i, is_word_char);
                let word = &line[start..i];
                let followed_by_colon = bytes.get(i) == Some(&b':');
                let token = classify(word, first_word, followed_by_colon);
                out.push((word, token));
                // Une étiquette ne consomme pas la place du mnémonique.
                first_word = matches!(token, Token::Label);
            }
            // Un caractère quelconque — y compris hors ASCII : on avance
            // d'un *caractère* et non d'un octet, sinon la découpe
            // tomberait au milieu d'un « é ».
            other => {
                let len = other.len_utf8();
                out.push((&line[i..i + len], Token::Plain));
                i += len;
            }
        }
    }
    out
}

/// À quelle famille appartient un mot, à sa place dans la ligne.
fn classify(word: &str, first_word: bool, followed_by_colon: bool) -> Token {
    let lower = word.to_ascii_lowercase();
    if followed_by_colon || word.starts_with('.') {
        // `.boucle` est une étiquette locale, même sans deux-points quand
        // on y saute : `jmp .boucle`.
        return Token::Label;
    }
    if REGISTERS.contains(&lower.as_str()) {
        return Token::Register;
    }
    if DIRECTIVES.contains(&lower.as_str()) {
        return Token::Directive;
    }
    if first_word {
        // Par position : un jeu d'instructions s'allonge, une liste
        // fermée finirait par mentir.
        return Token::Mnemonic;
    }
    Token::Plain
}

/// Où se termine une chaîne ouverte en `i`, échappements compris. Une
/// chaîne non refermée court jusqu'au bout de la ligne — c'est ce que
/// nasm lui reprochera, et l'afficher entière vaut mieux que de colorer
/// la suite comme du code.
fn end_of_string(bytes: &[u8], i: usize) -> usize {
    let quote = bytes[i];
    let mut j = i + 1;
    while j < bytes.len() {
        // Seuls les guillemets doubles et les accents graves connaissent
        // les échappements en NASM ; l'apostrophe prend tout au pied de
        // la lettre.
        if bytes[j] == b'\\' && quote != b'\'' && j + 1 < bytes.len() {
            j += 2;
            continue;
        }
        if bytes[j] == quote {
            return j + 1;
        }
        j += 1;
    }
    bytes.len()
}

/// Avance tant que le prédicat tient, en caractères et non en octets.
/// Rend l'indice d'octet où l'on s'arrête, toujours sur une frontière.
fn scan(line: &str, from: usize, keep: impl Fn(char) -> bool) -> usize {
    let mut i = from;
    for c in line[from..].chars() {
        if !keep(c) {
            break;
        }
        i += c.len_utf8();
    }
    i
}

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || matches!(c, '_' | '.' | '?' | '@')
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.' | '?' | '@' | '$' | '#' | '~')
}

/// Un nombre NASM : `42`, `0x1f`, `1Fh`, `0b1010`, `1010b`, `0o17`, `17q`,
/// `1_000`, `1.5e3`.
fn is_number_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.')
}

fn append(job: &mut LayoutJob, text: &str, font_id: &FontId, color: Color32, italics: bool) {
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color,
            italics,
            ..Default::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(line: &str) -> Vec<(&str, Token)> {
        tokenize(line)
    }

    /// La règle qui compte avant toutes les autres : colorer ne doit
    /// jamais déplacer un caractère. La concaténation des morceaux
    /// redonne la ligne, quoi qu'elle contienne.
    #[test]
    fn the_pieces_always_reassemble_into_the_line() {
        for line in [
            "    mov rax, [rsi + rcx*8]   ; commentaire",
            "msg db \"salut\\\"\", 10, 'a', `b`",
            "%macro sauve 1",
            ".boucle:  dec rcx",
            "",
            "   ",
            ";;;",
            "db 0x1f, 1Fh, 0b1010, 1010b, 17q, 1_000",
            "étiquette_accentuée: nop",
        ] {
            let rebuilt: String = tokenize(line).iter().map(|(piece, _)| *piece).collect();
            assert_eq!(rebuilt, line, "ligne « {line} »");
        }
    }

    /// Les registres ont enfin une couleur : ils forment la moitié d'une
    /// ligne d'assembleur et passaient pour du texte ordinaire.
    #[test]
    fn registers_are_recognised_in_every_width() {
        let line = tokens("    mov rax, cl");
        assert!(line.contains(&("rax", Token::Register)));
        assert!(line.contains(&("cl", Token::Register)));
        assert!(line.contains(&("mov", Token::Mnemonic)));

        // Y compris dans une adresse indexée, et quelle que soit la casse.
        let line = tokens("    mov RAX, [rsi + RCX*8]");
        assert!(line.contains(&("RAX", Token::Register)));
        assert!(line.contains(&("rsi", Token::Register)));
        assert!(line.contains(&("RCX", Token::Register)));
        assert!(line.contains(&("8", Token::Number)));
    }

    /// Les mnémoniques se reconnaissent à leur *place*, pas à une liste :
    /// celle-ci finirait par mentir à chaque instruction ajoutée.
    #[test]
    fn a_mnemonic_is_the_first_word_whatever_it_is() {
        assert!(tokens("    cpuid").contains(&("cpuid", Token::Mnemonic)));
        assert!(tokens("    vpxor xmm0, xmm0, xmm0").contains(&("vpxor", Token::Mnemonic)));
        // Même après une étiquette, qui ne prend pas sa place.
        let line = tokens("boucle: add rax, 1");
        assert!(line.contains(&("boucle", Token::Label)));
        assert!(line.contains(&("add", Token::Mnemonic)));
        // Mais un mot de NASM reste une directive, où qu'il soit.
        assert!(tokens("section .text").contains(&("section", Token::Directive)));
        assert!(tokens("    db 1").contains(&("db", Token::Directive)));
        assert!(tokens("    mov rax, qword [rsi]").contains(&("qword", Token::Directive)));
    }

    /// Chaînes, échappements, apostrophes : `db "a;b"` ne contient pas de
    /// commentaire, et c'était le contre-exemple du tokenizer précédent.
    #[test]
    fn a_semicolon_inside_a_string_is_not_a_comment() {
        let line = tokens("msg db \"a;b\", 10   ; vrai commentaire");
        assert!(line.contains(&("\"a;b\"", Token::Str)));
        assert!(line.contains(&("; vrai commentaire", Token::Comment)));

        // L'apostrophe ne connaît pas les échappements, le guillemet si.
        assert!(tokens("db 'a\\'").contains(&("'a\\'", Token::Str)));
        assert!(tokens("db \"a\\\"b\"").contains(&("\"a\\\"b\"", Token::Str)));
        // Une chaîne non refermée prend le reste de la ligne plutôt que
        // de faire passer la suite pour du code.
        assert_eq!(tokens("db \"oubli").last().unwrap().1, Token::Str);
    }

    /// Le préprocesseur et les étiquettes locales, que NASM écrit d'une
    /// façon bien à lui.
    #[test]
    fn preprocessor_and_local_labels_have_their_own_colour() {
        assert!(tokens("%include \"asmstd.inc\"").contains(&("%include", Token::Preprocessor)));
        assert!(tokens("%define TAILLE 64").contains(&("%define", Token::Preprocessor)));
        assert!(tokens("    jmp .boucle").contains(&(".boucle", Token::Label)));
        assert!(tokens(".boucle:").contains(&(".boucle", Token::Label)));
        // `$` et `$$` sont des nombres — l'adresse courante, le début de
        // section — et non des noms.
        assert!(tokens("longueur equ $ - msg").contains(&("$", Token::Number)));
        assert!(tokens("taille equ $$ - 1").contains(&("$$", Token::Number)));
    }

    /// Toutes les écritures de nombres que NASM accepte.
    #[test]
    fn numbers_are_recognised_in_every_nasm_spelling() {
        for text in [
            "42", "0x1f", "1Fh", "0b1010", "1010b", "0o17", "17q", "1_000", "1.5e3",
        ] {
            let line = format!("    mov rax, {text}");
            assert!(
                tokenize(&line)
                    .iter()
                    .any(|(piece, token)| { *piece == text && *token == Token::Number }),
                "« {text} » doit être un nombre"
            );
        }
    }
}
