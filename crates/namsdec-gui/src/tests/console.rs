//! La console : ce que le programme écrit, ce qu'on lui donne.
//!
//! L'entrée attendue et annoncée, les arguments, l'environnement, les
//! couleurs ANSI, et la rangée de saisie qui doit rester visible à toute
//! hauteur.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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
        // Le programme écrit beaucoup, puis lit : sans ce `read`, la
        // console ne proposerait aucune saisie — et elle a raison, un
        // programme qui ne lit rien n'a que faire d'un champ de saisie.
        source: "section .data\n    msg db \"beaucoup de sortie\", 10\n\
                 \x20   len equ $ - msg\nsection .text\n    global _start\n_start:\n\
                 \x20   mov rcx, 40\nboucle:\n    push rcx\n    mov rax, 1\n\
                 \x20   mov rdi, 1\n    mov rsi, msg\n    mov rdx, len\n    syscall\n\
                 \x20   pop rcx\n    dec rcx\n    jnz boucle\n\
                 \x20   xor rax, rax\n    xor rdi, rdi\n    mov rsi, msg\n\
                 \x20   mov rdx, 4\n    syscall\n    hlt\n"
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

    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));
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

    // TOUTE LA LARGEUR QUE LES BARRES LAISSENT, panneau latéral
    // compris.
    //
    // C'était « toute la largeur » tout court, et la raison tient
    // toujours : un programme qui écrit suppose quatre-vingts colonnes,
    // et le panneau latéral ne doit pas les lui prendre. Les barres
    // d'icônes des bords, elles, encadrent la console comme elles
    // encadrent l'éditeur — c'est ainsi chez JetBrains, d'où vient la
    // consigne, et trente-quatre pixels de chaque côté ne coûtent
    // aucune colonne.
    let barres = if app.show_rails {
        crate::barres::BARRE_LARGEUR * 2.0
    } else {
        0.0
    };
    assert_eq!(
        console.width(),
        screen.width() - barres,
        "la console prend toute la largeur que les barres laissent, \
         panneau latéral compris"
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

/// La console prend la hauteur de ce qu'elle a à dire.
///
/// « Le programme n'a rien écrit » occupait le quart de l'écran. Deux
/// lignes de sortie n'ont pas besoin de plus que deux lignes ; vingt en
/// réclament davantage, jusqu'à un plafond.
#[test]
fn the_console_takes_the_height_of_what_it_has_to_say() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mesure = |app: &mut NamsdecApp| {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 1400.0),
            )),
            ..Default::default()
        };
        let mut hauteur = 0.0;
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| {
                hauteur = app.console_wanted_height(ui);
                app.draw(ui);
            });
            output.textures_delta.clear();
        }
        hauteur
    };

    // Rien à dire : la console se réduit à ses onglets et à son champ.
    //
    // LE MINIMUM CONSTANT NE CONNAÎT PAS LES MARGES DU CADRE de la
    // sortie — seize pixels qu'une console, même vide, peint toujours.
    // La comparaison se faisait contre `CONSOLE_MIN_HEIGHT` seul, et
    // elle passait tant que ces seize pixels manquaient aussi au calcul
    // de la hauteur demandée : deux erreurs qui se tenaient chaud. La
    // console demandait alors moins que le plancher qu'elle se donne
    // elle-même, et arrivait à l'écran coincée à son minimum.
    let vide = CONSOLE_MIN_HEIGHT + crate::console::MARGES_DE_LA_SORTIE;
    let mut app = NamsdecApp::default();
    let mesurée = mesure(&mut app);
    assert!(
        mesurée <= vide + 1.0,
        "une console vide réclame {mesurée} pixels, et non les {vide} de ses \
         onglets, de son cadre et d'une ligne"
    );

    // Vingt lignes écrites : elle grandit, mais pas sans fin.
    app.source = "\
global _start

section .data
ligne: db \"x\", 10

section .text
_start:
    mov rcx, 20
boucle:
    push rcx
    mov rax, 1
    mov rdi, 1
    mov rsi, ligne
    mov rdx, 2
    syscall
    pop rcx
    dec rcx
    jnz boucle
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
    let pleine = mesure(&mut app);
    assert!(
        pleine > vide,
        "vingt lignes devraient réclamer plus que rien : {pleine} contre {vide}"
    );
    assert!(
        pleine < 900.0 * CONSOLE_MAX_FRACTION + 40.0,
        "et pas plus que le plafond : {pleine}"
    );

    // La hauteur réglée à la main est retenue, et se retrouve au
    // lancement suivant.
    app.console_height = Some(240.0);
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(écrit.contains("\"console_height\":240"), "{écrit}");
    // Et l'automatique revient en effaçant ce réglage.
    app.console_height = None;
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(écrit.contains("\"console_height\":null"), "{écrit}");

    // La part du panneau latéral laissée à l'onglet se règle de la même
    // façon, à la poignée, et se retient aussi.
    app.sidebar_tab_height = Some(320.0);
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(écrit.contains("\"sidebar_tab_height\":320"), "{écrit}");
    let relu = NamsdecApp::from_settings(serde_json::from_str(&écrit).unwrap());
    assert_eq!(relu.sidebar_tab_height, Some(320.0));
}

/// Quand le programme attend de quoi lire, on voit où taper.
///
/// Le champ et ses deux boutons sont la seule façon de répondre à un
/// programme qui demande quelque chose : s'ils manquent, l'exécution
/// reste suspendue sans qu'on puisse rien y faire.
#[test]
fn a_program_waiting_for_input_shows_where_to_type() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
section .bss
buf resb 64
section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, buf
mov rdx, 8
syscall
mov rax, 60
mov rdi, 0
syscall
"
        .to_owned(),
        // La hauteur qu'on obtient en tirant la poignée : c'est dans cet
        // état que le champ avait disparu.
        console_height: Some(128.0),
        ..Default::default()
    };
    app.do_assemble();
    app.do_run();
    for _ in 0..40 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        matches!(app.status, Status::WaitingForInput),
        "obtenu {:?}",
        app.status
    );

    let s = strings(app.lang);
    let ctx = egui::Context::default();
    // Deux formats : une fenêtre haute et une fenêtre large. La saisie
    // est en bas d'un panneau lui-même en bas ; c'est le genre de chose
    // qu'une hauteur mal comptée pousse hors de l'écran.
    for (largeur, hauteur) in [(1280.0, 1400.0), (1999.0, 1208.0)] {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, hauteur),
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
        for attendu in [s.console_send, s.console_eof, s.console_waiting] {
            assert!(
                peint.contains(attendu),
                "« {attendu} » manque en {largeur}x{hauteur} :\n{peint}"
            );
        }
    }
}

/// La ligne de saisie s'ouvre dès qu'on sait que le programme lira.
///
/// Un binaire lié dynamiquement ne fait aucun `syscall` de lecture : il
/// appelle `fgets` ou `scanf`, que namsdec fournit lui-même. Le scan des
/// appels système ne voyait donc rien, et la ligne n'apparaissait qu'au
/// moment où le programme se bloquait — après coup.
#[test]
fn the_input_line_opens_as_soon_as_we_know_the_program_will_read() {
    // Les noms que l'éditeur de liens écrit de dix façons.
    for nom in [
        "fgets",
        "scanf",
        "__isoc99_scanf",
        "gets@GLIBC_2.2.5",
        "read",
        "getchar",
    ] {
        assert!(
            namsdec_emu::name_reads_input(nom),
            "« {nom} » réclame une entrée"
        );
    }
    for nom in ["printf", "puts", "strcmp", "malloc", "exit"] {
        assert!(
            !namsdec_emu::name_reads_input(nom),
            "« {nom} » ne réclame rien"
        );
    }

    // Et sur un vrai binaire : celui-ci demande un mot de passe, sans
    // jamais faire de syscall de lecture lui-même.
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    assert!(
        app.program_reads,
        "namsdec doit savoir qu'il demandera quelque chose avant de le lancer"
    );

    // La ligne est donc là dès le chargement, sans avoir rien exécuté.
    let s = strings(app.lang);
    let peint = paint(&mut app);
    for attendu in [s.console_send, s.console_eof] {
        assert!(peint.contains(attendu), "« {attendu} » manque :\n{peint}");
    }
}

