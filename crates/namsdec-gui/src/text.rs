//! Le texte, et le curseur qui s'y déplace.
//!
//! Rien ici ne connaît egui, sauf pour poser un curseur : ce sont des
//! fonctions sur des chaînes, et c'est ce qui permet de les éprouver au
//! caractère près plutôt que par ce qu'on voit à l'écran.
//!
//! Deux pièges reviennent partout et expliquent la moitié de ce
//! fichier. Le premier : egui compte en OCTETS et le reste du monde en
//! caractères — un accent pèse deux octets, et confondre les deux pose
//! le curseur au milieu d'une lettre. Le second : ce qu'un utilisateur
//! appelle « un mot » n'est pas ce qu'une bibliothèque appelle un mot,
//! et le pas à pas de Vim a sa propre idée de la question.

use super::*;

/// Les positions, en octets, de toutes les occurrences de `query` dans
/// `text`, sans tenir compte de la casse.
///
/// Sans la casse : on écrit `RAX` dans un source et `rax` dans un
/// listing désassemblé, et chercher l'un doit trouver l'autre.
pub(crate) fn find_matches(text: &str, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let haystack = text.to_lowercase();
    let needle = query.to_lowercase();
    // `to_lowercase` peut changer la longueur d'un texte (ß → ss) : les
    // offsets ne vaudraient plus rien. Le cas ne se présente pas dans un
    // source assembleur, mais on ne le suppose pas.
    if haystack.len() != text.len() {
        return text.match_indices(query).map(|(at, _)| at).collect();
    }
    haystack.match_indices(&needle).map(|(at, _)| at).collect()
}

/// Les occurrences de `query` dans `text`, situées ligne par ligne.
///
/// La largeur est celle de la REQUÊTE, en caractères : c'est elle qu'on
/// surligne, et une occurrence trouvée sans tenir compte de la casse
/// peut peser d'autres octets que le texte cherché.
pub(crate) fn find_places(text: &str, query: &str) -> Vec<Place> {
    let largeur = query.chars().count();
    let étendues: Vec<std::ops::Range<usize>> = find_matches(text, query)
        .into_iter()
        .map(|at| at..(at + query.len()).min(text.len()))
        .collect();
    let mut places = place_spans(text, &étendues);
    for place in &mut places {
        place.largeur = largeur;
    }
    places
}

/// Où une occurrence tombe dans le texte : sa ligne, et sa colonne en
/// CARACTÈRES.
///
/// Les étendues que rendent `find_matches` et `occurrences::find_all`
/// sont en octets, parce que c'est ce qu'il faut pour remplacer du
/// texte. Les peindre demande autre chose : un galley d'egui se repère
/// par rangée et par colonne de caractères, et rien d'autre.
///
/// Traduire à la peinture coûtait cher et le coût ne se voyait pas :
/// `byte_to_char` recompte les caractères DEPUIS LE DÉBUT du texte, et
/// `Galley::pos_from_cursor` parcourt les rangées depuis la première.
/// Deux parcours complets par occurrence, à chaque image — sur un
/// listing de soixante mille lignes avec vingt mille occurrences, une
/// minute par image. Mesuré le 14 octobre 2026.
///
/// La ligne et la colonne sont donc relevées UNE fois, quand les
/// étendues changent, et par une seule passe sur le texte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Place {
    /// L'étendue d'origine, en octets : c'est elle qui sert à remplacer.
    pub octets: std::ops::Range<usize>,
    /// La ligne, comptée depuis 1 comme partout ailleurs dans namsdec.
    pub ligne: u32,
    /// La colonne du début, en caractères depuis le début de la ligne.
    pub colonne: usize,
    /// La largeur, en caractères.
    pub largeur: usize,
}

