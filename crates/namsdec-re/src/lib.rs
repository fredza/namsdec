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
pub mod comparaison;
pub mod cyclic;
pub mod detour_module;
/// L'empreinte d'un fichier, vécue ici et **écrite dans le socle**.
///
/// Elle y est montée le 27 novembre 2026, le jour où la mise à jour en
/// a eu besoin : `namsdec-update` doit vérifier la somme d'un binaire
/// téléchargé, et il ne peut pas dépendre de tout un moteur d'analyse
/// pour soixante lignes de SHA-256. Une seconde copie aurait divergé —
/// c'est toujours ainsi que deux empreintes finissent par ne plus
/// s'accorder.
///
/// Réexporté, et non déplacé à la vue de tous : `namsdec_re::digest`
/// reste le chemin que les modules d'analyse écrivent depuis le début.
pub use namsdec_core::digest;
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
pub mod indirect;
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
pub mod rapport;
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
pub mod toolchain;
mod upx_module;
pub mod vue;
pub mod vue_peinte;
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
// LA SEULE PART DE L'ÉDITEUR BINAIRE QUI PASSE LA FRONTIÈRE : ce que
// l'octet choisi vaut, lu dans chaque format. Le reste du panneau est
// un instrument — une grille, un curseur, vingt-six touches — et reste
// peint sur place ; voir le relevé dans `TRAVAUX.md`.
pub use hex_editor_module::vue_de_l_inspecteur;
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
    /// Les correctifs déjà posés sur ce binaire, dans l'ordre où ils
    /// l'ont été.
    ///
    /// `binary` est l'image CORRIGÉE — chaque écriture recharge l'ELF —,
    /// mais des octets ne disent pas d'où ils viennent : un `jmp` posé
    /// par un détour se décode comme n'importe quel `jmp`. Un panneau
    /// qui s'apprête à écrire a besoin de savoir ce qui est déjà écrit,
    /// faute de quoi il passe par-dessus le travail fait, et deux
    /// correctifs qui se chevauchent ne se défont plus dans un ordre
    /// unique.
    ///
    /// Vide quand rien n'a été posé, ce qui est le cas d'un fichier
    /// qu'on vient d'ouvrir — et le cas d'un test qui ne s'en occupe pas.
    pub patches: &'a [patch::Patch],
}

