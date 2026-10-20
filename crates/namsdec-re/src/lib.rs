//! Point d'extension pour les fonctions de reverse engineering : chaque
//! analyse (CFG, xrefs, détection de fonctions, recherche de patterns...)
//! s'implémente comme un `ReModule` greffé au registre, avec son propre
//! panneau dans l'interface. Aucun module "métier" n'existe encore en
//! phase 1 — `BinaryInfoModule` sert de preuve de concept pour valider le
//! mécanisme avant la phase 2.

pub mod behaviour;
mod binary_info;
mod call_graph_module;
pub mod cfg;
pub mod cyclic;
pub mod detour_module;
pub mod digest;
mod digest_module;
pub mod dump_module;
pub mod dzf;
mod dzf_decompiler_module;
pub mod elf_scan;
pub mod entropy;
mod entropy_module;
pub mod explain;
pub mod functions;
mod functions_module;
pub mod gadgets;
mod gadgets_module;
mod hex_editor_module;
pub mod imports;
pub(crate) mod imports_module;
pub mod ioc;
pub mod libc;
pub mod literals;
mod map_module;
pub mod misalign;
pub mod packer;
pub mod patch;
pub mod patterns;
mod patterns_module;
mod protections;
pub mod provenance;
pub mod report;
pub mod route;
pub mod signals;
pub mod slack;
mod slack_module;
pub mod strings;
pub mod structure;
pub mod syscalls;
pub mod table;
mod tool_module;
mod upx_module;
pub mod xrefs;
mod xrefs_module;
mod yara_module;

pub use binary_info::BinaryInfoModule;
pub use call_graph_module::CallGraphModule;
pub use detour_module::DetourModule;
pub use digest_module::DigestModule;
pub use dump_module::DumpModule;
pub use dzf_decompiler_module::DzfDecompilerModule;
pub use entropy_module::EntropyModule;
pub use functions_module::FunctionsModule;
pub use gadgets_module::GadgetsModule;
pub use hex_editor_module::HexEditorModule;
pub use imports_module::ImportsModule;
pub use map_module::MapModule;
pub use patterns_module::PatternsModule;
pub use protections::ProtectionsModule;
pub use signals::SignalsModule;
pub use slack_module::SlackModule;
pub use strings::StringsModule;
pub use syscalls::SyscallsModule;
pub use tool_module::ToolModule;
pub use upx_module::UpxModule;
pub use xrefs_module::XrefsModule;
pub use yara_module::YaraModule;

use namsdec_core::Cpu;
use namsdec_disasm::DecodedInstruction;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ce que voit un module à chaque point d'accroche : l'état du CPU simulé,
/// le désassemblage du binaire chargé, et le source NASM d'origine.
pub struct ReContext<'a> {
    pub cpu: &'a Cpu,
    pub disasm: &'a [DecodedInstruction],
    pub source: &'a str,
    /// Le fichier tel qu'il est sur le disque — en-têtes ELF compris.
    ///
    /// L'image chargée en mémoire ne suffit pas à tout : les protections
    /// d'un binaire se lisent dans ses en-têtes de programme et sa table
    /// dynamique, qui ne sont projetées nulle part. Vide quand le
    /// fichier d'origine n'est pas connu (un tampon brut désassemblé,
    /// par exemple).
    pub binary: &'a [u8],
    /// La langue de l'interface, en code ISO (« fr », « en »).
    ///
    /// Contrairement aux messages d'erreur, qu'une crate métier rend en
    /// anglais et que l'interface traduit, ce qu'un module de RE affiche
    /// *est* son contenu : il n'y a personne au-dessus pour le traduire.
    /// L'interface dit donc la langue, et le module écrit dedans.
    pub lang: &'a str,
    /// Les noms que l'utilisateur a donnés à des adresses — ses fonctions
    /// rebaptisées.
    ///
    /// Un désassembleur appelle une fonction sans nom `sub_10450` ; dès
    /// qu'on a compris ce qu'elle fait, on veut l'appeler « vérifie_clé ».
    /// Ce nom vit dans l'application, qui le pose ici pour que chaque
    /// panneau l'emploie — l'arbre d'appels comme la liste des fonctions.
    /// `None` quand personne n'a encore renommé quoi que ce soit, ou dans
    /// un contexte qui n'en porte pas (un test, par exemple).
    ///
    /// C'est la même table que celle du listing et du déroulement — les
    /// « notes » de ce binaire, rangées par adresse et gardées d'une
    /// séance à l'autre. Un nom donné dans l'arbre d'appels se lit donc
    /// aussi dans le listing, et survit à la fermeture.
    pub names: Option<&'a std::collections::BTreeMap<u64, String>>,
}

