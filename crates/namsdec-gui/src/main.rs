//! Coquille eframe/egui du MVP d'apprentissage NASM. Assemble via
//! `namsdec-asm` (nasm externe), désassemble via `namsdec-disasm`
//! (vérification pédagogique) et exécute pas à pas via `namsdec-emu`,
//! en affichant les panneaux des modules `ReModule` enregistrés.

mod explorer;
mod i18n;
mod keybindings;
mod syntax;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use eframe::egui;
use namsdec_asm::{AsmError, Diagnostic, Severity};
use namsdec_core::{Reg64, RFlags};
use namsdec_disasm::DecodedInstruction;
use namsdec_emu::{Emulator, RunStop, StepOverResult, StepResult};
use namsdec_re::{BinaryInfoModule, ReContext, ReModuleRegistry};

use explorer::{ExplorerAction, FileExplorer};
use i18n::{strings, Lang, Strings};
use keybindings::{Action, Dispatcher, KeyBindings, Rebinding};

const BASE_ADDR: u64 = 0x400000;
const MEM_SIZE: usize = 0x10000;
const MAX_RUN_STEPS: u32 = 1_000_000;
/// Nombre de pas exécutés par frame pendant un Run — assez petit pour que
/// Stop réagisse vite (au prochain batch), assez grand pour que la plupart
/// des programmes finissent en une poignée de frames.
const RUN_BATCH_STEPS: u32 = 5_000;
/// Plafond de sécurité total, réparti sur tous les batches d'un même Run —
/// distinct de RUN_BATCH_STEPS, qui ne borne qu'un seul batch.
const RUN_TOTAL_STEPS_CAP: u64 = 50_000_000;

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

        // Coins arrondis partout — une des choses les plus visibles qui
        // distinguent un look "moderne" du carré par défaut d'egui.
        visuals.window_corner_radius = egui::CornerRadius::from(10);
        visuals.menu_corner_radius = egui::CornerRadius::from(8);
        let widget_radius = egui::CornerRadius::from(6);
        visuals.widgets.noninteractive.corner_radius = widget_radius;
        visuals.widgets.inactive.corner_radius = widget_radius;
        visuals.widgets.hovered.corner_radius = widget_radius;
        visuals.widgets.active.corner_radius = widget_radius;
        visuals.widgets.open.corner_radius = widget_radius;

        // Boutons plats au repos (fond et bordure transparents, comme dans
        // la maquette HTML — `.tool { background: transparent; border:
        // 1px solid transparent; }`, coloré seulement au survol) : le
        // style par défaut d'egui donne à CHAQUE bouton un fond visible en
        // permanence, ce qui fait que la barre de menus (Fichier/Édition/…)
        // et la barre d'outils (icônes) finissent par se ressembler — une
        // rangée de boîtes uniformes sans hiérarchie visuelle entre elles.
        visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

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
}

impl SidebarTab {
    const ALL: [SidebarTab; 5] = [
        SidebarTab::Registers,
        SidebarTab::Flags,
        SidebarTab::Stack,
        SidebarTab::Memory,
        SidebarTab::Re,
    ];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            SidebarTab::Registers => s.tab_registers,
            SidebarTab::Flags => s.tab_flags,
            SidebarTab::Stack => s.tab_stack,
            SidebarTab::Memory => s.tab_memory,
            SidebarTab::Re => s.tab_re,
        }
    }
}

enum Status {
    Idle,
    AssembleOk { bytes: usize },
    AssembleFailed,
    Running,
    Halted,
    Breakpoint(u64),
    StepLimit,
    NasmMissing,
    Error(String),
}

/// Mode d'édition façon (n)vim : en Normal, l'éditeur ne prend pas la
/// frappe comme du texte (`i`/`I` pour passer en Insertion) ; en
/// Insertion, il se comporte comme un éditeur de texte classique
/// (`Échap` pour revenir en Normal). Volontairement limité à ces deux
/// modes et ces trois touches — pas les mouvements hjkl ni les commandes
/// (dd, yy, ...) de vim, qui n'ont pas été demandés.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum EditorMode {
    #[default]
    Normal,
    Insert,
}

impl EditorMode {
    fn label(self) -> &'static str {
        // Comme dans (n)vim lui-même : jamais traduit, y compris en UI
        // anglaise — c'est la convention, pas un texte applicatif.
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
        }
    }

    fn color(self) -> egui::Color32 {
        match self {
            EditorMode::Normal => ACCENT,
            EditorMode::Insert => GOOD,
        }
    }
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
    assemble_state: Option<AssembleState>,
    emulator: Option<Emulator>,
    re_registry: ReModuleRegistry,
    status: Status,
    diagnostics: Vec<Diagnostic>,
    active_tab: SidebarTab,
    mem_addr_input: String,
    show_preferences: bool,
    show_about: bool,
    keybindings: KeyBindings,
    dispatcher: Dispatcher,
    rebinding: Option<Rebinding>,
    running: bool,
    run_steps_taken: u64,
    editor_focused: bool,
    editor_mode: EditorMode,
    preferences_tab: PreferencesTab,
    show_re_tab: bool,
    explorer: FileExplorer,
    show_explorer: bool,
    /// L'explorateur a-t-il le focus clavier ? Comme dans nvim, les touches
    /// vont soit à l'arborescence (j/k/l/h…), soit à l'éditeur, jamais aux
    /// deux.
    explorer_focused: bool,
    current_file: Option<PathBuf>,
    show_toolbar: bool,
    show_sidebar: bool,
    show_statusbar: bool,
    command_palette: Option<CommandPalette>,
}

