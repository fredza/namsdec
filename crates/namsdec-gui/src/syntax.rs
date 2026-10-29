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
    "ymm0", "ymm1", "ymm2", "ymm3", "ymm4", "ymm5", "ymm6", "ymm7", "ymm8", "ymm9", "ymm10",
    "ymm11", "ymm12", "ymm13", "ymm14", "ymm15",
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
    // Les structures : une déclaration de disposition, pas du code.
    "struc", "endstruc", "istruc", "iend", "at",
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
    /// Un premier mot qui n'est aucune instruction connue.
    ///
    /// `mav` au lieu de `mov` : la faute de frappe qu'on cherche des
    /// yeux avant de lancer l'assembleur. La peindre comme une
    /// instruction ne l'apprenait pas — elle la cachait.
    UnknownMnemonic,
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
            // La couleur des ennuis, et rien d'autre : l'oeil s'arrête
            // dessus sans avoir à lire.
            Token::UnknownMnemonic => theme.bad,
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

/// Au-delà de combien d'octets un texte n'est plus coloré d'un bloc.
///
/// LA COULEUR SE PAIE À LA POSE, et non à la coloration. Mesuré le
/// 28 octobre 2026 sur deux cent mille lignes : colorer coûte 437 ms,
/// mais le job qui en sort porte 1 200 001 SECTIONS — six par ligne,
/// une par morceau coloré —, et egui met 2,46 s à les poser. Le même
/// texte en une seule section se pose en 1,13 s. La couleur coûte donc
/// 1,77 s sur 2,9, c'est-à-dire la plus grande part du gel qu'on
/// mesurait en entrant en Insertion.
///
/// Quatre cent mille octets : de quoi couvrir tout ce qu'on écrit à la
/// main et les désassemblages ordinaires. Au-delà, on est devant un
/// fichier qu'on ne lit pas d'un bout à l'autre, et la vue ligne à
/// ligne du mode Normal, elle, garde ses couleurs — elle ne pose que
/// ce qu'elle montre.
pub const SANS_COULEUR_AU_DELÀ: usize = 400_000;

/// Ce texte est-il trop gros pour être coloré d'un bloc ?
pub fn trop_gros_pour_la_couleur(text: &str) -> bool {
    text.len() > SANS_COULEUR_AU_DELÀ
}

/// Les couleurs viennent du thème actif (`namsdec-theme`) et non du
/// `dark_mode` d'egui : un thème installé plus tard apporte sa propre
/// palette de code sans qu'il y ait ici deux branches à rallonger.
pub fn highlight(ui: &Ui, theme: &Theme, text: &str, wrap_width: f32) -> LayoutJob {
    let font_id = TextStyle::Monospace.resolve(ui.style());

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    // UNE SEULE SECTION quand le texte est énorme : voir
    // `SANS_COULEUR_AU_DELÀ`, qui porte la mesure. Ce n'est pas la
    // coloration qu'on épargne, c'est la POSE des six sections par
    // ligne qu'elle produit.
    if trop_gros_pour_la_couleur(text) {
        append(&mut job, text, &font_id, theme.text, false);
        return job;
    }

    // Relevées une fois pour tout le fichier : une macro définie en bas
    // s'emploie en haut, et colorer ligne à ligne sans les connaître
    // signalerait ses appels comme des mots inventés.
    let macros = Macros::of(text);
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            append(&mut job, "\n", &font_id, theme.text, false);
        }
        append_line(&mut job, line, &font_id, theme, &macros);
    }

    job
}

/// La coloration d'UNE ligne, avec ce que le fichier entier a appris.
///
/// La vue qui ne pose que les lignes visibles en a besoin ligne par
/// ligne (voir `vue_listing`). Les règles restent ici, et une seule
/// fois : deux colorations qui dévieraient l'une de l'autre se verraient
/// au changement de mode, le même code changeant de couleur sous les
/// yeux.
pub fn highlight_line(ui: &Ui, theme: &Theme, line: &str, macros: &Macros) -> LayoutJob {
    let font_id = TextStyle::Monospace.resolve(ui.style());
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    append_line(&mut job, line, &font_id, theme, macros);
    job
}

/// Ajoute une ligne colorée au travail de mise en page.
fn append_line(job: &mut LayoutJob, line: &str, font_id: &FontId, theme: &Theme, macros: &Macros) {
    for (piece, token) in tokenize_with(line, macros) {
        append(
            job,
            piece,
            font_id,
            token.colour(theme),
            token == Token::Comment,
        );
    }
}

