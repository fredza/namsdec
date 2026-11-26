//! La Scène : regarder la machine faire, en grand.
//!
//! **Ce qu'elle ajoute à ce qui existe déjà.** Le panneau de la machine
//! montre seize registres, huit indicateurs et douze cases de pile, en
//! chiffres de treize points : c'est ce qu'il faut pour travailler, et
//! c'est illisible à un mètre. La Scène prend le contre-pied — **trois
//! chiffres en très grand, et seulement ceux qui viennent de bouger** —
//! parce qu'elle répond à une autre question : non pas « où en est la
//! machine ? », mais « qu'est-ce que cette instruction vient de faire ?
//! »
//!
//! Elle sert donc deux usages que le panneau latéral sert mal : suivre
//! un pas quand on apprend, et montrer un programme à quelqu'un — en
//! classe, en vidéoprojection, par-dessus une épaule.
//!
//! **Pourquoi une fenêtre et non une vue.** C'est le choix de Fred, le
//! 9 novembre 2026 : l'Établi pour le mode simple, « et la Scène en
//! bouton ». Une fenêtre s'ouvre par-dessus ce qu'on faisait et se
//! referme sans rien déranger, elle existe AUX DEUX NIVEAUX, et elle ne
//! prend la place de rien. Une dixième vue dans la rangée aurait coûté
//! un onglet de plus à tout le monde, et elle n'aurait pas été
//! projetable : une vue partage l'écran, une fenêtre peut le prendre.
//!
//! Elle ne calcule rien qui lui soit propre : le ruban décode depuis
//! RIP comme le fait le bandeau de la prochaine instruction, les
//! valeurs viennent de `regs_before` et du processeur, la fiche vient
//! de `namsdec-doc`. **Ce qui change n'est pas ce qu'on montre, c'est
//! la taille à laquelle on le montre.**

use super::*;

/// Combien d'instructions le ruban montre après celle qui vient.
///
/// Quatre : de quoi voir où l'on va sans que la bande devienne un
/// second listing — il y en a déjà un, et il est plus complet.
const RUBAN_APRÈS: usize = 4;

/// Combien de registres passent en grand, au plus.
///
/// Trois. Une instruction x86 en touche rarement plus de deux, et le
/// troisième laisse la place à l'indicateur qui a basculé. Au-delà, les
/// chiffres rapetissent et la Scène ne sert plus à rien.
const CARTES: usize = 3;

/// La taille du chiffre d'une carte.
const CHIFFRE: f32 = 28.0;

