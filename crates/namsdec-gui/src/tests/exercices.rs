//! L'Atelier : les exercices, le cours, les buts, les budgets.
//!
//! Un exercice doit atteindre son but tel qu'il est livré, dire ce qui
//! manque quand il échoue, et coûter ce qu'il annonce. Les greffons de
//! paquets sont ici aussi.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// Le module des exercices ne se charge que si on le demande : sans la
/// case cochée, rien de lui n'existe dans l'interface.
#[test]
fn the_exercises_module_stays_out_of_the_way_until_it_is_asked_for() {
    let app = NamsdecApp::default();
    assert!(!app.exercises, "éteint au premier lancement");

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let painted = |app: &mut NamsdecApp| {
        let mut texts = String::new();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                texts.push_str(text.galley.text());
                texts.push('|');
            }
        }
        output.textures_delta.clear();
        texts
    };

    let mut app = NamsdecApp {
        show_exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    assert!(
        !painted(&mut app).contains("Une boucle qui compte"),
        "la fenêtre ne s'ouvre pas tant que le module est éteint"
    );

    app.exercises = true;
    // Deux frames : egui pose la fenêtre à la première, la peint à la
    // seconde.
    painted(&mut app);
    assert!(
        painted(&mut app).contains("Une boucle qui compte"),
        "et elle s'ouvre quand il est allumé"
    );
}

/// Le choix est gardé d'une session à l'autre — contrairement à l'onglet
/// RE, c'est une case qu'on coche une fois.
#[test]
fn asking_for_the_exercises_is_remembered() {
    let app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    let written = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(written.contains("\"exercises\":true"), "{written}");

    // Et un fichier de réglages écrit par une version plus ancienne, qui
    // ne connaît pas le réglage, ne fait pas échouer la lecture.
    let older = r#"{"version":2,"theme":"vscode-dark","language":"fr","show_toolbar":true,
        "show_sidebar":true,"show_statusbar":true,"show_console":true,"show_tooltips":true,
        "completion":true,"memory_format":"bytes","explorer_root":null,"current_file":null,
        "shortcuts":{}}"#;
    let settings: Settings = serde_json::from_str(older).expect("les anciens réglages se relisent");
    // ET LE MODULE RESTE ÉTEINT POUR CELUI-LÀ. Le parcours est allumé
    // par défaut depuis le 16 novembre 2026 — une installation neuve
    // doit trouver ce que namsdec a de plus pédagogique —, mais une
    // configuration d'avant le mode débutant appartient à quelqu'un qui
    // écrit son propre code : la migration le lui laisse (version < 3).
    // Le champ absent prend le défaut ; c'est la migration qui décide
    // ensuite, et elle est éprouvée dans `settings.rs`.
    assert!(settings.exercises);
    // Une installation neuve, elle, ouvre sur le parcours.
    assert!(
        Settings::default().exercises,
        "le premier écran doit mener au parcours"
    );
}

/// Charger un exercice met son programme dans l'éditeur, et rien de plus :
/// pas de fichier courant, donc « Enregistrer » n'écrase pas celui qu'on
/// avait ouvert.
#[test]
fn loading_an_exercise_fills_the_editor_without_claiming_a_file() {
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        current_file: Some(std::path::PathBuf::from("/tmp/le-mien.asm")),
        ..Default::default()
    };
    app.load_exercise("namsdec/boucle");

    assert!(app.source.contains("boucle"), "le programme est chargé");
    assert!(
        app.source.contains("global _start"),
        "et c'est un programme complet"
    );
    assert!(app.current_file.is_none(), "un modèle n'est pas un fichier");
    assert!(app.emulator.is_none(), "et rien n'est encore assemblé");

    // Un identifiant inconnu ne casse rien et ne touche pas au tampon.
    let before = app.source.clone();
    app.load_exercise("namsdec/n-existe-pas");
    assert_eq!(app.source, before);
}

/// Et la commande qui ouvre les exercices n'existe que si le module est
/// chargé : proposer une commande qui n'ouvre rien n'aide personne.
#[test]
fn the_palette_offers_the_exercises_only_when_the_module_is_loaded() {
    let s = strings(Lang::Fr);
    let labels = |exercises| -> Vec<String> {
        palette_commands(s, false, exercises, namsdec_re::Niveau::Avancé)
            .into_iter()
            .map(|(label, _)| label)
            .collect()
    };
    assert!(!labels(false).contains(&s.exercises_open.to_string()));
    assert!(labels(true).contains(&s.exercises_open.to_string()));
}

/// Une série installée depuis le disque s'ajoute à celle qui est livrée,
/// sans la remplacer — et chaque exercice reste identifiable par sa
/// série, deux d'entre elles pouvant nommer le leur pareil.
#[test]
fn an_installed_pack_joins_the_built_in_one() {
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let path = dir.path().join("mes-exos.json");
    std::fs::write(
        &path,
        r#"{
            "id": "mes-exos",
            "name": "Mes exercices",
            "exercises": [
                {
                    "id": "boucle",
                    "title": "Le mien",
                    "description": "À moi.",
                    "code": "BITS 64\nsection .text\n    hlt\n"
                }
            ]
        }"#,
    )
    .unwrap();

    // Le dossier de données est celui du test : rien n'est écrit chez
    // l'utilisateur.
    let data = dir.path().join("data");
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    with_data_home(&data, || app.install_exercises(&path));
    let note = app.plugin_note.as_ref().expect("un compte rendu");
    assert!(note.ok, "installation refusée : {}", note.text);
    assert!(
        note.text.starts_with("Série installée"),
        "une série n'est pas un thème : {}",
        note.text
    );
    assert_eq!(
        app.exercise_packs.len(),
        2,
        "la série livrée est toujours là"
    );
    assert!(data.join("namsdec/exercises/mes-exos.json").is_file());

    // Les deux « sum_loop » ne se confondent pas.
    app.load_exercise("mes-exos/boucle");
    assert!(app.source.contains("hlt"));
    assert!(!app.source.contains("add rax"), "c'est bien le mien");
    app.load_exercise("namsdec/boucle");
    assert!(
        app.source.contains("Une boucle qui compte"),
        "et celui d'origine aussi : {}",
        app.source
    );

    // Retirée, elle disparaît de la liste et du disque.
    with_data_home(&data, || app.remove_exercises("mes-exos"));
    assert_eq!(app.exercise_packs.len(), 1);
    assert!(!data.join("namsdec/exercises/mes-exos.json").exists());
    assert!(namsdec_exercises::find(&app.exercise_packs, "mes-exos/boucle").is_none());
}

/// Une série qui se donne l'identifiant de celle d'origine est refusée :
/// sans quoi elle ferait disparaître les exercices livrés sans que
/// personne ne l'ait demandé.
#[test]
fn a_pack_cannot_take_the_name_of_the_built_in_one() {
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let path = dir.path().join("faux.json");
    std::fs::write(
        &path,
        r#"{"id":"namsdec","name":"Faux","exercises":[
            {"id":"a","title":"A","description":"B","code":"BITS 64\nhlt\n"}]}"#,
    )
    .unwrap();
    let mut app = app_with_exercises();
    with_data_home(&dir.path().join("data"), || app.install_exercises(&path));
    assert!(
        app.plugin_note.as_ref().is_some_and(|n| !n.ok),
        "elle doit être refusée"
    );
    assert_eq!(
        app.exercise_packs.len(),
        1,
        "la série livrée reste seule : l'usurpatrice n'a pas pris sa place"
    );
    assert!(app.exercise_files.is_empty());
}

/// Un fichier qui n'est pas une série est refusé en le disant, dans la
/// langue de l'interface, et rien n'est copié.
#[test]
fn a_file_that_is_not_a_pack_is_refused_in_plain_words() {
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let path = dir.path().join("pas-une-serie.json");
    std::fs::write(&path, "{ ceci n'est pas du JSON").unwrap();
    let data = dir.path().join("data");
    let mut app = NamsdecApp::default();
    with_data_home(&data, || app.install_exercises(&path));
    let note = app.plugin_note.as_ref().expect("un compte rendu");
    assert!(!note.ok);
    assert!(
        note.text
            .starts_with("ce n'est pas un fichier d'exercices valide"),
        "{}",
        note.text
    );
    assert!(!data.join("namsdec/exercises").exists(), "rien n'est copié");
}

/// Une série s'écarte sans se désinstaller : le fichier reste, son
/// contenu cesse d'être proposé. Y compris celle qui est livrée avec
/// l'application, qui ne s'enlève pas mais se range.
#[test]
fn a_pack_can_be_set_aside_without_being_uninstalled() {
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        show_exercises: true,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let painted = |app: &mut NamsdecApp| {
        let mut texts = String::new();
        for _ in 0..2 {
            texts.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &clipped.shape {
                    texts.push_str(text.galley.text());
                    texts.push('|');
                }
            }
            output.textures_delta.clear();
        }
        texts
    };

    assert!(painted(&mut app).contains("Une boucle qui compte"));

    app.disabled_packs
        .insert(namsdec_exercises::BUILTIN_ID.to_owned());
    assert!(
        !painted(&mut app).contains("Une boucle qui compte"),
        "écartée, la série ne se propose plus"
    );

    // Mais elle est toujours là, et le choix part dans les réglages.
    assert_eq!(app.exercise_packs.len(), 1, "rien n'a été désinstallé");
    let written = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(
        written.contains("\"disabled_packs\":[\"namsdec\"]"),
        "{written}"
    );
}

/// Les trois étapes ajoutées au parcours tiennent ce que leur énoncé
/// promet : le but s'atteint vraiment, dans cet émulateur-ci.
///
/// Un exercice dont le but est inatteignable n'apprend qu'une chose :
/// que l'outil ment. Le programme de départ est donc exécuté ici, puis
/// corrigé comme l'énoncé le demande, et les registres vérifiés.
#[test]
fn the_new_steps_of_the_course_keep_their_promise() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    fn exécute(source: &str) -> NamsdecApp {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "l'exercice doit s'assembler : {:?}",
            app.status
        );
        app.do_run();
        for _ in 0..50 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
        app
    }
    let pack = builtin_pack();
    // DEUX LECTURES, ET ELLES NE DISENT PAS LA MÊME CHOSE depuis le
    // 11 novembre 2026 : le code LIVRÉ a un trou — c'est ce qui en fait
    // un exercice —, la SOLUTION est le programme entier. Ce qui est
    // promis se vérifie sur la seconde ; ce que l'élève voit en
    // ouvrant, sur la première.
    let fini = |id: &str| programme_éprouvé(pack.by_id(id).expect("l'étape existe")).to_owned();
    let livré = |id: &str| pack.by_id(id).expect("l'étape existe").starter_code.clone();

    // L'ordre des octets : le premier en mémoire est celui de poids
    // faible, le dernier celui de poids fort.
    let app = exécute(&fini("boutisme"));
    let regs = &app.emulator.as_ref().expect("chargé").cpu().regs;
    assert_eq!(regs.get64(Reg64::Rax), 0x1122334455667788);
    assert_eq!(regs.get64(Reg64::Rcx), 0x88);
    assert_eq!(regs.get64(Reg64::Rdx), 0x11);

    // Signé ou non : sur la même comparaison, `ja` saute et `jg` non.
    let app = exécute(&fini("signe"));
    let regs = &app.emulator.as_ref().expect("chargé").cpu().regs;
    assert_eq!(
        regs.get64(Reg64::Rcx),
        1,
        "non signé : -1 est le plus grand de tous"
    );
    assert_eq!(regs.get64(Reg64::Rdx), 0, "signé : -1 est plus petit que 1");

    // La division part sur un refus — c'est l'énoncé —, et la
    // correction que l'énoncé demande donne le quotient et le reste.
    let app = exécute(&livré("division"));
    assert!(
        matches!(app.status, Status::Error(_)),
        "le quotient ne tient pas dans 64 bits : {:?}",
        app.status
    );
    // Et le refus dit quoi faire : l'exercice se corrige sans aide
    // extérieure.
    let Status::Error(message) = &app.status else {
        unreachable!()
    };
    assert!(message.contains("moitié haute du dividende"), "{message}");
    let corrigé = fini("division");
    let app = exécute(&corrigé);
    let regs = &app.emulator.as_ref().expect("chargé").cpu().regs;
    assert_eq!(regs.get64(Reg64::Rax), 7, "le quotient de 45 par 6");
    assert_eq!(regs.get64(Reg64::Rdx), 3, "et son reste");
}

/// Chaque exercice montre les fiches dont son programme se sert : le
/// manuel est à côté de l'énoncé, pas à chercher.
#[test]
fn an_exercise_points_at_the_sheets_it_uses() {
    // Les mots lus sont ceux du code, pas des commentaires, et une
    // étiquette n'est pas une instruction.
    let fiches = sheets_used(
        "; div n'est pas ici\n_start:\n    mov rax, 45\n    xor rdx, rdx\n    div rcx\n",
    );
    // `xor` s'affiche « xor », même si sa fiche s'appelle « and » : on
    // cherche le mot qu'on a sous les yeux.
    assert_eq!(fiches, vec!["mov", "xor", "div"]);

    // Chaque étape du parcours en a au moins une : un exercice sans
    // fiche serait un exercice sur rien.
    for exercise in &builtin_pack().exercises {
        assert!(
            !sheets_used(&exercise.starter_code).is_empty(),
            "{} ne renvoie à aucune fiche",
            exercise.id
        );
    }

    // Et elles s'affichent sous l'énoncé.
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        show_exercises: true,
        ..Default::default()
    };
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).exercises_sheets),
        "{painted}"
    );
}

/// Le parcours livré est un parcours : quarante-quatre étapes dans l'ordre où
/// l'on apprend, chacune avec son but écrit dans le programme.
///
/// L'ordre n'est pas décoratif. On lit les registres avant d'écrire une
/// boucle, on sait boucler avant de parcourir un tableau, et l'on a vu
/// une chaîne avant de l'écrire sur la sortie.
#[test]
fn the_built_in_pack_is_a_course_in_order() {
    let pack = builtin_pack();
    let ids: Vec<&str> = pack.exercises.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "registres",
            "echange",
            "zero",
            "boucle",
            "compteur",
            "bits",
            "comparer",
            "drapeaux",
            "signe",
            "negatif",
            "hexadecimal",
            "incrementer",
            "test_and",
            "xchg_bswap",
            "masques",
            "constantes",
            "rotations",
            "setcc",
            "not_neg",
            "division_signee",
            "immediats",
            "prevoir_tous_les_bits",
            "decalages",
            "compter_bits",
            "multiplier",
            "division",
            "memoire",
            "tailles",
            "boutisme",
            "donnees",
            "tableau",
            "matrice",
            "recherche",
            "adresses",
            "min_max",
            "pair_impair",
            "occurrences",
            "inverser_tableau",
            "borne",
            "puissances",
            "somme_moyenne",
            "tableau_octets",
            "memmove",
            "remplir",
            "bit_scan",
            "adc_128",
            "table_conversion",
            "padding",
            // La rétro-ingénierie entre ici : lire un fichier avant
            // de lire son code.
            "re_entete",
            "re_chaines",
            "re_position_adresse",
            "prevoir_decalage",
            "prevoir_somme",
            "prevoir_octet_choisi",
            "prevoir_appel_systeme",
            "chaine",
            "chaines",
            "comparer_chaines",
            "lire_nombre",
            "macros",
            "structure",
            "branchements",
            "sans_branche",
            "pile",
            "appels",
            "convention",
            "recursion",
            "fibonacci",
            "cadre",
            "pile_locale",
            "tri",
            "longueur_chaine",
            "inverser_chaine",
            "compter_mots",
            "pile_inverse",
            "args_pile",
            "pointeur_fonction",
            "alignement",
            "concatener",
            "premier_mot",
            "recursion_terminale",
            "tri_insertion",
            "recherche_binaire",
            "liste_chainee",
            "transposee",
            // Ce qu'un contrôle d'accès a l'air d'être, vu du
            // désassemblage.
            "re_comparaison",
            "re_derouler",
            "re_table_sauts",
            "prevoir_pile",
            "prevoir_signe",
            "prevoir_convention",
            "sortie",
            "entree",
            "tas",
            "flottants",
            "simd",
            "afficher",
            "majuscules",
            "debordement",
            "protection",
            "hexa_affichage",
            "xor_chiffrement",
            "somme_controle",
            "canari",
            "prevoir_canari",
            "flottants_masque",
            "simd_somme",
            "flottants_conversion",
            "table_ascii",
            "afficher_signe",
            "echo",
            // Ce qu'un programme fait pour ne pas être lu, et ce
            // qu'on y répond.
            "re_chaines_cachees",
            "re_antidebogage",
            "re_int3",
            "re_entropie",
            "re_gadget",
            "re_un_octet",
        ]
    );
    for exercise in &pack.exercises {
        assert!(
            exercise.starter_code.contains("; But :"),
            "{} ne dit pas ce qu'on cherche",
            exercise.id
        );
    }
}

/// Les exercices qui annoncent un résultat doivent le donner.
///
/// « But : RAX doit valoir 15 » est une promesse faite à quelqu'un qui
/// apprend : s'il obtient autre chose, il cherchera son erreur là où il
/// n'y en a pas. On exécute donc le programme de départ et l'on vérifie
/// ce qu'il annonce.
#[test]
fn the_exercises_deliver_what_they_promise() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Ce que chaque exercice doit avoir dans ses registres à l'arrêt,
    // tel que son propre commentaire l'annonce.
    let attendus: &[(&str, &[(Reg64, u64)])] = &[
        ("boucle", &[(Reg64::Rax, 15)]),
        ("bits", &[(Reg64::Rax, 40), (Reg64::Rbx, 1)]),
        ("multiplier", &[(Reg64::Rax, 7), (Reg64::Rdx, 1)]),
        ("chaine", &[(Reg64::Rcx, 5)]),
        ("tableau", &[(Reg64::Rax, 100)]),
    ];

    let packs = [builtin_pack()];
    for (id, registres) in attendus {
        let exercise = namsdec_exercises::find(&packs, &format!("namsdec/{id}"))
            .unwrap_or_else(|| panic!("l'exercice {id} existe"));

        let mut app = NamsdecApp {
            source: programme_éprouvé(exercise).to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "{id} ne s'assemble pas : {:?}",
            app.status
        );
        app.do_run();
        let ctx = egui::Context::default();
        for _ in 0..50 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }
        assert!(!app.running, "{id} ne rend pas la main");

        let regs = &app.emulator.as_ref().unwrap().cpu().regs;
        for (reg, valeur) in *registres {
            assert_eq!(
                regs.get64(*reg),
                *valeur,
                "{id} : {reg:?} devrait valoir {valeur}, et son commentaire le promet"
            );
        }
    }
}

