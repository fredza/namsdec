//! La séance : ce qu'on retrouve en revenant.
//!
//! Le fichier ouvert et son curseur, les fenêtres qui étaient là, les
//! marques posées sur un binaire, le thème, les raccourcis. Rouvrir
//! namsdec doit rendre l'écran qu'on a quitté — sans quoi chaque
//! démarrage commence par remettre en place ce qu'on avait déjà mis.
//!
//! La règle du projet tient en une phrase : un réglage qui se voit dans
//! l'interface part dans `settings.json` à l'instant où on le règle.
//! `settings.rs` dit la forme du fichier ; ce module dit quand on
//! l'écrit et ce qu'on en refait.

use super::*;

impl NamsdecApp {
    /// Construit l'application à partir des réglages relus sur le disque.
    pub(crate) fn from_settings(settings: Settings) -> Self {
        // Le dossier des réglages s'il est toujours là, sinon celui d'où
        // l'application a été lancée : l'explorateur ne doit jamais
        // s'ouvrir sur du vide.
        let root = settings
            .explorer_root()
            .filter(|p| p.is_dir())
            .or_else(default_explorer_root);
        let explorer = FileExplorer::with_root(root);
        let mut app = Self {
            theme: settings.theme(),
            lang: settings.lang(),
            // Ce que les RÉGLAGES DONNÉS portent, et rien d'autre.
            //
            // Le fichier `keymap.json` de la machine était lu ici, ce
            // qui paraissait juste — c'est lui qu'on ouvre et qu'on
            // copie d'une machine à l'autre. Mais `from_settings` sert
            // aussi à construire l'application dans les tests, et ceux-ci
            // recevaient alors les raccourcis de l'utilisateur : le jour
            // où Fred est passé au jeu « Éditeur ordinaire », DIX-SEPT
            // tests ont échoué d'un coup. Une suite qui passe ou échoue
            // selon ce que l'utilisateur a réglé la veille ne prouve
            // rien.
            //
            // C'est `adopt_keymap_from_disk`, appelé au démarrage juste
            // après que l'application a reçu son fichier de réglages, qui
            // lit le disque. Un seul endroit, et il est nommé.
            keybindings: settings.keybindings(),
            keymap_preset: keybindings::Preset::default(),
            keymap_mtime: None,
            show_toolbar: settings.show_toolbar,
            show_sidebar: settings.show_sidebar,
            show_statusbar: settings.show_statusbar,
            show_console: settings.show_console,
            show_explorer: settings.show_explorer,
            show_tooltips: settings.show_tooltips,
            flow_pseudo: settings.flow_pseudo,
            completion: settings.completion,
            auto_indent: settings.auto_indent,
            auto_pairs: settings.auto_pairs,
            hover_help: settings.hover_help,
            donation_url: settings.donation_url.clone(),
            exercises: settings.exercises,
            disabled_packs: settings.disabled_packs.iter().cloned().collect(),
            located_tools: settings
                .located_tools
                .iter()
                .map(|(id, chemin)| (id.clone(), std::path::PathBuf::from(chemin)))
                .collect(),
            locating_tool: None,
            current_exercise: settings.current_exercise.clone(),
            memory_format: MemoryFormat::by_id(&settings.memory_format),
            analysis_module: settings.analysis_module.clone(),
            // Une famille inconnue — un réglage écrit par une version
            // plus récente — retombe sur la première, plutôt que sur
            // une liste vide qui passerait pour un panneau cassé.
            calc_family: if crate::calc::FAMILIES
                .iter()
                .any(|(id, ..)| *id == settings.calc_family)
            {
                settings.calc_family.clone()
            } else {
                crate::calc::FAMILIES[0].0.to_owned()
            },
            calc_base: CalcBase::by_id(&settings.calc_base),
            calc_width: CalcWidth::by_id(&settings.calc_width),
            memory_follow: MemFollow::by_id(&settings.memory_follow),
            console_height: settings.console_height.map(|h| h as f32),
            sidebar_tab_height: settings.sidebar_tab_height.map(|h| h as f32),
            show_re_tab: settings.re_tab,
            program_args: settings.program_args.clone(),
            program_env: settings.program_env.clone(),
            active_tab: settings
                .sidebar_tab
                .as_deref()
                .and_then(SidebarTab::from_id)
                .unwrap_or(SidebarTab::Stack),
            show_decompiler_module: settings.decompiler_module,
            show_installer_module: settings.installer_module,
            decompiler: settings.decompiler.clone(),
            sidebar_banner_height: 0.0,
            sidebar_tab_last: SIDEBAR_TAB_MIN,
            system_titlebar: settings.system_titlebar,
            heat_legend: settings.heat_legend,
            flow_current_only: settings.flow_current_only,
            timeline_notable_only: settings.timeline_notable_only,
            console_env_open: settings.console_env_open,
            exercises_done: settings.exercises_done.iter().cloned().collect(),
            exercises_best: settings.exercises_best.iter().cloned().collect(),
            recent_files: settings.recent_files.iter().map(PathBuf::from).collect(),
            disassembler: settings.disassembler.clone(),
            explorer,
            saved_settings: settings,
            ..Default::default()
        };
        app.ensure_explorer_root();
        app.reopen_last_file();
        // Après les sources, et hors de `reopen_last_file` : celle-ci
        // rend la main dès qu'il n'y a pas de source à rouvrir, ce qui
        // est précisément le cas quand on travaillait un binaire —
        // `current_file` reste vide pour lui.
        app.restore_binary_session();
        // En dernier, et volontairement : deux des fenêtres reprises
        // dépendent d'un module dont l'état vient d'être posé plus haut,
        // et l'une d'elles — le désassemblage — n'a de sens qu'une fois
        // la séance revenue.
        app.reopen_windows();
        app
    }

