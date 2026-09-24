//! Ce que le binaire emprunte à d'autres : les fonctions importées.
//!
//! Un `call 0x4010a0` ne dit rien. Le même appel, quand on sait que
//! `0x4010a0` est l'entrée de `strlen` dans la table de liaison, dit
//! tout : c'est l'information numéro un quand on lit du code qu'on n'a
//! pas écrit, et celle qu'aucun désassembleur brut ne donne.
//!
//! La méthode ne fait confiance à aucune convention de disposition :
//! chaque entrée de `.plt` (ou `.plt.sec`, `.plt.got`) est **décodée**,
//! le `jmp [rip + …]` qu'elle contient donne l'emplacement de la table
//! globale, et c'est la relocation posée à cet emplacement qui nomme le
//! symbole. Un binaire qui range ses entrées autrement se lit quand
//! même.

use std::collections::BTreeMap;

/// Une fonction importée, et l'entrée par laquelle on l'appelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// Le nom du symbole, tel que la table dynamique le donne.
    pub name: String,
    /// L'adresse de l'entrée de liaison : ce qu'un `call` désigne.
    pub plt: u64,
    /// L'emplacement que l'éditeur de liens remplira à l'exécution.
    pub got: u64,
}

const SHT_RELA: u32 = 4;
/// Relocations qui nomment une fonction appelée par la table de liaison,
/// et celles des tables déjà résolues au chargement (`-z now`).
const R_X86_64_JUMP_SLOT: u32 = 7;
const R_X86_64_GLOB_DAT: u32 = 6;
const R_X86_64_IRELATIVE: u32 = 37;

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

fn string_at(bytes: &[u8], at: usize) -> Option<String> {
    let rest = bytes.get(at..)?;
    let end = rest.iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&rest[..end]).into_owned())
}

/// Les fonctions importées, triées par adresse d'entrée.
///
/// Rend une liste vide pour un binaire statique, un objet sans table
/// dynamique, ou tout ce qui n'est pas un ELF64 : il n'y a alors rien à
/// nommer, et ce n'est pas une erreur.
pub fn scan(bytes: &[u8]) -> Vec<Import> {
    let noms = relocations(bytes).unwrap_or_default();
    if noms.is_empty() {
        return Vec::new();
    }
    let mut imports: Vec<Import> = Vec::new();
    for (start, addr, data) in plt_sections(bytes).unwrap_or_default() {
        let mut decoder =
            iced_x86::Decoder::with_ip(64, &data, addr, iced_x86::DecoderOptions::NONE);
        let mut instr = iced_x86::Instruction::default();
        while decoder.can_decode() {
            decoder.decode_out(&mut instr);
            if instr.mnemonic() != iced_x86::Mnemonic::Jmp
                || instr.op0_kind() != iced_x86::OpKind::Memory
                || !instr.is_ip_rel_memory_operand()
            {
                continue;
            }
            let got = instr.ip_rel_memory_address();
            let Some(name) = noms.get(&got) else { continue };
            // L'entrée commence au début de son bloc de seize octets :
            // le `jmp` est la première instruction pour une `.plt`
            // classique, la seconde quand un `endbr64` la précède.
            let plt = instr.ip() - (instr.ip() - start) % 16;
            imports.push(Import {
                name: name.clone(),
                plt,
                got,
            });
        }
    }
    imports.sort_by(|a, b| a.plt.cmp(&b.plt).then_with(|| a.name.cmp(&b.name)));
    imports.dedup();
    imports
}

