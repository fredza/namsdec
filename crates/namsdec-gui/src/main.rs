//! Coquille eframe/egui du MVP d'apprentissage NASM. Assemble via
//! `namsdec-asm` (nasm externe), désassemble via `namsdec-disasm`
//! (vérification pédagogique) et exécute pas à pas via `namsdec-emu`,
//! en affichant les panneaux des modules `ReModule` enregistrés.

mod i18n;

use std::collections::{BTreeMap, BTreeSet};

use eframe::egui;
use namsdec_asm::{AsmError, Diagnostic, Severity};
use namsdec_core::{Reg64, RFlags};
use namsdec_disasm::DecodedInstruction;
use namsdec_emu::{Emulator, RunStop, StepResult};
use namsdec_re::{BinaryInfoModule, ReContext, ReModuleRegistry};

use i18n::{strings, Lang, Strings};

const BASE_ADDR: u64 = 0x400000;
const MEM_SIZE: usize = 0x10000;
const MAX_RUN_STEPS: u32 = 1_000_000;

/// Numéro de version SemVer — ne JAMAIS le monter sans l'accord explicite
/// de l'utilisateur (voir memory: feedback_version_bump_approval).
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Date de compilation (YYYYMMDD), calculée par build.rs — se met à jour
/// seule, aucune approbation requise contrairement à APP_VERSION.
const BUILD_DATE: &str = env!("NAMSDEC_BUILD_DATE");

const GOOD: egui::Color32 = egui::Color32::from_rgb(0x89, 0xd1, 0x85);
const BAD: egui::Color32 = egui::Color32::from_rgb(0xf4, 0x87, 0x71);
const WARN: egui::Color32 = egui::Color32::from_rgb(0xdc, 0xdc, 0xaa);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x37, 0x94, 0xff);

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppTheme {
    Dark,
    Light,
}

impl AppTheme {
    fn visuals(self) -> egui::Visuals {
        let mut visuals = match self {
            AppTheme::Dark => egui::Visuals::dark(),
            AppTheme::Light => egui::Visuals::light(),
        };
        match self {
            AppTheme::Dark => {
                visuals.panel_fill = egui::Color32::from_rgb(0x25, 0x25, 0x26);
                visuals.window_fill = egui::Color32::from_rgb(0x1e, 0x1e, 0x1e);
                visuals.extreme_bg_color = egui::Color32::from_rgb(0x1e, 0x1e, 0x1e);
                visuals.selection.bg_fill = ACCENT;
            }
            AppTheme::Light => {
                visuals.panel_fill = egui::Color32::from_rgb(0xf3, 0xf3, 0xf3);
                visuals.window_fill = egui::Color32::WHITE;
                visuals.extreme_bg_color = egui::Color32::WHITE;
                visuals.selection.bg_fill = egui::Color32::from_rgb(0x00, 0x5f, 0xb8);
            }
        }
        visuals
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SidebarTab {
    Registers,
    Flags,
    Stack,
    Memory,
    Re,
    Modules,
}

impl SidebarTab {
    const ALL: [SidebarTab; 6] = [
        SidebarTab::Registers,
        SidebarTab::Flags,
        SidebarTab::Stack,
        SidebarTab::Memory,
        SidebarTab::Re,
        SidebarTab::Modules,
    ];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            SidebarTab::Registers => s.tab_registers,
            SidebarTab::Flags => s.tab_flags,
            SidebarTab::Stack => s.tab_stack,
            SidebarTab::Memory => s.tab_memory,
            SidebarTab::Re => s.tab_re,
            SidebarTab::Modules => s.tab_modules,
        }
    }
}

enum Status {
    Idle,
    AssembleOk { bytes: usize },
    AssembleFailed,
    Halted,
    Breakpoint(u64),
    StepLimit,
    NasmMissing,
    Error(String),
}

struct AssembleState {
    output: namsdec_asm::AssembleOutput,
    disasm: Vec<DecodedInstruction>,
    addr_to_line: BTreeMap<u64, u32>,
}

