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
use crate::keybindings::{Action, Chord, KeyBindings, Preset, Sequence};

/// `#[serde(default)]` au niveau de la struct : un champ absent du fichier
/// est repris de `Settings::default()`, donc une configuration écrite par
/// une version plus ancienne reste lisible.
/// Version du format de réglages. À incrémenter quand un *défaut* change
/// d'une façon qui doit atteindre les fichiers déjà écrits — voir
/// `migrate`.
pub const SETTINGS_VERSION: u32 = 3;

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
    /// Les barres d'icônes des bords, qui montrent et cachent les
    /// panneaux.
    ///
    /// Vraies sur une installation neuve : un panneau fermé ne laissait
    /// aucune trace à l'écran, et l'on ne ferme pas ce qu'on ne sait pas
    /// rouvrir. Qui les trouve encombrantes les éteint au menu
    /// Affichage, et retrouve l'écran d'avant.
    #[serde(default = "vrai")]
    pub show_rails: bool,
    pub show_sidebar: bool,
    pub show_statusbar: bool,
    /// Panneau bas — la console du programme et les diagnostics de nasm.
    ///
    /// **FAUX sur une installation neuve**, et c'est la seule des
    /// quatre bascules d'affichage qui commence fermée. Avant d'avoir
    /// assemblé quoi que ce soit, la console n'a rien à dire : elle
    /// occupe soixante-douze pixels pour écrire « la sortie du
    /// programme s'affichera ici », c'est-à-dire pour annoncer qu'elle
    /// est vide. Sur l'écran que voit quelqu'un qui ouvre namsdec pour
    /// la première fois — et c'est désormais le mode débutant —, cela
    /// fait une zone de plus à comprendre pour rien.
    ///
    /// Elle s'ouvre **d'elle-même dès qu'elle a quelque chose à
    /// montrer** : une erreur de nasm (`show_diagnostics_if_any`) ou le
    /// premier octet qu'un programme écrit. C'est ce qui rend la
    /// fermeture tenable — sans cela, un programme qui affiche « salut »
    /// n'aurait nulle part où l'afficher, et l'on croirait qu'il n'a
    /// rien fait.
    ///
    /// Une configuration qui existe déjà garde la sienne : le champ y
    /// est écrit, donc ce défaut-ci ne la concerne pas.
    pub show_console: bool,
    /// L'explorateur de fichiers, à gauche.
    ///
    /// Persisté comme ses frères : le fermer puis rouvrir namsdec le
    /// ramenait, alors que la console, la barre latérale et la barre
    /// d'état, elles, se souvenaient.
    #[serde(default)]
    pub show_explorer: bool,
    /// Les infobulles au survol. Certains les veulent, d'autres les
    /// trouvent envahissantes — d'où un réglage plutôt qu'un choix imposé.
    pub show_tooltips: bool,
    /// Le déroulement montre-t-il le pseudo-code plutôt que le graphe ?
    ///
    /// Deux façons de lire la même fonction : la forme d'un côté, le
    /// texte de l'autre. Le choix se garde parce qu'il tient à la
    /// personne — on lit un graphe pour repérer, un pseudo-code pour
    /// comprendre — et non au binaire qu'on ouvre.
    #[serde(default)]
    pub flow_pseudo: bool,
    /// L'autocomplétion de l'éditeur : elle propose, `Alt` accepte.
    pub completion: bool,
    /// Entrée garde l'indentation de la ligne qu'on quitte.
    #[serde(default = "vrai")]
    pub auto_indent: bool,
    /// Les paires — ( ) [ ] { } " ' — se ferment toutes seules.
    #[serde(default = "vrai")]
    pub auto_pairs: bool,
    /// La fiche de l'instruction au survol du code.
    #[serde(default = "vrai")]
    pub hover_help: bool,
    /// Où mène le bouton de don de la fenêtre « À propos ».
    ///
    /// UN RÉGLAGE ET NON UNE CONSTANTE : une adresse de paiement
    /// appartient à qui reçoit, et ne se devine pas. Elle valait
    /// `https://example.com/…` dans le binaire — un domaine réservé à
    /// la documentation, où personne ne reçoit rien —, si bien que le
    /// bouton paraissait actif et menait nulle part.
    ///
    /// Vide, le bouton est éteint et dit où poser l'adresse. Ko-fi,
    /// PayPal.me, Liberapay, GitHub Sponsors : namsdec n'en privilégie
    /// aucun, il ouvre ce qu'on lui donne.
    #[serde(default)]
    pub donation_url: String,
    /// L'interface est-elle réduite à l'essentiel ?
    ///
    /// **namsdec sert à apprendre l'assembleur et la
    /// rétro-ingénierie.** Vingt-cinq rubriques d'analyse devant
    /// quelqu'un qui ouvre son premier binaire lui demandent de savoir
    /// ce qu'il cherche avant d'avoir appris à chercher. En mode
    /// débutant, la colonne n'en montre que neuf — celles qui se lisent
    /// sans rien savoir d'avance ; rien n'est désinstallé, et la case
    /// rend le reste d'un clic.
    ///
    /// VRAI PAR DÉFAUT, et c'est la pédagogie qui le décide : une
    /// première ouverture doit montrer un outil qu'on peut lire. Une
    /// configuration qui existe DÉJÀ, elle, passe à faux lors de la
    /// migration — son propriétaire se sert de l'outil entier depuis
    /// toujours, et un réglage neuf n'a pas à le lui retirer.
    #[serde(default = "vrai")]
    pub debutant: bool,
    /// Faut-il **prévoir** ce qu'une instruction va faire avant de
    /// l'exécuter ?
    ///
    /// Devant une instruction pas encore exécutée, namsdec dit tout :
    /// ce qu'elle fait en clair, ses opérandes résolus, et — devant un
    /// saut conditionnel — s'il sera pris. On appuie sur « Pas à pas »
    /// pour vérifier une réponse qu'on a lue, et l'on n'a jamais eu
    /// l'occasion de se tromper. Avec ce réglage, l'instruction reste à
    /// l'écran, la réponse attend le pas, et « Dernier pas » la révèle à
    /// côté de ce qui a vraiment changé.
    ///
    /// **Il ne vaut qu'au niveau débutant** : un exercice imposé à qui
    /// se sert de l'outil tous les jours serait une taxe. Vrai par
    /// défaut, pour la même raison que `debutant` — et sans effet tant
    /// que celui-ci est faux. Voir `crate::prevision`.
    #[serde(default = "vrai")]
    pub prevoir: bool,
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
    /// Les outils qu'on a localisés à la main : leur identifiant → le
    /// chemin absolu du binaire choisi. Pour un outil bâti hors du PATH —
    /// DZF, en cours de développement, dans le dépôt de l'utilisateur —,
    /// c'est la seule façon pour namsdec de le trouver.
    #[serde(default)]
    pub located_tools: std::collections::BTreeMap<String, String>,
    /// L'exercice chargé en dernier, sous la forme « série/exercice ».
    ///
    /// Un parcours de douze étapes se fait en plusieurs fois : au retour,
    /// l'outil doit pouvoir dire où l'on en était plutôt que de laisser
    /// chercher.
    #[serde(default)]
    pub current_exercise: Option<String>,
    /// Les derniers fichiers ouverts, le plus récent en tête.
    ///
    /// Des chemins, jamais leur contenu : rouvrir un fichier déplacé
    /// doit échouer en le disant, pas ressortir une copie d'hier.
    #[serde(default)]
    pub recent_files: Vec<String>,
    /// Les exercices dont le but a été atteint au moins une fois.
    ///
    /// Un parcours sans trace de ce qu'on a déjà fait oblige à s'en
    /// souvenir soi-même — et l'on refait le premier trois fois plutôt
    /// que d'attaquer le septième.
    #[serde(default)]
    pub exercises_done: Vec<String>,
    /// Le meilleur compte d'instructions obtenu sur chaque exercice.
    ///
    /// Le score d'un exercice d'assembleur, c'est ce qu'il coûte à
    /// exécuter : un programme juste peut être deux fois plus court, et
    /// c'est en cherchant comment qu'on apprend. Gardé par identifiant
    /// complet, comme le reste.
    #[serde(default)]
    pub exercises_best: Vec<(String, u64)>,
    /// Le moteur du panneau de désassemblage : « pseudo », « nasm », ou
    /// l'identifiant d'un outil installé. Celui qu'on a choisi doit
    /// revenir à la session suivante, comme le reste.
    #[serde(default = "pseudo_par_défaut")]
    pub disassembler: String,
    /// Comment l'onglet Mémoire lit les octets : « bytes », « words » ou
    /// « text ».
    pub memory_format: String,
    /// La base de la calculatrice : « hex », « dec », « oct » ou « bin ».
    /// Le module que la fenêtre d'analyse montre, ou rien pour les
    /// montrer tous.
    #[serde(default)]
    pub analysis_module: String,
    #[serde(default = "base_par_défaut")]
    pub calc_base: String,
    /// La famille de fonctions ouverte dans la calculatrice : « bits »,
    /// « ordre », « signe », « compter », « tourner », « calcul ».
    #[serde(default)]
    pub calc_family: String,
    /// Sur combien de bits elle montre son résultat : « 8 », « 16 »,
    /// « 32 » ou « 64 ».
    #[serde(default = "largeur_par_défaut")]
    pub calc_width: String,
    /// La légende des couleurs de la gouttière, sous l'éditeur.
    #[serde(default = "vrai")]
    pub heat_legend: bool,
    /// Le graphe de flot : n'afficher que le chemin courant.
    ///
    /// Vrai par défaut — d'où le défaut explicite : sans lui, un fichier
    /// de réglages écrit avant ce champ rendrait `false` et changerait la
    /// vue sans qu'on l'ait demandé.
    #[serde(default = "vrai")]
    pub flow_current_only: bool,
    /// Le déroulement : ne montrer que les pas remarquables.
    #[serde(default)]
    pub timeline_notable_only: bool,
    /// La console : le repli des variables d'environnement, déplié ou non.
    #[serde(default)]
    pub console_env_open: bool,
    /// La rangée des arguments du programme est-elle dépliée ?
    ///
    /// **Fermée par défaut, et c'est ce qui rend la console lisible.**
    /// Les arguments et l'environnement ne se tapent pas pendant qu'un
    /// programme tourne — ils sont posés sur la pile avant la première
    /// instruction. Toujours affichés, ils prenaient deux rangées sur
    /// les quatre du bas, et la sortie n'avait plus qu'une demi-ligne :
    /// « la console et les autres champs ne sont pas adaptés →
    /// inutilisable » (signalé à l'écran le 4 novembre 2026, capture à
    /// l'appui).
    ///
    /// C'est le partage des outils de développement : ce qu'on RÈGLE
    /// avant de lancer vit à part, ce qu'on LIT et ce qu'on TAPE pendant
    /// la course occupe la fenêtre. Le bouton « Arguments… » de la
    /// rangée d'onglets les rouvre, et le réglage est gardé.
    pub console_args_open: bool,
    /// Rendre la barre de titre au système au lieu de celle que namsdec
    /// dessine. Faux par défaut : les menus *sont* la barre de titre, ce
    /// qui économise une rangée. Vrai pour qui préfère la décoration de
    /// son bureau, ou pour qui a un ennui avec la nôtre.
    #[serde(default)]
    pub system_titlebar: bool,
    /// La hauteur de la console, en pixels, quand elle a été réglée à la
    /// main. Absente — le cas par défaut — la console prend la place de
    /// ce qu'elle a à dire.
    ///
    /// En entier : un fichier de réglages n'a que faire de la fraction
    /// de pixel qu'une poignée de souris laisse derrière elle.
    #[serde(default)]
    pub console_height: Option<u32>,
    /// La hauteur, en pixels, de la part du panneau latéral réservée au
    /// contenu de l'onglet — Pile, Mémoire, Tas, Octets, RE — quand la
    /// poignée a été tirée. Absente, cette part est ce que le bandeau
    /// laisse.
    #[serde(default)]
    pub sidebar_tab_height: Option<u32>,
    /// L'onglet de rétro-ingénierie est-il montré ?
    ///
    /// C'est une case des préférences, cochée à la main, et elle tient
    /// donc sur le disque comme les autres. Elle était restée hors du
    /// fichier au motif qu'un repli d'en-tête ne se sauvegarde pas —
    /// mais ce n'est pas un repli : c'est un choix, et le refaire à
    /// chaque lancement est exactement ce qu'un réglage sert à éviter.
    #[serde(default)]
    pub re_tab: bool,
    /// Le module de décompilation est-il autorisé ?
    ///
    /// Faux par défaut, et ce défaut compte : cocher cette case donne à
    /// namsdec la permission de lancer un programme extérieur. On ne
    /// l'accorde pas à la place de quelqu'un.
    #[serde(default)]
    pub decompiler_module: bool,
    /// Le module qui va chercher les outils et les construit.
    ///
    /// Faux par défaut, comme le précédent, et pour une raison plus
    /// forte encore : cocher cette case autorise namsdec à télécharger
    /// du code et à l'exécuter. Ce réglage ne se devine pas, il se
    /// prend.
    #[serde(default)]
    pub installer_module: bool,
    /// L'outil de décompilation choisi, par identifiant.
    #[serde(default)]
    pub decompiler: String,
    /// Ce que l'onglet Mémoire suit pas à pas : « fixed », « rsp » ou
    /// « rip ».
    #[serde(default = "fixe_par_défaut")]
    pub memory_follow: String,
    /// Dernier dossier ouvert dans l'explorateur, pour le retrouver au
    /// lancement suivant.
    pub explorer_root: Option<String>,
    /// Ce qu'on fait, au lancement, du travail de la séance précédente :
    /// « ask », « always » ou « never ».
    ///
    /// « ask » par défaut, et c'est un changement assumé : namsdec
    /// rouvrait d'autorité le fichier et le binaire qu'on avait quittés.
    /// Commode quand on reprend le même travail, faux dès qu'on ouvre
    /// namsdec pour autre chose — on tombe alors sur vingt mille lignes
    /// de désassemblage d'hier, qu'il faut d'abord fermer. Un fichier de
    /// réglages d'une version antérieure n'a pas ce champ, et reçoit
    /// donc la question lui aussi : c'est la bonne réponse pour qui n'a
    /// jamais choisi.
    #[serde(default = "demander_par_défaut")]
    pub session_restore: String,
    /// Dernier fichier ouvert dans l'éditeur. Le **chemin**, jamais le
    /// contenu : le fichier source vit sur le disque, où l'utilisateur
    /// peut l'éditer avec autre chose, et en recopier une version dans les
    /// réglages ne ferait qu'y entretenir un double périmé.
    pub current_file: Option<String>,
    /// La date qu'avait ce fichier quand on a quitté, en secondes depuis
    /// 1970.
    ///
    /// Elle sert à la même chose que celle du binaire : les points
    /// d'arrêt se comptent en numéros de ligne, et un fichier réécrit
    /// ailleurs entre deux séances les ferait désigner d'autres
    /// instructions. Absente d'un fichier de réglages écrit par une
    /// version antérieure — les repères sont alors simplement laissés de
    /// côté, ce qui est le comportement d'avant.
    #[serde(default)]
    pub current_file_mtime: Option<u64>,
    /// Les autres fichiers ouverts au moment où l'on a quitté, dans
    /// l'ordre des onglets — leurs **chemins**, pour les mêmes raisons
    /// que ci-dessus. Ce qu'on y avait tapé sans l'enregistrer n'y est
    /// pas : il n'a jamais quitté la mémoire.
    #[serde(default)]
    pub open_files: Vec<String>,
    /// Le binaire ouvert quand on a quitté, s'il y en avait un.
    ///
    /// Son **chemin**, comme pour les sources — mais pour une autre
    /// raison : un binaire ne s'édite pas ici, et ce qu'on veut retrouver
    /// n'est pas son contenu mais le travail qu'on avait commencé dessus.
    /// `current_file` ne peut pas servir : il est délibérément vide pour
    /// un binaire, afin qu'un Ctrl+S distrait n'écrase pas l'exécutable
    /// par son propre désassemblage.
    #[serde(default)]
    pub open_binary: Option<String>,
    /// Sa taille et sa date, au moment où on l'a quitté.
    ///
    /// Ce qui suit — points d'arrêt, zones surveillées — se repère par
    /// numéro de ligne du listing, et un listing est reconstruit à chaque
    /// ouverture. Tant que le fichier est le même, les lignes le sont
    /// aussi. S'il a changé, ces repères désignent autre chose, et les
    /// restaurer poserait des points d'arrêt au hasard : on les oublie
    /// alors, et on le dit.
    #[serde(default)]
    pub open_binary_len: Option<u64>,
    #[serde(default)]
    pub open_binary_mtime: Option<u64>,
    /// Les lignes où l'on avait posé un point d'arrêt.
    #[serde(default)]
    pub breakpoints: Vec<u32>,
    /// La ligne qu'on lisait en quittant.
    ///
    /// Un listing de binaire fait des dizaines de milliers de lignes :
    /// retrouver le fichier et ses points d'arrêt, mais rouvrir à la
    /// ligne 1, c'est laisser le plus long du travail à refaire — se
    /// replacer. Elle suit la même garde que les points d'arrêt : même
    /// taille, même date, sinon elle ne désigne plus la même
    /// instruction. Absente quand on lisait le début, pour ne pas
    /// écrire un réglage qui ne dit rien.
    #[serde(default)]
    pub listing_line: Option<u32>,
    /// Les zones de mémoire surveillées : adresse et longueur.
    #[serde(default)]
    pub watchpoints: Vec<(u64, u32)>,
    /// Les registres surveillés, par nom.
    #[serde(default)]
    pub watched_registers: Vec<String>,
    /// Ce qu'on donne au programme : ses arguments, et son
    /// environnement. Réglés à la main, donc gardés — les retaper à
    /// chaque lancement est exactement ce qu'un réglage évite.
    #[serde(default)]
    pub program_args: String,
    #[serde(default)]
    pub program_env: String,
    /// L'onglet du panneau latéral qu'on regardait.
    #[serde(default)]
    pub sidebar_tab: Option<String>,
    /// La vue qu'on regardait dans la zone centrale : la source, ou le
    /// désassemblage. Gardée comme l'onglet latéral — on retrouve son
    /// établi là où on l'a laissé.
    #[serde(default)]
    pub central_view: Option<String>,
    /// Les fenêtres laissées ouvertes à la dernière séance.
    ///
    /// Une liste de noms plutôt qu'un booléen par fenêtre : le fichier
    /// ne porte alors que ce qui est ouvert — une ligne courte et
    /// lisible — et une fenêtre retirée d'une version à l'autre ne
    /// laisse pas un réglage orphelin derrière elle. La table qui dit
    /// quel nom ouvre quoi est dans `main.rs`, sous `FENÊTRES_REPRISES`.
    #[serde(default)]
    pub open_windows: Vec<String>,
    /// Identifiant d'action -> raccourci, en texte.
    pub shortcuts: BTreeMap<String, String>,
}

