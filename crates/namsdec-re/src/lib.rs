//! Point d'extension pour les fonctions de reverse engineering : chaque
//! analyse (CFG, xrefs, détection de fonctions, recherche de patterns...)
//! s'implémente comme un `ReModule` greffé au registre, avec son propre
//! panneau dans l'interface. Aucun module "métier" n'existe encore en
//! phase 1 — `BinaryInfoModule` sert de preuve de concept pour valider le
//! mécanisme avant la phase 2.

mod binary_info;

pub use binary_info::BinaryInfoModule;

use namsdec_core::Cpu;
use namsdec_disasm::DecodedInstruction;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Ce que voit un module à chaque point d'accroche : l'état du CPU simulé,
/// le désassemblage du binaire chargé, et le source NASM d'origine.
pub struct ReContext<'a> {
    pub cpu: &'a Cpu,
    pub disasm: &'a [DecodedInstruction],
    pub source: &'a str,
}

pub trait ReModule {
    /// Identifiant stable (pour retrouver/désactiver un module).
    fn id(&self) -> &'static str;
    /// Titre affiché sur l'onglet du panneau.
    fn title(&self) -> &'static str;

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
    pub fn titles(&self) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.title()).collect()
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
