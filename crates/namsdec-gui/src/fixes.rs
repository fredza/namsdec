//! Les corrections que namsdec sait proposer, et qu'une touche confirme.
//!
//! Un assembleur ne pardonne rien et n'explique presque rien : `nasm`
//! répond « operation size not specified » et s'arrête. Celui qui
//! apprend relit sa ligne sans savoir ce qu'on lui reproche, et celui
//! qui sait perd trente secondes à réécrire ce qu'il savait déjà.
//!
//! Ce module regarde **une ligne**, dit ce qui cloche en français, et
//! propose la ligne réécrite. Rien ne s'applique tout seul : la
//! proposition s'affiche, et Alt+Entrée la pose. C'est la règle de
//! namsdec sur tout ce qui touche au code de quelqu'un — on montre, on
//! attend, on n'écrit qu'à la demande.
//!
//! **Ce qu'il ne fait pas.** Il ne lance pas l'assembleur, il ne lit pas
//! ses messages d'erreur, et il ne devine rien au-delà de six règles
//! qu'on peut justifier une par une. Une correction dont on ne saurait
//! pas dire pourquoi elle est juste n'a rien à faire ici : elle
//! apprendrait le faux, ce qui est pire que de se taire.
//!
//! Les six règles, par ordre de certitude :
//!
//! 1. un crochet ouvert et jamais refermé ;
//! 2. un mnémonique qui n'existe pas, à deux lettres près d'un qui existe ;
//! 3. `int 0x80` dans du code 64 bits, où l'appel système est `syscall` ;
//! 4. un saut vers une étiquette qui n'est définie nulle part ;
//! 5. deux registres de tailles différentes dans la même instruction ;
//! 6. une écriture en mémoire dont la taille n'est pas dite.

use crate::i18n::Strings;
use crate::syntax::{self, REGISTERS, Token};

/// Une correction proposée : la ligne réécrite, et ce qu'elle corrige.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    /// La ligne entière, telle qu'elle remplacera l'ancienne. Entière et
    /// non un morceau : l'appelant n'a pas à recoller l'indentation ni
    /// le commentaire, qu'on lui rend intacts.
    pub ligne: String,
    /// Ce qu'on corrige, en une phrase déjà traduite. C'est ce que la
    /// barre d'état affiche à côté du raccourci.
    pub titre: String,
}

/// Les mots qui disent la taille d'un accès mémoire.
const TAILLES: &[&str] = &[
    "byte", "word", "dword", "qword", "tword", "oword", "yword", "zword",
];

/// Les familles de registres généraux, des 64 bits aux 8 bits.
///
/// `ah`, `bh`, `ch`, `dh` n'y sont pas : ce sont les octets *hauts* de
/// `ax`, `bx`… Les proposer en élargissant une instruction reviendrait à
/// désigner un autre morceau du registre, et une correction qui change
/// ce qu'on lit est pire que l'erreur qu'elle répare.
const FAMILLES: &[[&str; 4]] = &[
    ["rax", "eax", "ax", "al"],
    ["rbx", "ebx", "bx", "bl"],
    ["rcx", "ecx", "cx", "cl"],
    ["rdx", "edx", "dx", "dl"],
    ["rsi", "esi", "si", "sil"],
    ["rdi", "edi", "di", "dil"],
    ["rsp", "esp", "sp", "spl"],
    ["rbp", "ebp", "bp", "bpl"],
    ["r8", "r8d", "r8w", "r8b"],
    ["r9", "r9d", "r9w", "r9b"],
    ["r10", "r10d", "r10w", "r10b"],
    ["r11", "r11d", "r11w", "r11b"],
    ["r12", "r12d", "r12w", "r12b"],
    ["r13", "r13d", "r13w", "r13b"],
    ["r14", "r14d", "r14w", "r14b"],
    ["r15", "r15d", "r15w", "r15b"],
];

/// Les instructions à deux opérandes dont les deux côtés doivent avoir
/// la même taille.
///
/// `movzx` et `movsx` n'y sont pas, et c'est tout leur objet : elles
/// existent précisément pour aller d'une taille à une autre.
const MÊME_TAILLE: &[&str] = &[
    "mov", "add", "sub", "cmp", "and", "or", "xor", "test", "adc", "sbb", "xchg",
];

/// Les instructions qui peuvent écrire en mémoire sans qu'un registre
/// dise la taille de l'écriture.
const ÉCRIT_EN_MÉMOIRE: &[&str] = &[
    "mov", "add", "sub", "cmp", "and", "or", "xor", "test", "adc", "sbb", "inc", "dec", "neg",
    "not", "push", "pop",
];