/// L'hexadécimal : c'est la base d'un désassemblage, et c'est de là
/// qu'on recopie ce qu'on veut calculer.
fn base_par_défaut() -> String {
    "hex".to_owned()
}

/// Soixante-quatre bits, la largeur de la machine simulée.
fn largeur_par_défaut() -> String {
    "64".to_owned()
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
            show_rails: true,
            show_sidebar: true,
            show_statusbar: true,
            // Faux : voir le champ. Elle s'ouvre dès qu'elle a quelque
            // chose à montrer.
            show_console: false,
            show_explorer: false,
            show_tooltips: true,
            // Vrai : voir le champ, et la raison — un outil qui sert à
            // apprendre s'ouvre sur ce qui s'apprend.
            debutant: true,
            // Vrai aussi, et sans effet tant que `debutant` est faux :
            // le réglage ne se montre qu'avec lui.
            prevoir: true,
            // Le graphe d'abord : c'est la vue qui montre la FORME, et
            // c'est elle qui manque quand on découvre un binaire.
            flow_pseudo: false,
            completion: true,
            exercises: false,
            auto_indent: true,
            auto_pairs: true,
            hover_help: true,
            donation_url: String::new(),
            disabled_packs: Vec::new(),
            located_tools: std::collections::BTreeMap::new(),
            current_exercise: None,
            recent_files: Vec::new(),
            exercises_done: Vec::new(),
            exercises_best: Vec::new(),
            disassembler: pseudo_par_défaut(),
            memory_format: "bytes".to_owned(),
            analysis_module: String::new(),
            calc_base: base_par_défaut(),
            calc_family: String::new(),
            calc_width: largeur_par_défaut(),
            heat_legend: true,
            flow_current_only: true,
            timeline_notable_only: false,
            console_env_open: false,
            // Fermée : voir le champ. La sortie passe avant ce qu'on
            // règle une fois pour toutes.
            console_args_open: false,
            system_titlebar: false,
            console_height: None,
            sidebar_tab_height: None,
            re_tab: false,
            decompiler_module: false,
            installer_module: false,
            decompiler: String::new(),
            memory_follow: fixe_par_défaut(),
            open_binary: None,
            open_binary_len: None,
            open_binary_mtime: None,
            breakpoints: Vec::new(),
            listing_line: None,
            watchpoints: Vec::new(),
            watched_registers: Vec::new(),
            program_args: String::new(),
            program_env: String::new(),
            sidebar_tab: None,
            central_view: None,
            open_windows: Vec::new(),
            explorer_root: None,
            session_restore: demander_par_défaut(),
            current_file: None,
            current_file_mtime: None,
            open_files: Vec::new(),
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
        if self.version < 3 {
            // v2 -> v3 : le mode débutant paraît, et il est vrai par
            // défaut. Mais une configuration qui existait AVANT lui
            // appartient à quelqu'un qui se sert de l'outil entier
            // depuis toujours : lui retirer seize rubriques au nom
            // d'un défaut neuf serait prendre une décision à sa place.
            //
            // C'est la même règle que pour les raccourcis juste
            // au-dessus, et elle vaut d'être redite : on hérite d'un
            // défaut, on ne se fait pas imposer un changement.
            self.debutant = false;
        }
        self.version = SETTINGS_VERSION;
        self
    }

    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        // Atomique : ce fichier porte TOUT ce que l'utilisateur a réglé,
        // et une écriture coupée en deux le rendrait illisible — donc
        // remplacé par les défauts au lancement suivant.
        crate::write_atomically(path, serde_json::to_string_pretty(self)?.as_bytes())
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

