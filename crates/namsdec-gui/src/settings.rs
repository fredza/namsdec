//! Réglages persistants — `~/.config/namsdec/settings.json`.
//!
//! Règle du projet : **tout paramètre réglé par l'utilisateur est
//! sauvegardé**. Un nouveau réglage ajouté à l'interface s'ajoute ici en
//! même temps, jamais dans un second temps.
//!
//! Le fichier est volontairement écrit avec des valeurs *lisibles* —
//! l'identifiant du thème, "fr"/"en", et les raccourcis sous la forme
//! `"Ctrl+Shift+B"` ou `"Space,E"` — plutôt que la représentation interne
//! des types d'egui : il doit rester modifiable à la main, et un
//! changement de version d'egui ne doit pas invalider la configuration de
//! l'utilisateur.
//!
//! Tout y est tolérant à l'erreur : un fichier absent, illisible, d'une
//! version antérieure ou à moitié rempli redonne les valeurs par défaut
//! au lieu d'empêcher l'application de démarrer.

use std::collections::BTreeMap;
use std::path::PathBuf;

use eframe::egui;
use egui::{Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::keybindings::{Action, Chord, KeyBindings, Sequence};

/// `#[serde(default)]` au niveau de la struct : un champ absent du fichier
/// est repris de `Settings::default()`, donc une configuration écrite par
/// une version plus ancienne reste lisible.
/// Version du format de réglages. À incrémenter quand un *défaut* change
/// d'une façon qui doit atteindre les fichiers déjà écrits — voir
/// `migrate`.
pub const SETTINGS_VERSION: u32 = 2;

/// Un fichier écrit avant que le numéro de version n'existe est, par
/// définition, un fichier de version 1.
///
/// Sans cette fonction, le `#[serde(default)]` de la struct donnerait aux
/// champs absents la valeur de `Settings::default()` — donc la version
/// *courante*, et une configuration ancienne se présenterait comme à jour
/// sans jamais être migrée.
fn version_before_versioning() -> u32 {
    1
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    #[serde(default = "version_before_versioning")]
    pub version: u32,
    pub theme: String,
    pub language: String,
    pub show_toolbar: bool,
    pub show_sidebar: bool,
    pub show_statusbar: bool,
    /// Panneau bas — la console du programme et les diagnostics de nasm.
    pub show_console: bool,
    /// Les infobulles au survol. Certains les veulent, d'autres les
    /// trouvent envahissantes — d'où un réglage plutôt qu'un choix imposé.
    pub show_tooltips: bool,
    /// L'autocomplétion de l'éditeur : elle propose, `Alt` accepte.
    pub completion: bool,
    /// Entrée garde l'indentation de la ligne qu'on quitte.
    #[serde(default = "vrai")]
    pub auto_indent: bool,
    /// La fiche de l'instruction au survol du code.
    #[serde(default = "vrai")]
    pub hover_help: bool,
    /// Le module des exercices. Éteint par défaut : quelqu'un qui écrit
    /// son propre code n'a pas besoin qu'on lui propose des devoirs, et
    /// rien n'apparaît tant qu'il ne le demande pas.
    #[serde(default)]
    pub exercises: bool,
    /// Les séries d'exercices qu'on ne veut pas voir, par identifiant.
    ///
    /// Écartées et non désinstallées : on garde le fichier, on cesse
    /// seulement de le proposer. Celle qui est livrée avec l'application
    /// peut l'être aussi — elle ne s'enlève pas, elle se range.
    #[serde(default)]
    pub disabled_packs: Vec<String>,
    /// L'exercice chargé en dernier, sous la forme « série/exercice ».
    ///
    /// Un parcours de douze étapes se fait en plusieurs fois : au retour,
    /// l'outil doit pouvoir dire où l'on en était plutôt que de laisser
    /// chercher.
    #[serde(default)]
    pub current_exercise: Option<String>,
    /// Le moteur du panneau de désassemblage : « pseudo », « nasm », ou
    /// l'identifiant d'un outil installé. Celui qu'on a choisi doit
    /// revenir à la session suivante, comme le reste.
    #[serde(default = "pseudo_par_défaut")]
    pub disassembler: String,
    /// Comment l'onglet Mémoire lit les octets : « bytes », « words » ou
    /// « text ».
    pub memory_format: String,
    /// Dernier dossier ouvert dans l'explorateur, pour le retrouver au
    /// lancement suivant.
    pub explorer_root: Option<String>,
    /// Dernier fichier ouvert dans l'éditeur. Le **chemin**, jamais le
    /// contenu : le fichier source vit sur le disque, où l'utilisateur
    /// peut l'éditer avec autre chose, et en recopier une version dans les
    /// réglages ne ferait qu'y entretenir un double périmé.
    pub current_file: Option<String>,
    /// Identifiant d'action -> raccourci, en texte.
    pub shortcuts: BTreeMap<String, String>,
}

impl Default for Settings {
    /// Les défauts de l'application, pas ceux de `Default` sur chaque type
    /// — sans quoi un fichier partiel masquerait les barres au lieu de les
    /// laisser visibles.
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            theme: namsdec_theme::VSCODE_DARK.id.to_owned(),
            language: "fr".to_owned(),
            show_toolbar: true,
            show_sidebar: true,
            show_statusbar: true,
            show_console: true,
            show_tooltips: true,
            completion: true,
            exercises: false,
            auto_indent: true,
            hover_help: true,
            disabled_packs: Vec::new(),
            current_exercise: None,
            disassembler: pseudo_par_défaut(),
            memory_format: "bytes".to_owned(),
            explorer_root: None,
            current_file: None,
            shortcuts: shortcuts_to_strings(&KeyBindings::default()),
        }
    }
}