/// La distance d'édition entre deux mots : combien d'insertions, de
/// retraits, de remplacements et d'**inversions** séparent l'un de
/// l'autre.
///
/// Les inversions comptent pour une, et c'est tout l'intérêt : la faute
/// de frappe qu'on fait vraiment est `mvo` pour `mov` — deux lettres
/// échangées sous les doigts. La distance de Levenshtein nue la compte
/// double et l'écarte ; celle-ci la voit pour ce qu'elle est.
///
/// Écrite ici plutôt que prise d'une bibliothèque : quinze lignes,
/// aucune dépendance, et l'on sait exactement ce qu'elle compte quand il
/// faut justifier pourquoi `mvo` propose `mov` et pourquoi `tralala` ne
/// propose rien.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    // Trois lignes du tableau suffisent : l'inversion ne regarde que
    // deux rangs en arrière.
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, ligne) in d.iter_mut().enumerate() {
        ligne[0] = i;
    }
    for (j, case) in d[0].iter_mut().enumerate() {
        *case = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let coût = usize::from(a[i - 1] != b[j - 1]);
            let mut meilleur = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + coût);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                meilleur = meilleur.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = meilleur;
        }
    }
    d[a.len()][b.len()]
}

/// Le candidat le plus proche d'un mot, s'il en est un d'assez proche.
///
/// « Assez proche » dépend de la longueur : sur trois lettres, une seule
/// faute sépare `mov` de `mul`, et proposer l'un pour l'autre serait
/// deviner. Au-delà de quatre lettres, deux fautes restent une coquille.
///
/// `casse` dit si les majuscules comptent. Elles ne comptent pas pour un
/// mnémonique — NASM lit `MOV` comme `mov` — et elles comptent pour une
/// étiquette, où `Boucle` et `boucle` sont deux noms différents : c'est
/// justement la faute qu'on veut savoir signaler.
///
/// À égalité de distance, le plus court l'emporte, puis l'ordre
/// alphabétique : il faut une réponse stable, sinon la proposition
/// changerait d'une frappe à l'autre sans raison visible.
fn plus_proche<'a>(
    mot: &str,
    candidats: impl Iterator<Item = &'a str>,
    casse: bool,
) -> Option<&'a str> {
    let plié = |m: &str| {
        if casse {
            m.to_owned()
        } else {
            m.to_ascii_lowercase()
        }
    };
    let mot = plié(mot);
    let toléré = if mot.chars().count() <= 4 { 1 } else { 2 };
    let mut meilleur: Option<(usize, &'a str)> = None;
    for candidat in candidats {
        let d = distance(&mot, &plié(candidat));
        if d == 0 || d > toléré {
            continue;
        }
        let mieux = match meilleur {
            None => true,
            Some((dd, précédent)) => {
                (d, candidat.len(), candidat) < (dd, précédent.len(), précédent)
            }
        };
        if mieux {
            meilleur = Some((d, candidat));
        }
    }
    meilleur.map(|(_, c)| c)
}

/// Où commence le commentaire d'une ligne, s'il y en a un.
///
/// Le point-virgule d'une chaîne n'en ouvre pas un : `db "; pas un
/// commentaire"` est une donnée, et couper là casserait la ligne.
fn début_du_commentaire(ligne: &str) -> usize {
    let mut chaîne: Option<char> = None;
    for (i, c) in ligne.char_indices() {
        match chaîne {
            Some(ouvrant) if c == ouvrant => chaîne = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => chaîne = Some(c),
            None if c == ';' => return i,
            None => {}
        }
    }
    ligne.len()
}

/// Ce qu'on lit d'une ligne : son code, son mnémonique, ses opérandes.
///
/// Les positions sont gardées, non les copies : une correction doit
/// pouvoir remplacer un mot en laissant le reste de la ligne à l'octet
/// près — l'indentation, l'alignement des virgules, le commentaire.
struct Vue<'a> {
    ligne: &'a str,
    /// Fin de la partie code, début du commentaire.
    fin_du_code: usize,
    mné: Option<std::ops::Range<usize>>,
    ops: Vec<std::ops::Range<usize>>,
}

impl<'a> Vue<'a> {
    fn lire(ligne: &'a str) -> Self {
        let fin_du_code = début_du_commentaire(ligne);
        let code = &ligne[..fin_du_code];
        let mut i = code.len() - code.trim_start().len();

        // Une étiquette en tête ne consomme pas le mnémonique :
        // `boucle: add rax, 1` a bien une instruction.
        if let Some(deux_points) = code[i..].find(':') {
            let nom = &code[i..i + deux_points];
            if !nom.is_empty()
                && nom
                    .chars()
                    .all(|c| c.is_alphanumeric() || "_$#@~.?".contains(c))
            {
                i += deux_points + 1;
                i += code[i..].len() - code[i..].trim_start().len();
            }
        }

        let début = i;
        let fin = début
            + code[début..]
                .find(|c: char| c.is_whitespace() || c == ',')
                .unwrap_or(code.len() - début);
        let mné = (fin > début).then_some(début..fin);

        // Les opérandes, séparés par les virgules de premier niveau :
        // celles d'un `[rax+rbx*4]` ne séparent rien, pas plus que
        // celles d'une chaîne.
        let mut ops = Vec::new();
        if mné.is_some() {
            let mut profondeur = 0usize;
            let mut chaîne: Option<char> = None;
            let mut départ = fin;
            let coupe = |code: &str, a: usize, b: usize, ops: &mut Vec<_>| {
                let morceau = &code[a..b];
                let gauche = morceau.len() - morceau.trim_start().len();
                let droite = morceau.len() - morceau.trim_end().len();
                if a + gauche < b - droite {
                    ops.push(a + gauche..b - droite);
                }
            };
            for (k, c) in code[fin..].char_indices() {
                let k = fin + k;
                match chaîne {
                    Some(ouvrant) if c == ouvrant => chaîne = None,
                    Some(_) => {}
                    None if c == '"' || c == '\'' => chaîne = Some(c),
                    None if c == '[' || c == '(' => profondeur += 1,
                    None if c == ']' || c == ')' => profondeur = profondeur.saturating_sub(1),
                    None if c == ',' && profondeur == 0 => {
                        coupe(code, départ, k, &mut ops);
                        départ = k + 1;
                    }
                    None => {}
                }
            }
            coupe(code, départ, code.len(), &mut ops);
        }

        Self {
            ligne,
            fin_du_code,
            mné,
            ops,
        }
    }