#[cfg(test)]
mod tests_keymap {
    use super::*;

    /// Le fichier de raccourcis dit le jeu, et ce qui s'en écarte.
    ///
    /// Écrire seulement l'écart plutôt que la liste entière : on voit
    /// d'un coup d'œil ce qu'on a changé, et le jour où un jeu gagne une
    /// action, elle arrive sans qu'on ait rien à faire.
    #[test]
    fn a_keymap_records_only_what_departs_from_its_set() {
        // Rien changé : rien à écrire.
        let tel_quel = Keymap::from_bindings(Preset::Vim, &KeyBindings::preset(Preset::Vim));
        assert_eq!(tel_quel.preset, "vim");
        assert!(
            tel_quel.shortcuts.is_empty(),
            "obtenu {:?}",
            tel_quel.shortcuts
        );

        // Une touche déplacée, et elle seule.
        let mut à_moi = KeyBindings::preset(Preset::Classic);
        à_moi.map.insert(
            Action::Run,
            Chord::Simple(egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::Enter,
            )),
        );
        let écrit = Keymap::from_bindings(Preset::Classic, &à_moi);
        assert_eq!(écrit.preset, "classic");
        assert_eq!(écrit.shortcuts.len(), 1, "obtenu {:?}", écrit.shortcuts);
        assert!(écrit.shortcuts.contains_key("run"));