impl Settings {
    pub fn theme(&self) -> &'static namsdec_theme::Theme {
        namsdec_theme::by_id(&self.theme)
    }

    pub fn lang(&self) -> Lang {
        match self.language.as_str() {
            "en" => Lang::En,
            _ => Lang::Fr,
        }
    }

    pub fn keybindings(&self) -> KeyBindings {
        // On part des défauts : une action absente du fichier (ajoutée par
        // une version plus récente que celui-ci) garde son raccourci
        // d'origine au lieu de n'en avoir aucun.
        let mut bindings = KeyBindings::default();
        for (id, text) in &self.shortcuts {
            if let Some(action) = action_from_id(id) {
                match chord_from_string(text) {
                    Some(chord) => {
                        bindings.map.insert(action, chord);
                    }
                    // Raccourci illisible : on ne supprime pas l'action,
                    // on la laisse sur son défaut.
                    None => continue,
                }
            }
        }
        bindings
    }

    pub fn explorer_root(&self) -> Option<PathBuf> {
        self.explorer_root.as_ref().map(PathBuf::from)
    }

    pub fn current_file(&self) -> Option<PathBuf> {
        self.current_file.as_ref().map(PathBuf::from)
    }

    /// Lit le fichier. Toute erreur — absent, JSON invalide, droits —
    /// redonne les valeurs par défaut : une configuration abîmée ne doit
    /// pas empêcher de lancer l'application.
    pub fn load() -> Self {
        settings_path().map(Self::load_from).unwrap_or_default()
    }

    /// Le chemin est un paramètre plutôt qu'une constante interne : c'est
    /// ce qui rend l'aller-retour testable sans toucher aux variables
    /// d'environnement du processus (`set_var` n'est pas sûr entre
    /// threads, et les tests tournent en parallèle).
    pub fn load_from(path: impl AsRef<std::path::Path>) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .map(Self::migrate)
            .unwrap_or_default()
    }

    /// Fait suivre à un fichier existant les changements de défauts.
    ///
    /// Un raccourci par défaut qui change ne bouge pas tout seul chez ceux
    /// qui ont déjà lancé l'application : leur `settings.json` fige la
    /// valeur d'alors, et c'est bien le but — un réglage enregistré ne
    /// doit pas se faire écraser à chaque mise à jour. Mais un utilisateur
    /// qui n'a *jamais touché* à un raccourci n'a rien choisi du tout : il
    /// a hérité d'un défaut, et il doit hériter du nouveau.
    ///
    /// D'où cette migration, qui ne remplace un raccourci que s'il vaut
    /// encore exactement l'ancien défaut. Toute personnalisation, même
    /// identique au nouveau défaut, est laissée intacte.
    fn migrate(mut self) -> Self {
        if self.version < 2 {
            // v1 -> v2 : les commandes d'exécution passent de touches de
            // fonction aux séquences de gdb/pwndbg (`Espace r`,
            // `Espace s i`, `Espace n i`).
            const V1_DEFAULTS: [(&str, &str); 3] = [
                ("run", "F5"),
                ("step_into", "Ctrl+Alt+I"),
                ("step_over", "Ctrl+Alt+O"),
            ];
            let new_defaults = shortcuts_to_strings(&KeyBindings::default());
            for (id, v1_default) in V1_DEFAULTS {
                let untouched = self.shortcuts.get(id).is_some_and(|s| s == v1_default);
                if untouched && let Some(new_default) = new_defaults.get(id) {
                    self.shortcuts.insert(id.to_owned(), new_default.clone());
                }
            }
        }
        self.version = SETTINGS_VERSION;
        self
    }

    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }
}

