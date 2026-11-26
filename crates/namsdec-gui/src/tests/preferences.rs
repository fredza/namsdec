//! Les préférences, et les thèmes qu'on y installe.
//!
//! Chaque page garde ses boutons à l'écran et ne grandit pas d'elle-même ;
//! un thème venu d'un fichier rejoint la liste et s'en retire.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// Les vignettes de thème ont toutes la même largeur, celle que
/// dicte le nom le plus long — c'est ce qui fait tomber les boutons
/// « Retirer » les uns sous les autres, au lieu de les décaler au
/// gré de la longueur des noms.
#[test]
fn theme_cards_share_a_width_set_by_the_longest_name() {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        // « Tokyo Night » est plus court que « VS Code Light+ ».
        let court = [&namsdec_theme::TOKYO_NIGHT];
        let long = [&namsdec_theme::TOKYO_NIGHT, &namsdec_theme::VSCODE_LIGHT];

        let avec_court = theme_card_width(ui, &court, "actif");
        let avec_long = theme_card_width(ui, &long, "actif");
        assert!(
            avec_long > avec_court,
            "un nom plus long élargit toutes les vignettes : {avec_long} contre {avec_court}"
        );

        // Le même jeu de thèmes donne toujours la même largeur, quel
        // que soit l'ordre : c'est ce qui les aligne.
        let inverse = [&namsdec_theme::VSCODE_LIGHT, &namsdec_theme::TOKYO_NIGHT];
        assert_eq!(avec_long, theme_card_width(ui, &inverse, "actif"));

        // Et le mot « actif » compte : il s'affiche à droite du nom,
        // dans la vignette.
        assert!(theme_card_width(ui, &long, "actif") > avec_long - 1.0);
        assert!(
            theme_card_width(ui, &long, "actuellement actif") > avec_long,
            "un mot plus long élargit la vignette d'autant"
        );
    });
    output.textures_delta.clear();
}

/// Les boutons « Retirer » tombent les uns sous les autres, quelle
/// que soit la longueur des noms — c'est ce qu'on attend d'une liste,
/// et ce qu'un nom court aurait défait si la vignette s'y ajustait.
#[test]
fn the_remove_buttons_line_up_whatever_the_names() {
    // Deux thèmes installés, aux noms de longueurs très différentes.
    let dir = tempfile::tempdir().unwrap();
    let sample = external_abyss()
        .and_then(|chemin| std::fs::read_to_string(chemin).ok())
        .unwrap_or_else(return_sample);
    let mut app = NamsdecApp::default();
    for (id, name) in [("court-a", "Ab"), ("long-a", "Un nom vraiment très long")] {
        let file = sample
            .replace("\"abyss\"", &format!("\"{id}\""))
            .replace("\"Abyss\"", &format!("\"{name}\""));
        let path = dir.path().join(format!("{id}.json"));
        std::fs::write(&path, &file).unwrap();
        let theme = namsdec_theme::plugin::read(&path).expect("thème de test");
        if namsdec_theme::install(theme).is_ok() {
            app.theme_files.insert(id.to_owned(), path);
        }
    }

    let ctx = egui::Context::default();

    app.apply_theme(&ctx);
    ctx.set_visuals(app.theme.visuals());
    let mut left_edges = Vec::new();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        app.preferences_plugins_ui(ui);
    });
    for clipped in &output.shapes {
        // Les cadres des boutons, et eux seuls (voir `framed_button`).
        if let egui::epaint::Shape::Rect(rect) = &clipped.shape
            && rect.stroke.width >= 1.0
            && rect.stroke.color == app.theme.dim
        {
            left_edges.push(rect.rect.min.x);
        }
    }
    output.textures_delta.clear();

    // Deux « Retirer » plus le bouton d'ajout, qui commence à gauche.
    assert!(left_edges.len() >= 3, "obtenu {left_edges:?}");
    let mut removes: Vec<f32> = left_edges.iter().copied().filter(|x| *x > 100.0).collect();
    removes.sort_by(f32::total_cmp);
    removes.dedup();
    assert_eq!(
        removes.len(),
        1,
        "les boutons « Retirer » doivent partager une abscisse, obtenu {removes:?}"
    );

    for id in ["court-a", "long-a"] {
        namsdec_theme::uninstall(id);
    }
}

