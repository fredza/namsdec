//! Les tests de l'interface.
//!
//! Ils dessinent l'application dans un contexte egui sans fenêtre et
//! regardent ce qui est *peint* : la largeur qu'un panneau prend, les
//! textes posés, les cadres des boutons. C'est ce qui permet de vérifier
//! une mise en page sans capture d'écran et sans clic simulé.
//!
//! Sortis de `main.rs`, qui portait autant de tests que de code.

use super::*;
// Le panneau latéral a ses propres fonctions : les tests les
// nomment sans préfixe, comme avant leur déménagement.
use crate::sidebar::*;

/// Le programme type qu'on écrit en apprenant l'assembleur sous
/// Linux : deux sections, un `_start`, des appels système. Il doit
/// s'assembler, démarrer à `_start` — et non au premier octet chargé,
/// qui appartient à `.data` — relier chaque ligne à la bonne adresse,
/// écrire son message et se terminer sur son propre code de sortie.
#[test]
fn a_two_section_linux_program_prints_its_message_and_exits() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }

    let source = "\
section .data
message db \"Je parle maintenant au processeur !\", 10
longueur equ $ - message

section .text
global _start
_start:
mov rax, 1
mov rdi, 1
mov rsi, message
mov rdx, longueur
syscall

mov rax, 60
xor rdi, rdi
syscall
";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        ..Default::default()
    };

    app.do_assemble();
    assert!(
        matches!(app.status, Status::AssembleOk),
        "l'assemblage doit réussir, obtenu : {:?}",
        app.status
    );

    let line_to_addr = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .line_to_addr
        .clone();
    assert_ne!(
        line_to_addr[&2], line_to_addr[&8],
        "une ligne de `.data` et une ligne de `.text` ne peuvent pas partager une adresse"
    );
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.rip,
        line_to_addr[&8],
        "l'exécution démarre à `_start`"
    );

    // Exécution complète : le programme parle, puis sort de lui-même.
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let emulator = app.emulator.as_ref().expect("un émulateur chargé");
    assert_eq!(
        String::from_utf8_lossy(emulator.output()),
        "Je parle maintenant au processeur !\n",
        "le message doit être arrivé sur la sortie"
    );
    assert_eq!(emulator.exit_code(), Some(0));
    assert!(
        matches!(app.status, Status::Exited { code: 0 }),
        "la barre de statut doit dire que le programme s'est terminé, obtenu : {:?}",
        app.status
    );
}

/// Un programme allé jusqu'au bout n'a plus rien à exécuter : les
/// commandes d'exécution deviennent inertes — au clavier comme au
/// bouton, sans quoi le raccourci ferait ce que le bouton grisé
/// refuse — et l'éditeur revient en mode commande, comme gdb rend la
/// main après la fin du programme.
#[test]
fn a_finished_program_disables_execution_and_returns_to_command_mode() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }

    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
mov rax, 60
mov rdi, 2
syscall
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    // L'utilisateur était en train d'écrire quand il a lancé.
    app.editor_mode = EditorMode::Insert;
    assert!(app.can_execute(), "avant de lancer, tout est disponible");

    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    assert!(app.program_finished());
    assert!(
        !app.can_execute(),
        "Exécuter, Step Into et Step Over n'ont plus de sens"
    );
    assert_eq!(
        app.editor_mode,
        EditorMode::Normal,
        "la fin du programme rend la main aux commandes"
    );

    // Le raccourci doit obéir à la même règle que le bouton : RIP ne
    // bouge plus, et le statut de fin reste affiché.
    let rip = app.emulator.as_ref().unwrap().cpu().regs.rip;
    app.dispatch(Action::StepInto);
    app.dispatch(Action::StepOver);
    app.dispatch(Action::Run);
    assert_eq!(app.emulator.as_ref().unwrap().cpu().regs.rip, rip);
    assert!(
        matches!(app.status, Status::Exited { code: 2 }),
        "obtenu {:?}",
        app.status
    );

    // Réinitialiser, en revanche, doit rester possible : c'est
    // justement ce qu'on veut faire ensuite.
    app.dispatch(Action::Reset);
    assert!(app.can_execute(), "après Réinitialiser, on peut relancer");
}

/// Avancer d'un pas ne doit pas ouvrir la saisie.
///
/// `Espace s i` et `Espace n i` se terminent par `i` — la touche qui,
/// dans l'éditeur, fait passer en mode Insertion. Sans précaution, un
/// pas d'exécution laissait donc l'éditeur en Insertion, et la frappe
/// suivante s'écrivait dans le code au lieu de piloter l'exécution.
#[test]
fn stepping_with_a_leader_sequence_stays_in_normal_mode() {
    for sequence in [
        [egui::Key::Space, egui::Key::S, egui::Key::I],
        [egui::Key::Space, egui::Key::N, egui::Key::I],
    ] {
        let mut app = NamsdecApp::default();
        assert_eq!(app.editor_mode, EditorMode::Normal);

        let ctx = egui::Context::default();
        let events = sequence
            .iter()
            .map(|key| egui::Event::Key {
                key: *key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect();
        let input = egui::RawInput {
            events,
            ..Default::default()
        };

        // Une frame complète, dans l'ordre réel : le dispatcher lit
        // les touches, puis l'éditeur relit les mêmes évènements.
        let mut action = None;
        let mut output = ctx.run_ui(input, |ui| {
            action = app.dispatcher.poll(ui.ctx(), &app.keybindings, false);
            app.editor_ui(ui);
        });
        output.textures_delta.clear();

        assert!(action.is_some(), "la séquence {sequence:?} doit agir");
        assert_eq!(
            app.editor_mode,
            EditorMode::Normal,
            "après {sequence:?}, l'éditeur doit rester en mode commande"
        );
    }
}

/// Le cas réel : les touches d'une séquence arrivent sur des frames
/// séparées, une par frappe, avec relâchement — et non toutes dans le
/// même lot d'évènements comme lorsqu'on tape très vite.
#[test]
fn stepping_stays_in_normal_mode_across_separate_frames() {
    for sequence in [
        [egui::Key::Space, egui::Key::S, egui::Key::I],
        [egui::Key::Space, egui::Key::N, egui::Key::I],
    ] {
        let mut app = NamsdecApp::default();
        let ctx = egui::Context::default();

        for key in sequence {
            let input = egui::RawInput {
                events: vec![
                    egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            };
            // La frame entière, telle qu'eframe l'exécute : dispatch
            // des raccourcis, panneaux, éditeur.
            let mut output = ctx.run_ui(input, |ui| app.draw(ui));
            output.textures_delta.clear();
        }

        assert_eq!(
            app.editor_mode,
            EditorMode::Normal,
            "après {sequence:?} tapée touche par touche"
        );
    }
}

/// Le vrai parcours : on écrit du code (mode Insertion), on en sort
/// par Échap, puis on lance un pas au clavier. C'est là que le
/// raccourci doit fonctionner — et surtout ne pas rouvrir la saisie.
#[test]
fn stepping_works_after_leaving_insert_mode() {
    let mut app = NamsdecApp::default();
    let ctx = egui::Context::default();

    let stroke = |key| {
        vec![
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    let frame = |app: &mut NamsdecApp, key| {
        let input = egui::RawInput {
            events: stroke(key),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    };

    // `i` ouvre la saisie, Échap la referme.
    frame(&mut app, egui::Key::I);
    assert_eq!(app.editor_mode, EditorMode::Insert, "`i` ouvre la saisie");
    frame(&mut app, egui::Key::Escape);
    assert_eq!(app.editor_mode, EditorMode::Normal, "Échap la referme");

    // Puis la séquence de pas, touche par touche.
    for key in [egui::Key::Space, egui::Key::S, egui::Key::I] {
        frame(&mut app, key);
    }

    assert_eq!(
        app.editor_mode,
        EditorMode::Normal,
        "`Espace s i` après une édition ne doit pas rouvrir la saisie"
    );
}

/// Mais `i` tapé seul, lui, ouvre toujours la saisie : seule la touche
/// qui a servi à un raccourci est retirée à l'éditeur.
#[test]
fn plain_i_still_enters_insert_mode() {
    let mut app = NamsdecApp::default();
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::I,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };

    let mut output = ctx.run_ui(input, |ui| {
        app.dispatcher.poll(ui.ctx(), &app.keybindings, false);
        app.editor_ui(ui);
    });
    output.textures_delta.clear();

    assert_eq!(app.editor_mode, EditorMode::Insert);
}

/// Un programme qui lit son entrée puis la répète : il doit se
/// suspendre pour de bon en attendant, puis repartir tout seul dès que
/// l'utilisateur a tapé quelque chose — l'interface, elle, ne bloque
/// jamais.
#[test]
fn a_program_waits_for_console_input_then_echoes_it() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }

    let mut app = NamsdecApp {
        source: "\
section .bss
tampon resb 32

section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, tampon
mov rdx, 32
syscall

mov rdx, rax
mov rax, 1
mov rdi, 1
mov rsi, tampon
syscall

mov rax, 60
xor rdi, rdi
syscall
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(
        matches!(app.status, Status::AssembleOk),
        "obtenu {:?}",
        app.status
    );

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }

    assert!(
        matches!(app.status, Status::WaitingForInput),
        "le programme doit attendre son entrée, obtenu {:?}",
        app.status
    );
    assert!(!app.running, "l'exécution est suspendue, pas en cours");
    assert!(
        app.emulator.as_ref().unwrap().output().is_empty(),
        "il n'a encore rien écrit : il n'a rien lu"
    );

    // L'utilisateur tape sa réponse et valide.
    app.send_input(Some("bonjour".to_owned()));
    assert!(app.running, "un Exécuter suspendu doit repartir tout seul");

    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }

    assert_eq!(
        String::from_utf8_lossy(app.emulator.as_ref().unwrap().output()),
        "bonjour\n",
        "le programme répète ce qu'on lui a donné, saut de ligne compris"
    );
    assert!(matches!(app.status, Status::Exited { code: 0 }));
}

/// Fermer l'entrée débloque un programme qui lit jusqu'au bout, au
/// lieu de le laisser attendre pour toujours.
#[test]
fn ending_the_input_releases_a_waiting_program() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }

    let mut app = NamsdecApp {
        source: "\
section .bss
tampon resb 16

section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, tampon
mov rdx, 16
syscall
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }
    assert!(matches!(app.status, Status::WaitingForInput));

    // « Fin d'entrée » : `read` rend 0 et le programme continue.
    app.send_input(None);
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }

    assert!(
        app.program_finished(),
        "le programme doit être allé au bout"
    );
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax),
        0,
        "`read` rend une fin de fichier"
    );
}

/// Une largeur de panneau aberrante, héritée d'une session où la
/// mise en page s'était emballée, ne doit pas survivre : egui garde
/// cette largeur d'un lancement à l'autre, et elle revenait telle
/// quelle — panneau latéral sur toute la fenêtre, éditeur réduit à
/// une colonne de quelques caractères. Les bornes la ramènent dans le
/// raisonnable dès la première frame.
#[test]
fn an_absurd_stored_sidebar_width_is_brought_back() {
    let mut app = NamsdecApp::default();
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));

    // Ce que le bug avait laissé dans `app.ron` : le panneau part du
    // cent-unième pixel et va jusqu'au bord droit.
    ctx.data_mut(|d| {
        d.insert_persisted(
            egui::Id::new("sidebar"),
            egui::containers::PanelState {
                outer_rect: egui::Rect::from_min_max(
                    egui::pos2(100.0, 0.0),
                    egui::pos2(1280.0, 820.0),
                ),
            },
        );
    });

    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| app.draw(ui));
    output.textures_delta.clear();

    let width = egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
        .expect("le panneau latéral est affiché")
        .outer_rect
        .width();
    assert!(
        width <= screen.width() * SIDEBAR_MAX_FRACTION,
        "le panneau doit laisser au moins la moitié de la fenêtre à l'éditeur, \
         obtenu : {width}"
    );
    assert!(width >= SIDEBAR_MIN_WIDTH, "obtenu : {width}");
}

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
    let sample = std::fs::read_to_string(EXTERNAL_ABYSS).unwrap_or_else(|_| return_sample());
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

/// Un bouton de cette page doit se *voir* sans qu'on le survole.
///
/// Le thème met les boutons à plat au repos — bon pour les barres,
/// désastreux dans une boîte de dialogue : « Ajouter un thème… » se
/// lisait comme une phrase, et personne n'allait cliquer dessus. On
/// compare donc ce que les deux dessinent : le bouton ordinaire ne
/// trace aucun cadre, le nôtre en trace un.
#[test]
fn a_framed_button_draws_the_outline_a_flat_one_does_not() {
    let outlines = |framed: bool| {
        let ctx = egui::Context::default();
        ctx.set_visuals(DEFAULT_THEME.visuals());
        let mut count = 0;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            if framed {
                framed_button(ui, DEFAULT_THEME, "Ajouter un thème…");
            } else {
                let _ = ui.button("Ajouter un thème…");
            }
        });
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Rect(rect) = &clipped.shape
                && rect.stroke.width >= 1.0
                && rect.stroke.color == DEFAULT_THEME.dim
            {
                count += 1;
            }
        }
        output.textures_delta.clear();
        count
    };

    assert_eq!(
        outlines(false),
        0,
        "un bouton ordinaire reste plat au repos"
    );
    assert_eq!(outlines(true), 1, "le nôtre porte son cadre");
}

