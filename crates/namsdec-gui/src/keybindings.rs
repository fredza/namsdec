//! Raccourcis clavier configurables. Deux formes de "chord" : un raccourci
//! simple (modificateurs + touche, ex. Ctrl+S) et une séquence "touche
//! leader" à la Vim (ex. Espace puis E) — la seule façon de proposer un
//! raccourci sur une touche aussi utilisée qu'Espace sans la voler au
//! texte tapé dans l'éditeur.

use std::collections::BTreeMap;

use eframe::egui;
use egui::{Key, KeyboardShortcut, Modifiers};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Action {
    ToggleExplorer,
    Assemble,
    Run,
    StepInto,
    StepOver,
    Stop,
    Reset,
    Save,
}

impl Action {
    pub const ALL: [Action; 8] = [
        Action::ToggleExplorer,
        Action::Assemble,
        Action::Run,
        Action::StepInto,
        Action::StepOver,
        Action::Stop,
        Action::Reset,
        Action::Save,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    /// Modificateurs + une touche, tous appuyés ensemble.
    Simple(KeyboardShortcut),
    /// Une touche, relâchée, puis une seconde touche juste après — style
    /// touche leader de Vim/Neovim. Armé seulement quand l'éditeur de code
    /// n'a pas le focus (sinon "Espace" taperait un espace *et* armerait
    /// la séquence à chaque mot).
    Leader(Key, Key),
}

impl Chord {
    pub fn display(&self, ctx: &egui::Context) -> String {
        match self {
            Chord::Simple(sc) => ctx.format_shortcut(sc),
            Chord::Leader(a, b) => format!("{} , {}", a.name(), b.name()),
        }
    }
}

pub struct KeyBindings {
    pub map: BTreeMap<Action, Chord>,
}

impl Default for KeyBindings {
    /// Les conventions de VS Code, délibérément — un clin d'œil cohérent
    /// avec le reste de l'app, pas des choix arbitraires : F5/F10/F11 pour
    /// Run/Step Over/Step Into, Maj+F5 pour Stop, Ctrl+Maj+F5 pour Reset
    /// ("Restart" côté VS Code), Ctrl+Maj+B pour Assembler ("Run Build
    /// Task"). `Quitter` n'a volontairement aucun raccourci par défaut :
    /// une frappe malheureuse qui ferme l'appli est pire qu'un clic de menu
    /// en plus.
    fn default() -> Self {
        let mut map = BTreeMap::new();
        map.insert(
            Action::Assemble,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::B)),
        );
        map.insert(Action::Run, Chord::Simple(KeyboardShortcut::new(Modifiers::NONE, Key::F5)));
        map.insert(
            Action::StepOver,
            Chord::Simple(KeyboardShortcut::new(Modifiers::NONE, Key::F10)),
        );
        map.insert(
            Action::StepInto,
            Chord::Simple(KeyboardShortcut::new(Modifiers::NONE, Key::F11)),
        );
        map.insert(
            Action::Stop,
            Chord::Simple(KeyboardShortcut::new(Modifiers::SHIFT, Key::F5)),
        );
        map.insert(
            Action::Reset,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::F5)),
        );
        map.insert(Action::Save, Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL, Key::S)));
        map.insert(Action::ToggleExplorer, Chord::Leader(Key::Space, Key::E));
        Self { map }
    }
}

/// Mode choisi pendant la capture d'un nouveau raccourci dans Préférences.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RebindMode {
    Simple,
    Leader,
}

/// État d'une capture de raccourci en cours.
pub struct Rebinding {
    pub action: Action,
    pub mode: RebindMode,
    /// Mode Leader seulement : la première touche déjà capturée, en
    /// attente de la seconde.
    first_captured: Option<Key>,
}

impl Rebinding {
    pub fn new(action: Action, mode: RebindMode) -> Self {
        Self {
            action,
            mode,
            first_captured: None,
        }
    }
}

/// État d'exécution du dispatch de raccourcis — distinct de `KeyBindings`
/// (pure configuration) parce qu'il porte la petite machine à états de la
/// touche leader entre deux frames.
#[derive(Default)]
pub struct Dispatcher {
    leader_armed: Option<Key>,
}

impl Dispatcher {
    /// À appeler une fois par frame, avant de construire les panneaux.
    /// `editor_focused` conditionne l'armement de la touche leader (voir
    /// la note sur `Chord::Leader`).
    pub fn poll(
        &mut self,
        ctx: &egui::Context,
        bindings: &KeyBindings,
        editor_focused: bool,
    ) -> Option<Action> {
        for (&action, chord) in &bindings.map {
            if let Chord::Simple(sc) = chord
                && ctx.input_mut(|i| i.consume_shortcut(sc)) {
                    self.leader_armed = None;
                    return Some(action);
                }
        }

        let pressed = pressed_keys_this_frame(ctx);

        if let Some(first) = self.leader_armed.take() {
            if let Some(&second) = pressed.first() {
                for (&action, chord) in &bindings.map {
                    if *chord == Chord::Leader(first, second) {
                        return Some(action);
                    }
                }
            }
            return None;
        }

        if !editor_focused
            && let Some(&k) = pressed.first() {
                let arms_a_leader = bindings
                    .map
                    .values()
                    .any(|c| matches!(c, Chord::Leader(first, _) if *first == k));
                if arms_a_leader {
                    self.leader_armed = Some(k);
                }
            }

        None
    }

    /// À appeler une fois par frame pendant qu'une capture (`Rebinding`)
    /// est en cours. Renvoie le nouveau `Chord` dès qu'il est complet.
    pub fn poll_rebind(&mut self, ctx: &egui::Context, rebinding: &mut Rebinding) -> Option<Chord> {
        let pressed = pressed_keys_this_frame(ctx);
        let &key = pressed.first()?;

        match rebinding.mode {
            RebindMode::Simple => {
                let modifiers = ctx.input(|i| i.modifiers);
                Some(Chord::Simple(KeyboardShortcut::new(modifiers, key)))
            }
            RebindMode::Leader => match rebinding.first_captured {
                None => {
                    rebinding.first_captured = Some(key);
                    None
                }
                Some(first) => Some(Chord::Leader(first, key)),
            },
        }
    }
}

fn pressed_keys_this_frame(ctx: &egui::Context) -> Vec<Key> {
    ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } => Some(*key),
                _ => None,
            })
            .collect()
    })
}