/// Situe des étendues d'octets dans les lignes du texte, en une passe.
///
/// Les étendues doivent être triées par début croissant — c'est ce que
/// rendent `find_matches` et `occurrences::find_all`, qui parcourent le
/// texte dans l'ordre. L'ordre permet d'avancer dans le texte et dans
/// les étendues ensemble, au lieu de repartir du début pour chacune.
///
/// Une étendue à cheval sur deux lignes n'est pas située : ni une
/// recherche ni un jeton n'en contient, et un surlignage qui traverserait
/// une fin de ligne serait faux.
pub(crate) fn place_spans(text: &str, spans: &[std::ops::Range<usize>]) -> Vec<Place> {
    let mut places = Vec::with_capacity(spans.len());
    let mut début_ligne = 0usize;
    let mut ligne = 1u32;
    let mut reste = spans;
    // Le texte est parcouru ligne par ligne ; les étendues qui tombent
    // dans la ligne courante sont consommées au passage.
    for morceau in text.split_inclusive('\n') {
        let fin_ligne = début_ligne + morceau.trim_end_matches('\n').len();
        while let Some(étendue) = reste.first() {
            if étendue.start >= début_ligne + morceau.len() {
                break;
            }
            let étendue = étendue.clone();
            reste = &reste[1..];
            if étendue.end > fin_ligne {
                continue;
            }
            let colonne = text[début_ligne..étendue.start].chars().count();
            let largeur = text[étendue.start..étendue.end].chars().count();
            places.push(Place {
                octets: étendue,
                ligne,
                colonne,
                largeur,
            });
        }
        début_ligne += morceau.len();
        ligne += 1;
    }
    places
}

/// Le rang des occurrences dont la ligne tombe entre `première` et
/// `dernière` (incluses), par recherche dichotomique.
///
/// Les places sont triées par ligne, puisque les étendues l'étaient par
/// octet : on trouve donc la tranche visible sans parcourir les autres.
/// C'est ce qui fait la différence entre peindre soixante surlignages et
/// en examiner vingt mille.
///
/// Des rangs et non une tranche : l'appelant a besoin du numéro de
/// l'occurrence — c'est lui que la barre de recherche compte, et c'est
/// lui qui dit laquelle est la courante.
pub(crate) fn places_entre(
    places: &[Place],
    première: u32,
    dernière: u32,
) -> std::ops::Range<usize> {
    let début = places.partition_point(|place| place.ligne < première);
    let fin = places.partition_point(|place| place.ligne <= dernière);
    début..fin.max(début)
}

/// Ce qu'une liste posée dans une fenêtre flottante peut prendre en
/// hauteur : une bonne moitié de l'écran.
///
/// Un plafond en pixels — trois cent soixante, trois cent vingt — faisait
/// défiler quatre thèmes au milieu d'un écran qui en aurait montré
/// quinze. Celui-ci suit l'écran qu'on a.
/// La marque d'un pas : un signe, sa couleur, et ce qu'il veut dire.
///
/// Un signe et non un mot : la colonne reste étroite, et l'on repère un
/// appel ou une écriture en parcourant la liste plutôt qu'en la lisant.
pub(crate) fn step_mark(
    s: &Strings,
    kind: namsdec_emu::StepKind,
    theme: &'static Theme,
) -> (&'static str, egui::Color32, &'static str) {
    use namsdec_emu::StepKind as K;
    match kind {
        K::Plain => ("   ", theme.dim, s.step_plain),
        K::Call => ("app", theme.accent, s.step_call),
        K::Return => ("ret", theme.accent, s.step_return),
        K::Jump => ("saut", theme.warn, s.step_jump),
        K::Syscall => ("sys", theme.good, s.step_syscall),
        K::Write => ("écr", theme.changed, s.step_write),
    }
}

/// Où mène un retour à la ligne qui garde l'indentation courante.
///
/// Rend le texte à insérer et de combien le curseur doit avancer. On
/// Ce qu'il faut faire après la frappe d'une parenthèse ou d'un
/// guillemet.
///
/// Une paire s'ouvre et se ferme : taper `(` et devoir taper `)` est le
/// genre de travail qu'une machine fait mieux. Mais mal fait, c'est pire
/// que rien — un `'` posé au milieu d'un commentaire français, et l'on
/// passe son temps à effacer ce qu'on n'a pas demandé.
///
/// D'où trois règles, et pas une de plus :
///
/// - on ne ferme pas devant un mot : `(` tapé avant `rax` ouvrirait un
///   `()` autour de rien ;
/// - on ne ferme pas une apostrophe qui suit une lettre — « l'adresse »
///   n'appelle aucune fermeture ;
/// - taper le fermant qui est déjà là ne l'écrit pas deux fois, il le
///   traverse.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum PairEdit {
    /// Poser ce caractère derrière le curseur, sans l'y déplacer.
    Close(char),
    /// Le fermant était déjà là : retirer celui qu'on vient de taper et
    /// laisser le curseur devant lui.
    StepOver,
    /// Ne rien faire.
    None,
}

