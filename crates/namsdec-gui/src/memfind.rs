//! Chercher une suite d'octets dans la mémoire.
//!
//! C'est le premier réflexe devant un programme qui compare quelque
//! chose. On a lu `"Correct Password"` dans les chaînes, on veut savoir
//! *où* cette adresse est écrite ; on a vu passer `0x48555648` dans un
//! registre, on veut savoir si ces quatre octets sont aussi quelque part
//! en mémoire ; on vient d'entrer `hunter2` au clavier et l'on cherche
//! dans quel tampon il a atterri.
//!
//! **Trois écritures, dans la même case.** Un texte entre guillemets,
//! des octets en hexadécimal, et `??` pour « n'importe lequel ». Elles
//! se mélangent : `"pass" 00` cherche le mot suivi de son zéro final,
//! `48 89 ?? 45` cherche une instruction dont un octet varie. C'est la
//! notation des outils de rétro-ingénierie, et celle qu'on recopie d'un
//! article sans la retraduire.
//!
//! Ce fichier ne connaît ni egui ni l'émulateur : il prend des octets et
//! rend des positions, ce qui se teste au caractère près.

/// Pourquoi un motif n'a pas pu être lu.
///
/// Chaque cas se dit en clair : un champ qui répond « motif invalide »
/// laisse chercher ce qui ne va pas dans dix caractères.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// Rien à chercher.
    Empty,
    /// Un texte ouvert et jamais refermé.
    Unterminated,
    /// Un caractère qui n'est ni un chiffre hexadécimal ni un `?`.
    BadHex(char),
    /// Un octet à demi écrit : `4` au lieu de `41`. Un octet fait deux
    /// chiffres, toujours, et deviner le second reviendrait à chercher
    /// autre chose que ce qui est demandé.
    HalfByte(String),
}

/// Un motif prêt à chercher : une suite d'octets dont certains sont
/// libres.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pattern {
    /// `None` est un joker : cette position-là compte, mais sa valeur ne
    /// se regarde pas.
    octets: Vec<Option<u8>>,
}

impl Pattern {
    /// Combien d'octets le motif occupe, jokers compris.
    pub fn len(&self) -> usize {
        self.octets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.octets.is_empty()
    }

    /// Les adresses où le motif se trouve, au plus `max`.
    ///
    /// `base` est l'adresse du premier octet de `foin` : la recherche
    /// travaille sur des positions, l'appelant veut des adresses, et
    /// c'est ici que les deux se rejoignent — plutôt que dans chaque
    /// appelant, qui finirait par oublier l'addition une fois sur deux.
    ///
    /// Borné volontairement : un motif d'un seul octet dans un mégaoctet
    /// de mémoire trouve des milliers de réponses, et une liste de
    /// milliers de lignes n'apprend rien. Ce qu'on veut savoir, c'est
    /// *où commencer à regarder*.
    pub fn find_all(&self, foin: &[u8], base: u64, max: usize) -> Vec<u64> {
        let mut out = Vec::new();
        if self.is_empty() || foin.len() < self.len() || max == 0 {
            return out;
        }
        let dernier = foin.len() - self.octets.len();
        // Le premier octet fixe sert de crible : sauter directement aux
        // positions où il apparaît évite de comparer le motif entier à
        // chaque offset. Sur un motif qui commence par un joker, il n'y
        // a pas de crible et l'on compare partout — c'est le prix du
        // joker en tête, et il est rare.
        let crible = self.octets[0];
        for i in 0..=dernier {
            if let Some(attendu) = crible
                && foin[i] != attendu
            {
                continue;
            }
            if self
                .octets
                .iter()
                .zip(&foin[i..])
                .all(|(motif, octet)| motif.is_none_or(|m| m == *octet))
            {
                out.push(base + i as u64);
                if out.len() == max {
                    return out;
                }
            }
        }
        out
    }
}

/// Lit un motif.
///
/// Les morceaux se suivent, séparés par des blancs ou collés : un texte
/// entre guillemets, ou des chiffres hexadécimaux par paires, `??` valant
/// « n'importe quel octet ».
pub fn parse(texte: &str) -> Result<Pattern, PatternError> {
    let chars: Vec<char> = texte.chars().collect();
    let mut octets = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            c if c.is_whitespace() => i += 1,
            '"' | '\'' => {
                let (lus, avance) = lire_texte(&chars, i)?;
                octets.extend(lus.into_iter().map(Some));
                i += avance;
            }
            _ => {
                let (lus, avance) = lire_hexa(&chars, i)?;
                octets.extend(lus);
                i += avance;
            }
        }
    }
    if octets.is_empty() {
        return Err(PatternError::Empty);
    }
    Ok(Pattern { octets })
}

