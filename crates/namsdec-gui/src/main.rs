//! Coquille eframe/egui du MVP d'apprentissage NASM. Assemble via
//! `namsdec-asm` (nasm externe), désassemble via `namsdec-disasm`
//! (vérification pédagogique) et exécute pas à pas via `namsdec-emu`,
//! en affichant les panneaux des modules `ReModule` enregistrés.

mod errors;
mod i18n;
mod keybindings;
mod settings;
mod syntax;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui;
use namsdec_asm::{AsmError, Diagnostic, Severity};
use namsdec_core::{RFlags, Reg64};
use namsdec_disasm::DecodedInstruction;
use namsdec_emu::{Backend, Emulator, RunStop, StepOverResult, StepResult};
use namsdec_explorer::{ExplorerAction, FileExplorer};
use namsdec_re::{BinaryInfoModule, ReContext, ReModuleRegistry};
use namsdec_theme::Theme;

use i18n::{Lang, Strings, strings};
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
/// Largeur du panneau latéral à la première ouverture. L'utilisateur la
/// change ensuite en tirant sur son bord, et egui la retient.
const SIDEBAR_WIDTH: f32 = 300.0;
/// Hauteur du panneau bas à la première ouverture, puis ses bornes — même
/// raison que pour le panneau latéral : une hauteur aberrante enregistrée
/// ne doit pas pouvoir revenir au lancement suivant.
const CONSOLE_HEIGHT: f32 = 180.0;
const CONSOLE_MIN_HEIGHT: f32 = 90.0;
const CONSOLE_MAX_FRACTION: f32 = 0.6;
/// L'illustration du bouton de don, en pixels bruts RGBA — 128 x 128.
///
/// Des pixels et non le JPEG d'origine : les décoder demanderait le crate
/// `image` et les chargeurs d'`egui_extras`, deux dépendances pour une
/// seule vignette. Le PNG lisible est gardé à côté dans `assets/`, et se
/// régénère ainsi :
///
/// ```text
/// magick offrir-un-repas.png -depth 8 RGBA:offrir-un-repas.rgba
/// ```
const TREAT_IMAGE: &[u8] = include_bytes!("../assets/offrir-un-repas.rgba");
const TREAT_IMAGE_SIDE: usize = 128;
/// Taille d'affichage dans la boîte : l'image est embarquée deux fois
/// plus grande, pour rester nette sur un écran à forte densité.
const TREAT_IMAGE_SHOWN: f32 = 84.0;

/// Le texte de la licence sous laquelle namsdec est publié.
const MIT_LICENSE_URL: &str = "https://opensource.org/license/mit";
/// Où mène le bouton de don. Adresse **fictive** pour l'instant —
/// `example.com` est réservé à la documentation, personne n'y reçoit
/// rien : à remplacer par le vrai lien (Ko-fi, PayPal.me, Liberapay…).
/// Vidée, la constante laisse le bouton affiché mais inactif plutôt que
/// d'envoyer l'utilisateur nulle part.
const DONATION_URL: &str = "https://example.com/namsdec/offrir-un-repas";

/// Les valeurs d'egui pour les infobulles, reprises telles quelles : le
/// réglage ne fait que les remplacer par un délai qui n'échoit jamais.
const TOOLTIP_DELAY: f32 = 0.5;
const TOOLTIP_GRACE_TIME: f32 = 0.2;
/// En deçà, le panneau ne montrerait plus rien de lisible.
const SIDEBAR_MIN_WIDTH: f32 = 200.0;
/// Et au-delà de cette part de la fenêtre, il mangerait l'éditeur. La
/// borne vaut aussi réparation : egui garde la largeur du panneau d'une
/// session à l'autre, donc une largeur aberrante — héritée d'un bug de
/// mise en page — revenait au lancement suivant et y restait. Bornée,
/// elle est ramenée dans le raisonnable dès la première frame.
const SIDEBAR_MAX_FRACTION: f32 = 0.5;
/// Cadence à laquelle on revient voir si le sélecteur de fichiers a
/// répondu. Assez lâche pour ne rien coûter pendant qu'il est ouvert,
/// assez serré pour que le fichier choisi apparaisse sans délai visible.
const DIALOG_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Numéro de version SemVer — ne JAMAIS le monter sans l'accord explicite
/// de l'utilisateur (voir memory: feedback_version_bump_approval).
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Date de compilation (YYYYMMDD), calculée par build.rs — se met à jour
/// seule, aucune approbation requise contrairement à APP_VERSION.
const BUILD_DATE: &str = env!("NAMSDEC_BUILD_DATE");

/// Thème appliqué au premier lancement. Les autres (dont Tokyo Night, le
/// thème nvim) se choisissent dans Préférences > Plugins.
const DEFAULT_THEME: &Theme = &namsdec_theme::VSCODE_DARK;

/// Les vues du panneau latéral qui s'échangent.
///
/// Registres et flags n'en font plus partie : ce sont eux qu'on surveille
/// en avançant instruction par instruction, ils ont donc un bandeau fixe
/// en haut du panneau plutôt qu'un onglet où il faut revenir. Et la
/// console a quitté le panneau pour le bas de la fenêtre, où elle a la
/// largeur d'un terminal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SidebarTab {
    Stack,
    Memory,
    Re,
}

impl SidebarTab {
    const ALL: [SidebarTab; 3] = [SidebarTab::Stack, SidebarTab::Memory, SidebarTab::Re];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            SidebarTab::Stack => s.tab_stack,
            SidebarTab::Memory => s.tab_memory,
            SidebarTab::Re => s.tab_re,
        }
    }
}

/// Les deux vues du panneau bas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ConsoleTab {
    /// Ce que le programme a écrit lui-même, via `write` — la seule chose
    /// qu'il affiche au monde, et donc celle qu'on vient vérifier après
    /// l'avoir exécuté.
    Output,
    /// Ce que nasm a eu à dire de la source : erreurs et avertissements,
    /// reliés à leur ligne. Ils vivaient sous la barre de statut, où une
    /// liste un peu longue n'avait pas la place d'être lue.
    Diagnostics,
}

impl ConsoleTab {
    const ALL: [ConsoleTab; 2] = [ConsoleTab::Output, ConsoleTab::Diagnostics];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            ConsoleTab::Output => s.tab_output,
            ConsoleTab::Diagnostics => s.tab_diagnostics,
        }
    }
}

/// `Debug` sert aux tests, qui doivent pouvoir dire quel statut ils ont
/// obtenu quand ils en attendaient un autre.
#[derive(Debug)]
enum Status {
    Idle,
    AssembleOk,
    AssembleFailed,
    Running,
    Halted,
    /// Le programme s'est terminé lui-même par `exit`, avec ce code — ce
    /// qui n'est pas la même chose que la machine arrêtée par `hlt`.
    Exited {
        code: i64,
    },
    /// Le programme attend son entrée standard : il ne reprendra que
    /// lorsqu'on lui aura donné de quoi lire.
    WaitingForInput,
    BinaryLoaded {
        bytes: usize,
        truncated: bool,
    },
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
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
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
    /// Texte en cours de saisie dans la console, pas encore envoyé au
    /// programme. Donnée de session et non réglage : elle n'a aucun sens
    /// hors du programme qui tourne, et n'a donc rien à faire dans
    /// `settings.json`.
    console_input: String,
    /// Un `Exécuter` a été suspendu faute d'entrée, et doit repartir tout
    /// seul dès qu'elle arrive. Un pas à pas, lui, ne reprend pas de
    /// lui-même : c'est l'utilisateur qui décide quand avancer.
    resume_run_after_input: bool,
    /// Le moteur d'exécution, derrière son trait : l'interface ne sait
    /// pas lequel tourne, et n'a pas à le savoir (voir
    /// `namsdec_emu::Backend`).
    emulator: Option<Box<dyn Backend>>,
    /// Les registres tels qu'ils étaient avant la dernière avancée —
    /// un pas, un pas principal, ou le départ d'une exécution. Le
    /// bandeau met en avant ceux qui ont changé depuis : c'est ce qui
    /// fait *voir* ce qu'une instruction fait, et gdb lui-même laisse
    /// l'utilisateur comparer de tête.
    regs_before: Option<namsdec_core::Registers>,
    re_registry: ReModuleRegistry,
    status: Status,
    diagnostics: Vec<Diagnostic>,
    active_tab: SidebarTab,
    console_tab: ConsoleTab,
    mem_addr_input: String,
    /// L'adresse effectivement montrée par l'onglet Mémoire. Distincte de
    /// la saisie : tant que celle-ci n'est pas une adresse valide et
    /// validée, le vidage reste là où il était.
    mem_addr: u64,
    show_preferences: bool,
    /// Réglages tels qu'ils étaient à l'ouverture de la boîte Préférences.
    /// C'est ce qui donne un sens distinct aux deux boutons : OK garde,
    /// Quitter revient à cet état.
    prefs_snapshot: Option<PreferencesSnapshot>,
    show_about: bool,
    /// L'illustration du bouton de don, montée en texture à la première
    /// ouverture de la boîte puis gardée.
    treat_texture: Option<egui::TextureHandle>,
    /// Le bouton « Rechercher une mise à jour » a été cliqué : la boîte
    /// dit alors franchement que rien n'est branché derrière, plutôt que
    /// de faire mine de chercher.
    about_update_clicked: bool,
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
    /// L'onglet RE est-il dans la barre latérale ? **Volontairement hors
    /// de `settings.json`** : il ouvre un module de reverse engineering
    /// encore à l'état d'ébauche, et il doit être absent à chaque
    /// lancement, pas seulement à l'installation. Enregistré, un « oui »
    /// donné une fois — voire donné par accident, ce que la palette
    /// faisait — le ramenait à toutes les sessions suivantes.
    ///
    /// C'est la seule exception à la règle « tout réglage va dans
    /// settings.json » ; la case de Préférences > Modules le rallume pour
    /// la session en cours.
    show_re_tab: bool,
    /// Le bandeau d'état est-il déployé ? Vrai à chaque lancement, et
    /// **hors de `settings.json`** comme `show_re_tab` : egui garde
    /// normalement l'état des en-têtes repliables d'une session à
    /// l'autre, si bien qu'un repli fait une fois — pour lire un vidage
    /// mémoire, par exemple — rouvrait l'application sur des registres
    /// cachés, sans qu'on sache pourquoi. Il se replie toujours à la
    /// main, le temps de la session.
    cpu_state_open: bool,
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
    /// Panneau bas : la console du programme et les diagnostics de nasm.
    show_console: bool,
    /// Les infobulles au survol (réglage Préférences > Général).
    show_tooltips: bool,
    command_palette: Option<CommandPalette>,
    /// Réglages tels qu'écrits sur le disque. Comparés à l'état courant à
    /// chaque frame : dès qu'ils divergent, le fichier est réécrit. C'est
    /// ce qui garantit qu'aucun réglage ne peut être oublié — il n'y a pas
    /// un appel à "sauvegarder" par endroit où un réglage se change.
    saved_settings: Settings,
    /// Le sélecteur de fichiers natif ouvert en ce moment, s'il y en a un.
    pending_dialog: Option<PendingDialog>,
}

/// Ce qu'on demande au sélecteur natif, et donc ce qu'on fera du chemin
/// qu'il rend.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FileDialogKind {
    OpenFile,
    OpenFolder,
    SaveAs,
}

/// Un sélecteur natif en cours, ouvert sur un thread à part.
///
/// `rfd` n'offre ici que des appels bloquants : `pick_file()` ne rend la
/// main qu'une fois la boîte refermée, et appelé depuis la frame, il
/// arrête la boucle d'évènements d'eframe pendant tout le temps que
/// l'utilisateur passe à chercher son fichier. La fenêtre ne répond alors
/// plus aux pings du compositeur, qui propose au bout de quelques
/// secondes de « forcer à quitter » — l'application n'est pourtant pas
/// plantée, elle attend. Le sélecteur tourne donc sur son propre thread et
/// l'interface vient lire son résultat d'une frame à l'autre.
struct PendingDialog {
    kind: FileDialogKind,
    result: Receiver<Option<PathBuf>>,
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
    GoConsoleTab(ConsoleTab),
    ToggleToolbar,
    ToggleSidebar,
    ToggleStatusbar,
    ToggleConsole,
}

/// `show_re_tab` filtre la seule commande qui mène à une vue
/// désactivable : proposer « Affichage : RE » quand l'onglet est masqué
/// revenait à offrir un raccourci qui le rallume — et le rallumage
/// était persisté, donc définitif.
fn palette_commands(s: &Strings, show_re_tab: bool) -> Vec<(String, PaletteCommand)> {
    let mut commands = vec![
        (
            s.action_assemble.to_string(),
            PaletteCommand::Do(Action::Assemble),
        ),
        (s.action_run.to_string(), PaletteCommand::Do(Action::Run)),
        (
            s.action_step_into.to_string(),
            PaletteCommand::Do(Action::StepInto),
        ),
        (
            s.action_step_over.to_string(),
            PaletteCommand::Do(Action::StepOver),
        ),
        (s.action_stop.to_string(), PaletteCommand::Do(Action::Stop)),
        (
            s.action_reset.to_string(),
            PaletteCommand::Do(Action::Reset),
        ),
        (
            s.action_toggle_explorer.to_string(),
            PaletteCommand::Do(Action::ToggleExplorer),
        ),
        (s.action_save.to_string(), PaletteCommand::Do(Action::Save)),
        (s.file_save_as.to_string(), PaletteCommand::SaveAs),
        (
            s.file_open_file.to_string(),
            PaletteCommand::Do(Action::OpenFile),
        ),
        (
            s.file_open_folder.to_string(),
            PaletteCommand::Do(Action::OpenFolder),
        ),
        (
            s.file_preferences.to_string(),
            PaletteCommand::ShowPreferences,
        ),
        (s.help_about.to_string(), PaletteCommand::ShowAbout),
        (
            format!("{}: Français", s.preferences_language),
            PaletteCommand::SetLang(Lang::Fr),
        ),
        (
            format!("{}: English", s.preferences_language),
            PaletteCommand::SetLang(Lang::En),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_stack),
            PaletteCommand::GoTab(SidebarTab::Stack),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_memory),
            PaletteCommand::GoTab(SidebarTab::Memory),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_output),
            PaletteCommand::GoConsoleTab(ConsoleTab::Output),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_diagnostics),
            PaletteCommand::GoConsoleTab(ConsoleTab::Diagnostics),
        ),
        (
            s.palette_toggle_toolbar.to_string(),
            PaletteCommand::ToggleToolbar,
        ),
        (
            s.palette_toggle_sidebar.to_string(),
            PaletteCommand::ToggleSidebar,
        ),
        (
            s.palette_toggle_statusbar.to_string(),
            PaletteCommand::ToggleStatusbar,
        ),
        (
            format!("{}: {}", s.menu_view, s.view_console),
            PaletteCommand::ToggleConsole,
        ),
        (s.file_quit.to_string(), PaletteCommand::Quit),
    ];

    if show_re_tab {
        commands.push((
            format!("{}: {}", s.menu_view, s.tab_re),
            PaletteCommand::GoTab(SidebarTab::Re),
        ));
    }

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
    show_tooltips: bool,
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
section .text

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
            regs_before: None,
            re_registry,
            status: Status::Idle,
            diagnostics: Vec::new(),
            active_tab: SidebarTab::Stack,
            console_tab: ConsoleTab::Output,
            mem_addr_input: format!("{BASE_ADDR:#x}"),
            mem_addr: BASE_ADDR,
            show_preferences: false,
            prefs_snapshot: None,
            show_about: false,
            treat_texture: None,
            about_update_clicked: false,
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
            cpu_state_open: true,
            console_input: String::new(),
            resume_run_after_input: false,
            explorer: FileExplorer::default(),
            show_explorer: false,
            explorer_just_opened: false,
            explorer_focused: false,
            current_file: None,
            show_toolbar: true,
            show_sidebar: true,
            show_statusbar: true,
            show_console: true,
            show_tooltips: true,
            command_palette: None,
            saved_settings: Settings::default(),
            pending_dialog: None,
        }
    }
}

