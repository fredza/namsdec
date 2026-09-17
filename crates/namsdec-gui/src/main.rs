//! Coquille eframe/egui du MVP d'apprentissage NASM. Assemble via
//! `namsdec-asm` (nasm externe), désassemble via `namsdec-disasm`
//! (vérification pédagogique) et exécute pas à pas via `namsdec-emu`,
//! en affichant les panneaux des modules `ReModule` enregistrés.

mod completion;
mod condition;
mod errors;
mod i18n;
mod keybindings;
mod nasm_errors;
mod readings;
mod roles;
mod settings;
mod sidebar;
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
use namsdec_re::{
    BinaryInfoModule, GadgetsModule, ImportsModule, ProtectionsModule, ReContext, ReModuleRegistry,
    StringsModule, SyscallsModule, XrefsModule,
};
use namsdec_theme::Theme;

use i18n::{Lang, Strings, strings};
use keybindings::{Action, Dispatcher, KeyBindings, Rebinding};
use settings::Settings;

const BASE_ADDR: u64 = 0x400000;
const MEM_SIZE: usize = 0x10000;
/// Le sommet de la pile : tout en haut de la fenêtre simulée, moins de
/// quoi laisser une adresse de retour lisible au-dessus.
const STACK_TOP: u64 = BASE_ADDR + MEM_SIZE as u64 - 0x100;
/// Le plafond du tas. Il monte vers la pile, qui descend vers lui : ce
/// qui les sépare est ce que la pile a le droit de consommer.
/// Combien d'octets du tas l'onglet montre d'un coup. Au-delà, c'est
/// l'onglet Mémoire qui prend le relais — avec son champ d'adresse, qui
/// sait aller n'importe où.
const HEAP_DUMP_BYTES: usize = 256;
/// Combien d'octets l'onglet Mémoire montre d'un coup — et donc combien
/// on garde d'un pas à l'autre pour savoir lesquels ont changé.
const MEMORY_WINDOW: usize = 128;
/// La place réservée, à droite de la barre de statut, aux trois
/// compartiments qui s'y rangent : la version de nasm, la position du
/// curseur, et la ligne où l'exécution est arrêtée.
const STATUS_RIGHT_ROOM: f32 = 340.0;
const HEAP_LIMIT: u64 = BASE_ADDR + MEM_SIZE as u64 - 0x2000;
// Le tas monte, la pile descend : ce qui les sépare doit rester assez
// grand pour que la seconde ne soit jamais écrasée par le premier. Une
// constante changée sans y penser ne compile plus.
const _: () = assert!(STACK_TOP - HEAP_LIMIT >= 0x1000);
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
///
/// De quoi poser les registres sur **deux** colonnes : à 300 px ils
/// tenaient sur une seule, dix-huit lignes de haut, et l'état du
/// processeur — ce qu'on regarde entre deux pas — ne se lisait qu'en le
/// faisant défiler.
const SIDEBAR_WIDTH: f32 = 380.0;
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
/// Le thème d'exemple, volontairement **hors du dépôt** : c'est un
/// plugin, il doit s'installer depuis n'importe où sur le disque.
#[cfg(test)]
const EXTERNAL_ABYSS: &str =
    "/home/fred/Projects/Rust/namsdec-plugin-abyss/abyss.namsdec-theme.json";

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
    Heap,
    /// Ce que chaque ligne a produit comme octets.
    Bytes,
    Re,
}

impl SidebarTab {
    const ALL: [SidebarTab; 5] = [
        SidebarTab::Stack,
        SidebarTab::Memory,
        SidebarTab::Heap,
        SidebarTab::Bytes,
        SidebarTab::Re,
    ];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            SidebarTab::Stack => s.tab_stack,
            SidebarTab::Memory => s.tab_memory,
            SidebarTab::Heap => s.tab_heap,
            SidebarTab::Bytes => s.tab_bytes,
            SidebarTab::Re => s.tab_re,
        }
    }
}

/// Sous quelle forme on écrit ce qu'on cherche en mémoire.
///
/// Trois, parce que ce sont les trois choses qu'on a sous les yeux : un
/// texte qu'on vient d'écrire, des octets relevés dans un vidage, une
/// valeur lue dans un registre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchKind {
    Text,
    Bytes,
    Value,
}

impl SearchKind {
    const ALL: [SearchKind; 3] = [SearchKind::Text, SearchKind::Bytes, SearchKind::Value];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            SearchKind::Text => s.search_text,
            SearchKind::Bytes => s.search_bytes,
            SearchKind::Value => s.search_value,
        }
    }

    /// Les octets à chercher, tels que la mémoire les porte.
    ///
    /// Une valeur est cherchée en petit-boutiste — l'ordre dans lequel
    /// le processeur l'a écrite — et sur ses octets utiles seulement :
    /// chercher `1` sur huit octets ne trouverait qu'un mot aligné, là
    /// où l'on veut l'octet.
    fn needle(self, saisie: &str) -> Option<Vec<u8>> {
        let saisie = saisie.trim();
        if saisie.is_empty() {
            return None;
        }
        match self {
            SearchKind::Text => Some(saisie.as_bytes().to_vec()),
            SearchKind::Bytes => {
                let propre: String = saisie
                    .chars()
                    .filter(|c| !c.is_whitespace() && *c != ',' && *c != '-')
                    .collect();
                let propre = propre.strip_prefix("0x").unwrap_or(&propre).to_owned();
                if propre.is_empty() || !propre.len().is_multiple_of(2) {
                    return None;
                }
                (0..propre.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&propre[i..i + 2], 16).ok())
                    .collect()
            }
            SearchKind::Value => {
                let value = match saisie
                    .strip_prefix("0x")
                    .or_else(|| saisie.strip_prefix("0X"))
                {
                    Some(hex) => u64::from_str_radix(hex, 16).ok()?,
                    None => saisie.parse::<u64>().ok()?,
                };
                let bytes = value.to_le_bytes();
                let utiles = bytes.iter().rposition(|&b| b != 0).map_or(1, |i| i + 1);
                Some(bytes[..utiles].to_vec())
            }
        }
    }
}

/// Comment lire les octets de l'onglet Mémoire.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MemoryFormat {
    /// Le vidage hexadécimal classique, seize octets par ligne.
    Bytes,
    /// Des mots de huit octets, en décimal à côté : la vue d'une pile.
    Words,
    /// Le texte jusqu'au premier octet nul.
    Text,
    /// Les pointeurs suivis jusqu'au bout : à cette adresse il y a cette
    /// adresse, où il y a cette chaîne. Le geste que pwndbg appelle
    /// *telescope*, et qu'on fait sinon à la main, une lecture après
    /// l'autre.
    Chain,
}

impl MemoryFormat {
    const ALL: [MemoryFormat; 4] = [
        MemoryFormat::Bytes,
        MemoryFormat::Words,
        MemoryFormat::Text,
        MemoryFormat::Chain,
    ];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            MemoryFormat::Bytes => s.memory_format_bytes,
            MemoryFormat::Words => s.memory_format_words,
            MemoryFormat::Text => s.memory_format_text,
            MemoryFormat::Chain => s.memory_format_chain,
        }
    }

    /// L'identifiant écrit dans `settings.json`.
    fn id(self) -> &'static str {
        match self {
            MemoryFormat::Bytes => "bytes",
            MemoryFormat::Words => "words",
            MemoryFormat::Text => "text",
            MemoryFormat::Chain => "chain",
        }
    }

    fn by_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|format| format.id() == id)
            .unwrap_or(MemoryFormat::Bytes)
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
    /// Une zone surveillée a changé de valeur.
    Watchpoint {
        addr: u64,
        old: u64,
        new: u64,
    },
    /// Un registre surveillé a changé : la question « qui écrit dans
    /// RAX ? » a sa réponse à l'écran.
    RegisterChanged {
        register: &'static str,
        old: u64,
        new: u64,
    },
    /// Le dernier pas vient d'être défait.
    SteppedBack,
    /// Il n'y avait plus rien à défaire.
    NothingToUndo,
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
    /// Une confirmation ordinaire : quelque chose a été fait, et il n'y
    /// a rien de plus à en dire qu'une phrase.
    Note(&'static str),
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
    /// Les conditions d'arrêt, par ligne : « arrête-toi là, mais
    /// seulement si… ». Une ligne sans entrée s'arrête toujours.
    breakpoint_conditions: BTreeMap<u32, String>,
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
    /// Les indicateurs avant le dernier pas : ce sont eux qu'on relit
    /// après un `cmp`, et savoir qu'ils *ont changé* ne suffit pas —
    /// c'est la valeur d'avant qui explique celle d'après.
    flags_before: Option<RFlags>,
    /// Le conseil qui accompagne la dernière erreur d'exécution : par où
    /// reprendre, quand l'erreur appelle un geste précis.
    error_advice: Option<&'static str>,
    /// L'instruction que le dernier pas vient d'exécuter : son adresse et
    /// son texte. `None` après un Run, un retour en arrière ou une
    /// réinitialisation — il n'y a plus « un » dernier pas à montrer.
    last_step: Option<(u64, String)>,
    /// Les octets de la fenêtre mémoire avant le dernier pas, pour
    /// mettre en avant ceux qui ont changé.
    mem_before: Option<Vec<u8>>,
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
    /// Comment l'onglet Mémoire lit les octets. Réglage utilisateur, donc
    /// enregistré dans `settings.json`.
    memory_format: MemoryFormat,
    /// L'adresse où s'arrêter pour un « exécuter jusqu'ici » : un point
    /// d'arrêt posé le temps d'une course, et retiré dès qu'elle
    /// s'achève. Il n'appartient pas à l'utilisateur, il ne doit donc pas
    /// survivre à ce qu'il a servi à faire.
    run_to: Option<u64>,
    /// La ligne où poser le curseur au prochain rendu de l'éditeur :
    /// cliquer un diagnostic doit y mener, sans quoi lire le message et
    /// retrouver l'endroit restent deux gestes.
    goto_line: Option<u32>,
    /// La recherche en cours, quand la barre est ouverte.
    find: Option<Find>,
    /// Ce qu'on cherche dans le manuel, quand la fenêtre d'aide est
    /// ouverte.
    help_query: String,
    /// L'action suspendue en attendant qu'on décide du tampon modifié.
    pending: Option<Pending>,
    /// La fermeture a été demandée et accordée : la prochaine demande
    /// passe sans question.
    quitting: bool,
    /// Le code tel qu'il est sur le disque — ou tel qu'il était au
    /// chargement. Ce qui diverge de lui n'est enregistré nulle part, et
    /// la barre de statut le dit.
    saved_source: String,
    /// L'onglet des octets doit-il ramener la ligne courante sous les
    /// yeux ? Posé à chaque pas, consommé au rendu suivant : sans cela,
    /// la vue se recentrerait à chaque frame et l'utilisateur ne pourrait
    /// plus la faire défiler lui-même.
    follow_rip: bool,
    /// Les registres surveillés, par leur nom tel qu'il s'affiche.
    /// L'exécution s'arrête quand l'un d'eux change — « qui écrit dans
    /// RAX ? » est une question qu'un point d'arrêt ne sait pas poser,
    /// puisqu'on ignore justement *où* le poser.
    watched_registers: BTreeSet<&'static str>,
    /// Les zones surveillées : adresse et longueur. Elles appartiennent
    /// au programme en cours, pas aux réglages — un point posé sur
    /// `0x401000` n'a aucun sens pour le binaire suivant.
    watchpoints: Vec<(u64, usize)>,
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
    /// Où se trouve le curseur dans la source, en octets. Relevé à chaque
    /// frame pendant que l'éditeur est dessiné : l'aide, elle, est
    /// demandée par une touche, longtemps après.
    cursor_byte: usize,
    /// Le mot sur lequel l'aide a été demandée, et sa fiche s'il y en a
    /// une. `None` tant que personne n'a appuyé sur F1.
    help: Option<(String, Option<&'static namsdec_doc::Entry>)>,
    /// La famille de fiches que montre le sommaire du manuel, faute de
    /// fiche à lire. Un choix de lecture : il ne va pas sur le disque.
    help_index_kind: namsdec_doc::Kind,
    /// Les fiches déjà lues, la plus récente en dernier : de quoi
    /// revenir sur ses pas quand un renvoi a mené trois fiches plus
    /// loin que prévu.
    help_back: Vec<String>,
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
    /// Le module des exercices : éteint tant qu'on ne le demande pas, et
    /// alors une entrée dans le menu Outils, rien de plus. Persisté,
    /// contrairement à l'onglet RE : c'est une case qu'on coche une fois,
    /// pas un affichage qu'on fait apparaître le temps d'un coup d'œil.
    exercises: bool,
    /// La fenêtre qui liste les exercices.
    show_exercises: bool,
    /// La fenêtre de l'aide sur l'instruction. Une fenêtre et non un
    /// onglet de la console : celle-ci porte ce que *le programme* dit —
    /// sa sortie, les diagnostics de nasm —, et une fiche d'instruction
    /// n'en fait pas partie.
    show_help: bool,
    /// La fenêtre d'analyse du binaire — les modules de rétro-ingénierie
    /// dans une fenêtre à eux, où ils ont la place de tout dire.
    show_analysis: bool,
    /// La fenêtre des registres étendus : SSE, AVX et la pile x87.
    show_vectors: bool,
    /// La fenêtre des points d'arrêt et de leurs conditions.
    show_breakpoints: bool,
    /// La chronologie : le chemin parcouru, pas par pas.
    show_timeline: bool,
    /// La fenêtre du motif cyclique — celle qui répond à « de combien ça
    /// déborde ».
    show_cyclic: bool,
    /// La longueur demandée, et la valeur qu'on cherche à replacer. Des
    /// saisies de travail, comme l'adresse du vidage mémoire : elles ne
    /// vont pas dans `settings.json`, où elles n'auraient aucun sens au
    /// lancement suivant.
    cyclic_len: String,
    cyclic_find: String,
    /// Ce qu'on cherche dans la mémoire, et sous quelle forme. Une
    /// saisie de travail, comme l'adresse du vidage.
    mem_search: String,
    mem_search_kind: SearchKind,
    /// Le panneau de désassemblage, et le moteur qu'il emploie.
    show_disassembly: bool,
    disassembler: String,
    /// Ce qu'on cherche dans le listing.
    disassembly_query: String,
    /// L'endroit où le listing commence, quand on a demandé à aller
    /// quelque part. `None` : tout le programme, depuis le début.
    disassembly_target: Option<u64>,
    /// Les désassembleurs décrits par des fichiers, trouvés au
    /// démarrage.
    disassembler_plugins: Vec<namsdec_disasm::plugin::Plugin>,
    /// Ce que le dernier plugin lancé a répondu, gardé tant qu'on le
    /// regarde : relancer un outil extérieur à chaque frame serait
    /// déraisonnable.
    plugin_output: Option<(String, String)>,
    /// Ne garder que les pas qui ont changé de direction ou touché la
    /// mémoire. Cinq mille pas ordinaires noient les dix qui comptent.
    timeline_notable_only: bool,
    /// Les séries chargées : celle livrée avec l'application, puis celles
    /// installées depuis le disque, dans l'ordre où on les a trouvées.
    exercise_packs: Vec<namsdec_exercises::Pack>,
    /// D'où vient chaque série installée — ce qui rend son retrait
    /// possible. La série livrée n'y figure pas : elle n'est nulle part
    /// sur le disque, et ne se retire pas.
    exercise_files: BTreeMap<String, PathBuf>,
    /// La série qu'on s'apprête à retirer, en attente de confirmation.
    pack_to_remove: Option<String>,
    /// Les séries écartées : installées, mais pas proposées.
    disabled_packs: BTreeSet<String>,
    /// L'exercice chargé en dernier — « où j'en étais » d'une session à
    /// l'autre.
    current_exercise: Option<String>,
    /// Où écrire les réglages, ou `None` pour ne les écrire nulle part.
    ///
    /// `None` est le défaut, et c'est délibéré : les réglages se
    /// sauvegardent à chaque frame, et un test qui dessine l'application
    /// écrasait donc le fichier de l'utilisateur — avec les valeurs par
    /// défaut, puisqu'un test part d'une application neuve. Seul le
    /// lancement réel donne ce chemin.
    settings_file: Option<PathBuf>,
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
    /// L'autocomplétion de l'éditeur (réglage Préférences > Général).
    completion: bool,
    /// Entrée garde l'indentation de la ligne qu'on quitte.
    auto_indent: bool,
    /// La fiche de l'instruction au survol du code. Distincte des
    /// infobulles ordinaires : celle-ci est grande, et s'ouvre en
    /// écrivant.
    hover_help: bool,
    /// Ce que l'autocomplétion propose en ce moment, recalculé à chaque
    /// frappe. Rien à enregistrer : c'est l'état d'un instant.
    suggestions: Option<completion::Suggestions>,
    /// `Alt` était-il enfoncé à la frame précédente ?
    ///
    /// egui n'émet pas d'évènement pour une touche de modification seule
    /// — elle n'apparaît que dans l'état des modificateurs. C'est donc le
    /// *passage* de relâché à enfoncé qu'on guette, et lui seul :
    /// autrement, garder Alt enfoncé accepterait une proposition à chaque
    /// frame.
    alt_was_down: bool,
    command_palette: Option<CommandPalette>,
    /// Réglages tels qu'écrits sur le disque. Comparés à l'état courant à
    /// chaque frame : dès qu'ils divergent, le fichier est réécrit. C'est
    /// ce qui garantit qu'aucun réglage ne peut être oublié — il n'y a pas
    /// un appel à "sauvegarder" par endroit où un réglage se change.
    saved_settings: Settings,
    /// Le sélecteur de fichiers natif ouvert en ce moment, s'il y en a un.
    pending_dialog: Option<PendingDialog>,
    /// Le compte rendu de la dernière installation de plugin, affiché
    /// dans Préférences > Plugins jusqu'à la suivante.
    plugin_note: Option<PluginNote>,
    /// Le fichier d'où vient chaque thème installé, par identifiant.
    ///
    /// C'est ce qui rend le retrait possible — et c'est aussi ce qui
    /// distingue un thème installé d'un thème d'origine : ces derniers
    /// n'y figurent pas, et n'ont donc pas de bouton pour les retirer.
    theme_files: BTreeMap<String, PathBuf>,
    /// Le thème dont on vient de demander le retrait, en attente de
    /// confirmation. Supprimer un fichier ne se fait pas d'un seul clic.
    theme_to_remove: Option<String>,
}

/// Ce qu'on demande au sélecteur natif, et donc ce qu'on fera du chemin
/// qu'il rend.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FileDialogKind {
    OpenFile,
    OpenFolder,
    SaveAs,
    /// Choisir un fichier de thème à installer.
    AddTheme,
    /// Choisir une série d'exercices à installer.
    AddExercises,
    /// Choisir où écrire le manuel entier, en Markdown.
    ExportManual,
    /// Choisir où écrire le rapport d'analyse du binaire chargé.
    ExportReport,
}

/// Les fiches dont un programme se sert, dans l'ordre où il les emploie.
///
/// Le premier mot d'une ligne est l'instruction, sauf s'il porte le
/// deux-points d'une étiquette. Ce qui a une fiche devient un renvoi ;
/// le reste est ignoré sans bruit.
fn sheets_used(source: &str) -> Vec<&'static str> {
    let mut vus: Vec<&'static str> = Vec::new();
    for ligne in source.lines() {
        let ligne = ligne.split(';').next().unwrap_or("").trim();
        for mot in ligne.split_whitespace() {
            let mot = mot.trim_end_matches(',');
            if mot.ends_with(':') {
                continue;
            }
            let Some(entry) = namsdec_doc::lookup(mot) else {
                continue;
            };
            // Le mot tel que le programme l'écrit, et non le premier nom
            // de la fiche : `xor` renvoie à la fiche des opérations
            // logiques, mais l'élève cherche `xor` dans son code.
            let name = entry
                .names
                .iter()
                .find(|name| name.eq_ignore_ascii_case(mot))
                .copied()
                .unwrap_or(entry.names[0]);
            // Une fiche une fois : deux noms de la même famille ne font
            // pas deux renvois.
            if vus.iter().any(|vu| namsdec_doc::lookup(vu) == Some(entry)) {
                continue;
            }
            vus.push(name);
        }
    }
    // Douze suffisent : au-delà, la liste cesse d'être un guide et
    // redevient une table des matières.
    vus.truncate(12);
    vus
}

/// Le compte rendu de la dernière installation de plugin.
struct PluginNote {
    ok: bool,
    text: String,
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
    /// La hauteur qu'a réellement prise **une ligne** de la liste, à la
    /// frame précédente.
    ///
    /// Une fenêtre auto-dimensionnée ne donne pas à son contenu la
    /// hauteur qu'il demande : la liste se retrouvait bornée à trois
    /// cents pixels et défilait au milieu d'un écran vide. On lui donne
    /// donc une hauteur explicite — le nombre de commandes affichées
    /// fois la hauteur d'une ligne, plafonné à la place disponible.
    ///
    /// Une ligne plutôt que le tout : ainsi la fenêtre suit le filtrage
    /// dans la frame même où l'on tape, au lieu d'une frame plus tard.
    row: Option<f32>,
}

