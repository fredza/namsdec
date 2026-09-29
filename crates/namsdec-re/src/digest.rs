//! L'empreinte d'un fichier : SHA-256 et CRC-32.
//!
//! C'est la première ligne d'un compte rendu d'analyse, avant même de
//! dire ce que le programme fait : *de quel fichier parle-t-on ?* Deux
//! personnes qui lisent « le crackme » n'ont pas forcément le même
//! octet sous les yeux, et l'empreinte est ce qui tranche.
//!
//! **Écrites à la main, sans dépendance.** SHA-256 tient en soixante
//! lignes, CRC-32 en quinze, et les deux sont figées par une norme : il
//! n'y a rien à suivre, rien à mettre à jour, et les vecteurs d'essai
//! publics disent si c'est juste. Ajouter deux bibliothèques pour cela
//! reviendrait à agrandir la surface d'un programme qui lit des
//! binaires de quelqu'un d'autre.
//!
//! **Pourquoi pas MD5 ni SHA-1.** Ce sont des empreintes cassées : on
//! sait fabriquer deux fichiers différents qui les partagent. Elles
//! servent encore à interroger de vieilles bases d'échantillons, ce qui
//! est un usage légitime — mais namsdec est un outil qui explique, et
//! afficher une empreinte à côté d'une autre sans dire que l'une ne
//! prouve rien apprendrait le faux.

/// L'empreinte SHA-256 de ces octets.
pub fn sha256(données: &[u8]) -> [u8; 32] {
    // Les trente-deux premières constantes de la norme : les parties
    // fractionnaires des racines cubiques des soixante-quatre premiers
    // nombres premiers.
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // Le rembourrage : un bit à 1, des zéros, puis la longueur en bits
    // sur soixante-quatre. C'est ce qui empêche deux messages dont l'un
    // est le préfixe de l'autre de rendre la même empreinte.
    let mut message = données.to_vec();
    let bits = (données.len() as u64).wrapping_mul(8);
    message.push(0x80);
    // Jusqu'à cinquante-six octets modulo soixante-quatre, et non
    // jusqu'à un bloc plein : les huit derniers octets du bloc sont
    // réservés à la longueur. La condition « ou bien le bloc est plein »
    // qu'on écrit d'instinct s'arrête un bloc trop tôt dès que le `0x80`
    // tombe dans les huit derniers octets — le cas des messages de
    // cinquante-six à soixante-trois octets, et d'eux seuls.
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_be_bytes());

    for bloc in message.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, mot) in bloc.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes([mot[0], mot[1], mot[2], mot[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choix = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(choix)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majorité = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majorité);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (place, valeur) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *place = place.wrapping_add(valeur);
        }
    }

    let mut out = [0u8; 32];
    for (i, mot) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&mot.to_be_bytes());
    }
    out
}

/// Le CRC-32 de ces octets, celui de zip et de PNG.
///
/// Ce n'est **pas** une empreinte cryptographique : on fabrique sans
/// peine deux fichiers qui le partagent. C'est un contrôle de
/// transmission — « le fichier est-il arrivé entier ? » —, et c'est
/// ainsi qu'il est présenté.
pub fn crc32(données: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for octet in données {
        crc ^= u32::from(*octet);
        for _ in 0..8 {
            // Le polynôme inversé de CRC-32, celui que tout le monde
            // emploie depuis Ethernet.
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Des octets en hexadécimal, minuscules et sans séparateur — la forme
/// qu'on colle dans un compte rendu et qu'on compare à l'œil.
pub fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les vecteurs d'essai de la norme.
    ///
    /// Une empreinte écrite à la main ne vaut que si on la confronte à
    /// des valeurs qu'on n'a pas calculées soi-même : celles-ci sont
    /// publiées avec l'algorithme, et n'importe qui peut les retrouver.
    #[test]
    fn sha256_matches_the_published_vectors() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // Cinquante-six octets : le cas qui déborde sur un second bloc à
        // cause du rembourrage, et que les implémentations ratent.
        assert_eq!(
            hex(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // Un message long, sur plusieurs blocs.
        assert_eq!(
            hex(&sha256(&[b'a'; 1000])),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    /// Les longueurs où le rembourrage bascule d'un bloc.
    ///
    /// C'est là que les implémentations écrites à la main se trompent,
    /// et c'est là que celle-ci s'est trompée : quand le bit de fin
    /// tombe dans les huit derniers octets du bloc, il faut un bloc de
    /// plus, et la condition qu'on écrit d'instinct s'arrête un bloc
    /// trop tôt. Les dix longueurs autour de la bascule sont donc
    /// gravées ici, toutes calculées ailleurs.
    #[test]
    fn sha256_crosses_the_block_boundary_correctly() {
        const ATTENDUS: [(usize, &str); 10] = [
            (
                55,
                "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
            ),
            (
                56,
                "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
            ),
            (
                57,
                "f13b2d724659eb3bf47f2dd6af1accc87b81f09f59f2b75e5c0bed6589dfe8c6",
            ),
            (
                58,
                "d5c039b748aa64665782974ec3dc3025c042edf54dcdc2b5de31385b094cb678",
            ),
            (
                59,
                "111bb261277afd65f0744b247cd3e47d386d71563d0ed995517807d5ebd4fba3",
            ),
            (
                60,
                "11ee391211c6256460b6ed375957fadd8061cafbb31daf967db875aebd5aaad4",
            ),
            (
                61,
                "35d5fc17cfbbadd00f5e710ada39f194c5ad7c766ad67072245f1fad45f0f530",
            ),
            (
                62,
                "f506898cc7c2e092f9eb9fadae7ba50383f5b46a2a4fe5597dbb553a78981268",
            ),
            (
                63,
                "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34",
            ),
            (
                64,
                "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
            ),
        ];
        for (longueur, attendu) in ATTENDUS {
            let message = vec![b'a'; longueur];
            assert_eq!(hex(&sha256(&message)), attendu, "{longueur} octets");
        }
    }

    /// Le vecteur d'essai de CRC-32, celui que tout le monde cite.
    #[test]
    fn crc32_matches_the_published_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
    }

    /// Deux contenus différents n'ont pas la même empreinte, et le même
    /// contenu l'a toujours.
    #[test]
    fn the_fingerprint_follows_the_content() {
        assert_eq!(sha256(b"namsdec"), sha256(b"namsdec"));
        assert_ne!(sha256(b"namsdec"), sha256(b"namsdeb"));
        assert_ne!(crc32(b"namsdec"), crc32(b"namsdeb"));
    }

    /// L'hexadécimal est celui qu'on colle : minuscules, sans blancs.
    #[test]
    fn hex_is_the_form_one_pastes() {
        assert_eq!(hex(&[0x00, 0x0f, 0xff]), "000fff");
        assert_eq!(hex(&[]), "");
    }
}
