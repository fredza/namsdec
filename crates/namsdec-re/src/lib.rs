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
pub mod xrefs;
mod xrefs_module;
mod yara_module;

pub use binary_info::BinaryInfoModule;
pub use call_graph_module::CallGraphModule;
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
    /// Montrer cette adresse dans le listing.
    ///
    /// Un panneau d'analyse désigne des endroits — une entrée de
    /// liaison, une zone libre, une figure, un fragment —, et la seule
    /// chose qu'on veuille en faire est d'aller voir. Sans ce canal, il
    /// fallait relever l'adresse à la main et la retaper dans la
    /// commande « aller à ».
    GoTo { addr: u64 },
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
}

/// Une adresse sur laquelle on peut cliquer pour l'aller voir.
///
/// Écrite une fois pour tous les panneaux : une adresse se montre de la
/// même façon partout, et un clic y fait la même chose. Rend `true`
/// quand on a cliqué ; au module de poser alors sa demande.
///
/// Le curseur change au survol et une infobulle le dit : sans cela,
/// rien ne distingue une adresse qui mène quelque part d'une adresse
/// qu'on ne fait que lire — et personne ne clique sur ce qui n'a pas
/// l'air cliquable.
pub fn clickable_address(ui: &mut egui::Ui, ctx: &ReContext, addr: u64) -> bool {
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
    ui.add(egui::Label::new(texte).sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(ctx.t(
            "Aller à cette adresse dans le listing",
            "Go to this address in the listing",
        ))
        .clicked()
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

    /// Identifiant et nom de colonne de chaque module, dans l'ordre.
    ///
    /// Comme [`Self::entries`], mais avec le nom court là où le titre
    /// entier ne tient pas dans la colonne — voir
    /// [`ReModule::nav_title`].
    pub fn nav_entries(&self, lang: &str) -> Vec<(&'static str, &'static str)> {
        self.modules
            .iter()
            .map(|m| (m.id(), m.nav_title(lang)))
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
