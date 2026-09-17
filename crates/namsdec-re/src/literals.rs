//! Les chaînes telles que le **code** les emploie, et non telles qu'un
//! scan des octets les devine.
//!
//! En C, une chaîne se termine par un zéro : la chercher dans les
//! données suffit. En Rust, en Go, en Zig, il n'y a pas de zéro — une
//! chaîne est une adresse **et** une longueur, et les littéraux sont
//! rangés bout à bout dans la section des constantes. Un scan naïf y lit
//! donc un seul bloc absurde :
//!
//! ```text
//! "Error flushing bufferError reading inputIncorrect password"
//! ```
//!
//! Alors que le programme, lui, connaît trois chaînes distinctes. La
//! découpe n'est pas dans les données : elle est dans le code, sous la
//! forme d'un couple d'instructions que tout compilateur produit —
//! l'adresse dans un registre, la longueur dans le suivant :
//!
//! ```text
//! lea rsi, [rip + 0x…]    ; l'adresse de la chaîne
//! mov edx, 17             ; sa longueur
//! call …                  ; et on l'affiche
//! ```
//!
//! Ce module relève ces couples. On retrouve alors les chaînes que le
//! programmeur a écrites, avec l'endroit d'où on s'en sert — ce qui est
//! précisément la question qu'on se pose devant un binaire inconnu.

use std::collections::BTreeMap;

use namsdec_disasm::{DecodedInstruction, Mnemonic, OpKind, Register};

/// Une chaîne, telle que le code la désigne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literal {
    /// Où elle commence en mémoire.
    pub addr: u64,
    /// Sa longueur, prise dans le code.
    pub len: u64,
    /// Son texte, lu à cette adresse sur cette longueur.
    pub text: String,
    /// L'adresse du `lea` qui la charge : l'endroit à aller lire.
    pub from: u64,
}

/// Combien d'instructions séparent au plus l'adresse de sa longueur.
///
/// Le compilateur les met souvent côte à côte, mais pas toujours : il
/// glisse volontiers une autre préparation d'argument entre les deux.
/// Au-delà de quatre, ce n'est plus la longueur de cette chaîne-là, et
/// l'associer serait inventer.
const PROXIMITÉ: usize = 4;

/// Une longueur plausible pour une chaîne d'un programme.
///
/// Zéro ne se distingue pas d'un registre mis à zéro pour autre chose,
/// et au-delà de quelques kilo-octets, ce n'est plus une chaîne mais un
/// tampon.
const LONGUEUR_MAX: u64 = 4096;

/// Les registres où l'on trouve une longueur, et celui d'adresse qui va
/// avec dans la convention System V.
///
/// `(rsi, rdx)` est la paire de `write` et de tout ce qui affiche en
/// Rust ; `(rdi, rsi)` celle d'une fonction qui reçoit la chaîne en
/// premier argument.
fn paires(adresse: Register) -> &'static [Register] {
    match adresse {
        Register::RDI => &[Register::RSI, Register::ESI],
        Register::RSI => &[Register::RDX, Register::EDX],
        Register::RDX => &[Register::RCX, Register::ECX],
        Register::RCX => &[Register::R8, Register::R8D],
        _ => &[],
    }
}

/// Le registre 64 bits derrière un registre 32 bits.
fn large(reg: Register) -> Register {
    match reg {
        Register::ESI => Register::RSI,
        Register::EDX => Register::RDX,
        Register::ECX => Register::RCX,
        Register::R8D => Register::R8,
        autre => autre,
    }
}

/// Lit `len` octets à l'adresse virtuelle `addr` dans un ELF chargé.
fn lire(
    sections: &[crate::elf_scan::SectionInfo],
    bytes: &[u8],
    addr: u64,
    len: u64,
) -> Option<String> {
    let section = sections
        .iter()
        .find(|s| !s.nobits && addr >= s.addr && addr + len <= s.addr + s.size)?;
    let début = (section.offset + (addr - section.addr)) as usize;
    let fin = début + len as usize;
    let tranche = bytes.get(début..fin)?;
    let texte = std::str::from_utf8(tranche).ok()?;
    // Ce qui n'est pas lisible n'est pas une chaîne : une longueur qui
    // tombe juste ne suffit pas à faire du texte.
    if texte
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return None;
    }
    Some(texte.to_owned())
}