struct NamsdecApp {
    source: String,
    theme: AppTheme,
    lang: Lang,
    nasm_info: Option<namsdec_asm::NasmInfo>,
    breakpoint_lines: BTreeSet<u32>,
    bp_input: u32,
    assemble_state: Option<AssembleState>,
    emulator: Option<Emulator>,
    re_registry: ReModuleRegistry,
    status: Status,
    diagnostics: Vec<Diagnostic>,
    active_tab: SidebarTab,
    mem_addr_input: String,
    show_preferences: bool,
    show_about: bool,
}

/// Programme de départ affiché au lancement, pour ouvrir sur un exemple
/// qui fonctionne plutôt qu'un éditeur vide. namsdec-exercises n'est
/// volontairement pas utilisé ici : les exercices deviendront un module
/// séparé, ajoutable indépendamment de la coquille de base.
const DEFAULT_SOURCE: &str = "\
BITS 64
org 0x400000

    mov rax, 0
    mov rcx, 5
sum_loop:
    add rax, rcx
    dec rcx
    jnz sum_loop
    hlt
";

impl Default for NamsdecApp {
    fn default() -> Self {
        let mut re_registry = ReModuleRegistry::new();
        re_registry.register(Box::new(BinaryInfoModule::default()));

        Self {
            source: DEFAULT_SOURCE.to_string(),
            theme: AppTheme::Dark,
            lang: Lang::Fr,
            nasm_info: namsdec_asm::find_nasm(),
            breakpoint_lines: BTreeSet::new(),
            bp_input: 1,
            assemble_state: None,
            emulator: None,
            re_registry,
            status: Status::Idle,
            diagnostics: Vec::new(),
            active_tab: SidebarTab::Registers,
            mem_addr_input: format!("{BASE_ADDR:#x}"),
            show_preferences: false,
            show_about: false,
        }
    }
}

