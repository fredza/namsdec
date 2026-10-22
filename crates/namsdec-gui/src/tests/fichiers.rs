//! Les fichiers : ouvrir, garder, retrouver, quitter.
//!
//! Plusieurs fichiers à la fois, les onglets, la session rouverte, les
//! réglages qui font l'aller-retour, et la question posée avant de perdre du
//! travail.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// Le sélecteur natif de `rfd` est bloquant : appelé depuis la frame,
/// il arrêtait la boucle d'évènements tout le temps que l'utilisateur
/// passait à chercher son fichier, et le compositeur finissait par
/// proposer de « forcer à quitter » une application qui attendait
/// simplement. Il tourne maintenant sur un thread : tant qu'il n'a pas
/// répondu, les frames continuent de s'enchaîner, et l'application
/// redemande elle-même à être réveillée — aucun évènement n'arrive
/// plus sur la fenêtre pendant qu'une boîte modale est ouverte.
#[test]
fn a_frame_runs_normally_while_a_dialog_is_open() {
    let (sender, result) = mpsc::channel();
    let mut app = NamsdecApp {
        pending_dialog: Some(PendingDialog {
            kind: FileDialogKind::OpenFile,
            result,
        }),
        ..Default::default()
    };

    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
    output.textures_delta.clear();

    assert!(
        app.pending_dialog.is_some(),
        "la boîte est toujours ouverte, on l'attend encore"
    );
    let repaint_delay = output
        .viewport_output
        .values()
        .map(|v| v.repaint_delay)
        .min()
        .expect("une frame produit toujours une sortie de viewport");
    assert!(
        repaint_delay <= DIALOG_POLL_INTERVAL,
        "la frame suivante doit venir d'elle-même, obtenu : {repaint_delay:?}"
    );

    // Et le chemin choisi est repris à la frame suivante, sans qu'une
    // touche ou un mouvement de souris ait à la provoquer.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("choisi.asm");
    std::fs::write(&path, "global _start\nsection .text\n_start:\n    hlt\n").unwrap();
    sender.send(Some(path.clone())).unwrap();

    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
    output.textures_delta.clear();

    assert!(app.pending_dialog.is_none(), "la boîte est refermée");
    assert_eq!(app.current_file.as_deref(), Some(path.as_path()));
    assert!(app.source.contains("hlt"), "obtenu : {:?}", app.source);
}

/// Une boîte refermée sans rien choisir ne doit toucher à rien : ni au
/// tampon, ni au fichier courant.
#[test]
fn a_cancelled_dialog_changes_nothing() {
    let (sender, result) = mpsc::channel();
    let mut app = NamsdecApp {
        pending_dialog: Some(PendingDialog {
            kind: FileDialogKind::OpenFile,
            result,
        }),
        ..Default::default()
    };
    let source = app.source.clone();
    sender.send(None).unwrap();

    app.poll_file_dialog(&egui::Context::default());

    assert!(app.pending_dialog.is_none());
    assert_eq!(app.current_file, None);
    assert_eq!(app.source, source);
}

/// Le dossier choisi ouvre l'explorateur dessus, comme le faisait
/// l'appel bloquant d'avant.
#[test]
fn a_picked_folder_opens_the_explorer_on_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = NamsdecApp::default();

    app.finish_file_dialog(
        FileDialogKind::OpenFolder,
        dir.path().to_path_buf(),
        &egui::Context::default(),
    );

    assert_eq!(app.explorer.root.as_deref(), Some(dir.path()));
    assert!(app.show_explorer, "l'explorateur s'ouvre sur ce dossier");
    assert!(app.explorer_focused, "et prend le clavier");
}

/// Le fichier sur lequel on travaillait se rouvre au lancement
/// suivant. C'est bien le **chemin** qui est enregistré : le contenu
/// relu est celui du disque, y compris s'il a changé entre-temps.
#[test]
fn the_last_file_is_reopened_at_startup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("travail.asm");
    std::fs::write(&path, "global _start\nsection .text\n_start:\n    hlt\n").unwrap();

    let settings = Settings {
        current_file: Some(path.display().to_string()),
        ..Default::default()
    };
    let app = NamsdecApp::from_settings(settings);

    assert_eq!(app.current_file.as_deref(), Some(path.as_path()));
    assert!(app.source.contains("hlt"), "obtenu : {:?}", app.source);

    // Modifié à l'extérieur : c'est la version du disque qui revient,
    // pas une copie gardée dans les réglages.
    std::fs::write(&path, "section .text\n    nop\n").unwrap();
    let app = NamsdecApp::from_settings(Settings {
        current_file: Some(path.display().to_string()),
        ..Default::default()
    });
    assert!(app.source.contains("nop"));
}

/// Un fichier déplacé ou supprimé entre deux lancements ne doit ni
/// empêcher de démarrer, ni accueillir l'utilisateur par une erreur.
#[test]
fn a_vanished_file_falls_back_to_the_example_program() {
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings {
        current_file: Some(dir.path().join("jamais-ecrit.asm").display().to_string()),
        ..Default::default()
    };

    let app = NamsdecApp::from_settings(settings);

    assert_eq!(app.current_file, None, "le chemin périmé est oublié");
    assert_eq!(app.source, DEFAULT_SOURCE);
    assert!(
        !matches!(app.status, Status::Error(_)),
        "démarrer ne doit pas afficher d'erreur, obtenu {:?}",
        app.status
    );
}

/// Le fichier ouvert part bien dans les réglages — et son contenu,
/// lui, n'y va jamais.
#[test]
fn settings_record_the_path_and_never_the_source() {
    let mut app = NamsdecApp {
        source: "section .text\n    mov rax, 1234\n".to_owned(),
        current_file: Some(PathBuf::from("/tmp/projet/travail.asm")),
        ..Default::default()
    };
    let snapshot = app.settings_snapshot();

    assert_eq!(
        snapshot.current_file.as_deref(),
        Some("/tmp/projet/travail.asm")
    );
    let written = serde_json::to_string(&snapshot).unwrap();
    assert!(
        !written.contains("1234"),
        "le programme ne doit pas se retrouver dans les réglages : {written}"
    );

    // Un tampon sans fichier (binaire désassemblé, exemple de départ)
    // n'enregistre aucun chemin.
    app.current_file = None;
    assert_eq!(app.settings_snapshot().current_file, None);
}