/// Tout ce que la palette de commandes (Ctrl+Maj+P) peut déclencher —
/// délibérément large ("toutes les fonctions du programme"), pas limité
/// aux huit `Action` liées à un raccourci.
#[derive(Clone, Copy)]
enum PaletteCommand {
    Do(Action),
    ShowPreferences,
    ShowAbout,
    /// Ouvrir la liste des exercices — seulement quand le module est
    /// chargé, sans quoi la commande n'est pas proposée.
    ShowExercises,
    ShowAnalysis,
    ShowVectors,
    ShowBreakpoints,
    ShowTimeline,
    ShowDisassembly,
    ShowCyclic,
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
fn palette_commands(
    s: &Strings,
    show_re_tab: bool,
    exercises: bool,
) -> Vec<(String, PaletteCommand)> {
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
        (
            s.action_instruction_help.to_string(),
            PaletteCommand::Do(Action::InstructionHelp),
        ),
        (
            s.action_goto_label.to_string(),
            PaletteCommand::Do(Action::GoToLabel),
        ),
        (s.action_find.to_string(), PaletteCommand::Do(Action::Find)),
        (s.help_about.to_string(), PaletteCommand::ShowAbout),
        (s.analysis_open.to_string(), PaletteCommand::ShowAnalysis),
        (s.vectors_open.to_string(), PaletteCommand::ShowVectors),
        (
            s.breakpoints_open.to_string(),
            PaletteCommand::ShowBreakpoints,
        ),
        (s.timeline_open.to_string(), PaletteCommand::ShowTimeline),
        (
            s.disassembly_open.to_string(),
            PaletteCommand::ShowDisassembly,
        ),
        (s.cyclic_open.to_string(), PaletteCommand::ShowCyclic),
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
            format!("{}: {}", s.menu_view, s.tab_heap),
            PaletteCommand::GoTab(SidebarTab::Heap),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_bytes),
            PaletteCommand::GoTab(SidebarTab::Bytes),
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

    // Le module des exercices n'entre dans la palette que s'il est
    // chargé : proposer une commande qui n'ouvre rien n'aide personne.
    if exercises {
        commands.push((s.exercises_open.to_string(), PaletteCommand::ShowExercises));
    }

    if show_re_tab {
        commands.push((
            format!("{}: {}", s.menu_view, s.tab_re),
            PaletteCommand::GoTab(SidebarTab::Re),
        ));
    }

