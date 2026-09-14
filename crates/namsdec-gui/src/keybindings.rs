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
    /// Les conventions de VS Code, dans la mesure où elles survivent au
    /// bureau Linux : F5/Maj+F5/Ctrl+Maj+F5 pour Run/Stop/Reset et
    /// Ctrl+Maj+B pour Assembler ("Run Build Task") restent tels quels.
    /// F10/F11 nus, en revanche, sont trop souvent interceptés avant même
    /// d'atteindre l'appli (F11 = plein écran quasi universel au niveau du
    /// gestionnaire de fenêtres, F10 = mnémonique "activer le menu" côté
    /// GTK) — Step Into/Step Over utilisent donc Ctrl+Alt+I/O à la place,
    /// pour rester fiables plutôt que fidèles à la lettre. `Quitter` n'a
    /// volontairement aucun raccourci par défaut : une frappe malheureuse
    /// qui ferme l'appli est pire qu'un clic de menu en plus.
    fn default() -> Self {
        let mut map = BTreeMap::new();
        map.insert(
            Action::Assemble,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::B)),
        );
        map.insert(Action::Run, Chord::Simple(KeyboardShortcut::new(Modifiers::NONE, Key::F5)));
        map.insert(
            Action::StepOver,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL | Modifiers::ALT, Key::O)),
        );
        map.insert(
            Action::StepInto,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL | Modifiers::ALT, Key::I)),
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
        let pressed = pressed_keys_this_frame(ctx);

        if let Some(&key) = pressed.first() {
            // Correspondance EXACTE des modificateurs (pas
            // `matches_logically`, qui ignore volontairement les
            // modificateurs *en trop* par rapport au motif — F5 sans
            // modificateur "matchait" donc tout aussi bien Maj+F5, ce qui
            // rendait Stop/Reset injoignables tant que Run (F5 nu) était
            // testé en premier). Avec une correspondance exacte, au plus un
            // seul raccourci peut matcher une combinaison de touches
            // donnée : plus besoin de départager par ordre ou spécificité.
            let modifiers = ctx.input(|i| i.modifiers);
            for (&action, chord) in &bindings.map {
                if let Chord::Simple(sc) = chord
                    && sc.logical_key == key
                    && modifiers.matches_exact(sc.modifiers)
                {
                    self.leader_armed = None;
                    return Some(action);
                }
            }
        }

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
