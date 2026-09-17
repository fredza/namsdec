//! Suivre un pointeur jusqu'à ce qu'il cesse d'en être un.
//!
//! Devant `0x40ff08 : 0x402000`, la question n'est jamais « quel
//! nombre ? » mais « et à *cette* adresse, il y a quoi ? ». On le fait à
//! la main, une lecture après l'autre, jusqu'à tomber sur une chaîne,
//! une instruction, ou un nombre qui n'est plus une adresse. C'est le
//! geste que pwndbg appelle *telescope*, et il vaut d'être fait par la
//! machine : c'est du déchiffrage, pas du raisonnement.
//!
//! Ce module rend la chaîne complète, annotée à chaque maillon. Aucune
//! langue : l'interface met les mots autour.

use namsdec_core::Memory;

/// Ce qu'on a trouvé au bout d'un maillon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum What {
    /// La valeur pointe une zone lisible : la chaîne continue.
    Pointer,
    /// Une instruction, parce que la valeur pointe du code exécutable.
    Code(String),
    /// Une chaîne de caractères imprimables terminée par un zéro.
    Text(String),
    /// Un nombre : plus rien à suivre.
    Number,
    /// L'adresse n'est pas lisible — hors de la mémoire projetée.
    Unreadable,
}

/// Un maillon : l'adresse lue, et ce qu'elle contenait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// L'adresse dont on a lu les huit octets.
    pub addr: u64,
    /// Les huit octets lus, comme un entier.
    pub value: u64,
    pub what: What,
}

/// Combien de maillons au plus.
///
/// Cinq, comme pwndbg : au-delà, la chaîne cesse d'être une explication
/// et devient un vidage.
pub const LIMIT: usize = 5;

/// Combien d'octets d'une chaîne on rapporte au plus.
const TEXT_LIMIT: usize = 48;

/// Suit `addr` de pointeur en pointeur.
///
/// Le premier maillon est la lecture *à* `addr` ; les suivants lisent ce
/// que le précédent contenait. La chaîne s'arrête sur ce qui n'est pas
/// une adresse lisible, sur un cycle, ou au bout de `limit` maillons.
pub fn chain(mem: &Memory, addr: u64, limit: usize) -> Vec<Link> {
    let mut links: Vec<Link> = Vec::new();
    let mut at = addr;
    for _ in 0..limit.min(LIMIT) {
        // Un cycle : une adresse qui se pointe elle-même, ou deux qui se
        // renvoient l'une à l'autre. On s'arrête au deuxième passage,
        // sans quoi la chaîne tournerait jusqu'à la limite sans rien
        // apprendre.
        if links.iter().any(|link| link.addr == at) {
            break;
        }
        let Ok(value) = mem.read_u64(at) else {
            links.push(Link {
                addr: at,
                value: 0,
                what: What::Unreadable,
            });
            break;
        };
        let what = describe(mem, value);
        let suivre = what == What::Pointer;
        links.push(Link {
            addr: at,
            value,
            what,
        });
        if !suivre {
            break;
        }
        at = value;
    }
    links
}

/// Ce qu'une valeur désigne, sans la suivre.
///
/// L'ordre des questions est celui qu'on se pose soi-même : est-ce
/// adressable ? est-ce du code ? est-ce du texte ? sinon, c'est un
/// nombre.
pub fn describe(mem: &Memory, value: u64) -> What {
    let Some(perms) = mem.perms_at(value) else {
        return What::Number;
    };
    if !perms.read {
        return What::Number;
    }
    if perms.execute
        && let Ok(window) = window_at(mem, value, 16)
        && let Some(instr) = namsdec_disasm::decode_one(window, value)
    {
        return What::Code(namsdec_disasm::format_one(&instr));
    }
    if let Some(text) = text_at(mem, value) {
        return What::Text(text);
    }
    // Lisible, mais rien de reconnaissable : c'est peut-être un pointeur
    // vers autre chose, et c'est au maillon suivant de le dire.
    if mem.read_u64(value).is_ok() {
        return What::Pointer;
    }
    What::Number
}