/// Retirer un thème installé : son fichier disparaît, il quitte la
/// liste, et les thèmes d'origine ne sont jamais concernés — ils ne
/// figurent pas dans la table des fichiers, donc aucun bouton ne les
/// propose.
#[test]
fn removing_an_installed_theme_deletes_its_file_and_frees_the_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jetable.namsdec-theme.json");
    let file = external_abyss()
        .and_then(|chemin| std::fs::read_to_string(chemin).ok())
        .unwrap_or_else(return_sample)
        .replace("\"abyss\"", "\"abyss-a-retirer\"");
    std::fs::write(&path, &file).unwrap();

    let theme = namsdec_theme::plugin::read(&path).expect("le thème se lit");
    let installed = namsdec_theme::install(theme).expect("installation");

    let mut app = NamsdecApp {
        theme: installed,
        ..Default::default()
    };
    app.theme_files
        .insert("abyss-a-retirer".to_owned(), path.clone());
    // Un thème d'origine n'a pas de fichier : rien ne peut le retirer.
    assert!(!app.theme_files.contains_key(DEFAULT_THEME.id));

    app.remove_theme("abyss-a-retirer", &egui::Context::default());

    assert!(!path.exists(), "le fichier est supprimé");
    assert!(
        !namsdec_theme::all()
            .iter()
            .any(|t| t.id == "abyss-a-retirer"),
        "et le thème quitte la liste"
    );
    assert!(app.theme_files.is_empty());
    assert_eq!(
        app.theme.id, DEFAULT_THEME.id,
        "le thème actif ayant disparu, on retombe sur celui d'origine"
    );
    let note = app.plugin_note.as_ref().expect("un compte rendu");
    assert!(note.ok, "{}", note.text);
}

/// Installe réellement le thème d'exemple, par le chemin de code du
/// bouton « Ajouter un thème… », dans le dossier de plugins de
/// l'utilisateur courant.
///
/// Marqué `ignore` : il écrit hors du dépôt, personne ne veut de ça
/// en lançant la suite. Il se lance à la main —
/// `cargo test -p namsdec-gui -- --ignored --exact` — et se défait
/// avec `uninstalls_the_external_theme_for_real`.
#[test]
#[ignore = "écrit dans le dossier de plugins de l'utilisateur"]
fn installs_the_external_theme_for_real() {
    let chemin =
        external_abyss().expect("NAMSDEC_EXTERNAL_THEME doit désigner le fichier du thème");
    let path = chemin.as_path();

    let mut app = NamsdecApp::default();
    let before = app.theme.id;
    // Exactement ce que le bouton appelle.
    app.install_theme(path, &egui::Context::default());

    let note = app.plugin_note.as_ref().expect("un compte rendu");
    assert!(note.ok, "l'installation doit réussir : {}", note.text);
    assert_eq!(app.theme.id, "abyss", "et le thème est appliqué aussitôt");
    assert_ne!(before, app.theme.id);

    let installed = plugins_dir()
        .expect("un dossier de données")
        .join("abyss.namsdec-theme.json");
    assert!(installed.exists(), "le fichier est recopié : {installed:?}");
}

/// Le retrait par le bouton, pour de vrai : même fonction que celle
/// que la confirmation appelle, sur le fichier réellement installé.
/// `ignore` pour la même raison que son jumeau.
#[test]
#[ignore = "écrit dans le dossier de plugins de l'utilisateur"]
fn removes_the_external_theme_for_real() {
    let dir = plugins_dir().expect("un dossier de données");
    let installed = dir.join("abyss.namsdec-theme.json");
    assert!(installed.exists(), "à lancer après l'installation");

    // Tel qu'au démarrage : les thèmes du dossier sont chargés, et
    // c'est de là que le bouton tient le chemin du fichier.
    let (theme_files, failures) = load_installed_themes();
    assert!(failures.is_empty(), "{failures:?}");
    let mut app = NamsdecApp {
        theme_files,
        ..Default::default()
    };
    assert!(
        app.theme_files.contains_key("abyss"),
        "le thème installé doit être retrouvé, avec son fichier"
    );

    app.remove_theme("abyss", &egui::Context::default());

    let note = app.plugin_note.as_ref().expect("un compte rendu");
    assert!(note.ok, "{}", note.text);
    assert!(!installed.exists(), "le fichier est supprimé");
    assert!(!namsdec_theme::all().iter().any(|t| t.id == "abyss"));
    if dir.read_dir().is_ok_and(|mut d| d.next().is_none()) {
        let _ = std::fs::remove_dir(&dir);
    }
}

