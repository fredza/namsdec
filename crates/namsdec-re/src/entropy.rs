//! L'entropie : ce qui distingue du texte, du code, et du chiffré.
//!
//! Un octet pris au hasard dans un fichier n'apprend rien. La
//! *distribution* des octets, elle, dit beaucoup : du texte anglais
//! n'emploie qu'une cinquantaine de valeurs sur 256 et en répète
//! quelques-unes sans cesse ; du code x86 a ses propres habitudes
//! — `48` devant presque toutes les instructions 64 bits, `e8` devant
//! chaque appel ; des données compressées ou chiffrées, elles, n'ont
//! aucune habitude, et c'est justement ce qui les trahit.
//!
//! L'entropie de Shannon met un nombre là-dessus : **combien de bits
//! d'information porte un octet, en moyenne**. Zéro quand tous les
//! octets sont identiques, huit quand ils sont parfaitement imprévisibles.
//!
//! C'est la mesure qu'on lance en premier sur un binaire inconnu, parce
//! qu'elle répond en une image à « où est le vrai code ? » — un
//! empaqueteur laisse un petit déchiffreur en clair et un gros bloc à 7,9.

/// L'entropie de Shannon d'un bloc d'octets, en bits par octet.
///
/// Entre 0 et 8. Un bloc vide vaut zéro : il n'y a aucune information
/// dans rien, et c'est la seule réponse qui ne suppose pas.
pub fn shannon(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut compte = [0usize; 256];
    for b in bytes {
        compte[*b as usize] += 1;
    }
    let total = bytes.len() as f64;
    -compte
        .iter()
        .filter(|n| **n > 0)
        .map(|n| {
            let p = *n as f64 / total;
            p * p.log2()
        })
        .sum::<f64>()
}

/// Ce qu'une valeur d'entropie suggère.
///
/// Des seuils, donc des approximations — et il faut le dire : un petit
/// bloc de code bien dense monte à 6,5 sans être chiffré pour autant.
/// C'est une piste, pas un verdict, et le libellé le formule ainsi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Presque rien ne varie : du remplissage, des zéros, une table.
    Flat,
    /// La distribution d'un texte lisible.
    Text,
    /// Celle d'un code machine ordinaire.
    Code,
    /// Trop régulier pour du code, trop varié pour du texte.
    Mixed,
    /// Aucune habitude : compressé, chiffré, ou déjà empaqueté.
    Random,
}

impl Verdict {
    /// Le verdict qui correspond à cette entropie.
    ///
    /// Les bornes viennent de mesures ordinaires : du texte ASCII tourne
    /// autour de 4 à 4,5 ; du code x86 entre 5,5 et 6,5 ; au-delà de 7,2
    /// il n'y a plus de structure à voir, et à 7,9 c'est du bruit.
    pub fn of(entropy: f64) -> Self {
        match entropy {
            e if e < 1.5 => Verdict::Flat,
            e if e < 5.0 => Verdict::Text,
            e if e < 6.8 => Verdict::Code,
            e if e < 7.2 => Verdict::Mixed,
            _ => Verdict::Random,
        }
    }

    pub fn label(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Verdict::Flat, true) => "uniforme — remplissage, zéros, une table",
            (Verdict::Flat, false) => "flat — padding, zeros, a table",
            (Verdict::Text, true) => "du texte, ou des données très régulières",
            (Verdict::Text, false) => "text, or very regular data",
            (Verdict::Code, true) => "du code machine",
            (Verdict::Code, false) => "machine code",
            (Verdict::Mixed, true) => "mêlé — du code et des données",
            (Verdict::Mixed, false) => "mixed — code and data",
            (Verdict::Random, true) => "compressé, chiffré, ou empaqueté",
            (Verdict::Random, false) => "compressed, encrypted, or packed",
        }
    }
}

/// Une tranche de fichier, et ce qu'elle contient.
#[derive(Debug, Clone, Copy)]
pub struct Window {
    /// Le décalage du début de la tranche dans le fichier.
    pub offset: usize,
    pub len: usize,
    pub entropy: f64,
}