    /// Prend les raccourcis du fichier `keymap.json` de la machine.
    ///
    /// Appelé une seule fois, au démarrage, et seulement par
    /// l'application qui a vraiment un fichier de réglages : c'est ce
    /// qui empêche les tests de lire — et donc de dépendre de — la
    /// configuration de l'utilisateur.
    ///
    /// `settings.json` garde les siens pour les versions qui ne
    /// connaissaient que lui : sans fichier de raccourcis sur le disque,
    /// on ne touche à rien.
    pub(crate) fn adopt_keymap_from_disk(&mut self) {
        if self.settings_file.is_none() {
            return;
        }
        let keymap = settings::load_keymap();
        self.keymap_preset = keymap.preset();
        self.keymap_mtime = settings::keymap_path().as_deref().and_then(file_mtime);
        if settings::keymap_path().is_some_and(|p| p.exists()) {
            self.keybindings = keymap.bindings();
        }
    }

    /// Rouvre les fenêtres qu'on avait laissées ouvertes.
    ///
    /// Les noms inconnus sont ignorés sans bruit : un fichier de réglages
    /// écrit par une version plus récente peut nommer une fenêtre que
    /// celle-ci n'a pas, et ce n'est pas une raison de se plaindre.
    pub(crate) fn reopen_windows(&mut self) {
        let voulues = self.saved_settings.open_windows.clone();
        for nom in &voulues {
            if let Some((_, _, ouvre)) = FENÊTRES_REPRISES.iter().find(|(id, _, _)| id == nom) {
                ouvre(self);
            }
        }
    }

