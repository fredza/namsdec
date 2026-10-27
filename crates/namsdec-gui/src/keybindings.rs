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
    /// Défaire le dernier pas — ce qu'un émulateur peut offrir et qu'un
    /// débogueur ordinaire rend laborieux.
    StepBack,
    Stop,
    Reset,
    Save,
    OpenFile,
    OpenFolder,
    /// Ouvrir la fiche de l'instruction sous le curseur.
    InstructionHelp,
    /// Exécuter jusqu'à la ligne où se trouve le curseur.
    RunToCursor,
    /// Courir jusqu'au prochain appel, retour, ou appel système.
    ///
    /// Les trois questions qu'on se pose en explorant un programme qu'on
    /// ne connaît pas : que va-t-il appeler, quand sort-il d'ici, et que
    /// demande-t-il au noyau ? pwndbg les appelle `nextcall`, `nextret`
    /// et `nextsyscall` ; ce sont les mêmes.
    RunToCall,
    RunToReturn,
    RunToSyscall,
    /// Aller à l'étiquette sous le curseur — là où elle est définie.
    GoToLabel,
    /// Chercher dans le code.
    Find,
    /// Repartir d'un programme neuf.
    New,
    /// Ouvrir le déroulement du programme : son graphe de blocs.
    ShowFlow,
    /// Passer d'un fichier ouvert au suivant, au précédent, ou fermer
    /// celui qu'on regarde.
    NextTab,
    PreviousTab,
    CloseTab,
    /// Surligner toutes les occurrences de ce qui est sous le curseur.
    ///
    /// Le `*` de vim, en plus visible : au lieu de sauter à la suivante,
    /// on les montre toutes d'un coup. C'est la question qu'on se pose en
    /// lisant — « où d'autre ce registre est-il touché ? » — et qu'on
    /// résout aujourd'hui avec les yeux.
    HighlightMatches,
    /// Donner un nom à l'adresse de la ligne courante.
    ///
    /// Le geste central de la rétro-ingénierie : on passe une soirée à
    /// comprendre que `0x1d491` vérifie le mot de passe, et sans cela il
    /// n'y a plus que `0x1d491` le lendemain.
    NameAddress,
    /// Écrire un commentaire sur la ligne courante.
    CommentAddress,
    /// Aller à l'occurrence suivante de ce qui est suivi, et la
    /// sélectionner.
    ///
    /// Le `n` de vim après un `*`, et le geste qui manque le plus quand
    /// on ne peut pas poser plusieurs curseurs : on ne les a pas tous en
    /// même temps, mais on passe de l'un à l'autre sans quitter le
    /// clavier, et chacun est sélectionné donc prêt à être remplacé.
    NextMatch,
    /// La précédente.
    PreviousMatch,
    /// Renommer d'un coup toutes les occurrences surlignées.
    ///
    /// Ce que le multicurseur sert à faire neuf fois sur dix : changer un
    /// registre partout, renommer une étiquette. egui ne tient qu'un seul
    /// curseur — on ne peut donc pas en poser plusieurs —, mais on peut
    /// faire l'édition elle-même, et c'est elle qu'on voulait.
    RenameMatches,
    /// Recopier la ligne où l'on est, juste en dessous.
    ///
    /// Le geste qu'on fait sans y penser dans tout éditeur, et qui manque
    /// cruellement en assembleur : quatre lignes qui ne diffèrent que
    /// par un registre se dupliquent plus vite qu'elles ne se retapent.
    DuplicateLine,
    /// Ouvrir ou refermer la calculatrice.
    ///
    /// La seule fenêtre qu'on ouvre au milieu d'une autre tâche : on lit
    /// `and eax, 0xff`, on veut savoir ce que ça garde, on revient. Un
    /// aller-retour par le menu Affichage casse justement ce qu'on était
    /// en train de suivre — et c'est pour cela qu'elle bascule au lieu
    /// de seulement s'ouvrir : la même frappe la renvoie d'où elle vient.
    ToggleCalculator,
    /// Ouvrir ou refermer la palette de commandes.
    ///
    /// Elle a toujours eu son raccourci — Ctrl+Maj+P —, mais posé en dur
    /// au milieu de la boucle d'affichage : absent des deux jeux, absent
    /// du tableau de Préférences, absent du fichier exporté. Le seul
    /// raccourci de l'application qu'on ne pouvait pas déplacer était
    /// aussi celui qui sert à trouver tous les autres.
    ///
    /// Une BASCULE, comme la calculatrice : la même frappe la renvoie
    /// d'où elle vient, sans viser sa croix.
    ToggleCommandPalette,
    /// Ouvrir les Préférences.
    ///
    /// Le dernier geste courant sans touche, et le plus ironique :
    /// c'est la fenêtre où l'on règle les raccourcis. `Ctrl+,` est la
    /// convention, de VS Code aux navigateurs.
    ShowPreferences,
    /// Montrer ou cacher la console, et le panneau latéral.
    ///
    /// DEUX DES QUATRE BASCULES D'AFFICHAGE, et c'est un choix. La barre
    /// d'outils et la barre de statut se règlent une fois et ne bougent
    /// plus : leur donner une frappe, ce serait retirer cette frappe à
    /// quelque chose qu'on fait tous les jours. Ces deux-là, au
    /// contraire, s'ouvrent et se referment pendant qu'on travaille —
    /// on cache le panneau pour lire un long listing, on montre la
    /// console pour voir ce que le programme a écrit.
    ToggleConsole,
    ToggleSidebar,
}