#[derive(Default)]
struct CommandPalette {
    query: String,
    selected: usize,
}

/// Tout ce que la palette de commandes (Ctrl+Maj+P) peut déclencher —
/// délibérément large ("toutes les fonctions du programme"), pas limité
/// aux huit `Action` liées à un raccourci.
#[derive(Clone, Copy)]
enum PaletteCommand {
    Do(Action),
    ShowPreferences,
    ShowAbout,
    SetTheme(AppTheme),
    SetLang(Lang),
    OpenFolder,
    OpenFile,
    SaveAs,
    Quit,
    GoTab(SidebarTab),
    ToggleToolbar,
    ToggleSidebar,
    ToggleStatusbar,
}

fn palette_commands(s: &Strings) -> Vec<(String, PaletteCommand)> {
    vec![
        (s.action_assemble.to_string(), PaletteCommand::Do(Action::Assemble)),
        (s.action_run.to_string(), PaletteCommand::Do(Action::Run)),
        (s.action_step_into.to_string(), PaletteCommand::Do(Action::StepInto)),
        (s.action_step_over.to_string(), PaletteCommand::Do(Action::StepOver)),
        (s.action_stop.to_string(), PaletteCommand::Do(Action::Stop)),
        (s.action_reset.to_string(), PaletteCommand::Do(Action::Reset)),
        (s.action_toggle_explorer.to_string(), PaletteCommand::Do(Action::ToggleExplorer)),
        (s.action_save.to_string(), PaletteCommand::Do(Action::Save)),
        (s.file_save_as.to_string(), PaletteCommand::SaveAs),
        (s.file_open_folder.to_string(), PaletteCommand::OpenFolder),
        (s.file_open_file.to_string(), PaletteCommand::OpenFile),
        (s.file_preferences.to_string(), PaletteCommand::ShowPreferences),
        (s.help_about.to_string(), PaletteCommand::ShowAbout),
        (format!("{}: {}", s.preferences_theme, s.theme_dark), PaletteCommand::SetTheme(AppTheme::Dark)),
        (format!("{}: {}", s.preferences_theme, s.theme_light), PaletteCommand::SetTheme(AppTheme::Light)),
        (format!("{}: Français", s.preferences_language), PaletteCommand::SetLang(Lang::Fr)),
        (format!("{}: English", s.preferences_language), PaletteCommand::SetLang(Lang::En)),
        (format!("{}: {}", s.menu_view, s.tab_registers), PaletteCommand::GoTab(SidebarTab::Registers)),
        (format!("{}: {}", s.menu_view, s.tab_flags), PaletteCommand::GoTab(SidebarTab::Flags)),
        (format!("{}: {}", s.menu_view, s.tab_stack), PaletteCommand::GoTab(SidebarTab::Stack)),
        (format!("{}: {}", s.menu_view, s.tab_memory), PaletteCommand::GoTab(SidebarTab::Memory)),
        (format!("{}: {}", s.menu_view, s.tab_re), PaletteCommand::GoTab(SidebarTab::Re)),
        (s.palette_toggle_toolbar.to_string(), PaletteCommand::ToggleToolbar),
        (s.palette_toggle_sidebar.to_string(), PaletteCommand::ToggleSidebar),
        (s.palette_toggle_statusbar.to_string(), PaletteCommand::ToggleStatusbar),
        (s.file_quit.to_string(), PaletteCommand::Quit),
    ]
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreferencesTab {
    General,
    Shortcuts,
    Modules,
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
            assemble_state: None,
            emulator: None,
            re_registry,
            status: Status::Idle,
            diagnostics: Vec::new(),
            active_tab: SidebarTab::Registers,
            mem_addr_input: format!("{BASE_ADDR:#x}"),
            show_preferences: false,
            show_about: false,
            keybindings: KeyBindings::default(),
            dispatcher: Dispatcher::default(),
            rebinding: None,
            running: false,
            run_steps_taken: 0,
            editor_focused: false,
            editor_mode: EditorMode::Normal,
            preferences_tab: PreferencesTab::General,
            show_re_tab: false,
            explorer: FileExplorer::default(),
            show_explorer: false,
            explorer_focused: false,
            current_file: None,
            show_toolbar: true,
            show_sidebar: true,
            show_statusbar: true,
            command_palette: None,
        }
    }
}

