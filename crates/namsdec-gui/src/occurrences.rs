//! « Où est-ce que ça revient ? »
//!
//! On lit une ligne, on tombe sur `rax`, et la question vient toute
//! seule : où d'autre ce registre est-il touché ? On la pose aujourd'hui
//! avec les yeux, en parcourant le fichier — ce qui marche sur vingt
//! lignes et plus du tout sur deux cents.
//!
//! Ce module répond, et ne dessine rien : il rend des étendues d'octets,
//! à charge pour l'éditeur de les peindre.
//!
//! **Il ne cherche pas du texte, il cherche des jetons.** Chercher
//! « rax » comme une suite de lettres trouverait le `rax` d'un
//! commentaire, celui d'une chaîne, et le milieu d'une étiquette
//! `sauve_rax`. On passe donc par la même découpe que la coloration : ce
//! qui est surligné est ce qui *est* ce registre, pas ce qui lui
//! ressemble.
//!
//! La casse suit NASM, et c'est une vraie distinction : `MOV` et `mov`
//! sont la même instruction, `Boucle` et `boucle` sont deux étiquettes
//! différentes. Surligner l'une en croyant l'autre serait pire que de ne
//! rien surligner.

use std::ops::Range;

use crate::syntax::{self, Macros, Token};

/// Ce qu'on a décidé de suivre.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracked {
    /// Le texte cherché, tel qu'il est écrit là où on l'a pris.
    pub word: String,
    /// Sa nature, qui décide de la comparaison.
    pub kind: Token,
    /// Où il a été pris, en octets. Sert à ne pas se surligner soi-même
    /// d'une couleur qui ferait croire à autre chose.
    pub origin: Range<usize>,
}

/// Le jeton sous le curseur, s'il y en a un qui vaille la peine d'être
/// suivi.
///
/// `None` sur un blanc, une virgule, un commentaire : suivre « , » ne
/// répondrait à aucune question.
pub fn tracked_at(source: &str, cursor: usize) -> Option<Tracked> {
    let cursor = cursor.min(source.len());
    let macros = Macros::of(source);
    for (début_ligne, ligne) in line_offsets(source) {
        let fin_ligne = début_ligne + ligne.len();
        // Le curseur en fin de ligne appartient à cette ligne-là.
        if cursor < début_ligne || cursor > fin_ligne {
            continue;
        }
        let mut à = début_ligne;
        for (morceau, token) in syntax::tokenize_with(ligne, &macros) {
            let étendue = à..à + morceau.len();
            à = étendue.end;
            if !étendue.contains(&cursor) && étendue.end != cursor {
                continue;
            }
            if !worth_tracking(token) || morceau.trim().is_empty() {
                continue;
            }
            return Some(Tracked {
                word: morceau.to_owned(),
                kind: token,
                origin: étendue,
            });
        }
    }
    None
}

/// Ce qu'on suit d'après une sélection, quand il y en a une.
///
/// Prise telle quelle, sans regarder si elle tombe sur des frontières de
/// jeton : quelqu'un qui sélectionne `ax` dans `rax` demande les `ax`, et
/// lui rendre les `rax` serait lui répondre à côté.
pub fn tracked_selection(source: &str, range: Range<usize>) -> Option<Tracked> {
    let texte = source.get(range.clone())?.trim();
    if texte.is_empty() {
        return None;
    }
    Some(Tracked {
        word: texte.to_owned(),
        kind: Token::Plain,
        origin: range,
    })
}

/// Toutes les occurrences de ce qu'on suit, dans l'ordre du fichier.
///
/// Pour un jeton, ce sont les jetons de même nature et de même nom. Pour
/// une sélection, c'est le texte, partout où il apparaît — y compris au
/// milieu d'un mot, puisque c'est ce qu'on a demandé.
pub fn find_all(source: &str, tracked: &Tracked) -> Vec<Range<usize>> {
    if tracked.kind == Token::Plain {
        return find_literal(source, &tracked.word);
    }
    let macros = Macros::of(source);
    let mut out = Vec::new();
    for (début_ligne, ligne) in line_offsets(source) {
        let mut à = début_ligne;
        for (morceau, token) in syntax::tokenize_with(ligne, &macros) {
            let étendue = à..à + morceau.len();
            à = étendue.end;
            if token == tracked.kind && same_word(morceau, &tracked.word, token) {
                out.push(étendue);
            }
        }
    }
    out
}

