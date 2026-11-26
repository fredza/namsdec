//! Les fichiers et leurs onglets.
//!
//! Ouvrir, enregistrer, recharger quand le disque a changé sous nos
//! pieds, exporter — et tenir la rangée d'onglets qui dit ce qui est
//! ouvert. Un onglet n'est pas qu'un titre : il porte son tampon, son
//! curseur, ses marques, et les retrouve intacts au retour.
//!
//! La garde des fichiers modifiés est ici aussi. Elle ne demande
//! qu'une chose — « et celui-là, vous le gardez ? » — mais elle doit la
//! demander à chaque sortie possible, ce qui fait beaucoup de chemins
//! pour une seule question.

use super::*;

impl NamsdecApp {
    pub(crate) fn do_open_folder(&mut self) {
        self.spawn_file_dialog(FileDialogKind::OpenFolder);
    }

    /// Sélecteur natif de fichier. Accessible directement (Ctrl+O, menu
    /// Fichier, palette) sans avoir à ouvrir l'explorateur d'abord.
    pub(crate) fn do_open_file_dialog(&mut self) {
        self.spawn_file_dialog(FileDialogKind::OpenFile);
    }

    /// Ouvre le sélecteur natif sur un thread et note qu'on attend sa
    /// réponse — voir `PendingDialog` pour la raison du thread.
    ///
    /// Un seul à la fois : tant qu'une boîte est ouverte, un second
    /// raccourci n'en empile pas une deuxième par-dessus.
    pub(crate) fn spawn_file_dialog(&mut self, kind: FileDialogKind) {
        if self.pending_dialog.is_some() {
            return;
        }
        let dialog = self.file_dialog(kind);
        let (sender, result) = mpsc::channel();
        std::thread::spawn(move || {
            let picked = match kind {
                FileDialogKind::OpenFile
                | FileDialogKind::AddTheme
                | FileDialogKind::AddExercises
                | FileDialogKind::LocateTool => dialog.pick_file(),
                FileDialogKind::OpenFolder => dialog.pick_folder(),
                FileDialogKind::SaveAs
                | FileDialogKind::ExportManual
                | FileDialogKind::ExportReport
                | FileDialogKind::ExportDump
                | FileDialogKind::ExportDot
                | FileDialogKind::ExportKeymap
                | FileDialogKind::ExportBinary
                | FileDialogKind::UnpackUpx => dialog.save_file(),
            };
            // Si l'application s'est arrêtée entre-temps, le résultat
            // n'intéresse plus personne : l'envoi échoue et le thread
            // s'achève là.
            let _ = sender.send(picked);
        });
        self.pending_dialog = Some(PendingDialog { kind, result });
    }