/// Le parcours se voit même quand son module n'est pas chargé.
///
/// Un module qu'on active si on le désire, soit — mais une porte qu'on
/// ne voit pas n'existe pas. Quelqu'un qui débute ne devinera pas qu'il
/// y a quarante-quatre exercices derrière un réglage.
#[test]
fn the_course_is_discoverable_before_it_is_loaded() {
    let mut app = NamsdecApp::default();
    assert!(!app.exercises, "le module est éteint au départ");
    // Le programme d'accueil dit où aller.
    assert!(
        app.source.contains("Préférences > Modules"),
        "{}",
        app.source
    );

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        // Le menu Outils ouvert : c'est là que l'entrée vit.
        events: vec![egui::Event::PointerButton {
            pos: egui::pos2(0.0, 0.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
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
    // Le texte d'accueil est à l'écran, avec sa mention du parcours.
    assert!(painted.contains("cent dix-sept exercices"), "{painted}");
}

/// Le programme d'accueil annonce le bon nombre d'exercices.
///
/// C'est la PREMIÈRE phrase qu'on lit en ouvrant namsdec, et elle a
/// annoncé « quatre-vingt-seize » le jour où le parcours en comptait
/// cent huit. Le même défaut que l'aide des appels système : un nombre
/// écrit dans une phrase ne suit pas le code qui le dément, et celui
/// qui ajoute la cent neuvième étape ne vient pas relire l'écran
/// d'accueil.
///
/// Trouvé par la checklist d'avant-commit, en regardant la capture de
/// la fenêtre des exercices — pas par une mesure, qui ne cherchait pas
/// là.
#[test]
fn le_programme_d_accueil_annonce_le_bon_nombre_d_exercices() {
    /// Un nombre en toutes lettres, comme le projet les écrit.
    ///
    /// Jusqu'à mille : au-delà, un parcours ne s'annonce plus par son
    /// compte.
    fn en_lettres(n: usize) -> String {
        const UNITÉS: [&str; 20] = [
            "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix",
            "onze", "douze", "treize", "quatorze", "quinze", "seize", "dix-sept", "dix-huit",
            "dix-neuf",
        ];
        const DIZAINES: [&str; 10] = [
            "",
            "",
            "vingt",
            "trente",
            "quarante",
            "cinquante",
            "soixante",
            "soixante",
            "quatre-vingt",
            "quatre-vingt",
        ];
        match n {
            0..=19 => UNITÉS[n].to_owned(),
            20..=99 => {
                let (d, u) = (n / 10, n % 10);
                // Soixante-dix et quatre-vingt-dix comptent jusqu'à
                // dix-neuf, comme le veut le français.
                let reste = if d == 7 || d == 9 { n - d * 10 + 10 } else { u };
                let tête = DIZAINES[d];
                let pluriel = if d == 8 && reste == 0 { "s" } else { "" };
                if reste == 0 {
                    format!("{tête}{pluriel}")
                } else if reste == 1 && d != 8 && d != 7 && d != 9 {
                    format!("{tête} et un")
                } else {
                    format!("{tête}-{}", en_lettres(reste))
                }
            }
            100..=999 => {
                let (c, reste) = (n / 100, n % 100);
                let tête = if c == 1 {
                    "cent".to_owned()
                } else if reste == 0 {
                    format!("{} cents", en_lettres(c))
                } else {
                    format!("{} cent", en_lettres(c))
                };
                if reste == 0 {
                    tête
                } else {
                    format!("{tête} {}", en_lettres(reste))
                }
            }
            _ => n.to_string(),
        }
    }
    // La table vaut ce qu'elle vaut si on ne l'éprouve pas.
    assert_eq!(en_lettres(96), "quatre-vingt-seize");
    assert_eq!(en_lettres(108), "cent huit");
    assert_eq!(en_lettres(21), "vingt et un");
    assert_eq!(en_lettres(80), "quatre-vingts");
    assert_eq!(en_lettres(71), "soixante-onze");

    let combien = builtin_pack().exercises.len();
    let attendu = format!("parcours de {} exercices", en_lettres(combien));
    assert!(
        crate::DEFAULT_SOURCE.contains(&attendu),
        "le programme d'accueil doit annoncer « {attendu} » — voici ce qu'il dit :\n{}",
        crate::DEFAULT_SOURCE
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Le parcours se reprend là où on l'avait laissé.
///
/// Douze étapes ne se font pas d'une traite : au retour, l'outil doit
/// dire où l'on en était plutôt que de laisser chercher. Le rang de
/// chaque exercice est affiché pour la même raison — un exercice n'est
/// pas un devoir isolé, il vient après les précédents.
#[test]
fn the_course_remembers_where_one_stopped() {
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        show_exercises: true,
        ..Default::default()
    };
    assert!(app.current_exercise.is_none(), "rien n'a encore été fait");

    app.load_exercise("namsdec/tableau");
    assert_eq!(app.current_exercise.as_deref(), Some("namsdec/tableau"));
    // Et le choix part dans les réglages, pour la session suivante.
    let written = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(
        written.contains("\"current_exercise\":\"namsdec/tableau\""),
        "{written}"
    );

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    // Rouvrir la fenêtre : elle doit se placer sur l'étape en cours.
    app.show_exercises = true;
    let mut painted = String::new();
    for _ in 0..3 {
        painted.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        fn collecte(shape: &egui::epaint::Shape, out: &mut String) {
            match shape {
                egui::epaint::Shape::Text(text) => {
                    out.push_str(text.galley.text());
                    out.push('|');
                }
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collecte(shape, out);
                    }
                }
                _ => {}
            }
        }
        for clipped in &output.shapes {
            collecte(&clipped.shape, &mut painted);
        }
        output.textures_delta.clear();
    }

    assert!(
        painted.contains("En cours :"),
        "l'étape en cours se voit sans avoir à la chercher : {painted}"
    );
    // Le rang dans le parcours, aussi : le tableau est la
    // trente et unième des cent dix-sept étapes.
    assert!(painted.contains("31/117"), "{painted}");
}

/// Un parcours se suit : l'étape suivante est à un clic, sans avoir à
/// retrouver son rang dans une liste de cent dix-sept.
#[test]
fn the_course_offers_the_next_step() {
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };

    // Depuis le premier, la suite est le deuxième.
    app.load_exercise("namsdec/registres");
    let (suivant, titre) = app
        .exercise_after("namsdec/registres")
        .expect("il y a une suite");
    assert_eq!(suivant, "namsdec/echange");
    assert!(!titre.is_empty());

    // Le dernier n'en a pas : un parcours a une fin, et l'outil ne
    // propose pas d'aller au-delà.
    assert!(app.exercise_after("namsdec/re_un_octet").is_none());
    // Un identifiant inconnu non plus.
    assert!(app.exercise_after("namsdec/ceci-nexiste-pas").is_none());

    // Et le bouton mène bien à l'étape suivante.
    app.show_exercises = true;
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
    assert!(painted.contains("Étape suivante"), "{painted}");
    assert!(
        painted.contains("1/117"),
        "le rang de l'étape en cours : {painted}"
    );
}

/// L'exercice du débordement tient sa promesse, et le motif cyclique y
/// répond : le décalage annoncé est bien celui qui sépare le tampon de
/// l'adresse de retour.
///
/// C'est la boucle complète — `cyclic`, l'entrée du programme, `RIP`,
/// l'offset — et elle doit marcher pour de vrai, sans quoi l'énoncé
/// décrit un outil qui ment.
#[test]
fn the_overflow_exercise_is_measured_by_the_cyclic_pattern() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let pack = builtin_pack();
    let exercice = pack.by_id("debordement").expect("l'étape existe");
    let mut app = NamsdecApp {
        source: exercice.starter_code.clone(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Le motif, tel que la fenêtre le pose dans l'entrée du programme.
    let motif = namsdec_re::cyclic::pattern(64);
    // Comme le bouton de la fenêtre : le motif part dans l'entrée du
    // programme, suivi du saut de ligne que la console ajoute.
    app.send_input(Some(String::from_utf8(motif.clone()).unwrap()));

    app.do_run();
    for _ in 0..200 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    // Le `ret` est parti dans le motif : RIP en porte quatre octets.
    let rip = app.emulator.as_ref().expect("chargé").cpu().regs.rip;
    let décalage = namsdec_re::cyclic::offset_of_value(rip)
        .unwrap_or_else(|| panic!("RIP ({rip:#x}) ne vient pas du motif"));

    // Et ce décalage est celui qu'on attend : seize octets de tampon,
    // puis l'adresse de retour que le `call` avait empilée.
    assert_eq!(décalage, 16, "seize octets de tampon avant le retour");
}

/// Le but d'un exercice se vérifie tout seul : l'énoncé dit « RAX doit
/// valoir 15 », et la machine sait comparer.
///
/// Un élève qui vérifie à l'œil se trompe de registre, ou lit du
/// décimal là où il y a de l'hexadécimal. Le verdict s'affiche là où
/// l'on regarde déjà les valeurs.
#[test]
fn an_exercise_says_whether_its_goal_is_reached() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    // Sans exercice chargé, rien à dire.
    assert!(app.exercise_goal().is_none());

    app.load_exercise("namsdec/boucle");
    assert_eq!(app.current_exercise.as_deref(), Some("namsdec/boucle"));
    // Chargé mais pas assemblé : toujours rien — on ne reproche pas son
    // échec à quelqu'un qui n'a rien lancé.
    assert!(app.exercise_goal().is_none());

    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let (conditions, atteint) = app.exercise_goal().expect("un but à vérifier");
    assert_eq!(
        conditions
            .iter()
            .map(|l| l.texte.as_str())
            .collect::<Vec<_>>(),
        vec!["rax == 0xf"]
    );
    assert!(!atteint, "le programme n'a pas encore tourné");

    // LA SOLUTION DANS L'ÉDITEUR : le programme livré a un trou, et
    // c'est la boucle elle-même. Ce qu'on vérifie ici, c'est que le
    // verdict passe au vert quand le travail est fait.
    charge_la_solution(&mut app, "namsdec/boucle");
    app.do_assemble();

    // Il tourne, et le but est atteint : 5+4+3+2+1 = 15.
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    let (_, atteint) = app.exercise_goal().expect("un but à vérifier");
    assert!(
        atteint,
        "RAX = {:#x}",
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax)
    );
    // Le but vit dans la fenêtre dédiée aux exercices, plus dans le
    // bandeau : on l'ouvre pour le lire.
    app.show_exercises = true;
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).goal_reached),
        "{painted}"
    );
}

/// Chaque but annoncé SE LIT, et il porte sur un registre que le
/// programme touche.
///
/// Un but qu'on ne sait pas analyser ne dira jamais « atteint », quoi
/// qu'on écrive, et personne ne saura pourquoi.
///
/// CE CONTRÔLE PORTAIT AUSSI DEUX LISTES — quatre-vingt-dix-sept
/// « démonstrations » dont le but était atteint d'emblée, quatre
/// « à faire » qui demandaient un geste. Elles disaient la vérité, et
/// c'était le défaut : quatre-vingt-dix-sept étapes du parcours
/// n'avaient rien à faire faire. Elles ont toutes un trou maintenant, et
/// c'est `chaque_exercice_a_quelque_chose_a_faire_et_une_solution_qui_marche`
/// qui le tient — en EXÉCUTANT les deux versions, plutôt qu'en tenant
/// une liste de noms à jour à la main.
#[test]
fn every_goal_is_readable_and_on_the_right_side() {
    let pack = builtin_pack();
    let mut avec_but = 0;
    for exercise in &pack.exercises {
        for texte in &exercise.goal {
            let condition = condition::parse(texte)
                .unwrap_or_else(|| panic!("{} : « {texte} » ne se lit pas", exercise.id));
            // Le côté gauche nomme ce qu'on interroge : un registre, ou
            // une adresse. Un but qui compare deux constantes serait vrai
            // ou faux pour toujours.
            let gauche = condition.left().to_lowercase();
            assert!(
                gauche.chars().any(|c| c.is_ascii_alphabetic()),
                "{} : « {texte} » n'interroge rien",
                exercise.id
            );
            avec_but += 1;
        }
    }
    assert!(
        avec_but > 100,
        "{avec_but} conditions relevées : le contrôle regarde-t-il encore \
         le parcours ?"
    );
}

/// Un exercice réussi le reste : la pastille s'allume, le compte monte,
/// et cela survit au lancement suivant.
///
/// Un parcours sans trace de ce qu'on a déjà fait oblige à s'en souvenir
/// soi-même, et l'on refait le premier trois fois plutôt que d'attaquer
/// le septième.
#[test]
fn a_solved_exercise_stays_solved() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    assert!(app.exercises_done.is_empty(), "rien de fait au départ");

    // Résolu, puisque c'est de cela qu'on parle : le sujet livré a un
    // trou, et un exercice qu'on n'a pas fait ne se note pas.
    charge_la_solution(&mut app, "namsdec/boucle");
    app.do_assemble();
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    // Le but est atteint : le rendu le note une fois pour toutes.
    paint(&mut app);
    assert!(
        app.exercises_done.contains("namsdec/boucle"),
        "{:?}",
        app.exercises_done
    );

    // Repartir du programme de départ ne le défait pas : on ne retire
    // pas ce qu'on a compris.
    app.load_exercise("namsdec/boucle");
    paint(&mut app);
    assert!(app.exercises_done.contains("namsdec/boucle"));

    // Et le disque s'en souvient.
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(
        écrit.contains("\"exercises_done\":[\"namsdec/boucle\"]"),
        "{écrit}"
    );
    let relu = NamsdecApp::from_settings(serde_json::from_str(&écrit).unwrap());
    assert!(relu.exercises_done.contains("namsdec/boucle"));

    // La fenêtre le montre : le compte de la série.
    app.show_exercises = true;
    let painted = paint(&mut app);
    assert!(
        painted.contains(&format!("1/117 {}", strings(app.lang).exercises_done_count)),
        "{painted}"
    );
}

/// Les cinq étapes ajoutées tiennent ce qu'elles montrent : la copie
/// sans boucle, le cadre de pile, le tas, les flottants, et le nombre
/// affiché.
///
/// Leur but est vérifié ailleurs ; ici on regarde ce qu'elles
/// *produisent*, parce qu'un exercice sur la mémoire ou sur la console
/// se juge à ce qu'il laisse derrière lui.
#[test]
fn the_five_new_steps_do_what_they_show() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let jouer = |id: &str| -> NamsdecApp {
        let mut app = NamsdecApp {
            exercises: true,
            exercise_packs: vec![builtin_pack()],
            ..Default::default()
        };
        // La solution, non le sujet : ce contrôle regarde ce que le
        // programme FAIT.
        charge_la_solution(&mut app, &format!("namsdec/{id}"));
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "{id} : {:?}",
            app.status
        );
        app.do_run();
        for _ in 0..200 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
        app
    };

    // `rep movsb` a bien copié la chaîne, zéro final compris.
    let app = jouer("chaines");
    let emulator = app.emulator.as_ref().expect("chargé");
    let copie = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|s| s.name == "copie")
        .expect("l'étiquette existe")
        .addr;
    let octets = emulator.cpu().mem.peek_bytes(copie, 6).unwrap();
    assert_eq!(octets, b"salut\0", "la copie est complète");
    assert_eq!(emulator.cpu().regs.get64(Reg64::Rcx), 0, "RCX décompté");

    // Le cadre de pile est défait proprement : RSP revenu, aucune
    // entorse à la convention.
    let app = jouer("cadre");
    let emulator = app.emulator.as_ref().expect("chargé");
    assert!(emulator.breaches().is_empty(), "{:?}", emulator.breaches());

    // Le tas a bougé, et la mémoire obtenue s'écrit.
    let app = jouer("tas");
    let emulator = app.emulator.as_ref().expect("chargé");
    let regs = &emulator.cpu().regs;
    assert!(
        regs.get64(Reg64::R9) > regs.get64(Reg64::R8),
        "la rupture est montée"
    );
    assert_eq!(emulator.heap().current(), regs.get64(Reg64::R9));

    // Les flottants : 1.5 + 2.25, et la lecture décimale le dit.
    let app = jouer("flottants");
    let rax = app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax);
    assert_eq!(f64::from_bits(rax), 3.75);

    // Et le nombre s'affiche, dans le bon ordre.
    let app = jouer("afficher");
    let sortie = String::from_utf8_lossy(app.emulator.as_ref().unwrap().output()).into_owned();
    assert_eq!(sortie, "1234", "les chiffres sortent à l'endroit");
}

/// Les cinq nouvelles étapes tiennent ce qu'elles montrent : les
/// données déclarées, le calcul d'adresse, le préprocesseur, la
/// récursion, et l'entrée qui attend.
#[test]
fn the_five_later_steps_do_what_they_show() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let jouer = |id: &str| -> NamsdecApp {
        let mut app = NamsdecApp {
            exercises: true,
            exercise_packs: vec![builtin_pack()],
            ..Default::default()
        };
        // La solution, non le sujet : ce contrôle regarde ce que le
        // programme FAIT.
        charge_la_solution(&mut app, &format!("namsdec/{id}"));
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "{id} : {:?}",
            app.status
        );
        app.do_run();
        for _ in 0..500 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
        app
    };

    // Les directives ont écrit ce qu'elles annoncent : quatre octets,
    // un mot, cinq lettres, puis quatre fois 0xff.
    let app = jouer("donnees");
    let state = app.assemble_state.as_ref().unwrap();
    let octets = state
        .output
        .symbols
        .iter()
        .find(|s| s.name == "octets")
        .expect("l'étiquette existe")
        .addr;
    let mémoire = &app.emulator.as_ref().unwrap().cpu().mem;
    assert_eq!(
        mémoire.peek_bytes(octets, 11).unwrap(),
        b"\x11\x22\x33\x44\x01\x02salut",
        "les octets, dans l'ordre où la machine les range"
    );

    // `lea` a calculé sans lire : RBX porte l'adresse de la table.
    let app = jouer("adresses");
    let state = app.assemble_state.as_ref().unwrap();
    let table = state
        .output
        .symbols
        .iter()
        .find(|s| s.name == "table")
        .expect("l'étiquette existe")
        .addr;
    let regs = &app.emulator.as_ref().unwrap().cpu().regs;
    assert_eq!(regs.get64(Reg64::Rbx), table, "l'adresse, pas le contenu");

    // Le préprocesseur n'a laissé ni `call` ni `ret` : la macro est du
    // texte recopié, et le `%rep` a bien écrit trois `add`.
    let app = jouer("macros");
    let ajouts = app
        .assemble_state
        .as_ref()
        .unwrap()
        .disasm
        .iter()
        .filter(|i| i.text.starts_with("add "))
        .count();
    assert_eq!(ajouts, 5, "deux de la macro, trois du %rep");
    assert!(
        !app.source.contains("call ajoute_deux_fois"),
        "une macro n'est pas un appel"
    );

    // La récursion est descendue puis remontée : la pile est rendue.
    let app = jouer("recursion");
    let emulator = app.emulator.as_ref().unwrap();
    assert!(emulator.frames().is_empty(), "tous les appels sont revenus");
    assert_eq!(emulator.cpu().regs.get64(Reg64::Rax), 120);

    // Et l'entrée attend : le programme rend la main sans s'arrêter.
    let app = jouer("entree");
    assert!(
        matches!(app.status, Status::WaitingForInput),
        "{:?}",
        app.status
    );
}

/// Le budget est une mesure, pas une note : et il est tenable.
///
/// Chaque exercice qui en porte un annonce ce que coûte la solution
/// livrée. Promettre un budget qu'aucune solution n'atteint serait pire
/// que de n'en pas donner : ce test vérifie, pour chacun, qu'une
/// solution existe — celle qu'on livre pour les démonstrations, celle
/// que l'énoncé demande d'écrire pour les autres.
#[test]
fn every_budget_is_one_a_program_can_hold() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // LA SOLUTION VIENT DU PACK, et non plus d'une table de
    // corrections tenue ici.
    //
    // Quatre exercices avaient un trou, et ce contrôle portait leur
    // correction en dur : « re_derouler → mov rax, 55647 ». Depuis le
    // 11 novembre 2026 ils en ont CENT UN, et chacun livre sa solution
    // dans le fichier de la série — où un contrôle l'exécute pour
    // vérifier qu'elle tient. Recopier ici cent une corrections aurait
    // fait deux vérités à tenir d'accord.
    for exercise in &builtin_pack().exercises {
        let budget = exercise.par.unwrap_or(u64::MAX);
        let mut app = NamsdecApp {
            source: programme_éprouvé(exercise).to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "{} : {:?}",
            exercise.id,
            app.status
        );
        app.do_run();
        for _ in 0..500 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
        let emulator = app.emulator.as_ref().expect("chargé");
        let exécutées = emulator.executed();
        assert!(
            exécutées <= budget,
            "{} : la solution coûte {exécutées} instructions pour un budget de {budget}",
            exercise.id
        );
        // Et elle atteint bien le but qu'on lui demande.
        let regs = &emulator.cpu().regs;
        for texte in &exercise.goal {
            let condition = condition::parse(texte).unwrap_or_else(|| panic!("{texte}"));
            assert!(
                condition.holds(regs, None),
                "{} : « {texte} » n'est pas atteint par la solution",
                exercise.id
            );
        }
    }
}

/// Le score d'un exercice est ce qu'il coûte, et c'est le meilleur qui
/// reste.
///
/// On essaie une idée plus courte, elle échoue, on revient à la
/// précédente : le score d'avant n'a pas à en pâtir.
#[test]
fn the_score_of_an_exercise_is_what_it_costs() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let jouer = |app: &mut NamsdecApp| {
        app.do_assemble();
        app.do_run();
        for _ in 0..200 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
        paint(app);
    };

    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    // La SOLUTION livrée : le sujet, lui, a un trou — il n'atteint pas
    // le but, et un but non atteint ne marque aucun score.
    charge_la_solution(&mut app, "namsdec/boucle");
    jouer(&mut app);
    let livrée = app.exercises_best.get("namsdec/boucle").copied();
    assert_eq!(livrée, Some(18), "le coût de la solution livrée");

    // Une solution plus courte : la somme des cinq premiers entiers ne
    // demande pas de boucle.
    app.source = "\
global _start
section .text
_start:
    mov rax, 15
    hlt