/// Les deux mots désignent-ils la même chose, selon les règles de NASM ?
///
/// Instructions, registres et directives ignorent la casse : `MOV RAX` et
/// `mov rax` sont le même code, et l'un des deux sort d'un désassembleur.
/// Les étiquettes, non : `Boucle` et `boucle` sont deux symboles, et
/// l'assembleur refusera de lier celui qu'on n'a pas défini.
fn same_word(a: &str, b: &str, kind: Token) -> bool {
    match kind {
        Token::Label => a == b,
        _ => a.eq_ignore_ascii_case(b),
    }
}

/// Ce qui mérite d'être suivi : ce qui a un nom.
fn worth_tracking(token: Token) -> bool {
    matches!(
        token,
        Token::Mnemonic
            | Token::UnknownMnemonic
            | Token::Register
            | Token::Label
            | Token::Directive
            | Token::Preprocessor
            | Token::Number
    )
}

/// Le texte, brut, partout où il revient.
fn find_literal(source: &str, needle: &str) -> Vec<Range<usize>> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut à = 0;
    while let Some(i) = source[à..].find(needle) {
        let début = à + i;
        out.push(début..début + needle.len());
        // On avance d'un caractère et non de la longueur trouvée : deux
        // occurrences peuvent se chevaucher (`aa` dans `aaa`), et les
        // sauter serait en perdre une.
        à = début + needle.chars().next().map_or(1, char::len_utf8);
    }
    out
}

/// Les lignes avec leur décalage en octets depuis le début.
fn line_offsets(source: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut à = 0;
    source.split('\n').map(move |ligne| {
        let début = à;
        à += ligne.len() + 1;
        (début, ligne)
    })
}