    /// Le sélecteur configuré pour ce qu'on cherche — ses filtres pour un
    /// fichier (source assembleur, binaire, ou tout), rien de particulier
    /// pour un dossier ou un enregistrement.
    pub(crate) fn file_dialog(&self, kind: FileDialogKind) -> rfd::FileDialog {
        let s = strings(self.lang);
        let dialog = rfd::FileDialog::new();
        match kind {
            // Un thème est un fichier JSON — le filtre le dit, sans
            // interdire d'en choisir un autre : c'est la lecture qui
            // tranche, pas l'extension.
            FileDialogKind::AddTheme => dialog.add_filter(s.filter_theme, &["json"]),
            FileDialogKind::AddExercises => dialog.add_filter(s.filter_exercises, &["json"]),
            FileDialogKind::LocateTool => dialog.add_filter(s.filter_all, &["*"]),
            FileDialogKind::OpenFile => dialog
                .add_filter(s.filter_asm, &["asm", "nasm", "s", "inc"])
                .add_filter(
                    s.filter_binary,
                    &["bin", "o", "obj", "elf", "so", "exe", "dll"],
                )
                // "*" est traduit en glob `*` par le backend portail de
                // rfd : c'est ce qui laisse choisir un exécutable ELF sans
                // extension, cas le plus courant sous Linux.
                .add_filter(s.filter_all, &["*"]),
            FileDialogKind::ExportManual => dialog
                .add_filter(s.filter_markdown, &["md"])
                .set_file_name("namsdec-manuel.md"),
            FileDialogKind::ExportBinary => dialog.set_file_name(
                self.current_file
                    .as_ref()
                    .and_then(|path| path.file_stem())
                    .map(|nom| nom.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "programme".to_owned()),
            ),
            FileDialogKind::ExportReport => dialog
                .add_filter(s.filter_markdown, &["md"])
                // ET LE JSON, qui est l'autre moitié de ce que la
                // frontière permet : le Markdown se lit, le JSON se
                // traite. Les deux sortent des mêmes vues, et c'est
                // l'extension choisie qui décide — un format qu'on
                // demande en nommant son fichier, plutôt qu'un bouton
                // de plus dans une barre qui en porte déjà trois.
                .add_filter(s.filter_json, &["json"])
                // Le nom du fichier analysé, pour ne pas se retrouver
                // avec trois rapports qui portent le même nom.
                .set_file_name(format!(
                    "{}-analyse.md",
                    self.current_file
                        .as_ref()
                        .and_then(|path| path.file_stem())
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "namsdec".to_owned())
                )),
            FileDialogKind::ExportDump => {
                dialog
                    .add_filter(s.filter_text, &["txt"])
                    .set_file_name(format!(
                        "{}-vidage.txt",
                        self.current_file
                            .as_ref()
                            .and_then(|path| path.file_stem())
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "namsdec".to_owned())
                    ))
            }
            FileDialogKind::ExportDot => dialog
                .add_filter(s.filter_dot, &["dot", "gv"])
                .set_file_name(format!(
                    "{}-graphe.dot",
                    self.current_file
                        .as_ref()
                        .and_then(|path| path.file_stem())
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "namsdec".to_owned())
                )),
            FileDialogKind::ExportKeymap => dialog
                .add_filter(s.filter_theme, &["json"])
                .set_file_name("keymap.json"),
            // Le nom de l'échantillon suivi de `-depaquete` : on se
            // retrouve avec les deux côte à côte dans le même dossier,
            // et le nom dit lequel est lequel. Sans extension imposée —
            // un exécutable Linux n'en a pas.
            FileDialogKind::UnpackUpx => dialog.set_file_name(format!(
                "{}-depaquete",
                self.current_file
                    .as_ref()
                    .and_then(|path| path.file_stem())
                    .map(|nom| nom.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "programme".to_owned())
            )),
            FileDialogKind::OpenFolder | FileDialogKind::SaveAs => dialog,
        }
    }

    /// Relit la réponse du sélecteur ouvert, s'il en a donné une.
    pub(crate) fn poll_file_dialog(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.pending_dialog else {
            return;
        };
        let kind = pending.kind;
        match pending.result.try_recv() {
            Ok(picked) => {
                self.pending_dialog = None;
                if let Some(path) = picked {
                    self.finish_file_dialog(kind, path, ctx);
                }
            }
            // Toujours ouvert. Aucun évènement n'arrive plus sur la
            // fenêtre pendant qu'une boîte modale est affichée : sans ce
            // réveil programmé, la réponse resterait dans le tuyau
            // jusqu'au prochain mouvement de souris.
            Err(TryRecvError::Empty) => ctx.request_repaint_after(DIALOG_POLL_INTERVAL),
            // Le thread s'est terminé sans rien envoyer : plus personne
            // ne répondra, on cesse d'attendre.
            Err(TryRecvError::Disconnected) => self.pending_dialog = None,
        }
    }

    /// Ce qu'on fait du chemin choisi, une fois la boîte refermée.
    pub(crate) fn finish_file_dialog(
        &mut self,
        kind: FileDialogKind,
        path: PathBuf,
        ctx: &egui::Context,
    ) {
        match kind {
            FileDialogKind::OpenFile => self.open_file(path),
            FileDialogKind::OpenFolder => {
                self.explorer.root = Some(path);
                self.show_explorer = true;
                // Même garde que pour la bascule : le clic qui a validé la
                // boîte native ne doit pas refermer aussitôt la fenêtre
                // qu'il vient de faire apparaître.
                self.explorer_just_opened = true;
                self.focus_explorer(true);
            }
            FileDialogKind::SaveAs => self.write_to(&path),
            FileDialogKind::AddTheme => self.install_theme(&path, ctx),
            FileDialogKind::AddExercises => self.install_exercises(&path),
            FileDialogKind::ExportManual => self.export_manual(&path),
            FileDialogKind::ExportReport => self.export_report(&path),
            FileDialogKind::ExportBinary => self.export_binary(&path),
            FileDialogKind::ExportDump => self.export_dump(&path),
            FileDialogKind::ExportDot => self.export_dot(&path),
            FileDialogKind::ExportKeymap => self.export_keymap(&path),
            FileDialogKind::LocateTool => self.locate_tool(path),
            FileDialogKind::UnpackUpx => self.unpack_upx_to(&path),
        }
    }

    /// Retient le binaire choisi pour l'outil qu'on localisait, et
    /// redemande son état : ce qui était « absent » devient trouvable.
    pub(crate) fn locate_tool(&mut self, path: PathBuf) {
        let Some(id) = self.locating_tool.take() else {
            return;
        };
        self.located_tools.insert(id, path);
        self.refresh_tool_availability();
        self.status = Status::Note(|s| s.tools_located);
    }

    /// Écrit le graphe du programme dans un fichier DOT.
    ///
    /// Le presse-papier ne survit pas à la copie suivante, et un graphe
    /// se garde : celui d'avant le correctif, celui d'après, celui qu'on
    /// joint au compte rendu. `dot -Tsvg graphe.dot` en fait une image.
    pub(crate) fn export_dot(&mut self, path: &Path) {
        let s = strings(self.lang);
        let Some(dot) = self.flow_dot() else {
            self.status = Status::Error(s.flow_none.to_owned());
            return;
        };
        match std::fs::write(path, dot) {
            Ok(()) => self.status = Status::Note(|s| s.flow_dot_saved),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.flow_dot_save_failed)),
        }
    }

    /// Écrit le jeu de raccourcis **entier**, à plat.
    ///
    /// `keymap.json` ne garde que les écarts : c'est ce qu'on veut d'un
    /// fichier de réglages — il suit le jeu quand celui-ci gagne une
    /// action. Mais qui veut partir d'une liste complète, la relire, la
    /// donner à quelqu'un ou la ranger dans un dépôt, n'a rien à quoi se
    /// raccrocher : un fichier de deux lignes ne dit pas ce que namsdec
    /// sait faire.
    pub(crate) fn export_keymap(&mut self, path: &Path) {
        let s = strings(self.lang);
        let entier = settings::Keymap {
            preset: self.keymap_preset.id().to_owned(),
            shortcuts: self
                .keybindings
                .map
                .iter()
                .map(|(action, chord)| {
                    (
                        settings::action_id(*action).to_owned(),
                        settings::chord_to_string(chord),
                    )
                })
                .collect(),
        };
        let texte = match serde_json::to_string_pretty(&entier) {
            Ok(texte) => texte,
            Err(e) => {
                self.status = Status::Error(format!("{} : {e}", s.shortcuts_export_failed));
                return;
            }
        };
        match std::fs::write(path, texte) {
            Ok(()) => self.status = Status::Note(|s| s.shortcuts_exported),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.shortcuts_export_failed)),
        }
    }

    /// Écrit le vidage hexadécimal du fichier chargé.
    ///
    /// Le fichier tel qu'il est sur le disque, au format de `xxd` : c'est
    /// ce qui permet de le comparer — à celui qu'on a sous les yeux, à
    /// celui d'un autre outil, ou à la même analyse faite la semaine
    /// dernière. Un `diff` de deux vidages dit en une ligne ce qu'un
    /// binaire a de changé.
    pub(crate) fn export_dump(&mut self, path: &Path) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            self.status = Status::Error(s.binary_none.to_owned());
            return;
        };
        let texte = namsdec_re::dump_module::hex_dump(&state.output.file);
        match std::fs::write(path, texte) {
            Ok(()) => self.status = Status::Note(|s| s.dump_saved),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.dump_save_failed)),
        }
    }

    /// Écrit le manuel entier dans un fichier Markdown.
    ///
    /// Ce que namsdec explique n'a pas à rester dans namsdec : la
    /// licence est libre, le manuel se lit, s'imprime et se distribue
    /// sans lui. La langue est celle de l'interface : on exporte ce
    /// qu'on lit.
    pub(crate) fn export_manual(&mut self, path: &Path) {
        let s = strings(self.lang);
        match std::fs::write(path, namsdec_doc::markdown(self.lang.code())) {
            Ok(()) => self.status = Status::Note(|s| s.manual_exported),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.manual_export_failed)),
        }
    }

    /// Écrit le programme assemblé, tel que `ld` l'a produit.
    ///
    /// L'ELF entier, et exécutable : c'est le même fichier que la ligne
    /// de commande aurait donné, et il se lance dans un vrai terminal,
    /// se lit avec `objdump`, se compare avec ce que namsdec en dit.
    /// Un atelier qui ne laisserait rien sortir apprendrait à se servir
    /// de lui, pas de l'assembleur.
    pub(crate) fn export_binary(&mut self, path: &Path) {
        let s = strings(self.lang);
        let Some(state) = &self.assemble_state else {
            self.status = Status::Error(s.binary_none.to_owned());
            return;
        };
        if state.output.file.is_empty() {
            self.status = Status::Error(s.binary_none.to_owned());
            return;
        }
        match std::fs::write(path, &state.output.file) {
            Ok(()) => {
                // Exécutable : un programme qu'il faut penser à rendre
                // exécutable soi-même n'est pas sorti, il est à moitié
                // sorti.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
                }
                self.status = Status::Note(|s| s.binary_exported);
            }
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.binary_export_failed)),
        }
    }

    /// Écrit le rapport d'analyse, en Markdown ou en JSON.
    ///
    /// L'EXTENSION DÉCIDE. Les deux sortent des mêmes vues : le
    /// Markdown se colle dans un compte rendu, le JSON se donne à un
    /// autre programme. C'est ce que la frontière de P7 a rendu
    /// possible — avant elle, ce que namsdec comprenait n'existait que
    /// peint à l'écran.
    pub(crate) fn export_report(&mut self, path: &Path) {
        let s = strings(self.lang);
        let json = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
        let rapport = if json {
            self.analysis_json()
        } else {
            self.analysis_report()
        };
        let Some(report) = rapport else {
            self.status = Status::Error(s.analysis_none.to_owned());
            return;
        };
        match std::fs::write(path, report) {
            Ok(()) => self.status = Status::Note(|s| s.report_saved),
            Err(e) => self.status = Status::Error(format!("{} : {e}", s.report_save_failed)),
        }
    }

    /// Installe un thème choisi dans le sélecteur.
    ///
    /// Le fichier est d'abord **lu**, donc validé : rien n'est copié dans
    /// le dossier des plugins tant qu'on ne sait pas que c'est un thème
    /// complet. Un fichier illisible ne laisse donc aucune trace, et le
    /// message dit ce qui cloche.
    pub(crate) fn install_theme(&mut self, path: &Path, ctx: &egui::Context) {
        let s = strings(self.lang);
        let note = match namsdec_theme::plugin::read(path) {
            Err(e) => PluginNote {
                ok: false,
                text: errors::plugin(self.lang, &e),
            },
            Ok(theme) => {
                let name = theme.name.to_owned();
                match namsdec_theme::install(theme) {
                    Err(e) => PluginNote {
                        ok: false,
                        text: errors::plugin(self.lang, &e),
                    },
                    Ok(installed) => {
                        // Copié seulement maintenant : le thème est bon,
                        // il doit revenir au lancement suivant.
                        let copied = copy_into_plugins(path);
                        if let Ok(destination) = &copied {
                            // D'où vient le thème : c'est ce qui rendra
                            // son retrait possible.
                            self.theme_files
                                .insert(installed.id.to_owned(), destination.clone());
                        }
                        // Appliqué aussitôt : on installe un thème pour
                        // le voir, pas pour le chercher ensuite dans la
                        // liste.
                        self.theme = installed;
                        self.apply_theme(ctx);
                        match copied {
                            Ok(_) => PluginNote {
                                ok: true,
                                text: format!("{} — {name}", s.plugins_added),
                            },
                            Err(e) => PluginNote {
                                ok: false,
                                text: format!("{} ({e})", s.plugins_added_not_kept),
                            },
                        }
                    }
                }
            }
        };
        self.plugin_note = Some(note);
    }

    /// Charge `path`. Du texte part dans l'éditeur ; tout le reste est
    /// traité comme un binaire brut à désassembler, plutôt que rejeté
    /// comme "pas de l'UTF-8" — c'est un atelier de reverse engineering,
    /// ouvrir un binaire doit y mener quelque part.
    /// L'état de travail, rangé tel quel pour y revenir.
    pub(crate) fn take_tab(&mut self) -> OpenTab {
        // L'ASSEMBLAGE EN COURS NE SUIT PAS L'ONGLET. Il travaille sur
        // un texte qu'on vient de ranger, et sa révision ne vaudra plus
        // rien au retour — `source_mut`, juste en dessous, la fait
        // avancer. Le laisser courir, c'est garder un nasm occupé et
        // une barre qui tourne pour un résultat qui sera jeté ; on
        // l'arrête, et l'on repart d'un Ctrl+Maj+B quand on revient.
        self.cancel_assemble();
        OpenTab {
            path: self.current_file.take(),
            file_mtime: self.file_mtime,
            // Par `source_mut` : le tampon part dans l'onglet et laisse
            // la chaîne vide derrière lui — c'est un changement de texte
            // comme un autre (voir `texte_a_changé`).
            source: std::mem::take(self.source_mut()),
            saved_source: std::mem::take(&mut self.saved_source),
            cursor_byte: self.cursor_byte,
            cursor_pos: self.cursor_pos,
            breakpoint_lines: std::mem::take(&mut self.breakpoint_lines),
            breakpoint_conditions: std::mem::take(&mut self.breakpoint_conditions),
            assemble_state: self.assemble_state.take(),
            emulator: self.emulator.take(),
            diagnostics: std::mem::take(&mut self.diagnostics),
            current_exercise: self.current_exercise.take(),
            program_reads: self.program_reads,
            program_writes: self.program_writes,
            untitled_no: self.untitled_no,
        }
    }

    /// Et l'inverse : l'onglet redevient le fichier de travail.
    pub(crate) fn restore_tab(&mut self, tab: OpenTab) {
        self.untitled_no = tab.untitled_no;
        self.current_file = tab.path;
        self.file_mtime = tab.file_mtime;
        self.set_source(tab.source);
        self.saved_source = tab.saved_source;
        self.cursor_byte = tab.cursor_byte;
        self.cursor_pos = tab.cursor_pos;
        self.breakpoint_lines = tab.breakpoint_lines;
        self.breakpoint_conditions = tab.breakpoint_conditions;
        self.assemble_state = tab.assemble_state;
        self.emulator = tab.emulator;
        self.diagnostics = tab.diagnostics;
        self.current_exercise = tab.current_exercise;
        self.program_reads = tab.program_reads;
        self.program_writes = tab.program_writes;
        // Ce qui n'appartient qu'au moment présent, et non au fichier :
        // une course en train de tourner, la photographie du dernier
        // pas, le curseur qu'on voulait poser ailleurs.
        self.running = false;
        self.last_step = None;
        // Un autre onglet, un autre programme : la réponse découverte
        // ne vaut que pour celui qu'on quitte.
        self.prevision_révélée = None;
        self.error_advice = None;
        self.regs_before = None;
        self.flags_before = None;
        self.mem_before = None;
        self.status = Status::Idle;
        // L'éditeur doit remettre le curseur là où il était : il ne le
        // fait qu'à la demande, sinon il le laisse où la souris l'a
        // posé.
        self.goto_line = Some(self.cursor_pos.0.max(1) as u32);
        // Le fichier a pu changer pendant qu'on travaillait ailleurs.
        self.check_disk_change();
    }

    /// Le fichier ouvert a-t-il changé sur le disque ?
    ///
    /// Non enregistré de notre côté, on recharge : c'est ce que le
    /// fichier dit maintenant, et rien de nôtre n'est perdu. Modifié des
    /// deux côtés, on ne choisit pas à la place de l'utilisateur — on le
    /// dit, et il tranche.
    pub(crate) fn check_disk_change(&mut self) {
        let (Some(path), Some(connue)) = (self.current_file.clone(), self.file_mtime) else {
            return;
        };
        let Some(sur_disque) = file_mtime(&path) else {
            // Disparu : ce n'est pas à nous de l'effacer de l'écran, et
            // Ctrl+S le réécrira très bien.
            return;
        };
        if sur_disque == connue {
            return;
        }
        if self.buffer_modified() {
            self.disk_changed = true;
            return;
        }
        let Ok(texte) = std::fs::read_to_string(&path) else {
            self.disk_changed = true;
            return;
        };
        self.set_source(texte.clone());
        self.saved_source = texte;
        self.file_mtime = Some(sur_disque);
        self.disk_changed = false;
        // Ce qui était assemblé décrit l'ancienne version : le garder
        // ferait pointer la gouttière sur des lignes qui ont bougé.
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.status = Status::Note(|s| s.disk_reloaded);
    }

    /// Recharge depuis le disque, en abandonnant ce qu'on avait tapé.
    pub(crate) fn reload_from_disk(&mut self) {
        if self.current_file.is_none() {
            return;
        }
        // On fait comme si le tampon était à jour : `check_disk_change`
        // relit alors le fichier et remplace tout.
        self.saved_source = self.source.clone();
        self.check_disk_change();
    }

    /// Le nom de ce qu'on travaille : le fichier, l'exercice, le
    /// binaire ouvert, ou « sans titre ».
    ///
    /// UNE SEULE RÉPONSE POUR TOUTE L'APPLICATION. Quatre endroits
    /// nommaient le tampon chacun de leur côté — la barre de titre, la
    /// barre de l'Établi, la barre d'état, la mention « décompilé
    /// depuis » — et tous les quatre lisaient `current_file` seul. Or
    /// `current_file` reste VIDE pour un binaire ouvert, exprès (un
    /// Ctrl+S distrait n'a pas à écraser l'exécutable par son propre
    /// désassemblage) : l'onglet, lui, savait retomber sur le nom du
    /// binaire, mais la barre de titre affichait « sans titre » au-dessus
    /// du désassemblage d'un programme qu'on venait d'ouvrir. C'est le
    /// défaut relevé à l'écran le 11 novembre 2026.
    pub(crate) fn nom_du_tampon(&self) -> String {
        // Le nom du binaire vient de `from_binary`, qui appartient à
        // l'onglet ; il est vide pour des octets bruts — un fichier sans
        // en-tête ELF —, et c'est alors le chemin qui le donne.
        let nom_du_binaire = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
            .map(|binaire| binaire.name.clone())
            .map(|nom| {
                if nom.is_empty() {
                    self.chemin_du_tampon()
                        .and_then(|chemin| chemin.file_name())
                        .map(|nom| nom.to_string_lossy().into_owned())
                        .unwrap_or_default()
                } else {
                    nom
                }
            });
        tab_label(
            self.current_file.as_deref(),
            self.current_exercise.as_deref(),
            nom_du_binaire.as_deref(),
            strings(self.lang),
            &self.exercise_packs,
            self.lang,
            self.untitled_no,
        )
    }

    /// Le chemin complet de ce que montre le tampon, quand il en a un.
    ///
    /// C'est ce que les infobulles de la barre de titre disent : un nom
    /// de fichier ne distingue pas deux `crackme` rangés dans deux
    /// dossiers, et l'on ne sait plus lequel on a chargé.
    pub(crate) fn chemin_du_tampon(&self) -> Option<&Path> {
        if let Some(chemin) = self.current_file.as_deref() {
            return Some(chemin);
        }
        // UN BINAIRE, ET CELUI DE CET ONGLET. `binary_path` appartient à
        // l'application et non à l'onglet — il sert à rouvrir le dernier
        // binaire au lancement suivant —, si bien qu'un tampon neuf
        // posé à côté d'un binaire aurait affiché le chemin du voisin.
        // On ne le rend donc que si l'onglet courant vient bien d'un
        // binaire, et que ce chemin porte son nom.
        let binaire = self.assemble_state.as_ref()?.from_binary.as_ref()?;
        let chemin = self.binary_path.as_deref()?;
        let porte_son_nom = binaire.name.is_empty()
            || chemin
                .file_name()
                .is_some_and(|nom| nom.to_string_lossy() == binaire.name);
        porte_son_nom.then_some(chemin)
    }

    /// Tous les onglets dans l'ordre, l'actif à sa place.
    ///
    /// L'actif n'est pas rangé dans la liste — c'est l'état de travail
    /// lui-même —, mais la barre d'onglets, elle, doit les montrer tous
    /// à leur rang.
    pub(crate) fn tab_labels(&self) -> Vec<(String, bool)> {
        let s = strings(self.lang);
        let actif = (
            self.nom_du_tampon(),
            // Le relevé de l'image : la barre d'onglets est dessinée à
            // chaque image, et chaque onglet comparait son texte entier
            // (voir `modifié_vu`). Le repli sur la comparaison vive sert
            // aux appels qui ne viennent pas d'une image — un test, par
            // exemple.
            self.modifié_vu,
        );
        let mut out: Vec<(String, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(rang, tab)| {
                (
                    tab.label(s, &self.exercise_packs, self.lang),
                    self.tabs_modifiés_vus
                        .get(rang)
                        .copied()
                        .unwrap_or_else(|| tab.modifié()),
                )
            })
            .collect();
        out.insert(self.active_tab_index.min(out.len()), actif);
        out
    }

    /// Passe à l'onglet de rang `index`, en rangeant celui qu'on quitte.
    pub(crate) fn go_to_tab(&mut self, index: usize) {
        if index == self.active_tab_index || index > self.tabs.len() {
            return;
        }
        // On retire d'abord la cible, puis on range l'actif à sa place :
        // l'ordre des onglets ne change pas sous les doigts.
        let cible = if index < self.active_tab_index {
            self.tabs.remove(index)
        } else {
            self.tabs.remove(index - 1)
        };
        let courant = self.take_tab();
        // OÙ RANGER CELUI QU'ON QUITTE — et c'est là que l'ordre se
        // jouait.
        //
        // `tabs` ne porte que les onglets qu'on ne regarde pas : le rang
        // d'affichage `r` y est à l'indice `r` s'il précède l'actif, et
        // `r - 1` s'il le suit. Celui qu'on quitte garde son rang
        // d'affichage ; son indice, lui, dépend du côté où va le
        // nouveau. En allant vers la GAUCHE, l'actif sortant passe
        // derrière le nouveau, et son indice descend d'un cran.
        //
        // Il était rangé à `active_tab_index` dans tous les cas. Relevé
        // le 21 novembre 2026 : quatre fichiers, aller de f3 à f1 puis à
        // f0 donnait f0, f2, f1, f3 — deux onglets avaient échangé leur
        // place sous les doigts. Invisible avec deux onglets (le
        // `.min()` rattrapait tout), et c'est pourquoi cela a tenu si
        // longtemps.
        let place = if index < self.active_tab_index {
            self.active_tab_index - 1
        } else {
            self.active_tab_index
        };
        self.tabs.insert(place.min(self.tabs.len()), courant);
        self.active_tab_index = index;
        self.restore_tab(cible);
    }

    /// D'un onglet au suivant, en bouclant.
    ///
    /// Boucler plutôt que buter : on parcourt trois fichiers avec la
    /// même touche, sans avoir à se demander où l'on est dans la file.
    pub(crate) fn cycle_tab(&mut self, pas: i32) {
        let total = self.tabs.len() + 1;
        if total < 2 {
            return;
        }
        let courant = self.active_tab_index as i32;
        let cible = (courant + pas).rem_euclid(total as i32) as usize;
        self.go_to_tab(cible);
    }

    /// Ferme l'onglet de rang `index`.
    ///
    /// Le dernier ne se ferme pas : une fenêtre d'éditeur sans éditeur
    /// n'a rien à montrer. Fermer celui-là revient à repartir d'un
    /// programme neuf, ce que Fichier ▸ Nouveau fait déjà.
    pub(crate) fn close_tab(&mut self, index: usize) {
        // Un tampon déjà vide et sans fichier n'a rien à fermer.
        if self.tabs.is_empty() && self.current_file.is_none() && self.source.is_empty() {
            return;
        }
        // Un onglet dont le texte n'est pas enregistré ne part pas sur
        // un clic : c'est le seul endroit d'où un travail peut
        // disparaître sans qu'on l'ait voulu.
        if self.tab_modified(index) {
            // La question porte sur l'onglet visé : on s'y installe
            // d'abord, pour que « Enregistrer » enregistre le bon
            // fichier et qu'on voie ce dont on parle.
            self.go_to_tab(index);
            self.pending = Some(Pending::CloseTab(self.active_tab_index));
            return;
        }
        self.close_tab_now(index);
    }

    /// Ce que l'onglet de rang `index` a de non enregistré.
    pub(crate) fn tab_modified(&self, index: usize) -> bool {
        if index == self.active_tab_index {
            return self.buffer_modified();
        }
        let rang = if index < self.active_tab_index {
            index
        } else {
            index.wrapping_sub(1)
        };
        self.tabs.get(rang).is_some_and(OpenTab::modifié)
    }

    /// Ferme plusieurs onglets d'un coup — les autres, tous, ou ceux
    /// d'un côté.
    ///
    /// Un par un et du dernier au premier : fermer le rang 5 ne déplace
    /// pas les rangs 0 à 4, si bien que la liste reste juste du début à
    /// la fin. Ceux qui ne sont pas enregistrés arrêtent le défilé le
    /// temps d'une question ; les autres partent sans rien demander.
    pub(crate) fn close_tabs(&mut self, rangs: &[usize]) {
        let mut file: Vec<usize> = rangs.to_vec();
        file.sort_unstable();
        file.dedup();
        // Rangée en ordre croissant, et dépilée par la fin : le plus
        // grand rang part le premier.
        self.fermetures_en_file = file;
        self.poursuivre_fermetures();
    }

    /// Reprend la file de fermeture là où une question l'avait arrêtée.
    pub(crate) fn poursuivre_fermetures(&mut self) {
        while let Some(index) = self.fermetures_en_file.pop() {
            if self.tab_modified(index) {
                // La question porte sur celui-là : on l'ouvre, on la
                // pose, et le reste de la file attend la réponse. Lui
                // est déjà sorti de la file — c'est `close_tab` qui le
                // fermera, une fois qu'on aura répondu.
                self.close_tab(index);
                return;
            }
            self.close_tab_now(index);
        }
    }

    /// Abandonne ce qui restait à fermer : « Annuler » annule la
    /// fermeture, pas seulement celle de l'onglet affiché.
    pub(crate) fn abandonner_fermetures(&mut self) {
        self.fermetures_en_file.clear();
    }

    /// Les rangs à fermer pour une entrée du menu d'un onglet.
    pub(crate) fn rangs_a_fermer(&self, visé: usize, quoi: FermerLesquels) -> Vec<usize> {
        let total = self.tabs.len() + 1;
        (0..total)
            .filter(|&rang| match quoi {
                FermerLesquels::Autres => rang != visé,
                FermerLesquels::Tous => true,
                FermerLesquels::Gauche => rang < visé,
                FermerLesquels::Droite => rang > visé,
            })
            .collect()
    }

    /// Ferme sans plus rien demander.
    pub(crate) fn close_tab_now(&mut self, index: usize) {
        // CE QU'UNE ÉTAPE AVAIT ÉCRIT NE SE FERME PAS AVEC SON ONGLET.
        //
        // C'était le seul endroit où le parcours perdait du travail :
        // l'onglet emportait le programme, et revenir à l'étape rendait
        // le code de départ. Le brouillon est rangé en mémoire avant
        // que le tampon parte, et relu quand on redemande l'étape.
        self.ranger_brouillon(index);
        if self.tabs.is_empty() {
            // Le DERNIER fichier se ferme aussi.
            //
            // Il ne se fermait pas : `tabs` ne porte que les fichiers
            // qu'on ne regarde PAS, si bien qu'avec un seul ouvert elle
            // est vide et que « Fermer le fichier » ne faisait rien —
            // l'entrée du menu disparaissait même, ce qui donnait à
            // croire qu'un fichier seul ne se ferme pas.
            //
            // Fermer, ici, c'est revenir à un tampon vierge : plus de
            // chemin, plus de programme assemblé, plus de points
            // d'arrêt. Vide et non le programme d'accueil : on a demandé
            // à fermer, pas à ouvrir autre chose.
            self.source_mut().clear();
            self.saved_source.clear();
            self.current_file = None;
            self.file_mtime = None;
            self.assemble_state = None;
            self.emulator = None;
            self.diagnostics.clear();
            self.breakpoint_lines.clear();
            self.status = Status::Idle;
            // ET L'ACCUEIL PREND LA PLACE DE L'ÉDITEUR.
            //
            // Demandé à l'écran, capture à l'appui : « quand je quitte,
            // j'aimerais que tout soit fermé, et une fenêtre comme la
            // suivante apparaisse ». Un éditeur vide qui ne dit rien est
            // le pire état d'une application : elle marche, elle n'a
            // rien fait de mal, et elle n'offre rien.
            self.montrer_accueil = true;
            return;
        }
        if index == self.active_tab_index {
            // On s'installe sur le voisin, puis on ferme celui qu'on
            // vient de quitter.
            let voisin = if index == 0 { 1 } else { index - 1 };
            self.go_to_tab(voisin);
            let rang = if index > self.active_tab_index {
                index - 1
            } else {
                index
            };
            self.tabs.remove(rang);
            if self.active_tab_index > index {
                self.active_tab_index -= 1;
            }
            return;
        }
        let rang = if index < self.active_tab_index {
            index
        } else {
            index - 1
        };
        self.tabs.remove(rang);
        if index < self.active_tab_index {
            self.active_tab_index -= 1;
        }
    }

    /// Ouvre un fichier dans un nouvel onglet, ou revient à celui qui le
    /// montre déjà.
    pub(crate) fn open_in_new_tab(&mut self, path: PathBuf) {
        if let Some(rang) = self.tab_index_of(&path) {
            // Déjà ouvert : on y retourne. Et il remonte quand même
            // parmi les récents — c'est bien le fichier qu'on vient de
            // demander, qu'il ait fallu le lire ou non.
            self.note_recent(&path);
            self.go_to_tab(rang);
            return;
        }
        // Un tampon vierge — pas de fichier, rien de tapé — n'a rien à
        // garder : le fichier demandé prend sa place au lieu d'ouvrir un
        // onglet « sans titre » que personne n'a demandé.
        if self.current_file.is_none() && !self.buffer_modified() {
            self.open_file_now(path);
            return;
        }
        // Sinon l'onglet courant est rangé tel quel, et le nouveau prend
        // sa place juste après : on ouvre un fichier à côté de celui
        // qu'on lisait, pas au bout de la file.
        let courant = self.take_tab();
        let insertion = self.active_tab_index.min(self.tabs.len());
        self.tabs.insert(insertion, courant);
        self.active_tab_index = insertion + 1;
        self.open_file_now(path);
    }

    /// Le rang de l'onglet qui travaille cet exercice, s'il y en a un.
    pub(crate) fn exercise_tab_index(&self, id: &str) -> Option<usize> {
        if self.current_exercise.as_deref() == Some(id) {
            return Some(self.active_tab_index);
        }
        let position = self
            .tabs
            .iter()
            .position(|tab| tab.current_exercise.as_deref() == Some(id))?;
        Some(if position >= self.active_tab_index {
            position + 1
        } else {
            position
        })
    }

    /// Le rang de l'onglet qui montre ce fichier, s'il y en a un.
    pub(crate) fn tab_index_of(&self, path: &Path) -> Option<usize> {
        if self.current_file.as_deref() == Some(path) {
            return Some(self.active_tab_index);
        }
        let position = self
            .tabs
            .iter()
            .position(|tab| tab.path.as_deref() == Some(path))?;
        Some(if position >= self.active_tab_index {
            position + 1
        } else {
            position
        })
    }

    /// Repart d'un programme neuf.
    pub(crate) fn do_new(&mut self) {
        self.guard(Pending::New);
    }

    /// Un programme neuf, dans son propre onglet.
    ///
    /// Le fichier en cours reste ouvert à côté : « nouveau » veut dire
    /// « un de plus », et non « à la place de ».
    pub(crate) fn new_program(&mut self) {
        // TOUJOURS un onglet de plus, même quand le tampon courant est
        // vierge et sans fichier.
        //
        // Il ne s'en créait pas dans ce cas : « un tampon vierge n'a
        // rien à garder ». C'est vrai de son contenu et faux de ce qu'on
        // vient de demander — on a cliqué « Nouveau programme » et il ne
        // se passait RIEN de visible, le tampon d'accueil étant remplacé
        // par un autre qui lui ressemble. Un geste sans effet visible
        // est un geste cassé, quelle que soit la logique derrière.
        // SAUF DEPUIS L'ACCUEIL, où il n'y a rien à garder À CÔTÉ.
        //
        // On vient de fermer, l'écran dit « plus rien n'est ouvert », et
        // empiler le tampon fermé comme un onglet de plus rouvrirait ce
        // qu'on vient de refermer : « sans titre » reviendrait seul dans
        // la barre. Ici le geste se voit de reste — l'accueil s'efface
        // et le programme neuf prend sa place.
        if !self.montrer_accueil {
            let courant = self.take_tab();
            let insertion = self.active_tab_index.min(self.tabs.len());
            self.tabs.insert(insertion, courant);
            self.active_tab_index = insertion + 1;
        }
        self.untitled_no = self.next_untitled;
        self.next_untitled += 1;
        self.montrer_accueil = false;
        self.set_source(NEW_PROGRAM.to_owned());
        self.saved_source = NEW_PROGRAM.to_owned();
        self.current_file = None;
        self.assemble_state = None;
        self.emulator = None;
        self.diagnostics.clear();
        self.breakpoint_lines.clear();
        self.status = Status::Idle;
    }

    /// Ouvre un fichier dans son propre onglet.
    ///
    /// Plus de question à poser avant : le fichier en cours n'est pas
    /// remplacé, il attend dans son onglet avec ses modifications, son
    /// curseur et ses points d'arrêt.
    pub(crate) fn open_file(&mut self, path: PathBuf) {
        // Un fichier s'ouvre : il n'y a plus d'accueil à montrer.
        self.montrer_accueil = false;
        self.open_in_new_tab(path);
    }

    /// Y a-t-il quelque chose à perdre ?
    ///
    /// La comparaison se fait sur le texte, pas sur un drapeau : taper
    /// un caractère puis l'effacer ne met rien en danger.
    pub(crate) fn buffer_modified(&self) -> bool {
        // LE PARCOURS NE DEMANDE RIEN : une étape qu'on travaille n'a
        // pas de fichier, et son programme est gardé tout seul dans le
        // dossier des résolus. Lui poser la question « Modifications
        // non enregistrées » revenait à ouvrir un sélecteur de fichiers
        // au milieu d'une leçon, pour écrire où ? — voir
        // `exercises::resolus_dir`.
        if self.current_file.is_none() && self.current_exercise.is_some() {
            return false;
        }
        self.source != self.saved_source
    }

    /// Combien de fichiers ouverts ont du texte non enregistré.
    pub(crate) fn modified_tabs(&self) -> usize {
        usize::from(self.buffer_modified()) + self.tabs.iter().filter(|t| t.modifié()).count()
    }

    /// Le rang du premier onglet — autre que l'actif — qui a du texte
    /// non enregistré.
    pub(crate) fn first_modified_tab(&self) -> Option<usize> {
        (0..=self.tabs.len())
            .find(|rang| *rang != self.active_tab_index && self.tab_modified(*rang))
    }

    /// Y a-t-il quelque chose à perdre **où que ce soit** ?
    ///
    /// Depuis qu'on peut ouvrir plusieurs fichiers, quitter ne met pas en
    /// danger le seul tampon visible : un onglet laissé de côté avec des
    /// modifications compte autant.
    pub(crate) fn anything_modified(&self) -> bool {
        self.buffer_modified() || self.tabs.iter().any(OpenTab::modifié)
    }

    /// Fait l'action, ou demande d'abord.
    pub(crate) fn guard(&mut self, action: Pending) {
        if self.anything_modified() {
            self.pending = Some(action);
        } else {
            self.perform(action);
        }
    }

    /// Note un fichier comme récemment ouvert.
    pub(crate) fn note_recent(&mut self, path: &Path) {
        if self.restoring_session {
            return;
        }
        // Le même fichier ne s'y met pas deux fois : il remonte.
        self.recent_files.retain(|autre| autre != path);
        self.recent_files.insert(0, path.to_path_buf());
        self.recent_files.truncate(Self::RECENT_LIMIT);
    }

    pub(crate) fn open_file_now(&mut self, path: PathBuf) {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.status = Status::Error(if self.lang == Lang::Fr {
                    format!("impossible d'ouvrir le fichier : {e}")
                } else {
                    format!("could not open file: {e}")
                });
                return;
            }
        };

        // La signature DÉCIDE, et l'UTF-8 ne décide plus rien.
        //
        // L'ouverture choisissait « source » dès que tous les octets
        // formaient un UTF-8 valide, ce qui ne démontre rien : les
        // octets nuls sont valides en UTF-8, et le petit ELF x86-64
        // qu'on écrit pour éprouver le lecteur satisfait justement ce
        // critère. Un exécutable partait donc dans l'éditeur de source.
        let texte = match sniff(&bytes) {
            FileKind::Binary => None,
            FileKind::Text => String::from_utf8(bytes.clone()).ok(),
        };

        match texte.ok_or(bytes) {
            Ok(text) => {
                self.saved_source = text.clone();
                self.set_source(text);
                self.note_recent(&path);
                self.file_mtime = file_mtime(&path);
                self.disk_changed = false;
                self.current_file = Some(path);
                // On travaille une source maintenant : le binaire de la
                // séance précédente n'a plus à revenir au lancement
                // suivant.
                self.binary_path = None;
                self.assemble_state = None;
                self.emulator = None;
                self.diagnostics.clear();
                self.status = Status::Idle;
            }
            Err(octets) => {
                let nom = path
                    .file_name()
                    .map(|nom| nom.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "programme".to_owned());
                self.note_recent(&path);
                self.load_binary(octets, &nom);
                // Retenu pour la séance suivante. Pas dans
                // `current_file`, qui reste vide exprès pour un binaire :
                // un Ctrl+S distrait n'a pas à écraser l'exécutable par
                // son propre désassemblage.
                self.binary_path = Some(path);
            }
        }
    }

    /// Ouvre un binaire brut : désassemblage direct à `BASE_ADDR`, sans
    /// passer par nasm. Le listing obtenu remplit l'éditeur, une
    /// instruction par ligne — ce qui donne aussi la correspondance
    /// ligne <-> adresse dont dépendent la gouttière, le suivi de RIP et
    /// les points d'arrêt.
    ///
    /// `current_file` est délibérément remis à zéro : le tampon affiché
    /// est reconstruit, pas le contenu du fichier, et un Ctrl+S distrait
    /// ne doit pas écraser le binaire par son propre désassemblage.
    /// Ouvre un ELF64 : ses sections vont à *leur* adresse, son point
    /// d'entrée est celui qu'il déclare, et seul le code est désassemblé.
    ///
    /// Sans cela, un vrai binaire commençait par seize octets d'en-tête
    /// lus comme des instructions : le listing ouvrait sur du charabia, et
    /// l'exécution démarrait dessus.
    ///
    /// Rend `false` si ce n'en est pas un — à l'appelant de retomber sur
    /// le désassemblage brut.
    pub(crate) fn load_elf(&mut self, bytes: &[u8], name: &str) -> bool {
        let Some(elf) = namsdec_asm::elf::parse(bytes) else {
            return false;
        };
        // L'ARCHITECTURE, avant de décoder quoi que ce soit.
        //
        // Le lecteur vérifiait ELF64 et le petit-boutisme ; un en-tête
        // déclarant AArch64 passait, son image se construisait, et le
        // reste du parcours la désassemblait en x86-64 — soixante mille
        // lignes qui n'ont aucun rapport avec le programme. On refuse
        // le chemin ELF en NOMMANT la machine : les octets restent
        // ouverts pour les panneaux RE et l'éditeur binaire, mais plus
        // rien ne prétend les comprendre.
        if !elf.machine.supported() {
            let machine = elf.machine.name();
            self.status = Status::Error(if self.lang == Lang::Fr {
                format!(
                    "{name} est un ELF {machine} : namsdec ne lit que x86-64. Les octets restent consultables, le listing ne veut rien dire."
                )
            } else {
                format!(
                    "{name} is a {machine} ELF: namsdec only reads x86-64. The bytes stay inspectable, the listing means nothing."
                )
            });
            return false;
        }
        // « Prochaine instruction » se replie sur un binaire, et pas sur
        // un source qu'on assemble.
        //
        // Ce n'est pas une préférence de goût : une instruction de
        // binaire porte deux lignes de plus — son adresse absolue et son
        // pseudo-code — et sur une fenêtre de 1280×820 avec la console
        // ouverte, le bandeau n'avait plus la place de finir. La coupe
        // ne se réglait pas par un partage : elle se déplaçait.
        //
        // Replié, il reste le titre et sa flèche, que l'on rouvre d'un
        // clic — et qui se rouvre de lui-même devant un appel, un saut
        // conditionnel ou un appel système, là où il y a quelque chose à
        // comprendre. Devant un binaire, ce qu'on lit d'abord est le
        // listing et les registres ; devant son propre programme, c'est
        // l'instruction qui vient.
        self.next_instruction_open = false;
        // Les notes de ce binaire, s'il en a : on les retrouve avant de
        // dessiner quoi que ce soit, pour que le premier coup d'oeil
        // porte déjà les noms qu'on avait trouvés.
        self.load_notes_for(bytes);
        // Un fichier de réadressage : aucune section n'a d'adresse, et
        // il n'y a pas de point d'entrée. `ld` n'est pas encore passé.
        let relocatable = elf.entry == 0 && elf.sections.iter().all(|section| section.addr == 0);
        // Le PLAN d'abord, l'image ensuite : `elf.image()` allouait
        // l'étendue décrite par les en-têtes, et le plafond n'était
        // contrôlé qu'après, sur un `Vec` déjà en mémoire. Un fichier
        // de cent vingt octets pouvait ainsi réclamer un demi-
        // gigaoctet avant d'être refusé.
        let Some(plan) = elf.image_plan() else {
            return false;
        };
        // La fenêtre s'adapte au programme : ce qui est refusé, c'est
        // ce qui dépasse ce qu'une machine accepte d'allouer.
        if memory_window(plan.addr, plan.len).1 > MEM_MAX {
            self.status = Status::Error(if self.lang == Lang::Fr {
                format!(
                    "{name} décrit {} Mio de mémoire : au-delà des {} Mio que namsdec simule.",
                    plan.len / (1024 * 1024),
                    MEM_MAX / (1024 * 1024)
                )
            } else {
                format!(
                    "{name} describes {} MiB of memory: beyond the {} MiB namsdec simulates.",
                    plan.len / (1024 * 1024),
                    MEM_MAX / (1024 * 1024)
                )
            });
            return false;
        }
        let (load_addr, image) = match elf.image(plan.len) {
            Ok(fait) => fait,
            Err(pourquoi) => {
                self.status = Status::Error(format!("{name} : {pourquoi}"));
                return false;
            }
        };

        // Le listing ne montre que les sections exécutables : le reste est
        // de la donnée, et la donnée désassemblée ne veut rien dire.
        // Par sections quand le binaire en a, par segments sinon : un
        // crackme dépouillé de sa table de sections n'en a aucune, et
        // namsdec le déclarait illisible alors qu'il s'exécute très bien.
        let mut disasm = Vec::new();
        for (adresse, octets) in elf.executable_parts() {
            disasm.extend(namsdec_disasm::disassemble(octets, adresse));
        }
        disasm.sort_by_key(|instruction| instruction.address);

        // Les entrées de la table de liaison rejoignent les étiquettes :
        // lire un binaire, c'est d'abord savoir ce qu'il appelle.
        let mut symbols = elf.symbols.clone();
        symbols.extend(plt_labels(bytes, &elf.symbols, Some));
        symbols.sort_by_key(|symbol| symbol.addr);
        // Les noms qu'on avait donnés à ce binaire — ils viennent d'être
        // relus — passent devant ceux du fichier : rouvrir un crackme
        // doit le rouvrir tel qu'on l'avait compris, non tel qu'il
        // était la première fois.
        // Ce que les branchements calculés visent, relevé une fois :
        // sur un gros binaire, c'est un parcours du désassemblage, et
        // il n'a pas à se refaire à chaque image.
        let nommés = symbols_with_names(&symbols, &self.notes);
        let indirects = indirect_notes(&disasm, bytes, &nommés, self.lang);
        let (listing, line_to_addr) = annotated_listing(&disasm, &nommés, &indirects);
        let addr_to_line = line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
        let loaded = image.len();

        self.saved_source = listing.clone();
        self.set_source(listing);
        self.current_file = None;
        self.breakpoint_lines.clear();
        self.diagnostics.clear();
        self.assemble_state = Some(AssembleState {
            output: namsdec_asm::AssembleOutput {
                load_addr,
                entry: elf.entry,
                sections: elf
                    .sections
                    .iter()
                    .map(|section| namsdec_asm::Section {
                        name: section.name.clone(),
                        addr: section.addr,
                        size: section.size,
                        executable: section.executable,
                        writable: section.writable,
                    })
                    .collect(),
                bytes: image,
                diagnostics: Vec::new(),
                line_to_addr,
                // Les étiquettes de l'ELF ouvert : elles nomment les
                // adresses aussi bien ici que pour un programme qu'on
                // vient d'assembler soi-même.
                symbols,
                // Le fichier entier : l'analyse des protections se fait
                // sur ses en-têtes, pas sur l'image chargée.
                file: bytes.to_vec(),
            },
            disasm,
            addr_to_line,
            // Un ELF ouvert : on retient ce qu'il réclame au chargement,
            // car c'est ce qui décide s'il peut tourner ici.
            // Ce que le fichier réclame au chargement se lit dans sa
            // table dynamique, que seul le scan complet parcourt.
            from_binary: Some(match namsdec_re::elf_scan::scan(bytes) {
                Some(scan) => LoadedBinary {
                    needed: scan.needed,
                    interpreter: scan.interpreter,
                    name: name.to_owned(),
                    ..Default::default()
                },
                None => LoadedBinary {
                    name: name.to_owned(),
                    ..Default::default()
                },
            }),
            octets_du_code: Default::default(),
        });
        self.note_program_io();
        // Un objet non lié — un `.o` — n'a ni point d'entrée ni adresse
        // de section : `ld` ne lui en a pas encore donné. Il se lit et
        // s'analyse très bien, mais l'exécuter n'a pas de sens, et le
        // chargeur s'en plaignait jusqu'ici en parlant de mémoire, ce
        // qui n'expliquait rien.
        if relocatable {
            self.emulator = None;
            self.status = Status::Error(strings(self.lang).binary_not_linked.to_owned());
            return true;
        }
        self.rebuild_emulator();
        if !matches!(self.status, Status::Error(_)) {
            self.status = Status::BinaryLoaded {
                bytes: loaded,
                truncated: false,
            };
        }
        // Un binaire lié dynamiquement se lit et s'analyse très bien ;
        // il ne s'exécute pas ici, faute des fonctions qu'il emprunte.
        // Le dire maintenant vaut mieux que de le laisser découvrir sur
        // un « rien n'est projeté à cette adresse » au premier appel,
        // qui parle de mémoire là où le problème est ailleurs.
        if let Some(chargé) = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
            && (!chargé.needed.is_empty() || chargé.interpreter.is_some())
        {
            let s = strings(self.lang);
            self.status = Status::Note(|s| s.binary_dynamic);
            self.diagnostics.push(namsdec_asm::Diagnostic {
                line: 0,
                severity: namsdec_asm::Severity::Warning,
                // Sans motif, celui-ci : sa phrase est bâtie autour d'une
                // chaîne de l'interface (`binary_dynamic`), et la
                // recopier dans namsdec-asm ferait deux sources pour un
                // même texte. Il garde donc la langue où il est né — ce
                // qui se voit, mais moins qu'une divergence entre deux
                // copies du même message.
                kind: None,
                message: format!(
                    "{} {}",
                    s.binary_dynamic,
                    if chargé.needed.is_empty() {
                        chargé.interpreter.clone().unwrap_or_default()
                    } else {
                        chargé.needed.join(", ")
                    }
                ),
            });
        }
        self.note_load_report();
        // Le vidage mémoire s'ouvre là où l'exécution commence : la
        // fenêtre historique (`0x400000`) ne veut plus rien dire pour un
        // binaire chargé à ses propres adresses, et la première chose
        // qu'on regarde est ce que le programme va exécuter.
        self.mem_addr = elf.entry;
        self.mem_addr_input = format!("{:#x}", elf.entry);
        true
    }

    /// Dit ce que le chargeur a fait, et ce que le programme emprunte.
    ///
    /// Un chargement complet n'appelle pas de commentaire : le programme
    /// tourne, et la carte mémoire montre le reste. Un programme qui
    /// emprunte des fonctions, si — car la question qui vient ensuite est
    /// « est-ce que ça va tourner ? », et la réponse tient en un compte :
    /// combien de ces fonctions namsdec sait faire, et lesquelles il ne
    /// sait pas.
    pub(crate) fn note_load_report(&mut self) {
        let Some(chargé) = self
            .assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
        else {
            return;
        };
        let rapport = chargé.load.clone();
        if rapport.external.is_empty() {
            return;
        }
        let s = strings(self.lang);
        let (fournies, absentes): (Vec<String>, Vec<String>) = rapport
            .external
            .iter()
            .cloned()
            .partition(|nom| namsdec_emu::provides(nom));
        // Les manquantes d'abord, et toutes si elles tiennent : ce sont
        // elles qui décident où l'exécution s'arrêtera.
        let montrées = absentes
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let suite = if absentes.len() > 8 {
            format!(", … {} {}", absentes.len() - 8, s.loaded_others)
        } else {
            String::new()
        };
        let emprunts = format!(
            "{} {} {} {} {}",
            s.loaded_provided_before,
            fournies.len(),
            s.loaded_provided_of,
            rapport.external.len(),
            s.loaded_provided_after,
        );
        let message = format!(
            "{} {} {}, {} {}, {} {}. {emprunts}{}",
            s.loaded_summary,
            rapport.segments,
            s.loaded_segments,
            rapport.relocations,
            s.loaded_relocations,
            rapport.tls,
            s.loaded_tls,
            if absentes.is_empty() {
                String::new()
            } else {
                format!(" {} {montrées}{suite}", s.loaded_missing)
            },
        );
        self.diagnostics.push(namsdec_asm::Diagnostic {
            line: 0,
            severity: namsdec_asm::Severity::Warning,
            message,
            // Même cas que le précédent : la phrase se compose de
            // chaînes de l'interface, qu'on ne recopie pas ailleurs.
            kind: None,
        });
    }

    pub(crate) fn load_binary(&mut self, mut bytes: Vec<u8>, name: &str) {
        // Un ELF64 dit lui-même où il va et par où il commence : autant
        // l'écouter.
        if self.load_elf(&bytes, name) {
            return;
        }
        // Des octets bruts : ils ont droit à des notes comme les autres.
        // C'est même là qu'elles servent le plus — sans en-tête, il n'y a
        // aucun symbole pour aider.
        self.load_notes_for(&bytes);

        // Le fichier entier est gardé de côté avant toute troncature :
        // l'analyse — protections, sections, symboles — se lit dans ses
        // en-têtes, et ceux-là ne dépendent pas de ce qui tient dans la
        // mémoire simulée. Un ELF trop gros pour s'exécuter ici reste
        // ainsi parfaitement lisible.
        let file = if bytes.starts_with(b"\x7fELF") {
            bytes.clone()
        } else {
            Vec::new()
        };

        // Borné à la fenêtre mémoire simulée : au-delà, `rebuild_emulator`
        // refuserait le chargement de toute façon, et désassembler
        // plusieurs centaines de kilo-octets pour les jeter ensuite
        // figerait l'éditeur le temps de la mise en page du listing.
        let truncated = bytes.len() > MEM_SIZE;
        bytes.truncate(MEM_SIZE);

        // Ce qu'on désassemble : le code, et seulement lui, quand le
        // fichier dit où il est. Partir de l'octet zéro faisait lire
        // l'en-tête ELF et les tables de relocation comme des
        // instructions — les premières lignes d'un binaire ouvert
        // étaient du charabia, et c'est la première chose qu'on voyait.
        let disasm = executable_ranges(&file, bytes.len())
            .map(|plages| {
                let mut out = Vec::new();
                for (offset, len) in plages {
                    let début = offset as usize;
                    let fin = début + len as usize;
                    if let Some(morceau) = bytes.get(début..fin) {
                        out.extend(namsdec_disasm::disassemble(morceau, BASE_ADDR + offset));
                    }
                }
                out
            })
            .filter(|out| !out.is_empty())
            .unwrap_or_else(|| namsdec_disasm::disassemble(&bytes, BASE_ADDR));

        // Même chose pour un fichier lu tel quel : ses entrées de liaison
        // se nomment, à ceci près que le listing part du premier octet du
        // fichier et non des adresses où il se chargerait.
        let symbols = plt_labels(&file, &[], |plt| {
            let offset = namsdec_re::imports::file_offset(&file, plt)?;
            let addr = BASE_ADDR.checked_add(offset)?;
            (offset < bytes.len() as u64).then_some(addr)
        });
        let indirects = indirect_notes(&disasm, &file, &symbols, self.lang);
        let (listing, line_to_addr) = annotated_listing(&disasm, &symbols, &indirects);

        let addr_to_line = line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
        let loaded = bytes.len();
        self.saved_source = listing.clone();
        self.set_source(listing);
        self.current_file = None;
        self.breakpoint_lines.clear();
        self.diagnostics.clear();
        self.assemble_state = Some(AssembleState {
            output: namsdec_asm::AssembleOutput {
                load_addr: BASE_ADDR,
                entry: BASE_ADDR,
                // Un binaire brut ouvert depuis le disque n'a pas de
                // sections : tout ce qu'on en sait, c'est qu'on l'a
                // désassemblé comme du code depuis son premier octet.
                sections: Vec::new(),
                bytes,
                diagnostics: Vec::new(),
                line_to_addr,
                // Un binaire brut n'a pas de table de symboles ; un ELF
                // lu tel quel a au moins ses entrées de liaison, qui
                // nomment ce qu'il appelle.
                symbols,
                file,
            },
            disasm,
            addr_to_line,
            // Un tampon brut désassemblé : pas de bibliothèque à
            // réclamer, mais il vient bien d'un binaire.
            from_binary: Some(LoadedBinary::default()),
            octets_du_code: Default::default(),
        });
        self.note_program_io();
        self.rebuild_emulator();
        if !matches!(self.status, Status::Error(_)) {
            self.status = Status::BinaryLoaded {
                bytes: loaded,
                truncated,
            };
        }
    }

    pub(crate) fn do_save(&mut self) {
        match self.current_file.clone() {
            Some(path) => self.write_to(&path),
            None => self.do_save_as(),
        }
    }

    pub(crate) fn do_save_as(&mut self) {
        self.spawn_file_dialog(FileDialogKind::SaveAs);
    }

    pub(crate) fn write_to(&mut self, path: &Path) {
        // Atomique : c'est le fichier de l'utilisateur. Un éditeur qui
        // tronque un source sur une coupure n'est pas un éditeur.
        match crate::write_atomically(path, self.source.as_bytes()) {
            Ok(()) => {
                self.current_file = Some(path.to_path_buf());
                self.saved_source = self.source.clone();
                // Ce qu'on vient d'écrire fait foi : la date change, et
                // l'avertissement éventuel n'a plus lieu d'être.
                self.file_mtime = file_mtime(path);
                self.disk_changed = false;
            }
            Err(e) => {
                self.status = Status::Error(if self.lang == Lang::Fr {
                    format!("impossible d'enregistrer : {e}")
                } else {
                    format!("could not save: {e}")
                });
            }
        }
    }

    /// La question qui protège un travail non enregistré.
    ///
    /// Trois réponses, parce qu'il y en a trois : enregistrer et
    /// continuer, continuer sans enregistrer, ou ne rien faire du tout.
    /// Une boîte à deux boutons oblige à choisir entre perdre son
    /// travail et renoncer à ce qu'on voulait faire.
    pub(crate) fn unsaved_window_ui(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending.clone() else {
            return;
        };
        let s = strings(self.lang);
        let theme = self.theme;
        let mut décision: Option<bool> = None;
        let mut annuler = false;
        egui::Window::new(s.unsaved_title)
            .id(egui::Id::unique("unsaved_changes"))
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.viewport_rect().center())
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.unsaved_title) {
                    annuler = true;
                }
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.set_max_width(380.0);
                ui.label(s.unsaved_question);
                // Quand plusieurs fichiers sont en jeu, le dire : « le
                // fichier n'est pas enregistré » devant trois onglets
                // modifiés laisse croire qu'on n'en perd qu'un.
                let en_danger = self.modified_tabs();
                if en_danger > 1 {
                    ui.add_space(4.0);
                    ui.colored_label(
                        theme.warn,
                        if self.lang == Lang::Fr {
                            format!("{en_danger} fichiers ouverts ne sont pas enregistrés.")
                        } else {
                            format!("{en_danger} open files are not saved.")
                        },
                    );
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let save =
                        egui::Button::new(egui::RichText::new(s.unsaved_save).color(theme.bg))
                            .fill(theme.accent);
                    if ui.add(save).clicked() {
                        décision = Some(true);
                    }
                    if ui
                        .add(
                            egui::Button::new(s.unsaved_discard)
                                .stroke(egui::Stroke::new(1.0, theme.bad)),
                        )
                        .clicked()
                    {
                        décision = Some(false);
                    }
                    if framed_button(ui, theme, s.unsaved_cancel).clicked() {
                        annuler = true;
                    }
                });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            annuler = true;
        }
        if annuler {
            self.pending = None;
            // « Annuler » annule la fermeture entière, et pas seulement
            // celle de l'onglet affiché : rien ne dirait à l'écran
            // combien d'autres allaient encore partir.
            self.abandonner_fermetures();
            return;
        }
        let Some(enregistrer) = décision else {
            return;
        };
        if enregistrer {
            self.do_save();
            // Un enregistrement qui échoue — disque plein, fichier en
            // lecture seule — ne doit pas emporter le tampon avec lui.
            if self.buffer_modified() {
                return;
            }
            // Quitter emporte tous les fichiers : tant qu'il en reste un
            // à enregistrer, on l'ouvre et l'on repose la question. « Ne
            // pas enregistrer », lui, vaut pour tous — c'est ce qu'il
            // veut dire.
            if matches!(pending, Pending::Quit)
                && let Some(rang) = self.first_modified_tab()
            {
                self.go_to_tab(rang);
                self.pending = Some(Pending::Quit);
                return;
            }
        }
        self.pending = None;
        let fermait_un_onglet = matches!(pending, Pending::CloseTab(_));
        self.perform(pending);
        // Une fermeture groupée reprend là où la question l'avait
        // arrêtée : sans ça, « fermer tous les onglets » s'arrêterait au
        // premier qui n'est pas enregistré. Seulement après une
        // fermeture : si la question portait sur autre chose, ce qui
        // restait en file n'a plus de raison de partir.
        if fermait_un_onglet {
            self.poursuivre_fermetures();
        } else {
            self.abandonner_fermetures();
        }
    }

    /// Intercepte la fermeture de la fenêtre tant qu'il reste quelque
    /// chose à enregistrer.
    pub(crate) fn guard_close(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if self.quitting || !self.buffer_modified() {
            return;
        }
        // On annule la fermeture et l'on pose la question : le
        // gestionnaire de fenêtres la reposera quand on aura répondu.
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        self.pending = Some(Pending::Quit);
    }
}