impl NamsdecApp {
    /// La fenêtre « Regarder tourner ».
    pub(crate) fn scene_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_scene {
            return;
        }
        let s = strings(self.lang);
        let mut ouverte = true;
        let mut fermer = false;
        egui::Window::new(s.scene_title)
            // Le titre suit la langue, l'identifiant non : une fenêtre
            // qui change d'identité en changeant de langue perd sa
            // place et sa taille.
            .id(egui::Id::unique("scene_window"))
            .open(&mut ouverte)
            .resizable(true)
            // La hauteur n'est PAS imposée : la Scène montre trois
            // cartes et un ruban, et rien d'autre ne vient s'y ajouter
            // selon l'état. Une hauteur figée lui aurait laissé six
            // cents pixels de vide au milieu, ce que
            // `a_window_opens_at_the_size_of_what_it_shows` refuse — et
            // à juste titre : une fenêtre plus grande que ce qu'elle
            // montre fait croire qu'il manque quelque chose.
            .default_width(760.0)
            .min_width(460.0)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "scene"))
            // LA BARRE DE TITRE EST LA NÔTRE, comme pour les quatorze
            // autres fenêtres : celle d'egui peint son titre en
            // dix-huit points, une taille qui n'est dans le jeu de
            // l'application nulle part ailleurs.
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.scene_title) {
                    fermer = true;
                }
                self.scene_body_ui(ui);
            });
        self.show_scene = ouverte && !fermer;
    }

    /// Le corps de la Scène, à part de sa fenêtre pour que le banc
    /// puisse le peindre seul.
    fn scene_body_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        if self.emulator.is_none() {
            // Un panneau simplement vide se lit comme cassé : il dit ce
            // qui manque, ce qu'on y verra, et porte le bouton qui y
            // mène — la règle de tous les états vides de
            // l'application.
            if crate::empty_state(
                ui,
                theme,
                s.scene_empty,
                Some(s.scene_hint),
                Some(s.action_assemble),
            ) {
                self.dispatch(Action::Assemble);
            }
            // Les commandes restent, grisées : elles sont à la même
            // place avant et après l'assemblage, et chacune dit ce qui
            // lui manque. Une fenêtre qui change de forme entre ses
            // deux états se réapprend à chaque fois.
            ui.separator();
            self.scene_commandes_ui(ui);
            return;
        }
        self.scene_ruban_ui(ui);
        ui.add_space(10.0);
        self.scene_cartes_ui(ui);
        ui.add_space(10.0);
        self.scene_fiche_ui(ui);
        ui.add_space(8.0);
        ui.separator();
        self.scene_commandes_ui(ui);
    }

    /// Le ruban : ce qui vient d'être fait, ce qui vient, et la suite.
    ///
    /// Décodé en avant depuis RIP, et en arrière d'une seule
    /// instruction — celle du dernier pas, que l'application retient
    /// déjà. Remonter plus loin demanderait de décoder à reculons, ce
    /// qu'on ne sait pas faire sur du x86 sans deviner.
    fn scene_ruban_ui(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let passée = self
            .last_step
            .as_ref()
            .map(|pas| (pas.addr, pas.texte.clone()));
        let suite = self.scene_suite();
        egui::ScrollArea::horizontal()
            .id_salt("scene_ruban")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some((addr, texte)) = passée {
                        vignette(ui, theme, addr, &texte, Place::Passée);
                    }
                    for (i, (addr, texte)) in suite.iter().enumerate() {
                        let place = if i == 0 { Place::Ici } else { Place::ÀVenir };
                        vignette(ui, theme, *addr, texte, place);
                    }
                });
            });
    }

    /// Les instructions à partir de RIP, celle qui vient comprise.
    fn scene_suite(&self) -> Vec<(u64, String)> {
        let Some(emulator) = &self.emulator else {
            return Vec::new();
        };
        let cpu = emulator.cpu();
        if cpu.halted {
            return Vec::new();
        }
        let mut suite = Vec::new();
        let mut adresse = cpu.regs.rip;
        for _ in 0..=RUBAN_APRÈS {
            // Seize octets : la plus longue instruction x86 possible.
            // Moins que cela en fin de mémoire, et le décodage s'arrête
            // de lui-même.
            let Ok(fenêtre) = cpu.mem.peek_bytes(adresse, 16).or_else(|_| {
                cpu.mem
                    .peek_bytes(adresse, (cpu.mem.range().end - adresse) as usize)
            }) else {
                break;
            };
            let Some(instr) = namsdec_disasm::decode_one(fenêtre, adresse) else {
                break;
            };
            suite.push((adresse, namsdec_disasm::format_one(&instr)));
            adresse += instr.len() as u64;
        }
        suite
    }

    /// Les trois grands chiffres : ce qui a changé au dernier pas.
    ///
    /// Quand rien n'a changé — avant le premier pas, ou après une
    /// instruction qui ne touche aucun registre —, on montre les trois
    /// qu'on regarde toujours plutôt que trois cartes vides : un écran
    /// vide se lit comme un écran cassé.
    fn scene_cartes_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let Some(emulator) = &self.emulator else {
            return;
        };
        let maintenant = emulator.cpu().regs.clone();
        let bougés: Vec<(&'static str, u64, Option<u64>)> = match &self.regs_before {
            Some(avant) => REGISTERS
                .iter()
                .filter(|(_, reg)| avant.get64(*reg) != maintenant.get64(*reg))
                .map(|(nom, reg)| (*nom, maintenant.get64(*reg), Some(avant.get64(*reg))))
                .take(CARTES)
                .collect(),
            None => Vec::new(),
        };
        let cartes = if bougés.is_empty() {
            [Reg64::Rax, Reg64::Rcx, Reg64::Rsp]
                .iter()
                .map(|reg| {
                    let nom = REGISTERS
                        .iter()
                        .find(|(_, r)| r == reg)
                        .map(|(nom, _)| *nom)
                        .unwrap_or("?");
                    (nom, maintenant.get64(*reg), None)
                })
                .collect()
        } else {
            bougés
        };

        ui.label(
            egui::RichText::new(if cartes.iter().any(|(_, _, avant)| avant.is_some()) {
                s.scene_changed
            } else {
                s.scene_nothing_changed
            })
            .color(theme.dim)
            .size(12.0),
        );
        ui.add_space(4.0);
        // Des colonnes de largeur égale : trois chiffres qui ne
        // commencent pas à la même hauteur se comparent mal, et c'est
        // exactement ce qu'on vient faire ici.
        ui.columns(cartes.len().max(1), |colonnes| {
            for (i, (nom, valeur, avant)) in cartes.iter().enumerate() {
                let ui = &mut colonnes[i];
                carte(ui, theme, nom, *valeur, *avant);
            }
        });
    }

    /// Ce que l'instruction qui vient veut dire, en toutes lettres.
    fn scene_fiche_ui(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let Some((_, texte)) = self.scene_suite().into_iter().next() else {
            return;
        };
        let mnémonique = texte
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_lowercase();
        let Some(fiche) = namsdec_doc::lookup(&mnémonique) else {
            return;
        };
        egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .corner_radius(egui::CornerRadius::from(6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&mnémonique)
                            .monospace()
                            .strong()
                            .color(theme.mnemonic)
                            .size(14.0),
                    );
                    ui.label(
                        egui::RichText::new(fiche.title.get(self.lang.code()))
                            .color(theme.text)
                            .size(13.0),
                    );
                });
            });
    }

    /// Les commandes, en bas : un geste en grand, les autres à côté.
    fn scene_commandes_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let peut = self.can_execute();
        let peut_revenir = self.emulator.as_ref().is_some_and(|e| e.can_step_back());
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            // Le pas d'abord, et plus gros que le reste : c'est le seul
            // geste qu'on refait cent fois dans cette fenêtre.
            ui.add_enabled_ui(peut, |ui| {
                let nom = action_label_pour(s, Action::StepInto, self.niveau());
                if icon_button(ui, icon_step_into, nom)
                    .on_disabled_hover_text(s.why_program_finished)
                    .clicked()
                {
                    self.dispatch(Action::StepInto);
                }
            });
            ui.add_enabled_ui(peut_revenir, |ui| {
                let nom = action_label_pour(s, Action::StepBack, self.niveau());
                if icon_button(ui, icon_step_back, nom)
                    .on_disabled_hover_text(s.why_nothing_to_undo)
                    .clicked()
                {
                    self.dispatch(Action::StepBack);
                }
            });
            ui.add_enabled_ui(peut, |ui| {
                if icon_button(ui, icon_run, s.action_run)
                    .on_disabled_hover_text(s.why_program_finished)
                    .clicked()
                {
                    self.dispatch(Action::Run);
                }
            });
            ui.add_enabled_ui(self.emulator.is_some(), |ui| {
                if icon_button(ui, icon_reset, s.action_reset)
                    .on_disabled_hover_text(s.why_no_program)
                    .clicked()
                {
                    self.dispatch(Action::Reset);
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(emulator) = self.emulator.as_ref() {
                    let rip = emulator.cpu().regs.rip;
                    ui.label(
                        egui::RichText::new(format!("RIP {rip:#x}"))
                            .monospace()
                            .color(theme.dim)
                            .size(12.0),
                    );
                }
            });
        });
    }
}

