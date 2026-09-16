//! Les fragments de code qui finissent par `ret`.
//!
//! Quand la pile n'est pas exécutable — ce que dit NX —, du code déposé
//! dedans ne s'exécutera pas. Reste à réutiliser celui du programme :
//! on enchaîne des fragments qui se terminent chacun par un `ret`, la
//! pile ne contenant plus que des adresses. C'est la technique dite ROP,
//! et c'est la raison d'être des protections que le panneau d'à côté
//! énumère : RELRO, le canari de pile, IBT.
//!
//! Les lister sert à comprendre ce que ces protections empêchent, et à
//! voir qu'un binaire ordinaire en contient des centaines sans que
//! personne les ait mis là : un `ret` précédé de trois octets qui se
//! décodent forme un fragment, même au milieu d'une autre instruction.
//! C'est d'ailleurs ce que corrige IBT, en exigeant que les sauts
//! indirects atterrissent sur un `endbr64`.

use namsdec_disasm::DecodedInstruction;

/// Un fragment : où il commence, et ce qu'il fait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gadget {
    pub addr: u64,
    /// Les instructions, `ret` compris, dans l'ordre.
    pub instructions: Vec<String>,
}

impl Gadget {
    /// Le fragment écrit sur une ligne, comme on l'écrit dans une note.
    pub fn text(&self) -> String {
        self.instructions.join(" ; ")
    }
}

/// Combien d'instructions au plus avant le `ret`. Au-delà, un fragment
/// fait autre chose que ce qu'on attend de lui.
const MAX_INSTRUCTIONS: usize = 4;
/// Jusqu'où remonter, en octets, avant un `ret`.
const LOOK_BACK: u64 = 12;

/// Cherche les fragments dans les octets exécutables chargés à `base`.
///
/// `limit` borne la récolte : un binaire de quelques centaines de
/// kilo-octets en contient des milliers, et les afficher tous n'apprend
/// rien de plus que les premiers.
pub fn find(bytes: &[u8], base: u64, limit: usize) -> Vec<Gadget> {
    let mut found = Vec::new();
    for (offset, byte) in bytes.iter().enumerate() {
        // 0xc3 : `ret`. C'est le seul octet qu'on cherche — tout le
        // reste se décode à partir de lui, à reculons.
        if *byte != 0xc3 {
            continue;
        }
        let ret_addr = base + offset as u64;
        // On repart de chaque point d'entrée possible avant le `ret` :
        // un décalage qui tombe au milieu d'une instruction donne une
        // autre suite, et c'est tout l'intérêt.
        for back in 1..=LOOK_BACK {
            let Some(start) = ret_addr.checked_sub(back) else {
                break;
            };
            // En deçà du début des octets : il n'y a rien à lire là.
            let Some(from) = start.checked_sub(base) else {
                break;
            };
            let Ok(from) = usize::try_from(from) else {
                break;
            };
            if from >= bytes.len() {
                continue;
            }
            let window = &bytes[from..=offset];
            if let Some(gadget) = decode_chain(window, start, ret_addr) {
                // Les doublons n'apprennent rien : deux décalages
                // peuvent mener au même fragment.
                if !found.iter().any(|g: &Gadget| g.addr == gadget.addr) {
                    found.push(gadget);
                }
            }
        }
        if found.len() >= limit {
            break;
        }
    }
    found.truncate(limit);
    found
}

/// Décode la suite qui commence à `start` et vérifie qu'elle tombe
/// exactement sur le `ret`, sans le dépasser ni s'arrêter avant.
fn decode_chain(window: &[u8], start: u64, ret_addr: u64) -> Option<Gadget> {
    let mut instructions = Vec::new();
    let mut at = start;
    let mut consumed = 0usize;
    while at < ret_addr {
        let instr = namsdec_disasm::decode_one(&window[consumed..], at)?;
        let text = namsdec_disasm::format_one(&instr);
        // Un saut ou un appel au milieu : la suite ne se lit plus, elle
        // part ailleurs.
        if text.starts_with('j') || text.starts_with("call") || text.starts_with("ret") {
            return None;
        }
        instructions.push(text);
        consumed += instr.len();
        at += instr.len() as u64;
        if instructions.len() > MAX_INSTRUCTIONS {
            return None;
        }
    }
    // Tombé pile sur le `ret` ? Sinon, la suite le chevauche et ce
    // fragment n'existe pas.
    if at != ret_addr || instructions.is_empty() {
        return None;
    }
    instructions.push("ret".to_owned());
    Some(Gadget {
        addr: start,
        instructions,
    })
}

/// Les fragments d'un désassemblage déjà fait : les octets des
/// instructions, mis bout à bout.
pub fn from_disasm(disasm: &[DecodedInstruction], limit: usize) -> Vec<Gadget> {
    let Some(first) = disasm.first() else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    for instruction in disasm {
        bytes.extend_from_slice(&instruction.bytes);
    }
    find(&bytes, first.address, limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le fragment le plus courant : `pop rdi ; ret`, celui qui met un
    /// argument en place avant d'appeler autre chose.
    #[test]
    fn a_pop_and_return_is_found() {
        // 5f c3 : pop rdi ; ret
        let gadgets = find(&[0x5f, 0xc3], 0x400000, 10);
        assert_eq!(gadgets.len(), 1, "{gadgets:?}");
        assert_eq!(gadgets[0].addr, 0x400000);
        assert_eq!(gadgets[0].text(), "pop rdi ; ret");
    }

    /// Un `ret` seul n'est pas un fragment : il ne fait rien d'utile à
    /// qui enchaîne, et le lister noierait les autres.
    #[test]
    fn a_bare_return_is_not_a_gadget() {
        assert!(find(&[0xc3], 0x400000, 10).is_empty());
    }

    /// Un saut au milieu coupe la suite : elle ne mène plus au `ret`.
    #[test]
    fn a_jump_breaks_the_chain() {
        // eb 00 : jmp +0 — puis ret. La suite part ailleurs.
        let gadgets = find(&[0xeb, 0x00, 0xc3], 0x400000, 10);
        assert!(
            gadgets.iter().all(|g| !g.text().contains("jmp")),
            "{gadgets:?}"
        );
    }

    /// Le fragment qu'on ne voit pas : un `ret` précédé d'octets qui
    /// appartiennent à une autre instruction se décode quand même, à
    /// partir du milieu. C'est ce que IBT empêche.
    #[test]
    fn a_gadget_can_start_in_the_middle_of_an_instruction() {
        // 48 c7 c0 5f c3 00 00 : mov rax, 0xc35f — et à l'octet 3,
        // on lit « pop rdi ; ret ».
        let gadgets = find(&[0x48, 0xc7, 0xc0, 0x5f, 0xc3, 0x00, 0x00], 0x400000, 10);
        assert!(
            gadgets.iter().any(|g| g.text() == "pop rdi ; ret"),
            "un fragment se cache au milieu : {gadgets:?}"
        );
    }

    /// La récolte est bornée : un vrai binaire en contient des milliers.
    #[test]
    fn the_harvest_is_bounded() {
        let mut bytes = Vec::new();
        for _ in 0..500 {
            bytes.extend_from_slice(&[0x5f, 0xc3]);
        }
        assert_eq!(find(&bytes, 0x400000, 20).len(), 20);
    }
}