/// Ce qu'une entrée du menu d'un onglet ferme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FermerLesquels {
    /// Tous sauf celui qu'on a désigné.
    Autres,
    Tous,
    /// Ceux d'avant : on garde celui-là et sa suite.
    Gauche,
    /// Ceux d'après.
    Droite,
}

/// Ce qu'un fichier est, décidé avant d'essayer de le lire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileKind {
    /// Du texte : il ira dans l'éditeur de source.
    Text,
    /// Des octets : ils iront au désassembleur et aux panneaux RE.
    Binary,
}

/// Les signatures qu'on reconnaît d'emblée, et le nom qu'on leur donne.
///
/// La liste n'a pas à être complète pour servir : elle a à contenir ce
/// qu'on ouvre par erreur. Un `.o`, une archive, une image, un PDF
/// traversaient le test « UTF-8 valide » quand leurs octets s'y
/// prêtaient, et finissaient dans l'éditeur.
const SIGNATURES: &[(&[u8], &str)] = &[
    (b"\x7fELF", "ELF"),
    (b"MZ", "PE/DOS"),
    (b"\xca\xfe\xba\xbe", "Mach-O universel"),
    (b"\xcf\xfa\xed\xfe", "Mach-O 64 bits"),
    (b"!<arch>", "archive ar"),
    (b"PK\x03\x04", "zip"),
    (b"%PDF", "PDF"),
    (b"\x89PNG", "PNG"),
    (b"\x1f\x8b", "gzip"),
    (b"\xfd7zXZ", "xz"),
    (b"\x04\x22\x4d\x18", "lz4"),
    (b"\x28\xb5\x2f\xfd", "zstd"),
    (b"\x00asm", "WebAssembly"),
    (b"dex\n", "Android dex"),
    (b"\xca\xfe\xba\xbe\x00\x00", "classe Java"),
];