    /// Rouvre le fichier sur lequel on travaillait au lancement précédent,
    /// pour reprendre où l'on s'était arrêté.
    ///
    /// Seul le chemin a été enregistré : c'est le fichier du disque qui
    /// fait foi, y compris s'il a été modifié ailleurs entre-temps. En
    /// garder une copie dans les réglages n'y entretiendrait qu'un double
    /// périmé.
    ///
    /// Un fichier disparu ou devenu illisible ne doit ni empêcher de
    /// démarrer, ni accueillir par une erreur quelqu'un qui vient
    /// seulement d'ouvrir l'application : on retombe sur le programme
    /// d'exemple, et l'entrée périmée disparaît des réglages d'elle-même.
    pub(crate) fn reopen_last_file(&mut self) {
        let Some(path) = self
            .saved_settings
            .current_file()
            .filter(|path| path.is_file())
        else {
            self.current_file = None;
            return;
        };
        self.restoring_session = true;
        self.open_file(path);
        self.restoring_session = false;
        if matches!(self.status, Status::Error(_)) {
            self.current_file = None;
            self.set_source(DEFAULT_SOURCE.to_owned());
            self.status = Status::Idle;
        }
        // Puis les autres fichiers de la dernière séance, dans leur
        // ordre. Ceux qui ont disparu du disque sont simplement passés :
        // une erreur au démarrage pour un fichier déplacé hier
        // n'apprendrait rien à personne.
        let autres: Vec<PathBuf> = self
            .saved_settings
            .open_files
            .iter()
            .map(PathBuf::from)
            .filter(|path| path.is_file() && Some(path.as_path()) != self.current_file.as_deref())
            .collect();
        // On revient ensuite sur celui qu'on regardait : ouvrir les
        // autres a déplacé l'onglet actif.
        let actif = self.current_file.clone();
        self.restoring_session = true;
        for path in autres {
            self.open_file(path);
        }
        self.restoring_session = false;
        if let Some(actif) = actif
            && let Some(rang) = self.tab_index_of(&actif)
        {
            self.go_to_tab(rang);
        }

        // Puis les repères posés sur ce fichier — mais seulement s'il
        // est bien celui qu'on a quitté. Ils se comptent en numéros de
        // ligne : réécrit ailleurs entre deux séances, le fichier les
        // ferait désigner d'autres instructions, et poser des points
        // d'arrêt au hasard est pire que de ne pas en poser.
        //
        // Sans date enregistrée — un fichier de réglages d'une version
        // antérieure —, on s'abstient : c'est ce qui se passait avant,
        // et cela ne surprendra personne.
        let date = self
            .current_file
            .as_deref()
            .and_then(file_mtime)
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let repères = !self.saved_settings.breakpoints.is_empty()
            || !self.saved_settings.watchpoints.is_empty();
        if date.is_some() && date == self.saved_settings.current_file_mtime {
            self.restore_session_marks();
        } else if repères && self.saved_settings.current_file_mtime.is_some() {
            // Le dire : quelqu'un qui a édité son programme ailleurs
            // doit savoir pourquoi ses points d'arrêt ont disparu.
            self.status = Status::Note(|s| s.session_file_changed);
        }
    }

    /// Reprend le travail commencé sur un binaire : le binaire lui-même,
    /// puis les repères posés dessus.
    ///
    /// Les sources se rouvrent depuis longtemps ; un binaire, non — et
    /// c'est pourtant là qu'il y a du travail à perdre. Rouvrir namsdec
    /// demandait de retrouver le fichier, de le recharger, de reposer ses
    /// points d'arrêt et de retaper ses arguments.
    ///
    /// **Les repères ne reviennent que si le fichier est le même.** Ils se
    /// comptent en numéros de ligne d'un listing reconstruit à chaque
    /// ouverture : sur un binaire recompilé entre-temps, ils désigneraient
    /// d'autres instructions, et poser des points d'arrêt au hasard est
    /// pire que de ne pas en poser.
    pub(crate) fn restore_binary_session(&mut self) {
        let Some(chemin) = self.saved_settings.open_binary.clone().map(PathBuf::from) else {
            return;
        };
        if !chemin.is_file() {
            return;
        }
        let Ok(octets) = std::fs::read(&chemin) else {
            return;
        };
        let nom = chemin
            .file_name()
            .map(|nom| nom.to_string_lossy().into_owned())
            .unwrap_or_else(|| "programme".to_owned());
        self.load_binary(octets, &nom);
        self.binary_path = Some(chemin.clone());

        // Le fichier est-il celui qu'on avait quitté ? Taille et date :
        // deux mesures qu'on a sous la main, et qui suffisent à écarter
        // le cas qui compte — un binaire recompilé depuis.
        let taille = std::fs::metadata(&chemin).ok().map(|m| m.len());
        let date = file_mtime(&chemin)
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let même = taille == self.saved_settings.open_binary_len
            && date == self.saved_settings.open_binary_mtime;
        if !même {
            // Le dire, plutôt que de laisser croire que rien n'a été
            // perdu : quelqu'un qui a recompilé son binaire doit savoir
            // pourquoi ses points d'arrêt ont disparu.
            if !self.saved_settings.breakpoints.is_empty()
                || !self.saved_settings.watchpoints.is_empty()
            {
                self.status = Status::Note(|s| s.session_binary_changed);
            }
            return;
        }

        self.restore_session_marks();
    }

