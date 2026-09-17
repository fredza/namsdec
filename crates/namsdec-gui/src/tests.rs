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

/// Les quatre lectures de la mémoire disent quatre choses différentes
/// des mêmes octets.
#[test]
fn the_memory_formats_read_the_same_bytes_differently() {
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

    // Et les pointeurs : le mot de huit octets qui vaut 1 ne pointe
    // nulle part, mais celui qu'on pose ici mène à « Hi ».
    mem.map(
        BASE_ADDR..BASE_ADDR + 0x1000,
        namsdec_core::Perms::DATA,
        "test",
    );
    mem.write_u64(BASE_ADDR + 0x100, BASE_ADDR).unwrap();
    let chaîne = read_memory(&mem, BASE_ADDR + 0x100, MemoryFormat::Chain, "hors bornes");
    let première = chaîne.lines().next().expect("une ligne");
    assert!(
        première.contains(&format!("{BASE_ADDR:016x}")) && première.contains("\"Hi\""),
        "l'adresse suivie jusqu'à son texte : {première}"
    );

    // Hors de la fenêtre simulée, les quatre le disent.
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
    // Et le listing commence par l'étiquette du point d'entrée, suivie
    // de sa première instruction — pas par les octets « 7f 45 4c 46 »
    // de l'en-tête lus comme une instruction.
    let mut lines = app.source.lines();
    assert_eq!(lines.next(), Some("_start:"), "listing : {}", app.source);
    let first = lines.next().unwrap_or_default();
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

/// Ce que l'interface écrit vraiment à l'écran, deux images durant —
/// la première sert à egui pour mesurer, la seconde montre le résultat.
fn paint(app: &mut NamsdecApp) -> String {
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
    painted
}

/// Le manuel s'exporte en Markdown, dans la langue lue, et le fichier
/// contient bien les 140 fiches : ce que namsdec explique doit pouvoir
/// se lire sans namsdec.
#[test]
fn the_whole_manual_leaves_in_one_markdown_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manuel.md");

    let mut app = NamsdecApp::default();
    app.export_manual(&path);
    assert!(
        matches!(app.status, Status::Note(_)),
        "l'export devrait réussir : {:?}",
        app.status
    );

    let texte = std::fs::read_to_string(&path).unwrap();
    for entry in namsdec_doc::all() {
        assert!(
            texte.contains(&format!("### {} — ", entry.names[0])),
            "{} manque au manuel exporté",
            entry.names[0]
        );
        assert!(
            texte.contains(entry.title.fr),
            "{} : titre absent",
            entry.names[0]
        );
    }
    // Les exemples sortent en blocs de code : autant qu'il y a de
    // fiches, ouverts et refermés.
    assert_eq!(
        texte.matches("```nasm").count(),
        namsdec_doc::count(),
        "un bloc de code par fiche"
    );
    assert_eq!(
        texte.matches("```").count(),
        namsdec_doc::count() * 2,
        "chaque bloc se referme"
    );

    // Et l'anglais dit la même chose, en anglais.
    app.lang = Lang::En;
    app.export_manual(&path);
    let anglais = std::fs::read_to_string(&path).unwrap();
    assert!(anglais.starts_with("# The namsdec manual"));
    assert!(anglais.contains("Flags written:"));
}

/// Suivre un renvoi ne perd pas le chemin : la fiche quittée revient
/// d'un bouton, et le sommaire efface l'historique.
#[test]
fn the_help_window_remembers_where_one_came_from() {
    let mut app = NamsdecApp::default();
    app.open_help("cmp");
    assert!(app.help_back.is_empty(), "rien à quitter la première fois");

    app.open_help("je");
    assert_eq!(app.help_back, vec!["cmp".to_owned()]);

    // Deux fois la même fiche ne s'empile pas : on n'a pas bougé.
    app.open_help("je");
    assert_eq!(app.help_back, vec!["cmp".to_owned()]);

    // L'historique ne grandit pas sans fin : c'est une lecture, pas un
    // journal.
    for _ in 0..40 {
        app.open_help("push");
        app.open_help("pop");
    }
    assert!(app.help_back.len() <= 32, "{}", app.help_back.len());
}

/// Une fiche ouverte mène aux fiches qu'elle cite : les renvois se
/// déduisent du texte, donc personne ne peut oublier de les écrire.
#[test]
fn a_sheet_leads_to_the_sheets_it_mentions() {
    let mut app = NamsdecApp {
        show_help: true,
        ..Default::default()
    };
    app.help = Some(("cmp".to_owned(), namsdec_doc::lookup("cmp")));

    let painted = paint(&mut app);

    assert!(
        painted.contains(strings(app.lang).help_see_also),
        "{painted}"
    );
    // `cmp` parle de `ja`/`jb` : la fiche des sauts conditionnels est à
    // un clic, sous son premier nom.
    assert!(painted.contains("|je|"), "{painted}");

    // Et les renvois sont vrais partout : aucune fiche ne se renvoie à
    // elle-même, et chaque renvoi est une fiche qui existe.
    for entry in namsdec_doc::all() {
        for lang in ["fr", "en"] {
            for autre in namsdec_doc::related(entry, lang) {
                assert!(
                    !std::ptr::eq(autre, entry),
                    "{} se renvoie à elle-même",
                    entry.names[0]
                );
                assert_eq!(
                    namsdec_doc::lookup(autre.names[0]),
                    Some(autre),
                    "renvoi vers une fiche introuvable"
                );
            }
        }
    }
}

/// F1 répond aussi dans le listing d'un binaire : le mot sous le curseur
/// y est souvent une fonction empruntée, pas une instruction.
#[test]
fn asking_about_a_borrowed_function_gets_an_answer() {
    let mut app = NamsdecApp {
        source: "_start:\n    call strcpy@plt\n".to_owned(),
        show_help: true,
        ..Default::default()
    };
    // Le curseur au milieu de « strcpy@plt ».
    app.cursor_byte = app.source.find("strcpy").unwrap() + 3;
    app.do_instruction_help();

    let painted = paint(&mut app);
    // Ce qu'elle fait, et pourquoi on la regarde en premier.
    assert!(painted.contains("Copie une chaîne"), "{painted}");
    assert!(painted.contains(strings(app.lang).help_watch), "{painted}");
    // Et non le message qui dit qu'on ne connaît pas ce mot.
    assert!(
        !painted.contains(strings(app.lang).help_unknown),
        "{painted}"
    );
}

/// Le manuel s'annonce par son compte de fiches. Un compte faux dans une
/// documentation qui apprend à compter serait un comble : il se vérifie.
#[test]
fn the_manual_announces_how_many_sheets_it_really_has() {
    let combien = namsdec_doc::all().len().to_string();
    for lang in [Lang::Fr, Lang::En] {
        let phrase = strings(lang).help_search_hint;
        assert!(
            phrase.contains(&combien),
            "« {phrase} » n'annonce pas les {combien} fiches"
        );
    }
}

/// Sans fiche à montrer, la fenêtre d'aide montre le manuel : ses quatre
/// familles et les fiches de chacune. Chercher suppose de savoir quoi
/// chercher ; un sommaire, non.
#[test]
fn the_help_window_offers_its_contents_when_no_sheet_is_open() {
    let mut app = NamsdecApp {
        show_help: true,
        ..Default::default()
    };
    app.help = None;

    let painted = paint(&mut app);

    let s = strings(app.lang);
    for famille in [
        s.help_kind_concepts,
        s.help_kind_instructions,
        s.help_kind_directives,
        s.help_kind_preprocessors,
    ] {
        assert!(painted.contains(famille), "le sommaire tait « {famille} »");
    }
    // Et la famille ouverte d'emblée est celle des notions : ce sont
    // elles qu'on lit quand on ne sait pas encore par où commencer.
    let notion = namsdec_doc::all()
        .iter()
        .find(|e| e.kind == namsdec_doc::Kind::Concept)
        .expect("au moins une notion");
    assert!(
        painted.contains(notion.title.get(app.lang.code())),
        "les notions devraient être ouvertes : {painted}"
    );
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
    let mut programs = vec![
        ("le programme d'accueil", DEFAULT_SOURCE.to_owned()),
        ("le squelette de « Nouveau »", NEW_PROGRAM.to_owned()),
    ];
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
        // une —, soit il attend ce qu'on lui tapera, ce qui est aussi
        // rendre la main. Ce qu'on refuse, c'est le programme qui tourne
        // sans fin.
        let mut app = NamsdecApp {
            source: source.clone(),
            ..Default::default()
        };
        app.do_assemble();
        for _ in 0..1000 {
            app.do_step();
        }
        let arrêté = app.emulator.as_ref().is_some_and(|e| e.cpu().halted)
            || matches!(app.status, Status::Error(_) | Status::WaitingForInput);
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

        // Et l'interpréteur les exécute — sauf si la fiche annonce le
        // contraire. La documentation couvre aussi ce que namsdec ne
        // simule pas, parce qu'on le lit dans les binaires des autres ;
        // mais alors elle doit le dire, sans quoi la fiche promet une
        // exécution qui n'aura pas lieu. Les autres erreurs — une
        // adresse hors bornes, une division par zéro — viennent des
        // données inventées ici et ne prouvent rien.
        let annoncée_non_simulée = entry.body.fr.contains("ne simule pas")
            || entry.body.fr.contains("ne les simule pas")
            || entry.body.fr.contains("ne la simule pas");
        if annoncée_non_simulée {
            assert!(
                entry.body.en.contains("does not simulate"),
                "{} le dit en français et pas en anglais",
                entry.names[0]
            );
            continue;
        }
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

    // Le panneau latéral, lui, n'en a plus aucune au-dessus de ses
    // onglets : l'état du processeur y tient en entier, et les registres
    // vectoriels sont partis dans leur propre fenêtre. Voir la
    // vérification complète dans `the_sidebar_never_scrolls_its_state`.
}

/// Le panneau latéral ne défile pas : ni les registres, ni les
/// vectoriels, ni la pile x87.
///
/// Les vectoriels y tenaient dans une zone bornée, et cette zone avait
/// sa barre — au milieu de l'état du processeur, en plein pas à pas.
/// Ils ont maintenant une fenêtre, et le panneau n'a plus rien qui
/// défile au-dessus de ses onglets.
#[test]
fn the_sidebar_never_scrolls_its_state() {
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        cpu_state_open: true,
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
    let mut barres: Vec<egui::Rect> = Vec::new();
    let mut onglet_y = f32::MAX;
    let mut lien = false;
    for _ in 0..3 {
        barres.clear();
        onglet_y = f32::MAX;
        lien = false;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => {
                    let texte = text.galley.text();
                    if texte == strings(Lang::Fr).tab_stack {
                        onglet_y = onglet_y.min(text.pos.y);
                    }
                    if texte == strings(Lang::Fr).vectors_open {
                        lien = true;
                    }
                    // Aucun registre vectoriel ne doit être peint dans le
                    // panneau : ils ne sont plus là.
                    assert_ne!(texte, "XMM0", "les vectoriels ont quitté le panneau");
                }
                egui::epaint::Shape::Rect(r) if r.rect.width() < 20.0 && r.rect.height() > 40.0 => {
                    barres.push(r.rect);
                }
                _ => {}
            }
        }
        output.textures_delta.clear();
    }

    assert!(lien, "le panneau doit mener aux registres étendus");
    let fautives: Vec<_> = barres
        .iter()
        .filter(|r| r.min.x > 1280.0 - SIDEBAR_WIDTH && r.max.y < onglet_y)
        .collect();
    assert!(
        fautives.is_empty(),
        "rien ne doit défiler au-dessus des onglets : {fautives:?}"
    );
}

/// Et les registres vectoriels, eux, s'affichent en entier dans leur
/// fenêtre — les seize, plus les huit emplacements x87.
#[test]
fn the_vector_window_shows_them_all() {
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_vectors: true,
        ..Default::default()
    };
    app.do_assemble();

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
    let mut barres: Vec<egui::Rect> = Vec::new();
    for _ in 0..3 {
        painted.clear();
        barres.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => {
                    painted.push_str(text.galley.text());
                    painted.push('|');
                }
                egui::epaint::Shape::Rect(r) if r.rect.width() < 20.0 && r.rect.height() > 40.0 => {
                    barres.push(r.rect);
                }
                _ => {}
            }
        }
        output.textures_delta.clear();
    }

    for attendu in ["XMM0", "XMM15", "ST0", "ST7"] {
        assert!(
            painted.contains(attendu),
            "« {attendu} » doit être visible dans la fenêtre : {painted}"
        );
    }
    // Et rien ne défile : c'est pour cela que cette fenêtre existe.
    // La fenêtre est centrée ; le panneau latéral, lui, est à droite.
    let fautives: Vec<_> = barres
        .iter()
        .filter(|r| r.min.x < 1280.0 - SIDEBAR_WIDTH)
        .collect();
    assert!(
        fautives.is_empty(),
        "les registres étendus ne doivent pas défiler : {fautives:?}"
    );
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
    // Une fenêtre haute : l'onglet montre d'abord la pile elle-même, et
    // l'aide-mémoire de la convention vient après — c'est lui qu'on
    // vérifie ici.
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
    let pack = namsdec_exercises::builtin_pack();
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
    for exercise in &namsdec_exercises::builtin_pack().exercises {
        assert!(
            !sheets_used(&exercise.starter_code).is_empty(),
            "{} ne renvoie à aucune fiche",
            exercise.id
        );
    }

    // Et elles s'affichent sous l'énoncé.
    let mut app = NamsdecApp {
        exercises: true,
        show_exercises: true,
        ..Default::default()
    };
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).exercises_sheets),
        "{painted}"
    );
}

