//! Le panneau des fonctions empruntées : ce que le binaire appelle
//! ailleurs que chez lui.
//!
//! Le premier réflexe devant un binaire inconnu est de regarder ce qu'il
//! demande au système et à la bibliothèque C : qui ouvre des fichiers,
//! qui alloue, qui écrit. Les appels sont classés par nombre
//! d'appelants, comme les références croisées — ce qui sert partout se
//! lit en premier.

use crate::behaviour::{self, Family};
use crate::imports::{self, Import};
use crate::libc;
use crate::{ReContext, ReModule};

#[derive(Default)]
pub struct ImportsModule {
    imports: Vec<Import>,
    /// Combien de `call` du code visent chaque entrée.
    appels: Vec<usize>,
}

impl ImportsModule {
    /// Les fonctions importées, pour qui veut les lire sans l'interface.
    pub fn imports(&self) -> &[Import] {
        &self.imports
    }

    /// Combien d'appels mènent à chacune, dans le même ordre.
    pub fn calls(&self) -> &[usize] {
        &self.appels
    }

    /// Les familles de comportement présentes, dans l'ordre où on les
    /// regarde — le portrait du binaire en six cases.
    pub fn families(&self) -> Vec<Family> {
        behaviour::families_of(self.imports.iter().map(|i| i.name.as_str()))
    }
}

/// La ligne de familles : ce que le binaire sait faire, d'un coup d'œil.
///
/// Dessinée avant la liste, parce que c'est la question qu'on pose
/// d'abord. Chaque case porte le nombre de fonctions qui l'ont nommée —
/// une famille tenue par un seul appel ne dit pas la même chose qu'une
/// famille tenue par quinze.
pub(crate) fn portrait(ui: &mut egui::Ui, ctx: &ReContext, comptes: &[(Family, usize)]) {
    if comptes.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(ctx.t("Ce qu'il sait faire", "What it can do"));
        for (famille, combien) in comptes {
            let texte = format!("{} ({combien})", famille.label(ctx.lang));
            ui.label(egui::RichText::new(texte).color(famille.color()).strong())
                .on_hover_text(famille.why(ctx.lang));
        }
    });
}