impl NamsdecApp {
    /// Construit l'application à partir des réglages relus sur le disque.
    fn from_settings(settings: Settings) -> Self {
        // Le dossier des réglages s'il est toujours là, sinon celui d'où
        // l'application a été lancée : l'explorateur ne doit jamais
        // s'ouvrir sur du vide.
        let root = settings
            .explorer_root()
            .filter(|p| p.is_dir())
            .or_else(default_explorer_root);
        let explorer = FileExplorer::with_root(root);
        let mut app = Self {
            theme: settings.theme(),
            lang: settings.lang(),
            keybindings: settings.keybindings(),
            show_toolbar: settings.show_toolbar,
            show_sidebar: settings.show_sidebar,
            show_statusbar: settings.show_statusbar,
            show_console: settings.show_console,
            show_tooltips: settings.show_tooltips,
            explorer,
            saved_settings: settings,
            ..Default::default()
        };
        app.ensure_explorer_root();
        app.reopen_last_file();
        app
    }

    /// Rouvre le fichier sur lequel on travaillait au lancement précédent,
    /// pour reprendre où l'on s'était arrêté.
    ///
    /// Seul le chemin a été enregistré : c'est le fichier du disque qui
    /// fait foi, y compris s'il a été modifié ailleurs entre-temps. En
    /// garder une copie dans les réglages n'y entretiendrait qu'un double
    /// périmé.
    ///
    /// Un fichier disparu ou devenu illisible ne doit ni empêcher de
    /// démarrer, ni accueillir par une erreur quelqu'un qui vient
    /// seulement d'ouvrir l'application : on retombe sur le programme
    /// d'exemple, et l'entrée périmée disparaît des réglages d'elle-même.
    fn reopen_last_file(&mut self) {
        let Some(path) = self
            .saved_settings
            .current_file()
            .filter(|path| path.is_file())
        else {
            self.current_file = None;
            return;
        };
        self.open_file(path);
        if matches!(self.status, Status::Error(_)) {
            self.current_file = None;
            self.source = DEFAULT_SOURCE.to_owned();
            self.status = Status::Idle;
        }
    }

    /// Écrit le fichier de réglages sans attendre qu'un réglage change :
    /// appelé au démarrage pour que l'utilisateur puisse le trouver et
    /// l'éditer dès le premier lancement, avec les défauts du jour gravés
    /// dedans plutôt que de suivre en silence ceux d'une version future.
    ///
    /// Volontairement hors de `from_settings` : construire l'application
    /// ne doit pas écrire sur le disque, sinon les tests qui la
    /// construisent écraseraient la configuration de la machine.
    fn save_settings_now(&mut self) {
        if let Err(e) = self.saved_settings.save() {
            self.status = Status::Error(if self.lang == Lang::Fr {
                format!("impossible d'enregistrer les réglages : {e}")
            } else {
                format!("could not save settings: {e}")
            });
        }
    }

    /// L'état courant des réglages, sous la forme qui part dans le
    /// fichier. Tout ce qui est réglable par l'utilisateur doit figurer
    /// ici — c'est le point unique à compléter en ajoutant un réglage.
    fn settings_snapshot(&self) -> Settings {
        Settings {
            // Toujours la version courante : l'instantané décrit l'état de
            // *cette* version de l'application. Reprendre la version lue
            // dans le fichier ferait diverger l'instantané de ce qui est
            // enregistré, et l'application réécrirait le fichier à chaque
            // frame en croyant qu'un réglage a bougé.
            version: settings::SETTINGS_VERSION,
            theme: self.theme.id.to_owned(),
            language: match self.lang {
                Lang::Fr => "fr".to_owned(),
                Lang::En => "en".to_owned(),
            },
            show_toolbar: self.show_toolbar,
            show_sidebar: self.show_sidebar,
            show_statusbar: self.show_statusbar,
            show_console: self.show_console,
            show_tooltips: self.show_tooltips,
            explorer_root: self.explorer.root.as_ref().map(|p| p.display().to_string()),
            current_file: self.current_file.as_ref().map(|p| p.display().to_string()),
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

    /// Les infobulles s'éteignent par le délai avant affichage plutôt
    /// qu'au cas par cas : porté par le style d'egui, le réglage vaut pour
    /// **toutes** les bulles, y compris celles de l'explorateur — qui est
    /// une crate à part — et celles qu'on ajoutera plus tard. Un délai
    /// infini n'arrive jamais à échéance ; le délai de grâce (qui laisse
    /// la bulle suivante s'afficher tout de suite après une première)
    /// tombe à zéro pour qu'aucune ne passe par cette porte.
    fn apply_tooltip_setting(&self, ctx: &egui::Context) {
        let delay = if self.show_tooltips {
            TOOLTIP_DELAY
        } else {
            f32::INFINITY
        };
        // Le style du thème clair et celui du thème sombre sont deux
        // objets distincts, réglés ensemble juste en dessous : il suffit
        // d'en interroger un.
        if ctx.style_of(egui::Theme::Dark).interaction.tooltip_delay == delay {
            return;
        }
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = delay;
            style.interaction.tooltip_grace_time = if self.show_tooltips {
                TOOLTIP_GRACE_TIME
            } else {
                0.0
            };
        });
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
        self.assemble_once();
        self.show_diagnostics_if_any();
    }

    /// Le panneau bas se met sur ce qui vient de se passer : les
    /// diagnostics si nasm a eu quelque chose à dire, la sortie du
    /// programme sinon. Sans ça, « Erreur d'assemblage » s'affichait dans
    /// la barre de statut sans que la raison soit nulle part en vue.
    fn show_diagnostics_if_any(&mut self) {
        if !self.diagnostics.is_empty() {
            self.show_console = true;
            self.console_tab = ConsoleTab::Diagnostics;
        } else if self.console_tab == ConsoleTab::Diagnostics {
            // Plus rien à signaler : on rend la place à la console, d'où
            // repartira l'exécution.
            self.console_tab = ConsoleTab::Output;
        }
    }

    fn assemble_once(&mut self) {
        match namsdec_asm::assemble(&self.source, BASE_ADDR) {
            Ok(output) => {
                let disasm = disassemble_code(&output);
                let addr_to_line = output.line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
                self.diagnostics = output.diagnostics.clone();
                self.assemble_state = Some(AssembleState {
                    output,
                    disasm,
                    addr_to_line,
                });
                self.status = Status::AssembleOk;
                self.rebuild_emulator();
            }
            // Les diagnostics de l'assemblage précédent n'expliquent rien
            // de ces échecs-là. Laissés en place, `show_diagnostics_if_any`
            // ouvrait le panneau dessus et les présentait, compte à
            // l'appui, comme la cause de l'erreur affichée.
            Err(AsmError::NasmNotFound) => {
                self.diagnostics.clear();
                self.status = Status::NasmMissing;
            }
            Err(
                e @ (AsmError::LinkerNotFound
                | AsmError::LinkFailed { .. }
                | AsmError::UnreadableProgram),
            ) => {
                self.diagnostics.clear();
                self.assemble_state = None;
                self.emulator = None;
                self.status = Status::Error(errors::asm(self.lang, &e));
            }
            Err(AsmError::Failed { diagnostics }) => {
                self.diagnostics = diagnostics;
                self.assemble_state = None;
                self.emulator = None;
                self.status = Status::AssembleFailed;
            }
            Err(e @ AsmError::Io(_)) => {
                self.diagnostics.clear();
                self.status = Status::Error(errors::asm(self.lang, &e));
            }
        }
    }

    /// Retient l'état des registres avant de laisser le programme
    /// avancer.
    fn remember_registers(&mut self) {
        self.regs_before = self.emulator.as_ref().map(|e| e.cpu().regs.clone());
    }