/// Les octets disponibles à `addr`, jusqu'à `len` — moins en fin de
/// mémoire, où la fenêtre est tronquée plutôt que refusée.
fn window_at(mem: &Memory, addr: u64, len: usize) -> Result<&[u8], namsdec_core::MemError> {
    mem.peek_bytes(addr, len).or_else(|e| {
        let reste = mem.range().end.saturating_sub(addr) as usize;
        if reste == 0 {
            Err(e)
        } else {
            mem.peek_bytes(addr, reste.min(len))
        }
    })
}

/// La chaîne terminée par zéro qui commence là, si c'en est une.
///
/// Exigeante à dessein : au moins deux caractères imprimables et un
/// zéro. Sans cela, deux octets de hasard suffiraient à faire passer
/// n'importe quelle donnée pour du texte.
fn text_at(mem: &Memory, addr: u64) -> Option<String> {
    let window = window_at(mem, addr, TEXT_LIMIT).ok()?;
    let mut texte = String::new();
    for &byte in window {
        if byte == 0 {
            return (texte.chars().count() >= 2).then_some(texte);
        }
        // Imprimable au sens strict : espace compris, tabulation et saut
        // de ligne compris, le reste non.
        if byte == b'\t' || byte == b'\n' || (0x20..0x7f).contains(&byte) {
            texte.push(byte as char);
        } else {
            return None;
        }
    }
    None
}

