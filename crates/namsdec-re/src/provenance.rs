//! D'où vient cette valeur ?
//!
//! C'est la question qu'on pose devant chaque comparaison d'un programme
//! qu'on ne connaît pas. `cmp rsi, 7` ne dit rien tant qu'on ignore ce
//! que RSI porte — et pour le savoir, il faut remonter le code jusqu'à
//! la dernière écriture de RSI, puis recommencer avec ce qui l'a
//! alimentée.
//!
//! On le fait à la main, en lisant à l'envers, et c'est fastidieux. Ce
//! module le fait pour vous, en s'arrêtant là où l'honnêteté l'exige :
//! au début du bloc de base. Plus haut, plusieurs chemins mènent ici, et
//! « la dernière écriture » n'a plus de sens unique — ce serait deviner.

use namsdec_disasm::{DecodedInstruction, OpKind, Register};

/// Un maillon de la provenance : une instruction, et ce qu'elle a écrit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub addr: u64,
    /// Le texte de l'instruction, tel qu'il se lit.
    pub text: String,
    /// Le registre qu'elle écrit, en minuscules.
    pub writes: String,
    /// Ce dont elle dépend, et qu'on va remonter ensuite.
    pub reads: Vec<String>,
    /// L'emplacement mémoire traversé, quand la valeur y a transité.
    ///
    /// `[rbp-8]` tel qu'on l'écrirait : de quoi dire « passé par la
    /// pile » plutôt que de faire croire à une chaîne de registres.
    pub through: Option<String>,
}

/// Jusqu'où l'on remonte, en instructions.
///
/// Au-delà, on est presque sûrement sorti du bloc — et l'on aurait
/// traversé un saut sans le savoir.
const REMONTÉE: usize = 64;

/// Cette instruction lit-elle la mémoire ?
///
/// La question décide de ce qu'on peut promettre. Une valeur qui vient
/// d'un registre se remonte de proche en proche ; une valeur qui vient
/// d'un `[rbp-8]` vient de la **mémoire**, et remonter le registre de
/// base mènerait à la mise en place du cadre de pile — pas à la valeur.
/// Mieux vaut s'arrêter là en le disant que de conduire ailleurs avec
/// assurance.
pub fn reads_memory(instr: &namsdec_disasm::Decoded) -> bool {
    (0..instr.op_count()).any(|index| matches!(instr.op_kind(index), OpKind::Memory))
}

/// Les registres que cette instruction lit, et celui qu'elle écrit.
pub fn registers(instr: &namsdec_disasm::Decoded) -> (Vec<Register>, Option<Register>) {
    lit_écrit(instr)
}

/// Les registres que cette instruction lit, et celui qu'elle écrit.
fn lit_écrit(instr: &namsdec_disasm::Decoded) -> (Vec<Register>, Option<Register>) {
    let mut lus = Vec::new();
    let mut écrit = None;
    for index in 0..instr.op_count() {
        match instr.op_kind(index) {
            OpKind::Register => {
                let reg = instr.op_register(index);
                // Le premier opérande d'une instruction à deux
                // opérandes est la destination — sauf pour `cmp` et
                // `test`, qui ne font que lire.
                let destination = index == 0
                    && !matches!(
                        instr.mnemonic(),
                        namsdec_disasm::Mnemonic::Cmp | namsdec_disasm::Mnemonic::Test
                    );
                if destination {
                    écrit = Some(reg.full_register());
                    // `add rax, rcx` lit aussi RAX : seule une écriture
                    // pure — `mov` — ne lit pas sa destination.
                    if instr.mnemonic() != namsdec_disasm::Mnemonic::Mov {
                        lus.push(reg.full_register());
                    }
                } else {
                    lus.push(reg.full_register());
                }
            }
            OpKind::Memory => {
                // Une lecture en mémoire dépend des registres de
                // l'adresse : c'est par eux qu'on remonte.
                for reg in [instr.memory_base(), instr.memory_index()] {
                    if reg != Register::None {
                        lus.push(reg.full_register());
                    }
                }
            }
            _ => {}
        }
    }
    lus.sort_by_key(|r| format!("{r:?}"));
    lus.dedup();
    (lus, écrit)
}

