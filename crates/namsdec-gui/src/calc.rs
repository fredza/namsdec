//! Une calculatrice pour la rétro-ingénierie.
//!
//! Le besoin est banal et revient tout le temps : on lit `and eax,
//! 0x1f`, on veut savoir ce que ça garde ; on voit `shr rax, 3`, on veut
//! le diviseur ; on trouve `0x6c6c6568` dans un registre et l'on se
//! demande quelles lettres c'est. Aujourd'hui on sort de namsdec pour
//! chercher une calculatrice ailleurs — et celle du système ne sait ni
//! lire l'hexadécimal sans cliquer, ni permuter des octets, ni dire
//! qu'une valeur est aussi une chaîne.
//!
//! Deux moitiés, séparées exprès :
//!
//! - **l'évaluation** — ce fichier —, qui ne connaît ni egui ni écran, et
//!   se teste donc au caractère près ;
//! - **l'affichage**, dans l'interface, qui n'a plus qu'à montrer ce
//!   qu'on lui donne.
//!
//! Tout se calcule sur 64 bits non signés, et **déborde en silence** —
//! comme le processeur. `0 - 1` vaut donc `0xffff_ffff_ffff_ffff`, ce qui
//! est la bonne réponse : c'est ce que le registre contiendra.

use namsdec_core::Registers;

/// Pourquoi une expression n'a pas pu être lue.
///
/// Chaque cas se dit en clair, parce qu'une calculatrice qui répond
/// « erreur » n'apprend rien : ce qu'on veut savoir, c'est *où* ça a
/// coincé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalcError {
    /// Rien à calculer.
    Empty,
    /// Un caractère qui n'a de sens nulle part.
    BadChar(char),
    /// Un mot qui n'est ni un nombre, ni un registre, ni une fonction.
    Unknown(String),
    /// Une parenthèse ouverte et jamais refermée, ou l'inverse.
    Unbalanced,
    /// Un opérateur sans son opérande : `1 +`, `* 3`.
    Missing,
    /// Cette fonction-là prend un autre nombre d'arguments.
    Arity {
        name: &'static str,
        wanted: usize,
        got: usize,
    },
    /// Un registre demandé alors qu'aucun programme ne tourne.
    NoRegisters(String),
    /// Diviser par zéro. Le processeur lève une exception ; ici on le
    /// dit, ce qui vaut mieux que de rendre un nombre inventé.
    DivideByZero,
    /// Ce qui reste après une expression complète : `1 2`.
    Trailing(String),
    /// Une lecture mémoire demandée alors qu'aucun programme ne tourne.
    NoMemory,
    /// Une adresse que le programme ne peut pas lire — hors de ses
    /// pages, ou dans une page sans droit de lecture.
    Unreadable(u64),
}

impl CalcError {
    /// L'expression est-elle **mal écrite**, ou seulement incalculable
    /// ici et maintenant ?
    ///
    /// La distinction n'est pas une subtilité : `[rsp+8] != 0` est une
    /// condition d'arrêt parfaitement valide, qu'on écrit avant que le
    /// programme ne tourne — et à ce moment-là il n'y a ni registre ni
    /// mémoire. Refuser le champ parce qu'on ne sait pas encore la
    /// calculer reviendrait à interdire d'écrire une condition avant de
    /// lancer, c'est-à-dire au seul moment où l'on en écrit une.
    ///
    /// Sont donc « incalculables » et non fautives : le programme absent
    /// (`NoRegisters`, `NoMemory`), l'adresse illisible, et la division
    /// par zéro, qui dépend des valeurs du moment. Tout le reste est une
    /// faute de frappe, et se dit tout de suite.
    pub fn is_syntax(&self) -> bool {
        !matches!(
            self,
            CalcError::NoRegisters(_)
                | CalcError::NoMemory
                | CalcError::Unreadable(_)
                | CalcError::DivideByZero
        )
    }
}

/// De quoi lire la mémoire du programme, quand l'expression le demande.
///
/// Un trait et non la mémoire elle-même : ce fichier ne connaît ni
/// l'émulateur ni egui, et c'est ce qui permet de le tester au caractère
/// près. L'appelant branche ce qu'il veut derrière.
///
/// La lecture doit être **sans effet** : évaluer une condition d'arrêt ne
/// doit ni déclencher une surveillance, ni compter comme un accès du
/// programme. C'est un coup d'œil, pas une instruction.
pub trait Memory {
    /// Les `taille` octets à cette adresse, lus en petit-boutiste, ou
    /// `None` si l'adresse n'est pas lisible.
    fn peek(&self, addr: u64, taille: usize) -> Option<u64>;
}