/// L'emplacement d'une adresse virtuelle dans le fichier.
///
/// Les modules lisent le fichier tel qu'il est sur le disque, et le
/// désassemblage que montre namsdec part, lui, du premier octet du
/// fichier. Convertir une adresse de section en position de fichier est
/// ce qui permet de rapprocher les deux.
pub fn file_offset(bytes: &[u8], vaddr: u64) -> Option<u64> {
    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        let addr = u64_at(bytes, header + 0x10)?;
        let offset = u64_at(bytes, header + 0x18)?;
        let size = u64_at(bytes, header + 0x20)?;
        // Une section sans adresse n'est projetée nulle part : elle ne
        // peut contenir aucune adresse virtuelle.
        if addr != 0 && vaddr >= addr && vaddr < addr + size {
            return Some(offset + (vaddr - addr));
        }
    }
    // À défaut de sections, les segments. Un binaire dépouillé n'a plus
    // de table de sections — il tourne quand même, parce que le noyau ne
    // lit que les segments —, et sans ce repli aucune adresse ne se
    // retrouverait dans le fichier. C'est précisément le cas des
    // crackmes.
    let phoff = u64_at(bytes, 0x20)? as usize;
    let phentsize = u16_at(bytes, 0x36)? as usize;
    let phnum = u16_at(bytes, 0x38)? as usize;
    for index in 0..phnum {
        let header = phoff.checked_add(index.checked_mul(phentsize)?)?;
        // Seuls les segments chargeables portent des adresses.
        if u32_at(bytes, header)? != 1 {
            continue;
        }
        let offset = u64_at(bytes, header + 0x08)?;
        let vaddr_segment = u64_at(bytes, header + 0x10)?;
        let filesz = u64_at(bytes, header + 0x20)?;
        if vaddr >= vaddr_segment && vaddr < vaddr_segment + filesz {
            return Some(offset + (vaddr - vaddr_segment));
        }
    }
    None
}

/// L'emplacement de chaque relocation et le nom qu'elle porte.
fn relocations(bytes: &[u8]) -> Option<BTreeMap<u64, String>> {
    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    let mut noms = BTreeMap::new();
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        if u32_at(bytes, header + 0x04)? != SHT_RELA {
            continue;
        }
        // `sh_link` désigne la table de symboles que ces relocations
        // indexent, et cette table désigne à son tour ses chaînes.
        let symtab = u32_at(bytes, header + 0x28)? as usize;
        let symtab_header = shoff.checked_add(symtab.checked_mul(shentsize)?)?;
        let sym_offset = u64_at(bytes, symtab_header + 0x18)? as usize;
        let sym_entry = u64_at(bytes, symtab_header + 0x38)? as usize;
        let strtab = u32_at(bytes, symtab_header + 0x28)? as usize;
        let strtab_header = shoff.checked_add(strtab.checked_mul(shentsize)?)?;
        let str_offset = u64_at(bytes, strtab_header + 0x18)? as usize;

        let offset = u64_at(bytes, header + 0x18)? as usize;
        let size = u64_at(bytes, header + 0x20)? as usize;
        let entry = u64_at(bytes, header + 0x38)? as usize;
        if entry == 0 || sym_entry == 0 {
            continue;
        }
        for at in (offset..offset.checked_add(size)?).step_by(entry) {
            let Some(place) = u64_at(bytes, at) else {
                break;
            };
            let Some(info) = u64_at(bytes, at + 8) else {
                break;
            };
            let kind = info as u32;
            if kind != R_X86_64_JUMP_SLOT && kind != R_X86_64_GLOB_DAT && kind != R_X86_64_IRELATIVE
            {
                continue;
            }
            let sym = (info >> 32) as usize;
            // Une relocation IRELATIVE ne nomme personne : c'est le
            // programme lui-même qui choisira, au chargement, laquelle
            // de ses variantes installer.
            if kind == R_X86_64_IRELATIVE || sym == 0 {
                continue;
            }
            let Some(name_offset) = u32_at(bytes, sym_offset + sym * sym_entry) else {
                continue;
            };
            let Some(name) = string_at(bytes, str_offset + name_offset as usize) else {
                continue;
            };
            if !name.is_empty() {
                noms.insert(place, name);
            }
        }
    }
    Some(noms)
}