/// Un emplacement mémoire, tel que l'instruction le décrit.
///
/// Gardé en pièces détachées plutôt qu'en texte : reconnaître la même
/// case d'un bout à l'autre demande de comparer les composants, et
/// PROUVER que deux cases sont distinctes demande en plus leur taille.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Case {
    base: Register,
    index: Register,
    échelle: u32,
    déplacement: i64,
    /// En octets : deux cases voisines ne se recouvrent que si leurs
    /// intervalles se croisent.
    taille: i64,
}

impl Case {
    /// Le nom qu'on en donne à l'utilisateur : `[RBP-8]`.
    fn nom(&self) -> String {
        let base = if self.base == Register::None {
            String::new()
        } else {
            format!("{:?}", self.base).to_uppercase()
        };
        let index = if self.index == Register::None {
            String::new()
        } else {
            format!("+{:?}*{}", self.index, self.échelle).to_uppercase()
        };
        format!("[{base}{index}{:+}]", self.déplacement)
    }

    /// Peut-on PROUVER que ces deux cases ne se touchent pas ?
    ///
    /// Seulement quand elles se lisent depuis le même point — même base,
    /// même index, même échelle — et que leurs intervalles d'octets ne
    /// se croisent pas. Des bases différentes ne prouvent rien : `[rax]`
    /// vaut peut-être `[rbp-8]`, et rien dans le code ne le dit.
    fn sûrement_distincte(&self, autre: &Case) -> bool {
        if self.base != autre.base || self.index != autre.index || self.échelle != autre.échelle {
            return false;
        }
        let fin = self.déplacement + self.taille;
        let fin_autre = autre.déplacement + autre.taille;
        fin <= autre.déplacement || fin_autre <= self.déplacement
    }
}

/// L'emplacement mémoire que cette instruction touche.
///
/// `None` quand elle ne touche pas la mémoire.
fn emplacement(instr: &namsdec_disasm::Decoded) -> Option<Case> {
    if !(0..instr.op_count()).any(|i| instr.op_kind(i) == OpKind::Memory) {
        return None;
    }
    // Sans base ni index, l'adresse est absolue : elle se nomme aussi,
    // et c'est même le cas le plus sûr — rien ne peut la déplacer.
    Some(Case {
        base: instr.memory_base(),
        index: instr.memory_index(),
        échelle: instr.memory_index_scale(),
        déplacement: instr.memory_displacement64() as i64,
        // Une taille inconnue vaut zéro : deux cases de taille nulle ne
        // se recouvrent jamais, donc `sûrement_distincte` deviendrait
        // trop généreuse. Un octet est le minimum honnête.
        taille: (instr.memory_size().size() as i64).max(1),
    })
}

/// Cette instruction écrit-elle en mémoire ?
///
/// Le premier opérande mémoire d'une instruction qui affecte est sa
/// destination. `cmp` et `test` ne font que lire, et `push` écrit un
/// emplacement qu'on ne suit pas ici.
fn écrit_en_mémoire(instr: &namsdec_disasm::Decoded) -> bool {
    use namsdec_disasm::Mnemonic as M;
    if matches!(instr.mnemonic(), M::Cmp | M::Test) {
        return false;
    }
    instr.op_count() > 0 && instr.op_kind(0) == OpKind::Memory
}