/// Les fonctions que la calculatrice connaît, avec ce qu'elles font.
///
/// Toutes celles qu'on écrit à la main en lisant un désassemblage, et
/// aucune qu'on n'y écrirait jamais. La liste sert aussi d'aide-mémoire à
/// l'interface : elle s'affiche telle quelle.
pub const FUNCTIONS: &[(&str, usize, &str, &str)] = &[
    (
        "bswap16",
        1,
        "permute les 2 octets de bas",
        "swaps the low 2 bytes",
    ),
    (
        "bswap32",
        1,
        "permute les 4 octets de bas",
        "swaps the low 4 bytes",
    ),
    (
        "bswap64",
        1,
        "permute les 8 octets",
        "swaps all 8 bytes",
    ),
    (
        "sext8",
        1,
        "étend le signe d'un octet",
        "sign-extends a byte",
    ),
    (
        "sext16",
        1,
        "étend le signe de 16 bits",
        "sign-extends 16 bits",
    ),
    (
        "sext32",
        1,
        "étend le signe de 32 bits",
        "sign-extends 32 bits",
    ),
    ("lo8", 1, "garde l'octet de bas", "keeps the low byte"),
    ("lo16", 1, "garde les 16 bits de bas", "keeps the low 16 bits"),
    ("lo32", 1, "garde les 32 bits de bas", "keeps the low 32 bits"),
    ("hi32", 1, "garde les 32 bits de haut", "keeps the high 32 bits"),
    ("popcnt", 1, "compte les bits à 1", "counts the set bits"),
    ("clz", 1, "zéros de tête", "leading zeros"),
    ("ctz", 1, "zéros de queue", "trailing zeros"),
    ("rol", 2, "rotation à gauche sur 64 bits", "64-bit left rotate"),
    ("ror", 2, "rotation à droite sur 64 bits", "64-bit right rotate"),
    (
        "align",
        2,
        "arrondit au multiple supérieur",
        "rounds up to a multiple",
    ),
    ("min", 2, "le plus petit des deux", "the smaller of the two"),
    ("max", 2, "le plus grand des deux", "the larger of the two"),
];

/// Évalue une expression.
///
/// `regs` donne accès aux registres du programme en cours, quand il y en
/// a un : `rax & 0xff` se lit alors comme on le pense. Sans programme,
/// nommer un registre est une erreur qui le dit, plutôt qu'un zéro
/// silencieux qui ferait croire à un résultat.
pub fn eval(expr: &str, regs: Option<&Registers>) -> Result<u64, CalcError> {
    eval_with(expr, regs, None)
}

/// La même, en sachant aussi lire la mémoire.
///
/// `[rsp+8]` lit alors huit octets à cette adresse, et `byte [rdi]` un
/// seul — la même grammaire que NASM, parce que c'est celle qu'on a sous
/// les yeux dans le listing.
pub fn eval_with(
    expr: &str,
    regs: Option<&Registers>,
    mem: Option<&dyn Memory>,
) -> Result<u64, CalcError> {
    let tokens = lex(expr)?;
    if tokens.is_empty() {
        return Err(CalcError::Empty);
    }
    let mut p = Parser {
        tokens: &tokens,
        at: 0,
        regs,
        mem,
    };
    let value = p.expression(0)?;
    // Ce qui reste est une faute, et non quelque chose à ignorer : `1 2`
    // n'est pas `1`, c'est une expression qu'on a mal tapée.
    if let Some(reste) = p.tokens.get(p.at) {
        return Err(CalcError::Trailing(reste.to_text()));
    }
    Ok(value)
}

// --- la lecture des mots -------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Num(u64),
    Word(String),
    Op(&'static str),
    Open,
    Close,
    /// `[` : le début d'une lecture mémoire.
    OpenBracket,
    /// `]`.
    CloseBracket,
    Comma,
}

impl Tok {
    fn to_text(&self) -> String {
        match self {
            Tok::Num(n) => n.to_string(),
            Tok::Word(w) => w.clone(),
            Tok::Op(o) => (*o).to_owned(),
            Tok::Open => "(".to_owned(),
            Tok::Close => ")".to_owned(),
            Tok::OpenBracket => "[".to_owned(),
            Tok::CloseBracket => "]".to_owned(),
            Tok::Comma => ",".to_owned(),
        }
    }
}

/// Découpe l'expression en mots.
fn lex(expr: &str) -> Result<Vec<Tok>, CalcError> {
    let chars: Vec<char> = expr.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '(' => {
                out.push(Tok::Open);
                i += 1;
            }
            ')' => {
                out.push(Tok::Close);
                i += 1;
            }
            '[' => {
                out.push(Tok::OpenBracket);
                i += 1;
            }
            ']' => {
                out.push(Tok::CloseBracket);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            // Un caractère entre apostrophes est son code : `'A'` vaut
            // 65. C'est la lecture qu'on cherche devant un `cmp al, 0x41`.
            '\'' => {
                let (valeur, avance) = read_char_literal(&chars, i)?;
                out.push(Tok::Num(valeur));
                i += avance;
            }
            c if c.is_ascii_digit() => {
                let (valeur, avance) = read_number(&chars, i)?;
                out.push(Tok::Num(valeur));
                i += avance;
            }
            c if c.is_alphabetic() || c == '_' => {
                let début = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push(Tok::Word(chars[début..i].iter().collect()));
            }
            _ => {
                // Les opérateurs de deux caractères d'abord : `<<` n'est
                // pas deux fois `<`.
                let deux: String = chars[i..(i + 2).min(chars.len())].iter().collect();
                if let Some(op) = ["<<", ">>"].iter().find(|o| **o == deux) {
                    out.push(Tok::Op(op));
                    i += 2;
                    continue;
                }
                let un = ["+", "-", "*", "/", "%", "&", "|", "^", "~"]
                    .iter()
                    .find(|o| o.starts_with(c));
                match un {
                    Some(op) => {
                        out.push(Tok::Op(op));
                        i += 1;
                    }
                    None => return Err(CalcError::BadChar(c)),
                }
            }
        }
    }
    Ok(out)
}

