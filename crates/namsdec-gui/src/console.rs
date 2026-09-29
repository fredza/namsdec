//! La console : le panneau du bas et ses cinq onglets.
//!
//! C'est là que le programme répond — ce qu'il écrit, ce que
//! l'assembleur reproche, où le temps est passé, quels modules sont
//! chargés — et là qu'on lui parle quand il attend une entrée.
//!
//! Un seul panneau, cinq onglets, parce qu'un débutant qui cherche « où
//! s'affiche mon texte » ne doit pas avoir à deviner lequel des cinq
//! coins de l'écran le porte.

use super::*;

impl NamsdecApp {
    /// Le panneau bas : la console du programme et les diagnostics de
    /// nasm, sur toute la largeur de la fenêtre.
    ///
    /// La console y a la largeur d'un terminal — un programme écrit en 80
    /// colonnes, le panneau latéral n'en offrait que trente-quatre — et
    /// les diagnostics y trouvent enfin de quoi être lus : ils étaient
    /// jusqu'ici entassés sous la barre de statut, sur deux lignes
    /// hautes de rien.
    pub(crate) fn console_panel_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal_wrapped(|ui| {
            for tab in ConsoleTab::ALL {
                // Le compte des diagnostics se lit sur l'onglet : c'est
                // ce qui fait remarquer un avertissement qu'on n'était
                // pas allé chercher.
                let label = if tab == ConsoleTab::Diagnostics && !self.diagnostics.is_empty() {
                    format!("{} ({})", tab.label(s), self.diagnostics.len())
                } else {
                    tab.label(s).to_owned()
                };
                if ui
                    .selectable_label(self.console_tab == tab, label)
                    .clicked()
                {
                    self.console_tab = tab;
                }
            }
        });
        ui.separator();
        match self.console_tab {
            ConsoleTab::Output => self.output_ui(ui),
            ConsoleTab::Diagnostics => self.diagnostics_ui(ui),
            ConsoleTab::Profile => self.profile_ui(ui),
        }
    }

    /// Le profil : où le programme a passé son temps.
    ///
    /// La bande de chaleur dans la gouttière répond à « par où ça
    /// chauffe » d'un coup d'œil, mais elle ne classe pas : deux rouges
    /// voisins ne disent pas lequel coûte le double de l'autre. Ce
    /// tableau le dit — le nombre de passages, la part du total, et la
    /// ligne elle-même — et y emmène d'un clic. C'est ce qu'un
    /// profileur montre d'un programme, à la seule échelle qui vaille
    /// ici : la ligne de source.
    pub(crate) fn profile_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let passages = self.line_visits();
        if passages.is_empty() {
            ui.colored_label(theme.dim, s.profile_empty);
            return;
        }
        // Le plus chaud d'abord ; à égalité, la ligne la plus haute,
        // pour que deux exécutions de suite donnent le même ordre.
        let mut classement: Vec<(u32, u64)> = passages.iter().map(|(l, c)| (*l, *c)).collect();
        classement.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let total: u64 = classement.iter().map(|(_, c)| *c).sum();
        let plus_chaude = classement.first().map_or(1, |(_, c)| *c).max(1);

        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                theme.dim,
                format!("{} {}", sidebar::group_digits(total), s.profile_total),
            );
            if ui.small_button(s.profile_copy).clicked() {
                ui.ctx().copy_text(self.profile_text(&classement, total));
                self.status = Status::Note(|s| s.profile_copied);
            }
        });
        // Le budget de l'exercice, quand il y en a un : c'est ici qu'on
        // vient quand on le dépasse, et la réponse — quelle ligne
        // reprendre — est juste en dessous.
        if let Some(budget) = self
            .current_exercise
            .as_deref()
            .and_then(|id| namsdec_exercises::find(&self.exercise_packs, id))
            .and_then(|exercice| exercice.par)
        {
            let exécutées = self.emulator.as_ref().map_or(0, |e| e.executed());
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, s.par_label);
                ui.colored_label(
                    if exécutées > budget {
                        theme.warn
                    } else {
                        theme.good
                    },
                    egui::RichText::new(format!("{exécutées} / {budget}")).monospace(),
                );
                if exécutées > budget {
                    ui.colored_label(theme.dim, s.profile_over_budget);
                }
            });
        }
        ui.colored_label(theme.dim, s.profile_hint);
        ui.separator();

        let mut goto = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Une grille, et non des rangées posées à la main : les
                // colonnes d'une rangée tombaient sous les mots d'une
                // autre, et l'en-tête — un seul texte dont les espaces ne
                // mesuraient rien — sous aucune.
                egui::Grid::new("profile_rows")
                    .num_columns(6)
                    .min_col_width(0.0)
                    .striped(true)
                    .show(ui, |ui| {
                        // L'en-tête : sans lui, « 12 » et « 4,2 % » se
                        // lisent, mais « 1 204 » se prend pour une
                        // adresse.
                        ui.label("");
                        // Les titres au même gabarit que les nombres
                        // qu'ils surmontent : dans une chasse fixe, c'est
                        // le remplissage qui aligne, et la grille fait le
                        // reste. Une cellule de grille ne prend que la
                        // largeur de son contenu — un calage à droite n'y
                        // change donc rien.
                        for (titre, largeur) in [
                            (s.line_word, COL_LIGNE),
                            (s.profile_count, COL_PASSAGES),
                            (s.profile_share, COL_PART),
                        ] {
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(caler_à_droite(titre, largeur)).monospace(),
                            );
                        }
                        ui.label("");
                        ui.label("");
                        ui.end_row();

                        for (ligne, compte) in &classement {
                            let part = *compte as f32 / total.max(1) as f32;
                            let teinte = heat_colour(theme, *compte, plus_chaude);
                            // La pastille reprend exactement la teinte de
                            // la gouttière : la même ligne a la même
                            // couleur aux deux endroits, sinon le lien se
                            // perd.
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(8.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 1.0, teinte);

                            // Les trois nombres, calés à droite de leur
                            // colonne : c'est ainsi que des chiffres se
                            // comparent d'une ligne à l'autre.
                            for (texte, largeur, couleur) in [
                                (format!("{ligne}"), COL_LIGNE, theme.dim),
                                (sidebar::group_digits(*compte), COL_PASSAGES, theme.text),
                                (pourcentage(part, self.lang), COL_PART, theme.dim),
                            ] {
                                ui.colored_label(
                                    couleur,
                                    egui::RichText::new(caler_à_droite(&texte, largeur))
                                        .monospace(),
                                );
                            }

                            // Une barre proportionnelle : les pourcentages
                            // se comparent bien plus vite en longueur
                            // qu'en chiffres.
                            let (barre, _) = ui
                                .allocate_exact_size(egui::vec2(60.0, 10.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    barre.min,
                                    egui::vec2(barre.width() * part, barre.height()),
                                ),
                                1.0,
                                teinte,
                            );

                            // Le bloc et la ligne elle-même : sans eux, le
                            // classement renvoie à un numéro qu'il faut
                            // aller chercher.
                            ui.horizontal(|ui| {
                                if let Some(étiquette) = enclosing_label(&self.source, *ligne) {
                                    ui.colored_label(
                                        theme.label,
                                        egui::RichText::new(format!("{étiquette}:")).monospace(),
                                    );
                                }
                                let réponse = cliquable(
                                    ui,
                                    egui::Label::new(
                                        egui::RichText::new(source_line(&self.source, *ligne))
                                            .monospace()
                                            .color(theme.text),
                                    )
                                    .truncate(),
                                );
                                if réponse.on_hover_text(s.profile_goto).clicked() {
                                    goto = Some(*ligne);
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
        if let Some(ligne) = goto {
            self.goto_line = Some(ligne);
        }
    }

    /// Ce que nasm a dit de la source, une ligne par diagnostic.
    pub(crate) fn diagnostics_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (lang, theme) = (self.lang, self.theme);
        let mut goto = None;
        // Les deux commandes réellement lancées. « Et si je veux le
        // faire sans namsdec ? » est la bonne question, et un atelier
        // d'apprentissage y répond au lieu de garder son tour de main
        // pour lui.
        let fichier = self
            .current_file
            .as_ref()
            .and_then(|path| path.file_stem())
            .map(|nom| nom.to_string_lossy().into_owned())
            .unwrap_or_else(|| "programme".to_owned());
        let commandes = namsdec_asm::build_commands(&format!("{fichier}.asm"), &fichier, BASE_ADDR);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.assemble_commands);
            if ui
                .small_button(s.assemble_commands_copy)
                .on_hover_text(s.assemble_commands_hint)
                .clicked()
            {
                ui.ctx().copy_text(commandes.join("\n"));
                self.status = Status::Note(|s| s.assemble_commands_copied);
            }
        });
        for commande in &commandes {
            ui.colored_label(theme.dim, egui::RichText::new(commande).monospace());
        }
        ui.separator();
        if self.diagnostics.is_empty() {
            ui.colored_label(theme.dim, s.no_diagnostics);
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for d in &self.diagnostics {
                    let color = if d.severity == Severity::Error {
                        theme.bad
                    } else {
                        theme.warn
                    };
                    // La phrase est reformée ici quand le diagnostic
                    // porte son motif : c'est ce qui la fait suivre la
                    // langue du moment. Sans cela, ceux qui étaient déjà
                    // dans la liste gardaient celle où ils étaient nés,
                    // et basculer de langue laissait du français dans une
                    // interface anglaise — les retraduire aurait demandé
                    // de réassembler.
                    let phrase = match &d.kind {
                        Some(kind) => kind.message(lang == Lang::Fr),
                        None => d.message.clone(),
                    };
                    // Un diagnostic sans ligne — ceux de l'éditeur de
                    // liens, qui parlent du programme entier — ne
                    // s'annonce pas par « ligne 0 ».
                    let text = match (d.line, lang) {
                        (0, _) => phrase,
                        (line, Lang::Fr) => format!("ligne {line} — {phrase}"),
                        (line, _) => format!("line {line} — {phrase}"),
                    };
                    // Cliquable quand nasm a nommé une ligne : lire le
                    // message et retrouver l'endroit sont deux gestes, et
                    // le second n'a aucune raison d'être manuel.
                    let response = cliquable(
                        ui,
                        egui::Label::new(egui::RichText::new(text).monospace().color(color)),
                    );
                    if d.line > 0 && response.on_hover_text(s.diagnostic_goto).clicked() {
                        goto = Some(d.line);
                    }
                    // Le message brut reste — c'est lui qu'on colle dans
                    // une recherche —, et l'explication vient dessous.
                    if let Some(explication) = nasm_errors::explain(lang, &d.message) {
                        ui.label(prose(ui, theme, explication));
                        ui.add_space(6.0);
                    }
                }
            });
        if let Some(line) = goto {
            self.goto_line = Some(line);
        }
    }

    /// La console du programme : ce qu'il a écrit, et de quoi lui donner
    /// ce qu'il veut lire.
    ///
    /// Les octets de sortie sont rendus en UTF-8 « au mieux » : un
    /// programme en assembleur écrit ce qu'il veut, y compris des octets
    /// qui ne forment aucun texte, et ce n'est pas une raison pour ne rien
    /// afficher.
    ///
    /// La sortie a sa fenêtre à elle — encadrée, qui garde sa place même
    /// vide et défile pour son compte — plutôt que d'être une suite de
    /// lignes posées au-dessus du champ de saisie : un programme bavard
    /// repoussait ce champ hors de vue, et il fallait faire défiler pour
    /// lui répondre.
    pub(crate) fn output_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // La fenêtre de sortie est là dans tous les cas : sans programme
        // chargé elle le dit, plutôt que de laisser l'onglet vide.
        let (output, exit_code, pending) = match &self.emulator {
            Some(emulator) => (
                String::from_utf8_lossy(emulator.output()).into_owned(),
                emulator.exit_code(),
                emulator.pending_input(),
            ),
            None => (String::new(), None, 0),
        };

        // Posée en bas du panneau, la saisie ne bouge plus quoi qu'écrive
        // le programme.
        if self.console_shows_input() {
            egui::Panel::bottom("console_input")
                .resizable(false)
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    if let Some(code) = exit_code {
                        ui.separator();
                        let color = if code == 0 {
                            self.theme.good
                        } else {
                            self.theme.bad
                        };
                        ui.colored_label(
                            color,
                            if self.lang == Lang::Fr {
                                format!("Le programme s'est terminé avec le code {code}.")
                            } else {
                                format!("The program exited with code {code}.")
                            },
                        );
                    }
                    self.console_input_ui(ui, pending);
                });
            // Ce que la zone a vraiment pris : c'est cela qu'il faudra
            // réserver à l'image suivante.
            self.console_input_height =
                egui::containers::panel::PanelState::load(ui.ctx(), egui::Id::new("console_input"))
                    .map(|état| état.outer_rect.height())
                    .filter(|hauteur| *hauteur > 1.0);
        } else {
            self.console_input_height = None;
        }

        // Le reste de la hauteur revient à la sortie, sur le fond sombre
        // d'un terminal.
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    // Ce que le programme vient d'écrire est ce qu'on veut
                    // lire : la vue suit le bas, comme un terminal.
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.emulator.is_none() {
                            // Sa phrase à elle, et non le constat qui
                            // est déjà écrit deux fois plus haut. Au
                            // démarrage, « Aucun binaire assemblé. »
                            // s'affichait trois fois sur le même écran
                            // — le bandeau, l'onglet, et ici — sans que
                            // la troisième apprenne rien de plus. Une
                            // console dit ce qu'elle montrera.
                            ui.colored_label(self.theme.dim, s.console_before_run);
                        } else if output.is_empty() {
                            // Dire *pourquoi* c'est vide. « Le programme
                            // n'a rien écrit » devant un programme qui
                            // n'écrit jamais rien laisse chercher une
                            // panne qui n'existe pas ; devant un
                            // programme qu'on n'a pas lancé, c'est
                            // simplement faux.
                            let exécuté = self.emulator.as_ref().is_some_and(|e| e.executed() > 0);
                            ui.colored_label(
                                self.theme.dim,
                                match (self.program_writes, exécuté) {
                                    (false, _) => s.output_never,
                                    (true, false) => s.output_not_yet,
                                    (true, true) => s.output_empty,
                                },
                            );
                        } else {
                            // Sélectionnable : la sortie d'un programme se
                            // recopie ailleurs (un rapport, un test).
                            ui.add(
                                egui::Label::new(egui::RichText::new(output).monospace())
                                    .selectable(true),
                            );
                        }
                    });
            });
    }

    /// La boîte de saisie : là où l'on répond au programme.
    ///
    /// La ligne de la console suffit quand on la voit. Elle ne suffit pas
    /// quand une fenêtre d'analyse couvre l'écran, quand la console est
    /// repliée, ou simplement quand on ne sait pas encore qu'elle existe
    /// — et l'on reste alors devant un programme qui ne répond plus, sans
    /// savoir qu'il attend *nous*.
    ///
    /// La boîte s'ouvre donc d'elle-même au moment où le programme se met
    /// à attendre, au premier plan, le curseur déjà dans le champ. Elle
    /// montre **ce que le programme vient d'écrire** : neuf fois sur dix
    /// c'est l'invite — « Enter the password: » —, et c'est la seule
    /// chose qui dise ce qu'on attend de nous.
    pub(crate) fn input_dialog_ui(&mut self, ctx: &egui::Context) {
        // Le passage à l'attente ouvre la boîte ; l'attente seule ne la
        // rouvre pas, sans quoi la refermer serait impossible.
        let attend = matches!(self.status, Status::WaitingForInput);
        if attend && !self.was_waiting_for_input {
            self.show_input_dialog = true;
            self.input_dialog_focus = true;
        }
        self.was_waiting_for_input = attend;
        if !self.show_input_dialog {
            return;
        }

        let s = strings(self.lang);
        let theme = self.theme;
        // Les dernières lignes écrites : l'invite, et ce qui la précède.
        let sortie = self
            .emulator
            .as_ref()
            .map(|e| String::from_utf8_lossy(e.output()).into_owned())
            .unwrap_or_default();
        let invite = {
            let mut lignes: Vec<&str> = sortie.lines().rev().take(4).collect();
            lignes.reverse();
            lignes.join("\n")
        };

        let mut envoyer = false;
        let mut fin = false;
        let mut fermer = false;
        egui::Window::new(s.input_dialog_title)
            .id(egui::Id::new(INPUT_DIALOG))
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .current_pos(ctx.viewport_rect().center())
            .default_width(480.0)
            .min_width(480.0)
            .show(ctx, |ui| {
                if chrome::window_head(ui, theme, s, s.input_dialog_title) {
                    fermer = true;
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(10.0, ui.text_style_height(&egui::TextStyle::Body)),
                        egui::Sense::hover(),
                    );
                    let (couleur, phrase) = if attend {
                        (theme.accent, s.console_waiting.to_owned())
                    } else {
                        // Le programme a repris et n'attend plus : la
                        // boîte reste pour montrer ce qu'il a répondu,
                        // et dit où il en est.
                        let (texte, couleur, _) = self.status_line();
                        (couleur, texte)
                    };
                    ui.painter().circle_filled(rect.center(), 4.0, couleur);
                    ui.colored_label(couleur, egui::RichText::new(phrase).strong());
                });

                ui.add_space(8.0);
                ui.colored_label(
                    theme.dim,
                    if attend {
                        s.input_dialog_asked
                    } else {
                        s.input_dialog_answered
                    },
                );
                ui.add_space(2.0);
                // Ce que le programme vient d'écrire, sur le fond d'un
                // terminal : c'est sa voix, elle ne doit pas se confondre
                // avec celle de l'application.
                egui::Frame::new()
                    .fill(theme.bg)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .corner_radius(4)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        if invite.trim().is_empty() {
                            ui.colored_label(theme.dim, s.input_dialog_nothing_asked);
                        } else {
                            ui.label(egui::RichText::new(&invite).monospace());
                        }
                    });

                // Les deux touches relevées avant le champ : un
                // `TextEdit` qui traite Entrée la retire de la file, si
                // bien que la guetter après lui ne verrait plus rien.
                let entrée_frappée = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let échap_frappé = ui.input(|i| i.key_pressed(egui::Key::Escape));
                fermer |= échap_frappé;

                if attend {
                    ui.add_space(10.0);
                    let champ = ui.add(
                        egui::TextEdit::singleline(&mut self.input_dialog_text)
                            .hint_text(s.console_input_hint)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width()),
                    );
                    // Le curseur y va à l'ouverture, une fois : la boîte
                    // s'est ouverte parce qu'on nous demande quelque
                    // chose, et la seule chose à faire est d'écrire. Le
                    // redemander à chaque image empêcherait Échap de
                    // rendre le clavier.
                    if self.input_dialog_focus {
                        champ.request_focus();
                        self.input_dialog_focus = false;
                    }
                    envoyer |= entrée_frappée && (champ.has_focus() || champ.lost_focus());
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if attend {
                        // « Fin d'entrée » à gauche, loin du bouton
                        // qu'on vise : fermer l'entrée d'un programme
                        // est sans retour pour lui, et ce n'est pas ce
                        // qu'on venait faire.
                        if framed_button(ui, self.theme, s.console_eof)
                            .on_hover_text(s.console_eof_hint)
                            .clicked()
                        {
                            fin = true;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if attend {
                            let envoi = egui::Button::new(
                                egui::RichText::new(s.console_send).color(theme.bg),
                            )
                            .fill(theme.accent)
                            .min_size(egui::vec2(110.0, 0.0));
                            envoyer |= ui.add(envoi).clicked();
                            if ui
                                .add(
                                    egui::Button::new(s.input_dialog_later)
                                        .stroke(egui::Stroke::new(1.0, theme.dim))
                                        .min_size(egui::vec2(110.0, 0.0)),
                                )
                                .on_hover_text(s.input_dialog_later_hint)
                                .clicked()
                            {
                                fermer = true;
                            }
                        } else {
                            // Plus rien à répondre : un seul bouton, et
                            // Entrée le presse.
                            let ok =
                                egui::Button::new(egui::RichText::new(s.dialog_ok).color(theme.bg))
                                    .fill(theme.accent)
                                    .min_size(egui::vec2(110.0, 0.0));
                            fermer |= ui.add(ok).clicked() || entrée_frappée;
                        }
                    });
                });
            });

        // Envoyer ne referme pas : c'est tout le point. Le programme
        // reprend, écrit sa réponse — « Incorrect password » —, et la
        // boîte la montre. Refermer aussitôt renvoyait devant une
        // console souvent repliée, où l'on ne voyait rien de ce qu'on
        // venait de provoquer.
        if envoyer {
            let ligne = std::mem::take(&mut self.input_dialog_text);
            self.send_input(Some(ligne));
        }
        if fin {
            self.send_input(None);
        }
        if fermer {
            self.show_input_dialog = false;
        }
    }

    /// Le côté « entrée » de la console.
    ///
    /// Une ligne validée part avec son saut de ligne, parce que c'est ce
    /// qu'un programme attend d'un terminal : il lit jusqu'au `\n`, et une
    /// ligne sans fin de ligne le laisserait croire qu'elle n'est pas
    /// terminée.
    pub(crate) fn console_input_ui(&mut self, ui: &mut egui::Ui, pending: usize) {
        let s = strings(self.lang);
        let waiting = matches!(self.status, Status::WaitingForInput);

        ui.separator();

        // Quand le programme attend, la ligne de saisie se distingue du
        // reste : elle est en bas d'un panneau lui-même en bas, sur un
        // fond de terminal, et l'on ne la trouvait pas — on cherchait
        // « où taper » en regardant la sortie.
        // Quand le programme attend, la zone doit se voir sans qu'on la
        // cherche : c'est le seul moment où l'application demande
        // quelque chose, et le manquer, c'est croire qu'elle a planté.
        // Un liseré d'un pixel et un fond à dix pour cent ne suffisaient
        // pas — sur un panneau déjà sombre, cela passait pour une nuance.
        let cadre = if waiting {
            egui::Frame::new()
                .fill(self.theme.accent.gamma_multiply(0.18))
                .stroke(egui::Stroke::new(2.0, self.theme.accent))
                .corner_radius(4)
                .inner_margin(egui::Margin::symmetric(8, 6))
        } else {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 2))
        };

        let mut send = false;
        let mut relancer = false;
        cadre.show(ui, |ui| {
            if waiting {
                ui.horizontal(|ui| {
                    // Une pastille pleine devant la phrase : l'oeil s'y pose
                    // avant de lire, et c'est ce qu'on veut d'un état qui ne
                    // dure que le temps d'une réponse.
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(10.0, ui.text_style_height(&egui::TextStyle::Body)),
                        egui::Sense::hover(),
                    );
                    ui.painter()
                        .circle_filled(rect.center(), 4.0, self.theme.accent);
                    ui.colored_label(
                        self.theme.accent,
                        egui::RichText::new(s.console_waiting).strong(),
                    );
                    ui.colored_label(self.theme.dim, s.console_waiting_how);
                });
            }

            // Les deux lignes — les arguments, puis l'entrée — sont bâties
            // pareil : un libellé de largeur commune, un champ qui prend ce
            // qui reste, des boutons calés à droite. Sans cette largeur
            // commune les deux champs commençaient à des abscisses
            // différentes ; sans le calage à droite, « Recharger » sortait
            // du cadre.
            let libellés = [s.console_args, s.console_input_label];
            let colonne = libellés
                .iter()
                .map(|texte| {
                    ui.painter()
                        .layout_no_wrap(
                            (*texte).to_owned(),
                            egui::TextStyle::Body.resolve(ui.style()),
                            egui::Color32::PLACEHOLDER,
                        )
                        .rect
                        .width()
                })
                .fold(0.0_f32, f32::max);

            // Les arguments : l'autre chose qu'un programme attend, et la
            // première pour un crackme, qui lit souvent son mot de passe
            // dans `argv[1]` plutôt que sur l'entrée standard. Ils ne se
            // tapent pas pendant qu'il tourne — ils sont posés sur la pile
            // avant la première instruction —, d'où le bouton qui recharge.
            if self
                .assemble_state
                .as_ref()
                .is_some_and(|état| état.from_binary.is_some())
            {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(colonne, ui.spacing().interact_size.y),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.colored_label(self.theme.dim, s.console_args);
                        },
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let appliquer = framed_button(ui, self.theme, s.console_args_apply)
                            .on_hover_text(s.console_args_apply_hint)
                            .clicked();
                        let réponse = ui.add(
                            egui::TextEdit::singleline(&mut self.program_args)
                                .hint_text(s.console_args_hint)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(ui.available_width()),
                        );
                        let entrée =
                            réponse.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        relancer = appliquer || entrée;
                    });
                });
                // L'environnement, replié : on s'en sert moins souvent que
                // des arguments, et une ligne de plus toujours ouverte
                // repousserait la saisie vers le bas.
                let ouvert = egui::CollapsingHeader::new(s.console_env)
                    .id_salt("console_env")
                    .open(Some(self.console_env_open))
                    .show(ui, |ui| {
                        ui.weak(s.console_env_hint);
                        let réponse = ui.add(
                            egui::TextEdit::multiline(&mut self.program_env)
                                .hint_text("SECRET=sesame")
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(2)
                                .desired_width(f32::INFINITY),
                        );
                        if réponse.lost_focus() {
                            relancer = true;
                        }
                    });
                if ouvert.header_response.clicked() {
                    self.console_env_open = !self.console_env_open;
                }
                ui.add_space(2.0);
            }
            ui.horizontal(|ui| {
                // La hauteur de la case DÉPASSE tout ce que la rangée porte.
                //
                // « Entrée : » se posait quatre pixels et demi au-dessus du
                // champ et des deux boutons : une rangée horizontale centre
                // chaque élément sur la hauteur qu'elle a À CET INSTANT et ne
                // recale pas ce qui est déjà posé. L'étiquette venant en
                // premier, elle se centrait sur elle-même. Imposer sa propre
                // hauteur n'y changeait rien — c'est la hauteur de la RANGÉE
                // qu'il faut fixer, et le seul moyen est que le premier
                // élément soit déjà le plus haut.
                //
                // TRENTE-QUATRE PIXELS, et c'est une mesure, non un calcul :
                // c'est la hauteur que la rangée prend d'elle-même une fois
                // le champ et les deux boutons posés. Essayé 27, 30 et 34 —
                // à 27 l'étiquette reste 3,5 px trop haut, à 30 il en reste
                // 2, à 34 elle tombe pile.
                //
                // Un nombre mesuré n'est acceptable que si quelque chose le
                // retient : l'invariant « une rangée de commandes se lit sur
                // une seule ligne » le fait, et dira le jour où il cessera
                // d'être juste.
                let rangée = ui.spacing().interact_size.y + 9.0;
                ui.allocate_ui_with_layout(
                    egui::vec2(colonne, rangée),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_height(rangée);
                        // En accent et en gras pendant l'attente : c'est la
                        // ligne où il faut taper, et rien d'autre à l'écran
                        // ne le dit.
                        if waiting {
                            ui.colored_label(
                                self.theme.accent,
                                egui::RichText::new(s.console_input_label).strong(),
                            );
                        } else {
                            ui.colored_label(self.theme.dim, s.console_input_label);
                        }
                    },
                );
                // Les boutons sont posés en premier, calés à droite, et le
                // champ prend ce qui reste. Deviner sa largeur (« la place
                // disponible moins 120 px ») la surestimait de quelques
                // pixels : la ligne débordait du panneau latéral, et ce
                // débordement était repris comme sa nouvelle largeur à la
                // frame suivante. L'onglet Console élargissait donc le
                // panneau sans fin, jusqu'à ce qu'il occupe la fenêtre.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Les deux touches relevées **avant** le champ, et sans
                    // les consommer : un `TextEdit` qui traite Entrée la
                    // retire de la file, si bien que la guetter après lui ne
                    // voyait plus rien. Le champ gardait le texte, le
                    // programme continuait d'attendre, et il fallait cliquer
                    // « Envoyer » — ce qui est exactement ce qu'on voulait
                    // éviter.
                    let entrée_frappée = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let échap_frappé = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    // Fin d'entrée : un programme qui lit jusqu'au bout de son
                    // entrée attendrait indéfiniment sans ce bouton.
                    if framed_button(ui, self.theme, s.console_eof)
                        .on_hover_text(s.console_eof_hint)
                        .clicked()
                    {
                        self.send_input(None);
                    }
                    send = framed_button(ui, self.theme, s.console_send).clicked();

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.console_input)
                            .hint_text(s.console_input_hint)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width()),
                    );
                    // Le programme réclame de quoi lire : le curseur va là où
                    // l'utilisateur doit taper, sans qu'il ait à le chercher.
                    // Échap le lui reprend — sans cette porte, le champ
                    // gardait toutes les touches tant que durait l'attente,
                    // et l'éditeur ne répondait plus : « i » s'écrivait ici
                    // au lieu de passer en Insertion.
                    //
                    // `lost_focus` et non `has_focus` : egui rend déjà le
                    // focus du champ sur Échap. Le guetter ici sur le focus
                    // encore détenu ne voyait donc rien, et la reprise plus
                    // bas le récupérait dans la même frame — la touche
                    // n'avait aucun effet visible.
                    if response.lost_focus() && échap_frappé {
                        self.console_input_released = true;
                        // Personne ne prend le focus derrière : c'est
                        // l'absence de champ focalisé qui réarme les
                        // commandes de l'éditeur.
                        self.editor_mode = EditorMode::Normal;
                    } else if response.has_focus() {
                        // Revenu dedans de son plein gré : l'attente suivante
                        // pourra de nouveau l'y amener.
                        self.console_input_released = false;
                    } else if waiting && !self.console_input_released && !self.show_input_dialog {
                        // Pas tant que la boîte est ouverte : c'est elle qui
                        // demande, et deux champs qui réclament le clavier
                        // dans la même image se l'arrachent.
                        response.request_focus();
                    }
                    // Le champ a le focus et l'on vient de frapper Entrée :
                    // c'est un envoi. On ne demande pas `lost_focus`, qui
                    // dépend de ce qu'egui a décidé de faire du focus, ni
                    // `key_pressed` après coup, qui ne voit plus rien.
                    send |= entrée_frappée && (response.has_focus() || response.lost_focus());
                });
            });

            if pending > 0 {
                ui.colored_label(
                    self.theme.dim,
                    if self.lang == Lang::Fr {
                        format!("{pending} octet(s) en attente d'être lus.")
                    } else {
                        format!("{pending} byte(s) waiting to be read.")
                    },
                );
            } else if !waiting && !self.program_reads {
                // Le champ est là, mais rien ne le lira tel que le programme
                // est écrit. Le dire plutôt que de faire disparaître le
                // champ : celui qui prépare son entrée avant d'avoir tapé
                // son `read` doit pouvoir la taper, et comprendre pourquoi
                // elle attend.
                ui.colored_label(self.theme.dim, s.console_no_read);
            }
        });

        // Reposer les arguments demande de refaire la pile : c'est un
        // rechargement, et le dire vaut mieux que de le faire en
        // silence sous un programme à demi exécuté.
        if relancer {
            self.do_reset();
        }
        if send {
            let line = std::mem::take(&mut self.console_input);
            self.send_input(Some(line));
        }
    }

    /// Vue d'ensemble des crates moteur (core/asm/disasm/emu/re), un peu
    /// comme un gestionnaire de plugins — namsdec-exercises n'y figure pas :
    /// il deviendra un module séparé, ajoutable indépendamment.
    pub(crate) fn modules_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let fr = self.lang == Lang::Fr;

        module_card(
            ui,
            "namsdec-core",
            namsdec_core::VERSION,
            true,
            theme.good,
            s.module_core_detail,
        );

        let (asm_ok, asm_status) = match &self.nasm_info {
            Some(info) => (
                true,
                if fr {
                    format!("nasm détecté — {}", info.version)
                } else {
                    format!("nasm detected — {}", info.version)
                },
            ),
            None => (
                false,
                if fr {
                    "nasm introuvable dans le PATH".to_string()
                } else {
                    "nasm not found in PATH".to_string()
                },
            ),
        };
        module_card(
            ui,
            "namsdec-asm",
            namsdec_asm::VERSION,
            asm_ok,
            if asm_ok { theme.good } else { theme.bad },
            &asm_status,
        );

        let disasm_status = match &self.assemble_state {
            Some(state) => {
                if fr {
                    format!(
                        "{} instructions désassemblées (vérification)",
                        state.disasm.len()
                    )
                } else {
                    format!(
                        "{} instructions disassembled (verification)",
                        state.disasm.len()
                    )
                }
            }
            None => s.no_binary_assembled.to_string(),
        };
        module_card(
            ui,
            "namsdec-disasm",
            namsdec_disasm::VERSION,
            self.assemble_state.is_some(),
            theme.accent,
            &disasm_status,
        );

        let emu_status = match &self.emulator {
            Some(emulator) => {
                let state_word = match (emulator.cpu().halted, fr) {
                    (true, true) => "arrêté (hlt)",
                    (true, false) => "halted (hlt)",
                    (false, true) => "en cours",
                    (false, false) => "running",
                };
                let bp_word = if fr {
                    "point(s) d'arrêt"
                } else {
                    "breakpoint(s)"
                };
                format!(
                    "RIP={:#x} · {} {bp_word} · {state_word}",
                    emulator.cpu().regs.rip,
                    emulator.breakpoints().len(),
                )
            }
            None => s.no_program_loaded.to_string(),
        };
        module_card(
            ui,
            "namsdec-emu",
            namsdec_emu::VERSION,
            self.emulator.is_some(),
            theme.accent,
            &emu_status,
        );

        let titles = self.re_registry.titles(self.lang.code());
        let re_status = if titles.is_empty() {
            s.no_re_module_registered.to_string()
        } else if fr {
            format!("{} module(s) : {}", titles.len(), titles.join(", "))
        } else {
            format!("{} module(s): {}", titles.len(), titles.join(", "))
        };
        module_card(
            ui,
            "namsdec-re",
            namsdec_re::VERSION,
            true,
            theme.good,
            &re_status,
        );

        // Les modules d'analyse, un par ligne plutôt qu'en énumération
        // dans la carte : à neuf titres séparés par des virgules, on ne
        // lisait plus lequel était là ni ce qu'il fait.
        if !titles.is_empty() {
            ui.add_space(2.0);
            // Repliés : neuf lignes de plus allongeaient la page d'autant,
            // et l'on vient d'abord y voir l'état des modules, non la
            // liste de ce que l'analyse sait faire.
            egui::CollapsingHeader::new(format!("{} ({})", s.modules_re_detail, titles.len()))
                .id_salt("modules_re_detail")
                .show(ui, |ui| {
                    for titre in &titles {
                        ui.horizontal(|ui| {
                            status_dot(ui, theme.good);
                            ui.label(*titre);
                        });
                    }
                });
            ui.add_space(6.0);
        }

        // Ce que namsdec porte en propre, et qui n'avait aucune carte :
        // l'explorateur, la documentation, les exercices, les thèmes et
        // la recherche de mise à jour. L'onglet annonçait les modules de
        // namsdec et en montrait la moitié.
        module_card(
            ui,
            "namsdec-explorer",
            namsdec_explorer::VERSION,
            self.explorer.root.is_some(),
            theme.accent,
            &match self.explorer.root.as_ref() {
                Some(racine) => format!("{} {}", s.modules_explorer_root, racine.display()),
                None => s.modules_explorer_none.to_string(),
            },
        );

        module_card(
            ui,
            "namsdec-doc",
            namsdec_doc::VERSION,
            true,
            theme.good,
            &if fr {
                format!("{} fiches d'instructions", namsdec_doc::count())
            } else {
                format!("{} instruction entries", namsdec_doc::count())
            },
        );

        let exercices = self
            .exercise_packs
            .iter()
            .map(|pack| pack.exercises.len())
            .sum::<usize>();
        module_card(
            ui,
            "namsdec-exercises",
            namsdec_exercises::VERSION,
            self.exercises,
            theme.accent,
            &if self.exercises {
                if fr {
                    format!(
                        "{} série(s), {exercices} exercice(s)",
                        self.exercise_packs.len()
                    )
                } else {
                    format!(
                        "{} pack(s), {exercices} exercise(s)",
                        self.exercise_packs.len()
                    )
                }
            } else {
                s.modules_inactive.to_string()
            },
        );

        let thèmes = namsdec_theme::all().len();
        module_card(
            ui,
            "namsdec-theme",
            namsdec_theme::VERSION,
            true,
            theme.good,
            &if fr {
                format!(
                    "{thèmes} thème(s), dont {} venu(s) de fichiers",
                    self.theme_files.len()
                )
            } else {
                format!(
                    "{thèmes} theme(s), {} of them from files",
                    self.theme_files.len()
                )
            },
        );

        let (maj_ok, maj_couleur, maj_texte) = match &self.update_result {
            Some(Ok(verdict)) => (true, theme.good, format!("{verdict:?}")),
            Some(Err(raison)) => (false, theme.bad, raison.clone()),
            None => (false, theme.dim, s.modules_update_idle.to_string()),
        };
        module_card(
            ui,
            "namsdec-update",
            namsdec_update::VERSION,
            maj_ok,
            maj_couleur,
            &maj_texte,
        );

        // Et ce qui vient de fichiers : thèmes et outils déposés dans le
        // dossier des greffons. Ils ne figuraient que dans l'onglet
        // Plugins, alors qu'ils sont chargés au même titre que le reste.
        ui.add_space(4.0);
        ui.strong(s.modules_plugins);
        ui.weak(s.modules_plugins_hint);
        ui.add_space(2.0);
        let mut venus_de_fichiers = false;
        for (id, chemin) in &self.theme_files {
            venus_de_fichiers = true;
            ui.horizontal(|ui| {
                status_dot(ui, theme.accent);
                ui.label(id);
                ui.weak(chemin.display().to_string());
            });
        }
        for outil in &self.decompilers {
            venus_de_fichiers = true;
            ui.horizontal(|ui| {
                status_dot(ui, theme.accent);
                ui.label(&outil.name);
                ui.weak(&outil.command);
            });
        }
        if !venus_de_fichiers {
            ui.weak(s.modules_none_loaded);
        }
    }
}