impl NamsdecApp {
    fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.theme.visuals());
    }

    fn do_assemble(&mut self) {
        match namsdec_asm::assemble(&self.source, BASE_ADDR) {
            Ok(output) => {
                let disasm = namsdec_disasm::disassemble(&output.bytes, BASE_ADDR);
                let addr_to_line = output.line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
                let bytes = output.bytes.len();
                self.diagnostics = output.diagnostics.clone();
                self.assemble_state = Some(AssembleState {
                    output,
                    disasm,
                    addr_to_line,
                });
                self.status = Status::AssembleOk { bytes };
                self.rebuild_emulator();
            }
            Err(AsmError::NasmNotFound) => {
                self.status = Status::NasmMissing;
            }
            Err(AsmError::Failed { diagnostics }) => {
                self.diagnostics = diagnostics;
                self.assemble_state = None;
                self.emulator = None;
                self.status = Status::AssembleFailed;
            }
            Err(AsmError::Io(e)) => {
                self.status = Status::Error(if self.lang == Lang::Fr {
                    format!("erreur d'entrée/sortie : {e}")
                } else {
                    format!("I/O error: {e}")
                });
            }
        }
    }

    fn rebuild_emulator(&mut self) {
        let Some(state) = &self.assemble_state else {
            return;
        };
        let mut mem = namsdec_core::Memory::new(BASE_ADDR, MEM_SIZE);
        if mem.load(BASE_ADDR, &state.output.bytes).is_err() {
            self.status = Status::Error(strings(self.lang).binary_too_large.to_string());
            self.emulator = None;
            return;
        }
        let mut cpu = namsdec_core::Cpu::new(mem);
        cpu.regs.rip = BASE_ADDR;
        cpu.regs
            .set64(Reg64::Rsp, BASE_ADDR + MEM_SIZE as u64 - 0x100);

        let mut emulator = Emulator::new(cpu);
        emulator.breakpoints = self
            .breakpoint_lines
            .iter()
            .filter_map(|l| state.output.line_to_addr.get(l).copied())
            .collect();

        let ctx = ReContext {
            cpu: &emulator.cpu,
            disasm: &state.disasm,
            source: &self.source,
        };
        self.re_registry.on_binary_loaded(&ctx);

        self.emulator = Some(emulator);
    }

    fn sync_breakpoints(&mut self) {
        let Some(state) = &self.assemble_state else {
            return;
        };
        let addrs: BTreeSet<u64> = self
            .breakpoint_lines
            .iter()
            .filter_map(|l| state.output.line_to_addr.get(l).copied())
            .collect();
        if let Some(emulator) = &mut self.emulator {
            emulator.breakpoints = addrs;
        }
    }

    fn notify_step(&mut self) {
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            return;
        };
        let ctx = ReContext {
            cpu: &emulator.cpu,
            disasm: &state.disasm,
            source: &self.source,
        };
        self.re_registry.on_step(&ctx);
    }

    fn do_step(&mut self) {
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match emulator.step() {
            Ok(StepResult::Halted) => self.status = Status::Halted,
            Ok(StepResult::Continued) => {}
            Err(e) => self.status = Status::Error(e.to_string()),
        }
        self.notify_step();
    }

    fn do_run(&mut self) {
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match emulator.run(MAX_RUN_STEPS) {
            Ok(RunStop::Halted) => self.status = Status::Halted,
            Ok(RunStop::Breakpoint(addr)) => self.status = Status::Breakpoint(addr),
            Ok(RunStop::StepLimit) => self.status = Status::StepLimit,
            Err(e) => self.status = Status::Error(e.to_string()),
        }
        self.notify_step();
    }

    fn current_line(&self) -> Option<u32> {
        let emulator = self.emulator.as_ref()?;
        let state = self.assemble_state.as_ref()?;
        state.addr_to_line.get(&emulator.cpu.regs.rip).copied()
    }

    fn menu_bar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal(|ui| {
            ui.menu_button(s.menu_file, |ui| {
                if ui.button(s.file_preferences).clicked() {
                    self.show_preferences = true;
                    ui.close();
                }
                ui.separator();
                if ui.button(s.file_quit).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button(s.menu_edit, |ui| {
                if ui.button(s.edit_copy_code).clicked() {
                    ui.ctx().copy_text(self.source.clone());
                    ui.close();
                }
                if ui.button(s.edit_paste).clicked() {
                    match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                        Ok(text) => {
                            self.source = text;
                            self.status = Status::Idle;
                        }
                        Err(e) => {
                            self.status = Status::Error(if self.lang == Lang::Fr {
                                format!("presse-papiers inaccessible : {e}")
                            } else {
                                format!("clipboard unavailable: {e}")
                            });
                        }
                    }
                    ui.close();
                }
            });

            ui.menu_button(s.menu_tools, |ui| {
                if ui.button(s.action_assemble).clicked() {
                    self.do_assemble();
                    ui.close();
                }
                let can_exec = self.emulator.is_some();
                ui.add_enabled_ui(can_exec, |ui| {
                    if ui.button(format!("⏭ {}", s.action_step)).clicked() {
                        self.do_step();
                        ui.close();
                    }
                    if ui.button(format!("⏵ {}", s.action_run)).clicked() {
                        self.do_run();
                        ui.close();
                    }
                    if ui.button(format!("⟲ {}", s.action_reset)).clicked() {
                        self.rebuild_emulator();
                        self.status = Status::Idle;
                        ui.close();
                    }
                });
            });

            ui.menu_button(s.menu_view, |ui| {
                for tab in SidebarTab::ALL {
                    if ui.button(tab.label(s)).clicked() {
                        self.active_tab = tab;
                        ui.close();
                    }
                }
            });

            ui.menu_button(s.menu_help, |ui| {
                if ui.button(s.help_about).clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }

    fn preferences_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let mut open = self.show_preferences;
        egui::Window::new(s.preferences_title)
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(s.preferences_theme);
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.theme == AppTheme::Dark, s.theme_dark)
                        .clicked()
                    {
                        self.theme = AppTheme::Dark;
                        self.apply_theme(ctx);
                    }
                    if ui
                        .selectable_label(self.theme == AppTheme::Light, s.theme_light)
                        .clicked()
                    {
                        self.theme = AppTheme::Light;
                        self.apply_theme(ctx);
                    }
                });

                ui.separator();

                ui.label(s.preferences_language);
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.lang == Lang::Fr, "Français").clicked() {
                        self.lang = Lang::Fr;
                    }
                    if ui.selectable_label(self.lang == Lang::En, "English").clicked() {
                        self.lang = Lang::En;
                    }
                });
            });
        self.show_preferences = open;
    }

    fn about_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let mut open = self.show_about;
        egui::Window::new(s.about_title)
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Namsdec");
                ui.label(format!(
                    "Version {APP_VERSION} ({} {BUILD_DATE})",
                    s.about_build_word
                ));
                ui.label(s.about_license);
                ui.separator();
                ui.label(s.about_description);
            });
        self.show_about = open;
    }

    fn toolbar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal(|ui| {
            ui.heading("Namsdec");
            ui.separator();

            if ui.button(s.action_assemble).clicked() {
                self.do_assemble();
            }
            let can_exec = self.emulator.is_some();
            ui.add_enabled_ui(can_exec, |ui| {
                if ui.button(format!("⏭ {}", s.action_step)).clicked() {
                    self.do_step();
                }
                if ui.button(format!("⏵ {}", s.action_run)).clicked() {
                    self.do_run();
                }
                if ui.button(format!("⟲ {}", s.action_reset)).clicked() {
                    self.rebuild_emulator();
                    self.status = Status::Idle;
                }
            });

            ui.separator();
            match &self.nasm_info {
                Some(info) => {
                    status_dot(ui, GOOD);
                    ui.label(&info.version);
                }
                None => {
                    status_dot(ui, BAD);
                    ui.label(s.nasm_not_found);
                }
            }
        });
    }

    fn statusbar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let lang = self.lang;
        ui.horizontal(|ui| match &self.status {
            Status::Idle => {
                ui.label(s.status_ready);
            }
            Status::AssembleOk { bytes } => {
                ui.colored_label(
                    GOOD,
                    if lang == Lang::Fr {
                        format!("✓ Assemblage réussi — {bytes} octets")
                    } else {
                        format!("✓ Assembled successfully — {bytes} bytes")
                    },
                );
            }
            Status::AssembleFailed => {
                ui.colored_label(
                    BAD,
                    if lang == Lang::Fr {
                        format!(
                            "✗ Échec de l'assemblage — {} diagnostic(s)",
                            self.diagnostics.len()
                        )
                    } else {
                        format!(
                            "✗ Assembly failed — {} diagnostic(s)",
                            self.diagnostics.len()
                        )
                    },
                );
            }
            Status::Halted => {
                ui.colored_label(GOOD, s.status_halted);
            }
            Status::Breakpoint(addr) => {
                ui.colored_label(
                    ACCENT,
                    if lang == Lang::Fr {
                        format!("Arrêté sur point d'arrêt — {addr:#x}")
                    } else {
                        format!("Stopped at breakpoint — {addr:#x}")
                    },
                );
            }
            Status::StepLimit => {
                ui.colored_label(BAD, s.status_step_limit);
            }
            Status::NasmMissing => {
                ui.colored_label(BAD, s.status_nasm_missing);
            }
            Status::Error(msg) => {
                ui.colored_label(
                    BAD,
                    if lang == Lang::Fr {
                        format!("Erreur : {msg}")
                    } else {
                        format!("Error: {msg}")
                    },
                );
            }
        });

        if !self.diagnostics.is_empty() {
            egui::ScrollArea::vertical()
                .max_height(96.0)
                .show(ui, |ui| {
                    for d in &self.diagnostics {
                        let color = if d.severity == Severity::Error { BAD } else { WARN };
                        let text = if lang == Lang::Fr {
                            format!("ligne {} — {}", d.line, d.message)
                        } else {
                            format!("line {} — {}", d.line, d.message)
                        };
                        ui.colored_label(color, text);
                    }
                });
        }
    }

    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal(|ui| {
            ui.label(s.breakpoint_on_line);
            ui.add(egui::DragValue::new(&mut self.bp_input).range(1..=9999));
            if ui.button(s.add).clicked() {
                self.breakpoint_lines.insert(self.bp_input);
                self.sync_breakpoints();
            }
            ui.separator();
            let mut to_remove = None;
            for line in &self.breakpoint_lines {
                if ui.button(format!("L{line} ✕")).clicked() {
                    to_remove = Some(*line);
                }
            }
            if let Some(line) = to_remove {
                self.breakpoint_lines.remove(&line);
                self.sync_breakpoints();
            }
        });

        if let Some(line) = self.current_line() {
            let text = if self.lang == Lang::Fr {
                format!("▶ Ligne courante : {line}")
            } else {
                format!("▶ Current line: {line}")
            };
            ui.colored_label(ACCENT, text);
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.source)
                    .desired_width(f32::INFINITY)
                    .desired_rows(24)
                    .code_editor(),
            );
        });
    }

    fn sidebar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal(|ui| {
            for tab in SidebarTab::ALL {
                if ui
                    .selectable_label(self.active_tab == tab, tab.label(s))
                    .clicked()
                {
                    self.active_tab = tab;
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
            SidebarTab::Registers => self.registers_ui(ui),
            SidebarTab::Flags => self.flags_ui(ui),
            SidebarTab::Stack => self.stack_ui(ui),
            SidebarTab::Memory => self.memory_ui(ui),
            SidebarTab::Re => self.re_ui(ui),
            SidebarTab::Modules => self.modules_ui(ui),
        });
    }

    fn registers_ui(&self, ui: &mut egui::Ui) {
        let Some(emulator) = &self.emulator else {
            ui.label(strings(self.lang).no_binary_assembled);
            return;
        };
        let regs = &emulator.cpu.regs;
        egui::Grid::new("registers_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for (name, value) in [
                    ("RAX", regs.get64(Reg64::Rax)),
                    ("RBX", regs.get64(Reg64::Rbx)),
                    ("RCX", regs.get64(Reg64::Rcx)),
                    ("RDX", regs.get64(Reg64::Rdx)),
                    ("RSI", regs.get64(Reg64::Rsi)),
                    ("RDI", regs.get64(Reg64::Rdi)),
                    ("RSP", regs.get64(Reg64::Rsp)),
                    ("RBP", regs.get64(Reg64::Rbp)),
                    ("RIP", regs.rip),
                ] {
                    ui.monospace(name);
                    ui.monospace(format!("{value:016x}"));
                    ui.end_row();
                }
            });
    }

    fn flags_ui(&self, ui: &mut egui::Ui) {
        let Some(emulator) = &self.emulator else {
            ui.label(strings(self.lang).no_binary_assembled);
            return;
        };
        let f = emulator.cpu.flags;
        for (name, bit) in [
            ("CF", RFlags::CF),
            ("PF", RFlags::PF),
            ("AF", RFlags::AF),
            ("ZF", RFlags::ZF),
            ("SF", RFlags::SF),
            ("OF", RFlags::OF),
        ] {
            ui.monospace(format!("{name}  {}", u8::from(f.contains(bit))));
        }
    }

    fn stack_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            ui.label(s.no_binary_assembled);
            return;
        };
        let addr = emulator.cpu.regs.get64(Reg64::Rsp).saturating_sub(16);
        ui.monospace(safe_hexdump(&emulator.cpu.mem, addr, 64, s.address_out_of_range));
    }

    fn memory_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            ui.label(s.no_binary_assembled);
            return;
        };
        ui.horizontal(|ui| {
            ui.label(s.memory_address_label);
            ui.text_edit_singleline(&mut self.mem_addr_input);
        });
        let addr = parse_addr(&self.mem_addr_input).unwrap_or(BASE_ADDR);
        ui.monospace(safe_hexdump(&emulator.cpu.mem, addr, 128, s.address_out_of_range));
    }

    fn re_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            ui.label(s.no_binary_assembled);
            return;
        };
        let ctx = ReContext {
            cpu: &emulator.cpu,
            disasm: &state.disasm,
            source: &self.source,
        };
        for module in self.re_registry.iter_mut() {
            ui.group(|ui| {
                ui.strong(module.title());
                module.ui(ui, &ctx);
            });
        }
    }

    /// Vue d'ensemble des crates moteur (core/asm/disasm/emu/re), un peu
    /// comme un gestionnaire de plugins — namsdec-exercises n'y figure pas :
    /// il deviendra un module séparé, ajoutable indépendamment.
    fn modules_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let fr = self.lang == Lang::Fr;

        module_card(
            ui,
            "namsdec-core",
            namsdec_core::VERSION,
            true,
            GOOD,
            s.module_core_detail,
        );

        let (asm_ok, asm_status) = match &self.nasm_info {
            Some(info) => (
                true,
                if fr {
                    format!("nasm détecté — {}", info.version)
                } else {
                    format!("nasm detected — {}", info.version)
                },
            ),
            None => (
                false,
                if fr {
                    "nasm introuvable dans le PATH".to_string()
                } else {
                    "nasm not found in PATH".to_string()
                },
            ),
        };
        module_card(
            ui,
            "namsdec-asm",
            namsdec_asm::VERSION,
            asm_ok,
            if asm_ok { GOOD } else { BAD },
            &asm_status,
        );

        let disasm_status = match &self.assemble_state {
            Some(state) => {
                if fr {
                    format!("{} instructions désassemblées (vérification)", state.disasm.len())
                } else {
                    format!("{} instructions disassembled (verification)", state.disasm.len())
                }
            }
            None => s.no_binary_assembled_yet.to_string(),
        };
        module_card(
            ui,
            "namsdec-disasm",
            namsdec_disasm::VERSION,
            self.assemble_state.is_some(),
            ACCENT,
            &disasm_status,
        );

        let emu_status = match &self.emulator {
            Some(emulator) => {
                let state_word = match (emulator.cpu.halted, fr) {
                    (true, true) => "arrêté (hlt)",
                    (true, false) => "halted (hlt)",
                    (false, true) => "en cours",
                    (false, false) => "running",
                };
                let bp_word = if fr { "point(s) d'arrêt" } else { "breakpoint(s)" };
                format!(
                    "RIP={:#x} · {} {bp_word} · {state_word}",
                    emulator.cpu.regs.rip,
                    emulator.breakpoints.len(),
                )
            }
            None => s.no_program_loaded.to_string(),
        };
        module_card(
            ui,
            "namsdec-emu",
            namsdec_emu::VERSION,
            self.emulator.is_some(),
            ACCENT,
            &emu_status,
        );

        let titles = self.re_registry.titles();
        let re_status = if titles.is_empty() {
            s.no_re_module_registered.to_string()
        } else if fr {
            format!("{} module(s) : {}", titles.len(), titles.join(", "))
        } else {
            format!("{} module(s): {}", titles.len(), titles.join(", "))
        };
        module_card(ui, "namsdec-re", namsdec_re::VERSION, true, GOOD, &re_status);
    }
}