/// Lit un nombre dans la base que son préfixe annonce.
///
/// `0x` hexadécimal, `0b` binaire, `0o` octal, rien du tout décimal — et
/// `h` en suffixe, que NASM écrit et qu'on recopie sans y penser.
fn read_number(chars: &[char], start: usize) -> Result<(u64, usize), CalcError> {
    let mut i = start;
    let (base, saut) = if chars[i] == '0' && i + 1 < chars.len() {
        match chars[i + 1].to_ascii_lowercase() {
            'x' => (16, 2),
            'b' => (2, 2),
            'o' => (8, 2),
            _ => (10, 0),
        }
    } else {
        (10, 0)
    };
    i += saut;
    let début = i;
    // Le souligné sépare les groupes : `0b1010_1010` se lit comme on
    // l'écrit dans du code.
    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    let brut: String = chars[début..i].iter().filter(|c| **c != '_').collect();
    if brut.is_empty() {
        return Err(CalcError::Unknown(chars[start..i].iter().collect()));
    }
    // Le suffixe `h` de NASM : `0FFh`. Seulement quand rien n'annonçait
    // déjà une base, sinon `0xbh` passerait.
    let (texte, base) = match brut.strip_suffix(['h', 'H']) {
        Some(sans) if base == 10 && !sans.is_empty() => (sans.to_owned(), 16),
        _ => (brut.clone(), base),
    };
    u64::from_str_radix(&texte, base)
        .map(|v| (v, i - start))
        .map_err(|_| CalcError::Unknown(chars[start..i].iter().collect()))
}

/// Lit `'A'`, ou `'\n'`.
fn read_char_literal(chars: &[char], start: usize) -> Result<(u64, usize), CalcError> {
    let mut i = start + 1;
    let Some(&c) = chars.get(i) else {
        return Err(CalcError::Unbalanced);
    };
    let valeur = if c == '\\' {
        i += 1;
        let Some(&échappé) = chars.get(i) else {
            return Err(CalcError::Unbalanced);
        };
        match échappé {
            'n' => 10u64,
            't' => 9,
            'r' => 13,
            '0' => 0,
            '\\' => 92,
            '\'' => 39,
            autre => autre as u64,
        }
    } else {
        c as u64
    };
    i += 1;
    if chars.get(i) != Some(&'\'') {
        return Err(CalcError::Unbalanced);
    }
    Ok((valeur, i + 1 - start))
}

// --- l'analyse -----------------------------------------------------

struct Parser<'a> {
    tokens: &'a [Tok],
    at: usize,
    regs: Option<&'a Registers>,
    mem: Option<&'a dyn Memory>,
}

/// Les mots qui disent la taille d'une lecture mémoire, et combien
/// d'octets ils valent. Ceux de NASM, parce que c'est ce qu'on lit dans
/// le listing juste au-dessus du champ.
const TAILLES: &[(&str, usize)] = &[
    ("byte", 1),
    ("word", 2),
    ("dword", 4),
    ("qword", 8),
];