/// L'explorateur ne doit jamais s'ouvrir sur "Aucun dossier ouvert" :
/// il y a toujours le répertoire de travail à montrer, et une boîte
/// de dialogue de plus avant de voir l'arborescence n'a pas lieu
/// d'être.
#[test]
fn the_explorer_always_opens_on_a_folder() {
    let app = NamsdecApp::from_settings(Settings::default());
    assert!(
        app.explorer.root.is_some(),
        "au démarrage, sans réglage enregistré"
    );

    // Et si la racine disparaît en cours de route, la rouvrir en
    // retrouve une plutôt que d'afficher l'invite vide.
    let mut app = NamsdecApp::default();
    app.explorer.root = None;
    app.dispatch(Action::ToggleExplorer);
    assert!(app.show_explorer);
    assert!(
        app.explorer.root.is_some(),
        "ouvrir l'explorateur doit lui donner un dossier"
    );
}

/// Un dossier enregistré qui n'existe plus (projet déplacé, clé USB
/// retirée) ne doit pas laisser l'explorateur sur une racine morte.
#[test]
fn a_stale_saved_folder_falls_back_to_the_working_directory() {
    let settings = Settings {
        explorer_root: Some("/ce/chemin/n/existe/pas".to_owned()),
        ..Default::default()
    };
    let app = NamsdecApp::from_settings(settings);
    let root = app.explorer.root.expect("une racine de repli");
    assert!(root.is_dir(), "la racine de repli doit exister : {root:?}");
}

/// Les réglages ne s'écrivent que si on a dit où. C'est ce qui empêche
/// un test — qui dessine l'application, laquelle sauvegarde à chaque
/// frame — d'écraser le fichier de l'utilisateur avec les valeurs par
/// défaut d'une application neuve. C'est arrivé.
#[test]
fn an_application_that_was_not_told_where_to_write_writes_nowhere() {
    let app = NamsdecApp::default();
    assert!(
        app.settings_file.is_none(),
        "une application de test n'écrit nulle part"
    );

    // Dessinée, elle ne doit toucher à aucun fichier.
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let path = dir.path().join("settings.json");
    let mut app = NamsdecApp::default();
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.draw(ui));
    output.textures_delta.clear();
    assert!(!path.exists());

    // Le chemin donné, en revanche, le changement part sur le disque.
    app.settings_file = Some(path.clone());
    app.show_toolbar = !app.show_toolbar;
    app.persist_settings_if_changed();
    let written = Settings::load_from(&path);
    assert_eq!(written.show_toolbar, app.show_toolbar);
}

/// Un travail non enregistré ne disparaît pas parce qu'on a cliqué
/// ailleurs : l'outil demande d'abord.
#[test]
fn unsaved_work_is_not_thrown_away_without_asking() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    let fichier = dossier.path().join("autre.asm");
    std::fs::write(&fichier, "global _start\nsection .text\n_start:\nhlt\n").unwrap();

    let mut app = app_with_exercises();
    // Tampon intact : l'ouverture se fait sans rien demander.
    app.open_file(fichier.clone());
    assert!(app.pending.is_none());
    assert!(app.source.contains("_start"), "{}", app.source);

    // Modifié : la même action attend maintenant une réponse.
    app.source.push_str("    nop\n");
    let travail = app.source.clone();
    app.load_exercise("namsdec/registres");
    assert_eq!(
        app.pending,
        Some(Pending::Exercise("namsdec/registres".to_owned()))
    );
    assert_eq!(app.source, travail, "rien n'a bougé tant qu'on n'a pas dit");

    // Annuler laisse tout en place.
    app.pending = None;
    assert_eq!(app.source, travail);

    // Abandonner fait l'action demandée.
    app.load_exercise("namsdec/registres");
    let attendu = app.pending.clone().expect("en attente");
    app.pending = None;
    app.perform(attendu);
    assert_ne!(app.source, travail, "l'exercice a bien été chargé");
    assert!(!app.buffer_modified(), "et il part d'un tampon propre");

    // Enregistrer d'abord : le fichier reçoit le travail, puis l'action
    // se fait.
    app.open_file(fichier.clone());
    app.source.push_str("    hlt ; ajouté\n");
    let travail = app.source.clone();
    app.load_exercise("namsdec/boucle");
    assert!(app.pending.is_some());
    app.do_save();
    assert!(!app.buffer_modified());
    assert_eq!(std::fs::read_to_string(&fichier).unwrap(), travail);
}

/// Et la question se pose à l'écran, avec ses trois réponses : une boîte
/// à deux boutons forcerait à choisir entre perdre son travail et
/// renoncer à ce qu'on voulait faire.
#[test]
fn the_unsaved_question_offers_all_three_answers() {
    let mut app = NamsdecApp {
        pending: Some(Pending::Quit),
        ..Default::default()
    };
    app.source.push_str("    nop\n");

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let mut painted = String::new();
    for _ in 0..2 {
        painted.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                painted.push_str(text.galley.text());
                painted.push('|');
            }
        }
        output.textures_delta.clear();
    }

    let s = strings(Lang::Fr);
    for réponse in [s.unsaved_save, s.unsaved_discard, s.unsaved_cancel] {
        assert!(
            painted.contains(réponse),
            "« {réponse} » doit être proposé : {painted}"
        );
    }
    assert!(painted.contains(s.unsaved_title), "{painted}");
}

/// Tout réglage réglé par l'utilisateur doit revenir tel quel à la
/// session suivante.
///
/// Un réglage qu'on pose et qui ne revient pas est pire qu'un réglage
/// absent : on croit l'avoir posé. Le contrôle porte sur le tour
/// complet — l'état, écrit en JSON, relu, remis dans une application
/// neuve — et non sur chaque champ pris à part, parce que c'est le tour
/// complet qui casse quand on ajoute un champ en oubliant un bout.
#[test]
fn every_setting_survives_a_round_trip() {
    let mut app = NamsdecApp {
        lang: Lang::En,
        show_toolbar: false,
        show_sidebar: false,
        show_statusbar: false,
        show_console: false,
        show_tooltips: false,
        completion: false,
        auto_indent: false,
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        show_re_tab: true,
        memory_format: MemoryFormat::Text,
        ..Default::default()
    };
    app.disabled_packs.insert("namsdec".to_owned());
    app.current_exercise = Some("namsdec/tableau".to_owned());
    app.theme = namsdec_theme::all()
        .into_iter()
        .find(|theme| theme.id != app.theme.id)
        .expect("plus d'un thème");

    let json = serde_json::to_string(&app.settings_snapshot()).expect("écriture");
    let relu: crate::settings::Settings = serde_json::from_str(&json).expect("relecture");
    let revenu = NamsdecApp::from_settings(relu);

    assert_eq!(revenu.lang, app.lang);
    assert_eq!(revenu.show_toolbar, app.show_toolbar);
    assert_eq!(revenu.show_sidebar, app.show_sidebar);
    assert_eq!(revenu.show_statusbar, app.show_statusbar);
    assert_eq!(revenu.show_console, app.show_console);
    assert_eq!(revenu.show_tooltips, app.show_tooltips);
    assert_eq!(revenu.completion, app.completion);
    assert_eq!(revenu.auto_indent, app.auto_indent);
    assert_eq!(revenu.auto_pairs, app.auto_pairs);
    assert_eq!(revenu.program_args, app.program_args);
    assert_eq!(revenu.program_env, app.program_env);
    assert_eq!(revenu.active_tab, app.active_tab);
    assert_eq!(revenu.exercises, app.exercises);
    assert_eq!(revenu.show_re_tab, app.show_re_tab);
    assert_eq!(revenu.memory_format, app.memory_format);
    assert_eq!(revenu.disabled_packs, app.disabled_packs);
    assert_eq!(revenu.current_exercise, app.current_exercise);
    assert_eq!(revenu.theme.id, app.theme.id);

    // Un fichier écrit par une version plus ancienne, sans les réglages
    // récents : ils prennent leur valeur de départ, et rien ne casse.
    let ancien = r#"{"version":1,"theme":"vscode-dark","language":"fr",
        "show_toolbar":true,"show_sidebar":true,"show_statusbar":true,
        "show_console":true,"show_tooltips":true,"completion":true,
        "memory_format":"bytes"}"#;
    let relu: crate::settings::Settings = serde_json::from_str(ancien).expect("relecture");
    assert!(relu.auto_indent, "absent veut dire allumé");
    assert!(relu.current_exercise.is_none());
    assert!(relu.disabled_packs.is_empty());
}

