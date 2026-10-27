//! Un panneau par outil extérieur : Ghidra, RetDec, radare2, objdump,
//! ndisasm.
//!
//! La fenêtre d'analyse ne portait qu'une rubrique de décompilation,
//! celle de DZF — le moteur maison, encore en chantier. Les outils que
//! namsdec sait pourtant déclarer et lancer n'avaient, eux, qu'une
//! fenêtre à part, avec une liste déroulante : pour comparer ce que
//! Ghidra et RetDec disent d'une même fonction, il fallait relancer en
//! changeant de choix, et la sortie précédente était perdue.
//!
//! Ici, chacun a sa rubrique. On les lance l'un après l'autre, et l'on
//! revient lire celle qu'on veut : **chaque panneau garde sa sortie**.
//!
//! Trois règles, les mêmes que pour la fenêtre des décompilateurs, et
//! elles sont le sujet de ce module autant que le pseudo-code qu'il
//! affiche :
//!
//! - **Rien ne s'exécute tout seul.** Ouvrir la rubrique ne lance rien ;
//!   l'outil ne part qu'au clic sur « Lancer ».
//! - **La commande est annoncée avant d'être lancée**, en toutes
//!   lettres : c'est ce qu'on s'apprête à exécuter sur sa propre
//!   machine.
//! - **namsdec n'installe rien à votre place.** Si l'outil manque, il
//!   dit ce qu'il faut taper, et laisse taper.

use namsdec_disasm::known::{Availability, Known};

use crate::dzf_decompiler_module::pseudocode_cliquable;
use crate::{ReContext, ReModule, ReRequest, ReResult, ReSection, framed, framed_small_button};

/// Combien de lignes une page de sortie porte.
///
/// La fenêtre d'analyse a sa propre zone de défilement et le panneau
/// n'en pose pas une seconde — deux zones imbriquées ne défilent bien
/// ni l'une ni l'autre. Un désassemblage d'objdump fait des dizaines de
/// milliers de lignes : on les peint donc par pages.
const PAR_PAGE: usize = 300;

/// Le panneau d'un outil extérieur.
pub struct ToolModule {
    /// La fiche de l'outil : son nom, sa commande, ses arguments, la
    /// façon de l'obtenir.
    known: Known,
    /// Le titre de la rubrique, dans les deux langues.
    titre_fr: &'static str,
    titre_en: &'static str,
    /// Le nom court, celui que la colonne de navigation écrit.
    court: &'static str,
    /// Ce que namsdec sait de l'outil sur cette machine. C'est
    /// l'application qui le dit — elle seule connaît le dossier où
    /// namsdec installe ce qu'il bâtit.
    disponible: Availability,
    /// Un travail est-il en cours ?
    en_cours: bool,
    /// Ce que l'outil a rendu, découpé en lignes.
    lignes: Vec<String>,
    /// Son refus, le cas échéant.
    erreur: Option<String>,
    /// Ce qu'on cherche dans la sortie.
    filtre: String,
    filtre_appliqué: String,
    /// Les rangs des lignes qui passent le filtre.
    visibles: Vec<usize>,
    /// La page qu'on regarde.
    page: usize,
    demande: Option<ReRequest>,
}

impl ToolModule {
    /// Le panneau d'un outil connu, avec le titre de sa rubrique.
    pub fn new(
        known: Known,
        titre_fr: &'static str,
        titre_en: &'static str,
        court: &'static str,
    ) -> Self {
        Self {
            known,
            titre_fr,
            titre_en,
            court,
            disponible: Availability::Missing,
            en_cours: false,
            lignes: Vec::new(),
            erreur: None,
            filtre: String::new(),
            filtre_appliqué: String::new(),
            visibles: Vec::new(),
            page: 0,
            demande: None,
        }
    }

    /// Le panneau de l'outil que namsdec connaît sous cet identifiant.
    ///
    /// `None` quand aucun ne porte ce nom : la liste des outils connus
    /// est une donnée, et une faute de frappe ne doit pas donner un
    /// panneau muet.
    pub fn connu(
        id: &str,
        titre_fr: &'static str,
        titre_en: &'static str,
        court: &'static str,
    ) -> Option<Self> {
        let known = namsdec_disasm::known::all()
            .into_iter()
            .find(|outil| outil.plugin.id == id)?;
        Some(Self::new(known, titre_fr, titre_en, court))
    }

