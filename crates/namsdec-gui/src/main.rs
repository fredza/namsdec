//! Coquille eframe/egui du MVP d'apprentissage NASM. Assemble via
//! `namsdec-asm` (nasm externe), désassemble via `namsdec-disasm`
//! (vérification pédagogique) et exécute pas à pas via `namsdec-emu`,
//! en affichant les panneaux des modules `ReModule` enregistrés.

mod i18n;
mod keybindings;
mod settings;
mod syntax;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use eframe::egui;
use namsdec_asm::{AsmError, Diagnostic, Severity};
use namsdec_core::{Reg64, RFlags};
use namsdec_disasm::DecodedInstruction;
use namsdec_emu::{Emulator, RunStop, StepOverResult, StepResult};
use namsdec_explorer::{ExplorerAction, FileExplorer};
use namsdec_re::{BinaryInfoModule, ReContext, ReModuleRegistry};
use namsdec_theme::Theme;

use i18n::{strings, Lang, Strings};
use keybindings::{Action, Dispatcher, KeyBindings, Rebinding};
use settings::Settings;

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

/// Thème appliqué au premier lancement. Les autres (dont Tokyo Night, le
/// thème nvim) se choisissent dans Préférences > Plugins.
const DEFAULT_THEME: &Theme = &namsdec_theme::VSCODE_DARK;

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
    BinaryLoaded { bytes: usize, truncated: bool },
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

    fn color(self, theme: &Theme) -> egui::Color32 {
        match self {
            EditorMode::Normal => theme.accent,
            EditorMode::Insert => theme.good,
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
    /// Thème actif. Une référence statique plutôt qu'un enum : ajouter un
    /// thème (installé ou fourni) ne doit pas obliger à rouvrir un `match`
    /// quelque part dans l'interface.
    theme: &'static Theme,
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
    /// Réglages tels qu'ils étaient à l'ouverture de la boîte Préférences.
    /// C'est ce qui donne un sens distinct aux deux boutons : OK garde,
    /// Quitter revient à cet état.
    prefs_snapshot: Option<PreferencesSnapshot>,
    show_about: bool,
    keybindings: KeyBindings,
    dispatcher: Dispatcher,
    rebinding: Option<Rebinding>,
    running: bool,
    run_steps_taken: u64,
    editor_focused: bool,
    editor_mode: EditorMode,
    /// Position du curseur de texte (ligne, colonne), toutes deux
    /// 1-indexées. Conservée d'une frame à l'autre : en mode Normal le
    /// `TextEdit` n'est pas interactif et ne rend donc aucun curseur, mais
    /// la barre de statut doit continuer d'indiquer où l'on est — comme
    /// vim, qui ne perd pas son curseur en quittant le mode Insertion.
    cursor_pos: (usize, usize),
    preferences_tab: PreferencesTab,
    show_re_tab: bool,
    /// Fenêtre flottante indépendante (crate `namsdec-explorer`), ouverte
    /// directement par son raccourci — elle ne fait pas partie des
    /// panneaux de la mise en page et n'apparaît donc pas dans le groupe
    /// de cases à cocher du menu Affichage.
    explorer: FileExplorer,
    show_explorer: bool,
    /// Vient d'être ouvert dans cette frame — voir `explorer_window_ui`.
    explorer_just_opened: bool,
    /// L'explorateur a-t-il le focus clavier ? Comme dans nvim, les touches
    /// vont soit à l'arborescence (j/k/l/h…), soit à l'éditeur, jamais aux
    /// deux.
    explorer_focused: bool,
    current_file: Option<PathBuf>,
    show_toolbar: bool,
    show_sidebar: bool,
    show_statusbar: bool,
    command_palette: Option<CommandPalette>,
    /// Réglages tels qu'écrits sur le disque. Comparés à l'état courant à
    /// chaque frame : dès qu'ils divergent, le fichier est réécrit. C'est
    /// ce qui garantit qu'aucun réglage ne peut être oublié — il n'y a pas
    /// un appel à "sauvegarder" par endroit où un réglage se change.
    saved_settings: Settings,
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
    SetTheme(&'static Theme),
    SetLang(Lang),
    SaveAs,
    Quit,
    GoTab(SidebarTab),
    ToggleToolbar,
    ToggleSidebar,
    ToggleStatusbar,
}

fn palette_commands(s: &Strings) -> Vec<(String, PaletteCommand)> {
    let mut commands = vec![
        (s.action_assemble.to_string(), PaletteCommand::Do(Action::Assemble)),
        (s.action_run.to_string(), PaletteCommand::Do(Action::Run)),
        (s.action_step_into.to_string(), PaletteCommand::Do(Action::StepInto)),
        (s.action_step_over.to_string(), PaletteCommand::Do(Action::StepOver)),
        (s.action_stop.to_string(), PaletteCommand::Do(Action::Stop)),
        (s.action_reset.to_string(), PaletteCommand::Do(Action::Reset)),
        (s.action_toggle_explorer.to_string(), PaletteCommand::Do(Action::ToggleExplorer)),
        (s.action_save.to_string(), PaletteCommand::Do(Action::Save)),
        (s.file_save_as.to_string(), PaletteCommand::SaveAs),
        (s.file_open_file.to_string(), PaletteCommand::Do(Action::OpenFile)),
        (s.file_open_folder.to_string(), PaletteCommand::Do(Action::OpenFolder)),
        (s.file_preferences.to_string(), PaletteCommand::ShowPreferences),
        (s.help_about.to_string(), PaletteCommand::ShowAbout),
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
    ];

    // Un thème installé plus tard apparaît ici sans rien changer d'autre.
    commands.extend(namsdec_theme::BUILT_IN.iter().map(|theme| {
        (
            format!("{}: {}", s.preferences_theme, theme.name),
            PaletteCommand::SetTheme(theme),
        )
    }));

    commands
}

struct PreferencesSnapshot {
    theme: &'static Theme,
    lang: Lang,
    show_re_tab: bool,
    keybindings: KeyBindings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreferencesTab {
    General,
    Shortcuts,
    Modules,
    Plugins,
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
            theme: DEFAULT_THEME,
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
            prefs_snapshot: None,
            show_about: false,
            keybindings: KeyBindings::default(),
            dispatcher: Dispatcher::default(),
            rebinding: None,
            running: false,
            run_steps_taken: 0,
            editor_focused: false,
            editor_mode: EditorMode::Normal,
            cursor_pos: (1, 1),
            preferences_tab: PreferencesTab::General,
            show_re_tab: false,
            explorer: FileExplorer::default(),
            show_explorer: false,
            explorer_just_opened: false,
            explorer_focused: false,
            current_file: None,
            show_toolbar: true,
            show_sidebar: true,
            show_statusbar: true,
            command_palette: None,
            saved_settings: Settings::default(),
        }
    }
}

impl NamsdecApp {
    /// Construit l'application à partir des réglages relus sur le disque.
    fn from_settings(settings: Settings) -> Self {
        let explorer = FileExplorer::with_root(settings.explorer_root());
        let mut app = Self {
            theme: settings.theme(),
            lang: settings.lang(),
            keybindings: settings.keybindings(),
            show_toolbar: settings.show_toolbar,
            show_sidebar: settings.show_sidebar,
            show_statusbar: settings.show_statusbar,
            show_re_tab: settings.show_re_tab,
            explorer,
            saved_settings: settings,
            ..Default::default()
        };

        // Écrit le fichier dès le premier lancement, même si rien n'a
        // encore été réglé : l'utilisateur doit pouvoir le trouver et
        // l'éditer sans devoir changer un réglage d'abord, et les défauts
        // du jour y sont gravés plutôt que de suivre en silence ceux
        // d'une version future.
        if let Err(e) = app.saved_settings.save() {
            app.status = Status::Error(if app.lang == Lang::Fr {
                format!("impossible d'enregistrer les réglages : {e}")
            } else {
                format!("could not save settings: {e}")
            });
        }
        app
    }

    /// L'état courant des réglages, sous la forme qui part dans le
    /// fichier. Tout ce qui est réglable par l'utilisateur doit figurer
    /// ici — c'est le point unique à compléter en ajoutant un réglage.
    fn settings_snapshot(&self) -> Settings {
        Settings {
            theme: self.theme.id.to_owned(),
            language: match self.lang {
                Lang::Fr => "fr".to_owned(),
                Lang::En => "en".to_owned(),
            },
            show_toolbar: self.show_toolbar,
            show_sidebar: self.show_sidebar,
            show_statusbar: self.show_statusbar,
            show_re_tab: self.show_re_tab,
            explorer_root: self
                .explorer
                .root
                .as_ref()
                .map(|p| p.display().to_string()),
            shortcuts: settings::shortcuts_to_strings(&self.keybindings),
        }
    }

    /// Appelé à chaque frame : n'écrit que si un réglage a bougé. Une
    /// écriture qui échoue est signalée une seule fois (l'instantané est
    /// mis à jour quoi qu'il arrive) plutôt que retentée à chaque frame.
    fn persist_settings_if_changed(&mut self) {
        let current = self.settings_snapshot();
        if current == self.saved_settings {
            return;
        }
        let result = current.save();
        self.saved_settings = current;
        if let Err(e) = result {
            self.status = Status::Error(if self.lang == Lang::Fr {
                format!("impossible d'enregistrer les réglages : {e}")
            } else {
                format!("could not save settings: {e}")
            });
        }
    }

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
                self.explorer_just_opened = self.show_explorer;
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
            Action::OpenFile => self.do_open_file_dialog(),
            Action::OpenFolder => self.do_open_folder(),
        }
    }

    fn do_open_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.explorer.root = Some(folder);
            self.show_explorer = true;
            // Même garde que pour la bascule : le clic du menu (ou celui
            // qui a validé la boîte native) ne doit pas refermer aussitôt
            // la fenêtre qu'il vient de faire apparaître.
            self.explorer_just_opened = true;
            self.focus_explorer(true);
        }
    }

    /// Sélecteur natif avec ses filtres — source assembleur, binaire, ou
    /// tout. Accessible directement (Ctrl+O, menu Fichier, palette) sans
    /// avoir à ouvrir l'explorateur d'abord.
    fn do_open_file_dialog(&mut self) {
        let s = strings(self.lang);
        let picked = rfd::FileDialog::new()
            .add_filter(s.filter_asm, &["asm", "nasm", "s", "inc"])
            .add_filter(s.filter_binary, &["bin", "o", "obj", "elf", "so", "exe", "dll"])
            // "*" est traduit en glob `*` par le backend portail de rfd :
            // c'est ce qui laisse choisir un exécutable ELF sans
            // extension, cas le plus courant sous Linux.
            .add_filter(s.filter_all, &["*"])
            .pick_file();
        if let Some(path) = picked {
            self.open_file(path);
        }
    }

    /// Charge `path`. Du texte part dans l'éditeur ; tout le reste est
    /// traité comme un binaire brut à désassembler, plutôt que rejeté
    /// comme "pas de l'UTF-8" — c'est un atelier de reverse engineering,
    /// ouvrir un binaire doit y mener quelque part.
    fn open_file(&mut self, path: PathBuf) {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.status = Status::Error(if self.lang == Lang::Fr {
                    format!("impossible d'ouvrir le fichier : {e}")
                } else {
                    format!("could not open file: {e}")
                });
                return;
            }
        };

        match String::from_utf8(bytes) {
            Ok(text) => {
                self.source = text;
                self.current_file = Some(path);
                self.assemble_state = None;
                self.emulator = None;
                self.diagnostics.clear();
                self.status = Status::Idle;
            }
            Err(e) => self.load_binary(e.into_bytes()),
        }
    }

    /// Ouvre un binaire brut : désassemblage direct à `BASE_ADDR`, sans
    /// passer par nasm. Le listing obtenu remplit l'éditeur, une
    /// instruction par ligne — ce qui donne aussi la correspondance
    /// ligne <-> adresse dont dépendent la gouttière, le suivi de RIP et
    /// les points d'arrêt.
    ///
    /// `current_file` est délibérément remis à zéro : le tampon affiché
    /// est reconstruit, pas le contenu du fichier, et un Ctrl+S distrait
    /// ne doit pas écraser le binaire par son propre désassemblage.
    fn load_binary(&mut self, mut bytes: Vec<u8>) {
        // Borné à la fenêtre mémoire simulée : au-delà, `rebuild_emulator`
        // refuserait le chargement de toute façon, et désassembler
        // plusieurs centaines de kilo-octets pour les jeter ensuite
        // figerait l'éditeur le temps de la mise en page du listing.
        let truncated = bytes.len() > MEM_SIZE;
        bytes.truncate(MEM_SIZE);

        let disasm = namsdec_disasm::disassemble(&bytes, BASE_ADDR);

        let mut listing = String::new();
        let mut line_to_addr = BTreeMap::new();
        for (index, instruction) in disasm.iter().enumerate() {
            line_to_addr.insert((index + 1) as u32, instruction.address);
            listing.push_str(&instruction.text);
            listing.push('\n');
        }

        let addr_to_line = line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
        let loaded = bytes.len();
        self.source = listing;
        self.current_file = None;
        self.breakpoint_lines.clear();
        self.diagnostics.clear();
        self.assemble_state = Some(AssembleState {
            output: namsdec_asm::AssembleOutput {
                bytes,
                diagnostics: Vec::new(),
                line_to_addr,
            },
            disasm,
            addr_to_line,
        });
        self.rebuild_emulator();
        if !matches!(self.status, Status::Error(_)) {
            self.status = Status::BinaryLoaded {
                bytes: loaded,
                truncated,
            };
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

    /// La fenêtre flottante de l'explorateur. Indépendante de la mise en
    /// page : elle se superpose à l'éditeur au lieu de le rétrécir, et
    /// n'existe que tant que son raccourci l'a ouverte.
    fn explorer_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_explorer {
            return;
        }
        let s = strings(self.lang);
        let labels = namsdec_explorer::Labels {
            title: s.explorer_title,
            filter_hint: s.explorer_filter_hint,
            empty: s.explorer_empty,
            open_folder: s.file_open_folder,
        };

        let response = self
            .explorer
            .show(ctx, self.theme, &labels, self.explorer_focused);

        if response.focus_gained {
            self.focus_explorer(true);
        }

        let mut action = response.action;
        if self.explorer_just_opened {
            // Le clic qui vient de l'ouvrir (entrée de menu, palette) est
            // par construction hors de la fenêtre : sans cette garde, il
            // la refermerait dans la frame même où elle apparaît.
            self.explorer_just_opened = false;
            if matches!(action, ExplorerAction::Close) {
                action = ExplorerAction::None;
            }
        }

        match action {
            ExplorerAction::OpenFile(path) => {
                self.open_file(path);
                // Ouvrir un fichier rend la main à l'éditeur, comme dans
                // neo-tree où la sélection d'un fichier saute au buffer.
                self.explorer_focused = false;
            }
            ExplorerAction::PickRoot => self.do_open_folder(),
            ExplorerAction::Close => {
                self.show_explorer = false;
                self.explorer_focused = false;
            }
            ExplorerAction::None => {}
        }
    }

    fn execute_palette_command(&mut self, ctx: &egui::Context, cmd: PaletteCommand) {
        match cmd {
            PaletteCommand::Do(action) => self.dispatch(action),
            PaletteCommand::ShowPreferences => self.open_preferences(),
            PaletteCommand::ShowAbout => self.show_about = true,
            PaletteCommand::SetTheme(theme) => {
                self.theme = theme;
                self.apply_theme(ctx);
            }
            PaletteCommand::SetLang(lang) => self.lang = lang,
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
                let ctx = ui.ctx().clone();
                if ui
                    .button(shortcut_hint(&self.keybindings, &ctx, s.file_open_file, Action::OpenFile))
                    .clicked()
                {
                    self.dispatch(Action::OpenFile);
                    ui.close();
                }
                if ui
                    .button(shortcut_hint(&self.keybindings, &ctx, s.file_open_folder, Action::OpenFolder))
                    .clicked()
                {
                    self.dispatch(Action::OpenFolder);
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
                    self.open_preferences();
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

                ui.separator();
                // L'explorateur n'est PAS un de ces panneaux : c'est une
                // fenêtre indépendante qu'on appelle, d'où une entrée à
                // part qui rappelle son raccourci plutôt qu'une case à
                // cocher dans le groupe ci-dessus.
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        &ui.ctx().clone(),
                        s.action_toggle_explorer,
                        Action::ToggleExplorer,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::ToggleExplorer);
                    ui.close();
                }

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

    /// Ouvre la boîte Préférences en mémorisant l'état des réglages, pour
    /// que "Quitter" ait quelque chose à quoi revenir. Rouvrir une boîte
    /// déjà ouverte ne réécrit pas l'instantané.
    fn open_preferences(&mut self) {
        if !self.show_preferences {
            self.prefs_snapshot = Some(PreferencesSnapshot {
                theme: self.theme,
                lang: self.lang,
                show_re_tab: self.show_re_tab,
                keybindings: self.keybindings.clone(),
            });
        }
        self.show_preferences = true;
    }

    /// Ferme la boîte Préférences. `keep` distingue les deux boutons :
    /// OK garde ce qui vient d'être réglé, Quitter restaure l'état
    /// d'avant l'ouverture — les réglages s'appliquent en direct (le
    /// thème surtout), il faut donc pouvoir revenir en arrière.
    fn close_preferences(&mut self, ctx: &egui::Context, keep: bool) {
        if let Some(snapshot) = self.prefs_snapshot.take()
            && !keep
        {
            self.theme = snapshot.theme;
            self.lang = snapshot.lang;
            self.show_re_tab = snapshot.show_re_tab;
            self.keybindings = snapshot.keybindings;
            self.apply_theme(ctx);
        }
        self.rebinding = None;
        self.show_preferences = false;
    }

    fn preferences_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_preferences;
        let mut close_keeping: Option<bool> = None;
        egui::Window::new(s.preferences_title)
            .open(&mut open)
            .resizable(true)
            // Pas de flèche de repli : replier une boîte de préférences
            // ne laisse qu'un bandeau de titre qui flotte, sans usage.
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (PreferencesTab::General, s.preferences_tab_general),
                        (PreferencesTab::Shortcuts, s.preferences_tab_shortcuts),
                        (PreferencesTab::Modules, s.preferences_tab_modules),
                        (PreferencesTab::Plugins, s.preferences_tab_plugins),
                    ] {
                        ui.selectable_value(&mut self.preferences_tab, tab, label);
                    }
                });
                ui.separator();
                match self.preferences_tab {
                    PreferencesTab::General => self.preferences_general_ui(ui),
                    PreferencesTab::Shortcuts => self.preferences_shortcuts_ui(ui),
                    PreferencesTab::Modules => self.preferences_modules_ui(ui),
                    PreferencesTab::Plugins => self.preferences_plugins_ui(ui),
                }
                ui.separator();
                close_keeping = dialog_buttons(ui, theme, s);
            });

        match close_keeping {
            Some(keep) => self.close_preferences(ctx, keep),
            // La croix du bandeau vaut OK : refermer une boîte ne doit pas
            // défaire en silence ce qu'on vient d'y régler.
            None if !open => self.close_preferences(ctx, true),
            None => {}
        }
    }

    fn preferences_general_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal(|ui| {
            ui.label(s.preferences_theme);
            ui.strong(self.theme.name);
        });
        ui.weak(s.preferences_theme_in_plugins);

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
        let theme = self.theme;
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
                        ui.colored_label(theme.accent, s.rebind_waiting);
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

    /// Onglet Plugins : pour l'instant les thèmes, qui en sont le premier
    /// type. Un thème est une simple palette (`namsdec-theme`), donc le
    /// jour où d'autres s'installent, ils se contentent d'allonger la
    /// liste — rien ici n'est écrit par thème.
    fn preferences_plugins_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let ctx = ui.ctx().clone();

        ui.strong(s.plugins_themes);
        ui.weak(s.plugins_themes_hint);
        ui.add_space(6.0);

        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for theme in namsdec_theme::BUILT_IN {
                let active = std::ptr::eq(self.theme, *theme);
                let response = theme_card(ui, theme, active, s.plugins_active);
                if response.clicked() && !active {
                    self.theme = theme;
                    self.apply_theme(&ctx);
                }
            }
        });

        ui.add_space(6.0);
        ui.weak(s.plugins_install_soon);
    }

    fn about_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_about;
        let mut close = false;
        egui::Window::new(s.about_title)
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Namsdec");
                ui.label(format!(
                    "Version {APP_VERSION} ({} {BUILD_DATE})",
                    s.about_build_word
                ));
                ui.label(s.about_license);
                ui.separator();
                ui.label(s.about_description);
                ui.separator();
                // Rien à valider ni à annuler ici : les deux boutons
                // referment, ils sont là pour que toutes les boîtes se
                // manipulent de la même façon.
                close = dialog_buttons(ui, theme, s).is_some();
            });
        self.show_about = open && !close;
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
        let theme = self.theme;

        if let Some(key) = self.dispatcher.leader_armed() {
            let text = if lang == Lang::Fr {
                format!("{} appuyée — touche suivante attendue…", key.name())
            } else {
                format!("{} pressed — waiting for next key…", key.name())
            };
            ui.colored_label(theme.accent, text);
        }

        ui.horizontal(|ui| {
            // Compartimenté plutôt qu'un seul bloc de texte : pastille de
            // mode, séparateur, message de statut, séparateur, statut nasm
            // (version raccourcie — "NASM 3.02", pas la ligne complète de
            // `nasm -v` avec sa date de compilation, hors de propos ici).
            mode_pill_ui(ui, theme, self.editor_mode);
            ui.separator();
            self.status_message_ui(ui);

            // Compartiments de droite, remplis de droite à gauche :
            // nasm, puis la position du curseur, puis — seulement quand un
            // programme est chargé — la ligne où l'exécution est arrêtée.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (dot_color, text) = match &self.nasm_info {
                    Some(info) => (theme.good, short_nasm_version(&info.version)),
                    None => (theme.bad, s.nasm_not_found.to_string()),
                };
                ui.label(text);
                status_dot(ui, dot_color);
                ui.separator();

                let (line, col) = self.cursor_pos;
                ui.label(if lang == Lang::Fr {
                    format!("Lig {line}, Col {col}")
                } else {
                    format!("Ln {line}, Col {col}")
                });

                if let Some(exec_line) = self.current_line() {
                    ui.separator();
                    ui.colored_label(
                        theme.accent,
                        if lang == Lang::Fr {
                            format!("ligne {exec_line}")
                        } else {
                            format!("line {exec_line}")
                        },
                    );
                    // La même flèche que la gouttière marque dans la marge,
                    // pour que les deux se lisent comme la même chose.
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    icon_run(ui.painter(), rect, theme.accent);
                }
            });
        });

        if !self.diagnostics.is_empty() {
            egui::ScrollArea::vertical()
                .max_height(96.0)
                .show(ui, |ui| {
                    for d in &self.diagnostics {
                        let color = if d.severity == Severity::Error { theme.bad } else { theme.warn };
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
        let theme = self.theme;
        match &self.status {
            Status::Idle => {
                ui.label(s.status_ready);
            }
            Status::Running => {
                ui.colored_label(theme.accent, s.status_running);
            }
            Status::AssembleOk { bytes } => {
                ui.colored_label(
                    theme.good,
                    if lang == Lang::Fr {
                        format!("✓ Assemblage réussi — {bytes} octets")
                    } else {
                        format!("✓ Assembled successfully — {bytes} bytes")
                    },
                );
            }
            Status::AssembleFailed => {
                ui.colored_label(
                    theme.bad,
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
                ui.colored_label(theme.good, s.status_halted);
            }
            Status::BinaryLoaded { bytes, truncated } => {
                let size = if *truncated {
                    if lang == Lang::Fr {
                        format!("{bytes} premiers octets")
                    } else {
                        format!("first {bytes} bytes")
                    }
                } else if lang == Lang::Fr {
                    format!("{bytes} octets")
                } else {
                    format!("{bytes} bytes")
                };
                ui.colored_label(theme.accent, format!("{} — {size}", s.binary_not_text));
            }
            Status::Breakpoint(addr) => {
                ui.colored_label(
                    theme.accent,
                    if lang == Lang::Fr {
                        format!("Arrêté sur point d'arrêt — {addr:#x}")
                    } else {
                        format!("Stopped at breakpoint — {addr:#x}")
                    },
                );
            }
            Status::StepLimit => {
                ui.colored_label(theme.bad, s.status_step_limit);
            }
            Status::NasmMissing => {
                ui.colored_label(theme.bad, s.status_nasm_missing);
            }
            Status::Error(msg) => {
                ui.colored_label(
                    theme.bad,
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
        let theme = self.theme;
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

        // La ligne courante d'exécution n'est plus annoncée par un bandeau
        // au-dessus de l'éditeur : elle a sa place dans la barre de
        // statut, avec la position du curseur.
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
                        let job = syntax::highlight(ui, theme, &self.source, f32::INFINITY);
                        ui.fonts_mut(|f| f.layout_job(job))
                    };

                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;

                        // `GUTTER_PAD` est l'air entre le dernier chiffre
                        // et la première colonne de code. Il fait partie
                        // de la gouttière plutôt que d'être un
                        // `item_spacing` : l'espacement horizontal est mis
                        // à zéro juste au-dessus pour que le `TextEdit`
                        // commence exactement où la gouttière s'arrête.
                        const GUTTER_PAD: f32 = 16.0;
                        const GUTTER_WIDTH: f32 = 38.0 + GUTTER_PAD;
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
                                    theme.bad,
                                );
                            }
                            if is_current {
                                let arrow_rect = egui::Rect::from_center_size(
                                    egui::pos2(gutter_rect.left() + 20.0, row_rect.center().y),
                                    egui::vec2(10.0, 10.0),
                                );
                                icon_run(painter, arrow_rect, theme.accent);
                            }
                            let color = if is_current { theme.accent } else { ui.visuals().weak_text_color() };
                            painter.text(
                                egui::pos2(gutter_rect.right() - GUTTER_PAD, row_rect.center().y),
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
                            let job = syntax::highlight(ui, theme, buf.as_str(), wrap_width);
                            ui.fonts_mut(|f| f.layout_job(job))
                        };
                        // `.show()` plutôt que `ui.add()` : c'est la seule
                        // forme qui donne accès au curseur, dont la barre
                        // de statut affiche la position.
                        let output = egui::TextEdit::multiline(&mut self.source)
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .desired_rows(24)
                            .code_editor()
                            .interactive(self.editor_mode == EditorMode::Insert)
                            .layouter(&mut layouter)
                            .show(ui);
                        if let Some(range) = output.cursor_range {
                            self.cursor_pos = line_col(&self.source, range.primary.index.0);
                        }
                        let response = output.response.response;
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
        let theme = self.theme;
        let fr = self.lang == Lang::Fr;

        module_card(
            ui,
            "namsdec-core",
            namsdec_core::VERSION,
            true,
            theme.good,
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
            if asm_ok { theme.good } else { theme.bad },
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
            theme.accent,
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
            theme.accent,
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
        module_card(ui, "namsdec-re", namsdec_re::VERSION, true, theme.good, &re_status);
    }
}

/// Rangée de boutons commune à toutes les boîtes de dialogue, alignée à
/// droite. Renvoie `Some(true)` pour OK, `Some(false)` pour Quitter,
/// `None` tant que rien n'est cliqué.
///
/// Ces deux-là sont dessinés avec un fond et un contour explicites : le
/// style de l'appli rend les boutons plats au repos, ce qui convient aux
/// barres d'outils mais laisserait les boutons d'une boîte de dialogue
/// se confondre avec du texte.
fn dialog_buttons(ui: &mut egui::Ui, theme: &Theme, s: &Strings) -> Option<bool> {
    let mut result = None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let ok = egui::Button::new(egui::RichText::new(s.dialog_ok).color(theme.bg))
            .fill(theme.accent);
        if ui.add(ok).clicked() {
            result = Some(true);
        }
        let quit = egui::Button::new(s.dialog_quit).stroke(egui::Stroke::new(1.0, theme.dim));
        if ui.add(quit).clicked() {
            result = Some(false);
        }
    });
    result
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
        Action::OpenFile => s.file_open_file,
        Action::OpenFolder => s.file_open_folder,
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
fn mode_pill_ui(ui: &mut egui::Ui, theme: &Theme, mode: EditorMode) {
    egui::Frame::new()
        .fill(mode.color(theme))
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.colored_label(egui::Color32::from_rgb(0x14, 0x14, 0x14), mode.label());
        });
}

/// Traduit un index de *caractère* dans le texte en (ligne, colonne),
/// toutes deux 1-indexées. egui ne donne que l'index brut ; le compter
/// ici plutôt que de lire la géométrie du `Galley` rend la position
/// indépendante de la mise en page (retour à la ligne automatique,
/// tabulations) — c'est bien la position dans le *texte* qu'on affiche.
fn line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for c in text.chars().take(char_index) {
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
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

/// Une entrée du catalogue de thèmes : un aperçu miniature peint avec les
/// couleurs du thème lui-même (fond, panneau, accents) plutôt qu'un nom
/// seul — c'est le thème qui se décrit, donc rien à écrire de plus quand
/// un nouveau s'ajoute.
fn theme_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    active: bool,
    active_word: &str,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 44.0), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, active);
        let painter = ui.painter();
        painter.rect(
            rect,
            8,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        let preview = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.center().y - 14.0),
            egui::vec2(58.0, 28.0),
        );
        painter.rect_filled(preview, 4, theme.bg);
        painter.rect_filled(
            egui::Rect::from_min_size(preview.min, egui::vec2(14.0, preview.height())),
            4,
            theme.panel,
        );
        for (i, color) in [theme.accent, theme.good, theme.warn, theme.bad]
            .iter()
            .enumerate()
        {
            painter.circle_filled(
                egui::pos2(preview.left() + 23.0 + i as f32 * 9.0, preview.center().y),
                3.0,
                *color,
            );
        }
        painter.rect_stroke(
            preview,
            4,
            egui::Stroke::new(1.0, theme.dim),
            egui::StrokeKind::Inside,
        );

        painter.text(
            egui::pos2(preview.right() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            theme.name,
            egui::FontId::proportional(14.0),
            visuals.text_color(),
        );
        if active {
            painter.text(
                egui::pos2(rect.right() - 10.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                active_word,
                egui::FontId::proportional(12.0),
                theme.accent,
            );
        }
    }

    ui.add_space(4.0);
    response
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
            //
            // N'importe quel champ de texte ayant le focus suffit à
            // désarmer la touche leader, pas seulement l'éditeur : taper
            // "une note" dans le filtre de l'explorateur ne doit pas
            // refermer celui-ci sur le "e" qui suit l'espace.
            let text_focused =
                self.editor_focused || ctx.memory(|m| m.focused().is_some());
            if let Some(action) = self.dispatcher.poll(&ctx, &self.keybindings, text_focused) {
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
        self.editor_ui(ui);

        // Fenêtres flottantes — dont l'explorateur, qui n'occupe donc plus
        // de place dans la mise en page et ne dépend d'aucun panneau.
        self.explorer_window_ui(&ctx);
        self.preferences_window_ui(&ctx);
        self.about_window_ui(&ctx);
        self.command_palette_ui(&ctx);

        // En dernier : tout réglage changé pendant cette frame, d'où qu'il
        // vienne (menu, préférences, palette, raccourci), est sur le
        // disque avant la frame suivante.
        self.persist_settings_if_changed();
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
            let app = NamsdecApp::from_settings(Settings::load());
            app.apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ouvrir un binaire doit mener quelque part : un vrai programme
    /// passé par nasm, relu comme des octets bruts, doit remplir
    /// l'éditeur d'un listing, donner la correspondance ligne <-> adresse
    /// dont dépend la gouttière, et produire un émulateur capable
    /// d'avancer.
    #[test]
    fn opening_a_raw_binary_disassembles_and_boots_the_emulator() {
        let Some(_nasm) = namsdec_asm::find_nasm() else {
            // Pas de nasm ici : ce test vérifie un aller-retour à travers
            // l'assembleur réel, il n'a rien à prouver sans lui.
            return;
        };
        let output = namsdec_asm::assemble(
            "BITS 64\norg 0x400000\n    mov rax, 7\n    hlt\n",
            BASE_ADDR,
        )
        .expect("le programme de test doit s'assembler");

        let mut app = NamsdecApp::default();
        app.load_binary(output.bytes.clone());

        assert!(
            app.source.contains("mov"),
            "le listing désassemblé doit remplir l'éditeur, obtenu : {:?}",
            app.source
        );
        assert_eq!(
            app.current_file, None,
            "le tampon est reconstruit : pas de fichier à réécrire dessus"
        );

        let state = app.assemble_state.as_ref().expect("un état chargé");
        assert_eq!(
            state.output.line_to_addr.get(&1).copied(),
            Some(BASE_ADDR),
            "la première ligne du listing est la première instruction"
        );
        assert_eq!(
            app.current_line(),
            Some(1),
            "RIP au départ doit se refléter sur la ligne 1 de la gouttière"
        );

        app.do_step();
        assert_eq!(
            app.emulator.as_ref().unwrap().cpu.regs.get64(Reg64::Rax),
            7,
            "le binaire chargé doit s'exécuter pas à pas comme un programme assemblé"
        );
    }

    #[test]
    fn cursor_position_counts_characters_not_bytes() {
        let text = "mov rax, 0\nsum_loop:\n    add rax, rcx";

        assert_eq!(line_col(text, 0), (1, 1), "début du texte");
        assert_eq!(line_col(text, 4), (1, 5), "milieu de la première ligne");
        // Index 11 = juste après le '\n' : début de la ligne 2.
        assert_eq!(line_col(text, 11), (2, 1), "après un saut de ligne");
        assert_eq!(line_col(text, text.chars().count()), (3, 17), "fin du texte");

        // Des caractères non-ASCII ne doivent pas décaler la colonne :
        // egui compte des caractères, pas des octets.
        assert_eq!(line_col("éàü|", 3), (1, 4), "trois caractères accentués");
    }

    /// Un fichier plus gros que la mémoire simulée est tronqué plutôt que
    /// rejeté — et le statut le dit, au lieu de laisser croire que tout a
    /// été chargé.
    #[test]
    fn oversized_binaries_are_truncated_not_rejected() {
        let mut app = NamsdecApp::default();
        app.load_binary(vec![0x90; MEM_SIZE + 1024]); // que des `nop`

        match app.status {
            Status::BinaryLoaded { bytes, truncated } => {
                assert_eq!(bytes, MEM_SIZE);
                assert!(truncated);
            }
            _ => panic!("un binaire trop gros doit tout de même se charger"),
        }
    }
}
