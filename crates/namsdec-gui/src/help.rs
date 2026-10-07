//! Ce que le programme dit de lui-même : l'aide, les exercices, et
//! « À propos ».
//!
//! Trois fenêtres qui n'agissent sur rien et n'existent que pour
//! enseigner — le manuel en ligne et son index, la série d'exercices et
//! les séries qu'on ajoute, et la page qui nomme les licences, les
//! versions et les outils trouvés sur la machine.

use super::*;
// Le parcours vit dans son module ; l'aide y mène.
use crate::exercises::exercises_dir;

impl NamsdecApp {
    /// La liste des séries installées, avec de quoi en ajouter et en
    /// retirer. La série livrée avec l'application n'y figure pas : on ne
    /// retire pas ce qui n'a pas été installé.
    pub(crate) fn exercise_packs_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        if self.exercise_files.is_empty() {
            ui.weak(s.exercises_packs_none);
        }
        // Toutes les séries, celle qui est livrée comprise : chacune
        // s'écarte d'une case. Écarter n'est pas désinstaller — on garde
        // le fichier, on cesse seulement de proposer son contenu, et
        // c'est ce qu'on veut d'une série qu'on a finie.
        let listed: Vec<(String, String, usize, bool)> = self
            .exercise_packs
            .iter()
            .map(|pack| {
                (
                    pack.id.clone(),
                    pack.name.clone(),
                    pack.exercises.len(),
                    self.exercise_files.contains_key(&pack.id),
                )
            })
            .collect();
        for (id, name, count, installed) in listed {
            ui.horizontal(|ui| {
                let mut active = !self.disabled_packs.contains(&id);
                if ui
                    .checkbox(&mut active, format!("{name} ({count})"))
                    .changed()
                {
                    if active {
                        self.disabled_packs.remove(&id);
                    } else {
                        self.disabled_packs.insert(id.clone());
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Seule une série installée se retire : celle qui est
                    // livrée n'est nulle part sur le disque.
                    if installed && framed_button(ui, theme, s.plugins_remove).clicked() {
                        self.pack_to_remove = Some(id.clone());
                    }
                });
            });
        }
        if let Some(id) = self.pack_to_remove.clone() {
            ui.colored_label(
                theme.warn,
                format!("{} {id}", s.exercises_pack_remove_question),
            );
            ui.horizontal(|ui| {
                if framed_button(ui, theme, s.plugins_remove_yes).clicked() {
                    self.remove_exercises(&id);
                    self.pack_to_remove = None;
                }
                if framed_button(ui, theme, s.plugins_remove_no).clicked() {
                    self.pack_to_remove = None;
                }
            });
        }
        if framed_button(ui, theme, s.exercises_packs_add).clicked() {
            self.spawn_file_dialog(FileDialogKind::AddExercises);
        }
        ui.weak(s.exercises_packs_hint);
    }

    pub(crate) fn exercises_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let mut open = self.show_exercises;
        let mut fermer_ok = false;
        let mut load = None;
        let mut fiche: Option<&'static str> = None;
        let fenêtre_exercises_window = egui::Window::new(s.exercises_title)
            // L'identifiant a changé de version, et il le fallait :
            // egui retient la taille d'une fenêtre par son identifiant,
            // si bien qu'élargir la taille de départ n'aurait rien
            // changé pour qui l'a déjà ouverte une fois. Le prix est
            // qu'elle reprend sa place au centre — pour une fenêtre
            // qu'on jugeait trop étroite, c'est ce qu'on veut.
            .id(egui::Id::new("exercises_window_v2"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // 480 px de large étaient trop peu : un énoncé y tombe sur
            // six lignes et la liste des séries se replie en accordéon.
            // La taille suit maintenant l'écran — quatre cinquièmes,
            // plafonnés — au lieu d'un chiffre fixe qui déborde d'un
            // petit écran et se perd sur un grand.
            .default_size(exercises_window_size(ctx))
            .min_size([420.0, 320.0])
            .pivot(egui::Align2::CENTER_CENTER)
            // `default_pos` et non `current_pos` : la seconde replace la
            // fenêtre à chaque frame, et l'on ne peut plus la déplacer.
            .default_pos(default_window_pos(ctx, "exercises"));
        let rendue_exercises_window = self
            .maximizable(fenêtre_exercises_window, Self::EXERCISES_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.exercises_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "exercises") {
                    fermer_ok = true;
                }
                // Aucune série lisible : le parcours est un fichier, et
                // ce fichier n'est pas là. Le dire, avec l'endroit où il
                // devrait être — c'est la seule chose utile ici.
                if self.exercise_packs.is_empty() {
                    ui.colored_label(theme.bad, s.exercises_missing);
                    if let Some(dir) = exercises_dir() {
                        ui.colored_label(
                            theme.dim,
                            egui::RichText::new(
                                dir.join(namsdec_exercises::BUILTIN_FILE)
                                    .display()
                                    .to_string(),
                            )
                            .monospace(),
                        );
                    }
                    ui.add_space(4.0);
                    ui.colored_label(theme.dim, s.exercises_missing_hint);
                    return;
                }
                ui.label(s.exercises_legend);
                // L'étape en cours, en tête : quatre-vingt-seize exercices ne tiennent
                // pas dans la fenêtre, et « où j'en étais » ne sert à
                // rien s'il faut le chercher en défilant.
                if let Some(id) = self.current_exercise.clone()
                    && let Some((rang, total, exercise)) = self.exercise_rank(&id)
                {
                    ui.add_space(6.0);
                    // L'étape suivante, quand il y en a une : un parcours
                    // se suit, et aller à la suite ne doit pas demander
                    // de retrouver son rang dans une liste.
                    let suivante = self.exercise_after(&id);
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.exercises_current);
                        ui.colored_label(theme.accent, format!("{rang}/{total}"));
                        ui.strong(exercise.title.get(lang));
                        if framed_button(ui, theme, s.exercises_resume).clicked() {
                            load = Some(id.clone());
                        }
                        if let Some((suivant_id, titre)) = &suivante
                            && framed_button(ui, theme, s.exercises_next)
                                .on_hover_text(titre)
                                .clicked()
                        {
                            load = Some(suivant_id.clone());
                        }
                    });
                    // Le but et la progression de l'exercice en cours,
                    // juste sous son titre. Ils vivaient dans le bandeau
                    // du processeur, toujours visible — et restaient donc
                    // affichés quand on chargeait un binaire sans rapport.
                    // Leur place est ici, dans la fenêtre qu'on ouvre quand
                    // on fait un exercice, et nulle part ailleurs.
                    self.goal_ui(ui);
                }
                ui.add_space(6.0);
                // De quoi retrouver une étape : trente-six titres se
                // parcourent mal quand on sait déjà lequel on veut
                // revoir. La recherche porte sur le titre et sur
                // l'énoncé — on se souvient rarement du nom exact, plus
                // souvent de ce qu'on y faisait.
                ui.horizontal(|ui| {
                    ui.colored_label(theme.dim, s.exercises_search);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.exercise_query)
                            .desired_width(200.0)
                            .hint_text(s.exercises_search_hint),
                    );
                    if !self.exercise_query.is_empty() && ui.small_button("x").clicked() {
                        self.exercise_query.clear();
                    }
                });
                ui.add_space(4.0);
                // La place que la fenêtre laisse vraiment, et pas une
                // fraction de l'écran : une zone qui se croit plus haute
                // que sa fenêtre ne défile pas — elle déborde, et ce qui
                // dépasse est coupé sans qu'aucune barre ne le dise.
                let room = ui.available_height().max(120.0);
                egui::ScrollArea::vertical()
                    .max_height(room)
                    // `false` en hauteur : une zone qui rétrécit à son
                    // contenu est aussi haute que ses quatre-vingt-seize exercices,
                    // donc ne défile jamais — elle déborde de la fenêtre,
                    // et ce qui dépasse est coupé.
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for pack in self
                            .exercise_packs
                            .iter()
                            .filter(|pack| !self.disabled_packs.contains(&pack.id))
                        {
                            // Une série par repli : on doit pouvoir dire
                            // d'où vient l'exercice qu'on charge, et
                            // ranger celles qu'on n'utilise pas. Ouvertes
                            // par défaut — une liste qu'il faut déplier
                            // pour voir ce qu'elle contient n'aide
                            // personne le premier jour.
                            // Combien de faits sur combien : le premier
                            // chiffre qu'on cherche en rouvrant un
                            // parcours.
                            let faits = pack
                                .exercises
                                .iter()
                                .filter(|exercise| {
                                    self.exercises_done
                                        .contains(&namsdec_exercises::full_id(pack, exercise))
                                })
                                .count();
                            let sous_budget = pack
                                .exercises
                                .iter()
                                .filter(|exercise| {
                                    let id = namsdec_exercises::full_id(pack, exercise);
                                    match (exercise.par, self.exercises_best.get(&id)) {
                                        (Some(budget), Some(meilleur)) => *meilleur < budget,
                                        _ => false,
                                    }
                                })
                                .count();
                            let header = egui::CollapsingHeader::new(
                                egui::RichText::new(format!(
                                    "{}  —  {faits}/{} {} · {sous_budget} {}",
                                    pack.name,
                                    pack.exercises.len(),
                                    s.exercises_done_count,
                                    s.par_under
                                ))
                                .color(theme.dim),
                            )
                            .id_salt(&pack.id)
                            .default_open(true);
                            let total = pack.exercises.len();
                            header.show(ui, |ui| {
                                let mut palier = None;
                                let cherché = self.exercise_query.to_lowercase();
                                for (index, exercise) in pack.exercises.iter().enumerate() {
                                    if !cherché.is_empty() {
                                        let titre = exercise.title.get(lang).to_lowercase();
                                        let énoncé = exercise.description.get(lang).to_lowercase();
                                        if !titre.contains(&cherché)
                                            && !énoncé.contains(&cherché)
                                            && !exercise.id.contains(&cherché)
                                        {
                                            continue;
                                        }
                                    }
                                    // Le palier, en tête de son groupe :
                                    // le parcours va de la découverte à
                                    // la maîtrise, et le dire évite de
                                    // prendre la vingt-huitième étape
                                    // pour la suite naturelle de la
                                    // première.
                                    if palier != Some(exercise.level) {
                                        palier = Some(exercise.level);
                                        ui.add_space(8.0);
                                        // Le palier, avec ce qu'il en
                                        // reste : quarante-quatre
                                        // étapes se lisent par groupes,
                                        // et « 8/10 » dit en un coup
                                        // d'œil où l'on en est.
                                        let (faits_ici, total_ici) = pack
                                            .exercises
                                            .iter()
                                            .filter(|autre| autre.level == exercise.level)
                                            .fold((0usize, 0usize), |(faits, total), autre| {
                                                let id = namsdec_exercises::full_id(pack, autre);
                                                (
                                                    faits
                                                        + usize::from(
                                                            self.exercises_done.contains(&id),
                                                        ),
                                                    total + 1,
                                                )
                                            });
                                        ui.horizontal(|ui| {
                                            ui.colored_label(
                                                theme.accent,
                                                egui::RichText::new(exercise.level.label(lang))
                                                    .strong(),
                                            );
                                            ui.colored_label(
                                                if faits_ici == total_ici {
                                                    theme.good
                                                } else {
                                                    theme.dim
                                                },
                                                format!("{faits_ici}/{total_ici}"),
                                            );
                                        });
                                    }
                                    let id = namsdec_exercises::full_id(pack, exercise);
                                    let en_cours = self.current_exercise.as_deref() == Some(&id);
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            // Le rang dans le parcours : un
                                            // exercice n'est pas un devoir
                                            // isolé, il vient après les
                                            // précédents.
                                            ui.colored_label(
                                                theme.dim,
                                                format!("{}/{total}", index + 1),
                                            );
                                            // Une pastille pour ce qui est
                                            // fait : un parcours sans trace
                                            // de ce qu'on a déjà réussi
                                            // oblige à s'en souvenir soi-
                                            // même, et l'on refait le
                                            // premier trois fois.
                                            let fait = self.exercises_done.contains(&id);
                                            // Trois états, pas de
                                            // médailles : rien tenté,
                                            // réussi, réussi sous le
                                            // budget de la solution
                                            // livrée.
                                            let sous_budget = fait
                                                && match (
                                                    exercise.par,
                                                    self.exercises_best.get(&id),
                                                ) {
                                                    (Some(budget), Some(meilleur)) => {
                                                        *meilleur < budget
                                                    }
                                                    _ => false,
                                                };
                                            let (pastille, _) = ui.allocate_exact_size(
                                                egui::vec2(10.0, 10.0),
                                                egui::Sense::hover(),
                                            );
                                            if fait {
                                                ui.painter().circle_filled(
                                                    pastille.center(),
                                                    4.0,
                                                    theme.good,
                                                );
                                                // Un anneau de plus pour
                                                // ce qui est passé sous
                                                // le budget : la seule
                                                // distinction du
                                                // parcours, et elle se
                                                // mérite en réécrivant.
                                                if sous_budget {
                                                    ui.painter().circle_stroke(
                                                        pastille.center(),
                                                        6.0,
                                                        egui::Stroke::new(1.2, theme.accent),
                                                    );
                                                }
                                            } else {
                                                ui.painter().circle_stroke(
                                                    pastille.center(),
                                                    4.0,
                                                    egui::Stroke::new(1.0, theme.dim),
                                                );
                                            }
                                            if let Some(meilleur) = self.exercises_best.get(&id) {
                                                ui.colored_label(
                                                    theme.dim,
                                                    egui::RichText::new(format!("{meilleur}"))
                                                        .monospace(),
                                                )
                                                .on_hover_text(s.par_hint);
                                            }
                                            ui.strong(exercise.title.get(lang));
                                            if en_cours {
                                                ui.colored_label(theme.accent, s.exercises_current);
                                            }
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if framed_button(ui, theme, s.exercises_load)
                                                        .clicked()
                                                    {
                                                        load = Some(namsdec_exercises::full_id(
                                                            pack, exercise,
                                                        ));
                                                    }
                                                },
                                            );
                                        });
                                        // Les morceaux de code de l'énoncé
                                        // s'écrivent en chasse fixe, comme
                                        // dans les fiches d'instructions.
                                        let texte =
                                            prose(ui, theme, exercise.description.get(lang));
                                        ui.label(texte);
                                        // Les fiches dont l'exercice se
                                        // sert : un exercice n'apprend
                                        // rien sans le manuel à côté, et
                                        // aller le chercher soi-même
                                        // suppose de savoir quoi
                                        // chercher.
                                        let fiches = sheets_used(&exercise.starter_code);
                                        if !fiches.is_empty() {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.colored_label(theme.dim, s.exercises_sheets);
                                                for name in fiches {
                                                    if ui
                                                        .small_button(
                                                            egui::RichText::new(name).monospace(),
                                                        )
                                                        .on_hover_text(s.help_open_sheet)
                                                        .clicked()
                                                    {
                                                        fiche = Some(name);
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                            ui.add_space(4.0);
                        }
                    });
            });
        self.place_maximize_button(ctx, Self::EXERCISES_WINDOW, rendue_exercises_window);
        if let Some(id) = load {
            self.load_exercise(&id);
            open = false;
        }
        if let Some(name) = fiche {
            self.open_help(name);
            self.show_help = true;
        }
        self.show_exercises = open && !fermer_ok;
    }

    pub(crate) fn about_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_about;
        let mut close = false;
        // Relevé dans la fenêtre, agi après : elle emprunte `self`.
        // Sans l'option `reseau`, rien ne le lit ni ne l'écrit : le
        // `cfg` évite deux avertissements plutôt que de les taire.
        #[cfg(feature = "reseau")]
        let mut lancer_recherche = false;
        egui::Window::new(s.about_title)
            // Un identifiant explicite : sans lui, egui dérive celui de
            // la fenêtre de son *titre*, qui change avec la langue —
            // passer en anglais lui faisait perdre sa position.
            .id(egui::Id::new(ABOUT_WINDOW))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            // Posée au centre à chaque frame, et non « ancrée » : l'ancre
            // d'egui aligne la boîte dans la zone qu'il lui laisse, ce
            // qui la décale vers le bas des panneaux. Une boîte « À
            // propos » n'a pas de place à retenir, elle s'ouvre au milieu
            // de la fenêtre.
            .pivot(egui::Align2::CENTER_CENTER)
            .current_pos(ctx.viewport_rect().center())
            // Large : trois boutons, un bandeau de préversion et une
            // recherche de mise à jour ne tiennent pas dans une colonne
            // de trois cents pixels sans se replier en accordéon.
            .default_width(ABOUT_WIDTH)
            .min_width(ABOUT_WIDTH)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.about_title) {
                    close = true;
                }
                ui.set_min_width(ABOUT_WIDTH - 24.0);
                // L'en-tête : l'icône de l'application à gauche — la
                // même que celle du lanceur —, et à droite ce qu'elle
                // est, ce qu'elle vaut, ce qu'elle fait.
                let icône = self.app_icon_texture(ctx);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::from_handle(&icône))
                            .fit_to_exact_size(egui::vec2(APP_ICON_SHOWN, APP_ICON_SHOWN))
                            .corner_radius(12),
                    );
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new("namsdec").strong().size(28.0));
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(format!("v{APP_VERSION}")).monospace(),
                            )
                            .on_hover_text(format!("{} {BUILD_DATE}", s.about_build_word));
                        });
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                        ui.colored_label(theme.dim, s.about_description);
                        ui.add_space(8.0);
                        // Les trois portes du projet : le site, le
                        // salon, le dépôt. Chacune avec sa marque —
                        // dessinée, donc de la couleur du thème et nette
                        // à toute taille.
                        ui.horizontal_wrapped(|ui| {
                            for (label, url, infobulle, dessin) in [
                                (
                                    s.about_site,
                                    SITE_URL,
                                    s.about_site_hint,
                                    chrome::icon_globe
                                        as fn(&egui::Painter, egui::Rect, egui::Color32),
                                ),
                                (
                                    s.about_discord,
                                    DISCORD_URL,
                                    s.about_discord_hint,
                                    chrome::icon_discord,
                                ),
                                (
                                    s.about_github,
                                    GITHUB_URL,
                                    s.about_github_hint,
                                    chrome::icon_github,
                                ),
                            ] {
                                if link_button_ui(ui, theme, label, dessin)
                                    .on_hover_text(format!("{infobulle}\n{url}"))
                                    .clicked()
                                {
                                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                }
                            }
                        });
                    });
                });
                ui.add_space(10.0);

                // Le bandeau des préversions : il dit ce qu'on tient
                // entre les mains, et disparaît de lui-même le jour où
                // la version n'en est plus une.
                if let Some(étiquette) = prerelease(APP_VERSION) {
                    egui::Frame::new()
                        .fill(theme.warn.gamma_multiply(0.15))
                        .stroke(egui::Stroke::new(1.0, theme.warn))
                        .inner_margin(egui::Margin::symmetric(12, 10))
                        .corner_radius(8.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ABOUT_WIDTH - 60.0);
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            ui.colored_label(
                                theme.warn,
                                egui::RichText::new(format!(
                                    "{} — {étiquette}",
                                    s.about_beta_title
                                ))
                                .strong(),
                            );
                            ui.add_space(4.0);
                            ui.colored_label(theme.text, s.about_beta_body);
                        });
                    ui.add_space(10.0);
                }

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme.dim, s.about_license);
                    ui.hyperlink_to(s.about_license_link, MIT_LICENSE_URL);
                });

                ui.separator();
                ui.colored_label(theme.dim, s.about_donate_hint);
                let can_donate = !DONATION_URL.is_empty();
                let texture = self.treat_texture(ctx);
                ui.add_enabled_ui(can_donate, |ui| {
                    // `Image::sense` plutôt qu'un `ImageButton` : egui
                    // 0.36 a retiré ce dernier, une image cliquable le
                    // remplace.
                    let image = egui::Image::new(egui::load::SizedTexture::from_handle(&texture))
                        .fit_to_exact_size(egui::vec2(TREAT_IMAGE_SHOWN, TREAT_IMAGE_SHOWN))
                        .corner_radius(6)
                        .sense(egui::Sense::click());
                    if ui
                        .add(image)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(s.about_donate)
                        .clicked()
                    {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(DONATION_URL));
                    }
                });
                if !can_donate {
                    ui.colored_label(theme.dim, s.about_donate_unset);
                }

                ui.separator();
                // La recherche est un geste, jamais un réflexe : rien
                // n'est demandé à GitHub tant que personne n'a cliqué,
                // et rien n'est envoyé d'autre que la requête.
                //
                // Un binaire bâti sans l'option `reseau` n'en montre pas
                // le bouton : un bouton qui ne peut rien faire est pire
                // qu'une absence, et ici l'absence se comprend — le
                // paquet d'une distribution met à jour lui-même.
                #[cfg(feature = "reseau")]
                ui.horizontal_wrapped(|ui| {
                    ui.add_enabled_ui(self.update_check.is_none(), |ui| {
                        if framed_button(ui, self.theme, s.about_check_update)
                            .on_hover_text(s.about_check_update_hint)
                            .clicked()
                        {
                            lancer_recherche = true;
                        }
                    });
                    if self.update_check.is_some() {
                        ui.colored_label(theme.dim, s.about_update_checking);
                    }
                });
                #[cfg(feature = "reseau")]
                match &self.update_result {
                    Some(Ok(namsdec_update::Verdict::UpToDate)) => {
                        ui.colored_label(theme.good, s.about_update_uptodate);
                    }
                    Some(Ok(namsdec_update::Verdict::Newer(release))) => {
                        ui.colored_label(
                            theme.warn,
                            format!("{} {}", s.about_update_newer, release.tag),
                        );
                        if !release.url.is_empty() {
                            ui.hyperlink_to(s.about_update_open, &release.url);
                        }
                        // Ce que namsdec ne fera pas : se remplacer tout
                        // seul. Dire comment mettre à jour vaut mieux que
                        // de le faire dans le dos de celui qui travaille.
                        ui.colored_label(theme.dim, s.about_update_how);
                    }
                    Some(Ok(namsdec_update::Verdict::Ahead(release))) => {
                        ui.colored_label(
                            theme.dim,
                            format!("{} ({})", s.about_update_ahead, release.tag),
                        );
                    }
                    Some(Err(message)) => {
                        ui.colored_label(theme.bad, message);
                    }
                    None => {}
                }

                ui.separator();
                // Un seul bouton : il n'y a rien à annuler dans une boîte
                // qui ne fait que renseigner. Échap la ferme quand même.
                // `|=` et non `=` : la pastille du bandeau a peut-être
                // déjà répondu plus haut, et une affectation sèche
                // l'effacerait.
                close |= dialog_bar(ui, theme, s.dialog_ok, None).is_some();
            });
        #[cfg(feature = "reseau")]
        if lancer_recherche {
            self.start_update_check();
        }
        self.show_about = open && !close;
        if !self.show_about {
            // La réponse ne survit pas à la fermeture : on rouvre, on
            // redemande — l'état du dépôt a pu changer entre-temps.
            self.update_result = None;
        }
    }

    /// La fiche d'une instruction : ce qu'elle fait, ce qu'elle laisse
    /// dans les indicateurs, et à quoi elle ressemble écrite.
    ///
    /// Les indicateurs sont dits explicitement parce que c'est ce qu'on
    /// oublie : une instruction qui n'en touche aucun n'est pas la même
    /// qu'une qui les écrase tous, et le saut qui suit ne lira pas la
    /// même chose.
    pub(crate) fn help_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let mut open = self.show_help;
        let mut fermer = false;
        egui::Window::new(s.help_title)
            // Un identifiant explicite : le titre change avec la langue,
            // et la fenêtre perdrait sa position en passant à l'anglais.
            .id(egui::Id::new("instruction_help"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // 460 × 320 était trop petit : une fiche s'y lisait par
            // tranches de six lignes, et la liste des résultats tenait
            // en quatre. Une barre de défilement qui parcourt quatre
            // lignes n'est pas une barre, c'est un aveu.
            .default_size([680.0, 560.0])
            .min_size([460.0, 320.0])
            // En bas à droite, loin du curseur : on lit la fiche en
            // continuant d'écrire, elle ne doit pas couvrir la ligne sur
            // laquelle on vient de demander de l'aide.
            .pivot(egui::Align2::RIGHT_BOTTOM)
            .default_pos(ctx.viewport_rect().max - egui::vec2(24.0, 24.0))
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.help_title) {
                    fermer = true;
                }
                // La fiche prend la place qui reste, la barre d'action
                // garde la sienne en bas : sans ce panneau, le bouton
                // était poussé hors de la fenêtre par une fiche longue.
                egui::Panel::bottom("help_actions")
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        ui.add_space(6.0);
                        if dialog_bar(ui, self.theme, strings(self.lang).dialog_ok, None).is_some()
                        {
                            fermer = true;
                        }
                    });
                self.help_ui(ui);
            });
        self.show_help = open && !fermer;
    }

    pub(crate) fn help_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang_code = self.lang.code();

        // Un champ de recherche au-dessus de la fiche : sans lui, on ne
        // peut lire une fiche qu'en écrivant l'instruction dans son
        // code — ce qui suppose de savoir qu'elle existe. Le manuel se
        // consulte aussi pour ce qu'on ne connaît pas encore.
        let mut ouvrir = None;
        // Relevée avant le champ, comme partout ailleurs : un `TextEdit`
        // qui traite Entrée la retire de la file.
        let entrée_frappée = ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.horizontal(|ui| {
            // Un vrai champ de recherche : la loupe devant, et la
            // largeur qui suit la fenêtre. Un champ de 200 px collé aux
            // boutons ne se voyait pas comme un champ.
            //
            // La loupe est dessinée, non écrite : l'emoji n'est dans
            // aucune police chargée par défaut et se peindrait en carré
            // vide — ce qu'un test du projet attrape.
            let (marque, _) = ui.allocate_exact_size(
                egui::vec2(16.0, ui.spacing().interact_size.y),
                egui::Sense::hover(),
            );
            chrome::icon_search(
                ui.painter(),
                egui::Rect::from_center_size(marque.center(), egui::vec2(13.0, 13.0)),
                theme.dim,
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.help_query)
                    .hint_text(s.help_search)
                    .font(egui::TextStyle::Monospace)
                    .desired_width((ui.available_width() - 180.0).max(180.0)),
            );
            if !self.help_query.is_empty() && framed_button(ui, theme, "x").clicked() {
                self.help_query.clear();
            }
            // De quoi revenir sur ses pas, puis au sommaire : une fiche
            // ouverte ne doit pas être un cul-de-sac.
            if !self.help_back.is_empty()
                && framed_button(ui, self.theme, s.help_back)
                    .on_hover_text(s.help_back_hint)
                    .clicked()
                && let Some(precedent) = self.help_back.pop()
            {
                self.help = Some((precedent.clone(), namsdec_doc::lookup(&precedent)));
                self.help_query.clear();
            }
            if self.help.is_some() && framed_button(ui, theme, s.help_index).clicked() {
                self.help = None;
                self.help_back.clear();
                self.help_query.clear();
            }
        });
        if !self.help_query.is_empty() {
            // Classées par pertinence, et sous le nom qui a correspondu.
            // Chercher « pop » rendait quatre fiches dans l'ordre du
            // manuel, celle qu'on voulait nulle part en tête — et
            // affichée sous le nom « push », premier de sa fiche. On
            // cherchait `pop`, on lisait `push`.
            let trouvées = namsdec_doc::search(&self.help_query, lang_code);
            ui.colored_label(theme.dim, format!("{} {}", trouvées.len(), s.help_results));
            // Entrée ouvre la première : c'est celle qu'on voulait dans
            // le cas ordinaire, et cela évite de viser à la souris ce
            // qu'on vient de taper au clavier.
            if entrée_frappée && let Some(première) = trouvées.first() {
                ouvrir = Some(première.name);
            }
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("help_index")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for trouvée in trouvées {
                        let ligne = ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                theme.mnemonic,
                                egui::RichText::new(trouvée.name).monospace().strong(),
                            );
                            ui.label(trouvée.entry.title.get(lang_code));
                            // Les autres noms de la fiche : une fiche
                            // couvre souvent une famille, et le dire
                            // évite de la rouvrir pour chacun.
                            let autres: Vec<&str> = trouvée
                                .entry
                                .names
                                .iter()
                                .copied()
                                .filter(|nom| *nom != trouvée.name)
                                .collect();
                            if !autres.is_empty() {
                                ui.colored_label(theme.dim, small_monospace(ui, autres.join(" ")));
                            }
                        });
                        if ligne
                            .response
                            .interact(egui::Sense::click())
                            .on_hover_text(s.help_open_sheet)
                            .clicked()
                        {
                            ouvrir = Some(trouvée.name);
                        }
                    }
                });
            if let Some(name) = ouvrir {
                self.open_help(name);
                self.help_query.clear();
            }
            return;
        }

        let Some((word, entry)) = &self.help else {
            // Rien sous le curseur : on montre le manuel lui-même. Une
            // documentation qu'on ne peut que *chercher* suppose qu'on
            // sache déjà ce qu'on cherche ; un sommaire se parcourt.
            ui.colored_label(theme.dim, s.help_empty);
            ui.colored_label(theme.dim, s.help_search_hint);
            self.help_index_ui(ui);
            return;
        };
        let Some(entry) = entry else {
            // Pas d'instruction de ce nom — mais dans le listing d'un
            // binaire, le mot sous le curseur est souvent une fonction
            // empruntée. F1 doit répondre là aussi.
            if let Some(note) = namsdec_re::libc::explain(word) {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme.mnemonic, egui::RichText::new(word).monospace());
                    if note.watch {
                        ui.colored_label(theme.changed, s.help_watch);
                    }
                });
                ui.add_space(4.0);
                ui.label(prose(ui, theme, note.text(self.lang.code())));
                ui.add_space(6.0);
                ui.colored_label(theme.dim, s.help_kind_borrowed);
                return;
            }
            ui.colored_label(theme.bad, format!("{} : {word}", s.help_unknown));
            ui.colored_label(theme.dim, s.help_unknown_hint);
            return;
        };
        let lang = if self.lang == Lang::Fr { "fr" } else { "en" };
        let mut copié = false;
        let mut suivante = None;
        let mut à_ouvrir: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("help")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for name in entry.names {
                        ui.colored_label(
                            if name.eq_ignore_ascii_case(word) {
                                theme.changed
                            } else {
                                theme.mnemonic
                            },
                            egui::RichText::new(*name).monospace(),
                        );
                    }
                });
                ui.add_space(4.0);
                ui.strong(entry.title.get(lang));
                ui.add_space(2.0);
                ui.label(prose(ui, theme, entry.body.get(lang)));
                ui.add_space(6.0);
                // Les indicateurs ne concernent que les instructions :
                // une directive ne s'exécute pas, elle n'en touche aucun,
                // et l'écrire reviendrait à laisser croire qu'elle
                // pourrait.
                if entry.kind == namsdec_doc::Kind::Instruction {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.help_flags);
                        if entry.flags.is_empty() {
                            ui.colored_label(theme.dim, s.help_no_flags);
                        } else {
                            ui.colored_label(
                                theme.register,
                                egui::RichText::new(entry.flags).monospace(),
                            );
                        }
                    });
                } else {
                    ui.colored_label(
                        theme.dim,
                        match entry.kind {
                            namsdec_doc::Kind::Directive => s.help_kind_directive,
                            namsdec_doc::Kind::Concept => s.help_kind_concept,
                            _ => s.help_kind_preprocessor,
                        },
                    );
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.colored_label(theme.dim, s.help_example);
                    // L'exemple se copie : on lit une fiche pour essayer
                    // ce qu'elle décrit, et le retaper à la main n'ajoute
                    // rien à ce qu'on en apprend.
                    if ui
                        .small_button(s.help_copy_example)
                        .on_hover_text(s.help_copy_hint)
                        .clicked()
                    {
                        ui.ctx().copy_text(entry.example.to_owned());
                        copié = true;
                    }
                });
                ui.add(
                    egui::Label::new(egui::RichText::new(entry.example).monospace())
                        .wrap_mode(egui::TextWrapMode::Extend),
                );

                // Les renvois : les mots que la fiche cite et qui ont
                // eux-mêmes une fiche. Une notion s'apprend en tirant
                // sur le fil, pas en lisant une page isolée.
                let renvois = namsdec_doc::related(entry, lang);
                if !renvois.is_empty() {
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.help_see_also);
                        for autre in renvois {
                            if ui
                                .small_button(egui::RichText::new(autre.names[0]).monospace())
                                .on_hover_text(autre.title.get(lang))
                                .clicked()
                            {
                                suivante = Some(autre.names[0]);
                            }
                        }
                    });
                }

                // La référence complète, pour le jour où la fiche ne
                // suffit pas. Elle tient en un écran — ce qu'on veut en
                // écrivant du code —, mais quand une instruction se
                // comporte autrement qu'on l'espérait, il faut le
                // détail : tous les encodages, tous les modes, toutes
                // les exceptions. C'est le manuel d'Intel, et le
                // recopier ici serait absurde : deux mille pages, déjà
                // en ligne, instruction par instruction.
                if let Some(url) = namsdec_doc::reference_url(entry) {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.help_reference);
                        if ui
                            .link(
                                egui::RichText::new(format!(
                                    "{} — Intel",
                                    entry.names[0].to_uppercase()
                                ))
                                .monospace(),
                            )
                            .on_hover_text(&url)
                            .clicked()
                        {
                            à_ouvrir = Some(url.clone());
                        }
                    });
                }
            });
        if let Some(name) = suivante {
            self.open_help(name);
        }
        if let Some(url) = à_ouvrir {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
        if copié {
            self.status = Status::Note(|s| s.help_example_copied);
        }
    }

    /// Le sommaire du manuel : les familles, dépliables, et leurs
    /// fiches.
    ///
    /// Chercher suppose de savoir quoi chercher. Parcourir, non — et
    /// c'est ce qu'on fait devant un manuel qu'on découvre.
    pub(crate) fn help_index_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let familles = [
            (namsdec_doc::Kind::Concept, s.help_kind_concepts),
            (namsdec_doc::Kind::Instruction, s.help_kind_instructions),
            (namsdec_doc::Kind::Directive, s.help_kind_directives),
            (namsdec_doc::Kind::Preprocessor, s.help_kind_preprocessors),
        ];

        // Les quatre familles restent visibles quoi qu'on lise : on ne
        // découvre pas un sommaire en faisant défiler ce qu'on ne sait
        // pas encore chercher.
        ui.horizontal_wrapped(|ui| {
            for (kind, titre) in familles {
                let combien = namsdec_doc::all().iter().filter(|e| e.kind == kind).count();
                ui.selectable_value(
                    &mut self.help_index_kind,
                    kind,
                    format!("{titre} ({combien})"),
                );
            }
        });
        ui.separator();

        let fiches: Vec<&namsdec_doc::Entry> = namsdec_doc::all()
            .iter()
            .filter(|entry| entry.kind == self.help_index_kind)
            .collect();
        // Combien de mnémoniques le palier montre, et non combien de
        // fiches : une famille en couvre souvent plusieurs, et c'est le
        // compte des instructions qui répond à « les ai-je toutes ? ».
        let combien_noms: usize = fiches.iter().map(|e| e.names.len()).sum();
        ui.colored_label(theme.dim, format!("{combien_noms} {}", s.help_index_count));
        ui.add_space(2.0);

        let mut ouvrir = None;
        egui::ScrollArea::vertical()
            .id_salt("help_sommaire")
            .max_height(window_room(ui))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in fiches {
                    ui.horizontal_wrapped(|ui| {
                        // TOUS les noms de la fiche, pas seulement le
                        // premier : `jmp` se voyait, mais `je`, `jne`,
                        // `jz`… restaient invisibles tant qu'on ne savait
                        // pas les chercher. Une famille couvre jusqu'à six
                        // mnémoniques, et aucun ne doit manquer au
                        // sommaire. Chacun est cliquable et ouvre la fiche
                        // avec son propre nom surligné.
                        for name in entry.names {
                            if ui
                                .add(
                                    egui::Label::new(
                                        egui::RichText::new(*name)
                                            .monospace()
                                            .color(theme.mnemonic),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                                .on_hover_text(s.help_open_sheet)
                                .clicked()
                            {
                                ouvrir = Some(*name);
                            }
                        }
                        ui.colored_label(theme.dim, entry.title.get(lang));
                    });
                }
            });

        if let Some(name) = ouvrir {
            self.open_help(name);
        }
    }
}
