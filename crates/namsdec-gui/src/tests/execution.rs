//! L'exécution vue de l'interface : avancer, revenir, s'arrêter.
//!
//! Le pas à pas, les points d'arrêt et les surveillances, la chronologie,
//! le profil et la gouttière, et le chargement d'un vrai binaire jusqu'à ce
//! qu'il tourne. L'état du processeur est dans [`super::processeur`], la
//! mémoire dans [`super::memoire`].
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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
    app.load_binary(std::fs::read(&program).unwrap(), "programme");

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
            egui::vec2(1280.0, 1400.0),
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
            // Le lint parle la langue de l'application : « inscriptible »
            // en français, « not writable » en anglais.
            .any(|d| d.message.contains("inscriptible") || d.message.contains("not writable")),
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

    // Et la gouttière le peint : une bande par ligne parcourue, d'autant
    // plus chaude qu'elle a tourné souvent. Un nombre par ligne se lit
    // une ligne à la fois ; une bande de couleur se lit d'un coup.
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let mut bandes: Vec<egui::Color32> = Vec::new();
    for _ in 0..2 {
        bandes.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Rect(forme) = &clipped.shape
                && (forme.rect.width() - HEAT_WIDTH).abs() < 0.5
            {
                bandes.push(forme.fill);
            }
        }
        output.textures_delta.clear();
    }
    assert_eq!(
        bandes.len(),
        app.line_visits().len(),
        "une bande par ligne parcourue, et pas une de plus"
    );
    // La ligne du corps de boucle est plus chaude que celle qui n'a
    // tourné qu'une fois : c'est tout ce que la couleur doit dire.
    let froide = heat_colour(app.theme, 1, 5);
    let chaude = heat_colour(app.theme, 5, 5);
    assert_ne!(froide, chaude);
    assert!(bandes.contains(&froide), "{bandes:?}");
    assert!(bandes.contains(&chaude), "{bandes:?}");
    // Une ligne jamais atteinte n'a pas de bande : l'absence dit
    // quelque chose, elle aussi.
    assert_eq!(heat_colour(app.theme, 0, 5), heat_colour(app.theme, 0, 5));
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
            egui::vec2(1280.0, 1400.0),
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
    assert!(painted.contains("Chronologie"), "{painted}");
    assert!(
        painted.contains("-1|"),
        "le dernier pas est « -1 » : {painted}"
    );
    assert!(painted.contains("-2|"), "{painted}");
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
    assert!(painted.contains("Seulement appels"), "{painted}");
    // Le recul affiché pour le `call` : total - 3.
    assert!(
        painted.contains(&format!("-{}", total - 3)),
        "le rang affiché doit être celui du chemin : {painted}"
    );
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

/// Tous les binaires du dossier d'épreuve se chargent et tournent.
///
/// Le dossier `tests/binaires/` est vide dans le dépôt : chacun y dépose
/// ce qu'il veut voir tenir — crackmes, exercices, programmes tordus. Le
/// test ne juge pas ce qu'ils font ; il vérifie que namsdec sait les lire
/// sans se rompre.
///
/// Un programme qui s'arrête sur un appel système non simulé n'est pas un
/// échec : c'est exactement ce que namsdec doit dire plutôt que de partir
/// dans le décor. Ce qu'on refuse, c'est la panique, le désassemblage
/// vide, et le point d'entrée qui ne mène nulle part.
#[test]
fn every_binary_here_loads_and_runs() {
    let Some(dossier) = dossier_dépreuve() else {
        return;
    };
    let Ok(entrées) = std::fs::read_dir(&dossier) else {
        return;
    };

    let mut vus = 0;
    for entrée in entrées.filter_map(Result::ok) {
        let chemin = entrée.path();
        // Le mode d'emploi et le .gitignore ne sont pas des programmes.
        if !chemin.is_file()
            || chemin
                .extension()
                .is_some_and(|e| e == "md" || e == "txt" || e == "gitignore")
            || chemin.file_name().is_some_and(|n| n == ".gitignore")
        {
            continue;
        }
        let Ok(octets) = std::fs::read(&chemin) else {
            continue;
        };
        let nom = chemin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let mut app = NamsdecApp::default();
        let chargé = app.load_elf(&octets, &nom);
        // Ce qui n'est pas un ELF chargeable doit être refusé sans
        // détour : c'est une réponse, pas une panne.
        if !chargé {
            vus += 1;
            continue;
        }
        vus += 1;

        // Le désassemblage a produit quelque chose.
        let état = app.assemble_state.as_ref().expect("un état d'assemblage");
        assert!(
            !état.disasm.is_empty(),
            "{nom} : chargé, mais rien de désassemblé"
        );

        // Le point d'entrée est une instruction qu'on sait nommer.
        let emulator = app.emulator.as_ref().expect("un émulateur");
        let rip = emulator.cpu().regs.rip;
        assert!(
            état.disasm.iter().any(|i| i.address == rip),
            "{nom} : le point d'entrée {rip:#x} n'est pas une instruction connue"
        );

        // Et il tourne : soit il avance, soit il s'arrête en le disant.
        app.do_run();
        for _ in 0..3000 {
            if !app.running {
                break;
            }
            app.tick_run(&egui::Context::default());
        }
        let raisonnable = matches!(
            app.status,
            Status::Exited { .. }
                | Status::WaitingForInput
                | Status::Running
                | Status::StepLimit
                | Status::Idle
                | Status::Error(_)
                | Status::Note(_)
                | Status::Breakpoint(_)
        );
        assert!(
            raisonnable,
            "{nom} : namsdec ne sait pas dire où il en est — {:?}",
            app.status
        );
    }
    eprintln!(
        "{vus} binaire(s) passés en revue dans {}",
        dossier.display()
    );
}

/// La légende dit ce que valent les couleurs de la gouttière, et elle
/// se coupe dans les préférences.
///
/// Une bande de couleur sans échelle ne dit rien : « plus rouge » ne
/// devient « mille passages » que si quelque chose l'écrit.
#[test]
fn the_heat_legend_says_what_the_colours_are_worth() {
    // Les repères suivent l'échelle logarithmique de la barre : entre
    // un passage et mille, ce sont les ordres de grandeur qui comptent.
    assert_eq!(heat_legend_steps(1), vec![1]);
    assert_eq!(heat_legend_steps(4), vec![1, 2, 3, 4]);
    let repères = heat_legend_steps(1000);
    assert_eq!(repères.first(), Some(&1));
    assert_eq!(repères.last(), Some(&1000));
    assert!(repères.len() <= 5, "{repères:?}");
    assert!(
        repères.windows(2).all(|p| p[0] < p[1]),
        "les repères montent : {repères:?}"
    );
    // Et ils ne se répètent pas quand le maximum est petit.
    let courts = heat_legend_steps(6);
    assert_eq!(
        courts.len(),
        courts
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "{courts:?}"
    );

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
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
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    // Allumée : la légende est sous l'éditeur, avec ses valeurs.
    assert!(app.heat_legend, "allumée par défaut");
    let painted = paint(&mut app);
    let s = strings(app.lang);
    assert!(painted.contains(s.heat_legend), "{painted}");
    assert!(painted.contains(s.heat_legend_hint), "{painted}");

    // Éteinte : plus rien, et le réglage se retrouve au lancement
    // suivant.
    app.heat_legend = false;
    let painted = paint(&mut app);
    assert!(!painted.contains(s.heat_legend), "{painted}");
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(écrit.contains("\"heat_legend\":false"), "{écrit}");
}

/// Le classement du profil se parcourt, et dit combien il en reste.
///
/// « Je ne sais pas me déplacer dans profil (voir les lignes) », d'une
/// capture du 30 septembre. Le classement d'un programme qui boucle
/// porte des dizaines de rangées ; la console en montre douze, et rien
/// ne disait qu'il y en avait d'autres — ni combien, ni comment y
/// aller. Une liste qu'on ne sait pas parcourir vaut sa première page.
///
/// Le contrôle mesure les deux : le compte est écrit, et la molette
/// change vraiment ce qui est peint.
#[test]
fn the_profile_ranking_can_be_walked_through() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Les numéros de ligne que le CLASSEMENT montre, et non ceux que
    // l'écran porte : la gouttière de l'éditeur en écrit un par ligne du
    // source, et un `contains` sur le tout ne distinguerait pas les deux.
    // Le classement cale ses numéros à droite dans une colonne large,
    // donc ses cellules portent des espaces ; la gouttière écrit le
    // nombre nu.
    fn rangées_du_classement(peint: &str) -> Vec<u32> {
        peint
            .split('|')
            .filter(|cellule| cellule.contains(' '))
            .filter_map(|cellule| cellule.trim().parse::<u32>().ok())
            .collect()
    }

    // Un programme dont le classement dépasse ce qu'une console montre :
    // quarante lignes distinctes, toutes exécutées.
    let mut source = String::from("global _start\nsection .text\n_start:\n");
    for i in 0..40 {
        source.push_str(&format!("    mov rax, {i}\n"));
    }
    source.push_str("    hlt\n");
    let mut app = NamsdecApp {
        source,
        console_tab: ConsoleTab::Profile,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    let ctx_run = egui::Context::default();
    for _ in 0..500 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx_run);
    }
    let classées = app.line_visits().len();
    assert!(classées >= 40, "{classées} lignes classées");

    let ctx = egui::Context::default();
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));
    let peindre = |ctx: &egui::Context, app: &mut NamsdecApp, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(écran),
            events,
            ..Default::default()
        };
        let mut sortie = ctx.run_ui(input, |ui| app.draw(ui));
        let mut peint = String::new();
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                peint.push_str(texte.galley.text());
                peint.push('|');
            }
        }
        sortie.textures_delta.clear();
        peint
    };

    let mut peint = String::new();
    for _ in 0..3 {
        peint = peindre(&ctx, &mut app, Vec::new());
    }

    // Combien il y en a : sans ce chiffre, on ne sait pas si la liste est
    // finie ou si elle continue sous le bord.
    let s = strings(Lang::Fr);
    assert!(
        peint.contains(&format!("{classées} {}", s.profile_lines_ranked)),
        "le nombre de lignes classées est écrit :\n{peint}"
    );

    // La liste ne tient pas d'un coup — sinon il n'y a rien à parcourir,
    // et le contrôle ne prouverait rien.
    let visibles = rangées_du_classement(&peint);
    assert!(
        !visibles.is_empty() && visibles.len() < classées,
        "{} rangées visibles sur {classées} :\n{peint}",
        visibles.len()
    );
    let dernière = *app.line_visits().keys().max().expect("une ligne");
    assert!(
        !visibles.contains(&dernière),
        "la fin du classement ne tient pas d'emblée : {visibles:?}"
    );

    // Et la molette l'atteint. C'est la seconde moitié du grief : savoir
    // qu'il y a une suite ne sert à rien si l'on ne peut pas y aller.
    let mut atteinte = false;
    for _ in 0..12 {
        let peint = peindre(
            &ctx,
            &mut app,
            vec![
                // Le pointeur DANS la liste, sans quoi la molette
                // s'applique à ce qui est sous elle — l'éditeur — et le
                // classement ne bouge pas d'un pixel.
                egui::Event::PointerMoved(egui::pos2(351.0, 1250.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    source: egui::MouseWheelSource::Wheel,
                    delta: egui::vec2(0.0, -200.0),
                    modifiers: egui::Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                },
            ],
        );
        if rangées_du_classement(&peint).contains(&dernière) {
            atteinte = true;
            break;
        }
    }
    assert!(
        atteinte,
        "douze tours de molette doivent atteindre la fin du classement"
    );
}

