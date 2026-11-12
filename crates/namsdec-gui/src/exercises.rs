//! Le parcours d'exercices.
//!
//! Poser un énoncé dans l'éditeur, savoir quand le but est atteint,
//! passer au suivant — et installer ou retirer une série, car le
//! parcours n'est pas dans le binaire : c'est un fichier de données,
//! comme un thème. Une correction d'énoncé ne demande donc pas de
//! recompiler.
//!
//! Le but est reconnu par le programme LUI-MÊME : ce qu'il a écrit, les
//! registres qu'il laisse, le code de sortie. Jamais par la forme de la
//! source — il y a mille façons d'écrire une boucle, et en exiger une
//! apprendrait à imiter plutôt qu'à comprendre.

use super::*;

impl NamsdecApp {
    /// Installe une série d'exercices choisie sur le disque.
    ///
    /// Lue d'abord, copiée ensuite : une série illisible est refusée en
    /// le disant, et rien n'est écrit dans le dossier des données.
    pub(crate) fn install_exercises(&mut self, path: &Path) {
        let s = strings(self.lang);
        let note = match namsdec_exercises::pack::read(path) {
            Err(e) => PluginNote {
                ok: false,
                text: errors::exercise_pack(self.lang, &e),
            },
            Ok(pack) => {
                if pack.id == namsdec_exercises::BUILTIN_ID {
                    // Le nom de la série livrée est pris : la laisser
                    // écraser ferait disparaître les exercices d'origine
                    // sans que personne ne l'ait demandé.
                    PluginNote {
                        ok: false,
                        text: format!("{} ({})", s.exercises_id_taken, pack.id),
                    }
                } else {
                    let name = pack.name.clone();
                    let id = pack.id.clone();
                    match copy_into_dir(path, exercises_dir()) {
                        Err(e) => PluginNote {
                            ok: false,
                            text: format!("{} ({e})", s.exercises_pack_not_kept),
                        },
                        Ok(destination) => {
                            // Une série réinstallée remplace la
                            // précédente au lieu de s'ajouter à côté.
                            self.exercise_packs.retain(|p| p.id != id);
                            self.exercise_packs.push(pack);
                            self.exercise_files.insert(id, destination);
                            PluginNote {
                                ok: true,
                                text: format!("{} — {name}", s.exercises_pack_added),
                            }
                        }
                    }
                }
            }
        };
        self.plugin_note = Some(note);
    }

