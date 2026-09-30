//! La place libre d'un binaire : le bourrage que l'éditeur de liens
//! laisse pour aligner.
//!
//! Un exécutable n'est jamais plein. Entre deux fonctions, entre deux
//! sections, à la fin d'un segment, l'éditeur de liens comble avec un
//! octet neutre — un zéro, un `nop` (0x90), un `int3` (0xcc) — jusqu'au
//! prochain multiple de 16 ou de la page. Ces trous se voient, et les
//! voir apprend deux choses à la fois : pourquoi un binaire pèse plus
//! que son code, et pourquoi un patch qui garde la longueur exacte n'est
//! pas la seule façon de le modifier — il y a de la place ailleurs.
//!
//! C'est de la lecture seule. Ce module ne dit que « voici où le fichier
//! ne contient rien d'utile » ; il n'y écrit pas, et ce qu'on ferait de
//! cette place est une autre histoire, qui passe par les correctifs et
//! leur capacité à défaire ce qu'ils posent.

use crate::elf_scan::{self, Segment};

/// Une suite de bourrage : de la place libre, une fois.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slack {
    /// L'adresse virtuelle du premier octet libre — celle que le
    /// désassemblage montre.
    pub vaddr: u64,
    /// Où ce trou commence dans le fichier.
    pub offset: u64,
    /// Combien d'octets libres à la file.
    pub len: u64,
    /// L'octet qui les remplit tous — ce qui dit au passage la
    /// convention de l'éditeur de liens : `0x00`, `0x90`, `0xcc`.
    pub filler: u8,
    /// Le segment où il vit est-il exécutable ? Un trou dans `.text` peut
    /// porter du code ; un trou dans les données, non.
    pub executable: bool,
}

/// En deçà de cette longueur, un trou n'apprend rien : deux octets
/// d'alignement sont partout, et les lister noierait les vraies zones.
const MIN: u64 = 8;

/// Les octets qui servent de bourrage. Un trou n'en est un que s'il est
/// fait d'un SEUL d'entre eux, répété : une suite de zéros au milieu du
/// code est de la place ; une suite d'octets variés est du code ou des
/// données, et n'est pas libre.
const BOURRAGES: [u8; 3] = [0x00, 0x90, 0xcc];

/// Toute la place libre d'un binaire, la plus grande d'abord.
///
/// On ne regarde que ce qui est chargé — les segments `PT_LOAD` —, parce
/// qu'un trou dans les en-têtes ou la table des symboles n'est pas de la
/// place utilisable : il ne finit pas en mémoire. Et l'on ne dépend pas
/// des en-têtes de section, qui manquent sur un binaire dépouillé : le
/// bourrage se reconnaît à lui-même.
pub fn find(bytes: &[u8]) -> Vec<Slack> {
    let Some(scan) = elf_scan::scan(bytes) else {
        return Vec::new();
    };

    let mut trouvés = Vec::new();
    for segment in &scan.segments {
        if segment.kind != "LOAD" {
            continue;
        }
        collect_in_segment(bytes, segment, &mut trouvés);
    }

    // La plus grande d'abord : c'est la seule qui intéresse quand on
    // cherche où mettre quelque chose, et à taille égale, la plus basse,
    // pour que deux lectures du même fichier donnent le même ordre.
    trouvés.sort_by(|a, b| b.len.cmp(&a.len).then(a.vaddr.cmp(&b.vaddr)));
    trouvés
}

/// Les trous d'un segment. On ne parcourt que ce que le fichier contient
/// vraiment — `filesz`, non `memsz` : `.bss` n'occupe rien sur le
/// disque, et le « trou » qu'on y verrait serait le contenu du voisin.
fn collect_in_segment(bytes: &[u8], segment: &Segment, out: &mut Vec<Slack>) {
    let début = segment.offset as usize;
    let fin = début
        .saturating_add(segment.filesz as usize)
        .min(bytes.len());
    let Some(tranche) = bytes.get(début..fin) else {
        return;
    };

    let mut i = 0usize;
    while i < tranche.len() {
        let octet = tranche[i];
        if !BOURRAGES.contains(&octet) {
            i += 1;
            continue;
        }
        // Une suite du même octet, aussi loin qu'elle va.
        let départ = i;
        while i < tranche.len() && tranche[i] == octet {
            i += 1;
        }
        let longueur = (i - départ) as u64;
        if longueur >= MIN {
            let offset_fichier = (début + départ) as u64;
            out.push(Slack {
                // L'adresse virtuelle se déduit du décalage dans le
                // segment : le même écart qu'entre `offset` et `vaddr`.
                vaddr: segment.vaddr + (offset_fichier - segment.offset),
                offset: offset_fichier,
                len: longueur,
                filler: octet,
                executable: segment.execute,
            });
        }
    }
}

