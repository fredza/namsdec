//! Raccourcis clavier configurables. Deux formes de "chord" : un raccourci
//! simple (modificateurs + touche, ex. Ctrl+S) et une *séquence* de
//! touches frappées l'une après l'autre à la Vim (ex. Espace puis S puis
//! I) — la seule façon de proposer un raccourci sur une touche aussi
//! utilisée qu'Espace sans la voler au texte tapé dans l'éditeur.
//!
//! Les séquences ignorent les modificateurs : `Espace s i` et
//! `Espace S I` sont le même raccourci. C'est ce qu'on attend d'une
//! séquence de commandes — Maj n'y sert qu'à changer la casse d'une
//! lettre qui n'est pas du texte.

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
    OpenFile,
    OpenFolder,
}

impl Action {
    pub const ALL: [Action; 10] = [
        Action::ToggleExplorer,
        Action::Assemble,
        Action::Run,
        Action::StepInto,
        Action::StepOver,
        Action::Stop,
        Action::Reset,
        Action::Save,
        Action::OpenFile,
        Action::OpenFolder,
    ];
}

/// Longueur maximale d'une séquence. Au-delà de trois touches, un
/// raccourci n'en est plus un : il faut le mémoriser au lieu de le taper.
pub const MAX_SEQUENCE: usize = 3;

/// Une suite de touches frappées l'une après l'autre (`Espace`, `s`, `i`).
///
/// Tableau de taille fixe plutôt qu'un `Vec` : `Chord` reste `Copy`, donc
/// comparable et stockable sans cérémonie dans la table des raccourcis.
#[derive(Clone, Copy, Debug, Eq)]
pub struct Sequence {
    keys: [Key; MAX_SEQUENCE],
    len: usize,
}

impl Sequence {
    /// `None` si la séquence est vide ou trop longue — deux cas qu'on ne
    /// peut ni taper ni afficher.
    pub fn new(keys: &[Key]) -> Option<Self> {
        if keys.is_empty() || keys.len() > MAX_SEQUENCE {
            return None;
        }
        // La case de remplissage n'est jamais lue : `as_slice` s'arrête à
        // `len`, et c'est `as_slice` qui définit l'égalité.
        let mut buffer = [keys[0]; MAX_SEQUENCE];
        buffer[..keys.len()].copy_from_slice(keys);
        Some(Self {
            keys: buffer,
            len: keys.len(),
        })
    }

    pub fn as_slice(&self) -> &[Key] {
        &self.keys[..self.len]
    }

    /// La séquence commence-t-elle par ces touches ? C'est ce qui permet
    /// de rester en attente après `Espace s` au lieu d'abandonner.
    fn starts_with(&self, prefix: &[Key]) -> bool {
        self.as_slice().starts_with(prefix)
    }

    /// La première touche, celle qui peut armer la séquence.
    pub fn first(&self) -> Key {
        self.keys[0]
    }
}

/// L'égalité porte sur les touches utiles, pas sur le remplissage du
/// tableau : sans ça, deux séquences identiques de deux touches pourraient
/// se retrouver différentes.
impl PartialEq for Sequence {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    /// Modificateurs + une touche, tous appuyés ensemble.
    Simple(KeyboardShortcut),
    /// Des touches relâchées l'une après l'autre — style touche leader de
    /// Vim/Neovim. Armée seulement quand l'éditeur de code n'a pas le
    /// focus (sinon "Espace" taperait un espace *et* armerait la séquence
    /// à chaque mot).
    Leader(Sequence),
}

impl Chord {
    /// Construit une séquence, pour les cas où l'on sait qu'elle est
    /// valide (les défauts, les tests).
    pub fn leader(keys: &[Key]) -> Self {
        Chord::Leader(Sequence::new(keys).expect("séquence valide"))
    }

    pub fn display(&self, ctx: &egui::Context) -> String {
        match self {
            Chord::Simple(sc) => ctx.format_shortcut(sc),
            Chord::Leader(sequence) => sequence
                .as_slice()
                .iter()
                .map(|k| k.name())
                .collect::<Vec<_>>()
                .join(" , "),
        }
    }
}

#[derive(Clone)]
pub struct KeyBindings {
    pub map: BTreeMap<Action, Chord>,
}