/// Les fichiers récemment ouverts se retrouvent dans le menu, et le même
/// fichier n'y figure qu'une fois.
///
/// On retravaille le même fichier trois fois dans une séance, et le
/// sélecteur natif demande à chaque fois de retrouver son dossier.
#[test]
fn recently_opened_files_come_back_in_the_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = NamsdecApp::default();
    assert!(app.recent_files.is_empty());

    // Ouvrir en note le chemin, le plus récent en tête.
    let mut chemins = Vec::new();
    for index in 0..3 {
        let chemin = dir.path().join(format!("p{index}.asm"));
        std::fs::write(&chemin, "global _start\nsection .text\n_start:\n    hlt\n").unwrap();
        app.open_file(chemin.clone());
        chemins.push(chemin);
    }
    assert_eq!(app.recent_files.len(), 3);
    assert_eq!(app.recent_files[0], chemins[2], "le dernier ouvert en tête");

    // Rouvrir le premier le fait remonter, sans le doubler.
    app.open_file(chemins[0].clone());
    assert_eq!(app.recent_files.len(), 3);
    assert_eq!(app.recent_files[0], chemins[0]);

    // La liste ne grandit pas sans fin.
    for index in 0..12 {
        let chemin = dir.path().join(format!("autre{index}.asm"));
        std::fs::write(&chemin, "section .text\n").unwrap();
        app.open_file(chemin);
    }
    assert_eq!(app.recent_files.len(), NamsdecApp::RECENT_LIMIT);

    // Et elle survit au lancement suivant.
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    let relu = NamsdecApp::from_settings(serde_json::from_str(&écrit).unwrap());
    assert_eq!(relu.recent_files, app.recent_files);

    // Un fichier qui n'existe plus se refuse en le disant, plutôt que de
    // ressortir une copie d'hier.
    let disparu = dir.path().join("disparu.asm");
    app.open_file(disparu);
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
}

/// Plusieurs fichiers ouverts à la fois, chacun là où on l'a laissé.
///
/// C'est la promesse des onglets : on passe d'un fichier à l'autre sans
/// rien perdre — ni le texte tapé, ni le curseur, ni les points d'arrêt,
/// ni l'exécution arrêtée au pas où on l'avait laissée.
#[test]
fn several_files_stay_open_each_where_it_was_left() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let un = dir.path().join("un.asm");
    let deux = dir.path().join("deux.asm");
    std::fs::write(&un, DEFAULT_SOURCE).unwrap();
    std::fs::write(
        &deux,
        "global _start\nsection .text\n_start:\n    mov rax, 7\n    hlt\n",
    )
    .unwrap();

    let mut app = NamsdecApp::default();
    app.open_file(un.clone());
    assert_eq!(app.current_file.as_deref(), Some(un.as_path()));
    // Le premier fichier chasse le tampon vierge plutôt que de s'ajouter
    // à lui : on n'ouvre pas deux onglets en en demandant un.
    assert!(app.tabs.is_empty(), "un seul fichier, un seul onglet");

    // Le premier prend de l'avance : assemblé, un pas fait, un point
    // d'arrêt posé, le curseur quelque part.
    app.do_assemble();
    app.do_step();
    app.breakpoint_lines.insert(4);
    app.cursor_byte = 12;
    let rip_du_premier = app.emulator.as_ref().expect("chargé").cpu().regs.rip;

    app.open_file(deux.clone());
    assert_eq!(app.current_file.as_deref(), Some(deux.as_path()));
    assert_eq!(app.tabs.len(), 1, "le premier attend dans son onglet");
    assert_eq!(app.active_tab_index, 1, "le nouveau se range juste après");
    assert!(
        app.emulator.is_none() && app.breakpoint_lines.is_empty(),
        "le nouveau fichier part de rien, et non de l'état du précédent"
    );

    // On tape dans le second sans l'enregistrer, puis on repart.
    app.source.push_str("    ; une note\n");
    app.go_to_tab(0);
    assert_eq!(app.current_file.as_deref(), Some(un.as_path()));
    assert_eq!(
        app.cursor_byte, 12,
        "le curseur revient là où on l'avait laissé"
    );
    assert!(app.breakpoint_lines.contains(&4), "et les points d'arrêt");
    assert_eq!(
        app.emulator
            .as_ref()
            .expect("toujours chargé")
            .cpu()
            .regs
            .rip,
        rip_du_premier,
        "l'exécution reprend au pas où elle s'était arrêtée"
    );

    // Et le second n'a rien perdu non plus.
    app.go_to_tab(1);
    assert!(
        app.source.contains("; une note"),
        "ce qu'on avait tapé sans l'enregistrer est toujours là"
    );
    assert!(app.buffer_modified(), "et il se sait modifié");

    // Fermer un fichier modifié demande d'abord : c'est le seul endroit
    // d'où un travail peut disparaître sans qu'on l'ait voulu.
    app.close_tab(1);
    assert!(
        matches!(app.pending, Some(Pending::CloseTab(1))),
        "la question doit être posée : {:?}",
        app.pending
    );
    assert_eq!(app.tabs.len(), 1, "et rien n'est encore fermé");

    // « Ne pas enregistrer » : on ferme, et l'on revient au voisin.
    let en_attente = app.pending.take().expect("une question en attente");
    app.perform(en_attente);
    assert!(app.tabs.is_empty());
    assert_eq!(app.current_file.as_deref(), Some(un.as_path()));

    // LE DERNIER SE FERME AUSSI, et c'est un revirement demandé le
    // 25 septembre 2026. Il ne se fermait pas — « un éditeur sans
    // fichier n'a rien à montrer, et Fichier ▸ Nouveau est là pour
    // repartir de zéro » — mais le geste manquait là où on le cherche :
    // avec un seul fichier ouvert, l'entrée « Fermer le fichier »
    // disparaissait même du menu, faute d'onglet voisin.
    //
    // Fermer, c'est revenir à un tampon vierge : plus de chemin, plus de
    // texte. Vide et non le programme d'accueil — on a demandé à
    // fermer, pas à ouvrir autre chose.
    app.close_tab(0);
    assert_eq!(app.current_file, None, "le dernier fichier se ferme");
    assert!(app.source.is_empty(), "et il laisse un tampon vide");

    // Et un tampon déjà vide n'a plus rien à fermer : le geste ne
    // redemande pas une question à laquelle il n'y a rien à répondre.
    app.close_tab(0);
    assert!(app.pending.is_none(), "rien à fermer, rien à demander");
}

