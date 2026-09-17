//! Comment arrive-t-on là ?
//!
//! Devant un programme qui refuse un mot de passe, on connaît vite
//! l'adresse du succès — la chaîne « Correct » est dans les données, et
//! quelque chose la charge. Reste la vraie question : **par quel chemin
//! y parvient-on, et à quelles conditions ?**
//!
//! Ce module la traite comme un problème de graphe. Le déroulement donne
//! les blocs et leurs liens ; il suffit de chercher le plus court chemin
//! de l'entrée jusqu'au bloc visé, puis de dire, pour chaque pas, ce
//! qu'il a fallu satisfaire : une comparaison réussie, un test négatif,
//! un saut pris ou évité.
//!
//! Le résultat n'est pas une preuve. C'est **une** route, la plus
//! courte ; une autre peut exister. namsdec le dit plutôt que de laisser
//! croire à une démonstration.

use std::collections::VecDeque;

use crate::cfg::{Cfg, Exit, Kind};

/// Un pas de la route : un bloc, et ce qu'il a fallu pour en sortir du
/// bon côté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    /// Le bloc d'où l'on part.
    pub block: usize,
    pub addr: u64,
    /// La dernière instruction du bloc — celle qui décide.
    pub decides: String,
    /// La condition à remplir pour continuer sur cette route.
    pub condition: Condition,
}

/// Ce qu'il faut pour passer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// Rien à faire : le fil coule vers le bloc suivant.
    Straight,
    /// Le saut doit être pris.
    Taken,
    /// Le saut ne doit **pas** être pris.
    NotTaken,
    /// Un appel : on y passe et l'on en revient.
    Through,
}

impl Condition {
    pub fn explain(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match self {
            Condition::Straight if fr => "rien à satisfaire : le fil continue",
            Condition::Straight => "nothing to satisfy: the thread continues",
            Condition::Taken if fr => "le saut doit être pris",
            Condition::Taken => "the branch must be taken",
            Condition::NotTaken if fr => "le saut ne doit pas être pris",
            Condition::NotTaken => "the branch must not be taken",
            Condition::Through if fr => "on passe par l'appel, et l'on en revient",
            Condition::Through => "we go through the call, and come back",
        }
    }
}

/// La route trouvée jusqu'à une adresse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub stages: Vec<Stage>,
    /// Le bloc d'arrivée.
    pub target: usize,
    /// Y avait-il d'autres chemins ? On le dit sans les énumérer :
    /// « la plus courte » n'est pas « la seule ».
    pub others: bool,
}

/// Cherche comment atteindre le bloc qui contient `addr`, depuis
/// l'entrée du graphe.
///
/// Un parcours en largeur : il donne le chemin le plus court en nombre
/// de blocs, ce qui est exactement ce qu'on veut lire en premier — le
/// chemin le plus direct, celui qui porte le moins de conditions.
pub fn to(cfg: &Cfg, addr: u64) -> Option<Route> {
    let cible = cfg.block_at(addr)?;
    if cfg.blocks.is_empty() {
        return None;
    }
    // D'où l'on vient, pour chaque bloc atteint.
    let mut venu_de: Vec<Option<(usize, Kind)>> = vec![None; cfg.blocks.len()];
    let mut vu = vec![false; cfg.blocks.len()];
    let mut file = VecDeque::new();
    vu[0] = true;
    file.push_back(0usize);
    let mut atteint = cible == 0;
    let mut arrivées = 0usize;

    while let Some(index) = file.pop_front() {
        for edge in cfg.edges.iter().filter(|e| e.from == index) {
            if edge.to == cible {
                arrivées += 1;
            }
            if vu[edge.to] {
                continue;
            }
            vu[edge.to] = true;
            venu_de[edge.to] = Some((index, edge.kind));
            if edge.to == cible {
                atteint = true;
            }
            file.push_back(edge.to);
        }
    }
    if !atteint {
        return None;
    }

    // Remonter la route, puis la remettre à l'endroit.
    let mut stages = Vec::new();
    let mut courant = cible;
    while let Some((précédent, kind)) = venu_de[courant] {
        let bloc = &cfg.blocks[précédent];
        stages.push(Stage {
            block: précédent,
            addr: bloc.start,
            decides: bloc.instructions.last().cloned().unwrap_or_default(),
            condition: match (bloc.exit, kind) {
                (Exit::Branch, Kind::Taken) => Condition::Taken,
                (Exit::Branch, Kind::NotTaken) => Condition::NotTaken,
                (Exit::Call, _) | (_, Kind::Call) => Condition::Through,
                _ => Condition::Straight,
            },
        });
        courant = précédent;
    }
    stages.reverse();
    Some(Route {
        stages,
        target: cible,
        others: arrivées > 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La route dit ce qu'il faut satisfaire, et dans quel ordre.
    #[test]
    fn the_route_says_what_has_to_hold() {
        // cmp esi, 7 ; jne +5 ; nop ; nop ; hlt ; hlt
        //   0x1000: 83 fe 07        cmp esi, 7
        //   0x1003: 75 02           jne 0x1007
        //   0x1005: 90 90           nop nop
        //   0x1007: f4              hlt
        let bytes: &[u8] = &[0x83, 0xFE, 0x07, 0x75, 0x02, 0x90, 0x90, 0xF4];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cfg = crate::cfg::build(&disasm);

        // Pour atteindre 0x1005 — la suite après le test —, il faut que
        // le saut ne soit **pas** pris.
        let route = to(&cfg, 0x1005).expect("une route existe");
        assert_eq!(route.stages.len(), 1, "{:?}", route.stages);
        assert_eq!(route.stages[0].condition, Condition::NotTaken);
        assert!(
            route.stages[0].decides.starts_with("jne"),
            "{:?}",
            route.stages[0]
        );

        // Et pour atteindre 0x1007, il faut qu'il le soit — ou qu'on
        // arrive par le fil d'à côté : namsdec dit qu'il y a plusieurs
        // chemins plutôt que de prétendre en connaître un seul.
        let route = to(&cfg, 0x1007).expect("une route existe");
        assert!(route.others, "plusieurs chemins mènent à la sortie");
    }

    /// Une adresse qu'aucun chemin n'atteint n'a pas de route, et l'on
    /// ne l'invente pas.
    #[test]
    fn an_unreachable_address_has_no_route() {
        let bytes: &[u8] = &[0xF4, 0x90]; // hlt ; nop
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cfg = crate::cfg::build(&disasm);
        assert!(to(&cfg, 0x1001).is_none());
        // Et une adresse hors du graphe non plus.
        assert!(to(&cfg, 0xdead).is_none());
    }
}
