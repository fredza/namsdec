//! Les lectures d'une même valeur.
//!
//! Un registre affiche `000000000000002a`. C'est juste, et c'est
//! insuffisant : la même chose vaut 42 en décimal, `*` en ASCII, et
//! `101010` en binaire — et la question « combien ça fait, au juste ? »
//! est celle qui envoie chercher une calculatrice ailleurs. Autant la
//! poser ici.
//!
//! Aucun rendu : une chaîne, faite pour une infobulle, et testable sans
//! écran.

use crate::i18n::{Lang, strings};

/// Toutes les lectures de `value`, une par ligne.
///
/// Les lectures qui ne disent rien sont omises : le caractère d'une
/// valeur qui n'en est pas un, la moitié haute d'un nombre qui tient
/// dans sa moitié basse.
pub fn of(lang: Lang, value: u64) -> String {
    let s = strings(lang);
    let mut out = String::new();
    let mut line = |label: &str, text: String| {
        out.push_str(&format!("{label} {text}\n"));
    };

    line(s.reading_unsigned, value.to_string());
    // Le signé n'est dit que s'il diffère : répéter « 42 » sous « 42 »
    // ne fait que noyer la ligne qui compte.
    let signed = value as i64;
    if signed < 0 {
        line(s.reading_signed, signed.to_string());
    }
    line(s.reading_hex, format!("{value:#x}"));
    line(s.reading_binary, binary(value));

    // Le caractère : seulement s'il s'agit d'un octet imprimable. C'est
    // la lecture qu'on cherche quand on travaille une chaîne, et elle
    // n'a aucun sens ailleurs.
    if let Some(ch) = printable(value) {
        line(s.reading_char, format!("'{ch}'"));
    }
    // Les octets en mémoire, petit-boutiste : l'ordre dans lequel on les
    // retrouvera dans un vidage, qui n'est pas celui qu'on écrit.
    line(s.reading_bytes, little_endian(value));

    // Le flottant, seulement si les bits en forment un qu'on peut lire.
    // `2a` lu comme un double donne 2e-322, qui n'apprend rien ; un
    // nombre normal, lui, est souvent exactement ce qu'on cherchait.
    let float = f64::from_bits(value);
    if float.is_normal() && float.abs() >= 1e-6 && float.abs() < 1e17 {
        line(s.reading_float, format!("{float}"));
    }
    out.pop();
    out
}

/// Les deux lectures qu'on veut d'un coup d'oeil : le décimal, et le
/// signé quand il diffère.
///
/// L'infobulle du *nom* d'un registre parlait de son rôle et de sa
/// provenance, mais pas de ce qu'il contient — il fallait viser la
/// valeur, à côté, pour l'apprendre. Or « combien ça fait ? » est la
/// question qu'on se pose en survolant RCX, et non « à quoi sert RCX ».
pub fn short(lang: Lang, value: u64) -> String {
    let s = strings(lang);
    let signed = value as i64;
    if signed < 0 {
        format!(
            "{} {value}   {} {signed}",
            s.reading_unsigned, s.reading_signed
        )
    } else {
        format!("{} {value}", s.reading_unsigned)
    }
}

/// Toutes les bases d'une même valeur, étiquetées, dans l'ordre où on
/// les consulte.
///
/// Pour la calculatrice, qui les montre **toutes en même temps** — c'est
/// même tout son objet : on tape `0x1f` et l'on voit 31, `11111`, `37`,
/// sans rien convertir. `of` ci-dessus tait ce qui n'apprend rien, ce qui
/// est juste dans une infobulle ; ici, une ligne qui disparaît selon la
/// valeur ferait bouger l'affichage sous les doigts.
pub fn all_bases(lang: Lang, value: u64) -> Vec<(&'static str, String)> {
    let s = strings(lang);
    vec![
        (s.reading_hex, format!("{value:#x}")),
        (s.reading_unsigned, value.to_string()),
        (s.reading_signed, (value as i64).to_string()),
        (s.reading_octal, format!("{value:#o}")),
        (s.reading_binary, binary(value)),
        (s.reading_ascii, ascii(value)),
        (s.reading_bytes, little_endian(value)),
    ]
}

/// Les huit octets lus comme des caractères, **dans l'ordre où ils sont
/// en mémoire** — le moins significatif d'abord.
///
/// C'est le point de toute la ligne. `0x6c6c6568` écrit à l'endroit se
/// lit « lleh », ce qui n'apprend rien ; les mêmes octets rangés comme le
/// processeur les range donnent « hell », et l'on a reconnu la chaîne.
/// Une valeur devient une chaîne en passant par la mémoire, et c'est donc
/// la mémoire qu'il faut montrer — la même que la ligne d'en dessous.
///
/// Ce qui n'est pas imprimable devient un point, comme dans tout vidage
/// hexadécimal depuis quarante ans.
fn ascii(value: u64) -> String {
    value
        .to_le_bytes()
        .iter()
        .map(|b| {
            if b.is_ascii_graphic() || *b == b' ' {
                *b as char
            } else {
                '.'
            }
        })
        .collect()
}