    /// Retire une série installée : son fichier est supprimé, et elle
    /// quitte la liste. La série livrée avec l'application n'est pas
    /// concernée — elle n'est nulle part sur le disque.
    pub(crate) fn remove_exercises(&mut self, id: &str) {
        let s = strings(self.lang);
        let Some(path) = self.exercise_files.get(id).cloned() else {
            return;
        };
        if let Err(e) = std::fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            self.plugin_note = Some(PluginNote {
                ok: false,
                text: format!("{} ({e})", s.plugins_remove_failed),
            });
            return;
        }
        let name = self
            .exercise_packs
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| id.to_owned());
        self.exercise_packs.retain(|p| p.id != id);
        self.exercise_files.remove(id);
        self.plugin_note = Some(PluginNote {
            ok: true,
            text: format!("{} — {name}", s.plugins_removed),
        });
    }

    /// La liste des exercices : un titre, ce qu'on y cherche, et de quoi
    /// le charger dans l'éditeur.
    ///
    /// Charger remplace le tampon — d'où l'avertissement quand le fichier
    /// courant n'a pas été enregistré. Il n'y a pas de correction
    /// automatique : l'énoncé dit ce qu'on doit obtenir, et c'est le
    /// bandeau des registres qui répond.
    /// L'exercice qui suit `id` dans sa série : son identifiant et son
    /// titre. `None` pour le dernier — un parcours a une fin.
    pub(crate) fn exercise_after(&self, id: &str) -> Option<(String, String)> {
        for pack in &self.exercise_packs {
            let Some(position) = pack
                .exercises
                .iter()
                .position(|exercise| namsdec_exercises::full_id(pack, exercise) == id)
            else {
                continue;
            };
            // La prochaine **non réussie**, et non la suivante dans la
            // liste : reprendre un parcours ne doit pas obliger à
            // repasser sur ce qu'on a déjà fait. À défaut — tout est
            // réussi jusqu'au bout —, celle qui suit.
            let suivant = pack
                .exercises
                .iter()
                .skip(position + 1)
                .find(|exercise| {
                    !self
                        .exercises_done
                        .contains(&namsdec_exercises::full_id(pack, exercise))
                })
                .or_else(|| pack.exercises.get(position + 1))?;
            return Some((
                namsdec_exercises::full_id(pack, suivant),
                suivant.title.get(self.lang.code()).to_owned(),
            ));
        }
        None
    }

    /// Le rang de l'exercice `id` dans sa série, le nombre d'étapes, et
    /// l'exercice lui-même.
    pub(crate) fn exercise_rank(
        &self,
        id: &str,
    ) -> Option<(usize, usize, &namsdec_exercises::Exercise)> {
        for pack in &self.exercise_packs {
            for (index, exercise) in pack.exercises.iter().enumerate() {
                if namsdec_exercises::full_id(pack, exercise) == id {
                    return Some((index + 1, pack.exercises.len(), exercise));
                }
            }
        }
        None
    }

    /// Met le programme de départ d'un exercice dans l'éditeur.
    ///
    /// `current_file` repart à zéro : ce qui est chargé est un modèle, pas
    /// le fichier de l'utilisateur, et « Enregistrer » ne doit pas écrire
    /// par-dessus celui qu'il avait ouvert.
    pub(crate) fn load_exercise(&mut self, id: &str) {
        self.guard(Pending::Exercise(id.to_owned()));
    }

    pub(crate) fn load_exercise_now(&mut self, id: &str) {
        let Some(exercise) = namsdec_exercises::find(&self.exercise_packs, id) else {
            return;
        };
        // Déjà ouvert : on y retourne, avec ce qu'on y avait écrit.
        if let Some(rang) = self.exercise_tab_index(id) {
            self.go_to_tab(rang);
            return;
        }
        // Sinon un onglet à lui, sauf si le tampon courant est vierge :
        // on garde ainsi son propre programme à côté de l'exercice, et
        // l'on peut comparer deux étapes du parcours.
        if self.current_file.is_some() || self.buffer_modified() || self.current_exercise.is_some()
        {
            let courant = self.take_tab();
            let insertion = self.active_tab_index.min(self.tabs.len());
            self.tabs.insert(insertion, courant);
            self.active_tab_index = insertion + 1;
            // `find` a emprunté `self` : on le refait après le
            // déplacement, sur un état stable.
            let Some(exercise) = namsdec_exercises::find(&self.exercise_packs, id) else {
                return;
            };
            self.current_exercise = Some(id.to_owned());
            self.saved_source = exercise.starter_code.clone();
            self.set_source(exercise.starter_code.clone());
            self.aller_au_trou();
            self.current_file = None;
            self.assemble_state = None;
            self.emulator = None;
            self.diagnostics.clear();
            self.breakpoint_lines.clear();
            self.status = Status::Idle;
            return;
        }
        self.current_exercise = Some(id.to_owned());
        self.saved_source = exercise.starter_code.clone();
        self.set_source(exercise.starter_code.clone());
        self.aller_au_trou();
        // La réponse d'avant n'est pas celle de celui-ci : la garder
        // ferait dire « juste » à une question qu'on n'a pas lue.
        self.question_answer.clear();
        self.current_file = None;
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.breakpoint_lines.clear();
        self.status = Status::Idle;
    }

    /// Pose le curseur sur le TROU de l'exercice qu'on vient d'ouvrir,
    /// et y amène la vue.
    ///
    /// Un exercice s'ouvre sur vingt lignes dont une seule est à
    /// écrire — « ; À FAIRE : … ». Sans cela, on lit l'énoncé, on
    /// regarde le programme, et l'on cherche OÙ. Le curseur y est
    /// maintenant : le premier caractère tapé part au bon endroit.
    ///
    /// `reprise_ligne` et non `goto_line` : celui-ci est un GESTE — il
    /// passe en Insertion et prend le clavier —, ce qui serait une
    /// surprise sur une étape qu'on vient seulement d'ouvrir. On amène
    /// la vue, on pose le curseur, et l'on ne change pas de mode.
    pub(crate) fn aller_au_trou(&mut self) {
        let Some(position) = self.source.find("À FAIRE :") else {
            self.reprise_ligne = None;
            return;
        };
        let ligne = self.source[..position].lines().count().max(1) as u32;
        self.reprise_ligne = (ligne > 1).then_some(ligne);
        self.cursor_pos = (ligne as usize, 1);
        self.cursor_byte = ligne_début(&self.source, ligne.saturating_sub(1) as usize).unwrap_or(0);
    }

    /// Ce qu'il reste à écrire, lu dans le tampon lui-même.
    ///
    /// Les consignes « ; À FAIRE : … » du programme, rendues en clair.
    /// La bande les répète SOUS l'énoncé, et c'est un retour vivant :
    /// elles disparaissent à mesure qu'on les remplace par du code. On
    /// sait donc, sans rien lancer, ce qui reste — et l'énoncé n'a plus
    /// à être relu pour s'en souvenir.
    ///
    /// Lu dans la SOURCE et non dans l'exercice : c'est ce qu'on a sous
    /// les yeux qui fait foi, pas ce qui a été livré.
    pub(crate) fn consignes_du_trou(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut dans_une_consigne = false;
        for ligne in self.source.lines() {
            let nu = ligne.trim_start();
            let Some(commentaire) = nu.strip_prefix(';') else {
                dans_une_consigne = false;
                continue;
            };
            let commentaire = commentaire.trim_start();
            if let Some(consigne) = commentaire
                .strip_prefix("À FAIRE :")
                .or_else(|| commentaire.strip_prefix("À faire :"))
            {
                out.push(consigne.trim().to_owned());
                dans_une_consigne = true;
            } else if dans_une_consigne {
                // La suite de la consigne : les lignes de continuation
                // sont alignées sous elle, et n'ont pas de nouveau
                // titre. Une ligne de commentaire ORDINAIRE arrête la
                // reprise — elle parle d'autre chose.
                if commentaire.is_empty() || !ligne.contains("   ") {
                    dans_une_consigne = false;
                    continue;
                }
                if let Some(dernière) = out.last_mut() {
                    dernière.push(' ');
                    dernière.push_str(commentaire.trim());
                }
            }
        }
        out
    }

    /// La solution de l'étape en cours, s'il y en a une.
    ///
    /// Une démonstration et un exercice de lecture n'en ont pas : il n'y
    /// a pas de programme à écrire, et montrer « la solution » d'un
    /// programme déjà complet n'aurait aucun sens.
    pub(crate) fn exercise_solution(&self) -> Option<&str> {
        let id = self.current_exercise.as_deref()?;
        namsdec_exercises::find(&self.exercise_packs, id)?
            .solution
            .as_deref()
    }

    /// La fenêtre qui montre le programme entier.
    ///
    /// EN LECTURE SEULE, ET SANS RIEN REMPLACER. Un bouton qui poserait
    /// la solution dans l'éditeur écraserait ce qu'on vient d'écrire —
    /// c'est-à-dire exactement le travail qu'on venait montrer. On la
    /// lit, on la copie si l'on veut, et l'on garde le sien sous les
    /// yeux.
    pub(crate) fn solution_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_solution {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let Some(solution) = self.exercise_solution().map(str::to_owned) else {
            // Demandée là où il n'y en a pas : on le dit, plutôt que
            // d'ouvrir une fenêtre vide.
            self.show_solution = false;
            self.status = Status::Note(|s| s.exercise_solution_none);
            return;
        };
        let budget = self
            .current_exercise
            .as_deref()
            .and_then(|id| namsdec_exercises::find(&self.exercise_packs, id))
            .and_then(|exercice| exercice.par);
        let mut fermer = false;
        let mut copier = false;
        let fenêtre = egui::Window::new(s.exercise_solution_title)
            .id(egui::Id::new("exercise_solution"))
            .collapsible(false)
            .resizable(true)
            .default_size([520.0, 420.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "exercise_solution"));
        fenêtre.title_bar(false).show(ctx, |ui| {
            if chrome::window_head(ui, theme, s, s.exercise_solution_title) {
                fermer = true;
            }
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.colored_label(theme.dim, s.exercise_solution_intro);
            if let Some(budget) = budget {
                ui.colored_label(
                    theme.dim,
                    format!(
                        "{} {budget} {}",
                        s.exercise_solution_cost, s.bytes_instructions
                    ),
                );
            }
            ui.add_space(6.0);
            // La place qui reste, et non une fraction de l'écran : une
            // zone qui se croit plus haute que sa fenêtre ne défile pas,
            // elle déborde.
            let reste = (ui.available_height() - 36.0).max(80.0);
            egui::ScrollArea::vertical()
                .max_height(reste)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&solution).monospace().color(theme.text),
                        )
                        .selectable(true),
                    );
                });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if framed_button(ui, theme, s.exercise_solution_copy).clicked() {
                    copier = true;
                }
                if framed_button(ui, theme, s.dialog_close).clicked() {
                    fermer = true;
                }
            });
        });
        if copier {
            ctx.copy_text(solution);
            self.status = Status::Note(|s| s.exercise_solution_copied);
        }
        if fermer || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_solution = false;
        }
    }

    /// L'exercice en cours est-il une DÉMONSTRATION ?
    ///
    /// C'est-à-dire : un programme à regarder tourner, sans but à
    /// atteindre. Neuf des cent dix-sept en sont, et le tout
    /// premier du parcours aussi — « Registres et largeurs » montre ce
    /// qu'une écriture 32 bits fait à la moitié haute, et il n'y a rien
    /// à y réussir.
    pub(crate) fn exercise_is_demo(&self) -> bool {
        self.current_exercise
            .as_deref()
            .and_then(|id| namsdec_exercises::find(&self.exercise_packs, id))
            // UN EXERCICE DE LECTURE N'EST PAS UNE DÉMONSTRATION, même
            // s'il n'a pas de but : il y a quelque chose à y réussir —
            // répondre juste —, et lui dire « regardez tourner » serait
            // lui retirer sa question.
            .is_some_and(|exercise| exercise.goal.is_empty() && exercise.question.is_none())
    }

    /// La question de l'exercice en cours, et où en est la réponse.
    ///
    /// `None` quand l'exercice n'en pose pas — les cent huit autres,
    /// ceux d'écriture.
    pub(crate) fn exercise_question(
        &self,
    ) -> Option<(
        &namsdec_exercises::Question,
        namsdec_exercises::reponse::Verdict,
    )> {
        let id = self.current_exercise.as_deref()?;
        let exercise = namsdec_exercises::find(&self.exercise_packs, id)?;
        let question = exercise.question.as_ref()?;
        let verdict = namsdec_exercises::reponse::verdict(&self.question_answer, &question.réponse);
        Some((question, verdict))
    }

    /// Note l'exercice en cours comme réussi si son but est atteint.
    ///
    /// Appelé à chaque image, mais n'écrit qu'une fois : le premier
    /// passage au vert suffit, et repasser sur un exercice déjà fait ne
    /// le défait pas — on ne retire pas ce qu'on a compris.
    ///
    /// UNE DÉMONSTRATION SE TERMINE QUAND ON L'A REGARDÉE TOURNER
    /// jusqu'au bout. C'était le défaut : sept exercices n'ont pas de
    /// but — ce sont des programmes à observer —, `exercise_goal` rend
    /// `None` pour eux, et l'on sortait ici sans rien noter. Le premier
    /// exercice du parcours en est un : on l'exécutait jusqu'au `hlt`,
    /// la pastille restait creuse, « en cours » ne partait jamais, et
    /// rien ne disait pourquoi. Un parcours dont la première étape ne
    /// peut pas se finir n'invite pas à faire la deuxième.
    ///
    /// Le critère n'est pas inventé pour l'occasion : c'est celui des
    /// autres, moins les conditions. Avoir exécuté quelque chose, et
    /// être allé jusqu'à l'arrêt.
    pub(crate) fn note_goal_reached(&mut self) {
        // UN EXERCICE DE LECTURE SE RÉUSSIT EN RÉPONDANT, et l'exécuter
        // ne suffit pas : son programme marche déjà, puisqu'il est
        // donné complet. Sans cette règle, la pastille passait au vert
        // au premier « Exécuter » — c'est-à-dire en sautant exactement
        // le geste que l'exercice vient enseigner.
        if let Some((_, verdict)) = self.exercise_question() {
            if !verdict.est_juste() {
                return;
            }
        } else {
            let démonstration = self.exercise_is_demo();
            if !démonstration && !matches!(self.exercise_goal(), Some((_, true))) {
                return;
            }
        }
        // Un but atteint sans avoir rien exécuté n'est pas une
        // solution : « rax == 0 » est vrai avant le premier pas, et
        // remettre à zéro suffisait à s'attribuer un score de zéro
        // instruction — imbattable, et faux. On exige que le programme
        // ait tourné *et* qu'il soit allé jusqu'à son arrêt.
        let Some(emulator) = &self.emulator else {
            return;
        };
        if emulator.executed() == 0 || !emulator.cpu().halted {
            return;
        }
        let Some(id) = self.current_exercise.clone() else {
            return;
        };
        // Le coût de la course qui vient de réussir. Le meilleur est
        // gardé, jamais le dernier : on essaie une idée plus courte, elle
        // échoue, et le score d'avant n'a pas à en pâtir.
        let exécutées = self.emulator.as_ref().map(|e| e.executed()).unwrap_or(0);
        let meilleur = self.exercises_best.entry(id.clone()).or_insert(exécutées);
        if exécutées < *meilleur {
            *meilleur = exécutées;
        }
        if self.exercises_done.insert(id) {
            self.status = Status::Note(|s| s.exercises_done_note);
        }
    }

    /// Les conditions du but, telles que l'exercice les a écrites,
    /// SANS les éprouver.
    ///
    /// C'est ce qu'on montre tant que rien n'a tourné : la cible se
    /// lit avant de commencer, et c'est elle qui dirige ce qu'on
    /// écrit. [`Self::exercise_goal`], lui, demande une machine à
    /// interroger et rend un verdict.
    ///
    /// `None` quand l'exercice n'a pas de but — une démonstration, une
    /// étape de lecture : il n'y a alors rien à viser, et le dire
    /// autrement serait inventer un devoir.
    pub(crate) fn exercise_targets(&self) -> Option<Vec<String>> {
        let id = self.current_exercise.as_deref()?;
        let exercise = namsdec_exercises::find(&self.exercise_packs, id)?;
        (!exercise.goal.is_empty()).then(|| exercise.goal.clone())
    }

    /// Où en est le but de l'exercice en cours.
    ///
    /// `None` quand aucun exercice n'est chargé, qu'il n'a pas de but
    /// vérifiable, ou qu'aucun programme ne tourne : on ne dit pas
    /// « pas atteint » à quelqu'un qui n'a encore rien lancé.
    ///
    /// Le but est une liste de conditions — la grammaire des points
    /// d'arrêt conditionnels — et elles doivent toutes tenir.
    ///
    /// Chacune est rendue avec son constat, parce que « pas encore » ne
    /// dit rien à qui ne voit pas laquelle manque : un but à trois
    /// conditions dont une seule échoue se lit alors d'un coup d'œil,
    /// et l'on sait quel registre regarder.
    pub(crate) fn exercise_goal(&self) -> Option<(Vec<GoalLine>, bool)> {
        let id = self.current_exercise.as_deref()?;
        let exercise = namsdec_exercises::find(&self.exercise_packs, id)?;
        if exercise.goal.is_empty() {
            return None;
        }
        let emulator = self.emulator.as_ref()?;
        let cpu = emulator.cpu();
        let mémoire = PeekMemory(&cpu.mem);
        let lignes: Vec<GoalLine> = exercise
            .goal
            .iter()
            .map(|texte| {
                // Une condition qu'on ne sait pas lire ne vaut pas
                // « atteint » : mieux vaut un but qu'on n'atteint
                // jamais qu'un faux succès.
                let Some(c) = condition::parse(texte) else {
                    return GoalLine {
                        texte: texte.clone(),
                        tenue: false,
                        constat: None,
                    };
                };
                let tenue = c.holds(&cpu.regs, Some(&mémoire));
                // Le constat n'est écrit que pour ce qui manque :
                // répéter « rax vaut 15 » sous « rax == 15 » n'apprend
                // rien, et allonge ce qu'on relit à chaque pas.
                let constat = (!tenue)
                    .then(|| c.sides(&cpu.regs, Some(&mémoire)))
                    .flatten()
                    .map(|(gauche, _)| (c.left().to_owned(), gauche));
                GoalLine {
                    texte: texte.clone(),
                    tenue,
                    constat,
                }
            })
            .collect();
        let atteint = lignes.iter().all(|l| l.tenue);
        Some((lignes, atteint))
    }
}

