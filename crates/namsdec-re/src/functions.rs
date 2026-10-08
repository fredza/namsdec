//! Retrouver les fonctions d'un binaire qui n'a plus ses noms.
//!
//! Un binaire dépouillé ne dit ni où commencent ses fonctions, ni
//! laquelle est la sienne : `.text` est un bloc de deux cent quarante
//! kilo-octets, et le point d'entrée appartient à la bibliothèque
//! standard. La première question devant un programme inconnu est
//! toujours la même — **par où commencer à lire ?** — et c'est celle à
//! laquelle ce module répond.
//!
//! Deux réponses, de qualité différente, et namsdec dit laquelle est
//! laquelle :
//!
//! - Les **cibles d'appel** : toute adresse qu'un `call` vise est une
//!   fonction. C'est sûr, et c'est incomplet — ce qu'on n'appelle que
//!   par pointeur échappe à ce relevé.
//! - Le **`main` du programme** : sur un binaire lié à la bibliothèque C,
//!   `_start` le charge dans RDI avant d'appeler `__libc_start_main`.
//!   C'est une convention, pas une loi ; on la suit, et on le dit.

use std::collections::{BTreeMap, BTreeSet};

use namsdec_disasm::{DecodedInstruction, FlowControl, Mnemonic, OpKind, Register};

/// Comment on a su qu'une fonction commençait là.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// Le point d'entrée déclaré par l'ELF.
    Entry,
    /// Le `main` du programme, trouvé par la convention de démarrage.
    Main,
    /// Une adresse qu'au moins un `call` vise.
    Called,
}

/// Une fonction retrouvée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub start: u64,
    /// La dernière adresse qui lui appartient, au mieux de ce qu'on sait.
    pub last: u64,
    /// Combien d'appels différents y mènent.
    pub callers: usize,
    /// Ce qu'elle appelle, dans l'ordre des adresses.
    pub calls: Vec<u64>,
    pub origin: Origin,
}

impl Function {
    /// Un nom lisible, à défaut d'un vrai.
    ///
    /// `sub_10450` plutôt que rien : on a besoin de le prononcer pour en
    /// parler, et la forme est celle que tous les désassembleurs
    /// emploient.
    pub fn name(&self) -> String {
        match self.origin {
            Origin::Entry => "_start".to_owned(),
            Origin::Main => "main".to_owned(),
            Origin::Called => format!("sub_{:x}", self.start),
        }
    }

    pub fn size(&self) -> u64 {
        self.last.saturating_sub(self.start)
    }
}

/// L'adresse que `call` vise, quand elle est écrite en clair.
fn call_target(instr: &namsdec_disasm::Decoded) -> Option<u64> {
    if instr.flow_control() != FlowControl::Call {
        return None;
    }
    match instr.op0_kind() {
        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
            Some(instr.near_branch_target())
        }
        _ => None,
    }
}

/// Le `main` d'un programme lié à la bibliothèque C.
///
/// Le démarrage d'un programme C ne saute pas dans `main` : il le
/// **passe** à `__libc_start_main`, qui met le monde en place — les
/// arguments, l'environnement, les constructeurs — avant de l'appeler.
/// Le `main` est donc le premier argument, dans RDI, et c'est ce `lea
/// rdi, [rip + …]` qu'on va chercher juste avant l'appel.
///
/// On ne remonte que quelques instructions : au-delà, RDI vient
/// d'ailleurs, et deviner mentirait.
pub fn main_of(disasm: &[DecodedInstruction], entry: u64) -> Option<u64> {
    const REGARD: usize = 12;
    let début = disasm.iter().position(|i| i.address == entry)?;
    let mut candidat = None;
    for instr in disasm.iter().skip(début).take(REGARD) {
        let decoded = namsdec_disasm::decode_one(&instr.bytes, instr.address)?;
        // `lea rdi, [rip + X]` : l'adresse d'une fonction, mise dans le
        // premier argument.
        if decoded.mnemonic() == Mnemonic::Lea
            && decoded.op0_kind() == OpKind::Register
            && decoded.op0_register() == Register::RDI
            && decoded.is_ip_rel_memory_operand()
        {
            candidat = Some(decoded.ip_rel_memory_address());
        }
        // Le premier appel qui suit termine le démarrage : ce qu'on
        // tient dans RDI à ce moment-là est le `main`.
        if decoded.flow_control() == FlowControl::Call
            || decoded.flow_control() == FlowControl::IndirectCall
        {
            return candidat;
        }
    }
    None
}