        // Et relu, on retrouve exactement ce qu'on avait.
        assert_eq!(écrit.bindings().map, à_moi.map);
    }

    /// Un fichier abîmé ne fait pas perdre le reste des raccourcis.
    #[test]
    fn an_unreadable_line_costs_only_its_own_shortcut() {
        let keymap = Keymap {
            preset: "classic".to_owned(),
            shortcuts: [
                ("run".to_owned(), "Ctrl+Enter".to_owned()),
                ("stop".to_owned(), "n'importe quoi".to_owned()),
                ("action_qui_nexiste_pas".to_owned(), "Ctrl+Z".to_owned()),
            ]
            .into_iter()
            .collect(),
        };
        let bindings = keymap.bindings();
        let socle = KeyBindings::preset(Preset::Classic);

        // Celle qu'on a su lire est prise.
        assert_ne!(bindings.map.get(&Action::Run), socle.map.get(&Action::Run));
        // Celle qu'on n'a pas su lire garde celle du jeu.
        assert_eq!(
            bindings.map.get(&Action::Stop),
            socle.map.get(&Action::Stop)
        );
        // Et une action inconnue est simplement passée.
        assert_eq!(bindings.map.len(), socle.map.len());
    }

    /// Un jeu inconnu retombe sur celui de départ, sans rien perdre.
    #[test]
    fn an_unknown_set_falls_back_to_the_starting_one() {
        let keymap = Keymap {
            preset: "emacs".to_owned(),
            shortcuts: BTreeMap::new(),
        };
        assert_eq!(keymap.preset(), Preset::default());
        assert_eq!(keymap.bindings().map, KeyBindings::default().map);
    }

    /// Le fichier se relit tel qu'on l'a écrit.
    #[test]
    fn a_keymap_survives_a_round_trip_through_json() {
        let keymap = Keymap::from_bindings(Preset::Classic, &KeyBindings::preset(Preset::Vim));
        let texte = serde_json::to_string_pretty(&keymap).expect("écriture");
        let relu: Keymap = serde_json::from_str(&texte).expect("lecture");
        assert_eq!(relu.preset, keymap.preset);
        assert_eq!(relu.shortcuts, keymap.shortcuts);
        // Et le fichier se lit à l'œil : une ligne par raccourci.
        assert!(texte.contains("\"preset\""), "{texte}");
        assert!(texte.contains("\"shortcuts\""), "{texte}");
    }
}