    fn mot(&self, plage: &std::ops::Range<usize>) -> &'a str {
        &self.ligne[plage.clone()]
    }

    fn mnémonique(&self) -> Option<&'a str> {
        self.mné.as_ref().map(|p| self.mot(p))
    }

    fn opérande(&self, rang: usize) -> Option<&'a str> {
        self.ops.get(rang).map(|p| self.mot(p))
    }

    /// La ligne, un morceau remplacé, tout le reste intact.
    fn remplaçant(&self, plage: &std::ops::Range<usize>, par: &str) -> String {
        let mut out = String::with_capacity(self.ligne.len() + par.len());
        out.push_str(&self.ligne[..plage.start]);
        out.push_str(par);
        out.push_str(&self.ligne[plage.end..]);
        out
    }
}

/// Ce mot est-il un nombre, tel que NASM les écrit ?
fn est_un_nombre(mot: &str) -> bool {
    let mot = mot.trim();
    if mot.is_empty() {
        return false;
    }
    // `'A'` est un nombre pour NASM : le code du caractère.
    if (mot.starts_with('\'') && mot.ends_with('\'') && mot.len() >= 3)
        || (mot.starts_with('"') && mot.ends_with('"') && mot.len() >= 3)
    {
        return true;
    }
    let sans_signe = mot.strip_prefix(['-', '+']).unwrap_or(mot);
    sans_signe.starts_with(|c: char| c.is_ascii_digit())
}

/// Ce mot est-il un registre ?
fn est_un_registre(mot: &str) -> bool {
    REGISTERS.contains(&mot.to_ascii_lowercase().as_str())
}

/// La taille d'un registre général, en rang dans sa famille.
fn rang_de_taille(reg: &str) -> Option<(usize, usize)> {
    let reg = reg.to_ascii_lowercase();
    FAMILLES.iter().enumerate().find_map(|(famille, noms)| {
        noms.iter()
            .position(|n| *n == reg)
            .map(|taille| (famille, taille))
    })
}

/// Ce mnémonique saute-t-il quelque part ?
fn saute(mné: &str) -> bool {
    let mné = mné.to_ascii_lowercase();
    mné == "call"
        || mné == "jmp"
        || mné == "loop"
        || mné == "loope"
        || mné == "loopne"
        || (mné.starts_with('j') && mné.len() >= 2 && mné.len() <= 4)
}

