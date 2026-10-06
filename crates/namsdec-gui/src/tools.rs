//! Les outils extérieurs : les désassembleurs, les décompilateurs, et
//! la recette qui va les chercher.
//!
//! Tout ce qui **lance un programme qui n'est pas namsdec** est ici, et
//! c'est voulu : ce sont les seuls endroits où l'application sort de son
//! bac à sable, et les regrouper permet de vérifier d'un coup d'œil
//! qu'aucun ne le fait sans qu'on l'ait demandé.
//!
//! Sorti de  le 29 septembre 2026.

use super::*;

impl NamsdecApp {
    /// Les outils d'analyse connus : présents ou non, et déclarables
    /// d'un clic.
    ///
    /// Écrire un fichier JSON à la main pour brancher Rizin serait une
    /// barrière absurde — les arguments qui marchent sont les mêmes pour
    /// tout le monde. Le bouton **écrit ce fichier**, et rien d'autre :
    /// il n'installe pas le programme, il ne le lance pas. Quand le
    /// programme manque, namsdec dit ce qu'il faut taper pour l'obtenir
    /// et laisse taper.
    pub(crate) fn known_tools_ui(&mut self, ui: &mut egui::Ui) {
        use namsdec_disasm::known::Availability;
        let s = strings(self.lang);
        let theme = self.theme;
        let fr = self.lang == Lang::Fr;

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(s.tools_title).strong().size(15.0));
            // Combien il y en a. Six fiches ne tiennent plus dans la
            // fenêtre quand aucun outil n'est installé — chacune écrit
            // alors une ligne de plus —, et rien ne disait que la liste
            // continuait sous le bord. C'est le même oubli que le
            // classement du profil : on ne cherche pas à dérouler ce
            // qu'on croit avoir vu en entier.
            ui.colored_label(
                theme.dim,
                format!("· {} {}", namsdec_disasm::known::all().len(), s.tools_count),
            );
        });
        ui.add_space(2.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.colored_label(theme.dim, s.tools_legend);
        ui.add_space(6.0);
        // Le relevé tient tant qu'on ne demande pas autre chose : il
        // coûte un processus par outil. Ce bouton est la porte de
        // sortie — on vient d'installer quelque chose dans un terminal
        // et l'on veut que namsdec le voie sans redémarrer.
        if framed_button(ui, theme, s.tools_recheck)
            .on_hover_text(s.tools_recheck_hint)
            .clicked()
        {
            self.refresh_tool_availability();
            self.status = Status::Note(|s| s.tools_rechecked);
        }
        ui.add_space(10.0);

        let utilisés: std::collections::BTreeSet<String> =
            self.decompilers.iter().map(|p| p.id.clone()).collect();
        let mut à_utiliser = None;
        let mut à_retirer = None;
        let mut plan = None;
        let mut allumer = false;

        // Le dossier où namsdec pose ce qu'il installe : un outil bâti
        // par le module d'installation n'est dans le PATH de personne, et
        // sans cela il restait « absent » alors qu'il tournait très bien.
        // Le relevé est fait une fois, avant la boucle : le demander
        // par outil et par image lançait `rizin` soixante fois par
        // seconde.
        if self.tools_ready.is_empty() {
            self.refresh_tool_availability();
        }
        let état = self.tools_ready.clone();
        // Le dossier où namsdec pose ce qu'il installe : un outil bâti
        // par le module d'installation n'est dans le PATH de personne.
        let outils = data_dir().map(|dir| namsdec_disasm::install::tools_dir(&dir));
        for outil in namsdec_disasm::known::all() {
            let dispo = état
                .get(&outil.plugin.id)
                .copied()
                .unwrap_or(namsdec_disasm::known::Availability::Missing);
            let sert = utilisés.contains(&outil.plugin.id);
            // Une fiche par outil. La grille d'avant mettait sur une même
            // ligne un nom, un état, une commande d'installation et deux
            // boutons : quatre colonnes qui ne s'alignaient pas et dont
            // aucune n'expliquait les autres.
            egui::Frame::new()
                .fill(theme.text.gamma_multiply(0.035))
                .stroke(egui::Stroke::new(
                    1.0,
                    if sert {
                        theme.accent.gamma_multiply(0.5)
                    } else {
                        theme.dim.gamma_multiply(0.3)
                    },
                ))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    // Le nom, les deux états, et l'action à droite, sur
                    // une seule ligne : c'est ce qu'on lit en diagonale
                    // pour savoir où en est un outil.
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&outil.plugin.name).strong());
                        let (couleur, état) = match dispo {
                            Availability::Ready => (theme.good, s.tools_state_found),
                            Availability::Incomplete => (theme.warn, s.tools_state_partial),
                            Availability::Missing => (theme.dim, s.tools_state_absent),
                        };
                        pastille(ui, couleur, état);
                        pastille(
                            ui,
                            if sert { theme.accent } else { theme.dim },
                            if sert { s.tools_in_use } else { s.tools_unused },
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if sert {
                                if framed_button(ui, theme, s.tools_stop).clicked() {
                                    à_retirer = Some(outil.plugin.id.clone());
                                }
                            } else if ui
                                .add_enabled(
                                    dispo != Availability::Missing,
                                    egui::Button::new(
                                        egui::RichText::new(s.tools_use).color(theme.bg),
                                    )
                                    .fill(theme.accent)
                                    .corner_radius(egui::CornerRadius::from(6)),
                                )
                                .clicked()
                            {
                                à_utiliser = Some(outil.resolved_plugin(outils.as_deref()));
                            }
                        });
                    });
                    ui.add_space(4.0);

                    // À quoi il sert, en clair et non plus au survol.
                    ui.colored_label(theme.dim, if fr { outil.about_fr } else { outil.about_en });

                    // Ce qui empêche de s'en servir se lit sans rien
                    // déplier : c'est une panne, pas un détail.
                    if sert && dispo == Availability::Missing {
                        ui.add_space(4.0);
                        ui.colored_label(theme.bad, s.tools_declared_but_missing);
                    }
                    // Le prérequis ne se dit que s'il manque. Affiché
                    // toujours, il se lisait comme une panne : « Il lui
                    // manque le greffon rz-ghidra » juste à côté de
                    // « programme détecté », sur une machine où le
                    // greffon était installé et fonctionnait.
                    if dispo == Availability::Incomplete
                        && let Some(besoin) = if fr { outil.needs_fr } else { outil.needs_en }
                    {
                        ui.add_space(4.0);
                        ui.colored_label(theme.warn, format!("{} {besoin}", s.tools_needs));
                    }

                    // Comment l'obtenir, et SEULEMENT quand il est absent :
                    // installer la base. Pour un outil INCOMPLET, la base
                    // est déjà là — afficher « Pour l'obtenir : sudo dnf
                    // install rizin » à côté de « installé, mais incomplet »
                    // se contredisait, et désignait le mauvais paquet : ce
                    // qui manque (rz-ghidra) n'est pas un paquet dnf, c'est
                    // pourquoi le module d'installation le construit. Pour
                    // ce cas-là, la ligne « il lui manque… » juste au-dessus
                    // et le bouton d'installation juste en dessous suffisent.
                    //
                    // Se lit sans déplier : c'est une panne, pas un détail.
                    if dispo == Availability::Missing
                        && let Some(commande) = &outil.plugin.install
                    {
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(s.tools_get_it).small(),
                            );
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(
                                        namsdec_disasm::install::install_command_here(commande),
                                    )
                                    .monospace()
                                    .small()
                                    .color(theme.warn),
                                )
                                .selectable(true),
                            );
                        });
                    }

                    // Le reste se déplie : la commande exacte, et comment
                    // obtenir ce qui manque. Cinq outils tout déplié, et
                    // l'on ne voyait déjà plus le troisième.
                    ui.add_space(2.0);
                    egui::CollapsingHeader::new(
                        egui::RichText::new(s.tools_details)
                            .small()
                            .color(theme.dim),
                    )
                    .id_salt(format!("outil_{}", outil.plugin.id))
                    // Fermé, toujours. Il ne reste dedans que la commande
                    // exacte qui sera lancée — ce qu'on va chercher quand
                    // on se demande « et sans namsdec, comment je fais ? »,
                    // jamais ce qu'on lit en arrivant sur la page.
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(s.tools_command).small(),
                            );
                            ui.add(
                                egui::Label::new(
                                    // Le programme **résolu**, et non son
                                    // nom nu : quand namsdec l'a installé
                                    // lui-même, c'est son chemin à lui
                                    // qui sera lancé, et annoncer
                                    // « rizin » désignerait celui du
                                    // système — un autre programme.
                                    egui::RichText::new({
                                        let résolu = outil.resolved_plugin(outils.as_deref());
                                        format!("{} {}", résolu.command, résolu.args.join(" "))
                                    })
                                    .monospace()
                                    .small()
                                    .color(theme.text),
                                )
                                .selectable(true),
                            );
                        });
                    });

                    // Aller le chercher reste une action, pas un détail :
                    // le bouton ne se replie pas.
                    if dispo != Availability::Ready && recette_existe(&outil.plugin.id) {
                        ui.add_space(4.0);
                        if self.show_installer_module {
                            if framed_button(ui, theme, s.install_button).clicked() {
                                plan = Some(outil.plugin.id.clone());
                            }
                        } else {
                            // Le bouton manquait sans un mot : on croyait
                            // que namsdec ne savait pas installer cet
                            // outil, alors qu'il attendait une case
                            // cochée ailleurs.
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(
                                    theme.dim,
                                    egui::RichText::new(s.tools_installer_off).small(),
                                );
                                if framed_button(ui, theme, s.tools_turn_on).clicked() {
                                    allumer = true;
                                }
                            });
                        }
                    }
                });
            ui.add_space(6.0);
        }

        if let Some(plugin) = à_utiliser {
            self.declare_tool(&plugin);
        }
        if let Some(id) = à_retirer {
            self.undeclare_tool(&id);
        }
        if let Some(id) = plan {
            self.install_plan = Some(id);
        }
        if allumer {
            // Installer suppose de pouvoir s'en servir : les deux
            // modules s'allument ensemble.
            self.show_decompiler_module = true;
            self.show_installer_module = true;
        }
    }

    /// La fenêtre qui montre une recette, avant que rien ne soit lancé.
    ///
    /// C'est le cœur de ce module : ce qu'on va exécuter est écrit là,
    /// commande par commande, avec d'où cela vient, ce que cela coûte et
    /// où cela sera posé. Le bouton n'est qu'en dessous.
    pub(crate) fn installer_window_ui(&mut self, ctx: &egui::Context) {
        let Some(outil) = self.install_plan.clone() else {
            return;
        };
        if !self.show_installer_module {
            self.install_plan = None;
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let fr = self.lang == Lang::Fr;
        let Some(data) = data_dir() else {
            self.install_plan = None;
            return;
        };
        let Some(recette) = namsdec_disasm::install::recipe_for(&outil, &data) else {
            self.plugin_note = Some(PluginNote {
                ok: false,
                text: s.install_no_recipe.to_owned(),
            });
            self.install_plan = None;
            return;
        };
        let manquants = namsdec_disasm::install::missing_requirements(&recette);

        let mut ouverte = true;
        let mut lancer = false;
        let mut arrêter = false;
        let mut fermer = false;
        // L'outil est-il déjà là ? Demandé à la machine, non déduit du
        // fait qu'un travail s'est terminé : une installation peut
        // aboutir sans que l'outil marche, et l'inverse aussi — celui du
        // système peut l'avoir précédée.
        let état = self.tool_availability(&outil);
        let déjà_prêt = état == namsdec_disasm::known::Availability::Ready;
        // « Il est là, mais il lui manque une pièce » — rizin sans
        // rz-ghidra. C'est le seul cas où relancer la recette sert à
        // quelque chose, et c'est donc le seul où on le propose.
        let à_réparer = état == namsdec_disasm::known::Availability::Incomplete;
        let en_cours = self
            .install_job
            .as_ref()
            .is_some_and(|job| job.tool == outil);
        let fait = self.install_job.as_ref().map(|job| job.done).unwrap_or(0);

        egui::Window::new(format!("{} — {}", s.install_title, recette.tool))
            .id(egui::Id::new("installer_window"))
            .open(&mut ouverte)
            // Au premier plan : elle s'ouvre depuis les préférences, qui
            // occupent le centre de l'écran. Rangée comme une fenêtre
            // ordinaire, elle se posait derrière elles — on cliquait
            // « Installer… » et il ne se passait rien de visible.
            .order(egui::Order::Foreground)
            .resizable(true)
            .collapsible(false)
            .default_size([700.0, 620.0])
            // Bornée à l'écran, et c'est ce qui manquait : le plan de
            // rizin compte neuf étapes et un journal, si bien que la
            // fenêtre dépassait en haut comme en bas. Sa barre de titre
            // passait hors champ — avec la croix qui s'y trouve —, et
            // l'on ne pouvait plus la fermer du tout.
            .max_size(egui::vec2(
                ctx.viewport_rect().width() * 0.9,
                ctx.viewport_rect().height() * 0.86,
            ))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(
                    ui,
                    self.theme,
                    s,
                    format!("{} — {}", s.install_title, recette.tool).as_str(),
                ) {
                    fermer = true;
                }
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                // La zone qui défile s'arrête avant le bas : sans cette
                // borne, elle prend toute la hauteur offerte et pousse
                // la barre d'action — l'avertissement et le bouton —
                // hors de l'écran.
                egui::ScrollArea::vertical()
                    .id_salt("installer_page")
                    .auto_shrink([false, true])
                    // Moins que la moitié : au-dessus, l'avertissement
                    // et les boutons se retrouvaient sous le bord de
                    // l'écran, journal déplié.
                    .max_height(ctx.viewport_rect().height() * 0.5)
                    .show(ui, |ui| {
                        // --- Ce qu'on va gagner ----------------------
                        // D'abord la leçon, ensuite la mécanique : une
                        // fenêtre qui ne dit que « voulez-vous
                        // installer ? » demande un acte de foi.
                        encadré(ui, theme.accent, |ui| {
                            ui.label(
                                egui::RichText::new(s.install_what_for)
                                    .strong()
                                    .color(theme.accent),
                            );
                            ui.add_space(3.0);
                            ui.label(if fr {
                                recette.teaches_fr
                            } else {
                                recette.teaches_en
                            });
                        });
                        ui.add_space(8.0);

                        // --- Les faits, en quatre lignes lisibles ----
                        //
                        // Pas de Grid ici : une cellule de grille laisse
                        // le texte s'étendre à l'infini vers la droite,
                        // et une phrase longue — le coût, justement —
                        // sort de la fenêtre sans jamais revenir à la
                        // ligne. Une colonne de titres à largeur fixe,
                        // puis une valeur qui se replie dans ce qui
                        // reste : c'est ce qui se lit.
                        for (rang, (titre, valeur, mono)) in [
                            (s.install_source, recette.source.to_owned(), false),
                            (
                                s.install_cost,
                                if fr { recette.cost_fr } else { recette.cost_en }.to_owned(),
                                false,
                            ),
                            (
                                s.install_where,
                                recette.produces.display().to_string(),
                                true,
                            ),
                            (s.install_requires, recette.requires.join(", "), true),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            let fond = if rang % 2 == 0 {
                                theme.text.gamma_multiply(0.04)
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            egui::Frame::new()
                                .fill(fond)
                                .inner_margin(egui::Margin::symmetric(6, 4))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal_top(|ui| {
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(150.0, 0.0),
                                            egui::Layout::top_down(egui::Align::RIGHT),
                                            |ui| {
                                                ui.colored_label(theme.dim, titre);
                                            },
                                        );
                                        let texte = egui::RichText::new(valeur);
                                        ui.add(
                                            egui::Label::new(if mono {
                                                small_monospace(ui, texte.text())
                                            } else {
                                                texte
                                            })
                                            .wrap()
                                            .selectable(true),
                                        );
                                    });
                                });
                        }
                        if !manquants.is_empty() {
                            ui.add_space(4.0);
                            ui.colored_label(
                                theme.bad,
                                format!("{} {}", s.install_missing, manquants.join(", ")),
                            );
                            // Le bouton reste gris tant qu'il manque
                            // quelque chose : sans la commande à poser,
                            // la fenêtre était un cul-de-sac où l'on
                            // apprenait seulement qu'on ne pouvait rien
                            // faire.
                            if let Some(commande) =
                                namsdec_disasm::install::requirement_command(&manquants)
                            {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(s.install_requirements_hint)
                                        .small()
                                        .color(theme.dim),
                                );
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&commande).monospace(),
                                        )
                                        .selectable(true),
                                    );
                                    let copié = ui
                                        .ctx()
                                        .data(|d| d.get_temp::<bool>(copie_id()))
                                        .unwrap_or(false);
                                    let étiquette = if copié {
                                        s.install_copied
                                    } else {
                                        s.install_copy
                                    };
                                    if framed_button(ui, theme, étiquette).clicked() {
                                        ui.ctx().copy_text(commande.clone());
                                        ui.ctx().data_mut(|d| d.insert_temp(copie_id(), true));
                                    }
                                });
                            }
                        }
                        ui.add_space(8.0);

                        // --- Le plan, étape par étape ----------------
                        ui.label(egui::RichText::new(s.install_plan).strong());
                        ui.add_space(4.0);
                        for (rang, étape) in recette.steps.iter().enumerate() {
                            // Chaque étape porte son état : faite, en
                            // cours, à venir. C'est ce qui transforme
                            // une liste en compte rendu.
                            let (marque, couleur) = if !en_cours {
                                ("•", theme.dim)
                            } else if rang < fait {
                                ("v", theme.good)
                            } else if rang == fait {
                                (">", theme.accent)
                            } else {
                                ("•", theme.dim)
                            };
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    couleur,
                                    egui::RichText::new(marque).monospace().strong(),
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}. {}",
                                            rang + 1,
                                            if fr {
                                                &étape.about_fr
                                            } else {
                                                &étape.about_en
                                            }
                                        ))
                                        .color(
                                            if rang == fait && en_cours {
                                                theme.text
                                            } else {
                                                theme.dim
                                            },
                                        ),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(étape.line())
                                                .monospace()
                                                .small()
                                                .color(theme.dim),
                                        )
                                        .selectable(true),
                                    );
                                });
                            });
                            ui.add_space(3.0);
                        }

                        // --- Le journal, quand ça tourne -------------
                        if let Some(job) = &self.install_job
                            && job.tool == outil
                        {
                            let lignes = job.log.lock().expect("le journal").clone();
                            if !lignes.is_empty() {
                                ui.add_space(6.0);
                                egui::CollapsingHeader::new(s.install_log)
                                    .id_salt("install_log_fold")
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        egui::Frame::new()
                                            .fill(ui.visuals().extreme_bg_color)
                                            .inner_margin(egui::Margin::same(6))
                                            .corner_radius(4.0)
                                            .show(ui, |ui| {
                                                egui::ScrollArea::vertical()
                                                    .id_salt("install_log")
                                                    .max_height(180.0)
                                                    .auto_shrink([false, false])
                                                    .stick_to_bottom(true)
                                                    .show(ui, |ui| {
                                                        for ligne in &lignes {
                                                            let couleur = if ligne.starts_with('$')
                                                            {
                                                                theme.accent
                                                            } else {
                                                                theme.dim
                                                            };
                                                            ui.colored_label(
                                                                couleur,
                                                                egui::RichText::new(ligne)
                                                                    .monospace()
                                                                    .small(),
                                                            );
                                                        }
                                                    });
                                            });
                                    });
                            }
                        }
                    });

                // --- Ce que les commandes ont écrit ------------------
                //
                // Replié par défaut : on ne l'ouvre que lorsqu'on veut
                // savoir. Mais il est là, et c'est tout ce qui permet de
                // comprendre un échec — « l'étape 2 a échoué (code 128) »
                // ne dit pas que git a trouvé le dossier déjà occupé.
                let journal: Vec<String> = if en_cours {
                    self.install_job
                        .as_ref()
                        .map(|job| job.log.lock().expect("le journal").clone())
                        .unwrap_or_default()
                } else {
                    self.install_log.clone()
                };
                if !journal.is_empty() {
                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(s.install_log_title)
                        .id_salt("install_log")
                        .default_open(!en_cours && self.plugin_note.as_ref().is_some_and(|n| !n.ok))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("install_log_scroll")
                                .max_height(160.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    for ligne in &journal {
                                        ui.add(
                                            egui::Label::new(small_monospace(ui, ligne))
                                                .selectable(true),
                                        );
                                    }
                                });
                            if framed_button(ui, theme, s.install_copy_log).clicked() {
                                ui.ctx().copy_text(journal.join("\n"));
                            }
                        });
                }

                // --- Ce qu'on fait, tout en bas et toujours visible --
                //
                // L'avertissement voisine le bouton : plus haut, il
                // serait sorti de l'écran au moment de cliquer, et un
                // avertissement qu'on ne lit pas n'en est pas un.
                ui.add_space(6.0);
                encadré(ui, theme.warn, |ui| {
                    ui.colored_label(theme.warn, s.install_warning);
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if en_cours {
                        ui.spinner();
                        ui.colored_label(theme.dim, s.install_running);
                        ui.add(
                            egui::ProgressBar::new(fait as f32 / recette.steps.len().max(1) as f32)
                                .desired_width(200.0)
                                .text(
                                    egui::RichText::new(format!(
                                        "{} / {}",
                                        fait,
                                        recette.steps.len()
                                    ))
                                    .small(),
                                ),
                        );
                        if framed_button(ui, theme, s.install_cancel).clicked() {
                            arrêter = true;
                        }
                    } else if déjà_prêt {
                        // Installé, et vérifié : le bouton principal
                        // devient « Fermer ». Il invitait à recommencer,
                        // si bien qu'on ne savait pas si l'installation
                        // avait abouti — la seule façon de s'en assurer
                        // était de tout relancer.
                        //
                        // Recommencer reste possible en second : c'est le
                        // geste de quelqu'un qui sait pourquoi il le fait.
                        ui.colored_label(theme.good, s.install_already_done);
                        // Et rien d'autre. « Recommencer » était proposé
                        // ici, et c'était une offre sans motif : l'outil
                        // marche, la recette a abouti, relancer vingt
                        // minutes de compilation ne changerait rien. Un
                        // bouton qu'on n'a aucune raison de cliquer fait
                        // douter de celui d'à côté — il laissait croire
                        // que l'installation avait peut-être échoué.
                        //
                        // Le cas où relancer sert vraiment est plus bas :
                        // l'outil incomplet, qu'on répare.
                        if dialog_bar(ui, theme, s.dialog_close, None).is_some() {
                            fermer = true;
                        }
                    } else if à_réparer {
                        // Le programme est là, il lui manque une pièce.
                        // Relancer la recette a ici un sens précis, et un
                        // nom : réparer. C'est le cas de rizin sans
                        // rz-ghidra — `pdg` n'existe pas, et rien dans la
                        // fenêtre ne disait quoi faire.
                        //
                        // « Réparer » n'est PAS le second bouton de la
                        // barre, et c'est une leçon qu'un test a rendue :
                        // Échap y répond `Cancel`, ce qui lançait une
                        // recompilation de vingt minutes au lieu de
                        // fermer. Échap veut dire « je m'en vais », jamais
                        // « fais l'autre chose ».
                        ui.colored_label(theme.warn, s.install_incomplete);
                        if framed_button(ui, theme, s.install_repair)
                            .on_hover_text(s.install_repair_hint)
                            .clicked()
                        {
                            lancer = true;
                        }
                        if dialog_bar(ui, theme, s.dialog_close, None).is_some() {
                            fermer = true;
                        }
                    } else {
                        // Rien d'installé : le principal est « Lancer »,
                        // et « Fermer » est la sortie. La croix de la
                        // barre de titre ne suffit pas — elle se retrouve
                        // loin du regard sur un plan de neuf étapes.
                        ui.add_enabled_ui(manquants.is_empty(), |ui| {
                            match dialog_bar(ui, theme, s.install_start, Some(s.dialog_close)) {
                                Some(Answer::Ok) => lancer = true,
                                Some(Answer::Cancel) => fermer = true,
                                None => {}
                            }
                        });
                        ui.colored_label(theme.dim, s.install_nothing_yet);
                    }
                });
            });

        if lancer {
            self.start_install(recette);
        }
        if arrêter && let Some(job) = &self.install_job {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        // Échap est pris par la barre d'action — elle le consomme, donc
        // il n'y a rien à guetter ici. Pendant l'installation, la barre
        // n'est pas affichée : on n'interrompt pas un `ninja` par
        // mégarde, et c'est « Arrêter » qui le fait, explicitement.
        if !ouverte || fermer {
            self.install_plan = None;
        }
    }

    /// Relève où en est l'installation, et conclut quand elle a fini.
    pub(crate) fn poll_install(&mut self, ctx: &egui::Context) {
        use std::sync::atomic::Ordering;
        let Some(job) = &mut self.install_job else {
            return;
        };
        job.done = job.progress.load(Ordering::Relaxed);
        let fini = job.outcome.lock().expect("le résultat").take();
        // Le journal, avant tout : ce qui suit jette le travail qui le
        // tient, et c'est justement quand cela tourne mal qu'on veut le
        // lire.
        if fini.is_some() {
            self.install_log = job.log.lock().expect("le journal").clone();
            // Une installation qui s'achève change la réponse des
            // outils : le relevé est jeté, et refait à la première
            // demande.
            self.tools_ready.clear();
        }
        match fini {
            Some(Ok(programme)) => {
                let outil = job.tool.clone();
                self.install_job = None;
                self.job = None;
                self.declare_installed_tool(&outil, &programme);
            }
            Some(Err(raison)) => {
                let outil = job.tool.clone();
                self.install_job = None;
                self.job = None;
                // Ce qui reste sur le disque : une recette s'arrête où
                // elle peut, et le dossier à demi rempli n'a l'air de
                // rien. La prochaine tentative l'effacera d'elle-même,
                // ce qui vaut la peine d'être dit.
                let reste = data_dir()
                    .map(|dir| namsdec_disasm::install::tools_dir(&dir).join(&outil))
                    .filter(|chemin| chemin.exists())
                    .map(|chemin| {
                        format!(
                            " {} {}",
                            strings(self.lang).install_leftover,
                            chemin.display()
                        )
                    })
                    .unwrap_or_default();
                self.plugin_note = Some(PluginNote {
                    ok: false,
                    text: format!("{} {raison}{reste}", strings(self.lang).install_failed),
                });
            }
            None => ctx.request_repaint_after(std::time::Duration::from_millis(250)),
        }
    }

    /// La fenêtre de décompilation : un outil extérieur, sur demande.
    ///
    /// Trois précautions, et elles sont le sujet de ce module autant que
    /// le pseudo-code qu'il affiche :
    ///
    /// - **Rien ne s'exécute tout seul.** Le module s'active dans les
    ///   préférences, la fenêtre s'ouvre à la demande, et l'outil ne part
    ///   qu'au clic sur « Lancer ».
    /// - **La commande est annoncée avant d'être lancée**, en toutes
    ///   lettres : c'est ce que l'on s'apprête à exécuter sur sa propre
    ///   machine, et l'on doit pouvoir le lire.
    /// - **Le travail se fait à côté.** Un décompilateur peut réfléchir
    ///   une demi-minute ; l'interface ne se fige pas, et passé le délai
    ///   l'outil est arrêté plutôt qu'attendu sans fin.
    pub(crate) fn decompiler_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_decompiler_module || !self.show_decompiler {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_decompiler;
        let mut fermer_ok = false;
        let mut lancer = false;
        let agrandie = self.window_maximized(Self::DECOMPILER_WINDOW);
        let mut bascule = false;
        let mut fenêtre = egui::Window::new(s.decompiler_title)
            .id(egui::Id::new("decompiler_window"))
            .open(&mut open)
            .collapsible(false);
        fenêtre = if agrandie {
            Self::fill_the_screen(fenêtre, ctx)
        } else {
            fenêtre
                .resizable(true)
                .default_size(window_size(
                    !self.decompilers.is_empty(),
                    egui::vec2(760.0, 560.0),
                    // Sans outil installé, la fenêtre montre un mode
                    // d'emploi : le dossier où déposer un fichier, et un
                    // exemple de ce fichier. C'est court et c'est fixe.
                    egui::vec2(700.0, 400.0),
                ))
                // Bornée en largeur, et ce n'est pas une préférence de
                // goût : sans borne, la première passe de mise en page
                // dispose le texte sur une largeur infinie. Une commande
                // de cent cinquante caractères tient alors sur une
                // ligne, s'alloue la hauteur d'une ligne, puis se peint
                // sur deux une fois la fenêtre ramenée à sa taille — et
                // tout ce qui suit se retrouve dessous d'une ligne trop
                // haut. De proche en proche, la fenêtre entière se
                // chevauche.
                .max_width(900.0)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(default_window_pos(ctx, "decompiler"))
        };
        let rendue = fenêtre.title_bar(false).show(ctx, |ui| {
            if chrome::window_head(ui, self.theme, s, s.decompiler_title) {
                fermer_ok = true;
            }
            if ok_bar(ui, self.theme, strings(self.lang), "decompiler") {
                fermer_ok = true;
            }
            // La largeur de retour à la ligne, dite explicitement :
            // une fenêtre agrandie n'a pas de borne, et le texte
            // s'allouerait de nouveau une ligne pour s'en peindre
            // deux.
            let large = ui.available_width();
            ui.set_max_width(if large.is_finite() && large > 0.0 {
                large.min(900.0)
            } else {
                760.0
            });
            if self.decompilers.is_empty() {
                ui.colored_label(theme.warn, s.decompiler_none_installed);
                ui.add_space(4.0);
                ui.colored_label(theme.dim, s.decompiler_install_hint);
                if let Some(dir) = decompilers_dir() {
                    ui.colored_label(
                        theme.dim,
                        egui::RichText::new(dir.display().to_string()).monospace(),
                    );
                }
                ui.add_space(6.0);
                ui.colored_label(theme.dim, s.decompiler_example);
                ui.add(egui::Label::new(small_monospace(ui, DECOMPILER_EXAMPLE)).selectable(true));
                return;
            }

            // Le choix de l'outil, retenu d'une session à l'autre.
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, s.decompiler_tool);
                let courant = self
                    .decompilers
                    .iter()
                    .find(|outil| outil.id == self.decompiler)
                    .map(|outil| outil.name.clone())
                    .unwrap_or_else(|| s.decompiler_choose.to_owned());
                let outils: Vec<(String, String)> = self
                    .decompilers
                    .iter()
                    .map(|outil| (outil.id.clone(), outil.name.clone()))
                    .collect();
                liste_déroulante(
                    ui,
                    theme,
                    egui::Id::new("decompiler_choice"),
                    0.0,
                    &mut self.decompiler,
                    &outils,
                    &courant,
                );
            });

            let choisi = self
                .decompilers
                .iter()
                .find(|outil| outil.id == self.decompiler)
                .cloned();
            let Some(outil) = choisi else {
                ui.colored_label(theme.dim, s.decompiler_choose_first);
                return;
            };

            // Ce qu'on s'apprête à lancer, en toutes lettres.
            ui.add_space(4.0);
            ui.colored_label(theme.dim, s.decompiler_will_run);
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("{} {}", outil.command, outil.args.join(" ")))
                        .monospace(),
                )
                .selectable(true),
            );
            ui.weak(s.decompiler_file_hint);
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                let occupé = self.decompile_job.is_some();
                let prêt = self.assemble_state.is_some();
                ui.add_enabled_ui(!occupé && prêt, |ui| {
                    lancer = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(s.decompiler_run).color(theme.bg),
                            )
                            .fill(theme.accent),
                        )
                        .clicked();
                });
                if occupé {
                    ui.spinner();
                }
                if let Some(Ok(texte)) = &self.decompile_result
                    && ui.small_button(s.decompiler_copy).clicked()
                {
                    ui.ctx().copy_text(texte.clone());
                }
            });
            // La phrase sur sa propre ligne, et non à la suite des
            // boutons : une rangée qui se replie place son dernier
            // élément à la ligne suivante sans savoir de combien
            // descendre, et le texte se posait sur le bouton.
            if occupé_maintenant(self) {
                ui.colored_label(theme.dim, s.decompiler_working);
            } else if self.assemble_state.is_none() {
                empty_state(
                    ui,
                    theme,
                    s.decompiler_no_binary,
                    Some(s.empty_decompiler),
                    None,
                );
            }

            // Ce qui a été décompilé, et de quel programme il
            // s'agissait : sans cela, un texte resté à l'écran passe
            // pour celui du programme qu'on regarde maintenant.
            if let Some(origine) = &self.decompiled_from {
                ui.weak(format!("{} {origine}", s.decompiler_of));
            }
            ui.separator();
            match &self.decompile_result {
                Some(Ok(_)) => {
                    // Une barre de lecture : chercher un mot, sauter
                    // à une fonction, savoir combien de lignes on a
                    // sous les yeux.
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(theme.dim, s.decompiler_find);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.decompile_filter)
                                .desired_width(200.0)
                                .hint_text(s.decompiler_find_hint),
                        );
                        if !self.decompile_functions.is_empty() {
                            egui::ComboBox::from_id_salt("decompile_functions")
                                .selected_text(format!(
                                    "{} ({})",
                                    s.decompiler_functions,
                                    self.decompile_functions.len()
                                ))
                                .show_ui(ui, |ui| {
                                    for (rang, titre) in self.decompile_functions.clone() {
                                        if ui.selectable_label(false, &titre).clicked() {
                                            self.decompile_filter.clear();
                                            self.decompile_goto = Some(rang);
                                        }
                                    }
                                });
                        }
                        ui.colored_label(
                            theme.dim,
                            format!(
                                "{} / {} {}",
                                self.decompile_visible.len(),
                                self.decompile_lines.len(),
                                s.decompiler_lines
                            ),
                        );
                    });
                    ui.add_space(4.0);

                    // Le filtre ne se recalcule qu'au changement.
                    if self.decompile_filter != self.decompile_filter_applied {
                        let cherché = self.decompile_filter.to_lowercase();
                        self.decompile_visible = if cherché.is_empty() {
                            (0..self.decompile_lines.len()).collect()
                        } else {
                            self.decompile_lines
                                .iter()
                                .enumerate()
                                .filter(|(_, ligne)| ligne.to_lowercase().contains(&cherché))
                                .map(|(rang, _)| rang)
                                .collect()
                        };
                        self.decompile_filter_applied = self.decompile_filter.clone();
                        // Pas de « decompile_goto = None » ici :
                        // sauter à une fonction efface justement le
                        // filtre, et le saut passerait à la trappe.
                    }

                    // Une ligne = une galette de texte, numéro
                    // compris : deux widgets par ligne coûteraient
                    // une espace verticale de plus, et le saut
                    // tomberait à côté.
                    let police = egui::TextStyle::Monospace.resolve(ui.style());
                    // `show_rows` veut la hauteur d'une ligne **sans**
                    // l'espace qui la sépare de la suivante : il
                    // l'ajoute lui-même. Le saut, lui, se calcule sur
                    // le pas complet. Confondre les deux décale
                    // l'arrivée, d'autant plus qu'on descend loin.
                    let ligne_seule = ui.ctx().fonts_mut(|f| f.row_height(&police));
                    let hauteur = ligne_seule + ui.spacing().item_spacing.y;
                    let mut zone = egui::ScrollArea::both()
                        .id_salt("decompile_text")
                        .auto_shrink([false, false]);
                    // Sauter à une fonction : la zone n'a pas de
                    // « aller à la ligne n », mais toutes les lignes
                    // font la même hauteur, alors le calcul suffit.
                    if let Some(rang) = self.decompile_goto.take() {
                        let place = self
                            .decompile_visible
                            .iter()
                            .position(|&l| l >= rang)
                            .unwrap_or(0);
                        zone = zone.vertical_scroll_offset(place as f32 * hauteur);
                    }
                    let largeur = (self.decompile_lines.len().max(1) as f32).log10() as usize + 1;
                    zone.show_rows(
                        ui,
                        ligne_seule,
                        self.decompile_visible.len(),
                        |ui, plage| {
                            for place in plage {
                                let rang = self.decompile_visible[place];
                                let mut texte = egui::text::LayoutJob::default();
                                texte.append(
                                    &format!("{:>largeur$} ", rang + 1),
                                    0.0,
                                    egui::TextFormat {
                                        font_id: police.clone(),
                                        color: theme.dim,
                                        ..Default::default()
                                    },
                                );
                                texte.append(
                                    &self.decompile_lines[rang],
                                    0.0,
                                    egui::TextFormat {
                                        font_id: police.clone(),
                                        color: theme.text,
                                        ..Default::default()
                                    },
                                );
                                ui.add(
                                    egui::Label::new(texte)
                                        .wrap_mode(egui::TextWrapMode::Extend)
                                        .selectable(true),
                                );
                            }
                        },
                    );
                }
                Some(Err(erreur)) => {
                    ui.colored_label(theme.bad, erreur);
                }
                None => {
                    ui.colored_label(theme.dim, s.decompiler_idle);
                }
            }
        });
        self.show_decompiler = open && !fermer_ok;
        if let Some(rendue) = rendue
            && title_bar_maximize_button(
                ctx,
                Self::DECOMPILER_WINDOW,
                rendue.response.rect,
                theme,
                s,
                agrandie,
            )
        {
            bascule = true;
        }
        if bascule {
            self.toggle_maximized(Self::DECOMPILER_WINDOW);
        }
        if lancer {
            self.start_decompile();
        }
    }

    /// Relève ce que l'outil a rendu, s'il a fini.
    pub(crate) fn poll_decompile(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.decompile_job else {
            return;
        };
        match receiver.try_recv() {
            Ok(réponse) => {
                self.decompile_job = None;
                self.job = None;
                if let Ok(texte) = &réponse {
                    self.index_decompilation(texte);
                }
                self.decompile_result = Some(réponse);
            }
            Err(TryRecvError::Empty) => {
                // Le fil travaille : on redemande une image, sinon
                // l'interface s'endort et la réponse attend.
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
            }
            Err(TryRecvError::Disconnected) => {
                self.decompile_job = None;
                self.decompile_result = Some(Err(strings(self.lang).decompiler_lost.to_owned()));
            }
        }
    }
}