/// Le fermant qui va avec cet ouvrant.
pub(crate) fn closing_of(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

/// Ce qu'on fait du caractère qui vient d'être tapé, `at` étant la
/// position du curseur **après** la frappe, en octets.
pub(crate) fn pair_edit(source: &str, at: usize) -> PairEdit {
    if at == 0 || at > source.len() || !source.is_char_boundary(at) {
        return PairEdit::None;
    }
    let Some(tapé) = source[..at].chars().next_back() else {
        return PairEdit::None;
    };
    let suivant = source[at..].chars().next();
    let précédent = {
        let avant = at - tapé.len_utf8();
        source[..avant].chars().next_back()
    };

    // Traverser un fermant déjà posé. Vaut pour les guillemets, qui sont
    // leur propre fermant : `"abc|"` puis `"` doit sortir de la chaîne,
    // non en ouvrir une seconde.
    let ferme = matches!(tapé, ')' | ']' | '}' | '"' | '\'');
    if ferme && suivant == Some(tapé) {
        return PairEdit::StepOver;
    }

    let Some(fermant) = closing_of(tapé) else {
        return PairEdit::None;
    };
    // Une apostrophe après une lettre est une apostrophe, pas une
    // chaîne : « l'adresse », « d'où ». C'est le cas qui rend
    // insupportables les éditeurs qui ferment tout.
    if matches!(tapé, '"' | '\'') && précédent.is_some_and(is_word_char) {
        return PairEdit::None;
    }
    // Ni devant un mot : on est en train d'écrire au milieu de quelque
    // chose, et la fermeture tomberait au mauvais endroit.
    if suivant.is_some_and(is_word_char) {
        return PairEdit::None;
    }
    PairEdit::Close(fermant)
}

/// Un effacement doit-il emporter la paire vide qu'il ouvre ?
///
/// `(|)` puis retour arrière : effacer la seule parenthèse ouvrante
/// laisserait la fermante orpheline — celle-là même qu'on n'avait pas
/// tapée.
pub(crate) fn backspace_clears_pair(source: &str, at: usize) -> bool {
    if at == 0 || at >= source.len() || !source.is_char_boundary(at) {
        return false;
    }
    let (Some(avant), Some(après)) = (
        source[..at].chars().next_back(),
        source[at..].chars().next(),
    ) else {
        return false;
    };
    closing_of(avant) == Some(après)
}

/// Place le curseur de l'éditeur à cet indice d'octet.
///
/// Les positions d'egui se comptent en *caractères*, celles d'une chaîne
/// Rust en octets : la conversion est ce qui sépare un curseur juste
/// d'un curseur qui dérive d'un cran à chaque accent.
///
/// Rend `false` quand le champ n'a pas encore d'état — à la toute
/// première image, il n'en a pas, et le curseur demandé se perdait en
/// silence. Qui pose un curseur AVANT que l'éditeur ait été dessiné une
/// fois doit pouvoir réessayer à l'image suivante.
pub(crate) fn set_cursor_byte(ctx: &egui::Context, source: &str, at: usize) -> bool {
    let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) else {
        return false;
    };
    let cible = byte_to_char(source, at);
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(
            egui::text::CCursor::new(cible),
        )));
    state.store(ctx, editor_id());
    true
}