    // Un thème installé plus tard apparaît ici sans rien changer d'autre.
    commands.extend(namsdec_theme::all().into_iter().map(|theme| {
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
    exercises: bool,
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
/// qui fonctionne plutôt qu'un éditeur vide.
///
/// Ce n'est pas un exercice : ceux-là sont un module à part, éteint tant
/// qu'on ne le demande pas, et l'application doit s'ouvrir sur quelque
/// chose qui marche même quand il ne l'est pas.
const DEFAULT_SOURCE: &str = "\
; Bienvenue dans namsdec. Assemblez (Ctrl+Maj+B), puis avancez pas à pas.
; Un parcours de quinze exercices vous attend : Préférences > Modules.

global _start

section .text
_start:
    mov rax, 0
    mov rcx, 5
boucle:
    add rax, rcx
    dec rcx
    jnz boucle          ; RAX vaut 5+4+3+2+1 = 15 à la sortie

    hlt                 ; arrête la machine en laissant les registres lisibles
";

/// Ce qu'on s'apprête à faire, et qui effacerait le tampon.
///
/// Gardé de côté le temps de demander : un programme qu'on vient
/// d'écrire ne disparaît pas parce qu'on a cliqué sur un exercice.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Pending {
    New,
    OpenFile(PathBuf),
    Exercise(String),
    Quit,
}

/// La recherche dans le code : ce qu'on cherche, et où l'on en est.
///
/// Les occurrences sont recalculées à chaque frappe plutôt que
/// mémorisées : le tampon change sous la recherche — on corrige souvent
/// ce qu'on vient de trouver — et une liste d'offsets périmée pointerait
/// à côté.
#[derive(Default)]
struct Find {
    query: String,
    /// L'occurrence visée, parmi celles que `matches` rend.
    current: usize,
    /// La barre vient de s'ouvrir : le champ doit prendre le clavier.
    just_opened: bool,
}

/// Les positions, en octets, de toutes les occurrences de `query` dans
/// `text`, sans tenir compte de la casse.
///
/// Sans la casse : on écrit `RAX` dans un source et `rax` dans un
/// listing désassemblé, et chercher l'un doit trouver l'autre.
fn find_matches(text: &str, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let haystack = text.to_lowercase();
    let needle = query.to_lowercase();
    // `to_lowercase` peut changer la longueur d'un texte (ß → ss) : les
    // offsets ne vaudraient plus rien. Le cas ne se présente pas dans un
    // source assembleur, mais on ne le suppose pas.
    if haystack.len() != text.len() {
        return text.match_indices(query).map(|(at, _)| at).collect();
    }
    haystack.match_indices(&needle).map(|(at, _)| at).collect()
}

/// Une ligne du panneau de désassemblage : l'adresse, et ce que le
/// moteur choisi en dit.
struct Ligne {
    address: u64,
    texte: String,
    /// Les octets, tels qu'ils sont dans le fichier : c'est ce qui
    /// distingue un désassemblage d'une traduction.
    octets: String,
}

/// Le moteur de pseudo-code, celui qu'on montre en premier : lire
/// « RAX = RAX + RCX » avant « add rax, rcx » est le chemin naturel, et
/// c'est celui que namsdec propose par défaut.
const PSEUDO_ID: &str = "pseudo";
/// Le désassembleur natif, en syntaxe NASM — celle qu'on écrit.
const NASM_ID: &str = "nasm";

/// Le programme que « Nouveau » pose dans l'éditeur.
///
/// Pas une page blanche : devant un assembleur, elle est un mur. Il faut
/// savoir qu'il faut `global _start`, une section de code et une
/// étiquette de départ — trois choses qu'on n'invente pas. Le squelette
/// les donne, et se termine par une sortie propre : un programme qui ne
/// sort pas laisse le processeur exécuter ce qui suit.
const NEW_PROGRAM: &str = "\
global _start

section .data
    ; vos données ici — db, dw, dd, dq

section .text
_start:
    ; votre programme ici

    mov rax, 60             ; exit
    xor edi, edi            ; code de retour 0
    syscall
";

impl Default for NamsdecApp {
    fn default() -> Self {
        let mut re_registry = ReModuleRegistry::new();
        // Le résumé d'abord : de quoi on parle. Puis les protections —
        // avant de lire ce que fait un binaire, on regarde comment il
        // est armé, car c'est ce qui décide de tout ce qu'on tentera
        // ensuite.
        re_registry.register(Box::new(BinaryInfoModule::default()));
        re_registry.register(Box::new(ProtectionsModule::default()));
        // Puis dans l'ordre où l'on regarde un binaire inconnu : ce qu'il
        // dit (ses chaînes), ce qu'il demande au système, puis comment il
        // est découpé.
        re_registry.register(Box::new(StringsModule::default()));
        re_registry.register(Box::new(SyscallsModule::default()));
        // Ce qu'il emprunte : les fonctions d'autrui qu'il appelle. On
        // les lit juste après les appels système, parce que c'est la
        // même question posée un cran plus haut — que demande-t-il à ce
        // qui n'est pas lui ?
        re_registry.register(Box::new(ImportsModule::default()));
        // Les références croisées en dernier : elles se lisent une fois
        // qu'on sait ce que le binaire contient, pour décider par où
        // entrer dedans.
        re_registry.register(Box::new(XrefsModule::default()));
        // Les fragments en dernier : ils n'ont de sens qu'après avoir lu
        // les protections, dont ils expliquent la raison d'être.
        re_registry.register(Box::new(GadgetsModule::default()));

        Self {
            source: DEFAULT_SOURCE.to_string(),
            theme: DEFAULT_THEME,
            lang: Lang::Fr,
            nasm_info: namsdec_asm::find_nasm(),
            breakpoint_lines: BTreeSet::new(),
            breakpoint_conditions: BTreeMap::new(),
            assemble_state: None,
            emulator: None,
            regs_before: None,
            flags_before: None,
            error_advice: None,
            last_step: None,
            mem_before: None,
            re_registry,
            status: Status::Idle,
            diagnostics: Vec::new(),
            active_tab: SidebarTab::Stack,
            console_tab: ConsoleTab::Output,
            mem_addr_input: format!("{BASE_ADDR:#x}"),
            mem_addr: BASE_ADDR,
            memory_format: MemoryFormat::Bytes,
            watchpoints: Vec::new(),
            run_to: None,
            goto_line: None,
            find: None,
            help_query: String::new(),
            pending: None,
            quitting: false,
            saved_source: DEFAULT_SOURCE.to_string(),
            follow_rip: false,
            watched_registers: BTreeSet::new(),
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
            cursor_byte: 0,
            help: None,
            help_index_kind: namsdec_doc::Kind::Concept,
            help_back: Vec::new(),
            preferences_tab: PreferencesTab::General,
            show_re_tab: false,
            exercises: false,
            show_exercises: false,
            show_help: false,
            show_analysis: false,
            show_vectors: false,
            show_breakpoints: false,
            show_timeline: false,
            show_cyclic: false,
            // Cent, comme pwndbg : de quoi couvrir un tampon
            // d'apprentissage sans noyer la fenêtre.
            cyclic_len: "100".to_owned(),
            cyclic_find: String::new(),
            mem_search: String::new(),
            mem_search_kind: SearchKind::Text,
            show_disassembly: false,
            disassembler: PSEUDO_ID.to_owned(),
            disassembly_query: String::new(),
            disassembly_target: None,
            disassembler_plugins: load_disassembler_plugins(),
            plugin_output: None,
            timeline_notable_only: false,
            exercise_packs: vec![namsdec_exercises::builtin_pack()],
            exercise_files: BTreeMap::new(),
            pack_to_remove: None,
            disabled_packs: BTreeSet::new(),
            current_exercise: None,
            settings_file: None,
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
            completion: true,
            auto_indent: true,
            hover_help: true,
            suggestions: None,
            alt_was_down: false,
            command_palette: None,
            saved_settings: Settings::default(),
            pending_dialog: None,
            plugin_note: None,
            theme_files: BTreeMap::new(),
            theme_to_remove: None,
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
            completion: settings.completion,
            auto_indent: settings.auto_indent,
            hover_help: settings.hover_help,
            exercises: settings.exercises,
            disabled_packs: settings.disabled_packs.iter().cloned().collect(),
            current_exercise: settings.current_exercise.clone(),
            memory_format: MemoryFormat::by_id(&settings.memory_format),
            disassembler: settings.disassembler.clone(),
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
        let Some(path) = self.settings_file.clone() else {
            return;
        };
        if let Err(e) = self.saved_settings.save_to(&path) {
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
            completion: self.completion,
            auto_indent: self.auto_indent,
            hover_help: self.hover_help,
            exercises: self.exercises,
            disabled_packs: self.disabled_packs.iter().cloned().collect(),
            current_exercise: self.current_exercise.clone(),
            disassembler: self.disassembler.clone(),
            memory_format: self.memory_format.id().to_owned(),
            explorer_root: self.explorer.root.as_ref().map(|p| p.display().to_string()),
            current_file: self.current_file.as_ref().map(|p| p.display().to_string()),
            shortcuts: settings::shortcuts_to_strings(&self.keybindings),
        }
    }

    /// Appelé à chaque frame : n'écrit que si un réglage a bougé. Une
    /// écriture qui échoue est signalée une seule fois (l'instantané est
    /// mis à jour quoi qu'il arrive) plutôt que retentée à chaque frame.
    fn persist_settings_if_changed(&mut self) {
        let Some(path) = self.settings_file.clone() else {
            return;
        };
        let current = self.settings_snapshot();
        if current == self.saved_settings {
            return;
        }
        let result = current.save_to(&path);
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
            // Des barres de défilement qui prennent leur place, au lieu
            // de flotter par-dessus le contenu et de s'effacer dès que la
            // souris s'en va : flottantes, elles n'apparaissaient qu'au
            // survol, et rien ne disait qu'il y avait quelque chose de
            // plus bas. Elles restent affichées seulement quand il y a de
            // quoi défiler.
            style.spacing.scroll.floating = false;
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
                // Ce que nasm ne dit pas : un programme qui ne s'arrête
                // pas, une écriture dans une zone en lecture seule. Ces
                // deux fautes-là passent l'assemblage sans un mot et ne
                // se manifestent qu'à l'exécution — ou pire, pas du tout.
                self.diagnostics.extend(namsdec_asm::lint::check(&output));
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
    /// Ramène la ligne courante sous les yeux au prochain rendu.
    fn follow_execution(&mut self) {
        self.follow_rip = true;
    }

    /// Combien de fois chaque ligne source a été exécutée.
    ///
    /// L'émulateur compte par adresse ; une ligne en produit parfois
    /// plusieurs, et c'est la ligne qu'on lit. Vide tant que rien n'a
    /// tourné — auquel cas la gouttière ne réserve aucune place.
    fn line_visits(&self) -> BTreeMap<u32, u64> {
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            return BTreeMap::new();
        };
        let mut par_ligne: BTreeMap<u32, u64> = BTreeMap::new();
        for (addr, count) in emulator.visits() {
            if let Some(line) = state.addr_to_line.get(addr) {
                *par_ligne.entry(*line).or_insert(0) += count;
            }
        }
        par_ligne
    }

    fn remember_registers(&mut self) {
        self.regs_before = self.emulator.as_ref().map(|e| e.cpu().regs.clone());
        self.flags_before = self.emulator.as_ref().map(|e| e.cpu().flags);
        // Et la fenêtre mémoire qu'on regarde : mettre en avant ce qui a
        // changé vaut pour elle autant que pour les registres, et cent
        // vingt-huit octets copiés à chaque pas ne coûtent rien.
        self.mem_before = self
            .emulator
            .as_ref()
            .and_then(|e| e.cpu().mem.peek_bytes(self.mem_addr, MEMORY_WINDOW).ok())
            .map(<[u8]>::to_vec);
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
        // Les droits : ce qui sépare un programme qui plante proprement
        // d'un programme qui écrit dans son propre code sans que personne
        // ne le dise.
        //
        // Seulement si l'on sait où sont les sections : un binaire brut
        // ouvert depuis le disque n'en déclare aucune, et inventer des
        // droits pour lui reviendrait à refuser ce qu'on ne comprend pas.
        if !state.output.sections.is_empty() {
            for section in &state.output.sections {
                let perms = namsdec_core::Perms {
                    read: true,
                    write: section.writable,
                    execute: section.executable,
                };
                mem.map(
                    section.addr..section.addr + section.size as u64,
                    perms,
                    region_label(section),
                );
            }
            // La pile : on y écrit, on n'y exécute pas. C'est la
            // protection qui a mis fin aux débordements de tampon des
            // années 90.
            mem.map(
                HEAP_LIMIT..BASE_ADDR + MEM_SIZE as u64,
                namsdec_core::Perms::DATA,
                "stack",
            );
        }

        let mut cpu = namsdec_core::Cpu::new(mem);
        // Le point d'entrée de l'ELF, et non le premier octet chargé : sur
        // un programme qui commence par `.data`, ce sont deux adresses
        // différentes.
        cpu.regs.rip = state.output.entry;
        cpu.regs.set64(Reg64::Rsp, STACK_TOP);

        // Un programme qui vient d'être chargé n'a pas de passé : rien à
        // mettre en avant tant qu'il n'a pas avancé d'un pas.
        self.regs_before = None;
        self.flags_before = None;
        self.last_step = None;
        self.error_advice = None;
        self.mem_before = None;
        let mut emulator = Emulator::new(cpu);
        // Le tas commence après l'image du programme et monte vers la
        // pile : c'est la disposition d'un vrai processus, et c'est elle
        // qui donne un sens à `brk`.
        emulator.set_heap(
            state.output.load_addr + state.output.bytes.len() as u64,
            HEAP_LIMIT,
        );
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
            binary: &state.output.file,
            lang: self.lang.code(),
        };
        self.re_registry.on_binary_loaded(&ctx);

        self.emulator = Some(Box::new(emulator));
        // Les zones surveillées survivent au réassemblage : on travaille
        // souvent sur la même adresse en corrigeant son programme.
        self.sync_watchpoints();
    }

    /// Pousse les zones surveillées dans le moteur, ce qui les arme sur
    /// l'état courant.
    fn sync_watchpoints(&mut self) {
        let points = self.watchpoints.clone();
        // Les registres suivent le même chemin : la liste vit dans
        // l'application, le moteur la reçoit et l'arme sur l'état courant.
        let registers: Vec<(&'static str, Reg64)> = REGISTERS
            .iter()
            .filter(|(name, _)| self.watched_registers.contains(name))
            .copied()
            .collect();
        if let Some(emulator) = &mut self.emulator {
            emulator.set_watchpoints(&points);
            emulator.set_watched_registers(&registers);
        }
    }

    /// Ce point d'arrêt doit-il arrêter, cette fois-ci ?
    ///
    /// Sans condition, oui, toujours. Avec une condition qu'on ne sait
    /// pas lire, oui aussi : un point d'arrêt qui ne s'arrête jamais à
    /// cause d'une faute de frappe est pire qu'un point d'arrêt qui
    /// s'arrête trop.
    fn breakpoint_holds(&self, addr: u64) -> bool {
        let Some(state) = &self.assemble_state else {
            return true;
        };
        let Some(line) = state.addr_to_line.get(&addr) else {
            return true;
        };
        let Some(texte) = self.breakpoint_conditions.get(line) else {
            return true;
        };
        let Some(condition) = condition::parse(texte) else {
            return true;
        };
        let Some(emulator) = &self.emulator else {
            return true;
        };
        condition.holds(&emulator.cpu().regs)
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
            binary: &state.output.file,
            lang: self.lang.code(),
        };
        self.re_registry.on_step(&ctx);
    }

    /// Retient l'instruction que le pas qui commence va exécuter.
    ///
    /// Prise *avant* le pas : après, RIP a bougé, et l'instruction qu'on
    /// vient de faire n'est plus sous le pointeur. C'est elle qui donne
    /// son sens à ce qui a changé — « RCX 0x5 → 0x4 » ne veut rien dire
    /// sans le `dec rcx` qui l'a fait.
    fn remember_step(&mut self) {
        self.last_step = self.emulator.as_ref().and_then(|emulator| {
            let cpu = emulator.cpu();
            let rip = cpu.regs.rip;
            let window = cpu.mem.peek_bytes(rip, 16).ok()?;
            let instr = namsdec_disasm::decode_one(window, rip)?;
            Some((rip, namsdec_disasm::format_one(&instr)))
        });
    }

    fn do_step(&mut self) {
        self.remember_registers();
        self.remember_step();
        // Le conseil vaut pour l'erreur qui vient de se produire : au pas
        // suivant, il parlerait d'autre chose.
        self.error_advice = None;
        self.follow_execution();
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
            Err(e) => {
                self.error_advice = errors::advice(self.lang, &e);
                self.error_advice = errors::advice(self.lang, &e);
                self.status = Status::Error(errors::emu(self.lang, &e));
            }
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
        // Une course n'a pas « un » dernier pas : le bandeau se tait
        // plutôt que de montrer la première instruction d'un millier.
        self.last_step = None;
        self.error_advice = None;
        // Photographie prise au départ : à l'arrêt suivant — un point
        // d'arrêt, un `hlt` — le bandeau montre tout ce que la course a
        // changé, et non le seul dernier pas.
        self.remember_registers();
        self.resume_run_after_input = false;
        self.running = true;
        self.run_steps_taken = 0;
        self.status = Status::Running;
    }

    /// Exécute jusqu'à la ligne du curseur.
    ///
    /// Un point d'arrêt pour un seul voyage : le geste qu'on fait vingt
    /// fois par heure en apprenant — « va jusque-là et montre-moi » —
    /// demandait jusqu'ici de poser un point, de courir, puis de penser à
    /// le retirer.
    fn do_run_to_cursor(&mut self) {
        self.last_step = None;
        self.error_advice = None;
        let Some(state) = &self.assemble_state else {
            return;
        };
        // La ligne du curseur, ou la première en dessous qui produise du
        // code : on pose souvent le curseur sur une ligne vide ou sur un
        // commentaire, et « aller là » veut alors dire « aller à ce qui
        // suit ».
        let from = self.cursor_pos.0 as u32;
        let Some((_, &addr)) = state.output.line_to_addr.range(from..).next() else {
            return;
        };
        let mut points = self
            .breakpoint_lines
            .iter()
            .filter_map(|l| state.output.line_to_addr.get(l).copied())
            .collect::<BTreeSet<u64>>();
        points.insert(addr);
        if let Some(emulator) = &mut self.emulator {
            emulator.set_breakpoints(points);
        }
        self.run_to = Some(addr);
        self.do_run();
    }

    /// À appeler une fois par frame. N'avance que si un Run est en cours.
    fn tick_run(&mut self, ctx: &egui::Context) {
        if !self.running {
            // La course finie, le point d'arrêt d'un seul voyage s'en va
            // avec elle : il n'appartient pas à l'utilisateur, et le
            // laisser là ferait s'arrêter la course suivante à un endroit
            // que personne n'a demandé.
            if self.run_to.take().is_some() {
                self.sync_breakpoints();
            }
            return;
        }
        // Un point d'arrêt dont la condition est fausse ne doit pas
        // coûter une frame : la course reprend tout de suite, dans le
        // budget de pas de cette frame-ci. Sans cela, traverser une
        // boucle de cinq cents tours demanderait cinq cents frames —
        // huit secondes à ne rien voir bouger.
        let mut budget = RUN_BATCH_STEPS;
        let arrêt = loop {
            let Some(emulator) = &mut self.emulator else {
                self.running = false;
                return;
            };
            let avant = emulator.executed();
            let arrêt = emulator.run(budget);
            let consommés = self
                .emulator
                .as_ref()
                .map(|e| e.executed().saturating_sub(avant))
                .unwrap_or(0);
            budget = budget.saturating_sub(consommés.min(u64::from(budget)) as u32);
            match &arrêt {
                Ok(RunStop::Breakpoint(addr)) if !self.breakpoint_holds(*addr) && budget > 0 => {
                    continue;
                }
                _ => break arrêt,
            }
        };
        let Some(emulator) = &mut self.emulator else {
            self.running = false;
            return;
        };
        match arrêt {
            Ok(RunStop::Halted) => {
                self.running = false;
                self.status = halted_status(emulator.as_ref());
                self.back_to_command_mode();
            }
            Ok(RunStop::Breakpoint(addr)) => {
                // La condition décide. Fausse, la course reprend sans
                // que personne n'ait à cliquer : c'est tout l'objet d'un
                // point d'arrêt conditionnel dans une boucle de mille
                // tours.
                if self.breakpoint_holds(addr) {
                    self.running = false;
                    self.status = Status::Breakpoint(addr);
                }
            }
            Ok(RunStop::Watchpoint { addr, old, new }) => {
                self.running = false;
                self.status = Status::Watchpoint { addr, old, new };
                self.back_to_command_mode();
            }
            Ok(RunStop::RegisterChanged { register, old, new }) => {
                self.running = false;
                self.status = Status::RegisterChanged { register, old, new };
                self.back_to_command_mode();
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
                self.error_advice = errors::advice(self.lang, &e);
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

    /// Défait le dernier pas.
    ///
    /// Le bandeau montre alors ce que ce pas avait changé : la
    /// photographie est prise *avant* de reculer, si bien que la mise en
    /// avant désigne exactement ce qu'on vient d'annuler.
    fn do_step_back(&mut self) {
        self.follow_execution();
        // Le pas qu'on défait n'est plus le dernier pas fait.
        self.last_step = None;
        // Un Run suspendu ne repart pas tout seul après un retour en
        // arrière : c'est l'utilisateur qui tient la main, comme pour un
        // pas ordinaire.
        self.resume_run_after_input = false;
        self.running = false;
        self.remember_registers();
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        if emulator.step_back() {
            self.status = Status::SteppedBack;
            self.back_to_command_mode();
            self.notify_step();
        } else {
            // Rien à défaire : on le dit plutôt que de ne rien faire.
            self.status = Status::NothingToUndo;
            self.regs_before = None;
            self.flags_before = None;
            self.last_step = None;
            self.error_advice = None;
            self.mem_before = None;
        }
    }

    fn do_step_over(&mut self) {
        self.follow_execution();
        self.remember_registers();
        self.remember_step();
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
            Ok(StepOverResult::Watchpoint { addr, old, new }) => {
                self.status = Status::Watchpoint { addr, old, new }
            }
            Ok(StepOverResult::StepLimit) => self.status = Status::StepLimit,
            Ok(StepOverResult::NeedsInput) => self.status = Status::WaitingForInput,
            Ok(StepOverResult::Stepped | StepOverResult::SteppedOverCall) => {}
            Err(e) => {
                self.error_advice = errors::advice(self.lang, &e);
                self.error_advice = errors::advice(self.lang, &e);
                self.status = Status::Error(errors::emu(self.lang, &e));
            }
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
            Action::StepBack => self.do_step_back(),
            Action::Stop => self.do_stop(),
            Action::Reset => {
                self.rebuild_emulator();
                self.status = Status::Idle;
            }
            Action::Save => self.do_save(),
            Action::OpenFile => self.do_open_file_dialog(),
            Action::OpenFolder => self.do_open_folder(),
            Action::InstructionHelp => self.do_instruction_help(),
            Action::RunToCursor => self.do_run_to_cursor(),
            Action::GoToLabel => self.do_goto_label(),
            Action::Find => self.do_find(),
            Action::New => self.do_new(),
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
                FileDialogKind::OpenFile
                | FileDialogKind::AddTheme
                | FileDialogKind::AddExercises => dialog.pick_file(),
                FileDialogKind::OpenFolder => dialog.pick_folder(),
                FileDialogKind::SaveAs
                | FileDialogKind::ExportManual
                | FileDialogKind::ExportReport => dialog.save_file(),
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
            // Un thème est un fichier JSON — le filtre le dit, sans
            // interdire d'en choisir un autre : c'est la lecture qui
            // tranche, pas l'extension.
            FileDialogKind::AddTheme => dialog.add_filter(s.filter_theme, &["json"]),
            FileDialogKind::AddExercises => dialog.add_filter(s.filter_exercises, &["json"]),
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
            FileDialogKind::ExportManual => dialog
                .add_filter(s.filter_markdown, &["md"])
                .set_file_name("namsdec-manuel.md"),
            FileDialogKind::ExportReport => dialog
                .add_filter(s.filter_markdown, &["md"])
                // Le nom du fichier analysé, pour ne pas se retrouver
                // avec trois rapports qui portent le même nom.
                .set_file_name(format!(
                    "{}-analyse.md",
                    self.current_file
                        .as_ref()
                        .and_then(|path| path.file_stem())
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "namsdec".to_owned())
                )),
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
                    self.finish_file_dialog(kind, path, ctx);
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
    fn finish_file_dialog(&mut self, kind: FileDialogKind, path: PathBuf, ctx: &egui::Context) {
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
            FileDialogKind::AddTheme => self.install_theme(&path, ctx),
            FileDialogKind::AddExercises => self.install_exercises(&path),
            FileDialogKind::ExportManual => self.export_manual(&path),
            FileDialogKind::ExportReport => self.export_report(&path),
        }
    }

    /// Écrit le manuel entier dans un fichier Markdown.
    ///
    /// Ce que namsdec explique n'a pas à rester dans namsdec : la
    /// licence est libre, le manuel se lit, s'imprime et se distribue
    /// sans lui. La langue est celle de l'interface : on exporte ce
    /// qu'on lit.
    fn export_manual(&mut self, path: &Path) {
        let s = strings(self.lang);
        match std::fs::write(path, namsdec_doc::markdown(self.lang.code())) {
            Ok(()) => self.status = Status::Note(s.manual_exported),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.manual_export_failed)),
        }
    }

    /// Écrit le rapport d'analyse dans un fichier Markdown.
    fn export_report(&mut self, path: &Path) {
        let s = strings(self.lang);
        let Some(report) = self.analysis_report() else {
            self.status = Status::Error(s.analysis_none.to_owned());
            return;
        };
        match std::fs::write(path, report) {
            Ok(()) => self.status = Status::Note(s.report_saved),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.report_save_failed)),
        }
    }

    /// Installe un thème choisi dans le sélecteur.
    ///
    /// Le fichier est d'abord **lu**, donc validé : rien n'est copié dans
    /// le dossier des plugins tant qu'on ne sait pas que c'est un thème
    /// complet. Un fichier illisible ne laisse donc aucune trace, et le
    /// message dit ce qui cloche.
    fn install_theme(&mut self, path: &Path, ctx: &egui::Context) {
        let s = strings(self.lang);
        let note = match namsdec_theme::plugin::read(path) {
            Err(e) => PluginNote {
                ok: false,
                text: errors::plugin(self.lang, &e),
            },
            Ok(theme) => {
                let name = theme.name.to_owned();
                match namsdec_theme::install(theme) {
                    Err(e) => PluginNote {
                        ok: false,
                        text: errors::plugin(self.lang, &e),
                    },
                    Ok(installed) => {
                        // Copié seulement maintenant : le thème est bon,
                        // il doit revenir au lancement suivant.
                        let copied = copy_into_plugins(path);
                        if let Ok(destination) = &copied {
                            // D'où vient le thème : c'est ce qui rendra
                            // son retrait possible.
                            self.theme_files
                                .insert(installed.id.to_owned(), destination.clone());
                        }
                        // Appliqué aussitôt : on installe un thème pour
                        // le voir, pas pour le chercher ensuite dans la
                        // liste.
                        self.theme = installed;
                        self.apply_theme(ctx);
                        match copied {
                            Ok(_) => PluginNote {
                                ok: true,
                                text: format!("{} — {name}", s.plugins_added),
                            },
                            Err(e) => PluginNote {
                                ok: false,
                                text: format!("{} ({e})", s.plugins_added_not_kept),
                            },
                        }
                    }
                }
            }
        };
        self.plugin_note = Some(note);
    }

    /// Charge `path`. Du texte part dans l'éditeur ; tout le reste est
    /// traité comme un binaire brut à désassembler, plutôt que rejeté
    /// comme "pas de l'UTF-8" — c'est un atelier de reverse engineering,
    /// ouvrir un binaire doit y mener quelque part.
    /// Repart d'un programme neuf.
    fn do_new(&mut self) {
        self.guard(Pending::New);
    }

    fn new_program(&mut self) {
        self.source = NEW_PROGRAM.to_owned();
        self.saved_source = NEW_PROGRAM.to_owned();
        self.current_file = None;
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.breakpoint_lines.clear();
        self.status = Status::Idle;
    }

    /// Ouvre un fichier — après s'être assuré qu'on ne perd rien.
    fn open_file(&mut self, path: PathBuf) {
        self.guard(Pending::OpenFile(path));
    }

    /// Y a-t-il quelque chose à perdre ?
    ///
    /// La comparaison se fait sur le texte, pas sur un drapeau : taper
    /// un caractère puis l'effacer ne met rien en danger.
    fn buffer_modified(&self) -> bool {
        self.source != self.saved_source
    }

    /// Fait l'action, ou demande d'abord.
    fn guard(&mut self, action: Pending) {
        if self.buffer_modified() {
            self.pending = Some(action);
        } else {
            self.perform(action);
        }
    }

    fn perform(&mut self, action: Pending) {
        match action {
            Pending::New => self.new_program(),
            Pending::OpenFile(path) => self.open_file_now(path),
            Pending::Exercise(id) => self.load_exercise_now(&id),
            Pending::Quit => self.quitting = true,
        }
    }

    fn open_file_now(&mut self, path: PathBuf) {
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
                self.saved_source = text.clone();
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
    /// Ouvre un ELF64 : ses sections vont à *leur* adresse, son point
    /// d'entrée est celui qu'il déclare, et seul le code est désassemblé.
    ///
    /// Sans cela, un vrai binaire commençait par seize octets d'en-tête
    /// lus comme des instructions : le listing ouvrait sur du charabia, et
    /// l'exécution démarrait dessus.
    ///
    /// Rend `false` si ce n'en est pas un — à l'appelant de retomber sur
    /// le désassemblage brut.
    fn load_elf(&mut self, bytes: &[u8]) -> bool {
        let Some(elf) = namsdec_asm::elf::parse(bytes) else {
            return false;
        };
        let Some((load_addr, image)) = elf.image() else {
            return false;
        };
        if image.len() > MEM_SIZE {
            return false;
        }

        // Le listing ne montre que les sections exécutables : le reste est
        // de la donnée, et la donnée désassemblée ne veut rien dire.
        let mut disasm = Vec::new();
        for section in elf.sections.iter().filter(|s| s.executable) {
            disasm.extend(namsdec_disasm::disassemble(&section.data, section.addr));
        }
        disasm.sort_by_key(|instruction| instruction.address);

        // Les entrées de la table de liaison rejoignent les étiquettes :
        // lire un binaire, c'est d'abord savoir ce qu'il appelle.
        let mut symbols = elf.symbols.clone();
        symbols.extend(plt_labels(bytes, &elf.symbols, Some));
        symbols.sort_by_key(|symbol| symbol.addr);
        let (listing, line_to_addr) = annotated_listing(&disasm, &symbols);
        let addr_to_line = line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
        let loaded = image.len();

        self.saved_source = listing.clone();
        self.source = listing;
        self.current_file = None;
        self.breakpoint_lines.clear();
        self.diagnostics.clear();
        self.assemble_state = Some(AssembleState {
            output: namsdec_asm::AssembleOutput {
                load_addr,
                entry: elf.entry,
                sections: elf
                    .sections
                    .iter()
                    .map(|section| namsdec_asm::Section {
                        name: section.name.clone(),
                        addr: section.addr,
                        size: section.size,
                        executable: section.executable,
                        writable: section.writable,
                    })
                    .collect(),
                bytes: image,
                diagnostics: Vec::new(),
                line_to_addr,
                // Les étiquettes de l'ELF ouvert : elles nomment les
                // adresses aussi bien ici que pour un programme qu'on
                // vient d'assembler soi-même.
                symbols,
                // Le fichier entier : l'analyse des protections se fait
                // sur ses en-têtes, pas sur l'image chargée.
                file: bytes.to_vec(),
            },
            disasm,
            addr_to_line,
        });
        self.rebuild_emulator();
        if !matches!(self.status, Status::Error(_)) {
            self.status = Status::BinaryLoaded {
                bytes: loaded,
                truncated: false,
            };
        }
        true
    }

    fn load_binary(&mut self, mut bytes: Vec<u8>) {
        // Un ELF64 dit lui-même où il va et par où il commence : autant
        // l'écouter.
        if self.load_elf(&bytes) {
            return;
        }

        // Le fichier entier est gardé de côté avant toute troncature :
        // l'analyse — protections, sections, symboles — se lit dans ses
        // en-têtes, et ceux-là ne dépendent pas de ce qui tient dans la
        // mémoire simulée. Un ELF trop gros pour s'exécuter ici reste
        // ainsi parfaitement lisible.
        let file = if bytes.starts_with(b"\x7fELF") {
            bytes.clone()
        } else {
            Vec::new()
        };

        // Borné à la fenêtre mémoire simulée : au-delà, `rebuild_emulator`
        // refuserait le chargement de toute façon, et désassembler
        // plusieurs centaines de kilo-octets pour les jeter ensuite
        // figerait l'éditeur le temps de la mise en page du listing.
        let truncated = bytes.len() > MEM_SIZE;
        bytes.truncate(MEM_SIZE);

        let disasm = namsdec_disasm::disassemble(&bytes, BASE_ADDR);

        // Même chose pour un fichier lu tel quel : ses entrées de liaison
        // se nomment, à ceci près que le listing part du premier octet du
        // fichier et non des adresses où il se chargerait.
        let symbols = plt_labels(&file, &[], |plt| {
            let offset = namsdec_re::imports::file_offset(&file, plt)?;
            let addr = BASE_ADDR.checked_add(offset)?;
            (offset < bytes.len() as u64).then_some(addr)
        });
        let (listing, line_to_addr) = annotated_listing(&disasm, &symbols);

        let addr_to_line = line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
        let loaded = bytes.len();
        self.saved_source = listing.clone();
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
                // Un binaire brut n'a pas de table de symboles ; un ELF
                // lu tel quel a au moins ses entrées de liaison, qui
                // nomment ce qu'il appelle.
                symbols,
                file,
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
            PaletteCommand::ShowExercises => self.show_exercises = true,
            PaletteCommand::ShowAnalysis => self.show_analysis = true,
            PaletteCommand::ShowVectors => self.show_vectors = true,
            PaletteCommand::ShowBreakpoints => self.show_breakpoints = true,
            PaletteCommand::ShowTimeline => self.show_timeline = true,
            PaletteCommand::ShowCyclic => self.show_cyclic = true,
            PaletteCommand::ShowDisassembly => self.show_disassembly = true,
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
        let commands = palette_commands(s, self.show_re_tab, self.exercises);

        let (mut query, mut selected, measured_row) = {
            let p = self.command_palette.as_ref().unwrap();
            (p.query.clone(), p.selected, p.row)
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

        // Ce que la palette peut occuper en hauteur : tout ce qui reste
        // sous son ancrage, moins une marge en bas. Une hauteur figée
        // laissait la liste défiler dans une fenêtre à moitié vide, sur
        // un écran qui l'aurait affichée en entier.
        const TOP: f32 = 48.0;
        const BOTTOM_MARGIN: f32 = 24.0;
        let screen = ctx.input(|i| i.content_rect()).height();
        let room = (screen - TOP - BOTTOM_MARGIN).max(180.0);
        // La place du champ de saisie, du séparateur et des marges de la
        // fenêtre, au-dessus de la liste.
        const HEAD: f32 = 64.0;
        // Avant la première mesure, une estimation : une ligne par
        // commande, à la hauteur d'un élément cliquable.
        let style = ctx.style_of(ctx.theme());
        let row =
            measured_row.unwrap_or(style.spacing.interact_size.y + style.spacing.item_spacing.y);
        let shown = filtered.len().max(1) as f32;
        let list = (shown * row).min(room - HEAD);

        egui::Window::new("command_palette")
            .id(egui::Id::new("command_palette"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, TOP))
            // Largeur figée, hauteur libre : la fenêtre descend jusqu'à
            // la dernière commande et s'arrête là.
            .fixed_size(egui::vec2(520.0, list + HEAD))
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
                let area = egui::ScrollArea::vertical()
                    .max_height(list)
                    // `false` en largeur : la liste s'étale sur toute la
                    // palette, sinon elle se serre à gauche et sa barre
                    // de défilement — quand il en faut une — tombe au
                    // milieu de la fenêtre.
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        // Des lignes un peu plus serrées que la normale :
                        // une palette est une liste qu'on parcourt des
                        // yeux, et chaque pixel gagné par ligne, c'est
                        // une commande de plus qui reste à l'écran au
                        // lieu de passer sous une barre de défilement.
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.spacing_mut().button_padding.y = 3.0;
                        if filtered.is_empty() {
                            ui.weak(s.palette_no_match);
                        }
                        for (i, (label, cmd)) in filtered.iter().enumerate() {
                            // Pleine largeur : la ligne entière se
                            // surligne et se clique, pas seulement son
                            // texte.
                            let item = ui.add(
                                egui::Button::selectable(i == selected, label.as_str())
                                    .min_size(egui::vec2(ui.available_width(), 0.0)),
                            );
                            // Les flèches déplacent la sélection ; si la
                            // liste défile, elle doit suivre.
                            if i == selected && !filtered.is_empty() {
                                item.scroll_to_me(None);
                            }
                            if item.clicked() {
                                execute = Some(*cmd);
                            }
                        }
                    });
                // Ce qu'une ligne prend vraiment, pour la frame suivante.
                if let Some(p) = &mut self.command_palette {
                    p.row = Some(area.content_size.y / shown);
                }
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
                self.saved_source = self.source.clone();
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
                        s.file_new,
                        Action::New,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::New);
                    ui.close();
                }
                ui.separator();
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
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        &ui.ctx().clone(),
                        s.action_find,
                        Action::Find,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::Find);
                    ui.close();
                }
                ui.separator();
                if ui.button(s.edit_copy_code).clicked() {
                    ui.ctx().copy_text(self.source.clone());
                    ui.close();
                }
                // Le désassemblage, lui, n'existe qu'après l'assemblage
                // — et c'est ce qu'on colle dans un compte rendu, pas le
                // source.
                ui.add_enabled_ui(self.assemble_state.is_some(), |ui| {
                    if ui.button(s.edit_copy_listing).clicked() {
                        if let Some(listing) = self.listing_text() {
                            ui.ctx().copy_text(listing);
                            self.status = Status::Note(s.listing_copied);
                        }
                        ui.close();
                    }
                });
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
                    if ui
                        .button(shortcut_hint(
                            &self.keybindings,
                            &ui.ctx().clone(),
                            s.action_run_to_cursor,
                            Action::RunToCursor,
                        ))
                        .clicked()
                    {
                        self.dispatch(Action::RunToCursor);
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
                ui.separator();
                // L'analyse du binaire dans sa propre fenêtre : le
                // panneau latéral est trop étroit et trop court pour une
                // liste de protections, de segments et de chaînes — et
                // l'onglet RE est masqué par défaut, donc invisible pour
                // qui ne sait pas qu'il existe.
                if ui.button(s.analysis_open).clicked() {
                    self.show_analysis = true;
                    ui.close();
                }
                if ui.button(s.vectors_open).clicked() {
                    self.show_vectors = true;
                    ui.close();
                }
                if ui.button(s.breakpoints_open).clicked() {
                    self.show_breakpoints = true;
                    ui.close();
                }
                if ui.button(s.timeline_open).clicked() {
                    self.show_timeline = true;
                    ui.close();
                }
                if ui.button(s.disassembly_open).clicked() {
                    self.show_disassembly = true;
                    ui.close();
                }
                if ui
                    .button(s.cyclic_open)
                    .on_hover_text(s.cyclic_hint)
                    .clicked()
                {
                    self.show_cyclic = true;
                    ui.close();
                }
                ui.separator();
                // Le module des exercices se charge à la demande. Mais
                // une porte qu'on ne voit pas n'existe pas : l'entrée
                // reste visible, grisée, et dit où l'ouvrir. Sans cela,
                // un débutant ne saura jamais qu'il y a un parcours.
                if self.exercises {
                    if ui.button(s.exercises_open).clicked() {
                        self.show_exercises = true;
                        ui.close();
                    }
                } else {
                    ui.add_enabled(false, egui::Button::new(s.exercises_open))
                        .on_disabled_hover_text(s.exercises_where);
                }
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
                // La fiche de l'instruction où l'on est : F1 la donne,
                // mais encore faut-il savoir que F1 fait quelque chose.
                if ui.button(s.action_instruction_help).clicked() {
                    self.dispatch(Action::InstructionHelp);
                    ui.close();
                }
                // Même raison : un raccourci qu'on ne connaît pas
                // n'existe pas.
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        &ui.ctx().clone(),
                        s.action_goto_label,
                        Action::GoToLabel,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::GoToLabel);
                    ui.close();
                }
                ui.separator();
                // Le manuel sort d'ici : un fichier qu'on garde, qu'on
                // imprime, qu'on donne. Le savoir ne sert que partagé.
                if ui
                    .button(s.help_export_manual)
                    .on_hover_text(s.help_export_manual_hint)
                    .clicked()
                {
                    self.spawn_file_dialog(FileDialogKind::ExportManual);
                    ui.close();
                }
                ui.separator();
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
                exercises: self.exercises,
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
            self.exercises = snapshot.exercises;
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

        ui.add_space(6.0);
        ui.checkbox(&mut self.completion, s.preferences_completion);
        ui.weak(s.preferences_completion_hint);
        ui.checkbox(&mut self.auto_indent, s.preferences_auto_indent);
        ui.weak(s.preferences_auto_indent_hint);
        ui.checkbox(&mut self.hover_help, s.preferences_hover_help);
        ui.weak(s.preferences_hover_help_hint);

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
            .max_height(window_room(ui))
            .show(ui, |ui| {
                self.modules_ui(ui);
                ui.checkbox(&mut self.show_re_tab, s.show_re_tab_checkbox);
                ui.checkbox(&mut self.exercises, s.exercises_checkbox);
                ui.weak(s.exercises_hint);
            });
    }

    /// Onglet Plugins : pour l'instant les thèmes, qui en sont le premier
    /// type. Un thème est une simple palette (`namsdec-theme`), donc le
    /// jour où d'autres s'installent, ils se contentent d'allonger la
    /// liste — rien ici n'est écrit par thème.
    fn preferences_plugins_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // Le thème *de l'interface*, à ne pas confondre avec ceux que la
        // page liste — nommés `listed` plus bas pour cette raison.
        let theme = self.theme;
        let ctx = ui.ctx().clone();

        ui.strong(s.plugins_themes);
        ui.weak(s.plugins_themes_hint);
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .max_height(window_room(ui))
            .show(ui, |ui| {
                let themes = namsdec_theme::all();
                let card_width = theme_card_width(ui, &themes, s.plugins_active);
                for listed in themes {
                    ui.horizontal(|ui| {
                        let active = std::ptr::eq(self.theme, listed);
                        let response = theme_card(ui, listed, active, s.plugins_active, card_width);
                        if response.clicked() && !active {
                            self.theme = listed;
                            self.apply_theme(&ctx);
                        }
                        // Seuls les thèmes venus d'un fichier se
                        // retirent : les trois d'origine ne figurent pas
                        // dans cette table, aucun bouton ne les propose
                        // donc. Et comme les vignettes ont toutes la même
                        // largeur, les boutons tombent les uns sous les
                        // autres.
                        if self.theme_files.contains_key(listed.id)
                            && framed_button(ui, theme, s.plugins_remove).clicked()
                        {
                            self.theme_to_remove = Some(listed.id.to_owned());
                        }
                    });
                    ui.add_space(4.0);
                }
            });

        // Le retrait attend une confirmation : supprimer un fichier ne
        // se fait pas d'un seul clic, surtout à côté de celui qui
        // applique le thème.
        if let Some(id) = self.theme_to_remove.clone() {
            ui.separator();
            ui.colored_label(
                theme.warn,
                format!(
                    "{} {}",
                    s.plugins_remove_question,
                    namsdec_theme::by_id(&id).name
                ),
            );
            ui.horizontal(|ui| {
                if framed_button(ui, theme, s.plugins_remove_yes).clicked() {
                    self.remove_theme(&id, &ctx);
                    self.theme_to_remove = None;
                }
                if framed_button(ui, theme, s.plugins_remove_no).clicked() {
                    self.theme_to_remove = None;
                }
            });
        }

        ui.add_space(6.0);
        ui.separator();
        if framed_button(ui, theme, s.plugins_add).clicked() {
            self.spawn_file_dialog(FileDialogKind::AddTheme);
        }
        ui.weak(s.plugins_add_hint);

        // Les séries d'exercices s'installent de la même façon : un
        // fichier JSON, copié dans le dossier de données, relu au
        // lancement suivant. Même page, même gestes.
        ui.add_space(10.0);
        ui.separator();
        ui.label(egui::RichText::new(s.exercises_packs_title).color(theme.dim));
        self.exercise_packs_ui(ui);
        // Sans thème installé, rien ne porte de bouton « Retirer » : la
        // page le dit, plutôt que de laisser croire qu'il manque quelque
        // chose.
        if self.theme_files.is_empty() {
            ui.weak(s.plugins_none_installed);
        }
        // Le compte rendu de la dernière installation reste affiché
        // jusqu'à la suivante : une boîte modale de plus pour dire « c'est
        // fait » n'apprendrait rien, un refus doit en revanche pouvoir
        // être relu.
        if let Some(note) = &self.plugin_note {
            let colour = if note.ok { theme.good } else { theme.bad };
            ui.colored_label(colour, &note.text);
        }
    }

    /// La liste des séries installées, avec de quoi en ajouter et en
    /// retirer. La série livrée avec l'application n'y figure pas : on ne
    /// retire pas ce qui n'a pas été installé.
    fn exercise_packs_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        if self.exercise_files.is_empty() {
            ui.weak(s.exercises_packs_none);
        }
        // Toutes les séries, celle qui est livrée comprise : chacune
        // s'écarte d'une case. Écarter n'est pas désinstaller — on garde
        // le fichier, on cesse seulement de proposer son contenu, et
        // c'est ce qu'on veut d'une série qu'on a finie.
        let listed: Vec<(String, String, usize, bool)> = self
            .exercise_packs
            .iter()
            .map(|pack| {
                (
                    pack.id.clone(),
                    pack.name.clone(),
                    pack.exercises.len(),
                    self.exercise_files.contains_key(&pack.id),
                )
            })
            .collect();
        for (id, name, count, installed) in listed {
            ui.horizontal(|ui| {
                let mut active = !self.disabled_packs.contains(&id);
                if ui
                    .checkbox(&mut active, format!("{name} ({count})"))
                    .changed()
                {
                    if active {
                        self.disabled_packs.remove(&id);
                    } else {
                        self.disabled_packs.insert(id.clone());
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Seule une série installée se retire : celle qui est
                    // livrée n'est nulle part sur le disque.
                    if installed && framed_button(ui, theme, s.plugins_remove).clicked() {
                        self.pack_to_remove = Some(id.clone());
                    }
                });
            });
        }
        if let Some(id) = self.pack_to_remove.clone() {
            ui.colored_label(
                theme.warn,
                format!("{} {id}", s.exercises_pack_remove_question),
            );
            ui.horizontal(|ui| {
                if framed_button(ui, theme, s.plugins_remove_yes).clicked() {
                    self.remove_exercises(&id);
                    self.pack_to_remove = None;
                }
                if framed_button(ui, theme, s.plugins_remove_no).clicked() {
                    self.pack_to_remove = None;
                }
            });
        }
        if framed_button(ui, theme, s.exercises_packs_add).clicked() {
            self.spawn_file_dialog(FileDialogKind::AddExercises);
        }
        ui.weak(s.exercises_packs_hint);
    }

    /// Retire un thème installé : son fichier est supprimé, et le thème
    /// quitte la liste.
    ///
    /// Si c'était le thème actif, on retombe sur celui d'origine plutôt
    /// que de garder à l'écran une palette qui n'existe plus nulle part.
    fn remove_theme(&mut self, id: &str, ctx: &egui::Context) {
        let s = strings(self.lang);
        let Some(path) = self.theme_files.get(id).cloned() else {
            return;
        };
        // Le fichier d'abord : s'il résiste, le thème reste installé et
        // on le dit, plutôt que de le faire disparaître de la liste pour
        // le voir revenir au lancement suivant.
        if let Err(e) = std::fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            self.plugin_note = Some(PluginNote {
                ok: false,
                text: format!("{} ({e})", s.plugins_remove_failed),
            });
            return;
        }

        let name = namsdec_theme::by_id(id).name;
        namsdec_theme::uninstall(id);
        self.theme_files.remove(id);
        if self.theme.id == id {
            self.theme = DEFAULT_THEME;
            self.apply_theme(ctx);
        }
        self.plugin_note = Some(PluginNote {
            ok: true,
            text: format!("{} — {name}", s.exercises_pack_removed),
        });
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

    /// Installe une série d'exercices choisie sur le disque.
    ///
    /// Lue d'abord, copiée ensuite : une série illisible est refusée en
    /// le disant, et rien n'est écrit dans le dossier des données.
    fn install_exercises(&mut self, path: &Path) {
        let s = strings(self.lang);
        let note = match namsdec_exercises::pack::read(path) {
            Err(e) => PluginNote {
                ok: false,
                text: errors::exercise_pack(self.lang, &e),
            },
            Ok(pack) => {
                if pack.id == namsdec_exercises::BUILTIN_ID {
                    // Le nom de la série livrée est pris : la laisser
                    // écraser ferait disparaître les exercices d'origine
                    // sans que personne ne l'ait demandé.
                    PluginNote {
                        ok: false,
                        text: format!("{} ({})", s.exercises_id_taken, pack.id),
                    }
                } else {
                    let name = pack.name.clone();
                    let id = pack.id.clone();
                    match copy_into_dir(path, exercises_dir()) {
                        Err(e) => PluginNote {
                            ok: false,
                            text: format!("{} ({e})", s.exercises_pack_not_kept),
                        },
                        Ok(destination) => {
                            // Une série réinstallée remplace la
                            // précédente au lieu de s'ajouter à côté.
                            self.exercise_packs.retain(|p| p.id != id);
                            self.exercise_packs.push(pack);
                            self.exercise_files.insert(id, destination);
                            PluginNote {
                                ok: true,
                                text: format!("{} — {name}", s.exercises_pack_added),
                            }
                        }
                    }
                }
            }
        };
        self.plugin_note = Some(note);
    }

    /// Retire une série installée : son fichier est supprimé, et elle
    /// quitte la liste. La série livrée avec l'application n'est pas
    /// concernée — elle n'est nulle part sur le disque.
    fn remove_exercises(&mut self, id: &str) {
        let s = strings(self.lang);
        let Some(path) = self.exercise_files.get(id).cloned() else {
            return;
        };
        if let Err(e) = std::fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            self.plugin_note = Some(PluginNote {
                ok: false,
                text: format!("{} ({e})", s.plugins_remove_failed),
            });
            return;
        }
        let name = self
            .exercise_packs
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| id.to_owned());
        self.exercise_packs.retain(|p| p.id != id);
        self.exercise_files.remove(id);
        self.plugin_note = Some(PluginNote {
            ok: true,
            text: format!("{} — {name}", s.plugins_removed),
        });
    }

    /// La liste des exercices : un titre, ce qu'on y cherche, et de quoi
    /// le charger dans l'éditeur.
    ///
    /// Charger remplace le tampon — d'où l'avertissement quand le fichier
    /// courant n'a pas été enregistré. Il n'y a pas de correction
    /// automatique : l'énoncé dit ce qu'on doit obtenir, et c'est le
    /// bandeau des registres qui répond.
    /// L'exercice qui suit `id` dans sa série : son identifiant et son
    /// titre. `None` pour le dernier — un parcours a une fin.
    fn exercise_after(&self, id: &str) -> Option<(String, String)> {
        for pack in &self.exercise_packs {
            let Some(position) = pack
                .exercises
                .iter()
                .position(|exercise| namsdec_exercises::full_id(pack, exercise) == id)
            else {
                continue;
            };
            let suivant = pack.exercises.get(position + 1)?;
            return Some((
                namsdec_exercises::full_id(pack, suivant),
                suivant.title.get(self.lang.code()).to_owned(),
            ));
        }
        None
    }

    /// Le rang de l'exercice `id` dans sa série, le nombre d'étapes, et
    /// l'exercice lui-même.
    fn exercise_rank(&self, id: &str) -> Option<(usize, usize, &namsdec_exercises::Exercise)> {
        for pack in &self.exercise_packs {
            for (index, exercise) in pack.exercises.iter().enumerate() {
                if namsdec_exercises::full_id(pack, exercise) == id {
                    return Some((index + 1, pack.exercises.len(), exercise));
                }
            }
        }
        None
    }

    fn exercises_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let mut open = self.show_exercises;
        let mut load = None;
        let mut fiche: Option<&'static str> = None;
        egui::Window::new(s.exercises_title)
            .id(egui::Id::new("exercises_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // Une taille de départ franche plutôt qu'une fenêtre qui
            // s'étire avec son contenu : deux séries installées la
            // faisaient déborder de l'écran, et la troisième n'avait
            // nulle part où aller.
            .default_size([480.0, 520.0])
            .pivot(egui::Align2::CENTER_CENTER)
            // `default_pos` et non `current_pos` : la seconde replace la
            // fenêtre à chaque frame, et l'on ne peut plus la déplacer.
            .default_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                ui.label(s.exercises_legend);
                // L'étape en cours, en tête : quinze exercices ne tiennent
                // pas dans la fenêtre, et « où j'en étais » ne sert à
                // rien s'il faut le chercher en défilant.
                if let Some(id) = self.current_exercise.clone()
                    && let Some((rang, total, exercise)) = self.exercise_rank(&id)
                {
                    ui.add_space(6.0);
                    // L'étape suivante, quand il y en a une : un parcours
                    // se suit, et aller à la suite ne doit pas demander
                    // de retrouver son rang dans une liste.
                    let suivante = self.exercise_after(&id);
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.exercises_current);
                        ui.colored_label(theme.accent, format!("{rang}/{total}"));
                        ui.strong(exercise.title.get(lang));
                        if framed_button(ui, theme, s.exercises_resume).clicked() {
                            load = Some(id.clone());
                        }
                        if let Some((suivant_id, titre)) = &suivante
                            && framed_button(ui, theme, s.exercises_next)
                                .on_hover_text(titre)
                                .clicked()
                        {
                            load = Some(suivant_id.clone());
                        }
                    });
                }
                ui.add_space(6.0);
                // La place que la fenêtre laisse vraiment, et pas une
                // fraction de l'écran : une zone qui se croit plus haute
                // que sa fenêtre ne défile pas — elle déborde, et ce qui
                // dépasse est coupé sans qu'aucune barre ne le dise.
                let room = ui.available_height().max(120.0);
                egui::ScrollArea::vertical()
                    .max_height(room)
                    // `false` en hauteur : une zone qui rétrécit à son
                    // contenu est aussi haute que ses quinze exercices,
                    // donc ne défile jamais — elle déborde de la fenêtre,
                    // et ce qui dépasse est coupé.
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for pack in self
                            .exercise_packs
                            .iter()
                            .filter(|pack| !self.disabled_packs.contains(&pack.id))
                        {
                            // Une série par repli : on doit pouvoir dire
                            // d'où vient l'exercice qu'on charge, et
                            // ranger celles qu'on n'utilise pas. Ouvertes
                            // par défaut — une liste qu'il faut déplier
                            // pour voir ce qu'elle contient n'aide
                            // personne le premier jour.
                            let header = egui::CollapsingHeader::new(
                                egui::RichText::new(&pack.name).color(theme.dim),
                            )
                            .id_salt(&pack.id)
                            .default_open(true);
                            let total = pack.exercises.len();
                            header.show(ui, |ui| {
                                for (index, exercise) in pack.exercises.iter().enumerate() {
                                    let id = namsdec_exercises::full_id(pack, exercise);
                                    let en_cours = self.current_exercise.as_deref() == Some(&id);
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            // Le rang dans le parcours : un
                                            // exercice n'est pas un devoir
                                            // isolé, il vient après les
                                            // précédents.
                                            ui.colored_label(
                                                theme.dim,
                                                format!("{}/{total}", index + 1),
                                            );
                                            ui.strong(exercise.title.get(lang));
                                            if en_cours {
                                                ui.colored_label(theme.accent, s.exercises_current);
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if framed_button(ui, theme, s.exercises_load)
                                                        .clicked()
                                                    {
                                                        load = Some(namsdec_exercises::full_id(
                                                            pack, exercise,
                                                        ));
                                                    }
                                                },
                                            );
                                        });
                                        // Les morceaux de code de l'énoncé
                                        // s'écrivent en chasse fixe, comme
                                        // dans les fiches d'instructions.
                                        let texte =
                                            prose(ui, theme, exercise.description.get(lang));
                                        ui.label(texte);
                                        // Les fiches dont l'exercice se
                                        // sert : un exercice n'apprend
                                        // rien sans le manuel à côté, et
                                        // aller le chercher soi-même
                                        // suppose de savoir quoi
                                        // chercher.
                                        let fiches = sheets_used(&exercise.starter_code);
                                        if !fiches.is_empty() {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.colored_label(theme.dim, s.exercises_sheets);
                                                for name in fiches {
                                                    if ui
                                                        .small_button(
                                                            egui::RichText::new(name).monospace(),
                                                        )
                                                        .on_hover_text(s.help_open_sheet)
                                                        .clicked()
                                                    {
                                                        fiche = Some(name);
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                            ui.add_space(4.0);
                        }
                    });
            });
        if let Some(id) = load {
            self.load_exercise(&id);
            open = false;
        }
        if let Some(name) = fiche {
            self.open_help(name);
            self.show_help = true;
        }
        self.show_exercises = open;
    }

    /// Met le programme de départ d'un exercice dans l'éditeur.
    ///
    /// `current_file` repart à zéro : ce qui est chargé est un modèle, pas
    /// le fichier de l'utilisateur, et « Enregistrer » ne doit pas écrire
    /// par-dessus celui qu'il avait ouvert.
    fn load_exercise(&mut self, id: &str) {
        self.guard(Pending::Exercise(id.to_owned()));
    }

    fn load_exercise_now(&mut self, id: &str) {
        let Some(exercise) = namsdec_exercises::find(&self.exercise_packs, id) else {
            return;
        };
        self.current_exercise = Some(id.to_owned());
        self.saved_source = exercise.starter_code.clone();
        self.source = exercise.starter_code.clone();
        self.current_file = None;
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.breakpoint_lines.clear();
        self.status = Status::Idle;
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
            // Le pas arrière a son propre grisé : il ne dépend pas de
            // l'état d'exécution mais de l'existence d'un pas à défaire.
            let can_undo = self.emulator.as_ref().is_some_and(|e| e.can_step_back());
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
            ui.add_enabled_ui(can_undo, |ui| {
                if icon_button(ui, icon_step_back, s.action_step_back)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.action_step_back,
                        Action::StepBack,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::StepBack);
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
            // Quel fichier on édite, et s'il reste quelque chose à
            // enregistrer. Sans cela, rien à l'écran ne distingue un
            // tampon sauvegardé d'un tampon qui ne l'est pas — et on ne
            // s'en aperçoit qu'en fermant.
            let file = self
                .current_file
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| s.untitled_buffer.to_owned());
            let modified = self.source != self.saved_source;
            ui.colored_label(
                if modified { theme.changed } else { theme.dim },
                if modified {
                    format!("{file} •")
                } else {
                    file
                },
            )
            .on_hover_text(if modified {
                s.buffer_modified
            } else {
                s.buffer_saved
            });
            ui.separator();
            // Le message est borné, et tronqué s'il déborde : les
            // compartiments de droite sont posés dans la place qui reste,
            // et un message long — le nom d'un fichier chargé, une erreur
            // de l'éditeur de liens — leur passait dessus, texte sur
            // texte.
            ui.scope(|ui| {
                ui.set_max_width((ui.available_width() - STATUS_RIGHT_ROOM).max(80.0));
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                self.status_message_ui(ui);
            });

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
            Status::Watchpoint { addr, old, new } => {
                status_dot(ui, theme.changed);
                // La valeur d'avant et celle d'après : c'est toute
                // l'information qu'on attendait en posant le point.
                ui.colored_label(
                    theme.changed,
                    if lang == Lang::Fr {
                        format!("Surveillance : {addr:#x} est passé de {old:#x} à {new:#x}")
                    } else {
                        format!("Watchpoint: {addr:#x} went from {old:#x} to {new:#x}")
                    },
                );
            }
            Status::RegisterChanged { register, old, new } => {
                status_dot(ui, theme.changed);
                ui.colored_label(
                    theme.changed,
                    if lang == Lang::Fr {
                        format!("Surveillance : {register} est passé de {old:#x} à {new:#x}")
                    } else {
                        format!("Watch: {register} went from {old:#x} to {new:#x}")
                    },
                );
            }
            Status::SteppedBack => {
                ui.colored_label(theme.accent, s.status_stepped_back);
            }
            Status::NothingToUndo => {
                ui.colored_label(theme.dim, s.status_nothing_to_undo);
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
            Status::Note(text) => {
                ui.colored_label(theme.good, *text);
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
        // La barre de recherche, au-dessus du code : elle pousse
        // l'éditeur d'une ligne plutôt que de flotter par-dessus, pour
        // ne jamais couvrir ce qu'on vient de trouver.
        self.find_ui(ui);
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
        // Mesurées avant d'entrer dans la zone de défilement : ce sont
        // les dimensions *visibles*, celles que le contenu doit remplir
        // pour qu'aucune barre n'ait lieu d'être.
        let visible = ui.available_size();
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        egui::Frame::new().fill(editor_bg).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt("editeur_scroll")
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
                        // Combien de fois chaque ligne a tourné. C'est le
                        // profil du programme : la boucle qu'on croyait
                        // écrire a-t-elle tourné cinq fois, ou une seule ?
                        // La branche qu'on croyait prendre a-t-elle
                        // seulement été atteinte ? Rien d'autre à l'écran
                        // ne répond à ça.
                        let passages = self.line_visits();
                        // La colonne ne prend de la place que lorsqu'il y
                        // a quelque chose à y mettre.
                        let compteur = if passages.is_empty() { 0.0 } else { 44.0 };
                        let (gutter_rect, gutter_resp) = ui.allocate_exact_size(
                            egui::vec2(GUTTER_WIDTH + compteur, galley.rect.height().max(1.0)),
                            egui::Sense::click(),
                        );
                        let click_y = gutter_resp.interact_pointer_pos().map(|p| p.y);
                        let mut clicked_line = None;
                        // Les lignes dont nasm s'est plaint : la console
                        // les liste, mais c'est ici qu'on écrit, et une
                        // erreur qu'on ne voit pas là où l'on tape ne
                        // sert qu'à moitié.
                        let fautives: BTreeMap<u32, Severity> = self
                            .diagnostics
                            .iter()
                            .filter(|d| d.line > 0)
                            .map(|d| (d.line, d.severity))
                            .collect();
                        let largeur_ligne = gutter_rect.width() + ui.available_width();
                        let painter = ui.painter();

                        // Les occurrences de la recherche, peintes sous
                        // le texte : une barre qui compte « 3/7 » sans
                        // montrer où ils sont ne sert qu'à moitié.
                        if let Some(find) = &self.find {
                            let matches = find_matches(&self.source, &find.query);
                            for (index, at) in matches.iter().enumerate() {
                                let debut = byte_to_char(&self.source, *at);
                                let fin = debut + find.query.chars().count();
                                let gauche =
                                    galley.pos_from_cursor(egui::text::CCursor::new(debut));
                                let droite = galley.pos_from_cursor(egui::text::CCursor::new(fin));
                                // Une occupation à cheval sur deux lignes
                                // n'existe pas ici : une recherche ne
                                // contient pas de retour à la ligne.
                                let rect = egui::Rect::from_min_max(gauche.min, droite.max)
                                    .translate(
                                        gutter_rect.min.to_vec2()
                                            + egui::vec2(gutter_rect.width(), 0.0),
                                    );
                                let teinte = if index == find.current {
                                    theme.accent.gamma_multiply(0.45)
                                } else {
                                    theme.accent.gamma_multiply(0.18)
                                };
                                painter.rect_filled(rect, 2.0, teinte);
                            }
                        }
                        // Quinze mille lignes dans un binaire ouvert :
                        // peindre un numéro pour chacune coûte une frame
                        // entière, alors que trente sont à l'écran. On
                        // s'en tient à ce que la zone montre.
                        let visible_rect = ui.clip_rect();
                        for (i, row) in galley.rows.iter().enumerate() {
                            let line = (i + 1) as u32;
                            let row_rect = row.rect().translate(gutter_rect.min.to_vec2());
                            if !row_rect.intersects(visible_rect) {
                                continue;
                            }

                            if gutter_resp.clicked()
                                && let Some(y) = click_y
                                && row_rect.y_range().contains(y)
                            {
                                clicked_line = Some(line);
                            }

                            // Le bandeau d'abord, sous tout le reste.
                            if let Some(severity) = fautives.get(&line) {
                                let teinte = if *severity == Severity::Error {
                                    theme.bad
                                } else {
                                    theme.warn
                                };
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        row_rect.min,
                                        egui::vec2(largeur_ligne, row_rect.height()),
                                    ),
                                    0.0,
                                    // Assez pour se voir, assez peu pour
                                    // laisser lire le code par-dessus.
                                    teinte.gamma_multiply(0.18),
                                );
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
                            // Le compteur de passages, à gauche du numéro
                            // et plus discret que lui : c'est une mesure,
                            // pas une adresse.
                            if let Some(count) = passages.get(&line) {
                                painter.text(
                                    egui::pos2(gutter_rect.left() + 30.0, row_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    format!("×{count}"),
                                    font_id.clone(),
                                    theme.dim,
                                );
                            }
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
                        // La largeur demandée est celle du code, ou celle
                        // du panneau s'il est plus large — et non l'infini.
                        // Demander l'infini faisait croire à la zone de
                        // défilement qu'il y avait toujours quelque chose à
                        // atteindre à droite : sa barre horizontale restait
                        // affichée même sur trois lignes de vingt
                        // caractères.
                        let width = editor_width(galley.rect.width(), ui.available_width());
                        let output = egui::TextEdit::multiline(&mut self.source)
                            .id(editor_id())
                            .frame(egui::Frame::NONE)
                            .desired_width(width)
                            .desired_rows(editor_rows(visible.y, row_height))
                            .code_editor()
                            .interactive(self.editor_mode == EditorMode::Insert)
                            .layouter(&mut layouter)
                            .show(ui);
                        if let Some(range) = output.cursor_range {
                            self.cursor_pos = line_col(&self.source, range.primary.index.0);
                            self.cursor_byte = char_to_byte(&self.source, range.primary.index.0);
                            // Entrée garde l'indentation de la ligne
                            // qu'on quitte. egui insère un `\n` nu ; on
                            // ajoute les blancs derrière, et l'on avance
                            // le curseur d'autant.
                            if self.auto_indent
                                && output.response.response.has_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                && range.is_empty()
                            {
                                let at = self.cursor_byte;
                                // Le `\n` vient d'être posé : l'indentation
                                // se lit sur la ligne d'avant.
                                let indent = newline_indent(
                                    &self.source[..at.saturating_sub(1)],
                                    at.saturating_sub(1),
                                );
                                let blancs = indent.trim_start_matches('\n');
                                if !blancs.is_empty() && at <= self.source.len() {
                                    self.source.insert_str(at, blancs);
                                    let cible = byte_to_char(&self.source, at + blancs.len());
                                    if let Some(mut state) =
                                        egui::TextEdit::load_state(ui.ctx(), editor_id())
                                    {
                                        state.cursor.set_char_range(Some(
                                            egui::text::CCursorRange::one(
                                                egui::text::CCursor::new(cible),
                                            ),
                                        ));
                                        state.store(ui.ctx(), editor_id());
                                    }
                                }
                            }
                        }
                        self.completion_ui(ui, &output);
                        self.hover_help_ui(ui, &output);
                        // Après le `TextEdit` : son état existe, et c'est
                        // lui qu'on déplace.
                        self.apply_goto_line(&ui.ctx().clone());
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

    /// La barre de recherche : ce qu'on cherche, combien de fois on le
    /// trouve, et de quoi passer d'une occurrence à l'autre.
    fn find_ui(&mut self, ui: &mut egui::Ui) {
        let Some(find) = &mut self.find else {
            return;
        };
        let s = strings(self.lang);
        let theme = self.theme;
        let mut query = std::mem::take(&mut find.query);
        let mut current = find.current;
        let just_opened = std::mem::take(&mut find.just_opened);
        let mut fermer = false;
        let mut aller = just_opened && !query.is_empty();

        let matches = find_matches(&self.source, &query);
        if matches.is_empty() {
            current = 0;
        } else {
            current %= matches.len();
        }

        ui.horizontal(|ui| {
            let champ = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .id(find_field_id())
                    .hint_text(s.find_placeholder)
                    .desired_width(220.0),
            );
            if just_opened {
                champ.request_focus();
            }
            // Entrée avance, Maj+Entrée recule : les deux gestes d'une
            // barre de recherche, sans quitter le clavier.
            if champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let arrière = ui.input(|i| i.modifiers.shift);
                if !matches.is_empty() {
                    current = if arrière {
                        (current + matches.len() - 1) % matches.len()
                    } else {
                        (current + 1) % matches.len()
                    };
                }
                aller = true;
                champ.request_focus();
            }
            if ui.button("<").on_hover_text(s.find_previous).clicked() && !matches.is_empty() {
                current = (current + matches.len() - 1) % matches.len();
                aller = true;
            }
            if ui.button(">").on_hover_text(s.find_next).clicked() && !matches.is_empty() {
                current = (current + 1) % matches.len();
                aller = true;
            }
            if matches.is_empty() {
                ui.colored_label(
                    if query.is_empty() {
                        theme.dim
                    } else {
                        theme.bad
                    },
                    if query.is_empty() {
                        String::new()
                    } else {
                        s.find_none.to_owned()
                    },
                );
            } else {
                ui.colored_label(theme.dim, format!("{}/{}", current + 1, matches.len()));
            }
            if ui.button("x").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                fermer = true;
            }
        });

        if fermer {
            self.find = None;
            return;
        }
        if let Some(find) = &mut self.find {
            find.query = query;
            find.current = current;
        }
        if aller {
            self.goto_match();
        }
    }

    /// L'autocomplétion : ce qu'on pourrait écrire là, et `Alt` pour
    /// l'accepter.
    ///
    /// Une liste flottante sous le curseur plutôt qu'un texte fantôme
    /// dans le champ : le tampon reste exactement ce que l'utilisateur a
    /// tapé, et rien de ce qu'il voit n'est écrit avant qu'il l'accepte.
    fn completion_ui(&mut self, ui: &mut egui::Ui, output: &egui::text_edit::TextEditOutput) {
        let s = strings(self.lang);
        let theme = self.theme;
        // Ni en mode Normal (on ne tape pas), ni si le réglage l'a
        // éteinte.
        // Relevé avant tout retour anticipé : gardée enfoncée pendant
        // qu'aucune liste n'est affichée, `Alt` serait lue comme une
        // nouvelle frappe au moment où la liste revient.
        //
        // Et `Alt` *seule* : les raccourcis d'exécution sont `Ctrl+Alt+I`
        // et `Ctrl+Alt+O`, qui accepteraient sinon la proposition au
        // passage, en plus de faire leur travail.
        let alt_alone = ui.input(|i| {
            let m = i.modifiers;
            m.alt && !m.ctrl && !m.shift && !m.command && !m.mac_cmd
        });
        let alt_pressed = alt_alone && !self.alt_was_down;
        self.alt_was_down = alt_alone;

        let Some(range) = output.cursor_range else {
            self.suggestions = None;
            return;
        };
        if !self.completion || self.editor_mode != EditorMode::Insert {
            self.suggestions = None;
            return;
        }

        let cursor = range.primary.index.0;
        let byte = char_to_byte(&self.source, cursor);
        self.suggestions = completion::suggest(&self.source, byte);
        let Some(suggestions) = self.suggestions.clone() else {
            return;
        };

        // `Alt` accepte : elle ne sert à rien d'autre dans l'éditeur, là
        // où Tab et Entrée ont déjà un sens.
        if alt_pressed {
            self.accept_completion(ui.ctx(), &suggestions);
            return;
        }

        // Sous le curseur, comme une bulle : la liste suit la frappe.
        let cursor_rect = output.galley.pos_from_cursor(range.primary);
        let at = output.galley_pos + cursor_rect.left_bottom().to_vec2();
        egui::Area::new(egui::Id::new("completion"))
            .order(egui::Order::Foreground)
            .fixed_pos(at + egui::vec2(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for (index, candidate) in suggestions.candidates.iter().enumerate() {
                        let colour = candidate.kind.colour(theme);
                        let text = egui::RichText::new(&candidate.text).monospace();
                        // Le premier est celui qu'`Alt` prendra : il est
                        // le seul en couleur pleine.
                        if index == 0 {
                            ui.horizontal(|ui| {
                                ui.colored_label(colour, text.strong());
                                ui.colored_label(theme.dim, s.completion_accept);
                            });
                        } else {
                            ui.colored_label(theme.dim, text);
                        }
                    }
                });
            });
    }

    /// Écrit la première proposition à la place du mot commencé, et
    /// replace le curseur derrière.
    fn accept_completion(&mut self, ctx: &egui::Context, suggestions: &completion::Suggestions) {
        let Some(accepted) = suggestions.accepted() else {
            return;
        };
        // Le mot commencé est *remplacé*, et non complété : le candidat
        // porte sa propre casse, et une étiquette écrite `Boucle` doit
        // s'écrire `Boucle`.
        let range = suggestions.range();
        let end = range.start + accepted.len();
        self.source.replace_range(range, accepted);

        // Le curseur d'egui compte en caractères, pas en octets.
        let after = byte_to_char(&self.source, end);
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(after),
                )));
            state.store(ctx, editor_id());
        }
        self.suggestions = None;
    }

    /// La fiche de l'instruction survolée, en infobulle.
    ///
    /// F1 donne la fiche entière ; ceci répond à la question plus petite
    /// qu'on se pose en lisant — « celle-là, c'est quoi ? » — sans
    /// quitter la ligne des yeux ni presser quoi que ce soit. Soumis au
    /// réglage des infobulles : ceux qui les trouvent envahissantes n'en
    /// veulent pas ici non plus.
    fn hover_help_ui(&self, ui: &mut egui::Ui, output: &egui::text_edit::TextEditOutput) {
        if !self.show_tooltips || !self.hover_help {
            return;
        }
        let Some(pointer) = ui.ctx().pointer_hover_pos() else {
            return;
        };
        if !output.response.response.hovered() {
            return;
        }
        // Le champ de texte occupe toute la hauteur de l'éditeur, bien
        // au-delà du code, et chaque ligne est bien plus courte que sa
        // largeur : sans cette vérification, le pointeur posé dans le
        // vide se voyait attribuer le caractère le plus proche — le
        // dernier du fichier — et la fiche de cette instruction-là
        // restait affichée sans qu'on comprenne d'où elle venait.
        let relatif = pointer - output.galley_pos;
        let Some(ligne) = output
            .galley
            .rows
            .iter()
            .find(|ligne| ligne.rect().y_range().contains(relatif.y))
        else {
            return;
        };
        // Un demi-caractère de tolérance au bout de la ligne : viser une
        // lettre, pas son coin.
        let marge = ligne.rect().height() / 2.0;
        if relatif.x < ligne.rect().left() - marge || relatif.x > ligne.rect().right() + marge {
            return;
        }
        let cursor = output.galley.cursor_from_pos(relatif);
        // La position du pointeur dans le texte, puis le mot qui s'y
        // trouve : le même que celui sur lequel F1 répondrait.
        let byte = char_to_byte(&self.source, cursor.index.0);
        let word = word_at(&self.source, byte);
        let Some(entry) = namsdec_doc::lookup(word) else {
            return;
        };
        let lang = self.lang.code();
        let theme = self.theme;
        output.response.response.clone().show_tooltip_ui(|ui| {
            ui.set_max_width(420.0);
            ui.strong(entry.title.get(lang));
            ui.label(prose(ui, theme, entry.body.get(lang)));
            if !entry.flags.is_empty() {
                ui.colored_label(
                    theme.dim,
                    format!("{} {}", strings(self.lang).help_flags, entry.flags),
                );
            }
        });
    }

    /// Pose le curseur au début d'une ligne, et y amène la vue.
    ///
    /// Appelé au rendu de l'éditeur, une fois seulement : le curseur
    /// appartient à l'utilisateur, on ne le lui reprend pas à chaque
    /// frame.
    fn apply_goto_line(&mut self, ctx: &egui::Context) {
        let Some(line) = self.goto_line.take() else {
            return;
        };
        let offset: usize = self
            .source
            .lines()
            .take(line.saturating_sub(1) as usize)
            .map(|l| l.chars().count() + 1)
            .sum();
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(offset),
                )));
            state.store(ctx, editor_id());
        }
        self.editor_mode = EditorMode::Insert;
        ctx.memory_mut(|m| m.request_focus(editor_id()));
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

    /// Ouvre la barre de recherche, ou la referme si elle est déjà là.
    ///
    /// Le mot sous le curseur devient la recherche : neuf fois sur dix
    /// c'est ce qu'on allait taper, et c'est ce que font les éditeurs
    /// où l'on a pris l'habitude.
    fn do_find(&mut self) {
        if self.find.is_some() {
            self.find = None;
            return;
        }
        let word = word_at(&self.source, self.cursor_byte).to_owned();
        self.find = Some(Find {
            query: word,
            current: 0,
            just_opened: true,
        });
    }

    /// Va à l'occurrence `current` : la ligne sous les yeux, le curseur
    /// dessus.
    fn goto_match(&mut self) {
        let Some(find) = &self.find else { return };
        let matches = find_matches(&self.source, &find.query);
        let Some(at) = matches.get(find.current) else {
            return;
        };
        let line = self.source[..*at].lines().count().max(1) as u32;
        self.goto_line = Some(line);
    }

    /// Va à l'endroit où l'étiquette sous le curseur est définie.
    ///
    /// Sur un programme d'une centaine de lignes, `jne boucle` et
    /// `boucle:` ne tiennent plus dans le même écran, et retrouver l'un
    /// depuis l'autre se fait à la molette. C'est le « aller à la
    /// définition » de n'importe quel éditeur de code, appliqué à la
    /// seule chose qui en a une ici : une étiquette.
    fn do_goto_label(&mut self) {
        let word = word_at(&self.source, self.cursor_byte);
        if word.is_empty() {
            return;
        }
        match label_line(&self.source, word) {
            Some(line) => {
                self.goto_line = Some(line);
                self.status = Status::Idle;
            }
            // Le dire plutôt que de ne rien faire : une touche qui ne
            // répond pas laisse croire qu'elle n'existe pas.
            None => self.status = Status::Note(strings(self.lang).goto_label_unknown),
        }
    }

    /// Cherche la fiche du mot sous le curseur et l'ouvre.
    ///
    /// Sous le curseur, et non « le premier mot de la ligne » : on demande
    /// aussi bien ce que fait `movzx` que ce que fait le `rep` qui le
    /// précède, et c'est là qu'on a mis le curseur.
    fn do_instruction_help(&mut self) {
        let word = word_at(&self.source, self.cursor_byte).to_owned();
        if word.is_empty() {
            return;
        }
        self.open_help(&word);
        self.show_help = true;
    }

    /// Ouvre une fiche en gardant celle qu'on quitte.
    ///
    /// Suivre un renvoi ne doit pas coûter le chemin parcouru : on
    /// apprend en tirant sur un fil, et on revient au point de départ.
    fn open_help(&mut self, word: &str) {
        if let Some((courant, _)) = &self.help
            && courant != word
        {
            self.help_back.push(courant.clone());
            // Une lecture, pas un journal : au-delà, le début ne sert
            // plus à rien.
            if self.help_back.len() > 32 {
                self.help_back.remove(0);
            }
        }
        self.help = Some((word.to_owned(), namsdec_doc::lookup(word)));
    }

    /// Le désassemblage du programme, en texte, prêt à être collé.
    ///
    /// Les étiquettes aux endroits où elles tombent, l'adresse, les
    /// octets et l'instruction : ce qu'on recopie à la main dans un
    /// compte rendu quand l'outil ne sait pas le donner.
    fn listing_text(&self) -> Option<String> {
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
    fn analysis_report(&self) -> Option<String> {
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
            self.lang.code(),
            &name,
        ))
    }

    /// La question qui protège un travail non enregistré.
    ///
    /// Trois réponses, parce qu'il y en a trois : enregistrer et
    /// continuer, continuer sans enregistrer, ou ne rien faire du tout.
    /// Une boîte à deux boutons oblige à choisir entre perdre son
    /// travail et renoncer à ce qu'on voulait faire.
    fn unsaved_window_ui(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending.clone() else {
            return;
        };
        let s = strings(self.lang);
        let theme = self.theme;
        let mut décision: Option<bool> = None;
        let mut annuler = false;
        egui::Window::new(s.unsaved_title)
            .id(egui::Id::new("unsaved_changes"))
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.set_max_width(380.0);
                ui.label(s.unsaved_question);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let save =
                        egui::Button::new(egui::RichText::new(s.unsaved_save).color(theme.bg))
                            .fill(theme.accent);
                    if ui.add(save).clicked() {
                        décision = Some(true);
                    }
                    if ui
                        .add(
                            egui::Button::new(s.unsaved_discard)
                                .stroke(egui::Stroke::new(1.0, theme.bad)),
                        )
                        .clicked()
                    {
                        décision = Some(false);
                    }
                    if ui.button(s.unsaved_cancel).clicked() {
                        annuler = true;
                    }
                });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            annuler = true;
        }
        if annuler {
            self.pending = None;
            return;
        }
        let Some(enregistrer) = décision else {
            return;
        };
        if enregistrer {
            self.do_save();
            // Un enregistrement qui échoue — disque plein, fichier en
            // lecture seule — ne doit pas emporter le tampon avec lui.
            if self.buffer_modified() {
                return;
            }
        }
        self.pending = None;
        self.perform(pending);
    }

    /// Intercepte la fermeture de la fenêtre tant qu'il reste quelque
    /// chose à enregistrer.
    fn guard_close(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if self.quitting || !self.buffer_modified() {
            return;
        }
        // On annule la fermeture et l'on pose la question : le
        // gestionnaire de fenêtres la reposera quand on aura répondu.
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.pending = Some(Pending::Quit);
    }

    /// Le panneau de désassemblage : le programme, lu par le moteur
    /// qu'on a choisi.
    ///
    /// Deux moteurs sont livrés — le pseudo-code, qui dit l'instruction
    /// en clair, et la syntaxe NASM, celle qu'on écrit. D'autres
    /// s'ajoutent en posant un fichier de description dans le dossier
    /// des plugins : namsdec appelle alors l'outil et montre ce qu'il
    /// répond.
    fn disassembly_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_disassembly {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let mut open = true;
        let mut aller: Option<u32> = None;
        let mut vise: Option<u64> = None;
        let mut revenir_au_début = false;
        // Relevée avant la fenêtre : elle emprunte `self` en écriture.
        let addr_to_line = self
            .assemble_state
            .as_ref()
            .map(|state| state.addr_to_line.clone())
            .unwrap_or_default();

        egui::Window::new(s.disassembly_title)
            .id(egui::Id::new("disassembly_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([680.0, 560.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                // Le choix du moteur, en tête : c'est lui qui décide de
                // tout ce qu'on lit en dessous.
                ui.horizontal_wrapped(|ui| {
                    ui.label(s.disassembly_engine);
                    ui.selectable_value(
                        &mut self.disassembler,
                        PSEUDO_ID.to_owned(),
                        s.disassembly_pseudo,
                    );
                    ui.selectable_value(
                        &mut self.disassembler,
                        NASM_ID.to_owned(),
                        s.disassembly_nasm,
                    );
                    for plugin in &self.disassembler_plugins {
                        ui.selectable_value(
                            &mut self.disassembler,
                            plugin.id.clone(),
                            &plugin.name,
                        )
                        .on_hover_text(format!("{} {}", s.disassembly_external, plugin.command));
                    }
                });
                ui.weak(if self.disassembler == PSEUDO_ID {
                    s.disassembly_pseudo_hint
                } else {
                    s.disassembly_nasm_hint
                });
                ui.separator();

                let Some(state) = &self.assemble_state else {
                    ui.weak(s.no_binary_assembled);
                    return;
                };
                if state.disasm.is_empty() {
                    ui.weak(s.no_binary_assembled);
                    return;
                }

                // Un moteur extérieur : on l'appelle sur demande, et
                // l'on garde sa réponse. Le relancer à chaque frame
                // reviendrait à lancer un programme soixante fois par
                // seconde.
                if let Some(plugin) = self
                    .disassembler_plugins
                    .iter()
                    .find(|plugin| plugin.id == self.disassembler)
                {
                    let déjà = self
                        .plugin_output
                        .as_ref()
                        .filter(|(id, _)| *id == plugin.id)
                        .map(|(_, texte)| texte.clone());
                    match déjà {
                        Some(texte) => {
                            egui::ScrollArea::both()
                                .id_salt("plugin_output")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(texte).monospace())
                                            .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                });
                        }
                        None => {
                            ui.weak(format!("{} {}", s.disassembly_will_run, plugin.command));
                            if ui.button(s.disassembly_run).clicked() {
                                let octets = state.output.file.clone();
                                let octets = if octets.is_empty() {
                                    state.output.bytes.clone()
                                } else {
                                    octets
                                };
                                let texte = plugin.run(&octets).unwrap_or_else(|e| format!("{e}"));
                                self.plugin_output = Some((plugin.id.clone(), texte));
                            }
                        }
                    }
                    return;
                }
                let rip = self.emulator.as_ref().map(|e| e.cpu().regs.rip);
                let symbols = &state.output.symbols;
                let pseudo = self.disassembler == PSEUDO_ID;

                // Ce qu'on cherche : une étiquette, un registre, une
                // adresse. Le listing d'un binaire fait quinze mille
                // lignes, et l'on sait ce qu'on y cherche.
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.disassembly_query)
                            .hint_text(s.find_placeholder)
                            .desired_width(200.0),
                    );
                    if !self.disassembly_query.is_empty() && ui.button("x").clicked() {
                        self.disassembly_query.clear();
                    }
                });
                let query = self.disassembly_query.to_lowercase();

                // Où aller : les étiquettes du programme, dans l'ordre
                // des adresses. Sur un binaire, c'est par là qu'on entre
                // — et faire défiler quinze mille lignes pour trouver
                // une fonction n'est pas une façon de lire.
                if !symbols.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(s.disassembly_goto_label);
                        egui::ComboBox::from_id_salt("disassembly_symbols")
                            .selected_text(s.disassembly_choose)
                            .show_ui(ui, |ui| {
                                for symbol in symbols {
                                    if ui.selectable_label(false, &symbol.name).clicked() {
                                        vise = Some(symbol.addr);
                                    }
                                }
                            });
                    });
                }

                // Le texte de chaque ligne est calculé ici, pour que la
                // recherche porte sur ce qu'on lit et non sur la forme
                // d'un autre moteur.
                let lignes: Vec<Ligne> = state
                    .disasm
                    .iter()
                    // Demandé d'aller quelque part : la liste commence
                    // là. Un défilement calculé dépend de la hauteur
                    // exacte d'une rangée et de ce que la fenêtre laisse
                    // voir — deux grandeurs qu'on ne connaît qu'après
                    // avoir dessiné, et qui font tomber à côté.
                    .skip_while(|instruction| {
                        self.disassembly_target
                            .is_some_and(|addr| instruction.address < addr)
                    })
                    .map(|instruction| Ligne {
                        address: instruction.address,
                        octets: instruction
                            .bytes
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<Vec<_>>()
                            .join(" "),
                        texte: namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
                            .map(|decoded| {
                                if pseudo {
                                    namsdec_disasm::pseudo::of(&decoded, lang)
                                } else {
                                    namsdec_disasm::format_one(&decoded)
                                }
                            })
                            .unwrap_or_else(|| instruction.text.clone()),
                    })
                    .filter(|ligne| {
                        query.is_empty()
                            || ligne.texte.to_lowercase().contains(&query)
                            || format!("{:08x}", ligne.address).contains(&query)
                            || symbols.iter().any(|symbol| {
                                symbol.addr == ligne.address
                                    && symbol.name.to_lowercase().contains(&query)
                            })
                    })
                    .collect();
                if !query.is_empty() {
                    ui.weak(format!("{} / {}", lignes.len(), state.disasm.len()));
                }
                if let Some(addr) = self.disassembly_target {
                    ui.horizontal(|ui| {
                        let nom = symbols
                            .iter()
                            .find(|s| s.addr == addr)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| format!("{addr:#x}"));
                        ui.colored_label(theme.accent, format!("{} {nom}", s.disassembly_from));
                        if ui.button(s.disassembly_all).clicked() {
                            revenir_au_début = true;
                        }
                    });
                }

                let row_height =
                    ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y;
                // Suivre l'exécution, comme l'onglet Octets — sauf
                // pendant une recherche, où c'est elle qui commande.
                let suivie =
                    (query.is_empty() && self.disassembly_target.is_none() && self.follow_rip)
                        .then(|| {
                            rip.and_then(|rip| lignes.iter().position(|ligne| ligne.address == rip))
                        })
                        .flatten();
                // Bornée à la place que la fenêtre laisse : une zone qui
                // se croit plus haute déborde, et ce qui dépasse est
                // coupé — y compris les lignes vers lesquelles on vient
                // de demander à aller.
                let mut zone = egui::ScrollArea::vertical()
                    .id_salt("disassembly_rows")
                    .max_height(ui.available_height().max(120.0))
                    .auto_shrink([false, false]);
                if let Some(index) = suivie {
                    // Trois lignes de contexte au-dessus, et non un
                    // centrage : la hauteur visible n'est pas connue
                    // avant que la zone existe, et un centrage calculé
                    // sur une hauteur approximative tombe à côté.
                    const CONTEXTE: usize = 3;
                    zone = zone
                        .vertical_scroll_offset(index.saturating_sub(CONTEXTE) as f32 * row_height);
                }
                zone.show_rows(ui, row_height, lignes.len(), |ui, rows| {
                    for ligne in &lignes[rows] {
                        let ici = rip == Some(ligne.address);
                        let rangée = ui
                            .horizontal(|ui| {
                                ui.colored_label(
                                    if ici { theme.accent } else { theme.dim },
                                    egui::RichText::new(format!("{:08x}", ligne.address))
                                        .monospace(),
                                );
                                // L'étiquette sur la rangée de son
                                // instruction, et non sur une ligne à
                                // elle : toutes les rangées gardent la
                                // même hauteur, ce dont la zone a besoin
                                // pour savoir où elle en est.
                                let étiquette = symbols
                                    .iter()
                                    .find(|s| s.addr == ligne.address)
                                    .map(|s| format!("{}:", s.name))
                                    .unwrap_or_default();
                                ui.colored_label(
                                    theme.label,
                                    egui::RichText::new(étiquette).monospace(),
                                );
                                // Les octets : ce qui distingue un
                                // désassemblage d'une traduction. Sur une
                                // largeur fixe, sinon les colonnes
                                // dansent d'une ligne à l'autre.
                                ui.colored_label(
                                    theme.dim,
                                    egui::RichText::new(format!("{:<23}", ligne.octets))
                                        .monospace(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&ligne.texte).monospace().color(
                                            if ici {
                                                theme.accent
                                            } else {
                                                ui.visuals().text_color()
                                            },
                                        ),
                                    )
                                    .truncate(),
                                );
                            })
                            .response;
                        // Un clic mène à la ligne source : le listing
                        // montre ce que le programme *est*, l'éditeur ce
                        // qu'on a écrit, et l'on passe de l'un à l'autre.
                        if rangée
                            .interact(egui::Sense::click())
                            .on_hover_text(s.disassembly_goto)
                            .clicked()
                        {
                            aller = addr_to_line.get(&ligne.address).copied();
                        }
                    }
                });
            });
        if let Some(ligne) = aller {
            self.goto_line = Some(ligne);
        }
        // Posée pour la frame suivante, puis oubliée : sans quoi la vue
        // resterait collée là et l'on ne pourrait plus la faire défiler.
        if let Some(addr) = vise {
            self.disassembly_target = Some(addr);
        }
        if revenir_au_début {
            self.disassembly_target = None;
        }
        self.show_disassembly = open;
    }

    /// La chronologie : le chemin parcouru, et de quoi y revenir.
    ///
    /// « Comment suis-je arrivé là ? » est la question qu'on pose devant
    /// un état qu'on n'attendait pas, et aucun compteur n'y répond : il
    /// faut l'ordre des pas. Revenir en arrière d'un pas à la fois y
    /// répond mal aussi — on cherche un endroit, pas un pas.
    ///
    /// Chaque ligne est un pas qu'on peut vraiment défaire : cliquer
    /// dessus rembobine le programme jusque-là, mémoire et registres
    /// compris.
    /// Le motif cyclique : « de combien ça déborde ».
    ///
    /// On remplit l'entrée d'une suite dont toute tranche de quatre
    /// octets est unique, on laisse le programme déborder, et la valeur
    /// qu'on retrouve dans RIP dit d'elle-même à quelle distance du
    /// début du tampon se trouvait l'adresse de retour. Le calcul est
    /// mécanique — c'est le `cyclic` de pwndbg, et il n'y a aucune
    /// raison de le faire à la main.
    fn cyclic_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_cyclic {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = true;
        // Relevés avant la fenêtre, qui emprunte `self` en écriture.
        let registres: Vec<(&'static str, u64)> = self
            .emulator
            .as_ref()
            .map(|emulator| {
                let regs = &emulator.cpu().regs;
                vec![
                    ("RIP", regs.rip),
                    ("RSP", regs.get64(Reg64::Rsp)),
                    ("RAX", regs.get64(Reg64::Rax)),
                ]
            })
            .unwrap_or_default();
        let mut vers_entrée = None;

        egui::Window::new(s.cyclic_title)
            .open(&mut open)
            .resizable(true)
            .default_size([560.0, 400.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                ui.label(prose(ui, theme, s.cyclic_explain));
                ui.separator();

                // Engendrer.
                ui.horizontal(|ui| {
                    ui.colored_label(theme.dim, s.cyclic_length);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyclic_len)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(60.0),
                    );
                });
                // Une longueur qui ne se lit pas ne fait pas disparaître
                // le motif : elle retombe sur cent.
                let combien = self
                    .cyclic_len
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(100)
                    .clamp(1, 4096);
                let motif =
                    String::from_utf8_lossy(&namsdec_re::cyclic::pattern(combien)).into_owned();
                ui.add(
                    egui::Label::new(egui::RichText::new(&motif).monospace())
                        .wrap_mode(egui::TextWrapMode::Wrap),
                );
                ui.horizontal_wrapped(|ui| {
                    if ui.button(s.cyclic_copy).clicked() {
                        ui.ctx().copy_text(motif.clone());
                        self.status = Status::Note(s.cyclic_copied);
                    }
                    // Le motif sert à être *mangé* par le programme :
                    // l'envoyer dans l'entrée évite le détour par le
                    // presse-papier.
                    if ui
                        .button(s.cyclic_to_input)
                        .on_hover_text(s.cyclic_to_input_hint)
                        .clicked()
                    {
                        vers_entrée = Some(motif.clone());
                    }
                });

                ui.separator();
                // Retrouver.
                ui.colored_label(theme.dim, s.cyclic_find_label);
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyclic_find)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(180.0)
                            .hint_text(s.cyclic_find_hint),
                    );
                    // Les registres où l'on retrouve le motif après un
                    // débordement : il n'y a pas à les recopier.
                    for (name, value) in &registres {
                        if ui
                            .small_button(*name)
                            .on_hover_text(format!("{value:#x}"))
                            .clicked()
                        {
                            self.cyclic_find = format!("{value:#x}");
                        }
                    }
                });
                match cyclic_offset(&self.cyclic_find) {
                    Some(offset) => {
                        ui.colored_label(
                            theme.good,
                            format!("{} {offset} ({offset:#x})", s.cyclic_offset),
                        );
                        ui.colored_label(theme.dim, s.cyclic_offset_hint);
                    }
                    None if self.cyclic_find.trim().is_empty() => {}
                    None => {
                        ui.colored_label(theme.warn, s.cyclic_not_found);
                    }
                }
            });

        if let Some(motif) = vers_entrée {
            self.console_input = motif;
            self.console_tab = ConsoleTab::Output;
            self.show_console = true;
            self.status = Status::Note(s.cyclic_sent);
        }
        self.show_cyclic = open;
    }

    fn timeline_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_timeline {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = true;
        let mut revenir = None;
        let mut aller = None;

        // Relevé avant la fenêtre : elle emprunte `self` en écriture
        // pour poser le retour en arrière.
        // Le rang est celui du chemin entier, même quand on n'en montre
        // qu'une partie : « revenir à ce pas » doit rembobiner du bon
        // nombre de pas, filtre ou pas filtre.
        let pas: Vec<(usize, namsdec_emu::Step)> = self
            .emulator
            .as_ref()
            .map(|emulator| {
                emulator
                    .trace()
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, pas)| {
                        !self.timeline_notable_only || pas.kind != namsdec_emu::StepKind::Plain
                    })
                    .collect()
            })
            .unwrap_or_default();

        egui::Window::new(s.timeline_title)
            .id(egui::Id::new("timeline_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([520.0, 420.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                if pas.is_empty() {
                    ui.weak(if self.timeline_notable_only {
                        s.timeline_nothing_notable
                    } else {
                        s.timeline_empty
                    });
                    return;
                }
                ui.weak(s.timeline_hint);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.timeline_notable_only, s.timeline_notable);
                });
                ui.separator();
                let row_height =
                    ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y;
                // Le nombre de pas du chemin, et non celui des lignes
                // montrées : c'est de lui que se compte le recul.
                let total = self
                    .emulator
                    .as_ref()
                    .map(|emulator| emulator.trace().len())
                    .unwrap_or(0);
                // Cinq mille pas : on ne dessine que ce qui est à
                // l'écran, comme pour l'onglet Octets.
                // Collée au bas : le dernier pas est celui qu'on regarde
                // entre deux avancées, et le chercher à chaque fois
                // reviendrait à faire défiler cinq mille lignes.
                egui::ScrollArea::vertical()
                    .id_salt("timeline_rows")
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show_rows(ui, row_height, pas.len(), |ui, rows| {
                        for (index, pas) in &pas[rows] {
                            let addr = &pas.addr;
                            // Le rang par rapport à maintenant : le
                            // dernier pas est « -1 », celui d'avant
                            // « -2 ». C'est ainsi qu'on le pense.
                            let recul = total - index;
                            let ligne = self
                                .assemble_state
                                .as_ref()
                                .and_then(|state| state.addr_to_line.get(addr).copied());
                            let texte = self
                                .assemble_state
                                .as_ref()
                                .and_then(|state| {
                                    state
                                        .disasm
                                        .iter()
                                        .find(|instruction| instruction.address == *addr)
                                })
                                .map(|instruction| instruction.text.clone())
                                .unwrap_or_default();
                            let réponse = ui
                                .horizontal(|ui| {
                                    ui.colored_label(theme.dim, format!("-{recul}"));
                                    // Ce que le pas a fait, en une
                                    // marque : c'est ce qu'on cherche des
                                    // yeux en parcourant un chemin.
                                    let (marque, couleur, quoi) = step_mark(s, pas.kind, theme);
                                    ui.colored_label(couleur, marque).on_hover_text(quoi);
                                    ui.monospace(format!("{addr:#010x}"));
                                    match ligne {
                                        Some(ligne) => {
                                            ui.colored_label(theme.accent, format!("{ligne}"))
                                        }
                                        None => ui.colored_label(theme.dim, " "),
                                    };
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(texte).monospace())
                                            .truncate(),
                                    );
                                })
                                .response;
                            let réponse = réponse
                                .interact(egui::Sense::click())
                                .on_hover_text(s.timeline_go_back);
                            if réponse.clicked() {
                                revenir = Some(recul);
                            }
                            // Le clic droit mène à la ligne dans
                            // l'éditeur, sans toucher à l'exécution :
                            // lire le code d'un pas et rembobiner
                            // jusqu'à lui sont deux gestes différents.
                            if réponse.secondary_clicked() {
                                aller = ligne;
                            }
                        }
                    });
            });

        if let Some(recul) = revenir {
            self.rewind(recul);
        }
        if let Some(ligne) = aller {
            self.goto_line = Some(ligne);
        }
        self.show_timeline = open;
    }

    /// Rembobine de `pas` pas : autant de retours en arrière, d'un coup.
    ///
    /// Le moteur ne sait défaire qu'un pas à la fois — c'est ce qui rend
    /// chaque retour exact, mémoire comprise. Les enchaîner ici évite de
    /// demander à l'utilisateur de cliquer mille fois pour reculer de
    /// mille.
    fn rewind(&mut self, pas: usize) {
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        let mut faits = 0;
        for _ in 0..pas {
            if !emulator.step_back() {
                break;
            }
            faits += 1;
        }
        if faits == 0 {
            return;
        }
        self.running = false;
        self.regs_before = None;
        self.flags_before = None;
        self.last_step = None;
        self.error_advice = None;
        self.follow_execution();
        self.status = Status::SteppedBack;
    }

    /// Les points d'arrêt, avec leur condition.
    ///
    /// Un point d'arrêt se pose d'un clic dans la gouttière ; sa
    /// condition, elle, s'écrit — et il faut bien un endroit pour cela.
    /// La liste dit aussi lesquels sont posés, ce que la gouttière ne
    /// montre que ligne par ligne.
    fn breakpoints_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_breakpoints {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = true;
        let mut retirer = None;
        egui::Window::new(s.breakpoints_title)
            .id(egui::Id::new("breakpoints_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([480.0, 320.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                if self.breakpoint_lines.is_empty() {
                    ui.weak(s.breakpoints_empty);
                    return;
                }
                ui.weak(s.breakpoints_hint);
                ui.separator();
                let lignes: Vec<u32> = self.breakpoint_lines.iter().copied().collect();
                for ligne in lignes {
                    ui.horizontal(|ui| {
                        ui.colored_label(theme.accent, format!("{} {ligne}", s.breakpoints_line));
                        let texte = self.breakpoint_conditions.entry(ligne).or_default();
                        let réponse = ui.add(
                            egui::TextEdit::singleline(texte)
                                .id(egui::Id::new(("bp_cond", ligne)))
                                .hint_text(s.breakpoints_condition_hint)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(180.0),
                        );
                        // Le refus se voit sur le champ : une condition
                        // qu'on ne sait pas lire ne filtre rien, et il
                        // vaut mieux le dire que de laisser croire.
                        if !texte.trim().is_empty() && condition::parse(texte).is_none() {
                            ui.painter().rect_stroke(
                                réponse.rect,
                                ui.visuals().widgets.inactive.corner_radius,
                                egui::Stroke::new(1.0, theme.bad),
                                egui::StrokeKind::Inside,
                            );
                            ui.colored_label(theme.bad, s.breakpoints_condition_bad);
                        }
                        if ui.button("x").on_hover_text(s.breakpoints_remove).clicked() {
                            retirer = Some(ligne);
                        }
                    });
                }
            });
        if let Some(ligne) = retirer {
            self.breakpoint_lines.remove(&ligne);
            self.breakpoint_conditions.remove(&ligne);
            self.sync_breakpoints();
        }
        self.show_breakpoints = open;
    }

    /// La fenêtre d'analyse : tous les modules de rétro-ingénierie, avec
    /// la place de s'exprimer.
    ///
    /// Les mêmes modules qu'à l'onglet RE, au même instant et sur le même
    /// binaire — mais l'onglet fait trois cent quatre-vingts pixels de
    /// large et cent cinquante de haut, ce qui suffit pour une liste de
    /// registres et pas pour une carte de segments.
    fn analysis_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_analysis {
            return;
        }
        let s = strings(self.lang);
        let mut open = true;
        egui::Window::new(s.analysis_title)
            // Un identifiant explicite : le titre change avec la langue,
            // et la fenêtre perdrait sa position en passant à l'anglais.
            .id(egui::Id::new("analysis_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // Six modules à la suite : la fenêtre part plus haute que
            // large, et se redimensionne.
            .default_size([620.0, 760.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.weak(s.analysis_hint);
                });
                // Ce qu'on a compris d'un binaire ne vaut que si on peut
                // le transmettre : le rapport part en Markdown, prêt à
                // coller dans un compte rendu ou un support de cours.
                ui.horizontal(|ui| {
                    if ui.button(s.analysis_copy).clicked()
                        && let Some(report) = self.analysis_report()
                    {
                        ui.ctx().copy_text(report);
                        self.status = Status::Note(s.analysis_copied);
                    }
                    // Et sur le disque : un compte rendu se garde, se
                    // relit, s'attache à un courriel. Le presse-papier
                    // ne survit pas à la fenêtre suivante.
                    if ui
                        .button(s.analysis_save)
                        .on_hover_text(s.analysis_save_hint)
                        .clicked()
                    {
                        self.spawn_file_dialog(FileDialogKind::ExportReport);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.re_modules_ui(ui));
            });
        self.show_analysis = open;
    }

    /// La fiche d'une instruction : ce qu'elle fait, ce qu'elle laisse
    /// dans les indicateurs, et à quoi elle ressemble écrite.
    ///
    /// Les indicateurs sont dits explicitement parce que c'est ce qu'on
    /// oublie : une instruction qui n'en touche aucun n'est pas la même
    /// qu'une qui les écrase tous, et le saut qui suit ne lira pas la
    /// même chose.
    fn help_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let mut open = self.show_help;
        egui::Window::new(s.help_title)
            // Un identifiant explicite : le titre change avec la langue,
            // et la fenêtre perdrait sa position en passant à l'anglais.
            .id(egui::Id::new("instruction_help"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([460.0, 320.0])
            // En bas à droite, loin du curseur : on lit la fiche en
            // continuant d'écrire, elle ne doit pas couvrir la ligne sur
            // laquelle on vient de demander de l'aide.
            .pivot(egui::Align2::RIGHT_BOTTOM)
            .default_pos(ctx.viewport_rect().max - egui::vec2(24.0, 24.0))
            .show(ctx, |ui| self.help_ui(ui));
        self.show_help = open;
    }

    /// Les registres étendus, dans une fenêtre à eux.
    ///
    /// Seize registres vectoriels de deux cent cinquante-six bits et huit
    /// emplacements x87 ne tiennent pas dans un panneau qui porte déjà
    /// dix-huit registres entiers : dans le panneau, ils poussaient la
    /// rangée d'onglets dehors, et bornés par une zone de défilement ils
    /// se lisaient par tranches de trois lignes. Ici ils tiennent en
    /// entier, et la fenêtre s'agrandit si l'on veut.
    fn vectors_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_vectors {
            return;
        }
        let s = strings(self.lang);
        let mut open = true;
        egui::Window::new(s.vectors_title)
            .id(egui::Id::new("vectors_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([520.0, 620.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .show(ctx, |ui| {
                if self.emulator.is_none() {
                    ui.label(s.no_binary_assembled);
                    return;
                }
                // Aucune zone de défilement : les vingt-quatre valeurs
                // se posent sur autant de colonnes que la fenêtre en
                // tient, et tiennent donc en entier. C'est toute la
                // raison d'être de cette fenêtre.
                self.xmm_ui(ui);
                ui.add_space(10.0);
                self.x87_ui(ui);
            });
        self.show_vectors = open;
    }

    fn help_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang_code = self.lang.code();

        // Un champ de recherche au-dessus de la fiche : sans lui, on ne
        // peut lire une fiche qu'en écrivant l'instruction dans son
        // code — ce qui suppose de savoir qu'elle existe. Le manuel se
        // consulte aussi pour ce qu'on ne connaît pas encore.
        let mut ouvrir = None;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.help_query)
                    .hint_text(s.help_search)
                    .desired_width(200.0),
            );
            if !self.help_query.is_empty() && ui.button("x").clicked() {
                self.help_query.clear();
            }
            // De quoi revenir sur ses pas, puis au sommaire : une fiche
            // ouverte ne doit pas être un cul-de-sac.
            if !self.help_back.is_empty()
                && ui
                    .button(s.help_back)
                    .on_hover_text(s.help_back_hint)
                    .clicked()
                && let Some(precedent) = self.help_back.pop()
            {
                self.help = Some((precedent.clone(), namsdec_doc::lookup(&precedent)));
                self.help_query.clear();
            }
            if self.help.is_some() && ui.button(s.help_index).clicked() {
                self.help = None;
                self.help_back.clear();
                self.help_query.clear();
            }
        });
        if !self.help_query.is_empty() {
            let query = self.help_query.to_lowercase();
            let trouvées: Vec<&namsdec_doc::Entry> = namsdec_doc::all()
                .iter()
                .filter(|entry| {
                    entry.names.iter().any(|name| name.contains(&query))
                        || entry.title.get(lang_code).to_lowercase().contains(&query)
                })
                .collect();
            ui.colored_label(theme.dim, format!("{} {}", trouvées.len(), s.help_results));
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("help_index")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in trouvées {
                        let ligne = ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                theme.mnemonic,
                                egui::RichText::new(entry.names[0]).monospace(),
                            );
                            ui.label(entry.title.get(lang_code));
                        });
                        if ligne
                            .response
                            .interact(egui::Sense::click())
                            .on_hover_text(s.help_open_sheet)
                            .clicked()
                        {
                            ouvrir = Some(entry.names[0]);
                        }
                    }
                });
            if let Some(name) = ouvrir {
                self.open_help(name);
                self.help_query.clear();
            }
            return;
        }

        let Some((word, entry)) = &self.help else {
            // Rien sous le curseur : on montre le manuel lui-même. Une
            // documentation qu'on ne peut que *chercher* suppose qu'on
            // sache déjà ce qu'on cherche ; un sommaire se parcourt.
            ui.colored_label(theme.dim, s.help_empty);
            ui.colored_label(theme.dim, s.help_search_hint);
            self.help_index_ui(ui);
            return;
        };
        let Some(entry) = entry else {
            // Pas d'instruction de ce nom — mais dans le listing d'un
            // binaire, le mot sous le curseur est souvent une fonction
            // empruntée. F1 doit répondre là aussi.
            if let Some(note) = namsdec_re::libc::explain(word) {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme.mnemonic, egui::RichText::new(word).monospace());
                    if note.watch {
                        ui.colored_label(theme.changed, s.help_watch);
                    }
                });
                ui.add_space(4.0);
                ui.label(prose(ui, theme, note.text(self.lang.code())));
                ui.add_space(6.0);
                ui.colored_label(theme.dim, s.help_kind_borrowed);
                return;
            }
            ui.colored_label(theme.bad, format!("{} : {word}", s.help_unknown));
            ui.colored_label(theme.dim, s.help_unknown_hint);
            return;
        };
        let lang = if self.lang == Lang::Fr { "fr" } else { "en" };
        let mut copié = false;
        let mut suivante = None;
        egui::ScrollArea::vertical()
            .id_salt("help")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for name in entry.names {
                        ui.colored_label(
                            if name.eq_ignore_ascii_case(word) {
                                theme.changed
                            } else {
                                theme.mnemonic
                            },
                            egui::RichText::new(*name).monospace(),
                        );
                    }
                });
                ui.add_space(4.0);
                ui.strong(entry.title.get(lang));
                ui.add_space(2.0);
                ui.label(prose(ui, theme, entry.body.get(lang)));
                ui.add_space(6.0);
                // Les indicateurs ne concernent que les instructions :
                // une directive ne s'exécute pas, elle n'en touche aucun,
                // et l'écrire reviendrait à laisser croire qu'elle
                // pourrait.
                if entry.kind == namsdec_doc::Kind::Instruction {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.help_flags);
                        if entry.flags.is_empty() {
                            ui.colored_label(theme.dim, s.help_no_flags);
                        } else {
                            ui.colored_label(
                                theme.register,
                                egui::RichText::new(entry.flags).monospace(),
                            );
                        }
                    });
                } else {
                    ui.colored_label(
                        theme.dim,
                        match entry.kind {
                            namsdec_doc::Kind::Directive => s.help_kind_directive,
                            namsdec_doc::Kind::Concept => s.help_kind_concept,
                            _ => s.help_kind_preprocessor,
                        },
                    );
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.colored_label(theme.dim, s.help_example);
                    // L'exemple se copie : on lit une fiche pour essayer
                    // ce qu'elle décrit, et le retaper à la main n'ajoute
                    // rien à ce qu'on en apprend.
                    if ui
                        .small_button(s.help_copy_example)
                        .on_hover_text(s.help_copy_hint)
                        .clicked()
                    {
                        ui.ctx().copy_text(entry.example.to_owned());
                        copié = true;
                    }
                });
                ui.add(
                    egui::Label::new(egui::RichText::new(entry.example).monospace())
                        .wrap_mode(egui::TextWrapMode::Extend),
                );

                // Les renvois : les mots que la fiche cite et qui ont
                // eux-mêmes une fiche. Une notion s'apprend en tirant
                // sur le fil, pas en lisant une page isolée.
                let renvois = namsdec_doc::related(entry, lang);
                if !renvois.is_empty() {
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.help_see_also);
                        for autre in renvois {
                            if ui
                                .small_button(egui::RichText::new(autre.names[0]).monospace())
                                .on_hover_text(autre.title.get(lang))
                                .clicked()
                            {
                                suivante = Some(autre.names[0]);
                            }
                        }
                    });
                }
            });
        if let Some(name) = suivante {
            self.open_help(name);
        }
        if copié {
            self.status = Status::Note(s.help_example_copied);
        }
    }

    /// Le sommaire du manuel : les familles, dépliables, et leurs
    /// fiches.
    ///
    /// Chercher suppose de savoir quoi chercher. Parcourir, non — et
    /// c'est ce qu'on fait devant un manuel qu'on découvre.
    fn help_index_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let familles = [
            (namsdec_doc::Kind::Concept, s.help_kind_concepts),
            (namsdec_doc::Kind::Instruction, s.help_kind_instructions),
            (namsdec_doc::Kind::Directive, s.help_kind_directives),
            (namsdec_doc::Kind::Preprocessor, s.help_kind_preprocessors),
        ];

        // Les quatre familles restent visibles quoi qu'on lise : on ne
        // découvre pas un sommaire en faisant défiler ce qu'on ne sait
        // pas encore chercher.
        ui.horizontal_wrapped(|ui| {
            for (kind, titre) in familles {
                let combien = namsdec_doc::all().iter().filter(|e| e.kind == kind).count();
                ui.selectable_value(
                    &mut self.help_index_kind,
                    kind,
                    format!("{titre} ({combien})"),
                );
            }
        });
        ui.separator();

        let fiches: Vec<&namsdec_doc::Entry> = namsdec_doc::all()
            .iter()
            .filter(|entry| entry.kind == self.help_index_kind)
            .collect();
        let mut ouvrir = None;
        egui::ScrollArea::vertical()
            .id_salt("help_sommaire")
            .max_height(window_room(ui))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in fiches {
                    let ligne = ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            theme.mnemonic,
                            egui::RichText::new(entry.names[0]).monospace(),
                        );
                        ui.colored_label(theme.dim, entry.title.get(lang));
                    });
                    if ligne
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_text(s.help_open_sheet)
                        .clicked()
                    {
                        ouvrir = Some(entry.names[0]);
                    }
                }
            });

        if let Some(name) = ouvrir {
            self.open_help(name);
        }
    }

    /// Ce que nasm a dit de la source, une ligne par diagnostic.
    fn diagnostics_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (lang, theme) = (self.lang, self.theme);
        let mut goto = None;
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
                    // Un diagnostic sans ligne — ceux de l'éditeur de
                    // liens, qui parlent du programme entier — ne
                    // s'annonce pas par « ligne 0 ».
                    let text = match (d.line, lang) {
                        (0, _) => d.message.clone(),
                        (line, Lang::Fr) => format!("ligne {line} — {}", d.message),
                        (line, _) => format!("line {line} — {}", d.message),
                    };
                    // Cliquable quand nasm a nommé une ligne : lire le
                    // message et retrouver l'endroit sont deux gestes, et
                    // le second n'a aucune raison d'être manuel.
                    let response = ui.add(
                        egui::Label::new(egui::RichText::new(text).monospace().color(color))
                            .sense(egui::Sense::click()),
                    );
                    if d.line > 0 && response.on_hover_text(s.diagnostic_goto).clicked() {
                        goto = Some(d.line);
                    }
                    // Le message brut reste — c'est lui qu'on colle dans
                    // une recherche —, et l'explication vient dessous.
                    if let Some(explication) = nasm_errors::explain(lang, &d.message) {
                        ui.label(prose(ui, theme, explication));
                        ui.add_space(6.0);
                    }
                }
            });
        if let Some(line) = goto {
            self.goto_line = Some(line);
        }
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

        let titles = self.re_registry.titles(self.lang.code());
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
/// Ce qu'une liste posée dans une fenêtre flottante peut prendre en
/// hauteur : une bonne moitié de l'écran.
///
/// Un plafond en pixels — trois cent soixante, trois cent vingt — faisait
/// défiler quatre thèmes au milieu d'un écran qui en aurait montré
/// quinze. Celui-ci suit l'écran qu'on a.
/// La marque d'un pas : un signe, sa couleur, et ce qu'il veut dire.
///
/// Un signe et non un mot : la colonne reste étroite, et l'on repère un
/// appel ou une écriture en parcourant la liste plutôt qu'en la lisant.
fn step_mark(
    s: &Strings,
    kind: namsdec_emu::StepKind,
    theme: &'static Theme,
) -> (&'static str, egui::Color32, &'static str) {
    use namsdec_emu::StepKind as K;
    match kind {
        K::Plain => ("   ", theme.dim, s.step_plain),
        K::Call => ("app", theme.accent, s.step_call),
        K::Return => ("ret", theme.accent, s.step_return),
        K::Jump => ("saut", theme.warn, s.step_jump),
        K::Syscall => ("sys", theme.good, s.step_syscall),
        K::Write => ("écr", theme.changed, s.step_write),
    }
}