impl Action {
    pub const ALL: [Action; 35] = [
        Action::ToggleExplorer,
        Action::Assemble,
        Action::Run,
        Action::StepInto,
        Action::StepOver,
        Action::StepBack,
        Action::Stop,
        Action::Reset,
        Action::Save,
        Action::OpenFile,
        Action::OpenFolder,
        Action::InstructionHelp,
        Action::RunToCursor,
        Action::RunToCall,
        Action::RunToReturn,
        Action::RunToSyscall,
        Action::GoToLabel,
        Action::Find,
        Action::New,
        Action::ShowFlow,
        Action::NextTab,
        Action::PreviousTab,
        Action::CloseTab,
        Action::DuplicateLine,
        Action::HighlightMatches,
        Action::RenameMatches,
        Action::NextMatch,
        Action::PreviousMatch,
        Action::NameAddress,
        Action::CommentAddress,
        Action::ToggleCalculator,
        Action::ToggleCommandPalette,
        Action::ShowPreferences,
        Action::ToggleConsole,
        Action::ToggleSidebar,
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

    pub fn display(&self, ctx: &egui::Context, fr: bool) -> String {
        match self {
            // Notre propre mise en forme plutôt que `format_shortcut` :
            // celle d'egui écrit « Shift », et le manuel écrit « Maj »
            // depuis la première page.
            Chord::Simple(sc) => {
                let noms = if fr {
                    MODIFICATEURS_FR
                } else {
                    MODIFICATEURS_EN
                };
                let mut texte = noms.format(&sc.modifiers, ctx.os().is_mac());
                if !texte.is_empty() {
                    texte += noms.concat;
                }
                texte += key_name(sc.logical_key, fr);
                texte
            }
            // Séparées par une espace, et non par « , ».
            //
            // Le tableau des deux jeux a mis les deux formes côte à
            // côte, trente lignes de suite : `Space , P , I` en regard
            // de `Ctrl+Shift+B`. La virgule entre deux espaces n'ajoute
            // rien et se lit comme un signe de plus à démêler. Le projet
            // écrit d'ailleurs « Espace p i » partout dans sa prose, et
            // c'est cette forme-là qu'on retrouve maintenant à l'écran.
            Chord::Leader(sequence) => keys_display(sequence.as_slice(), fr),
        }
    }
}

/// Les modificateurs comme le manuel les écrit : « Maj » et non
/// « Shift ».
///
/// egui n'a qu'un jeu de noms, en anglais. Les deux notations se sont
/// croisées pendant longtemps — le guide imprimé disait « Ctrl+Maj+B »,
/// l'écran disait « Ctrl+Shift+B », et c'était la même frappe. Une
/// notation de trop est une notation qu'il faut traduire de tête au
/// moment où l'on cherche son raccourci.
const MODIFICATEURS_FR: &egui::ModifierNames<'static> = &egui::ModifierNames {
    is_short: false,
    alt: "Alt",
    ctrl: "Ctrl",
    shift: "Maj",
    mac_cmd: "Cmd",
    mac_alt: "Option",
    concat: "+",
};

/// Ceux d'egui, inchangés : en anglais, « Shift » est le bon mot.
const MODIFICATEURS_EN: &egui::ModifierNames<'static> = &egui::ModifierNames::NAMES;

/// Le nom d'une touche, tel qu'on le dit — et tel que le manuel l'écrit.
///
/// `Key::name()` rend des noms d'identificateurs : « Space », « Slash »,
/// « Escape ». Le générateur du guide les traduit depuis toujours
/// (`docs/guide.py::_touche`) parce que « Slash » dans une phrase ne dit
/// pas sur quoi appuyer. L'écran, lui, affichait le nom brut : on lisait
/// « Espace puis / » dans le PDF et « Space Slash » dans l'application.
///
/// Les deux tables sont maintenant la même, et c'est tout ce qu'on
/// demande à une notation : qu'il n'y en ait qu'une.
pub fn key_name(key: Key, fr: bool) -> &'static str {
    // La ponctuation d'abord, dans les deux langues : `symbol_or_name`
    // rend « / », « ; », « [ » — ce qui est gravé sur la touche, donc
    // ce qui se cherche des yeux.
    match key {
        Key::Space => {
            if fr {
                "Espace"
            } else {
                "Space"
            }
        }
        Key::Escape => {
            if fr {
                "Échap"
            } else {
                "Escape"
            }
        }
        Key::Enter => {
            if fr {
                "Entrée"
            } else {
                "Enter"
            }
        }
        Key::Backspace => {
            if fr {
                "Retour arr."
            } else {
                "Backspace"
            }
        }
        Key::Delete => {
            if fr {
                "Suppr"
            } else {
                "Delete"
            }
        }
        Key::Insert => {
            if fr {
                "Inser"
            } else {
                "Insert"
            }
        }
        Key::Home => {
            if fr {
                "Origine"
            } else {
                "Home"
            }
        }
        Key::End => {
            if fr {
                "Fin"
            } else {
                "End"
            }
        }
        // Le trait d'union, et non le SIGNE MOINS d'egui (U+2212).
        //
        // Relevé par le contrôle qui confronte cette table à celle du
        // manuel, et c'est exactement ce qu'on lui demandait de
        // trouver : egui écrit « − », le manuel écrit « - », et la
        // touche du clavier porte le second. Un signe typographique
        // plus beau qui ne se trouve pas sur le clavier ne rend pas
        // service à qui cherche sur quoi appuyer.
        Key::Minus => "-",
        // Flèches et ponctuation : le symbole plutôt que le mot. Pour
        // tout le reste — lettres, chiffres, touches de fonction —
        // `symbol_or_name` rend déjà le nom, identique dans les deux
        // langues.
        _ => key.symbol_or_name(),
    }
}