"
    .to_owned();
    jouer(&mut app);
    let mieux = app.exercises_best.get("namsdec/boucle").copied();
    assert_eq!(mieux, Some(2), "le meilleur descend");

    // Une tentative plus longue ne remonte pas le score.
    charge_la_solution(&mut app, "namsdec/boucle");
    jouer(&mut app);
    assert_eq!(app.exercises_best.get("namsdec/boucle").copied(), Some(2));

    // Et le score survit au lancement suivant.
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    let relu = NamsdecApp::from_settings(serde_json::from_str(&écrit).unwrap());
    assert_eq!(relu.exercises_best.get("namsdec/boucle").copied(), Some(2));
}

/// Le coût s'affiche à côté du but : ce que le programme a dépensé, le
/// budget, et le meilleur obtenu jusqu'ici.
#[test]
fn the_cost_is_shown_next_to_the_goal() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    charge_la_solution(&mut app, "namsdec/boucle");
    app.do_assemble();
    app.do_run();
    for _ in 0..200 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    app.show_exercises = true;
    let painted = paint(&mut app);
    let s = strings(app.lang);
    assert!(painted.contains(s.par_label), "{painted}");
    // Le coût de la solution livrée, et son budget.
    assert!(painted.contains("18"), "{painted}");
    assert!(painted.contains("/ 18"), "{painted}");
    assert!(painted.contains(s.par_best), "le meilleur est dit");
}

/// Le parcours est rangé par difficulté croissante, et la liste le dit.
///
/// Quatre paliers, nommés par ce qu'on y fait plutôt que par un
/// chiffre : « Découverte » annonce ce qui attend, « niveau 3 » non.
#[test]
fn the_course_climbs_and_says_so() {
    let pack = builtin_pack();
    // Les paliers ne reculent pas, et les quatre sont là.
    let niveaux: Vec<namsdec_exercises::Level> = pack.exercises.iter().map(|e| e.level).collect();
    assert!(
        niveaux.windows(2).all(|p| p[0] <= p[1]),
        "le parcours redescend : {niveaux:?}"
    );
    for niveau in namsdec_exercises::Level::ALL {
        assert!(niveaux.contains(&niveau), "aucun exercice en {niveau:?}");
    }
    // Le premier exercice ouvre la Découverte, le dernier ferme la
    // Maîtrise : ce n'est pas un hasard de tri.
    assert_eq!(niveaux.first(), Some(&namsdec_exercises::Level::Discovery));
    assert_eq!(niveaux.last(), Some(&namsdec_exercises::Level::Mastery));

    // Et la fenêtre les annonce, en tête de chaque groupe. Trois
    // images : une fenêtre flottante se place à la deuxième, et son
    // contenu n'est peint qu'ensuite.
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        show_exercises: true,
        ..Default::default()
    };
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
    for _ in 0..3 {
        painted.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        fn collecte(shape: &egui::epaint::Shape, out: &mut String) {
            match shape {
                egui::epaint::Shape::Text(text) => {
                    out.push_str(text.galley.text());
                    out.push('|');
                }
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collecte(shape, out);
                    }
                }
                _ => {}
            }
        }
        for clipped in &output.shapes {
            collecte(&clipped.shape, &mut painted);
        }
        output.textures_delta.clear();
    }
    // Le premier palier au moins : la fenêtre ne montre que ce qui tient
    // à l'écran, et les suivants attendent qu'on fasse défiler.
    assert!(
        painted.contains(namsdec_exercises::Level::Discovery.label(app.lang.code())),
        "{painted}"
    );

    // Les deux langues nomment les paliers, et pas de la même façon.
    assert_ne!(
        namsdec_exercises::Level::Discovery.label("fr"),
        namsdec_exercises::Level::Discovery.label("en")
    );
    // Un identifiant inconnu ne range pas au hasard : il retombe sur le
    // premier palier, qui reste vrai d'une série qu'on découvre.
    assert_eq!(
        namsdec_exercises::Level::by_id("n'importe quoi"),
        namsdec_exercises::Level::Discovery
    );
}

/// Un but atteint sans rien exécuter n'est pas une solution.
///
/// « rax == 0 » est vrai avant le premier pas : remettre l'exercice à
/// zéro s'attribuait un score de zéro instruction — imbattable, et faux.
/// On exige désormais que le programme ait tourné et qu'il soit allé
/// jusqu'à son arrêt.
#[test]
fn a_goal_true_before_running_scores_nothing() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    // « Mettre à zéro » : sa solution atteint le but, mais tant que
    // rien n'a TOURNÉ, rien n'est noté.
    charge_la_solution(&mut app, "namsdec/zero");
    app.do_assemble();
    paint(&mut app);
    assert!(
        !app.exercises_best.contains_key("namsdec/zero"),
        "rien n'a tourné : pas de score"
    );
    assert!(!app.exercises_done.contains("namsdec/zero"));

    // Il faut le faire tourner pour valoir quelque chose.
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    paint(&mut app);
    assert_eq!(app.exercises_best.get("namsdec/zero").copied(), Some(7));

    // Et remettre à zéro ne vient pas écraser ce score par un zéro.
    app.dispatch(Action::Reset);
    paint(&mut app);
    assert_eq!(app.exercises_best.get("namsdec/zero").copied(), Some(7));

    // Un programme arrêté en cours de route ne compte pas non plus : le
    // but est peut-être atteint, mais le programme n'est pas fini.
    app.dispatch(Action::Reset);
    app.do_step();
    paint(&mut app);
    assert_eq!(app.exercises_best.get("namsdec/zero").copied(), Some(7));
}

/// « Étape suivante » saute ce qui est déjà réussi, et la fenêtre sait
/// retrouver une étape parmi trente-six.
#[test]
fn the_next_step_skips_what_is_already_done() {
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    // Rien de fait : la suivante est la suivante.
    let (suivant, _) = app
        .exercise_after("namsdec/registres")
        .expect("il y a une suite");
    assert_eq!(suivant, "namsdec/echange");

    // Les deux d'après déjà réussies : on saute par-dessus.
    app.exercises_done.insert("namsdec/echange".to_owned());
    app.exercises_done.insert("namsdec/zero".to_owned());
    let (suivant, _) = app
        .exercise_after("namsdec/registres")
        .expect("il y a une suite");
    assert_eq!(suivant, "namsdec/boucle");

    // Tout réussi jusqu'au bout : on retombe sur la suivante, plutôt
    // que de ne rien proposer.
    for exercise in &builtin_pack().exercises {
        app.exercises_done
            .insert(format!("namsdec/{}", exercise.id));
    }
    let (suivant, _) = app
        .exercise_after("namsdec/registres")
        .expect("il y a une suite");
    assert_eq!(suivant, "namsdec/echange");

    // Et la recherche filtre la liste : sur « pile », il reste les
    // étapes qui en parlent, pas les trente-six.
    app.show_exercises = true;
    app.exercise_query = "récursion".to_owned();
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).exercises_search),
        "{painted}"
    );
}

/// Tout ce qu'un exercice écrit s'explique sous F1.
///
/// Un parcours qui emploie une instruction que le manuel ignore laisse
/// l'élève devant un mot qu'aucune aide ne couvre — et c'est justement
/// l'instruction qu'il vient de rencontrer pour la première fois.
#[test]
fn every_instruction_the_course_writes_has_a_sheet() {
    let pack = builtin_pack();
    let mut sans_fiche: std::collections::BTreeMap<String, String> = Default::default();

    for exercise in &pack.exercises {
        // Ce que le fichier définit lui-même : étiquettes, constantes,
        // macros, noms de données. Ce ne sont pas des instructions, et
        // le manuel n'a pas à les connaître.
        let mut siens: std::collections::BTreeSet<String> = Default::default();
        for ligne in exercise.starter_code.lines() {
            let ligne = ligne.split(';').next().unwrap_or("").trim();
            let mut mots = ligne.split_whitespace();
            let Some(premier) = mots.next() else { continue };
            if let Some(étiquette) = premier.strip_suffix(':') {
                siens.insert(étiquette.to_lowercase());
                continue;
            }
            // `nom db …`, `nom equ …`, `nom resb …` : le premier mot est
            // un nom que ce fichier invente.
            let deuxième = mots.next().map(str::to_lowercase);
            if matches!(
                deuxième.as_deref(),
                Some(
                    "db" | "dw" | "dd" | "dq" | "equ" | "resb" | "resw" | "resd" | "resq" | "times"
                )
            ) {
                siens.insert(premier.to_lowercase());
            }
            if (premier.eq_ignore_ascii_case("%macro")
                || premier.eq_ignore_ascii_case("%define")
                // `struc Personne` : le nom d'une structure est inventé
                // par le fichier, au même titre qu'une étiquette.
                || premier.eq_ignore_ascii_case("struc"))
                && let Some(nom) = ligne.split_whitespace().nth(1)
            {
                siens.insert(nom.to_lowercase());
            }
        }

        for ligne in exercise.starter_code.lines() {
            let ligne = ligne.split(';').next().unwrap_or("").trim();
            if ligne.is_empty() {
                continue;
            }
            for mot in ligne.split_whitespace().take(2) {
                let mot = mot.trim_end_matches([',', ':']).to_lowercase();
                if mot.is_empty() || siens.contains(&mot) {
                    continue;
                }
                // Un nombre, une adresse, un registre : rien à
                // documenter ici — les registres ont leur propre fiche,
                // sous « registre ».
                if mot.chars().next().is_some_and(|c| c.is_ascii_digit())
                    || mot.starts_with('[')
                    || mot.starts_with('\'')
                    || mot.starts_with('"')
                    // Un nom de section : `.text`, `.data`, `.bss`. La
                    // directive `section`, elle, a bien sa fiche.
                    || mot.starts_with('.')
                    || est_un_registre(&mot)
                {
                    continue;
                }
                if namsdec_doc::lookup(&mot).is_none() {
                    sans_fiche.insert(mot, exercise.id.clone());
                }
            }
        }
    }

    // Les mots de taille que l'assembleur attend devant un opérande
    // mémoire : ils appartiennent à la fiche des directives de données,
    // qui les nomme.
    let tailles: std::collections::BTreeSet<&str> =
        ["byte", "word", "dword", "qword"].into_iter().collect();
    sans_fiche.retain(|mot, _| !tailles.contains(mot.as_str()));

    assert!(
        sans_fiche.is_empty(),
        "un exercice écrit ce que le manuel n'explique pas : {sans_fiche:?}"
    );
}

/// Chaque palier dit où l'on en est : « Découverte 3/10 ».
///
/// Quarante-quatre étapes se lisent par groupes. Un compte global ne dit
/// pas ce qui reste *ici*, et c'est pourtant la question qu'on se pose
/// en rouvrant le parcours.
#[test]
fn each_level_says_how_far_one_has_got() {
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        show_exercises: true,
        ..Default::default()
    };
    // Rien de fait : le premier palier est à zéro.
    let pack = builtin_pack();
    let combien = |niveau: namsdec_exercises::Level| {
        pack.exercises
            .iter()
            .filter(|exercise| exercise.level == niveau)
            .count()
    };
    let découverte = combien(namsdec_exercises::Level::Discovery);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| {
        let mut painted = String::new();
        for _ in 0..3 {
            painted.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            fn collecte(shape: &egui::epaint::Shape, out: &mut String) {
                match shape {
                    egui::epaint::Shape::Text(text) => {
                        out.push_str(text.galley.text());
                        out.push('|');
                    }
                    egui::epaint::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collecte(shape, out);
                        }
                    }
                    _ => {}
                }
            }
            for clipped in &output.shapes {
                collecte(&clipped.shape, &mut painted);
            }
            output.textures_delta.clear();
        }
        painted
    };
    let painted = peindre(&mut app);
    assert!(painted.contains(&format!("0/{découverte}")), "{painted}");

    // Deux étapes du premier palier réussies : le compte suit.
    app.exercises_done.insert("namsdec/registres".to_owned());
    app.exercises_done.insert("namsdec/echange".to_owned());
    let painted = peindre(&mut app);
    assert!(painted.contains(&format!("2/{découverte}")), "{painted}");
}

/// Le profil rappelle le budget de l'exercice, et dit quoi en faire.
///
/// « 61 instructions pour un budget de 30 » sans rien d'autre laisse
/// devant la même question qu'avant ; la ligne en tête du classement est
/// la réponse, et le profil est l'endroit où les deux se rencontrent.
#[test]
fn the_profile_recalls_the_exercise_budget() {
    let ctx = egui::Context::default();
    // Un exercice qui a un budget, et un programme qui le dépasse de
    // beaucoup : une boucle de cent tours là où l'énoncé en veut peu.
    let pack = builtin_pack();
    let exercice = pack
        .exercises
        .iter()
        .find(|e| e.par.is_some())
        .expect("au moins un exercice a un budget");
    let budget = exercice.par.expect("le budget existe");
    let id = namsdec_exercises::full_id(&pack, exercice);

    let mut app = NamsdecApp {
        source: format!(
            "\
section .text
global _start
_start:
    mov rcx, {tours}
boucle:
    dec rcx
    jnz boucle
    mov rax, 60
    xor rdi, rdi
    syscall
",
            tours = budget + 100
        ),
        current_exercise: Some(id),
        exercise_packs: vec![builtin_pack()],
        console_tab: ConsoleTab::Profile,
        ..Default::default()
    };
    app.do_assemble();
    app.do_run();
    for _ in 0..20 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    let exécutées = app.emulator.as_ref().expect("un émulateur").executed();
    assert!(
        exécutées > budget,
        "le programme doit dépasser le budget pour que le rappel ait un sens : {exécutées} contre {budget}"
    );

    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let mut peint = String::new();
    for _ in 0..2 {
        peint.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.console_panel_ui(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                peint.push_str(text.galley.text());
                peint.push('\n');
            }
        }
        output.textures_delta.clear();
    }
    assert!(
        peint.contains(&format!("{exécutées} / {budget}")),
        "le budget doit se lire dans le profil :\n{peint}"
    );
    assert!(
        peint.contains("au-dessus du budget"),
        "et le dépassement doit dire par où reprendre :\n{peint}"
    );
}

/// Fermer l'onglet d'une étape ne jette pas ce qu'on y avait écrit.
///
/// C'était le seul endroit du parcours où du travail disparaissait :
/// l'onglet emportait le programme, et revenir à l'étape rendait le
/// code de départ — c'est-à-dire exactement ce qu'on ne cherchait pas.
/// Le brouillon est rangé en mémoire à la fermeture, et relu quand on
/// redemande l'étape, pas avant.
#[test]
fn a_closed_step_keeps_what_was_written() {
    let pack = builtin_pack();
    let premier = namsdec_exercises::full_id(&pack, &pack.exercises[0]);
    let second = namsdec_exercises::full_id(&pack, &pack.exercises[1]);

    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    app.load_exercise_now(&premier);
    app.source.push_str("    ; mon essai\n");
    app.load_exercise_now(&second);
    assert_eq!(app.tabs.len(), 1, "le premier attend dans son onglet");

    // On ferme l'onglet du premier — celui qui n'est pas à l'écran.
    let rang = app
        .exercise_tab_index(&premier)
        .expect("le premier a son onglet");
    app.close_tab_now(rang);
    assert!(app.tabs.is_empty(), "il ne reste que l'étape courante");
    assert!(
        app.exercise_drafts.contains_key(&premier),
        "le brouillon a été rangé : {:?}",
        app.exercise_drafts.keys().collect::<Vec<_>>()
    );

    // Et y revenir le rend.
    app.load_exercise_now(&premier);
    assert_eq!(app.current_exercise.as_deref(), Some(premier.as_str()));
    assert!(
        app.source.contains("; mon essai"),
        "ce qu'on avait écrit revient avec l'étape :\n{}",
        app.source
    );
    // Il n'est écrit nulle part : l'étape porte sa marque de
    // non-enregistré, comme elle la portait dans son onglet.
    assert!(
        app.buffer_modified(),
        "un brouillon rendu n'est pas un fichier enregistré"
    );
}

/// Une étape qu'on n'a pas touchée ne laisse pas de brouillon.
///
/// En garder un ferait croire, au retour, à un travail qu'on n'a pas
/// fait — et il empêcherait une correction d'énoncé d'arriver jusqu'à
/// l'éditeur : les séries sont des fichiers de données, elles changent
/// sans qu'on recompile.
#[test]
fn an_untouched_step_leaves_no_draft() {
    let pack = builtin_pack();
    let premier = namsdec_exercises::full_id(&pack, &pack.exercises[0]);
    let second = namsdec_exercises::full_id(&pack, &pack.exercises[1]);

    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    app.load_exercise_now(&premier);
    app.load_exercise_now(&second);
    let rang = app
        .exercise_tab_index(&premier)
        .expect("le premier a son onglet");
    app.close_tab_now(rang);
    assert!(
        app.exercise_drafts.is_empty(),
        "rien n'a été écrit, rien n'est gardé : {:?}",
        app.exercise_drafts.keys().collect::<Vec<_>>()
    );

    // Le code de départ revient donc tel que la série le donne.
    app.load_exercise_now(&premier);
    assert_eq!(
        app.source, pack.exercises[0].starter_code,
        "l'étape rouvre sur le code de départ"
    );
    assert!(!app.buffer_modified(), "et il n'y a rien à enregistrer");
}

/// Chaque exercice a son onglet, et porte son nom.
///
/// Trois onglets « sans titre » côte à côte ne disent rien ; le titre de
/// l'exercice dit exactement ce qu'on travaille. Et y revenir rend ce
/// qu'on y avait écrit : c'est tout l'intérêt d'ouvrir le suivant sans
/// abandonner le précédent.
#[test]
fn each_exercise_gets_its_own_named_tab() {
    let pack = builtin_pack();
    let premier = namsdec_exercises::full_id(&pack, &pack.exercises[0]);
    let second = namsdec_exercises::full_id(&pack, &pack.exercises[1]);

    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    app.load_exercise_now(&premier);
    assert!(app.tabs.is_empty(), "le tampon vierge laisse la place");
    app.source.push_str("    ; mon essai\n");

    app.load_exercise_now(&second);
    assert_eq!(app.tabs.len(), 1, "le premier attend dans son onglet");
    assert_eq!(app.current_exercise.as_deref(), Some(second.as_str()));

    // Les onglets portent le titre de leur exercice, pas « sans titre ».
    let noms: Vec<String> = app.tab_labels().into_iter().map(|(nom, _)| nom).collect();
    assert_eq!(noms.len(), 2, "{noms:?}");
    assert!(
        noms.iter().any(|n| n == &pack.exercises[0].title.fr)
            && noms.iter().any(|n| n == &pack.exercises[1].title.fr),
        "les deux titres doivent se lire : {noms:?}"
    );

    // Revenir au premier rend ce qu'on y avait écrit.
    app.load_exercise_now(&premier);
    assert_eq!(app.current_exercise.as_deref(), Some(premier.as_str()));
    assert!(
        app.source.contains("; mon essai"),
        "l'essai laissé dans l'exercice précédent est toujours là"
    );
    assert_eq!(app.tabs.len(), 1, "et l'on n'a pas ouvert un troisième");
}