/// Où mène un retour à la ligne qui garde l'indentation courante.
///
/// Rend le texte à insérer et de combien le curseur doit avancer. On
/// écrit l'assembleur en colonnes — les étiquettes à gauche, les
/// instructions en retrait — et retaper quatre espaces à chaque ligne
/// use la patience avant d'user le clavier.
///
/// L'indentation reprise est celle de la ligne où l'on est, blancs
/// seulement : ce qui suit appartient au code.
fn newline_indent(source: &str, cursor: usize) -> String {
    let cursor = cursor.min(source.len());
    let line_start = source[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let indent: String = source[line_start..cursor]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    format!("\n{indent}")
}

/// La ligne où l'étiquette `name` est définie, en comptant à partir de 1.
///
/// Une étiquette se déclare en tête de ligne, suivie de deux points :
/// `boucle:`. Ce qui apparaît ailleurs — dans un `jmp`, dans un
/// commentaire — n'est pas une définition et ne compte pas.
fn label_line(source: &str, name: &str) -> Option<u32> {
    source.lines().enumerate().find_map(|(index, line)| {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix(name)?;
        // `boucle:` et non `bouclette:` — le nom doit s'arrêter là.
        let rest = rest.trim_start();
        rest.starts_with(':').then_some(index as u32 + 1)
    })
}

/// Le décalage d'une valeur écrite à la main dans le champ.
///
/// Trois écritures sont acceptées, parce que c'est sous ces trois
/// formes-là qu'on retrouve le motif : un nombre (`0x6161616c`, ce que
/// montre un registre), les caractères eux-mêmes (`laaa`, ce que montre
/// un vidage), et le même nombre en décimal.
fn cyclic_offset(saisie: &str) -> Option<usize> {
    let saisie = saisie.trim();
    if saisie.is_empty() {
        return None;
    }
    if let Some(hex) = saisie
        .strip_prefix("0x")
        .or_else(|| saisie.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16)
            .ok()
            .and_then(namsdec_re::cyclic::offset_of_value);
    }
    if let Ok(value) = saisie.parse::<u64>() {
        return namsdec_re::cyclic::offset_of_value(value);
    }
    namsdec_re::cyclic::offset(saisie.as_bytes())
}