/// Découpe une ligne en morceaux colorables, dans l'ordre. La
/// concaténation des morceaux redonne la ligne, exactement — c'est ce
/// qui garantit que la coloration ne déplace jamais un caractère.
pub fn tokenize(line: &str) -> Vec<(&str, Token)> {
    tokenize_with(line, &Macros::default())
}

/// Ce que le fichier définit lui-même, relevé avant de le colorer.
///
/// Deux listes, pour deux raisons voisines :
///
/// - **les macros**. Une macro s'invoque comme une instruction —
///   `ajoute_deux_fois DIX` — et n'en est pas une. Sans cette liste,
///   chaque appel se voyait signalé comme un mot inventé, ce qui est le
///   contraire du but : on ne signale une faute de frappe que si l'on ne
///   crie pas sur le reste.
/// - **les étiquettes**. `boucle:` était bien peinte en étiquette là où
///   elle est définie, et pas du tout dans `jnz boucle`, où elle passait
///   pour un mot quelconque. Or c'est justement là qu'on a besoin de la
///   reconnaître : lire `jnz boucle` et savoir d'un coup d'oeil que
///   `boucle` est un endroit de *ce* fichier — et non un registre, un
///   nombre ou une faute — est la moitié du travail de lecture.
#[derive(Default, Clone)]
pub struct Macros {
    noms: std::collections::HashSet<String>,
    /// Les étiquettes définies, à leur casse exacte : NASM distingue
    /// `Boucle` de `boucle`, et les confondre lierait le mauvais symbole.
    étiquettes: std::collections::HashSet<String>,
}

impl Macros {
    /// Les noms définis par `%macro`, `%imacro` et leurs cousins, relevés
    /// dans un fichier entier.
    pub fn of(text: &str) -> Self {
        let mut noms = std::collections::HashSet::new();
        for ligne in text.lines() {
            let ligne = ligne.trim_start();
            let Some(reste) = ligne.strip_prefix('%') else {
                continue;
            };
            let (mot, suite) = match reste.find(char::is_whitespace) {
                Some(i) => (&reste[..i], &reste[i..]),
                None => continue,
            };
            // `%macro nom 1` définit ; `%define nom …` aussi, et les deux
            // s'écrivent ensuite en tête de ligne.
            if !matches!(
                mot.to_ascii_lowercase().as_str(),
                "macro" | "imacro" | "rmacro" | "irmacro" | "define" | "xdefine" | "idefine"
            ) {
                continue;
            }
            let nom = suite.trim_start();
            let fin = nom.find(|c: char| !is_word_char(c)).unwrap_or(nom.len());
            if fin > 0 {
                noms.insert(nom[..fin].to_ascii_lowercase());
            }
        }
        Self {
            noms,
            étiquettes: labels_of(text),
        }
    }

    fn contient(&self, mot: &str) -> bool {
        self.noms.contains(&mot.to_ascii_lowercase())
    }

    /// Ce mot est-il une étiquette définie dans le fichier ?
    ///
    /// À la casse près : NASM distingue `Boucle` de `boucle`, et sauter
    /// à l'une en croyant l'autre est une erreur de liaison, non une
    /// coquille d'affichage.
    fn est_une_étiquette(&self, mot: &str) -> bool {
        self.étiquettes.contains(mot)
    }
}

/// Les noms **définis** par le fichier : étiquettes et données.
///
/// Deux formes, et il faut les deux :
///
/// - `boucle:` — une étiquette, avec ses deux-points ;
/// - `nom db "fred"` — une donnée, que NASM laisse écrire sans
///   deux-points, et qui est tout autant un nom auquel on se réfère.
///
/// La seconde manquait, et c'est elle qu'on emploie le plus : un
/// programme déclare ses chaînes en tête, puis les nomme partout. Les
/// ignorer revenait à ne pas proposer ce qu'on écrit le plus souvent.
///
/// Définis seulement : tout ce qui ressemble à un nom n'en est pas un.
/// Le `.text` de `section .text` nomme une section, et le peindre en
/// étiquette laisserait croire qu'on peut y sauter.
pub(crate) fn labels_of(text: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for ligne in text.lines() {
        let taillée = ligne.trim_start();
        // La forme avec deux-points.
        if let Some(deux_points) = taillée.find(':') {
            let nom = &taillée[..deux_points];
            if !nom.is_empty() && nom.chars().all(is_word_char) {
                out.insert(nom.to_owned());
                continue;
            }
        }
        // La forme sans : un nom, puis une directive qui pose des
        // octets. C'est ce qui distingue `nom db "fred"` de
        // `mov rax, 1` — le second mot, et lui seul.
        let fin = taillée
            .find(|c: char| !is_word_char(c))
            .unwrap_or(taillée.len());
        if fin == 0 {
            continue;
        }
        let nom = &taillée[..fin];
        if next_word_defines_data(taillée, fin) {
            out.insert(nom.to_owned());
        }
    }
    out
}

