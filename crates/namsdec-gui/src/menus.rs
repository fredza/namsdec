//! Les menus : la barre du haut, la barre d'outils, et la palette de
//! commandes.
//!
//! Trois chemins vers les mêmes actions, et c'est voulu : le menu pour
//! découvrir, la barre d'outils pour la main qui sait déjà, la palette
//! pour celle qui préfère écrire. Ce qu'ils déclenchent vit ailleurs —
//! ici on ne fait que peindre les chemins et nommer l'action.
//!
//! L'état (`CommandPalette`, `PaletteCommand`) reste dans `main.rs` :
//! c'est de l'état de l'application, pas du dessin.

use super::*;

impl NamsdecApp {
    /// Palette de commandes (Ctrl+Maj+P, ou le raccourci ré-armé depuis
    /// n'importe quel état puisqu'elle bascule plutôt que de seulement
    /// s'ouvrir) : recherche par sous-chaîne sur toutes les fonctions du
    /// programme, navigation clavier haut/bas, Entrée pour exécuter.
    pub(crate) fn command_palette_ui(&mut self, ctx: &egui::Context) {
        if self.command_palette.is_none() {
            return;
        }
        let s = strings(self.lang);
        let mut commands = palette_commands(s, self.show_re_tab, self.exercises, self.niveau());
        // Les étiquettes du programme, à la suite : sur deux cents
        // lignes, « où est donc `affiche_nombre` ? » se répond en tapant
        // son nom, comme on ouvre un fichier par son nom dans un éditeur.
        // Rien à retenir de plus : c'est la même palette.
        commands.extend(source_labels(&self.source).into_iter().map(|(nom, ligne)| {
            (
                format!("{} {nom} ({} {ligne})", s.palette_goto_label, s.line_word),
                PaletteCommand::GoLine(ligne),
            )
        }));

        // Le raccourci de chaque commande, lu dans le jeu ACTIF.
        //
        // La palette est l'endroit où l'on cherche une action dont on ne
        // connaît pas la frappe : c'est donc l'endroit où l'apprendre.
        // Sans cela elle reste le chemin le plus court pour toujours, et
        // les trente-deux raccourcis du programme n'existent que dans le
        // tableau de Préférences, qu'on n'ouvre pas en travaillant.
        //
        // Lu en direct, et non figé : un raccourci déplacé à l'instant
        // s'affiche déplacé.
        let commands: Vec<(String, PaletteCommand, Option<String>)> = commands
            .into_iter()
            .map(|(label, cmd)| {
                let touches = match cmd {
                    PaletteCommand::Do(action) => self
                        .keybindings
                        .map
                        .get(&action)
                        .map(|chord| chord.display(ctx, self.lang == Lang::Fr)),
                    // Les autres n'en ont pas : ce ne sont pas des
                    // actions du tableau, et inventer une colonne vide
                    // ferait croire à un raccourci oublié.
                    _ => None,
                };
                (label, cmd, touches)
            })
            .collect();

        let (mut query, mut selected, measured_row, measured_head) = {
            let p = self.command_palette.as_ref().unwrap();
            (p.query.clone(), p.selected, p.row, p.tete)
        };

        let query_lower = query.to_lowercase();
        let filtered: Vec<&(String, PaletteCommand, Option<String>)> = commands
            .iter()
            .filter(|(label, _, _)| {
                query_lower.is_empty() || label.to_lowercase().contains(&query_lower)
            })
            .collect();
        if filtered.is_empty() {
            selected = 0;
        } else {
            selected = selected.min(filtered.len() - 1);
        }

        let mut close = false;
        let mut execute: Option<PaletteCommand> = None;

        // Ce que la palette peut occuper en hauteur : tout ce qui reste
        // sous son ancrage, moins une marge en bas. Une hauteur figée
        // laissait la liste défiler dans une fenêtre à moitié vide, sur
        // un écran qui l'aurait affichée en entier.
        const TOP: f32 = 48.0;
        const BOTTOM_MARGIN: f32 = 24.0;
        let screen = ctx.input(|i| i.content_rect()).height();
        let room = (screen - TOP - BOTTOM_MARGIN).max(180.0);
        // La place du champ de saisie, du séparateur et des marges de
        // la fenêtre, au-dessus de la liste — MESURÉE à la frame
        // précédente. La constante n'est que la première estimation,
        // le temps d'une image : écrite à la main, elle se trompait
        // dès qu'egui changeait la hauteur d'un champ.
        const HEAD_ESTIMÉ: f32 = 64.0;
        let head = measured_head.unwrap_or(HEAD_ESTIMÉ);
        // Avant la première mesure, une estimation : une ligne par
        // commande, à la hauteur d'un élément cliquable.
        let style = ctx.style_of(ctx.theme());
        let row =
            measured_row.unwrap_or(style.spacing.interact_size.y + style.spacing.item_spacing.y);
        let shown = filtered.len().max(1) as f32;
        let list = (shown * row).min(room - head);

        // Ce que la liste a vraiment occupé, relevé dans la fenêtre et
        // comparé à sa hauteur totale : la différence est la tête.
        let mut hauteur_de_la_liste: Option<f32> = None;
        let fenêtre = egui::Window::new("command_palette")
            .id(egui::Id::unique("command_palette"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, TOP))
            // Largeur figée, hauteur libre : la fenêtre descend jusqu'à
            // la dernière commande et s'arrête là.
            .fixed_size(egui::vec2(520.0, list + head))
            .show(ctx, |ui| {
                let text_response = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .hint_text(s.palette_placeholder)
                        .desired_width(f32::INFINITY),
                );
                if !text_response.has_focus() {
                    text_response.request_focus();
                }
                ui.separator();
                let area = egui::ScrollArea::vertical()
                    .max_height(list)
                    // `false` en largeur : la liste s'étale sur toute la
                    // palette, sinon elle se serre à gauche et sa barre
                    // de défilement — quand il en faut une — tombe au
                    // milieu de la fenêtre.
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        // Des lignes un peu plus serrées que la normale :
                        // une palette est une liste qu'on parcourt des
                        // yeux, et chaque pixel gagné par ligne, c'est
                        // une commande de plus qui reste à l'écran au
                        // lieu de passer sous une barre de défilement.
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.spacing_mut().button_padding.y = 3.0;
                        if filtered.is_empty() {
                            ui.weak(s.palette_no_match);
                        }
                        for (i, (label, cmd, touches)) in filtered.iter().enumerate() {
                            // Pleine largeur : la ligne entière se
                            // surligne et se clique, pas seulement son
                            // texte.
                            let mut bouton =
                                egui::Button::selectable(i == selected, label.as_str())
                                    .min_size(egui::vec2(ui.available_width(), 0.0))
                                    // Le nom d'une étiquette peut être
                                    // long, et la palette a une largeur
                                    // fixe : on rogne le libellé plutôt
                                    // que de pousser le raccourci hors
                                    // du cadre.
                                    .truncate();
                            if let Some(touches) = touches {
                                // `shortcut_text` pose la frappe à
                                // droite, en couleur atténuée, et c'est
                                // la place qu'elle occupe dans tous les
                                // menus — y compris ceux de namsdec.
                                bouton = bouton.shortcut_text(touches.as_str());
                            }
                            let item = ui.add(bouton);
                            // Les flèches déplacent la sélection ; si la
                            // liste défile, elle doit suivre.
                            if i == selected && !filtered.is_empty() {
                                item.scroll_to_me(None);
                            }
                            if item.clicked() {
                                execute = Some(*cmd);
                            }
                        }
                    });
                // Ce qu'une ligne prend vraiment, pour la frame suivante.
                if let Some(p) = &mut self.command_palette {
                    p.row = Some(area.content_size.y / shown);
                }
                // Et ce que la tête a pris : tout ce qui, dans la
                // fenêtre, n'est pas la liste. On le relit à la frame
                // suivante plutôt que de l'écrire ici.
                hauteur_de_la_liste = Some(area.inner_rect.height());
            });

        if let Some(fenêtre) = fenêtre
            && let Some(liste) = hauteur_de_la_liste
            && let Some(p) = &mut self.command_palette
        {
            p.tete = Some((fenêtre.response.rect.height() - liste).max(0.0));
        }

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                close = true;
            }
            if i.key_pressed(egui::Key::ArrowDown) && !filtered.is_empty() {
                selected = (selected + 1).min(filtered.len() - 1);
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                selected = selected.saturating_sub(1);
            }
            if i.key_pressed(egui::Key::Enter)
                && let Some((_, cmd, _)) = filtered.get(selected)
            {
                execute = Some(*cmd);
            }
        });

        if close {
            self.command_palette = None;
            return;
        }
        if let Some(cmd) = execute {
            self.command_palette = None;
            self.execute_palette_command(ctx, cmd);
            return;
        }
        if let Some(p) = &mut self.command_palette {
            p.query = query;
            p.selected = selected;
        }
    }

    pub(crate) fn menu_bar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let mut bouton_fenêtre = None;
        ui.horizontal(|ui| {
            // Les pastilles en tête de la barre : la fermeture occupe
            // alors le coin de l'écran, l'angle qu'une souris lancée
            // vers le haut à gauche atteint sans viser.
            if !self.system_titlebar {
                let maximisée = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                bouton_fenêtre = chrome::window_buttons_ui(ui, s, maximisée);
                ui.add_space(4.0);
            }
            ui.menu_button(s.menu_file, |ui| {
                let ctx = ui.ctx().clone();
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ctx,
                        s.file_new,
                        Action::New,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::New);
                    ui.close();
                }
                ui.separator();
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ctx,
                        s.file_open_file,
                        Action::OpenFile,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::OpenFile);
                    ui.close();
                }
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ctx,
                        s.file_open_folder,
                        Action::OpenFolder,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::OpenFolder);
                    ui.close();
                }
                // Fermer le fichier qu'on regarde : le geste existe dans
                // la barre d'onglets, mais il se cherche d'abord ici.
                //
                // L'entrée ne dépend PLUS du nombre d'onglets ouverts.
                // `tabs` ne porte que les fichiers qu'on ne regarde pas :
                // avec un seul ouvert elle est vide, et « Fermer le
                // fichier » disparaissait du menu au moment précis où
                // l'on en avait besoin.
                // TOUJOURS PRÉSENTE, grisée s'il n'y a rien à fermer.
                //
                // Elle disparaissait quand aucun fichier n'était ouvert
                // — c'est-à-dire, pour qui ouvre le menu Fichier en se
                // demandant comment fermer, exactement au mauvais
                // moment. Une entrée qui s'efface laisse croire qu'elle
                // n'existe pas ; une entrée grise qui dit pourquoi
                // enseigne l'état du programme.
                let quelque_chose_à_fermer = self.current_file.is_some() || !self.tabs.is_empty();
                ui.add_enabled_ui(quelque_chose_à_fermer, |ui| {
                    if ui
                        .button(shortcut_hint(
                            &self.keybindings,
                            self.lang,
                            &ctx,
                            s.action_close_tab,
                            Action::CloseTab,
                        ))
                        .on_disabled_hover_text(s.why_nothing_to_close)
                        .clicked()
                    {
                        self.dispatch(Action::CloseTab);
                        ui.close();
                    }
                });
                // « Fichier suivant » demande, lui, qu'il y en ait un
                // autre : c'est une navigation, pas une fermeture.
                if !self.tabs.is_empty() {
                    for action in [Action::NextTab, Action::PreviousTab] {
                        if ui
                            .button(shortcut_hint(
                                &self.keybindings,
                                self.lang,
                                &ctx,
                                action_label(s, action),
                                action,
                            ))
                            .clicked()
                        {
                            self.dispatch(action);
                            ui.close();
                        }
                    }
                }
                // Les derniers ouverts : on retravaille le même fichier
                // trois fois dans une séance, et le sélecteur natif
                // demande à chaque fois de retrouver son dossier.
                let récents = self.recent_files.clone();
                ui.add_enabled_ui(!récents.is_empty(), |ui| {
                    ui.menu_button(s.file_recent, |ui| {
                        for chemin in &récents {
                            let nom = chemin
                                .file_name()
                                .map(|nom| nom.to_string_lossy().into_owned())
                                .unwrap_or_else(|| chemin.to_string_lossy().into_owned());
                            // Le chemin entier au survol : deux fichiers
                            // peuvent porter le même nom dans deux
                            // dossiers, et c'est même le cas courant.
                            if ui
                                .button(nom)
                                .on_hover_text(chemin.to_string_lossy())
                                .clicked()
                            {
                                self.open_file(chemin.clone());
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button(s.file_recent_clear).clicked() {
                            self.recent_files.clear();
                            ui.close();
                        }
                    });
                });
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ui.ctx().clone(),
                        s.action_save,
                        Action::Save,
                    ))
                    .clicked()
                {
                    self.do_save();
                    ui.close();
                }
                if ui.button(s.file_save_as).clicked() {
                    self.do_save_as();
                    ui.close();
                }
                // Le programme assemblé, tel que `ld` l'a produit : de
                // quoi le lancer dans un vrai terminal et le comparer à
                // ce que namsdec en dit.
                ui.add_enabled_ui(
                    self.assemble_state
                        .as_ref()
                        .is_some_and(|state| !state.output.file.is_empty()),
                    |ui| {
                        if ui
                            .button(s.file_export_binary)
                            .on_hover_text(s.file_export_binary_hint)
                            .clicked()
                        {
                            self.spawn_file_dialog(FileDialogKind::ExportBinary);
                            ui.close();
                        }
                    },
                );
                ui.separator();
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ui.ctx().clone(),
                        s.file_preferences,
                        Action::ShowPreferences,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::ShowPreferences);
                    ui.close();
                }
                ui.separator();
                if ui.button(s.file_quit).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button(s.menu_edit, |ui| {
                // CE QUI SE FAIT SUR LE TEXTE, et qui n'avait de chemin
                // qu'au clavier.
                //
                // Huit actions sur trente n'étaient accessibles que par
                // leur raccourci et par la palette : dupliquer une
                // ligne, surligner les occurrences, aller à la suivante
                // et à la précédente, les renommer toutes, nommer une
                // adresse, la commenter, revenir au fichier précédent.
                // La palette est un filet, pas un chemin — on l'ouvre
                // quand on sait déjà ce qu'on cherche. Un menu, lui, se
                // parcourt : c'est là qu'on découvre ce qu'un programme
                // sait faire, et chaque entrée porte son raccourci, ce
                // qui l'apprend au passage.
                for action in [
                    Action::Find,
                    Action::NextMatch,
                    Action::PreviousMatch,
                    Action::HighlightMatches,
                    Action::RenameMatches,
                ] {
                    if ui
                        .button(shortcut_hint(
                            &self.keybindings,
                            self.lang,
                            &ui.ctx().clone(),
                            action_label(s, action),
                            action,
                        ))
                        .clicked()
                    {
                        self.dispatch(action);
                        ui.close();
                    }
                }
                ui.separator();
                for action in [Action::DuplicateLine, Action::GoToLabel] {
                    if ui
                        .button(shortcut_hint(
                            &self.keybindings,
                            self.lang,
                            &ui.ctx().clone(),
                            action_label(s, action),
                            action,
                        ))
                        .clicked()
                    {
                        self.dispatch(action);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button(s.edit_copy_code).clicked() {
                    ui.ctx().copy_text(self.source.clone());
                    ui.close();
                }
                // Le désassemblage, lui, n'existe qu'après l'assemblage
                // — et c'est ce qu'on colle dans un compte rendu, pas le
                // source.
                // L'état du processeur, en texte : c'est ce qu'on colle
                // pour demander de l'aide. Un état qui ne sort pas de
                // l'écran se recopie à la main, donc mal.
                ui.add_enabled_ui(self.emulator.is_some(), |ui| {
                    if ui
                        .button(s.edit_copy_context)
                        .on_hover_text(s.edit_copy_context_hint)
                        .on_disabled_hover_text(s.why_no_program)
                        .clicked()
                    {
                        if let Some(contexte) = self.context_text() {
                            ui.ctx().copy_text(contexte);
                            self.status = Status::Note(|s| s.context_copied);
                        }
                        ui.close();
                    }
                });
                ui.add_enabled_ui(self.assemble_state.is_some(), |ui| {
                    if ui
                        .button(s.edit_copy_listing)
                        .on_disabled_hover_text(s.why_no_program)
                        .clicked()
                    {
                        if let Some(listing) = self.listing_text() {
                            ui.ctx().copy_text(listing);
                            self.status = Status::Note(|s| s.listing_copied);
                        }
                        ui.close();
                    }
                });
                ui.separator();
                // « Coller » colle à l'endroit du curseur, comme partout
                // ailleurs. Cette entrée remplaçait TOUT le programme,
                // sans le dire et sans retour possible : on cherchait à
                // recopier trois lignes d'un cours et l'on perdait son
                // travail.
                ui.add_enabled_ui(!self.buffer_came_from_a_binary(), |ui| {
                    if ui
                        .button(s.edit_paste)
                        .on_hover_text(s.edit_paste_hint)
                        .clicked()
                    {
                        self.paste_at_cursor();
                        ui.close();
                    }
                });
                // Remplacer tout, c'est un autre geste, et il porte
                // maintenant son nom.
                ui.add_enabled_ui(!self.buffer_came_from_a_binary(), |ui| {
                    if ui
                        .button(s.edit_replace_all)
                        .on_hover_text(s.edit_replace_all_hint)
                        .clicked()
                    {
                        self.replace_all_from_clipboard();
                        ui.close();
                    }
                });
            });

            ui.menu_button(s.menu_tools, |ui| {
                if ui.button(s.action_assemble).clicked() {
                    self.dispatch(Action::Assemble);
                    ui.close();
                }
                let can_exec = self.can_execute();
                // La même explication que dans la barre d'outils : une
                // entrée de menu grisée qui ne dit rien laisse chercher.
                let pourquoi_pas = if self.emulator.is_none() {
                    s.why_no_program
                } else {
                    s.why_program_finished
                };
                ui.add_enabled_ui(can_exec, |ui| {
                    if ui
                        .button(action_label_pour(s, Action::StepInto, self.niveau()))
                        .on_disabled_hover_text(pourquoi_pas)
                        .clicked()
                    {
                        self.dispatch(Action::StepInto);
                        ui.close();
                    }
                    if self.montre(Action::StepOver)
                        && ui
                            .button(s.action_step_over)
                            .on_disabled_hover_text(pourquoi_pas)
                            .clicked()
                    {
                        self.dispatch(Action::StepOver);
                        ui.close();
                    }
                    if ui
                        .button(s.action_run)
                        .on_disabled_hover_text(pourquoi_pas)
                        .clicked()
                    {
                        self.dispatch(Action::Run);
                        ui.close();
                    }
                    if ui
                        .button(shortcut_hint(
                            &self.keybindings,
                            self.lang,
                            &ui.ctx().clone(),
                            s.action_run_to_cursor,
                            Action::RunToCursor,
                        ))
                        .on_disabled_hover_text(pourquoi_pas)
                        .clicked()
                    {
                        self.dispatch(Action::RunToCursor);
                        ui.close();
                    }
                    ui.add_enabled_ui(self.running || self.assembling(), |ui| {
                        if ui
                            .button(s.action_stop)
                            .on_disabled_hover_text(s.why_nothing_running)
                            .clicked()
                        {
                            self.dispatch(Action::Stop);
                            ui.close();
                        }
                    });
                    // Explorer un programme qu'on ne connaît pas : ce
                    // qu'il appelle, quand il en sort, ce qu'il demande
                    // au noyau. Un pas à pas de dix mille pas ne répond
                    // pas à ces questions-là.
                    //
                    // LES TROIS ENSEMBLE OU AUCUNE : ce sont les
                    // `nextcall`, `nextret` et `nextsyscall` de
                    // pwndbg, et elles supposent qu'on sache déjà ce
                    // qu'est un appel, un retour, un appel système.
                    // Leur séparateur part avec elles — un trait qui
                    // ne sépare plus rien est du bruit.
                    if self.montre(Action::RunToCall) {
                        ui.separator();
                    }
                    for (action, label) in [
                        (Action::RunToCall, s.action_run_to_call),
                        (Action::RunToReturn, s.action_run_to_return),
                        (Action::RunToSyscall, s.action_run_to_syscall),
                    ] {
                        if self.montre(action)
                            && ui
                                .button(shortcut_hint(
                                    &self.keybindings,
                                    self.lang,
                                    &ui.ctx().clone(),
                                    label,
                                    action,
                                ))
                                .on_disabled_hover_text(pourquoi_pas)
                                .clicked()
                        {
                            self.dispatch(action);
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button(s.action_reset).clicked() {
                        self.dispatch(Action::Reset);
                        ui.close();
                    }
                });
                ui.separator();
                // L'analyse du binaire dans la zone centrale : le
                // panneau latéral est trop étroit et trop court pour une
                // liste de protections, de segments et de chaînes — et
                // l'onglet RE est masqué par défaut, donc invisible pour
                // qui ne sait pas qu'il existe.
                //
                // LES HUIT ENTRÉES QUI SUIVENT CHOISISSENT UN ONGLET de
                // la rangée des vues, et ne font rien de plus quand la
                // vue est détachée : sa fenêtre est déjà sous les yeux,
                // et la rattacher d'autorité défairait le côte à côte
                // qu'on venait d'installer.
                if self.montre_vue(VueCentrale::Analysis) && ui.button(s.analysis_open).clicked() {
                    self.montrer_vue(VueCentrale::Analysis);
                    ui.close();
                }
                // Patcher : le geste qui manquait. On ouvre un crackme,
                // on voit le `je` qu'il faudrait inverser, et jusqu'ici
                // l'on ne pouvait rien en faire. Grisé sans binaire
                // ouvert — un programme qu'on vient d'assembler se
                // change en changeant son source, ce qui est plus simple
                // et plus sûr.
                let ouvert = self
                    .assemble_state
                    .as_ref()
                    .is_some_and(|état| état.from_binary.is_some());
                ui.add_enabled_ui(ouvert, |ui| {
                    // Nommer et commenter une adresse : deux gestes de
                    // rétro-ingénierie qui n'avaient de chemin qu'au
                    // clavier. Ils sont ici, avec le patcheur, parce
                    // qu'ils portent sur le binaire qu'on lit.
                    for action in [Action::NameAddress, Action::CommentAddress] {
                        if ui
                            .button(shortcut_hint(
                                &self.keybindings,
                                self.lang,
                                &ui.ctx().clone(),
                                action_label(s, action),
                                action,
                            ))
                            .clicked()
                        {
                            self.dispatch(action);
                            ui.close();
                        }
                    }
                    if self.montre_vue(VueCentrale::Patches)
                        && ui
                            .button(s.patch_open)
                            .on_hover_text(s.patch_hint)
                            .clicked()
                    {
                        self.montrer_vue(VueCentrale::Patches);
                        ui.close();
                    }
                });
                // Le déroulement du programme : là où l'on comprend sa
                // forme, et non sa suite d'instructions.
                if self.montre_vue(VueCentrale::Flow)
                    && ui
                        .button(shortcut_hint(
                            &self.keybindings,
                            self.lang,
                            &ui.ctx().clone(),
                            s.flow_open,
                            Action::ShowFlow,
                        ))
                        .clicked()
                {
                    self.montrer_vue(VueCentrale::Flow);
                    ui.close();
                }
                // La décompilation : une entrée seulement si le module
                // est allumé. Un menu qui propose ce qui ne se fera pas
                // ment poliment.
                if self.show_decompiler_module
                    && self.montre_vue(VueCentrale::Decompiler)
                    && ui.button(s.decompiler_open).clicked()
                {
                    self.montrer_vue(VueCentrale::Decompiler);
                    ui.close();
                }
                if self.montre_vue(VueCentrale::Vectors) && ui.button(s.vectors_open).clicked() {
                    self.montrer_vue(VueCentrale::Vectors);
                    ui.close();
                }
                // LA SCÈNE : la machine en grand, pour regarder un pas
                // se faire — ou le montrer. Elle existe aux deux
                // niveaux ; au mode simple, c'est la frise qui l'ouvre.
                if ui
                    .button(s.scene_open)
                    .on_hover_text(s.scene_hint)
                    .clicked()
                {
                    self.show_scene = true;
                    ui.close();
                }
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ui.ctx().clone(),
                        s.calculator_open,
                        Action::ToggleCalculator,
                    ))
                    .on_hover_text(s.calculator_hint)
                    .clicked()
                {
                    self.dispatch(Action::ToggleCalculator);
                    ui.close();
                }
                if self.montre_vue(VueCentrale::Breakpoints)
                    && ui.button(s.breakpoints_open).clicked()
                {
                    self.montrer_vue(VueCentrale::Breakpoints);
                    ui.close();
                }
                if self.montre_vue(VueCentrale::Timeline) && ui.button(s.timeline_open).clicked() {
                    self.montrer_vue(VueCentrale::Timeline);
                    ui.close();
                }
                if self.montre_vue(VueCentrale::Disassembly)
                    && ui.button(s.disassembly_open).clicked()
                {
                    self.montrer_vue(VueCentrale::Disassembly);
                    ui.close();
                }
                // LE MOTIF CYCLIQUE est un outil d'exploitation : on
                // s'en sert pour retrouver, dans un débordement de
                // tampon, de combien d'octets on a dépassé. La question
                // ne se pose pas avant d'avoir appris ce qu'est une
                // pile.
                if !self.debutant
                    && ui
                        .button(s.cyclic_open)
                        .on_hover_text(s.cyclic_hint)
                        .clicked()
                {
                    self.show_cyclic = true;
                    ui.close();
                }
                ui.separator();
                // Le module des exercices se charge à la demande. Mais
                // une porte qu'on ne voit pas n'existe pas : l'entrée
                // reste visible, grisée, et dit où l'ouvrir. Sans cela,
                // un débutant ne saura jamais qu'il y a un parcours.
                if self.exercises {
                    if ui.button(s.exercises_open).clicked() {
                        self.show_exercises = true;
                        ui.close();
                    }
                } else {
                    ui.add_enabled(false, egui::Button::new(s.exercises_open))
                        .on_disabled_hover_text(s.exercises_where);
                }
            });

            ui.menu_button(s.menu_view, |ui| {
                // Toutes les fenêtres non-dialogue sont affichables/masquables
                // ici — des cases à cocher, pas des boutons, pour pouvoir en
                // activer/désactiver plusieurs sans refermer le menu à
                // chaque fois.
                //
                // DEUX DES QUATRE PORTENT UNE FRAPPE, et il fallait
                // d'abord savoir où l'écrire : `ui.checkbox` n'a pas de
                // `shortcut_text`, et la poser dans une infobulle en
                // aurait fait les seules entrées de menu dont le
                // raccourci ne se lise pas sans survoler. La réponse
                // est dans les atomes d'egui 0.36 : un `Atom::grow`
                // entre le libellé et la frappe, et celle-ci va au bord
                // droit, atténuée — la place exacte qu'elle occupe sur
                // les boutons voisins.
                //
                // Les deux autres n'en ont pas, et c'est un choix : on
                // règle la barre d'outils et la barre de statut une
                // fois. Leur donner une frappe, ce serait la retirer à
                // quelque chose qu'on fait tous les jours.
                let ctx_menu = ui.ctx().clone();
                let frappe = |app: &Self, action| {
                    app.keybindings
                        .map
                        .get(&action)
                        .map(|c: &keybindings::Chord| c.display(&ctx_menu, app.lang == Lang::Fr))
                        .unwrap_or_default()
                };
                let frappe_panneau = frappe(self, Action::ToggleSidebar);
                let frappe_console = frappe(self, Action::ToggleConsole);
                // LE MODE DÉBUTANT EN TÊTE, et c'est un défaut signalé
                // à l'écran qui l'a mis là : « je ne sais pas basculer
                // entre les modes ». Il n'avait de chemin que par
                // Fichier › Préférences › Modules — trois gestes, et
                // au seul endroit où quelqu'un qui débute n'ira pas
                // chercher.
                //
                // Sa place est ici : ce menu ne contient que des
                // bascules qui décident de ce qu'on VOIT — la barre
                // d'outils, le panneau, la barre d'état, la console —,
                // et celle-ci les gouverne toutes. En tête, parce
                // qu'elle décide de ce que les autres montrent.
                //
                // La case des Préférences reste : on y RÈGLE, on ne
                // bascule pas. Les deux touchent le même champ, donc
                // elles ne peuvent pas diverger.
                ui.checkbox(&mut self.debutant, s.beginner_toggle)
                    .on_hover_text(s.beginner_hint);
                ui.separator();
                ui.checkbox(&mut self.show_toolbar, s.view_toolbar);
                ui.checkbox(&mut self.show_rails, s.view_rails)
                    .on_hover_text(s.view_rails_hint);
                ui.add(egui::Checkbox::new(
                    &mut self.show_sidebar,
                    (
                        s.view_sidebar,
                        egui::Atom::grow(),
                        egui::RichText::new(frappe_panneau).weak(),
                    ),
                ));
                ui.checkbox(&mut self.show_statusbar, s.view_statusbar);
                ui.add(egui::Checkbox::new(
                    &mut self.show_console,
                    (
                        s.view_console,
                        egui::Atom::grow(),
                        egui::RichText::new(frappe_console).weak(),
                    ),
                ));
                // Rendre la hauteur à ce qu'elle dit : une console qu'on
                // a agrandie pour lire vingt lignes n'a plus de raison
                // d'occuper autant devant deux.
                let mut automatique = self.console_height.is_none();
                if ui
                    .add_enabled(
                        self.show_console,
                        egui::Checkbox::new(&mut automatique, s.view_console_auto),
                    )
                    .on_hover_text(s.view_console_auto_hint)
                    .changed()
                {
                    // Décochée, la case rend la console à une hauteur
                    // fixe — celle qu'elle avait avant qu'on sache la
                    // calculer.
                    self.console_height = if automatique {
                        None
                    } else {
                        Some(CONSOLE_HEIGHT)
                    };
                }
                // Et la même chose pour le panneau latéral : une poignée
                // tirée une fois se reprend, sinon le partage automatique
                // est perdu pour de bon.
                let mut partagé = self.sidebar_tab_height.is_none();
                if ui
                    .add_enabled(
                        self.show_sidebar,
                        egui::Checkbox::new(&mut partagé, s.view_sidebar_auto),
                    )
                    .on_hover_text(s.view_sidebar_auto_hint)
                    .changed()
                {
                    // Décochée, la part garde la hauteur qu'elle a
                    // maintenant : décocher ne doit rien faire sauter
                    // sous les yeux.
                    self.sidebar_tab_height = if partagé {
                        None
                    } else {
                        Some(SIDEBAR_TAB_MIN.max(self.sidebar_tab_last))
                    };
                }

                ui.separator();
                // L'explorateur n'est PAS un de ces panneaux : c'est une
                // fenêtre indépendante qu'on appelle, d'où une entrée à
                // part qui rappelle son raccourci plutôt qu'une case à
                // cocher dans le groupe ci-dessus.
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ui.ctx().clone(),
                        s.action_toggle_explorer,
                        Action::ToggleExplorer,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::ToggleExplorer);
                    ui.close();
                }

                ui.separator();
                // Son raccourci écrit à côté, comme les autres entrées
                // du menu : c'est là qu'on l'apprend, et la palette est
                // justement ce qu'on ouvre quand on ne sait pas encore.
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ui.ctx().clone(),
                        s.palette_open,
                        Action::ToggleCommandPalette,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::ToggleCommandPalette);
                    ui.close();
                }
                ui.separator();
                for tab in SidebarTab::ALL {
                    if !self.show_re_tab && tab == SidebarTab::Re {
                        continue;
                    }
                    // Le menu suit la rangée du panneau : proposer ici
                    // un onglet qui n'y paraît pas enverrait le
                    // débutant dans un endroit qu'il ne saurait plus
                    // retrouver.
                    if tab.niveau() > self.niveau() {
                        continue;
                    }
                    if ui.button(tab.label(s)).clicked() {
                        self.show_sidebar = true;
                        self.active_tab = tab;
                        ui.close();
                    }
                }
                for tab in ConsoleTab::ALL {
                    if ui.button(tab.label(s)).clicked() {
                        self.show_console = true;
                        self.console_tab = tab;
                        ui.close();
                    }
                }
            });

            ui.menu_button(s.menu_help, |ui| {
                // La fiche de l'instruction où l'on est : F1 la donne,
                // mais encore faut-il savoir que F1 fait quelque chose.
                if ui.button(s.action_instruction_help).clicked() {
                    self.dispatch(Action::InstructionHelp);
                    ui.close();
                }
                // Même raison : un raccourci qu'on ne connaît pas
                // n'existe pas.
                if ui
                    .button(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ui.ctx().clone(),
                        s.action_goto_label,
                        Action::GoToLabel,
                    ))
                    .clicked()
                {
                    self.dispatch(Action::GoToLabel);
                    ui.close();
                }
                ui.separator();
                // Le manuel sort d'ici : un fichier qu'on garde, qu'on
                // imprime, qu'on donne. Le savoir ne sert que partagé.
                if ui
                    .button(s.help_export_manual)
                    .on_hover_text(s.help_export_manual_hint)
                    .clicked()
                {
                    self.spawn_file_dialog(FileDialogKind::ExportManual);
                    ui.close();
                }
                ui.separator();
                // SIGNALER, et non « contacter » : ce que namsdec
                // rassemble est le CONTEXTE d'un défaut, pas un
                // message. La moitié du travail de qui signale est de
                // retrouver sa version, son système et ce que l'outil
                // répondait — et c'est cette moitié-là qui fait qu'une
                // anomalie n'est jamais signalée.
                if ui
                    .button(s.help_report_bug)
                    .on_hover_text(s.help_report_bug_hint)
                    .clicked()
                {
                    self.show_bug_report = true;
                    ui.close();
                }
                if ui.button(s.help_about).clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });

            // La barre de menus *est* la barre de titre : tout ce qui
            // reste après les menus porte le nom du fichier, se saisit pour
            // déplacer la fenêtre, et le double-clic l'agrandit. Une barre
            // de titre du système par-dessus ferait deux rangées pour le
            // même travail.
            //
            // Les indicateurs se posent à DROITE du nom du fichier, au bout
            // de la barre : toujours visibles et jamais repliés, mais hors
            // du chemin du titre. D'où la disposition droite-à-gauche —
            // c'est elle qui range au bout.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // En droite-à-gauche, le premier ajouté va le plus à
                // droite. Les pastilles d'abord ; `flags_ui` tient son
                // propre ordre CF…OF par une disposition gauche-à-droite
                // explicite.
                self.flags_ui(ui);
                // Puis, vers la gauche, le nom du fichier et la zone qu'on
                // saisit — elle remplit tout ce qui reste.
                if !self.system_titlebar {
                    ui.add_space(8.0);
                    let maximisée = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    // `nom_du_tampon` et non `current_file` : celui-ci
                    // reste vide pour un binaire ouvert, et la barre de
                    // titre annonçait « sans titre » au-dessus du
                    // désassemblage d'un programme chargé.
                    let fichier = self.nom_du_tampon();
                    chrome::title_drag_ui(
                        ui,
                        self.theme,
                        &fichier,
                        self.modifié_vu,
                        maximisée,
                        self.chemin_du_tampon()
                            .map(|chemin| chemin.display().to_string()),
                    );
                }
            });
        });
        if let Some(bouton) = bouton_fenêtre {
            self.window_button(bouton);
        }
    }

    pub(crate) fn toolbar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            ui.add_enabled_ui(self.can_assemble(), |ui| {
                if ui
                    .button(s.action_assemble)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ctx,
                        s.action_assemble,
                        Action::Assemble,
                    ))
                    .on_disabled_hover_text(s.binary_no_reassemble_hint)
                    .clicked()
                {
                    self.dispatch(Action::Assemble);
                }
            });

            let can_exec = self.can_execute();
            // POURQUOI la commande est grise. `on_hover_text` ne dit
            // rien sur un widget éteint — egui réserve l'infobulle
            // ordinaire aux widgets vivants —, si bien qu'on survolait
            // « Exécuter » grisé et qu'il ne se passait rien du tout.
            // Une commande éteinte qui se tait passe pour cassée ; celle
            // qui dit ce qui manque enseigne l'ordre des gestes, et
            // c'est ce que namsdec est là pour faire.
            let pourquoi_pas = if self.emulator.is_none() {
                s.why_no_program
            } else {
                s.why_program_finished
            };
            // Le pas arrière a son propre grisé : il ne dépend pas de
            // l'état d'exécution mais de l'existence d'un pas à défaire.
            let can_undo = self.emulator.as_ref().is_some_and(|e| e.can_step_back());
            ui.add_enabled_ui(can_exec, |ui| {
                if icon_button(ui, icon_run, s.action_run)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ctx,
                        s.action_run,
                        Action::Run,
                    ))
                    .on_disabled_hover_text(pourquoi_pas)
                    .clicked()
                {
                    self.dispatch(Action::Run);
                }
                // LE NOM DU PAS CHANGE AVEC LE NIVEAU, et c'est tout
                // ce qui le distingue.
                //
                // « Step Into » est le mot de gdb et de pwndbg, et
                // c'est le bon pour qui s'en sert déjà : les raccourcis
                // de namsdec sont les leurs. Mais dans une interface
                // française qui sert à APPRENDRE, trois boutons nommés
                // « Step Into », « Step Over », « Step Back » sont trois
                // murs. En mode débutant, le pas est seul — les deux
                // autres ne paraissent pas —, et il s'appelle donc par
                // ce qu'il fait.
                let nom_du_pas = action_label_pour(s, Action::StepInto, self.niveau());
                if icon_button(ui, icon_step_into, nom_du_pas)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ctx,
                        nom_du_pas,
                        Action::StepInto,
                    ))
                    .on_disabled_hover_text(pourquoi_pas)
                    .clicked()
                {
                    self.dispatch(Action::StepInto);
                }
                if self.montre(Action::StepOver)
                    && icon_button(ui, icon_step_over, s.action_step_over)
                        .on_hover_text(shortcut_hint(
                            &self.keybindings,
                            self.lang,
                            &ctx,
                            s.action_step_over,
                            Action::StepOver,
                        ))
                        .on_disabled_hover_text(pourquoi_pas)
                        .clicked()
                {
                    self.dispatch(Action::StepOver);
                }
            });
            ui.add_enabled_ui(can_undo, |ui| {
                let nom_du_retour = action_label_pour(s, Action::StepBack, self.niveau());
                if self.montre(Action::StepBack)
                    && icon_button(ui, icon_step_back, nom_du_retour)
                        .on_hover_text(shortcut_hint(
                            &self.keybindings,
                            self.lang,
                            &ctx,
                            nom_du_retour,
                            Action::StepBack,
                        ))
                        .on_disabled_hover_text(s.why_nothing_to_undo)
                        .clicked()
                {
                    self.dispatch(Action::StepBack);
                }
            });
            ui.add_enabled_ui(self.running || self.assembling(), |ui| {
                if icon_button(ui, icon_stop, s.action_stop)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ctx,
                        s.action_stop,
                        Action::Stop,
                    ))
                    .on_disabled_hover_text(s.why_nothing_running)
                    .clicked()
                {
                    self.dispatch(Action::Stop);
                }
            });

            // Réinitialiser ne se grise pas comme le reste : c'est le
            // programme terminé, tous les autres boutons éteints, qu'on
            // le cherche — précisément l'état où `can_execute` est faux.
            // Sa seule condition est qu'il y ait une machine à remettre
            // à zéro.
            ui.add_enabled_ui(self.emulator.is_some(), |ui| {
                if icon_button(ui, icon_reset, s.action_reset)
                    .on_hover_text(shortcut_hint(
                        &self.keybindings,
                        self.lang,
                        &ctx,
                        s.action_reset,
                        Action::Reset,
                    ))
                    .on_disabled_hover_text(s.why_no_program)
                    .clicked()
                {
                    self.dispatch(Action::Reset);
                }
            });

            // À droite, la calculatrice — et elle seule.
            //
            // LES ONGLETS DU PANNEAU SONT PARTIS EN TÊTE DU PANNEAU, le
            // 26 octobre 2026 : ils y sont collés à ce qu'ils
            // commandent, et la barre d'outils n'a plus à se dédoubler
            // sous mille pixels de large. Ce qui suit — la mesure de
            // leur largeur, la rangée de repli — n'a plus lieu d'être.
            //
            // L'icône, elle, RESTE ICI. Sa place se justifiait par la
            // proximité des valeurs qu'on vient convertir, et elle la
            // garde ; mais surtout elle ne doit pas dépendre du panneau :
            // panneau masqué, la calculatrice n'aurait plus de chemin
            // visible, et il faudrait la deviner dans un menu. Une seule
            // icône tient dans la barre à toutes les largeurs.
            self.bouton_calculatrice(ui);
        });
    }

    /// Cette commande paraît-elle dans la barre, au niveau en vigueur ?
    ///
    /// Une seule question, posée au même endroit pour toutes : la barre
    /// ne doit pas devenir une liste d'exceptions. Voir
    /// [`Action::niveau`] pour le critère.
    fn montre(&self, action: Action) -> bool {
        action.niveau() <= self.niveau()
    }

    /// Cette vue a-t-elle une ENTRÉE DE MENU au niveau en vigueur ?
    ///
    /// Sans l'exception de [`Self::vue_au_niveau`] — « ce qu'on regarde
    /// reste listé » —, et c'est voulu : un onglet dit où l'on est, une
    /// entrée de menu est une PORTE. Une porte qui ne paraît qu'une
    /// fois qu'on est passé ne sert à rien.
    fn montre_vue(&self, vue: VueCentrale) -> bool {
        vue.niveau() <= self.niveau()
    }

    /// La calculatrice, au bout de la barre d'outils.
    ///
    /// Sa place est là parce que c'est là qu'on s'en sert : on lit une
    /// valeur dans la pile ou dans un registre, et l'on veut savoir ce
    /// qu'elle vaut dans une autre base. Elle était dans le menu Outils,
    /// à trois gestes de l'endroit où le besoin naît.
    fn bouton_calculatrice(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_square_button(ui, chrome::icon_calculator)
                .on_hover_text(shortcut_hint(
                    &self.keybindings,
                    self.lang,
                    &ui.ctx().clone(),
                    s.calculator_title,
                    Action::ToggleCalculator,
                ))
                .clicked()
            {
                self.dispatch(Action::ToggleCalculator);
            }
        });
    }
}