/// Un point d'arrêt qui regarde dans la mémoire, et non dans un
/// registre.
///
/// C'est la condition qu'on veut devant un contrôle de mot de passe :
/// l'intéressant n'est pas dans un registre, il est à l'adresse qu'un
/// registre désigne. Le programme ci-dessous écrit les lettres de
/// `NAMSDEC` une par une sur la pile ; on demande à s'arrêter au moment
/// où c'est le `S` qui vient d'être posé.
#[test]
fn a_breakpoint_can_watch_what_is_in_memory() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
mot db \"NAMSDEC\"
section .text
_start:
sub rsp, 16
xor rcx, rcx
boucle:
mov al, [mot + rcx]
mov [rsp], al
inc rcx
cmp rcx, 7
jne boucle
add rsp, 16
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Sur la ligne qui incrémente le compteur : l'octet est alors posé.
    let ligne = app
        .source
        .lines()
        .position(|l| l.trim() == "inc rcx")
        .map(|i| i as u32 + 1)
        .expect("la ligne existe");
    app.breakpoint_lines.insert(ligne);
    app.breakpoint_conditions
        .insert(ligne, "byte [rsp] == 'S'".to_owned());
    app.sync_breakpoints();

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..40 {
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
    // `NAMSDEC` : le S est la quatrième lettre, donc RCX vaut 3 quand on
    // vient de la poser.
    let rcx = app
        .emulator
        .as_ref()
        .unwrap()
        .cpu()
        .regs
        .get64(namsdec_core::Reg64::Rcx);
    assert_eq!(rcx, 3, "au tour du S, et pas au premier");
}

/// La chaleur va du vert au rouge en passant par le jaune et l'orange :
/// la rampe qu'on lit sans légende.
#[test]
fn the_heat_ramp_runs_green_to_red() {
    let theme = DEFAULT_THEME;
    // Une ligne parcourue une fois sur mille est froide, la plus chaude
    // est rouge, et le milieu passe par le jaune puis l'orange.
    let froid = heat_colour(theme, 1, 1000);
    let chaud = heat_colour(theme, 1000, 1000);
    assert_eq!(froid, theme.good, "le froid est le vert du thème");
    assert_eq!(chaud, theme.bad, "le chaud est le rouge du thème");

    // Le rouge monte, le vert descend : c'est ce qui fait une rampe.
    let étapes: Vec<egui::Color32> = [1u64, 4, 30, 200, 1000]
        .into_iter()
        .map(|n| heat_colour(theme, n, 1000))
        .collect();
    // Ce qui fait une rampe du vert au rouge : la part de rouge par
    // rapport au vert ne cesse de monter. Le bleu, lui, ne se compare
    // pas — le rouge d'un thème sombre en contient toujours un peu,
    // sans quoi il piquerait les yeux.
    let chaleurs: Vec<f32> = étapes
        .iter()
        .map(|c| c.r() as f32 / c.g().max(1) as f32)
        .collect();
    assert!(
        chaleurs.windows(2).all(|p| p[0] <= p[1] + 0.01),
        "la rampe doit se réchauffer : {chaleurs:?}"
    );
    // Au milieu, du jaune puis de l'orange : beaucoup de rouge, du vert
    // encore, et moins de bleu qu'aux deux bouts.
    let milieu = heat_colour(theme, 30, 1000);
    assert!(milieu.r() > 180 && milieu.g() > 100, "{milieu:?}");
    assert!(milieu.b() < froid.b(), "{milieu:?} contre {froid:?}");

    // Et une ligne jamais atteinte n'a pas de couleur du tout : elle n'a
    // pas de bande.
    assert_eq!(heat_colour(theme, 0, 0), heat_colour(theme, 0, 0));
}

/// Le profil classe les lignes par temps passé dedans.
///
/// La bande de chaleur dit « ça chauffe par là » ; elle ne dit pas
/// laquelle des trois lignes rouges coûte le double des deux autres.
/// C'est ce que ce classement doit répondre — et il doit répondre en
/// pourcentages qui font cent, sinon il ment sur la part de chacune.
#[test]
fn the_profile_ranks_the_lines_by_where_the_time_went() {
    let source = "\
section .text
global _start
_start:
    mov rcx, 10
boucle:
    dec rcx
    jnz boucle
    mov rax, 60
    xor rdi, rdi
    syscall
";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let passages = app.line_visits();
    let (corps, saut) = (passages[&6], passages[&7]);
    assert_eq!(corps, 10, "le corps de la boucle tourne dix fois");
    assert_eq!(saut, 10, "et le saut qui la referme autant");
    assert_eq!(passages[&4], 1, "l'initialisation, une seule fois");

    // Le classement : le plus chaud d'abord, et les parts font cent.
    let mut classement: Vec<(u32, u64)> = passages.iter().map(|(l, c)| (*l, *c)).collect();
    classement.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    assert_eq!(
        classement[0].0, 6,
        "la ligne la plus chaude arrive en tête, obtenu : {classement:?}"
    );
    let total: u64 = classement.iter().map(|(_, c)| *c).sum();

    let texte = app.profile_text(&classement, total);
    // Le texte copié porte la même information que l'écran : le total,
    // le numéro de ligne, la part, et la ligne elle-même.
    assert!(texte.contains("instructions exécutées"), "{texte}");
    assert!(texte.contains("dec rcx"), "{texte}");
    assert!(texte.contains("jnz boucle"), "{texte}");
    let parts: Vec<f32> = texte
        .lines()
        .skip(1)
        .filter_map(|l| l.split_whitespace().nth(2))
        .filter_map(|n| n.replace(',', ".").parse::<f32>().ok())
        .collect();
    let somme: f32 = parts.iter().sum();
    assert!(
        (somme - 100.0).abs() < 0.5,
        "les parts doivent faire cent, obtenu {somme} depuis {parts:?}"
    );
    // Une ligne jamais atteinte n'a pas de part : `syscall` de sortie
    // s'exécute, mais rien après.
    assert!(
        !passages.contains_key(&11),
        "rien ne tourne après la fin du programme"
    );
}

/// Le profil s'affiche dans la console et emmène à la ligne d'un clic.
#[test]
fn the_profile_tab_shows_the_hot_lines_and_leads_to_them() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    mov rax, 60
    xor rdi, rdi
    syscall
"
        .to_owned(),
        console_tab: ConsoleTab::Profile,
        show_console: true,
        ..Default::default()
    };
    app.do_assemble();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

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
    for attendu in ["Profil", "instructions exécutées", "dec rcx", "jnz boucle"] {
        assert!(
            peint.contains(attendu),
            "« {attendu} » manque dans :\n{peint}"
        );
    }

    // Un profil vide le dit au lieu de montrer un tableau vide.
    let mut vierge = NamsdecApp {
        console_tab: ConsoleTab::Profile,
        ..Default::default()
    };
    let mut vide = String::new();
    let mut output = ctx.run_ui(input, |ui| vierge.console_panel_ui(ui));
    for clipped in &output.shapes {
        if let egui::epaint::Shape::Text(text) = &clipped.shape {
            vide.push_str(text.galley.text());
        }
    }
    output.textures_delta.clear();
    assert!(
        vide.contains("Rien n'a encore tourné"),
        "le profil vide doit s'expliquer, obtenu :\n{vide}"
    );
}

/// Le NUMÉRO DE LIGNE du classement mène à la ligne, lui aussi.
///
/// Seul le texte de la ligne y menait — tout à droite de la rangée, et
/// tronqué. On clique « 7 », qui est la réponse à « où ça ? », et il ne
/// se passait rien : le classement passait pour mort. Signalé le
/// 20 octobre, avec le reste de ce qui ne se clique pas.
///
/// Le contrôle clique le numéro à l'endroit où il est PEINT, et non une
/// case devinée : le classement est une grille, et la place d'une
/// colonne s'y mesure.
#[test]
fn the_line_number_of_the_profile_leads_to_its_line() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    mov rax, 60
    xor rdi, rdi
    syscall