/// Retirer un thème installé : son fichier disparaît, il quitte la
/// liste, et les thèmes d'origine ne sont jamais concernés — ils ne
/// figurent pas dans la table des fichiers, donc aucun bouton ne les
/// propose.
#[test]
fn removing_an_installed_theme_deletes_its_file_and_frees_the_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jetable.namsdec-theme.json");
    let file = std::fs::read_to_string(EXTERNAL_ABYSS)
        .unwrap_or_else(|_| return_sample())
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

/// Le fichier d'exemple, ou une copie minimale quand le plugin n'est
/// pas sur cette machine : le test ne dépend pas de ce qui traîne
/// hors du dépôt.
fn return_sample() -> String {
    include_str!("../../namsdec-theme/tests/abyss.json").to_owned()
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
    let path = std::path::Path::new(EXTERNAL_ABYSS);
    assert!(path.exists(), "le plugin doit être là : {EXTERNAL_ABYSS}");

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
    let path = std::path::Path::new(EXTERNAL_ABYSS);
    if !path.exists() {
        // Le plugin vit hors du dépôt : sur une autre machine, ce
        // test n'a rien à vérifier.
        return;
    }
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

/// Accepter écrit le candidat *tel qu'il est*, casse comprise : une
/// étiquette vient de la source, et NASM distingue `Boucle` de
/// `boucle`. Compléter derrière ce qui est tapé écrivait une
/// étiquette qui n'existe pas.
#[test]
fn accepting_a_label_keeps_the_case_it_was_defined_with() {
    let mut app = NamsdecApp {
        source: "Boucle:\n    dec rcx\n    jnz bo".to_owned(),
        ..Default::default()
    };
    let suggestions = completion::suggest(&app.source, app.source.len()).expect("des propositions");
    assert_eq!(suggestions.accepted(), Some("Boucle"));

    app.accept_completion(&egui::Context::default(), &suggestions);
    assert!(
        app.source.ends_with("jnz Boucle"),
        "obtenu {:?}",
        app.source
    );
}

/// L'autocomplétion se règle, et le réglage est enregistré : c'est
/// une préférence de travail, pas une humeur du moment.
#[test]
fn completion_can_be_turned_off_and_the_choice_is_kept() {
    assert!(NamsdecApp::default().completion, "proposée par défaut");
    assert!(Settings::default().completion);

    let app = NamsdecApp {
        completion: false,
        ..Default::default()
    };
    let written = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(written.contains("\"completion\":false"), "{written}");

    // Et un fichier qui l'éteint la rend éteinte au lancement.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, r#"{"version": 2, "completion": false}"#).unwrap();
    let app = NamsdecApp::from_settings(Settings::load_from(&path));
    assert!(!app.completion);
}

/// Accepter une proposition écrit ce qu'il manque, là où il faut —
/// et rien d'autre. Le tampon reste ce que l'utilisateur a tapé
/// jusqu'à ce qu'il accepte.
#[test]
fn accepting_a_suggestion_writes_only_what_was_missing() {
    let mut app = NamsdecApp {
        source: "section .text\n_start:\n    mo".to_owned(),
        ..Default::default()
    };
    let suggestions = completion::suggest(&app.source, app.source.len()).expect("des propositions");
    assert_eq!(suggestions.candidates[0].text, "mov");

    app.accept_completion(&egui::Context::default(), &suggestions);
    assert_eq!(app.source, "section .text\n_start:\n    mov");
    assert!(app.suggestions.is_none(), "la liste se referme");

    // Au milieu d'une ligne, l'insertion se fait à la place du mot,
    // sans toucher à ce qui suit.
    let mut app = NamsdecApp {
        source: "    mo rax, 1".to_owned(),
        ..Default::default()
    };
    let suggestions = completion::suggest(&app.source, 6).expect("des propositions");
    app.accept_completion(&egui::Context::default(), &suggestions);
    assert_eq!(app.source, "    mov rax, 1");
}

/// L'éditeur réclame juste la hauteur visible — ni vingt-quatre
/// lignes en dur, qui donnaient une barre de défilement verticale sur
/// un fichier de six lignes, ni la hauteur du texte, qui aurait rendu
/// le clic inactif sous la dernière ligne.
#[test]
fn the_editor_asks_for_the_height_it_can_show() {
    // Vingt lignes de vingt pixels tiennent dans quatre cents.
    assert_eq!(editor_rows(400.0, 20.0), 20);
    // On arrondit vers le bas : une ligne à moitié visible ne compte
    // pas, sinon elle déborde et la barre revient.
    assert_eq!(editor_rows(409.0, 20.0), 20);
    // Jamais zéro ligne, et jamais de division par zéro.
    assert_eq!(editor_rows(5.0, 20.0), 1);
    assert_eq!(editor_rows(400.0, 0.0), 1);
}

/// L'éditeur ne réclame que la largeur de son code — jamais
/// l'infini, qui laissait la zone de défilement croire qu'il y avait
/// toujours quelque chose à atteindre à droite, barre horizontale
/// comprise, sous trois lignes de vingt caractères.
#[test]
fn the_editor_asks_for_the_width_of_its_code() {
    // Code étroit : on prend la place offerte, ni plus (rien à
    // faire défiler) ni moins (le clic reste actif jusqu'au bord).
    assert_eq!(editor_width(120.0, 800.0), 800.0);
    // Code plus large : c'est lui qui donne la mesure, et la barre
    // horizontale a alors une raison d'être.
    assert_eq!(editor_width(2400.0, 800.0), 2400.0);
    assert!(editor_width(2400.0, 800.0).is_finite(), "jamais l'infini");
}

/// Un numéro de la règle par ligne de code, quelle que soit la
/// largeur de l'éditeur. La gouttière compte une ligne rendue pour un
/// numéro : si le texte se repliait, une ligne source en occupait
/// plusieurs et la numérotation se décalait de tout ce qui précède.
#[test]
fn the_ruler_stays_line_for_line_however_narrow_the_editor() {
    let source = format!(
        "section .text\n    mov rax, 1 ; {}\n    hlt\n",
        "un commentaire à rallonge ".repeat(20)
    );
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        // Un éditeur réduit à une colonne étroite, comme lorsque le
        // panneau latéral avait tout pris.
        ui.set_max_width(80.0);
        let galley = layout_code(ui, DEFAULT_THEME, &source);
        assert_eq!(
            galley.rows.len(),
            // `split` et non `lines` : le retour à la ligne final
            // ouvre une ligne de plus, que l'éditeur numérote aussi.
            source.split('\n').count(),
            "une ligne source doit rester une ligne rendue"
        );
    });
    output.textures_delta.clear();
}

/// Un pas à pas reprend la main sur un Run suspendu faute d'entrée.
///
/// Le drapeau « reprendre le Run dès que l'entrée arrive » ne tombait
/// qu'à l'appel suivant de Exécuter. Un Run bloqué sur `read`, un Stop
/// sans effet (il ne regardait que `running`, faux pendant l'attente),
/// puis un Step Into : la ligne tapée pour ce pas-là relançait le
/// programme jusqu'au bout au lieu d'avancer d'une instruction.
#[test]
fn stepping_takes_over_a_run_that_was_waiting_for_input() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
section .bss
tampon resb 32

section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, tampon
mov rdx, 32
syscall
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }
    assert!(matches!(app.status, Status::WaitingForInput));
    assert!(app.resume_run_after_input, "le Run attend de repartir");

    // L'utilisateur reprend la main instruction par instruction.
    app.do_step();
    app.send_input(Some("oui".to_owned()));

    assert!(
        !app.running,
        "la saisie ne doit pas relancer le programme : c'est un pas à pas"
    );
    assert!(!app.resume_run_after_input);
}

/// Une saisie qui n'est pas une adresse laisse le vidage là où il
/// est. Avant, la moindre frappe de trop — `0x40_1000`, un `g` qui
/// traîne, un champ à moitié effacé — renvoyait l'affichage au début
/// du programme sans un mot d'explication.
#[test]
fn an_unparsable_address_keeps_the_one_on_screen() {
    assert_eq!(validated_address(0x401000, "0x400500", None), 0x400500);
    assert_eq!(
        validated_address(0x401000, "400500", None),
        0x400500,
        "le 0x est facultatif"
    );
    assert_eq!(validated_address(0x401000, "truc", None), 0x401000);
    assert_eq!(validated_address(0x401000, "0x40_1000", None), 0x401000);
    assert_eq!(validated_address(0x401000, "", None), 0x401000);
}

/// Et l'affichage d'une frame entière avec une saisie invalide ne
/// déplace rien de lui-même : il faut valider (Entrée ou le bouton).
#[test]
fn typing_alone_does_not_move_the_memory_view() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Memory,
        mem_addr: 0x401000,
        mem_addr_input: "pas une adresse".to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
        )),
        ..Default::default()
    };
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
    }

    assert_eq!(
        app.mem_addr, 0x401000,
        "le vidage reste sur la dernière adresse validée"
    );
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

/// Le champ Adresse comprend les registres, et une petite
/// arithmétique autour : c'est ce qui permet de suivre un pointeur
/// qu'on a sous les yeux dans le bandeau, sans recopier seize
/// chiffres à la main.
#[test]
fn the_address_field_understands_registers_and_arithmetic() {
    let mut regs = namsdec_core::Registers::default();
    regs.set64(Reg64::Rsp, 0x40ff00);
    regs.set64(Reg64::Rsi, 0x401000);
    regs.set64(Reg64::Rcx, 4);
    regs.rip = 0x400020;
    let at = |expr: &str| eval_address(expr, Some(&regs));

    // Ce qui marchait déjà marche toujours.
    assert_eq!(at("0x401000"), Some(0x401000));
    assert_eq!(at("401000"), Some(0x401000), "le 0x reste facultatif");

    // Les registres, RIP compris.
    assert_eq!(at("rsp"), Some(0x40ff00));
    assert_eq!(at("RSI"), Some(0x401000), "la casse est indifférente");
    assert_eq!(at("rip"), Some(0x400020));

    // L'arithmétique, en hexadécimal de bout en bout.
    assert_eq!(
        at("rsi+10"),
        Some(0x401010),
        "les décalages aussi sont hexa"
    );
    assert_eq!(at("rsp-8"), Some(0x40fef8));
    assert_eq!(
        at(" rsi + rcx * 4 "),
        Some(0x401010),
        "l'indexation d'un tableau"
    );
    assert_eq!(at("rsp-8+8"), Some(0x40ff00));

    // Et ce qui n'est pas une adresse est refusé, plutôt que deviné.
    assert_eq!(at("truc"), None);
    assert_eq!(at("rsi+"), None, "une somme inachevée");
    assert_eq!(at(""), None);
    assert_eq!(at("rax*"), None);

    // Sans processeur chargé, les registres n'ont pas de valeur : le
    // champ le dit au lieu d'inventer un zéro.
    assert_eq!(eval_address("rsp", None), None);
    assert_eq!(eval_address("0x401000", None), Some(0x401000));
}

/// Les trois lectures de la mémoire disent trois choses différentes
/// des mêmes octets.
#[test]
fn the_three_memory_formats_read_the_same_bytes_differently() {
    let mut mem = namsdec_core::Memory::new(BASE_ADDR, 0x1000);
    // « Hi », un octet nul, puis un mot de huit octets valant 1.
    mem.load(BASE_ADDR, b"Hi\0\0\0\0\0\0\x01\0\0\0\0\0\0\0")
        .unwrap();

    let bytes = read_memory(&mem, BASE_ADDR, MemoryFormat::Bytes, "hors bornes");
    assert!(bytes.contains("48 69"), "l'hexadécimal brut : {bytes}");

    let words = read_memory(&mem, BASE_ADDR, MemoryFormat::Words, "hors bornes");
    let second = words.lines().nth(1).expect("un deuxième mot");
    assert!(
        second.ends_with("  1") && second.contains("0000000000000001"),
        "le mot suivant, en hexa et en décimal : {second}"
    );

    let text = read_memory(&mem, BASE_ADDR, MemoryFormat::Text, "hors bornes");
    assert!(
        text.starts_with("Hi"),
        "le texte s'arrête au premier nul : {text}"
    );
    assert!(!text.contains("48"), "et ne montre plus d'hexadécimal");

    // Hors de la fenêtre simulée, les trois le disent.
    for format in MemoryFormat::ALL {
        assert_eq!(
            read_memory(&mem, 0xdead_beef, format, "hors bornes"),
            "hors bornes"
        );
    }
}

