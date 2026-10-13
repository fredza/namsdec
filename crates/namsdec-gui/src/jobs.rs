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
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let outcome = Arc::new(Mutex::new(None));
        let progress = Arc::new(AtomicUsize::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let total = recette.steps.len();

        let (journal, résultat, avancement, arrêt) = (
            Arc::clone(&log),
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
                    *résultat.lock().expect("le journal") =
                        Some(Err("arrêté à la demande".to_owned()));
                    return;
                }
                journal
                    .lock()
                    .expect("le journal")
                    .push(format!("$ {}", étape.line()));
                let mut commande = std::process::Command::new(&étape.command);
                commande
                    .args(&étape.args)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());
                if let Some(dir) = &étape.cwd {
                    commande.current_dir(dir);
                }
                // On lance l'enfant plutôt que d'attendre sa sortie
                // complète : un téléchargement de 180 Mo ou une
                // compilation de quinze minutes doivent s'écrire au fur
                // et à mesure, et pouvoir s'arrêter avant la fin.
                let mut enfant = match commande.spawn() {
                    Ok(enfant) => enfant,
                    Err(e) => {
                        journal.lock().expect("le journal").push(e.to_string());
                        *résultat.lock().expect("le résultat") =
                            Some(Err(format!("« {} » : {e}", étape.line())));
                        return;
                    }
                };
                let mut lecteurs = Vec::new();
                if let Some(flux) = enfant.stdout.take() {
                    lecteurs.push(lire_flux(flux, Arc::clone(&journal)));
                }
                if let Some(flux) = enfant.stderr.take() {
                    lecteurs.push(lire_flux(flux, Arc::clone(&journal)));
                }
                let statut = loop {
                    if arrêt.load(Ordering::Relaxed) {
                        let _ = enfant.kill();
                        let _ = enfant.wait();
                        journal
                            .lock()
                            .expect("le journal")
                            .push("(arrêté)".to_owned());
                        *résultat.lock().expect("le résultat") =
                            Some(Err("arrêté à la demande".to_owned()));
                        return;
                    }
                    match enfant.try_wait() {
                        Ok(Some(statut)) => break statut,
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                        Err(e) => {
                            *résultat.lock().expect("le résultat") =
                                Some(Err(format!("« {} » : {e}", étape.line())));
                            return;
                        }
                    }
                };
                for lecteur in lecteurs {
                    let _ = lecteur.join();
                }
                if !statut.success() {
                    let code = statut
                        .code()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "?".to_owned());
                    // Le code de sortie dans le journal aussi : il ne
                    // figurait que dans la note d'échec, ailleurs dans la
                    // fenêtre, et l'on lisait les deux séparément.
                    journal
                        .lock()
                        .expect("le journal")
                        .push(format!("(code {code})"));
                    *résultat.lock().expect("le résultat") =
                        Some(Err(format!("« {} » a échoué (code {code})", étape.line())));
                    return;
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
            None => APP_NAME.to_owned(),
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
        self.decompiled_from = Some(
            self.current_file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|nom| nom.to_string_lossy().into_owned())
                .unwrap_or_else(|| strings(self.lang).untitled_buffer.to_owned()),
        );
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
        std::thread::spawn(move || {
            let cible = namsdec_disasm::plugin::Cible {
                fichier: &fichier,
                code: (!code.is_empty()).then_some(&code),
                base,
            };
            let réponse = plugin
                .run_on(&cible, DECOMPILER_TIMEOUT)
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