    fn rebuild_emulator(&mut self) {
        let Some(state) = &self.assemble_state else {
            return;
        };
        let mut mem = namsdec_core::Memory::new(BASE_ADDR, MEM_SIZE);
        // `load_addr` plutôt que `BASE_ADDR` : le programme lié dit
        // lui-même où ses sections doivent se trouver, et `.data` n'est
        // pas collée derrière `.text` mais sur la page suivante.
        if mem
            .load(state.output.load_addr, &state.output.bytes)
            .is_err()
        {
            self.status = Status::Error(strings(self.lang).binary_too_large.to_string());
            self.emulator = None;
            return;
        }
        let mut cpu = namsdec_core::Cpu::new(mem);
        // Le point d'entrée de l'ELF, et non le premier octet chargé : sur
        // un programme qui commence par `.data`, ce sont deux adresses
        // différentes.
        cpu.regs.rip = state.output.entry;
        cpu.regs
            .set64(Reg64::Rsp, BASE_ADDR + MEM_SIZE as u64 - 0x100);

        // Un programme qui vient d'être chargé n'a pas de passé : rien à
        // mettre en avant tant qu'il n'a pas avancé d'un pas.
        self.regs_before = None;
        let mut emulator = Emulator::new(cpu);
        emulator.set_breakpoints(
            self.breakpoint_lines
                .iter()
                .filter_map(|l| state.output.line_to_addr.get(l).copied())
                .collect(),
        );

        let ctx = ReContext {
            cpu: emulator.cpu(),
            disasm: &state.disasm,
            source: &self.source,
        };
        self.re_registry.on_binary_loaded(&ctx);

        self.emulator = Some(Box::new(emulator));
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
            emulator.set_breakpoints(addrs);
        }
    }

    fn notify_step(&mut self) {
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            return;
        };
        let ctx = ReContext {
            cpu: emulator.cpu(),
            disasm: &state.disasm,
            source: &self.source,
        };
        self.re_registry.on_step(&ctx);
    }

    fn do_step(&mut self) {
        self.remember_registers();
        // Avancer d'un pas, c'est reprendre la main : un Run suspendu
        // faute d'entrée ne doit pas repartir tout seul à la saisie
        // suivante, sans quoi la ligne tapée pour ce pas-ci relancerait
        // le programme jusqu'au bout.
        self.resume_run_after_input = false;
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match emulator.step() {
            Ok(StepResult::Halted) => {
                self.status = halted_status(emulator.as_ref());
                self.back_to_command_mode();
            }
            Ok(StepResult::NeedsInput) => self.status = Status::WaitingForInput,
            Ok(StepResult::Continued) => {}
            Err(e) => self.status = Status::Error(errors::emu(self.lang, &e)),
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
        // Photographie prise au départ : à l'arrêt suivant — un point
        // d'arrêt, un `hlt` — le bandeau montre tout ce que la course a
        // changé, et non le seul dernier pas.
        self.remember_registers();
        self.resume_run_after_input = false;
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
                self.status = halted_status(emulator.as_ref());
                self.back_to_command_mode();
            }
            Ok(RunStop::Breakpoint(addr)) => {
                self.running = false;
                self.status = Status::Breakpoint(addr);
            }
            Ok(RunStop::NeedsInput) => {
                // Suspendu, pas arrêté : l'exécution redémarrera d'
                // elle-même dès que l'utilisateur aura tapé quelque chose.
                self.running = false;
                self.resume_run_after_input = true;
                self.status = Status::WaitingForInput;
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
                self.status = Status::Error(errors::emu(self.lang, &e));
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
        // Le drapeau tombe même quand rien ne tourne : un Run suspendu
        // faute d'entrée a `running == false` et attend pourtant de
        // repartir tout seul. Sans ça, Stop le laissait armé.
        self.resume_run_after_input = false;
        if self.running {
            self.running = false;
            self.status = Status::Idle;
        }
    }

    fn do_step_over(&mut self) {
        self.remember_registers();
        // Même raison que dans `do_step`.
        self.resume_run_after_input = false;
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match emulator.step_over(MAX_RUN_STEPS) {
            Ok(StepOverResult::Halted) => {
                self.status = halted_status(emulator.as_ref());
                self.back_to_command_mode();
            }
            Ok(StepOverResult::Breakpoint(addr)) => self.status = Status::Breakpoint(addr),
            Ok(StepOverResult::StepLimit) => self.status = Status::StepLimit,
            Ok(StepOverResult::NeedsInput) => self.status = Status::WaitingForInput,
            Ok(StepOverResult::Stepped | StepOverResult::SteppedOverCall) => {}
            Err(e) => self.status = Status::Error(errors::emu(self.lang, &e)),
        }
        self.notify_step();
    }

    /// Point d'entrée unique pour tout ce qui déclenche une action —
    /// raccourci clavier ou clic de bouton/menu, pour ne jamais avoir deux
    /// implémentations de "ce que fait Assembler" qui divergent.
    /// Le programme chargé est-il allé jusqu'au bout ? `halted` couvre les
    /// deux fins possibles : `exit`, et la machine arrêtée par `hlt`.
    fn program_finished(&self) -> bool {
        self.emulator
            .as_ref()
            .is_some_and(|emulator| emulator.cpu().halted)
    }

    /// Les commandes d'exécution ont-elles encore un sens ?
    ///
    /// Il faut un programme chargé, et qu'il lui reste quelque chose à
    /// faire : relancer Step Into sur un programme terminé ne peut plus
    /// rien produire, et laisser le bouton actif laisse croire le
    /// contraire. `Réinitialiser` reste disponible — c'est précisément ce
    /// qu'on veut faire ensuite.
    fn can_execute(&self) -> bool {
        self.emulator.is_some() && !self.program_finished()
    }

    /// Ce que fait namsdec quand l'exécution rend la main, comme gdb :
    /// on revient aux commandes. Rester en mode Insertion ferait taper du
    /// texte dans le code à quelqu'un qui pianote `Espace r` pour
    /// relancer.
    fn back_to_command_mode(&mut self) {
        self.editor_mode = EditorMode::Normal;
    }

    fn dispatch(&mut self, action: Action) {
        // Un raccourci doit obéir aux mêmes règles qu'un bouton : si la
        // commande est grisée dans la barre d'outils, la frapper au
        // clavier ne doit pas la déclencher non plus.
        if matches!(action, Action::Run | Action::StepInto | Action::StepOver)
            && !self.can_execute()
        {
            return;
        }

        match action {
            Action::ToggleExplorer => {
                self.show_explorer = !self.show_explorer;
                self.explorer_just_opened = self.show_explorer;
                if self.show_explorer {
                    self.ensure_explorer_root();
                }
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
        self.spawn_file_dialog(FileDialogKind::OpenFolder);
    }

    /// Sélecteur natif de fichier. Accessible directement (Ctrl+O, menu
    /// Fichier, palette) sans avoir à ouvrir l'explorateur d'abord.
    fn do_open_file_dialog(&mut self) {
        self.spawn_file_dialog(FileDialogKind::OpenFile);
    }

    /// Ouvre le sélecteur natif sur un thread et note qu'on attend sa
    /// réponse — voir `PendingDialog` pour la raison du thread.
    ///
    /// Un seul à la fois : tant qu'une boîte est ouverte, un second
    /// raccourci n'en empile pas une deuxième par-dessus.
    fn spawn_file_dialog(&mut self, kind: FileDialogKind) {
        if self.pending_dialog.is_some() {
            return;
        }
        let dialog = self.file_dialog(kind);
        let (sender, result) = mpsc::channel();
        std::thread::spawn(move || {
            let picked = match kind {
                FileDialogKind::OpenFile => dialog.pick_file(),
                FileDialogKind::OpenFolder => dialog.pick_folder(),
                FileDialogKind::SaveAs => dialog.save_file(),
            };
            // Si l'application s'est arrêtée entre-temps, le résultat
            // n'intéresse plus personne : l'envoi échoue et le thread
            // s'achève là.
            let _ = sender.send(picked);
        });
        self.pending_dialog = Some(PendingDialog { kind, result });
    }

    /// Le sélecteur configuré pour ce qu'on cherche — ses filtres pour un
    /// fichier (source assembleur, binaire, ou tout), rien de particulier
    /// pour un dossier ou un enregistrement.
    fn file_dialog(&self, kind: FileDialogKind) -> rfd::FileDialog {
        let s = strings(self.lang);
        let dialog = rfd::FileDialog::new();
        match kind {
            FileDialogKind::OpenFile => dialog
                .add_filter(s.filter_asm, &["asm", "nasm", "s", "inc"])
                .add_filter(
                    s.filter_binary,
                    &["bin", "o", "obj", "elf", "so", "exe", "dll"],
                )
                // "*" est traduit en glob `*` par le backend portail de
                // rfd : c'est ce qui laisse choisir un exécutable ELF sans
                // extension, cas le plus courant sous Linux.
                .add_filter(s.filter_all, &["*"]),
            FileDialogKind::OpenFolder | FileDialogKind::SaveAs => dialog,
        }
    }

    /// Relit la réponse du sélecteur ouvert, s'il en a donné une.
    fn poll_file_dialog(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.pending_dialog else {
            return;
        };
        let kind = pending.kind;
        match pending.result.try_recv() {
            Ok(picked) => {
                self.pending_dialog = None;
                if let Some(path) = picked {
                    self.finish_file_dialog(kind, path);
                }
            }
            // Toujours ouvert. Aucun évènement n'arrive plus sur la
            // fenêtre pendant qu'une boîte modale est affichée : sans ce
            // réveil programmé, la réponse resterait dans le tuyau
            // jusqu'au prochain mouvement de souris.
            Err(TryRecvError::Empty) => ctx.request_repaint_after(DIALOG_POLL_INTERVAL),
            // Le thread s'est terminé sans rien envoyer : plus personne
            // ne répondra, on cesse d'attendre.
            Err(TryRecvError::Disconnected) => self.pending_dialog = None,
        }
    }

    /// Ce qu'on fait du chemin choisi, une fois la boîte refermée.
    fn finish_file_dialog(&mut self, kind: FileDialogKind, path: PathBuf) {
        match kind {
            FileDialogKind::OpenFile => self.open_file(path),
            FileDialogKind::OpenFolder => {
                self.explorer.root = Some(path);
                self.show_explorer = true;
                // Même garde que pour la bascule : le clic qui a validé la
                // boîte native ne doit pas refermer aussitôt la fenêtre
                // qu'il vient de faire apparaître.
                self.explorer_just_opened = true;
                self.focus_explorer(true);
            }
            FileDialogKind::SaveAs => self.write_to(&path),
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
                load_addr: BASE_ADDR,
                entry: BASE_ADDR,
                // Un binaire brut ouvert depuis le disque n'a pas de
                // sections : tout ce qu'on en sait, c'est qu'on l'a
                // désassemblé comme du code depuis son premier octet.
                sections: Vec::new(),
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

    /// Garantit que l'explorateur a un dossier à montrer. Sans ça, il
    /// s'ouvre sur une invite "Aucun dossier ouvert" qui oblige à passer
    /// par une boîte de dialogue avant de voir quoi que ce soit — alors
    /// qu'il y a toujours un dossier évident à afficher : celui depuis
    /// lequel l'application a été lancée, comme `nvim .`.
    fn ensure_explorer_root(&mut self) {
        if self.explorer.root.is_none() {
            self.explorer.root = default_explorer_root();
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
            path_hint: s.explorer_path_hint,
            go_parent: s.explorer_go_parent,
            go_root: s.explorer_go_root,
            go_home: s.explorer_go_home,
            go_current_file: s.explorer_go_current_file,
            rename_hint: s.explorer_rename_hint,
            rename_failed: s.explorer_rename_failed,
            rename_exists: s.explorer_rename_exists,
            path_not_a_directory: s.explorer_path_not_a_directory,
        };

        let response = self.explorer.show(
            ctx,
            self.theme,
            &labels,
            self.explorer_focused,
            self.current_file.as_deref(),
        );

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
            ExplorerAction::Renamed { from, to } => {
                // Si c'est le fichier ouvert qui vient d'être renommé, le
                // tampon doit suivre : sans ça, le prochain Enregistrer
                // réécrirait l'ancien chemin et ferait réapparaître le
                // fichier sous son ancien nom.
                if self.current_file.as_deref() == Some(from.as_path()) {
                    self.current_file = Some(to);
                }
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
                // Aucune bascule de réglage ici : la commande n'existe
                // que pour les vues déjà visibles (voir
                // `palette_commands`). Se rendre quelque part ne doit pas
                // modifier — ni enregistrer — ce qu'on affiche.
                self.show_sidebar = true;
                self.active_tab = tab;
            }
            PaletteCommand::GoConsoleTab(tab) => {
                self.show_console = true;
                self.console_tab = tab;
            }
            PaletteCommand::ToggleConsole => self.show_console = !self.show_console,
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
        let commands = palette_commands(s, self.show_re_tab);

        let (mut query, mut selected) = {
            let p = self.command_palette.as_ref().unwrap();
            (p.query.clone(), p.selected)
        };

        let query_lower = query.to_lowercase();
        let filtered: Vec<&(String, PaletteCommand)> = commands
            .iter()
            .filter(|(label, _)| {
                query_lower.is_empty() || label.to_lowercase().contains(&query_lower)
            })
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
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
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
        self.spawn_file_dialog(FileDialogKind::SaveAs);
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
        state.addr_to_line.get(&emulator.cpu().regs.rip).copied()
    }

    fn menu_bar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal(|ui| {
            ui.menu_button(s.menu_file, |ui| {
                let ctx = ui.ctx().clone();
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.file_open_file,
                        Action::OpenFile,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::OpenFile);
                    ui.close();
                }
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.file_open_folder,
                        Action::OpenFolder,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::OpenFolder);
                    ui.close();
                }
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        &ui.ctx().clone(),
                        s.action_save,
                        Action::Save,
                    ))
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
                let can_exec = self.can_execute();
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
                ui.checkbox(&mut self.show_console, s.view_console);

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
                for tab in ConsoleTab::ALL {
                    if ui.button(tab.label(s)).clicked() {
                        self.show_console = true;
                        self.console_tab = tab;
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
                show_tooltips: self.show_tooltips,
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
            self.show_tooltips = snapshot.show_tooltips;
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

        ui.checkbox(&mut self.show_tooltips, s.preferences_tooltips);
        ui.weak(s.preferences_tooltips_hint);

        ui.separator();

        ui.label(s.preferences_language);
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.lang == Lang::Fr, "Français")
                .clicked()
            {
                self.lang = Lang::Fr;
            }
            if ui
                .selectable_label(self.lang == Lang::En, "English")
                .clicked()
            {
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
            && let Some(chord) = self.dispatcher.poll_rebind(&ctx, rebinding)
        {
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
                        // Une séquence n'a pas de longueur imposée : sans
                        // voir ce qu'il a déjà tapé, l'utilisateur ne sait
                        // pas s'il doit continuer ou valider.
                        let captured: Vec<&str> = self
                            .rebinding
                            .as_ref()
                            .map(|r| r.captured().iter().map(|k| k.name()).collect())
                            .unwrap_or_default();
                        if captured.is_empty() {
                            ui.colored_label(theme.accent, s.rebind_waiting);
                        } else {
                            ui.colored_label(theme.accent, format!("{} …", captured.join(" ")));
                        }
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
                        self.rebinding =
                            Some(Rebinding::new(action, keybindings::RebindMode::Simple));
                    }
                    if ui
                        .add_enabled(!is_rebinding, egui::Button::new(s.rebind_leader))
                        .clicked()
                    {
                        self.rebinding =
                            Some(Rebinding::new(action, keybindings::RebindMode::Leader));
                    }
                    ui.end_row();
                }
            });
    }

    fn preferences_modules_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.label(s.modules_legend);
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .show(ui, |ui| {
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

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
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

    /// L'illustration du bouton de don. Montée en texture au premier
    /// affichage : la boîte « À propos » s'ouvre rarement, rien ne
    /// justifie de la charger au démarrage.
    fn treat_texture(&mut self, ctx: &egui::Context) -> egui::TextureHandle {
        self.treat_texture
            .get_or_insert_with(|| {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [TREAT_IMAGE_SIDE, TREAT_IMAGE_SIDE],
                    TREAT_IMAGE,
                );
                ctx.load_texture("offrir-un-repas", image, egui::TextureOptions::LINEAR)
            })
            .clone()
    }

    fn about_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_about;
        let mut close = false;
        egui::Window::new(s.about_title)
            // Un identifiant explicite : sans lui, egui dérive celui de
            // la fenêtre de son *titre*, qui change avec la langue —
            // passer en anglais lui faisait perdre sa position.
            .id(about_window_id())
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            // Posée au centre à chaque frame, et non « ancrée » : l'ancre
            // d'egui aligne la boîte dans la zone qu'il lui laisse, ce
            // qui la décale vers le bas des panneaux. Une boîte « À
            // propos » n'a pas de place à retenir, elle s'ouvre au milieu
            // de la fenêtre.
            .pivot(egui::Align2::CENTER_CENTER)
            .current_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                ui.heading("Namsdec");
                ui.label(format!(
                    "Version {APP_VERSION} ({} {BUILD_DATE})",
                    s.about_build_word
                ));
                ui.label(s.about_description);

                ui.separator();
                ui.label(s.about_license);
                ui.hyperlink_to(s.about_license_link, MIT_LICENSE_URL);

                ui.separator();
                ui.label(s.about_donate_hint);
                let can_donate = !DONATION_URL.is_empty();
                let texture = self.treat_texture(ctx);
                ui.add_enabled_ui(can_donate, |ui| {
                    // `Image::sense` plutôt qu'un `ImageButton` : egui
                    // 0.36 a retiré ce dernier, une image cliquable le
                    // remplace.
                    let image = egui::Image::new(egui::load::SizedTexture::from_handle(&texture))
                        .fit_to_exact_size(egui::vec2(TREAT_IMAGE_SHOWN, TREAT_IMAGE_SHOWN))
                        .corner_radius(6)
                        .sense(egui::Sense::click());
                    if ui.add(image).on_hover_text(s.about_donate).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(DONATION_URL));
                    }
                });
                if !can_donate {
                    ui.colored_label(theme.dim, s.about_donate_unset);
                }

                ui.separator();
                if ui.button(s.about_check_update).clicked() {
                    self.about_update_clicked = true;
                }
                if self.about_update_clicked {
                    ui.colored_label(theme.warn, s.about_check_update_todo);
                }

                ui.separator();
                // Un seul bouton : il n'y a rien à annuler dans une boîte
                // qui ne fait que renseigner. « Quitter » n'y aurait fait
                // que doubler « OK ».
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let ok = egui::Button::new(egui::RichText::new(s.dialog_ok).color(theme.bg))
                        .fill(theme.accent);
                    close = ui.add(ok).clicked();
                });
            });
        self.show_about = open && !close;
        if !self.show_about {
            // La note de mise à jour ne survit pas à la fermeture.
            self.about_update_clicked = false;
        }
    }

    fn toolbar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            if ui
                .button(s.action_assemble)
                .on_hover_text(shortcut_hint(
                    &self.keybindings,
                    &ctx,
                    s.action_assemble,
                    Action::Assemble,
                ))
                .clicked()
            {
                self.dispatch(Action::Assemble);
            }

            let can_exec = self.can_execute();
            ui.add_enabled_ui(can_exec, |ui| {
                if icon_button(ui, icon_run, s.action_run)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.action_run,
                        Action::Run,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::Run);
                }
                if icon_button(ui, icon_step_into, s.action_step_into)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.action_step_into,
                        Action::StepInto,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::StepInto);
                }
                if icon_button(ui, icon_step_over, s.action_step_over)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.action_step_over,
                        Action::StepOver,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::StepOver);
                }
            });
            ui.add_enabled_ui(self.running, |ui| {
                if icon_button(ui, icon_stop, s.action_stop)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.action_stop,
                        Action::Stop,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::Stop);
                }
            });
        });
    }

    fn statusbar_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let lang = self.lang;
        let theme = self.theme;

        // Toutes les touches déjà tapées, pas seulement la première : sur
        // `Espace s i`, l'indicateur doit montrer où l'on en est dans la
        // séquence, sinon il ne dit plus rien après la deuxième frappe.
        let pending = self.dispatcher.pending();
        if !pending.is_empty() {
            let keys = pending
                .iter()
                .map(|k| k.name())
                .collect::<Vec<_>>()
                .join(" ");
            let text = if lang == Lang::Fr {
                format!("{keys} — touche suivante attendue…")
            } else {
                format!("{keys} — waiting for next key…")
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
    }

    fn status_message_ui(&self, ui: &mut egui::Ui) {
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
            // Une pastille peinte, pas un glyphe : "✓" et "✗" ne sont pas
            // dans la police par défaut d'egui et retombaient sur le carré
            // de remplacement. Le compte d'octets et de diagnostics n'y
            // figure plus non plus — les diagnostics sont listés juste en
            // dessous, avec leur ligne, et la taille du binaire
            // n'apprenait rien d'utile à cet endroit.
            Status::AssembleOk => {
                status_dot(ui, theme.good);
                ui.colored_label(theme.good, s.status_assembled);
            }
            Status::AssembleFailed => {
                status_dot(ui, theme.bad);
                ui.colored_label(theme.bad, s.status_assemble_failed);
            }
            Status::Halted => {
                ui.colored_label(theme.good, s.status_halted);
            }
            Status::WaitingForInput => {
                ui.colored_label(theme.accent, s.status_waiting_for_input);
            }
            Status::Exited { code } => {
                let color = if *code == 0 { theme.good } else { theme.bad };
                ui.colored_label(
                    color,
                    if lang == Lang::Fr {
                        format!("Programme terminé (code {code}).")
                    } else {
                        format!("Program exited (code {code}).")
                    },
                );
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
            // Les touches déjà prises par un raccourci ne sont plus
            // disponibles ici : le `i` qui termine `Espace s i` a servi à
            // demander un pas, pas à ouvrir la saisie.
            let consumed = self.dispatcher.consumed().to_vec();
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
                            egui::Key::I if plain && !consumed.contains(key) => enter = true,
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
                    let galley = layout_code(ui, theme, &self.source);

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
                            let color = if is_current {
                                theme.accent
                            } else {
                                ui.visuals().weak_text_color()
                            };
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

                        // La largeur de repli que propose egui est
                        // délibérément ignorée : voir `layout_code`.
                        let mut layouter =
                            |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
                                layout_code(ui, theme, buf.as_str())
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
        // sur la pile plutôt que de laisser un panneau vide.
        if self.active_tab == SidebarTab::Re && !self.show_re_tab {
            self.active_tab = SidebarTab::Stack;
        }

        // Le bandeau d'état, en haut et toujours là : registres et flags
        // sont ce qu'on surveille en avançant instruction par
        // instruction, les faire réapparaître par un onglet coupait le
        // geste en deux. Repliable quand même — un binaire ouvert pour
        // être lu n'a pas besoin d'un processeur en marche.
        // `show_unindented` : le retrait qu'ajoute un en-tête repliable
        // vaut une vingtaine de pixels, et une valeur de registre fait
        // seize chiffres — avec le retrait, la ligne débordait du panneau.
        // `open(Some(...))` plutôt que `default_open(true)` : le premier
        // impose l'état à chaque frame, le second ne vaut qu'à la toute
        // première ouverture et laisse ensuite egui ressortir ce qu'il a
        // retenu de la session précédente.
        let banner = egui::CollapsingHeader::new(s.cpu_state)
            .open(Some(self.cpu_state_open))
            .show_unindented(ui, |ui| {
                // Seize registres et RIP font une colonne haute : bornée à
                // la moitié du panneau, elle défile pour son compte au
                // lieu de repousser les onglets hors de vue sur une
                // fenêtre courte. Tant qu'il y a la place, aucune barre
                // n'apparaît.
                egui::ScrollArea::vertical()
                    .id_salt("cpu_state")
                    .max_height(ui.available_height() * 0.6)
                    .show(ui, |ui| self.cpu_state_ui(ui));
            });
        // L'état imposé prive l'en-tête de son propre clic : on le
        // reprend ici, sinon le bandeau ne se replierait plus du tout.
        if banner.header_response.clicked() {
            self.cpu_state_open = !self.cpu_state_open;
        }

        ui.separator();
        ui.horizontal_wrapped(|ui| {
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
        // Chaque vue pose sa propre zone de défilement : un vidage
        // mémoire défile dans les deux sens (seize octets et leur
        // transcription, c'est plus large que le panneau), mais une
        // commande — le champ Adresse — reste bornée par le panneau.
        // Une zone de défilement horizontale commune les étirait toutes
        // les deux à la largeur du vidage.
        match self.active_tab {
            SidebarTab::Stack => self.stack_ui(ui),
            SidebarTab::Memory => self.memory_ui(ui),
            SidebarTab::Re => self.re_ui(ui),
        }
    }

    /// Le bandeau fixe : les registres, puis les flags en pastilles.
    fn cpu_state_ui(&self, ui: &mut egui::Ui) {
        self.registers_ui(ui);
        ui.add_space(6.0);
        self.flags_ui(ui);
    }

    /// Le panneau bas : la console du programme et les diagnostics de
    /// nasm, sur toute la largeur de la fenêtre.
    ///
    /// La console y a la largeur d'un terminal — un programme écrit en 80
    /// colonnes, le panneau latéral n'en offrait que trente-quatre — et
    /// les diagnostics y trouvent enfin de quoi être lus : ils étaient
    /// jusqu'ici entassés sous la barre de statut, sur deux lignes
    /// hautes de rien.
    fn console_panel_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal_wrapped(|ui| {
            for tab in ConsoleTab::ALL {
                // Le compte des diagnostics se lit sur l'onglet : c'est
                // ce qui fait remarquer un avertissement qu'on n'était
                // pas allé chercher.
                let label = if tab == ConsoleTab::Diagnostics && !self.diagnostics.is_empty() {
                    format!("{} ({})", tab.label(s), self.diagnostics.len())
                } else {
                    tab.label(s).to_owned()
                };
                if ui
                    .selectable_label(self.console_tab == tab, label)
                    .clicked()
                {
                    self.console_tab = tab;
                }
            }
        });
        ui.separator();
        match self.console_tab {
            ConsoleTab::Output => self.output_ui(ui),
            ConsoleTab::Diagnostics => self.diagnostics_ui(ui),
        }
    }

    /// Ce que nasm a dit de la source, une ligne par diagnostic.
    fn diagnostics_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (lang, theme) = (self.lang, self.theme);
        if self.diagnostics.is_empty() {
            ui.colored_label(theme.dim, s.no_diagnostics);
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for d in &self.diagnostics {
                    let color = if d.severity == Severity::Error {
                        theme.bad
                    } else {
                        theme.warn
                    };
                    let text = if lang == Lang::Fr {
                        format!("ligne {} — {}", d.line, d.message)
                    } else {
                        format!("line {} — {}", d.line, d.message)
                    };
                    ui.colored_label(color, egui::RichText::new(text).monospace());
                }
            });
    }

    fn registers_ui(&self, ui: &mut egui::Ui) {
        let Some(emulator) = &self.emulator else {
            ui.label(strings(self.lang).no_binary_assembled);
            return;
        };
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        let changed = changed_registers(self.regs_before.as_ref(), regs);
        // Dix-sept lignes d'affilée, c'est haut. Le panneau élargi, elles
        // tiennent sur deux colonnes et le bandeau retrouve la moitié de
        // sa hauteur — celui qui garde le panneau étroit garde une
        // colonne, qui défile.
        let pairs = register_columns(ui.available_width());
        egui::Grid::new("registers_grid")
            // Deux cellules par paire, plus une cellule vide entre les
            // paires : c'est elle qui sépare les colonnes.
            .num_columns(3 * pairs - 1)
            .striped(true)
            .show(ui, |ui| {
                // Entre la valeur d'une paire et le nom de la suivante,
                // l'espacement ordinaire de la grille ne suffit pas :
                // seize chiffres suivis de trois lettres se lisent comme
                // un seul mot (« …003c RBX »). Une cellule vide, large de
                // trois espacements, sépare les deux colonnes.
                let gap = ui.spacing().item_spacing.x * 3.0;
                let mut in_row = 0;
                for (index, (name, value)) in REGISTERS
                    .iter()
                    .map(|&(name, reg)| (name, regs.get64(reg)))
                    .chain(std::iter::once(("RIP", regs.rip)))
                    .enumerate()
                {
                    // RIP n'est pas un registre comme un autre ici :
                    // c'est lui que la gouttière suit, et c'est la ligne
                    // qu'on cherche des yeux après chaque pas. Il change
                    // à chaque instruction, le mettre en avant pour ça
                    // n'apprendrait rien.
                    let color = if name == "RIP" {
                        theme.accent
                    } else if changed.contains(&name) {
                        theme.warn
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(color, egui::RichText::new(name).monospace());
                    ui.colored_label(
                        color,
                        egui::RichText::new(format!("{value:016x}")).monospace(),
                    );
                    in_row += 1;
                    if in_row == pairs {
                        ui.end_row();
                        in_row = 0;
                    } else if index != REGISTERS.len() {
                        // `allocate_space` et non `add_space` : dans une
                        // grille, la seconde avance un curseur qui
                        // n'existe pas (egui le refuse). Celle-ci réserve
                        // une vraie cellule, vide et large de `gap`.
                        ui.allocate_space(egui::vec2(gap, 0.0));
                    }
                }
                // RIP peut rester seul sur sa ligne : elle se ferme quand
                // même, sinon la grille avale la dernière.
                if in_row != 0 {
                    ui.end_row();
                }
            });
    }

    fn flags_ui(&self, ui: &mut egui::Ui) {
        let Some(emulator) = &self.emulator else {
            return;
        };
        let f = emulator.cpu().flags;
        let theme = self.theme;
        ui.horizontal_wrapped(|ui| {
            for (name, bit) in [
                ("CF", RFlags::CF),
                ("PF", RFlags::PF),
                ("AF", RFlags::AF),
                ("ZF", RFlags::ZF),
                ("SF", RFlags::SF),
                ("OF", RFlags::OF),
            ] {
                flag_chip(ui, theme, name, f.contains(bit));
            }
        });
    }

    /// La console du programme : ce qu'il a écrit, et de quoi lui donner
    /// ce qu'il veut lire.
    ///
    /// Les octets de sortie sont rendus en UTF-8 « au mieux » : un
    /// programme en assembleur écrit ce qu'il veut, y compris des octets
    /// qui ne forment aucun texte, et ce n'est pas une raison pour ne rien
    /// afficher.
    ///
    /// La sortie a sa fenêtre à elle — encadrée, qui garde sa place même
    /// vide et défile pour son compte — plutôt que d'être une suite de
    /// lignes posées au-dessus du champ de saisie : un programme bavard
    /// repoussait ce champ hors de vue, et il fallait faire défiler pour
    /// lui répondre.
    fn output_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // La fenêtre de sortie est là dans tous les cas : sans programme
        // chargé elle le dit, plutôt que de laisser l'onglet vide.
        let (output, exit_code, pending) = match &self.emulator {
            Some(emulator) => (
                String::from_utf8_lossy(emulator.output()).into_owned(),
                emulator.exit_code(),
                emulator.pending_input(),
            ),
            None => (String::new(), None, 0),
        };

        // Posée en bas du panneau, la saisie ne bouge plus quoi qu'écrive
        // le programme. Elle n'apparaît que s'il y a quelqu'un pour lire.
        if self.emulator.is_some() {
            egui::Panel::bottom("console_input")
                .resizable(false)
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    if let Some(code) = exit_code {
                        ui.separator();
                        let color = if code == 0 {
                            self.theme.good
                        } else {
                            self.theme.bad
                        };
                        ui.colored_label(
                            color,
                            if self.lang == Lang::Fr {
                                format!("Le programme s'est terminé avec le code {code}.")
                            } else {
                                format!("The program exited with code {code}.")
                            },
                        );
                    }
                    self.console_input_ui(ui, pending);
                });
        }

        // Le reste de la hauteur revient à la sortie, sur le fond sombre
        // d'un terminal.
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    // Ce que le programme vient d'écrire est ce qu'on veut
                    // lire : la vue suit le bas, comme un terminal.
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.emulator.is_none() {
                            ui.colored_label(self.theme.dim, s.no_binary_assembled);
                        } else if output.is_empty() {
                            ui.colored_label(self.theme.dim, s.output_empty);
                        } else {
                            // Sélectionnable : la sortie d'un programme se
                            // recopie ailleurs (un rapport, un test).
                            ui.add(
                                egui::Label::new(egui::RichText::new(output).monospace())
                                    .selectable(true),
                            );
                        }
                    });
            });
    }

    /// Le côté « entrée » de la console.
    ///
    /// Une ligne validée part avec son saut de ligne, parce que c'est ce
    /// qu'un programme attend d'un terminal : il lit jusqu'au `\n`, et une
    /// ligne sans fin de ligne le laisserait croire qu'elle n'est pas
    /// terminée.
    fn console_input_ui(&mut self, ui: &mut egui::Ui, pending: usize) {
        let s = strings(self.lang);
        let waiting = matches!(self.status, Status::WaitingForInput);

        ui.separator();
        if waiting {
            ui.colored_label(self.theme.accent, s.console_waiting);
        }

        let mut send = false;
        ui.horizontal(|ui| {
            // Les boutons sont posés en premier, calés à droite, et le
            // champ prend ce qui reste. Deviner sa largeur (« la place
            // disponible moins 120 px ») la surestimait de quelques
            // pixels : la ligne débordait du panneau latéral, et ce
            // débordement était repris comme sa nouvelle largeur à la
            // frame suivante. L'onglet Console élargissait donc le
            // panneau sans fin, jusqu'à ce qu'il occupe la fenêtre.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Fin d'entrée : un programme qui lit jusqu'au bout de son
                // entrée attendrait indéfiniment sans ce bouton.
                if ui
                    .button(s.console_eof)
                    .on_hover_text(s.console_eof_hint)
                    .clicked()
                {
                    self.send_input(None);
                }
                send = ui.button(s.console_send).clicked();

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.console_input)
                        .hint_text(s.console_input_hint)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width()),
                );
                // Le programme réclame de quoi lire : le curseur va là où
                // l'utilisateur doit taper, sans qu'il ait à le chercher.
                if waiting && !response.has_focus() {
                    response.request_focus();
                }
                send |= response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        });

        if pending > 0 {
            ui.colored_label(
                self.theme.dim,
                if self.lang == Lang::Fr {
                    format!("{pending} octet(s) en attente d'être lus.")
                } else {
                    format!("{pending} byte(s) waiting to be read.")
                },
            );
        }

        if send {
            let line = std::mem::take(&mut self.console_input);
            self.send_input(Some(line));
        }
    }

    /// Donne au programme la ligne saisie, ou ferme son entrée (`None`),
    /// puis relance l'exécution si c'est elle qui attendait.
    fn send_input(&mut self, line: Option<String>) {
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match line {
            Some(line) => emulator.provide_input(format!("{line}\n").as_bytes()),
            None => emulator.close_input(),
        }

        // Un `Exécuter` suspendu repart tout seul : c'est le programme qui
        // attendait, pas l'utilisateur. Un pas à pas, lui, garde la main à
        // l'utilisateur, qui décide quand avancer.
        if matches!(self.status, Status::WaitingForInput) {
            self.status = if self.resume_run_after_input {
                self.resume_run_after_input = false;
                self.running = true;
                Status::Running
            } else {
                Status::Idle
            };
        }
    }

    fn stack_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            ui.label(s.no_binary_assembled);
            return;
        };
        let addr = emulator.cpu().regs.get64(Reg64::Rsp).saturating_sub(16);
        let dump = safe_hexdump(&emulator.cpu().mem, addr, 64, s.address_out_of_range);
        hexdump_ui(ui, "stack_dump", &dump);
    }

    fn memory_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            ui.label(s.no_binary_assembled);
            return;
        };
        // Le champ est posé *avant* la zone de défilement, et non dedans :
        // à l'intérieur, il se serait étiré à la largeur du vidage.
        let theme = self.theme;
        let typed = parse_addr(&self.mem_addr_input);
        let mut go = false;
        ui.horizontal(|ui| {
            ui.label(s.memory_address_label);
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.mem_addr_input)
                    .id(memory_address_id())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(address_field_width(ui)),
            );
            // Le refus se voit sur le champ lui-même, peint par-dessus :
            // le liseré d'egui change avec le focus, celui-ci ne dépend
            // que de ce qui est écrit.
            if typed.is_none() {
                ui.painter().rect_stroke(
                    response.rect,
                    ui.visuals().widgets.inactive.corner_radius,
                    egui::Stroke::new(1.0, theme.bad),
                    egui::StrokeKind::Inside,
                );
            }
            // Entrée vaut Go : c'est le geste du clavier, et il évite
            // d'aller chercher le bouton à la souris.
            go |= response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.add_enabled_ui(typed.is_some(), |ui| {
                go |= icon_square_button(ui, icon_go)
                    .on_hover_text(s.memory_go_hint)
                    .clicked();
            });
        });
        // Rien n'a bougé tant que l'adresse n'est pas validée : le vidage
        // ne saute plus à chaque frappe, et une saisie invalide le laisse
        // là où il est au lieu de le renvoyer au début du programme.
        if go {
            self.mem_addr = validated_address(self.mem_addr, &self.mem_addr_input);
        }
        if typed.is_none() {
            ui.colored_label(theme.bad, s.memory_address_invalid);
        }
        let dump = safe_hexdump(
            &emulator.cpu().mem,
            self.mem_addr,
            128,
            s.address_out_of_range,
        );
        hexdump_ui(ui, "memory_dump", &dump);
    }

    fn re_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            ui.label(s.no_binary_assembled);
            return;
        };
        let ctx = ReContext {
            cpu: emulator.cpu(),
            disasm: &state.disasm,
            source: &self.source,
        };
        // Défilement vertical seulement : les modules posent des widgets
        // ordinaires, qui prennent la largeur qu'on leur laisse. Dans une
        // zone qui défile aussi horizontalement, cette largeur n'a plus
        // de fin.
        egui::ScrollArea::vertical().show(ui, |ui| {
            for module in self.re_registry.iter_mut() {
                ui.group(|ui| {
                    ui.strong(module.title());
                    module.ui(ui, &ctx);
                });
            }
        });
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
                    format!(
                        "{} instructions désassemblées (vérification)",
                        state.disasm.len()
                    )
                } else {
                    format!(
                        "{} instructions disassembled (verification)",
                        state.disasm.len()
                    )
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
                let state_word = match (emulator.cpu().halted, fr) {
                    (true, true) => "arrêté (hlt)",
                    (true, false) => "halted (hlt)",
                    (false, true) => "en cours",
                    (false, false) => "running",
                };
                let bp_word = if fr {
                    "point(s) d'arrêt"
                } else {
                    "breakpoint(s)"
                };
                format!(
                    "RIP={:#x} · {} {bp_word} · {state_word}",
                    emulator.cpu().regs.rip,
                    emulator.breakpoints().len(),
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
        module_card(
            ui,
            "namsdec-re",
            namsdec_re::VERSION,
            true,
            theme.good,
            &re_status,
        );
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
        let ok =
            egui::Button::new(egui::RichText::new(s.dialog_ok).color(theme.bg)).fill(theme.accent);
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
fn shortcut_hint(
    bindings: &KeyBindings,
    ctx: &egui::Context,
    label: &str,
    action: Action,
) -> String {
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
        let text_pos = egui::pos2(
            icon_rect.right() + gap,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(text_pos, galley, visuals.text_color());
    }

    response
}

/// Un bouton carré qui ne porte qu'une icône, pour les commandes serrées
/// contre un champ — là où un libellé prendrait la place de la saisie.
fn icon_square_button(
    ui: &mut egui::Ui,
    icon: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
) -> egui::Response {
    let side = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        icon(ui.painter(), rect.shrink(5.0), visuals.text_color());
    }
    response
}

/// « Aller à » : une flèche qui bute sur un montant, comme la fin d'une
/// course. Distincte de la flèche de Step Over, qui enjambe.
fn icon_go(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5, color);
    let y = rect.center().y;
    let right = rect.right() - 1.0;
    painter.line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(right - 2.5, y)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(right - 5.0, y - 3.0), egui::pos2(right - 2.0, y)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(right - 5.0, y + 3.0), egui::pos2(right - 2.0, y)],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(right, rect.top() + 1.0),
            egui::pos2(right, rect.bottom() - 1.0),
        ],
        stroke,
    );
}

