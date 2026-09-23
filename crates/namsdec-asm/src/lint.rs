//! Ce que nasm ne dit pas.
//!
//! L'assembleur vérifie la grammaire : il refuse ce qui ne s'encode pas,
//! et se tait sur tout le reste. Or les deux fautes qui coûtent le plus
//! cher à qui apprend passent sans un mot — un programme qui ne s'arrête
//! pas et continue dans ce qui suit son code, et une écriture dans une
//! zone qui n'est pas inscriptible, qui ne se manifestera qu'à
//! l'exécution.
//!
//! Ces contrôles-là se font *après* l'assemblage, sur le programme lié :
//! c'est là qu'on connaît les adresses, les droits de chaque section et
//! la dernière instruction de chacune. Deviner la même chose dans le
//! texte du source donnerait des avertissements faux, ce qui est pire
//! que pas d'avertissement du tout.

use std::collections::BTreeMap;

use crate::{AssembleOutput, Diagnostic, Severity};

/// Ce que namsdec a remarqué, en plus de ce que nasm a dit.
///
/// Les messages sont en anglais, comme toutes les erreurs des crates
/// métier : c'est l'interface qui les traduit. Ils portent leur ligne
/// source quand on peut la retrouver, et la ligne 0 — « le programme
/// entier » — sinon.
pub fn check(output: &AssembleOutput, fr: bool) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    falls_off_the_end(output, fr, &mut found);
    writes_to_read_only(output, fr, &mut found);
    found
}

/// Le programme peut-il tomber hors de son code ?
///
/// La dernière instruction exécutable décide : si elle n'arrête pas la
/// machine, ne saute pas ailleurs et ne revient pas d'un appel, le
/// processeur exécutera ce qui la suit — les données, puis n'importe
/// quoi. C'est la faute qui fait dire « mon programme marche mais il
/// plante à la fin ».
fn falls_off_the_end(output: &AssembleOutput, fr: bool, found: &mut Vec<Diagnostic>) {
    for section in output.sections.iter().filter(|s| s.executable) {
        let Some(code) = section_bytes(output, section) else {
            continue;
        };
        let instructions = namsdec_disasm::disassemble(code, section.addr);
        let Some(last) = instructions.last() else {
            continue;
        };
        let mnemonic = last
            .text
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        // `syscall` compte : c'est ainsi qu'un programme sort pour de
        // bon, et l'on ne peut pas savoir ici que RAX vaut 60 — mais
        // supposer le contraire crierait au loup sur tout programme
        // correct.
        let stops = matches!(
            mnemonic.as_str(),
            "hlt" | "ret" | "jmp" | "syscall" | "int" | "int3" | "ud2"
        );
        if !stops {
            found.push(Diagnostic {
                line: output
                    .line_to_addr
                    .iter()
                    .find(|(_, addr)| **addr == last.address)
                    .map(|(line, _)| *line)
                    .unwrap_or(0),
                severity: Severity::Warning,
                message: if fr {
                    format!(
                        "l'exécution peut sortir par la fin de {} : la dernière \
                         instruction est « {} », qui n'arrête ni ne saute — ajoutez \
                         « hlt », ou sortez par l'appel système 60",
                        section.name, last.text
                    )
                } else {
                    format!(
                        "execution can run off the end of {}: the last instruction is `{}`, \
                         which neither stops nor jumps — add `hlt`, or exit with syscall 60",
                        section.name, last.text
                    )
                },
            });
        }
    }
}