"
        .to_owned(),
        console_tab: ConsoleTab::Profile,
        show_console: true,
        ..Default::default()
    };
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Le style de l'application AVANT toute mesure : une galère mesurée
    // sous la police d'egui n'a pas les dimensions de celle qu'on peint.
    app.apply_theme(&ctx);
    app.do_assemble();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };

    // Où le premier numéro de ligne du classement est peint. Les
    // numéros sont calés à droite dans leur colonne, donc entourés
    // d'espaces : on cherche un texte qui, une fois rogné, est un
    // nombre, et qui n'est ni un compte ni un pourcentage — les deux
    // autres colonnes vivent plus à droite.
    let mut où: Option<egui::Pos2> = None;
    for _ in 0..2 {
        où = None;
        let mut output = ctx.run_ui(input(), |ui| app.console_panel_ui(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let texte = t.galley.text().trim().to_owned();
                if texte.parse::<u32>().is_ok() && où.is_none() {
                    let rect = cadre_du_texte(t);
                    let vu = rect.intersect(clipped.clip_rect);
                    if vu.is_positive() {
                        où = Some(vu.center());
                    }
                }
            }
        }
        output.textures_delta.clear();
    }
    // UN NUMÉRO DE LIGNE EST PEINT, ou c'est un défaut : le profil
    // vient d'être rempli, et c'est son numéro qu'on s'apprête à
    // cliquer. S'abstenir ici, c'est se taire le jour où la colonne des
    // numéros disparaît du classement.
    let où = où.expect(
        "aucun numéro de ligne visible dans le classement du profil — \
         la colonne a disparu, ou rien n'a été classé",
    );

    app.goto_line = None;
    for étape in 0..4 {
        let mut entrée = input();
        match étape {
            1 => {
                entrée.events.push(egui::Event::PointerMoved(où));
                entrée.events.push(egui::Event::PointerButton {
                    pos: où,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            2 => entrée.events.push(egui::Event::PointerButton {
                pos: où,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }),
            _ => {}
        }
        let mut output = ctx.run_ui(entrée, |ui| app.console_panel_ui(ui));
        output.textures_delta.clear();
    }
    assert!(
        app.goto_line.is_some(),
        "cliquer le numéro de ligne du classement doit mener à cette ligne"
    );
}

/// Le profil situe chaque ligne dans son bloc.
///
/// « ligne 6 » oblige à retourner dans le programme pour savoir de quoi
/// il s'agit ; « boucle: dec rcx » le dit sur place. L'étiquette est la
/// plus proche au-dessus, et une étiquette locale — celles qui
/// commencent par un point — ne compte pas : elle ne veut rien dire
/// hors de son bloc.
#[test]
fn the_profile_says_which_block_a_line_belongs_to() {
    let source = "\
section .text
global _start
_start:
    mov rcx, 2
boucle:
    dec rcx
.suite:
    jnz boucle
    hlt
";
    assert_eq!(enclosing_label(source, 4).as_deref(), Some("_start"));
    assert_eq!(enclosing_label(source, 6).as_deref(), Some("boucle"));
    assert_eq!(
        enclosing_label(source, 8).as_deref(),
        Some("boucle"),
        "une étiquette locale ne prend pas la place de son bloc"
    );
    assert_eq!(
        enclosing_label(source, 1),
        None,
        "avant la première étiquette, il n'y a rien à nommer"
    );
    // Une étiquette indentée n'existe pas en NASM : elle ne doit pas
    // être prise pour le nom d'un bloc.
    assert_eq!(
        enclosing_label("_start:\n    ceci: nop\n", 2).as_deref(),
        Some("_start")
    );

    // Et le texte copié le porte aussi.
    let mut app = NamsdecApp {
        source: source.to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    let passages = app.line_visits();
    let mut classement: Vec<(u32, u64)> = passages.iter().map(|(l, c)| (*l, *c)).collect();
    classement.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let total: u64 = classement.iter().map(|(_, c)| *c).sum();
    let texte = app.profile_text(&classement, total);
    assert!(
        texte.contains("boucle: dec rcx"),
        "le profil copié situe la ligne dans son bloc :\n{texte}"
    );
}

/// Un appel système que namsdec ne simule pas se dit à l'assemblage.
///
/// `execve`, `socket`, `connect` : namsdec les nomme mais ne les fait
/// pas — il n'a ni second processus ni réseau à offrir. Un programme
/// qui en contient un s'arrête dessus, et rien dans le code ne le
/// laisse deviner. Le dire avant, avec sa ligne, vaut mieux que de le
/// découvrir au milieu d'une exécution.
///
/// `open` était l'exemple de ce contrôle, et ne l'est plus : il REPOND
/// « introuvable » depuis le 28 octobre 2026, comme `fopen`. Un appel
/// qui répond n'a rien à annoncer — c'est la console qui le dit, à
/// l'exécution et une seule fois.
#[test]
fn an_unsimulated_syscall_is_announced_before_it_stops_the_program() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n\
                 \x20   mov rax, 59\n    xor rdi, rdi\n    xor rsi, rsi\n\
                 \x20   xor rdx, rdx\n    syscall\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let dit: Vec<&namsdec_asm::Diagnostic> = app
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("execve"))
        .collect();
    assert_eq!(
        dit.len(),
        1,
        "execve doit être annoncé une fois : {:?}",
        app.diagnostics
    );
    assert_eq!(
        dit[0].severity,
        Severity::Warning,
        "un avertissement, pas une erreur"
    );
    assert_eq!(
        dit[0].line, 8,
        "et la ligne du syscall, pour y aller d'un clic"
    );

    // Un programme qui ne fait qu'écrire et sortir n'a rien à s'entendre
    // dire : `write` et `exit` sont exécutés pour de vrai.
    let mut sage = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    sage.do_assemble();
    assert!(
        !sage
            .diagnostics
            .iter()
            .any(|d| d.message.contains("ne simule pas")),
        "rien à signaler ici : {:?}",
        sage.diagnostics
    );
}

/// Les colonnes du profil tombent sous leur en-tête.
///
/// « ligne passages part » écrit d'un seul trait, avec des espaces qui
/// ne mesurent rien, ne surmonte aucune colonne : les titres flottaient
/// à gauche pendant que les nombres se rangeaient ailleurs. Une grille
/// aligne les uns sous les autres, et les nombres sont calés à droite —
/// c'est ainsi que des chiffres se comparent d'une ligne à l'autre.
#[test]
fn the_profile_columns_line_up_under_their_headings() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    mov rax, 60
    xor rdi, rdi
    syscall
"
        .to_owned(),
        console_tab: ConsoleTab::Profile,
        show_console: true,
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

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    // Chaque texte peint, avec son bord droit : c'est lui qui compte,
    // les nombres étant calés à droite.
    let mut textes: Vec<(String, f32, f32)> = Vec::new();
    for _ in 0..2 {
        textes.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.console_panel_ui(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                textes.push((
                    text.galley.text().to_owned(),
                    text.pos.x,
                    text.pos.x + text.galley.size().x,
                ));
            }
        }
        output.textures_delta.clear();
    }
    for t in &textes {
        if ["ligne", "passages", "part", "6", "3", "33.3 %", "7"].contains(&t.0.as_str()) {
            eprintln!("T {:?} x={} droite={}", t.0, t.1, t.2);
        }
    }
    // Les textes portent leur remplissage — c'est lui qui aligne dans
    // une chasse fixe —, donc on les compare débarrassés de leurs
    // espaces.
    let droite_de = |mot: &str| -> f32 {
        textes
            .iter()
            .find(|(t, _, _)| t.trim() == mot)
            .unwrap_or_else(|| panic!("« {mot} » doit être peint : {textes:?}"))
            .2
    };

    // Les trois titres, et sous chacun sa colonne. Le programme tourne
    // trois fois dans sa boucle : la ligne 6 y passe 3 fois sur les dix
    // instructions exécutées, soit 30 %.
    for (titre, valeur) in [
        (strings(Lang::Fr).line_word, "6"),
        (strings(Lang::Fr).profile_count, "3"),
        // En français, une virgule décimale et l'espace avant le signe.
        (strings(Lang::Fr).profile_share, "30,0 %"),
    ] {
        let écart = (droite_de(titre) - droite_de(valeur)).abs();
        assert!(
            écart < 4.0,
            "« {titre} » et « {valeur} » doivent finir à la même abscisse, \
             obtenu {écart} px d'écart"
        );
    }

    // Et les colonnes restent dans l'ordre : la ligne avant les
    // passages, les passages avant la part.
    assert!(
        droite_de(strings(Lang::Fr).line_word) < droite_de(strings(Lang::Fr).profile_count),
        "la colonne des lignes vient d'abord"
    );
    assert!(
        droite_de(strings(Lang::Fr).profile_count) < droite_de(strings(Lang::Fr).profile_share),
        "puis celle des passages"
    );
}

/// Un binaire lié dynamiquement le dit dès qu'on l'ouvre.
///
/// Il se lit, se désassemble et s'analyse très bien ; il ne s'exécute
/// pas ici, faute des fonctions qu'il emprunte. Le dire au chargement
/// vaut mieux que de le laisser découvrir sur un « rien n'est projeté à
/// cette adresse » au premier appel — un message qui parle de mémoire là
/// où le problème est ailleurs.
#[test]
fn a_dynamically_linked_binary_says_it_cannot_run_here() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Un binaire du système fait l'affaire : tous sont liés à la libc.
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "ls"), "l'ELF se charge");

    // Le chargement réussit — c'est bien le but : on vient le lire.
    assert!(app.assemble_state.is_some(), "le programme est chargé");
    assert!(
        !app.assemble_state
            .as_ref()
            .expect("chargé")
            .disasm
            .is_empty(),
        "et désassemblé"
    );

    // Mais namsdec prévient, et nomme les bibliothèques réclamées.
    // La note porte le chemin vers sa phrase, non la phrase : on la
    // résout dans la langue de l'application pour la lire.
    let dit = matches!(&app.status, Status::Note(phrase) if phrase(strings(app.lang)).contains("dynamiquement"));
    assert!(dit, "l'avertissement doit être posé : {:?}", app.status);
    assert!(
        app.diagnostics
            .iter()
            .any(|d| d.message.contains("libc") || d.message.contains("ld-linux")),
        "avec ce qu'il réclame : {:?}",
        app.diagnostics
    );

    // Et réassembler ce listing ne détruit pas ce qui est chargé.
    let avant = app.assemble_state.as_ref().expect("chargé").disasm.len();
    app.do_assemble();
    assert!(
        matches!(&app.status, Status::Error(texte) if texte.contains("vient d'un binaire")),
        "le réassemblage doit être refusé en le disant : {:?}",
        app.status
    );
    assert_eq!(
        app.assemble_state
            .as_ref()
            .expect("toujours chargé")
            .disasm
            .len(),
        avant,
        "et rien ne doit avoir bougé"
    );
}

/// Un vrai binaire tient dans la mémoire simulée, à ses vraies adresses.
///
/// La fenêtre faisait soixante-quatre kilo-octets : un programme de
/// trois cent mille octets n'y entrait pas, namsdec retombait sur un
/// désassemblage brut tassé à `BASE_ADDR`, et l'on croyait lire le
/// fichier qu'on venait d'ouvrir alors que toutes les adresses étaient
/// décalées.
#[test]
fn a_real_binary_loads_at_the_addresses_it_declares() {
    // Un binaire du système fait l'affaire : aucun n'entre dans
    // soixante-quatre kilo-octets.
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(
        app.load_elf(&octets, "ls"),
        "un binaire de cette taille se charge"
    );

    let state = app.assemble_state.as_ref().expect("chargé");
    let elf = namsdec_re::elf_scan::scan(&state.output.file).expect("un ELF");

    // Le point d'entrée déclaré par le fichier est bien celui qu'on a.
    assert_eq!(
        state.output.entry, elf.entry,
        "le point d'entrée est celui du fichier, pas un décalage"
    );
    // Et l'émulateur peut lire à cette adresse : la fenêtre la couvre.
    let emulator = app.emulator.as_ref().expect("un émulateur");
    assert!(
        emulator.cpu().mem.peek_bytes(elf.entry, 4).is_ok(),
        "le point d'entrée {:#x} doit être dans la mémoire simulée",
        elf.entry
    );
    assert_eq!(
        emulator.cpu().regs.rip,
        elf.entry,
        "et l'exécution commencerait là"
    );
    // Et surtout : le point d'entrée est *projeté*, avec le droit de
    // s'exécuter. Un vrai binaire a quatre sections de code — `.init`,
    // `.text`, `.fini`, `.plt` — qui portent toutes l'étiquette
    // « .text » ; quand ce nom seul identifiait la zone, chacune
    // effaçait la précédente et le premier pas répondait « rien n'est
    // projeté à cette adresse ».
    assert_eq!(
        emulator
            .cpu()
            .mem
            .check(elf.entry, 1, namsdec_core::Access::Execute),
        Ok(()),
        "le point d'entrée {:#x} doit être exécutable",
        elf.entry
    );

    // La fenêtre s'est bien élargie, sans déplacer la pile des exercices.
    let (base, taille, sommet) = memory_window(state.output.load_addr, state.output.bytes.len());
    assert!(taille > MEM_SIZE, "la fenêtre s'adapte : {taille:#x}");
    assert!(base <= elf.entry && elf.entry < base + taille as u64);
    assert!(sommet > 0, "la pile est quelque part");

    // Et un programme ordinaire — un exercice — retrouve exactement la
    // fenêtre d'avant : même base, même taille, même sommet de pile.
    let (base, taille, sommet) = memory_window(BASE_ADDR, 0x200);
    assert_eq!((base, taille, sommet), (BASE_ADDR, MEM_SIZE, STACK_TOP));
}