/// Les entrées de la table de liaison, en étiquettes du listing.
///
/// Un `call 0x4010a0` ne dit rien ; le même appel, quand l'adresse porte
/// le nom `malloc@plt`, dit tout. Les entrées deviennent donc des
/// symboles comme les autres : le listing les écrit, le bandeau les
/// nomme, et « Aller à une étiquette » les propose.
///
/// `place` dit où l'adresse d'une entrée tombe dans ce qui est
/// désassemblé : telle quelle pour une image chargée à ses adresses
/// virtuelles, décalée quand c'est le fichier brut qu'on lit.
fn plt_labels(
    file: &[u8],
    existants: &[namsdec_asm::elf::Symbol],
    place: impl Fn(u64) -> Option<u64>,
) -> Vec<namsdec_asm::elf::Symbol> {
    namsdec_re::imports::scan(file)
        .into_iter()
        .filter_map(|import| {
            let addr = place(import.plt)?;
            // Un nom donné par la table des symboles prime : il vient du
            // binaire, celui-ci est déduit.
            if existants.iter().any(|symbol| symbol.addr == addr) {
                return None;
            }
            Some(namsdec_asm::elf::Symbol {
                name: format!("{}@plt", import.name),
                addr,
                size: 0,
            })
        })
        .collect()
}