/// Où se trouve une vignette du ruban par rapport à l'exécution.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    /// Déjà exécutée — celle du dernier pas.
    Passée,
    /// Celle qui va s'exécuter.
    Ici,
    /// Celles d'après.
    ÀVenir,
}

/// Une vignette du ruban : une adresse, une instruction, et son rang.
fn vignette(ui: &mut egui::Ui, theme: &'static Theme, addr: u64, texte: &str, place: Place) {
    let (fond, encre, bordure) = match place {
        Place::Ici => (theme.selection, theme.text, theme.accent),
        Place::Passée => (theme.surface, theme.dim, theme.dim.gamma_multiply(0.45)),
        Place::ÀVenir => (theme.panel, theme.dim, theme.dim.gamma_multiply(0.45)),
    };
    egui::Frame::new()
        .fill(fond)
        .stroke(egui::Stroke::new(1.0, bordure))
        .corner_radius(egui::CornerRadius::from(6))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(format!("{addr:#x}"))
                        .monospace()
                        .color(theme.dim)
                        .size(9.0),
                );
                ui.label(
                    egui::RichText::new(texte)
                        .monospace()
                        .color(encre)
                        .size(if place == Place::Ici { 14.0 } else { 12.0 }),
                );
            });
        });
}

/// Une carte : un registre, sa valeur en grand, et d'où elle vient.
fn carte(ui: &mut egui::Ui, theme: &'static Theme, nom: &str, valeur: u64, avant: Option<u64>) {
    let bouge = avant.is_some();
    egui::Frame::new()
        .fill(if bouge { theme.surface } else { theme.panel })
        .stroke(egui::Stroke::new(
            1.0,
            if bouge {
                theme.changed
            } else {
                theme.dim.gamma_multiply(0.45)
            },
        ))
        .corner_radius(egui::CornerRadius::from(8))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(nom)
                        .monospace()
                        .strong()
                        .color(theme.register)
                        .size(12.0),
                );
                // Le décimal d'abord quand il tient, l'hexadécimal
                // dessous : on apprend en décimal, on débogue en
                // hexadécimal, et les deux doivent être là.
                let grand = if valeur <= 9_999_999 {
                    format!("{valeur}")
                } else {
                    format!("{valeur:#x}")
                };
                ui.label(
                    egui::RichText::new(grand)
                        .monospace()
                        .color(if bouge { theme.changed } else { theme.text })
                        .size(CHIFFRE),
                );
                match avant {
                    // « -> » et non « → » : la flèche d'Unicode manque
                    // de la police par défaut d'egui et se peint en
                    // carré vide. C'est aussi ce qu'écrit le bloc
                    // « Dernier pas », donc les deux se lisent pareil.
                    Some(avant) => ui.label(
                        egui::RichText::new(format!("{avant:#x} -> {valeur:#x}"))
                            .monospace()
                            .color(theme.dim)
                            .size(12.0),
                    ),
                    None => ui.label(
                        egui::RichText::new(format!("{valeur:#x}"))
                            .monospace()
                            .color(theme.dim)
                            .size(12.0),
                    ),
                };
            });
        });
}