/// `$XDG_CONFIG_HOME/namsdec/settings.json`, sinon
/// `$HOME/.config/namsdec/settings.json`.
pub fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("namsdec").join("settings.json"))
}

pub fn shortcuts_to_strings(bindings: &KeyBindings) -> BTreeMap<String, String> {
    bindings
        .map
        .iter()
        .map(|(action, chord)| (action_id(*action).to_owned(), chord_to_string(chord)))
        .collect()
}

/// Le moteur de départ : le pseudo-code, celui qu'on veut lire en
/// premier quand on apprend.
fn pseudo_par_défaut() -> String {
    "pseudo".to_owned()
}

/// La valeur d'un réglage absent d'un fichier écrit par une version
/// plus ancienne : on l'allume, comme il l'est pour qui découvre.
fn vrai() -> bool {
    true
}

/// Identifiant stable d'une action dans le fichier — délibérément distinct
/// du nom de la variante Rust, qu'on doit pouvoir renommer sans casser la
/// configuration des utilisateurs.
fn action_id(action: Action) -> &'static str {
    match action {
        Action::ToggleExplorer => "toggle_explorer",
        Action::Assemble => "assemble",
        Action::Run => "run",
        Action::StepInto => "step_into",
        Action::StepOver => "step_over",
        Action::StepBack => "step_back",
        Action::Stop => "stop",
        Action::Reset => "reset",
        Action::Save => "save",
        Action::OpenFile => "open_file",
        Action::OpenFolder => "open_folder",
        Action::InstructionHelp => "instruction_help",
        Action::RunToCursor => "run_to_cursor",
        Action::GoToLabel => "goto_label",
        Action::Find => "find",
        Action::New => "new",
    }
}

fn action_from_id(id: &str) -> Option<Action> {
    Action::ALL.into_iter().find(|a| action_id(*a) == id)
}

/// `"Ctrl+Shift+B"` pour un raccourci simple, `"Space,S,I"` pour une
/// séquence. La virgule ne peut pas apparaître dans un nom de touche
/// (`Key::name()` rend `"Comma"`), elle distingue donc les deux formes
/// sans ambiguïté, et en sépare les touches quel qu'en soit le nombre.
fn chord_to_string(chord: &Chord) -> String {
    match chord {
        Chord::Leader(sequence) => sequence
            .as_slice()
            .iter()
            .map(|k| k.name())
            .collect::<Vec<_>>()
            .join(","),
        Chord::Simple(shortcut) => {
            let m = shortcut.modifiers;
            let mut parts = Vec::new();
            for (present, name) in [
                (m.ctrl, "Ctrl"),
                (m.alt, "Alt"),
                (m.shift, "Shift"),
                (m.mac_cmd, "Cmd"),
                (m.command, "Command"),
            ] {
                if present {
                    parts.push(name);
                }
            }
            parts.push(shortcut.logical_key.name());
            parts.join("+")
        }
    }
}

