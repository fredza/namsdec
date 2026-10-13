use crate::behaviour::{self, Family};
use crate::{ReContext, ReModule, ReSection, elf_scan, imports, strings, syscalls, xrefs};

/// Le résumé : de quoi on parle, avant d'ouvrir les autres panneaux.
///
/// Six modules détaillent chacun une chose ; celui-ci répond à la
/// question qu'on pose en premier — qu'est-ce que c'est, et qu'est-ce
/// qu'il y a dedans ? Tout ce qu'il montre est mesuré, pas déclaré :
/// les fonctions viennent des `call` relevés, les chaînes des octets
/// lisibles, la nature du fichier de ses en-têtes, et ce qu'il *sait
/// faire* des fonctions qu'il emprunte et des appels système qu'il passe.
#[derive(Default)]
pub struct BinaryInfoModule {
    size: usize,
    entry: u64,
    instructions: usize,
    functions: usize,
    strings: usize,
    imports: usize,
    syscalls: usize,
    /// Le comportement, par famille, dans l'ordre de lecture — ce que le
    /// binaire sait faire, avec le nombre d'appels qui le tiennent.
    comportement: Vec<(Family, usize)>,
    /// Ce que l'en-tête ELF dit du fichier, quand on l'a : nature,
    /// liaison, protections. Gardé entier pour en tirer plusieurs lignes
    /// sans rescanner à chaque image.
    scan: Option<elf_scan::Scan>,
}

impl ReModule for BinaryInfoModule {
    fn section(&self) -> ReSection {
        ReSection::File
    }