/// Les arguments d'un programme se donnent, et arrivent sur sa pile.
///
/// Beaucoup de programmes — et la plupart des crackmes — lisent leur mot
/// de passe dans `argv[1]` plutôt que sur l'entrée standard. Le chargeur
/// posait une pile avec `argc = 1` et rien d'autre : on ne pouvait
/// qu'assister à un « usage: … » sans jamais rien pouvoir tenter.
#[test]
fn a_program_can_be_given_its_arguments() {
    // La ligne se découpe comme un interpréteur le ferait.
    assert_eq!(split_args(""), Vec::<String>::new());
    assert_eq!(split_args("  "), Vec::<String>::new());
    assert_eq!(split_args("un deux"), vec!["un", "deux"]);
    assert_eq!(split_args("  un   deux  "), vec!["un", "deux"]);
    // Les guillemets groupent : un mot de passe peut contenir une espace.
    assert_eq!(
        split_args("\"mon mot de passe\" autre"),
        vec!["mon mot de passe", "autre"]
    );
    assert_eq!(split_args("'a b' c"), vec!["a b", "c"]);
    // Une chaîne vide entre guillemets reste un argument.
    assert_eq!(split_args("\"\" x"), vec!["", "x"]);

    // Et sur un vrai binaire : ce qu'on donne se retrouve dans argv.
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp {
        program_args: "sesame \"deux mots\"".to_owned(),
        ..Default::default()
    };
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");

    let binaire = app
        .assemble_state
        .as_ref()
        .and_then(|état| état.from_binary.as_ref())
        .expect("un binaire chargé");
    assert_eq!(
        binaire.load.argv,
        vec![NOM_FABRIQUÉ, "sesame", "deux mots"],
        "argv[0] est le nom, puis ce qu'on a donné"
    );

    // La pile le dit aussi : argc vaut trois.
    let emulator = app.emulator.as_ref().expect("un émulateur");
    let rsp = emulator.cpu().regs.get64(Reg64::Rsp);
    let argc = emulator.cpu().mem.read_u64(rsp).expect("argc");
    assert_eq!(
        argc, 3,
        "argc compte le nom du programme et les deux autres"
    );
}

/// Les deux lignes de la zone de saisie ne se chevauchent pas.
///
/// Le champ des arguments avait une largeur devinée — « la place
/// disponible moins cent quarante » —, si bien que la ligne débordait du
/// cadre par le haut et que « Recharger » sortait par la droite. Deux
/// libellés de largeurs différentes faisaient en outre commencer les
/// champs à des abscisses différentes.
///
/// Les arguments se replient par défaut depuis le 4 novembre 2026 ; ce
/// contrôle les ouvre, puisque c'est leur ALIGNEMENT qu'il mesure.
#[test]
fn the_input_rows_line_up_and_stay_inside_their_frame() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp {
        console_args_open: true,
        ..Default::default()
    };
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");

    let s = strings(app.lang);
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1999.0, 1208.0),
        )),
        ..Default::default()
    };
    // Où chaque texte est peint, et dans quel rectangle il est découpé.
    let mut vus: std::collections::BTreeMap<String, (egui::Pos2, egui::Rect)> =
        std::collections::BTreeMap::new();
    for _ in 0..3 {
        vus.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                vus.insert(
                    texte.galley.text().to_owned(),
                    (texte.pos, clipped.clip_rect),
                );
            }
        }
        sortie.textures_delta.clear();
    }

    let (args, _) = vus
        .get(s.console_args)
        .copied()
        .expect("le libellé des arguments");
    let (entrée_lbl, cadre) = vus
        .get(s.console_input_label)
        .copied()
        .expect("le libellé de l'entrée");
    let (recharger, _) = vus
        .get(s.console_args_apply)
        .copied()
        .expect("le bouton Recharger");
    let (envoyer, _) = vus.get(s.console_send).copied().expect("le bouton Envoyer");

    // Les deux libellés commencent à la même abscisse.
    assert!(
        (args.x - entrée_lbl.x).abs() < 0.5,
        "les libellés doivent s'aligner : {} contre {}",
        args.x,
        entrée_lbl.x
    );
    // Les arguments sont au-dessus de l'entrée, sur deux lignes
    // distinctes.
    assert!(
        args.y < entrée_lbl.y,
        "les arguments viennent avant l'entrée : {args:?} / {entrée_lbl:?}"
    );
    // Et les deux boutons de droite tiennent dans le cadre.
    for (nom, pos) in [("Recharger", recharger), ("Envoyer", envoyer)] {
        assert!(
            cadre.contains(pos),
            "« {nom} » sort du cadre : {pos:?} hors de {cadre:?}"
        );
    }
    // Les boutons de droite sont bien à droite des champs.
    assert!(recharger.x > args.x + 200.0, "{recharger:?}");
}

/// Les variables d'environnement se donnent, et l'emportent.
///
/// Le chargeur écrivait cinq variables en dur : un programme qui lit
/// `getenv("SECRET")` ne pouvait rien recevoir.
#[test]
fn a_program_can_be_given_its_environment() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp {
        program_env: "SECRET=sesame\n\nLANG=fr_FR.UTF-8\n".to_owned(),
        ..Default::default()
    };
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");

    // On relit la pile là où le programme ira les chercher : après argv
    // et son zéro terminal.
    let emulator = app.emulator.as_ref().expect("un émulateur");
    let cpu = emulator.cpu();
    let rsp = cpu.regs.get64(Reg64::Rsp);
    let argc = cpu.mem.read_u64(rsp).expect("argc");
    let mut at = rsp + 8 + argc * 8 + 8; // argc, argv…, le zéro
    let mut vues = Vec::new();
    loop {
        let pointeur = cpu.mem.read_u64(at).expect("une variable");
        if pointeur == 0 {
            break;
        }
        let mut texte = Vec::new();
        let mut octet = pointeur;
        while let Ok(o) = cpu.mem.peek_bytes(octet, 1) {
            if o[0] == 0 {
                break;
            }
            texte.push(o[0]);
            octet += 1;
        }
        vues.push(String::from_utf8_lossy(&texte).into_owned());
        at += 8;
        assert!(vues.len() < 32, "l'environnement se termine");
    }

    assert!(
        vues.iter().any(|v| v == "SECRET=sesame"),
        "la variable ajoutée doit être là : {vues:?}"
    );
    // Celle qu'on redéfinit l'emporte, et n'apparaît qu'une fois.
    assert!(
        vues.iter().any(|v| v == "LANG=fr_FR.UTF-8"),
        "la variable redéfinie doit être la nôtre : {vues:?}"
    );
    assert!(
        !vues.iter().any(|v| v == "LANG=C"),
        "celle d'origine ne doit pas rester à côté : {vues:?}"
    );
    // Et celles qu'on n'a pas touchées restent.
    assert!(vues.iter().any(|v| v.starts_with("PATH=")), "{vues:?}");
    // Les lignes vides ne deviennent pas des variables vides.
    assert!(!vues.iter().any(String::is_empty), "{vues:?}");
}

