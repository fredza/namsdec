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
use crate::keybindings::{Action, Chord, KeyBindings};

/// `#[serde(default)]` au niveau de la struct : un champ absent du fichier
/// est repris de `Settings::default()`, donc une configuration écrite par
/// une version plus ancienne reste lisible.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub show_toolbar: bool,
    pub show_sidebar: bool,
    pub show_statusbar: bool,
    pub show_re_tab: bool,
    /// Dernier dossier ouvert dans l'explorateur, pour le retrouver au
    /// lancement suivant.
    pub explorer_root: Option<String>,
    /// Identifiant d'action -> raccourci, en texte.
    pub shortcuts: BTreeMap<String, String>,
}

impl Default for Settings {
    /// Les défauts de l'application, pas ceux de `Default` sur chaque type
    /// — sans quoi un fichier partiel masquerait les barres au lieu de les
    /// laisser visibles.
    fn default() -> Self {
        Self {
            theme: namsdec_theme::VSCODE_DARK.id.to_owned(),
            language: "fr".to_owned(),
            show_toolbar: true,
            show_sidebar: true,
            show_statusbar: true,
            show_re_tab: false,
            explorer_root: None,
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

    /// Lit le fichier. Toute erreur — absent, JSON invalide, droits —
    /// redonne les valeurs par défaut : une configuration abîmée ne doit
    /// pas empêcher de lancer l'application.
    pub fn load() -> Self {
        settings_path().map(Self::load_from).unwrap_or_default()
    }

    /// Écrit le fichier, en créant `~/.config/namsdec` au besoin.
    pub fn save(&self) -> std::io::Result<()> {
        let path = settings_path()
            .ok_or_else(|| std::io::Error::other("aucun dossier de configuration utilisable"))?;
        self.save_to(&path)
    }

    /// Le chemin est un paramètre plutôt qu'une constante interne : c'est
    /// ce qui rend l'aller-retour testable sans toucher aux variables
    /// d'environnement du processus (`set_var` n'est pas sûr entre
    /// threads, et les tests tournent en parallèle).
    pub fn load_from(path: impl AsRef<std::path::Path>) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
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
        Action::Stop => "stop",
        Action::Reset => "reset",
        Action::Save => "save",
        Action::OpenFile => "open_file",
        Action::OpenFolder => "open_folder",
    }
}

fn action_from_id(id: &str) -> Option<Action> {
    Action::ALL.into_iter().find(|a| action_id(*a) == id)
}

/// `"Ctrl+Shift+B"` pour un raccourci simple, `"Space,E"` pour une
/// séquence. La virgule ne peut pas apparaître dans un nom de touche
/// (`Key::name()` rend `"Comma"`), elle distingue donc les deux formes
/// sans ambiguïté.
fn chord_to_string(chord: &Chord) -> String {
    match chord {
        Chord::Leader(first, second) => format!("{},{}", first.name(), second.name()),
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
    if let Some((first, second)) = text.split_once(',') {
        return Some(Chord::Leader(
            Key::from_name(first.trim())?,
            Key::from_name(second.trim())?,
        ));
    }

    let mut parts: Vec<&str> = text.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
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
            chord_to_string(&Chord::Leader(Key::Space, Key::E)),
            "Space,E"
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
            ..Default::default()
        };
        written.save_to(&path).expect("l'écriture doit réussir");
        assert!(path.exists(), "le fichier doit être créé avec son dossier");

        let read = Settings::load_from(&path);
        assert_eq!(read, written);
        assert_eq!(read.theme().id, namsdec_theme::TOKYO_NIGHT.id);
        assert_eq!(read.explorer_root(), Some(PathBuf::from("/tmp/projet")));
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