/// Le champ « Adresse » a la largeur d'une adresse, pas celle du
/// panneau. Un champ de saisie egui réclame par défaut toute la place
/// disponible : sur un panneau large, celui-ci s'étirait sur des
/// centaines de pixels pour dix-huit caractères.
#[test]
fn the_address_field_is_as_wide_as_an_address_not_as_the_panel() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));

    // La même vue dans un panneau étroit, puis dans un panneau deux
    // fois plus large.
    let field_width = |sidebar_width: f32| {
        let mut app = NamsdecApp {
            active_tab: SidebarTab::Memory,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        ctx.data_mut(|d| {
            d.insert_persisted(
                egui::Id::new("sidebar"),
                egui::containers::PanelState {
                    outer_rect: egui::Rect::from_min_max(
                        egui::pos2(screen.right() - sidebar_width, 0.0),
                        egui::pos2(screen.right(), screen.bottom()),
                    ),
                },
            );
        });
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }
        let sidebar = egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
            .expect("le panneau latéral est affiché")
            .outer_rect;
        let field = ctx
            .read_response(memory_address_id())
            .expect("le champ Adresse est affiché")
            .rect;
        assert!(
            field.right() <= sidebar.right() + 1.0,
            "le champ ne doit pas dépasser le panneau ({} contre {})",
            field.right(),
            sidebar.right()
        );
        (sidebar.width(), field.width())
    };

    let (narrow_panel, narrow_field) = field_width(300.0);
    let (wide_panel, wide_field) = field_width(620.0);

    assert!(
        wide_panel > narrow_panel,
        "les deux panneaux doivent différer"
    );
    assert_eq!(
        narrow_field, wide_field,
        "le champ doit garder sa taille quand le panneau s'élargit"
    );
    assert!(
        wide_field < wide_panel * 0.6,
        "et rester un champ d'adresse, pas une barre : {wide_field} px"
    );
}

/// La console vit maintenant dans le panneau bas, sur toute la
/// largeur de la fenêtre : c'est ce qui lui rend les 80 colonnes
/// qu'un programme suppose quand il écrit, là où le panneau latéral
/// n'en offrait que trente-quatre. Sa saisie reste collée en bas,
/// même quand le programme a beaucoup écrit.
#[test]
fn the_console_panel_spans_the_window_and_keeps_its_input_at_the_bottom() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "section .data\n    msg db \"beaucoup de sortie\", 10\n\
                 \x20   len equ $ - msg\nsection .text\n    global _start\n_start:\n\
                 \x20   mov rcx, 40\nboucle:\n    push rcx\n    mov rax, 1\n\
                 \x20   mov rdi, 1\n    mov rsi, msg\n    mov rdx, len\n    syscall\n\
                 \x20   pop rcx\n    dec rcx\n    jnz boucle\n    hlt\n"
            .to_owned(),
        console_tab: ConsoleTab::Output,
        ..Default::default()
    };
    app.do_assemble();
    assert!(
        matches!(app.status, Status::AssembleOk),
        "obtenu : {:?}",
        app.status
    );
    app.do_run();

    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    // Deux frames : la première pose les panneaux, la seconde les
    // montre à leur vraie place.
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
    }

    assert!(
        !app.emulator.as_ref().unwrap().output().is_empty(),
        "le programme doit avoir écrit quelque chose"
    );
    let console = egui::containers::PanelState::load(&ctx, egui::Id::new("console"))
        .expect("le panneau de la console est affiché")
        .outer_rect;
    let console_input = egui::containers::PanelState::load(&ctx, egui::Id::new("console_input"))
        .expect("la saisie de la console est affichée")
        .outer_rect;

    assert_eq!(
        console.width(),
        screen.width(),
        "la console prend toute la largeur, panneau latéral compris"
    );
    assert!(
        console.height() <= screen.height() * CONSOLE_MAX_FRACTION,
        "sans jamais dévorer l'éditeur, obtenu : {}",
        console.height()
    );
    // À la marge intérieure du panneau près.
    assert!(
        (console.bottom() - console_input.bottom()).abs() <= 8.0,
        "la saisie doit toucher le bas du panneau ({} contre {})",
        console_input.bottom(),
        console.bottom()
    );
}

/// Un assemblage qui échoue amène ses diagnostics sous les yeux :
/// le panneau bas s'ouvre sur l'onglet qui les porte. La barre de
/// statut dit « Erreur d'assemblage » — encore faut-il pouvoir lire
/// laquelle. Et quand l'erreur est corrigée, la place revient à la
/// console, d'où repart l'exécution.
#[test]
fn a_failed_assembly_brings_up_its_diagnostics() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    mov rax, pasunregistre\n".to_owned(),
        show_console: false,
        ..Default::default()
    };

    app.do_assemble();
    assert!(
        matches!(app.status, Status::AssembleFailed),
        "{:?}",
        app.status
    );
    assert!(app.show_console, "le panneau bas doit s'ouvrir");
    assert_eq!(app.console_tab, ConsoleTab::Diagnostics);

    app.source = "global _start\nsection .text\n_start:\n    hlt\n".to_owned();
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    assert_eq!(
        app.console_tab,
        ConsoleTab::Output,
        "plus de diagnostic : la console reprend la place"
    );
}

/// Les diagnostics de nasm ont quitté la barre de statut pour l'onglet
/// Diagnostics : la barre ne doit plus grandir quand l'assemblage
/// échoue. Elle s'étirait jusqu'à cinq lignes, en bas de la fenêtre,
/// pour une liste qu'on n'y lisait pas.
#[test]
fn diagnostics_no_longer_swell_the_status_bar() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
    let height_of_status_bar = |source: &str| {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }
        let bar = egui::containers::PanelState::load(&ctx, egui::Id::new("statusbar"))
            .expect("la barre de statut est affichée")
            .outer_rect
            .height();
        (app.diagnostics.len(), bar)
    };

    let (clean, clean_height) =
        height_of_status_bar("global _start\nsection .text\n_start:\n    hlt\n");
    let (broken, broken_height) = height_of_status_bar(
        "global _start\nsection .text\n_start:\n    mov rax, pasunregistre\n\
         \n    mov rbx, pasnonplus\n",
    );

    assert_eq!(clean, 0);
    assert!(broken >= 2, "obtenu {broken} diagnostic(s)");
    assert_eq!(
        clean_height, broken_height,
        "la barre de statut doit garder sa hauteur, diagnostics ou pas"
    );
}

/// Le bandeau montre les seize registres généraux et RIP : un
/// programme qui travaille dans R8 ou R12 doit pouvoir être suivi
/// comme un autre.
#[test]
fn the_cpu_banner_shows_every_general_register() {
    use std::collections::BTreeSet;
    let names: BTreeSet<&str> = REGISTERS.iter().map(|&(name, _)| name).collect();
    assert_eq!(names.len(), 16, "seize registres, sans doublon");
    for expected in [
        "RAX", "RBX", "RCX", "RDX", "RSI", "RDI", "RSP", "RBP", "R8", "R9", "R10", "R11", "R12",
        "R13", "R14", "R15",
    ] {
        assert!(names.contains(expected), "{expected} manque au bandeau");
    }

    // Et chaque nom désigne bien son registre : une valeur écrite
    // dans l'un doit ressortir sous le bon nom.
    let mut regs = namsdec_core::Registers::default();
    for (index, &(_, reg)) in REGISTERS.iter().enumerate() {
        regs.set64(reg, 0x1000 + index as u64);
    }
    for (index, &(name, reg)) in REGISTERS.iter().enumerate() {
        assert_eq!(
            regs.get64(reg),
            0x1000 + index as u64,
            "{name} ne lit pas son propre registre"
        );
    }
}

/// Le bandeau sur deux colonnes doit tenir dans le panneau élargi
/// sans le pousser — et sans casser : la cellule qui sépare les deux
/// colonnes est une cellule *allouée*, pas un `add_space`, qu'egui
/// refuse dans une grille.
#[test]
fn the_two_column_banner_fits_the_panel_it_is_given() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
    let wide = 620.0;
    let mut app = NamsdecApp::default();
    app.do_assemble();
    assert!(app.emulator.is_some(), "il faut un processeur à montrer");

    let ctx = egui::Context::default();
    ctx.data_mut(|d| {
        d.insert_persisted(
            egui::Id::new("sidebar"),
            egui::containers::PanelState {
                outer_rect: egui::Rect::from_min_max(
                    egui::pos2(screen.right() - wide, 0.0),
                    egui::pos2(screen.right(), screen.bottom()),
                ),
            },
        );
    });
    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let mut widths = Vec::new();
    for _ in 0..3 {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
        widths.push(
            egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
                .expect("le panneau latéral est affiché")
                .outer_rect
                .width(),
        );
    }
    assert!(
        widths.iter().all(|w| *w == wide),
        "le panneau doit garder ses {wide} px, obtenu : {widths:?}"
    );
}

/// La grille des registres ajoute une colonne dès qu'elle tient, et
/// jamais avant : une paire de trop, et c'est le panneau lui-même qui
/// s'élargirait pour la contenir.
///
/// Les largeurs sont celles d'une paire mesurée dans la police réelle —
/// autour de 157 px pour la police par défaut.
#[test]
fn the_register_grid_adds_a_column_only_when_it_fits() {
    const PAIR: f32 = 157.0;
    const GAP: f32 = 24.0;
    assert_eq!(register_columns(SIDEBAR_MIN_WIDTH, PAIR, GAP), 1);
    assert_eq!(
        register_columns(337.0, PAIR, GAP),
        1,
        "trop juste : une seule"
    );
    assert_eq!(
        register_columns(338.0, PAIR, GAP),
        2,
        "la deuxième tient pile"
    );
    assert_eq!(register_columns(518.0, PAIR, GAP), 2, "pas encore trois");
    assert_eq!(
        register_columns(519.0, PAIR, GAP),
        3,
        "la troisième tient pile"
    );
    assert_eq!(
        register_columns(1000.0, PAIR, GAP),
        4,
        "jamais plus de quatre"
    );
    // Une police introuvable ne doit pas faire diviser par zéro.
    assert_eq!(register_columns(400.0, 0.0, GAP), 1);
}

/// La palette ne propose pas la vue RE tant qu'elle est masquée.
///
/// Elle la proposait toujours, et s'y rendre *allumait* le réglage —
/// enregistré dans la foulée. Un aller-retour dans la palette
/// suffisait donc à faire réapparaître l'onglet pour de bon, sans
/// être passé par Préférences.
#[test]
fn the_palette_offers_the_re_view_only_when_it_is_visible() {
    let s = strings(Lang::Fr);
    let labels = |show_re_tab| -> Vec<String> {
        palette_commands(s, show_re_tab, false)
            .into_iter()
            .map(|(label, _)| label)
            .collect()
    };
    let entry = format!("{}: {}", s.menu_view, s.tab_re);

    assert!(
        !labels(false).contains(&entry),
        "onglet masqué : la commande n'a pas à être offerte"
    );
    assert!(labels(true).contains(&entry), "onglet visible : elle l'est");

    // Et s'y rendre ne touche plus au réglage.
    let mut app = NamsdecApp {
        show_re_tab: false,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.execute_palette_command(&ctx, PaletteCommand::GoTab(SidebarTab::Re));
    assert!(
        !app.show_re_tab,
        "aller quelque part ne doit pas activer une vue désactivée"
    );
}

/// La boîte « À propos » s'ouvre au centre, porte le lien de la
/// licence, le bouton de don et celui de mise à jour — lequel dit
/// franchement que rien n'est branché derrière.
#[test]
fn the_about_box_is_centred_and_carries_its_three_offers() {
    let mut app = NamsdecApp {
        show_about: true,
        ..Default::default()
    };
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
    }

    let window = egui::AreaState::load(&ctx, about_window_id()).expect("la boîte est affichée");
    let rect = window.rect();
    let offset = rect.center() - screen.center();
    assert!(
        offset.length() < 1.0,
        "la boîte doit être centrée, écart : {offset:?}"
    );

    // Le lien de licence pointe sur le texte de la MIT, et celui du
    // don sur une adresse https (fictive pour l'instant).
    assert!(MIT_LICENSE_URL.starts_with("https://"));
    assert!(
        DONATION_URL.is_empty() || DONATION_URL.starts_with("https://"),
        "un lien de don renseigné doit être une adresse https"
    );

    // L'illustration embarquée fait bien la taille annoncée : quatre
    // octets par pixel, sur un carré de `TREAT_IMAGE_SIDE`. Un
    // fichier régénéré à une autre dimension ferait paniquer
    // `ColorImage::from_rgba_unmultiplied` à l'ouverture de la boîte.
    assert_eq!(
        TREAT_IMAGE.len(),
        TREAT_IMAGE_SIDE * TREAT_IMAGE_SIDE * 4,
        "assets/offrir-un-repas.rgba ne correspond plus à TREAT_IMAGE_SIDE"
    );

    // Le bouton de mise à jour ne prétend rien : il lève seulement la
    // note « pas encore branché », qui ne survit pas à la fermeture.
    app.about_update_clicked = true;
    app.show_about = false;
    let mut output = ctx.run_ui(input, |ui| app.draw(ui));
    output.textures_delta.clear();
    assert!(
        !app.about_update_clicked,
        "la note se referme avec la boîte"
    );
}