/// Les chaînes que le code désigne, par adresse croissante.
///
/// Une même chaîne employée deux fois n'apparaît qu'une, avec le premier
/// endroit d'où on s'en sert : c'est là qu'on ira lire.
pub fn scan(
    disasm: &[DecodedInstruction],
    sections: &[crate::elf_scan::SectionInfo],
    bytes: &[u8],
) -> Vec<Literal> {
    let décodées: Vec<Option<namsdec_disasm::Decoded>> = disasm
        .iter()
        .map(|instr| namsdec_disasm::decode_one(&instr.bytes, instr.address))
        .collect();

    let mut trouvées: BTreeMap<(u64, u64), Literal> = BTreeMap::new();
    for (index, decoded) in décodées.iter().enumerate() {
        // Une adresse chargée : `lea reg, [rip + …]`.
        let Some(lea) = decoded else { continue };
        if lea.mnemonic() != Mnemonic::Lea
            || lea.op0_kind() != OpKind::Register
            || !lea.is_ip_rel_memory_operand()
        {
            continue;
        }
        let adresse = lea.ip_rel_memory_address();
        let registres = paires(lea.op0_register());
        if registres.is_empty() {
            continue;
        }

        // La longueur, dans les quelques instructions qui suivent.
        for suivant in décodées.iter().skip(index + 1).take(PROXIMITÉ).flatten() {
            // Un `call` clôt la préparation des arguments : au-delà, ce
            // qu'on lirait appartient à l'appel suivant.
            if suivant.flow_control() == namsdec_disasm::FlowControl::Call {
                break;
            }
            if suivant.mnemonic() != Mnemonic::Mov || suivant.op0_kind() != OpKind::Register {
                continue;
            }
            if !registres.contains(&suivant.op0_register()) {
                // Le registre d'adresse réécrit : la chaîne a changé, on
                // ne cherche plus la longueur de celle-ci.
                if large(suivant.op0_register()) == lea.op0_register() {
                    break;
                }
                continue;
            }
            let longueur = match suivant.op1_kind() {
                OpKind::Immediate8
                | OpKind::Immediate16
                | OpKind::Immediate32
                | OpKind::Immediate32to64
                | OpKind::Immediate64 => suivant.immediate(1),
                _ => break,
            };
            if longueur == 0 || longueur > LONGUEUR_MAX {
                break;
            }
            if let Some(texte) = lire(sections, bytes, adresse, longueur) {
                trouvées
                    .entry((adresse, longueur))
                    .or_insert_with(|| Literal {
                        addr: adresse,
                        len: longueur,
                        text: texte,
                        from: disasm[index].address,
                    });
            }
            break;
        }
    }
    trouvées.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le cas d'un binaire Rust : l'adresse et la longueur, côte à côte.
    #[test]
    fn a_string_is_cut_where_the_code_says_it_ends() {
        // Deux chaînes collées dans les données, comme Rust les range.
        // Le code, lui, sait où l'une finit et l'autre commence.
        let données = b"Correct PasswordIncorrect password";
        let mut fichier = vec![0u8; 0x1000];
        fichier[0x200..0x200 + données.len()].copy_from_slice(données);
        let sections = vec![crate::elf_scan::SectionInfo {
            name: ".rodata".into(),
            addr: 0x2000,
            size: 0x100,
            write: false,
            execute: false,
            offset: 0x200,
            nobits: false,
        }];

        // lea rsi, [rip+0]  ; mov edx, 16  ; call …
        //   à 0x1000 : lea de 7 octets, cible = 0x1007 + déplacement.
        let déplacement = (0x2000i64 - 0x1007) as i32;
        let mut code = vec![0x48, 0x8D, 0x35];
        code.extend_from_slice(&déplacement.to_le_bytes());
        code.extend_from_slice(&[0xBA, 0x10, 0x00, 0x00, 0x00]); // mov edx, 16
        code.extend_from_slice(&[0xE8, 0x00, 0x00, 0x00, 0x00]); // call
        let disasm = namsdec_disasm::disassemble(&code, 0x1000);

        let trouvées = scan(&disasm, &sections, &fichier);
        assert_eq!(trouvées.len(), 1, "{trouvées:?}");
        assert_eq!(trouvées[0].text, "Correct Password");
        assert_eq!(trouvées[0].len, 16);
        assert_eq!(trouvées[0].addr, 0x2000);
        assert_eq!(trouvées[0].from, 0x1000, "l'endroit où l'on s'en sert");
    }

    /// Une longueur absurde n'est pas une longueur : on ne coupe pas.
    #[test]
    fn an_absurd_length_is_not_a_length() {
        let sections = vec![crate::elf_scan::SectionInfo {
            name: ".rodata".into(),
            addr: 0x2000,
            size: 0x10,
            write: false,
            execute: false,
            offset: 0x200,
            nobits: false,
        }];
        let fichier = vec![b'A'; 0x1000];
        let déplacement = (0x2000i64 - 0x1007) as i32;
        let mut code = vec![0x48, 0x8D, 0x35];
        code.extend_from_slice(&déplacement.to_le_bytes());
        // mov edx, 0x100000 : bien au-delà de la section.
        code.extend_from_slice(&[0xBA, 0x00, 0x00, 0x10, 0x00]);
        let disasm = namsdec_disasm::disassemble(&code, 0x1000);
        assert!(scan(&disasm, &sections, &fichier).is_empty());
    }
}