/// Remonte la provenance du registre `reg` depuis l'instruction à
/// `from`, dans la limite du bloc.
///
/// Rend les maillons du plus proche au plus lointain : la dernière
/// écriture d'abord, puis ce qui l'a alimentée. Un maillon par écriture,
/// et l'on s'arrête dès qu'on rencontre un saut, un appel ou une cible
/// de saut — au-delà, plusieurs chemins mènent ici.
pub fn trace(disasm: &[DecodedInstruction], from: u64, reg: &str) -> Vec<Step> {
    let Some(départ) = disasm.iter().position(|i| i.address == from) else {
        return Vec::new();
    };
    let cible = reg.to_ascii_lowercase();
    // Les registres qu'on cherche encore : on part d'un, et chaque
    // écriture trouvée le remplace par ce dont elle dépend.
    let mut recherchés: Vec<String> = vec![cible];
    // Les emplacements mémoire dont on cherche l'écriture. C'est la
    // moitié qui manquait : une valeur rangée sur la pile puis relue
    // arrêtait net la remontée, alors que c'est le chemin ordinaire de
    // tout argument dans du code compilé sans optimisation.
    let mut emplacements: Vec<Case> = Vec::new();
    let mut out = Vec::new();

    for instr in disasm[..départ].iter().rev().take(REMONTÉE) {
        let Some(decoded) = namsdec_disasm::decode_one(&instr.bytes, instr.address) else {
            continue;
        };
        // Un saut ou un appel ferme la remontée : ce qui précède n'est
        // pas forcément ce qui a été exécuté.
        use namsdec_disasm::FlowControl;
        if !matches!(
            decoded.flow_control(),
            FlowControl::Next | FlowControl::ConditionalBranch
        ) {
            break;
        }
        // Une écriture EN MÉMOIRE : soit elle remplit l'emplacement
        // qu'on cherche, soit elle pourrait l'écraser sans qu'on le
        // sache.
        if écrit_en_mémoire(&decoded) {
            let ici = emplacement(&decoded);
            match &ici {
                Some(case) if emplacements.contains(case) => {
                    let (lus, _) = lit_écrit(&decoded);
                    // Ce qui alimente la case : les registres LUS par
                    // cette écriture, sauf ceux qui ne servent qu'à
                    // calculer l'adresse — ils disent où, non quoi.
                    let adresse: Vec<Register> = [decoded.memory_base(), decoded.memory_index()]
                        .into_iter()
                        .filter(|r| *r != Register::None)
                        .map(|r| r.full_register())
                        .collect();
                    let dépendances: Vec<String> = lus
                        .iter()
                        .filter(|r| !adresse.contains(r))
                        .map(|r| format!("{r:?}").to_ascii_lowercase())
                        .collect();
                    out.push(Step {
                        addr: instr.address,
                        text: instr.text.clone(),
                        writes: case.nom(),
                        reads: dépendances.clone(),
                        through: Some(case.nom()),
                    });
                    emplacements.retain(|e| e != case);
                    for dépendance in dépendances {
                        if !recherchés.contains(&dépendance) {
                            recherchés.push(dépendance);
                        }
                    }
                }
                // Une écriture ailleurs. Deux cas, et la différence
                // est tout ce qui rend la traversée utilisable :
                //
                //   - on PROUVE qu'elle ne touche pas la nôtre —
                //     `[rbp-16]` quand on suit `[rbp-8]` — et l'on
                //     continue. Sans cela, le premier cadre de pile
                //     venu couperait la piste, puisqu'on y range
                //     toujours plusieurs choses ;
                //   - on ne prouve rien — `[rax]` vaut peut-être
                //     `[rbp-8]`, rien dans le code ne le dit — et l'on
                //     abandonne les emplacements. Une chaîne qu'une
                //     écriture invisible aurait coupée se lirait bien
                //     et serait fausse.
                //
                // Les registres, eux, restent sûrs : rien ne les aliase.
                Some(ailleurs) => {
                    emplacements.retain(|cherché| ailleurs.sûrement_distincte(cherché));
                }
                // Inatteignable : `écrit_en_mémoire` a déjà vu un
                // opérande mémoire, donc `emplacement` en rend un. La
                // branche existait, et l'épreuve du contrôle l'a
                // montrée morte — on ne garde pas un chemin que rien ne
                // peut prendre, il se lirait comme une garde.
                None => {}
            }
            if recherchés.is_empty() && emplacements.is_empty() {
                break;
            }
            continue;
        }

        let (lus, écrit) = lit_écrit(&decoded);
        let Some(écrit) = écrit else { continue };
        let nom = format!("{écrit:?}").to_ascii_lowercase();
        if !recherchés.contains(&nom) {
            continue;
        }
        // Une lecture en mémoire ajoute une piste, elle n'en remplace
        // aucune. Les deux questions sont bonnes, et la réponse utile
        // n'est pas toujours la même :
        //
        //   - QUELLE case ? Elle dépend des registres de l'adresse.
        //     `mov rax, [rsi+8]` avec RSI reçu en argument veut dire
        //     « le champ +8 de l'objet qu'on m'a passé », et c'est tout
        //     ce qu'on veut savoir.
        //   - QUOI dans la case ? Ce que la dernière écriture y a mis.
        //     `mov rax, [rbp-8]` après `mov [rbp-8], rdi` veut dire
        //     « le premier argument, rangé sur la pile » — et remonter
        //     RBP ne mènerait qu'à la mise en place du cadre.
        //
        // On suit donc les deux. Le premier jet remplaçait la première
        // par la seconde, et perdait une information vraie.
        let case = emplacement(&decoded);
        let dépendances: Vec<String> = lus
            .iter()
            .map(|r| format!("{r:?}").to_ascii_lowercase())
            .collect();
        out.push(Step {
            addr: instr.address,
            text: instr.text.clone(),
            writes: nom.clone(),
            reads: dépendances.clone(),
            through: case.as_ref().map(Case::nom),
        });
        // Ce registre est expliqué ; on cherche maintenant ce dont il
        // dépendait.
        recherchés.retain(|r| *r != nom);
        if let Some(case) = case
            && !emplacements.contains(&case)
        {
            emplacements.push(case);
        }
        for dépendance in dépendances {
            if !recherchés.contains(&dépendance) {
                recherchés.push(dépendance);
            }
        }
        if recherchés.is_empty() && emplacements.is_empty() {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La chaîne d'une valeur se remonte jusqu'à sa source.
    #[test]
    fn a_value_is_traced_back_to_where_it_came_from() {
        // mov rax, 5 ; mov rcx, rax ; add rcx, 2 ; cmp rcx, 7
        let bytes: &[u8] = &[
            0x48, 0xC7, 0xC0, 0x05, 0x00, 0x00, 0x00, // mov rax, 5
            0x48, 0x89, 0xC1, // mov rcx, rax
            0x48, 0x83, 0xC1, 0x02, // add rcx, 2
            0x48, 0x83, 0xF9, 0x07, // cmp rcx, 7
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cmp = disasm.last().expect("le cmp").address;

        let chaîne = trace(&disasm, cmp, "rcx");
        // Du plus proche au plus lointain : le `add`, le `mov rcx, rax`,
        // puis le `mov rax, 5` qui explique RAX.
        assert_eq!(chaîne.len(), 3, "{chaîne:?}");
        assert!(chaîne[0].text.starts_with("add"), "{:?}", chaîne[0]);
        assert_eq!(chaîne[0].writes, "rcx");
        assert!(chaîne[1].text.contains("rcx,rax"), "{:?}", chaîne[1]);
        assert!(chaîne[2].text.contains("5"), "{:?}", chaîne[2]);
        assert_eq!(chaîne[2].writes, "rax", "on a remonté jusqu'à la source");
    }

    /// Ce qui dépend de la mémoire dépend des registres de l'adresse :
    /// c'est par eux qu'on continue.
    #[test]
    fn a_memory_read_depends_on_the_registers_of_its_address() {
        // mov rsi, rdi ; mov rax, [rsi+8] ; test rax, rax
        let bytes: &[u8] = &[
            0x48, 0x89, 0xFE, // mov rsi, rdi
            0x48, 0x8B, 0x46, 0x08, // mov rax, [rsi+8]
            0x48, 0x85, 0xC0, // test rax, rax
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let test = disasm.last().expect("le test").address;

        let chaîne = trace(&disasm, test, "rax");
        assert!(chaîne.len() >= 2, "{chaîne:?}");
        assert!(
            chaîne[0].reads.contains(&"rsi".to_owned()),
            "{:?}",
            chaîne[0]
        );
        assert_eq!(chaîne[1].writes, "rsi");
        assert!(
            chaîne[1].reads.contains(&"rdi".to_owned()),
            "et RSI venait du premier argument : {:?}",
            chaîne[1]
        );
    }

    /// On ne remonte pas au-delà d'un appel : ce qui précède n'est pas
    /// forcément ce qui a été exécuté.
    #[test]
    fn the_trail_stops_at_a_call() {
        // mov rax, 1 ; call +0 ; cmp rax, 7
        let bytes: &[u8] = &[
            0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00, // mov rax, 1
            0xE8, 0x00, 0x00, 0x00, 0x00, // call
            0x48, 0x83, 0xF8, 0x07, // cmp rax, 7
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cmp = disasm.last().expect("le cmp").address;
        assert!(
            trace(&disasm, cmp, "rax").is_empty(),
            "RAX vient de l'appel, et l'on ne prétend pas savoir d'où"
        );
    }

    /// Une valeur rangée sur la pile puis relue se remonte jusqu'à elle.
    ///
    /// C'est le chemin ordinaire de tout argument dans du code compilé
    /// sans optimisation : on le range dans le cadre, on le ressort plus
    /// bas. La remontée s'arrêtait net à la lecture — « ça vient de la
    /// mémoire », ce qui est vrai et n'apprend rien. Elle traverse
    /// maintenant, et dit par où.
    #[test]
    fn a_value_that_went_through_the_stack_is_followed_across_it() {
        // mov [rbp-8], rdi ; mov rax, [rbp-8] ; cmp rax, 5
        let bytes: &[u8] = &[
            0x48, 0x89, 0x7D, 0xF8, // mov [rbp-8], rdi
            0x48, 0x8B, 0x45, 0xF8, // mov rax, [rbp-8]
            0x48, 0x83, 0xF8, 0x05, // cmp rax, 5
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cmp = disasm.last().expect("le cmp").address;
        let chaîne = trace(&disasm, cmp, "rax");

        assert!(chaîne.len() >= 2, "{chaîne:?}");
        // Le premier maillon : la lecture, qui nomme la case traversée.
        assert_eq!(chaîne[0].writes, "rax");
        assert!(chaîne[0].through.is_some(), "{:?}", chaîne[0]);
        // Le second : l'écriture qui a rempli cette case, et ce qu'elle
        // y a mis — le premier argument.
        assert!(
            chaîne[1].text.contains("rdi"),
            "la valeur vient de RDI : {:?}",
            chaîne[1]
        );
        assert!(
            chaîne[1].reads.contains(&"rdi".to_owned()),
            "et l'on continue par RDI : {:?}",
            chaîne[1]
        );
        // Les deux parlent bien de la MÊME case.
        assert_eq!(chaîne[0].through, chaîne[1].through);
    }

    /// Une écriture vers une case qu'on ne sait pas nommer coupe la
    /// piste, plutôt que de promettre une chaîne fausse.
    ///
    /// `mov [rax], rcx` peut très bien viser `[rbp-8]` si RAX vaut
    /// RBP-8 : rien dans le code ne le dit. Traverser quand même
    /// donnerait une provenance qui se lit bien et qui est fausse — le
    /// pire des cas, puisqu'on la croirait.
    #[test]
    fn an_unknown_write_cuts_the_trail_instead_of_guessing() {
        // mov [rbp-8], rdi ; mov [rax], rcx ; mov rdx, [rbp-8] ; cmp rdx, 5
        let bytes: &[u8] = &[
            0x48, 0x89, 0x7D, 0xF8, // mov [rbp-8], rdi
            0x48, 0x89, 0x08, // mov [rax], rcx
            0x48, 0x8B, 0x55, 0xF8, // mov rdx, [rbp-8]
            0x48, 0x83, 0xFA, 0x05, // cmp rdx, 5
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        // On part du `cmp` : la remontée regarde ce qui PRÉCÈDE son
        // point de départ, donc la lecture doit être avant lui.
        let cmp = disasm.last().expect("le cmp").address;
        let chaîne = trace(&disasm, cmp, "rdx");
        assert!(
            !chaîne.is_empty(),
            "la remontée doit au moins trouver la lecture"
        );
        assert!(
            !chaîne.iter().any(|pas| pas.text.contains("rdi")),
            "la piste ne doit pas traverser une écriture inconnue : {chaîne:?}"
        );
    }

    /// Une écriture ailleurs, elle, ne coupe rien.
    ///
    /// `[rbp-16]` et `[rbp-8]` sont deux cases distinctes, nommées de la
    /// même façon des deux côtés : rien ne les confond. Couper là
    /// rendrait la traversée inutile — dans un cadre de pile, il y a
    /// toujours d'autres écritures entre les deux.
    #[test]
    fn a_write_somewhere_else_does_not_cut_the_trail() {
        // mov [rbp-8], rdi ; mov [rbp-16], rsi ; mov rax, [rbp-8] ; cmp rax, 5
        let bytes: &[u8] = &[
            0x48, 0x89, 0x7D, 0xF8, // mov [rbp-8], rdi
            0x48, 0x89, 0x75, 0xF0, // mov [rbp-16], rsi
            0x48, 0x8B, 0x45, 0xF8, // mov rax, [rbp-8]
            0x48, 0x83, 0xF8, 0x05, // cmp rax, 5
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cmp = disasm.last().expect("le cmp").address;
        let chaîne = trace(&disasm, cmp, "rax");
        assert!(
            chaîne.iter().any(|pas| pas.text.contains("rdi")),
            "la case voisine ne doit pas couper la piste : {chaîne:?}"
        );
        assert!(
            !chaîne.iter().any(|pas| pas.text.contains("rsi")),
            "et l'on ne confond pas les deux cases : {chaîne:?}"
        );
    }
}