/// Un binaire qui attend montre TOUTE la zone de saisie.
///
/// Sur un binaire, cette zone porte quatre rangées — la mention
/// d'attente, les arguments, l'en-tête de l'environnement, l'entrée —
/// alors que le panneau n'en réservait qu'une. Le champ et ses boutons
/// se faisaient rogner par le bas : il ne restait que « Variables
/// d'environnement », au moment précis où le programme demandait
/// quelque chose.
///
/// Le test précédent ne l'a pas vu parce qu'il assemblait un source, où
/// les deux rangées du milieu n'existent pas.
///
/// **LES ARGUMENTS SONT DEMANDÉS EXPRÈS** depuis le 4 novembre 2026 :
/// ils ne se déplient plus d'eux-mêmes — ils ne se tapent pas pendant
/// qu'un programme tourne, et ils prenaient deux rangées sur quatre à
/// la sortie (voir `Settings::console_args_open`). Ce contrôle les
/// ouvre pour mesurer ce qu'il mesurait : la zone ENTIÈRE, quand elle
/// est entière.
#[test]
fn a_waiting_binary_shows_the_whole_input_area() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp {
        console_args_open: true,
        ..Default::default()
    };
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    app.do_run();
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        matches!(app.status, Status::WaitingForInput),
        "le programme doit attendre : {:?}",
        app.status
    );

    let s = strings(app.lang);
    let ctx = egui::Context::default();
    // Deux formats, dont celui de la fenêtre où le défaut se voyait.
    for (largeur, hauteur) in [(1999.0, 1208.0), (1280.0, 900.0)] {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, hauteur),
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
        // Les quatre rangées, toutes présentes.
        for attendu in [
            s.console_waiting,
            s.console_args,
            s.console_env,
            s.console_input_label,
            s.console_send,
            s.console_eof,
        ] {
            assert!(
                peint.contains(attendu),
                "« {attendu} » manque en {largeur}x{hauteur} :\n{peint}"
            );
        }
    }
}

/// La zone de saisie reste à l'écran, même l'environnement déplié.
///
/// Sa hauteur ne se devine pas : le bloc des variables se déplie, et ce
/// qu'il découvre — un texte d'aide et un champ de deux lignes — n'était
/// compté nulle part. Le panneau restait à la hauteur qu'on lui avait
/// donnée à la poignée, et le champ d'entrée passait dessous.
///
/// Le test regarde **où** le champ est peint, non s'il l'est : egui
/// dessine aussi ce qu'une zone défilante découpe, et chercher le texte
/// ne prouve donc rien.
#[test]
fn the_input_area_stays_on_screen_when_the_environment_unfolds() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp {
        // La hauteur qu'on obtient en tirant la poignée : une hauteur
        // figée ne consulte plus celle que la console voudrait.
        console_height: Some(128.0),
        ..Default::default()
    };
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    app.do_run();
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(matches!(app.status, Status::WaitingForInput));

    let s = strings(app.lang);
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1999.0, 1208.0));
    let entrée = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    // Où le libellé de l'entrée est peint, et dans quel rectangle il est
    // découpé.
    let situer = |app: &mut NamsdecApp| -> Option<(egui::Pos2, egui::Rect)> {
        let mut vu = None;
        for _ in 0..3 {
            vu = None;
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape
                    && texte.galley.text() == s.console_input_label
                {
                    vu = Some((texte.pos, clipped.clip_rect));
                }
            }
            sortie.textures_delta.clear();
        }
        vu
    };

    for (état, déplié) in [("replié", false), ("déplié", true)] {
        app.console_env_open = déplié;
        let (position, découpe) = situer(&mut app).expect("le libellé de l'entrée est peint");
        assert!(
            écran.contains(position),
            "{état} : le champ est peint hors de l'écran, en {position:?}"
        );
        // Et il n'est pas rogné par le découpage de son panneau : ce qui
        // dépasse est dessiné, mais invisible.
        assert!(
            découpe.contains(position),
            "{état} : le champ tombe hors de ce que son panneau montre — \
             {position:?} hors de {découpe:?}"
        );
    }
}

/// La boîte de saisie s'ouvre quand le programme attend, et répond.
///
/// La ligne de la console suffit quand on la voit. Elle ne suffit pas
/// quand une fenêtre d'analyse couvre l'écran, quand la console est
/// repliée, ou quand on ne sait pas encore qu'elle existe — et l'on
/// reste devant un programme qui ne répond plus, sans savoir qu'il
/// attend *nous*.
#[test]
fn the_input_dialog_opens_when_the_program_waits_and_shows_what_it_asked() {
    let s = strings(Lang::Fr);
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    assert!(!app.show_input_dialog, "rien tant que rien n'attend");

    app.do_run();
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        matches!(app.status, Status::WaitingForInput),
        "{:?}",
        app.status
    );

    let peint = paint(&mut app);
    assert!(app.show_input_dialog, "la boîte s'ouvre d'elle-même");
    assert!(peint.contains(s.input_dialog_title), "son titre :\n{peint}");
    // Ce que le programme vient d'écrire : c'est l'invite, et c'est la
    // seule chose qui dise ce qu'on attend de nous.
    assert!(
        peint.contains("Enter the password:"),
        "l'invite du programme doit être montrée :\n{peint}"
    );
    assert!(
        peint.contains(s.console_send),
        "et de quoi répondre :\n{peint}"
    );
    assert!(peint.contains(s.console_eof), "{peint}");

    // « Plus tard » la referme sans rien envoyer : le programme attend
    // toujours, et la ligne de la console reste.
    app.show_input_dialog = false;
    assert!(
        matches!(app.status, Status::WaitingForInput),
        "{:?}",
        app.status
    );
    let peint = paint(&mut app);
    assert!(
        !peint.contains(s.input_dialog_asked),
        "refermée, elle ne revient pas toute seule :\n{peint}"
    );
    assert!(
        peint.contains(s.console_input_label),
        "et l'autre chemin est toujours là :\n{peint}"
    );

    // Et une fois la réponse envoyée, la boîte change de rôle : elle ne
    // recueille plus rien, elle montre ce que le programme a écrit. Son
    // titre doit le dire — « Entrée du programme » au-dessus d'une
    // sortie est un contresens, et c'est celui qu'une capture a trouvé.
    app.show_input_dialog = true;
    app.input_dialog_text = "coucou".to_owned();
    app.send_input(Some("coucou".to_owned()));
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        !matches!(app.status, Status::WaitingForInput),
        "le programme a repris : {:?}",
        app.status
    );
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.input_dialog_title_done),
        "le titre dit que c'est une sortie :\n{peint}"
    );
    // Et plus l'ancien. Comparé texte entier : l'invite de la console
    // commence par la même phrase, et un `contains` accuserait celle-ci
    // — un contrôle qui ne peut pas passer n'apprend rien.
    assert!(
        !peint
            .split('|')
            .any(|écrit| écrit.trim() == s.input_dialog_title),
        "et non une entrée :\n{peint}"
    );
}

