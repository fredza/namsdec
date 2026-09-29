//! La barre d'état : une ligne, tout en bas, qui dit où l'on en est.
//!
//! Elle porte trois choses et rien d'autre : ce que le programme vient
//! de faire, le mode de l'éditeur, et la légende des couleurs quand la
//! carte de chaleur est allumée. Une ligne qui déborde ou qui change de
//! hauteur se remarque à chaque frappe ; c'est pourquoi elle est
//! mesurée de près par le banc.

use super::*;

impl NamsdecApp {
    pub(crate) fn statusbar_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let lang = self.lang;
        let theme = self.theme;

        // Toutes les touches déjà tapées, pas seulement la première : sur
        // `Espace s i`, l'indicateur doit montrer où l'on en est dans la
        // séquence, sinon il ne dit plus rien après la deuxième frappe.
        let pending = self.dispatcher.pending();
        if !pending.is_empty() {
            let keys = pending
                .iter()
                .map(|k| k.name())
                .collect::<Vec<_>>()
                .join(" ");
            let text = if lang == Lang::Fr {
                format!("{keys} — touche suivante attendue…")
            } else {
                format!("{keys} — waiting for next key…")
            };
            ui.colored_label(theme.accent, text);
        }

        // Ce qui tourne à côté, tout à gauche : c'est la première chose
        // qu'on veut voir quand on se demande si l'outil est parti.
        if let Some(job) = &self.job {
            ui.horizontal(|ui| {
                // Une installation compte ses étapes : c'est une
                // progression vraie, là où le temps ne dirait rien.
                let étapes = self.install_progress();
                let barre = match étapes
                    .map(|(f, t)| f as f32 / t.max(1) as f32)
                    .or_else(|| job.fraction())
                {
                    // Une progression vraie : le temps écoulé sur le
                    // temps accordé. C'est ce qui décide de l'abandon,
                    // donc c'est ce qui intéresse.
                    Some(part) => egui::ProgressBar::new(part).desired_width(150.0).text(
                        egui::RichText::new(format!("{} — {}", job.label, job.timing())).small(),
                    ),
                    // Rien ne borne : une barre qui avance toute seule
                    // dit « ça travaille » sans prétendre savoir combien
                    // il reste.
                    None => egui::ProgressBar::new(0.0)
                        .animate(true)
                        .desired_width(150.0)
                        .text(
                            egui::RichText::new(format!("{} — {}", job.label, job.timing()))
                                .small(),
                        ),
                };
                ui.add(barre);
                ui.separator();
            });
        }