impl ReContext<'_> {
    /// Le texte de la langue courante.
    pub fn t(&self, fr: &'static str, en: &'static str) -> &'static str {
        if self.lang.starts_with("fr") { fr } else { en }
    }

    /// Le nom d'une adresse : celui que l'utilisateur a donné, sinon le
    /// nom par défaut qu'on propose.
    ///
    /// Écrit une fois pour tous les panneaux : une fonction rebaptisée
    /// porte le même nom partout, sans quoi l'arbre dirait « vérifie_clé »
    /// et la liste des fonctions « sub_10450 » pour la même chose.
    pub fn display_name(&self, addr: u64, défaut: &str) -> String {
        self.names
            .and_then(|m| m.get(&addr))
            .cloned()
            .unwrap_or_else(|| défaut.to_owned())
    }

    /// Le nom donné à la main à cette adresse, s'il y en a un.
    ///
    /// Sert à deux choses : montrer ce nom partout où l'adresse paraît
    /// (une fonction rebaptisée l'est dans tous les panneaux, pas
    /// seulement là où on l'a renommée), et le mettre légèrement en
    /// évidence pour qu'on repère d'un coup d'œil ce qu'on a déjà nommé.
    pub fn user_name(&self, addr: u64) -> Option<&str> {
        self.names.and_then(|m| m.get(&addr)).map(String::as_str)
    }
}

/// Où l'on va voir une adresse.
///
/// Il y a deux façons de regarder le même endroit, et elles ne
/// répondent pas à la même question. Le **code** dit ce que la machine
/// y fera : une instruction, ses opérandes, le saut qu'elle prend. Les
/// **octets** disent ce qui y est écrit : la valeur exacte, les
/// caractères à côté, et l'on peut la changer. Une chaîne, une table,
/// un bourrage n'ont aucun sens dans le premier et se lisent dans le
/// second ; une fonction, l'inverse.
///
/// D'où le choix laissé à qui clique, plutôt qu'une destination décidée
/// pour lui : le panneau ne sait pas ce qu'on cherche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoTarget {
    /// Le listing désassemblé, à la ligne de cette adresse.
    Listing,
    /// L'éditeur binaire, à la position du fichier qui porte cette
    /// adresse.
    Bytes,
}

/// Ce qu'un module demande à l'application, faute de pouvoir le faire
/// lui-même.
///
/// Un module ne connaît ni le disque, ni les fenêtres, ni l'historique
/// des changements : il décrit ce qu'il voudrait, et l'application
/// décide. C'est ce qui permet au vidage d'offrir l'écriture d'octets
/// sans rien savoir des patchs — et à un module futur d'en demander
/// autant sans qu'on lui ouvre l'application en grand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReRequest {
    /// Écrire ces octets à cette position du fichier.
    PatchBytes { offset: usize, bytes: Vec<u8> },
    /// Poser un détour : les DEUX changements d'un coup.
    ///
    /// Pas deux `PatchBytes` à la file, et c'est la raison même de cette
    /// variante : entre les deux, le fichier porterait un `jmp` vers un
    /// corps pas encore écrit — donc vers du bourrage. L'application
    /// les applique sur la même image, et les enregistre sous un même
    /// numéro de groupe pour qu'ils se défassent ensemble.
    ///
    /// Encadré pour ne pas gonfler la taille de l'énumération : un
    /// détour porte ses octets d'avant et d'après, et toutes les
    /// demandes paieraient ce poids.
    PatchDetour { detour: Box<crate::patch::Detour> },
    /// Montrer cette adresse — dans le code, ou dans les octets.
    ///
    /// Un panneau d'analyse désigne des endroits — une entrée de
    /// liaison, une zone libre, une figure, un fragment —, et la seule
    /// chose qu'on veuille en faire est d'aller voir. Sans ce canal, il
    /// fallait relever l'adresse à la main et la retaper dans la
    /// commande « aller à ».
    GoTo { addr: u64, target: GoTarget },
    /// Donner un nom à une adresse — rebaptiser une fonction.
    ///
    /// Un nom vide efface le nom donné et rend son nom par défaut à
    /// l'adresse : renommer, c'est aussi pouvoir se dédire. Le module ne
    /// range rien lui-même ; il décrit, et l'application tient la table.
    Rename { addr: u64, name: String },
    /// Lancer la décompilation (DZF, le décompilateur actif) et en
    /// ramener le résultat. Le module décrit l'intention ; l'application
    /// lance l'outil et lui livre la réponse par un `ReResult`.
    Decompile,
    /// Lancer cet outil extérieur — Ghidra, RetDec, radare2, objdump,
    /// ndisasm — sur le binaire ouvert.
    ///
    /// Le module nomme l'outil et rien de plus : il ne sait pas où est
    /// le programme, ni ce que namsdec a installé lui-même, ni comment
    /// on lance un travail en fond. L'application le sait, et c'est
    /// elle qui décide — comme pour tout ce qui sort du bac à sable.
    RunTool { tool: String },
    /// Lancer UPX sur le binaire ouvert : l'interroger, ou le déballer.
    ///
    /// À part de [`ReRequest::RunTool`], et pas par coquetterie : les
    /// outils de cette liste-là sont des **moteurs de désassemblage**
    /// qu'on peut activer, chacun décrit par une fiche qu'on installe.
    /// UPX n'en est pas un — il ne lit pas, il écrit un fichier —, et
    /// le ranger parmi eux l'aurait fait proposer comme décompilateur
    /// dans Outils › Désassembleurs.
    Upx { action: UpxAction },
}