/// Les fichiers ouverts reviennent au lancement suivant.
#[test]
fn the_open_files_come_back_next_time() {
    let dir = tempfile::tempdir().unwrap();
    let un = dir.path().join("un.asm");
    let deux = dir.path().join("deux.asm");
    for chemin in [&un, &deux] {
        std::fs::write(chemin, "global _start\nsection .text\n_start:\n    hlt\n").unwrap();
    }

    let mut app = NamsdecApp::default();
    app.open_file(un.clone());
    app.open_file(deux.clone());
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(écrit.contains("open_files"), "{écrit}");

    let relu = NamsdecApp::from_settings(serde_json::from_str(&écrit).unwrap());
    assert_eq!(
        relu.current_file.as_deref(),
        Some(deux.as_path()),
        "on revient sur le fichier qu'on regardait"
    );
    assert_eq!(relu.tabs.len(), 1, "et l'autre est rouvert à côté");
    assert_eq!(relu.tabs[0].path.as_deref(), Some(un.as_path()));
    // Rouvrir n'est pas ouvrir : l'ordre des récents ne doit pas s'en
    // trouver bouleversé.
    assert_eq!(relu.recent_files, app.recent_files);
}

/// Un nom long se raccourcit par le milieu, jamais par la fin.
///
/// `boucle_imbriquee.asm` et `boucle_imbriquee_2.asm` coupés à la fin
/// donnent deux onglets identiques ; en gardant le début et la fin, on
/// garde l'extension et ce qui les distingue.
#[test]
fn a_long_file_name_is_shortened_in_the_middle() {
    assert_eq!(raccourcir_nom("court.asm", 18), "court.asm");
    let a = raccourcir_nom("boucle_imbriquee.asm", 18);
    let b = raccourcir_nom("boucle_imbriquee_2.asm", 18);
    assert_ne!(a, b, "deux noms voisins restent distincts : {a} / {b}");
    assert!(a.ends_with(".asm") && b.ends_with(".asm"), "{a} / {b}");
    assert!(
        a.chars().count() <= 18 && b.chars().count() <= 18,
        "{a} / {b}"
    );
    assert!(a.contains('…') && b.contains('…'));
}

/// La barre d'onglets montre les fichiers ouverts, et le point de ce qui
/// n'est pas enregistré.
#[test]
fn the_tab_bar_shows_the_open_files() {
    let dir = tempfile::tempdir().unwrap();
    let un = dir.path().join("un.asm");
    let deux = dir.path().join("deux.asm");
    for chemin in [&un, &deux] {
        std::fs::write(chemin, "global _start\nsection .text\n_start:\n    hlt\n").unwrap();
    }
    let mut app = NamsdecApp::default();
    app.open_file(un);
    app.open_file(deux);
    app.source.push_str("; modifié\n");

    let peint = paint(&mut app);
    assert!(peint.contains("un.asm"), "{peint}");
    assert!(peint.contains("deux.asm"), "{peint}");

    // LE POINT N'EST PLUS UN SUFFIXE mais un rond peint dans l'onglet,
    // à la place de sa croix — c'est ce que fait tout éditeur, et la
    // croix reparaît quand on s'approche. Le chercher dans le texte ne
    // donnerait plus rien ; on le cherche donc dans les formes.
    let ctx = egui::Context::default();
    let mut posé = false;
    for _ in 0..3 {
        let mut sortie = ctx.run_ui(écran_du_banc(), |ui| app.draw(ui));
        posé = sortie.shapes.iter().any(|c| {
            matches!(&c.shape, egui::epaint::Shape::Circle(rond)
                if (rond.radius - 3.5).abs() < 0.5 && rond.center.y < 120.0)
        });
        sortie.textures_delta.clear();
    }
    assert!(
        posé,
        "le rond dit, dans l'onglet, ce qui n'est pas enregistré"
    );

    // UN SEUL TAMPON, et la barre est là quand même.
    //
    // Elle ne l'était pas : « elle n'apprend rien tant qu'il n'y a rien
    // à choisir », disait ce test. C'est faux, et c'est même l'inverse —
    // l'écran d'accueil est le seul moment où personne ne sait encore
    // que les fichiers se rangent en onglets, donc le seul où la barre
    // a quelque chose à apprendre. Sans elle, rien ne le dit jamais.
    let mut seul = NamsdecApp::default();
    let ctx = egui::Context::default();
    let (_, accueil) = ce_qui_est_écrit(&ctx, &mut seul, None, 3);
    let onglet = accueil
        .iter()
        .filter(|(t, _)| t == strings(seul.lang).untitled_buffer)
        .any(|(_, r)| r.top() < 120.0);
    assert!(
        onglet,
        "la barre d'onglets est là dès l'accueil : {:?}",
        accueil.iter().map(|(t, _)| t).take(14).collect::<Vec<_>>()
    );
}

/// Quitter regarde tous les fichiers ouverts, pas seulement celui qu'on
/// voit.
///
/// Un onglet laissé de côté avec des modifications compte autant que le
/// tampon sous les yeux : la question doit être posée pour lui aussi, et
/// « Enregistrer » doit passer au suivant plutôt que d'emporter le
/// reste.
#[test]
fn quitting_looks_at_every_open_file() {
    let dir = tempfile::tempdir().unwrap();
    let un = dir.path().join("un.asm");
    let deux = dir.path().join("deux.asm");
    for chemin in [&un, &deux] {
        std::fs::write(chemin, "global _start\nsection .text\n_start:\n    hlt\n").unwrap();
    }
    let mut app = NamsdecApp::default();
    app.open_file(un.clone());
    app.source.push_str("; tapé dans le premier\n");
    app.open_file(deux.clone());

    // Le fichier visible est propre, l'autre non : quitter doit quand
    // même demander.
    assert!(!app.buffer_modified(), "celui qu'on voit est enregistré");
    assert!(app.anything_modified(), "mais pas celui d'à côté");
    assert_eq!(app.modified_tabs(), 1);
    app.guard(Pending::Quit);
    assert!(
        matches!(app.pending, Some(Pending::Quit)),
        "la question doit être posée : {:?}",
        app.pending
    );
    assert!(!app.quitting, "et l'on ne quitte pas avant d'avoir répondu");

    // Le premier onglet modifié se retrouve, pour que « Enregistrer »
    // enregistre le bon fichier.
    let rang = app.first_modified_tab().expect("un onglet en danger");
    app.go_to_tab(rang);
    assert_eq!(app.current_file.as_deref(), Some(un.as_path()));
    app.do_save();
    assert!(!app.anything_modified(), "plus rien à perdre");
    assert_eq!(app.first_modified_tab(), None);
}