/// Le parcours livré est un fichier, et il s'installe.
///
/// Les exercices ne sont plus compilés dans le programme : sans le
/// fichier, il n'y a pas de parcours, et il vaut mieux que le dépôt s'en
/// aperçoive avant l'utilisateur. Ce test vérifie donc ce que le dépôt
/// livre — le fichier lui-même —, et qu'`install.sh` le pose bien.
#[test]
fn the_shipped_course_is_a_file_that_install_puts_in_place() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    // Le fichier est dans le dépôt, et il se lit.
    let fichier = racine.join("crates/namsdec-exercises/packs/namsdec.json");
    assert!(fichier.is_file(), "{} manque", fichier.display());
    let pack = namsdec_exercises::pack::read(&fichier).expect("la série livrée se lit");
    assert_eq!(pack.id, namsdec_exercises::BUILTIN_ID);
    assert!(pack.exercises.len() >= 96, "{}", pack.exercises.len());

    // Et l'installateur le pose là où le programme va le chercher.
    let install = std::fs::read_to_string(racine.join("install.sh")).expect("install.sh");
    assert!(
        install.contains("packs/namsdec.json"),
        "install.sh doit installer le parcours"
    );
    assert!(
        install.contains("exercises"),
        "et dans le dossier où le programme le lit"
    );

    // Le programme, lui, ne le porte plus : plus un seul énoncé dans le
    // code du crate qui les définissait.
    let lib = std::fs::read_to_string(racine.join("crates/namsdec-exercises/src/lib.rs"))
        .expect("le crate des exercices");
    assert!(
        !lib.contains("include_str!"),
        "les programmes des exercices ne doivent plus être compilés dans le binaire"
    );
}

/// La fenêtre des exercices s'ouvre assez large, sur tous les écrans.
///
/// 480 px de large étaient trop peu : un énoncé y tombait sur six lignes.
/// Une taille fixe plus grande aurait déplacé le problème — elle déborde
/// d'un petit écran —, d'où une taille proportionnée et plafonnée.
#[test]
fn the_exercises_window_opens_wide_enough_on_any_screen() {
    for (largeur, hauteur) in [(1280.0, 800.0), (1999.0, 1208.0), (3840.0, 2160.0)] {
        let ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, hauteur),
            )),
            ..Default::default()
        };
        let mut taille = egui::Vec2::ZERO;
        let mut sortie = ctx.run_ui(entrée, |ui| {
            taille = exercises_window_size(ui.ctx());
        });
        sortie.textures_delta.clear();

        assert!(
            taille.x >= 480.0,
            "{largeur}x{hauteur} : {} de large, ce n'est pas plus qu'avant",
            taille.x
        );
        // Et jamais plus grande que l'écran : une fenêtre dont on ne
        // peut pas atteindre le bord ne se redimensionne plus.
        assert!(
            taille.x <= largeur && taille.y <= hauteur,
            "{largeur}x{hauteur} : {taille:?} déborde de l'écran"
        );
    }

    // Sur un écran minuscule, elle se contente de ce qu'il y a — mais
    // reste utilisable.
    let ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(500.0, 400.0),
        )),
        ..Default::default()
    };
    let mut taille = egui::Vec2::ZERO;
    let mut sortie = ctx.run_ui(entrée, |ui| taille = exercises_window_size(ui.ctx()));
    sortie.textures_delta.clear();
    assert!(taille.x >= 420.0 && taille.y >= 320.0, "{taille:?}");
}

/// Un but manqué dit laquelle de ses conditions manque, et ce que
/// l'expression vaut à la place.
///
/// « pas encore » tout seul est un verdict, non une explication : sur un
/// but à plusieurs conditions on ne savait pas laquelle regarder, et
/// l'on relançait au hasard. Ce que namsdec sait — la valeur — il le
/// dit. Ce qu'il ne sait pas — comment y arriver — il se tait, parce
/// que c'est le geste qu'on est venu apprendre.
#[test]
fn a_missed_goal_says_which_condition_misses_and_what_the_value_is() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = app_with_exercises();
    app.load_exercise("namsdec/boucle");
    // Un programme qui s'arrête proprement, mais avec la mauvaise
    // valeur : c'est exactement le cas où l'on veut être renseigné.
    app.source = "\
bits 64
global _start
section .text
_start:
    mov rax, 12
    hlt
"
    .to_owned();
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let (lignes, atteint) = app.exercise_goal().expect("un but à vérifier");
    assert!(!atteint, "12 n'est pas 15");
    let ligne = lignes.first().expect("une condition");
    assert_eq!(ligne.texte, "rax == 0xf");
    assert!(!ligne.tenue);
    assert_eq!(
        ligne.constat,
        Some(("rax".to_owned(), 12)),
        "le constat nomme l'expression et donne sa valeur"
    );

    // Et cela s'écrit dans la fenêtre dédiée aux exercices.
    app.show_exercises = true;
    let peint = paint(&mut app);
    let s = strings(app.lang);
    assert!(
        peint.contains("rax == 0xf"),
        "la condition manquante :\n{peint}"
    );
    // LE CONSTAT SE SÉPARE DE LA CONDITION, et ne répète plus le nom du
    // registre : « rax == 0xf — il vaut 0xc ». Les deux phrases
    // s'écrivaient à six pixels l'une de l'autre, en chasse fixe, et se
    // lisaient comme une seule (capture du 16 novembre 2026, « RAX vaut
    // 2 RAX vaut 1 »).
    assert!(
        peint.contains(&format!("— {} 0xc", s.goal_actual)),
        "et ce que rax vaut à la place, séparé :\n{peint}"
    );
    assert!(
        !peint.contains(&format!("rax {} 0xc", s.goal_is)),
        "sans répéter le registre que la condition vient de nommer :\n{peint}"
    );

    // Une fois le but atteint, plus de constat : répéter la valeur sous
    // une condition qui tient n'apprend rien.
    app.source = app.source.replace("mov rax, 12", "mov rax, 15");
    app.do_assemble();
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    let (lignes, atteint) = app.exercise_goal().expect("un but à vérifier");
    assert!(atteint, "15 est 15");
    assert_eq!(lignes[0].constat, None, "rien à expliquer d'un but atteint");
}

/// Un but manqué dit aussi ce que le programme A FAIT.
///
/// La condition dit ce que RAX vaut ; elle ne dit pas pourquoi. « La
/// boucle de la ligne 6 a tourné 4 fois » le dit, et c'est le moment
/// précis où quelqu'un qui apprend abandonne ou comprend.
///
/// Rien n'est déduit : ce sont des compteurs. Le module refuse de
/// conclure sur une branche dont la suite a d'autres entrées — son
/// compteur mélangerait les passages venus d'ailleurs, et la phrase
/// serait fausse exactement dans les cas intéressants.
#[test]
fn a_missed_goal_also_says_what_the_program_did() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = app_with_exercises();
    app.load_exercise("namsdec/boucle");
    // Une boucle qui tourne quatre fois au lieu de cinq : le cas
    // d'école, et celui qu'on ne sait pas lire tout seul.
    app.source = "\
bits 64
global _start
section .text
_start:
    xor rax, rax
    mov rcx, 4
boucle:
    add rax, rcx
    dec rcx
    jnz boucle
    hlt
"
    .to_owned();
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    for _ in 0..200 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let dit = app.ce_que_le_programme_a_fait();
    assert!(
        dit.iter().any(|p| p.contains("a tourné 4 fois")),
        "la boucle dit combien de tours elle a faits — {dit:?}"
    );
    assert!(
        dit.len() <= 2,
        "deux phrases au plus : une liste de quinze n'explique plus rien — {dit:?}"
    );

    // Et cela ne paraît que sous un but MANQUÉ : un but atteint n'a
    // rien à expliquer.
    let (_, atteint) = app.exercise_goal().expect("un but à vérifier");
    assert!(!atteint, "la somme de 1 à 4 n'est pas celle de 1 à 5");
    app.show_exercises = true;
    let peint = paint(&mut app);
    assert!(
        peint.contains("a tourné 4 fois"),
        "et cela s'écrit sous le but, dans la fenêtre des exercices :\n{peint}"
    );
}

/// CHAQUE exercice tient ce qu'il annonce : son but est atteint, et son
/// budget est celui qu'il consomme.
///
/// Le contrôle d'à côté en vérifie cinq, nommés à la main. Celui-ci les
/// prend tous, de toutes les séries du dépôt — et il vérifie deux
/// choses de plus que le regard d'un relecteur ne peut tenir sur cent
/// exercices :
///
/// - le BUT est atteint par le programme livré. Un but inatteignable
///   fait chercher son erreur à quelqu'un qui n'en a pas fait ;
/// - le BUDGET est celui qu'on consomme vraiment. Il est présenté comme
///   « la solution livrée le tient, descendre en dessous est l'exercice » :
///   un budget plus généreux que la solution rend l'exercice vide, un
///   budget plus serré le rend impossible. Les deux découragent, et
///   aucun ne se voit à la lecture.
#[test]
fn every_exercise_keeps_its_promise() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut séries = vec![builtin_pack()];
    // Les séries livrées comme plugins, quand il y en a.
    if let Ok(entrées) = std::fs::read_dir(racine.join("plugins/exercices")) {
        for entrée in entrées.flatten() {
            if entrée.path().extension().is_some_and(|e| e == "json")
                && let Ok(pack) = namsdec_exercises::pack::read(entrée.path())
            {
                séries.push(pack);
            }
        }
    }

    // Les séries sont-elles toutes là ? La livrée en compte quatre-vingt-
    // seize, et le dépôt en porte d'autres sous forme de plugins. Sans
    // ce garde-fou, un chemin qui change ferait passer ce contrôle au
    // vert en ne vérifiant plus rien — et c'est la façon de mentir qui
    // ne se signale pas.
    let combien: usize = séries.iter().map(|p| p.exercises.len()).sum();
    assert!(
        combien > 100,
        "{combien} exercices relevés dans {} série(s) — le contrôle \
         regarde-t-il au bon endroit ?",
        séries.len()
    );
    assert!(
        séries.iter().any(|p| p.id != namsdec_exercises::BUILTIN_ID),
        "les séries livrées en plugin doivent être vérifiées aussi : {:?}",
        séries.iter().map(|p| &p.id).collect::<Vec<_>>()
    );

    let mut fautifs: Vec<String> = Vec::new();
    for pack in &séries {
        for exercice in &pack.exercises {
            // Un exercice sans but est une démonstration à regarder : il
            // n'y a rien à vérifier, et l'exécuter ne prouverait rien.
            if exercice.goal.is_empty() {
                continue;
            }
            let mut app = NamsdecApp {
                source: exercice.starter_code.clone(),
                ..Default::default()
            };
            app.do_assemble();
            if !matches!(app.status, Status::AssembleOk) {
                fautifs.push(format!(
                    "{}/{} ne s'assemble pas : {:?}",
                    pack.id, exercice.id, app.status
                ));
                continue;
            }
            app.do_run();
            let ctx = egui::Context::default();
            for _ in 0..200 {
                if !app.running {
                    break;
                }
                app.tick_run(&ctx);
            }
            if app.running {
                fautifs.push(format!("{}/{} ne rend pas la main", pack.id, exercice.id));
                continue;
            }
            let Some(emulator) = app.emulator.as_ref() else {
                fautifs.push(format!("{}/{} n'a pas de machine", pack.id, exercice.id));
                continue;
            };
            let regs = emulator.cpu().regs.clone();
            // TOUS les exercices n'atteignent pas leur but : certains
            // sont des SQUELETTES, et c'est tout l'exercice — « À faire :
            // remettez-les à l'endroit, avec deux instructions ».
            // Exiger le but de ceux-là reviendrait à reprocher au sujet
            // de ne pas être sa propre correction.
            //
            // Ce qui vaut pour tous, en revanche : le but doit SE LIRE.
            // Une condition mal écrite ne dira jamais « atteint », quoi
            // qu'on fasse, et personne ne saura pourquoi.
            let mut tout_atteint = true;
            for condition in &exercice.goal {
                let Some(analysée) = condition::parse(condition) else {
                    fautifs.push(format!(
                        "{}/{} : « {condition} » ne se lit pas",
                        pack.id, exercice.id
                    ));
                    tout_atteint = false;
                    continue;
                };
                if !analysée.holds(&regs, None) {
                    tout_atteint = false;
                }
            }
            // Le budget ne se vérifie que sur un exercice COMPLET : sur
            // un squelette, le compte est celui du sujet, non celui de
            // la solution.
            //
            // Et l'égalité stricte serait fausse. « Branchements » invite
            // à essayer trois valeurs d'indice, dont deux coûtent une
            // instruction de plus que celle qui est livrée : un budget
            // calé sur le chemin le plus court ferait échouer deux des
            // trois essais que l'énoncé propose.
            //
            // Ce qui compte, et qui se vérifie : un budget PLUS SERRÉ que
            // la solution livrée la rend impossible, et personne ne peut
            // le deviner ; un budget BEAUCOUP plus large rend l'exercice
            // vide, puisqu'on l'atteint sans rien chercher. Entre les
            // deux, l'auteur a pu tenir compte d'un autre chemin.
            if tout_atteint && let Some(budget) = exercice.par {
                let exécutées = emulator.executed();
                if budget < exécutées {
                    fautifs.push(format!(
                        "{}/{} : budget {budget} plus serré que la solution livrée, \
                         qui en prend {exécutées} — impossible",
                        pack.id, exercice.id
                    ));
                } else if budget > exécutées + 3 {
                    fautifs.push(format!(
                        "{}/{} : budget {budget} pour une solution qui en prend \
                         {exécutées} — il n'y a rien à chercher",
                        pack.id, exercice.id
                    ));
                }
            }
        }
    }
    assert!(
        fautifs.is_empty(),
        "{} exercice(s) qui ne tiennent pas leur promesse :\n  {}",
        fautifs.len(),
        fautifs.join("\n  ")
    );
}

/// La série « atelier » est faite d'exercices COMPLETS : chacun atteint
/// son but tel qu'il est livré.
///
/// C'est ce qui la distingue de la série d'accueil, où plusieurs
/// exercices sont des squelettes à finir — « À faire : remettez-les à
/// l'endroit ». Ici le programme marche, et le travail est de le
/// comprendre puis de faire plus court.
///
/// Le contrôle d'à côté ne peut pas voir un but faux : sur un squelette,
/// un but non atteint est normal, et rien ne distingue les deux dans le
/// format. Pour cette série-ci, on peut l'exiger — et il faut l'exiger,
/// parce qu'un exercice qui n'atteint pas ce qu'il annonce envoie
/// chercher une erreur là où il n'y en a pas.
#[test]
fn the_workshop_exercises_are_complete_and_reach_their_goal() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/exercices/atelier.json");
    let pack = namsdec_exercises::pack::read(&chemin).expect("la série atelier se lit");
    assert!(
        pack.exercises.len() >= 8,
        "la série doit être fournie : {} exercices",
        pack.exercises.len()
    );

    let mut fautifs = Vec::new();
    for exercice in &pack.exercises {
        assert!(
            !exercice.goal.is_empty(),
            "{} : chaque exercice de cette série a un but vérifiable",
            exercice.id
        );
        let mut app = NamsdecApp {
            source: exercice.starter_code.clone(),
            ..Default::default()
        };
        app.do_assemble();
        if !matches!(app.status, Status::AssembleOk) {
            fautifs.push(format!(
                "{} ne s'assemble pas : {:?}",
                exercice.id, app.status
            ));
            continue;
        }
        app.do_run();
        let ctx = egui::Context::default();
        for _ in 0..500 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }
        let Some(emulator) = app.emulator.as_ref() else {
            fautifs.push(format!("{} n'a pas de machine", exercice.id));
            continue;
        };
        if app.running {
            fautifs.push(format!(
                "{} ne rend pas la main ({} instructions)",
                exercice.id,
                emulator.executed()
            ));
            continue;
        }
        let regs = emulator.cpu().regs.clone();
        for condition in &exercice.goal {
            let analysée = condition::parse(condition)
                .unwrap_or_else(|| panic!("{} : « {condition} » ne se lit pas", exercice.id));
            if !analysée.holds(&regs, None) {
                let (gauche, droite) = analysée.sides(&regs, None).unwrap_or((0, 0));
                fautifs.push(format!(
                    "{} : « {condition} » — on trouve {gauche:#x} et non {droite:#x}",
                    exercice.id
                ));
            }
        }
    }
    assert!(
        fautifs.is_empty(),
        "{} exercice(s) de l'atelier qui ne marchent pas :\n  {}",
        fautifs.len(),
        fautifs.join("\n  ")
    );
}

/// Un exercice SANS but se termine quand on l'a regardé tourner.
///
/// Sept des cent quatorze sont des programmes à observer, et le
/// tout premier du parcours en est un. `exercise_goal` rend `None` pour
/// eux, et `note_goal_reached` sortait là sans rien noter : on allait
/// jusqu'au `hlt`, la pastille restait creuse, « en cours » ne partait
/// jamais, et rien ne disait pourquoi. Un parcours dont la PREMIÈRE
/// étape ne peut pas se finir n'invite pas à faire la deuxième.
///
/// Le contrôle prend le premier exercice sans but que la série porte,
/// et non un identifiant écrit ici : celui du jour peut changer de nom,
/// la règle non.
#[test]
fn a_demonstration_is_finished_once_it_has_been_watched_through() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let pack = builtin_pack();
    let démo = pack
        .exercises
        .iter()
        .find(|e| e.goal.is_empty())
        .expect("la série porte au moins un exercice à regarder");
    let id = namsdec_exercises::full_id(&pack, démo);

    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![pack.clone()],
        ..Default::default()
    };
    app.load_exercise(&id);
    assert!(
        app.exercise_is_demo(),
        "« {id} » n'a pas de but à atteindre"
    );

    // Rien n'est noté tant qu'on n'a pas lancé : charger un exercice ne
    // le fait pas.
    paint(&mut app);
    assert!(
        !app.exercises_done.contains(&id),
        "un exercice seulement chargé n'est pas fait"
    );

    app.do_assemble();
    app.do_run();
    for _ in 0..200 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    paint(&mut app);
    assert!(
        app.exercises_done.contains(&id),
        "« {id} » a tourné jusqu'à son arrêt : il est fait. {:?}",
        app.exercises_done
    );
}

/// Et le panneau le DIT, au lieu de se taire.
///
/// Le but d'un exercice ordinaire s'affiche sous son titre ; pour une
/// démonstration, rien ne s'affichait du tout. On cherchait le but
/// qu'on venait de lire dans l'énoncé, il n'y en avait pas, et rien ne
/// disait que c'était normal.
#[test]
fn a_demonstration_says_that_it_is_one() {
    let pack = builtin_pack();
    let démo = pack
        .exercises
        .iter()
        .find(|e| e.goal.is_empty())
        .expect("la série porte au moins un exercice à regarder");
    let id = namsdec_exercises::full_id(&pack, démo);
    let mut app = NamsdecApp {
        exercises: true,
        exercise_packs: vec![pack],
        show_exercises: true,
        ..Default::default()
    };
    app.load_exercise(&id);
    let s = strings(app.lang);
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.goal_demo),
        "le panneau doit annoncer une démonstration :\n{peint}"
    );
    assert!(
        peint.contains(s.goal_demo_watch),
        "et dire ce qu'on en attend :\n{peint}"
    );
}

/// **La réponse écrite dans la série est celle que la machine donne.**
///
/// C'est l'invariant qui fait exister les exercices de lecture : une
/// réponse écrite à la main dans un fichier JSON est une affirmation —
/// la mienne, et je me suis trompé deux fois en les écrivant. Passée à
/// l'épreuve de l'émulateur, c'est un fait.
///
/// Le programme est assemblé par le vrai nasm, exécuté jusqu'à son
/// arrêt, et la condition de la réponse est évaluée par la même
/// grammaire que les buts. Un exercice dont la réponse ne tiendrait pas
/// serait un exercice qui apprend quelque chose de faux, et c'est la
/// seule chose qu'un atelier ne peut pas se permettre.
#[test]
fn la_réponse_d_un_exercice_de_lecture_est_celle_que_la_machine_donne() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let pack = builtin_pack();
    let questions: Vec<&namsdec_exercises::Exercise> = pack
        .exercises
        .iter()
        .filter(|e| e.question.is_some())
        .collect();
    assert!(
        questions.len() >= 6,
        "un exercice de lecture tout seul ne fait pas un apprentissage : {}",
        questions.len()
    );

    // TOUTES LES RÉPONSES D'UN COUP, et non la première qui tombe : un
    // relevé qui s'arrête au premier écart fait recommencer l'épreuve
    // autant de fois qu'il y a de fautes — et une épreuve ici coûte un
    // assemblage et une exécution.
    let mut fautes = Vec::new();
    for exercice in &questions {
        let question = exercice.question.as_ref().unwrap();
        let mut app = NamsdecApp {
            source: exercice.starter_code.clone(),
            ..Default::default()
        };
        app.do_assemble();
        if !matches!(app.status, Status::AssembleOk) {
            fautes.push(format!(
                "{} ne s'assemble pas : {:?}",
                exercice.id, app.status
            ));
            continue;
        }
        app.do_run();
        for _ in 0..50 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
        let Some(emulator) = &app.emulator else {
            fautes.push(format!("{} ne tourne pas", exercice.id));
            continue;
        };
        let cpu = emulator.cpu();
        if !cpu.halted {
            fautes.push(format!("{} ne s'arrête pas", exercice.id));
            continue;
        }
        // La condition de la réponse, lue par la grammaire des buts.
        let Some(condition) = condition::parse(&question.réponse) else {
            fautes.push(format!(
                "{} : « {} » ne se lit pas comme une condition",
                exercice.id, question.réponse
            ));
            continue;
        };
        let mémoire = PeekMemory(&cpu.mem);
        if !condition.holds(&cpu.regs, Some(&mémoire)) {
            // CE QUE LA MACHINE DIT, et non seulement « c'est faux » :
            // sans cela, il faut relancer le programme à la main pour
            // trouver la bonne réponse — ce que ce contrôle vient de
            // faire.
            let constat = condition
                .sides(&cpu.regs, Some(&mémoire))
                .map(|(gauche, _)| format!("{} vaut {gauche:#x}", condition.left()))
                .unwrap_or_else(|| "illisible".to_owned());
            fautes.push(format!(
                "{} annonce « {} », mais {constat}",
                exercice.id, question.réponse
            ));
        }
    }
    assert!(fautes.is_empty(), "{fautes:#?}");
}

