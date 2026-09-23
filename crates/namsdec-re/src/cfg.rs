//! Le déroulement d'un programme, sous forme de graphe.
//!
//! Un listing se lit de haut en bas ; un programme, non. Les sauts le
//! plient — une boucle remonte, un test ouvre deux chemins, un appel
//! part ailleurs et revient. Ce module découpe le code en **blocs de
//! base** et relie ces blocs : ce qu'on dessine ensuite est la forme
//! réelle du programme, celle qu'on reconstitue de tête en lisant, et
//! qu'un débutant ne voit pas encore.
//!
//! Un bloc de base est une suite d'instructions qu'on ne peut
//! qu'entrer par le haut et quitter par le bas : rien n'y saute au
//! milieu, rien n'en sort avant la fin. Les frontières sont donc les
//! cibles de saut, et les instructions qui rompent le fil.

use std::collections::{BTreeMap, BTreeSet};

use namsdec_disasm::{DecodedInstruction, FlowControl, Mnemonic};

/// Ce qui termine un bloc — et donc ce qui explique la suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// Rien de particulier : le bloc coule dans le suivant, parce que
    /// quelque chose saute au milieu du code plus bas.
    Fallthrough,
    /// Un saut inconditionnel.
    Jump,
    /// Un saut conditionnel : deux suites possibles.
    Branch,
    /// Un appel : la suite reprend juste après, une fois revenu.
    Call,
    /// Un retour.
    Return,
    /// `hlt`, un `syscall` de sortie, une instruction indéchiffrable :
    /// le fil s'arrête là.
    Stop,
    /// Un saut dont la cible se calcule — une table de sauts, un
    /// pointeur de fonction. On le dit plutôt que d'inventer une flèche.
    Indirect,
}

/// Un bloc de base : une suite d'instructions d'un seul tenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub start: u64,
    /// L'adresse de la dernière instruction du bloc — et non celle qui
    /// suit : c'est elle qu'on met en évidence quand RIP y est.
    pub last: u64,
    /// Le texte de chaque instruction, dans l'ordre.
    pub instructions: Vec<String>,
    pub exit: Exit,
}

impl Block {
    /// Le bloc contient-il cette adresse ?
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr <= self.last
    }
}

/// Le lien entre deux blocs, et ce qu'il veut dire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Le fil ordinaire : l'instruction suivante.
    Next,
    /// Le saut est pris.
    Taken,
    /// Le saut n'est pas pris — l'autre branche d'un test.
    NotTaken,
    /// L'appel d'une sous-routine.
    Call,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: Kind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cfg {
    pub blocks: Vec<Block>,
    pub edges: Vec<Edge>,
}

impl Cfg {
    /// Le bloc qui contient cette adresse.
    pub fn block_at(&self, addr: u64) -> Option<usize> {
        self.blocks.iter().position(|b| b.contains(addr))
    }

    /// Les blocs, rangés par couches : l'entrée au niveau zéro, puis
    /// ceux qu'on atteint en un saut, en deux…
    ///
    /// C'est ce qui donne au dessin sa forme. Un parcours en largeur
    /// suffit : il place chaque bloc au plus court chemin depuis
    /// l'entrée, et les arêtes qui remontent — les boucles — se voient
    /// alors comme telles.
    /// Le graphe en DOT, tel que Graphviz le lit.
    ///
    /// De quoi le mettre dans une note, un rapport ou un cours : `dot -Tsvg`
    /// en fait une image, et le format se lit à l'œil sans outil. Les
    /// couleurs reprennent celles du dessin — vert pour un saut pris,
    /// rouge pour un saut évité, bleu pour un appel.
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph flux {\n");
        dot.push_str("  node [shape=box fontname=\"monospace\" fontsize=10];\n");
        dot.push_str("  edge [fontname=\"monospace\" fontsize=9];\n");
        for (index, bloc) in self.blocks.iter().enumerate() {
            // Le texte du bloc : son adresse, puis ses instructions, une
            // par ligne — `\l` aligne à gauche dans Graphviz.
            let mut étiquette = format!("{:#010x}\\l", bloc.start);
            for instruction in &bloc.instructions {
                étiquette.push_str(&instruction.replace('\\', " ").replace('"', "'"));
                étiquette.push_str("\\l");
            }
            dot.push_str(&format!("  b{index} [label=\"{étiquette}\"];\n"));
        }
        for edge in &self.edges {
            let couleur = match edge.kind {
                Kind::Taken => "darkgreen",
                Kind::NotTaken => "firebrick",
                Kind::Call => "steelblue",
                Kind::Next => "gray40",
            };
            dot.push_str(&format!(
                "  b{} -> b{} [color={couleur}];\n",
                edge.from, edge.to
            ));
        }
        dot.push_str("}\n");
        dot
    }