/// L'onglet d'un binaire ouvert porte son nom, pas « sans titre ».
///
/// Un binaire chargé n'a pas de `path` — on ne le rouvre pas en
/// l'enregistrant par-dessus l'original — et l'onglet restait donc « sans
/// titre ». Son nom, celui du fichier d'où il vient, est pourtant connu :
/// il voyage dans `from_binary`. C'est lui qui s'écrit maintenant.
#[test]
fn l_onglet_d_un_binaire_porte_son_nom() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    let noms: Vec<String> = app.tab_labels().into_iter().map(|(nom, _)| nom).collect();
    assert!(
        noms.iter().any(|n| n == NOM_FABRIQUÉ),
        "l'onglet devrait porter le nom du binaire : {noms:?}"
    );
}

/// Un fichier changé sous nos pieds ne passe pas inaperçu.
///
/// `git checkout`, un autre éditeur, un `sed` : le fichier bouge pendant
/// qu'on regarde ailleurs. Rien de tapé de notre côté, on relit — c'est
/// ce que le fichier dit maintenant. Modifié des deux côtés, on ne
/// choisit pas à la place de l'utilisateur : on le dit, et il tranche.
#[test]
fn a_file_changed_on_disk_is_noticed_and_never_silently_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let chemin = dir.path().join("travail.asm");
    std::fs::write(&chemin, "global _start\nsection .text\n_start:\n    hlt\n").unwrap();

    let mut app = NamsdecApp::default();
    app.open_file(chemin.clone());
    assert!(app.file_mtime.is_some(), "la date du fichier est relevée");

    // Quelqu'un d'autre écrit dedans. La date doit changer : un système
    // de fichiers à la seconde près demande qu'on l'attende.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(
        &chemin,
        "global _start\nsection .text\n_start:\n    nop\n    hlt\n",
    )
    .unwrap();

    // Rien de tapé ici : on relit, et on le dit.
    app.check_disk_change();
    assert!(app.source.contains("nop"), "le tampon suit le disque");
    assert!(!app.disk_changed, "rien à trancher : {:?}", app.status);
    assert!(
        matches!(app.status, Status::Note(_)),
        "et le rechargement se dit : {:?}",
        app.status
    );
    assert!(
        app.assemble_state.is_none(),
        "ce qui était assemblé décrivait l'ancienne version"
    );

    // Cette fois, on a tapé quelque chose *et* le fichier a rebougé.
    app.source.push_str("    ; mon ajout\n");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(
        &chemin,
        "global _start\nsection .text\n_start:\n    xor rax, rax\n    hlt\n",
    )
    .unwrap();
    app.check_disk_change();
    assert!(
        app.disk_changed,
        "les deux versions comptent : il faut le signaler"
    );
    assert!(
        app.source.contains("; mon ajout"),
        "et surtout ne rien écraser sans le demander"
    );

    // Le bandeau le dit, avec de quoi trancher.
    let peint = paint(&mut app);
    assert!(peint.contains(strings(Lang::Fr).disk_reload), "{peint}");
    assert!(peint.contains(strings(Lang::Fr).disk_keep), "{peint}");

    // Recharger prend la version du disque, et l'avertissement s'en va.
    app.reload_from_disk();
    assert!(app.source.contains("xor rax, rax"), "{}", app.source);
    assert!(!app.source.contains("; mon ajout"));
    assert!(!app.disk_changed);

    // Enregistrer fait foi : la date relevée est celle qu'on vient
    // d'écrire, et l'avertissement ne revient pas sur notre propre
    // écriture.
    app.source.push_str("    ; encore un ajout\n");
    app.do_save();
    app.check_disk_change();
    assert!(!app.disk_changed, "notre propre écriture ne s'annonce pas");
    assert!(app.source.contains("; encore un ajout"));
}

/// On retrouve le travail commencé sur un binaire au lancement suivant.
///
/// Les sources se rouvraient depuis longtemps ; un binaire, non — et
/// c'est pourtant là qu'il y a du travail à perdre. Rouvrir namsdec
/// demandait de retrouver le fichier, de le recharger, de reposer ses
/// points d'arrêt et de retaper ses arguments.
#[test]
fn a_binary_session_comes_back_at_the_next_launch() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let dossier = tempfile::tempdir().unwrap();
    let binaire = dossier.path().join(NOM_FABRIQUÉ);
    std::fs::write(&binaire, &octets).unwrap();

    // Une séance : on ouvre le binaire, on pose des repères, on règle
    // des arguments.
    let mut app = NamsdecApp::default();
    app.open_file_now(binaire.clone());
    assert!(
        app.assemble_state.is_some(),
        "le binaire doit être chargé : {:?}",
        app.status
    );
    assert_eq!(app.binary_path.as_deref(), Some(binaire.as_path()));
    app.breakpoint_lines.insert(12);
    app.breakpoint_lines.insert(40);
    app.watchpoints.push((0x404000, 8));
    app.watched_registers.insert("RAX");
    app.program_args = "mot_de_passe".to_owned();
    app.active_tab = SidebarTab::Bytes;

    // On quitte : ce qui part sur le disque.
    let enregistré = app.settings_snapshot();
    assert_eq!(
        enregistré.open_binary.as_deref(),
        Some(binaire.display().to_string().as_str())
    );

    // On relance.
    // `from_settings` rouvre déjà la séance : c'est ce que fait le
    // lancement réel.
    let revenu = NamsdecApp::from_settings(enregistré);
    assert!(
        revenu.assemble_state.is_some(),
        "le binaire doit être rouvert : {:?}",
        revenu.status
    );
    assert_eq!(
        revenu.breakpoint_lines.iter().copied().collect::<Vec<_>>(),
        vec![12, 40],
        "les points d'arrêt reviennent"
    );
    assert_eq!(revenu.watchpoints, vec![(0x404000, 8)]);
    assert!(revenu.watched_registers.contains("RAX"));
    assert_eq!(revenu.program_args, "mot_de_passe");
    assert_eq!(revenu.active_tab, SidebarTab::Bytes);
}