/// La chaîne des appels nomme les lignes source, pas seulement des
/// adresses : c'est ce qui la rend lisible pour quelqu'un qui apprend
/// — « appelé depuis la ligne 4 », et non « 0x400000 ».
#[test]
fn the_call_chain_names_source_lines() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
call externe
hlt

externe:
call interne
ret

interne:
mov rax, 1
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    app.do_step(); // call externe
    app.do_step(); // call interne

    let state = app.assemble_state.as_ref().expect("un binaire assemblé");
    let frames = app.emulator.as_ref().unwrap().frames();
    assert_eq!(frames.len(), 2, "deux appels imbriqués");

    // Les deux sites d'appel se traduisent en lignes du source : le
    // `call externe` de la ligne 4, le `call interne` de la ligne 8.
    let lines: Vec<u32> = frames
        .iter()
        .map(|f| state.addr_to_line[&f.call_site])
        .collect();
    assert_eq!(lines, [4, 8], "obtenu {lines:?}");

    // Et le pas arrière défait la chaîne comme le reste.
    app.do_step_back();
    assert_eq!(app.emulator.as_ref().unwrap().frames().len(), 1);
}

/// Le pas arrière rend la main à l'utilisateur et met en avant ce
/// que le pas défait avait changé — la photographie est prise avant
/// de reculer, donc la couleur désigne exactement ce qu'on annule.
#[test]
fn stepping_back_shows_what_it_undid() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
mov rax, 7
mov rbx, 9
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    app.do_step(); // mov rax, 7
    app.do_step(); // mov rbx, 9
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rbx),
        9
    );

    app.do_step_back();
    assert!(
        matches!(app.status, Status::SteppedBack),
        "{:?}",
        app.status
    );
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rbx),
        0,
        "RBX est revenu à ce qu'il était"
    );
    assert_eq!(
        changed_registers(
            app.regs_before.as_ref(),
            &app.emulator.as_ref().unwrap().cpu().regs
        ),
        ["RBX"],
        "et c'est lui que le bandeau montre"
    );

    // Jusqu'au début du programme, puis plus rien.
    while app.emulator.as_ref().unwrap().can_step_back() {
        app.do_step_back();
    }
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax),
        0
    );
    app.do_step_back();
    assert!(
        matches!(app.status, Status::NothingToUndo),
        "au début, il le dit plutôt que de ne rien faire : {:?}",
        app.status
    );
}

/// Une zone surveillée arrête l'exécution et le dit : l'adresse, la
/// valeur d'avant, celle d'après. C'est l'information qu'on attendait
/// en posant le point — sans elle, l'arrêt ne servirait à rien.
#[test]
fn a_watched_area_stops_the_run_and_says_what_changed() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
section .data
compteur dq 0

section .text
global _start
_start:
mov rax, 42
mov [compteur], rax
mov rbx, 7
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let compteur = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .sections
        .iter()
        .find(|s| s.name == ".data")
        .expect("une section .data")
        .addr;

    // Comme le bouton « Surveiller » : l'adresse entre dans la liste,
    // et le moteur l'arme sur l'état courant.
    app.watchpoints.push((compteur, 8));
    app.sync_watchpoints();

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }

    match app.status {
        Status::Watchpoint { addr, old, new } => {
            assert_eq!(addr, compteur);
            assert_eq!((old, new), (0, 42));
        }
        ref other => panic!("attendu un arrêt sur surveillance, obtenu {other:?}"),
    }
    assert!(!app.running, "l'exécution est suspendue");
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rbx),
        0,
        "arrêté juste après l'écriture, avant la suite"
    );

    // Et le réassemblage garde la zone : on corrige son programme
    // sans avoir à reposer ses points.
    app.do_assemble();
    assert_eq!(app.emulator.as_ref().unwrap().watchpoints().len(), 1);
}

/// Le bandeau met en avant les registres que la dernière avancée a
/// changés — c'est ce qui fait *voir* ce qu'une instruction fait,
/// sans comparer seize valeurs de tête à chaque pas.
#[test]
fn the_banner_points_out_what_the_last_step_changed() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
mov rax, 7
mov rbx, 9
add rax, rbx
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let changed = |app: &NamsdecApp| {
        changed_registers(
            app.regs_before.as_ref(),
            &app.emulator.as_ref().unwrap().cpu().regs,
        )
    };

    // Rien n'a encore bougé : le bandeau reste uni.
    assert!(
        changed(&app).is_empty(),
        "un programme jamais exécuté n'a rien changé"
    );

    app.do_step(); // mov rax, 7
    assert_eq!(changed(&app), ["RAX"]);

    app.do_step(); // mov rbx, 9
    assert_eq!(changed(&app), ["RBX"], "seul le dernier pas compte");

    app.do_step(); // add rax, rbx
    assert_eq!(changed(&app), ["RAX"]);

    // Et une exécution complète montre tout ce qu'elle a changé,
    // depuis son départ.
    app.do_assemble();
    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }
    assert_eq!(changed(&app), ["RAX", "RBX"]);
}

/// Le bandeau d'état s'ouvre déployé à chaque lancement.
///
/// egui retient l'état des en-têtes repliables d'une session à
/// l'autre : replié une fois, il revenait replié, registres cachés,
/// sans que rien ne dise pourquoi. La mesure porte sur ce qui suit le
/// bandeau — le champ Adresse de l'onglet Mémoire descend de toute la
/// hauteur des registres quand ils sont là.
#[test]
fn the_cpu_banner_opens_deployed() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    assert!(
        NamsdecApp::default().cpu_state_open,
        "déployé par défaut, sans rien avoir à régler"
    );

    let top_of_address_field = |open: bool| {
        let mut app = NamsdecApp {
            active_tab: SidebarTab::Memory,
            cpu_state_open: open,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 820.0),
            )),
            ..Default::default()
        };
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }
        ctx.read_response(memory_address_id())
            .expect("le champ Adresse est affiché")
            .rect
            .top()
    };

    assert!(
        top_of_address_field(true) > top_of_address_field(false) + 100.0,
        "déployé, le bandeau doit occuper la place des registres"
    );
}

/// L'onglet RE n'apparaît dans la barre latérale que si on l'a
/// demandé, **et seulement pour la session en cours**.
///
/// Il ouvre un module de reverse engineering encore à l'état
/// d'ébauche. Tant que le « oui » était enregistré dans
/// `settings.json`, il suffisait de l'avoir donné une fois — ou de
/// l'avoir donné sans le vouloir, ce que la commande « Affichage :
/// RE » de la palette faisait au passage — pour le retrouver à tous
/// les lancements suivants, sans savoir d'où il venait.
#[test]
fn the_re_tab_is_absent_at_every_launch_until_it_is_asked_for() {
    assert!(!NamsdecApp::default().show_re_tab, "application par défaut");

    // Un fichier de réglages n'a plus voix au chapitre, même celui
    // d'une version qui écrivait encore ce champ.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(
        &path,
        r#"{"version": 2, "show_re_tab": true, "language": "fr"}"#,
    )
    .unwrap();
    let settings = Settings::load_from(&path);
    let app = NamsdecApp::from_settings(settings);
    assert!(
        !app.show_re_tab,
        "un ancien fichier ne doit plus pouvoir rallumer l'onglet"
    );

    // Et la rangée d'onglets ne le propose pas.
    let shown: Vec<&str> = SidebarTab::ALL
        .iter()
        .filter(|tab| app.show_re_tab || **tab != SidebarTab::Re)
        .map(|tab| tab.label(strings(Lang::Fr)))
        .collect();
    assert_eq!(shown, ["Pile", "Mémoire", "Tas", "Octets"]);

    // Ce qu'on coche dans Préférences vaut pour la session, et ne
    // part pas dans les réglages enregistrés.
    let app = NamsdecApp {
        show_re_tab: true,
        ..Default::default()
    };
    let written = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(
        !written.contains("show_re_tab"),
        "le champ n'a plus à être écrit : {written}"
    );
}

/// Le panneau latéral garde la largeur qu'on lui a donnée, quel que
/// soit l'onglet ouvert et ce qu'il contient.
///
/// La ligne de saisie de la Console devinait sa largeur (« la place
/// disponible moins 120 px ») et débordait de quelques pixels. Or un
/// panneau egui retient comme largeur celle de son contenu : le
/// débordement d'une frame devenait la largeur de la suivante, qui
/// débordait à son tour. Ouvrir l'onglet Console élargissait donc le
/// panneau de 28 px par frame — soixante fois par seconde — jusqu'à
/// ce qu'il occupe la fenêtre, éditeur compris. Et comme egui
/// enregistre cette largeur, elle revenait telle quelle au lancement
/// suivant.
#[test]
fn no_sidebar_tab_widens_the_panel() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // De quoi remplir chaque onglet : un programme qui tourne (donc
    // des registres, une pile, une sortie), une longue ligne de
    // sortie, et une longue ligne tapée dans la console.
    let source = format!(
        "section .data\n    msg db \"{}\", 10\n    len equ $ - msg\n\
         section .text\n    global _start\n_start:\n\
         \x20   mov rax, 1\n    mov rdi, 1\n    mov rsi, msg\n    mov rdx, len\n\
         \x20   syscall\n    hlt\n",
        "texte bien trop long pour la largeur du panneau ".repeat(6)
    );

    for tab in SidebarTab::ALL {
        let mut app = NamsdecApp {
            source: source.clone(),
            // Même l'onglet RE, masqué par défaut, ne doit pas
            // pousser la rangée d'onglets au-delà du panneau.
            show_re_tab: true,
            console_input: "x".repeat(300),
            active_tab: tab,
            ..Default::default()
        };
        app.do_assemble();
        app.do_run();

        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 820.0),
            )),
            ..Default::default()
        };
        let mut widths = Vec::new();
        for _ in 0..4 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
            widths.push(
                egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
                    .expect("le panneau latéral est affiché")
                    .outer_rect
                    .width(),
            );
        }

        let name = tab.label(strings(Lang::Fr));
        assert!(
            widths.iter().all(|w| *w == SIDEBAR_WIDTH),
            "l'onglet {name} doit laisser le panneau à {SIDEBAR_WIDTH} px, obtenu : {widths:?}"
        );
    }
}

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

/// Ouvrir un binaire doit mener quelque part : un vrai programme
/// passé par nasm, relu comme des octets bruts, doit remplir
/// l'éditeur d'un listing, donner la correspondance ligne <-> adresse
/// dont dépend la gouttière, et produire un émulateur capable
/// d'avancer.
#[test]
fn opening_a_raw_binary_disassembles_and_boots_the_emulator() {
    let Some(_nasm) = namsdec_asm::find_nasm() else {
        // Pas de nasm ici : ce test vérifie un aller-retour à travers
        // l'assembleur réel, il n'a rien à prouver sans lui.
        return;
    };
    let output = namsdec_asm::assemble(
        "BITS 64\nsection .text\n    mov rax, 7\n    hlt\n",
        BASE_ADDR,
    )
    .expect("le programme de test doit s'assembler");

    let mut app = NamsdecApp::default();
    app.load_binary(output.bytes);

    assert!(
        app.source.contains("mov"),
        "le listing désassemblé doit remplir l'éditeur, obtenu : {:?}",
        app.source
    );
    assert_eq!(
        app.current_file, None,
        "le tampon est reconstruit : pas de fichier à réécrire dessus"
    );

    let state = app.assemble_state.as_ref().expect("un état chargé");
    assert_eq!(
        state.output.line_to_addr.get(&1).copied(),
        Some(BASE_ADDR),
        "la première ligne du listing est la première instruction"
    );
    assert_eq!(
        app.current_line(),
        Some(1),
        "RIP au départ doit se refléter sur la ligne 1 de la gouttière"
    );

    app.do_step();
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax),
        7,
        "le binaire chargé doit s'exécuter pas à pas comme un programme assemblé"
    );
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

#[test]
fn cursor_position_counts_characters_not_bytes() {
    let text = "mov rax, 0\nsum_loop:\n    add rax, rcx";

    assert_eq!(line_col(text, 0), (1, 1), "début du texte");
    assert_eq!(line_col(text, 4), (1, 5), "milieu de la première ligne");
    // Index 11 = juste après le '\n' : début de la ligne 2.
    assert_eq!(line_col(text, 11), (2, 1), "après un saut de ligne");
    assert_eq!(
        line_col(text, text.chars().count()),
        (3, 17),
        "fin du texte"
    );

    // Des caractères non-ASCII ne doivent pas décaler la colonne :
    // egui compte des caractères, pas des octets.
    assert_eq!(line_col("éàü|", 3), (1, 4), "trois caractères accentués");
}