/// Le listing d'un binaire ouvert : les instructions, avec les étiquettes
/// que sa table des symboles porte encore.
///
/// Un désassemblage brut est une colonne d'instructions sans début ni
/// fin : on ne voit pas où une fonction commence, et un `call 401030h`
/// ne dit pas *qui* est appelé. Les étiquettes rendent au binaire la
/// structure que le source avait — c'est la première chose qu'on
/// reconstitue à la main en lisant du code qu'on n'a pas écrit.
///
/// Rend le texte et la table ligne → adresse : les lignes d'étiquette
/// n'y figurent pas, elles ne portent aucune instruction.
fn annotated_listing(
    disasm: &[DecodedInstruction],
    symbols: &[namsdec_asm::elf::Symbol],
) -> (String, BTreeMap<u32, u64>) {
    let mut listing = String::new();
    let mut line_to_addr = BTreeMap::new();
    let mut line = 1u32;
    for instruction in disasm {
        // Une étiquette qui commence ici : elle prend sa ligne, précédée
        // d'une ligne vide — sauf tout en haut, où elle ferait un trou.
        for symbol in symbols.iter().filter(|s| s.addr == instruction.address) {
            if line > 1 {
                listing.push('\n');
                line += 1;
            }
            listing.push_str(&symbol.name);
            listing.push_str(":\n");
            line += 1;
        }
        line_to_addr.insert(line, instruction.address);
        listing.push_str(&instruction.text);
        // La cible d'un saut ou d'un appel, nommée en commentaire : le
        // nombre reste — c'est lui qui est dans les octets — mais on
        // sait enfin où il mène.
        if let Some(target) = branch_target(&instruction.text)
            && let Some(label) = symbol_at(symbols, target)
        {
            listing.push_str(&format!("    ; -> {label}"));
        }
        listing.push('\n');
        line += 1;
    }
    (listing, line_to_addr)
}