impl NamsdecApp {
    fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.theme.visuals());
        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);
            style.spacing.button_padding = egui::vec2(10.0, 5.0);
            style.spacing.window_margin = egui::Margin::from(12);
        });
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

    /// Démarre un Run — ne bloque plus le thread UI jusqu'à la fin :
    /// `tick_run` avance par petits paquets, une frame à la fois, pour que
    /// Stop reste réactif.
    fn do_run(&mut self) {
        if self.emulator.is_none() {
            return;
        }
        self.running = true;
        self.run_steps_taken = 0;
        self.status = Status::Running;
    }

    /// À appeler une fois par frame. N'avance que si un Run est en cours.
    fn tick_run(&mut self, ctx: &egui::Context) {
        if !self.running {
            return;
        }
        let Some(emulator) = &mut self.emulator else {
            self.running = false;
            return;
        };
        match emulator.run(RUN_BATCH_STEPS) {
            Ok(RunStop::Halted) => {
                self.running = false;
                self.status = Status::Halted;
            }
            Ok(RunStop::Breakpoint(addr)) => {
                self.running = false;
                self.status = Status::Breakpoint(addr);
            }
            Ok(RunStop::StepLimit) => {
                self.run_steps_taken += RUN_BATCH_STEPS as u64;
                if self.run_steps_taken >= RUN_TOTAL_STEPS_CAP {
                    self.running = false;
                    self.status = Status::StepLimit;
                }
            }
            Err(e) => {
                self.running = false;
                self.status = Status::Error(e.to_string());
            }
        }
        self.notify_step();
        if self.running {
            // egui ne redessine que sur un événement d'entrée par défaut :
            // il faut forcer des frames tant qu'un Run est en cours.
            ctx.request_repaint();
        }
    }

    /// Interrompt un Run en cours. Prend effet au prochain batch (latence
    /// bornée par RUN_BATCH_STEPS, pas instantanée), sans exiger que
    /// `Emulator` devienne `Send`/threadable.
    fn do_stop(&mut self) {
        if self.running {
            self.running = false;
            self.status = Status::Idle;
        }
    }

    fn do_step_over(&mut self) {
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match emulator.step_over(MAX_RUN_STEPS) {
            Ok(StepOverResult::Halted) => self.status = Status::Halted,
            Ok(StepOverResult::Breakpoint(addr)) => self.status = Status::Breakpoint(addr),
            Ok(StepOverResult::StepLimit) => self.status = Status::StepLimit,
            Ok(StepOverResult::Stepped | StepOverResult::SteppedOverCall) => {}
            Err(e) => self.status = Status::Error(e.to_string()),
        }
        self.notify_step();
    }

    /// Point d'entrée unique pour tout ce qui déclenche une action —
    /// raccourci clavier ou clic de bouton/menu, pour ne jamais avoir deux
    /// implémentations de "ce que fait Assembler" qui divergent.
    fn dispatch(&mut self, action: Action) {
        match action {
            Action::ToggleExplorer => {
                self.show_explorer = !self.show_explorer;
                // L'ouvrir lui donne le focus (comme `:Neotree focus`) ;
                // le refermer le rend à l'éditeur.
                self.focus_explorer(self.show_explorer);
            }
            Action::Assemble => self.do_assemble(),
            Action::Run => self.do_run(),
            Action::StepInto => self.do_step(),
            Action::StepOver => self.do_step_over(),
            Action::Stop => self.do_stop(),
            Action::Reset => {
                self.rebuild_emulator();
                self.status = Status::Idle;
            }
            Action::Save => self.do_save(),
        }
    }

    fn do_open_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.explorer.root = Some(folder);
            self.show_explorer = true;
        }
    }

    fn do_open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_file(path);
        }
    }

    /// Charge `path` dans l'éditeur — n'importe quel fichier texte, pas
    /// seulement les `.asm` (comme VS Code, qui n'écarte pas les fichiers
    /// non reconnus). Une erreur d'E/S ou un contenu non-UTF8 finit dans
    /// la barre de statut, jamais en panique.
    fn open_file(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.source = text;
                self.current_file = Some(path);
                self.assemble_state = None;
                self.emulator = None;
                self.diagnostics.clear();
                self.status = Status::Idle;
            }
            Err(e) => {
                self.status = Status::Error(if self.lang == Lang::Fr {
                    format!("impossible d'ouvrir le fichier : {e}")
                } else {
                    format!("could not open file: {e}")
                });
            }
        }
    }

    /// Donner le focus à l'explorateur repasse forcément l'éditeur en mode
    /// Normal : sinon le `TextEdit` garderait le focus clavier d'egui et
    /// chaque `j`/`k` de navigation serait *aussi* tapé dans le code.
    fn focus_explorer(&mut self, focused: bool) {
        self.explorer_focused = focused;
        if focused {
            self.editor_mode = EditorMode::Normal;
        }
    }

    /// L'arborescence elle-même, ou une invite claire tant qu'aucun dossier
    /// n'a été choisi — un panneau simplement vide se lit comme cassé.
    fn explorer_panel_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        if self.explorer.root.is_none() {
            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                ui.label(s.explorer_empty);
                ui.add_space(6.0);
                if ui.button(s.file_open_folder).clicked() {
                    self.do_open_folder();
                }
            });
            return;
        }
        // Cliquer dans le panneau lui donne le focus clavier.
        if ui.ui_contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
            self.focus_explorer(true);
        }

        let focused = self.explorer_focused;
        match self.explorer.ui(ui, focused, s.explorer_filter_hint) {
            ExplorerAction::OpenFile(path) => {
                self.open_file(path);
                // Ouvrir un fichier rend la main à l'éditeur, comme dans
                // neo-tree où la sélection d'un fichier saute au buffer.
                self.explorer_focused = false;
            }
            ExplorerAction::Defocus => self.explorer_focused = false,
            ExplorerAction::None => {}
        }
    }

    fn execute_palette_command(&mut self, ctx: &egui::Context, cmd: PaletteCommand) {
        match cmd {
            PaletteCommand::Do(action) => self.dispatch(action),
            PaletteCommand::ShowPreferences => self.show_preferences = true,
            PaletteCommand::ShowAbout => self.show_about = true,
            PaletteCommand::SetTheme(theme) => {
                self.theme = theme;
                self.apply_theme(ctx);
            }
            PaletteCommand::SetLang(lang) => self.lang = lang,
            PaletteCommand::OpenFolder => self.do_open_folder(),
            PaletteCommand::OpenFile => self.do_open_file_dialog(),
            PaletteCommand::SaveAs => self.do_save_as(),
            PaletteCommand::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            PaletteCommand::GoTab(tab) => {
                if tab == SidebarTab::Re {
                    self.show_re_tab = true;
                }
                self.show_sidebar = true;
                self.active_tab = tab;
            }
            PaletteCommand::ToggleToolbar => self.show_toolbar = !self.show_toolbar,
            PaletteCommand::ToggleSidebar => self.show_sidebar = !self.show_sidebar,
            PaletteCommand::ToggleStatusbar => self.show_statusbar = !self.show_statusbar,
        }
    }

    /// Palette de commandes (Ctrl+Maj+P, ou le raccourci ré-armé depuis
    /// n'importe quel état puisqu'elle bascule plutôt que de seulement
    /// s'ouvrir) : recherche par sous-chaîne sur toutes les fonctions du
    /// programme, navigation clavier haut/bas, Entrée pour exécuter.
    fn command_palette_ui(&mut self, ctx: &egui::Context) {
        if self.command_palette.is_none() {
            return;
        }
        let s = strings(self.lang);
        let commands = palette_commands(s);

        let (mut query, mut selected) = {
            let p = self.command_palette.as_ref().unwrap();
            (p.query.clone(), p.selected)
        };

        let query_lower = query.to_lowercase();
        let filtered: Vec<&(String, PaletteCommand)> = commands
            .iter()
            .filter(|(label, _)| query_lower.is_empty() || label.to_lowercase().contains(&query_lower))
            .collect();
        if filtered.is_empty() {
            selected = 0;
        } else {
            selected = selected.min(filtered.len() - 1);
        }

        let mut close = false;
        let mut execute: Option<PaletteCommand> = None;

        egui::Window::new("command_palette")
            .id(egui::Id::new("command_palette"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
            .fixed_size(egui::vec2(480.0, 340.0))
            .show(ctx, |ui| {
                let text_response = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text(s.palette_placeholder)
                        .desired_width(f32::INFINITY),
                );
                if !text_response.has_focus() {
                    text_response.request_focus();
                }
                ui.separator();
                egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                    if filtered.is_empty() {
                        ui.weak(s.palette_no_match);
                    }
                    for (i, (label, cmd)) in filtered.iter().enumerate() {
                        if ui.selectable_label(i == selected, label.as_str()).clicked() {
                            execute = Some(*cmd);
                        }
                    }
                });
            });

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                close = true;
            }
            if i.key_pressed(egui::Key::ArrowDown) && !filtered.is_empty() {
                selected = (selected + 1).min(filtered.len() - 1);
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                selected = selected.saturating_sub(1);
            }
            if i.key_pressed(egui::Key::Enter)
                && let Some((_, cmd)) = filtered.get(selected)
            {
                execute = Some(*cmd);
            }
        });

        if close {
            self.command_palette = None;
            return;
        }
        if let Some(cmd) = execute {
            self.command_palette = None;
            self.execute_palette_command(ctx, cmd);
            return;
        }
        if let Some(p) = &mut self.command_palette {
            p.query = query;
            p.selected = selected;
        }
    }

    fn do_save(&mut self) {
        match self.current_file.clone() {
            Some(path) => self.write_to(&path),
            None => self.do_save_as(),
        }
    }

    fn do_save_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new().save_file() {
            self.write_to(&path);
        }
    }

    fn write_to(&mut self, path: &Path) {
        match std::fs::write(path, &self.source) {
            Ok(()) => {
                self.current_file = Some(path.to_path_buf());
            }
            Err(e) => {
                self.status = Status::Error(if self.lang == Lang::Fr {
                    format!("impossible d'enregistrer : {e}")
                } else {
                    format!("could not save: {e}")
                });
            }
        }
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
                if ui.button(s.file_open_folder).clicked() {
                    self.do_open_folder();
                    ui.close();
                }
                if ui.button(s.file_open_file).clicked() {
                    self.do_open_file_dialog();
                    ui.close();
                }
                if ui
                    .button(shortcut_hint(&self.keybindings, &ui.ctx().clone(), s.action_save, Action::Save))
                    .clicked()
                {
                    self.do_save();
                    ui.close();
                }
                if ui.button(s.file_save_as).clicked() {
                    self.do_save_as();
                    ui.close();
                }
                ui.separator();
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
                    self.dispatch(Action::Assemble);
                    ui.close();
                }
                let can_exec = self.emulator.is_some();
                ui.add_enabled_ui(can_exec, |ui| {
                    if ui.button(s.action_step_into).clicked() {
                        self.dispatch(Action::StepInto);
                        ui.close();
                    }
                    if ui.button(s.action_step_over).clicked() {
                        self.dispatch(Action::StepOver);
                        ui.close();
                    }
                    if ui.button(s.action_run).clicked() {
                        self.dispatch(Action::Run);
                        ui.close();
                    }
                    ui.add_enabled_ui(self.running, |ui| {
                        if ui.button(s.action_stop).clicked() {
                            self.dispatch(Action::Stop);
                            ui.close();
                        }
                    });
                    if ui.button(s.action_reset).clicked() {
                        self.dispatch(Action::Reset);
                        ui.close();
                    }
                });
            });

            ui.menu_button(s.menu_view, |ui| {
                // Toutes les fenêtres non-dialogue sont affichables/masquables
                // ici — des cases à cocher, pas des boutons, pour pouvoir en
                // activer/désactiver plusieurs sans refermer le menu à
                // chaque fois.
                ui.checkbox(&mut self.show_toolbar, s.view_toolbar);
                ui.checkbox(&mut self.show_sidebar, s.view_sidebar);
                ui.checkbox(&mut self.show_statusbar, s.view_statusbar);
                ui.checkbox(&mut self.show_explorer, s.action_toggle_explorer);

                ui.separator();
                if ui.button(s.palette_open).clicked() {
                    self.command_palette = Some(CommandPalette::default());
                    ui.close();
                }
                ui.separator();
                for tab in SidebarTab::ALL {
                    if !self.show_re_tab && tab == SidebarTab::Re {
                        continue;
                    }
                    if ui.button(tab.label(s)).clicked() {
                        self.show_sidebar = true;
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
            .resizable(true)
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.preferences_tab,
                        PreferencesTab::General,
                        s.preferences_tab_general,
                    );
                    ui.selectable_value(
                        &mut self.preferences_tab,
                        PreferencesTab::Shortcuts,
                        s.preferences_tab_shortcuts,
                    );
                    ui.selectable_value(
                        &mut self.preferences_tab,
                        PreferencesTab::Modules,
                        s.preferences_tab_modules,
                    );
                });
                ui.separator();
                match self.preferences_tab {
                    PreferencesTab::General => self.preferences_general_ui(ui),
                    PreferencesTab::Shortcuts => self.preferences_shortcuts_ui(ui),
                    PreferencesTab::Modules => self.preferences_modules_ui(ui),
                }
            });
        self.show_preferences = open;
    }

    fn preferences_general_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let ctx = ui.ctx().clone();
        ui.label(s.preferences_theme);
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.theme == AppTheme::Dark, s.theme_dark)
                .clicked()
            {
                self.theme = AppTheme::Dark;
                self.apply_theme(&ctx);
            }
            if ui
                .selectable_label(self.theme == AppTheme::Light, s.theme_light)
                .clicked()
            {
                self.theme = AppTheme::Light;
                self.apply_theme(&ctx);
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
    }

    fn preferences_shortcuts_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let ctx = ui.ctx().clone();

        // Une capture en cours avance ici plutôt que dans le dispatch
        // normal (voir App::ui, qui saute le dispatch tant que
        // `self.rebinding` est `Some`).
        if let Some(rebinding) = &mut self.rebinding
            && let Some(chord) = self.dispatcher.poll_rebind(&ctx, rebinding) {
                let action = rebinding.action;
                self.keybindings.map.insert(action, chord);
                self.rebinding = None;
            }

        egui::Grid::new("shortcuts_grid")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                for action in Action::ALL {
                    let is_rebinding = self.rebinding.as_ref().is_some_and(|r| r.action == action);

                    ui.label(action_label(s, action));

                    if is_rebinding {
                        ui.colored_label(ACCENT, s.rebind_waiting);
                    } else {
                        let current = self
                            .keybindings
                            .map
                            .get(&action)
                            .map(|c| c.display(&ctx))
                            .unwrap_or_default();
                        ui.monospace(current);
                    }

                    if ui
                        .add_enabled(!is_rebinding, egui::Button::new(s.rebind_simple))
                        .clicked()
                    {
                        self.rebinding = Some(Rebinding::new(action, keybindings::RebindMode::Simple));
                    }
                    if ui
                        .add_enabled(!is_rebinding, egui::Button::new(s.rebind_leader))
                        .clicked()
                    {
                        self.rebinding = Some(Rebinding::new(action, keybindings::RebindMode::Leader));
                    }
                    ui.end_row();
                }
            });
    }

    fn preferences_modules_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.label(s.modules_legend);
        ui.add_space(6.0);
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            self.modules_ui(ui);
            ui.checkbox(&mut self.show_re_tab, s.show_re_tab_checkbox);
        });
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
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            if ui
                .button(s.action_assemble)
                .on_hover_text(shortcut_hint(&self.keybindings, &ctx, s.action_assemble, Action::Assemble))
                .clicked()
            {
                self.dispatch(Action::Assemble);
            }

            let can_exec = self.emulator.is_some();
            ui.add_enabled_ui(can_exec, |ui| {
                if icon_button(ui, icon_run, s.action_run)
                    .on_hover_text(shortcut_hint(&self.keybindings, &ctx, s.action_run, Action::Run))
                    .clicked()
                {
                    self.dispatch(Action::Run);
                }
                if icon_button(ui, icon_step_into, s.action_step_into)
                    .on_hover_text(shortcut_hint(&self.keybindings, &ctx, s.action_step_into, Action::StepInto))
                    .clicked()
                {
                    self.dispatch(Action::StepInto);
                }
                if icon_button(ui, icon_step_over, s.action_step_over)
                    .on_hover_text(shortcut_hint(&self.keybindings, &ctx, s.action_step_over, Action::StepOver))
                    .clicked()
                {
                    self.dispatch(Action::StepOver);
                }
            });
            ui.add_enabled_ui(self.running, |ui| {
                if icon_button(ui, icon_stop, s.action_stop)
                    .on_hover_text(shortcut_hint(&self.keybindings, &ctx, s.action_stop, Action::Stop))
                    .clicked()
                {
                    self.dispatch(Action::Stop);
                }
            });

        });
    }

    fn statusbar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let lang = self.lang;

        if let Some(key) = self.dispatcher.leader_armed() {
            let text = if lang == Lang::Fr {
                format!("{} appuyée — touche suivante attendue…", key.name())
            } else {
                format!("{} pressed — waiting for next key…", key.name())
            };
            ui.colored_label(ACCENT, text);
        }

        ui.horizontal(|ui| {
            // Compartimenté plutôt qu'un seul bloc de texte : pastille de
            // mode, séparateur, message de statut, séparateur, statut nasm
            // (version raccourcie — "NASM 3.02", pas la ligne complète de
            // `nasm -v` avec sa date de compilation, hors de propos ici).
            mode_pill_ui(ui, self.editor_mode);
            ui.separator();
            self.status_message_ui(ui);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (dot_color, text) = match &self.nasm_info {
                    Some(info) => (GOOD, short_nasm_version(&info.version)),
                    None => (BAD, s.nasm_not_found.to_string()),
                };
                ui.label(text);
                status_dot(ui, dot_color);
                ui.separator();
            });
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

    fn status_message_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let lang = self.lang;
        match &self.status {
            Status::Idle => {
                ui.label(s.status_ready);
            }
            Status::Running => {
                ui.colored_label(ACCENT, s.status_running);
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
        }
    }

    /// Un vrai éditeur : gouttière avec numéros de ligne et points d'arrêt
    /// cliquables (plus de saisie de numéro à la main), ligne courante en
    /// surbrillance. La gouttière et le `TextEdit` partagent la même
    /// `ScrollArea` pour rester synchronisés au défilement ; le
    /// retour-à-la-ligne automatique du `TextEdit` est désactivé
    /// (`desired_width(INFINITY)` + défilement horizontal disponible) pour
    /// qu'une ligne logique corresponde toujours à une seule ligne visuelle
    /// — sinon la gouttière se décale sur une ligne trop longue.
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        // Modes façon (n)vim : `i`/`I` pour passer en Insertion, `Échap`
        // pour revenir en Normal — désactivé pendant qu'une autre capture
        // de texte est active (palette de commandes, rebind de raccourci)
        // pour ne pas leur voler leurs propres touches.
        let mut just_entered_insert = false;
        if self.command_palette.is_none() && self.rebinding.is_none() && !self.explorer_focused {
            let (enter_insert, exit_insert) = ui.ctx().input(|inp| {
                let mut enter = false;
                let mut exit = false;
                for event in &inp.events {
                    if let egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } = event
                    {
                        // `Maj+I` doit rester "I" (vim l'utilise pour insérer
                        // en début de ligne) — seuls Ctrl/Alt/Cmd distinguent
                        // ça d'un raccourci sans rapport, comme Ctrl+Alt+I
                        // (Step Into).
                        let plain = !modifiers.ctrl && !modifiers.alt && !modifiers.mac_cmd;
                        match key {
                            egui::Key::I if plain => enter = true,
                            egui::Key::Escape => exit = true,
                            _ => {}
                        }
                    }
                }
                (enter, exit)
            });
            match self.editor_mode {
                EditorMode::Normal if enter_insert => {
                    self.editor_mode = EditorMode::Insert;
                    just_entered_insert = true;
                }
                EditorMode::Insert if exit_insert => self.editor_mode = EditorMode::Normal,
                _ => {}
            }
        }

        if let Some(line) = self.current_line() {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                icon_run(ui.painter(), rect, ACCENT);
                let text = if self.lang == Lang::Fr {
                    format!("Ligne courante : {line}")
                } else {
                    format!("Current line: {line}")
                };
                ui.colored_label(ACCENT, text);
            });
        }

        let current_line = self.current_line();
        let editor_bg = ui.visuals().extreme_bg_color;
        let font_id = egui::TextStyle::Monospace.resolve(ui.style());

        // Gouttière et texte peints sur UN seul fond continu (celui du
        // `TextEdit`, `extreme_bg_color`) plutôt que le `panel_fill` du
        // panneau englobant, et le `TextEdit` est rendu sans cadre
        // (`Frame::NONE`) pour ne pas repeindre un fond/une bordure
        // différents par-dessus.
        egui::Frame::new().fill(editor_bg).show(ui, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Calculé UNE fois ici, en dehors du `TextEdit`, pour que
                    // la gouttière connaisse la position Y *exacte* de
                    // chaque ligne telle que le texte la rendra réellement,
                    // plutôt que de deviner une hauteur de ligne générique
                    // (`ui.text_style_height`) qui dérivait de la vraie
                    // mise en page ligne après ligne — c'est ce décalage
                    // cumulatif qui désynchronisait gouttière et texte.
                    let galley = {
                        let job = syntax::highlight(ui, &self.source, f32::INFINITY);
                        ui.fonts_mut(|f| f.layout_job(job))
                    };

                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;

                        const GUTTER_WIDTH: f32 = 44.0;
                        let (gutter_rect, gutter_resp) = ui.allocate_exact_size(
                            egui::vec2(GUTTER_WIDTH, galley.rect.height().max(1.0)),
                            egui::Sense::click(),
                        );
                        let click_y = gutter_resp.interact_pointer_pos().map(|p| p.y);
                        let mut clicked_line = None;
                        let painter = ui.painter();
                        for (i, row) in galley.rows.iter().enumerate() {
                            let line = (i + 1) as u32;
                            let row_rect = row.rect().translate(gutter_rect.min.to_vec2());

                            if gutter_resp.clicked()
                                && let Some(y) = click_y
                                && row_rect.y_range().contains(y)
                            {
                                clicked_line = Some(line);
                            }

                            let is_current = current_line == Some(line);
                            if self.breakpoint_lines.contains(&line) {
                                painter.circle_filled(
                                    egui::pos2(gutter_rect.left() + 8.0, row_rect.center().y),
                                    4.0,
                                    BAD,
                                );
                            }
                            if is_current {
                                let arrow_rect = egui::Rect::from_center_size(
                                    egui::pos2(gutter_rect.left() + 20.0, row_rect.center().y),
                                    egui::vec2(10.0, 10.0),
                                );
                                icon_run(painter, arrow_rect, ACCENT);
                            }
                            let color = if is_current { ACCENT } else { ui.visuals().weak_text_color() };
                            painter.text(
                                egui::pos2(gutter_rect.right() - 6.0, row_rect.center().y),
                                egui::Align2::RIGHT_CENTER,
                                line.to_string(),
                                font_id.clone(),
                                color,
                            );
                        }
                        if let Some(line) = clicked_line {
                            if self.breakpoint_lines.contains(&line) {
                                self.breakpoint_lines.remove(&line);
                            } else {
                                self.breakpoint_lines.insert(line);
                            }
                            self.sync_breakpoints();
                        }

                        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                            let job = syntax::highlight(ui, buf.as_str(), wrap_width);
                            ui.fonts_mut(|f| f.layout_job(job))
                        };
                        let response = ui.add(
                            egui::TextEdit::multiline(&mut self.source)
                                .frame(egui::Frame::NONE)
                                .desired_width(f32::INFINITY)
                                .desired_rows(24)
                                .code_editor()
                                .interactive(self.editor_mode == EditorMode::Insert)
                                .layouter(&mut layouter),
                        );
                        if just_entered_insert {
                            response.request_focus();
                        }
                        if response.clicked() {
                            // Cliquer dans le code reprend le focus clavier
                            // à l'explorateur.
                            self.explorer_focused = false;
                        }
                        // Conditionne l'armement de la touche leader
                        // (Espace, ...) : sinon "Espace" taperait un
                        // espace *et* armerait la séquence à chaque mot
                        // pendant la frappe.
                        self.editor_focused = response.has_focus();
                    });
                });
        });
    }

    fn sidebar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // RE est masqué par défaut (activable dans Préférences > Modules) :
        // s'il vient d'être caché pendant qu'il était actif, on retombe
        // sur Registres plutôt que de laisser un panneau vide.
        if self.active_tab == SidebarTab::Re && !self.show_re_tab {
            self.active_tab = SidebarTab::Registers;
        }
        ui.horizontal(|ui| {
            for tab in SidebarTab::ALL {
                if tab == SidebarTab::Re && !self.show_re_tab {
                    continue;
                }
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

/// Libellé d'une action, pour le tableau de raccourcis de Préférences.
fn action_label(s: &Strings, action: Action) -> &'static str {
    match action {
        Action::ToggleExplorer => s.action_toggle_explorer,
        Action::Assemble => s.action_assemble,
        Action::Run => s.action_run,
        Action::StepInto => s.action_step_into,
        Action::StepOver => s.action_step_over,
        Action::Stop => s.action_stop,
        Action::Reset => s.action_reset,
        Action::Save => s.action_save,
    }
}

/// Texte d'info-bulle pour un bouton lié à une action : le libellé plus,
/// s'il existe, son raccourci courant — lu en direct dans `KeyBindings`
/// pour qu'un changement dans Préférences se reflète aussitôt.
fn shortcut_hint(bindings: &KeyBindings, ctx: &egui::Context, label: &str, action: Action) -> String {
    match bindings.map.get(&action) {
        Some(chord) => format!("{label} ({})", chord.display(ctx)),
        None => label.to_string(),
    }
}

/// Bouton de barre d'outils avec une icône dessinée directement plutôt
/// qu'un glyphe Unicode — "●", puis "↓"/"→"/"🔨" se sont tous révélés
/// absents de la police par défaut d'egui et retombaient sur des carrés
/// vides ou des glyphes de repli erratiques (voir commit 5d95e09 et le
/// retour utilisateur qui a suivi). Peindre la forme soi-même élimine le
/// risque une fois pour toutes.
fn icon_button(
    ui: &mut egui::Ui,
    icon: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
    label: &str,
) -> egui::Response {
    let icon_size = 14.0;
    let gap = 6.0;
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(13.0),
        egui::Color32::PLACEHOLDER,
    );
    let padding = ui.spacing().button_padding;
    let desired_size = egui::vec2(
        icon_size + gap + galley.size().x + padding.x * 2.0,
        ui.spacing().interact_size.y,
    );
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + padding.x, rect.center().y - icon_size / 2.0),
            egui::vec2(icon_size, icon_size),
        );
        icon(ui.painter(), icon_rect, visuals.text_color());
        let text_pos = egui::pos2(icon_rect.right() + gap, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(text_pos, galley, visuals.text_color());
    }

    response
}

