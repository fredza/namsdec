//! Le panneau « Protections » : ce que le binaire oppose à qui l'attaque.
//!
//! C'est la première chose qu'on regarde d'un binaire inconnu, et la
//! dernière qu'on vérifie sur le sien. Chaque ligne dit l'état, et ce que
//! cet état change concrètement — une case rouge sans explication
//! n'apprend rien à qui découvre.

use crate::elf_scan::{self, Kind, Relro, Scan};
use crate::{ReContext, ReModule, ReSection};

#[derive(Default)]
pub struct ProtectionsModule {
    scan: Option<Scan>,
    /// Le fichier n'était pas un ELF64 : on le dit, plutôt que de
    /// laisser un panneau vide qu'on croirait cassé.
    unreadable: bool,
}

impl ProtectionsModule {
    /// L'analyse, pour qui veut la lire sans passer par l'interface.
    pub fn scan(&self) -> Option<&Scan> {
        self.scan.as_ref()
    }
}

/// Une ligne de verdict : vert quand la protection est là, rouge quand
/// elle manque, et une phrase qui dit pourquoi ça compte.
///
/// La phrase est écrite sous la ligne quand la protection *manque* — un
/// point faible se lit sans avoir à le chercher — et reste en infobulle
/// quand elle est en place : la liste tient alors d'un coup d'œil, ce
/// qu'on lui demande d'abord.
fn verdict(ui: &mut egui::Ui, good: bool, label: &str, state: &str, why: &str) {
    let color = if good {
        egui::Color32::from_rgb(0x4e, 0xc9, 0xb0)
    } else {
        egui::Color32::from_rgb(0xf4, 0x87, 0x71)
    };
    let line = ui
        .horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 4.0, color);
            ui.label(label);
            ui.colored_label(color, state);
        })
        .response;
    if good {
        line.on_hover_text(why);
    } else {
        ui.indent(label, |ui| {
            ui.weak(why);
        });
    }
}

impl ReModule for ProtectionsModule {
    fn section(&self) -> ReSection {
        ReSection::File
    }