    /// L'identifiant de l'outil que ce panneau lance.
    pub fn tool_id(&self) -> &str {
        &self.known.plugin.id
    }

    /// La sortie telle qu'elle est, pour qui veut la lire sans
    /// l'interface.
    pub fn lines(&self) -> &[String] {
        &self.lignes
    }

    fn poser(&mut self, texte: &str) {
        self.lignes = texte.lines().map(str::to_owned).collect();
        self.erreur = None;
        self.filtre.clear();
        self.filtre_appliqué.clear();
        self.visibles = (0..self.lignes.len()).collect();
        self.page = 0;
    }
}

impl ReModule for ToolModule {
    fn section(&self) -> ReSection {
        ReSection::Decompile
    }

    fn id(&self) -> &'static str {
        // Un identifiant stable et statique, comme les autres panneaux.
        // Celui de l'outil est une `String` venue de sa fiche : on la
        // retrouve, et l'on rend la constante qui lui correspond.
        match self.known.plugin.id.as_str() {
            "objdump" => "tool_objdump",
            "ndisasm" => "tool_ndisasm",
            "radare2" => "tool_radare2",
            "rizin-ghidra" => "tool_ghidra",
            "retdec" => "tool_retdec",
            _ => "tool_autre",
        }
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.titre_fr
        } else {
            self.titre_en
        }
    }

    /// Le nom de l'outil, tout court : c'est ainsi qu'on le cherche
    /// dans la colonne, et le panneau garde son titre entier.
    fn nav_title(&self, _lang: &str) -> &'static str {
        self.court
    }

    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn on_binary_loaded(&mut self, _ctx: &ReContext) {
        // Un autre binaire : la sortie d'avant ne le concerne plus. La
        // laisser à l'écran ferait lire le désassemblage d'un programme
        // en croyant lire celui d'un autre.
        self.lignes.clear();
        self.visibles.clear();
        self.erreur = None;
        self.en_cours = false;
        self.page = 0;
    }

    fn deliver(&mut self, result: &ReResult) {
        match result {
            ReResult::ToolState { tool, availability } if *tool == self.known.plugin.id => {
                self.disponible = *availability;
            }
            ReResult::ToolOutput { tool, text } if *tool == self.known.plugin.id => {
                self.en_cours = false;
                match text {
                    Ok(texte) => self.poser(texte),
                    Err(message) => self.erreur = Some(message.clone()),
                }
            }
            _ => {}
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let fr = ctx.lang.starts_with("fr");
        ui.weak(if fr {
            self.known.about_fr
        } else {
            self.known.about_en
        });
        ui.add_space(4.0);

        // CE QU'ON S'APPRÊTE À LANCER, en toutes lettres. Un panneau
        // qui exécute un programme sur la machine de quelqu'un doit
        // pouvoir se lire avant d'être cliqué.
        ui.horizontal_wrapped(|ui| {
            ui.weak(ctx.t("La commande :", "The command:"));
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!(
                        "{} {}",
                        self.known.plugin.command,
                        self.known.plugin.args.join(" ")
                    ))
                    .monospace(),
                )
                .selectable(true),
            );
        });
        ui.weak(ctx.t(
            "`{file}` reçoit le binaire ouvert, par un fichier temporaire. Aucun \
             interpréteur de commandes n'est en jeu.",
            "`{file}` gets the open binary, through a temporary file. No shell is \
             involved.",
        ));
        ui.add_space(6.0);

        match self.disponible {
            Availability::Ready => {}
            Availability::Incomplete => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    ctx.t(
                        "L'outil est là, mais il lui manque une pièce :",
                        "The tool is here, but a piece is missing:",
                    ),
                );
                if let Some(manque) = if fr {
                    self.known.needs_fr
                } else {
                    self.known.needs_en
                } {
                    ui.indent("manque", |ui| ui.weak(manque));
                }
            }
            Availability::Missing => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    ctx.t(
                        "Cet outil n'est pas sur cette machine.",
                        "This tool is not on this machine.",
                    ),
                );
                if let Some(install) = &self.known.plugin.install {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(ctx.t("Pour l'obtenir :", "To get it:"));
                        ui.add(
                            egui::Label::new(egui::RichText::new(install).monospace())
                                .selectable(true),
                        );
                    });
                }
                ui.weak(ctx.t(
                    "namsdec ne l'installe pas à votre place : exécuter du code au nom de \
                     quelqu'un n'est pas une commodité, c'est une décision. Outils › \
                     Désassembleurs en dit plus.",
                    "namsdec does not install it for you: running code on someone's behalf \
                     is not a convenience, it is a decision. Tools › Disassemblers says \
                     more.",
                ));
            }
        }

        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            let lançable = self.disponible != Availability::Missing && !self.en_cours;
            if ui
                .add_enabled(
                    lançable,
                    framed(
                        ui,
                        if self.lignes.is_empty() {
                            ctx.t("Lancer", "Run")
                        } else {
                            ctx.t("Relancer", "Run again")
                        },
                    ),
                )
                .on_hover_text(ctx.t(
                    "Lancer cet outil sur le binaire ouvert, et lire sa sortie ici.",
                    "Run this tool on the open binary, and read its output here.",
                ))
                .clicked()
            {
                self.en_cours = true;
                self.erreur = None;
                self.demande = Some(ReRequest::RunTool {
                    tool: self.known.plugin.id.clone(),
                });
            }
            if self.en_cours {
                ui.spinner();
                ui.weak(ctx.t(
                    "L'outil travaille — un décompilateur peut réfléchir une minute.",
                    "The tool is working — a decompiler may think for a minute.",
                ));
            }
            if !self.lignes.is_empty() {
                ui.weak(format!(
                    "{} {}",
                    self.lignes.len(),
                    ctx.t("lignes", "lines")
                ));
            }
        });

        if let Some(message) = &self.erreur {
            ui.add_space(4.0);
            ui.colored_label(ui.visuals().error_fg_color, message);
        }
        if self.lignes.is_empty() {
            return;
        }

        // Chercher dans la sortie : un désassemblage d'objdump fait
        // cinquante mille lignes, et ce qu'on y cherche tient en un mot.
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.weak(ctx.t("Chercher :", "Find:"));
            ui.add(
                egui::TextEdit::singleline(&mut self.filtre)
                    .desired_width(200.0)
                    .hint_text(ctx.t("un nom, une adresse…", "a name, an address…")),
            );
            if !self.filtre.is_empty() && framed_small_button(ui, "x").clicked() {
                self.filtre.clear();
            }
        });
        if self.filtre != self.filtre_appliqué {
            let cherché = self.filtre.to_lowercase();
            self.visibles = if cherché.is_empty() {
                (0..self.lignes.len()).collect()
            } else {
                self.lignes
                    .iter()
                    .enumerate()
                    .filter(|(_, ligne)| ligne.to_lowercase().contains(&cherché))
                    .map(|(rang, _)| rang)
                    .collect()
            };
            self.filtre_appliqué = self.filtre.clone();
            self.page = 0;
        }

        let pages = self.visibles.len().div_ceil(PAR_PAGE).max(1);
        self.page = self.page.min(pages - 1);
        let début = self.page * PAR_PAGE;
        let fin = (début + PAR_PAGE).min(self.visibles.len());
        ui.horizontal_wrapped(|ui| {
            if self.visibles.is_empty() {
                ui.label(ctx.t("Aucune ligne ne correspond", "No line matches"));
            } else {
                ui.label(format!(
                    "{}–{} {} {}",
                    début + 1,
                    fin,
                    ctx.t("sur", "of"),
                    self.visibles.len()
                ));
            }
            if pages > 1 {
                let première = self.page == 0;
                let dernière = self.page + 1 >= pages;
                if ui
                    .add_enabled(!première, framed(ui, "\u{2039}"))
                    .on_hover_text(ctx.t("Page précédente", "Previous page"))
                    .clicked()
                {
                    self.page -= 1;
                }
                ui.label(format!(
                    "{} {} / {}",
                    ctx.t("page", "page"),
                    self.page + 1,
                    pages
                ));
                if ui
                    .add_enabled(!dernière, framed(ui, "\u{203a}"))
                    .on_hover_text(ctx.t("Page suivante", "Next page"))
                    .clicked()
                {
                    self.page += 1;
                }
            }
        });
        ui.add_space(4.0);

        // La sortie, peinte en une galère par ligne, les adresses en
        // couleur de lien : `0x401030`, `sub_401030`, `401030h` mènent
        // au listing, comme dans le panneau DZF. C'est ce qui fait la
        // différence entre lire la sortie d'un outil et s'en servir.
        let mut aller = None;
        // Bornée à la largeur du panneau : une ligne d'`objdump` fait
        // parfois deux cents caractères, et peinte sans borne elle
        // court hors du panneau — où rien ne la rattrape, puisque la
        // fenêtre d'analyse ne défile pas en largeur.
        let largeur = ui.available_width().max(80.0);
        for rang in &self.visibles[début..fin] {
            if let Some(addr) = pseudocode_cliquable(ui, &self.lignes[*rang], largeur) {
                aller = Some(addr);
            }
        }
        if let Some(addr) = aller {
            // Vers le CODE, sans demander : une adresse lue dans la
            // sortie d'un désassembleur désigne une instruction, et le
            // choix ne se pose que pour une adresse dont on ignore ce
            // qu'elle porte.
            self.demande = Some(ReRequest::GoTo {
                addr,
                target: crate::GoTarget::Listing,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn contexte<'a>(cpu: &'a Cpu) -> ReContext<'a> {
        ReContext {
            cpu,
            disasm: &[],
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        }
    }

    fn peindre(module: &mut ToolModule, ctx: &ReContext) -> String {
        let egui_ctx = egui::Context::default();
        let mut textes: Vec<String> = Vec::new();
        for _ in 0..2 {
            textes.clear();
            let mut sortie = egui_ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ui| module.ui(ui, ctx),
            );
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    textes.push(texte.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        textes.join(" | ")
    }

    /// Chaque outil connu a son panneau, et son panneau dit ce qu'il
    /// lancerait — sans rien lancer.
    #[test]
    fn a_panel_says_what_it_would_run_without_running_it() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu(
            "objdump",
            "Désassemblage par objdump",
            "Disassembly by objdump",
            "objdump",
        )
        .expect("objdump est un outil connu");
        assert_eq!(module.id(), "tool_objdump");
        assert_eq!(module.title("fr"), "Désassemblage par objdump");
        assert_eq!(module.title("en"), "Disassembly by objdump");
        // La colonne, elle, écrit le nom tout court : vingt-cinq
        // rubriques sur deux cents pixels ne tiennent pas autrement.
        assert_eq!(module.nav_title("fr"), "objdump");

        let peint = peindre(&mut module, &ctx);
        // La commande est écrite avant d'être lancée.
        assert!(peint.contains("objdump -d -M intel"), "{peint}");
        // Et rien n'a été lancé : aucune sortie, aucune demande.
        assert!(module.lines().is_empty());
        assert!(module.demande.is_none());
    }

    /// Un outil absent le dit, et dit comment l'obtenir — plutôt que de
    /// laisser cliquer un bouton qui ne peut pas marcher.
    #[test]
    fn a_missing_tool_says_how_to_get_it() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu(
            "retdec",
            "Décompilation par RetDec",
            "Decompilation by RetDec",
            "RetDec",
        )
        .expect("RetDec est un outil connu");
        // L'état par défaut est « absent » : c'est l'application qui dit
        // le contraire, après avoir regardé le disque.
        let peint = peindre(&mut module, &ctx);
        assert!(peint.contains("n'est pas sur cette machine"), "{peint}");
        assert!(peint.contains("github.com/avast/retdec"), "{peint}");
    }

    /// Ce que l'outil rend arrive par le canal des résultats, et le
    /// panneau le garde : on lance Ghidra, puis RetDec, et l'on revient
    /// lire le premier.
    #[test]
    fn the_output_arrives_and_stays() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu(
            "radare2",
            "Décompilation par radare2",
            "Decompilation by radare2",
            "radare2",
        )
        .expect("radare2 est un outil connu");
        module.deliver(&ReResult::ToolState {
            tool: "radare2".to_owned(),
            availability: Availability::Ready,
        });
        // Ce qui s'adresse à un autre outil ne le concerne pas.
        module.deliver(&ReResult::ToolOutput {
            tool: "objdump".to_owned(),
            text: Ok("rien pour moi".to_owned()),
        });
        assert!(module.lines().is_empty());

        module.deliver(&ReResult::ToolOutput {
            tool: "radare2".to_owned(),
            text: Ok("entry0 ()\n    call 0x401030\n".to_owned()),
        });
        assert_eq!(module.lines().len(), 2);
        let peint = peindre(&mut module, &ctx);
        assert!(peint.contains("call 0x401030"), "{peint}");
        assert!(peint.contains("1–2 sur 2"), "{peint}");

        // Et un refus s'affiche tel quel, plutôt que de laisser le
        // panneau muet.
        module.deliver(&ReResult::ToolOutput {
            tool: "radare2".to_owned(),
            text: Err("l'outil a été arrêté au bout de 120 secondes".to_owned()),
        });
        let peint = peindre(&mut module, &ctx);
        assert!(peint.contains("arrêté au bout de"), "{peint}");
    }

    /// Une sortie aux lignes très longues reste dans le panneau.
    ///
    /// Une ligne d'`objdump` fait parfois deux cents caractères.
    /// Peinte sans borne, elle court hors du panneau — et la fenêtre
    /// d'analyse ne défile pas en largeur, donc ce qui dépasse est
    /// perdu, quand il ne fait pas enfler la fenêtre.
    #[test]
    fn a_very_long_line_stays_inside_the_panel() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu("objdump", "objdump", "objdump", "objdump")
            .expect("objdump est un outil connu");
        module.deliver(&ReResult::ToolState {
            tool: "objdump".to_owned(),
            availability: Availability::Ready,
        });
        let longue = format!(
            "  401030:\t{}\t{}",
            "48 8d 3d 10 00 00 00 ".repeat(8),
            "lea rdi, [rip + 0x10] ; un commentaire très long qui n'en finit pas de finir"
        );
        module.deliver(&ReResult::ToolOutput {
            tool: "objdump".to_owned(),
            text: Ok(longue.clone()),
        });

        let largeur = 600.0;
        let egui_ctx = egui::Context::default();
        let mut dehors: Vec<(String, f32)> = Vec::new();
        for _ in 0..2 {
            dehors.clear();
            let mut sortie = egui_ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(largeur, 1600.0),
                    )),
                    ..Default::default()
                },
                |ui| module.ui(ui, &ctx),
            );
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    for ligne in &t.galley.rows {
                        if ligne.glyphs.is_empty() {
                            continue;
                        }
                        let posée = ligne
                            .rect_without_leading_space()
                            .translate(t.pos.to_vec2());
                        if posée.right() > largeur + 0.5 {
                            dehors.push((t.galley.text().to_owned(), posée.right()));
                        }
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        assert!(
            dehors.is_empty(),
            "du texte est posé hors du panneau : {dehors:?}"
        );
    }

    /// Le bouton « Lancer » pose une demande, et c'est l'application qui
    /// lance : un module ne sort jamais du bac à sable lui-même.
    #[test]
    fn running_is_asked_for_never_done_here() {
        let mut module =
            ToolModule::connu("objdump", "objdump", "objdump", "objdump").expect("connu");
        module.deliver(&ReResult::ToolState {
            tool: "objdump".to_owned(),
            availability: Availability::Ready,
        });
        // On simule le clic en posant la demande comme le bouton le
        // fait : ce qu'on vérifie ici est le CONTRAT — une demande
        // nommée, relevée une fois, et rien d'autre.
        module.demande = Some(ReRequest::RunTool {
            tool: "objdump".to_owned(),
        });
        assert_eq!(
            module.take_request(),
            Some(ReRequest::RunTool {
                tool: "objdump".to_owned()
            })
        );
        assert_eq!(module.take_request(), None, "relevée une seule fois");
    }
}
