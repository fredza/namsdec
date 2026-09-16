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
    if signed != value as i64 || signed < 0 {
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

    /// Et les deux langues disent la même chose autrement.
    #[test]
    fn both_languages_answer() {
        assert_ne!(of(Lang::Fr, 42), of(Lang::En, 42));
        assert!(of(Lang::En, 42).contains("Unsigned"));
    }
}