/// Un fichier plus gros que la mémoire simulée est tronqué plutôt que
/// rejeté — et le statut le dit, au lieu de laisser croire que tout a
/// été chargé.
#[test]
fn oversized_binaries_are_truncated_not_rejected() {
    let mut app = NamsdecApp::default();
    app.load_binary(vec![0x90; MEM_SIZE + 1024]); // que des `nop`

    match app.status {
        Status::BinaryLoaded { bytes, truncated } => {
            assert_eq!(bytes, MEM_SIZE);
            assert!(truncated);
        }
        _ => panic!("un binaire trop gros doit tout de même se charger"),
    }
}
/// L'onglet Tas montre la frontière que `brk` déplace. Aucun registre
/// ne la porte : sans l'émulateur qui la tient, ni l'utilisateur ni un
/// débogueur ne sauraient où la chercher.
#[test]
fn the_heap_tab_shows_the_break_the_program_moved() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Heap,
        source: "\
section .text
global _start
_start:
mov rax, 12
xor rdi, rdi
syscall
mov rdi, rax
add rdi, 4096
mov rax, 12
syscall
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    let heap = app.emulator.as_ref().unwrap().heap();
    assert_eq!(heap.len(), 0, "rien n'est demandé avant de tourner");

    for _ in 0..10 {
        app.do_step();
    }
    let heap = app.emulator.as_ref().unwrap().heap();
    assert_eq!(heap.len(), 4096, "le programme a pris 4 Kio");

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
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
                painted.push('\n');
            }
        }
        output.textures_delta.clear();
    }

    for expected in [
        format!("{:#x}", heap.start()),
        format!("{:#x}", heap.current()),
        format!("{:#x}", heap.limit()),
    ] {
        assert!(
            painted.contains(&expected),
            "l'onglet doit montrer {expected} ; peint : {painted}"
        );
    }
    assert!(
        painted.contains("4096 / "),
        "il doit dire ce qui est pris sur ce qui reste"
    );
}
/// Un registre SSE qui change se met en avant comme un registre
/// général : c'est la même question — qu'est-ce que ce pas a fait ?
#[test]
fn a_changed_xmm_register_stands_out_like_the_others() {
    let before = namsdec_core::Registers::new();
    let mut now = before.clone();
    now.set_xmm(namsdec_core::RegXmm::Xmm3, 5.0f64.to_bits().into());
    assert_eq!(changed_registers(Some(&before), &now), ["XMM3"]);

    // Et la valeur se lit en clair à côté des chiffres.
    assert_eq!(short_float(5.0), "= 5");
    assert_eq!(short_float(0.0), "", "un registre vide ne dit rien");
    assert_eq!(
        short_float(5.0 / 9.0),
        "= 0.555556",
        "six décimales au plus"
    );
    assert_eq!(
        short_float(f64::from_bits(1.0f32.to_bits().into())),
        "= 5.264e-315",
        "un simple précision relu comme un double reste lisible"
    );
}
/// Une case de la pile x87 qui se remplit se met en avant comme un
/// registre : empiler *est* un changement d'état.
#[test]
fn a_pushed_x87_slot_stands_out() {
    let before = namsdec_core::Registers::new();
    let mut now = before.clone();
    now.x87.push(3.5);
    assert_eq!(changed_registers(Some(&before), &now), ["ST0"]);
}
/// AVX élargit les registres sans les renommer : `xmm1` est la moitié
/// basse de `ymm1`. Le panneau doit le dire — et une moitié haute qui
/// change est un changement comme un autre.
#[test]
fn a_register_with_an_upper_half_is_shown_as_ymm() {
    let before = namsdec_core::Registers::new();
    let mut now = before.clone();
    now.set_ymm(namsdec_core::RegXmm::Xmm1, [0, 1]);
    assert_eq!(
        changed_registers(Some(&before), &now),
        ["XMM1"],
        "la moitié haute compte"
    );

    assert_eq!(vector_name("XMM1", [7, 0]), "XMM1");
    assert_eq!(vector_name("XMM1", [7, 1]), "YMM1", "il y a plus à voir");
    let tip = xmm_tooltip(strings(Lang::Fr), [7, 1]);
    assert!(
        tip.contains(&format!("{:032x}{:032x}", 1u128, 7u128)),
        "les 256 bits, la haute d'abord : {tip}"
    );
}
/// Le mot sous le curseur : c'est lui qu'on documente, pas le premier
/// de la ligne. On demande aussi bien ce que fait `movzx` que le
/// `rep` qui le précède.
#[test]
fn the_word_under_the_cursor_is_the_one_asked_about() {
    let source = "    rep movsb\n_start:\n    mov rax, 1\n";
    // Au milieu d'un mot, à son début, à sa fin.
    assert_eq!(word_at(source, 6), "rep");
    assert_eq!(word_at(source, 4), "rep");
    assert_eq!(word_at(source, 7), "rep", "juste après, on est dedans");
    assert_eq!(word_at(source, 9), "movsb");
    // Les noms d'étiquettes gardent leur souligné et leur point.
    assert_eq!(word_at("_start:\n", 3), "_start");
    assert_eq!(word_at("    jmp .boucle\n", 12), ".boucle");
    // Mais un point de ponctuation n'en fait pas partie.
    assert_eq!(word_at("; … avant idiv.\n", 15), "idiv");
    // Dans le blanc, il n'y a pas de mot.
    assert_eq!(word_at(source, 0), "");
    // Et hors bornes, pas de panique.
    assert_eq!(word_at(source, 9_999), "");
}

/// F1 ouvre la fiche de l'instruction où l'on est, dans l'onglet
/// d'aide — une fenêtre, et non un onglet de la console : celle-ci porte
/// ce que le programme dit, pas une fiche d'instruction.
#[test]
fn f1_opens_the_entry_for_the_instruction_under_the_cursor() {
    let mut app = NamsdecApp {
        source: "_start:\n    cqo\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = 13; // au milieu de « cqo »
    app.dispatch(Action::InstructionHelp);

    assert!(app.show_help, "l'aide ouvre sa fenêtre");
    assert_eq!(
        ConsoleTab::ALL.len(),
        2,
        "et la console garde ses deux onglets"
    );
    let (word, entry) = app.help.clone().expect("une demande d'aide");
    assert_eq!(word, "cqo");
    let entry = entry.expect("cqo doit être documentée");
    assert!(entry.names.contains(&"cdq"), "la fiche couvre la famille");
    assert!(
        entry.body.fr.contains("idiv"),
        "et dit à quoi elle sert : {}",
        entry.body.fr
    );

    // Un mot qui n'est pas une instruction : on le dit, sans fiche.
    app.source = "_start:\n    truc\n".to_owned();
    app.cursor_byte = 14;
    app.dispatch(Action::InstructionHelp);
    let (word, entry) = app.help.clone().expect("une demande d'aide");
    assert_eq!(word, "truc");
    assert!(entry.is_none());
}

/// L'autocomplétion et l'aide lisent la même liste : une instruction
/// proposée est une instruction qu'on sait expliquer.
#[test]
fn everything_proposed_can_be_explained() {
    for name in completion::mnemonics() {
        assert!(
            namsdec_doc::lookup(name).is_some(),
            "« {name} » est proposé sans fiche"
        );
    }
    assert!(completion::mnemonics().len() > 400);
}
/// Les morceaux de code d'une fiche s'écrivent en chasse fixe, et
/// l'accent grave qui les marque ne s'affiche pas.
#[test]
fn code_inside_an_explanation_is_written_as_code() {
    let ctx = egui::Context::default();
    let mut job = None;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        job = Some(prose(
            ui,
            namsdec_theme::by_id("vscode-dark"),
            "préparer `rdx` avant",
        ));
    });
    output.textures_delta.clear();
    let job = job.expect("un texte mis en forme");
    assert_eq!(job.text, "préparer rdx avant", "pas d'accent grave affiché");
    assert_eq!(job.sections.len(), 3);
    let fonts: Vec<_> = job
        .sections
        .iter()
        .map(|s| s.format.font_id.family.clone())
        .collect();
    assert_eq!(
        fonts[0], fonts[2],
        "le texte ordinaire est d'une seule police"
    );
    assert_ne!(fonts[0], fonts[1], "le code est dans une autre");
}
/// Ouvrir un vrai ELF : ses sections vont à leur adresse, et le
/// listing commence à son point d'entrée — pas sur son en-tête, qui
/// n'est pas du code et ne se lit pas comme tel.
#[test]
fn an_elf_is_opened_where_it_says_to_be() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Un vrai binaire, fabriqué par la vraie chaîne d'outils, puis
    // ouvert comme n'importe quel fichier trouvé sur le disque.
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let asm = dir.path().join("p.asm");
    std::fs::write(
        &asm,
        "section .data\n    msg db \"salut\", 0\n\
         section .text\n    global _start\n_start:\n\
         \x20   mov rax, 60\n    xor rdi, rdi\n    syscall\n",
    )
    .unwrap();
    let object = dir.path().join("p.o");
    let program = dir.path().join("p");
    assert!(
        std::process::Command::new("nasm")
            .args(["-f", "elf64", "-o"])
            .arg(&object)
            .arg(&asm)
            .status()
            .expect("nasm")
            .success()
    );
    assert!(
        std::process::Command::new("ld")
            .arg("-o")
            .arg(&program)
            .arg(&object)
            .status()
            .expect("ld")
            .success()
    );

    let mut app = NamsdecApp::default();
    app.load_binary(std::fs::read(&program).unwrap());

    let state = app.assemble_state.as_ref().expect("un binaire chargé");
    assert!(
        !state.output.sections.is_empty(),
        "les sections de l'ELF sont connues"
    );
    assert_eq!(
        state.disasm.first().map(|i| i.address),
        Some(state.output.entry),
        "le désassemblage commence au code, à l'entrée déclarée"
    );
    // Et la première ligne du listing est une instruction, pas les
    // octets « 7f 45 4c 46 » de l'en-tête lus comme telle.
    let first = app.source.lines().next().unwrap_or_default();
    assert!(first.contains("mov"), "listing : {first}");
}
/// Un message de statut long ne passe pas sous les compartiments de
/// droite : il est tronqué avant de les atteindre. Sans cela, le nom
/// d'un binaire chargé s'imprimait par-dessus « Lig 1, Col 1 ».
#[test]
fn a_long_status_message_does_not_run_under_the_right_hand_side() {
    let mut app = NamsdecApp {
        status: Status::Error("erreur ".repeat(40)),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
        )),
        ..Default::default()
    };
    let mut message = None;
    let mut nasm = None;
    for _ in 0..2 {
        message = None;
        nasm = None;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                let content = text.galley.text();
                let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                if content.contains("erreur erreur") {
                    message = Some(rect);
                } else if content.starts_with("NASM") || content.contains("nasm") {
                    nasm = Some(rect);
                }
            }
        }
        output.textures_delta.clear();
    }

    let message = message.expect("le message de statut est peint");
    let nasm = nasm.expect("le compartiment de droite est peint");
    assert!(
        message.max.x <= nasm.min.x,
        "le message ({message:?}) mord sur la droite ({nasm:?})"
    );
}
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
            egui::vec2(1280.0, 820.0),
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

