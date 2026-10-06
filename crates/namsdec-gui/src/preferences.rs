//! Les préférences : une fenêtre, cinq pages.
//!
//! Tout ce qu'on règle ici part dans `settings.json` à l'instant où on
//! le règle — la règle du projet est qu'un paramètre visible est un
//! paramètre persisté, sans quoi l'utilisateur le repose à chaque
//! démarrage sans comprendre pourquoi.

use super::*;

/// La hauteur que la boîte de préférences ne dépasse pas.
///
/// C'est ELLE qui coupe la boucle de mesure, et il a fallu deux essais
/// pour le comprendre. La zone qui défile prend sa hauteur sur la
/// fenêtre, moins la place des boutons ; la fenêtre prend la sienne sur
/// son contenu. Tant que rien ne borne la fenêtre, les deux se relancent
/// l'une l'autre : mesuré, un pixel de plus par image, sans fin, sur les
/// quatre pages plus hautes que la boîte.
///
/// Le premier essai bornait la ZONE sur le viewport. La boucle
/// s'arrêtait, mais la zone ne connaissait plus la place qui lui restait
/// vraiment : elle prenait toute la hauteur, et « OK » et « Quitter »
/// passaient sous le bord. Un contrôle le disait déjà — c'est le
/// défaut qu'il avait été écrit pour empêcher.
///
/// Le second essai bornait la fenêtre par le haut seulement. La boucle
/// devenait finie — elle s'arrêtait au plafond — mais elle tournait
/// encore : deux cent quarante images à grandir d'un pixel, soit quatre
/// secondes de reptation sous les yeux.
///
/// La hauteur est donc IMPOSÉE : plancher et plafond à la même valeur.
/// La fenêtre ne tire plus sa taille de son contenu, la hauteur
/// disponible est une constante, et la zone continue de se régler sur ce
/// qui lui reste — donc la barre du bas reste dedans. La largeur, elle,
/// se règle encore : c'est la dimension dont on a besoin ici.
///
/// Prise sur le viewport, qui ne dépend pas de la fenêtre. Les
/// quatre-vingts pixels gardés sont les marges au-dessus et au-dessous :
/// une boîte collée aux deux bords n'a plus l'air d'une boîte.
fn hauteur_maximale(ctx: &egui::Context) -> f32 {
    /// La hauteur qu'on vise quand l'écran la permet : cinq pages y
    /// tiennent sans défiler, ou presque.
    const VOULUE: f32 = 760.0;
    const MARGES: f32 = 80.0;
    const PLANCHER: f32 = 320.0;
    VOULUE
        .min(ctx.viewport_rect().height() - MARGES)
        .max(PLANCHER)
}

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
            .default_width(780.0)
            // La hauteur est IMPOSÉE — plancher et plafond à la même
            // valeur —, et c'est elle qui coupe la boucle. Voir
            // `hauteur_maximale` pour ce que les deux autres essais ont
            // donné. La largeur, elle, se règle encore : c'est la
            // dimension dont on a besoin ici, pour les longs chemins de
            // fichiers et le tableau des deux jeux.
            .min_height(hauteur_maximale(ctx))
            .max_height(hauteur_maximale(ctx))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.preferences_title) {
                    close_keeping = Some(true);
                }
                // La liste des pages est déclarée une fois, dans
                // `PreferencesTab::ALL` : la rangée d'onglets et le banc
                // de mise en page la lisent au même endroit, et une page
                // nouvelle se trouve mesurée sans qu'on y pense.
                ui.horizontal(|ui| {
                    for tab in PreferencesTab::ALL {
                        ui.selectable_value(&mut self.preferences_tab, tab, tab.label(s));
                    }
                });
                ui.separator();
                // Une seule zone qui défile, et qui s'arrête avant le
                // bas : les pages avaient chacune la leur, prenant toute
                // la hauteur offerte, et les boutons OK / Quitter
                // sortaient de la fenêtre par le bas.
                //
                // Sa hauteur se règle sur ce qui RESTE, et c'est voulu :
                // c'est ainsi que la barre du bas reste dedans quelle
                // que soit la page. La boucle de mesure que cela
                // formait est coupée plus haut, en bornant la fenêtre —
                // voir `hauteur_maximale`.
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

        // Les deux jeux côte à côte, action par action.
        //
        // L'onglet n'en montrait qu'un, et c'est précisément ce qui
        // empêchait de voir les trous et les doublons : on ne compare
        // pas de mémoire deux listes de trente lignes. Côte à côte,
        // « cette action n'a rien dans l'autre jeu » et « ces deux-là
        // répondent à la même frappe » se lisent sans chercher.
        ui.weak(s.shortcuts_both);
        ui.add_space(4.0);

        let jeux: Vec<(keybindings::Preset, keybindings::KeyBindings)> = keybindings::Preset::ALL
            .into_iter()
            .map(|preset| (preset, keybindings::KeyBindings::preset(preset)))
            .collect();

        // Les frappes qui servent deux fois dans le jeu EFFECTIF — celui
        // qui a cours, écarts de l'utilisateur compris. Les jeux livrés
        // n'en ont aucune, un contrôle le tient ; un raccourci qu'on
        // déplace soi-même, lui, peut très bien en écraser un autre, et
        // rien ne le disait.
        let mut par_frappe: BTreeMap<String, Vec<Action>> = BTreeMap::new();
        for (action, chord) in &self.keybindings.map {
            par_frappe
                .entry(chord.display(&ctx))
                .or_default()
                .push(*action);
        }

        egui::Grid::new("shortcuts_grid")
            .num_columns(5)
            .striped(true)
            .show(ui, |ui| {
                ui.strong(s.shortcuts_action_col);
                for (preset, _) in &jeux {
                    // Le jeu actif en clair, l'autre estompé : la colonne
                    // qu'on règle se distingue de celle qu'on compare.
                    let actif = self.keymap_preset == *preset;
                    let texte = egui::RichText::new(preset.label(fr));
                    if actif {
                        ui.strong(texte);
                    } else {
                        ui.colored_label(theme.dim, texte);
                    }
                }
                ui.label("");
                ui.label("");
                ui.end_row();

                for action in Action::ALL {
                    let is_rebinding = self.rebinding.as_ref().is_some_and(|r| r.action == action);

                    ui.label(action_label(s, action));

                    // Ce que la colonne du jeu actif écrit : la colonne
                    // de l'autre s'y compare au lieu de le répéter.
                    let écrit_actif = self
                        .keybindings
                        .map
                        .get(&action)
                        .map(|chord| chord.display(&ctx));

                    for (preset, jeu) in &jeux {
                        let actif = self.keymap_preset == *preset;
                        if actif && is_rebinding {
                            // Une séquence n'a pas de longueur imposée :
                            // sans voir ce qu'il a déjà tapé,
                            // l'utilisateur ne sait pas s'il doit
                            // continuer ou valider.
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
                            continue;
                        }
                        // La colonne du jeu actif montre ce qui a cours —
                        // écart de l'utilisateur compris. L'autre montre
                        // ce que son jeu propose : c'est ce qu'on aurait
                        // en basculant.
                        let effective = actif
                            .then(|| self.keybindings.map.get(&action))
                            .flatten()
                            .or_else(|| jeu.map.get(&action));
                        let Some(chord) = effective else {
                            ui.colored_label(theme.warn, s.shortcuts_missing);
                            continue;
                        };
                        let écrit = chord.display(&ctx);
                        // La même frappe que dans le jeu actif : on le
                        // dit, on ne la recopie pas. Les deux jeux en
                        // partagent une sur trois, et deux fois la même
                        // chose côte à côte n'apprend rien — c'est ce qui
                        // DIFFÈRE qu'on vient lire.
                        if !actif && écrit_actif.as_deref() == Some(écrit.as_str()) {
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(s.shortcuts_same).small(),
                            )
                            .on_hover_text(&écrit);
                            continue;
                        }
                        let déplacé = actif && jeu.map.get(&action) != Some(chord);
                        let partagée = par_frappe
                            .get(&écrit)
                            .is_some_and(|actions| actif && actions.len() > 1);

                        let couleur = if partagée {
                            theme.bad
                        } else if déplacé {
                            theme.changed
                        } else if actif {
                            theme.text
                        } else {
                            theme.dim
                        };
                        ui.horizontal(|ui| {
                            let étiquette = ui.add(egui::Label::new(
                                egui::RichText::new(&écrit).monospace().color(couleur),
                            ));
                            if partagée {
                                let autres: Vec<&'static str> = par_frappe[&écrit]
                                    .iter()
                                    .filter(|autre| **autre != action)
                                    .map(|autre| action_label(s, *autre))
                                    .collect();
                                étiquette.on_hover_text(format!(
                                    "{} {}",
                                    s.shortcuts_conflict,
                                    autres.join(", ")
                                ));
                            } else if déplacé {
                                let proposé = jeu
                                    .map
                                    .get(&action)
                                    .map(|c| c.display(&ctx))
                                    .unwrap_or_else(|| s.shortcuts_missing.to_owned());
                                étiquette.on_hover_text(format!(
                                    "{} {proposé}",
                                    s.shortcuts_preset_said
                                ));
                                ui.colored_label(
                                    theme.changed,
                                    egui::RichText::new(s.shortcuts_changed).small(),
                                );
                            }
                        });
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
        // Deux choses sous le même mot « module », et c'est ce qui prêtait
        // à confusion : d'un côté les COMPOSANTS dont namsdec est fait
        // (sains ou non — on ne les allume pas), de l'autre les FONCTIONS
        // qu'on choisit d'allumer. Un titre pour chacune les sépare.
        ui.strong(s.modules_components_title);
        ui.weak(s.modules_legend);
        ui.add_space(6.0);
        self.modules_ui(ui);

        ui.add_space(12.0);
        ui.strong(s.modules_features_title);
        ui.weak(s.modules_features_hint);
        ui.add_space(6.0);
        ui.checkbox(&mut self.show_re_tab, s.show_re_tab_checkbox);
        ui.checkbox(&mut self.exercises, s.exercises_checkbox);
        ui.weak(s.exercises_hint);
        ui.add_space(6.0);
        // Ce module-ci n'ajoute pas un affichage : il accorde la
        // permission de lancer un programme extérieur. On le dit avant de
        // cocher, pas après.
        ui.checkbox(&mut self.show_decompiler_module, s.decompiler_checkbox);
        ui.weak(s.decompiler_hint);
        // Et celui-ci va plus loin encore : il télécharge et construit. Il
        // ne s'allume que sous l'autre, parce qu'installer un décompilateur
        // sans pouvoir s'en servir n'aurait aucun sens.
        ui.add_enabled_ui(self.show_decompiler_module, |ui| {
            ui.checkbox(&mut self.show_installer_module, s.installer_checkbox);
            ui.weak(s.installer_hint);
        });
        if self.show_decompiler_module && self.decompilers.is_empty() {
            ui.colored_label(self.theme.warn, s.decompiler_none_installed);
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