/// Un binaire ouvert est chargé comme le noyau le ferait.
///
/// C'est la différence entre « les octets sont quelque part en mémoire »
/// et « le programme est chargé » : des segments projetés en pages
/// entières, des relocations appliquées, une base FS installée sur un
/// bloc de mémoire locale au fil, et une pile qui porte `argc`, `argv`,
/// l'environnement et le vecteur auxiliaire. Sans cela, un binaire
/// moderne ne survit pas à ses dix premières instructions.
#[test]
fn a_binary_is_loaded_the_way_the_kernel_would() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "ls"), "l'ELF se charge");
    let elf = namsdec_asm::elf::parse(&octets).expect("un ELF");
    let emulator = app.emulator.as_ref().expect("un émulateur");
    let cpu = emulator.cpu();

    // 1. Le point d'entrée est projeté, et exécutable.
    assert_eq!(cpu.regs.rip, elf.entry);
    assert_eq!(
        cpu.mem.check(elf.entry, 1, namsdec_core::Access::Execute),
        Ok(()),
        "le point d'entrée {:#x} doit être exécutable",
        elf.entry
    );

    // 2. Les pages entières sont projetées : entre la fin d'une section
    // et le début de la suivante, un vrai processus n'a pas de trou.
    let dernier = elf
        .segments
        .iter()
        .find(|segment| segment.execute)
        .expect("un segment de code");
    let bout = dernier.vaddr + dernier.memsz as u64 - 1;
    assert_eq!(
        cpu.mem.check(bout, 1, namsdec_core::Access::Read),
        Ok(()),
        "le rembourrage de fin de segment est lisible, comme sur une vraie machine"
    );

    // 3. La base FS est posée, et le canari s'y lit.
    assert_ne!(cpu.regs.fs_base, 0, "la base FS est installée");
    assert_eq!(
        cpu.mem.read_u64(cpu.regs.fs_base + 0x28),
        Ok(crate::loader::STACK_GUARD),
        "le canari de pile est en fs:0x28"
    );
    assert_eq!(
        cpu.mem.read_u64(cpu.regs.fs_base),
        Ok(cpu.regs.fs_base),
        "et fs:0 pointe sur lui-même, comme l'attend toute bibliothèque C"
    );

    // 4. La pile initiale : alignée, et portant ce que le noyau y met.
    let rsp = cpu.regs.get64(Reg64::Rsp);
    assert_eq!(rsp % 16, 0, "RSP est aligné sur seize octets à l'entrée");
    assert_eq!(cpu.mem.read_u64(rsp), Ok(1), "argc vaut un");
    let argv0 = cpu.mem.read_u64(rsp + 8).expect("argv[0]");
    let nom = cpu.mem.peek_bytes(argv0, 2).expect("le nom du programme");
    assert_eq!(nom, b"ls", "argv[0] est le nom du fichier ouvert");
    assert_eq!(
        cpu.mem.read_u64(rsp + 16),
        Ok(0),
        "puis le zéro qui clôt argv"
    );

    // 5. Le vecteur auxiliaire dit au programme où il est.
    const AT_NULL: u64 = 0;
    const AT_ENTRY: u64 = 9;
    const AT_PHDR: u64 = 3;
    const AT_RANDOM: u64 = 25;
    let mut at = rsp + 24;
    // L'environnement, jusqu'à son zéro.
    while cpu.mem.read_u64(at).expect("l'environnement") != 0 {
        at += 8;
    }
    at += 8;
    let mut vu: BTreeMap<u64, u64> = BTreeMap::new();
    loop {
        let clé = cpu.mem.read_u64(at).expect("une clé");
        let valeur = cpu.mem.read_u64(at + 8).expect("une valeur");
        if clé == AT_NULL {
            break;
        }
        vu.insert(clé, valeur);
        at += 16;
        assert!(vu.len() < 64, "le vecteur auxiliaire se termine");
    }
    assert_eq!(vu.get(&AT_ENTRY), Some(&elf.entry), "AT_ENTRY");
    assert_eq!(
        vu.get(&AT_PHDR).copied(),
        elf.phdr.map(|(addr, _, _)| addr),
        "AT_PHDR montre les en-têtes projetés"
    );
    let random = *vu.get(&AT_RANDOM).expect("AT_RANDOM");
    assert!(
        cpu.mem.peek_bytes(random, 16).is_ok(),
        "et les seize octets de hasard sont lisibles"
    );

    // 6. Les relocations ont été appliquées.
    let binaire = app
        .assemble_state
        .as_ref()
        .and_then(|state| state.from_binary.as_ref())
        .expect("un binaire chargé");
    if !elf.relocs.is_empty() {
        assert!(
            binaire.load.relocations > 0,
            "un binaire qui en porte doit en avoir reçu : {:?}",
            binaire.load
        );
    }
}

/// Un appel qui sort du programme nomme la fonction qu'il appelait.
///
/// Le chargeur pose une borne à la place de chaque fonction qu'une
/// bibliothèque aurait dû fournir. Sauter dessus donnait « on ne saute
/// pas dans des données », un conseil qui envoie chercher une faute de
/// pointeur là où il n'y en a pas : le programme appelait `printf`, tout
/// simplement, et c'est cela qu'il faut lire.
#[test]
fn a_call_that_leaves_the_program_says_which_function_it_was() {
    // Un programme qui appelle une fonction que namsdec ne fournit pas :
    // charger une bibliothèque à l'exécution, ce qu'il ne fera jamais —
    // il n'est pas un éditeur de liens dynamique, c'est sa définition.
    //
    // `opendir` servait ici jusqu'au 20 octobre et est fournie depuis.
    // Un contrôle de LIMITE doit viser ce qui ne bougera pas, sans quoi
    // il tombe à chaque fonction ajoutée.
    let Some((_dir, binaire)) = compile_c(
        "#include <dlfcn.h>\nint main(void) { return dlopen(\"x.so\", 1) == 0; }\n",
        &[],
    ) else {
        return;
    };
    let octets = std::fs::read(&binaire).expect("le binaire se lit");
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "essai"), "l'ELF se charge");
    assert!(
        !app.assemble_state
            .as_ref()
            .and_then(|state| state.from_binary.as_ref())
            .expect("un binaire chargé")
            .externals
            .is_empty(),
        "il emprunte des fonctions"
    );

    for _ in 0..100_000 {
        app.do_step();
        if matches!(app.status, Status::Error(_)) {
            break;
        }
        if app.emulator.as_ref().is_some_and(|e| e.cpu().halted) {
            break;
        }
    }
    let Status::Error(message) = &app.status else {
        panic!(
            "l'exécution doit buter sur la fonction absente : {:?}",
            app.status
        );
    };
    assert!(
        message.contains("dlopen"),
        "le message nomme la fonction appelée : {message}"
    );
    let conseil = app.error_advice.as_ref().expect("et il explique");
    assert!(
        conseil.contains("éditeur de liens dynamique"),
        "en disant ce qui manque : {conseil}"
    );
}

/// Un vrai programme C lié statiquement s'exécute entièrement.
///
/// C'est l'épreuve du chargeur : trois cent mille instructions de code de
/// démarrage de la glibc — installation de la base FS, allocation des
/// tampons, lecture du vecteur auxiliaire —, puis `main`, puis `printf`,
/// puis la sortie propre. Rien de tout cela ne fonctionne si une seule
/// pièce manque au chargement.
#[test]
fn a_static_c_program_runs_all_the_way_through() {
    let Some((_dir, binaire)) = compile_c(
        "#include <stdio.h>\nint main(int argc, char **argv) {\n\
         \x20   printf(\"bonjour %s (%d)\\n\", argv[0], argc);\n    return 0;\n}\n",
        &["-static"],
    ) else {
        return;
    };

    let octets = std::fs::read(&binaire).expect("le binaire se lit");
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "bonjour"), "l'ELF se charge");

    // Un programme lié statiquement n'attend rien de personne : aucune
    // borne de bibliothèque n'a dû être posée.
    let chargé = app
        .assemble_state
        .as_ref()
        .and_then(|state| state.from_binary.as_ref())
        .expect("un binaire chargé");
    assert!(
        chargé.externals.is_empty(),
        "rien à emprunter : {:?}",
        chargé.load.external
    );

    for pas in 0..1_000_000u32 {
        app.do_step();
        if let Status::Error(message) = &app.status {
            panic!(
                "arrêt au pas {pas}, à {:#x} : {message}",
                app.emulator.as_ref().map(|e| e.cpu().regs.rip).unwrap_or(0)
            );
        }
        if app.emulator.as_ref().is_some_and(|e| e.cpu().halted) {
            break;
        }
    }

    let emulator = app.emulator.as_ref().expect("un émulateur");
    assert!(emulator.cpu().halted, "le programme doit finir");
    assert_eq!(emulator.exit_code(), Some(0), "et sortir proprement");
    let sortie = String::from_utf8_lossy(emulator.output()).into_owned();
    assert_eq!(
        sortie, "bonjour bonjour (1)\n",
        "et avoir écrit ce qu'il devait, `printf` compris"
    );
}

#[test]
fn a_dynamic_c_program_reads_its_input_and_answers() {
    let Some((_dir, binaire)) = compile_c(
        "#include <stdio.h>\n#include <string.h>\n#include <stdlib.h>\n\
         int main(void) {\n\
         \x20   char *saisie = malloc(64);\n\
         \x20   printf(\"mot de passe : \");\n\
         \x20   if (!fgets(saisie, 64, stdin)) return 2;\n\
         \x20   saisie[strcspn(saisie, \"\\n\")] = 0;\n\
         \x20   if (strcmp(saisie, \"ouvre-toi\") == 0) {\n\
         \x20       printf(\"entrez, %s (%d)\\n\", saisie, (int) strlen(saisie));\n\
         \x20       return 0;\n\
         \x20   }\n\
         \x20   puts(\"non\");\n\
         \x20   return 1;\n\
         }\n",
        &[],
    ) else {
        return;
    };
    let octets = std::fs::read(&binaire).expect("le binaire se lit");

    for (saisie, attendu, code) in [
        ("ouvre-toi", "mot de passe : entrez, ouvre-toi (9)\n", 0),
        ("au hasard", "mot de passe : non\n", 1),
    ] {
        let mut app = NamsdecApp::default();
        assert!(app.load_elf(&octets, "essai"), "l'ELF se charge");
        app.send_input(Some(saisie.to_owned()));

        for _ in 0..1_000_000 {
            app.do_step();
            if let Status::Error(message) = &app.status {
                panic!(
                    "« {saisie} » s'arrête à {:#x} : {message}",
                    app.emulator.as_ref().map(|e| e.cpu().regs.rip).unwrap_or(0)
                );
            }
            if app.emulator.as_ref().is_some_and(|e| e.cpu().halted) {
                break;
            }
        }
        let emulator = app.emulator.as_ref().expect("un émulateur");
        assert!(emulator.cpu().halted, "« {saisie} » doit finir");
        assert_eq!(
            String::from_utf8_lossy(emulator.output()),
            attendu,
            "ce que le programme écrit pour « {saisie} »"
        );
        assert_eq!(emulator.exit_code(), Some(code), "et son code de sortie");
    }
}

/// Ce que namsdec annonce fournir, il le fournit vraiment.
///
/// La liste est ce que l'interface montre après avoir chargé un binaire,
/// et c'est sur elle qu'on décide si un programme va tourner. Une seule
/// entrée en trop, et la promesse est fausse.
#[test]
fn every_function_namsdec_claims_to_provide_really_answers() {
    for nom in namsdec_emu::PROVIDED_FUNCTIONS {
        assert!(
            namsdec_emu::provides(nom),
            "« {nom} » est annoncée mais le moteur ne la reconnaît pas"
        );
    }
    // Et ce qu'elle ne contient pas n'est pas fourni en douce.
    //
    // Les quatre choisies ne bougeront pas : namsdec ne lance pas de
    // processus, n'ouvre pas de connexion et ne charge aucune
    // bibliothèque — c'est sa définition. `opendir` figurait ici
    // jusqu'au 20 octobre et est fournie depuis : un exemple qui
    // déménage ne vaut rien dans un contrôle de limite.
    for absente in ["fork", "execvp", "socket", "dlopen"] {
        assert!(
            !namsdec_emu::provides(absente),
            "« {absente} » ne doit pas être annoncée comme fournie"
        );
    }
    // Les variantes que l'éditeur de liens fabrique désignent la même
    // fonction : c'est ce qui fait marcher un binaire compilé avec les
    // protections de la glibc.
    for variante in [
        "printf@GLIBC_2.2.5",
        "__printf_chk",
        "__isoc99_scanf",
        "memcpy@plt",
    ] {
        assert!(
            namsdec_emu::provides(variante),
            "« {variante} » doit être reconnue comme sa fonction d'origine"
        );
    }
}

/// La barre de chronologie se dessine, et rembobine où l'on clique.
///
/// Le regroupement est déjà couvert dans son module ; ce qu'on vérifie
/// ici est qu'elle apparaît dans la fenêtre et qu'un clic remonte bien
/// dans le temps du programme.
#[test]
fn the_timeline_bar_appears_and_rewinds() {
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rcx, 20
boucle:
    add rax, rcx
    dec rcx
    jnz boucle
    hlt
"
        .to_owned(),
        show_timeline: true,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    for _ in 0..2000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let avant = app
        .emulator
        .as_ref()
        .map(|e| e.trace().len())
        .expect("un émulateur");
    assert!(avant > 20, "la boucle a dû tourner : {avant} pas");

    // La barre occupe la largeur de la fenêtre et porte une colonne par
    // tranche de temps : elle se dessine donc, et sur tout le parcours.
    let colonnes = timeline::columns(&app.timeline_trace(), 400);
    assert!(!colonnes.is_empty(), "la barre doit avoir des colonnes");
    let couvert: usize = colonnes.iter().map(|c| c.count).sum();
    assert_eq!(couvert, avant, "elle couvre tout le parcours");

    // Et cliquer au milieu rembobine jusque-là, sans dépasser.
    let visé = timeline::step_at(avant, 0.5);
    let recul = timeline::rewind_to(avant, visé);
    assert!(recul > 0 && recul <= avant, "recul de {recul} sur {avant}");
    for _ in 0..recul {
        app.do_step_back();
    }
    let après = app.emulator.as_ref().map(|e| e.trace().len()).unwrap();
    assert!(
        après < avant,
        "le rembobinage doit avoir reculé : {avant} -> {après}"
    );
}

