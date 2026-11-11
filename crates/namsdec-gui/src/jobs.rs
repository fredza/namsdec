//! Ce que le programme fait pendant qu'on fait autre chose.
//!
//! Installer un outil, décompiler un binaire, chercher une mise à jour :
//! trois travaux qui prennent des secondes ou des minutes, et qui ne
//! doivent geler ni la frappe ni le pas à pas.
//!
//! Chacun part dans un fil, rend compte par un canal, et l'interface
//! s'en informe à chaque frame. La règle commune : un travail en cours
//! se VOIT — dans le titre de la fenêtre, dans une barre, dans une
//! phrase — parce qu'un programme qui semble ne rien faire pendant
//! trois minutes passe pour cassé.
//!
//! **Le lancement lui-même ne vit plus ici.** Il est dans
//! `namsdec-disasm::lanceur`, partagé avec `nasm`, `ld`, les
//! désassembleurs extérieurs et `upx` : groupe de processus, journal
//! borné en lignes et en taille de ligne, délai par tâche, arrêt qui
//! atteint l'étape en cours. Ce fichier-ci décide de ce qu'on lance et
//! de ce qu'on en dit ; il ne redécouvre plus comment l'arrêter.
//!
//! Ce que l'arrêt ne peut pas promettre, et qu'il vaut mieux dire : un
//! descendant qui s'est donné un nouveau groupe par `setsid` échappe au
//! signal. Rien ne le rattrape sans un cgroup. L'attente, elle, reste
//! bornée dans tous les cas — c'est ce qui compte pour la fenêtre.

use super::*;

