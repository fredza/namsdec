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
    assert!(!settings.exercises, "et le module reste éteint");
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
        palette_commands(s, false, exercises)
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
        app.source.contains("add rax, rcx"),
        "et celui d'origine aussi"
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
    let code = |id: &str| pack.by_id(id).expect("l'étape existe").starter_code.clone();

    // L'ordre des octets : le premier en mémoire est celui de poids
    // faible, le dernier celui de poids fort.
    let app = exécute(&code("boutisme"));
    let regs = &app.emulator.as_ref().expect("chargé").cpu().regs;
    assert_eq!(regs.get64(Reg64::Rax), 0x1122334455667788);
    assert_eq!(regs.get64(Reg64::Rcx), 0x88);
    assert_eq!(regs.get64(Reg64::Rdx), 0x11);

    // Signé ou non : sur la même comparaison, `ja` saute et `jg` non.
    let app = exécute(&code("signe"));
    let regs = &app.emulator.as_ref().expect("chargé").cpu().regs;
    assert_eq!(
        regs.get64(Reg64::Rcx),
        1,
        "non signé : -1 est le plus grand de tous"
    );
    assert_eq!(regs.get64(Reg64::Rdx), 0, "signé : -1 est plus petit que 1");

    // La division part sur un refus — c'est l'énoncé —, et la
    // correction que l'énoncé demande donne le quotient et le reste.
    let app = exécute(&code("division"));
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
    let corrigé = code("division").replace("mov rdx, 99", "xor rdx, rdx");
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
            source: exercise.starter_code.clone(),
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
    assert!(painted.contains("cent huit exercices"), "{painted}");
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
    // vingt-quatrième des cent huit étapes.
    assert!(painted.contains("30/108"), "{painted}");
}

/// Un parcours se suit : l'étape suivante est à un clic, sans avoir à
/// retrouver son rang dans une liste de cent huit.
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
        painted.contains("1/108"),
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