/// Ce qu'on demande à UPX.
///
/// Deux gestes, et un seul des deux écrit quelque chose : la distinction
/// est dans l'énumération plutôt que dans un booléen, pour qu'on ne
/// puisse pas lancer un déballage en croyant lancer une lecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpxAction {
    /// `upx -l` : ce qu'UPX voit. Rien n'est écrit.
    List,
    /// `upx -d -o <neuf> <copie>` : l'original déballé, dans un fichier
    /// neuf. **Jamais en place** — c'est tout l'écart avec la commande
    /// qu'on tape de mémoire, qui écrase l'échantillon.
    Unpack,
}

/// Ce que l'application rend à un module, en retour de ce qu'il a demandé.
///
/// Symétrique de [`ReRequest`] : là, le module demandait ; ici,
/// l'application répond. Livré à tous les modules — chacun prend ce qui le
/// concerne, le reste l'ignore.
#[derive(Debug, Clone)]
pub enum ReResult {
    /// La décompilation que DZF a rendue, lue et prête à montrer.
    Decompilation(crate::dzf::DzfOutput),
    /// La décompilation a échoué : l'outil absent, le JSON illisible.
    DecompilationError(String),
    /// Ce qu'un outil extérieur a rendu — sa sortie, ou son refus.
    ToolOutput {
        tool: String,
        text: Result<String, String>,
    },
    /// Ce que namsdec sait d'un outil sur cette machine.
    ///
    /// Le panneau ne va pas le chercher lui-même : la question se pose
    /// en parcourant le PATH et parfois en lançant le programme, ce qui
    /// ne se fait pas soixante fois par seconde. L'application le relève
    /// une fois et le dit.
    ToolState {
        tool: String,
        availability: namsdec_disasm::known::Availability,
    },
    /// Montrer cette position du fichier, et dire d'où l'on vient.
    ///
    /// La réponse à un `GoTo { target: Bytes }` : l'application a
    /// converti l'adresse en position — elle seule sait lire les
    /// en-têtes du fichier chargé — et le dit à l'éditeur binaire, qui
    /// s'y rend. L'adresse est gardée pour qu'il puisse l'écrire : on
    /// vient de cliquer dessus, et la retrouver à l'arrivée est ce qui
    /// dit qu'on est au bon endroit.
    ShowBytesAt { offset: usize, addr: Option<u64> },
    /// Ce qu'UPX a répondu — sa sortie, ou son refus.
    ///
    /// `unpacked` porte le fichier écrit quand `upx -d` a réussi :
    /// le panneau ne le devine pas, puisque c'est l'application qui a
    /// demandé où l'écrire.
    UpxOutput {
        action: UpxAction,
        text: Result<String, String>,
        unpacked: Option<std::path::PathBuf>,
    },
}