/// Après la réponse, la boîte montre ce que le programme a répondu.
///
/// C'est tout l'intérêt de la garder ouverte. On tape « sesame », le
/// programme écrit « Incorrect password » et s'arrête : refermer la
/// boîte renvoyait devant une console souvent repliée, où l'on ne
/// voyait rien de ce qu'on venait de provoquer — on croyait que rien ne
/// s'était passé.
#[test]
fn after_the_answer_the_dialog_shows_what_the_program_said() {
    let s = strings(Lang::Fr);
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    // La console repliée, comme sur la capture qui a servi à trouver le
    // défaut : rien d'autre que la boîte ne dira ce qui s'est passé.
    app.show_console = false;

    app.do_run();
    for _ in 0..8000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        matches!(app.status, Status::WaitingForInput),
        "{:?}",
        app.status
    );
    let peint = paint(&mut app);
    assert!(peint.contains("Enter the password:"), "l'invite :\n{peint}");

    // On répond, et le programme va au bout.
    app.send_input(Some("sesame".to_owned()));
    for _ in 0..8000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        matches!(app.status, Status::Exited { .. }),
        "{:?}",
        app.status
    );

    let peint = paint(&mut app);
    assert!(app.show_input_dialog, "la boîte reste ouverte");
    assert!(
        peint.contains("Incorrect password"),
        "elle doit montrer ce que le programme a répondu :\n{peint}"
    );
    assert!(
        peint.contains(s.input_dialog_answered),
        "et le dire comme tel :\n{peint}"
    );
    // Plus rien à taper : un seul bouton.
    assert!(peint.contains(s.dialog_ok), "{peint}");
    // DANS LA BOÎTE, ET NON SUR TOUT L'ÉCRAN.
    //
    // Cette assertion portait sur l'écran entier, et elle passait pour
    // une mauvaise raison : la console porte sa PROPRE rangée
    // « Entrée : / Fin d'entrée / Envoyer », qui a tout droit d'être
    // là — et elle n'était pas peinte parce que la console manquait de
    // onze pixels. Le jour où ces pixels lui ont été rendus (voir
    // `console_wanted_height`), le contrôle a rougi sans qu'aucun
    // comportement ait changé.
    //
    // Ce qu'il veut dire est : la BOÎTE ne propose plus d'envoyer. On
    // découpe donc son bloc, qui va de la phrase qui annonce la réponse
    // jusqu'à la fin de ce qui est peint.
    let début = peint
        .find(s.input_dialog_answered)
        .expect("la boîte est peinte");
    let boîte = &peint[début..];
    assert!(
        !boîte.contains(s.console_send),
        "on ne propose plus d'envoyer à un programme terminé :\n{boîte}"
    );

    // Relancer la referme : elle parlait de la course précédente.
    app.do_run();
    assert!(!app.show_input_dialog, "une nouvelle course repart à zéro");
}

/// La ligne de saisie est là dès qu'un programme est assemblé, même
/// s'il ne lit rien encore.
///
/// C'est le cas qui compte le plus, et c'est celui qui manquait : on
/// écrit son programme, on a posé ses chaînes, et l'on veut préparer ce
/// qu'il lira avant d'avoir tapé le `read`. La ligne n'apparaissait que
/// si namsdec avait *repéré* une lecture dans le code — il n'y avait
/// donc nulle part où taper, et rien à l'écran n'expliquait pourquoi.
#[test]
fn the_input_line_is_there_even_before_the_program_reads() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    // Le squelette qu'on a sous les yeux en commençant : des données, et
    // une sortie. Aucune lecture.
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
invite db \"Entrez un nombre : \", 10
section .text
_start:
    mov rax, 60
    xor edi, edi
    syscall
"
        .to_owned(),
        show_console: true,
        console_tab: ConsoleTab::Output,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    assert!(
        !app.program_reads,
        "ce programme ne lit rien, et c'est tout le point du test"
    );

    let peint = paint(&mut app);
    assert!(
        peint.contains(s.console_input_label),
        "la ligne « {} » doit être là :\n{peint}",
        s.console_input_label
    );
    assert!(
        peint.contains(s.console_send),
        "avec son bouton d'envoi :\n{peint}"
    );
    // Et l'application dit pourquoi ce qu'on tape attendra, au lieu de
    // faire disparaître le champ.
    assert!(
        peint.contains(s.console_no_read),
        "et l'explication :\n{peint}"
    );

    // Ce qu'on y tape part vraiment dans l'entrée du programme.
    app.console_input = "42".to_owned();
    let ligne = std::mem::take(&mut app.console_input);
    app.send_input(Some(ligne));
    assert_eq!(
        app.emulator.as_ref().expect("un émulateur").pending_input(),
        3,
        "« 42 » et son retour à la ligne attendent d'être lus"
    );
}

/// La console dit pourquoi elle est vide, et laisse toujours taper.
///
/// Elle ne proposait de saisie qu'aux programmes où namsdec avait
/// *repéré* un `read`, au nom d'une économie : « un champ, deux boutons
/// et une invite devant un programme qui ne lit pas, c'est trois lignes
/// qui ne serviront jamais ». Le raisonnement tombait précisément dans
/// le cas le plus fréquent — celui de quelqu'un qui écrit son programme,
/// a posé ses chaînes, et veut préparer ce qu'il lira avant d'avoir tapé
/// le `read`. Il n'y avait alors nulle part où taper, et rien à l'écran
/// n'expliquait pourquoi.
///
/// La règle est donc inversée : la ligne est là dès qu'un programme est
/// assemblé, et une phrase dit ce que la saisie deviendra quand rien ne
/// la lit encore. Un champ qui n'apparaît qu'à une condition invisible
/// n'est pas une économie, c'est une disparition.
///
/// Ce qu'affiche la console *vide*, en revanche, garde son ancienne
/// règle, et elle est juste : « le programme n'a rien écrit » devant un
/// programme qui n'écrit jamais rien laisse chercher une panne qui
/// n'existe pas.
#[test]
fn the_console_says_why_it_is_empty_and_always_lets_one_type() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let peindre = |source: &str, exécuter: bool| {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            console_tab: ConsoleTab::Output,
            ..Default::default()
        };
        app.do_assemble();
        assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
        if exécuter {
            app.do_run();
            for _ in 0..50 {
                if !app.running {
                    break;
                }
                app.tick_run(&egui::Context::default());
            }
        }
        (paint(&mut app), app.program_reads, app.program_writes)
    };

    // Un programme qui ne fait que calculer : il a quand même sa ligne
    // de saisie, et une phrase qui dit qu'elle attendra.
    let muet = "global _start\nsection .text\n_start:\n    mov rax, 2\n    add rax, 3\n    hlt\n";
    let (peint, lit, écrit) = peindre(muet, true);
    assert!(!lit && !écrit, "ce programme ne parle à personne");
    assert!(
        peint.contains(s.console_send),
        "on doit pouvoir lui parler quand même :\n{peint}"
    );
    assert!(
        peint.contains(s.console_no_read),
        "et l'application dit ce que la saisie deviendra :\n{peint}"
    );
    assert!(
        peint.contains(s.output_never),
        "et la console dit pourquoi elle est vide :\n{peint}"
    );

    // Le même, pas encore lancé : « rien pour l'instant », et non « il
    // n'a rien écrit » — il n'a rien fait du tout.
    let écrivain = "section .data\n    msg db \"x\", 10\nsection .text\nglobal _start\n_start:\n\
                    \x20   mov rax, 1\n    mov rdi, 1\n    mov rsi, msg\n    mov rdx, 2\n\
                    \x20   syscall\n    hlt\n";
    let (peint, _, écrit) = peindre(écrivain, false);
    assert!(écrit, "celui-ci écrit");
    assert!(
        peint.contains(s.output_not_yet),
        "avant l'exécution, la console ne reproche rien :\n{peint}"
    );

    // Et un programme qui lit garde son champ, sans la phrase — il n'y a
    // rien à expliquer, il lira.
    let lecteur = "section .bss\n    tampon resb 16\nsection .text\nglobal _start\n_start:\n\
                   \x20   xor rax, rax\n    xor rdi, rdi\n    mov rsi, tampon\n    mov rdx, 8\n\
                   \x20   syscall\n    hlt\n";
    let (peint, lit, _) = peindre(lecteur, false);
    assert!(lit, "celui-ci lit");
    assert!(
        peint.contains(s.console_send),
        "un programme qui lit garde de quoi lui parler :\n{peint}"
    );
    assert!(
        !peint.contains(s.console_no_read),
        "et sans la phrase, qui n'aurait plus d'objet :\n{peint}"
    );
}