/// Le pendant du précédent : retire le fichier installé et vérifie
/// qu'il ne reste rien.
#[test]
#[ignore = "écrit dans le dossier de plugins de l'utilisateur"]
fn uninstalls_the_external_theme_for_real() {
    let dir = plugins_dir().expect("un dossier de données");
    let installed = dir.join("abyss.namsdec-theme.json");
    if installed.exists() {
        std::fs::remove_file(&installed).expect("le retrait du fichier");
    }
    assert!(!installed.exists());
    // Un dossier vide ne sert à rien : on le retire aussi, pour ne
    // laisser aucune trace.
    if dir.read_dir().is_ok_and(|mut d| d.next().is_none()) {
        let _ = std::fs::remove_dir(&dir);
    }
}

/// Le thème d'exemple livré à côté du dépôt s'installe : lu,
/// enregistré, retrouvé par son identifiant, et proposé dans la
/// liste comme les thèmes d'origine.
///
/// Il n'est pas *copié* ici : ce test lit un fichier, il n'a rien à
/// faire dans le dossier de plugins de qui exécute la suite.
#[test]
fn an_external_theme_file_installs_and_joins_the_list() {
    // Le plugin vit hors du dépôt : sans `NAMSDEC_EXTERNAL_THEME`, ce
    // test n'a rien à vérifier.
    let Some(path) = external_abyss() else {
        return;
    };
    let path = path.as_path();
    let theme = namsdec_theme::plugin::read(path).expect("le thème doit se lire");
    assert_eq!(theme.name, "Abyss");

    let before = namsdec_theme::all().len();
    let installed = match namsdec_theme::install(theme) {
        Ok(installed) => installed,
        // Un autre test de ce binaire l'a déjà installé : la liste
        // est partagée par tout le processus.
        Err(_) => namsdec_theme::by_id("abyss"),
    };
    assert_eq!(installed.id, "abyss");
    assert!(namsdec_theme::all().len() >= before);
    assert!(
        std::ptr::eq(namsdec_theme::by_id("abyss"), installed),
        "il se retrouve par son identifiant, comme un thème d'origine"
    );

    // Et un identifiant déjà pris est refusé : deux thèmes qui
    // répondent au même nom rendraient le réglage imprévisible.
    let again = namsdec_theme::plugin::read(path).unwrap();
    assert!(matches!(
        namsdec_theme::install(again),
        Err(namsdec_theme::PluginError::DuplicateId(_))
    ));
}

/// Les infobulles s'éteignent pour toute l'application, explorateur
/// compris : c'est le style d'egui qui porte le réglage, pas chaque
/// appel à `on_hover_text`.
#[test]
fn tooltips_can_be_turned_off_and_back_on() {
    let mut app = NamsdecApp {
        show_tooltips: false,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let frame = |app: &mut NamsdecApp, ctx: &egui::Context| {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
        output.textures_delta.clear();
    };

    frame(&mut app, &ctx);
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let interaction = &ctx.style_of(theme).interaction;
        assert!(
            interaction.tooltip_delay.is_infinite(),
            "aucune bulle ne doit arriver à échéance"
        );
        assert_eq!(
            interaction.tooltip_grace_time, 0.0,
            "ni passer par le délai de grâce"
        );
    }

    app.show_tooltips = true;
    frame(&mut app, &ctx);
    assert_eq!(
        ctx.style_of(egui::Theme::Dark).interaction.tooltip_delay,
        TOOLTIP_DELAY
    );
}