/// La correction à proposer pour une ligne, s'il y en a une.
///
/// `source` sert au contexte — les étiquettes définies ailleurs, les
/// macros du fichier — et `index` désigne la ligne regardée. Rendre
/// `None` est le cas normal : la plupart des lignes vont bien.
pub fn propose(source: &str, index: usize, s: &Strings) -> Option<Correction> {
    let ligne = source.lines().nth(index)?;
    let vue = Vue::lire(ligne);
    let code = &ligne[..vue.fin_du_code];
    if code.trim().is_empty() {
        return None;
    }

    // 1. Un crochet ouvert et jamais refermé. C'est la seule faute qu'on
    //    corrige sans rien comprendre à la ligne, et la plus sûre.
    if let Some(correction) = crochet_manquant(&vue, code, s) {
        return Some(correction);
    }

    let mné = vue.mnémonique()?;
    let bas = mné.to_ascii_lowercase();

    // 2. Un mnémonique qui n'existe pas. Tant qu'on ne sait pas ce que
    //    la ligne fait, le reste ne se juge pas.
    if let Some(correction) = mnémonique_inconnu(&vue, source, s) {
        return Some(correction);
    }

    // 2 bis. Un opérande qui se donne pour un registre et n'en est pas
    //    un. Après le mnémonique, parce qu'une ligne dont le verbe est
    //    faux n'a pas d'opérandes à juger.
    if let Some(correction) = registre_douteux(&vue, source, s) {
        return Some(correction);
    }

    // 3. `int 0x80` : l'appel système de Linux 32 bits, écrit dans du
    //    code 64 bits. Il n'échoue pas à l'assemblage — il échoue à
    //    l'exécution, ce qui est bien plus long à comprendre.
    if bas == "int"
        && vue.ops.len() == 1
        && matches!(
            vue.opérande(0).unwrap().to_ascii_lowercase().as_str(),
            "0x80" | "80h" | "128"
        )
    {
        let début = vue.mné.as_ref().unwrap().start;
        // Le blanc entre le code et le commentaire reste : un
        // commentaire qui se colle à l'instruction n'est pas la
        // correction qu'on a demandée.
        let fin = vue.fin_du_code - (code.len() - code.trim_end().len());
        let mut ligne = String::from(&vue.ligne[..début]);
        ligne.push_str("syscall");
        ligne.push_str(&vue.ligne[fin..]);
        return Some(Correction {
            ligne,
            titre: s.fix_int80.to_owned(),
        });
    }

    // 4. Un saut vers une étiquette qui n'existe pas.
    if saute(&bas)
        && vue.ops.len() == 1
        && let Some(plage) = vue.ops.first()
    {
        let cible = vue.mot(plage);
        let nom_valide = !cible.is_empty()
            && cible
                .chars()
                .all(|c| c.is_alphanumeric() || "_$#@~.?".contains(c));
        let étiquettes = syntax::labels_of(source);
        if nom_valide
            && !est_un_nombre(cible)
            && !est_un_registre(cible)
            && !étiquettes.contains(cible)
            && let Some(proche) = plus_proche(cible, étiquettes.iter().map(String::as_str), true)
        {
            return Some(Correction {
                ligne: vue.remplaçant(plage, proche),
                titre: s
                    .fix_unknown_label
                    .replacen("{}", cible, 1)
                    .replacen("{}", proche, 1),
            });
        }
    }

    // 5. Deux registres de tailles différentes.
    if MÊME_TAILLE.contains(&bas.as_str())
        && vue.ops.len() == 2
        && let (Some(gauche), Some(droite)) = (vue.opérande(0), vue.opérande(1))
        && let (Some((_, taille_g)), Some((famille_d, taille_d))) =
            (rang_de_taille(gauche), rang_de_taille(droite))
        && taille_g != taille_d
    {
        let correct = FAMILLES[famille_d][taille_g];
        let plage = &vue.ops[1];
        return Some(Correction {
            ligne: vue.remplaçant(plage, correct),
            titre: s
                .fix_register_width
                .replacen("{}", droite, 1)
                .replacen("{}", gauche, 1)
                .replacen("{}", correct, 1),
        });
    }

    // 6. Une écriture en mémoire dont rien ne dit la taille.
    if let Some(correction) = taille_manquante(&vue, &bas, s) {
        return Some(correction);
    }

    None
}

/// 1. Le crochet qu'on a oublié de refermer.
fn crochet_manquant(vue: &Vue<'_>, code: &str, s: &Strings) -> Option<Correction> {
    let mut profondeur = 0i32;
    let mut chaîne: Option<char> = None;
    for c in code.chars() {
        match chaîne {
            Some(ouvrant) if c == ouvrant => chaîne = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => chaîne = Some(c),
            None if c == '[' => profondeur += 1,
            None if c == ']' => profondeur -= 1,
            None => {}
        }
    }
    if profondeur <= 0 {
        return None;
    }
    // Refermé au bout du code, avant le commentaire : c'est là que
    // l'expression s'arrête.
    let fin = vue.fin_du_code - (code.len() - code.trim_end().len());
    let mut ligne = String::from(&vue.ligne[..fin]);
    for _ in 0..profondeur {
        ligne.push(']');
    }
    ligne.push_str(&vue.ligne[fin..]);
    Some(Correction {
        ligne,
        titre: s.fix_unclosed_bracket.to_owned(),
    })
}

/// 2. Le mnémonique qui n'existe pas, à une ou deux lettres près d'un
///    qui existe.
///
/// La détection est celle de la coloration — `Token::UnknownMnemonic` —
/// et non une seconde règle écrite ici : ce que l'éditeur peint en rouge
/// est exactement ce qu'on propose de corriger, et les deux ne peuvent
/// pas diverger.
fn mnémonique_inconnu(vue: &Vue<'_>, source: &str, s: &Strings) -> Option<Correction> {
    let macros = syntax::Macros::of(source);
    let plage = vue.mné.as_ref()?;
    let mot = vue.mot(plage);
    let inconnu = syntax::tokenize_with(vue.ligne, &macros)
        .into_iter()
        .any(|(m, t)| t == Token::UnknownMnemonic && std::ptr::eq(m.as_ptr(), mot.as_ptr()));
    if !inconnu {
        return None;
    }
    // Les instructions documentées, non les mille huit cents que le
    // décodeur connaît : proposer une extension vectorielle obscure à
    // qui a tapé trois lettres de travers n'aide personne.
    let proche = plus_proche(mot, namsdec_doc::mnemonics().iter().copied(), false)?;
    // La casse de ce qu'on remplace : qui écrit en majuscules continue.
    let proche = if mot.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        proche.to_ascii_uppercase()
    } else {
        proche.to_owned()
    };
    Some(Correction {
        ligne: vue.remplaçant(plage, &proche),
        titre: s
            .fix_unknown_mnemonic
            .replacen("{}", mot, 1)
            .replacen("{}", &proche, 1),
    })
}