/// Le dossier de données est une variable d'environnement, donc partagée
/// par tous les tests du binaire. Ceux qui y touchent prennent ce verrou :
/// sans lui, deux tests en parallèle se volent le dossier de l'autre.
static DATA_HOME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Détourne le dossier de données vers `dir` le temps du test, pour que
/// rien ne soit écrit chez l'utilisateur.
fn with_data_home<T>(dir: &std::path::Path, body: impl FnOnce() -> T) -> T {
    let _guard = DATA_HOME.lock().unwrap_or_else(|e| e.into_inner());
    let previous = std::env::var_os("XDG_DATA_HOME");
    unsafe { std::env::set_var("XDG_DATA_HOME", dir) };
    let result = body();
    match previous {
        Some(value) => unsafe { std::env::set_var("XDG_DATA_HOME", value) },
        None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
    }
    result
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
    let mut app = NamsdecApp::default();
    with_data_home(&dir.path().join("data"), || app.install_exercises(&path));
    assert!(
        app.plugin_note.as_ref().is_some_and(|n| !n.ok),
        "elle doit être refusée"
    );
    assert_eq!(app.exercise_packs.len(), 1);
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

/// La fenêtre d'aide s'ouvre par le menu Aide comme par F1, et montre la
/// fiche — pas seulement son titre.
#[test]
fn the_help_window_shows_the_entry_it_was_asked_for() {
    let mut app = NamsdecApp {
        source: "_start:\n    movzx eax, byte [buf]\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = 12; // au milieu de « movzx »
    app.execute_palette_command(
        &egui::Context::default(),
        PaletteCommand::Do(Action::InstructionHelp),
    );

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
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

    assert!(
        painted.contains("Élargir une valeur"),
        "le titre de la fiche"
    );
    assert!(painted.contains("movsx"), "les mnémoniques de la famille");
    assert!(
        painted.contains("Indicateurs écrits"),
        "et ce qu'elle laisse dans les indicateurs"
    );
    // La console, elle, n'a pas gagné d'onglet : ses deux onglets se
    // suivent. (« Aide » tout court reste le menu, dans la barre.)
    assert!(painted.contains("|Console|Diagnostics|"), "{painted}");
}

/// Tout programme livré avec namsdec — celui qui s'ouvre au lancement,
/// ceux des exercices — doit être un *programme*, pas un extrait : il
/// déclare son point d'entrée, et ni nasm ni l'éditeur de liens n'ont
/// rien à en dire.
///
/// Sans `global _start`, `ld` choisit une entrée par défaut en
/// avertissant, l'avertissement était jeté, et l'application démarrait en
/// silence à un endroit que personne n'avait choisi.
#[test]
fn every_program_we_ship_is_a_complete_program() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut programs = vec![("le programme d'accueil", DEFAULT_SOURCE.to_owned())];
    for exercise in namsdec_exercises::builtin_pack().exercises {
        programs.push((
            Box::leak(exercise.id.clone().into_boxed_str()) as &str,
            exercise.starter_code,
        ));
    }

    for (name, source) in programs {
        assert!(
            source.contains("global _start"),
            "{name} ne déclare pas son point d'entrée"
        );
        let out = namsdec_asm::assemble(&source, BASE_ADDR)
            .unwrap_or_else(|e| panic!("{name} ne s'assemble pas : {e}"));
        assert!(out.diagnostics.is_empty(), "{name} : {:?}", out.diagnostics);
        // Et il rend la main : soit il s'arrête, soit il bute sur une
        // faute — un exercice peut très bien être fait pour en montrer
        // une. Ce qu'on refuse, c'est le programme qui tourne sans fin.
        let mut app = NamsdecApp {
            source: source.clone(),
            ..Default::default()
        };
        app.do_assemble();
        for _ in 0..1000 {
            app.do_step();
        }
        let arrêté = app.emulator.as_ref().is_some_and(|e| e.cpu().halted)
            || matches!(app.status, Status::Error(_));
        assert!(arrêté, "{name} ne rend jamais la main");
    }
}
/// Les quatre-vingt-onze exemples des fiches d'instructions s'assemblent,
/// et namsdec les exécute.
///
/// Un exemple est un extrait — deux lignes qui montrent la forme d'une
/// instruction —, on l'entoure donc de ce qu'il nomme : des données, et
/// les étiquettes vers lesquelles il saute. Ce qui reste vérifié est ce
/// qui compte : l'instruction existe, ses opérandes sont du bon type et
/// dans le bon ordre. Deux exemples écrits à la main s'y sont fait
/// prendre, qui donnaient des étiquettes de données à `mulx` et `andn`
/// là où il faut des registres.
///
/// L'exécution, elle, ne cherche qu'une chose : aucune instruction
/// documentée ne doit être refusée par l'interpréteur. Décrire ce qu'on
/// ne sait pas exécuter serait une promesse en l'air.
#[test]
fn every_documented_example_assembles_and_runs() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let prelude = "default rel\nglobal _start\n\nsection .data\n\
        \x20   msg db \"salut\", 10\n    len equ $ - msg\n    x dq 1.5\n    y dq 2.5\n\
        \x20   a dq 1.0\n    b dq 2.0\n    n dd 7\n    haut dq 0\n    bas dq 0\n\
        \x20   masque dq 0\n    résultat dq 0\n    entier dq 0\n    compteur dq 0\n\
        \x20   table db 1, 2, 3, 4\n    drapeaux times 32 db 0\n    verrou dq 0\n\
        \x20   attendu dq 0\n    facteur dd 1.0\n    masque_signe dq 0\n    valeur dq 0\n\
        \x20   align 32\n    huit_valeurs dd 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0\n\
        \x20   vecteur times 16 db 0\n    non_aligné times 32 db 0\n\
        \x20   valeurs dq 1.0\n    sortie times 32 db 0\n    buf times 64 db 0\n\
        \x20   texte db \"abc\", 0\n\nsection .text\n_start:\n";
    let epilogue = "\n    hlt\nplus_grand:\nfin:\nnul:\nfini:\nidentiques:\n\
        non_comparable:\nétait_déjà_posé:\n    hlt\n";

    // Trois exemples ne peuvent pas s'assembler seuls, et c'est ce
    // qu'ils montrent : deux nomment un fichier qui n'existe que chez
    // celui qui l'écrit, le troisième provoque une erreur exprès.
    const SANS_FICHIER: [&str; 3] = ["incbin", "%include", "%error"];

    let mut fautifs = Vec::new();
    for entry in namsdec_doc::ENTRIES {
        if SANS_FICHIER.contains(&entry.names[0]) {
            continue;
        }
        let source = format!("{prelude}{}{epilogue}", entry.example);
        if let Err(e) = namsdec_asm::assemble(&source, BASE_ADDR) {
            let détail = match &e {
                namsdec_asm::AsmError::Failed { diagnostics } => diagnostics
                    .iter()
                    .map(|d| d.message.clone())
                    .collect::<Vec<_>>()
                    .join(" ; "),
                autre => format!("{autre}"),
            };
            fautifs.push(format!("{} : {détail}", entry.names[0]));
            continue;
        }

        // Et l'interpréteur les exécute. Une fiche qui décrit une
        // instruction que le moteur refuse serait une promesse en l'air ;
        // les autres erreurs — une adresse hors bornes, une division par
        // zéro — viennent des données inventées ici et ne prouvent rien.
        let mut app = NamsdecApp {
            source,
            ..Default::default()
        };
        app.do_assemble();
        if let Some(emu) = app.emulator.as_mut() {
            emu.cpu_mut().regs.set64(Reg64::Rcx, 4);
            emu.cpu_mut().regs.set64(Reg64::Rbx, 5);
        }
        for _ in 0..60 {
            let Some(emu) = app.emulator.as_mut() else {
                break;
            };
            match emu.step() {
                Ok(namsdec_emu::StepResult::Halted) => break,
                Ok(_) => {}
                Err(e) => {
                    let texte = format!("{e:?}");
                    if texte.contains("Unsupported") {
                        fautifs.push(format!("{} : {texte}", entry.names[0]));
                    }
                    break;
                }
            }
        }
    }
    assert!(fautifs.is_empty(), "{}", fautifs.join("\n"));
}

/// Les onglets du panneau restent atteignables, quoi qu'on déplie.
///
/// Le bandeau, les registres vectoriels et la pile x87 étaient bornés
/// chacun de son côté : chacun tenait dans sa moitié de panneau, mais
/// leurs hauteurs s'ajoutaient et la rangée d'onglets passait sous le
/// bord — sans barre de défilement pour aller la rechercher.
#[test]
fn nothing_unfolded_can_push_the_tabs_out_of_the_panel() {
    let mut app = NamsdecApp {
        // Tout ouvert, et un programme chargé pour que chaque section ait
        // quelque chose à montrer.
        cpu_state_open: true,
        sse_open: true,
        x87_open: true,
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        app.do_assemble();
    }

    // Une fenêtre courte, celle qui pose problème.
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 700.0),
        )),
        ..Default::default()
    };
    let mut painted = String::new();
    for _ in 0..3 {
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
    for onglet in [s.tab_stack, s.tab_memory, s.tab_heap] {
        assert!(
            painted.contains(onglet),
            "l'onglet « {onglet} » est hors du panneau : {painted}"
        );
    }
}
/// Une barre de défilement quand il y a de quoi défiler, et pas avant.
///
/// Les deux moitiés comptent autant : une barre permanente sous un
/// fichier de quatre lignes gêne, et une barre qui n'apparaît qu'au
/// survol — ce que fait egui par défaut, avec des barres flottantes —
/// laisse croire qu'il n'y a rien de plus bas. Elles prennent leur place
/// et ne s'affichent que si elles servent.
#[test]
fn a_scroll_bar_shows_up_when_there_is_something_to_scroll_and_not_before() {
    let barres_de = |source: String, ouvert: bool| -> Vec<(f32, f32)> {
        let mut app = NamsdecApp {
            source,
            cpu_state_open: ouvert,
            sse_open: ouvert,
            x87_open: ouvert,
            ..Default::default()
        };
        if ouvert && namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
            app.do_assemble();
        }
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 820.0),
            )),
            ..Default::default()
        };
        let mut barres = Vec::new();
        for _ in 0..3 {
            barres.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Rect(r) = &clipped.shape {
                    // Une barre de défilement : étroite et haute.
                    if r.rect.width() < 20.0 && r.rect.height() > 60.0 {
                        barres.push((r.rect.min.x.round(), r.rect.width().round()));
                    }
                }
            }
            output.textures_delta.clear();
        }
        barres
    };

    let court = "global _start\nsection .text\n_start:\n    hlt\n".to_owned();
    assert!(
        barres_de(court.clone(), false).is_empty(),
        "quatre lignes ne méritent pas de barre"
    );

    let long = format!(
        "global _start\nsection .text\n_start:\n{}    hlt\n",
        "    nop\n".repeat(80)
    );
    assert!(
        !barres_de(long, false).is_empty(),
        "quatre-vingts lignes, elles, en méritent une"
    );

    // Et dans le panneau latéral, les vectoriels dépliés : la barre doit
    // être là, sans quoi rien ne dit que les onglets sont plus bas.
    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        let barres = barres_de(court, true);
        assert!(
            barres.iter().any(|(x, _)| *x > 900.0),
            "pas de barre dans le panneau d'état : {barres:?}"
        );
    }
}

/// Un programme ne peut pas écrire dans son propre code, ni exécuter ses
/// données, ni toucher ce qui n'est projeté nulle part — et chaque refus
/// se lit en français, avec l'adresse et la zone en cause.
///
/// C'est la leçon que namsdec ne savait pas donner : « pourquoi ça
/// segfault » restait invisible, le programme écrivait dans `.text` et
/// continuait comme si de rien n'était.
#[test]
fn a_program_cannot_write_its_own_code_or_run_its_data() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let faute = |source: &str| -> String {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        for _ in 0..40 {
            app.do_step();
            if let Status::Error(message) = &app.status {
                return message.clone();
            }
        }
        String::new()
    };

    // Écrire dans le code.
    let message =
        faute("global _start\nsection .text\n_start:\n    mov byte [_start], 0x90\n    hlt\n");
    assert!(
        message.contains("interdit d'écrire") && message.contains(".text"),
        "obtenu : {message}"
    );

    // Sauter dans les données.
    let message = faute(
        "global _start\nsection .data\n    tampon times 8 db 0\n\
         section .text\n_start:\n    jmp tampon\n",
    );
    assert!(
        message.contains("interdit d'exécuter") && message.contains(".data"),
        "obtenu : {message}"
    );

    // Écrire là où il n'y a rien : le défaut de segmentation, dit en
    // clair.
    let message = faute(
        "global _start\nsection .text\n_start:\n    mov rax, 0x407000\n\
         \x20   mov qword [rax], 1\n    hlt\n",
    );
    assert!(message.contains("rien n'est projeté"), "obtenu : {message}");

    // Et un programme honnête tourne toujours : il écrit dans ses
    // données, lit son code, et s'arrête.
    let message = faute(
        "global _start\nsection .data\n    n dq 0\nsection .text\n_start:\n\
         \x20   mov qword [n], 42\n    mov rax, [n]\n    hlt\n",
    );
    assert!(
        message.is_empty(),
        "un programme correct ne doit rien déclencher : {message}"
    );
}

/// Le tas n'existe que jusqu'à la rupture : au-delà, il n'y a rien, même
/// si l'adresse est dans la fenêtre. C'est ce qui apprend à demander la
/// place avant de s'en servir.
#[test]
fn the_heap_exists_only_up_to_the_break() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n\
             \x20   mov rax, 12\n    xor rdi, rdi\n    syscall\n\
             \x20   mov r12, rax\n\
             \x20   mov qword [r12], 1      ; rien n'est encore alloué\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    for _ in 0..20 {
        app.do_step();
    }
    let Status::Error(message) = &app.status else {
        panic!(
            "écrire avant d'avoir demandé doit échouer : {:?}",
            app.status
        );
    };
    assert!(message.contains("rien n'est projeté"), "obtenu : {message}");

    // Avec le brk qui va bien, la même écriture passe.
    app.source = "global _start\nsection .text\n_start:\n\
         \x20   mov rax, 12\n    xor rdi, rdi\n    syscall\n    mov r12, rax\n\
         \x20   lea rdi, [rax + 4096]\n    mov rax, 12\n    syscall\n\
         \x20   mov qword [r12], 1\n    hlt\n"
        .to_owned();
    app.do_assemble();
    for _ in 0..20 {
        app.do_step();
    }
    assert!(
        matches!(app.status, Status::Halted),
        "obtenu : {:?}",
        app.status
    );
}

/// L'onglet Mémoire dit à quelle zone appartient l'adresse regardée, et
/// avec quels droits : on lit la protection avant de se la voir opposer.
#[test]
fn the_memory_tab_names_the_region_and_its_rights() {
    let s = strings(Lang::Fr);
    let mut mem = namsdec_core::Memory::new(BASE_ADDR, 0x4000);
    // Sans zone déclarée, rien à dire : un binaire brut n'a pas de
    // sections, et inventer des droits serait mentir.
    assert_eq!(region_line(s, &mem, BASE_ADDR), "");

    mem.map(
        BASE_ADDR..BASE_ADDR + 0x1000,
        namsdec_core::Perms::TEXT,
        ".text",
    );
    mem.map(
        BASE_ADDR + 0x1000..BASE_ADDR + 0x2000,
        namsdec_core::Perms::DATA,
        ".data",
    );
    assert_eq!(
        region_line(s, &mem, BASE_ADDR),
        ".text · lecture, exécution"
    );
    assert_eq!(
        region_line(s, &mem, BASE_ADDR + 0x1000),
        ".data · lecture, écriture"
    );
    assert_eq!(
        region_line(s, &mem, BASE_ADDR + 0x3000),
        "rien n'est projeté ici"
    );
}