/// Quelle que soit la page, les boutons de la boîte restent à l'écran.
///
/// Chaque page avait sa propre zone qui défile, réglée pour prendre
/// toute la hauteur offerte : la fenêtre s'étirait jusqu'en bas de
/// l'écran et « OK » et « Quitter » passaient dessous. Une seule zone,
/// qui s'arrête avant la barre du bas, et le problème ne peut pas
/// revenir.
#[test]
fn every_preferences_page_keeps_its_buttons_on_screen() {
    for onglet in [
        PreferencesTab::General,
        PreferencesTab::Shortcuts,
        PreferencesTab::Modules,
        PreferencesTab::Plugins,
        PreferencesTab::Tools,
    ] {
        let mut app = NamsdecApp {
            show_preferences: true,
            preferences_tab: onglet,
            ..Default::default()
        };
        let peint = paint(&mut app);
        assert!(
            peint.contains(strings(Lang::Fr).dialog_ok)
                && peint.contains(strings(Lang::Fr).dialog_cancel),
            "OK et Quitter doivent se lire sur chaque page :\n{peint}"
        );
    }
}

/// Les deux jeux de raccourcis se choisissent dans les Préférences.
///
/// Ils sont tous les deux dans le binaire, et Vim est le défaut — d'où le
/// « NORMAL » de la barre d'état au premier lancement.
#[test]
fn both_keymap_presets_are_offered_in_the_preferences() {
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Shortcuts,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };
    let mut peint = String::new();
    for _ in 0..3 {
        peint.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                peint.push_str(texte.galley.text());
                peint.push('|');
            }
        }
        sortie.textures_delta.clear();
    }
    for preset in keybindings::Preset::ALL {
        assert!(
            peint.contains(preset.label(true)),
            "« {} » doit se lire dans l'onglet Raccourcis :\n{peint}",
            preset.label(true)
        );
    }
}