/// Le parcours livré est un parcours : seize étapes dans l'ordre où
/// l'on apprend, chacune avec son but écrit dans le programme.
///
/// L'ordre n'est pas décoratif. On lit les registres avant d'écrire une
/// boucle, on sait boucler avant de parcourir un tableau, et l'on a vu
/// une chaîne avant de l'écrire sur la sortie.
#[test]
fn the_built_in_pack_is_a_course_in_order() {
    let pack = namsdec_exercises::builtin_pack();
    let ids: Vec<&str> = pack.exercises.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "registres",
            "boucle",
            "bits",
            "comparer",
            "signe",
            "multiplier",
            "division",
            "memoire",
            "boutisme",
            "tableau",
            "chaine",
            "pile",
            "appels",
            "sortie",
            "debordement",
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
                // Le *dernier* : le code est peint deux fois, une fois
                // pour aligner la gouttière et une fois par le champ de
                // texte, et c'est le second qui reçoit le pointeur.
                let (pos, galley) = texts
                    .iter()
                    .rfind(|(_, g)| g.text().starts_with("global _start"))
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

/// Le titre de fiche affiché sous l'instruction courante ouvre la
/// fiche : on s'arrête sur une instruction parce qu'on ne la comprend
/// pas, et le curseur de l'éditeur est resté ailleurs.
#[test]
fn the_banner_leads_to_the_sheet_of_the_instruction_it_shows() {
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    movzx eax, al\n    hlt\n".to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Le bandeau annonce la fiche de l'instruction à venir.
    let painted = paint(&mut app);
    let titre = namsdec_doc::lookup("movzx").unwrap().title.fr;
    assert!(painted.contains(titre), "{painted}");

    // Et le clic sur ce titre l'ouvre, fenêtre comprise.
    assert!(!app.show_help);
    app.open_requested_sheet(Some("movzx".to_owned()));
    assert!(app.show_help);
    assert_eq!(
        app.help.as_ref().map(|(mot, _)| mot.as_str()),
        Some("movzx")
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
        painted.contains("-> 0x33"),
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
/// « Tant que l'écran le permet » est la clause qui compte : une
/// quarantaine de commandes ne tiennent pas dans huit cents pixels, et
/// une barre a alors sa place. Ce qu'on refuse, c'est la barre sur un
/// écran qui aurait pu tout montrer.
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
            egui::vec2(1280.0, 1400.0),
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

/// Ce que namsdec comprend d'un binaire doit pouvoir sortir de namsdec :
/// le rapport se copie en Markdown, explications comprises, et dit d'où
/// il vient.
#[test]
fn the_analysis_can_be_taken_away_as_a_report() {
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    let report = app
        .analysis_report()
        .expect("un programme assemblé s'analyse");
    assert!(report.contains("# Analyse du binaire"), "{report}");
    assert!(report.contains("PIE"), "{report}");
    assert!(
        report.contains("Ce que cela change"),
        "un tableau d'états sans explications n'apprend rien : {report}"
    );
    assert!(
        report.contains("namsdec (MIT)"),
        "le rapport dit d'où il vient et sous quelle licence : {report}"
    );

    // Et en anglais, c'est le même rapport dans l'autre langue.
    app.lang = Lang::En;
    let english = app.analysis_report().unwrap();
    assert!(english.contains("# Binary analysis"), "{english}");
    assert!(english.contains("What it changes"), "{english}");

    // Rien à analyser tant que rien n'est chargé.
    let vierge = NamsdecApp::default();
    assert!(vierge.analysis_report().is_none());
}

/// Après un pas, le panneau raconte ce que l'instruction a fait — avec
/// la valeur d'avant, celle qui explique celle d'après.
#[test]
fn the_last_step_says_what_it_changed() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rcx, 5
dec rcx
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| {
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
        painted
    };

    // Rien tant qu'aucun pas n'a été fait.
    assert!(!peindre(&mut app).contains("Dernier pas"));

    app.do_step(); // mov rcx, 5
    let apres_mov = peindre(&mut app);
    assert!(apres_mov.contains("Dernier pas"), "{apres_mov}");
    assert!(
        apres_mov.contains("mov rcx, 5"),
        "l'instruction qui vient de s'exécuter : {apres_mov}"
    );
    assert!(
        apres_mov.contains("RCX 0x0 -> 0x5"),
        "son effet, valeur d'avant comprise : {apres_mov}"
    );

    app.do_step(); // dec rcx
    let apres_dec = peindre(&mut app);
    assert!(
        apres_dec.contains("RCX 0x5 -> 0x4"),
        "le décrément part de ce que le pas précédent avait posé : {apres_dec}"
    );
    // `dec` touche les indicateurs, et c'est souvent tout ce qu'on lui
    // demande.
    assert!(
        apres_dec.contains("PF 0 → 1") || apres_dec.contains("ZF") || apres_dec.contains("SF"),
        "les indicateurs modifiés doivent être dits : {apres_dec}"
    );

    // Une course n'a pas « un » dernier pas : le bandeau se tait.
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }
    assert!(!peindre(&mut app).contains("Dernier pas"));
}

/// La pile se lit par mots, avec ses repères : où est RSP, où est RBP,
/// et laquelle de ces valeurs est une adresse de retour.
#[test]
fn the_stack_shows_rsp_rbp_and_the_return_address() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
call sous
hlt
sous:
push rbp
mov rbp, rsp
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step(); // call  → l'adresse de retour est empilée
    app.do_step(); // push rbp
    app.do_step(); // mov rbp, rsp

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
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

    assert!(
        painted.contains("<- RSP"),
        "RSP doit être repéré : {painted}"
    );
    assert!(
        painted.contains("<- RBP") || painted.contains("RSP, <- RBP"),
        "RBP aussi, une fois le cadre posé : {painted}"
    );
    assert!(
        painted.contains("adresse de code"),
        "l'adresse de retour empilée par `call` doit être nommée : {painted}"
    );
    // Et elle porte l'étiquette de l'endroit d'où l'on vient.
    assert!(
        painted.contains("_start+"),
        "l'adresse de retour pointe juste après le call, dans _start : {painted}"
    );
}

/// Devant un saut conditionnel, le panneau dit s'il sera pris, selon
/// quelle règle, et si la comparaison lit les nombres comme signés.
#[test]
fn a_conditional_jump_says_whether_it_will_be_taken() {
    // 0xff contre 1 : le piège classique. En non signé 0xff est le plus
    // grand (`ja` pris) ; en signé, c'est -1, donc le plus petit
    // (`jg` non pris) — mêmes octets, mêmes indicateurs.
    let peindre = |code: &str, pas: usize| -> String {
        let mut app = NamsdecApp {
            source: code.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
        for _ in 0..pas {
            app.do_step();
        }
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
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
        painted
    };

    let source = |saut: &str| {
        format!(
            "global _start\nsection .text\n_start:\nmov al, 0xff\ncmp al, 1\n{saut} fin\nhlt\nfin:\nhlt\n"
        )
    };

    // Après `mov` et `cmp`, on est arrêté *sur* le saut.
    let non_signe = peindre(&source("ja"), 2);
    assert!(
        non_signe.contains("le saut sera pris"),
        "0xff > 1 en non signé : {non_signe}"
    );
    assert!(
        non_signe.contains("CF = 0 et ZF = 0"),
        "la règle doit être dite : {non_signe}"
    );
    assert!(
        non_signe.contains("comparaison non signée"),
        "et le fait qu'elle ne soit pas signée : {non_signe}"
    );

    let signe = peindre(&source("jg"), 2);
    assert!(
        signe.contains("le saut ne sera pas pris"),
        "0xff vaut -1 en signé : {signe}"
    );
    assert!(
        signe.contains("comparaison signée"),
        "c'est toute la différence : {signe}"
    );

    // Une instruction sans condition ne dit rien de tel.
    let sans = peindre(&source("ja"), 1);
    assert!(!sans.contains("le saut"), "{sans}");
}

/// Devant un `syscall`, le panneau dit lequel et avec quoi : le nom de
/// l'appel, ses arguments nommés, et le texte que le tampon contient.
#[test]
fn a_syscall_is_spelled_out_before_it_runs() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
message: db \"salut\", 10
section .text
_start:
mov rax, 1
mov rdi, 1
mov rsi, message
mov rdx, 6
syscall
mov rax, 60
mov rdi, 0
syscall
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    // Les quatre `mov` faits, on est arrêté *sur* le `syscall`.
    for _ in 0..4 {
        app.do_step();
    }

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| {
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
        painted
    };

    let painted = peindre(&mut app);
    assert!(
        painted.contains("write(fd = 0x1"),
        "l'appel doit être nommé, ses arguments aussi : {painted}"
    );
    assert!(
        painted.contains("count = 0x6"),
        "le troisième argument vient de RDX : {painted}"
    );
    assert!(
        painted.contains("salut"),
        "et le tampon montre ce qui va être écrit : {painted}"
    );

    // L'`exit` de la fin, une fois le write passé et les deux `mov`
    // suivants faits.
    for _ in 0..3 {
        app.do_step();
    }
    let painted = peindre(&mut app);
    assert!(
        painted.contains("exit(code = 0x0)"),
        "le dernier appel se lit aussi bien : {painted}"
    );
}

/// Quand le programme se heurte à une protection mémoire, le panneau ne
/// se contente pas de le constater : il dit par où reprendre.
#[test]
fn a_memory_fault_says_what_to_do_about_it() {
    let mut app = NamsdecApp {
        // Écrire dans son propre code : l'erreur que fait tout débutant
        // qui met ses données dans `.text`.
        source: "\
global _start
section .text
_start:
mov byte [_start], 0x90
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();
    assert!(
        matches!(app.status, Status::Error(_)),
        "écrire dans .text doit échouer : {:?}",
        app.status
    );

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
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

    assert!(
        painted.contains("Cette zone n'est pas inscriptible"),
        "le conseil doit dire où mettre ce qui change : {painted}"
    );
    // Et namsdec l'avait dit avant même d'exécuter : le contrôle
    // d'après-assemblage voit l'écriture dans `.text`, l'adresse étant
    // dans l'encodage.
    assert!(
        app.diagnostics
            .iter()
            .any(|d| d.message.contains("not writable")),
        "l'avertissement doit précéder l'exécution : {:?}",
        app.diagnostics
    );

    // Et il disparaît quand on repart de zéro : il parlait de cette
    // erreur-là. (Refaire le pas, lui, refait la même faute — et le
    // conseil vaut toujours.)
    app.dispatch(Action::Reset);
    let painted = {
        let mut painted = String::new();
        for _ in 0..2 {
            painted.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &clipped.shape {
                    painted.push_str(text.galley.text());
                }
            }
            output.textures_delta.clear();
        }
        painted
    };
    assert!(
        !painted.contains("Cette zone n'est pas inscriptible"),
        "{painted}"
    );
}

/// La gouttière compte les passages : une boucle qui tourne cinq fois se
/// voit, et une ligne jamais atteinte aussi.
#[test]
fn the_gutter_counts_how_many_times_each_line_ran() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rcx, 5
boucle:
dec rcx
jnz boucle
jmp fin
mov rax, 1
fin:
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Rien n'a tourné : pas de colonne, pas de compteur.
    assert!(app.line_visits().is_empty());

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..20 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }

    let passages = app.line_visits();
    assert_eq!(passages.get(&4), Some(&1), "le `mov rcx, 5` : une fois");
    assert_eq!(passages.get(&6), Some(&5), "le corps de boucle : cinq fois");
    assert_eq!(passages.get(&7), Some(&5), "et son saut autant");
    assert_eq!(
        passages.get(&9),
        None,
        "la ligne sautée n'a jamais tourné, et ça se voit : {passages:?}"
    );

    // Et le compteur se peint bien dans la gouttière.
    app.apply_theme(&ctx);
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
    assert!(painted.contains("×5"), "{painted}");
}

/// Le README annonce des chiffres : ils doivent rester vrais.
///
/// C'est la page que voient ceux à qui l'on partage l'outil, et une
/// promesse fausse y coûte plus cher qu'ailleurs. Les chiffres qu'on
/// peut vérifier le sont ici, pour qu'ils vieillissent avec le code.
#[test]
fn the_readme_tells_the_truth() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"))
        .expect("le README est à la racine du dépôt");

    let fiches = namsdec_doc::count();
    assert!(
        readme.contains(&format!("{fiches} instructions, directives et notions")),
        "le README annonce un autre nombre de fiches que les {fiches} existantes"
    );

    let exercices = namsdec_exercises::builtin_pack().exercises.len();
    assert_eq!(exercices, 16, "le parcours livré compte seize exercices");
    assert!(
        readme.contains("parcours de seize exercices"),
        "le README doit annoncer le bon nombre d'exercices"
    );
    // Les crates listées dans le tableau existent toutes.
    for crate_name in [
        "namsdec-core",
        "namsdec-asm",
        "namsdec-disasm",
        "namsdec-emu",
        "namsdec-re",
        "namsdec-doc",
        "namsdec-exercises",
        "namsdec-theme",
        "namsdec-explorer",
        "namsdec-gui",
    ] {
        assert!(
            readme.contains(crate_name),
            "le tableau des crates oublie {crate_name}"
        );
        assert!(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../"))
                .join(crate_name)
                .is_dir(),
            "{crate_name} est annoncée mais n'existe pas"
        );
    }
    // Et les limites annoncées le sont toujours : les appels système
    // simulés sont bien ceux-là, ni plus ni moins.
    for simulé in [0u64, 1, 12, 60, 39, 102, 107, 24, 35, 318] {
        assert!(
            namsdec_re::table::is_simulated(simulé),
            "l'appel {simulé} est annoncé comme simulé"
        );
    }
    assert!(
        !namsdec_re::table::is_simulated(9),
        "mmap n'est pas simulé, et le README le dit"
    );
    for nom in ["getpid", "getrandom", "sched_yield"] {
        assert!(readme.contains(nom), "le README annonce {nom}");
    }
}

/// L'onglet Mémoire nomme ce qu'on regarde : l'étiquette du source, la
/// zone et ses droits — pas seulement une adresse.
#[test]
fn the_memory_tab_names_what_it_shows() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
message: db \"salut\", 10
section .text
_start:
hlt
"
        .to_owned(),
        active_tab: SidebarTab::Memory,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // On va voir l'étiquette, comme on le ferait en tapant son nom.
    let message = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|symbol| symbol.name == "message")
        .expect("l'étiquette est dans l'ELF")
        .addr;
    app.mem_addr = message;

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
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

    assert!(
        painted.contains("message · "),
        "l'étiquette doit précéder la zone : {painted}"
    );
    assert!(
        painted.contains("écriture"),
        "et les droits de .data avec : {painted}"
    );
}

/// `int 0x80` est le piège du vieux tutoriel : namsdec ne le simule pas,
/// et dit pourquoi plutôt que de refuser une instruction de plus.
#[test]
fn the_old_32_bit_interface_is_explained() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov eax, 4
int 0x80
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step(); // mov eax, 4
    app.do_step(); // int 0x80 → refusé

    assert!(
        matches!(app.status, Status::Error(_)),
        "l'instruction n'est pas simulée : {:?}",
        app.status
    );
    let advice = app.error_advice.expect("et elle appelle une explication");
    assert!(advice.contains("syscall"), "{advice}");
    assert!(advice.contains("32 bits"), "{advice}");
}

