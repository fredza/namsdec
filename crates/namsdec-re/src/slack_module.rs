//! Le panneau de la place libre : où l'éditeur de liens a laissé du vide.
//!
//! Il existe pour expliquer la disposition d'un ELF, pas pour armer qui
//! que ce soit : voir qu'un binaire de dix lignes contient déjà des
//! dizaines d'octets de bourrage, sans que personne les ait demandés,
//! fait comprendre pourquoi un exécutable pèse plus que son code — et
//! pourquoi « garder la longueur exacte » n'est pas la seule façon de le
//! modifier. C'est de la lecture seule : le panneau montre, il n'écrit
//! rien.

use crate::slack::{self, Slack};
use crate::{ReContext, ReModule};

/// Combien on en montre. Au-delà, la liste cesse d'apprendre et devient
/// un vidage.
const SHOWN: usize = 30;

#[derive(Default)]
pub struct SlackModule {
    found: Vec<Slack>,
    /// Combien il y en avait avant la coupe, et leur somme.
    total: usize,
    somme: u64,
    plus_grande: u64,
}

impl ReModule for SlackModule {
    fn id(&self) -> &'static str {
        "slack"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Place libre (bourrage)"
        } else {
            "Free space (padding)"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        let tous = slack::find(ctx.binary);
        self.total = tous.len();
        let (somme, plus_grande) = slack::totals(&tous);
        self.somme = somme;
        self.plus_grande = plus_grande;
        self.found = tous.into_iter().take(SHOWN).collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.weak(ctx.t(
            "Un exécutable n'est jamais plein. Entre deux fonctions, entre deux \
             sections, à la fin d'un segment, l'éditeur de liens comble avec un \
             octet neutre — un zéro, un `nop` (0x90), un `int3` (0xcc) — jusqu'au \
             prochain alignement. Ces trous se voient, et c'est de la place que le \
             code n'utilise pas.",
            "An executable is never full. Between two functions, between two \
             sections, at the end of a segment, the linker fills with a neutral \
             byte — a zero, a `nop` (0x90), an `int3` (0xcc) — up to the next \
             alignment. These gaps are visible, and they are space the code does \
             not use.",
        ));

        if self.found.is_empty() {
            ui.add_space(4.0);
            ui.weak(ctx.t(
                "Aucun trou notable ici — ou ce n'est pas un ELF chargé.",
                "No notable gap here — or this is not a loaded ELF.",
            ));
            return;
        }

        ui.add_space(6.0);
        ui.label(format!(
            "{} {} · {} {} · {} {}",
            self.total,
            ctx.t("zones", "zones"),
            self.somme,
            ctx.t("octets en tout", "bytes in all"),
            self.plus_grande,
            ctx.t("d'un tenant", "in one run"),
        ));
        ui.add_space(4.0);

        egui::Grid::new("slack_grid")
            .num_columns(5)
            .striped(true)
            .min_col_width(0.0)
            .show(ui, |ui| {
                ui.strong(ctx.t("adresse", "address"));
                ui.strong(ctx.t("octets", "bytes"));
                ui.strong(ctx.t("rempli de", "filled with"));
                ui.strong(ctx.t("section", "section"));
                ui.strong("");
                ui.end_row();
                for trou in &self.found {
                    ui.monospace(format!("{:#x}", trou.vaddr));
                    ui.monospace(format!("{}", trou.len));
                    ui.monospace(nom_du_bourrage(trou.filler));
                    // Le nom de section raconte l'histoire : un trou en
                    // fin de `.text` n'est pas un trou entre deux entrées
                    // de `.plt`. Un tiret quand aucune section déclarée ne
                    // le couvre — l'interstice entre deux, ou un binaire
                    // dépouillé.
                    match &trou.section {
                        Some(nom) => ui.monospace(nom.clone()),
                        None => ui.weak("—"),
                    };
                    // Un trou dans du code exécutable n'a pas le même sens
                    // qu'un trou dans les données : le dire d'un mot.
                    ui.weak(if trou.executable {
                        ctx.t("exécutable", "executable")
                    } else {
                        ctx.t("données", "data")
                    });
                    ui.end_row();
                }
            });

        if self.total > self.found.len() {
            ui.add_space(4.0);
            ui.weak(format!(
                "… {} {}",
                self.total - self.found.len(),
                ctx.t("de plus, plus petites", "more, smaller"),
            ));
        }
    }
}

/// L'octet de bourrage, et ce qu'il veut dire. `0xcc` est `int3`, le
/// piège d'un débogueur ; `0x90` est `nop` ; `0x00` est le remplissage
/// par défaut. Le montrer nommé apprend plus que la seule valeur.
fn nom_du_bourrage(octet: u8) -> String {
    match octet {
        0x00 => "0x00 (zéro)".to_owned(),
        0x90 => "0x90 (nop)".to_owned(),
        0xcc => "0xcc (int3)".to_owned(),
        autre => format!("{autre:#04x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn contexte<'a>(binary: &'a [u8], cpu: &'a Cpu) -> ReContext<'a> {
        ReContext {
            cpu,
            disasm: &[],
            source: "",
            binary,
            lang: "fr",
        }
    }

    /// Un module frais ne montre rien et ne panique pas.
    #[test]
    fn a_fresh_module_shows_nothing() {
        let module = SlackModule::default();
        assert!(module.found.is_empty());
        assert_eq!(module.total, 0);
    }

    /// Chargé sur un tampon qui n'est pas un ELF, il reste vide.
    #[test]
    fn a_non_elf_leaves_it_empty() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let mut module = SlackModule::default();
        module.on_binary_loaded(&contexte(b"pas un elf", &cpu));
        assert!(module.found.is_empty());
    }

    /// Le nom du bourrage dit ce que l'octet fait, pas seulement sa
    /// valeur.
    #[test]
    fn the_filler_is_named_not_just_numbered() {
        assert!(nom_du_bourrage(0xcc).contains("int3"));
        assert!(nom_du_bourrage(0x90).contains("nop"));
        assert!(nom_du_bourrage(0x00).contains("zéro"));
        assert_eq!(nom_du_bourrage(0xab), "0xab");
    }
}