fn icon_run(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let r = rect.shrink(1.0);
    painter.add(egui::Shape::convex_polygon(
        vec![
            r.left_top(),
            r.left_bottom(),
            egui::pos2(r.right(), r.center().y),
        ],
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
    painter.line_segment(
        [
            egui::pos2(x, rect.top()),
            egui::pos2(x, rect.bottom() - 3.0),
        ],
        stroke,
    );
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
    painter.line_segment(
        [
            egui::pos2(rect.left(), y),
            egui::pos2(rect.right() - 3.0, y),
        ],
        stroke,
    );
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

/// Dossier sur lequel ouvrir l'explorateur faute de mieux : le répertoire
/// de travail, puis le dossier personnel. Les deux peuvent échouer (un
/// répertoire courant supprimé, un environnement sans `HOME`) — dans ce
/// cas seulement, l'explorateur retombe sur son invite "Aucun dossier
/// ouvert".
fn default_explorer_root() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .filter(|path| path.is_dir())
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
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// Une entrée du catalogue de thèmes : un aperçu miniature peint avec les
/// couleurs du thème lui-même (fond, panneau, accents) plutôt qu'un nom
/// seul — c'est le thème qui se décrit, donc rien à écrire de plus quand
/// un nouveau s'ajoute.
fn theme_card(ui: &mut egui::Ui, theme: &Theme, active: bool, active_word: &str) -> egui::Response {
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
        self.draw(ui);
    }
}

impl NamsdecApp {
    /// Le contenu d'une frame, séparé de l'implémentation d'`eframe::App`
    /// pour qu'un test puisse le faire tourner : construire un
    /// `eframe::Frame` hors d'eframe n'est pas possible, et c'est bien
    /// cette fonction-ci — l'enchaînement complet, dispatch des touches
    /// compris — qu'il faut exercer.
    fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        // Avant tout le reste : le fichier qu'on vient de choisir dans la
        // boîte native doit être chargé avant que la frame ne s'affiche.
        self.poll_file_dialog(&ctx);
        self.apply_tooltip_setting(&ctx);
        self.tick_run(&ctx);
        if self.rebinding.is_none() {
            // Un rebind en cours capture les touches lui-même (voir
            // rebinding_ui) — pas de dispatch normal pendant ce temps.
            //
            // N'importe quel champ de texte ayant le focus suffit à
            // désarmer la touche leader, pas seulement l'éditeur : taper
            // "une note" dans le filtre de l'explorateur ne doit pas
            // refermer celui-ci sur le "e" qui suit l'espace.
            let text_focused = self.editor_focused || ctx.memory(|m| m.focused().is_some());
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

        let mut spans: Vec<(&'static str, Duration)> = Vec::new();
        let frame_start = Instant::now();

        timed(&mut spans, "barres", || {
            egui::Panel::top("menu_bar").show(ui, |ui| self.menu_bar_ui(ui));
            if self.show_toolbar {
                egui::Panel::top("toolbar").show(ui, |ui| self.toolbar_ui(ui));
            }
        });
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
        // Au-dessus de la barre de statut, sous l'éditeur, et sur toute
        // la largeur : la console garde ainsi les 80 colonnes qu'un
        // programme suppose quand il écrit.
        if self.show_console {
            timed(&mut spans, "console", || {
                egui::Panel::bottom("console")
                    .resizable(true)
                    .default_size(CONSOLE_HEIGHT)
                    .min_size(CONSOLE_MIN_HEIGHT)
                    .max_size(
                        (ui.available_height() * CONSOLE_MAX_FRACTION).max(CONSOLE_MIN_HEIGHT),
                    )
                    .show(ui, |ui| self.console_panel_ui(ui));
            });
        }
        timed(&mut spans, "panneau latéral", || {
            if self.show_sidebar {
                egui::Panel::right("sidebar")
                    .resizable(true)
                    .default_size(SIDEBAR_WIDTH)
                    .min_size(SIDEBAR_MIN_WIDTH)
                    .max_size((ui.available_width() * SIDEBAR_MAX_FRACTION).max(SIDEBAR_MIN_WIDTH))
                    .show(ui, |ui| self.sidebar_ui(ui));
            }
        });
        // L'éditeur recolore tout le tampon à chaque frame : c'est le
        // premier suspect dès que la source devient longue (un binaire
        // désassemblé, par exemple).
        timed(&mut spans, "éditeur", || self.editor_ui(ui));

        // Fenêtres flottantes — dont l'explorateur, qui n'occupe donc plus
        // de place dans la mise en page et ne dépend d'aucun panneau.
        // L'explorateur relit le disque, d'où sa mesure à part.
        timed(&mut spans, "explorateur", || self.explorer_window_ui(&ctx));
        timed(&mut spans, "fenêtres", || {
            self.preferences_window_ui(&ctx);
            self.about_window_ui(&ctx);
            self.command_palette_ui(&ctx);
        });

        // En dernier : tout réglage changé pendant cette frame, d'où qu'il
        // vienne (menu, préférences, palette, raccourci), est sur le
        // disque avant la frame suivante.
        timed(&mut spans, "réglages", || {
            self.persist_settings_if_changed()
        });

        report_slow_frame(frame_start.elapsed(), &spans);
    }
}

/// Un vidage mémoire, dans une zone qui défile des deux côtés : seize
/// octets et leur transcription font bien plus large que le panneau,
/// et la ligne était jusqu'ici coupée net sans moyen d'aller voir la
/// suite.
fn hexdump_ui(ui: &mut egui::Ui, id: &str, dump: &str) {
    egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.monospace(dump);
        });
}

/// La boîte « À propos » porte un identifiant fixe, indépendant de son
/// titre traduit.
fn about_window_id() -> egui::Id {
    egui::Id::new("about_window")
}

/// Le champ « Adresse » de l'onglet Mémoire porte un identifiant fixe :
/// c'est par lui qu'un test retrouve sa largeur.
fn memory_address_id() -> egui::Id {
    egui::Id::new("memory_address")
}

/// La largeur du champ « Adresse » : celle de ce qu'il peut contenir, et
/// non celle du panneau.
///
/// Un champ de saisie egui réclame par défaut toute la place disponible.
/// Pour une ligne de commande, c'est ce qu'on veut ; pour une adresse —
/// « 0x » suivi de seize chiffres au plus — ça donnait un champ long
/// comme le panneau, vide aux trois quarts, dont on ne savait plus ce
/// qu'il attendait.
fn address_field_width(ui: &egui::Ui) -> f32 {
    const MAX_CHARS: f32 = 18.0;
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let digit = ui.fonts_mut(|f| f.glyph_width(&font_id, '0'));
    digit * MAX_CHARS + 12.0
}

/// Une pastille de flag. Allumée, elle prend la couleur « bon » du thème ;
/// éteinte, elle reste un contour discret. Le 0/1 est gardé à côté du
/// nom : c'est la valeur du flag, et c'est elle qu'on apprend à lire.
///
/// Peinte à la main sur une place réservée d'avance, et non montée en
/// `Frame` : dans une rangée qui se replie, egui décide du passage à la
/// ligne d'après la taille annoncée par l'élément. Un `Frame` ne l'annonce
/// qu'une fois son contenu posé — les six pastilles restaient donc sur une
/// seule ligne, qui dépassait du panneau.
/// Les noms des registres dont la valeur a changé depuis la photographie
/// prise avant la dernière avancée.
///
/// Sans photographie — programme tout juste assemblé, jamais exécuté —
/// personne n'a changé : le bandeau reste uni plutôt que de tout
/// souligner.
fn changed_registers(
    before: Option<&namsdec_core::Registers>,
    now: &namsdec_core::Registers,
) -> Vec<&'static str> {
    let Some(before) = before else {
        return Vec::new();
    };
    REGISTERS
        .iter()
        .filter(|&&(_, reg)| before.get64(reg) != now.get64(reg))
        .map(|&(name, _)| name)
        .collect()
}