/// Un exercice de lecture donne un programme COMPLET, et dit qu'on n'a
/// pas à l'écrire.
///
/// **C'est ce qui le distingue des cent huit autres**, et ce qu'un
/// énoncé doit dire d'emblée : quelqu'un qui croit avoir un trou à
/// remplir cherche ce qui manque, ne trouve rien, et conclut que
/// l'exercice est cassé.
#[test]
fn un_exercice_de_lecture_dit_qu_il_ne_se_remplit_pas() {
    let pack = builtin_pack();
    let questions: Vec<&namsdec_exercises::Exercise> = pack
        .exercises
        .iter()
        .filter(|e| e.question.is_some())
        .collect();

    for exercice in &questions {
        let question = exercice.question.as_ref().unwrap();
        // L'énoncé le dit, dans les deux langues.
        assert!(
            exercice
                .description
                .fr
                .contains("Ne touchez pas au programme"),
            "{} ne dit pas qu'il est complet",
            exercice.id
        );
        assert!(
            exercice
                .description
                .en
                .contains("Do not change the program"),
            "{} ne le dit pas en anglais",
            exercice.id
        );
        // Et le programme le redit en tête, là où on le lit vraiment :
        // dans l'éditeur, pas dans la liste.
        assert!(
            exercice.starter_code.contains("À LIRE, pas à écrire"),
            "{} ne le redit pas dans son code",
            exercice.id
        );
        // La question est posée dans les deux langues, et se termine
        // par un point d'interrogation — c'est une question.
        for texte in [question.texte.fr.as_str(), question.texte.en.as_str()] {
            assert!(texte.ends_with('?'), "{} : « {texte} »", exercice.id);
        }
        // PAS DE BUT À ATTEINDRE : il n'y a rien à écrire, donc rien à
        // réussir au sens des autres. Un but afficherait la réponse —
        // « rax == 0x20 » sous une question qui demande RAX.
        assert!(
            exercice.goal.is_empty(),
            "{} afficherait sa réponse dans son but",
            exercice.id
        );
        // ET L'ÉNONCÉ NE LA DONNE PAS NON PLUS. C'est le défaut qu'on
        // ne verrait pas en relisant le code : les douze exercices
        // `re_` disent « à l'arrêt, RAX vaut 1 » dans leur énoncé,
        // parce que c'était leur but. Ici, ce serait la réponse.
        let valeur = namsdec_exercises::reponse::attendue(&question.réponse);
        let nombre = namsdec_exercises::reponse::nombre(valeur);
        // DEUX ÉCRITURES, ET PAS LA TROISIÈME. L'hexadécimal NU est
        // écarté, et c'est la première version de ce contrôle qui l'a
        // appris : pour quinze, il vaut « f », une lettre qu'on trouve
        // dans n'importe quelle phrase française. Un contrôle qui crie
        // au loup sur la lettre f ne sera plus lu.
        let écritures: Vec<String> = match nombre {
            Some(n) if n > 9 => vec![format!("{n}"), format!("{n:#x}")],
            // Sous dix, un chiffre se rencontre partout — « 5 » est
            // dans « mov rax, 5 », qui est l'énoncé lui-même.
            _ => Vec::new(),
        };
        // L'ÉNONCÉ, ET NON LE PROGRAMME, et la distinction s'est
        // imposée d'elle-même : `prevoir_octet_choisi` porte
        // « db 0x11, 0x22, 0x33, 0x44 » et sa réponse est 0x33. Elle
        // est donc écrite dans le code — et c'est tout l'exercice :
        // les quatre octets sont là, la question est de savoir LEQUEL
        // est lu. Les données qu'on doit lire ne sont pas une réponse
        // donnée ; une phrase d'énoncé, si.
        let texte = format!("{} {}", exercice.description.fr, exercice.description.en);
        for écriture in écritures {
            assert!(
                !texte.contains(&écriture),
                "{} donne sa réponse (« {écriture} ») au lieu de la demander",
                exercice.id
            );
        }
    }

    // Sur plusieurs paliers : prévoir ce qu'une instruction fait
    // s'apprend dès le premier jour, et ne cesse jamais.
    let paliers: std::collections::BTreeSet<namsdec_exercises::Level> =
        questions.iter().map(|e| e.level).collect();
    assert!(
        paliers.len() >= 3,
        "la lecture s'apprend par paliers, pas d'un bloc : {paliers:?}"
    );
}

/// La question d'un exercice de lecture se pose dans le panneau, et le
/// verdict ne donne pas la réponse.
///
/// Trois états mesurés : rien d'écrit, une réponse fausse, la bonne. Un
/// contrôle qui ne verrait que le dernier passerait aussi si le panneau
/// disait « juste » à tout.
#[test]
fn la_question_se_pose_dans_le_panneau_et_le_verdict_ne_vend_pas_la_réponse() {
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        exercise_packs: vec![builtin_pack()],
        exercises: true,
        // LA FENÊTRE DES EXERCICES, parce que c'est là que le but d'un
        // exercice vit — et non dans le bandeau, qui restait affiché
        // quand on chargeait un binaire sans rapport. La question suit
        // le but, là où l'on lit l'énoncé.
        show_exercises: true,
        ..Default::default()
    };
    app.load_exercise("namsdec/prevoir_decalage");
    let question = app
        .exercise_question()
        .map(|(q, _)| q.texte.fr.clone())
        .expect("l'exercice pose une question");
    assert_eq!(question, "Que vaut RAX à l'arrêt ?");

    // 1. RIEN D'ÉCRIT : la question est là, aucun verdict.
    //
    // L'ÉTIQUETTE ET NON LA QUESTION, et la nuance a fait tomber la
    // première écriture de ce contrôle : le programme de l'exercice
    // REDIT sa question en commentaire — « ; À LIRE, pas à écrire. Que
    // vaut RAX à l'arrêt ? » —, et chercher la question dans ce qui est
    // peint la trouvait dans l'éditeur, panneau éteint ou non.
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.question_label),
        "la question est posée par le panneau : {peint}"
    );
    assert!(
        peint.contains(s.question_answer),
        "et le champ invite à répondre : {peint}"
    );
    assert!(
        !peint.contains(s.question_right) && !peint.contains(s.question_wrong),
        "mais rien n'est jugé avant qu'on ait écrit : {peint}"
    );

    // 2. UNE RÉPONSE FAUSSE : on le dit, SANS donner la bonne.
    app.question_answer = "30".to_owned();
    let peint = paint(&mut app);
    assert!(peint.contains(s.question_wrong), "{peint}");
    assert!(
        peint.contains(s.question_run_to_see),
        "et l'on dit où est l'arbitre : {peint}"
    );
    // LA RÉPONSE N'EST PAS DANS LE BLOC DE LA QUESTION. C'est tout
    // l'exercice : « 32 » ou « 0x20 » affichés là le rendraient inutile.
    //
    // LE BLOC, ET NON TOUT L'ÉCRAN, et la première écriture de ce
    // contrôle s'y est trompée : la fenêtre liste les cent quatorze
    // énoncés, et le troisième parle d'« un registre 32 bits ». Un
    // contrôle qui cherche « 32 » dans tout ce qui est peint le trouve
    // toujours, et ne regarde donc rien.
    let bloc = bloc_de_la_question(&peint);
    for écriture in ["32", "0x20"] {
        assert!(
            !bloc.contains(écriture),
            "« {écriture} » ne doit pas être montré : {bloc}"
        );
    }

    // 3. LA BONNE, écrite comme on veut : « 0x20 » vaut « 32 ».
    app.question_answer = "0x20".to_owned();
    let peint = paint(&mut app);
    assert!(peint.contains(s.question_right), "{peint}");
    assert!(
        !peint.contains(s.question_run_to_see),
        "et il n'y a plus rien à aller voir : {peint}"
    );
}

/// Un exercice de lecture ne se note pas fait en appuyant sur
/// « Exécuter ».
///
/// **C'est le défaut que ce lot devait éviter**, et il serait passé
/// inaperçu : ces exercices n'ont pas de but, donc la règle des
/// démonstrations — avoir exécuté jusqu'à l'arrêt — les aurait notés
/// faits au premier lancement. Leur programme MARCHE DÉJÀ : la pastille
/// serait passée au vert en sautant exactement le geste que l'exercice
/// enseigne.
#[test]
fn un_exercice_de_lecture_ne_se_réussit_pas_en_exécutant() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        exercise_packs: vec![builtin_pack()],
        exercises: true,
        ..Default::default()
    };
    app.load_exercise("namsdec/prevoir_decalage");
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        app.emulator.as_ref().is_some_and(|e| e.cpu().halted),
        "le programme est allé jusqu'au bout"
    );

    // Exécuté jusqu'à l'arrêt, et toujours pas fait : il reste à
    // répondre.
    app.note_goal_reached();
    assert!(
        !app.exercises_done.contains("namsdec/prevoir_decalage"),
        "exécuter ne suffit pas"
    );
    // Une réponse fausse ne suffit pas non plus.
    app.question_answer = "30".to_owned();
    app.note_goal_reached();
    assert!(!app.exercises_done.contains("namsdec/prevoir_decalage"));
    // La bonne, oui.
    app.question_answer = "32".to_owned();
    app.note_goal_reached();
    assert!(
        app.exercises_done.contains("namsdec/prevoir_decalage"),
        "répondre juste le réussit"
    );

    // ET CE N'EST PAS UNE DÉMONSTRATION : lui dire « regardez tourner »
    // serait lui retirer sa question.
    assert!(!app.exercise_is_demo());
}

/// Ce que le bloc de la question écrit, et rien d'autre.
///
/// `paint` rend TOUT l'écran, et la fenêtre des exercices liste les cent
/// quatorze énoncés : y chercher un nombre le trouve toujours quelque
/// part. On découpe donc entre l'étiquette de la question et ce qui la
/// suit immédiatement dans la fenêtre — la ligne « Chercher : ».
fn bloc_de_la_question(peint: &str) -> String {
    let s = strings(Lang::Fr);
    let début = peint
        .find(s.question_label)
        .expect("le bloc de la question est peint");
    let reste = &peint[début..];
    let fin = reste.find(s.exercises_search).unwrap_or(reste.len());
    reste[..fin].to_owned()
}

/// L'ÉNONCÉ ET LE BUT RESTENT SOUS LES YEUX, AU MODE SIMPLE.
///
/// C'était le trou de ce mode-là, et il ne se voyait qu'en s'en
/// servant. Au mode avancé, l'énoncé et le but vivent dans le panneau
/// latéral, qui est toujours là. L'Établi n'a pas de panneau latéral :
/// on ouvrait « Exercices… », on chargeait une étape, on refermait la
/// fenêtre — et l'on se retrouvait seul devant un programme à trous,
/// sans l'énoncé, sans le but, sans savoir si l'on avait réussi. Le
/// parcours se consultait au lieu de se faire.
///
/// LA FENÊTRE EST FERMÉE PENDANT LA MESURE, et c'est tout le sujet :
/// peindre l'énoncé dans la fenêtre qu'on vient de fermer ne prouve
/// rien. Ce contrôle regarde ce qui reste à l'écran quand elle n'est
/// plus là.
///
/// L'ÉMULATEUR EST POSÉ À LA MAIN, sans passer par nasm : le but ne
/// s'évalue que devant une machine arrêtée, et ce qu'on mesure ici est
/// une mise en page. Les registres sont ceux que l'exercice demande —
/// RAX à 2, RBX à 1 —, donc le but doit s'allumer.
#[test]
fn la_bande_de_l_exercice_porte_l_enonce_et_le_but() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;

    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = app_with_exercises();
    app.apply_theme(&ctx);
    app.debutant = true;
    app.load_exercise_now("namsdec/echange");
    app.show_exercises = false;

    // La machine à l'arrêt, dans l'état que l'exercice demande.
    let mut mem = Memory::new(BASE, 0x1000);
    mem.load(BASE, &[0xf4]).expect("un hlt se charge");
    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = BASE;
    cpu.regs.set64(Reg64::Rax, 2);
    cpu.regs.set64(Reg64::Rbx, 1);
    app.emulator = Some(Box::new(namsdec_emu::Emulator::new(cpu)));

    let s = strings(app.lang);
    // UN MORCEAU SANS BALISAGE. L'énoncé est peint par `prose` depuis
    // le 10 novembre 2026 : ses accents graves ouvrent des morceaux de
    // code, donc la galley ne porte plus la phrase d'un seul tenant et
    // les accents n'y sont plus. Ce contrôle cherchait les trente
    // premiers caractères du texte BRUT — « `xchg` le fait… » — et
    // tombait sur le balisage qu'on venait justement de retirer. On
    // cherche donc un morceau qui traverse les deux formes.
    let énoncé = namsdec_exercises::find(&app.exercise_packs, "namsdec/echange")
        .expect("l'étape existe")
        .description
        .get(app.lang.code())
        .to_owned();
    let début = énoncé
        .split('`')
        .map(str::trim)
        .filter(|morceau| morceau.len() > 25)
        .map(|morceau| morceau.chars().take(25).collect::<String>())
        .next()
        .expect("l'énoncé a une phrase hors des accents graves");

    // AU-DESSUS DU CODE, ET SEULEMENT LÀ. Le programme de départ d'un
    // exercice REDIT son énoncé en commentaire : chercher la phrase
    // dans tout l'écran, c'est la trouver dans l'éditeur, et le
    // contrôle passait au vert la bande retirée. On ne garde donc que
    // ce qui est peint au-dessus de la rangée des vues.
    let mut peints: Vec<String> = Vec::new();
    for _ in 0..3 {
        peints.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && cadre_du_texte(t).top() < app.zone_de_travail.top()
            {
                peints.push(t.galley.text().to_owned());
            }
        }
        sortie.textures_delta.clear();
    }
    let tout = peints.join("|");

    assert!(
        tout.contains(&début),
        "l'énoncé de l'étape doit rester à l'écran, la fenêtre fermée — peint : {tout:.400}"
    );
    // En toutes lettres : la bande de l'Établi est le mode débutant.
    assert!(
        tout.contains("RAX vaut 2") && tout.contains("RBX vaut 1"),
        "les deux conditions du but doivent être peintes, chacune avec sa \
         pastille — peint : {tout:.400}"
    );
    assert!(
        tout.contains(s.goal_reached),
        "et le but doit se dire ATTEINT : les registres sont ceux qu'il demande"
    );
    assert!(
        tout.contains(s.etabli_exercise.to_uppercase().as_str()),
        "la bande porte son titre, comme les quatre autres"
    );
}

/// ET ELLE NE PARAÎT PAS QUAND IL N'Y A RIEN À MONTRER.
///
/// Trois états où la bande ne doit prendre aucune place : aucun
/// exercice chargé — c'est le cas de quelqu'un qui écrit son propre
/// programme, et c'est le cas ordinaire —, le module éteint dans les
/// préférences, et le mode avancé, qui a son panneau latéral pour
/// cela. Une bande qui dirait « aucun exercice » prendrait cent
/// quatre-vingt-dix pixels pour l'annoncer.
#[test]
fn la_bande_de_l_exercice_ne_parait_que_s_il_y_en_a_un() {
    let mut app = app_with_exercises();
    assert!(
        !app.exercices_en_bande(),
        "rien de chargé : pas de bande — c'est l'état de qui écrit son propre programme"
    );

    app.load_exercise_now("namsdec/echange");
    assert!(
        app.exercices_en_bande(),
        "une étape chargée : la bande est là"
    );

    app.exercises = false;
    assert!(
        !app.exercices_en_bande(),
        "le module éteint, plus rien ne mène au parcours : la bande part avec"
    );
}

/// RIEN N'EST COUPÉ DANS LA BANDE, À SA HAUTEUR DE DÉPART.
///
/// La hauteur a été MESURÉE et non choisie, et ce contrôle garde la
/// mesure. À cent soixante-quatre pixels — le chiffre écrit d'instinct
/// —, la capture montrait la dernière ligne coupée des DEUX formes :
/// le budget d'instructions pour un exercice d'écriture, « exécutez
/// pour voir ce que la machine en dit » pour un exercice de lecture.
/// La ligne qui manque est à chaque fois celle qui dit quoi faire
/// ensuite.
///
/// On compare l'encre à l'encre rognée par le découpage : c'est la
/// seule façon de voir une ligne qu'egui peint et que la bande cache.
#[test]
fn la_bande_de_l_exercice_ne_coupe_pas_sa_derniere_ligne() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;

    for (étape, attendu) in [
        ("namsdec/echange", "Instructions"),
        ("namsdec/prevoir_somme", "Que vaut RAX"),
    ] {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let mut app = app_with_exercises();
        app.apply_theme(&ctx);
        app.debutant = true;
        app.load_exercise_now(étape);
        app.show_exercises = false;
        // LE PROGRAMME DE L'ÉTAPE, VRAIMENT EXÉCUTÉ.
        //
        // Un `hlt` seul posé à la main donnait une machine à
        // interroger, mais pas le CONTENU qu'elle fait écrire : un but
        // évalué porte, pour chaque condition qui manque, le constat de
        // ce que le registre vaut à la place — deux lignes de plus
        // qu'avec un programme qui n'a rien fait. La bande était donc
        // mesurée plus courte qu'elle n'est, et la capture du
        // 16 novembre 2026 a montré « RBX vaut 1 — il vaut 2 » tranché
        // par le bas pendant que ce contrôle restait vert.
        app.do_assemble();
        for _ in 0..200 {
            app.do_step();
        }
        let _ = (BASE, std::marker::PhantomData::<(Cpu, Memory)>);

        // La bande est en haut, sous la barre et la frise : on borne
        // la mesure à sa hauteur plutôt qu'à son contenu, pour ne pas
        // juger la bande sur les lignes du code.
        let mut coupés: Vec<String> = Vec::new();
        let mut vu = false;
        for _ in 0..3 {
            coupés.clear();
            vu = false;
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                let encre = cadre_du_texte(t);
                if encre.top() > 300.0 {
                    continue;
                }
                if t.galley.text().contains(attendu) {
                    vu = true;
                }
                let visible = encre.intersect(clipped.clip_rect);
                if visible.height() + 0.5 < encre.height() || visible.width() + 0.5 < encre.width()
                {
                    coupés.push(format!(
                        "« {} » : {:.0}x{:.0} peints, {:.0}x{:.0} visibles",
                        t.galley.text().chars().take(40).collect::<String>(),
                        encre.width(),
                        encre.height(),
                        visible.width().max(0.0),
                        visible.height().max(0.0),
                    ));
                }
            }
            sortie.textures_delta.clear();
        }
        assert!(
            vu,
            "{étape} : la bande doit porter sa dernière ligne — « {attendu} » n'est pas peint"
        );
        assert!(
            coupés.is_empty(),
            "{étape} : {} ligne(s) coupée(s) dans la bande de l'exercice :\n  {}",
            coupés.len(),
            coupés.join("\n  ")
        );
    }
}