/// 2 bis. Un opérande qui se donne pour un registre et n'en est pas un.
///
/// **Le défaut qu'elle corrige, vu à l'écran le 16 novembre 2026.**
/// `mov rzx, 1` fait répondre à nasm « invalid combination of opcode
/// and operands », et l'explication que namsdec posait dessous parlait
/// de deux opérandes en mémoire et de tailles qui ne correspondent
/// pas — un diagnostic juste en général, et faux pour cette ligne-là. La
/// vraie cause tient pourtant en quatre mots : **il n'y a pas de
/// registre `rzx`**, et nasm ne le dira jamais, puisqu'il ne voit qu'un
/// symbole qu'il ne connaît pas.
///
/// **Ce qu'on exige avant d'accuser**, parce qu'une correction qu'on ne
/// sait pas justifier apprend le faux :
///
/// - le mot a la FORME d'un registre : deux à quatre caractères, des
///   lettres et des chiffres, commençant par `r` ou `e` ;
/// - il n'est pas un registre, pas un nombre, pas un mot-clé de taille ;
/// - **il n'est défini nulle part dans le programme** — ni étiquette, ni
///   constante, ni donnée. C'est la condition qui écarte `msg`, `buf` et
///   tout ce que l'auteur a nommé lui-même ;
/// - il est à **une seule lettre** d'un vrai registre.
///
/// **Et l'on ne réécrit que si un seul candidat répond.** `rzx` est à
/// une lettre de `rax`, `rbx`, `rcx` ET `rdx` : en choisir un serait
/// tirer au sort sous couvert de correction. Dans ce cas la ligne reste
/// intacte et c'est [`registre_inconnu`] qui parle, en les nommant tous
/// les quatre — c'est la liste qui enseigne, pas le choix.
fn registre_douteux(vue: &Vue<'_>, source: &str, s: &Strings) -> Option<Correction> {
    let (plage, _, candidats) = suspect_de_registre(vue, source)?;
    let [seul] = candidats[..] else {
        return None;
    };
    let mot = vue.mot(&plage);
    let proche = if mot.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        seul.to_ascii_uppercase()
    } else {
        seul.to_owned()
    };
    Some(Correction {
        ligne: vue.remplaçant(&plage, &proche),
        titre: s
            .fix_unknown_register
            .replacen("{}", mot, 1)
            .replacen("{}", &proche, 1),
    })
}

/// Le mot d'une ligne qui se donne pour un registre sans en être un, et
/// les vrais registres dont il est le plus proche.
///
/// Rendu pour la ligne entière plutôt que pour la correction : quand
/// plusieurs registres sont à égale distance, il n'y a rien à réécrire,
/// mais il y a tout à dire. C'est ce que lit la liste des problèmes.
pub fn registre_inconnu(source: &str, index: usize) -> Option<(String, Vec<&'static str>)> {
    let ligne = source.lines().nth(index)?;
    let vue = Vue::lire(ligne);
    let (plage, mot, candidats) = suspect_de_registre(&vue, source)?;
    let _ = plage;
    Some((mot, candidats))
}

/// Le travail commun aux deux : trouver l'opérande fautif.
fn suspect_de_registre<'a>(
    vue: &Vue<'a>,
    source: &str,
) -> Option<(std::ops::Range<usize>, String, Vec<&'static str>)> {
    // Une ligne dont le mnémonique n'existe pas se juge d'abord sur son
    // mnémonique : deux reproches à la fois sur une même ligne, et l'on
    // ne sait plus lequel lire.
    let mné = vue.mnémonique()?;
    if !namsdec_disasm::is_mnemonic(&mné.to_ascii_lowercase()) {
        return None;
    }
    let étiquettes = syntax::labels_of(source);
    let macros = syntax::Macros::of(source);
    for plage in &vue.ops {
        let mot = vue.mot(plage);
        // L'opérande nu, sans crochets ni taille : `[rzx]` cache le même
        // mot que `rzx`, et c'est lui qu'on juge.
        let nu = mot.trim_matches(|c: char| c == '[' || c == ']' || c.is_whitespace());
        if nu.len() < 2 || nu.len() > 4 {
            continue;
        }
        let bas = nu.to_ascii_lowercase();
        if !bas.starts_with('r') && !bas.starts_with('e') {
            continue;
        }
        if !bas.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        if est_un_registre(&bas)
            || est_un_nombre(nu)
            || TAILLES.contains(&bas.as_str())
            || syntax::DIRECTIVES.contains(&bas.as_str())
            || étiquettes.contains(nu)
            || macros.contient(&bas)
        {
            continue;
        }
        // À UNE LETTRE, ET PAS DEUX. `rax` et `rdx` sont déjà à deux
        // l'un de l'autre : tolérer deux ferait de n'importe quel nom de
        // trois lettres un registre mal tapé.
        let candidats: Vec<&'static str> = syntax::REGISTERS
            .iter()
            .copied()
            .filter(|r| distance(&bas, r) == 1)
            .collect();
        if candidats.is_empty() {
            continue;
        }
        return Some((plage.clone(), nu.to_owned(), candidats));
    }
    None
}