/// L'onglet Pile montre la convention d'appel — par où passent les
/// arguments, ce qu'une routine doit rendre — et nomme celles qui ne l'ont
/// pas respectée.
#[test]
fn the_stack_tab_teaches_the_calling_convention_and_checks_it() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Stack,
        source: "\
global _start

section .text
_start:
    mov rdi, 1
    mov rsi, 2
    mov rbx, 7
    call malpolie
    hlt

malpolie:
    mov rbx, 42             ; empruntée, jamais rendue
    ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    for _ in 0..20 {
        app.do_step();
    }

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
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

    // L'aide-mémoire : les six registres d'arguments, dans l'ordre.
    assert!(painted.contains("RDI|0x1|RSI|0x2"), "{painted}");
    assert!(painted.contains("Convention d'appel"), "{painted}");
    // Et la vérification, sur ce programme-ci.
    assert!(painted.contains("Entorses relevées"), "{painted}");
    assert!(painted.contains("n'a pas rendu"), "{painted}");
    assert!(painted.contains("RBX"), "{painted}");
    // Et d'où partait l'appel, en ligne de source — pas en hexadécimal.
    assert!(painted.contains("appelée depuis ligne 8"), "{painted}");
}

/// Une série s'écarte sans se désinstaller : le fichier reste, son
/// contenu cesse d'être proposé. Y compris celle qui est livrée avec
/// l'application, qui ne s'enlève pas mais se range.
#[test]
fn a_pack_can_be_set_aside_without_being_uninstalled() {
    let mut app = NamsdecApp {
        exercises: true,
        show_exercises: true,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
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

/// Le parcours livré est un parcours : huit étapes dans l'ordre où l'on
/// apprend, chacune avec son but écrit dans le programme.
#[test]
fn the_built_in_pack_is_a_course_in_order() {
    let pack = namsdec_exercises::builtin_pack();
    let ids: Vec<&str> = pack.exercises.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "registres",
            "boucle",
            "comparer",
            "memoire",
            "pile",
            "appels",
            "sortie",
            "protection"
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

/// Surveiller un registre : l'exécution s'arrête quand il change, sans
/// qu'on ait eu à deviner quelle instruction le ferait.
///
/// Un point d'arrêt répond à « arrête-toi *là* ». La question qu'on se
/// pose en apprenant est « qui écrit dans RAX ? », et elle n'a pas de
/// point d'arrêt possible : on ignore justement où.
#[test]
fn watching_a_register_stops_the_run_when_it_changes() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n\
             \x20   mov rcx, 3\n    mov rbx, 7\n    mov rax, 42\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    // Surveiller RAX, comme un clic sur son nom dans le bandeau.
    app.watched_registers.insert("RAX");
    app.sync_watchpoints();

    app.do_run();
    let ctx = egui::Context::default();
    for _ in 0..10 {
        app.tick_run(&ctx);
    }

    match app.status {
        Status::RegisterChanged { register, old, new } => {
            assert_eq!(register, "RAX");
            assert_eq!((old, new), (0, 42));
        }
        autre => panic!("l'exécution devait s'arrêter sur RAX : {autre:?}"),
    }
    // Et elle s'est arrêtée *à* l'instruction fautive, pas à la fin.
    assert!(
        app.emulator.as_ref().is_some_and(|e| !e.cpu().halted),
        "le programme n'est pas allé jusqu'au hlt"
    );

    // Ne plus surveiller : le programme va au bout.
    app.watched_registers.remove("RAX");
    app.do_assemble();
    app.sync_watchpoints();
    app.do_run();
    for _ in 0..10 {
        app.tick_run(&ctx);
    }
    assert!(matches!(app.status, Status::Halted), "{:?}", app.status);
}

/// F1 documente aussi le langage de NASM, pas seulement le processeur :
/// `times`, `equ`, `$`, `%macro` sont ce sur quoi on bute en écrivant, et
/// ils n'ont rien à voir avec ce que le processeur exécute.
#[test]
fn the_help_covers_the_assembler_language_too() {
    for (mot, attendu) in [
        ("times", "Répéter"),
        ("equ", "Donner un nom à une valeur"),
        ("db", "Écrire des octets"),
        ("resb", "Réserver sans écrire"),
        ("$", "Où en est l'assembleur"),
        ("%macro", "Écrire une fois, poser partout"),
        ("%define", "Remplacer avant d'assembler"),
        ("qword", "Dire la taille"),
        ("global", "Ce qui sort, ce qui entre"),
    ] {
        let entry =
            namsdec_doc::lookup(mot).unwrap_or_else(|| panic!("« {mot} » n'est pas documenté"));
        assert_eq!(entry.title.fr, attendu, "pour « {mot} »");
        assert_ne!(
            entry.kind,
            namsdec_doc::Kind::Instruction,
            "« {mot} » n'est pas une instruction"
        );
    }

    // Et l'autocomplétion, elle, ne propose que des instructions : une
    // directive n'a rien à faire dans une liste de mnémoniques.
    let proposés = completion::mnemonics();
    assert!(proposés.contains(&"mov"));
    for directive in ["times", "equ", "db", "%macro", "qword"] {
        assert!(
            !proposés.contains(&directive),
            "« {directive} » est proposé comme une instruction"
        );
    }
}

/// L'onglet Octets montre ce que l'assembleur a produit : une ligne par
/// instruction, son adresse, sa longueur, ses octets — et la ligne source
/// d'où elle vient.
///
/// C'est là qu'on voit, sans qu'on ait à l'expliquer, ce que coûte ce
/// qu'on écrit — et ce que nasm choisit parfois pour vous : `mov rax, 1`
/// devient `mov eax, 1` et tient en cinq octets, puisque le haut de RAX
/// sera mis à zéro de toute façon. Il faut un négatif pour voir la forme
/// à sept octets, et une grande constante pour celle à dix.
#[test]
fn the_bytes_tab_shows_what_each_line_produced() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Bytes,
        source: "global _start\nsection .text\n_start:\n\
             \x20   mov rax, 1\n    mov rax, -1\n\
             \x20   mov rax, 0x1122334455667788\n    xor eax, eax\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
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

    // Les octets eux-mêmes.
    assert!(
        painted.contains("b8 01 00 00 00"),
        "mov rax, 1 tient en cinq octets : {painted}"
    );
    assert!(painted.contains("48 c7 c0 ff ff ff ff"), "{painted}");
    assert!(
        painted.contains("48 b8 88 77 66 55 44 33 22 11"),
        "{painted}"
    );
    assert!(painted.contains("31 c0"), "{painted}");
    // Les longueurs, qui sont tout l'intérêt : 5, 7, 10, 2.
    for longueur in [" 5|", " 7|", "10|", " 2|"] {
        assert!(
            painted.contains(longueur),
            "longueur {longueur} : {painted}"
        );
    }
    // Et le lien avec la source : la ligne de chaque instruction, collée
    // à son adresse.
    assert!(painted.contains("  4 00400000"), "{painted}");
}

/// Un message de nasm arrive tel quel, et une phrase dessous dit ce qu'il
/// veut dire. Le brut reste — c'est lui qu'on colle dans une recherche.
#[test]
fn a_nasm_error_comes_with_its_explanation() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    let mut app = NamsdecApp {
        // Deux tailles qui ne se rencontrent pas : l'erreur qu'on
        // ramasse le jour où l'on mélange RAX et EBX.
        source: "global _start\nsection .text\n_start:\n    mov rax, ebx\n    hlt\n".to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    assert!(
        matches!(app.status, Status::AssembleFailed),
        "{:?}",
        app.status
    );
    let diagnostic = app.diagnostics.first().expect("un diagnostic");
    assert!(
        diagnostic
            .message
            .contains("invalid combination of opcode and operands"),
        "{}",
        diagnostic.message
    );

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
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

    assert!(
        painted.contains("invalid combination of opcode and operands"),
        "le message brut reste : {painted}"
    );
    assert!(
        painted.contains("mov rax, ebx"),
        "l'explication doit nommer le cas : {painted}"
    );
}

/// Cliquer un diagnostic pose le curseur sur sa ligne : lire le message
/// et retrouver l'endroit ne sont pas deux gestes.
#[test]
fn clicking_a_diagnostic_moves_the_cursor_to_its_line() {
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    hlt\n".to_owned(),
        ..Default::default()
    };
    // Comme si l'on avait cliqué le diagnostic de la ligne 3.
    app.goto_line = Some(3);

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
        )),
        ..Default::default()
    };
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
    }

    assert!(app.goto_line.is_none(), "la demande est consommée");
    let state = egui::TextEdit::load_state(&ctx, editor_id()).expect("l'éditeur a un état");
    let curseur = state
        .cursor
        .char_range()
        .expect("un curseur")
        .primary
        .index
        .0;
    // « global _start\n » = 14 caractères, « section .text\n » = 14.
    assert_eq!(curseur, 28, "le curseur est au début de la ligne 3");
}

/// Les octets qui viennent de changer ressortent dans le vidage, comme un
/// registre dans le bandeau : c'est ce qui répond à « qu'est-ce que cette
/// instruction a écrit ? » sans relire seize octets un à un.
#[test]
fn the_memory_dump_highlights_what_just_changed() {
    let ctx = egui::Context::default();
    let theme = namsdec_theme::by_id("vscode-dark");
    let mut job = None;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let avant = [0u8; 8];
        let maintenant = [0u8, 0, 0x42, 0, 0, 0, 0, 0];
        job = Some(hexdump_job(ui, theme, &maintenant, 0x400000, Some(&avant)));
    });
    output.textures_delta.clear();
    let job = job.expect("un vidage");

    // Le troisième octet, et lui seul, porte la couleur du changement —
    // deux fois : en hexadécimal et dans la colonne de texte.
    let changés: Vec<&str> = job
        .sections
        .iter()
        .filter(|section| section.format.color == theme.changed)
        .map(|section| &job.text[section.byte_range.start.0..section.byte_range.end.0])
        .collect();
    assert_eq!(changés, ["42 ", "B"], "obtenu {changés:?}");

    // Sans point de comparaison — au chargement, avant le premier pas —
    // rien n'est mis en avant.
    let mut job = None;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        job = Some(hexdump_job(ui, theme, &[0u8, 0, 0x42], 0x400000, None));
    });
    output.textures_delta.clear();
    assert!(
        job.expect("un vidage")
            .sections
            .iter()
            .all(|section| section.format.color != theme.changed)
    );
}

/// « Exécuter jusqu'ici » : le curseur sur une ligne, et l'on y va — sans
/// poser un point d'arrêt puis penser à le retirer.
#[test]
fn run_to_cursor_stops_there_and_leaves_no_breakpoint_behind() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n\
             \x20   mov rax, 1\n    mov rbx, 2\n\
             \x20   ; un commentaire, qui ne produit rien\n\
             \x20   mov rcx, 3\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    // Le curseur sur la ligne du commentaire : « aller là » veut dire
    // « aller à ce qui suit », sinon la commande ne ferait rien.
    app.cursor_pos = (6, 1);
    app.dispatch(Action::RunToCursor);
    let ctx = egui::Context::default();
    for _ in 0..10 {
        app.tick_run(&ctx);
    }

    let emu = app.emulator.as_ref().expect("un émulateur");
    assert_eq!(
        emu.cpu().regs.get64(Reg64::Rbx),
        2,
        "les deux premières instructions ont tourné"
    );
    assert_eq!(
        emu.cpu().regs.get64(Reg64::Rcx),
        0,
        "et l'on s'est arrêté avant la troisième"
    );
    assert!(
        matches!(app.status, Status::Breakpoint(_)),
        "{:?}",
        app.status
    );

    // Le point d'arrêt du voyage n'a pas survécu : relancer va au bout.
    assert!(app.run_to.is_none());
    assert!(
        app.breakpoint_lines.is_empty(),
        "aucun point d'arrêt n'appartient à l'utilisateur"
    );
    app.do_run();
    for _ in 0..10 {
        app.tick_run(&ctx);
    }
    assert!(matches!(app.status, Status::Halted), "{:?}", app.status);
}

/// La ligne fautive se voit dans l'éditeur, là où l'on écrit — pas
/// seulement dans la console.
#[test]
fn the_offending_line_is_marked_in_the_editor() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    let attendu = namsdec_theme::by_id("vscode-dark").bad.gamma_multiply(0.18);
    let bandes = |source: &str| -> Vec<(f32, egui::Color32)> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 820.0),
            )),
            ..Default::default()
        };
        let mut trouvées = Vec::new();
        for _ in 0..2 {
            trouvées.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Rect(r) = &clipped.shape {
                    // Le bandeau d'une ligne fautive : sa teinte le
                    // distingue de tout le reste.
                    if r.fill == attendu {
                        trouvées.push((r.rect.min.y, r.fill));
                    }
                }
            }
            output.textures_delta.clear();
        }
        trouvées
    };

    let propre = "global _start\nsection .text\n_start:\n    hlt\n";
    assert!(
        bandes(propre).is_empty(),
        "un programme correct ne porte aucune marque"
    );

    let fautif = "global _start\nsection .text\n_start:\n    mov rax, ebx\n    hlt\n";
    let marquées = bandes(fautif);
    assert_eq!(marquées.len(), 1, "une seule ligne fautive : {marquées:?}");
}

