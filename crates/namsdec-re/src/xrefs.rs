//! Qui appelle quoi, et qui pointe où.
//!
//! C'est la première question qu'on pose d'un binaire qu'on n'a pas
//! écrit : cette fonction, qui l'appelle ? cette chaîne, qui la lit ?
//! Un désassemblage seul ne répond pas — il faut le relire en entier en
//! notant les cibles, ce qu'on fait à la main faute de mieux.
//!
//! Les références croisées sont ce carnet de notes, tenu pour vous.
//! Elles servent aussi à retrouver les fonctions d'un binaire dépouillé :
//! une adresse appelée par un `call` est une fonction, que son nom ait
//! été retiré ou non.

use std::collections::BTreeMap;

use iced_x86::{FlowControl, Instruction, OpKind, Register};
use namsdec_disasm::DecodedInstruction;

/// Ce qu'une instruction fait de l'adresse qu'elle désigne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Un `call` : la cible est une fonction.
    Call,
    /// Un saut, conditionnel ou non : la cible est dans la même
    /// fonction, le plus souvent.
    Jump,
    /// Une lecture ou une écriture en mémoire à une adresse fixe.
    Data,
    /// Une adresse chargée sans être suivie — `lea`, ou une immédiate
    /// qui tombe dans le programme.
    Address,
}

/// Une référence : d'où elle part, et ce qu'elle fait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ref {
    pub from: u64,
    pub kind: Kind,
}

/// Toutes les références d'un programme, rangées par cible.
#[derive(Debug, Default, Clone)]
pub struct Xrefs {
    to: BTreeMap<u64, Vec<Ref>>,
}

impl Xrefs {
    /// Qui désigne `addr` ?
    pub fn to(&self, addr: u64) -> &[Ref] {
        self.to.get(&addr).map(Vec::as_slice).unwrap_or_default()
    }

    /// Toutes les adresses désignées, dans l'ordre.
    pub fn targets(&self) -> impl Iterator<Item = (&u64, &Vec<Ref>)> {
        self.to.iter()
    }

    /// Les adresses appelées par un `call` : les fonctions du programme,
    /// que leurs noms aient survécu ou non.
    ///
    /// Triées par nombre d'appelants décroissant : ce qui est appelé de
    /// partout est ce par quoi l'on commence à lire.
    pub fn functions(&self) -> Vec<(u64, usize)> {
        let mut found: Vec<(u64, usize)> = self
            .to
            .iter()
            .filter_map(|(addr, refs)| {
                let calls = refs.iter().filter(|r| r.kind == Kind::Call).count();
                (calls > 0).then_some((*addr, calls))
            })
            .collect();
        found.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        found
    }

    /// Les adresses qu'un saut vise sans qu'aucun appel ne les vise :
    /// les débuts de boucle et de branche, ce qu'un désassembleur
    /// appelle des étiquettes.
    ///
    /// Distinguées des fonctions : on entre dans une fonction, on
    /// revient d'une fonction ; une étiquette, elle, appartient à celle
    /// qui la contient.
    pub fn labels(&self) -> Vec<u64> {
        self.to
            .iter()
            .filter(|(_, refs)| {
                refs.iter().any(|r| r.kind == Kind::Jump)
                    && !refs.iter().any(|r| r.kind == Kind::Call)
            })
            .map(|(addr, _)| *addr)
            .collect()
    }

    /// Les boucles : un saut qui revient en arrière, et d'où il part.
    ///
    /// C'est la définition la plus simple qui soit juste — un saut vers
    /// une adresse déjà passée ferme un chemin, et ce chemin est une
    /// boucle. Elle ne dit pas *combien de fois* on y tourne : cela ne
    /// se sait qu'en exécutant, et le compteur de passages de la
    /// gouttière est là pour ça.
    pub fn loops(&self) -> Vec<(u64, u64)> {
        let mut found: Vec<(u64, u64)> = self
            .to
            .iter()
            .flat_map(|(target, refs)| {
                refs.iter()
                    .filter(move |r| r.kind == Kind::Jump && r.from >= *target)
                    .map(move |r| (*target, r.from))
            })
            .collect();
        found.sort_unstable();
        found
    }

    pub fn is_empty(&self) -> bool {
        self.to.is_empty()
    }
}