    fn id(&self) -> &'static str {
        "binary_info"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "En bref"
        } else {
            "At a glance"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.size = ctx.disasm.iter().map(|i| i.bytes.len()).sum();
        self.entry = ctx.disasm.first().map(|i| i.address).unwrap_or(0);
        self.instructions = ctx.disasm.len();

        let range = ctx.cpu.mem.range();
        self.functions = xrefs::scan(ctx.disasm, range.clone()).functions().len();
        self.strings = (range.end - range.start)
            .try_into()
            .ok()
            .and_then(|len: usize| ctx.cpu.mem.peek_bytes(range.start, len).ok())
            .map(|bytes| strings::scan(bytes, range.start).len())
            .unwrap_or(0);

        // Ce que le binaire sait faire : relevé de ses fonctions
        // empruntées ET de ses appels système, rangé par famille. Les
        // deux, car un programme qui veut passer inaperçu appelle le
        // noyau par son numéro plutôt que par la table de liaison.
        let imports = imports::scan(ctx.binary);
        self.imports = imports.len();
        let appels = syscalls::scan(ctx.disasm);
        self.syscalls = appels.len();
        self.comportement = behaviour::ALL
            .iter()
            .map(|&famille| {
                let n = imports
                    .iter()
                    .filter(|i| behaviour::family_of(&i.name) == Some(famille))
                    .count()
                    + appels
                        .iter()
                        .filter(|c| c.family() == Some(famille))
                        .count();
                (famille, n)
            })
            .collect();

        self.scan = elf_scan::scan(ctx.binary);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        egui::Grid::new("binary_info_grid")
            .num_columns(2)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                let mut ligne = |label: &str, valeur: String| {
                    ui.label(label);
                    ui.monospace(valeur);
                    ui.end_row();
                };
                ligne(
                    ctx.t("Code", "Code"),
                    format!(
                        "{} {} · {} {}",
                        self.size,
                        ctx.t("octets", "bytes"),
                        self.instructions,
                        ctx.t("instructions", "instructions")
                    ),
                );
                ligne(
                    ctx.t("Point d'entrée", "Entry point"),
                    format!("{:#x}", self.entry),
                );
                ligne(
                    ctx.t("Fonctions", "Functions"),
                    format!(
                        "{} ({})",
                        self.functions,
                        ctx.t("cibles de `call`", "`call` targets")
                    ),
                );
                ligne(ctx.t("Chaînes", "Strings"), format!("{}", self.strings));
                ligne(
                    ctx.t("Emprunte", "Borrows"),
                    format!(
                        "{} {} · {} {}",
                        self.imports,
                        ctx.t("fonctions", "functions"),
                        self.syscalls,
                        ctx.t("appels système", "system calls")
                    ),
                );
                if let Some(scan) = &self.scan {
                    ligne(
                        ctx.t("Fichier", "File"),
                        format!(
                            "{} · {}",
                            if scan.pie() { "PIE" } else { "EXEC" },
                            if scan.dynamic() { "dyn" } else { "static" }
                        ),
                    );
                }
            });

        // Ce qu'il sait faire : le portrait, puis une ligne par famille
        // présente. C'est la réponse à « a-t-il accès au réseau ? » et aux
        // cinq autres questions du même ordre — lue d'un coup d'œil, et
        // expliquée juste en dessous.
        let présentes: Vec<(Family, usize)> = self
            .comportement
            .iter()
            .copied()
            .filter(|(_, n)| *n > 0)
            .collect();
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.strong(ctx.t("Ce qu'il sait faire", "What it can do"));
        if présentes.is_empty() {
            ui.weak(ctx.t(
                "Rien de remarquable : ni réseau, ni exécution, ni chiffrement, ni accès \
                 fichier repéré. Un programme peut n'avoir besoin de rien de tout cela.",
                "Nothing notable: no network, execution, cryptography or file access spotted. \
                 A program may need none of these.",
            ));
        } else {
            crate::imports_module::portrait(ui, ctx, &présentes);
            ui.add_space(2.0);
            for (famille, combien) in &présentes {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        famille.color(),
                        format!("{} ({combien})", famille.label(ctx.lang)),
                    );
                    ui.weak(famille.why(ctx.lang));
                });
            }
        }

        // Les protections, en un coup d'œil : le détail complet reste au
        // panneau « Protections », mais les quatre qui décident de tout se
        // lisent ici.
        if let Some(scan) = &self.scan {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.strong(ctx.t("Protections", "Protections"));
            // « √ » et « × », et non « ✓ » et « ✗ » : ces deux-là
            // n'existent pas dans les polices qu'egui embarque, et se
            // peignaient donc en carré vide — mesuré, voir
            // `tout_caractere_ecrit_existe_dans_la_police`. Une coche
            // absente ne dit pas « protection absente » : elle ne dit
            // rien du tout.
            let oui_non = |bon: bool| {
                if bon {
                    ("\u{221a}", egui::Color32::from_rgb(0x4e, 0xc9, 0xb0))
                } else {
                    ("\u{00d7}", egui::Color32::from_rgb(0xf4, 0x87, 0x71))
                }
            };
            egui::Grid::new("binary_info_prot")
                .num_columns(2)
                .min_col_width(0.0)
                .striped(true)
                .show(ui, |ui| {
                    let prot = |ui: &mut egui::Ui, label: &str, bon: bool, note: String| {
                        let (marque, couleur) = oui_non(bon);
                        ui.colored_label(couleur, format!("{marque} {label}"));
                        ui.weak(note);
                        ui.end_row();
                    };
                    prot(
                        ui,
                        ctx.t("PIE", "PIE"),
                        scan.pie(),
                        ctx.t(
                            "chargé à une adresse au hasard",
                            "loaded at a random address",
                        )
                        .to_owned(),
                    );
                    prot(
                        ui,
                        ctx.t("NX", "NX"),
                        scan.nx.unwrap_or(false),
                        ctx.t("la pile ne s'exécute pas", "the stack does not execute")
                            .to_owned(),
                    );
                    prot(
                        ui,
                        ctx.t("RELRO", "RELRO"),
                        scan.relro == elf_scan::Relro::Full,
                        match scan.relro {
                            elf_scan::Relro::Full => ctx.t("complet", "full"),
                            elf_scan::Relro::Partial => ctx.t("partiel", "partial"),
                            elf_scan::Relro::None => ctx.t("absent", "none"),
                        }
                        .to_owned(),
                    );
                    prot(
                        ui,
                        ctx.t("Canari", "Canary"),
                        scan.canary,
                        ctx.t(
                            "sentinelle contre l'écrasement de pile",
                            "sentinel against stack smashing",
                        )
                        .to_owned(),
                    );
                });
        } else {
            ui.add_space(6.0);
            ui.weak(ctx.t(
                "Pas d'en-tête ELF : ce sont des octets bruts, désassemblés depuis le premier.",
                "No ELF header: these are raw bytes, disassembled from the first one.",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    #[test]
    fn the_summary_counts_what_it_sees() {
        let bytes: &[u8] = &[
            0xe8, 0x01, 0x00, 0x00, 0x00, // call +1
            0x90, // nop  (la cible)
            0xf4, // hlt
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };

        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.size, 7);
        assert_eq!(module.entry, 0x400000);
        assert_eq!(module.instructions, 3);
        // Le `call` désigne une adresse : c'est une fonction.
        assert_eq!(module.functions, 1);
        // Pas d'en-tête : on ne prétend rien sur la nature du fichier.
        assert!(module.scan.is_none());
    }

    /// Le portrait compte ce que le code DEMANDE au système, même par
    /// numéro : `mov eax, 41 ; syscall` est un `socket`, donc du réseau.
    #[test]
    fn the_portrait_counts_a_network_syscall() {
        // b8 29 00 00 00 = mov eax, 41 (socket) ; 0f 05 = syscall.
        let bytes: &[u8] = &[0xb8, 0x29, 0x00, 0x00, 0x00, 0x0f, 0x05];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };
        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.syscalls, 1, "un appel système relevé");
        let réseau = module
            .comportement
            .iter()
            .find(|(f, _)| *f == Family::Network)
            .map(|(_, n)| *n)
            .unwrap_or(0);
        assert_eq!(
            réseau, 1,
            "le socket compte comme du réseau : {:?}",
            module.comportement
        );
    }
}