fn chord_from_string(text: &str) -> Option<Chord> {
    let text = text.trim();
    if text.contains(',') {
        let keys: Option<Vec<Key>> = text
            .split(',')
            .map(|name| Key::from_name(name.trim()))
            .collect();
        return Sequence::new(&keys?).map(Chord::Leader);
    }

    let mut parts: Vec<&str> = text
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let key = Key::from_name(parts.pop()?)?;
    let mut modifiers = Modifiers::NONE;
    for part in parts {
        match part {
            "Ctrl" => modifiers.ctrl = true,
            "Alt" => modifiers.alt = true,
            "Shift" => modifiers.shift = true,
            "Cmd" => modifiers.mac_cmd = true,
            "Command" => modifiers.command = true,
            _ => return None,
        }
    }
    Some(Chord::Simple(KeyboardShortcut::new(modifiers, key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque raccourci par défaut doit survivre à l'aller-retour texte :
    /// c'est ce qui décide si la configuration écrite sur le disque est
    /// relue correctement au lancement suivant.
    #[test]
    fn every_default_shortcut_survives_a_round_trip() {
        let defaults = KeyBindings::default();
        for (action, chord) in &defaults.map {
            let text = chord_to_string(chord);
            assert_eq!(
                chord_from_string(&text).as_ref(),
                Some(chord),
                "{action:?} écrit « {text} » ne se relit pas à l'identique"
            );
        }

        // Et le tour complet, en passant par la même conversion que le
        // fichier : les défauts se relisent bien comme les défauts.
        let settings = Settings::default();
        assert_eq!(settings.keybindings().map, defaults.map);
    }

    #[test]
    fn shortcut_text_is_the_readable_form() {
        assert_eq!(
            chord_to_string(&Chord::Simple(KeyboardShortcut::new(
                Modifiers::CTRL | Modifiers::SHIFT,
                Key::B
            ))),
            "Ctrl+Shift+B"
        );
        assert_eq!(
            chord_to_string(&Chord::leader(&[Key::Space, Key::E])),
            "Space,E"
        );
        // Une séquence de trois touches, celle de Step Into.
        assert_eq!(
            chord_to_string(&Chord::leader(&[Key::Space, Key::S, Key::I])),
            "Space,S,I"
        );
        assert_eq!(
            chord_from_string("Space,S,I"),
            Some(Chord::leader(&[Key::Space, Key::S, Key::I]))
        );
    }

    /// Un fichier écrit par une version antérieure — ici, qui ne connaît
    /// ni le thème ni la moitié des réglages — doit se relire sans perdre
    /// ce qu'il contient ni inventer de faux défauts.
    #[test]
    fn a_partial_file_keeps_its_values_and_defaults_the_rest() {
        let settings: Settings =
            serde_json::from_str(r#"{"language":"en","show_sidebar":false}"#).unwrap();

        assert_eq!(settings.lang(), Lang::En);
        assert!(!settings.show_sidebar);
        // Absents du fichier : les défauts de l'application, pas ceux des
        // types (un `bool` manquant ne doit pas masquer la barre d'outils).
        assert!(settings.show_toolbar);
        assert_eq!(settings.theme().id, namsdec_theme::VSCODE_DARK.id);
        assert_eq!(settings.keybindings().map, KeyBindings::default().map);
    }

    /// Un fichier écrit avant que les raccourcis d'exécution ne changent
    /// doit recevoir les nouveaux — sinon changer un défaut ne toucherait
    /// jamais personne ayant déjà lancé l'application une fois, puisque
    /// son fichier fige l'ancien.
    #[test]
    fn an_old_file_receives_the_new_execution_shortcuts() {
        let settings: Settings = serde_json::from_str(
            r#"{"shortcuts":{"run":"F5","step_into":"Ctrl+Alt+I","step_over":"Ctrl+Alt+O"}}"#,
        )
        .unwrap();
        // Un fichier sans numéro de version est un fichier de version 1.
        assert_eq!(settings.version, 1);

        let migrated = settings.migrate();
        assert_eq!(migrated.version, SETTINGS_VERSION);
        assert_eq!(migrated.shortcuts["run"], "Space,R");
        assert_eq!(migrated.shortcuts["step_into"], "Space,S,I");
        assert_eq!(migrated.shortcuts["step_over"], "Space,N,I");

        // Et ils se relisent bien comme des séquences.
        let bindings = migrated.keybindings();
        assert_eq!(
            bindings.map.get(&Action::StepInto),
            Some(&Chord::leader(&[Key::Space, Key::S, Key::I]))
        );
    }

    /// Un raccourci que l'utilisateur a choisi lui-même n'est pas un
    /// défaut hérité : la migration doit le laisser tranquille.
    #[test]
    fn migration_never_overwrites_a_customised_shortcut() {
        let settings: Settings =
            serde_json::from_str(r#"{"shortcuts":{"run":"Ctrl+Alt+P","step_into":"F7"}}"#).unwrap();

        let migrated = settings.migrate();
        assert_eq!(migrated.shortcuts["run"], "Ctrl+Alt+P");
        assert_eq!(migrated.shortcuts["step_into"], "F7");
    }

    /// Une configuration déjà à jour traverse la migration sans bouger.
    #[test]
    fn a_current_file_is_left_alone() {
        let settings = Settings::default();
        assert_eq!(settings.clone().migrate(), settings);
    }

    #[test]
    fn a_corrupt_file_falls_back_instead_of_failing() {
        assert_eq!(
            serde_json::from_str::<Settings>("{ pas du json").unwrap_or_default(),
            Settings::default()
        );
        // Un identifiant de thème inconnu (thème désinstallé) ne doit pas
        // laisser l'application sans thème.
        let settings = Settings {
            theme: "thème-disparu".to_owned(),
            ..Default::default()
        };
        assert_eq!(settings.theme().id, namsdec_theme::VSCODE_DARK.id);
    }

    #[test]
    fn an_unreadable_shortcut_keeps_the_default_for_that_action() {
        let mut settings = Settings::default();
        settings
            .shortcuts
            .insert("run".to_owned(), "Ctrl+Pas_Une_Touche".to_owned());

        let bindings = settings.keybindings();
        assert_eq!(
            bindings.map.get(&Action::Run),
            KeyBindings::default().map.get(&Action::Run),
            "un raccourci illisible ne doit pas laisser l'action sans raccourci"
        );
    }

    /// Écriture puis relecture d'un vrai fichier, dossier parent compris.
    #[test]
    fn saving_then_loading_round_trips_through_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        // Un sous-dossier qui n'existe pas encore : `save_to` doit le
        // créer, comme il créera `~/.config/namsdec` au premier lancement.
        let path = dir.path().join("namsdec").join("settings.json");

        let written = Settings {
            theme: namsdec_theme::TOKYO_NIGHT.id.to_owned(),
            language: "en".to_owned(),
            show_statusbar: false,
            explorer_root: Some("/tmp/projet".to_owned()),
            current_file: Some("/tmp/projet/main.asm".to_owned()),
            ..Default::default()
        };
        written.save_to(&path).expect("l'écriture doit réussir");
        assert!(path.exists(), "le fichier doit être créé avec son dossier");

        let read = Settings::load_from(&path);
        assert_eq!(read, written);
        assert_eq!(read.theme().id, namsdec_theme::TOKYO_NIGHT.id);
        assert_eq!(read.explorer_root(), Some(PathBuf::from("/tmp/projet")));
        assert_eq!(
            read.current_file(),
            Some(PathBuf::from("/tmp/projet/main.asm"))
        );
    }

    #[test]
    fn a_missing_file_gives_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            Settings::load_from(dir.path().join("jamais-écrit.json")),
            Settings::default()
        );
    }
}