fn icon_run(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let r = rect.shrink(1.0);
    painter.add(egui::Shape::convex_polygon(
        vec![r.left_top(), r.left_bottom(), egui::pos2(r.right(), r.center().y)],
        color,
        egui::Stroke::NONE,
    ));
}

fn icon_stop(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    painter.rect_filled(rect.shrink(2.0), 1.0, color);
}

fn icon_step_into(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let x = rect.center().x;
    let stroke = egui::Stroke::new(1.6, color);
    painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom() - 3.0)], stroke);
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(x - 3.0, rect.bottom() - 5.0),
            egui::pos2(x + 3.0, rect.bottom() - 5.0),
            egui::pos2(x, rect.bottom()),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

fn icon_step_over(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let y = rect.center().y;
    let stroke = egui::Stroke::new(1.6, color);
    painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right() - 3.0, y)], stroke);
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(rect.right() - 5.0, y - 3.0),
            egui::pos2(rect.right() - 5.0, y + 3.0),
            egui::pos2(rect.right(), y),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

/// Pastille de mode façon vim-airline : fond plein de la couleur du mode,
/// texte sombre par-dessus pour le contraste.
fn mode_pill_ui(ui: &mut egui::Ui, mode: EditorMode) {
    egui::Frame::new()
        .fill(mode.color())
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.colored_label(egui::Color32::from_rgb(0x14, 0x14, 0x14), mode.label());
        });
}

