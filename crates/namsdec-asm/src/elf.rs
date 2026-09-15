//! Lecture de l'ELF64 produit par `ld` — juste ce qu'il faut pour charger
//! un programme en mémoire simulée et relier ses lignes source à des
//! adresses.
//!
//! Volontairement minimal, et volontairement ici plutôt que dans une
//! crate d'analyse : on ne lit pas un binaire quelconque venu du disque,
//! on relit la sortie de l'éditeur de liens qu'on vient d'appeler
//! soi-même. Les seules choses à en tirer sont le point d'entrée et les
//! sections à charger. Le vrai support ELF de la phase 2 (reverse
//! engineering) aura d'autres exigences — symboles, relocations, formats
//! malformés — et sa place est dans `namsdec-re`.

/// Une section chargeable du programme lié.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    /// Adresse virtuelle où la section doit se trouver en mémoire.
    pub addr: u64,
    /// Contenu. Vide et de taille `size` pour `.bss`, qui n'occupe pas de
    /// place dans le fichier mais bien en mémoire.
    pub data: Vec<u8>,
    pub size: usize,
    pub executable: bool,
}

#[derive(Debug, Clone)]
pub struct Elf {
    pub entry: u64,
    pub sections: Vec<Section>,
}

const SHF_ALLOC: u64 = 0x2;
const SHF_EXECINSTR: u64 = 0x4;
/// `.bss` : la section occupe de la mémoire mais rien dans le fichier.
const SHT_NOBITS: u32 = 8;

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn u64_at(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

/// Lit l'en-tête, la table des sections et les noms. Rend `None` sur tout
/// ce qui n'est pas un ELF64 petit-boutiste intact — le cas ne devrait pas
/// se produire sur notre propre sortie de `ld`, et s'il se produit, il n'y
/// a rien à sauver du fichier.
pub fn parse(bytes: &[u8]) -> Option<Elf> {
    if bytes.get(..4)? != b"\x7fELF" {
        return None;
    }
    // EI_CLASS = 2 (64 bits), EI_DATA = 1 (petit-boutiste).
    if *bytes.get(4)? != 2 || *bytes.get(5)? != 1 {
        return None;
    }

    let entry = u64_at(bytes, 0x18)?;
    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    let shstrndx = u16_at(bytes, 0x3e)? as usize;

    // La table des noms de sections est elle-même une section : il faut la
    // lire en premier pour pouvoir nommer les autres.
    let shstr_header = shoff.checked_add(shstrndx.checked_mul(shentsize)?)?;
    let shstr_offset = u64_at(bytes, shstr_header + 0x18)? as usize;
    let shstr_size = u64_at(bytes, shstr_header + 0x20)? as usize;
    let names = bytes.get(shstr_offset..shstr_offset.checked_add(shstr_size)?)?;

    let mut sections = Vec::new();
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        let flags = u64_at(bytes, header + 0x08)?;
        // Seules les sections présentes en mémoire à l'exécution nous
        // intéressent : les tables de symboles et de chaînes n'ont pas
        // d'adresse et n'ont rien à faire dans la mémoire simulée.
        if flags & SHF_ALLOC == 0 {
            continue;
        }

        let name_offset = u32_at(bytes, header)? as usize;
        let name = names
            .get(name_offset..)?
            .iter()
            .position(|&b| b == 0)
            .map(|end| String::from_utf8_lossy(&names[name_offset..name_offset + end]).into_owned())
            .unwrap_or_default();

        let kind = u32_at(bytes, header + 0x04)?;
        let addr = u64_at(bytes, header + 0x10)?;
        let offset = u64_at(bytes, header + 0x18)? as usize;
        let size = u64_at(bytes, header + 0x20)? as usize;

        let data = if kind == SHT_NOBITS {
            Vec::new()
        } else {
            bytes.get(offset..offset.checked_add(size)?)?.to_vec()
        };

        sections.push(Section {
            name,
            addr,
            data,
            size,
            executable: flags & SHF_EXECINSTR != 0,
        });
    }

    Some(Elf { entry, sections })
}

impl Elf {
    /// L'adresse de la section nommée, pour rattacher à une adresse réelle
    /// les décalages du listing de nasm, qui sont relatifs à leur section.
    pub fn section_addr(&self, name: &str) -> Option<u64> {
        self.sections
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.addr)
    }

    /// Aplatit les sections en une image mémoire contiguë : l'adresse de
    /// départ, et les octets à y charger.
    ///
    /// Les trous entre sections (l'écart entre `.text` et `.data`, aligné
    /// sur une page) sont comblés par des zéros — comme `.bss`, qui n'a
    /// rien à copier mais doit exister en mémoire.
    pub fn image(&self) -> Option<(u64, Vec<u8>)> {
        let start = self.sections.iter().map(|s| s.addr).min()?;
        let end = self.sections.iter().map(|s| s.addr + s.size as u64).max()?;
        let mut image = vec![0u8; (end - start) as usize];
        for section in &self.sections {
            let at = (section.addr - start) as usize;
            image[at..at + section.data.len()].copy_from_slice(&section.data);
        }
        Some((start, image))
    }
}