/// Ouvrir un binaire en donne un listing structuré : les fonctions
/// portent leur nom, et chaque saut dit où il va.
#[test]
fn an_opened_binary_gets_its_labels_back() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        eprintln!("nasm ou ld absent : test ignoré");
        return;
    }
    // Un vrai programme, assemblé et lié, puis rouvert comme un binaire
    // qu'on n'aurait pas écrit.
    let out = namsdec_asm::assemble(
        "\
global _start
section .text
_start:
call sous
hlt
sous:
mov rax, 1
ret
",
        0x400000,
    )
    .expect("assemblage");

    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&out.file), "l'ELF doit se charger");

    // Les étiquettes du source sont revenues dans le listing.
    assert!(
        app.source.contains("_start:"),
        "le point d'entrée porte son nom : {}",
        app.source
    );
    assert!(
        app.source.contains("sous:"),
        "la sous-routine aussi : {}",
        app.source
    );
    // Et l'appel dit qui il appelle.
    assert!(
        app.source.contains("; -> sous"),
        "le `call` doit nommer sa cible : {}",
        app.source
    );

    // La gouttière reste juste : la ligne marquée pendant l'exécution
    // est celle de l'instruction, pas celle d'une étiquette.
    let state = app.assemble_state.as_ref().unwrap();
    let ligne_entree = state.addr_to_line[&out.entry];
    let texte_ligne = app
        .source
        .lines()
        .nth(ligne_entree as usize - 1)
        .expect("la ligne existe");
    assert!(
        texte_ligne.starts_with("call"),
        "la ligne du point d'entrée porte son instruction, pas l'étiquette : {texte_ligne:?}"
    );
}

/// F12 sur une étiquette mène à l'endroit où elle est définie.
#[test]
fn going_to_a_label_lands_on_its_definition() {
    let source = "\
global _start
section .text
_start:
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    hlt
";
    // La fonction pure, d'abord : c'est elle qui décide.
    assert_eq!(label_line(source, "boucle"), Some(5));
    assert_eq!(label_line(source, "_start"), Some(3));
    // Ce qui n'est pas une définition n'en est pas une : le `boucle` du
    // `jnz` ne compte pas, et un nom plus long non plus.
    assert_eq!(label_line(source, "bouclette"), None);
    assert_eq!(label_line(source, "hlt"), None);

    let mut app = NamsdecApp {
        source: source.to_owned(),
        // Le curseur sur le `boucle` du `jnz`, en ligne 7.
        cursor_byte: source.find("jnz boucle").unwrap() + 4,
        ..Default::default()
    };
    app.dispatch(Action::GoToLabel);
    assert_eq!(app.goto_line, Some(5), "F12 doit mener à `boucle:`");

    // Sur un mot qui n'est pas une étiquette, l'outil le dit plutôt que
    // de ne rien faire.
    app.goto_line = None;
    app.cursor_byte = source.find("mov rcx").unwrap();
    app.dispatch(Action::GoToLabel);
    assert_eq!(app.goto_line, None);
    assert!(
        matches!(app.status, Status::Note(_)),
        "une touche qui ne répond pas laisse croire qu'elle n'existe pas : {:?}",
        app.status
    );
}

/// La barre de statut dit quel fichier on édite, et s'il reste quelque
/// chose à enregistrer.
#[test]
fn the_status_bar_says_what_is_unsaved() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    let fichier = dossier.path().join("essai.asm");
    std::fs::write(&fichier, "global _start\nsection .text\n_start:\nhlt\n").unwrap();

    let mut app = NamsdecApp::default();
    app.open_file(fichier.clone());

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| {
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
        painted
    };

    // Tout juste ouvert : le nom, sans marque.
    let painted = peindre(&mut app);
    assert!(painted.contains("|essai.asm|"), "{painted}");

    // Une frappe, et la marque apparaît.
    app.source.push_str("    nop\n");
    let painted = peindre(&mut app);
    assert!(
        painted.contains("essai.asm •"),
        "le tampon modifié doit se voir : {painted}"
    );

    // Enregistré, elle disparaît.
    app.do_save();
    let painted = peindre(&mut app);
    assert!(!painted.contains("essai.asm •"), "{painted}");
    assert_eq!(
        std::fs::read_to_string(&fichier).unwrap(),
        app.source,
        "et le disque porte bien ce qu'on voit"
    );
}

/// Les octets de l'instruction sont découpés par rôle : un préfixe n'est
/// pas un opcode, et un immédiat encore moins.
#[test]
fn the_bytes_of_an_instruction_are_split_by_role() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rax, rbx
mov eax, 42
syscall
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| {
        let mut textes = Vec::new();
        for _ in 0..2 {
            textes.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &clipped.shape {
                    textes.push(text.galley.text().to_owned());
                }
            }
            output.textures_delta.clear();
        }
        textes
    };

    // `mov rax, rbx` s'encode 48 89 d8 : le REX.W à part, l'opcode à
    // part, l'octet d'adressage à part.
    let textes = peindre(&mut app);
    assert!(textes.iter().any(|t| t == "48"), "{textes:?}");
    assert!(textes.iter().any(|t| t == "89"), "{textes:?}");
    assert!(textes.iter().any(|t| t == "d8"), "{textes:?}");
    // Et surtout : pas d'un seul tenant.
    assert!(!textes.iter().any(|t| t == "48 89 d8"), "{textes:?}");

    // `mov eax, 42` : l'immédiat forme un groupe à lui seul.
    app.do_step();
    let textes = peindre(&mut app);
    assert!(
        textes.iter().any(|t| t == "2a 00 00 00"),
        "les quatre octets de l'immédiat vont ensemble : {textes:?}"
    );
}

/// Ctrl+F cherche dans le code : les occurrences se comptent, se
/// parcourent, et se voient.
#[test]
fn finding_in_the_code_counts_and_walks_the_matches() {
    let source = "\
global _start
section .text
_start:
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    hlt
";
    // La fonction pure d'abord : sans la casse, et par octets.
    assert_eq!(
        find_matches(source, "boucle").len(),
        2,
        "l'étiquette et le saut qui y mène"
    );
    assert_eq!(
        find_matches(source, "BOUCLE").len(),
        2,
        "la casse est ignorée"
    );
    assert!(find_matches(source, "").is_empty());
    assert!(find_matches(source, "introuvable").is_empty());

    let mut app = NamsdecApp {
        source: source.to_owned(),
        // Le curseur sur le `boucle` du `jnz` : c'est lui que Ctrl+F
        // doit proposer sans qu'on tape.
        cursor_byte: source.find("jnz boucle").unwrap() + 4,
        ..Default::default()
    };
    app.dispatch(Action::Find);
    let find = app.find.as_ref().expect("la barre doit s'ouvrir");
    assert_eq!(find.query, "boucle", "le mot sous le curseur est repris");

    // La barre se dessine, et compte.
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
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
    assert!(painted.contains("1/2"), "deux occurrences : {painted}");

    // Et Ctrl+F referme ce qu'elle avait ouvert.
    app.dispatch(Action::Find);
    assert!(app.find.is_none());
}

/// Un travail non enregistré ne disparaît pas parce qu'on a cliqué
/// ailleurs : l'outil demande d'abord.
#[test]
fn unsaved_work_is_not_thrown_away_without_asking() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    let fichier = dossier.path().join("autre.asm");
    std::fs::write(&fichier, "global _start\nsection .text\n_start:\nhlt\n").unwrap();

    let mut app = NamsdecApp::default();
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

    let s = strings(Lang::Fr);
    for réponse in [s.unsaved_save, s.unsaved_discard, s.unsaved_cancel] {
        assert!(
            painted.contains(réponse),
            "« {réponse} » doit être proposé : {painted}"
        );
    }
    assert!(painted.contains(s.unsaved_title), "{painted}");
}

/// Tout ce que namsdec sait exécuter doit savoir s'expliquer.
///
/// F1 est la réponse à « qu'est-ce que ça fait ? ». Une instruction que
/// l'interpréteur exécute mais dont la fiche manque laisse la question
/// sans réponse précisément là où elle se pose : devant du code qui
/// tourne. L'inverse est permis — on documente des instructions qu'on
/// ne simule pas encore, parce qu'on les lit dans les binaires des
/// autres.
#[test]
fn everything_the_emulator_runs_has_a_sheet() {
    // Les mnémoniques nommés dans le moteur, lus dans son source : c'est
    // la seule liste qui ne puisse pas se désynchroniser de lui.
    let mut moteur: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let racine = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../namsdec-emu/src"));
    for fichier in std::fs::read_dir(racine).expect("les sources du moteur") {
        let chemin = fichier.expect("une entrée").path();
        if chemin.extension().is_none_or(|ext| ext != "rs")
            || chemin.file_name().is_some_and(|nom| nom == "tests.rs")
        {
            continue;
        }
        let texte = std::fs::read_to_string(&chemin).expect("source lisible");
        for préfixe in ["M::", "Mnemonic::"] {
            for morceau in texte.split(préfixe).skip(1) {
                let nom: String = morceau
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                // `Mnemonic::INVALID` et les noms vides ne sont pas des
                // instructions.
                if nom.len() > 1 && nom.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                    moteur.insert(nom.to_ascii_lowercase());
                }
            }
        }
    }
    assert!(
        moteur.len() > 300,
        "la lecture des sources doit trouver les instructions : {}",
        moteur.len()
    );

    let manquantes: Vec<&String> = moteur
        .iter()
        .filter(|nom| namsdec_doc::lookup(nom).is_none())
        .collect();
    assert!(
        manquantes.is_empty(),
        "ces instructions s'exécutent sans pouvoir s'expliquer : {manquantes:?}"
    );
}

/// Un binaire ouvert fait quinze mille lignes, et l'éditeur doit rester
/// utilisable dessus.
///
/// Deux choses coûtaient une frame entière : recolorer tout le texte à
/// chaque passage — deux fois par frame, pour la gouttière puis pour le
/// champ — et peindre un numéro de ligne pour chacune des quinze mille,
/// quand trente sont à l'écran.
#[test]
fn a_large_listing_still_draws_fast() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes);
    assert!(
        app.source.lines().count() > 5_000,
        "le listing doit être gros pour que la mesure ait un sens : {}",
        app.source.lines().count()
    );

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    // Une frame pour chauffer les caches, puis la mesure.
    let mut premier = ctx.run_ui(input.clone(), |ui| app.draw(ui));
    premier.textures_delta.clear();

    let start = std::time::Instant::now();
    const FRAMES: u32 = 5;
    for _ in 0..FRAMES {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
    }
    let par_frame = start.elapsed() / FRAMES;
    // Large : en debug la mesure est plusieurs fois plus lente qu'en
    // release, et une machine chargée l'est encore plus. Le seuil
    // attrape une régression d'ordre de grandeur — celle qui rendrait
    // l'éditeur saccadé — pas une variation de quelques pour cent.
    assert!(
        par_frame < std::time::Duration::from_millis(100),
        "une frame prend {par_frame:?} sur {} lignes",
        app.source.lines().count()
    );
}

/// Sur un long programme, l'onglet Octets ramène la ligne courante sous
/// les yeux à chaque pas.
///
/// Il ne dessine plus que les rangées visibles — trente sur quinze
/// mille —, ce qui veut dire qu'il ne peut plus compter sur egui pour
/// faire défiler jusqu'à une rangée qu'il n'a pas dessinée : la position
/// se calcule.
#[test]
fn the_bytes_tab_follows_execution_in_a_long_program() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Bytes,
        source: format!(
            "global _start\nsection .text\n_start:\n{}    hlt\n",
            "    nop\n".repeat(400)
        ),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    for _ in 0..300 {
        app.do_step();
    }
    let rip = app.emulator.as_ref().unwrap().cpu().regs.rip;

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
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
        painted.contains(&format!("{rip:08x}")),
        "l'instruction courante ({rip:#x}) doit être à l'écran"
    );
    // Et la première, restée loin au-dessus, ne l'est pas : la vue a
    // bien suivi plutôt que de tout dessiner.
    assert!(
        !painted.contains("00400000"),
        "le début du programme est hors de la vue : {painted}"
    );
}

/// « Nouveau » donne un programme qui tient debout, pas une page
/// blanche : devant un assembleur, elle est un mur.
#[test]
fn a_new_program_is_ready_to_assemble() {
    let mut app = NamsdecApp {
        source: "de quoi salir le tampon".to_owned(),
        ..Default::default()
    };

    // Il y a quelque chose à perdre : on demande avant.
    app.dispatch(Action::New);
    assert_eq!(app.pending, Some(Pending::New));
    assert_eq!(app.source, "de quoi salir le tampon");

    // Une fois accordé, le squelette est posé — et il est propre.
    app.pending = None;
    app.perform(Pending::New);
    assert_eq!(app.source, NEW_PROGRAM);
    assert!(!app.buffer_modified(), "rien à enregistrer d'emblée");
    assert!(app.current_file.is_none());

    // Les trois choses qu'on n'invente pas devant une page blanche.
    for attendu in ["global _start", "section .text", "_start:"] {
        assert!(app.source.contains(attendu), "{}", app.source);
    }
    // Et il sort proprement, plutôt que de laisser le processeur
    // continuer dans ce qui suit.
    assert!(app.source.contains("mov rax, 60"), "{}", app.source);

    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "le squelette doit s'assembler : {:?}",
            app.status
        );
        app.do_run();
        let ctx = egui::Context::default();
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }
        assert!(
            matches!(app.status, Status::Exited { code: 0 }),
            "et rendre la main avec le code 0 : {:?}",
            app.status
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

    let packs = [namsdec_exercises::builtin_pack()];
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
/// y a seize exercices derrière un réglage.
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
            egui::vec2(1280.0, 900.0),
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
    assert!(painted.contains("seize exercices"), "{painted}");
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
            egui::vec2(1280.0, 1000.0),
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
    // Le rang dans le parcours, aussi : le tableau est la dixième des
    // seize étapes.
    assert!(painted.contains("10/16"), "{painted}");
}

/// Un parcours se suit : l'étape suivante est à un clic, sans avoir à
/// retrouver son rang dans une liste de seize.
#[test]
fn the_course_offers_the_next_step() {
    let mut app = NamsdecApp {
        exercises: true,
        ..Default::default()
    };

    // Depuis le premier, la suite est le deuxième.
    app.load_exercise("namsdec/registres");
    let (suivant, titre) = app
        .exercise_after("namsdec/registres")
        .expect("il y a une suite");
    assert_eq!(suivant, "namsdec/boucle");
    assert!(!titre.is_empty());

    // Le dernier n'en a pas : un parcours a une fin, et l'outil ne
    // propose pas d'aller au-delà.
    assert!(app.exercise_after("namsdec/protection").is_none());
    // Un identifiant inconnu non plus.
    assert!(app.exercise_after("namsdec/ceci-nexiste-pas").is_none());

    // Et le bouton mène bien à l'étape suivante.
    app.show_exercises = true;
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
        painted.contains("1/16"),
        "le rang de l'étape en cours : {painted}"
    );
}