/// Survoler une instruction dans l'éditeur en donne la fiche, sans rien
/// presser — et se tait quand les infobulles sont éteintes.
#[test]
fn hovering_an_instruction_explains_it() {
    let source = "global _start\nsection .text\n_start:\n    movzx eax, byte [rsi]\n";
    let painted = |tooltips: bool| -> String {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            show_tooltips: tooltips,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
        let mut texts = Vec::new();
        let mut pointer = None;
        for _ in 0..4 {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events: pointer.map(egui::Event::PointerMoved).into_iter().collect(),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| app.draw(ui));
            texts.clear();
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &clipped.shape {
                    texts.push((text.pos, text.galley.clone()));
                }
            }
            // Le pointeur se pose sur « movzx » une fois l'éditeur dessiné :
            // sa place se lit dans le galley du code, pas dans des
            // coordonnées devinées.
            if pointer.is_none() {
                let (pos, galley) = texts
                    .iter()
                    .find(|(_, g)| g.text().starts_with("global _start"))
                    .expect("le code doit être dessiné");
                let row = galley.rows.get(3).expect("quatre lignes de code");
                pointer = Some(egui::pos2(
                    pos.x + row.rect().left() + 30.0,
                    pos.y + row.rect().center().y,
                ));
            }
            output.textures_delta.clear();
        }
        texts
            .iter()
            .map(|(_, g)| g.text())
            .collect::<Vec<_>>()
            .join("|")
    };

    let avec = painted(true);
    assert!(
        avec.contains("Élargir une valeur"),
        "la fiche de movzx doit apparaître : {avec}"
    );
    assert!(
        !painted(false).contains("Élargir une valeur"),
        "infobulles éteintes : rien ne doit apparaître"
    );
}

/// Le bandeau « Prochaine instruction » fait le calcul d'adresse que le
/// débutant fait de tête : `[tampon + rcx]`, c'est *cette* adresse-là, et
/// il y a *cette* valeur dedans.
#[test]
fn the_next_instruction_resolves_its_operands() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
tampon: db 0x11, 0x22, 0x33, 0x44
section .text
_start:
mov rcx, 2
mov al, [tampon + rcx]
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    // Arrêté *sur* le `mov al, [tampon+rcx]`, pas encore exécuté : c'est
    // là que la résolution sert.
    app.do_step();

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
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

    assert!(painted.contains("Prochaine instruction"), "{painted}");
    // Le désassembleur ne connaît pas les étiquettes — il montre
    // l'adresse que nasm a mise à leur place.
    assert!(
        painted.contains("mov al, [rcx+401000h]"),
        "l'instruction elle-même doit être montrée : {painted}"
    );
    // L'adresse de `tampon` plus deux, et l'octet qui s'y trouve.
    let emulator = app.emulator.as_ref().unwrap();
    let rcx = emulator.cpu().regs.get64(namsdec_core::Reg64::Rcx);
    assert_eq!(rcx, 2, "le premier pas charge RCX");
    assert!(
        painted.contains("→ 0x33"),
        "le contenu de l'adresse calculée doit apparaître : {painted}"
    );
    // Et la fiche de l'instruction, en une ligne.
    assert!(painted.contains("Copier"), "{painted}");
}

/// L'état du processeur tient dans le panneau, sur toute sa largeur, et
/// ne demande jamais de barre de défilement.
///
/// C'est l'affichage qu'on regarde entre deux pas : devoir le faire
/// défiler pour voir R15 ou ZF coupe la lecture. Les registres sont donc
/// posés sur autant de colonnes que le panneau en tient — deux à la
/// largeur par défaut — et hors de toute zone défilante.
#[test]
fn the_cpu_state_fills_the_panel_without_a_scroll_bar() {
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
        )),
        ..Default::default()
    };
    let mut registres: Vec<egui::Pos2> = Vec::new();
    let mut onglet_y = f32::MAX;
    let mut barres: Vec<egui::Rect> = Vec::new();
    for _ in 0..4 {
        registres.clear();
        barres.clear();
        onglet_y = f32::MAX;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => match text.galley.text() {
                    "RAX" | "R15" | "RIP" => registres.push(text.pos),
                    t if t == strings(Lang::Fr).tab_stack => onglet_y = onglet_y.min(text.pos.y),
                    _ => {}
                },
                egui::epaint::Shape::Rect(r) if r.rect.width() < 20.0 && r.rect.height() > 40.0 => {
                    barres.push(r.rect);
                }
                _ => {}
            }
        }
        output.textures_delta.clear();
    }

    assert_eq!(registres.len(), 3, "RAX, R15 et RIP doivent être peints");
    // Deux colonnes : RAX et R15 ne sont pas sur la même ligne, mais RAX
    // n'est pas seul sur la sienne — la grille occupe la largeur.
    let colonnes: std::collections::BTreeSet<i32> =
        registres.iter().map(|p| p.x.round() as i32).collect();
    assert_eq!(
        colonnes.len(),
        2,
        "les registres doivent tenir sur deux colonnes : {registres:?}"
    );
    // La seconde colonne s'appuie sur le bord droit du panneau, à
    // quelques pixels près : la grille ne se tasse pas à gauche.
    let droite = *colonnes.iter().next_back().unwrap() as f32;
    assert!(
        droite > 1280.0 - SIDEBAR_WIDTH / 2.0,
        "la seconde colonne doit occuper la moitié droite du panneau : {colonnes:?}"
    );
    // Et aucune barre de défilement au-dessus des onglets, là où vit
    // l'état du processeur.
    let fautives: Vec<_> = barres
        .iter()
        .filter(|r| r.min.x > 1280.0 - SIDEBAR_WIDTH && r.max.y < onglet_y)
        .collect();
    assert!(
        fautives.is_empty(),
        "l'état du processeur ne doit pas défiler : {fautives:?}"
    );
}

/// La palette de commandes montre tout ce qu'elle a, sans barre de
/// défilement, tant que l'écran le permet — et ses lignes occupent
/// toute sa largeur.
///
/// Elle avait une hauteur figée de 340 px : la liste défilait dans une
/// fenêtre à moitié vide, et sa barre tombait au milieu parce que la
/// zone se serrait à gauche.
#[test]
fn the_command_palette_shows_everything_it_can_without_scrolling() {
    let mut app = NamsdecApp {
        command_palette: Some(CommandPalette::default()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1000.0),
        )),
        ..Default::default()
    };

    let mut barres: Vec<egui::Rect> = Vec::new();
    let mut premiere = None;
    let mut derniere = None;
    for _ in 0..3 {
        barres.clear();
        premiere = None;
        derniere = None;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => {
                    let s = strings(Lang::Fr);
                    if text.galley.text() == s.action_assemble {
                        premiere = Some(text.pos);
                    }
                    if text.galley.text() == s.file_quit {
                        derniere = Some(text.pos);
                    }
                }
                egui::epaint::Shape::Rect(r) if r.rect.width() < 20.0 && r.rect.height() > 40.0 => {
                    barres.push(r.rect);
                }
                _ => {}
            }
        }
        output.textures_delta.clear();
    }

    // La première et la dernière commande sont toutes les deux peintes :
    // rien n'est resté sous le bord.
    assert!(premiere.is_some(), "la première commande doit être visible");
    assert!(
        derniere.is_some(),
        "la dernière commande aussi : c'est tout l'objet du changement"
    );
    // Et aucune barre de défilement dans la palette — elle est centrée,
    // donc au milieu de l'écran.
    let fautives: Vec<_> = barres
        .iter()
        .filter(|r| r.min.x > 400.0 && r.max.x < 900.0)
        .collect();
    assert!(
        fautives.is_empty(),
        "la palette ne doit pas défiler quand tout tient : {fautives:?}"
    );
}

/// Filtrée, la palette rétrécit : quatre thèmes ne se lisent pas au
/// milieu d'une fenêtre de huit cents pixels.
#[test]
fn the_command_palette_shrinks_to_what_it_shows() {
    let cadre = |query: &str| -> f32 {
        let mut app = NamsdecApp {
            command_palette: Some(CommandPalette {
                query: query.to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 1000.0),
            )),
            ..Default::default()
        };
        let mut hauteur = 0.0f32;
        for _ in 0..3 {
            hauteur = 0.0;
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Rect(r) = &clipped.shape {
                    // Le cadre de la palette : large, et centré.
                    if r.rect.width() > 400.0 && r.rect.width() < 700.0 {
                        hauteur = hauteur.max(r.rect.height());
                    }
                }
            }
            output.textures_delta.clear();
        }
        hauteur
    };

    let tout = cadre("");
    let filtre = cadre("Thème");
    assert!(tout > 400.0, "la palette entière est haute : {tout}");
    assert!(
        filtre < tout / 2.0,
        "filtrée, elle doit se replier : {filtre} contre {tout}"
    );
}

/// Le bandeau nomme l'adresse qu'il calcule : `tampon+2`, l'étiquette
/// telle qu'elle est écrite dans le source, à côté du nombre.
#[test]
fn the_next_instruction_names_the_address_it_computes() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
tampon: db 0x11, 0x22, 0x33, 0x44
section .text
_start:
mov rcx, 2
mov al, [tampon + rcx]
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 820.0),
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

    assert!(
        painted.contains("tampon+2"),
        "l'adresse calculée doit porter son étiquette : {painted}"
    );
}

/// Une adresse qu'aucune étiquette ne couvre reste un nombre : on ne
/// rattache pas `_start+4000` à une étiquette qui s'arrête bien avant.
#[test]
fn an_address_past_its_label_is_not_named() {
    use namsdec_asm::elf::Symbol;
    let symbols = vec![
        Symbol {
            name: "tampon".to_owned(),
            addr: 0x401000,
            size: 4,
        },
        Symbol {
            name: "suite".to_owned(),
            addr: 0x401010,
            size: 0,
        },
    ];
    assert_eq!(symbol_at(&symbols, 0x401000).as_deref(), Some("tampon"));
    assert_eq!(symbol_at(&symbols, 0x401002).as_deref(), Some("tampon+2"));
    // Au-delà de sa taille déclarée, l'étiquette ne dit plus rien.
    assert_eq!(symbol_at(&symbols, 0x401004), None);
    // Avant la première, non plus.
    assert_eq!(symbol_at(&symbols, 0x400fff), None);
    // Sans taille déclarée, une étiquette couvre jusqu'à la suivante —
    // ici il n'y en a pas, donc son seul octet.
    assert_eq!(symbol_at(&symbols, 0x401010).as_deref(), Some("suite"));
    assert_eq!(symbol_at(&symbols, 0x401011), None);
}

/// La fenêtre d'analyse montre les protections du binaire assemblé :
/// PIE, NX, RELRO, canari, segments — et ce que chacune change.
#[test]
fn the_analysis_window_reports_the_binary_protections() {
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_analysis: true,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1000.0),
        )),
        ..Default::default()
    };
    let mut painted = String::new();
    for _ in 0..3 {
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

    assert!(painted.contains("Protections du binaire"), "{painted}");
    for attendu in [
        "PIE",
        "NX",
        "RELRO",
        "Canari de pile",
        "Symboles",
        "Segments",
        "LOAD",
    ] {
        assert!(
            painted.contains(attendu),
            "« {attendu} » doit figurer dans l'analyse : {painted}"
        );
    }
    // Et les verdicts doivent être ceux du vrai fichier, pas un texte
    // fixe : `ld` sans `-pie`, sans bibliothèque, donne un exécutable à
    // adresse fixe et lié statiquement.
    assert!(
        painted.contains("lié statiquement"),
        "un programme sans bibliothèque est lié statiquement : {painted}"
    );
}

/// Le cas qui compte pour la rétro-ingénierie : un binaire qu'on n'a pas
/// écrit, ouvert depuis le disque. Son analyse doit être celle de *ce*
/// fichier — PIE, lié dynamiquement, RELRO complet — et non celle du
/// petit programme d'exemple.
#[test]
fn a_binary_from_disk_is_analysed_as_itself() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp {
        show_analysis: true,
        ..Default::default()
    };
    // Trop gros pour la mémoire simulée — il ne s'exécutera pas ici,
    // mais il doit s'*analyser*.
    app.load_binary(bytes);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1000.0),
        )),
        ..Default::default()
    };
    let mut painted = String::new();
    for _ in 0..3 {
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

    assert!(
        painted.contains("lié dynamiquement"),
        "un binaire de distribution est lié dynamiquement : {painted}"
    );
    assert!(
        painted.contains("Exécutable relogeable"),
        "et compilé en PIE : {painted}"
    );
    assert!(
        painted.contains("Bibliothèques réclamées"),
        "ses bibliothèques doivent être listées : {painted}"
    );
}