/// Une suite de touches, écrite comme on la tape : `Espace s i`, `y y`.
///
/// Les lettres en minuscule, et c'est voulu : une séquence ignore les
/// modificateurs (voir l'en-tête du module), donc une majuscule à
/// l'écran annoncerait un Maj à presser qui ne sert à rien. C'est aussi
/// la forme que le projet écrit dans sa prose depuis le début.
pub fn keys_display(keys: &[Key], fr: bool) -> String {
    keys.iter()
        .map(|k| {
            let nom = key_name(*k, fr);
            if nom.len() == 1 && nom.chars().all(|c| c.is_ascii_uppercase()) {
                nom.to_lowercase()
            } else {
                nom.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone)]
pub struct KeyBindings {
    pub map: BTreeMap<Action, Chord>,
}

impl Default for KeyBindings {
    /// Le jeu de départ : celui de vim et de gdb.
    ///
    /// namsdec s'adresse à qui apprend l'assembleur, et qui passe donc sa
    /// vie dans un éditeur modal et un débogueur. Les deux jeux vivent
    /// dans le binaire ; celui-ci est seulement celui qu'on trouve sans
    /// rien régler.
    fn default() -> Self {
        Self::preset(Preset::Vim)
    }
}

#[cfg(test)]
mod tests_presets {
    use super::*;

    /// Les deux jeux sont dans le binaire, entiers.
    ///
    /// Aucun n'est un fichier à installer : on doit pouvoir passer de
    /// l'un à l'autre sans réseau, sans dossier de configuration, et sans
    /// qu'une action se retrouve sans raccourci au passage.
    #[test]
    fn both_sets_live_in_the_binary_and_cover_the_same_actions() {
        let vim = KeyBindings::preset(Preset::Vim);
        let classic = KeyBindings::preset(Preset::Classic);
        assert!(!vim.map.is_empty() && !classic.map.is_empty());

        let manquantes: Vec<_> = vim
            .map
            .keys()
            .filter(|action| !classic.map.contains_key(action))
            .collect();
        assert!(
            manquantes.is_empty(),
            "ces actions n'ont pas de raccourci dans le jeu ordinaire : {manquantes:?}"
        );
        let en_trop: Vec<_> = classic
            .map
            .keys()
            .filter(|action| !vim.map.contains_key(action))
            .collect();
        assert!(
            en_trop.is_empty(),
            "et celles-ci manquent au jeu vim : {en_trop:?}"
        );
    }

    /// Dans un jeu, deux actions ne partagent pas la même frappe.
    #[test]
    fn no_two_actions_answer_to_the_same_keys() {
        for preset in Preset::ALL {
            let bindings = KeyBindings::preset(preset);
            let mut vus: BTreeMap<String, Action> = BTreeMap::new();
            for (action, chord) in &bindings.map {
                let touche = format!("{chord:?}");
                if let Some(autre) = vus.insert(touche.clone(), *action) {
                    panic!(
                        "{} : {autre:?} et {action:?} répondent tous deux à {touche}",
                        preset.id()
                    );
                }
            }
        }
    }

    /// Le jeu ordinaire n'a aucune séquence à touche leader.
    ///
    /// C'est ce qu'on vient y chercher : une frappe, une action.
    #[test]
    fn the_ordinary_set_has_no_leader_sequences() {
        for (action, chord) in &KeyBindings::preset(Preset::Classic).map {
            assert!(
                matches!(chord, Chord::Simple(_)),
                "{action:?} garde une séquence : {chord:?}"
            );
        }
        // Et le jeu vim, lui, en a bien.
        assert!(
            KeyBindings::preset(Preset::Vim)
                .map
                .values()
                .any(|c| matches!(c, Chord::Leader(_))),
            "le jeu vim doit garder ses séquences"
        );
    }

    /// Le nom d'un jeu survit à l'écriture et à la relecture.
    #[test]
    fn a_set_is_named_the_same_way_coming_and_going() {
        for preset in Preset::ALL {
            assert_eq!(Preset::from_id(preset.id()), Some(preset));
        }
        assert_eq!(Preset::from_id("inconnu"), None);
        assert_eq!(Preset::default(), Preset::Vim, "vim est celui de départ");
    }
}

/// Les deux jeux de raccourcis que namsdec porte.
///
/// Deux, et toujours les deux : l'un pour qui vient de vim, l'autre pour
/// qui vient d'un éditeur ordinaire. Aucun n'est un fichier à installer —
/// ils sont dans le binaire, et le fichier de l'utilisateur ne fait que
/// s'en écarter là où il le veut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preset {
    /// Touche leader, séquences de lettres, commandes de gdb.
    #[default]
    Vim,
    /// Touches de fonction et Ctrl, comme dans la plupart des éditeurs.
    Classic,
}

impl Preset {
    /// Les deux, dans l'ordre où on les propose.
    pub const ALL: [Self; 2] = [Self::Vim, Self::Classic];

    /// Le nom écrit dans le fichier — stable, à ne pas traduire.
    pub fn id(self) -> &'static str {
        match self {
            Self::Vim => "vim",
            Self::Classic => "classic",
        }
    }

    /// Le jeu que ce nom désigne, si on le connaît.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.id() == id)
    }

    /// Comment l'appeler à l'écran.
    ///
    /// « Fake Vim-GDB » et non « Vim et gdb » : le jeu s'inspire des deux
    /// sans les imiter — il n'y a ni `:` ni registres de Vim, ni langage
    /// de commande de gdb. Le nom le dit maintenant plutôt que de le
    /// laisser découvrir, et il ne se traduit pas : c'est un nom propre.
    pub fn label(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Self::Vim, _) => "Fake Vim-GDB",
            (Self::Classic, true) => "Éditeur ordinaire",
            (Self::Classic, false) => "Ordinary editor",
        }
    }

    /// Ce qu'il change, en une phrase.
    pub fn about(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Self::Vim, true) => {
                "Touche leader (Espace) et séquences de lettres ; les commandes \
                 d'exécution reprennent celles de gdb."
            }
            (Self::Vim, false) => {
                "Leader key (Space) and letter sequences; the execution commands \
                 are gdb's."
            }
            (Self::Classic, true) => {
                "Touches de fonction et Ctrl, comme dans la plupart des éditeurs : \
                 F5 lance, F10 enjambe, F11 entre."
            }
            (Self::Classic, false) => {
                "Function keys and Ctrl, as in most editors: F5 runs, F10 steps \
                 over, F11 steps into."
            }
        }
    }
}

impl KeyBindings {
    /// Le jeu demandé, tel qu'il est dans le binaire.
    pub fn preset(preset: Preset) -> Self {
        match preset {
            Preset::Vim => Self::vim(),
            Preset::Classic => Self::classic(),
        }
    }

