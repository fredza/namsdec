//! Le module « Fonctions » : par où commencer à lire.
//!
//! Devant un binaire dépouillé, la première question n'est pas « que
//! fait cette instruction » mais « où est le programme ». Deux cent
//! quarante kilo-octets de `.text` dont la quasi-totalité appartient à
//! la bibliothèque standard : ce qu'on cherche tient en quelques
//! centaines d'octets, et rien ne dit lesquels.
//!
//! Ce module répond en trois temps : le `main`, retrouvé par la
//! convention de démarrage ; les fonctions qu'on sait nommer ; et le
//! nombre d'appels qui mènent à chacune, parce qu'une fonction appelée
//! de quarante endroits est un utilitaire, et que celle qui n'est
//! appelée qu'une fois est le programme.

use crate::functions::{Function, Origin, by_importance, discover};
use crate::{ReContext, ReModule};

#[derive(Default)]
pub struct FunctionsModule {
    found: Vec<Function>,
    main: Option<u64>,
    entry: u64,
}

/// Combien de fonctions la liste montre avant de s'arrêter.
///
/// Trois cents lignes ne se lisent pas : on montre les plus employées,
/// et le compte total dit ce qu'on ne montre pas.
const MONTRÉES: usize = 40;

impl ReModule for FunctionsModule {
    fn id(&self) -> &'static str {
        "functions"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Fonctions"
        } else {
            "Functions"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        // Le point d'entrée déclaré par le fichier, quand on l'a : c'est
        // de lui qu'on remonte jusqu'à `main`.
        self.entry = crate::elf_scan::scan(ctx.binary)
            .map(|elf| elf.entry)
            .or_else(|| ctx.disasm.first().map(|i| i.address))
            .unwrap_or(0);
        self.found = discover(ctx.disasm, self.entry);
        self.main = crate::functions::main_of(ctx.disasm, self.entry);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.found.is_empty() {
            ui.label(ctx.t(
                "Rien à découper : aucun code n'est chargé.",
                "Nothing to carve up: no code is loaded.",
            ));
            return;
        }

        match self.main {
            Some(main) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(ctx.t("Le programme commence à", "The program starts at"));
                    ui.monospace(format!("{main:#010x}"));
                });
                ui.weak(ctx.t(
                    "Trouvé par la convention de démarrage : le code d'amorçage passe \
                     l'adresse de `main` à `__libc_start_main`, dans RDI. C'est une \
                     convention, pas une loi — un programme peut s'en passer.",
                    "Found through the startup convention: the bootstrap code hands the \
                     address of `main` to `__libc_start_main`, in RDI. That is a \
                     convention, not a law — a program may do without it.",
                ));
            }
            None => {
                ui.label(ctx.t(
                    "Le `main` n'a pas été retrouvé : ce programme ne suit pas la \
                     convention de démarrage de la bibliothèque C.",
                    "No `main` was found: this program does not follow the C library's \
                     startup convention.",
                ));
            }
        }
        ui.add_space(4.0);
        ui.weak(format!(
            "{} {}",
            self.found.len(),
            ctx.t(
                "fonctions retrouvées — les cibles d'appel, et rien d'autre : ce qu'on \
                 n'appelle que par pointeur échappe à ce relevé.",
                "functions recovered — call targets, and nothing else: what is only \
                 called through a pointer escapes this survey.",
            )
        ));
        ui.add_space(4.0);

        egui::Grid::new("re_functions")
            .num_columns(4)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                ui.weak(ctx.t("nom", "name"));
                ui.weak(ctx.t("adresse", "address"));
                ui.weak(ctx.t("taille", "size"));
                ui.weak(ctx.t("appelants", "callers"));
                ui.end_row();
                for fonction in by_importance(&self.found).into_iter().take(MONTRÉES) {
                    let nom = ctx.display_name(fonction.start, &fonction.name());
                    let mut étiquette = match fonction.origin {
                        Origin::Main | Origin::Entry => egui::RichText::new(nom).strong(),
                        Origin::Called => egui::RichText::new(nom),
                    };
                    // Rebaptisée à la main : un léger fond, comme ailleurs,
                    // pour repérer ce qu'on a déjà nommé.
                    if ctx.user_name(fonction.start).is_some() {
                        étiquette = étiquette
                            .background_color(ui.visuals().selection.bg_fill.gamma_multiply(0.25));
                    }
                    ui.label(étiquette.monospace());
                    ui.monospace(format!("{:#010x}", fonction.start));
                    ui.monospace(format!("{}", fonction.size()));
                    ui.monospace(format!("{}", fonction.callers));
                    ui.end_row();
                }
            });
        if self.found.len() > MONTRÉES {
            ui.weak(format!(
                "… {} {}",
                self.found.len() - MONTRÉES,
                ctx.t("de plus", "more")
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// Le module nomme le point de départ, et ne prétend rien quand la
    /// convention n'est pas suivie.
    #[test]
    fn the_module_names_where_to_start_or_says_it_cannot() {
        // Un démarrage en règle : lea rdi, [rip+0x10] puis call.
        let bytes: &[u8] = &[
            0xF3, 0x0F, 0x1E, 0xFA, 0x48, 0x8D, 0x3D, 0x10, 0x00, 0x00, 0x00, 0xFF, 0x15, 0x00,
            0x20, 0x00, 0x00, 0xF4,
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };
        let mut module = FunctionsModule::default();
        module.on_binary_loaded(&ctx);
        // `lea` à 0x1004, longue de sept octets : 0x100b + 0x10.
        assert_eq!(module.main, Some(0x101b));
        assert!(!module.found.is_empty());
    }
}