/// `$XDG_CONFIG_HOME/namsdec/keymap.json`, sinon
/// `$HOME/.config/namsdec/keymap.json`.
///
/// Un fichier à part de `settings.json` : on y touche autrement — on
/// l'ouvre pour le lire, on le copie d'une machine à l'autre, on le met
/// dans un dépôt de fichiers de configuration. Le mêler aux réglages
/// obligerait à trier tout cela à la main.
pub fn keymap_path() -> Option<PathBuf> {
    settings_path().map(|p| p.with_file_name("keymap.json"))
}

/// Ce que le fichier de raccourcis contient.
///
/// Un jeu de départ nommé, et ce dont l'utilisateur s'en écarte. Écrire
/// les deux plutôt que la seule liste finale : on voit ainsi d'un coup
/// d'œil ce qui a été changé, et le jour où un jeu gagne une action, elle
/// arrive sans qu'on ait rien à faire.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Keymap {
    /// `vim` ou `classic`.
    pub preset: String,
    /// Les frappes, par identifiant d'action. Celles qui manquent
    /// viennent du jeu.
    #[serde(default)]
    pub shortcuts: BTreeMap<String, String>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            preset: Preset::default().id().to_owned(),
            shortcuts: BTreeMap::new(),
        }
    }
}

impl Keymap {
    /// Le jeu nommé, ou celui de départ si le nom ne dit rien.
    pub fn preset(&self) -> Preset {
        Preset::from_id(&self.preset).unwrap_or_default()
    }