impl ReModule for ImportsModule {
    fn id(&self) -> &'static str {
        "imports"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Fonctions importées"
        } else {
            "Imported functions"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.imports = imports::scan(ctx.binary);
        // Un appel par entrée : c'est ce qui distingue la fonction dont
        // tout dépend de celle qui est liée sans jamais servir.
        //
        // Les adresses ne se comparent pas telles quelles : le nom vient
        // des sections du fichier — des adresses virtuelles — et le
        // désassemblage part du premier octet de ce même fichier, à
        // l'adresse où namsdec l'a posé. Les deux se rejoignent sur la
        // *position dans le fichier*.
        let base = ctx.disasm.first().map(|first| first.address);
        let cibles: Vec<u64> = ctx
            .disasm
            .iter()
            .filter_map(|instruction| {
                namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
            })
            .filter(|decoded| decoded.flow_control() == iced_x86::FlowControl::Call)
            .map(|decoded| decoded.near_branch_target())
            .collect();
        self.appels = self
            .imports
            .iter()
            .map(|import| {
                let Some(offset) = imports::file_offset(ctx.binary, import.plt) else {
                    return 0;
                };
                cibles
                    .iter()
                    .filter(|cible| match base {
                        Some(base) => cible.checked_sub(base) == Some(offset),
                        None => **cible == import.plt,
                    })
                    .count()
            })
            .collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.imports.is_empty() {
            ui.weak(ctx.t(
                "Aucune : ce programme n'emprunte rien: il est statique, ou il ne parle \
                 qu'au noyau.",
                "None: this program borrows nothing — it is static, or it only talks to \
                 the kernel.",
            ));
            return;
        }
        ui.strong(format!(
            "{} {}",
            self.imports.len(),
            ctx.t("fonctions importées", "imported functions")
        ));
        ui.weak(ctx.t(
            "Chaque entrée est une petite fonction qui saute là où l'éditeur de liens \
             aura écrit la vraie adresse. C'est pour cela qu'un `call` vers la \
             bibliothèque C passe par le programme lui-même.",
            "Each entry is a small function jumping to wherever the linker will have \
             written the real address. That is why a call into the C library goes \
             through the program itself.",
        ));
        // Le portrait avant la liste : « qu'est-ce que ce binaire fait »
        // se répond en six cases, là où cinquante noms demandent de les
        // connaître un par un.
        let comptes: Vec<(Family, usize)> = behaviour::ALL
            .into_iter()
            .filter_map(|famille| {
                let combien = self
                    .imports
                    .iter()
                    .filter(|i| behaviour::family_of(&i.name) == Some(famille))
                    .count();
                (combien > 0).then_some((famille, combien))
            })
            .collect();
        if !comptes.is_empty() {
            ui.add_space(4.0);
            portrait(ui, ctx, &comptes);
        }
        ui.add_space(4.0);

        // Par nombre d'appels : ce qui sert partout d'abord, le reste
        // par ordre alphabétique pour s'y retrouver.
        let mut rangs: Vec<usize> = (0..self.imports.len()).collect();
        rangs.sort_by(|&a, &b| {
            self.appels[b]
                .cmp(&self.appels[a])
                .then_with(|| self.imports[a].name.cmp(&self.imports[b].name))
        });
        for rang in rangs {
            let import = &self.imports[rang];
            let appels = self.appels[rang];
            let note = libc::explain(&import.name);
            ui.horizontal_wrapped(|ui| {
                ui.monospace(format!("{:#x}", import.plt));
                ui.strong(&import.name);
                if appels > 0 {
                    ui.weak(format!(
                        "{appels} {}",
                        if appels > 1 {
                            ctx.t("appels", "calls")
                        } else {
                            ctx.t("appel", "call")
                        }
                    ));
                }
                // Ce qu'on cherche vraiment en lisant la liste : par où
                // une entrée peut-elle déborder ? Ce n'est pas un
                // verdict sur le programme, c'est l'endroit qu'on
                // regarde en premier.
                if note.is_some_and(|note| note.watch) {
                    ui.label(
                        egui::RichText::new(ctx.t("à regarder", "worth a look"))
                            .color(egui::Color32::from_rgb(0xd7, 0xba, 0x7d)),
                    );
                }
                // Et sa famille de comportement, quand elle en a une :
                // c'est ce qui relie une ligne au portrait du dessus.
                if let Some(famille) = behaviour::family_of(&import.name) {
                    ui.label(egui::RichText::new(famille.label(ctx.lang)).color(famille.color()))
                        .on_hover_text(famille.why(ctx.lang));
                }
            });
            if let Some(note) = note {
                ui.indent(&import.name, |ui| ui.weak(note.text(ctx.lang)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// Sur un vrai binaire, le module nomme ce qui est emprunté *et*
    /// compte les appels qui y mènent.
    ///
    /// Le comptage est la partie fragile : les noms viennent des
    /// sections — des adresses virtuelles — et le désassemblage part du
    /// premier octet du fichier. Sans le rapprochement par position dans
    /// le fichier, tous les comptes resteraient à zéro sans que rien ne
    /// le signale.
    #[test]
    fn a_system_binary_shows_what_it_calls_and_how_often() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        // Comme l'interface : le fichier entier, désassemblé depuis son
        // premier octet, posé à l'adresse de travail de namsdec.
        let taille = bytes.len().min(0x10000);
        let disasm = namsdec_disasm::disassemble(&bytes[..taille], 0x400000);
        let mem = Memory::new(0x400000, 0x1000);
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &bytes,
            lang: "fr",
        };

        let mut module = ImportsModule::default();
        module.on_binary_loaded(&ctx);

        let noms: Vec<&str> = module
            .imports()
            .iter()
            .map(|import| import.name.as_str())
            .collect();
        assert!(noms.contains(&"malloc"), "{noms:?}");
        assert!(noms.contains(&"free"), "{noms:?}");
        assert_eq!(module.calls().len(), module.imports().len());
        assert!(
            module.calls().iter().sum::<usize>() > 50,
            "un binaire de cette taille appelle ses imports : {:?}",
            module.calls()
        );
    }

    /// Un programme qui n'emprunte rien ne se voit pas reprocher de
    /// tables manquantes : la liste est vide, et c'est tout.
    #[test]
    fn a_program_that_borrows_nothing_shows_nothing() {
        let bytes: &[u8] = &[0x90, 0xc3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: bytes,
            lang: "fr",
        };
        let mut module = ImportsModule::default();
        module.on_binary_loaded(&ctx);
        assert!(module.imports().is_empty());
    }
}