/// Le manuel se consulte aussi pour ce qu'on ne connaît pas encore.
///
/// F1 répond sur le mot où l'on est : encore faut-il savoir quoi
/// écrire. La fenêtre d'aide cherche maintenant dans les 144 fiches, par
/// nom comme par titre.
#[test]
fn the_manual_can_be_searched_for_what_one_does_not_know_yet() {
    let mut app = NamsdecApp {
        show_help: true,
        help_query: "mul".to_owned(),
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
    let peindre = |app: &mut NamsdecApp| {
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
        painted
    };

    let painted = peindre(&mut app);
    // `mul` trouve la multiplication — et `imul`, et les vectorielles,
    // qui portent le même mot.
    assert!(painted.contains("mul"), "{painted}");
    assert!(
        painted.contains("fiches"),
        "le compte doit être dit : {painted}"
    );

    // Chercher par le titre marche aussi : on ne sait pas toujours le
    // nom de ce qu'on cherche.
    app.help_query = "pile".to_owned();
    let painted = peindre(&mut app);
    assert!(painted.contains("push"), "{painted}");

    // Et ce qui n'existe pas rend une liste vide, pas une fiche fausse.
    app.help_query = "zzzz".to_owned();
    let painted = peindre(&mut app);
    assert!(painted.contains("0 fiches"), "{painted}");
}

/// Le panneau dit ce que le programme a coûté : le nombre d'instructions
/// exécutées, la seule mesure qui permette de comparer deux façons
/// d'écrire la même chose.
#[test]
fn the_panel_says_how_many_instructions_ran() {
    assert_eq!(group_digits(7), "7");
    assert_eq!(group_digits(1234), "1\u{202f}234");
    assert_eq!(group_digits(1234567), "1\u{202f}234\u{202f}567");

    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rcx, 5
boucle:
dec rcx
jnz boucle
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| {
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
        painted
    };

    // Rien n'a tourné : rien à dire.
    assert!(!peindre(&mut app).contains("Instructions exécutées"));

    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }
    // 1 `mov`, cinq tours de deux instructions, un `hlt` : douze.
    assert_eq!(app.emulator.as_ref().unwrap().executed(), 12);
    let painted = peindre(&mut app);
    assert!(painted.contains("Instructions exécutées"), "{painted}");
    assert!(painted.contains("12"), "{painted}");
}

/// L'exemple d'une fiche se copie : on lit une fiche pour essayer ce
/// qu'elle décrit.
#[test]
fn a_sheet_example_can_be_taken_along() {
    let mut app = NamsdecApp {
        show_help: true,
        help: Some(("movzx".to_owned(), namsdec_doc::lookup("movzx"))),
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
        painted.contains("Copier"),
        "le bouton doit être là : {painted}"
    );
    // Et l'exemple lui-même, qui est ce qu'on copie.
    let exemple = namsdec_doc::lookup("movzx").unwrap().example;
    assert!(painted.contains(exemple), "{painted}");
}

/// Les références croisées répondent à la première question qu'on pose
/// d'un binaire : qui appelle quoi, et qui pointe où.
///
/// Sur un programme assemblé pour de vrai, avec ses adresses réelles —
/// c'est là que les fausses références se voient, pas sur des octets
/// écrits à la main.
#[test]
fn the_analysis_shows_who_calls_what() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
message: db \"salut\", 10
section .text
_start:
call ecrire
call ecrire
mov rax, 60
hlt
ecrire:
mov rsi, message
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let state = app.assemble_state.as_ref().expect("assemblé");

    let début = state.output.load_addr;
    let xrefs = namsdec_re::xrefs::scan(&state.disasm, début..début + 0x2000);

    // `ecrire` est appelée deux fois : c'est une fonction, et c'est par
    // là qu'on entre dans le binaire.
    let fonctions = xrefs.functions();
    assert_eq!(fonctions.len(), 1, "{fonctions:?}");
    let (adresse, appels) = fonctions[0];
    assert_eq!(appels, 2);
    let ecrire = state
        .output
        .symbols
        .iter()
        .find(|symbol| symbol.name == "ecrire")
        .expect("l'étiquette existe");
    assert_eq!(
        adresse, ecrire.addr,
        "la fonction trouvée est bien `ecrire`"
    );

    // Et `message` est désignée par le `mov rsi, message`.
    let message = state
        .output
        .symbols
        .iter()
        .find(|symbol| symbol.name == "message")
        .expect("l'étiquette existe");
    let vers_message = xrefs.to(message.addr);
    assert!(
        vers_message
            .iter()
            .any(|r| r.kind == namsdec_re::xrefs::Kind::Address),
        "qui lit la chaîne ? {vers_message:?}"
    );

    // Le 60 de `mov rax, 60` n'est pas une adresse : il ne tombe pas
    // dans le programme, et le compter noierait les vraies références.
    assert!(xrefs.to(60).is_empty());
}

/// Les fragments réutilisables expliquent à quoi servent les autres
/// protections : un programme de dix lignes en contient déjà.
#[test]
fn the_analysis_finds_reusable_fragments() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
call sous
hlt
sous:
pop rdi
pop rsi
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let state = app.assemble_state.as_ref().unwrap();

    let fragments = namsdec_re::gadgets::from_disasm(&state.disasm, 50);
    assert!(
        fragments.iter().any(|g| g.text() == "pop rsi ; ret"),
        "le fragment le plus simple doit être trouvé : {:?}",
        fragments.iter().map(|g| g.text()).collect::<Vec<_>>()
    );
    // Et celui qui enchaîne les deux `pop`.
    assert!(
        fragments
            .iter()
            .any(|g| g.text() == "pop rdi ; pop rsi ; ret"),
        "{:?}",
        fragments.iter().map(|g| g.text()).collect::<Vec<_>>()
    );
}

/// Le contrôle d'après-assemblage prévient de ce que nasm ne dit pas,
/// et se tait sur les programmes corrects.
///
/// Un avertissement faux est pire que pas d'avertissement : il apprend
/// à ignorer les avertissements.
#[test]
fn the_extra_checks_warn_without_crying_wolf() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let diagnostics = |source: &str| -> Vec<namsdec_asm::Diagnostic> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        app.diagnostics.clone()
    };

    // Un programme qui ne s'arrête pas : le processeur continuerait dans
    // ce qui suit son code.
    let sans_fin = diagnostics("global _start\nsection .text\n_start:\n    mov rax, 1\n");
    assert!(
        sans_fin
            .iter()
            .any(|d| d.message.contains("run off the end")),
        "{sans_fin:?}"
    );

    // Les programmes livrés avec l'application, eux, ne doivent
    // déclencher aucun avertissement : ce sont les modèles.
    let mut modèles = vec![("accueil", DEFAULT_SOURCE), ("nouveau", NEW_PROGRAM)];
    let pack = namsdec_exercises::builtin_pack();
    for exercise in &pack.exercises {
        // Sauf celui qui met une faute en scène exprès.
        if exercise.id != "protection" {
            modèles.push((exercise.id.as_str(), exercise.starter_code.as_str()));
        }
    }
    for (nom, source) in modèles {
        let found = diagnostics(source);
        assert!(
            found.is_empty(),
            "« {nom} » ne doit rien déclencher : {found:?}"
        );
    }
}

/// Entrée garde l'indentation : l'assembleur s'écrit en colonnes, et
/// retaper quatre espaces à chaque ligne use la patience avant d'user le
/// clavier.
#[test]
fn a_new_line_keeps_the_indentation() {
    // La fonction pure, d'abord.
    assert_eq!(newline_indent("    mov rax, 1", 13), "\n    ");
    assert_eq!(
        newline_indent("_start:", 7),
        "\n",
        "une étiquette est à gauche"
    );
    assert_eq!(
        newline_indent("\tmov rax, 1", 11),
        "\n\t",
        "la tabulation est reprise telle quelle"
    );
    // Seuls les blancs sont repris — pas le code.
    assert_eq!(newline_indent("  add rax", 9), "\n  ");
    // Au milieu d'une ligne, c'est l'indentation de cette ligne-là.
    assert_eq!(newline_indent("        hlt", 4), "\n    ");

    // Et le réglage voyage dans settings.json.
    let app = NamsdecApp {
        auto_indent: false,
        ..Default::default()
    };
    let written = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(written.contains("\"auto_indent\":false"), "{written}");
}

/// Un vrai binaire doit se lire avec F1 sous la main.
///
/// La documentation n'est pas faite que pour ce qu'on écrit : en
/// rétro-ingénierie, on lit le code des autres, et une instruction sans
/// fiche y est un trou. Mesuré sur un binaire du système — celui qu'on
/// a sous la main, pas un cas choisi.
#[test]
fn a_real_binary_reads_with_the_manual_at_hand() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes);
    let state = app.assemble_state.as_ref().expect("chargé");

    let mut sans_fiche: std::collections::BTreeSet<String> = Default::default();
    for instruction in &state.disasm {
        let mnemonic = instruction
            .text
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        // `(bad)` : le désassembleur bute sur des octets qui ne sont pas
        // du code. Ce n'est pas une instruction, et il n'y a pas de
        // fiche à écrire pour ça.
        if mnemonic.is_empty() || mnemonic == "(bad)" {
            continue;
        }
        if namsdec_doc::lookup(&mnemonic).is_none() {
            sans_fiche.insert(mnemonic);
        }
    }
    assert!(
        sans_fiche.is_empty(),
        "ces instructions se lisent dans un binaire ordinaire sans pouvoir s'expliquer : {sans_fiche:?}"
    );
}

/// Une chaîne, c'est une piste : le panneau dit qui la désigne.
///
/// « mot de passe » quelque part dans les données ne dit rien ; « mot de
/// passe, lu ici » dit par où entrer dans le programme.
#[test]
fn a_string_says_who_points_at_it() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_analysis: true,
        source: "\
global _start
section .data
message: db \"salutations\", 0
oublie:  db \"personne ne me lit\", 0
section .text
_start:
mov rsi, message
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let state = app.assemble_state.as_ref().unwrap();

    let message = state
        .output
        .symbols
        .iter()
        .find(|s| s.name == "message")
        .expect("l'étiquette existe")
        .addr;
    let oublie = state
        .output
        .symbols
        .iter()
        .find(|s| s.name == "oublie")
        .expect("l'étiquette existe")
        .addr;

    let mémoire = app.emulator.as_ref().unwrap().cpu().mem.range();
    let xrefs = namsdec_re::xrefs::scan(&state.disasm, mémoire);

    assert!(
        !xrefs.to(message).is_empty(),
        "la chaîne utilisée doit avoir sa référence"
    );
    assert!(
        xrefs.to(oublie).is_empty(),
        "celle que personne ne lit n'en a pas — et c'est une information"
    );
}

/// Le résumé répond à la première question : qu'est-ce que c'est, et
/// qu'est-ce qu'il y a dedans ?
///
/// Tout y est mesuré, jamais déclaré : les fonctions viennent des `call`
/// relevés, les chaînes des octets lisibles, la nature du fichier de ses
/// en-têtes.
#[test]
fn the_analysis_opens_on_a_summary() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_analysis: true,
        source: "\
global _start
section .data
message: db \"salutations\", 0
section .text
_start:
call ecrire
hlt
ecrire:
mov rsi, message
ret
"
        .to_owned(),
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

    assert!(painted.contains("En bref"), "{painted}");
    for attendu in ["Code", "Point d'entrée", "Fonctions", "Chaînes", "Fichier"] {
        assert!(
            painted.contains(attendu),
            "« {attendu} » manque : {painted}"
        );
    }
    // Le fichier est un exécutable à adresse fixe, lié statiquement :
    // c'est ce que `ld` produit sans options.
    assert!(painted.contains("EXEC · static"), "{painted}");
}

/// Un point d'arrêt conditionnel attrape le tour qu'on cherche.
///
/// Dans une boucle de cinq cents tours, s'arrêter à chaque passage n'est
/// pas déboguer, c'est une punition : la condition fait le tri.
#[test]
fn a_conditional_breakpoint_catches_the_turn_one_is_after() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rcx, 500
boucle:
dec rcx
jnz boucle
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Un point d'arrêt sur le corps de boucle, qui passe cinq cents
    // fois — mais on ne veut que le dernier tour.
    app.breakpoint_lines.insert(6);
    app.breakpoint_conditions.insert(6, "rcx == 1".to_owned());
    app.sync_breakpoints();

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..20 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }

    assert!(
        matches!(app.status, Status::Breakpoint(_)),
        "l'arrêt doit avoir eu lieu : {:?}",
        app.status
    );
    let rcx = app
        .emulator
        .as_ref()
        .unwrap()
        .cpu()
        .regs
        .get64(namsdec_core::Reg64::Rcx);
    assert_eq!(rcx, 1, "et au tour qu'on avait demandé, pas au premier");

    // Sans condition, le même point d'arrêt s'arrête dès le premier
    // passage.
    let mut app = NamsdecApp {
        source: app.source.clone(),
        ..Default::default()
    };
    app.do_assemble();
    app.breakpoint_lines.insert(6);
    app.sync_breakpoints();
    app.do_run();
    for _ in 0..20 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }
    assert_eq!(
        app.emulator
            .as_ref()
            .unwrap()
            .cpu()
            .regs
            .get64(namsdec_core::Reg64::Rcx),
        500,
        "le premier passage, RCX n'a pas encore été décrémenté"
    );

    // Une condition illisible n'empêche pas l'arrêt : une faute de
    // frappe qui rendrait un point d'arrêt muet serait pire que tout.
    let mut app = NamsdecApp {
        source: app.source.clone(),
        ..Default::default()
    };
    app.do_assemble();
    app.breakpoint_lines.insert(6);
    app.breakpoint_conditions
        .insert(6, "rxc === zéro".to_owned());
    app.sync_breakpoints();
    app.do_run();
    for _ in 0..20 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }
    assert!(
        matches!(app.status, Status::Breakpoint(_)),
        "{:?}",
        app.status
    );
}