/// Avancer d'un pas ramène le code sous les yeux.
///
/// Si l'on avait défilé l'éditeur vers la droite — en lisant le bout d'une
/// longue ligne de commentaire —, la zone de défilement y restait, et
/// avancer d'un pas laissait la ligne courante visible en hauteur mais le
/// code et la gouttière sortis à gauche : on ne voyait plus que les
/// commentaires. Un pas doit rendre les instructions visibles.
///
/// On reproduit le défilement vers la droite par la molette (le défaut
/// n'apparaît qu'avec une vraie interaction — en moteur nu, l'éditeur ne
/// défile pas tout seul), on vérifie qu'il cache bien le code, puis qu'un
/// pas le ramène.
#[test]
fn stepping_brings_the_code_back_into_view() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let src = "global _start\n\
               section .text\n\
               _start:\n\
               \x20   mov rax, 10          ; rax = 10, commentaire assez long pour deborder la vue\n\
               \x20   mov rcx, 10          ; rcx = 10, un long commentaire qui pousse loin a droite\n\
               \x20   dec rcx              ; rcx = 9, commentaire long apres le decrement aussi\n\
               \x20   mov rax, 60          ; syscall sys_exit, un commentaire vraiment tres long la\n\
               \x20   syscall              ; fin du programme, commentaire de fin bien long encore\n";
    let mut app = NamsdecApp {
        source: src.to_owned(),
        system_titlebar: false,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let scr = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(700.0, 800.0));
    app.do_assemble();
    app.do_step();

    // Le x le plus à gauche parmi les mnémoniques peintes dans l'éditeur
    // (sous la barre d'outils, d'où le filtre en Y). Négatif = sorties du
    // cadre par la gauche.
    let minx = |app: &mut NamsdecApp, ctx: &egui::Context, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(scr),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.draw(ui));
        let mut m = f32::MAX;
        for clipped in &out.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let txt = t.galley.text();
                if (txt.contains("mov") || txt.contains("dec") || txt.contains("syscall"))
                    && t.pos.y > 90.0
                    && t.pos.x < m
                {
                    m = t.pos.x;
                }
            }
        }
        out.textures_delta.clear();
        m
    };

    let _ = minx(&mut app, &ctx, Vec::new());
    // Molette vers la droite, pointeur sur l'éditeur.
    let pos = egui::pos2(300.0, 400.0);
    for _ in 0..8 {
        let _ = minx(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    source: egui::MouseWheelSource::Wheel,
                    delta: egui::vec2(-600.0, 0.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let caché = minx(&mut app, &ctx, Vec::new());
    assert!(
        caché < 0.0,
        "la molette aurait dû sortir le code à gauche (minx = {caché})"
    );

    app.do_step_over();
    // Quelques images dans la fenêtre où l'éditeur tient le défilement à
    // zéro : le code est revenu, près de la gouttière.
    let _ = minx(&mut app, &ctx, Vec::new());
    let revenu = minx(&mut app, &ctx, Vec::new());
    assert!(
        (0.0..=120.0).contains(&revenu),
        "un pas aurait dû ramener le code près de la gauche (minx = {revenu})"
    );
}

/// SONDE : les points d'arrêt d'un fichier source survivent-ils ?
#[test]
fn sonde_points_arret_source() {
    let dir = tempfile::tempdir().unwrap();
    let chemin = dir.path().join("prog.asm");
    std::fs::write(
        &chemin,
        "bits 64\nglobal _start\n_start:\n    mov rax, 1\n    hlt\n",
    )
    .unwrap();

    // Ce qu'on avait en quittant : un fichier source et deux points d'arrêt.
    let mut app = NamsdecApp {
        source: std::fs::read_to_string(&chemin).unwrap(),
        current_file: Some(chemin.clone()),
        ..Default::default()
    };
    app.breakpoint_lines.insert(4);
    app.watched_registers.insert("RAX");
    let écrit = app.settings_snapshot();
    eprintln!(
        "écrit : breakpoints={:?} watched={:?} current_file={:?} open_binary={:?}",
        écrit.breakpoints, écrit.watched_registers, écrit.current_file, écrit.open_binary
    );

    // Et au lancement suivant.
    let relu = NamsdecApp::from_settings(écrit);
    eprintln!(
        "relu  : breakpoints={:?} watched={:?} source={} octets",
        relu.breakpoint_lines,
        relu.watched_registers,
        relu.source.len()
    );
}

/// Les points d'arrêt d'un fichier source reviennent à la séance
/// suivante — et seulement si le fichier n'a pas bougé.
///
/// Ils étaient écrits dans les réglages depuis toujours, et relus
/// nulle part : seule une séance sur un *binaire* les reposait. Celui
/// qui écrit son propre programme — le cas ordinaire — retrouvait donc
/// son fichier, ses onglets, ses arguments, et pas un seul de ses
/// points d'arrêt.
#[test]
fn the_breakpoints_of_a_source_come_back_next_time() {
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let chemin = dir.path().join("prog.asm");
    let programme = "bits 64\nglobal _start\nsection .text\n_start:\n    mov rax, 1\n    hlt\n";
    std::fs::write(&chemin, programme).unwrap();

    // Ce qu'on avait en quittant.
    let mut app = NamsdecApp {
        source: programme.to_owned(),
        current_file: Some(chemin.clone()),
        // Rouvrir sans demander : depuis le 11 novembre 2026, le défaut
        // est de poser la question, qui a ses propres contrôles. Celui-ci
        // parle des points d'arrêt.
        session_restore: Reprise::Toujours,
        ..Default::default()
    };
    app.breakpoint_lines.insert(5);
    app.watched_registers.insert("RAX");
    let écrit = app.settings_snapshot();
    assert_eq!(écrit.breakpoints, vec![5], "les réglages les portent");
    assert!(
        écrit.current_file_mtime.is_some(),
        "avec la date du fichier, sans quoi on ne saurait pas s'il a bougé"
    );

    // Au lancement suivant, tout revient.
    let relu = NamsdecApp::from_settings(écrit.clone());
    assert_eq!(relu.source, programme, "le fichier est rouvert");
    assert!(
        relu.breakpoint_lines.contains(&5),
        "et le point d'arrêt avec lui : {:?}",
        relu.breakpoint_lines
    );
    assert!(
        relu.watched_registers.contains("RAX"),
        "le registre suivi aussi"
    );

    // Mais pas si le fichier a été réécrit ailleurs entre-temps : les
    // lignes ont pu bouger, et un point d'arrêt au hasard est pire que
    // pas de point d'arrêt du tout.
    std::fs::write(&chemin, format!("; une ligne de plus\n{programme}")).unwrap();
    filetime_plus_tard(&chemin);
    let après = NamsdecApp::from_settings(écrit);
    assert!(
        après.breakpoint_lines.is_empty(),
        "un fichier changé ne reprend pas ses repères : {:?}",
        après.breakpoint_lines
    );
    // Et cela se dit, au lieu de disparaître en silence.
    assert!(
        matches!(après.status, Status::Note(f) if f(strings(Lang::Fr))
            == strings(Lang::Fr).session_file_changed),
        "l'oubli est annoncé : {:?}",
        après.status
    );
}

/// Forcer un saut : il est pris quoi qu'il arrive, et la cible ne
/// bouge pas.
///
/// Le troisième geste manquait, et ce n'est pas le même que
/// l'inversion. Inverser demande « et si la condition était l'autre ? »,
/// forcer demande « et si on y allait de toute façon ? » — c'est celui
/// qu'on veut devant un contrôle qu'on n'a pas encore compris.
#[test]
fn a_jump_can_be_forced_and_still_lands_where_it_pointed() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");

    // On cherche un saut conditionnel long : c'est le cas qui demande
    // du soin, puisque `jmp` long fait un octet de moins.
    let état = app.assemble_state.as_ref().expect("chargé");
    let trouvé = état.disasm.iter().find_map(|instruction| {
        let long = instruction.bytes.len() == 6
            && instruction.bytes[0] == 0x0f
            && (0x80..=0x8f).contains(&instruction.bytes[1]);
        let ligne = état.addr_to_line.get(&instruction.address)?;
        long.then(|| (instruction.address, *ligne, instruction.bytes.clone()))
    });
    let Some((adresse, ligne, avant)) = trouvé else {
        // Pas de saut long dans ce binaire-là : la règle est déjà
        // vérifiée sur les octets, dans namsdec-re.
        return;
    };
    app.cursor_pos = (ligne as usize, 1);

    let (vu, origine, après) = app
        .patch_preview(PatchGeste::Forcer)
        .expect("un saut conditionnel se force");
    assert_eq!(vu, adresse);
    assert_eq!(origine, avant);
    assert_eq!(après.len(), avant.len(), "la longueur ne bouge pas");
    assert_eq!(après[0], 0xe9, "`jmp` long");
    assert_eq!(*après.last().unwrap(), 0x90, "et le `nop` de bourrage");

    // La cible : comptée depuis la fin de chaque instruction, elle doit
    // tomber au même endroit.
    let cible_avant = adresse as i64
        + avant.len() as i64
        + i64::from(i32::from_le_bytes([avant[2], avant[3], avant[4], avant[5]]));
    let cible_après = adresse as i64
        + 5
        + i64::from(i32::from_le_bytes([après[1], après[2], après[3], après[4]]));
    assert_eq!(
        cible_avant, cible_après,
        "un saut forcé qui n'arrive pas où l'autre arrivait ne sert à rien"
    );

    // Appliqué, puis défait : comme les deux autres gestes.
    app.apply_patch(PatchGeste::Forcer);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    let offset = app.patches[0].offset;
    let patché = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(&patché[offset..offset + après.len()], après.as_slice());
    assert_eq!(patché.len(), octets.len(), "le fichier garde sa taille");
    app.undo_patch(0);
    let rendu = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(&rendu[offset..offset + avant.len()], avant.as_slice());
}

/// La gouttière montre ce qu'un clic y ferait.
///
/// On y pose les points d'arrêt, et rien ne le disait : le manuel
/// l'écrit, la fenêtre des points d'arrêt l'écrit, l'écran où l'on
/// travaille se taisait. Quelqu'un qui n'a pas lu le manuel n'a aucune
/// raison de cliquer dans une colonne de numéros.
///
/// Au survol, un point pâle paraît donc à l'endroit exact où le vrai se
/// posera — c'est ce que fait n'importe quel éditeur, et c'est là que ça
/// s'apprend. Pâle, parce qu'il montre ce qui VA se passer et ne
/// prétend pas que c'est fait.
///
/// Le survol se simule pour de vrai : un mouvement de pointeur, puis
/// des images où il ne bouge plus.
#[test]
fn the_gutter_shows_what_a_click_would_do() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = écran_du_banc();

    // Où sont les numéros de ligne : dans la gouttière, à gauche, et ce
    // sont les seuls nombres peints là.
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
    let numéro = lignes
        .iter()
        .find(|(t, r)| t == "5" && r.left() < 80.0)
        .map(|(_, r)| *r)
        .expect("le numéro de la cinquième ligne est peint dans la gouttière");
    // Le point se pose à huit pixels du bord gauche de la gouttière,
    // laquelle commence au bord de la ZONE DE TRAVAIL — et non du
    // panneau : la barre d'icônes de gauche en occupe désormais le
    // premier bord, et viser « huit pixels » dans l'absolu visait une
    // icône.
    let visée = egui::pos2(app.zone_de_travail.left() + 8.0, numéro.center().y);

    /// Les ronds pleins d'un rayon de point d'arrêt peints sur cette ligne.
    fn points(sortie: &egui::FullOutput, y: f32) -> Vec<egui::Color32> {
        let mut trouvés = Vec::new();
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Circle(c) = &clipped.shape
                && (c.radius - 4.0).abs() < 0.5
                && (c.center.y - y).abs() < 4.0
                // Dans la gouttière, que la barre d'icônes décale.
                && c.center.x < crate::barres::BARRE_LARGEUR + 40.0
            {
                trouvés.push(c.fill);
            }
        }
        trouvés
    }

    // Sans le pointeur : rien.
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    let avant = points(&sortie, visée.y);
    sortie.textures_delta.clear();
    assert!(
        avant.is_empty(),
        "sans survol, la gouttière ne montre aucun point — {avant:?}"
    );

    // Le pointeur arrive, puis ne bouge plus.
    let survol = egui::RawInput {
        events: vec![egui::Event::PointerMoved(visée)],
        ..entrée.clone()
    };
    let mut sortie = ctx.run_ui(survol, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    let mut fantômes = Vec::new();
    for _ in 0..3 {
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        fantômes = points(&sortie, visée.y);
        sortie.textures_delta.clear();
    }
    assert_eq!(
        fantômes.len(),
        1,
        "au survol, un point et un seul paraît à l'endroit du futur point \
         d'arrêt — {fantômes:?}"
    );
    assert!(
        fantômes[0].a() < 255,
        "et il est pâle : il montre ce qui va se passer, il ne dit pas que \
         c'est fait — {:?}",
        fantômes[0]
    );
    assert!(
        app.breakpoint_lines.is_empty(),
        "survoler ne pose rien : {:?}",
        app.breakpoint_lines
    );
}