/// Comme `tokenize`, en sachant quelles macros le fichier définit.
pub fn tokenize_with<'a>(line: &'a str, macros: &Macros) -> Vec<(&'a str, Token)> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    // Le premier mot d'une instruction est un mnémonique — sauf s'il est
    // un mot de NASM. Une étiquette *en tête de ligne* ne consomme pas ce
    // droit : `boucle: add rax, 1` a bien un mnémonique. Une étiquette
    // employée comme opérande, elle, n'ouvre rien : dans `db .foo, bar`,
    // `bar` n'est pas une instruction.
    let mut first_word = true;
    let mut at_line_start = true;

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
                at_line_start = false;
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
                at_line_start = false;
            }
            '$' => {
                // `$` est l'adresse courante, `$$` le début de section :
                // des nombres, pas des noms.
                let start = i;
                i = scan(line, i, |c| c == '$' || is_word_char(c));
                out.push((&line[start..i], Token::Number));
                first_word = false;
                at_line_start = false;
            }
            c if c.is_ascii_digit() => {
                let start = i;
                i = scan(line, i, is_number_char);
                out.push((&line[start..i], Token::Number));
                first_word = false;
                at_line_start = false;
            }
            c if is_word_start(c) => {
                let start = i;
                i = scan(line, i, is_word_char);
                let word = &line[start..i];
                let followed_by_colon = bytes.get(i) == Some(&b':');
                // `message db "salut"` : NASM autorise l'étiquette sans
                // deux-points quand une directive de données suit. Sans
                // ce regard en avant, « message » occupait la place du
                // mnémonique et se voyait signalé comme une instruction
                // inventée — sur du code parfaitement juste.
                let followed_by_data = first_word && next_word_defines_data(line, i);
                let token = classify(
                    word,
                    first_word && !followed_by_data,
                    followed_by_colon,
                    macros,
                );
                out.push((word, token));
                // Une étiquette en tête de ligne ne consomme pas la place
                // du mnémonique ; ailleurs, elle est une opérande comme
                // une autre.
                first_word = at_line_start && token == Token::Label;
                at_line_start = false;
            }
            // Un caractère quelconque — y compris hors ASCII : on avance
            // d'un *caractère* et non d'un octet, sinon la découpe
            // tomberait au milieu d'un « é ».
            other => {
                let len = other.len_utf8();
                out.push((&line[i..i + len], Token::Plain));
                i += len;
                at_line_start = false;
            }
        }
    }
    out
}

/// Le mot qui suit `from` est-il une directive de définition de données ?
///
/// C'est ce qui distingue `message db "salut"` — une étiquette suivie
/// d'une directive — de `mov rax, 1`. NASM tolère l'étiquette sans
/// deux-points dans ce seul cas, et le code des exercices s'en sert.
fn next_word_defines_data(line: &str, from: usize) -> bool {
    /// Celles qui peuvent suivre une étiquette nue. `equ` et `times` en
    /// font partie ; `section` ou `global` non, qui ne s'étiquettent pas.
    const APRÈS_ÉTIQUETTE: &[&str] = &[
        "db", "dw", "dd", "dq", "dt", "do", "dy", "dz", "resb", "resw", "resd", "resq", "rest",
        "reso", "resy", "resz", "incbin", "equ", "times",
    ];
    let reste = &line[from..];
    let début = reste.len() - reste.trim_start().len();
    let suite = &reste[début..];
    // Rien après, ou pas un mot : ce n'en est pas une.
    if début == 0 || suite.is_empty() {
        return false;
    }
    let fin = suite
        .find(|c: char| !is_word_char(c))
        .unwrap_or(suite.len());
    APRÈS_ÉTIQUETTE.contains(&suite[..fin].to_ascii_lowercase().as_str())
}