/// Le nom de la signature que ces octets portent, s'ils en portent une.
pub(crate) fn signature_name(bytes: &[u8]) -> Option<&'static str> {
    SIGNATURES
        .iter()
        .find(|(magie, _)| bytes.starts_with(magie))
        .map(|(_, nom)| *nom)
}

/// Texte ou octets ?
///
/// Trois questions, dans cet ordre, et l'ordre est la correction :
///
/// 1. **une signature connue** — un ELF est un ELF, même si tous ses
///    octets forment par chance de l'UTF-8 valide ;
/// 2. **un octet nul** — aucun fichier source n'en contient, et c'est le
///    test que tout le monde emploie depuis `diff` ;
/// 3. **trop de caractères de contrôle** — au-delà d'un centième, ce
///    n'est pas du texte qu'on a sous les yeux.
///
/// Ce n'est qu'après que l'UTF-8 a son mot à dire, et seulement pour
/// refuser : un fichier qui a passé les trois questions mais n'est pas
/// de l'UTF-8 valide (du latin-1, par exemple) n'ira pas dans l'éditeur.
pub(crate) fn sniff(bytes: &[u8]) -> FileKind {
    if signature_name(bytes).is_some() {
        return FileKind::Binary;
    }
    // Un fichier vide est un source vide : c'est ce qu'on vient de
    // créer, pas un binaire de zéro octet.
    if bytes.is_empty() {
        return FileKind::Text;
    }
    // Le début suffit, et le borner garde l'ouverture d'un gros fichier
    // instantanée : un source dont les huit premiers kilo-octets sont du
    // texte l'est.
    let tête = &bytes[..bytes.len().min(8 * 1024)];
    if tête.contains(&0) {
        return FileKind::Binary;
    }
    let contrôles = tête
        .iter()
        .filter(|octet| **octet < 0x20 && !matches!(octet, b'\t' | b'\n' | b'\r' | 0x0c))
        .count();
    if contrôles * 100 > tête.len() {
        return FileKind::Binary;
    }
    FileKind::Text
}