impl NamsdecApp {
    /// Lance la recette d'un outil, étape par étape, dans un fil à part.
    ///
    /// Chaque commande est écrite dans le journal **avant** d'être
    /// lancée, puis ce qu'elle répond y est ajouté : on lit donc ce qui
    /// se passe pendant que ça se passe, et non un compte rendu écrit
    /// après coup par le programme qui l'a fait.
    pub(crate) fn start_install(&mut self, recette: namsdec_disasm::install::Recipe) {
        use std::sync::atomic::{AtomicBool, AtomicUsize};
        use std::sync::{Arc, Mutex};
        if self.install_job.is_some() {
            return;
        }
        let log = namsdec_asm::lanceur::Journal::default();
        let outcome = Arc::new(Mutex::new(None));
        let progress = Arc::new(AtomicUsize::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let total = recette.steps.len();

        let (journal, résultat, avancement, arrêt) = (
            log.clone(),
            Arc::clone(&outcome),
            Arc::clone(&progress),
            Arc::clone(&cancel),
        );
        let attendu = recette.produces.clone();
        let étapes = recette.steps.clone();
        std::thread::spawn(move || {
            use std::sync::atomic::Ordering;
            for (rang, étape) in étapes.iter().enumerate() {
                if arrêt.load(Ordering::Relaxed) {
                    *résultat.lock().expect("le résultat") =
                        Some(Err("arrêté à la demande".to_owned()));
                    return;
                }
                // La commande est écrite AVANT d'être lancée, puis ce
                // qu'elle répond s'y ajoute : on lit donc ce qui se
                // passe pendant que ça se passe, et non un compte rendu
                // écrit après coup par le programme qui l'a fait.
                journal.écrire(format!("$ {}", étape.line()));
                let mut commande = std::process::Command::new(&étape.command);
                commande.args(&étape.args);
                if let Some(dir) = &étape.cwd {
                    commande.current_dir(dir);
                }
                // PAR LE LANCEUR PARTAGÉ, et c'est tout l'objet de P3 :
                // l'enfant part dans son propre groupe, le journal est
                // borné en lignes ET en taille de ligne, et l'arrêt
                // rend la main sans attendre qu'un descendant veuille
                // bien fermer le tube. Mesuré avant la correction :
                // trente secondes pour un arrêt demandé au bout de
                // trois cents millisecondes.
                let bornes = namsdec_asm::Bornes::en(DÉLAI_ÉTAPE).arrêtable(Arc::clone(&arrêt));
                let fin =
                    match namsdec_asm::lanceur::lancer_en_journal(&mut commande, &bornes, &journal)
                    {
                        Ok(fin) => fin,
                        Err(e) => {
                            journal.écrire(e.to_string());
                            *résultat.lock().expect("le résultat") =
                                Some(Err(format!("« {} » : {e}", étape.line())));
                            return;
                        }
                    };
                match fin {
                    namsdec_asm::lanceur::Fin::Arrêté => {
                        journal.écrire("(arrêté)");
                        *résultat.lock().expect("le résultat") =
                            Some(Err("arrêté à la demande".to_owned()));
                        return;
                    }
                    namsdec_asm::lanceur::Fin::TropLong => {
                        let minutes = DÉLAI_ÉTAPE.as_secs() / 60;
                        journal.écrire(format!("(arrêtée après {minutes} minutes)"));
                        *résultat.lock().expect("le résultat") = Some(Err(format!(
                            "« {} » a dépassé {minutes} minutes et a été arrêtée",
                            étape.line()
                        )));
                        return;
                    }
                    namsdec_asm::lanceur::Fin::Terminé(code) => {
                        if code != Some(0) {
                            let code = code
                                .map(|c| c.to_string())
                                .unwrap_or_else(|| "?".to_owned());
                            // Le code de sortie dans le journal aussi :
                            // il ne figurait que dans la note d'échec,
                            // ailleurs dans la fenêtre, et l'on lisait
                            // les deux séparément.
                            journal.écrire(format!("(code {code})"));
                            *résultat.lock().expect("le résultat") =
                                Some(Err(format!("« {} » a échoué (code {code})", étape.line())));
                            return;
                        }
                    }
                }
                avancement.store(rang + 1, Ordering::Relaxed);
            }
            // Le programme attendu est-il vraiment là ? Une suite de
            // commandes qui réussissent ne prouve pas qu'on a obtenu ce
            // qu'on cherchait.
            *résultat.lock().expect("le résultat") = Some(if attendu.is_file() {
                Ok(attendu)
            } else {
                Err(format!(
                    "les commandes ont réussi, mais {} n'existe pas",
                    attendu.display()
                ))
            });
        });

        self.install_job = Some(InstallJob {
            tool: recette.tool.to_owned(),
            total,
            done: 0,
            log,
            outcome,
            progress,
            cancel,
        });
        self.job = Some(BackgroundJob {
            label: format!("{} {}", strings(self.lang).job_installing, recette.tool),
            started: std::time::Instant::now(),
            // Une progression vraie : les étapes faites sur les étapes à
            // faire. Le temps, lui, ne veut rien dire ici — une
            // compilation ne se prévoit pas.
            limit: None,
        });
    }

    /// Où en est l'installation, pour la barre d'état : « 3 / 8 ».
    ///
    /// La fenêtre peut être fermée pendant qu'une compilation tourne :
    /// l'avancement doit se lire ailleurs qu'en la rouvrant.
    pub(crate) fn install_progress(&self) -> Option<(usize, usize)> {
        let job = self.install_job.as_ref()?;
        Some((job.done, job.total))
    }

    /// Déclare l'outil qu'on vient d'installer, avec le chemin exact du
    /// programme obtenu.
    ///
    /// Le chemin absolu plutôt que le nom : ce qu'on vient de poser n'est
    /// pas dans le PATH, et il n'a pas à y être — namsdec sait où il l'a
    /// mis.
    pub(crate) fn declare_installed_tool(&mut self, outil: &str, programme: &Path) {
        let s = strings(self.lang);
        let Some(mut plugin) = namsdec_disasm::known::all()
            .into_iter()
            .find(|connu| connu.plugin.id == outil)
            .map(|connu| connu.plugin)
        else {
            return;
        };
        plugin.command = programme.display().to_string();
        self.declare_tool(&plugin);
        self.plugin_note = Some(PluginNote {
            ok: true,
            text: format!("{} {}", s.install_done, plugin.name),
        });
    }

    /// Porte l'avancement jusque dans la barre des tâches du système.
    ///
    /// Un gestionnaire de fenêtres n'a pas de notion de progression —
    /// aucune qui soit portable, en tout cas : ce que GNOME, KDE et XFCE
    /// affichent tous, c'est le **titre de la fenêtre**. On y écrit donc
    /// le pourcentage, et il apparaît là où l'on regarde quand on a
    /// changé d'application : dans la barre des tâches, à côté du nom.
    ///
    /// Le titre est remis en place dès que le travail s'achève : une
    /// application qui reste marquée « 100 % » indéfiniment fait douter
    /// de tout ce qu'elle affiche.
    pub(crate) fn reflect_job_in_title(&mut self, ctx: &egui::Context) {
        let voulu = match &self.job {
            Some(job) => match job.fraction() {
                Some(part) => format!("{APP_NAME} — {} {:.0} %", job.label, part * 100.0),
                None => format!("{APP_NAME} — {}", job.label),
            },
            // AU REPOS, LE NOM DE CE QU'ON TRAVAILLE. La barre des
            // tâches et l'alt-tab ne montraient que « namsdec » : deux
            // fenêtres ouvertes sur deux programmes étaient
            // indiscernables. Le nom vient de `nom_du_tampon`, donc un
            // binaire chargé s'y nomme aussi.
            None => format!("{APP_NAME} — {}", self.nom_du_tampon()),
        };
        if self.window_title.as_deref() != Some(voulu.as_str()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(voulu.clone()));
            self.window_title = Some(voulu);
        }
    }