impl ReContext<'_> {
    /// Le texte de la langue courante.
    pub fn t(&self, fr: &'static str, en: &'static str) -> &'static str {
        if self.lang.starts_with("fr") { fr } else { en }
    }

    /// Un compte et son nom ACCORDÉ : « 1 fonction », « 12 fonctions ».
    ///
    /// Écrit une fois, parce que le cas à UN est celui qu'on ne voit
    /// jamais en développant — on essaie toujours sur un binaire qui en
    /// a douze. Les captures, elles, l'ont montré quatre fois : « 1
    /// fonctions retrouvées », « 1 boucles », « 1 étiquettes », « 1
    /// adresses de données ». Un compte qui ne s'accorde pas est la
    /// faute la plus visible d'une interface française.
    ///
    /// Les deux langues séparément : l'anglais accorde le nom et pas
    /// l'adjectif, le français les deux.
    pub fn compte(
        &self,
        n: usize,
        fr: (&'static str, &'static str),
        en: (&'static str, &'static str),
    ) -> String {
        let (un, plusieurs) = if self.lang.starts_with("fr") { fr } else { en };
        format!("{n} {}", if n == 1 { un } else { plusieurs })
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
    /// Écrire ces octets à cette position du fichier, à la place des
    /// `retirés` qui s'y trouvent.
    ///
    /// `retirés == bytes.len()` est le cas sûr : le fichier garde sa
    /// taille, et rien ne bouge ailleurs. Les deux autres — insérer
    /// (`retirés == 0`), supprimer (`bytes` vide) — DÉCALENT tout ce
    /// qui suit, et l'application le dit avant de les faire. Voir
    /// [`patch::splice`].
    PatchBytes {
        offset: usize,
        retirés: usize,
        bytes: Vec<u8>,
    },
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

/// Souligne une RANGÉE quand la souris est dessus — le signe qu'elle se
/// clique.
///
/// Réservé aux rangées entières d'une liste : une ligne de
/// désassemblage, une zone de la carte mémoire, une entrée d'index.
/// Pour un TEXTE au milieu d'une phrase, c'est [`clickable_label`] qu'il
/// faut, et son trait se voit au repos — un trait sous chacune des trois
/// cents lignes d'une liste ne serait plus un signe, seulement du bruit.
///
/// LA QUESTION À LAQUELLE CECI RÉPOND est « comment sait-on qu'un texte
/// se clique ? ». La main du pointeur le dit, mais seulement quand on
/// est déjà dessus, et sur un texte elle se confond avec le curseur de
/// sélection. Le soulignement est le signe que tout le monde connaît.
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

/// Un texte sur lequel on peut cliquer : le trait dessous — discret au
/// repos, franc au survol — et la main du pointeur.
///
/// Écrit une fois pour tous les panneaux. Un contrôle lit les sources
/// et refuse qu'une étiquette devienne cliquable ailleurs qu'ici : une
/// seule grammaire, et rien qui se clique sans le dire.
pub fn clickable_label(ui: &mut egui::Ui, étiquette: egui::Label) -> egui::Response {
    let réponse = ui
        .add(étiquette.sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    // AU REPOS AUSSI, et c'est la correction du 20 octobre.
    //
    // Le trait ne paraissait qu'au survol, au nom d'un panneau calme.
    // Mais un signe qui n'apparaît qu'une fois qu'on est dessus ne sert
    // qu'à celui qui savait déjà : on ne survole pas les textes au
    // hasard pour voir lesquels se cliquent. Signalé à l'écran — « les
    // textes où une action est attendue ne l'indiquent pas ».
    //
    // Le calme est gardé autrement : au repos le trait est à un tiers
    // d'opacité, assez pour se voir en parcourant une colonne, trop peu
    // pour faire du bruit. Au survol il passe plein.
    let couleur = if réponse.hovered() {
        ui.visuals().widgets.hovered.fg_stroke.color
    } else {
        ui.visuals().hyperlink_color.gamma_multiply(0.35)
    };
    ui.painter().hline(
        réponse.rect.x_range(),
        réponse.rect.bottom() - 1.0,
        egui::Stroke::new(1.0, couleur),
    );
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
        .stroke(egui::Stroke::new(
            FILET_DU_BOUTON,
            ui.visuals().weak_text_color(),
        ))
        .corner_radius(egui::CornerRadius::from(6))
}

/// L'épaisseur du liseré d'un bouton encadré.
///
/// Nommée parce que deux endroits en dépendent : le bouton qui le
/// trace, et `hauteur_d_un_bouton_encadré` qui doit le compter.
pub const FILET_DU_BOUTON: f32 = 1.0;

/// LA HAUTEUR QU'UN BOUTON ENCADRÉ PREND DANS UNE RANGÉE.
///
/// Elle n'est pas « le texte plus les marges » : depuis egui 0.37, un
/// bouton à qui l'on pose un liseré se RÉSERVE aussi la place de ce
/// liseré — un pixel en haut, un en bas. Ce qui est posé à côté de lui
/// sur la rangée — un champ de saisie, un bouton plat — ne porte pas
/// de liseré et sortait donc deux pixels plus court. Douze rangées de
/// l'onglet RE se lisaient de travers : 25,1 px contre 27, à huit
/// pixels l'un de l'autre.
///
/// Les voisins visent donc cette hauteur-ci, par `min_size`. Elle se
/// mesure — police, marges du thème et liseré — plutôt que de
/// s'écrire à la main : les trois changent, et un nombre écrit ici ne
/// survivrait pas au premier qui bouge.
pub fn hauteur_d_un_bouton_encadré(ui: &egui::Ui) -> f32 {
    let texte = ui.text_style_height(&egui::TextStyle::Body);
    ui.spacing()
        .interact_size
        .y
        .max(texte + 2.0 * ui.spacing().button_padding.y + 2.0 * FILET_DU_BOUTON)
}

/// RELÈVE LE PLANCHER D'UNE RANGÉE À LA HAUTEUR D'UN BOUTON ENCADRÉ.
///
/// À appeler en tête d'une rangée qui mêle un bouton encadré à autre
/// chose — un bouton plat, une case à cocher, une étiquette
/// sélectionnable, un champ de saisie. egui 0.37 leur donne trois
/// hauteurs différentes : seul le bouton encadré se réserve la place
/// de son liseré, et un champ en chasse fixe compose sur une ligne
/// plus courte qu'un texte ordinaire. Mesuré dans l'onglet RE : 25,0
/// px, 25,1 px et 27,0 px côte à côte, à huit pixels l'un de l'autre.
///
/// Un plancher relevé une fois valant mieux qu'un `min_size` repris
/// sur chaque contrôle : ce qu'on ajoutera demain dans la rangée
/// s'alignera sans qu'on y pense.
pub fn aligne_la_rangée(ui: &mut egui::Ui) {
    let hauteur = hauteur_d_un_bouton_encadré(ui);
    ui.spacing_mut().interact_size.y = hauteur;
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
        // LES DEUX ENTRÉES DISENT OÙ ELLES MÈNENT.
        //
        // Elles disaient « dans le code désassemblé » et « dans
        // l'éditeur binaire », ce qui nomme une VUE et pas un endroit :
        // « mais où ? » (signalé à l'écran le 20 octobre 2026). Les deux
        // destinations ne sont pas au même endroit du tout — l'une est
        // derrière cette fenêtre, qui s'efface pour la laisser voir ;
        // l'autre est une rubrique de cette fenêtre-ci, qui reste
        // ouverte. C'est cela qu'il faut lire avant de cliquer.
        if framed_button(
            ui,
            ctx.t(
                "Dans le listing — cette fenêtre s'efface",
                "In the listing — this window steps aside",
            ),
        )
        .on_hover_text(ctx.t(
            "Le listing défile et centre la ligne derrière cette fenêtre, qui se \
             referme pour la laisser voir. « Analyse du binaire… » la rouvre à la \
             même rubrique.",
            "The listing scrolls and centres the line behind this window, which \
             closes to let you see it. “Analyse the binary…” reopens it at the same \
             section.",
        ))
        .clicked()
        {
            choix = Some(GoTarget::Listing);
            ui.close();
        }
        if framed_button(
            ui,
            ctx.t(
                "Dans les octets — rubrique « Éditeur binaire », ici même",
                "In the bytes — “Binary editor” section, right here",
            ),
        )
        .on_hover_text(ctx.t(
            "Cette fenêtre reste ouverte et passe à sa rubrique « Éditeur binaire », \
             positionnée sur cette adresse. C'est la bonne vue pour une chaîne, une \
             table ou du bourrage — tout ce qu'un désassembleur lit comme des \
             instructions faute de savoir faire autrement.",
            "This window stays open and switches to its “Binary editor” section, \
             placed at this address. That is the right view for a string, a table or \
             padding — anything a disassembler reads as instructions for lack of \
             anything better to do.",
        ))
        .clicked()
        {
            choix = Some(GoTarget::Bytes);
            ui.close();
        }
    });
    choix
}

