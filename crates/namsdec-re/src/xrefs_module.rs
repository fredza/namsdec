//! Le panneau des références croisées : par où entrer dans un binaire.
//!
//! On y trouve les fonctions — les adresses qu'un `call` désigne, que
//! leurs noms aient été retirés ou non — classées par nombre
//! d'appelants, et pour chacune d'où elle est appelée. C'est le chemin
//! qu'on suit en lisant du code qu'on n'a pas écrit : commencer par ce
//! qui sert partout, puis remonter.

use crate::xrefs::{self, Kind, Xrefs};
use crate::{ReContext, ReModule};

#[derive(Default)]
pub struct XrefsModule {
    xrefs: Xrefs,
    /// La cible dépliée, s'il y en a une.
    ouverte: Option<u64>,
}

impl XrefsModule {
    /// Les références relevées, pour qui veut les lire sans l'interface.
    pub fn xrefs(&self) -> &Xrefs {
        &self.xrefs
    }
}

impl ReModule for XrefsModule {
    fn id(&self) -> &'static str {
        "xrefs"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Références croisées"
        } else {
            "Cross-references"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        // La plage du programme : ce qui tombe dedans est une adresse,
        // le reste est un nombre.
        let range = match (ctx.disasm.first(), ctx.disasm.last()) {
            (Some(first), Some(last)) => {
                first.address..last.address + last.bytes.len() as u64 + 0x2000
            }
            _ => 0..0,
        };
        self.xrefs = xrefs::scan(ctx.disasm, range);
        self.ouverte = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.xrefs.is_empty() {
            ui.weak(ctx.t(
                "Aucune référence : le programme n'appelle rien et ne pointe nulle part.",
                "No references: this program calls nothing and points nowhere.",
            ));
            return;
        }

        let fonctions = self.xrefs.functions();
        if !fonctions.is_empty() {
            ui.strong(format!(
                "{} ({})",
                ctx.t("Fonctions appelées", "Called functions"),
                fonctions.len()
            ));
            ui.weak(ctx.t(
                "Une adresse qu'un `call` désigne est une fonction, même sans son nom.",
                "An address a `call` points at is a function, even without its name.",
            ));
            for (addr, appels) in &fonctions {
                let ouverte = self.ouverte == Some(*addr);
                let ligne = ui.horizontal(|ui| {
                    ui.monospace(format!("{addr:#x}"));
                    ui.weak(format!(
                        "{appels} {}",
                        if *appels > 1 {
                            ctx.t("appels", "calls")
                        } else {
                            ctx.t("appel", "call")
                        }
                    ));
                });
                if ligne
                    .response
                    .interact(egui::Sense::click())
                    .on_hover_text(ctx.t("Voir d'où", "Show from where"))
                    .clicked()
                {
                    self.ouverte = (!ouverte).then_some(*addr);
                }
                if ouverte {
                    ui.indent(addr, |ui| {
                        for reference in self.xrefs.to(*addr) {
                            ui.horizontal(|ui| {
                                ui.weak(match reference.kind {
                                    Kind::Call => ctx.t("appelé depuis", "called from"),
                                    Kind::Jump => ctx.t("saut depuis", "jumped from"),
                                    Kind::Data => ctx.t("lu depuis", "read from"),
                                    Kind::Address => ctx.t("adresse prise en", "address taken at"),
                                });
                                ui.monospace(format!("{:#x}", reference.from));
                            });
                        }
                    });
                }
            }
        }

        // Ce que le programme désigne sans l'appeler : ses données. Le
        // compte suffit ici — le détail est dans le panneau des chaînes,
        // qui les montre avec leur contenu.
        let données = self
            .xrefs
            .targets()
            .filter(|(_, refs)| {
                refs.iter()
                    .any(|r| matches!(r.kind, Kind::Data | Kind::Address))
            })
            .count();
        if données > 0 {
            ui.add_space(4.0);
            ui.weak(format!(
                "{données} {}",
                ctx.t(
                    "adresses de données désignées par le code",
                    "data addresses pointed at by the code"
                )
            ));
        }
    }
}
