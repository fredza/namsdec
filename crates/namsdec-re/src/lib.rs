//! Point d'extension pour les fonctions de reverse engineering : chaque
//! analyse (CFG, xrefs, détection de fonctions, recherche de patterns...)
//! s'implémente comme un `ReModule` greffé au registre, avec son propre
//! panneau dans l'interface. Aucun module "métier" n'existe encore en
//! phase 1 — `BinaryInfoModule` sert de preuve de concept pour valider le
//! mécanisme avant la phase 2.

mod binary_info;
pub mod elf_scan;
pub mod gadgets;
mod gadgets_module;
pub mod imports;
mod imports_module;
pub mod libc;
mod protections;
pub mod report;
pub mod strings;
mod syscalls;
pub mod table;
pub mod xrefs;
mod xrefs_module;

pub use binary_info::BinaryInfoModule;
pub use gadgets_module::GadgetsModule;
pub use imports_module::ImportsModule;
pub use protections::ProtectionsModule;
pub use strings::StringsModule;
pub use syscalls::SyscallsModule;
pub use xrefs_module::XrefsModule;

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
}

impl ReContext<'_> {
    /// Le texte de la langue courante.
    pub fn t(&self, fr: &'static str, en: &'static str) -> &'static str {
        if self.lang.starts_with("fr") { fr } else { en }
    }
}

pub trait ReModule {
    /// Identifiant stable (pour retrouver/désactiver un module).
    fn id(&self) -> &'static str;
    /// Titre affiché en tête du panneau du module, dans la langue
    /// demandée.
    fn title(&self, lang: &str) -> &'static str;

    /// Appelé après un (ré)assemblage réussi, avant la première exécution.
    fn on_binary_loaded(&mut self, _ctx: &ReContext) {}

    /// Appelé après chaque `step()`/`run()` de l'émulateur.
    fn on_step(&mut self, _ctx: &ReContext) {}

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
