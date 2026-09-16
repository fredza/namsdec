//! Le panneau des fragments : ce que NX laisse quand même faire.
//!
//! Il existe pour expliquer les protections d'à côté, pas pour armer
//! qui que ce soit : voir qu'un programme de dix lignes contient déjà
//! des fragments enchaînables, sans que personne les ait écrits, est ce
//! qui fait comprendre pourquoi une pile non exécutable ne suffit pas —
//! et pourquoi RELRO, le canari et IBT existent.

use crate::gadgets::{self, Gadget};
use crate::{ReContext, ReModule};

/// Combien on en montre. Au-delà, la liste cesse d'apprendre quoi que
/// ce soit et devient un vidage.
const SHOWN: usize = 40;

#[derive(Default)]
pub struct GadgetsModule {
    found: Vec<Gadget>,
    /// Combien il y en avait en tout, avant la coupe.
    total: usize,
}

impl ReModule for GadgetsModule {
    fn id(&self) -> &'static str {
        "gadgets"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Fragments réutilisables (ROP)"
        } else {
            "Reusable fragments (ROP)"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        // Une récolte plus large que ce qu'on montre : le total dit
        // quelque chose, même tronqué.
        let tous = gadgets::from_disasm(ctx.disasm, SHOWN * 8);
        self.total = tous.len();
        self.found = tous.into_iter().take(SHOWN).collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.weak(ctx.t(
            "Une pile non exécutable (NX) empêche d'exécuter des octets qu'on y \
             dépose. Ces fragments-là, eux, sont déjà dans le code : chacun finit \
             par `ret`, et les enchaîner revient à écrire un programme avec les \
             morceaux d'un autre. C'est ce que les autres protections — RELRO, le \
             canari, IBT — cherchent à empêcher.",
            "A non-executable stack (NX) stops bytes written onto it from running. \
             These fragments are already in the code: each ends with `ret`, and \
             chaining them amounts to writing a program out of someone else's \
             pieces. That is what the other protections — RELRO, the canary, IBT — \
             are there to prevent.",
        ));
        if self.found.is_empty() {
            ui.add_space(4.0);
            ui.weak(ctx.t(
                "Aucun fragment ici : le programme est trop court.",
                "No fragments here: the program is too short.",
            ));
            return;
        }
        ui.add_space(4.0);
        ui.label(if self.total > self.found.len() {
            format!(
                "{} {} ({} {})",
                self.total,
                ctx.t("fragments trouvés", "fragments found"),
                self.found.len(),
                ctx.t("montrés", "shown")
            )
        } else {
            format!("{} {}", self.total, ctx.t("fragments", "fragments"))
        });
        egui::Grid::new("gadgets_grid")
            .num_columns(2)
            .striped(true)
            .min_col_width(0.0)
            .show(ui, |ui| {
                for gadget in &self.found {
                    ui.monospace(format!("{:#x}", gadget.addr));
                    ui.monospace(gadget.text());
                    ui.end_row();
                }
            });
    }
}