    /// Les blocs de la fonction à laquelle celui-ci appartient.
    ///
    /// On part du bloc donné et l'on avance dans les deux sens, sans
    /// jamais franchir un `call` : un appel mène ailleurs, et c'est
    /// précisément ce qu'on veut laisser dehors. Ce qui reste est ce
    /// qu'on atteint par des sauts — la fonction telle qu'on la lit.
    ///
    /// Sans symboles pour dire où chaque fonction commence, cette
    /// fermeture est ce qui s'en approche le mieux : elle ne se trompe
    /// que là où deux fonctions partagent un bloc, ce qu'un compilateur
    /// ne fait guère que sur du code écrit à la main.
    pub fn function_around(&self, block: usize) -> std::collections::BTreeSet<usize> {
        let mut vus = std::collections::BTreeSet::new();
        if block >= self.blocks.len() {
            return vus;
        }
        let mut file = std::collections::VecDeque::from([block]);
        vus.insert(block);
        while let Some(index) = file.pop_front() {
            let voisins = self
                .edges
                .iter()
                .filter(|edge| edge.kind != Kind::Call)
                .filter_map(|edge| {
                    if edge.from == index {
                        Some(edge.to)
                    } else if edge.to == index {
                        Some(edge.from)
                    } else {
                        None
                    }
                });
            for voisin in voisins.collect::<Vec<_>>() {
                if vus.insert(voisin) {
                    file.push_back(voisin);
                }
            }
        }
        vus
    }

    /// Le graphe réduit à ces blocs, et aux liens qui restent entre eux.
    ///
    /// Un vrai `Cfg` plutôt qu'un filtre passé au dessin : tout ce qui
    /// sait dessiner un graphe sait alors dessiner celui-ci, sans rien
    /// connaître de la restriction.
    pub fn subgraph(&self, keep: &std::collections::BTreeSet<usize>) -> Self {
        let mut rang = std::collections::BTreeMap::new();
        let mut blocks = Vec::with_capacity(keep.len());
        for index in keep {
            let Some(bloc) = self.blocks.get(*index) else {
                continue;
            };
            rang.insert(*index, blocks.len());
            blocks.push(bloc.clone());
        }
        let edges = self
            .edges
            .iter()
            .filter_map(|edge| {
                Some(Edge {
                    from: *rang.get(&edge.from)?,
                    to: *rang.get(&edge.to)?,
                    kind: edge.kind,
                })
            })
            .collect();
        Self { blocks, edges }
    }

    pub fn layers(&self) -> Vec<usize> {
        let mut niveau = vec![usize::MAX; self.blocks.len()];
        if self.blocks.is_empty() {
            return niveau;
        }
        let mut file = std::collections::VecDeque::new();
        niveau[0] = 0;
        file.push_back(0usize);
        while let Some(index) = file.pop_front() {
            for edge in self.edges.iter().filter(|e| e.from == index) {
                if niveau[edge.to] == usize::MAX {
                    niveau[edge.to] = niveau[index] + 1;
                    file.push_back(edge.to);
                }
            }
        }
        // Ce qu'aucun chemin n'atteint — du code mort, une fonction
        // qu'on n'appelle que par pointeur — se range à la suite plutôt
        // que de disparaître du dessin.
        let plus_bas = niveau.iter().filter(|n| **n != usize::MAX).max().copied();
        for n in niveau.iter_mut() {
            if *n == usize::MAX {
                *n = plus_bas.map_or(0, |max| max + 1);
            }
        }
        niveau
    }
}

/// La cible d'un saut ou d'un appel, quand elle est écrite en clair.
fn target(instr: &namsdec_disasm::Decoded) -> Option<u64> {
    use namsdec_disasm::OpKind;
    match instr.op0_kind() {
        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
            Some(instr.near_branch_target())
        }
        _ => None,
    }
}