    /// Repose les repères de la séance précédente : points d'arrêt,
    /// zones surveillées, registres suivis.
    ///
    /// Un seul endroit pour les deux sortes de séance. Ils n'étaient
    /// reposés que pour un binaire, si bien que quelqu'un qui travaille
    /// son propre programme — le cas ordinaire — retrouvait son fichier
    /// mais aucun de ses points d'arrêt. Ils étaient pourtant écrits
    /// dans les réglages : personne ne les relisait.
    pub(crate) fn restore_session_marks(&mut self) {
        self.breakpoint_lines = self.saved_settings.breakpoints.iter().copied().collect();
        // La ligne qu'on lisait : même garde que les points d'arrêt —
        // c'est pour cela qu'elle est ici et nulle part ailleurs. La vue
        // y va à la première image, le curseur l'y suit, et le mode ne
        // change pas (voir `reprise_ligne`).
        self.reprise_ligne = self
            .saved_settings
            .listing_line
            .filter(|ligne| *ligne > 1 && (*ligne as usize) <= self.source.lines().count());
        // Le curseur y est tout de suite, et pas seulement quand
        // l'éditeur se dessine : la barre d'état est peinte AVANT lui, et
        // annoncer « Lig 1 » au premier regard sur un listing qu'on
        // rouvre à la ligne 4 012 serait faux.
        if let Some(ligne) = self.reprise_ligne {
            self.cursor_pos = (ligne as usize, 1);
            self.cursor_byte =
                ligne_début(&self.source, ligne.saturating_sub(1) as usize).unwrap_or(0);
        }
        self.watchpoints = self
            .saved_settings
            .watchpoints
            .iter()
            .map(|(addr, len)| (*addr, *len as usize))
            .collect();
        self.watched_registers = REGISTERS
            .iter()
            .map(|(nom, _)| *nom)
            .filter(|nom| {
                self.saved_settings
                    .watched_registers
                    .iter()
                    .any(|gardé| gardé == nom)
            })
            .collect();
        self.sync_watchpoints();
    }

    /// Écrit `keymap.json` et retient sa nouvelle date.
    ///
    /// La date, sans quoi la surveillance prendrait notre propre
    /// écriture pour une modification venue d'ailleurs et annoncerait
    /// une reprise à chaque changement de raccourci.
    pub(crate) fn write_keymap(&mut self, keymap: &settings::Keymap) {
        let _ = settings::save_keymap(keymap);
        self.keymap_mtime = settings::keymap_path().as_deref().and_then(file_mtime);
    }