/// Souligne un texte quand la souris est dessus — le signe qu'il se
/// clique.
///
/// LA QUESTION À LAQUELLE CECI RÉPOND est « comment sait-on qu'un texte
/// se clique ? ». La main du pointeur le dit, mais seulement quand on
/// est déjà dessus, et sur un texte elle se confond avec le curseur de
/// sélection. Le soulignement est le signe que tout le monde connaît,
/// et il ne coûte rien au repos : le panneau reste calme, et ce qu'on
/// survole se déclare.
///
/// Peint APRÈS coup, sous le rectangle de la réponse, plutôt que posé
/// dans le `RichText` : on ne sait qu'un texte est survolé qu'une fois
/// qu'il est dessiné, et le relire de l'image précédente ferait
/// clignoter le trait d'une image sur deux.
///
/// Le même geste existe côté interface (`souligne_au_survol` dans
/// `main.rs`) : les panneaux d'analyse ne dépendent pas de
/// l'application, et une grammaire commune vaut mieux que deux façons
/// de dire la même chose.
pub fn souligne_au_survol(ui: &egui::Ui, réponse: &egui::Response) {
    if !réponse.hovered() {
        return;
    }
    let couleur = ui.visuals().widgets.hovered.fg_stroke.color;
    ui.painter().hline(
        réponse.rect.x_range(),
        réponse.rect.bottom() - 1.0,
        egui::Stroke::new(1.0, couleur),
    );
}

