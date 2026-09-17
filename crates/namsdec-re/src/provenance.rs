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
}

/// Jusqu'où l'on remonte, en instructions.
///
/// Au-delà, on est presque sûrement sorti du bloc — et l'on aurait
/// traversé un saut sans le savoir.
const REMONTÉE: usize = 64;

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
        let (lus, écrit) = lit_écrit(&decoded);
        let Some(écrit) = écrit else { continue };
        let nom = format!("{écrit:?}").to_ascii_lowercase();
        if !recherchés.contains(&nom) {
            continue;
        }
        let dépendances: Vec<String> = lus
            .iter()
            .map(|r| format!("{r:?}").to_ascii_lowercase())
            .collect();
        out.push(Step {
            addr: instr.address,
            text: instr.text.clone(),
            writes: nom.clone(),
            reads: dépendances.clone(),
        });
        // Ce registre est expliqué ; on cherche maintenant ce dont il
        // dépendait.
        recherchés.retain(|r| *r != nom);
        for dépendance in dépendances {
            if !recherchés.contains(&dépendance) {
                recherchés.push(dépendance);
            }
        }
        if recherchés.is_empty() {
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
}