    /// Le jeu de qui vient d'un éditeur ordinaire.
    ///
    /// Touches de fonction pour l'exécution — F5 lance, F10 enjambe, F11
    /// entre —, comme le font Visual Studio, VS Code, Qt Creator et la
    /// plupart des débogueurs à fenêtres depuis trente ans. Le reste suit
    /// les habitudes d'édition : Ctrl+S, Ctrl+O, Ctrl+F, Ctrl+W.
    ///
    /// Aucune séquence à touche leader ici : c'est précisément ce qu'on
    /// vient chercher en changeant de jeu.
    pub fn classic() -> Self {
        use Modifiers as M;
        let simple = |m: Modifiers, k: Key| Chord::Simple(KeyboardShortcut::new(m, k));
        let mut map = BTreeMap::new();

        // Construire et exécuter.
        map.insert(Action::Assemble, simple(M::CTRL | M::SHIFT, Key::B));
        map.insert(Action::Run, simple(M::NONE, Key::F5));
        map.insert(Action::StepInto, simple(M::NONE, Key::F11));
        map.insert(Action::StepOver, simple(M::NONE, Key::F10));
        // Le pas arrière n'a pas de tradition : Maj+F11 est « sortir de
        // la fonction » ailleurs, ce que namsdec ne fait pas.
        map.insert(Action::StepBack, simple(M::SHIFT, Key::F11));
        map.insert(Action::Stop, simple(M::SHIFT, Key::F5));
        map.insert(Action::Reset, simple(M::CTRL | M::SHIFT, Key::F5));
        map.insert(Action::RunToCursor, simple(M::CTRL, Key::F10));

        // Courir jusqu'au prochain appel, retour ou appel système : sans
        // équivalent ailleurs, on les range ensemble derrière Ctrl+Alt.
        map.insert(Action::RunToCall, simple(M::CTRL | M::ALT, Key::C));
        map.insert(Action::RunToReturn, simple(M::CTRL | M::ALT, Key::R));
        map.insert(Action::RunToSyscall, simple(M::CTRL | M::ALT, Key::S));

        // Les fichiers.
        map.insert(Action::New, simple(M::CTRL, Key::N));
        map.insert(Action::Save, simple(M::CTRL, Key::S));
        map.insert(Action::OpenFile, simple(M::CTRL, Key::O));
        map.insert(Action::OpenFolder, simple(M::CTRL | M::SHIFT, Key::O));
        map.insert(Action::CloseTab, simple(M::CTRL, Key::W));
        // Ctrl+Maj+D : celui de VS Code. Le jeu ordinaire n'a aucune
        // séquence — c'est sa raison d'être.
        map.insert(
            Action::DuplicateLine,
            simple(M::CTRL.plus(M::SHIFT), Key::D),
        );
        // Ctrl+F2 : celui de VS Code pour « toutes les occurrences ».
        // Ctrl+F2 : « sélectionner toutes les occurrences » de VS Code.
        map.insert(Action::HighlightMatches, simple(M::CTRL, Key::F2));
        // F2 tout court : « renommer », partout où ce geste existe.
        map.insert(Action::RenameMatches, simple(M::NONE, Key::F2));
        // F3 / Maj+F3 : « suivant » et « précédent », partout où l'on
        // cherche quelque chose.
        map.insert(Action::NextMatch, simple(M::NONE, Key::F3));
        map.insert(Action::PreviousMatch, simple(M::SHIFT, Key::F3));
        // Ctrl+Maj+N pour nommer, Ctrl+Maj+C pour commenter. Ctrl+C seul
        // reste la copie, qu'on ne touche pas.
        map.insert(Action::NameAddress, simple(M::CTRL.plus(M::SHIFT), Key::N));
        map.insert(
            Action::CommentAddress,
            simple(M::CTRL.plus(M::SHIFT), Key::C),
        );
        map.insert(Action::NextTab, simple(M::CTRL, Key::Tab));
        map.insert(Action::PreviousTab, simple(M::CTRL | M::SHIFT, Key::Tab));

        // Chercher et se déplacer.
        map.insert(Action::Find, simple(M::CTRL, Key::F));
        map.insert(Action::GoToLabel, simple(M::NONE, Key::F12));
        map.insert(Action::InstructionHelp, simple(M::NONE, Key::F1));

        // Les panneaux.
        map.insert(Action::ToggleExplorer, simple(M::CTRL, Key::B));
        map.insert(Action::ShowFlow, simple(M::CTRL | M::SHIFT, Key::G));
        // La calculatrice sur la DEUXIÈME lettre de son nom : Ctrl+C
        // copie, Ctrl+Maj+C commente et Ctrl+Alt+C court jusqu'au
        // prochain appel. Sa première lettre est prise trois fois, on
        // prend la suivante plutôt que d'inventer une touche sans
        // rapport.
        map.insert(Action::ToggleCalculator, simple(M::CTRL | M::SHIFT, Key::A));
        // Ctrl+Maj+P : celui de VS Code, et celui que namsdec posait
        // déjà en dur. Il ne change pas de place en devenant
        // déplaçable — on rend le geste réglable, on ne rééduque pas
        // les doigts.
        map.insert(
            Action::ToggleCommandPalette,
            simple(M::CTRL | M::SHIFT, Key::P),
        );
        // Ctrl+, : la convention des préférences, de VS Code aux
        // navigateurs. La virgule est libre partout ailleurs.
        map.insert(Action::ShowPreferences, simple(M::CTRL, Key::Comma));
        // Ctrl+J : le panneau du bas de VS Code, et c'est la même
        // chose ici — la sortie, les diagnostics, le profil.
        map.insert(Action::ToggleConsole, simple(M::CTRL, Key::J));
        // Ctrl+Alt+B : « l'autre B ». Ctrl+B montre l'explorateur, à
        // gauche ; celui-ci montre le panneau, à droite. VS Code n'a
        // rien à imiter ici — il n'a qu'une barre latérale.
        map.insert(Action::ToggleSidebar, simple(M::CTRL | M::ALT, Key::B));

        Self { map }
    }