/// Le champ de saisie est visible quelle que soit la hauteur de la console.
///
/// Le défaut s'était vu en photo : le programme affichait « Enter the
/// password: », la barre du bas disait « En attente d'une entrée… », et il
/// n'y avait nulle part où taper. Seul l'en-tête replié de l'environnement
/// dépassait — le reste du cadre passait sous le bord de la fenêtre.
///
/// Un test qui ne vérifierait qu'une hauteur ne prouverait rien : la
/// console se tire à la poignée, et c'est justement à certaines hauteurs
/// que le compte tombait faux. On les balaie donc, celle qu'aucune main
/// n'a fixée comprise.
#[test]
fn the_input_row_is_visible_at_every_console_height() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let s = strings(Lang::Fr);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1999.0, 1208.0));
    // Ce qu'il faut pouvoir lire et atteindre pour répondre au programme.
    let rangées = [
        s.console_waiting,
        s.console_args,
        s.console_env,
        s.console_input_label,
        s.console_send,
        s.console_eof,
    ];

    for hauteur in [None, Some(96.0), Some(128.0), Some(200.0), Some(260.0)] {
        for env_ouvert in [false, true] {
            let mut app = NamsdecApp {
                console_height: hauteur,
                console_env_open: env_ouvert,
                // LES ARGUMENTS OUVERTS, parce que c'est la zone
                // ENTIÈRE qu'on balaie ici — six rangées, la pire des
                // configurations pour la hauteur. Depuis le 4 novembre
                // 2026 ils se replient par défaut (voir
                // `Settings::console_args_open`), mais rien n'empêche
                // de les rouvrir, et c'est alors que le compte doit
                // tenir.
                console_args_open: true,
                ..Default::default()
            };
            assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
            app.do_run();
            for _ in 0..4000 {
                if !app.running {
                    break;
                }
                app.tick_run(&egui::Context::default());
            }
            assert!(
                matches!(app.status, Status::WaitingForInput),
                "{hauteur:?} : le programme devrait attendre une entrée — {:?}",
                app.status
            );

            let ctx = egui::Context::default();
            // Le thème posé, et c'est **indispensable** : il change les
            // espacements, donc la hauteur des rangées. Sans lui, ce test
            // passait alors que la rangée « Entrée » disparaissait pour de
            // vrai sur la machine, à 128 px de console — il mesurait une
            // application qui n'existe nulle part.
            app.apply_theme(&ctx);
            let entrée = egui::RawInput {
                screen_rect: Some(écran),
                ..Default::default()
            };
            // Quatre images : la zone se mesure elle-même, et la hauteur
            // qu'elle réserve à la suivante vient de ce qu'elle a pris.
            let mut vus: std::collections::BTreeMap<&str, (egui::Rect, egui::Rect)> =
                std::collections::BTreeMap::new();
            for _ in 0..4 {
                vus.clear();
                let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
                for clipped in &sortie.shapes {
                    if let egui::epaint::Shape::Text(texte) = &clipped.shape
                        && let Some(rangée) = rangées.iter().find(|r| **r == texte.galley.text())
                    {
                        // Le rectangle ENTIER du texte, et non sa position.
                        // C'est le trou qu'avait ce test : `pos` est le coin
                        // supérieur gauche, si bien qu'une rangée dont seul
                        // le haut dépassait dans le panneau passait pour
                        // visible — alors qu'à l'écran elle était coupée en
                        // deux, champ et boutons sous le bord.
                        let boîte = egui::Rect::from_min_size(texte.pos, texte.galley.size());
                        vus.insert(rangée, (boîte, clipped.clip_rect));
                    }
                }
                sortie.textures_delta.clear();
            }

            let état = if env_ouvert { "déplié" } else { "replié" };
            for rangée in rangées {
                // Le rectangle entier, et non la seule présence : egui peint
                // aussi ce qu'un découpage escamote, et chercher le texte ne
                // prouverait donc rien.
                let (boîte, découpe) = vus.get(rangée).copied().unwrap_or_else(|| {
                    panic!(
                        "{hauteur:?}, environnement {état} : « {rangée} » n'est pas peint du tout"
                    )
                });
                assert!(
                    écran.contains_rect(boîte),
                    "{hauteur:?}, environnement {état} : « {rangée} » déborde de l'écran — {boîte:?}"
                );
                assert!(
                    découpe.contains_rect(boîte),
                    "{hauteur:?}, environnement {état} : « {rangée} » est coupée — {boîte:?} \
                 dépasse de {découpe:?}. À l'écran, on en voit le haut et rien d'autre."
                );
            }
        }
    }
}

/// Entrée envoie la ligne au programme — sans toucher à la souris.
///
/// La boîte de saisie s'ouvre quand le programme attend, prend le
/// clavier, et Entrée envoie. Avant elle, ce chemin passait par la ligne
/// de la console ; il existe toujours, et il est vérifié plus bas.
///
/// Le piège que ce test garde : un `TextEdit` qui traite Entrée la
/// retire de la file d'évènements, si bien que la guetter *après* lui ne
/// voit plus rien. Le champ gardait le texte, le programme continuait
/// d'attendre, et il fallait cliquer « Envoyer ».
#[test]
fn pressing_enter_sends_the_line_to_the_program() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    app.do_run();
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        matches!(app.status, Status::WaitingForInput),
        "{:?}",
        app.status
    );

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1100.0));
    let base = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    // Deux images : la boîte s'ouvre, son champ réclame le clavier et
    // l'obtient.
    for _ in 0..2 {
        let mut sortie = ctx.run_ui(base.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    assert!(app.show_input_dialog, "la boîte s'ouvre d'elle-même");
    assert!(
        ctx.memory(|m| m.focused()).is_some(),
        "le curseur doit aller tout seul là où il faut taper"
    );

    // On tape, sans avoir rien cliqué.
    let mut frappe = base.clone();
    frappe.events.push(egui::Event::Text("secret".to_owned()));
    let mut sortie = ctx.run_ui(frappe, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    assert_eq!(
        app.input_dialog_text, "secret",
        "ce qu'on tape arrive dans le champ de la boîte"
    );

    // Puis Entrée.
    let mut entrée = base.clone();
    entrée.events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
    sortie.textures_delta.clear();

    assert!(
        app.input_dialog_text.is_empty(),
        "le champ se vide : la ligne est partie — {:?}",
        app.input_dialog_text
    );
    assert!(
        app.show_input_dialog,
        "et la boîte reste : c'est elle qui montrera ce que le programme répond"
    );
    assert!(
        !matches!(app.status, Status::WaitingForInput),
        "et le programme repart au lieu d'attendre encore : {:?}",
        app.status
    );

    // L'autre chemin marche toujours : la ligne de la console, pour qui
    // préfère ne pas voir de boîte.
    app.do_run();
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    if matches!(app.status, Status::WaitingForInput) {
        app.show_input_dialog = false;
        app.console_input = "encore".to_owned();
        let ligne = std::mem::take(&mut app.console_input);
        app.send_input(Some(ligne));
        assert!(
            !matches!(app.status, Status::WaitingForInput),
            "la ligne de la console envoie aussi : {:?}",
            app.status
        );
    }
}

/// Pendant l'attente, la zone de saisie se voit sans qu'on la cherche.
///
/// C'est le seul moment où l'application demande quelque chose ; la
/// manquer, c'est croire qu'elle a planté. Un liseré d'un pixel sur un
/// panneau déjà sombre passait pour une nuance.
#[test]
fn the_waiting_input_area_announces_itself() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ));
    app.do_run();
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let accent = app.theme.accent;
    let mut peint = String::new();
    let mut liserés = 0usize;
    let mut pastilles = 0usize;
    for _ in 0..3 {
        peint.clear();
        liserés = 0;
        pastilles = 0;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(texte) => {
                    peint.push_str(texte.galley.text());
                    peint.push('|');
                }
                // Le cadre : deux pixels d'accent, et non plus un.
                egui::epaint::Shape::Rect(rect)
                    if rect.stroke.color == accent && rect.stroke.width >= 2.0 =>
                {
                    liserés += 1;
                }
                egui::epaint::Shape::Circle(cercle) if cercle.fill == accent => {
                    pastilles += 1;
                }
                _ => {}
            }
        }
        sortie.textures_delta.clear();
    }

    let s = strings(Lang::Fr);
    assert!(liserés >= 1, "le cadre d'attente doit être franc");
    assert!(pastilles >= 1, "la pastille doit accrocher l'oeil");
    // Et le geste à faire est écrit, pas seulement suggéré.
    assert!(
        peint.contains(s.console_waiting_how),
        "il faut dire quoi faire : « {} »\n{peint}",
        s.console_waiting_how
    );
}