/// Mais si le binaire a changé, les repères ne reviennent pas — et on le
/// dit.
///
/// Ils se comptent en numéros de ligne d'un listing reconstruit à chaque
/// ouverture. Sur un binaire recompilé entre-temps, ils désigneraient
/// d'autres instructions : poser des points d'arrêt au hasard est pire
/// que de ne pas en poser, et le faire en silence est pire encore.
#[test]
fn a_changed_binary_drops_its_marks_and_says_so() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let dossier = tempfile::tempdir().unwrap();
    let binaire = dossier.path().join(NOM_FABRIQUÉ);
    std::fs::write(&binaire, &octets).unwrap();

    let mut app = NamsdecApp::default();
    app.open_file_now(binaire.clone());
    app.breakpoint_lines.insert(12);
    let enregistré = app.settings_snapshot();

    // Le binaire est recompilé : même nom, autre contenu.
    let mut autre = octets.clone();
    autre.extend_from_slice(&[0u8; 4096]);
    std::fs::write(&binaire, &autre).unwrap();

    let revenu = NamsdecApp::from_settings(enregistré);
    assert!(
        revenu.assemble_state.is_some(),
        "le binaire lui-même se rouvre quand même"
    );
    assert!(
        revenu.breakpoint_lines.is_empty(),
        "mais pas ses points d'arrêt : {:?}",
        revenu.breakpoint_lines
    );
    assert!(
        matches!(revenu.status, Status::Note(_)),
        "et l'application le dit : {:?}",
        revenu.status
    );
}

/// `keymap.json` existe dès le premier lancement — mais jamais depuis un
/// test.
///
/// Il n'était posé qu'au premier changement de raccourci, si bien que
/// personne ne pouvait le lire pour savoir de quel jeu il partait. Le
/// garde-fou compte autant : construire une application dans un test ne
/// doit rien écrire, sinon la suite écraserait les raccourcis de la
/// machine — c'est `settings_file` qui fait la différence, et il est vide
/// partout sauf au vrai lancement.
#[test]
fn the_keymap_file_is_written_at_launch_and_never_by_a_test() {
    let mut app = NamsdecApp::default();
    assert!(
        app.settings_file.is_none(),
        "une application de test n'a pas de fichier de réglages"
    );
    // Sans fichier de réglages, l'écriture ne part pas : on peut donc
    // l'appeler ici sans toucher au disque de l'utilisateur.
    app.save_keymap_now();

    // Et le contenu qui serait écrit est bien celui du jeu courant.
    let keymap = settings::Keymap::from_bindings(app.keymap_preset, &app.keybindings);
    assert_eq!(
        keymap.preset(),
        keybindings::Preset::Vim,
        "le jeu de départ est celui de vim et gdb"
    );
    assert!(
        keymap.shortcuts.is_empty(),
        "et rien ne s'en écarte tant qu'on n'a rien changé : {:?}",
        keymap.shortcuts
    );
}

/// Tout réglage écrit est relu.
///
/// Un réglage qui part dans `settings.json` et que personne ne
/// reprend au lancement suivant est pire qu'un réglage absent : il
/// donne l'impression d'avoir été gardé. C'était le cas des points
/// d'arrêt d'un fichier source, écrits depuis toujours et relus
/// seulement pour un binaire.
///
/// Le contrôle est exhaustif par construction : il compare les deux
/// structures **champ par champ, en JSON**, si bien qu'un réglage
/// ajouté demain sans être relu fait échouer ce test sans qu'on ait
/// pensé à lui.
#[test]
fn every_setting_that_is_written_is_read_back() {
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let source = dir.path().join("prog.asm");
    std::fs::write(&source, "bits 64\nglobal _start\n_start:\n    hlt\n").unwrap();
    let date = std::fs::metadata(&source)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    // Chaque champ à une valeur qui n'est pas celle par défaut : un
    // réglage perdu ne peut donc pas se confondre avec un réglage
    // resté à sa valeur d'origine.
    let écrits = Settings {
        version: settings::SETTINGS_VERSION,
        theme: namsdec_theme::TOKYO_NIGHT.id.to_owned(),
        language: "en".to_owned(),
        show_toolbar: false,
        show_sidebar: false,
        show_statusbar: false,
        show_console: false,
        show_explorer: true,
        show_tooltips: false,
        flow_pseudo: true,
        completion: false,
        exercises: true,
        auto_indent: false,
        auto_pairs: false,
        hover_help: false,
        donation_url: "https://exemple.invalid/don".to_owned(),
        disabled_packs: vec!["une-serie".to_owned()],
        located_tools: std::collections::BTreeMap::new(),
        current_exercise: Some("namsdec/boucle".to_owned()),
        recent_files: vec![source.display().to_string()],
        exercises_done: vec!["namsdec/boucle".to_owned()],
        exercises_best: vec![("namsdec/boucle".to_owned(), 7)],
        disassembler: "iced".to_owned(),
        memory_format: "words".to_owned(),
        analysis_module: "dump".to_owned(),
        calc_base: "dec".to_owned(),
        calc_family: "tourner".to_owned(),
        calc_width: "32".to_owned(),
        heat_legend: false,
        flow_current_only: false,
        timeline_notable_only: true,
        console_env_open: true,
        system_titlebar: true,
        console_height: Some(321),
        sidebar_tab_height: Some(222),
        re_tab: true,
        decompiler_module: true,
        installer_module: true,
        decompiler: "dzf".to_owned(),
        memory_follow: "rsp".to_owned(),
        // Les trois champs du binaire demandent un ELF sur le disque
        // pour revenir ; leur reprise a son propre test.
        open_binary: None,
        open_binary_len: None,
        open_binary_mtime: None,
        breakpoints: vec![3],
        // Dans les quatre lignes du fichier écrit plus haut : une ligne
        // qui n'existe pas serait écartée à la reprise, et c'est voulu.
        listing_line: Some(3),
        watchpoints: vec![(0x1000, 4)],
        watched_registers: vec!["RAX".to_owned()],
        program_args: "-v --long".to_owned(),
        program_env: "A=1".to_owned(),
        sidebar_tab: Some("heap".to_owned()),
        open_windows: vec!["calculator".to_owned(), "timeline".to_owned()],
        explorer_root: Some(dir.path().display().to_string()),
        current_file: Some(source.display().to_string()),
        current_file_mtime: date,
        open_files: Vec::new(),
        shortcuts: Settings::default().shortcuts,
    };

    let relus = NamsdecApp::from_settings(écrits.clone()).settings_snapshot();
    let avant = serde_json::to_value(&écrits).unwrap();
    let après = serde_json::to_value(&relus).unwrap();
    let mut perdus = Vec::new();
    for (clé, valeur) in avant.as_object().unwrap() {
        let revenu = après.as_object().unwrap().get(clé);
        if revenu != Some(valeur) {
            perdus.push(format!("{clé} : {valeur} devient {revenu:?}"));
        }
    }
    assert!(
        perdus.is_empty(),
        "{} réglage(s) écrit(s) puis perdu(s) :\n  {}",
        perdus.len(),
        perdus.join("\n  ")
    );
}