impl Default for KeyBindings {
    /// Les commandes d'exécution reprennent celles de gdb/pwndbg, derrière
    /// la touche leader : `Espace r` pour lancer, `Espace s i` pour entrer
    /// dans l'instruction (`stepi`), `Espace n i` pour l'enjamber
    /// (`nexti`). Quelqu'un qui apprend l'assembleur passe sa vie dans un
    /// débogueur : autant que les mêmes lettres y fassent la même chose,
    /// plutôt que d'inventer des raccourcis propres à namsdec.
    ///
    /// Le reste suit les conventions de VS Code dans la mesure où elles
    /// survivent au bureau Linux : Maj+F5/Ctrl+Maj+F5 pour Stop/Reset et
    /// Ctrl+Maj+B pour Assembler ("Run Build Task"). F10/F11 nus sont trop
    /// souvent interceptés avant même d'atteindre l'appli (F11 = plein
    /// écran quasi universel au niveau du gestionnaire de fenêtres,
    /// F10 = mnémonique "activer le menu" côté GTK). `Quitter` n'a
    /// volontairement aucun raccourci par défaut : une frappe malheureuse
    /// qui ferme l'appli est pire qu'un clic de menu en plus.
    fn default() -> Self {
        let mut map = BTreeMap::new();
        map.insert(
            Action::Assemble,
            Chord::Simple(KeyboardShortcut::new(
                Modifiers::CTRL | Modifiers::SHIFT,
                Key::B,
            )),
        );
        map.insert(Action::Run, Chord::leader(&[Key::Space, Key::R]));
        map.insert(
            Action::StepOver,
            Chord::leader(&[Key::Space, Key::N, Key::I]),
        );
        map.insert(
            Action::StepInto,
            Chord::leader(&[Key::Space, Key::S, Key::I]),
        );
        map.insert(
            Action::Stop,
            Chord::Simple(KeyboardShortcut::new(Modifiers::SHIFT, Key::F5)),
        );
        map.insert(
            Action::Reset,
            Chord::Simple(KeyboardShortcut::new(
                Modifiers::CTRL | Modifiers::SHIFT,
                Key::F5,
            )),
        );
        map.insert(
            Action::Save,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL, Key::S)),
        );
        // Ouvrir un fichier sans passer par l'explorateur — Ctrl+O comme
        // partout ailleurs ; Ctrl+Maj+O pour un dossier, qui lui alimente
        // l'explorateur.
        map.insert(
            Action::OpenFile,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL, Key::O)),
        );
        map.insert(
            Action::OpenFolder,
            Chord::Simple(KeyboardShortcut::new(
                Modifiers::CTRL | Modifiers::SHIFT,
                Key::O,
            )),
        );
        map.insert(Action::ToggleExplorer, Chord::leader(&[Key::Space, Key::E]));
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
    /// Mode Leader seulement : les touches déjà capturées, en attente de
    /// la suivante ou de la validation.
    captured: Vec<Key>,
}

impl Rebinding {
    pub fn new(action: Action, mode: RebindMode) -> Self {
        Self {
            action,
            mode,
            captured: Vec::new(),
        }
    }

    /// Ce qui a déjà été tapé, pour le montrer pendant la capture — une
    /// séquence de longueur libre se tape sans savoir où elle s'arrête si
    /// rien ne s'affiche.
    pub fn captured(&self) -> &[Key] {
        &self.captured
    }
}

/// État d'exécution du dispatch de raccourcis — distinct de `KeyBindings`
/// (pure configuration) parce qu'il porte la petite machine à états de la
/// touche leader entre deux frames.
#[derive(Default)]
pub struct Dispatcher {
    /// Les touches de séquence déjà frappées, en attente de la suite.
    /// Vide quand rien n'est armé.
    pending: Vec<Key>,
    /// Les touches que le dispatcher a prises pour lui pendant la frame
    /// en cours, et que le reste de l'interface ne doit donc plus
    /// interpréter.
    consumed: Vec<Key>,
}