/// Combien de paires nom/valeur tiennent côte à côte dans `width`.
///
/// Une paire mesure un peu moins de 175 px (trois lettres, seize chiffres
/// hexadécimaux en chasse fixe, l'espacement de la grille) ; on n'en met
/// deux que s'il reste de la marge, plutôt que de déborder pour quelques
/// pixels — un débordement, dans un panneau, ce n'est pas du texte coupé,
/// c'est la largeur du panneau qui change.
fn register_columns(width: f32) -> usize {
    const PAIR: f32 = 175.0;
    /// Le blanc entre les deux paires, plus la marge du panneau.
    const GAP: f32 = 30.0;
    if width >= 2.0 * PAIR + GAP { 2 } else { 1 }
}

/// Les seize registres généraux, dans l'ordre où un débogueur les montre
/// — celui des numéros d'encodage pour les huit premiers (l'ordre
/// historique de x86), puis R8 à R15. RIP les suit, à part : il n'est pas
/// un registre général, et c'est lui que la gouttière suit.
const REGISTERS: [(&str, Reg64); 16] = [
    ("RAX", Reg64::Rax),
    ("RBX", Reg64::Rbx),
    ("RCX", Reg64::Rcx),
    ("RDX", Reg64::Rdx),
    ("RSI", Reg64::Rsi),
    ("RDI", Reg64::Rdi),
    ("RSP", Reg64::Rsp),
    ("RBP", Reg64::Rbp),
    ("R8", Reg64::R8),
    ("R9", Reg64::R9),
    ("R10", Reg64::R10),
    ("R11", Reg64::R11),
    ("R12", Reg64::R12),
    ("R13", Reg64::R13),
    ("R14", Reg64::R14),
    ("R15", Reg64::R15),
];

fn flag_chip(ui: &mut egui::Ui, theme: &'static Theme, name: &str, on: bool) {
    let color = if on { theme.good } else { theme.dim };
    let galley = ui.painter().layout_no_wrap(
        format!("{name} {}", u8::from(on)),
        egui::TextStyle::Monospace.resolve(ui.style()),
        color,
    );
    let (rect, _) =
        ui.allocate_exact_size(galley.size() + egui::vec2(12.0, 4.0), egui::Sense::hover());
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::from(3),
        egui::Stroke::new(1.0, color),
        egui::StrokeKind::Inside,
    );
    let pos = rect.center() - 0.5 * galley.size();
    ui.painter().galley(pos, galley, color);
}

/// La mise en page du code, telle que la gouttière **et** l'éditeur la
/// voient — c'est le fait qu'ils partagent celle-ci qui les garde alignés.
///
/// La largeur de repli est volontairement infinie : dans un éditeur de
/// code, une ligne trop longue part sur la droite (la zone défile dans les
/// deux sens) plutôt que de se replier. Une ligne repliée compterait
/// plusieurs lignes rendues pour un seul numéro, et la règle de gauche —
/// un numéro par ligne rendue — se décalait d'autant : c'est ce qui
/// arrivait dès que l'éditeur devenait étroit.
fn layout_code(ui: &egui::Ui, theme: &'static Theme, text: &str) -> std::sync::Arc<egui::Galley> {
    let job = syntax::highlight(ui, theme, text, f32::INFINITY);
    ui.fonts_mut(|f| f.layout_job(job))
}

/// Désassemble les sections exécutables, et elles seules.
///
/// Passer l'image entière au désassembleur revenait à décoder `.data`
/// comme des instructions : une chaîne de caractères y produit des
/// « instructions » qui n'existent pas, et le listing comme les modules de
/// reverse engineering héritaient de ce bruit.
fn disassemble_code(output: &namsdec_asm::AssembleOutput) -> Vec<DecodedInstruction> {
    let load_addr = output.load_addr;
    let slice = |addr: u64, size: usize| -> Option<&[u8]> {
        let start = addr.checked_sub(load_addr)? as usize;
        output.bytes.get(start..start.checked_add(size)?)
    };

    let mut code: Vec<&namsdec_asm::Section> =
        output.sections.iter().filter(|s| s.executable).collect();
    code.sort_by_key(|s| s.addr);

    if code.is_empty() {
        // Un binaire brut, sans table des sections : il n'y a rien d'autre
        // à faire que de tout prendre pour du code.
        return namsdec_disasm::disassemble(&output.bytes, load_addr);
    }

    code.iter()
        .filter_map(|s| Some(namsdec_disasm::disassemble(slice(s.addr, s.size)?, s.addr)))
        .flatten()
        .collect()
}

/// Comment le programme s'est arrêté : de lui-même par `exit`, ou parce
/// que la machine a rencontré `hlt`. Les deux se voient pareil depuis
/// `StepResult::Halted`, et ne veulent pourtant pas dire la même chose.
fn halted_status(emulator: &dyn Backend) -> Status {
    match emulator.exit_code() {
        Some(code) => Status::Exited { code },
        None => Status::Halted,
    }
}

/// L'adresse à montrer après validation : celle qui vient d'être tapée si
/// elle en est une, sinon celle qu'on montrait déjà.
///
/// La règle tient dans cette ligne, et c'est elle qui a changé : le vidage
/// retombait sur le début du programme dès que la saisie ne s'analysait
/// pas — une frappe de trop, et l'affichage sautait ailleurs sans rien
/// dire.
fn validated_address(current: u64, typed: &str) -> u64 {
    parse_addr(typed).unwrap_or(current)
}

fn parse_addr(s: &str) -> Option<u64> {
    let s = s.trim();
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
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
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        out.push_str(&format!("{addr:08x}  {:<47}  {ascii}\n", hex.join(" ")));
    }
    out
}

/// Au-delà de ce seuil, une frame n'est plus une saccade : la fenêtre a
/// cessé de répondre assez longtemps pour que l'utilisateur le voie, et
/// le compositeur finit par proposer de forcer l'application à quitter.
///
/// Un gel ne laisse aucune trace une fois passé — surtout un gel
/// intermittent, qu'on ne peut pas provoquer à la demande sous un
/// profileur. La frame lente s'écrit donc sur la sortie d'erreur avec le
/// détail de ce qui l'a occupée, pour que le prochain gel désigne son
/// coupable au lieu d'être à reproduire.
const SLOW_FRAME: Duration = Duration::from_millis(120);