/// La chronologie répond à « comment suis-je arrivé là ? », et permet
/// d'y retourner.
///
/// Revenir d'un pas à la fois répond mal : on cherche un endroit, pas un
/// pas. Cliquer sur une ligne rembobine tout — registres, indicateurs,
/// mémoire.
#[test]
fn the_timeline_shows_the_path_and_goes_back_to_it() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_timeline: true,
        source: "\
global _start
section .text
_start:
mov rcx, 4
boucle:
dec rcx
jnz boucle
mov rax, 7
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
    assert!(!app.running, "le programme doit s'arrêter");

    let emulator = app.emulator.as_ref().unwrap();
    let trace: Vec<namsdec_emu::Step> = emulator.trace().iter().copied().collect();
    assert_eq!(trace.len() as u64, emulator.executed());
    assert_eq!(
        emulator.cpu().regs.get64(namsdec_core::Reg64::Rax),
        7,
        "à l'arrivée, RAX vaut 7"
    );

    // Revenir de trois pas : on repasse avant le `mov rax, 7`.
    app.rewind(3);
    let emulator = app.emulator.as_ref().unwrap();
    assert_eq!(
        emulator.cpu().regs.get64(namsdec_core::Reg64::Rax),
        0,
        "le `mov rax, 7` a été défait"
    );
    assert_eq!(
        emulator.trace().len(),
        trace.len() - 3,
        "et le chemin s'est raccourci d'autant"
    );
    assert!(
        matches!(app.status, Status::SteppedBack),
        "{:?}",
        app.status
    );

    // Reculer plus loin que le chemin ne va pas plus loin que son début.
    app.rewind(10_000);
    assert!(app.emulator.as_ref().unwrap().trace().is_empty());

    // Et la fenêtre montre bien le chemin quand il y en a un.
    app.do_assemble();
    app.do_step();
    app.do_step();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1000.0),
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
    assert!(painted.contains("Chronologie"), "{painted}");
    assert!(
        painted.contains("-1|"),
        "le dernier pas est « -1 » : {painted}"
    );
    assert!(painted.contains("-2|"), "{painted}");
}

/// Tout ouvert en même temps, dans les deux langues, avec et sans
/// programme : rien ne doit paniquer ni disparaître.
///
/// Les fenêtres se sont multipliées — analyse, registres étendus, points
/// d'arrêt, chronologie, exercices, aide, préférences, palette. Chacune
/// a son test ; ce qu'aucun ne vérifie, c'est ce qui arrive quand elles
/// sont toutes là en même temps, sur un état incomplet.
#[test]
fn every_window_at_once_holds_together() {
    for lang in [Lang::Fr, Lang::En] {
        for assemblé in [false, true] {
            let mut app = NamsdecApp {
                lang,
                exercises: true,
                show_re_tab: true,
                show_analysis: true,
                show_vectors: true,
                show_breakpoints: true,
                show_timeline: true,
                show_exercises: true,
                show_help: true,
                show_about: true,
                show_preferences: true,
                command_palette: Some(CommandPalette::default()),
                find: Some(Find::default()),
                ..Default::default()
            };
            if assemblé && namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
                app.do_assemble();
                app.do_step();
                app.breakpoint_lines.insert(7);
                app.breakpoint_conditions.insert(7, "rcx == 0".to_owned());
                app.sync_breakpoints();
            }

            let ctx = egui::Context::default();
            app.apply_theme(&ctx);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 1000.0),
                )),
                ..Default::default()
            };
            let mut vus = 0;
            for _ in 0..3 {
                let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                vus = output
                    .shapes
                    .iter()
                    .filter(|clipped| matches!(clipped.shape, egui::epaint::Shape::Text(_)))
                    .count();
                output.textures_delta.clear();
            }
            assert!(
                vus > 50,
                "{lang:?}, assemblé={assemblé} : l'écran est presque vide ({vus} textes)"
            );
            // Et les réglages restent relisibles : une fenêtre ouverte
            // ne doit pas corrompre ce qu'on enregistre.
            let json = serde_json::to_string(&app.settings_snapshot()).unwrap();
            let relu: crate::settings::Settings = serde_json::from_str(&json).unwrap();
            assert_eq!(relu.language, app.lang.code());
        }
    }
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
    assert_eq!(revenu.exercises, app.exercises);
    // `show_re_tab` est délibérément absent des réglages : voir
    // `the_re_tab_is_absent_at_every_launch_until_it_is_asked_for`. Ce
    // qu'on vérifie ici, c'est qu'il le reste.
    assert!(!revenu.show_re_tab, "l'onglet RE ne se persiste pas");
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

/// Les nouveautés ne doivent pas s'effondrer sur un état dégénéré.
///
/// Une chronologie sans programme, une condition posée sur une ligne qui
/// n'existe pas, un rembobinage sans rien à défaire : ces cas-là
/// arrivent, et ils doivent être sans conséquence plutôt que fatals.
#[test]
fn the_new_views_hold_on_a_degenerate_state() {
    // Rien du tout : ni source, ni programme assemblé.
    let mut app = NamsdecApp {
        source: String::new(),
        show_timeline: true,
        show_breakpoints: true,
        ..Default::default()
    };
    app.rewind(5);
    assert!(app.emulator.is_none());
    // Une condition sur une ligne qui n'existe pas ne fait rien.
    app.breakpoint_lines.insert(42);
    app.breakpoint_conditions.insert(42, "rcx == 0".to_owned());
    app.sync_breakpoints();
    assert!(
        app.breakpoint_holds(0x400000),
        "sans programme, on s'arrête"
    );

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
    }

    // Un programme assemblé, puis réinitialisé : la chronologie repart
    // de zéro plutôt que de montrer un chemin qui n'existe plus.
    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        app.do_step();
        app.do_step();
        assert!(!app.emulator.as_ref().unwrap().trace().is_empty());
        app.dispatch(Action::Reset);
        assert!(
            app.emulator.as_ref().unwrap().trace().is_empty(),
            "après réinitialisation, il n'y a plus de chemin parcouru"
        );
        // Et rembobiner au-delà du début ne défait rien de plus.
        app.do_step();
        app.rewind(1000);
        assert!(app.emulator.as_ref().unwrap().trace().is_empty());
    }
}

/// La chronologie dit ce que chaque pas a fait, d'un signe.
///
/// Cent adresses se ressemblent ; ce qu'on cherche en parcourant un
/// chemin, ce sont les appels, les sauts pris et les écritures.
#[test]
fn the_timeline_marks_what_each_step_did() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_timeline: true,
        source: "\
global _start
section .data
case: dq 0
section .text
_start:
mov rax, 1
mov [case], rax
call sous
hlt
sous:
ret
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

    let kinds: Vec<namsdec_emu::StepKind> = app
        .emulator
        .as_ref()
        .unwrap()
        .trace()
        .iter()
        .map(|step| step.kind)
        .collect();
    assert!(kinds.contains(&namsdec_emu::StepKind::Write), "{kinds:?}");
    assert!(kinds.contains(&namsdec_emu::StepKind::Call), "{kinds:?}");
    assert!(kinds.contains(&namsdec_emu::StepKind::Return), "{kinds:?}");

    // Et les marques se peignent.
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1000.0),
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
    for marque in ["app", "ret", "écr"] {
        assert!(
            painted.contains(marque),
            "la marque « {marque} » doit apparaître : {painted}"
        );
    }
}

/// Le filtre de la chronologie garde les pas qui comptent, et le rang
/// reste celui du chemin entier.
///
/// Cinq mille pas ordinaires noient les dix qui comptent. Mais si le
/// rang affiché suivait la liste filtrée, cliquer dessus rembobinerait
/// du mauvais nombre de pas : c'est le chemin qui compte, pas ce qu'on
/// en montre.
#[test]
fn the_timeline_filter_keeps_the_true_ranks() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_timeline: true,
        timeline_notable_only: true,
        source: "\
global _start
section .text
_start:
mov rax, 1
mov rbx, 2
mov rcx, 3
call sous
hlt
sous:
ret
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

    let trace = app.emulator.as_ref().unwrap().trace();
    let total = trace.len();
    let notables: Vec<(usize, namsdec_emu::Step)> = trace
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, pas)| pas.kind != namsdec_emu::StepKind::Plain)
        .collect();
    // Le `call` et le `ret` — les trois `mov` ne sont pas notables.
    assert_eq!(notables.len(), 2, "{notables:?}");
    // Et leurs rangs sont ceux du chemin entier : le `call` est le
    // quatrième pas, pas le premier de la liste filtrée.
    assert_eq!(notables[0].0, 3, "le `call` est le quatrième pas");
    assert!(notables[1].0 < total);

    // À l'écran, la case est cochée et le chemin filtré s'affiche.
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1000.0),
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
    assert!(painted.contains("Seulement appels"), "{painted}");
    // Le recul affiché pour le `call` : total - 3.
    assert!(
        painted.contains(&format!("-{}", total - 3)),
        "le rang affiché doit être celui du chemin : {painted}"
    );
}

/// « Qui a mis ça là ? » : le panneau sait retrouver le pas qui a écrit
/// dans un registre.
#[test]
fn a_register_says_where_its_value_came_from() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rax, 1
mov rbx, 2
add rax, rbx
mov rcx, 9
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

    // RAX vient du `add`, en ligne 6.
    let texte = app.last_write_line("RAX").expect("RAX a été écrit");
    assert!(texte.contains("ligne 6"), "{texte}");
    assert!(texte.contains("add rax, rbx"), "{texte}");

    // RBX, lui, vient du `mov` de la ligne 5.
    let texte = app.last_write_line("RBX").expect("RBX a été écrit");
    assert!(texte.contains("ligne 5"), "{texte}");

    // Et un registre auquel personne n'a touché n'a rien à dire, plutôt
    // que d'inventer une provenance.
    assert!(app.last_write_line("R15").is_none());
    // Ni un nom qui n'est pas un registre général.
    assert!(app.last_write_line("XMM0").is_none());
}

/// L'onglet Mémoire dit qui a écrit à l'adresse qu'on regarde.
#[test]
fn the_memory_tab_says_who_wrote_there() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Memory,
        source: "\
global _start
section .data
case: dq 0
section .text
_start:
mov rax, 7
mov [case], rax
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let case = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|s| s.name == "case")
        .expect("l'étiquette existe")
        .addr;
    app.mem_addr = case;

    // Avant l'exécution, personne n'a écrit : rien à dire.
    assert!(app.last_memory_write_line(case).is_none());

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }

    let texte = app
        .last_memory_write_line(case)
        .expect("le `mov [case], rax` a écrit là");
    assert!(texte.contains("ligne 7"), "{texte}");

    // Et cela se peint dans l'onglet.
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1000.0),
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
    assert!(painted.contains("Écrit au pas"), "{painted}");
}

/// Depuis la chronologie, le clic droit mène à la ligne dans l'éditeur.
///
/// Lire le code d'un pas et rembobiner jusqu'à lui sont deux gestes
/// différents : le second défait le travail de l'exécution, le premier
/// ne touche à rien.
#[test]
fn the_timeline_can_point_at_the_source_without_rewinding() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_timeline: true,
        source: DEFAULT_SOURCE.to_owned(),
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
    let chemin = app.emulator.as_ref().unwrap().trace().len();

    // Le clic droit se simule par ce qu'il déclenche : la ligne visée,
    // sans rembobinage.
    app.goto_line = app
        .assemble_state
        .as_ref()
        .and_then(|state| state.addr_to_line.values().copied().next());
    assert!(app.goto_line.is_some());
    assert_eq!(
        app.emulator.as_ref().unwrap().trace().len(),
        chemin,
        "voir une ligne ne défait rien"
    );

    // Le rembobinage, lui, raccourcit le chemin.
    app.rewind(2);
    assert_eq!(app.emulator.as_ref().unwrap().trace().len(), chemin - 2);
}

/// Les étiquettes se distinguent des fonctions : on entre dans une
/// fonction, on revient d'une fonction ; une étiquette appartient à
/// celle qui la contient.
#[test]
fn jump_targets_are_not_mistaken_for_functions() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rcx, 3
boucle:
dec rcx
jnz boucle
call sous
hlt
sous:
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let state = app.assemble_state.as_ref().unwrap();
    let début = state.output.load_addr;
    let xrefs = namsdec_re::xrefs::scan(&state.disasm, début..début + 0x2000);

    let fonctions = xrefs.functions();
    let étiquettes = xrefs.labels();
    assert_eq!(
        fonctions.len(),
        1,
        "seule `sous` est appelée : {fonctions:?}"
    );
    assert_eq!(
        étiquettes.len(),
        1,
        "et seule `boucle` est sautée : {étiquettes:?}"
    );
    // Aucune n'est dans les deux listes.
    assert!(!étiquettes.contains(&fonctions[0].0));
}

/// Les boucles se voient sans exécuter : un saut qui revient en arrière
/// ferme un chemin.
///
/// C'est ce qui explique qu'un programme de dix lignes exécute des
/// milliers d'instructions — et la gouttière, elle, dit combien de fois
/// on y tourne.
#[test]
fn the_analysis_finds_the_loops() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rcx, 3
boucle:
dec rcx
jnz boucle
jmp fin
hlt
fin:
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let state = app.assemble_state.as_ref().unwrap();
    let début = state.output.load_addr;
    let xrefs = namsdec_re::xrefs::scan(&state.disasm, début..début + 0x2000);

    let boucles = xrefs.loops();
    assert_eq!(
        boucles.len(),
        1,
        "un seul saut revient en arrière : {boucles:?}"
    );
    // Et c'est bien `boucle:` qu'il vise.
    let boucle = state
        .output
        .symbols
        .iter()
        .find(|s| s.name == "boucle")
        .expect("l'étiquette existe")
        .addr;
    assert_eq!(boucles[0].0, boucle);
    // Le `jmp fin`, qui va en avant, n'en est pas une.
    assert!(boucles.iter().all(|(cible, _)| *cible == boucle));
}

/// Le désassemblage se copie, étiquettes comprises.
///
/// C'est ce qu'on colle dans un compte rendu ou un message — pas le
/// source, que l'autre a peut-être déjà.
#[test]
fn the_disassembly_can_be_taken_away() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Rien à copier tant que rien n'est assemblé.
    assert!(NamsdecApp::default().listing_text().is_none());

    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
call sous
hlt
sous:
mov rax, 1
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let listing = app.listing_text().expect("un programme assemblé");
    assert!(listing.contains("_start:"), "{listing}");
    assert!(listing.contains("sous:"), "{listing}");
    assert!(
        listing.contains("; -> sous"),
        "l'appel nomme sa cible : {listing}"
    );
    // Les octets y sont, en colonne.
    assert!(listing.contains("c3"), "le `ret` et ses octets : {listing}");
    // Et l'instruction est écrite comme on l'écrit — ici telle que nasm
    // l'a encodée : `mov rax, 1` devient `mov eax, 1`, cinq octets, le
    // haut de RAX étant mis à zéro de toute façon.
    assert!(listing.contains("mov eax, 1"), "{listing}");
}