/// Un texte sur lequel on peut cliquer : la main au survol, et le trait
/// dessous.
///
/// Écrit une fois pour tous les panneaux. Un contrôle lit les sources
/// et refuse qu'une étiquette devienne cliquable ailleurs qu'ici : une
/// seule grammaire, et rien qui se clique sans le dire.
pub fn clickable_label(ui: &mut egui::Ui, étiquette: egui::Label) -> egui::Response {
    let réponse = ui
        .add(étiquette.sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    souligne_au_survol(ui, &réponse);
    réponse
}

/// Un bouton qui se voit **sans qu'on le survole**.
///
/// Le thème rend les boutons plats au repos — fond transparent, liseré
/// absent — et c'est voulu dans la barre de menus et la barre d'outils,
/// où la platitude EST le dessin. Partout ailleurs, un bouton plat ne se
/// distingue d'une étiquette qu'au survol : on ne sait pas qu'il y a
/// quelque chose à cliquer, donc on ne clique pas.
///
/// L'interface a son `framed_button` depuis longtemps, mais il est
/// interne à `namsdec-gui` et les panneaux d'analyse vivent ici : leurs
/// trente-trois boutons étaient donc plats, dans la même fenêtre où ceux
/// de l'interface portent un liseré. Vu à l'écran le 15 octobre 2026,
/// pendant l'audit du panneau « Détourner ».
///
/// Le liseré se prend dans les `visuals` et non dans le thème : un
/// panneau n'a pas à connaître la palette de l'application, et la
/// couleur suit ainsi le thème clair comme le sombre.
pub fn framed(ui: &egui::Ui, label: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(label)
        .stroke(egui::Stroke::new(1.0, ui.visuals().weak_text_color()))
        .corner_radius(egui::CornerRadius::from(6))
}

/// Le même, posé tout de suite.
///
/// `framed` le rend sans le poser, pour `add_enabled` — qui grise un
/// bouton tant qu'il n'y a rien à faire, et a donc besoin de l'ajouter
/// lui-même.
pub fn framed_button(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>) -> egui::Response {
    let bouton = framed(ui, label);
    ui.add(bouton)
}

/// Le même, en petit — pour une rangée serrée.
pub fn framed_small_button(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    let bouton = framed(ui, label).small();
    ui.add(bouton)
}

/// Une adresse sur laquelle on peut cliquer pour l'aller voir.
///
/// Écrite une fois pour tous les panneaux : une adresse se montre de la
/// même façon partout, et un clic y fait la même chose. Rend la
/// destination choisie ; au module de poser alors sa demande.
///
/// LE CLIC DEMANDE OÙ. Il emmenait au listing sans rien demander, ce
/// qui est la bonne réponse pour une fonction et la mauvaise pour une
/// chaîne, une table ou un bourrage — on voulait voir les octets, on
/// atterrissait sur un désassemblage de données. Deux entrées, et l'on
/// choisit : c'est une question par clic, mais c'est la question qu'on
/// se posait déjà.
///
/// Le curseur change au survol et une infobulle le dit : sans cela,
/// rien ne distingue une adresse qui mène quelque part d'une adresse
/// qu'on ne fait que lire — et personne ne clique sur ce qui n'a pas
/// l'air cliquable.
pub fn clickable_address(ui: &mut egui::Ui, ctx: &ReContext, addr: u64) -> Option<GoTarget> {
    // Un nom donné à la main suit l'adresse PARTOUT où elle paraît, et se
    // mange un léger fond en évidence : on repère ce qu'on a déjà nommé
    // sans avoir à rouvrir la liste des fonctions.
    let texte = match ctx.user_name(addr) {
        Some(nom) => egui::RichText::new(format!("{nom}  {addr:#010x}"))
            .monospace()
            .color(ui.visuals().hyperlink_color)
            .background_color(ui.visuals().selection.bg_fill.gamma_multiply(0.25)),
        None => egui::RichText::new(format!("{addr:#010x}"))
            .monospace()
            .color(ui.visuals().hyperlink_color),
    };
    let réponse = clickable_label(ui, egui::Label::new(texte)).on_hover_text(ctx.t(
        "Aller voir cette adresse — dans le code, ou dans les octets",
        "Go and see this address — in the code, or in the bytes",
    ));
    let mut choix = None;
    egui::Popup::menu(&réponse).show(|ui| {
        if framed_button(ui, ctx.t("Dans le code désassemblé", "In the disassembly")).clicked() {
            choix = Some(GoTarget::Listing);
            ui.close();
        }
        if framed_button(ui, ctx.t("Dans l'éditeur binaire", "In the binary editor")).clicked() {
            choix = Some(GoTarget::Bytes);
            ui.close();
        }
    });
    choix
}

/// La famille à laquelle un panneau appartient.
///
/// Vingt-quatre rubriques à plat dans une colonne ne se cherchent pas :
/// on les parcourt une à une, et l'on finit par ne plus voir celles du
/// bas. Rangées par famille, on sait où regarder avant d'avoir lu — et
/// une rubrique qu'on n'a jamais ouverte se trouve tout de même, parce
/// qu'on sait de quelle question elle relève.
///
/// L'ordre des variantes est celui de la colonne, et c'est aussi celui
/// dans lequel on lit un binaire inconnu : ce que le fichier dit de
/// lui-même, puis son code, puis ce qu'on peut en rendre lisible, puis
/// ce qu'il dit et ce qu'il demande, puis les octets eux-mêmes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReSection {
    /// Ce que le fichier dit de lui-même : son identité, sa carte, ses
    /// protections, sa densité.
    File,
    /// Le code : où il est, comment il s'enchaîne, ce qu'il fait de
    /// remarquable.
    Code,
    /// Le rendre lisible : le pseudo-code, et les outils des autres.
    Decompile,
    /// Ce qu'il dit : ses chaînes, et la règle qu'on en tire.
    Says,
    /// Ce qu'il demande au dehors : appels système et fonctions
    /// empruntées.
    Borrows,
    /// Les octets eux-mêmes — les lire, les retrouver, les changer.
    Bytes,
}

impl ReSection {
    /// Le nom de la section, dans la langue demandée.
    ///
    /// Court : c'est un intertitre dans une colonne de deux cents
    /// pixels, et un intertitre qui passe sur deux lignes coûte la
    /// place d'une rubrique.
    pub fn label(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match (self, fr) {
            (ReSection::File, true) => "Le fichier",
            (ReSection::File, false) => "The file",
            (ReSection::Code, true) => "Le code",
            (ReSection::Code, false) => "The code",
            (ReSection::Decompile, true) => "Décompiler",
            (ReSection::Decompile, false) => "Decompile",
            (ReSection::Says, true) => "Ce qu'il dit",
            (ReSection::Says, false) => "What it says",
            (ReSection::Borrows, true) => "Ce qu'il emprunte",
            (ReSection::Borrows, false) => "What it borrows",
            (ReSection::Bytes, true) => "Les octets",
            (ReSection::Bytes, false) => "The bytes",
        }
    }
}

