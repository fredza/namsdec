//! Coquille eframe/egui du MVP d'apprentissage NASM. Assemble via
//! `namsdec-asm` (nasm externe), désassemble via `namsdec-disasm`
//! (vérification pédagogique) et exécute pas à pas via `namsdec-emu`,
//! en affichant les panneaux des modules `ReModule` enregistrés.

mod accueil;
mod ansi;
mod barres;
mod bugreport;
mod calc;
mod calc_ui;
mod chrome;
mod completion;
mod condition;
mod console;
mod editing;
mod editor;
mod enums;
mod errors;
mod etabli;
mod exercises;
mod exports;
mod files;
mod fixes;
mod flow;
mod help;
mod i18n;
mod icons;
mod jobs;
mod keybindings;
mod labels;
mod limites;
mod listing;
mod loader;
mod marks;
mod maximize;
mod memfind;
mod menus;
mod nasm_errors;
mod notes;
mod occurrences;
mod patching;
mod preferences;
mod prevision;
mod profiling;
mod readings;
mod resources;
mod roles;
mod run;
mod scene;
mod session;
mod settings;
mod sidebar;
mod statusbar;
mod syntax;
mod text;
mod timeline;
mod tools;
mod vue_listing;
mod vues;
mod widgets;
mod windows;

// Les outils extérieurs vivent dans `tools` depuis le 29 septembre
// 2026 ; `main.rs` appelle encore celui-là directement.
use tools::load_installed_decompilers;
// L'éditeur vit dans son module depuis le 29 septembre 2026 ;
// `main.rs` a encore besoin de ces deux-là.
use editor::{editor_id, word_at};
// Le parcours d'exercices aussi ; le démarrage lit ce qui est installé.
use exercises::load_installed_exercises;
// Les fonctions sur le texte sont réclamées par la moitié des modules ;
// les rouvrir ici leur évite à chacun sa ligne d'import, puisqu'ils
// font tous `use super::*`.
use text::*;

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
use namsdec_re::{ReContext, ReModuleRegistry};
use namsdec_theme::Theme;

use enums::*;
use i18n::{Lang, Strings, strings};
use icons::*;
use keybindings::{Action, Dispatcher, KeyBindings, Rebinding};
use labels::*;
use listing::*;
// La vue qui ne pose que les lignes visibles : sa géométrie sert à la
// gouttière, aux surlignages et au curseur, quel que soit le peintre.
use profiling::*;
use vue_listing::{GUTTER_PAD, Géométrie, gutter_width};

use resources::*;
use settings::Settings;
use widgets::*;

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
/// Le plus petit écran que l'application accepte — ce que le
/// gestionnaire de fenêtres ne laissera pas réduire davantage.
///
/// Elle est ici et non écrite à la main dans les options d'eframe parce
/// qu'un invariant s'en sert : les fenêtres internes doivent tenir dans
/// CE format, et pas seulement dans celui du banc. Deux nombres écrits
/// à deux endroits auraient fini par diverger, et c'est le contrôle qui
/// se serait tu.
pub(crate) const MIN_SCREEN: [f32; 2] = [800.0, 520.0];
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

/// Le texte de la licence sous laquelle namsdec est publié.
const MIT_LICENSE_URL: &str = "https://opensource.org/license/mit";
/// Où l'on trouve namsdec, et où l'on en parle.
///
/// Trois adresses et pas une de plus : le site, le salon, le dépôt.
/// Une boîte « À propos » qui aligne dix liens ne dit plus lequel
/// compte.
const SITE_URL: &str = "https://namsdec.io";
const DISCORD_URL: &str = "https://discord.gg/pv6pHkzB5";
const GITHUB_URL: &str = "https://github.com/fredza/namsdec";
/// Où mène le bouton de don, **quand rien n'est réglé**.
///
/// VIDE, et c'est délibéré. Elle valait `https://example.com/…`, un
/// domaine réservé à la documentation où personne ne reçoit rien : le
/// bouton paraissait actif et menait nulle part, ce qui est pire qu'un
/// bouton éteint. Une adresse de paiement ne se devine pas — elle
/// appartient à qui reçoit —, et namsdec ne peut donc que la LIRE.
///
/// Elle se règle dans `settings.json`, champ `donation_url`, ou dans
/// Préférences › Général. Tant qu'elle est vide, le bouton est éteint
/// et dit où la poser.
const DONATION_URL: &str = "";

/// Les valeurs d'egui pour les infobulles, reprises telles quelles : le
/// réglage ne fait que les remplacer par un délai qui n'échoit jamais.
const TOOLTIP_DELAY: f32 = 0.5;
const TOOLTIP_GRACE_TIME: f32 = 0.2;
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
    /// Ce que les commandes ont écrit, dans l'ordre — borné en nombre
    /// de lignes ET en taille de ligne, ce qui n'était pas le cas.
    log: namsdec_asm::lanceur::Journal,
    /// Fini ? Et si oui, comment.
    outcome: std::sync::Arc<std::sync::Mutex<Option<Result<PathBuf, String>>>>,
    /// Combien d'étapes sont passées, vu depuis le fil de travail.
    progress: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// Mis à vrai pour demander l'arrêt.
    ///
    /// Il atteint maintenant l'ÉTAPE EN COURS et non plus seulement
    /// l'intervalle entre deux : le drapeau est passé au lanceur, qui
    /// le consulte toutes les vingt millisecondes et arrête le groupe
    /// de processus. Auparavant, le fil attendait la fin de l'étape —
    /// c'est-à-dire la fin de la compilation dont on voulait
    /// précisément sortir.
    ///
    /// Ce qui reste vrai, et qui est le prix de cette brutalité : une
    /// étape arrêtée au milieu peut laisser un dossier à demi
    /// décompressé. C'est pour cela que le résultat attendu est vérifié
    /// à la fin — une suite de commandes qui réussissent ne prouve pas
    /// qu'on a obtenu ce qu'on cherchait.
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

/// Un assemblage en cours dans un fil à part.
///
/// Ce qu'il porte tient en une idée : **à quoi ce résultat se
/// rapporte**. Un assemblage lancé sur un texte, rendu dix secondes
/// plus tard, n'a aucune raison de décrire encore le texte à l'écran —
/// on a pu écrire entre-temps, ou changer d'onglet. La révision tranche
/// à l'arrivée.
struct AssembleJob {
    /// La révision du texte au moment du lancement.
    rev: u64,
    /// Levé pour arrêter l'outil avant la fin.
    arrêt: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Là où le fil pose ce qu'il a obtenu.
    #[allow(clippy::type_complexity)]
    résultat: std::sync::Arc<
        std::sync::Mutex<Option<Result<namsdec_asm::AssembleOutput, namsdec_asm::AsmError>>>,
    >,
    démarré: std::time::Instant,
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

/// Ce qu'on accorde à UNE étape d'installation.
///
/// Trente minutes, et non les deux de la décompilation : ces deux
/// travaux n'ont rien de commun. Un décompilateur qui n'a pas répondu
/// en deux minutes ne répondra pas ; une compilation d'outil, elle,
/// peut légitimement durer un quart d'heure, et un téléchargement de
/// cent quatre-vingts méga-octets sur une ligne lente aussi. Leur
/// imposer le même chiffre reviendrait à tuer les secondes pour
/// surveiller les premières.
///
/// Le vrai garde-fou n'est pas ce délai : c'est le bouton d'arrêt, qui
/// rend la main en une demi-seconde depuis qu'il passe par le lanceur
/// partagé. Ce délai-ci n'est là que pour la fenêtre qu'on a oubliée
/// ouverte.
const DÉLAI_ÉTAPE: std::time::Duration = std::time::Duration::from_secs(30 * 60);

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

/// En deçà, le panneau ne montrerait plus rien de lisible.
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
///
/// Deux cent vingt auparavant, et c'était vingt de trop : sur une
/// fenêtre de 1280×820 — la taille par défaut — avec la console
/// ouverte, le bandeau n'avait plus la place de finir et la zone
/// défilante coupait une ligne en deux. Mesuré en faisant varier ce
/// nombre : la coupe ne disparaissait pas, elle se déplaçait, jusqu'à
/// ce que la ligne d'octets passe sous le repli et libère ses
/// vingt-quatre pixels. Les deux changements ensemble suffisent.
///
/// Cent quatre-vingts, et c'est un arbitrage assumé. Avec un *source*
/// assemblé — le cas de qui apprend — le bandeau ne coupe plus rien, à
/// aucune des tailles courantes. Avec un *binaire* ouvert, dont la
/// prochaine instruction tient deux lignes de plus, il manque encore une
/// ligne à 1280×820 quand la console est ouverte : aucun partage n'y
/// change rien, on ne fait que déplacer la coupe. Essayé à 170 et à
/// 160 : à 170 c'est le pseudo-code qui est rogné, à 160 le repli
/// « Détail » sort de vue. Le mur est arithmétique, pas dans le réglage.
const SIDEBAR_TAB_MIN: f32 = 180.0;

/// Et ce qui reste au bandeau, quoi qu'il arrive : de quoi lire ce qui
/// vient, et de quoi le replier.
///
/// Cent vingt et non deux cent quatre-vingts : l'ancienne valeur
/// réservait la place du bandeau ET de l'état du processeur, qui
/// partageaient la même part. Les registres ont leur onglet depuis le
/// 13 octobre 2026 ; garder 280 px pour un bandeau qui en demande 44
/// replié revenait à retenir un quart du panneau pour du vide.
const SIDEBAR_BANNER_MIN: f32 = 120.0;
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

/// Tous les combien on regarde si `keymap.json` a changé sur le disque.
///
/// Une seconde : bien en deçà du temps qu'il faut pour passer de son
/// éditeur à namsdec et essayer la touche, et bien au-delà de ce qu'un
/// appel système par image coûterait.
const KEYMAP_POLL_INTERVAL: Duration = Duration::from_secs(1);

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

/// Une condition du but d'un exercice, et où elle en est.
///
/// `constat` porte le côté gauche tel qu'il est écrit et ce qu'il vaut
/// en ce moment — de quoi dire « rax vaut 12 » sous « rax == 15 ».
/// Absent quand la condition tient (il n'y a rien à expliquer) ou quand
/// elle ne se calcule pas encore : une adresse hors des pages chargées
/// ne se devine pas, et une valeur inventée serait pire que le silence.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GoalLine {
    texte: String,
    tenue: bool,
    constat: Option<(String, u64)>,
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
    /// Le rang de ce tampon parmi les « sans titre », quand il n'a pas
    /// de fichier. Trois tampons neufs s'appelaient tous « sans titre »,
    /// et l'on ne savait plus lequel on regardait.
    untitled_no: usize,
}

impl OpenTab {
    /// Le nom qui s'écrit sur l'onglet : celui du fichier, le titre de
    /// l'exercice s'il n'y a pas de fichier, et « sans titre » sinon.
    fn label(&self, s: &Strings, packs: &[namsdec_exercises::Pack], lang: Lang) -> String {
        tab_label(
            self.path.as_deref(),
            self.current_exercise.as_deref(),
            self.assemble_state
                .as_ref()
                .and_then(|state| state.from_binary.as_ref())
                .map(|binaire| binaire.name.as_str()),
            s,
            packs,
            lang,
            self.untitled_no,
        )
    }

    fn modifié(&self) -> bool {
        self.source != self.saved_source
    }
}

/// Où le curseur est, et ce qui le rendait vrai.
///
/// Relever ligne, colonne et octet demande une passe sur le texte qui
/// précède le curseur ; la question se pose deux fois par image, et la
/// réponse ne change que si le curseur ou le texte a bougé. Voir
/// `NamsdecApp::cursor_place`.
#[derive(Clone, Copy, PartialEq, Eq)]
struct CurseurVu {
    /// La ligne, la colonne, et l'octet.
    place: (usize, usize, usize),
    /// Le rang du caractère, la longueur du texte, et le nombre de poses
    /// du listing : si les trois sont les mêmes, la place l'est aussi.
    clé: (usize, usize, u64),
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
    /// Le poids du code, en octets — calculé une fois, à la demande.
    ///
    /// L'onglet Octets l'affiche, et le recalculait à chaque image : sur
    /// un million d'instructions, c'est une somme d'un million de
    /// termes soixante fois par seconde pour un nombre qui ne change
    /// jamais. Il ne peut pas vieillir : un listing qui change — un
    /// correctif écrit, un autre binaire ouvert — reconstruit tout cet
    /// état, cellule comprise.
    octets_du_code: std::cell::OnceCell<usize>,
}