/// LE CODE GARDE DE QUOI S'ÉCRIRE, MÊME AU PLUS PETIT ÉCRAN.
///
/// La bande de l'exercice prend de la place, et elle la prend AU CODE.
/// Sur un écran de 1600x1100 c'est un sixième ; à 800x520 — le plus
/// petit que l'application accepte —, cent quatre-vingt-dix pixels
/// étaient plus d'un tiers, la rangée des vues était coupée en deux et
/// il ne restait DEUX lignes de programme. Un énoncé qui chasse le
/// programme qu'il demande d'écrire ne sert à rien.
///
/// La bande prend donc une PART de ce qui reste, plafonnée à ce qu'on
/// a mesuré, et la bande de la ligne courante est bornée de même —
/// elle était la seule des trois à prendre un nombre de pixels fixe.
///
/// ON COMPTE DES LIGNES DE PROGRAMME, et non des pixels : c'est ce
/// qu'on vient faire, et c'est la seule mesure qui ne se périme pas
/// quand une police change.
///
/// LES TROIS CHIFFRES SONT MESURÉS, ET DEUX ONT BAISSÉ EXPRÈS. Sans
/// exercice, six lignes sont devenues trois le jour où la bande du bas
/// s'est mise à expliquer la ligne où est le curseur (A 9.88) : c'est
/// le prix de cette bande, payé en connaissance de cause. Avec un
/// exercice, deux : la bande du curseur s'efface alors devant celle de
/// l'exercice, et ce chiffre-ci garde la règle — sans elle, les deux
/// bandes ensemble ne laissaient plus AUCUNE ligne visible.
///
/// DEUX EST PEU, ET C'EST ÉCRIT ICI POUR CELA. À 800x520, un exercice
/// chargé ne laisse que deux lignes — avec une hauteur de bande fixe,
/// il n'en laissait AUCUNE. Les essais pour en gagner davantage sont
/// consignés là où ils ont échoué (`etabli.rs`) : rapetisser encore la
/// bande ne donne rien, la bande du résultat reprenant tout ce qu'elle
/// rend, et borner la bande de la ligne courante faisait se recouvrir
/// deux panneaux. Le seul levier qui reste est la part du résultat,
/// qui vient d'une demande explicite : c'est un arbitrage à prendre,
/// pas à deviner. Le chiffre est ici pour qu'on voie le jour où il
/// baisse, et le jour où quelqu'un le remonte.
#[test]
fn au_plus_petit_ecran_l_etabli_montre_encore_du_code() {
    for (écran, avec_exercice, plancher) in [
        (MIN_SCREEN, false, 3),
        (MIN_SCREEN, true, 2),
        // ET UN PORTABLE ORDINAIRE, qui compte plus que le minimum
        // déclaré : 1366x768 est l'écran sur lequel namsdec sera le
        // plus souvent ouvert, et personne ne le mesurait.
        ([1366.0, 768.0], true, 14),
    ] {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, écran.into())),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let mut app = app_with_exercises();
        app.apply_theme(&ctx);
        app.debutant = true;
        if avec_exercice {
            app.load_exercise_now("namsdec/echange");
        }
        // Un programme assez long pour que la place soit le seul
        // facteur, et des lignes qu'on sait reconnaître.
        app.source = (1..=30).map(|n| format!("    nop ; ligne {n}\n")).collect();
        // LA VUE REPART EN HAUT, et c'est la source posée ici qui
        // l'exige : charger une étape amène la vue sur son TROU (voir
        // `aller_au_trou`), et ce programme-ci n'est pas celui de
        // l'étape — la ligne visée n'y désigne plus rien. Sans cela, on
        // mesurait une vue défilée au hasard, dont la première ligne
        // est coupée par le haut : une ligne de moins, pour une raison
        // qui n'a rien à voir avec la place que prennent les bandes.
        app.reprise_ligne = None;
        app.show_exercises = false;

        let mut lignes = 0;
        for _ in 0..3 {
            lignes = 0;
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                for ligne in t.galley.rows.iter() {
                    if !ligne.text().contains("ligne ") {
                        continue;
                    }
                    // ENTIÈREMENT VISIBLE, et non simplement peinte :
                    // une ligne rognée par le bas se lit à moitié, ce
                    // qui ne vaut pas mieux que pas du tout.
                    let encre = ligne
                        .rect_without_leading_space()
                        .translate(t.pos.to_vec2());
                    if encre.intersect(clipped.clip_rect).height() > encre.height() - 0.5 {
                        lignes += 1;
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        eprintln!(
            "MESURE {:.0}x{:.0} exercice={avec_exercice} lignes={lignes}",
            écran[0], écran[1]
        );
        eprintln!(
            "MESURE {:.0}x{:.0} exercice={avec_exercice} lignes={lignes}",
            écran[0], écran[1]
        );
        eprintln!(
            "MESURE {:.0}x{:.0} ex={avec_exercice} lignes={lignes}",
            écran[0], écran[1]
        );
        assert!(
            lignes >= plancher,
            "à {:.0}x{:.0}, exercice = {avec_exercice} : {lignes} ligne(s) de programme \
             entièrement visibles, {plancher} attendues — les bandes ont mangé le code",
            écran[0],
            écran[1],
        );
    }
}

/// LA LIGNE OÙ EST LE CURSEUR S'EXPLIQUE, AVANT MÊME D'ASSEMBLER.
///
/// C'était le trou du premier écran, et il durait tout le temps qu'on
/// passe à LIRE un programme avant de l'exécuter — la première moitié
/// de l'apprentissage. L'Établi portait six instructions qu'un
/// débutant ne connaît pas, la bande du bas disait « revenez quand
/// vous aurez assemblé », et la seule porte vers les cent
/// quatre-vingt-neuf fiches était une touche dont rien ne parle.
///
/// Les fiches étaient écrites ; il manquait l'endroit et le moment.
///
/// TROIS ÉTATS SONT MESURÉS, parce que la bande en a trois : une ligne
/// qui a une fiche, une ligne qui n'en a pas — où elle doit dire
/// comment la faire parler plutôt que de se taire —, et un programme
/// en cours d'exécution, où elle reprend son rôle d'avant. Sans le
/// troisième, rien ne dirait que la fiche NE PREND PAS la place de ce
/// qui se passe.
#[test]
fn la_ligne_du_curseur_s_explique_avant_l_assemblage() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;

    let rendu = |app: &mut NamsdecApp| -> String {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let mut peints = String::new();
        for _ in 0..3 {
            peints.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    peints.push_str(t.galley.text());
                    peints.push('|');
                }
            }
            sortie.textures_delta.clear();
        }
        peints
    };

    let s = strings(Lang::Fr);
    let fiche = namsdec_doc::lookup("add").expect("`add` a une fiche");
    let titre = fiche.title.get("fr");

    // 1. LE CURSEUR SUR UNE INSTRUCTION, rien qui tourne.
    let mut app = NamsdecApp {
        debutant: true,
        ..Default::default()
    };
    app.cursor_byte = app
        .source
        .find("add ")
        .expect("le programme d'accueil additionne");
    let peints = rendu(&mut app);
    assert!(
        peints.contains(titre),
        "la bande doit porter le titre de la fiche de `add` — « {titre} »"
    );
    assert!(
        peints.contains(s.etabli_cursor_line.to_uppercase().as_str()),
        "et dire de quelle ligne elle parle"
    );

    // 2. LE CURSEUR SUR UN COMMENTAIRE : rien à expliquer, mais on dit
    //    comment faire parler la bande. C'est l'état du PREMIER ÉCRAN,
    //    où le curseur est au premier caractère.
    let mut app = NamsdecApp {
        debutant: true,
        ..Default::default()
    };
    app.cursor_byte = 0;
    let peints = rendu(&mut app);
    assert!(
        !peints.contains(titre),
        "une ligne de commentaire n'explique aucune instruction"
    );
    assert!(
        peints.contains(s.etabli_cursor_hint),
        "mais la bande doit dire comment la faire parler, au lieu de se taire"
    );

    // 3. UN PROGRAMME EN COURS : la fiche ne prend pas la place de ce
    //    qui se passe.
    let mut app = NamsdecApp {
        debutant: true,
        ..Default::default()
    };
    app.cursor_byte = app
        .source
        .find("add ")
        .expect("le programme d'accueil additionne");
    let mut mem = Memory::new(BASE, 0x1000);
    mem.load(BASE, &[0xf4]).expect("un hlt se charge");
    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = BASE;
    app.emulator = Some(Box::new(namsdec_emu::Emulator::new(cpu)));
    let peints = rendu(&mut app);
    assert!(
        !peints.contains(s.etabli_cursor_line.to_uppercase().as_str()),
        "un programme tourne : la bande dit ce qui se passe, pas ce qu'on lisait"
    );

    // 4. UN EXERCICE EN COURS : une seule bande d'explication à la
    //    fois. Celle de l'exercice porte déjà la consigne, et les deux
    //    ensemble ne laissaient plus aucune ligne de programme visible
    //    à 800x520 — voir
    //    `au_plus_petit_ecran_l_etabli_montre_encore_du_code`, qui
    //    garde le chiffre.
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/echange");
    app.show_exercises = false;
    app.cursor_byte = app.source.find("mov ").unwrap_or(0);
    let peints = rendu(&mut app);
    assert!(
        !peints.contains(s.etabli_cursor_line.to_uppercase().as_str()),
        "un exercice est en cours : c'est lui qui explique, et la bande du curseur \
         s'efface — sans quoi le plus petit écran ne montre plus une ligne de code"
    );
}

/// LE CONSTAT DU BUT PARLE LA MÊME LANGUE QUE LE DERNIER PAS.
///
/// Vu à l'écran le 10 novembre 2026, sur une seule image : la bande du
/// bas disait « RCX 3 -> 2 » pendant que le but, trois centimètres
/// plus haut, disait « rax vaut 0x0 ». Deux façons d'écrire le même
/// nombre sur un même écran, c'est une de trop — et celle du haut
/// demandait justement de savoir lire l'hexadécimal, ce qu'on vient
/// apprendre.
///
/// LA CONDITION, ELLE, NE BOUGE PAS. `rax == 0xf` est le texte de
/// l'exercice, pas le nôtre : la réécrire ferait dire à l'énoncé autre
/// chose que ce que son auteur a tapé. Le contrôle vérifie les deux —
/// le constat traduit, la condition intacte.
#[test]
fn le_constat_du_but_suit_la_langue_du_niveau() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;

    for débutant in [true, false] {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let mut app = app_with_exercises();
        app.apply_theme(&ctx);
        app.debutant = débutant;
        // Une étape dont le but porte `rax == 2`, et une machine qui
        // n'y est pas : c'est le constat qu'on veut lire.
        app.load_exercise_now("namsdec/echange");
        app.show_exercises = false;
        let mut mem = Memory::new(BASE, 0x1000);
        mem.load(BASE, &[0xf4]).expect("un hlt se charge");
        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = BASE;
        cpu.regs.set64(Reg64::Rax, 12);
        app.emulator = Some(Box::new(namsdec_emu::Emulator::new(cpu)));

        // LE BUT SEUL, et non tout l'écran : au mode simple il vit
        // dans la bande de l'exercice, au mode avancé dans la fenêtre
        // des exercices. Deux endroits, une seule fonction — et c'est
        // elle qu'on mesure, sans quoi le contrôle mesurerait surtout
        // lequel des deux écrans est ouvert.
        let mut peints = String::new();
        for _ in 0..3 {
            peints.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.goal_ui(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    peints.push_str(t.galley.text());
                    peints.push('|');
                }
            }
            sortie.textures_delta.clear();
        }

        // LA CONDITION SUIT LE NIVEAU, ELLE AUSSI, depuis le
        // 11 novembre 2026. Elle restait « rax == 2 » des deux côtés —
        // c'était le texte de l'auteur, et c'était l'argument. Mais au
        // mode DÉBUTANT, trois règles de l'application s'y trouvaient
        // enfreintes d'un coup : registres en majuscules, nombres en
        // décimal sous le million (A 9.87), et rien d'écrit dans une
        // syntaxe qu'on n'a pas encore apprise. Au mode avancé, rien ne
        // change : c'est celle qu'on tape dans un point d'arrêt
        // conditionnel.
        if débutant {
            assert!(
                peints.contains("RAX vaut 2"),
                "la condition se lit en toutes lettres : {peints:.600}"
            );
            // Et le constat se sépare de la condition sans répéter le
            // registre : « RAX vaut 2 — il vaut 12 » (16 novembre
            // 2026 ; les deux phrases se lisaient comme une seule).
            assert!(
                peints.contains("— il vaut 12"),
                "et le constat se dit en décimal, comme la bande du dernier pas : {peints:.600}"
            );
        } else {
            assert!(
                peints.contains("— il vaut 0xc"),
                "au-dessus du niveau débutant, l'hexadécimal reste : {peints:.600}"
            );
        }
    }
}

/// LA CIBLE SE LIT AVANT DE COMMENCER.
///
/// `exercise_goal` se tait tant qu'aucun programme ne tourne, et c'est
/// juste : on ne dit pas « pas atteint » à qui n'a encore rien lancé.
/// Mais se taire faisait qu'on écrivait SANS SAVOIR OÙ L'ON VA — les
/// conditions ne se lisaient qu'après le premier essai, alors qu'elles
/// sont ce qui dirige ce qu'on écrit.
///
/// SANS VERDICT, et le contrôle le vérifie : ni « atteint » ni « pas
/// encore » tant que rien n'a tourné. Montrer la cible est une aide ;
/// la juger avant l'essai serait un reproche.
#[test]
fn la_cible_de_l_exercice_se_lit_avant_le_premier_essai() {
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = app_with_exercises();
    app.apply_theme(&ctx);
    app.debutant = true;
    app.load_exercise_now("namsdec/echange");
    app.show_exercises = false;
    assert!(
        app.emulator.is_none(),
        "l'étape vient d'être chargée : rien n'a encore tourné"
    );

    let s = strings(app.lang);
    let mut peints = String::new();
    for _ in 0..3 {
        peints.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.goal_ui(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                peints.push_str(t.galley.text());
                peints.push('|');
            }
        }
        sortie.textures_delta.clear();
    }

    assert!(
        peints.contains("RAX vaut 2") && peints.contains("RBX vaut 1"),
        "les deux conditions doivent se lire avant d'avoir rien lancé : {peints:?}"
    );
    assert!(
        peints.contains(s.goal_ahead),
        "et l'on dit ce qu'elles sont : une cible, pas un verdict"
    );
    assert!(
        !peints.contains(s.goal_not_yet) && !peints.contains(s.goal_reached),
        "aucun verdict avant le premier essai — ce serait un reproche : {peints:?}"
    );
}

/// LA BANDE DU RÉSULTAT PORTE SA RANGÉE DE SAISIE.
///
/// Relevé à l'écran le 10 novembre 2026, une flèche verte sur
/// « Variables d'environnement » : « innaccesible ». La bande porte,
/// de haut en bas, les notes du binaire, la sortie du programme, puis
/// les rangées du bas — les arguments, l'environnement, et LE CHAMP OÙ
/// L'ON RÉPOND AU PROGRAMME. Ces rangées-là sont dans un panneau ancré
/// au bas de la sortie : la bande trop courte, elles passaient sous
/// son bord, c'est-à-dire sous le bord de la fenêtre. Le programme
/// demandait « Enter your name: » et il n'y avait plus nulle part où
/// le lui dire.
///
/// LE CAS LE PLUS HAUT EST MESURÉ — un binaire ouvert, les arguments
/// dépliés : c'est celui de la capture, et c'est celui où la bande a
/// le plus à porter. Les trois rangées doivent être ENTIÈREMENT
/// visibles : une rangée coupée en deux n'est pas plus cliquable
/// qu'une rangée absente.
#[test]
fn la_bande_du_resultat_porte_sa_rangee_de_saisie() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.debutant = true;
    app.load_elf(&octets, NOM_FABRIQUÉ);
    app.console_args_open = true;
    // ET LES NOTES DU CHARGEMENT, qui occupent le HAUT de la bande.
    //
    // Sans elles, la bande n'a que sa sortie à porter, ses deux cents
    // pixels par défaut suffisent, et le contrôle passe quoi qu'on
    // fasse du plancher. C'est l'état de la capture : un binaire lié
    // dynamiquement arrive avec une note de quatre lignes, et c'est
    // elle qui pousse les rangées du bas sous le bord.
    app.diagnostics = (0..2)
        .map(|_| namsdec_asm::Diagnostic {
            line: 0,
            severity: namsdec_asm::Severity::Warning,
            message: strings(Lang::Fr).binary_dynamic.to_owned(),
            kind: None,
        })
        .collect();

    let s = strings(app.lang);
    let attendus = [s.console_args, s.console_env, s.console_input_label];
    let mut manquants: Vec<String> = Vec::new();
    for _ in 0..4 {
        manquants.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for attendu in attendus {
            let mut vue = None;
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape
                    && t.galley.text().trim() == attendu.trim()
                {
                    let encre = cadre_du_texte(t);
                    vue = Some((encre, encre.intersect(clipped.clip_rect)));
                }
            }
            match vue {
                None => manquants.push(format!("« {attendu} » n'est pas peint")),
                Some((encre, visible)) if visible.height() + 0.5 < encre.height() => manquants
                    .push(format!(
                        "« {attendu} » est coupé : {:.0} px sur {:.0}",
                        visible.height().max(0.0),
                        encre.height()
                    )),
                Some((encre, _)) if encre.bottom() > 1100.0 => {
                    manquants.push(format!("« {attendu} » est sous le bord de la fenêtre"))
                }
                Some(_) => {}
            }
        }
        sortie.textures_delta.clear();
    }
    assert!(
        manquants.is_empty(),
        "la bande du résultat doit porter ce qu'elle porte :\n  {}",
        manquants.join("\n  ")
    );
}

