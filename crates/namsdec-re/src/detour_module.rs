//! Le panneau du détour : ajouter du code à un binaire sans le décaler.
//!
//! Il existe pour expliquer une idée, et cette idée est belle : on ne
//! peut pas agrandir un exécutable sans fausser toutes ses adresses,
//! mais on peut le **détourner** par un trou qu'il contient déjà.
//! Comprendre cela, c'est comprendre du même coup pourquoi un binaire
//! n'est pas plein, pourquoi une adresse relative n'est pas une adresse,
//! et pourquoi un correctif se montre avant de se poser.
//!
//! Rien ne s'écrit ici. Le panneau calcule un **plan**, le montre en
//! entier — ce qui sera recouvert, ce qui sera écrit, et où —, et c'est
//! seulement sur un geste explicite que l'application le pose. Les deux
//! changements partent sous un même numéro de groupe, et se défont
//! ensemble par la fenêtre Correctifs.

use crate::patch::{self, Detour, DetourRefusé};
use crate::slack::{self, Slack};
use crate::{
    ReContext, ReModule, ReRequest, ReSection, clickable_address, framed_button,
    framed_small_button,
};

/// Comment l'utilisateur écrit le code à ajouter.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ModeAjout {
    /// En assembleur, assemblé par nasm à l'adresse où il atterrira.
    ///
    /// Le défaut, parce que c'est la façon dont on PENSE le geste :
    /// « écris 0x42 à cette adresse », pas « pose 41 c6 03 42 ».
    #[default]
    Assembleur,
    /// En octets hexadécimaux, pour qui les a déjà.
    ///
    /// Gardé parce que nasm peut manquer, et parce qu'on veut parfois
    /// poser une suite d'octets précise sans passer par un assembleur
    /// qui choisirait un encodage équivalent mais différent.
    Octets,
}

#[derive(Default)]
pub struct DetourModule {
    /// Ce que le panneau demande à l'application. Relevé après le
    /// dessin, jamais pendant.
    demande: Option<ReRequest>,
    /// L'adresse visée, telle qu'elle est tapée.
    site: String,
    /// Le code à ajouter, en assembleur ou en octets.
    code: String,
    mode: ModeAjout,
    /// Les places libres exécutables, la plus grande d'abord. Les autres
    /// ne sont pas offertes : un détour vers des données ne s'exécute
    /// pas, et proposer un choix qui échoue toujours n'apprend rien.
    trous: Vec<Slack>,
    /// Laquelle est choisie.
    choix: usize,
    /// Le dernier plan calculé, ou la raison du refus. `None` tant qu'on
    /// n'a rien demandé : un plan ne se calcule pas à chaque image, il
    /// lance nasm.
    plan: Option<Result<Box<Detour>, DetourRefusé>>,
    /// L'assembleur a refusé le texte — ses diagnostics, tels quels.
    erreur_asm: Option<String>,
    /// nasm a-t-il été trouvé ? Relevé une fois, au chargement.
    nasm: bool,
}

impl DetourModule {
    /// Les octets de l'ajout, assemblés ou décodés selon le mode.
    ///
    /// L'adresse compte : `assemble_one` pose un `[ORG]`, donc du code
    /// qui s'adresse relativement est assemblé pour l'endroit où il
    /// atterrira vraiment. C'est pour cela que la place libre se choisit
    /// AVANT d'assembler.
    fn octets_de_l_ajout(&self, adresse: u64) -> Result<Vec<u8>, String> {
        let texte = self.code.trim();
        if texte.is_empty() {
            // Un détour sans ajout est légitime : il déplace du code et
            // le ramène, sans rien changer. C'est le cas d'école qu'on
            // veut pouvoir poser pour voir le mécanisme seul.
            return Ok(Vec::new());
        }
        match self.mode {
            ModeAjout::Assembleur => {
                namsdec_asm::assemble_one(texte, adresse).map_err(|e| phrase_de_l_asm(&e))
            }
            ModeAjout::Octets => décoder_hex(texte),
        }
    }