    /// Lance l'outil choisi sur le programme lié, dans un fil à part.
    pub(crate) fn start_decompile(&mut self) {
        if self.decompile_job.is_some() {
            return;
        }
        let (Some(state), Some(outil)) = (
            &self.assemble_state,
            self.decompilers
                .iter()
                .find(|outil| outil.id == self.decompiler)
                .cloned(),
        ) else {
            return;
        };
        let octets = state.output.file.clone();
        // Ce qu'on décompile est le plus souvent un binaire ouvert, et
        // un binaire ouvert n'a pas de `current_file` : la mention
        // « décompilé depuis » disait « sans titre » dans le cas même
        // pour lequel elle existe.
        self.decompiled_from = Some(self.nom_du_tampon());
        self.decompile_result = None;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let réponse = outil
                .run_within(&octets, DECOMPILER_TIMEOUT)
                .map_err(|e| e.to_string());
            // Personne n'attend plus si la fenêtre est fermée : l'envoi
            // échoue, et le fil s'achève là.
            let _ = sender.send(réponse);
        });
        self.decompile_job = Some(receiver);
        self.job = Some(BackgroundJob {
            label: strings(self.lang).job_decompiling.to_owned(),
            started: std::time::Instant::now(),
            limit: Some(DECOMPILER_TIMEOUT),
        });
    }

    /// Lance un outil extérieur pour la rubrique qui l'a demandé.
    ///
    /// Le panneau nomme l'outil ; c'est ici qu'on décide — où est le
    /// programme (le PATH, ou le dossier où namsdec installe ce qu'il
    /// bâtit), ce qu'on lui passe, combien de temps on l'attend. Le
    /// panneau, lui, ne sort jamais du bac à sable.
    pub(crate) fn start_tool_run(&mut self, id: &str) {
        if self.tool_job.is_some() {
            return;
        }
        let Some(state) = &self.assemble_state else {
            return;
        };
        let Some(connu) = namsdec_disasm::known::all()
            .into_iter()
            .find(|outil| outil.plugin.id == id)
        else {
            return;
        };
        let dossier = data_dir().map(|dir| namsdec_disasm::install::tools_dir(&dir));
        let mut plugin = connu.resolved_plugin(dossier.as_deref());
        // Un binaire qu'on a localisé à la main l'emporte : c'est la
        // seule voie pour ce qu'on a bâti soi-même hors du PATH.
        if let Some(chemin) = self.located_tools.get(id)
            && chemin.is_file()
        {
            plugin.command = chemin.display().to_string();
        }
        // Ce que namsdec sait du binaire, et que l'outil n'a pas à
        // réapprendre : le fichier entier, ses octets exécutables, et
        // l'adresse où ils vivent.
        let fichier = state.output.file.clone();
        let code: Vec<u8> = state
            .disasm
            .iter()
            .flat_map(|instruction| instruction.bytes.clone())
            .collect();
        let base = state.disasm.first().map(|i| i.address).unwrap_or(0);
        let (sender, receiver) = mpsc::channel();
        let lang = self.lang;
        std::thread::spawn(move || {
            let cible = namsdec_disasm::plugin::Cible {
                fichier: &fichier,
                code: (!code.is_empty()).then_some(&code),
                base,
            };
            let réponse = plugin
                .run_on(&cible, DECOMPILER_TIMEOUT)
                .map(|rendu| texte_de_sortie(&rendu, lang))
                .map_err(|e| e.to_string());
            // Personne n'attend plus si l'on a changé de binaire :
            // l'envoi échoue, et le fil s'achève là.
            let _ = sender.send(réponse);
        });
        self.tool_job = Some((id.to_owned(), receiver));
        self.job = Some(BackgroundJob {
            label: format!(
                "{} — {}",
                strings(self.lang).job_decompiling,
                connu.plugin.name
            ),
            started: std::time::Instant::now(),
            limit: Some(DECOMPILER_TIMEOUT),
        });
    }

    /// Relève ce que l'outil d'une rubrique a rendu, s'il a fini, et le
    /// livre au panneau qui l'attend.
    pub(crate) fn poll_tool_run(&mut self, ctx: &egui::Context) {
        let Some((id, receiver)) = &self.tool_job else {
            return;
        };
        match receiver.try_recv() {
            Ok(réponse) => {
                let id = id.clone();
                self.tool_job = None;
                self.job = None;
                self.re_registry.deliver(&namsdec_re::ReResult::ToolOutput {
                    tool: id,
                    text: réponse,
                });
            }
            Err(TryRecvError::Empty) => {
                // Le fil travaille : on redemande une image, sinon
                // l'interface s'endort et la réponse attend.
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
            }
            Err(TryRecvError::Disconnected) => {
                let id = id.clone();
                self.tool_job = None;
                self.job = None;
                let perdu = strings(self.lang).decompiler_lost.to_owned();
                self.re_registry.deliver(&namsdec_re::ReResult::ToolOutput {
                    tool: id,
                    text: Err(perdu),
                });
            }
        }
    }

    /// Dit à chaque rubrique d'outil ce que namsdec sait de son
    /// programme sur cette machine.
    ///
    /// Relevé ici et non dans le panneau : la question se pose en
    /// parcourant le PATH, et parfois en lançant le programme — ce qui
    /// ne se fait pas soixante fois par seconde.
    pub(crate) fn deliver_tool_states(&mut self) {
        if self.tools_ready.is_empty() {
            self.refresh_tool_availability();
        }
        let états: Vec<(String, namsdec_disasm::known::Availability)> = self
            .tools_ready
            .iter()
            .map(|(id, dispo)| (id.clone(), *dispo))
            .collect();
        for (tool, availability) in états {
            self.re_registry
                .deliver(&namsdec_re::ReResult::ToolState { tool, availability });
        }
    }

    /// Découpe le texte rendu et repère les fonctions qu'il contient.
    ///
    /// La règle est volontairement simple — une ligne sans indentation
    /// qui ouvre une accolade après une parenthèse — parce qu'elle vaut
    /// pour le C de RetDec comme pour le pseudo-code de Rizin. Une
    /// heuristique qui se lit en deux lignes vaut mieux qu'un analyseur
    /// qui se trompe sans le dire.
    pub(crate) fn index_decompilation(&mut self, texte: &str) {
        self.decompile_lines = texte.lines().map(str::to_owned).collect();
        self.decompile_functions = self
            .decompile_lines
            .iter()
            .enumerate()
            .filter(|(_, ligne)| {
                let nu = !ligne.starts_with(' ') && !ligne.starts_with('\t');
                nu && ligne.contains('(') && ligne.trim_end().ends_with('{')
            })
            .map(|(rang, ligne)| (rang, ligne.trim_end_matches(['{', ' ']).to_owned()))
            .collect();
        self.decompile_filter.clear();
        self.decompile_filter_applied.clear();
        self.decompile_visible = (0..self.decompile_lines.len()).collect();
        self.decompile_goto = None;
    }

    /// Demande à GitHub la dernière version publiée, sur un thread.
    ///
    /// Une requête réseau dans la boucle de rendu fige la fenêtre le
    /// temps du voyage ; celle-ci arrive par un canal, relevé à chaque
    /// frame comme la réponse du sélecteur de fichiers.
    #[cfg(feature = "reseau")]
    pub(crate) fn start_update_check(&mut self) {
        if self.update_check.is_some() {
            return;
        }
        self.update_result = None;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            // Une préversion accepte d'en voir d'autres ; une version
            // définitive ne se voit proposer que du définitif.
            let réponse = namsdec_update::fetch_latest(
                namsdec_update::OWNER,
                namsdec_update::REPO,
                prerelease(APP_VERSION).is_some(),
            )
            .map_err(|e| e.to_string());
            // Si la fenêtre s'est fermée entre-temps, plus personne
            // n'attend : l'envoi échoue et le thread s'achève là.
            let _ = sender.send(réponse);
        });
        self.update_check = Some(receiver);
        // Une recherche de mise à jour ne dit pas non plus où elle en
        // est, et rien ne la borne : on montre qu'elle tourne, et depuis
        // combien de temps. Une barre qui avancerait au hasard mentirait.
        self.job = Some(BackgroundJob {
            label: strings(self.lang).job_checking_update.to_owned(),
            started: std::time::Instant::now(),
            limit: None,
        });
    }

    /// Relève la réponse de la recherche, si elle est arrivée.
    #[cfg(feature = "reseau")]
    pub(crate) fn poll_update_check(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.update_check else {
            return;
        };
        match receiver.try_recv() {
            Ok(réponse) => {
                self.update_check = None;
                self.job = None;
                let s = strings(self.lang);
                self.update_result = Some(match réponse {
                    Ok(release) => Ok(namsdec_update::verdict(APP_VERSION, release)),
                    // Le cas le plus probable au début de la vie d'un
                    // dépôt : rien n'a encore été publié. Ce n'est pas
                    // une panne, et le dire ainsi évite d'inquiéter.
                    Err(message) if message.contains("no release") => {
                        Err(s.about_update_none.to_owned())
                    }
                    // Un dépôt privé répond comme un dépôt inexistant :
                    // le message dit les deux, parce qu'on ne peut pas
                    // savoir lequel c'est de l'extérieur.
                    Err(message) if message.contains("not found") => {
                        Err(s.about_update_private.to_owned())
                    }
                    Err(message) => Err(format!("{} : {message}", s.about_update_failed)),
                });
            }
            // Toujours en route : sans ce réveil, la réponse attendrait
            // le prochain mouvement de souris pour être lue.
            Err(TryRecvError::Empty) => ctx.request_repaint_after(DIALOG_POLL_INTERVAL),
            Err(TryRecvError::Disconnected) => {
                self.update_check = None;
                self.job = None;
                self.update_result = Some(Err(strings(self.lang).about_update_failed.to_owned()));
            }
        }
    }
}