/// UN EXERCICE QUI POSE UN BUT A QUELQUE CHOSE À FAIRE.
///
/// MESURÉ LE 11 NOVEMBRE 2026, ET C'EST LA RAISON DE CE CONTRÔLE : sur
/// les cent une étapes qui posent un but, QUATRE-VINGT-DIX-SEPT
/// l'atteignaient déjà telles qu'elles étaient livrées. On chargeait
/// l'étape, on cliquait « Exécuter », la pastille passait au vert — sans
/// avoir écrit une ligne. C'étaient des démonstrations commentées
/// présentées comme des devoirs, et rien ne le disait.
///
/// TROIS CHOSES SONT VÉRIFIÉES, et chacune par l'ÉMULATEUR plutôt que
/// par la forme du texte :
///
/// 1. le programme de départ s'assemble. Un exercice qui s'ouvre sur une
///    erreur rouge n'invite personne à chercher ;
/// 2. il n'atteint PAS le but. C'est ce qui empêche un exercice de
///    redevenir une démonstration sans qu'on le voie ;
/// 3. la solution, elle, l'atteint — et en exactement `par`
///    instructions, puisque le budget est par définition celui de la
///    solution livrée.
///
/// Une solution écrite à la main dans un fichier JSON est une
/// affirmation ; passée à l'épreuve de l'émulateur, c'est un fait.
///
/// Sans nasm, le contrôle s'abstient : il n'y a alors rien à assembler,
/// et crier l'absence d'un outil n'apprend rien sur les exercices.
#[test]
fn chaque_exercice_a_quelque_chose_a_faire_et_une_solution_qui_marche() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let packs = [builtin_pack()];
    // Ce que l'émulateur dit d'un programme : s'assemble-t-il, le but
    // est-il atteint, et combien d'instructions ont tourné.
    let essaie = |programme: &str, id: &str| -> (bool, bool, u64) {
        let mut app = NamsdecApp {
            source: programme.to_owned(),
            exercises: true,
            exercise_packs: vec![builtin_pack()],
            current_exercise: Some(id.to_owned()),
            ..Default::default()
        };
        app.do_assemble();
        if !matches!(app.status, Status::AssembleOk) {
            return (false, false, 0);
        }
        app.do_run();
        let ctx = egui::Context::default();
        // Une boucle bornée : un exercice dont la solution ne rend pas
        // la main est un défaut comme un autre, et il doit se voir ici
        // plutôt que figer la suite.
        for _ in 0..400 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }
        let exécutées = app.emulator.as_ref().map(|e| e.executed()).unwrap_or(0);
        // RENDRE LA MAIN, au sens où l'entend
        // `every_program_we_ship_is_a_complete_program` : s'être arrêté,
        // avoir buté sur une faute, ou attendre une entrée. « Ne tourne
        // plus » ne suffit pas — l'exécution s'interrompt aussi sur sa
        // propre borne, et un programme qui tourne sans fin passait
        // alors pour sage.
        let rendu = app.emulator.as_ref().is_some_and(|e| e.cpu().halted)
            || matches!(app.status, Status::Error(_) | Status::WaitingForInput);
        (
            rendu,
            matches!(app.exercise_goal(), Some((_, true))),
            exécutées,
        )
    };

    let mut travail = 0;
    for exercice in &packs[0].exercises {
        // Les démonstrations et les exercices de lecture n'ont rien à
        // écrire : ils se regardent, ou se répondent.
        if exercice.goal.is_empty() || exercice.question.is_some() {
            assert!(
                exercice.solution.is_none(),
                "{} n'a rien à écrire et porte pourtant une solution",
                exercice.id
            );
            continue;
        }
        travail += 1;
        let id = namsdec_exercises::full_id(&packs[0], exercice);

        // RENDRE LA MAIN FAIT PARTIE DU CONTRAT, et c'est ce qui
        // manquait : huit squelettes creusés le 11 novembre 2026
        // partaient en boucle sans fin, parce que le trou avait emporté
        // le corps de la boucle en laissant le saut de retour. Un
        // exercice qui, au premier « Exécuter », ne s'arrête jamais
        // n'apprend rien — il fait douter de l'outil. Un trou dans une
        // boucle emporte donc la boucle entière.
        let (fini, atteint, _) = essaie(&exercice.starter_code, &id);
        assert!(
            fini,
            "{} ne s'assemble pas, ou ne rend jamais la main, tel qu'il \
             est livré : un exercice ne s'ouvre ni sur une erreur ni sur \
             une boucle sans fin",
            exercice.id
        );
        assert!(
            !atteint,
            "{} atteint son but SANS QU'ON AIT RIEN ÉCRIT : ce n'est pas \
             un exercice, c'est une démonstration. Creusez-y un trou, ou \
             retirez-lui son but.",
            exercice.id
        );

        let solution = exercice
            .solution
            .as_deref()
            .unwrap_or_else(|| panic!("{} a un but et pas de solution", exercice.id));
        let (fini, atteint, coût) = essaie(solution, &id);
        assert!(
            fini,
            "la solution de {} ne s'assemble pas, ou ne rend pas la main",
            exercice.id
        );
        assert!(
            atteint,
            "la solution de {} n'atteint pas son propre but",
            exercice.id
        );
        if let Some(budget) = exercice.par {
            assert_eq!(
                coût, budget,
                "{} : le budget annoncé est {budget}, la solution livrée \
                 en coûte {coût}. Le budget EST celui de la solution.",
                exercice.id
            );
        }
    }
    assert!(
        travail >= 100,
        "le parcours n'a plus que {travail} étapes à faire : le relevé \
         regarde-t-il encore quelque chose ?"
    );
}

/// Le trou se voit, et il dit quoi faire.
///
/// Un programme amputé sans un mot est une panne, pas un exercice : on
/// relit trois fois en cherchant l'erreur. La ligne « ; À FAIRE : » est
/// la seule chose qui distingue les deux, et c'est elle que l'énoncé
/// prolonge.
#[test]
fn chaque_trou_dit_ce_qu_il_attend() {
    let pack = builtin_pack();
    for exercice in &pack.exercises {
        if exercice.solution.is_none() {
            continue;
        }
        // LE DEUX-POINTS FAIT PARTIE DU MARQUEUR : « À FAIRE DANS
        // NAMSDEC », en tête de plusieurs étapes de lecture, dit où
        // regarder dans l'application et n'est pas un trou. Les
        // confondre ferait passer pour annoncé un trou qui ne l'est
        // pas.
        let dit = exercice.starter_code.contains("À FAIRE :")
            || exercice.starter_code.contains("À faire :");
        assert!(
            dit,
            "{} a un trou que rien n'annonce : il manque une ligne \
             « ; À FAIRE : … »",
            exercice.id
        );
        // Et la solution, elle, n'en porte plus : une solution qui dit
        // encore « à faire » est une solution qu'on n'a pas finie.
        let solution = exercice.solution.as_deref().unwrap_or_default();
        assert!(
            !solution.contains("À FAIRE :") && !solution.contains("À faire :"),
            "la solution de {} porte encore une consigne",
            exercice.id
        );
    }
}

/// En ouvrant une étape, le curseur est SUR LE TROU, et la bande dit ce
/// qu'il reste à écrire.
///
/// Un exercice s'ouvre sur une vingtaine de lignes dont une seule est à
/// écrire. Sans ces deux repères, on lit l'énoncé, on regarde le
/// programme, et l'on cherche OÙ — ce qui est le seul moment de
/// l'exercice qui n'apprend rien.
#[test]
fn en_ouvrant_une_etape_le_curseur_est_sur_le_trou() {
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/boucle");

    let ligne_du_trou = app
        .source
        .lines()
        .position(|l| l.contains("À FAIRE :"))
        .expect("l'étape a un trou")
        + 1;
    assert_eq!(
        app.cursor_pos.0, ligne_du_trou,
        "le curseur doit être sur la consigne : {:?}",
        app.cursor_pos
    );
    assert_eq!(
        app.reprise_ligne,
        Some(ligne_du_trou as u32),
        "et la vue doit l'y amener"
    );

    // La bande répète la consigne, en clair et sans son point-virgule.
    let consignes = app.consignes_du_trou();
    assert_eq!(consignes.len(), 1, "{consignes:?}");
    assert!(
        consignes[0].starts_with("le corps de la boucle"),
        "{consignes:?}"
    );
    let s = strings(app.lang);
    let painted = paint(&mut app);
    assert!(
        painted.contains(s.exercise_todo),
        "la bande annonce ce qu'il reste à écrire : {painted}"
    );

    // ET ELLE S'EFFACE QUAND LE TRAVAIL EST FAIT : c'est ce qui en fait
    // un retour vivant plutôt qu'un rappel de l'énoncé.
    let fini = builtin_pack()
        .by_id("boucle")
        .and_then(|e| e.solution.clone())
        .expect("une solution");
    app.set_source(fini);
    assert!(app.consignes_du_trou().is_empty());
    let painted = paint(&mut app);
    assert!(
        !painted.contains(s.exercise_todo),
        "plus rien à écrire, plus de mention : {painted}"
    );
}

/// La solution se demande, et la regarder n'efface rien.
///
/// Chercher sans jamais pouvoir regarder la réponse n'apprend rien à qui
/// est bloqué. Elle s'ouvre à côté — le programme qu'on a écrit reste
/// dans l'éditeur, et aucun score n'est retiré.
#[test]
fn la_solution_se_demande_et_n_efface_rien() {
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/boucle");
    let s = strings(app.lang);

    // Le bouton est dans la bande de l'Établi.
    let painted = paint(&mut app);
    assert!(
        painted.contains(s.exercise_solution),
        "la bande porte le bouton : {painted}"
    );

    // Ce qu'on a écrit de son côté, avant de demander.
    app.set_source("mon travail".to_owned());
    app.show_solution = true;
    let painted = paint(&mut app);
    assert!(
        painted.contains(s.exercise_solution_title),
        "la fenêtre s'ouvre : {painted}"
    );
    assert!(
        painted.contains("dec rcx"),
        "et montre le programme entier : {painted}"
    );
    assert_eq!(app.source, "mon travail", "sans toucher à l'éditeur");

    // Une démonstration n'en a pas : le bouton ne paraît pas.
    app.show_solution = false;
    app.load_exercise_now("namsdec/registres");
    assert!(app.exercise_solution().is_none());
    let painted = paint(&mut app);
    assert!(
        !painted.contains(s.exercise_solution),
        "rien à écrire, rien à montrer : {painted}"
    );
}

/// Le but se lit en toutes lettres au mode débutant, et en syntaxe de
/// point d'arrêt au mode avancé.
///
/// Relevé à l'écran le 11 novembre 2026 : la bande du bas disait
/// « RCX 3 → 2 » pendant que le but, trois centimètres plus haut,
/// disait « rax == 0xf ». Deux façons d'écrire la même chose sur un même
/// écran — et c'est le mode qui se veut le plus clair qui portait la
/// plus obscure.
#[test]
fn le_but_se_lit_en_toutes_lettres_au_mode_debutant() {
    // La réécriture, seule : majuscules, opérateur en mots, décimal.
    assert_eq!(
        condition_lisible("rax == 0xf", true, Lang::Fr),
        "RAX vaut 15"
    );
    assert_eq!(
        condition_lisible("rbx != 0x10", true, Lang::Fr),
        "RBX n'est pas 16"
    );
    assert_eq!(condition_lisible("rax == 0xf", true, Lang::En), "RAX is 15");
    // Au mode avancé, la condition reste celle qu'on taperait dans un
    // point d'arrêt conditionnel.
    assert_eq!(
        condition_lisible("rax == 0xf", false, Lang::Fr),
        "rax == 0xf"
    );
    // Une expression ne se traduit pas : la calculer ferait dire au but
    // autre chose que ce que son auteur a écrit.
    assert_eq!(
        condition_lisible("[rsp] == rbx + 1", true, Lang::Fr),
        "[RSP] vaut rbx + 1"
    );
    // Et ce qui ne se lit pas est laissé tel quel, plutôt que perdu.
    assert_eq!(condition_lisible("", true, Lang::Fr), "");

    // À l'écran : la cible de l'étape, avant même d'avoir rien lancé.
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/boucle");
    let painted = paint(&mut app);
    assert!(
        painted.contains("RAX vaut 15"),
        "le but doit se lire en clair : {painted}"
    );
    assert!(
        !painted.contains("rax == 0xf"),
        "et pas dans les deux écritures à la fois : {painted}"
    );

    // Au mode avancé, la fenêtre des exercices montre la syntaxe.
    app.debutant = false;
    app.show_exercises = true;
    let painted = paint(&mut app);
    assert!(painted.contains("rax == 0xf"), "{painted}");
}

/// CHAQUE ÉTAPE À TROU A UN INDICE, et l'indice n'est pas la solution.
///
/// Entre la consigne et le programme entier, il n'y avait rien : on
/// relisait la consigne, on ne trouvait pas, et le seul recours mettait
/// fin à l'exercice. L'indice est le palier qui manquait.
///
/// Deux sources, et le contrôle accepte les deux : l'exercice porte son
/// indice (`hint`), ou namsdec le CALCULE — les instructions que la
/// solution emploie et que le programme livré n'a pas. Le calcul suffit
/// pour quatre-vingts étapes ; les vingt et une autres l'ont écrit.
#[test]
fn chaque_etape_a_trou_a_un_indice_qui_ne_donne_pas_la_solution() {
    let pack = builtin_pack();
    let mut écrits = 0;
    let mut calculés = 0;
    for exercice in &pack.exercises {
        let Some(solution) = exercice.solution.as_deref() else {
            // Pas de trou, pas d'indice : une démonstration ne se
            // cherche pas.
            assert!(
                exercice.hint.is_none(),
                "{} n'a rien à chercher et porte pourtant un indice",
                exercice.id
            );
            continue;
        };
        // L'app, telle qu'elle est en ouvrant l'étape.
        let mut app = app_with_exercises();
        app.load_exercise_now(&namsdec_exercises::full_id(&pack, exercice));
        let (écrit, manquantes) = app
            .exercise_hint()
            .unwrap_or_else(|| panic!("{} : aucun indice, écrit ou calculé", exercice.id));
        if écrit.is_some() {
            écrits += 1;
        }
        if !manquantes.is_empty() {
            calculés += 1;
        }

        // UN INDICE N'EST PAS UNE SOLUTION. S'il portait une ligne
        // entière du programme à écrire, le palier intermédiaire
        // n'existerait plus : il n'y aurait que deux états, bloqué et
        // fini.
        if let Some(écrit) = écrit {
            for ligne in solution.lines() {
                let ligne = ligne.split(';').next().unwrap_or("").trim();
                // Les lignes trop courtes ne prouvent rien : « ret »
                // ou « hlt » se citent sans rien révéler.
                if ligne.len() < 12 || ligne.ends_with(':') {
                    continue;
                }
                assert!(
                    !écrit.contains(ligne),
                    "l'indice de {} recopie une ligne de la solution : « {ligne} »",
                    exercice.id
                );
            }
        }
    }
    // Les deux sources servent vraiment : si l'une tombait à zéro, le
    // contrôle passerait au vert en ne regardant plus qu'une moitié.
    assert!(écrits >= 20, "seulement {écrits} indices écrits");
    assert!(calculés >= 70, "seulement {calculés} indices calculés");
}

/// L'indice se demande, s'affine à mesure qu'on écrit, et mène aux
/// fiches.
#[test]
fn l_indice_se_demande_et_suit_ce_qu_on_a_deja_ecrit() {
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/boucle");
    let s = strings(app.lang);

    // Ce que la solution emploie et que le sujet n'a pas.
    let (écrit, manquantes) = app.exercise_hint().expect("un indice");
    assert!(écrit.is_none(), "celle-ci n'en a pas besoin d'écrit");
    assert_eq!(manquantes, vec!["add", "dec", "jnz"]);

    // IL S'AFFINE : ce qu'on vient d'écrire n'est plus proposé.
    app.set_source(format!("{}\n    add rax, rcx\n", app.source));
    let (_, manquantes) = app.exercise_hint().expect("un indice");
    assert_eq!(manquantes, vec!["dec", "jnz"], "l'indice suit le tampon");

    // IL NE SE MONTRE PAS TOUT SEUL : un indice qu'on n'a pas réclamé
    // est une réponse soufflée.
    let painted = paint(&mut app);
    assert!(painted.contains(s.exercise_hint), "le bouton est là");
    assert!(
        !painted.contains(s.exercise_hint_uses),
        "mais pas son contenu : {painted}"
    );

    app.indice_montré = true;
    let painted = paint(&mut app);
    assert!(painted.contains(s.exercise_hint_uses), "{painted}");
    assert!(painted.contains("jnz"), "{painted}");

    // Et il repart à zéro à l'étape suivante : c'est l'étape qu'on
    // cherche, pas la séance.
    app.load_exercise_now("namsdec/compteur");
    assert!(!app.indice_montré);
    let (_, manquantes) = app.exercise_hint().expect("un indice");
    assert_eq!(manquantes, vec!["loop"]);
}

/// Le parcours va dans LES DEUX SENS.
///
/// Il n'allait que dans un : « Étape suivante » dans la bande et dans
/// la fenêtre, et rien pour revenir. Pour revoir l'étape qu'on venait
/// de faire — comment on s'y était pris, ou pour la refaire plus
/// court —, il fallait rouvrir la liste et y retrouver son rang.
#[test]
fn on_revient_a_l_etape_precedente() {
    let mut app = app_with_exercises();
    let pack = builtin_pack();
    let premier = namsdec_exercises::full_id(&pack, &pack.exercises[0]);
    let second = namsdec_exercises::full_id(&pack, &pack.exercises[1]);
    let troisième = namsdec_exercises::full_id(&pack, &pack.exercises[2]);

    // LA PRÉCÉDENTE TOUT COURT, et non « la précédente non réussie » :
    // on recule vers ce qu'on vient de faire, et sauter les étapes
    // réussies ferait manquer celle qu'on cherche.
    app.exercises_done.insert(second.clone());
    assert_eq!(
        app.exercise_before(&troisième).map(|(id, _)| id),
        Some(second.clone()),
        "même réussie, l'étape d'avant est l'étape d'avant"
    );
    // Le titre vient avec : c'est ce que le survol montre.
    let (_, titre) = app.exercise_before(&second).expect("la première existe");
    assert_eq!(titre, pack.exercises[0].title.get(app.lang.code()));
    // Et le premier n'a rien avant lui : un parcours a un début.
    assert_eq!(app.exercise_before(&premier), None);

    // À L'ÉCRAN, aux deux endroits. Dans la bande de l'Établi, en
    // flèche — la place y est comptée ; dans la fenêtre, en toutes
    // lettres.
    app.debutant = true;
    app.load_exercise_now(&troisième);
    let painted = paint(&mut app);
    assert!(
        painted.contains('‹'),
        "la bande porte le retour : {painted}"
    );

    app.debutant = false;
    app.show_exercises = true;
    let s = strings(app.lang);
    let painted = paint(&mut app);
    assert!(
        painted.contains(s.exercises_previous),
        "la fenêtre aussi : {painted}"
    );
}

/// L'ÉTABLI DIT QUAND LA LISTE DES PROBLÈMES NE TIENT PAS DANS SA
/// BANDE, ET IL MONTRE LA CORRECTION QU'IL SAIT PROPOSER.
///
/// Deux défauts vus sur la même capture, `établi / l'assemblage a
/// échoué` (16 novembre 2026) : la dernière explication était tranchée
/// par le milieu au bas de la bande — une ligne coupée ne ressemble pas
/// à une liste qui continue, mais à un texte perdu —, et la correction
/// que `fixes` sait écrire depuis longtemps n'avait aucune porte au
/// mode simple, qui n'a pas la barre d'état où Alt+Entrée s'annonce.
#[test]
fn la_bande_des_problemes_dit_la_suite_et_propose_la_correction() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        // Deux fautes, et deux explications qui débordent.
        source: "\
section .text
global _start
_start:
    mvo rax, rbx
    mov rzx, 1
    bougetoi rax, rbx
    int 0x80
    mov rdi, 0
    syscall
"
        .to_owned(),
        debutant: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    assert!(
        !app.diagnostics.is_empty(),
        "nasm refuse : {:?}",
        app.status
    );

    let s = strings(app.lang);
    let mut peints = String::new();
    // Quatre images : la mention paraît à partir de la seconde, le
    // débordement n'étant connu qu'une fois la zone posée.
    // AU PLUS PETIT ÉCRAN QUE L'APPLICATION ACCEPTE : c'est là que la
    // bande manque de place, et un contrôle ne voit un défaut de place
    // que si la place manque vraiment (A 9.85).
    let petit = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 520.0),
        )),
        ..écran_du_banc()
    };
    for _ in 0..4 {
        peints.clear();
        let mut sortie = ctx.run_ui(petit.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                peints.push_str(t.galley.text());
                peints.push('|');
            }
        }
        sortie.textures_delta.clear();
    }

    assert!(
        peints.contains(s.etabli_problemes_suite),
        "ce qui est sous le pli se dit :\n{peints:.1200}"
    );
    // IL N'Y A PAS DE REGISTRE `rzx`, et nasm ne le dira jamais : il ne
    // voit qu'un symbole inconnu, et répond « invalid combination of
    // opcode and operands ».
    assert!(
        peints.contains("rzx"),
        "le registre inventé est nommé :\n{peints:.1200}"
    );
    // ET LA CORRECTION DU MNÉMONIQUE SE POSE D'UN CLIC, là où elle ne
    // s'annonçait que dans une barre d'état que ce mode n'a pas.
    assert!(
        peints.contains(s.fix_apply),
        "la correction se propose sur place :\n{peints:.1200}"
    );

    // Et le bouton fait ce qu'Alt+Entrée fait.
    let ligne = app
        .diagnostics
        .iter()
        .find(|d| d.line > 0)
        .map(|d| d.line as usize - 1)
        .expect("une faute située");
    let avant = app.source.clone();
    let correction = fixes::propose(&app.source, ligne, s).expect("une correction");
    app.poser_la_ligne(ligne, &correction.ligne);
    assert_ne!(app.source, avant, "la ligne est réécrite");
    assert!(
        app.source.lines().nth(ligne) == Some(correction.ligne.as_str()),
        "et c'est bien celle qu'on proposait"
    );
}