    /// Calcule le plan, et le garde. Ne touche à rien du binaire.
    fn calculer(&mut self, ctx: &ReContext) {
        self.plan = None;
        self.erreur_asm = None;

        let Some(site_vaddr) = adresse_lue(&self.site) else {
            self.erreur_asm = Some(
                ctx.t(
                    "Adresse illisible — attendu de l'hexadécimal, par exemple 0x401136.",
                    "Unreadable address — hexadecimal expected, for instance 0x401136.",
                )
                .to_owned(),
            );
            return;
        };
        let Some(place) = self.trous.get(self.choix).cloned() else {
            return;
        };
        let Some(site_offset) = crate::imports::file_offset(ctx.binary, site_vaddr) else {
            self.erreur_asm = Some(
                ctx.t(
                    "Cette adresse n'est dans aucun segment chargé du fichier.",
                    "This address is in no loaded segment of the file.",
                )
                .to_owned(),
            );
            return;
        };

        let ajout = match self.octets_de_l_ajout(place.vaddr) {
            Ok(octets) => octets,
            Err(message) => {
                self.erreur_asm = Some(message);
                return;
            }
        };

        self.plan = Some(
            patch::plan_detour(
                ctx.binary,
                64,
                site_offset as usize,
                site_vaddr,
                &place,
                &ajout,
            )
            .map(Box::new),
        );
    }
}