/// Le panneau de désassemblage s'ouvre sur le pseudo-code : lire
/// « RAX = RAX + RCX » avant « add rax, rcx » est le chemin naturel.
#[test]
fn the_disassembly_panel_opens_on_pseudo_code() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_disassembly: true,
        source: "\
global _start
section .text
_start:
mov rax, 0
mov rcx, 5
boucle:
add rax, rcx
dec rcx
jnz boucle
hlt
"
        .to_owned(),
        ..Default::default()
    };
    // Le moteur par défaut est le pseudo-code, et non la syntaxe
    // machine : c'est le premier qu'on veut lire.
    assert_eq!(app.disassembler, PSEUDO_ID);
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| {
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
        painted
    };

    let painted = peindre(&mut app);
    assert!(painted.contains("Désassemblage"), "{painted}");
    assert!(painted.contains("RAX = RAX + RCX"), "{painted}");
    assert!(painted.contains("RCX = RCX − 1"), "{painted}");
    assert!(
        painted.contains("si différent, aller à"),
        "le saut dit sa condition : {painted}"
    );
    // Les étiquettes structurent le listing.
    assert!(painted.contains("boucle:"), "{painted}");

    // L'autre moteur montre la syntaxe qu'on écrit.
    app.disassembler = NASM_ID.to_owned();
    let painted = peindre(&mut app);
    assert!(painted.contains("add rax, rcx"), "{painted}");
    assert!(!painted.contains("RAX = RAX + RCX"), "{painted}");
}

/// Un désassembleur extérieur ne s'exécute que sur demande, et le
/// panneau dit ce qu'il va lancer.
///
/// C'est la différence avec un thème : un thème est une palette, un
/// désassembleur de ce genre est un programme. namsdec lit ce qui est
/// déclaré, l'annonce, et attend un clic.
#[test]
fn an_external_disassembler_runs_only_when_asked() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    std::fs::write(
        dossier.path().join("cat.json"),
        r#"{"id":"cat","name":"cat","command":"cat","args":["{file}"]}"#,
    )
    .unwrap();

    let plugins = namsdec_disasm::plugin::read_dir(dossier.path());
    assert_eq!(plugins.len(), 1);

    let mut app = NamsdecApp {
        show_disassembly: true,
        disassembler_plugins: plugins,
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    app.do_assemble();
    app.disassembler = "cat".to_owned();

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
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

    // Rien n'a été lancé : le panneau annonce ce qu'il ferait.
    assert!(app.plugin_output.is_none(), "rien ne s'exécute tout seul");
    assert!(painted.contains("programme extérieur"), "{painted}");
    assert!(painted.contains("cat"), "{painted}");
    assert!(painted.contains("Lancer"), "{painted}");
}

/// Tout ce que l'interface écrit doit exister dans sa police.
///
/// Un caractère absent est peint en carré vide — ce qui, à l'écran,
/// ressemble à un bogue et ne se voit pas en relisant le code. La police
/// livrée avec egui ne couvre ni les flèches, ni les pictogrammes : on
/// s'en tient donc à ce qu'elle a, et ce test le vérifie plutôt que de
/// compter sur la vigilance.
///
/// La méthode : un caractère de la zone privée Unicode, qu'aucune police
/// ne connaît, sert de témoin. Tout ce qui mesure exactement sa largeur
/// est rendu par le même carré.
#[test]
fn every_glyph_the_interface_writes_exists_in_its_font() {
    let ctx = egui::Context::default();
    let app = NamsdecApp::default();
    app.apply_theme(&ctx);

    // Les textes de l'interface, lus dans leurs fichiers : c'est la
    // seule façon de n'en oublier aucun, y compris les marques écrites
    // à même le code des panneaux.
    let sources = [
        include_str!("i18n.rs"),
        include_str!("main.rs"),
        include_str!("sidebar.rs"),
        include_str!("errors.rs"),
        include_str!("roles.rs"),
        include_str!("readings.rs"),
        include_str!("nasm_errors.rs"),
        include_str!("condition.rs"),
        include_str!("../../namsdec-re/src/strings.rs"),
        include_str!("../../namsdec-re/src/protections.rs"),
        include_str!("../../namsdec-re/src/xrefs_module.rs"),
        include_str!("../../namsdec-re/src/gadgets_module.rs"),
        include_str!("../../namsdec-re/src/imports_module.rs"),
        include_str!("../../namsdec-re/src/libc.rs"),
        include_str!("../../namsdec-re/src/binary_info.rs"),
        include_str!("../../namsdec-disasm/src/pseudo.rs"),
    ];

    // Les littéraux de chaîne seulement : ce qui est dans un
    // commentaire n'est jamais peint, et les sources en contiennent
    // beaucoup — flèches, symboles, caractères d'exemple.
    let mut affichés = String::new();
    for source in sources {
        for ligne in source.lines() {
            let taillée = ligne.trim_start();
            if taillée.starts_with("//") {
                continue;
            }
            let mut dans_chaîne = false;
            let mut échappé = false;
            for c in ligne.chars() {
                match c {
                    '\\' if dans_chaîne => échappé = !échappé,
                    '"' if !échappé => dans_chaîne = !dans_chaîne,
                    _ => {
                        if dans_chaîne {
                            affichés.push(c);
                        }
                        échappé = false;
                    }
                }
            }
        }
    }

    let mut manquants: std::collections::BTreeSet<char> = Default::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        // En chasse fixe, tous les caractères ont la même largeur : la
        // mesure n'y distingue rien. La famille proportionnelle partage
        // les mêmes polices de repli, et c'est donc elle qui répond.
        let font = egui::TextStyle::Body.resolve(ui.style());
        let largeur = |c: char| {
            ui.painter()
                .layout_no_wrap(c.to_string(), font.clone(), egui::Color32::WHITE)
                .rect
                .width()
        };
        // Un caractère de la zone privée Unicode : aucune police ne le
        // connaît, et tout ce qui mesure exactement sa largeur est rendu
        // par le même carré.
        let témoin = largeur('\u{e000}');
        for c in affichés.chars() {
            if c.is_ascii() || c.is_whitespace() {
                continue;
            }
            if largeur(c) == témoin {
                manquants.insert(c);
            }
        }
    });
    output.textures_delta.clear();

    assert!(
        manquants.is_empty(),
        "ces caractères seront peints en carré vide : {manquants:?}"
    );
}

/// Le rapport d'analyse s'écrit aussi sur le disque : un compte rendu se
/// garde et se relit, ce que le presse-papier ne permet pas.
#[test]
fn the_analysis_report_can_be_written_to_a_file() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("analyse.md");

    // Sans binaire, rien à écrire, et le message le dit plutôt que de
    // laisser un fichier vide derrière lui.
    let mut app = NamsdecApp::default();
    app.export_report(&path);
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
    assert!(!path.exists(), "aucun fichier n'a été créé");

    app.load_binary(bytes);
    app.export_report(&path);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    let texte = std::fs::read_to_string(&path).unwrap();
    assert!(texte.starts_with("# Analyse du binaire"));
    assert!(texte.contains("Fonctions importées"));
    assert!(texte.contains("malloc"), "avec ce qu'elles font");
}

/// Le listing d'un binaire du système nomme ce qu'il appelle.
///
/// Sans cela, lire /bin/ls revient à suivre des `call 0x…` vers des
/// adresses muettes. Les entrées de la table de liaison portent donc une
/// étiquette, comme n'importe quel symbole du programme.
#[test]
fn the_listing_labels_the_functions_a_binary_calls() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes);

    let state = app.assemble_state.as_ref().expect("chargé");
    let noms: Vec<&str> = state
        .output
        .symbols
        .iter()
        .map(|symbol| symbol.name.as_str())
        .collect();
    assert!(noms.contains(&"malloc@plt"), "{} symboles", noms.len());
    assert!(noms.contains(&"free@plt"));
    // Et elles sont dans le listing, là où l'œil les cherche.
    assert!(app.source.contains("malloc@plt:"), "l'étiquette est posée");

    // Chaque étiquette tombe sur une instruction du listing : une
    // étiquette entre deux lignes n'existerait nulle part.
    for symbol in &state.output.symbols {
        assert!(
            state.addr_to_line.contains_key(&symbol.addr),
            "{} pointe {:#x}, qui n'est aucune ligne",
            symbol.name,
            symbol.addr
        );
    }
}

/// Devant un `call`, le bandeau dit avec quoi : les registres de la
/// convention, leurs valeurs suivies comme des pointeurs, et — quand la
/// fonction appelée est connue — le nom de ses paramètres.
///
/// C'est le `dumpargs` de pwndbg, sous les yeux plutôt que sur commande :
/// « où sont les arguments ? » est la question qu'on se pose chaque fois
/// qu'on lit un appel.
#[test]
fn the_banner_says_what_a_call_is_called_with() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
default rel
global _start

section .data
message: db \"salut\", 0

section .text
_start:
    lea rdi, [message]
    mov rsi, 3
    call quelquepart
    hlt
quelquepart:
    ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    // Arrêté *sur* le `call`, pas après : c'est là que les arguments
    // sont en place.
    app.do_step();
    app.do_step();

    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).call_arguments),
        "{painted}"
    );
    assert!(painted.contains("RDI"), "les registres de la convention");
    assert!(
        painted.contains("\"salut\""),
        "et ce que le premier argument désigne : {painted}"
    );
}

/// Le motif cyclique répond à « de combien ça déborde » : on l'engendre,
/// on le donne au programme, et la valeur retrouvée dans un registre dit
/// le décalage.
#[test]
fn the_cyclic_pattern_answers_by_how_much() {
    let mut app = NamsdecApp {
        show_cyclic: true,
        ..Default::default()
    };

    let painted = paint(&mut app);
    assert!(
        painted.contains("aaaabaaacaaa"),
        "le motif est là : {painted}"
    );

    // La saisie se lit sous les trois formes qu'on a sous les yeux : le
    // nombre d'un registre, les caractères d'un vidage, le décimal.
    assert_eq!(cyclic_offset("0x6161616c"), Some(44));
    assert_eq!(cyclic_offset("laaa"), Some(44));
    assert_eq!(cyclic_offset(&format!("{}", 0x6161616cu64)), Some(44));
    // Et ce qui ne vient pas du motif n'obtient pas de décalage.
    assert_eq!(cyclic_offset("0xdeadbeef"), None);
    assert_eq!(cyclic_offset(""), None);

    // Le motif va dans l'entrée du programme sans passer par le
    // presse-papier : c'est là qu'il sert.
    app.cyclic_len = "24".to_owned();
    app.console_input.clear();
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).cyclic_to_input),
        "{painted}"
    );
}

/// Chercher en mémoire : « où est cette chaîne ? » ne se répond pas en
/// faisant défiler un vidage.
///
/// Les trois écritures sont celles qu'on a sous les yeux : un texte
/// qu'on vient d'écrire, des octets relevés dans un vidage, une valeur
/// lue dans un registre.
#[test]
fn memory_can_be_searched_the_three_ways_one_writes_a_needle() {
    // Le texte, tel quel.
    assert_eq!(SearchKind::Text.needle("salut"), Some(b"salut".to_vec()));
    // Les octets, avec ou sans espaces, avec ou sans 0x.
    assert_eq!(
        SearchKind::Bytes.needle("48 89 d8"),
        Some(vec![0x48, 0x89, 0xd8])
    );
    assert_eq!(SearchKind::Bytes.needle("0x4889"), Some(vec![0x48, 0x89]));
    // Un nombre impair de chiffres ne désigne pas des octets.
    assert_eq!(SearchKind::Bytes.needle("489"), None);
    assert_eq!(SearchKind::Bytes.needle("zz"), None);
    // Une valeur : petit-boutiste, sur ses octets utiles.
    assert_eq!(SearchKind::Value.needle("0x1234"), Some(vec![0x34, 0x12]));
    assert_eq!(SearchKind::Value.needle("255"), Some(vec![0xff]));
    // Rien à chercher n'est pas une recherche.
    assert_eq!(SearchKind::Text.needle("   "), None);

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start

section .data
message: db \"salut\", 0

section .text
_start:
    hlt
"
        .to_owned(),
        active_tab: SidebarTab::Memory,
        mem_search: "salut".to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Et la chaîne se trouve là où le programme l'a écrite.
    let emulator = app.emulator.as_ref().expect("chargé");
    let trouvés = namsdec_emu::telescope::search(&emulator.cpu().mem, b"salut", 24);
    assert_eq!(trouvés.len(), 1, "{trouvés:?}");
    let symbols = &app.assemble_state.as_ref().unwrap().output.symbols;
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "message" && s.addr == trouvés[0]),
        "l'adresse trouvée est celle de `message`"
    );
}

/// « Jusqu'au prochain appel », « jusqu'au prochain retour », « jusqu'au
/// prochain appel système » : les trois questions qu'on se pose en
/// explorant un programme qu'on n'a pas écrit.
///
/// Ce sont les `nextcall`, `nextret` et `nextsyscall` de pwndbg. Un pas
/// à pas de dix mille pas n'y répond pas.
#[test]
fn running_to_the_next_call_return_or_syscall() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start

section .text
_start:
    mov rax, 1
    mov rbx, 2
    call quelquepart
    mov rcx, 3
    hlt

quelquepart:
    mov rdx, 4
    ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let jusque_arrêt = |app: &mut NamsdecApp| {
        for _ in 0..100 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
    };

    // Depuis le début, le prochain appel est à deux instructions de là :
    // on s'arrête *sur* le `call`, pas après.
    app.dispatch(Action::RunToCall);
    jusque_arrêt(&mut app);
    let emulator = app.emulator.as_ref().expect("chargé");
    let rip = emulator.cpu().regs.rip;
    let texte = app
        .assemble_state
        .as_ref()
        .unwrap()
        .disasm
        .iter()
        .find(|i| i.address == rip)
        .map(|i| i.text.clone())
        .unwrap_or_default();
    assert!(texte.starts_with("call"), "arrêté sur « {texte} »");
    // Les deux `mov` d'avant ont bien été exécutés.
    assert_eq!(emulator.cpu().regs.get64(Reg64::Rbx), 2);

    // Puis jusqu'au retour : on entre dans la sous-routine et l'on
    // s'arrête sur son `ret`.
    app.dispatch(Action::RunToReturn);
    jusque_arrêt(&mut app);
    let emulator = app.emulator.as_ref().expect("chargé");
    assert_eq!(
        emulator.cpu().regs.get64(Reg64::Rdx),
        4,
        "la sous-routine a tourné"
    );

    // Et ce qui n'existe pas se dit, au lieu de laisser courir le
    // programme jusqu'à sa fin.
    app.dispatch(Action::RunToSyscall);
    assert!(
        matches!(app.status, Status::Note(_)),
        "aucun appel système ici : {:?}",
        app.status
    );
    assert!(!app.running);

    // Les points d'arrêt posés le temps de la course ne restent pas :
    // ils n'appartiennent pas à l'utilisateur.
    app.dispatch(Action::RunToCall);
    jusque_arrêt(&mut app);
    app.tick_run(&egui::Context::default());
    assert!(app.run_until.is_none(), "la course est finie");
    assert!(app.breakpoint_lines.is_empty(), "et rien n'a été posé");
}