    /// Les commandes d'exécution reprennent celles de gdb/pwndbg, derrière
    /// la touche leader : `Espace p r` pour lancer, `Espace s i` pour
    /// entrer dans l'instruction (`stepi`), `Espace n i` pour l'enjamber
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
    pub fn vim() -> Self {
        let mut map = BTreeMap::new();
        map.insert(
            Action::Assemble,
            Chord::Simple(KeyboardShortcut::new(
                Modifiers::CTRL | Modifiers::SHIFT,
                Key::B,
            )),
        );
        // --- `Espace p…` : les pas ---------------------------------
        //
        // Les séquences sont rangées par FAMILLE, et non par la première
        // lettre de l'action. C'est un changement de méthode, et il vient
        // d'un constat : rangées par action, elles se plaçaient où il
        // restait de la place. `Espace s` avait fini par servir aux pas
        // (`s i`) et à la recherche (`s s`), `n` court-circuitait
        // `Espace n i`, et « nommer » avait dû prendre `Espace a` faute
        // de mieux — alors que `n` est la touche de tous les outils du
        // métier.
        //
        // Rangées par famille, le préfixe dit de quoi l'on parle et la
        // lettre dit quoi en faire. Chaque ajout a dès lors une place
        // évidente, au lieu d'une place libre.
        //
        // Le prix est réel et il faut le dire : toutes les habitudes
        // prises sur `Espace r`, `Espace b`, `Espace s i` changent.
        map.insert(Action::Run, Chord::leader(&[Key::Space, Key::P, Key::R]));
        // LES DEUX PAS GARDENT LES LETTRES DE gdb, et c'est un retour en
        // arrière assumé sur le rangement par famille.
        //
        // `stepi` et `nexti` sont ce que les doigts tapent depuis des
        // années dans gdb et pwndbg ; `Espace p i` / `Espace p o` étaient
        // plus cohérents sur le papier et faux sous les doigts. Entre une
        // règle de rangement et une habitude de métier, c'est l'habitude
        // qui gagne : ces deux-là sont les raccourcis les plus frappés de
        // tout le programme.
        //
        // Le reste de la famille `Espace p…` ne bouge pas : il n'a pas
        // d'équivalent à imiter.
        map.insert(
            Action::StepInto,
            Chord::leader(&[Key::Space, Key::S, Key::I]),
        );
        map.insert(
            Action::StepOver,
            Chord::leader(&[Key::Space, Key::N, Key::I]),
        );
        // `p b` comme *back* : gdb écrit `reverse-stepi`, qui n'a pas
        // d'abréviation courte à imiter.
        map.insert(
            Action::StepBack,
            Chord::leader(&[Key::Space, Key::P, Key::B]),
        );
        map.insert(
            Action::RunToCursor,
            Chord::leader(&[Key::Space, Key::P, Key::C]),
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
        // F1 sur le mot où l'on est : c'est la touche d'aide partout
        // ailleurs, et elle ne sert à rien d'autre ici.
        map.insert(
            Action::InstructionHelp,
            Chord::Simple(KeyboardShortcut::new(Modifiers::NONE, Key::F1)),
        );
        // « Exécuter jusqu'ici » : on pose le curseur sur une ligne et
        // l'on y va, sans poser puis retirer un point d'arrêt. Il vit
        // désormais dans la famille des pas (`Espace p c`) : c'en est un,
        // simplement plus long.
        // F12 : « aller à la définition », la touche qu'un éditeur de
        // code donne à ce geste depuis toujours.
        map.insert(
            Action::GoToLabel,
            Chord::Simple(KeyboardShortcut::new(Modifiers::NONE, Key::F12)),
        );
        // Ctrl+N : un programme neuf, comme partout ailleurs.
        map.insert(
            Action::New,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL, Key::N)),
        );
        // Ctrl+Tab et Ctrl+Maj+Tab : d'un fichier ouvert au suivant,
        // comme dans tous les éditeurs — et Ctrl+W ferme celui qu'on
        // regarde.
        map.insert(
            Action::NextTab,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL, Key::Tab)),
        );
        map.insert(
            Action::PreviousTab,
            Chord::Simple(KeyboardShortcut::new(
                Modifiers::CTRL.plus(Modifiers::SHIFT),
                Key::Tab,
            )),
        );
        map.insert(
            Action::CloseTab,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL, Key::W)),
        );
        // `y y` : le doublement de vim, qui copie une ligne. Ici il la
        // recopie directement — c'est le geste, sans le presse-papiers.
        map.insert(Action::DuplicateLine, Chord::leader(&[Key::Y, Key::Y]));
        // --- `Espace /…` : chercher --------------------------------
        //
        // La barre oblique est la touche de recherche de vim : le préfixe
        // se devine donc sans l'apprendre. Le doublement — `/ /` —
        // surligne, comme `y y` duplique.
        map.insert(
            Action::HighlightMatches,
            Chord::leader(&[Key::Space, Key::Slash, Key::Slash]),
        );
        map.insert(
            Action::RenameMatches,
            Chord::leader(&[Key::Space, Key::Slash, Key::R]),
        );
        // --- `Espace m…` : marquer ---------------------------------
        //
        // Ce qu'on écrit sur le binaire pour s'en souvenir. `m n` nomme,
        // `m c` commente — et `n` retrouve enfin sa place, celle qu'il a
        // dans tous les outils du métier. Il avait dû céder à `Espace a`
        // parce que `Espace n` était le préfixe de `Espace n i` ; ce
        // préfixe n'existe plus.
        map.insert(
            Action::NameAddress,
            Chord::leader(&[Key::Space, Key::M, Key::N]),
        );
        map.insert(
            Action::CommentAddress,
            Chord::leader(&[Key::Space, Key::M, Key::C]),
        );
        map.insert(
            Action::NextMatch,
            Chord::leader(&[Key::Space, Key::Slash, Key::N]),
        );
        map.insert(
            Action::PreviousMatch,
            Chord::leader(&[Key::Space, Key::Slash, Key::P]),
        );
        // Ctrl+F : chercher, comme partout ailleurs.
        map.insert(
            Action::Find,
            Chord::Simple(KeyboardShortcut::new(Modifiers::CTRL, Key::F)),
        );
        // --- `Espace v…` : voir ------------------------------------
        //
        // Ce qui ouvre une vue sur ce qu'on a sous la main, sans rien
        // changer au programme.
        map.insert(
            Action::ToggleExplorer,
            Chord::leader(&[Key::Space, Key::V, Key::E]),
        );
        map.insert(
            Action::ShowFlow,
            Chord::leader(&[Key::Space, Key::V, Key::G]),
        );
        // Trois pas d'exploration sous le même préfixe, `u` comme
        // *until* — le mot de gdb pour « cours jusqu'à ». Trois touches
        // et non deux : `Espace r` et `Espace s…` sont déjà pris, et une
        // séquence qui en court-circuite une autre la rend inatteignable.
        map.insert(
            Action::RunToCall,
            Chord::leader(&[Key::Space, Key::U, Key::C]),
        );
        map.insert(
            Action::RunToReturn,
            Chord::leader(&[Key::Space, Key::U, Key::R]),
        );
        map.insert(
            Action::RunToSyscall,
            Chord::leader(&[Key::Space, Key::U, Key::S]),
        );
        // --- `Espace c…` : calculer --------------------------------
        //
        // `c a` comme les deux premières lettres de « calculatrice » —
        // la famille n'en compte qu'une pour l'instant, et c'est
        // délibéré : le préfixe est posé là où le reste de ce qui
        // CALCULE viendra se ranger, plutôt que sur la place libre du
        // moment. C'est la leçon du rangement par famille, appliquée
        // avant d'en avoir besoin.
        map.insert(
            Action::ToggleCalculator,
            Chord::leader(&[Key::Space, Key::C, Key::A]),
        );
        // Ctrl+Maj+P ici aussi, et non une séquence.
        //
        // Le jeu vim garde les raccourcis simples là où ils sont une
        // habitude de métier plus forte que sa propre règle : Ctrl+S,
        // Ctrl+O, Ctrl+F, Ctrl+N, Ctrl+W. La palette est de ceux-là.
        // `Espace Espace` aurait été le geste des configurations
        // Neovim et suivait l'idiome du doublement (`y y`, `/ /`) —
        // mais c'est la frappe que namsdec posait en dur depuis le
        // début, et la changer en la rendant réglable aurait fait payer
        // une amélioration à ceux qui la connaissent déjà. Elle se
        // déplace maintenant d'un clic pour qui préfère l'autre.
        map.insert(
            Action::ToggleCommandPalette,
            Chord::Simple(KeyboardShortcut::new(
                Modifiers::CTRL.plus(Modifiers::SHIFT),
                Key::P,
            )),
        );
        // `Espace v r` : voir les réglages. La famille `v…` est celle
        // de ce qui ouvre une vue sans rien changer au programme, et
        // les Préférences en sont une — on y regarde ce qu'on a réglé
        // au moins aussi souvent qu'on y change quelque chose.
        map.insert(
            Action::ShowPreferences,
            Chord::leader(&[Key::Space, Key::V, Key::R]),
        );
        // La famille `v…` s'étoffe, et c'est exactement ce pour quoi
        // elle a été posée : chaque ajout a une place évidente au lieu
        // d'une place libre. `v c` la console, `v p` le panneau.
        map.insert(
            Action::ToggleConsole,
            Chord::leader(&[Key::Space, Key::V, Key::C]),
        );
        map.insert(
            Action::ToggleSidebar,
            Chord::leader(&[Key::Space, Key::V, Key::P]),
        );
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
        // Les répétitions comptent quand une séquence est en cours, et
        // c'est indispensable : egui marque la SECONDE pression d'une
        // même touche comme une répétition, si bien qu'une séquence de
        // deux touches identiques ne pouvait pas se faire. `y y` ne
        // dupliquait pas, `g g` n'allait nulle part — les deux gestes
        // les plus vim du lot, et les deux muets.
        //
        // Hors séquence, les répétitions restent écartées : garder le
        // doigt sur une touche ne doit pas rejouer un raccourci trente
        // fois par seconde.
        let pressed = pressed_keys_this_frame(ctx, !self.pending.is_empty());
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
            //
            // **Sauf si une séquence est en cours.** Une fois `Espace`
            // tapé, la touche suivante appartient à la séquence : c'est
            // ce que fait tout éditeur modal, et sans cette règle,
            // n'importe quelle lettre servant de deuxième touche
            // devenait interdite comme raccourci simple. `n` court-
            // circuitait ainsi `Espace n i` — Step Over n'était plus
            // atteignable, et l'on ne voyait pas pourquoi.
            let simple_match = if self.pending.is_empty() {
                bindings
                    .map
                    .iter()
                    .find_map(|(&action, chord)| match chord {
                        Chord::Simple(sc)
                            if sc.logical_key == key && modifiers.matches_exact(sc.modifiers) =>
                        {
                            Some(action)
                        }
                        _ => None,
                    })
            } else {
                None
            };
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
            //
            // Sauf si le clavier appartient à un champ de texte : aucune
            // séquence ne s'y arme (étape 3), aucune ne doit donc s'y
            // terminer, et celle qui l'était déjà est abandonnée. Un
            // `Espace` armé dans l'éditeur en mode Normal survivait à un
            // clic dans le filtre de l'explorateur ou la console : le
            // premier `e` tapé là complétait `Espace e`, l'explorateur
            // basculait, et la lettre s'écrivait quand même dans le champ
            // — `take` ne retire que l'évènement Touche, pas le Texte.
            if text_focused {
                self.pending.clear();
                continue;
            }
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
    pub fn poll_rebind(&self, ctx: &egui::Context, rebinding: &mut Rebinding) -> Option<Chord> {
        // En capture de raccourci, une séquence peut aussi doubler une
        // touche : quelqu'un qui pose `y y` doit pouvoir le taper.
        let pressed = pressed_keys_this_frame(ctx, !rebinding.captured.is_empty());
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
fn pressed_keys_this_frame(ctx: &egui::Context, sequence_en_cours: bool) -> Vec<(Key, Modifiers)> {
    ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat,
                    modifiers,
                    ..
                } if !*repeat || sequence_en_cours => Some((*key, *modifiers)),
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

        run(
            &ctx,
            vec![key_event(Key::Space), key_event(Key::V), key_event(Key::E)],
            |ui| {
                result = dispatcher.poll(ui.ctx(), &bindings, false);
            },
        );

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

        // `Espace v e` compte trois touches depuis que les séquences
        // sont rangées par famille : `v` ouvre la famille « voir », `e`
        // désigne l'explorateur.
        run(&ctx, vec![key_event(Key::V)], |ui| {
            assert_eq!(dispatcher.poll(ui.ctx(), &bindings, false), None);
        });
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

        run(
            &ctx,
            vec![key_event(Key::Space), key_event(Key::V), key_event(Key::E)],
            |ui| {
                result = dispatcher.poll(ui.ctx(), &bindings, true);
            },
        );

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

    /// Une séquence armée ne survit pas au clavier qui part dans un
    /// champ de texte.
    ///
    /// `Espace` s'arme dans l'éditeur en mode Normal (rien n'a le focus).
    /// L'utilisateur clique ensuite dans le filtre de l'explorateur ou la
    /// console et tape `e` : sans cette garde, la frappe complétait
    /// `Espace e`, l'explorateur basculait — et la lettre s'écrivait
    /// quand même dans le champ.
    #[test]
    fn an_armed_sequence_is_dropped_when_a_text_field_takes_over() {
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();

        // Frame 1 : Espace, hors de tout champ. Rien ne se déclenche,
        // la séquence est armée.
        let mut armed = None;
        run(&ctx, vec![key_event(Key::Space)], |ui| {
            armed = dispatcher.poll(ui.ctx(), &bindings, false);
        });
        assert_eq!(armed, None);
        assert_eq!(
            dispatcher.pending(),
            [Key::Space],
            "Espace arme la séquence"
        );

        // Frame 2 : le clavier est dans un champ, et on y tape `e`.
        let mut fired = None;
        run(&ctx, vec![key_event(Key::V)], |ui| {
            fired = dispatcher.poll(ui.ctx(), &bindings, true);
        });

        assert_eq!(fired, None, "aucune action ne doit partir");
        assert!(
            dispatcher.pending().is_empty(),
            "et la séquence en attente est abandonnée"
        );
        assert!(
            dispatcher.consumed().is_empty(),
            "la touche appartient au champ, le dispatcher n'en prend rien"
        );
    }

    /// Une séquence peut doubler une touche : `y y`, `g g`.
    ///
    /// Elle ne le pouvait pas, et les deux gestes les plus vim du lot
    /// étaient muets. egui marque la SECONDE pression d'une même touche
    /// comme une répétition — le doigt qui reste appuyé —, et le relevé
    /// les écartait toutes. `y y` ne dupliquait rien, `g g` n'allait
    /// nulle part, sans que rien ne le dise.
    ///
    /// Hors séquence, les répétitions restent écartées : garder le doigt
    /// sur une touche ne doit pas rejouer un raccourci trente fois par
    /// seconde. C'est la seconde moitié du test.
    #[test]
    fn a_sequence_can_double_a_key() {
        let bindings = KeyBindings::vim();
        let ctx = egui::Context::default();
        let mut dispatcher = Dispatcher::default();

        // Le premier `y` arme sans rien faire.
        let mut premier = None;
        run(&ctx, vec![key_event(Key::Y)], |ui| {
            premier = dispatcher.poll(ui.ctx(), &bindings, false);
        });
        assert_eq!(premier, None);
        assert_eq!(dispatcher.pending(), [Key::Y]);

        // Le second arrive marqué « répétition » — c'est ainsi qu'egui
        // décrit deux pressions rapprochées de la même touche.
        let répétition = egui::Event::Key {
            key: Key::Y,
            physical_key: None,
            pressed: true,
            repeat: true,
            modifiers: Modifiers::NONE,
        };
        let mut second = None;
        run(&ctx, vec![répétition.clone()], |ui| {
            second = dispatcher.poll(ui.ctx(), &bindings, false);
        });
        assert_eq!(
            second,
            Some(Action::DuplicateLine),
            "`y y` doit dupliquer, répétition ou non"
        );

        let _ = répétition;

        // Ce qu'on ne teste PAS ici, et pourquoi : « hors séquence, une
        // répétition ne déclenche rien ». La règle est dans le code — on
        // ne rejoue pas un raccourci trente fois par seconde parce qu'un
        // doigt traîne — mais egui recalcule lui-même le drapeau de
        // répétition d'après son historique de touches. Sur un contexte
        // neuf, la première frappe n'en est jamais une, quoi qu'on lui
        // passe : un test qui prétendrait le vérifier mesurerait autre
        // chose que ce qu'il annonce.
    }

    /// Les commandes d'exécution de gdb/pwndbg, en trois touches.
    #[test]
    fn a_three_key_sequence_resolves() {
        for (keys, expected) in [
            (vec![Key::Space, Key::S, Key::I], Action::StepInto),
            (vec![Key::Space, Key::N, Key::I], Action::StepOver),
            (vec![Key::Space, Key::P, Key::R], Action::Run),
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

    /// Une séquence en cours l'emporte sur un raccourci simple.
    ///
    /// `n` seul va à l'occurrence suivante ; `Espace n i` est Step Over.
    /// Sans cette règle, le `n` de la séquence déclenchait le raccourci
    /// simple et Step Over n'était plus atteignable — ce qui aurait
    /// interdit comme raccourci simple toute lettre servant de deuxième
    /// touche quelque part.
    #[test]
    fn a_sequence_in_progress_beats_a_simple_shortcut() {
        // Une table bâtie pour le test, où le conflit existe : `o` seul
        // est un raccourci, et `o` est aussi la deuxième touche de
        // `Espace p o`. Aucun des deux jeux livrés n'a plus ce cas — le
        // rangement par famille l'a fait disparaître —, mais la règle
        // doit rester vraie pour le jour où l'on ajoutera un raccourci.
        let mut map = BTreeMap::new();
        map.insert(
            Action::Stop,
            Chord::Simple(KeyboardShortcut::new(Modifiers::NONE, Key::O)),
        );
        map.insert(
            Action::StepOver,
            Chord::leader(&[Key::Space, Key::P, Key::O]),
        );
        let bindings = KeyBindings { map };

        // Seule, la touche déclenche son raccourci.
        let ctx = egui::Context::default();
        let mut dispatcher = Dispatcher::default();
        let mut seule = None;
        run(&ctx, vec![key_event(Key::O)], |ui| {
            seule = dispatcher.poll(ui.ctx(), &bindings, false);
        });
        assert_eq!(seule, Some(Action::Stop), "`o` seul fait son raccourci");

        // Au milieu d'une séquence, elle appartient à la séquence.
        let ctx = egui::Context::default();
        let mut dispatcher = Dispatcher::default();
        let mut suite = None;
        run(
            &ctx,
            vec![Key::Space, Key::P, Key::O]
                .into_iter()
                .map(key_event)
                .collect(),
            |ui| {
                suite = dispatcher.poll(ui.ctx(), &bindings, false);
            },
        );
        assert_eq!(
            suite,
            Some(Action::StepOver),
            "et la séquence reste atteignable"
        );
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
        events.extend(key_stroke(Key::P));
        events.extend(key_stroke(Key::R));
        let mut result = None;
        run(&ctx, events, |ui| {
            result = dispatcher.poll(ui.ctx(), &bindings, false);
        });
        assert_eq!(result, Some(Action::Run));
    }

    /// Une touche prise par un raccourci doit *disparaître* de l'entrée,
    /// et pas seulement être notée quelque part.
    ///
    /// C'est la différence qui comptait : tant que la touche restait dans
    /// les évènements, l'éditeur la voyait passer et le `i` final de
    /// `Espace s i` ouvrait le mode Insertion en même temps qu'il
    /// demandait un pas. La marquer ne protégeait que le code qui pensait
    /// à consulter la liste ; la retirer protège tout le monde.
    #[test]
    fn a_key_taken_by_a_shortcut_is_removed_from_the_input() {
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();
        let mut remaining = Vec::new();

        run(
            &ctx,
            vec![key_event(Key::Space), key_event(Key::S), key_event(Key::I)],
            |ui| {
                assert_eq!(
                    dispatcher.poll(ui.ctx(), &bindings, false),
                    Some(Action::StepInto)
                );
                // Ce que verra tout widget lisant l'entrée après nous.
                remaining = ui.ctx().input(|i| {
                    i.events
                        .iter()
                        .filter_map(|e| match e {
                            Event::Key {
                                key, pressed: true, ..
                            } => Some(*key),
                            _ => None,
                        })
                        .collect()
                });
            },
        );

        assert!(
            remaining.is_empty(),
            "les touches du raccourci doivent avoir quitté l'entrée, reste : {remaining:?}"
        );
    }

    /// Ce qui n'appartient à aucun raccourci, en revanche, doit traverser
    /// intact — sans quoi le dispatcher mangerait la frappe de tout le
    /// monde.
    #[test]
    fn keys_that_match_nothing_are_left_in_the_input() {
        let ctx = egui::Context::default();
        let bindings = KeyBindings::default();
        let mut dispatcher = Dispatcher::default();
        let mut remaining = Vec::new();

        run(&ctx, vec![key_event(Key::I), key_event(Key::Z)], |ui| {
            assert_eq!(dispatcher.poll(ui.ctx(), &bindings, false), None);
            remaining = ui.ctx().input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        Event::Key {
                            key, pressed: true, ..
                        } => Some(*key),
                        _ => None,
                    })
                    .collect()
            });
        });

        assert_eq!(remaining, [Key::I, Key::Z]);
    }

    /// Une séquence de trois touches se relit comme elle s'écrit, et une
    /// séquence vide ou trop longue n'existe pas.
    #[test]
    fn a_sequence_holds_between_one_and_three_keys() {
        let sequence = Sequence::new(&[Key::Space, Key::P, Key::I]).unwrap();
        assert_eq!(sequence.as_slice(), [Key::Space, Key::P, Key::I]);
        assert_eq!(sequence.first(), Key::Space);

        assert!(Sequence::new(&[]).is_none());
        assert!(Sequence::new(&[Key::A, Key::B, Key::C, Key::D]).is_none());

        // L'égalité ne regarde que les touches utiles, pas le remplissage
        // du tableau.
        assert_eq!(
            Sequence::new(&[Key::Space, Key::V, Key::E]),
            Sequence::new(&[Key::Space, Key::V, Key::E])
        );
        assert_ne!(
            Sequence::new(&[Key::Space, Key::V, Key::E]),
            Sequence::new(&[Key::Space, Key::E, Key::I])
        );
    }
}