/// La boîte d'entrée montre ce que le programme a écrit EN ENTIER.
///
/// Quatre lignes et 480 pixels de large : un logo de sept lignes sur
/// quatre-vingts colonnes — ce par quoi commence la moitié des
/// crackmes — y perdait ses trois premières lignes et sa moitié
/// droite. On lisait un fragment, et rien ne disait que c'en était un.
///
/// Le contrôle porte sur `lignes_qui_tiennent`, qui décide de ce que la
/// boîte peut montrer : c'est là qu'est la règle, la fenêtre ne fait
/// que la suivre. Il part de la sortie BRUTE du programme, séquences
/// comprises, parce que c'est elle qui arrive.
#[test]
fn the_input_box_shows_everything_the_program_wrote() {
    use crate::console::lignes_qui_tiennent;

    // Un logo en couleur, puis une invite : ce que montre un crackme au
    // démarrage, tel que l'émulateur le rend — avec son effacement
    // d'écran et son cyan.
    let brut = "\u{1b}[2J\u{1b}[H\u{1b}[36m                 _____ ____   ___   ____ __  __    _    _   _\n                | ____|  _ \\ / _ \\ / ___|  \\/  |  / \\  | \\ | |\n                |  _| | |_) | | | | |  _| |\\/| | / _ \\ |  \\| |\n                |_|   |_| \\_\\\\___/ \\____|_|  |_|/_/ \\_\\|_| \\_|\n                \u{1b}[0m\n                IDENTIFIER : ACCESS KEY:";
    let écran = crate::ansi::écran(brut.as_bytes());
    let largeurs: Vec<usize> = écran
        .lignes
        .iter()
        .map(|ligne| ligne.iter().map(|m| m.texte.chars().count()).sum())
        .collect();
    let total = largeurs.len();
    assert!(total >= 5, "le logo fait plusieurs lignes : {total}");

    // De la place pour tout : tout y est, rien n'est laissé derrière.
    assert_eq!(lignes_qui_tiennent(&largeurs, 120, 40), total);

    // Trop étroit : ce qui dépasse se REPLIE, donc compte double en
    // hauteur. Une boîte de six lignes ne peut plus porter les six du
    // logo.
    let prises = lignes_qui_tiennent(&largeurs, 24, 6);
    assert!(
        prises < total,
        "une boîte étroite laisse des lignes derrière"
    );
    // Et ce qu'elle garde est LA FIN — l'invite est toujours en bas.
    let vue: Vec<&Vec<crate::ansi::Morceau>> = écran.lignes[total - prises..].iter().collect();
    let dernière: String = vue
        .last()
        .map(|l| l.iter().map(|m| m.texte.as_str()).collect())
        .unwrap_or_default();
    assert!(
        dernière.ends_with("IDENTIFIER : ACCESS KEY:"),
        "l'invite doit rester : « {dernière} »"
    );

    // Une boîte d'une seule ligne en montre quand même une : une boîte
    // vide devant un programme qui attend n'apprend rien.
    assert_eq!(lignes_qui_tiennent(&largeurs, 10, 1), 1);
    // Et les cas limites ne paniquent pas.
    assert_eq!(lignes_qui_tiennent(&[], 80, 10), 0);
    assert_eq!(lignes_qui_tiennent(&[3], 0, 0), 1);
}

/// La console montre les COULEURS du programme, et plus ses séquences.
///
/// Le grief se voyait sur deux captures : le logo de FROGMAN ouvre par
/// « ESC[2J ESC[H ESC[36m », et l'on lisait ces trois-là en toutes
/// lettres — « □[2J□[H□[36m » — avant le dessin, dans la console comme
/// dans la boîte de saisie.
///
/// Le contrôle assemble un programme qui écrit en couleur, l'exécute
/// pour de vrai, et mesure CE QUI EST PEINT : le texte d'un côté, la
/// couleur de ses galettes de l'autre. Vérifier l'analyseur seul ne
/// dirait pas si la couleur arrive à l'écran.
#[test]
fn the_console_paints_the_colours_and_not_the_escapes() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
section .data
msg db 27,\"[2J\",27,\"[H\",27,\"[31mrouge\",27,\"[0m \",27,\"[32mvert\",27,\"[0m\",10
len equ $ - msg

section .text
global _start
_start:
mov rax, 1
mov rdi, 1
mov rsi, msg
mov rdx, len
syscall
hlt
"
        .to_owned(),
        show_console: true,
        ..Default::default()
    };
    app.do_assemble();
    app.do_run();
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    // Ce qui est peint, et de quelle couleur.
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = écran_du_banc();
    let mut galettes: Vec<(String, egui::Color32)> = Vec::new();
    for _ in 0..3 {
        galettes.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        // DANS LA CONSOLE SEULEMENT : l'éditeur montre le source du
        // programme, qui porte « [31m » en toutes lettres — c'est son
        // texte, et l'accuser serait accuser le programme d'être
        // écrit.
        let console = ctx
            .memory(|m| {
                m.data
                    .get_temp::<egui::containers::panel::PanelState>(egui::Id::new("console"))
            })
            .map(|état| état.outer_rect)
            .unwrap_or(egui::Rect::EVERYTHING);
        for clipped in &sortie.shapes {
            if !console.contains(clipped.clip_rect.center()) {
                continue;
            }
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                // Les SECTIONS de la mise en page : c'est là que
                // vivent les couleurs, une par morceau de texte.
                for section in &t.galley.job.sections {
                    let plage = section.byte_range.start.0..section.byte_range.end.0;
                    let morceau = &t.galley.job.text[plage];
                    if !morceau.trim().is_empty() {
                        galettes.push((morceau.to_owned(), section.format.color));
                    }
                }
            }
        }
        sortie.textures_delta.clear();
    }
    let tout: String = galettes.iter().map(|(t, _)| t.as_str()).collect();

    // Plus une seule séquence en toutes lettres.
    assert!(
        !tout.contains("[31m") && !tout.contains("[2J") && !tout.contains("[0m"),
        "les séquences ne doivent plus se lire :\n{tout}"
    );
    // Et les mots sont là, chacun dans SA couleur — celles du thème,
    // parce que les huit couleurs d'un terminal sont des rôles, pas
    // des valeurs.
    let couleur_de = |mot: &str| -> Option<egui::Color32> {
        galettes
            .iter()
            .find(|(t, _)| t.contains(mot))
            .map(|(_, c)| *c)
    };
    let theme = app.theme;
    assert_eq!(
        couleur_de("rouge"),
        Some(theme.bad),
        "« rouge » doit porter le rouge du thème :\n{galettes:?}"
    );
    assert_eq!(
        couleur_de("vert"),
        Some(theme.good),
        "« vert » doit porter le vert du thème :\n{galettes:?}"
    );
}