pub trait ReModule {
    /// Identifiant stable (pour retrouver/désactiver un module).
    fn id(&self) -> &'static str;

    /// Relève ce que le module demande, et l'oublie.
    ///
    /// Relevé et non agi : le module se dessine pendant que
    /// l'application tient déjà ses propres emprunts, et ne peut donc
    /// rien lui faire faire sur-le-champ. Rien à demander, dans le cas
    /// général — d'où la réponse par défaut.
    fn take_request(&mut self) -> Option<ReRequest> {
        None
    }
    /// Titre affiché en tête du panneau du module, dans la langue
    /// demandée.
    fn title(&self, lang: &str) -> &'static str;

    /// La famille de ce panneau : c'est elle qui le range dans la
    /// colonne.
    ///
    /// Déclarée par le module et non par l'interface : c'est le panneau
    /// qui sait de quelle question il relève, et un panneau nouveau
    /// doit choisir sa place plutôt que d'échouer en bas d'une liste.
    fn section(&self) -> ReSection;

    /// Le nom de la rubrique dans la colonne de navigation, quand il
    /// doit être plus court que le titre.
    ///
    /// Une colonne de deux cents pixels qui porte vingt-cinq entrées
    /// n'a pas la place d'écrire « Décompilation par RetDec » :
    /// l'entrée passe sur deux lignes, cinq entrées de ce genre
    /// repoussent les dernières sous le bord, et l'on cherche à
    /// l'ascenseur une rubrique dont on ignore l'existence. La colonne
    /// écrit donc « RetDec », et le panneau garde son titre entier.
    ///
    /// Le même que le titre, dans le cas général.
    fn nav_title(&self, lang: &str) -> &'static str {
        self.title(lang)
    }

    /// Appelé après un (ré)assemblage réussi, avant la première exécution.
    fn on_binary_loaded(&mut self, _ctx: &ReContext) {}

    /// Appelé après chaque `step()`/`run()` de l'émulateur.
    fn on_step(&mut self, _ctx: &ReContext) {}

    /// Reçoit une réponse de l'application à ce qu'on a demandé. Sans
    /// objet pour la plupart des modules — d'où la version vide.
    fn deliver(&mut self, _result: &ReResult) {}

    /// Rendu du panneau du module.
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext);
}

#[derive(Default)]
pub struct ReModuleRegistry {
    modules: Vec<Box<dyn ReModule>>,
}

impl ReModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Porte une réponse de l'application à tous les modules : chacun y
    /// prend ce qui le concerne.
    pub fn deliver(&mut self, result: &ReResult) {
        for module in &mut self.modules {
            module.deliver(result);
        }
    }

    pub fn register(&mut self, module: Box<dyn ReModule>) {
        self.modules.push(module);
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn ReModule>> {
        self.modules.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Titres des modules enregistrés, dans l'ordre d'enregistrement —
    /// utilisé par la vue d'ensemble de l'interface graphique.
    pub fn titles(&self, lang: &str) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.title(lang)).collect()
    }

    /// Identifiant et titre de chaque module, dans l'ordre.
    ///
    /// L'identifiant ne change pas avec la langue, le titre si : une
    /// colonne de navigation a besoin des deux — l'un pour retenir ce
    /// qu'on a choisi, l'autre pour l'écrire.
    pub fn entries(&self, lang: &str) -> Vec<(&'static str, &'static str)> {
        self.modules
            .iter()
            .map(|m| (m.id(), m.title(lang)))
            .collect()
    }

    /// Identifiant, nom de colonne et famille de chaque module, dans
    /// l'ordre.
    ///
    /// Comme [`Self::entries`], mais avec le nom court là où le titre
    /// entier ne tient pas dans la colonne — voir [`ReModule::nav_title`]
    /// — et la section qui le range, voir [`ReSection`].
    pub fn nav_entries(&self, lang: &str) -> Vec<(&'static str, &'static str, ReSection)> {
        self.modules
            .iter()
            .map(|m| (m.id(), m.nav_title(lang), m.section()))
            .collect()
    }

    pub fn on_binary_loaded(&mut self, ctx: &ReContext) {
        for module in &mut self.modules {
            module.on_binary_loaded(ctx);
        }
    }

    pub fn on_step(&mut self, ctx: &ReContext) {
        for module in &mut self.modules {
            module.on_step(ctx);
        }
    }
}