/// À quelle famille appartient un mot, à sa place dans la ligne.
fn classify(word: &str, first_word: bool, followed_by_colon: bool, macros: &Macros) -> Token {
    let lower = word.to_ascii_lowercase();
    // Les registres passent avant la règle des deux-points : le `fs` de
    // `[fs:0x28]` est un registre de segment, pas une étiquette.
    if REGISTERS.contains(&lower.as_str()) {
        return Token::Register;
    }
    if followed_by_colon || word.starts_with('.') {
        // `.boucle` est une étiquette locale, même sans deux-points quand
        // on y saute : `jmp .boucle`.
        return Token::Label;
    }
    if DIRECTIVES.contains(&lower.as_str()) {
        return Token::Directive;
    }
    // Une étiquette du fichier, employée comme opérande : `jmp boucle`,
    // `call vérifie`, `dq table`. Elle n'a pas de deux-points ici — ils
    // sont à sa définition — et passait donc pour un mot quelconque.
    if macros.est_une_étiquette(word) {
        return Token::Label;
    }
    if first_word {
        // La liste vient du décodeur qu'emploie namsdec, non d'un
        // tableau écrit ici : les deux ne peuvent donc pas diverger, et
        // le reproche qu'on faisait aux listes fermées — « un jeu
        // d'instructions s'allonge » — tombe avec elle.
        //
        // Ce qui n'y est pas se peint en couleur d'erreur. C'est le
        // point : `mav` recevait la couleur de `mov`, si bien que la
        // coloration ne distinguait pas ce qui existe de ce qui
        // n'existe pas. Elle n'apprenait rien, et rassurait à tort.
        // Une macro du fichier s'écrit à la place d'une instruction et
        // se colore comme telle : c'en est une, pour qui lit.
        return if namsdec_disasm::is_mnemonic(&lower) || macros.contient(&lower) {
            Token::Mnemonic
        } else {
            Token::UnknownMnemonic
        };
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
        // Seul l'accent grave connaît les échappements en NASM :
        // guillemets et apostrophes prennent tout au pied de la lettre.
        // `db "C:\", 0` ferme donc bien sa chaîne sur le guillemet, et
        // la suite de la ligne reste du code.
        if bytes[j] == b'\\' && quote == b'`' && j + 1 < bytes.len() {
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
    // Une section par morceau, et c'est très bien : `LayoutJob::append`
    // fusionne lui-même deux morceaux voisins de même format quand il n'y
    // a pas d'espace de tête. Mesuré : dix appels par ligne de listing
    // deviennent huit sections, exactement ce que la fusion prévoit.
    //
    // Le refaire ici a été essayé le 30 septembre 2026, et coûtait 22 ms
    // de plus sur quinze mille lignes — un `FontId` cloné par appel pour
    // comparer des formats déjà comparés en aval. Voir AMELIORATIONS 3.7.
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

    /// Un mnémonique se reconnaît à sa place **et** à la liste du
    /// décodeur : la place seule donnait sa couleur à `mav`.
    ///
    /// La liste ne risque pas de mentir, puisqu'elle est celle
    /// d'iced-x86 — ce qui se décode se colore. Restent les mots qui ne
    /// sont pas des instructions du tout : ils gardent leur famille, où
    /// qu'ils soient dans la ligne.
    #[test]
    fn a_mnemonic_is_the_first_word_and_a_known_one() {
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

        // Seul l'accent grave traite les échappements — c'est la règle
        // de NASM, et elle a son importance : `db "C:\", 0` ferme bien
        // sa chaîne sur le guillemet, et le `0` qui suit reste un nombre.
        let line = tokens("db \"C:\\\", 0");
        assert!(line.contains(&("\"C:\\\"", Token::Str)), "{line:?}");
        assert!(line.contains(&("0", Token::Number)), "{line:?}");
        // Entre accents graves, au contraire, l'échappement protège.
        assert!(tokens("db `a\\`b`").contains(&("`a\\`b`", Token::Str)));
        // Une chaîne non refermée prend le reste de la ligne plutôt que
        // de faire passer la suite pour du code.
        assert_eq!(tokens("db \"oubli").last().unwrap().1, Token::Str);
    }

    /// Une étiquette n'ouvre la porte à un mnémonique que si elle est en
    /// tête de ligne : employée comme opérande, elle est une opérande.
    #[test]
    fn a_label_used_as_an_operand_does_not_start_an_instruction() {
        let line = tokens("    db .foo, bar");
        assert!(line.contains(&(".foo", Token::Label)));
        assert!(
            line.contains(&("bar", Token::Plain)),
            "« bar » est une opérande, pas une instruction : {line:?}"
        );
        // Alors qu'en tête de ligne, l'étiquette laisse bien la place.
        assert!(tokens("boucle: add rax, 1").contains(&("add", Token::Mnemonic)));
    }

    /// Un registre de segment reste un registre, même suivi des
    /// deux-points d'un préfixe d'adresse.
    #[test]
    fn a_segment_override_is_a_register_not_a_label() {
        let line = tokens("    mov rax, [fs:0x28]");
        assert!(line.contains(&("fs", Token::Register)), "{line:?}");
        assert!(line.contains(&("rax", Token::Register)));
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

#[cfg(test)]
mod mnemonic_colour_tests {
    use super::*;

    fn premiers_mots(ligne: &str) -> Vec<(&str, Token)> {
        tokenize(ligne)
            .into_iter()
            .filter(|(mot, _)| !mot.trim().is_empty())
            .collect()
    }

    /// `mov` et `mav` ne se peignent plus pareil.
    ///
    /// C'était le reproche, et il était juste : une coloration qui donne
    /// la même couleur à une instruction et à une faute de frappe
    /// n'apprend rien, et rassure à tort — on relit sa ligne, on la voit
    /// colorée, on la croit bonne.
    #[test]
    fn a_typo_does_not_look_like_an_instruction() {
        let bons = premiers_mots("    mov rax, 1");
        assert_eq!(bons[0].1, Token::Mnemonic, "{bons:?}");
        let mauvais = premiers_mots("    mav rax, 1");
        assert_eq!(mauvais[0].1, Token::UnknownMnemonic, "{mauvais:?}");
        assert_ne!(
            Token::Mnemonic.colour(namsdec_theme::by_id("dark-plus")),
            Token::UnknownMnemonic.colour(namsdec_theme::by_id("dark-plus")),
            "et les deux couleurs diffèrent vraiment"
        );
    }

    /// Une macro du fichier s'écrit comme une instruction, et n'est pas
    /// signalée.
    ///
    /// C'est la contrepartie indispensable : on ne signale une faute de
    /// frappe que si l'on ne crie pas sur le reste. Une coloration qui
    /// se trompe une fois sur dix cesse d'être lue.
    #[test]
    fn a_macro_defined_in_the_file_is_not_flagged() {
        let source = "\
%macro ajoute_deux_fois 1
    add rax, %1
    add rax, %1
%endmacro

_start:
    ajoute_deux_fois 10
    mav rax, 1
";
        let macros = Macros::of(source);
        let appel: Vec<_> = tokenize_with("    ajoute_deux_fois 10", &macros)
            .into_iter()
            .filter(|(mot, _)| !mot.trim().is_empty())
            .collect();
        assert_eq!(
            appel[0].1,
            Token::Mnemonic,
            "l'appel de macro se colore comme une instruction : {appel:?}"
        );
        // Et la faute de frappe, elle, est toujours vue.
        let faute: Vec<_> = tokenize_with("    mav rax, 1", &macros)
            .into_iter()
            .filter(|(mot, _)| !mot.trim().is_empty())
            .collect();
        assert_eq!(faute[0].1, Token::UnknownMnemonic, "{faute:?}");
    }

    /// Une étiquette du fichier se colore aussi là où l'on y saute.
    ///
    /// `boucle:` était peinte en étiquette à sa définition, et pas du
    /// tout dans `jnz boucle` — où elle passait pour un mot quelconque.
    /// C'est pourtant là qu'on a besoin de la reconnaître : savoir d'un
    /// coup d'oeil que `boucle` est un endroit de *ce* fichier, et non un
    /// registre, un nombre ou une faute de frappe.
    #[test]
    fn a_label_is_coloured_where_one_jumps_to_it() {
        let source = "\
boucle:
    add rax, 1
    jnz boucle
    call vérifie
vérifie:
    ret
";
        let macros = Macros::of(source);
        let saut: Vec<_> = tokenize_with("    jnz boucle", &macros)
            .into_iter()
            .filter(|(mot, _)| !mot.trim().is_empty())
            .collect();
        assert_eq!(saut[0].1, Token::Mnemonic, "{saut:?}");
        assert_eq!(saut[1].1, Token::Label, "la cible du saut : {saut:?}");

        let appel: Vec<_> = tokenize_with("    call vérifie", &macros)
            .into_iter()
            .filter(|(mot, _)| !mot.trim().is_empty())
            .collect();
        assert_eq!(appel[1].1, Token::Label, "{appel:?}");

        // Mais un mot qui n'est aucune étiquette du fichier garde sa
        // nature : on ne peint pas en étiquette ce qui n'en est pas une.
        let ailleurs: Vec<_> = tokenize_with("    jmp nulle_part", &macros)
            .into_iter()
            .filter(|(mot, _)| !mot.trim().is_empty())
            .collect();
        assert_ne!(ailleurs[1].1, Token::Label, "{ailleurs:?}");

        // Et `.text` de `section .text` nomme une section, non un
        // endroit où sauter.
        let section: Vec<_> = tokenize_with("section .text", &macros)
            .into_iter()
            .filter(|(mot, _)| !mot.trim().is_empty())
            .collect();
        assert_eq!(section[0].1, Token::Directive, "{section:?}");
    }

    /// Ce qui est valide reste calme — y compris les préfixes et les
    /// instructions récentes.
    #[test]
    fn valid_assembly_is_never_flagged() {
        for ligne in [
            "    mov rax, 1",
            "    syscall",
            "    endbr64",
            "    rep movsb",
            "    lock xchg [rax], rbx",
            "    vpxor xmm0, xmm0, xmm0",
            "    cmovne rax, rbx",
            // Ce qui n'est pas une instruction du tout ne doit pas être
            // jugé comme telle : directives, étiquettes, préprocesseur.
            "section .text",
            "global _start",
            "_start:",
            ".boucle:",
            "msg: db \"salut\", 0",
            "%define TAILLE 64",
            "    ; un commentaire",
            "",
        ] {
            for (mot, token) in premiers_mots(ligne) {
                assert_ne!(
                    token,
                    Token::UnknownMnemonic,
                    "« {mot} » est signalé à tort dans : {ligne}"
                );
            }
        }
    }
}

#[cfg(test)]
mod corpus_tests {
    use super::*;

    /// Tout l'assembleur que namsdec livre doit passer sans un seul mot
    /// signalé.
    ///
    /// Une coloration qui crie sur du code juste est pire que celle
    /// qu'elle remplace : on apprend à ne plus la croire. Le corpus, ce
    /// sont les exercices livrés — le code que quelqu'un qui apprend a
    /// sous les yeux le premier jour.
    #[test]
    fn every_shipped_exercise_colours_clean() {
        // Le fichier du dépôt, et non celui qui est installé : le test
        // doit juger ce que namsdec livre, pas ce qu'une machine a.
        let dossier =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../namsdec-exercises/packs");
        let mut séries =
            vec![namsdec_exercises::builtin_pack_in(&dossier).expect("la série livrée se lit")];

        // Et les séries installées à côté, dans `plugins/exercices/`.
        // Sans elles, ce contrôle jugerait un corpus qui ne bouge plus
        // pendant que les nouveaux exercices, eux, emploient des
        // instructions que la coloration n'a peut-être jamais vues —
        // et il passerait d'autant mieux qu'il regarde moins.
        let ailleurs =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/exercices");
        let mut greffons = 0usize;
        if let Ok(entrées) = std::fs::read_dir(&ailleurs) {
            for entrée in entrées.flatten() {
                let chemin = entrée.path();
                if chemin.extension().is_some_and(|e| e == "json") {
                    séries
                        .push(namsdec_exercises::pack::read(&chemin).unwrap_or_else(|e| {
                            panic!("{} ne se lit pas : {e}", chemin.display())
                        }));
                    greffons += 1;
                }
            }
        }
        assert!(
            greffons > 0,
            "aucune série dans {} : le contrôle ne verrait plus que le parcours livré",
            ailleurs.display()
        );

        let mut vus = 0usize;
        let mut fautifs: Vec<String> = Vec::new();
        for exercice in séries.iter().flat_map(|p| &p.exercises) {
            vus += 1;
            let macros = Macros::of(&exercice.starter_code);
            for ligne in exercice.starter_code.lines() {
                for (mot, token) in tokenize_with(ligne, &macros) {
                    if token == Token::UnknownMnemonic {
                        fautifs.push(format!("{} : « {mot} » dans « {ligne} »", exercice.id));
                    }
                }
            }
        }
        assert!(vus > 0, "le corpus ne doit pas être vide");
        assert!(
            fautifs.is_empty(),
            "{} source(s) signalée(s) à tort sur {vus} :\n{}",
            fautifs.len(),
            fautifs.join("\n")
        );
    }
}