/// Un caractère qui fait partie d'un mot, pour les règles ci-dessus.
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// écrit l'assembleur en colonnes — les étiquettes à gauche, les
/// instructions en retrait — et retaper quatre espaces à chaque ligne
/// use la patience avant d'user le clavier.
///
/// L'indentation reprise est celle de la ligne où l'on est, blancs
/// seulement : ce qui suit appartient au code.
pub(crate) fn newline_indent(source: &str, cursor: usize) -> String {
    let cursor = cursor.min(source.len());
    let line_start = source[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let indent: String = source[line_start..cursor]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    format!("\n{indent}")
}

/// Traduit un index de *caractère* dans le texte en (ligne, colonne),
/// toutes deux 1-indexées. egui ne donne que l'index brut ; le compter
/// ici plutôt que de lire la géométrie du `Galley` rend la position
/// indépendante de la mise en page (retour à la ligne automatique,
/// tabulations) — c'est bien la position dans le *texte* qu'on affiche.
/// La forme que les contrôles emploient, là où l'octet n'est pas le
/// sujet. L'application, elle, demande les trois d'un coup — elle en a
/// besoin des trois, et une seule passe les donne.
#[cfg(test)]
pub(crate) fn line_col(text: &str, char_index: usize) -> (usize, usize) {
    let (ligne, colonne, _) = line_col_byte(text, char_index);
    (ligne, colonne)
}

/// La ligne qui contient `octet`, prise AUTOUR de lui.
///
/// `lines().nth(n)` répondrait aussi, en relisant tout le texte jusque
/// là — à chaque image, pour la ligne du curseur. Sur un listing de
/// seize mégaoctets avec le curseur au milieu, c'est huit mégaoctets
/// relus soixante fois par seconde. Reculer jusqu'au retour à la ligne
/// précédent, avancer jusqu'au suivant : le coût est celui de la ligne,
/// non celui du fichier.
pub(crate) fn ligne_autour(text: &str, octet: usize) -> &str {
    let octet = octet.min(text.len());
    // Un octet qui tomberait au milieu d'un caractère ne peut pas servir
    // à découper : on se rabat alors sur le début du texte, ce qui ne
    // perd qu'une proposition de correction.
    if !text.is_char_boundary(octet) {
        return text.lines().next().unwrap_or("");
    }
    let début = text[..octet].rfind('\n').map_or(0, |at| at + 1);
    let fin = text[octet..].find('\n').map_or(text.len(), |at| octet + at);
    &text[début..fin]
}

/// La ligne, la colonne et l'octet d'un rang de caractère, en UNE passe.
///
/// Les trois se lisaient par trois fonctions, donc trois parcours du
/// texte depuis le début — et ils sont demandés ensemble, à chaque
/// image, pour peindre le curseur et l'écrire dans la barre d'état. Sur
/// un listing de seize mégaoctets avec le curseur vers la fin, c'est
/// trois fois seize millions de caractères comptés par image.
pub(crate) fn line_col_byte(text: &str, char_index: usize) -> (usize, usize, usize) {
    let mut ligne = 1;
    let mut colonne = 1;
    let mut octet = text.len();
    for (rang, (position, c)) in text.char_indices().enumerate() {
        if rang == char_index {
            octet = position;
            break;
        }
        if c == '\n' {
            ligne += 1;
            colonne = 1;
        } else {
            colonne += 1;
        }
    }
    (ligne, colonne, octet)
}

/// Découpe une ligne d'arguments comme un interpréteur de commandes le
/// ferait, en plus simple.
///
/// L'espace sépare, les guillemets — simples ou doubles — groupent ce qui
/// doit rester ensemble : un mot de passe contenant une espace se donne
/// ainsi sans qu'on ait à y penser. Pas d'échappement ni de variables :
/// ce qu'on écrit là part tel quel dans `argv`, et une ligne qui ne se
/// lit pas à l'œil n'aurait pas sa place devant un programme qu'on
/// cherche à comprendre.
pub(crate) fn split_args(ligne: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut courant = String::new();
    let mut ouvert: Option<char> = None;
    let mut commencé = false;
    for c in ligne.chars() {
        match (ouvert, c) {
            (Some(guillemet), _) if c == guillemet => ouvert = None,
            (Some(_), _) => courant.push(c),
            (None, '\'') | (None, '"') => {
                ouvert = Some(c);
                commencé = true;
            }
            (None, c) if c.is_whitespace() => {
                if commencé {
                    arguments.push(std::mem::take(&mut courant));
                    commencé = false;
                }
            }
            (None, c) => {
                courant.push(c);
                commencé = true;
            }
        }
    }
    if commencé {
        arguments.push(courant);
    }
    arguments
}

/// Un déplacement du mode Normal, tel que vim le nomme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Move {
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
    FileEnd,
    /// Le début du fichier — `gg` dans vim.
    FileStart,
    /// Le début du mot suivant — `w`.
    WordForward,
    /// Le début du mot précédent — `b`.
    WordBack,
    /// La fin du mot où l'on est — `e`.
    WordEnd,
}

/// Les trois familles de caractères que vim distingue pour `w` et `b`.
///
/// Un mot, une suite de ponctuation, ou du blanc. C'est ce découpage qui
/// fait que `w` s'arrête sur le `[` de `[rbp-0x18]` au lieu de sauter la
/// moitié de la ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Classe {
    Mot,
    Ponctuation,
    Blanc,
}