/// L'adresse que vise un saut ou un appel, lue dans le texte que le
/// désassembleur a écrit (`call 401030h`, `jne 400012h`).
///
/// Par le texte plutôt qu'en redécodant : c'est ce texte-là qui est
/// affiché, et l'étiquette doit nommer ce qu'il montre.
fn branch_target(text: &str) -> Option<u64> {
    let (mnemonic, operand) = text.split_once(' ')?;
    if !(mnemonic.starts_with('j') || mnemonic == "call" || mnemonic.starts_with("loop")) {
        return None;
    }
    let operand = operand.trim();
    let hex = operand.strip_suffix('h')?;
    u64::from_str_radix(hex, 16).ok()
}

/// L'étiquette qui nomme `addr`, telle qu'on l'a écrite dans le source :
/// `tampon`, ou `tampon+2` deux octets plus loin.
///
/// Les symboles sont triés par adresse : on prend le dernier qui commence
/// avant, et on ne le propose que s'il est assez proche pour dire encore
/// quelque chose — sa taille déclarée quand nasm en donne une, sinon la
/// distance jusqu'au symbole suivant.
pub(crate) fn symbol_at(symbols: &[namsdec_asm::elf::Symbol], addr: u64) -> Option<String> {
    let index = symbols.partition_point(|s| s.addr <= addr).checked_sub(1)?;
    let symbol = &symbols[index];
    let offset = addr - symbol.addr;
    let reach = if symbol.size > 0 {
        symbol.size
    } else {
        symbols
            .get(index + 1)
            .map(|next| next.addr - symbol.addr)
            .unwrap_or(1)
    };
    if offset >= reach {
        return None;
    }
    if offset == 0 {
        Some(symbol.name.clone())
    } else {
        Some(format!("{}+{offset}", symbol.name))
    }
}