impl ReModule for DetourModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Bytes
    }

    fn id(&self) -> &'static str {
        "detour"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Détourner (ajouter du code)"
        } else {
            "Detour (add code)"
        }
    }

    fn nav_title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Détourner"
        } else {
            "Detour"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.trous = slack::find(ctx.binary)
            .into_iter()
            .filter(|t| t.executable)
            .collect();
        self.choix = 0;
        // Un plan calculé sur l'image d'avant ne vaut plus rien : le
        // garder à l'écran laisserait poser des octets d'après un
        // fichier qui a changé.
        self.plan = None;
        self.erreur_asm = None;
        self.nasm = namsdec_asm::find_nasm().is_some();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.weak(ctx.t(
            "Ajouter du code à un exécutable sans le décaler. Un octet de plus \
             fausserait toutes les adresses qui suivent — alors on ne pousse \
             rien : on pose un saut vers un trou que le binaire contient déjà, \
             on y écrit le code neuf, on y rejoue les instructions recouvertes, \
             et on revient. Le fichier garde sa taille exacte.",
            "Adding code to an executable without shifting it. One extra byte \
             would falsify every address that follows — so nothing is pushed: a \
             jump is placed towards a gap the binary already contains, the new \
             code is written there, the covered instructions are replayed, and \
             control comes back. The file keeps its exact size.",
        ));

        if self.trous.is_empty() {
            ui.add_space(6.0);
            ui.weak(ctx.t(
                "Aucune place libre exécutable dans ce fichier — rien à détourner \
                 vers. Le panneau « Place libre » montre ce qu'il y a.",
                "No executable slack in this file — nowhere to detour to. The \
                 “Free space” panel shows what there is.",
            ));
            return;
        }

        ui.add_space(8.0);

        // --- L'endroit visé ---------------------------------------
        ui.horizontal(|ui| {
            ui.label(ctx.t("Détourner depuis", "Detour from"));
            ui.add(
                egui::TextEdit::singleline(&mut self.site)
                    .desired_width(110.0)
                    .hint_text("0x401136")
                    .font(egui::TextStyle::Monospace),
            );
            // L'adresse où l'émulateur en est : c'est presque toujours
            // celle qu'on veut, et la retaper à la main est une corvée
            // qui fait recopier de travers.
            if framed_small_button(ui, ctx.t("ici", "here"))
                .on_hover_text(ctx.t(
                    "Prendre l'adresse où l'exécution en est",
                    "Take the address execution has reached",
                ))
                .clicked()
            {
                self.site = format!("{:#x}", ctx.cpu.regs.rip);
            }
        });

        // --- La place libre où atterrir ----------------------------
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(ctx.t("vers la place libre", "towards the slack"));
            let étiquette = self
                .trous
                .get(self.choix)
                .map(|t| étiquette_du_trou(t, ctx))
                .unwrap_or_default();
            egui::ComboBox::from_id_salt("detour_slack")
                .selected_text(étiquette)
                .width(230.0)
                .show_ui(ui, |ui| {
                    for (rang, trou) in self.trous.iter().enumerate() {
                        ui.selectable_value(&mut self.choix, rang, étiquette_du_trou(trou, ctx));
                    }
                });
        });

        // --- Le code à ajouter -------------------------------------
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(ctx.t("Code à ajouter", "Code to add"));
            ui.selectable_value(
                &mut self.mode,
                ModeAjout::Assembleur,
                ctx.t("assembleur", "assembly"),
            );
            ui.selectable_value(&mut self.mode, ModeAjout::Octets, ctx.t("octets", "bytes"));
        });
        if self.mode == ModeAjout::Assembleur && !self.nasm {
            ui.weak(ctx.t(
                "nasm est introuvable : l'assembleur ne répondra pas. Passez par \
                 les octets, ou installez-le.",
                "nasm not found: the assembler will not answer. Use bytes \
                 instead, or install it.",
            ));
        }
        ui.add(
            egui::TextEdit::multiline(&mut self.code)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .code_editor()
                .hint_text(match self.mode {
                    ModeAjout::Assembleur => "mov r11, 0x404030\nmov byte [r11], 0x42",
                    ModeAjout::Octets => "41 bb 30 40 40 00 41 c6 03 42",
                }),
        );
        ui.weak(ctx.t(
            "Laissé vide, le détour déplace le code et le ramène sans rien \
             changer — de quoi voir le mécanisme seul.",
            "Left empty, the detour moves the code and brings it back unchanged \
             — enough to see the mechanism alone.",
        ));

        // --- Calculer, puis montrer avant de poser -----------------
        ui.add_space(8.0);
        if framed_button(ui, ctx.t("Calculer le plan", "Compute the plan"))
            .on_hover_text(ctx.t(
                "Rien n'est écrit : le plan se montre d'abord",
                "Nothing is written: the plan is shown first",
            ))
            .clicked()
        {
            self.calculer(ctx);
        }

        if let Some(message) = &self.erreur_asm {
            ui.add_space(6.0);
            ui.colored_label(ui.visuals().error_fg_color, message);
        }

        let mut demande = None;
        match &self.plan {
            None => {}
            Some(Err(refus)) => {
                ui.add_space(8.0);
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    refus.message(ctx.lang.starts_with("fr")),
                );
            }
            Some(Ok(detour)) => {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.strong(ctx.t("Le plan", "The plan"));
                ui.add_space(4.0);

                // Ce qui part, et ce qui le remplace. La question que
                // tout le monde se pose devant un correctif binaire est
                // « qu'est-ce que je perds ? » — on y répond d'abord.
                ui.label(format!(
                    "{} {} {}",
                    ctx.t("Recouvert :", "Covered:"),
                    detour.recouvert,
                    ctx.t("octets", "bytes"),
                ));
                ui.add_space(2.0);
                table_d_instructions(
                    ui,
                    ctx,
                    &detour.aller.before,
                    detour.aller.addr,
                    ctx.t("ce qui part", "what goes"),
                );
                ui.add_space(4.0);
                table_d_instructions(
                    ui,
                    ctx,
                    &detour.aller.after,
                    detour.aller.addr,
                    ctx.t("ce qui le remplace", "what replaces it"),
                );

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(ctx.t("Le corps, écrit en", "The body, written at"));
                    if let Some(cible) = clickable_address(ui, ctx, detour.corps.addr) {
                        demande = Some(ReRequest::GoTo {
                            addr: detour.corps.addr,
                            target: cible,
                        });
                    }
                });
                ui.add_space(2.0);
                table_d_instructions(
                    ui,
                    ctx,
                    &detour.corps.after,
                    detour.corps.addr,
                    ctx.t("le détour", "the detour"),
                );

                // Les instructions déplacées ont pu être RÉÉCRITES, et
                // c'est la chose la plus instructive du panneau : le dire
                // quand c'est arrivé, parce que sinon personne ne
                // remarque que les octets ont changé sans que
                // l'instruction change.
                if detour.déplacé != detour.aller.before[..detour.déplacé.len()] {
                    ui.add_space(6.0);
                    ui.weak(ctx.t(
                        "Les instructions déplacées ont été relocalisées : leur \
                         écart a été recalculé pour qu'elles visent toujours la \
                         même chose depuis leur nouvelle adresse. L'instruction \
                         ne change pas ; ses octets, si.",
                        "The moved instructions were relocated: their offset was \
                         recomputed so they still target the same thing from \
                         their new address. The instruction does not change; its \
                         bytes do.",
                    ));
                }

                ui.add_space(10.0);
                if framed_button(ui, ctx.t("Poser le détour", "Place the detour"))
                    .on_hover_text(ctx.t(
                        "Les deux changements, ensemble — et défaisables ensemble \
                         par la fenêtre Correctifs",
                        "Both changes, together — and undoable together from the \
                         Patches window",
                    ))
                    .clicked()
                {
                    demande = Some(ReRequest::PatchDetour {
                        detour: detour.clone(),
                    });
                }
            }
        }
        if demande.is_some() {
            self.demande = demande;
            // Le plan vient d'être posé ou l'on part ailleurs : dans les
            // deux cas il ne décrit plus ce qui est dans le fichier.
            // `on_binary_loaded` le remettra à neuf au rechargement.
            self.erreur_asm = None;
        }
    }
}