/// La ligne où le processeur est arrêté se voit sans qu'on la cherche.
///
/// Une flèche dans la gouttière et un numéro en couleur d'accent la
/// disaient, mais tous deux sont à GAUCHE : on suit du regard la colonne
/// du code, et il fallait revenir au bord de l'écran pour retrouver où
/// l'on en était. Une bande pâle sur toute la largeur se voit sans
/// effort, et c'est ce que fait tout débogueur.
///
/// Le contrôle demande la bande, sa largeur, et qu'elle ne paraisse que
/// là où le processeur est : une bande qui suivrait le curseur de saisie
/// dirait autre chose.
#[test]
fn the_line_where_the_cpu_stopped_wears_a_band() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = écran_du_banc();

    /// Les bandes pâles peintes en travers de l'éditeur.
    fn bandes(sortie: &egui::FullOutput) -> Vec<egui::Rect> {
        let mut vues = Vec::new();
        for clipped in &sortie.shapes {
            // La bande part du bord de la zone de travail, que la
            // barre d'icônes de gauche décale : on la reconnaît à sa
            // largeur et à sa pâleur, non à une abscisse absolue.
            if let egui::epaint::Shape::Rect(r) = &clipped.shape
                && r.rect.left() < crate::barres::BARRE_LARGEUR + 4.0
                && r.rect.width() > 400.0
                && (10.0..24.0).contains(&r.rect.height())
                && r.fill.a() > 0
                && r.fill.a() < 128
            {
                vues.push(r.rect);
            }
        }
        vues
    }

    // À l'arrêt, rien ne tourne : aucune bande.
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    let avant = bandes(&sortie);
    sortie.textures_delta.clear();
    assert!(
        avant.is_empty(),
        "sans programme en marche, aucune ligne n'est marquée — {avant:?}"
    );

    app.do_assemble();
    app.do_step();
    let mut vues = Vec::new();
    for _ in 0..3 {
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        vues = bandes(&sortie);
        sortie.textures_delta.clear();
    }
    assert_eq!(
        vues.len(),
        1,
        "une ligne est marquée, et une seule : celle où le processeur est \
         arrêté — {vues:?}"
    );
    assert!(
        vues[0].width() > 600.0,
        "et la bande traverse l'éditeur, sinon elle ne se voit pas mieux que \
         la flèche qu'elle remplace : {:.0} px",
        vues[0].width()
    );
}

/// UN DÉTOUR POSÉ S'EXÉCUTE VRAIMENT — l'oracle qui manquait.
///
/// `plan_detour` était éprouvé par une douzaine de contrôles, et aucun
/// ne faisait TOURNER ce qu'il fabrique. Ils vérifiaient les octets du
/// plan et l'arithmétique des sauts — justes, et c'est nécessaire —,
/// mais l'idée tout entière (« on sort par un `jmp`, on exécute le code
/// ajouté, on rejoue ce qu'on a recouvert, on revient ») n'avait jamais
/// été confrontée à l'émulateur. Un plan dont les octets sont justes et
/// qui, exécuté, part dans le décor, serait passé au vert.
///
/// Monté À LA MAIN, sans nasm : l'oracle tourne donc partout, CI
/// comprise, sans s'abstenir. Et le site est un `mov eax, [rip+…]` —
/// relatif à `rip` — à dessein : c'est la forme que la relocalisation
/// doit corriger, et la seule façon de prouver qu'elle la corrige
/// VRAIMENT est de lire, après exécution, la même valeur qu'avant le
/// détour. Une relocalisation fausse lirait ailleurs, et `eax` ne
/// vaudrait pas 0xcafe.
#[test]
fn a_planned_detour_runs_in_the_emulator() {
    use namsdec_core::{Cpu, Memory, Reg64};

    const BASE: u64 = 0x40_0000;
    const TAILLE: usize = 0x2000;
    // Ce que la globale du site vaut (elle est posée à 0x401006, là où
    // l'instruction rip-relative pointe).
    const DONNÉE: u32 = 0xcafe;

    // L'image nue : six octets de `mov eax, [rip+0x1000]`, puis `hlt`.
    //
    // `8B 05` + un déplacement de quatre octets : l'instruction fait six
    // octets, finit à BASE+6, et vise donc BASE+6+0x1000 = 0x401006,
    // où l'on pose 0xcafe. Six octets recouverts (> cinq), donc le
    // détour comblera d'un `nop`.
    let image_nue = |image: &mut Vec<u8>| {
        image[0..6].copy_from_slice(&[0x8b, 0x05, 0x00, 0x10, 0x00, 0x00]);
        image[6] = 0xf4; // hlt
        image[0x1006..0x100a].copy_from_slice(&DONNÉE.to_le_bytes());
    };

    // Ce que l'image nue fait, pour servir de témoin : eax lit la
    // globale, rbx ne bouge pas, et l'on s'arrête sur le `hlt`.
    let exécute = |image: &[u8]| -> (u64, u64, bool, u64) {
        let mut mem = Memory::new(BASE, TAILLE);
        mem.load(BASE, image).expect("l'image se charge");
        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = BASE;
        let mut emu = namsdec_emu::Emulator::new(cpu);
        let arrêt = emu.run(10_000).expect("l'exécution aboutit");
        (
            emu.cpu().regs.get64(Reg64::Rax),
            emu.cpu().regs.get64(Reg64::Rbx),
            matches!(arrêt, namsdec_emu::RunStop::Halted),
            emu.cpu().regs.rip,
        )
    };

    // Le témoin, sans détour : eax lit la globale, rbx reste nul.
    let mut témoin = vec![0u8; TAILLE];
    image_nue(&mut témoin);
    let (rax0, rbx0, halt0, _) = exécute(&témoin);
    assert_eq!(rax0, DONNÉE as u64, "sans détour, eax lit la globale");
    assert_eq!(rbx0, 0, "et rbx n'est pas touché");
    assert!(halt0, "et le programme s'arrête sur son hlt");

    // Le détour : une place libre exécutable à 0x400800, et du code
    // ajouté qui pose rbx — un registre que l'original NE touche pas,
    // pour qu'on sache ensuite que c'est bien l'ajout qui l'a écrit.
    let mut image = vec![0u8; TAILLE];
    image_nue(&mut image);
    let place = namsdec_re::slack::Slack {
        vaddr: 0x40_0800,
        offset: 0x800,
        len: 0x100,
        filler: 0x00,
        executable: true,
        section: Some(".text".to_owned()),
    };
    let ajout = [0xbb, 0x42, 0x00, 0x00, 0x00]; // mov ebx, 0x42
    let detour = namsdec_re::patch::plan_detour(&image, 64, 0x00, BASE, &place, &ajout, &[])
        .expect("ce détour est posable");

    // On pose les deux changements, comme la fenêtre Correctifs le ferait.
    namsdec_re::patch::apply(&mut image, detour.aller.offset, &detour.aller.after)
        .expect("l'aller s'écrit");
    namsdec_re::patch::apply(&mut image, detour.corps.offset, &detour.corps.after)
        .expect("le corps s'écrit");

    let (rax, rbx, halt, rip) = exécute(&image);

    // L'ajout a tourné : rbx porte ce que seul lui écrit.
    assert_eq!(rbx, 0x42, "le code ajouté par le détour s'est exécuté");
    // Ce qu'on a recouvert a été REJOUÉ, et sa relocalisation est juste :
    // le `mov eax, [rip+…]`, déplacé dans la place libre, lit toujours
    // 0x401006 — donc toujours 0xcafe. C'est l'assertion qui compte le
    // plus : une relocalisation fausse donnerait une autre valeur, sans
    // rien signaler.
    assert_eq!(
        rax, DONNÉE as u64,
        "l'instruction recouverte, relocalisée, lit la même globale"
    );
    // Et l'on est REVENU : l'arrêt est le `hlt` d'origine, pas une
    // dérive dans la place libre.
    assert!(halt, "l'exécution revient et s'arrête sur le hlt d'origine");
    assert_eq!(rip, BASE + 7, "le hlt atteint est celui du site, à BASE+6");
}