/// Où vivent les séries d'exercices installées. À côté des thèmes, et
/// pour la même raison : ce sont des données que l'utilisateur ajoute, et
/// elles doivent survivre à une mise à jour de l'application.
pub(crate) fn exercises_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("exercises"))
}

/// Charge les séries d'exercices déjà installées. Comme pour les thèmes :
/// un fichier illisible est nommé dans le compte rendu et n'empêche pas
/// les autres de se charger.
pub(crate) fn load_installed_exercises() -> (
    Vec<namsdec_exercises::Pack>,
    BTreeMap<String, PathBuf>,
    Vec<String>,
) {
    let mut packs = Vec::new();
    let mut files = BTreeMap::new();
    let mut failures = Vec::new();
    let Some(dir) = exercises_dir() else {
        return (packs, files, failures);
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return (packs, files, failures);
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    // La série livrée d'abord, quel que soit l'ordre alphabétique :
    // c'est par elle qu'on commence, et les autres viennent ensuite.
    let livrée = dir.join(namsdec_exercises::BUILTIN_FILE);
    paths.sort_by_key(|path| *path != livrée);
    for path in paths {
        let est_livrée = path == livrée;
        match namsdec_exercises::pack::read(&path) {
            Ok(pack) if est_livrée || pack.id != namsdec_exercises::BUILTIN_ID => {
                files.insert(pack.id.clone(), path);
                packs.push(pack);
            }
            // Une série qui se fait passer pour celle d'origine est
            // ignorée plutôt que de la remplacer en silence.
            Ok(pack) => failures.push(format!("{} : {}", path.display(), pack.id)),
            Err(e) => failures.push(format!("{} : {e}", path.display())),
        }
    }
    (packs, files, failures)
}