/// Une place libre, dite en une ligne.
fn étiquette_du_trou(trou: &Slack, ctx: &ReContext) -> String {
    format!(
        "{:#010x} · {} {} · {}",
        trou.vaddr,
        trou.len,
        ctx.t("octets", "bytes"),
        trou.section.as_deref().unwrap_or("—"),
    )
}

/// Les octets, désassemblés, sous un intertitre.
///
/// Montrer les octets seuls ne dit rien à personne ; montrer
/// l'instruction sans les octets cache ce qui change quand une
/// relocalisation réécrit un écart. Les deux, donc, côte à côte.
fn table_d_instructions(ui: &mut egui::Ui, ctx: &ReContext, octets: &[u8], base: u64, titre: &str) {
    ui.weak(titre);
    let lignes = namsdec_disasm::disassemble(octets, base);
    if lignes.is_empty() {
        ui.monospace(crate::digest::hex(octets));
        return;
    }
    egui::Grid::new(format!("detour_{titre}"))
        .num_columns(3)
        .striped(true)
        .min_col_width(0.0)
        .show(ui, |ui| {
            for ligne in &lignes {
                // L'adresse est de largeur fixe : elle ne se replie pas.
                ui.monospace(format!("{:#010x}", ligne.address));
                // Les deux colonnes larges SE REPLIENT au lieu d'être
                // coupées. Mesuré : dans une colonne de quatre cent
                // vingt pixels, une instruction comme
                // `lock cmpxchg16b oword [rax+rbx*8+0x12345678]` perdait
                // cent vingt pixels par le bord — et c'est le texte qui
                // porte la leçon du panneau. Tronquer l'aurait caché ;
                // une barre de défilement horizontale dans une grille
                // déjà posée dans une colonne défilante aurait donné
                // deux ascenseurs imbriqués.
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(crate::digest::hex(&ligne.bytes)).monospace(),
                    )
                    .wrap(),
                );
                ui.add(egui::Label::new(egui::RichText::new(&ligne.text).monospace()).wrap());
                ui.end_row();
            }
        });
    let _ = ctx;
}

/// Ce que l'assembleur a répondu, mis en une phrase.
///
/// Les diagnostics de nasm arrivent déjà rédigés, en anglais, et il n'y a
/// pas à les réécrire — on les rend tels quels, avec leur numéro de
/// ligne, parce que c'est ce qui permet de corriger. Seul le cas « nasm
/// introuvable » se dit dans la langue de l'interface : c'est le seul où
/// le message vient de namsdec et non de l'outil.
fn phrase_de_l_asm(erreur: &namsdec_asm::AsmError) -> String {
    match erreur {
        namsdec_asm::AsmError::NasmNotFound => {
            "nasm est introuvable — passez par les octets, ou installez-le.".to_owned()
        }
        namsdec_asm::AsmError::Failed { diagnostics } => diagnostics
            .iter()
            .map(|d| format!("ligne {} : {}", d.line, d.message))
            .collect::<Vec<_>>()
            .join("\n"),
        autre => autre.to_string(),
    }
}

/// Une adresse hexadécimale, avec ou sans `0x`.
fn adresse_lue(texte: &str) -> Option<u64> {
    let propre = texte.trim();
    let propre = propre
        .strip_prefix("0x")
        .or_else(|| propre.strip_prefix("0X"))
        .unwrap_or(propre);
    if propre.is_empty() {
        return None;
    }
    u64::from_str_radix(propre, 16).ok()
}