/// DEUX DÉTOURS QUI SE CROISENT CASSENT LE PROGRAMME — mesuré, puis
/// refusé.
///
/// Le danger était écrit dans AMELIORATIONS depuis le 15 octobre 2026,
/// et il y était resté à l'état de phrase : « deux détours au même
/// endroit ne cohabitent pas ». Une phrase ne retient rien. Celui-ci le
/// FAIT FAIRE à l'émulateur — d'abord sans la garde, pour voir ce qui
/// arrive, puis avec, pour voir que cela n'arrive plus.
///
/// Il faut un oracle d'exécution pour ce défaut-là, et aucun contrôle
/// d'octets ne l'aurait trouvé : les deux plans sont parfaitement
/// formés, chacun pris séparément. C'est leur superposition qui ment,
/// et une superposition ne se lit pas dans un plan.
///
/// LE SITE CHOISI EST LE `nop` DE COMPLÉMENT, et ce choix a été mesuré.
/// Viser le milieu du `jmp` posé — un octet de son déplacement — est
/// refusé de toute façon, mais par hasard : `0xFB 0x07` ne se décode
/// pas, et l'on reçoit `SiteIndécodable`. Le hasard protège ici et ne
/// protégera pas ailleurs — un autre déplacement se décode très bien.
/// Le `nop` que le détour laisse derrière lui, en revanche, se décode
/// toujours, et c'est l'endroit le plus tentant du fichier : le listing
/// le montre comme une instruction ordinaire, inoffensive, et le
/// désigner ne semble rien coûter.
#[test]
fn two_crossing_detours_break_the_program_and_are_refused() {
    use namsdec_core::{Cpu, Memory, Reg64};

    const BASE: u64 = 0x40_0000;
    const TAILLE: usize = 0x2000;
    const DONNÉE: u32 = 0xcafe;

    let mut image = vec![0u8; TAILLE];
    // `mov eax, [rip+0x1000]` (six octets), puis `hlt`.
    image[0..6].copy_from_slice(&[0x8b, 0x05, 0x00, 0x10, 0x00, 0x00]);
    image[6] = 0xf4;
    image[0x1006..0x100a].copy_from_slice(&DONNÉE.to_le_bytes());

    let exécute = |image: &[u8]| -> (u64, u64, bool) {
        let mut mem = Memory::new(BASE, TAILLE);
        mem.load(BASE, image).expect("l'image se charge");
        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = BASE;
        let mut emu = namsdec_emu::Emulator::new(cpu);
        match emu.run(10_000) {
            Ok(arrêt) => (
                emu.cpu().regs.get64(Reg64::Rax),
                emu.cpu().regs.get64(Reg64::Rbx),
                matches!(arrêt, namsdec_emu::RunStop::Halted),
            ),
            // Un détour cassé ne s'arrête pas proprement : il décode du
            // bourrage, et l'émulateur le dit. C'est un échec, pas une
            // exception du contrôle.
            Err(_) => (0, 0, false),
        }
    };

    let trou = |vaddr: u64, offset: u64, len: u64| namsdec_re::slack::Slack {
        vaddr,
        offset,
        len,
        filler: 0x00,
        executable: true,
        section: Some(".text".to_owned()),
    };

    // LE PREMIER DÉTOUR, posé sur un fichier neuf : il tient, c'est le
    // contrôle voisin qui l'a montré.
    let premier = namsdec_re::patch::plan_detour(
        &image,
        64,
        0x00,
        BASE,
        &trou(0x40_0800, 0x800, 0x100),
        &[0xbb, 0x42, 0x00, 0x00, 0x00], // mov ebx, 0x42
        &[],
    )
    .expect("le premier détour est posable");
    namsdec_re::patch::apply(&mut image, premier.aller.offset, &premier.aller.after).unwrap();
    namsdec_re::patch::apply(&mut image, premier.corps.offset, &premier.corps.after).unwrap();

    let (rax, rbx, halt) = exécute(&image);
    assert_eq!(rbx, 0x42, "le premier détour s'exécute");
    assert_eq!(rax, DONNÉE as u64, "et l'original garde son sens");
    assert!(halt, "et le programme s'arrête où il faut");

    let posés = vec![premier.corps.clone(), premier.aller.clone()];
    // La queue du trou : ce que `slack::find` offrirait encore.
    let pris = premier.corps.after.len() as u64;
    let reste = trou(0x40_0800 + pris, 0x800 + pris, 0x100 - pris);

    // LE SECOND, SUR LE `nop` DE COMPLÉMENT, SANS LA GARDE. `&[]` est
    // exactement ce que namsdec passait avant aujourd'hui : l'aveu que
    // rien n'est posé, alors que tout l'est.
    let mut cassée = image.clone();
    assert_eq!(
        cassée[0x05], 0x90,
        "l'octet visé est bien le nop que le premier détour a laissé"
    );
    let second = namsdec_re::patch::plan_detour(
        &cassée,
        64,
        0x05,
        BASE + 5,
        &reste,
        &[0xb9, 0x07, 0x00, 0x00, 0x00], // mov ecx, 7
        &[],
    )
    .expect("sans la garde, ce plan sort sans se plaindre");
    namsdec_re::patch::apply(&mut cassée, second.aller.offset, &second.aller.after).unwrap();
    namsdec_re::patch::apply(&mut cassée, second.corps.offset, &second.corps.after).unwrap();

    let (rax2, rbx2, halt2) = exécute(&cassée);
    // CE QUE LA MESURE DONNE, et c'est pire que « le premier détour est
    // perdu » : le programme ne s'arrête plus du tout. Son `hlt` était
    // à BASE+6, le second aller l'a recouvert, et le retour du premier
    // détour y revient quand même — au milieu d'un `jmp`. L'émulateur
    // décode alors du bourrage et s'arrête en erreur. rax et rbx
    // restent nuls : ni l'original ni l'ajout n'ont eu lieu.
    assert!(
        !halt2,
        "le programme ne devrait plus s'arrêter : son hlt a été recouvert \
         (rax={rax2:#x}, rbx={rbx2:#x})"
    );
    assert_eq!(rbx2, 0, "et l'ajout du premier détour n'a pas tourné");
    assert_eq!(rax2, 0, "et l'original n'a pas tourné non plus");

    // AVEC LA GARDE : la même demande, et rien n'est écrit.
    let refus = namsdec_re::patch::plan_detour(
        &image,
        64,
        0x05,
        BASE + 5,
        &reste,
        &[0xb9, 0x07, 0x00, 0x00, 0x00],
        &posés,
    );
    assert_eq!(
        refus,
        Err(namsdec_re::patch::DetourRefusé::SurUnCorrectif {
            à: BASE + 5,
            offset: 0x05,
            correctif: BASE,
        }),
        "le second détour est refusé, et nomme le correctif qu'il aurait abîmé"
    );
    // Et le refus se dit dans les deux langues, sans « {:?} » qui
    // traîne : c'est ce que la fenêtre affiche.
    let message = refus.unwrap_err().message(true);
    assert!(
        message.contains("0x400005") && message.contains("0x400000"),
        "le message nomme l'adresse visée et le correctif : {message}"
    );
}

/// POSER DEUX FOIS LE MÊME PLAN NE DOIT RIEN FAIRE LA SECONDE — et ce
/// qui serait perdu, c'est la réversibilité.
///
/// Les octets écrits sont les mêmes, et c'est ce qui rend le défaut
/// invisible : le fichier obtenu est juste. Mais le `before` du second
/// correctif est relevé sur l'image DÉJÀ corrigée — c'est-à-dire le
/// `jmp` du premier. Défaire ramènerait alors le détour au lieu de
/// l'original, et l'on ne pourrait plus revenir en arrière : le fichier
/// garderait son détour pour toujours, sans qu'aucune ligne de la
/// fenêtre Correctifs le dise. Mesuré sans la garde : le second appel
/// répond « Octets changés, binaire rechargé », c'est-à-dire qu'il
/// annonce un succès.
///
/// CE N'EST PAS CE QUE L'INTERFACE PERMET AUJOURD'HUI, et le dire
/// autrement serait faux : le panneau jette son plan à chaque
/// rechargement, donc le bouton « Poser » ne se reclique pas sur un
/// plan déjà posé. Ce contrôle porte sur `write_detour` lui-même, qui
/// est le seul endroit où les octets partent et qui sert aussi la
/// demande `ReRequest::PatchDetour` — d'où qu'elle vienne. Une garde
/// posée ailleurs que là où l'on écrit dépend de qui appelle.
#[test]
fn posing_the_same_detour_twice_changes_nothing_the_second_time() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "le binaire s'ouvre");

    let state = app.assemble_state.as_ref().expect("un état d'assemblage");
    let origine = state.output.file.clone();
    let place = namsdec_re::slack::find(&origine)
        .into_iter()
        .find(|t| t.executable && t.len > 64)
        .expect("le binaire fabriqué porte du bourrage exécutable");

    // Le premier site détournable du listing. On cherche plutôt que de
    // viser une adresse écrite en clair : ce que nasm produit peut
    // changer d'une version à l'autre, la règle non.
    let ajout = [0x90u8];
    let plan = state
        .disasm
        .iter()
        .find_map(|instr| {
            let offset = namsdec_re::imports::file_offset(&origine, instr.address)?;
            namsdec_re::patch::plan_detour(
                &origine,
                64,
                offset as usize,
                instr.address,
                &place,
                &ajout,
                &[],
            )
            .ok()
        })
        .expect("au moins une instruction du binaire fabriqué se détourne");

    // LE PREMIER CLIC : les deux écritures partent, sous un seul groupe.
    app.write_detour(&plan);
    assert_eq!(app.patches.len(), 2, "un détour pose deux correctifs");
    let après_un = app
        .assemble_state
        .as_ref()
        .expect("un état")
        .output
        .file
        .clone();
    assert_ne!(après_un, origine, "et le fichier a changé");

    // LE SECOND CLIC : refusé, dit, et sans effet.
    app.write_detour(&plan);
    assert!(
        matches!(app.status, Status::Error(_)),
        "le second clic doit se refuser À HAUTE VOIX : {:?}",
        app.status
    );
    assert_eq!(
        app.patches.len(),
        2,
        "et n'inscrire aucun correctif de plus : {}",
        app.patches.len()
    );
    assert_eq!(
        app.assemble_state.as_ref().expect("un état").output.file,
        après_un,
        "et ne pas toucher au fichier"
    );

    // ET LE DÉTOUR SE DÉFAIT ENCORE, jusqu'à l'octet : c'est ce que le
    // second clic aurait perdu.
    app.undo_patch(0);
    assert!(app.patches.is_empty(), "le groupe entier est retiré");
    assert_eq!(
        app.assemble_state.as_ref().expect("un état").output.file,
        origine,
        "et le fichier est exactement celui du départ"
    );
}

/// Un source qui occupe nasm assez longtemps pour qu'on ait le temps
/// de regarder : `%rep 200000` lui prend environ 180 ms, mesuré sur la
/// machine de développement, et produit neuf méga-octets de listing.
///
/// Assez lent pour que l'écart entre « lancer » et « attendre la fin »
/// soit sans ambiguïté — un facteur dix sépare les deux —, assez court
/// pour qu'un contrôle qui le lance deux fois reste une affaire d'une
/// demi-seconde.
const SOURCE_LENT: &str = "\
section .text
global _start
_start:
%rep 200000
    nop
%endrep
    hlt
";

/// Lancer un assemblage REND LA MAIN, et c'est tout l'objet de P2.
///
/// La mesure est un rapport, pas un seuil absolu : on assemble d'abord
/// en attendant, pour savoir ce que ce source coûte sur CETTE
/// machine ; puis on relance sans attendre, et le lancement doit
/// prendre une fraction de ce temps-là. Un seuil en millisecondes
/// aurait fini par échouer sur une machine lente ou chargée, ce qui
/// est arrivé à ce projet (A 9.51).
#[test]
fn lancer_un_assemblage_rend_la_main_tout_de_suite() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: SOURCE_LENT.to_owned(),
        ..Default::default()
    };
    let début = std::time::Instant::now();
    app.do_assemble();
    let complet = début.elapsed();
    assert!(
        matches!(app.status, Status::AssembleOk),
        "le source d'épreuve doit s'assembler : {:?}",
        app.status
    );

    // Le même travail, lancé sans l'attendre.
    app.set_source(SOURCE_LENT.to_owned());
    let début = std::time::Instant::now();
    app.start_assemble();
    let lancement = début.elapsed();
    assert!(
        app.assembling(),
        "le travail tourne, et l'interface a déjà la main"
    );
    assert!(
        matches!(app.status, Status::Assembling),
        "et la barre d'état le dit : {:?}",
        app.status
    );
    assert!(
        lancement * 3 < complet,
        "lancer doit coûter bien moins que faire : {lancement:?} contre {complet:?}"
    );
    app.attendre_l_assemblage();
    assert!(matches!(app.status, Status::AssembleOk));
}

/// Un résultat qui arrive après coup ne remplace pas ce qui est à
/// l'écran.
///
/// Le cas qu'on veut interdire : on lance un assemblage, on continue
/// d'écrire — ou l'on change d'onglet —, et dix secondes plus tard
/// nasm rend un listing qui décrit un texte que personne ne regarde
/// plus. Appliqué, il ferait pointer la gouttière, les points d'arrêt
/// et les diagnostics sur des lignes qui ont bougé.
///
/// Déterministe quelle que soit la vitesse de la machine : la récolte
/// n'a lieu que dans `tick_assemble`, donc le changement de texte
/// arrive toujours avant elle.
#[test]
fn un_resultat_perime_ne_remplace_pas_ce_qui_est_a_l_ecran() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: SOURCE_LENT.to_owned(),
        ..Default::default()
    };
    app.start_assemble();
    // On continue d'écrire pendant que nasm travaille.
    app.set_source("section .text\nglobal _start\n_start:\n    hlt\n".to_owned());
    app.attendre_l_assemblage();
    assert!(
        app.assemble_state.is_none(),
        "le résultat décrivait l'ancien texte : il est écarté"
    );
    assert!(
        app.emulator.is_none(),
        "et aucun émulateur n'est bâti sur lui"
    );
    assert!(
        !matches!(app.status, Status::Assembling),
        "la barre ne reste pas à « en cours » : {:?}",
        app.status
    );

    // Et le texte à l'écran s'assemble normalement derrière.
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    assert!(app.emulator.is_some());
}