/// Les sections de liaison : leur adresse de départ et leur contenu.
fn plt_sections(bytes: &[u8]) -> Option<Vec<(u64, u64, Vec<u8>)>> {
    let shoff = u64_at(bytes, 0x28)? as usize;
    let shentsize = u16_at(bytes, 0x3a)? as usize;
    let shnum = u16_at(bytes, 0x3c)? as usize;
    let shstrndx = u16_at(bytes, 0x3e)? as usize;
    let str_header = shoff.checked_add(shstrndx.checked_mul(shentsize)?)?;
    let str_offset = u64_at(bytes, str_header + 0x18)? as usize;

    let mut sections = Vec::new();
    for index in 0..shnum {
        let header = shoff.checked_add(index.checked_mul(shentsize)?)?;
        let name_offset = u32_at(bytes, header)? as usize;
        let name = string_at(bytes, str_offset + name_offset)?;
        if !name.starts_with(".plt") {
            continue;
        }
        let addr = u64_at(bytes, header + 0x10)?;
        let offset = u64_at(bytes, header + 0x18)? as usize;
        let size = u64_at(bytes, header + 0x20)? as usize;
        let data = bytes.get(offset..offset.checked_add(size)?)?.to_vec();
        sections.push((addr, addr, data));
    }
    Some(sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un binaire du système, pas un cas choisi : `/bin/ls` appelle la
    /// bibliothèque C par sa table de liaison, et chaque entrée doit
    /// retrouver son nom.
    #[test]
    fn a_system_binary_names_what_it_borrows() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        let imports = scan(&bytes);
        assert!(
            imports.len() > 20,
            "un binaire lié dynamiquement importe beaucoup : {}",
            imports.len()
        );
        // Des fonctions que tout programme de ce genre emploie.
        for attendu in ["malloc", "free"] {
            assert!(
                imports.iter().any(|i| i.name == attendu),
                "{attendu} devrait être importée : {:?}",
                imports.iter().map(|i| &i.name).collect::<Vec<_>>()
            );
        }
        // Une entrée par adresse, et des adresses dans le programme.
        for import in &imports {
            assert!(import.plt > 0 && import.got > 0);
            assert_eq!(import.plt % 16, 0, "une entrée est alignée");
            // Et chaque entrée se retrouve dans le fichier : c'est ce
            // qui permet de la rapprocher du désassemblage affiché.
            assert!(
                file_offset(&bytes, import.plt).is_some(),
                "{} : entrée hors de toute section",
                import.name
            );
        }
    }

    /// Tout ce que la machine a sous la main, pas un binaire choisi.
    ///
    /// Un analyseur qui marche sur un exemple et panique sur le
    /// vingtième fichier venu n'est pas un analyseur. Ce test lit les
    /// binaires du système jusqu'à concurrence de vingt, et exige que
    /// chacun rende quelque chose de cohérent — ou rien, ce qui est une
    /// réponse aussi.
    #[test]
    fn every_binary_on_this_machine_is_read_without_flinching() {
        let Ok(dir) = std::fs::read_dir("/usr/bin") else {
            eprintln!("/usr/bin illisible : test ignoré");
            return;
        };
        let mut lus = 0usize;
        for entry in dir.flatten() {
            if lus >= 20 {
                break;
            }
            let Ok(meta) = entry.metadata() else { continue };
            // Les gros fichiers ne disent rien de plus et coûtent du
            // temps de test ; les liens sont suivis par `read`.
            if !meta.is_file() || meta.len() > 4 << 20 {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            if !bytes.starts_with(b"\x7fELF") {
                continue;
            }
            lus += 1;

            let imports = scan(&bytes);
            for import in &imports {
                assert!(
                    !import.name.is_empty(),
                    "{:?} : une entrée sans nom",
                    entry.path()
                );
                assert_eq!(
                    import.plt % 16,
                    0,
                    "{:?} : entrée mal alignée",
                    entry.path()
                );
                assert!(
                    file_offset(&bytes, import.plt).is_some(),
                    "{:?} : {} hors de toute section",
                    entry.path(),
                    import.name
                );
                // Deux entrées ne peuvent pas désigner le même
                // emplacement de la table globale sous deux noms.
                assert_eq!(
                    imports
                        .iter()
                        .filter(|autre| autre.got == import.got)
                        .count(),
                    1,
                    "{:?} : {:#x} nommé deux fois",
                    entry.path(),
                    import.got
                );
            }
        }
        assert!(lus > 0, "aucun binaire lu : le test n'a rien prouvé");
    }

    /// Rien à nommer n'est pas une erreur : un tampon quelconque rend
    /// une liste vide plutôt que de paniquer.
    #[test]
    fn anything_else_yields_nothing() {
        assert!(scan(&[]).is_empty());
        assert!(scan(&[0u8; 64]).is_empty());
        assert!(scan(b"\x7fELF not really").is_empty());
    }
}