/// Un registre se pose à la main, et un indicateur se bascule : « et si
/// RCX valait 1 ? » se répond en une seconde au lieu de se réécrire.
///
/// C'est le `set $rax = …` de gdb et le `setflag` de pwndbg. Le pas
/// suivant part de là, et la valeur posée entre dans l'histoire du
/// programme : revenir en arrière restaure ce qui était vrai avant
/// chaque pas, la main comprise.
#[test]
fn a_register_is_set_and_a_flag_is_flipped_by_hand() {
    // Ce qu'on tape : de l'hexadécimal, comme ce qu'on lit à l'écran.
    assert_eq!(parse_register_value("1f"), Some(0x1f));
    assert_eq!(parse_register_value("0x1F"), Some(0x1f));
    assert_eq!(parse_register_value("dead_beef"), Some(0xdeadbeef));
    // Un moins donne le complément à deux : « mets -1 » est la façon
    // dont on le pense.
    assert_eq!(parse_register_value("-1"), Some(u64::MAX));
    assert_eq!(parse_register_value(""), None);
    assert_eq!(parse_register_value("zz"), None);

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start

section .text
_start:
    mov rcx, 5
    dec rcx
    jz fini
    mov rax, 1
fini:
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // La boucle part de 5 ; on la pose à 1, et le `dec` suivant met ZF.
    app.do_step();
    app.set_register("RCX", 1);
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rcx),
        1
    );
    app.do_step(); // dec rcx -> 0, ZF posé
    assert!(
        app.emulator
            .as_ref()
            .unwrap()
            .cpu()
            .flags
            .contains(namsdec_core::RFlags::ZF),
        "le dec a bien vu 1"
    );

    // L'indicateur se bascule aussi : le saut lira ce qu'on a posé.
    app.toggle_flag(namsdec_core::RFlags::ZF);
    assert!(
        !app.emulator
            .as_ref()
            .unwrap()
            .cpu()
            .flags
            .contains(namsdec_core::RFlags::ZF)
    );
    app.do_step(); // jz : ZF est retombé, le saut n'est pas pris
    let rip = app.emulator.as_ref().unwrap().cpu().regs.rip;
    let texte = app
        .assemble_state
        .as_ref()
        .unwrap()
        .disasm
        .iter()
        .find(|i| i.address == rip)
        .map(|i| i.text.clone())
        .unwrap_or_default();
    assert!(
        texte.starts_with("mov"),
        "le saut ne devait pas être pris : « {texte} »"
    );

    // Et revenir en arrière restaure ce qui était vrai *avant chaque
    // pas*, la main comprise : la valeur posée fait désormais partie de
    // l'histoire du programme, elle ne s'évapore pas.
    app.do_step_back(); // défait le `jz`
    app.do_step_back(); // défait le `dec` : RCX retrouve le 1 posé
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rcx),
        1
    );
    app.do_step_back(); // défait le `mov rcx, 5` : on est avant tout
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rcx),
        0
    );
}

/// L'état du processeur se copie en texte : c'est ce qu'on colle pour
/// demander de l'aide, ou dans un compte rendu.
///
/// C'est le `context` de pwndbg. Un état qui ne sort pas de l'écran se
/// recopie à la main, donc mal.
#[test]
fn the_processor_state_can_be_copied_as_text() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp::default();
    // Sans processeur, rien à copier — et l'entrée de menu est grisée.
    assert!(app.context_text().is_none());

    app.source = "\
default rel
global _start

section .data
message: db \"salut\", 0

section .text
_start:
    lea rax, [message]
    push rax
    mov rbx, 7
    hlt
"
    .to_owned();
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();
    app.do_step();
    app.do_step();

    let contexte = app.context_text().expect("un état à copier");
    assert!(contexte.starts_with("namsdec "), "{contexte}");
    assert!(contexte.contains("RIP 0x"), "où l'on est : {contexte}");
    assert!(contexte.contains("RBX 0000000000000007"), "{contexte}");
    assert!(contexte.contains("ZF "), "les indicateurs : {contexte}");
    // La pile est suivie comme ailleurs : le mot empilé pointe la chaîne.
    assert!(contexte.contains("\"salut\""), "{contexte}");
}

/// On écrit en mémoire à la main, y compris là où le programme n'aurait
/// pas le droit — c'est ce que fait un débogueur —, et le programme,
/// lui, continue d'obéir aux règles.
#[test]
fn memory_is_written_by_hand_where_the_program_could_not() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
default rel
global _start

section .data
tampon: times 8 db 0

section .text
_start:
    mov al, [tampon]
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let tampon = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|s| s.name == "tampon")
        .expect("l'étiquette existe")
        .addr;

    // Ce qu'on écrit, le programme le lit.
    app.poke_memory(tampon, b"\x42");
    app.do_step();
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax) & 0xff,
        0x42
    );

    // Et l'on écrit même dans le code, que le programme ne pourrait pas
    // toucher : c'est l'observateur qui écrit, pas lui.
    let code = app.assemble_state.as_ref().unwrap().output.entry;
    app.poke_memory(code, &[0x90]);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    assert_eq!(
        app.emulator
            .as_ref()
            .unwrap()
            .cpu()
            .mem
            .peek_bytes(code, 1)
            .unwrap(),
        &[0x90]
    );

    // Hors de la mémoire simulée, en revanche, il n'y a rien à écrire.
    app.poke_memory(0xdead_beef, &[0]);
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
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
    let pack = namsdec_exercises::builtin_pack();
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

/// La pile dit toutes ses adresses de retour, pas seulement celles des
/// douze mots affichés : après un débordement, la question est
/// « laquelle a été écrasée ».
#[test]
fn the_stack_lists_every_return_address_it_carries() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start

section .text
_start:
    call une
    hlt
une:
    call deux
    ret
deux:
    ret
"
        .to_owned(),
        active_tab: SidebarTab::Stack,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step(); // call une
    app.do_step(); // call deux

    // Deux appels imbriqués : deux adresses de retour sur la pile, la
    // plus récente en premier, chacune nommée par l'étiquette de son
    // appelant.
    let emulator = app.emulator.as_ref().expect("chargé");
    let symbols = app.assemble_state.as_ref().unwrap().output.symbols.clone();
    let trouvées = return_addresses(emulator.cpu(), &symbols, 32);
    assert_eq!(trouvées.len(), 2, "{trouvées:?}");
    assert!(trouvées[0].2.starts_with("une"), "{trouvées:?}");
    assert!(trouvées[1].2.starts_with("_start"), "{trouvées:?}");
    assert!(trouvées[0].0 < trouvées[1].0, "de RSP vers le haut");

    // Écrasée par un débordement, elle disparaît de la liste : c'est
    // exactement ce qu'on vient y lire.
    app.poke_memory(trouvées[0].0, b"aaaabbbb");
    let emulator = app.emulator.as_ref().expect("chargé");
    let après = return_addresses(emulator.cpu(), &symbols, 32);
    assert_eq!(après.len(), 1, "{après:?}");
}

/// La palette promet « toutes les commandes » : elle les a toutes.
///
/// Une action qui n'y figure pas n'existe que pour qui connaît déjà son
/// raccourci — c'est-à-dire pour personne, le premier jour.
#[test]
fn the_palette_offers_every_action_there_is() {
    let s = strings(Lang::Fr);
    let commandes = palette_commands(s, true, true);
    for action in Action::ALL {
        assert!(
            commandes
                .iter()
                .any(|(_, command)| matches!(command, PaletteCommand::Do(a) if *a == action)),
            "{action:?} n'est proposée nulle part dans la palette"
        );
    }
    // Et chaque entrée porte un libellé : une ligne vide ne se cherche
    // pas.
    for (label, _) in &commandes {
        assert!(!label.trim().is_empty());
    }
}

/// Ce que la table annonce comme simulé, l'interpréteur le fait
/// vraiment — et ce qu'elle annonce comme non simulé, il le refuse.
///
/// Deux listes vivent dans deux crates : celle qui *nomme* les appels
/// et celle qui les *exécute*. Rien n'empêche l'une d'avancer sans
/// l'autre, sinon ceci — et l'écart se paierait d'un panneau qui
/// promet un appel qui s'arrête, ou qui décourage d'en tenter un qui
/// marche.
#[test]
fn what_the_table_calls_simulated_really_runs() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Un tampon pour ceux qui écrivent quelque part, et des arguments
    // inoffensifs : ce qu'on teste est le refus, pas le résultat.
    let programme = |numéro: u64| {
        format!(
            "\
default rel
global _start

section .bss
tampon: resb 64

section .text
_start:
    mov rax, {numéro}
    lea rdi, [tampon]
    lea rsi, [tampon]
    mov rdx, 8
    syscall
    hlt
"
        )
    };

    for numéro in 0u64..=400 {
        let Some(call) = namsdec_re::table::syscall(numéro) else {
            continue;
        };
        // `read` attend l'entrée, `exit` arrête : les deux répondent,
        // mais pas par un registre — leur cas est couvert ailleurs.
        if matches!(numéro, 0 | 60 | 231) {
            continue;
        }
        let mut app = NamsdecApp {
            source: programme(numéro),
            ..Default::default()
        };
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "{} : {:?}",
            call.name,
            app.status
        );
        app.do_step();
        app.do_step();
        app.do_step();
        app.do_step();
        app.do_step();
        let refusé = match &app.status {
            Status::Error(message) => message.contains(&numéro.to_string()),
            _ => false,
        };
        assert_eq!(
            refusé,
            !call.simulated,
            "{} ({numéro}) : la table dit simulé={}, l'interpréteur dit {}",
            call.name,
            call.simulated,
            if refusé { "non" } else { "oui" }
        );
    }
}

/// L'onglet Mémoire suit ce qu'on lui demande : la pile ou le code,
/// pas à pas, sans qu'on retape l'adresse à chaque instruction.
#[test]
fn the_memory_tab_follows_the_register_it_is_told_to() {
    // Alignée sur seize octets : un vidage qui danse d'une colonne à
    // l'autre à chaque pas ne se lit pas.
    let mut regs = namsdec_core::Registers::new();
    regs.set64(Reg64::Rsp, 0x40fef8);
    regs.rip = 0x400007;
    assert_eq!(MemFollow::Fixed.address(&regs), None);
    assert_eq!(MemFollow::Rsp.address(&regs), Some(0x40fef0));
    assert_eq!(MemFollow::Rip.address(&regs), Some(0x400000));

    // Et le choix survit au lancement suivant, comme tout réglage.
    let mut app = NamsdecApp {
        memory_follow: MemFollow::Rsp,
        ..Default::default()
    };
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(écrit.contains("\"memory_follow\":\"rsp\""), "{écrit}");
    app.memory_follow = MemFollow::by_id("rip");
    assert_eq!(app.memory_follow, MemFollow::Rip);
    // Un identifiant inconnu ne fait pas suivre n'importe quoi.
    assert_eq!(MemFollow::by_id("n'importe quoi"), MemFollow::Fixed);
}

/// La mémoire montre sa carte : où commence le code, où finit la pile,
/// ce que le tas occupe, avec les droits de chacun.
///
/// C'est le `vmmap` de pwndbg. Un refus d'accès n'apprend rien à qui ne
/// sait pas où il se trouvait ; la carte le dit avant.
#[test]
fn the_memory_tab_shows_the_map_of_the_process() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start

section .data
    tampon times 8 db 0

section .text
_start:
    mov rax, 1
    hlt
"
        .to_owned(),
        active_tab: SidebarTab::Memory,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Les zones existent et se lisent comme un vmmap : trois caractères
    // de droits par ligne.
    let emulator = app.emulator.as_ref().expect("chargé");
    let zones = emulator.cpu().mem.regions();
    assert!(zones.len() >= 3, "code, données, pile au moins : {zones:?}");
    let texte: Vec<String> = zones.iter().map(|z| perms_text(z.perms)).collect();
    assert!(texte.contains(&"r-x".to_owned()), "du code : {texte:?}");
    assert!(texte.contains(&"rw-".to_owned()), "des données : {texte:?}");
    // Les zones ne se chevauchent pas dans la carte, et sont triées.
    for paire in zones.windows(2) {
        assert!(
            paire[0].range.start <= paire[1].range.start,
            "la carte est triée : {zones:?}"
        );
    }

    // Et les outils qui la portent sont là, repliés : le vidage reste
    // ce qu'on voit en premier.
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).memory_tools),
        "{painted}"
    );
    assert!(
        painted.contains("_start"),
        "l'étiquette de l'adresse regardée reste visible : {painted}"
    );
}

/// La pile suit ses pointeurs : un mot qui pointe une chaîne le dit, un
/// mot qui pointe du code montre l'instruction.
///
/// C'est le geste qu'on fait sinon à la main, une lecture après l'autre —
/// celui que pwndbg appelle *telescope*. Un mot de pile n'est presque
/// jamais un nombre.
#[test]
fn the_stack_follows_its_pointers() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
default rel
global _start

section .data
message: db \"salut\", 0

section .text
_start:
    lea rax, [message]
    push rax
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();
    app.do_step();

    let painted = paint(&mut app);
    assert!(
        painted.contains("\"salut\""),
        "le mot empilé pointe une chaîne, et la pile le dit : {painted}"
    );

    // Et en mémoire, le format « Pointeurs » dit la même chose à partir
    // de n'importe quelle adresse.
    let emulator = app.emulator.as_ref().expect("chargé");
    let rsp = emulator.cpu().regs.get64(Reg64::Rsp);
    let dump = read_memory(
        &emulator.cpu().mem,
        rsp,
        MemoryFormat::Chain,
        "hors de la mémoire",
    );
    assert!(dump.contains("\"salut\""), "{dump}");
    assert_eq!(dump.lines().count(), 8, "huit mots, comme telescope");
}