/// À partir de quel niveau un panneau paraît.
///
/// **namsdec sert à apprendre l'assembleur et la rétro-ingénierie**, et
/// vingt-cinq rubriques devant quelqu'un qui ouvre son premier binaire
/// ne l'aident pas : elles lui demandent de savoir ce qu'il cherche
/// avant d'avoir appris à chercher. Le niveau « débutant » ne retire
/// rien de l'outil — il met de côté ce qui ne se comprend qu'une fois
/// les bases en place, et une case le rend d'un clic.
///
/// DÉCLARÉ PAR LE PANNEAU, comme sa famille ([`ReSection`]) : c'est lui
/// qui sait ce qu'il demande de savoir. Un panneau neuf est « avancé »
/// par défaut, et doit se déclarer autrement pour paraître d'emblée —
/// le silence range du bon côté.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Niveau {
    /// Ce qu'on regarde pour apprendre : où est le code, ce qu'il
    /// emprunte, ce qu'il dit, et les octets eux-mêmes.
    Débutant,
    /// Ce qui suppose qu'on sache déjà ce qu'on cherche.
    #[default]
    Avancé,
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

    /// À partir de quel niveau ce panneau paraît.
    ///
    /// « Avancé » par défaut, et c'est voulu : un panneau neuf ne
    /// s'invite pas devant quelqu'un qui apprend sans qu'on l'ait
    /// décidé. Voir [`Niveau`].
    fn niveau(&self) -> Niveau {
        Niveau::Avancé
    }

    /// Ce module veut-il toute la largeur du panneau, ou la largeur de
    /// LECTURE ?
    ///
    /// La fenêtre d'analyse borne son contenu à mille pixels, et c'est
    /// juste : au-delà, une table à deux colonnes pose la seconde si
    /// loin de la première qu'on suit la ligne du doigt, et une phrase
    /// se perd en revenant à la gauche.
    ///
    /// Une GRILLE ne lit pas comme une phrase. L'éditeur binaire gagne
    /// à chaque pixel — plus d'octets par ligne, un motif qui se répète
    /// toutes les trente-deux positions qui se voit d'un coup d'œil, et
    /// la place de poser l'inspecteur à côté du vidage plutôt que sous
    /// lui. La borne de lecture lui coûtait le quart droit du panneau,
    /// même fenêtre agrandie.
    fn pleine_largeur(&self) -> bool {
        false
    }

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

    /// Ce que le module a à montrer, **sans savoir comment on le
    /// peint**.
    ///
    /// C'est la sortie qui permet à ce que namsdec comprend d'un
    /// binaire d'exister ailleurs qu'à l'écran : un rapport, un export,
    /// une autre interface, un contrôle qui lise du texte plutôt que
    /// des pixels. Voir [`crate::vue`] pour le vocabulaire, et le lot
    /// P7 de `TRAVAUX.md` pour la mesure dont il est tiré.
    ///
    /// `None` par défaut, et c'est voulu : les modules y passent
    /// groupe par groupe, et un module qui n'a pas encore sa vue
    /// continue de dessiner comme avant. Un module qui la rend DOIT en
    /// dériver son `ui()` — sinon il y aurait deux descriptions du même
    /// panneau, et elles divergeraient.
    fn vue(&self, _ctx: &ReContext) -> Option<crate::vue::Vue> {
        None
    }

    /// Ce panneau a-t-il sa place dans un rapport ?
    ///
    /// Oui pour tout ce qui parle du BINAIRE, et c'est la règle.
    /// Non pour ce qui parle de la MACHINE : les cinq panneaux
    /// d'outils extérieurs disent « radare2 n'est pas installé ici »,
    /// ce qui est utile à l'écran et n'apprend rien à qui lira le
    /// rapport ailleurs. Dès qu'un outil a répondu, en revanche, sa
    /// sortie est de l'analyse comme une autre.
    fn dans_un_rapport(&self, _ctx: &ReContext) -> bool {
        true
    }

    /// Rendu du panneau du module.
    ///
    /// Pour un module qui rend une [`ReModule::vue`], ce n'est qu'une
    /// ligne — voir `digest_module`.
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

    /// Les vingt-deux panneaux, dans l'ordre où on les lit.
    ///
    /// **CET ORDRE EST UNE PROPRIÉTÉ DE L'ANALYSE, et c'est pour cela
    /// qu'il vit ici.** Il était écrit dans `NamsdecApp::default`,
    /// c'est-à-dire dans l'interface : un rapport, un export, une
    /// analyse sans écran auraient dû le recopier, et deux ordres qui
    /// se recopient divergent. Le raisonnement de chaque rang est
    /// gardé ligne à ligne — c'est la partie qui ne se devine pas.
    pub fn standard() -> Self {
        let mut re_registry = Self::new();
        // Le résumé d'abord : de quoi on parle. Puis les protections —
        // avant de lire ce que fait un binaire, on regarde comment il
        // est armé, car c'est ce qui décide de tout ce qu'on tentera
        // ensuite.
        re_registry.register(Box::new(crate::BinaryInfoModule::default()));
        // Puis les signaux : ce que tous les autres ont trouvé, rassemblé.
        // Il n'analyse rien de neuf — il dit par où commencer, et renvoie
        // chaque fois au panneau qui explique. C'est le tableau de bord
        // qu'on lit en premier, et il est donc en second : d'abord de
        // quoi on parle, ensuite ce qui en ressort.
        re_registry.register(Box::new(crate::SignalsModule::default()));
        // Puis l'identité du fichier : de quel octet parle-t-on ? C'est
        // la première ligne d'un compte rendu, et ce qui permet à deux
        // personnes de savoir qu'elles lisent le même fichier.
        re_registry.register(Box::new(crate::DigestModule::default()));
        // Puis « où est quoi » : un binaire n'est pas un bloc. Savoir à
        // quel endroit on regarde change ce qu'on y cherche — une
        // entropie haute dans `.text` est normale, la même dans
        // `.rodata` annonce des données chiffrées.
        re_registry.register(Box::new(crate::MapModule::default()));
        re_registry.register(Box::new(crate::ProtectionsModule::default()));
        // L'entropie juste après : elle répond avant tout le reste à
        // « est-ce qu'il y a quelque chose à lire ici ? ». Sur un binaire
        // empaqueté, les modules suivants analyseraient un déchiffreur de
        // trois cents octets en croyant analyser le programme — autant le
        // savoir tout de suite.
        re_registry.register(Box::new(crate::EntropyModule::default()));
        // Puis UPX, tout de suite après : c'est l'entropie qui pose la
        // question — « y a-t-il seulement le vrai programme ici ? » —,
        // et sur le plus répandu des empaqueteurs la réponse est à un
        // clic. Avant les modules de code, parce qu'un binaire
        // empaqueté les ferait analyser un déballeur de trois cents
        // octets en croyant analyser le programme : il n'y a rien à
        // décoder tant qu'on n'a pas déballé.
        re_registry.register(Box::new(crate::UpxModule::default()));
        // Puis « par où commencer » : dans un binaire dépouillé, la
        // question précède toutes les autres. Le module retrouve le
        // `main` et découpe le code en fonctions.
        re_registry.register(Box::new(crate::FunctionsModule::default()));
        // Juste après : comment ces fonctions s'appellent. Le module
        // « Fonctions » les dénombre ; celui-ci montre leur structure —
        // l'arbre des appels depuis le point d'entrée, à la façon d'un
        // diagramme UML —, laisse aller à une adresse, et déplie le code
        // binaire et le pseudo-code de chacune. C'est la carte qu'on lit
        // avant de descendre dans une fonction.
        re_registry.register(Box::new(crate::CallGraphModule::default()));
        // Ce que le code *fait* de remarquable : les figures, rangées de
        // la plus rare à la plus commune. C'est ce classement qui fait
        // sortir une comparaison déguisée d'un binaire de soixante mille
        // instructions.
        re_registry.register(Box::new(crate::PatternsModule::default()));
        // Et par où entrer dedans : qui appelle quoi, qui saute où. Les
        // références croisées ferment la famille du code — elles se
        // lisent une fois qu'on sait ce que le binaire contient.
        re_registry.register(Box::new(crate::XrefsModule::default()));
        // La décompilation DZF : les fonctions en pseudo-C, quand le
        // moteur maison les a rendues. Après le graphe d'appels, dont elle
        // prolonge la lecture — du squelette des appels au corps en clair.
        re_registry.register(Box::new(crate::DzfDecompilerModule::default()));
        // Puis les outils des autres, un panneau chacun. La fenêtre
        // « Décompilation » les propose déjà dans une liste déroulante,
        // mais une liste déroulante oblige à relancer pour comparer, et
        // perd la sortie précédente : ici, chacun garde la sienne, et
        // l'on passe de Ghidra à RetDec comme on change de page.
        //
        // Dans l'ordre de ce qu'ils rendent : du plus haut — du C qu'on
        // lit presque comme un source — au plus bas, les instructions
        // brutes. Aucun ne se lance tout seul ; chacun dit, avant,
        // la commande qu'il exécuterait.
        for (id, fr, en, court) in [
            (
                "rizin-ghidra",
                "Décompilation par Ghidra",
                "Decompilation by Ghidra",
                "Ghidra",
            ),
            (
                "retdec",
                "Décompilation par RetDec",
                "Decompilation by RetDec",
                "RetDec",
            ),
            (
                "radare2",
                "Décompilation par radare2",
                "Decompilation by radare2",
                "radare2",
            ),
            (
                "objdump",
                "Désassemblage par objdump",
                "Disassembly by objdump",
                "objdump",
            ),
            (
                "ndisasm",
                "Désassemblage par ndisasm",
                "Disassembly by ndisasm",
                "ndisasm",
            ),
        ] {
            // « Décompilation » pour les trois premiers, « Désassemblage »
            // pour les deux derniers, et ce n'est pas une coquetterie :
            // objdump et ndisasm rendent des instructions, non du C.
            // Les ranger sous le même mot ferait attendre d'eux ce
            // qu'ils ne savent pas faire.
            if let Some(module) = crate::ToolModule::connu(id, fr, en, court) {
                re_registry.register(Box::new(module));
            }
        }
        // Ce qu'il DIT : ses chaînes, et la règle qu'on en tire. Deux
        // rubriques qui vont ensemble — la seconde se nourrit de la
        // première.
        re_registry.register(Box::new(crate::StringsModule::default()));
        // Le geste qui suit l'analyse : une règle YARA pour retrouver les
        // frères de cet échantillon. Juste après les chaînes, car c'est
        // d'elles qu'elle tire ses motifs.
        re_registry.register(Box::new(crate::YaraModule::default()));
        // Puis ce qu'il DEMANDE au dehors. Les appels système d'abord —
        // ce qu'il demande au noyau, sans intermédiaire.
        re_registry.register(Box::new(crate::SyscallsModule::default()));
        // Puis les fonctions d'autrui qu'il appelle : la même question
        // posée un cran plus haut — que demande-t-il à ce qui n'est pas
        // lui ?
        re_registry.register(Box::new(crate::ImportsModule::default()));
        // Restent les OCTETS eux-mêmes. Les fragments d'abord : ils
        // n'ont de sens qu'après avoir lu les protections, dont ils
        // expliquent la raison d'être.
        re_registry.register(Box::new(crate::GadgetsModule::default()));
        // La place libre juste après : là où les fragments disent « il y a
        // déjà du code réutilisable », celle-ci dit « il y a aussi de la
        // place inutilisée ». Deux façons de voir qu'un binaire n'est pas
        // le bloc figé qu'on croit.
        re_registry.register(Box::new(crate::SlackModule::default()));
        // Le vidage en dernier : c'est là qu'on descend quand les
        // modules d'au-dessus ont désigné un endroit, et qu'on veut
        // savoir ce que ces octets-là sont vraiment.
        re_registry.register(Box::new(crate::DumpModule::default()));
        // Et l'éditeur binaire : on a compris quel octet décide, on le
        // change pour voir. Réversible (Correctifs), et rien n'atteint le
        // disque sans un enregistrement explicite.
        re_registry.register(Box::new(crate::HexEditorModule::default()));
        // Le détour en dernier, et c'est sa place : il suppose tout ce
        // qui précède. Il faut avoir vu la place libre pour savoir où
        // atterrir, le listing pour choisir l'endroit, et les correctifs
        // pour comprendre qu'on peut se dédire. C'est aussi le seul
        // panneau qui AJOUTE du code au lieu d'en retirer.
        re_registry.register(Box::new(crate::DetourModule::default()));
        re_registry
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

    /// Les modules, en lecture seule — pour les INTERROGER sans les
    /// faire travailler : un rapport lit des vues, il n'analyse rien.
    pub fn iter(&self) -> impl Iterator<Item = &Box<dyn ReModule>> {
        self.modules.iter()
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
        self.nav_entries_jusqu_a(lang, Niveau::Avancé)
    }

    /// Les mêmes, bornées à un niveau.
    ///
    /// `Niveau::Débutant` ne garde que les panneaux qui se lisent sans
    /// rien savoir d'avance ; `Niveau::Avancé` les garde tous. C'est
    /// l'unique endroit où le niveau décide de ce qu'on voit — la
    /// colonne de navigation —, et c'est voulu : un panneau caché
    /// reste enregistré, continue de travailler pour le compte rendu
    /// « Tout », et reparaît sans rien recharger.
    pub fn nav_entries_jusqu_a(
        &self,
        lang: &str,
        niveau: Niveau,
    ) -> Vec<(&'static str, &'static str, ReSection)> {
        self.nav_entries_pour(lang, niveau, "")
    }

    /// Les mêmes, en gardant d'office la rubrique OUVERTE.
    ///
    /// Le niveau allège la liste ; il n'escamote pas ce qu'on regarde.
    /// Un panneau au-dessus du niveau peut très bien être à l'écran :
    /// on y est arrivé par un lien d'un autre panneau — « aller voir
    /// cette adresse dans les octets » —, et c'est une demande. Le
    /// retirer de la colonne laisserait une rubrique ouverte sans
    /// entrée : on ne saurait plus où l'on est, ni comment en sortir.
    pub fn nav_entries_pour(
        &self,
        lang: &str,
        niveau: Niveau,
        ouvert: &str,
    ) -> Vec<(&'static str, &'static str, ReSection)> {
        self.modules
            .iter()
            .filter(|m| m.niveau() <= niveau || m.id() == ouvert)
            .map(|m| (m.id(), m.nav_title(lang), m.section()))
            .collect()
    }

    /// Ce module-là veut-il toute la largeur ? Faux s'il n'existe pas.
    pub fn pleine_largeur(&self, id: &str) -> bool {
        self.modules
            .iter()
            .find(|m| m.id() == id)
            .is_some_and(|m| m.pleine_largeur())
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