/// Petit disque de statut peint directement (plutôt qu'un glyphe texte
/// "●", que la police par défaut d'egui ne contient pas et qui retombe
/// sur un carré de remplacement).
fn status_dot(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// Une ligne de la vue d'ensemble des modules : nom, version, point coloré
/// de statut, et une courte description de ce que fait ce module là,
/// maintenant.
fn module_card(
    ui: &mut egui::Ui,
    name: &str,
    version: &str,
    active: bool,
    color: egui::Color32,
    detail: &str,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            status_dot(ui, if active { color } else { egui::Color32::GRAY });
            ui.strong(name);
            ui.weak(format!("v{version}"));
        });
        ui.label(detail);
    });
    ui.add_space(4.0);
}

impl eframe::App for NamsdecApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("menu_bar").show(ui, |ui| self.menu_bar_ui(ui));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar_ui(ui));
        egui::Panel::bottom("statusbar").show(ui, |ui| self.statusbar_ui(ui));
        egui::Panel::right("sidebar")
            .resizable(true)
            .default_size(300.0)
            .show(ui, |ui| self.sidebar_ui(ui));
        self.editor_ui(ui);

        self.preferences_window_ui(ui.ctx());
        self.about_window_ui(ui.ctx());
    }
}

fn parse_addr(s: &str) -> Option<u64> {
    let s = s.trim();
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    u64::from_str_radix(s, 16).ok()
}

fn safe_hexdump(mem: &namsdec_core::Memory, addr: u64, len: usize, out_of_range: &str) -> String {
    let range = mem.range();
    if addr < range.start || addr >= range.end {
        return out_of_range.to_string();
    }
    let len = len.min((range.end - addr) as usize);
    match mem.bytes_at(addr, len) {
        Ok(bytes) => hexdump(bytes, addr),
        Err(_) => out_of_range.to_string(),
    }
}

fn hexdump(bytes: &[u8], base_addr: u64) -> String {
    let mut out = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let addr = base_addr + (i * 16) as u64;
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
            .collect();
        out.push_str(&format!("{addr:08x}  {:<47}  {ascii}\n", hex.join(" ")));
    }
    out
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Namsdec",
        options,
        Box::new(|cc| {
            let app = NamsdecApp::default();
            app.apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