pub(crate) fn classe_de(c: Option<char>) -> Classe {
    match c {
        None => Classe::Blanc,
        Some(c) if c.is_alphanumeric() || c == '_' => Classe::Mot,
        Some(c) if c.is_whitespace() => Classe::Blanc,
        Some(_) => Classe::Ponctuation,
    }
}

/// Où va le curseur, compté en caractères depuis le début du texte.
///
/// Le mode Normal n'avait aucun déplacement : `i` entrait en Insertion,
/// Échap en sortait, et c'était tout. On ne pouvait donc pas se promener
/// dans le code sans repasser par la souris — la première chose que
/// cherche quiconque vient de vim.
///
/// Compté en caractères et non en octets : une ligne de commentaire
/// accentuée ferait sauter le curseur au milieu d'un caractère.
/// La forme à un seul pas, que les contrôles emploient : là, c'est la
/// règle du mouvement qui est en cause, jamais le compteur.
#[cfg(test)]
pub(crate) fn move_cursor(source: &str, at: usize, mouvement: Move) -> usize {
    move_cursor_times(source, at, mouvement, 1)
}

/// Le même mouvement `fois` fois : le compteur de vim — `5j` descend de
/// cinq lignes, `3w` avance de trois mots.
///
/// Le texte n'est découpé en caractères qu'UNE fois pour toute la
/// répétition. Rappeler `move_cursor` cinq fois le recopierait cinq
/// fois, et un listing de seize mégaoctets pèse soixante-quatre
/// mégaoctets une fois mis en `char`.
///
/// Buté — le début du fichier, la fin de la ligne —, on s'arrête :
/// insister ne mène nulle part, et `9999j` n'a pas à faire dix mille
/// tours pour le découvrir.
pub(crate) fn move_cursor_times(source: &str, at: usize, mouvement: Move, fois: u32) -> usize {
    let caractères: Vec<char> = source.chars().collect();
    let mut at = at.min(caractères.len());
    for _ in 0..fois.max(1) {
        let suivant = move_once(&caractères, at, mouvement);
        if suivant == at {
            break;
        }
        at = suivant;
    }
    at
}

/// Un pas de `mouvement`, sur le texte déjà découpé en caractères.
fn move_once(caractères: &[char], at: usize, mouvement: Move) -> usize {
    let at = at.min(caractères.len());
    // Le début de la ligne où l'on est, et sa longueur.
    let début = caractères[..at]
        .iter()
        .rposition(|c| *c == '\n')
        .map_or(0, |i| i + 1);
    let fin = caractères[at..]
        .iter()
        .position(|c| *c == '\n')
        .map_or(caractères.len(), |i| at + i);
    let colonne = at - début;

    match mouvement {
        // `h` et `l` restent sur leur ligne : dans vim, ils ne la
        // quittent pas, et c'est ce qui les rend prévisibles.
        Move::Left => at.saturating_sub(1).max(début),
        Move::Right => (at + 1).min(fin),
        Move::LineStart => début,
        Move::LineEnd => fin,
        Move::FileEnd => caractères.len(),
        Move::FileStart => 0,
        // Les mouvements par mot, et c'est eux qu'on réclame en premier
        // quand on a l'habitude de vim : se déplacer caractère par
        // caractère dans `mov qword [rbp-0x18], rax` use la patience.
        //
        // La notion de mot est celle de vim, pas celle d'un traitement
        // de texte : une suite de lettres, chiffres et soulignés, ou
        // bien une suite de ponctuation. `[rbp-0x18]` compte donc pour
        // plusieurs mots, ce qui est exactement ce qu'on veut pour
        // atteindre le déplacement sans viser à la souris.
        Move::WordForward => {
            let mut i = at;
            let classe = classe_de(caractères.get(i).copied());
            // On sort du mot courant…
            while i < caractères.len()
                && classe_de(Some(caractères[i])) == classe
                && classe != Classe::Blanc
            {
                i += 1;
            }
            // …puis des blancs qui suivent. Les retours à la ligne
            // arrêtent : `w` en fin de ligne descend d'une ligne dans
            // vim, mais s'arrête au premier mot — il ne saute pas une
            // ligne vide.
            while i < caractères.len() && caractères[i] != '\n' && caractères[i].is_whitespace()
            {
                i += 1;
            }
            i
        }
        Move::WordBack => {
            let mut i = at;
            // Les blancs d'abord : on recule jusqu'à toucher un mot.
            while i > 0 && caractères[i - 1] != '\n' && caractères[i - 1].is_whitespace() {
                i -= 1;
            }
            if i == 0 {
                return 0;
            }
            let classe = classe_de(Some(caractères[i - 1]));
            while i > 0 && classe_de(Some(caractères[i - 1])) == classe {
                i -= 1;
            }
            i
        }
        Move::WordEnd => {
            let mut i = at;
            // Si l'on est déjà sur la fin d'un mot, on va à la suivante :
            // `e` répété doit avancer, non piétiner.
            if i < caractères.len() {
                i += 1;
            }
            while i < caractères.len() && caractères[i] != '\n' && caractères[i].is_whitespace()
            {
                i += 1;
            }
            let classe = classe_de(caractères.get(i).copied());
            while i + 1 < caractères.len()
                && classe_de(Some(caractères[i + 1])) == classe
                && classe != Classe::Blanc
            {
                i += 1;
            }
            i
        }
        Move::Up => {
            if début == 0 {
                return at;
            }
            let fin_précédente = début - 1;
            let début_précédente = caractères[..fin_précédente]
                .iter()
                .rposition(|c| *c == '\n')
                .map_or(0, |i| i + 1);
            // La colonne est gardée tant que la ligne visée est assez
            // longue, comme partout ailleurs.
            (début_précédente + colonne).min(fin_précédente)
        }
        Move::Down => {
            if fin >= caractères.len() {
                return at;
            }
            let début_suivante = fin + 1;
            let fin_suivante = caractères[début_suivante..]
                .iter()
                .position(|c| *c == '\n')
                .map_or(caractères.len(), |i| début_suivante + i);
            (début_suivante + colonne).min(fin_suivante)
        }
    }
}