/// Ce que cette instruction fait au fil du programme.
fn exit_of(instr: &namsdec_disasm::Decoded) -> Option<Exit> {
    // `hlt` s'arrête ; `syscall` continue — sauf `exit`, mais le numéro
    // est dans RAX, et deviner ici mentirait.
    if instr.mnemonic() == Mnemonic::Hlt {
        return Some(Exit::Stop);
    }
    match instr.flow_control() {
        FlowControl::UnconditionalBranch => Some(if target(instr).is_some() {
            Exit::Jump
        } else {
            Exit::Indirect
        }),
        FlowControl::ConditionalBranch => Some(Exit::Branch),
        FlowControl::Call => Some(if target(instr).is_some() {
            Exit::Call
        } else {
            Exit::Indirect
        }),
        FlowControl::Return => Some(Exit::Return),
        FlowControl::Interrupt | FlowControl::Exception => Some(Exit::Stop),
        FlowControl::IndirectBranch => Some(Exit::Indirect),
        FlowControl::IndirectCall => Some(Exit::Indirect),
        _ => None,
    }
}

/// Découpe un désassemblage en blocs de base, et relie les blocs.
///
/// Deux passages : le premier relève les frontières — les cibles de
/// saut, et ce qui suit une rupture —, le second pose les blocs et les
/// flèches. On ne peut pas faire autrement : une boucle saute en
/// arrière, donc vers une frontière qu'on ne connaît pas encore quand
/// on la rencontre.
pub fn build(disasm: &[DecodedInstruction]) -> Cfg {
    if disasm.is_empty() {
        return Cfg::default();
    }
    let décodées: Vec<(u64, Option<namsdec_disasm::Decoded>, &str)> = disasm
        .iter()
        .map(|instr| {
            (
                instr.address,
                namsdec_disasm::decode_one(&instr.bytes, instr.address),
                instr.text.as_str(),
            )
        })
        .collect();

    // Premier passage : où commencent les blocs.
    let mut frontières: BTreeSet<u64> = BTreeSet::new();
    frontières.insert(décodées[0].0);
    for (index, (addr, decoded, _)) in décodées.iter().enumerate() {
        let Some(decoded) = decoded else { continue };
        let Some(exit) = exit_of(decoded) else {
            continue;
        };
        if let Some(cible) = target(decoded) {
            frontières.insert(cible);
        }
        // Ce qui suit une rupture commence un bloc, quoi qu'il arrive.
        let _ = (addr, exit);
        if let Some((suivante, _, _)) = décodées.get(index + 1) {
            frontières.insert(*suivante);
        }
    }

    // Second passage : poser les blocs.
    let mut blocks: Vec<Block> = Vec::new();
    let mut par_adresse: BTreeMap<u64, usize> = BTreeMap::new();
    let mut courant: Option<Block> = None;
    let mut sorties: Vec<(usize, Option<u64>, Option<u64>)> = Vec::new();
    for (index, (addr, decoded, texte)) in décodées.iter().enumerate() {
        if frontières.contains(addr)
            && let Some(bloc) = courant.take()
        {
            par_adresse.insert(bloc.start, blocks.len());
            blocks.push(bloc);
        }
        let bloc = courant.get_or_insert(Block {
            start: *addr,
            last: *addr,
            instructions: Vec::new(),
            exit: Exit::Fallthrough,
        });
        bloc.last = *addr;
        bloc.instructions.push((*texte).to_owned());

        let exit = decoded.as_ref().and_then(exit_of);
        if let Some(exit) = exit {
            bloc.exit = exit;
            let cible = decoded.as_ref().and_then(target);
            let suite = décodées.get(index + 1).map(|(a, _, _)| *a);
            let bloc = courant.take().expect("le bloc courant existe");
            par_adresse.insert(bloc.start, blocks.len());
            sorties.push((blocks.len(), cible, suite));
            blocks.push(bloc);
        }
    }
    if let Some(bloc) = courant.take() {
        // Le dernier bloc n'a pas été fermé par une rupture : il coule
        // vers ce qui suit, et il n'y a rien. C'est un `hlt` manquant.
        par_adresse.insert(bloc.start, blocks.len());
        sorties.push((blocks.len(), None, None));
        blocks.push(bloc);
    }

    // Les blocs qui n'ont pas été fermés par une rupture coulent dans le
    // suivant : on les relève maintenant, l'ordre des blocs étant connu.
    let mut edges = Vec::new();
    let fermés: BTreeSet<usize> = sorties.iter().map(|(index, _, _)| *index).collect();
    for index in 0..blocks.len() {
        if !fermés.contains(&index) && index + 1 < blocks.len() {
            edges.push(Edge {
                from: index,
                to: index + 1,
                kind: Kind::Next,
            });
        }
    }
    for (index, cible, suite) in sorties {
        let exit = blocks[index].exit;
        if let Some(cible) = cible
            && let Some(&vers) = par_adresse.get(&cible)
        {
            edges.push(Edge {
                from: index,
                to: vers,
                kind: match exit {
                    Exit::Call => Kind::Call,
                    Exit::Branch => Kind::Taken,
                    _ => Kind::Next,
                },
            });
        }
        // Ce qui reprend après : la branche non prise d'un test, ou le
        // retour d'un appel. Un saut inconditionnel, un arrêt et un
        // retour n'ont pas de suite.
        let reprend = matches!(
            exit,
            Exit::Branch | Exit::Call | Exit::Fallthrough | Exit::Indirect
        );
        if reprend
            && let Some(suite) = suite
            && let Some(&vers) = par_adresse.get(&suite)
        {
            edges.push(Edge {
                from: index,
                to: vers,
                kind: if exit == Exit::Branch {
                    Kind::NotTaken
                } else {
                    Kind::Next
                },
            });
        }
    }
    edges.sort_by_key(|e| (e.from, e.to));
    edges.dedup();
    Cfg { blocks, edges }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_of(bytes: &[u8]) -> Cfg {
        build(&namsdec_disasm::disassemble(bytes, 0x400000))
    }

    /// Une boucle se voit : un bloc s'y jette en arrière.
    #[test]
    fn a_loop_shows_as_an_edge_going_back() {
        // mov ecx, 3 / dec ecx / jnz vers le `dec` / hlt
        let bytes: &[u8] = &[0xB9, 0x03, 0x00, 0x00, 0x00, 0xFF, 0xC9, 0x75, 0xFC, 0xF4];
        let cfg = cfg_of(bytes);
        assert_eq!(cfg.blocks.len(), 3, "{:?}", cfg.blocks);

        // Le corps de la boucle se termine par un saut conditionnel.
        let corps = cfg
            .blocks
            .iter()
            .position(|b| b.exit == Exit::Branch)
            .expect("un bloc se termine par un test");
        let retour = cfg
            .edges
            .iter()
            .find(|e| e.from == corps && e.kind == Kind::Taken)
            .expect("le saut pris");
        assert!(
            cfg.blocks[retour.to].start <= cfg.blocks[corps].start,
            "une boucle remonte : {:?}",
            cfg.blocks
        );
        // Et l'autre branche continue vers la suite.
        assert!(
            cfg.edges
                .iter()
                .any(|e| e.from == corps && e.kind == Kind::NotTaken)
        );
    }

    /// Un appel part ailleurs **et** revient : deux flèches, pas une.
    #[test]
    fn a_call_goes_away_and_comes_back() {
        // call +1 / hlt / ret
        let bytes: &[u8] = &[0xE8, 0x01, 0x00, 0x00, 0x00, 0xF4, 0xC3];
        let cfg = cfg_of(bytes);
        let appelant = cfg
            .blocks
            .iter()
            .position(|b| b.exit == Exit::Call)
            .expect("un bloc appelle");
        assert!(
            cfg.edges
                .iter()
                .any(|e| e.from == appelant && e.kind == Kind::Call),
            "la flèche de l'appel"
        );
        assert!(
            cfg.edges
                .iter()
                .any(|e| e.from == appelant && e.kind == Kind::Next),
            "et celle du retour"
        );
        assert!(
            cfg.blocks.iter().any(|b| b.exit == Exit::Return),
            "la sous-routine se termine par un retour"
        );
    }

    /// Les couches placent l'entrée en haut et ce qui suit en dessous.
    #[test]
    fn the_layers_start_at_the_entry() {
        let bytes: &[u8] = &[0xB9, 0x03, 0x00, 0x00, 0x00, 0xFF, 0xC9, 0x75, 0xFB, 0xF4];
        let cfg = cfg_of(bytes);
        let niveaux = cfg.layers();
        assert_eq!(niveaux[0], 0, "l'entrée est en haut");
        assert!(
            niveaux.iter().all(|n| *n != usize::MAX),
            "aucun bloc ne reste sans place : {niveaux:?}"
        );
        assert!(
            niveaux[1..].iter().all(|n| *n > 0),
            "et rien d'autre ne partage le niveau de l'entrée : {niveaux:?}"
        );
    }

    /// Un saut dont la cible se calcule est dit tel quel : on ne dessine
    /// pas une flèche vers une adresse qu'on ne connaît pas.
    #[test]
    fn an_indirect_jump_is_named_not_guessed() {
        // jmp rax / hlt
        let bytes: &[u8] = &[0xFF, 0xE0, 0xF4];
        let cfg = cfg_of(bytes);
        assert_eq!(cfg.blocks[0].exit, Exit::Indirect);
        assert!(
            !cfg.edges
                .iter()
                .any(|e| e.from == 0 && e.kind == Kind::Taken),
            "aucune flèche inventée"
        );
    }

    /// Rien à désassembler : un graphe vide, et non une panique.
    #[test]
    fn nothing_gives_an_empty_graph() {
        assert_eq!(build(&[]), Cfg::default());
    }
}


