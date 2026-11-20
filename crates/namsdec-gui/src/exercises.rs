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
        // Les brouillons de la série s'en vont avec elle : leurs étapes
        // n'existent plus, et plus rien ne pourrait les rouvrir.
        let préfixe = format!("{id}/");
        self.exercise_drafts
            .retain(|étape, _| !étape.starts_with(&préfixe));
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

    /// L'exercice qui PRÉCÈDE `id` dans sa série : son identifiant et
    /// son titre. `None` pour le premier — un parcours a un début.
    ///
    /// LA PRÉCÉDENTE TOUT COURT, et non « la précédente non réussie »
    /// comme pour la suivante. Les deux sens ne servent pas à la même
    /// chose : on avance vers ce qu'on n'a pas fait, on recule vers ce
    /// qu'on vient de faire — pour revoir comment on s'y était pris, ou
    /// pour refaire plus court. Sauter les étapes réussies en reculant
    /// ferait manquer exactement celle qu'on cherche.
    ///
    /// Le parcours n'allait que dans un sens : « Étape suivante » dans
    /// la bande et dans la fenêtre, et rien pour revenir. Il fallait
    /// rouvrir la liste et y retrouver son rang — ou se souvenir que
    /// l'étape d'avant était peut-être restée dans un onglet.
    pub(crate) fn exercise_before(&self, id: &str) -> Option<(String, String)> {
        for pack in &self.exercise_packs {
            let Some(position) = pack
                .exercises
                .iter()
                .position(|exercise| namsdec_exercises::full_id(pack, exercise) == id)
            else {
                continue;
            };
            let précédent = pack.exercises.get(position.checked_sub(1)?)?;
            return Some((
                namsdec_exercises::full_id(pack, précédent),
                précédent.title.get(self.lang.code()).to_owned(),
            ));
        }
        None
    }

    /// Range ce qu'une étape avait dans son tampon, avant que celui-ci
    /// disparaisse — on ferme son onglet, ou le dernier fichier.
    ///
    /// `index` est un rang d'onglet au sens de `tab_labels`, l'onglet
    /// courant compris. Si ce n'est pas une étape, il n'y a rien à
    /// garder et la méthode ne fait rien.
    pub(crate) fn ranger_brouillon(&mut self, index: usize) {
        let (id, écrit) = if index == self.active_tab_index {
            let Some(id) = self.current_exercise.clone() else {
                return;
            };
            (id, self.source.clone())
        } else {
            let rang = if index < self.active_tab_index {
                index
            } else {
                index - 1
            };
            let Some(tab) = self.tabs.get(rang) else {
                return;
            };
            let Some(id) = tab.current_exercise.clone() else {
                return;
            };
            (id, tab.source.clone())
        };
        self.garder_brouillon(id, écrit);
    }

    /// Garde `écrit` comme brouillon de l'étape `id` — ou l'oublie.
    ///
    /// UN BROUILLON QUI NE DIFFÈRE PAS DU CODE DE DÉPART N'EN EST PAS
    /// UN : le garder ferait croire, au retour, à un travail qu'on n'a
    /// pas fait, et il empêcherait une correction d'énoncé d'arriver
    /// jusqu'à l'éditeur. Un tampon vide non plus — on a fermé une
    /// étape à laquelle on n'avait rien laissé.
    pub(crate) fn garder_brouillon(&mut self, id: String, écrit: String) {
        let départ = namsdec_exercises::find(&self.exercise_packs, &id)
            .map(|exercise| exercise.starter_code.as_str());
        if écrit.trim().is_empty() || départ == Some(écrit.as_str()) {
            self.exercise_drafts.remove(&id);
            return;
        }
        self.exercise_drafts.insert(id, écrit);
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
        // LE VERDICT NE SUIT PAS D'UNE ÉTAPE À L'AUTRE. Il parle de la
        // course qu'on vient de demander, et celle-ci appartient à
        // l'étape qu'on quitte : rendu sur la suivante, il dirait
        // « Réussi » devant un programme qu'on n'a pas encore écrit.
        self.verification_demandee = false;
        // Le code de départ est pris MAINTENANT : il appartient à la
        // série, pas au tampon, et l'emprunter une seule fois évite de
        // refaire `find` après chaque déplacement d'onglet.
        let départ = exercise.starter_code.clone();
        // Déjà ouvert : on y retourne, avec ce qu'on y avait écrit.
        if let Some(rang) = self.exercise_tab_index(id) {
            self.go_to_tab(rang);
            return;
        }
        // PLUS D'ONGLET, MAIS LE BROUILLON EST RESTÉ. C'est ici que se
        // lit ce qu'on avait écrit la dernière fois qu'on a quitté
        // cette étape — rangé en mémoire à la fermeture, relu sur
        // demande, c'est-à-dire au moment où l'on revient. À défaut, le
        // code de départ, comme pour une étape qu'on ouvre pour la
        // première fois.
        let écrit = self
            .exercise_drafts
            .get(id)
            .cloned()
            .unwrap_or_else(|| départ.clone());
        // Sinon un onglet à lui, sauf si le tampon courant est vierge :
        // on garde ainsi son propre programme à côté de l'exercice, et
        // l'on peut comparer deux étapes du parcours.
        if self.current_file.is_some() || self.buffer_modified() || self.current_exercise.is_some()
        {
            let courant = self.take_tab();
            let insertion = self.active_tab_index.min(self.tabs.len());
            self.tabs.insert(insertion, courant);
            self.active_tab_index = insertion + 1;
            self.current_exercise = Some(id.to_owned());
            // `saved_source` reste le code de DÉPART, même quand on
            // remet un brouillon : l'étape porte alors la marque du
            // non-enregistré, ce qu'elle avait dans son onglet et ce
            // qu'elle est — du travail qui n'est écrit nulle part.
            self.saved_source = départ;
            self.set_source(écrit);
            self.aller_au_trou();
            self.indice_montré = false;
            self.current_file = None;
            self.assemble_state = None;
            self.emulator = None;
            self.diagnostics.clear();
            self.breakpoint_lines.clear();
            self.status = Status::Idle;
            return;
        }
        self.current_exercise = Some(id.to_owned());
        self.saved_source = départ;
        self.set_source(écrit);
        self.aller_au_trou();
        self.indice_montré = false;
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

    /// L'INDICE de l'étape en cours : ce qu'on dit à qui cherche
    /// encore, sans lui donner le programme.
    ///
    /// Deux sources, et la première est écrite : l'exercice peut porter
    /// son propre indice (`hint`). Faute de quoi, namsdec en CALCULE
    /// un — les instructions que la solution emploie et que le tampon
    /// n'a pas encore. Mesuré le 12 novembre 2026 : quatre-vingts des
    /// cent une étapes en ont un de cette façon, sans qu'on ait rien à
    /// écrire, et il est toujours juste puisqu'il se lit dans la
    /// solution.
    ///
    /// COMPARÉ AU TAMPON, et non au sujet livré : l'indice s'affine à
    /// mesure qu'on écrit, et ce qu'on a déjà trouvé n'est pas répété.
    ///
    /// Rend les instructions, à charge pour l'interface de les rendre
    /// cliquables : chacune a sa fiche, et c'est le chemin le plus
    /// court entre une étape du parcours et les cent quatre-vingt-neuf
    /// fiches du manuel.
    pub(crate) fn exercise_hint(&self) -> Option<(Option<String>, Vec<&'static str>)> {
        let id = self.current_exercise.as_deref()?;
        let exercice = namsdec_exercises::find(&self.exercise_packs, id)?;
        let écrit = exercice
            .hint
            .as_ref()
            .map(|texte| texte.get(self.lang.code()).to_owned());
        let manquantes = match exercice.solution.as_deref() {
            Some(solution) => {
                let déjà = mnemoniques(&self.source);
                mnemoniques(solution)
                    .into_iter()
                    .filter(|mot| !déjà.contains(mot))
                    .collect()
            }
            None => Vec::new(),
        };
        (écrit.is_some() || !manquantes.is_empty()).then_some((écrit, manquantes))
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
}

/// Les instructions qu'un programme ÉCRIT, par fiche et sans doublon.
///
/// Le premier mot de chaque ligne, une fois l'étiquette retirée et le
/// commentaire coupé : c'est ce que le processeur exécutera. Les
/// opérandes ne comptent pas — `mov rax, rbx` emploie `mov`, et `rbx`
/// est un registre, pas une notion de ce parcours-là.
///
/// **Rendu par FICHE et non par nom** : `je` et `jz` sont deux noms du
/// même opcode, et compter deux notions là où il y en a une donnerait
/// un carnet qui flatte. C'est le premier nom de la fiche qui
/// représente le groupe, comme partout ailleurs dans `namsdec-doc`.
pub(crate) fn instructions_du_programme(code: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for mot in mnemoniques_du_programme(code) {
        if let Some(entrée) = namsdec_doc::lookup(&mot)
            && let Some(fiche) = entrée.names.first()
            && !out.contains(fiche)
        {
            out.push(fiche);
        }
    }
    out
}

/// Les mnémoniques qu'un programme écrit, TELLES QU'ON LES A TAPÉES.
///
/// Le même découpage que [`instructions_du_programme`] — c'est lui qui
/// le fait —, mais rendu **par nom** et non par fiche. Les deux vues
/// servent à des questions différentes, et c'est pourquoi il n'y a
/// qu'un seul découpage : deux analyseurs d'assembleur écrits côte à
/// côte finiraient par ne plus lire le même programme.
///
/// La fiche répond à « quelle notion ? » : `je` et `jz` sont deux noms
/// du même opcode, et le carnet des notions n'en compte qu'une. Le nom
/// répond à « que faut-il taper ? », et là ils ne sont plus
/// interchangeables : quelqu'un qui n'a jamais vu que `jnz` ne devine
/// pas `jz`, et quelqu'un à qui on a dit « `setcc` » ne peut pas
/// l'écrire — `cc` n'est pas une condition, c'est la place d'une
/// condition. C'est ce que mesure le contrôle du palier Découverte.
pub(crate) fn mnemoniques_du_programme(code: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for ligne in code.lines() {
        let ligne = ligne.split(';').next().unwrap_or("").trim();
        if ligne.is_empty() {
            continue;
        }
        // `boucle: dec rcx` porte bien une instruction.
        let reste = match ligne.split_once(':') {
            Some((étiquette, suite))
                if !étiquette.contains(char::is_whitespace) && !étiquette.contains('[') =>
            {
                suite.trim()
            }
            _ => ligne,
        };
        let Some(mot) = reste.split_whitespace().next() else {
            continue;
        };
        let mot = mot.to_ascii_lowercase();
        // Une instruction, et non une directive : `section` et `db` ont
        // une fiche eux aussi, et ne s'exécutent pas.
        if namsdec_doc::lookup(&mot).is_some_and(|e| e.kind == namsdec_doc::Kind::Instruction)
            && !out.contains(&mot)
        {
            out.push(mot);
        }
    }
    out
}

impl NamsdecApp {
    /// Les notions que le parcours a déjà fait écrire ou lire.
    ///
    /// **Ce que le carnet répond, et que rien ne répondait.** « Où
    /// j'en suis » se lisait en une fraction — 12/120 —, qui dit le
    /// chemin parcouru et rien de ce qu'on y a appris. Un débutant qui
    /// a fait douze étapes connaît une vingtaine d'instructions ; il ne
    /// le sait pas, ne peut pas les relire, et le parcours ne lui en
    /// montre la trace nulle part.
    ///
    /// **Une étape réussie compte, les autres non.** On ne retient pas
    /// une notion en ouvrant son énoncé : le carnet suit les étapes
    /// dont le but a été atteint, ou dont la question a reçu la bonne
    /// réponse. C'est exactement ce que mesure `exercises_done`.
    ///
    /// **La solution plutôt que le programme de départ**, quand il y en
    /// a une : elle porte ce qu'on a eu à écrire, trou compris. Une
    /// démonstration et une étape de lecture, elles, n'ont que leur
    /// programme — et c'est bien celui-là qu'on a lu.
    ///
    /// Rien n'est retenu en mémoire : le carnet se recalcule des
    /// étapes réussies, qui sont déjà dans `settings.json`. Un état de
    /// plus serait un état de plus à tenir d'accord.
    pub(crate) fn notions_vues(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for pack in &self.exercise_packs {
            for exercise in &pack.exercises {
                let id = namsdec_exercises::full_id(pack, exercise);
                if !self.exercises_done.contains(&id) {
                    continue;
                }
                let programme = exercise
                    .solution
                    .as_deref()
                    .unwrap_or(&exercise.starter_code);
                for fiche in instructions_du_programme(programme) {
                    if !out.contains(&fiche) {
                        out.push(fiche);
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }
}

impl NamsdecApp {
    /// Remet l'étape en cours dans l'état où on l'a trouvée.
    ///
    /// **Pourquoi un geste pour cela.** On essaie une idée, on la rate,
    /// on efface une ligne de trop — et il n'y avait pas de chemin de
    /// retour : il fallait fermer l'onglet, répondre « ne pas
    /// enregistrer » à une question qui parle de fichier alors qu'il
    /// n'y en a pas, puis recharger l'étape depuis la liste. Trois
    /// gestes et une inquiétude pour revenir à un programme qu'on n'a
    /// jamais perdu.
    ///
    /// **Ce qui repart à zéro** : le texte, le brouillon mémorisé, la
    /// machine, les diagnostics, l'indice, la réponse d'une étape de
    /// lecture et le verdict. **Ce qui reste** : la réussite déjà
    /// acquise et le meilleur score — on ne retire pas ce qu'on a
    /// compris, et recommencer pour faire plus court est précisément ce
    /// que le budget encourage.
    pub(crate) fn recommencer_l_exercice(&mut self) {
        let Some(id) = self.current_exercise.clone() else {
            return;
        };
        let Some(exercise) = namsdec_exercises::find(&self.exercise_packs, &id) else {
            return;
        };
        let départ = exercise.starter_code.clone();
        // Le brouillon aussi : sans cela, fermer l'onglet puis revenir
        // rendrait le travail qu'on vient justement de jeter.
        self.exercise_drafts.remove(&id);
        self.saved_source = départ.clone();
        self.set_source(départ);
        self.aller_au_trou();
        self.indice_montré = false;
        self.question_answer.clear();
        self.verification_demandee = false;
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.status = Status::Note(|s| s.exercise_restarted);
    }

    /// Efface tout ce que le parcours a retenu : réussites et scores.
    ///
    /// **Demandé pour une raison qui se comprend d'elle-même** : on
    /// prête l'application, on la montre, ou l'on veut refaire les cent
    /// dix-sept étapes depuis le début sans que les pastilles vertes
    /// disent déjà la réponse.
    ///
    /// **Les brouillons partent avec**, et c'est la même idée : garder
    /// le programme à moitié écrit d'une étape qu'on vient de déclarer
    /// jamais faite serait garder la seule chose qui prouve le
    /// contraire.
    ///
    /// Rien n'est demandé à l'utilisateur ici : la confirmation est
    /// posée par l'interface qui appelle, parce que ceci ne se défait
    /// pas.
    pub(crate) fn reinitialiser_le_parcours(&mut self) {
        self.exercises_done.clear();
        self.exercises_best.clear();
        self.exercise_drafts.clear();
        self.status = Status::Note(|s| s.exercises_reset_done);
    }

    /// Assembler, exécuter, et répondre — en un seul geste.
    ///
    /// **Ce qu'il fallait savoir avant.** Pour apprendre si l'on avait
    /// réussi, il fallait trois gestes dans le bon ordre — Assembler,
    /// Exécuter, puis regarder si la pastille du titre a changé — et
    /// savoir qu'il les fallait. C'est beaucoup pour une question qui
    /// tient en trois mots : « est-ce juste ? ». Un parcours dont
    /// chaque vérification demande de connaître la chaîne de
    /// compilation n'est pas un parcours interactif, c'est un manuel
    /// avec des devoirs.
    ///
    /// **Rien de neuf n'est calculé** : ce sont les deux actions que la
    /// barre porte déjà, dans l'ordre où on les ferait soi-même.
    /// `do_run` attend l'assembleur de lui-même (`assurer_le_programme`),
    /// et refuse quand l'assemblage a échoué — on reste alors devant
    /// les erreurs, qui sont la réponse à ce moment-là.
    ///
    /// **Le drapeau sert au verdict** : il dit que la course en cours a
    /// été demandée POUR VÉRIFIER, ce qui autorise la bande à conclure
    /// franchement quand la machine s'arrête — « Réussi », et non une
    /// pastille qu'il faut remarquer.
    pub(crate) fn verifier_l_exercice(&mut self) {
        self.verification_demandee = true;
        // L'EXÉCUTION EST UNE INTENTION, et non l'appel suivant : nasm
        // travaille dans un fil, et tant qu'il travaille il n'y a pas
        // de machine à lancer. Elle est consommée par `tick_assemble`,
        // au moment où la machine existe — voir là-bas.
        self.executer_apres_assemblage = true;
        self.dispatch(Action::Assemble);
    }

    /// Y a-t-il quelque chose à vérifier dans l'étape en cours ?
    ///
    /// Une démonstration n'a rien à réussir : le bouton y dirait
    /// « vérifiez » devant un programme qu'on regarde tourner. Les deux
    /// autres formes en ont : un but à atteindre, ou une question à
    /// laquelle répondre.
    pub(crate) fn exercice_a_verifier(&self) -> bool {
        self.current_exercise.is_some() && !self.exercise_is_demo()
    }

    /// Peut-on vérifier MAINTENANT, et sinon pourquoi.
    ///
    /// **Une étape de lecture se répond avant de s'exécuter**, et c'est
    /// toute sa leçon : « lisez-le, répondez, puis exécutez — la
    /// machine tranchera ». Vérifier sans avoir répondu ferait tourner
    /// le programme, et le programme DONNE la réponse — RAX vaut 55
    /// dans le volet avant qu'on ait eu l'occasion de se tromper, et
    /// se tromper est la seule façon de savoir qu'on n'avait pas
    /// compris. C'est la règle de `prevision`, appliquée au seul
    /// endroit où elle manquait encore.
    ///
    /// Le bouton se grise donc, et dit pourquoi : une commande éteinte
    /// qui se tait passe pour cassée.
    pub(crate) fn verification_possible(&self) -> bool {
        match self.exercise_question() {
            Some(_) => !self.question_answer.trim().is_empty(),
            None => true,
        }
    }

    /// Le verdict d'une vérification qui vient de se terminer.
    ///
    /// `None` tant qu'on n'a rien demandé, ou tant que la machine
    /// tourne : un verdict rendu au milieu d'une course parlerait d'un
    /// programme qui n'a pas fini sa phrase.
    pub(crate) fn verdict_de_la_verification(&self) -> Option<bool> {
        if !self.verification_demandee {
            return None;
        }
        let emulator = self.emulator.as_ref()?;
        if self.assembling()
            || self.executer_apres_assemblage
            || self.running
            || emulator.executed() == 0
            || !emulator.cpu().halted
        {
            return None;
        }
        Some(self.exercice_reussi())
    }

    /// L'exercice en cours est-il une DÉMONSTRATION ?
    ///
    /// C'est-à-dire : un programme à regarder tourner, sans but à
    /// atteindre. Sept des cent vingt en sont, et le tout
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
    /// L'étape en cours est-elle réussie, en l'état de la machine ?
    ///
    /// **Une seule vérité, et deux lecteurs** : le carnet de bord
    /// ([`Self::note_goal_reached`]), qui l'inscrit une fois pour
    /// toutes, et le verdict de « Vérifier », qui le dit à l'écran.
    /// Écrire la règle deux fois aurait fini par donner un bouton qui
    /// félicite et une pastille qui refuse.
    ///
    /// Les trois formes d'étape, et leur critère :
    ///
    /// - une étape de LECTURE se réussit en répondant juste. L'exécuter
    ///   ne suffit pas : son programme marche déjà, puisqu'il est donné
    ///   complet — sans cette règle, la pastille passait au vert au
    ///   premier « Exécuter », c'est-à-dire en sautant exactement le
    ///   geste que l'étape vient enseigner ;
    /// - une DÉMONSTRATION se termine quand on l'a regardée tourner
    ///   jusqu'au bout : il n'y a rien d'autre à y réussir ;
    /// - les autres demandent que toutes les conditions du but tiennent.
    ///
    /// **Et dans les trois cas, le programme doit avoir tourné jusqu'à
    /// son arrêt.** Un but atteint sans avoir rien exécuté n'est pas une
    /// solution : « rax == 0 » est vrai avant le premier pas, et
    /// remettre à zéro suffisait à s'attribuer un score de zéro
    /// instruction — imbattable, et faux.
    pub(crate) fn exercice_reussi(&self) -> bool {
        let gagné = match self.exercise_question() {
            Some((_, verdict)) => verdict.est_juste(),
            None => self.exercise_is_demo() || matches!(self.exercise_goal(), Some((_, true))),
        };
        gagné
            && self
                .emulator
                .as_ref()
                .is_some_and(|e| e.executed() > 0 && e.cpu().halted)
    }

    pub(crate) fn note_goal_reached(&mut self) {
        // UN EXERCICE DE LECTURE SE RÉUSSIT EN RÉPONDANT, et l'exécuter
        // ne suffit pas : son programme marche déjà, puisqu'il est
        // donné complet. Sans cette règle, la pastille passait au vert
        // au premier « Exécuter » — c'est-à-dire en sautant exactement
        // le geste que l'exercice vient enseigner.
        if !self.exercice_reussi() {
            return;
        }
        // Le programme a tourné jusqu'à son arrêt : `exercice_reussi`
        // vient de l'exiger, et c'est lui qui porte la règle.
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