/// `$` ou `$$` sous le curseur, s'il y en a un.
pub(crate) fn dollars_at(text: &str, byte: usize) -> Option<&str> {
    let before = &text[..byte];
    let start = before.len() - before.chars().rev().take_while(|c| *c == '$').count();
    let end = byte + text[byte..].chars().take_while(|c| *c == '$').count();
    (end > start).then(|| &text[start..end])
}

/// Décalage en octets du `n`-ième caractère, et l'inverse. egui compte
/// les curseurs en caractères, Rust tranche les chaînes en octets, et
/// confondre les deux coupe un « é » en deux.
pub(crate) fn char_to_byte(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(index, _)| index)
}

pub(crate) fn byte_to_char(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].chars().count()
}

/// Recopie la ligne qui contient `cursor`, juste en dessous.
///
/// Rend la source modifiée et la nouvelle position du curseur, qui suit
/// la copie : on duplique une ligne pour la changer, et la changer
/// commence par être dessus.
///
/// La dernière ligne d'un fichier sans retour final est un cas à part —
/// il faut en ajouter un, sinon la copie se collerait derrière l'original
/// et n'en ferait qu'une seule ligne deux fois plus longue.
pub(crate) fn duplicated_line(source: &str, cursor: usize) -> (String, usize) {
    let cursor = cursor.min(source.len());
    let début = source[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let fin = source[cursor..]
        .find('\n')
        .map_or(source.len(), |i| cursor + i);
    let ligne = &source[début..fin];

    let mut out = String::with_capacity(source.len() + ligne.len() + 1);
    out.push_str(&source[..fin]);
    out.push('\n');
    out.push_str(ligne);
    out.push_str(&source[fin..]);
    // Le curseur garde sa colonne, une ligne plus bas.
    (out, cursor + ligne.len() + 1)
}

/// Où commence la ligne de rang `index`, en octets.
///
/// `str::lines()` ne rend pas les positions, et les recalculer à la main
/// à chaque usage finit par diverger de ce que `lines()` appelle une
/// ligne — notamment sur `\r\n`. Une seule fonction, donc, et les
/// autres s'y réfèrent.
pub(crate) fn ligne_début(source: &str, index: usize) -> Option<usize> {
    let mut reste = index;
    let mut début = 0usize;
    while reste > 0 {
        let saut = source[début..].find('\n')?;
        début += saut + 1;
        reste -= 1;
    }
    (début <= source.len()).then_some(début)
}