/// Découpe `bytes` en tranches et mesure chacune.
///
/// Une seule entropie pour tout un fichier ne sert à rien : elle mélange
/// l'en-tête, le code, les chaînes et le remplissage, et rend une moyenne
/// qui ne décrit aucun d'entre eux. C'est le *profil* qu'on veut — la
/// suite des mesures, où une bosse saute aux yeux.
///
/// `count` tranches au plus, d'au moins `MIN_WINDOW` octets : mesurer
/// l'entropie de trente-deux octets rendrait un nombre sans valeur,
/// puisqu'il n'y a pas de quoi voir une habitude.
pub fn profile(bytes: &[u8], count: usize) -> Vec<Window> {
    /// En deçà, la mesure ne veut rien dire : 256 octets, c'est déjà peu
    /// pour juger d'une distribution sur 256 valeurs possibles.
    const MIN_WINDOW: usize = 256;

    if bytes.is_empty() || count == 0 {
        return Vec::new();
    }
    let taille = bytes.len().div_ceil(count).max(MIN_WINDOW);
    bytes
        .chunks(taille)
        .enumerate()
        .map(|(i, tranche)| Window {
            offset: i * taille,
            len: tranche.len(),
            entropy: shannon(tranche),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux extrêmes, qui bornent tout le reste.
    #[test]
    fn the_two_extremes_are_zero_and_eight() {
        // Rien ne varie : aucune information.
        assert_eq!(shannon(&[0u8; 4096]), 0.0);
        assert_eq!(shannon(&[0x90u8; 100]), 0.0);
        // Les 256 valeurs une fois chacune : le maximum, exactement.
        let toutes: Vec<u8> = (0..=255u8).collect();
        assert!(
            (shannon(&toutes) - 8.0).abs() < 1e-9,
            "{}",
            shannon(&toutes)
        );
        // Et rien du tout vaut zéro, sans diviser par zéro au passage.
        assert_eq!(shannon(&[]), 0.0);
    }

    /// Deux valeurs équiprobables portent exactement un bit.
    ///
    /// C'est la définition même, et la vérifier sur un cas qu'on sait
    /// calculer de tête vaut mieux que de faire confiance à la formule.
    #[test]
    fn two_equal_values_carry_exactly_one_bit() {
        let mut bytes = vec![0u8; 512];
        bytes.extend(std::iter::repeat_n(1u8, 512));
        assert!((shannon(&bytes) - 1.0).abs() < 1e-9, "{}", shannon(&bytes));
    }

    /// Du texte, du code et du bruit ne se ressemblent pas.
    ///
    /// C'est tout l'objet du module : les distinguer sans les lire.
    #[test]
    fn text_code_and_noise_fall_into_different_verdicts() {
        let texte: Vec<u8> = "le petit chat boit du lait tous les matins dans la cuisine "
            .repeat(40)
            .into_bytes();
        assert_eq!(Verdict::of(shannon(&texte)), Verdict::Text);

        // Du remplissage : ce qu'on trouve entre deux sections.
        assert_eq!(Verdict::of(shannon(&[0u8; 2048])), Verdict::Flat);

        // Du bruit : une suite qui ne se répète pas avant longtemps.
        // Un générateur congruentiel suffit — on ne teste pas sa qualité,
        // seulement que l'entropie le voit comme imprévisible.
        let mut graine = 0x2545f4914f6cdd1du64;
        let bruit: Vec<u8> = (0..8192)
            .map(|_| {
                graine ^= graine << 13;
                graine ^= graine >> 7;
                graine ^= graine << 17;
                (graine >> 24) as u8
            })
            .collect();
        assert_eq!(Verdict::of(shannon(&bruit)), Verdict::Random);
    }

    /// Le profil découpe, et couvre tout le fichier sans trou.
    ///
    /// Un profil qui oublierait la fin cacherait exactement ce qu'on
    /// cherche : un bloc chiffré rangé en queue de fichier.
    #[test]
    fn the_profile_covers_the_whole_file() {
        let octets: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let tranches = profile(&octets, 16);
        assert!(!tranches.is_empty());
        let couvert: usize = tranches.iter().map(|w| w.len).sum();
        assert_eq!(couvert, octets.len(), "aucun octet ne doit manquer");
        // Et les tranches se suivent, sans recouvrement.
        for paire in tranches.windows(2) {
            assert_eq!(paire[0].offset + paire[0].len, paire[1].offset);
        }
    }

    /// Une bosse de bruit au milieu d'un fichier calme se voit.
    ///
    /// C'est le cas qui justifie le profil plutôt qu'une mesure unique :
    /// la moyenne du fichier entier ne dirait rien, la tranche le crie.
    #[test]
    fn a_block_of_noise_stands_out_in_the_profile() {
        let mut octets = vec![0u8; 4096];
        let mut graine = 0x9e3779b97f4a7c15u64;
        for i in 4096..8192 {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            octets.push((graine >> 24) as u8);
            let _ = i;
        }
        octets.extend(std::iter::repeat_n(0u8, 4096));

        let tranches = profile(&octets, 12);
        let plus_haute = tranches
            .iter()
            .max_by(|a, b| a.entropy.total_cmp(&b.entropy))
            .expect("il y a des tranches");
        assert!(
            plus_haute.entropy > 7.0,
            "la bosse doit ressortir : {plus_haute:?}"
        );
        // Et elle est bien au milieu, là où on l'a mise.
        assert!(
            (4096..8192).contains(&plus_haute.offset),
            "au mauvais endroit : {plus_haute:?}"
        );
        // Tandis que la moyenne du tout ne dirait rien de tranché.
        assert!(
            Verdict::of(shannon(&octets)) != Verdict::Random,
            "c'est justement pourquoi une mesure unique ne suffit pas"
        );
    }

    /// Aucune taille ne fait tomber le découpage.
    #[test]
    fn no_size_breaks_the_profile() {
        for taille in [0usize, 1, 2, 255, 256, 257, 1000, 4096] {
            let octets = vec![0x41u8; taille];
            for tranches in [0usize, 1, 7, 64, 1000] {
                let profil = profile(&octets, tranches);
                let couvert: usize = profil.iter().map(|w| w.len).sum();
                assert!(
                    couvert == taille || profil.is_empty(),
                    "taille {taille}, {tranches} tranches : {couvert} octets couverts"
                );
            }
        }
    }
}