/// Le texte qu'un panneau peint à partir de ce qu'un outil a rendu — et
/// ce qu'on en pense.
///
/// Le statut de sortie était jeté : un programme qui écrit
/// « tool failed » sur sa sortie d'erreur puis sort avec le code 7
/// rendait un texte que rien ne distinguait d'un résultat normal. Il
/// est gardé, et il se lit **avant** le texte — un résultat partiel
/// peut être utile, à condition d'être nommé comme tel.
pub(crate) fn texte_de_sortie(rendu: &namsdec_disasm::plugin::ToolOutput, lang: Lang) -> String {
    use namsdec_disasm::plugin::Source;
    let mut lignes = Vec::new();
    match rendu.status {
        Some(0) => {}
        Some(code) => lignes.push(if lang == Lang::Fr {
            format!("⚠ L'outil a échoué (code de sortie {code}). Ce qui suit est ce qu'il a tout de même écrit.")
        } else {
            format!("⚠ The tool failed (exit code {code}). What follows is what it wrote anyway.")
        }),
        None => lignes.push(if lang == Lang::Fr {
            "⚠ L'outil a été arrêté par un signal : il n'a pas fini de lui-même.".to_owned()
        } else {
            "⚠ The tool was killed by a signal: it did not finish on its own.".to_owned()
        }),
    }
    if rendu.source == Source::Stderr && !rendu.text.trim().is_empty() {
        lignes.push(if lang == Lang::Fr {
            "Rien sur la sortie ordinaire ; ce qui suit vient de la sortie d'erreur.".to_owned()
        } else {
            "Nothing on standard output; what follows comes from standard error.".to_owned()
        });
    }
    if rendu.truncated {
        lignes.push(if lang == Lang::Fr {
            "La sortie dépassait 4 Mio : elle est coupée, et incomplète.".to_owned()
        } else {
            "The output was over 4 MiB: it is cut, and incomplete.".to_owned()
        });
    }
    if lignes.is_empty() {
        return rendu.text.clone();
    }
    lignes.push(String::new());
    format!("{}{}", lignes.join("\n"), rendu.text)
}
