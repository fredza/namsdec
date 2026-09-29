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
            self.source = exercise.starter_code.clone();
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
        self.source = exercise.starter_code.clone();
        self.current_file = None;
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.breakpoint_lines.clear();
        self.status = Status::Idle;
    }

    /// Note l'exercice en cours comme réussi si son but est atteint.
    ///
    /// Appelé à chaque image, mais n'écrit qu'une fois : le premier
    /// passage au vert suffit, et repasser sur un exercice déjà fait ne
    /// le défait pas — on ne retire pas ce qu'on a compris.
    pub(crate) fn note_goal_reached(&mut self) {
        let Some((_, true)) = self.exercise_goal() else {
            return;
        };
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
