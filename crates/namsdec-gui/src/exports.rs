//! Le texte que les vues produisent pour être copié ou enregistré : le
//! contexte d'une ligne, le listing entier, le rapport d'analyse.
//!
//! Des méthodes de `NamsdecApp` regroupées hors de `main.rs` — elles
//! répondent toutes à « emporter ce qui est à l'écran », et rien d'autre.

use super::*;

impl NamsdecApp {
    /// L'état du processeur, en texte : ce qu'on colle pour demander de
    /// l'aide, ou dans un compte rendu de TP.
    ///
    /// C'est le `context` de pwndbg : la prochaine instruction, les
    /// registres, les indicateurs, le haut de la pile et la chaîne des
    /// appels. Un état qui ne sort pas de l'écran oblige à le recopier à
    /// la main — et on recopie mal.
    pub(crate) fn context_text(&self) -> Option<String> {
        let emulator = self.emulator.as_ref()?;
        let s = strings(self.lang);
        let cpu = emulator.cpu();
        let regs = &cpu.regs;
        let mut out = format!("namsdec {APP_VERSION} — {}\n\n", s.context_title);

        // Là où l'on est, et ce qui va s'exécuter.
        let window = cpu.mem.peek_bytes(regs.rip, 16).ok();
        let instruction = window
            .and_then(|bytes| namsdec_disasm::decode_one(bytes, regs.rip))
            .map(|decoded| namsdec_disasm::format_one(&decoded));
        out.push_str(&format!("RIP {:#x}", regs.rip));
        if let Some(texte) = &instruction {
            out.push_str(&format!("  {texte}"));
        }
        if cpu.halted {
            out.push_str(&format!("  ({})", s.next_instruction_done));
        }
        out.push_str("\n\n");

        // Les registres, quatre par ligne : c'est ce qui se relit dans un
        // message sans partir à la ligne.
        out.push_str(&format!("{}\n", s.context_registers));
        for (index, (name, reg)) in REGISTERS.iter().enumerate() {
            out.push_str(&format!("{name:<3} {:016x}   ", regs.get64(*reg)));
            if index % 4 == 3 {
                out.push('\n');
            }
        }
        if !REGISTERS.len().is_multiple_of(4) {
            out.push('\n');
        }
        let indicateurs: Vec<String> = [
            ("CF", namsdec_core::RFlags::CF),
            ("PF", namsdec_core::RFlags::PF),
            ("AF", namsdec_core::RFlags::AF),
            ("ZF", namsdec_core::RFlags::ZF),
            ("SF", namsdec_core::RFlags::SF),
            ("OF", namsdec_core::RFlags::OF),
        ]
        .iter()
        .map(|(name, bit)| format!("{name} {}", u8::from(cpu.flags.contains(*bit))))
        .collect();
        out.push_str(&format!(
            "{} {}\n\n",
            s.context_flags,
            indicateurs.join(" ")
        ));

        // Le haut de la pile, pointeurs suivis : c'est là que se lisent
        // les adresses de retour.
        out.push_str(&format!("{}\n", s.context_stack));
        let rsp = regs.get64(Reg64::Rsp);
        for index in 0..8u64 {
            let addr = rsp + index * 8;
            let links =
                namsdec_emu::telescope::chain(&cpu.mem, addr, namsdec_emu::telescope::LIMIT);
            let Some(first) = links.first() else { break };
            out.push_str(&format!("{addr:#010x}  {:016x}", first.value));
            for link in &links[1..] {
                out.push_str(&format!("  -> {:#x}", link.value));
            }
            if let Some(fin) = sidebar::chain_end(links.last()) {
                out.push_str(&format!("  {fin}"));
            }
            out.push('\n');
        }

        // Et la chaîne des appels, quand il y en a une.
        let frames = emulator.frames();
        if !frames.is_empty() {
            out.push_str(&format!("\n{}\n", s.context_calls));
            for frame in frames {
                out.push_str(&format!("{:#x}\n", frame.entry));
            }
        }
        Some(out)
    }

    /// Le désassemblage du programme, en texte, prêt à être collé.
    ///
    /// Les étiquettes aux endroits où elles tombent, l'adresse, les
    /// octets et l'instruction : ce qu'on recopie à la main dans un
    /// compte rendu quand l'outil ne sait pas le donner.
    pub(crate) fn listing_text(&self) -> Option<String> {
        let state = self.assemble_state.as_ref()?;
        if state.disasm.is_empty() {
            return None;
        }
        let symbols = &state.output.symbols;
        let mut out = String::new();
        for instruction in &state.disasm {
            for symbol in symbols.iter().filter(|s| s.addr == instruction.address) {
                out.push_str(&format!("\n{}:\n", symbol.name));
            }
            let octets = instruction
                .bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            let texte = namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
                .map(|decoded| namsdec_disasm::format_one(&decoded))
                .unwrap_or_else(|| instruction.text.clone());
            // Les octets sur une largeur fixe : c'est ce qui rend un
            // listing lisible en colonnes, et dix octets est le maximum
            // d'une instruction ordinaire.
            out.push_str(&format!(
                "{:08x}  {octets:<30}  {texte}\n",
                instruction.address
            ));
            // La cible nommée, comme dans l'éditeur.
            if let Some(target) = branch_target(&texte)
                && let Some(label) = symbol_at(symbols, target)
            {
                let fin = out.trim_end().len();
                out.truncate(fin);
                out.push_str(&format!("    ; -> {label}\n"));
            }
        }
        Some(out)
    }

    /// Le rapport d'analyse du binaire chargé, en Markdown.
    ///
    /// `None` quand il n'y a rien à analyser — pas de binaire, ou un
    /// tampon brut qui n'est pas un ELF.
    pub(crate) fn analysis_report(&self) -> Option<String> {
        let state = self.assemble_state.as_ref()?;
        let scan = namsdec_re::elf_scan::scan(&state.output.file)?;
        let name = self
            .current_file
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| strings(self.lang).analysis_title.to_owned());
        Some(namsdec_re::report::markdown(
            &scan,
            &state.output.file,
            &state.disasm,
            self.lang.code(),
            &name,
        ))
    }
}
