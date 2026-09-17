//! Le motif cyclique : retrouver *où* un débordement a écrit.
//!
//! Quand un programme écrit au-delà de son tampon et que le retour part
//! dans le décor, la question n'est pas « est-ce que ça déborde ? » —
//! on le voit bien — mais « de combien ». On remplit donc l'entrée avec
//! une suite dont **toute tranche de quatre octets est unique** : la
//! valeur qu'on retrouve dans RIP dit alors, à elle seule, à quelle
//! distance du début du tampon se trouve l'adresse de retour.
//!
//! C'est la suite de De Bruijn, et c'est ce que `cyclic` fait dans
//! pwndbg. Le calcul est mécanique : autant que la machine le fasse.

/// L'alphabet, comme pwndbg et pwntools : les minuscules latines.
pub const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

/// La longueur d'une tranche unique. Quatre octets suffisent pour un
/// motif de 456 976 octets — largement plus qu'aucune pile
/// d'apprentissage.
pub const WINDOW: usize = 4;

/// Les `len` premiers octets du motif.
///
/// ```
/// let motif = namsdec_re::cyclic::pattern(16);
/// assert_eq!(motif, b"aaaabaaacaaadaaa");
/// ```
pub fn pattern(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    de_bruijn(ALPHABET, WINDOW, len, &mut out);
    out
}

/// À quelle distance du début du motif cette suite d'octets se trouve.
///
/// Rend `None` quand la suite n'y figure pas — le cas qui compte autant
/// que l'autre : cela veut dire que ce qu'on a retrouvé ne vient pas du
/// motif, et donc que l'hypothèse était fausse.
///
/// ```
/// // « aaba » commence au cinquième octet du motif.
/// assert_eq!(namsdec_re::cyclic::offset(b"baaa"), Some(4));
/// assert_eq!(namsdec_re::cyclic::offset(b"..."), None);
/// ```
pub fn offset(needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    // Le motif entier : 26^4 octets, un demi-méga-octet, calculé une
    // fois pour la recherche puis rendu.
    let mut whole = Vec::new();
    de_bruijn(
        ALPHABET,
        WINDOW,
        ALPHABET.len().pow(WINDOW as u32),
        &mut whole,
    );
    whole
        .windows(needle.len())
        .position(|window| window == needle)
}

/// La même question posée d'une valeur de registre.
///
/// Les octets sont pris en petit-boutiste — l'ordre dans lequel ils ont
/// été écrits en mémoire — et les zéros de tête sont retirés : une
/// adresse de retour écrasée par quatre caractères tient dans les quatre
/// octets bas, le reste est du remplissage.
pub fn offset_of_value(value: u64) -> Option<usize> {
    let bytes = value.to_le_bytes();
    let utiles = bytes.iter().rposition(|&b| b != 0).map(|i| i + 1)?;
    // Au moins quatre octets : moins que la fenêtre, et la réponse ne
    // serait plus unique.
    if utiles < WINDOW {
        return None;
    }
    offset(&bytes[..utiles])
}

/// La suite de De Bruijn B(k, n), tronquée à `len` octets.
///
/// L'algorithme est celui de Fredricksen et Maiorana : on énumère les
/// mots de Lyndon dans l'ordre, et leur concaténation est la suite.
/// C'est celui de pwntools, d'où des motifs identiques octet pour
/// octet.
fn de_bruijn(alphabet: &[u8], n: usize, len: usize, out: &mut Vec<u8>) {
    /// Une descente : `t` est la position courante dans le mot,
    /// `p` la longueur de sa plus petite période.
    fn descend(
        t: usize,
        p: usize,
        a: &mut [usize],
        alphabet: &[u8],
        n: usize,
        len: usize,
        out: &mut Vec<u8>,
    ) {
        if out.len() >= len {
            return;
        }
        if t > n {
            // Un mot de Lyndon : sa période divise n, et c'est lui qu'on
            // écrit. Leur concaténation, dans cet ordre, est la suite.
            if n.is_multiple_of(p) {
                for index in 1..=p {
                    if out.len() >= len {
                        return;
                    }
                    out.push(alphabet[a[index]]);
                }
            }
            return;
        }
        a[t] = a[t - p];
        descend(t + 1, p, a, alphabet, n, len, out);
        for j in (a[t - p] + 1)..alphabet.len() {
            a[t] = j;
            descend(t + 1, t, a, alphabet, n, len, out);
        }
    }

    // La profondeur de la récursion est bornée par n, pas par la
    // longueur demandée : quatre niveaux ici.
    let mut a = vec![0usize; alphabet.len() * n + 1];
    out.clear();
    descend(1, 1, &mut a, alphabet, n, len, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le motif est celui de pwntools : les mêmes octets, dans le même
    /// ordre. Un décalage trouvé ici vaut donc ailleurs — dans pwndbg,
    /// dans un script pwntools, dans un compte rendu.
    ///
    /// La référence a été relevée sur `pwnlib.util.cyclic.cyclic(64)`.
    #[test]
    fn the_pattern_is_the_one_everybody_else_generates() {
        assert_eq!(
            &pattern(64),
            b"aaaabaaacaaadaaaeaaafaaagaaahaaaiaaajaaakaaalaaamaaanaaaoaaapaaa"
        );
        // Et la réciproque : `cyclic_find(b"laaa")` rend 44.
        assert_eq!(offset(b"laaa"), Some(44));
    }

    /// Ce qui fait tout : toute tranche de quatre octets n'apparaît
    /// qu'une fois. Sans cela, un décalage serait ambigu.
    #[test]
    fn every_window_of_four_appears_once() {
        let motif = pattern(20_000);
        let mut vues = std::collections::HashSet::new();
        for window in motif.windows(WINDOW) {
            assert!(
                vues.insert(window.to_vec()),
                "« {} » apparaît deux fois",
                String::from_utf8_lossy(window)
            );
        }
        assert_eq!(vues.len(), motif.len() - WINDOW + 1);
    }

    /// Le décalage retrouvé est bien celui du motif, et la valeur d'un
    /// registre se pose de la même façon.
    #[test]
    fn an_offset_comes_back_from_what_was_found() {
        let motif = pattern(600);
        for at in [0usize, 4, 37, 128, 400] {
            let tranche = &motif[at..at + WINDOW];
            assert_eq!(offset(tranche), Some(at));
            // La même chose vue comme un registre : les quatre octets
            // lus en petit-boutiste.
            let mut mot = [0u8; 8];
            mot[..WINDOW].copy_from_slice(tranche);
            assert_eq!(offset_of_value(u64::from_le_bytes(mot)), Some(at));
        }
    }

    /// Ce qui ne vient pas du motif ne reçoit pas de décalage inventé.
    #[test]
    fn what_is_not_from_the_pattern_gets_no_answer() {
        assert_eq!(offset(b"ZZZZ"), None);
        assert_eq!(offset_of_value(0), None);
        // Trois octets ne suffisent pas à désigner un endroit unique.
        assert_eq!(offset_of_value(0x616161), None);
    }
}