/// LE PARCOURS A UNE PORTE QUI S'OUVRE.
///
/// Sur la capture du premier écran, « Exercices… » était GRISÉ, et le
/// seul endroit qui disait où l'allumer était un commentaire dans le
/// programme de bienvenue (« Préférences > Modules »). C'est le seul
/// bouton de la barre qui mène au parcours, et il était éteint au
/// moment où il sert le plus.
#[test]
fn le_bouton_du_parcours_allume_le_module_qu_il_demande() {
    let mut app = NamsdecApp {
        debutant: true,
        exercises: false,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);

    // Il se peint, et il n'est pas éteint.
    let s = strings(app.lang);
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.exercises_open),
        "le bouton est là :\n{peint:.300}"
    );

    // Le clic — simulé en appelant ce que le bouton fait — allume le
    // module et ouvre la fenêtre, au lieu de renvoyer vers une
    // préférence qu'on n'a pas encore ouverte.
    app.exercises = true;
    app.show_exercises = true;
    let peint = paint(&mut app);
    assert!(
        peint.contains("117") || peint.contains("Découverte"),
        "et la fenêtre du parcours s'ouvre :\n{peint:.600}"
    );
    // Le choix se retient : c'est un réglage comme un autre.
    assert!(
        serde_json::to_string(&app.settings_snapshot())
            .unwrap()
            .contains("\"exercises\":true")
    );
}

/// ON NE DEMANDE JAMAIS D'ÉCRIRE UNE NOTION QU'ON N'A NI VUE NI
/// ANNONCÉE.
///
/// **C'est la règle de progression du parcours**, et elle n'était
/// écrite nulle part : chaque étape pouvait, en toute discrétion,
/// demander une instruction que les cinquante précédentes n'avaient
/// jamais montrée et que rien dans son énoncé ne nommait. L'élève n'a
/// alors que deux issues — la solution, qui met fin à l'exercice, ou
/// l'abandon.
///
/// **Trois façons d'être en règle**, et c'est volontairement large :
///
/// - la notion a déjà servi dans une étape précédente, ou elle est
///   écrite dans le programme de départ qu'on a sous les yeux ;
/// - l'étape la NOMME dans son énoncé, ses commentaires ou son indice
///   écrit — c'est ainsi qu'on introduit quelque chose ;
/// - elle nomme sa FAMILLE : « `setcc` recopie l'indicateur » introduit
///   `setz`, `sets`, `setc` et `seto` d'un coup, et c'est exactement ce
///   que fait l'étape 8. La convention est celle d'Intel, et
///   l'application l'emploie déjà.
///
/// **Mesuré le 16 novembre 2026 sur les cent dix-sept étapes** : vingt
/// introduisent une notion dans leur trou, dix-huit la nommaient, une
/// la nommait par sa famille, et UNE SEULE ne la nommait pas — `rep`,
/// à l'étape `memmove`. Elle a reçu l'indice qui lui manquait.
#[test]
fn aucune_etape_ne_demande_une_notion_jamais_vue_ni_annoncee() {
    let pack = builtin_pack();
    let mut vues: Vec<&'static str> = Vec::new();
    let mut fautes: Vec<String> = Vec::new();

    for (rang, exercise) in pack.exercises.iter().enumerate() {
        let sujet = crate::exercises::instructions_du_programme(&exercise.starter_code);
        let Some(solution) = exercise.solution.as_deref() else {
            // Une démonstration ou une étape de lecture ne demande rien
            // à écrire : tout ce qu'elle montre est sous les yeux.
            for fiche in crate::exercises::instructions_du_programme(&exercise.starter_code) {
                if !vues.contains(&fiche) {
                    vues.push(fiche);
                }
            }
            continue;
        };
        // CE QUE LE TROU DEMANDE : dans la solution, absent du sujet.
        for fiche in crate::exercises::instructions_du_programme(solution) {
            let à_écrire = !sujet.contains(&fiche);
            if !à_écrire || vues.contains(&fiche) {
                continue;
            }
            // Tout ce que l'étape donne à lire, dans les deux langues :
            // un parcours suivi en anglais doit l'être aussi.
            let mut lu = String::new();
            for langue in ["fr", "en"] {
                lu.push_str(exercise.description.get(langue));
                lu.push(' ');
                if let Some(indice) = &exercise.hint {
                    lu.push_str(indice.get(langue));
                    lu.push(' ');
                }
            }
            lu.push_str(&exercise.starter_code);
            let lu = lu.to_ascii_lowercase();
            // La fiche entière : « je » s'annonce aussi bien par `jz`,
            // puisque ce sont deux noms du même opcode.
            let noms = namsdec_doc::lookup(fiche)
                .map(|e| e.names)
                .unwrap_or(&[] as &[&str]);
            let nommée = noms.iter().any(|nom| mot_présent(&lu, nom));
            // Ou sa famille : setcc, jcc, cmovcc.
            let famille = ["set", "cmov", "j"].iter().find_map(|préfixe| {
                fiche
                    .strip_prefix(préfixe)
                    .map(|_| format!("{préfixe}cc"))
                    .filter(|f| lu.contains(f.as_str()))
            });
            if !nommée && famille.is_none() {
                fautes.push(format!(
                    "étape {} ({}) : il faut écrire « {fiche} », jamais rencontrée et nommée nulle part",
                    rang + 1,
                    exercise.id
                ));
            }
        }
        for fiche in crate::exercises::instructions_du_programme(solution) {
            if !vues.contains(&fiche) {
                vues.push(fiche);
            }
        }
    }

    assert!(
        fautes.is_empty(),
        "le parcours demande {} notion(s) qu'il n'a ni montrées ni annoncées :\n  {}",
        fautes.len(),
        fautes.join("\n  ")
    );
}

/// Un mot entier, et non un morceau : `add` ne doit pas se reconnaître
/// dans « adresse », ni `or` dans « ordre ».
fn mot_présent(texte: &str, mot: &str) -> bool {
    let borne = |c: char| !c.is_ascii_alphanumeric() && c != '_';
    texte.match_indices(mot).any(|(i, _)| {
        let avant = texte[..i].chars().next_back().is_none_or(borne);
        let après = texte[i + mot.len()..].chars().next().is_none_or(borne);
        avant && après
    })
}

/// « EST-CE JUSTE ? » SE DEMANDE EN UN GESTE.
///
/// Il en fallait trois avant, dans le bon ordre, et il fallait savoir
/// qu'il les fallait : assembler, exécuter, puis remarquer qu'une
/// pastille a changé de couleur. Un parcours dont chaque vérification
/// demande de connaître la chaîne de compilation n'est pas interactif.
#[test]
fn verifier_assemble_execute_et_rend_un_verdict() {
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/echange");

    // Rien n'a encore été demandé : pas de verdict.
    assert_eq!(app.verdict_de_la_verification(), None);
    assert!(app.exercice_a_verifier(), "cette étape a un but");

    // LE PROGRAMME LIVRÉ A UN TROU : vérifier doit donc répondre
    // « pas encore », et ne rien noter comme réussi.
    app.verifier_l_exercice();
    // Les deux battements d'une image : nasm rend son résultat, puis la
    // course avance. C'est ce que `draw` fait soixante fois par
    // seconde, et le geste n'attend rien en bloquant la fenêtre.
    for _ in 0..8000 {
        app.tick_assemble();
        app.tick_run(&egui::Context::default());
        // Le fil de nasm a besoin de quelques millisecondes ; une image
        // en dure seize. Sans cette pause, huit mille tours de boucle
        // passent en moins d'une milliseconde et l'on conclurait que
        // l'assemblage ne rend jamais rien.
        std::thread::sleep(std::time::Duration::from_millis(1));
        if !app.running && !app.assembling() && app.emulator.is_some() {
            break;
        }
    }
    assert!(
        app.emulator.is_some(),
        "l'assemblage a eu lieu : {:?}",
        app.status
    );
    assert_eq!(
        app.verdict_de_la_verification(),
        Some(false),
        "le trou n'est pas rempli — {:?}",
        app.exercise_goal()
    );
    assert!(!app.exercises_done.contains("namsdec/echange"));

    // LA SOLUTION, ET LE MÊME GESTE : le verdict bascule.
    charge_la_solution(&mut app, "namsdec/echange");
    app.verifier_l_exercice();
    for _ in 0..8000 {
        app.tick_assemble();
        app.tick_run(&egui::Context::default());
        // Le fil de nasm a besoin de quelques millisecondes ; une image
        // en dure seize. Sans cette pause, huit mille tours de boucle
        // passent en moins d'une milliseconde et l'on conclurait que
        // l'assemblage ne rend jamais rien.
        std::thread::sleep(std::time::Duration::from_millis(1));
        if !app.running && !app.assembling() && app.verdict_de_la_verification().is_some() {
            break;
        }
    }
    assert_eq!(
        app.verdict_de_la_verification(),
        Some(true),
        "la solution atteint le but — {:?}",
        app.exercise_goal()
    );
    // Et le carnet de bord l'inscrit : l'application appelle cette
    // méthode à chaque image, c'est elle qui fait passer la pastille au
    // vert — et c'est la MÊME règle que le verdict, depuis qu'ils
    // partagent `exercice_reussi`.
    app.note_goal_reached();
    assert!(app.exercises_done.contains("namsdec/echange"));

    // ET IL NE SUIT PAS D'UNE ÉTAPE À L'AUTRE : rendu sur la suivante,
    // il dirait « Réussi » devant un programme qu'on n'a pas écrit.
    app.load_exercise_now("namsdec/zero");
    assert_eq!(app.verdict_de_la_verification(), None);
}

/// UNE ÉTAPE DE LECTURE SE RÉPOND AVANT DE S'EXÉCUTER.
///
/// C'est toute sa leçon — « lisez-le, répondez, puis exécutez : la
/// machine tranchera ». Vérifier sans avoir répondu ferait tourner le
/// programme, et le programme DONNE la réponse : RAX vaut 55 dans le
/// volet avant qu'on ait eu l'occasion de se tromper. C'est la règle
/// de `prevision`, au seul endroit où elle manquait.
#[test]
fn une_question_ne_se_verifie_pas_avant_d_avoir_repondu() {
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/prevoir_somme");
    assert!(
        app.exercise_question().is_some(),
        "cette étape pose une question"
    );
    assert!(app.exercice_a_verifier(), "il y a quelque chose à vérifier");
    assert!(
        !app.verification_possible(),
        "mais pas tant que le champ est vide"
    );

    app.question_answer = "55".to_owned();
    assert!(
        app.verification_possible(),
        "une réponse écrite ouvre la vérification"
    );

    // UN EXERCICE D'ÉCRITURE, LUI, SE VÉRIFIE TOUT DE SUITE : il n'y a
    // pas de réponse à souffler, et ce qu'on a écrit EST la réponse.
    app.load_exercise_now("namsdec/echange");
    assert!(app.verification_possible());

    // ET UNE DÉMONSTRATION N'A RIEN À VÉRIFIER : le bouton dirait
    // « vérifiez » devant un programme qu'on regarde tourner.
    app.load_exercise_now("namsdec/registres");
    assert!(app.exercise_is_demo());
    assert!(!app.exercice_a_verifier());
}

/// LE CARNET NE COMPTE QUE CE QU'ON A RÉUSSI.
///
/// « 12/117 » dit le chemin parcouru et rien de ce qu'on y a appris.
/// Le carnet répond à l'autre question — qu'est-ce que je sais
/// maintenant — et il ne la flatte pas : ouvrir un énoncé n'apprend
/// rien, et deux noms du même opcode ne font pas deux notions.
#[test]
fn le_carnet_des_notions_suit_les_etapes_reussies() {
    let mut app = app_with_exercises();
    assert!(
        app.notions_vues().is_empty(),
        "rien n'est acquis avant d'avoir rien fait"
    );

    // Ouvrir une étape ne suffit pas.
    app.load_exercise_now("namsdec/echange");
    assert!(app.notions_vues().is_empty(), "ouvrir n'est pas réussir");

    // La réussir, oui — et le carnet porte alors ce que la solution
    // emploie : `mov`, `xchg`, `hlt` et les directives qui ne sont pas
    // des instructions n'y sont pas.
    app.exercises_done.insert("namsdec/echange".to_owned());
    let notions = app.notions_vues();
    assert!(notions.contains(&"mov"), "{notions:?}");
    assert!(notions.contains(&"xchg"), "{notions:?}");
    assert!(
        !notions.contains(&"global") && !notions.contains(&"section"),
        "une directive n'est pas une instruction : {notions:?}"
    );

    // TRIÉ ET SANS DOUBLON, d'une étape à l'autre : deux étapes qui
    // emploient `mov` ne l'inscrivent qu'une fois.
    app.exercises_done.insert("namsdec/zero".to_owned());
    let après = app.notions_vues();
    assert_eq!(
        après.iter().filter(|n| **n == "mov").count(),
        1,
        "{après:?}"
    );
    let mut trié = après.clone();
    trié.sort_unstable();
    assert_eq!(après, trié, "le carnet se lit dans l'ordre");
    assert!(après.len() > notions.len(), "il grandit : {après:?}");
}

/// L'INDICE SE CACHE COMME IL SE MONTRE.
///
/// « Quand on clique sur indice plus moyen de le cacher » (16 novembre
/// 2026). Le bouton disparaissait une fois l'indice demandé — or on
/// cache un indice pour la même raison qu'on le demande : retrouver la
/// place et l'envie de chercher soi-même. C'est la famille des trois
/// gestes qui ne se défaisaient pas (A 9.89).
#[test]
fn l_indice_se_cache_comme_il_se_montre() {
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/echange");
    let s = strings(app.lang);

    let peint = paint(&mut app);
    assert!(
        peint.contains(s.exercise_hint),
        "le bouton propose l'indice"
    );
    assert!(
        !peint.contains(s.exercise_hint_hide),
        "et ne propose pas encore de le cacher"
    );

    app.indice_montré = true;
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.exercise_hint_hide),
        "l'indice montré, le bouton propose de le cacher :\n{peint:.400}"
    );
    assert!(
        !peint.contains(&format!("|{}|", s.exercise_hint)),
        "et ne propose plus de le demander deux fois"
    );
}

/// RECOMMENCER REND L'ÉTAPE TELLE QU'ON L'A TROUVÉE — et ne retire pas
/// ce qu'on a compris.
#[test]
fn recommencer_rend_le_programme_de_depart() {
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/echange");
    let départ = app.source.clone();

    // On écrit, on assemble, on réussit.
    charge_la_solution(&mut app, "namsdec/echange");
    assert_ne!(app.source, départ, "la solution n'est pas le départ");
    app.exercises_done.insert("namsdec/echange".to_owned());
    app.exercises_best.insert("namsdec/echange".to_owned(), 4);
    app.indice_montré = true;
    app.do_assemble();

    app.recommencer_l_exercice();
    assert_eq!(app.source, départ, "le programme de départ est rendu");
    assert!(!app.indice_montré, "l'indice se referme");
    assert!(app.emulator.is_none(), "et la machine repart de rien");
    assert!(
        !app.exercise_drafts.contains_key("namsdec/echange"),
        "le brouillon s'en va avec : sans cela, fermer l'onglet puis \
         revenir rendrait le travail qu'on vient de jeter"
    );
    // CE QU'ON A COMPRIS RESTE : recommencer pour faire plus court est
    // précisément ce que le budget encourage.
    assert!(app.exercises_done.contains("namsdec/echange"));
    assert_eq!(app.exercises_best.get("namsdec/echange"), Some(&4));
}

/// TOUT REMETTRE À ZÉRO EFFACE CE QUE LE PARCOURS A RETENU, ET RIEN
/// D'AUTRE.
#[test]
fn remettre_le_parcours_a_zero_efface_ce_qu_il_retenait() {
    let mut app = app_with_exercises();
    app.exercises_done.insert("namsdec/echange".to_owned());
    app.exercises_done.insert("namsdec/zero".to_owned());
    app.exercises_best.insert("namsdec/echange".to_owned(), 4);
    app.exercise_drafts
        .insert("namsdec/zero".to_owned(), "mov rax, 1\n".to_owned());
    app.source = "mon programme à moi\n".to_owned();

    app.reinitialiser_le_parcours();

    assert!(app.exercises_done.is_empty(), "les réussites s'effacent");
    assert!(app.exercises_best.is_empty(), "les scores aussi");
    assert!(app.exercise_drafts.is_empty(), "et les brouillons");
    assert_eq!(
        app.source, "mon programme à moi\n",
        "mais pas ce qu'on a sous les yeux"
    );
    assert!(app.notions_vues().is_empty(), "le carnet repart de rien");
}

/// L'ÉNONCÉ MÈNE AUX FICHES : une instruction qu'il nomme s'ouvre d'un
/// clic.
///
/// « Dans l'énoncé, une instruction ne dit pas ce qu'elle fait »
/// (16 novembre 2026). Les cent quatre-vingt-neuf fiches existaient, et
/// l'indice y menait depuis le 12 novembre — mais l'énoncé, qu'on lit
/// AVANT l'indice, n'y menait pas.
#[test]
fn l_enonce_mene_aux_fiches() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise_now("namsdec/echange");
    // L'énoncé de cette étape nomme `xchg` et `xor` entre accents
    // graves : ce sont eux que la bande peint en violet.
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1600.0, 1000.0))
        .build_ui_state(|ui, app: &mut NamsdecApp| app.draw(ui), app);
    harness.run();
    harness.get_by_label("xchg").click();
    harness.run();

    let app = harness.state();
    assert!(app.show_help, "la fenêtre d'aide s'ouvre");
    assert_eq!(
        app.help.as_ref().map(|(mot, _)| mot.as_str()),
        Some("xchg"),
        "sur la fiche de l'instruction qu'on a cliquée"
    );
}

/// La théorie s'offre, et elle se lit.
///
/// Trois choses, et il faut les trois : le geste existe quand il y a
/// une théorie, il ne paraît pas quand il n'y en a pas, et ce qu'il
/// ouvre est bien peint. Un contrôle qui ne vérifierait que le bouton
/// passerait au vert le jour où le repli s'ouvre sur rien.
///
/// **Le bouton est dans la rangée des gestes, et non dans la bande qui
/// défile** : c'est une mesure qui l'a décidé. Posé sous l'énoncé, il
/// coûtait deux pixels à la dernière ligne de la bande, et
/// `la_bande_de_l_exercice_ne_coupe_pas_sa_derniere_ligne` les a
/// comptés. Ce contrôle-ci vérifie donc qu'il est bien là où il ne
/// coûte rien, en le cherchant sur une scène où la bande est pleine.
#[test]
fn la_theorie_s_offre_et_se_lit() {
    let s = strings(Lang::Fr);
    let mut app = app_with_exercises();
    app.debutant = true;
    app.load_exercise("namsdec/prevoir_somme");
    app.do_assemble();

    // 1. REPLIÉE : le geste est offert, le texte n'est pas là.
    app.theorie_ouverte = false;
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.etabli_exercise_theory),
        "le geste « {} » doit être offert : {peint}",
        s.etabli_exercise_theory
    );

    // Ce que la théorie dit, pris dans la série plutôt qu'écrit ici :
    // un contrôle qui recopierait le texte passerait au vert le jour
    // où la série en change.
    let théorie = namsdec_exercises::find(&app.exercise_packs, "namsdec/prevoir_somme")
        .and_then(|ex| ex.theorie.clone())
        .expect("cet exercice porte une théorie");
    let début: String = théorie.fr.split(':').next().unwrap_or_default().to_owned();
    assert!(début.len() > 20, "de quoi reconnaître le texte : {début:?}");
    assert!(
        !peint.contains(début.trim()),
        "repliée, la théorie ne doit pas être peinte : {peint}"
    );

    // 2. DÉPLIÉE : le texte est là, et le geste dit comment la refermer.
    app.theorie_ouverte = true;
    let peint = paint(&mut app);
    assert!(
        peint.contains(début.trim()),
        "dépliée, la théorie doit être peinte — « {début} » : {peint}"
    );
    assert!(
        peint.contains(s.etabli_exercise_theory_hide),
        "et le geste doit dire comment la refermer : {peint}"
    );

    // 3. SANS THÉORIE, PAS DE GESTE. Une porte qui n'ouvre sur rien
    //    est pire qu'une absence de porte (A 9.89).
    let mut nue = app_with_exercises();
    nue.debutant = true;
    nue.load_exercise("namsdec/prevoir_somme");
    nue.do_assemble();
    nue.theorie_ouverte = false;
    for pack in &mut nue.exercise_packs {
        for exercice in &mut pack.exercises {
            exercice.theorie = None;
        }
    }
    let peint = paint(&mut nue);
    assert!(
        !peint.contains(s.etabli_exercise_theory),
        "sans théorie, le geste ne doit pas être offert : {peint}"
    );
}