/// La valeur en binaire, par groupes de huit — sans les octets de tête
/// nuls, qui ne font que rallonger.
fn binary(value: u64) -> String {
    let bytes = value.to_be_bytes();
    let first = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[first..]
        .iter()
        .map(|b| format!("{b:08b}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Les octets tels qu'ils sont rangés en mémoire : le moins significatif
/// d'abord.
fn little_endian(value: u64) -> String {
    value
        .to_le_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Le caractère ASCII imprimable que porte la valeur, si elle n'est que
/// cela.
fn printable(value: u64) -> Option<char> {
    if value > 0x7e {
        return None;
    }
    let byte = value as u8;
    (byte.is_ascii_graphic() || byte == b' ').then_some(byte as char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_value_reads_every_way_that_makes_sense() {
        let text = of(Lang::Fr, 42);
        assert!(text.contains("42"), "{text}");
        assert!(text.contains("0x2a"), "{text}");
        assert!(text.contains("00101010"), "{text}");
        assert!(text.contains("'*'"), "42, c'est l'astérisque : {text}");
        assert!(
            text.contains("2a 00 00 00 00 00 00 00"),
            "et voilà comment on le retrouve en mémoire : {text}"
        );
        // 42 lu comme un double vaut 2e-322 : une lecture qui n'apprend
        // rien, et qu'on n'affiche donc pas.
        assert!(!text.contains("e-322"), "{text}");
    }

    /// La valeur qu'on vient de mettre dans RAX avant un `exit` : rien
    /// d'imprimable, rien de flottant, mais un signé qui compte.
    #[test]
    fn a_negative_value_says_so() {
        let text = of(Lang::Fr, u64::MAX);
        assert!(text.contains("18446744073709551615"), "{text}");
        assert!(text.contains("-1"), "c'est aussi -1 : {text}");
    }

    /// Les bits d'un vrai double se lisent comme un nombre.
    #[test]
    fn real_float_bits_read_as_a_number() {
        let text = of(Lang::Fr, 2.5f64.to_bits());
        assert!(text.contains("2.5"), "{text}");
    }

    /// La calculatrice montre toutes les bases, toujours les mêmes.
    ///
    /// Aucune ne disparaît selon la valeur : une ligne qui va et vient
    /// ferait bouger l'affichage pendant qu'on tape.
    #[test]
    fn the_calculator_shows_every_base_every_time() {
        let petite = all_bases(Lang::Fr, 31);
        let grande = all_bases(Lang::Fr, u64::MAX);
        assert_eq!(petite.len(), grande.len(), "le même nombre de lignes");
        let valeurs: Vec<&str> = petite.iter().map(|(_, v)| v.as_str()).collect();
        assert!(valeurs.contains(&"0x1f"), "{valeurs:?}");
        assert!(valeurs.contains(&"31"), "{valeurs:?}");
        assert!(valeurs.contains(&"0o37"), "{valeurs:?}");
        assert!(valeurs.contains(&"00011111"), "{valeurs:?}");
        // Le signé est toujours là, même quand il répète le non signé :
        // c'est ce qu'on vient vérifier devant un `cmp`.
        assert!(
            all_bases(Lang::Fr, u64::MAX).iter().any(|(_, v)| v == "-1"),
            "0xffffffffffffffff, c'est -1"
        );
    }

    /// Les octets se lisent comme des lettres quand ils en sont.
    #[test]
    fn a_value_that_spells_something_says_so() {
        // Les octets 68 65 6c 6c en mémoire : « hell ». Écrite à
        // l'endroit, la même valeur donnerait « lleh », qui n'apprend
        // rien — c'est la mémoire qu'on veut lire, pas le nombre.
        let lignes = all_bases(Lang::Fr, 0x6c6c6568);
        let ascii = lignes
            .iter()
            .find(|(nom, _)| *nom == strings(Lang::Fr).reading_ascii)
            .expect("la ligne ASCII");
        assert_eq!(ascii.1, "hell....", "{lignes:?}");
    }

    /// Et les deux langues disent la même chose autrement.
    #[test]
    fn both_languages_answer() {
        assert_ne!(of(Lang::Fr, 42), of(Lang::En, 42));
        assert!(of(Lang::En, 42).contains("Unsigned"));
    }
}