impl Dispatcher {
    /// À appeler une fois par frame, avant de construire les panneaux.
    /// `text_focused` conditionne l'armement de la touche leader (voir la
    /// note sur `Chord::Leader`) : dès qu'un champ capte la frappe —
    /// l'éditeur, mais aussi le filtre de l'explorateur ou la palette —
    /// Espace doit rester un simple espace.
    pub fn poll(
        &mut self,
        ctx: &egui::Context,
        bindings: &KeyBindings,
        text_focused: bool,
    ) -> Option<Action> {
        let pressed = pressed_keys_this_frame(ctx);
        self.consumed.clear();
        // Retenu pour la trace de diagnostic, plus bas.
        let seen: Vec<Key> = pressed.iter().map(|(k, _)| *k).collect();

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
            let simple_match = bindings
                .map
                .iter()
                .find_map(|(&action, chord)| match chord {
                    Chord::Simple(sc)
                        if sc.logical_key == key && modifiers.matches_exact(sc.modifiers) =>
                    {
                        Some(action)
                    }
                    _ => None,
                });
            if let Some(action) = simple_match {
                self.pending.clear();
                self.take(ctx, key);
                trace(
                    &seen,
                    &self.pending,
                    &self.consumed,
                    Some(action),
                    text_focused,
                );
                return Some(action);
            }

            // 2. Suite d'une séquence en cours. La touche vient allonger
            // ce qui a déjà été tapé : `Espace` puis `s` puis `i`.
            self.pending.push(key);
            let completed = bindings.map.iter().find_map(|(&action, chord)| {
                matches!(chord, Chord::Leader(sequence) if sequence.as_slice() == self.pending)
                    .then_some(action)
            });
            if let Some(action) = completed {
                self.pending.clear();
                self.take(ctx, key);
                trace(
                    &seen,
                    &self.pending,
                    &self.consumed,
                    Some(action),
                    text_focused,
                );
                return Some(action);
            }

            // 3. Sinon : reste-t-il une séquence qui commence par ce
            // qu'on a tapé ? Si oui on attend la suite, sinon on
            // abandonne — mais sans jeter la touche, qui peut très bien
            // être le début d'une *autre* séquence.
            let still_possible = !text_focused
                && bindings
                    .map
                    .values()
                    .any(|c| matches!(c, Chord::Leader(s) if s.starts_with(&self.pending)));
            if still_possible {
                self.take(ctx, key);
            } else {
                self.pending.clear();
                let arms_a_leader = !text_focused
                    && bindings
                        .map
                        .values()
                        .any(|c| matches!(c, Chord::Leader(s) if s.first() == key));
                if arms_a_leader {
                    self.pending.push(key);
                    self.take(ctx, key);
                }
            }
        }

        trace(&seen, &self.pending, &self.consumed, None, text_focused);
        None
    }

    /// Prend une touche pour le dispatcher : elle est notée comme
    /// consommée **et retirée de l'entrée d'egui**.
    ///
    /// Le retrait est la vraie garantie. Marquer la touche ne protège que
    /// le code qui pense à consulter `consumed()` — et seulement tant
    /// qu'il le fait dans la même passe, avec la même liste. La retirer
    /// des évènements la fait disparaître pour tout le monde, quel que
    /// soit l'ordre dans lequel les widgets lisent l'entrée : la touche a
    /// servi à un raccourci, elle n'existe plus pour le reste de
    /// l'interface.
    fn take(&mut self, ctx: &egui::Context, key: Key) {
        self.consumed.push(key);
        ctx.input_mut(|input| {
            input.events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key { key: k, pressed: true, .. } if *k == key
                )
            });
        });
    }

    /// Tout ce qui a été tapé de la séquence en cours, pour l'afficher —
    /// un état muet serait impossible à distinguer d'un raccourci qui ne
    /// fait rien. Après `Espace s`, l'indicateur doit montrer les deux
    /// touches, pas seulement `Espace`.
    pub fn pending(&self) -> &[Key] {
        &self.pending
    }

    /// Les touches que le dispatcher s'est attribuées pendant cette frame.
    ///
    /// Une touche qui fait partie d'un raccourci a déjà servi : le reste
    /// de l'interface doit l'ignorer. Sans ça, le `i` final de
    /// `Espace s i` déclenchait Step Into *et* faisait basculer l'éditeur
    /// en mode Insertion, puisque `i` y est aussi la touche qui ouvre la
    /// saisie.
    pub fn consumed(&self) -> &[Key] {
        &self.consumed
    }

    /// À appeler une fois par frame pendant qu'une capture (`Rebinding`)
    /// est en cours. Renvoie le nouveau `Chord` dès qu'il est complet.
    pub fn poll_rebind(&mut self, ctx: &egui::Context, rebinding: &mut Rebinding) -> Option<Chord> {
        let pressed = pressed_keys_this_frame(ctx);
        let &(key, modifiers) = pressed.first()?;

        match rebinding.mode {
            RebindMode::Simple => Some(Chord::Simple(KeyboardShortcut::new(modifiers, key))),
            // La séquence n'a pas de longueur fixe : on accumule jusqu'à
            // `Entrée`, qui valide, ou jusqu'à la longueur maximale, qui
            // valide d'elle-même puisqu'on ne peut plus rien ajouter.
            RebindMode::Leader => {
                if key == Key::Enter {
                    return Sequence::new(&rebinding.captured).map(Chord::Leader);
                }
                rebinding.captured.push(key);
                if rebinding.captured.len() == MAX_SEQUENCE {
                    return Sequence::new(&rebinding.captured).map(Chord::Leader);
                }
                None
            }
        }
    }
}