/// Combien de place libre en tout, et la plus grande d'un tenant : de
/// quoi répondre « y a-t-il seulement la place ? » sans lire la liste.
pub fn totals(slack: &[Slack]) -> (u64, u64) {
    let somme = slack.iter().map(|s| s.len).sum();
    let plus_grande = slack.iter().map(|s| s.len).max().unwrap_or(0);
    (somme, plus_grande)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un tampon qui n'est pas un ELF ne rend aucun trou, et ne panique
    /// pas : un désassemblage brut passe aussi par ici.
    #[test]
    fn a_non_elf_yields_nothing() {
        assert!(find(b"pas un elf").is_empty());
        assert!(find(&[]).is_empty());
    }

    /// Une suite trop courte n'est pas de la place : deux octets
    /// d'alignement sont partout, et les compter noierait le reste.
    #[test]
    fn a_short_run_is_not_slack() {
        let mut seg = vec![0xaa; 4];
        seg.extend_from_slice(&[0x00; 4]); // trop court
        seg.extend_from_slice(&[0xbb; 4]);
        // Pas d'ELF autour : on éprouve la longueur minimale par le
        // parcours direct d'un segment fabriqué.
        let segment = Segment {
            kind: "LOAD",
            offset: 0,
            vaddr: 0x1000,
            filesz: seg.len() as u64,
            memsz: seg.len() as u64,
            read: true,
            write: false,
            execute: true,
        };
        let mut out = Vec::new();
        collect_in_segment(&seg, &segment, &mut out);
        assert!(
            out.is_empty(),
            "quatre octets ne font pas un trou : {out:?}"
        );
    }

    /// Une vraie suite est trouvée, à la bonne adresse et de la bonne
    /// longueur, et son octet de bourrage est rapporté.
    #[test]
    fn a_real_run_is_found_with_its_address() {
        // `offset` est absolu dans le FICHIER : le tampon porte donc un
        // en-tête factice avant le segment, comme un vrai ELF.
        let mut fichier = vec![0x7f; 0x40];
        let contenu_début = fichier.len();
        fichier.extend_from_slice(&[0xaa; 16]);
        fichier.extend_from_slice(&[0x90; 32]); // un vrai trou de nop
        fichier.extend_from_slice(&[0xbb; 8]);
        let segment = Segment {
            kind: "LOAD",
            offset: contenu_début as u64,
            vaddr: 0x401000,
            filesz: 56,
            memsz: 56,
            read: true,
            write: false,
            execute: true,
        };
        let mut out = Vec::new();
        collect_in_segment(&fichier, &segment, &mut out);
        assert_eq!(out.len(), 1, "{out:?}");
        let trou = &out[0];
        assert_eq!(trou.len, 32);
        assert_eq!(trou.filler, 0x90);
        assert_eq!(trou.vaddr, 0x401000 + 16);
        assert_eq!(trou.offset, (contenu_début + 16) as u64);
        assert!(trou.executable);
    }

    /// Ce qui n'occupe rien sur le disque n'est pas de la place :
    /// `filesz` borne la lecture, non `memsz`.
    #[test]
    fn bss_beyond_filesz_is_not_counted() {
        let seg = vec![0x00; 8]; // huit octets de fichier
        let segment = Segment {
            kind: "LOAD",
            offset: 0,
            vaddr: 0x1000,
            filesz: 8,
            memsz: 8192, // .bss : beaucoup de mémoire, aucun fichier
            read: true,
            write: true,
            execute: false,
        };
        let mut out = Vec::new();
        collect_in_segment(&seg, &segment, &mut out);
        // Les huit octets présents comptent ; pas les 8184 qui ne sont
        // pas dans le fichier.
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len, 8);
    }

    /// Les totaux résument sans qu'on lise la liste.
    #[test]
    fn totals_add_up() {
        let slack = vec![
            Slack {
                vaddr: 0x1000,
                offset: 0,
                len: 40,
                filler: 0x00,
                executable: true,
            },
            Slack {
                vaddr: 0x2000,
                offset: 0x1000,
                len: 12,
                filler: 0x90,
                executable: false,
            },
        ];
        assert_eq!(totals(&slack), (52, 40));
    }
}