/// Le panneau des diagnostics ne dit « lancées » que si elles l'ont été.
///
/// Il montre les deux commandes — `nasm` puis `ld` — dès l'ouverture, et
/// c'est tout son intérêt : « et si je veux le faire sans namsdec ? »
/// est la bonne question, et un atelier d'apprentissage y répond. Mais
/// il les annonçait au passé avant le premier assemblage, alors que rien
/// n'avait tourné.
///
/// Vu sur une capture, et seulement parce que les onglets de la console
/// venaient d'entrer au banc : l'écran existait, personne ne l'avait
/// jamais regardé sans programme assemblé.
#[test]
fn les_commandes_ne_sont_dites_lancees_que_si_elles_l_ont_ete() {
    let s = strings(Lang::Fr);
    assert_ne!(
        s.assemble_commands, s.assemble_commands_future,
        "les deux formes doivent différer, sinon la distinction ne sert à rien"
    );

    // Rien d'assemblé : au futur.
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_console: true,
        console_tab: ConsoleTab::Diagnostics,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    assert!(app.assemble_state.is_none());
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
    let peint: Vec<&str> = lignes.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        peint.contains(&s.assemble_commands_future),
        "avant d'assembler, les commandes s'annoncent au futur : {peint:?}"
    );
    assert!(
        !peint.contains(&s.assemble_commands),
        "et surtout pas au passé : {peint:?}"
    );

    // Assemblé : au passé.
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    app.source = PROGRAMME_DU_BANC.to_owned();
    app.do_assemble();
    assert!(app.assemble_state.is_some(), "le programme est assemblé");
    // Assembler ramène la console sur « Sortie » : on revient à
    // l'onglet qu'on mesure. Sans cela, on mesurait un écran qui ne
    // porte pas le sujet — et le contrôle accusait le bon code.
    app.console_tab = ConsoleTab::Diagnostics;
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
    let peint: Vec<&str> = lignes.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        peint.contains(&s.assemble_commands),
        "une fois lancées, elles se disent au passé : {peint:?}"
    );
    assert!(
        !peint.contains(&s.assemble_commands_future),
        "et plus au futur : {peint:?}"
    );

    // Les deux langues portent les deux formes.
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        assert!(!s.assemble_commands.trim().is_empty());
        assert!(!s.assemble_commands_future.trim().is_empty());
    }
}

/// Rien de la zone de saisie n'est coupé ni recouvert, à quelque
/// hauteur que soit la console.
///
/// Sous la rangée « Entrée : », la zone dit encore une chose : combien
/// d'octets attendent d'être lus, ou — quand rien dans le programme ne
/// les lira — ce que la saisie deviendra. Console figée à la poignée et
/// environnement déplié, il n'en restait que TROIS pixels de haut sur
/// quinze : un filet de sommets de lettres au ras de la barre de
/// statut, qu'on prenait pour un trait.
///
/// La phrase n'était comptée nulle part dans `console_input_estimate`,
/// et la mesure de l'image précédente ne pouvait pas rattraper
/// l'oubli : quand la zone réclame plus que la place, egui la RAMÈNE à
/// la place disponible en gardant son bas, si bien que la hauteur
/// relevée est celle qu'on lui a allouée, jamais celle qu'il lui
/// fallait — le panneau restait court parce qu'il était court.
///
/// L'AUTRE MOITIÉ DU CONTRÔLE — et c'est elle qui a trouvé le second
/// défaut. Une zone défilante réclame soixante-quatre pixels quand on
/// ne lui dit rien, et les prend même s'il n'en reste que sept : la
/// fenêtre de sortie s'étendait alors sur la saisie et peignait son
/// fond opaque par-dessus. La rangée des arguments et l'en-tête de
/// l'environnement étaient là, entiers, mesurables — et invisibles.
/// Aucune mesure de découpage ne bouge dans ce cas ; il faut regarder
/// l'ORDRE des formes.
///
/// Éprouvé en remettant chacun des deux défauts : le premier fait
/// rougir le découpage sur quinze scènes, le second le recouvrement.
#[test]
fn nothing_of_the_input_area_is_ever_cut_off_or_covered() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let s = strings(Lang::Fr);
    // Ce que la zone de saisie écrit, d'un bout à l'autre : l'attente,
    // les arguments, l'environnement, la rangée où l'on tape, et la
    // phrase du dessous.
    let à_elle = [
        s.console_waiting,
        s.console_waiting_how,
        s.console_args,
        s.console_args_apply,
        s.console_env,
        s.console_env_hint,
        s.console_input_label,
        s.console_input_hint,
        s.console_send,
        s.console_eof,
        s.console_no_read,
    ];
    let mut coupés: Vec<String> = Vec::new();
    for écran in [700.0_f32, 900.0, 1209.0] {
        // Automatique, puis trois hauteurs figées comme on les fige :
        // en tirant la poignée.
        for figée in [None, Some(130.0_f32), Some(200.0), Some(260.0)] {
            for env in [false, true] {
                for lit in [false, true] {
                    let ctx = egui::Context::default();
                    let mut app = NamsdecApp {
                        show_console: true,
                        console_tab: ConsoleTab::Output,
                        ..Default::default()
                    };
                    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
                    app.console_env_open = env;
                    app.program_reads = lit;
                    app.console_height = figée;
                    app.apply_theme(&ctx);
                    let input = egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1600.0, écran),
                        )),
                        ..Default::default()
                    };
                    // Quatre images : une surface ancrée ne connaît sa
                    // hauteur qu'après s'être disposée une fois, et la
                    // console s'ajuste à ce qu'elle a relevé.
                    for _ in 0..4 {
                        coupés.clear();
                        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                        for (rang, clipped) in output.shapes.iter().enumerate() {
                            let egui::epaint::Shape::Text(t) = &clipped.shape else {
                                continue;
                            };
                            if !à_elle
                                .iter()
                                .any(|texte| t.galley.text().starts_with(texte))
                            {
                                continue;
                            }
                            // Ce qui est peint APRÈS ce texte et le
                            // couvre d'un fond opaque. On s'en tient
                            // aux fonds pleins : le liseré de l'attente
                            // et les survols sont translucides, et ils
                            // ont le droit de passer dessus.
                            let encre_du_texte = egui::Rect::from_min_size(t.pos, t.galley.size());
                            for dessus in output.shapes.iter().skip(rang + 1) {
                                let egui::epaint::Shape::Rect(r) = &dessus.shape else {
                                    continue;
                                };
                                if r.fill.a() < 255 {
                                    continue;
                                }
                                let couvert =
                                    encre_du_texte.intersect(r.rect.intersect(dessus.clip_rect));
                                if couvert.is_positive()
                                    && couvert.height() > encre_du_texte.height() * 0.5
                                    && couvert.width() > encre_du_texte.width() * 0.5
                                {
                                    coupés.push(format!(
                                        "écran {écran:.0}, console {figée:?}, \
                                         environnement {env} : « {} » recouvert par un fond \
                                         {:?} peint après lui",
                                        t.galley.text().chars().take(32).collect::<String>(),
                                        r.fill,
                                    ));
                                }
                            }
                            for ligne in t.galley.rows.iter() {
                                let encre = ligne
                                    .rect_without_leading_space()
                                    .translate(t.pos.to_vec2());
                                if !encre.is_positive() {
                                    continue;
                                }
                                let vu = encre.intersect(clipped.clip_rect);
                                if vu.height() + 0.5 < encre.height() {
                                    coupés.push(format!(
                                        "écran {écran:.0}, console {figée:?}, \
                                         environnement {env} : « {} » {:.0} px de haut sur {:.0}",
                                        ligne.text().chars().take(32).collect::<String>(),
                                        vu.height().max(0.0),
                                        encre.height()
                                    ));
                                }
                            }
                        }
                        output.textures_delta.clear();
                    }
                    assert!(
                        coupés.is_empty(),
                        "la zone de saisie est coupée : {coupés:#?}"
                    );
                }
            }
        }
    }
}

/// La rubrique des simplifications paraît dans Diagnostics quand le
/// programme en a touché une, et pas avant.
///
/// Trois choses mesurées sur ce qui est PEINT, et non sur le code :
/// la rubrique est absente d'un programme qui ne touche pas au
/// coprocesseur ; elle paraît avec son titre et son explication dès
/// qu'un `fsin` est passé ; et aucune de ses lignes n'est rognée en
/// hauteur par la découpe du panneau — une explication de quatre lignes
/// dans une console basse est exactement le cas où une rubrique
/// nouvelle vole la place des diagnostics de nasm.
#[test]
fn les_simplifications_paraissent_dans_les_diagnostics() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    const SANS_X87: &str = "\
section .text
global _start
_start:
    mov rax, 60
    xor edi, edi
    syscall