/// Mesure une étape de la frame et l'ajoute au relevé.
fn timed<R>(
    spans: &mut Vec<(&'static str, Duration)>,
    name: &'static str,
    body: impl FnOnce() -> R,
) -> R {
    let start = Instant::now();
    let out = body();
    spans.push((name, start.elapsed()));
    out
}

/// N'écrit rien tant que la frame tient dans son budget : en marche
/// normale cette instrumentation est silencieuse.
fn report_slow_frame(total: Duration, spans: &[(&'static str, Duration)]) {
    if total < SLOW_FRAME {
        return;
    }
    let detail: Vec<String> = spans
        .iter()
        .filter(|(_, d)| *d >= Duration::from_millis(1))
        .map(|(name, d)| format!("{name} {}ms", d.as_millis()))
        .collect();
    eprintln!(
        "namsdec: frame lente — {}ms au total ({})",
        total.as_millis(),
        if detail.is_empty() {
            "hors des étapes mesurées".to_owned()
        } else {
            detail.join(", ")
        }
    );
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([800.0, 520.0])
            // Sous Wayland, c'est par cet identifiant que le bureau relie
            // la fenêtre à son fichier `.desktop`, et donc à son icône :
            // il doit valoir le nom de ce fichier (`namsdec.desktop`),
            // sans quoi la barre des tâches affiche une icône générique.
            .with_app_id("namsdec"),
        ..Default::default()
    };
    eframe::run_native(
        "Namsdec",
        options,
        Box::new(|cc| {
            let mut app = NamsdecApp::from_settings(Settings::load());
            app.save_settings_now();
            app.apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le programme type qu'on écrit en apprenant l'assembleur sous
    /// Linux : deux sections, un `_start`, des appels système. Il doit
    /// s'assembler, démarrer à `_start` — et non au premier octet chargé,
    /// qui appartient à `.data` — relier chaque ligne à la bonne adresse,
    /// écrire son message et se terminer sur son propre code de sortie.
    #[test]
    fn a_two_section_linux_program_prints_its_message_and_exits() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }

        let source = "\
section .data
    message db \"Je parle maintenant au processeur !\", 10
    longueur equ $ - message

section .text
    global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, message
    mov rdx, longueur
    syscall

    mov rax, 60
    xor rdi, rdi
    syscall
";
        let mut app = NamsdecApp {
            source: source.to_owned(),
            ..Default::default()
        };

        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "l'assemblage doit réussir, obtenu : {:?}",
            app.status
        );

        let line_to_addr = app
            .assemble_state
            .as_ref()
            .unwrap()
            .output
            .line_to_addr
            .clone();
        assert_ne!(
            line_to_addr[&2], line_to_addr[&8],
            "une ligne de `.data` et une ligne de `.text` ne peuvent pas partager une adresse"
        );
        assert_eq!(
            app.emulator.as_ref().unwrap().cpu().regs.rip,
            line_to_addr[&8],
            "l'exécution démarre à `_start`"
        );

        // Exécution complète : le programme parle, puis sort de lui-même.
        app.do_run();
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }

        let emulator = app.emulator.as_ref().expect("un émulateur chargé");
        assert_eq!(
            String::from_utf8_lossy(emulator.output()),
            "Je parle maintenant au processeur !\n",
            "le message doit être arrivé sur la sortie"
        );
        assert_eq!(emulator.exit_code(), Some(0));
        assert!(
            matches!(app.status, Status::Exited { code: 0 }),
            "la barre de statut doit dire que le programme s'est terminé, obtenu : {:?}",
            app.status
        );
    }

    /// Un programme allé jusqu'au bout n'a plus rien à exécuter : les
    /// commandes d'exécution deviennent inertes — au clavier comme au
    /// bouton, sans quoi le raccourci ferait ce que le bouton grisé
    /// refuse — et l'éditeur revient en mode commande, comme gdb rend la
    /// main après la fin du programme.
    #[test]
    fn a_finished_program_disables_execution_and_returns_to_command_mode() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }

        let mut app = NamsdecApp {
            source: "\
section .text
    global _start
_start:
    mov rax, 60
    mov rdi, 2
    syscall
"
            .to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        // L'utilisateur était en train d'écrire quand il a lancé.
        app.editor_mode = EditorMode::Insert;
        assert!(app.can_execute(), "avant de lancer, tout est disponible");

        app.do_run();
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }

        assert!(app.program_finished());
        assert!(
            !app.can_execute(),
            "Exécuter, Step Into et Step Over n'ont plus de sens"
        );
        assert_eq!(
            app.editor_mode,
            EditorMode::Normal,
            "la fin du programme rend la main aux commandes"
        );

        // Le raccourci doit obéir à la même règle que le bouton : RIP ne
        // bouge plus, et le statut de fin reste affiché.
        let rip = app.emulator.as_ref().unwrap().cpu().regs.rip;
        app.dispatch(Action::StepInto);
        app.dispatch(Action::StepOver);
        app.dispatch(Action::Run);
        assert_eq!(app.emulator.as_ref().unwrap().cpu().regs.rip, rip);
        assert!(
            matches!(app.status, Status::Exited { code: 2 }),
            "obtenu {:?}",
            app.status
        );

        // Réinitialiser, en revanche, doit rester possible : c'est
        // justement ce qu'on veut faire ensuite.
        app.dispatch(Action::Reset);
        assert!(app.can_execute(), "après Réinitialiser, on peut relancer");
    }

    /// Avancer d'un pas ne doit pas ouvrir la saisie.
    ///
    /// `Espace s i` et `Espace n i` se terminent par `i` — la touche qui,
    /// dans l'éditeur, fait passer en mode Insertion. Sans précaution, un
    /// pas d'exécution laissait donc l'éditeur en Insertion, et la frappe
    /// suivante s'écrivait dans le code au lieu de piloter l'exécution.
    #[test]
    fn stepping_with_a_leader_sequence_stays_in_normal_mode() {
        for sequence in [
            [egui::Key::Space, egui::Key::S, egui::Key::I],
            [egui::Key::Space, egui::Key::N, egui::Key::I],
        ] {
            let mut app = NamsdecApp::default();
            assert_eq!(app.editor_mode, EditorMode::Normal);

            let ctx = egui::Context::default();
            let events = sequence
                .iter()
                .map(|key| egui::Event::Key {
                    key: *key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .collect();
            let input = egui::RawInput {
                events,
                ..Default::default()
            };

            // Une frame complète, dans l'ordre réel : le dispatcher lit
            // les touches, puis l'éditeur relit les mêmes évènements.
            let mut action = None;
            let mut output = ctx.run_ui(input, |ui| {
                action = app.dispatcher.poll(ui.ctx(), &app.keybindings, false);
                app.editor_ui(ui);
            });
            output.textures_delta.clear();

            assert!(action.is_some(), "la séquence {sequence:?} doit agir");
            assert_eq!(
                app.editor_mode,
                EditorMode::Normal,
                "après {sequence:?}, l'éditeur doit rester en mode commande"
            );
        }
    }

    /// Le cas réel : les touches d'une séquence arrivent sur des frames
    /// séparées, une par frappe, avec relâchement — et non toutes dans le
    /// même lot d'évènements comme lorsqu'on tape très vite.
    #[test]
    fn stepping_stays_in_normal_mode_across_separate_frames() {
        for sequence in [
            [egui::Key::Space, egui::Key::S, egui::Key::I],
            [egui::Key::Space, egui::Key::N, egui::Key::I],
        ] {
            let mut app = NamsdecApp::default();
            let ctx = egui::Context::default();

            for key in sequence {
                let input = egui::RawInput {
                    events: vec![
                        egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                        egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: false,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                };
                // La frame entière, telle qu'eframe l'exécute : dispatch
                // des raccourcis, panneaux, éditeur.
                let mut output = ctx.run_ui(input, |ui| app.draw(ui));
                output.textures_delta.clear();
            }

            assert_eq!(
                app.editor_mode,
                EditorMode::Normal,
                "après {sequence:?} tapée touche par touche"
            );
        }
    }

    /// Le vrai parcours : on écrit du code (mode Insertion), on en sort
    /// par Échap, puis on lance un pas au clavier. C'est là que le
    /// raccourci doit fonctionner — et surtout ne pas rouvrir la saisie.
    #[test]
    fn stepping_works_after_leaving_insert_mode() {
        let mut app = NamsdecApp::default();
        let ctx = egui::Context::default();

        let stroke = |key| {
            vec![
                egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let frame = |app: &mut NamsdecApp, key| {
            let input = egui::RawInput {
                events: stroke(key),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.draw(ui));
            output.textures_delta.clear();
        };

        // `i` ouvre la saisie, Échap la referme.
        frame(&mut app, egui::Key::I);
        assert_eq!(app.editor_mode, EditorMode::Insert, "`i` ouvre la saisie");
        frame(&mut app, egui::Key::Escape);
        assert_eq!(app.editor_mode, EditorMode::Normal, "Échap la referme");

        // Puis la séquence de pas, touche par touche.
        for key in [egui::Key::Space, egui::Key::S, egui::Key::I] {
            frame(&mut app, key);
        }

        assert_eq!(
            app.editor_mode,
            EditorMode::Normal,
            "`Espace s i` après une édition ne doit pas rouvrir la saisie"
        );
    }

    /// Mais `i` tapé seul, lui, ouvre toujours la saisie : seule la touche
    /// qui a servi à un raccourci est retirée à l'éditeur.
    #[test]
    fn plain_i_still_enters_insert_mode() {
        let mut app = NamsdecApp::default();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::I,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };

        let mut output = ctx.run_ui(input, |ui| {
            app.dispatcher.poll(ui.ctx(), &app.keybindings, false);
            app.editor_ui(ui);
        });
        output.textures_delta.clear();

        assert_eq!(app.editor_mode, EditorMode::Insert);
    }

    /// Un programme qui lit son entrée puis la répète : il doit se
    /// suspendre pour de bon en attendant, puis repartir tout seul dès que
    /// l'utilisateur a tapé quelque chose — l'interface, elle, ne bloque
    /// jamais.
    #[test]
    fn a_program_waits_for_console_input_then_echoes_it() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }

        let mut app = NamsdecApp {
            source: "\
section .bss
    tampon resb 32

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 32
    syscall

    mov rdx, rax
    mov rax, 1
    mov rdi, 1
    mov rsi, tampon
    syscall

    mov rax, 60
    xor rdi, rdi
    syscall
"
            .to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "obtenu {:?}",
            app.status
        );

        let ctx = egui::Context::default();
        app.do_run();
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }

        assert!(
            matches!(app.status, Status::WaitingForInput),
            "le programme doit attendre son entrée, obtenu {:?}",
            app.status
        );
        assert!(!app.running, "l'exécution est suspendue, pas en cours");
        assert!(
            app.emulator.as_ref().unwrap().output().is_empty(),
            "il n'a encore rien écrit : il n'a rien lu"
        );

        // L'utilisateur tape sa réponse et valide.
        app.send_input(Some("bonjour".to_owned()));
        assert!(app.running, "un Exécuter suspendu doit repartir tout seul");

        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }

        assert_eq!(
            String::from_utf8_lossy(app.emulator.as_ref().unwrap().output()),
            "bonjour\n",
            "le programme répète ce qu'on lui a donné, saut de ligne compris"
        );
        assert!(matches!(app.status, Status::Exited { code: 0 }));
    }

    /// Fermer l'entrée débloque un programme qui lit jusqu'au bout, au
    /// lieu de le laisser attendre pour toujours.
    #[test]
    fn ending_the_input_releases_a_waiting_program() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }

        let mut app = NamsdecApp {
            source: "\
section .bss
    tampon resb 16

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 16
    syscall
    hlt
"
            .to_owned(),
            ..Default::default()
        };
        app.do_assemble();

        let ctx = egui::Context::default();
        app.do_run();
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }
        assert!(matches!(app.status, Status::WaitingForInput));

        // « Fin d'entrée » : `read` rend 0 et le programme continue.
        app.send_input(None);
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }

        assert!(
            app.program_finished(),
            "le programme doit être allé au bout"
        );
        assert_eq!(
            app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax),
            0,
            "`read` rend une fin de fichier"
        );
    }

    /// Une largeur de panneau aberrante, héritée d'une session où la
    /// mise en page s'était emballée, ne doit pas survivre : egui garde
    /// cette largeur d'un lancement à l'autre, et elle revenait telle
    /// quelle — panneau latéral sur toute la fenêtre, éditeur réduit à
    /// une colonne de quelques caractères. Les bornes la ramènent dans le
    /// raisonnable dès la première frame.
    #[test]
    fn an_absurd_stored_sidebar_width_is_brought_back() {
        let mut app = NamsdecApp::default();
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));

        // Ce que le bug avait laissé dans `app.ron` : le panneau part du
        // cent-unième pixel et va jusqu'au bord droit.
        ctx.data_mut(|d| {
            d.insert_persisted(
                egui::Id::new("sidebar"),
                egui::containers::PanelState {
                    outer_rect: egui::Rect::from_min_max(
                        egui::pos2(100.0, 0.0),
                        egui::pos2(1280.0, 820.0),
                    ),
                },
            );
        });

        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();

        let width = egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
            .expect("le panneau latéral est affiché")
            .outer_rect
            .width();
        assert!(
            width <= screen.width() * SIDEBAR_MAX_FRACTION,
            "le panneau doit laisser au moins la moitié de la fenêtre à l'éditeur, \
             obtenu : {width}"
        );
        assert!(width >= SIDEBAR_MIN_WIDTH, "obtenu : {width}");
    }

    /// Un numéro de la règle par ligne de code, quelle que soit la
    /// largeur de l'éditeur. La gouttière compte une ligne rendue pour un
    /// numéro : si le texte se repliait, une ligne source en occupait
    /// plusieurs et la numérotation se décalait de tout ce qui précède.
    #[test]
    fn the_ruler_stays_line_for_line_however_narrow_the_editor() {
        let source = format!(
            "section .text\n    mov rax, 1 ; {}\n    hlt\n",
            "un commentaire à rallonge ".repeat(20)
        );
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            // Un éditeur réduit à une colonne étroite, comme lorsque le
            // panneau latéral avait tout pris.
            ui.set_max_width(80.0);
            let galley = layout_code(ui, DEFAULT_THEME, &source);
            assert_eq!(
                galley.rows.len(),
                // `split` et non `lines` : le retour à la ligne final
                // ouvre une ligne de plus, que l'éditeur numérote aussi.
                source.split('\n').count(),
                "une ligne source doit rester une ligne rendue"
            );
        });
        output.textures_delta.clear();
    }

    /// Un pas à pas reprend la main sur un Run suspendu faute d'entrée.
    ///
    /// Le drapeau « reprendre le Run dès que l'entrée arrive » ne tombait
    /// qu'à l'appel suivant de Exécuter. Un Run bloqué sur `read`, un Stop
    /// sans effet (il ne regardait que `running`, faux pendant l'attente),
    /// puis un Step Into : la ligne tapée pour ce pas-là relançait le
    /// programme jusqu'au bout au lieu d'avancer d'une instruction.
    #[test]
    fn stepping_takes_over_a_run_that_was_waiting_for_input() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        let mut app = NamsdecApp {
            source: "\
section .bss
    tampon resb 32

section .text
    global _start
_start:
    mov rax, 0
    mov rdi, 0
    mov rsi, tampon
    mov rdx, 32
    syscall
    hlt
"
            .to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

        let ctx = egui::Context::default();
        app.do_run();
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }
        assert!(matches!(app.status, Status::WaitingForInput));
        assert!(app.resume_run_after_input, "le Run attend de repartir");

        // L'utilisateur reprend la main instruction par instruction.
        app.do_step();
        app.send_input(Some("oui".to_owned()));

        assert!(
            !app.running,
            "la saisie ne doit pas relancer le programme : c'est un pas à pas"
        );
        assert!(!app.resume_run_after_input);
    }

    /// Une saisie qui n'est pas une adresse laisse le vidage là où il
    /// est. Avant, la moindre frappe de trop — `0x40_1000`, un `g` qui
    /// traîne, un champ à moitié effacé — renvoyait l'affichage au début
    /// du programme sans un mot d'explication.
    #[test]
    fn an_unparsable_address_keeps_the_one_on_screen() {
        assert_eq!(validated_address(0x401000, "0x400500"), 0x400500);
        assert_eq!(
            validated_address(0x401000, "400500"),
            0x400500,
            "le 0x est facultatif"
        );
        assert_eq!(validated_address(0x401000, "truc"), 0x401000);
        assert_eq!(validated_address(0x401000, "0x40_1000"), 0x401000);
        assert_eq!(validated_address(0x401000, ""), 0x401000);
    }

    /// Et l'affichage d'une frame entière avec une saisie invalide ne
    /// déplace rien de lui-même : il faut valider (Entrée ou le bouton).
    #[test]
    fn typing_alone_does_not_move_the_memory_view() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        let mut app = NamsdecApp {
            active_tab: SidebarTab::Memory,
            mem_addr: 0x401000,
            mem_addr_input: "pas une adresse".to_owned(),
            ..Default::default()
        };
        app.do_assemble();

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 820.0),
            )),
            ..Default::default()
        };
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }

        assert_eq!(
            app.mem_addr, 0x401000,
            "le vidage reste sur la dernière adresse validée"
        );
    }

    /// Les infobulles s'éteignent pour toute l'application, explorateur
    /// compris : c'est le style d'egui qui porte le réglage, pas chaque
    /// appel à `on_hover_text`.
    #[test]
    fn tooltips_can_be_turned_off_and_back_on() {
        let mut app = NamsdecApp {
            show_tooltips: false,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let frame = |app: &mut NamsdecApp, ctx: &egui::Context| {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
            output.textures_delta.clear();
        };

        frame(&mut app, &ctx);
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let interaction = &ctx.style_of(theme).interaction;
            assert!(
                interaction.tooltip_delay.is_infinite(),
                "aucune bulle ne doit arriver à échéance"
            );
            assert_eq!(
                interaction.tooltip_grace_time, 0.0,
                "ni passer par le délai de grâce"
            );
        }

        app.show_tooltips = true;
        frame(&mut app, &ctx);
        assert_eq!(
            ctx.style_of(egui::Theme::Dark).interaction.tooltip_delay,
            TOOLTIP_DELAY
        );
    }

    /// Le champ « Adresse » a la largeur d'une adresse, pas celle du
    /// panneau. Un champ de saisie egui réclame par défaut toute la place
    /// disponible : sur un panneau large, celui-ci s'étirait sur des
    /// centaines de pixels pour dix-huit caractères.
    #[test]
    fn the_address_field_is_as_wide_as_an_address_not_as_the_panel() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));

        // La même vue dans un panneau étroit, puis dans un panneau deux
        // fois plus large.
        let field_width = |sidebar_width: f32| {
            let mut app = NamsdecApp {
                active_tab: SidebarTab::Memory,
                ..Default::default()
            };
            app.do_assemble();
            let ctx = egui::Context::default();
            ctx.data_mut(|d| {
                d.insert_persisted(
                    egui::Id::new("sidebar"),
                    egui::containers::PanelState {
                        outer_rect: egui::Rect::from_min_max(
                            egui::pos2(screen.right() - sidebar_width, 0.0),
                            egui::pos2(screen.right(), screen.bottom()),
                        ),
                    },
                );
            });
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            for _ in 0..2 {
                let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                output.textures_delta.clear();
            }
            let sidebar = egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
                .expect("le panneau latéral est affiché")
                .outer_rect;
            let field = ctx
                .read_response(memory_address_id())
                .expect("le champ Adresse est affiché")
                .rect;
            assert!(
                field.right() <= sidebar.right() + 1.0,
                "le champ ne doit pas dépasser le panneau ({} contre {})",
                field.right(),
                sidebar.right()
            );
            (sidebar.width(), field.width())
        };

        let (narrow_panel, narrow_field) = field_width(300.0);
        let (wide_panel, wide_field) = field_width(620.0);

        assert!(
            wide_panel > narrow_panel,
            "les deux panneaux doivent différer"
        );
        assert_eq!(
            narrow_field, wide_field,
            "le champ doit garder sa taille quand le panneau s'élargit"
        );
        assert!(
            wide_field < wide_panel * 0.6,
            "et rester un champ d'adresse, pas une barre : {wide_field} px"
        );
    }

    /// La console vit maintenant dans le panneau bas, sur toute la
    /// largeur de la fenêtre : c'est ce qui lui rend les 80 colonnes
    /// qu'un programme suppose quand il écrit, là où le panneau latéral
    /// n'en offrait que trente-quatre. Sa saisie reste collée en bas,
    /// même quand le programme a beaucoup écrit.
    #[test]
    fn the_console_panel_spans_the_window_and_keeps_its_input_at_the_bottom() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        let mut app = NamsdecApp {
            source: "section .data\n    msg db \"beaucoup de sortie\", 10\n\
                     \x20   len equ $ - msg\nsection .text\n    global _start\n_start:\n\
                     \x20   mov rcx, 40\nboucle:\n    push rcx\n    mov rax, 1\n\
                     \x20   mov rdi, 1\n    mov rsi, msg\n    mov rdx, len\n    syscall\n\
                     \x20   pop rcx\n    dec rcx\n    jnz boucle\n    hlt\n"
                .to_owned(),
            console_tab: ConsoleTab::Output,
            ..Default::default()
        };
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "obtenu : {:?}",
            app.status
        );
        app.do_run();

        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        // Deux frames : la première pose les panneaux, la seconde les
        // montre à leur vraie place.
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }

        assert!(
            !app.emulator.as_ref().unwrap().output().is_empty(),
            "le programme doit avoir écrit quelque chose"
        );
        let console = egui::containers::PanelState::load(&ctx, egui::Id::new("console"))
            .expect("le panneau de la console est affiché")
            .outer_rect;
        let console_input =
            egui::containers::PanelState::load(&ctx, egui::Id::new("console_input"))
                .expect("la saisie de la console est affichée")
                .outer_rect;

        assert_eq!(
            console.width(),
            screen.width(),
            "la console prend toute la largeur, panneau latéral compris"
        );
        assert!(
            console.height() <= screen.height() * CONSOLE_MAX_FRACTION,
            "sans jamais dévorer l'éditeur, obtenu : {}",
            console.height()
        );
        // À la marge intérieure du panneau près.
        assert!(
            (console.bottom() - console_input.bottom()).abs() <= 8.0,
            "la saisie doit toucher le bas du panneau ({} contre {})",
            console_input.bottom(),
            console.bottom()
        );
    }

    /// Un assemblage qui échoue amène ses diagnostics sous les yeux :
    /// le panneau bas s'ouvre sur l'onglet qui les porte. La barre de
    /// statut dit « Erreur d'assemblage » — encore faut-il pouvoir lire
    /// laquelle. Et quand l'erreur est corrigée, la place revient à la
    /// console, d'où repart l'exécution.
    #[test]
    fn a_failed_assembly_brings_up_its_diagnostics() {
        if namsdec_asm::find_nasm().is_none() {
            return;
        }
        let mut app = NamsdecApp {
            source: "section .text\n    mov rax, pasunregistre\n".to_owned(),
            show_console: false,
            ..Default::default()
        };

        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleFailed),
            "{:?}",
            app.status
        );
        assert!(app.show_console, "le panneau bas doit s'ouvrir");
        assert_eq!(app.console_tab, ConsoleTab::Diagnostics);

        app.source = "section .text\n    hlt\n".to_owned();
        app.do_assemble();
        assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
        assert_eq!(
            app.console_tab,
            ConsoleTab::Output,
            "plus de diagnostic : la console reprend la place"
        );
    }

    /// Les diagnostics de nasm ont quitté la barre de statut pour l'onglet
    /// Diagnostics : la barre ne doit plus grandir quand l'assemblage
    /// échoue. Elle s'étirait jusqu'à cinq lignes, en bas de la fenêtre,
    /// pour une liste qu'on n'y lisait pas.
    #[test]
    fn diagnostics_no_longer_swell_the_status_bar() {
        if namsdec_asm::find_nasm().is_none() {
            return;
        }
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
        let height_of_status_bar = |source: &str| {
            let mut app = NamsdecApp {
                source: source.to_owned(),
                ..Default::default()
            };
            app.do_assemble();
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            for _ in 0..2 {
                let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                output.textures_delta.clear();
            }
            let bar = egui::containers::PanelState::load(&ctx, egui::Id::new("statusbar"))
                .expect("la barre de statut est affichée")
                .outer_rect
                .height();
            (app.diagnostics.len(), bar)
        };

        let (clean, clean_height) = height_of_status_bar("section .text\n    hlt\n");
        let (broken, broken_height) = height_of_status_bar(
            "section .text\n    mov rax, pasunregistre\n    mov rbx, pasnonplus\n",
        );

        assert_eq!(clean, 0);
        assert!(broken >= 2, "obtenu {broken} diagnostic(s)");
        assert_eq!(
            clean_height, broken_height,
            "la barre de statut doit garder sa hauteur, diagnostics ou pas"
        );
    }

    /// Le bandeau montre les seize registres généraux et RIP : un
    /// programme qui travaille dans R8 ou R12 doit pouvoir être suivi
    /// comme un autre.
    #[test]
    fn the_cpu_banner_shows_every_general_register() {
        use std::collections::BTreeSet;
        let names: BTreeSet<&str> = REGISTERS.iter().map(|&(name, _)| name).collect();
        assert_eq!(names.len(), 16, "seize registres, sans doublon");
        for expected in [
            "RAX", "RBX", "RCX", "RDX", "RSI", "RDI", "RSP", "RBP", "R8", "R9", "R10", "R11",
            "R12", "R13", "R14", "R15",
        ] {
            assert!(names.contains(expected), "{expected} manque au bandeau");
        }

        // Et chaque nom désigne bien son registre : une valeur écrite
        // dans l'un doit ressortir sous le bon nom.
        let mut regs = namsdec_core::Registers::default();
        for (index, &(_, reg)) in REGISTERS.iter().enumerate() {
            regs.set64(reg, 0x1000 + index as u64);
        }
        for (index, &(name, reg)) in REGISTERS.iter().enumerate() {
            assert_eq!(
                regs.get64(reg),
                0x1000 + index as u64,
                "{name} ne lit pas son propre registre"
            );
        }
    }

    /// Le bandeau sur deux colonnes doit tenir dans le panneau élargi
    /// sans le pousser — et sans casser : la cellule qui sépare les deux
    /// colonnes est une cellule *allouée*, pas un `add_space`, qu'egui
    /// refuse dans une grille.
    #[test]
    fn the_two_column_banner_fits_the_panel_it_is_given() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
        let wide = 620.0;
        let mut app = NamsdecApp::default();
        app.do_assemble();
        assert!(app.emulator.is_some(), "il faut un processeur à montrer");

        let ctx = egui::Context::default();
        ctx.data_mut(|d| {
            d.insert_persisted(
                egui::Id::new("sidebar"),
                egui::containers::PanelState {
                    outer_rect: egui::Rect::from_min_max(
                        egui::pos2(screen.right() - wide, 0.0),
                        egui::pos2(screen.right(), screen.bottom()),
                    ),
                },
            );
        });
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut widths = Vec::new();
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
            widths.push(
                egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
                    .expect("le panneau latéral est affiché")
                    .outer_rect
                    .width(),
            );
        }
        assert!(
            widths.iter().all(|w| *w == wide),
            "le panneau doit garder ses {wide} px, obtenu : {widths:?}"
        );
        assert_eq!(register_columns(wide), 2, "à cette largeur, deux colonnes");
    }

    /// La grille des registres passe à deux colonnes quand le panneau
    /// s'élargit, et jamais avant : une paire de trop, et c'est le
    /// panneau lui-même qui s'élargirait pour la contenir.
    #[test]
    fn the_register_grid_uses_two_columns_only_when_they_fit() {
        assert_eq!(register_columns(SIDEBAR_WIDTH), 1, "300 px : une colonne");
        assert_eq!(register_columns(SIDEBAR_MIN_WIDTH), 1);
        assert_eq!(register_columns(379.0), 1, "trop juste : une seule");
        assert_eq!(register_columns(380.0), 2, "la deuxième tient pile");
        assert_eq!(register_columns(420.0), 2);
        assert_eq!(register_columns(760.0), 2, "un panneau large en tient deux");
    }

    /// La palette ne propose pas la vue RE tant qu'elle est masquée.
    ///
    /// Elle la proposait toujours, et s'y rendre *allumait* le réglage —
    /// enregistré dans la foulée. Un aller-retour dans la palette
    /// suffisait donc à faire réapparaître l'onglet pour de bon, sans
    /// être passé par Préférences.
    #[test]
    fn the_palette_offers_the_re_view_only_when_it_is_visible() {
        let s = strings(Lang::Fr);
        let labels = |show_re_tab| -> Vec<String> {
            palette_commands(s, show_re_tab)
                .into_iter()
                .map(|(label, _)| label)
                .collect()
        };
        let entry = format!("{}: {}", s.menu_view, s.tab_re);

        assert!(
            !labels(false).contains(&entry),
            "onglet masqué : la commande n'a pas à être offerte"
        );
        assert!(labels(true).contains(&entry), "onglet visible : elle l'est");

        // Et s'y rendre ne touche plus au réglage.
        let mut app = NamsdecApp {
            show_re_tab: false,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.execute_palette_command(&ctx, PaletteCommand::GoTab(SidebarTab::Re));
        assert!(
            !app.show_re_tab,
            "aller quelque part ne doit pas activer une vue désactivée"
        );
    }

    /// La boîte « À propos » s'ouvre au centre, porte le lien de la
    /// licence, le bouton de don et celui de mise à jour — lequel dit
    /// franchement que rien n'est branché derrière.
    #[test]
    fn the_about_box_is_centred_and_carries_its_three_offers() {
        let mut app = NamsdecApp {
            show_about: true,
            ..Default::default()
        };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }

        let window = egui::AreaState::load(&ctx, about_window_id()).expect("la boîte est affichée");
        let rect = window.rect();
        let offset = rect.center() - screen.center();
        assert!(
            offset.length() < 1.0,
            "la boîte doit être centrée, écart : {offset:?}"
        );

        // Le lien de licence pointe sur le texte de la MIT, et celui du
        // don sur une adresse https (fictive pour l'instant).
        assert!(MIT_LICENSE_URL.starts_with("https://"));
        assert!(
            DONATION_URL.is_empty() || DONATION_URL.starts_with("https://"),
            "un lien de don renseigné doit être une adresse https"
        );

        // L'illustration embarquée fait bien la taille annoncée : quatre
        // octets par pixel, sur un carré de `TREAT_IMAGE_SIDE`. Un
        // fichier régénéré à une autre dimension ferait paniquer
        // `ColorImage::from_rgba_unmultiplied` à l'ouverture de la boîte.
        assert_eq!(
            TREAT_IMAGE.len(),
            TREAT_IMAGE_SIDE * TREAT_IMAGE_SIDE * 4,
            "assets/offrir-un-repas.rgba ne correspond plus à TREAT_IMAGE_SIDE"
        );

        // Le bouton de mise à jour ne prétend rien : il lève seulement la
        // note « pas encore branché », qui ne survit pas à la fermeture.
        app.about_update_clicked = true;
        app.show_about = false;
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
        assert!(
            !app.about_update_clicked,
            "la note se referme avec la boîte"
        );
    }

    /// Le bandeau met en avant les registres que la dernière avancée a
    /// changés — c'est ce qui fait *voir* ce qu'une instruction fait,
    /// sans comparer seize valeurs de tête à chaque pas.
    #[test]
    fn the_banner_points_out_what_the_last_step_changed() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        let mut app = NamsdecApp {
            source: "\
section .text
    global _start
_start:
    mov rax, 7
    mov rbx, 9
    add rax, rbx
    hlt
"
            .to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

        let changed = |app: &NamsdecApp| {
            changed_registers(
                app.regs_before.as_ref(),
                &app.emulator.as_ref().unwrap().cpu().regs,
            )
        };

        // Rien n'a encore bougé : le bandeau reste uni.
        assert!(
            changed(&app).is_empty(),
            "un programme jamais exécuté n'a rien changé"
        );

        app.do_step(); // mov rax, 7
        assert_eq!(changed(&app), ["RAX"]);

        app.do_step(); // mov rbx, 9
        assert_eq!(changed(&app), ["RBX"], "seul le dernier pas compte");

        app.do_step(); // add rax, rbx
        assert_eq!(changed(&app), ["RAX"]);

        // Et une exécution complète montre tout ce qu'elle a changé,
        // depuis son départ.
        app.do_assemble();
        let ctx = egui::Context::default();
        app.do_run();
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }
        assert_eq!(changed(&app), ["RAX", "RBX"]);
    }

    /// Le bandeau d'état s'ouvre déployé à chaque lancement.
    ///
    /// egui retient l'état des en-têtes repliables d'une session à
    /// l'autre : replié une fois, il revenait replié, registres cachés,
    /// sans que rien ne dise pourquoi. La mesure porte sur ce qui suit le
    /// bandeau — le champ Adresse de l'onglet Mémoire descend de toute la
    /// hauteur des registres quand ils sont là.
    #[test]
    fn the_cpu_banner_opens_deployed() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        assert!(
            NamsdecApp::default().cpu_state_open,
            "déployé par défaut, sans rien avoir à régler"
        );

        let top_of_address_field = |open: bool| {
            let mut app = NamsdecApp {
                active_tab: SidebarTab::Memory,
                cpu_state_open: open,
                ..Default::default()
            };
            app.do_assemble();
            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 820.0),
                )),
                ..Default::default()
            };
            for _ in 0..2 {
                let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                output.textures_delta.clear();
            }
            ctx.read_response(memory_address_id())
                .expect("le champ Adresse est affiché")
                .rect
                .top()
        };

        assert!(
            top_of_address_field(true) > top_of_address_field(false) + 100.0,
            "déployé, le bandeau doit occuper la place des registres"
        );
    }

    /// L'onglet RE n'apparaît dans la barre latérale que si on l'a
    /// demandé, **et seulement pour la session en cours**.
    ///
    /// Il ouvre un module de reverse engineering encore à l'état
    /// d'ébauche. Tant que le « oui » était enregistré dans
    /// `settings.json`, il suffisait de l'avoir donné une fois — ou de
    /// l'avoir donné sans le vouloir, ce que la commande « Affichage :
    /// RE » de la palette faisait au passage — pour le retrouver à tous
    /// les lancements suivants, sans savoir d'où il venait.
    #[test]
    fn the_re_tab_is_absent_at_every_launch_until_it_is_asked_for() {
        assert!(!NamsdecApp::default().show_re_tab, "application par défaut");

        // Un fichier de réglages n'a plus voix au chapitre, même celui
        // d'une version qui écrivait encore ce champ.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"version": 2, "show_re_tab": true, "language": "fr"}"#,
        )
        .unwrap();
        let settings = Settings::load_from(&path);
        let app = NamsdecApp::from_settings(settings);
        assert!(
            !app.show_re_tab,
            "un ancien fichier ne doit plus pouvoir rallumer l'onglet"
        );

        // Et la rangée d'onglets ne le propose pas.
        let shown: Vec<&str> = SidebarTab::ALL
            .iter()
            .filter(|tab| app.show_re_tab || **tab != SidebarTab::Re)
            .map(|tab| tab.label(strings(Lang::Fr)))
            .collect();
        assert_eq!(shown, ["Pile", "Mémoire"]);

        // Ce qu'on coche dans Préférences vaut pour la session, et ne
        // part pas dans les réglages enregistrés.
        let app = NamsdecApp {
            show_re_tab: true,
            ..Default::default()
        };
        let written = serde_json::to_string(&app.settings_snapshot()).unwrap();
        assert!(
            !written.contains("show_re_tab"),
            "le champ n'a plus à être écrit : {written}"
        );
    }

    /// Le panneau latéral garde la largeur qu'on lui a donnée, quel que
    /// soit l'onglet ouvert et ce qu'il contient.
    ///
    /// La ligne de saisie de la Console devinait sa largeur (« la place
    /// disponible moins 120 px ») et débordait de quelques pixels. Or un
    /// panneau egui retient comme largeur celle de son contenu : le
    /// débordement d'une frame devenait la largeur de la suivante, qui
    /// débordait à son tour. Ouvrir l'onglet Console élargissait donc le
    /// panneau de 28 px par frame — soixante fois par seconde — jusqu'à
    /// ce qu'il occupe la fenêtre, éditeur compris. Et comme egui
    /// enregistre cette largeur, elle revenait telle quelle au lancement
    /// suivant.
    #[test]
    fn no_sidebar_tab_widens_the_panel() {
        if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
            return;
        }
        // De quoi remplir chaque onglet : un programme qui tourne (donc
        // des registres, une pile, une sortie), une longue ligne de
        // sortie, et une longue ligne tapée dans la console.
        let source = format!(
            "section .data\n    msg db \"{}\", 10\n    len equ $ - msg\n\
             section .text\n    global _start\n_start:\n\
             \x20   mov rax, 1\n    mov rdi, 1\n    mov rsi, msg\n    mov rdx, len\n\
             \x20   syscall\n    hlt\n",
            "texte bien trop long pour la largeur du panneau ".repeat(6)
        );

        for tab in SidebarTab::ALL {
            let mut app = NamsdecApp {
                source: source.clone(),
                // Même l'onglet RE, masqué par défaut, ne doit pas
                // pousser la rangée d'onglets au-delà du panneau.
                show_re_tab: true,
                console_input: "x".repeat(300),
                active_tab: tab,
                ..Default::default()
            };
            app.do_assemble();
            app.do_run();

            let ctx = egui::Context::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 820.0),
                )),
                ..Default::default()
            };
            let mut widths = Vec::new();
            for _ in 0..4 {
                let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                output.textures_delta.clear();
                widths.push(
                    egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
                        .expect("le panneau latéral est affiché")
                        .outer_rect
                        .width(),
                );
            }

            let name = tab.label(strings(Lang::Fr));
            assert!(
                widths.iter().all(|w| *w == SIDEBAR_WIDTH),
                "l'onglet {name} doit laisser le panneau à {SIDEBAR_WIDTH} px, obtenu : {widths:?}"
            );
        }
    }

    /// Le sélecteur natif de `rfd` est bloquant : appelé depuis la frame,
    /// il arrêtait la boucle d'évènements tout le temps que l'utilisateur
    /// passait à chercher son fichier, et le compositeur finissait par
    /// proposer de « forcer à quitter » une application qui attendait
    /// simplement. Il tourne maintenant sur un thread : tant qu'il n'a pas
    /// répondu, les frames continuent de s'enchaîner, et l'application
    /// redemande elle-même à être réveillée — aucun évènement n'arrive
    /// plus sur la fenêtre pendant qu'une boîte modale est ouverte.
    #[test]
    fn a_frame_runs_normally_while_a_dialog_is_open() {
        let (sender, result) = mpsc::channel();
        let mut app = NamsdecApp {
            pending_dialog: Some(PendingDialog {
                kind: FileDialogKind::OpenFile,
                result,
            }),
            ..Default::default()
        };

        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
        output.textures_delta.clear();

        assert!(
            app.pending_dialog.is_some(),
            "la boîte est toujours ouverte, on l'attend encore"
        );
        let repaint_delay = output
            .viewport_output
            .values()
            .map(|v| v.repaint_delay)
            .min()
            .expect("une frame produit toujours une sortie de viewport");
        assert!(
            repaint_delay <= DIALOG_POLL_INTERVAL,
            "la frame suivante doit venir d'elle-même, obtenu : {repaint_delay:?}"
        );

        // Et le chemin choisi est repris à la frame suivante, sans qu'une
        // touche ou un mouvement de souris ait à la provoquer.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("choisi.asm");
        std::fs::write(&path, "section .text\n    hlt\n").unwrap();
        sender.send(Some(path.clone())).unwrap();

        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
        output.textures_delta.clear();

        assert!(app.pending_dialog.is_none(), "la boîte est refermée");
        assert_eq!(app.current_file.as_deref(), Some(path.as_path()));
        assert!(app.source.contains("hlt"), "obtenu : {:?}", app.source);
    }

    /// Une boîte refermée sans rien choisir ne doit toucher à rien : ni au
    /// tampon, ni au fichier courant.
    #[test]
    fn a_cancelled_dialog_changes_nothing() {
        let (sender, result) = mpsc::channel();
        let mut app = NamsdecApp {
            pending_dialog: Some(PendingDialog {
                kind: FileDialogKind::OpenFile,
                result,
            }),
            ..Default::default()
        };
        let source = app.source.clone();
        sender.send(None).unwrap();

        app.poll_file_dialog(&egui::Context::default());

        assert!(app.pending_dialog.is_none());
        assert_eq!(app.current_file, None);
        assert_eq!(app.source, source);
    }

    /// Le dossier choisi ouvre l'explorateur dessus, comme le faisait
    /// l'appel bloquant d'avant.
    #[test]
    fn a_picked_folder_opens_the_explorer_on_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = NamsdecApp::default();

        app.finish_file_dialog(FileDialogKind::OpenFolder, dir.path().to_path_buf());

        assert_eq!(app.explorer.root.as_deref(), Some(dir.path()));
        assert!(app.show_explorer, "l'explorateur s'ouvre sur ce dossier");
        assert!(app.explorer_focused, "et prend le clavier");
    }

    /// Le fichier sur lequel on travaillait se rouvre au lancement
    /// suivant. C'est bien le **chemin** qui est enregistré : le contenu
    /// relu est celui du disque, y compris s'il a changé entre-temps.
    #[test]
    fn the_last_file_is_reopened_at_startup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("travail.asm");
        std::fs::write(&path, "section .text\n    hlt\n").unwrap();

        let settings = Settings {
            current_file: Some(path.display().to_string()),
            ..Default::default()
        };
        let app = NamsdecApp::from_settings(settings);

        assert_eq!(app.current_file.as_deref(), Some(path.as_path()));
        assert!(app.source.contains("hlt"), "obtenu : {:?}", app.source);

        // Modifié à l'extérieur : c'est la version du disque qui revient,
        // pas une copie gardée dans les réglages.
        std::fs::write(&path, "section .text\n    nop\n").unwrap();
        let app = NamsdecApp::from_settings(Settings {
            current_file: Some(path.display().to_string()),
            ..Default::default()
        });
        assert!(app.source.contains("nop"));
    }

    /// Un fichier déplacé ou supprimé entre deux lancements ne doit ni
    /// empêcher de démarrer, ni accueillir l'utilisateur par une erreur.
    #[test]
    fn a_vanished_file_falls_back_to_the_example_program() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            current_file: Some(dir.path().join("jamais-ecrit.asm").display().to_string()),
            ..Default::default()
        };

        let app = NamsdecApp::from_settings(settings);

        assert_eq!(app.current_file, None, "le chemin périmé est oublié");
        assert_eq!(app.source, DEFAULT_SOURCE);
        assert!(
            !matches!(app.status, Status::Error(_)),
            "démarrer ne doit pas afficher d'erreur, obtenu {:?}",
            app.status
        );
    }

    /// Le fichier ouvert part bien dans les réglages — et son contenu,
    /// lui, n'y va jamais.
    #[test]
    fn settings_record_the_path_and_never_the_source() {
        let mut app = NamsdecApp {
            source: "section .text\n    mov rax, 1234\n".to_owned(),
            current_file: Some(PathBuf::from("/tmp/projet/travail.asm")),
            ..Default::default()
        };
        let snapshot = app.settings_snapshot();

        assert_eq!(
            snapshot.current_file.as_deref(),
            Some("/tmp/projet/travail.asm")
        );
        let written = serde_json::to_string(&snapshot).unwrap();
        assert!(
            !written.contains("1234"),
            "le programme ne doit pas se retrouver dans les réglages : {written}"
        );

        // Un tampon sans fichier (binaire désassemblé, exemple de départ)
        // n'enregistre aucun chemin.
        app.current_file = None;
        assert_eq!(app.settings_snapshot().current_file, None);
    }

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
            "BITS 64\nsection .text\n    mov rax, 7\n    hlt\n",
            BASE_ADDR,
        )
        .expect("le programme de test doit s'assembler");

        let mut app = NamsdecApp::default();
        app.load_binary(output.bytes);

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
            app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax),
            7,
            "le binaire chargé doit s'exécuter pas à pas comme un programme assemblé"
        );
    }

    /// L'explorateur ne doit jamais s'ouvrir sur "Aucun dossier ouvert" :
    /// il y a toujours le répertoire de travail à montrer, et une boîte
    /// de dialogue de plus avant de voir l'arborescence n'a pas lieu
    /// d'être.
    #[test]
    fn the_explorer_always_opens_on_a_folder() {
        let app = NamsdecApp::from_settings(Settings::default());
        assert!(
            app.explorer.root.is_some(),
            "au démarrage, sans réglage enregistré"
        );

        // Et si la racine disparaît en cours de route, la rouvrir en
        // retrouve une plutôt que d'afficher l'invite vide.
        let mut app = NamsdecApp::default();
        app.explorer.root = None;
        app.dispatch(Action::ToggleExplorer);
        assert!(app.show_explorer);
        assert!(
            app.explorer.root.is_some(),
            "ouvrir l'explorateur doit lui donner un dossier"
        );
    }

    /// Un dossier enregistré qui n'existe plus (projet déplacé, clé USB
    /// retirée) ne doit pas laisser l'explorateur sur une racine morte.
    #[test]
    fn a_stale_saved_folder_falls_back_to_the_working_directory() {
        let settings = Settings {
            explorer_root: Some("/ce/chemin/n/existe/pas".to_owned()),
            ..Default::default()
        };
        let app = NamsdecApp::from_settings(settings);
        let root = app.explorer.root.expect("une racine de repli");
        assert!(root.is_dir(), "la racine de repli doit exister : {root:?}");
    }

    #[test]
    fn cursor_position_counts_characters_not_bytes() {
        let text = "mov rax, 0\nsum_loop:\n    add rax, rcx";

        assert_eq!(line_col(text, 0), (1, 1), "début du texte");
        assert_eq!(line_col(text, 4), (1, 5), "milieu de la première ligne");
        // Index 11 = juste après le '\n' : début de la ligne 2.
        assert_eq!(line_col(text, 11), (2, 1), "après un saut de ligne");
        assert_eq!(
            line_col(text, text.chars().count()),
            (3, 17),
            "fin du texte"
        );

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
