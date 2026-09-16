//! Lecture de l'ELF64 produit par `ld` — juste ce qu'il faut pour charger
//! un programme en mémoire simulée et relier ses lignes source à des
//! adresses.
//!
//! Volontairement minimal : le point d'entrée et les sections à charger,
//! rien de plus. Écrit pour relire la sortie de l'éditeur de liens qu'on
//! vient d'appeler soi-même, il sert aussi à ouvrir un ELF64 venu du
//! disque — un binaire qu'on n'a pas produit se charge alors à *son*
//! adresse, avec *son* point d'entrée, au lieu d'être désassemblé depuis
//! son en-tête comme s'il s'agissait de code.
//!
//! Il lit aussi la table des symboles, quand `ld` en laisse une : ce sont
//! les étiquettes écrites dans le source, et les revoir en face d'une
//! adresse est toute la différence entre « 0x401002 » et « tampon+2 ».
//!
//! Ce qu'il ne lit pas — relocations, formats abîmés — reste du ressort
//! de `namsdec-re`, qui a d'autres exigences.

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
    /// `.data` et `.bss` s'écrivent ; `.text` et `.rodata` non. C'est ce
    /// qui permet de refuser à un programme d'écrire dans son propre
    /// code plutôt que de le laisser faire en silence.
    pub writable: bool,
}

/// Une étiquette du programme, telle qu'elle est écrite dans le source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub addr: u64,
    /// La taille déclarée, souvent nulle : nasm ne la renseigne que pour
    /// les symboles passés par `size`.
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct Elf {
    pub entry: u64,
    pub sections: Vec<Section>,
    /// Les étiquettes du programme, triées par adresse croissante.
    pub symbols: Vec<Symbol>,
}

const SHF_ALLOC: u64 = 0x2;
const SHF_WRITE: u64 = 0x1;
const SHF_EXECINSTR: u64 = 0x4;
/// `.bss` : la section occupe de la mémoire mais rien dans le fichier.
const SHT_NOBITS: u32 = 8;
/// La table des symboles laissée par l'éditeur de liens.
const SHT_SYMTAB: u32 = 2;
/// Types de symboles qui désignent quelque chose en mémoire : une
/// donnée, une fonction, ou une étiquette sans type — le cas ordinaire
/// en assembleur.
const STT_NOTYPE: u8 = 0;
const STT_OBJECT: u8 = 1;
const STT_FUNC: u8 = 2;
/// L'index de section « aucune » : un symbole absolu ou indéfini, qui ne
/// nomme pas une adresse du programme.
const SHN_UNDEF: u16 = 0;
const SHN_ABS: u16 = 0xfff1;

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
            writable: flags & SHF_WRITE != 0,
        });
    }

    let symbols = parse_symbols(bytes, shoff, shentsize, shnum).unwrap_or_default();
    Some(Elf {
        entry,
        sections,
        symbols,
    })
}

/// Les étiquettes du programme, lues dans `.symtab`.
///
/// Rend `None` — et non une erreur — quand il n'y en a pas : un binaire
/// dépouillé (`strip`) se charge et s'exécute très bien, il se lit
/// seulement moins agréablement.
fn parse_symbols(
    bytes: &[u8],
    shoff: usize,
    shentsize: usize,
    shnum: usize,
) -> Option<Vec<Symbol>> {
    let mut symbols = Vec::new();
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        if u32_at(bytes, header + 0x04)? != SHT_SYMTAB {
            continue;
        }
        // `sh_link` d'une table de symboles donne la section de chaînes
        // où sont leurs noms.
        let strtab = u32_at(bytes, header + 0x28)? as usize;
        let strtab_header = shoff.checked_add(strtab.checked_mul(shentsize)?)?;
        let str_offset = u64_at(bytes, strtab_header + 0x18)? as usize;
        let str_size = u64_at(bytes, strtab_header + 0x20)? as usize;
        let names = bytes.get(str_offset..str_offset.checked_add(str_size)?)?;

        let offset = u64_at(bytes, header + 0x18)? as usize;
        let size = u64_at(bytes, header + 0x20)? as usize;
        let entry_size = u64_at(bytes, header + 0x38)? as usize;
        if entry_size == 0 {
            continue;
        }
        for at in (offset..offset.checked_add(size)?).step_by(entry_size) {
            let name_offset = u32_at(bytes, at)? as usize;
            let info = *bytes.get(at + 4)?;
            let shndx = u16_at(bytes, at + 6)?;
            let value = u64_at(bytes, at + 8)?;
            let sym_size = u64_at(bytes, at + 16)?;
            // Le type est dans les quatre bits bas de st_info.
            let kind = info & 0xf;
            if !matches!(kind, STT_NOTYPE | STT_OBJECT | STT_FUNC) {
                continue;
            }
            if shndx == SHN_UNDEF || shndx == SHN_ABS {
                continue;
            }
            let name = names
                .get(name_offset..)?
                .iter()
                .position(|&b| b == 0)
                .map(|end| {
                    String::from_utf8_lossy(&names[name_offset..name_offset + end]).into_owned()
                })
                .unwrap_or_default();
            // Les symboles que produit l'assembleur pour son propre usage
            // — le nom du fichier, les étiquettes locales `.L…` — ne
            // disent rien à qui lit son programme.
            if name.is_empty() || name.starts_with(".L") || name.ends_with(".asm") {
                continue;
            }
            symbols.push(Symbol {
                name,
                addr: value,
                size: sym_size,
            });
        }
    }
    symbols.sort_by_key(|s| (s.addr, s.name.clone()));
    symbols.dedup();
    Some(symbols)
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

#[cfg(test)]
mod tests {

    /// Les étiquettes du source se retrouvent dans l'ELF lié, à leur
    /// adresse — c'est ce qui permet de dire « tampon » plutôt que
    /// « 0x401000 ».
    #[test]
    fn labels_come_back_with_their_addresses() {
        let Some(_) = crate::find_nasm() else {
            eprintln!("nasm absent : test ignoré");
            return;
        };
        if !crate::find_linker() {
            eprintln!("ld absent : test ignoré");
            return;
        }
        let source = "\
global _start
section .data
tampon: db 1, 2, 3, 4
section .text
_start:
    mov al, [tampon]
    hlt
";
        let out = crate::assemble(source, 0x400000).expect("le programme doit s'assembler");
        let tampon = out
            .symbols
            .iter()
            .find(|s| s.name == "tampon")
            .expect("l'étiquette tampon doit être connue");
        let start = out
            .symbols
            .iter()
            .find(|s| s.name == "_start")
            .expect("le point d'entrée aussi");
        assert_eq!(start.addr, out.entry);
        assert!(
            tampon.addr != start.addr,
            "les deux étiquettes sont dans des sections différentes"
        );
        // Triés par adresse : l'interface s'appuie dessus pour trouver
        // « la dernière étiquette avant cette adresse ».
        let mut adresses: Vec<u64> = out.symbols.iter().map(|s| s.addr).collect();
        let triees = adresses.clone();
        adresses.sort_unstable();
        assert_eq!(adresses, triees, "les symboles doivent être triés");
        // Et pas de bruit d'assembleur dans la liste.
        assert!(
            !out.symbols.iter().any(|s| s.name.ends_with(".asm")),
            "le nom du fichier n'est pas une étiquette : {:?}",
            out.symbols
        );
    }
}