#[cfg(test)]
mod family_tests {
    use super::*;

    /// Aucune séquence n'est le préfixe d'une autre.
    ///
    /// C'était le défaut de fond, et il s'est manifesté trois fois : une
    /// séquence courte se résout avant la longue, si bien que la longue
    /// devient inatteignable. `n` court-circuitait `Espace n i`, puis
    /// `Espace n` aussi.
    ///
    /// Ce contrôle vaut pour les deux jeux, et couvre aussi le cas mêlé —
    /// un raccourci simple qui est la deuxième touche d'une séquence est
    /// désormais permis (le dispatcher donne la priorité à la séquence en
    /// cours), mais une séquence préfixe d'une autre ne l'est jamais.
    #[test]
    fn no_sequence_is_the_prefix_of_another() {
        for (nom, jeu) in [
            ("vim", KeyBindings::vim()),
            ("ordinaire", KeyBindings::classic()),
        ] {
            let séquences: Vec<(Action, Vec<Key>)> = jeu
                .map
                .iter()
                .filter_map(|(action, chord)| match chord {
                    Chord::Leader(suite) => Some((*action, suite.as_slice().to_vec())),
                    Chord::Simple(_) => None,
                })
                .collect();
            for (a, courte) in &séquences {
                for (b, longue) in &séquences {
                    if a == b {
                        continue;
                    }
                    assert!(
                        !longue.starts_with(courte),
                        "jeu {nom} : {a:?} ({courte:?}) est le préfixe de {b:?} \
                         ({longue:?}), qui devient donc inatteignable"
                    );
                }
            }
        }
    }