#[cfg(test)]
mod tests_fonction {
    use super::*;

    /// La fonction s'arrête au `call`, et le graphe réduit tient debout.
    ///
    /// Sur un binaire ordinaire le graphe entier compte des milliers de
    /// blocs, où l'on ne trouve rien : la vue n'est utilisable qu'une
    /// fonction à la fois. Ce qu'on garde est ce qu'on atteint par des
    /// sauts ; un appel mène ailleurs, et reste dehors.
    #[test]
    fn a_function_stops_at_its_calls() {
        let cfg = Cfg {
            blocks: vec![
                Block { start: 0x10, last: 0x12, instructions: vec!["jmp".into()], exit: Exit::Jump },
                Block { start: 0x20, last: 0x22, instructions: vec!["call".into()], exit: Exit::Call },
                // Le corps de l'appelée : un autre monde.
                Block { start: 0x90, last: 0x92, instructions: vec!["ret".into()], exit: Exit::Return },
            ],
            edges: vec![
                Edge { from: 0, to: 1, kind: Kind::Next },
                Edge { from: 1, to: 2, kind: Kind::Call },
            ],
        };

        let fonction = cfg.function_around(0);
        assert!(fonction.contains(&0) && fonction.contains(&1));
        assert!(
            !fonction.contains(&2),
            "ce qu'on atteint par un call n'est pas de cette fonction : {fonction:?}"
        );

        // Et depuis l'appelée, on ne remonte pas non plus vers l'appelant.
        assert_eq!(cfg.function_around(2), [2].into_iter().collect());

        // Le sous-graphe renumérote ses blocs et ne garde que les liens
        // dont les deux bouts restent.
        let réduit = cfg.subgraph(&fonction);
        assert_eq!(réduit.blocks.len(), 2);
        assert_eq!(réduit.blocks[0].start, 0x10);
        assert_eq!(réduit.edges.len(), 1, "le call n'a plus de cible : {:?}", réduit.edges);
        assert_eq!((réduit.edges[0].from, réduit.edges[0].to), (0, 1));
    }

