//! Le panneau des fragments : ce que NX laisse quand même faire.
//!
//! Il existe pour expliquer les protections d'à côté, pas pour armer
//! qui que ce soit : voir qu'un programme de dix lignes contient déjà
//! des fragments enchaînables, sans que personne les ait écrits, est ce
//! qui fait comprendre pourquoi une pile non exécutable ne suffit pas —
//! et pourquoi RELRO, le canari et IBT existent.

use crate::gadgets::{self, Gadget};
use crate::{ReContext, ReModule, ReRequest, clickable_address};

/// Combien on en montre. Au-delà, la liste cesse d'apprendre quoi que
/// ce soit et devient un vidage.
const SHOWN: usize = 40;

#[derive(Default)]
pub struct GadgetsModule {
    /// Ce que le panneau demande à l'application : aller voir une
    /// adresse. Relevé après le dessin, jamais pendant.
    demande: Option<ReRequest>,
    found: Vec<Gadget>,
    /// Combien il y en avait en tout, avant la coupe.
    total: usize,
    /// Ce qu'on cherche parmi eux : on ne parcourt pas une liste de
    /// fragments, on y cherche celui qui met un argument en place.
    query: String,
}

impl ReModule for GadgetsModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

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
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(ctx.t("chercher — « pop rdi »…", "search — \"pop rdi\"…"))
                    .desired_width(160.0),
            );
            if !self.query.is_empty() && ui.small_button("x").clicked() {
                self.query.clear();
            }
        });
        let query = self.query.to_lowercase();
        let montrés: Vec<&Gadget> = self
            .found
            .iter()
            .filter(|gadget| query.is_empty() || gadget.text().to_lowercase().contains(&query))
            .collect();
        ui.label(if !query.is_empty() {
            format!(
                "{} / {} {}",
                montrés.len(),
                self.found.len(),
                ctx.t("montrés", "shown")
            )
        } else if self.total > self.found.len() {
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
        // Relevé pendant le dessin, posé après : la grille emprunte
        // `self`, et le module ne peut donc pas s'écrire dedans.
        let mut demande: Option<ReRequest> = None;
        egui::Grid::new("gadgets_grid")
            .num_columns(2)
            .striped(true)
            .min_col_width(0.0)
            .show(ui, |ui| {
                for gadget in &montrés {
                    if clickable_address(ui, ctx, gadget.addr) {
                        demande = Some(ReRequest::GoTo { addr: gadget.addr });
                    }
                    ui.monospace(gadget.text());
                    ui.end_row();
                }
            });
        if demande.is_some() {
            self.demande = demande;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// Le filtre sert à trouver le fragment qu'on cherche — « pop rdi »
    /// met un argument en place, et c'est celui-là qu'on veut.
    #[test]
    fn the_filter_finds_the_fragment_one_is_after() {
        // pop rdi ; ret, puis pop rsi ; ret.
        let bytes: &[u8] = &[0x5f, 0xc3, 0x5e, 0xc3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };

        let mut module = GadgetsModule::default();
        module.on_binary_loaded(&ctx);
        assert!(module.found.len() >= 2, "{:?}", module.found);

        // Le filtre est une simple recherche dans le texte du fragment :
        // c'est ce qu'on teste, le rendu n'y ajoutant rien.
        module.query = "pop rdi".to_owned();
        let montrés: Vec<&Gadget> = module
            .found
            .iter()
            .filter(|gadget| gadget.text().to_lowercase().contains(&module.query))
            .collect();
        assert_eq!(montrés.len(), 1, "{montrés:?}");
        assert_eq!(montrés[0].text(), "pop rdi ; ret");
    }
}