fn window_room(ui: &egui::Ui) -> f32 {
    (ui.ctx().input(|i| i.content_rect()).height() * 0.55).max(240.0)
}

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
        Action::StepBack => s.action_step_back,
        Action::Stop => s.action_stop,
        Action::Reset => s.action_reset,
        Action::Save => s.action_save,
        Action::OpenFile => s.file_open_file,
        Action::OpenFolder => s.file_open_folder,
        Action::InstructionHelp => s.action_instruction_help,
        Action::GoToLabel => s.action_goto_label,
        Action::Find => s.action_find,
        Action::New => s.file_new,
        Action::RunToCursor => s.action_run_to_cursor,
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
        // Un cadre, toujours : au repos, le thème rend le fond des
        // boutons transparent et le liseré invisible — deux icônes
        // posées à côté d'un champ passaient alors pour de la décoration.
        // Un bouton doit se voir sans qu'on ait à le survoler.
        let stroke = if visuals.bg_stroke.width > 0.0 {
            visuals.bg_stroke
        } else {
            egui::Stroke::new(1.0, ui.visuals().weak_text_color())
        };
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            stroke,
            egui::StrokeKind::Inside,
        );
        icon(ui.painter(), rect.shrink(5.0), visuals.text_color());
    }
    response
}

/// Surveiller : un œil. Deux arcs et une pupille — de quoi se
/// reconnaître à quatorze pixels sans ressembler au reste.
fn icon_watch(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.3, color);
    let centre = rect.center();
    let half = rect.width() * 0.5;
    // Les deux paupières, chacune une suite de segments.
    for sign in [-1.0, 1.0] {
        let points: Vec<egui::Pos2> = (0..=8)
            .map(|i| {
                let t = -1.0 + i as f32 * 0.25;
                egui::pos2(
                    centre.x + t * half,
                    centre.y + sign * (1.0 - t * t) * half * 0.62,
                )
            })
            .collect();
        painter.add(egui::Shape::line(points, stroke));
    }
    painter.circle_stroke(centre, half * 0.26, stroke);
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

/// Le pas arrière : une flèche qui remonte, exactement l'inverse de celle
/// de Step Into, qui descend dans l'instruction.
fn icon_step_back(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let x = rect.center().x;
    painter.line_segment(
        [
            egui::pos2(x, rect.bottom() - 1.0),
            egui::pos2(x, rect.top() + 1.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x - 3.5, rect.top() + 4.5),
            egui::pos2(x, rect.top() + 1.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x + 3.5, rect.top() + 4.5),
            egui::pos2(x, rect.top() + 1.0),
        ],
        stroke,
    );
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
/// Un bouton **encadré**, pour les endroits où il doit se voir sans
/// qu'on ait à le survoler.
///
/// Le thème met les boutons à plat au repos — c'est ce qui empêche la
/// barre de menus et la barre d'outils de se ressembler. Dans une boîte
/// de dialogue, l'effet se retourne contre nous : « Ajouter un thème… »
/// se lisait comme une phrase, et personne n'allait cliquer dessus.
fn framed_button(ui: &mut egui::Ui, theme: &Theme, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .stroke(egui::Stroke::new(1.0, theme.dim))
            .corner_radius(egui::CornerRadius::from(6)),
    )
}

/// La vignette d'un thème, de largeur imposée.
///
/// Imposée, et non « toute la place » : les boutons « Retirer » qui la
/// suivent doivent s'aligner d'une ligne à l'autre, donc toutes les
/// vignettes doivent finir à la même abscisse — celle que dicte le nom
/// le plus long (voir `theme_card_width`).
fn theme_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    active: bool,
    active_word: &str,
    width: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 44.0), egui::Sense::click());

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

    response
}

/// La largeur commune des vignettes : de quoi loger la plus longue des
/// étiquettes — le nom, et « actif » qui le suit quand le thème l'est.
///
/// Mesurée sur les polices réelles plutôt qu'estimée : un thème installé
/// peut s'appeler « Solarized Light Extra Contrast », et les boutons
/// doivent rester alignés.
fn theme_card_width(ui: &egui::Ui, themes: &[&Theme], active_word: &str) -> f32 {
    let text_width = |text: &str, size: f32| {
        ui.painter()
            .layout_no_wrap(
                text.to_owned(),
                egui::FontId::proportional(size),
                egui::Color32::PLACEHOLDER,
            )
            .size()
            .x
    };
    let widest_name = themes
        .iter()
        .map(|theme| text_width(theme.name, 14.0))
        .fold(0.0_f32, f32::max);
    // Vignette (8 + 58) + espace (10) + nom + espace + « actif » + marge.
    76.0 + widest_name + 12.0 + text_width(active_word, 12.0) + 10.0
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

        self.guard_close(&ctx);
        // Accordée : on repasse la demande au gestionnaire de fenêtres,
        // qui l'avait vue refusée.
        if self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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
            self.help_window_ui(&ctx);
            self.vectors_window_ui(&ctx);
            self.breakpoints_window_ui(&ctx);
            self.timeline_window_ui(&ctx);
            self.disassembly_window_ui(&ctx);
            self.cyclic_window_ui(&ctx);
            self.analysis_window_ui(&ctx);
            if self.exercises {
                self.exercises_window_ui(&ctx);
            }
            self.command_palette_ui(&ctx);
            self.unsaved_window_ui(&ctx);
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

/// L'éditeur porte un identifiant fixe : l'autocomplétion doit pouvoir
/// replacer son curseur après avoir écrit, ce qui demande de retrouver
/// son état.
fn find_field_id() -> egui::Id {
    egui::Id::new("find_field")
}

fn editor_id() -> egui::Id {
    egui::Id::new("editeur")
}

/// Décalage en octets du `n`-ième caractère, et l'inverse. egui compte
/// les curseurs en caractères, Rust tranche les chaînes en octets, et
/// confondre les deux coupe un « é » en deux.
/// Un texte d'explication où les morceaux de code sont écrits comme du
/// code.
///
/// Les fiches marquent ces morceaux entre accents graves, comme partout
/// ailleurs. Les laisser tels quels donnerait « préparer RDX — `xor rdx,
/// rdx` » ; les retirer sans rien mettre à la place perdrait la
/// distinction. Un `LayoutJob` permet de changer de police au milieu
/// d'une phrase, sans casser le retour à la ligne.
fn prose(ui: &egui::Ui, theme: &'static Theme, text: &str) -> egui::text::LayoutJob {
    let normal = egui::TextFormat {
        font_id: egui::TextStyle::Body.resolve(ui.style()),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let code = egui::TextFormat {
        font_id: egui::TextStyle::Monospace.resolve(ui.style()),
        color: theme.mnemonic,
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    for (index, part) in text.split('`').enumerate() {
        if part.is_empty() {
            continue;
        }
        // Un accent grave sur deux ouvre un morceau de code.
        let format = if index % 2 == 0 { &normal } else { &code };
        job.append(part, 0.0, format.clone());
    }
    job
}

/// Le mot qui entoure `byte` dans `text`.
///
/// « Mot » au sens de l'assembleur : lettres, chiffres, et le souligné et
/// le point qui font partie des noms (`_start`, `.boucle`). Le curseur
/// posé juste après un mot compte comme dedans — c'est là qu'il se trouve
/// quand on vient de le taper.
fn word_at(text: &str, byte: usize) -> &str {
    fn is_word(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '.'
    }
    let byte = byte.min(text.len());
    // `$` et `$$` ne sont pas des mots au sens des lettres, mais ce sont
    // bien des choses sur lesquelles on demande de l'aide : l'adresse
    // courante et le début de la section.
    if let Some(dollars) = dollars_at(text, byte) {
        return dollars;
    }
    // Reculer jusqu'au début du mot, avancer jusqu'à sa fin.
    let start = text[..byte]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word(*c))
        .map(|(i, _)| i)
        .last()
        .unwrap_or(byte);
    let end = byte
        + text[byte..]
            .char_indices()
            .take_while(|(_, c)| is_word(*c))
            .map(|(i, c)| i + c.len_utf8())
            .last()
            .unwrap_or(0);
    // Un `%` devant fait partie du nom : `%macro`, `%define`. Le
    // préprocesseur est un langage à part entière, et c'est là qu'on
    // demande ce qu'il fait.
    let start = if text[..start].ends_with('%') {
        start - 1
    } else {
        start
    };
    // Un point *devant* fait partie du nom (`.boucle`) ; un point
    // derrière est de la ponctuation — « … avant idiv. » dans un
    // commentaire ne demande pas d'aide sur « idiv. ».
    text[start..end].trim_end_matches('.')
}

/// `$` ou `$$` sous le curseur, s'il y en a un.
fn dollars_at(text: &str, byte: usize) -> Option<&str> {
    let before = &text[..byte];
    let start = before.len() - before.chars().rev().take_while(|c| *c == '$').count();
    let end = byte + text[byte..].chars().take_while(|c| *c == '$').count();
    (end > start).then(|| &text[start..end])
}

fn char_to_byte(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(index, _)| index)
}

fn byte_to_char(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].chars().count()
}

/// La boîte « À propos » porte un identifiant fixe, indépendant de son
/// titre traduit.
fn about_window_id() -> egui::Id {
    egui::Id::new("about_window")
}

/// Les seize registres SSE, dans l'ordre de leur numéro.
const XMM_REGISTERS: [(&str, namsdec_core::RegXmm); 16] = {
    use namsdec_core::RegXmm::*;
    [
        ("XMM0", Xmm0),
        ("XMM1", Xmm1),
        ("XMM2", Xmm2),
        ("XMM3", Xmm3),
        ("XMM4", Xmm4),
        ("XMM5", Xmm5),
        ("XMM6", Xmm6),
        ("XMM7", Xmm7),
        ("XMM8", Xmm8),
        ("XMM9", Xmm9),
        ("XMM10", Xmm10),
        ("XMM11", Xmm11),
        ("XMM12", Xmm12),
        ("XMM13", Xmm13),
        ("XMM14", Xmm14),
        ("XMM15", Xmm15),
    ]
};

/// Les huit cases de la pile du x87, nommées comme le programme les
/// désigne : relativement au sommet.
const X87_SLOTS: [&str; 8] = ["ST0", "ST1", "ST2", "ST3", "ST4", "ST5", "ST6", "ST7"];

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

/// Où vivent les thèmes installés : à côté des réglages, dans le dossier
/// de données de l'utilisateur.
/// Les désassembleurs installés : un fichier JSON par outil, dans le
/// dossier des plugins.
///
/// Rien n'est lancé ici — on lit seulement ce qui est déclaré. Un
/// désassembleur extérieur *est un programme* : il ne s'exécute que
/// lorsqu'on le choisit, et le panneau le dit avant.
fn load_disassembler_plugins() -> Vec<namsdec_disasm::plugin::Plugin> {
    let Some(dir) = data_dir().map(|dir| dir.join("disassemblers")) else {
        return Vec::new();
    };
    namsdec_disasm::plugin::read_dir(&dir)
}

fn plugins_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("themes"))
}

/// Le dossier de données de l'application : ce que l'utilisateur y ajoute
/// — thèmes, séries d'exercices — y reste d'une version à l'autre.
fn data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })?;
    Some(base.join("namsdec"))
}

/// Le nom sous lequel annoncer une section dans un message de faute.
///
/// Les quatre noms usuels passent tels quels ; une section au nom exotique
/// est annoncée par ce qu'elle *est* — du code, des données, de la donnée
/// en lecture seule. Le message doit pouvoir être écrit dans les deux
/// langues, et un nom venu du fichier ne peut pas l'être : il n'a pas de
/// durée de vie statique, et il ne se traduit pas.
fn region_label(section: &namsdec_asm::Section) -> &'static str {
    match section.name.as_str() {
        ".text" => ".text",
        ".data" => ".data",
        ".bss" => ".bss",
        ".rodata" => ".rodata",
        _ if section.executable => ".text",
        _ if section.writable => ".data",
        _ => ".rodata",
    }
}

/// Où vivent les séries d'exercices installées. À côté des thèmes, et
/// pour la même raison : ce sont des données que l'utilisateur ajoute, et
/// elles doivent survivre à une mise à jour de l'application.
fn exercises_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("exercises"))
}

/// Recopie le fichier de thème dans le dossier des plugins, pour qu'il
/// revienne au lancement suivant. Un fichier déjà installé est écrasé :
/// c'est la version qu'on vient de choisir qui fait foi.
fn copy_into_plugins(path: &Path) -> std::io::Result<PathBuf> {
    copy_into_dir(path, plugins_dir())
}

/// Recopie un fichier dans un dossier de données, en le créant au besoin.
/// Un fichier déjà installé est écrasé : c'est la version qu'on vient de
/// choisir qui fait foi.
fn copy_into_dir(path: &Path, dir: Option<PathBuf>) -> std::io::Result<PathBuf> {
    let dir = dir.ok_or_else(|| {
        std::io::Error::other("aucun dossier de données (ni XDG_DATA_HOME ni HOME)")
    })?;
    std::fs::create_dir_all(&dir)?;
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("le chemin ne nomme pas un fichier"))?;
    let destination = dir.join(name);
    // Se recopier sur soi-même n'a pas de sens, et `copy` s'en plaindrait.
    if path != destination {
        std::fs::copy(path, &destination)?;
    }
    Ok(destination)
}

/// Charge les séries d'exercices déjà installées. Comme pour les thèmes :
/// un fichier illisible est nommé dans le compte rendu et n'empêche pas
/// les autres de se charger.
fn load_installed_exercises() -> (
    Vec<namsdec_exercises::Pack>,
    BTreeMap<String, PathBuf>,
    Vec<String>,
) {
    let mut packs = Vec::new();
    let mut files = BTreeMap::new();
    let mut failures = Vec::new();
    let Some(dir) = exercises_dir() else {
        return (packs, files, failures);
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return (packs, files, failures);
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    for path in paths {
        match namsdec_exercises::pack::read(&path) {
            Ok(pack) if pack.id != namsdec_exercises::BUILTIN_ID => {
                files.insert(pack.id.clone(), path);
                packs.push(pack);
            }
            // Une série qui se fait passer pour celle d'origine est
            // ignorée plutôt que de la remplacer en silence.
            Ok(pack) => failures.push(format!("{} : {}", path.display(), pack.id)),
            Err(e) => failures.push(format!("{} : {e}", path.display())),
        }
    }
    (packs, files, failures)
}

/// Charge les thèmes déjà installés. Appelé une fois au démarrage.
///
/// Un fichier illisible n'empêche pas les autres de se charger : il est
/// nommé dans le compte rendu, et l'application démarre quand même —
/// perdre son éditeur parce qu'un thème a mal vieilli serait absurde.
fn load_installed_themes() -> (BTreeMap<String, PathBuf>, Vec<String>) {
    let mut files = BTreeMap::new();
    let mut failures = Vec::new();
    let Some(dir) = plugins_dir() else {
        return (files, failures);
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return (files, failures);
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    // Ordre stable : la liste des thèmes ne doit pas changer d'ordre d'un
    // lancement à l'autre.
    paths.sort();
    for path in paths {
        match namsdec_theme::plugin::read(&path).and_then(namsdec_theme::install) {
            Ok(theme) => {
                files.insert(theme.id.to_owned(), path);
            }
            Err(e) => failures.push(format!("{} : {e}", path.display())),
        }
    }
    (files, failures)
}

/// La largeur que l'éditeur réclame : celle du code, ou celle de la place
/// disponible s'il est plus étroit.
///
/// Surtout pas l'infini, qui était demandé jusqu'ici : la zone de
/// défilement croyait alors toujours avoir quelque chose à atteindre à
/// droite, et gardait sa barre horizontale sous trois lignes de vingt
/// caractères. Prendre au moins la place disponible garde en revanche le
/// clic actif sur toute la largeur, jusqu'au bord droit.
fn editor_width(code: f32, available: f32) -> f32 {
    code.max(available)
}

/// Combien de lignes l'éditeur réclame : de quoi remplir la hauteur
/// visible, ni plus ni moins.
///
/// Un nombre fixe — vingt-quatre, hérité du terminal — donnait une barre
/// de défilement verticale sur un fichier de six lignes : le champ
/// réclamait plus de hauteur que la fenêtre n'en montrait, et la zone de
/// défilement en concluait qu'il y avait à voir plus bas. Rempli à la
/// hauteur visible, il n'y a rien à atteindre tant que le texte ne
/// dépasse pas — et le clic reste actif jusqu'en bas, ce qu'un champ
/// ajusté au texte aurait perdu.
fn editor_rows(visible_height: f32, row_height: f32) -> usize {
    if row_height <= 0.0 {
        return 1;
    }
    ((visible_height / row_height).floor() as usize).max(1)
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
    use std::hash::{Hash, Hasher};

    // La coloration d'un binaire désassemblé — quinze mille lignes —
    // prend une dizaine de millisecondes, et la mise en page autant.
    // C'est refait au moins deux fois par frame : une fois pour la
    // gouttière, une fois par le champ de texte. Tant que ni le texte,
    // ni le thème, ni la taille de la police ne changent, le résultat
    // est le même : on le garde.
    thread_local! {
        static CACHE: std::cell::RefCell<Option<(u64, std::sync::Arc<egui::Galley>)>> =
            const { std::cell::RefCell::new(None) };
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    (theme as *const Theme as usize).hash(&mut hasher);
    // Le zoom change la taille de la police sans toucher au texte.
    let size = egui::TextStyle::Monospace.resolve(ui.style()).size;
    size.to_bits().hash(&mut hasher);
    let key = hasher.finish();

    if let Some(galley) = CACHE.with(|cache| {
        cache
            .borrow()
            .as_ref()
            .filter(|(cached, _)| *cached == key)
            .map(|(_, galley)| galley.clone())
    }) {
        return galley;
    }

    let job = syntax::highlight(ui, theme, text, f32::INFINITY);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    CACHE.with(|cache| *cache.borrow_mut() = Some((key, galley.clone())));
    galley
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

/// Le registre que ce nom désigne, en minuscules — les seize généraux.
fn register_by_name(name: &str) -> Option<Reg64> {
    REGISTERS
        .iter()
        .find(|(label, _)| label.eq_ignore_ascii_case(name))
        .map(|&(_, reg)| reg)
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
            // Les thèmes installés sont chargés *avant* les réglages :
            // celui que l'utilisateur avait choisi peut être l'un d'eux,
            // et `by_id` doit pouvoir le retrouver.
            let (theme_files, mut failures) = load_installed_themes();
            let (packs, exercise_files, pack_failures) = load_installed_exercises();
            failures.extend(pack_failures);
            let mut app = NamsdecApp::from_settings(Settings::load());
            // Le seul endroit qui donne ce chemin : c'est ici que
            // l'application devient celle qui écrit vraiment.
            app.settings_file = settings::settings_path();
            app.theme_files = theme_files;
            // Les séries installées viennent après celle d'origine, qui
            // reste en tête : c'est là qu'on commence.
            app.exercise_packs.extend(packs);
            app.exercise_files = exercise_files;
            if !failures.is_empty() {
                app.plugin_note = Some(PluginNote {
                    ok: false,
                    text: format!(
                        "{} {}",
                        strings(app.lang).plugins_failed,
                        failures.join(" ; ")
                    ),
                });
            }
            app.save_settings_now();
            app.apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests;