";
    // `fsin` et rien d'autre : une seule simplification touchée, donc
    // une seule ligne attendue — un programme qui en toucherait quatre
    // ne dirait pas si la rubrique les montre toutes ou se répète.
    const AVEC_X87: &str = "\
section .data
angle dq 1.0
sortie dq 0.0

section .text
global _start
_start:
    fld qword [angle]
    fsin
    fstp qword [sortie]
    hlt
";

    for lang in [Lang::Fr, Lang::En] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            lang,
            show_console: true,
            source: SANS_X87.to_owned(),
            ..Default::default()
        };
        app.apply_theme(&ctx);
        app.do_assemble();
        for _ in 0..40 {
            app.do_step();
        }
        app.console_tab = ConsoleTab::Diagnostics;
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
        let peint = |lignes: &[(String, egui::Rect)], quoi: &str| {
            lignes.iter().any(|(t, _)| t.contains(quoi))
        };
        let annonce = if lang == Lang::Fr {
            "Ce que cette exécution a simplifié"
        } else {
            "What this run simplified"
        };
        assert!(
            !peint(&lignes, annonce),
            "un programme sans x87 ne doit rien annoncer ({lang:?})"
        );

        // Le même écran, avec un programme qui touche le coprocesseur.
        app.source = AVEC_X87.to_owned();
        app.do_assemble();
        for _ in 0..40 {
            app.do_step();
        }
        app.console_tab = ConsoleTab::Diagnostics;
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
        assert!(
            peint(&lignes, annonce),
            "`fsin` a été exécuté : la rubrique doit paraître ({lang:?}) — \
             peint : {:?}",
            lignes.iter().map(|(t, _)| t).take(24).collect::<Vec<_>>()
        );
        let titre = limites::titre(lang, namsdec_emu::Simplification::X87Transcendantes);
        // La puce est devant le titre : on cherche le titre, pas la
        // ligne entière.
        assert!(
            peint(&lignes, titre),
            "le titre de la simplification est peint ({lang:?}) : {titre}"
        );
        let explication =
            limites::explication(lang, namsdec_emu::Simplification::X87Transcendantes);
        // Un fragment, et non la phrase : egui replie l'explication sur
        // plusieurs lignes, et aucune ne porte le texte entier. Pris
        // dans le plus long morceau SANS accent grave, parce que
        // `prose` découpe le texte à chacun pour peindre le code en
        // monospace — un fragment à cheval sur un accent grave ne
        // serait jamais peint d'un seul tenant.
        let début: String = explication
            .split('`')
            .step_by(2)
            .max_by_key(|morceau| morceau.len())
            .unwrap_or(explication)
            .trim()
            .chars()
            .take(18)
            .collect();
        assert!(
            peint(&lignes, &début),
            "l'explication est peinte et non seulement le titre ({lang:?}) : {début}"
        );
    }
}

/// La console est fermée au premier lancement, et paraît dès qu'elle a
/// quelque chose à montrer.
///
/// **Simplifier, c'est retirer ce qui ne sert pas encore.** Avant tout
/// assemblage, la console occupe soixante-douze pixels pour écrire « la
/// sortie du programme s'affichera ici » — c'est-à-dire pour annoncer
/// qu'elle est vide. Sur l'écran qu'un nouveau venu voit en premier, et
/// qui est désormais le mode débutant, cela fait une zone de plus à
/// comprendre pour rien.
///
/// Ce qui rend la fermeture tenable est la seconde moitié de la règle :
/// elle s'ouvre **d'elle-même** au premier octet écrit. Sans cela, un
/// programme qui affiche « salut » n'aurait nulle part où l'afficher, et
/// l'on croirait qu'il n'a rien fait. Les deux moitiés sont mesurées
/// ici ; l'une sans l'autre serait un défaut.
#[test]
fn la_console_parait_quand_le_programme_ecrit() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Une installation neuve : c'est `Settings`, et non
    // `NamsdecApp::default()`, qui dit ce qu'un nouveau venu reçoit.
    assert!(
        !Settings::default().show_console,
        "une installation neuve s'ouvre sans la console"
    );

    let mut app = NamsdecApp {
        source: "\
global _start
section .data
    msg db \"salut\", 10
section .text
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, msg
    mov rdx, 6
    syscall
    hlt
"
        .to_owned(),
        show_console: false,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Tant que rien n'est écrit, elle reste fermée : un pas qui ne
    // produit rien ne doit pas faire surgir un panneau.
    app.do_step();
    assert!(
        !app.show_console,
        "rien n'a encore été écrit : la console n'a pas à paraître"
    );

    // Et au premier octet, elle paraît — sur l'onglet de la sortie.
    for _ in 0..20 {
        app.do_step();
        if app.show_console {
            break;
        }
    }
    assert!(
        app.show_console,
        "le programme a écrit : la console doit paraître"
    );
    assert_eq!(app.console_tab, ConsoleTab::Output);

    // UNE FOIS, ET PAS DAVANTAGE : qui la referme en pleine course a
    // choisi. Sans ce témoin, la règle rouvrirait le panneau sous les
    // doigts à chaque pas suivant, ce qui est pire que de l'avoir
    // laissé ouvert.
    app.show_console = false;
    for _ in 0..5 {
        app.do_step();
    }
    assert!(
        !app.show_console,
        "refermée pendant la course, elle le reste"
    );
}

/// Les arguments ne prennent plus la place de la sortie.
///
/// **Signalé à l'écran le 4 novembre 2026 :** « la console et les autres
/// champs ne sont pas adaptés → inutilisable ». Sur la capture, la
/// sortie d'un programme qui attendait une réponse tenait en une
/// demi-ligne, pendant que les arguments, l'en-tête de l'environnement
/// et la ligne de saisie occupaient quatre rangées.
///
/// Les arguments et l'environnement ne se tapent pas pendant qu'un
/// programme tourne — ils sont posés sur la pile avant la première
/// instruction. C'est le partage des outils de développement, et celui
/// des captures envoyées en modèle : ce qu'on RÈGLE avant de lancer vit
/// à part ; ce qu'on LIT et ce qu'on TAPE pendant la course occupe la
/// fenêtre.
#[test]
fn les_arguments_ne_prennent_plus_la_place_de_la_sortie() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp::default();
    app.load_elf(&octets, NOM_FABRIQUÉ);
    app.show_console = true;
    app.console_tab = ConsoleTab::Output;

    // Une installation neuve les garde fermés.
    assert!(
        !Settings::default().console_args_open,
        "fermés sur une installation neuve"
    );

    // 1. FERMÉS : la rangée n'est pas peinte, mais le bouton qui la
    //    rouvre l'est — une commande qu'on ne voit pas n'existe pas.
    app.console_args_open = false;
    let peint = paint(&mut app);
    assert!(
        !peint.contains(s.console_args),
        "la rangée des arguments ne doit pas être peinte : {peint}"
    );
    assert!(
        peint.contains(s.console_args_toggle),
        "mais le bouton qui la rouvre, oui : {peint}"
    );
    // ET CE QU'ON NE TOUCHE PAS RESTE : la ligne où l'on tape est
    // toujours là, programme en marche ou non. C'est une décision
    // écrite (`console_shows_input`) — « un champ qui n'apparaît qu'à
    // une condition que l'utilisateur ne peut pas voir n'est pas une
    // économie, c'est une disparition » —, et ce lot ne la défait pas :
    // il retire ce qu'on RÈGLE, pas ce qu'on TAPE.
    assert!(
        peint.contains(s.console_input_label) && peint.contains(s.console_send),
        "la ligne de saisie reste : {peint}"
    );

    // 2. DEMANDÉS : la rangée paraît, avec son champ et son bouton.
    app.console_args_open = true;
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.console_args) && peint.contains(s.console_args_apply),
        "demandée, la rangée est là : {peint}"
    );
}
