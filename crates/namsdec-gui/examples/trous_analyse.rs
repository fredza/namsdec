//! Où l'analyse statique s'arrête, et combien de fois.
//!
//! « Améliorer fortement l'analyse statique » demande d'abord de savoir
//! OÙ elle est faible. Ce banc compte, sur un corpus, les endroits où
//! elle rend « je ne sais pas » : un saut dont la destination se calcule,
//! un appel dont la cible se calcule, un appel système dont le numéro ne
//! se lit pas. Le plus gros compte désigne le chantier.
use namsdec_disasm::{Mnemonic, OpKind};

fn main() {
    let mut total = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut déjà: std::collections::BTreeSet<[u8; 32]> = Default::default();
    for chemin in std::env::args().skip(1) {
        let Ok(octets) = std::fs::read(&chemin) else {
            continue;
        };
        if !déjà.insert(namsdec_re::digest::sha256(&octets)) {
            continue;
        }
        let Some(elf) = namsdec_asm::elf::parse(&octets) else {
            continue;
        };
        if !elf.machine.supported() {
            continue;
        }
        let mut disasm = Vec::new();
        for (adresse, code) in elf.executable_parts() {
            disasm.extend(namsdec_disasm::disassemble(code, adresse));
        }
        if disasm.is_empty() {
            continue;
        }
        let nom = std::path::Path::new(&chemin)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let (mut jmp_ind, mut call_ind, mut jmp_dir, mut call_dir) = (0, 0, 0, 0);
        for instruction in &disasm {
            let Some(décodée) = namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
            else {
                continue;
            };
            let indirect = décodée.op_count() > 0
                && !matches!(
                    décodée.op_kind(0),
                    OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64
                );
            match décodée.mnemonic() {
                Mnemonic::Jmp if indirect => jmp_ind += 1,
                Mnemonic::Jmp => jmp_dir += 1,
                Mnemonic::Call if indirect => call_ind += 1,
                Mnemonic::Call => call_dir += 1,
                _ => {}
            }
        }
        // Les appels système dont le numéro ne se lit pas d'ici.
        let appels = namsdec_re::syscalls::scan(&disasm);
        let illisibles = appels.iter().filter(|c| c.number.is_none()).count();
        // Les fonctions trouvées, et celles que les symboles nomment.
        let entrée = elf.entry;
        let fonctions = namsdec_re::functions::discover(&disasm, entrée).len();
        let imports_f = namsdec_re::imports::scan(&octets);
        let avec: Vec<(u64, u64)> = namsdec_re::indirect::resolve(&disasm, &octets, &imports_f)
            .into_iter()
            .filter(|r| r.appel)
            .filter_map(|r| Some((r.at, r.target?)))
            .collect();
        let fonctions_avec = namsdec_re::functions::discover_with(&disasm, entrée, &avec).len();
        let nommées = elf.symbols.iter().filter(|s| s.addr != 0).count();

        // Ce que la résolution des cibles indirectes retrouve.
        let imports = namsdec_re::imports::scan(&octets);
        let résolus = namsdec_re::indirect::resolve(&disasm, &octets, &imports);
        let indirects = jmp_ind + call_ind;
        let part = résolus
            .len()
            .checked_mul(100)
            .and_then(|n| n.checked_div(indirects))
            .unwrap_or(0);
        let mut par_chemin: std::collections::BTreeMap<String, usize> = Default::default();
        for r in &résolus {
            *par_chemin.entry(format!("{:?}", r.par)).or_default() += 1;
        }
        println!(
            "{nom:22} {} instr · indirects {indirects} dont {} RÉSOLUS ({part} %) {:?} · \
             syscall {} / {} illisible · fonctions {fonctions} -> {fonctions_avec} · \
             arêtes d'appel retrouvées {}",
            disasm.len(),
            résolus.len(),
            par_chemin,
            appels.len(),
            illisibles,
            avec.len()
        );
        let _ = nommées;
        let tables = namsdec_re::indirect::jump_tables(&disasm, &octets);
        let cas: usize = tables.iter().map(|t| t.cases.len()).sum();
        if !tables.is_empty() {
            println!(
                "                       -> {} tables de sauts, {cas} cas",
                tables.len()
            );
        }
        total.6 += résolus.len();
        total.0 += jmp_ind;
        total.1 += call_ind;
        total.2 += illisibles;
        total.3 += appels.len();
        total.4 += fonctions;
        total.5 += jmp_dir + call_dir;
    }
    let indirects = total.0 + total.1;
    println!(
        "\n=== TOTAL : {indirects} branchements calculés ({} sauts, {} appels), dont \
         {} RÉSOLUS — {} %. {} appels système sur {} au numéro illisible. \
         {} fonctions trouvées, {} branchements directs. ===",
        total.0,
        total.1,
        total.6,
        total
            .6
            .checked_mul(100)
            .and_then(|n| n.checked_div(indirects))
            .unwrap_or(0),
        total.2,
        total.3,
        total.4,
        total.5
    );
}