    fn id(&self) -> &'static str {
        "protections"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Protections du binaire"
        } else {
            "Binary protections"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        if ctx.binary.is_empty() {
            self.scan = None;
            self.unreadable = false;
            return;
        }
        self.scan = elf_scan::scan(ctx.binary);
        self.unreadable = self.scan.is_none();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let Some(scan) = &self.scan else {
            ui.weak(if self.unreadable {
                ctx.t(
                    "Ce fichier n'est pas un ELF64 : rien à analyser ici.",
                    "This file is not an ELF64: nothing to analyse here.",
                )
            } else {
                ctx.t("Aucun binaire chargé.", "No binary loaded.")
            });
            return;
        };

        // L'identité du fichier, d'abord : le reste ne se lit qu'en
        // sachant de quoi on parle.
        ui.horizontal_wrapped(|ui| {
            ui.strong(match scan.kind {
                Kind::Executable => ctx.t("Exécutable", "Executable"),
                Kind::Shared => ctx.t("Exécutable relogeable", "Relocatable executable"),
                Kind::Relocatable => ctx.t("Objet à lier", "Relocatable object"),
                Kind::Core => ctx.t("Image mémoire (core)", "Core dump"),
                Kind::Other(_) => ctx.t("Type inconnu", "Unknown type"),
            });
            ui.weak("·");
            ui.label(scan.machine);
            ui.weak("·");
            ui.label(format!("{} {:#x}", ctx.t("entrée", "entry"), scan.entry));
            ui.weak("·");
            ui.label(if scan.dynamic() {
                ctx.t("lié dynamiquement", "dynamically linked")
            } else {
                ctx.t("lié statiquement", "statically linked")
            });
        });
        ui.add_space(6.0);

        verdict(
            ui,
            scan.pie(),
            ctx.t("PIE", "PIE"),
            if scan.pie() {
                ctx.t("oui", "yes")
            } else {
                ctx.t("non", "no")
            },
            if scan.pie() {
                ctx.t(
                    "Le programme est posé à une adresse différente à chaque \
                     exécution : les adresses lues ici ne seront pas celles de \
                     la prochaine fois.",
                    "The program lands at a different address every run: the \
                     addresses read here will not be next run's.",
                )
            } else {
                ctx.t(
                    "Les adresses sont écrites en dur : celles du désassemblage \
                     sont celles qu'il aura, à chaque exécution.",
                    "Addresses are fixed: the ones in the disassembly are the \
                     ones it will have, every run.",
                )
            },
        );

        let nx = scan.nx.unwrap_or(false);
        verdict(
            ui,
            nx,
            "NX",
            match scan.nx {
                Some(true) => ctx.t("pile non exécutable", "non-executable stack"),
                Some(false) => ctx.t("pile exécutable", "executable stack"),
                None => ctx.t("non déclaré", "not declared"),
            },
            match scan.nx {
                Some(true) => ctx.t(
                    "Du code déposé sur la pile ne s'exécutera pas : il faut \
                     réutiliser celui du programme (ROP) plutôt qu'en injecter.",
                    "Code written onto the stack will not run: one has to reuse \
                     the program's own (ROP) instead of injecting any.",
                ),
                Some(false) => ctx.t(
                    "La pile est exécutable : des octets qu'on y écrit peuvent \
                     être exécutés tels quels.",
                    "The stack is executable: bytes written there can be run as \
                     they are.",
                ),
                None => ctx.t(
                    "Le fichier ne déclare pas de segment de pile : le noyau \
                     décide seul, et il ne l'a pas toujours rendue non \
                     exécutable.",
                    "The file declares no stack segment: the kernel decides \
                     alone, and it has not always made it non-executable.",
                ),
            },
        );

        verdict(
            ui,
            scan.relro == Relro::Full,
            "RELRO",
            match scan.relro {
                Relro::Full => ctx.t("complet", "full"),
                Relro::Partial => ctx.t("partiel", "partial"),
                Relro::None => ctx.t("absent", "none"),
            },
            match scan.relro {
                Relro::Full => ctx.t(
                    "Les tables de liaison sont résolues au démarrage puis \
                     passées en lecture seule : une écriture arbitraire ne peut \
                     plus les détourner.",
                    "Linking tables are resolved at startup then made read-only: \
                     an arbitrary write can no longer hijack them.",
                ),
                Relro::Partial => ctx.t(
                    "Une partie des données est protégée, mais la table des \
                     sauts reste inscriptible — c'est elle qu'on détourne pour \
                     rediriger un appel de bibliothèque.",
                    "Some data is protected, but the jump table stays writable — \
                     that is what gets hijacked to redirect a library call.",
                ),
                Relro::None => ctx.t(
                    "Rien n'est repassé en lecture seule après le chargement.",
                    "Nothing is made read-only after loading.",
                ),
            },
        );

        verdict(
            ui,
            scan.canary,
            ctx.t("Canari de pile", "Stack canary"),
            if scan.canary {
                ctx.t("présent", "present")
            } else {
                ctx.t("absent", "absent")
            },
            if scan.canary {
                ctx.t(
                    "Une valeur secrète est posée avant l'adresse de retour et \
                     vérifiée avant de s'en servir : un débordement qui écrase \
                     l'une écrase l'autre, et le programme s'arrête.",
                    "A secret value sits before the return address and is checked \
                     before use: an overflow that smashes one smashes the other, \
                     and the program aborts.",
                )
            } else {
                ctx.t(
                    "Rien ne surveille l'adresse de retour : un débordement de \
                     tampon sur la pile la remplace sans que personne ne le voie.",
                    "Nothing watches the return address: a stack buffer overflow \
                     replaces it unnoticed.",
                )
            },
        );

        verdict(
            ui,
            scan.cet.ibt,
            "IBT",
            if scan.cet.ibt {
                ctx.t("demandé", "requested")
            } else {
                ctx.t("non demandé", "not requested")
            },
            if scan.cet.ibt {
                ctx.t(
                    "Un saut indirect ne peut atterrir que sur un `endbr64` : \
                     les morceaux de code choisis au milieu d'une instruction \
                     ne sont plus des cibles valables.",
                    "An indirect jump may only land on an `endbr64`: code \
                     fragments picked mid-instruction are no longer valid \
                     targets.",
                )
            } else {
                ctx.t(
                    "N'importe quelle adresse peut être la cible d'un saut \
                     indirect, y compris au milieu d'une instruction.",
                    "Any address can be the target of an indirect jump, \
                     including mid-instruction.",
                )
            },
        );
        if scan.cet.shadow_stack {
            verdict(
                ui,
                true,
                ctx.t("Pile fantôme", "Shadow stack"),
                ctx.t("demandée", "requested"),
                ctx.t(
                    "Le processeur garde sa propre copie des adresses de retour \
                     et compare : écraser celle de la pile ne suffit plus.",
                    "The processor keeps its own copy of return addresses and \
                     compares: overwriting the one on the stack is no longer \
                     enough.",
                ),
            );
        }

        if !scan.fortified.is_empty() {
            verdict(
                ui,
                true,
                "FORTIFY",
                &format!(
                    "{} {}",
                    scan.fortified.len(),
                    ctx.t("fonctions", "functions")
                ),
                ctx.t(
                    "Certaines fonctions de copie ont été remplacées par des \
                     versions qui connaissent la taille de la destination.",
                    "Some copy functions were replaced by versions that know the \
                     size of their destination.",
                ),
            );
        }

        let rwx = scan.writable_executable();
        if !rwx.is_empty() {
            verdict(
                ui,
                false,
                ctx.t("Segment W+X", "W+X segment"),
                &rwx.iter()
                    .map(|s| format!("{:#x}", s.vaddr))
                    .collect::<Vec<_>>()
                    .join(", "),
                ctx.t(
                    "Une zone est à la fois inscriptible et exécutable : on peut \
                     y écrire du code et sauter dedans, sans rien contourner.",
                    "A region is both writable and executable: one can write code \
                     there and jump into it, with nothing to bypass.",
                ),
            );
        }

        let symbols = if scan.stripped {
            ctx.t("dépouillé", "stripped").to_owned()
        } else {
            scan.symbol_count.to_string()
        };
        verdict(
            ui,
            !scan.stripped,
            ctx.t("Symboles", "Symbols"),
            &symbols,
            if scan.stripped {
                ctx.t(
                    "Les noms ont été retirés : les fonctions n'ont plus que \
                     leur adresse.",
                    "Names were removed: functions have nothing but their address.",
                )
            } else {
                ctx.t(
                    "Les noms sont là : les fonctions se lisent par leur nom \
                     plutôt que par leur adresse.",
                    "The names are there: functions read by name rather than by \
                     address.",
                )
            },
        );

        // Ce que le chargeur ira chercher ailleurs : autant de code qui
        // s'exécutera dans le processus sans être dans le fichier.
        if !scan.needed.is_empty() {
            ui.add_space(6.0);
            ui.strong(ctx.t("Bibliothèques réclamées", "Required libraries"));
            for lib in &scan.needed {
                ui.monospace(lib);
            }
        }
        for (label, value) in [
            (ctx.t("RUNPATH", "RUNPATH"), &scan.runpath),
            (ctx.t("RPATH", "RPATH"), &scan.rpath),
        ] {
            if let Some(value) = value {
                ui.horizontal_wrapped(|ui| {
                    ui.label(label);
                    ui.monospace(value);
                });
                ui.weak(ctx.t(
                    "Un chemin de recherche inscrit dans le binaire : qui peut \
                     écrire dedans peut y placer sa propre bibliothèque.",
                    "A search path baked into the binary: whoever can write there \
                     can drop their own library in it.",
                ));
            }
        }

        if let Some(build_id) = &scan.build_id {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(ctx.t("Empreinte de build", "Build ID"));
                ui.monospace(build_id);
            })
            .response
            .on_hover_text(ctx.t(
                "Deux fichiers qui la partagent sont le même programme, \
                 compilé une seule fois.",
                "Two files sharing it are the same program, built once.",
            ));
        }

        // Les fonctions nommées : par où commencer la lecture.
        if !scan.functions.is_empty() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(format!(
                "{} ({})",
                ctx.t("Fonctions", "Functions"),
                scan.functions.len()
            ))
            .id_salt("protections_functions")
            .show(ui, |ui| {
                egui::Grid::new("protections_functions_grid")
                    .num_columns(3)
                    .striped(true)
                    .min_col_width(0.0)
                    .show(ui, |ui| {
                        for function in &scan.functions {
                            ui.monospace(format!("{:#x}", function.addr));
                            ui.monospace(&function.name);
                            ui.weak(if function.size > 0 {
                                format!("{} {}", function.size, ctx.t("o", "B"))
                            } else {
                                String::new()
                            });
                            ui.end_row();
                        }
                    });
            });
        }

        // Les sections : le découpage fin, celui que l'éditeur de liens a
        // choisi — plus précis que les segments, qui les regroupent par
        // droits.
        if !scan.sections.is_empty() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(format!(
                "{} ({})",
                ctx.t("Sections", "Sections"),
                scan.sections.len()
            ))
            .id_salt("protections_sections")
            .show(ui, |ui| {
                egui::Grid::new("protections_sections_grid")
                    .num_columns(4)
                    .striped(true)
                    .min_col_width(0.0)
                    .show(ui, |ui| {
                        for section in &scan.sections {
                            ui.monospace(&section.name);
                            ui.monospace(format!(
                                "{}{}",
                                if section.write { 'w' } else { '-' },
                                if section.execute { 'x' } else { '-' }
                            ));
                            ui.monospace(if section.addr > 0 {
                                format!("{:#x}", section.addr)
                            } else {
                                String::new()
                            });
                            ui.monospace(format!("{} {}", section.size, ctx.t("o", "B")));
                            ui.end_row();
                        }
                    });
            });
        }

        // Et la carte de ce qui sera projeté, droits compris : c'est elle
        // qui explique pourquoi telle adresse est lisible et telle autre
        // pas.
        ui.add_space(6.0);
        ui.strong(ctx.t("Segments", "Segments"));
        egui::Grid::new("protections_segments")
            .num_columns(4)
            .striped(true)
            .min_col_width(0.0)
            .show(ui, |ui| {
                for segment in &scan.segments {
                    ui.monospace(segment.kind);
                    ui.monospace(segment.rights());
                    ui.monospace(format!("{:#x}", segment.vaddr));
                    ui.monospace(format!("{} {}", segment.memsz, ctx.t("o", "B")));
                    ui.end_row();
                }
            });
    }
}