/// 6. L'écriture en mémoire dont rien ne dit la taille.
///
/// `mov [compteur], 1` écrit-il un octet ou huit ? NASM ne devine pas
/// et refuse. Un registre en face lèverait le doute — `mov [compteur],
/// rax` écrit huit octets, personne n'hésite — mais un nombre nu ne dit
/// rien.
fn taille_manquante(vue: &Vue<'_>, bas: &str, s: &Strings) -> Option<Correction> {
    if !ÉCRIT_EN_MÉMOIRE.contains(&bas) {
        return None;
    }
    // Une taille déjà écrite quelque part sur la ligne suffit.
    let code = &vue.ligne[..vue.fin_du_code].to_ascii_lowercase();
    if TAILLES.iter().any(|t| {
        code.split(|c: char| !c.is_alphanumeric())
            .any(|mot| mot == *t)
    }) {
        return None;
    }
    let mémoire = vue.ops.iter().position(|p| vue.mot(p).contains('['))?;
    // Un registre en face dit la taille ; deux opérandes dont aucun
    // n'est un registre, ou un seul opérande, ne la disent pas.
    let dite = match vue.ops.len() {
        1 => false,
        2 => {
            let autre = vue.opérande(1 - mémoire)?;
            est_un_registre(autre)
        }
        _ => return None,
    };
    if dite {
        return None;
    }
    // `push` et `pop` sur 64 bits n'ont qu'une taille possible : rien à
    // préciser, et NASM ne s'en plaint pas.
    if bas == "push" || bas == "pop" {
        return None;
    }
    let plage = &vue.ops[mémoire];
    let morceau = vue.mot(plage);
    Some(Correction {
        ligne: vue.remplaçant(plage, &format!("qword {morceau}")),
        titre: s.fix_missing_size.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{Lang, strings};

    /// Corrige la ligne demandée, ou rend `None`.
    fn corrige(source: &str, index: usize) -> Option<String> {
        propose(source, index, strings(Lang::Fr)).map(|c| c.ligne)
    }

    /// Le titre de la correction, pour vérifier ce qu'on explique.
    fn titre(source: &str, index: usize) -> Option<String> {
        propose(source, index, strings(Lang::Fr)).map(|c| c.titre)
    }

    /// Une ligne juste ne reçoit rien.
    ///
    /// C'est la moitié du travail : une proposition qui s'affiche
    /// devant du code correct apprend le doute, et l'on finit par ne
    /// plus la lire.
    #[test]
    fn a_correct_line_gets_nothing() {
        let programme = "\
global _start
section .data
message db \"salut\", 10
section .text
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, message
    mov rdx, 6
    syscall
    mov qword [rsp-8], 0
    jmp _start
    rep movsb
    lock xchg [rsp], rax
    push rbp
    pop rbp
    movzx eax, al
";
        for (i, ligne) in programme.lines().enumerate() {
            assert_eq!(
                corrige(programme, i),
                None,
                "rien à corriger ligne {i} : « {ligne} »"
            );
        }
    }

    /// Le crochet qu'on oublie de refermer.
    #[test]
    fn an_unclosed_bracket_is_closed() {
        let src = "    mov rax, [rbp-8\n";
        assert_eq!(corrige(src, 0).as_deref(), Some("    mov rax, [rbp-8]"));

        // Le commentaire reste derrière, et le crochet se pose avant lui.
        let src = "    mov rax, [rbp-8   ; le compteur\n";
        assert_eq!(
            corrige(src, 0).as_deref(),
            Some("    mov rax, [rbp-8]   ; le compteur")
        );

        // Un point-virgule dans une chaîne n'ouvre pas de commentaire,
        // et un crochet dans une chaîne n'ouvre rien du tout.
        assert_eq!(corrige("txt db \"[pas un crochet\"\n", 0), None);
    }

    /// La coquille de mnémonique, à une ou deux lettres près.
    #[test]
    fn a_misspelled_mnemonic_is_proposed_the_right_one() {
        assert_eq!(
            corrige("    mvo rax, 1\n", 0).as_deref(),
            Some("    mov rax, 1")
        );
        assert_eq!(
            corrige("    puhs rbp\n", 0).as_deref(),
            Some("    push rbp")
        );

        // L'indentation et l'alignement survivent : on remplace un mot,
        // pas une ligne.
        assert_eq!(
            corrige("        sycall          ; l'appel\n", 0).as_deref(),
            Some("        syscall          ; l'appel")
        );

        // La casse suit celle qu'on employait.
        assert_eq!(
            corrige("    MVO RAX, 1\n", 0).as_deref(),
            Some("    MOV RAX, 1")
        );

        // Une étiquette en tête ne prend pas la place du mnémonique.
        assert_eq!(
            corrige("boucle: mvo rax, 1\n", 0).as_deref(),
            Some("boucle: mov rax, 1")
        );

        // Ce qui est trop loin ne reçoit rien : on ne devine pas.
        assert_eq!(corrige("    tralala rax, 1\n", 0), None);

        // Et le message dit les deux mots.
        let dit = titre("    mvo rax, 1\n", 0).unwrap();
        assert!(dit.contains("mvo") && dit.contains("mov"), "{dit}");
    }

    /// Une macro du fichier n'est pas une coquille.
    #[test]
    fn a_macro_of_the_file_is_not_a_typo() {
        let src = "%macro sortir 1\n    mov rax, 60\n%endmacro\n    sortir 0\n";
        assert_eq!(
            corrige(src, 3),
            None,
            "« sortir » est défini juste au-dessus"
        );
    }

    /// `int 0x80` dans du code 64 bits.
    ///
    /// Il s'assemble sans un mot et échoue à l'exécution, ce qui est
    /// bien plus long à comprendre qu'une erreur de nasm.
    #[test]
    fn int_80_becomes_syscall() {
        assert_eq!(corrige("    int 0x80\n", 0).as_deref(), Some("    syscall"));
        assert_eq!(corrige("    int 80h\n", 0).as_deref(), Some("    syscall"));
        assert_eq!(
            corrige("    int 0x80    ; sortir\n", 0).as_deref(),
            Some("    syscall    ; sortir")
        );
        // Les autres interruptions ne se touchent pas.
        assert_eq!(corrige("    int 3\n", 0), None);
    }

    /// Le saut vers une étiquette qui n'existe pas.
    #[test]
    fn a_jump_to_a_missing_label_finds_the_right_one() {
        let src = "boucle:\n    add rax, 1\n    jmp bouce\n";
        assert_eq!(corrige(src, 2).as_deref(), Some("    jmp boucle"));

        // Une étiquette qui existe ne reçoit rien.
        let src = "boucle:\n    jmp boucle\n";
        assert_eq!(corrige(src, 1), None);

        // Et l'on ne corrige pas un saut vers un nom qui ne ressemble à
        // rien de connu : ce serait envoyer le programme ailleurs.
        let src = "boucle:\n    jmp ailleurs\n";
        assert_eq!(corrige(src, 1), None);

        // La casse compte, comme pour NASM : `Boucle` n'est pas
        // `boucle`, et c'est justement la faute qu'on veut voir.
        let src = "boucle:\n    jmp Boucle\n";
        assert_eq!(corrige(src, 1).as_deref(), Some("    jmp boucle"));
    }

    /// Deux registres de tailles différentes.
    #[test]
    fn two_registers_of_different_widths() {
        assert_eq!(
            corrige("    mov rax, ebx\n", 0).as_deref(),
            Some("    mov rax, rbx")
        );
        assert_eq!(
            corrige("    add eax, bl\n", 0).as_deref(),
            Some("    add eax, ebx")
        );
        assert_eq!(
            corrige("    cmp r8, r9d\n", 0).as_deref(),
            Some("    cmp r8, r9")
        );

        // `movzx` et `movsx` existent pour cela : on ne les corrige pas.
        assert_eq!(corrige("    movzx rax, bl\n", 0), None);
        // Même taille : rien.
        assert_eq!(corrige("    mov rax, rbx\n", 0), None);

        // Le message nomme les trois : ce qu'on a écrit, en face de
        // quoi, et ce qu'on propose.
        let dit = titre("    mov rax, ebx\n", 0).unwrap();
        assert!(
            dit.contains("ebx") && dit.contains("rax") && dit.contains("rbx"),
            "{dit}"
        );
    }

    /// L'écriture en mémoire dont rien ne dit la taille.
    #[test]
    fn a_write_to_memory_needs_its_size() {
        assert_eq!(
            corrige("    mov [compteur], 1\n", 0).as_deref(),
            Some("    mov qword [compteur], 1")
        );
        assert_eq!(
            corrige("    inc [rbp-8]\n", 0).as_deref(),
            Some("    inc qword [rbp-8]")
        );
        // Un registre en face dit la taille : rien à ajouter.
        assert_eq!(corrige("    mov [compteur], rax\n", 0), None);
        // Une taille déjà écrite : rien non plus.
        assert_eq!(corrige("    mov byte [compteur], 1\n", 0), None);
        // `push` et `pop` n'ont qu'une taille possible sur 64 bits.
        assert_eq!(corrige("    push [rsp]\n", 0), None);
        // Et la virgule d'une adresse complexe ne coupe pas les
        // opérandes : `[rax+rbx*4]` est un seul opérande.
        assert_eq!(
            corrige("    mov [rax+rbx*4], 7\n", 0).as_deref(),
            Some("    mov qword [rax+rbx*4], 7")
        );
    }

    /// La distance d'édition compte ce qu'on croit qu'elle compte.
    #[test]
    fn the_edit_distance_counts_what_we_think() {
        assert_eq!(distance("mov", "mov"), 0);
        assert_eq!(
            distance("mvo", "mov"),
            1,
            "deux lettres échangées comptent pour une"
        );
        assert_eq!(distance("mav", "mov"), 1, "une lettre pour une autre");
        assert_eq!(distance("mo", "mov"), 1);
        assert_eq!(distance("", "mov"), 3);
    }

    /// Sur un mot court, une seule faute est tolérée.
    ///
    /// `mov` et `mul` ne sont séparés que d'une lettre, `add` et `and`
    /// aussi : accepter deux fautes sur trois lettres reviendrait à
    /// tirer au sort.
    #[test]
    fn a_short_word_tolerates_only_one_mistake() {
        assert_eq!(
            plus_proche("mo", ["mov", "mul"].into_iter(), false),
            Some("mov")
        );
        assert_eq!(plus_proche("xyz", ["mov", "mul"].into_iter(), false), None);
        // Plus long, deux fautes restent une coquille.
        assert_eq!(
            plus_proche("sycal", ["syscall"].into_iter(), false),
            Some("syscall")
        );
    }

    /// Les lignes vides, les commentaires seuls et les directives ne
    /// reçoivent rien.
    #[test]
    fn nothing_is_proposed_where_there_is_nothing_to_fix() {
        for ligne in [
            "",
            "   ",
            "; un commentaire",
            "section .text",
            "global _start",
            "_start:",
            "%include \"macros.inc\"",
        ] {
            let src = format!("{ligne}\n");
            assert_eq!(corrige(&src, 0), None, "« {ligne} »");
        }
    }

    /// Une ligne hors du fichier ne fait pas tomber l'appel.
    #[test]
    fn a_line_past_the_end_is_not_a_crash() {
        assert_eq!(corrige("mov rax, 1\n", 42), None);
    }

    /// Un registre qui n'existe pas se nomme, et la ligne reste intacte
    /// quand plusieurs vrais registres en sont également proches.
    ///
    /// C'est le cas vu à l'écran le 16 novembre 2026 : `mov rzx, 1`,
    /// que nasm refuse avec « invalid combination of opcode and
    /// operands » — un message qui ne nomme pas le coupable, et dont
    /// l'explication générique parlait de deux opérandes en mémoire.
    #[test]
    fn a_register_that_does_not_exist_is_named() {
        let source = "_start:\n    mov rzx, 1\n";
        let (mot, proches) = registre_inconnu(source, 1).expect("rzx n'est pas un registre");
        assert_eq!(mot, "rzx");
        assert_eq!(
            proches,
            vec!["rax", "rbx", "rcx", "rdx"],
            "les quatre à une lettre, et aucun choisi d'office"
        );
        assert_eq!(
            corrige(source, 1),
            None,
            "quatre candidats à égalité : on ne réécrit pas"
        );
    }

    /// Un seul candidat, et la ligne se réécrit.
    #[test]
    fn a_single_candidate_rewrites_the_line() {
        // `rbpp` n'est à une lettre que de `rbp` — là où `rzx` l'est
        // de quatre registres à la fois, les huit premiers partageant
        // leurs deux dernières lettres.
        let source = "_start:\n    mov rsp, rbpp\n";
        assert_eq!(corrige(source, 1).as_deref(), Some("    mov rsp, rbp"));
        assert!(
            titre(source, 1).is_some_and(|t| t.contains("rbpp") && t.contains("rbp")),
            "le titre nomme les deux : {:?}",
            titre(source, 1)
        );
    }

    /// CE QUE L'AUTEUR A NOMMÉ LUI-MÊME N'EST PAS UNE FAUTE.
    ///
    /// La moitié qui compte : une étiquette, une constante `equ`, une
    /// macro ou une donnée portent des noms courts qui ressemblent à
    /// des registres. Les accuser ferait de cette règle un bruit de
    /// fond, et apprendrait le faux.
    #[test]
    fn what_the_program_defines_is_never_accused() {
        for (source, ligne) in [
            ("ret0:\n    jmp ret0\n", 1),
            ("eax2 equ 4\n_start:\n    mov rax, eax2\n", 2),
            ("%define rax1 7\n_start:\n    mov rax, rax1\n", 2),
            ("msg db \"x\"\n_start:\n    mov rax, msg\n", 2),
            // Un vrai registre, évidemment.
            ("_start:\n    mov rax, rbx\n", 1),
            // Et un nom qui ne ressemble à aucun registre.
            ("_start:\n    mov rax, compteur\n", 1),
        ] {
            assert_eq!(
                registre_inconnu(source, ligne).map(|(m, _)| m),
                None,
                "« {source} »"
            );
        }
    }

    /// Une ligne dont le mnémonique est déjà faux ne reçoit qu'un
    /// reproche : le sien.
    #[test]
    fn a_wrong_mnemonic_comes_first() {
        let source = "_start:\n    mvo rzx, 1\n";
        assert_eq!(
            registre_inconnu(source, 1).map(|(m, _)| m),
            None,
            "tant que le verbe est faux, les opérandes ne se jugent pas"
        );
        assert!(
            titre(source, 1).is_some_and(|t| t.contains("mvo") && t.contains("mov")),
            "c'est le verbe qu'on corrige d'abord : {:?}",
            titre(source, 1)
        );
    }
}