/// `show_re_tab` filtre la seule commande qui mène à une vue
/// désactivable : proposer « Affichage : RE » quand l'onglet est masqué
/// revenait à offrir un raccourci qui le rallume — et le rallumage
/// était persisté, donc définitif.
pub(crate) fn palette_commands(
    s: &Strings,
    show_re_tab: bool,
    exercises: bool,
    niveau: namsdec_re::Niveau,
) -> Vec<(String, PaletteCommand)> {
    let mut commands = vec![
        (
            s.action_assemble.to_string(),
            PaletteCommand::Do(Action::Assemble),
        ),
        (s.action_run.to_string(), PaletteCommand::Do(Action::Run)),
        // LA PALETTE GARDE TOUT, et ne nomme pas autrement que le
        // reste. C'est une recherche, pas une vitrine : y cacher une
        // commande la rendrait introuvable à qui la connaît, alors
        // qu'une barre allégée ne fait que ne pas la proposer. Mais
        // elle doit employer le mot que l'interface emploie, sans quoi
        // un débutant qui cherche « pas » ne trouverait rien.
        (
            action_label_pour(s, Action::StepInto, niveau).to_string(),
            PaletteCommand::Do(Action::StepInto),
        ),
        (
            action_label_pour(s, Action::StepBack, niveau).to_string(),
            PaletteCommand::Do(Action::StepBack),
        ),
        (
            s.action_step_over.to_string(),
            PaletteCommand::Do(Action::StepOver),
        ),
        (s.action_stop.to_string(), PaletteCommand::Do(Action::Stop)),
        (
            s.action_reset.to_string(),
            PaletteCommand::Do(Action::Reset),
        ),
        (
            s.action_toggle_explorer.to_string(),
            PaletteCommand::Do(Action::ToggleExplorer),
        ),
        (s.action_save.to_string(), PaletteCommand::Do(Action::Save)),
        (s.file_save_as.to_string(), PaletteCommand::SaveAs),
        (
            s.file_open_file.to_string(),
            PaletteCommand::Do(Action::OpenFile),
        ),
        (
            s.file_open_folder.to_string(),
            PaletteCommand::Do(Action::OpenFolder),
        ),
        (
            s.file_preferences.to_string(),
            PaletteCommand::Do(Action::ShowPreferences),
        ),
        (
            s.action_instruction_help.to_string(),
            PaletteCommand::Do(Action::InstructionHelp),
        ),
        (
            s.action_goto_label.to_string(),
            PaletteCommand::Do(Action::GoToLabel),
        ),
        (s.action_find.to_string(), PaletteCommand::Do(Action::Find)),
        // Les gestes d'exécution que les menus portent aussi : la
        // palette promet « toutes les commandes », et une commande
        // qu'elle ne propose pas n'existe que pour qui connaît déjà son
        // raccourci.
        (
            s.action_step_back.to_string(),
            PaletteCommand::Do(Action::StepBack),
        ),
        (
            s.action_run_to_cursor.to_string(),
            PaletteCommand::Do(Action::RunToCursor),
        ),
        (
            s.action_run_to_call.to_string(),
            PaletteCommand::Do(Action::RunToCall),
        ),
        (
            s.action_run_to_return.to_string(),
            PaletteCommand::Do(Action::RunToReturn),
        ),
        (
            s.action_run_to_syscall.to_string(),
            PaletteCommand::Do(Action::RunToSyscall),
        ),
        (s.file_new.to_string(), PaletteCommand::Do(Action::New)),
        (s.help_about.to_string(), PaletteCommand::ShowAbout),
        (s.analysis_open.to_string(), PaletteCommand::ShowAnalysis),
        (s.vectors_open.to_string(), PaletteCommand::ShowVectors),
        (
            s.breakpoints_open.to_string(),
            PaletteCommand::ShowBreakpoints,
        ),
        (s.timeline_open.to_string(), PaletteCommand::ShowTimeline),
        (
            s.flow_open.to_string(),
            PaletteCommand::Do(Action::ShowFlow),
        ),
        (
            s.disassembly_open.to_string(),
            PaletteCommand::ShowDisassembly,
        ),
        (s.cyclic_open.to_string(), PaletteCommand::ShowCyclic),
        (s.scene_open.to_string(), PaletteCommand::ShowScene),
        (
            format!("{}: Français", s.preferences_language),
            PaletteCommand::SetLang(Lang::Fr),
        ),
        (
            format!("{}: English", s.preferences_language),
            PaletteCommand::SetLang(Lang::En),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_stack),
            PaletteCommand::GoTab(SidebarTab::Stack),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_memory),
            PaletteCommand::GoTab(SidebarTab::Memory),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_heap),
            PaletteCommand::GoTab(SidebarTab::Heap),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_bytes),
            PaletteCommand::GoTab(SidebarTab::Bytes),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_output),
            PaletteCommand::GoConsoleTab(ConsoleTab::Output),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_diagnostics),
            PaletteCommand::GoConsoleTab(ConsoleTab::Diagnostics),
        ),
        (
            format!("{}: {}", s.menu_view, s.tab_profile),
            PaletteCommand::GoConsoleTab(ConsoleTab::Profile),
        ),
        // Les fichiers ouverts : d'un onglet au suivant, et la fermeture
        // de celui qu'on regarde.
        (
            s.action_next_tab.to_string(),
            PaletteCommand::Do(Action::NextTab),
        ),
        (
            s.action_previous_tab.to_string(),
            PaletteCommand::Do(Action::PreviousTab),
        ),
        (
            s.action_close_tab.to_string(),
            PaletteCommand::Do(Action::CloseTab),
        ),
        (
            s.action_duplicate_line.to_string(),
            PaletteCommand::Do(Action::DuplicateLine),
        ),
        (
            s.action_highlight_matches.to_string(),
            PaletteCommand::Do(Action::HighlightMatches),
        ),
        (
            s.action_next_match.to_string(),
            PaletteCommand::Do(Action::NextMatch),
        ),
        (
            s.action_previous_match.to_string(),
            PaletteCommand::Do(Action::PreviousMatch),
        ),
        (
            s.action_rename_matches.to_string(),
            PaletteCommand::Do(Action::RenameMatches),
        ),
        (
            s.action_name_address.to_string(),
            PaletteCommand::Do(Action::NameAddress),
        ),
        (
            s.action_comment_address.to_string(),
            PaletteCommand::Do(Action::CommentAddress),
        ),
        (
            s.palette_toggle_toolbar.to_string(),
            PaletteCommand::ToggleToolbar,
        ),
        (
            s.palette_toggle_sidebar.to_string(),
            PaletteCommand::Do(Action::ToggleSidebar),
        ),
        (
            s.palette_toggle_statusbar.to_string(),
            PaletteCommand::ToggleStatusbar,
        ),
        (
            // « Afficher/masquer la console », comme ses trois sœurs :
            // elle disait « Affichage: Console », seule de son groupe.
            s.palette_toggle_console.to_string(),
            PaletteCommand::Do(Action::ToggleConsole),
        ),
        (s.file_quit.to_string(), PaletteCommand::Quit),
    ];

    // Le module des exercices n'entre dans la palette que s'il est
    // chargé : proposer une commande qui n'ouvre rien n'aide personne.
    if exercises {
        commands.push((s.exercises_open.to_string(), PaletteCommand::ShowExercises));
    }
    // Sans condition : elle ne dépend d'aucun module, d'aucun binaire
    // chargé, et sert autant à qui écrit son premier programme qu'à qui
    // démonte un crackme.
    commands.push((
        s.calculator_open.to_string(),
        PaletteCommand::Do(Action::ToggleCalculator),
    ));
    // Elle-même, pour la forme : la palette promet TOUTES les
    // commandes, et une action absente de la liste n'existe que pour qui
    // connaît déjà sa frappe. L'y choisir la referme — voir
    // `execute_palette_command`.
    commands.push((
        s.palette_action.to_string(),
        PaletteCommand::Do(Action::ToggleCommandPalette),
    ));

    if show_re_tab {
        commands.push((
            format!("{}: {}", s.menu_view, s.tab_re),
            PaletteCommand::GoTab(SidebarTab::Re),
        ));
    }

    // Un thème installé plus tard apparaît ici sans rien changer d'autre.
    commands.extend(namsdec_theme::all().into_iter().map(|theme| {
        (
            format!("{}: {}", s.preferences_theme, theme.name),
            PaletteCommand::SetTheme(theme),
        )
    }));

    commands
}