/// Chaque but annoncé se lit, et le programme livré est du bon côté :
/// une démonstration l'atteint d'emblée, un travail ne l'atteint pas.
///
/// Un but qu'aucune exécution ne satisfait ferait mentir le verdict ; un
/// but déjà atteint dans un exercice à faire ne demanderait rien. Les
/// deux listes disent donc, noir sur blanc, ce que chaque étape attend
/// de celui qui la fait.
#[test]
fn every_goal_is_readable_and_on_the_right_side() {
    for exercise in &builtin_pack().exercises {
        for texte in &exercise.goal {
            assert!(
                condition::parse(texte).is_some(),
                "{} : « {texte} » ne se lit pas",
                exercise.id
            );
        }
    }
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Ce que le programme livré fait déjà : on le regarde tourner et on
    // comprend ce qui s'est passé.
    const DÉMONSTRATIONS: [&str; 97] = [
        "constantes",
        "rotations",
        "setcc",
        "not_neg",
        "division_signee",
        "immediats",
        "tableau_octets",
        "memmove",
        "remplir",
        "bit_scan",
        "adc_128",
        "table_conversion",
        "padding",
        "concatener",
        "premier_mot",
        "recursion_terminale",
        "tri_insertion",
        "recherche_binaire",
        "liste_chainee",
        "transposee",
        "simd_somme",
        "flottants_conversion",
        "table_ascii",
        "afficher_signe",
        "hexadecimal",
        "incrementer",
        "test_and",
        "xchg_bswap",
        "masques",
        "min_max",
        "pair_impair",
        "occurrences",
        "inverser_tableau",
        "borne",
        "puissances",
        "somme_moyenne",
        "longueur_chaine",
        "inverser_chaine",
        "compter_mots",
        "pile_inverse",
        "args_pile",
        "pointeur_fonction",
        "alignement",
        "hexa_affichage",
        "xor_chiffrement",
        "somme_controle",
        "canari",
        "flottants_masque",
        "boucle",
        "lire_nombre",
        "structure",
        "tri",
        "bits",
        "memoire",
        "tableau",
        "chaine",
        "appels",
        "multiplier",
        "boutisme",
        "signe",
        "chaines",
        "cadre",
        "tas",
        "flottants",
        "afficher",
        "donnees",
        "adresses",
        "macros",
        "recursion",
        "echange",
        "decalages",
        "branchements",
        "simd",
        "zero",
        "compteur",
        "tailles",
        "recherche",
        "comparer_chaines",
        "drapeaux",
        "negatif",
        "compter_bits",
        "matrice",
        "sans_branche",
        "fibonacci",
        "pile_locale",
        "majuscules",
        // La rétro-ingénierie : on regarde d'abord, et ce qu'on
        // regarde tourne déjà. Seul `re_derouler` demande un geste.
        "re_entete",
        "re_chaines",
        "re_position_adresse",
        "re_comparaison",
        "re_table_sauts",
        "re_chaines_cachees",
        "re_antidebogage",
        "re_int3",
        "re_entropie",
        "re_gadget",
        "re_un_octet",
    ];
    // Ce qu'il reste à faire : le but n'est pas atteint tant qu'on n'a
    // pas écrit ce qui manque.
    const À_FAIRE: [&str; 4] = ["pile", "division", "convention", "re_derouler"];

    for (id, attendu) in DÉMONSTRATIONS
        .iter()
        .map(|id| (*id, true))
        .chain(À_FAIRE.iter().map(|id| (*id, false)))
    {
        let mut app = NamsdecApp {
            exercises: true,
            exercise_packs: vec![builtin_pack()],
            ..Default::default()
        };
        app.load_exercise(&format!("namsdec/{id}"));
        app.do_assemble();
        app.do_run();
        for _ in 0..200 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
        let (conditions, atteint) = app
            .exercise_goal()
            .unwrap_or_else(|| panic!("{id} : aucun but"));
        assert_eq!(
            atteint,
            attendu,
            "{id} : {:?} — attendu {}",
            conditions
                .iter()
                .map(|l| l.texte.as_str())
                .collect::<Vec<_>>(),
            if attendu {
                "atteint d'emblée"
            } else {
                "à atteindre soi-même"
            }
        );
    }

    // Et chaque exercice du parcours est dans l'une des deux listes, ou
    // n'a pas de but : personne n'est oublié en chemin.
    for exercise in &builtin_pack().exercises {
        let classé = DÉMONSTRATIONS.contains(&exercise.id.as_str())
            || À_FAIRE.contains(&exercise.id.as_str());
        assert_eq!(
            classé,
            !exercise.goal.is_empty(),
            "{} : un but non classé, ou un classement sans but",
            exercise.id
        );
    }
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

    app.load_exercise("namsdec/boucle");
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
        painted.contains(&format!("1/108 {}", strings(app.lang).exercises_done_count)),
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
        app.load_exercise(&format!("namsdec/{id}"));
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
        app.load_exercise(&format!("namsdec/{id}"));
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
    // Les deux exercices à faire : le budget vise la solution que
    // l'énoncé demande, et la voici.
    let corrections: [(&str, &str, &str); 4] = [
        // La valeur que l'énoncé demande de trouver :
        // ((0x1234 ^ 0x5a5a) + 7) * 3.
        ("re_derouler", "mov rax, 0", "mov rax, 55647"),
        ("division", "mov rdx, 99", "xor rdx, rdx"),
        ("pile", "    hlt", "    xchg rax, rbx\n    hlt"),
        // La fonction honnête : elle met RBX de côté et le rend.
        (
            "convention",
            "triche:\n    mov rbx, 99",
            "triche:\n    push rbx\n    mov rbx, 99",
        ),
    ];

    // Tous les exercices, et non les seuls budgétés : un exercice qui
    // n'atteint pas son propre but est un exercice faux, budget ou non.
    for exercise in &builtin_pack().exercises {
        let budget = exercise.par.unwrap_or(u64::MAX);
        let mut source = exercise.starter_code.clone();
        for (id, avant, après) in corrections {
            if exercise.id == id {
                source = source.replace(avant, après);
            }
        }
        // La deuxième moitié de la correction de « convention » : le
        // `pop` qui rend ce que le `push` a mis de côté.
        if exercise.id == "convention" {
            source = source.replace(
                "    add rax, rbx\n    ret",
                "    add rax, rbx\n    pop rbx\n    ret",
            );
        }
        let mut app = NamsdecApp {
            source,
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
    app.load_exercise("namsdec/boucle");
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
    app.source = builtin_pack().by_id("boucle").unwrap().starter_code.clone();
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
    app.load_exercise("namsdec/boucle");
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
    // « Mettre à zéro » : son but est vrai dès le départ, registres
    // neufs.
    app.load_exercise("namsdec/zero");
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
    assert_eq!(app.exercises_best.get("namsdec/zero").copied(), Some(4));

    // Et remettre à zéro ne vient pas écraser ce score par un zéro.
    app.dispatch(Action::Reset);
    paint(&mut app);
    assert_eq!(app.exercises_best.get("namsdec/zero").copied(), Some(4));

    // Un programme arrêté en cours de route ne compte pas non plus : le
    // but est peut-être atteint, mais le programme n'est pas fini.
    app.dispatch(Action::Reset);
    app.do_step();
    paint(&mut app);
    assert_eq!(app.exercises_best.get("namsdec/zero").copied(), Some(4));
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
    assert!(
        peint.contains(&format!("rax {} 0xc", s.goal_is)),
        "et ce que rax vaut à la place :\n{peint}"
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
/// Sept des cent huit sont des programmes à observer, et le
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