    /// Reprend `keymap.json` s'il a changé sur le disque.
    ///
    /// Le fichier se relisait au lancement et nulle part ailleurs : on
    /// l'éditait pendant que namsdec tournait, on essayait la touche, et
    /// il ne se passait rien — sans que rien ne dise pourquoi. C'est un
    /// fichier qu'on donne à éditer ; il doit donc être écouté.
    ///
    /// La date est relevée au plus une fois par seconde : un appel
    /// système par image pour un fichier qu'on modifie deux fois par an
    /// serait absurde, et une seconde est bien en deçà du temps qu'il
    /// faut pour passer de son éditeur à namsdec.
    ///
    /// Une capture de raccourci en cours suspend la reprise : changer le
    /// jeu sous les doigts de quelqu'un qui appuie sur une touche serait
    /// la dernière chose à faire.
    pub(crate) fn reload_keymap_if_changed(&mut self) {
        // Pas de fichier de réglages, pas de surveillance.
        //
        // C'est le même garde que pour l'écriture, et pour une raison
        // aussi forte : une application construite par un test n'a pas
        // de fichier de réglages, et celle-ci lisait quand même le
        // `keymap.json` de l'utilisateur à chaque image. Le jour où Fred
        // est passé au jeu « Éditeur ordinaire » dans les Préférences,
        // DIX-SEPT tests ont échoué d'un coup — ils comparaient le jeu
        // Vim et recevaient celui que la machine venait d'enregistrer.
        //
        // Un test ne doit pas écrire chez l'utilisateur ; il ne doit pas
        // non plus le LIRE, sans quoi la suite passe ou échoue selon ce
        // que l'utilisateur a réglé la veille.
        if self.settings_file.is_none() || self.rebinding.is_some() {
            return;
        }
        let maintenant = std::time::Instant::now();
        if self
            .keymap_checked
            .is_some_and(|vu| maintenant.duration_since(vu) < KEYMAP_POLL_INTERVAL)
        {
            return;
        }
        self.keymap_checked = Some(maintenant);
        let Some(chemin) = settings::keymap_path() else {
            return;
        };
        let date = file_mtime(&chemin);
        // Rien n'a bougé — ou le fichier n'existe pas encore, ce qui
        // n'est pas un changement.
        if date.is_none() || date == self.keymap_mtime {
            return;
        }
        self.keymap_mtime = date;
        let keymap = settings::load_keymap();
        let nouveaux = keymap.bindings();
        if nouveaux.map == self.keybindings.map && keymap.preset() == self.keymap_preset {
            return;
        }
        self.keybindings = nouveaux;
        self.keymap_preset = keymap.preset();
        self.status = Status::Note(|s| s.shortcuts_reloaded);
    }

    /// Écrit le fichier de réglages sans attendre qu'un réglage change :
    /// appelé au démarrage pour que l'utilisateur puisse le trouver et
    /// l'éditer dès le premier lancement, avec les défauts du jour gravés
    /// dedans plutôt que de suivre en silence ceux d'une version future.
    ///
    /// Volontairement hors de `from_settings` : construire l'application
    /// ne doit pas écrire sur le disque, sinon les tests qui la
    /// construisent écraseraient la configuration de la machine.
    /// Écrit `keymap.json` dès le premier lancement, comme on le fait
    /// pour les réglages.
    ///
    /// Le fichier n'était posé qu'au premier changement de raccourci.
    /// Deux jeux vivent pourtant dans le binaire — Fake Vim-GDB, qui est
    /// le défaut, et l'éditeur ordinaire — et rien ne le disait à qui
    /// n'ouvrait pas les Préférences : on cherchait un fichier qui
    /// n'existait pas encore pour savoir de quoi on partait.
    ///
    /// Hors de `from_settings`, pour la même raison que les réglages :
    /// construire l'application ne doit pas écrire sur le disque, sinon
    /// les tests qui la construisent écraseraient les raccourcis de
    /// l'utilisateur.
    pub(crate) fn save_keymap_now(&mut self) {
        if self.settings_file.is_none() {
            return;
        }
        let keymap = settings::Keymap::from_bindings(self.keymap_preset, &self.keybindings);
        self.write_keymap(&keymap);
    }

    pub(crate) fn save_settings_now(&mut self) {
        let Some(path) = self.settings_file.clone() else {
            return;
        };
        if let Err(e) = self.saved_settings.save_to(&path) {
            self.status = Status::Error(if self.lang == Lang::Fr {
                format!("impossible d'enregistrer les réglages : {e}")
            } else {
                format!("could not save settings: {e}")
            });
        }
    }