/// "NASM version 3.02 compiled on Jul 14 2026" -> "NASM 3.02" — la barre
/// de statut n'a pas besoin de la date de compilation de nasm lui-même,
/// seulement de savoir qu'il est disponible et sa version.
fn short_nasm_version(full: &str) -> String {
    let mut words = full.split_whitespace();
    match (words.next(), words.next(), words.next()) {
        (Some(name), Some("version"), Some(version)) => format!("{name} {version}"),
        _ => full.to_string(),
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
        let ctx = ui.ctx().clone();
        self.tick_run(&ctx);
        if self.rebinding.is_none() {
            // Un rebind en cours capture les touches lui-même (voir
            // rebinding_ui) — pas de dispatch normal pendant ce temps.
            if let Some(action) = self
                .dispatcher
                .poll(&ctx, &self.keybindings, self.editor_focused)
            {
                self.dispatch(action);
            }
        }

        if self.rebinding.is_none()
            && ctx.input_mut(|i| {
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                    egui::Key::P,
                ))
            })
        {
            self.command_palette = match self.command_palette {
                Some(_) => None,
                None => Some(CommandPalette::default()),
            };
        }

        egui::Panel::top("menu_bar").show(ui, |ui| self.menu_bar_ui(ui));
        if self.show_toolbar {
            egui::Panel::top("toolbar").show(ui, |ui| self.toolbar_ui(ui));
        }
        if self.show_statusbar {
            // Un fond légèrement distinct, sans bordure complète autour —
            // une pleine boîte s'est révélée trop marquée (retour
            // utilisateur), un simple ton différent suffit à la séparer du
            // panneau au-dessus sans "encadrer" toute la barre.
            let statusbar_frame = egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(egui::Margin::symmetric(10, 5));
            egui::Panel::bottom("statusbar")
                .frame(statusbar_frame)
                .show(ui, |ui| self.statusbar_ui(ui));
        }
        if self.show_sidebar {
            egui::Panel::right("sidebar")
                .resizable(true)
                .default_size(300.0)
                .show(ui, |ui| self.sidebar_ui(ui));
        }
        if self.show_explorer {
            egui::Panel::left("explorer")
                .resizable(true)
                .default_size(260.0)
                .show(ui, |ui| self.explorer_panel_ui(ui));
        }
        self.editor_ui(ui);

        self.preferences_window_ui(ui.ctx());
        self.about_window_ui(ui.ctx());
        self.command_palette_ui(&ctx);
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
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([800.0, 520.0]),
        ..Default::default()
    };
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