        ui.horizontal(|ui| {
            // Compartimenté plutôt qu'un seul bloc de texte : pastille de
            // mode, séparateur, message de statut, séparateur, statut nasm
            // (version raccourcie — "NASM 3.02", pas la ligne complète de
            // `nasm -v` avec sa date de compilation, hors de propos ici).
            mode_pill_ui(ui, theme, self.editor_mode);
            ui.separator();
            // Quel fichier on édite, et s'il reste quelque chose à
            // enregistrer. Sans cela, rien à l'écran ne distingue un
            // tampon sauvegardé d'un tampon qui ne l'est pas — et on ne
            // s'en aperçoit qu'en fermant.
            let file = self
                .current_file
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| s.untitled_buffer.to_owned());
            let modified = self.source != self.saved_source;
            ui.colored_label(
                if modified { theme.changed } else { theme.dim },
                if modified {
                    format!("{file} •")
                } else {
                    file
                },
            )
            .on_hover_text(if modified {
                s.buffer_modified
            } else {
                s.buffer_saved
            });
            ui.separator();

            // La correction proposée pour la ligne du curseur, et la
            // touche qui la pose. À gauche du message d'état : elle
            // parle de ce qu'on est en train d'écrire, non de ce que
            // namsdec vient de faire, et c'est l'ordre dans lequel on
            // lit la barre.
            if let Some(correction) = self.current_fix() {
                ui.scope(|ui| {
                    // Bornée et tronquée, comme le message : une phrase
                    // longue ne doit pas pousser les compartiments de
                    // droite hors de la fenêtre.
                    ui.set_max_width(420.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    ui.colored_label(
                        theme.changed,
                        s.fix_hint.replacen("{}", &correction.titre, 1),
                    )
                    // La ligne telle qu'elle sera écrite : on confirme
                    // ce qu'on a vu, non ce qu'on a deviné.
                    .on_hover_text(&correction.ligne);
                });
                ui.separator();
            }
            // Le message est borné, et tronqué s'il déborde : les
            // compartiments de droite sont posés dans la place qui reste,
            // et un message long — le nom d'un fichier chargé, une erreur
            // de l'éditeur de liens — leur passait dessus, texte sur
            // texte.
            ui.scope(|ui| {
                ui.set_max_width((ui.available_width() - STATUS_RIGHT_ROOM).max(80.0));
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                self.status_message_ui(ui);
            });

            // Compartiments de droite, remplis de droite à gauche :
            // nasm, puis la position du curseur, puis — seulement quand un
            // programme est chargé — la ligne où l'exécution est arrêtée.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (dot_color, text) = match &self.nasm_info {
                    Some(info) => (theme.good, short_nasm_version(&info.version)),
                    None => (theme.bad, s.nasm_not_found.to_string()),
                };
                ui.label(text);
                status_dot(ui, dot_color);
                ui.separator();

                let (line, col) = self.cursor_pos;
                ui.label(if lang == Lang::Fr {
                    format!("Lig {line}, Col {col}")
                } else {
                    format!("Ln {line}, Col {col}")
                });

                if let Some(exec_line) = self.current_line() {
                    ui.separator();
                    ui.colored_label(
                        theme.accent,
                        if lang == Lang::Fr {
                            format!("ligne {exec_line}")
                        } else {
                            format!("line {exec_line}")
                        },
                    );
                    // La même flèche que la gouttière marque dans la marge,
                    // pour que les deux se lisent comme la même chose.
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    icon_run(ui.painter(), rect, theme.accent);
                }
            });
        });
    }

    /// Le message d'état : sa phrase, sa couleur, et la pastille qui la
    /// précède quand il y en a une.
    ///
    /// **Un seul endroit décide de la phrase.** La barre la peint, et
    /// l'infobulle la répète en entier quand la place a manqué. Deux
    /// fonctions qui la composeraient chacune finiraient par diverger,
    /// et ce serait l'infobulle qui aurait tort — celle qu'on ne
    /// consulte justement que pour vérifier.
    pub(crate) fn status_line(&self) -> (String, egui::Color32, Option<egui::Color32>) {
        let s = strings(self.lang);
        let fr = self.lang == Lang::Fr;
        let theme = self.theme;
        match &self.status {
            Status::Idle => (s.status_ready.to_owned(), theme.text, None),
            Status::Running => (s.status_running.to_owned(), theme.accent, None),
            // Une pastille peinte, pas un glyphe : « ✓ » et « ✗ » ne
            // sont pas dans la police par défaut d'egui et retombaient
            // sur le carré de remplacement.
            Status::AssembleOk => (s.status_assembled.to_owned(), theme.good, Some(theme.good)),
            Status::AssembleFailed => (
                s.status_assemble_failed.to_owned(),
                theme.bad,
                Some(theme.bad),
            ),
            // La valeur d'avant et celle d'après : c'est toute
            // l'information qu'on attendait en posant le point.
            Status::Watchpoint { addr, old, new } => (
                if fr {
                    format!("Surveillance : {addr:#x} est passé de {old:#x} à {new:#x}")
                } else {
                    format!("Watchpoint: {addr:#x} went from {old:#x} to {new:#x}")
                },
                theme.changed,
                Some(theme.changed),
            ),
            Status::RegisterChanged { register, old, new } => (
                if fr {
                    format!("Surveillance : {register} est passé de {old:#x} à {new:#x}")
                } else {
                    format!("Watch: {register} went from {old:#x} to {new:#x}")
                },
                theme.changed,
                Some(theme.changed),
            ),
            Status::SteppedBack => (s.status_stepped_back.to_owned(), theme.accent, None),
            Status::NothingToUndo => (s.status_nothing_to_undo.to_owned(), theme.dim, None),
            Status::Halted => (s.status_halted.to_owned(), theme.good, None),
            Status::WaitingForInput => (s.status_waiting_for_input.to_owned(), theme.accent, None),
            Status::Exited { code } => (
                if fr {
                    format!("Programme terminé (code {code}).")
                } else {
                    format!("Program exited (code {code}).")
                },
                if *code == 0 { theme.good } else { theme.bad },
                None,
            ),
            Status::BinaryLoaded { bytes, truncated } => {
                let taille = match (*truncated, fr) {
                    (true, true) => format!("{bytes} premiers octets"),
                    (true, false) => format!("first {bytes} bytes"),
                    (false, true) => format!("{bytes} octets"),
                    (false, false) => format!("{bytes} bytes"),
                };
                (
                    format!("{} — {taille}", s.binary_not_text),
                    theme.accent,
                    None,
                )
            }
            Status::Breakpoint(addr) => (
                if fr {
                    format!("Arrêté sur point d'arrêt — {addr:#x}")
                } else {
                    format!("Stopped at breakpoint — {addr:#x}")
                },
                theme.accent,
                None,
            ),
            Status::StepLimit => (s.status_step_limit.to_owned(), theme.bad, None),
            Status::NasmMissing => (s.status_nasm_missing.to_owned(), theme.bad, None),
            Status::Note(phrase) => (phrase(s).to_owned(), theme.good, None),
            // Le nombre devant la phrase : « 7 occurrences surlignées ».
            // Le compte ne se traduit pas, la phrase si — d'où les deux
            // morceaux séparés jusqu'ici.
            Status::Counted(combien, phrase) => {
                (format!("{combien} {}", phrase(s)), theme.good, None)
            }
            Status::Error(msg) => (
                if fr {
                    format!("Erreur : {msg}")
                } else {
                    format!("Error: {msg}")
                },
                theme.bad,
                None,
            ),
        }
    }

    /// Le message d'état, peint — et lisible en entier au survol quand la
    /// place a manqué.
    ///
    /// La troncature est **mesurée** et non devinée : la règle d'avant
    /// (« plus de quatre-vingt-dix caractères ») était une estimation, et
    /// la place dépend de la largeur de la fenêtre, de la police et du
    /// réglage de zoom. On compare ce que la phrase demande à ce qu'on
    /// lui a laissé, et l'infobulle n'apparaît que si le second est plus
    /// petit — une infobulle qui répète un texte entièrement visible
    /// apprend à ne plus les lire.
    pub(crate) fn status_message_ui(&self, ui: &mut egui::Ui) {
        let (texte, couleur, pastille) = self.status_line();
        if texte.is_empty() {
            return;
        }
        if let Some(couleur_pastille) = pastille {
            status_dot(ui, couleur_pastille);
        }
        let demandé = ui
            .painter()
            .layout_no_wrap(
                texte.clone(),
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::PLACEHOLDER,
            )
            .rect
            .width();
        let réponse = ui.colored_label(couleur, &texte);
        if demandé > réponse.rect.width() + 0.5 {
            réponse.on_hover_text(&texte);
        }
    }

    /// La légende de la barre de chaleur : ce que chaque teinte vaut.
    ///
    /// Une bande de couleur sans échelle ne dit rien — « plus rouge »
    /// ne devient « mille passages » que si on l'écrit. Les graduations
    /// suivent l'échelle réelle, celle de la ligne la plus chaude du
    /// programme, et sont logarithmiques comme elle.
    pub(crate) fn heat_legend_ui(&mut self, ui: &mut egui::Ui, max: u64) {
        let s = strings(self.lang);
        let theme = self.theme;
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.heat_legend);
            // Cinq repères entre une fois et le maximum : assez pour
            // situer une teinte, assez peu pour tenir sur une ligne.
            let repères = heat_legend_steps(max);
            for valeur in repères {
                let (rect, réponse) =
                    ui.allocate_exact_size(egui::vec2(14.0, 12.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 2.0, heat_colour(theme, valeur, max));
                réponse.on_hover_text(if self.lang == Lang::Fr {
                    format!("{valeur} passage(s)")
                } else {
                    format!("{valeur} pass(es)")
                });
                ui.colored_label(
                    theme.dim,
                    egui::RichText::new(sidebar::group_digits(valeur)).monospace(),
                );
                ui.add_space(4.0);
            }
            ui.colored_label(theme.dim, s.heat_legend_hint);
        });
    }
}

/// Pastille de mode façon vim-airline : fond plein de la couleur du
/// mode, et par-dessus l'encre qui s'y lit le mieux.
///
/// Un noir fixe était écrit ici, avec pour raison « le contraste ». Il
/// l'avait sur le bleu vif d'un thème sombre, et pas du tout sur le
/// bleu profond d'un thème clair — 2,9, où le blanc en donne 6,3. Le
/// fond change avec le thème et avec le mode ; l'encre doit changer
/// avec lui.
fn mode_pill_ui(ui: &mut egui::Ui, theme: &Theme, mode: EditorMode) {
    let fond = mode.color(theme);
    egui::Frame::new()
        .fill(fond)
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.colored_label(encre_sur(fond, theme), mode.label());
        });
}