/// On voit qu'on peut ouvrir plusieurs fichiers, dès le premier écran.
///
/// On le pouvait depuis longtemps : « Ouvrir un fichier… » range le
/// second dans un onglet. Mais la barre d'onglets n'apparaissait qu'à
/// partir de DEUX fichiers ouverts, si bien que rien, jamais, ne le
/// disait — il fallait l'avoir deviné pour s'en servir. Un mécanisme
/// qu'on ne découvre qu'en l'ayant déjà employé n'existe pas.
///
/// Premier remède, insuffisant : la montrer dès le premier fichier
/// OUVERT. Or l'écran d'accueil n'en a pas — il porte un tampon « sans
/// titre » —, si bien que la barre restait invisible exactement là où
/// elle enseigne quelque chose. Et ce contrôle AFFIRMAIT que c'était
/// normal : mon erreur, posée en invariant. Il a fallu que Fred regarde
/// son écran et n'y voie rien.
///
/// La leçon tient en une ligne : j'avais vérifié sur une séance où un
/// fichier était ouvert, c'est-à-dire dans le seul cas qui marchait
/// déjà. Vérifier dans l'état où le défaut a été signalé, pas dans
/// celui qui arrange.
#[test]
fn the_tab_bar_and_its_plus_are_there_from_the_first_screen() {
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let un = dir.path().join("un.asm");
    std::fs::write(&un, "bits 64\nglobal _start\n_start:\n    hlt\n").unwrap();

    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);

    // DÈS L'ÉCRAN D'ACCUEIL, sur le tampon « sans titre ».
    //
    // La première version de ce contrôle affirmait le contraire — « sans
    // fichier ouvert, la barre n'a rien à montrer » — et c'était mon
    // erreur mise en invariant. L'écran d'accueil est justement celui où
    // personne ne sait encore qu'on peut ouvrir plusieurs fichiers ;
    // cacher là ce qui l'enseigne revient à ne rien enseigner. Fred a
    // regardé son écran et n'a rien vu, ce que ce test affirmait normal.
    let (_, accueil) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
    assert!(
        accueil
            .iter()
            .any(|(t, _)| t == strings(app.lang).untitled_buffer),
        "le tampon sans titre se montre comme un onglet, dès l'accueil"
    );
    // Le « + » FAIT un programme de plus.
    //
    // Il a d'abord mené au sélecteur de fichiers, puis à un menu de deux
    // entrées. Les deux fois c'était trop : dans une barre d'onglets,
    // « + » veut dire « un de plus », un point c'est tout.
    //
    // Le contrôle porte sur le GESTE et non sur le dessin : la croix et
    // le plus sont des traits peints, pas des textes, et un test qui
    // chercherait « + » dans ce qui est écrit ne trouverait rien.
    assert!(app.tabs.is_empty(), "un seul tampon au départ");
    let avant = app.tab_labels().len();
    app.dispatch(Action::New);
    assert_eq!(
        app.tab_labels().len(),
        avant + 1,
        "« Nouveau programme » ajoute un onglet, même depuis un tampon vierge"
    );
    let noms: Vec<String> = app.tab_labels().into_iter().map(|(n, _)| n).collect();
    assert!(
        noms.iter().any(|n| n.ends_with(" 2")),
        "et les tampons sans titre se numérotent : {noms:?}"
    );

    // Un fichier ouvert se nomme dans la barre, à côté des autres.
    app.open_file(un.clone());
    let (_, après) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
    let dans_la_barre = après
        .iter()
        .filter(|(t, _)| t == "un.asm")
        .any(|(_, r)| r.top() < 120.0);
    assert!(
        dans_la_barre,
        "le fichier ouvert se nomme dans la barre d'onglets — {:?}",
        après.iter().map(|(t, _)| t).take(14).collect::<Vec<_>>()
    );
}

/// Un fichier que namsdec réécrit n'est jamais laissé à moitié remplacé.
///
/// `fs::write` tronque puis écrit : une coupure entre les deux laissait
/// un `settings.json` vide — donc illisible, donc remplacé par les
/// défauts au lancement suivant, en silence. Le même piège guettait les
/// raccourcis, les noms donnés aux fonctions d'un binaire, et le source
/// de l'utilisateur.
///
/// On ne peut pas couper le courant dans un contrôle ; on vérifie donc ce
/// qui en découle : le remplacement a bien eu lieu, et rien de provisoire
/// ne traîne à côté — un `.tmp` oublié dans le dossier de configuration
/// serait la trace d'un renommage qui n'a pas eu lieu.
#[test]
fn un_fichier_reecrit_est_remplace_d_un_bloc() {
    let dossier = tempfile::tempdir().expect("un dossier temporaire");
    let cible = dossier.path().join("creux").join("reglages.json");

    // Le dossier manquant est créé au passage.
    crate::write_atomically(&cible, b"premier").expect("premiere ecriture");
    assert_eq!(std::fs::read_to_string(&cible).unwrap(), "premier");

    // Remplacement par un contenu PLUS COURT : c'est le cas qui révélait
    // une troncature mal faite, l'ancien texte dépassant du nouveau.
    crate::write_atomically(&cible, b"court").expect("remplacement");
    assert_eq!(std::fs::read_to_string(&cible).unwrap(), "court");

    // Et plus long, pour l'autre sens.
    crate::write_atomically(&cible, b"un contenu nettement plus long").expect("remplacement");
    assert_eq!(
        std::fs::read_to_string(&cible).unwrap(),
        "un contenu nettement plus long"
    );

    // Rien de provisoire n'est resté dans le dossier.
    let restes: Vec<String> = std::fs::read_dir(cible.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|nom| nom != "reglages.json")
        .collect();
    assert!(
        restes.is_empty(),
        "un fichier provisoire a survécu au renommage : {restes:?}"
    );

    // Une écriture qui échoue laisse l'ancien contenu intact : on vise un
    // chemin dont le parent est un FICHIER, ce qu'aucun dossier ne peut
    // être — l'ancien fichier, lui, ne doit pas en souffrir.
    let impossible = cible.join("sous-le-fichier.json");
    assert!(crate::write_atomically(&impossible, b"perdu").is_err());
    assert_eq!(
        std::fs::read_to_string(&cible).unwrap(),
        "un contenu nettement plus long",
        "une écriture impossible ne doit toucher à rien"
    );
}