/// Toutes les fonctions qu'on sait retrouver, par adresse croissante.
pub fn discover(disasm: &[DecodedInstruction], entry: u64) -> Vec<Function> {
    if disasm.is_empty() {
        return Vec::new();
    }
    // Qui appelle quoi : le relevé, en un passage.
    let mut appels: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    let mut décodées: Vec<(u64, Option<namsdec_disasm::Decoded>)> =
        Vec::with_capacity(disasm.len());
    for instr in disasm {
        let decoded = namsdec_disasm::decode_one(&instr.bytes, instr.address);
        if let Some(decoded) = &decoded
            && let Some(cible) = call_target(decoded)
        {
            appels.entry(cible).or_default().insert(instr.address);
        }
        décodées.push((instr.address, decoded));
    }

    let mut débuts: BTreeMap<u64, Origin> = BTreeMap::new();
    for cible in appels.keys() {
        débuts.insert(*cible, Origin::Called);
    }
    if disasm.iter().any(|i| i.address == entry) {
        débuts.insert(entry, Origin::Entry);
    }
    if let Some(main) = main_of(disasm, entry) {
        débuts.insert(main, Origin::Main);
    }

    // Chaque fonction va jusqu'au début de la suivante : c'est
    // approximatif — du bourrage traîne à la fin, une fonction peut en
    // contenir une autre —, et c'est ce que tout désassembleur fait en
    // première approche.
    let bornes: Vec<u64> = débuts.keys().copied().collect();
    let dernière = disasm.last().map(|i| i.address).unwrap_or(entry);

    // Les appels de chaque fonction, relevés EN UN SEUL PASSAGE.
    //
    // La version d'avant rebalayait les quatre cent mille instructions
    // POUR CHAQUE fonction (`filter(addr >= start && addr < fin)`) : avec
    // des milliers de fonctions, c'est le produit des deux — quadratique.
    // Mesuré trois secondes sur un binaire de trois mégaoctets, pendant
    // lesquelles le chargement figeait l'interface au point que le système
    // proposait de « forcer à quitter ».
    //
    // Chaque instruction tombe dans une et une seule fonction : celle dont
    // le début est le plus grand qui ne la dépasse pas. Une recherche
    // dichotomique dans `bornes` (trié) le dit en O(log f), sans supposer
    // que `décodées` soit rangé. On ne regarde que les instructions qui
    // appellent, c'est-à-dire presque aucune.
    let mut appels_par_fn: Vec<BTreeSet<u64>> = vec![BTreeSet::new(); bornes.len()];
    for (addr, decoded) in &décodées {
        let Some(cible) = decoded.as_ref().and_then(call_target) else {
            continue;
        };
        let rang = match bornes.binary_search(addr) {
            Ok(rang) => rang,
            Err(0) => continue, // avant la première fonction
            Err(suivant) => suivant - 1,
        };
        appels_par_fn[rang].insert(cible);
    }

    let mut out = Vec::new();
    for (rang, start) in bornes.iter().enumerate() {
        let fin = bornes.get(rang + 1).copied().unwrap_or(dernière + 1);
        out.push(Function {
            start: *start,
            last: fin.saturating_sub(1),
            callers: appels.get(start).map_or(0, BTreeSet::len),
            calls: std::mem::take(&mut appels_par_fn[rang])
                .into_iter()
                .collect(),
            origin: débuts[start],
        });
    }
    out
}