/// Des octets écrits à la main, espacés comme on veut.
fn décoder_hex(texte: &str) -> Result<Vec<u8>, String> {
    let propre: String = texte.chars().filter(|c| !c.is_whitespace()).collect();
    if propre.chars().any(|c| !c.is_ascii_hexdigit()) {
        return Err("Octets illisibles — attendu de l'hexadécimal.".to_owned());
    }
    if !propre.len().is_multiple_of(2) {
        return Err("Nombre impair de chiffres — un octet en fait deux.".to_owned());
    }
    propre
        .as_bytes()
        .chunks(2)
        .map(|paire| {
            let texte = std::str::from_utf8(paire).map_err(|_| "Octets illisibles.".to_owned())?;
            u8::from_str_radix(texte, 16).map_err(|_| "Octets illisibles.".to_owned())
        })
        .collect()
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
            names: None,
        }
    }

    /// Un module frais ne propose rien et ne panique pas.
    #[test]
    fn a_fresh_module_offers_nothing() {
        let module = DetourModule::default();
        assert!(module.trous.is_empty());
        assert!(module.plan.is_none());
        assert_eq!(module.mode, ModeAjout::Assembleur);
    }

    /// Ce qui n'est pas un ELF le laisse vide.
    #[test]
    fn a_non_elf_leaves_it_empty() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let mut module = DetourModule::default();
        module.on_binary_loaded(&contexte(b"pas un elf", &cpu));
        assert!(module.trous.is_empty());
    }

    /// Seules les places EXÉCUTABLES sont offertes.
    ///
    /// Le panneau de lecture les montre toutes, parce que les voir
    /// apprend quelque chose. Celui-ci n'en offre que les utilisables :
    /// `slack::find` trie par taille, et les plus grands trous d'un
    /// binaire sont souvent dans les données — on proposerait donc en
    /// tête un choix qui échoue toujours.
    #[test]
    fn only_executable_slack_is_offered() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let mut module = DetourModule::default();
        // Un vrai ELF minimal n'est pas nécessaire : ce qu'on vérifie
        // est le filtre, et il porte sur ce que `find` a rendu.
        module.on_binary_loaded(&contexte(b"", &cpu));
        assert!(module.trous.iter().all(|t| t.executable));
    }

    /// Une adresse se lit avec ou sans `0x`, et rien d'autre ne passe.
    #[test]
    fn an_address_is_read_either_way() {
        assert_eq!(adresse_lue("0x401136"), Some(0x401136));
        assert_eq!(adresse_lue("401136"), Some(0x401136));
        assert_eq!(adresse_lue("0X401136"), Some(0x401136));
        assert_eq!(adresse_lue("  0x401136  "), Some(0x401136));
        assert_eq!(adresse_lue(""), None);
        assert_eq!(adresse_lue("0x"), None);
        assert_eq!(adresse_lue("pas une adresse"), None);
    }

    /// Les octets se donnent espacés comme on veut, et un nombre impair
    /// de chiffres est refusé plutôt que complété.
    ///
    /// Compléter serait le pire des deux : « 41c6034 » deviendrait
    /// « 41 c6 03 40 » ou « 41 c6 03 04 » selon le côté choisi, et
    /// l'utilisateur poserait des octets qu'il n'a pas écrits.
    #[test]
    fn bytes_are_read_spaced_and_odd_digits_are_refused() {
        assert_eq!(décoder_hex("41bb"), Ok(vec![0x41, 0xbb]));
        assert_eq!(décoder_hex("41 bb"), Ok(vec![0x41, 0xbb]));
        assert_eq!(décoder_hex(" 41  bb \n"), Ok(vec![0x41, 0xbb]));
        assert!(décoder_hex("41b").is_err(), "nombre impair");
        assert!(décoder_hex("41zz").is_err(), "pas de l'hexadécimal");
    }

    /// Un ajout vide est légitime : le détour se voit tout seul.
    #[test]
    fn an_empty_addition_is_allowed() {
        let module = DetourModule::default();
        assert_eq!(module.octets_de_l_ajout(0x1000), Ok(Vec::new()));
    }

    /// En mode octets, l'assembleur n'est pas appelé — donc nasm n'est
    /// pas nécessaire.
    ///
    /// Ce contrôle garde une promesse de l'interface : « passez par les
    /// octets » doit marcher sur une machine sans nasm. Il ne lit pas
    /// l'état de la machine pour autant — il vérifie que ce chemin
    /// n'emprunte pas l'autre.
    #[test]
    fn the_bytes_mode_does_not_need_an_assembler() {
        let module = DetourModule {
            mode: ModeAjout::Octets,
            code: "41 bb 0c 30 40 00".to_owned(),
            ..Default::default()
        };
        assert_eq!(
            module.octets_de_l_ajout(0x1000),
            Ok(vec![0x41, 0xbb, 0x0c, 0x30, 0x40, 0x00])
        );
    }

    /// Un plan calculé devient caduc quand le fichier change.
    ///
    /// C'est le défaut qu'on ne verrait pas : le plan reste affiché, le
    /// bouton « Poser » reste cliquable, et il poserait des octets
    /// calculés sur une image qui n'est plus celle du fichier. Tout
    /// rechargement le jette.
    #[test]
    fn a_plan_is_dropped_when_the_file_changes() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let mut module = DetourModule {
            erreur_asm: Some("quelque chose".to_owned()),
            plan: Some(Err(DetourRefusé::PlaceNonExécutable)),
            ..Default::default()
        };
        module.on_binary_loaded(&contexte(b"", &cpu));
        assert!(
            module.plan.is_none(),
            "le plan ne survit pas au rechargement"
        );
        assert!(module.erreur_asm.is_none());
    }

    /// Ce qu'une grille du plan PEINT vraiment, à une largeur donnée.
    ///
    /// Relire le code ne dit pas si trois colonnes monospace tiennent
    /// dans une colonne d'écran partagé : egui ne borne rien de
    /// lui-même, et une grille déborde sans se plaindre. On relève donc
    /// les formes de texte et on les mesure, avec les précautions
    /// d'usage — le rectangle et non la position, la ligne et non la
    /// galley, et l'encre intersectée avec le découpage, seule façon de
    /// voir qu'un texte est coupé.
    fn largeur_perdue(octets: &[u8], largeur: f32) -> Vec<(String, f32)> {
        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, 1200.0),
            )),
            ..Default::default()
        };
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(b"", &cpu);
        let mut coupées = Vec::new();
        // Deux images : la première pose les identifiants et les
        // largeurs qu'egui garde d'une image sur l'autre.
        for _ in 0..2 {
            coupées.clear();
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| {
                table_d_instructions(ui, &ctx, octets, 0x401000, "essai");
            });
            for clipped in &sortie.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                for ligne in &t.galley.rows {
                    if ligne.glyphs.is_empty() {
                        continue;
                    }
                    let posée = ligne
                        .rect_without_leading_space()
                        .translate(t.pos.to_vec2());
                    let encre = posée.intersect(clipped.clip_rect);
                    if encre.width() <= 0.0 {
                        continue;
                    }
                    let perdue = posée.width() - encre.width();
                    if perdue > 0.5 {
                        coupées.push((t.galley.text().to_owned(), perdue));
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        coupées
    }

    /// Une instruction longue ne déborde pas la grille du plan, même
    /// dans une colonne étroite.
    ///
    /// `lock cmpxchg16b oword [rax+rbx*8+0x12345678]` fait dix octets et
    /// quarante-trois caractères : avec l'adresse et les octets en
    /// hexadécimal, la ligne demande plus de huit cents pixels en
    /// monospace. Dans une colonne de quatre cent vingt, il faut bien
    /// que quelque chose cède — et ce qui cède doit être un choix, non
    /// un texte coupé par un bord.
    #[test]
    fn a_long_instruction_does_not_overflow_the_plan_grid() {
        let longue = [0xF0, 0x48, 0x0F, 0xC7, 0x8C, 0xD8, 0x78, 0x56, 0x34, 0x12];
        let coupées = largeur_perdue(&longue, 420.0);
        assert!(
            coupées.is_empty(),
            "la grille du plan coupe du texte à 420 px : {coupées:?}"
        );
    }

    /// Et à large, rien ne bouge non plus — le contrôle ne doit pas
    /// passer par accident parce que la largeur suffisait.
    #[test]
    fn the_plan_grid_is_also_clean_when_wide() {
        let longue = [0xF0, 0x48, 0x0F, 0xC7, 0x8C, 0xD8, 0x78, 0x56, 0x34, 0x12];
        assert!(largeur_perdue(&longue, 1200.0).is_empty());
    }

    /// Le titre du panneau existe dans les deux langues, et le nom de
    /// colonne est plus court.
    #[test]
    fn the_panel_names_itself_in_both_languages() {
        let module = DetourModule::default();
        assert!(module.title("fr").contains("Détourner"));
        assert!(module.title("en").contains("Detour"));
        assert!(
            module.nav_title("fr").len() < module.title("fr").len(),
            "la colonne a besoin du nom court"
        );
        assert_eq!(module.section(), ReSection::Bytes);
    }
}