    /// Les raccourcis qu'il décrit : le jeu, puis ce qui s'en écarte.
    pub fn bindings(&self) -> KeyBindings {
        let mut bindings = KeyBindings::preset(self.preset());
        for (id, frappe) in &self.shortcuts {
            let Some(action) = action_from_id(id) else {
                continue;
            };
            match chord_from_string(frappe) {
                Some(chord) => {
                    bindings.map.insert(action, chord);
                }
                // Une frappe illisible ne doit pas emporter le reste du
                // fichier : on garde celle du jeu et l'on passe.
                None => continue,
            }
        }
        bindings
    }

    /// Ce qui s'écarte du jeu, et cela seulement.
    pub fn from_bindings(preset: Preset, bindings: &KeyBindings) -> Self {
        let socle = KeyBindings::preset(preset);
        let shortcuts = bindings
            .map
            .iter()
            .filter(|(action, chord)| socle.map.get(action) != Some(chord))
            .map(|(action, chord)| (action_id(*action).to_owned(), chord_to_string(chord)))
            .collect();
        Self {
            preset: preset.id().to_owned(),
            shortcuts,
        }
    }
}

/// Lit le fichier de raccourcis, s'il existe et se laisse lire.
///
/// Un fichier absent n'est pas une faute : c'est le cas de qui n'a jamais
/// rien changé, et le jeu de départ répond pour lui.
pub fn load_keymap() -> Keymap {
    let Some(chemin) = keymap_path() else {
        return Keymap::default();
    };
    let Ok(texte) = std::fs::read_to_string(&chemin) else {
        return Keymap::default();
    };
    serde_json::from_str(&texte).unwrap_or_default()
}