/// Chercher une suite d'octets dans toute la mémoire projetée.
///
/// C'est le `search` de pwndbg : on a une chaîne, une valeur, une
/// adresse, et on veut savoir *où* elle se trouve — dans quel tampon,
/// combien de fois, et si le programme l'a recopiée ailleurs.
///
/// Seules les zones lisibles sont parcourues, et les résultats sont
/// bornés : une recherche qui rendrait dix mille adresses ne se lit pas.
pub fn search(mem: &Memory, needle: &[u8], limit: usize) -> Vec<u64> {
    let mut trouvés = Vec::new();
    if needle.is_empty() {
        return trouvés;
    }
    let zones = if mem.regions().is_empty() {
        vec![mem.range()]
    } else {
        mem.regions()
            .into_iter()
            .filter(|region| region.perms.read)
            .map(|region| region.range)
            .collect()
    };
    for zone in zones {
        // Bornée à la mémoire réellement allouée : une zone peut être
        // déclarée au-delà du bloc simulé.
        let début = zone.start.max(mem.range().start);
        let fin = zone.end.min(mem.range().end);
        if fin <= début {
            continue;
        }
        let Ok(bytes) = mem.peek_bytes(début, (fin - début) as usize) else {
            continue;
        };
        for (offset, window) in bytes.windows(needle.len()).enumerate() {
            if window == needle {
                trouvés.push(début + offset as u64);
                if trouvés.len() >= limit {
                    return trouvés;
                }
            }
        }
    }
    trouvés.sort_unstable();
    trouvés.dedup();
    trouvés
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::Perms;

    /// Chercher des octets : la chaîne qu'on vient d'écrire, et rien
    /// d'autre que ce qui est lisible.
    #[test]
    fn a_search_finds_the_bytes_where_they_are() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.map(0x400000..0x400800, Perms::TEXT, "code");
        mem.map(0x400800..0x401000, Perms::DATA, "données");
        mem.load(0x400900, b"salut").unwrap();
        mem.load(0x400950, b"salut").unwrap();

        assert_eq!(search(&mem, b"salut", 10), vec![0x400900, 0x400950]);
        // La limite arrête la recherche plutôt que de rendre une liste
        // qu'on ne lira pas.
        assert_eq!(search(&mem, b"salut", 1), vec![0x400900]);
        // Ce qui n'y est pas ne s'y trouve pas.
        assert!(search(&mem, b"bonjour", 10).is_empty());
        assert!(search(&mem, b"", 10).is_empty());
    }

    /// Ce qui n'est pas lisible n'est pas fouillé : on cherche ce qu'un
    /// programme peut lire, pas ce que le simulateur garde en mémoire.
    #[test]
    fn a_search_skips_what_cannot_be_read() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.load(0x400100, "caché".as_bytes()).unwrap();
        // Une zone sans droit de lecture : l'écriture est passée par
        // `load`, qui ne les vérifie pas.
        mem.map(
            0x400000..0x401000,
            Perms {
                read: false,
                write: false,
                execute: false,
            },
            "interdit",
        );
        assert!(search(&mem, "caché".as_bytes(), 10).is_empty());
    }

    /// Un pointeur vers un pointeur vers une chaîne : les trois maillons
    /// se suivent, et le dernier dit le texte.
    #[test]
    fn a_pointer_to_a_pointer_to_a_string_reads_to_the_end() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.map(0x400000..0x401000, Perms::DATA, "données");
        mem.write_u64(0x400000, 0x400100).unwrap();
        mem.write_u64(0x400100, 0x400200).unwrap();
        mem.write_bytes(0x400200, b"salut\0").unwrap();

        // Deux maillons, pas trois : le dernier *décrit* déjà ce qu'il
        // pointe, comme le fait pwndbg — la chaîne s'arrête sur le texte
        // plutôt que de le relire.
        let chaîne = chain(&mem, 0x400000, LIMIT);
        assert_eq!(chaîne.len(), 2, "{chaîne:?}");
        assert_eq!(chaîne[0].value, 0x400100);
        assert_eq!(chaîne[0].what, What::Pointer);
        assert_eq!(chaîne[1].value, 0x400200);
        assert_eq!(chaîne[1].what, What::Text("salut".to_owned()));
    }

    /// Une valeur qui ne pointe nulle part est un nombre, et la chaîne
    /// s'arrête là.
    #[test]
    fn a_number_is_not_followed() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.map(0x400000..0x401000, Perms::DATA, "données");
        mem.write_u64(0x400000, 42).unwrap();

        let chaîne = chain(&mem, 0x400000, LIMIT);
        assert_eq!(chaîne.len(), 1);
        assert_eq!(chaîne[0].what, What::Number);
    }

    /// Ce qui pointe du code exécutable se lit comme une instruction :
    /// c'est ainsi qu'on reconnaît une adresse de retour dans la pile.
    #[test]
    fn what_points_at_code_reads_as_an_instruction() {
        let mut mem = Memory::new(0x400000, 0x2000);
        mem.map(0x400000..0x401000, Perms::TEXT, "code");
        mem.map(0x401000..0x402000, Perms::DATA, "pile");
        // 48 89 d8 = mov rax, rbx
        mem.load(0x400000, &[0x48, 0x89, 0xd8]).unwrap();
        mem.write_u64(0x401000, 0x400000).unwrap();

        let chaîne = chain(&mem, 0x401000, LIMIT);
        assert_eq!(chaîne.len(), 1, "{chaîne:?}");
        assert_eq!(chaîne[0].value, 0x400000);
        assert_eq!(chaîne[0].what, What::Code("mov rax, rbx".to_owned()));
    }

    /// Une adresse qui se pointe elle-même ne fait pas tourner la
    /// chaîne : on s'arrête au deuxième passage.
    #[test]
    fn a_cycle_stops_the_chain() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.map(0x400000..0x401000, Perms::DATA, "données");
        mem.write_u64(0x400000, 0x400000).unwrap();

        let chaîne = chain(&mem, 0x400000, LIMIT);
        assert_eq!(chaîne.len(), 1, "{chaîne:?}");
    }

    /// Hors de toute zone projetée, on le dit — plutôt que de rendre un
    /// zéro qui ressemblerait à une lecture réussie.
    #[test]
    fn an_unmapped_address_says_so() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.map(0x400000..0x400800, Perms::DATA, "données");

        let chaîne = chain(&mem, 0x400900, LIMIT);
        assert_eq!(chaîne.len(), 1);
        assert_eq!(chaîne[0].what, What::Unreadable);
    }

    /// Deux octets de hasard ne font pas une chaîne de caractères.
    #[test]
    fn two_random_bytes_are_not_a_string() {
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.map(0x400000..0x401000, Perms::DATA, "données");
        mem.write_u64(0x400000, 0x400100).unwrap();
        mem.write_bytes(0x400100, &[0x41, 0x00]).unwrap();

        // Un seul caractère avant le zéro : pas assez pour appeler cela
        // du texte.
        let chaîne = chain(&mem, 0x400000, LIMIT);
        assert!(
            !matches!(chaîne.last().map(|l| &l.what), Some(What::Text(_))),
            "{chaîne:?}"
        );
    }
}