/// La force d'un opérateur binaire. Celle du C, parce que c'est celle
/// qu'on a en tête en lisant du code — et que se tromper là-dessus
/// donnerait des résultats justes en apparence.
fn precedence(op: &str) -> Option<u8> {
    Some(match op {
        "|" => 1,
        "^" => 2,
        "&" => 3,
        "<<" | ">>" => 4,
        "+" | "-" => 5,
        "*" | "/" | "%" => 6,
        _ => return None,
    })
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.at)
    }

    /// Une expression dont tous les opérateurs sont au moins aussi forts
    /// que `mini`.
    fn expression(&mut self, mini: u8) -> Result<u64, CalcError> {
        let mut gauche = self.unary()?;
        while let Some(Tok::Op(op)) = self.peek() {
            let Some(force) = precedence(op) else { break };
            if force < mini {
                break;
            }
            let op = *op;
            self.at += 1;
            // `force + 1` : nos opérateurs associent tous à gauche, si
            // bien que `1 - 2 - 3` fait -4 et non 2.
            let droite = self.expression(force + 1)?;
            gauche = apply(op, gauche, droite)?;
        }
        Ok(gauche)
    }

    fn unary(&mut self) -> Result<u64, CalcError> {
        match self.peek() {
            Some(Tok::Op("-")) => {
                self.at += 1;
                Ok(0u64.wrapping_sub(self.unary()?))
            }
            Some(Tok::Op("~")) => {
                self.at += 1;
                Ok(!self.unary()?)
            }
            Some(Tok::Op("+")) => {
                self.at += 1;
                self.unary()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<u64, CalcError> {
        let Some(tok) = self.peek().cloned() else {
            return Err(CalcError::Missing);
        };
        match tok {
            Tok::Num(n) => {
                self.at += 1;
                Ok(n)
            }
            Tok::Open => {
                self.at += 1;
                let valeur = self.expression(0)?;
                if self.peek() != Some(&Tok::Close) {
                    return Err(CalcError::Unbalanced);
                }
                self.at += 1;
                Ok(valeur)
            }
            Tok::OpenBracket => {
                self.at += 1;
                // Sans taille dite, huit octets : c'est ce que vaut un
                // mot sur cette machine, et ce qu'on lit neuf fois sur
                // dix. `byte [rdi]` sert à le restreindre.
                self.memory(8)
            }
            Tok::Word(mot) => {
                self.at += 1;
                // Une fonction se reconnaît à la parenthèse qui suit ;
                // sans elle, c'est un registre — ou un mot de taille, si
                // un crochet suit.
                if self.peek() == Some(&Tok::Open) {
                    return self.call(&mot);
                }
                if self.peek() == Some(&Tok::OpenBracket)
                    && let Some(&(_, octets)) = TAILLES
                        .iter()
                        .find(|(nom, _)| *nom == mot.to_ascii_lowercase())
                {
                    self.at += 1;
                    return self.memory(octets);
                }
                self.register(&mot)
            }
            Tok::Close | Tok::CloseBracket | Tok::Comma => Err(CalcError::Missing),
            Tok::Op(_) => Err(CalcError::Missing),
        }
    }

    /// Ce que la mémoire contient à l'adresse entre crochets.
    ///
    /// Le crochet ouvrant est déjà consommé. L'adresse est une
    /// expression entière : `[rbp - 8]`, `[rbx + rcx*4]`, tout ce que la
    /// calculatrice sait calculer.
    fn memory(&mut self, octets: usize) -> Result<u64, CalcError> {
        let adresse = self.expression(0)?;
        if self.peek() != Some(&Tok::CloseBracket) {
            return Err(CalcError::Unbalanced);
        }
        self.at += 1;
        let Some(mem) = self.mem else {
            return Err(CalcError::NoMemory);
        };
        mem.peek(adresse, octets)
            .ok_or(CalcError::Unreadable(adresse))
    }

    /// La valeur d'un registre nommé.
    fn register(&mut self, mot: &str) -> Result<u64, CalcError> {
        let bas = mot.to_ascii_lowercase();
        let Some(regs) = self.regs else {
            // Distinguer les deux cas compte : « il n'y a pas de
            // programme » et « ce mot n'existe pas » n'appellent pas la
            // même correction.
            return Err(if is_register_name(&bas) {
                CalcError::NoRegisters(bas)
            } else {
                CalcError::Unknown(bas)
            });
        };
        register_value(regs, &bas).ok_or(CalcError::Unknown(bas))
    }

    fn call(&mut self, nom: &str) -> Result<u64, CalcError> {
        let bas = nom.to_ascii_lowercase();
        let Some(&(nom_connu, arité, _, _)) = FUNCTIONS.iter().find(|(n, ..)| *n == bas) else {
            return Err(CalcError::Unknown(bas));
        };
        self.at += 1; // la parenthèse ouvrante
        let mut args = Vec::new();
        if self.peek() != Some(&Tok::Close) {
            loop {
                args.push(self.expression(0)?);
                match self.peek() {
                    Some(Tok::Comma) => self.at += 1,
                    _ => break,
                }
            }
        }
        if self.peek() != Some(&Tok::Close) {
            return Err(CalcError::Unbalanced);
        }
        self.at += 1;
        if args.len() != arité {
            return Err(CalcError::Arity {
                name: nom_connu,
                wanted: arité,
                got: args.len(),
            });
        }
        Ok(call_function(nom_connu, &args))
    }
}

/// Un opérateur binaire, appliqué.
fn apply(op: &str, a: u64, b: u64) -> Result<u64, CalcError> {
    Ok(match op {
        "+" => a.wrapping_add(b),
        "-" => a.wrapping_sub(b),
        "*" => a.wrapping_mul(b),
        "/" => {
            if b == 0 {
                return Err(CalcError::DivideByZero);
            }
            a / b
        }
        "%" => {
            if b == 0 {
                return Err(CalcError::DivideByZero);
            }
            a % b
        }
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        // Décaler de 64 ou plus rend zéro, et ne panique pas. Le
        // processeur, lui, ne regarde que les six bits de bas — `shl
        // rax, 64` ne fait donc rien du tout. On choisit ici la réponse
        // mathématique, qui est celle qu'on attend d'une calculatrice.
        "<<" => {
            if b >= 64 {
                0
            } else {
                a << b
            }
        }
        ">>" => {
            if b >= 64 {
                0
            } else {
                a >> b
            }
        }
        _ => return Err(CalcError::Unknown(op.to_owned())),
    })
}

/// Une fonction, appliquée. L'arité a déjà été vérifiée.
fn call_function(nom: &str, args: &[u64]) -> u64 {
    let a = args.first().copied().unwrap_or(0);
    let b = args.get(1).copied().unwrap_or(0);
    match nom {
        "bswap16" => u64::from((a as u16).swap_bytes()),
        "bswap32" => u64::from((a as u32).swap_bytes()),
        "bswap64" => a.swap_bytes(),
        "sext8" => a as u8 as i8 as i64 as u64,
        "sext16" => a as u16 as i16 as i64 as u64,
        "sext32" => a as u32 as i32 as i64 as u64,
        "lo8" => a & 0xff,
        "lo16" => a & 0xffff,
        "lo32" => a & 0xffff_ffff,
        "hi32" => a >> 32,
        "popcnt" => u64::from(a.count_ones()),
        "clz" => u64::from(a.leading_zeros()),
        "ctz" => u64::from(a.trailing_zeros()),
        "rol" => a.rotate_left((b % 64) as u32),
        "ror" => a.rotate_right((b % 64) as u32),
        // Aligner sur zéro ne veut rien dire : on rend la valeur telle
        // quelle plutôt que de diviser par zéro.
        "align" => {
            if b == 0 {
                a
            } else {
                a.div_ceil(b).wrapping_mul(b)
            }
        }
        "min" => a.min(b),
        "max" => a.max(b),
        _ => 0,
    }
}

/// Ce nom désigne-t-il un registre ?
fn is_register_name(nom: &str) -> bool {
    matches!(nom, "rip") || REGISTER_NAMES.contains(&nom)
}

/// Les seize généraux, plus `rip`. Les noms courts — `eax`, `ax`, `al` —
/// sont acceptés aussi : c'est ainsi qu'ils sont écrits dans le code
/// qu'on lit.
const REGISTER_NAMES: &[&str] = &[
    "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15", "eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp", "ax", "bx", "cx", "dx",
    "si", "di", "al", "bl", "cl", "dl", "ah", "bh", "ch", "dh",
];

/// La valeur d'un registre, nom court compris.
fn register_value(regs: &Registers, nom: &str) -> Option<u64> {
    use namsdec_core::Reg64::*;
    if nom == "rip" {
        return Some(regs.rip);
    }
    // Le registre large qui porte ce nom, et le masque qui en garde la
    // part nommée. `al` est l'octet de bas de RAX ; `ah` est le second,
    // ce qui surprend toujours et mérite d'être juste.
    let (large, masque, décalage) = match nom {
        "rax" | "eax" | "ax" | "al" | "ah" => (Rax, width_of(nom), shift_of(nom)),
        "rbx" | "ebx" | "bx" | "bl" | "bh" => (Rbx, width_of(nom), shift_of(nom)),
        "rcx" | "ecx" | "cx" | "cl" | "ch" => (Rcx, width_of(nom), shift_of(nom)),
        "rdx" | "edx" | "dx" | "dl" | "dh" => (Rdx, width_of(nom), shift_of(nom)),
        "rsi" | "esi" | "si" => (Rsi, width_of(nom), 0),
        "rdi" | "edi" | "di" => (Rdi, width_of(nom), 0),
        "rbp" | "ebp" => (Rbp, width_of(nom), 0),
        "rsp" | "esp" => (Rsp, width_of(nom), 0),
        "r8" => (R8, u64::MAX, 0),
        "r9" => (R9, u64::MAX, 0),
        "r10" => (R10, u64::MAX, 0),
        "r11" => (R11, u64::MAX, 0),
        "r12" => (R12, u64::MAX, 0),
        "r13" => (R13, u64::MAX, 0),
        "r14" => (R14, u64::MAX, 0),
        "r15" => (R15, u64::MAX, 0),
        _ => return None,
    };
    Some((regs.get64(large) >> décalage) & masque)
}

fn width_of(nom: &str) -> u64 {
    match nom.len() {
        2 if nom.ends_with('l') || nom.ends_with('h') => 0xff,
        2 => 0xffff,
        3 if nom.starts_with('e') => 0xffff_ffff,
        _ => u64::MAX,
    }
}

fn shift_of(nom: &str) -> u32 {
    if nom.len() == 2 && nom.ends_with('h') {
        8
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(expr: &str) -> u64 {
        eval(expr, None).unwrap_or_else(|e| panic!("« {expr} » : {e:?}"))
    }

    /// Une mémoire d'essai : une tranche d'octets posée à une adresse.
    struct Tranche {
        base: u64,
        octets: Vec<u8>,
    }

    impl Memory for Tranche {
        fn peek(&self, addr: u64, taille: usize) -> Option<u64> {
            let début = addr.checked_sub(self.base)? as usize;
            let morceau = self.octets.get(début..début + taille)?;
            Some(
                morceau
                    .iter()
                    .enumerate()
                    .fold(0u64, |acc, (rang, o)| acc | u64::from(*o) << (8 * rang)),
            )
        }
    }

    /// La mémoire se lit entre crochets, et la taille se dit comme dans
    /// NASM.
    ///
    /// C'est ce qui manquait pour écrire une condition d'arrêt utile :
    /// « arrête-toi quand ce qui est sur la pile n'est plus nul » ne
    /// s'exprimait pas.
    #[test]
    fn memory_reads_between_brackets() {
        // 0x41 0x42 0x43 0x44 0x00 0x00 0x00 0x00 à 0x1000.
        let mem = Tranche {
            base: 0x1000,
            octets: vec![0x41, 0x42, 0x43, 0x44, 0, 0, 0, 0, 0xff],
        };
        let lis = |expr: &str| eval_with(expr, None, Some(&mem as &dyn Memory));

        // Sans taille dite : huit octets, petit-boutiste.
        assert_eq!(lis("[0x1000]"), Ok(0x4443_4241));
        // Les quatre tailles de NASM.
        assert_eq!(lis("byte [0x1000]"), Ok(0x41));
        assert_eq!(lis("word [0x1000]"), Ok(0x4241));
        assert_eq!(lis("dword [0x1000]"), Ok(0x4443_4241));
        assert_eq!(lis("qword [0x1000]"), Ok(0x4443_4241));
        // L'adresse est une expression entière, comme dans un listing.
        assert_eq!(lis("byte [0x1000 + 1]"), Ok(0x42));
        assert_eq!(lis("byte [0x0ff8 + 2*4]"), Ok(0x41));
        // Et la lecture se combine au reste.
        assert_eq!(lis("(byte [0x1000] & 0x0f) + 1"), Ok(2));

        // Hors des pages : dit, et non zéro.
        assert_eq!(lis("[0x2000]"), Err(CalcError::Unreadable(0x2000)));
        // Sans mémoire du tout : dit aussi.
        assert_eq!(eval("[0x1000]", None), Err(CalcError::NoMemory));
        // Un crochet jamais refermé reste une faute.
        assert_eq!(lis("[0x1000"), Err(CalcError::Unbalanced));
    }

    /// Un jeu de valeurs qui couvre les cas où l'arithmétique trébuche.
    ///
    /// Zéro, un, les puissances de deux, les bords de chaque largeur, et
    /// quelques valeurs quelconques. C'est là que se cachent les
    /// débordements, les décalages de soixante-quatre et les signes mal
    /// étendus — jamais au milieu.
    const ÉPREUVES: [u64; 16] = [
        0,
        1,
        2,
        7,
        0xff,
        0x100,
        0xffff,
        0x1_0000,
        0xffff_ffff,
        0x1_0000_0000,
        0x8000_0000_0000_0000,
        0xffff_ffff_ffff_fffe,
        u64::MAX,
        0x1234_5678_9abc_def0,
        0xdead_beef,
        0x0f0f_0f0f_0f0f_0f0f,
    ];

    /// Chaque opérateur rend ce que Rust rendrait.
    ///
    /// Non pas une poignée d'exemples choisis, mais tout le produit
    /// croisé des valeurs limites : seize par seize, pour chacun des dix
    /// opérateurs. Une calculatrice de rétro-ingénierie qui se trompe
    /// d'un bit fait chercher un mot de passe qui n'existe pas, et l'on
    /// accuse le binaire.
    #[test]
    fn every_operator_gives_what_the_machine_gives() {
        for a in ÉPREUVES {
            for b in ÉPREUVES {
                let attendu: [(&str, Option<u64>); 10] = [
                    ("+", Some(a.wrapping_add(b))),
                    ("-", Some(a.wrapping_sub(b))),
                    ("*", Some(a.wrapping_mul(b))),
                    ("/", (b != 0).then(|| a / b)),
                    ("%", (b != 0).then(|| a % b)),
                    ("&", Some(a & b)),
                    ("|", Some(a | b)),
                    ("^", Some(a ^ b)),
                    // Décaler de soixante-quatre ou plus rend zéro. Le
                    // processeur, lui, ne regarde que les six bits de
                    // bas — `shl rax, 64` ne fait rien du tout — mais
                    // une calculatrice doit la réponse mathématique.
                    ("<<", Some(if b >= 64 { 0 } else { a << b })),
                    (">>", Some(if b >= 64 { 0 } else { a >> b })),
                ];
                for (op, veut) in attendu {
                    let écrit = format!("{a:#x} {op} {b:#x}");
                    match veut {
                        Some(valeur) => assert_eq!(
                            eval(&écrit, None),
                            Ok(valeur),
                            "{écrit} doit valoir {valeur:#x}"
                        ),
                        None => assert_eq!(
                            eval(&écrit, None),
                            Err(CalcError::DivideByZero),
                            "{écrit} divise par zéro"
                        ),
                    }
                }
                // Les unaires, sur la même série.
                assert_eq!(eval(&format!("~{a:#x}"), None), Ok(!a), "~{a:#x}");
                assert_eq!(
                    eval(&format!("-{a:#x}"), None),
                    Ok(0u64.wrapping_sub(a)),
                    "-{a:#x}"
                );
            }
        }
    }

    /// Les dix-huit fonctions rendent ce que leur nom promet.
    ///
    /// Comparées à l'opération de Rust qu'elles nomment, sur la même
    /// série de valeurs limites : `popcnt` compte les bits à un,
    /// `bswap32` permute quatre octets et pas huit, `sext8` étend le
    /// signe d'un octet — et l'on veut que ce soit vrai pour
    /// `0x8000_0000_0000_0000` comme pour 7.
    #[test]
    fn every_function_does_what_its_name_promises() {
        for a in ÉPREUVES {
            let un = |nom: &str| eval(&format!("{nom}({a:#x})"), None);
            assert_eq!(un("bswap16"), Ok(u64::from((a as u16).swap_bytes())));
            assert_eq!(un("bswap32"), Ok(u64::from((a as u32).swap_bytes())));
            assert_eq!(un("bswap64"), Ok(a.swap_bytes()));
            assert_eq!(un("sext8"), Ok(a as u8 as i8 as i64 as u64));
            assert_eq!(un("sext16"), Ok(a as u16 as i16 as i64 as u64));
            assert_eq!(un("sext32"), Ok(a as u32 as i32 as i64 as u64));
            assert_eq!(un("lo8"), Ok(a & 0xff));
            assert_eq!(un("lo16"), Ok(a & 0xffff));
            assert_eq!(un("lo32"), Ok(a & 0xffff_ffff));
            assert_eq!(un("hi32"), Ok(a >> 32));
            assert_eq!(un("popcnt"), Ok(u64::from(a.count_ones())));
            assert_eq!(un("clz"), Ok(u64::from(a.leading_zeros())));
            assert_eq!(un("ctz"), Ok(u64::from(a.trailing_zeros())));

            for b in ÉPREUVES {
                let deux = |nom: &str| eval(&format!("{nom}({a:#x}, {b:#x})"), None);
                assert_eq!(deux("rol"), Ok(a.rotate_left((b % 64) as u32)));
                assert_eq!(deux("ror"), Ok(a.rotate_right((b % 64) as u32)));
                assert_eq!(deux("min"), Ok(a.min(b)));
                assert_eq!(deux("max"), Ok(a.max(b)));
                // Aligner sur zéro ne veut rien dire : on rend la valeur
                // telle quelle plutôt que de diviser par zéro.
                let aligné = if b == 0 {
                    a
                } else {
                    a.div_ceil(b).wrapping_mul(b)
                };
                assert_eq!(deux("align"), Ok(aligné), "align({a:#x}, {b:#x})");
            }
        }
    }

    /// Les priorités sont bien celles du C, vérifiées par parenthèses.
    ///
    /// Une priorité inversée donne des résultats *plausibles* — c'est ce
    /// qui la rend dangereuse : `1 | 2 & 3` vaut 3 dans un sens et 2
    /// dans l'autre, et rien à l'écran ne dit lequel on a obtenu. On
    /// compare donc chaque expression à sa forme entièrement
    /// parenthésée.
    #[test]
    fn precedence_matches_the_fully_parenthesised_form() {
        const CAS: [(&str, &str); 14] = [
            ("1 + 2 * 3", "1 + (2 * 3)"),
            ("2 * 3 + 1", "(2 * 3) + 1"),
            ("8 / 4 / 2", "(8 / 4) / 2"),
            ("10 - 3 - 2", "(10 - 3) - 2"),
            ("1 << 2 + 3", "1 << (2 + 3)"),
            ("0xff >> 2 * 2", "0xff >> (2 * 2)"),
            ("1 | 2 & 3", "1 | (2 & 3)"),
            ("1 ^ 2 & 3", "1 ^ (2 & 3)"),
            ("1 | 2 ^ 3", "1 | (2 ^ 3)"),
            ("0xf0 & 0x30 | 0x0f", "(0xf0 & 0x30) | 0x0f"),
            ("~1 + 1", "(~1) + 1"),
            ("-2 * 3", "(-2) * 3"),
            ("1 + 2 << 1", "(1 + 2) << 1"),
            ("7 % 4 * 2", "(7 % 4) * 2"),
        ];
        for (nu, parenthésé) in CAS {
            assert_eq!(
                eval(nu, None),
                eval(parenthésé, None),
                "« {nu} » doit se lire « {parenthésé} »"
            );
        }
        // Et les priorités sont bien celles du C, non l'ordre d'écriture.
        assert_eq!(eval("1 + 2 * 3", None), Ok(7), "et non 9");
        assert_eq!(eval("1 | 2 & 3", None), Ok(3), "et non 0");
        assert_eq!(eval("1 << 2 + 3", None), Ok(32), "et non 7");
    }

    /// Mal écrit, ou seulement incalculable ici ?
    ///
    /// La distinction décide si un champ de condition doit devenir
    /// rouge. `[rsp+8] != 0` s'écrit avant de lancer le programme — et
    /// c'est même le seul moment où l'on écrit une condition d'arrêt.
    #[test]
    fn a_typo_and_a_missing_program_are_not_the_same_thing() {
        for expr in ["totomatic", "1 +", "(1", "1 2", "min(1)", "@"] {
            let e = eval(expr, None).unwrap_err();
            assert!(e.is_syntax(), "« {expr} » est mal écrit : {e:?}");
        }
        for expr in ["rax", "[0x10]", "byte [rsp]", "1 / 0"] {
            let e = eval(expr, None).unwrap_err();
            assert!(
                !e.is_syntax(),
                "« {expr} » s'écrit bien, il manque seulement un programme : {e:?}"
            );
        }
    }

    /// Les quatre bases se lisent, et se mélangent dans une même ligne.
    ///
    /// C'est le premier service rendu : on recopie `0x1f` d'un
    /// désassemblage à côté d'un 31 décimal sans convertir quoi que ce
    /// soit.
    #[test]
    fn every_base_reads() {
        assert_eq!(ok("0x1f"), 31);
        assert_eq!(ok("0b1_1111"), 31);
        assert_eq!(ok("0o37"), 31);
        assert_eq!(ok("31"), 31);
        // Le suffixe de NASM, qu'on recopie sans y penser.
        assert_eq!(ok("1Fh"), 31);
        // Et un caractère est son code.
        assert_eq!(ok("'A'"), 65);
        assert_eq!(ok("'\\n'"), 10);
        // Mélangées.
        assert_eq!(ok("0x10 + 0b1 + 0o1 + 1"), 19);
    }

    /// Les opérations qu'on fait en lisant du code.
    #[test]
    fn the_operations_one_actually_needs() {
        assert_eq!(ok("0xff & 0x0f"), 0x0f);
        assert_eq!(ok("0xf0 | 0x0f"), 0xff);
        assert_eq!(ok("0xff ^ 0x0f"), 0xf0);
        assert_eq!(ok("~0"), u64::MAX);
        assert_eq!(ok("1 << 8"), 256);
        assert_eq!(ok("0x100 >> 4"), 0x10);
        assert_eq!(ok("7 % 3"), 1);
    }

    /// La priorité est celle du C, parce que c'est celle qu'on a en tête.
    ///
    /// Se tromper ici donnerait des résultats faux d'apparence juste, ce
    /// qui est la pire sorte de faux dans un outil de calcul.
    #[test]
    fn precedence_is_the_one_from_c() {
        assert_eq!(ok("1 + 2 * 3"), 7);
        assert_eq!(ok("(1 + 2) * 3"), 9);
        // `&` lie plus fort que `|`, et les décalages plus fort que `&`.
        assert_eq!(ok("1 | 2 & 0"), 1);
        assert_eq!(ok("0xff & 1 << 4"), 0x10);
        // À gauche : `1 - 2 - 3` vaut -4, non 2.
        assert_eq!(ok("1 - 2 - 3"), 0u64.wrapping_sub(4));
    }

    /// Tout déborde en silence, comme dans un registre.
    #[test]
    fn arithmetic_wraps_like_a_register() {
        assert_eq!(ok("0 - 1"), u64::MAX);
        assert_eq!(ok("-1"), u64::MAX);
        // Décaler au-delà de la largeur rend zéro, sans paniquer.
        assert_eq!(ok("1 << 64"), 0);
        assert_eq!(ok("1 << 200"), 0);
    }

    /// Les fonctions de rétro-ingénierie font ce qu'elles annoncent.
    #[test]
    fn the_reverse_engineering_functions_work() {
        // Un mot lu à l'envers : l'erreur de boutisme la plus courante.
        assert_eq!(ok("bswap32(0x12345678)"), 0x78563412);
        assert_eq!(ok("bswap16(0x1234)"), 0x3412);
        // Une comparaison signée sur un octet : 0xff est -1.
        assert_eq!(ok("sext8(0xff)"), u64::MAX);
        assert_eq!(ok("sext8(0x7f)"), 0x7f);
        assert_eq!(ok("sext32(0xffffffff)"), u64::MAX);
        assert_eq!(ok("lo8(0x1234)"), 0x34);
        assert_eq!(ok("hi32(0x1122334455667788)"), 0x11223344);
        assert_eq!(ok("popcnt(0xff)"), 8);
        assert_eq!(ok("ctz(0x100)"), 8);
        assert_eq!(ok("clz(1)"), 63);
        assert_eq!(ok("rol(0x8000000000000000, 1)"), 1);
        assert_eq!(ok("ror(1, 1)"), 0x8000000000000000);
        // L'alignement d'une pile ou d'une page.
        assert_eq!(ok("align(0x1001, 0x1000)"), 0x2000);
        assert_eq!(ok("align(0x1000, 0x1000)"), 0x1000);
        assert_eq!(ok("min(3, 5) + max(3, 5)"), 8);
    }

    /// Et l'on peut les emboîter, ce qui est tout l'intérêt.
    #[test]
    fn functions_nest() {
        assert_eq!(ok("popcnt(0xff & 0x0f)"), 4);
        assert_eq!(ok("bswap32(bswap32(0x12345678))"), 0x12345678);
        assert_eq!(ok("align(popcnt(0xff) + 1, 4)"), 12);
    }

    /// Chaque erreur se nomme, parce que « erreur » n'apprend rien.
    #[test]
    fn every_mistake_says_which_one_it_is() {
        assert_eq!(eval("", None), Err(CalcError::Empty));
        assert_eq!(eval("   ", None), Err(CalcError::Empty));
        assert_eq!(eval("1 +", None), Err(CalcError::Missing));
        assert_eq!(eval("(1", None), Err(CalcError::Unbalanced));
        assert_eq!(eval("1 / 0", None), Err(CalcError::DivideByZero));
        assert_eq!(eval("1 % 0", None), Err(CalcError::DivideByZero));
        assert!(matches!(eval("1 2", None), Err(CalcError::Trailing(_))));
        assert!(matches!(eval("zork", None), Err(CalcError::Unknown(_))));
        assert!(matches!(eval("1 $ 2", None), Err(CalcError::BadChar('$'))));
        assert_eq!(
            eval("rol(1)", None),
            Err(CalcError::Arity {
                name: "rol",
                wanted: 2,
                got: 1
            })
        );
        // Un registre sans programme : ce n'est pas un mot inconnu, et
        // la correction n'est pas la même.
        assert!(matches!(eval("rax", None), Err(CalcError::NoRegisters(_))));
    }

    /// Les registres se nomment, à toutes les largeurs.
    #[test]
    fn registers_read_at_every_width() {
        let mut regs = Registers::default();
        regs.set64(namsdec_core::Reg64::Rax, 0x1122334455667788);
        regs.rip = 0x401000;

        assert_eq!(eval("rax", Some(&regs)), Ok(0x1122334455667788));
        assert_eq!(eval("eax", Some(&regs)), Ok(0x55667788));
        assert_eq!(eval("ax", Some(&regs)), Ok(0x7788));
        assert_eq!(eval("al", Some(&regs)), Ok(0x88));
        // `ah` est le *second* octet, ce qui surprend toujours.
        assert_eq!(eval("ah", Some(&regs)), Ok(0x77));
        assert_eq!(eval("rip", Some(&regs)), Ok(0x401000));
        // Et dans un calcul, ce qui est le but.
        assert_eq!(eval("al & 0x0f", Some(&regs)), Ok(8));
        assert_eq!(eval("RAX", Some(&regs)), Ok(0x1122334455667788), "la casse ne compte pas");
    }

    /// Aucune expression, si tordue soit-elle, ne fait paniquer.
    ///
    /// Une calculatrice reçoit ce qu'on tape *pendant* qu'on le tape :
    /// chaque état intermédiaire d'une frappe lui passe sous les yeux, y
    /// compris les plus absurdes.
    #[test]
    fn nothing_ever_panics() {
        let morceaux = [
            "", "(", ")", "1", "+", "-", "~", "*", "0x", "0b", "'", "'a", ",", "rax", "rol", "<<",
            "%", "/", "é", "0o9", "999999999999999999999999",
        ];
        for a in morceaux {
            let _ = eval(a, None);
            for b in morceaux {
                let _ = eval(&format!("{a}{b}"), None);
                for c in morceaux {
                    let _ = eval(&format!("{a}{b}{c}"), None);
                }
            }
        }
    }
}