/// Écrit le fichier de raccourcis.
pub fn save_keymap(keymap: &Keymap) -> std::io::Result<()> {
    let Some(chemin) = keymap_path() else {
        return Ok(());
    };
    let texte = serde_json::to_string_pretty(keymap)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    // Atomique, pour la même raison que les réglages : un jeu de
    // raccourcis se façonne touche par touche, et se reperdrait d'un coup.
    crate::write_atomically(&chemin, (texte + "\n").as_bytes())
}

/// Le dossier des jeux de raccourcis NOMMÉS, à côté de `keymap.json`.
///
/// `keymap.json` est le jeu ACTIF, celui que namsdec lit au démarrage et
/// réécrit à chaque touche posée. Il n'y en a qu'un, et c'est ce qui
/// manquait : on façonne un jeu pour la rétro-ingénierie, un autre pour
/// écrire de l'assembleur, un troisième qu'on a copié d'une machine, et
/// il n'y avait nulle part où les garder — passer de l'un à l'autre
/// voulait dire les recopier à la main.
///
/// Ce dossier les garde. namsdec le LIT seulement : il n'y écrit que sur
/// demande explicite, et n'y touche jamais au démarrage.
pub fn keymaps_dir() -> Option<PathBuf> {
    keymap_path().and_then(|p| p.parent().map(|d| d.join("keymaps")))
}

/// Les jeux enregistrés, par leur nom de fichier sans l'extension.
///
/// Triés, pour que la liste ne danse pas d'un lancement à l'autre selon
/// l'ordre que le système rend.
pub fn list_keymaps() -> Vec<(String, PathBuf)> {
    let Some(dir) = keymaps_dir() else {
        return Vec::new();
    };
    let Ok(entrées) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut trouvés: Vec<(String, PathBuf)> = entrées
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .filter_map(|p| {
            let nom = p.file_stem()?.to_string_lossy().into_owned();
            Some((nom, p))
        })
        .collect();
    trouvés.sort_by(|a, b| a.0.cmp(&b.0));
    trouvés
}

/// Lit un jeu enregistré.
pub fn read_keymap(chemin: &std::path::Path) -> Option<Keymap> {
    serde_json::from_str(&std::fs::read_to_string(chemin).ok()?).ok()
}

/// Enregistre le jeu courant sous un nom, dans le dossier des jeux.
///
/// Le nom est NETTOYÉ : il devient un nom de fichier, et un nom de
/// fichier ne contient ni séparateur ni `..`. Sans cela, « ../../tout »
/// écrirait où il veut — ce qui n'est pas un risque théorique dans un
/// champ de saisie libre.
pub fn save_keymap_as(nom: &str, keymap: &Keymap) -> std::io::Result<PathBuf> {
    let propre: String = nom
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let propre = propre.trim().to_owned();
    if propre.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "un nom vide",
        ));
    }
    let Some(dir) = keymaps_dir() else {
        return Err(std::io::Error::other("aucun dossier de réglages"));
    };
    std::fs::create_dir_all(&dir)?;
    let chemin = dir.join(format!("{propre}.json"));
    let texte = serde_json::to_string_pretty(keymap)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    crate::write_atomically(&chemin, (texte + "\n").as_bytes())?;
    Ok(chemin)
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
/// Par défaut, l'adresse ne bouge pas : c'est ce qu'on attend d'un
/// vidage qu'on vient de poser quelque part.
fn fixe_par_défaut() -> String {
    "fixed".to_owned()
}