impl AssembleState {
    /// Le poids du code, en octets.
    fn octets_du_code(&self) -> usize {
        *self
            .octets_du_code
            .get_or_init(|| self.disasm.iter().map(|i| i.bytes.len()).sum())
    }
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
    /// Combien de fois le texte de l'éditeur a changé.
    ///
    /// Plusieurs relevés coûteux ne valent que pour un texte donné : où
    /// tombe le curseur, où sont les occurrences d'une recherche,
    /// quelles instructions la fenêtre de désassemblage retient. Les
    /// garder d'une image à l'autre demande de savoir quand le texte a
    /// changé, et la seule réponse à la fois exacte et gratuite est un
    /// compteur qu'on incrémente EN L'ÉCRIVANT.
    ///
    /// Le listing tenait lieu de réponse jusqu'ici : il était mis en
    /// page d'un bloc, et le nombre de poses disait quand le texte avait
    /// bougé. La vue qui ne pose que les lignes visibles lui retire ce
    /// rôle — elle ne voit plus le texte entier.
    ///
    /// `texte_a_changé` est le seul endroit qui l'incrémente, et le
    /// contrôle `tout_ce_qui_ecrit_le_texte_le_dit` vérifie que personne
    /// n'écrit `source` sans passer par là.
    texte_rev: u64,
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
    /// La rangée des arguments est-elle dépliée ? Voir
    /// `Settings::console_args_open`.
    console_args_open: bool,
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
    /// Quand le dernier pas a rendu la main : de quoi faire clignoter en
    /// orange, le temps d'un battement, ce qu'il a changé. Une valeur qui
    /// change sans bouger à l'écran se rate ; un bref éclat la désigne,
    /// puis s'efface pour ne pas peser. `None` tant que rien n'a tourné.
    values_changed_at: Option<std::time::Instant>,
    /// Un pas vient d'avancer : pendant les quelques rendus que ce compteur
    /// décompte, l'éditeur tient son défilement horizontal à zéro.
    ///
    /// Sans quoi, si l'on avait laissé le curseur en fin d'une longue
    /// ligne — la zone de défilement s'était alors portée à droite pour le
    /// montrer —, avancer d'un pas gardait cette vue : la ligne courante
    /// restait visible en hauteur, mais le code et la gouttière étaient
    /// sortis à gauche, et l'on ne voyait plus que les commentaires.
    ///
    /// Tenu quelques images, et non une seule, pour absorber l'élan d'un
    /// défilement cinétique (molette lancée) qui, sinon, reprendrait la
    /// vue vers la droite juste après. Bref, pour ne pas empêcher de
    /// défiler à la main ensuite.
    editor_follow: u8,
    /// Le conseil qui accompagne la dernière erreur d'exécution : par où
    /// reprendre, quand l'erreur appelle un geste précis.
    ///
    /// Une chaîne possédée, et non statique : certains conseils nomment
    /// ce sur quoi on vient de buter — la fonction de bibliothèque qu'on
    /// appelait — et ce nom vient du fichier ouvert.
    error_advice: Option<String>,
    /// L'instruction que le dernier pas vient d'exécuter. `None` après
    /// un Run, un retour en arrière ou une réinitialisation — il n'y a
    /// plus « un » dernier pas à montrer.
    last_step: Option<DernierPas>,
    /// Les octets de la fenêtre mémoire avant le dernier pas, pour
    /// mettre en avant ceux qui ont changé.
    mem_before: Option<Vec<u8>>,
    re_registry: ReModuleRegistry,
    status: Status,
    /// L'assemblage qui tourne en fond, s'il y en a un.
    assemble_job: Option<AssembleJob>,
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
    /// Les octets qu'on a changés dans le binaire chargé, du plus ancien
    /// au plus récent.
    ///
    /// Gardés avec ce qu'il y avait avant : un patch qu'on ne peut pas
    /// défaire n'est pas un outil d'analyse, c'est une décision
    /// définitive — et l'on patche justement pour *essayer*.
    patches: Vec<namsdec_re::patch::Patch>,
    /// La fenêtre des patchs est-elle ouverte ?
    show_patches: bool,
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
    /// La famille de fonctions ouverte dans la calculatrice.
    ///
    /// Quarante-sept pastilles tiennent sur douze lignes, et douze
    /// lignes de plus poussaient le bas de la fenêtre hors de l'écran.
    /// Une famille à la fois, donc, et celle qu'on consulte est celle
    /// qu'on retrouve au retour : on ne travaille pas sur les bits un
    /// jour et sur les rotations le lendemain par hasard.
    calc_family: String,
    /// Combien de fois le texte avait été posé au début de la frame.
    ///
    /// Sert à distinguer les deux sortes de frames lentes : celle d'une
    /// mise en page qu'on paie une fois, et celle qui reviendrait à
    /// chaque image. Le compte vient du cache de mise en page lui-même,
    /// qui sait s'il a payé — la longueur du texte, elle, ne le devinait
    /// qu'approximativement.
    poses_au_début: u64,
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
    /// La ligne de lecture d'une séance précédente, à ramener sous les
    /// yeux à l'ouverture.
    ///
    /// Distincte de `goto_line`, qui est un GESTE : celui-ci pose le
    /// curseur, passe en Insertion et prend le clavier, ce qui est juste
    /// quand on vient de demander « aller à la ligne » et faux au
    /// démarrage — on rouvre un binaire pour le LIRE, et « INSERT » dans
    /// la barre d'état au premier regard serait une surprise.
    reprise_ligne: Option<u32>,
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
    /// Les occurrences de la recherche, situées une fois pour toutes —
    /// voir `refresh_find_hits`.
    find_hits: Vec<Place>,
    /// Où le curseur est, et pour quelle question (voir `cursor_place`).
    cursor_place_vu: Option<CurseurVu>,
    /// Ce que la vue ligne à ligne sait du texte entier, et la clé qui
    /// le valide : la longueur du texte et sa révision.
    relevé_du_texte: Option<(vue_listing::ReleveDuTexte, (usize, u64))>,
    /// Ce que l'image en cours sait du tampon : modifié, ou non.
    ///
    /// La question se pose partout dans une image — la barre d'état, le
    /// titre, les onglets, les boutons qui se grisent — et chacun
    /// comparait DEUX TEXTES ENTIERS. Sur un listing de seize
    /// mégaoctets, c'est une milliseconde par question, et c'était la
    /// moitié du temps d'une image (mesuré le 14 octobre 2026).
    ///
    /// La comparaison se fait donc une fois, au début de l'image, et
    /// tout l'affichage lit la même réponse. Ce qui DÉCIDE — enregistrer,
    /// fermer, quitter — continue de comparer sur le moment : une image
    /// de retard ne se voit pas à l'écran, mais écraser un fichier sur
    /// une réponse périmée se verrait.
    modifié_vu: bool,
    /// La même réponse pour chaque onglet laissé de côté, dans leur
    /// ordre.
    tabs_modifiés_vus: Vec<bool>,
    /// Ce qui les a produites : la requête, la longueur du texte, et le
    /// nombre de poses du listing.
    find_hits_clé: Option<(String, usize, u64)>,
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
    ///
    /// Gardé même sans l'option `reseau` : le champ ne coûte rien, et le
    /// borner ferait un `cfg` de plus dans `Default`, dans les réglages
    /// et dans trois contrôles — beaucoup de bruit pour huit octets.
    #[cfg_attr(not(feature = "reseau"), allow(dead_code))]
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
    /// L'outil qu'une rubrique d'analyse a demandé, et son fil.
    ///
    /// À part du travail de la fenêtre « Décompilation » : les deux
    /// peuvent tourner en même temps, et surtout ils ne rendent pas au
    /// même endroit — celui-ci revient au panneau qui l'a demandé, et à
    /// lui seul.
    tool_job: Option<(String, mpsc::Receiver<Result<String, String>>)>,
    /// Le geste UPX en cours, le fichier où il doit écrire, et son fil.
    ///
    /// À part de `tool_job` : celui-là porte un **moteur de
    /// désassemblage** qu'on a activé, celui-ci un outil qui écrit un
    /// fichier. Les deux peuvent tourner en même temps et ne rendent
    /// pas au même endroit.
    upx_job: Option<UpxJob>,
    /// Ce qui tourne à côté, pour la barre d'état et le titre de la
    /// fenêtre : un seul travail à la fois suffit à ce que namsdec fait.
    job: Option<BackgroundJob>,
    /// Le titre posé en dernier, pour ne pas le réécrire à chaque image :
    /// un `ViewportCommand` par frame réveille le gestionnaire de
    /// fenêtres pour rien.
    window_title: Option<String>,
    decompile_result: Option<Result<String, String>>,
    /// La dernière décompilation a-t-elle été demandée depuis le panneau
    /// DZF ? Si oui, on lui livre la réponse (ou l'échec), JSON ou pas.
    dzf_wanted: bool,
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
    /// La date de `keymap.json` telle qu'on l'a lue ou écrite.
    ///
    /// Elle sert à repérer une modification venue d'ailleurs — un
    /// éditeur de texte, un dépôt qu'on synchronise. Sans elle, le
    /// fichier n'était repris qu'au lancement suivant, et rien ne le
    /// disait : on éditait, on essayait la touche, il ne se passait
    /// rien.
    keymap_mtime: Option<std::time::SystemTime>,
    /// Ce que chaque outil connu répond, relevé une fois.
    ///
    /// Savoir si un outil est là et complet demande de **le lancer** :
    /// `rizin` sans `rz-ghidra` a le programme et pas la commande, et on
    /// le lui demande plutôt que de le supposer. Cette réponse était
    /// redemandée à chaque image — ouvrir Préférences › Outils lançait
    /// donc `rizin` soixante fois par seconde, ce qui saturait un cœur
    /// et faisait hésiter l'interface. Et deux réponses successives
    /// pouvaient différer sous la charge, ce qui rendait le test de la
    /// fenêtre d'installation instable : c'est par là qu'on l'a vu.
    ///
    /// Vide tant qu'on n'a pas demandé. Rempli d'un coup, et refait
    /// quand quelque chose a pu changer : une installation qui
    /// s'achève, une fenêtre qui s'ouvre, ou le bouton « Revérifier ».
    tools_ready: std::collections::BTreeMap<String, namsdec_disasm::known::Availability>,
    /// Quand on a regardé la date pour la dernière fois.
    ///
    /// Interroger le disque soixante fois par seconde pour un fichier
    /// qu'on modifie deux fois par an serait absurde ; une fois par
    /// seconde suffit à ce qu'un essai au clavier vienne après la
    /// sauvegarde.
    keymap_checked: Option<std::time::Instant>,
    dispatcher: Dispatcher,
    /// Depuis quand une séquence est armée, pour ne montrer les suites
    /// possibles qu'après un temps d'hésitation (`LEADER_HINT_DELAY`).
    ///
    /// Remis à zéro dès que la séquence se termine ou s'abandonne : le
    /// panneau ne doit pas reparaître instantanément à la séquence
    /// suivante sous prétexte qu'on a hésité à la précédente.
    leader_since: Option<std::time::Instant>,
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
    /// L'interface est-elle réduite à l'essentiel ?
    ///
    /// Voir `Settings::debutant` : neuf rubriques d'analyse au lieu de
    /// vingt-cinq, pour que namsdec s'ouvre sur ce qui s'apprend. Rien
    /// n'est désinstallé — les panneaux cachés continuent de travailler
    /// pour le compte rendu « Tout », et la case les rend d'un clic.
    debutant: bool,
    /// Faut-il prévoir ce qu'une instruction va faire avant de
    /// l'exécuter ?
    ///
    /// Voir `Settings::prevoir` et `crate::prevision` : la réponse
    /// attend le pas, au niveau débutant seulement. Sans effet au
    /// niveau avancé, et le réglage le dit en se grisant.
    prevoir: bool,
    /// La console a-t-elle déjà été ouverte pour la sortie de CE
    /// programme ?
    ///
    /// Une fois par programme : qui la referme en pleine course a
    /// choisi, et on ne la lui rouvre pas sous les doigts. Voir
    /// `montrer_la_console_si_le_programme_ecrit`.
    console_ouverte_pour_la_sortie: bool,
    /// La réponse que l'on écrit à la question d'un exercice de lecture.
    ///
    /// De la session, pas du disque : une réponse à moitié tapée n'a
    /// rien à survivre à la fermeture, et la remettre au lancement
    /// suivant priverait du geste qu'on vient apprendre.
    question_answer: String,
    /// L'adresse dont la réponse a été demandée par « Montrer ».
    ///
    /// Une ADRESSE et non un drapeau : au pas suivant, la question se
    /// repose d'elle-même. Un booléen aurait éteint la prévision pour
    /// toute la séance au premier clic — et aurait marché à l'écran,
    /// ce qui est le propre des défauts qu'on ne trouve pas.
    prevision_révélée: Option<u64>,
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
    /// Le clavier doit-il revenir à l'expression à la prochaine image ?
    ///
    /// Un clic sur une touche du clavier, sur un bit ou sur une fonction
    /// RETIRE le focus au champ de saisie — ce sont des surfaces
    /// cliquables qui ne le prennent pas, et egui rend alors le focus à
    /// personne. Le champ muet, Ctrl+V ne collait plus rien et Ctrl+C ne
    /// copiait plus rien : la calculatrice perdait le presse-papiers dès
    /// la première touche cliquée, ce qui est exactement le moment où
    /// l'on s'en sert.
    ///
    /// Relevé à la fin d'une image et agi à la suivante : le focus se
    /// rend pendant l'image, et le redemander avant que ce soit fait ne
    /// tient pas.
    calc_reprendre_clavier: bool,
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
    /// Les fenêtres internes qu'on a agrandies.
    ///
    /// Une `egui::Window` n'a pas les pastilles du système : elle se
    /// traîne et se redimensionne à la main, ce qui est long quand on
    /// veut simplement tout voir. Un identifiant par fenêtre plutôt
    /// qu'un booléen par fenêtre : le mécanisme sert à toutes, et
    /// n'ajoute rien à la structure quand on en ouvre une de plus.
    maximized_windows: std::collections::BTreeSet<&'static str>,
    /// Où chaque fenêtre agrandissable se tenait AVANT de grandir.
    ///
    /// Relevé à chaque image tant qu'elle est normale : c'est la seule
    /// mesure qui dise où l'utilisateur l'avait mise et de quelle
    /// taille il l'avait faite. Sans elle, « Restaurer » ne savait pas
    /// à quoi revenir — et ne revenait à rien.
    /// Le rectangle ET le pivot : egui recalcule le coin haut gauche à
    /// partir du pivot, et celui qu'il garde après un agrandissement
    /// est le nôtre — « coin haut gauche » —, pas celui de la fenêtre.
    /// Relevés ensemble, ils disent où elle était vraiment.
    rects_avant_agrandissement:
        std::collections::BTreeMap<&'static str, (egui::Rect, egui::Align2)>,
    /// Les fenêtres à qui réimposer ce rectangle, le temps d'une image.
    à_restaurer: std::collections::BTreeSet<&'static str>,
    /// N'afficher que la fonction où l'on se trouve.
    ///
    /// Vrai par défaut : sur un binaire ordinaire le graphe entier
    /// compte des milliers de blocs, où l'on ne trouve rien. La fonction
    /// courante est ce qu'on regarde neuf fois sur dix.
    flow_current_only: bool,
    /// Le pseudo-code structuré à la place du graphe.
    ///
    /// Le graphe dit par où l'on passe ; le pseudo-code dit ce qui se
    /// passe. Un `jne` devient « si égal », une arête qui remonte devient
    /// « tant que » — c'est le chaînon que tout le monde franchit de
    /// tête après quelques semaines, et qui manque les premiers jours.
    flow_pseudo: bool,
    /// Le grossissement du dessin, réglé à la molette.
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
    /// Les deux bandes de l'Établi qu'on a sorties dans une fenêtre.
    ///
    /// De la session, comme les vues détachées de la rangée : on sort
    /// une bande pour lire ce qui n'y tient pas, et on la referme.
    /// Voir `etabli::etabli_fenetres_ui` pour le pourquoi — la pile
    /// d'un vrai binaire et les explications de namsdec ne tiennent pas
    /// toujours dans une place répartie d'avance.
    etabli_machine_detachee: bool,
    etabli_resultat_detache: bool,
    /// La Scène : la machine en grand, pour regarder un pas se faire.
    ///
    /// De la session et non du disque, comme les autres fenêtres : on
    /// l'ouvre pour suivre un passage, on la referme après. Voir
    /// `scene.rs` pour ce qu'elle montre et pourquoi elle n'est pas
    /// une vue.
    show_scene: bool,
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
    /// Ce que la zone centrale montre : la source, ou l'une des vues
    /// qui ont quitté les fenêtres flottantes pour la rangée.
    vue_centrale: VueCentrale,
    /// Le désassemblage DANS SA FENÊTRE, c'est-à-dire détaché de la
    /// rangée. Fermer la fenêtre l'y ramène : `vue_détachée` lit ce
    /// booléen, et la rangée s'en déduit.
    show_disassembly: bool,
    /// Le moteur que le désassemblage emploie.
    disassembler: String,
    /// Ce qu'on cherche dans le listing.
    disassembly_query: String,
    /// Les rangs des instructions que la fenêtre de désassemblage
    /// montre, dans l'ordre — voir `refresh_disassembly_rows`.
    ///
    /// Des rangs et non des lignes toutes faites : une ligne demande de
    /// redécoder l'instruction et de composer deux chaînes, et la zone
    /// n'en affiche qu'une quarantaine. Un `u32` suffit — quatre
    /// milliards d'instructions, c'est plus que ce qu'un fichier peut
    /// porter.
    disassembly_rows: Vec<u32>,
    /// La question à laquelle ces rangs répondent : le filtre, le point
    /// de départ, le moteur, la langue, le nombre d'instructions, et la
    /// révision du texte — laquelle change quand une fonction est
    /// renommée, ce que le filtre voit aussi.
    disassembly_rows_clé: Option<(String, Option<u64>, String, Lang, usize, u64)>,
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
    /// Les binaires d'outils localisés à la main (identifiant → chemin
    /// absolu). Pour ce qu'on a bâti soi-même, hors du PATH.
    located_tools: BTreeMap<String, std::path::PathBuf>,
    /// L'outil dont on est en train de choisir le binaire, le temps que le
    /// sélecteur de fichier rende sa réponse.
    locating_tool: Option<String>,
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
    /// Le facteur d'échelle de l'écran à l'image précédente : quand il
    /// change, toute la fenêtre est à repeindre (voir `draw`).
    dernière_échelle: Option<f32>,
    /// La zone de travail à l'image précédente : ce qui reste une fois
    /// les barres posées.
    ///
    /// Relevée pendant le dessin, employée après par les fenêtres qui
    /// s'agrandissent sans recouvrir les barres. egui ne la rend pas :
    /// le contexte connaît l'écran entier, et ce qu'un panneau a pris
    /// n'est su que du `Ui` qui le suit.
    zone_de_travail: egui::Rect,
    /// L'instruction dont on récrit les octets, s'il y en a une, et ce
    /// qui est tapé. L'adresse et non le rang : la liste est reconstruite
    /// à chaque rechargement, les rangs bougent, l'adresse non.
    bytes_edit: Option<u64>,
    bytes_edit_input: String,
    /// Le rang du tampon courant parmi les « sans titre ».
    untitled_no: usize,
    /// Le prochain rang à distribuer. Il ne redescend jamais : fermer
    /// « sans titre 2 » ne fait pas du suivant un second « sans titre 2 »
    /// pendant qu'on regarde encore le premier.
    next_untitled: usize,
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
    ///
    /// L'onglet actif n'y est pas : c'est l'état de travail lui-même.
    tabs: Vec<OpenTab>,
    /// On a fermé le dernier fichier : l'accueil prend la place de
    /// l'éditeur (voir `accueil.rs`).
    ///
    /// UN DRAPEAU, ET NON LA SEULE VUE DU TAMPON VIDE. Déduire l'accueil
    /// de « le texte est vide et aucun fichier n'est ouvert » le ferait
    /// apparaître au milieu d'une frappe — on sélectionne tout, on
    /// efface pour recommencer, et l'éditeur disparaîtrait sous les
    /// doigts. L'accueil se demande, par un geste : fermer.
    ///
    /// Il ne se retient pas d'une séance à l'autre : au démarrage, le
    /// programme d'accueil est chargé, et un éditeur qui a du texte
    /// n'est pas un accueil.
    montrer_accueil: bool,
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
    /// Les barres d'icônes des bords (voir `barres.rs`).
    ///
    /// Elles ne commandent rien que le menu Affichage ne commande
    /// déjà ; ce qu'elles font est de RENDRE VISIBLE CE QUI EXISTE. On
    /// peut les éteindre — le menu les a —, et il n'y a alors plus
    /// aucune trace à l'écran d'un panneau fermé, ce qui était l'état
    /// d'avant.
    show_rails: bool,
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
    /// Les mêmes étendues, situées en lignes et en colonnes : c'est
    /// sous cette forme qu'elles se peignent (voir `text::Place`).
    tracked_places: Vec<Place>,
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
    /// Le compteur tapé avant un mouvement, en mode Normal — le `5` de
    /// `5j`.
    ///
    /// Gardé d'une image à l'autre, comme `g_pending` : les chiffres
    /// s'accumulent (`12j` descend de douze lignes) et le premier
    /// mouvement les consomme. Tout le reste les efface — un compteur
    /// oublié qui s'appliquerait à un mouvement tapé une minute plus
    /// tard serait pire que pas de compteur du tout.
    count_pending: Option<u32>,
    /// Le nom sous lequel enregistrer le jeu de raccourcis courant.
    keymap_save_name: String,
    /// Où mène le bouton de don. Vide : il est éteint, et dit pourquoi.
    donation_url: String,
    /// La fenêtre « Signaler une anomalie » est-elle ouverte ?
    show_bug_report: bool,
    /// Ce qu'on y a saisi.
    bug_report: bugreport::BugReport,
    /// La fiche d'identité du binaire, pour l'onglet RE — relevée une
    /// fois par fichier, et gardée.
    ///
    /// La clé est ce qui la périme : la taille du fichier et l'adresse
    /// de chargement. Deux binaires différents ne partagent pas les
    /// deux, et un détour posé recharge le fichier, donc la refait.
    re_bref: Option<((usize, u64), ReBref)>,
    /// L'ANCRE du mode Visuel : l'octet où la sélection a commencé.
    ///
    /// Le curseur, lui, est `cursor_byte` et bouge avec les mouvements.
    /// La sélection est l'intervalle entre les deux, dans l'ordre où ils
    /// se trouvent — on peut étendre vers le haut comme vers le bas,
    /// comme dans vim.
    ///
    /// `None` hors du mode Visuel, et remis à `None` en en sortant :
    /// une ancre oubliée ferait surligner au hasard à la prochaine
    /// entrée en Visuel.
    visual_anchor: Option<usize>,
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

/// Le geste UPX en cours : ce qu'on lui demande, où il doit écrire, et
/// le fil qui attend sa réponse.
///
/// Nommé plutôt qu'écrit en place : un triplet dont le troisième membre
/// est un `Receiver` d'un `Result` de deux chaînes ne se lit plus au
/// milieu d'une déclaration de champ.
type UpxJob = (
    namsdec_re::UpxAction,
    Option<PathBuf>,
    mpsc::Receiver<Result<String, String>>,
);

/// LA FICHE D'IDENTITÉ D'UN BINAIRE, telle que l'onglet RE la peint.
///
/// L'onglet ne portait qu'un bouton — « Analyse du binaire… » — et deux
/// mille pixels de vide sous lui. Tout ce qu'il savait vivait derrière
/// une fenêtre qu'il fallait ouvrir, et un onglet dont le seul contenu
/// est « ouvrir une fenêtre » ne mérite pas la place qu'il occupe.
///
/// Ce qu'il y a ici est ce qu'on veut savoir SANS ouvrir quoi que ce
/// soit : de quoi on parle, qui l'a produit, comment il est armé, ce
/// qu'il sait faire. Le détail reste dans la fenêtre, et le bouton y
/// mène toujours.
///
/// Relevé une fois par fichier : `elf_scan`, la reconnaissance de
/// chaîne d'outils et le relevé des appels lisent le fichier entier, et
/// le refaire par image coûterait une image par seconde sur un gros
/// binaire.
#[derive(Clone, Debug, Default)]
pub(crate) struct ReBref {
    /// La taille du fichier, en octets.
    taille: usize,
    /// « ELF64 · PIE · dyn », ou ce qu'on en sait.
    forme: String,
    /// Le langage et son degré de certitude.
    langage: Option<(String, String)>,
    /// Le compilateur et son degré de certitude.
    compilateur: Option<(String, String)>,
    /// Les quatre protections qui décident de tout.
    protections: Vec<(&'static str, bool)>,
    /// Ce qu'il sait faire, par famille, avec le nombre d'appels.
    comportement: Vec<(namsdec_re::behaviour::Family, usize)>,
    /// Fonctions, chaînes, fonctions empruntées, appels système.
    chiffres: (usize, usize, usize, usize),
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
    /// Choisir où écrire le graphe du programme, en DOT.
    ExportDot,
    /// Choisir où écrire le jeu de raccourcis **entier**, à plat.
    ExportKeymap,
    /// Choisir le binaire d'un outil bâti hors du PATH (DZF, par exemple).
    /// L'outil visé est retenu à part, dans `locating_tool`.
    LocateTool,
    /// Choisir où écrire l'original déballé par `upx -d` — un fichier
    /// NEUF, et c'est tout l'objet de la question : `upx -d fichier`
    /// écrase l'échantillon en place, et l'on perd ce qu'on analysait.
    UnpackUpx,
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
    /// La Scène : la machine en grand (voir `scene.rs`).
    ShowScene,
    SetTheme(&'static Theme),
    SetLang(Lang),
    SaveAs,
    Quit,
    GoTab(SidebarTab),
    /// Aller à une étiquette du programme, par son nom.
    GoLine(u32),
    GoConsoleTab(ConsoleTab),
    ToggleToolbar,
    ToggleStatusbar,
}

/// Ce que le dernier pas a exécuté : l'adresse, l'instruction, et ce
/// qu'elle ANNONÇAIT.
///
/// L'annonce est prise avant le pas, comme le reste : c'est elle que
/// « prévoir avant d'avancer » avait retenue, et la relire à côté de ce
/// qui a changé est tout l'intérêt du chemin. Voir
/// [`crate::prevision`].
#[derive(Debug, Clone)]
struct DernierPas {
    addr: u64,
    texte: String,
    /// Le pseudo-code en clair, quand il dit plus que l'instruction.
    annonce: Option<String>,
}

struct PreferencesSnapshot {
    decompiler_module: bool,
    installer_module: bool,
    theme: &'static Theme,
    lang: Lang,
    show_re_tab: bool,
    debutant: bool,
    prevoir: bool,
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

impl PreferencesTab {
    /// Les cinq pages, dans l'ordre où elles s'affichent.
    ///
    /// Déclarées une fois : le banc de mise en page les parcourt toutes,
    /// et une page nouvelle s'y trouve contrôlée sans qu'on y pense.
    /// Elles ne l'étaient pas — seule la première s'ouvrait, et les
    /// quatre autres n'avaient jamais été mesurées.
    pub(crate) const ALL: [PreferencesTab; 5] = [
        PreferencesTab::General,
        PreferencesTab::Shortcuts,
        PreferencesTab::Modules,
        PreferencesTab::Plugins,
        PreferencesTab::Tools,
    ];

    /// Le nom de l'onglet, dans la langue de l'interface.
    pub(crate) fn label(self, s: &'static Strings) -> &'static str {
        match self {
            PreferencesTab::General => s.preferences_tab_general,
            PreferencesTab::Shortcuts => s.preferences_tab_shortcuts,
            PreferencesTab::Modules => s.preferences_tab_modules,
            PreferencesTab::Plugins => s.preferences_tab_plugins,
            PreferencesTab::Tools => s.preferences_tab_tools,
        }
    }

    /// Un nom court, pour nommer la scène d'un échec du banc.
    ///
    /// Réservé aux contrôles : à l'écran, une page se nomme par son
    /// libellé traduit, non par cet identifiant.
    #[cfg(test)]
    pub(crate) fn id(self) -> &'static str {
        match self {
            PreferencesTab::General => "general",
            PreferencesTab::Shortcuts => "raccourcis",
            PreferencesTab::Modules => "modules",
            PreferencesTab::Plugins => "plugins",
            PreferencesTab::Tools => "outils",
        }
    }
}

/// Programme de départ affiché au lancement, pour ouvrir sur un exemple
/// qui fonctionne plutôt qu'un éditeur vide.
///
/// Ce n'est pas un exercice : ceux-là sont un module à part, éteint tant
/// qu'on ne le demande pas, et l'application doit s'ouvrir sur quelque
/// chose qui marche même quand il ne l'est pas.
const DEFAULT_SOURCE: &str = "\
; Bienvenue dans namsdec. Assemblez (Ctrl+Maj+B), puis avancez pas à pas.
; Un parcours de cent dix-sept exercices vous attend : Préférences > Modules.

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

/// Le moteur qui montre la syntaxe du SYSTÈME — celle de gdb, d'objdump
/// et de as.
///
/// Livré au même titre que les deux autres, et pour la même raison
/// qu'eux : ce n'est pas un outil extérieur, c'est une façon de lire ce
/// qu'on a déjà. Dès qu'on sort de namsdec — un rapport d'outil, une
/// session gdb, un tutoriel trouvé en ligne —, tout est écrit ainsi, et
/// la traduction de tête est la marche où l'on abandonne.
const SYSTEM_ID: &str = "system";
/// Le désassembleur extérieur que le niveau débutant garde : `objdump`
/// en syntaxe Intel.
///
/// C'est celui qu'on a sous la main sur toute machine — il vient avec
/// `binutils` —, et sa syntaxe Intel est celle qu'on vient d'apprendre
/// en écrivant du nasm. Les autres moteurs extérieurs se choisissent
/// quand on sait ce qu'ils font de différent.
const OBJDUMP_ID: &str = "objdump";

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
        // L'ORDRE DES PANNEAUX VIT DANS `namsdec-re`, depuis P7 :
        // c'est une propriété de l'ANALYSE — de quoi on parle, puis
        // ce qui en ressort, puis le code, puis les octets — et non
        // de l'interface qui la montre. Un rapport la suit, et le
        // raisonnement de chaque rang se lit auprès de lui, dans
        // `ReModuleRegistry::standard`.
        let re_registry = ReModuleRegistry::standard();

        Self {
            source: DEFAULT_SOURCE.to_string(),
            texte_rev: 0,
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
            values_changed_at: None,
            editor_follow: 0,
            error_advice: None,
            last_step: None,
            mem_before: None,
            re_registry,
            status: Status::Idle,
            assemble_job: None,
            diagnostics: Vec::new(),
            // Le processeur en premier : c'est ce qu'on regarde entre
            // deux pas, et c'était jusqu'ici un bandeau toujours
            // visible. Devenu onglet, il doit être celui qu'on trouve
            // en ouvrant.
            active_tab: SidebarTab::Cpu,
            console_tab: ConsoleTab::Output,
            mem_addr_input: format!("{BASE_ADDR:#x}"),
            analysis_module: String::new(),
            input_dialog_text: String::new(),
            input_dialog_focus: false,
            patches: Vec::new(),
            show_patches: false,
            show_input_dialog: false,
            was_waiting_for_input: false,
            calc_base: CalcBase::Hex,
            calc_width: CalcWidth::W64,
            calc_family: calc::FAMILIES[0].0.to_owned(),
            poses_au_début: 0,
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
            reprise_ligne: None,
            pending_cursor: None,
            find: None,
            find_hits: Vec::new(),
            cursor_place_vu: None,
            relevé_du_texte: None,
            modifié_vu: false,
            tabs_modifiés_vus: Vec::new(),
            find_hits_clé: None,
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
            tool_job: None,
            upx_job: None,
            job: None,
            window_title: None,
            decompile_result: None,
            dzf_wanted: false,
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
            tools_ready: std::collections::BTreeMap::new(),
            keymap_mtime: None,
            keymap_checked: None,
            dispatcher: Dispatcher::default(),
            leader_since: None,
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
            // FAUX ICI, VRAI DANS `Settings::default()`, et la
            // différence est voulue.
            //
            // Ce `Default`-ci n'est pas ce qu'un utilisateur reçoit :
            // l'application vient toujours de ses réglages
            // (`session.rs`), et un fichier neuf porte `debutant:
            // true` — une installation fraîche s'ouvre donc sur ce qui
            // s'apprend. Ce `Default`-ci sert aux contrôles et au
            // banc, qui doivent mesurer l'interface ENTIÈRE : sinon
            // seize rubriques sortiraient d'un coup de toute mesure,
            // ce qui est exactement le trou qu'on a passé la séance à
            // reprocher ailleurs. Une scène du banc allume le mode
            // débutant pour le mesurer lui aussi.
            debutant: false,
            exercises: false,
            show_exercises: false,
            show_calculator: false,
            calc_input: String::new(),
            calculator_was_open: false,
            calc_reprendre_clavier: false,
            show_help: false,
            show_analysis: false,
            show_vectors: false,
            show_breakpoints: false,
            show_timeline: false,
            show_flow: false,
            maximized_windows: std::collections::BTreeSet::new(),
            rects_avant_agrandissement: std::collections::BTreeMap::new(),
            à_restaurer: std::collections::BTreeSet::new(),
            flow_current_only: true,
            flow_pseudo: false,
            flow_zoom: 1.0,
            flow_offset: egui::Vec2::ZERO,
            flow_fit: false,
            flow_goal: String::new(),
            show_cyclic: false,
            etabli_machine_detachee: false,
            etabli_resultat_detache: false,
            show_scene: false,
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
            vue_centrale: VueCentrale::Source,
            show_disassembly: false,
            disassembler: PSEUDO_ID.to_owned(),
            disassembly_query: String::new(),
            disassembly_rows: Vec::new(),
            disassembly_rows_clé: None,
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
            located_tools: BTreeMap::new(),
            locating_tool: None,
            current_exercise: None,
            settings_file: None,
            dernière_échelle: None,
            zone_de_travail: egui::Rect::NOTHING,
            bytes_edit: None,
            bytes_edit_input: String::new(),
            untitled_no: 1,
            next_untitled: 2,
            next_instruction_open: true,
            // VRAI, comme dans `Settings::default()`, et sans effet
            // ici : la prévision ne vaut qu'au niveau débutant, qui
            // est faux dans ce `Default`-ci (voir plus haut). Les
            // contrôles et le banc mesurent donc l'interface telle
            // qu'elle a toujours été, et ceux de la prévision allument
            // les deux cases.
            console_ouverte_pour_la_sortie: false,
            prevoir: true,
            question_answer: String::new(),
            prevision_révélée: None,
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
            console_args_open: false,
            console_input_released: false,
            resume_run_after_input: false,
            explorer: FileExplorer::default(),
            show_explorer: false,
            explorer_just_opened: false,
            explorer_focused: false,
            current_file: None,
            tabs: Vec::new(),
            montrer_accueil: false,
            file_mtime: None,
            disk_changed: false,
            had_focus: true,
            restoring_session: false,
            active_tab_index: 0,
            show_toolbar: true,
            show_rails: true,
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
            tracked_places: Vec::new(),
            tracked_source_len: 0,
            editor_selection: None,
            rename: None,
            rename_just_opened: false,
            note_edit: None,
            note_just_opened: false,
            pending_selection: None,
            g_pending: false,
            count_pending: None,
            keymap_save_name: String::new(),
            donation_url: DONATION_URL.to_owned(),
            show_bug_report: false,
            bug_report: bugreport::BugReport::default(),
            re_bref: None,
            visual_anchor: None,
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
    /// Le texte de l'éditeur vient d'être écrit : tout ce qui en dépend
    /// est périmé.
    ///
    /// Un seul endroit incrémente la révision, et c'est celui-ci. Deux
    /// portes y mènent — `set_source`, qui remplace le texte, et
    /// `source_mut`, qui le prête en écriture — plus le champ de texte
    /// d'egui, qui écrit lui-même dans le tampon et le dit par
    /// `response.changed()`. Le contrôle
    /// `tout_ce_qui_ecrit_le_texte_le_dit` tient la liste à jour.
    pub(crate) fn texte_a_changé(&mut self) {
        self.texte_rev = self.texte_rev.wrapping_add(1);
    }

    /// Remplace le texte de l'éditeur, et le dit.
    pub(crate) fn set_source(&mut self, texte: String) {
        self.source = texte;
        self.texte_a_changé();
    }

    /// Le texte de l'éditeur, prêté en écriture — et la révision avance.
    ///
    /// Elle avance même si l'appelant ne change finalement rien : se
    /// tromper dans ce sens-là fait recalculer un relevé pour rien, se
    /// tromper dans l'autre fait peindre un relevé périmé.
    pub(crate) fn source_mut(&mut self) -> &mut String {
        self.texte_a_changé();
        &mut self.source
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
            // La comparaison vive, et non le relevé de l'image : cette
            // question DÉCIDE — elle grise le bouton « Assembler », mais
            // elle refuse aussi le geste. Le contrôle
            // `assembling_is_greyed_out_on_a_binary_listing` touche le
            // texte puis demande aussitôt : une réponse d'une image de
            // retard dirait « c'est encore un binaire » à quelqu'un qui
            // vient de l'éditer.
            && !self.buffer_modified()
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
            // L'interface LANCE et rend la main : la fenêtre continue
            // de vivre, et le résultat est récolté par `tick_assemble`
            // à l'image où il arrive. `do_assemble`, qui attend, reste
            // pour qui a besoin du programme dans la foulée.
            Action::Assemble => self.start_assemble(),
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
            Action::ShowFlow => self.montrer_vue(VueCentrale::Flow),
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
            // Une BASCULE, et c'est le seul chemin qui en soit une :
            // la calculatrice s'ouvre au milieu d'autre chose, et la
            // même frappe doit la renvoyer. L'ouvrir sans pouvoir la
            // refermer de la même main obligeait à viser sa croix.
            Action::ToggleCalculator => {
                self.show_calculator = !self.show_calculator;
                if !self.show_calculator {
                    self.calculator_was_open = false;
                }
            }
            // L'autre bascule : la palette se referme de la frappe qui
            // l'a ouverte. Elle le faisait déjà — mais depuis un
            // `consume_shortcut` posé en dur dans la boucle
            // d'affichage, qu'aucun réglage ne pouvait atteindre.
            Action::ToggleCommandPalette => {
                self.command_palette = match self.command_palette {
                    Some(_) => None,
                    None => Some(CommandPalette::default()),
                };
            }
            Action::ShowPreferences => self.open_preferences(),
            Action::ToggleConsole => self.show_console = !self.show_console,
            Action::ToggleSidebar => self.show_sidebar = !self.show_sidebar,
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
                // UN CLIC SUR LA BARRE D'ICÔNES N'EST PAS UN CLIC
                // « AILLEURS », et sans cette exception l'icône de
                // l'explorateur ne pouvait pas le refermer.
                //
                // Mesuré : l'APPUI tombe hors de la fenêtre, qui se
                // ferme ; le RELÂCHEMENT arrive sur une icône devenue
                // celle d'un panneau fermé, qui le rouvre. L'explorateur
                // restait ouvert et clignotait, et l'on ne pouvait le
                // fermer que par le menu ou en cliquant ailleurs — ce
                // qui est précisément ce que la barre devait éviter.
                let sur_la_barre = self.show_rails
                    && ctx
                        .pointer_interact_pos()
                        .is_some_and(|pos| pos.x < barres::BARRE_LARGEUR);
                if !sur_la_barre {
                    self.show_explorer = false;
                    self.explorer_focused = false;
                }
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
            // Choisir « Palette de commandes » DANS la palette la
            // referme, et ne la rouvre pas.
            //
            // L'appelant l'a déjà fermée avant d'arriver ici : la
            // bascule la verrait absente et la rouvrirait aussitôt,
            // vidée de ce qu'on venait d'y taper. Une commande qui se
            // relance elle-même n'a rien fait, et le geste qu'on
            // attendait — fermer — est celui qui arrive.
            PaletteCommand::Do(Action::ToggleCommandPalette) => {}
            PaletteCommand::Do(action) => self.dispatch(action),
            PaletteCommand::ShowAbout => self.show_about = true,
            PaletteCommand::ShowExercises => self.show_exercises = true,
            // Les vues de la rangée : la palette les y choisit, comme
            // le menu Outils, et laisse en place celles qu'on a
            // détachées.
            PaletteCommand::ShowAnalysis => self.montrer_vue(VueCentrale::Analysis),
            PaletteCommand::ShowVectors => self.montrer_vue(VueCentrale::Vectors),
            PaletteCommand::ShowBreakpoints => self.montrer_vue(VueCentrale::Breakpoints),
            PaletteCommand::ShowTimeline => self.montrer_vue(VueCentrale::Timeline),
            PaletteCommand::ShowCyclic => self.show_cyclic = true,
            PaletteCommand::ShowScene => self.show_scene = true,
            PaletteCommand::ShowDisassembly => self.montrer_vue(VueCentrale::Disassembly),
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
            PaletteCommand::ToggleToolbar => self.show_toolbar = !self.show_toolbar,
            PaletteCommand::ToggleStatusbar => self.show_statusbar = !self.show_statusbar,
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

    /// Les fenêtres qui peuvent s'agrandir, nommées une fois pour que
    /// l'état et le bouton parlent de la même.
    const FLOW_WINDOW: &'static str = "flow";
    const DECOMPILER_WINDOW: &'static str = "decompiler";
    const DISASSEMBLY_WINDOW: &'static str = "disassembly";
    const TIMELINE_WINDOW: &'static str = "timeline";
    // « analysis » y est revenue : la raison de l'en sortir — le
    // listing qui défilait derrière un panneau opaque — est tombée, le
    // clic sur une adresse effaçant la fenêtre. Voir `analysis_window_ui`.
    const ANALYSIS_WINDOW: &'static str = "analysis";
    const EXERCISES_WINDOW: &'static str = "exercises";
    const CALCULATOR_WINDOW: &'static str = "calculator";

    /// Toutes les fenêtres internes, et si chacune est ouverte.
    ///
    /// Une seule table, et un test vérifie qu'elle les cite toutes :
    /// ajouter une fenêtre sans y penser reviendrait à réintroduire le
    /// défaut sans le voir. Les noms sont les identifiants qu'on passe à
    /// `egui::Window::id`, lesquels ne changent pas avec la langue.
    fn window_states(&self) -> [(&'static str, bool); 23] {
        [
            ("preferences_window", self.show_preferences),
            (ABOUT_WINDOW, self.show_about),
            ("bug_report", self.show_bug_report),
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
            ("scene_window", self.show_scene),
            ("etabli_machine_window", self.etabli_machine_detachee),
            ("etabli_resultat_window", self.etabli_resultat_detache),
            ("analysis_window", self.show_analysis),
            ("calculator_window", self.show_calculator),
            ("exercises_window_v2", self.exercises && self.show_exercises),
            ("unsaved_changes", self.pending.is_some()),
            ("command_palette", self.command_palette.is_some()),
            (INPUT_DIALOG, self.show_input_dialog),
            (PATCHES_WINDOW, self.show_patches),
            // Les suites d'une séquence armée. Elle n'a ni titre ni
            // poignée, mais c'est bien une fenêtre — et elle doit
            // passer devant en paraissant, comme les autres : un
            // panneau qui sort sous une fenêtre ouverte ne dit rien à
            // personne.
            ("leader_hints", !self.dispatcher.pending().is_empty()),
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
            // LES DEUX RANGS, et non le seul rang ordinaire.
            //
            // Trois fenêtres vivent au premier plan — le plan
            // d'installation, « À propos », la saisie du programme — et
            // les remettre devant au rang ordinaire ne faisait rien :
            // on déplaçait un calque vide qui porte leur nom, pendant
            // que le vrai restait où il était. Pire, ce calque fantôme
            // existe ensuite pour de bon, et un test qui cherche « le
            // calque de cette fenêtre » peut tomber sur lui.
            //
            // Remettre les deux devant coûte un appel et ne suppose
            // rien : la fenêtre est à l'un des deux rangs, jamais aux
            // deux.
            for rang in [egui::Order::Middle, egui::Order::Foreground] {
                ctx.move_to_top(egui::LayerId::new(rang, egui::Id::new(*nom)));
            }
            // Les préférences et le plan d'installation parlent des
            // outils : on redemande à la machine en les ouvrant, parce
            // qu'on a pu en installer un à la main dans un terminal
            // depuis la dernière fois. Le reste du temps, le relevé
            // tient — c'est un processus par outil.
            if matches!(*nom, "preferences_window" | "installer_window") {
                self.tools_ready.clear();
            }
        }
        self.windows_open = ouvertes;
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
        self.values_changed_at = None;
        self.last_step = None;
        self.error_advice = None;
        self.follow_execution();
        self.status = Status::SteppedBack;
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
}

/// La date de dernière modification d'un fichier, si on peut la lire.
fn file_mtime(path: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// Ce que prend l'en-tête d'un bloc : « 0x0001d491  ? ».
const ENTÊTE_COLONNES: usize = 16;

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

/// Le nom d'un onglet : le fichier, l'exercice, ou « sans titre ».
///
/// Un exercice n'a pas de fichier sur le disque, et trois onglets
/// « sans titre » côte à côte ne disent rien. Son titre, lui, dit
/// exactement ce qu'on travaille.
fn tab_label(
    path: Option<&Path>,
    exercise: Option<&str>,
    binary_name: Option<&str>,
    s: &Strings,
    packs: &[namsdec_exercises::Pack],
    lang: Lang,
    untitled_no: usize,
) -> String {
    if let Some(nom) = path.and_then(|p| p.file_name()) {
        return nom.to_string_lossy().into_owned();
    }
    if let Some(exercice) = exercise.and_then(|id| namsdec_exercises::find(packs, id)) {
        return exercice.title.get(lang.code()).to_owned();
    }
    // Un binaire ouvert n'a pas de `path` — on ne le rouvre pas en
    // l'enregistrant par-dessus — mais il a un nom, celui du fichier d'où
    // il vient. Sans ça, l'onglet d'un binaire chargé restait « sans
    // titre », et l'on ne savait plus lequel on regardait.
    if let Some(nom) = binary_name.filter(|nom| !nom.is_empty()) {
        return nom.to_owned();
    }
    // NUMÉROTÉ, comme partout ailleurs. Trois programmes neufs
    // s'appelaient tous « sans titre » : on ouvrait le troisième et
    // rien ne disait qu'il y en avait trois, ni lequel on regardait.
    // Le premier garde son nom nu — tant qu'il est seul, un numéro
    // n'apprend rien.
    if untitled_no > 1 {
        return format!("{} {untitled_no}", s.untitled_buffer);
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

fn window_room(ui: &egui::Ui) -> f32 {
    (ui.ctx().input(|i| i.content_rect()).height() * 0.55).max(240.0)
}

/// Ce qu'on écrit sur une adresse : son nom, ou un commentaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteKind {
    Name,
    Comment,
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

/// Des paires hexadécimales, séparées comme on veut.
///
/// « 48 31 c0 », « 4831c0 », « 48,31,c0 » : on recopie ce qu'on lit
/// ailleurs, et ce qu'on lit ailleurs n'a pas toujours la même ponctuation.
/// Ce qui est refusé, c'est un nombre impair de chiffres — là, on ne sait
/// pas ce que la personne voulait dire.
fn lire_octets(saisie: &str) -> Option<Vec<u8>> {
    let chiffres: String = saisie
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    if chiffres.is_empty() || !chiffres.len().is_multiple_of(2) {
        return None;
    }
    if !chiffres.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    chiffres
        .as_bytes()
        .chunks(2)
        .map(|paire| u8::from_str_radix(std::str::from_utf8(paire).ok()?, 16).ok())
        .collect()
}

/// Le sens du clic, et le pointeur qui l'annonce.
///
/// Une étiquette qu'on peut cliquer ne ressemble à rien : c'est du
/// texte. Douze d'entre elles dans namsdec — un nom de registre qu'on
/// surveille, une valeur qu'on remplace, une ligne de pile où l'on va,
/// une zone mémoire, un module d'analyse — et aucune ne changeait le
/// pointeur. Les infobulles disaient ce qu'un clic ferait, mais il
/// fallait déjà s'être arrêté dessus pour les lire ; la main, elle, se
/// voit en passant.
///
/// egui ne le fait pas tout seul : il réserve la main aux liens. Ce qui
/// se clique doit pourtant se voir avant qu'on ait lu quoi que ce soit,
/// et c'est le seul signe qui ne coûte pas une ligne de texte.
///
/// Le trait, lui, se voit SANS SURVOL : voir le corps.
fn cliquable(ui: &mut egui::Ui, étiquette: egui::Label) -> egui::Response {
    let réponse = ui
        .add(étiquette.sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    // LE TRAIT SE VOIT AU REPOS, et c'est la correction du 20 octobre.
    //
    // Il ne paraissait qu'au survol, au nom d'une page calme. Mais un
    // signe qui n'apparaît qu'une fois qu'on est dessus ne sert qu'à
    // celui qui savait déjà : on ne survole pas les textes au hasard
    // pour voir lesquels se cliquent. Signalé à l'écran — « les textes
    // où une action est attendue ne l'indiquent pas ».
    //
    // Le calme est gardé autrement : un tiers d'opacité au repos, assez
    // pour se voir en parcourant une colonne, trop peu pour faire du
    // bruit. Plein au survol.
    let couleur = if réponse.hovered() {
        ui.visuals().widgets.hovered.fg_stroke.color
    } else {
        ui.visuals().hyperlink_color.gamma_multiply(0.35)
    };
    ui.painter().hline(
        réponse.rect.x_range(),
        réponse.rect.bottom() - 1.0,
        egui::Stroke::new(1.0, couleur),
    );
    réponse
}

/// Souligne une RANGÉE quand la souris est dessus.
///
/// Réservé aux rangées entières d'une liste — une ligne de
/// désassemblage, une zone de la carte mémoire, une entrée d'index :
/// un trait permanent sous chacune des trois cents lignes d'une liste
/// ne serait plus un signe, seulement du bruit. Pour un texte au milieu
/// d'une phrase, c'est `cliquable`, dont le trait se voit au repos.
///
/// Peint APRÈS coup, sous le rectangle de la réponse, plutôt que posé
/// dans le `RichText` : on ne sait qu'un texte est survolé qu'une fois
/// qu'il est dessiné, et le relire de l'image précédente ferait clignoter
/// le trait d'une image sur deux.
pub(crate) fn souligne_au_survol(ui: &egui::Ui, réponse: &egui::Response) {
    // Au survol seulement : il sert aux RANGÉES d'une liste, où un
    // trait permanent sous chacune des trois cents lignes ne serait plus
    // un signe mais du bruit. Pour un texte au milieu d'une phrase,
    // c'est `cliquable` qu'il faut, dont le trait se voit au repos.
    if !réponse.hovered() {
        return;
    }
    let couleur = ui.visuals().widgets.hovered.fg_stroke.color;
    let y = réponse.rect.bottom() - 1.0;
    ui.painter()
        .hline(réponse.rect.x_range(), y, egui::Stroke::new(1.0, couleur));
}

/// Une liste déroulante à nous, et non celle d'egui.
///
/// Celle d'egui ne se pose pas au centre de sa rangée : son cadre part
/// de `available_rect_before_wrap()`, si bien que son texte se retrouve
/// trois pixels et demi sous l'étiquette qui l'annonce. Six façons de
/// l'entourer ont été essayées — imposer la hauteur de la case, de la
/// rangée, `add_sized`, une boîte centrée, annuler l'espacement
/// vertical, aligner la rangée par le haut — et l'écart n'a pas bougé
/// d'un pixel : il est dedans, pas autour.
///
/// Celle-ci est un BOUTON, et un bouton se centre comme tout le reste.
/// Elle a le liseré de `framed_button`, donc la même façon que les
/// autres commandes, et son chevron est posé à droite par egui lui-même.
/// Un contrôle de moins qui ne ressemble à rien d'autre.
fn liste_déroulante<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    theme: &Theme,
    id: egui::Id,
    largeur: f32,
    valeur: &mut T,
    options: &[(T, String)],
    quand_rien: &str,
) -> bool {
    // Ce qui s'écrit quand la valeur courante n'est dans aucune option :
    // aucun décompilateur choisi, par exemple. Une liste vide qui ne dit
    // rien laisse croire qu'elle est cassée.
    let courant = options
        .iter()
        .find(|(v, _)| v == valeur)
        .map(|(_, nom)| nom.clone())
        .unwrap_or_else(|| quand_rien.to_owned());
    // Le chevron est PEINT, non écrit : « ▾ » n'existe dans aucune des
    // polices livrées, et un contrôle l'a dit avant qu'on ne le voie —
    // il serait sorti en carré vide.
    //
    // Sa place est réservée par la LARGEUR du bouton et non par des
    // espaces ajoutés au texte : deux espaces à la fin changent le
    // texte peint, et deux contrôles qui cherchaient « Hexadécimal »
    // ne l'ont plus trouvé. Ce qui est écrit doit rester ce qu'on a
    // demandé d'écrire.
    const CHEVRON: f32 = 20.0;
    let mesure = ui
        .painter()
        .layout_no_wrap(
            courant.clone(),
            egui::TextStyle::Body.resolve(ui.style()),
            egui::Color32::PLACEHOLDER,
        )
        .size()
        .x;
    let voulue = largeur.max(mesure + CHEVRON + ui.spacing().button_padding.x * 2.0);
    // Le bouton est VIDE, et son texte peint à gauche.
    //
    // egui centre le texte d'un bouton. Deux listes de même largeur
    // portant « Hexadécimal » et « 64 bits » plaçaient donc leurs textes
    // à dix-sept pixels d'écart, et l'invariant des jumeaux l'a dit :
    // « la valeur doit être au même écart de son étiquette — 71 px pour
    // Base, 88,6 px pour Largeur ». Une liste déroulante aligne à
    // gauche, comme partout ailleurs.
    //
    // Et ce n'est PAS un `Button` au libellé vide : celui-ci peint quand
    // même une galette de largeur nulle, que l'invariant des
    // recouvrements a trouvée posée sur le texte — « " " et "DZF —
    // décompilation ELF64 (beta)" se recouvrent ». On alloue donc la
    // place et l'on peint le cadre soi-même, ce qui est de toute façon
    // ce que fait `framed_button` en trois lignes.
    let (cadre, réponse) = ui.allocate_exact_size(
        egui::vec2(voulue, ui.spacing().interact_size.y),
        egui::Sense::click(),
    );
    let visuels = ui.style().interact(&réponse);
    ui.painter().rect(
        cadre,
        egui::CornerRadius::from(6),
        visuels.weak_bg_fill,
        egui::Stroke::new(1.0, theme.dim),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        egui::pos2(
            cadre.left() + ui.spacing().button_padding.x,
            cadre.center().y,
        ),
        egui::Align2::LEFT_CENTER,
        &courant,
        egui::TextStyle::Body.resolve(ui.style()),
        theme.text,
    );
    let pointe = egui::pos2(cadre.right() - 11.0, cadre.center().y);
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(pointe.x - 4.0, pointe.y - 2.0),
            egui::pos2(pointe.x + 4.0, pointe.y - 2.0),
            egui::pos2(pointe.x, pointe.y + 3.0),
        ],
        theme.dim,
        egui::Stroke::NONE,
    ));
    let mut changé = false;
    egui::Popup::menu(&réponse)
        .id(id)
        .width(largeur.max(réponse.rect.width()))
        .show(|ui| {
            for (v, nom) in options {
                if ui.selectable_label(v == valeur, nom).clicked() {
                    *valeur = v.clone();
                    changé = true;
                    ui.close();
                }
            }
        });
    changé
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
        // L'ÉCHELLE A-T-ELLE CHANGÉ ? Alors tout est à repeindre.
        //
        // Deux captures du 25 septembre au soir montraient le panneau de
        // droite avec deux blocs superposés : l'état du processeur et le
        // haut de l'onglet Mémoire, sur les mêmes pixels. Mesuré sur
        // l'image : « RIP » et « Carte de la mémoire » font 33 et 31
        // pixels de haut, et les textes qui se superposent à eux en font
        // DIX-HUIT. Ce n'est donc pas une mise en page qui se recouvre —
        // egui dessine une image à une seule échelle — mais une bande
        // restée d'un rendu PRÉCÉDENT, plus petit, que rien n'a
        // repeinte. Le facteur d'échelle de l'écran avait changé entre
        // les deux.
        //
        // On ne peut pas réparer le compositeur, mais on peut ne pas
        // l'attendre : dès que l'échelle bouge, on redemande une image.
        // C'est une ligne, ça ne coûte rien, et c'est le seul endroit
        // d'où l'on voit passer le changement.
        let échelle = ctx.pixels_per_point();
        if self
            .dernière_échelle
            .is_some_and(|vue| (vue - échelle).abs() > 0.001)
        {
            ctx.request_repaint();
        }
        self.dernière_échelle = Some(échelle);
        // OÙ CHAQUE FENÊTRE AGRANDISSABLE SE TIENT QUAND ELLE EST
        // NORMALE, relevé avant que les fenêtres ne soient construites :
        // c'est ce que « Restaurer » leur rendra. Voir `maximize.rs`.
        self.retenir_les_fenetres_normales(&ctx);
        // Avant tout le reste : le fichier qu'on vient de choisir dans la
        // boîte native doit être chargé avant que la frame ne s'affiche.
        self.poll_file_dialog(&ctx);
        #[cfg(feature = "reseau")]
        self.poll_update_check(&ctx);
        self.poll_decompile(&ctx);
        self.poll_tool_run(&ctx);
        self.poll_upx(&ctx);
        self.poll_install(&ctx);
        self.reflect_job_in_title(&ctx);
        // Au retour dans la fenêtre : un autre programme a pu passer par
        // là — un `git checkout`, un éditeur, un `sed`. On regarde à ce
        // moment-là, et non à chaque image : c'est un accès au disque.
        let focus = ctx.input(|i| i.focused);
        if focus && !self.had_focus {
            self.check_disk_change();
        }
        // Le fichier de raccourcis, lui, se surveille en continu : on
        // l'édite souvent sans quitter namsdec des yeux, et un jeu qui
        // n'arrive qu'au lancement suivant passe pour un fichier qui ne
        // sert à rien.
        self.reload_keymap_if_changed();
        self.had_focus = focus;
        self.apply_tooltip_setting(&ctx);
        self.tick_assemble();
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
            //
            // LE MODE VISUEL compte comme un champ de saisie, et pour la
            // même raison : ses lettres lui appartiennent. `y` y copie
            // la sélection, et ne doit pas armer la séquence `y y` qui
            // duplique une ligne — ni `d` ouvrir quoi que ce soit. En
            // Visuel on a quelque chose de sélectionné sous les yeux, et
            // la frappe suivante porte dessus.
            let text_focused = self.editor_focused
                || self.editor_mode == EditorMode::Visual
                || ctx.memory(|m| m.focused().is_some());
            if let Some(action) = self.dispatcher.poll(&ctx, &self.keybindings, text_focused) {
                self.dispatch(action);
            }
            // Depuis quand on attend la touche suivante. La date est
            // posée à l'armement et NON remise à chaque image : c'est
            // elle qui mesure l'hésitation.
            if self.dispatcher.pending().is_empty() {
                self.leader_since = None;
            } else {
                self.leader_since.get_or_insert_with(Instant::now);
            }
        }

        // Après les panneaux : le relevé de leur hauteur a besoin
        // qu'ils aient été posés.
        self.leader_hints_ui(&ctx);

        self.guard_close(&ctx);
        // Accordée : on repasse la demande au gestionnaire de fenêtres,
        // qui l'avait vue refusée.
        if self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Modifié ou non : relevé une fois pour toute l'image (voir
        // `modifié_vu`).
        self.modifié_vu = self.buffer_modified();
        self.tabs_modifiés_vus = self.tabs.iter().map(OpenTab::modifié).collect();

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

        // DEUX DISPOSITIONS, ET NON UN FILTRE DE PLUS.
        //
        // Jusqu'au 9 novembre 2026, le mode débutant était l'interface
        // complète avec des morceaux en moins : même barre de menus,
        // mêmes rails, même barre d'état, et des panneaux qui ne
        // paraissaient pas. L'Établi est une mise en page À ELLE —
        // quatre bandes fixes, rien à ouvrir — et c'est le niveau qui
        // choisit entre les deux, non plus ce que chaque panneau
        // décide de montrer. Voir `etabli.rs` pour le pourquoi, et
        // `mockups/2026-11-09/` pour les trois maquettes départagées.
        //
        // Le reste de la frame ne change pas : les fenêtres flottantes,
        // la palette, les réglages et les raccourcis sont les mêmes aux
        // deux niveaux. Un mode simple qui perdrait l'aide ou les
        // préférences ne serait pas simple, il serait amputé.
        if self.debutant {
            timed(&mut spans, "établi", || self.etabli_ui(ui));
        } else {
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
            // LES BARRES D'ICÔNES DES BORDS, posées avant les panneaux
            // qu'elles commandent.
            //
            // L'ordre est tout : un panneau egui prend la place qui reste au
            // moment où il est posé. Les barres passant les premières, elles
            // tiennent toute la hauteur — de sous la barre d'outils jusqu'à
            // la barre d'état — et la console s'ouvre ENTRE elles, comme
            // chez JetBrains. Posées après, elles s'arrêteraient au-dessus
            // de la console et l'icône qui l'ouvre serait séparée d'elle.
            if self.show_rails {
                timed(&mut spans, "barres d'icônes", || {
                    let cadre = egui::Frame::new()
                        .fill(self.theme.panel)
                        .inner_margin(egui::Margin::symmetric(4, 6));
                    egui::Panel::left("barre_gauche")
                        .resizable(false)
                        .exact_size(barres::BARRE_LARGEUR)
                        .frame(cadre)
                        .show(ui, |ui| self.barre_gauche_ui(ui));
                    egui::Panel::right("barre_droite")
                        .resizable(false)
                        .exact_size(barres::BARRE_LARGEUR)
                        .frame(cadre)
                        .show(ui, |ui| self.barre_droite_ui(ui));
                });
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
                        //
                        // LA LIGNE DE JOURNAL EST COMPTÉE DEPUIS, et elle
                        // ne l'était pas : le plancher ne gardait rien pour
                        // la sortie, qui réclame alors son propre minimum
                        // et le prend sur la saisie — son fond est opaque,
                        // et il est peint par-dessus. Les deux rangées du
                        // haut de la saisie disparaissaient dessous sans
                        // qu'aucune mesure ne bouge : elles étaient peintes
                        // et entières, simplement recouvertes.
                        // Les marges du cadre de la sortie, et elles
                        // seules : la sortie sait se replier jusqu'à ne
                        // plus rien montrer, son cadre non.
                        let sortie = console::MARGES_DE_LA_SORTIE;
                        (saisie + ui.text_style_height(&egui::TextStyle::Button) + 28.0 + sortie)
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
            // La zone de travail, relevée ICI : les barres sont posées, le
            // panneau latéral aussi, et ce qui reste est la place de
            // l'éditeur. C'est elle qu'une fenêtre agrandie occupe, pour ne
            // pas recouvrir les menus ni la barre d'état.
            self.zone_de_travail = ui.available_rect_before_wrap();
            // Les fichiers, puis les vues, puis la vue elle-même.
            //
            // La barre des fichiers est sortie de `editor_ui` : elle dit
            // quel programme on travaille, ce qui reste vrai quelle que
            // soit la vue — le désassemblage est celui de CE fichier. Elle
            // disparaissait en même temps que l'éditeur, et l'on ne savait
            // plus de quoi on lisait le code machine.
            timed(&mut spans, "onglets", || {
                self.tabs_bar_ui(ui);
                self.views_bar_ui(ui);
            });
            timed(&mut spans, "éditeur", || self.vue_centrale_ui(ui));
        }

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
            self.bug_report_ui(&ctx);
            self.help_window_ui(&ctx);
            self.vectors_window_ui(&ctx);
            self.breakpoints_window_ui(&ctx);
            self.timeline_window_ui(&ctx);
            self.flow_window_ui(&ctx);
            self.decompiler_window_ui(&ctx);
            self.installer_window_ui(&ctx);
            self.disassembly_window_ui(&ctx);
            self.cyclic_window_ui(&ctx);
            self.scene_window_ui(&ctx);
            self.etabli_fenetres_ui(&ctx);
            self.analysis_window_ui(&ctx);
            self.calculator_window_ui(&ctx);
            if self.exercises {
                self.exercises_window_ui(&ctx);
            }
            self.command_palette_ui(&ctx);
            self.patches_window_ui(&ctx);
            self.unsaved_window_ui(&ctx);
            // En dernier : c'est la seule fenêtre qui réclame une
            // réponse, elle passe donc devant toutes les autres.
            self.input_dialog_ui(&ctx);
        });

        // Les fenêtres qu'on restaurait viennent d'être construites avec
        // le rectangle qu'elles avaient : le drapeau a fait son office.
        self.fin_des_restaurations();

        // En dernier : tout réglage changé pendant cette frame, d'où qu'il
        // vienne (menu, préférences, palette, raccourci), est sur le
        // disque avant la frame suivante.
        timed(&mut spans, "réglages", || {
            self.persist_settings_if_changed()
        });

        // Cette frame a-t-elle POSÉ le texte, ou seulement relu ce qui
        // était déjà en page ?
        //
        // La question comptait, et la réponse était devinée : on
        // comparait la longueur du texte à celle de la frame d'avant.
        // Cela marche quand le texte change et que l'éditeur le pose
        // dans la même frame — le cas ordinaire —, et se trompe
        // exactement quand il ne faut pas : un fichier choisi dans un
        // dialogue arrive parfois après le dessin de l'éditeur, et c'est
        // la frame SUIVANTE qui paie la mise en page. La longueur y a
        // déjà été notée, donc on criait « frame lente » pour un
        // chargement — le seul cas où le message ne devait pas paraître.
        //
        // Le cache de mise en page compte ses poses : il sait, lui, s'il
        // a payé.
        let posé = editor::poses_du_texte() != self.poses_au_début;
        self.poses_au_début = editor::poses_du_texte();
        report_slow_frame(frame_start.elapsed(), &spans, posé);
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

/// Un texte court en chasse fixe.
///
/// `RichText::monospace().small()` ne fait PAS ce qu'il dit. Les deux
/// posent le même champ — `text_style` —, si bien que le second efface
/// le premier : on demandait du code en petit et l'on obtenait du
/// proportionnel en petit. Le JSON d'exemple du décompilateur
/// s'affichait ainsi en proportionnel, accolades et indentation
/// désalignées, trente pixels sous un chemin en chasse fixe et dans la
/// même fenêtre. Trois autres endroits faisaient la même chose.
///
/// Un `FontId` explicite dit les deux à la fois. La taille est celle du
/// style « petit » du thème en cours, et non un nombre écrit ici : ce
/// qui doit rester petit doit le rester quand le thème change d'échelle.
fn small_monospace(ui: &egui::Ui, texte: impl Into<String>) -> egui::RichText {
    let taille = egui::TextStyle::Small.resolve(ui.style()).size;
    egui::RichText::new(texte.into()).font(egui::FontId::monospace(taille))
}

/// Un texte d'explication où les morceaux de code sont écrits comme du
/// code.
///
/// Les fiches marquent ces morceaux entre accents graves, comme partout
/// ailleurs. Les laisser tels quels donnerait « préparer RDX — `xor rdx,
/// rdx` » ; les retirer sans rien mettre à la place perdrait la
/// distinction. Un `LayoutJob` permet de changer de police au milieu
/// d'une phrase, sans casser le retour à la ligne.
///
/// Les DOUBLES ASTÉRISQUES appuient un passage, et ils avaient été
/// oubliés : l'explication du motif cyclique s'affichait « une suite dont
/// \*\*toute tranche de quatre octets est unique\*\* », astérisques
/// comprises. On les voyait sur une capture et nulle part ailleurs — le
/// texte est juste, il est simplement écrit dans une convention que
/// personne n'honorait.
///
/// L'appui se rend par une encre plus vive et non par un gras : egui
/// n'a qu'une graisse par famille, et c'est ainsi qu'il marque lui-même
/// un texte fort. Ce qui est entre accents graves reste du code, même
/// s'il contient des astérisques.
fn prose(ui: &egui::Ui, theme: &'static Theme, text: &str) -> egui::text::LayoutJob {
    let normal = egui::TextFormat {
        font_id: egui::TextStyle::Body.resolve(ui.style()),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let fort = egui::TextFormat {
        color: ui.visuals().strong_text_color(),
        ..normal.clone()
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
        if index % 2 == 1 {
            job.append(part, 0.0, code.clone());
            continue;
        }
        // Hors du code, une paire d'astérisques sur deux ouvre un
        // passage appuyé. Une astérisque solitaire ne ferme rien et
        // reste donc telle quelle, ce qui est le comportement voulu :
        // une phrase peut parler d'une multiplication.
        for (morceau, appuyé) in part.split("**").zip([false, true].into_iter().cycle()) {
            if morceau.is_empty() {
                continue;
            }
            if appuyé {
                job.append(morceau, 0.0, fort.clone());
                continue;
            }
            // ET L'ASTÉRISQUE SIMPLE, QUI NE SE RENDAIT PAS.
            //
            // Cinquante passages des fiches l'emploient — « le
            // débordement *non signé* », « laisse RAX *intacte* » — et
            // la fenêtre d'aide montrait ses astérisques depuis
            // toujours. Personne ne l'avait vu parce qu'elles sont
            // discrètes ; la bande qui explique la ligne où est le
            // curseur les a mises sous le nez, en gros, au premier
            // écran.
            //
            // LA RÈGLE EXCLUT LA MULTIPLICATION, et c'est tout le soin
            // à prendre : une paire n'ouvre un passage appuyé que si
            // son ouvrante est collée au mot qui suit et sa fermante
            // au mot qui précède. « RDX:RAX = RAX * 3 » garde donc son
            // astérisque — elle a un blanc de chaque côté.
            emphase_simple(&mut job, morceau, &normal, &fort);
        }
    }
    job
}

/// Pose un morceau de prose en rendant ses `*emphases*` simples.
///
/// Appelé après le découpage sur `**` : ce qui arrive ici ne porte plus
/// que des astérisques seules. Une paire ouvre un passage appuyé quand
/// elle se comporte comme en Markdown — ouvrante collée à ce qui suit,
/// fermante collée à ce qui précède. Tout le reste est rendu tel quel,
/// ce qui laisse passer la multiplication et une astérisque qui ne
/// ferme rien.
fn emphase_simple(
    job: &mut egui::text::LayoutJob,
    texte: &str,
    normal: &egui::TextFormat,
    fort: &egui::TextFormat,
) {
    let mut reste = texte;
    // `depuis` avance quand une astérisque ne s'est pas révélée être
    // une ouvrante : sans lui, on la réexaminerait sans fin.
    let mut depuis = 0usize;
    while let Some(relatif) = reste.get(depuis..).and_then(|q| q.find('*')) {
        let ouvre = depuis + relatif;
        let octets = reste.as_bytes();
        // Collée à ce qui suit, sinon ce n'est pas une ouvrante.
        if !octets
            .get(ouvre + 1)
            .is_some_and(|c| !c.is_ascii_whitespace() && *c != b'*')
        {
            depuis = ouvre + 1;
            continue;
        }
        // La fermante : la première astérisque collée au mot qui la
        // précède, sans passer de ligne — une emphase ne s'étend pas
        // sur un paragraphe.
        let mut ferme = None;
        let mut curseur = ouvre + 1;
        while let Some(relatif) = reste.get(curseur..).and_then(|q| q.find('*')) {
            let candidat = curseur + relatif;
            if reste[ouvre + 1..candidat].contains('\n') {
                break;
            }
            if octets
                .get(candidat - 1)
                .is_some_and(|c| !c.is_ascii_whitespace())
            {
                ferme = Some(candidat);
                break;
            }
            curseur = candidat + 1;
        }
        let Some(ferme) = ferme else {
            depuis = ouvre + 1;
            continue;
        };
        if ouvre > 0 {
            job.append(&reste[..ouvre], 0.0, normal.clone());
        }
        job.append(&reste[ouvre + 1..ferme], 0.0, fort.clone());
        reste = &reste[ferme + 1..];
        depuis = 0;
    }
    if !reste.is_empty() {
        job.append(reste, 0.0, normal.clone());
    }
}

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

/// L'identifiant de la fenêtre des patchs.
pub(crate) const PATCHES_WINDOW: &str = "patches_window";

/// Les fenêtres qui reviennent d'une séance à l'autre.
///
/// Fermer la barre d'outils la gardait fermée au lancement suivant ;
/// ouvrir la calculatrice, non — elle partait avec la séance. Deux
/// gestes de même nature, deux réponses différentes : c'est cette
/// incohérence-là qu'on enlève. Ce qu'on a disposé autour de son code
/// est un réglage comme un autre, et un réglage se retrouve.
///
/// Toutes les fenêtres n'y sont pas. Préférences, « À propos » et la
/// saisie du programme s'ouvrent sur une question et se ferment sur la
/// réponse ; les rouvrir au lancement serait répondre à une question
/// que personne n'a posée. Ne reviennent que les fenêtres d'**établi**,
/// celles qu'on laisse ouvertes à côté du code parce qu'on s'en sert.
///
/// Deux d'entre elles appartiennent à un module qui peut avoir été
/// éteint entre-temps : leur reprise le redemande plutôt que de le
/// supposer, si bien qu'éteindre le module emporte bien sa fenêtre.
/// La taille d'ouverture d'une fenêtre, selon qu'elle a de quoi la
/// remplir.
///
/// Une fenêtre s'ouvrait toujours à la taille de ce qu'elle montre QUAND
/// elle montre quelque chose. Sans binaire, le graphe prenait donc
/// quatre-vingt-douze pour cent de l'écran pour une phrase et son
/// explication : mille quatre cent soixante-douze pixels de large, trois
/// cent quatre-vingts de vide entre le texte et le bouton « OK ». Les
/// registres étendus, cinq cent vingt sur six cent vingt pour deux
/// lignes. C'est l'écran du PREMIER lancement, celui que voit quelqu'un
/// qui découvre namsdec, et c'est celui qu'on avait laissé pour la fin.
///
/// `default_size` ne vaut qu'à la création de la fenêtre, c'est-à-dire
/// au moment où on l'ouvre : on sait alors s'il y a quelque chose
/// dedans. Ouverte à vide puis remplie, elle garde sa petite taille —
/// elle est redimensionnable, et son contenu défile ; c'est un moindre
/// mal devant une fenêtre vide grande comme l'écran.
fn window_size(plein: bool, pleine: egui::Vec2, vide: egui::Vec2) -> egui::Vec2 {
    if plein { pleine } else { vide }
}

/// Où poser une fenêtre d'établi qu'on ouvre pour la première fois.
///
/// Toutes partaient du centre exact du viewport, avec un pivot centré :
/// deux fenêtres ouvertes ensemble se superposaient AU PIXEL PRÈS, et
/// celle du dessous devenait introuvable. On ne s'en apercevait guère
/// tant qu'on les ouvrait une à une — depuis que la séance rouvre celles
/// qu'on avait laissées, on peut en retrouver douze au même endroit.
///
/// Chacune reçoit donc sa case dans une GRILLE, comme un jeu de cartes
/// qu'on étale. La case vient du RANG de la fenêtre dans
/// `FENÊTRES_REPRISES`, c'est-à-dire d'une table fixe : la place d'une
/// fenêtre ne dépend ni de ce qu'elle contient, ni de sa taille, ni de
/// ce qui est déjà ouvert. Rien ne se mesure, donc rien ne peut se
/// reboucler d'une image à l'autre — c'est le premier piège de cette
/// interface, et il a déjà coûté trois corrections retirées.
///
/// Une grille, et non plus une diagonale à pas unique : celle-ci
/// n'étalait que sur un axe, et son pas écrêté à vingt-quatre pixels
/// tassait douze fenêtres dans deux cent soixante pixels — sur l'écran
/// de cette machine, dix-huit paires se recouvraient entièrement, et
/// l'on cherchait dans le menu Outils des fenêtres qu'on croyait
/// fermées. Le contrôle d'alors ne voyait rien : il mesurait l'écart
/// entre centres, que huit pixels contentaient.
///
/// La grille est CENTRÉE sur le milieu de l'écran, et son pas est
/// proportionnel à lui : l'éventail couvre la majeure partie de la
/// place quand il y en a, et se resserre quand il n'y en a plus. Un
/// plafond le retient de partir aux quatre coins d'un très grand écran,
/// où l'ensemble doit rester groupé là où l'œil va le chercher.
///
/// Une fenêtre absente de la table garde le centre : c'est ce qu'il faut
/// pour les boîtes modales — Préférences, « À propos », la saisie du
/// programme —, qui ne s'ouvrent jamais à plusieurs. La fiche d'aide,
/// elle, a son propre coin en bas à droite et ne demande pas de cran :
/// on la lit en continuant d'écrire.
fn default_window_pos(ctx: &egui::Context, nom: &str) -> egui::Pos2 {
    default_window_pos_sized(ctx, nom, egui::Vec2::ZERO)
}

/// Le même, pour une fenêtre dont on sait déjà la taille qu'elle
/// demande.
///
/// UNE FENÊTRE QUI REMPLIT L'ÉCRAN NE PREND PAS DE CRAN : il n'y a pas
/// de place pour l'étaler, et l'écart la ferait sortir — egui la ramène
/// alors, en rognant ce qu'elle montre. Le graphe en a fait la preuve :
/// chargé, il s'ouvre à quatre-vingt-douze pour cent de la largeur, et
/// son cran lui faisait écrire le numéro d'un bloc par-dessus sa propre
/// légende.
///
/// La taille passée est celle que la fenêtre DEMANDE, un calcul et non
/// une mesure de l'image précédente : rien ne se reboucle.
fn default_window_pos_sized(ctx: &egui::Context, nom: &str, voulue: egui::Vec2) -> egui::Pos2 {
    let écran = ctx.viewport_rect();
    if voulue.x > écran.width() * 0.66 || voulue.y > écran.height() * 0.66 {
        return écran.center();
    }
    let Some(rang) = FENÊTRES_REPRISES.iter().position(|(id, _, _)| *id == nom) else {
        return écran.center();
    };
    let combien = FENÊTRES_REPRISES.len() as f32;
    // Le nombre de colonnes suit la LARGEUR DISPONIBLE, et non la
    // racine du nombre de fenêtres : à 1280 points de large — ce que
    // donne cet écran sous une densité de trois —, quatre colonnes
    // écartent tant la première qu'egui la plaque contre le bord, par
    // dessus les pastilles de la barre de titre. Une fenêtre ordinaire
    // demande ses six cents points ; on en met autant que l'écran en
    // porte, et jamais plus de quatre.
    const LARGEUR_TYPE: f32 = 640.0;
    let colonnes = (écran.width() / LARGEUR_TYPE).floor().clamp(1.0, 4.0);
    let lignes = (combien / colonnes).ceil().max(1.0);
    // Le pas : une bonne part de l'écran, sans dépasser la largeur d'une
    // fenêtre ordinaire. Les deux bornes se mesurent — au-dessus du
    // plafond, les fenêtres se dispersent aux bords et egui les y ramène
    // en tas ; en dessous du plancher, elles se tassent au centre et
    // s'enterrent. Les valeurs retenues laissent à chaque poignée au
    // moins deux cents pixels libres à partir de 1920 × 1200.
    let pas_x = (écran.width() * 0.7 / colonnes).clamp(24.0, 320.0);
    let pas_y = (écran.height() * 0.7 / lignes).clamp(24.0, 260.0);
    let colonne = (rang as f32) % colonnes;
    let ligne = (rang as f32 / colonnes).floor();
    // UNE LIGNE SUR DEUX EST DÉCALÉE d'un quart de pas, de part et
    // d'autre. Sans ce quinconce, deux fenêtres de la même colonne sur
    // des lignes voisines se superposent EXACTEMENT en largeur : il
    // suffit alors que l'une soit plus haute que le pas pour couvrir la
    // barre de titre de l'autre sur toute sa longueur. Mesuré aux
    // quatre résolutions du contrôle, le quart de pas fait passer le
    // compte des fenêtres enterrées de sept à cinq ; la demie, qui
    // paraissait plus franche, le remonte à onze — elle recolle les
    // colonnes voisines l'une sur l'autre.
    let quinconce = if (ligne as i32) % 2 == 0 { -0.25 } else { 0.25 };
    let cran_x = colonne - (colonnes - 1.0) / 2.0 + quinconce;
    let cran_y = ligne - (lignes - 1.0) / 2.0;
    écran.center() + egui::vec2(cran_x * pas_x, cran_y * pas_y)
}

#[allow(clippy::type_complexity)]
pub(crate) const FENÊTRES_REPRISES: &[(&str, fn(&NamsdecApp) -> bool, fn(&mut NamsdecApp))] = &[
    (
        "calculator",
        |a| a.show_calculator,
        |a| a.show_calculator = true,
    ),
    ("help", |a| a.show_help, |a| a.show_help = true),
    ("vectors", |a| a.show_vectors, |a| a.show_vectors = true),
    (
        "breakpoints",
        |a| a.show_breakpoints,
        |a| a.show_breakpoints = true,
    ),
    ("timeline", |a| a.show_timeline, |a| a.show_timeline = true),
    ("cyclic", |a| a.show_cyclic, |a| a.show_cyclic = true),
    ("analysis", |a| a.show_analysis, |a| a.show_analysis = true),
    (
        "disassembly",
        |a| a.show_disassembly,
        |a| a.show_disassembly = true,
    ),
    ("flow", |a| a.show_flow, |a| a.show_flow = true),
    ("patches", |a| a.show_patches, |a| a.show_patches = true),
    (
        "decompiler",
        |a| a.show_decompiler,
        |a| {
            a.show_decompiler = a.show_decompiler_module;
        },
    ),
    (
        "exercises",
        |a| a.show_exercises,
        |a| {
            a.show_exercises = a.exercises;
        },
    ),
];

/// Ce qu'on fait aux octets d'une instruction.
///
/// Deux gestes, et ils couvrent presque tous les cas : inverser un saut
/// conditionnel pour prendre l'autre branche, ou neutraliser une
/// instruction pour qu'elle ne fasse rien. Les deux gardent la longueur
/// exacte — un octet de plus ou de moins décalerait tout ce qui suit.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PatchGeste {
    Inverser,
    Neutraliser,
    /// Prendre la branche quoi qu'il arrive : `je` devient `jmp`.
    ///
    /// Inverser et forcer ne répondent pas à la même question.
    /// Inverser demande « et si la condition était l'autre ? » ; forcer
    /// demande « et si on y allait de toute façon ? ». C'est le second
    /// qu'on veut devant un contrôle qu'on n'a pas encore compris.
    Forcer,
}

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

/// Les registres vectoriels, dans l'ordre de leur numéro.
///
/// TRENTE-DEUX : les seize derniers n'existent qu'avec AVX-512, dont
/// l'encodage EVEX a quatre bits pour en nommer un. L'affichage ne les
/// montre que lorsqu'ils portent quelque chose — voir `xmm_ui` —, de la
/// même façon que les registres de masque.
const XMM_REGISTERS: [(&str, namsdec_core::RegXmm); 32] = {
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
        ("XMM16", Xmm16),
        ("XMM17", Xmm17),
        ("XMM18", Xmm18),
        ("XMM19", Xmm19),
        ("XMM20", Xmm20),
        ("XMM21", Xmm21),
        ("XMM22", Xmm22),
        ("XMM23", Xmm23),
        ("XMM24", Xmm24),
        ("XMM25", Xmm25),
        ("XMM26", Xmm26),
        ("XMM27", Xmm27),
        ("XMM28", Xmm28),
        ("XMM29", Xmm29),
        ("XMM30", Xmm30),
        ("XMM31", Xmm31),
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

/// Mélange deux couleurs. `part` vaut 0 pour la première, 1 pour la
/// seconde.
///
/// Et non `gamma_multiply`, qui ne touche que l'alpha : une touche
/// peinte à vingt pour cent d'opacité sur un panneau sombre donne une
/// touche sombre, qu'on ne distingue pas du fond. Un mélange rend une
/// couleur **opaque**, celle qu'on aurait choisie à la main, et le
/// clavier se détache alors du panneau au lieu de s'y fondre.
fn mélange(a: egui::Color32, b: egui::Color32, part: f32) -> egui::Color32 {
    let part = part.clamp(0.0, 1.0);
    let mêle = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * part) as u8;
    egui::Color32::from_rgb(mêle(a.r(), b.r()), mêle(a.g(), b.g()), mêle(a.b(), b.b()))
}

/// L'encre qui se lit le mieux sur ce fond.
///
/// Celle des deux du thème — le texte ou le fond de l'éditeur — qui
/// contraste le plus avec lui. Choisir à la main marcherait sur un
/// thème sombre et donnerait du gris sur gris sur un thème clair ; ici
/// la règle vaut pour les deux, et pour ceux qu'on installera.
///
/// Le *contraste*, et non l'écart de clarté : les deux ne classent pas
/// pareil. La clarté d'un `#d4d4d4` est à 79 d'un bleu franc et à 187
/// du fond sombre, ce qui désignait le gris clair ; mesuré comme l'œil
/// le reçoit, ce gris ne donne que 2,9 sur ce bleu quand l'encre sombre
/// en donne 4,0. Les touches d'opérateur de la calculatrice portaient
/// donc l'encre la moins lisible des deux.
fn encre_sur(fond: egui::Color32, theme: &Theme) -> egui::Color32 {
    let clair = namsdec_theme::contrast_ratio(fond, theme.text);
    let sombre = namsdec_theme::contrast_ratio(fond, theme.bg);
    if clair >= sombre {
        theme.text
    } else {
        theme.bg
    }
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
    let (rect, réponse) =
        ui.allocate_exact_size(egui::vec2(largeur, HAUTEUR), egui::Sense::click_and_drag());
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
            egui::pos2(rect.left() + i as f32 * pas, rect.bottom() - part * HAUTEUR),
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
    let visé = réponse
        .interact_pointer_pos()
        .or(réponse.hover_pos())
        .map(|position| timeline::step_at(trace.len(), (position.x - rect.left()) / largeur));
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
        // Mille et non neuf cents : la colonne porte vingt-cinq
        // rubriques depuis que chaque outil extérieur a la sienne, et
        // les quatre dernières passaient sous le bord. Une rubrique
        // qu'on ne voit pas est une rubrique dont on ignore
        // l'existence.
        (écran.y * 0.82).clamp(420.0, 1000.0),
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

/// Ce que la ligne de commande demande.
///
/// namsdec ignorait purement et simplement ses arguments — alors que son
/// propre lanceur déclare `Exec=namsdec %f` et `MimeType=text/x-asm`,
/// c'est-à-dire qu'il se propose d'ouvrir les fichiers `.asm` du
/// gestionnaire de fichiers. « Ouvrir avec namsdec » ouvrait donc une
/// fenêtre vide, ce qui est pire que de ne rien proposer.
///
/// Et `namsdec --version` lançait l'interface graphique : un programme
/// qu'on installe dans son PATH doit savoir répondre à la question la
/// plus simple qu'on lui pose depuis un terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intention {
    /// Ouvrir l'interface, éventuellement sur un fichier.
    Ouvrir(Option<PathBuf>),
    /// Écrire une ligne et s'arrêter.
    Dire(String),
}

/// Lit les arguments et dit ce qu'ils demandent.
///
/// Séparée de `main` pour être éprouvée : elle ne touche ni au disque ni
/// à l'écran, elle ne fait que décider.
pub fn intention(arguments: &[String]) -> Intention {
    let mut fichier = None;
    for argument in arguments {
        match argument.as_str() {
            "--version" | "-V" => {
                return Intention::Dire(format!("{APP_NAME} {APP_VERSION}"));
            }
            "--help" | "-h" => {
                return Intention::Dire(format!(
                    "{APP_NAME} {APP_VERSION}\n\n\
                     Usage : namsdec [FICHIER]\n\n\
                     \x20 FICHIER       un source assembleur ou un binaire à ouvrir\n\
                     \x20 -h, --help    afficher cette aide\n\
                     \x20 -V, --version afficher la version"
                ));
            }
            // Un argument inconnu qui commence par un tiret est une
            // faute de frappe, non un nom de fichier : le prendre pour
            // un fichier donnerait « fichier introuvable : --verison »,
            // ce qui n'aide personne.
            autre if autre.starts_with('-') => {
                return Intention::Dire(format!(
                    "namsdec : argument inconnu « {autre} ». Essayez namsdec --help."
                ));
            }
            autre => fichier = Some(PathBuf::from(autre)),
        }
    }
    Intention::Ouvrir(fichier)
}

fn main() -> eframe::Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let à_ouvrir = match intention(&arguments) {
        Intention::Dire(texte) => {
            println!("{texte}");
            return Ok(());
        }
        Intention::Ouvrir(fichier) => fichier,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size(MIN_SCREEN)
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
            // LES MODULES D'AVANT SONT REPRIS ICI, et nulle part
            // ailleurs. C'est une ÉCRITURE dans le dossier de données de
            // la personne : la poser dans `NamsdecApp::default()`
            // reviendrait à la faire à chaque construction de
            // l'application — y compris dans les six cent quatre-vingt-dix
            // contrôles, qui écriraient alors dans les fichiers de
            // quelqu'un. C'est arrivé en l'écrivant, et c'est la raison
            // de ce commentaire.
            //
            // Ici, l'application est celle qui démarre pour de vrai.
            tools::migrer_les_modules();
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
            // Et maintenant seulement, les raccourcis de la machine :
            // `from_settings` ne lit pas le disque, pour que les tests
            // qui s'en servent ne dépendent pas de ce que l'utilisateur
            // a réglé.
            app.adopt_keymap_from_disk();
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
            // Le fichier passé sur la ligne de commande, en dernier :
            // il prend la place de ce que la séance précédente avait
            // rouvert, parce qu'on vient de le demander explicitement.
            if let Some(chemin) = à_ouvrir.clone() {
                app.open_file(chemin);
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
