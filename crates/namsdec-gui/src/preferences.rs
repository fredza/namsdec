//! Les préférences : une fenêtre, cinq pages.
//!
//! Tout ce qu'on règle ici part dans `settings.json` à l'instant où on
//! le règle — la règle du projet est qu'un paramètre visible est un
//! paramètre persisté, sans quoi l'utilisateur le repose à chaque
//! démarrage sans comprendre pourquoi.

use super::*;

impl NamsdecApp {
    pub(crate) fn preferences_window_ui(&mut self, ctx: &egui::Context) {
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_preferences;
        let mut close_keeping: Option<bool> = None;
        egui::Window::new(s.preferences_title)
            // Un identifiant explicite : sans lui, egui dérive celui de
            // la fenêtre de son *titre*, qui change avec la langue —
            // passer en anglais lui faisait perdre sa position et sa
            // taille.
            .id(egui::Id::new("preferences_window"))
            .open(&mut open)
            .resizable(true)
            // Pas de flèche de repli : replier une boîte de préférences
            // ne laisse qu'un bandeau de titre qui flotte, sans usage.
            .collapsible(false)
            // Une taille posée, et au centre. Sans taille, egui ouvrait
            // la boîte là où il restait de la place — trois cent
            // cinquante pixels de haut, où cinq fiches ne tenaient pas.
            .default_size([780.0, 760.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.preferences_title) {
                    close_keeping = Some(true);
                }
                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (PreferencesTab::General, s.preferences_tab_general),
                        (PreferencesTab::Shortcuts, s.preferences_tab_shortcuts),
                        (PreferencesTab::Modules, s.preferences_tab_modules),
                        (PreferencesTab::Plugins, s.preferences_tab_plugins),
                        (PreferencesTab::Tools, s.preferences_tab_tools),
                    ] {
                        ui.selectable_value(&mut self.preferences_tab, tab, label);
                    }
                });
                ui.separator();
                // Une seule zone qui défile, et qui s'arrête avant le
                // bas : les pages avaient chacune la leur, prenant toute
                // la hauteur offerte, et les boutons OK / Quitter
                // sortaient de la fenêtre par le bas.
                egui::ScrollArea::vertical()
                    .id_salt("preferences_page")
                    .auto_shrink([false, true])
                    .max_height((ui.available_height() - 44.0).max(160.0))
                    .show(ui, |ui| match self.preferences_tab {
                        PreferencesTab::General => self.preferences_general_ui(ui),
                        PreferencesTab::Shortcuts => self.preferences_shortcuts_ui(ui),
                        PreferencesTab::Modules => self.preferences_modules_ui(ui),
                        PreferencesTab::Plugins => self.preferences_plugins_ui(ui),
                        PreferencesTab::Tools => self.preferences_tools_ui(ui),
                    });
                ui.separator();
                // Les préférences s'appliquent à mesure qu'on coche :
                // « Annuler » n'y défait rien, et un bouton qui ne défait
                // rien ment. D'où « Quitter » — qui dit ce qu'il fait —,
                // et non « Annuler ».
                // `or` et non une affectation sèche : la pastille du
                // bandeau a peut-être déjà répondu plus haut.
                close_keeping = close_keeping.or_else(|| {
                    dialog_bar(ui, theme, s.dialog_ok, Some(s.dialog_quit))
                        .map(|réponse| réponse == Answer::Ok)
                });
            });

        match close_keeping {
            Some(keep) => self.close_preferences(ctx, keep),
            // La croix du bandeau vaut OK : refermer une boîte ne doit pas
            // défaire en silence ce qu'on vient d'y régler.
            None if !open => self.close_preferences(ctx, true),
            None => {}
        }
    }

    pub(crate) fn preferences_general_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.horizontal(|ui| {
            ui.label(s.preferences_theme);
            ui.strong(self.theme.name);
        });
        ui.weak(s.preferences_theme_in_plugins);

        ui.separator();

        // La décoration de la fenêtre : celle que namsdec dessine — les
        // menus font office de barre de titre — ou celle du bureau.
        ui.checkbox(&mut self.system_titlebar, s.preferences_system_titlebar);
        ui.weak(s.preferences_system_titlebar_hint);

        ui.add_space(6.0);
        ui.checkbox(&mut self.heat_legend, s.preferences_heat_legend);
        ui.weak(s.preferences_heat_legend_hint);

        ui.add_space(6.0);
        ui.checkbox(&mut self.show_tooltips, s.preferences_tooltips);
        ui.weak(s.preferences_tooltips_hint);

        ui.add_space(6.0);
        ui.checkbox(&mut self.completion, s.preferences_completion);
        ui.weak(s.preferences_completion_hint);
        // Le même blanc avant chacun : il manquait devant les quatre
        // derniers, et la colonne changeait de rythme à mi-liste —
        // 13,5 px trois fois puis 7,5 px trois fois. Huit réglages du
        // même rang, deux respirations : on ne sait pas le nommer en le
        // voyant, mais on voit que quelque chose ne va pas.
        ui.add_space(6.0);
        ui.checkbox(&mut self.auto_indent, s.preferences_auto_indent);
        ui.weak(s.preferences_auto_indent_hint);
        ui.add_space(6.0);
        ui.checkbox(&mut self.auto_pairs, s.preferences_auto_pairs);
        ui.weak(s.preferences_auto_pairs_hint);
        ui.add_space(6.0);
        ui.checkbox(&mut self.hover_help, s.preferences_hover_help);
        ui.weak(s.preferences_hover_help_hint);

        ui.separator();

        ui.label(s.preferences_language);
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.lang == Lang::Fr, "Français")
                .clicked()
            {
                self.lang = Lang::Fr;
            }
            if ui
                .selectable_label(self.lang == Lang::En, "English")
                .clicked()
            {
                self.lang = Lang::En;
            }
        });
    }

    pub(crate) fn preferences_shortcuts_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let ctx = ui.ctx().clone();
        // Relevé ici, agi à la fin : ouvrir un sélecteur de fichier
        // emprunte `self` en écriture.
        let mut exporter_raccourcis = false;

        // Une capture en cours avance ici plutôt que dans le dispatch
        // normal (voir App::ui, qui saute le dispatch tant que
        // `self.rebinding` est `Some`).
        if let Some(rebinding) = &mut self.rebinding
            && let Some(chord) = self.dispatcher.poll_rebind(&ctx, rebinding)
        {
            let action = rebinding.action;
            self.keybindings.map.insert(action, chord);
            self.rebinding = None;
            // Écrit tout de suite : un raccourci qu'on vient de poser et
            // qui disparaît au prochain lancement se repose deux fois.
            let keymap = settings::Keymap::from_bindings(self.keymap_preset, &self.keybindings);
            self.write_keymap(&keymap);
        }

        // Le jeu dont on part. Les deux sont dans le binaire : en changer
        // remplace les raccourcis qu'on n'a pas déplacés soi-même, et
        // garde les autres.
        let fr = self.lang == Lang::Fr;
        ui.horizontal_wrapped(|ui| {
            ui.strong(s.shortcuts_preset);
            for preset in keybindings::Preset::ALL {
                let choisi = self.keymap_preset == preset;
                if ui
                    .selectable_label(choisi, preset.label(fr))
                    .on_hover_text(preset.about(fr))
                    .clicked()
                    && !choisi
                {
                    self.apply_keymap_preset(preset);
                }
            }
        });
        ui.weak(self.keymap_preset.about(fr));
        if let Some(chemin) = settings::keymap_path() {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, egui::RichText::new(s.shortcuts_file).small());
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(chemin.display().to_string())
                            .monospace()
                            .small()
                            .color(theme.dim),
                    )
                    .selectable(true),
                );
            });
        }
        ui.add_space(4.0);
        // Le jeu entier, à plat. Le fichier de réglages ne garde que les
        // écarts — c'est ce qu'on veut d'un réglage, qui suit alors le
        // jeu quand celui-ci gagne une action. Mais un fichier de deux
        // lignes ne dit pas ce que namsdec sait faire, et l'on n'a rien
        // à quoi se raccrocher pour partir d'une liste complète, la
        // relire, ou la donner à quelqu'un.
        if framed_button(ui, theme, s.shortcuts_export)
            .on_hover_text(s.shortcuts_export_hint)
            .clicked()
        {
            exporter_raccourcis = true;
        }
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        egui::Grid::new("shortcuts_grid")
            .num_columns(4)
            .striped(true)
            .show(ui, |ui| {
                for action in Action::ALL {
                    let is_rebinding = self.rebinding.as_ref().is_some_and(|r| r.action == action);

                    ui.label(action_label(s, action));

                    if is_rebinding {
                        // Une séquence n'a pas de longueur imposée : sans
                        // voir ce qu'il a déjà tapé, l'utilisateur ne sait
                        // pas s'il doit continuer ou valider.
                        let captured: Vec<&str> = self
                            .rebinding
                            .as_ref()
                            .map(|r| r.captured().iter().map(|k| k.name()).collect())
                            .unwrap_or_default();
                        if captured.is_empty() {
                            ui.colored_label(theme.accent, s.rebind_waiting);
                        } else {
                            ui.colored_label(theme.accent, format!("{} …", captured.join(" ")));
                        }
                    } else {
                        let current = self
                            .keybindings
                            .map
                            .get(&action)
                            .map(|c| c.display(&ctx))
                            .unwrap_or_default();
                        ui.monospace(current);
                    }

                    if ui
                        .add_enabled(!is_rebinding, framed(theme, s.rebind_simple))
                        .clicked()
                    {
                        self.rebinding =
                            Some(Rebinding::new(action, keybindings::RebindMode::Simple));
                    }
                    if ui
                        .add_enabled(!is_rebinding, framed(theme, s.rebind_leader))
                        .clicked()
                    {
                        self.rebinding =
                            Some(Rebinding::new(action, keybindings::RebindMode::Leader));
                    }
                    ui.end_row();
                }
            });
        if exporter_raccourcis {
            self.spawn_file_dialog(FileDialogKind::ExportKeymap);
        }
    }

    pub(crate) fn preferences_modules_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.label(s.modules_legend);
        ui.add_space(6.0);
        {
            {
                self.modules_ui(ui);
                ui.checkbox(&mut self.show_re_tab, s.show_re_tab_checkbox);
                ui.checkbox(&mut self.exercises, s.exercises_checkbox);
                ui.weak(s.exercises_hint);
                ui.add_space(6.0);
                // Ce module-ci n'ajoute pas un affichage : il accorde la
                // permission de lancer un programme extérieur. On le dit
                // avant de cocher, pas après.
                ui.checkbox(&mut self.show_decompiler_module, s.decompiler_checkbox);
                ui.weak(s.decompiler_hint);
                // Et celui-ci va plus loin encore : il télécharge et
                // construit. Il ne s'allume que sous l'autre, parce
                // qu'installer un décompilateur sans pouvoir s'en servir
                // n'aurait aucun sens.
                ui.add_enabled_ui(self.show_decompiler_module, |ui| {
                    ui.checkbox(&mut self.show_installer_module, s.installer_checkbox);
                    ui.weak(s.installer_hint);
                });
                if self.show_decompiler_module && self.decompilers.is_empty() {
                    ui.colored_label(self.theme.warn, s.decompiler_none_installed);
                }
            }
        }
    }

    /// L'onglet des outils : tout ce qui vient d'ailleurs et s'exécute.
    pub(crate) fn preferences_tools_ui(&mut self, ui: &mut egui::Ui) {
        self.known_tools_ui(ui);
    }

    /// Onglet Plugins : pour l'instant les thèmes, qui en sont le premier
    /// type. Un thème est une simple palette (`namsdec-theme`), donc le
    /// jour où d'autres s'installent, ils se contentent d'allonger la
    /// liste — rien ici n'est écrit par thème.
    pub(crate) fn preferences_plugins_ui(&mut self, ui: &mut egui::Ui) {
        // Toute la page défile d'un seul tenant. Auparavant, seule la
        // liste des thèmes le faisait : ce qui venait après — les outils
        // d'analyse, les séries d'exercices — se retrouvait poussé hors
        // de la fenêtre, présent mais invisible. Une section qu'on ne
        // peut pas atteindre n'existe pas.
        self.plugins_page_ui(ui);
    }

    /// Le contenu de la page des plugins, dans l'ordre où on le lit.
    pub(crate) fn plugins_page_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // Le thème *de l'interface*, à ne pas confondre avec ceux que la
        // page liste — nommés `listed` plus bas pour cette raison.
        let theme = self.theme;
        let ctx = ui.ctx().clone();

        ui.strong(s.plugins_themes);
        ui.weak(s.plugins_themes_hint);
        ui.add_space(6.0);

        {
            {
                let themes = namsdec_theme::all();
                let card_width = theme_card_width(ui, &themes, s.plugins_active);
                for listed in themes {
                    ui.horizontal(|ui| {
                        let active = std::ptr::eq(self.theme, listed);
                        let response = theme_card(ui, listed, active, s.plugins_active, card_width);
                        if response.clicked() && !active {
                            self.theme = listed;
                            self.apply_theme(&ctx);
                        }
                        // Seuls les thèmes venus d'un fichier se
                        // retirent : les trois d'origine ne figurent pas
                        // dans cette table, aucun bouton ne les propose
                        // donc. Et comme les vignettes ont toutes la même
                        // largeur, les boutons tombent les uns sous les
                        // autres.
                        if self.theme_files.contains_key(listed.id)
                            && framed_button(ui, theme, s.plugins_remove).clicked()
                        {
                            self.theme_to_remove = Some(listed.id.to_owned());
                        }
                    });
                    ui.add_space(4.0);
                }
            }
        }

        // Le retrait attend une confirmation : supprimer un fichier ne
        // se fait pas d'un seul clic, surtout à côté de celui qui
        // applique le thème.
        if let Some(id) = self.theme_to_remove.clone() {
            ui.separator();
            ui.colored_label(
                theme.warn,
                format!(
                    "{} {}",
                    s.plugins_remove_question,
                    namsdec_theme::by_id(&id).name
                ),
            );
            ui.horizontal(|ui| {
                if framed_button(ui, theme, s.plugins_remove_yes).clicked() {
                    self.remove_theme(&id, &ctx);
                    self.theme_to_remove = None;
                }
                if framed_button(ui, theme, s.plugins_remove_no).clicked() {
                    self.theme_to_remove = None;
                }
            });
        }

        ui.add_space(6.0);
        ui.separator();
        if framed_button(ui, theme, s.plugins_add).clicked() {
            self.spawn_file_dialog(FileDialogKind::AddTheme);
        }
        ui.weak(s.plugins_add_hint);

        // Les séries d'exercices s'installent de la même façon : un
        // fichier JSON, copié dans le dossier de données, relu au
        // lancement suivant. Même page, même gestes.
        ui.add_space(10.0);
        ui.separator();
        ui.label(egui::RichText::new(s.exercises_packs_title).color(theme.dim));
        self.exercise_packs_ui(ui);
        // Sans thème installé, rien ne porte de bouton « Retirer » : la
        // page le dit, plutôt que de laisser croire qu'il manque quelque
        // chose.
        if self.theme_files.is_empty() {
            ui.weak(s.plugins_none_installed);
        }
        // Le compte rendu de la dernière installation reste affiché
        // jusqu'à la suivante : une boîte modale de plus pour dire « c'est
        // fait » n'apprendrait rien, un refus doit en revanche pouvoir
        // être relu.
        if let Some(note) = &self.plugin_note {
            let colour = if note.ok { theme.good } else { theme.bad };
            ui.colored_label(colour, &note.text);
        }
    }
}