/// Au lancement, on demande : voir `session_restore`.
fn demander_par_défaut() -> String {
    "ask".to_owned()
}

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
pub fn action_id(action: Action) -> &'static str {
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
        Action::RunToCall => "run_to_call",
        Action::RunToReturn => "run_to_return",
        Action::RunToSyscall => "run_to_syscall",
        Action::GoToLabel => "goto_label",
        Action::Find => "find",
        Action::New => "new",
        Action::ShowFlow => "show_flow",
        Action::NextTab => "next_tab",
        Action::PreviousTab => "previous_tab",
        Action::CloseTab => "close_tab",
        Action::DuplicateLine => "duplicate_line",
        Action::HighlightMatches => "highlight_matches",
        Action::RenameMatches => "rename_matches",
        Action::NextMatch => "next_match",
        Action::PreviousMatch => "previous_match",
        Action::NameAddress => "name_address",
        Action::CommentAddress => "comment_address",
        Action::ToggleCalculator => "toggle_calculator",
        Action::ToggleCommandPalette => "toggle_command_palette",
        Action::ShowPreferences => "show_preferences",
        Action::ToggleConsole => "toggle_console",
        Action::ToggleSidebar => "toggle_sidebar",
    }
}

fn action_from_id(id: &str) -> Option<Action> {
    Action::ALL.into_iter().find(|a| action_id(*a) == id)
}

/// `"Ctrl+Shift+B"` pour un raccourci simple, `"Space,S,I"` pour une
/// séquence. La virgule ne peut pas apparaître dans un nom de touche
/// (`Key::name()` rend `"Comma"`), elle distingue donc les deux formes
/// sans ambiguïté, et en sépare les touches quel qu'en soit le nombre.
pub fn chord_to_string(chord: &Chord) -> String {
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

    /// LE MODE DÉBUTANT EST VRAI POUR UNE INSTALLATION NEUVE, ET FAUX
    /// POUR UNE CONFIGURATION QUI EXISTAIT DÉJÀ.
    ///
    /// namsdec sert à apprendre : une première ouverture doit montrer
    /// un outil qu'on peut lire, donc neuf rubriques et non vingt-cinq.
    /// Mais le fichier de quelqu'un qui se sert de l'outil entier
    /// depuis des mois ne doit pas se réveiller amputé — il a hérité
    /// d'un défaut qui n'existait pas, et la migration le range du côté
    /// où il était.
    #[test]
    fn le_mode_debutant_est_neuf_pour_une_installation_neuve() {
        // Une installation neuve : le mode débutant.
        assert!(
            Settings::default().debutant,
            "une installation neuve s'ouvre sur ce qui s'apprend"
        );
        // Un fichier qui existait avant lui : l'interface entière.
        let ancien: Settings = serde_json::from_str(r#"{"version":2}"#).unwrap();
        assert!(
            ancien.debutant,
            "le champ absent prend le défaut, c'est la migration qui décide"
        );
        let migré = ancien.migrate();
        assert!(
            !migré.debutant,
            "une configuration d'avant garde l'interface entière"
        );
        assert_eq!(migré.version, SETTINGS_VERSION);
        // Et un fichier déjà à jour n'est pas retouché : ce qu'on a
        // choisi reste choisi.
        let choisi: Settings = serde_json::from_str(&format!(
            r#"{{"version":{SETTINGS_VERSION},"debutant":true}}"#
        ))
        .unwrap();
        assert!(choisi.migrate().debutant);
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

        // Ce qu'on vérifie est la MIGRATION, non les touches du jour :
        // les valeurs attendues viennent du jeu par défaut, et le test ne
        // tombe donc plus chaque fois qu'un raccourci change de place.
        let défauts = shortcuts_to_strings(&KeyBindings::default());
        for id in ["run", "step_into", "step_over"] {
            assert_eq!(
                migrated.shortcuts[id], défauts[id],
                "« {id} » doit avoir reçu le raccourci du jeu par défaut"
            );
        }

        // Et ils se relisent bien comme des séquences.
        let bindings = migrated.keybindings();
        assert!(
            matches!(bindings.map.get(&Action::StepInto), Some(Chord::Leader(_))),
            "le pas détaillé est une séquence : {:?}",
            bindings.map.get(&Action::StepInto)
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