    /// L'état courant des réglages, sous la forme qui part dans le
    /// fichier. Tout ce qui est réglable par l'utilisateur doit figurer
    /// ici — c'est le point unique à compléter en ajoutant un réglage.
    pub(crate) fn settings_snapshot(&self) -> Settings {
        Settings {
            // Toujours la version courante : l'instantané décrit l'état de
            // *cette* version de l'application. Reprendre la version lue
            // dans le fichier ferait diverger l'instantané de ce qui est
            // enregistré, et l'application réécrirait le fichier à chaque
            // frame en croyant qu'un réglage a bougé.
            version: settings::SETTINGS_VERSION,
            theme: self.theme.id.to_owned(),
            language: match self.lang {
                Lang::Fr => "fr".to_owned(),
                Lang::En => "en".to_owned(),
            },
            open_binary: self.binary_path.as_ref().map(|p| p.display().to_string()),
            open_binary_len: self
                .binary_path
                .as_ref()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len()),
            open_binary_mtime: self
                .binary_path
                .as_deref()
                .and_then(file_mtime)
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            breakpoints: self.breakpoint_lines.iter().copied().collect(),
            // La ligne qu'on lisait — rien quand c'était la première,
            // qui est là où l'on rouvre de toute façon.
            listing_line: (self.cursor_pos.0 > 1).then_some(self.cursor_pos.0 as u32),
            watchpoints: self
                .watchpoints
                .iter()
                .map(|(addr, len)| (*addr, *len as u32))
                .collect(),
            watched_registers: self
                .watched_registers
                .iter()
                .map(|nom| (*nom).to_owned())
                .collect(),
            program_args: self.program_args.clone(),
            program_env: self.program_env.clone(),
            sidebar_tab: Some(self.active_tab.id().to_owned()),
            open_windows: FENÊTRES_REPRISES
                .iter()
                .filter(|(_, ouverte, _)| ouverte(self))
                .map(|(nom, _, _)| (*nom).to_owned())
                .collect(),
            show_toolbar: self.show_toolbar,
            show_sidebar: self.show_sidebar,
            show_statusbar: self.show_statusbar,
            show_console: self.show_console,
            show_explorer: self.show_explorer,
            show_tooltips: self.show_tooltips,
            flow_pseudo: self.flow_pseudo,
            completion: self.completion,
            auto_indent: self.auto_indent,
            auto_pairs: self.auto_pairs,
            hover_help: self.hover_help,
            donation_url: self.donation_url.clone(),
            exercises: self.exercises,
            disabled_packs: self.disabled_packs.iter().cloned().collect(),
            located_tools: self
                .located_tools
                .iter()
                .map(|(id, chemin)| (id.clone(), chemin.to_string_lossy().into_owned()))
                .collect(),
            current_exercise: self.current_exercise.clone(),
            disassembler: self.disassembler.clone(),
            memory_format: self.memory_format.id().to_owned(),
            analysis_module: self.analysis_module.clone(),
            calc_base: self.calc_base.id().to_owned(),
            calc_family: self.calc_family.clone(),
            calc_width: self.calc_width.id(),
            memory_follow: self.memory_follow.id().to_owned(),
            console_height: self.console_height.map(|h| h.round() as u32),
            sidebar_tab_height: self.sidebar_tab_height.map(|h| h.round() as u32),
            re_tab: self.show_re_tab,
            decompiler_module: self.show_decompiler_module,
            installer_module: self.show_installer_module,
            decompiler: self.decompiler.clone(),
            system_titlebar: self.system_titlebar,
            heat_legend: self.heat_legend,
            flow_current_only: self.flow_current_only,
            timeline_notable_only: self.timeline_notable_only,
            console_env_open: self.console_env_open,
            exercises_done: self.exercises_done.iter().cloned().collect(),
            exercises_best: self
                .exercises_best
                .iter()
                .map(|(id, compte)| (id.clone(), *compte))
                .collect(),
            recent_files: self
                .recent_files
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            explorer_root: self.explorer.root.as_ref().map(|p| p.display().to_string()),
            current_file: self.current_file.as_ref().map(|p| p.display().to_string()),
            // La date du source, pour la même raison que celle du
            // binaire : elle dit, au retour, si les points d'arrêt
            // désignent encore les mêmes lignes.
            current_file_mtime: self
                .current_file
                .as_deref()
                .and_then(file_mtime)
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            // Les autres onglets, dans leur ordre : au lancement
            // suivant, on retrouve les fichiers sur lesquels on
            // travaillait, et non le seul dernier.
            open_files: self
                .tabs
                .iter()
                .filter_map(|tab| tab.path.as_ref())
                .map(|p| p.display().to_string())
                .collect(),
            shortcuts: settings::shortcuts_to_strings(&self.keybindings),
        }
    }

    /// Appelé à chaque frame : n'écrit que si un réglage a bougé. Une
    /// écriture qui échoue est signalée une seule fois (l'instantané est
    /// mis à jour quoi qu'il arrive) plutôt que retentée à chaque frame.
    pub(crate) fn persist_settings_if_changed(&mut self) {
        let Some(path) = self.settings_file.clone() else {
            return;
        };
        let current = self.settings_snapshot();
        if current == self.saved_settings {
            return;
        }
        let result = current.save_to(&path);
        self.saved_settings = current;
        if let Err(e) = result {
            self.status = Status::Error(if self.lang == Lang::Fr {
                format!("impossible d'enregistrer les réglages : {e}")
            } else {
                format!("could not save settings: {e}")
            });
        }
    }

    /// Les infobulles s'éteignent par le délai avant affichage plutôt
    /// qu'au cas par cas : porté par le style d'egui, le réglage vaut pour
    /// **toutes** les bulles, y compris celles de l'explorateur — qui est
    /// une crate à part — et celles qu'on ajoutera plus tard. Un délai
    /// infini n'arrive jamais à échéance ; le délai de grâce (qui laisse
    /// la bulle suivante s'afficher tout de suite après une première)
    /// tombe à zéro pour qu'aucune ne passe par cette porte.
    pub(crate) fn apply_tooltip_setting(&self, ctx: &egui::Context) {
        let delay = if self.show_tooltips {
            TOOLTIP_DELAY
        } else {
            f32::INFINITY
        };
        // Le style du thème clair et celui du thème sombre sont deux
        // objets distincts, réglés ensemble juste en dessous : il suffit
        // d'en interroger un.
        if ctx.style_of(egui::Theme::Dark).interaction.tooltip_delay == delay {
            return;
        }
        ctx.all_styles_mut(|style| {
            style.interaction.tooltip_delay = delay;
            style.interaction.tooltip_grace_time = if self.show_tooltips {
                TOOLTIP_GRACE_TIME
            } else {
                0.0
            };
        });
    }

    pub(crate) fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.theme.visuals());
        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);
            style.spacing.button_padding = egui::vec2(10.0, 5.0);
            style.spacing.window_margin = egui::Margin::from(12);
            // Des barres de défilement qui prennent leur place, au lieu
            // de flotter par-dessus le contenu et de s'effacer dès que la
            // souris s'en va : flottantes, elles n'apparaissaient qu'au
            // survol, et rien ne disait qu'il y avait quelque chose de
            // plus bas. Elles restent affichées seulement quand il y a de
            // quoi défiler.
            style.spacing.scroll.floating = false;
        });
    }

    /// Passe d'un jeu de raccourcis à l'autre.
    ///
    /// Ce qu'on a déplacé soi-même est gardé : changer de jeu répond à
    /// « d'où je pars », non à « efface ce que j'ai réglé ».
    pub(crate) fn apply_keymap_preset(&mut self, preset: keybindings::Preset) {
        let écarts = settings::Keymap::from_bindings(self.keymap_preset, &self.keybindings);
        self.keymap_preset = preset;
        let keymap = settings::Keymap {
            preset: preset.id().to_owned(),
            shortcuts: écarts.shortcuts,
        };
        self.keybindings = keymap.bindings();
        self.write_keymap(&keymap);
    }
}