/// Les fonctions par lesquelles commencer, les plus appelées d'abord.
///
/// Dans un binaire qu'on ne connaît pas, l'ordre des adresses ne veut
/// rien dire. Ce qui compte, c'est ce qui sert : une fonction appelée de
/// quarante endroits est un utilitaire, et celle qui n'est appelée que
/// par `__libc_start_main` est le programme lui-même.
pub fn by_importance(functions: &[Function]) -> Vec<&Function> {
    let mut out: Vec<&Function> = functions.iter().collect();
    out.sort_by(|a, b| {
        // `main` en tête, quoi qu'il arrive : c'est par là qu'on lit.
        a.origin
            .cmp(&b.origin)
            .then(b.callers.cmp(&a.callers))
            .then(a.start.cmp(&b.start))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le `main` d'un programme dépouillé se retrouve par la convention
    /// de démarrage — c'est la première chose qu'on cherche.
    #[test]
    fn the_main_of_a_stripped_program_is_found_through_its_start() {
        // Le démarrage d'un programme C, tel que le produit un
        // compilateur : on met `main` dans RDI, puis on passe la main à
        // la bibliothèque.
        //   endbr64 / xor ebp,ebp / lea rdi,[rip+0x10] / call [rip+0x2000]
        let bytes: &[u8] = &[
            0xF3, 0x0F, 0x1E, 0xFA, // endbr64
            0x31, 0xED, // xor ebp, ebp
            0x48, 0x8D, 0x3D, 0x10, 0x00, 0x00, 0x00, // lea rdi, [rip+0x10]
            0xFF, 0x15, 0x00, 0x20, 0x00, 0x00, // call [rip+0x2000]
            0xF4, // hlt
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        // `lea` à 0x1006, longue de sept octets : la cible est 0x100d + 0x10.
        assert_eq!(main_of(&disasm, 0x1000), Some(0x101d));
    }

    /// Rien devant un démarrage qui ne passe rien : on ne devine pas.
    #[test]
    fn nothing_is_invented_when_the_convention_is_not_followed() {
        // xor ebp, ebp / call [rip+0] / hlt : aucun `lea rdi`.
        let bytes: &[u8] = &[0x31, 0xED, 0xFF, 0x15, 0x00, 0x00, 0x00, 0x00, 0xF4];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        assert_eq!(main_of(&disasm, 0x1000), None);
    }

    /// Toute adresse qu'un `call` vise est une fonction, et l'on compte
    /// combien d'appels y mènent.
    #[test]
    fn a_called_address_is_a_function_and_its_callers_are_counted() {
        // call +5 / call +0 / ret / ret
        //   0x1000: e8 05 00 00 00   call 0x100a
        //   0x1005: e8 00 00 00 00   call 0x100a
        //   0x100a: c3               ret
        let bytes: &[u8] = &[
            0xE8, 0x05, 0x00, 0x00, 0x00, 0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3,
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let fonctions = discover(&disasm, 0x1000);
        let appelée = fonctions
            .iter()
            .find(|f| f.start == 0x100a)
            .expect("la fonction appelée");
        assert_eq!(appelée.callers, 2, "deux appels y mènent");
        assert_eq!(appelée.origin, Origin::Called);
        assert_eq!(appelée.name(), "sub_100a");

        // Et le point d'entrée, qui appelle, est connu comme tel.
        let entrée = fonctions
            .iter()
            .find(|f| f.start == 0x1000)
            .expect("le point d'entrée");
        assert_eq!(entrée.origin, Origin::Entry);
        assert_eq!(entrée.calls, vec![0x100a]);
    }

    /// Découvrir les fonctions reste linéaire, même sur un gros binaire.
    ///
    /// La version d'avant rebalayait toutes les instructions pour CHAQUE
    /// fonction : quadratique, trois secondes sur trois mégaoctets, le
    /// chargement figeait l'interface. Ce contrôle fabrique vingt mille
    /// appels — donc autant de fonctions — et exige que `discover` finisse
    /// vite. En quadratique, ce serait quatre cents millions de passages,
    /// hors de portée de la borne ; en linéaire, quelques dizaines de
    /// millisecondes. La borne est large pour ne pas clignoter sur une CI
    /// lente, mais une régression quadratique la dépasse de loin.
    #[test]
    fn discovering_functions_stays_linear_on_a_big_binary() {
        const N: u32 = 20_000;
        // `e8` + rel32 : un `call` de cinq octets, une cible différente à
        // chaque fois, donc une fonction de plus à chaque fois.
        let mut bytes = Vec::with_capacity(N as usize * 5);
        for i in 0..N {
            bytes.push(0xE8);
            bytes.extend_from_slice(&(i as i32).to_le_bytes());
        }
        let disasm = namsdec_disasm::disassemble(&bytes, 0x1000);
        let début = std::time::Instant::now();
        let fonctions = discover(&disasm, 0x1000);
        let durée = début.elapsed();
        assert!(
            !fonctions.is_empty(),
            "des fonctions sont découvertes : {}",
            fonctions.len()
        );
        assert!(
            durée < std::time::Duration::from_secs(2),
            "discover a mis {durée:?} sur {N} appels — une régression quadratique ?"
        );
    }

    /// L'ordre de lecture n'est pas celui des adresses : `main` d'abord,
    /// puis ce qui sert le plus.
    #[test]
    fn the_reading_order_puts_main_first() {
        let bytes: &[u8] = &[
            0xE8, 0x05, 0x00, 0x00, 0x00, 0xE8, 0x00, 0x00, 0x00, 0x00, 0xC3,
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let fonctions = discover(&disasm, 0x1000);
        let ordre = by_importance(&fonctions);
        assert_eq!(ordre[0].origin, Origin::Entry, "l'entrée avant le reste");
        assert!(ordre.len() >= 2);
    }
}