/// Où vivent les décompilateurs déclarés par l'utilisateur.
///
/// À côté des désassembleurs, et pour la même raison qu'ils en sont
/// séparés : les uns rendent des instructions, les autres du
/// pseudo-code, et l'on ne choisit pas dans la même liste selon la
/// question qu'on se pose.
/// namsdec sait-il aller chercher cet outil lui-même ?
///
/// Non pour ceux qui viennent des paquets du système : les installer
/// demanderait `sudo`, et namsdec ne le prendra pas.
pub(crate) fn recette_existe(outil: &str) -> bool {
    data_dir().is_some_and(|dir| namsdec_disasm::install::recipe_for(outil, &dir).is_some())
}

pub(crate) fn decompilers_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("decompilers"))
}

/// Les décompilateurs installés. Lus une fois au démarrage.
pub(crate) fn load_installed_decompilers() -> Vec<namsdec_disasm::plugin::Plugin> {
    decompilers_dir()
        .map(|dir| namsdec_disasm::plugin::read_dir(&dir))
        .unwrap_or_default()
}

/// Un travail de décompilation est-il en cours ?
///
/// Nommé plutôt qu'écrit deux fois : la rangée de boutons le lit pour
/// éteindre « Lancer », la ligne d'en dessous pour dire ce qu'on attend.
pub(crate) fn occupé_maintenant(app: &NamsdecApp) -> bool {
    app.decompile_job.is_some()
}