/// Les fonctions qu'un binaire emprunte se lisent dans la fenêtre
/// d'analyse, rangées après les appels système : même question, un cran
/// plus haut — que demande-t-il à ce qui n'est pas lui ?
///
/// Ce que le module trouve est vérifié chez lui, sur un binaire du
/// système ; ici on vérifie qu'il est bien branché, et au bon endroit.
#[test]
fn what_a_binary_borrows_has_its_place_in_the_analysis() {
    let mut app = NamsdecApp::default();
    let titres: Vec<&str> = app
        .re_registry
        .iter_mut()
        .map(|module| module.title("fr"))
        .collect();
    let importées = titres
        .iter()
        .position(|titre| *titre == "Fonctions importées")
        .unwrap_or_else(|| panic!("le module manque : {titres:?}"));
    let syscalls = titres
        .iter()
        .position(|titre| titre.contains("ppels système"))
        .unwrap_or_else(|| panic!("celui des appels système manque : {titres:?}"));
    assert!(importées > syscalls, "{titres:?}");
}

/// Le pseudo-code doit couvrir ce qu'on lit vraiment dans un binaire.
///
/// Mesuré sur un binaire du système, pas sur des cas choisis : ce qui
/// n'est pas traduit tombe dans le repli NASM, ce qui reste honnête mais
/// n'apprend rien. Le seuil est large — il attrape une régression, pas
/// une variation de quelques instructions.
#[test]
fn the_pseudo_code_covers_a_real_binary() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes);
    let state = app.assemble_state.as_ref().expect("chargé");

    let mut total = 0usize;
    let mut traduites = 0usize;
    for instruction in &state.disasm {
        let Some(decoded) = namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
        else {
            continue;
        };
        total += 1;
        // Le repli rend la forme NASM telle quelle : c'est à cela qu'on
        // reconnaît une instruction qu'on ne sait pas dire.
        if namsdec_disasm::pseudo::of(&decoded, "fr") != namsdec_disasm::format_one(&decoded) {
            traduites += 1;
        }
    }
    assert!(total > 5_000, "le binaire doit être gros : {total}");
    let part = traduites as f32 / total as f32;
    assert!(
        part > 0.95,
        "seulement {traduites} instructions sur {total} se disent en clair ({:.0} %)",
        part * 100.0
    );
}

/// Le panneau de désassemblage se cherche et suit l'exécution.
#[test]
fn the_disassembly_panel_searches_and_follows() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_disassembly: true,
        source: "\
global _start
section .text
_start:
mov rax, 1
mov rbx, 2
call sous
hlt
sous:
mov rcx, 3
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| {
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
        painted
    };

    // Sans recherche, tout est là — en pseudo-code, et sur les
    // registres que nasm a réellement encodés : `mov rax, 1` tient en
    // cinq octets sous la forme `mov eax, 1`.
    let painted = peindre(&mut app);
    assert!(painted.contains("EAX = 1"), "{painted}");
    assert!(painted.contains("ECX = 3"), "{painted}");
    assert!(
        painted.contains("appeler"),
        "l'appel se dit en clair : {painted}"
    );

    // Une recherche filtre.
    app.disassembly_query = "ECX".to_owned();
    let painted = peindre(&mut app);
    assert!(painted.contains("ECX = 3"), "{painted}");
    // Le compte des lignes retenues, et la disparition de ce qui ne
    // correspond pas. (`EAX = 1` reste visible ailleurs : le bandeau du
    // panneau latéral dit l'instruction courante, filtre ou pas.)
    assert!(painted.contains("1 / 6"), "{painted}");
    assert!(
        !painted.contains("appeler"),
        "le `call` ne correspond pas à la recherche : {painted}"
    );
    // Et les octets sont là, à côté : c'est ce qui distingue un
    // désassemblage d'une traduction.
    assert!(painted.contains("b9 03 00 00 00"), "{painted}");

    // Chercher une étiquette la trouve, bien qu'elle ne soit pas dans le
    // texte de l'instruction.
    app.disassembly_query = "sous".to_owned();
    let painted = peindre(&mut app);
    assert!(painted.contains("sous:"), "{painted}");
}

/// Le bandeau dit aussi l'instruction en clair : c'est devant celle qui
/// va s'exécuter qu'on en a le plus besoin.
#[test]
fn the_next_instruction_is_also_said_plainly() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rcx, 5
add rcx, 2
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    app.do_step(); // arrêté sur le `add`

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
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

    // La forme machine, et la même chose en clair juste en dessous.
    assert!(painted.contains("add rcx, 2"), "{painted}");
    assert!(painted.contains("RCX = RCX + 2"), "{painted}");

    // Et pour ce qui se dit déjà tout seul, on ne répète pas.
    app.do_step(); // arrêté sur le `hlt`
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
    assert!(painted.contains("arrêter la machine"), "{painted}");
}

/// L'exemple de plugin donné dans le README doit être un plugin valide.
///
/// Un format documenté par un exemple faux coûte une soirée à qui le
/// recopie.
#[test]
fn the_plugin_example_in_the_readme_is_valid() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"))
        .expect("le README est à la racine");
    // Le bloc JSON qui suit la phrase d'introduction du format.
    let début = readme
        .find("\"id\": \"objdump\"")
        .expect("le README montre un exemple de plugin");
    let ouvrante = readme[..début].rfind('{').expect("son accolade");
    let fermante = readme[ouvrante..].find("\n}").expect("et sa fermeture") + ouvrante + 2;
    let exemple = &readme[ouvrante..fermante];

    let dossier = tempfile::tempdir().expect("dossier temporaire");
    let chemin = dossier.path().join("exemple.json");
    std::fs::write(&chemin, exemple).unwrap();
    let plugin = namsdec_disasm::plugin::read(&chemin)
        .unwrap_or_else(|e| panic!("l'exemple du README est refusé : {e}\n{exemple}"));
    assert_eq!(plugin.id, "objdump");
    assert!(plugin.args.iter().any(|arg| arg.contains("{file}")));
}

/// Le moteur choisi revient à la session suivante, et le panneau sait
/// aller à une fonction sans qu'on fasse défiler.
#[test]
fn the_disassembly_panel_remembers_and_navigates() {
    // Le réglage voyage comme les autres.
    let app = NamsdecApp {
        disassembler: NASM_ID.to_owned(),
        ..Default::default()
    };
    let json = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(json.contains("\"disassembler\":\"nasm\""), "{json}");
    let relu: crate::settings::Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(NamsdecApp::from_settings(relu).disassembler, NASM_ID);

    // Un fichier d'une version plus ancienne n'a pas le champ : on
    // revient au pseudo-code, celui qu'on lit en premier.
    let ancien: crate::settings::Settings =
        serde_json::from_str(r#"{"version":1,"theme":"vscode-dark","language":"fr"}"#).unwrap();
    assert_eq!(ancien.disassembler, PSEUDO_ID);

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Et la liste des étiquettes mène où on lui demande.
    let mut app = NamsdecApp {
        show_disassembly: true,
        source: format!(
            "global _start\nsection .text\n_start:\n{}call sous\nhlt\nsous:\nret\n",
            "nop\n".repeat(200)
        ),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let sous = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|s| s.name == "sous")
        .expect("l'étiquette existe")
        .addr;

    app.disassembly_target = Some(sous);
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
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
        painted.contains("sous:"),
        "la cible doit être à l'écran : {painted}"
    );
    assert!(
        painted.contains("revenir à l'appelant"),
        "et ce qu'elle contient : {painted}"
    );
}

/// Le pointeur posé dans le vide ne fait pas apparaître de fiche.
///
/// Le champ de texte occupe toute la hauteur de l'éditeur, bien au-delà
/// du code, et chaque ligne est plus courte que sa largeur. Le caractère
/// le plus proche — le dernier du fichier — se voyait attribuer le
/// survol, et sa fiche restait affichée sans qu'on comprenne d'où elle
/// venait.
#[test]
fn hovering_outside_the_code_explains_nothing() {
    let source = "global _start\nsection .text\n_start:\n    syscall\n";
    let peindre = |cible: egui::Vec2| -> String {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 820.0));
        let mut texts: Vec<(egui::Pos2, std::sync::Arc<egui::Galley>)> = Vec::new();
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
            if pointer.is_none()
                && let Some((pos, _)) = texts
                    .iter()
                    .rfind(|(_, g)| g.text().starts_with("global _start"))
            {
                pointer = Some(*pos + cible);
            }
            output.textures_delta.clear();
        }
        texts
            .iter()
            .map(|(_, g)| g.text())
            .collect::<Vec<_>>()
            .join("|")
    };

    // Sur le `syscall` de la quatrième ligne : la fiche est là.
    let sur = peindre(egui::vec2(40.0, 52.5));
    assert!(sur.contains("Appeler le système"), "{sur}");

    // Loin sous le code, là où il n'y a rien : rien non plus.
    let dessous = peindre(egui::vec2(40.0, 400.0));
    assert!(
        !dessous.contains("Appeler le système"),
        "le vide sous le code n'explique rien : {dessous}"
    );

    // À droite de la ligne, au-delà de son dernier caractère : rien.
    let à_droite = peindre(egui::vec2(600.0, 52.5));
    assert!(
        !à_droite.contains("Appeler le système"),
        "après la fin de la ligne non plus : {à_droite}"
    );
}

/// Et la fiche au survol s'éteint dans les Préférences, séparément des
/// infobulles ordinaires : elle est grande, et s'ouvre pendant qu'on
/// écrit.
#[test]
fn the_hover_sheet_can_be_turned_off_on_its_own() {
    let app = NamsdecApp {
        hover_help: false,
        ..Default::default()
    };
    assert!(app.show_tooltips, "les autres infobulles restent");
    let json = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(json.contains("\"hover_help\":false"), "{json}");
    let relu: crate::settings::Settings = serde_json::from_str(&json).unwrap();
    assert!(!NamsdecApp::from_settings(relu).hover_help);

    // Absent d'un fichier plus ancien : allumée, comme elle l'était.
    let ancien: crate::settings::Settings =
        serde_json::from_str(r#"{"version":1,"theme":"vscode-dark","language":"fr"}"#).unwrap();
    assert!(ancien.hover_help);
}

/// Le manuel explique aussi ses propres mots.
///
/// Un manuel d'instructions qui ne dit pas ce qu'est un registre, une
/// section ou un appel système laisse le lecteur les deviner — or ce
/// sont eux que toutes les autres fiches emploient.
#[test]
fn the_manual_explains_its_own_words() {
    for mot in [
        "registre",
        "indicateur",
        "pile",
        "tas",
        "etiquette",
        "convention",
        "adressage",
        "immediat",
        "elf",
    ] {
        let entry = namsdec_doc::lookup(mot).unwrap_or_else(|| panic!("« {mot} » doit être dit"));
        assert_eq!(
            entry.kind,
            namsdec_doc::Kind::Concept,
            "« {mot} » est une notion"
        );
        assert!(entry.body.fr.len() > 150, "« {mot} » : {}", entry.body.fr);
        assert!(entry.body.en.len() > 150, "« {mot} »");
    }

    // Une notion qui porte le nom d'une directive ne la remplace pas :
    // F1 sur `section` donne la directive, celle qu'on écrit.
    assert_eq!(
        namsdec_doc::lookup("section").map(|e| e.kind),
        Some(namsdec_doc::Kind::Directive)
    );
    assert_eq!(
        namsdec_doc::lookup("notion-section").map(|e| e.kind),
        Some(namsdec_doc::Kind::Concept)
    );

    // Et la recherche du manuel trouve les deux : c'est là qu'on
    // cherche un mot sans savoir ce qu'il est.
    let trouvées = namsdec_doc::all()
        .iter()
        .filter(|entry| {
            entry.names.iter().any(|n| n.contains("section"))
                || entry.title.get("fr").to_lowercase().contains("section")
        })
        .count();
    assert!(trouvées >= 2, "la directive et la notion : {trouvées}");
}

/// Les directives que NASM documente doivent toutes se retrouver ici.
///
/// La liste vient du manuel de NASM 3.02 : ce qu'on peut écrire dans un
/// source et que namsdec ne saurait pas expliquer est un trou, et un
/// trou qu'on ne découvre qu'en tombant dedans.
#[test]
fn every_nasm_directive_has_a_sheet() {
    // Les directives de l'assembleur, puis celles du préprocesseur, puis
    // les mots qui n'en sont ni l'un ni l'autre mais qu'on écrit tout
    // autant : tailles, opérateurs d'adresse, symboles de position.
    let attendues = [
        "bits",
        "use16",
        "use32",
        "use64",
        "default",
        "section",
        "segment",
        "absolute",
        "extern",
        "required",
        "global",
        "common",
        "static",
        "prefix",
        "suffix",
        "cpu",
        "float",
        "warning",
        "list",
        "org",
        "dollarhex",
        "%define",
        "%idefine",
        "%xdefine",
        "%defstr",
        "%deftok",
        "%defalias",
        "%undef",
        "%undefalias",
        "%macro",
        "%imacro",
        "%unmacro",
        "%assign",
        "%clear",
        "%endmacro",
        "%exitmacro",
        "%rotate",
        "%rep",
        "%endrep",
        "%exitrep",
        "%if",
        "%ifdef",
        "%ifmacro",
        "%ifctx",
        "%ifidn",
        "%ifid",
        "%ifnum",
        "%ifstr",
        "%iftoken",
        "%ifempty",
        "%elif",
        "%else",
        "%endif",
        "%push",
        "%pop",
        "%repl",
        "%include",
        "%pathsearch",
        "%depend",
        "%use",
        "%strcat",
        "%strlen",
        "%substr",
        "%arg",
        "%stacksize",
        "%local",
        "%error",
        "%warning",
        "%fatal",
        "%note",
        "%pragma",
        "%line",
        "%!",
        "struc",
        "endstruc",
        "istruc",
        "at",
        "iend",
        "align",
        "alignb",
        "incbin",
        "equ",
        "times",
        "db",
        "dw",
        "dd",
        "dq",
        "byte",
        "word",
        "dword",
        "qword",
        "rel",
        "abs",
        "wrt",
        "seg",
        "strict",
        "$",
        "$$",
        // Chapitre 6 : ce que NASM définit de lui-même avant la
        // première ligne du source.
        "__?LINE?__",
        "__?FILE?__",
        "__?BITS?__",
        "__?DATE?__",
        "__?TIME?__",
        "__?NASM_VERSION_ID?__",
        "__?OUTPUT_FORMAT?__",
        // Chapitre 7 : les paquets que `%use` charge sans fichier.
        "altreg",
        "smartalign",
        "fp",
        "ifunc",
        "masm",
    ];
    let manquantes: Vec<&str> = attendues
        .into_iter()
        .filter(|mot| namsdec_doc::lookup(mot).is_none())
        .collect();
    assert!(
        manquantes.is_empty(),
        "ces mots s'écrivent dans un source sans que namsdec sache les expliquer : {manquantes:?}"
    );
}