/// Relit le désassemblage et note toutes les références.
///
/// `range` borne ce qu'on considère comme une adresse du programme : une
/// constante qui ne tombe pas dedans est un nombre, pas un pointeur, et
/// la prendre pour une référence noierait les vraies.
pub fn scan(disasm: &[DecodedInstruction], range: std::ops::Range<u64>) -> Xrefs {
    let mut xrefs = Xrefs::default();
    for decoded in disasm {
        let Some(instr) = namsdec_disasm::decode_one(&decoded.bytes, decoded.address) else {
            continue;
        };
        let from = decoded.address;
        // La cible d'un saut ou d'un appel : elle est dans l'encodage,
        // pas dans un registre — un `call rax` ne dit rien d'ici.
        if let Some(target) = branch_target(&instr) {
            let kind = if instr.flow_control() == FlowControl::Call {
                Kind::Call
            } else {
                Kind::Jump
            };
            xrefs.to.entry(target).or_default().push(Ref { from, kind });
            continue;
        }
        for index in 0..instr.op_count() {
            match instr.op_kind(index) {
                // Une adresse absolue ou relative à RIP : le cas des
                // données d'un programme, où qu'il soit chargé.
                OpKind::Memory => {
                    let base = instr.memory_base();
                    let target = if base == Register::RIP || base == Register::EIP {
                        instr.next_ip().wrapping_add(instr.memory_displacement64())
                    } else if base == Register::None && instr.memory_index() == Register::None {
                        instr.memory_displacement64()
                    } else {
                        continue;
                    };
                    if range.contains(&target) {
                        xrefs.to.entry(target).or_default().push(Ref {
                            from,
                            kind: Kind::Data,
                        });
                    }
                }
                // Une constante qui tombe dans le programme : c'est
                // presque toujours une adresse — `mov rsi, message`.
                OpKind::Immediate32 | OpKind::Immediate64 | OpKind::Immediate32to64 => {
                    let Ok(value) = instr.try_immediate(index) else {
                        continue;
                    };
                    if range.contains(&value) {
                        xrefs.to.entry(value).or_default().push(Ref {
                            from,
                            kind: Kind::Address,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    xrefs
}

/// L'adresse que vise un saut ou un appel écrit en dur.
fn branch_target(instr: &Instruction) -> Option<u64> {
    matches!(
        instr.op0_kind(),
        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64
    )
    .then(|| instr.near_branch_target())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disassemble(bytes: &[u8], base: u64) -> Vec<DecodedInstruction> {
        namsdec_disasm::disassemble(bytes, base)
    }

    /// Un `call` deux fois vers la même adresse, et un saut de boucle :
    /// c'est ce qu'on relève à la main en lisant un désassemblage.
    #[test]
    fn calls_and_jumps_are_noted_by_target() {
        // 0x400000: e8 05 00 00 00   call 0x40000a
        // 0x400005: e8 00 00 00 00   call 0x40000a
        // 0x40000a: 90               nop
        // 0x40000b: eb fd            jmp  0x40000a
        let bytes = [
            0xe8, 0x05, 0x00, 0x00, 0x00, 0xe8, 0x00, 0x00, 0x00, 0x00, 0x90, 0xeb, 0xfd,
        ];
        let xrefs = scan(&disassemble(&bytes, 0x400000), 0x400000..0x401000);

        let vers = xrefs.to(0x40000a);
        assert_eq!(vers.len(), 3, "{vers:?}");
        assert_eq!(
            vers.iter().filter(|r| r.kind == Kind::Call).count(),
            2,
            "deux appels"
        );
        assert_eq!(
            vers.iter().filter(|r| r.kind == Kind::Jump).count(),
            1,
            "et un saut"
        );

        // Et c'est donc une fonction, appelée deux fois — et non une
        // étiquette, bien qu'un saut la vise aussi : on entre dans une
        // fonction, on revient d'une fonction.
        assert_eq!(xrefs.functions(), vec![(0x40000a, 2)]);
        assert!(xrefs.labels().is_empty());
    }

    /// Une adresse chargée dans un registre est une référence aux
    /// données — c'est ainsi qu'on retrouve qui lit une chaîne.
    #[test]
    fn an_address_loaded_into_a_register_counts() {
        // mov esi, 0x401000  (be 00 10 40 00)
        let bytes = [0xbe, 0x00, 0x10, 0x40, 0x00];
        let xrefs = scan(&disassemble(&bytes, 0x400000), 0x400000..0x402000);
        assert_eq!(
            xrefs.to(0x401000),
            [Ref {
                from: 0x400000,
                kind: Kind::Address
            }]
        );
    }

    /// Une constante qui ne tombe pas dans le programme est un nombre,
    /// pas un pointeur : la compter noierait les vraies références.
    #[test]
    fn a_plain_number_is_not_a_reference() {
        // mov esi, 60   (be 3c 00 00 00)
        let bytes = [0xbe, 0x3c, 0x00, 0x00, 0x00];
        let xrefs = scan(&disassemble(&bytes, 0x400000), 0x400000..0x402000);
        assert!(xrefs.is_empty(), "{xrefs:?}");
    }

    /// Une boucle, c'est un saut qui revient en arrière.
    #[test]
    fn a_backward_jump_closes_a_loop() {
        // 0x400000: 90        nop
        // 0x400001: eb fd     jmp 0x400000   (en arrière)
        // 0x400003: eb 00     jmp 0x400005   (en avant)
        let bytes = [0x90, 0xeb, 0xfd, 0xeb, 0x00];
        let xrefs = scan(&disassemble(&bytes, 0x400000), 0x400000..0x401000);
        assert_eq!(
            xrefs.loops(),
            vec![(0x400000, 0x400001)],
            "seul le saut en arrière ferme un chemin"
        );
    }

    /// Un appel dont la cible est dans un registre ne dit rien d'ici :
    /// on ne l'invente pas.
    #[test]
    fn an_indirect_call_says_nothing() {
        // call rax  (ff d0)
        let xrefs = scan(&disassemble(&[0xff, 0xd0], 0x400000), 0x400000..0x402000);
        assert!(xrefs.functions().is_empty());
    }
}