/// Un texte entre guillemets, avec les échappements qu'on écrit sans y
/// penser.
fn lire_texte(chars: &[char], début: usize) -> Result<(Vec<u8>, usize), PatternError> {
    let ouvrant = chars[début];
    let mut out = String::new();
    let mut i = début + 1;
    while i < chars.len() {
        match chars[i] {
            c if c == ouvrant => {
                return Ok((out.into_bytes(), i + 1 - début));
            }
            '\\' if i + 1 < chars.len() => {
                out.push(match chars[i + 1] {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    autre => autre,
                });
                i += 2;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    Err(PatternError::Unterminated)
}

/// Une suite de chiffres hexadécimaux, par paires, avec `??` pour joker.
fn lire_hexa(chars: &[char], début: usize) -> Result<(Vec<Option<u8>>, usize), PatternError> {
    let mut fin = début;
    while fin < chars.len() && !chars[fin].is_whitespace() && chars[fin] != '"' && chars[fin] != '\''
    {
        fin += 1;
    }
    let mot: String = chars[début..fin].iter().collect();

    // `?` tout seul vaut un joker : c'est l'écriture la plus courte, et
    // celle qu'on tape en premier.
    if mot == "?" {
        return Ok((vec![None], fin - début));
    }
    for c in mot.chars() {
        if !c.is_ascii_hexdigit() && c != '?' {
            return Err(PatternError::BadHex(c));
        }
    }
    if !mot.len().is_multiple_of(2) {
        return Err(PatternError::HalfByte(mot));
    }
    let paires: Vec<char> = mot.chars().collect();
    let mut out = Vec::with_capacity(paires.len() / 2);
    for paire in paires.chunks(2) {
        match (paire[0], paire[1]) {
            ('?', '?') => out.push(None),
            // Un demi-joker — `?1`, `4?` — désignerait un quartet, ce qui
            // est une autre idée : on la refuse plutôt que d'en inventer
            // une lecture.
            ('?', autre) | (autre, '?') => return Err(PatternError::BadHex(autre)),
            (a, b) => {
                let haut = a.to_digit(16).ok_or(PatternError::BadHex(a))?;
                let bas = b.to_digit(16).ok_or(PatternError::BadHex(b))?;
                out.push(Some((haut * 16 + bas) as u8));
            }
        }
    }
    Ok((out, fin - début))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motif(texte: &str) -> Pattern {
        parse(texte).unwrap_or_else(|e| panic!("« {texte} » : {e:?}"))
    }

    /// Les trois écritures, et leur mélange.
    #[test]
    fn the_three_ways_of_writing_a_pattern() {
        // De l'hexadécimal, collé ou espacé : c'est le même motif.
        assert_eq!(motif("41424344"), motif("41 42 43 44"));
        assert_eq!(motif("41424344").len(), 4);

        // Un texte, avec ses échappements.
        assert_eq!(motif("\"ABCD\""), motif("41424344"));
        assert_eq!(motif("\"a\\nb\"").len(), 3);
        // Les apostrophes valent les guillemets : on recopie de l'une ou
        // l'autre sorte selon d'où l'on vient.
        assert_eq!(motif("'AB'"), motif("4142"));

        // Les jokers.
        assert_eq!(motif("48 89 ?? 45").len(), 4);
        assert_eq!(motif("48 ? 45").len(), 3);

        // Et tout ensemble : le mot, puis son zéro final.
        assert_eq!(motif("\"pass\" 00").len(), 5);
    }

    /// Ce qui est mal écrit est refusé, et dit lequel.
    #[test]
    fn what_is_badly_written_says_so() {
        assert_eq!(parse(""), Err(PatternError::Empty));
        assert_eq!(parse("   "), Err(PatternError::Empty));
        assert_eq!(parse("\"ouvert"), Err(PatternError::Unterminated));
        assert_eq!(parse("4z"), Err(PatternError::BadHex('z')));
        // Un octet fait deux chiffres, toujours.
        assert_eq!(parse("414"), Err(PatternError::HalfByte("414".to_owned())));
        // Un demi-joker désignerait un quartet, ce qui est une autre
        // idée : on la refuse plutôt que d'en inventer une lecture.
        assert_eq!(parse("?1"), Err(PatternError::BadHex('1')));
    }

    /// Chercher, et trouver là où il faut.
    #[test]
    fn a_pattern_is_found_where_it_is() {
        let foin = b"il y a un mot de passe: hunter2, et un autre: hunter7".to_vec();
        let base = 0x4000;

        let trouvés = motif("\"hunter2\"").find_all(&foin, base, 16);
        assert_eq!(trouvés.len(), 1);
        let position = (trouvés[0] - base) as usize;
        assert_eq!(&foin[position..position + 7], b"hunter2");

        // Le joker rend les deux.
        let trouvés = motif("\"hunter\" ??").find_all(&foin, base, 16);
        assert_eq!(trouvés.len(), 2, "hunter2 et hunter7");

        // Ce qui n'y est pas ne se trouve pas.
        assert!(motif("\"hunter9\"").find_all(&foin, base, 16).is_empty());
    }

    /// Le compte est borné, et la borne est respectée.
    ///
    /// Un motif d'un seul octet dans un mégaoctet trouve des milliers de
    /// réponses. Une liste de milliers de lignes n'apprend rien : ce
    /// qu'on veut savoir, c'est où commencer à regarder.
    #[test]
    fn the_count_is_bounded() {
        let foin = vec![0x41u8; 1000];
        assert_eq!(motif("41").find_all(&foin, 0, 5).len(), 5);
        assert_eq!(motif("41").find_all(&foin, 0, 0).len(), 0);
        // Et le motif plus long que la mémoire ne trouve rien, sans
        // déborder.
        assert!(motif("41").find_all(&[], 0, 5).is_empty());
        assert!(motif("4141").find_all(&[0x41], 0, 5).is_empty());
    }

    /// Un joker en tête ne fait pas trébucher la recherche.
    ///
    /// Le premier octet sert de crible ; quand il est libre, il n'y en a
    /// pas, et l'on compare partout. C'est plus lent, et ce doit être
    /// juste quand même.
    #[test]
    fn a_wildcard_may_come_first() {
        let foin = b"xAyAzB".to_vec();
        let trouvés = motif("?? 41").find_all(&foin, 0, 16);
        assert_eq!(trouvés, vec![0, 2], "avant chaque A");
    }

    /// Les adresses rendues sont celles de la mémoire, non des
    /// positions dans un tableau.
    #[test]
    fn the_addresses_are_real_ones() {
        let foin = b"..ABC".to_vec();
        assert_eq!(motif("\"ABC\"").find_all(&foin, 0x40_1000, 4), vec![0x40_1002]);
    }
}
