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

        // Parcourt CHAQUE touche pressée cette frame, dans l'ordre — pas
        // seulement la première. Si l'appli n'a pas repeint entre les deux
        // frappes d'une séquence (rien ne force un repaint entre elles),
        // Espace *et* E peuvent atterrir dans le même lot d'évènements
        // d'une seule frame ; ne regarder que `pressed.first()` armait
        // Espace puis jetait silencieusement le E qui suivait dans la même
        // frame, sans jamais compléter la séquence.
        for &(key, modifiers) in &pressed {
            // 1. Raccourcis simples — correspondance EXACTE des
            // modificateurs (pas `matches_logically`, qui tolère des
            // modificateurs *en trop* par rapport au motif), lus sur
            // l'évènement lui-même — pas sur l'agrégat `InputState::
            // modifiers`, qui ne se met à jour que sur
            // `Event::ModifiersChanged` et peut donc être en retard d'une
            // frame par rapport au `Event::Key` qu'on est en train de
            // traiter. C'est aussi exactement ce que fait en interne
            // `consume_shortcut`/`count_and_consume_key` d'egui.
            let simple_match = bindings.map.iter().find_map(|(&action, chord)| match chord {
                Chord::Simple(sc) if sc.logical_key == key && modifiers.matches_exact(sc.modifiers) => {
                    Some(action)
                }
                _ => None,
            });
            if let Some(action) = simple_match {
                self.leader_armed = None;
                return Some(action);
            }

            // 2. Seconde touche d'une séquence déjà armée.
            if let Some(first) = self.leader_armed.take() {
                let leader_match = bindings
                    .map
                    .iter()
                    .find_map(|(&action, chord)| (*chord == Chord::Leader(first, key)).then_some(action));
                if let Some(action) = leader_match {
                    return Some(action);
                }
                // Pas de correspondance : cette touche redevient
                // éligible à armer une *nouvelle* séquence ci-dessous.
            }

            // 3. Cette touche peut-elle armer une séquence ?
            if !editor_focused {
                let arms_a_leader = bindings
                    .map
                    .values()
                    .any(|c| matches!(c, Chord::Leader(first, _) if *first == key));
                if arms_a_leader {
                    self.leader_armed = Some(key);
                }
            }
        }

        None
    }

    /// La première touche d'une séquence en attente de la seconde, s'il y
    /// en a une — pour afficher un indicateur visible plutôt qu'un état
    /// muet impossible à distinguer d'un raccourci qui ne fait rien.
    pub fn leader_armed(&self) -> Option<Key> {
        self.leader_armed
    }

    /// À appeler une fois par frame pendant qu'une capture (`Rebinding`)
    /// est en cours. Renvoie le nouveau `Chord` dès qu'il est complet.
    pub fn poll_rebind(&mut self, ctx: &egui::Context, rebinding: &mut Rebinding) -> Option<Chord> {
        let pressed = pressed_keys_this_frame(ctx);
        let &(key, modifiers) = pressed.first()?;

        match rebinding.mode {
            RebindMode::Simple => Some(Chord::Simple(KeyboardShortcut::new(modifiers, key))),
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

/// Chaque touche pressée cette frame, avec les modificateurs *attachés à
/// cet évènement précis* — pas l'agrégat `InputState::modifiers`, qui ne
/// se met à jour que sur `Event::ModifiersChanged` et peut donc être en
/// retard d'une frame par rapport à l'`Event::Key` en cours de traitement.
fn pressed_keys_this_frame(ctx: &egui::Context) -> Vec<(Key, Modifiers)> {
    ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, RawInput};

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn raw_input(events: Vec<Event>) -> RawInput {
        RawInput {
            events,
            ..Default::default()
        }
    }

    /// `Context::run_ui` renvoie un `FullOutput` dont `textures_delta` doit
    /// être explicitement vidé, sinon egui panique volontairement à sa
    /// destruction (garde-fou pensé pour les intégrations réelles, qui
    /// n'a pas de sens pour ces tests headless).
    fn run(ctx: &egui::Context, events: Vec<Event>, mut logic: impl FnMut(&egui::Ui)) {
        let mut output = ctx.run_ui(raw_input(events), |ui| logic(ui));
        output.textures_delta.clear();
    }

    #[test]
    fn leader_sequence_resolves_even_when_both_keys_land_in_one_frame() {
        // Régression : si l'appli n'a pas repeint entre les deux frappes,
        // Espace et E peuvent arriver dans le même lot d'évènements d'une
        // seule frame plutôt que sur deux frames séparées.
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();
        let mut result = None;

        run(&ctx, vec![key_event(Key::Space), key_event(Key::E)], |ui| {
            result = dispatcher.poll(ui.ctx(), &bindings, false);
        });

        assert_eq!(result, Some(Action::ToggleExplorer));
    }

    #[test]
    fn leader_sequence_resolves_across_two_separate_frames() {
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();

        run(&ctx, vec![key_event(Key::Space)], |ui| {
            assert_eq!(dispatcher.poll(ui.ctx(), &bindings, false), None);
        });
        assert_eq!(dispatcher.leader_armed(), Some(Key::Space));

        let mut result = None;
        run(&ctx, vec![key_event(Key::E)], |ui| {
            result = dispatcher.poll(ui.ctx(), &bindings, false);
        });

        assert_eq!(result, Some(Action::ToggleExplorer));
    }

    #[test]
    fn leader_does_not_arm_while_editor_focused() {
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();
        let mut result = None;

        run(&ctx, vec![key_event(Key::Space), key_event(Key::E)], |ui| {
            result = dispatcher.poll(ui.ctx(), &bindings, true);
        });

        assert_eq!(result, None);
    }

    #[test]
    fn simple_shortcut_requires_exact_modifiers() {
        let bindings = KeyBindings::default();

        // Maj+F5 (Stop) ne doit pas se faire happer par Run (F5 nu).
        let ctx = egui::Context::default();
        let mut dispatcher = Dispatcher::default();
        let mut result = None;
        run(
            &ctx,
            vec![Event::Key {
                key: Key::F5,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::SHIFT,
            }],
            |ui| {
                result = dispatcher.poll(ui.ctx(), &bindings, false);
            },
        );
        assert_eq!(result, Some(Action::Stop));

        // F5 nu doit toujours déclencher Run — un contexte/dispatcher
        // frais, pour ne pas mélanger cette assertion avec l'état
        // (répétitions détectées, etc.) de la précédente.
        let ctx = egui::Context::default();
        let mut dispatcher = Dispatcher::default();
        let mut result = None;
        run(&ctx, vec![key_event(Key::F5)], |ui| {
            result = dispatcher.poll(ui.ctx(), &bindings, false);
        });
        assert_eq!(result, Some(Action::Run));
    }
}