/// L'assemblage s'arrête à la demande, et l'état revient au repos.
#[test]
fn un_assemblage_s_arrete_a_la_demande() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: SOURCE_LENT.to_owned(),
        ..Default::default()
    };
    app.start_assemble();
    assert!(app.assembling());
    app.cancel_assemble();
    assert!(!app.assembling(), "plus rien en cours");
    assert!(
        !matches!(app.status, Status::Assembling),
        "et la barre ne promet plus un travail qui n'existe pas : {:?}",
        app.status
    );
    // Récolter après l'arrêt ne ressuscite rien : le travail a été
    // oublié, pas mis de côté.
    app.tick_assemble();
    assert!(app.assemble_state.is_none());
    assert!(!app.assembling());
}

/// Deux assemblages lancés coup sur coup : le premier est arrêté, le
/// second fait foi.
///
/// C'est le Ctrl+Maj+B qu'on frappe deux fois parce que rien n'a l'air
/// de se passer. Sans cette règle, deux nasm travailleraient sur le
/// même tampon et le dernier à finir — pas le dernier lancé —
/// gagnerait.
#[test]
fn un_second_assemblage_remplace_le_premier() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: SOURCE_LENT.to_owned(),
        ..Default::default()
    };
    app.start_assemble();
    app.start_assemble();
    assert!(app.assembling(), "un seul, et c'est le dernier");
    app.attendre_l_assemblage();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    assert!(app.emulator.is_some());
}

/// Changer d'onglet pendant un assemblage ne laisse rien derrière.
///
/// Trois choses à ne pas abîmer : le travail en fond ne doit pas
/// continuer pour rien, la barre ne doit pas rester à « en cours », et
/// l'onglet rangé ne doit pas recevoir plus tard le listing d'un texte
/// qu'il ne contient plus.
#[test]
fn changer_d_onglet_pendant_un_assemblage_ne_laisse_rien_derriere() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: SOURCE_LENT.to_owned(),
        ..Default::default()
    };
    app.start_assemble();
    assert!(app.assembling());

    let rangé = app.take_tab();
    assert!(
        !app.assembling(),
        "le travail ne suit pas l'onglet qu'on vient de ranger"
    );
    assert!(
        !matches!(app.status, Status::Assembling),
        "{:?}",
        app.status
    );

    // On revient dessus : rien n'a été corrompu, et l'assemblage se
    // refait normalement.
    app.restore_tab(rangé);
    assert_eq!(app.source, SOURCE_LENT, "le texte est revenu intact");
    assert!(
        app.assemble_state.is_none(),
        "et rien ne s'est glissé dedans"
    );
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
}

/// Stop arrête l'assemblage, et c'est le seul bouton qui le dise.
#[test]
fn stop_arrete_aussi_un_assemblage() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: SOURCE_LENT.to_owned(),
        ..Default::default()
    };
    app.start_assemble();
    app.dispatch(Action::Stop);
    assert!(!app.assembling(), "nasm a été arrêté");
    assert!(
        matches!(app.status, Status::Note(_)),
        "et on le dit sans crier à l'erreur : {:?}",
        app.status
    );
}

/// Pendant l'assemblage, la barre d'état le dit — et dit depuis
/// combien de temps.
///
/// Le temps écoulé plutôt qu'une barre qui avance : un assembleur ne
/// dit pas où il en est, et une progression inventée est pire qu'un
/// chiffre vrai. C'est aussi le temps qui décide de l'abandon, donc
/// c'est celui qu'on a envie de lire.
///
/// Relevé sur `status_line` et non sur une image : rien ne récolte ici,
/// donc l'état mesuré est bien celui de l'assemblage en cours, quelle
/// que soit la vitesse de la machine.
#[test]
fn la_barre_dit_que_l_assemblage_tourne() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    for lang in [Lang::Fr, Lang::En] {
        let mut app = NamsdecApp {
            lang,
            source: SOURCE_LENT.to_owned(),
            ..Default::default()
        };
        app.start_assemble();
        let (texte, _, _) = app.status_line();
        assert!(
            texte.contains(strings(lang).assemble_running),
            "la barre dit ce qui tourne ({lang:?}) : {texte}"
        );
        assert!(
            texte.contains(" s"),
            "et depuis combien de temps ({lang:?}) : {texte}"
        );
        app.cancel_assemble();
    }
}

/// Une erreur d'assemblage ouvre le panneau des diagnostics — même
/// quand le travail s'est fait en fond.
///
/// Le défaut que ce contrôle retient : `show_diagnostics_if_any` était
/// appelé juste après le LANCEMENT, c'est-à-dire avant que le moindre
/// diagnostic n'existe. Il montrait donc ceux de l'assemblage
/// précédent — une erreur corrigée restait affichée, une erreur neuve
/// n'ouvrait rien. Le panneau s'ouvre maintenant à la récolte, seul
/// moment où l'on sait ce que nasm a eu à dire.
#[test]
fn une_erreur_ouvre_le_panneau_meme_assemblee_en_fond() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        // Un programme correct d'abord : le panneau n'a rien à dire.
        source: "section .text\nglobal _start\n_start:\n    hlt\n".to_owned(),
        show_console: true,
        ..Default::default()
    };
    app.dispatch(Action::Assemble);
    app.attendre_l_assemblage();
    assert!(app.diagnostics.is_empty(), "{:?}", app.diagnostics);
    assert_ne!(app.console_tab, ConsoleTab::Diagnostics);

    // Puis une faute de frappe, par le bouton.
    app.set_source("section .text\nglobal _start\n_start:\n    mob rax, 1\n".to_owned());
    app.dispatch(Action::Assemble);
    app.attendre_l_assemblage();
    assert!(
        !app.diagnostics.is_empty(),
        "nasm a quelque chose à dire : {:?}",
        app.status
    );
    assert_eq!(
        app.console_tab,
        ConsoleTab::Diagnostics,
        "et le panneau s'ouvre dessus"
    );

    // Et corrigée, le panneau rend la place : sans quoi l'erreur
    // d'avant resterait sous les yeux.
    app.set_source("section .text\nglobal _start\n_start:\n    mov rax, 1\n    hlt\n".to_owned());
    app.dispatch(Action::Assemble);
    app.attendre_l_assemblage();
    assert!(app.diagnostics.is_empty(), "{:?}", app.diagnostics);
    assert_ne!(app.console_tab, ConsoleTab::Diagnostics);
}

/// L'adresse d'un saut porte le nom que la source lui donne.
///
/// Relevé à l'écran le 16 novembre 2026 : la bande du bas annonçait
/// `jne short 0000000000400021h` devant un programme dont la ligne dit
/// `jnz boucle`. Celui qui apprend vient d'écrire un nom, et on lui
/// rendait seize chiffres.
#[test]
fn a_jump_target_is_named_as_the_source_named_it() {
    let source = "\
section .text
global _start
_start:
    mov rcx, 2
boucle:
    dec rcx
    jnz boucle
    hlt
";
    // L'étiquette est sur sa propre ligne : l'adresse tombe sur la
    // ligne SUIVANTE, et c'est le cas qui demande de remonter.
    assert_eq!(label_at_line(source, 6).as_deref(), Some("boucle"));
    assert_eq!(
        label_at_line(source, 5).as_deref(),
        Some("boucle"),
        "la ligne de l'étiquette elle-même"
    );
    assert_eq!(
        label_at_line(source, 7),
        None,
        "au milieu d'un bloc, l'adresse n'a pas de nom — `boucle` désigne \
         la première instruction, pas celle-ci"
    );
    // Une étiquette locale compte : c'est ce que l'auteur a tapé.
    assert_eq!(
        label_at_line("_start:\n.suite:\n    nop\n", 3).as_deref(),
        Some(".suite")
    );
    // Et une ligne hors du fichier ne fait pas tomber l'appel.
    assert_eq!(label_at_line(source, 999), None);
    assert_eq!(label_at_line(source, 0), None);

    // Puis le texte de l'instruction, à travers l'application.
    let mut app = NamsdecApp {
        source: source.to_owned(),
        debutant: true,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    // Trois pas : `mov`, `dec`, puis le saut est l'instruction qu'on
    // retient avant de l'exécuter.
    app.do_step();
    app.do_step();
    app.do_step();
    let pas = app.last_step.as_ref().expect("un dernier pas");
    assert!(
        pas.texte.contains("boucle"),
        "le saut porte le nom de la source : {}",
        pas.texte
    );
    assert!(
        !pas.texte.contains("0000"),
        "et plus l'adresse de seize chiffres : {}",
        pas.texte
    );

    // AU NIVEAU AVANCÉ, L'ADRESSE RESTE : c'est elle qu'on compare au
    // listing, au vidage et à ce que dit gdb.
    app.debutant = false;
    app.do_reset();
    app.do_step();
    app.do_step();
    app.do_step();
    let pas = app.last_step.as_ref().expect("un dernier pas");
    assert!(
        !pas.texte.contains("boucle"),
        "le mode avancé garde l'adresse : {}",
        pas.texte
    );
}

/// UN PAS APRÈS LA FIN N'EN EST PAS UN.
///
/// Le défaut que deux captures du banc montraient le 16 novembre 2026 :
/// le bloc « Dernier pas » annonçait `add [rax], al` sur un programme
/// arrêté — les octets nuls qui suivent le `hlt`, décodés comme une
/// instruction parfaitement valide, avec son pseudo-code et son « rien
/// n'a changé ». Le bouton était bien éteint ; le raccourci, lui,
/// passait.
#[test]
fn a_finished_program_does_not_invent_one_more_step() {
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rax, 1
    hlt
"
        .to_owned(),
        debutant: true,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();
    app.do_step();
    assert!(app.program_finished(), "le hlt a arrêté la machine");
    let avant = app.last_step.clone();
    let rip = app.emulator.as_ref().unwrap().cpu().regs.rip;

    // Dix pas de plus, comme le ferait une touche gardée enfoncée.
    for _ in 0..10 {
        app.do_step();
    }
    assert_eq!(
        app.last_step.as_ref().map(|p| p.texte.clone()),
        avant.as_ref().map(|p| p.texte.clone()),
        "le dernier pas reste celui qu'on a vraiment fait"
    );
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.rip,
        rip,
        "et la machine n'a pas bougé"
    );
    // L'ÉTAT D'ARRÊT RESTE : « Arrêté (hlt) » dit déjà pourquoi la
    // touche ne fait rien, et il dit COMMENT le programme s'est
    // arrêté — ce qu'une phrase plus vague effacerait.
    assert!(
        matches!(app.status, Status::Halted),
        "l'arrêt garde son nom : {:?}",
        app.status
    );
    // Quand la barre parle d'autre chose, en revanche, la phrase
    // paraît : sans elle, la touche ne ferait rien du tout, et rien du
    // tout ressemble à une panne.
    app.status = Status::SteppedBack;
    app.do_step();
    assert!(
        matches!(app.status, Status::Note(_)),
        "on le dit plutôt que de laisser croire à une panne : {:?}",
        app.status
    );
    // Le pas large et la course entière suivent la même règle.
    app.do_step_over();
    app.do_run();
    assert!(!app.running, "une course sur un programme fini ne part pas");
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.rip,
        rip,
        "rien n'a bougé non plus"
    );
}