/// Trace de diagnostic des raccourcis, activée par `NAMSDEC_TRACE_KEYS=1`.
///
/// Un raccourci qui se comporte mal ne laisse rien derrière lui : on ne
/// peut ni le rejouer, ni deviner après coup ce que le dispatcher avait
/// en main au moment de la frappe. Cette trace dit, frame par frame, ce
/// qu'il a vu et ce qu'il en a fait.
fn trace(
    seen: &[Key],
    pending: &[Key],
    consumed: &[Key],
    action: Option<Action>,
    text_focused: bool,
) {
    if seen.is_empty() {
        return;
    }
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !ENABLED.get_or_init(|| std::env::var_os("NAMSDEC_TRACE_KEYS").is_some()) {
        return;
    }
    let names = |keys: &[Key]| keys.iter().map(|k| k.name()).collect::<Vec<_>>().join("+");
    eprintln!(
        "namsdec: touches [{}] · en attente [{}] · consommées [{}] · action {:?} · champ actif {}",
        names(seen),
        names(pending),
        names(consumed),
        action,
        text_focused
    );
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

    /// Une frappe complète : appui **et** relâchement.
    ///
    /// Sans le relâchement, egui voit la même touche pressée deux frames
    /// de suite sans interruption et la marque comme une répétition —
    /// que le dispatcher ignore délibérément. C'est un artefact de test :
    /// un vrai clavier relâche toujours.
    fn key_stroke(key: Key) -> Vec<Event> {
        vec![
            key_event(key),
            Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ]
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
        assert_eq!(dispatcher.pending(), [Key::Space]);

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

        // Ctrl+Maj+O (Ouvrir un dossier) ne doit pas se faire happer par
        // Ctrl+O (Ouvrir un fichier), dont il ne diffère que par Maj.
        let ctx = egui::Context::default();
        let mut dispatcher = Dispatcher::default();
        let mut result = None;
        run(
            &ctx,
            vec![Event::Key {
                key: Key::O,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL | Modifiers::SHIFT,
            }],
            |ui| {
                result = dispatcher.poll(ui.ctx(), &bindings, false);
            },
        );
        assert_eq!(result, Some(Action::OpenFolder));

        // Ctrl+O seul ouvre bien un fichier — un contexte et un
        // dispatcher frais, pour ne pas mélanger cette assertion avec
        // l'état de la précédente.
        let ctx = egui::Context::default();
        let mut dispatcher = Dispatcher::default();
        let mut result = None;
        run(
            &ctx,
            vec![Event::Key {
                key: Key::O,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            }],
            |ui| {
                result = dispatcher.poll(ui.ctx(), &bindings, false);
            },
        );
        assert_eq!(result, Some(Action::OpenFile));
    }

    /// Les commandes d'exécution de gdb/pwndbg, en trois touches.
    #[test]
    fn a_three_key_sequence_resolves() {
        for (keys, expected) in [
            (vec![Key::Space, Key::S, Key::I], Action::StepInto),
            (vec![Key::Space, Key::N, Key::I], Action::StepOver),
            (vec![Key::Space, Key::R], Action::Run),
        ] {
            let ctx = egui::Context::default();
            let bindings = KeyBindings::default();
            let mut dispatcher = Dispatcher::default();
            let mut result = None;

            run(&ctx, keys.iter().copied().map(key_event).collect(), |ui| {
                result = dispatcher.poll(ui.ctx(), &bindings, false);
            });

            assert_eq!(result, Some(expected), "pour {keys:?}");
        }
    }

    /// Les deux premières touches de `Espace s i` ne déclenchent rien
    /// toutes seules, et l'indicateur montre où l'on en est.
    #[test]
    fn a_sequence_waits_for_all_its_keys() {
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();

        run(&ctx, vec![key_event(Key::Space)], |ui| {
            assert_eq!(dispatcher.poll(ui.ctx(), &bindings, false), None);
        });
        run(&ctx, vec![key_event(Key::S)], |ui| {
            assert_eq!(dispatcher.poll(ui.ctx(), &bindings, false), None);
        });
        assert_eq!(dispatcher.pending(), [Key::Space, Key::S]);

        let mut result = None;
        run(&ctx, vec![key_event(Key::I)], |ui| {
            result = dispatcher.poll(ui.ctx(), &bindings, false);
        });
        assert_eq!(result, Some(Action::StepInto));
        assert!(dispatcher.pending().is_empty(), "la séquence est consommée");
    }

    /// La casse n'a pas d'importance : `Espace S I` vaut `Espace s i`.
    /// egui rend la même `Key` dans les deux cas, Maj n'étant qu'un
    /// modificateur — encore faut-il ne pas l'exiger.
    #[test]
    fn sequences_ignore_the_shift_key() {
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();
        let mut result = None;

        let shifted = |key: Key| Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::SHIFT,
        };
        run(
            &ctx,
            vec![key_event(Key::Space), shifted(Key::S), shifted(Key::I)],
            |ui| {
                result = dispatcher.poll(ui.ctx(), &bindings, false);
            },
        );

        assert_eq!(result, Some(Action::StepInto));
    }

    /// Une séquence abandonnée en cours de route ne doit pas bloquer la
    /// suivante : la touche qui l'a interrompue peut elle-même en armer
    /// une nouvelle.
    #[test]
    fn an_abandoned_sequence_does_not_swallow_the_next_one() {
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();

        // `Espace` puis `z` : aucune séquence ne commence ainsi.
        let mut events = key_stroke(Key::Space);
        events.extend(key_stroke(Key::Z));
        run(&ctx, events, |ui| {
            assert_eq!(dispatcher.poll(ui.ctx(), &bindings, false), None);
        });
        assert!(dispatcher.pending().is_empty());

        let mut events = key_stroke(Key::Space);
        events.extend(key_stroke(Key::R));
        let mut result = None;
        run(&ctx, events, |ui| {
            result = dispatcher.poll(ui.ctx(), &bindings, false);
        });
        assert_eq!(result, Some(Action::Run));
    }

    /// Une séquence de trois touches se relit comme elle s'écrit, et une
    /// séquence vide ou trop longue n'existe pas.
    #[test]
    fn a_sequence_holds_between_one_and_three_keys() {
        let sequence = Sequence::new(&[Key::Space, Key::S, Key::I]).unwrap();
        assert_eq!(sequence.as_slice(), [Key::Space, Key::S, Key::I]);
        assert_eq!(sequence.first(), Key::Space);

        assert!(Sequence::new(&[]).is_none());
        assert!(Sequence::new(&[Key::A, Key::B, Key::C, Key::D]).is_none());

        // L'égalité ne regarde que les touches utiles, pas le remplissage
        // du tableau.
        assert_eq!(
            Sequence::new(&[Key::Space, Key::E]),
            Sequence::new(&[Key::Space, Key::E])
        );
        assert_ne!(
            Sequence::new(&[Key::Space, Key::E]),
            Sequence::new(&[Key::Space, Key::E, Key::I])
        );
    }
}
