//! Coquille eframe/egui du MVP d'apprentissage NASM. Assemble via
//! `namsdec-asm` (nasm externe), désassemble via `namsdec-disasm`
//! (vérification pédagogique) et exécute pas à pas via `namsdec-emu`,
//! en affichant les panneaux des modules `ReModule` enregistrés.

mod chrome;
mod completion;
mod condition;
mod errors;
mod fixes;
mod i18n;
mod memfind;
mod keybindings;
mod loader;
mod nasm_errors;
mod calc;
mod notes;
mod occurrences;
mod readings;
mod timeline;
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
    BinaryInfoModule, DigestModule, DumpModule, EntropyModule,
    MapModule, FunctionsModule, GadgetsModule, ImportsModule, PatternsModule,
    ProtectionsModule, ReContext, ReModuleRegistry, StringsModule, SyscallsModule, XrefsModule,
};
use namsdec_theme::Theme;

use i18n::{Lang, Strings, strings};
use keybindings::{Action, Dispatcher, KeyBindings, Rebinding};
use settings::Settings;

const BASE_ADDR: u64 = 0x400000;
const MEM_SIZE: usize = 0x10000;

/// Ce que la mémoire simulée accepte de couvrir, au plus.
///
/// Soixante-quatre kilo-octets suffisent à un exercice, et ne suffisent
/// à aucun vrai binaire : un programme de trois cent mille octets n'y
/// entrait pas, namsdec retombait alors sur un désassemblage brut aux
/// adresses décalées, et l'on croyait lire le programme qu'on venait
/// d'ouvrir. La fenêtre s'adapte donc à ce qu'on charge — jusqu'à cette
/// borne, au-delà de laquelle on refuse en le disant plutôt que
/// d'allouer un gigaoctet.
const MEM_MAX: usize = 256 * 1024 * 1024;

/// La fenêtre de mémoire simulée pour un programme donné.
///
/// Un exercice — chargé à `BASE_ADDR`, quelques centaines d'octets —
/// retrouve exactement la fenêtre d'avant : même base, même taille, même
/// sommet de pile. Un binaire ouvert, lui, étend la fenêtre jusqu'à
/// couvrir son image, en gardant la pile tout en haut.
fn memory_window(load_addr: u64, image_len: usize) -> (u64, usize, u64) {
    let début = load_addr.min(BASE_ADDR);
    // La fin couvre l'image **et** la fenêtre historique : ainsi la pile
    // d'un exercice ne bouge pas d'un octet, quoi qu'on ait chargé.
    let fin = (load_addr + image_len as u64).max(BASE_ADDR + MEM_SIZE as u64);
    // Arrondi à la page : une fenêtre qui s'arrête au milieu d'une page
    // n'a de sens pour personne.
    let fin = fin.div_ceil(0x1000) * 0x1000;
    let taille = (fin - début) as usize;
    (début, taille, fin - 0x100)
}
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
const CONSOLE_MIN_HEIGHT: f32 = 72.0;
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
/// L'icône de l'application, en pixels bruts — 128 x 128, la même que
/// celle du lanceur. Embarquée comme la vignette ci-dessus, et pour la
/// même raison : pas de décodeur à traîner pour une image.
///
/// ```text
/// magick assets/icons/namsdec-128.png -depth 8 RGBA:crates/namsdec-gui/assets/namsdec-128.rgba
/// ```
const APP_ICON: &[u8] = include_bytes!("../assets/namsdec-128.rgba");
const APP_ICON_SIDE: usize = 128;
/// La taille à laquelle la boîte « À propos » la montre, et la largeur
/// de la boîte elle-même.
const APP_ICON_SHOWN: f32 = 88.0;
const ABOUT_WIDTH: f32 = 560.0;
const TREAT_IMAGE_SIDE: usize = 128;
/// Taille d'affichage dans la boîte : l'image est embarquée deux fois
/// plus grande, pour rester nette sur un écran à forte densité.
const TREAT_IMAGE_SHOWN: f32 = 84.0;

/// Le texte de la licence sous laquelle namsdec est publié.
/// Le thème d'exemple, volontairement **hors du dépôt** : c'est un
/// plugin, il doit s'installer depuis n'importe où sur le disque.
///
/// D'où la variable d'environnement plutôt qu'un chemin en dur : celui
/// qui était écrit ici ne désignait quelque chose que sur une seule
/// machine, et disait au passage où vivaient ses dossiers.
#[cfg(test)]
fn external_abyss() -> Option<std::path::PathBuf> {
    let chemin = std::path::PathBuf::from(std::env::var_os("NAMSDEC_EXTERNAL_THEME")?);
    chemin.is_file().then_some(chemin)
}

const MIT_LICENSE_URL: &str = "https://opensource.org/license/mit";
/// Où l'on trouve namsdec, et où l'on en parle.
///
/// Trois adresses et pas une de plus : le site, le salon, le dépôt.
/// Une boîte « À propos » qui aligne dix liens ne dit plus lequel
/// compte.
const SITE_URL: &str = "https://namsdec.io";
const DISCORD_URL: &str = "https://discord.gg/pv6pHkzB5";
const GITHUB_URL: &str = "https://github.com/fredza/namsdec";
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
/// Une installation en cours : ce qu'elle fait, et ce qu'elle a dit.
///
/// Le journal est gardé en entier. Une installation qui échoue à la
/// septième étape sur huit n'apprend rien si l'on n'a que la dernière
/// ligne : ce qu'on veut lire, c'est ce que la commande a répondu.
struct InstallJob {
    tool: String,
    /// Les étapes, pour dire où l'on en est.
    total: usize,
    done: usize,
    /// Ce que les commandes ont écrit, dans l'ordre.
    log: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Fini ? Et si oui, comment.
    outcome: std::sync::Arc<std::sync::Mutex<Option<Result<PathBuf, String>>>>,
    /// Combien d'étapes sont passées, vu depuis le fil de travail.
    progress: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// Mis à vrai pour demander l'arrêt : le fil s'arrête à la fin de
    /// l'étape en cours plutôt qu'au milieu d'une décompression.
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// Un travail qui se fait pendant qu'on continue à travailler.
///
/// Le fil d'à côté est invisible : sans un mot quelque part, on ne sait
/// ni que quelque chose tourne, ni depuis combien de temps, ni combien
/// de temps encore. Trois questions auxquelles cette structure répond.
#[derive(Clone)]
struct BackgroundJob {
    /// Ce qui tourne, dit en un mot.
    label: String,
    started: std::time::Instant,
    /// Le délai au bout duquel on arrêtera l'outil, quand il y en a un.
    ///
    /// C'est la **seule** progression honnête qu'on puisse montrer : un
    /// programme extérieur ne dit pas où il en est. Le temps écoulé sur
    /// le temps accordé, lui, est vrai — et c'est ce qui intéresse,
    /// puisque c'est ce qui décide de l'abandon.
    limit: Option<std::time::Duration>,
}

impl BackgroundJob {
    /// La part du temps accordé qui s'est écoulée, quand elle a un sens.
    fn fraction(&self) -> Option<f32> {
        let limite = self.limit?.as_secs_f32();
        (limite > 0.0).then(|| (self.started.elapsed().as_secs_f32() / limite).clamp(0.0, 1.0))
    }

    /// « 12 s sur 2 min », ou simplement « 12 s » quand rien ne borne.
    fn timing(&self) -> String {
        let écoulé = self.started.elapsed().as_secs();
        match self.limit {
            Some(limite) => format!("{écoulé} s / {} s", limite.as_secs()),
            None => format!("{écoulé} s"),
        }
    }
}

/// Le temps qu'on laisse à un décompilateur.
///
/// Un désassembleur répond en un souffle ; un décompilateur reconstruit
/// des structures de contrôle et peut y passer une demi-minute. Au-delà,
/// c'est qu'il ne rendra pas la main : on l'arrête, et on le dit.
/// Le nom de la fenêtre, tel que la barre des tâches l'affiche.
const APP_NAME: &str = "Namsdec";

const DECOMPILER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Un fichier d'exemple, montré quand aucun outil n'est installé.
///
/// Écrit en toutes lettres plutôt que renvoyé à une documentation : ce
/// qu'on demande à l'utilisateur, c'est d'autoriser l'exécution d'un
/// programme, et il doit voir exactement à quoi cela ressemble.
const DECOMPILER_EXAMPLE: &str = r#"{
  "id": "objdump",
  "name": "objdump (désassemblage)",
  "command": "objdump",
  "args": ["-d", "-M", "intel", "{file}"]
}

{
  "id": "retdec",
  "name": "RetDec (décompilation en C)",
  "command": "retdec-decompiler",
  "args": ["--stdout", "{file}"]
}"#;

const SIDEBAR_MIN_WIDTH: f32 = 200.0;

/// La largeur au-dessous de laquelle le panneau latéral ne se resserre
/// plus.
///
/// Mesurée, et non fixée : elle vaut une paire « nom de registre +
/// valeur sur seize chiffres », plus les marges. Plus étroit, le
/// seizième chiffre passe sous le bord — et une valeur tronquée n'est
/// pas une valeur abrégée, c'est une valeur fausse que rien ne signale.
/// La police et le zoom la font varier, d'où la mesure à chaque image.
fn sidebar_min_width(ui: &egui::Ui) -> f32 {
    let spacing = ui.spacing();
    // Ce qui se prend de la largeur avant que la grille n'en voie la
    // couleur : les marges du panneau, celles de la grille, et la barre
    // de défilement du bandeau, qui est réservée qu'elle serve ou non.
    let marges = 4.0 * spacing.item_spacing.x
        + spacing.scroll.bar_width
        + spacing.scroll.bar_inner_margin
        + spacing.scroll.bar_outer_margin
        + 16.0;
    (sidebar::register_pair_width(ui) + marges).max(SIDEBAR_MIN_WIDTH)
}

/// La hauteur minimale laissée au contenu de l'onglet du panneau
/// latéral — Pile, Mémoire, Tas, Octets, RE.
///
/// Le bandeau du haut grandit avec ce qu'il montre : une instruction
/// détaillée, le dernier pas, dix-huit registres. Sur un panneau étroit,
/// il finissait par tout prendre et l'onglet se réduisait à son titre et
/// à deux lignes — l'analyse d'un binaire à travers une meurtrière. Ce
/// plancher lui garantit de quoi être lu.
const SIDEBAR_TAB_MIN: f32 = 220.0;

/// Et ce qui reste au bandeau, quoi qu'il arrive : de quoi lire ce qui
/// vient et les premières lignes de l'état du processeur.
const SIDEBAR_BANNER_MIN: f32 = 280.0;
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

/// Le nom de la préversion, quand il y en a une : « beta.2 », « rc.1 ».
///
/// Lu dans le numéro de version lui-même, jamais écrit à la main : le
/// jour où `Cargo.toml` passe à une version stable, le bandeau qui
/// prévient disparaît tout seul. Un avertissement qu'il faut penser à
/// retirer finit toujours par mentir.
///
/// ```ignore
/// assert_eq!(prerelease("0.2.1-beta.2"), Some("beta.2"));
/// assert_eq!(prerelease("0.2.1"), None);
/// ```
fn prerelease(version: &str) -> Option<&str> {
    // SemVer : ce qui suit le premier tiret est la préversion, ce qui
    // suit un `+` est la métadonnée de compilation et n'en fait pas
    // partie.
    let reste = version.split('+').next().unwrap_or(version);
    let étiquette = reste.split_once('-')?.1;
    (!étiquette.is_empty()).then_some(étiquette)
}

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

    /// L'identifiant qui part dans les réglages.
    ///
    /// Stable, et distinct du libellé : celui-ci change avec la langue,
    /// et un fichier écrit en français ne doit pas devenir illisible
    /// parce qu'on a basculé en anglais.
    fn id(self) -> &'static str {
        match self {
            SidebarTab::Stack => "stack",
            SidebarTab::Memory => "memory",
            SidebarTab::Heap => "heap",
            SidebarTab::Bytes => "bytes",
            SidebarTab::Re => "re",
        }
    }

    /// L'onglet que cet identifiant désigne, s'il en désigne un.
    fn from_id(id: &str) -> Option<Self> {
        SidebarTab::ALL.into_iter().find(|tab| tab.id() == id)
    }

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

/// Jusqu'où court « aller au prochain… ».
///
/// Trois questions, celles qu'on se pose en explorant un programme
/// inconnu : qu'appelle-t-il, quand sort-il d'ici, que demande-t-il au
/// noyau ? `nextcall`, `nextret` et `nextsyscall` chez pwndbg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UntilKind {
    Call,
    Return,
    Syscall,
}

impl UntilKind {
    /// Cette instruction est-elle de celles qu'on cherche ?
    fn matches(self, instr: &namsdec_disasm::Decoded) -> bool {
        use namsdec_disasm::FlowControl;
        match self {
            // L'appel indirect compte autant que l'autre : `call rax`
            // est précisément celui qu'on veut voir arriver.
            UntilKind::Call => matches!(
                instr.flow_control(),
                FlowControl::Call | FlowControl::IndirectCall
            ),
            UntilKind::Return => instr.flow_control() == FlowControl::Return,
            UntilKind::Syscall => instr.mnemonic() == namsdec_disasm::Mnemonic::Syscall,
        }
    }
}

/// Ce qu'une pastille de fenêtre demande au système.
///
/// La fermeture n'y est pas : elle passe par la garde du travail non
/// enregistré, comme « Quitter ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowCommand {
    Minimize,
    Maximize,
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

/// Ce que l'onglet Mémoire suit, pas à pas.
///
/// Sans cela, regarder la pile pendant qu'on avance demande de retaper
/// l'adresse à chaque instruction. pwndbg résout la même chose en
/// réaffichant son contexte à chaque arrêt ; ici, on choisit ce qu'on
/// veut voir bouger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemFollow {
    /// L'adresse reste où on l'a mise.
    Fixed,
    Rsp,
    Rip,
}

impl MemFollow {
    const ALL: [MemFollow; 3] = [MemFollow::Fixed, MemFollow::Rsp, MemFollow::Rip];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            MemFollow::Fixed => s.memory_follow_none,
            MemFollow::Rsp => "RSP",
            MemFollow::Rip => "RIP",
        }
    }

    fn id(self) -> &'static str {
        match self {
            MemFollow::Fixed => "fixed",
            MemFollow::Rsp => "rsp",
            MemFollow::Rip => "rip",
        }
    }

    fn by_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|kind| kind.id() == id)
            .unwrap_or(MemFollow::Fixed)
    }

    /// L'adresse à regarder maintenant, alignée sur seize octets pour
    /// que le vidage ne danse pas d'une colonne à l'autre.
    fn address(self, regs: &namsdec_core::Registers) -> Option<u64> {
        let value = match self {
            MemFollow::Fixed => return None,
            MemFollow::Rsp => regs.get64(Reg64::Rsp),
            MemFollow::Rip => regs.rip,
        };
        Some(value & !0xf)
    }
}

/// La base dans laquelle la calculatrice lit et écrit.
///
/// Elle gouverne les deux : le grand résultat est écrit dedans, et un
/// chiffre tapé au clavier de l'écran y entre avec le préfixe qui va
/// avec. C'est ce que fait la calculatrice de GNOME, et c'est ce qu'on
/// attend — choisir « hexadécimal » puis taper `1f` doit donner
/// trente et un, non mille neuf cent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CalcBase {
    Hex,
    Dec,
    Oct,
    Bin,
}

impl CalcBase {
    const ALL: [CalcBase; 4] = [CalcBase::Hex, CalcBase::Dec, CalcBase::Oct, CalcBase::Bin];

    fn id(self) -> &'static str {
        match self {
            CalcBase::Hex => "hex",
            CalcBase::Dec => "dec",
            CalcBase::Oct => "oct",
            CalcBase::Bin => "bin",
        }
    }

    fn by_id(id: &str) -> Self {
        CalcBase::ALL
            .into_iter()
            .find(|b| b.id() == id)
            .unwrap_or(CalcBase::Hex)
    }

    fn label(self, s: &Strings) -> &'static str {
        match self {
            CalcBase::Hex => s.reading_hex,
            CalcBase::Dec => s.reading_unsigned,
            CalcBase::Oct => s.reading_octal,
            CalcBase::Bin => s.calculator_base_binary,
        }
    }

    /// Ce qu'on écrit devant un nombre pour que l'évaluateur le lise
    /// dans cette base. Rien pour le décimal, qui est le défaut de tout
    /// le monde.
    fn prefix(self) -> &'static str {
        match self {
            CalcBase::Hex => "0x",
            CalcBase::Dec => "",
            CalcBase::Oct => "0o",
            CalcBase::Bin => "0b",
        }
    }

    /// Ce chiffre a-t-il un sens dans cette base ?
    ///
    /// Sert à éteindre les touches plutôt qu'à les laisser produire une
    /// expression que l'évaluateur refusera : un clavier qui propose `9`
    /// en binaire ment sur ce qu'on peut taper.
    fn accepte(self, c: char) -> bool {
        let valeur = match c.to_ascii_lowercase() {
            c if c.is_ascii_digit() => c as u32 - '0' as u32,
            c if ('a'..='f').contains(&c) => c as u32 - 'a' as u32 + 10,
            _ => return false,
        };
        valeur
            < match self {
                CalcBase::Hex => 16,
                CalcBase::Dec => 10,
                CalcBase::Oct => 8,
                CalcBase::Bin => 2,
            }
    }

    /// Une valeur écrite dans cette base, préfixe compris.
    fn écrire(self, valeur: u64) -> String {
        match self {
            CalcBase::Hex => format!("{valeur:#x}"),
            CalcBase::Dec => valeur.to_string(),
            CalcBase::Oct => format!("{valeur:#o}"),
            CalcBase::Bin => format!("{valeur:#b}"),
        }
    }
}

/// Sur combien de bits la calculatrice montre son résultat.
///
/// Un `and eax, 0x1f` travaille sur trente-deux bits ; lire le résultat
/// sur soixante-quatre fait compter des zéros qui ne veulent rien dire.
/// La largeur masque l'affichage — elle ne change pas le calcul, qui se
/// fait toujours sur soixante-quatre bits, comme le processeur.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CalcWidth {
    W8,
    W16,
    W32,
    W64,
}

impl CalcWidth {
    const ALL: [CalcWidth; 4] = [CalcWidth::W8, CalcWidth::W16, CalcWidth::W32, CalcWidth::W64];

    fn bits(self) -> u32 {
        match self {
            CalcWidth::W8 => 8,
            CalcWidth::W16 => 16,
            CalcWidth::W32 => 32,
            CalcWidth::W64 => 64,
        }
    }

    fn id(self) -> String {
        self.bits().to_string()
    }

    fn by_id(id: &str) -> Self {
        CalcWidth::ALL
            .into_iter()
            .find(|w| w.id() == id)
            .unwrap_or(CalcWidth::W64)
    }

    /// Le masque de cette largeur. `u64::MAX` pour soixante-quatre :
    /// `1 << 64` déborderait.
    fn mask(self) -> u64 {
        match self {
            CalcWidth::W64 => u64::MAX,
            autre => (1u64 << autre.bits()) - 1,
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
    /// Où le programme a passé son temps : les lignes classées par
    /// nombre de passages. La bande de chaleur dans la gouttière dit
    /// « c'est chaud par là » ; ce classement dit laquelle est la plus
    /// chaude, de combien, et y emmène d'un clic.
    Profile,
}

impl ConsoleTab {
    const ALL: [ConsoleTab; 3] = [
        ConsoleTab::Output,
        ConsoleTab::Diagnostics,
        ConsoleTab::Profile,
    ];

    fn label(self, s: &Strings) -> &'static str {
        match self {
            ConsoleTab::Output => s.tab_output,
            ConsoleTab::Diagnostics => s.tab_diagnostics,
            ConsoleTab::Profile => s.tab_profile,
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
    ///
    /// De quoi retrouver la phrase, et non la phrase : celle-ci était
    /// figée dans la langue du moment où la note avait été posée, si
    /// bien qu'après un changement de langue la barre de statut gardait
    /// l'ancienne — du français dans une interface anglaise, et
    /// l'inverse. On garde le chemin vers le texte ; il se lit à
    /// l'affichage, dans la langue d'alors.
    Note(fn(&Strings) -> &'static str),
    /// Un nombre et ce qu'il compte, la phrase venant des textes : le
    /// nombre ne se traduit pas, la phrase si.
    Counted(usize, fn(&Strings) -> &'static str),
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

/// Un fichier ouvert, tel qu'on l'a laissé.
///
/// namsdec ne tient qu'un seul jeu de champs de travail — la source,
/// l'émulateur, les points d'arrêt — parce que tout, de la gouttière au
/// panneau latéral, les lit directement. Passer d'un fichier à l'autre
/// range donc l'état courant ici et déplie celui d'en face : le
/// programme retrouve son curseur, ses points d'arrêt, et l'exécution
/// arrêtée au pas où on l'avait laissée.
struct OpenTab {
    path: Option<PathBuf>,
    /// La date qu'avait le fichier quand on l'a lu ou écrit. C'est elle
    /// qui dit, au retour, qu'un autre programme est passé par là.
    file_mtime: Option<std::time::SystemTime>,
    source: String,
    saved_source: String,
    cursor_byte: usize,
    cursor_pos: (usize, usize),
    breakpoint_lines: BTreeSet<u32>,
    breakpoint_conditions: BTreeMap<u32, String>,
    assemble_state: Option<AssembleState>,
    emulator: Option<Box<dyn Backend>>,
    diagnostics: Vec<Diagnostic>,
    current_exercise: Option<String>,
    program_reads: bool,
    program_writes: bool,
}

impl OpenTab {
    /// Le nom qui s'écrit sur l'onglet : celui du fichier, le titre de
    /// l'exercice s'il n'y a pas de fichier, et « sans titre » sinon.
    fn label(&self, s: &Strings, packs: &[namsdec_exercises::Pack], lang: Lang) -> String {
        tab_label(
            self.path.as_deref(),
            self.current_exercise.as_deref(),
            s,
            packs,
            lang,
        )
    }

    fn modifié(&self) -> bool {
        self.source != self.saved_source
    }
}

struct AssembleState {
    output: namsdec_asm::AssembleOutput,
    disasm: Vec<DecodedInstruction>,
    addr_to_line: BTreeMap<u64, u32>,
    /// Ce programme vient-il d'un binaire ouvert, plutôt que d'un source
    /// qu'on a écrit ?
    ///
    /// La différence compte à deux endroits. Réassembler un listing issu
    /// d'un binaire produit un programme qui n'a plus ses données, ses
    /// relocations ni sa table de liaison : il part dans le décor à la
    /// première instruction qui lit quelque chose. Et un binaire lié
    /// dynamiquement ne peut pas s'exécuter ici, faute des fonctions
    /// qu'il emprunte.
    from_binary: Option<LoadedBinary>,
}

/// Ce qu'on sait d'un binaire ouvert, et qui change ce qu'on peut en
/// faire.
#[derive(Clone, Default)]
struct LoadedBinary {
    /// Les bibliothèques qu'il réclame au chargement. Non vide : il ne
    /// tournera pas ici.
    needed: Vec<String>,
    /// Le chargeur qu'il demande, quand il en demande un.
    interpreter: Option<String>,
    /// Le nom sous lequel le programme se voit lui-même — `argv[0]`, et
    /// ce qu'il cite quand il se plaint.
    name: String,
    /// Les bornes posées à la place des fonctions qu'une bibliothèque
    /// aurait dû fournir : adresse -> nom. Sauter sur l'une d'elles est
    /// une erreur qui sait dire qui l'on voulait appeler.
    externals: BTreeMap<u64, String>,
    /// Ce que le chargeur a fait, pour pouvoir le raconter.
    load: loader::Report,
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
    /// Le programme chargé peut-il lire son entrée, peut-il écrire sur sa
    /// sortie ? Lu une fois dans son code — un `syscall` précédé du
    /// numéro 0 ou 1 —, et non deviné à chaque image.
    ///
    /// La console s'en sert pour ne pas proposer un champ de saisie à un
    /// programme qui ne lira jamais, et pour dire « ce programme n'écrit
    /// rien » au lieu de « le programme n'a rien écrit », qui laisse
    /// croire à un raté.
    program_reads: bool,
    program_writes: bool,
    /// Texte en cours de saisie dans la console, pas encore envoyé au
    /// programme. Donnée de session et non réglage : elle n'a aucun sens
    /// hors du programme qui tourne, et n'a donc rien à faire dans
    /// `settings.json`.
    console_input: String,
    /// Les arguments passés au programme, tels qu'on les taperait après
    /// son nom dans un terminal.
    ///
    /// Beaucoup de programmes — et la plupart des crackmes — attendent
    /// ce qu'on leur donne là plutôt que sur l'entrée standard. Découpés
    /// à l'espace, les guillemets groupant ce qui doit rester ensemble.
    /// Le chemin du binaire ouvert, quand c'en est un.
    ///
    /// `current_file` reste vide pour un binaire, exprès. Celui-ci sert
    /// à le retrouver au lancement suivant, et à rien d'autre.
    binary_path: Option<PathBuf>,
    /// Les noms et commentaires du binaire ouvert, et son empreinte.
    ///
    /// L'empreinte est celle du contenu : un fichier déplacé ou renommé
    /// retrouve ses notes, un binaire recompilé n'hérite pas de celles
    /// d'un autre.
    notes: notes::Notes,
    notes_fingerprint: Option<String>,
    program_args: String,
    /// Les variables d'environnement ajoutées, une par ligne.
    ///
    /// Un programme qui lit `getenv("SECRET")` ne trouvait rien : le
    /// chargeur écrivait cinq variables en dur et rien d'autre.
    program_env: String,
    /// La hauteur qu'a prise la zone de saisie à la dernière image.
    ///
    /// On la mesure au lieu de la deviner : elle porte selon les cas une
    /// ligne ou six — la mention d'attente, les arguments, le bloc des
    /// variables d'environnement une fois déplié —, et toute estimation
    /// se trouve démentie dès qu'on ouvre quelque chose. Le panneau était
    /// alors trop court, et le champ passait sous la barre de statut.
    console_input_height: Option<f32>,
    /// Le bloc des variables d'environnement est-il déplié ?
    ///
    /// Tenu ici plutôt que laissé au repliable : sa hauteur entre dans
    /// celle que la console doit réserver, et l'on doit pouvoir la
    /// connaître sans fouiller l'état interne d'egui.
    console_env_open: bool,
    /// L'utilisateur a repris la main sur le champ d'entrée par Échap.
    ///
    /// Tant que le programme attend, le champ réclame le focus à chaque
    /// frame — sans quoi il faudrait le chercher à la souris. Mais cela
    /// confisquait du même coup toutes les touches de l'éditeur : « i »
    /// ne passait plus en Insertion, il s'écrivait dans le champ. Ce
    /// drapeau est la porte de sortie : posé par Échap, levé dès qu'une
    /// nouvelle attente commence ou qu'on revient dans le champ de son
    /// plein gré.
    console_input_released: bool,
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
    /// Le conseil qui accompagne la dernière erreur d'exécution.
    ///
    /// Une chaîne possédée, et non statique : certains conseils nomment
    /// ce sur quoi on vient de buter — la fonction de bibliothèque qu'on
    /// appelait — et ce nom vient du fichier ouvert.
    error_advice: Option<String>,
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
    /// La base dans laquelle la calculatrice lit et écrit.
    /// Le module que la fenêtre d'analyse montre, ou une chaîne vide
    /// pour les montrer tous.
    analysis_module: String,
    /// Ce qu'on tape dans la boîte de saisie.
    ///
    /// Son propre tampon, distinct de celui de la ligne de console. Les
    /// deux champs partageaient la même chaîne, et deux widgets qui
    /// écrivent dans un même texte se disputent le focus et se
    /// contredisent : ce qu'on tapait dans l'un disparaissait de
    /// l'autre. Chacun le sien, et l'envoi part de celui qui a servi.
    input_dialog_text: String,
    /// La boîte doit-elle prendre le clavier à la prochaine image ?
    ///
    /// Une fois, à l'ouverture — et non à chaque image : demander le
    /// focus en boucle empêche Échap de le rendre, et la boîte gardait
    /// alors le clavier pour toujours.
    input_dialog_focus: bool,
    /// La boîte de saisie est-elle ouverte ?
    ///
    /// Elle s'ouvre d'elle-même quand le programme se met à attendre, et
    /// se referme dès qu'on lui a répondu.
    show_input_dialog: bool,
    /// Le programme attendait-il à l'image précédente ?
    ///
    /// C'est le *passage* à l'attente qui ouvre la boîte, non l'attente
    /// elle-même : sans cela, la refermer la rouvrirait aussitôt.
    was_waiting_for_input: bool,
    calc_base: CalcBase,
    /// Sur combien de bits elle montre son résultat.
    calc_width: CalcWidth,
    /// La longueur du texte à la frame précédente.
    ///
    /// Sert à distinguer les deux sortes de frames lentes : celle d'une
    /// mise en page qu'on paie une fois, et celle qui reviendrait à
    /// chaque image.
    last_layout_len: usize,
    /// Les fenêtres qui étaient ouvertes à la frame précédente.
    ///
    /// Sert à reconnaître celles qui viennent de s'ouvrir, pour les
    /// poser devant. egui empile les zones dans l'ordre où on les
    /// touche, et une fenêtre qu'on vient de créer n'a jamais été
    /// touchée : elle se retrouvait derrière celle d'où elle part.
    windows_open: BTreeSet<&'static str>,
    /// Ce qu'on cherche dans la mémoire, tel qu'on l'a tapé.
    ///
    /// Pas un réglage : une saisie de travail, comme l'adresse juste
    /// au-dessus. Elle ne va donc pas dans `settings.json` — un champ de
    /// recherche qui revient rempli de la question de l'avant-veille est
    /// une nuisance, pas un service.
    mem_find_input: String,
    /// Les adresses trouvées au dernier « Chercher ».
    mem_find_hits: Vec<u64>,
    /// A-t-on cherché ? Sans cela, « rien trouvé » s'afficherait avant
    /// qu'on ait rien demandé.
    mem_find_done: bool,
    /// L'adresse effectivement montrée par l'onglet Mémoire. Distincte de
    /// la saisie : tant que celle-ci n'est pas une adresse valide et
    /// validée, le vidage reste là où il était.
    mem_addr: u64,
    /// Comment l'onglet Mémoire lit les octets. Réglage utilisateur, donc
    /// enregistré dans `settings.json`.
    memory_format: MemoryFormat,
    /// Ce que l'onglet Mémoire suit. Réglage utilisateur, donc
    /// enregistré dans `settings.json`.
    memory_follow: MemFollow,
    /// L'adresse où s'arrêter pour un « exécuter jusqu'ici » : un point
    /// d'arrêt posé le temps d'une course, et retiré dès qu'elle
    /// s'achève. Il n'appartient pas à l'utilisateur, il ne doit donc pas
    /// survivre à ce qu'il a servi à faire.
    run_to: Option<u64>,
    /// La course « jusqu'au prochain appel » en cours, s'il y en a une.
    /// Comme `run_to`, elle a posé des points d'arrêt qui ne sont pas
    /// ceux de l'utilisateur, et qu'il faut retirer en arrivant.
    run_until: Option<UntilKind>,
    /// La ligne où poser le curseur au prochain rendu de l'éditeur :
    /// cliquer un diagnostic doit y mener, sans quoi lire le message et
    /// retrouver l'endroit restent deux gestes.
    goto_line: Option<u32>,
    /// Un curseur à replacer, en octets, à la prochaine image.
    ///
    /// Comme `goto_line`, et pour la même raison : ce qui change la
    /// source ne tient pas toujours le contexte egui dont dépend l'état
    /// du champ de texte. À la différence de `goto_line`, la colonne est
    /// gardée — dupliquer une ligne pour se retrouver à son début
    /// obligerait à y revenir à la main.
    pending_cursor: Option<usize>,
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
    /// L'icône de l'application, même histoire.
    icon_texture: Option<egui::TextureHandle>,
    /// La recherche de mise à jour en cours, s'il y en a une : elle se
    /// fait sur un thread, parce qu'une requête réseau dans la boucle de
    /// rendu fige la fenêtre le temps du voyage.
    update_check: Option<mpsc::Receiver<Result<namsdec_update::Release, String>>>,
    /// Le module d'installation d'outils : éteint par défaut, et plus
    /// franchement encore que les autres.
    ///
    /// Les autres modules affichent ; celui-ci **télécharge et exécute du
    /// code venu d'Internet**. C'est une décision, pas une commodité :
    /// elle se prend une fois, en connaissance de cause, et se défait de
    /// la même façon.
    show_installer_module: bool,
    /// La recette en cours d'exécution, et ce qu'elle a donné.
    install_job: Option<InstallJob>,
    /// L'outil dont on regarde la recette, avant de rien lancer.
    install_plan: Option<String>,
    /// Ce que la dernière installation a écrit, gardé après sa fin.
    ///
    /// Le travail tient un journal ligne à ligne, mais il disparaissait
    /// avec lui : on apprenait « l'étape 2 a échoué (code 128) » sans la
    /// phrase de git qui disait pourquoi — la seule chose utile.
    install_log: Vec<String>,
    /// Le module de décompilation : éteint par défaut.
    ///
    /// Il ne s'agit pas d'un affichage de plus, mais de la **permission
    /// de lancer un programme extérieur**. On ne l'accorde pas en
    /// silence : la case existe, elle est décochée, et rien ne s'exécute
    /// tant qu'on n'a pas cliqué « Lancer ».
    show_decompiler_module: bool,
    /// La fenêtre est-elle ouverte ?
    show_decompiler: bool,
    /// Les outils installés dans le dossier des décompilateurs.
    decompilers: Vec<namsdec_disasm::plugin::Plugin>,
    /// L'identifiant de celui qu'on a choisi.
    decompiler: String,
    /// Le travail en cours, et ce qu'il a rendu.
    decompile_job: Option<mpsc::Receiver<Result<String, String>>>,
    /// Ce qui tourne à côté, pour la barre d'état et le titre de la
    /// fenêtre : un seul travail à la fois suffit à ce que namsdec fait.
    job: Option<BackgroundJob>,
    /// Le titre posé en dernier, pour ne pas le réécrire à chaque image :
    /// un `ViewportCommand` par frame réveille le gestionnaire de
    /// fenêtres pour rien.
    window_title: Option<String>,
    decompile_result: Option<Result<String, String>>,
    /// Le texte rendu, découpé une fois pour toutes.
    ///
    /// RetDec rend deux millions de caractères pour un petit crackme.
    /// En un seul bloc de texte, egui remet tout en page à chaque image
    /// et la fenêtre se fige : on découpe, et l'on ne peint que les
    /// lignes qui tiennent à l'écran.
    decompile_lines: Vec<String>,
    /// Les lignes qui ouvrent une fonction : où sauter, et sous quel nom.
    decompile_functions: Vec<(usize, String)>,
    /// Ce qu'on cherche dans le texte rendu.
    decompile_filter: String,
    /// Le filtre déjà appliqué, et les lignes qu'il retient : recalculer
    /// soixante mille lignes à chaque image se sent sous les doigts.
    decompile_filter_applied: String,
    decompile_visible: Vec<usize>,
    /// La ligne où sauter à la prochaine image.
    decompile_goto: Option<usize>,
    /// Ce qui a été décompilé : de quoi dire « ce texte parle d'un autre
    /// programme » quand on en a changé depuis.
    decompiled_from: Option<String>,
    /// Ce que la dernière recherche a répondu. Gardé jusqu'à la
    /// fermeture de la boîte : on clique, on lit, on referme.
    update_result: Option<Result<namsdec_update::Verdict, String>>,
    keybindings: KeyBindings,
    /// Le jeu de raccourcis dont on part — vim ou éditeur ordinaire.
    ///
    /// Les deux sont dans le binaire. Celui-ci dit seulement lequel sert
    /// de socle ; ce que l'utilisateur a déplacé lui-même vient par-dessus.
    keymap_preset: keybindings::Preset,
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
    /// La calculatrice est-elle ouverte, et sur quoi ?
    ///
    /// L'expression survit à la fermeture de la fenêtre : on la rouvre
    /// pour reprendre le calcul qu'on avait laissé, non pour le retaper.
    show_calculator: bool,
    calc_input: String,
    /// La fenêtre était-elle déjà ouverte à l'image précédente ?
    ///
    /// Sans ce témoin, le champ redemanderait le focus à chaque image et
    /// l'on ne pourrait plus rien cliquer d'autre dans la fenêtre.
    calculator_was_open: bool,
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
    /// La fenêtre du déroulement — le graphe des blocs du programme.
    show_flow: bool,
    /// Le grossissement du dessin, réglé à la molette.
    /// Les fenêtres internes qu'on a agrandies.
    ///
    /// Une `egui::Window` n'a pas les pastilles du système : elle se
    /// traîne et se redimensionne à la main, ce qui est long quand on
    /// veut simplement tout voir. Un identifiant par fenêtre plutôt
    /// qu'un booléen par fenêtre : le mécanisme sert à toutes, et
    /// n'ajoute rien à la structure quand on en ouvre une de plus.
    maximized_windows: std::collections::BTreeSet<&'static str>,
    /// N'afficher que la fonction où l'on se trouve.
    ///
    /// Vrai par défaut : sur un binaire ordinaire le graphe entier
    /// compte des milliers de blocs, où l'on ne trouve rien. La fonction
    /// courante est ce qu'on regarde neuf fois sur dix.
    flow_current_only: bool,
    flow_zoom: f32,
    /// Où l'on regarde dans le graphe, en pixels depuis son coin haut
    /// gauche. Tenu ici plutôt que laissé à la zone défilante : le
    /// glisser à la souris et les boutons d'ajustement le déplacent
    /// aussi, et il faut qu'ils parlent tous du même point.
    flow_offset: egui::Vec2,
    /// Demande de ramener tout le graphe dans la fenêtre. Consommée au
    /// dessin suivant, quand on connaît enfin la place disponible.
    flow_fit: bool,
    /// L'adresse dont on demande « comment y arrive-t-on ? ».
    flow_goal: String,
    /// La fenêtre du motif cyclique — celle qui répond à « de combien ça
    /// déborde ».
    show_cyclic: bool,
    /// La longueur demandée, et la valeur qu'on cherche à replacer. Des
    /// saisies de travail, comme l'adresse du vidage mémoire : elles ne
    /// vont pas dans `settings.json`, où elles n'auraient aucun sens au
    /// lancement suivant.
    cyclic_len: String,
    cyclic_find: String,
    /// Ce qu'on cherche parmi les exercices. Trente-six étapes se
    /// parcourent mal quand on sait déjà ce qu'on veut revoir.
    exercise_query: String,
    /// Ce qu'on cherche dans la mémoire, et sous quelle forme. Une
    /// saisie de travail, comme l'adresse du vidage.
    mem_search: String,
    mem_search_kind: SearchKind,
    /// Le registre en cours d'édition, et ce qui est tapé dedans.
    ///
    /// Poser une valeur à la main est un geste d'apprentissage à part
    /// entière : « et si RCX valait 1 ? » se répond en une seconde au
    /// lieu de se réécrire. `set $rax = …` chez gdb.
    editing_register: Option<(&'static str, String)>,
    /// Ce que les pastilles ont demandé, à exécuter une fois la barre de
    /// menus rendue : un `ViewportCommand` envoyé au milieu du rendu
    /// arriverait avant que la frame soit finie.
    window_command: Option<WindowCommand>,
    /// Les derniers fichiers ouverts, le plus récent en tête.
    recent_files: Vec<PathBuf>,
    /// Les exercices réussis, par identifiant complet.
    ///
    /// Retenu sur le disque : un parcours se fait en plusieurs fois, et
    /// ce qu'on a déjà réussi ne se redemande pas.
    exercises_done: BTreeSet<String>,
    /// Le meilleur compte d'instructions par exercice : le score.
    exercises_best: BTreeMap<String, u64>,
    /// La légende de la barre de chaleur, sous l'éditeur.
    ///
    /// Utile le temps d'apprendre à lire les teintes, superflue ensuite :
    /// elle se coupe dans les préférences.
    heat_legend: bool,
    /// La barre de titre du système, plutôt que celle qu'on dessine.
    ///
    /// Réglage utilisateur, donc dans `settings.json` — et appliqué à
    /// chaud : on ne redémarre pas une application pour retrouver son
    /// cadre.
    system_titlebar: bool,
    /// Les décorations telles qu'elles sont en ce moment, pour ne
    /// renvoyer la commande au système que lorsque le réglage change.
    decorations_applied: Option<bool>,
    /// La hauteur de la console quand l'utilisateur l'a réglée lui-même.
    ///
    /// `None` — le cas par défaut — veut dire « qu'elle prenne la place
    /// de ce qu'elle a à dire » : deux lignes de sortie n'ont pas besoin
    /// du quart de l'écran. Dès qu'on tire la poignée, sa hauteur est
    /// retenue et l'ajustement cesse : on ne lutte pas contre la main
    /// de celui qui travaille.
    console_height: Option<f32>,
    /// La part du panneau latéral laissée au contenu de l'onglet, quand
    /// la poignée a été tirée. `None` : elle suit ce que le bandeau
    /// laisse, sans jamais descendre sous [`SIDEBAR_TAB_MIN`].
    sidebar_tab_height: Option<f32>,
    /// La hauteur qu'a prise le bandeau — prochaine instruction, dernier
    /// pas, état du processeur — à l'image précédente. Mesurée, parce
    /// qu'on doit poser la part de l'onglet *avant* de dessiner le
    /// bandeau, et qu'elle change avec ce qui est replié.
    sidebar_banner_height: f32,
    /// La hauteur qu'a réellement prise la part de l'onglet à la
    /// dernière image : celle qu'on fige quand on cesse de la calculer.
    sidebar_tab_last: f32,
    /// Ce qu'on s'apprête à écrire dans la mémoire, à la main.
    ///
    /// Le pendant de l'édition d'un registre : poser une valeur dans un
    /// tampon pour voir ce que le programme en fera. `patch` chez
    /// pwndbg.
    mem_poke: String,
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
    /// **hors de `settings.json`** : egui garde
    /// normalement l'état des en-têtes repliables d'une session à
    /// l'autre, si bien qu'un repli fait une fois — pour lire un vidage
    /// mémoire, par exemple — rouvrait l'application sur des registres
    /// cachés, sans qu'on sache pourquoi. Il se replie toujours à la
    /// main, le temps de la session.
    cpu_state_open: bool,
    /// Le bloc de la prochaine instruction est-il déployé ? Même règle
    /// que ci-dessus : l'état d'une session, jamais celui du disque.
    next_instruction_open: bool,
    /// Et son détail — la fiche, les opérandes résolus, les arguments
    /// d'un appel. Fermé au départ, ouvert de lui-même devant ce qui
    /// mérite d'être expliqué : un appel, un saut conditionnel, un
    /// appel système.
    next_detail_open: bool,
    /// L'adresse à laquelle ce détail s'est ouvert tout seul, pour ne le
    /// faire qu'une fois par instruction — sinon replier n'aurait aucun
    /// effet.
    next_detail_addr: Option<u64>,
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
    /// Les autres fichiers ouverts, rangés tels qu'on les a laissés.
    /// L'onglet actif n'y est pas : c'est l'état de travail lui-même.
    tabs: Vec<OpenTab>,
    /// La date du fichier ouvert, telle qu'on l'a lue ou écrite.
    ///
    /// Un fichier qu'on édite aussi ailleurs — un autre éditeur, un
    /// `sed`, un `git checkout` — change sous l'application sans que rien
    /// ne le dise : on écrasait alors le travail de l'autre au premier
    /// Ctrl+S, ou l'on assemblait une version périmée.
    file_mtime: Option<std::time::SystemTime>,
    /// Le fichier a changé sur le disque et le tampon, lui, n'est pas
    /// enregistrable sans écraser : on le signale au lieu de choisir.
    disk_changed: bool,
    /// La fenêtre avait-elle le focus à l'image précédente ? C'est le
    /// passage de « non » à « oui » qui déclenche la vérification.
    had_focus: bool,
    /// Vrai le temps de rouvrir les fichiers de la séance précédente.
    ///
    /// Rouvrir n'est pas ouvrir : la liste des fichiers récents garde
    /// l'ordre dans lequel *on* les a demandés, et non celui dans lequel
    /// l'application les a rechargés au démarrage.
    restoring_session: bool,
    /// Le rang de l'onglet actif parmi les autres — c'est-à-dire
    /// l'endroit où l'état courant se range quand on en change.
    active_tab_index: usize,
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
    /// Les paires se ferment-elles toutes seules ?
    auto_pairs: bool,
    /// La fiche de l'instruction au survol du code. Distincte des
    /// infobulles ordinaires : celle-ci est grande, et s'ouvre en
    /// écrivant.
    hover_help: bool,
    /// Ce que l'autocomplétion propose en ce moment, recalculé à chaque
    /// frappe. Rien à enregistrer : c'est l'état d'un instant.
    suggestions: Option<completion::Suggestions>,
    /// Ce qu'on suit à l'écran, et où ça revient.
    ///
    /// Vidé par Échap. Recalculé à chaque changement de la source : les
    /// étendues sont des positions en octets, et taper une lettre plus
    /// haut décalerait tout ce qui suit — surligner alors les anciennes
    /// désignerait le milieu des mots.
    tracked: Option<occurrences::Tracked>,
    tracked_spans: Vec<std::ops::Range<usize>>,
    /// La source telle qu'elle était au dernier calcul.
    tracked_source_len: usize,
    /// Ce qui est sélectionné dans l'éditeur, en octets.
    ///
    /// Relevé pendant le rendu, comme `cursor_byte` : les actions du
    /// clavier n'ont pas le contexte egui où vit l'état du champ.
    editor_selection: Option<std::ops::Range<usize>>,
    /// Le nouveau nom en cours de frappe, quand on renomme.
    rename: Option<String>,
    rename_just_opened: bool,
    /// La note en cours d'écriture : son genre, son adresse, son texte.
    note_edit: Option<(NoteKind, u64, String)>,
    note_just_opened: bool,
    /// Une étendue à sélectionner à la prochaine image.
    ///
    /// Comme `pending_cursor` : ce qui décide n'a pas le contexte egui
    /// où vit l'état du champ de texte.
    pending_selection: Option<std::ops::Range<usize>>,
    /// Un `g` frappé attend-il son second ? C'est `gg` de vim.
    g_pending: bool,
    /// La proposition mise en avant, dans la liste affichée.
    ///
    /// Zéro le plus souvent : on tape, et la première est la bonne. Les
    /// flèches servent aux fois où elle ne l'est pas — et sans elles, on
    /// ne pouvait accepter que la première, ce qui rendait la liste
    /// décorative dès qu'elle en montrait plusieurs.
    completion_index: usize,
    /// La liste vient-elle d'être écartée par Échap ?
    completion_dismissed: bool,
    /// Les touches que la liste de propositions vient de prendre pour
    /// elle, relevées **avant** l'éditeur.
    ///
    /// Tab, les flèches et Échap ont un sens dans l'éditeur et un autre
    /// dans la liste. Les réclamer après coup ne marcherait pas : le
    /// `TextEdit` les aurait déjà appliquées — Tab aurait posé une
    /// tabulation, les flèches auraient déplacé le curseur.
    completion_keys: CompletionKeys,
    /// `Alt` était-il enfoncé à la frame précédente ?
    ///
    /// egui n'émet pas d'évènement pour une touche de modification seule
    /// — elle n'apparaît que dans l'état des modificateurs. C'est donc le
    /// *passage* de relâché à enfoncé qu'on guette, et lui seul :
    /// autrement, garder Alt enfoncé accepterait une proposition à chaque
    /// frame.
    alt_was_down: bool,
    /// `Alt` vient-il d'être enfoncé, à cette frame ?
    ///
    /// Relevé une fois et partagé : la liste de propositions le prend en
    /// premier, et ce qui reste va à la correction de code. Sans ce
    /// partage, la même frappe accepterait une proposition *et*
    /// corrigerait la ligne.
    alt_pressed: bool,
    /// La correction proposée pour la ligne du curseur, et de quoi
    /// savoir si elle est encore à jour.
    ///
    /// Gardée d'une frame à l'autre parce que la calculer relit les
    /// étiquettes et les macros de tout le fichier : sur un binaire
    /// désassemblé, ce n'est pas ce qu'on refait soixante fois par
    /// seconde.
    fix: Option<FixCache>,
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
    /// Choisir où écrire le programme assemblé, tel que `ld` l'a produit.
    ExportBinary,
    /// Choisir où écrire le vidage hexadécimal du fichier chargé.
    ExportDump,
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
    ShowCalculator,
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
    /// Aller à une étiquette du programme, par son nom.
    GoLine(u32),
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
        // Les gestes d'exécution que les menus portent aussi : la
        // palette promet « toutes les commandes », et une commande
        // qu'elle ne propose pas n'existe que pour qui connaît déjà son
        // raccourci.
        (
            s.action_step_back.to_string(),
            PaletteCommand::Do(Action::StepBack),
        ),
        (
            s.action_run_to_cursor.to_string(),
            PaletteCommand::Do(Action::RunToCursor),
        ),
        (
            s.action_run_to_call.to_string(),
            PaletteCommand::Do(Action::RunToCall),
        ),
        (
            s.action_run_to_return.to_string(),
            PaletteCommand::Do(Action::RunToReturn),
        ),
        (
            s.action_run_to_syscall.to_string(),
            PaletteCommand::Do(Action::RunToSyscall),
        ),
        (s.file_new.to_string(), PaletteCommand::Do(Action::New)),
        (s.help_about.to_string(), PaletteCommand::ShowAbout),
        (s.analysis_open.to_string(), PaletteCommand::ShowAnalysis),
        (s.vectors_open.to_string(), PaletteCommand::ShowVectors),
        (
            s.breakpoints_open.to_string(),
            PaletteCommand::ShowBreakpoints,
        ),
        (s.timeline_open.to_string(), PaletteCommand::ShowTimeline),
        (
            s.flow_open.to_string(),
            PaletteCommand::Do(Action::ShowFlow),
        ),
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
            format!("{}: {}", s.menu_view, s.tab_profile),
            PaletteCommand::GoConsoleTab(ConsoleTab::Profile),
        ),
        // Les fichiers ouverts : d'un onglet au suivant, et la fermeture
        // de celui qu'on regarde.
        (
            s.action_next_tab.to_string(),
            PaletteCommand::Do(Action::NextTab),
        ),
        (
            s.action_previous_tab.to_string(),
            PaletteCommand::Do(Action::PreviousTab),
        ),
        (
            s.action_close_tab.to_string(),
            PaletteCommand::Do(Action::CloseTab),
        ),
        (
            s.action_duplicate_line.to_string(),
            PaletteCommand::Do(Action::DuplicateLine),
        ),
        (
            s.action_highlight_matches.to_string(),
            PaletteCommand::Do(Action::HighlightMatches),
        ),
        (
            s.action_next_match.to_string(),
            PaletteCommand::Do(Action::NextMatch),
        ),
        (
            s.action_previous_match.to_string(),
            PaletteCommand::Do(Action::PreviousMatch),
        ),
        (
            s.action_rename_matches.to_string(),
            PaletteCommand::Do(Action::RenameMatches),
        ),
        (
            s.action_name_address.to_string(),
            PaletteCommand::Do(Action::NameAddress),
        ),
        (
            s.action_comment_address.to_string(),
            PaletteCommand::Do(Action::CommentAddress),
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
    // Sans condition : elle ne dépend d'aucun module, d'aucun binaire
    // chargé, et sert autant à qui écrit son premier programme qu'à qui
    // démonte un crackme.
    commands.push((s.calculator_open.to_string(), PaletteCommand::ShowCalculator));

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
    decompiler_module: bool,
    installer_module: bool,
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
    /// Les désassembleurs et décompilateurs extérieurs.
    ///
    /// Un onglet à eux, et non une section au bas des plugins : c'est là
    /// qu'on vient chercher « où sont mes outils », et une section qu'il
    /// faut trouver en faisant défiler n'existe pas.
    Tools,
}

/// Programme de départ affiché au lancement, pour ouvrir sur un exemple
/// qui fonctionne plutôt qu'un éditeur vide.
///
/// Ce n'est pas un exercice : ceux-là sont un module à part, éteint tant
/// qu'on ne le demande pas, et l'application doit s'ouvrir sur quelque
/// chose qui marche même quand il ne l'est pas.
const DEFAULT_SOURCE: &str = "\
; Bienvenue dans namsdec. Assemblez (Ctrl+Maj+B), puis avancez pas à pas.
; Un parcours de quatre-vingt-seize exercices vous attend : Préférences > Modules.

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
    /// Fermer un onglet dont le texte n'est pas enregistré.
    CloseTab(usize),
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
        // Puis l'identité du fichier : de quel octet parle-t-on ? C'est
        // la première ligne d'un compte rendu, et ce qui permet à deux
        // personnes de savoir qu'elles lisent le même fichier.
        re_registry.register(Box::new(DigestModule::default()));
        // Puis « où est quoi » : un binaire n'est pas un bloc. Savoir à
        // quel endroit on regarde change ce qu'on y cherche — une
        // entropie haute dans `.text` est normale, la même dans
        // `.rodata` annonce des données chiffrées.
        re_registry.register(Box::new(MapModule::default()));
        re_registry.register(Box::new(ProtectionsModule::default()));
        // L'entropie juste après : elle répond avant tout le reste à
        // « est-ce qu'il y a quelque chose à lire ici ? ». Sur un binaire
        // empaqueté, les modules suivants analyseraient un déchiffreur de
        // trois cents octets en croyant analyser le programme — autant le
        // savoir tout de suite.
        re_registry.register(Box::new(EntropyModule::default()));
        // Puis « par où commencer » : dans un binaire dépouillé, la
        // question précède toutes les autres. Le module retrouve le
        // `main` et découpe le code en fonctions.
        re_registry.register(Box::new(FunctionsModule::default()));
        // Puis dans l'ordre où l'on regarde un binaire inconnu : ce qu'il
        // dit (ses chaînes), ce qu'il demande au système, puis comment il
        // est découpé.
        re_registry.register(Box::new(StringsModule::default()));
        // Ce que le code *fait* de remarquable : les figures, rangées de
        // la plus rare à la plus commune. C'est ce classement qui fait
        // sortir une comparaison déguisée d'un binaire de soixante mille
        // instructions.
        re_registry.register(Box::new(PatternsModule::default()));
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
        // Le vidage en dernier : c'est là qu'on descend quand les
        // modules d'au-dessus ont désigné un endroit, et qu'on veut
        // savoir ce que ces octets-là sont vraiment.
        re_registry.register(Box::new(DumpModule::default()));

        Self {
            source: DEFAULT_SOURCE.to_string(),
            theme: DEFAULT_THEME,
            lang: Lang::Fr,
            nasm_info: namsdec_asm::find_nasm(),
            breakpoint_lines: BTreeSet::new(),
            breakpoint_conditions: BTreeMap::new(),
            assemble_state: None,
            program_reads: false,
            program_writes: false,
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
            analysis_module: String::new(),
            input_dialog_text: String::new(),
            input_dialog_focus: false,
            show_input_dialog: false,
            was_waiting_for_input: false,
            calc_base: CalcBase::Hex,
            calc_width: CalcWidth::W64,
            last_layout_len: 0,
            windows_open: BTreeSet::new(),
            mem_find_input: String::new(),
            mem_find_hits: Vec::new(),
            mem_find_done: false,
            mem_addr: BASE_ADDR,
            memory_format: MemoryFormat::Bytes,
            memory_follow: MemFollow::Fixed,
            watchpoints: Vec::new(),
            run_to: None,
            run_until: None,
            goto_line: None,
            pending_cursor: None,
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
            icon_texture: None,
            update_check: None,
            show_installer_module: false,
            install_job: None,
            install_plan: None,
            install_log: Vec::new(),
            show_decompiler_module: false,
            show_decompiler: false,
            decompilers: Vec::new(),
            decompiler: String::new(),
            decompile_job: None,
            job: None,
            window_title: None,
            decompile_result: None,
            decompile_lines: Vec::new(),
            decompile_functions: Vec::new(),
            decompile_filter: String::new(),
            decompile_filter_applied: String::new(),
            decompile_visible: Vec::new(),
            decompile_goto: None,
            decompiled_from: None,
            update_result: None,
            keybindings: KeyBindings::default(),
            keymap_preset: keybindings::Preset::default(),
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
            show_calculator: false,
            calc_input: String::new(),
            calculator_was_open: false,
            show_help: false,
            show_analysis: false,
            show_vectors: false,
            show_breakpoints: false,
            show_timeline: false,
            show_flow: false,
            maximized_windows: std::collections::BTreeSet::new(),
            flow_current_only: true,
            flow_zoom: 1.0,
            flow_offset: egui::Vec2::ZERO,
            flow_fit: false,
            flow_goal: String::new(),
            show_cyclic: false,
            // Cent, comme pwndbg : de quoi couvrir un tampon
            // d'apprentissage sans noyer la fenêtre.
            cyclic_len: "100".to_owned(),
            cyclic_find: String::new(),
            exercise_query: String::new(),
            mem_search: String::new(),
            mem_search_kind: SearchKind::Text,
            editing_register: None,
            recent_files: Vec::new(),
            exercises_done: BTreeSet::new(),
            exercises_best: BTreeMap::new(),
            heat_legend: true,
            window_command: None,
            system_titlebar: false,
            decorations_applied: None,
            console_height: None,
            sidebar_tab_height: None,
            sidebar_banner_height: 0.0,
            sidebar_tab_last: SIDEBAR_TAB_MIN,
            mem_poke: String::new(),
            show_disassembly: false,
            disassembler: PSEUDO_ID.to_owned(),
            disassembly_query: String::new(),
            disassembly_target: None,
            disassembler_plugins: load_disassembler_plugins(),
            plugin_output: None,
            timeline_notable_only: false,
            // Aucune série au départ : elles sont des fichiers, et
            // c'est le démarrage qui les lit — celle qui est livrée
            // comme celles qu'on ajoute.
            exercise_packs: Vec::new(),
            exercise_files: BTreeMap::new(),
            pack_to_remove: None,
            disabled_packs: BTreeSet::new(),
            current_exercise: None,
            settings_file: None,
            cpu_state_open: true,
            next_instruction_open: true,
            next_detail_open: false,
            next_detail_addr: None,
            console_input: String::new(),
            binary_path: None,
            notes: notes::Notes::default(),
            notes_fingerprint: None,
            program_args: String::new(),
            program_env: String::new(),
            console_input_height: None,
            console_env_open: false,
            console_input_released: false,
            resume_run_after_input: false,
            explorer: FileExplorer::default(),
            show_explorer: false,
            explorer_just_opened: false,
            explorer_focused: false,
            current_file: None,
            tabs: Vec::new(),
            file_mtime: None,
            disk_changed: false,
            had_focus: true,
            restoring_session: false,
            active_tab_index: 0,
            show_toolbar: true,
            show_sidebar: true,
            show_statusbar: true,
            show_console: true,
            show_tooltips: true,
            completion: true,
            auto_indent: true,
            auto_pairs: true,
            hover_help: true,
            suggestions: None,
            tracked: None,
            tracked_spans: Vec::new(),
            tracked_source_len: 0,
            editor_selection: None,
            rename: None,
            rename_just_opened: false,
            note_edit: None,
            note_just_opened: false,
            pending_selection: None,
            g_pending: false,
            completion_index: 0,
            completion_dismissed: false,
            completion_keys: CompletionKeys::default(),
            alt_was_down: false,
            alt_pressed: false,
            fix: None,
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
            // Le fichier de raccourcis d'abord, quand il existe : c'est
            // celui qu'on ouvre et qu'on copie d'une machine à l'autre.
            // `settings.json` garde les siens pour les versions qui ne
            // connaissaient que lui.
            keybindings: {
                let keymap = settings::load_keymap();
                if settings::keymap_path().is_some_and(|p| p.exists()) {
                    keymap.bindings()
                } else {
                    settings.keybindings()
                }
            },
            keymap_preset: settings::load_keymap().preset(),
            show_toolbar: settings.show_toolbar,
            show_sidebar: settings.show_sidebar,
            show_statusbar: settings.show_statusbar,
            show_console: settings.show_console,
            show_tooltips: settings.show_tooltips,
            completion: settings.completion,
            auto_indent: settings.auto_indent,
            auto_pairs: settings.auto_pairs,
            hover_help: settings.hover_help,
            exercises: settings.exercises,
            disabled_packs: settings.disabled_packs.iter().cloned().collect(),
            current_exercise: settings.current_exercise.clone(),
            memory_format: MemoryFormat::by_id(&settings.memory_format),
            analysis_module: settings.analysis_module.clone(),
            calc_base: CalcBase::by_id(&settings.calc_base),
            calc_width: CalcWidth::by_id(&settings.calc_width),
            memory_follow: MemFollow::by_id(&settings.memory_follow),
            console_height: settings.console_height.map(|h| h as f32),
            sidebar_tab_height: settings.sidebar_tab_height.map(|h| h as f32),
            show_re_tab: settings.re_tab,
            program_args: settings.program_args.clone(),
            program_env: settings.program_env.clone(),
            active_tab: settings
                .sidebar_tab
                .as_deref()
                .and_then(SidebarTab::from_id)
                .unwrap_or(SidebarTab::Stack),
            show_decompiler_module: settings.decompiler_module,
            show_installer_module: settings.installer_module,
            decompiler: settings.decompiler.clone(),
            sidebar_banner_height: 0.0,
            sidebar_tab_last: SIDEBAR_TAB_MIN,
            system_titlebar: settings.system_titlebar,
            heat_legend: settings.heat_legend,
            exercises_done: settings.exercises_done.iter().cloned().collect(),
            exercises_best: settings.exercises_best.iter().cloned().collect(),
            recent_files: settings.recent_files.iter().map(PathBuf::from).collect(),
            disassembler: settings.disassembler.clone(),
            explorer,
            saved_settings: settings,
            ..Default::default()
        };
        app.ensure_explorer_root();
        app.reopen_last_file();
        // Après les sources, et hors de `reopen_last_file` : celle-ci
        // rend la main dès qu'il n'y a pas de source à rouvrir, ce qui
        // est précisément le cas quand on travaillait un binaire —
        // `current_file` reste vide pour lui.
        app.restore_binary_session();
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
        self.restoring_session = true;
        self.open_file(path);
        self.restoring_session = false;
        if matches!(self.status, Status::Error(_)) {
            self.current_file = None;
            self.source = DEFAULT_SOURCE.to_owned();
            self.status = Status::Idle;
        }
        // Puis les autres fichiers de la dernière séance, dans leur
        // ordre. Ceux qui ont disparu du disque sont simplement passés :
        // une erreur au démarrage pour un fichier déplacé hier
        // n'apprendrait rien à personne.
        let autres: Vec<PathBuf> = self
            .saved_settings
            .open_files
            .iter()
            .map(PathBuf::from)
            .filter(|path| path.is_file() && Some(path.as_path()) != self.current_file.as_deref())
            .collect();
        // On revient ensuite sur celui qu'on regardait : ouvrir les
        // autres a déplacé l'onglet actif.
        let actif = self.current_file.clone();
        self.restoring_session = true;
        for path in autres {
            self.open_file(path);
        }
        self.restoring_session = false;
        if let Some(actif) = actif
            && let Some(rang) = self.tab_index_of(&actif)
        {
            self.go_to_tab(rang);
        }
    }

    /// Reprend le travail commencé sur un binaire : le binaire lui-même,
    /// puis les repères posés dessus.
    ///
    /// Les sources se rouvrent depuis longtemps ; un binaire, non — et
    /// c'est pourtant là qu'il y a du travail à perdre. Rouvrir namsdec
    /// demandait de retrouver le fichier, de le recharger, de reposer ses
    /// points d'arrêt et de retaper ses arguments.
    ///
    /// **Les repères ne reviennent que si le fichier est le même.** Ils se
    /// comptent en numéros de ligne d'un listing reconstruit à chaque
    /// ouverture : sur un binaire recompilé entre-temps, ils désigneraient
    /// d'autres instructions, et poser des points d'arrêt au hasard est
    /// pire que de ne pas en poser.
    fn restore_binary_session(&mut self) {
        let Some(chemin) = self.saved_settings.open_binary.clone().map(PathBuf::from) else {
            return;
        };
        if !chemin.is_file() {
            return;
        }
        let Ok(octets) = std::fs::read(&chemin) else {
            return;
        };
        let nom = chemin
            .file_name()
            .map(|nom| nom.to_string_lossy().into_owned())
            .unwrap_or_else(|| "programme".to_owned());
        self.load_binary(octets, &nom);
        self.binary_path = Some(chemin.clone());

        // Le fichier est-il celui qu'on avait quitté ? Taille et date :
        // deux mesures qu'on a sous la main, et qui suffisent à écarter
        // le cas qui compte — un binaire recompilé depuis.
        let taille = std::fs::metadata(&chemin).ok().map(|m| m.len());
        let date = file_mtime(&chemin)
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let même = taille == self.saved_settings.open_binary_len
            && date == self.saved_settings.open_binary_mtime;
        if !même {
            // Le dire, plutôt que de laisser croire que rien n'a été
            // perdu : quelqu'un qui a recompilé son binaire doit savoir
            // pourquoi ses points d'arrêt ont disparu.
            if !self.saved_settings.breakpoints.is_empty()
                || !self.saved_settings.watchpoints.is_empty()
            {
                self.status = Status::Note(|s| s.session_binary_changed);
            }
            return;
        }

        self.breakpoint_lines = self.saved_settings.breakpoints.iter().copied().collect();
        self.watchpoints = self
            .saved_settings
            .watchpoints
            .iter()
            .map(|(addr, len)| (*addr, *len as usize))
            .collect();
        self.watched_registers = REGISTERS
            .iter()
            .map(|(nom, _)| *nom)
            .filter(|nom| {
                self.saved_settings
                    .watched_registers
                    .iter()
                    .any(|gardé| gardé == nom)
            })
            .collect();
        self.sync_watchpoints();
    }

    /// Écrit le fichier de réglages sans attendre qu'un réglage change :
    /// appelé au démarrage pour que l'utilisateur puisse le trouver et
    /// l'éditer dès le premier lancement, avec les défauts du jour gravés
    /// dedans plutôt que de suivre en silence ceux d'une version future.
    ///
    /// Volontairement hors de `from_settings` : construire l'application
    /// ne doit pas écrire sur le disque, sinon les tests qui la
    /// construisent écraseraient la configuration de la machine.
    /// Écrit `keymap.json` dès le premier lancement, comme on le fait
    /// pour les réglages.
    ///
    /// Le fichier n'était posé qu'au premier changement de raccourci.
    /// Deux jeux vivent pourtant dans le binaire — Vim et gdb, qui est
    /// le défaut, et l'éditeur ordinaire — et rien ne le disait à qui
    /// n'ouvrait pas les Préférences : on cherchait un fichier qui
    /// n'existait pas encore pour savoir de quoi on partait.
    ///
    /// Hors de `from_settings`, pour la même raison que les réglages :
    /// construire l'application ne doit pas écrire sur le disque, sinon
    /// les tests qui la construisent écraseraient les raccourcis de
    /// l'utilisateur.
    fn save_keymap_now(&mut self) {
        if self.settings_file.is_none() {
            return;
        }
        let _ = settings::save_keymap(&settings::Keymap::from_bindings(
            self.keymap_preset,
            &self.keybindings,
        ));
    }

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
            open_binary: self
                .binary_path
                .as_ref()
                .map(|p| p.display().to_string()),
            open_binary_len: self
                .binary_path
                .as_ref()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len()),
            open_binary_mtime: self
                .binary_path
                .as_deref()
                .and_then(file_mtime)
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            breakpoints: self.breakpoint_lines.iter().copied().collect(),
            watchpoints: self
                .watchpoints
                .iter()
                .map(|(addr, len)| (*addr, *len as u32))
                .collect(),
            watched_registers: self
                .watched_registers
                .iter()
                .map(|nom| (*nom).to_owned())
                .collect(),
            program_args: self.program_args.clone(),
            program_env: self.program_env.clone(),
            sidebar_tab: Some(self.active_tab.id().to_owned()),
            show_toolbar: self.show_toolbar,
            show_sidebar: self.show_sidebar,
            show_statusbar: self.show_statusbar,
            show_console: self.show_console,
            show_tooltips: self.show_tooltips,
            completion: self.completion,
            auto_indent: self.auto_indent,
            auto_pairs: self.auto_pairs,
            hover_help: self.hover_help,
            exercises: self.exercises,
            disabled_packs: self.disabled_packs.iter().cloned().collect(),
            current_exercise: self.current_exercise.clone(),
            disassembler: self.disassembler.clone(),
            memory_format: self.memory_format.id().to_owned(),
            analysis_module: self.analysis_module.clone(),
            calc_base: self.calc_base.id().to_owned(),
            calc_width: self.calc_width.id(),
            memory_follow: self.memory_follow.id().to_owned(),
            console_height: self.console_height.map(|h| h.round() as u32),
            sidebar_tab_height: self.sidebar_tab_height.map(|h| h.round() as u32),
            re_tab: self.show_re_tab,
            decompiler_module: self.show_decompiler_module,
            installer_module: self.show_installer_module,
            decompiler: self.decompiler.clone(),
            system_titlebar: self.system_titlebar,
            heat_legend: self.heat_legend,
            exercises_done: self.exercises_done.iter().cloned().collect(),
            exercises_best: self
                .exercises_best
                .iter()
                .map(|(id, compte)| (id.clone(), *compte))
                .collect(),
            recent_files: self
                .recent_files
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            explorer_root: self.explorer.root.as_ref().map(|p| p.display().to_string()),
            current_file: self.current_file.as_ref().map(|p| p.display().to_string()),
            // Les autres onglets, dans leur ordre : au lancement
            // suivant, on retrouve les fichiers sur lesquels on
            // travaillait, et non le seul dernier.
            open_files: self
                .tabs
                .iter()
                .filter_map(|tab| tab.path.as_ref())
                .map(|p| p.display().to_string())
                .collect(),
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

    /// Ce tampon vient-il d'un binaire ouvert ?
    ///
    /// Un listing produit par le désassemblage n'est pas un source : il
    /// n'a ni les données, ni les relocations, ni la table de liaison du
    /// programme d'origine. Le réassembler donne un exécutable qui part
    /// dans le décor à la première instruction qui lit quelque chose.
    fn buffer_came_from_a_binary(&self) -> bool {
        self.assemble_state
            .as_ref()
            .is_some_and(|state| state.from_binary.is_some())
            && !self.buffer_modified()
    }

    fn do_assemble(&mut self) {
        // Réassembler un listing de binaire est presque toujours une
        // erreur de geste — un Ctrl+Maj+B par habitude. On le dit, et
        // l'on ne touche à rien : ce qui est chargé vaut mieux que ce
        // qu'on en referait.
        if self.buffer_came_from_a_binary() {
            self.status = Status::Error(strings(self.lang).binary_no_reassemble.to_owned());
            return;
        }
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
                self.diagnostics
                    .extend(namsdec_asm::lint::check(&output, self.lang == Lang::Fr));
                self.assemble_state = Some(AssembleState {
                    output,
                    disasm,
                    addr_to_line,
                    // Ce programme-ci vient d'un source : il est à nous.
                    from_binary: None,
                });
                self.note_program_io();
                // Et ce que namsdec ne saura pas faire : un appel
                // système qu'il ne simule pas arrête le programme au
                // milieu. Le dire à l'assemblage, avec sa ligne, évite
                // de chercher pourquoi l'exécution s'interrompt là.
                self.diagnostics.extend(self.unsimulated_syscalls());
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
    /// Les appels système que le programme contient et que namsdec
    /// n'exécute pas.
    ///
    /// `mmap`, `open`, `fork` : namsdec les nomme mais ne les fait pas —
    /// il n'a ni fichiers ni processus à offrir. Un programme qui en
    /// contient un s'arrête dessus, et rien dans le code ne le laisse
    /// deviner. C'est exactement le genre de chose qu'un atelier doit
    /// dire avant, pas pendant.
    fn unsimulated_syscalls(&self) -> Vec<namsdec_asm::Diagnostic> {
        let Some(state) = &self.assemble_state else {
            return Vec::new();
        };
        namsdec_re::syscalls::scan(&state.disasm)
            .into_iter()
            .filter_map(|call| {
                let number = call.number?;
                if namsdec_re::table::is_simulated(number) {
                    return None;
                }
                let nom = namsdec_re::table::syscall_name(number)?;
                let line = state.addr_to_line.get(&call.addr).copied().unwrap_or(0);
                let kind = namsdec_asm::LintKind::UnsimulatedSyscall {
                    name: nom.to_owned(),
                    number,
                };
                Some(namsdec_asm::Diagnostic {
                    line,
                    severity: namsdec_asm::Severity::Warning,
                    message: kind.message(self.lang == Lang::Fr),
                    kind: Some(kind),
                })
            })
            .collect()
    }

    /// Relit ce que le programme chargé demande au monde extérieur.
    ///
    /// Une fois par programme, et non à chaque image : remonter le
    /// numéro d'un `syscall` coûte un décodage par instruction, ce qui
    /// se paie cher sur un binaire du système.
    fn note_program_io(&mut self) {
        let (mut lit, mut écrit) = (false, false);
        if let Some(state) = &self.assemble_state {
            for call in namsdec_re::syscalls::scan(&state.disasm) {
                match call.number {
                    // Un numéro illisible d'ici — calculé, lu en
                    // mémoire — peut être n'importe lequel : on ne
                    // refuse pas la saisie à un programme qui l'attend
                    // peut-être.
                    None => {
                        lit = true;
                        écrit = true;
                    }
                    Some(0) => lit = true,
                    Some(1) => écrit = true,
                    _ => {}
                }
            }
        }
        // Ce qu'un binaire lié dynamiquement réclame ne passe par aucun
        // `syscall` : il appelle `fgets` ou `scanf`, que namsdec fournit
        // lui-même. Sans regarder ces noms-là, rien ne disait qu'il
        // demanderait quelque chose, et la ligne de saisie n'apparaissait
        // qu'au moment où il se bloquait dessus.
        if let Some(binaire) = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
            && binaire
                .externals
                .values()
                .any(|nom| namsdec_emu::name_reads_input(nom))
        {
            lit = true;
        }
        self.program_reads = lit;
        self.program_writes = écrit;
    }

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

    /// (Re)construit l'émulateur dans l'état du tout début.
    ///
    /// Deux chemins, parce qu'il y a deux sortes de programmes. Celui
    /// qu'on vient d'écrire est posé tel quel, à l'adresse qu'on lui a
    /// donnée, avec une pile vierge : c'est ce qu'on veut en apprenant, et
    /// rien d'autre ne doit bouger sous les pieds. Celui qui vient d'un
    /// fichier, lui, passe par le **chargeur** — segments, relocations,
    /// TLS, pile de processus —, faute de quoi il ne survit pas à ses dix
    /// premières instructions.
    /// Remet la machine dans l'état où le chargement l'avait laissée.
    ///
    /// Reconstruire l'émulateur suffit pour le processeur et la mémoire,
    /// mais ce qui l'entoure gardait les traces de la course précédente :
    /// les diagnostics d'un binaire ouvert, l'onglet resté ouvert sur
    /// eux, le compte de pas, et le listing immobile là où l'exécution
    /// s'était arrêtée. Rien de cela ne décrit plus le programme qu'on
    /// vient de ramener à son point d'entrée, et le bouton passait pour
    /// n'avoir rien fait.
    fn do_reset(&mut self) {
        // Les diagnostics d'un binaire ouvert ne viennent pas d'un
        // assemblage : `load_elf` les vide en le chargeant, et les
        // laisser ici ferait de « Réinitialiser » un demi-chargement.
        // Ceux d'un source, eux, désignent des lignes qui n'ont pas
        // bougé : les effacer perdrait ce que nasm avait à dire.
        let from_binary = self
            .assemble_state
            .as_ref()
            .is_some_and(|state| state.from_binary.is_some());
        if from_binary {
            self.diagnostics.clear();
            if self.console_tab == ConsoleTab::Diagnostics {
                self.console_tab = ConsoleTab::Output;
            }
        }
        self.rebuild_emulator();
        self.run_steps_taken = 0;
        // Le listing suit le programme jusqu'à son point d'entrée : le
        // laisser où la course s'était arrêtée montrait du code que plus
        // rien n'allait exécuter.
        self.follow_execution();
        self.status = Status::Idle;
    }

    fn rebuild_emulator(&mut self) {
        if let Some(elf) = self.binary_elf() {
            self.load_process(&elf);
            return;
        }
        self.rebuild_from_assembly();
    }

    /// L'ELF du binaire ouvert, quand c'en est un et qu'il est chargeable.
    ///
    /// Relu à chaque reconstruction plutôt que gardé : l'analyse coûte
    /// une milliseconde sur un binaire de trois cent mille octets, et
    /// garder une image déjà relogée obligerait à la dupliquer pour
    /// pouvoir revenir à l'état initial — ce que « Reset » demande.
    fn binary_elf(&self) -> Option<namsdec_asm::elf::Elf> {
        let state = self.assemble_state.as_ref()?;
        state.from_binary.as_ref()?;
        namsdec_asm::elf::parse(&state.output.file).filter(|elf| !elf.segments.is_empty())
    }

    /// Charge un binaire comme le noyau le ferait, et arme l'émulateur
    /// sur l'état que `_start` trouverait.
    fn load_process(&mut self, elf: &namsdec_asm::elf::Elf) {
        let nom = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
            .map(|binaire| binaire.name.clone())
            .filter(|nom| !nom.is_empty())
            .unwrap_or_else(|| "programme".to_owned());

        let arguments = split_args(&self.program_args);
        let environnement: Vec<String> = self
            .program_env
            .lines()
            .map(str::trim)
            .filter(|ligne| !ligne.is_empty())
            .map(str::to_owned)
            .collect();
        let chargé = match loader::load(elf, &nom, &arguments, &environnement, MEM_MAX) {
            Ok(chargé) => chargé,
            Err(loader::LoadError::TooLarge { .. }) => {
                self.status = Status::Error(strings(self.lang).binary_too_large.to_string());
                self.emulator = None;
                return;
            }
            Err(loader::LoadError::NotLoadable) => {
                self.status = Status::Error(strings(self.lang).binary_not_linked.to_owned());
                self.emulator = None;
                return;
            }
        };

        let mut cpu = namsdec_core::Cpu::new(chargé.memory);
        cpu.regs.rip = chargé.entry;
        cpu.regs.set64(Reg64::Rsp, chargé.rsp);
        // La base FS : sans elle, la première fonction protégée lit son
        // canari à une adresse qui n'existe pas.
        cpu.regs.fs_base = chargé.fs_base;

        // Un programme qui vient d'être chargé n'a pas de passé.
        self.regs_before = None;
        self.flags_before = None;
        self.last_step = None;
        self.error_advice = None;
        self.mem_before = None;

        let mut emulator = Emulator::new(cpu);
        emulator.set_heap(chargé.layout.heap.start, chargé.layout.heap.end);
        emulator.set_mmap(chargé.layout.mmap.start, chargé.layout.mmap.end);
        // Les bornes : arriver sur l'une d'elles, c'est appeler la
        // fonction qu'elle remplace. namsdec en fournit une poignée.
        emulator.set_externals(chargé.externals.clone(), chargé.main_return);
        emulator.set_breakpoints(
            self.breakpoint_lines
                .iter()
                .filter_map(|l| {
                    self.assemble_state
                        .as_ref()
                        .and_then(|state| state.output.line_to_addr.get(l).copied())
                })
                .collect(),
        );

        if let Some(state) = self.assemble_state.as_ref() {
            let ctx = ReContext {
                cpu: emulator.cpu(),
                disasm: &state.disasm,
                source: &self.source,
                binary: &state.output.file,
                lang: self.lang.code(),
            };
            self.re_registry.on_binary_loaded(&ctx);
        }

        self.emulator = Some(Box::new(emulator));
        // Ce que le chargeur a posé suit le programme : les bornes
        // servent à nommer un appel qui n'aboutira pas, le rapport à dire
        // ce qui a été fait.
        if let Some(binaire) = self
            .assemble_state
            .as_mut()
            .and_then(|state| state.from_binary.as_mut())
        {
            binaire.externals = chargé.externals;
            binaire.load = chargé.report;
        }
        // Les bornes viennent d'être posées : c'est maintenant qu'on sait
        // si ce programme réclamera une entrée.
        self.note_program_io();
        self.sync_watchpoints();
    }

    /// Le chemin d'un programme qu'on vient d'assembler : l'image telle
    /// quelle, à son adresse, dans la fenêtre historique de l'atelier.
    fn rebuild_from_assembly(&mut self) {
        let Some(state) = &self.assemble_state else {
            return;
        };
        let (base, taille, sommet) =
            memory_window(state.output.load_addr, state.output.bytes.len());
        if taille > MEM_MAX {
            self.status = Status::Error(strings(self.lang).binary_too_large.to_string());
            self.emulator = None;
            return;
        }
        let mut mem = namsdec_core::Memory::new(base, taille);
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
                // Une section vide ne projette rien : la déclarer
                // n'ajouterait qu'une ligne sans contenu à la carte.
                if section.size == 0 {
                    continue;
                }
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
                HEAP_LIMIT..sommet + 0x100,
                namsdec_core::Perms::DATA,
                "stack",
            );
        }

        let mut cpu = namsdec_core::Cpu::new(mem);
        // Le point d'entrée de l'ELF, et non le premier octet chargé : sur
        // un programme qui commence par `.data`, ce sont deux adresses
        // différentes.
        cpu.regs.rip = state.output.entry;
        cpu.regs.set64(Reg64::Rsp, sommet);

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
        let cpu = emulator.cpu();
        condition.holds(&cpu.regs, Some(&PeekMemory(&cpu.mem)))
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

    /// Rend compte d'une erreur d'exécution : le message et, quand il y
    /// en a un, le conseil qui va avec.
    fn report_error(&mut self, e: &namsdec_emu::EmuError) {
        self.error_advice = errors::advice(self.lang, e).map(str::to_owned);
        self.status = Status::Error(errors::emu(self.lang, e));
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
            Ok(StepResult::NeedsInput) => self.wait_for_input(),
            Ok(StepResult::Continued) => {}
            Err(e) => self.report_error(&e),
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

    /// Court jusqu'à la prochaine instruction de ce genre.
    ///
    /// Les points d'arrêt sont posés d'avance sur toutes les
    /// instructions visées — un pas à pas jusqu'à la bonne aurait figé
    /// l'interface sur une boucle un peu longue. L'instruction courante
    /// est exclue, sans quoi « au prochain appel » n'irait nulle part
    /// quand on est déjà sur un `call`.
    ///
    /// Rien à viser n'est pas une erreur : le programme ne contient
    /// simplement pas ce qu'on cherche, et le dire vaut mieux que de le
    /// laisser courir jusqu'à la fin.
    fn do_run_until(&mut self, kind: UntilKind) {
        self.last_step = None;
        self.error_advice = None;
        let Some(state) = &self.assemble_state else {
            return;
        };
        let ici = self.emulator.as_ref().map(|e| e.cpu().regs.rip);
        let mut points: BTreeSet<u64> = state
            .disasm
            .iter()
            .filter(|instruction| Some(instruction.address) != ici)
            .filter_map(|instruction| {
                namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
                    .filter(|decoded| kind.matches(decoded))
                    .map(|_| instruction.address)
            })
            .collect();
        if points.is_empty() {
            self.status = Status::Note(match kind {
                UntilKind::Call => |s: &Strings| s.run_until_no_call,
                UntilKind::Return => |s: &Strings| s.run_until_no_return,
                UntilKind::Syscall => |s: &Strings| s.run_until_no_syscall,
            });
            return;
        }
        // Les points de l'utilisateur restent armés : on court vers le
        // prochain appel, mais on s'arrête quand même là où il l'a
        // demandé.
        points.extend(
            self.breakpoint_lines
                .iter()
                .filter_map(|l| state.output.line_to_addr.get(l).copied()),
        );
        if let Some(emulator) = &mut self.emulator {
            emulator.set_breakpoints(points);
        }
        self.run_until = Some(kind);
        self.do_run();
    }

    /// À appeler une fois par frame. N'avance que si un Run est en cours.
    fn tick_run(&mut self, ctx: &egui::Context) {
        if !self.running {
            // La course finie, le point d'arrêt d'un seul voyage s'en va
            // avec elle : il n'appartient pas à l'utilisateur, et le
            // laisser là ferait s'arrêter la course suivante à un endroit
            // que personne n'a demandé.
            if self.run_to.take().is_some() | self.run_until.take().is_some() {
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
                self.wait_for_input();
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
                self.report_error(&e);
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
            Ok(StepOverResult::NeedsInput) => self.wait_for_input(),
            Ok(StepOverResult::Stepped | StepOverResult::SteppedOverCall) => {}
            Err(e) => self.report_error(&e),
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
    /// Assembler a-t-il une chance d'aboutir ?
    ///
    /// Le bouton se grise plutôt que d'échouer : un listing de binaire
    /// sera refusé à tous les coups, et rien ne l'annonçait avant le
    /// clic. Le raccourci et la palette, eux, passent toujours par
    /// `do_assemble` — refuser en disant pourquoi vaut mieux que ne rien
    /// faire du tout quand le geste ne vient pas d'un bouton qu'on voit.
    /// Le programme réclame de quoi lire.
    ///
    /// Chaque nouvelle attente rend son focus au champ, même si la
    /// précédente s'était terminée par un Échap : ce qu'on a refusé une
    /// fois, on ne le refuse pas pour toujours.
    fn wait_for_input(&mut self) {
        self.status = Status::WaitingForInput;
        self.console_input_released = false;
    }

    fn can_assemble(&self) -> bool {
        !self.buffer_came_from_a_binary()
    }

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
            Action::Reset => self.do_reset(),
            Action::Save => self.do_save(),
            Action::OpenFile => self.do_open_file_dialog(),
            Action::OpenFolder => self.do_open_folder(),
            Action::InstructionHelp => self.do_instruction_help(),
            Action::RunToCursor => self.do_run_to_cursor(),
            Action::RunToCall => self.do_run_until(UntilKind::Call),
            Action::RunToReturn => self.do_run_until(UntilKind::Return),
            Action::RunToSyscall => self.do_run_until(UntilKind::Syscall),
            Action::GoToLabel => self.do_goto_label(),
            Action::Find => self.do_find(),
            Action::New => self.do_new(),
            Action::ShowFlow => self.show_flow = true,
            Action::NextTab => self.cycle_tab(1),
            Action::PreviousTab => self.cycle_tab(-1),
            Action::CloseTab => self.close_tab(self.active_tab_index),
            Action::DuplicateLine => self.duplicate_line(),
            Action::HighlightMatches => self.highlight_matches(),
            Action::RenameMatches => self.start_rename(),
            Action::NameAddress => self.start_note(NoteKind::Name),
            Action::CommentAddress => self.start_note(NoteKind::Comment),
            Action::NextMatch => self.go_to_match(true),
            Action::PreviousMatch => self.go_to_match(false),
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
                | FileDialogKind::ExportReport
                | FileDialogKind::ExportDump
                | FileDialogKind::ExportBinary => dialog.save_file(),
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
            FileDialogKind::ExportBinary => dialog.set_file_name(
                self.current_file
                    .as_ref()
                    .and_then(|path| path.file_stem())
                    .map(|nom| nom.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "programme".to_owned()),
            ),
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
            FileDialogKind::ExportDump => dialog
                .add_filter(s.filter_text, &["txt"])
                .set_file_name(format!(
                    "{}-vidage.txt",
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
            FileDialogKind::ExportBinary => self.export_binary(&path),
            FileDialogKind::ExportDump => self.export_dump(&path),
        }
    }

    /// Écrit le vidage hexadécimal du fichier chargé.
    ///
    /// Le fichier tel qu'il est sur le disque, au format de `xxd` : c'est
    /// ce qui permet de le comparer — à celui qu'on a sous les yeux, à
    /// celui d'un autre outil, ou à la même analyse faite la semaine
    /// dernière. Un `diff` de deux vidages dit en une ligne ce qu'un
    /// binaire a de changé.
    fn export_dump(&mut self, path: &Path) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            self.status = Status::Error(s.binary_none.to_owned());
            return;
        };
        let texte = namsdec_re::dump_module::hex_dump(&state.output.file);
        match std::fs::write(path, texte) {
            Ok(()) => self.status = Status::Note(|s| s.dump_saved),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.dump_save_failed)),
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
            Ok(()) => self.status = Status::Note(|s| s.manual_exported),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.manual_export_failed)),
        }
    }

    /// Écrit le programme assemblé, tel que `ld` l'a produit.
    ///
    /// L'ELF entier, et exécutable : c'est le même fichier que la ligne
    /// de commande aurait donné, et il se lance dans un vrai terminal,
    /// se lit avec `objdump`, se compare avec ce que namsdec en dit.
    /// Un atelier qui ne laisserait rien sortir apprendrait à se servir
    /// de lui, pas de l'assembleur.
    fn export_binary(&mut self, path: &Path) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            self.status = Status::Error(s.binary_none.to_owned());
            return;
        };
        if state.output.file.is_empty() {
            self.status = Status::Error(s.binary_none.to_owned());
            return;
        }
        match std::fs::write(path, &state.output.file) {
            Ok(()) => {
                // Exécutable : un programme qu'il faut penser à rendre
                // exécutable soi-même n'est pas sorti, il est à moitié
                // sorti.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
                }
                self.status = Status::Note(|s| s.binary_exported);
            }
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.binary_export_failed)),
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
            Ok(()) => self.status = Status::Note(|s| s.report_saved),
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
    /// L'état de travail, rangé tel quel pour y revenir.
    fn take_tab(&mut self) -> OpenTab {
        OpenTab {
            path: self.current_file.take(),
            file_mtime: self.file_mtime,
            source: std::mem::take(&mut self.source),
            saved_source: std::mem::take(&mut self.saved_source),
            cursor_byte: self.cursor_byte,
            cursor_pos: self.cursor_pos,
            breakpoint_lines: std::mem::take(&mut self.breakpoint_lines),
            breakpoint_conditions: std::mem::take(&mut self.breakpoint_conditions),
            assemble_state: self.assemble_state.take(),
            emulator: self.emulator.take(),
            diagnostics: std::mem::take(&mut self.diagnostics),
            current_exercise: self.current_exercise.take(),
            program_reads: self.program_reads,
            program_writes: self.program_writes,
        }
    }

    /// Et l'inverse : l'onglet redevient le fichier de travail.
    fn restore_tab(&mut self, tab: OpenTab) {
        self.current_file = tab.path;
        self.file_mtime = tab.file_mtime;
        self.source = tab.source;
        self.saved_source = tab.saved_source;
        self.cursor_byte = tab.cursor_byte;
        self.cursor_pos = tab.cursor_pos;
        self.breakpoint_lines = tab.breakpoint_lines;
        self.breakpoint_conditions = tab.breakpoint_conditions;
        self.assemble_state = tab.assemble_state;
        self.emulator = tab.emulator;
        self.diagnostics = tab.diagnostics;
        self.current_exercise = tab.current_exercise;
        self.program_reads = tab.program_reads;
        self.program_writes = tab.program_writes;
        // Ce qui n'appartient qu'au moment présent, et non au fichier :
        // une course en train de tourner, la photographie du dernier
        // pas, le curseur qu'on voulait poser ailleurs.
        self.running = false;
        self.last_step = None;
        self.error_advice = None;
        self.regs_before = None;
        self.flags_before = None;
        self.mem_before = None;
        self.status = Status::Idle;
        // L'éditeur doit remettre le curseur là où il était : il ne le
        // fait qu'à la demande, sinon il le laisse où la souris l'a
        // posé.
        self.goto_line = Some(self.cursor_pos.0.max(1) as u32);
        // Le fichier a pu changer pendant qu'on travaillait ailleurs.
        self.check_disk_change();
    }

    /// Le fichier ouvert a-t-il changé sur le disque ?
    ///
    /// Non enregistré de notre côté, on recharge : c'est ce que le
    /// fichier dit maintenant, et rien de nôtre n'est perdu. Modifié des
    /// deux côtés, on ne choisit pas à la place de l'utilisateur — on le
    /// dit, et il tranche.
    fn check_disk_change(&mut self) {
        let (Some(path), Some(connue)) = (self.current_file.clone(), self.file_mtime) else {
            return;
        };
        let Some(sur_disque) = file_mtime(&path) else {
            // Disparu : ce n'est pas à nous de l'effacer de l'écran, et
            // Ctrl+S le réécrira très bien.
            return;
        };
        if sur_disque == connue {
            return;
        }
        if self.buffer_modified() {
            self.disk_changed = true;
            return;
        }
        let Ok(texte) = std::fs::read_to_string(&path) else {
            self.disk_changed = true;
            return;
        };
        self.source = texte.clone();
        self.saved_source = texte;
        self.file_mtime = Some(sur_disque);
        self.disk_changed = false;
        // Ce qui était assemblé décrit l'ancienne version : le garder
        // ferait pointer la gouttière sur des lignes qui ont bougé.
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.status = Status::Note(|s| s.disk_reloaded);
    }

    /// Recharge depuis le disque, en abandonnant ce qu'on avait tapé.
    fn reload_from_disk(&mut self) {
        if self.current_file.is_none() {
            return;
        }
        // On fait comme si le tampon était à jour : `check_disk_change`
        // relit alors le fichier et remplace tout.
        self.saved_source = self.source.clone();
        self.check_disk_change();
    }

    /// Tous les onglets dans l'ordre, l'actif à sa place.
    ///
    /// L'actif n'est pas rangé dans la liste — c'est l'état de travail
    /// lui-même —, mais la barre d'onglets, elle, doit les montrer tous
    /// à leur rang.
    fn tab_labels(&self) -> Vec<(String, bool)> {
        let s = strings(self.lang);
        let actif = (
            tab_label(
                self.current_file.as_deref(),
                self.current_exercise.as_deref(),
                s,
                &self.exercise_packs,
                self.lang,
            ),
            self.buffer_modified(),
        );
        let mut out: Vec<(String, bool)> = self
            .tabs
            .iter()
            .map(|tab| (tab.label(s, &self.exercise_packs, self.lang), tab.modifié()))
            .collect();
        out.insert(self.active_tab_index.min(out.len()), actif);
        out
    }

    /// Passe à l'onglet de rang `index`, en rangeant celui qu'on quitte.
    fn go_to_tab(&mut self, index: usize) {
        if index == self.active_tab_index || index > self.tabs.len() {
            return;
        }
        // On retire d'abord la cible, puis on range l'actif à sa place :
        // l'ordre des onglets ne change pas sous les doigts.
        let cible = if index < self.active_tab_index {
            self.tabs.remove(index)
        } else {
            self.tabs.remove(index - 1)
        };
        let courant = self.take_tab();
        self.tabs
            .insert(self.active_tab_index.min(self.tabs.len()), courant);
        self.active_tab_index = index;
        self.restore_tab(cible);
    }

    /// D'un onglet au suivant, en bouclant.
    ///
    /// Boucler plutôt que buter : on parcourt trois fichiers avec la
    /// même touche, sans avoir à se demander où l'on est dans la file.
    fn cycle_tab(&mut self, pas: i32) {
        let total = self.tabs.len() + 1;
        if total < 2 {
            return;
        }
        let courant = self.active_tab_index as i32;
        let cible = (courant + pas).rem_euclid(total as i32) as usize;
        self.go_to_tab(cible);
    }

    /// Ferme l'onglet de rang `index`.
    ///
    /// Le dernier ne se ferme pas : une fenêtre d'éditeur sans éditeur
    /// n'a rien à montrer. Fermer celui-là revient à repartir d'un
    /// programme neuf, ce que Fichier ▸ Nouveau fait déjà.
    fn close_tab(&mut self, index: usize) {
        if self.tabs.is_empty() {
            return;
        }
        // Un onglet dont le texte n'est pas enregistré ne part pas sur
        // un clic : c'est le seul endroit d'où un travail peut
        // disparaître sans qu'on l'ait voulu.
        if self.tab_modified(index) {
            // La question porte sur l'onglet visé : on s'y installe
            // d'abord, pour que « Enregistrer » enregistre le bon
            // fichier et qu'on voie ce dont on parle.
            self.go_to_tab(index);
            self.pending = Some(Pending::CloseTab(self.active_tab_index));
            return;
        }
        self.close_tab_now(index);
    }

    /// Ce que l'onglet de rang `index` a de non enregistré.
    fn tab_modified(&self, index: usize) -> bool {
        if index == self.active_tab_index {
            return self.buffer_modified();
        }
        let rang = if index < self.active_tab_index {
            index
        } else {
            index.wrapping_sub(1)
        };
        self.tabs.get(rang).is_some_and(OpenTab::modifié)
    }

    /// Ferme sans plus rien demander.
    fn close_tab_now(&mut self, index: usize) {
        if self.tabs.is_empty() {
            return;
        }
        if index == self.active_tab_index {
            // On s'installe sur le voisin, puis on ferme celui qu'on
            // vient de quitter.
            let voisin = if index == 0 { 1 } else { index - 1 };
            self.go_to_tab(voisin);
            let rang = if index > self.active_tab_index {
                index - 1
            } else {
                index
            };
            self.tabs.remove(rang);
            if self.active_tab_index > index {
                self.active_tab_index -= 1;
            }
            return;
        }
        let rang = if index < self.active_tab_index {
            index
        } else {
            index - 1
        };
        self.tabs.remove(rang);
        if index < self.active_tab_index {
            self.active_tab_index -= 1;
        }
    }

    /// Ouvre un fichier dans un nouvel onglet, ou revient à celui qui le
    /// montre déjà.
    fn open_in_new_tab(&mut self, path: PathBuf) {
        if let Some(rang) = self.tab_index_of(&path) {
            // Déjà ouvert : on y retourne. Et il remonte quand même
            // parmi les récents — c'est bien le fichier qu'on vient de
            // demander, qu'il ait fallu le lire ou non.
            self.note_recent(&path);
            self.go_to_tab(rang);
            return;
        }
        // Un tampon vierge — pas de fichier, rien de tapé — n'a rien à
        // garder : le fichier demandé prend sa place au lieu d'ouvrir un
        // onglet « sans titre » que personne n'a demandé.
        if self.current_file.is_none() && !self.buffer_modified() {
            self.open_file_now(path);
            return;
        }
        // Sinon l'onglet courant est rangé tel quel, et le nouveau prend
        // sa place juste après : on ouvre un fichier à côté de celui
        // qu'on lisait, pas au bout de la file.
        let courant = self.take_tab();
        let insertion = self.active_tab_index.min(self.tabs.len());
        self.tabs.insert(insertion, courant);
        self.active_tab_index = insertion + 1;
        self.open_file_now(path);
    }

    /// Le rang de l'onglet qui travaille cet exercice, s'il y en a un.
    fn exercise_tab_index(&self, id: &str) -> Option<usize> {
        if self.current_exercise.as_deref() == Some(id) {
            return Some(self.active_tab_index);
        }
        let position = self
            .tabs
            .iter()
            .position(|tab| tab.current_exercise.as_deref() == Some(id))?;
        Some(if position >= self.active_tab_index {
            position + 1
        } else {
            position
        })
    }

    /// Le rang de l'onglet qui montre ce fichier, s'il y en a un.
    fn tab_index_of(&self, path: &Path) -> Option<usize> {
        if self.current_file.as_deref() == Some(path) {
            return Some(self.active_tab_index);
        }
        let position = self
            .tabs
            .iter()
            .position(|tab| tab.path.as_deref() == Some(path))?;
        Some(if position >= self.active_tab_index {
            position + 1
        } else {
            position
        })
    }

    /// Repart d'un programme neuf.
    fn do_new(&mut self) {
        self.guard(Pending::New);
    }

    /// Un programme neuf, dans son propre onglet.
    ///
    /// Le fichier en cours reste ouvert à côté : « nouveau » veut dire
    /// « un de plus », et non « à la place de ».
    fn new_program(&mut self) {
        if self.current_file.is_some() || self.buffer_modified() {
            let courant = self.take_tab();
            let insertion = self.active_tab_index.min(self.tabs.len());
            self.tabs.insert(insertion, courant);
            self.active_tab_index = insertion + 1;
        }
        self.source = NEW_PROGRAM.to_owned();
        self.saved_source = NEW_PROGRAM.to_owned();
        self.current_file = None;
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.breakpoint_lines.clear();
        self.status = Status::Idle;
    }

    /// Ouvre un fichier dans son propre onglet.
    ///
    /// Plus de question à poser avant : le fichier en cours n'est pas
    /// remplacé, il attend dans son onglet avec ses modifications, son
    /// curseur et ses points d'arrêt.
    fn open_file(&mut self, path: PathBuf) {
        self.open_in_new_tab(path);
    }

    /// Y a-t-il quelque chose à perdre ?
    ///
    /// La comparaison se fait sur le texte, pas sur un drapeau : taper
    /// un caractère puis l'effacer ne met rien en danger.
    fn buffer_modified(&self) -> bool {
        self.source != self.saved_source
    }

    /// Combien de fichiers ouverts ont du texte non enregistré.
    fn modified_tabs(&self) -> usize {
        usize::from(self.buffer_modified()) + self.tabs.iter().filter(|t| t.modifié()).count()
    }

    /// Le rang du premier onglet — autre que l'actif — qui a du texte
    /// non enregistré.
    fn first_modified_tab(&self) -> Option<usize> {
        (0..=self.tabs.len())
            .find(|rang| *rang != self.active_tab_index && self.tab_modified(*rang))
    }

    /// Y a-t-il quelque chose à perdre **où que ce soit** ?
    ///
    /// Depuis qu'on peut ouvrir plusieurs fichiers, quitter ne met pas en
    /// danger le seul tampon visible : un onglet laissé de côté avec des
    /// modifications compte autant.
    fn anything_modified(&self) -> bool {
        self.buffer_modified() || self.tabs.iter().any(OpenTab::modifié)
    }

    /// Fait l'action, ou demande d'abord.
    fn guard(&mut self, action: Pending) {
        if self.anything_modified() {
            self.pending = Some(action);
        } else {
            self.perform(action);
        }
    }

    fn perform(&mut self, action: Pending) {
        match action {
            Pending::New => self.new_program(),
            Pending::CloseTab(index) => self.close_tab_now(index),
            Pending::Exercise(id) => self.load_exercise_now(&id),
            Pending::Quit => self.quitting = true,
        }
    }

    /// Combien de fichiers récents on garde.
    ///
    /// Huit : de quoi couvrir une séance de travail. Au-delà, la liste
    /// devient un historique, qu'on parcourt au lieu de le lire.
    const RECENT_LIMIT: usize = 8;

    /// Note un fichier comme récemment ouvert.
    fn note_recent(&mut self, path: &Path) {
        if self.restoring_session {
            return;
        }
        // Le même fichier ne s'y met pas deux fois : il remonte.
        self.recent_files.retain(|autre| autre != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(Self::RECENT_LIMIT);
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
                self.note_recent(&path);
                self.file_mtime = file_mtime(&path);
                self.disk_changed = false;
                self.current_file = Some(path);
                // On travaille une source maintenant : le binaire de la
                // séance précédente n'a plus à revenir au lancement
                // suivant.
                self.binary_path = None;
                self.assemble_state = None;
                self.emulator = None;
                self.diagnostics.clear();
                self.status = Status::Idle;
            }
            Err(e) => {
                let nom = path
                    .file_name()
                    .map(|nom| nom.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "programme".to_owned());
                self.note_recent(&path);
                self.load_binary(e.into_bytes(), &nom);
                // Retenu pour la séance suivante. Pas dans
                // `current_file`, qui reste vide exprès pour un binaire :
                // un Ctrl+S distrait n'a pas à écraser l'exécutable par
                // son propre désassemblage.
                self.binary_path = Some(path);
            }
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
    fn load_elf(&mut self, bytes: &[u8], name: &str) -> bool {
        let Some(elf) = namsdec_asm::elf::parse(bytes) else {
            return false;
        };
        // Les notes de ce binaire, s'il en a : on les retrouve avant de
        // dessiner quoi que ce soit, pour que le premier coup d'oeil
        // porte déjà les noms qu'on avait trouvés.
        self.load_notes_for(bytes);
        // Un fichier de réadressage : aucune section n'a d'adresse, et
        // il n'y a pas de point d'entrée. `ld` n'est pas encore passé.
        let relocatable = elf.entry == 0 && elf.sections.iter().all(|section| section.addr == 0);
        let Some((load_addr, image)) = elf.image() else {
            return false;
        };
        // La fenêtre s'adapte au programme : ce qui est refusé, c'est
        // ce qui dépasse ce qu'une machine accepte d'allouer.
        if memory_window(load_addr, image.len()).1 > MEM_MAX {
            return false;
        }

        // Le listing ne montre que les sections exécutables : le reste est
        // de la donnée, et la donnée désassemblée ne veut rien dire.
        // Par sections quand le binaire en a, par segments sinon : un
        // crackme dépouillé de sa table de sections n'en a aucune, et
        // namsdec le déclarait illisible alors qu'il s'exécute très bien.
        let mut disasm = Vec::new();
        for (adresse, octets) in elf.executable_parts() {
            disasm.extend(namsdec_disasm::disassemble(octets, adresse));
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
            // Un ELF ouvert : on retient ce qu'il réclame au chargement,
            // car c'est ce qui décide s'il peut tourner ici.
            // Ce que le fichier réclame au chargement se lit dans sa
            // table dynamique, que seul le scan complet parcourt.
            from_binary: Some(match namsdec_re::elf_scan::scan(bytes) {
                Some(scan) => LoadedBinary {
                    needed: scan.needed,
                    interpreter: scan.interpreter,
                    name: name.to_owned(),
                    ..Default::default()
                },
                None => LoadedBinary {
                    name: name.to_owned(),
                    ..Default::default()
                },
            }),
        });
        self.note_program_io();
        // Un objet non lié — un `.o` — n'a ni point d'entrée ni adresse
        // de section : `ld` ne lui en a pas encore donné. Il se lit et
        // s'analyse très bien, mais l'exécuter n'a pas de sens, et le
        // chargeur s'en plaignait jusqu'ici en parlant de mémoire, ce
        // qui n'expliquait rien.
        if relocatable {
            self.emulator = None;
            self.status = Status::Error(strings(self.lang).binary_not_linked.to_owned());
            return true;
        }
        self.rebuild_emulator();
        if !matches!(self.status, Status::Error(_)) {
            self.status = Status::BinaryLoaded {
                bytes: loaded,
                truncated: false,
            };
        }
        // Un binaire lié dynamiquement se lit et s'analyse très bien ;
        // il ne s'exécute pas ici, faute des fonctions qu'il emprunte.
        // Le dire maintenant vaut mieux que de le laisser découvrir sur
        // un « rien n'est projeté à cette adresse » au premier appel,
        // qui parle de mémoire là où le problème est ailleurs.
        if let Some(chargé) = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
            && (!chargé.needed.is_empty() || chargé.interpreter.is_some())
        {
            let s = strings(self.lang);
            self.status = Status::Note(|s| s.binary_dynamic);
            self.diagnostics.push(namsdec_asm::Diagnostic {
                line: 0,
                severity: namsdec_asm::Severity::Warning,
                // Sans motif, celui-ci : sa phrase est bâtie autour d'une
                // chaîne de l'interface (`binary_dynamic`), et la
                // recopier dans namsdec-asm ferait deux sources pour un
                // même texte. Il garde donc la langue où il est né — ce
                // qui se voit, mais moins qu'une divergence entre deux
                // copies du même message.
                kind: None,
                message: format!(
                    "{} {}",
                    s.binary_dynamic,
                    if chargé.needed.is_empty() {
                        chargé.interpreter.clone().unwrap_or_default()
                    } else {
                        chargé.needed.join(", ")
                    }
                ),
            });
        }
        self.note_load_report();
        // Le vidage mémoire s'ouvre là où l'exécution commence : la
        // fenêtre historique (`0x400000`) ne veut plus rien dire pour un
        // binaire chargé à ses propres adresses, et la première chose
        // qu'on regarde est ce que le programme va exécuter.
        self.mem_addr = elf.entry;
        self.mem_addr_input = format!("{:#x}", elf.entry);
        true
    }

    /// Dit ce que le chargeur a fait, et ce que le programme emprunte.
    ///
    /// Un chargement complet n'appelle pas de commentaire : le programme
    /// tourne, et la carte mémoire montre le reste. Un programme qui
    /// emprunte des fonctions, si — car la question qui vient ensuite est
    /// « est-ce que ça va tourner ? », et la réponse tient en un compte :
    /// combien de ces fonctions namsdec sait faire, et lesquelles il ne
    /// sait pas.
    fn note_load_report(&mut self) {
        let Some(chargé) = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
        else {
            return;
        };
        let rapport = chargé.load.clone();
        if rapport.external.is_empty() {
            return;
        }
        let s = strings(self.lang);
        let (fournies, absentes): (Vec<String>, Vec<String>) = rapport
            .external
            .iter()
            .cloned()
            .partition(|nom| namsdec_emu::provides(nom));
        // Les manquantes d'abord, et toutes si elles tiennent : ce sont
        // elles qui décident où l'exécution s'arrêtera.
        let montrées = absentes
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let suite = if absentes.len() > 8 {
            format!(", … {} {}", absentes.len() - 8, s.loaded_others)
        } else {
            String::new()
        };
        let emprunts = format!(
            "{} {} {} {} {}",
            s.loaded_provided_before,
            fournies.len(),
            s.loaded_provided_of,
            rapport.external.len(),
            s.loaded_provided_after,
        );
        let message = format!(
            "{} {} {}, {} {}, {} {}. {emprunts}{}",
            s.loaded_summary,
            rapport.segments,
            s.loaded_segments,
            rapport.relocations,
            s.loaded_relocations,
            rapport.tls,
            s.loaded_tls,
            if absentes.is_empty() {
                String::new()
            } else {
                format!(" {} {montrées}{suite}", s.loaded_missing)
            },
        );
        self.diagnostics.push(namsdec_asm::Diagnostic {
            line: 0,
            severity: namsdec_asm::Severity::Warning,
            message,
            // Même cas que le précédent : la phrase se compose de
            // chaînes de l'interface, qu'on ne recopie pas ailleurs.
            kind: None,
        });
    }

    fn load_binary(&mut self, mut bytes: Vec<u8>, name: &str) {
        // Un ELF64 dit lui-même où il va et par où il commence : autant
        // l'écouter.
        if self.load_elf(&bytes, name) {
            return;
        }
        // Des octets bruts : ils ont droit à des notes comme les autres.
        // C'est même là qu'elles servent le plus — sans en-tête, il n'y a
        // aucun symbole pour aider.
        self.load_notes_for(&bytes);

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

        // Ce qu'on désassemble : le code, et seulement lui, quand le
        // fichier dit où il est. Partir de l'octet zéro faisait lire
        // l'en-tête ELF et les tables de relocation comme des
        // instructions — les premières lignes d'un binaire ouvert
        // étaient du charabia, et c'est la première chose qu'on voyait.
        let disasm = executable_ranges(&file, bytes.len())
            .map(|plages| {
                let mut out = Vec::new();
                for (offset, len) in plages {
                    let début = offset as usize;
                    let fin = début + len as usize;
                    if let Some(morceau) = bytes.get(début..fin) {
                        out.extend(namsdec_disasm::disassemble(morceau, BASE_ADDR + offset));
                    }
                }
                out
            })
            .filter(|out| !out.is_empty())
            .unwrap_or_else(|| namsdec_disasm::disassemble(&bytes, BASE_ADDR));

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
            // Un tampon brut désassemblé : pas de bibliothèque à
            // réclamer, mais il vient bien d'un binaire.
            from_binary: Some(LoadedBinary::default()),
        });
        self.note_program_io();
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
            PaletteCommand::ShowCalculator => self.show_calculator = true,
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
            PaletteCommand::GoLine(line) => {
                self.goto_line = Some(line);
                self.status = Status::Idle;
            }
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
        let mut commands = palette_commands(s, self.show_re_tab, self.exercises);
        // Les étiquettes du programme, à la suite : sur deux cents
        // lignes, « où est donc `affiche_nombre` ? » se répond en tapant
        // son nom, comme on ouvre un fichier par son nom dans un éditeur.
        // Rien à retenir de plus : c'est la même palette.
        commands.extend(source_labels(&self.source).into_iter().map(|(nom, ligne)| {
            (
                format!("{} {nom} ({} {ligne})", s.palette_goto_label, s.line_word),
                PaletteCommand::GoLine(ligne),
            )
        }));

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
                // Ce qu'on vient d'écrire fait foi : la date change, et
                // l'avertissement éventuel n'a plus lieu d'être.
                self.file_mtime = file_mtime(path);
                self.disk_changed = false;
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
        let mut bouton_fenêtre = None;
        ui.horizontal(|ui| {
            // Les pastilles en tête de la barre : la fermeture occupe
            // alors le coin de l'écran, l'angle qu'une souris lancée
            // vers le haut à gauche atteint sans viser.
            if !self.system_titlebar {
                let maximisée = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                bouton_fenêtre = chrome::window_buttons_ui(ui, s, maximisée);
                ui.add_space(4.0);
            }
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
                // Fermer le fichier qu'on regarde : le geste existe dans
                // la barre d'onglets, mais il se cherche d'abord ici.
                if !self.tabs.is_empty() {
                    if ui
                        .button(shortcut_hint(
                            &self.keybindings,
                            &ctx,
                            s.action_close_tab,
                            Action::CloseTab,
                        ))
                        .clicked()
                    {
                        self.dispatch(Action::CloseTab);
                        ui.close();
                    }
                    if ui
                        .button(shortcut_hint(
                            &self.keybindings,
                            &ctx,
                            s.action_next_tab,
                            Action::NextTab,
                        ))
                        .clicked()
                    {
                        self.dispatch(Action::NextTab);
                        ui.close();
                    }
                }
                // Les derniers ouverts : on retravaille le même fichier
                // trois fois dans une séance, et le sélecteur natif
                // demande à chaque fois de retrouver son dossier.
                let récents = self.recent_files.clone();
                ui.add_enabled_ui(!récents.is_empty(), |ui| {
                    ui.menu_button(s.file_recent, |ui| {
                        for chemin in &récents {
                            let nom = chemin
                                .file_name()
                                .map(|nom| nom.to_string_lossy().into_owned())
                                .unwrap_or_else(|| chemin.to_string_lossy().into_owned());
                            // Le chemin entier au survol : deux fichiers
                            // peuvent porter le même nom dans deux
                            // dossiers, et c'est même le cas courant.
                            if ui
                                .button(nom)
                                .on_hover_text(chemin.to_string_lossy())
                                .clicked()
                            {
                                self.open_file(chemin.clone());
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button(s.file_recent_clear).clicked() {
                            self.recent_files.clear();
                            ui.close();
                        }
                    });
                });
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
                // Le programme assemblé, tel que `ld` l'a produit : de
                // quoi le lancer dans un vrai terminal et le comparer à
                // ce que namsdec en dit.
                ui.add_enabled_ui(
                    self.assemble_state
                        .as_ref()
                        .is_some_and(|state| !state.output.file.is_empty()),
                    |ui| {
                        if ui
                            .button(s.file_export_binary)
                            .on_hover_text(s.file_export_binary_hint)
                            .clicked()
                        {
                            self.spawn_file_dialog(FileDialogKind::ExportBinary);
                            ui.close();
                        }
                    },
                );
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
                // L'état du processeur, en texte : c'est ce qu'on colle
                // pour demander de l'aide. Un état qui ne sort pas de
                // l'écran se recopie à la main, donc mal.
                ui.add_enabled_ui(self.emulator.is_some(), |ui| {
                    if ui
                        .button(s.edit_copy_context)
                        .on_hover_text(s.edit_copy_context_hint)
                        .clicked()
                    {
                        if let Some(contexte) = self.context_text() {
                            ui.ctx().copy_text(contexte);
                            self.status = Status::Note(|s| s.context_copied);
                        }
                        ui.close();
                    }
                });
                ui.add_enabled_ui(self.assemble_state.is_some(), |ui| {
                    if ui.button(s.edit_copy_listing).clicked() {
                        if let Some(listing) = self.listing_text() {
                            ui.ctx().copy_text(listing);
                            self.status = Status::Note(|s| s.listing_copied);
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
                    ui.separator();
                    // Explorer un programme qu'on ne connaît pas : ce
                    // qu'il appelle, quand il en sort, ce qu'il demande
                    // au noyau. Un pas à pas de dix mille pas ne répond
                    // pas à ces questions-là.
                    for (action, label) in [
                        (Action::RunToCall, s.action_run_to_call),
                        (Action::RunToReturn, s.action_run_to_return),
                        (Action::RunToSyscall, s.action_run_to_syscall),
                    ] {
                        if ui
                            .button(shortcut_hint(
                                &self.keybindings,
                                &ui.ctx().clone(),
                                label,
                                action,
                            ))
                            .clicked()
                        {
                            self.dispatch(action);
                            ui.close();
                        }
                    }
                    ui.separator();
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
                // Le déroulement du programme : là où l'on comprend sa
                // forme, et non sa suite d'instructions.
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        &ui.ctx().clone(),
                        s.flow_open,
                        Action::ShowFlow,
                    ))
                    .clicked()
                {
                    self.show_flow = true;
                    ui.close();
                }
                // La décompilation : une entrée seulement si le module
                // est allumé. Un menu qui propose ce qui ne se fera pas
                // ment poliment.
                if self.show_decompiler_module && ui.button(s.decompiler_open).clicked() {
                    self.show_decompiler = true;
                    ui.close();
                }
                if ui.button(s.vectors_open).clicked() {
                    self.show_vectors = true;
                    ui.close();
                }
                if ui
                    .button(s.calculator_open)
                    .on_hover_text(s.calculator_hint)
                    .clicked()
                {
                    self.show_calculator = true;
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
                // Rendre la hauteur à ce qu'elle dit : une console qu'on
                // a agrandie pour lire vingt lignes n'a plus de raison
                // d'occuper autant devant deux.
                let mut automatique = self.console_height.is_none();
                if ui
                    .add_enabled(
                        self.show_console,
                        egui::Checkbox::new(&mut automatique, s.view_console_auto),
                    )
                    .on_hover_text(s.view_console_auto_hint)
                    .changed()
                {
                    // Décochée, la case rend la console à une hauteur
                    // fixe — celle qu'elle avait avant qu'on sache la
                    // calculer.
                    self.console_height = if automatique {
                        None
                    } else {
                        Some(CONSOLE_HEIGHT)
                    };
                }
                // Et la même chose pour le panneau latéral : une poignée
                // tirée une fois se reprend, sinon le partage automatique
                // est perdu pour de bon.
                let mut partagé = self.sidebar_tab_height.is_none();
                if ui
                    .add_enabled(
                        self.show_sidebar,
                        egui::Checkbox::new(&mut partagé, s.view_sidebar_auto),
                    )
                    .on_hover_text(s.view_sidebar_auto_hint)
                    .changed()
                {
                    // Décochée, la part garde la hauteur qu'elle a
                    // maintenant : décocher ne doit rien faire sauter
                    // sous les yeux.
                    self.sidebar_tab_height = if partagé {
                        None
                    } else {
                        Some(SIDEBAR_TAB_MIN.max(self.sidebar_tab_last))
                    };
                }

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

            // La barre de menus *est* la barre de titre : tout ce qui
            // reste après les menus se saisit pour déplacer la fenêtre,
            // porte le nom du fichier, et le double-clic l'agrandit. Une
            // barre de titre du système par-dessus celle-ci ferait deux
            // rangées pour le même travail.
            if !self.system_titlebar {
                let maximisée = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                let fichier = self
                    .current_file
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| s.untitled_buffer.to_owned());
                chrome::title_drag_ui(
                    ui,
                    self.theme,
                    &fichier,
                    self.source != self.saved_source,
                    maximisée,
                );
            }
        });
        if let Some(bouton) = bouton_fenêtre {
            self.window_button(bouton);
        }
    }

    /// Ce que fait chacune des trois pastilles.
    ///
    /// La fermeture passe par la même garde que « Quitter » : un travail
    /// non enregistré ne part pas parce qu'on a visé un rond rouge.
    fn window_button(&mut self, bouton: chrome::WindowButton) {
        match bouton {
            chrome::WindowButton::Minimize => self.window_command = Some(WindowCommand::Minimize),
            chrome::WindowButton::Maximize => self.window_command = Some(WindowCommand::Maximize),
            chrome::WindowButton::Close => self.guard(Pending::Quit),
        }
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
                decompiler_module: self.show_decompiler_module,
                installer_module: self.show_installer_module,
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
            self.show_decompiler_module = snapshot.decompiler_module;
            self.show_installer_module = snapshot.installer_module;
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
            // Un identifiant explicite : sans lui, egui dérive celui de
            // la fenêtre de son *titre*, qui change avec la langue —
            // passer en anglais lui faisait perdre sa position et sa
            // taille.
            .id(egui::Id::new("preferences_window"))
            .open(&mut open)
            .resizable(true)
            // Pas de flèche de repli : replier une boîte de préférences
            // ne laisse qu'un bandeau de titre qui flotte, sans usage.
            .collapsible(false)
            // Une taille posée, et au centre. Sans taille, egui ouvrait
            // la boîte là où il restait de la place — trois cent
            // cinquante pixels de haut, où cinq fiches ne tenaient pas.
            .default_size([780.0, 760.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.preferences_title) {
                    close_keeping = Some(true);
                }
                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (PreferencesTab::General, s.preferences_tab_general),
                        (PreferencesTab::Shortcuts, s.preferences_tab_shortcuts),
                        (PreferencesTab::Modules, s.preferences_tab_modules),
                        (PreferencesTab::Plugins, s.preferences_tab_plugins),
                        (PreferencesTab::Tools, s.preferences_tab_tools),
                    ] {
                        ui.selectable_value(&mut self.preferences_tab, tab, label);
                    }
                });
                ui.separator();
                // Une seule zone qui défile, et qui s'arrête avant le
                // bas : les pages avaient chacune la leur, prenant toute
                // la hauteur offerte, et les boutons OK / Quitter
                // sortaient de la fenêtre par le bas.
                egui::ScrollArea::vertical()
                    .id_salt("preferences_page")
                    .auto_shrink([false, true])
                    .max_height((ui.available_height() - 44.0).max(160.0))
                    .show(ui, |ui| match self.preferences_tab {
                        PreferencesTab::General => self.preferences_general_ui(ui),
                        PreferencesTab::Shortcuts => self.preferences_shortcuts_ui(ui),
                        PreferencesTab::Modules => self.preferences_modules_ui(ui),
                        PreferencesTab::Plugins => self.preferences_plugins_ui(ui),
                        PreferencesTab::Tools => self.preferences_tools_ui(ui),
                    });
                ui.separator();
                // Les préférences s'appliquent à mesure qu'on coche :
                // « Annuler » n'y défait rien, et un bouton qui ne défait
                // rien ment. D'où « Quitter » — qui dit ce qu'il fait —,
                // et non « Annuler ».
                // `or` et non une affectation sèche : la pastille du
                // bandeau a peut-être déjà répondu plus haut.
                close_keeping = close_keeping.or_else(|| {
                    dialog_bar(ui, theme, s.dialog_ok, Some(s.dialog_quit))
                        .map(|réponse| réponse == Answer::Ok)
                });
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

        // La décoration de la fenêtre : celle que namsdec dessine — les
        // menus font office de barre de titre — ou celle du bureau.
        ui.checkbox(&mut self.system_titlebar, s.preferences_system_titlebar);
        ui.weak(s.preferences_system_titlebar_hint);

        ui.add_space(6.0);
        ui.checkbox(&mut self.heat_legend, s.preferences_heat_legend);
        ui.weak(s.preferences_heat_legend_hint);

        ui.add_space(6.0);
        ui.checkbox(&mut self.show_tooltips, s.preferences_tooltips);
        ui.weak(s.preferences_tooltips_hint);

        ui.add_space(6.0);
        ui.checkbox(&mut self.completion, s.preferences_completion);
        ui.weak(s.preferences_completion_hint);
        ui.checkbox(&mut self.auto_indent, s.preferences_auto_indent);
        ui.weak(s.preferences_auto_indent_hint);
        ui.checkbox(&mut self.auto_pairs, s.preferences_auto_pairs);
        ui.weak(s.preferences_auto_pairs_hint);
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

    /// Passe d'un jeu de raccourcis à l'autre.
    ///
    /// Ce qu'on a déplacé soi-même est gardé : changer de jeu répond à
    /// « d'où je pars », non à « efface ce que j'ai réglé ».
    fn apply_keymap_preset(&mut self, preset: keybindings::Preset) {
        let écarts = settings::Keymap::from_bindings(self.keymap_preset, &self.keybindings);
        self.keymap_preset = preset;
        let keymap = settings::Keymap {
            preset: preset.id().to_owned(),
            shortcuts: écarts.shortcuts,
        };
        self.keybindings = keymap.bindings();
        let _ = settings::save_keymap(&keymap);
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
            // Écrit tout de suite : un raccourci qu'on vient de poser et
            // qui disparaît au prochain lancement se repose deux fois.
            let _ = settings::save_keymap(&settings::Keymap::from_bindings(
                self.keymap_preset,
                &self.keybindings,
            ));
        }

        // Le jeu dont on part. Les deux sont dans le binaire : en changer
        // remplace les raccourcis qu'on n'a pas déplacés soi-même, et
        // garde les autres.
        let fr = self.lang == Lang::Fr;
        ui.horizontal_wrapped(|ui| {
            ui.strong(s.shortcuts_preset);
            for preset in keybindings::Preset::ALL {
                let choisi = self.keymap_preset == preset;
                if ui
                    .selectable_label(choisi, preset.label(fr))
                    .on_hover_text(preset.about(fr))
                    .clicked()
                    && !choisi
                {
                    self.apply_keymap_preset(preset);
                }
            }
        });
        ui.weak(self.keymap_preset.about(fr));
        if let Some(chemin) = settings::keymap_path() {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, egui::RichText::new(s.shortcuts_file).small());
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(chemin.display().to_string())
                            .monospace()
                            .small()
                            .color(theme.dim),
                    )
                    .selectable(true),
                );
            });
        }
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

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
        {
            {
                self.modules_ui(ui);
                ui.checkbox(&mut self.show_re_tab, s.show_re_tab_checkbox);
                ui.checkbox(&mut self.exercises, s.exercises_checkbox);
                ui.weak(s.exercises_hint);
                ui.add_space(6.0);
                // Ce module-ci n'ajoute pas un affichage : il accorde la
                // permission de lancer un programme extérieur. On le dit
                // avant de cocher, pas après.
                ui.checkbox(&mut self.show_decompiler_module, s.decompiler_checkbox);
                ui.weak(s.decompiler_hint);
                // Et celui-ci va plus loin encore : il télécharge et
                // construit. Il ne s'allume que sous l'autre, parce
                // qu'installer un décompilateur sans pouvoir s'en servir
                // n'aurait aucun sens.
                ui.add_enabled_ui(self.show_decompiler_module, |ui| {
                    ui.checkbox(&mut self.show_installer_module, s.installer_checkbox);
                    ui.weak(s.installer_hint);
                });
                if self.show_decompiler_module && self.decompilers.is_empty() {
                    ui.colored_label(self.theme.warn, s.decompiler_none_installed);
                }
            }
        }
    }

    /// L'onglet des outils : tout ce qui vient d'ailleurs et s'exécute.
    fn preferences_tools_ui(&mut self, ui: &mut egui::Ui) {
        self.known_tools_ui(ui);
    }

    /// Les outils d'analyse connus : présents ou non, et déclarables
    /// d'un clic.
    ///
    /// Écrire un fichier JSON à la main pour brancher Rizin serait une
    /// barrière absurde — les arguments qui marchent sont les mêmes pour
    /// tout le monde. Le bouton **écrit ce fichier**, et rien d'autre :
    /// il n'installe pas le programme, il ne le lance pas. Quand le
    /// programme manque, namsdec dit ce qu'il faut taper pour l'obtenir
    /// et laisse taper.
    fn known_tools_ui(&mut self, ui: &mut egui::Ui) {
        use namsdec_disasm::known::Availability;
        let s = strings(self.lang);
        let theme = self.theme;
        let fr = self.lang == Lang::Fr;

        ui.label(egui::RichText::new(s.tools_title).strong().size(15.0));
        ui.add_space(2.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.colored_label(theme.dim, s.tools_legend);
        ui.add_space(10.0);

        let utilisés: std::collections::BTreeSet<String> =
            self.decompilers.iter().map(|p| p.id.clone()).collect();
        let mut à_utiliser = None;
        let mut à_retirer = None;
        let mut plan = None;
        let mut allumer = false;

        // Le dossier où namsdec pose ce qu'il installe : un outil bâti
        // par le module d'installation n'est dans le PATH de personne, et
        // sans cela il restait « absent » alors qu'il tournait très bien.
        let outils = data_dir().map(|dir| namsdec_disasm::install::tools_dir(&dir));
        for outil in namsdec_disasm::known::all() {
            let dispo = outil.availability_in(outils.as_deref());
            let sert = utilisés.contains(&outil.plugin.id);
            // Une fiche par outil. La grille d'avant mettait sur une même
            // ligne un nom, un état, une commande d'installation et deux
            // boutons : quatre colonnes qui ne s'alignaient pas et dont
            // aucune n'expliquait les autres.
            egui::Frame::new()
                .fill(theme.text.gamma_multiply(0.035))
                .stroke(egui::Stroke::new(
                    1.0,
                    if sert {
                        theme.accent.gamma_multiply(0.5)
                    } else {
                        theme.dim.gamma_multiply(0.3)
                    },
                ))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    // Le nom, les deux états, et l'action à droite, sur
                    // une seule ligne : c'est ce qu'on lit en diagonale
                    // pour savoir où en est un outil.
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&outil.plugin.name).strong());
                        let (couleur, état) = match dispo {
                            Availability::Ready => (theme.good, s.tools_state_found),
                            Availability::Incomplete => (theme.warn, s.tools_state_partial),
                            Availability::Missing => (theme.dim, s.tools_state_absent),
                        };
                        pastille(ui, couleur, état);
                        pastille(
                            ui,
                            if sert { theme.accent } else { theme.dim },
                            if sert { s.tools_in_use } else { s.tools_unused },
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if sert {
                                if framed_button(ui, theme, s.tools_stop).clicked() {
                                    à_retirer = Some(outil.plugin.id.clone());
                                }
                            } else if ui
                                .add_enabled(
                                    dispo != Availability::Missing,
                                    egui::Button::new(
                                        egui::RichText::new(s.tools_use).color(theme.bg),
                                    )
                                    .fill(theme.accent)
                                    .corner_radius(egui::CornerRadius::from(6)),
                                )
                                .clicked()
                            {
                                à_utiliser = Some(outil.resolved_plugin(outils.as_deref()));
                            }
                        });
                    });
                    ui.add_space(4.0);

                    // À quoi il sert, en clair et non plus au survol.
                    ui.colored_label(theme.dim, if fr { outil.about_fr } else { outil.about_en });

                    // Ce qui empêche de s'en servir se lit sans rien
                    // déplier : c'est une panne, pas un détail.
                    if sert && dispo == Availability::Missing {
                        ui.add_space(4.0);
                        ui.colored_label(theme.bad, s.tools_declared_but_missing);
                    }
                    // Le prérequis ne se dit que s'il manque. Affiché
                    // toujours, il se lisait comme une panne : « Il lui
                    // manque le greffon rz-ghidra » juste à côté de
                    // « programme détecté », sur une machine où le
                    // greffon était installé et fonctionnait.
                    if dispo == Availability::Incomplete
                        && let Some(besoin) = if fr { outil.needs_fr } else { outil.needs_en }
                    {
                        ui.add_space(4.0);
                        ui.colored_label(theme.warn, format!("{} {besoin}", s.tools_needs));
                    }

                    // Le reste se déplie : la commande exacte, et comment
                    // obtenir ce qui manque. Cinq outils tout déplié, et
                    // l'on ne voyait déjà plus le troisième.
                    ui.add_space(2.0);
                    egui::CollapsingHeader::new(
                        egui::RichText::new(s.tools_details)
                            .small()
                            .color(theme.dim),
                    )
                    .id_salt(format!("outil_{}", outil.plugin.id))
                    // Déplié d'office quand l'outil manque : la façon de
                    // l'obtenir est alors la seule chose qu'on cherche.
                    .default_open(dispo != Availability::Ready)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(s.tools_command).small(),
                            );
                            ui.add(
                                egui::Label::new(
                                    // Le programme **résolu**, et non son
                                    // nom nu : quand namsdec l'a installé
                                    // lui-même, c'est son chemin à lui
                                    // qui sera lancé, et annoncer
                                    // « rizin » désignerait celui du
                                    // système — un autre programme.
                                    egui::RichText::new({
                                        let résolu = outil.resolved_plugin(outils.as_deref());
                                        format!("{} {}", résolu.command, résolu.args.join(" "))
                                    })
                                    .monospace()
                                    .small()
                                    .color(theme.text),
                                )
                                .selectable(true),
                            );
                        });
                        if dispo != Availability::Ready
                            && let Some(commande) = &outil.plugin.install
                        {
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(
                                    theme.dim,
                                    egui::RichText::new(s.tools_get_it).small(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(
                                            namsdec_disasm::install::install_command_here(
                                                commande,
                                            ),
                                        )
                                        .monospace()
                                        .small()
                                        .color(theme.warn),
                                    )
                                    .selectable(true),
                                );
                            });
                        }
                    });

                    // Aller le chercher reste une action, pas un détail :
                    // le bouton ne se replie pas.
                    if dispo != Availability::Ready && recette_existe(&outil.plugin.id) {
                        ui.add_space(4.0);
                        if self.show_installer_module {
                            if framed_button(ui, theme, s.install_button).clicked() {
                                plan = Some(outil.plugin.id.clone());
                            }
                        } else {
                            // Le bouton manquait sans un mot : on croyait
                            // que namsdec ne savait pas installer cet
                            // outil, alors qu'il attendait une case
                            // cochée ailleurs.
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(
                                    theme.dim,
                                    egui::RichText::new(s.tools_installer_off).small(),
                                );
                                if framed_button(ui, theme, s.tools_turn_on).clicked() {
                                    allumer = true;
                                }
                            });
                        }
                    }
                });
            ui.add_space(6.0);
        }

        if let Some(plugin) = à_utiliser {
            self.declare_tool(&plugin);
        }
        if let Some(id) = à_retirer {
            self.undeclare_tool(&id);
        }
        if let Some(id) = plan {
            self.install_plan = Some(id);
        }
        if allumer {
            // Installer suppose de pouvoir s'en servir : les deux
            // modules s'allument ensemble.
            self.show_decompiler_module = true;
            self.show_installer_module = true;
        }
    }

    /// La fenêtre qui montre une recette, avant que rien ne soit lancé.
    ///
    /// C'est le cœur de ce module : ce qu'on va exécuter est écrit là,
    /// commande par commande, avec d'où cela vient, ce que cela coûte et
    /// où cela sera posé. Le bouton n'est qu'en dessous.
    fn installer_window_ui(&mut self, ctx: &egui::Context) {
        let Some(outil) = self.install_plan.clone() else {
            return;
        };
        if !self.show_installer_module {
            self.install_plan = None;
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let fr = self.lang == Lang::Fr;
        let Some(data) = data_dir() else {
            self.install_plan = None;
            return;
        };
        let Some(recette) = namsdec_disasm::install::recipe_for(&outil, &data) else {
            self.plugin_note = Some(PluginNote {
                ok: false,
                text: s.install_no_recipe.to_owned(),
            });
            self.install_plan = None;
            return;
        };
        let manquants = namsdec_disasm::install::missing_requirements(&recette);

        let mut ouverte = true;
        let mut lancer = false;
        let mut arrêter = false;
        let mut fermer = false;
        // L'outil est-il déjà là ? Demandé à la machine, non déduit du
        // fait qu'un travail s'est terminé : une installation peut
        // aboutir sans que l'outil marche, et l'inverse aussi — celui du
        // système peut l'avoir précédée.
        let outils = namsdec_disasm::install::tools_dir(&data);
        let déjà_prêt = namsdec_disasm::known::all()
            .into_iter()
            .find(|connu| connu.plugin.id == outil)
            .map(|connu| {
                connu.availability_in(Some(&outils)) == namsdec_disasm::known::Availability::Ready
            })
            .unwrap_or(false);
        let en_cours = self
            .install_job
            .as_ref()
            .is_some_and(|job| job.tool == outil);
        let fait = self.install_job.as_ref().map(|job| job.done).unwrap_or(0);

        egui::Window::new(format!("{} — {}", s.install_title, recette.tool))
            .id(egui::Id::new("installer_window"))
            .open(&mut ouverte)
            // Au premier plan : elle s'ouvre depuis les préférences, qui
            // occupent le centre de l'écran. Rangée comme une fenêtre
            // ordinaire, elle se posait derrière elles — on cliquait
            // « Installer… » et il ne se passait rien de visible.
            .order(egui::Order::Foreground)
            .resizable(true)
            .collapsible(false)
            .default_size([700.0, 620.0])
            // Bornée à l'écran, et c'est ce qui manquait : le plan de
            // rizin compte neuf étapes et un journal, si bien que la
            // fenêtre dépassait en haut comme en bas. Sa barre de titre
            // passait hors champ — avec la croix qui s'y trouve —, et
            // l'on ne pouvait plus la fermer du tout.
            .max_size(egui::vec2(
                ctx.viewport_rect().width() * 0.9,
                ctx.viewport_rect().height() * 0.86,
            ))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, format!("{} — {}", s.install_title, recette.tool).as_str()) {
                    fermer = true;
                }
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                // La zone qui défile s'arrête avant le bas : sans cette
                // borne, elle prend toute la hauteur offerte et pousse
                // la barre d'action — l'avertissement et le bouton —
                // hors de l'écran.
                egui::ScrollArea::vertical()
                    .id_salt("installer_page")
                    .auto_shrink([false, true])
                    // Moins que la moitié : au-dessus, l'avertissement
                    // et les boutons se retrouvaient sous le bord de
                    // l'écran, journal déplié.
                    .max_height(ctx.viewport_rect().height() * 0.5)
                    .show(ui, |ui| {
                        // --- Ce qu'on va gagner ----------------------
                        // D'abord la leçon, ensuite la mécanique : une
                        // fenêtre qui ne dit que « voulez-vous
                        // installer ? » demande un acte de foi.
                        encadré(ui, theme.accent, |ui| {
                            ui.label(
                                egui::RichText::new(s.install_what_for)
                                    .strong()
                                    .color(theme.accent),
                            );
                            ui.add_space(3.0);
                            ui.label(if fr {
                                recette.teaches_fr
                            } else {
                                recette.teaches_en
                            });
                        });
                        ui.add_space(8.0);

                        // --- Les faits, en quatre lignes lisibles ----
                        //
                        // Pas de Grid ici : une cellule de grille laisse
                        // le texte s'étendre à l'infini vers la droite,
                        // et une phrase longue — le coût, justement —
                        // sort de la fenêtre sans jamais revenir à la
                        // ligne. Une colonne de titres à largeur fixe,
                        // puis une valeur qui se replie dans ce qui
                        // reste : c'est ce qui se lit.
                        for (rang, (titre, valeur, mono)) in [
                            (s.install_source, recette.source.to_owned(), false),
                            (
                                s.install_cost,
                                if fr { recette.cost_fr } else { recette.cost_en }.to_owned(),
                                false,
                            ),
                            (
                                s.install_where,
                                recette.produces.display().to_string(),
                                true,
                            ),
                            (s.install_requires, recette.requires.join(", "), true),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let fond = if rang % 2 == 0 {
                                theme.text.gamma_multiply(0.04)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            egui::Frame::new()
                                .fill(fond)
                                .inner_margin(egui::Margin::symmetric(6, 4))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal_top(|ui| {
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(150.0, 0.0),
                                            egui::Layout::top_down(egui::Align::RIGHT),
                                            |ui| {
                                                ui.colored_label(theme.dim, titre);
                                            },
                                        );
                                        let texte = egui::RichText::new(valeur);
                                        ui.add(
                                            egui::Label::new(if mono {
                                                texte.monospace().small()
                                            } else {
                                                texte
                                            })
                                            .wrap()
                                            .selectable(true),
                                        );
                                    });
                                });
                        }
                        if !manquants.is_empty() {
                            ui.add_space(4.0);
                            ui.colored_label(
                                theme.bad,
                                format!("{} {}", s.install_missing, manquants.join(", ")),
                            );
                            // Le bouton reste gris tant qu'il manque
                            // quelque chose : sans la commande à poser,
                            // la fenêtre était un cul-de-sac où l'on
                            // apprenait seulement qu'on ne pouvait rien
                            // faire.
                            if let Some(commande) =
                                namsdec_disasm::install::requirement_command(&manquants)
                            {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(s.install_requirements_hint)
                                        .small()
                                        .color(theme.dim),
                                );
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&commande).monospace(),
                                        )
                                        .selectable(true),
                                    );
                                    let copié = ui
                                        .ctx()
                                        .data(|d| d.get_temp::<bool>(copie_id()))
                                        .unwrap_or(false);
                                    let étiquette =
                                        if copié { s.install_copied } else { s.install_copy };
                                    if framed_button(ui, theme, étiquette).clicked() {
                                        ui.ctx().copy_text(commande.clone());
                                        ui.ctx()
                                            .data_mut(|d| d.insert_temp(copie_id(), true));
                                    }
                                });
                            }
                        }
                        ui.add_space(8.0);

                        // --- Le plan, étape par étape ----------------
                        ui.label(egui::RichText::new(s.install_plan).strong());
                        ui.add_space(4.0);
                        for (rang, étape) in recette.steps.iter().enumerate() {
                            // Chaque étape porte son état : faite, en
                            // cours, à venir. C'est ce qui transforme
                            // une liste en compte rendu.
                            let (marque, couleur) = if !en_cours {
                                ("•", theme.dim)
                            } else if rang < fait {
                                ("v", theme.good)
                            } else if rang == fait {
                                (">", theme.accent)
                            } else {
                                ("•", theme.dim)
                            };
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    couleur,
                                    egui::RichText::new(marque).monospace().strong(),
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}. {}",
                                            rang + 1,
                                            if fr {
                                                &étape.about_fr
                                            } else {
                                                &étape.about_en
                                            }
                                        ))
                                        .color(
                                            if rang == fait && en_cours {
                                                theme.text
                                            } else {
                                                theme.dim
                                            },
                                        ),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(étape.line())
                                                .monospace()
                                                .small()
                                                .color(theme.dim),
                                        )
                                        .selectable(true),
                                    );
                                });
                            });
                            ui.add_space(3.0);
                        }

                        // --- Le journal, quand ça tourne -------------
                        if let Some(job) = &self.install_job
                            && job.tool == outil
                        {
                            let lignes = job.log.lock().expect("le journal").clone();
                            if !lignes.is_empty() {
                                ui.add_space(6.0);
                                egui::CollapsingHeader::new(s.install_log)
                                    .id_salt("install_log_fold")
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        egui::Frame::new()
                                            .fill(ui.visuals().extreme_bg_color)
                                            .inner_margin(egui::Margin::same(6))
                                            .corner_radius(4.0)
                                            .show(ui, |ui| {
                                                egui::ScrollArea::vertical()
                                                    .id_salt("install_log")
                                                    .max_height(180.0)
                                                    .auto_shrink([false, false])
                                                    .stick_to_bottom(true)
                                                    .show(ui, |ui| {
                                                        for ligne in &lignes {
                                                            let couleur = if ligne.starts_with('$')
                                                            {
                                                                theme.accent
                                                            } else {
                                                                theme.dim
                                                            };
                                                            ui.colored_label(
                                                                couleur,
                                                                egui::RichText::new(ligne)
                                                                    .monospace()
                                                                    .small(),
                                                            );
                                                        }
                                                    });
                                            });
                                    });
                            }
                        }
                    });

                // --- Ce que les commandes ont écrit ------------------
                //
                // Replié par défaut : on ne l'ouvre que lorsqu'on veut
                // savoir. Mais il est là, et c'est tout ce qui permet de
                // comprendre un échec — « l'étape 2 a échoué (code 128) »
                // ne dit pas que git a trouvé le dossier déjà occupé.
                let journal: Vec<String> = if en_cours {
                    self.install_job
                        .as_ref()
                        .map(|job| job.log.lock().expect("le journal").clone())
                        .unwrap_or_default()
                } else {
                    self.install_log.clone()
                };
                if !journal.is_empty() {
                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(s.install_log_title)
                        .id_salt("install_log")
                        .default_open(!en_cours && self.plugin_note.as_ref().is_some_and(|n| !n.ok))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("install_log_scroll")
                                .max_height(160.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for ligne in &journal {
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(ligne).monospace().small(),
                                            )
                                            .selectable(true),
                                        );
                                    }
                                });
                            if framed_button(ui, theme, s.install_copy_log).clicked() {
                                ui.ctx().copy_text(journal.join("\n"));
                            }
                        });
                }

                // --- Ce qu'on fait, tout en bas et toujours visible --
                //
                // L'avertissement voisine le bouton : plus haut, il
                // serait sorti de l'écran au moment de cliquer, et un
                // avertissement qu'on ne lit pas n'en est pas un.
                ui.add_space(6.0);
                encadré(ui, theme.warn, |ui| {
                    ui.colored_label(theme.warn, s.install_warning);
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if en_cours {
                        ui.spinner();
                        ui.colored_label(theme.dim, s.install_running);
                        ui.add(
                            egui::ProgressBar::new(fait as f32 / recette.steps.len().max(1) as f32)
                                .desired_width(200.0)
                                .text(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        fait,
                                        recette.steps.len()
                                    ))
                                    .small(),
                                ),
                        );
                        if framed_button(ui, theme, s.install_cancel).clicked() {
                            arrêter = true;
                        }
                    } else if déjà_prêt {
                        // Installé, et vérifié : le bouton principal
                        // devient « Fermer ». Il invitait à recommencer,
                        // si bien qu'on ne savait pas si l'installation
                        // avait abouti — la seule façon de s'en assurer
                        // était de tout relancer.
                        //
                        // Recommencer reste possible en second : c'est le
                        // geste de quelqu'un qui sait pourquoi il le fait.
                        ui.colored_label(theme.good, s.install_already_done);
                        // « Recommencer » n'est PAS le second bouton de la
                        // barre, et c'est une leçon qu'un test a rendue :
                        // Échap y répond `Cancel`, ce qui lançait une
                        // recompilation de vingt minutes au lieu de
                        // fermer. Échap veut dire « je m'en vais », jamais
                        // « fais l'autre chose ».
                        //
                        // La barre ne porte donc que le geste principal et
                        // le renoncement. Une troisième action se pose à
                        // côté, et se clique exprès.
                        if framed_button(ui, theme, s.install_again)
                            .on_hover_text(s.install_again_hint)
                            .clicked()
                        {
                            lancer = true;
                        }
                        if dialog_bar(ui, theme, s.dialog_close, None).is_some() {
                            fermer = true;
                        }
                    } else {
                        // Rien d'installé : le principal est « Lancer »,
                        // et « Fermer » est la sortie. La croix de la
                        // barre de titre ne suffit pas — elle se retrouve
                        // loin du regard sur un plan de neuf étapes.
                        ui.add_enabled_ui(manquants.is_empty(), |ui| {
                            match dialog_bar(ui, theme, s.install_start, Some(s.dialog_close)) {
                                Some(Answer::Ok) => lancer = true,
                                Some(Answer::Cancel) => fermer = true,
                                None => {}
                            }
                        });
                        ui.colored_label(theme.dim, s.install_nothing_yet);
                    }
                });
            });

        if lancer {
            self.start_install(recette);
        }
        if arrêter && let Some(job) = &self.install_job {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        // Échap est pris par la barre d'action — elle le consomme, donc
        // il n'y a rien à guetter ici. Pendant l'installation, la barre
        // n'est pas affichée : on n'interrompt pas un `ninja` par
        // mégarde, et c'est « Arrêter » qui le fait, explicitement.
        if !ouverte || fermer {
            self.install_plan = None;
        }
    }

    /// Écrit la description d'un outil dans le dossier des
    /// décompilateurs. Aucun programme n'est téléchargé, aucun n'est
    /// lancé : on pose un fichier de quelques lignes.
    fn declare_tool(&mut self, plugin: &namsdec_disasm::plugin::Plugin) {
        let s = strings(self.lang);
        let Some(dir) = decompilers_dir() else {
            self.plugin_note = Some(PluginNote {
                ok: false,
                text: s.tools_no_dir.to_owned(),
            });
            return;
        };
        let écrire = || -> std::io::Result<()> {
            std::fs::create_dir_all(&dir)?;
            let json = serde_json::to_string_pretty(plugin)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            std::fs::write(dir.join(format!("{}.json", plugin.id)), json)
        };
        match écrire() {
            Ok(()) => {
                self.decompilers = load_installed_decompilers();
                // Le premier déclaré devient celui qu'on lancera : sans
                // cela, la fenêtre s'ouvre sur « choisir… » alors qu'il
                // n'y a qu'un choix.
                if self.decompiler.is_empty() {
                    self.decompiler = plugin.id.clone();
                }
                self.plugin_note = Some(PluginNote {
                    ok: true,
                    text: format!("{} {}", s.tools_declared, plugin.name),
                });
            }
            Err(e) => {
                self.plugin_note = Some(PluginNote {
                    ok: false,
                    text: format!("{} {e}", s.tools_failed),
                });
            }
        }
    }

    /// Retire la description d'un outil : le fichier s'en va, le
    /// programme reste là où il est.
    fn undeclare_tool(&mut self, id: &str) {
        let s = strings(self.lang);
        if let Some(dir) = decompilers_dir() {
            let _ = std::fs::remove_file(dir.join(format!("{id}.json")));
        }
        self.decompilers = load_installed_decompilers();
        if self.decompiler == id {
            self.decompiler = self
                .decompilers
                .first()
                .map(|p| p.id.clone())
                .unwrap_or_default();
        }
        self.plugin_note = Some(PluginNote {
            ok: true,
            text: s.tools_undeclared.to_owned(),
        });
    }

    /// Onglet Plugins : pour l'instant les thèmes, qui en sont le premier
    /// type. Un thème est une simple palette (`namsdec-theme`), donc le
    /// jour où d'autres s'installent, ils se contentent d'allonger la
    /// liste — rien ici n'est écrit par thème.
    fn preferences_plugins_ui(&mut self, ui: &mut egui::Ui) {
        // Toute la page défile d'un seul tenant. Auparavant, seule la
        // liste des thèmes le faisait : ce qui venait après — les outils
        // d'analyse, les séries d'exercices — se retrouvait poussé hors
        // de la fenêtre, présent mais invisible. Une section qu'on ne
        // peut pas atteindre n'existe pas.
        self.plugins_page_ui(ui);
    }

    /// Le contenu de la page des plugins, dans l'ordre où on le lit.
    fn plugins_page_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // Le thème *de l'interface*, à ne pas confondre avec ceux que la
        // page liste — nommés `listed` plus bas pour cette raison.
        let theme = self.theme;
        let ctx = ui.ctx().clone();

        ui.strong(s.plugins_themes);
        ui.weak(s.plugins_themes_hint);
        ui.add_space(6.0);

        {
            {
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
            }
        }

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

    /// L'icône de l'application, montée en texture à la première
    /// ouverture de la boîte puis gardée.
    fn app_icon_texture(&mut self, ctx: &egui::Context) -> egui::TextureHandle {
        self.icon_texture
            .get_or_insert_with(|| {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [APP_ICON_SIDE, APP_ICON_SIDE],
                    APP_ICON,
                );
                ctx.load_texture("namsdec-icon", image, egui::TextureOptions::LINEAR)
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
            // La prochaine **non réussie**, et non la suivante dans la
            // liste : reprendre un parcours ne doit pas obliger à
            // repasser sur ce qu'on a déjà fait. À défaut — tout est
            // réussi jusqu'au bout —, celle qui suit.
            let suivant = pack
                .exercises
                .iter()
                .skip(position + 1)
                .find(|exercise| {
                    !self
                        .exercises_done
                        .contains(&namsdec_exercises::full_id(pack, exercise))
                })
                .or_else(|| pack.exercises.get(position + 1))?;
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

    /// La calculatrice : une expression, et toutes ses lectures.
    ///
    /// Elle répond pendant qu'on tape, sans bouton « égale ». C'est ce
    /// qui la rend utile : on essaie `0xff & 0x0f`, on voit, on corrige,
    /// on voit. Un bouton imposerait un aller-retour à chaque essai.
    ///
    /// Les registres du programme en cours sont des noms comme les
    /// autres : `rax & 0xff` se lit comme on le pense, et c'est très
    /// exactement la question qu'on se pose devant un `and eax, 0xff`.
    /// La calculatrice : un champ, un clavier, une grille de bits.
    ///
    /// Taillée comme celle de GNOME en mode Programmation, parce que
    /// c'est la disposition qu'on connaît : l'expression en haut, le
    /// résultat en grand, les soixante-quatre bits dessous, et un
    /// clavier qui écrit ce que l'évaluateur lit — `&`, `|`, `<<`, et
    /// non des symboles mathématiques qu'il faudrait retraduire pour les
    /// recopier dans du code.
    ///
    /// S'y ajoute ce qu'aucune calculatrice de bureau ne fait : les
    /// registres du programme en cours, et les dix-huit fonctions qu'on
    /// écrit à la main en lisant un désassemblage.
    fn calculator_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_calculator {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang;
        let mut open = true;
        let mut fermer_ok = false;
        let regs = self.emulator.as_ref().map(|e| &e.cpu().regs);
        let résultat = calc::eval(&self.calc_input, regs);
        let mut à_copier: Option<String> = None;
        // Relevés dans la fenêtre, agis après : les fermetures empruntent
        // `self` en écriture, et la fenêtre le tient déjà.
        let mut frappe = Frappe::default();
        let mut bit_basculé: Option<u32> = None;

        let fenêtre = egui::Window::new(s.calculator_title)
            .id(egui::Id::new("calculator_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // Plus grande qu'avant : un clavier et une grille de bits ne
            // tiennent pas dans quatre cents pixels de haut.
            .default_size([520.0, 720.0])
            .min_size([440.0, 520.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center());
        let rendue = self
            .maximizable(fenêtre, Self::CALCULATOR_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.calculator_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, theme, s, "calculator") {
                    fermer_ok = true;
                }

                ui.horizontal(|ui| {
                    ui.colored_label(theme.dim, s.calculator_expression);
                    let champ = ui.add(
                        egui::TextEdit::singleline(&mut self.calc_input)
                            .hint_text(s.calculator_hint_text)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width()),
                    );
                    // Le curseur est dans le champ dès l'ouverture : on
                    // vient là pour taper, pas pour cliquer d'abord.
                    if !self.calculator_was_open {
                        champ.request_focus();
                    }
                });

                // Le résultat en grand, dans la base choisie et sur la
                // largeur choisie. C'est la ligne qu'on regarde, elle a
                // donc la taille qui va avec.
                let valeur = résultat.as_ref().ok().map(|v| *v & self.calc_width.mask());
                ui.add_space(6.0);
                // Tout ce qui suit est centré : le résultat, les bits,
                // les sélecteurs et le clavier. Alignés à gauche dans une
                // fenêtre large, ils flottaient contre un bord avec du
                // vide en face — une page, et non un appareil.
                ui.vertical_centered(|ui| match (&résultat, valeur) {
                    (Ok(_), Some(valeur)) => {
                        let texte = self.calc_base.écrire(valeur);
                        let réponse = ui.add(
                            egui::Label::new(
                                egui::RichText::new(&texte)
                                    .monospace()
                                    .size(26.0)
                                    .color(theme.accent),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if réponse.on_hover_text(s.calculator_copy).clicked() {
                            à_copier = Some(texte);
                        }
                    }
                    (Err(erreur), _) => {
                        // Vide, on ne reproche rien : la fenêtre vient de
                        // s'ouvrir, il n'y a pas d'erreur à signaler. La
                        // place reste prise, sinon tout ce qui suit
                        // sauterait d'une ligne à la première frappe.
                        if matches!(erreur, calc::CalcError::Empty) {
                            ui.add_space(32.0);
                        } else {
                            ui.colored_label(theme.bad, calc_error_text(s, erreur));
                        }
                    }
                    _ => {}
                });

                ui.add_space(8.0);
                calculator_bits_ui(
                    ui,
                    theme,
                    s,
                    valeur.unwrap_or(0),
                    self.calc_width,
                    valeur.is_some(),
                    &mut bit_basculé,
                );

                ui.add_space(8.0);
                // `vertical_centered` autour d'une rangée : c'est ainsi
                // qu'on centre un groupe de largeur naturelle dans egui,
                // qui ne sait centrer que les enfants d'une colonne.
                // Une boîte de largeur posée, dont le contenu est centré :
                // les libellés changent de longueur avec la langue, et
                // c'est la boîte qui se centre, non les mots.
                centered_row(ui, 380.0, |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(380.0, 24.0),
                        egui::Layout::left_to_right(egui::Align::Center)
                            .with_main_align(egui::Align::Center),
                        |ui| {
                        ui.colored_label(theme.dim, s.calculator_base_label);
                        egui::ComboBox::from_id_salt("calculator_base")
                            .selected_text(self.calc_base.label(s))
                            .show_ui(ui, |ui| {
                                for base in CalcBase::ALL {
                                    ui.selectable_value(&mut self.calc_base, base, base.label(s));
                                }
                            });
                        ui.separator();
                        ui.colored_label(theme.dim, s.calculator_width_label);
                        egui::ComboBox::from_id_salt("calculator_width")
                            .selected_text(format!("{} bits", self.calc_width.bits()))
                            .show_ui(ui, |ui| {
                                for largeur in CalcWidth::ALL {
                                    ui.selectable_value(
                                        &mut self.calc_width,
                                        largeur,
                                        format!("{} bits", largeur.bits()),
                                    );
                                }
                            });
                    });
                });

                ui.add_space(8.0);
                calculator_keypad_ui(ui, theme, s, self.calc_base, &self.calc_input, &mut frappe);

                ui.add_space(8.0);
                ui.separator();
                if regs.is_none() {
                    ui.colored_label(theme.dim, s.calculator_no_registers);
                }
                calculator_functions_ui(ui, theme, s, lang, &mut frappe.écrire);

                ui.add_space(8.0);
                ui.separator();
                // Toutes les bases à la fois, en petit : c'est ce qu'on
                // vient chercher quand on ne sait pas encore *laquelle*
                // on veut. Chacune se copie d'un clic.
                if let Some(valeur) = valeur {
                    egui::Grid::new("calculator_bases")
                        .num_columns(2)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            for (nom, texte) in readings::all_bases(lang, valeur) {
                                ui.colored_label(theme.dim, nom);
                                let réponse = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&texte).monospace().color(theme.text),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if réponse.on_hover_text(s.calculator_copy).clicked() {
                                    à_copier = Some(texte.clone());
                                }
                                ui.end_row();
                            }
                        });
                }
            });
        self.place_maximize_button(ctx, Self::CALCULATOR_WINDOW, rendue);

        if frappe.effacer {
            self.calc_input.clear();
        }
        if frappe.reculer {
            self.calc_input.pop();
        }
        if let Some(morceau) = frappe.écrire {
            self.calc_input.push_str(&morceau);
        }
        if let Some(rang) = bit_basculé
            && let Ok(valeur) = résultat
        {
            // Le bit basculé devient la nouvelle expression, écrite dans
            // la base courante : on repart de ce qu'on vient de poser,
            // comme sur une calculatrice de bureau.
            let nouvelle = (valeur ^ (1u64 << rang)) & self.calc_width.mask();
            self.calc_input = self.calc_base.écrire(nouvelle);
        }
        if let Some(texte) = à_copier {
            ctx.copy_text(texte);
            self.status = Status::Note(|s| s.calculator_copied);
        }
        self.calculator_was_open = self.show_calculator;
        self.show_calculator = open && !fermer_ok;
        if !open {
            self.calculator_was_open = false;
        }
    }

    fn exercises_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let mut open = self.show_exercises;
        let mut fermer_ok = false;
        let mut load = None;
        let mut fiche: Option<&'static str> = None;
        let fenêtre_exercises_window = egui::Window::new(s.exercises_title)
            // L'identifiant a changé de version, et il le fallait :
            // egui retient la taille d'une fenêtre par son identifiant,
            // si bien qu'élargir la taille de départ n'aurait rien
            // changé pour qui l'a déjà ouverte une fois. Le prix est
            // qu'elle reprend sa place au centre — pour une fenêtre
            // qu'on jugeait trop étroite, c'est ce qu'on veut.
            .id(egui::Id::new("exercises_window_v2"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // 480 px de large étaient trop peu : un énoncé y tombe sur
            // six lignes et la liste des séries se replie en accordéon.
            // La taille suit maintenant l'écran — quatre cinquièmes,
            // plafonnés — au lieu d'un chiffre fixe qui déborde d'un
            // petit écran et se perd sur un grand.
            .default_size(exercises_window_size(ctx))
            .min_size([420.0, 320.0])
            .pivot(egui::Align2::CENTER_CENTER)
            // `default_pos` et non `current_pos` : la seconde replace la
            // fenêtre à chaque frame, et l'on ne peut plus la déplacer.
            .default_pos(ctx.viewport_rect().center());
        let rendue_exercises_window = self
            .maximizable(fenêtre_exercises_window, Self::EXERCISES_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.exercises_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "exercises") {
                    fermer_ok = true;
                }
                // Aucune série lisible : le parcours est un fichier, et
                // ce fichier n'est pas là. Le dire, avec l'endroit où il
                // devrait être — c'est la seule chose utile ici.
                if self.exercise_packs.is_empty() {
                    ui.colored_label(theme.bad, s.exercises_missing);
                    if let Some(dir) = exercises_dir() {
                        ui.colored_label(
                            theme.dim,
                            egui::RichText::new(
                                dir.join(namsdec_exercises::BUILTIN_FILE)
                                    .display()
                                    .to_string(),
                            )
                            .monospace(),
                        );
                    }
                    ui.add_space(4.0);
                    ui.colored_label(theme.dim, s.exercises_missing_hint);
                    return;
                }
                ui.label(s.exercises_legend);
                // L'étape en cours, en tête : quatre-vingt-seize exercices ne tiennent
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
                // De quoi retrouver une étape : trente-six titres se
                // parcourent mal quand on sait déjà lequel on veut
                // revoir. La recherche porte sur le titre et sur
                // l'énoncé — on se souvient rarement du nom exact, plus
                // souvent de ce qu'on y faisait.
                ui.horizontal(|ui| {
                    ui.colored_label(theme.dim, s.exercises_search);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.exercise_query)
                            .desired_width(200.0)
                            .hint_text(s.exercises_search_hint),
                    );
                    if !self.exercise_query.is_empty() && ui.small_button("x").clicked() {
                        self.exercise_query.clear();
                    }
                });
                ui.add_space(4.0);
                // La place que la fenêtre laisse vraiment, et pas une
                // fraction de l'écran : une zone qui se croit plus haute
                // que sa fenêtre ne défile pas — elle déborde, et ce qui
                // dépasse est coupé sans qu'aucune barre ne le dise.
                let room = ui.available_height().max(120.0);
                egui::ScrollArea::vertical()
                    .max_height(room)
                    // `false` en hauteur : une zone qui rétrécit à son
                    // contenu est aussi haute que ses quatre-vingt-seize exercices,
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
                            // Combien de faits sur combien : le premier
                            // chiffre qu'on cherche en rouvrant un
                            // parcours.
                            let faits = pack
                                .exercises
                                .iter()
                                .filter(|exercise| {
                                    self.exercises_done
                                        .contains(&namsdec_exercises::full_id(pack, exercise))
                                })
                                .count();
                            let sous_budget = pack
                                .exercises
                                .iter()
                                .filter(|exercise| {
                                    let id = namsdec_exercises::full_id(pack, exercise);
                                    match (exercise.par, self.exercises_best.get(&id)) {
                                        (Some(budget), Some(meilleur)) => *meilleur < budget,
                                        _ => false,
                                    }
                                })
                                .count();
                            let header = egui::CollapsingHeader::new(
                                egui::RichText::new(format!(
                                    "{}  —  {faits}/{} {} · {sous_budget} {}",
                                    pack.name,
                                    pack.exercises.len(),
                                    s.exercises_done_count,
                                    s.par_under
                                ))
                                .color(theme.dim),
                            )
                            .id_salt(&pack.id)
                            .default_open(true);
                            let total = pack.exercises.len();
                            header.show(ui, |ui| {
                                let mut palier = None;
                                let cherché = self.exercise_query.to_lowercase();
                                for (index, exercise) in pack.exercises.iter().enumerate() {
                                    if !cherché.is_empty() {
                                        let titre = exercise.title.get(lang).to_lowercase();
                                        let énoncé = exercise.description.get(lang).to_lowercase();
                                        if !titre.contains(&cherché)
                                            && !énoncé.contains(&cherché)
                                            && !exercise.id.contains(&cherché)
                                        {
                                            continue;
                                        }
                                    }
                                    // Le palier, en tête de son groupe :
                                    // le parcours va de la découverte à
                                    // la maîtrise, et le dire évite de
                                    // prendre la vingt-huitième étape
                                    // pour la suite naturelle de la
                                    // première.
                                    if palier != Some(exercise.level) {
                                        palier = Some(exercise.level);
                                        ui.add_space(8.0);
                                        // Le palier, avec ce qu'il en
                                        // reste : quarante-quatre
                                        // étapes se lisent par groupes,
                                        // et « 8/10 » dit en un coup
                                        // d'œil où l'on en est.
                                        let (faits_ici, total_ici) = pack
                                            .exercises
                                            .iter()
                                            .filter(|autre| autre.level == exercise.level)
                                            .fold((0usize, 0usize), |(faits, total), autre| {
                                                let id = namsdec_exercises::full_id(pack, autre);
                                                (
                                                    faits
                                                        + usize::from(
                                                            self.exercises_done.contains(&id),
                                                        ),
                                                    total + 1,
                                                )
                                            });
                                        ui.horizontal(|ui| {
                                            ui.colored_label(
                                                theme.accent,
                                                egui::RichText::new(exercise.level.label(lang))
                                                    .strong(),
                                            );
                                            ui.colored_label(
                                                if faits_ici == total_ici {
                                                    theme.good
                                                } else {
                                                    theme.dim
                                                },
                                                format!("{faits_ici}/{total_ici}"),
                                            );
                                        });
                                    }
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
                                            // Une pastille pour ce qui est
                                            // fait : un parcours sans trace
                                            // de ce qu'on a déjà réussi
                                            // oblige à s'en souvenir soi-
                                            // même, et l'on refait le
                                            // premier trois fois.
                                            let fait = self.exercises_done.contains(&id);
                                            // Trois états, pas de
                                            // médailles : rien tenté,
                                            // réussi, réussi sous le
                                            // budget de la solution
                                            // livrée.
                                            let sous_budget = fait
                                                && match (
                                                    exercise.par,
                                                    self.exercises_best.get(&id),
                                                ) {
                                                    (Some(budget), Some(meilleur)) => {
                                                        *meilleur < budget
                                                    }
                                                    _ => false,
                                                };
                                            let (pastille, _) = ui.allocate_exact_size(
                                                egui::vec2(10.0, 10.0),
                                                egui::Sense::hover(),
                                            );
                                            if fait {
                                                ui.painter().circle_filled(
                                                    pastille.center(),
                                                    4.0,
                                                    theme.good,
                                                );
                                                // Un anneau de plus pour
                                                // ce qui est passé sous
                                                // le budget : la seule
                                                // distinction du
                                                // parcours, et elle se
                                                // mérite en réécrivant.
                                                if sous_budget {
                                                    ui.painter().circle_stroke(
                                                        pastille.center(),
                                                        6.0,
                                                        egui::Stroke::new(1.2, theme.accent),
                                                    );
                                                }
                                            } else {
                                                ui.painter().circle_stroke(
                                                    pastille.center(),
                                                    4.0,
                                                    egui::Stroke::new(1.0, theme.dim),
                                                );
                                            }
                                            if let Some(meilleur) = self.exercises_best.get(&id) {
                                                ui.colored_label(
                                                    theme.dim,
                                                    egui::RichText::new(format!("{meilleur}"))
                                                        .monospace(),
                                                )
                                                .on_hover_text(s.par_hint);
                                            }
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
        self.place_maximize_button(ctx, Self::EXERCISES_WINDOW, rendue_exercises_window);
        if let Some(id) = load {
            self.load_exercise(&id);
            open = false;
        }
        if let Some(name) = fiche {
            self.open_help(name);
            self.show_help = true;
        }
        self.show_exercises = open && !fermer_ok;
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
        // Déjà ouvert : on y retourne, avec ce qu'on y avait écrit.
        if let Some(rang) = self.exercise_tab_index(id) {
            self.go_to_tab(rang);
            return;
        }
        // Sinon un onglet à lui, sauf si le tampon courant est vierge :
        // on garde ainsi son propre programme à côté de l'exercice, et
        // l'on peut comparer deux étapes du parcours.
        if self.current_file.is_some() || self.buffer_modified() || self.current_exercise.is_some()
        {
            let courant = self.take_tab();
            let insertion = self.active_tab_index.min(self.tabs.len());
            self.tabs.insert(insertion, courant);
            self.active_tab_index = insertion + 1;
            // `find` a emprunté `self` : on le refait après le
            // déplacement, sur un état stable.
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
            return;
        }
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

    /// La fenêtre du déroulement : le programme, tel qu'il se parcourt.
    ///
    /// Un listing se lit de haut en bas ; un programme, non. Les sauts
    /// le plient — une boucle remonte, un test ouvre deux chemins, un
    /// appel part et revient. Ce dessin est la forme que l'on
    /// reconstitue de tête en lisant, et qu'un débutant ne voit pas
    /// encore.
    /// Cette fenêtre est-elle agrandie ?
    fn window_maximized(&self, id: &'static str) -> bool {
        self.maximized_windows.contains(id)
    }

    /// Règle une fenêtre pour qu'elle occupe tout l'écran.
    ///
    /// `fixed_size` donne la taille du *contenu* : le cadre de la fenêtre
    /// s'ajoute autour. Sans retrancher ses marges, la fenêtre dépassait
    /// de l'écran et se faisait rogner. Et `pivot` n'a plus lieu d'être —
    /// c'est un coin qu'on vise, non un centre.
    fn fill_the_screen<'a>(
        fenêtre: egui::Window<'a>,
        ctx: &egui::Context,
    ) -> egui::Window<'a> {
        let place = Self::maximized_rect(ctx);
        let marge = ctx.style_of(ctx.theme()).spacing.window_margin;
        let cadre = egui::vec2(
            f32::from(marge.left + marge.right),
            f32::from(marge.top + marge.bottom),
        );
        fenêtre
            .resizable(false)
            .pivot(egui::Align2::LEFT_TOP)
            .fixed_pos(place.min)
            .fixed_size(place.size() - cadre)
    }

    /// Le rectangle d'une fenêtre agrandie : tout ce qu'il y a.
    ///
    /// Sans marge : une fenêtre agrandie qui laisse voir douze pixels de
    /// ce qu'elle recouvre n'a pas l'air agrandie, elle a l'air mal
    /// posée.
    fn maximized_rect(ctx: &egui::Context) -> egui::Rect {
        ctx.viewport_rect()
    }

    /// Rend cette fenêtre agrandissable : elle prend tout l'écran quand
    /// on le lui a demandé, et garde ses réglages sinon.
    ///
    /// À poser en dernier sur la fenêtre, après ses tailles : `fixed_*`
    /// et `default_*` se disputent la même place, et le dernier écrit
    /// gagne.
    fn maximizable<'a>(
        &self,
        fenêtre: egui::Window<'a>,
        id: &'static str,
        ctx: &egui::Context,
    ) -> egui::Window<'a> {
        if self.window_maximized(id) {
            Self::fill_the_screen(fenêtre, ctx)
        } else {
            fenêtre
        }
    }

    /// Pose le bouton d'agrandissement dans le bandeau d'une fenêtre qui
    /// vient d'être dessinée, et applique ce qu'on en a fait.
    fn place_maximize_button<T>(
        &mut self,
        ctx: &egui::Context,
        id: &'static str,
        rendue: Option<egui::InnerResponse<Option<T>>>,
    ) {
        let agrandie = self.window_maximized(id);
        if let Some(rendue) = rendue
            && title_bar_maximize_button(
                ctx,
                id,
                rendue.response.rect,
                self.theme,
                strings(self.lang),
                agrandie,
            )
        {
            self.toggle_maximized(id);
        }
    }

    /// Déplace le curseur du mode Normal, et l'écrit dans l'état du
    /// champ — c'est lui qui le porte, même quand il n'a pas le focus.
    fn move_in_normal_mode(&mut self, ctx: &egui::Context, mouvement: Move) {
        let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) else {
            return;
        };
        let at = state
            .cursor
            .char_range()
            .map_or(0, |range| range.primary.index.0);
        let cible = move_cursor(&self.source, at, mouvement);
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(cible),
            )));
        state.store(ctx, editor_id());
        self.cursor_pos = line_col(&self.source, cible);
        self.cursor_byte = char_to_byte(&self.source, cible);
        // Le listing suit : se déplacer hors de l'écran sans que la vue
        // bouge reviendrait à ne pas se déplacer.
        self.follow_execution();
    }

    /// Applique ce qu'un clic sur le bouton d'agrandissement demande.
    fn toggle_maximized(&mut self, id: &'static str) {
        if !self.maximized_windows.remove(id) {
            self.maximized_windows.insert(id);
        }
    }

    /// Les fenêtres qui peuvent s'agrandir, nommées une fois pour que
    /// l'état et le bouton parlent de la même.
    const FLOW_WINDOW: &'static str = "flow";
    const DECOMPILER_WINDOW: &'static str = "decompiler";
    const DISASSEMBLY_WINDOW: &'static str = "disassembly";
    const TIMELINE_WINDOW: &'static str = "timeline";
    const ANALYSIS_WINDOW: &'static str = "analysis";
    const EXERCISES_WINDOW: &'static str = "exercises";
    const CALCULATOR_WINDOW: &'static str = "calculator";

    fn flow_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_flow {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_flow;
        let mut fermer_ok = false;
        let mut aller: Option<u32> = None;
        let agrandie = self.window_maximized(Self::FLOW_WINDOW);
        // Ce que le bouton a demandé : appliqué une fois la fenêtre
        // refermée, car tant qu'elle se dessine, ce qu'elle montre est
        // emprunté à `self`.
        let mut bascule = false;
        let mut fenêtre = egui::Window::new(s.flow_title)
            .id(egui::Id::new("flow_window"))
            .open(&mut open)
            .collapsible(false);
        // Agrandie ou libre : deux réglages qui ne se mélangent pas. Les
        // poser à la suite ne marchait pas — `min_size` et `default_size`
        // écrasaient ce que `fixed_rect` venait de fixer, et la fenêtre
        // gardait sa taille d'origine en changeant seulement de coin.
        fenêtre = if agrandie {
            Self::fill_the_screen(fenêtre, ctx)
        } else {
            fenêtre
                // Un graphe de blocs n'est pas une boîte de dialogue : il
                // se lit à la taille de l'écran. Sept cent soixante
                // pixels de large montraient trois boîtes sur mille.
                .resizable(true)
                .default_size({
                    let écran = ctx.viewport_rect().size();
                    egui::vec2(écran.x * 0.92, écran.y * 0.88)
                })
                .min_size([640.0, 420.0])
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.viewport_rect().center())
        };
        let rendue = fenêtre
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.flow_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "flow") {
                    fermer_ok = true;
                }
                let Some(state) = &self.assemble_state else {
                    ui.colored_label(theme.dim, s.flow_empty);
                    return;
                };
                let entier = namsdec_re::cfg::build(&state.disasm);
                if entier.blocks.is_empty() {
                    ui.colored_label(theme.dim, s.flow_empty);
                    return;
                }
                // Le bloc d'où l'on regarde : celui où le programme en
                // est, ou son point d'entrée tant que rien n'a tourné.
                let ancre = self
                    .emulator
                    .as_ref()
                    .and_then(|e| entier.block_at(e.cpu().regs.rip))
                    .unwrap_or(0);
                // La fonction seule, ou tout. Un vrai sous-graphe : le
                // dessin, la mise en page et la recherche de route
                // travaillent dessus sans rien savoir de la restriction.
                let restreint = self.flow_current_only && entier.blocks.len() > 1;
                let cfg = if restreint {
                    entier.subgraph(&entier.function_around(ancre))
                } else {
                    entier.clone()
                };
                if cfg.blocks.is_empty() {
                    ui.colored_label(theme.dim, s.flow_empty);
                    return;
                }
                ui.colored_label(theme.dim, s.flow_hint);
                flow_legend_ui(ui, theme, s);

                // « Comment arrive-t-on là ? » — la question qu'on pose
                // devant un programme qui refuse un mot de passe : on
                // connaît l'adresse du succès, on cherche la route.
                let mut route = None;
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme.dim, s.flow_goal_label);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.flow_goal)
                            .hint_text(s.flow_goal_hint)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(120.0),
                    );
                    if !self.flow_goal.is_empty() && ui.small_button("x").clicked() {
                        self.flow_goal.clear();
                    }
                });
                if let Some(visée) = sidebar::eval_address(&self.flow_goal, None) {
                    match namsdec_re::route::to(&cfg, visée) {
                        Some(trouvée) => {
                            flow_route_ui(ui, theme, s, &cfg, &trouvée, self.lang);
                            route = Some(trouvée);
                        }
                        None => {
                            ui.colored_label(theme.warn, s.flow_no_route);
                            // La route se cherche dans ce qu'on montre.
                            // Si elle existe ailleurs dans le programme,
                            // le dire vaut mieux que de laisser croire
                            // qu'il n'y en a aucune — l'adresse visée est
                            // souvent dans une autre fonction, et c'est
                            // même tout l'intérêt de la question.
                            if restreint && namsdec_re::route::to(&entier, visée).is_some() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.colored_label(theme.dim, s.flow_route_elsewhere);
                                    if framed_button(ui, theme, s.flow_show_whole).clicked() {
                                        self.flow_current_only = false;
                                    }
                                });
                            }
                        }
                    }
                }
                let sur_la_route: std::collections::BTreeSet<usize> = route
                    .as_ref()
                    .map(|r| {
                        r.stages
                            .iter()
                            .map(|é| é.block)
                            .chain(std::iter::once(r.target))
                            .collect()
                    })
                    .unwrap_or_default();
                ui.separator();

                // Le zoom à la molette, comme sur une carte : un
                // programme un peu long ne tient pas à l'échelle d'une
                // ligne lisible, et l'on veut d'abord voir sa forme.
                let survolé = ui.rect_contains_pointer(ui.max_rect());
                if survolé {
                    let molette = ui.input(|i| i.smooth_scroll_delta.y);
                    if molette.abs() > 0.5 && ui.input(|i| i.modifiers.ctrl) {
                        let avant = self.flow_zoom;
                        self.flow_zoom = (self.flow_zoom * (1.0 + molette * 0.002))
                            .clamp(FLOW_ZOOM_MIN, FLOW_ZOOM_MAX);
                        // Zoomer sous le pointeur, comme sur une carte :
                        // ce qu'on vise reste sous la souris. Zoomer
                        // depuis le coin faisait fuir le bloc qu'on
                        // regardait dès le deuxième cran.
                        if let Some(souris) = ui.input(|i| i.pointer.interact_pos()) {
                            let facteur = self.flow_zoom / avant;
                            let vue = ui.min_rect().min.to_vec2();
                            let sous_souris = self.flow_offset + (souris.to_vec2() - vue);
                            self.flow_offset =
                                sous_souris * facteur - (souris.to_vec2() - vue);
                            self.flow_offset.x = self.flow_offset.x.max(0.0);
                            self.flow_offset.y = self.flow_offset.y.max(0.0);
                        }
                    }
                }

                // ── 2.8 : « Tout voir » à la touche F ────────────────
                //
                // Le bouton existait ; la touche manquait. `f` comme
                // « fit », et seulement quand aucun champ ne retient les
                // frappes — la case d'adresse est juste au-dessus.
                if survolé
                    && ui.ctx().memory(|m| m.focused()).is_none()
                    && ui.input(|i| i.key_pressed(egui::Key::F))
                {
                    self.flow_fit = true;
                }

                // Et les mêmes commandes à portée de clic : la molette
                // avec Ctrl ne s'invente pas, et rien ne l'annonçait.
                ui.horizontal_wrapped(|ui| {
                    if framed_button(ui, theme, "−")
                        .on_hover_text(s.flow_zoom_out)
                        .clicked()
                    {
                        self.flow_zoom = (self.flow_zoom / 1.25).clamp(FLOW_ZOOM_MIN, FLOW_ZOOM_MAX);
                    }
                    ui.colored_label(
                        theme.dim,
                        format!("{:.0} %", self.flow_zoom * 100.0),
                    );
                    if framed_button(ui, theme, "+")
                        .on_hover_text(s.flow_zoom_in)
                        .clicked()
                    {
                        self.flow_zoom = (self.flow_zoom * 1.25).clamp(FLOW_ZOOM_MIN, FLOW_ZOOM_MAX);
                    }
                    if framed_button(ui, theme, s.flow_zoom_reset).clicked() {
                        self.flow_zoom = 1.0;
                        self.flow_offset = egui::Vec2::ZERO;
                    }
                    if framed_button(ui, theme, s.flow_fit).clicked() {
                        self.flow_fit = true;
                    }
                    // Le graphe s'emporte : dans une note, un rapport,
                    // un cours. DOT plutôt qu'une image — il se lit à
                    // l'œil, et `dot -Tsvg` en fait une image quand on en
                    // veut une.
                    if framed_button(ui, theme, s.flow_copy_dot)
                        .on_hover_text(s.flow_copy_dot_hint)
                        .clicked()
                    {
                        ui.ctx().copy_text(cfg.to_dot());
                        self.status = Status::Note(|s| s.flow_dot_copied);
                    }
                    ui.separator();
                    ui.checkbox(&mut self.flow_current_only, s.flow_current_only);
                    // Et laquelle. Le nom quand le binaire en porte un,
                    // l'adresse sinon : « vérifie_mdp » se retient, pas
                    // `0x1d491`. L'adresse reste montrée à côté — c'est
                    // elle qu'on tape dans « Comment arrive-t-on à », et
                    // c'est elle qui lève l'ambiguïté quand la fermeture
                    // par sauts se trompe, faute de symboles.
                    if restreint
                        && let Some(entrée) = cfg.blocks.first()
                    {
                        let nom = named_at(&self.notes, &state.output.symbols, entrée.start);
                        let texte = match &nom {
                            Some(nom) => format!("{nom}  {:#x}", entrée.start),
                            None => format!("{:#x}", entrée.start),
                        };
                        ui.colored_label(theme.dim, egui::RichText::new(texte).monospace())
                            .on_hover_text(s.flow_function_hint);
                    }
                    ui.separator();
                    // Combien on en montre, et combien il y en a : sur un
                    // binaire ordinaire, l'écart est tout le sujet.
                    ui.colored_label(
                        theme.dim,
                        if restreint {
                            format!(
                                "{} / {} {}",
                                cfg.blocks.len(),
                                entier.blocks.len(),
                                s.flow_blocks
                            )
                        } else {
                            format!("{} {}", cfg.blocks.len(), s.flow_blocks)
                        },
                    );
                    ui.separator();
                    ui.weak(s.flow_pan_hint);
                });
                ui.add_space(4.0);

                // « Tout voir » : le zoom qui fait entrer le graphe
                // entier dans la place qu'on a. Il se calcule ici, où
                // cette place est enfin connue — la demande, elle, a été
                // faite au clic.
                let vue = ui.available_size();
                if self.flow_fit {
                    self.flow_fit = false;
                    let naturel = flow_extent(ui, &cfg, s, 1.0);
                    let facteur = (vue.x / naturel.x.max(1.0))
                        .min(vue.y / naturel.y.max(1.0))
                        .clamp(FLOW_ZOOM_MIN, FLOW_ZOOM_MAX);
                    self.flow_zoom = facteur;
                    self.flow_offset = egui::Vec2::ZERO;
                }
                let zoom = self.flow_zoom;
                // Où le programme en est : le bloc qui contient RIP est
                // mis en avant, comme la ligne courante dans l'éditeur.
                let ici = self
                    .emulator
                    .as_ref()
                    .and_then(|e| cfg.block_at(e.cpu().regs.rip));
                let lignes = &state.addr_to_line;
                let sortie = egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .scroll_offset(self.flow_offset)
                    .show(ui, |ui| {
                        flow_graph_ui(ui, theme, s, &cfg, ici, lignes, zoom, &sur_la_route)
                    });
                let (destination, tiré) = sortie.inner;
                aller = destination;
                // Le glisser déplace la vue, comme sur une carte. On
                // repart de ce que la zone défilante dit d'elle-même :
                // les barres et la molette la déplacent aussi, et les
                // trois doivent parler du même point.
                self.flow_offset = sortie.state.offset - tiré;
            });
        self.show_flow = open && !fermer_ok;
        if let Some(rendue) = rendue
            && title_bar_maximize_button(
                ctx,
                Self::FLOW_WINDOW,
                rendue.response.rect,
                theme,
                s,
                agrandie,
            )
        {
            bascule = true;
        }
        if bascule {
            self.toggle_maximized(Self::FLOW_WINDOW);
        }
        if let Some(ligne) = aller {
            self.goto_line = Some(ligne);
        }
    }

    /// Lance la recette d'un outil, étape par étape, dans un fil à part.
    ///
    /// Chaque commande est écrite dans le journal **avant** d'être
    /// lancée, puis ce qu'elle répond y est ajouté : on lit donc ce qui
    /// se passe pendant que ça se passe, et non un compte rendu écrit
    /// après coup par le programme qui l'a fait.
    fn start_install(&mut self, recette: namsdec_disasm::install::Recipe) {
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        use std::sync::{Arc, Mutex};
        if self.install_job.is_some() {
            return;
        }
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let outcome = Arc::new(Mutex::new(None));
        let progress = Arc::new(AtomicUsize::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let total = recette.steps.len();

        let (journal, résultat, avancement, arrêt) = (
            Arc::clone(&log),
            Arc::clone(&outcome),
            Arc::clone(&progress),
            Arc::clone(&cancel),
        );
        let attendu = recette.produces.clone();
        let étapes = recette.steps.clone();
        std::thread::spawn(move || {
            use std::sync::atomic::Ordering;
            for (rang, étape) in étapes.iter().enumerate() {
                if arrêt.load(Ordering::Relaxed) {
                    *résultat.lock().expect("le journal") =
                        Some(Err("arrêté à la demande".to_owned()));
                    return;
                }
                journal
                    .lock()
                    .expect("le journal")
                    .push(format!("$ {}", étape.line()));
                let mut commande = std::process::Command::new(&étape.command);
                commande
                    .args(&étape.args)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());
                if let Some(dir) = &étape.cwd {
                    commande.current_dir(dir);
                }
                // On lance l'enfant plutôt que d'attendre sa sortie
                // complète : un téléchargement de 180 Mo ou une
                // compilation de quinze minutes doivent s'écrire au fur
                // et à mesure, et pouvoir s'arrêter avant la fin.
                let mut enfant = match commande.spawn() {
                    Ok(enfant) => enfant,
                    Err(e) => {
                        journal.lock().expect("le journal").push(e.to_string());
                        *résultat.lock().expect("le résultat") =
                            Some(Err(format!("« {} » : {e}", étape.line())));
                        return;
                    }
                };
                let mut lecteurs = Vec::new();
                if let Some(flux) = enfant.stdout.take() {
                    lecteurs.push(lire_flux(flux, Arc::clone(&journal)));
                }
                if let Some(flux) = enfant.stderr.take() {
                    lecteurs.push(lire_flux(flux, Arc::clone(&journal)));
                }
                let statut = loop {
                    if arrêt.load(Ordering::Relaxed) {
                        let _ = enfant.kill();
                        let _ = enfant.wait();
                        journal
                            .lock()
                            .expect("le journal")
                            .push("(arrêté)".to_owned());
                        *résultat.lock().expect("le résultat") =
                            Some(Err("arrêté à la demande".to_owned()));
                        return;
                    }
                    match enfant.try_wait() {
                        Ok(Some(statut)) => break statut,
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                        Err(e) => {
                            *résultat.lock().expect("le résultat") =
                                Some(Err(format!("« {} » : {e}", étape.line())));
                            return;
                        }
                    }
                };
                for lecteur in lecteurs {
                    let _ = lecteur.join();
                }
                if !statut.success() {
                    let code = statut
                        .code()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "?".to_owned());
                    // Le code de sortie dans le journal aussi : il ne
                    // figurait que dans la note d'échec, ailleurs dans la
                    // fenêtre, et l'on lisait les deux séparément.
                    journal
                        .lock()
                        .expect("le journal")
                        .push(format!("(code {code})"));
                    *résultat.lock().expect("le résultat") =
                        Some(Err(format!("« {} » a échoué (code {code})", étape.line())));
                    return;
                }
                avancement.store(rang + 1, Ordering::Relaxed);
            }
            // Le programme attendu est-il vraiment là ? Une suite de
            // commandes qui réussissent ne prouve pas qu'on a obtenu ce
            // qu'on cherchait.
            *résultat.lock().expect("le résultat") = Some(if attendu.is_file() {
                Ok(attendu)
            } else {
                Err(format!(
                    "les commandes ont réussi, mais {} n'existe pas",
                    attendu.display()
                ))
            });
        });

        self.install_job = Some(InstallJob {
            tool: recette.tool.to_owned(),
            total,
            done: 0,
            log,
            outcome,
            progress,
            cancel,
        });
        self.job = Some(BackgroundJob {
            label: format!("{} {}", strings(self.lang).job_installing, recette.tool),
            started: std::time::Instant::now(),
            // Une progression vraie : les étapes faites sur les étapes à
            // faire. Le temps, lui, ne veut rien dire ici — une
            // compilation ne se prévoit pas.
            limit: None,
        });
    }

    /// Où en est l'installation, pour la barre d'état : « 3 / 8 ».
    ///
    /// La fenêtre peut être fermée pendant qu'une compilation tourne :
    /// l'avancement doit se lire ailleurs qu'en la rouvrant.
    fn install_progress(&self) -> Option<(usize, usize)> {
        let job = self.install_job.as_ref()?;
        Some((job.done, job.total))
    }

    /// Relève où en est l'installation, et conclut quand elle a fini.
    fn poll_install(&mut self, ctx: &egui::Context) {
        use std::sync::atomic::Ordering;
        let Some(job) = &mut self.install_job else {
            return;
        };
        job.done = job.progress.load(Ordering::Relaxed);
        let fini = job.outcome.lock().expect("le résultat").take();
        // Le journal, avant tout : ce qui suit jette le travail qui le
        // tient, et c'est justement quand cela tourne mal qu'on veut le
        // lire.
        if fini.is_some() {
            self.install_log = job.log.lock().expect("le journal").clone();
        }
        match fini {
            Some(Ok(programme)) => {
                let outil = job.tool.clone();
                self.install_job = None;
                self.job = None;
                self.declare_installed_tool(&outil, &programme);
            }
            Some(Err(raison)) => {
                let outil = job.tool.clone();
                self.install_job = None;
                self.job = None;
                // Ce qui reste sur le disque : une recette s'arrête où
                // elle peut, et le dossier à demi rempli n'a l'air de
                // rien. La prochaine tentative l'effacera d'elle-même,
                // ce qui vaut la peine d'être dit.
                let reste = data_dir()
                    .map(|dir| namsdec_disasm::install::tools_dir(&dir).join(&outil))
                    .filter(|chemin| chemin.exists())
                    .map(|chemin| format!(" {} {}", strings(self.lang).install_leftover, chemin.display()))
                    .unwrap_or_default();
                self.plugin_note = Some(PluginNote {
                    ok: false,
                    text: format!("{} {raison}{reste}", strings(self.lang).install_failed),
                });
            }
            None => ctx.request_repaint_after(std::time::Duration::from_millis(250)),
        }
    }

    /// Déclare l'outil qu'on vient d'installer, avec le chemin exact du
    /// programme obtenu.
    ///
    /// Le chemin absolu plutôt que le nom : ce qu'on vient de poser n'est
    /// pas dans le PATH, et il n'a pas à y être — namsdec sait où il l'a
    /// mis.
    fn declare_installed_tool(&mut self, outil: &str, programme: &Path) {
        let s = strings(self.lang);
        let Some(mut plugin) = namsdec_disasm::known::all()
            .into_iter()
            .find(|connu| connu.plugin.id == outil)
            .map(|connu| connu.plugin)
        else {
            return;
        };
        plugin.command = programme.display().to_string();
        self.declare_tool(&plugin);
        self.plugin_note = Some(PluginNote {
            ok: true,
            text: format!("{} {}", s.install_done, plugin.name),
        });
    }

    /// La fenêtre de décompilation : un outil extérieur, sur demande.
    ///
    /// Trois précautions, et elles sont le sujet de ce module autant que
    /// le pseudo-code qu'il affiche :
    ///
    /// - **Rien ne s'exécute tout seul.** Le module s'active dans les
    ///   préférences, la fenêtre s'ouvre à la demande, et l'outil ne part
    ///   qu'au clic sur « Lancer ».
    /// - **La commande est annoncée avant d'être lancée**, en toutes
    ///   lettres : c'est ce que l'on s'apprête à exécuter sur sa propre
    ///   machine, et l'on doit pouvoir le lire.
    /// - **Le travail se fait à côté.** Un décompilateur peut réfléchir
    ///   une demi-minute ; l'interface ne se fige pas, et passé le délai
    ///   l'outil est arrêté plutôt qu'attendu sans fin.
    fn decompiler_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_decompiler_module || !self.show_decompiler {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_decompiler;
        let mut fermer_ok = false;
        let mut lancer = false;
        let agrandie = self.window_maximized(Self::DECOMPILER_WINDOW);
        let mut bascule = false;
        let mut fenêtre = egui::Window::new(s.decompiler_title)
            .id(egui::Id::new("decompiler_window"))
            .open(&mut open)
            .collapsible(false);
        fenêtre = if agrandie {
            Self::fill_the_screen(fenêtre, ctx)
        } else {
            fenêtre
                .resizable(true)
                .default_size([720.0, 520.0])
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ctx.viewport_rect().center())
        };
        let rendue = fenêtre
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.decompiler_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "decompiler") {
                    fermer_ok = true;
                }
                if self.decompilers.is_empty() {
                    ui.colored_label(theme.warn, s.decompiler_none_installed);
                    ui.add_space(4.0);
                    ui.colored_label(theme.dim, s.decompiler_install_hint);
                    if let Some(dir) = decompilers_dir() {
                        ui.colored_label(
                            theme.dim,
                            egui::RichText::new(dir.display().to_string()).monospace(),
                        );
                    }
                    ui.add_space(6.0);
                    ui.colored_label(theme.dim, s.decompiler_example);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(DECOMPILER_EXAMPLE).monospace().small(),
                        )
                        .selectable(true),
                    );
                    return;
                }

                // Le choix de l'outil, retenu d'une session à l'autre.
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme.dim, s.decompiler_tool);
                    let courant = self
                        .decompilers
                        .iter()
                        .find(|outil| outil.id == self.decompiler)
                        .map(|outil| outil.name.clone())
                        .unwrap_or_else(|| s.decompiler_choose.to_owned());
                    egui::ComboBox::from_id_salt("decompiler_choice")
                        .selected_text(courant)
                        .show_ui(ui, |ui| {
                            for outil in &self.decompilers {
                                ui.selectable_value(
                                    &mut self.decompiler,
                                    outil.id.clone(),
                                    &outil.name,
                                );
                            }
                        });
                });

                let choisi = self
                    .decompilers
                    .iter()
                    .find(|outil| outil.id == self.decompiler)
                    .cloned();
                let Some(outil) = choisi else {
                    ui.colored_label(theme.dim, s.decompiler_choose_first);
                    return;
                };

                // Ce qu'on s'apprête à lancer, en toutes lettres.
                ui.add_space(4.0);
                ui.colored_label(theme.dim, s.decompiler_will_run);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("{} {}", outil.command, outil.args.join(" ")))
                            .monospace(),
                    )
                    .selectable(true),
                );
                ui.weak(s.decompiler_file_hint);
                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    let occupé = self.decompile_job.is_some();
                    let prêt = self.assemble_state.is_some();
                    ui.add_enabled_ui(!occupé && prêt, |ui| {
                        lancer = ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(s.decompiler_run).color(theme.bg),
                                )
                                .fill(theme.accent),
                            )
                            .clicked();
                    });
                    if occupé {
                        ui.spinner();
                        ui.colored_label(theme.dim, s.decompiler_working);
                    } else if !prêt {
                        ui.colored_label(theme.dim, s.decompiler_no_binary);
                    }
                    if let Some(Ok(texte)) = &self.decompile_result
                        && ui.small_button(s.decompiler_copy).clicked()
                    {
                        ui.ctx().copy_text(texte.clone());
                    }
                });

                // Ce qui a été décompilé, et de quel programme il
                // s'agissait : sans cela, un texte resté à l'écran passe
                // pour celui du programme qu'on regarde maintenant.
                if let Some(origine) = &self.decompiled_from {
                    ui.weak(format!("{} {origine}", s.decompiler_of));
                }
                ui.separator();
                match &self.decompile_result {
                    Some(Ok(_)) => {
                        // Une barre de lecture : chercher un mot, sauter
                        // à une fonction, savoir combien de lignes on a
                        // sous les yeux.
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(theme.dim, s.decompiler_find);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.decompile_filter)
                                    .desired_width(200.0)
                                    .hint_text(s.decompiler_find_hint),
                            );
                            if !self.decompile_functions.is_empty() {
                                egui::ComboBox::from_id_salt("decompile_functions")
                                    .selected_text(format!(
                                        "{} ({})",
                                        s.decompiler_functions,
                                        self.decompile_functions.len()
                                    ))
                                    .show_ui(ui, |ui| {
                                        for (rang, titre) in self.decompile_functions.clone() {
                                            if ui.selectable_label(false, &titre).clicked() {
                                                self.decompile_filter.clear();
                                                self.decompile_goto = Some(rang);
                                            }
                                        }
                                    });
                            }
                            ui.colored_label(
                                theme.dim,
                                format!(
                                    "{} / {} {}",
                                    self.decompile_visible.len(),
                                    self.decompile_lines.len(),
                                    s.decompiler_lines
                                ),
                            );
                        });
                        ui.add_space(4.0);

                        // Le filtre ne se recalcule qu'au changement.
                        if self.decompile_filter != self.decompile_filter_applied {
                            let cherché = self.decompile_filter.to_lowercase();
                            self.decompile_visible = if cherché.is_empty() {
                                (0..self.decompile_lines.len()).collect()
                            } else {
                                self.decompile_lines
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, ligne)| ligne.to_lowercase().contains(&cherché))
                                    .map(|(rang, _)| rang)
                                    .collect()
                            };
                            self.decompile_filter_applied = self.decompile_filter.clone();
                            // Pas de « decompile_goto = None » ici :
                            // sauter à une fonction efface justement le
                            // filtre, et le saut passerait à la trappe.
                        }

                        // Une ligne = une galette de texte, numéro
                        // compris : deux widgets par ligne coûteraient
                        // une espace verticale de plus, et le saut
                        // tomberait à côté.
                        let police = egui::TextStyle::Monospace.resolve(ui.style());
                        // `show_rows` veut la hauteur d'une ligne **sans**
                        // l'espace qui la sépare de la suivante : il
                        // l'ajoute lui-même. Le saut, lui, se calcule sur
                        // le pas complet. Confondre les deux décale
                        // l'arrivée, d'autant plus qu'on descend loin.
                        let ligne_seule = ui.ctx().fonts_mut(|f| f.row_height(&police));
                        let hauteur = ligne_seule + ui.spacing().item_spacing.y;
                        let mut zone = egui::ScrollArea::both()
                            .id_salt("decompile_text")
                            .auto_shrink([false, false]);
                        // Sauter à une fonction : la zone n'a pas de
                        // « aller à la ligne n », mais toutes les lignes
                        // font la même hauteur, alors le calcul suffit.
                        if let Some(rang) = self.decompile_goto.take() {
                            let place = self
                                .decompile_visible
                                .iter()
                                .position(|&l| l >= rang)
                                .unwrap_or(0);
                            zone = zone.vertical_scroll_offset(place as f32 * hauteur);
                        }
                        let largeur =
                            (self.decompile_lines.len().max(1) as f32).log10() as usize + 1;
                        zone.show_rows(
                            ui,
                            ligne_seule,
                            self.decompile_visible.len(),
                            |ui, plage| {
                                for place in plage {
                                    let rang = self.decompile_visible[place];
                                    let mut texte = egui::text::LayoutJob::default();
                                    texte.append(
                                        &format!("{:>largeur$} ", rang + 1),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: police.clone(),
                                            color: theme.dim,
                                            ..Default::default()
                                        },
                                    );
                                    texte.append(
                                        &self.decompile_lines[rang],
                                        0.0,
                                        egui::TextFormat {
                                            font_id: police.clone(),
                                            color: theme.text,
                                            ..Default::default()
                                        },
                                    );
                                    ui.add(
                                        egui::Label::new(texte)
                                            .wrap_mode(egui::TextWrapMode::Extend)
                                            .selectable(true),
                                    );
                                }
                            },
                        );
                    }
                    Some(Err(erreur)) => {
                        ui.colored_label(theme.bad, erreur);
                    }
                    None => {
                        ui.colored_label(theme.dim, s.decompiler_idle);
                    }
                }
            });
        self.show_decompiler = open && !fermer_ok;
        if let Some(rendue) = rendue
            && title_bar_maximize_button(
                ctx,
                Self::DECOMPILER_WINDOW,
                rendue.response.rect,
                theme,
                s,
                agrandie,
            )
        {
            bascule = true;
        }
        if bascule {
            self.toggle_maximized(Self::DECOMPILER_WINDOW);
        }
        if lancer {
            self.start_decompile();
        }
    }

    /// Porte l'avancement jusque dans la barre des tâches du système.
    ///
    /// Un gestionnaire de fenêtres n'a pas de notion de progression —
    /// aucune qui soit portable, en tout cas : ce que GNOME, KDE et XFCE
    /// affichent tous, c'est le **titre de la fenêtre**. On y écrit donc
    /// le pourcentage, et il apparaît là où l'on regarde quand on a
    /// changé d'application : dans la barre des tâches, à côté du nom.
    ///
    /// Le titre est remis en place dès que le travail s'achève : une
    /// application qui reste marquée « 100 % » indéfiniment fait douter
    /// de tout ce qu'elle affiche.
    fn reflect_job_in_title(&mut self, ctx: &egui::Context) {
        let voulu = match &self.job {
            Some(job) => match job.fraction() {
                Some(part) => format!("{APP_NAME} — {} {:.0} %", job.label, part * 100.0),
                None => format!("{APP_NAME} — {}", job.label),
            },
            None => APP_NAME.to_owned(),
        };
        if self.window_title.as_deref() != Some(voulu.as_str()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(voulu.clone()));
            self.window_title = Some(voulu);
        }
    }

    /// Lance l'outil choisi sur le programme lié, dans un fil à part.
    fn start_decompile(&mut self) {
        if self.decompile_job.is_some() {
            return;
        }
        let (Some(state), Some(outil)) = (
            &self.assemble_state,
            self.decompilers
                .iter()
                .find(|outil| outil.id == self.decompiler)
                .cloned(),
        ) else {
            return;
        };
        let octets = state.output.file.clone();
        self.decompiled_from = Some(
            self.current_file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|nom| nom.to_string_lossy().into_owned())
                .unwrap_or_else(|| strings(self.lang).untitled_buffer.to_owned()),
        );
        self.decompile_result = None;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let réponse = outil
                .run_within(&octets, DECOMPILER_TIMEOUT)
                .map_err(|e| e.to_string());
            // Personne n'attend plus si la fenêtre est fermée : l'envoi
            // échoue, et le fil s'achève là.
            let _ = sender.send(réponse);
        });
        self.decompile_job = Some(receiver);
        self.job = Some(BackgroundJob {
            label: strings(self.lang).job_decompiling.to_owned(),
            started: std::time::Instant::now(),
            limit: Some(DECOMPILER_TIMEOUT),
        });
    }

    /// Découpe le texte rendu et repère les fonctions qu'il contient.
    ///
    /// La règle est volontairement simple — une ligne sans indentation
    /// qui ouvre une accolade après une parenthèse — parce qu'elle vaut
    /// pour le C de RetDec comme pour le pseudo-code de Rizin. Une
    /// heuristique qui se lit en deux lignes vaut mieux qu'un analyseur
    /// qui se trompe sans le dire.
    fn index_decompilation(&mut self, texte: &str) {
        self.decompile_lines = texte.lines().map(str::to_owned).collect();
        self.decompile_functions = self
            .decompile_lines
            .iter()
            .enumerate()
            .filter(|(_, ligne)| {
                let nu = !ligne.starts_with(' ') && !ligne.starts_with('\t');
                nu && ligne.contains('(') && ligne.trim_end().ends_with('{')
            })
            .map(|(rang, ligne)| (rang, ligne.trim_end_matches(['{', ' ']).to_owned()))
            .collect();
        self.decompile_filter.clear();
        self.decompile_filter_applied.clear();
        self.decompile_visible = (0..self.decompile_lines.len()).collect();
        self.decompile_goto = None;
    }

    /// Relève ce que l'outil a rendu, s'il a fini.
    fn poll_decompile(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.decompile_job else {
            return;
        };
        match receiver.try_recv() {
            Ok(réponse) => {
                self.decompile_job = None;
                self.job = None;
                if let Ok(texte) = &réponse {
                    self.index_decompilation(texte);
                }
                self.decompile_result = Some(réponse);
            }
            Err(TryRecvError::Empty) => {
                // Le fil travaille : on redemande une image, sinon
                // l'interface s'endort et la réponse attend.
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
            }
            Err(TryRecvError::Disconnected) => {
                self.decompile_job = None;
                self.decompile_result = Some(Err(strings(self.lang).decompiler_lost.to_owned()));
            }
        }
    }

    fn about_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_about;
        let mut close = false;
        // Relevé dans la fenêtre, agi après : elle emprunte `self`.
        let mut lancer_recherche = false;
        egui::Window::new(s.about_title)
            // Un identifiant explicite : sans lui, egui dérive celui de
            // la fenêtre de son *titre*, qui change avec la langue —
            // passer en anglais lui faisait perdre sa position.
            .id(egui::Id::new(ABOUT_WINDOW))
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
            // Large : trois boutons, un bandeau de préversion et une
            // recherche de mise à jour ne tiennent pas dans une colonne
            // de trois cents pixels sans se replier en accordéon.
            .default_width(ABOUT_WIDTH)
            .min_width(ABOUT_WIDTH)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.about_title) {
                    close = true;
                }
                ui.set_min_width(ABOUT_WIDTH - 24.0);
                // L'en-tête : l'icône de l'application à gauche — la
                // même que celle du lanceur —, et à droite ce qu'elle
                // est, ce qu'elle vaut, ce qu'elle fait.
                let icône = self.app_icon_texture(ctx);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::from_handle(&icône))
                            .fit_to_exact_size(egui::vec2(APP_ICON_SHOWN, APP_ICON_SHOWN))
                            .corner_radius(12),
                    );
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new("namsdec").strong().size(28.0));
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(format!("v{APP_VERSION}")).monospace(),
                            )
                            .on_hover_text(format!("{} {BUILD_DATE}", s.about_build_word));
                        });
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                        ui.colored_label(theme.dim, s.about_description);
                        ui.add_space(8.0);
                        // Les trois portes du projet : le site, le
                        // salon, le dépôt. Chacune avec sa marque —
                        // dessinée, donc de la couleur du thème et nette
                        // à toute taille.
                        ui.horizontal_wrapped(|ui| {
                            for (label, url, infobulle, dessin) in [
                                (
                                    s.about_site,
                                    SITE_URL,
                                    s.about_site_hint,
                                    chrome::icon_globe
                                        as fn(&egui::Painter, egui::Rect, egui::Color32),
                                ),
                                (
                                    s.about_discord,
                                    DISCORD_URL,
                                    s.about_discord_hint,
                                    chrome::icon_discord,
                                ),
                                (
                                    s.about_github,
                                    GITHUB_URL,
                                    s.about_github_hint,
                                    chrome::icon_github,
                                ),
                            ] {
                                if link_button_ui(ui, theme, label, dessin)
                                    .on_hover_text(format!("{infobulle}\n{url}"))
                                    .clicked()
                                {
                                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                }
                            }
                        });
                    });
                });
                ui.add_space(10.0);

                // Le bandeau des préversions : il dit ce qu'on tient
                // entre les mains, et disparaît de lui-même le jour où
                // la version n'en est plus une.
                if let Some(étiquette) = prerelease(APP_VERSION) {
                    egui::Frame::new()
                        .fill(theme.warn.gamma_multiply(0.15))
                        .stroke(egui::Stroke::new(1.0, theme.warn))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .corner_radius(8.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ABOUT_WIDTH - 60.0);
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.colored_label(
                                theme.warn,
                                egui::RichText::new(format!(
                                    "{} — {étiquette}",
                                    s.about_beta_title
                                ))
                                .strong(),
                            );
                            ui.add_space(4.0);
                            ui.colored_label(theme.text, s.about_beta_body);
                        });
                    ui.add_space(10.0);
                }

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme.dim, s.about_license);
                    ui.hyperlink_to(s.about_license_link, MIT_LICENSE_URL);
                });

                ui.separator();
                ui.colored_label(theme.dim, s.about_donate_hint);
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
                // La recherche est un geste, jamais un réflexe : rien
                // n'est demandé à GitHub tant que personne n'a cliqué,
                // et rien n'est envoyé d'autre que la requête.
                ui.horizontal_wrapped(|ui| {
                    ui.add_enabled_ui(self.update_check.is_none(), |ui| {
                        if ui
                            .button(s.about_check_update)
                            .on_hover_text(s.about_check_update_hint)
                            .clicked()
                        {
                            lancer_recherche = true;
                        }
                    });
                    if self.update_check.is_some() {
                        ui.colored_label(theme.dim, s.about_update_checking);
                    }
                });
                match &self.update_result {
                    Some(Ok(namsdec_update::Verdict::UpToDate)) => {
                        ui.colored_label(theme.good, s.about_update_uptodate);
                    }
                    Some(Ok(namsdec_update::Verdict::Newer(release))) => {
                        ui.colored_label(
                            theme.warn,
                            format!("{} {}", s.about_update_newer, release.tag),
                        );
                        if !release.url.is_empty() {
                            ui.hyperlink_to(s.about_update_open, &release.url);
                        }
                        // Ce que namsdec ne fera pas : se remplacer tout
                        // seul. Dire comment mettre à jour vaut mieux que
                        // de le faire dans le dos de celui qui travaille.
                        ui.colored_label(theme.dim, s.about_update_how);
                    }
                    Some(Ok(namsdec_update::Verdict::Ahead(release))) => {
                        ui.colored_label(
                            theme.dim,
                            format!("{} ({})", s.about_update_ahead, release.tag),
                        );
                    }
                    Some(Err(message)) => {
                        ui.colored_label(theme.bad, message);
                    }
                    None => {}
                }

                ui.separator();
                // Un seul bouton : il n'y a rien à annuler dans une boîte
                // qui ne fait que renseigner. Échap la ferme quand même.
                // `|=` et non `=` : la pastille du bandeau a peut-être
                // déjà répondu plus haut, et une affectation sèche
                // l'effacerait.
                close |= dialog_bar(ui, theme, s.dialog_ok, None).is_some();
            });
        if lancer_recherche {
            self.start_update_check();
        }
        self.show_about = open && !close;
        if !self.show_about {
            // La réponse ne survit pas à la fermeture : on rouvre, on
            // redemande — l'état du dépôt a pu changer entre-temps.
            self.update_result = None;
        }
    }

    /// Demande à GitHub la dernière version publiée, sur un thread.
    ///
    /// Une requête réseau dans la boucle de rendu fige la fenêtre le
    /// temps du voyage ; celle-ci arrive par un canal, relevé à chaque
    /// frame comme la réponse du sélecteur de fichiers.
    fn start_update_check(&mut self) {
        if self.update_check.is_some() {
            return;
        }
        self.update_result = None;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            // Une préversion accepte d'en voir d'autres ; une version
            // définitive ne se voit proposer que du définitif.
            let réponse = namsdec_update::fetch_latest(
                namsdec_update::OWNER,
                namsdec_update::REPO,
                prerelease(APP_VERSION).is_some(),
            )
            .map_err(|e| e.to_string());
            // Si la fenêtre s'est fermée entre-temps, plus personne
            // n'attend : l'envoi échoue et le thread s'achève là.
            let _ = sender.send(réponse);
        });
        self.update_check = Some(receiver);
        // Une recherche de mise à jour ne dit pas non plus où elle en
        // est, et rien ne la borne : on montre qu'elle tourne, et depuis
        // combien de temps. Une barre qui avancerait au hasard mentirait.
        self.job = Some(BackgroundJob {
            label: strings(self.lang).job_checking_update.to_owned(),
            started: std::time::Instant::now(),
            limit: None,
        });
    }

    /// Relève la réponse de la recherche, si elle est arrivée.
    fn poll_update_check(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.update_check else {
            return;
        };
        match receiver.try_recv() {
            Ok(réponse) => {
                self.update_check = None;
                self.job = None;
                let s = strings(self.lang);
                self.update_result = Some(match réponse {
                    Ok(release) => Ok(namsdec_update::verdict(APP_VERSION, release)),
                    // Le cas le plus probable au début de la vie d'un
                    // dépôt : rien n'a encore été publié. Ce n'est pas
                    // une panne, et le dire ainsi évite d'inquiéter.
                    Err(message) if message.contains("no release") => {
                        Err(s.about_update_none.to_owned())
                    }
                    // Un dépôt privé répond comme un dépôt inexistant :
                    // le message dit les deux, parce qu'on ne peut pas
                    // savoir lequel c'est de l'extérieur.
                    Err(message) if message.contains("not found") => {
                        Err(s.about_update_private.to_owned())
                    }
                    Err(message) => Err(format!("{} : {message}", s.about_update_failed)),
                });
            }
            // Toujours en route : sans ce réveil, la réponse attendrait
            // le prochain mouvement de souris pour être lue.
            Err(TryRecvError::Empty) => ctx.request_repaint_after(DIALOG_POLL_INTERVAL),
            Err(TryRecvError::Disconnected) => {
                self.update_check = None;
                self.job = None;
                self.update_result = Some(Err(strings(self.lang).about_update_failed.to_owned()));
            }
        }
    }

    fn toolbar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            ui.add_enabled_ui(self.can_assemble(), |ui| {
                if ui
                    .button(s.action_assemble)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.action_assemble,
                        Action::Assemble,
                    ))
                    .on_disabled_hover_text(s.binary_no_reassemble_hint)
                    .clicked()
                {
                    self.dispatch(Action::Assemble);
                }
            });

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

            // Réinitialiser ne se grise pas comme le reste : c'est le
            // programme terminé, tous les autres boutons éteints, qu'on
            // le cherche — précisément l'état où `can_execute` est faux.
            // Sa seule condition est qu'il y ait une machine à remettre
            // à zéro.
            ui.add_enabled_ui(self.emulator.is_some(), |ui| {
                if icon_button(ui, icon_reset, s.action_reset)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        &ctx,
                        s.action_reset,
                        Action::Reset,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::Reset);
                }
            });

            // À droite, ce que le panneau latéral montre : Pile,
            // Mémoire, Tas, Octets, et RE quand il est demandé. Ces
            // onglets étaient en bas du panneau, entre l'état du
            // processeur et leur propre contenu — ils montaient et
            // descendaient selon la taille du bandeau. Ici, ils sont
            // toujours au même endroit, et le panneau gagne leur
            // hauteur.
            if self.show_sidebar {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // De droite à gauche : la boucle est inversée pour
                    // que l'ordre lu reste Pile, Mémoire, Tas, Octets.
                    for tab in SidebarTab::ALL.iter().rev() {
                        if *tab == SidebarTab::Re && !self.show_re_tab {
                            continue;
                        }
                        if ui
                            .selectable_label(self.active_tab == *tab, tab.label(s))
                            .clicked()
                        {
                            self.active_tab = *tab;
                        }
                    }
                    // La calculatrice, juste à gauche de « Pile ». La
                    // disposition est de droite à gauche, donc elle vient
                    // après la boucle. Sa place est là parce que c'est là
                    // qu'on s'en sert : on lit une valeur dans la pile ou
                    // dans un registre, et l'on veut savoir ce qu'elle
                    // vaut dans une autre base. Elle était dans le menu
                    // Outils, à trois gestes de l'endroit où le besoin
                    // naît.
                    ui.add_space(4.0);
                    if icon_square_button(ui, chrome::icon_calculator)
                        .on_hover_text(s.calculator_title)
                        .clicked()
                    {
                        self.show_calculator = true;
                    }
                });
            }
        });
    }

    /// Toutes les fenêtres internes, et si chacune est ouverte.
    ///
    /// Une seule table, et un test vérifie qu'elle les cite toutes :
    /// ajouter une fenêtre sans y penser reviendrait à réintroduire le
    /// défaut sans le voir. Les noms sont les identifiants qu'on passe à
    /// `egui::Window::id`, lesquels ne changent pas avec la langue.
    fn window_states(&self) -> [(&'static str, bool); 17] {
        [
            ("preferences_window", self.show_preferences),
            (ABOUT_WINDOW, self.show_about),
            ("instruction_help", self.show_help),
            ("vectors_window", self.show_vectors),
            ("breakpoints_window", self.show_breakpoints),
            ("timeline_window", self.show_timeline),
            ("flow_window", self.show_flow),
            (
                "decompiler_window",
                self.show_decompiler && self.show_decompiler_module,
            ),
            (
                "installer_window",
                self.install_plan.is_some() && self.show_installer_module,
            ),
            ("disassembly_window", self.show_disassembly),
            ("cyclic_window", self.show_cyclic),
            ("analysis_window", self.show_analysis),
            ("calculator_window", self.show_calculator),
            ("exercises_window_v2", self.exercises && self.show_exercises),
            ("unsaved_changes", self.pending.is_some()),
            ("command_palette", self.command_palette.is_some()),
            (INPUT_DIALOG, self.show_input_dialog),
        ]
    }

    /// Pose devant toute fenêtre qui vient de s'ouvrir.
    ///
    /// Le plan d'installation s'ouvre depuis les préférences, et se
    /// posait derrière elles : on cliquait « Installer… » et il ne se
    /// passait rien de visible. Le cas n'avait rien de particulier à
    /// cette fenêtre-là — rien n'oblige une fenêtre ouverte depuis une
    /// autre à passer devant elle —, d'où une règle valable pour toutes
    /// plutôt qu'un rang choisi au cas par cas.
    ///
    /// « Vient de s'ouvrir » et non « est ouverte » : la remettre devant
    /// à chaque frame empêcherait d'en poser une autre par-dessus.
    fn raise_new_windows(&mut self, ctx: &egui::Context) {
        let ouvertes: BTreeSet<&'static str> = self
            .window_states()
            .into_iter()
            .filter(|(_, ouverte)| *ouverte)
            .map(|(nom, _)| nom)
            .collect();
        for nom in ouvertes.difference(&self.windows_open) {
            ctx.move_to_top(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::new(*nom),
            ));
        }
        self.windows_open = ouvertes;
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

        // Ce qui tourne à côté, tout à gauche : c'est la première chose
        // qu'on veut voir quand on se demande si l'outil est parti.
        if let Some(job) = &self.job {
            ui.horizontal(|ui| {
                // Une installation compte ses étapes : c'est une
                // progression vraie, là où le temps ne dirait rien.
                let étapes = self.install_progress();
                let barre = match étapes
                    .map(|(f, t)| f as f32 / t.max(1) as f32)
                    .or_else(|| job.fraction())
                {
                    // Une progression vraie : le temps écoulé sur le
                    // temps accordé. C'est ce qui décide de l'abandon,
                    // donc c'est ce qui intéresse.
                    Some(part) => egui::ProgressBar::new(part).desired_width(150.0).text(
                        egui::RichText::new(format!("{} — {}", job.label, job.timing())).small(),
                    ),
                    // Rien ne borne : une barre qui avance toute seule
                    // dit « ça travaille » sans prétendre savoir combien
                    // il reste.
                    None => egui::ProgressBar::new(0.0)
                        .animate(true)
                        .desired_width(150.0)
                        .text(
                            egui::RichText::new(format!("{} — {}", job.label, job.timing()))
                                .small(),
                        ),
                };
                ui.add(barre);
                ui.separator();
            });
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

            // La correction proposée pour la ligne du curseur, et la
            // touche qui la pose. À gauche du message d'état : elle
            // parle de ce qu'on est en train d'écrire, non de ce que
            // namsdec vient de faire, et c'est l'ordre dans lequel on
            // lit la barre.
            if let Some(correction) = self.current_fix() {
                ui.scope(|ui| {
                    // Bornée et tronquée, comme le message : une phrase
                    // longue ne doit pas pousser les compartiments de
                    // droite hors de la fenêtre.
                    ui.set_max_width(420.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    ui.colored_label(
                        theme.changed,
                        s.fix_hint.replacen("{}", &correction.titre, 1),
                    )
                    // La ligne telle qu'elle sera écrite : on confirme
                    // ce qu'on a vu, non ce qu'on a deviné.
                    .on_hover_text(&correction.ligne);
                });
                ui.separator();
            }
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

    /// Le message d'état : sa phrase, sa couleur, et la pastille qui la
    /// précède quand il y en a une.
    ///
    /// **Un seul endroit décide de la phrase.** La barre la peint, et
    /// l'infobulle la répète en entier quand la place a manqué. Deux
    /// fonctions qui la composeraient chacune finiraient par diverger,
    /// et ce serait l'infobulle qui aurait tort — celle qu'on ne
    /// consulte justement que pour vérifier.
    fn status_line(&self) -> (String, egui::Color32, Option<egui::Color32>) {
        let s = strings(self.lang);
        let fr = self.lang == Lang::Fr;
        let theme = self.theme;
        match &self.status {
            Status::Idle => (s.status_ready.to_owned(), theme.text, None),
            Status::Running => (s.status_running.to_owned(), theme.accent, None),
            // Une pastille peinte, pas un glyphe : « ✓ » et « ✗ » ne
            // sont pas dans la police par défaut d'egui et retombaient
            // sur le carré de remplacement.
            Status::AssembleOk => (
                s.status_assembled.to_owned(),
                theme.good,
                Some(theme.good),
            ),
            Status::AssembleFailed => (
                s.status_assemble_failed.to_owned(),
                theme.bad,
                Some(theme.bad),
            ),
            // La valeur d'avant et celle d'après : c'est toute
            // l'information qu'on attendait en posant le point.
            Status::Watchpoint { addr, old, new } => (
                if fr {
                    format!("Surveillance : {addr:#x} est passé de {old:#x} à {new:#x}")
                } else {
                    format!("Watchpoint: {addr:#x} went from {old:#x} to {new:#x}")
                },
                theme.changed,
                Some(theme.changed),
            ),
            Status::RegisterChanged { register, old, new } => (
                if fr {
                    format!("Surveillance : {register} est passé de {old:#x} à {new:#x}")
                } else {
                    format!("Watch: {register} went from {old:#x} to {new:#x}")
                },
                theme.changed,
                Some(theme.changed),
            ),
            Status::SteppedBack => (s.status_stepped_back.to_owned(), theme.accent, None),
            Status::NothingToUndo => (s.status_nothing_to_undo.to_owned(), theme.dim, None),
            Status::Halted => (s.status_halted.to_owned(), theme.good, None),
            Status::WaitingForInput => {
                (s.status_waiting_for_input.to_owned(), theme.accent, None)
            }
            Status::Exited { code } => (
                if fr {
                    format!("Programme terminé (code {code}).")
                } else {
                    format!("Program exited (code {code}).")
                },
                if *code == 0 { theme.good } else { theme.bad },
                None,
            ),
            Status::BinaryLoaded { bytes, truncated } => {
                let taille = match (*truncated, fr) {
                    (true, true) => format!("{bytes} premiers octets"),
                    (true, false) => format!("first {bytes} bytes"),
                    (false, true) => format!("{bytes} octets"),
                    (false, false) => format!("{bytes} bytes"),
                };
                (
                    format!("{} — {taille}", s.binary_not_text),
                    theme.accent,
                    None,
                )
            }
            Status::Breakpoint(addr) => (
                if fr {
                    format!("Arrêté sur point d'arrêt — {addr:#x}")
                } else {
                    format!("Stopped at breakpoint — {addr:#x}")
                },
                theme.accent,
                None,
            ),
            Status::StepLimit => (s.status_step_limit.to_owned(), theme.bad, None),
            Status::NasmMissing => (s.status_nasm_missing.to_owned(), theme.bad, None),
            Status::Note(phrase) => (phrase(s).to_owned(), theme.good, None),
            // Le nombre devant la phrase : « 7 occurrences surlignées ».
            // Le compte ne se traduit pas, la phrase si — d'où les deux
            // morceaux séparés jusqu'ici.
            Status::Counted(combien, phrase) => {
                (format!("{combien} {}", phrase(s)), theme.good, None)
            }
            Status::Error(msg) => (
                if fr {
                    format!("Erreur : {msg}")
                } else {
                    format!("Error: {msg}")
                },
                theme.bad,
                None,
            ),
        }
    }

    /// Le message d'état, peint — et lisible en entier au survol quand la
    /// place a manqué.
    ///
    /// La troncature est **mesurée** et non devinée : la règle d'avant
    /// (« plus de quatre-vingt-dix caractères ») était une estimation, et
    /// la place dépend de la largeur de la fenêtre, de la police et du
    /// réglage de zoom. On compare ce que la phrase demande à ce qu'on
    /// lui a laissé, et l'infobulle n'apparaît que si le second est plus
    /// petit — une infobulle qui répète un texte entièrement visible
    /// apprend à ne plus les lire.
    fn status_message_ui(&self, ui: &mut egui::Ui) {
        let (texte, couleur, pastille) = self.status_line();
        if texte.is_empty() {
            return;
        }
        if let Some(couleur_pastille) = pastille {
            status_dot(ui, couleur_pastille);
        }
        let demandé = ui
            .painter()
            .layout_no_wrap(
                texte.clone(),
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::PLACEHOLDER,
            )
            .rect
            .width();
        let réponse = ui.colored_label(couleur, &texte);
        if demandé > réponse.rect.width() + 0.5 {
            réponse.on_hover_text(&texte);
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
    /// La légende de la barre de chaleur : ce que chaque teinte vaut.
    ///
    /// Une bande de couleur sans échelle ne dit rien — « plus rouge »
    /// ne devient « mille passages » que si on l'écrit. Les graduations
    /// suivent l'échelle réelle, celle de la ligne la plus chaude du
    /// programme, et sont logarithmiques comme elle.
    fn heat_legend_ui(&mut self, ui: &mut egui::Ui, max: u64) {
        let s = strings(self.lang);
        let theme = self.theme;
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.heat_legend);
            // Cinq repères entre une fois et le maximum : assez pour
            // situer une teinte, assez peu pour tenir sur une ligne.
            let repères = heat_legend_steps(max);
            for valeur in repères {
                let (rect, réponse) =
                    ui.allocate_exact_size(egui::vec2(14.0, 12.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 2.0, heat_colour(theme, valeur, max));
                réponse.on_hover_text(if self.lang == Lang::Fr {
                    format!("{valeur} passage(s)")
                } else {
                    format!("{valeur} pass(es)")
                });
                ui.colored_label(
                    theme.dim,
                    egui::RichText::new(sidebar::group_digits(valeur)).monospace(),
                );
                ui.add_space(4.0);
            }
            ui.colored_label(theme.dim, s.heat_legend_hint);
        });
    }

    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        // Avant de dessiner : la barre d'état, peinte plus tôt dans la
        // frame, lit ce qu'on calcule ici.
        self.refresh_fix();
        let theme = self.theme;
        // Les fichiers ouverts, en tête : on passe de l'un à l'autre
        // sans rien perdre de ce qu'on faisait sur les autres.
        self.tabs_bar_ui(ui);
        self.disk_changed_ui(ui);
        // La barre de recherche, au-dessus du code : elle pousse
        // l'éditeur d'une ligne plutôt que de flotter par-dessus, pour
        // ne jamais couvrir ce qu'on vient de trouver.
        self.find_ui(ui);
        self.rename_ui(ui);
        self.note_ui(ui);
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
            let g_armé = self.g_pending;
            let (enter_insert, exit_insert, mouvement, armer_g, désarmer_g) =
                ui.ctx().input(|inp| {
                let mut enter = false;
                let mut exit = false;
                let mut mouvement = None;
                let mut armer_g = false;
                let mut désarmer_g = false;
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
                            // Les déplacements, seulement en mode Normal
                            // et seulement s'ils n'ont pas déjà servi à
                            // un raccourci.
                            egui::Key::H if plain && !consumed.contains(key) => {
                                mouvement = Some(Move::Left);
                            }
                            egui::Key::L if plain && !consumed.contains(key) => {
                                mouvement = Some(Move::Right);
                            }
                            egui::Key::K if plain && !consumed.contains(key) => {
                                mouvement = Some(Move::Up);
                            }
                            egui::Key::J if plain && !consumed.contains(key) => {
                                mouvement = Some(Move::Down);
                            }
                            egui::Key::Num0 if plain => mouvement = Some(Move::LineStart),
                            egui::Key::Home => mouvement = Some(Move::LineStart),
                            egui::Key::End => mouvement = Some(Move::LineEnd),
                            // `G` : la fin du fichier, comme dans vim.
                            egui::Key::G if plain && modifiers.shift => {
                                mouvement = Some(Move::FileEnd);
                            }
                            // `g g` : le début. Deux frappes de la même
                            // touche, comme dans vim, et donc un témoin
                            // à garder d'une image à l'autre — `G` est
                            // la même touche avec Maj, il n'y a pas de
                            // conflit.
                            egui::Key::G if plain && !modifiers.shift => {
                                if g_armé {
                                    mouvement = Some(Move::FileStart);
                                    désarmer_g = true;
                                } else {
                                    armer_g = true;
                                }
                            }
                            // Les mouvements par mot : ce qu'on réclame
                            // en premier avec l'habitude de vim. Avancer
                            // caractère par caractère dans
                            // `mov qword [rbp-0x18], rax` use la patience.
                            egui::Key::W if plain && !consumed.contains(key) => {
                                mouvement = Some(Move::WordForward);
                            }
                            egui::Key::B if plain && !consumed.contains(key) => {
                                mouvement = Some(Move::WordBack);
                            }
                            egui::Key::E if plain && !consumed.contains(key) => {
                                mouvement = Some(Move::WordEnd);
                            }
                            _ => {}
                        }
                    }
                }
                (enter, exit, mouvement, armer_g, désarmer_g)
            });
            // `g` seul attend son second `g` ; tout le reste le désarme.
            // Sans cela, `g` puis `j` sauterait au début du fichier à la
            // prochaine frappe de `g`, une minute plus tard.
            self.g_pending = if désarmer_g {
                false
            } else {
                armer_g
            };
            // Se déplacer avant de changer de mode : `i` entre en
            // Insertion là où le curseur est, non là où il était.
            if self.editor_mode == EditorMode::Normal
                && let Some(mouvement) = mouvement
            {
                self.move_in_normal_mode(ui.ctx(), mouvement);
            }
            // Échap éteint aussi le surlignage, avant de changer de
            // mode : c'est le geste qui annule ce qu'on vient de
            // demander, et l'attendre d'un second raccourci serait une
            // chose de plus à retenir.
            //
            // Sans rien annoncer, en revanche : on appuie sur Échap pour
            // quitter l'insertion, et lire « Surlignage effacé » là où
            // l'on attend « NORMAL » répond à côté. L'effacement se voit
            // — les marques disparaissent —, il n'a pas à être dit.
            if exit_insert && self.tracked.is_some() {
                self.forget_matches();
            }
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
                        // Le plus grand nombre de passages : c'est lui qui
                        // donne l'échelle. Une ligne exécutée mille fois
                        // dans un programme qui en compte mille n'est pas
                        // chaude — elle est la normale.
                        let plus_chaude = passages.values().copied().max().unwrap_or(0);
                        let (gutter_rect, gutter_resp) = ui.allocate_exact_size(
                            egui::vec2(GUTTER_WIDTH, galley.rect.height().max(1.0)),
                            egui::Sense::click_and_drag(),
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
                            // La chaleur de la ligne : une bande à
                            // l'extrême gauche, d'autant plus vive que la
                            // ligne a tourné souvent. Un nombre par ligne
                            // se lit une ligne à la fois ; une bande de
                            // couleur se lit d'un coup d'œil, et c'est
                            // ainsi qu'on repère la boucle qui coûte.
                            if let Some(count) = passages.get(&line) {
                                painter.rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(gutter_rect.left(), row_rect.top()),
                                        egui::vec2(HEAT_WIDTH, row_rect.height()),
                                    ),
                                    0.0,
                                    heat_colour(theme, *count, plus_chaude),
                                );
                            }
                        }
                        // Combien de fois, au survol : la bande dit
                        // « souvent », le nombre dit « mille deux cent
                        // trois ». Quel que soit le mode de l'éditeur —
                        // on regarde un programme bien plus souvent
                        // qu'on ne le tape.
                        if self.show_tooltips
                            && let Some(pos) = gutter_resp.hover_pos()
                            && let Some(ligne) = ligne_sous(&galley, gutter_rect, pos)
                            && let Some(count) = passages.get(&ligne)
                        {
                            let texte = if self.lang == Lang::Fr {
                                format!("Ligne {ligne} : exécutée {count} fois")
                            } else {
                                format!("Line {ligne}: run {count} times")
                            };
                            gutter_resp.clone().show_tooltip_text(texte);
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
                        // Avant le `TextEdit` : les touches que la liste
                        // de propositions veut sont retirées de la file,
                        // sans quoi l'éditeur les appliquerait d'abord.
                        self.claim_completion_keys(ui);
                        let output = egui::TextEdit::multiline(&mut self.source)
                            .id(editor_id())
                            .frame(egui::Frame::NONE)
                            .desired_width(width)
                            .desired_rows(editor_rows(visible.y, row_height))
                            .code_editor()
                            .interactive(self.editor_mode == EditorMode::Insert)
                            .layouter(&mut layouter)
                            .show(ui);
                        // Les occurrences suivies, peintes *sous* le
                        // texte : par-dessus, même translucide, le fond
                        // ternirait les lettres — or c'est le mot qu'on
                        // cherche à lire, pas le rectangle.
                        self.paint_matches(ui, &output);
                        // Les commentaires qu'on a écrits, à droite de
                        // leur ligne : c'est là qu'on les lit, et non
                        // dans un panneau à côté qu'il faudrait mettre
                        // en face du code de mémoire.
                        self.paint_notes(ui, &output);
                        // En mode Normal, le `TextEdit` n'est pas
                        // interactif : egui ne peint aucun caret et ne
                        // rend plus de `cursor_range`. La position du
                        // curseur n'était donc visible nulle part — ni
                        // dans le code, ni dans la barre de statut, qui
                        // restait figée sur « Lig 1, Col 1 ». On la lit
                        // dans l'état conservé du champ, et on la peint
                        // soi-même : un bloc plein, comme vim, là où
                        // l'insertion reprendra.
                        if self.editor_mode == EditorMode::Normal
                            && let Some(state) =
                                egui::TextEdit::load_state(ui.ctx(), editor_id())
                            && let Some(range) = state.cursor.char_range()
                        {
                            let primary = range.primary;
                            self.cursor_pos = line_col(&self.source, primary.index.0);
                            self.cursor_byte = char_to_byte(&self.source, primary.index.0);
                            let place = output.galley.pos_from_cursor(primary);
                            // Monospace : la largeur d'une espace est
                            // celle de n'importe quel caractère, et vaut
                            // mieux que de mesurer le glyphe visé — il
                            // n'y en a aucun en fin de ligne.
                            let largeur = ui.ctx().fonts_mut(|f| f.glyph_width(&font_id, ' '));
                            let bloc = egui::Rect::from_min_size(
                                output.galley_pos + place.left_top().to_vec2(),
                                egui::vec2(largeur, place.height()),
                            );
                            // Translucide : le caractère sous le bloc
                            // doit rester lisible. L'inversion de vim
                            // demanderait de repeindre le glyphe.
                            ui.painter().rect_filled(
                                bloc,
                                1.0,
                                self.theme.accent.gamma_multiply(0.45),
                            );
                        }
                        if let Some(range) = output.cursor_range {
                            self.cursor_pos = line_col(&self.source, range.primary.index.0);
                            self.cursor_byte = char_to_byte(&self.source, range.primary.index.0);
                            // La sélection, pour « surligner les
                            // occurrences » : elle l'emporte sur le
                            // curseur quand il y en a une.
                            let (a, b) = (range.primary.index.0, range.secondary.index.0);
                            self.editor_selection = (a != b).then(|| {
                                char_to_byte(&self.source, a.min(b))
                                    ..char_to_byte(&self.source, a.max(b))
                            });
                            // Entrée garde l'indentation de la ligne
                            // qu'on quitte. egui insère un `\n` nu ; on
                            // ajoute les blancs derrière, et l'on avance
                            // le curseur d'autant.
                            // Les paires : posées après coup, comme
                            // l'indentation, parce que le `TextEdit` a
                            // déjà écrit ce qu'on vient de taper et que
                            // c'est de là qu'on raisonne.
                            if self.auto_pairs
                                && output.response.response.has_focus()
                                && range.is_empty()
                            {
                                let at = self.cursor_byte;
                                match pair_edit(&self.source, at) {
                                    PairEdit::Close(fermant) => {
                                        self.source.insert(at, fermant);
                                        // Le curseur ne bouge pas : il
                                        // reste entre les deux, qui est
                                        // le seul endroit utile.
                                        set_cursor_byte(ui.ctx(), &self.source, at);
                                    }
                                    PairEdit::StepOver => {
                                        // Retirer le fermant qu'on vient
                                        // d'écrire revient à traverser
                                        // celui qui était déjà là.
                                        let début = at
                                            - self.source[..at]
                                                .chars()
                                                .next_back()
                                                .map_or(0, char::len_utf8);
                                        self.source.replace_range(début..at, "");
                                        set_cursor_byte(ui.ctx(), &self.source, at);
                                    }
                                    PairEdit::None => {}
                                }
                            }
                            // Le retour arrière emporte la paire vide
                            // qu'il rouvrirait sans cela.
                            if self.auto_pairs
                                && output.response.response.has_focus()
                                && range.is_empty()
                                && ui.input(|i| i.key_pressed(egui::Key::Backspace))
                                && backspace_clears_pair(&self.source, self.cursor_byte)
                            {
                                let at = self.cursor_byte;
                                let fin = at
                                    + self.source[at..].chars().next().map_or(0, char::len_utf8);
                                self.source.replace_range(at..fin, "");
                            }
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
                        self.apply_fix_if_confirmed();
                        self.hover_help_ui(ui, &output);
                        // Après le `TextEdit` : son état existe, et c'est
                        // lui qu'on déplace.
                        self.apply_goto_line(&ui.ctx().clone());
                        self.apply_pending_cursor(&ui.ctx().clone());
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
    /// L'avertissement d'un fichier changé sous nos pieds.
    ///
    /// Non bloquant : une fenêtre modale au retour d'un `git checkout`
    /// interromprait pour une question qui n'est pas urgente. Deux
    /// boutons suffisent — relire le disque, ou garder ce qu'on a écrit
    /// et l'enregistrer quand on voudra.
    fn disk_changed_ui(&mut self, ui: &mut egui::Ui) {
        if !self.disk_changed {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        egui::Frame::new()
            .fill(theme.warn.gamma_multiply(0.12))
            .inner_margin(egui::Margin::symmetric(8, 5))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme.warn, s.disk_changed);
                    if ui.button(s.disk_reload).clicked() {
                        self.reload_from_disk();
                    }
                    if ui.button(s.disk_keep).clicked() {
                        // Garder, c'est accepter la version du disque
                        // comme point de comparaison : sans cela,
                        // l'avertissement reviendrait à chaque regard.
                        self.file_mtime = self
                            .current_file
                            .as_deref()
                            .and_then(file_mtime)
                            .or(self.file_mtime);
                        self.disk_changed = false;
                    }
                });
            });
        ui.add_space(2.0);
    }

    /// La barre des fichiers ouverts.
    ///
    /// Un onglet par fichier, le nom raccourci quand il est long — mais
    /// par le milieu, jamais par la fin : `long_nom_de_prog…` et
    /// `long_nom_de_prog2…` se ressemblent trop, alors que
    /// `long_n…rog.asm` et `long_n…og2.asm` se distinguent. Le point dit
    /// ce qui n'est pas enregistré, et la croix ferme.
    ///
    /// Un seul fichier ouvert : pas de barre. Elle n'apprend rien tant
    /// qu'il n'y a rien à choisir, et l'éditeur gagne sa hauteur.
    fn tabs_bar_ui(&mut self, ui: &mut egui::Ui) {
        if self.tabs.is_empty() {
            return;
        }
        let theme = self.theme;
        let s = strings(self.lang);
        let mut aller = None;
        let mut fermer = None;
        egui::ScrollArea::horizontal()
            .id_salt("tabs_bar")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (rang, (nom, modifié)) in self.tab_labels().into_iter().enumerate() {
                        let actif = rang == self.active_tab_index;
                        let étiquette = format!(
                            "{}{}",
                            raccourcir_nom(&nom, 18),
                            if modifié { " •" } else { "" }
                        );
                        let texte = egui::RichText::new(étiquette).color(if actif {
                            theme.text
                        } else {
                            theme.dim
                        });
                        if ui
                            .selectable_label(actif, texte)
                            .on_hover_text(&nom)
                            .clicked()
                        {
                            aller = Some(rang);
                        }
                        if ui
                            .small_button("×")
                            .on_hover_text(s.tab_close_hint)
                            .clicked()
                        {
                            fermer = Some(rang);
                        }
                        ui.add_space(6.0);
                    }
                });
            });
        ui.add_space(2.0);
        if let Some(rang) = aller {
            self.go_to_tab(rang);
        }
        if let Some(rang) = fermer {
            self.close_tab(rang);
        }
    }

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
    /// Va à l'occurrence suivante — ou précédente — et la sélectionne.
    ///
    /// Rien de surligné ? On surligne d'abord ce qui est sous le curseur.
    /// C'est le `*` puis `n` de vim en un seul geste, et cela évite de
    /// répondre « il n'y a rien » à quelqu'un qui a le mot sous les yeux.
    ///
    /// La boucle est circulaire : après la dernière vient la première.
    /// Buter contre la fin obligerait à remonter à la main pour repasser
    /// sur ce qu'on vient de dépasser.
    fn go_to_match(&mut self, avant: bool) {
        if self.tracked.is_none() {
            self.highlight_matches();
            if self.tracked.is_none() {
                return;
            }
        }
        self.refresh_matches();
        if self.tracked_spans.is_empty() {
            return;
        }
        let ici = self.cursor_byte;
        // La suivante est la première qui commence après le curseur ; à
        // défaut, on repart du début.
        let rang = if avant {
            self.tracked_spans
                .iter()
                .position(|é| é.start > ici)
                .unwrap_or(0)
        } else {
            self.tracked_spans
                .iter()
                .rposition(|é| é.end < ici)
                .unwrap_or(self.tracked_spans.len() - 1)
        };
        let cible = self.tracked_spans[rang].clone();
        self.cursor_byte = cible.start;
        self.pending_selection = Some(cible);
        // Sélectionner, c'est pouvoir remplacer tout de suite : on passe
        // donc en insertion, comme après un `i`.
        self.editor_mode = EditorMode::Insert;
        self.goto_line = None;
        self.status = Status::Counted(rang + 1, |s| s.matches_at);
    }

    /// Ouvre la saisie du nouveau nom.
    ///
    /// Il faut avoir surligné d'abord : renommer sans savoir *quoi*
    /// n'aurait pas de sens, et deviner le mot sous le curseur à ce
    /// moment-là ferait renommer autre chose que ce qu'on voyait.
    fn start_rename(&mut self) {
        if self.tracked.is_none() {
            // Un geste rendu : on surligne, et l'on pourra renommer.
            self.highlight_matches();
            if self.tracked.is_none() {
                self.status = Status::Note(|s| s.rename_nothing);
                return;
            }
        }
        self.rename = Some(
            self.tracked
                .as_ref()
                .map(|suivi| suivi.word.clone())
                .unwrap_or_default(),
        );
        self.rename_just_opened = true;
    }

    /// Applique le nouveau nom à toutes les occurrences surlignées.
    fn apply_rename(&mut self, nouveau: &str) {
        self.refresh_matches();
        let combien = self.tracked_spans.len();
        if combien == 0 || nouveau.is_empty() {
            self.rename = None;
            return;
        }
        self.source = occurrences::replace_all(&self.source, &self.tracked_spans, nouveau);
        // Le surlignage suit le nouveau nom : on vient de le poser
        // partout, et le voir confirme que rien n'a été oublié.
        if let Some(suivi) = &mut self.tracked {
            suivi.word = nouveau.to_owned();
        }
        self.tracked_source_len = 0;
        self.refresh_matches();
        self.rename = None;
        self.status = Status::Counted(combien, |s| s.rename_done);
    }

    /// La barre de renommage, quand elle est ouverte.
    fn rename_ui(&mut self, ui: &mut egui::Ui) {
        let Some(mut nouveau) = self.rename.clone() else {
            return;
        };
        let s = strings(self.lang);
        let theme = self.theme;
        let just_opened = std::mem::take(&mut self.rename_just_opened);
        let combien = self.tracked_spans.len();
        let mut appliquer = false;
        let mut annuler = false;

        ui.horizontal(|ui| {
            ui.colored_label(theme.good, format!("{combien} ×"));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut nouveau)
                    .id(egui::Id::new("rename_field"))
                    .hint_text(s.rename_placeholder)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(220.0),
            );
            if just_opened {
                champ.request_focus();
                // Tout sélectionné : on remplace le nom, on ne le
                // complète pas.
                if let Some(mut état) =
                    egui::TextEdit::load_state(ui.ctx(), egui::Id::new("rename_field"))
                {
                    état
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(nouveau.chars().count()),
                        )));
                    état.store(ui.ctx(), egui::Id::new("rename_field"));
                }
            }
            if champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                appliquer = true;
            }
            if framed_button(ui, theme, s.rename_apply).clicked() {
                appliquer = true;
            }
            if ui.button("x").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                annuler = true;
            }
        });

        if appliquer {
            self.apply_rename(&nouveau);
        } else if annuler {
            self.rename = None;
        } else {
            self.rename = Some(nouveau);
        }
    }

    /// Peint les occurrences suivies.
    ///
    /// En vert, parce que c'est la couleur que le thème réserve à « ça va
    /// bien » — et qu'elle est verte dans tous les thèmes livrés comme
    /// dans ceux qu'on écrit : chacun choisit la sienne, donc chacun
    /// garde son contraste. Un vert en dur serait illisible sur un thème
    /// clair.
    ///
    /// Un fond translucide et un liseré plein : le fond seul disparaît
    /// sur les thèmes très contrastés, le liseré seul se perd dans une
    /// ligne dense.
    fn paint_matches(&mut self, ui: &egui::Ui, output: &egui::text_edit::TextEditOutput) {
        self.refresh_matches();
        if self.tracked_spans.is_empty() {
            return;
        }
        let peintre = ui.painter_at(output.response.rect);
        for étendue in &self.tracked_spans {
            let début = byte_to_char(&self.source, étendue.start);
            let fin = byte_to_char(&self.source, étendue.end);
            let gauche = output
                .galley
                .pos_from_cursor(egui::text::CCursor::new(début));
            let droite = output
                .galley
                .pos_from_cursor(egui::text::CCursor::new(fin));
            // Sur deux rangées différentes : le texte s'est replié au
            // milieu du mot. On ne peint alors rien plutôt qu'un
            // rectangle qui traverserait la ligne du dessous — un
            // surlignage faux est pire qu'un surlignage manquant.
            if (gauche.top() - droite.top()).abs() > 0.5 {
                continue;
            }
            let rect = egui::Rect::from_min_max(
                output.galley_pos + gauche.left_top().to_vec2(),
                output.galley_pos + droite.left_bottom().to_vec2(),
            );
            if rect.width() <= 0.0 {
                continue;
            }
            peintre.rect_filled(rect, 2.0, self.theme.good.gamma_multiply(0.28));
            peintre.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, self.theme.good),
                egui::StrokeKind::Inside,
            );
        }
    }

    /// Ouvre la saisie d'un nom ou d'un commentaire pour la ligne où
    /// l'on est.
    ///
    /// Il faut une adresse : sur une ligne qui n'en a pas — une
    /// étiquette, un blanc — il n'y a rien à nommer, et le dire vaut
    /// mieux que d'ouvrir un champ qui n'écrira nulle part.
    fn start_note(&mut self, genre: NoteKind) {
        let ligne = self.cursor_pos.0;
        let adresse = self
            .assemble_state
            .as_ref()
            .and_then(|état| état.output.line_to_addr.get(&(ligne as u32)).copied());
        let Some(adresse) = adresse else {
            self.status = Status::Note(|s| s.notes_no_address);
            return;
        };
        // Le champ s'ouvre sur ce qui est déjà là : on corrige plus
        // souvent qu'on n'écrit de zéro.
        let actuel = match genre {
            NoteKind::Name => self.notes.name(adresse),
            NoteKind::Comment => self.notes.comment(adresse),
        }
        .unwrap_or_default()
        .to_owned();
        self.note_edit = Some((genre, adresse, actuel));
        self.note_just_opened = true;
    }

    /// La barre de saisie d'une note, quand elle est ouverte.
    fn note_ui(&mut self, ui: &mut egui::Ui) {
        let Some((genre, adresse, mut texte)) = self.note_edit.clone() else {
            return;
        };
        let s = strings(self.lang);
        let theme = self.theme;
        let just_opened = std::mem::take(&mut self.note_just_opened);
        let mut appliquer = false;
        let mut annuler = false;

        ui.horizontal(|ui| {
            ui.colored_label(
                theme.accent,
                egui::RichText::new(format!("{adresse:#x}")).monospace(),
            );
            ui.colored_label(
                theme.dim,
                match genre {
                    NoteKind::Name => s.action_name_address,
                    NoteKind::Comment => s.action_comment_address,
                },
            );
            // Relevées avant le champ : un `TextEdit` qui traite Entrée
            // la retire de la file, et la guetter après lui ne verrait
            // rien. Même piège que partout ailleurs.
            let entrée = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let échap = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut texte)
                    .id(egui::Id::new("note_field"))
                    .hint_text(match genre {
                        NoteKind::Name => s.notes_name_placeholder,
                        NoteKind::Comment => s.notes_comment_placeholder,
                    })
                    .font(egui::TextStyle::Monospace)
                    .desired_width(ui.available_width() - 120.0),
            );
            if just_opened {
                champ.request_focus();
            }
            if entrée && (champ.has_focus() || champ.lost_focus()) {
                appliquer = true;
            }
            if échap {
                annuler = true;
            }
            if framed_button(ui, theme, s.dialog_ok).clicked() {
                appliquer = true;
            }
        });

        if appliquer {
            let vide = texte.trim().is_empty();
            match genre {
                NoteKind::Name => self.notes.set_name(adresse, &texte),
                NoteKind::Comment => self.notes.set_comment(adresse, &texte),
            }
            self.save_notes();
            self.note_edit = None;
            self.status = if vide {
                Status::Note(|s| s.notes_cleared)
            } else {
                match genre {
                    NoteKind::Name => Status::Note(|s| s.notes_named),
                    NoteKind::Comment => Status::Note(|s| s.notes_commented),
                }
            };
        } else if annuler {
            self.note_edit = None;
        } else {
            self.note_edit = Some((genre, adresse, texte));
        }
    }

    /// Retrouve les notes de ce binaire, s'il en a.
    ///
    /// Appelé à l'ouverture. L'empreinte suit le contenu : un fichier
    /// déplacé ou renommé retrouve ses notes, et un binaire recompilé
    /// n'hérite pas de celles d'un autre.
    fn load_notes_for(&mut self, bytes: &[u8]) {
        let empreinte = notes::fingerprint(bytes);
        self.notes = match notes_dir() {
            Some(dossier) => notes::load(&dossier, &empreinte),
            None => notes::Notes::default(),
        };
        self.notes_fingerprint = Some(empreinte);
    }

    /// Écrit les notes du binaire ouvert.
    ///
    /// Tout de suite après chaque changement : un nom qu'on vient de
    /// trouver et qui disparaît au prochain lancement se cherche deux
    /// fois.
    fn save_notes(&mut self) {
        let (Some(empreinte), Some(dossier)) = (&self.notes_fingerprint, notes_dir()) else {
            return;
        };
        if let Err(e) = notes::save(&dossier, empreinte, &self.notes) {
            self.status = Status::Error(format!("{}: {e}", strings(self.lang).notes_not_saved));
        }
    }


    /// Peint les commentaires à droite de leur ligne.
    ///
    /// À droite du code, et non dans un panneau à côté : un commentaire
    /// qu'il faut mettre en face de sa ligne de mémoire ne se lit pas.
    /// En couleur de commentaire, comme ceux qu'on écrit dans une source
    /// — ce sont les mêmes, à ceci près qu'ils ne sont pas dans le
    /// fichier.
    fn paint_notes(&self, ui: &egui::Ui, output: &egui::text_edit::TextEditOutput) {
        if self.notes.comments.is_empty() {
            return;
        }
        let Some(état) = self.assemble_state.as_ref() else {
            return;
        };
        let peintre = ui.painter_at(output.response.rect);
        let police = egui::TextStyle::Monospace.resolve(ui.style());
        // Le listing est une ligne par instruction : la rangée `n` du
        // galley est la ligne `n + 1` de la source.
        for (rang, rangée) in output.galley.rows.iter().enumerate() {
            let ligne = rang as u32 + 1;
            let Some(adresse) = état.output.line_to_addr.get(&ligne) else {
                continue;
            };
            let Some(texte) = self.notes.comment(*adresse) else {
                continue;
            };
            // Après la fin du texte de la ligne, avec un blanc : collé,
            // on ne saurait plus où finit le code.
            let rect = rangée.rect().translate(output.galley_pos.to_vec2());
            let à = egui::pos2(rect.right() + 24.0, rect.top());
            peintre.text(
                à,
                egui::Align2::LEFT_TOP,
                format!("; {texte}"),
                police.clone(),
                self.theme.comment,
            );
        }
    }

    /// Relève ce qui est sous le curseur et montre où ça revient.
    ///
    /// Une sélection l'emporte sur le curseur : quelqu'un qui a
    /// sélectionné `ax` demande les `ax`, et lui rendre les `rax` serait
    /// répondre à côté.
    ///
    /// Refait sur le même mot, cela efface : c'est la même touche qui
    /// allume et qui éteint, comme un interrupteur — et l'on n'a pas à
    /// se souvenir d'un second raccourci pour revenir en arrière.
    fn highlight_matches(&mut self) {
        let suivi = match self.editor_selection.clone() {
            Some(étendue) if étendue.end > étendue.start => {
                occurrences::tracked_selection(&self.source, étendue)
            }
            _ => occurrences::tracked_at(&self.source, self.cursor_byte),
        };
        let Some(suivi) = suivi else {
            self.status = Status::Note(|s| s.matches_none);
            return;
        };
        // Le même que la dernière fois : on éteint.
        if self
            .tracked
            .as_ref()
            .is_some_and(|actuel| actuel.word == suivi.word && actuel.kind == suivi.kind)
        {
            self.clear_matches();
            return;
        }
        self.tracked_spans = occurrences::find_all(&self.source, &suivi);
        let combien = self.tracked_spans.len();
        self.tracked = Some(suivi);
        self.tracked_source_len = self.source.len();
        self.status = Status::Counted(combien, |s| s.matches_found);
    }

    /// Éteint le surlignage, et le dit.
    ///
    /// À n'employer que lorsque c'est le geste demandé — refaire la
    /// commande de surlignage, qui fonctionne en interrupteur. Éteindre
    /// est visible à l'écran : les marques disparaissent. L'annoncer
    /// quand personne ne l'a demandé écrase un message plus utile.
    fn clear_matches(&mut self) {
        if self.tracked.is_none() {
            return;
        }
        self.forget_matches();
        self.status = Status::Note(|s| s.matches_cleared);
    }

    /// Éteint le surlignage sans rien dire.
    ///
    /// Pour les effets de bord : Échap sert d'abord à quitter le mode
    /// Insertion, et annoncer « Surlignage effacé » à ce moment-là
    /// répond à côté de ce qu'on vient de faire.
    fn forget_matches(&mut self) {
        self.tracked = None;
        self.tracked_spans.clear();
    }

    /// Remet les étendues à jour quand la source a changé.
    ///
    /// Les positions sont en octets : taper une lettre plus haut décale
    /// tout ce qui suit, et garder les anciennes surlignerait le milieu
    /// des mots. On recalcule plutôt que de deviner le décalage — c'est
    /// une recherche sur un fichier qu'on a déjà en mémoire, et cela
    /// coûte moins que de se tromper.
    fn refresh_matches(&mut self) {
        let Some(suivi) = self.tracked.clone() else {
            return;
        };
        if self.source.len() == self.tracked_source_len {
            return;
        }
        self.tracked_spans = occurrences::find_all(&self.source, &suivi);
        self.tracked_source_len = self.source.len();
    }

    /// Recopie la ligne courante en dessous, et descend dessus.
    ///
    /// Sans effet sur un listing venu d'un binaire : celui-ci est
    /// reconstruit, et y insérer une ligne décalerait toutes les
    /// correspondances ligne <-> adresse dont dépendent la gouttière, les
    /// points d'arrêt et le suivi de RIP.
    fn duplicate_line(&mut self) {
        if self.buffer_came_from_a_binary() {
            return;
        }
        let (source, curseur) = duplicated_line(&self.source, self.cursor_byte);
        self.source = source;
        self.cursor_byte = curseur;
        self.pending_cursor = Some(curseur);
    }

    /// Calcule, si besoin, la correction proposée pour la ligne du
    /// curseur.
    ///
    /// Ce qui rend le calcul caduc : changer de ligne, changer le texte
    /// de cette ligne, changer la longueur du fichier — un `jmp` vers
    /// une étiquette qu'on vient d'écrire ailleurs doit cesser d'être
    /// signalé — ou changer de langue, puisque la phrase l'est.
    ///
    /// Une modification ailleurs dans le fichier qui n'en change pas la
    /// longueur passe à travers, et c'est assumé : renommer une
    /// étiquette sans changer le nombre de lettres laisse une
    /// proposition d'une frappe en retard, ce qui se corrige à la frappe
    /// suivante et ne coûte rien à personne.
    fn refresh_fix(&mut self) {
        let index = self.cursor_pos.0.saturating_sub(1);
        let clé = FixKey {
            index,
            ligne: self.source.lines().nth(index).unwrap_or("").to_owned(),
            taille: self.source.len(),
            lang: self.lang,
        };
        if self.fix.as_ref().is_some_and(|f| f.clé == clé) {
            return;
        }
        let correction = fixes::propose(&self.source, index, strings(self.lang));
        self.fix = Some(FixCache { clé, correction });
    }

    /// La correction proposée en ce moment, s'il y en a une.
    fn current_fix(&self) -> Option<&fixes::Correction> {
        self.fix.as_ref()?.correction.as_ref()
    }

    /// Alt vient d'être enfoncé et rien d'autre ne l'a pris : la
    /// correction s'applique.
    ///
    /// À appeler **après** la liste de propositions, qui a la priorité :
    /// elle est sous le curseur, on la regarde, et c'est elle qu'on
    /// croit confirmer. La correction, elle, est écrite dans la barre
    /// d'état — on la confirme quand plus rien ne la précède.
    fn apply_fix_if_confirmed(&mut self) {
        if !self.alt_pressed || self.suggestions.is_some() {
            return;
        }
        self.alt_pressed = false;
        let Some(cache) = self.fix.as_ref() else {
            return;
        };
        let Some(correction) = cache.correction.as_ref() else {
            return;
        };
        let index = cache.clé.index;
        let Some(début) = ligne_début(&self.source, index) else {
            return;
        };
        let fin = début + self.source[début..].find('\n').unwrap_or(self.source.len() - début);
        let nouvelle = correction.ligne.clone();
        self.source.replace_range(début..fin, &nouvelle);
        // Le curseur au bout de la ligne réécrite : c'est là qu'on
        // était, et c'est de là qu'on repart.
        self.pending_cursor = Some(byte_to_char(&self.source, début + nouvelle.len()));
        self.status = Status::Note(|s| s.fix_applied);
        // Le cache parle d'une ligne qui n'existe plus.
        self.fix = None;
    }

    /// Retire du clavier ce que la liste de propositions veut pour elle.
    ///
    /// À appeler **avant** le `TextEdit` : Tab, les flèches et Échap ont
    /// un sens dans l'éditeur, et celui-ci les appliquerait le premier.
    /// Ne prend rien quand aucune liste n'est ouverte — Tab indente alors
    /// comme d'habitude, et les flèches déplacent le curseur.
    fn claim_completion_keys(&mut self, ui: &egui::Ui) {
        self.completion_keys = CompletionKeys::default();
        if self.suggestions.is_none() || !self.completion {
            return;
        }
        // `consume_key` retire l'événement : ce que la liste prend,
        // l'éditeur ne le verra pas.
        ui.input_mut(|i| {
            let vide = egui::Modifiers::NONE;
            self.completion_keys = CompletionKeys {
                accept: i.consume_key(vide, egui::Key::Tab),
                down: i.consume_key(vide, egui::Key::ArrowDown),
                up: i.consume_key(vide, egui::Key::ArrowUp),
                dismiss: i.consume_key(vide, egui::Key::Escape),
            };
        });
    }

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
        // Partagé avec la correction de code, qui passe après.
        self.alt_pressed = alt_pressed;

        let Some(range) = output.cursor_range else {
            self.suggestions = None;
            return;
        };
        if !self.completion || self.editor_mode != EditorMode::Insert {
            self.suggestions = None;
            return;
        }

        let touches = self.completion_keys;
        // Échap referme sans rien écrire. La liste revient au caractère
        // suivant : ce n'est pas « plus jamais », c'est « pas celle-ci ».
        if touches.dismiss {
            self.suggestions = None;
            self.completion_dismissed = true;
            return;
        }

        let cursor = range.primary.index.0;
        let byte = char_to_byte(&self.source, cursor);
        let avant = self.suggestions.clone();
        self.suggestions = completion::suggest(&self.source, byte);
        // Une liste qu'on vient d'écarter ne se rouvre qu'une fois le mot
        // changé : la rouvrir sur le même mot rendrait Échap inopérant.
        if self.completion_dismissed {
            if self.suggestions.as_ref().map(|s| &s.prefix) == avant.as_ref().map(|s| &s.prefix) {
                self.suggestions = None;
                return;
            }
            self.completion_dismissed = false;
        }
        // Le choix repart du haut dès que la liste change : il désignait
        // une proposition qui n'y est peut-être plus.
        if self.suggestions.as_ref().map(|s| &s.prefix) != avant.as_ref().map(|s| &s.prefix) {
            self.completion_index = 0;
        }
        let Some(suggestions) = self.suggestions.clone() else {
            return;
        };

        // Les flèches choisissent. Sans elles, on ne pouvait accepter que
        // la première proposition, et la liste ne servait qu'à regarder.
        if touches.down || touches.up {
            self.completion_index = completion::next_index(
                self.completion_index,
                suggestions.candidates.len(),
                touches.down,
            );
        }

        // Tab accepte, comme partout ailleurs, et `Alt` aussi — celle-ci
        // était le seul moyen jusqu'ici, ce qui ne se devinait pas.
        if alt_pressed || touches.accept {
            // Pris : la correction de code ne verra pas cette frappe.
            self.alt_pressed = false;
            self.accept_completion(ui.ctx(), &suggestions);
            return;
        }

        // Sous le curseur, comme une bulle : la liste suit la frappe.
        let mut cliqué: Option<usize> = None;
        let cursor_rect = output.galley.pos_from_cursor(range.primary);
        let at = output.galley_pos + cursor_rect.left_bottom().to_vec2();
        egui::Area::new(egui::Id::new("completion"))
            .order(egui::Order::Foreground)
            .fixed_pos(at + egui::vec2(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let choisi = self
                        .completion_index
                        .min(suggestions.candidates.len().saturating_sub(1));
                    for (index, candidate) in suggestions.candidates.iter().enumerate() {
                        let colour = candidate.kind.colour(theme);
                        let text = egui::RichText::new(&candidate.text).monospace();
                        // Celui qui sera pris, et non plus le premier :
                        // les flèches déplacent ce choix, et ne rien
                        // montrer bouger rendrait les flèches inutiles.
                        if index == choisi {
                            ui.horizontal(|ui| {
                                ui.colored_label(colour, text.strong());
                                ui.colored_label(theme.dim, s.completion_accept);
                            });
                        } else {
                            // Cliquable : la liste est sous le curseur,
                            // et viser une ligne à la souris est parfois
                            // plus court que de compter les flèches.
                            if ui
                                .add(
                                    egui::Label::new(text.color(theme.dim))
                                        .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                cliqué = Some(index);
                            }
                        }
                    }
                });
            });
        if let Some(index) = cliqué {
            self.completion_index = index;
            self.accept_completion(ui.ctx(), &suggestions);
        }
    }

    /// Écrit la proposition choisie à la place du mot commencé, et
    /// replace le curseur derrière.
    fn accept_completion(&mut self, ctx: &egui::Context, suggestions: &completion::Suggestions) {
        let Some(accepted) = suggestions.accepted_at(self.completion_index) else {
            return;
        };
        let accepted = accepted.to_owned();
        let accepted = accepted.as_str();
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
        self.completion_index = 0;
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

    /// Replace le curseur là où la dernière modification l'attend.
    fn apply_pending_cursor(&mut self, ctx: &egui::Context) {
        if let Some(étendue) = self.pending_selection.take() {
            let début = byte_to_char(&self.source, étendue.start.min(self.source.len()));
            let fin = byte_to_char(&self.source, étendue.end.min(self.source.len()));
            if let Some(mut état) = egui::TextEdit::load_state(ctx, editor_id()) {
                état
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(début),
                        egui::text::CCursor::new(fin),
                    )));
                état.store(ctx, editor_id());
            }
            ctx.memory_mut(|m| m.request_focus(editor_id()));
            return;
        }
        let Some(at) = self.pending_cursor.take() else {
            return;
        };
        set_cursor_byte(ctx, &self.source, at.min(self.source.len()));
    }

    /// Le panneau bas : la console du programme et les diagnostics de
    /// nasm, sur toute la largeur de la fenêtre.
    ///
    /// La console y a la largeur d'un terminal — un programme écrit en 80
    /// colonnes, le panneau latéral n'en offrait que trente-quatre — et
    /// les diagnostics y trouvent enfin de quoi être lus : ils étaient
    /// jusqu'ici entassés sous la barre de statut, sur deux lignes
    /// hautes de rien.
    /// La hauteur que la console réclame pour ce qu'elle a à dire.
    ///
    /// Une console qui annonce « le programme n'a rien écrit » n'a pas
    /// besoin du quart de l'écran ; vingt lignes de sortie, si. On
    /// compte donc les lignes, on borne — au-delà d'une douzaine, c'est
    /// la barre de défilement qui prend le relais — et on ajoute ce qui
    /// entoure : les onglets, le séparateur, et le champ d'entrée quand
    /// un programme peut lire.
    fn console_wanted_height(&self, ui: &egui::Ui) -> f32 {
        let ligne = ui.text_style_height(&egui::TextStyle::Monospace);
        let lignes = match self.console_tab {
            ConsoleTab::Output => self
                .emulator
                .as_ref()
                .map(|emulator| {
                    String::from_utf8_lossy(emulator.output())
                        .lines()
                        .count()
                        .max(1)
                })
                .unwrap_or(1),
            ConsoleTab::Diagnostics => self.diagnostics.len().max(1),
            // Une ligne par ligne chaude, plus l'en-tête et le total.
            ConsoleTab::Profile => self.line_visits().len().max(1) + 2,
        };
        // Les onglets, le séparateur et les marges du panneau.
        let chrome = ui.text_style_height(&egui::TextStyle::Button) + 28.0;
        // La zone de saisie, comptée ligne à ligne comme elle se dessine.
        //
        // Une seule ligne y était réservée. Elle en porte jusqu'à quatre
        // — la mention d'attente, les arguments, l'en-tête de
        // l'environnement, l'entrée elle-même —, plus les marges de son
        // cadre : le panneau était trop court, et le champ se faisait
        // rogner par le bas. On ne voyait plus où taper au moment précis
        // où le programme le demandait.
        // Ce que la zone de saisie réclame. La mesure de l'image
        // précédente quand elle est plus grande — l'environnement déplié
        // découvre un champ de deux lignes —, jamais quand elle est plus
        // petite : voir `console_input_estimate`.
        let entrée = self
            .console_input_estimate(ui)
            .max(self.console_input_height.unwrap_or(0.0));
        let contenu = ligne * lignes.min(12) as f32;
        (chrome + entrée + contenu).max(CONSOLE_MIN_HEIGHT)
    }

    /// La hauteur que la zone de saisie réclame, comptée rangée par
    /// rangée. Zéro quand il n'y a pas de saisie à montrer.
    ///
    /// **Calculée, et non mesurée** — et c'est tout le point. On se
    /// servait de la hauteur que la zone avait prise à l'image
    /// précédente, ce qui paraît plus juste puisqu'elle seule connaît sa
    /// vraie taille. Mais cette hauteur est celle qu'on lui a *allouée* :
    /// trop courte, la zone est tronquée, la mesure rend la hauteur
    /// tronquée, et le plancher qu'elle sert à calculer reste sous ce
    /// qu'il faut. Le panneau restait court parce qu'il était court.
    ///
    /// C'est ce qui faisait disparaître la rangée « Entrée : » — avec
    /// ses deux boutons — sur une console dont la hauteur avait été
    /// fixée à la main à 128 px : le programme demandait un mot de
    /// passe, et il n'y avait nulle part où le taper.
    fn console_input_estimate(&self, ui: &egui::Ui) -> f32 {
        if !self.console_shows_input() {
            return 0.0;
        }
        let rangée = ui
            .spacing()
            .interact_size
            .y
            .max(ui.text_style_height(&egui::TextStyle::Body))
            + ui.spacing().item_spacing.y;
        // L'entrée, toujours : c'est la rangée qui porte le champ et ses
        // deux boutons.
        let mut hauteur = rangée;
        if matches!(self.status, Status::WaitingForInput) {
            hauteur += rangée;
        }
        if self
            .assemble_state
            .as_ref()
            .is_some_and(|état| état.from_binary.is_some())
        {
            // Les arguments, et l'en-tête de l'environnement.
            hauteur += rangée * 2.0;
            if self.console_env_open {
                // Déplié : une ligne d'aide et un champ de deux lignes.
                hauteur += rangée * 3.0;
            }
        }
        // Ce qui n'est pas une rangée, compté terme à terme plutôt
        // qu'arrondi : un forfait de 24 px paraissait large et manquait
        // de neuf — juste assez pour couper la rangée « Entrée » en
        // deux, dont on ne voyait plus que le haut, champ et boutons
        // sous le bord de la fenêtre.
        let espacement = ui.spacing().item_spacing.y;
        // Le trait de séparation, et le blanc de part et d'autre.
        let séparateur = 6.0 + espacement * 2.0;
        // Les marges hautes et basses du cadre qui entoure la saisie,
        // plus son liseré quand il y en a un — deux pixels en haut, deux
        // en bas. Les oublier suffisait à couper « Envoyer » d'un
        // cheveu : le cadre a été épaissi pour se voir, et il fallait
        // que la place suive.
        let marges_du_cadre = if matches!(self.status, Status::WaitingForInput) {
            12.0 + 4.0
        } else {
            12.0
        };
        // La ligne d'attente porte une pastille, qui prend la hauteur
        // d'une rangée déjà comptée — rien à ajouter pour elle.
        // Le `add_space` posé avant la dernière rangée.
        let respiration = 2.0;
        // Et de quoi ne pas jouer au pixel près : les hauteurs de
        // police varient d'un thème à l'autre, et deux pixels de trop ne
        // coûtent rien quand deux de moins coupent la ligne où l'on tape.
        let sûreté = 8.0;
        hauteur + séparateur + marges_du_cadre + respiration + sûreté
    }

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
            ConsoleTab::Profile => self.profile_ui(ui),
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
            None => self.status = Status::Note(|s| s.goto_label_unknown),
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

    /// Note l'exercice en cours comme réussi si son but est atteint.
    ///
    /// Appelé à chaque image, mais n'écrit qu'une fois : le premier
    /// passage au vert suffit, et repasser sur un exercice déjà fait ne
    /// le défait pas — on ne retire pas ce qu'on a compris.
    fn note_goal_reached(&mut self) {
        let Some((_, true)) = self.exercise_goal() else {
            return;
        };
        // Un but atteint sans avoir rien exécuté n'est pas une
        // solution : « rax == 0 » est vrai avant le premier pas, et
        // remettre à zéro suffisait à s'attribuer un score de zéro
        // instruction — imbattable, et faux. On exige que le programme
        // ait tourné *et* qu'il soit allé jusqu'à son arrêt.
        let Some(emulator) = &self.emulator else {
            return;
        };
        if emulator.executed() == 0 || !emulator.cpu().halted {
            return;
        }
        let Some(id) = self.current_exercise.clone() else {
            return;
        };
        // Le coût de la course qui vient de réussir. Le meilleur est
        // gardé, jamais le dernier : on essaie une idée plus courte, elle
        // échoue, et le score d'avant n'a pas à en pâtir.
        let exécutées = self.emulator.as_ref().map(|e| e.executed()).unwrap_or(0);
        let meilleur = self.exercises_best.entry(id.clone()).or_insert(exécutées);
        if exécutées < *meilleur {
            *meilleur = exécutées;
        }
        if self.exercises_done.insert(id) {
            self.status = Status::Note(|s| s.exercises_done_note);
        }
    }

    /// Où en est le but de l'exercice en cours.
    ///
    /// `None` quand aucun exercice n'est chargé, qu'il n'a pas de but
    /// vérifiable, ou qu'aucun programme ne tourne : on ne dit pas
    /// « pas atteint » à quelqu'un qui n'a encore rien lancé.
    ///
    /// Le but est une liste de conditions — la grammaire des points
    /// d'arrêt conditionnels — et elles doivent toutes tenir.
    fn exercise_goal(&self) -> Option<(Vec<String>, bool)> {
        let id = self.current_exercise.as_deref()?;
        let exercise = namsdec_exercises::find(&self.exercise_packs, id)?;
        if exercise.goal.is_empty() {
            return None;
        }
        let emulator = self.emulator.as_ref()?;
        let cpu = emulator.cpu();
        let mémoire = PeekMemory(&cpu.mem);
        // Une condition qu'on ne sait pas lire ne vaut pas « atteint » :
        // mieux vaut un but qu'on n'atteint jamais qu'un faux succès.
        let atteint = exercise.goal.iter().all(|texte| {
            condition::parse(texte).is_some_and(|c| c.holds(&cpu.regs, Some(&mémoire)))
        });
        Some((exercise.goal.clone(), atteint))
    }

    /// L'état du processeur, en texte : ce qu'on colle pour demander de
    /// l'aide, ou dans un compte rendu de TP.
    ///
    /// C'est le `context` de pwndbg : la prochaine instruction, les
    /// registres, les indicateurs, le haut de la pile et la chaîne des
    /// appels. Un état qui ne sort pas de l'écran oblige à le recopier à
    /// la main — et on recopie mal.
    fn context_text(&self) -> Option<String> {
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
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.unsaved_title) {
                    annuler = true;
                }
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.set_max_width(380.0);
                ui.label(s.unsaved_question);
                // Quand plusieurs fichiers sont en jeu, le dire : « le
                // fichier n'est pas enregistré » devant trois onglets
                // modifiés laisse croire qu'on n'en perd qu'un.
                let en_danger = self.modified_tabs();
                if en_danger > 1 {
                    ui.add_space(4.0);
                    ui.colored_label(
                        theme.warn,
                        if self.lang == Lang::Fr {
                            format!("{en_danger} fichiers ouverts ne sont pas enregistrés.")
                        } else {
                            format!("{en_danger} open files are not saved.")
                        },
                    );
                }
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
            // Quitter emporte tous les fichiers : tant qu'il en reste un
            // à enregistrer, on l'ouvre et l'on repose la question. « Ne
            // pas enregistrer », lui, vaut pour tous — c'est ce qu'il
            // veut dire.
            if matches!(pending, Pending::Quit)
                && let Some(rang) = self.first_modified_tab()
            {
                self.go_to_tab(rang);
                self.pending = Some(Pending::Quit);
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
        let mut fermer_ok = false;
        let mut aller: Option<u32> = None;
        let mut vise: Option<u64> = None;
        let mut revenir_au_début = false;
        // Relevée avant la fenêtre : elle emprunte `self` en écriture.
        let addr_to_line = self
            .assemble_state
            .as_ref()
            .map(|state| state.addr_to_line.clone())
            .unwrap_or_default();

        let fenêtre_disassembly_window = egui::Window::new(s.disassembly_title)
            .id(egui::Id::new("disassembly_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([680.0, 560.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center());
        let rendue_disassembly_window = self
            .maximizable(fenêtre_disassembly_window, Self::DISASSEMBLY_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.disassembly_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "disassembly") {
                    fermer_ok = true;
                }
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
        self.place_maximize_button(ctx, Self::DISASSEMBLY_WINDOW, rendue_disassembly_window);
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
        self.show_disassembly = open && !fermer_ok;
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
        let mut fermer_ok = false;
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
            // Même raison qu'ailleurs : le titre change avec la langue,
            // l'identifiant ne doit pas.
            .id(egui::Id::new("cyclic_window"))
            .open(&mut open)
            .resizable(true)
            .default_size([560.0, 400.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.cyclic_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "cyclic") {
                    fermer_ok = true;
                }
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
                        self.status = Status::Note(|s| s.cyclic_copied);
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
            self.status = Status::Note(|s| s.cyclic_sent);
        }
        self.show_cyclic = open && !fermer_ok;
    }

    /// Le parcours entier, tel que l'émulateur l'a retenu.
    ///
    /// Sans filtre : c'est celui de la barre, dont la forme n'aurait
    /// aucun sens si on lui retirait les pas ordinaires — une boucle de
    /// mille tours y paraîtrait aussi courte qu'un appel.
    fn timeline_trace(&self) -> Vec<namsdec_emu::Step> {
        self.emulator
            .as_ref()
            .map(|emulator| emulator.trace().iter().copied().collect())
            .unwrap_or_default()
    }

    fn timeline_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_timeline {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = true;
        let mut fermer_ok = false;
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

        let fenêtre_timeline_window = egui::Window::new(s.timeline_title)
            .id(egui::Id::new("timeline_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([520.0, 420.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center());
        let rendue_timeline_window = self
            .maximizable(fenêtre_timeline_window, Self::TIMELINE_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.timeline_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "timeline") {
                    fermer_ok = true;
                }
                if pas.is_empty() {
                    ui.weak(if self.timeline_notable_only {
                        s.timeline_nothing_notable
                    } else {
                        s.timeline_empty
                    });
                    return;
                }
                ui.weak(s.timeline_hint);
                // La barre, avant la liste : elle répond à « où j'en
                // suis », question qui vient avant « qu'est-ce qui s'est
                // passé ». Le filtre ne la touche pas — c'est le parcours
                // entier qu'elle montre, sans quoi sa forme mentirait.
                if let Some(index) = timeline_bar(ui, theme, self.timeline_trace()) {
                    let total = self.timeline_trace().len();
                    revenir = Some(timeline::rewind_to(total, index));
                }
                ui.add_space(4.0);
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
        self.place_maximize_button(ctx, Self::TIMELINE_WINDOW, rendue_timeline_window);

        if let Some(recul) = revenir {
            self.rewind(recul);
        }
        if let Some(ligne) = aller {
            self.goto_line = Some(ligne);
        }
        self.show_timeline = open && !fermer_ok;
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
        let mut fermer_ok = false;
        let mut retirer = None;
        egui::Window::new(s.breakpoints_title)
            .id(egui::Id::new("breakpoints_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([480.0, 320.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.breakpoints_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "breakpoints") {
                    fermer_ok = true;
                }
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
        self.show_breakpoints = open && !fermer_ok;
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
        let mut fermer_ok = false;
        let fenêtre_analysis_window = egui::Window::new(s.analysis_title)
            // Un identifiant explicite : le titre change avec la langue,
            // et la fenêtre perdrait sa position en passant à l'anglais.
            .id(egui::Id::new("analysis_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // Sept modules à la suite, dont un histogramme et plusieurs
            // tableaux : la fenêtre part large ET haute. 620 px de large
            // coupaient les tables de fonctions au milieu d'une colonne,
            // et l'on relisait un rapport en le faisant défiler dans les
            // deux sens.
            //
            // Proportionnée à l'écran, comme celle des exercices : un
            // chiffre en dur déborde d'un petit écran et se perd sur un
            // grand.
            .default_size(analysis_window_size(ctx))
            .min_size([560.0, 420.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center());
        let rendue_analysis_window = self
            .maximizable(fenêtre_analysis_window, Self::ANALYSIS_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.analysis_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "analysis") {
                    fermer_ok = true;
                }
                // Ce qu'on a compris d'un binaire ne vaut que si on peut
                // le transmettre : le rapport part en Markdown, prêt à
                // coller dans un compte rendu ou un support de cours.
                //
                // La rangée est encadrée d'air : collée sous le filet du
                // bandeau, elle passait pour une seconde barre de titre
                // plutôt que pour deux boutons.
                ui.add_space(6.0);
                egui::Frame::NONE
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 0,
                        bottom: 0,
                    })
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(s.analysis_copy).clicked()
                                && let Some(report) = self.analysis_report()
                            {
                                ui.ctx().copy_text(report);
                                self.status = Status::Note(|s| s.analysis_copied);
                            }
                            // Et sur le disque : un compte rendu se
                            // garde, se relit, s'attache à un courriel.
                            // Le presse-papier ne survit pas à la
                            // fenêtre suivante.
                            if ui
                                .button(s.analysis_save)
                                .on_hover_text(s.analysis_save_hint)
                                .clicked()
                            {
                                self.spawn_file_dialog(FileDialogKind::ExportReport);
                            }
                            // Le vidage a son propre enregistrement :
                            // ce n'est pas un rapport, c'est le fichier
                            // lui-même, et il sert à être comparé.
                            if self.analysis_module == "dump"
                                && ui
                                    .button(s.dump_save)
                                    .on_hover_text(s.dump_save_hint)
                                    .clicked()
                            {
                                self.spawn_file_dialog(FileDialogKind::ExportDump);
                            }
                            ui.separator();
                            ui.weak(s.analysis_hint);
                        });
                    });
                ui.add_space(8.0);

                // Une colonne de navigation à gauche, le module à
                // droite. Les sept modules empilés les uns sous les
                // autres n'employaient que la hauteur : on relisait une
                // page de deux mille pixels dans une fenêtre large de
                // mille, en cherchant un titre à l'ascenseur. Ici, on
                // choisit, et le module a toute la largeur.
                let lang = self.lang.code();
                let titres = self.re_registry.entries(lang);
                let theme = self.theme;
                egui::Panel::left(egui::Id::new("analysis_nav"))
                    .resizable(true)
                    .default_size(200.0)
                    .min_size(150.0)
                    .max_size(340.0)
                    // Un fond léger et une marge : les deux colonnes se
                    // lisent alors comme deux zones, et non comme un
                    // texte coupé par un trait. Sans la marge de droite,
                    // les libellés touchaient la ligne de séparation.
                    .frame(
                        egui::Frame::NONE
                            .fill(theme.dim.gamma_multiply(0.07))
                            .inner_margin(egui::Margin {
                                left: 8,
                                right: 10,
                                top: 8,
                                bottom: 8,
                            }),
                    )
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("analysis_nav_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                // Justifié à gauche : les entrées
                                // prennent toute la largeur — on clique
                                // la ligne, non le mot — et leur texte
                                // commence au même endroit. Centrés,
                                // treize libellés de longueurs
                                // différentes n'ont plus de bord commun,
                                // et l'œil doit chercher chacun.
                                ui.with_layout(
                                    egui::Layout::top_down_justified(egui::Align::LEFT),
                                    |ui| {
                                        ui.spacing_mut().item_spacing.y = 3.0;
                                        ui.spacing_mut().button_padding =
                                            egui::vec2(8.0, 5.0);
                                        // « Tout » en tête : c'est la
                                        // forme d'un compte rendu, celle
                                        // qu'on copie.
                                        let tout = self.analysis_module.is_empty();
                                        if ui
                                            .add(egui::Button::selectable(tout, s.analysis_all))
                                            .clicked()
                                        {
                                            self.analysis_module.clear();
                                        }
                                        ui.add_space(4.0);
                                        ui.separator();
                                        ui.add_space(4.0);
                                        for (id, titre) in &titres {
                                            let choisi = self.analysis_module == *id;
                                            if ui
                                                .add(egui::Button::selectable(choisi, *titre))
                                                .clicked()
                                            {
                                                self.analysis_module = (*id).to_owned();
                                            }
                                        }
                                    },
                                );
                            });
                    });

                // Le contenu, avec la même marge : collé au trait, un
                // tableau donne l'impression d'être coupé.
                egui::Frame::NONE
                    .inner_margin(egui::Margin {
                        left: 14,
                        right: 6,
                        top: 8,
                        bottom: 4,
                    })
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("analysis_content")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                // Une largeur de lecture, et non toute
                                // la place : sur un écran large, une
                                // table à deux colonnes posait la
                                // seconde à mille pixels de la première,
                                // et l'on suivait la ligne du doigt. Ce
                                // que fait tout volet de détail.
                                ui.set_max_width(
                                    ui.available_width().min(ANALYSIS_CONTENT_MAX),
                                );
                                let choisi = self.analysis_module.clone();
                                let seul = (!choisi.is_empty()).then_some(choisi.as_str());
                                self.re_modules_ui(ui, seul);
                            });
                    });
            });
        self.place_maximize_button(ctx, Self::ANALYSIS_WINDOW, rendue_analysis_window);
        self.show_analysis = open && !fermer_ok;
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
        let mut fermer = false;
        egui::Window::new(s.help_title)
            // Un identifiant explicite : le titre change avec la langue,
            // et la fenêtre perdrait sa position en passant à l'anglais.
            .id(egui::Id::new("instruction_help"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // 460 × 320 était trop petit : une fiche s'y lisait par
            // tranches de six lignes, et la liste des résultats tenait
            // en quatre. Une barre de défilement qui parcourt quatre
            // lignes n'est pas une barre, c'est un aveu.
            .default_size([680.0, 560.0])
            .min_size([460.0, 320.0])
            // En bas à droite, loin du curseur : on lit la fiche en
            // continuant d'écrire, elle ne doit pas couvrir la ligne sur
            // laquelle on vient de demander de l'aide.
            .pivot(egui::Align2::RIGHT_BOTTOM)
            .default_pos(ctx.viewport_rect().max - egui::vec2(24.0, 24.0))
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.help_title) {
                    fermer = true;
                }
                // La fiche prend la place qui reste, la barre d'action
                // garde la sienne en bas : sans ce panneau, le bouton
                // était poussé hors de la fenêtre par une fiche longue.
                egui::Panel::bottom("help_actions")
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        ui.add_space(6.0);
                        if dialog_bar(ui, self.theme, strings(self.lang).dialog_ok, None).is_some()
                        {
                            fermer = true;
                        }
                    });
                self.help_ui(ui);
            });
        self.show_help = open && !fermer;
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
        let mut fermer_ok = false;
        egui::Window::new(s.vectors_title)
            .id(egui::Id::new("vectors_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size([520.0, 620.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.vectors_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "vectors") {
                    fermer_ok = true;
                }
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
        self.show_vectors = open && !fermer_ok;
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
        // Relevée avant le champ, comme partout ailleurs : un `TextEdit`
        // qui traite Entrée la retire de la file.
        let entrée_frappée = ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.horizontal(|ui| {
            // Un vrai champ de recherche : la loupe devant, et la
            // largeur qui suit la fenêtre. Un champ de 200 px collé aux
            // boutons ne se voyait pas comme un champ.
            //
            // La loupe est dessinée, non écrite : l'emoji n'est dans
            // aucune police chargée par défaut et se peindrait en carré
            // vide — ce qu'un test du projet attrape.
            let (marque, _) = ui.allocate_exact_size(
                egui::vec2(16.0, ui.spacing().interact_size.y),
                egui::Sense::hover(),
            );
            chrome::icon_search(
                ui.painter(),
                egui::Rect::from_center_size(marque.center(), egui::vec2(13.0, 13.0)),
                theme.dim,
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.help_query)
                    .hint_text(s.help_search)
                    .font(egui::TextStyle::Monospace)
                    .desired_width((ui.available_width() - 180.0).max(180.0)),
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
            // Classées par pertinence, et sous le nom qui a correspondu.
            // Chercher « pop » rendait quatre fiches dans l'ordre du
            // manuel, celle qu'on voulait nulle part en tête — et
            // affichée sous le nom « push », premier de sa fiche. On
            // cherchait `pop`, on lisait `push`.
            let trouvées = namsdec_doc::search(&self.help_query, lang_code);
            ui.colored_label(theme.dim, format!("{} {}", trouvées.len(), s.help_results));
            // Entrée ouvre la première : c'est celle qu'on voulait dans
            // le cas ordinaire, et cela évite de viser à la souris ce
            // qu'on vient de taper au clavier.
            if entrée_frappée && let Some(première) = trouvées.first() {
                ouvrir = Some(première.name);
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("help_index")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for trouvée in trouvées {
                        let ligne = ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                theme.mnemonic,
                                egui::RichText::new(trouvée.name).monospace().strong(),
                            );
                            ui.label(trouvée.entry.title.get(lang_code));
                            // Les autres noms de la fiche : une fiche
                            // couvre souvent une famille, et le dire
                            // évite de la rouvrir pour chacun.
                            let autres: Vec<&str> = trouvée
                                .entry
                                .names
                                .iter()
                                .copied()
                                .filter(|nom| *nom != trouvée.name)
                                .collect();
                            if !autres.is_empty() {
                                ui.colored_label(
                                    theme.dim,
                                    egui::RichText::new(autres.join(" ")).monospace().small(),
                                );
                            }
                        });
                        if ligne
                            .response
                            .interact(egui::Sense::click())
                            .on_hover_text(s.help_open_sheet)
                            .clicked()
                        {
                            ouvrir = Some(trouvée.name);
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
        let mut à_ouvrir: Option<String> = None;
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

                // La référence complète, pour le jour où la fiche ne
                // suffit pas. Elle tient en un écran — ce qu'on veut en
                // écrivant du code —, mais quand une instruction se
                // comporte autrement qu'on l'espérait, il faut le
                // détail : tous les encodages, tous les modes, toutes
                // les exceptions. C'est le manuel d'Intel, et le
                // recopier ici serait absurde : deux mille pages, déjà
                // en ligne, instruction par instruction.
                if let Some(url) = namsdec_doc::reference_url(entry) {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.help_reference);
                        if ui
                            .link(
                                egui::RichText::new(format!(
                                    "{} — Intel",
                                    entry.names[0].to_uppercase()
                                ))
                                .monospace(),
                            )
                            .on_hover_text(&url)
                            .clicked()
                        {
                            à_ouvrir = Some(url.clone());
                        }
                    });
                }
            });
        if let Some(name) = suivante {
            self.open_help(name);
        }
        if let Some(url) = à_ouvrir {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
        if copié {
            self.status = Status::Note(|s| s.help_example_copied);
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
    /// Le profil : où le programme a passé son temps.
    ///
    /// La bande de chaleur dans la gouttière répond à « par où ça
    /// chauffe » d'un coup d'œil, mais elle ne classe pas : deux rouges
    /// voisins ne disent pas lequel coûte le double de l'autre. Ce
    /// tableau le dit — le nombre de passages, la part du total, et la
    /// ligne elle-même — et y emmène d'un clic. C'est ce qu'un
    /// profileur montre d'un programme, à la seule échelle qui vaille
    /// ici : la ligne de source.
    fn profile_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let passages = self.line_visits();
        if passages.is_empty() {
            ui.colored_label(theme.dim, s.profile_empty);
            return;
        }
        // Le plus chaud d'abord ; à égalité, la ligne la plus haute,
        // pour que deux exécutions de suite donnent le même ordre.
        let mut classement: Vec<(u32, u64)> = passages.iter().map(|(l, c)| (*l, *c)).collect();
        classement.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let total: u64 = classement.iter().map(|(_, c)| *c).sum();
        let plus_chaude = classement.first().map_or(1, |(_, c)| *c).max(1);

        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                theme.dim,
                format!("{} {}", sidebar::group_digits(total), s.profile_total),
            );
            if ui.small_button(s.profile_copy).clicked() {
                ui.ctx().copy_text(self.profile_text(&classement, total));
                self.status = Status::Note(|s| s.profile_copied);
            }
        });
        // Le budget de l'exercice, quand il y en a un : c'est ici qu'on
        // vient quand on le dépasse, et la réponse — quelle ligne
        // reprendre — est juste en dessous.
        if let Some(budget) = self
            .current_exercise
            .as_deref()
            .and_then(|id| namsdec_exercises::find(&self.exercise_packs, id))
            .and_then(|exercice| exercice.par)
        {
            let exécutées = self.emulator.as_ref().map_or(0, |e| e.executed());
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, s.par_label);
                ui.colored_label(
                    if exécutées > budget {
                        theme.warn
                    } else {
                        theme.good
                    },
                    egui::RichText::new(format!("{exécutées} / {budget}")).monospace(),
                );
                if exécutées > budget {
                    ui.colored_label(theme.dim, s.profile_over_budget);
                }
            });
        }
        ui.colored_label(theme.dim, s.profile_hint);
        ui.separator();

        let mut goto = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Une grille, et non des rangées posées à la main : les
                // colonnes d'une rangée tombaient sous les mots d'une
                // autre, et l'en-tête — un seul texte dont les espaces ne
                // mesuraient rien — sous aucune.
                egui::Grid::new("profile_rows")
                    .num_columns(6)
                    .min_col_width(0.0)
                    .striped(true)
                    .show(ui, |ui| {
                        // L'en-tête : sans lui, « 12 » et « 4,2 % » se
                        // lisent, mais « 1 204 » se prend pour une
                        // adresse.
                        ui.label("");
                        // Les titres au même gabarit que les nombres
                        // qu'ils surmontent : dans une chasse fixe, c'est
                        // le remplissage qui aligne, et la grille fait le
                        // reste. Une cellule de grille ne prend que la
                        // largeur de son contenu — un calage à droite n'y
                        // change donc rien.
                        for (titre, largeur) in [
                            (s.line_word, COL_LIGNE),
                            (s.profile_count, COL_PASSAGES),
                            (s.profile_share, COL_PART),
                        ] {
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(caler_à_droite(titre, largeur)).monospace(),
                            );
                        }
                        ui.label("");
                        ui.label("");
                        ui.end_row();

                        for (ligne, compte) in &classement {
                            let part = *compte as f32 / total.max(1) as f32;
                            let teinte = heat_colour(theme, *compte, plus_chaude);
                            // La pastille reprend exactement la teinte de
                            // la gouttière : la même ligne a la même
                            // couleur aux deux endroits, sinon le lien se
                            // perd.
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(8.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 1.0, teinte);

                            // Les trois nombres, calés à droite de leur
                            // colonne : c'est ainsi que des chiffres se
                            // comparent d'une ligne à l'autre.
                            for (texte, largeur, couleur) in [
                                (format!("{ligne}"), COL_LIGNE, theme.dim),
                                (sidebar::group_digits(*compte), COL_PASSAGES, theme.text),
                                (pourcentage(part, self.lang), COL_PART, theme.dim),
                            ] {
                                ui.colored_label(
                                    couleur,
                                    egui::RichText::new(caler_à_droite(&texte, largeur))
                                        .monospace(),
                                );
                            }

                            // Une barre proportionnelle : les pourcentages
                            // se comparent bien plus vite en longueur
                            // qu'en chiffres.
                            let (barre, _) = ui
                                .allocate_exact_size(egui::vec2(60.0, 10.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    barre.min,
                                    egui::vec2(barre.width() * part, barre.height()),
                                ),
                                1.0,
                                teinte,
                            );

                            // Le bloc et la ligne elle-même : sans eux, le
                            // classement renvoie à un numéro qu'il faut
                            // aller chercher.
                            ui.horizontal(|ui| {
                                if let Some(étiquette) = enclosing_label(&self.source, *ligne) {
                                    ui.colored_label(
                                        theme.label,
                                        egui::RichText::new(format!("{étiquette}:")).monospace(),
                                    );
                                }
                                let réponse = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(source_line(&self.source, *ligne))
                                            .monospace()
                                            .color(theme.text),
                                    )
                                    .sense(egui::Sense::click())
                                    .truncate(),
                                );
                                if réponse.on_hover_text(s.profile_goto).clicked() {
                                    goto = Some(*ligne);
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
        if let Some(ligne) = goto {
            self.goto_line = Some(ligne);
        }
    }

    /// Le profil sous forme de texte, pour le presse-papiers : la même
    /// information, collable dans un message ou un rapport.
    fn profile_text(&self, classement: &[(u32, u64)], total: u64) -> String {
        let s = strings(self.lang);
        let mut out = format!("{} {}\n", sidebar::group_digits(total), s.profile_total);
        for (ligne, compte) in classement {
            let part = pourcentage(*compte as f32 / total.max(1) as f32, self.lang);
            let bloc = enclosing_label(&self.source, *ligne)
                .map(|nom| format!("{nom}: "))
                .unwrap_or_default();
            out.push_str(&format!(
                "{ligne:>5}  {:>9}  {part:>7}  {bloc}{}\n",
                sidebar::group_digits(*compte),
                source_line(&self.source, *ligne)
            ));
        }
        out
    }

    fn diagnostics_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (lang, theme) = (self.lang, self.theme);
        let mut goto = None;
        // Les deux commandes réellement lancées. « Et si je veux le
        // faire sans namsdec ? » est la bonne question, et un atelier
        // d'apprentissage y répond au lieu de garder son tour de main
        // pour lui.
        let fichier = self
            .current_file
            .as_ref()
            .and_then(|path| path.file_stem())
            .map(|nom| nom.to_string_lossy().into_owned())
            .unwrap_or_else(|| "programme".to_owned());
        let commandes = namsdec_asm::build_commands(&format!("{fichier}.asm"), &fichier, BASE_ADDR);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.assemble_commands);
            if ui
                .small_button(s.assemble_commands_copy)
                .on_hover_text(s.assemble_commands_hint)
                .clicked()
            {
                ui.ctx().copy_text(commandes.join("\n"));
                self.status = Status::Note(|s| s.assemble_commands_copied);
            }
        });
        for commande in &commandes {
            ui.colored_label(theme.dim, egui::RichText::new(commande).monospace());
        }
        ui.separator();
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
                    // La phrase est reformée ici quand le diagnostic
                    // porte son motif : c'est ce qui la fait suivre la
                    // langue du moment. Sans cela, ceux qui étaient déjà
                    // dans la liste gardaient celle où ils étaient nés,
                    // et basculer de langue laissait du français dans une
                    // interface anglaise — les retraduire aurait demandé
                    // de réassembler.
                    let phrase = match &d.kind {
                        Some(kind) => kind.message(lang == Lang::Fr),
                        None => d.message.clone(),
                    };
                    // Un diagnostic sans ligne — ceux de l'éditeur de
                    // liens, qui parlent du programme entier — ne
                    // s'annonce pas par « ligne 0 ».
                    let text = match (d.line, lang) {
                        (0, _) => phrase,
                        (line, Lang::Fr) => format!("ligne {line} — {phrase}"),
                        (line, _) => format!("line {line} — {phrase}"),
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
        // le programme.
        if self.console_shows_input() {
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
            // Ce que la zone a vraiment pris : c'est cela qu'il faudra
            // réserver à l'image suivante.
            self.console_input_height =
                egui::containers::panel::PanelState::load(ui.ctx(), egui::Id::new("console_input"))
                    .map(|état| état.outer_rect.height())
                    .filter(|hauteur| *hauteur > 1.0);
        } else {
            self.console_input_height = None;
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
                            // Dire *pourquoi* c'est vide. « Le programme
                            // n'a rien écrit » devant un programme qui
                            // n'écrit jamais rien laisse chercher une
                            // panne qui n'existe pas ; devant un
                            // programme qu'on n'a pas lancé, c'est
                            // simplement faux.
                            let exécuté = self.emulator.as_ref().is_some_and(|e| e.executed() > 0);
                            ui.colored_label(
                                self.theme.dim,
                                match (self.program_writes, exécuté) {
                                    (false, _) => s.output_never,
                                    (true, false) => s.output_not_yet,
                                    (true, true) => s.output_empty,
                                },
                            );
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
    /// La boîte de saisie : là où l'on répond au programme.
    ///
    /// La ligne de la console suffit quand on la voit. Elle ne suffit pas
    /// quand une fenêtre d'analyse couvre l'écran, quand la console est
    /// repliée, ou simplement quand on ne sait pas encore qu'elle existe
    /// — et l'on reste alors devant un programme qui ne répond plus, sans
    /// savoir qu'il attend *nous*.
    ///
    /// La boîte s'ouvre donc d'elle-même au moment où le programme se met
    /// à attendre, au premier plan, le curseur déjà dans le champ. Elle
    /// montre **ce que le programme vient d'écrire** : neuf fois sur dix
    /// c'est l'invite — « Enter the password: » —, et c'est la seule
    /// chose qui dise ce qu'on attend de nous.
    fn input_dialog_ui(&mut self, ctx: &egui::Context) {
        // Le passage à l'attente ouvre la boîte ; l'attente seule ne la
        // rouvre pas, sans quoi la refermer serait impossible.
        let attend = matches!(self.status, Status::WaitingForInput);
        if attend && !self.was_waiting_for_input {
            self.show_input_dialog = true;
            self.input_dialog_focus = true;
        }
        if !attend {
            self.show_input_dialog = false;
        }
        self.was_waiting_for_input = attend;
        if !self.show_input_dialog {
            return;
        }

        let s = strings(self.lang);
        let theme = self.theme;
        // Les dernières lignes écrites : l'invite, et ce qui la précède.
        let sortie = self
            .emulator
            .as_ref()
            .map(|e| String::from_utf8_lossy(e.output()).into_owned())
            .unwrap_or_default();
        let invite = {
            let mut lignes: Vec<&str> = sortie.lines().rev().take(4).collect();
            lignes.reverse();
            lignes.join("\n")
        };

        let mut envoyer = false;
        let mut fin = false;
        let mut fermer = false;
        egui::Window::new(s.input_dialog_title)
            .id(egui::Id::new(INPUT_DIALOG))
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .current_pos(ctx.viewport_rect().center())
            .default_width(480.0)
            .min_width(480.0)
            .show(ctx, |ui| {
                if chrome::window_head(ui, theme, s, s.input_dialog_title) {
                    fermer = true;
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(10.0, ui.text_style_height(&egui::TextStyle::Body)),
                        egui::Sense::hover(),
                    );
                    ui.painter().circle_filled(rect.center(), 4.0, theme.accent);
                    ui.colored_label(
                        theme.accent,
                        egui::RichText::new(s.console_waiting).strong(),
                    );
                });

                ui.add_space(8.0);
                ui.colored_label(theme.dim, s.input_dialog_asked);
                ui.add_space(2.0);
                // Ce que le programme vient d'écrire, sur le fond d'un
                // terminal : c'est sa voix, elle ne doit pas se confondre
                // avec celle de l'application.
                egui::Frame::new()
                    .fill(theme.bg)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .corner_radius(4)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        if invite.trim().is_empty() {
                            ui.colored_label(theme.dim, s.input_dialog_nothing_asked);
                        } else {
                            ui.label(egui::RichText::new(&invite).monospace());
                        }
                    });

                ui.add_space(10.0);
                // Les deux touches relevées avant le champ : un
                // `TextEdit` qui traite Entrée la retire de la file, si
                // bien que la guetter après lui ne verrait plus rien.
                let entrée_frappée = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let échap_frappé = ui.input(|i| i.key_pressed(egui::Key::Escape));
                let champ = ui.add(
                    egui::TextEdit::singleline(&mut self.input_dialog_text)
                        .hint_text(s.console_input_hint)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width()),
                );
                // Le curseur y va à l'ouverture, une fois : la boîte
                // s'est ouverte parce qu'on nous demande quelque chose,
                // et la seule chose à faire est d'écrire. Le redemander
                // à chaque image empêcherait Échap de rendre le clavier.
                if self.input_dialog_focus {
                    champ.request_focus();
                    self.input_dialog_focus = false;
                }
                envoyer |= entrée_frappée && (champ.has_focus() || champ.lost_focus());
                fermer |= échap_frappé;

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    // « Fin d'entrée » à gauche, loin du bouton qu'on
                    // vise : fermer l'entrée d'un programme est sans
                    // retour pour lui, et ce n'est pas ce qu'on venait
                    // faire.
                    if ui
                        .button(s.console_eof)
                        .on_hover_text(s.console_eof_hint)
                        .clicked()
                    {
                        fin = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let envoi =
                            egui::Button::new(egui::RichText::new(s.console_send).color(theme.bg))
                                .fill(theme.accent)
                                .min_size(egui::vec2(110.0, 0.0));
                        envoyer |= ui.add(envoi).clicked();
                        if ui
                            .add(
                                egui::Button::new(s.input_dialog_later)
                                    .stroke(egui::Stroke::new(1.0, theme.dim))
                                    .min_size(egui::vec2(110.0, 0.0)),
                            )
                            .on_hover_text(s.input_dialog_later_hint)
                            .clicked()
                        {
                            fermer = true;
                        }
                    });
                });
            });

        if envoyer {
            let ligne = std::mem::take(&mut self.input_dialog_text);
            self.send_input(Some(ligne));
            self.show_input_dialog = false;
        }
        if fin {
            self.send_input(None);
            self.show_input_dialog = false;
        }
        if fermer {
            self.show_input_dialog = false;
        }
    }

    /// La console montre-t-elle sa ligne de saisie ?
    ///
    /// Dès qu'un programme est assemblé, et sans autre condition.
    ///
    /// Elle n'apparaissait avant que si namsdec avait *repéré* un `read`
    /// dans le code, au nom d'une économie : « un programme sans `read`
    /// n'attendra jamais rien, et lui tendre un champ de saisie, c'est
    /// trois lignes qui ne servent à rien ». Le raisonnement tombe
    /// précisément dans le cas qui compte le plus — celui de quelqu'un
    /// qui écrit son programme, a posé ses chaînes, et veut préparer ce
    /// qu'il lira avant d'avoir tapé le `read`. Il n'y avait alors nulle
    /// part où taper, et rien à l'écran n'expliquait pourquoi.
    ///
    /// Plus généralement : un champ qui n'apparaît qu'à une condition que
    /// l'utilisateur ne peut pas voir n'est pas une économie, c'est une
    /// disparition. Trois lignes toujours là se comprennent ; trois
    /// lignes qui vont et viennent se cherchent.
    fn console_shows_input(&self) -> bool {
        self.emulator.is_some()
    }

    fn console_input_ui(&mut self, ui: &mut egui::Ui, pending: usize) {
        let s = strings(self.lang);
        let waiting = matches!(self.status, Status::WaitingForInput);

        ui.separator();

        // Quand le programme attend, la ligne de saisie se distingue du
        // reste : elle est en bas d'un panneau lui-même en bas, sur un
        // fond de terminal, et l'on ne la trouvait pas — on cherchait
        // « où taper » en regardant la sortie.
        // Quand le programme attend, la zone doit se voir sans qu'on la
        // cherche : c'est le seul moment où l'application demande
        // quelque chose, et le manquer, c'est croire qu'elle a planté.
        // Un liseré d'un pixel et un fond à dix pour cent ne suffisaient
        // pas — sur un panneau déjà sombre, cela passait pour une nuance.
        let cadre = if waiting {
            egui::Frame::new()
                .fill(self.theme.accent.gamma_multiply(0.18))
                .stroke(egui::Stroke::new(2.0, self.theme.accent))
                .corner_radius(4)
                .inner_margin(egui::Margin::symmetric(8, 6))
        } else {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 2))
        };

        let mut send = false;
        let mut relancer = false;
        cadre.show(ui, |ui| {
        if waiting {
            ui.horizontal(|ui| {
                // Une pastille pleine devant la phrase : l'oeil s'y pose
                // avant de lire, et c'est ce qu'on veut d'un état qui ne
                // dure que le temps d'une réponse.
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(10.0, ui.text_style_height(&egui::TextStyle::Body)),
                    egui::Sense::hover(),
                );
                ui.painter()
                    .circle_filled(rect.center(), 4.0, self.theme.accent);
                ui.colored_label(
                    self.theme.accent,
                    egui::RichText::new(s.console_waiting).strong(),
                );
                ui.colored_label(self.theme.dim, s.console_waiting_how);
            });
        }

        // Les deux lignes — les arguments, puis l'entrée — sont bâties
        // pareil : un libellé de largeur commune, un champ qui prend ce
        // qui reste, des boutons calés à droite. Sans cette largeur
        // commune les deux champs commençaient à des abscisses
        // différentes ; sans le calage à droite, « Recharger » sortait
        // du cadre.
        let libellés = [s.console_args, s.console_input_label];
        let colonne = libellés
            .iter()
            .map(|texte| {
                ui.painter()
                    .layout_no_wrap(
                        (*texte).to_owned(),
                        egui::TextStyle::Body.resolve(ui.style()),
                        egui::Color32::PLACEHOLDER,
                    )
                    .rect
                    .width()
            })
            .fold(0.0_f32, f32::max);

        // Les arguments : l'autre chose qu'un programme attend, et la
        // première pour un crackme, qui lit souvent son mot de passe
        // dans `argv[1]` plutôt que sur l'entrée standard. Ils ne se
        // tapent pas pendant qu'il tourne — ils sont posés sur la pile
        // avant la première instruction —, d'où le bouton qui recharge.
        if self
            .assemble_state
            .as_ref()
            .is_some_and(|état| état.from_binary.is_some())
        {
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(colonne, ui.spacing().interact_size.y),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.colored_label(self.theme.dim, s.console_args);
                    },
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let appliquer = framed_button(ui, self.theme, s.console_args_apply)
                        .on_hover_text(s.console_args_apply_hint)
                        .clicked();
                    let réponse = ui.add(
                        egui::TextEdit::singleline(&mut self.program_args)
                            .hint_text(s.console_args_hint)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width()),
                    );
                    let entrée =
                        réponse.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    relancer = appliquer || entrée;
                });
            });
            // L'environnement, replié : on s'en sert moins souvent que
            // des arguments, et une ligne de plus toujours ouverte
            // repousserait la saisie vers le bas.
            let ouvert = egui::CollapsingHeader::new(s.console_env)
                .id_salt("console_env")
                .open(Some(self.console_env_open))
                .show(ui, |ui| {
                    ui.weak(s.console_env_hint);
                    let réponse = ui.add(
                        egui::TextEdit::multiline(&mut self.program_env)
                            .hint_text("SECRET=sesame")
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    );
                    if réponse.lost_focus() {
                        relancer = true;
                    }
                });
            if ouvert.header_response.clicked() {
                self.console_env_open = !self.console_env_open;
            }
            ui.add_space(2.0);
        }
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(colonne, ui.spacing().interact_size.y),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    // En accent et en gras pendant l'attente : c'est la
                    // ligne où il faut taper, et rien d'autre à l'écran
                    // ne le dit.
                    if waiting {
                        ui.colored_label(
                            self.theme.accent,
                            egui::RichText::new(s.console_input_label).strong(),
                        );
                    } else {
                        ui.colored_label(self.theme.dim, s.console_input_label);
                    }
                },
            );
            // Les boutons sont posés en premier, calés à droite, et le
            // champ prend ce qui reste. Deviner sa largeur (« la place
            // disponible moins 120 px ») la surestimait de quelques
            // pixels : la ligne débordait du panneau latéral, et ce
            // débordement était repris comme sa nouvelle largeur à la
            // frame suivante. L'onglet Console élargissait donc le
            // panneau sans fin, jusqu'à ce qu'il occupe la fenêtre.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Les deux touches relevées **avant** le champ, et sans
                // les consommer : un `TextEdit` qui traite Entrée la
                // retire de la file, si bien que la guetter après lui ne
                // voyait plus rien. Le champ gardait le texte, le
                // programme continuait d'attendre, et il fallait cliquer
                // « Envoyer » — ce qui est exactement ce qu'on voulait
                // éviter.
                let entrée_frappée = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let échap_frappé = ui.input(|i| i.key_pressed(egui::Key::Escape));
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
                // Échap le lui reprend — sans cette porte, le champ
                // gardait toutes les touches tant que durait l'attente,
                // et l'éditeur ne répondait plus : « i » s'écrivait ici
                // au lieu de passer en Insertion.
                //
                // `lost_focus` et non `has_focus` : egui rend déjà le
                // focus du champ sur Échap. Le guetter ici sur le focus
                // encore détenu ne voyait donc rien, et la reprise plus
                // bas le récupérait dans la même frame — la touche
                // n'avait aucun effet visible.
                if response.lost_focus() && échap_frappé {
                    self.console_input_released = true;
                    // Personne ne prend le focus derrière : c'est
                    // l'absence de champ focalisé qui réarme les
                    // commandes de l'éditeur.
                    self.editor_mode = EditorMode::Normal;
                } else if response.has_focus() {
                    // Revenu dedans de son plein gré : l'attente suivante
                    // pourra de nouveau l'y amener.
                    self.console_input_released = false;
                } else if waiting && !self.console_input_released && !self.show_input_dialog {
                    // Pas tant que la boîte est ouverte : c'est elle qui
                    // demande, et deux champs qui réclament le clavier
                    // dans la même image se l'arrachent.
                    response.request_focus();
                }
                // Le champ a le focus et l'on vient de frapper Entrée :
                // c'est un envoi. On ne demande pas `lost_focus`, qui
                // dépend de ce qu'egui a décidé de faire du focus, ni
                // `key_pressed` après coup, qui ne voit plus rien.
                send |= entrée_frappée && (response.has_focus() || response.lost_focus());
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
        } else if !waiting && !self.program_reads {
            // Le champ est là, mais rien ne le lira tel que le programme
            // est écrit. Le dire plutôt que de faire disparaître le
            // champ : celui qui prépare son entrée avant d'avoir tapé
            // son `read` doit pouvoir la taper, et comprendre pourquoi
            // elle attend.
            ui.colored_label(self.theme.dim, s.console_no_read);
        }

        });

        // Reposer les arguments demande de refaire la pile : c'est un
        // rechargement, et le dire vaut mieux que de le faire en
        // silence sous un programme à demi exécuté.
        if relancer {
            self.do_reset();
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

        // Les modules d'analyse, un par ligne plutôt qu'en énumération
        // dans la carte : à neuf titres séparés par des virgules, on ne
        // lisait plus lequel était là ni ce qu'il fait.
        if !titles.is_empty() {
            ui.add_space(2.0);
            // Repliés : neuf lignes de plus allongeaient la page d'autant,
            // et l'on vient d'abord y voir l'état des modules, non la
            // liste de ce que l'analyse sait faire.
            egui::CollapsingHeader::new(format!("{} ({})", s.modules_re_detail, titles.len()))
                .id_salt("modules_re_detail")
                .show(ui, |ui| {
                    for titre in &titles {
                        ui.horizontal(|ui| {
                            status_dot(ui, theme.good);
                            ui.label(*titre);
                        });
                    }
                });
            ui.add_space(6.0);
        }

        // Ce que namsdec porte en propre, et qui n'avait aucune carte :
        // l'explorateur, la documentation, les exercices, les thèmes et
        // la recherche de mise à jour. L'onglet annonçait les modules de
        // namsdec et en montrait la moitié.
        module_card(
            ui,
            "namsdec-explorer",
            namsdec_explorer::VERSION,
            self.explorer.root.is_some(),
            theme.accent,
            &match self.explorer.root.as_ref() {
                Some(racine) => format!("{} {}", s.modules_explorer_root, racine.display()),
                None => s.modules_explorer_none.to_string(),
            },
        );

        module_card(
            ui,
            "namsdec-doc",
            namsdec_doc::VERSION,
            true,
            theme.good,
            &if fr {
                format!("{} fiches d'instructions", namsdec_doc::count())
            } else {
                format!("{} instruction entries", namsdec_doc::count())
            },
        );

        let exercices = self
            .exercise_packs
            .iter()
            .map(|pack| pack.exercises.len())
            .sum::<usize>();
        module_card(
            ui,
            "namsdec-exercises",
            namsdec_exercises::VERSION,
            self.exercises,
            theme.accent,
            &if self.exercises {
                if fr {
                    format!(
                        "{} série(s), {exercices} exercice(s)",
                        self.exercise_packs.len()
                    )
                } else {
                    format!(
                        "{} pack(s), {exercices} exercise(s)",
                        self.exercise_packs.len()
                    )
                }
            } else {
                s.modules_inactive.to_string()
            },
        );

        let thèmes = namsdec_theme::all().len();
        module_card(
            ui,
            "namsdec-theme",
            namsdec_theme::VERSION,
            true,
            theme.good,
            &if fr {
                format!(
                    "{thèmes} thème(s), dont {} venu(s) de fichiers",
                    self.theme_files.len()
                )
            } else {
                format!(
                    "{thèmes} theme(s), {} of them from files",
                    self.theme_files.len()
                )
            },
        );

        let (maj_ok, maj_couleur, maj_texte) = match &self.update_result {
            Some(Ok(verdict)) => (true, theme.good, format!("{verdict:?}")),
            Some(Err(raison)) => (false, theme.bad, raison.clone()),
            None => (false, theme.dim, s.modules_update_idle.to_string()),
        };
        module_card(
            ui,
            "namsdec-update",
            namsdec_update::VERSION,
            maj_ok,
            maj_couleur,
            &maj_texte,
        );

        // Et ce qui vient de fichiers : thèmes et outils déposés dans le
        // dossier des greffons. Ils ne figuraient que dans l'onglet
        // Plugins, alors qu'ils sont chargés au même titre que le reste.
        ui.add_space(4.0);
        ui.strong(s.modules_plugins);
        ui.weak(s.modules_plugins_hint);
        ui.add_space(2.0);
        let mut venus_de_fichiers = false;
        for (id, chemin) in &self.theme_files {
            venus_de_fichiers = true;
            ui.horizontal(|ui| {
                status_dot(ui, theme.accent);
                ui.label(id);
                ui.weak(chemin.display().to_string());
            });
        }
        for outil in &self.decompilers {
            venus_de_fichiers = true;
            ui.horizontal(|ui| {
                status_dot(ui, theme.accent);
                ui.label(&outil.name);
                ui.weak(&outil.command);
            });
        }
        if !venus_de_fichiers {
            ui.weak(s.modules_none_loaded);
        }
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
/// Ce qu'il faut faire après la frappe d'une parenthèse ou d'un
/// guillemet.
///
/// Une paire s'ouvre et se ferme : taper `(` et devoir taper `)` est le
/// genre de travail qu'une machine fait mieux. Mais mal fait, c'est pire
/// que rien — un `'` posé au milieu d'un commentaire français, et l'on
/// passe son temps à effacer ce qu'on n'a pas demandé.
///
/// D'où trois règles, et pas une de plus :
///
/// - on ne ferme pas devant un mot : `(` tapé avant `rax` ouvrirait un
///   `()` autour de rien ;
/// - on ne ferme pas une apostrophe qui suit une lettre — « l'adresse »
///   n'appelle aucune fermeture ;
/// - taper le fermant qui est déjà là ne l'écrit pas deux fois, il le
///   traverse.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum PairEdit {
    /// Poser ce caractère derrière le curseur, sans l'y déplacer.
    Close(char),
    /// Le fermant était déjà là : retirer celui qu'on vient de taper et
    /// laisser le curseur devant lui.
    StepOver,
    /// Ne rien faire.
    None,
}

/// Le fermant qui va avec cet ouvrant.
fn closing_of(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

/// Ce qu'on fait du caractère qui vient d'être tapé, `at` étant la
/// position du curseur **après** la frappe, en octets.
fn pair_edit(source: &str, at: usize) -> PairEdit {
    if at == 0 || at > source.len() || !source.is_char_boundary(at) {
        return PairEdit::None;
    }
    let Some(tapé) = source[..at].chars().next_back() else {
        return PairEdit::None;
    };
    let suivant = source[at..].chars().next();
    let précédent = {
        let avant = at - tapé.len_utf8();
        source[..avant].chars().next_back()
    };

    // Traverser un fermant déjà posé. Vaut pour les guillemets, qui sont
    // leur propre fermant : `"abc|"` puis `"` doit sortir de la chaîne,
    // non en ouvrir une seconde.
    let ferme = matches!(tapé, ')' | ']' | '}' | '"' | '\'');
    if ferme && suivant == Some(tapé) {
        return PairEdit::StepOver;
    }

    let Some(fermant) = closing_of(tapé) else {
        return PairEdit::None;
    };
    // Une apostrophe après une lettre est une apostrophe, pas une
    // chaîne : « l'adresse », « d'où ». C'est le cas qui rend
    // insupportables les éditeurs qui ferment tout.
    if matches!(tapé, '"' | '\'') && précédent.is_some_and(is_word_char) {
        return PairEdit::None;
    }
    // Ni devant un mot : on est en train d'écrire au milieu de quelque
    // chose, et la fermeture tomberait au mauvais endroit.
    if suivant.is_some_and(is_word_char) {
        return PairEdit::None;
    }
    PairEdit::Close(fermant)
}

/// Un effacement doit-il emporter la paire vide qu'il ouvre ?
///
/// `(|)` puis retour arrière : effacer la seule parenthèse ouvrante
/// laisserait la fermante orpheline — celle-là même qu'on n'avait pas
/// tapée.
fn backspace_clears_pair(source: &str, at: usize) -> bool {
    if at == 0 || at >= source.len() || !source.is_char_boundary(at) {
        return false;
    }
    let (Some(avant), Some(après)) = (
        source[..at].chars().next_back(),
        source[at..].chars().next(),
    ) else {
        return false;
    };
    closing_of(avant) == Some(après)
}

/// Place le curseur de l'éditeur à cet indice d'octet.
///
/// Les positions d'egui se comptent en *caractères*, celles d'une chaîne
/// Rust en octets : la conversion est ce qui sépare un curseur juste
/// d'un curseur qui dérive d'un cran à chaque accent.
fn set_cursor_byte(ctx: &egui::Context, source: &str, at: usize) {
    let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) else {
        return;
    };
    let cible = byte_to_char(source, at);
    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
        egui::text::CCursor::new(cible),
    )));
    state.store(ctx, editor_id());
}

/// Un caractère qui fait partie d'un mot, pour les règles ci-dessus.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

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

/// La date de dernière modification d'un fichier, si on peut la lire.
fn file_mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// Les bornes du grossissement du déroulement.
///
/// Au-delà, les boîtes deviennent des traits ; en deçà, on ne lit plus
/// rien. Entre les deux, on voit soit la forme, soit les instructions —
/// et c'est exactement le choix qu'on veut laisser.
const FLOW_ZOOM_MIN: f32 = 0.4;
const FLOW_ZOOM_MAX: f32 = 2.0;

/// La couleur d'une flèche, selon ce qu'elle veut dire.
fn flow_edge_colour(theme: &'static Theme, kind: namsdec_re::cfg::Kind) -> egui::Color32 {
    use namsdec_re::cfg::Kind;
    match kind {
        // Le vert et le rouge d'une condition, comme partout ailleurs
        // dans namsdec : ce qui se fait, ce qui ne se fait pas.
        Kind::Taken => theme.good,
        Kind::NotTaken => theme.bad,
        Kind::Call => theme.accent,
        Kind::Next => theme.dim,
    }
}

/// La légende des flèches : quatre traits et leur sens.
fn flow_legend_ui(ui: &mut egui::Ui, theme: &'static Theme, s: &Strings) {
    use namsdec_re::cfg::Kind;
    ui.horizontal_wrapped(|ui| {
        for (kind, texte) in [
            (Kind::Taken, s.flow_legend_taken),
            (Kind::NotTaken, s.flow_legend_not_taken),
            (Kind::Call, s.flow_legend_call),
        ] {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 10.0), egui::Sense::hover());
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left(), rect.center().y),
                    egui::pos2(rect.right(), rect.center().y),
                ],
                egui::Stroke::new(2.0, flow_edge_colour(theme, kind)),
            );
            ui.colored_label(theme.dim, texte);
            ui.add_space(6.0);
        }
        ui.colored_label(theme.dim, format!("? {}", s.flow_legend_indirect));
    });
}

/// Ce que prend l'en-tête d'un bloc : « 0x0001d491  ? ».
const ENTÊTE_COLONNES: usize = 16;

/// La largeur qu'il faut aux boîtes pour que leur texte y tienne.
///
/// Une largeur commune à tout le graphe, et non une par boîte : les
/// colonnes restent alignées, ce qui est la moitié de ce qu'un tel dessin
/// sert à montrer. Bornée des deux côtés — en deçà on ne lit plus rien,
/// au-delà une seule instruction bavarde étalerait le graphe entier.
fn flow_box_width(
    ui: &egui::Ui,
    cfg: &namsdec_re::cfg::Cfg,
    s: &Strings,
    police: &egui::FontId,
    zoom: f32,
) -> f32 {
    const LIGNES_MAX: usize = 8;
    let plus_longue = cfg
        .blocks
        .iter()
        .flat_map(|bloc| bloc.instructions.iter().take(LIGNES_MAX))
        .map(|texte| texte.chars().count())
        .max()
        .unwrap_or(24)
        // L'en-tête doit tenir lui aussi — et il porte, sur le bloc où
        // le programme en est, « le programme est ici » à sa droite. Les
        // deux étaient peints sans se demander la place : sur une boîte
        // étroite, le libellé venait recouvrir l'adresse.
        .max(ENTÊTE_COLONNES + s.flow_here.chars().count() + 3);
    let glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(police, ' ')).max(1.0);
    (plus_longue as f32 * glyphe + 12.0 * zoom).clamp(220.0 * zoom, 560.0 * zoom)
}

/// La place que prendrait le graphe entier à ce zoom.
///
/// Sert au bouton « Tout voir », qui en déduit le facteur : on mesure
/// avant de dessiner plutôt que de dessiner pour voir si ça dépasse.
fn flow_extent(
    ui: &egui::Ui,
    cfg: &namsdec_re::cfg::Cfg,
    s: &Strings,
    zoom: f32,
) -> egui::Vec2 {
    const LIGNES_MAX: usize = 8;
    let niveaux = cfg.layers();
    let profondeur = niveaux.iter().max().copied().unwrap_or(0) + 1;
    let mut par_rangée: Vec<Vec<usize>> = vec![Vec::new(); profondeur];
    for (index, niveau) in niveaux.iter().enumerate() {
        par_rangée[*niveau].push(index);
    }
    let police = egui::FontId::monospace(11.0 * zoom);
    let hauteur_ligne = ui.ctx().fonts_mut(|f| f.row_height(&police));
    let large = flow_box_width(ui, cfg, s, &police, zoom);
    let marge = 18.0 * zoom;

    let mut largeur = 0.0f32;
    let mut hauteur = 0.0f32;
    for rangée in &par_rangée {
        let mut x = 0.0f32;
        let mut haut = 0.0f32;
        for index in rangée {
            let lignes = cfg.blocks[*index].instructions.len().min(LIGNES_MAX) + 1;
            haut = haut.max(lignes as f32 * hauteur_ligne + 10.0 * zoom);
            x += large + marge;
        }
        largeur = largeur.max(x);
        hauteur += haut + marge * 2.0;
    }
    egui::vec2(largeur.max(100.0), hauteur.max(100.0))
}

/// Le texte, coupé à la largeur de la boîte, avec de quoi le dire.
///
/// Couper vaut mieux que laisser déborder : une instruction trop longue
/// se posait sur la boîte voisine, et les deux devenaient illisibles.
fn flow_ellipsis(texte: &str, colonnes: usize) -> String {
    if texte.chars().count() <= colonnes {
        return texte.to_owned();
    }
    let gardé: String = texte.chars().take(colonnes.saturating_sub(1)).collect();
    format!("{gardé}…")
}

/// Le dessin du graphe.
///
/// Rend la ligne où aller si l'on a cliqué un bloc, et de combien la
/// main a tiré le graphe : c'est la fenêtre qui tient la position de la
/// vue, puisque les barres de défilement et les boutons la déplacent
/// aussi.
///
/// La mise en page tient en deux règles : un bloc se range au niveau de
/// son plus court chemin depuis l'entrée, et les blocs d'un même niveau
/// s'alignent sur une rangée. Cela ne donne pas le plus joli graphe du
/// monde — les vrais outils y mettent des heures de calcul —, mais cela
/// donne le bon : l'entrée en haut, le sens de lecture vers le bas, et
/// les boucles qui remontent, visiblement.
#[allow(clippy::too_many_arguments)]
fn flow_graph_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    cfg: &namsdec_re::cfg::Cfg,
    ici: Option<usize>,
    lignes: &BTreeMap<u64, u32>,
    zoom: f32,
    route: &std::collections::BTreeSet<usize>,
) -> (Option<u32>, egui::Vec2) {
    use namsdec_re::cfg::Exit;

    let niveaux = cfg.layers();
    let profondeur = niveaux.iter().max().copied().unwrap_or(0) + 1;
    // Combien de blocs par rangée, et le rang de chacun dans la sienne.
    let mut par_rangée: Vec<Vec<usize>> = vec![Vec::new(); profondeur];
    for (index, niveau) in niveaux.iter().enumerate() {
        par_rangée[*niveau].push(index);
    }

    let police = egui::FontId::monospace(11.0 * zoom);
    let hauteur_ligne = ui.ctx().fonts_mut(|f| f.row_height(&police));
    // La largeur d'une boîte suit ce qu'elle contient. Fixée à deux cent
    // soixante pixels, elle laissait « jne near 000000000001D903h » sortir
    // de son cadre et se poser sur la boîte voisine — sur un vrai binaire,
    // le dessin devenait illisible avant même d'être grand.
    let large = flow_box_width(ui, cfg, s, &police, zoom);
    let marge = 18.0 * zoom;
    let glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(&police, ' ')).max(1.0);
    // Ce qui tient dans une boîte, marges déduites.
    let colonnes = (((large - 12.0 * zoom) / glyphe).floor() as usize).max(8);

    // La hauteur d'un bloc dépend de ce qu'il contient : on borne à
    // huit lignes, au-delà le dessin ne montrerait plus la forme.
    const LIGNES_MAX: usize = 8;
    let hauteur_de = |index: usize| {
        let combien = cfg.blocks[index].instructions.len();
        // L'en-tête, les instructions montrées, et la ligne « … n » qui
        // dit ce qu'on ne montre pas. Cette dernière manquait au compte :
        // elle se peignait sous le cadre, à côté du bloc suivant.
        let lignes = 1 + combien.min(LIGNES_MAX) + usize::from(combien > LIGNES_MAX);
        lignes as f32 * hauteur_ligne + 10.0 * zoom
    };

    // Poser les boîtes, rangée par rangée.
    let mut rects: Vec<egui::Rect> = vec![egui::Rect::NOTHING; cfg.blocks.len()];
    let mut y = 0.0f32;
    let mut largeur_totale = 0.0f32;
    for rangée in &par_rangée {
        let mut x = 0.0f32;
        let mut haut = 0.0f32;
        for index in rangée {
            let h = hauteur_de(*index);
            rects[*index] = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(large, h));
            x += large + marge;
            haut = haut.max(h);
        }
        largeur_totale = largeur_totale.max(x);
        y += haut + marge * 2.0;
    }

    let (réponse, painter) = ui.allocate_painter(
        egui::vec2(largeur_totale.max(100.0), y.max(100.0)),
        egui::Sense::click_and_drag(),
    );
    // De combien la main a tiré le graphe pendant cette frame.
    let tiré = réponse.drag_delta();
    let origine = réponse.rect.min.to_vec2();
    let décalé = |rect: egui::Rect| rect.translate(origine);

    // Les flèches d'abord : elles passent sous les boîtes.
    for edge in &cfg.edges {
        let (de, vers) = (décalé(rects[edge.from]), décalé(rects[edge.to]));
        let couleur = flow_edge_colour(theme, edge.kind);
        let remonte = rects[edge.to].top() <= rects[edge.from].top();
        let départ = egui::pos2(de.center().x, de.bottom());
        let arrivée = egui::pos2(vers.center().x, vers.top());
        if remonte {
            // Une boucle : on la fait passer par le côté, sinon elle
            // traverse tout ce qui la sépare de sa cible.
            let côté = de.right() + marge * 0.6;
            let retour = egui::pos2(vers.center().x, vers.bottom());
            for segment in [
                [
                    egui::pos2(de.right(), de.center().y),
                    egui::pos2(côté, de.center().y),
                ],
                [egui::pos2(côté, de.center().y), egui::pos2(côté, retour.y)],
                [egui::pos2(côté, retour.y), retour],
            ] {
                painter.line_segment(segment, egui::Stroke::new(1.5 * zoom, couleur));
            }
            flow_arrow_head(&painter, retour, egui::vec2(0.0, -1.0), couleur, zoom);
        } else {
            painter.line_segment([départ, arrivée], egui::Stroke::new(1.5 * zoom, couleur));
            flow_arrow_head(&painter, arrivée, egui::vec2(0.0, 1.0), couleur, zoom);
        }
    }

    // Puis les boîtes.
    let mut aller = None;
    for (index, bloc) in cfg.blocks.iter().enumerate() {
        let rect = décalé(rects[index]);
        let courant = ici == Some(index);
        // Un bloc de la route demandée se voit : c'est le chemin qu'on
        // vient de chercher, et il doit se suivre des yeux.
        let sur_route = route.contains(&index);
        let fond = if courant {
            theme.accent.gamma_multiply(0.22)
        } else if sur_route {
            theme.good.gamma_multiply(0.16)
        } else {
            theme.dim.gamma_multiply(0.10)
        };
        painter.rect_filled(rect, 4.0 * zoom, fond);
        painter.rect_stroke(
            rect,
            4.0 * zoom,
            egui::Stroke::new(
                if courant || sur_route { 2.0 } else { 1.0 },
                if courant {
                    theme.accent
                } else if sur_route {
                    theme.good
                } else {
                    theme.dim
                },
            ),
            egui::StrokeKind::Inside,
        );

        // L'adresse du bloc en tête, puis ses instructions.
        let mut ligne_y = rect.top() + 4.0 * zoom;
        let entête = match bloc.exit {
            Exit::Indirect => format!("{:#010x}  ?", bloc.start),
            _ => format!("{:#010x}", bloc.start),
        };
        painter.text(
            egui::pos2(rect.left() + 6.0 * zoom, ligne_y),
            egui::Align2::LEFT_TOP,
            entête.clone(),
            police.clone(),
            if courant { theme.accent } else { theme.dim },
        );
        ligne_y += hauteur_ligne;
        for (rang, texte) in bloc.instructions.iter().enumerate() {
            if rang == LIGNES_MAX {
                painter.text(
                    egui::pos2(rect.left() + 6.0 * zoom, ligne_y),
                    egui::Align2::LEFT_TOP,
                    format!("… {}", bloc.instructions.len() - LIGNES_MAX),
                    police.clone(),
                    theme.dim,
                );
                break;
            }
            painter.text(
                egui::pos2(rect.left() + 6.0 * zoom, ligne_y),
                egui::Align2::LEFT_TOP,
                flow_ellipsis(texte, colonnes),
                police.clone(),
                theme.text,
            );
            ligne_y += hauteur_ligne;
        }
        // « le programme est ici », à droite de l'en-tête. La largeur
        // des boîtes réserve de quoi les mettre côte à côte : sans cela
        // le libellé venait recouvrir l'adresse dès que la boîte était
        // étroite, et l'on ne lisait plus ni l'un ni l'autre.
        if courant {
            painter.text(
                egui::pos2(rect.right() - 6.0 * zoom, rect.top() + 4.0 * zoom),
                egui::Align2::RIGHT_TOP,
                s.flow_here,
                police.clone(),
                theme.accent,
            );
        }

        // Un clic emmène à la ligne du bloc dans l'éditeur : le dessin
        // sert à s'orienter, pas à remplacer le code.
        if réponse.clicked()
            && let Some(pos) = réponse.interact_pointer_pos()
            && rect.contains(pos)
        {
            aller = lignes.get(&bloc.start).copied();
        }
    }
    réponse.on_hover_text(s.flow_goto);
    (aller, tiré)
}

/// La route vers une adresse : ce qu'il faut satisfaire, dans l'ordre.
///
/// C'est la lecture d'un programme qui refuse quelque chose : on part de
/// l'endroit où il accepte, et l'on remonte les conditions. Chaque ligne
/// nomme l'instruction qui décide et ce qu'elle doit répondre.
fn flow_route_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    cfg: &namsdec_re::cfg::Cfg,
    route: &namsdec_re::route::Route,
    lang: Lang,
) {
    use namsdec_re::route::Condition;
    ui.add_space(2.0);
    ui.colored_label(
        theme.good,
        format!(
            "{} {}",
            route.stages.len(),
            if route.stages.len() > 1 {
                s.flow_route_steps
            } else {
                s.flow_route_step
            }
        ),
    );
    egui::Grid::new("flow_route")
        .num_columns(3)
        .min_col_width(0.0)
        .striped(true)
        .show(ui, |ui| {
            for étape in &route.stages {
                ui.monospace(format!("{:#010x}", étape.addr));
                ui.monospace(&étape.decides);
                ui.colored_label(
                    match étape.condition {
                        Condition::Taken => theme.good,
                        Condition::NotTaken => theme.bad,
                        _ => theme.dim,
                    },
                    étape.condition.explain(lang.code()),
                );
                ui.end_row();
            }
            ui.monospace(format!("{:#010x}", cfg.blocks[route.target].start));
            ui.colored_label(theme.good, s.flow_route_arrival);
            ui.label("");
            ui.end_row();
        });
    if route.others {
        ui.colored_label(theme.dim, s.flow_route_others);
    }
    ui.add_space(2.0);
}

/// Recopie un flux d'un programme en cours dans le journal, ligne à ligne.
///
/// Deux subtilités : on lit par blocs, sans attendre la fin du programme,
/// et l'on coupe aussi bien sur `\r` que sur `\n`. C'est ce second point
/// qui rend la jauge de `curl` lisible : elle réécrit sa ligne avec un
/// retour chariot, alors on réécrit la dernière ligne du journal au lieu
/// d'en empiler mille.
fn lire_flux(
    mut flux: impl std::io::Read + Send + 'static,
    journal: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut ligne = Vec::new();
        let mut bloc = [0u8; 4096];
        let mut réécrire = false;
        loop {
            let lu = match flux.read(&mut bloc) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            for &octet in &bloc[..lu] {
                if octet != b'\n' && octet != b'\r' {
                    ligne.push(octet);
                    continue;
                }
                let texte = String::from_utf8_lossy(&ligne).trim_end().to_owned();
                ligne.clear();
                if !texte.is_empty() {
                    let mut lignes = journal.lock().expect("le journal");
                    match lignes.last_mut() {
                        Some(dernière) if réécrire => *dernière = texte,
                        _ => lignes.push(texte),
                    }
                    // Un journal de compilation tient des dizaines de
                    // milliers de lignes : on garde la fin, qui est ce
                    // qu'on lit quand ça casse.
                    if lignes.len() > 2000 {
                        lignes.drain(..500);
                    }
                }
                réécrire = octet == b'\r';
            }
        }
    })
}

/// Une pastille d'état : un mot, teinté, sur un fond de la même teinte.
///
/// Un mot coloré seul se lit mal quand il en côtoie un autre de couleur
/// voisine ; le fond en fait une étiquette, et l'œil compte les
/// étiquettes avant de les lire.
fn pastille(ui: &mut egui::Ui, couleur: egui::Color32, texte: &str) {
    egui::Frame::new()
        .fill(couleur.gamma_multiply(0.15))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .corner_radius(10.0)
        .show(ui, |ui| {
            ui.colored_label(couleur, egui::RichText::new(texte).small());
        });
}

/// Un encadré teinté, pour ce qui doit se lire avant le reste.
///
/// Un filet de couleur à gauche plutôt qu'un cadre complet : une pleine
/// boîte enferme, un filet désigne.
fn encadré(ui: &mut egui::Ui, couleur: egui::Color32, contenu: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(couleur.gamma_multiply(0.10))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            contenu(ui);
        });
}

/// La pointe d'une flèche, à `bout`, dans la direction `sens`.
fn flow_arrow_head(
    painter: &egui::Painter,
    bout: egui::Pos2,
    sens: egui::Vec2,
    couleur: egui::Color32,
    zoom: f32,
) {
    let taille = 5.0 * zoom;
    let arrière = bout - sens * taille;
    let côté = egui::vec2(-sens.y, sens.x) * (taille * 0.6);
    painter.add(egui::Shape::convex_polygon(
        vec![bout, arrière + côté, arrière - côté],
        couleur,
        egui::Stroke::NONE,
    ));
}

/// Le nom d'un onglet : le fichier, l'exercice, ou « sans titre ».
///
/// Un exercice n'a pas de fichier sur le disque, et trois onglets
/// « sans titre » côte à côte ne disent rien. Son titre, lui, dit
/// exactement ce qu'on travaille.
fn tab_label(
    path: Option<&Path>,
    exercise: Option<&str>,
    s: &Strings,
    packs: &[namsdec_exercises::Pack],
    lang: Lang,
) -> String {
    if let Some(nom) = path.and_then(|p| p.file_name()) {
        return nom.to_string_lossy().into_owned();
    }
    if let Some(exercice) = exercise.and_then(|id| namsdec_exercises::find(packs, id)) {
        return exercice.title.get(lang.code()).to_owned();
    }
    s.untitled_buffer.to_owned()
}

/// Un nom de fichier raccourci **par le milieu**.
///
/// Couper la fin ferait de `boucle_imbriquee.asm` et
/// `boucle_imbriquee_2.asm` deux onglets identiques ; garder le début et
/// la fin garde l'extension et ce qui distingue — c'est ce que font les
/// éditeurs qui affichent des chemins longs.
fn raccourcir_nom(nom: &str, maximum: usize) -> String {
    let lettres: Vec<char> = nom.chars().collect();
    if lettres.len() <= maximum {
        return nom.to_owned();
    }
    // Un caractère de moins pour le signe de coupure lui-même.
    let reste = maximum.saturating_sub(1);
    let fin = reste / 2;
    let début = reste - fin;
    let mut out: String = lettres[..début].iter().collect();
    out.push('…');
    out.extend(&lettres[lettres.len() - fin..]);
    out
}

/// Le texte de la ligne `ligne`, en comptant à partir de 1, débarrassé
/// de son indentation.
///
/// Le profil aligne ses colonnes lui-même : garder les espaces du début
/// décalerait chaque ligne d'une profondeur qui n'a plus de sens une
/// fois sortie du programme.
fn source_line(source: &str, ligne: u32) -> String {
    source
        .lines()
        .nth(ligne.saturating_sub(1) as usize)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// L'étiquette sous laquelle vit la ligne `ligne`, s'il y en a une.
///
/// La plus proche au-dessus : c'est le bloc auquel la ligne appartient.
/// Dans un profil, « boucle: dec rcx » situe la ligne sans rien aller
/// chercher, là où un numéro seul oblige à retourner dans le programme.
/// Les étiquettes locales de nasm — celles qui commencent par un point
/// — ne comptent pas : elles ne portent pas de sens à elles seules.
fn enclosing_label(source: &str, ligne: u32) -> Option<String> {
    let lignes: Vec<&str> = source.lines().take(ligne as usize).collect();
    lignes.iter().rev().find_map(|texte| {
        let texte = texte.split(';').next().unwrap_or("");
        // En tête de ligne, sans indentation : une étiquette
        // indentée n'existe pas en NASM.
        if texte.starts_with([' ', '\t']) {
            return None;
        }
        let nom = texte.split_whitespace().next()?.strip_suffix(':')?;
        (!nom.is_empty() && !nom.starts_with('.')).then(|| nom.to_owned())
    })
}

/// Toutes les étiquettes du programme, avec leur ligne, dans l'ordre où
/// elles apparaissent.
///
/// Les mêmes règles que partout ici : en tête de ligne, suivies de deux
/// points, et les étiquettes locales de nasm — celles qui commencent par
/// un point — laissées de côté, puisqu'elles ne veulent rien dire hors
/// de leur bloc.
fn source_labels(source: &str) -> Vec<(String, u32)> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, texte)| {
            let texte = texte.split(';').next().unwrap_or("");
            if texte.starts_with([' ', '\t']) {
                return None;
            }
            let nom = texte.split_whitespace().next()?.strip_suffix(':')?;
            (!nom.is_empty() && !nom.starts_with('.')).then(|| (nom.to_owned(), index as u32 + 1))
        })
        .collect()
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

/// Les morceaux exécutables d'un ELF, en position dans le fichier, bornés
/// à ce qui a été chargé.
///
/// `None` quand ce n'est pas un ELF : il n'y a alors rien d'autre à
/// faire que de tout lire comme du code, faute de savoir où il commence.
fn executable_ranges(file: &[u8], loaded: usize) -> Option<Vec<(u64, u64)>> {
    let elf = namsdec_re::elf_scan::scan(file)?;
    let mut plages: Vec<(u64, u64)> = elf
        .sections
        .iter()
        .filter(|section| section.execute && !section.nobits && section.size > 0)
        .filter_map(|section| {
            let fin = section.offset.checked_add(section.size)?;
            // Bornée à la fenêtre chargée : ce qui dépasse n'existe pas
            // en mémoire, et un listing qui le montrerait mentirait.
            let fin = fin.min(loaded as u64);
            // `then` et non `then_some` : l'argument de ce dernier est
            // évalué quoi qu'il arrive, et la soustraction déborderait
            // pour une section qui commence après la fenêtre.
            (fin > section.offset).then(|| (section.offset, fin - section.offset))
        })
        .collect();
    plages.sort_unstable();
    (!plages.is_empty()).then_some(plages)
}

/// Les largeurs, en caractères, des trois colonnes chiffrées du profil.
///
/// Assez pour un numéro de ligne à cinq chiffres, un compte de passages
/// groupé par milliers, et « 100.0 % ». Les titres partagent le même
/// gabarit : c'est ce qui les pose exactement au-dessus de ce qu'ils
/// nomment.
const COL_LIGNE: usize = 6;
const COL_PASSAGES: usize = 9;
const COL_PART: usize = 7;

/// Une part en pourcentage, écrite comme la langue l'écrit.
///
/// Le français met une virgule décimale et une espace avant le signe ;
/// l'anglais un point et rien. Un atelier qui parle deux langues écrit
/// ses nombres dans les deux.
fn pourcentage(part: f32, lang: Lang) -> String {
    let brut = format!("{:.1}", part * 100.0);
    if lang == Lang::Fr {
        format!("{} %", brut.replace('.', ","))
    } else {
        format!("{brut}%")
    }
}

/// Un texte complété d'espaces à gauche, pour tenir la largeur voulue
/// dans une police à chasse fixe.
///
/// Plus long que la largeur demandée, il est rendu tel quel : mieux vaut
/// une colonne qui s'élargit qu'un nombre amputé.
fn caler_à_droite(texte: &str, largeur: usize) -> String {
    let compte = texte.chars().count();
    if compte >= largeur {
        return texte.to_owned();
    }
    let mut out = " ".repeat(largeur - compte);
    out.push_str(texte);
    out
}

/// Les valeurs que montre la légende, de la plus froide à la plus
/// chaude.
///
/// Réparties sur l'échelle logarithmique de la barre elle-même : entre
/// un passage et mille, les repères intéressants sont 1, 6, 32, 178,
/// 1000 — et non 200, 400, 600, 800.
fn heat_legend_steps(max: u64) -> Vec<u64> {
    let max = max.max(1);
    if max <= 5 {
        return (1..=max).collect();
    }
    let échelle = (max as f32).ln();
    (0..5)
        .map(|i| {
            let t = i as f32 / 4.0;
            (échelle * t).exp().round().max(1.0) as u64
        })
        // Deux repères qui tombent sur la même valeur ne se distinguent
        // pas : on n'en garde qu'un.
        .fold(Vec::new(), |mut out, valeur| {
            if out.last() != Some(&valeur) {
                out.push(valeur);
            }
            out
        })
}

/// La largeur de la bande de chaleur, à l'extrême gauche de la
/// gouttière. Assez pour se voir, assez peu pour ne pas se disputer la
/// place avec le point d'arrêt.
const HEAT_WIDTH: f32 = 4.0;

/// La couleur d'une ligne selon ce qu'elle a coûté.
///
/// L'échelle est logarithmique : entre une ligne parcourue une fois et
/// une autre parcourue mille fois, ce qui compte est l'ordre de
/// grandeur, pas la différence. Du vert d'une ligne traversée une fois
/// au rouge de celle où le programme s'attarde, en passant par le jaune
/// et l'orange : c'est la convention de toutes les cartes de chaleur,
/// et elle se lit sans légende — celle sous l'éditeur donne les
/// valeurs, l'onglet Profil le classement.
fn heat_colour(theme: &'static Theme, count: u64, max: u64) -> egui::Color32 {
    // L'échelle couvre l'intervalle observé : une ligne parcourue une
    // seule fois est au vert pur, la plus chaude au rouge pur. Compter
    // depuis zéro tassait tout le programme dans le milieu de la rampe.
    let échelle = (max.max(1) as f32).ln();
    let part = if échelle > 0.0 {
        ((count.max(1) as f32).ln() / échelle).clamp(0.0, 1.0)
    } else {
        0.0
    };
    // Vert, jaune, orange, rouge : la rampe qu'on lit sans légende,
    // celle des cartes de chaleur et des jauges de température. Le vert
    // dit « passé une fois, rien à voir ici », le rouge « c'est là que
    // le programme passe son temps ».
    //
    // Le jaune et l'orange sont posés en dur et non pris au thème : ce
    // sont des repères de lecture, pas des couleurs d'interface, et un
    // thème qui choisirait un « avertissement » violet casserait la
    // progression sans qu'on comprenne pourquoi.
    const JAUNE: egui::Color32 = egui::Color32::from_rgb(0xe3, 0xc5, 0x4a);
    const ORANGE: egui::Color32 = egui::Color32::from_rgb(0xe2, 0x88, 0x3c);
    let arrêts = [theme.good, JAUNE, ORANGE, theme.bad];
    // Trois segments entre quatre arrêts.
    let segments = (arrêts.len() - 1) as f32;
    let position = part * segments;
    let index = (position.floor() as usize).min(arrêts.len() - 2);
    arrêts[index].lerp_to_gamma(arrêts[index + 1], position - index as f32)
}

/// La ligne qu'un point de l'écran désigne dans la gouttière.
fn ligne_sous(galley: &egui::Galley, gutter_rect: egui::Rect, pos: egui::Pos2) -> Option<u32> {
    galley
        .rows
        .iter()
        .position(|row| {
            row.rect()
                .translate(gutter_rect.min.to_vec2())
                .y_range()
                .contains(pos.y)
        })
        .map(|index| index as u32 + 1)
}

/// Un bouton de lien : une marque dessinée, puis son nom.
///
/// Fait à la main plutôt qu'avec `Button::image_and_text` : ces marques
/// sont peintes, pas chargées, et elles doivent prendre la couleur du
/// texte du bouton — celle qui change au survol.
fn link_button_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    label: &str,
    dessin: fn(&egui::Painter, egui::Rect, egui::Color32),
) -> egui::Response {
    let police = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), police, theme.text);
    const MARQUE: f32 = 16.0;
    const AIR: f32 = 8.0;
    let taille = egui::vec2(
        MARQUE + AIR + galley.size().x + 24.0,
        galley.size().y.max(MARQUE) + 12.0,
    );
    let (rect, réponse) = ui.allocate_exact_size(taille, egui::Sense::click());
    let visuals = ui.style().interact(&réponse);
    ui.painter().rect_filled(rect, 6.0, visuals.weak_bg_fill);
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, visuals.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let marque = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 12.0 + MARQUE * 0.5, rect.center().y),
        egui::vec2(MARQUE, MARQUE),
    );
    dessin(ui.painter(), marque, visuals.text_color());
    ui.painter().galley(
        egui::pos2(
            marque.right() + AIR,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        visuals.text_color(),
    );
    réponse
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
/// Le nom d'une adresse : celui qu'on a posé, sinon celui du binaire.
///
/// Le nom posé à la main l'emporte, et c'est voulu : quelqu'un qui
/// renomme `sub_1d491` en `vérifie_mdp` veut voir le second — c'est même
/// toute la raison de l'avoir écrit. Retiré, on retombe sur le symbole,
/// et non sur rien.
///
/// Fonction libre plutôt que méthode : elle sert dans la barre latérale,
/// dans le graphe, dans le listing, à des endroits qui tiennent déjà un
/// emprunt sur l'application. En faire une méthode obligeait à la
/// contourner là où elle servait le plus.
pub(crate) fn named_at(
    notes: &notes::Notes,
    symbols: &[namsdec_asm::elf::Symbol],
    addr: u64,
) -> Option<String> {
    if let Some(nom) = notes.name(addr) {
        return Some(nom.to_owned());
    }
    symbol_at(symbols, addr)
}

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

/// Ce qu'on écrit sur une adresse : son nom, ou un commentaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteKind {
    Name,
    Comment,
}

/// La barre « OK » d'une fenêtre qui n'a rien à annuler.
///
/// Posée en panneau bas, et c'est ce qui compte : le contenu prend la
/// place qui reste, le bouton garde la sienne. Ajouté au corps de la
/// fenêtre, il se faisait pousser hors de l'écran par un contenu long —
/// c'est ce qui était arrivé au plan d'installation, dont la croix
/// elle-même sortait du cadre.
///
/// Rend `true` quand il faut fermer. Le séparateur au-dessus est léger :
/// une barre d'action doit se voir sans découper la fenêtre en deux.
fn ok_bar(ui: &mut egui::Ui, theme: &'static Theme, s: &Strings, id: &str) -> bool {
    let mut fermer = false;
    egui::Panel::bottom(egui::Id::new(format!("{id}_ok")))
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            if dialog_bar(ui, theme, s.dialog_ok, None).is_some() {
                fermer = true;
            }
        });
    fermer
}

/// Ce qu'une boîte de dialogue a reçu comme réponse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Le bouton principal, ou Entrée.
    Ok,
    /// Le second bouton, ou Échap, ou la croix.
    Cancel,
}

/// La barre d'action d'une boîte de dialogue : toujours la même.
///
/// Elles ne se ressemblaient pas d'une fenêtre à l'autre — OK et Quitter
/// ici, Fermer et Recommencer là, une croix seule ailleurs, et cette
/// croix sortait de l'écran dès que la fenêtre était haute. Or c'est
/// l'exception qui fait chercher : une barre d'action se trouve
/// d'autant plus vite qu'elle est au même endroit partout.
///
/// Trois règles tenues ici :
///
/// - le bouton principal à droite, plein, en couleur d'accent ;
/// - le second à sa gauche, seulement quand il y a vraiment quelque
///   chose à annuler — un bouton qui ne défait rien ment, et il vaut
///   mieux n'en pas mettre ;
/// - **Entrée vaut le principal, Échap vaut l'autre.** Relevés avant les
///   boutons et pris à la file : sans cela, un champ de texte de la même
///   fenêtre les aurait déjà consommés.
///
/// `annuler` porte le libellé du second bouton, ou `None` pour n'en pas
/// avoir. Dans ce dernier cas Échap répond quand même `Cancel` : fermer
/// une boîte qui ne fait que renseigner n'annule rien, mais on doit
/// pouvoir la fermer d'une touche.
fn dialog_bar(
    ui: &mut egui::Ui,
    theme: &Theme,
    principal: &str,
    annuler: Option<&str>,
) -> Option<Answer> {
    dialog_bar_hinted(ui, theme, principal, annuler, None)
}

/// La même, avec une infobulle sur le second bouton.
///
/// Séparée pour que le cas ordinaire reste court : la plupart des boîtes
/// n'ont rien à expliquer de plus que le mot écrit sur le bouton. Quand
/// ce mot ne suffit pas — « Recommencer » vaut un quart d'heure de
/// compilation —, le dire au survol vaut mieux que de l'allonger.
fn dialog_bar_hinted(
    ui: &mut egui::Ui,
    theme: &Theme,
    principal: &str,
    annuler: Option<&str>,
    infobulle: Option<&str>,
) -> Option<Answer> {
    let mut réponse = None;
    // Avant les boutons : les touches se prennent à la file, et un champ
    // de la même fenêtre les aurait sinon déjà mangées.
    ui.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
            réponse = Some(Answer::Ok);
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
            réponse = Some(Answer::Cancel);
        }
    });
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let ok = egui::Button::new(egui::RichText::new(principal).color(theme.bg))
            .fill(theme.accent)
            .min_size(egui::vec2(96.0, 0.0));
        if ui.add(ok).clicked() {
            réponse = Some(Answer::Ok);
        }
        if let Some(annuler) = annuler {
            let autre = egui::Button::new(annuler)
                .stroke(egui::Stroke::new(1.0, theme.dim))
                .min_size(egui::vec2(96.0, 0.0));
            let cliqué = match infobulle {
                Some(texte) => ui.add(autre).on_hover_text(texte).clicked(),
                None => ui.add(autre).clicked(),
            };
            if cliqué {
                réponse = Some(Answer::Cancel);
            }
        }
    });
    réponse
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
        Action::ShowFlow => s.flow_open,
        Action::NextTab => s.action_next_tab,
        Action::PreviousTab => s.action_previous_tab,
        Action::CloseTab => s.action_close_tab,
        Action::DuplicateLine => s.action_duplicate_line,
        Action::HighlightMatches => s.action_highlight_matches,
        Action::RenameMatches => s.action_rename_matches,
        Action::NameAddress => s.action_name_address,
        Action::CommentAddress => s.action_comment_address,
        Action::NextMatch => s.action_next_match,
        Action::PreviousMatch => s.action_previous_match,
        Action::RunToCursor => s.action_run_to_cursor,
        Action::RunToCall => s.action_run_to_call,
        Action::RunToReturn => s.action_run_to_return,
        Action::RunToSyscall => s.action_run_to_syscall,
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
/// Un grand bouton d'action : pleine largeur, rempli de la couleur
/// d'accent, une icône et un mot.
///
/// Pour les endroits où il n'y a **qu'une** chose à faire. Un bouton
/// ordinaire y est un rectangle gris de la taille de son texte, posé en
/// haut d'une colonne : on le prend pour une étiquette, et l'on cherche
/// ailleurs la commande qu'on a sous les yeux. Celui-ci occupe la
/// largeur, porte la couleur que le thème réserve à ce qui compte, et
/// se voit sans être lu.
///
/// Rendu désactivé quand il n'y a rien à faire, plutôt que caché : un
/// bouton qui disparaît laisse croire que la fonction n'existe pas.
fn big_action_button(
    ui: &mut egui::Ui,
    theme: &Theme,
    icon: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
    label: &str,
) -> egui::Response {
    const HAUTEUR: f32 = 42.0;
    const TAILLE_ICÔNE: f32 = 18.0;
    const ÉCART: f32 = 10.0;

    let largeur = ui.available_width();
    let (rect, réponse) =
        ui.allocate_exact_size(egui::vec2(largeur, HAUTEUR), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return réponse;
    }

    let actif = ui.is_enabled();
    // Le survol éclaircit, le clic assombrit : la même grammaire que les
    // boutons d'egui, avec la couleur du thème.
    let fond = match (actif, réponse.is_pointer_button_down_on(), réponse.hovered()) {
        (false, _, _) => theme.dim.gamma_multiply(0.25),
        (true, true, _) => theme.accent.gamma_multiply(0.80),
        (true, false, true) => theme.accent.gamma_multiply(1.15),
        (true, false, false) => theme.accent,
    };
    let encre = if actif { theme.bg } else { theme.dim };

    ui.painter().rect_filled(rect, 8.0, fond);

    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(14.0),
        encre,
    );
    // Icône et texte comptés ensemble, puis centrés d'un bloc : centrer
    // chacun de son côté laisse le texte de travers dès que le libellé
    // change de langue.
    let bloc = TAILLE_ICÔNE + ÉCART + galley.size().x;
    // Dans un panneau étroit, l'icône cède la place au mot : une icône
    // seule ne dit pas ce que le bouton fait, un mot tronqué non plus,
    // et entre les deux c'est le mot qui en dit le plus.
    if bloc > rect.width() - 8.0 {
        ui.painter().galley(
            egui::pos2(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            encre,
        );
        return réponse;
    }
    let gauche = rect.center().x - bloc * 0.5;
    let case = egui::Rect::from_min_size(
        egui::pos2(gauche, rect.center().y - TAILLE_ICÔNE * 0.5),
        egui::vec2(TAILLE_ICÔNE, TAILLE_ICÔNE),
    );
    icon(ui.painter(), case, encre);
    ui.painter().galley(
        egui::pos2(case.right() + ÉCART, rect.center().y - galley.size().y * 0.5),
        galley,
        encre,
    );
    réponse
}

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

/// La réinitialisation : une flèche qui fait presque le tour et revient
/// d'où elle part. L'ouverture est en haut, là où l'œil attend la
/// boucle ; la pointe marque le sens de la course.
fn icon_reset(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let centre = rect.center();
    let rayon = rect.width().min(rect.height()) / 2.0 - 1.5;

    // Un arc de trois cents degrés : le tour est fait, mais on voit
    // encore qu'il s'agit d'un cycle et non d'un disque.
    let depart = -std::f32::consts::PI * 0.35;
    let arrivee = depart + std::f32::consts::PI * 1.7;
    let points: Vec<egui::Pos2> = (0..=28)
        .map(|i| {
            let t = depart + (arrivee - depart) * (i as f32 / 28.0);
            centre + egui::vec2(t.cos(), t.sin()) * rayon
        })
        .collect();
    painter.add(egui::Shape::line(points, stroke));

    // La pointe, au bout de l'arc : deux traits en chevron, l'un et
    // l'autre pris sur la tangente et la normale du cercle en ce point,
    // pour qu'elle suive la courbe au lieu de la couper.
    let bout = centre + egui::vec2(arrivee.cos(), arrivee.sin()) * rayon;
    let tangente = egui::vec2(-arrivee.sin(), arrivee.cos());
    let normale = egui::vec2(arrivee.cos(), arrivee.sin());
    let taille = 3.5;
    painter.line_segment([bout, bout - tangente * taille + normale * taille * 0.8], stroke);
    painter.line_segment([bout, bout - tangente * taille - normale * taille * 0.8], stroke);
}

/// Agrandir : le cadre qu'on veut voir occuper toute la place.
fn icon_maximize(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    painter.rect_stroke(
        rect.shrink(1.5),
        1.0,
        egui::Stroke::new(1.5, color),
        egui::StrokeKind::Inside,
    );
}

/// Rendre sa taille : deux cadres décalés, celui de devant plus petit.
fn icon_restore(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.4, color);
    let petit = rect.shrink(1.5);
    // Celui de derrière, décalé vers le haut à droite.
    let derrière = egui::Rect::from_min_size(
        petit.min + egui::vec2(3.0, -2.0),
        petit.size() - egui::vec2(1.0, 1.0),
    );
    painter.rect_stroke(derrière, 1.0, trait_, egui::StrokeKind::Inside);
    // Celui de devant : peint par-dessus, fond compris, pour que le
    // recouvrement se lise.
    painter.rect_filled(
        egui::Rect::from_min_size(petit.min + egui::vec2(0.0, 2.0), petit.size()),
        1.0,
        egui::Color32::TRANSPARENT,
    );
    painter.rect_stroke(
        egui::Rect::from_min_size(petit.min + egui::vec2(0.0, 2.0), petit.size()),
        1.0,
        trait_,
        egui::StrokeKind::Inside,
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
/// Le bouton d'agrandissement, posé dans le bandeau d'une fenêtre.
///
/// `egui::Window` ne laisse rien mettre dans son bandeau : la croix y est
/// posée par son propre agencement, hors d'atteinte. On dessine donc le
/// bouton par-dessus, une fois la fenêtre rendue — mais à la géométrie
/// exacte de la croix, et non à vue de nez.
///
/// Cette géométrie est celle qu'egui emploie pour ses propres boutons de
/// bandeau : une case carrée de la hauteur d'une ligne de titre, dans
/// laquelle l'icône est centrée à `icon_width`. La case allouée est plus
/// grande que l'icône, faute de quoi les boutons paraîtraient décalés
/// vers le haut à côté d'un titre plus haut qu'eux. Le nôtre se range
/// d'une case à gauche, séparé par l'écart ordinaire entre éléments.
fn title_bar_maximize_button(
    ctx: &egui::Context,
    id: &'static str,
    fenêtre: egui::Rect,
    theme: &'static Theme,
    s: &Strings,
    agrandie: bool,
) -> bool {
    let style = ctx.style_of(ctx.theme());
    let icône_côté = style.spacing.icon_width;
    // La hauteur d'une ligne de titre : c'est elle qui donne sa taille à
    // la case, non l'icône.
    let case_côté = ctx.fonts_mut(|f| f.row_height(&egui::TextStyle::Heading.resolve(&style)));
    let écart = style.spacing.item_spacing.x;
    let marge_droite = f32::from(style.spacing.window_margin.right);
    let marge_haut = f32::from(style.spacing.window_margin.top);

    // La case de la croix, puis la nôtre juste à sa gauche.
    let croix = egui::Rect::from_min_size(
        egui::pos2(
            fenêtre.right() - marge_droite - case_côté,
            fenêtre.top() + marge_haut,
        ),
        egui::vec2(case_côté, case_côté),
    );
    let case = croix.translate(egui::vec2(-(case_côté + écart), 0.0));
    let dessin = egui::Rect::from_center_size(case.center(), egui::Vec2::splat(icône_côté));

    // Le double-clic sur le bandeau fait la même chose, partout ailleurs.
    // Il est libre ici : ces fenêtres ne se replient pas, et c'est le
    // repli qu'egui lui donne d'ordinaire.
    let bandeau = egui::Rect::from_min_max(
        fenêtre.min,
        egui::pos2(croix.left(), croix.bottom()),
    );
    let double = ctx.input(|i| {
        i.pointer.button_double_clicked(egui::PointerButton::Primary)
            && i.pointer
                .interact_pos()
                .is_some_and(|p| bandeau.contains(p))
    });

    let mut cliqué = double;
    egui::Area::new(egui::Id::new(("maximize", id)))
        .order(egui::Order::Foreground)
        .fixed_pos(case.min)
        .show(ctx, |ui| {
            let (rect, réponse) = ui.allocate_exact_size(case.size(), egui::Sense::click());
            let couleur = if réponse.hovered() {
                theme.text
            } else {
                theme.dim
            };
            if réponse.hovered() {
                ui.painter()
                    .rect_filled(rect, 3.0, theme.dim.gamma_multiply(0.25));
            }
            // L'icône au centre de la case, à la taille des icônes
            // d'egui : la croix voisine est peinte exactement ainsi.
            let dessin = egui::Rect::from_center_size(rect.center(), dessin.size());
            if agrandie {
                icon_restore(ui.painter(), dessin, couleur);
            } else {
                icon_maximize(ui.painter(), dessin, couleur);
            }
            let quoi = if agrandie {
                s.window_restore
            } else {
                s.window_maximize
            };
            cliqué |= réponse.on_hover_text(quoi).clicked();
        });
    cliqué
}


/// Découpe une ligne d'arguments comme un interpréteur de commandes le
/// ferait, en plus simple.
///
/// L'espace sépare, les guillemets — simples ou doubles — groupent ce qui
/// doit rester ensemble : un mot de passe contenant une espace se donne
/// ainsi sans qu'on ait à y penser. Pas d'échappement ni de variables :
/// ce qu'on écrit là part tel quel dans `argv`, et une ligne qui ne se
/// lit pas à l'œil n'aurait pas sa place devant un programme qu'on
/// cherche à comprendre.
fn split_args(ligne: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut courant = String::new();
    let mut ouvert: Option<char> = None;
    let mut commencé = false;
    for c in ligne.chars() {
        match (ouvert, c) {
            (Some(guillemet), _) if c == guillemet => ouvert = None,
            (Some(_), _) => courant.push(c),
            (None, '\'') | (None, '"') => {
                ouvert = Some(c);
                commencé = true;
            }
            (None, c) if c.is_whitespace() => {
                if commencé {
                    arguments.push(std::mem::take(&mut courant));
                    commencé = false;
                }
            }
            (None, c) => {
                courant.push(c);
                commencé = true;
            }
        }
    }
    if commencé {
        arguments.push(courant);
    }
    arguments
}

/// Un déplacement du mode Normal, tel que vim le nomme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Move {
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
    FileEnd,
    /// Le début du fichier — `gg` dans vim.
    FileStart,
    /// Le début du mot suivant — `w`.
    WordForward,
    /// Le début du mot précédent — `b`.
    WordBack,
    /// La fin du mot où l'on est — `e`.
    WordEnd,
}

/// Les trois familles de caractères que vim distingue pour `w` et `b`.
///
/// Un mot, une suite de ponctuation, ou du blanc. C'est ce découpage qui
/// fait que `w` s'arrête sur le `[` de `[rbp-0x18]` au lieu de sauter la
/// moitié de la ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Classe {
    Mot,
    Ponctuation,
    Blanc,
}

fn classe_de(c: Option<char>) -> Classe {
    match c {
        None => Classe::Blanc,
        Some(c) if c.is_alphanumeric() || c == '_' => Classe::Mot,
        Some(c) if c.is_whitespace() => Classe::Blanc,
        Some(_) => Classe::Ponctuation,
    }
}

/// Où va le curseur, compté en caractères depuis le début du texte.
///
/// Le mode Normal n'avait aucun déplacement : `i` entrait en Insertion,
/// Échap en sortait, et c'était tout. On ne pouvait donc pas se promener
/// dans le code sans repasser par la souris — la première chose que
/// cherche quiconque vient de vim.
///
/// Compté en caractères et non en octets : une ligne de commentaire
/// accentuée ferait sauter le curseur au milieu d'un caractère.
fn move_cursor(source: &str, at: usize, mouvement: Move) -> usize {
    let caractères: Vec<char> = source.chars().collect();
    let at = at.min(caractères.len());
    // Le début de la ligne où l'on est, et sa longueur.
    let début = caractères[..at]
        .iter()
        .rposition(|c| *c == '\n')
        .map_or(0, |i| i + 1);
    let fin = caractères[at..]
        .iter()
        .position(|c| *c == '\n')
        .map_or(caractères.len(), |i| at + i);
    let colonne = at - début;

    match mouvement {
        // `h` et `l` restent sur leur ligne : dans vim, ils ne la
        // quittent pas, et c'est ce qui les rend prévisibles.
        Move::Left => at.saturating_sub(1).max(début),
        Move::Right => (at + 1).min(fin),
        Move::LineStart => début,
        Move::LineEnd => fin,
        Move::FileEnd => caractères.len(),
        Move::FileStart => 0,
        // Les mouvements par mot, et c'est eux qu'on réclame en premier
        // quand on a l'habitude de vim : se déplacer caractère par
        // caractère dans `mov qword [rbp-0x18], rax` use la patience.
        //
        // La notion de mot est celle de vim, pas celle d'un traitement
        // de texte : une suite de lettres, chiffres et soulignés, ou
        // bien une suite de ponctuation. `[rbp-0x18]` compte donc pour
        // plusieurs mots, ce qui est exactement ce qu'on veut pour
        // atteindre le déplacement sans viser à la souris.
        Move::WordForward => {
            let mut i = at;
            let classe = classe_de(caractères.get(i).copied());
            // On sort du mot courant…
            while i < caractères.len()
                && classe_de(Some(caractères[i])) == classe
                && classe != Classe::Blanc
            {
                i += 1;
            }
            // …puis des blancs qui suivent. Les retours à la ligne
            // arrêtent : `w` en fin de ligne descend d'une ligne dans
            // vim, mais s'arrête au premier mot — il ne saute pas une
            // ligne vide.
            while i < caractères.len() && caractères[i] != '\n' && caractères[i].is_whitespace() {
                i += 1;
            }
            i
        }
        Move::WordBack => {
            let mut i = at;
            // Les blancs d'abord : on recule jusqu'à toucher un mot.
            while i > 0 && caractères[i - 1] != '\n' && caractères[i - 1].is_whitespace() {
                i -= 1;
            }
            if i == 0 {
                return 0;
            }
            let classe = classe_de(Some(caractères[i - 1]));
            while i > 0 && classe_de(Some(caractères[i - 1])) == classe {
                i -= 1;
            }
            i
        }
        Move::WordEnd => {
            let mut i = at;
            // Si l'on est déjà sur la fin d'un mot, on va à la suivante :
            // `e` répété doit avancer, non piétiner.
            if i < caractères.len() {
                i += 1;
            }
            while i < caractères.len() && caractères[i] != '\n' && caractères[i].is_whitespace() {
                i += 1;
            }
            let classe = classe_de(caractères.get(i).copied());
            while i + 1 < caractères.len()
                && classe_de(Some(caractères[i + 1])) == classe
                && classe != Classe::Blanc
            {
                i += 1;
            }
            i
        }
        Move::Up => {
            if début == 0 {
                return at;
            }
            let fin_précédente = début - 1;
            let début_précédente = caractères[..fin_précédente]
                .iter()
                .rposition(|c| *c == '\n')
                .map_or(0, |i| i + 1);
            // La colonne est gardée tant que la ligne visée est assez
            // longue, comme partout ailleurs.
            (début_précédente + colonne).min(fin_précédente)
        }
        Move::Down => {
            if fin >= caractères.len() {
                return at;
            }
            let début_suivante = fin + 1;
            let fin_suivante = caractères[début_suivante..]
                .iter()
                .position(|c| *c == '\n')
                .map_or(caractères.len(), |i| début_suivante + i);
            (début_suivante + colonne).min(fin_suivante)
        }
    }
}

/// Une ligne de la barre de statut, avec de quoi lire ce qui dépasse.
///
/// Les messages longs — celui du binaire lié dynamiquement en fait
/// trois lignes — se coupaient avec des points de suspension, et rien ne
/// disait qu'il y avait plus à lire. L'infobulle le donne en entier, et
/// une mention l'annonce plutôt que de compter sur la curiosité.
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
        self.poll_update_check(&ctx);
        self.poll_decompile(&ctx);
        self.poll_install(&ctx);
        self.reflect_job_in_title(&ctx);
        // Au retour dans la fenêtre : un autre programme a pu passer par
        // là — un `git checkout`, un éditeur, un `sed`. On regarde à ce
        // moment-là, et non à chaque image : c'est un accès au disque.
        let focus = ctx.input(|i| i.focused);
        if focus && !self.had_focus {
            self.check_disk_change();
        }
        self.had_focus = focus;
        self.apply_tooltip_setting(&ctx);
        self.tick_run(&ctx);
        self.note_goal_reached();
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

        // Les décorations suivent le réglage, et ne sont renvoyées au
        // système que lorsqu'il change : une commande par frame ferait
        // clignoter la fenêtre sous certains bureaux.
        if self.decorations_applied != Some(self.system_titlebar) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(self.system_titlebar));
            self.decorations_applied = Some(self.system_titlebar);
        }
        // Ce qu'une pastille a demandé à la frame précédente.
        match self.window_command.take() {
            Some(WindowCommand::Minimize) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            Some(WindowCommand::Maximize) => {
                let maximisée = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximisée));
            }
            None => {}
        }

        // Les prises de redimensionnement, quand la fenêtre n'a pas
        // celles du système : posées avant le reste pour être dessous,
        // elles ne prennent le clic que sur les huit pixels du bord.
        if !self.system_titlebar {
            chrome::resize_edges_ui(&ctx, self.theme);
        }

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
                let plafond =
                    (ui.available_height() * CONSOLE_MAX_FRACTION).max(CONSOLE_MIN_HEIGHT);
                let voulue = self.console_wanted_height(ui).min(plafond);
                let id = egui::Id::new("console");
                // En mode automatique, la hauteur est réécrite à chaque
                // image : c'est ainsi qu'elle suit ce que la console a à
                // dire, plutôt que de garder celle d'une image passée.
                if self.console_height.is_none() {
                    let état = egui::containers::panel::PanelState {
                        outer_rect: egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(ui.available_width(), voulue),
                        ),
                    };
                    ui.ctx().data_mut(|data| data.insert_persisted(id, état));
                }
                // Le plancher : ce que la zone de saisie prend, quand il
                // y en a une. Une hauteur figée à la poignée ne doit pas
                // pouvoir passer dessous — sinon le champ où l'on tape se
                // retrouve sous la barre de statut, au moment précis où
                // le programme attend quelque chose. L'utilisateur règle
                // la place de la sortie, pas celle de ce qui commande.
                let saisie = self
                    .console_input_estimate(ui)
                    .max(self.console_input_height.unwrap_or(0.0));
                let plancher = if saisie > 0.0 {
                    // La saisie, les onglets, et une ligne de journal :
                    // en dessous, on ne verrait plus où taper.
                    (saisie + ui.text_style_height(&egui::TextStyle::Button) + 28.0)
                        .max(CONSOLE_MIN_HEIGHT)
                        .min(plafond)
                } else {
                    CONSOLE_MIN_HEIGHT.min(plafond)
                };
                egui::Panel::bottom(id)
                    .resizable(true)
                    .default_size(self.console_height.unwrap_or(voulue).max(plancher))
                    .min_size(plancher)
                    .max_size(plafond)
                    .show(ui, |ui| self.console_panel_ui(ui));
                // Ce que le panneau fait vraiment. S'il n'est pas à la
                // hauteur demandée, c'est que la poignée a été tirée :
                // on retient alors ce choix, et l'ajustement s'arrête.
                let rendue = egui::containers::panel::PanelState::load(ui.ctx(), id)
                    .map(|état| état.outer_rect.height());
                if let Some(rendue) = rendue {
                    match self.console_height {
                        Some(gardée) if (rendue - gardée).abs() > 1.0 => {
                            self.console_height = Some(rendue);
                        }
                        None if (rendue - voulue).abs() > 1.0 => {
                            self.console_height = Some(rendue);
                        }
                        _ => {}
                    }
                }
            });
        }
        timed(&mut spans, "panneau latéral", || {
            if self.show_sidebar {
                let minimum = sidebar_min_width(ui);
                let id = egui::Id::new("sidebar");
                // `min_size` ne borne que la poignée : une largeur
                // retenue d'une session précédente — ou d'avant un
                // changement de police ou de zoom — passe dessous sans
                // être corrigée, et les registres perdent leur dernier
                // chiffre. On la relève donc nous-mêmes.
                if let Some(état) = egui::containers::panel::PanelState::load(ui.ctx(), id)
                    && état.outer_rect.width() < minimum
                {
                    let corrigé = egui::containers::panel::PanelState {
                        outer_rect: egui::Rect::from_min_size(
                            état.outer_rect.min,
                            egui::vec2(minimum, état.outer_rect.height()),
                        ),
                    };
                    ui.ctx().data_mut(|data| data.insert_persisted(id, corrigé));
                }
                egui::Panel::right(id)
                    .resizable(true)
                    .default_size(SIDEBAR_WIDTH)
                    .min_size(minimum)
                    .max_size((ui.available_width() * SIDEBAR_MAX_FRACTION).max(minimum))
                    .show(ui, |ui| self.sidebar_ui(ui));
            }
        });
        // La légende de la chaleur, sous l'éditeur : une bande de
        // couleur sans échelle ne dit rien — « plus rouge » ne se
        // convertit en « mille passages » que si quelque chose le dit.
        if self.heat_legend {
            timed(&mut spans, "légende", || {
                let passages = self.line_visits();
                if !passages.is_empty() {
                    let max = passages.values().copied().max().unwrap_or(1);
                    egui::Panel::bottom("heat_legend")
                        .resizable(false)
                        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 4)))
                        .show(ui, |ui| self.heat_legend_ui(ui, max));
                }
            });
        }

        // L'éditeur recolore tout le tampon à chaque frame : c'est le
        // premier suspect dès que la source devient longue (un binaire
        // désassemblé, par exemple).
        timed(&mut spans, "éditeur", || self.editor_ui(ui));

        // Fenêtres flottantes — dont l'explorateur, qui n'occupe donc plus
        // de place dans la mise en page et ne dépend d'aucun panneau.
        // L'explorateur relit le disque, d'où sa mesure à part.
        timed(&mut spans, "explorateur", || self.explorer_window_ui(&ctx));
        timed(&mut spans, "fenêtres", || {
            // Avant de les dessiner : celle qui vient de s'ouvrir doit
            // passer devant celle d'où elle part.
            self.raise_new_windows(&ctx);
            self.preferences_window_ui(&ctx);
            self.about_window_ui(&ctx);
            self.help_window_ui(&ctx);
            self.vectors_window_ui(&ctx);
            self.breakpoints_window_ui(&ctx);
            self.timeline_window_ui(&ctx);
            self.flow_window_ui(&ctx);
            self.decompiler_window_ui(&ctx);
            self.installer_window_ui(&ctx);
            self.disassembly_window_ui(&ctx);
            self.cyclic_window_ui(&ctx);
            self.analysis_window_ui(&ctx);
            self.calculator_window_ui(&ctx);
            if self.exercises {
                self.exercises_window_ui(&ctx);
            }
            self.command_palette_ui(&ctx);
            self.unsaved_window_ui(&ctx);
            // En dernier : c'est la seule fenêtre qui réclame une
            // réponse, elle passe donc devant toutes les autres.
            self.input_dialog_ui(&ctx);
        });

        // En dernier : tout réglage changé pendant cette frame, d'où qu'il
        // vienne (menu, préférences, palette, raccourci), est sur le
        // disque avant la frame suivante.
        timed(&mut spans, "réglages", || {
            self.persist_settings_if_changed()
        });

        // Le texte a-t-il changé pendant cette frame ? Une frappe ne
        // rend pas une frame lente ; un listing de soixante mille lignes
        // qui arrive, si. La conjonction des deux désigne donc la mise
        // en page initiale, sans qu'aucun appelant ait à le signaler —
        // et il y en a une dizaine qui remplacent le texte.
        let texte_changé = self.source.len() != self.last_layout_len;
        self.last_layout_len = self.source.len();
        report_slow_frame(frame_start.elapsed(), &spans, texte_changé);
    }
}

/// L'éditeur porte un identifiant fixe : l'autocomplétion doit pouvoir
/// replacer son curseur après avoir écrit, ce qui demande de retrouver
/// son état.
fn find_field_id() -> egui::Id {
    egui::Id::new("find_field")
}

/// Le drapeau « la commande vient d'être copiée », pour que le bouton
/// le dise. Volatil : il n'a de sens que le temps que la fenêtre reste
/// ouverte.
fn copie_id() -> egui::Id {
    egui::Id::new("prerequis_copies")
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
/// L'identifiant de la boîte « À propos ».
///
/// Écrit une fois, employé partout : la fenêtre elle-même, la table qui
/// remet devant les fenêtres qui s'ouvrent, et le test qui relit sa
/// position. Trois endroits, un seul mot — sans quoi renommer l'un
/// laisserait les autres chercher une fenêtre qui n'existe plus.
pub(crate) const ABOUT_WINDOW: &str = "about_window";

/// L'identifiant de la boîte de saisie.
///
/// Nommé une fois : la fenêtre s'en sert, et la table qui remet devant
/// les fenêtres qui s'ouvrent aussi — c'est elle qui fait passer la
/// boîte au-dessus d'une fenêtre d'analyse ouverte en grand.
pub(crate) const INPUT_DIALOG: &str = "input_dialog";

/// La largeur au-delà de laquelle le contenu de la fenêtre d'analyse ne
/// s'étire plus.
///
/// Un volet de détail qui prend deux mille pixels ne se lit pas : une
/// table à deux colonnes y pose la seconde à mille pixels de la
/// première, et l'on suit la ligne du doigt. Mille pixels, c'est une
/// centaine de caractères de chasse fixe — la largeur d'un terminal
/// large, et celle au-delà de laquelle l'œil perd la ligne en revenant à
/// la gauche.
pub(crate) const ANALYSIS_CONTENT_MAX: f32 = 1000.0;

/// L'identifiant de la boîte « À propos », sous la forme qu'egui attend.
///
/// **Pour les tests seulement** : la fenêtre, elle, passe la constante.
/// Sans le `cfg`, la compilation du binaire signalait une fonction morte
/// qui ne l'était pas, et un avertissement qu'on apprend à ignorer finit
/// par en cacher un vrai.
#[cfg(test)]
fn about_window_id() -> egui::Id {
    egui::Id::new(ABOUT_WINDOW)
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

/// Les sections qu'un binaire ordinaire porte, et qui méritent d'être
/// nommées telles quelles dans la carte mémoire et dans les messages de
/// faute.
///
/// Un programme qu'on vient d'écrire en a trois ou quatre ; un exécutable
/// venu du disque en a vingt, et savoir qu'on est tombé dans `.got` ou
/// dans `.plt` plutôt que dans « des données » est souvent toute la
/// réponse à la question qu'on se pose.
const SECTIONS_CONNUES: &[&str] = &[
    ".text",
    ".data",
    ".bss",
    ".rodata",
    ".init",
    ".fini",
    ".plt",
    ".plt.sec",
    ".plt.got",
    ".got",
    ".got.plt",
    ".init_array",
    ".fini_array",
    ".data.rel.ro",
    ".dynamic",
    ".dynsym",
    ".dynstr",
    ".interp",
    ".eh_frame",
    ".eh_frame_hdr",
    ".gcc_except_table",
    ".tdata",
    ".tbss",
    ".rela.dyn",
    ".rela.plt",
    ".gnu.hash",
];

/// Le nom sous lequel annoncer une section dans un message de faute.
///
/// Les noms usuels passent tels quels ; une section au nom exotique est
/// annoncée par ce qu'elle *est* — du code, des données, de la donnée
/// en lecture seule. Le message doit pouvoir être écrit dans les deux
/// langues, et un nom venu du fichier ne peut pas l'être : il n'a pas de
/// durée de vie statique, et il ne se traduit pas.
fn region_label(section: &namsdec_asm::Section) -> &'static str {
    region_label_named(&section.name, section.executable, section.writable)
}

/// Même chose, depuis les faits plutôt que depuis la structure : le
/// chargeur travaille sur les sections de l'ELF, qui ne sont pas du même
/// type que celles d'un programme assemblé mais portent les mêmes noms.
pub(crate) fn region_label_named(name: &str, executable: bool, writable: bool) -> &'static str {
    if let Some(connu) = SECTIONS_CONNUES.iter().find(|connu| **connu == name) {
        return connu;
    }
    if executable {
        ".text"
    } else if writable {
        ".data"
    } else {
        ".rodata"
    }
}

/// Où vivent les décompilateurs déclarés par l'utilisateur.
///
/// À côté des désassembleurs, et pour la même raison qu'ils en sont
/// séparés : les uns rendent des instructions, les autres du
/// pseudo-code, et l'on ne choisit pas dans la même liste selon la
/// question qu'on se pose.
/// namsdec sait-il aller chercher cet outil lui-même ?
///
/// Non pour ceux qui viennent des paquets du système : les installer
/// demanderait `sudo`, et namsdec ne le prendra pas.
fn recette_existe(outil: &str) -> bool {
    data_dir().is_some_and(|dir| namsdec_disasm::install::recipe_for(outil, &dir).is_some())
}

fn decompilers_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("decompilers"))
}

/// Les décompilateurs installés. Lus une fois au démarrage.
fn load_installed_decompilers() -> Vec<namsdec_disasm::plugin::Plugin> {
    decompilers_dir()
        .map(|dir| namsdec_disasm::plugin::read_dir(&dir))
        .unwrap_or_default()
}

/// Où vivent les séries d'exercices installées. À côté des thèmes, et
/// pour la même raison : ce sont des données que l'utilisateur ajoute, et
/// elles doivent survivre à une mise à jour de l'application.
fn exercises_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("exercises"))
}

/// Où vivent les noms et les commentaires qu'on pose sur un binaire.
///
/// À côté du binaire, jamais dedans : on n'écrit pas dans l'exécutable
/// de quelqu'un pour y noter ce qu'on en pense.
fn notes_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("notes"))
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
    // La série livrée d'abord, quel que soit l'ordre alphabétique :
    // c'est par elle qu'on commence, et les autres viennent ensuite.
    let livrée = dir.join(namsdec_exercises::BUILTIN_FILE);
    paths.sort_by_key(|path| *path != livrée);
    for path in paths {
        let est_livrée = path == livrée;
        match namsdec_exercises::pack::read(&path) {
            Ok(pack) if est_livrée || pack.id != namsdec_exercises::BUILTIN_ID => {
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

/// Recopie la ligne qui contient `cursor`, juste en dessous.
///
/// Rend la source modifiée et la nouvelle position du curseur, qui suit
/// la copie : on duplique une ligne pour la changer, et la changer
/// commence par être dessus.
///
/// La dernière ligne d'un fichier sans retour final est un cas à part —
/// il faut en ajouter un, sinon la copie se collerait derrière l'original
/// et n'en ferait qu'une seule ligne deux fois plus longue.
fn duplicated_line(source: &str, cursor: usize) -> (String, usize) {
    let cursor = cursor.min(source.len());
    let début = source[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let fin = source[cursor..]
        .find('\n')
        .map_or(source.len(), |i| cursor + i);
    let ligne = &source[début..fin];

    let mut out = String::with_capacity(source.len() + ligne.len() + 1);
    out.push_str(&source[..fin]);
    out.push('\n');
    out.push_str(ligne);
    out.push_str(&source[fin..]);
    // Le curseur garde sa colonne, une ligne plus bas.
    (out, cursor + ligne.len() + 1)
}

/// Où commence la ligne de rang `index`, en octets.
///
/// `str::lines()` ne rend pas les positions, et les recalculer à la main
/// à chaque usage finit par diverger de ce que `lines()` appelle une
/// ligne — notamment sur `\r\n`. Une seule fonction, donc, et les
/// autres s'y réfèrent.
fn ligne_début(source: &str, index: usize) -> Option<usize> {
    let mut reste = index;
    let mut début = 0usize;
    while reste > 0 {
        let saut = source[début..].find('\n')?;
        début += saut + 1;
        reste -= 1;
    }
    (début <= source.len()).then_some(début)
}

/// La correction calculée, et ce pour quoi elle l'a été.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FixKey {
    index: usize,
    ligne: String,
    taille: usize,
    lang: Lang,
}

#[derive(Debug, Clone)]
struct FixCache {
    clé: FixKey,
    correction: Option<fixes::Correction>,
}

/// Ce que la liste de propositions a pris au clavier cette image-ci.
#[derive(Default, Clone, Copy)]
struct CompletionKeys {
    accept: bool,
    down: bool,
    up: bool,
    dismiss: bool,
}

/// Ce qu'une erreur de calcul dit à qui l'a faite.
///
/// En clair, et en nommant ce qui coince : « erreur de syntaxe » oblige à
/// chercher soi-même, ce qui est exactement le travail qu'on venait
/// éviter.
/// Combien de résultats de recherche mémoire on montre au plus.
///
/// Un motif d'un seul octet dans un mégaoctet trouve des milliers de
/// réponses, et une liste de milliers de lignes n'apprend rien : ce
/// qu'on veut savoir, c'est où commencer à regarder. Quand la borne est
/// atteinte, le panneau le dit — une liste tronquée qui se tait fait
/// conclure à tort qu'il n'y en a pas davantage.
pub(crate) const MEM_FIND_MAX: usize = 64;

/// La grille des bits, façon calculatrice de programmeur.
///
/// Deux rangées de trente-deux, groupées par quartets : c'est ainsi
/// qu'on lit un masque, et c'est ainsi que l'octet se retrouve du
/// premier coup d'œil. Les index encadrent chaque rangée — sans eux, on
/// compte les colonnes avec le doigt.
///
/// Chaque bit se clique pour le basculer. C'est le geste qui manque
/// partout ailleurs : on veut savoir ce que vaut `0x1f` avec le
/// quatrième bit en moins, et l'écrire à la main est plus long que de le
/// cliquer.
///
/// Les largeurs sont posées et l'espacement mis à zéro, ce qui permet de
/// calculer la largeur exacte d'une rangée — et donc de la centrer.
fn calculator_bits_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    valeur: u64,
    largeur: CalcWidth,
    actif: bool,
    basculé: &mut Option<u32>,
) {
    const PAR_RANGÉE: u32 = 32;
    /// La case d'un index, celle d'un bit, et le blanc entre deux
    /// quartets.
    const INDEX_L: f32 = 26.0;
    const BIT_L: f32 = 11.0;
    const QUARTET: f32 = 5.0;

    let bits = largeur.bits();
    let rangées = bits.div_ceil(PAR_RANGÉE);
    let par_rangée = bits.min(PAR_RANGÉE);
    let quartets = par_rangée / 4;
    let largeur_rangée = INDEX_L * 2.0
        + par_rangée as f32 * BIT_L
        + (quartets.saturating_sub(1)) as f32 * QUARTET
        + 6.0;
    let police = egui::FontId::monospace(12.0);

    for rangée in 0..rangées {
        // De haut en bas, du poids fort au poids faible : la rangée 0
        // porte les bits les plus hauts, comme un nombre s'écrit.
        let haut = bits - rangée * PAR_RANGÉE - 1;
        let bas = haut.saturating_sub(PAR_RANGÉE - 1);
        centered_row(ui, largeur_rangée, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_sized(
                [INDEX_L, 16.0],
                egui::Label::new(
                    egui::RichText::new(haut.to_string())
                        .monospace()
                        .size(10.0)
                        .color(theme.dim),
                ),
            );
            for rang in (bas..=haut).rev() {
                let (case, réponse) =
                    ui.allocate_exact_size(egui::vec2(BIT_L, 16.0), egui::Sense::click());
                let mis = valeur >> rang & 1 == 1;
                if actif && réponse.hovered() {
                    ui.painter()
                        .rect_filled(case.expand(1.0), 2.0, theme.accent.gamma_multiply(0.35));
                }
                ui.painter().text(
                    case.center(),
                    egui::Align2::CENTER_CENTER,
                    if mis { "1" } else { "0" },
                    police.clone(),
                    if !actif {
                        theme.dim.gamma_multiply(0.5)
                    } else if mis {
                        theme.accent
                    } else {
                        theme.dim
                    },
                );
                if actif && réponse.on_hover_text(s.calculator_bits_hint).clicked() {
                    *basculé = Some(rang);
                }
                // Un blanc tous les quatre bits : c'est la coupure du
                // quartet, celle qu'on lit en hexadécimal.
                if rang.is_multiple_of(4) && rang > bas {
                    ui.add_space(QUARTET);
                }
            }
            ui.add_space(6.0);
            ui.add_sized(
                [INDEX_L, 16.0],
                egui::Label::new(
                    egui::RichText::new(bas.to_string())
                        .monospace()
                        .size(10.0)
                        .color(theme.dim),
                ),
            );
        });
    }
}

/// Ce que le clavier a demandé pendant cette image.
///
/// Relevé et non agi sur place : la fenêtre emprunte `self` en écriture
/// pendant qu'elle se dessine, et le clavier ne peut donc pas toucher à
/// l'expression lui-même. Un seul paquet plutôt que trois sorties
/// séparées — c'est la même réponse, elle se transporte ensemble.
#[derive(Default)]
struct Frappe {
    écrire: Option<String>,
    effacer: bool,
    reculer: bool,
}

/// Pose une rangée de largeur connue, centrée dans la place qui reste.
///
/// `vertical_centered` ne centre que les enfants dont la largeur est
/// connue ; une rangée horizontale, elle, prend toute la largeur
/// disponible, si bien qu'elle ne se centre jamais — elle se contente
/// de remplir, et son contenu reste collé à gauche. C'est le piège
/// classique d'egui, et il s'évite en disant soi-même la largeur.
fn centered_row<R>(
    ui: &mut egui::Ui,
    largeur: f32,
    contenu: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let marge = ((ui.available_width() - largeur) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(marge);
        contenu(ui)
    })
    .inner
}

/// À quelle famille une touche appartient, et donc de quelle couleur
/// elle est.
///
/// Trois familles et pas davantage : les chiffres, les opérateurs, et ce
/// qui efface. C'est le partage de toutes les calculatrices de
/// programmeur, et c'est ce qui permet de viser une touche sans la lire
/// — on va vers la zone grise pour un chiffre, vers la bleue pour une
/// opération, vers la rouge quand on s'est trompé.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TouchePour {
    Chiffre,
    Opérateur,
    Effacer,
}

impl TouchePour {
    /// Le fond et l'encre de cette famille, dans le thème courant.
    fn couleurs(self, theme: &Theme) -> (egui::Color32, egui::Color32) {
        match self {
            TouchePour::Chiffre => (theme.dim.gamma_multiply(0.22), theme.text),
            TouchePour::Opérateur => (theme.accent.gamma_multiply(0.22), theme.accent),
            TouchePour::Effacer => (theme.bad.gamma_multiply(0.22), theme.bad),
        }
    }
}

/// La largeur d'une touche, sa hauteur, et l'écart entre deux.
///
/// Des tailles posées et non proportionnelles à la fenêtre : un clavier
/// dont les touches s'étirent n'a plus de forme qu'on reconnaisse, et
/// l'on rate celle qu'on visait après un redimensionnement. C'est le
/// bloc entier qui se centre, pas chaque touche qui s'allonge.
const TOUCHE_L: f32 = 48.0;
const TOUCHE_H: f32 = 36.0;
const TOUCHE_ÉCART: f32 = 6.0;
const COLONNES: usize = 6;

/// Cinq rangées de six touches.
///
/// Les chiffres hexadécimaux sur leur propre rangée, comme sur une
/// calculatrice de programmeur : on les cherche ensemble, on les trouve
/// ensemble. Puis le pavé décimal à gauche et les opérateurs à droite,
/// dans la disposition que tout le monde a dans les doigts.
const TOUCHES: [[&str; COLONNES]; 5] = [
    ["A", "B", "C", "D", "E", "F"],
    ["7", "8", "9", "/", "&", "<<"],
    ["4", "5", "6", "*", "|", ">>"],
    ["1", "2", "3", "-", "^", "~"],
    ["0", "0x", "(", ")", "+", "%"],
];

/// Ce qu'une touche désigne.
fn famille_de(touche: &str) -> TouchePour {
    let chiffre = touche.len() == 1
        && touche
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric());
    if chiffre {
        TouchePour::Chiffre
    } else {
        TouchePour::Opérateur
    }
}

/// Une touche du clavier, posée et peinte. Rend sa réponse.
fn calculator_key(
    ui: &mut egui::Ui,
    theme: &Theme,
    texte: &str,
    famille: TouchePour,
    largeur: f32,
    permis: bool,
) -> egui::Response {
    let (fond, encre) = famille.couleurs(theme);
    // Éteinte, et non cachée : savoir qu'une touche existe mais ne sert
    // pas dans cette base en apprend plus que de la voir disparaître.
    let (fond, encre) = if permis {
        (fond, encre)
    } else {
        (fond.gamma_multiply(0.35), encre.gamma_multiply(0.35))
    };
    let bouton = egui::Button::new(
        egui::RichText::new(texte)
            .monospace()
            .size(15.0)
            .color(encre),
    )
    .fill(fond)
    .stroke(egui::Stroke::new(1.0, encre.gamma_multiply(0.35)))
    .corner_radius(6.0)
    .min_size(egui::vec2(largeur, TOUCHE_H));
    ui.add_enabled(permis, bouton)
}

/// Le clavier de la calculatrice.
///
/// Il écrit ce que l'évaluateur lit — `&`, `|`, `^`, `<<` — et non des
/// symboles mathématiques qu'il faudrait retraduire pour les recopier
/// dans du code. C'est la même raison qui fait écrire `qword [rsp]`
/// plutôt qu'une notation à nous : namsdec sert à lire du code, et ce
/// qu'on y tape doit ressembler à ce qu'on y lit.
///
/// Les chiffres qui n'ont pas de sens dans la base courante sont
/// éteints : un clavier qui propose `9` en binaire ment sur ce qu'on
/// peut taper.
///
/// **Chaque touche est un vrai bouton**, avec son fond, son liseré et sa
/// couleur de famille. Au repos, le thème rend le fond des boutons
/// transparent et le liseré invisible : le clavier n'était alors qu'une
/// trentaine de caractères posés dans du vide, qu'on ne pensait même pas
/// à cliquer.
fn calculator_keypad_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    base: CalcBase,
    courant: &str,
    frappe: &mut Frappe,
) {
    let total = COLONNES as f32 * TOUCHE_L + (COLONNES as f32 - 1.0) * TOUCHE_ÉCART;
    let moitié = (total - TOUCHE_ÉCART) / 2.0;

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(TOUCHE_ÉCART, TOUCHE_ÉCART);

        // Effacer et revenir en arrière, au-dessus et en rouge : ce sont
        // les deux touches qu'on cherche quand on s'est trompé, et les
        // seules qui défont.
        centered_row(ui, total, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(TOUCHE_ÉCART, TOUCHE_ÉCART);
            frappe.effacer |= calculator_key(
                ui,
                theme,
                s.calculator_clear,
                TouchePour::Effacer,
                moitié,
                true,
            )
            .clicked();

            // Le retour arrière porte une icône et non un caractère :
            // `←` n'est pas dans la police d'egui.
            let (fond, encre) = TouchePour::Effacer.couleurs(theme);
            let (rect, réponse) =
                ui.allocate_exact_size(egui::vec2(moitié, TOUCHE_H), egui::Sense::click());
            ui.painter().rect(
                rect,
                6.0,
                if réponse.hovered() {
                    fond.gamma_multiply(1.6)
                } else {
                    fond
                },
                egui::Stroke::new(1.0, encre.gamma_multiply(0.35)),
                egui::StrokeKind::Inside,
            );
            chrome::icon_backspace(
                ui.painter(),
                egui::Rect::from_center_size(rect.center(), egui::vec2(18.0, 14.0)),
                encre,
            );
            frappe.reculer |= réponse.on_hover_text(s.calculator_backspace).clicked();
        });

        ui.add_space(2.0);
        for rangée in TOUCHES {
            centered_row(ui, total, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(TOUCHE_ÉCART, TOUCHE_ÉCART);
                for étiquette in rangée {
                    let famille = famille_de(étiquette);
                    let permis = famille != TouchePour::Chiffre
                        || étiquette.chars().all(|c| base.accepte(c));
                    if calculator_key(ui, theme, étiquette, famille, TOUCHE_L, permis).clicked() {
                        frappe.écrire = Some(écriture_de_touche(étiquette, base, courant));
                    }
                }
            });
        }
    });
}

/// Ce qu'une touche du clavier écrit dans l'expression.
///
/// Un chiffre qui **commence un nombre** entre avec le préfixe de la
/// base : en hexadécimal, `1` puis `f` doit donner `0x1f` et non `1f`,
/// que l'évaluateur refuserait. Un chiffre qui continue un nombre entre
/// seul — sans quoi on écrirait `0x10x5`.
///
/// « Commence un nombre » se reconnaît au dernier caractère déjà là :
/// une lettre, un chiffre ou un point le continuent, tout le reste
/// l'ouvre. C'est grossier, et c'est juste dans tous les cas qu'on
/// rencontre.
fn écriture_de_touche(touche: &str, base: CalcBase, courant: &str) -> String {
    let est_un_chiffre = touche.len() == 1
        && touche
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric());
    if !est_un_chiffre {
        return touche.to_owned();
    }
    let continue_un_nombre = courant
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_alphanumeric());
    if continue_un_nombre {
        touche.to_owned()
    } else {
        format!("{}{touche}", base.prefix())
    }
}

/// Les fonctions de rétro-ingénierie, en boutons.
///
/// Une grille qui se replie à la largeur de la fenêtre, et **une ligne
/// sous elle** qui dit ce que fait celle qu'on survole. L'infobulle
/// seule ne suffisait pas : elle se mérite après une demi-seconde
/// d'immobilité, alors qu'on parcourt dix-huit noms des yeux pour
/// trouver le bon.
fn calculator_functions_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    lang: Lang,
    à_écrire: &mut Option<String>,
) {
    let mut survolée: Option<&'static str> = None;
    ui.horizontal_wrapped(|ui| {
        for (nom, arité, fr, en) in calc::FUNCTIONS {
            let appel = if *arité == 2 {
                format!("{nom}(x, n)")
            } else {
                format!("{nom}(x)")
            };
            let réponse = ui.add(
                egui::Button::new(egui::RichText::new(&appel).monospace().size(11.5))
                    .fill(theme.dim.gamma_multiply(0.10)),
            );
            if réponse.hovered() {
                survolée = Some(if lang == Lang::Fr { *fr } else { *en });
            }
            if réponse.clicked() {
                // Le nom et sa parenthèse : on cherche ce qu'une
                // fonction fait, on la clique, on complète.
                *à_écrire = Some(format!("{nom}("));
            }
        }
    });
    ui.add_space(2.0);
    // La place de la ligne est réservée même quand rien n'est survolé :
    // sinon la grille sautait d'une ligne sous le pointeur.
    ui.colored_label(
        if survolée.is_some() {
            theme.text
        } else {
            theme.dim
        },
        survolée.unwrap_or(s.calculator_pick_function),
    );
}

/// Ce qui cloche dans un motif de recherche, dit en clair.
fn pattern_error_text(s: &Strings, erreur: &memfind::PatternError) -> String {
    use memfind::PatternError as E;
    match erreur {
        E::Empty => String::new(),
        E::Unterminated => s.pattern_unterminated.to_owned(),
        E::BadHex(c) => format!("{} {c}", s.pattern_bad_hex),
        E::HalfByte(mot) => format!("{} {mot}", s.pattern_half_byte),
    }
}

/// La mémoire du programme, telle que la calculatrice sait la lire.
///
/// `peek` et non `read` : évaluer une condition d'arrêt ne doit ni
/// déclencher une surveillance, ni compter comme un accès du programme,
/// ni échouer sur une page sans droit de lecture. C'est un coup d'œil de
/// l'extérieur, pas une instruction que la machine exécute.
struct PeekMemory<'a>(&'a namsdec_core::Memory);

impl calc::Memory for PeekMemory<'_> {
    fn peek(&self, addr: u64, taille: usize) -> Option<u64> {
        let octets = self.0.peek_bytes(addr, taille).ok()?;
        // Petit-boutiste, comme la machine : le premier octet lu est
        // celui de poids faible.
        Some(
            octets
                .iter()
                .enumerate()
                .fold(0u64, |acc, (rang, o)| acc | u64::from(*o) << (8 * rang)),
        )
    }
}

fn calc_error_text(s: &Strings, erreur: &calc::CalcError) -> String {
    use calc::CalcError as E;
    match erreur {
        E::Empty => String::new(),
        E::BadChar(c) => format!("{} {c}", s.calc_bad_char),
        E::Unknown(mot) => format!("{} {mot}", s.calc_unknown),
        E::Unbalanced => s.calc_unbalanced.to_owned(),
        E::Missing => s.calc_missing.to_owned(),
        E::Arity { name, wanted, got } => {
            format!("{name} {} {wanted}, {} {got}", s.calc_arity_wants, s.calc_arity_got)
        }
        E::NoRegisters(nom) => format!("{nom} : {}", s.calc_no_program),
        E::DivideByZero => s.calc_divide_by_zero.to_owned(),
        E::Trailing(reste) => format!("{} {reste}", s.calc_trailing),
        E::NoMemory => s.calc_no_memory.to_owned(),
        E::Unreadable(addr) => format!("{} {addr:#x}", s.calc_unreadable),
    }
}

/// La barre de chronologie. Rend le rang du pas cliqué, s'il y en a un.
///
/// Une colonne par tranche de temps, colorée par ce qui s'y est passé de
/// plus notable. L'échelle est celle du parcours entier : la barre remplit
/// toujours toute la largeur, et la position d'un pas dedans dit sa place
/// dans le temps — non sa place dans une liste qu'on a filtrée.
fn timeline_bar(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    trace: Vec<namsdec_emu::Step>,
) -> Option<usize> {
    const HAUTEUR: f32 = 28.0;
    if trace.is_empty() {
        return None;
    }
    let largeur = ui.available_width().max(32.0);
    let (rect, réponse) = ui.allocate_exact_size(
        egui::vec2(largeur, HAUTEUR),
        egui::Sense::click_and_drag(),
    );
    let peintre = ui.painter_at(rect);
    peintre.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);

    // Une colonne par pixel au plus : dessiner plus fin ne se verrait
    // pas, et regrouper moins coûterait pour rien.
    let colonnes = timeline::columns(&trace, largeur as usize);
    let pas = largeur / colonnes.len().max(1) as f32;
    for (i, colonne) in colonnes.iter().enumerate() {
        // Un pas ordinaire reste discret : si tout criait, la barre ne
        // montrerait qu'un mur uni, ce qui est exactement ce qu'elle doit
        // éviter.
        let (couleur, part) = match colonne.kind {
            namsdec_emu::StepKind::Plain => (theme.dim.gamma_multiply(0.5), 0.35),
            namsdec_emu::StepKind::Jump => (theme.accent.gamma_multiply(0.7), 0.6),
            namsdec_emu::StepKind::Write => (theme.warn, 0.7),
            namsdec_emu::StepKind::Return => (theme.label, 0.8),
            namsdec_emu::StepKind::Call => (theme.accent, 0.9),
            namsdec_emu::StepKind::Syscall => (theme.bad, 1.0),
        };
        let barre = egui::Rect::from_min_max(
            egui::pos2(
                rect.left() + i as f32 * pas,
                rect.bottom() - part * HAUTEUR,
            ),
            egui::pos2(rect.left() + (i as f32 + 1.0) * pas, rect.bottom()),
        );
        peintre.rect_filled(barre, 0.0, couleur);
    }

    // Le repère de maintenant, au bout : c'est de là que se compte le
    // recul, et le voir évite de chercher où l'on est.
    peintre.line_segment(
        [
            egui::pos2(rect.right() - 1.0, rect.top()),
            egui::pos2(rect.right() - 1.0, rect.bottom()),
        ],
        egui::Stroke::new(2.0, theme.text),
    );

    // Cliquer ou glisser rembobine. Glisser autant que cliquer : on
    // cherche un endroit en balayant, comme sur une piste de lecteur.
    let visé = réponse.interact_pointer_pos().or(réponse.hover_pos()).map(|position| {
        timeline::step_at(trace.len(), (position.x - rect.left()) / largeur)
    });
    if let Some(index) = visé {
        // Le trait de visée, et ce qu'il désigne.
        let x = rect.left() + (index as f32 / trace.len() as f32) * largeur;
        peintre.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, theme.text),
        );
        let recul = timeline::rewind_to(trace.len(), index);
        réponse.clone().on_hover_text(format!(
            "-{recul}  ·  {:#010x}",
            trace.get(index).map(|pas| pas.addr).unwrap_or(0)
        ));
    }
    (réponse.clicked() || réponse.dragged()).then_some(visé?)
}

/// La taille de départ de la fenêtre d'analyse.
///
/// Plus large que celle des exercices : elle porte des tableaux — nom,
/// adresse, taille, appelants — qu'une colonne étroite coupe au milieu.
fn analysis_window_size(ctx: &egui::Context) -> egui::Vec2 {
    let écran = ctx.viewport_rect().size();
    egui::vec2(
        (écran.x * 0.72).clamp(560.0, 1100.0),
        (écran.y * 0.82).clamp(420.0, 900.0),
    )
}

/// La taille de départ de la fenêtre des exercices.
///
/// Proportionnée à l'écran plutôt que fixe : un chiffre en dur déborde
/// d'un petit écran et se perd sur un grand. Plafonnée, parce qu'un
/// énoncé lu sur toute la largeur d'un écran large est illisible — au
/// delà d'une certaine longueur de ligne, l'oeil ne retrouve plus le
/// début de la suivante.
fn exercises_window_size(ctx: &egui::Context) -> egui::Vec2 {
    let écran = ctx.viewport_rect().size();
    egui::vec2(
        (écran.x * 0.8).clamp(420.0, 920.0),
        (écran.y * 0.8).clamp(320.0, 720.0),
    )
}

/// Comment le programme s'est arrêté : de lui-même par `exit`, ou parce
/// que la machine a rencontré `hlt`. Les deux se voient pareil depuis
/// `StepResult::Halted`, et ne veulent pourtant pas dire la même chose.
fn halted_status(emulator: &dyn Backend) -> Status {
    if let Some(code) = emulator.exit_code() {
        return Status::Exited { code };
    }
    // Un `int3` n'est pas une fin : c'est une question posée au débogueur.
    // Dire « arrêté » tout court laisserait croire que le programme a
    // fini son travail, alors qu'il attend qu'on le relance.
    if emulator.software_breakpoint().is_some() {
        return Status::Note(|s| s.status_software_breakpoint);
    }
    Status::Halted
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

/// Ce qu'il y a à dire d'une frame, s'il y a quelque chose à dire.
///
/// Rien tant qu'elle tient dans son budget : en marche normale cette
/// instrumentation est silencieuse.
///
/// `texte_changé` distingue les deux sortes de lenteur, et la distinction
/// n'est pas cosmétique. Ouvrir un binaire de soixante mille lignes
/// demande une demi-seconde de mise en page, **une fois** : le cache
/// porte ensuite, et les frames suivantes tiennent en deux
/// millisecondes. Crier « frame lente » sans le dire laissait croire à
/// un coût permanent, et l'on cherchait une fuite là où il n'y avait
/// qu'un chargement.
fn slow_frame_note(
    total: Duration,
    spans: &[(&'static str, Duration)],
    texte_changé: bool,
) -> Option<String> {
    if total < SLOW_FRAME {
        return None;
    }
    let detail: Vec<String> = spans
        .iter()
        .filter(|(_, d)| *d >= Duration::from_millis(1))
        .map(|(name, d)| format!("{name} {}ms", d.as_millis()))
        .collect();
    let detail = if detail.is_empty() {
        "hors des étapes mesurées".to_owned()
    } else {
        detail.join(", ")
    };
    Some(if texte_changé {
        format!(
            "namsdec: mise en page — {}ms ({detail}). Le texte vient de changer : \
             c'est payé une fois, les frames suivantes n'en gardent rien.",
            total.as_millis()
        )
    } else {
        format!("namsdec: frame lente — {}ms au total ({detail})", total.as_millis())
    })
}

/// Écrit sur la sortie d'erreur ce que `slow_frame_note` a trouvé à dire.
fn report_slow_frame(total: Duration, spans: &[(&'static str, Duration)], texte_changé: bool) {
    if let Some(note) = slow_frame_note(total, spans, texte_changé) {
        eprintln!("{note}");
    }
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
            .with_app_id("namsdec")
            // La fenêtre s'ouvre déjà sans décoration quand le réglage le
            // dit : la poser d'abord avec, pour la retirer à la première
            // frame, ferait sauter la fenêtre sous les yeux.
            .with_decorations(Settings::load().system_titlebar),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
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
            // Toutes les séries lues sur le disque, celle qui est
            // livrée en tête.
            app.exercise_packs.extend(packs);
            app.exercise_files = exercise_files;
            // Les décompilateurs déclarés : on lit leur description, on
            // ne lance rien. Lancer se demande, et se demande deux fois
            // — la case du module, puis le bouton.
            app.decompilers = load_installed_decompilers();
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
            app.save_keymap_now();
            app.apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests;