    /// Le graphe s'emporte au format DOT, et ce DOT tient debout.
    #[test]
    fn the_graph_can_be_carried_away_as_dot() {
        let cfg = Cfg {
            blocks: vec![
                Block {
                    start: 0x10,
                    last: 0x12,
                    instructions: vec!["cmp rax, 1".into(), "je 0x20".into()],
                    exit: Exit::Jump,
                },
                Block {
                    start: 0x20,
                    last: 0x20,
                    instructions: vec!["ret".into()],
                    exit: Exit::Return,
                },
            ],
            edges: vec![Edge {
                from: 0,
                to: 1,
                kind: Kind::Taken,
            }],
        };
        let dot = cfg.to_dot();
        assert!(dot.starts_with("digraph flux {"), "{dot}");
        assert!(dot.trim_end().ends_with('}'), "{dot}");
        // Chaque bloc a son nœud, chaque lien son arête.
        assert!(dot.contains("b0 [label="), "{dot}");
        assert!(dot.contains("b1 [label="), "{dot}");
        assert!(dot.contains("b0 -> b1"), "{dot}");
        // L'adresse et les instructions s'y lisent.
        assert!(dot.contains("0x00000010"), "{dot}");
        assert!(dot.contains("cmp rax, 1"), "{dot}");
        // La couleur dit ce que le lien veut dire.
        assert!(dot.contains("color=darkgreen"), "{dot}");
        // Et rien n'y casse la syntaxe : les guillemets d'une
        // instruction deviennent des apostrophes.
        let piégé = Cfg {
            blocks: vec![Block {
                start: 0,
                last: 0,
                instructions: vec!["mov rax, \"x\"".into()],
                exit: Exit::Return,
            }],
            edges: Vec::new(),
        };
        let dot = piégé.to_dot();
        assert_eq!(
            dot.matches('"').count() % 2,
            0,
            "les guillemets doivent rester appariés : {dot}"
        );
    }

    /// Un bloc hors du graphe ne fait pas tomber la fermeture.
    #[test]
    fn an_unknown_block_yields_nothing() {
        let cfg = Cfg::default();
        assert!(cfg.function_around(7).is_empty());
        assert!(cfg.subgraph(&[3usize].into_iter().collect()).blocks.is_empty());
    }
}