/// Remplace toutes les occurrences par `remplacement`, de la fin vers le
/// début.
///
/// De la fin vers le début, et c'est la seule façon qui marche : chaque
/// remplacement décale ce qui suit, si bien qu'en partant du début, la
/// deuxième étendue désignerait déjà autre chose.
///
/// C'est ce qui remplace le multicurseur pour son usage le plus courant —
/// renommer une étiquette, changer un registre partout — sans avoir à
/// tenir plusieurs curseurs.
pub fn replace_all(source: &str, étendues: &[Range<usize>], remplacement: &str) -> String {
    let mut out = source.to_owned();
    for étendue in étendues.iter().rev() {
        if étendue.end <= out.len()
            && out.is_char_boundary(étendue.start)
            && out.is_char_boundary(étendue.end)
        {
            out.replace_range(étendue.clone(), remplacement);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "\
section .text
global _start
_start:
    mov rax, 1          ; rax reçoit un
    mov rbx, rax
boucle:
    add rax, rbx
    jnz boucle
    hlt
";

    fn suivi(mot: &str) -> Tracked {
        let at = SOURCE.find(mot).expect("le mot est dans la source");
        tracked_at(SOURCE, at).expect("il y a quelque chose à suivre")
    }

    /// Un registre se retrouve partout où il est vraiment employé.
    #[test]
    fn a_register_is_found_everywhere_it_is_used() {
        let rax = suivi("rax");
        assert_eq!(rax.word, "rax");
        assert_eq!(rax.kind, Token::Register);
        let trouvés = find_all(SOURCE, &rax);
        assert_eq!(trouvés.len(), 3, "trois `rax` de code : {trouvés:?}");
        for étendue in &trouvés {
            assert_eq!(&SOURCE[étendue.clone()], "rax");
        }
    }

    /// Mais pas celui d'un commentaire.
    ///
    /// C'est tout l'intérêt de passer par la découpe : chercher « rax »
    /// comme du texte trouverait le cinquième, qui n'est pas un registre
    /// mais un mot français.
    #[test]
    fn the_rax_of_a_comment_is_not_a_register() {
        let rax = suivi("rax");
        let trouvés = find_all(SOURCE, &rax);
        // Le `rax` du commentaire, et lui seul : il faut comparer à son
        // étendue, non à sa position — les occurrences des lignes
        // suivantes sont évidemment plus loin dans le fichier.
        let commentaire = SOURCE.find("; rax").expect("le commentaire est là");
        let dans_le_commentaire = commentaire..commentaire + "; rax".len();
        assert!(
            !trouvés
                .iter()
                .any(|é| dans_le_commentaire.contains(&é.start)),
            "le `rax` du commentaire ne doit pas être surligné : {trouvés:?}"
        );
        // Et le texte brut, lui, le trouverait : voilà la différence.
        assert_eq!(find_literal(SOURCE, "rax").len(), 4);
    }

    /// Une étiquette se retrouve à sa définition comme à ses sauts.
    #[test]
    fn a_label_is_found_where_it_is_defined_and_where_it_is_jumped_to() {
        let boucle = suivi("boucle");
        assert_eq!(boucle.kind, Token::Label);
        let trouvés = find_all(SOURCE, &boucle);
        assert_eq!(trouvés.len(), 2, "la définition et le saut : {trouvés:?}");
    }

    /// La casse suit NASM : elle compte pour une étiquette, pas pour une
    /// instruction.
    ///
    /// Se tromper là-dessus surlignerait deux symboles distincts comme
    /// s'ils n'en faisaient qu'un — et l'assembleur, lui, refuserait de
    /// lier celui qu'on n'a pas défini.
    #[test]
    fn case_follows_what_nasm_does() {
        let source = "\
Boucle:
    MOV rax, 1
    mov RAX, 2
    jmp boucle
";
        // Les instructions : la casse ne compte pas.
        let at = source.find("MOV").unwrap();
        let mov = tracked_at(source, at).unwrap();
        assert_eq!(find_all(source, &mov).len(), 2, "MOV et mov, c'est pareil");

        // Les registres non plus.
        let at = source.find("rax").unwrap();
        let rax = tracked_at(source, at).unwrap();
        assert_eq!(find_all(source, &rax).len(), 2, "rax et RAX aussi");

        // Les étiquettes, si.
        let at = source.find("Boucle").unwrap();
        let étiquette = tracked_at(source, at).unwrap();
        assert_eq!(étiquette.kind, Token::Label);
        assert_eq!(
            find_all(source, &étiquette).len(),
            1,
            "`Boucle` n'est pas `boucle` : l'assembleur ne les confond pas"
        );
    }

    /// Une sélection se cherche telle quelle, même au milieu d'un mot.
    ///
    /// Quelqu'un qui sélectionne `ax` demande les `ax`. Lui rendre les
    /// `rax` serait répondre à côté.
    #[test]
    fn a_selection_is_searched_as_it_was_given() {
        let source = "mov rax, 1\nmov eax, 2\n";
        let début = source.find("ax").unwrap();
        let sélection = tracked_selection(source, début..début + 2).unwrap();
        assert_eq!(sélection.word, "ax");
        assert_eq!(find_all(source, &sélection).len(), 2);
    }

    /// Rien à suivre dans un commentaire — mais le mot qu'on vient de
    /// taper compte, même curseur posé juste derrière.
    #[test]
    fn a_comment_holds_nothing_to_track_but_the_word_just_typed_does() {
        // Dans un commentaire on écrit du français : rien à suivre.
        let source = "    hlt   ; on s'arrête ici\n";
        let dans = source.find("arrête").unwrap();
        assert!(tracked_at(source, dans).is_none());

        // Un blanc en tête de ligne non plus.
        let source = "    mov rax, 1\n";
        assert!(tracked_at(source, 2).is_none());

        // En revanche, le curseur posé *juste après* un mot désigne ce
        // mot : c'est là qu'il se trouve quand on vient de le taper, et
        // demander de reculer d'un caractère pour être compris serait
        // une exigence de machine.
        let après_rax = source.find(',').unwrap();
        let suivi = tracked_at(source, après_rax).expect("le mot qui finit ici");
        assert_eq!(suivi.word, "rax");
    }

    /// Remplacer partout se fait de la fin vers le début.
    ///
    /// C'est la seule façon qui marche : chaque remplacement décale ce
    /// qui suit, si bien qu'en partant du début, la deuxième étendue
    /// désignerait déjà autre chose. Le test le vérifie avec un
    /// remplacement plus long que l'original, où l'erreur se voit.
    #[test]
    fn replacing_everywhere_survives_a_longer_replacement() {
        let source = "mov rax, 1\nadd rax, rbx\n";
        let rax = tracked_at(source, source.find("rax").unwrap()).unwrap();
        let étendues = find_all(source, &rax);
        assert_eq!(étendues.len(), 2);
        let après = replace_all(source, &étendues, "compteur");
        assert_eq!(après, "mov compteur, 1\nadd compteur, rbx\n");
    }

    /// Aucune position ne fait paniquer, accents compris.
    #[test]
    fn nothing_panics_anywhere_in_the_file() {
        for source in [
            "",
            "\n",
            "é",
            "; drapeau à zéro\n    hlt\n",
            "mov rax, 'é'\n",
            "боucle:\n    jmp боucle\n",
        ] {
            for at in 0..=source.len() {
                if !source.is_char_boundary(at) {
                    continue;
                }
                if let Some(suivi) = tracked_at(source, at) {
                    let étendues = find_all(source, &suivi);
                    let _ = replace_all(source, &étendues, "x");
                }
                let _ = tracked_selection(source, at..source.len());
            }
        }
    }
}