/// Une écriture à une adresse fixe qui tombe dans une zone en lecture
/// seule.
///
/// L'adresse est dans l'encodage : pas besoin d'exécuter quoi que ce
/// soit pour savoir que `mov [message], al` échouera si `message` est
/// dans `.rodata` — ou dans `.text`, ce qui arrive quand on a oublié
/// d'ouvrir une section de données.
fn writes_to_read_only(output: &AssembleOutput, fr: bool, found: &mut Vec<Diagnostic>) {
    // Les zones en lecture seule, par adresse de début.
    let protégées: BTreeMap<u64, &str> = output
        .sections
        .iter()
        .filter(|section| !section.writable && section.size > 0)
        .map(|section| (section.addr, section.name.as_str()))
        .collect();
    if protégées.is_empty() {
        return;
    }

    for section in output.sections.iter().filter(|s| s.executable) {
        let Some(code) = section_bytes(output, section) else {
            continue;
        };
        for decoded in namsdec_disasm::disassemble(code, section.addr) {
            let Some(instr) = namsdec_disasm::decode_one(&decoded.bytes, decoded.address) else {
                continue;
            };
            let Some((addr, nom)) = written_address(&instr, &protégées, output) else {
                continue;
            };
            found.push(Diagnostic {
                line: output
                    .line_to_addr
                    .iter()
                    .find(|(_, a)| **a == decoded.address)
                    .map(|(line, _)| *line)
                    .unwrap_or(0),
                severity: Severity::Warning,
                message: if fr {
                    format!(
                        "ceci écrit en {addr:#x}, dans {nom}, qui n'est pas \
                         inscriptible — ce qui change pendant l'exécution a sa place \
                         dans « section .data » ou « section .bss »"
                    )
                } else {
                    format!(
                        "this writes to {addr:#x}, inside {nom}, which is not writable — \
                         data that changes while running belongs in `section .data` or \
                         `section .bss`"
                    )
                },
            });
        }
    }
}

/// L'adresse fixe qu'une instruction écrit, si elle tombe dans une zone
/// protégée.
fn written_address(
    instr: &namsdec_disasm::Decoded,
    protégées: &BTreeMap<u64, &str>,
    output: &AssembleOutput,
) -> Option<(u64, String)> {
    use namsdec_disasm::{OpKind, Register};
    // Le premier opérande seulement : c'est celui qu'une instruction
    // écrit. Les autres sont lus, et lire une constante est permis.
    if instr.op_count() == 0 || instr.op0_kind() != OpKind::Memory {
        return None;
    }
    // Seules les adresses écrites en dur se lisent ici : `[rsi]` dépend
    // d'un registre, et personne ne sait ce qu'il vaudra.
    let base = instr.memory_base();
    // Pour un opérande relatif à RIP, iced rend déjà l'adresse absolue :
    // y rajouter `next_ip` donnait une adresse qui n'existe nulle part,
    // et l'avertissement visait alors à côté.
    let addr = if base == Register::RIP || base == Register::EIP {
        instr.ip_rel_memory_address()
    } else if base == Register::None && instr.memory_index() == Register::None {
        instr.memory_displacement64()
    } else {
        return None;
    };
    // Une instruction qui ne fait que lire n'écrit rien : `cmp` et
    // `test` comparent, `push` empile ce qu'elle lit.
    let mnemonic = format!("{:?}", instr.mnemonic()).to_ascii_lowercase();
    if matches!(mnemonic.as_str(), "cmp" | "test" | "push" | "bt") {
        return None;
    }
    let section = output
        .sections
        .iter()
        .find(|s| addr >= s.addr && addr < s.addr + s.size as u64)?;
    if !protégées.contains_key(&section.addr) {
        return None;
    }
    Some((addr, section.name.clone()))
}