/// La fenêtre des préférences ne grandit pas d'elle-même, page par page.
///
/// Le banc le dit déjà pour les quatorze fenêtres, mais il n'ouvrait que
/// la PREMIÈRE page des préférences — la seule qui tienne sans défiler.
/// Les quatre autres grandissaient d'un pixel par image, sans fin : la
/// zone qui défile prenait sa hauteur sur la fenêtre, et la fenêtre
/// prenait la sienne sur son contenu. Vingt images, vingt hauteurs.
///
/// Ce contrôle-ci garde la cause plutôt que le symptôme : il ouvre
/// chaque page, et aucune ne doit bouger.
///
/// IL INTERROGEAIT LA MACHINE EN LE FAISANT, et il a bronché une fois
/// pour cela, sous charge, le 28 octobre 2026. Ouvrir la fenêtre des
/// préférences JETTE le relevé des outils par conception — on a pu en
/// installer un dans un terminal depuis la dernière fois —, et l'image
/// suivante le refait en lançant VRAIMENT un processus par outil. Ce
/// contrôle mesurait donc une convergence pendant qu'il lançait des
/// processus, et ce que ces processus répondent n'appartient pas au
/// dépôt.
///
/// `no_control_asks_the_machine_what_it_should_be_checking` ne
/// l'attrapait pas : il cherche les contrôles qui APPELLENT
/// `tool_availability` et consorts, et ici c'est l'application qui
/// l'appelle pour lui. La règle valait quand même, et son remède est
/// celui qu'elle prescrit : semer l'état, après chaque image, puisque
/// c'est l'ouverture qui le jette.
#[test]
fn no_preferences_page_grows_by_itself() {
    /// L'état des outils, posé plutôt que demandé : tous absents, ce
    /// qui est le cas d'une machine nue et le seul qui ne dépende de
    /// rien. La page les peint alors tous de la même façon, et sa
    /// hauteur ne dépend plus que du dépôt.
    fn semence() -> std::collections::BTreeMap<String, namsdec_disasm::known::Availability> {
        let mut semés: std::collections::BTreeMap<_, _> = namsdec_disasm::known::all()
            .into_iter()
            .map(|outil| {
                (
                    outil.plugin.id.clone(),
                    namsdec_disasm::known::Availability::Missing,
                )
            })
            .collect();
        // upx ne désassemble rien, donc il n'est pas dans `all()` — mais
        // la page le montre, et un relevé qui l'oublie le ferait
        // redemander à la machine.
        semés.insert(
            "upx".to_owned(),
            namsdec_disasm::known::Availability::Missing,
        );
        semés
    }

    for onglet in PreferencesTab::ALL {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        app.show_preferences = true;
        app.preferences_tab = onglet;
        // UNE HORLOGE POSÉE, et non celle qu'egui se donne. Sans
        // `time`, egui avance la sienne de `predicted_dt` à chaque
        // image ; c'est reproductible, mais rien ne l'écrit, et ce
        // contrôle mesure précisément une CONVERGENCE — c'est-à-dire la
        // seule chose qu'une animation puisse troubler. Posée, l'image
        // numéro n tombe toujours au même instant.
        const PAS: f64 = 1.0 / 60.0;
        let entrée = écran_du_banc();

        let mut hauteurs: Vec<f32> = Vec::new();
        for image in 0..20 {
            // AVANT chaque image : l'ouverture de la fenêtre jette le
            // relevé pendant le dessin, et la page le redemanderait à
            // l'image suivante.
            app.tools_ready = semence();
            let entrée = egui::RawInput {
                time: Some(image as f64 * PAS),
                predicted_dt: PAS as f32,
                ..entrée.clone()
            };
            let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
            sortie.textures_delta.clear();
            if let Some(rect) = egui::AreaState::load(&ctx, egui::Id::unique("preferences_window"))
                .map(|a| a.rect())
            {
                hauteurs.push(rect.height());
            }
        }
        // Les huit dernières identiques : egui AMÈNE une fenêtre à sa
        // taille au lieu de l'y poser, ce qui prend quelques images. Une
        // mise en place qui converge n'est pas une dérive ; ce qu'on
        // refuse, c'est ce qui ne s'arrête jamais.
        let fin = &hauteurs[hauteurs.len().saturating_sub(8)..];
        for paire in fin.windows(2) {
            assert!(
                (paire[0] - paire[1]).abs() < 0.01,
                "la page « {} » fait grandir la fenêtre toute seule : {:?}",
                onglet.id(),
                hauteurs
            );
        }
    }
}

/// Le second bouton des préférences annule, et le dit.
///
/// Il s'est appelé « Quitter » tant que les réglages s'appliquaient à
/// mesure qu'on cochait : un bouton qui ne défait rien ne doit pas
/// prétendre défaire, et le commentaire du code le justifiait ainsi.
/// L'instantané est venu depuis — `close_preferences(ctx, false)` remet
/// le thème, la langue, les onglets et les raccourcis tels qu'ils
/// étaient en ouvrant — et le commentaire est resté pendant que le code
/// changeait sous lui. Le bouton défait, et portait encore le mot de
/// Fichier > Quitter : le même mot pour deux gestes dont l'un perd des
/// réglages et l'autre perd la séance.
///
/// Deux choses tenues ensemble, parce que l'une sans l'autre se
/// remettrait à mentir : ce que le bouton FAIT, et ce qu'il DIT.
#[test]
fn the_second_button_of_the_preferences_cancels_and_says_so() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let avant = (app.lang, app.show_re_tab, app.show_tooltips);

    app.open_preferences();
    app.lang = Lang::En;
    app.show_re_tab = !avant.1;
    app.show_tooltips = !avant.2;
    app.close_preferences(&ctx, false);

    assert_eq!(
        (app.lang, app.show_re_tab, app.show_tooltips),
        avant,
        "le second bouton des préférences remet les réglages tels qu'ils \
         étaient à l'ouverture"
    );

    // Et il ne porte pas le mot qui ferme l'application : « Quitter »
    // dans un dialogue de réglages se lit comme Fichier > Quitter.
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        assert_ne!(
            s.dialog_cancel, s.file_quit,
            "le bouton d'annulation des préférences ne doit pas porter le mot \
             qui ferme l'application"
        );
    }
}