/// Tout réglage que l'interface bascule se retrouve dans `settings.json`.
///
/// La règle est posée : un paramètre réglé par l'utilisateur est gardé
/// d'une séance à l'autre. Quatre y échappaient — l'explorateur de
/// fichiers (alors que la console, la barre latérale et la barre d'état,
/// ses frères, se souvenaient), le filtre du graphe de flot, celui du
/// déroulement, et le repli des variables d'environnement. On les
/// réglait, on relançait, ils étaient revenus comme avant.
///
/// Le contrôle lit le code qui bascule — `checkbox(&mut self.x)`,
/// `self.x = !self.x` — et exige que `x` soit un champ de `Settings`. Un
/// réglage nouveau entre donc sous la règle sans qu'on y pense.
#[test]
fn tout_reglage_bascule_par_l_interface_est_garde() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // Les bascules qui ne SONT pas des réglages : à inscrire ici avec la
    // raison, plutôt que d'élargir les motifs jusqu'à ne plus rien voir.
    const EPHEMERES: [(&str, &str); 0] = [];

    let mut bascules: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // `src/` est le dépôt lui-même : illisible, c'est une anomalie, non
    // une machine qui n'a pas l'outil. Le relevé vide passerait au vert
    // en ne regardant aucune bascule.
    let entrees = std::fs::read_dir(&src).expect("les sources de l'interface");
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.extension().is_none_or(|e| e != "rs")
            || chemin.file_name().is_some_and(|n| n == "tests.rs")
        {
            continue;
        }
        let Ok(texte) = std::fs::read_to_string(&chemin) else {
            continue;
        };
        for (motif, apres) in [
            ("checkbox(&mut self.", ','),
            ("toggle_value(&mut self.", ','),
        ] {
            let mut reste = texte.as_str();
            while let Some(i) = reste.find(motif) {
                reste = &reste[i + motif.len()..];
                if let Some(fin) = reste.find([apres, ')']) {
                    let nom = reste[..fin].trim();
                    if !nom.is_empty() && nom.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        bascules.insert(nom.to_owned());
                    }
                }
            }
        }
        // `self.x = !self.x` : la bascule écrite à la main.
        let mut reste = texte.as_str();
        while let Some(i) = reste.find("= !self.") {
            let avant = &reste[..i];
            reste = &reste[i + "= !self.".len()..];
            let Some(gauche) = avant.rsplit("self.").next() else {
                continue;
            };
            let nom = gauche.trim();
            if nom.chars().all(|c| c.is_alphanumeric() || c == '_') && !nom.is_empty() {
                bascules.insert(nom.to_owned());
            }
        }
    }
    assert!(
        bascules.len() >= 15,
        "le relevé n'a trouvé que {} bascules : les motifs ne mordent plus",
        bascules.len()
    );

    // On interroge la table de correspondance, et non les noms de champs :
    // `show_re_tab` est gardé sous le nom `re_tab`, et chercher
    // « pub show_re_tab » l'aurait déclaré oublié à tort. Ce qui prouve
    // qu'un réglage est gardé, c'est qu'il est LU des réglages et ÉCRIT
    // dans l'instantané — les deux sens, dans session.rs.
    let session = std::fs::read_to_string(src.join("session.rs")).expect("session.rs");
    let oublies: Vec<&String> = bascules
        .iter()
        .filter(|nom| !EPHEMERES.iter().any(|(e, _)| *e == nom.as_str()))
        .filter(|nom| {
            let lu = session.contains(&format!("{nom}: settings."));
            let ecrit = session.contains(&format!(": self.{nom},"));
            !(lu && ecrit)
        })
        .collect();
    assert!(
        oublies.is_empty(),
        "{} réglage(s) que l'interface bascule mais que settings.json ne garde pas : {oublies:?}\n\n\
         Ajoutez le champ à `Settings` (avec son défaut serde si le défaut n'est pas `false`) \
         et sa lecture/écriture dans session.rs, dans les DEUX sens — ou inscrivez-le dans \
         EPHEMERES en disant pourquoi ce n'en est pas un.",
        oublies.len()
    );
}

/// Préférences › Raccourcis montre les **deux** jeux, action par action.
///
/// L'onglet n'en montrait qu'un, et c'est précisément ce qui empêchait de
/// voir les trous et les doublons : on ne compare pas de mémoire deux
/// listes de trente lignes. Côte à côte, « cette action n'a rien dans
/// l'autre jeu » et « ces deux-là répondent à la même frappe » se lisent
/// sans chercher.
#[test]
fn the_shortcuts_tab_shows_both_sets_side_by_side() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.show_preferences = true;
    app.preferences_tab = PreferencesTab::Shortcuts;

    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("preferences_window"), 4);
    let textes: Vec<&str> = lignes.iter().map(|(t, _)| t.as_str()).collect();

    // Les deux jeux sont nommés en tête de colonne.
    for preset in crate::keybindings::Preset::ALL {
        assert!(
            textes.contains(&preset.label(true)),
            "« {} » doit nommer sa colonne : {textes:?}",
            preset.label(true)
        );
    }

    // Et pour une action, les frappes des DEUX jeux sont écrites. « Pas
    // dedans » est le cas d'école : `Space P I` d'un côté, `F11` de
    // l'autre.
    let vim = crate::keybindings::KeyBindings::preset(crate::keybindings::Preset::Vim);
    let classic = crate::keybindings::KeyBindings::preset(crate::keybindings::Preset::Classic);
    for jeu in [&vim, &classic] {
        let frappe = jeu
            .map
            .get(&Action::StepInto)
            .expect("« pas dedans » a un raccourci dans les deux jeux")
            .display(&ctx);
        assert!(
            textes.contains(&frappe.as_str()),
            "la frappe « {frappe} » doit être écrite : {textes:?}"
        );
    }

    // Quand les deux jeux disent la même chose, on l'écrit au lieu de le
    // recopier : deux fois la même frappe côte à côte n'apprend rien, et
    // c'est ce qui DIFFÈRE qu'on vient lire. « Enregistrer » est
    // Ctrl+S des deux côtés.
    let s = strings(app.lang);
    assert_eq!(
        vim.map.get(&Action::Save),
        classic.map.get(&Action::Save),
        "« enregistrer » est la même frappe dans les deux jeux"
    );
    assert!(
        textes.contains(&s.shortcuts_same),
        "« {} » doit remplacer la répétition : {textes:?}",
        s.shortcuts_same
    );
    let ctrl_s = vim
        .map
        .get(&Action::Save)
        .expect("« enregistrer » a un raccourci")
        .display(&ctx);
    assert_eq!(
        textes.iter().filter(|t| **t == ctrl_s).count(),
        1,
        "et elle n'est écrite qu'une fois : {textes:?}"
    );
}