/// Les octets d'une section, découpés dans l'image chargée.
fn section_bytes<'a>(output: &'a AssembleOutput, section: &crate::Section) -> Option<&'a [u8]> {
    let start = section.addr.checked_sub(output.load_addr)? as usize;
    output.bytes.get(start..start + section.size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(source: &str) -> Option<AssembleOutput> {
        if crate::find_nasm().is_none() || !crate::find_linker() {
            eprintln!("nasm ou ld absent : test ignoré");
            return None;
        }
        Some(crate::assemble(source, 0x400000).expect("assemblage"))
    }

    /// La faute qui fait dire « mon programme marche, mais il plante à
    /// la fin » : rien ne l'arrête, et le processeur continue dans ce
    /// qui suit le code.
    #[test]
    fn a_program_that_never_stops_is_reported() {
        let Some(out) = build("global _start\nsection .text\n_start:\n    mov rax, 1\n") else {
            return;
        };
        let found = check(&out, false);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("run off the end"), "{found:?}");
        assert_eq!(found[0].severity, Severity::Warning);
        // Et la ligne est celle de la dernière instruction, pas le
        // programme entier.
        assert_eq!(found[0].line, 4);
    }

    /// Ce qui s'arrête ne reçoit rien : `hlt`, un saut, un retour, un
    /// appel système.
    #[test]
    fn programs_that_do_stop_are_left_alone() {
        for fin in ["hlt", "jmp _start", "ret", "syscall"] {
            let Some(out) = build(&format!(
                "global _start\nsection .text\n_start:\n    mov rax, 1\n    {fin}\n"
            )) else {
                return;
            };
            let found = check(&out, false);
            assert!(
                found.iter().all(|d| !d.message.contains("run off the end")),
                "« {fin} » arrête le programme : {found:?}"
            );
        }
    }

    /// Écrire dans une zone en lecture seule se voit sans exécuter quoi
    /// que ce soit : l'adresse est dans l'encodage.
    #[test]
    fn a_write_into_read_only_memory_is_reported_before_running() {
        let Some(out) = build(
            "global _start\n\
             section .rodata\n\
             constante: dq 7\n\
             section .text\n\
             _start:\n\
             \x20   mov qword [constante], 9\n\
             \x20   hlt\n",
        ) else {
            return;
        };
        let found = check(&out, false);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("not writable"), "{found:?}");
        assert!(found[0].message.contains("section .data"), "{found:?}");
        assert_eq!(found[0].line, 6, "la ligne de l'écriture");
    }

    /// La même écriture, en adressage relatif à RIP — celui que produit
    /// `default rel`, et donc celui de presque tout programme écrit
    /// aujourd'hui.
    ///
    /// Elle passait au travers : l'adresse calculée tombait à côté, et
    /// l'avertissement ne partait jamais là où il fallait.
    #[test]
    fn a_rip_relative_write_into_read_only_memory_is_reported_too() {
        let Some(out) = build(
            "default rel\n\
             global _start\n\
             section .rodata\n\
             constante: dq 7\n\
             section .text\n\
             _start:\n\
             \x20   mov qword [constante], 9\n\
             \x20   hlt\n",
        ) else {
            return;
        };
        let found = check(&out, false);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("not writable"), "{found:?}");
    }

    /// Lire une constante est permis, et écrire dans `.data` aussi :
    /// un avertissement faux est pire que pas d'avertissement.
    #[test]
    fn reading_a_constant_and_writing_to_data_are_fine() {
        let Some(out) = build(
            "global _start\n\
             section .rodata\n\
             constante: dq 7\n\
             section .data\n\
             variable: dq 0\n\
             section .text\n\
             _start:\n\
             \x20   mov rax, [constante]\n\
             \x20   cmp rax, [constante]\n\
             \x20   mov qword [variable], 9\n\
             \x20   hlt\n",
        ) else {
            return;
        };
        assert!(check(&out, false).is_empty(), "{:?}", check(&out, false));
    }

    /// Une écriture par registre ne se juge pas ici : personne ne sait
    /// ce que vaudra RSI, et le supposer donnerait des avertissements
    /// faux.
    #[test]
    fn an_indirect_write_is_not_judged() {
        let Some(out) = build(
            "global _start\n\
             section .rodata\n\
             constante: dq 7\n\
             section .text\n\
             _start:\n\
             \x20   mov rsi, constante\n\
             \x20   mov qword [rsi], 9\n\
             \x20   hlt\n",
        ) else {
            return;
        };
        assert!(check(&out, false).is_empty(), "{:?}", check(&out, false));
    }
}