    /// Chaque famille range ce qui va ensemble, et rien d'autre.
    ///
    /// C'est la règle qui remplace l'ancienne — « la première lettre de
    /// l'action » —, laquelle plaçait chaque ajout là où il restait de la
    /// place plutôt qu'à un endroit qui se devine.
    #[test]
    fn each_family_holds_what_belongs_together() {
        let jeu = KeyBindings::vim();
        let préfixe_de = |action: Action| -> Option<Vec<Key>> {
            match jeu.map.get(&action)? {
                Chord::Leader(suite) => {
                    let touches = suite.as_slice();
                    (touches.len() >= 2).then(|| touches[..2].to_vec())
                }
                Chord::Simple(_) => None,
            }
        };

        // Les pas, sous `Espace p` — SAUF les deux qui portent les
        // lettres de gdb.
        //
        // L'exception est écrite dans le contrôle plutôt que la règle
        // assouplie : `stepi` et `nexti` sont ce que les doigts tapent
        // depuis des années, et c'est une raison suffisante pour sortir
        // de la famille. Toute AUTRE sortie reste un défaut, et c'est ce
        // que la boucle continue de retenir.
        for action in [Action::Run, Action::StepBack, Action::RunToCursor] {
            assert_eq!(
                préfixe_de(action),
                Some(vec![Key::Space, Key::P]),
                "{action:?} appartient à la famille des pas"
            );
        }
        assert_eq!(
            préfixe_de(Action::StepInto),
            Some(vec![Key::Space, Key::S]),
            "entrer dans l'instruction garde le `s` de `stepi`"
        );
        assert_eq!(
            préfixe_de(Action::StepOver),
            Some(vec![Key::Space, Key::N]),
            "l'enjamber garde le `n` de `nexti`"
        );

        // Chercher, tous sous `Espace /`.
        for action in [
            Action::HighlightMatches,
            Action::NextMatch,
            Action::PreviousMatch,
            Action::RenameMatches,
        ] {
            assert_eq!(
                préfixe_de(action),
                Some(vec![Key::Space, Key::Slash]),
                "{action:?} appartient à la famille « chercher »"
            );
        }

        // Marquer, sous `Espace m`.
        for action in [Action::NameAddress, Action::CommentAddress] {
            assert_eq!(
                préfixe_de(action),
                Some(vec![Key::Space, Key::M]),
                "{action:?} appartient à la famille « marquer »"
            );
        }

        // Voir, sous `Espace v`.
        for action in [Action::ToggleExplorer, Action::ShowFlow] {
            assert_eq!(
                préfixe_de(action),
                Some(vec![Key::Space, Key::V]),
                "{action:?} appartient à la famille « voir »"
            );
        }

        // Et « cours jusqu'à », qui était déjà rangé ainsi.
        for action in [Action::RunToCall, Action::RunToReturn, Action::RunToSyscall] {
            assert_eq!(
                préfixe_de(action),
                Some(vec![Key::Space, Key::U]),
                "{action:?} appartient à la famille « cours jusqu'à »"
            );
        }
    }

    /// Une même touche ne sert pas deux fois dans un jeu.
    ///
    /// Deux actions sur le même raccourci en rendent une inatteignable,
    /// et laquelle dépend de l'ordre de parcours de la table — c'est-à-dire
    /// du hasard.
    #[test]
    fn no_shortcut_is_used_twice() {
        for (nom, jeu) in [
            ("vim", KeyBindings::vim()),
            ("ordinaire", KeyBindings::classic()),
        ] {
            let mut vus: std::collections::BTreeMap<String, Action> = Default::default();
            for (action, chord) in &jeu.map {
                let clé = format!("{chord:?}");
                if let Some(déjà) = vus.insert(clé.clone(), *action) {
                    panic!("jeu {nom} : {déjà:?} et {action:?} partagent {clé}");
                }
            }
        }
    }
}
