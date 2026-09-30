//! Les tests de l'interface.
//!
//! Ils dessinent l'application dans un contexte egui sans fenêtre et
//! regardent ce qui est *peint* : la largeur qu'un panneau prend, les
//! textes posés, les cadres des boutons. C'est ce qui permet de vérifier
//! une mise en page sans capture d'écran et sans clic simulé.
//!
//! Sortis de `main.rs`, qui portait autant de tests que de code.

use super::*;
// Le graphe vit dans son propre module depuis le 29 septembre 2026 ;
// deux de ses fonctions libres se mesurent ici.
use crate::flow::{flow_box_width, flow_ellipsis};
// L'éditeur aussi a son module ; le banc mesure trois de ses
// fonctions libres.
use crate::editor::{editor_width, layout_code, word_at};
// Les menus aussi ; le banc appelle la liste des commandes.
use crate::menus::palette_commands;
// L'affichage de la calculatrice a son module, distinct de son
// calcul ; le banc mesure son clavier.
use crate::calc_ui::{CALC_LARGEUR, TOUCHE_H, TOUCHE_L, écriture_de_touche};
// Le panneau latéral a ses propres fonctions : les tests les
// nomment sans préfixe, comme avant leur déménagement.
use crate::sidebar::*;

/// TOUT ce qui dessine l'interface, fichier par fichier.
///
/// Quelques contrôles lisent le code au lieu de le peindre — un bouton
/// posé sans cadre, deux styles de texte qui s'écrasent — parce que la
/// règle est dans le code et non dans les pixels. Ils lisaient
/// `main.rs`, ce qui était vrai tant que tout y vivait.
///
/// Le découpage de septembre 2026 a changé cela en silence : une
/// fonction sortie de `main.rs` sortait aussi de leur regard. Rien n'a
/// rougi — et c'est la pire façon dont un contrôle peut mentir, celle
/// qui ne se voit pas, puisqu'il passe *mieux* qu'avant.
///
/// La liste est donc écrite une fois, ici, et
/// [`la_liste_des_sources_ne_rate_aucun_module`] vérifie qu'elle suit
/// les `mod` de `main.rs`.
const SOURCES_DE_L_INTERFACE: &[(&str, &str)] = &[
    ("main.rs", include_str!("main.rs")),
    ("calc.rs", include_str!("calc.rs")),
    ("calc_ui.rs", include_str!("calc_ui.rs")),
    ("chrome.rs", include_str!("chrome.rs")),
    ("console.rs", include_str!("console.rs")),
    ("completion.rs", include_str!("completion.rs")),
    ("condition.rs", include_str!("condition.rs")),
    ("editor.rs", include_str!("editor.rs")),
    ("editing.rs", include_str!("editing.rs")),
    ("errors.rs", include_str!("errors.rs")),
    ("exercises.rs", include_str!("exercises.rs")),
    ("fixes.rs", include_str!("fixes.rs")),
    ("files.rs", include_str!("files.rs")),
    ("flow.rs", include_str!("flow.rs")),
    ("help.rs", include_str!("help.rs")),
    ("i18n.rs", include_str!("i18n.rs")),
    ("keybindings.rs", include_str!("keybindings.rs")),
    ("jobs.rs", include_str!("jobs.rs")),
    ("loader.rs", include_str!("loader.rs")),
    ("maximize.rs", include_str!("maximize.rs")),
    ("marks.rs", include_str!("marks.rs")),
    ("memfind.rs", include_str!("memfind.rs")),
    ("menus.rs", include_str!("menus.rs")),
    ("nasm_errors.rs", include_str!("nasm_errors.rs")),
    ("notes.rs", include_str!("notes.rs")),
    ("occurrences.rs", include_str!("occurrences.rs")),
    ("preferences.rs", include_str!("preferences.rs")),
    ("readings.rs", include_str!("readings.rs")),
    ("roles.rs", include_str!("roles.rs")),
    ("run.rs", include_str!("run.rs")),
    ("settings.rs", include_str!("settings.rs")),
    ("session.rs", include_str!("session.rs")),
    ("sidebar.rs", include_str!("sidebar.rs")),
    ("statusbar.rs", include_str!("statusbar.rs")),
    ("syntax.rs", include_str!("syntax.rs")),
    ("text.rs", include_str!("text.rs")),
    ("timeline.rs", include_str!("timeline.rs")),
    ("tools.rs", include_str!("tools.rs")),
    ("windows.rs", include_str!("windows.rs")),
];

/// Aucun commentaire de documentation n'a perdu sa fonction.
///
/// Rust laisse deux blocs de `///` se suivre : ils se concatènent sur
/// l'item d'après. Donc quand un item disparaît — renommé, supprimé,
/// déplacé sans son commentaire —, le sien reste, se colle au suivant,
/// et **plus rien ne le signale**. Ni le compilateur, ni clippy, ni les
/// cinq cent quatre-vingts contrôles de ce fichier.
///
/// Vingt-cinq blocs étaient dans ce cas, dont vingt-deux antérieurs au
/// découpage de `main.rs`. `module_card` était annoncé par le
/// commentaire de la barre de statut ; `flag_chip` n'en avait plus ;
/// `can_assemble` portait celui de `wait_for_input`. Un lecteur qui
/// ouvre ce dépôt pour apprendre lit donc la description d'autre chose
/// que ce qu'il a sous les yeux — c'est le contraire de ce qu'on a
/// voulu écrire.
///
/// La règle tient à la forme des blocs de ce dépôt : un résumé, une
/// ligne `///` vide, l'explication. Un bloc qui RECOMMENCE au milieu
/// d'un autre — une phrase finie en bout de ligne, puis une majuscule
/// qui ouvre la ligne suivante, puis un `///` vide juste après — est un
/// bloc collé, donc un commentaire qui a perdu sa fonction.
///
/// Le remède, quand ce contrôle rougit, n'est pas de couper le texte :
/// c'est de retrouver ce que le bloc documentait. `git log -S` sur sa
/// première ligne le dit en une commande.
#[test]
fn aucun_commentaire_n_a_perdu_sa_fonction() {
    // TOUT le dépôt, et non la seule interface : le premier cas trouvé
    // hors de ce crate était dans `cfg.rs`, où la description des
    // couches du graphe annonçait l'export Graphviz. Les fichiers sont
    // lus sur le disque parce qu'ils sont trop nombreux pour
    // `include_str!` — et c'est le dépôt qu'on lit, non la machine.
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut sources: Vec<std::path::PathBuf> = Vec::new();
    let mut à_voir = vec![racine];
    while let Some(dossier) = à_voir.pop() {
        let Ok(entrées) = std::fs::read_dir(&dossier) else {
            continue;
        };
        for entrée in entrées.flatten() {
            let chemin = entrée.path();
            if chemin.is_dir() {
                à_voir.push(chemin);
            } else if chemin.extension().is_some_and(|e| e == "rs")
                // `tests.rs` porte des récits en plusieurs paragraphes —
                // la forme y est trop libre pour que la règle tienne.
                && chemin.file_name().is_some_and(|n| n != "tests.rs")
            {
                sources.push(chemin);
            }
        }
    }
    sources.sort();
    assert!(
        sources.len() > 40,
        "le relevé ne trouve presque aucun fichier ({}) — il regarde au \
         mauvais endroit",
        sources.len()
    );

    let mut collés = Vec::new();
    for chemin in &sources {
        let fichier = chemin
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(chemin)
            .display()
            .to_string();
        let Ok(source) = std::fs::read_to_string(chemin) else {
            continue;
        };
        let lignes: Vec<&str> = source.lines().collect();
        if lignes.len() < 5 {
            continue;
        }
        for i in 1..lignes.len().saturating_sub(3) {
            let (avant, ici) = (lignes[i - 1].trim(), lignes[i].trim());
            let (Some(a), Some(b)) = (avant.strip_prefix("///"), ici.strip_prefix("///")) else {
                continue;
            };
            let (a, b) = (a.trim(), b.trim());
            // Une phrase finie en bout de ligne, une majuscule qui ouvre
            // la suivante. Une puce n'en est pas une : une énumération
            // se poursuit ligne à ligne sans séparation.
            if a.is_empty()
                || b.is_empty()
                || !a.ends_with('.')
                || !b.starts_with(char::is_uppercase)
                || b.starts_with("- ")
                || b.starts_with("* ")
            {
                continue;
            }
            // Et un nouveau bloc commence vraiment : son résumé est
            // suivi d'une ligne `///` vide, alors que le bloc d'avant
            // n'en avait pas.
            let suivantes = (1..4).map(|k| lignes[i + k].trim());
            let précédentes = (1..4).map(|k| lignes[i - 1 - k].trim());
            if suivantes.clone().any(|l| l == "///") && !précédentes.clone().any(|l| l == "///") {
                collés.push(format!("{fichier}:{}\n      …{a}\n      {b}", i + 1));
            }
        }
    }
    assert!(
        collés.is_empty(),
        "{} commentaire(s) collé(s) au suivant — leur fonction a disparu \
         entre les deux, cherchez-la avec `git log -S` :\n  {}",
        collés.len(),
        collés.join("\n  ")
    );
}

/// La liste ci-dessus n'oublie aucun module.
///
/// Sans ce contrôle, la liste serait exacte le jour où elle est écrite
/// et fausse au module suivant — et sa fausseté ferait passer les
/// autres contrôles, non échouer.
#[test]
fn la_liste_des_sources_ne_rate_aucun_module() {
    let déclarés: Vec<String> = include_str!("main.rs")
        .lines()
        .filter_map(|l| l.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')))
        .filter(|m| *m != "tests")
        .map(|m| format!("{m}.rs"))
        .collect();
    let oubliés: Vec<&String> = déclarés
        .iter()
        .filter(|m| !SOURCES_DE_L_INTERFACE.iter().any(|(nom, _)| nom == m))
        .collect();
    assert!(
        oubliés.is_empty(),
        "SOURCES_DE_L_INTERFACE ignore {} module(s) — les contrôles qui \
         lisent le code ne les verraient pas :\n  {:?}",
        oubliés.len(),
        oubliés
    );
}

/// La série livrée, lue dans le dépôt.
///
/// Elle n'est plus compilée dans le programme : c'est un fichier de
/// données. Les tests lisent donc celui **du dépôt** — jamais celui qui
/// est installé sur la machine, dont l'état ne prouverait rien.
fn builtin_pack() -> namsdec_exercises::Pack {
    let packs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../namsdec-exercises/packs");
    namsdec_exercises::builtin_pack_in(&packs).expect("la série livrée est lisible")
}

/// Une application qui connaît la série livrée.
///
/// `NamsdecApp::default()` n'en charge aucune — c'est le démarrage qui
/// lit les fichiers —, donc un test qui parle d'exercices la lui donne.
fn app_with_exercises() -> NamsdecApp {
    NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    }
}

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
        [egui::Key::Space, egui::Key::P, egui::Key::I],
        [egui::Key::Space, egui::Key::P, egui::Key::O],
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
        [egui::Key::Space, egui::Key::P, egui::Key::I],
        [egui::Key::Space, egui::Key::P, egui::Key::O],
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
    for key in [egui::Key::Space, egui::Key::P, egui::Key::I] {
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
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));

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
            egui::vec2(1280.0, 1400.0),
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

    let bytes = read_memory(
        &mem,
        BASE_ADDR,
        MemoryFormat::Bytes,
        "hors bornes",
        Lang::Fr,
    );
    assert!(bytes.contains("48 69"), "l'hexadécimal brut : {bytes}");

    let words = read_memory(
        &mem,
        BASE_ADDR,
        MemoryFormat::Words,
        "hors bornes",
        Lang::Fr,
    );
    let second = words.lines().nth(1).expect("un deuxième mot");
    assert!(
        second.ends_with("  1") && second.contains("0000000000000001"),
        "le mot suivant, en hexa et en décimal : {second}"
    );

    let text = read_memory(&mem, BASE_ADDR, MemoryFormat::Text, "hors bornes", Lang::Fr);
    let première = text.lines().next().expect("une ligne");
    assert!(
        première.contains("Hi") && !première.contains("48"),
        "le texte s'arrête au premier nul, sans hexadécimal : {text}"
    );
    assert!(
        text.contains("puis un zéro"),
        "et il dit où `write` s'arrêterait : {text}"
    );

    // Et les pointeurs : le mot de huit octets qui vaut 1 ne pointe
    // nulle part, mais celui qu'on pose ici mène à « Hi ».
    mem.map(
        BASE_ADDR..BASE_ADDR + 0x1000,
        namsdec_core::Perms::DATA,
        "test",
    );
    mem.write_u64(BASE_ADDR + 0x100, BASE_ADDR).unwrap();
    let chaîne = read_memory(
        &mem,
        BASE_ADDR + 0x100,
        MemoryFormat::Chain,
        "hors bornes",
        Lang::Fr,
    );
    let première = chaîne.lines().next().expect("une ligne");
    assert!(
        première.contains(&format!("{BASE_ADDR:016x}")) && première.contains("\"Hi\""),
        "l'adresse suivie jusqu'à son texte : {première}"
    );

    // Hors de la fenêtre simulée, les quatre le disent.
    for format in MemoryFormat::ALL {
        assert_eq!(
            read_memory(&mem, 0xdead_beef, format, "hors bornes", Lang::Fr),
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
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));

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
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));
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
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));
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
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));
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

    // La réponse d'une recherche de mise à jour ne survit pas à la
    // fermeture : on rouvre, on redemande.
    app.update_result = Some(Err("peu importe".to_owned()));
    app.show_about = false;
    let mut output = ctx.run_ui(input, |ui| app.draw(ui));
    output.textures_delta.clear();
    assert!(app.update_result.is_none(), "la réponse part avec la boîte");
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

/// Le bandeau d'état s'ouvre déployé à chaque lancement, et replié il
/// rend sa place au contenu de l'onglet.
///
/// egui retient l'état des en-têtes repliables d'une session à
/// l'autre : replié une fois, il revenait replié, registres cachés,
/// sans que rien ne dise pourquoi. La mesure porte sur ce que l'état
/// coûte : l'onglet, ancré sous lui, gagne toute la hauteur des
/// registres dès qu'on les replie.
#[test]
fn the_cpu_banner_opens_deployed() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    assert!(
        NamsdecApp::default().cpu_state_open,
        "déployé par défaut, sans rien avoir à régler"
    );

    let hauteur_de_l_onglet = |open: bool| {
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
                egui::vec2(1280.0, 1400.0),
            )),
            ..Default::default()
        };
        // Trois images : la première ne sait pas encore ce que le
        // bandeau mesure, la deuxième le partage, la troisième s'y
        // tient.
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }
        // Le champ Adresse doit rester visible : c'est la commande de
        // l'onglet, et un onglet dont on ne voit plus la commande ne
        // sert à rien.
        assert!(
            ctx.read_response(memory_address_id()).is_some(),
            "le champ Adresse est affiché, bandeau {}",
            if open { "déployé" } else { "replié" }
        );
        egui::containers::panel::PanelState::load(&ctx, egui::Id::new("sidebar_tab"))
            .expect("l'onglet a sa part du panneau")
            .outer_rect
            .height()
    };

    let déployé = hauteur_de_l_onglet(true);
    let replié = hauteur_de_l_onglet(false);
    // QUATRE-VINGT-DIX ET NON CENT, et le pourquoi mérite d'être écrit,
    // parce qu'abaisser un seuil pour faire passer un changement est
    // exactement la mauvaise manière.
    //
    // Le seuil valait 100 quand l'état du processeur portait l'en-tête
    // repliable d'egui PRÉCÉDÉ d'un filet à lui. Unifier les trois
    // en-têtes du bandeau a supprimé ce filet en double : le bandeau
    // DÉPLOYÉ a maigri de quinze pixels, que l'onglet a gagnés — 826 au
    // lieu de 811. Le repli rend donc 99 px au lieu de 114, non parce
    // qu'il rend moins, mais parce qu'il y avait moins à rendre.
    //
    // Ce que la règle protège est intact : replier doit valoir la peine.
    // Ce qu'elle mesurait par ricochet — la taille du bandeau déployé —
    // a bougé dans le bon sens, et c'est l'autre assertion, celle du
    // plancher de l'onglet, qui le dit.
    assert!(
        replié > déployé + 90.0,
        "replié, l'état du processeur rend sa place à l'onglet : {replié} contre {déployé}"
    );
    assert!(
        déployé >= SIDEBAR_TAB_MIN,
        "et déployé, l'onglet garde de quoi montrer quelque chose : {déployé}"
    );
}

/// L'onglet RE est absent tant qu'on ne l'a pas demandé — et une fois
/// demandé, il le reste d'un lancement à l'autre.
///
/// Il n'en allait pas ainsi : le « oui » ne durait que la session. La
/// raison tenait alors : le module était une ébauche, et la commande
/// « Affichage : RE » de la palette pouvait l'allumer sans qu'on l'ait
/// voulu, si bien qu'on le retrouvait ensuite sans savoir d'où il
/// venait. Ce n'est plus le cas — les modules sont là, et la case vit
/// dans les Préférences, visible et décochable. Refaire ce choix à
/// chaque lancement est exactement ce qu'un réglage sert à éviter.
///
/// Le champ a changé de nom au passage (`re_tab`, non `show_re_tab`) :
/// un fichier écrit du temps où le « oui » pouvait être involontaire ne
/// rallume donc rien.
#[test]
fn the_re_tab_is_absent_until_asked_for_then_remembered() {
    assert!(!NamsdecApp::default().show_re_tab, "application par défaut");

    // Un ancien fichier, écrit quand le champ s'appelait autrement, ne
    // rallume rien : ce « oui »-là pouvait avoir été donné sans le
    // vouloir.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(
        &path,
        r#"{"version": 2, "show_re_tab": true, "language": "fr"}"#,
    )
    .unwrap();
    let app = NamsdecApp::from_settings(Settings::load_from(&path));
    assert!(
        !app.show_re_tab,
        "l'ancien nom ne doit pas rallumer l'onglet"
    );

    // Et la rangée d'onglets ne le propose pas.
    let shown: Vec<&str> = SidebarTab::ALL
        .iter()
        .filter(|tab| app.show_re_tab || **tab != SidebarTab::Re)
        .map(|tab| tab.label(strings(Lang::Fr)))
        .collect();
    assert_eq!(shown, ["Pile", "Mémoire", "Tas", "Octets"]);

    // Mais ce qu'on coche dans Préférences, lui, part sur le disque et
    // revient.
    let app = NamsdecApp {
        show_re_tab: true,
        ..Default::default()
    };
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(
        écrit.contains("re_tab"),
        "le champ doit être écrit : {écrit}"
    );
    let relu: Settings = serde_json::from_str(&écrit).unwrap();
    assert!(
        NamsdecApp::from_settings(relu).show_re_tab,
        "et relu au lancement suivant"
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
                egui::vec2(1280.0, 1400.0),
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
    app.load_binary(output.bytes, "programme");

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
    app.load_binary(vec![0x90; MEM_SIZE + 1024], "programme"); // que des `nop`

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
        3,
        "et la console garde ses onglets : l'aide est une fenêtre à part"
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
            egui::vec2(1280.0, 1400.0),
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
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    // Trois images, et non deux : une fenêtre dont la hauteur épouse son
    // contenu ne connaît sa taille qu'après l'avoir disposé, et ne se
    // pose donc qu'à la troisième. Deux suffisaient tant que toutes les
    // fenêtres avaient une taille imposée ; ce n'est plus le cas.
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
    for exercise in builtin_pack().exercises {
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
                egui::vec2(1280.0, 1400.0),
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
            egui::vec2(1280.0, 1400.0),
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
            egui::vec2(1280.0, 1400.0),
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
    // Une fenêtre très haute : l'onglet montre d'abord la pile
    // elle-même, puis les adresses de retour, puis l'aide-mémoire de la
    // convention — c'est lui qu'on vérifie ici. Ce qui dépasse est
    // maintenant coupé par la zone de défilement de l'onglet, comme il
    // se doit : pour le lire, il faut donc la place de l'afficher.
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 2400.0),
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
            egui::vec2(1400.0, 1400.0),
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
            egui::vec2(1280.0, 1400.0),
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
                egui::vec2(1280.0, 1400.0),
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
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));
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
    // Le détail — fiche, opérandes résolus, arguments — est replié
    // devant une instruction ordinaire : ici, c'est lui qu'on regarde.
    app.next_detail_open = true;
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
    // Le détail — fiche, opérandes résolus, arguments — est replié
    // devant une instruction ordinaire : ici, c'est lui qu'on regarde.
    app.next_detail_open = true;
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
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    // Les mêmes noms se peignent ailleurs — la pile nomme le registre
    // qui pointe sur un mot —, donc on ne retient que ce qui est dans
    // l'état du processeur : entre son titre et celui de l'onglet, qui
    // vit dessous.
    let mut registres: Vec<egui::Pos2> = Vec::new();
    let mut onglet_y = f32::MAX;
    let mut barres: Vec<egui::Rect> = Vec::new();
    for _ in 0..4 {
        let mut trouvés: Vec<(&'static str, egui::Pos2)> = Vec::new();
        let mut état_y = f32::MAX;
        barres.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        // Deux passes : l'ordre de peinture ne dit pas l'ordre à
        // l'écran — le panneau de l'onglet est posé avant le bandeau
        // qu'il surmonte —, donc on relève d'abord les repères, on
        // décide ensuite.
        let mut titres_onglet: Vec<egui::Pos2> = Vec::new();
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => match text.galley.text() {
                    "RAX" => trouvés.push(("RAX", text.pos)),
                    "R15" => trouvés.push(("R15", text.pos)),
                    "RIP" => trouvés.push(("RIP", text.pos)),
                    t if t == strings(Lang::Fr).cpu_state => état_y = état_y.min(text.pos.y),
                    // Le titre de l'onglet dans le panneau, et non celui
                    // de la barre d'outils, qui est tout en haut.
                    t if t == strings(Lang::Fr).tab_stack
                        && text.pos.x > 1280.0 - SIDEBAR_WIDTH =>
                    {
                        titres_onglet.push(text.pos)
                    }
                    _ => {}
                },
                egui::epaint::Shape::Rect(r) if r.rect.width() < 20.0 && r.rect.height() > 40.0 => {
                    barres.push(r.rect);
                }
                _ => {}
            }
        }
        onglet_y = titres_onglet
            .iter()
            .map(|pos| pos.y)
            .filter(|y| *y > état_y)
            .fold(f32::MAX, f32::min);
        registres = trouvés
            .into_iter()
            .filter(|(_, pos)| pos.y > état_y && pos.y < onglet_y)
            .map(|(_, pos)| pos)
            .collect();
        output.textures_delta.clear();
    }
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
            // Assez haut pour les tenir toutes, quel qu'en soit le
            // nombre : une hauteur en dur devenait fausse dès qu'on
            // ajoutait une commande, et le test tombait alors sans rien
            // révéler. Ce qu'on vérifie n'est pas qu'elles tiennent dans
            // 1400 px, c'est qu'elles ne défilent pas quand il y a la
            // place.
            egui::vec2(
                1280.0,
                palette_commands(strings(Lang::Fr), true, true).len() as f32 * 40.0 + 200.0,
            ),
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
                egui::vec2(1280.0, 1400.0),
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
    // Le détail — fiche, opérandes résolus, arguments — est replié
    // devant une instruction ordinaire : ici, c'est lui qu'on regarde.
    app.next_detail_open = true;
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();

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
        // Le module choisi dans la colonne de gauche : c'est ainsi
        // qu'on lit celui-ci, et c'est ce qui lui donne toute la place.
        // « Tout » les empile, et ce qui dépasse de la fenêtre n'est
        // pas peint — un test qui lit ce qui est peint ne le verrait
        // pas.
        analysis_module: "protections".to_owned(),
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
        analysis_module: "protections".to_owned(),
        ..Default::default()
    };
    // Trop gros pour la mémoire simulée — il ne s'exécutera pas ici,
    // mais il doit s'*analyser*.
    app.load_binary(bytes, "programme");

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

    let exercices = builtin_pack().exercises.len();
    assert_eq!(
        exercices, 96,
        "le parcours livré compte quatre-vingt-seize exercices"
    );
    assert!(
        readme.contains("parcours de quatre-vingt-seize exercices"),
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
        "namsdec-update",
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
    for simulé in [
        0u64, 1, 12, 60, 39, 102, 107, 24, 35, 318, 9, 10, 158, 5, 302,
    ] {
        assert!(
            namsdec_re::table::is_simulated(simulé),
            "l'appel {simulé} est annoncé comme simulé"
        );
    }
    assert!(
        !namsdec_re::table::is_simulated(2),
        "ouvrir un fichier n'est pas simulé, et le README le dit"
    );
    for nom in ["getpid", "getrandom", "sched_yield", "arch_prctl", "mmap"] {
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
    assert!(
        app.load_elf(&out.file, "programme"),
        "l'ELF doit se charger"
    );

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
    // Les octets sont passés sous le repli « Détail » : ils découpent
    // l'instruction par champs, ce qu'on regarde en s'arrêtant, non à
    // chaque pas — le bandeau n'avait plus la place de les montrer sans
    // couper une ligne en deux sur une fenêtre par défaut. Le repli
    // s'ouvre de lui-même devant un appel ou un saut ; devant un `mov`,
    // c'est à nous de l'ouvrir.
    app.next_detail_open = true;

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
    app.load_binary(bytes, "programme");
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
            egui::vec2(1280.0, 1400.0),
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
            egui::vec2(1280.0, 1400.0),
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
    assert!(
        painted.contains("quatre-vingt-seize exercices"),
        "{painted}"
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
    // vingt-quatrième des quatre-vingt-seize étapes.
    assert!(painted.contains("30/96"), "{painted}");
}

/// Un parcours se suit : l'étape suivante est à un clic, sans avoir à
/// retrouver son rang dans une liste de quatre-vingt-seize.
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
    assert!(app.exercise_after("namsdec/echo").is_none());
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
        painted.contains("1/96"),
        "le rang de l'étape en cours : {painted}"
    );
}

/// Le manuel se consulte aussi pour ce qu'on ne connaît pas encore.
///
/// F1 répond sur le mot où l'on est : encore faut-il savoir quoi
/// écrire. La fenêtre d'aide cherche maintenant dans les 152 fiches, par
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
            // Le lint parle la langue de l'application : en français par
            // défaut, ce message dit « sortir par la fin ».
            .any(|d| d.message.contains("sortir par la fin")
                || d.message.contains("run off the end")),
        "{sans_fin:?}"
    );

    // Les programmes livrés avec l'application, eux, ne doivent
    // déclencher aucun avertissement : ce sont les modèles.
    let mut modèles = vec![("accueil", DEFAULT_SOURCE), ("nouveau", NEW_PROGRAM)];
    let pack = builtin_pack();
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
    app.load_binary(bytes, "programme");
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
                exercise_packs: vec![builtin_pack()],
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
                    egui::vec2(1280.0, 1400.0),
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
            egui::vec2(1280.0, 1400.0),
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

    app.load_binary(bytes, "programme");
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
    app.load_binary(bytes, "programme");

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
    let trouvées = return_addresses(emulator.cpu(), &app.notes, &symbols, 32);
    assert_eq!(trouvées.len(), 2, "{trouvées:?}");
    assert!(trouvées[0].2.starts_with("une"), "{trouvées:?}");
    assert!(trouvées[1].2.starts_with("_start"), "{trouvées:?}");
    assert!(trouvées[0].0 < trouvées[1].0, "de RSP vers le haut");

    // Écrasée par un débordement, elle disparaît de la liste : c'est
    // exactement ce qu'on vient y lire.
    app.poke_memory(trouvées[0].0, b"aaaabbbb");
    let emulator = app.emulator.as_ref().expect("chargé");
    let après = return_addresses(emulator.cpu(), &app.notes, &symbols, 32);
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

/// Une préversion le dit dans sa boîte « À propos », et le bandeau
/// disparaît de lui-même le jour où elle n'en est plus une.
///
/// Un avertissement qu'il faut penser à retirer à la main finit toujours
/// par mentir : celui-ci est lu dans le numéro de version.
#[test]
fn a_pre_release_says_so_and_a_release_says_nothing() {
    // Ce que SemVer appelle une préversion : ce qui suit le tiret.
    assert_eq!(prerelease("0.2.1-beta.2"), Some("beta.2"));
    assert_eq!(prerelease("1.0.0-rc.1"), Some("rc.1"));
    assert_eq!(prerelease("0.2.1-alpha"), Some("alpha"));
    // Une version définitive n'a rien à annoncer, métadonnée de
    // compilation comprise.
    assert_eq!(prerelease("0.2.1"), None);
    assert_eq!(prerelease("1.0.0+20260917"), None);

    // Et la version d'aujourd'hui, telle qu'elle est : le bandeau est
    // là, avec ce qu'il faut dire.
    let mut app = NamsdecApp {
        show_about: true,
        ..Default::default()
    };
    let painted = paint(&mut app);
    let s = strings(app.lang);
    if let Some(étiquette) = prerelease(APP_VERSION) {
        assert!(
            painted.contains(&format!("{} — {étiquette}", s.about_beta_title)),
            "{painted}"
        );
        // Le texte dit ce qu'il faut savoir : que les réglages peuvent
        // bouger, et que les erreurs se disent.
        assert!(s.about_beta_body.contains("réglages"));
        assert!(strings(Lang::En).about_beta_body.contains("settings"));
    } else {
        assert!(
            !painted.contains(s.about_beta_title),
            "version définitive : aucun bandeau à afficher"
        );
    }
}

/// Le journal des versions parle bien de la version qu'on compile.
///
/// Un journal en retard d'une version est pire que pas de journal : il
/// dit que rien n'a changé alors que tout a bougé.
#[test]
fn the_changelog_covers_the_version_being_built() {
    let changelog = include_str!("../../../CHANGELOG.md");
    assert!(
        changelog.contains(&format!("## {APP_VERSION} ")),
        "le journal ne mentionne pas {APP_VERSION}"
    );
    // La version courante vient en tête : c'est là qu'on la cherche.
    let première = changelog
        .lines()
        .find(|ligne| ligne.starts_with("## "))
        .unwrap_or_default();
    assert!(
        première.contains(APP_VERSION),
        "la version en tête du journal est « {première} »"
    );
    // Et le README y renvoie.
    let readme = include_str!("../../../README.md");
    assert!(readme.contains("CHANGELOG.md"));
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
    const DÉMONSTRATIONS: [&str; 86] = [
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
    ];
    // Ce qu'il reste à faire : le but n'est pas atteint tant qu'on n'a
    // pas écrit ce qui manque.
    const À_FAIRE: [&str; 3] = ["pile", "division", "convention"];

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

/// La recherche de mise à jour est branchée : elle demande, elle répond,
/// et elle n'installe rien.
///
/// Le réseau n'est pas touché ici — ce que GitHub répond est vérifié
/// chez `namsdec-update`. Ce qui se vérifie ici, c'est le chemin :
/// l'état d'attente, le verdict affiché, et la promesse tenue de ne
/// jamais se remplacer tout seul.
#[test]
fn the_update_check_is_wired_and_installs_nothing() {
    let mut app = NamsdecApp {
        show_about: true,
        ..Default::default()
    };
    // Rien n'a été demandé tant que personne n'a cliqué.
    assert!(app.update_check.is_none());
    assert!(app.update_result.is_none());

    let painted = paint(&mut app);
    let s = strings(app.lang);
    assert!(painted.contains(s.about_check_update), "{painted}");

    // Le verdict, tel qu'il s'affiche : plus récent, à jour, devant.
    let release = |v: &str| namsdec_update::Release {
        tag: format!("v{v}"),
        version: v.to_owned(),
        name: v.to_owned(),
        url: format!("https://github.com/fredza/namsdec/releases/tag/v{v}"),
        prerelease: v.contains('-'),
    };
    app.update_result = Some(Ok(namsdec_update::verdict(APP_VERSION, release("9.9.9"))));
    let painted = paint(&mut app);
    assert!(painted.contains(s.about_update_newer), "{painted}");
    assert!(painted.contains("v9.9.9"), "{painted}");
    // Et la façon de mettre à jour est dite : à la main.
    assert!(painted.contains(s.about_update_how), "{painted}");
    assert!(s.about_update_how.contains("install.sh"));

    app.update_result = Some(Ok(namsdec_update::verdict(
        APP_VERSION,
        release(APP_VERSION),
    )));
    let painted = paint(&mut app);
    assert!(painted.contains(s.about_update_uptodate), "{painted}");

    // Un dépôt qui n'a rien publié n'est pas une panne, et le message
    // le dit autrement qu'un échec.
    app.update_result = Some(Err(s.about_update_none.to_owned()));
    let painted = paint(&mut app);
    assert!(painted.contains(s.about_update_none), "{painted}");

    // Refermer la boîte oublie la réponse : l'état du dépôt a pu changer
    // entre deux ouvertures.
    app.show_about = false;
    paint(&mut app);
    assert!(app.update_result.is_none());
}

/// Ce que la recherche de mise à jour va trouver doit exister : le
/// dépôt publie ses versions, et la façon de les publier est décrite
/// dans le dépôt lui-même.
///
/// Un bouton qui interroge une API que rien n'alimente ne dit jamais
/// rien d'autre que « aucune version publiée ».
#[test]
fn the_repository_knows_how_to_publish_what_the_check_looks_for() {
    let workflow = include_str!("../../../.github/workflows/release.yml");
    // Publié sur étiquette : c'est ce que « dès qu'un build est
    // publié » veut dire.
    assert!(workflow.contains("tags:"), "{workflow}");
    assert!(workflow.contains("softprops/action-gh-release"));
    // Et pas n'importe quoi : les tests passent avant la publication.
    assert!(workflow.contains("cargo test --workspace"));
    assert!(workflow.contains("cargo clippy"));
    // Une somme de contrôle accompagne le binaire.
    assert!(workflow.contains("sha256sum"));
    // Une étiquette à tiret est publiée comme préversion, comme le dit
    // SemVer et comme le programme le lit.
    assert!(workflow.contains("prerelease:"));

    // Le dépôt interrogé est celui qui publie.
    assert_eq!(namsdec_update::OWNER, "fredza");
    assert_eq!(namsdec_update::REPO, "namsdec");
}

/// Le dépôt sait aussi se vérifier à chaque poussée, pas seulement au
/// moment de publier.
///
/// Publier ce qui n'a pas été vérifié transformerait la vérification en
/// formalité ; vérifier seulement au moment de publier laisserait `main`
/// casser sans que personne le sache.
#[test]
fn the_repository_checks_itself_on_every_push() {
    let ci = include_str!("../../../.github/workflows/ci.yml");
    assert!(ci.contains("branches: [main]"), "{ci}");
    assert!(ci.contains("pull_request"), "{ci}");
    for étape in [
        "cargo fmt --all --check",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test --workspace",
    ] {
        assert!(ci.contains(étape), "le CI ne fait pas « {étape} »");
    }
    // nasm et ld sont installés : sans eux, la moitié des tests se
    // taisent au lieu de vérifier quoi que ce soit.
    assert!(ci.contains("nasm binutils"), "{ci}");
}

/// Les chaînes se lisent aux adresses que le code désigne : c'est ce
/// qui permet de dire « lue ici », et c'est tout l'intérêt.
///
/// Sur un programme assemblé ici, l'adresse trouvée doit être celle de
/// l'étiquette. Sur un gros binaire lu par morceaux, les chaînes de
/// `.rodata` tombent au-delà de ce que namsdec charge : elles se lisent
/// quand même, mais aucune référence ne peut les croiser, et il vaut
/// mieux le savoir que le découvrir.
#[test]
fn the_strings_of_a_program_land_on_their_label() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
default rel
global _start

section .data
message: db \"un message assez long\", 0

section .text
_start:
    lea rax, [message]
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let state = app.assemble_state.as_ref().expect("assemblé");
    let attendue = state
        .output
        .symbols
        .iter()
        .find(|s| s.name == "message")
        .expect("l'étiquette existe")
        .addr;
    let trouvées = namsdec_re::strings::scan_elf(&state.output.file).expect("un ELF");
    let chaîne = trouvées
        .iter()
        .find(|found| found.text.contains("un message assez long"))
        .unwrap_or_else(|| panic!("la chaîne n'est pas trouvée : {trouvées:?}"));
    assert_eq!(
        chaîne.addr, attendue,
        "l'adresse trouvée est celle de l'étiquette"
    );

    // Et le code la désigne : c'est le « lue ici » du panneau.
    // La plage est celle du programme entier : c'est elle qui dit à
    // `xrefs` ce qui est une adresse et ce qui est un nombre.
    let premier = state.disasm.first().expect("du code").address;
    let dernier = state.disasm.last().expect("du code").address;
    let xrefs = namsdec_re::xrefs::scan(&state.disasm, premier..dernier + 0x2000);
    println!(
        "XREF attendue={attendue:#x} ; cibles = {:?}",
        xrefs.labels()
    );
    for i in &state.disasm {
        println!("  {:#x} {}", i.address, i.text);
    }
    assert!(
        !xrefs.to(attendue).is_empty(),
        "personne ne pointe sur la chaîne"
    );
}

/// Ouvrir un binaire montre son code, et non son en-tête décodé comme
/// des instructions.
///
/// Les premières lignes d'un ELF ouvert étaient du charabia : le
/// désassemblage partait de l'octet zéro, donc de `\x7fELF`, des tables
/// de relocation et des chaînes de symboles. C'est pourtant la première
/// chose qu'on voit en ouvrant un fichier.
#[test]
fn opening_a_binary_shows_its_code_not_its_header() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes, "programme");
    let state = app.assemble_state.as_ref().expect("chargé");

    // Le listing commence à une section exécutable, et non à l'octet
    // zéro du fichier — qui porte « \x7fELF », pas des instructions.
    let elf_entier = namsdec_re::elf_scan::scan(&state.output.file).expect("un ELF");
    let exécutables: Vec<&namsdec_re::elf_scan::SectionInfo> = elf_entier
        .sections
        .iter()
        .filter(|s| s.execute && !s.nobits && s.size > 0)
        .collect();
    let première = state.disasm.first().expect("du code").address;
    assert!(
        exécutables
            .iter()
            .any(|s| première >= s.addr && première < s.addr + s.size),
        "le listing ne commence pas dans du code : {première:#x}"
    );

    // Et tout ce qu'il montre tombe dans une section exécutable du
    // fichier — c'est la définition de « c'est du code ».
    // Les adresses sont celles que le fichier déclare, désormais : la
    // fenêtre mémoire s'adapte au programme au lieu de le tasser à
    // `BASE_ADDR`, ce qui décalait tout.
    let plages: Vec<(u64, u64)> = exécutables
        .iter()
        .map(|s| (s.addr, s.addr + s.size))
        .collect();
    for instruction in &state.disasm {
        assert!(
            plages
                .iter()
                .any(|(début, fin)| (*début..*fin).contains(&instruction.address)),
            "{:#x} n'est dans aucune section exécutable",
            instruction.address
        );
    }

    // Les entrées de liaison y sont toujours nommées.
    assert!(app.source.contains("malloc@plt:"), "les étiquettes restent");
}

/// Les deux langues disent les mêmes choses : mêmes champs, aucun vide,
/// et rien qui soit resté en français dans la version anglaise.
///
/// C'est l'erreur qu'on fait en ajoutant un message : le compilateur
/// exige les deux champs, mais rien n'empêche d'y écrire deux fois la
/// même phrase. Les mots qui sont *les mêmes* dans les deux langues —
/// « Step Into », « OK », un nom d'onglet technique — sont listés ici
/// un par un, pour qu'un oubli ne puisse pas s'y cacher.
#[test]
fn both_languages_say_the_same_things() {
    /// Les champs dont les deux langues partagent le texte, et pourquoi.
    const LES_MÊMES: [&str; 27] = [
        // « Pseudo-code » s'écrit pareil des deux côtés de la Manche.
        "flow_pseudo",
        // Une adresse d'exemple : elle s'écrit pareil partout.
        "flow_goal_hint",
        // Une adresse et deux noms propres : les traduire n'aurait pas
        // de sens.
        "about_site_hint",
        "about_discord",
        "about_github",
        // Les commandes que tout débogueur écrit ainsi, y compris en
        // français : les traduire couperait du reste du monde.
        "action_step_into",
        "action_step_over",
        "action_step_back",
        "action_stop",
        // Un mot identique dans les deux langues.
        "abi_arguments",
        "about_build_word",
        "dialog_ok",
        "bytes_instructions",
        "disassembly_nasm",
        "disassembly_pseudo",
        "filter_markdown",
        "preferences_tab_modules",
        "preferences_tab_plugins",
        "tab_diagnostics",
        "tab_output",
        "tab_re",
        "view_console",
        "x87_empty",
        // Le mot est le même des deux côtés de la Manche.
        "calculator_base_label",
        "reading_octal",
        "reading_ascii",
        // Une expression de calcul : `0xff & 0x0f` ne se traduit pas.
        "calculator_hint_text",
    ];

    let source = include_str!("i18n.rs");
    let bloc = |nom: &str| {
        let début = source
            .find(&format!("const {nom}: Strings = Strings {{"))
            .unwrap_or_else(|| panic!("le bloc {nom} existe"));
        let fin = source[début..].find("\n};").expect("le bloc se ferme") + début;
        &source[début..fin]
    };
    let champs = |bloc: &str| -> Vec<(String, String)> {
        let mut out = Vec::new();
        for ligne in bloc.lines() {
            let taillée = ligne.trim();
            // Une ligne de champ : `nom: "valeur",`. Les commentaires et
            // les suites de `concat!` n'en sont pas.
            let Some((nom, reste)) = taillée.split_once(": ") else {
                continue;
            };
            if nom.starts_with("//") || !nom.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                continue;
            }
            out.push((nom.to_owned(), reste.to_owned()));
        }
        out
    };

    let fr = champs(bloc("FR"));
    let en = champs(bloc("EN"));
    assert_eq!(
        fr.len(),
        en.len(),
        "les deux langues n'ont pas le même nombre de messages"
    );
    for ((nom_fr, texte_fr), (nom_en, texte_en)) in fr.iter().zip(&en) {
        assert_eq!(nom_fr, nom_en, "les champs ne sont pas dans le même ordre");
        assert!(texte_fr.len() > 2, "{nom_fr} : rien en français");
        assert!(texte_en.len() > 2, "{nom_en} : rien en anglais");
        if texte_fr == texte_en {
            assert!(
                LES_MÊMES.contains(&nom_fr.as_str()),
                "{nom_fr} dit la même chose dans les deux langues : traduction oubliée ?"
            );
        }
    }
    // Et la liste ne garde pas d'entrées mortes : un champ traduit
    // depuis n'a plus rien à y faire.
    for nom in LES_MÊMES {
        let (_, texte_fr) = fr.iter().find(|(n, _)| n == nom).expect(nom);
        let (_, texte_en) = en.iter().find(|(n, _)| n == nom).expect(nom);
        assert_eq!(
            texte_fr, texte_en,
            "{nom} est traduit : à retirer de la liste"
        );
    }
}

/// Ce qu'on lit dans les binaires de cette machine s'explique, et pas
/// seulement ce qu'on lit dans `/bin/ls`.
///
/// Un manuel vérifié sur un seul fichier documente ce fichier. Celui-ci
/// passe sur une douzaine de binaires du système et exige une fiche pour
/// tout mnémonique qui revient — le seuil laisse passer l'exotique isolé,
/// parce qu'écrire une fiche pour une instruction vue une fois dans une
/// bibliothèque de compression n'apprend à personne.
///
/// IL LIT LA MACHINE, ET C'EST ASSUMÉ. Un test qui lit la machine ment
/// d'ordinaire : il passe ou échoue selon ce qui a été installé la
/// veille. Celui-ci est l'exception, parce que ce qu'il trouve n'est
/// jamais un faux positif — une instruction qu'un binaire réel contient
/// et que le manuel ignore est un manque du manuel, où qu'on soit.
///
/// La conséquence est qu'il peut rougir sur une machine où il n'a jamais
/// tourné. C'est arrivé : le coureur d'intégration continue, sur une
/// autre distribution et un autre processeur, a trouvé seize mnémoniques
/// que la machine de développement ne contenait nulle part —
/// multiplications parallèles, additions saturantes, préchargement de
/// cache, appels à l'hyperviseur. Neuf fiches ont été écrites, et le
/// manuel est meilleur qu'avant.
///
/// Donc : un échec ici n'est pas un défaut de l'application. C'est une
/// liste de fiches à écrire.
#[test]
fn what_this_machine_runs_can_be_explained() {
    let Ok(dir) = std::fs::read_dir("/usr/bin") else {
        eprintln!("/usr/bin illisible : test ignoré");
        return;
    };
    let mut compte: std::collections::BTreeMap<String, usize> = Default::default();
    let mut lus = 0usize;
    for entry in dir.flatten() {
        if lus >= 60 {
            break;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() || meta.len() > 2 << 20 {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path()) else {
            continue;
        };
        if !bytes.starts_with(b"\x7fELF") {
            continue;
        }
        let Some(elf) = namsdec_re::elf_scan::scan(&bytes) else {
            continue;
        };
        lus += 1;
        for section in elf.sections.iter().filter(|s| s.execute && !s.nobits) {
            let début = section.offset as usize;
            let fin = (début + section.size as usize).min(bytes.len());
            if fin <= début {
                continue;
            }
            for instruction in namsdec_disasm::disassemble(&bytes[début..fin], section.addr) {
                let mnemonic = instruction
                    .text
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if mnemonic.is_empty() || mnemonic == "(bad)" {
                    continue;
                }
                *compte.entry(mnemonic).or_default() += 1;
            }
        }
    }
    assert!(lus > 0, "aucun binaire lu : le test n'a rien prouvé");

    // Une seule fois suffit désormais : sur soixante binaires du
    // système, il n'y a plus une instruction que le manuel ne sache
    // expliquer. L'échantillon a quintuplé et le seuil est tombé à un —
    // c'est l'exigence la plus forte que ce test puisse porter.
    let manquants: Vec<(&String, &usize)> = compte
        .iter()
        .filter(|(mnemonic, _)| namsdec_doc::lookup(mnemonic).is_none())
        .collect();
    assert!(
        manquants.is_empty(),
        "ces instructions reviennent dans les binaires de cette machine sans pouvoir \
         s'expliquer — ce sont autant de fiches à écrire dans \
         `namsdec-doc`, non un défaut de l'application ({lus} binaires \
         lus) : {manquants:?}"
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
    let mut app = NamsdecApp::default();
    let vide = mesure(&mut app);
    assert!(
        vide <= CONSOLE_MIN_HEIGHT + 1.0,
        "une console vide réclame {vide} pixels"
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

/// Les onglets du panneau latéral se choisissent dans la barre d'outils,
/// à droite, et non plus au fond du panneau.
///
/// Ils vivaient entre l'état du processeur et leur propre contenu :
/// selon que le bandeau était déplié ou non, la rangée montait ou
/// descendait, et l'on visait une cible qui bouge.
#[test]
fn the_sidebar_tabs_live_in_the_toolbar() {
    let mut app = NamsdecApp {
        show_toolbar: true,
        show_sidebar: true,
        show_re_tab: false,
        ..Default::default()
    };
    let s = strings(app.lang);
    // Où chaque étiquette est peinte, et non dans quel ordre : ce qui
    // compte est ce qu'on lit de gauche à droite.
    let abscisse = |app: &mut NamsdecApp, texte: &str| -> Option<f32> {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 1400.0),
            )),
            ..Default::default()
        };
        let mut trouvée = None;
        for _ in 0..2 {
            trouvée = None;
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(forme) = &clipped.shape
                    && forme.galley.text() == texte
                {
                    trouvée = Some(forme.pos.x);
                }
            }
            output.textures_delta.clear();
        }
        trouvée
    };
    let mut x = Vec::new();
    for label in [s.tab_stack, s.tab_memory, s.tab_heap, s.tab_bytes] {
        x.push(
            abscisse(&mut app, label)
                .unwrap_or_else(|| panic!("l'onglet « {label} » n'est pas peint")),
        );
    }
    assert!(
        x.windows(2).all(|paire| paire[0] < paire[1]),
        "de gauche à droite : Pile, Mémoire, Tas, Octets — obtenu {x:?}"
    );
    // Et à droite de la barre, pas à gauche : au-delà de sa moitié.
    assert!(x[0] > 1280.0 / 2.0, "la rangée est à droite : {x:?}");
    let painted = paint(&mut app);
    // L'onglet RE ne s'affiche que s'il est demandé.
    assert!(!painted.contains(s.tab_re), "{painted}");
    app.show_re_tab = true;
    assert!(paint(&mut app).contains(s.tab_re));

    // Sans panneau latéral, pas d'onglets : ils commandent ce qu'il
    // montre, et il ne montre rien.
    app.show_sidebar = false;
    let painted = paint(&mut app);
    assert!(!painted.contains(s.tab_heap), "{painted}");
}

/// Réinitialiser se prend dans la barre d'outils.
///
/// Le programme terminé, Exécuter, Step Into et Step Over s'éteignent —
/// et c'est justement là qu'on veut repartir de `_start`. L'action
/// existait, mais ses trois chemins — le menu Outils, la palette,
/// Ctrl+Maj+F5 — ont ceci de commun qu'aucun ne se voit : rien dans la
/// barre ne disait qu'on pouvait recommencer.
#[test]
fn the_toolbar_offers_reset() {
    let mut app = NamsdecApp {
        show_toolbar: true,
        ..Default::default()
    };
    let s = strings(app.lang);

    let painted = paint(&mut app);
    assert!(
        painted.contains(s.action_reset),
        "« {} » manque dans la barre d'outils : {painted}",
        s.action_reset
    );

    // Et c'est bien la barre qui le porte, non un menu resté ouvert :
    // sans elle, le libellé disparaît.
    app.show_toolbar = false;
    let painted = paint(&mut app);
    assert!(
        !painted.contains(s.action_reset),
        "sans barre d'outils, le bouton ne doit venir de nulle part ailleurs : {painted}"
    );
}

/// Réinitialiser remet tout ce qui décrivait la course précédente.
///
/// Le processeur repartait bien du point d'entrée, mais le reste de
/// l'écran ne bougeait pas : les diagnostics du binaire restaient
/// affichés avec leur compte, l'onglet restait ouvert dessus, et le
/// listing ne quittait pas la ligne où l'exécution s'était arrêtée. Le
/// bouton avait l'air de n'avoir rien fait.
#[test]
fn reset_clears_what_the_previous_run_left_behind() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "ls"), "l'ELF se charge");

    // Une séance ordinaire : on avance, puis on frappe Assembler par
    // habitude — ce que namsdec refuse sur un listing de binaire.
    for _ in 0..50 {
        app.dispatch(Action::StepInto);
    }
    app.dispatch(Action::Assemble);
    app.console_tab = ConsoleTab::Diagnostics;
    app.follow_rip = false;
    let entree = namsdec_asm::elf::parse(&octets).expect("un ELF").entry;
    assert!(
        matches!(app.status, Status::Error(_)),
        "le refus de réassembler est bien affiché"
    );
    assert_ne!(
        app.emulator.as_ref().expect("un émulateur").cpu().regs.rip,
        entree,
        "l'exécution a quitté le point d'entrée"
    );

    app.dispatch(Action::Reset);

    assert_eq!(
        app.emulator.as_ref().expect("un émulateur").cpu().regs.rip,
        entree,
        "le programme repart de son point d'entrée"
    );
    assert!(
        matches!(app.status, Status::Idle),
        "obtenu {:?}",
        app.status
    );
    assert!(
        app.diagnostics.is_empty(),
        "les diagnostics du binaire s'en vont comme au chargement : {:?}",
        app.diagnostics
    );
    assert_eq!(
        app.console_tab,
        ConsoleTab::Output,
        "l'onglet ne reste pas ouvert sur des diagnostics qui n'existent plus"
    );
    assert!(app.follow_rip, "et le listing retourne au point d'entrée");
    assert_eq!(app.run_steps_taken, 0);
}

/// Assembler se grise sur un listing de binaire, et revient dès qu'on
/// touche au texte.
///
/// Le refus existait déjà, mais il arrivait après le clic : on frappait
/// Assembler, on récoltait une erreur rouge, et rien ne l'avait annoncé.
/// Un bouton qui échouera à tous les coups doit se voir avant, comme
/// Exécuter et Step Into se grisent quand plus rien ne tourne.
#[test]
fn assembling_is_greyed_out_on_a_binary_listing() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(
        app.can_assemble(),
        "sur un source ordinaire, rien n'empêche d'assembler"
    );

    assert!(app.load_elf(&octets, "ls"), "l'ELF se charge");
    assert!(
        !app.can_assemble(),
        "le listing d'un binaire ouvert n'est pas un source"
    );

    // Réinitialiser ne rend pas un désassemblage assemblable : il remet
    // la machine à son point d'entrée, pas le texte à l'état de source.
    app.dispatch(Action::Reset);
    assert!(!app.can_assemble(), "Réinitialiser ne change rien à cela");

    // La porte de sortie est celle qu'annonce le message : modifier le
    // texte. La première frappe suffit.
    app.source.push_str("\n; à moi\n");
    assert!(
        app.can_assemble(),
        "dès que le texte est touché, c'est un source comme un autre"
    );

    // Et le raccourci, lui, continue de passer par le refus expliqué :
    // un geste qui ne vient pas d'un bouton visible mérite sa raison.
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "ls"));
    app.dispatch(Action::Assemble);
    assert!(
        matches!(&app.status, Status::Error(e) if e == strings(app.lang).binary_no_reassemble),
        "obtenu {:?}",
        app.status
    );
}

/// Échap reprend au champ d'entrée le focus qu'il avait pris.
///
/// Tant que le programme attend de quoi lire, le champ réclame le
/// curseur à chaque frame. C'est ce qu'on veut — mais cela confisquait
/// aussi les touches de l'éditeur, et « i » s'écrivait dans le champ au
/// lieu de passer en Insertion, sans autre issue que la souris.
#[test]
fn escape_gives_the_keyboard_back_to_the_editor() {
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
        "le programme attend son entrée : {:?}",
        app.status
    );

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let frame = |app: &mut NamsdecApp, ctx: &egui::Context, input: egui::RawInput| {
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    };

    // La boîte de saisie s'ouvre d'elle-même : c'est elle qui demande,
    // et c'est elle qui prend le clavier. On la referme d'abord — le
    // reste du test porte sur la ligne de la console, l'autre chemin.
    frame(&mut app, &ctx, input.clone());
    assert!(app.show_input_dialog, "la boîte s'ouvre quand on attend");
    let échap = |input: &egui::RawInput| {
        let mut avec = input.clone();
        avec.events.push(egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        avec
    };
    frame(&mut app, &ctx, échap(&input));
    assert!(!app.show_input_dialog, "Échap referme la boîte");

    frame(&mut app, &ctx, input.clone());
    let au_champ = ctx.memory(|m| m.focused());
    assert!(
        au_champ.is_some(),
        "la ligne de la console prend alors le curseur"
    );
    assert_ne!(au_champ, Some(editor_id()), "et ce n'est pas l'éditeur");

    // Échap de nouveau, et le clavier revient aux commandes.
    frame(&mut app, &ctx, échap(&input));
    assert_eq!(
        ctx.memory(|m| m.focused()),
        None,
        "plus aucun champ ne retient les touches"
    );
    assert!(
        app.console_input_released,
        "et le champ ne le reprendra pas"
    );
    assert_eq!(app.editor_mode, EditorMode::Normal);

    // Il ne le reprend pas à la frame suivante, bien que l'attente dure.
    frame(&mut app, &ctx, input.clone());
    assert!(matches!(app.status, Status::WaitingForInput));
    assert_eq!(
        ctx.memory(|m| m.focused()),
        None,
        "ce qu'on a refusé une fois ne revient pas de soi-même"
    );

    // Mais une nouvelle attente, elle, ramène le curseur : ce qu'on a
    // refusé une fois, on ne le refuse pas pour toujours.
    app.wait_for_input();
    assert!(!app.console_input_released);
}

/// En mode Normal, la position du curseur se voit.
///
/// Le `TextEdit` n'y est pas interactif — c'est ce qui laisse les
/// touches aux commandes —, donc egui ne peint aucun caret et ne rend
/// plus de `cursor_range`. La barre de statut restait figée sur la
/// première colonne et rien, dans le code, ne disait où l'insertion
/// reprendrait.
#[test]
fn normal_mode_still_shows_where_the_cursor_is() {
    let mut app = NamsdecApp {
        source: "mov rax, 60\nmov rdi, 0\nsyscall\n".to_owned(),
        editor_mode: EditorMode::Normal,
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
    let frame = |app: &mut NamsdecApp| {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        let rects = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Rect(forme) => Some(forme.rect),
                _ => None,
            })
            .collect::<Vec<_>>();
        output.textures_delta.clear();
        rects
    };

    frame(&mut app);
    // Le curseur est posé sur la deuxième ligne, comme le ferait un clic.
    let cible = "mov rax, 60\n".chars().count() + 4;
    if let Some(mut state) = egui::TextEdit::load_state(&ctx, editor_id()) {
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(cible),
            )));
        state.store(&ctx, editor_id());
    }
    let rects = frame(&mut app);

    assert_eq!(
        app.cursor_pos,
        (2, 5),
        "la barre de statut suit le curseur même sans focus"
    );
    // Un bloc de la largeur d'un caractère : assez fin pour n'être ni le
    // fond de l'éditeur, ni la ligne surlignée.
    let police = egui::TextStyle::Monospace.resolve(&ctx.style_of(egui::Theme::Dark));
    let largeur = ctx.fonts_mut(|f| f.glyph_width(&police, ' '));
    assert!(
        rects
            .iter()
            .any(|r| (r.width() - largeur).abs() < 0.5 && r.height() > 1.0),
        "un bloc large d'un caractère est peint quelque part"
    );
}

/// L'onglet Modules montre tous les modules, pas la moitié.
///
/// Il en affichait cinq — core, asm, disasm, emu, re — sur les dix que
/// namsdec porte. L'explorateur, la documentation, les exercices, les
/// thèmes et la recherche de mise à jour n'avaient aucune carte : rien
/// ne disait qu'ils existaient, ni dans quel état ils étaient.
///
/// Le test nomme les crates un par un plutôt que de compter : un module
/// ajouté au dépôt et oublié ici doit faire échouer quelque chose.
#[test]
fn the_modules_tab_shows_every_module_namsdec_carries() {
    let app = NamsdecApp::default();

    // La page seule, sur une hauteur qui la contient tout entière : dans
    // la fenêtre des préférences elle défile, et egui ne peint pas ce
    // qu'une zone défilante laisse hors de vue. Ce qu'on vérifie ici est
    // ce que la page écrit, non ce que la fenêtre en montre d'un coup.
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 4000.0),
        )),
        ..Default::default()
    };
    let mut peint = String::new();
    for _ in 0..2 {
        peint.clear();
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.modules_ui(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                peint.push_str(texte.galley.text());
                peint.push('|');
            }
        }
        sortie.textures_delta.clear();
    }

    for module in [
        "namsdec-core",
        "namsdec-asm",
        "namsdec-disasm",
        "namsdec-emu",
        "namsdec-re",
        "namsdec-explorer",
        "namsdec-doc",
        "namsdec-exercises",
        "namsdec-theme",
        "namsdec-update",
    ] {
        assert!(
            peint.contains(module),
            "« {module} » doit avoir sa carte dans l'onglet Modules :\n{peint}"
        );
    }

    // Les modules d'analyse sont nommés un par un, et non entassés dans
    // une ligne de titres séparés par des virgules.
    let s = strings(app.lang);
    assert!(peint.contains(s.modules_re_detail), "{peint}");
    for titre in app.re_registry.titles(app.lang.code()) {
        assert!(
            peint.contains(titre),
            "le module d'analyse « {titre} » doit se lire :\n{peint}"
        );
    }

    // Et la section de ce qui vient de fichiers, même vide : son absence
    // et « aucun greffon » ne se disent pas de la même façon.
    assert!(peint.contains(s.modules_plugins), "{peint}");
}

/// Une instruction trop longue est coupée, pas laissée à déborder.
///
/// Les boîtes avaient une largeur fixe et le texte n'était jamais
/// mesuré : « jne near 000000000001D903h » sortait de son cadre et se
/// posait sur la boîte voisine. Sur un binaire réel, le dessin devenait
/// illisible avant même d'être grand.
#[test]
fn a_long_instruction_is_cut_rather_than_spilled() {
    assert_eq!(flow_ellipsis("mov rax, rbx", 20), "mov rax, rbx");
    // Pile la largeur : rien à couper.
    assert_eq!(flow_ellipsis("mov rax, rbx", 12), "mov rax, rbx");
    // Un caractère de trop : on coupe, et l'on montre qu'on a coupé.
    let coupé = flow_ellipsis("jne near 000000000001D903h", 12);
    assert_eq!(coupé.chars().count(), 12, "la coupe tient dans la boîte");
    assert!(coupé.ends_with('…'), "et se voit : {coupé}");
    assert!(coupé.starts_with("jne near"), "{coupé}");
    // Les caractères accentués comptent pour un, pas pour leurs octets.
    assert_eq!(flow_ellipsis("éééééé", 3), "éé…");
}

/// La boîte s'élargit pour ce qu'elle porte, entre deux bornes.
///
/// Une largeur commune à tout le graphe : les colonnes restent alignées,
/// ce qui est la moitié de ce qu'un tel dessin sert à montrer.
#[test]
fn flow_boxes_take_the_width_their_text_needs() {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };

    let mesure = |cfg: &namsdec_re::cfg::Cfg| {
        let mut large = 0.0;
        for _ in 0..2 {
            let mut sortie = ctx.run_ui(input.clone(), |ui| {
                let police = egui::FontId::monospace(11.0);
                large = flow_box_width(ui, cfg, strings(Lang::Fr), &police, 1.0);
            });
            sortie.textures_delta.clear();
        }
        large
    };

    // Un programme aux instructions courtes, et le même avec une
    // instruction longue : le second doit demander plus de place.
    let court = namsdec_re::cfg::build(&disassemble_code(
        &namsdec_asm::assemble(
            "section .text\nglobal _start\n_start:\nnop\nhlt\n",
            BASE_ADDR,
        )
        .expect("l'assemblage"),
    ));
    let long = namsdec_re::cfg::build(&disassemble_code(
        &namsdec_asm::assemble(
            "section .text\nglobal _start\n_start:\nmov qword [rax+rbx*8+0x11223344], 0x55667788\nhlt\n",
            BASE_ADDR,
        )
        .expect("l'assemblage"),
    ));

    let (étroit, large) = (mesure(&court), mesure(&long));
    assert!(
        large > étroit,
        "une instruction plus longue demande une boîte plus large : {large} contre {étroit}"
    );
    // Et jamais au-delà des bornes : une seule instruction bavarde ne
    // doit pas étaler le graphe entier.
    assert!((220.0..=560.0).contains(&large), "{large}");
    assert!((220.0..=560.0).contains(&étroit), "{étroit}");
}

/// Changer de langue retraduit ce qui est déjà affiché.
///
/// La barre de statut gardait le texte figé dans la langue où la note
/// avait été posée : on chargeait un binaire en anglais, on passait en
/// français, et l'avertissement restait en anglais — dans la barre comme
/// dans son infobulle, qui montre la même phrase en entier.
#[test]
fn switching_language_retranslates_what_is_already_on_screen() {
    let mut app = NamsdecApp {
        lang: Lang::En,
        ..Default::default()
    };
    app.status = Status::Note(|s| s.binary_dynamic);

    let anglais = match &app.status {
        Status::Note(phrase) => phrase(strings(app.lang)),
        autre => panic!("obtenu {autre:?}"),
    };
    assert!(anglais.contains("dynamically linked"), "{anglais}");

    // La même note, lue en français : c'est la lecture qui traduit.
    app.lang = Lang::Fr;
    let français = match &app.status {
        Status::Note(phrase) => phrase(strings(app.lang)),
        autre => panic!("obtenu {autre:?}"),
    };
    assert!(français.contains("lié dynamiquement"), "{français}");
    assert_ne!(anglais, français, "la note suit la langue");

    // Et ce que la barre peint suit, elle aussi.
    let peint = paint(&mut app);
    assert!(
        peint.contains("lié dynamiquement"),
        "la barre de statut doit être en français :\n{peint}"
    );
}

/// Une boîte est assez haute pour tout ce qu'elle écrit.
///
/// Elle comptait l'en-tête et les huit instructions montrées, mais pas
/// la ligne « … 4 » qui dit combien restent : celle-ci se peignait sous
/// le cadre, à côté du bloc d'en dessous.
#[test]
fn a_block_is_tall_enough_for_the_line_that_counts_the_rest() {
    const LIGNES_MAX: usize = 8;
    // La règle telle que le dessin l'applique : l'en-tête, ce qu'on
    // montre, et la ligne de reste quand il y en a un.
    let lignes = |combien: usize| 1 + combien.min(LIGNES_MAX) + usize::from(combien > LIGNES_MAX);

    // Un bloc court : l'en-tête et ses instructions, rien de plus.
    assert_eq!(lignes(3), 4);
    // Pile la limite : rien n'est caché, donc pas de ligne de reste.
    assert_eq!(lignes(LIGNES_MAX), LIGNES_MAX + 1);
    // Une de trop : la ligne « … 1 » doit avoir sa place.
    assert_eq!(lignes(LIGNES_MAX + 1), LIGNES_MAX + 2);
    // Et le bloc de la capture : douze instructions, huit montrées.
    assert_eq!(lignes(12), 10);
}

/// L'en-tête d'un bloc et « le programme est ici » ne se recouvrent pas.
///
/// Les deux étaient peints aux deux bouts de la même ligne sans que
/// personne ne mesure la place : sur une boîte étroite, le libellé
/// venait s'imprimer par-dessus l'adresse, et l'on ne lisait plus ni
/// l'un ni l'autre. La largeur des boîtes leur réserve désormais de quoi
/// tenir côte à côte.
#[test]
fn a_block_header_and_the_here_label_never_overlap() {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };

    // Le cas le plus serré : un programme dont toutes les instructions
    // sont courtes, où rien d'autre ne force la boîte à s'élargir.
    let cfg = namsdec_re::cfg::build(&disassemble_code(
        &namsdec_asm::assemble(
            "section .text\nglobal _start\n_start:\nnop\nhlt\n",
            BASE_ADDR,
        )
        .expect("l'assemblage"),
    ));

    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        let mut large = 0.0;
        let mut glyphe = 0.0;
        for _ in 0..2 {
            let mut sortie = ctx.run_ui(input.clone(), |ui| {
                let police = egui::FontId::monospace(11.0);
                large = flow_box_width(ui, &cfg, s, &police, 1.0);
                glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(&police, ' '));
            });
            sortie.textures_delta.clear();
        }
        // Ce qui tient dans la boîte, marges déduites — le même calcul
        // que le dessin.
        let colonnes = ((large - 12.0) / glyphe).floor() as usize;
        // « 0x0001d491  ? » puis le libellé, avec de quoi les séparer.
        let nécessaire = 16 + s.flow_here.chars().count() + 3;
        assert!(
            colonnes >= nécessaire,
            "{lang:?} : {colonnes} colonnes pour {nécessaire} nécessaires — \
             « {} » recouvrirait l'adresse",
            s.flow_here
        );
    }
}

/// Une fenêtre interne s'agrandit et retrouve sa taille.
///
/// Une `egui::Window` n'a pas les pastilles du système : elle se traîne
/// et se redimensionne à la main, ce qui est long quand on veut
/// simplement tout voir — un graphe de blocs ou du C décompilé se lisent
/// sur toute la hauteur qu'on a.
#[test]
fn an_inner_window_can_be_maximized_and_restored() {
    let mut app = NamsdecApp::default();
    assert!(!app.window_maximized(NamsdecApp::FLOW_WINDOW));

    app.toggle_maximized(NamsdecApp::FLOW_WINDOW);
    assert!(app.window_maximized(NamsdecApp::FLOW_WINDOW));
    // Chaque fenêtre garde son état : agrandir l'une n'agrandit pas
    // l'autre.
    assert!(!app.window_maximized(NamsdecApp::DECOMPILER_WINDOW));

    app.toggle_maximized(NamsdecApp::DECOMPILER_WINDOW);
    assert!(app.window_maximized(NamsdecApp::DECOMPILER_WINDOW));
    assert!(app.window_maximized(NamsdecApp::FLOW_WINDOW));

    app.toggle_maximized(NamsdecApp::FLOW_WINDOW);
    assert!(!app.window_maximized(NamsdecApp::FLOW_WINDOW));
    assert!(
        app.window_maximized(NamsdecApp::DECOMPILER_WINDOW),
        "rendre sa taille à l'une laisse l'autre agrandie"
    );

    // Le rectangle d'une fenêtre agrandie.
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(input, |_ui| {});
    sortie.textures_delta.clear();
    // Agrandie veut dire tout l'écran. Une marge décorative, même de
    // douze pixels, laissait voir ce que la fenêtre recouvre : elle
    // n'avait pas l'air agrandie, elle avait l'air mal posée.
    let rect = NamsdecApp::maximized_rect(&ctx);
    assert_eq!(rect, ctx.viewport_rect(), "toute la place, sans marge");
}

/// Le bouton du bandeau agrandit vraiment la fenêtre.
///
/// Il est dessiné par-dessus la fenêtre, à la géométrie que celle-ci
/// emploie pour sa croix : on vérifie que le clic y arrive, et non
/// seulement que le dessin tombe au bon endroit.
#[test]
fn the_title_bar_button_really_maximizes_the_window() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "section .text\nglobal _start\n_start:\nnop\nhlt\n".to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.do_assemble();

    let ctx = egui::Context::default();
    let base = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    let frame = |app: &mut NamsdecApp, input: egui::RawInput| {
        let mut sortie = ctx.run_ui(input, |ui| app.draw(ui));
        sortie.textures_delta.clear();
    };
    // Trois passes : la fenêtre doit exister et avoir sa place avant
    // qu'on vise quoi que ce soit dedans.
    for _ in 0..3 {
        frame(&mut app, base.clone());
    }
    assert!(!app.window_maximized(NamsdecApp::FLOW_WINDOW));

    // Là où le bouton se trouve, calculé comme le dessin le calcule.
    let style = ctx.style_of(ctx.theme());
    let case = ctx.fonts_mut(|f| f.row_height(&egui::TextStyle::Heading.resolve(&style)));
    let écart = style.spacing.item_spacing.x;
    let marge_droite = f32::from(style.spacing.window_margin.right);
    let marge_haut = f32::from(style.spacing.window_margin.top);
    let rect = ctx
        .memory(|m| m.area_rect(egui::Id::new("flow_window")))
        .expect("la fenêtre est ouverte");
    let croix = egui::Rect::from_min_size(
        egui::pos2(rect.right() - marge_droite - case, rect.top() + marge_haut),
        egui::vec2(case, case),
    );
    let cible = croix.translate(egui::vec2(-(case + écart), 0.0)).center();
    assert!(
        rect.contains(cible),
        "le bouton doit tomber dans la fenêtre : {cible:?} hors de {rect:?}"
    );

    let mut presse = base.clone();
    presse.events.push(egui::Event::PointerMoved(cible));
    presse.events.push(egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    frame(&mut app, presse);
    let mut relâche = base.clone();
    relâche.events.push(egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    frame(&mut app, relâche);

    assert!(
        app.window_maximized(NamsdecApp::FLOW_WINDOW),
        "le clic sur le bouton du bandeau doit agrandir la fenêtre"
    );
    // Et agrandir veut dire tout prendre.
    frame(&mut app, base.clone());
    let agrandie = ctx
        .memory(|m| m.area_rect(egui::Id::new("flow_window")))
        .expect("toujours ouverte");
    assert!(
        agrandie.width() > rect.width() && agrandie.height() > rect.height(),
        "{agrandie:?} n'est pas plus grande que {rect:?}"
    );
}

/// « Installer… » ouvre son plan, et le plan se voit.
///
/// Le bouton posait bien la demande, et la fenêtre se dessinait bien —
/// mais derrière celle des préférences, d'où elle était partie. On
/// cliquait, et il ne se passait rien de visible.
#[test]
fn the_install_button_brings_its_plan_to_the_front() {
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Tools,
        show_decompiler_module: true,
        show_installer_module: true,
        // CE QUE LA MACHINE RÉPOND, POSÉ ICI plutôt que demandé au
        // disque. Le test lisait vraiment `~/.local/share` pour savoir
        // quels outils portent un bouton « Installer » — il dépendait
        // donc de ce qui est installé sur le poste, et il a fini par
        // échouer le jour où namsdec lui-même a été installé dessus.
        //
        // Un seul outil manquant, tous les autres prêts : le bouton
        // cherché est alors le sien, et il n'y en a qu'un.
        tools_ready: namsdec_disasm::known::all()
            .into_iter()
            .map(|k| {
                // Deux outils incomplets, et non un seul : lequel des
                // deux boutons se trouve en premier dans la fenêtre
                // dépend de la mise en page, laquelle bouge quand on
                // touche à un widget voisin. Le test n'a pas à le
                // savoir — il a à vérifier que le bouton cliqué est
                // celui d'un outil qui a vraiment besoin d'être
                // installé.
                let état = if matches!(k.plugin.id.as_str(), "rizin-ghidra" | "retdec") {
                    namsdec_disasm::known::Availability::Incomplete
                } else {
                    namsdec_disasm::known::Availability::Ready
                };
                (k.plugin.id.clone(), état)
            })
            .collect(),
        ..Default::default()
    };
    let s = strings(app.lang);
    let ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };

    // Où se trouve le bouton, d'après ce qui est peint.
    let mut cible = None;
    for _ in 0..3 {
        cible = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape
                && texte.galley.text() == s.install_button
            {
                let milieu = texte.pos + texte.galley.size() / 2.0;
                // **Visible**, et pas seulement peint : la liste des
                // outils défile, et le dernier bouton peint est souvent
                // celui qui dépasse du bas. Cliquer à sa position ne
                // l'atteindrait pas — le test échouait alors en accusant
                // le clic, alors que c'est la cible qui était mauvaise.
                if clipped.clip_rect.contains(milieu) {
                    cible = Some(milieu);
                    break;
                }
            }
        }
        sortie.textures_delta.clear();
    }
    // Pas de bouton s'il n'y a rien à installer : sur une machine où
    // tout est déjà bâti, ce test n'a rien à observer. Il le dit plutôt
    // que d'échouer sur l'état du poste.
    let Some(cible) = cible else {
        return;
    };

    let mut presse = entrée.clone();
    presse.events.push(egui::Event::PointerMoved(cible));
    presse.events.push(egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(presse, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    let mut relâche = entrée.clone();
    relâche.events.push(egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(relâche, |ui| app.draw(ui));
    sortie.textures_delta.clear();

    // Le plan demandé est celui d'un outil qui a vraiment besoin d'être
    // installé — et non un nom écrit en dur : lequel des outils porte un
    // bouton « Installer » dépend de ce que la machine a déjà. Nommer
    // « rizin-ghidra » faisait échouer le test sur un poste où rizin
    // était complet, c'est-à-dire là où tout allait bien.
    let plan = app
        .install_plan
        .as_deref()
        .expect("le clic doit demander un plan");
    assert!(
        matches!(plan, "rizin-ghidra" | "retdec"),
        "le bouton cliqué est celui d'un outil qu'on a dit incomplet — « {plan} »"
    );

    // Et la fenêtre passe devant : son calque est au premier plan, non
    // rangé avec les fenêtres ordinaires comme les préférences.
    let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    // UN calque au premier plan, et non « le premier calque trouvé ».
    // Remettre une fenêtre devant crée un calque à son nom au rang
    // ordinaire, même quand la fenêtre vit au premier plan : deux
    // calques portent alors le même nom, et `find` rendait celui qui se
    // présentait en premier.
    let au_premier_plan = ctx.memory(|m| {
        m.layer_ids().any(|id| {
            id.id == egui::Id::new("installer_window") && id.order == egui::Order::Foreground
        })
    });
    assert!(
        au_premier_plan,
        "le plan doit se poser au-dessus des préférences d'où il part"
    );
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp {
        program_args: "sesame \"deux mots\"".to_owned(),
        ..Default::default()
    };
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");

    let binaire = app
        .assemble_state
        .as_ref()
        .and_then(|état| état.from_binary.as_ref())
        .expect("un binaire chargé");
    assert_eq!(
        binaire.load.argv,
        vec!["crackme-01", "sesame", "deux mots"],
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
#[test]
fn the_input_rows_line_up_and_stay_inside_their_frame() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");

    let s = strings(app.lang);
    let ctx = egui::Context::default();
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp {
        program_env: "SECRET=sesame\n\nLANG=fr_FR.UTF-8\n".to_owned(),
        ..Default::default()
    };
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");

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

/// Le mode Normal se promène dans le code.
///
/// `i` entrait en Insertion, Échap en sortait, et c'était tout : on ne
/// pouvait pas bouger le curseur sans repasser par la souris — la
/// première chose que cherche quiconque vient de vim.
#[test]
fn normal_mode_moves_the_cursor_like_vim() {
    let source = "mov rax, 1\nnop\nsyscall\n";
    // Position 0 : début de la première ligne.
    assert_eq!(move_cursor(source, 0, Move::Right), 1);
    // `h` et `l` ne quittent pas leur ligne, comme dans vim.
    assert_eq!(
        move_cursor(source, 0, Move::Left),
        0,
        "rien à gauche du début"
    );
    assert_eq!(
        move_cursor(source, 10, Move::Right),
        10,
        "rien après la fin de ligne"
    );
    // Début et fin de ligne.
    assert_eq!(move_cursor(source, 5, Move::LineStart), 0);
    assert_eq!(move_cursor(source, 5, Move::LineEnd), 10);
    // Descendre garde la colonne quand la ligne visée est assez longue.
    assert_eq!(move_cursor(source, 2, Move::Down), 13, "ligne 2, colonne 2");
    // Et la raccourcit sinon : « nop » n'a que trois caractères.
    assert_eq!(
        move_cursor(source, 8, Move::Down),
        14,
        "collé à la fin de « nop »"
    );
    // Remonter de même.
    assert_eq!(move_cursor(source, 13, Move::Up), 2);
    assert_eq!(
        move_cursor(source, 2, Move::Up),
        2,
        "rien au-dessus de la première"
    );
    // La fin du fichier.
    assert_eq!(
        move_cursor(source, 0, Move::FileEnd),
        source.chars().count()
    );

    // Les caractères comptent pour un, non pour leurs octets : sans quoi
    // le curseur tomberait au milieu d'un « é ».
    let accentué = "; déjà vu\nnop\n";
    let fin = move_cursor(accentué, 0, Move::LineEnd);
    assert_eq!(fin, 9, "neuf caractères, non douze octets");
    assert_eq!(move_cursor(accentué, fin, Move::Down), 13);
}

/// Le dossier des binaires d'épreuve : `tests/binaires/` du dépôt.
///
/// Voir son LISEZMOI. `NAMSDEC_TEST_BINARIES` le déplace ailleurs, pour
/// qui garde ses programmes hors du dépôt.
fn dossier_dépreuve() -> Option<std::path::PathBuf> {
    if let Some(ailleurs) = std::env::var_os("NAMSDEC_TEST_BINARIES") {
        let chemin = std::path::PathBuf::from(ailleurs);
        return chemin.is_dir().then_some(chemin);
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/binaires")
        .canonicalize()
        .ok()
}

/// Un binaire d'épreuve, s'il est là.
///
/// `None` plutôt qu'une panique : ces programmes ne sont pas dans le
/// dépôt — ce sont ceux de chacun —, et un test qui exigerait leur
/// présence échouerait sur toute machine fraîchement clonée. Il se tait,
/// et c'est la bonne réponse : il n'a rien à juger.
fn binaire_dépreuve(nom: &str) -> Option<Vec<u8>> {
    std::fs::read(dossier_dépreuve()?.join(nom)).ok()
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
#[test]
fn a_waiting_binary_shows_the_whole_input_area() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp {
        // La hauteur qu'on obtient en tirant la poignée : une hauteur
        // figée ne consulte plus celle que la console voudrait.
        console_height: Some(128.0),
        ..Default::default()
    };
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
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

/// La boîte « À propos » dit où trouver le projet : le site, le salon,
/// le dépôt.
///
/// Trois portes et pas une de plus — une boîte qui aligne dix liens ne
/// dit plus lequel compte —, et des adresses qui tiennent debout.
#[test]
fn the_about_box_opens_the_three_doors_of_the_project() {
    let mut app = NamsdecApp {
        show_about: true,
        ..Default::default()
    };
    let painted = paint(&mut app);
    let s = strings(app.lang);
    for porte in [s.about_site, s.about_discord, s.about_github] {
        assert!(painted.contains(porte), "« {porte} » manque : {painted}");
    }
    // Le nom en grand et la version à côté, en toutes lettres.
    assert!(painted.contains("namsdec"), "{painted}");
    assert!(painted.contains(&format!("v{APP_VERSION}")), "{painted}");

    // Les adresses : chiffrées, et celles qu'on a annoncées.
    for url in [SITE_URL, DISCORD_URL, GITHUB_URL, MIT_LICENSE_URL] {
        assert!(url.starts_with("https://"), "{url}");
        assert!(!url.ends_with('/'), "{url}");
    }
    assert_eq!(SITE_URL, "https://namsdec.io");
    assert!(DISCORD_URL.starts_with("https://discord.gg/"));
    assert_eq!(GITHUB_URL, "https://github.com/fredza/namsdec");
    // Le dépôt des liens et celui qu'interroge la recherche de mise à
    // jour sont le même : deux adresses qui divergent, et l'une des deux
    // finit fausse.
    assert!(GITHUB_URL.ends_with(&format!(
        "{}/{}",
        namsdec_update::OWNER,
        namsdec_update::REPO
    )));
}

/// La fenêtre porte ses trois pastilles au bout de la barre de menus,
/// et la barre de titre du système reste à un réglage près.
///
/// Dessiner la décoration soi-même économise une rangée : les menus
/// *sont* la barre de titre. Mais tout ce que le système faisait — la
/// déplacer, l'agrandir, la redimensionner — revient alors à notre
/// charge, et il faut pouvoir revenir en arrière si un bureau s'y prend
/// autrement.
#[test]
fn the_window_carries_its_own_three_buttons() {
    let mut app = NamsdecApp::default();
    assert!(!app.system_titlebar, "la décoration maison par défaut");

    // Les trois pastilles sont peintes : trois disques, de trois
    // couleurs, au bout de la barre.
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let disques = |app: &mut NamsdecApp| -> Vec<(egui::Color32, f32)> {
        let mut out = Vec::new();
        for _ in 0..2 {
            out.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Circle(cercle) = &clipped.shape
                    && cercle.center.y < 40.0
                {
                    out.push((cercle.fill, cercle.center.x));
                }
            }
            output.textures_delta.clear();
        }
        out
    };
    let peints = disques(&mut app);
    assert_eq!(peints.len(), 3, "trois pastilles : {peints:?}");
    // En tête de barre, à gauche : la fermeture occupe le coin de
    // l'écran, l'angle qu'une souris lancée vers le haut à gauche
    // atteint sans viser.
    let mut x: Vec<f32> = peints.iter().map(|(_, x)| *x).collect();
    x.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(x[2] < 1280.0 * 0.15, "en tête de la barre : {x:?}");
    // Alignées : le même écart entre les trois, et la première ne touche
    // pas le bord de la fenêtre.
    let écarts: Vec<f32> = x.windows(2).map(|p| p[1] - p[0]).collect();
    assert!(
        (écarts[0] - écarts[1]).abs() < 0.5,
        "l'écart doit être le même : {écarts:?}"
    );
    assert!(
        x[0] > 6.0,
        "la fermeture est collée au bord : {} pour une fenêtre de 1280",
        x[0]
    );
    // Et dans l'ordre : fermer d'abord, puis réduire, puis agrandir —
    // celui de macOS, qui va avec le côté gauche.
    let couleur_à = |abscisse: f32| {
        peints
            .iter()
            .find(|(_, x)| (*x - abscisse).abs() < 0.5)
            .map(|(couleur, _)| *couleur)
            .expect("une pastille à cette abscisse")
    };
    // Les pastilles sont atténuées tant que la souris n'est pas dessus :
    // on compare donc à la couleur telle qu'elle est peinte au repos.
    let au_repos = |bouton: chrome::WindowButton| bouton.colour().gamma_multiply(0.55);
    assert_eq!(
        couleur_à(x[0]),
        au_repos(chrome::WindowButton::Close),
        "fermer au coin : {peints:?}"
    );
    assert_eq!(
        couleur_à(x[1]),
        au_repos(chrome::WindowButton::Minimize),
        "réduire au milieu : {peints:?}"
    );
    assert_eq!(
        couleur_à(x[2]),
        au_repos(chrome::WindowButton::Maximize),
        "agrandir en dernier : {peints:?}"
    );
    // Trois couleurs distinctes, et aucune n'est celle du fond.
    let couleurs: std::collections::BTreeSet<[u8; 4]> =
        peints.iter().map(|(c, _)| c.to_array()).collect();
    assert_eq!(couleurs.len(), 3, "{peints:?}");

    // Le réglage rend la décoration au système, et les pastilles s'en
    // vont avec elle.
    app.system_titlebar = true;
    assert!(disques(&mut app).is_empty());
    // Et il se retrouve au lancement suivant, comme tout réglage.
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(écrit.contains("\"system_titlebar\":true"), "{écrit}");
}

/// La boîte « À propos » est large, porte l'icône de l'application, et
/// ses trois portes ont chacune leur marque.
///
/// Une colonne de trois cents pixels repliait les boutons en accordéon
/// et coupait le bandeau de préversion en tranches.
#[test]
fn the_about_box_is_wide_and_carries_its_marks() {
    let mut app = NamsdecApp {
        show_about: true,
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
    let mut cadres: Vec<egui::Rect> = Vec::new();
    let mut images = 0usize;
    for _ in 0..3 {
        cadres.clear();
        images = 0;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Rect(forme) if forme.rect.width() > 400.0 => {
                    cadres.push(forme.rect);
                }
                egui::epaint::Shape::Mesh(_) => images += 1,
                egui::epaint::Shape::Rect(forme) if forme.brush.is_some() => images += 1,
                _ => {}
            }
        }
        output.textures_delta.clear();
    }
    assert!(
        cadres.iter().any(|rect| rect.width() >= ABOUT_WIDTH - 24.0),
        "la boîte doit faire au moins {ABOUT_WIDTH} de large : {cadres:?}"
    );
    // L'icône de l'application : une image, et non un carré de couleur.
    assert!(images > 0, "l'icône n'est pas peinte");

    // Les trois portes sont là, avec leurs adresses.
    let painted = paint(&mut app);
    let s = strings(app.lang);
    for porte in [s.about_site, s.about_discord, s.about_github] {
        assert!(painted.contains(porte), "{painted}");
    }
    // Et le bandeau de préversion n'a pas disparu dans la refonte.
    if prerelease(APP_VERSION).is_some() {
        assert!(painted.contains(s.about_beta_title), "{painted}");
    }
}

/// Un clic sur une porte du projet ouvre vraiment son adresse.
///
/// Les trois boutons ne faisaient rien : `open_url` d'eframe est derrière
/// la fonctionnalité `links`, que nous n'activions pas — elle fait partie
/// des `default` que le projet désactive pour choisir son moteur de
/// rendu. Le clic partait, la commande aussi, et personne ne l'écoutait.
#[test]
fn clicking_a_door_really_opens_its_address() {
    // La fonctionnalité est demandée : sans elle, rien de ce qui suit
    // n'arriverait jusqu'au navigateur.
    let manifeste = include_str!("../../../Cargo.toml");
    let ligne = manifeste
        .lines()
        .find(|ligne| ligne.starts_with("eframe = "))
        .expect("eframe est déclarée");
    assert!(
        ligne.contains("\"links\""),
        "sans `links`, `open_url` ne fait rien : {ligne}"
    );

    // Et le clic envoie bien la commande d'ouverture, avec la bonne
    // adresse.
    let mut app = NamsdecApp {
        show_about: true,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = |events: Vec<egui::Event>| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        events,
        ..Default::default()
    };
    // Deux images pour poser la fenêtre, puis on cherche où est le
    // bouton « Site » avant de cliquer dessus.
    let mut position = None;
    for _ in 0..3 {
        let mut output = ctx.run_ui(input(Vec::new()), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(forme) = &clipped.shape
                && forme.galley.text() == strings(app.lang).about_site
            {
                position = Some(forme.pos + forme.galley.size() * 0.5);
            }
        }
        output.textures_delta.clear();
    }
    let position = position.expect("le bouton « Site » est peint");
    let clic = vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ];
    let mut output = ctx.run_ui(input(clic), |ui| app.draw(ui));
    output.textures_delta.clear();
    let demandée = output
        .platform_output
        .commands
        .iter()
        .find_map(|commande| match commande {
            egui::OutputCommand::OpenUrl(open) => Some(open.url.clone()),
            _ => None,
        });
    assert_eq!(
        demandée,
        Some(SITE_URL.to_owned()),
        "le clic doit demander l'ouverture du site"
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

/// Le panneau dit ce qu'il montre : depuis que les onglets sont montés
/// dans la barre d'outils, une colonne d'adresses n'annonçait plus
/// qu'elle était une pile.
#[test]
fn the_sidebar_names_the_view_it_shows() {
    let mut app = NamsdecApp {
        show_sidebar: true,
        active_tab: SidebarTab::Stack,
        ..Default::default()
    };
    let s = strings(app.lang);
    // Le titre est peint deux fois : une fois comme onglet dans la barre
    // d'outils, une fois comme en-tête de la vue.
    let compte =
        |app: &mut NamsdecApp, titre: &str| paint(app).matches(&format!("{titre}|")).count();
    assert_eq!(compte(&mut app, s.tab_stack), 2, "onglet et en-tête");
    app.active_tab = SidebarTab::Heap;
    assert_eq!(compte(&mut app, s.tab_heap), 2);
    // Et l'onglet qu'on ne regarde pas n'a que son bouton.
    assert_eq!(compte(&mut app, s.tab_stack), 1);
}

/// Le panneau se lit d'un coup d'œil : les zéros de tête s'effacent, le
/// détail d'une instruction se replie, et les modules RE ne s'ouvrent
/// pas tous à la fois.
#[test]
fn the_panel_puts_forward_what_matters() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Les zéros de tête sont peints dans une teinte à part : ce qu'on
    // lit en premier est la valeur, pas le remplissage.
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let mut sections = 0usize;
    for _ in 0..2 {
        sections = 0;
        let mut output = ctx.run_ui(input.clone(), |ui| {
            let job = hex_job(ui, app.theme, 0x40ff00, app.theme.text);
            // Deux morceaux : les zéros, puis la valeur.
            sections = job.sections.len();
            app.draw(ui);
        });
        output.textures_delta.clear();
    }
    assert_eq!(sections, 2, "les zéros de tête sont à part");
    // Une valeur nulle s'efface en entier : un seul morceau, et rien
    // qui attire l'œil sur le registre qui n'a rien à dire.
    let mut vides = 0usize;
    let mut output = ctx.run_ui(input.clone(), |ui| {
        vides = hex_job(ui, app.theme, 0, app.theme.text).sections.len();
    });
    output.textures_delta.clear();
    assert_eq!(vides, 1, "à zéro, tout passe au second plan");

    // Le détail de l'instruction est replié devant une instruction
    // ordinaire, et s'ouvre de lui-même devant un appel.
    app.source = "\
global _start
section .text
_start:
    mov rax, 1
    call quelquepart
    hlt
quelquepart:
    ret
"
    .to_owned();
    app.do_assemble();
    assert!(!app.next_detail_open, "replié au départ");
    let s = strings(app.lang);
    let painted = paint(&mut app);
    assert!(painted.contains(s.next_instruction_detail), "{painted}");
    assert!(!painted.contains(s.call_arguments), "rien du détail encore");

    app.do_step(); // sur le `call`
    let painted = paint(&mut app);
    assert!(app.next_detail_open, "un appel mérite son détail");
    assert!(painted.contains(s.call_arguments), "{painted}");

    // Et replié à la main, il le reste sur cette instruction-là.
    app.next_detail_open = false;
    let painted = paint(&mut app);
    assert!(!painted.contains(s.call_arguments), "{painted}");
}

/// La barre de titre porte un titre : le nom du fichier, au milieu, avec
/// la marque de ce qui n'est pas enregistré.
///
/// Une barre de titre sans titre laisse deviner, à l'alt-tab comme au
/// réveil, sur quoi l'on travaillait.
#[test]
fn the_title_bar_carries_the_file_name() {
    let mut app = NamsdecApp::default();
    let s = strings(app.lang);
    let painted = paint(&mut app);
    // Sans fichier, le nom du tampon — et il est déjà modifié ou non
    // comme n'importe quel autre.
    assert!(painted.contains(s.untitled_buffer), "{painted}");

    // Avec un fichier : son nom, et la marque quand il a changé.
    app.current_file = Some(std::path::PathBuf::from("/tmp/exemple.asm"));
    app.saved_source = app.source.clone();
    let painted = paint(&mut app);
    assert!(painted.contains("exemple.asm"), "{painted}");
    assert!(!painted.contains("exemple.asm •"), "rien à enregistrer");

    app.source.push_str("    nop\n");
    // DEUX fois avec le point : le titre de la fenêtre et la barre
    // d'état. L'onglet le dit aussi, mais par un ROND peint à la place
    // de sa croix — c'est ce que fait tout éditeur, et cela ne se
    // compte pas dans le texte.
    assert_eq!(
        paint(&mut app).matches("exemple.asm •").count(),
        2,
        "le titre et le statut portent le point ; l'onglet, un rond"
    );

    // Et quand la barre du système reprend la main, namsdec n'écrit
    // plus de titre : ce serait le dire deux fois. Reste la barre
    // d'état.
    app.system_titlebar = true;
    assert_eq!(
        paint(&mut app).matches("exemple.asm •").count(),
        1,
        "le statut seul"
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
        painted.contains(&format!("1/96 {}", strings(app.lang).exercises_done_count)),
        "{painted}"
    );
}

/// La fenêtre d'analyse respire, et sa colonne s'aligne.
///
/// Sans marge, le texte se colle au cadre de la fenêtre et à la ligne
/// qui sépare les deux colonnes : un tableau donne alors l'impression
/// d'être coupé. Et treize libellés centrés n'ont plus de bord commun —
/// l'œil doit chercher chacun au lieu de descendre une liste.
#[test]
fn the_analysis_window_has_margins_and_a_left_aligned_column() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
    app.show_analysis = true;
    app.analysis_module = "protections".to_owned();

    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    // Où chaque texte est peint, et jusqu'où il va.
    let mut posés: Vec<(String, f32, f32)> = Vec::new();
    for _ in 0..3 {
        posés.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                posés.push((
                    texte.galley.text().to_owned(),
                    texte.pos.x,
                    texte.pos.x + texte.galley.size().x,
                ));
            }
        }
        sortie.textures_delta.clear();
    }

    let fenêtre = egui::AreaState::load(&ctx, egui::Id::new("analysis_window"))
        .expect("la fenêtre est affichée");
    let bord_gauche = fenêtre.left_top_pos().x;

    let où = |quoi: &str| -> (f32, f32) {
        posés
            .iter()
            .find(|(t, _, _)| t == quoi)
            .map(|(_, g, d)| (*g, *d))
            .unwrap_or_else(|| panic!("« {quoi} » est peint"))
    };

    // Les entrées de la colonne commencent toutes au même endroit.
    let titres = app.re_registry.titles("fr");
    let gauches: Vec<f32> = titres.iter().map(|t| où(t).0).collect();
    let mini = gauches.iter().cloned().fold(f32::MAX, f32::min);
    let maxi = gauches.iter().cloned().fold(f32::MIN, f32::max);
    assert!(
        maxi - mini < 0.5,
        "les entrées doivent partager un bord gauche : {gauches:?}"
    );

    // Et cette colonne ne touche pas le cadre de la fenêtre.
    assert!(
        mini - bord_gauche >= 6.0,
        "la colonne colle au bord : {mini} pour une fenêtre à {bord_gauche}"
    );

    // Le contenu ne touche pas la colonne : le texte du module commence
    // franchement à droite du plus long libellé.
    let fin_de_colonne = titres.iter().map(|t| où(t).1).fold(f32::MIN, f32::max);
    // Le titre du module est peint deux fois : dans la colonne et en
    // tête du contenu. C'est celui de droite qu'on veut.
    let début_du_contenu = posés
        .iter()
        .filter(|(t, _, _)| t == "Protections du binaire")
        .map(|(_, g, _)| *g)
        .fold(f32::MIN, f32::max);
    assert!(
        début_du_contenu - fin_de_colonne >= 10.0,
        "le contenu colle à la colonne : {début_du_contenu} contre {fin_de_colonne}"
    );
}

/// La carte du fichier dit où est quoi, et le vidage ce que ces
/// octets-là sont.
///
/// Les deux répondent à la question qui vient après « qu'est-ce que
/// c'est ? » : savoir à quel endroit on regarde change ce qu'on y
/// cherche — une entropie haute dans `.text` est normale, la même dans
/// `.rodata` annonce des données chiffrées.
#[test]
fn the_map_and_the_dump_say_where_and_what() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
    app.show_analysis = true;

    let ids: Vec<&str> = app
        .re_registry
        .entries("fr")
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    for voulu in ["map", "dump"] {
        assert!(
            ids.contains(&voulu),
            "le module « {voulu} » existe : {ids:?}"
        );
    }

    // La carte : les segments que le noyau charge, avec leurs droits, et
    // les sections que l'éditeur de liens a rangées.
    app.analysis_module = "map".to_owned();
    let peint = paint(&mut app);
    assert!(
        peint.contains("LOAD"),
        "les segments chargeables :\n{peint}"
    );
    assert!(peint.contains("r-x"), "et leurs droits :\n{peint}");
    assert!(peint.contains(".text"), "les sections nommées :\n{peint}");

    // Le vidage : la tranche de départ, son empreinte, son entropie.
    app.analysis_module = "dump".to_owned();
    let peint = paint(&mut app);
    // Les seize premiers octets d'un ELF commencent par son nombre
    // magique, et c'est ce que la colonne de texte montre.
    assert!(peint.contains("ELF"), "le début du fichier :\n{peint}");
    let empreinte = namsdec_re::digest::hex(&namsdec_re::digest::sha256(&octets[..16]));
    assert!(
        peint.contains(&empreinte),
        "l'empreinte des seize premiers octets, et non celle du fichier :\n{peint}"
    );
    assert!(peint.contains("SHA-256"), "{peint}");
    assert!(peint.contains("CRC-32"), "{peint}");
}

/// Le rapport d'analyse porte l'empreinte du fichier.
///
/// C'est la première ligne d'un compte rendu : de quel octet parle-t-on ?
/// Deux personnes qui lisent « le crackme » n'ont pas forcément le même
/// fichier sous les yeux.
#[test]
fn the_analysis_gives_the_fingerprint_of_the_file() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");

    // Celle du fichier entier, comme `sha256sum` la donne — sinon on ne
    // pourrait pas la comparer à celle d'un autre outil.
    let attendue = namsdec_re::digest::hex(&namsdec_re::digest::sha256(&octets));
    assert_eq!(attendue.len(), 64);

    app.show_analysis = true;
    // Le module est dans la fenêtre, et son titre dans la colonne.
    let ids: Vec<&str> = app
        .re_registry
        .entries("fr")
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(ids.contains(&"digest"), "le module existe : {ids:?}");

    app.analysis_module = "digest".to_owned();
    let peint = paint(&mut app);
    assert!(
        peint.contains(&attendue),
        "l'empreinte du fichier doit s'y lire :\n{peint}"
    );
}

/// La fenêtre d'analyse se lit en largeur, pas seulement en hauteur.
///
/// Les sept modules empilés n'employaient que la hauteur : on relisait
/// une page de deux mille pixels dans une fenêtre large de mille, en
/// cherchant un titre à l'ascenseur. Une colonne choisit, le module
/// prend le reste.
#[test]
fn the_analysis_window_reads_across_and_not_only_down() {
    let s = strings(Lang::Fr);
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let ctx = egui::Context::default();
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1900.0, 1200.0));
    let entrée = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| -> String {
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
        peint
    };

    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
    app.show_analysis = true;

    // La colonne porte « Tout » et le nom de chaque module.
    let titres = app.re_registry.titles("fr");
    assert!(titres.len() >= 5, "il y a bien des modules : {titres:?}");
    let peint = peindre(&mut app);
    assert!(
        peint.contains(s.analysis_all),
        "« Tout » est dans la colonne :\n{peint}"
    );
    for titre in &titres {
        assert!(peint.contains(titre), "« {titre} » est dans la colonne");
    }

    // Choisi, un seul module s'affiche — et les autres ne sont plus que
    // des entrées de la colonne, non des pages à faire défiler.
    let (id, titre) = app.re_registry.entries("fr")[1];
    app.analysis_module = id.to_owned();
    let peint = peindre(&mut app);
    assert!(
        peint.contains(titre),
        "le module choisi est montré :\n{peint}"
    );

    // Et le choix se retrouve au retour : c'est un réglage.
    assert_eq!(app.settings_snapshot().analysis_module, id);
    let relu = NamsdecApp::from_settings(app.settings_snapshot());
    assert_eq!(relu.analysis_module, id);
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
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

/// La provenance remonte plusieurs pas, et non un seul.
///
/// « D'où vient ce que RAX contient ? » est la question qu'on pose
/// devant chaque comparaison d'un programme qu'on ne connaît pas, et
/// une seule réponse ne suffit presque jamais : le pas qui a écrit RAX
/// lisait RCX, et c'est RCX qu'il faut alors remonter. On le faisait à
/// la main, un survol après l'autre.
#[test]
fn the_provenance_goes_back_several_steps() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Une chaîne volontairement nette : RDX vient de RCX, qui vient de
    // RBX, qui vient d'une constante.
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
    mov rbx, 7
    mov rcx, rbx
    mov rdx, rcx
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let chaîne = app.provenance_chain("RDX", 4);
    assert!(
        chaîne.len() >= 3,
        "trois maillons au moins : RDX, RCX, RBX — {chaîne:?}"
    );
    // Chaque maillon nomme le registre qu'il explique, et l'instruction
    // qui l'a écrit.
    assert!(chaîne[0].starts_with("RDX"), "{:?}", chaîne[0]);
    assert!(chaîne[0].contains("mov rdx, rcx"), "{:?}", chaîne[0]);
    assert!(chaîne[1].starts_with("RCX"), "{:?}", chaîne[1]);
    assert!(chaîne[1].contains("mov rcx, rbx"), "{:?}", chaîne[1]);
    assert!(chaîne[2].starts_with("RBX"), "{:?}", chaîne[2]);
    // `mov ebx, 7` et non `mov rbx, 7` : nasm choisit la forme 32 bits
    // pour un petit immédiat, et l'écriture d'un registre de 32 bits met
    // à zéro la moitié haute du registre de 64. C'est le même effet en
    // deux octets de moins, et c'est ce que le désassemblage montre.
    assert!(chaîne[2].contains("bx, 7"), "{:?}", chaîne[2]);

    // Bornée : on demande quatre maillons, on n'en reçoit pas cinq.
    assert!(app.provenance_chain("RDX", 2).len() <= 2);
    // Un registre que personne n'a écrit n'a pas de provenance.
    assert!(app.provenance_chain("R15", 4).is_empty());
}

/// La chaîne s'arrête à la mémoire, et le dit.
///
/// Une valeur qui vient d'un `[rbp-8]` ne se remonte pas par son
/// registre de base : cela mènerait à la mise en place du cadre de
/// pile, pas à la valeur. Mieux vaut s'arrêter en le disant que de
/// conduire ailleurs avec assurance.
#[test]
fn the_chain_stops_at_memory_and_says_so() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
valeur dq 42
section .text
_start:
    mov rax, [valeur]
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let chaîne = app.provenance_chain("RAX", 4);
    assert!(chaîne.len() >= 2, "{chaîne:?}");
    assert!(chaîne[0].contains("mov rax,"), "{:?}", chaîne[0]);
    assert_eq!(
        chaîne[1], s.provenance_from_memory,
        "la piste s'arrête là, et le dit"
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
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
    assert!(
        !peint.contains(s.console_send),
        "on ne propose plus d'envoyer à un programme terminé :\n{peint}"
    );

    // Relancer la referme : elle parlait de la course précédente.
    app.do_run();
    assert!(!app.show_input_dialog, "une nouvelle course repart à zéro");
}

/// Le vidage s'enregistre, au format que `diff` sait lire.
///
/// Un vidage sert à être comparé : à celui qu'on a sous les yeux, à
/// celui d'un autre outil, à la même analyse faite la semaine dernière.
#[test]
fn the_dump_can_be_saved_in_a_shape_one_can_compare() {
    let s = strings(Lang::Fr);
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
    app.show_analysis = true;

    // Le bouton n'apparaît que sur le module qui le concerne : c'est le
    // fichier qu'on enregistre, pas un rapport.
    app.analysis_module = "protections".to_owned();
    let peint = paint(&mut app);
    assert!(!peint.contains(s.dump_save), "pas ailleurs :\n{peint}");
    app.analysis_module = "dump".to_owned();
    let peint = paint(&mut app);
    assert!(peint.contains(s.dump_save), "et là, oui :\n{peint}");

    // Et ce qu'il écrit est le fichier entier, au format de xxd.
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("vidage.txt");
    app.export_dump(&chemin);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    let écrit = std::fs::read_to_string(&chemin).expect("le vidage est là");
    assert!(
        écrit.starts_with("00000000  7f 45 4c 46"),
        "un ELF :\n{}",
        &écrit[..80]
    );
    assert_eq!(
        écrit.lines().count(),
        octets.len().div_ceil(16),
        "seize octets par ligne, et tout le fichier"
    );
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

/// Un message d'état trop long se lit en entier au survol.
///
/// La barre le tronque pour ne pas passer sur les compartiments de
/// droite. Tronqué sans recours, il cache justement ce qu'on voulait
/// savoir : le chemin d'un fichier, la phrase d'une erreur de liaison.
#[test]
fn a_truncated_status_message_can_be_read_in_full() {
    // Le texte que la barre composerait, et celui que l'infobulle doit
    // répéter : c'est le même, et une seule fonction le produit.
    let long = "x".repeat(400);
    let mut app = NamsdecApp {
        status: Status::Error(long.clone()),
        ..Default::default()
    };
    let (texte, _, pastille) = app.status_line();
    assert!(texte.contains(&long), "la phrase entière : {}", texte.len());
    assert!(texte.starts_with("Erreur : "));
    assert!(pastille.is_none());

    // La barre la peint tronquée : egui garde le texte entier dans sa
    // galley — c'est pourquoi le comparer ne dirait rien — mais marque
    // qu'il a été élidé.
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    // Le temps avance d'une image à l'autre : namsdec fait attendre une
    // demi-seconde avant de montrer une infobulle — le réglage existe
    // pour que personne ne les subisse —, et sans horloge qui tourne
    // elle n'arriverait jamais.
    let entrée = |instant: f64, événements: Vec<egui::Event>| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 700.0),
        )),
        time: Some(instant),
        predicted_dt: 0.1,
        events: événements,
        ..Default::default()
    };
    let mut horloge = 0.0_f64;
    let mut case = None;
    let mut élidé = false;
    for _ in 0..2 {
        horloge += 0.1;
        let mut sortie = ctx.run_ui(entrée(horloge, Vec::new()), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && t.galley.text() == texte
            {
                élidé = t.galley.elided;
                case = Some(egui::Rect::from_min_size(t.pos, t.galley.size()));
            }
        }
        sortie.textures_delta.clear();
    }
    let case = case.expect("le message est peint");
    assert!(
        élidé,
        "le message est trop long pour tenir : il doit être coupé"
    );

    // Et au survol, il se lit en entier : l'infobulle le répète, donc le
    // texte est peint une seconde fois.
    let mut combien = 0;
    for tour in 0..12 {
        horloge += 0.1;
        combien = 0;
        // Le pointeur est posé une fois, puis il reste là : un
        // déplacement à chaque image remettrait le délai à zéro.
        let événements = if tour == 0 {
            vec![egui::Event::PointerMoved(case.center())]
        } else {
            Vec::new()
        };
        let mut sortie = ctx.run_ui(entrée(horloge, événements), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && t.galley.text() == texte
            {
                combien += 1;
            }
        }
        sortie.textures_delta.clear();
        if combien >= 2 {
            break;
        }
    }
    assert!(
        combien >= 2,
        "au survol, l'infobulle doit répéter le message en entier ({combien} fois vu)"
    );

    // Et chaque état sait se dire en texte : sans cela, l'infobulle
    // dirait autre chose que la barre.
    for état in [
        Status::Idle,
        Status::Running,
        Status::AssembleOk,
        Status::AssembleFailed,
        Status::Halted,
        Status::WaitingForInput,
        Status::SteppedBack,
        Status::NothingToUndo,
        Status::StepLimit,
        Status::NasmMissing,
        Status::Exited { code: 0 },
        Status::Breakpoint(0x400000),
        Status::Watchpoint {
            addr: 0x401000,
            old: 0,
            new: 1,
        },
    ] {
        app.status = état;
        let (texte, _, _) = app.status_line();
        assert!(
            !texte.trim().is_empty(),
            "un état sans phrase : {:?}",
            app.status
        );
    }
}

/// Le vidage laisse écrire, et ce qu'il écrit devient un patch qu'on
/// peut défaire.
///
/// Le vidage montrait ; il laisse maintenant changer, ce qui est la
/// suite naturelle : on a repéré la constante qu'on veut voir
/// autrement, et l'on n'avait aucun moyen de la poser.
#[test]
fn the_dump_can_write_bytes_and_the_change_is_a_patch() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
    assert!(app.patches.is_empty());

    // Ce qu'un module demande, l'application le fait : c'est le chemin
    // que suit le bouton « Écrire » du vidage.
    let offset = 0x1000;
    let nouveaux = vec![0x90u8, 0x90, 0x90, 0x90];
    let avant = octets[offset..offset + nouveaux.len()].to_vec();
    assert_ne!(avant, nouveaux, "de quoi voir la différence");
    app.write_patch(offset, nouveaux.clone(), 0);

    assert_eq!(app.patches.len(), 1, "un changement enregistré");
    assert_eq!(app.patches[0].offset, offset);
    assert_eq!(app.patches[0].before, avant);
    assert_eq!(app.patches[0].addr, 0, "une position, non une adresse");
    let patché = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(
        &patché[offset..offset + nouveaux.len()],
        nouveaux.as_slice()
    );

    // Écrire ce qui s'y trouve déjà n'enregistre rien : un patch qui ne
    // change rien encombrerait la liste de ce qu'on peut défaire.
    app.write_patch(offset, nouveaux.clone(), 0);
    assert_eq!(app.patches.len(), 1, "et toujours un seul");

    // Et il se défait comme les autres.
    app.undo_patch(0);
    assert!(app.patches.is_empty());
    let rendu = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(&rendu[offset..offset + avant.len()], avant.as_slice());
}

/// Patcher un binaire : inverser un saut, et le retrouver dans le
/// fichier exporté.
///
/// C'est le geste qui manquait le plus. On ouvre un crackme, on voit le
/// `je` qu'il faudrait inverser, et l'on ne pouvait rien en faire :
/// réassembler le listing est refusé — à juste titre, un désassemblage
/// n'est pas un source.
#[test]
fn a_jump_can_be_inverted_and_the_change_reaches_the_exported_file() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");

    // La ligne d'un saut conditionnel : on la cherche dans le listing,
    // comme on la chercherait des yeux.
    let état = app.assemble_state.as_ref().expect("chargé");
    let (adresse, ligne, avant) = état
        .disasm
        .iter()
        .find_map(|instruction| {
            let court = instruction
                .bytes
                .first()
                .is_some_and(|o| (0x70..=0x7f).contains(o));
            let ligne = état.addr_to_line.get(&instruction.address)?;
            court.then(|| (instruction.address, *ligne, instruction.bytes.clone()))
        })
        .expect("un crackme a bien un saut conditionnel");
    app.cursor_pos = (ligne as usize, 1);

    // Ce que le geste ferait, dit avant de cliquer.
    let (vu, origine, nouveaux) = app
        .patch_preview(PatchGeste::Inverser)
        .expect("l'inversion est possible sur cette ligne");
    assert_eq!(vu, adresse);
    assert_eq!(origine, avant);
    assert_eq!(nouveaux[0], avant[0] ^ 1, "un bit, et un seul");
    assert_eq!(nouveaux.len(), avant.len(), "la longueur ne bouge pas");

    // Appliqué : le binaire est rechargé avec les octets changés.
    app.apply_patch(PatchGeste::Inverser);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    assert_eq!(app.patches.len(), 1);

    let patché = &app.assemble_state.as_ref().expect("rechargé").output.file;
    let offset = app.patches[0].offset;
    assert_eq!(
        &patché[offset..offset + nouveaux.len()],
        nouveaux.as_slice(),
        "les octets changés sont dans l'image"
    );
    assert_ne!(patché[offset], octets[offset], "et ils ont bien changé");

    // Et le fichier exporté les porte : c'est tout l'intérêt.
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("patché");
    app.export_binary(&chemin);
    let écrit = std::fs::read(&chemin).expect("le binaire exporté");
    assert_eq!(écrit.len(), octets.len(), "même taille, à l'octet près");
    assert_eq!(&écrit[offset..offset + nouveaux.len()], nouveaux.as_slice());

    // Défait : on retrouve le fichier d'origine, octet pour octet.
    app.undo_patch(0);
    assert!(app.patches.is_empty());
    let rendu = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(
        &rendu[offset..offset + avant.len()],
        avant.as_slice(),
        "un patch qu'on ne peut pas défaire n'est pas un outil d'analyse"
    );
}

/// Neutraliser une instruction garde la longueur du fichier.
///
/// C'est la règle qui rend l'opération sûre : un octet de plus ou de
/// moins décalerait tout ce qui suit, et chaque adresse du fichier
/// deviendrait fausse.
#[test]
fn neutralising_an_instruction_keeps_every_other_address() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");

    let état = app.assemble_state.as_ref().expect("chargé");
    let (ligne, longueur) = état
        .disasm
        .iter()
        .find_map(|instruction| {
            let ligne = état.addr_to_line.get(&instruction.address)?;
            (instruction.bytes.len() >= 3).then_some((*ligne, instruction.bytes.len()))
        })
        .expect("une instruction d'au moins trois octets");
    app.cursor_pos = (ligne as usize, 1);

    app.apply_patch(PatchGeste::Neutraliser);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);

    let patché = app
        .assemble_state
        .as_ref()
        .expect("rechargé")
        .output
        .file
        .clone();
    assert_eq!(patché.len(), octets.len(), "le fichier garde sa taille");
    let offset = app.patches[0].offset;
    assert_eq!(
        &patché[offset..offset + longueur],
        vec![0x90u8; longueur].as_slice(),
        "autant de nop que d'octets remplacés"
    );
    // Et rien d'autre n'a bougé : c'est ce qui garantit que toutes les
    // autres adresses restent justes.
    let différents = patché
        .iter()
        .zip(octets.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(différents, longueur, "seuls ces octets-là ont changé");
}

/// La fenêtre des patchs dit ce qu'elle ferait avant qu'on clique.
#[test]
fn the_patch_window_says_what_it_would_do_before_one_clicks() {
    let s = strings(Lang::Fr);
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
    app.show_patches = true;

    let peint = paint(&mut app);
    assert!(peint.contains(s.patch_title), "{peint}");
    assert!(peint.contains(s.patch_invert), "{peint}");
    assert!(peint.contains(s.patch_neutralise), "{peint}");
    assert!(
        peint.contains(s.patch_none_yet),
        "et l'on n'a encore rien changé :\n{peint}"
    );
}

/// Deux textes d'une même fenêtre ne se recouvrent jamais.
///
/// Le défaut se voit tout de suite et se diagnostique mal : une commande
/// de cent cinquante caractères s'allouait la hauteur d'une ligne et
/// s'en peignait deux, si bien que tout ce qui suivait se posait une
/// ligne trop haut. De proche en proche, la fenêtre entière se
/// chevauchait — « illisible ».
///
/// La cause est une largeur de retour à la ligne indéterminée : sans
/// borne, la première passe de mise en page dispose sur une largeur
/// infinie, et le texte ne revient à la ligne qu'à la peinture.
#[test]
fn two_texts_of_a_window_never_overlap() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_decompiler: true,
        show_decompiler_module: true,
        // Une commande volontairement longue : c'est elle qui révélait
        // le défaut, et c'est celle du pont DZF.
        decompilers: vec![namsdec_disasm::plugin::Plugin {
            id: "dzf".to_owned(),
            name: "DZF — décompilation ELF64 (beta)".to_owned(),
            command: "/usr/bin/python3".to_owned(),
            args: vec![
                "/home/quelquun/Projects/DZF/integrations/namsdec/dzf_namsdec.py".to_owned(),
                "--cli".to_owned(),
                "/home/quelquun/Projects/DZF/build-dzf/x64-disassembler".to_owned(),
                "--input".to_owned(),
                "{file}".to_owned(),
                "--output".to_owned(),
                "{out}".to_owned(),
            ],
            install: None,
        }],
        decompiler: "dzf".to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1900.0, 1200.0),
        )),
        ..Default::default()
    };

    let mut textes: Vec<(String, egui::Rect)> = Vec::new();
    for _ in 0..3 {
        textes.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                textes.push((
                    t.galley.text().to_owned(),
                    egui::Rect::from_min_size(t.pos, t.galley.size()),
                ));
            }
        }
        sortie.textures_delta.clear();
    }

    let fenêtre = egui::AreaState::load(&ctx, egui::Id::new("decompiler_window"))
        .expect("la fenêtre est affichée")
        .rect();
    let dedans: Vec<&(String, egui::Rect)> = textes
        .iter()
        .filter(|(_, r)| fenêtre.contains_rect(r.shrink(1.0)))
        .collect();
    assert!(dedans.len() > 4, "la fenêtre écrit bien : {}", dedans.len());

    for (i, (texte_a, a)) in dedans.iter().enumerate() {
        for (texte_b, b) in &dedans[i + 1..] {
            // Deux pixels de tolérance : les jambages se frôlent d'une
            // ligne à l'autre sans se gêner.
            let chevauche = a.shrink(2.0).intersects(b.shrink(2.0));
            assert!(
                !chevauche,
                "« {texte_a} » et « {texte_b} » se recouvrent : {a:?} et {b:?}"
            );
        }
    }
}
/// Deux textes d'une même scène ne se recouvrent jamais.
///
/// Le défaut le plus visible qui soit, et le plus mal diagnostiqué : on
/// voit « une fenêtre illisible » et l'on cherche la couleur, la
/// police, l'ascenseur. Mesuré, il se dit en une ligne — tel texte et
/// tel autre occupent le même endroit. C'était le cas du décompilateur,
/// dont une commande longue s'allouait la hauteur d'une ligne et s'en
/// peignait deux ; la règle vaut partout, parce que rien dans egui ne
/// l'empêche nulle part.
#[test]
fn no_two_texts_of_a_scene_ever_overlap() {
    for (nom, id, prépare) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        prépare(&mut app);
        let (_, textes) = ce_qui_est_écrit(&ctx, &mut app, id, 3);
        assert!(
            textes.len() > 2,
            "« {nom} » n'écrit presque rien : le test ne prouverait rien"
        );
        for (i, (texte_a, a)) in textes.iter().enumerate() {
            for (texte_b, b) in &textes[i + 1..] {
                // Deux pixels de tolérance : les jambages d'une ligne
                // frôlent les hampes de la suivante sans se gêner.
                assert!(
                    !a.shrink(2.0).intersects(b.shrink(2.0)),
                    "« {nom} » : « {texte_a} » et « {texte_b} » se recouvrent\n  {a:?}\n  {b:?}"
                );
            }
        }
    }
}

/// Aucune fenêtre ne porte de filet horizontal sous son titre ni
/// au-dessus de son bouton.
///
/// Une fenêtre qui en porte deux se lit comme un formulaire : trois
/// bandes empilées. C'est l'espace qui sépare, et il sépare aussi bien.
#[test]
fn a_window_separates_with_space_not_with_bars() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_calculator: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };

    let fenêtre = {
        let mut rect = None;
        for _ in 0..3 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            sortie.textures_delta.clear();
            rect = egui::AreaState::load(&ctx, egui::Id::new("calculator_window"))
                .map(|état| état.rect());
        }
        rect.expect("la calculatrice est affichée")
    };

    // Un filet, c'est un rectangle d'un pixel de haut sur presque toute
    // la largeur de la fenêtre. On les compte.
    let mut filets = Vec::new();
    let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
    for clipped in &sortie.shapes {
        if let egui::epaint::Shape::Rect(cadre) = &clipped.shape
            && cadre.rect.height() <= 2.0
            && cadre.rect.width() > fenêtre.width() * 0.6
            && fenêtre.contains(cadre.rect.center())
        {
            filets.push(cadre.rect);
        }
    }
    sortie.textures_delta.clear();
    assert!(
        filets.is_empty(),
        "la fenêtre ne doit porter aucun filet horizontal : {filets:?}"
    );
}

/// La calculatrice ne laisse pas de vide sous sa dernière ligne.
///
/// Une hauteur figée laissait cent pixels entre la dernière ligne et le
/// bouton : un appareil ne se termine pas par du vide. La fenêtre
/// épouse la hauteur de son contenu.
#[test]
fn the_calculator_ends_where_its_content_ends() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_calculator: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };
    let mut bas_du_texte = f32::MIN;
    for _ in 0..3 {
        bas_du_texte = f32::MIN;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                // Ce que la calculatrice écrit, et rien d'autre : le
                // reste de la fenêtre principale est ailleurs.
                let dans = t.pos.x > 400.0 && t.pos.x < 1200.0;
                if dans {
                    bas_du_texte = bas_du_texte.max(t.pos.y + t.galley.size().y);
                }
            }
        }
        sortie.textures_delta.clear();
    }

    let fenêtre = egui::AreaState::load(&ctx, egui::Id::new("calculator_window"))
        .expect("la calculatrice est affichée")
        .rect();
    let vide = fenêtre.bottom() - bas_du_texte;
    assert!(
        vide < 40.0,
        "la fenêtre traîne {vide} pixels de vide sous sa dernière ligne"
    );
}

/// L'expression se tape dans l'écran, au-dessus du résultat.
///
/// Un champ de saisie encadré, posé au-dessus avec son étiquette
/// « Expression », faisait un formulaire. Sur une calculatrice, la ligne
/// du haut de l'écran *est* le champ : c'est là que les chiffres
/// arrivent, et le résultat s'écrit dessous.
#[test]
fn the_expression_is_typed_inside_the_screen_above_the_result() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_calculator: true,
        calc_input: "0xff & 0x0f".to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };

    let mut textes: Vec<(String, egui::Rect)> = Vec::new();
    let mut écran: Option<egui::Rect> = None;
    for _ in 0..3 {
        textes.clear();
        écran = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(t) => {
                    textes.push((
                        t.galley.text().to_owned(),
                        egui::Rect::from_min_size(t.pos, t.galley.size()),
                    ));
                }
                // Le cadre de l'écran : la seule forme à cette largeur.
                // Deux pixels de tolérance, parce que le liseré déborde
                // du rectangle qu'on lui a donné.
                egui::epaint::Shape::Rect(cadre)
                    if (cadre.rect.width() - (CALC_LARGEUR + 20.0)).abs() < 3.0 =>
                {
                    écran = Some(cadre.rect);
                }
                _ => {}
            }
        }
        sortie.textures_delta.clear();
    }
    let écran = écran.expect("l'écran a son cadre");

    let où = |quoi: &str| -> egui::Rect {
        textes
            .iter()
            .find(|(t, _)| t == quoi)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("« {quoi} » doit être peint"))
    };
    let expression = où("0xff & 0x0f");
    // `0xff & 0x0f` vaut 15, écrit en hexadécimal comme le veut la base
    // par défaut.
    let résultat = où("0xf");

    assert!(
        écran.contains_rect(expression.shrink(1.0)),
        "l'expression est dans l'écran : {expression:?} dans {écran:?}"
    );
    assert!(
        écran.contains_rect(résultat.shrink(1.0)),
        "et le résultat aussi : {résultat:?}"
    );
    assert!(
        expression.bottom() <= résultat.top() + 1.0,
        "l'expression est au-dessus du résultat : {expression:?} puis {résultat:?}"
    );
    // En grand, et pas à la taille d'un champ ordinaire.
    assert!(
        expression.height() > 20.0,
        "l'expression se lit en grand : {} de haut",
        expression.height()
    );
    // Les deux calées à gauche, au même bord : on écrit une expression,
    // qui se lit de gauche à droite comme la ligne de code d'où elle
    // sort.
    assert!(
        (expression.left() - résultat.left()).abs() < 4.0,
        "les deux lignes partagent un bord gauche : {expression:?} et {résultat:?}"
    );
}

/// Les touches du clavier sont de vrais boutons, et le bloc est centré.
///
/// Le thème rend le fond des boutons transparent au repos et leur liseré
/// invisible : sans fond posé à la main, le clavier n'était qu'une
/// trentaine de caractères flottant dans du vide, qu'on ne pensait même
/// pas à cliquer. Et aligné à gauche dans une fenêtre large, il tombait
/// contre un bord avec du vide en face — une page, et non un appareil.
#[test]
fn the_keypad_is_made_of_real_buttons_and_is_centred() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_calculator: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };

    // Les touches : des rectangles à la taille posée, et rien d'autre
    // dans la fenêtre n'a cette taille-là.
    let mut touches: Vec<(egui::Rect, egui::Color32)> = Vec::new();
    for _ in 0..3 {
        touches.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Rect(cadre) = &clipped.shape
                && (cadre.rect.width() - TOUCHE_L).abs() < 0.5
                && (cadre.rect.height() - TOUCHE_H).abs() < 0.5
            {
                touches.push((cadre.rect, cadre.fill));
            }
        }
        sortie.textures_delta.clear();
    }

    assert_eq!(touches.len(), 30, "cinq rangées de six touches");
    // La teinte rendue à son opacité pleine : egui fait apparaître une
    // fenêtre en fondu, et ses formes arrivent en alpha prémultiplié
    // tant que le fondu dure. On juge la couleur, non l'avancement
    // d'une animation.
    let teinte = |c: egui::Color32| -> [i32; 3] {
        let a = i32::from(c.a()).max(1);
        [
            i32::from(c.r()) * 255 / a,
            i32::from(c.g()) * 255 / a,
            i32::from(c.b()) * 255 / a,
        ]
    };
    let fond_panneau = teinte(app.theme.surface);
    for (rect, fond) in &touches {
        assert!(fond.a() > 0, "une touche invisible : {rect:?}");
        // Assez loin du fond pour se voir : c'est ce qui manquait quand
        // les touches étaient peintes à vingt pour cent d'opacité —
        // elles prenaient la couleur du panneau, à peine plus claires,
        // et le clavier n'avait plus l'air d'un clavier.
        let couleur = teinte(*fond);
        let distance: i32 = couleur
            .iter()
            .zip(fond_panneau.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(
            distance >= 60,
            "une touche doit se détacher franchement du panneau : {couleur:?} contre \
             {fond_panneau:?}"
        );
    }

    // Trois familles, trois teintes : les chiffres, les opérateurs et ce
    // qui efface ne se ressemblent pas. Des mélanges timides donnaient
    // six nuances du même bleu sombre, où il fallait lire chaque touche
    // pour la trouver.
    let familles: std::collections::BTreeSet<[i32; 3]> =
        touches.iter().map(|(_, fond)| teinte(*fond)).collect();
    assert!(
        familles.len() >= 2,
        "les chiffres et les opérateurs n'ont pas la même couleur : {familles:?}"
    );
    let écart = |a: &[i32; 3], b: &[i32; 3]| -> i32 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
    };
    let familles: Vec<[i32; 3]> = familles.into_iter().collect();
    let plus_proches = familles
        .iter()
        .enumerate()
        .flat_map(|(i, a)| familles[i + 1..].iter().map(move |b| écart(a, b)))
        .min()
        .unwrap_or(0);
    assert!(
        plus_proches >= 40,
        "deux familles doivent se distinguer d'un coup d'œil : {familles:?}"
    );

    // Centré dans la fenêtre : le milieu du bloc et celui de la fenêtre
    // tombent au même endroit.
    let fenêtre = egui::AreaState::load(&ctx, egui::Id::new("calculator_window"))
        .expect("la calculatrice est affichée");
    let gauche = touches
        .iter()
        .map(|(r, _)| r.left())
        .fold(f32::MAX, f32::min);
    let droite = touches
        .iter()
        .map(|(r, _)| r.right())
        .fold(f32::MIN, f32::max);
    let milieu_du_bloc = (gauche + droite) / 2.0;
    let milieu_fenêtre = fenêtre.rect().center().x;
    assert!(
        (milieu_du_bloc - milieu_fenêtre).abs() < 12.0,
        "le clavier doit être centré : {milieu_du_bloc} contre {milieu_fenêtre}"
    );

    // La grille de bits est centrée elle aussi : ses deux index
    // encadrent la rangée, et leur milieu tombe au milieu de la fenêtre.
    let mut index: std::collections::BTreeMap<String, (f32, f32)> =
        std::collections::BTreeMap::new();
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    for clipped in &sortie.shapes {
        if let egui::epaint::Shape::Text(texte) = &clipped.shape {
            let mot = texte.galley.text();
            if ["63", "32"].contains(&mot) {
                index.insert(
                    mot.to_owned(),
                    (texte.pos.x, texte.pos.x + texte.galley.size().x),
                );
            }
        }
    }
    sortie.textures_delta.clear();
    let (gauche_index, _) = index["63"];
    let (_, droite_index) = index["32"];
    assert!(
        ((gauche_index + droite_index) / 2.0 - milieu_fenêtre).abs() < 16.0,
        "la grille de bits doit être centrée : {gauche_index}..{droite_index}"
    );

    // Six colonnes : les touches d'une rangée se suivent sans se
    // chevaucher, et chaque rangée en compte six.
    let mut par_rangée: std::collections::BTreeMap<i32, usize> = std::collections::BTreeMap::new();
    for (rect, _) in &touches {
        *par_rangée.entry(rect.top() as i32).or_default() += 1;
    }
    assert_eq!(par_rangée.len(), 5, "cinq rangées : {par_rangée:?}");
    for (haut, combien) in &par_rangée {
        assert_eq!(*combien, 6, "six touches à la hauteur {haut}");
    }
}

/// Le clavier écrit ce que l'évaluateur lit.
///
/// C'est toute la règle : une touche qui produirait une expression que
/// la calculatrice refuse serait une touche qui ment. En hexadécimal,
/// taper 1 puis F doit donner trente et un — non mille neuf cent.
#[test]
fn the_keypad_writes_what_the_evaluator_reads() {
    // Une frappe après l'autre, comme au clavier de l'écran.
    let tape = |base: CalcBase, touches: &[&str]| -> String {
        let mut expression = String::new();
        for touche in touches {
            expression.push_str(&écriture_de_touche(touche, base, &expression));
        }
        expression
    };

    // Le préfixe arrive avec le premier chiffre, et une seule fois.
    assert_eq!(tape(CalcBase::Hex, &["1", "F"]), "0x1F");
    assert_eq!(calc::eval("0x1F", None), Ok(31));
    assert_eq!(tape(CalcBase::Bin, &["1", "1", "1", "1", "1"]), "0b11111");
    assert_eq!(calc::eval("0b11111", None), Ok(31));
    assert_eq!(tape(CalcBase::Oct, &["3", "7"]), "0o37");
    // Le décimal n'a pas de préfixe : c'est le défaut de tout le monde.
    assert_eq!(tape(CalcBase::Dec, &["3", "1"]), "31");

    // Après un opérateur, un nouveau nombre commence, donc un nouveau
    // préfixe.
    assert_eq!(tape(CalcBase::Hex, &["1", "F", "&", "F"]), "0x1F&0xF");
    assert_eq!(calc::eval("0x1F&0xF", None), Ok(15));

    // Les opérateurs et les parenthèses s'écrivent tels quels.
    assert_eq!(
        tape(CalcBase::Hex, &["(", "1", ")", "<<", "4"]),
        "(0x1)<<0x4"
    );
    assert_eq!(calc::eval("(0x1)<<0x4", None), Ok(16));
}

/// Un chiffre n'est proposé que s'il existe dans la base.
///
/// Un clavier qui propose `9` en binaire ment sur ce qu'on peut taper,
/// et l'on apprend l'erreur en la commettant.
#[test]
fn a_digit_is_offered_only_where_it_exists() {
    assert!(CalcBase::Hex.accepte('f') && CalcBase::Hex.accepte('F'));
    assert!(CalcBase::Dec.accepte('9') && !CalcBase::Dec.accepte('A'));
    assert!(CalcBase::Oct.accepte('7') && !CalcBase::Oct.accepte('8'));
    assert!(CalcBase::Bin.accepte('1') && !CalcBase::Bin.accepte('2'));
}

/// La largeur masque l'affichage, elle ne change pas le calcul.
///
/// Le processeur travaille sur soixante-quatre bits quoi qu'on affiche ;
/// ce qu'on choisit ici, c'est combien de bits on veut *lire*. Un `and
/// eax, 0x1f` travaille sur trente-deux, et lire son résultat sur
/// soixante-quatre fait compter des zéros qui ne veulent rien dire.
#[test]
fn the_width_masks_the_reading_not_the_computation() {
    assert_eq!(CalcWidth::W64.mask(), u64::MAX, "et non un débordement");
    assert_eq!(CalcWidth::W32.mask(), 0xffff_ffff);
    assert_eq!(CalcWidth::W16.mask(), 0xffff);
    assert_eq!(CalcWidth::W8.mask(), 0xff);

    let valeur = calc::eval("0 - 1", None).expect("le calcul passe");
    assert_eq!(valeur, u64::MAX, "le calcul déborde comme un registre");
    assert_eq!(
        valeur & CalcWidth::W8.mask(),
        0xff,
        "et la lecture se borne"
    );
}

/// La base et la largeur se retrouvent au retour.
///
/// Ce sont des réglages : quelqu'un qui travaille en hexadécimal sur
/// trente-deux bits ne veut pas le redire à chaque lancement.
#[test]
fn the_calculator_remembers_its_base_and_width() {
    let mut app = NamsdecApp {
        calc_base: CalcBase::Bin,
        calc_width: CalcWidth::W16,
        ..Default::default()
    };
    let écrits = app.settings_snapshot();
    assert_eq!(écrits.calc_base, "bin");
    assert_eq!(écrits.calc_width, "16");

    app = NamsdecApp::from_settings(écrits);
    assert_eq!(app.calc_base, CalcBase::Bin);
    assert_eq!(app.calc_width, CalcWidth::W16);

    // Un fichier qui ne les connaît pas retombe sur l'hexadécimal et
    // soixante-quatre bits : la base d'un désassemblage, et la largeur
    // de la machine simulée.
    assert_eq!(CalcBase::by_id("n'importe quoi"), CalcBase::Hex);
    assert_eq!(CalcWidth::by_id("n'importe quoi"), CalcWidth::W64);
}

/// La calculatrice montre son clavier, ses bits et ses fonctions.
#[test]
fn the_calculator_shows_its_keypad_its_bits_and_its_functions() {
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        show_calculator: true,
        calc_input: "0x1f".to_owned(),
        ..Default::default()
    };
    let peint = paint(&mut app);

    // Le résultat, dans la base choisie.
    assert!(peint.contains("0x1f"), "{peint}");
    // Le clavier : des chiffres et des opérateurs de code, non des
    // symboles mathématiques qu'il faudrait retraduire.
    for touche in ["7", "F", "&", "<<", "0x"] {
        assert!(
            peint.contains(&format!("{touche}|")),
            "la touche {touche} :\n{peint}"
        );
    }
    // Les fonctions, en toutes lettres.
    assert!(peint.contains("popcnt(x)"), "{peint}");
    assert!(peint.contains("rol(x, n)"), "{peint}");
    // Et la ligne qui invite à les survoler.
    assert!(peint.contains(s.calculator_pick_function), "{peint}");
    // Les index de la grille de bits.
    assert!(peint.contains("63|"), "{peint}");
    assert!(peint.contains("32|"), "{peint}");
}

/// Une fenêtre qui vient de s'ouvrir passe devant.
///
/// Le plan d'installation s'ouvre depuis les préférences, et se posait
/// derrière elles : on cliquait « Installer… » et il ne se passait rien
/// de visible. egui empile les zones dans l'ordre où on les touche, et
/// une fenêtre qu'on vient de créer n'a jamais été touchée.
///
/// La règle vaut pour toutes, d'où une table — et ce test dit qu'aucune
/// fenêtre ne manque à cette table. Sans lui, ajouter une fenêtre
/// réintroduirait le défaut sans qu'on le voie.
#[test]
fn every_window_is_in_the_table_that_raises_new_ones() {
    // TOUS les fichiers : les fenêtres ont quitté `main.rs` module par
    // module, et ce contrôle s'est vidé en silence. Seul le garde-fou
    // « le fichier a bien des fenêtres », en bas, a rougi — et c'est
    // pour cela qu'il est là.
    let tout: String = SOURCES_DE_L_INTERFACE
        .iter()
        .map(|(_, s)| *s)
        .collect::<Vec<_>>()
        .join("\n");
    let connus: std::collections::BTreeSet<&str> = NamsdecApp::default()
        .window_states()
        .into_iter()
        .map(|(nom, _)| nom)
        .collect();

    // Chaque `egui::Window::new(...)` du fichier, et l'identifiant qui
    // le suit : il est toujours dans les lignes qui suivent
    // immédiatement, parce que c'est le premier appel de la chaîne.
    let mut vus = std::collections::BTreeSet::new();
    for (position, _) in tout.match_indices("egui::Window::new(") {
        let suite = &tout[position..(position + 600).min(tout.len())];
        // Certaines fenêtres nomment leur identifiant par une constante,
        // parce que plusieurs endroits s'y réfèrent ; le reste l'écrit
        // sur place. On résout la constante en relisant sa déclaration,
        // plutôt qu'en tenant ici une liste qu'il faudrait allonger à
        // chaque fenêtre — et qu'on oublierait.
        let id = suite
            .find("egui::Id::new(\"")
            .map(|i| &suite[i + "egui::Id::new(\"".len()..])
            .and_then(|reste| reste.find('"').map(|fin| &reste[..fin]))
            .or_else(|| {
                let i = suite.find("egui::Id::new(")? + "egui::Id::new(".len();
                let nom = &suite[i..i + suite[i..].find(')')?];
                if !nom.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                    return None;
                }
                let déclaration = format!("const {nom}: &str = \"");
                let j = tout.find(&déclaration)? + déclaration.len();
                let reste = &tout[j..];
                reste.find('"').map(|fin| &reste[..fin])
            })
            .unwrap_or_else(|| {
                panic!(
                    "une fenêtre sans identifiant explicite — elle perdrait sa \
                     position en changeant de langue :\n{}",
                    &suite[..120.min(suite.len())]
                )
            });
        vus.insert(id);
    }

    // LES DEUX SENS, et non plus un seul.
    //
    // Le contrôle ne demandait que « toute fenêtre dessinée est dans la
    // table ». Cette moitié-là passe au vert dès qu'on ne dessine plus
    // rien — ce qui est exactement arrivé : `main.rs` a perdu sa
    // dernière fenêtre en même temps que `unsaved_window_ui`, et seul
    // un garde-fou « le fichier a bien des fenêtres » a rougi.
    //
    // L'autre sens — « toute fenêtre de la table est dessinée quelque
    // part » — ferme le trou pour de bon : il ne peut pas être satisfait
    // en ne regardant rien.
    let manquants: Vec<&&str> = vus.difference(&connus).collect();
    assert!(
        manquants.is_empty(),
        "ces fenêtres ne sont pas dans window_states : {manquants:?}"
    );
    let jamais_peintes: Vec<&&str> = connus.difference(&vus).collect();
    assert!(
        jamais_peintes.is_empty(),
        "ces fenêtres sont dans window_states et ne sont peintes nulle \
         part — ou le relevé a cessé de les voir : {jamais_peintes:?}"
    );
}

/// Et la règle s'applique : ouvrir une fenêtre la met devant.
#[test]
fn opening_a_window_puts_it_in_front() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };

    // Les préférences d'abord, puis la calculatrice par-dessus : c'est
    // l'ordre dans lequel on ouvre la seconde depuis la première.
    app.show_preferences = true;
    for _ in 0..2 {
        ctx.run_ui(entrée.clone(), |ui| app.draw(ui))
            .textures_delta
            .clear();
    }
    app.show_calculator = true;
    ctx.run_ui(entrée.clone(), |ui| app.draw(ui))
        .textures_delta
        .clear();

    let rang = |nom: &str| -> usize {
        ctx.memory(|m| {
            m.layer_ids()
                .position(|id| id == egui::LayerId::new(egui::Order::Middle, egui::Id::new(nom)))
                .unwrap_or_else(|| panic!("la fenêtre « {nom} » est dessinée"))
        })
    };
    assert!(
        rang("calculator_window") > rang("preferences_window"),
        "la dernière ouverte est au-dessus"
    );
}

/// Une frame lente ne dit pas la même chose selon ce qui l'a ralentie.
///
/// Ouvrir un binaire de soixante mille lignes demande une demi-seconde
/// de mise en page — **une fois**. Le cache porte ensuite, et les frames
/// suivantes tiennent en deux millisecondes. Crier « frame lente » sans
/// le dire faisait chercher une fuite là où il n'y avait qu'un
/// chargement.
#[test]
fn a_slow_frame_says_whether_it_will_come_back() {
    use std::time::Duration;

    let étapes = [
        ("éditeur", Duration::from_millis(440)),
        ("panneaux", Duration::from_millis(2)),
    ];
    let vite = Duration::from_millis(3);
    let lente = Duration::from_millis(450);

    // En marche normale, rien : l'instrumentation est silencieuse.
    assert_eq!(slow_frame_note(vite, &étapes, false), None);
    assert_eq!(slow_frame_note(vite, &étapes, true), None);

    // Lente, et le texte vient de changer : c'est une mise en page.
    let note = slow_frame_note(lente, &étapes, true).expect("il y a à dire");
    assert!(note.contains("mise en page"), "{note}");
    assert!(note.contains("payé une fois"), "{note}");
    assert!(note.contains("éditeur 440ms"), "{note}");

    // Lente sans que rien n'ait changé : là, c'est un vrai reproche.
    let note = slow_frame_note(lente, &étapes, false).expect("il y a à dire");
    assert!(note.contains("frame lente"), "{note}");
    assert!(!note.contains("payé une fois"), "{note}");

    // Les étapes trop courtes ne noient pas le message, et l'absence de
    // toute étape mesurée se dit plutôt que de laisser une parenthèse
    // vide.
    let note = slow_frame_note(lente, &[], false).expect("il y a à dire");
    assert!(note.contains("hors des étapes mesurées"), "{note}");
}

/// Chercher une suite d'octets dans la mémoire du programme.
///
/// C'est le premier réflexe devant un programme qui compare quelque
/// chose : on a lu une chaîne, on veut savoir où elle est écrite, et
/// c'est de là qu'on remonte à ce qui la lit.
#[test]
fn a_byte_pattern_is_found_in_the_running_memory() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
mot db \"HVUHADN\", 0
section .text
_start:
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let mem = &app.emulator.as_ref().expect("un émulateur").cpu().mem;

    // L'adresse que l'assembleur a donnée à l'étiquette : c'est elle
    // qu'on doit retrouver, et non une adresse voisine.
    let attendue = app
        .assemble_state
        .as_ref()
        .expect("assemblé")
        .output
        .symbols
        .iter()
        .find(|s| s.name == "mot")
        .map(|s| s.addr)
        .expect("l'étiquette « mot » existe");

    // Le texte.
    let trouvés = sidebar::search_memory(mem, &memfind::parse("\"HVUHADN\"").unwrap());
    assert!(
        trouvés.contains(&attendue),
        "« HVUHADN » doit être en {attendue:#x} : {trouvés:x?}"
    );

    // Les mêmes octets en hexadécimal, et avec un joker au milieu.
    for écriture in ["48 56 55 48", "48 56 ?? 48", "\"HV\" ?? 48"] {
        let trouvés = sidebar::search_memory(mem, &memfind::parse(écriture).unwrap());
        assert!(
            trouvés.contains(&attendue),
            "« {écriture} » doit trouver {attendue:#x} : {trouvés:x?}"
        );
    }

    // Ce qui n'y est pas ne se trouve pas — et ne fait rien tomber.
    let trouvés = sidebar::search_memory(mem, &memfind::parse("\"PASDEDANS\"").unwrap());
    assert!(trouvés.is_empty(), "{trouvés:x?}");
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

/// Le nom du fichier suit les menus, il ne flotte pas au centre.
///
/// Centré dans la place qui reste, il se déplaçait à chaque panneau
/// ouvert et à chaque redimensionnement de la fenêtre. Un repère qui
/// bouge n'en est pas un : on le cherche au lieu de le voir.
#[test]
fn the_file_name_follows_the_menus_instead_of_floating() {
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(
        !app.system_titlebar,
        "la barre maison, celle qui porte le nom"
    );

    let large = egui::vec2(1600.0, 1000.0);
    let étroite = egui::vec2(1100.0, 1000.0);

    // Les textes de la barre de titre : les quarante premiers pixels.
    let barre = |app: &mut NamsdecApp, taille: egui::Vec2| -> Vec<(String, f32, f32)> {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, taille)),
            ..Default::default()
        };
        let mut vus = Vec::new();
        for _ in 0..2 {
            vus.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape
                    && texte.pos.y < 40.0
                {
                    vus.push((
                        texte.galley.text().to_owned(),
                        texte.pos.x,
                        texte.pos.x + texte.galley.size().x,
                    ));
                }
            }
            sortie.textures_delta.clear();
        }
        vus
    };

    let position = |vus: &[(String, f32, f32)], quoi: &str| -> (f32, f32) {
        vus.iter()
            .find(|(t, _, _)| t == quoi)
            .map(|(_, g, d)| (*g, *d))
            .unwrap_or_else(|| panic!("« {quoi} » n'est pas dans la barre : {vus:?}"))
    };

    let vus = barre(&mut app, large);
    let (_, fin_du_menu) = position(&vus, s.menu_help);
    let (début_du_nom, _) = position(&vus, s.untitled_buffer);

    assert!(
        début_du_nom > fin_du_menu,
        "le nom vient après « {} » : {fin_du_menu} puis {début_du_nom}",
        s.menu_help
    );
    // Quelques tabulations, pas un collage : « Aide sans titre » se
    // lirait comme un sixième menu.
    let écart = début_du_nom - fin_du_menu;
    assert!(
        (12.0..120.0).contains(&écart),
        "l'écart après le menu vaut {écart}"
    );
    // Et loin du milieu, qui est là où il était.
    assert!(
        début_du_nom < large.x * 0.4,
        "le nom est à {début_du_nom} pour une fenêtre de {}",
        large.x
    );

    // Rétrécir la fenêtre ne le déplace pas : c'est tout le point.
    let vus = barre(&mut app, étroite);
    let (après, _) = position(&vus, s.untitled_buffer);
    assert!(
        (après - début_du_nom).abs() < 0.5,
        "le nom a bougé de {} en changeant la largeur",
        après - début_du_nom
    );
}

/// La calculatrice se prend juste à gauche de l'onglet Pile.
///
/// C'est là qu'on s'en sert : on lit une valeur dans la pile ou dans un
/// registre, et l'on veut la même dans une autre base. Elle était dans
/// le menu Outils, à trois gestes de l'endroit où le besoin naît.
#[test]
fn the_calculator_sits_just_left_of_the_stack_tab() {
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);

    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };
    let mut touches = Vec::new();
    let mut pile: Option<f32> = None;
    for _ in 0..2 {
        touches.clear();
        pile = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            match &clipped.shape {
                // Les six touches de l'icône : des points, les seuls
                // cercles aussi petits de la barre d'outils.
                // La barre d'outils occupe la deuxième rangée : « Pile »
                // est aussi le titre du panneau, plus bas, et c'est le
                // titre qu'on attrapait.
                egui::epaint::Shape::Circle(cercle)
                    if cercle.radius < 1.5 && cercle.center.y < 60.0 =>
                {
                    touches.push(cercle.center);
                }
                egui::epaint::Shape::Text(texte)
                    if texte.galley.text() == s.tab_stack && texte.pos.y < 60.0 =>
                {
                    pile = Some(texte.pos.x);
                }
                _ => {}
            }
        }
        sortie.textures_delta.clear();
    }

    let pile = pile.expect("l'onglet Pile est peint");
    assert_eq!(touches.len(), 6, "les six touches de la calculatrice");
    for point in &touches {
        assert!(
            point.x < pile,
            "la calculatrice est à gauche de « {} » : {} contre {pile}",
            s.tab_stack,
            point.x
        );
    }
    // Dans la barre d'outils, non perdue ailleurs dans la fenêtre.
    let haut = touches.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    assert!(
        haut > 30.0,
        "dans la barre d'outils, non dans les menus : {haut}"
    );
}

/// L'onglet RE ne porte plus qu'un bouton.
///
/// Les sept modules empilés dessous faisaient deux mille pixels à
/// parcourir dans une colonne de trois cents. La fenêtre d'analyse
/// montre exactement les mêmes, en grand et tout dépliés : c'est là
/// qu'on les lit.
#[test]
fn the_re_tab_carries_only_its_button() {
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        show_re_tab: true,
        active_tab: SidebarTab::Re,
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.analysis_open),
        "le bouton est là :\n{peint}"
    );
    for module in [
        "En bref",
        "Fonctions importées",
        "Fragments réutilisables (ROP)",
    ] {
        assert!(
            !peint.contains(module),
            "« {module} » n'a plus sa place dans le panneau :\n{peint}"
        );
    }
}

/// La correction proposée s'écrit dans la barre d'état, et Alt la pose.
///
/// Deux choses à vérifier, et la seconde compte autant que la première :
/// que namsdec **propose** quand il y a lieu, et qu'il n'écrit rien
/// avant qu'on le lui dise. Le code de quelqu'un ne se corrige pas tout
/// seul.
#[test]
fn a_proposed_fix_is_written_in_the_status_bar_and_alt_applies_it() {
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: "    mvo rax, 1\n".to_owned(),
        // Le curseur est sur la première ligne, comme au chargement.
        cursor_pos: (1, 1),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));

    let peindre = |app: &mut NamsdecApp, alt: bool| -> String {
        // `Alt` seul n'est pas une touche pour egui : il arrive par
        // l'évènement qui annonce un changement de modificateurs.
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            events: vec![egui::Event::ModifiersChanged(egui::Modifiers {
                alt,
                ..Default::default()
            })],
            ..Default::default()
        };
        let mut peint = String::new();
        let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                peint.push_str(texte.galley.text());
                peint.push('|');
            }
        }
        sortie.textures_delta.clear();
        peint
    };

    // Deux frames : la barre d'état est peinte avant l'éditeur, qui est
    // ce qui calcule la proposition.
    peindre(&mut app, false);
    let peint = peindre(&mut app, false);
    let attendu = s.fix_hint.replacen(
        "{}",
        &s.fix_unknown_mnemonic
            .replacen("{}", "mvo", 1)
            .replacen("{}", "mov", 1),
        1,
    );
    assert!(
        peint.contains(&attendu),
        "attendu « {attendu} » dans :\n{peint}"
    );
    assert_eq!(
        app.source, "    mvo rax, 1\n",
        "rien n'est écrit tant qu'on n'a pas confirmé"
    );

    // Alt confirme.
    peindre(&mut app, true);
    assert_eq!(app.source, "    mov rax, 1\n", "Alt pose la correction");

    // Et la ligne corrigée ne propose plus rien.
    peindre(&mut app, false);
    let peint = peindre(&mut app, false);
    assert!(
        !peint.contains("n'existe pas"),
        "plus rien à proposer :\n{peint}"
    );
}

/// Garder Alt enfoncé ne corrige qu'une fois.
///
/// egui n'émet pas d'évènement pour une touche de modification seule :
/// elle n'apparaît que dans l'état des modificateurs, vrai à chaque
/// frame tant qu'on appuie. C'est le *passage* de relâché à enfoncé
/// qu'on guette — sans quoi une seconde d'Alt corrigerait tout ce qui
/// passe.
///
/// La ligne choisie le montre bien : une fois le mnémonique réparé, il
/// reste une seconde faute sur la même ligne. Si l'appui tenu comptait
/// pour plusieurs, elle partirait dans la foulée, sans qu'on l'ait lue.
#[test]
fn holding_alt_does_not_correct_twice() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: "    mvo [compteur], 1\n".to_owned(),
        cursor_pos: (1, 1),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
    let frame = |app: &mut NamsdecApp, alt: bool| {
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            events: vec![egui::Event::ModifiersChanged(egui::Modifiers {
                alt,
                ..Default::default()
            })],
            ..Default::default()
        };
        ctx.run_ui(entrée, |ui| app.draw(ui)).textures_delta.clear();
    };

    frame(&mut app, false);
    frame(&mut app, false);
    frame(&mut app, true);
    assert_eq!(
        app.source, "    mov [compteur], 1\n",
        "le mnémonique d'abord"
    );

    // Toujours enfoncé : la seconde faute reste, et c'est le but.
    frame(&mut app, true);
    frame(&mut app, true);
    assert_eq!(
        app.source, "    mov [compteur], 1\n",
        "Alt maintenu ne corrige pas une seconde fois"
    );

    // Relâché puis réenfoncé : la taille de l'écriture, qui n'était pas
    // dite.
    frame(&mut app, false);
    frame(&mut app, true);
    assert_eq!(app.source, "    mov qword [compteur], 1\n");
}

/// Les fenêtres internes se ferment par la même pastille que la grande.
///
/// egui pose sa croix à lui : deux traits maigres, à droite, de la
/// couleur du texte. Dans une application dont la fenêtre principale se
/// ferme par un rond rouge posé à gauche, cela faisait deux grammaires
/// pour le même geste — et celle qu'on voit le moins fermait le plus de
/// choses. Ce test dit qu'il n'en reste qu'une, et qu'elle est bien la
/// même.
#[test]
fn every_inner_window_closes_with_the_same_pastille() {
    /// La teinte d'une couleur peinte, rendue à son opacité pleine.
    ///
    /// egui fait apparaître une fenêtre en fondu : tant que le fondu
    /// dure, ses formes arrivent en alpha prémultiplié, et le rouge de
    /// la pastille se présente comme un rouge sombre. On divise par
    /// l'alpha pour retrouver la teinte, qui est ce qu'on vérifie — la
    /// couleur, non l'avancement d'une animation.
    fn teinte(c: egui::Color32) -> [i32; 3] {
        let a = i32::from(c.a()).max(1);
        [
            i32::from(c.r()) * 255 / a,
            i32::from(c.g()) * 255 / a,
            i32::from(c.b()) * 255 / a,
        ]
    }
    let attendue = teinte(chrome::WindowButton::Close.colour());

    // Les fenêtres qu'un démarrage nu sait ouvrir, chacune seule : deux
    // à la fois se recouvrent, et l'on ne saurait plus laquelle porte la
    // pastille vue.
    /// Comment ouvrir une fenêtre, et comment la nommer dans l'échec.
    type Ouverture = (&'static str, fn(&mut NamsdecApp));
    let fenêtres: Vec<Ouverture> = vec![
        ("préférences", |app| app.show_preferences = true),
        ("à propos", |app| app.show_about = true),
        ("calculatrice", |app| app.show_calculator = true),
        ("aide", |app| app.show_help = true),
        ("registres étendus", |app| app.show_vectors = true),
        ("points d'arrêt", |app| app.show_breakpoints = true),
        ("chronologie", |app| app.show_timeline = true),
        ("motif cyclique", |app| app.show_cyclic = true),
        ("analyse", |app| app.show_analysis = true),
        ("désassemblage", |app| app.show_disassembly = true),
        ("graphe", |app| app.show_flow = true),
        // Le décompilateur est un module : il s'active dans les
        // préférences avant que sa fenêtre existe.
        ("décompilateur", |app| {
            app.show_decompiler_module = true;
            app.show_decompiler = true;
        }),
    ];

    for (nom, ouvre) in fenêtres {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 1400.0),
            )),
            ..Default::default()
        };
        // Les pastilles peintes ailleurs que dans la barre de titre de
        // la fenêtre principale, qui occupe les quarante premiers
        // pixels.
        let sous_la_barre = |app: &mut NamsdecApp| -> Vec<[i32; 3]> {
            let mut vues = Vec::new();
            for _ in 0..3 {
                vues.clear();
                let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                for clipped in &output.shapes {
                    if let egui::epaint::Shape::Circle(cercle) = &clipped.shape
                        && cercle.center.y > 40.0
                        && (cercle.radius - 6.5).abs() < 0.01
                    {
                        vues.push(teinte(cercle.fill));
                    }
                }
                output.textures_delta.clear();
            }
            vues
        };

        assert!(
            sous_la_barre(&mut app).is_empty(),
            "{nom} : fermée, rien ne traîne"
        );

        ouvre(&mut app);
        let vues = sous_la_barre(&mut app);
        assert_eq!(
            vues.len(),
            1,
            "{nom} : une pastille de fermeture, et une seule : {vues:?}"
        );
        // Le fondu laisse quelques unités d'écart après division : on
        // juge la teinte, pas l'arrondi.
        let écart = vues[0]
            .iter()
            .zip(attendue.iter())
            .map(|(a, b)| (a - b).abs())
            .max()
            .unwrap();
        assert!(
            écart <= 8,
            "{nom} : la pastille doit être celle de la fermeture, {:?} contre {attendue:?}",
            vues[0]
        );
    }
}

/// Lance un binaire qu'on vient d'écrire, en attendant qu'il se libère.
///
/// Ce test était le seul instable du dépôt : il tombait une fois sur
/// vingt, et seulement dans un `cargo test --workspace` complet. La
/// cause n'est pas dans le binaire produit, elle est chez les voisins.
/// Des dizaines de tests tournent en parallèle **dans le même
/// processus**, et plusieurs lancent `nasm` ou `ld`. Quand l'un d'eux
/// fait son `fork` pendant qu'on tient ici un fichier ouvert en
/// écriture, le fils hérite d'une copie du descripteur ; tant qu'il n'a
/// pas fait son `exec` — quelques microsecondes —, le noyau compte
/// encore une écriture ouverte sur notre fichier et refuse de
/// l'exécuter : `ETXTBSY`. Le programme est bon, c'est le voisin qui
/// tient encore la porte.
///
/// On attend donc, et seulement pour cette raison-là : toute autre
/// erreur remonte au premier tour, sans délai ajouté. Une seconde de
/// patience au total, ce qu'aucune de ces courses n'a jamais approché.
fn lance_ce_qui_vient_d_etre_ecrit(chemin: &std::path::Path) -> std::process::ExitStatus {
    let mut attente = std::time::Duration::from_millis(1);
    for _ in 0..12 {
        match std::process::Command::new(chemin).status() {
            Ok(statut) => return statut,
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(attente);
                attente *= 2;
            }
            Err(e) => panic!("le binaire exporté se lance : {e}"),
        }
    }
    panic!("le binaire exporté reste « occupé » après plusieurs secondes");
}

/// Le programme assemblé sort de l'atelier : un ELF exécutable, celui
/// que la ligne de commande aurait donné.
///
/// Un atelier qui ne laisse rien sortir apprend à se servir de lui, pas
/// de l'assembleur. Les deux commandes lancées sont affichées avec, pour
/// pouvoir refaire le travail sans namsdec.
#[test]
fn the_assembled_program_can_leave_the_workbench() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let chemin = dir.path().join("programme");

    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
    mov rax, 60
    xor rdi, rdi
    syscall
"
        .to_owned(),
        ..Default::default()
    };
    // Rien d'assemblé : rien à écrire, et le message le dit.
    app.export_binary(&chemin);
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
    assert!(!chemin.exists());

    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.export_binary(&chemin);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);

    // Un vrai ELF, et exécutable.
    let octets = std::fs::read(&chemin).unwrap();
    assert!(octets.starts_with(b"\x7fELF"), "un ELF");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&chemin).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "exécutable : {mode:o}");
    }
    // Et il tourne pour de vrai, avec le code qu'on lui a écrit.
    let sortie = lance_ce_qui_vient_d_etre_ecrit(&chemin);
    assert_eq!(sortie.code(), Some(0));

    // Les deux commandes sont dites, et elles sont celles qu'on
    // taperait.
    let commandes = namsdec_asm::build_commands("programme.asm", "programme", BASE_ADDR);
    assert!(commandes[0].contains("nasm -f elf64"));
    assert!(commandes[1].contains(&format!("-Ttext {BASE_ADDR:#x}")));
    app.console_tab = ConsoleTab::Diagnostics;
    app.show_console = true;
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).assemble_commands),
        "{painted}"
    );
    assert!(painted.contains("nasm -f elf64"), "{painted}");
}

/// Ouvrir un fichier objet — un `.o` — se lit et s'analyse, mais ne
/// s'exécute pas, et namsdec dit pourquoi.
///
/// Il s'en plaignait jusqu'ici en parlant de mémoire : « le binaire
/// dépasse la fenêtre simulée », devant un fichier de 560 octets. La
/// vraie raison est que `ld` n'est pas passé — aucune section n'a
/// d'adresse, il n'y a pas de point d'entrée.
#[test]
fn an_unlinked_object_reads_but_does_not_run() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let asm = dir.path().join("t.asm");
    let objet = dir.path().join("t.o");
    std::fs::write(
        &asm,
        "global _start\nsection .text\n_start:\n    mov rax, 60\n    xor rdi, rdi\n    syscall\n",
    )
    .unwrap();
    let ok = std::process::Command::new("nasm")
        .args(["-f", "elf64", "-o"])
        .arg(&objet)
        .arg(&asm)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return;
    }

    let mut app = NamsdecApp::default();
    app.load_binary(std::fs::read(&objet).unwrap(), "objet.o");

    // Il se lit : le désassemblage est là, avec l'étiquette.
    let state = app.assemble_state.as_ref().expect("chargé");
    assert!(!state.disasm.is_empty(), "le code est désassemblé");
    assert!(app.source.contains("_start:"), "{}", app.source);
    assert!(state.output.symbols.iter().any(|s| s.name == "_start"));

    // Il ne s'exécute pas, et le message dit pourquoi et quoi faire.
    assert!(app.emulator.is_none(), "rien à exécuter");
    let Status::Error(message) = &app.status else {
        panic!("attendu une explication, obtenu {:?}", app.status);
    };
    assert!(message.contains("ld -Ttext"), "{message}");
    assert!(
        !message.contains("mémoire"),
        "la mémoire n'y est pour rien : {message}"
    );
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
    let corrections: [(&str, &str, &str); 3] = [
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

/// Ce mot est-il un nom de registre, sous n'importe quelle largeur ?
///
/// Les seize généraux et leurs moitiés, les vectoriels, la pile x87. Ils
/// n'ont pas chacun leur fiche — c'est la notion « registre » qui les
/// explique —, et un test de couverture du manuel n'a pas à les
/// réclamer.
fn est_un_registre(mot: &str) -> bool {
    const CLASSIQUES: [&str; 8] = ["ax", "bx", "cx", "dx", "si", "di", "bp", "sp"];
    let mot = mot.to_ascii_lowercase();
    if let Some(reste) = mot.strip_prefix('r') {
        // r8 … r15, avec leurs suffixes d, w, b.
        let chiffres = reste.trim_end_matches(['d', 'w', 'b']);
        if !chiffres.is_empty() && chiffres.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    for base in CLASSIQUES {
        for forme in [
            format!("r{base}"),
            format!("e{base}"),
            base.to_owned(),
            format!("{}l", &base[..1]),
            format!("{}h", &base[..1]),
            format!("{base}l"),
        ] {
            if mot == forme {
                return true;
            }
        }
    }
    (mot.starts_with("xmm") || mot.starts_with("ymm") || mot.starts_with("st"))
        && mot.chars().any(|c| c.is_ascii_digit())
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
        Lang::Fr,
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
    app.load_binary(bytes, "programme");
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
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));
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

/// Le contenu de l'onglet a sa part du panneau, et elle tient dedans.
///
/// Le bandeau du haut grandit avec ce qu'il montre : l'instruction
/// détaillée, le dernier pas, dix-huit registres. Il finissait par tout
/// prendre, et ce qui suivait sortait par le bas du panneau — peint,
/// mais hors de l'écran. L'onglet RE, lui, se réduisait à son titre et à
/// son champ de recherche coupé en deux : une analyse de binaire lue par
/// une meurtrière.
#[test]
fn the_tab_keeps_its_share_of_the_side_panel() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_re_tab: true,
        active_tab: SidebarTab::Re,
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::vec2(1280.0, 1400.0);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, écran)),
        ..Default::default()
    };
    for _ in 0..3 {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
    }

    let part = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("sidebar_tab"))
        .expect("l'onglet a sa part du panneau")
        .outer_rect;
    assert!(
        part.height() >= SIDEBAR_TAB_MIN,
        "l'onglet doit garder de quoi montrer quelque chose : {}",
        part.height()
    );
    // Et le bandeau garde la sienne : ce qui vient, puis les registres,
    // au-dessus de l'onglet et non par-dessus.
    let panneau = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("sidebar"))
        .expect("le panneau latéral est affiché")
        .outer_rect;
    assert!(
        panneau.height() - part.height() >= SIDEBAR_BANNER_MIN,
        "il reste au bandeau de quoi se lire : {} sur {}",
        panneau.height() - part.height(),
        panneau.height()
    );
    // Rien ne sort par le bas : ce qui dépassait du panneau était peint
    // là où personne ne peut le lire.
    assert!(
        part.bottom() <= écran.y + 1.0,
        "le panneau de l'onglet déborde de l'écran : {part:?}"
    );
}

/// L'onglet RE mène à la fenêtre d'analyse.
///
/// Sept modules dans une colonne de trois cents pixels, cela se
/// consulte ; une analyse se lit en grand, tout déplié, et c'est là
/// qu'elle se copie ou s'enregistre. Le chemin doit être sous la main,
/// pas dans un menu.
#[test]
fn the_re_tab_leads_to_the_analysis_window() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_re_tab: true,
        active_tab: SidebarTab::Re,
        ..Default::default()
    };
    app.do_assemble();

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
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
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                peint.push_str(text.galley.text());
                peint.push('|');
            }
        }
        output.textures_delta.clear();
    }
    assert!(
        peint.contains(strings(Lang::Fr).analysis_open),
        "le chemin vers la fenêtre d'analyse doit être dans l'onglet :\n{peint}"
    );
    assert!(
        !app.show_analysis,
        "et il n'ouvre rien tant qu'on ne clique pas"
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

/// Un appel système que namsdec ne simule pas se dit à l'assemblage.
///
/// `open`, `execve`, `fork` : namsdec les nomme mais ne les fait pas — il
/// n'a ni fichiers ni processus à offrir. Un programme qui en contient
/// un s'arrête dessus, et rien dans le code ne le laisse deviner. Le
/// dire avant, avec sa ligne, vaut mieux que de le découvrir au milieu
/// d'une exécution.
#[test]
fn an_unsimulated_syscall_is_announced_before_it_stops_the_program() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n\
                 \x20   mov rax, 2\n    xor rdi, rdi\n    xor rsi, rsi\n\
                 \x20   xor rdx, rdx\n    syscall\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let dit: Vec<&namsdec_asm::Diagnostic> = app
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("open"))
        .collect();
    assert_eq!(
        dit.len(),
        1,
        "open doit être annoncé une fois : {:?}",
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

/// La part de l'onglet se fige à la poignée, et se rend au partage
/// automatique.
///
/// Une poignée tirée une fois qui ne se reprendrait plus, c'est un
/// réglage perdu pour de bon : la case d'Affichage doit rendre au
/// panneau le partage qu'il calcule lui-même, comme pour la console.
#[test]
fn the_tab_share_can_be_frozen_and_given_back() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let rendre = |app: &mut NamsdecApp, ctx: &egui::Context| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 1400.0),
            )),
            ..Default::default()
        };
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }
        egui::containers::panel::PanelState::load(ctx, egui::Id::new("sidebar_tab"))
            .expect("l'onglet a sa part")
            .outer_rect
            .height()
    };

    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let partagée = rendre(&mut app, &ctx);

    // Figée — ce que fait la poignée, et ce que fait la case d'Affichage
    // qu'on décoche : la part garde sa hauteur, même quand le bandeau se
    // replie et libère de la place.
    app.sidebar_tab_height = Some(partagée);
    app.cpu_state_open = false;
    let figée = rendre(&mut app, &ctx);
    assert!(
        (figée - partagée).abs() < 2.0,
        "la hauteur choisie est celle qu'on garde : {figée} contre {partagée}"
    );

    // Rendue au partage : elle suit de nouveau ce que le bandeau laisse,
    // et le bandeau replié lui en laisse plus qu'au départ.
    app.sidebar_tab_height = None;
    let rendue = rendre(&mut app, &ctx);
    assert!(
        rendue > partagée,
        "l'état replié rend sa place à l'onglet : {rendue} contre {partagée}"
    );
}

/// À toutes les tailles de fenêtre, chaque panneau tient dans l'écran.
///
/// Un panneau qui déborde ne se voit pas : egui peint quand même, et le
/// contenu part sous le bord. C'est ainsi que l'onglet du panneau
/// latéral se réduisait à son titre sans que rien ne le dise. Le même
/// piège guette la console, la barre de statut et la légende — d'où ce
/// tour d'horizon, du portable à l'écran large.
#[test]
fn every_panel_fits_the_window_at_any_size() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    for taille in [
        egui::vec2(1024.0, 600.0),
        egui::vec2(1280.0, 800.0),
        egui::vec2(1600.0, 1000.0),
        egui::vec2(1920.0, 1080.0),
    ] {
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            show_re_tab: true,
            ..Default::default()
        };
        app.do_assemble();
        app.do_step();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, taille)),
            ..Default::default()
        };
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }

        for nom in [
            "console",
            "sidebar",
            "sidebar_tab",
            "cpu_state",
            "statusbar",
        ] {
            let Some(état) = egui::containers::panel::PanelState::load(&ctx, egui::Id::new(nom))
            else {
                continue;
            };
            let rect = état.outer_rect;
            assert!(
                rect.height() <= taille.y + 1.0 && rect.width() <= taille.x + 1.0,
                "{nom} déborde de la fenêtre {taille:?} : {rect:?}"
            );
            assert!(
                rect.height() >= 0.0,
                "{nom} a une hauteur négative à {taille:?} : {rect:?}"
            );
        }

        // Et l'essentiel reste lisible même sur le plus petit écran :
        // l'instruction qui vient, les registres, la vue choisie.
        let mut peint = String::new();
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                peint.push_str(text.galley.text());
                peint.push('|');
            }
        }
        output.textures_delta.clear();
        for attendu in [
            "RIP",
            strings(Lang::Fr).cpu_state,
            strings(Lang::Fr).tab_stack,
        ] {
            assert!(
                peint.contains(attendu),
                "« {attendu} » manque à la taille {taille:?}"
            );
        }
    }
}

/// La palette mène aux étiquettes du programme, par leur nom.
///
/// Sur deux cents lignes, « où est donc `affiche_nombre` ? » se répond
/// en tapant son nom — comme on ouvre un fichier dans un éditeur. Rien
/// de nouveau à retenir : c'est la palette qu'on connaît déjà.
#[test]
fn the_palette_leads_to_the_labels_of_the_program() {
    let source = "global _start\nsection .text\n_start:\n    call affiche\n    hlt\n\
                  affiche:\n.local:\n    ret\n";
    let étiquettes = source_labels(source);
    assert_eq!(
        étiquettes,
        vec![("_start".to_owned(), 3), ("affiche".to_owned(), 6)],
        "les étiquettes locales ne comptent pas, ni les lignes indentées"
    );

    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: source.to_owned(),
        command_palette: Some(CommandPalette {
            query: "affiche".to_owned(),
            ..Default::default()
        }),
        ..Default::default()
    };
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
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                peint.push_str(text.galley.text());
                peint.push('|');
            }
        }
        output.textures_delta.clear();
    }
    assert!(
        peint.contains("Aller à : affiche (ligne 6)"),
        "l'étiquette cherchée doit être proposée :\n{peint}"
    );

    // Et la commande y emmène.
    app.execute_palette_command(&ctx, PaletteCommand::GoLine(6));
    assert_eq!(app.goto_line, Some(6), "la palette pose le curseur là-bas");
}

/// Le bandeau ne se peint jamais par-dessus la vue du dessous.
///
/// Un panneau hors zone de défilement dont le contenu dépasse la hauteur
/// allouée n'est pas rogné : il déborde. L'état du processeur se
/// retrouvait ainsi peint par-dessus l'onglet Mémoire, les deux
/// illisibles, sur une fenêtre où il manquait deux cents pixels. Ce que
/// le bandeau ne peut pas montrer doit disparaître, pas se superposer.
#[test]
fn the_banner_never_paints_over_the_tab_below() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Des fenêtres étroites et courtes : c'est là que le bandeau ne
    // tient pas, et c'est là que le débordement se voyait.
    for taille in [
        egui::vec2(900.0, 560.0),
        egui::vec2(1024.0, 640.0),
        egui::vec2(1280.0, 800.0),
    ] {
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            active_tab: SidebarTab::Memory,
            ..Default::default()
        };
        app.do_assemble();
        app.do_step();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, taille)),
            ..Default::default()
        };
        let mut débordements: Vec<(String, f32)> = Vec::new();
        for _ in 0..3 {
            débordements.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            let Some(onglet) =
                egui::containers::panel::PanelState::load(&ctx, egui::Id::new("sidebar_tab"))
                    .map(|état| état.outer_rect)
            else {
                output.textures_delta.clear();
                continue;
            };
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &clipped.shape {
                    // Ce que seul le bandeau écrit : les noms de
                    // registres, les indicateurs, ses propres titres.
                    let mot = text.galley.text();
                    let du_bandeau =
                        // « RIP » est écarté : l'onglet Mémoire le
                        // propose lui aussi, dans son « Suivre ».
                        matches!(mot, "RAX" | "R15" | "CF 0" | "ZF 0" | "ZF 1")
                            || mot == strings(Lang::Fr).cpu_state
                            || mot == strings(Lang::Fr).next_instruction;
                    // Peint *et* visible : un texte que sa zone de
                    // défilement coupe ne se voit pas, et c'est
                    // précisément ce qu'on veut qu'il arrive.
                    let visible = text.pos.y < clipped.clip_rect.bottom();
                    if du_bandeau && visible && text.pos.y > onglet.top() + 1.0 {
                        débordements.push((mot.to_owned(), text.pos.y));
                    }
                }
            }
            output.textures_delta.clear();
        }
        assert!(
            débordements.is_empty(),
            "à {taille:?}, le bandeau déborde sur la vue du dessous : {débordements:?}"
        );
    }
}

/// Une valeur de huit octets se lit en entier, jusqu'au dernier
/// chiffre — dans les registres comme dans la pile.
///
/// Seize chiffres dont le seizième passe sous le bord du panneau, ce
/// n'est pas une valeur abrégée : c'est une valeur *fausse*, et rien ne
/// le dit. Chaque vue doit donc compter sur la largeur dont elle dispose
/// réellement, barre de défilement comprise — quitte à raccourcir ce qui
/// peut l'être sans mentir, comme l'adresse d'un mot de pile, que son
/// écart depuis RSP remplace alors.
#[test]
fn an_eight_byte_value_is_never_cut_by_the_edge_of_the_panel() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // La largeur du panneau lui-même, et non celle de la fenêtre :
    // c'est en tirant le panneau qu'on gagne une colonne, et c'est là
    // que la dernière risque de passer sous le bord.
    for panneau in [
        SIDEBAR_MIN_WIDTH,
        SIDEBAR_WIDTH,
        420.0_f32,
        520.0,
        620.0,
        720.0,
        860.0,
    ] {
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        app.do_step();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        ctx.data_mut(|data| {
            data.insert_persisted(
                egui::Id::new("sidebar"),
                egui::containers::panel::PanelState {
                    outer_rect: egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(panneau, 1400.0),
                    ),
                },
            )
        });
        let taille = egui::vec2(1920.0, 1400.0);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, taille)),
            ..Default::default()
        };
        let mut coupées: Vec<(String, f32, f32)> = Vec::new();
        for _ in 0..3 {
            coupées.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &clipped.shape {
                    let mot = text.galley.text();
                    // Une valeur de registre : seize chiffres
                    // hexadécimaux, et rien d'autre.
                    if mot.len() != 16 || !mot.chars().all(|c| c.is_ascii_hexdigit()) {
                        continue;
                    }
                    let droite = text.pos.x + text.galley.size().x;
                    eprintln!(
                        "VAL x={} w={} droite={droite} clip={:?}",
                        text.pos.x,
                        text.galley.size().x,
                        clipped.clip_rect
                    );
                    // repère : où commence le nom

                    if droite > clipped.clip_rect.right() + 0.5 {
                        coupées.push((mot.to_owned(), droite, clipped.clip_rect.right()));
                    }
                }
            }
            output.textures_delta.clear();
        }
        assert!(
            coupées.is_empty(),
            "un panneau de {panneau} laisse passer des valeurs sous le bord : {coupées:?}"
        );
    }
}

/// Lire du code « en Texte » doit s'expliquer, pas laisser deviner.
///
/// À `_start`, les octets sont `b8 01 00 00 00` : deux illisibles, puis
/// un zéro qui termine la chaîne. On lisait donc « <b8><01> » et « \0 »,
/// sans un mot pour dire que ce n'était pas du texte, que le zéro
/// arrêtait la lecture, ni où se trouvait le message qu'on cherchait.
#[test]
fn reading_code_as_text_says_why_it_looks_like_nothing() {
    let mut mem = namsdec_core::Memory::new(BASE_ADDR, 0x1000);
    // `mov eax, 1`, puis de quoi remplir, puis un vrai message.
    mem.load(BASE_ADDR, b"\xb8\x01\x00\x00\x00").unwrap();
    mem.load(BASE_ADDR + 0x20, b"salut, monde\n\0").unwrap();

    let lu = read_memory(&mem, BASE_ADDR, MemoryFormat::Text, "hors bornes", Lang::Fr);
    // Ce que `write` enverrait : deux octets, montrés tels quels.
    assert!(lu.contains("<b8><01>"), "{lu}");
    assert!(
        lu.contains("2 octets, puis un zéro"),
        "le zéro qui arrête la lecture doit être nommé : {lu}"
    );
    assert!(
        lu.contains("ne forment pas du texte"),
        "et l'on doit dire que ce n'en est pas : {lu}"
    );
    // Et le message qui suit, avec son adresse : c'est la question
    // suivante, et elle n'avait pas de réponse.
    assert!(
        lu.contains(&format!("{:#010x}", BASE_ADDR + 0x20)) && lu.contains("salut, monde"),
        "le texte lisible de la fenêtre doit être listé : {lu}"
    );

    // Sur du vrai texte, rien de tout cela : la chaîne, et c'est tout.
    let lu = read_memory(
        &mem,
        BASE_ADDR + 0x20,
        MemoryFormat::Text,
        "hors bornes",
        Lang::Fr,
    );
    assert!(lu.contains("salut, monde"), "{lu}");
    assert!(
        !lu.contains("ne forment pas du texte"),
        "ici c'en est, et il ne faut pas dire le contraire : {lu}"
    );
    assert!(
        !lu.contains("Texte lisible dans les octets qui suivent"),
        "et rien à chercher ailleurs quand on est déjà dessus : {lu}"
    );

    // Un premier octet nul : la chaîne est vide, et c'est une réponse.
    let lu = read_memory(
        &mem,
        BASE_ADDR + 0x100,
        MemoryFormat::Text,
        "hors bornes",
        Lang::Fr,
    );
    assert!(lu.contains("Chaîne vide"), "{lu}");
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

/// Le déroulement s'ouvre par « Espace g » et montre la forme du
/// programme.
///
/// Un listing se lit de haut en bas ; un programme, non. La fenêtre
/// dessine ce qu'on reconstitue de tête en lisant : les blocs, les
/// sauts pris et non pris, et la boucle qui remonte.
#[test]
fn the_flow_window_draws_the_shape_of_the_program() {
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
    call fin
    hlt
fin:
    ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();

    // Le raccourci : `Espace g`, sous le même préfixe que l'explorateur.
    let chord = app
        .keybindings
        .map
        .get(&Action::ShowFlow)
        .expect("le raccourci existe");
    assert_eq!(
        *chord,
        crate::keybindings::Chord::leader(&[egui::Key::Space, egui::Key::V, egui::Key::G]),
        "« Espace g » ouvre le déroulement"
    );

    app.dispatch(Action::ShowFlow);
    assert!(app.show_flow, "l'action ouvre la fenêtre");

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let mut peint = String::new();
    let mut traits = 0usize;
    let mut flèches = 0usize;
    for _ in 0..2 {
        peint.clear();
        traits = 0;
        flèches = 0;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => {
                    peint.push_str(text.galley.text());
                    peint.push('|');
                }
                // Les flèches : un trait pour le corps, un triangle pour
                // la pointe.
                egui::epaint::Shape::LineSegment { .. } => traits += 1,
                egui::epaint::Shape::Path(path) if path.points.len() == 3 => flèches += 1,
                _ => {}
            }
        }
        output.textures_delta.clear();
    }

    assert!(peint.contains(strings(Lang::Fr).flow_title), "{peint}");
    // Les instructions du programme sont dans les boîtes.
    for attendu in ["dec rcx", "hlt"] {
        assert!(peint.contains(attendu), "« {attendu} » manque :\n{peint}");
    }
    // Et le bloc où le programme se trouve est signalé.
    assert!(
        peint.contains(strings(Lang::Fr).flow_here),
        "le bloc courant doit se voir :\n{peint}"
    );
    // Des flèches, et pas qu'une : le saut pris, le saut non pris,
    // l'appel, le retour de l'appel.
    assert!(traits >= 4, "trop peu de traits : {traits}");
    assert!(flèches >= 4, "trop peu de pointes : {flèches}");

    // Sans programme assemblé, la fenêtre dit quoi faire plutôt que de
    // rester vide.
    let mut vierge = NamsdecApp {
        show_flow: true,
        ..Default::default()
    };
    let peint = paint(&mut vierge);
    assert!(peint.contains(strings(Lang::Fr).flow_empty), "{peint}");
}

/// Le graphe se lit comme le programme : entrée en haut, boucle qui
/// remonte, deux chemins après un test.
#[test]
fn the_flow_graph_says_what_the_listing_hides() {
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
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    let state = app.assemble_state.as_ref().expect("assemblé");
    let cfg = namsdec_re::cfg::build(&state.disasm);

    // Trois blocs : l'entrée, le corps de la boucle, la sortie.
    assert!(cfg.blocks.len() >= 3, "{:?}", cfg.blocks);
    let niveaux = cfg.layers();
    assert_eq!(niveaux[0], 0, "l'entrée est en haut");

    // Le test ouvre bien deux chemins, et l'un des deux remonte.
    let corps = cfg
        .blocks
        .iter()
        .position(|b| b.exit == namsdec_re::cfg::Exit::Branch)
        .expect("un bloc se termine par un saut conditionnel");
    let pris = cfg
        .edges
        .iter()
        .find(|e| e.from == corps && e.kind == namsdec_re::cfg::Kind::Taken)
        .expect("le chemin du saut pris");
    assert!(
        cfg.blocks[pris.to].start <= cfg.blocks[corps].start,
        "la boucle remonte"
    );
    assert!(
        cfg.edges
            .iter()
            .any(|e| e.from == corps && e.kind == namsdec_re::cfg::Kind::NotTaken),
        "et l'autre chemin continue"
    );
}

/// Le guide de l'utilisateur dit la vérité sur la version qu'il décrit.
///
/// Un guide qui annonce une autre version que celle du programme, ou
/// quarante-sept exercices quand il y en a quatre-vingt-seize, est pire
/// qu'une absence de guide. Son générateur lit donc le dépôt, et ce test
/// vérifie qu'il n'y a rien d'écrit en dur qui puisse vieillir.
#[test]
fn the_user_guide_is_generated_from_what_the_repository_knows() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(racine.join("docs/guide.py"))
        .expect("le générateur du guide est dans le dépôt");

    // La version, les comptes : lus, jamais recopiés.
    for lecture in [
        "Cargo.toml",
        "packs/namsdec.json",
        "namsdec-doc/src/table.rs",
        "keybindings.rs",
    ] {
        assert!(
            source.contains(lecture),
            "le guide doit tirer ses nombres de {lecture}"
        );
    }
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        !source.contains(&format!("\"{version}\"")),
        "la version ne doit pas être écrite en dur dans le générateur"
    );

    // Et le PDF livré porte bien la version du jour dans son nom.
    let guide = racine.join(format!("docs/guide-namsdec-{version}.pdf"));
    assert!(
        guide.is_file(),
        "le guide de la version {version} manque : {}",
        guide.display()
    );
    let octets = std::fs::read(&guide).expect("le guide se lit");
    assert!(octets.starts_with(b"%PDF-"), "ce n'est pas un PDF");
    assert!(
        octets.len() > 10_000,
        "un guide de {} octets est trop court pour en être un",
        octets.len()
    );
}

/// Devant un binaire dépouillé, namsdec dit par où commencer.
///
/// Un `.text` de deux cent quarante kilo-octets dont presque tout
/// appartient à la bibliothèque standard : la première question est « où
/// est le programme ». La convention de démarrage y répond — le code
/// d'amorçage passe l'adresse de `main` dans RDI —, et les cibles
/// d'appel donnent le reste.
#[test]
fn a_stripped_binary_says_where_the_program_starts() {
    // Le démarrage que produit un compilateur C, à l'octet près.
    let bytes: &[u8] = &[
        0xF3, 0x0F, 0x1E, 0xFA, // endbr64
        0x31, 0xED, // xor ebp, ebp
        0x49, 0x89, 0xD1, // mov r9, rdx
        0x48, 0x8D, 0x3D, 0x41, 0x0D, 0x00, 0x00, // lea rdi, [rip+0xd41]
        0xFF, 0x15, 0xEB, 0xE3, 0x03, 0x00, // call [rip+0x3e3eb]
        0xF4, // hlt
    ];
    let disasm = namsdec_disasm::disassemble(bytes, 0xf6f0);
    // `lea` à 0xf6f9, longue de sept octets : 0xf700 + 0xd41.
    assert_eq!(
        namsdec_re::functions::main_of(&disasm, 0xf6f0),
        Some(0x10441),
        "le `main` se lit dans le premier argument passé au démarrage"
    );

    // Et les fonctions se découpent sur les cibles d'appel.
    let fonctions = namsdec_re::functions::discover(&disasm, 0xf6f0);
    assert!(
        fonctions
            .iter()
            .any(|f| f.origin == namsdec_re::functions::Origin::Entry),
        "le point d'entrée est nommé"
    );
}

/// Les chaînes d'un binaire Rust se découpent par le code, pas par les
/// octets.
///
/// Sans zéro final, un scan des données agglomère « Error flushing
/// bufferError reading inputIncorrect password » en une seule ligne
/// absurde. Le code, lui, porte la longueur de chacune juste à côté de
/// son adresse.
#[test]
fn strings_without_a_final_zero_are_cut_by_the_code_that_uses_them() {
    let données = b"Incorrect password\nCorrect Password\n";
    let mut fichier = vec![0u8; 0x1000];
    fichier[0x300..0x300 + données.len()].copy_from_slice(données);
    let sections = vec![namsdec_re::elf_scan::SectionInfo {
        name: ".rodata".into(),
        addr: 0x3000,
        size: 0x100,
        write: false,
        execute: false,
        offset: 0x300,
        nobits: false,
    }];

    // lea rsi, [rip+…] vers « Correct Password\n » ; mov edx, 17 ; call
    let cible = 0x3000u64 + 19;
    let déplacement = (cible as i64 - 0x1007) as i32;
    let mut code = vec![0x48, 0x8D, 0x35];
    code.extend_from_slice(&déplacement.to_le_bytes());
    code.extend_from_slice(&[0xBA, 0x11, 0x00, 0x00, 0x00]);
    code.extend_from_slice(&[0xE8, 0x00, 0x00, 0x00, 0x00]);
    let disasm = namsdec_disasm::disassemble(&code, 0x1000);

    let littérales = namsdec_re::literals::scan(&disasm, &sections, &fichier);
    assert_eq!(littérales.len(), 1, "{littérales:?}");
    assert_eq!(
        littérales[0].text, "Correct Password\n",
        "la chaîne s'arrête où le code dit qu'elle s'arrête"
    );
    assert_eq!(
        littérales[0].from, 0x1000,
        "et l'on sait d'où l'on s'en sert"
    );
}

/// Devant un binaire qu'on ne peut pas exécuter, la provenance se lit
/// quand même — dans le code, à l'envers.
///
/// L'historique d'exécution est la meilleure réponse à « d'où vient
/// cette valeur ». Mais un binaire qui appelle une bibliothèque qu'on
/// n'a pas ne s'exécute pas : il n'y a pas d'historique, et la seule
/// voie est de remonter les instructions. C'est ce qu'on fait à la main
/// devant un listing.
#[test]
fn where_a_value_comes_from_can_be_read_in_the_code_itself() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
    mov rax, 5
    mov rcx, rax
    add rcx, 2
    cmp rcx, 7
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    // Trois pas : on s'arrête sur le `cmp`, sans l'exécuter.
    for _ in 0..3 {
        app.do_step();
    }

    let chaîne = app.static_provenance("rcx");
    assert!(chaîne.len() >= 3, "{chaîne:?}");
    assert!(
        chaîne[0].text.starts_with("add"),
        "la dernière écriture d'abord : {:?}",
        chaîne[0]
    );
    assert_eq!(
        chaîne.last().expect("la source").writes,
        "rax",
        "et l'on remonte jusqu'à la source : {chaîne:?}"
    );

    // Ce que le survol du registre montre alors.
    let peint = paint(&mut app);
    assert!(peint.contains("RCX"), "{peint}");
}

/// Les figures du code sortent une comparaison déguisée d'un binaire
/// entier.
///
/// C'est la leçon d'un vrai crackme : `mov ecx, 0x48555648` ne dit rien,
/// et les mêmes octets lus comme du texte disent « HVUH ». Un XOR nul
/// veut dire égalité, donc la constante *est* la valeur attendue.
#[test]
fn a_disguised_comparison_hands_over_what_it_expects() {
    // Le cœur d'un crackme, tel qu'il se lit dans le binaire.
    let bytes: &[u8] = &[
        0x48, 0x83, 0xFE, 0x07, // cmp rsi, 7
        0x75, 0x19, // jne
        0xB9, 0x48, 0x56, 0x55, 0x48, // mov ecx, 0x48555648
        0x33, 0x0C, 0x03, // xor ecx, [rbx+rax]
        0xBA, 0x48, 0x41, 0x44, 0x4E, // mov edx, 0x4E444148
        0x33, 0x54, 0x03, 0x03, // xor edx, [rbx+rax+3]
        0x09, 0xCA, // or edx, ecx
        0x74, 0x10, // je
    ];
    let disasm = namsdec_disasm::disassemble(bytes, 0x10106);
    let figures = namsdec_re::patterns::scan(&disasm);

    // La longueur attendue.
    let longueur = figures
        .iter()
        .find(|f| f.pattern == namsdec_re::patterns::Pattern::LengthCheck)
        .expect("le contrôle de longueur");
    assert_eq!(longueur.detail, "7");

    // Et les deux morceaux de la valeur attendue, lus en clair.
    let xors: Vec<&str> = figures
        .iter()
        .filter(|f| f.pattern == namsdec_re::patterns::Pattern::XorCompare)
        .map(|f| f.detail.as_str())
        .collect();
    assert_eq!(xors.len(), 2, "{xors:?}");
    assert!(xors[0].contains("HVUH"), "{xors:?}");
    assert!(xors[1].contains("HADN"), "{xors:?}");
}

/// « Comment arrive-t-on là ? » : la route, et ses conditions.
///
/// C'est la lecture d'un programme qui refuse quelque chose. On connaît
/// l'adresse où il accepte — la chaîne « Correct » est dans les données,
/// et quelque chose la charge —, et l'on cherche le chemin, avec ce
/// qu'il faut satisfaire en route.
#[test]
fn the_route_to_an_address_lists_what_must_hold() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
    cmp rsi, 7
    jne refus
    mov ecx, 0x48555648
    xor ecx, [rbx]
    jne refus
succes:
    mov rax, 1
    hlt
refus:
    xor rax, rax
    hlt
"
        .to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.do_assemble();
    let state = app.assemble_state.as_ref().expect("assemblé");
    let cfg = namsdec_re::cfg::build(&state.disasm);

    // L'adresse du succès, telle qu'on la trouverait par les chaînes.
    let succes = *state
        .output
        .symbols
        .iter()
        .find(|s| s.name == "succes")
        .map(|s| &s.addr)
        .expect("l'étiquette du succès");

    let route = namsdec_re::route::to(&cfg, succes).expect("une route mène au succès");
    // Deux décisions à passer : la longueur, puis la comparaison.
    assert_eq!(route.stages.len(), 2, "{:?}", route.stages);
    for étape in &route.stages {
        assert_eq!(
            étape.condition,
            namsdec_re::route::Condition::NotTaken,
            "les deux `jne` doivent échouer pour arriver au succès : {étape:?}"
        );
        assert!(étape.decides.starts_with("jne"), "{étape:?}");
    }

    // Et la fenêtre montre la route quand on la demande.
    app.flow_goal = format!("{succes:#x}");
    let peint = paint(&mut app);
    assert!(
        peint.contains(strings(Lang::Fr).flow_route_steps),
        "les conditions doivent être listées :\n{peint}"
    );
    assert!(
        peint.contains("jne"),
        "avec l'instruction qui décide :\n{peint}"
    );
}

/// La décompilation ne s'exécute pas toute seule, et le dit avant.
///
/// Ce module n'ajoute pas un affichage : il accorde à namsdec la
/// permission de lancer un programme extérieur. Trois verrous, et ce
/// test les vérifie tous les trois — la case des préférences, l'entrée
/// de menu qui n'existe qu'ensuite, et le bouton qui seul déclenche.
#[test]
fn the_decompiler_never_runs_on_its_own() {
    let app = NamsdecApp::default();
    assert!(
        !app.show_decompiler_module,
        "le module est éteint tant que personne ne l'allume"
    );
    assert!(app.decompile_job.is_none() && app.decompile_result.is_none());

    // Éteint, la fenêtre ne s'ouvre pas — même si on le demande.
    let mut app = NamsdecApp {
        show_decompiler: true,
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        !peint.contains(strings(Lang::Fr).decompiler_title),
        "la fenêtre reste fermée tant que le module est éteint :\n{peint}"
    );

    // Allumé mais sans outil installé, la fenêtre explique comment en
    // installer un, et ne lance rien.
    let mut app = NamsdecApp {
        show_decompiler_module: true,
        show_decompiler: true,
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        peint.contains(strings(Lang::Fr).decompiler_title),
        "{peint}"
    );
    assert!(
        peint.contains(strings(Lang::Fr).decompiler_none_installed),
        "{peint}"
    );
    assert!(app.decompile_job.is_none(), "rien n'a été lancé");
}

/// Deux millions de caractères se lisent, se cherchent et se parcourent.
///
/// RetDec rend soixante mille lignes de C pour un petit crackme. Ce test
/// tient trois promesses : on ne peint que ce qui tient à l'écran — sinon
/// la fenêtre se fige —, les fonctions sont repérées, et le filtre réduit
/// vraiment ce qu'on regarde.
#[test]
fn a_huge_decompilation_stays_readable() {
    let mut texte = String::new();
    for i in 0..20_000 {
        texte.push_str(&format!("int32_t function_{i:05x}(int32_t a) {{\n"));
        texte.push_str("    return a + 1;\n");
        texte.push_str("}\n");
    }
    let dernière = "function_04e1f";

    let mut app = NamsdecApp {
        show_decompiler_module: true,
        show_decompiler: true,
        decompilers: vec![namsdec_disasm::plugin::Plugin {
            id: "essai".to_owned(),
            name: "Essai".to_owned(),
            command: "wc".to_owned(),
            args: vec!["-c".to_owned(), "{file}".to_owned()],
            install: None,
        }],
        decompiler: "essai".to_owned(),
        ..Default::default()
    };
    app.index_decompilation(&texte);
    app.decompile_result = Some(Ok(texte));

    assert_eq!(
        app.decompile_functions.len(),
        20_000,
        "chaque fonction ouvre une ligne sans indentation"
    );
    assert_eq!(app.decompile_lines.len(), 60_000);

    let peint = paint(&mut app);
    assert!(
        peint.contains("function_00000"),
        "le début se lit :\n{}",
        &peint[..peint.len().min(2000)]
    );
    assert!(
        !peint.contains(dernière),
        "mais pas la soixante-millième ligne : on ne peint que l'écran"
    );

    // Le filtre ne garde que ce qu'on cherche, et le compte le dit.
    app.decompile_filter = "function_04e1f".to_owned();
    let peint = paint(&mut app);
    assert_eq!(
        app.decompile_visible.len(),
        1,
        "une seule ligne porte ce nom"
    );
    assert!(
        peint.contains(dernière),
        "et c'est elle qu'on voit maintenant"
    );

    // Sauter à une fonction remet le texte entier, à la bonne hauteur.
    app.decompile_filter.clear();
    app.decompile_goto = Some(app.decompile_functions[10_000].0);
    let peint = paint(&mut app);
    assert!(
        peint.contains("function_02710"),
        "le saut amène où l'on a cliqué :\n{peint}"
    );
}

/// Avec un outil installé, la commande est écrite avant d'être lancée —
/// et « Lancer » la lance vraiment.
#[test]
fn the_decompiler_shows_the_command_then_runs_it() {
    let outil = namsdec_disasm::plugin::Plugin {
        id: "essai".to_owned(),
        name: "Essai".to_owned(),
        // `wc -c` sur le fichier : un outil qui existe partout, dont la
        // réponse dépend vraiment du binaire qu'on lui donne.
        command: "wc".to_owned(),
        args: vec!["-c".to_owned(), "{file}".to_owned()],
        install: None,
    };
    let mut app = NamsdecApp {
        show_decompiler_module: true,
        show_decompiler: true,
        decompilers: vec![outil],
        decompiler: "essai".to_owned(),
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    app.do_assemble();

    // La commande est annoncée, en toutes lettres.
    let peint = paint(&mut app);
    assert!(
        peint.contains("wc -c {file}"),
        "ce qui sera lancé doit se lire avant de l'être :\n{peint}"
    );
    assert!(peint.contains(strings(Lang::Fr).decompiler_run), "{peint}");
    assert!(app.decompile_result.is_none(), "afficher n'est pas lancer");

    // Et le lancement, lui, rend ce que l'outil a écrit.
    app.start_decompile();
    let ctx = egui::Context::default();
    for _ in 0..200 {
        app.poll_decompile(&ctx);
        if app.decompile_result.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let réponse = app
        .decompile_result
        .as_ref()
        .expect("l'outil a fini")
        .as_ref()
        .expect("sans erreur");
    // `wc -c` écrit la taille du fichier : c'est bien notre binaire.
    let taille = app
        .assemble_state
        .as_ref()
        .expect("assemblé")
        .output
        .file
        .len();
    assert!(
        réponse.contains(&taille.to_string()),
        "l'outil a bien reçu le programme lié : {réponse:?} pour {taille} octets"
    );
    assert!(app.decompiled_from.is_some(), "on dit de quoi il s'agit");
}

/// Un outil qui ne rend pas la main est arrêté, et l'on dit pourquoi.
#[test]
fn a_tool_that_never_finishes_is_stopped() {
    let dormeur = namsdec_disasm::plugin::Plugin {
        id: "dormeur".to_owned(),
        name: "Dormeur".to_owned(),
        // `tail -f` suit un fichier sans fin : c'est exactement l'outil
        // qui ne rend jamais la main, et il existe partout.
        command: "tail".to_owned(),
        args: vec!["-f".to_owned(), "{file}".to_owned()],
        install: None,
    };
    let début = std::time::Instant::now();
    let erreur = dormeur
        .run_within(b"peu importe", std::time::Duration::from_millis(300))
        .expect_err("l'outil doit être arrêté");
    assert!(
        début.elapsed() < std::time::Duration::from_secs(5),
        "on ne l'a pas attendu : {:?}",
        début.elapsed()
    );
    assert!(
        erreur.to_string().contains("longer"),
        "et l'on dit qu'il a été arrêté : {erreur}"
    );
}

/// Ce qui tourne à côté se voit : dans la barre d'état, et jusque dans
/// la barre des tâches du système.
///
/// Un fil d'exécution est invisible. Sans un mot quelque part, on ne
/// sait ni que quelque chose tourne, ni depuis combien de temps, ni
/// combien de temps encore avant l'abandon. La progression montrée est
/// celle du **temps accordé** — la seule qu'un programme extérieur nous
/// laisse connaître, et la seule qui décide de quelque chose.
#[test]
fn a_background_job_shows_its_progress_where_one_looks() {
    let mut app = NamsdecApp {
        show_statusbar: true,
        ..Default::default()
    };
    // Rien ne tourne : rien ne s'affiche, et le titre reste le nom.
    let peint = paint(&mut app);
    assert!(
        !peint.contains(strings(Lang::Fr).job_decompiling),
        "aucun travail, aucun bruit :\n{peint}"
    );

    // Un travail borné : la barre montre une part vraie, et le texte dit
    // le temps écoulé sur le temps accordé.
    app.job = Some(BackgroundJob {
        label: strings(Lang::Fr).job_decompiling.to_owned(),
        started: std::time::Instant::now() - std::time::Duration::from_secs(30),
        limit: Some(std::time::Duration::from_secs(120)),
    });
    let part = app.job.as_ref().and_then(BackgroundJob::fraction);
    assert!(
        part.is_some_and(|p| (p - 0.25).abs() < 0.02),
        "trente secondes sur deux minutes : un quart, obtenu {part:?}"
    );
    let peint = paint(&mut app);
    assert!(
        peint.contains(strings(Lang::Fr).job_decompiling),
        "la barre d'état nomme ce qui tourne :\n{peint}"
    );
    assert!(
        peint.contains("30 s / 120 s"),
        "avec le temps écoulé sur le temps accordé :\n{peint}"
    );

    // Un travail que rien ne borne : pas de fraction inventée.
    app.job = Some(BackgroundJob {
        label: strings(Lang::Fr).job_checking_update.to_owned(),
        started: std::time::Instant::now(),
        limit: None,
    });
    assert_eq!(
        app.job.as_ref().and_then(BackgroundJob::fraction),
        None,
        "sans limite, aucune progression ne peut être honnête"
    );

    // Et le titre de la fenêtre porte l'avancement : c'est lui que la
    // barre des tâches du système affiche.
    let ctx = egui::Context::default();
    app.job = Some(BackgroundJob {
        label: "Décompilation".to_owned(),
        started: std::time::Instant::now() - std::time::Duration::from_secs(60),
        limit: Some(std::time::Duration::from_secs(120)),
    });
    app.reflect_job_in_title(&ctx);
    let titre = app.window_title.clone().expect("un titre a été posé");
    assert!(titre.contains("50 %"), "{titre}");
    assert!(titre.starts_with("Namsdec"), "{titre}");

    // Le travail fini, le titre revient à ce qu'il était : une
    // application marquée « 100 % » pour toujours fait douter du reste.
    app.job = None;
    app.reflect_job_in_title(&ctx);
    assert_eq!(app.window_title.as_deref(), Some("Namsdec"));
}

/// Déclarer un outil écrit un fichier — et rien d'autre.
///
/// C'est la promesse de cet écran : namsdec ne télécharge pas Rizin, ne
/// l'installe pas, ne le lance pas. Il pose la description que l'outil
/// attendait, avec les arguments qui marchent — ceux qu'on aurait mis
/// une heure à trouver soi-même.
#[test]
fn declaring_a_tool_writes_a_file_and_nothing_else() {
    let dossier = tempfile::tempdir().expect("un dossier temporaire");
    let mut app = NamsdecApp::default();
    assert!(app.decompilers.is_empty());

    let rizin = namsdec_disasm::known::all()
        .into_iter()
        .find(|outil| outil.plugin.id == "rizin-ghidra")
        .expect("Rizin est un outil connu");

    with_data_home(dossier.path(), || app.declare_tool(&rizin.plugin));

    // Le fichier est là, lisible, et namsdec le relit comme n'importe
    // quelle description écrite à la main.
    let écrit = dossier.path().join("namsdec/decompilers/rizin-ghidra.json");
    assert!(écrit.is_file(), "{} manque", écrit.display());
    let relu = namsdec_disasm::plugin::read(&écrit).expect("la description est valide");
    assert_eq!(relu.command, "rizin");
    assert!(
        relu.args.iter().any(|a| a.contains("pdg")),
        "c'est `pdg` qui décompile : {:?}",
        relu.args
    );

    // Et le premier outil déclaré devient celui qu'on lancera : une
    // liste d'un seul élément ne devrait pas s'ouvrir sur « choisir… ».
    with_data_home(dossier.path(), || {
        app.decompilers = load_installed_decompilers();
    });
    assert_eq!(app.decompiler, "rizin-ghidra");

    // Le retrait enlève la description, pas le programme.
    with_data_home(dossier.path(), || app.undeclare_tool("rizin-ghidra"));
    assert!(!écrit.exists(), "la description s'en va");
    assert!(app.decompiler.is_empty() || app.decompiler != "rizin-ghidra");
}

/// namsdec dit ce que la machine a, et ce qu'elle n'a pas.
///
/// Proposer un outil sans dire s'il est là laisse découvrir son absence
/// au moment de s'en servir — c'est-à-dire au pire moment. Et Rizin
/// mérite un mot de plus : le programme peut être installé sans que la
/// décompilation le soit, faute du greffon qui l'apporte.
#[test]
fn the_tools_say_whether_this_machine_has_them() {
    use namsdec_disasm::known::Availability;
    let outils = namsdec_disasm::known::all();
    assert!(outils.len() >= 5, "{}", outils.len());

    for outil in &outils {
        let dispo = outil.availability();
        let trouvé = namsdec_disasm::known::program_path(&outil.plugin.command).is_some();
        if !trouvé {
            assert_eq!(
                dispo,
                Availability::Missing,
                "{} n'est pas installé ici, il faut le dire",
                outil.plugin.id
            );
        }
        // Et ce qui est absent dit toujours comment l'obtenir.
        if dispo == Availability::Missing {
            assert!(outil.plugin.install.is_some(), "{}", outil.plugin.id);
        }
    }

    // Rizin est le seul dont la présence ne suffit pas : sans
    // `rz-ghidra`, `pdg` n'existe pas, et l'outil est dit incomplet.
    let rizin = outils
        .iter()
        .find(|outil| outil.plugin.id == "rizin-ghidra")
        .expect("Rizin");
    assert!(
        rizin.needs_fr.is_some() && rizin.needs_en.is_some(),
        "la pièce manquante doit être nommée dans les deux langues"
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
                && peint.contains(strings(Lang::Fr).dialog_quit),
            "OK et Quitter doivent se lire sur chaque page :\n{peint}"
        );
    }
}

/// L'onglet Outils montre tous les outils, et non le premier.
///
/// Ils tenaient sur quatre lignes chacun : seul objdump entrait dans la
/// fenêtre, et Rizin et RetDec étaient là sans que personne puisse les
/// voir. Une ligne par outil, le détail au survol — et l'on trouve ce
/// qu'on est venu chercher.
#[test]
fn the_tools_tab_shows_every_tool_not_just_the_first() {
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Tools,
        ..Default::default()
    };
    let peint = paint(&mut app);
    for attendu in ["objdump", "ndisasm", "radare2", "Rizin", "RetDec"] {
        assert!(
            peint.contains(attendu),
            "« {attendu} » doit se lire dans l'onglet Outils :\n{peint}"
        );
    }
    // Et ce qu'il faut taper pour obtenir ce qui manque.
    //
    // L'état est SEMÉ, non lu sur la machine. Il était lu, et le test
    // disait alors deux choses différentes selon le poste : sur celui où
    // tous les outils sont bâtis il ne vérifiait rien, et sur un poste
    // où le premier outil absent tombait en bas de liste il échouait
    // pour une question de place et non de contenu.
    //
    // Semé APRÈS une première image : ouvrir les préférences jette le
    // relevé par conception — on a pu installer un outil à la main dans
    // un terminal entre-temps.
    let premier = namsdec_disasm::known::all()
        .first()
        .expect("au moins un outil connu")
        .plugin
        .id
        .clone();
    app.tools_ready = namsdec_disasm::known::all()
        .into_iter()
        .map(|outil| {
            let dispo = if outil.plugin.id == premier {
                namsdec_disasm::known::Availability::Missing
            } else {
                namsdec_disasm::known::Availability::Ready
            };
            (outil.plugin.id, dispo)
        })
        .collect();
    let peint = paint(&mut app);
    assert!(
        peint.contains(strings(Lang::Fr).tools_get_it),
        "la façon d'obtenir un outil absent doit se lire :\n{peint}"
    );
    // Les deux états sont dits séparément : « absent » parle du
    // programme, « namsdec s'en sert » parle de namsdec. Les confondre
    // était toute la confusion de l'ancienne page.
    //
    // Les deux états sont SEMÉS, un pour chaque moitié de la liste : le
    // panneau doit écrire les deux phrases en même temps, et c'est
    // précisément ce qu'on veut vérifier — que « absent de la machine »
    // et « installé sur la machine » ne se confondent pas.
    //
    // C'était lu sur la machine, et alors la moitié des assertions ne
    // s'exécutait pas : depuis que namsdec retrouve ce qu'il a installé
    // lui-même, un poste bien équipé n'a plus un seul outil absent, et le
    // test ne vérifiait plus qu'une phrase sur deux.
    let s_fr = strings(Lang::Fr);
    let connus = namsdec_disasm::known::all();
    app.tools_ready = connus
        .iter()
        .enumerate()
        .map(|(rang, outil)| {
            let dispo = if rang % 2 == 0 {
                namsdec_disasm::known::Availability::Ready
            } else {
                namsdec_disasm::known::Availability::Missing
            };
            (outil.plugin.id.clone(), dispo)
        })
        .collect();
    assert!(connus.len() >= 2, "il faut deux outils pour deux états");
    let peint = paint(&mut app);
    assert!(peint.contains(s_fr.tools_unused), "{peint}");
    for attendu in [s_fr.tools_state_found, s_fr.tools_state_absent] {
        assert!(peint.contains(attendu), "« {attendu} » :\n{peint}");
    }
    // Et les boutons de la boîte restent visibles : la page ne prend
    // plus toute la hauteur de l'écran.
    assert!(
        peint.contains(strings(Lang::Fr).dialog_ok)
            && peint.contains(strings(Lang::Fr).dialog_quit),
        "OK et Quitter doivent rester à l'écran :\n{peint}"
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

/// Les pastilles tiennent le coin, et rien ne les bouscule.
///
/// « Intelligemment à gauche » veut dire trois choses : la fermeture
/// occupe le coin — l'angle qu'une souris lancée sans viser atteint —,
/// les menus commencent après elles sans les toucher, et le nom du
/// fichier reste au centre de la barre plutôt que collé aux pastilles.
#[test]
fn the_window_buttons_hold_the_corner_without_crowding_the_menus() {
    let mut app = NamsdecApp::default();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };

    let mut pastilles: Vec<f32> = Vec::new();
    let mut menu_x = f32::MAX;
    let mut titre_x = 0.0f32;
    for _ in 0..2 {
        pastilles.clear();
        menu_x = f32::MAX;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Circle(c) if c.radius > 5.0 && c.radius < 9.0 => {
                    pastilles.push(c.center.x);
                }
                egui::epaint::Shape::Text(t) => {
                    if t.galley.text() == strings(Lang::Fr).menu_file {
                        menu_x = menu_x.min(t.pos.x);
                    }
                    if t.galley.text() == strings(Lang::Fr).untitled_buffer && t.pos.y < 40.0 {
                        titre_x = t.pos.x;
                    }
                }
                _ => {}
            }
        }
        output.textures_delta.clear();
    }
    pastilles.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(pastilles.len(), 3, "{pastilles:?}");

    // La fermeture au coin, sans y être collée.
    assert!(
        (6.0..40.0).contains(&pastilles[0]),
        "la première pastille tient le coin : {pastilles:?}"
    );
    // Le menu Fichier commence après la dernière pastille, et ne la
    // touche pas : deux cibles voisines se manquent l'une l'autre.
    assert!(
        menu_x > pastilles[2] + 8.0,
        "les menus doivent laisser de l'air : menu à {menu_x}, pastilles {pastilles:?}"
    );
    // Et le nom du fichier reste au milieu de la barre, loin des deux.
    assert!(
        titre_x > 1280.0 * 0.3,
        "le nom du fichier reste centré : {titre_x}"
    );
}

/// Le répertoire `plugins/` dit la vérité sur ce que namsdec lance.
///
/// Ses fichiers sont engendrés par le code qui connaît ces outils : si
/// l'un des deux changeait sans l'autre, le dépôt proposerait de copier
/// une commande que namsdec ne lance pas — c'est-à-dire un mensonge dans
/// le seul endroit où l'on ne peut pas se le permettre, puisqu'il s'agit
/// d'exécuter un programme.
#[test]
fn the_plugins_directory_matches_what_namsdec_would_run() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let plugins = racine.join("plugins");

    // Les deux familles, et leur mode d'emploi.
    for (dossier, attendu) in [
        ("themes", "Préférences ▸ Plugins"),
        ("fonctionnalites", "rz-ghidra"),
    ] {
        let readme = plugins.join(dossier).join("README.md");
        let texte = std::fs::read_to_string(&readme)
            .unwrap_or_else(|_| panic!("{} manque", readme.display()));
        assert!(
            texte.contains(attendu),
            "{} doit expliquer ce qu'on y range",
            readme.display()
        );
    }
    let sommaire = std::fs::read_to_string(plugins.join("README.md")).expect("plugins/README.md");
    assert!(
        sommaire.contains("Thèmes") && sommaire.contains("Fonctionnalités"),
        "le sommaire nomme les deux familles"
    );

    // Chaque outil connu a son fichier, et ce fichier est exactement ce
    // que namsdec lancerait.
    for outil in namsdec_disasm::known::all() {
        let chemin = plugins
            .join("fonctionnalites")
            .join(format!("{}.json", outil.plugin.id));
        let écrit = namsdec_disasm::plugin::read(&chemin)
            .unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
        assert_eq!(
            écrit,
            outil.plugin,
            "{} ne décrit pas ce que namsdec lance",
            chemin.display()
        );
    }

    // Et les thèmes proposés se lisent vraiment.
    let mut thèmes = 0;
    for entrée in std::fs::read_dir(plugins.join("themes")).expect("le dossier des thèmes") {
        let chemin = entrée.expect("une entrée").path();
        if chemin.extension().is_some_and(|e| e == "json") {
            namsdec_theme::plugin::read(&chemin)
                .unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
            thèmes += 1;
        }
    }
    assert!(
        thèmes >= 2,
        "au moins deux thèmes d'exemple, obtenu {thèmes}"
    );
}

/// Les instructions d'installation des décompilateurs sont là, et
/// complètes.
///
/// C'est la seule étape que namsdec ne fait pas à la place de
/// l'utilisateur : autant qu'elle soit écrite en entier, pour les trois
/// outils qui demandent un geste.
#[test]
fn the_decompilers_come_with_their_installation_instructions() {
    let readme = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/fonctionnalites/README.md");
    let texte = std::fs::read_to_string(&readme).expect("le mode d'emploi des outils");

    for attendu in [
        // Ce qu'il faut taper, pour chacun.
        "dnf install radare2",
        "rizin-plugin-ghidra",
        "rz-pm -i rz-ghidra",
        "retdec/releases",
        // Et les deux marqueurs du format, sans lesquels on ne peut pas
        // écrire sa propre description.
        "{file}",
        "{out}",
    ] {
        assert!(
            texte.contains(attendu),
            "le mode d'emploi doit contenir « {attendu} »"
        );
    }
    // Le piège de Rizin est nommé : le programme seul ne décompile pas.
    assert!(
        texte.contains("Rizin seul ne décompile pas"),
        "la nuance qui coûte une soirée doit être dite"
    );
}

/// Aller chercher un outil ne se fait pas tout seul, et jamais sans
/// l'avoir montré.
///
/// Ce module-ci ne se contente pas de lancer un programme installé : il
/// **télécharge du code et l'exécute**. Les verrous sont donc plus
/// nombreux, et ce test les compte : le module de décompilation d'abord,
/// celui-ci ensuite, la fenêtre qui montre le plan, et seulement alors
/// un bouton.
#[test]
fn fetching_a_tool_never_happens_by_itself() {
    let app = NamsdecApp::default();
    assert!(
        !app.show_installer_module,
        "éteint tant que personne ne l'allume"
    );
    assert!(app.install_job.is_none() && app.install_plan.is_none());

    // Demander le plan sans avoir allumé le module ne montre rien.
    let mut app = NamsdecApp {
        install_plan: Some("retdec".to_owned()),
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        !peint.contains(strings(Lang::Fr).install_title),
        "la fenêtre reste fermée :\n{peint}"
    );
    assert!(app.install_job.is_none(), "et rien n'a été lancé");

    // Module allumé : la fenêtre montre le plan — d'où ça vient, ce que
    // ça coûte, où ça va, et chaque commande.
    let mut app = NamsdecApp {
        show_decompiler_module: true,
        show_installer_module: true,
        install_plan: Some("retdec".to_owned()),
        ..Default::default()
    };
    let peint = paint(&mut app);
    for attendu in [
        // La leçon d'abord : à quoi sert un décompilateur.
        strings(Lang::Fr).install_what_for,
        "il ne restitue pas la source",
        // Puis les faits. Le coût est une phrase longue : s'il manque,
        // c'est qu'il file hors de la fenêtre au lieu de se replier.
        strings(Lang::Fr).install_cost,
        "180 Mo à télécharger",
        strings(Lang::Fr).install_where,
        // L'avertissement, tout en bas et toujours peint.
        strings(Lang::Fr).install_warning,
        strings(Lang::Fr).install_plan,
        "github.com/avast/retdec",
        "sha256sum",
    ] {
        assert!(
            peint.contains(attendu),
            "« {attendu} » doit se lire avant tout lancement :\n{peint}"
        );
    }
    // Le bouton de lancement, ou le constat que c'est déjà fait. L'état
    // est SEMÉ, non lu sur la machine : le lire faisait dire au test deux
    // choses différentes selon le poste, et il ne vérifiait donc rien de
    // stable. Semé APRÈS une première image, parce qu'ouvrir la fenêtre
    // jette le relevé des outils par conception — et que l'image suivante
    // le refait en lançant vraiment l'outil.
    for (état, attendu) in [
        (
            namsdec_disasm::known::Availability::Ready,
            strings(Lang::Fr).install_already_done,
        ),
        (
            namsdec_disasm::known::Availability::Missing,
            strings(Lang::Fr).install_start,
        ),
    ] {
        app.tools_ready = [("retdec".to_owned(), état)].into_iter().collect();
        let peint = paint(&mut app);
        assert!(
            peint.contains(attendu),
            "{état:?} : « {attendu} » doit se lire avant tout lancement :\n{peint}"
        );
        assert!(
            app.install_job.is_none(),
            "montrer le plan n'est pas l'exécuter"
        );
    }
}

/// Ce que les recettes s'interdisent, et qui compte plus que ce
/// qu'elles font.
#[test]
fn a_recipe_stays_inside_namsdec_and_shows_everything() {
    let data = std::path::Path::new("/tmp/namsdec-essai");
    for outil in ["retdec", "rizin-ghidra"] {
        let recette = namsdec_disasm::install::recipe_for(outil, data).expect("une recette existe");

        // Tout ce qui est produit reste sous le dossier de namsdec :
        // rien dans /usr, rien qui demande des droits.
        assert!(
            recette
                .produces
                .starts_with(namsdec_disasm::install::tools_dir(data)),
            "{} sort du dossier de namsdec",
            recette.produces.display()
        );
        for étape in &recette.steps {
            assert_ne!(étape.command, "sudo", "{}", étape.line());
            // Chaque étape sait dire ce qu'elle fait, dans les deux
            // langues : une commande sans explication n'est pas montrée,
            // elle est infligée.
            assert!(
                !étape.about_fr.is_empty() && !étape.about_en.is_empty(),
                "{} n'explique pas ce qu'elle fait",
                étape.line()
            );
        }
    }

    // Et ce qu'on ne sait pas installer ne s'invente pas.
    assert!(
        namsdec_disasm::install::recipe_for("objdump", data).is_none(),
        "objdump vient des paquets du système, pas de namsdec"
    );
}

/// Le module d'installation n'alourdit pas le programme.
///
/// C'était la condition posée : pouvoir aller chercher Rizin et RetDec
/// sans que namsdec grossisse. La réponse tient dans une règle — **rien
/// n'est embarqué**. Les téléchargements passent par `curl`,
/// l'extraction par `tar`, la compilation par `meson` et `cmake` : ce
/// sont des commandes du système, exactement comme `nasm` et `ld`.
/// Aucune bibliothèque de plus, donc aucun octet de plus que le code qui
/// décrit les recettes.
#[test]
fn installing_tools_adds_no_dependency_to_the_binary() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    // Aucune crate de téléchargement, d'archive ou de compilation n'a
    // rejoint le programme.
    let workspace = std::fs::read_to_string(racine.join("Cargo.toml")).expect("Cargo.toml");
    for interdite in [
        "reqwest", "hyper", "flate2", "tar =", "zip", "xz2", "git2", "cmake", "curl =",
    ] {
        assert!(
            !workspace.contains(interdite),
            "« {interdite} » alourdirait le binaire : le module doit passer par les \
             commandes du système"
        );
    }

    // Et les recettes n'appellent que des programmes du système.
    let data = std::path::Path::new("/tmp/x");
    let mut appelés: std::collections::BTreeSet<String> = Default::default();
    for outil in ["retdec", "rizin-ghidra"] {
        let recette = namsdec_disasm::install::recipe_for(outil, data).expect("une recette");
        for étape in &recette.steps {
            appelés.insert(étape.command.clone());
        }
    }
    for commande in &appelés {
        assert!(
            [
                "mkdir",
                "curl",
                "sha256sum",
                "tar",
                "rm",
                "git",
                "meson",
                "ninja",
                "cmake",
                "env"
            ]
            .contains(&commande.as_str()),
            "« {commande} » n'est pas un outil du système attendu"
        );
    }
}

/// Compile un petit programme C, et rend son binaire.
///
/// `None` quand la machine n'a pas de quoi — pas de `gcc`, pas de
/// bibliothèque C statique. Ce n'est pas un échec de namsdec, et le test
/// qui s'en sert se tait alors plutôt que d'échouer ailleurs.
fn compile_c(source: &str, options: &[&str]) -> Option<(tempfile::TempDir, std::path::PathBuf)> {
    let dir = tempfile::tempdir().ok()?;
    let chemin = dir.path().join("essai.c");
    std::fs::write(&chemin, source).ok()?;
    let binaire = dir.path().join("essai");
    let sortie = std::process::Command::new("gcc")
        .args(options)
        .arg("-O1")
        .arg("-o")
        .arg(&binaire)
        .arg(&chemin)
        .output()
        .ok()?;
    if !sortie.status.success() {
        eprintln!(
            "gcc n'a pas pu compiler ({}) : test ignoré",
            String::from_utf8_lossy(&sortie.stderr)
                .lines()
                .next()
                .unwrap_or("")
        );
        return None;
    }
    Some((dir, binaire))
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
    // ouvrir un répertoire demanderait un système de fichiers.
    let Some((_dir, binaire)) = compile_c(
        "#include <dirent.h>\nint main(void) { return opendir(\"/\") == 0; }\n",
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
        message.contains("opendir"),
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
    for absente in ["opendir", "fork", "socket", "dlopen"] {
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
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
                ..Default::default()
            };
            assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
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

/// Les paires se ferment toutes seules — et se taisent quand il le faut.
///
/// C'est la moitié difficile : un éditeur qui ferme tout, tout le temps,
/// se fait détester en une journée. On mesure donc autant ce qu'il ne
/// fait pas que ce qu'il fait.
#[test]
fn pairs_close_themselves_but_know_when_to_keep_quiet() {
    // Le curseur est noté « | » : la fonction reçoit sa position en
    // octets, et le texte sans la barre.
    let cas = |texte: &str| -> PairEdit {
        let at = texte.find('|').expect("le curseur est noté |");
        pair_edit(&texte.replace('|', ""), at)
    };

    // Ce qu'elle fait.
    assert_eq!(cas("    mov rax, [|"), PairEdit::Close(']'));
    assert_eq!(cas("%macro f (|"), PairEdit::Close(')'));
    assert_eq!(cas("msg db \"|"), PairEdit::Close('"'));

    // Ce qu'elle ne fait pas, et c'est le plus important.
    assert_eq!(
        cas("    ; l'|adresse"),
        PairEdit::None,
        "une apostrophe après une lettre est une apostrophe"
    );
    assert_eq!(
        cas("    mov rax, [|rsi]"),
        PairEdit::None,
        "on n'ouvre pas une paire devant un mot"
    );

    // Traverser ce qui est déjà là, plutôt que de l'écrire deux fois.
    assert_eq!(cas("    mov rax, [rsi]|]"), PairEdit::StepOver);
    assert_eq!(
        cas("msg db \"salut\"|\""),
        PairEdit::StepOver,
        "un guillemet est son propre fermant : il faut pouvoir en sortir"
    );

    // Et l'effacement emporte la paire vide qu'on vient d'ouvrir.
    let vide = "    mov rax, []";
    assert!(backspace_clears_pair(vide, vide.find(']').unwrap()));
    let pleine = "    mov rax, [rsi]";
    assert!(
        !backspace_clears_pair(pleine, pleine.find('r').unwrap() + 1),
        "mais pas une paire qui contient quelque chose"
    );
}

/// Aucun cas ne fait paniquer, y compris sur du texte accentué.
///
/// Les positions sont en octets et « é » en prend deux : une borne prise
/// au milieu d'un caractère ferait tomber l'éditeur pendant qu'on écrit
/// un commentaire.
#[test]
fn the_pair_rules_never_panic_mid_character() {
    for texte in ["", "é", "; drapeau à zéro", "«»", "[é]", "'é'"] {
        for at in 0..=texte.len() {
            let _ = pair_edit(texte, at);
            let _ = backspace_clears_pair(texte, at);
        }
    }
}

/// La calculatrice s'ouvre, calcule, et montre toutes les bases.
///
/// Le test regarde ce qui est **peint** : l'évaluateur est déjà couvert
/// au caractère près ailleurs, ce qu'on vérifie ici est que la fenêtre
/// montre bien ce qu'il rend.
#[test]
fn the_calculator_shows_every_reading_of_what_is_typed() {
    let mut app = NamsdecApp {
        show_calculator: true,
        calc_input: "0xff & 0x0f".to_owned(),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
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

    // 0xff & 0x0f fait 15 : on doit le lire dans chaque base, sans avoir
    // à cliquer quoi que ce soit.
    for attendu in ["0xf", "15", "0o17", "00001111"] {
        assert!(
            peint.contains(attendu),
            "« {attendu} » manque dans la calculatrice :\n{peint}"
        );
    }
}

/// Une expression fautive se dit en clair, et ne fait rien tomber.
#[test]
fn the_calculator_names_what_went_wrong() {
    let ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    // Chacune des fautes qu'on fait en tapant, y compris les états
    // intermédiaires d'une frappe.
    for (expression, attendu) in [
        ("1 / 0", strings(Lang::Fr).calc_divide_by_zero),
        ("(1", strings(Lang::Fr).calc_unbalanced),
        ("1 +", strings(Lang::Fr).calc_missing),
    ] {
        let mut app = NamsdecApp {
            show_calculator: true,
            calc_input: expression.to_owned(),
            ..Default::default()
        };
        let mut peint = String::new();
        for _ in 0..2 {
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
        assert!(
            peint.contains(attendu),
            "« {expression} » devrait dire « {attendu} » :\n{peint}"
        );
    }

    // Et une fenêtre vide ne reproche rien : elle vient de s'ouvrir.
    let mut app = NamsdecApp {
        show_calculator: true,
        ..Default::default()
    };
    let mut peint = String::new();
    for _ in 0..2 {
        peint.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                peint.push_str(texte.galley.text());
            }
        }
        sortie.textures_delta.clear();
    }
    assert!(
        !peint.contains(strings(Lang::Fr).calc_missing),
        "rien de tapé, rien à reprocher :\n{peint}"
    );
}

/// Sur un écran ordinaire, la palette défile — et c'est le bon repli.
///
/// Elle porte cinquante-deux commandes : aucune fenêtre courante ne les
/// tient toutes. Le test voisin vérifie qu'on ne défile pas *quand il y a
/// la place* ; celui-ci vérifie l'autre moitié, qui est le cas réel :
/// quand il n'y en a pas, la liste défile au lieu de couper la fin.
#[test]
fn on_a_real_screen_the_palette_scrolls_rather_than_cutting_the_end() {
    let mut app = NamsdecApp {
        command_palette: Some(CommandPalette::default()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    // La hauteur d'un écran d'ordinateur portable ordinaire.
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };

    let s = strings(Lang::Fr);
    let mut première = None;
    let mut fond = 0.0f32;
    for _ in 0..3 {
        première = None;
        fond = 0.0;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                if texte.galley.text() == s.action_assemble {
                    première = Some(texte.pos);
                }
                fond = fond.max(texte.pos.y);
            }
        }
        sortie.textures_delta.clear();
    }

    // La première commande est là : la liste commence bien au début.
    assert!(première.is_some(), "la première commande doit être visible");
    // Et rien n'est peint sous le bord de l'écran — ce qui ne tient pas
    // est derrière la barre de défilement, non coupé dans le vide.
    assert!(
        fond <= 900.0,
        "quelque chose est peint sous le bord de l'écran, en y={fond}"
    );
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

/// On retrouve le travail commencé sur un binaire au lancement suivant.
///
/// Les sources se rouvraient depuis longtemps ; un binaire, non — et
/// c'est pourtant là qu'il y a du travail à perdre. Rouvrir namsdec
/// demandait de retrouver le fichier, de le recharger, de reposer ses
/// points d'arrêt et de retaper ses arguments.
#[test]
fn a_binary_session_comes_back_at_the_next_launch() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let dossier = tempfile::tempdir().unwrap();
    let binaire = dossier.path().join("crackme-01");
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let dossier = tempfile::tempdir().unwrap();
    let binaire = dossier.path().join("crackme-01");
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

/// Dupliquer une ligne la recopie dessous, et descend dessus.
///
/// Le geste qu'on fait sans y penser dans tout éditeur, et qui manque le
/// plus en assembleur : quatre lignes qui ne diffèrent que par un
/// registre se dupliquent plus vite qu'elles ne se retapent.
#[test]
fn duplicating_a_line_copies_it_below() {
    let source = "mov rax, 1\nmov rbx, 2\nmov rcx, 3\n";
    // Le curseur au milieu de la deuxième ligne.
    let au_milieu = source.find("rbx").unwrap() + 1;
    let (après, curseur) = duplicated_line(source, au_milieu);
    assert_eq!(après, "mov rax, 1\nmov rbx, 2\nmov rbx, 2\nmov rcx, 3\n");
    // Le curseur a gardé sa colonne, une ligne plus bas.
    assert_eq!(
        après[..curseur].lines().count(),
        3,
        "le curseur est sur la copie : {curseur}"
    );
    let colonne_avant = au_milieu - source[..au_milieu].rfind('\n').map_or(0, |i| i + 1);
    let colonne_après = curseur - après[..curseur].rfind('\n').map_or(0, |i| i + 1);
    assert_eq!(colonne_avant, colonne_après, "et sa colonne");
}

/// La dernière ligne d'un fichier sans retour final se duplique aussi.
///
/// C'est le cas qui casse une implémentation naïve : sans ajouter le
/// retour à la ligne, la copie se colle derrière l'original et n'en fait
/// qu'une seule, deux fois plus longue.
#[test]
fn the_last_line_duplicates_even_without_a_trailing_newline() {
    let source = "mov rax, 1\nhlt";
    let (après, _) = duplicated_line(source, source.len());
    assert_eq!(après, "mov rax, 1\nhlt\nhlt");

    // Et un fichier vide, ou d'une seule ligne, ne fait rien tomber.
    assert_eq!(duplicated_line("", 0).0, "\n");
    assert_eq!(duplicated_line("hlt", 0).0, "hlt\nhlt");
}

/// Aucune position ne fait paniquer, y compris au milieu d'un accent.
#[test]
fn duplicating_never_panics_mid_character() {
    for source in ["", "\n", "é", "; drapeau à zéro\nhlt", "a\né\nb"] {
        for at in 0..=source.len() {
            // Les positions au milieu d'un caractère sont bornées par la
            // fonction elle-même : ce qu'on vérifie est qu'aucune ne
            // tombe.
            if !source.is_char_boundary(at) {
                continue;
            }
            let (après, curseur) = duplicated_line(source, at);
            assert!(curseur <= après.len(), "{source:?} à {at}");
        }
    }
}

/// Mais pas sur un listing venu d'un binaire.
///
/// Celui-ci est reconstruit, et y insérer une ligne décalerait toutes les
/// correspondances ligne <-> adresse dont dépendent la gouttière, les
/// points d'arrêt et le suivi de RIP.
#[test]
fn a_binary_listing_does_not_duplicate_lines() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"));
    let avant = app.source.clone();
    app.cursor_byte = 40;
    app.duplicate_line();
    assert_eq!(app.source, avant, "le listing ne doit pas bouger");
}

/// « Il lui manque… » ne s'écrit que s'il manque vraiment quelque chose.
///
/// Le message était affiché sans condition, si bien qu'un outil installé
/// et parfaitement fonctionnel s'annonçait cassé : « programme détecté »
/// et « namsdec s'en sert » d'un côté, « Il lui manque le greffon
/// rz-ghidra » juste en dessous. On en concluait, très logiquement, que
/// l'installation n'avait pas marché.
#[test]
fn a_working_tool_is_not_announced_as_missing_something() {
    use namsdec_disasm::known::Availability;

    let s = strings(Lang::Fr);
    // L'état est SEMÉ, et les deux cas sont éprouvés. Il était lu sur la
    // machine, et le test ne jugeait alors qu'un seul des deux — celui
    // que le poste lui présentait. Sur une machine où rizin fonctionne,
    // le cas « incomplet » — le seul qui nous intéressait — n'était
    // jamais vérifié.
    for (état, doit_le_dire) in [
        (Availability::Ready, false),
        (Availability::Incomplete, true),
    ] {
        let mut app = NamsdecApp {
            show_preferences: true,
            preferences_tab: PreferencesTab::Tools,
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

        // Première image : ouvrir les préférences jette le relevé des
        // outils par conception, et l'image suivante le refait en lançant
        // vraiment rizin. Ce qu'on sème avant serait donc perdu.
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();

        // Tous les outils prêts sauf rizin, dont l'état est celui qu'on
        // éprouve : un autre outil absent écrirait « Il lui manque » de
        // son côté, et l'on accuserait rizin.
        app.tools_ready = namsdec_disasm::known::all()
            .into_iter()
            .map(|outil| {
                let dispo = if outil.plugin.id == "rizin-ghidra" {
                    état
                } else {
                    Availability::Ready
                };
                (outil.plugin.id, dispo)
            })
            .collect();

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

        if doit_le_dire {
            assert!(
                peint.contains(s.tools_needs),
                "le greffon manque vraiment : il faut le dire.\n{peint}"
            );
        } else {
            assert!(
                !peint.contains(s.tools_needs),
                "rizin+rz-ghidra fonctionne : rien ne doit dire qu'il manque \
                 quelque chose.\n{peint}"
            );
        }
    }
}

/// La fenêtre du plan d'installation reste entièrement à l'écran.
///
/// Le plan de rizin compte neuf étapes et un journal : la fenêtre
/// dépassait en haut comme en bas, et sa barre de titre — avec la croix
/// qui s'y trouve — passait hors champ. On ne pouvait plus la fermer du
/// tout.
#[test]
fn the_install_plan_window_never_runs_off_the_screen() {
    for (largeur, hauteur) in [(1280.0, 800.0), (1999.0, 1248.0)] {
        let mut app = NamsdecApp {
            show_installer_module: true,
            install_plan: Some("rizin-ghidra".to_owned()),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(largeur, hauteur));
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        };
        let mut haut = f32::MAX;
        let mut bas = f32::MIN;
        for _ in 0..3 {
            haut = f32::MAX;
            bas = f32::MIN;
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    haut = haut.min(texte.pos.y);
                    bas = bas.max(texte.pos.y);
                }
            }
            sortie.textures_delta.clear();
        }
        // La fenêtre n'est peut-être pas ouverte : sans dossier de
        // données, le plan se referme tout seul. Le test le dit plutôt
        // que d'échouer sur l'état du poste.
        if app.install_plan.is_none() {
            return;
        }
        assert!(
            haut >= 0.0,
            "{largeur}x{hauteur} : quelque chose est peint au-dessus de l'écran (y={haut})"
        );
        assert!(
            bas <= hauteur,
            "{largeur}x{hauteur} : quelque chose est peint sous l'écran (y={bas})"
        );
    }
}

/// Échap referme le plan, et le bouton « Fermer » aussi.
///
/// La croix vit dans la barre de titre, qu'on perd de vue sur un plan de
/// neuf étapes. Il faut un second chemin — et le geste qu'on fait sans y
/// penser devant une fenêtre dont on a fini, c'est Échap.
#[test]
fn escape_closes_the_install_plan() {
    let mut app = NamsdecApp {
        show_installer_module: true,
        install_plan: Some("rizin-ghidra".to_owned()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
    let mut sortie = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        },
        |ui| app.draw(ui),
    );
    sortie.textures_delta.clear();
    if app.install_plan.is_none() {
        return;
    }

    let mut échap = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    échap.events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(échap, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    assert!(
        app.install_plan.is_none(),
        "Échap doit refermer le plan d'installation"
    );
}

/// Ce que la fenêtre d'installation propose répond à l'état de l'outil.
///
/// Trois états, trois offres, et pas une de plus :
///
/// - il répond → **Fermer**, et rien d'autre. « Recommencer » était là,
///   et c'était une offre sans motif : vingt minutes de compilation pour
///   un résultat identique. Un bouton qu'on n'a aucune raison de cliquer
///   fait douter de celui d'à côté — il laissait croire que
///   l'installation avait peut-être échoué.
/// - il est là mais incomplet → **Réparer**. C'est le cas de rizin sans
///   `rz-ghidra` : `pdg` n'existe pas, et rien dans la fenêtre ne disait
///   quoi faire.
/// - il n'est pas là → **Lancer l'installation**.
///
/// L'état est SEMÉ, et semé **après la première image**. L'ouverture de
/// la fenêtre jette le relevé par conception — on a pu installer l'outil
/// à la main dans un terminal entre-temps —, et l'image suivante le
/// refait en lançant vraiment rizin. Semer avant ne servait donc à rien :
/// le test lisait la machine malgré lui, passait ici parce que rizin y
/// est installé, et échouait sur la CI qui ne l'a pas. Quatre CI rouges
/// d'affilée, sans qu'une ligne de namsdec soit en cause.
#[test]
fn what_the_installer_window_offers_answers_the_tools_state() {
    use namsdec_disasm::known::Availability;

    let s = strings(Lang::Fr);
    for (état, attendus, absents) in [
        (
            Availability::Ready,
            vec![s.install_already_done, s.dialog_close],
            vec![s.install_start, s.install_repair],
        ),
        (
            Availability::Incomplete,
            vec![s.install_incomplete, s.install_repair, s.dialog_close],
            vec![s.install_start, s.install_already_done],
        ),
        (
            Availability::Missing,
            vec![s.install_start],
            vec![s.install_already_done, s.install_repair],
        ),
    ] {
        let mut app = NamsdecApp {
            show_installer_module: true,
            install_plan: Some("rizin-ghidra".to_owned()),
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

        // Première image : la fenêtre s'ouvre, et vide le relevé.
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
        if app.install_plan.is_none() {
            // Pas de dossier de données sur cette machine : la fenêtre se
            // referme d'elle-même, et il n'y a rien à juger.
            return;
        }

        // Maintenant on sème, et plus rien ne le jettera : la fenêtre
        // n'est plus « nouvellement ouverte ».
        app.tools_ready = [("rizin-ghidra".to_owned(), état)].into_iter().collect();

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

        for attendu in attendus {
            assert!(
                peint.contains(attendu),
                "{état:?} : « {attendu} » doit être proposé.\n{peint}"
            );
        }
        for absent in absents {
            assert!(
                !peint.contains(absent),
                "{état:?} : « {absent} » n'a rien à faire là.\n{peint}"
            );
        }
    }
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

/// Les deux dossiers de fiches sont bien deux, et le README le dit.
///
/// namsdec lit `disassemblers/` et `decompilers/` séparément : les deux
/// ne répondent pas à la même question, et une fiche rangée dans le
/// mauvais dossier est simplement ignorée — sans message, puisque namsdec
/// ne sait pas qu'elle existe. Le mode d'emploi n'en annonçait qu'un, ce
/// qui fait perdre une soirée à qui écrit son premier outil.
#[test]
fn the_readme_names_both_plugin_directories() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sommaire =
        std::fs::read_to_string(racine.join("plugins/README.md")).expect("plugins/README.md");
    // Les noms exacts que le code emploie, relevés ici même : si l'un
    // change, ce test le rappelle.
    for dossier in ["disassemblers/", "decompilers/", "themes/", "exercises/"] {
        assert!(
            sommaire.contains(dossier),
            "le mode d'emploi doit nommer « {dossier} » : c'est là que namsdec regarde"
        );
    }
    // Et le code lit bien ces deux-là, sous ces noms.
    let data = data_dir().expect("un dossier de données");
    assert_eq!(
        decompilers_dir().expect("decompilers"),
        data.join("decompilers")
    );
    assert!(
        data.join("disassemblers").ends_with("disassemblers"),
        "c'est le dossier que load_disassembler_plugins lit"
    );
}

/// Un désassembleur maison déclaré devient un moteur de listing.
///
/// C'est tout le point du format : DZF est écrit à la main, dans un autre
/// dépôt, en C++ sans dépendance — et namsdec doit pouvoir s'en servir
/// sans rien savoir de lui d'autre que la commande à lancer.
#[test]
fn a_home_made_disassembler_can_become_the_listing_engine() {
    let fiche = data_dir()
        .map(|dir| dir.join("disassemblers/dzf.json"))
        .filter(|p| p.is_file());
    // DZF vit ailleurs : sur une machine qui ne l'a pas déclaré, il n'y a
    // rien à vérifier, et le test le dit plutôt que d'échouer.
    let Some(fiche) = fiche else { return };

    let plugin = namsdec_disasm::plugin::read(&fiche).expect("la fiche déclarée doit être lisible");
    assert_eq!(plugin.id, "dzf");
    // Le programme qu'elle nomme doit exister, sans quoi la déclarer
    // n'aurait servi qu'à promettre.
    let programme = std::path::Path::new(&plugin.command);
    if !programme.is_file() {
        return;
    }
    // Et il rend un listing sur les octets d'une vraie fonction.
    let octets: &[u8] = &[0x55, 0x48, 0x89, 0xe5, 0x5d, 0xc3];
    let listing = plugin.run(octets).expect("DZF doit répondre");
    for attendu in ["push rbp", "mov rbp, rsp", "pop rbp", "ret"] {
        assert!(
            listing.contains(attendu),
            "« {attendu} » manque :\n{listing}"
        );
    }
}

/// Surligner les occurrences : la marque est peinte, en vert du thème.
///
/// Le test regarde les rectangles peints, et non l'état interne : ce
/// qu'on promet est de *voir* où le mot revient.
#[test]
fn highlighting_matches_paints_them_in_the_theme_green() {
    let mut app = NamsdecApp {
        source: "\
_start:
    mov rax, 1
    add rax, rbx
    hlt
"
        .to_owned(),
        editor_mode: EditorMode::Insert,
        ..Default::default()
    };
    // Le curseur sur le premier `rax`.
    app.cursor_byte = app.source.find("rax").unwrap() + 1;
    app.highlight_matches();

    assert_eq!(
        app.tracked_spans.len(),
        2,
        "deux `rax` : {:?}",
        app.tracked_spans
    );
    assert!(
        matches!(app.status, Status::Counted(2, _)),
        "la barre d'état dit combien : {:?}",
        app.status
    );

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let vert = app.theme.good;
    let mut marques = 0usize;
    for _ in 0..3 {
        marques = 0;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Rect(rect) = &clipped.shape {
                // Le liseré, qui porte la couleur pleine du thème.
                if rect.stroke.color == vert {
                    marques += 1;
                }
            }
        }
        sortie.textures_delta.clear();
    }
    assert_eq!(
        marques, 2,
        "les deux occurrences doivent être peintes en vert du thème"
    );
}

/// La même touche allume et éteint, et Échap efface.
#[test]
fn the_same_key_lights_and_clears_the_highlighting() {
    let mut app = NamsdecApp {
        source: "    mov rax, 1\n    add rax, rbx\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = app.source.find("rax").unwrap() + 1;

    app.highlight_matches();
    assert!(app.tracked.is_some());
    // Refait sur le même mot : on éteint, comme un interrupteur.
    app.highlight_matches();
    assert!(app.tracked.is_none(), "la même touche doit éteindre");
    assert!(app.tracked_spans.is_empty());

    // Et sur un mot où il n'y a rien à suivre, on le dit.
    app.cursor_byte = 1; // un blanc en tête de ligne
    app.highlight_matches();
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
}

/// Le surlignage suit le texte quand on tape.
///
/// Les étendues sont des positions en octets : taper une lettre plus
/// haut décale tout ce qui suit, et garder les anciennes surlignerait le
/// milieu des mots.
#[test]
fn the_highlighting_follows_the_text_as_it_is_typed() {
    let mut app = NamsdecApp {
        source: "    mov rax, 1\n    add rax, rbx\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = app.source.find("rax").unwrap() + 1;
    app.highlight_matches();
    let avant = app.tracked_spans.clone();
    assert_eq!(avant.len(), 2);

    // Une ligne insérée au-dessus : tout ce qui suit se décale.
    app.source.insert_str(0, "; un commentaire\n");
    app.refresh_matches();
    assert_eq!(app.tracked_spans.len(), 2, "toujours deux");
    for étendue in &app.tracked_spans {
        assert_eq!(
            &app.source[étendue.clone()],
            "rax",
            "chaque marque doit tomber sur un vrai `rax`, non au milieu d'un mot"
        );
    }
    assert_ne!(app.tracked_spans, avant, "et les positions ont bougé");
}

/// Renommer toutes les occurrences d'un coup.
///
/// C'est ce que le multicurseur sert à faire neuf fois sur dix : changer
/// un registre partout, renommer une étiquette. egui ne tient qu'un seul
/// curseur — impossible d'en poser plusieurs —, mais l'édition
/// elle-même, on peut la faire, et c'est elle qu'on voulait.
#[test]
fn renaming_changes_every_match_at_once() {
    let mut app = NamsdecApp {
        source: "\
boucle:
    add rax, 1
    dec rcx
    jnz boucle
"
        .to_owned(),
        ..Default::default()
    };
    app.cursor_byte = 2; // sur `boucle:`
    app.highlight_matches();
    assert_eq!(app.tracked_spans.len(), 2, "{:?}", app.tracked_spans);

    app.start_rename();
    assert_eq!(
        app.rename.as_deref(),
        Some("boucle"),
        "le champ s'ouvre sur le nom actuel"
    );
    app.apply_rename("encore");

    assert!(app.source.contains("encore:"), "{}", app.source);
    assert!(app.source.contains("jnz encore"), "{}", app.source);
    assert!(
        !app.source.contains("boucle"),
        "plus aucun ancien nom : {}",
        app.source
    );
    assert!(
        matches!(app.status, Status::Counted(2, _)),
        "{:?}",
        app.status
    );
    // Et le surlignage suit le nouveau nom : le voir confirme que rien
    // n'a été oublié.
    assert_eq!(app.tracked_spans.len(), 2);
    for étendue in &app.tracked_spans {
        assert_eq!(&app.source[étendue.clone()], "encore");
    }
}

/// Renommer un registre ne touche pas au texte des commentaires.
///
/// C'est la différence avec un chercher-remplacer, et c'est tout
/// l'intérêt : le surlignage passe par la découpe du code, donc le `rax`
/// d'un commentaire français n'en fait pas partie.
#[test]
fn renaming_leaves_comments_alone() {
    let mut app = NamsdecApp {
        source: "    mov rax, 1   ; rax reçoit un\n    add rax, rbx\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = app.source.find("rax").unwrap() + 1;
    app.highlight_matches();
    assert_eq!(app.tracked_spans.len(), 2, "deux `rax` de code");

    app.apply_rename("rdx");
    assert!(
        app.source.contains("; rax reçoit un"),
        "le commentaire n'est pas du code : {}",
        app.source
    );
    assert!(app.source.contains("mov rdx, 1"), "{}", app.source);
    assert!(app.source.contains("add rdx, rbx"), "{}", app.source);
}

/// Renommer sans rien avoir surligné surligne d'abord.
///
/// Un geste rendu plutôt qu'un refus : on est sur le mot, on demande à le
/// renommer, c'est de celui-là qu'on parle.
#[test]
fn renaming_without_a_selection_highlights_first() {
    let mut app = NamsdecApp {
        source: "    mov rax, 1\n    add rax, rbx\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = app.source.find("rax").unwrap() + 1;
    app.start_rename();
    assert_eq!(app.rename.as_deref(), Some("rax"));
    assert_eq!(app.tracked_spans.len(), 2);

    // Mais sur un blanc, il n'y a rien à renommer, et on le dit.
    let mut app = NamsdecApp {
        source: "    mov rax, 1\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = 1;
    app.start_rename();
    assert!(app.rename.is_none());
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
}

/// Une touche passe à l'occurrence suivante, et la sélectionne.
///
/// C'est le geste qui manque le plus quand on ne peut pas poser
/// plusieurs curseurs : on ne les a pas tous en même temps, mais on va
/// de l'un à l'autre sans quitter le clavier, et chacun arrive
/// sélectionné — donc prêt à être remplacé.
#[test]
fn one_key_walks_from_match_to_match() {
    let mut app = NamsdecApp {
        source: "    mov rax, 1\n    add rax, rbx\n    xor rax, rax\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = 0;
    app.highlight_matches();
    // Le curseur est en tête de ligne : rien à suivre là, on prend donc
    // le premier mot suivi explicitement.
    app.cursor_byte = app.source.find("rax").unwrap();
    app.highlight_matches();
    let total = app.tracked_spans.len();
    assert_eq!(total, 4, "quatre `rax` : {:?}", app.tracked_spans);

    // On avance : chaque saut doit tomber sur une occurrence, et la
    // sélectionner.
    let mut vus = Vec::new();
    for _ in 0..total {
        app.go_to_match(true);
        let sélection = app
            .pending_selection
            .clone()
            .expect("aller à une occurrence doit la sélectionner");
        assert_eq!(
            &app.source[sélection.clone()],
            "rax",
            "la sélection doit tomber sur le mot, non à côté"
        );
        vus.push(sélection.start);
    }
    // Les quatre, chacune une fois, puis on revient au début.
    let mut triés = vus.clone();
    triés.sort_unstable();
    triés.dedup();
    assert_eq!(triés.len(), total, "chaque occurrence une fois : {vus:?}");

    // Et en arrière, on repart en sens inverse.
    app.cursor_byte = app.source.len();
    app.go_to_match(false);
    let dernière = app.pending_selection.clone().unwrap();
    assert_eq!(dernière.start, *triés.last().unwrap());
}

/// La boucle est circulaire, dans les deux sens.
///
/// Buter contre la fin obligerait à remonter à la main pour repasser sur
/// ce qu'on vient de dépasser.
#[test]
fn walking_past_the_last_match_comes_back_to_the_first() {
    let mut app = NamsdecApp {
        source: "    mov rax, 1\n    add rax, rbx\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = app.source.find("rax").unwrap();
    app.highlight_matches();
    let premières = app.tracked_spans.clone();
    assert_eq!(premières.len(), 2);

    // Curseur après la dernière : la suivante est la première.
    app.cursor_byte = app.source.len();
    app.go_to_match(true);
    assert_eq!(
        app.pending_selection.clone().unwrap().start,
        premières[0].start
    );

    // Curseur avant la première : la précédente est la dernière.
    app.cursor_byte = 0;
    app.go_to_match(false);
    assert_eq!(
        app.pending_selection.clone().unwrap().start,
        premières[1].start
    );
}

/// Sans rien de surligné, la touche surligne d'abord puis avance.
///
/// Répondre « il n'y a rien » à quelqu'un qui a le mot sous les yeux
/// serait lui demander un geste de plus pour rien.
#[test]
fn walking_without_a_highlight_starts_one() {
    let mut app = NamsdecApp {
        source: "    mov rax, 1\n    add rax, rbx\n".to_owned(),
        ..Default::default()
    };
    app.cursor_byte = app.source.find("rax").unwrap() + 1;
    assert!(app.tracked.is_none());
    app.go_to_match(true);
    assert!(app.tracked.is_some(), "la touche doit surligner d'abord");
    assert!(app.pending_selection.is_some(), "puis avancer");
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"));
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

/// Toute boîte de dialogue répond à Entrée et à Échap.
///
/// Elles ne se ressemblaient pas — OK et Quitter ici, Fermer et
/// Recommencer là, une croix seule ailleurs — et aucune n'écoutait le
/// clavier. Or c'est l'exception qui fait chercher : une barre d'action
/// se trouve d'autant plus vite qu'elle est au même endroit partout.
#[test]
fn a_dialog_bar_answers_enter_and_escape() {
    let theme = namsdec_theme::by_id("dark-plus");
    let ctx = egui::Context::default();
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));

    // Entrée vaut le bouton principal.
    let mut entrée = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    entrée.events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut réponse = None;
    let mut sortie = ctx.run_ui(entrée, |ui| {
        réponse = dialog_bar(ui, theme, "OK", Some("Annuler"));
    });
    sortie.textures_delta.clear();
    assert_eq!(réponse, Some(Answer::Ok), "Entrée doit valider");

    // Échap vaut l'autre.
    let mut échap = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    échap.events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut réponse = None;
    let mut sortie = ctx.run_ui(échap, |ui| {
        réponse = dialog_bar(ui, theme, "OK", Some("Annuler"));
    });
    sortie.textures_delta.clear();
    assert_eq!(réponse, Some(Answer::Cancel), "Échap doit annuler");

    // Et Échap ferme même une boîte sans second bouton : fermer ce qui
    // ne fait que renseigner n'annule rien, mais doit rester possible
    // d'une touche.
    let mut échap = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    échap.events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut réponse = None;
    let mut sortie = ctx.run_ui(échap, |ui| {
        réponse = dialog_bar(ui, theme, "OK", None);
    });
    sortie.textures_delta.clear();
    assert_eq!(réponse, Some(Answer::Cancel));
}

/// Sans touche ni clic, la barre ne répond rien.
///
/// Une barre qui rendrait `Ok` par défaut fermerait la boîte dès la
/// première image, et personne n'aurait rien vu.
#[test]
fn a_dialog_bar_says_nothing_until_asked() {
    let theme = namsdec_theme::by_id("dark-plus");
    let ctx = egui::Context::default();
    let mut réponse = Some(Answer::Ok);
    let mut sortie = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 400.0),
            )),
            ..Default::default()
        },
        |ui| {
            réponse = dialog_bar(ui, theme, "OK", Some("Annuler"));
        },
    );
    sortie.textures_delta.clear();
    assert_eq!(réponse, None);
}

/// Les deux boutons ont la même largeur, où qu'ils soient.
///
/// Deux boutons de tailles différentes côte à côte se lisent comme une
/// hiérarchie qui n'existe pas : c'est la couleur qui dit lequel est le
/// principal, pas la taille.
#[test]
fn the_two_dialog_buttons_share_a_width() {
    let theme = namsdec_theme::by_id("dark-plus");
    let ctx = egui::Context::default();
    let mut largeurs: Vec<f32> = Vec::new();
    for _ in 0..2 {
        largeurs.clear();
        let mut sortie = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 400.0),
                )),
                ..Default::default()
            },
            |ui| {
                let _ = dialog_bar(ui, theme, "OK", Some("Quitter"));
            },
        );
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Rect(rect) = &clipped.shape
                && rect.rect.height() > 10.0
                && rect.rect.width() > 40.0
            {
                largeurs.push(rect.rect.width());
            }
        }
        sortie.textures_delta.clear();
    }
    assert!(
        largeurs.len() >= 2,
        "les deux boutons sont peints : {largeurs:?}"
    );
    let plus_petit = largeurs.iter().copied().fold(f32::MAX, f32::min);
    let plus_grand = largeurs.iter().copied().fold(0.0f32, f32::max);
    assert!(
        (plus_grand - plus_petit).abs() < 1.0,
        "même largeur attendue : {largeurs:?}"
    );
}

/// Les mouvements par mot, comme dans vim.
///
/// C'est ce qu'on réclame en premier avec l'habitude de vim : avancer
/// caractère par caractère dans `mov qword [rbp-0x18], rax` use la
/// patience. Et la notion de mot est celle de vim, non celle d'un
/// traitement de texte — `[rbp-0x18]` compte pour plusieurs mots, ce qui
/// est exactement ce qu'il faut pour atteindre le déplacement sans viser
/// à la souris.
#[test]
fn word_moves_follow_vims_idea_of_a_word() {
    let ligne = "mov qword [rbp-0x18], rax";
    // `w` depuis le début : chaque saut tombe sur un début de mot ou de
    // ponctuation, et jamais au milieu.
    let mut at = 0;
    let mut étapes = Vec::new();
    for _ in 0..12 {
        let suivant = move_cursor(ligne, at, Move::WordForward);
        if suivant == at {
            break;
        }
        at = suivant;
        étapes.push(&ligne[at.min(ligne.len())..]);
    }
    assert!(
        étapes.iter().any(|reste| reste.starts_with("qword")),
        "`w` doit s'arrêter sur « qword » : {étapes:?}"
    );
    assert!(
        étapes.iter().any(|reste| reste.starts_with('[')),
        "et sur le crochet, qui est une ponctuation : {étapes:?}"
    );
    assert!(
        étapes.iter().any(|reste| reste.starts_with("rbp")),
        "et sur « rbp » : {étapes:?}"
    );

    // `b` revient sur ses pas.
    let depuis_la_fin = move_cursor(ligne, ligne.len(), Move::WordBack);
    assert_eq!(&ligne[depuis_la_fin..], "rax", "`b` recule d'un mot");

    // `e` va à la fin du mot, et répété, avance.
    let fin_du_premier = move_cursor(ligne, 0, Move::WordEnd);
    assert_eq!(&ligne[..fin_du_premier + 1], "mov");
    let fin_du_second = move_cursor(ligne, fin_du_premier, Move::WordEnd);
    assert!(fin_du_second > fin_du_premier, "`e` répété doit avancer");
}

/// Le début et la fin du fichier, et aucune position qui déborde.
#[test]
fn file_start_and_end_never_overshoot() {
    let source = "mov rax, 1\nadd rax, rbx\nhlt\n";
    assert_eq!(move_cursor(source, 20, Move::FileStart), 0);
    assert_eq!(
        move_cursor(source, 0, Move::FileEnd),
        source.chars().count()
    );

    // Et sur un texte accentué, on compte en caractères — jamais en
    // octets, sinon le curseur dériverait d'un cran par accent.
    let accentué = "; drapeau à zéro\n    hlt\n";
    for mouvement in [
        Move::WordForward,
        Move::WordBack,
        Move::WordEnd,
        Move::FileStart,
        Move::FileEnd,
    ] {
        for at in 0..=accentué.chars().count() {
            let cible = move_cursor(accentué, at, mouvement);
            assert!(
                cible <= accentué.chars().count(),
                "{mouvement:?} depuis {at} sort du texte : {cible}"
            );
        }
    }
}

/// `gg` demande ses deux frappes, et rien d'autre ne l'arme.
#[test]
fn the_double_g_needs_both_presses() {
    let mut app = NamsdecApp {
        source: "mov rax, 1\nadd rax, rbx\nhlt\n".to_owned(),
        editor_mode: EditorMode::Normal,
        ..Default::default()
    };
    assert!(!app.g_pending, "rien d'armé au départ");

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0));
    let frappe = |key: egui::Key, shift: bool| egui::RawInput {
        screen_rect: Some(écran),
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: if shift {
                egui::Modifiers::SHIFT
            } else {
                egui::Modifiers::NONE
            },
        }],
        ..Default::default()
    };

    // Un `g` arme, sans bouger.
    let mut sortie = ctx.run_ui(frappe(egui::Key::G, false), |ui| app.draw(ui));
    sortie.textures_delta.clear();
    assert!(app.g_pending, "le premier `g` attend le second");

    // `G` (avec Maj) est un autre geste : la fin du fichier.
    let mut app2 = NamsdecApp {
        source: app.source.clone(),
        editor_mode: EditorMode::Normal,
        ..Default::default()
    };
    let ctx2 = egui::Context::default();
    app2.apply_theme(&ctx2);
    let mut sortie = ctx2.run_ui(frappe(egui::Key::G, true), |ui| app2.draw(ui));
    sortie.textures_delta.clear();
    assert!(
        !app2.g_pending,
        "`G` ne doit pas armer une séquence : c'est déjà un mouvement"
    );
}

/// Un diagnostic déjà affiché suit la langue quand on la change.
///
/// Ils gardaient celle où ils étaient nés : basculer en anglais laissait
/// du français dans l'interface, et les retraduire aurait demandé de
/// réassembler. Ceux qui viennent de namsdec portent maintenant leur
/// motif et non leur phrase, laquelle se reforme au dessin.
#[test]
fn a_diagnostic_already_shown_follows_the_language() {
    // Un programme dont l'exécution sort par la fin : le lint le dit.
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rax, 1
"
        .to_owned(),
        lang: Lang::Fr,
        ..Default::default()
    };
    app.do_assemble();
    let avec_motif: Vec<_> = app
        .diagnostics
        .iter()
        .filter(|d| d.kind.is_some())
        .collect();
    // Sans nasm sur la machine, rien n'a été assemblé : le test se tait.
    if avec_motif.is_empty() {
        return;
    }

    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| -> String {
        app.console_tab = ConsoleTab::Diagnostics;
        let mut peint = String::new();
        for _ in 0..2 {
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
        peint
    };

    let en_français = peindre(&mut app);
    assert!(
        en_français.contains("l'exécution peut sortir"),
        "le diagnostic doit se lire en français :\n{en_français}"
    );
    let _ = s;

    // On bascule, **sans réassembler** : c'est tout le point.
    app.lang = Lang::En;
    let en_anglais = peindre(&mut app);
    assert!(
        en_anglais.contains("execution can run off the end"),
        "et se relire en anglais sans réassembler :\n{en_anglais}"
    );
    assert!(
        !en_anglais.contains("l'exécution peut sortir"),
        "sans laisser du français derrière :\n{en_anglais}"
    );
}

/// Les messages de nasm, eux, ne se retraduisent pas — et c'est dit.
///
/// Ils arrivent rédigés, en anglais. Les réécrire demanderait de
/// comprendre ce que nasm a voulu dire, ce qui n'est pas le travail de
/// namsdec : il les montre tels quels, et explique à côté quand il sait.
#[test]
fn a_message_from_nasm_carries_no_pattern() {
    let venu_de_nasm = namsdec_asm::Diagnostic {
        line: 3,
        severity: namsdec_asm::Severity::Error,
        message: "symbol `foo' undefined".to_owned(),
        kind: None,
    };
    assert!(
        venu_de_nasm.kind.is_none(),
        "un message rédigé ailleurs n'a pas de motif à reformer"
    );
}

/// Le graphe nomme la fonction quand le binaire porte ses symboles.
///
/// Il n'en donnait que l'adresse : « vérifie_mdp » se retient, `0x1d491`
/// non. L'adresse reste affichée à côté — c'est elle qu'on tape dans
/// « Comment arrive-t-on à », et elle lève l'ambiguïté quand la
/// fermeture par sauts se trompe, faute de symboles.
#[test]
fn the_flow_window_names_the_function_it_shows() {
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    call vérifie
    hlt
vérifie:
    mov rax, 1
    ret
"
        .to_owned(),
        show_flow: true,
        flow_current_only: true,
        ..Default::default()
    };
    app.do_assemble();
    // Sans nasm, rien à vérifier.
    let Some(état) = app.assemble_state.as_ref() else {
        return;
    };
    if état.output.symbols.is_empty() {
        return;
    }

    // Le symbole se retrouve bien à son adresse : c'est ce dont dépend
    // l'affichage.
    let vérifie = état
        .output
        .symbols
        .iter()
        .find(|symbole| symbole.name == "vérifie");
    let Some(vérifie) = vérifie else {
        return;
    };
    assert_eq!(
        symbol_at(&état.output.symbols, vérifie.addr).as_deref(),
        Some("vérifie"),
        "l'adresse de la fonction doit rendre son nom"
    );

    // Et au milieu de la fonction, le nom porte son décalage : on sait
    // où l'on est, et dans quoi.
    let dedans = symbol_at(&état.output.symbols, vérifie.addr + 1);
    assert!(
        dedans
            .as_deref()
            .is_some_and(|nom| nom.starts_with("vérifie+")),
        "au milieu, le nom porte son décalage : {dedans:?}"
    );
}

/// Nommer une adresse, et retrouver ce nom au lancement suivant.
///
/// Le geste central de la rétro-ingénierie : on passe une soirée à
/// comprendre que `0x1d491` vérifie le mot de passe, et sans cela il n'y
/// a plus que `0x1d491` le lendemain.
#[test]
fn a_name_given_to_an_address_comes_back_later() {
    let dossier = tempfile::tempdir().unwrap();
    let octets: &[u8] = b"\x7fELF un binaire d'essai";
    let empreinte = notes::fingerprint(octets);

    let mut notes = notes::Notes::default();
    notes.set_name(0x1d491, "vérifie_mdp");
    notes.set_comment(0x1d4a0, "compare octet par octet");
    notes::save(dossier.path(), &empreinte, &notes).unwrap();

    // Le même contenu, sous un autre nom de fichier : les notes suivent.
    let relues = notes::load(dossier.path(), &notes::fingerprint(octets));
    assert_eq!(relues.name(0x1d491), Some("vérifie_mdp"));
    assert_eq!(relues.comment(0x1d4a0), Some("compare octet par octet"));

    // Un binaire recompilé, lui, n'hérite de rien : ses notes
    // désigneraient d'autres instructions, ce qui est pire que rien.
    let recompilé = notes::load(dossier.path(), &notes::fingerprint(b"\x7fELF autre chose"));
    assert!(
        recompilé.is_empty(),
        "un autre contenu ne doit pas hériter des notes"
    );
}

/// Le nom posé à la main l'emporte sur le symbole du binaire.
///
/// C'est toute la raison de l'avoir écrit : quelqu'un qui renomme
/// `sub_1d491` en `vérifie_mdp` veut voir le second.
#[test]
fn a_hand_written_name_wins_over_the_symbol() {
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rax, 1
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    let Some(état) = app.assemble_state.as_ref() else {
        return;
    };
    let Some(start) = état
        .output
        .symbols
        .iter()
        .find(|symbole| symbole.name == "_start")
        .map(|symbole| symbole.addr)
    else {
        return;
    };

    let symboles = app
        .assemble_state
        .as_ref()
        .map(|état| état.output.symbols.clone())
        .unwrap_or_default();

    // Le symbole, d'abord.
    assert_eq!(
        named_at(&app.notes, &symboles, start).as_deref(),
        Some("_start")
    );

    // Puis le nom posé.
    app.notes.set_name(start, "le_vrai_début");
    assert_eq!(
        named_at(&app.notes, &symboles, start).as_deref(),
        Some("le_vrai_début"),
        "le nom écrit à la main doit primer"
    );

    // Retiré, on retombe sur le symbole — et non sur rien.
    app.notes.set_name(start, "");
    assert_eq!(
        named_at(&app.notes, &symboles, start).as_deref(),
        Some("_start")
    );
}

/// Nommer une ligne qui n'a pas d'adresse le dit, plutôt que d'ouvrir un
/// champ qui n'écrirait nulle part.
#[test]
fn naming_a_line_without_an_address_says_so() {
    let mut app = NamsdecApp {
        source: "; rien que du commentaire\n".to_owned(),
        ..Default::default()
    };
    app.cursor_pos = (1, 1);
    app.start_note(NoteKind::Name);
    assert!(app.note_edit.is_none(), "aucun champ ne doit s'ouvrir");
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
}

/// Échap ne déclenche jamais l'action secondaire d'une fenêtre.
///
/// La barre de dialogue répond `Cancel` à Échap. Tant que le second
/// bouton est un renoncement, c'est juste — mais la fenêtre
/// d'installation y avait mis « Recommencer », si bien qu'Échap lançait
/// une recompilation de vingt minutes au lieu de fermer.
///
/// La règle est donc : la barre porte le geste principal et le
/// renoncement, jamais une troisième action. Celle-ci se pose à côté et
/// se clique exprès.
#[test]
fn escape_never_triggers_a_second_action() {
    let mut app = NamsdecApp {
        show_installer_module: true,
        install_plan: Some("rizin-ghidra".to_owned()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1100.0));
    let mut sortie = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        },
        |ui| app.draw(ui),
    );
    sortie.textures_delta.clear();
    if app.install_plan.is_none() {
        return;
    }

    let mut échap = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    échap.events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(échap, |ui| app.draw(ui));
    sortie.textures_delta.clear();

    assert!(app.install_plan.is_none(), "Échap referme");
    assert!(
        app.install_job.is_none(),
        "et surtout ne lance rien : une recompilation de vingt minutes \
         ne se déclenche pas par la touche qui veut dire « je m'en vais »"
    );
}

/// Toute fenêtre qui s'ouvre porte un bouton pour la fermer.
///
/// La croix de la barre de titre ne suffit pas : elle est petite, elle
/// est loin quand la fenêtre est haute, et il lui est arrivé de sortir
/// de l'écran. Un bouton dans le corps de la fenêtre se trouve toujours,
/// et il se trouve au même endroit partout.
#[test]
fn every_window_carries_a_way_out() {
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        // Sans binaire d'épreuve, plusieurs fenêtres restent vides : on
        // ne peut pas juger.
        return;
    };
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1900.0, 1200.0));
    let entrée = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };

    /// Comment ouvrir une fenêtre, et comment l'appeler dans l'échec.
    type Ouverture = (&'static str, fn(&mut NamsdecApp));

    // Chaque fenêtre, ouverte seule : deux fenêtres à la fois se
    // recouvrent, et l'on ne saurait pas laquelle porte le bouton vu.
    let fenêtres: [Ouverture; 9] = [
        ("calculatrice", |app| app.show_calculator = true),
        ("graphe", |app| app.show_flow = true),
        ("désassemblage", |app| app.show_disassembly = true),
        ("motif cyclique", |app| app.show_cyclic = true),
        ("chronologie", |app| app.show_timeline = true),
        ("points d'arrêt", |app| app.show_breakpoints = true),
        ("analyse", |app| app.show_analysis = true),
        ("registres étendus", |app| app.show_vectors = true),
        ("aide", |app| app.show_help = true),
    ];

    for (nom, ouvrir) in fenêtres {
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
        ouvrir(&mut app);

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
        assert!(
            peint.contains(s.dialog_ok),
            "la fenêtre « {nom} » doit porter un bouton « {} » :\n{peint}",
            s.dialog_ok
        );
    }
}

/// Échap quitte le mode Insertion sans rien annoncer d'autre.
///
/// Il effaçait le surlignage — ce qui est juste — mais l'annonçait :
/// « Surlignage effacé » s'écrivait dans la barre d'état au moment où
/// l'on y cherchait « NORMAL ». On appuie sur Échap pour changer de
/// mode ; lui faire dire autre chose répond à côté de ce qu'on a fait.
#[test]
fn escape_leaves_insert_mode_without_announcing_anything_else() {
    let mut app = NamsdecApp {
        source: "    mov rax, 1\n    add rax, rbx\n".to_owned(),
        editor_mode: EditorMode::Insert,
        ..Default::default()
    };
    app.cursor_byte = app.source.find("rax").unwrap() + 1;
    app.highlight_matches();
    assert!(app.tracked.is_some(), "quelque chose est surligné");

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0));
    let mut échap = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    échap.events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(échap, |ui| app.draw(ui));
    sortie.textures_delta.clear();

    assert_eq!(app.editor_mode, EditorMode::Normal, "on quitte l'insertion");
    assert!(app.tracked.is_none(), "et le surlignage s'éteint");
    assert!(
        !matches!(app.status, Status::Note(_)),
        "mais rien ne l'annonce : {:?}",
        app.status
    );

    // Refaire la commande, en revanche, le dit : là, c'est le geste
    // demandé, et le retour compte.
    app.highlight_matches();
    app.highlight_matches();
    assert!(
        matches!(app.status, Status::Note(_)),
        "l'interrupteur, lui, annonce : {:?}",
        app.status
    );
}

/// `y y` duplique la ligne, au clavier et pour de vrai.
///
/// L'action existait et marchait ; le raccourci ne l'atteignait pas, et
/// personne ne pouvait le savoir sans essayer. Le test part donc du
/// clavier, non de l'action.
#[test]
fn typing_y_y_duplicates_the_line() {
    let mut app = NamsdecApp {
        source: "mov rax, 1\nmov rbx, 2\n".to_owned(),
        editor_mode: EditorMode::Normal,
        ..Default::default()
    };
    app.cursor_byte = 3;
    app.cursor_pos = (1, 4);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0));
    // Deux frappes, comme on les taperait — la seconde arrivant marquée
    // « répétition », ce qu'egui fait de deux pressions rapprochées.
    for répétition in [false, true] {
        let mut entrée = egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        };
        entrée.events.push(egui::Event::Key {
            key: egui::Key::Y,
            physical_key: None,
            pressed: true,
            repeat: répétition,
            modifiers: egui::Modifiers::NONE,
        });
        let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }

    assert_eq!(
        app.source, "mov rax, 1\nmov rax, 1\nmov rbx, 2\n",
        "la ligne doit être recopiée dessous"
    );
}

// ============================================================
//  Le banc de mise en page
// ============================================================
//
// Quatre défauts de suite ont été trouvés par l'œil et non par la suite
// de tests : des textes qui se recouvraient, une fenêtre qui montait
// toute seule, un champ de saisie absent, une colonne centrée au lieu
// d'être alignée. Aucun n'était subtil ; tous étaient visibles à la
// première capture d'écran. Ce qui manquait n'était pas des tests — il
// y en a des centaines — mais des tests qui regardent la *mise en
// page* au lieu de regarder un détail choisi d'avance.
//
// D'où ce banc : une liste de fenêtres, déclarée une fois, et des
// invariants qui valent pour toutes. Ajouter une fenêtre à la liste la
// soumet d'un coup à tout ce qui suit ; et un invariant nouveau
// s'applique aussitôt aux quatorze.

/// Comment ouvrir une fenêtre : son nom dans l'échec, son identifiant
/// egui, et ce qu'il faut poser dans l'application pour qu'elle
/// s'affiche.
type Ouverture = (&'static str, &'static str, fn(&mut NamsdecApp));

/// Les fenêtres qu'un démarrage nu sait ouvrir.
///
/// Une seule liste, et non une par test : c'est ce qui fait qu'une
/// fenêtre nouvelle est contrôlée sans qu'on y pense. Celles qui
/// demandent un binaire chargé ou une exécution en cours n'y sont pas —
/// elles se contrôlent séparément, avec ce qu'il leur faut.
fn fenêtres_ouvrables() -> Vec<Ouverture> {
    vec![
        ("préférences", "preferences_window", |app| {
            app.show_preferences = true;
        }),
        ("à propos", ABOUT_WINDOW, |app| app.show_about = true),
        ("calculatrice", "calculator_window", |app| {
            app.show_calculator = true;
        }),
        ("aide", "instruction_help", |app| app.show_help = true),
        ("registres étendus", "vectors_window", |app| {
            app.show_vectors = true;
        }),
        ("points d'arrêt", "breakpoints_window", |app| {
            app.show_breakpoints = true;
        }),
        ("chronologie", "timeline_window", |app| {
            app.show_timeline = true;
        }),
        ("motif cyclique", "cyclic_window", |app| {
            app.show_cyclic = true
        }),
        ("analyse", "analysis_window", |app| app.show_analysis = true),
        ("désassemblage", "disassembly_window", |app| {
            app.show_disassembly = true;
        }),
        ("graphe", "flow_window", |app| app.show_flow = true),
        ("décompilateur", "decompiler_window", |app| {
            app.show_decompiler_module = true;
            app.show_decompiler = true;
        }),
        ("saisie du programme", INPUT_DIALOG, |app| {
            app.show_input_dialog = true;
        }),
        ("correctifs", PATCHES_WINDOW, |app| app.show_patches = true),
    ]
}

/// Une scène à contrôler : son nom, la fenêtre où regarder (`None`
/// pour l'application entière), et ce qu'il faut poser pour l'obtenir.
///
/// Les fenêtres ne sont pas tout : la barre d'outils, le panneau
/// latéral et la barre d'état sont sous les yeux en permanence, et
/// c'est là qu'on passe ses journées. Une règle qui ne vaudrait que
/// pour les fenêtres laisserait l'essentiel hors contrôle.
type Scène = (String, Option<&'static str>, Box<dyn Fn(&mut NamsdecApp)>);

/// Le programme du banc : court, et il touche à tout ce que
/// l'interface sait montrer.
///
/// Deux sections, un appel système, une pile qu'on remue, une boucle.
/// Assemblé et avancé de quelques pas, il remplit les registres, la
/// pile, le listing, le graphe et la chronologie — c'est-à-dire tout
/// ce qui reste vide dans une application qu'on vient d'ouvrir, et
/// donc tout ce que le banc ne regardait pas.
const PROGRAMME_DU_BANC: &str = "\
section .data
message db \"bonjour\", 10
longueur equ $ - message

section .text
global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, message
    mov rdx, longueur
    syscall
    push rax
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    pop rax
    mov rax, 60
    xor rdi, rdi
    syscall
";

/// Tout ce que le banc regarde : les quatorze fenêtres et les cinq
/// onglets du panneau, à vide puis programme en marche.
///
/// « En marche » n'est pas un raffinement : une application qu'on vient
/// d'ouvrir n'affiche presque rien, et les défauts de mise en page se
/// révèlent avec le contenu. Cette moitié-là du banc demande nasm ;
/// faute de lui, elle est simplement absente, et le reste tient.
fn scènes() -> Vec<Scène> {
    let mut scènes: Vec<Scène> = Vec::new();
    let en_marche = namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker();
    for marche in [false, true] {
        if marche && !en_marche {
            continue;
        }
        let suffixe = if marche { " (en marche)" } else { "" };
        /// Pose le programme du banc et avance de quelques pas.
        fn démarre(app: &mut NamsdecApp) {
            app.source = PROGRAMME_DU_BANC.to_owned();
            app.do_assemble();
            for _ in 0..8 {
                app.do_step();
            }
        }
        for (nom, id, ouvre) in fenêtres_ouvrables() {
            scènes.push((
                format!("{nom}{suffixe}"),
                Some(id),
                Box::new(move |app| {
                    if marche {
                        démarre(app);
                    }
                    ouvre(app);
                }),
            ));
        }
        for onglet in SidebarTab::ALL {
            scènes.push((
                format!("application / {}{suffixe}", onglet.id()),
                None,
                Box::new(move |app| {
                    if marche {
                        démarre(app);
                    }
                    app.show_sidebar = true;
                    app.active_tab = onglet;
                }),
            ));
        }
    }
    scènes
}

/// L'écran du banc : assez grand pour qu'aucune fenêtre n'ait à se
/// serrer, et fixe, pour que deux exécutions disent la même chose.
fn écran_du_banc() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    }
}

/// Ce qu'une fenêtre écrit, ligne par ligne, et l'encre que chaque
/// ligne occupe vraiment.
///
/// Trois précautions, et chacune vient d'une erreur commise :
///
/// * **Le rectangle, non la position.** Un test qui compare des
///   positions laisse passer un texte qui déborde — c'est ce qui est
///   arrivé au champ de saisie coupé.
/// * **La ligne, non la galley.** Un texte de trois lignes a un cadre
///   qui couvre toute sa largeur, y compris le blanc au bout de la
///   dernière ligne ; deux de ces cadres se croisent sans qu'aucune
///   lettre ne se touche.
/// * **Sans l'espace de tête.** Dans une rangée qui se replie, egui
///   pose la galley au *début* de la rangée et décale son premier mot
///   à l'intérieur. Mesurée depuis son origine, une étiquette paraît
///   alors recouvrir celle qui la précède, alors qu'à l'écran les deux
///   se suivent. On demande donc à egui l'encre, et non le cadre.
///
/// `images` dit combien de fois dessiner avant de relever — une fenêtre
/// dont la hauteur épouse son contenu ne se pose qu'à la troisième.
fn ce_qui_est_écrit(
    ctx: &egui::Context,
    app: &mut NamsdecApp,
    id: Option<&str>,
    images: usize,
) -> (Option<egui::Rect>, Vec<(String, egui::Rect)>) {
    let entrée = écran_du_banc();
    let mut lignes: Vec<(String, egui::Rect)> = Vec::new();
    for _ in 0..images {
        lignes.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        let cadre =
            id.and_then(|id| egui::AreaState::load(ctx, egui::Id::new(id)).map(|a| a.rect()));
        for clipped in &sortie.shapes {
            // Ce que la fenêtre peint *elle-même*, et non ce qui se
            // trouve sous elle. Sans ce tri, un registre caché derrière
            // la fenêtre d'aide passait pour un recouvrement — et cela
            // n'arrivait qu'une fois un programme chargé, c'est-à-dire
            // jamais dans les tests d'alors.
            //
            // Le tri se fait sur la LARGEUR du rectangle de découpe. Une
            // fenêtre découpe son contenu à sa propre largeur ; un
            // panneau, lui, est découpé à celle de l'écran ou part de
            // son bord — jamais à l'intérieur d'une fenêtre. La hauteur
            // ne dirait rien : egui laisse au découpage d'une fenêtre
            // plus de hauteur qu'elle n'en occupe.
            if cadre.is_some_and(|c| {
                let large = c.x_range().expand(2.0);
                !large.contains(clipped.clip_rect.left())
                    || !large.contains(clipped.clip_rect.right())
            }) {
                continue;
            }
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                for ligne in &t.galley.rows {
                    if ligne.glyphs.is_empty() {
                        continue;
                    }
                    // Ce qui se voit, et non ce qui est peint : une
                    // ligne sortie d'un ascenseur est bien dans la
                    // liste des formes, mais rognée à néant.
                    let encre = ligne
                        .rect_without_leading_space()
                        .translate(t.pos.to_vec2())
                        .intersect(clipped.clip_rect);
                    if encre.width() <= 0.0 || encre.height() <= 0.0 {
                        continue;
                    }
                    lignes.push((t.galley.text().to_owned(), encre));
                }
            }
        }
        sortie.textures_delta.clear();
    }
    let cadre = id.map(|id| {
        egui::AreaState::load(ctx, egui::Id::new(id))
            .unwrap_or_else(|| panic!("la fenêtre « {id} » est affichée"))
            .rect()
    });
    (cadre, lignes)
}

/// Ce qu'on ne touche pas ne bouge pas.
///
/// C'est la règle la plus simple d'une interface en mode immédiat, et
/// c'est justement celle qu'elle ne garantit pas : chaque image est
/// redessinée depuis zéro, et rien n'empêche une mesure d'être reprise
/// en entrée de la mesure suivante. Quand cela arrive, la fenêtre se
/// déplace toute seule — d'un peu à chaque image, donc d'autant plus
/// vite qu'on remue la souris, puisque chaque mouvement de souris vaut
/// une image de plus. C'était le cas de la barre « OK », dont la rangée
/// prenait toute la hauteur restante du panneau bas, laquelle était
/// précisément mesurée sur cette rangée : la fenêtre d'analyse montait
/// de cinq pixels par image jusqu'à n'avoir plus de contenu visible.
///
/// On dessine donc chaque scène vingt fois sans rien changer, et on
/// demande que les huit dernières images soient la même.
///
/// Vingt et non douze : egui n'installe pas une fenêtre à sa taille
/// d'un coup, il l'y amène — la fenêtre d'analyse met neuf images à
/// gagner son dernier pixel de largeur. Une mise en place qui converge
/// n'est pas une dérive ; ce qu'on refuse, c'est ce qui ne s'arrête
/// jamais.
#[test]
fn a_window_that_is_left_alone_stays_where_it_is() {
    let entrée = écran_du_banc();

    for (nom, id, prépare) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        prépare(&mut app);

        // Ce qui est écrit dans la fenêtre, et où — arrondi au pixel,
        // parce qu'un demi-pixel de fondu n'est pas un déplacement.
        let mut images: Vec<Vec<(String, i32, i32)>> = Vec::new();
        for _ in 0..20 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            let cadre =
                id.and_then(|id| egui::AreaState::load(&ctx, egui::Id::new(id)).map(|a| a.rect()));
            let mut posés = Vec::new();
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape
                    && (id.is_none() || cadre.is_some_and(|c| c.contains(texte.pos)))
                {
                    posés.push((
                        texte.galley.text().to_owned(),
                        texte.pos.x.round() as i32,
                        texte.pos.y.round() as i32,
                    ));
                }
            }
            sortie.textures_delta.clear();
            images.push(posés);
        }

        let repère = &images[12];
        assert!(
            !repère.is_empty(),
            "« {nom} » ne peint aucun texte : le test ne prouverait rien"
        );
        for (n, image) in images.iter().enumerate().skip(13) {
            if image != repère {
                let bougé = repère
                    .iter()
                    .zip(image.iter())
                    .find(|(a, b)| a != b)
                    .map(|(a, b)| format!("« {} » de {:?} à {:?}", a.0, (a.1, a.2), (b.1, b.2)))
                    .unwrap_or_else(|| format!("{} textes puis {}", repère.len(), image.len()));
                panic!("« {nom} » bouge tout seul entre l'image 12 et l'image {n} : {bougé}");
            }
        }
    }
}

/// Un texte tronqué se lit encore en entier, au survol.
///
/// Tronquer est souvent la bonne réponse : une barre d'état ne
/// s'allonge pas, une colonne ne se déforme pas pour un nom trop long.
/// Mais ce qui est coupé doit rester atteignable, sans quoi
/// l'information est perdue et rien ne le dit. On cherche donc chaque
/// texte que egui a dû élider, on pose le pointeur dessus, et l'on
/// demande que le texte entier apparaisse.
///
/// Le survol est simulé pour de vrai — un mouvement de pointeur, puis
/// des images où il ne bouge plus, parce qu'une infobulle attend que la
/// main s'arrête.
#[test]
fn a_truncated_text_can_still_be_read_in_full() {
    /// Une situation à contrôler : son nom, l'écran, et ce qu'on ouvre.
    type Situation = (String, egui::Vec2, fn(&mut NamsdecApp));
    let mut situations: Vec<Situation> = fenêtres_ouvrables()
        .into_iter()
        .map(|(nom, _, ouvre)| (nom.to_owned(), egui::vec2(1600.0, 1100.0), ouvre))
        .collect();
    // Et l'application nue sur un petit écran : c'est là que la barre
    // d'état coupe, et c'est le cas qui a motivé la règle.
    for largeur in [1000.0, 760.0] {
        situations.push((
            format!("application à {largeur:.0} px"),
            egui::vec2(largeur, 620.0),
            |app| app.status = Status::Note(|s| s.analysis_copied),
        ));
    }

    for (nom, écran, ouvre) in situations {
        let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, écran);
        let repos = egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        };

        // Premier passage : ce qui est coupé, et où.
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let mut coupés: Vec<(String, egui::Rect)> = Vec::new();
        for _ in 0..3 {
            coupés.clear();
            let mut sortie = ctx.run_ui(repos.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape
                    && t.galley.elided
                {
                    coupés.push((
                        t.galley.text().to_owned(),
                        egui::Rect::from_min_size(t.pos, t.galley.size()),
                    ));
                }
            }
            sortie.textures_delta.clear();
        }

        // Second passage, un par texte coupé : une application neuve à
        // chaque fois, pour qu'un survol n'en explique pas un autre.
        for (entier, place) in coupés {
            let ctx = egui::Context::default();
            let mut app = NamsdecApp::default();
            app.apply_theme(&ctx);
            ouvre(&mut app);
            for _ in 0..3 {
                let mut sortie = ctx.run_ui(repos.clone(), |ui| app.draw(ui));
                sortie.textures_delta.clear();
            }

            let survol = egui::RawInput {
                screen_rect: Some(écran),
                predicted_dt: 0.5,
                events: vec![egui::Event::PointerMoved(place.center())],
                ..Default::default()
            };
            let immobile = egui::RawInput {
                screen_rect: Some(écran),
                predicted_dt: 0.5,
                ..Default::default()
            };
            let mut sortie = ctx.run_ui(survol, |ui| app.draw(ui));
            sortie.textures_delta.clear();

            let mut lu = false;
            for _ in 0..4 {
                let mut sortie = ctx.run_ui(immobile.clone(), |ui| app.draw(ui));
                lu |= sortie.shapes.iter().any(|c| {
                    matches!(&c.shape, egui::epaint::Shape::Text(t)
                        if !t.galley.elided && t.galley.text().contains(entier.trim()))
                });
                sortie.textures_delta.clear();
            }
            assert!(
                lu,
                "{nom} : « {entier} » est coupé et rien ne le donne en entier — \
                 il y faut une infobulle"
            );
        }
    }
}

/// Tout texte se détache de son fond.
///
/// La mesure est celle des recommandations d'accessibilité, appliquée à
/// ce que l'œil reçoit : la clarté d'une couleur n'est pas la moyenne
/// de ses canaux, et c'est l'erreur qui faisait choisir la moins
/// lisible des deux encres.
///
/// **Deux planchers**, parce qu'il y a deux sortes de texte :
///
/// * `PLANCHER` (3) pour ce qui est actif. En dessous, un texte se
///   devine plus qu'il ne se lit, et c'est un défaut, non une
///   hiérarchie. Au-dessus, la nuance entre `text` et `dim` est une
///   décision de thème, qui ne regarde pas un test — c'est pourquoi le
///   plancher n'est pas 4,5.
/// * `PLANCHER_ESTOMPÉ` (2) pour ce qu'egui peint en transparence,
///   c'est-à-dire les commandes inactives. Un bouton grisé DOIT être
///   moins lisible — c'est ce que « grisé » veut dire, et les
///   recommandations d'accessibilité exemptent d'ailleurs les commandes
///   désactivées. Il doit rester visible, cela dit : sans plancher du
///   tout, une barre d'outils au démarrage pourrait être vide.
///
/// Le fond est celui qu'on a peint dessous, composé dans l'ordre des
/// formes : un texte sur une touche colorée est mesuré sur la couleur
/// de la touche. Sans cela la règle serait vide — c'est justement sur
/// les fonds colorés qu'une encre se perd.
///
/// Cinq défauts trouvés le jour où elle a été écrite : le bleu de
/// sélection, où toute étiquette choisie tombait à 2,07 ; les zéros de
/// la calculatrice, à 2,10 ; les touches d'opérateur, qui portaient
/// l'encre la moins lisible des deux ; la pastille du mode, dont le
/// noir fixe ne tenait que sur un thème sombre ; et le gris-bleu de
/// Tokyo Night, à 2,35, qui porte le nom de l'onglet et les
/// commentaires du programme.
#[test]
fn every_text_stands_out_from_its_background() {
    const PLANCHER: f32 = 3.0;
    const PLANCHER_ESTOMPÉ: f32 = 2.0;

    /// Une encre posée sur un fond : ce que l'œil reçoit.
    fn composé(dessus: egui::Color32, dessous: egui::Color32) -> egui::Color32 {
        let a = f32::from(dessus.a()) / 255.0;
        let [r1, g1, b1, _] = dessus.to_srgba_unmultiplied();
        let [r2, g2, b2, _] = dessous.to_srgba_unmultiplied();
        let m = |x: u8, y: u8| (f32::from(x) * a + f32::from(y) * (1.0 - a)) as u8;
        egui::Color32::from_rgb(m(r1, r2), m(g1, g2), m(b1, b2))
    }

    /// Les couleurs d'encre d'une galley : une par section, car un
    /// texte riche en porte plusieurs.
    fn encres(t: &egui::epaint::TextShape) -> Vec<egui::Color32> {
        if let Some(imposée) = t.override_text_color {
            return vec![imposée];
        }
        let mut couleurs: Vec<egui::Color32> = t
            .galley
            .job
            .sections
            .iter()
            .map(|s| {
                if s.format.color == egui::Color32::PLACEHOLDER {
                    t.fallback_color
                } else {
                    s.format.color
                }
            })
            .collect();
        couleurs.dedup();
        if couleurs.is_empty() {
            couleurs.push(t.fallback_color);
        }
        couleurs
    }

    /// Relève les contrastes d'une situation déjà dessinée.
    ///
    /// `dans` borne la mesure à une fenêtre ; `None` prend tout ce que
    /// l'application peint.
    fn relève(
        formes: &[egui::epaint::ClippedShape],
        fond_initial: egui::Color32,
        dans: Option<egui::Rect>,
        situation: &str,
        fautes: &mut Vec<String>,
    ) -> usize {
        let mut mesurés = 0;
        let mut fonds: Vec<(egui::Rect, egui::Color32)> = Vec::new();
        for clipped in formes {
            match &clipped.shape {
                egui::epaint::Shape::Rect(r) if r.fill.a() > 0 => {
                    fonds.push((r.rect, r.fill));
                }
                egui::epaint::Shape::Text(t) => {
                    let place = egui::Rect::from_min_size(t.pos, t.galley.size());
                    if dans.is_some_and(|c| !c.contains_rect(place.shrink(1.0))) {
                        continue;
                    }
                    let mut fond = fond_initial;
                    for (rect, couleur) in &fonds {
                        if rect.contains(place.center()) {
                            fond = composé(*couleur, fond);
                        }
                    }
                    mesurés += 1;
                    for encre in encres(t) {
                        // Une encre transparente est une commande
                        // inactive : egui grise en baissant l'alpha.
                        let estompé = encre.a() < 255;
                        let seuil = if estompé {
                            PLANCHER_ESTOMPÉ
                        } else {
                            PLANCHER
                        };
                        let vu = composé(encre, fond);
                        let contraste = namsdec_theme::contrast_ratio(vu, fond);
                        if contraste < seuil {
                            let quoi: String = t.galley.text().chars().take(40).collect();
                            fautes.push(format!(
                                "[{situation}] {contraste:.2} < {seuil} — « {quoi} » {:?} sur {:?}",
                                vu.to_srgba_unmultiplied(),
                                fond.to_srgba_unmultiplied()
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        mesurés
    }

    /// Six images et un pas de temps franc : egui fait apparaître une
    /// fenêtre en fondu, et tant qu'il dure les couleurs peintes ne
    /// sont pas celles qu'on verra.
    fn dessine(ctx: &egui::Context, app: &mut NamsdecApp) -> Vec<egui::epaint::ClippedShape> {
        let entrée = egui::RawInput {
            predicted_dt: 0.5,
            ..écran_du_banc()
        };
        let mut formes = Vec::new();
        for _ in 0..6 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            formes = std::mem::take(&mut sortie.shapes);
            sortie.textures_delta.clear();
        }
        formes
    }

    let mut fautes: Vec<String> = Vec::new();

    // Les fenêtres, chacune sur son propre fond.
    for (nom, id, ouvre) in fenêtres_ouvrables() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let formes = dessine(&ctx, &mut app);
        let cadre = egui::AreaState::load(&ctx, egui::Id::new(id))
            .unwrap_or_else(|| panic!("la fenêtre « {nom} » est affichée"))
            .rect();
        let mesurés = relève(&formes, DEFAULT_THEME.bg, Some(cadre), nom, &mut fautes);
        assert!(
            mesurés > 2,
            "la fenêtre « {nom} » n'écrit presque rien : le test ne prouverait rien"
        );
    }

    // Et l'application entière, dans chacun de ses thèmes et sur
    // chacun de ses onglets : c'est là que vivent les couleurs des
    // thèmes, et un thème ne se relit pas en le regardant.
    for theme in namsdec_theme::BUILT_IN {
        for onglet in SidebarTab::ALL {
            let ctx = egui::Context::default();
            let mut app = NamsdecApp {
                theme,
                show_sidebar: true,
                show_console: true,
                console_height: Some(169.0),
                sidebar_tab_height: Some(180.0),
                active_tab: onglet,
                ..Default::default()
            };
            app.apply_theme(&ctx);
            let formes = dessine(&ctx, &mut app);
            let situation = format!("{} / {}", theme.id, onglet.id());
            let mesurés = relève(&formes, theme.bg, None, &situation, &mut fautes);
            assert!(mesurés > 20, "{situation} : presque rien n'est peint");
        }
    }

    fautes.sort();
    fautes.dedup();
    assert!(
        fautes.is_empty(),
        "{} texte(s) sous leur plancher de contraste :\n  {}",
        fautes.len(),
        fautes.join("\n  ")
    );
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

    // Et cela s'écrit à l'écran, là où l'on regarde déjà les registres.
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

/// Pousse la date de dernière écriture d'un fichier d'une seconde, pour
/// que le changement soit visible d'un test qui écrit deux fois dans la
/// même seconde.
fn filetime_plus_tard(chemin: &std::path::Path) {
    let plus_tard = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
    let f = std::fs::File::options().write(true).open(chemin).unwrap();
    f.set_modified(plus_tard).unwrap();
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
        show_tooltips: false,
        flow_pseudo: true,
        completion: false,
        exercises: true,
        auto_indent: false,
        auto_pairs: false,
        hover_help: false,
        disabled_packs: vec!["une-serie".to_owned()],
        current_exercise: Some("namsdec/boucle".to_owned()),
        recent_files: vec![source.display().to_string()],
        exercises_done: vec!["namsdec/boucle".to_owned()],
        exercises_best: vec![("namsdec/boucle".to_owned(), 7)],
        disassembler: "iced".to_owned(),
        memory_format: "words".to_owned(),
        analysis_module: "dump".to_owned(),
        calc_base: "dec".to_owned(),
        calc_width: "32".to_owned(),
        heat_legend: false,
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

/// Toute action a un raccourci, dans chacun des deux jeux.
///
/// Une action sans raccourci n'est pas forcément inaccessible — la
/// plupart ont aussi un bouton ou une entrée de menu —, mais elle est
/// invisible dans Préférences › Raccourcis, où l'on vient justement
/// voir ce que namsdec sait faire. Et changer de jeu ne doit pas faire
/// disparaître une commande : les deux couvrent donc les mêmes trente.
#[test]
fn every_action_has_a_shortcut_in_both_presets() {
    for preset in crate::keybindings::Preset::ALL {
        let jeu = KeyBindings::preset(preset);
        let sans: Vec<Action> = Action::ALL
            .into_iter()
            .filter(|a| !jeu.map.contains_key(a))
            .collect();
        assert!(
            sans.is_empty(),
            "jeu « {} » : {} action(s) sans raccourci — {sans:?}",
            preset.id(),
            sans.len()
        );
        // Et deux actions ne partagent pas la même frappe : la seconde
        // ne se déclencherait jamais.
        let mut vus: std::collections::BTreeMap<String, Action> = Default::default();
        for (action, chord) in &jeu.map {
            let touches = format!("{chord:?}");
            if let Some(déjà) = vus.insert(touches.clone(), *action) {
                panic!(
                    "jeu « {} » : {déjà:?} et {action:?} partagent {touches}",
                    preset.id()
                );
            }
        }
    }
}

/// SONDE : chaque module de RE dit-il quelque chose ?
#[test]
fn sonde_modules_re() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    for _ in 0..8 {
        app.do_step();
    }
    app.show_analysis = true;
    let ids: Vec<String> = app
        .re_registry
        .entries("fr")
        .iter()
        .map(|(id, _)| (*id).to_owned())
        .collect();
    for id in ids {
        app.analysis_module = id.clone();
        let (_, textes) = ce_qui_est_écrit(&ctx, &mut app, Some("analysis_window"), 4);
        let mots: usize = textes.len();
        let échantillon: Vec<&str> = textes
            .iter()
            .map(|(t, _)| t.as_str())
            .skip(6)
            .take(3)
            .collect();
        eprintln!("{id:24} {mots:4} textes  {échantillon:?}");
    }
}

/// Le graphe s'écrit dans un fichier, et pas seulement dans le
/// presse-papier.
///
/// Un graphe se garde : celui d'avant le correctif, celui d'après,
/// celui qu'on joint au compte rendu. Le presse-papier, lui, ne survit
/// pas à la copie suivante.
#[test]
fn the_graph_goes_to_a_file_and_not_only_to_the_clipboard() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let chemin = dir.path().join("graphe.dot");

    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    // Sans programme, rien à écrire — et on le dit plutôt que d'écrire
    // un fichier vide.
    app.export_dot(&chemin);
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
    assert!(!chemin.exists(), "aucun fichier n'est créé pour rien");

    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.export_dot(&chemin);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    let dot = std::fs::read_to_string(&chemin).expect("le fichier est écrit");
    assert!(
        dot.starts_with("digraph"),
        "du DOT, et pas autre chose :\n{dot}"
    );
    assert!(dot.contains("->") || dot.contains("label"), "{dot}");
    // Et c'est bien le graphe qu'on regarde : la case « la fonction
    // seule » change ce qui est écrit.
    assert_eq!(
        dot,
        app.flow_dot().expect("le même graphe qu'à l'écran"),
        "le fichier et l'écran doivent montrer le même graphe"
    );
}

/// Le jeu de raccourcis s'exporte **en entier**, à plat.
///
/// `keymap.json` ne garde que les écarts, et c'est ce qu'on veut d'un
/// fichier de réglages : il suit le jeu quand celui-ci gagne une
/// action. Mais un fichier de deux lignes ne dit pas ce que namsdec
/// sait faire, et l'on n'avait rien à quoi se raccrocher pour partir
/// d'une liste complète.
#[test]
fn the_whole_shortcut_set_can_be_exported_flat() {
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let chemin = dir.path().join("keymap.json");
    let mut app = NamsdecApp::default();
    app.export_keymap(&chemin);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);

    let écrit = std::fs::read_to_string(&chemin).expect("le fichier est écrit");
    let relu: settings::Keymap = serde_json::from_str(&écrit).expect("du JSON valide");
    assert_eq!(
        relu.shortcuts.len(),
        Action::ALL.len(),
        "les trente actions y sont, une par une"
    );
    // Et il décrit exactement le jeu courant : relu, il rend les mêmes
    // raccourcis.
    assert_eq!(relu.bindings().map, app.keybindings.map);
}

/// `keymap.json` modifié pendant que namsdec tourne est repris.
///
/// C'est un fichier qu'on donne à éditer ; il n'était relu qu'au
/// lancement suivant. On l'éditait, on essayait la touche, il ne se
/// passait rien — et rien ne disait pourquoi.
#[test]
fn a_keymap_edited_on_disk_is_picked_up_while_running() {
    let mut app = NamsdecApp::default();
    let avant = app.keybindings.map.clone();

    // Le fichier n'a pas de date connue : la première visite en prend
    // une sans rien changer.
    app.keymap_mtime = None;
    app.keymap_checked = None;
    app.reload_keymap_if_changed();

    // Une capture de raccourci en cours suspend la reprise : changer le
    // jeu sous les doigts de quelqu'un qui appuie serait la dernière
    // chose à faire.
    app.rebinding = Some(keybindings::Rebinding::new(
        Action::Assemble,
        keybindings::RebindMode::Simple,
    ));
    app.keymap_checked = None;
    app.keymap_mtime = Some(std::time::SystemTime::UNIX_EPOCH);
    app.reload_keymap_if_changed();
    assert_eq!(
        app.keymap_mtime,
        Some(std::time::SystemTime::UNIX_EPOCH),
        "pendant une capture, on ne regarde même pas"
    );
    app.rebinding = None;

    // Et l'on ne se réveille pas plus d'une fois par seconde.
    app.keymap_checked = Some(std::time::Instant::now());
    app.keymap_mtime = Some(std::time::SystemTime::UNIX_EPOCH);
    app.reload_keymap_if_changed();
    assert_eq!(
        app.keymap_mtime,
        Some(std::time::SystemTime::UNIX_EPOCH),
        "l'intervalle est respecté : pas d'accès au disque à chaque image"
    );
    assert_eq!(app.keybindings.map, avant);
}

/// Toutes les icônes dessinées, avec la taille qu'elles reçoivent
/// vraiment.
///
/// Le bouton carré du panneau donne douze pixels, la barre d'outils
/// quatorze, le gros bouton d'analyse dix-huit. Une icône qui n'a été
/// regardée qu'en grand ment sur ce qu'elle sera.
type Icône = (&'static str, fn(&egui::Painter, egui::Rect, egui::Color32));

fn icônes_du_banc() -> Vec<Icône> {
    vec![
        ("calculatrice", chrome::icon_calculator),
        ("analyse", chrome::icon_analysis),
        ("recherche", chrome::icon_search),
        ("globe", chrome::icon_globe),
        ("github", chrome::icon_github),
        ("discord", chrome::icon_discord),
        ("retour arrière", chrome::icon_backspace),
        ("œil", icon_watch),
        ("aller", icon_go),
        ("exécuter", icon_run),
        ("arrêter", icon_stop),
        ("pas dans", icon_step_into),
        ("pas sur", icon_step_over),
        ("pas arrière", icon_step_back),
        ("réinitialiser", icon_reset),
        ("agrandir", icon_maximize),
        ("restaurer", icon_restore),
    ]
}

/// Une icône ne se peint jamais en bouillie.
///
/// La règle vient d'un défaut qu'on a vu à l'écran avant de le
/// mesurer : l'icône de la calculatrice « ne ressemblait à rien ». Elle
/// superposait deux rectangles tracés au trait de 1,2 px dans un
/// boîtier de cinq pixels de large. Le contour du boîtier en mangeait
/// deux et demi, l'écran intérieur — 1,45 px de haut pour 1,2 px de
/// trait — se remplissait de son propre contour, et les quatre touches
/// débordaient. Rendue, une tache blanche avec un trou.
///
/// Deux mesures suffisent à l'interdire :
///
/// * **un rectangle tracé garde un intérieur** au moins aussi épais que
///   son trait — sinon les deux bords se rejoignent et la forme est
///   pleine, ce qui n'est pas ce qu'on a demandé ;
/// * **deux points pleins de même taille ne se touchent pas** — sinon
///   la grille de touches devient un pâté. De même taille seulement :
///   un petit rond posé sur un grand disque est une composition (l'œil
///   de la pupille, l'oreille du chat de GitHub), pas un défaut.
///
/// Aux trois tailles auxquelles les icônes sont réellement posées :
/// c'est en petit que les dessins se cassent, et c'est en petit qu'on
/// ne les regarde jamais.
#[test]
fn an_icon_never_paints_itself_into_a_blob() {
    let ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(200.0, 200.0),
        )),
        ..Default::default()
    };
    for (nom, dessin) in icônes_du_banc() {
        for taille in [12.0_f32, 14.0, 18.0] {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| {
                let boîte =
                    egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(taille, taille));
                dessin(ui.painter(), boîte, egui::Color32::WHITE);
            });
            let mut ronds: Vec<(egui::Pos2, f32)> = Vec::new();
            for clipped in &sortie.shapes {
                match &clipped.shape {
                    egui::epaint::Shape::Rect(r) if r.stroke.width > 0.0 => {
                        let mince = r.rect.width().min(r.rect.height());
                        assert!(
                            mince >= r.stroke.width * 3.0,
                            "« {nom} » à {taille} px : un rectangle de {mince:.2} px \
                             tracé au trait de {:.2} se remplit de son propre contour",
                            r.stroke.width
                        );
                    }
                    egui::epaint::Shape::Circle(c) if c.fill.a() > 0 => {
                        ronds.push((c.center, c.radius));
                    }
                    _ => {}
                }
            }
            sortie.textures_delta.clear();
            for (i, (a, ra)) in ronds.iter().enumerate() {
                for (b, rb) in &ronds[i + 1..] {
                    // Des points d'une même famille : au-delà du simple
                    // au double, c'est une composition et non une grille.
                    if ra.max(*rb) > ra.min(*rb) * 2.0 {
                        continue;
                    }
                    let écart = a.distance(*b);
                    assert!(
                        écart >= ra + rb + 0.3,
                        "« {nom} » à {taille} px : deux points se touchent \
                         ({écart:.2} px pour des rayons de {ra:.2} et {rb:.2})"
                    );
                }
            }
        }
    }
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
    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");

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

/// « Coller » colle à l'endroit du curseur, et ne remplace pas tout le
/// programme.
///
/// L'entrée du menu Édition remplaçait le programme entier par le
/// presse-papier. Elle s'appelait « Coller ». On cherchait à recopier
/// trois lignes d'un cours, on perdait son travail, et rien ne
/// prévenait — pas même un intitulé qui l'aurait laissé deviner.
#[test]
fn pasting_inserts_at_the_cursor_instead_of_replacing_the_program() {
    // Par la fonction qui prend le texte, et non par le presse-papier :
    // un test n'écrit pas dans celui de l'utilisateur, pas plus que
    // dans ses réglages.
    const COLLÉ: &str = "    mov rax, 60\n";

    let programme = "bits 64\nglobal _start\n_start:\n    hlt\n";
    let mut app = NamsdecApp {
        source: programme.to_owned(),
        ..Default::default()
    };
    // Curseur au début de la ligne « hlt ».
    let position = programme.find("    hlt").expect("la ligne existe");
    app.cursor_byte = position;

    app.insert_at_cursor(COLLÉ);
    assert_eq!(
        app.source, "bits 64\nglobal _start\n_start:\n    mov rax, 60\n    hlt\n",
        "le texte s'insère où était le curseur, et rien n'est perdu"
    );
    assert_eq!(
        app.cursor_byte,
        position + COLLÉ.len(),
        "le curseur suit ce qu'on vient de coller"
    );

    // Avec une sélection, le collé la remplace — comme partout ailleurs.
    let mut app = NamsdecApp {
        source: programme.to_owned(),
        ..Default::default()
    };
    let début = programme.find("    hlt").unwrap();
    app.editor_selection = Some(début..début + "    hlt\n".len());
    app.insert_at_cursor(COLLÉ);
    assert_eq!(
        app.source,
        "bits 64\nglobal _start\n_start:\n    mov rax, 60\n"
    );
    assert!(app.editor_selection.is_none(), "la sélection est consommée");

    // Et le remplacement total existe toujours, sous son propre nom.
    let mut app = NamsdecApp {
        source: programme.to_owned(),
        ..Default::default()
    };
    app.replace_all_with(COLLÉ);
    assert_eq!(app.source, COLLÉ);

    // Sur un listing venu d'un binaire, les deux refusent : y insérer
    // du texte décalerait les correspondances ligne ↔ adresse.
    if let Some(octets) = binaire_dépreuve("crackme-01") {
        let mut app = NamsdecApp::default();
        assert!(app.load_elf(&octets, "crackme-01"));
        let listing = app.source.clone();
        app.insert_at_cursor(COLLÉ);
        app.replace_all_with(COLLÉ);
        assert_eq!(app.source, listing, "un listing ne se colle pas dedans");
    }
}

/// Un panneau vide dit ce qui manque, ce qu'on y verra, et propose ce
/// qui y mène.
///
/// La règle venait de l'explorateur, qui l'appliquait seul : « un
/// panneau simplement vide se lit comme cassé ». Les six panneaux du
/// bandeau latéral, eux, disaient « Aucun binaire assemblé. » et
/// s'arrêtaient là — un constat, pas une porte. Or c'est exactement
/// l'écran qu'on a sous les yeux la première fois qu'on ouvre namsdec.
#[test]
fn an_empty_panel_says_what_is_missing_and_offers_the_way_out() {
    let s = strings(Lang::Fr);
    // Une application au démarrage : un programme écrit, rien
    // d'assemblé.
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_sidebar: true,
        ..Default::default()
    };
    assert!(app.emulator.is_none(), "rien n'a encore tourné");

    let peint = paint(&mut app);
    assert!(
        peint.contains(s.no_binary_assembled),
        "le constat :\n{peint}"
    );
    assert!(
        peint.contains(s.empty_assemble_hint),
        "ce qu'il faut faire :\n{peint}"
    );
    assert!(
        peint.contains(s.action_assemble),
        "et le bouton qui le fait :\n{peint}"
    );

    // Chaque onglet dit ce qu'il montrera : un panneau vide qui ne dit
    // pas à quoi il sert n'apprend rien à qui le découvre.
    for (onglet, attendu) in [
        (SidebarTab::Stack, s.empty_stack),
        (SidebarTab::Memory, s.empty_memory),
        (SidebarTab::Heap, s.empty_heap),
        (SidebarTab::Bytes, s.empty_bytes),
    ] {
        app.active_tab = onglet;
        let peint = paint(&mut app);
        assert!(
            peint.contains(attendu),
            "l'onglet {} doit dire ce qu'il montrera :\n{peint}",
            onglet.id()
        );
    }

    // Et le bouton assemble vraiment : il ne décore pas.
    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        app.dispatch(Action::Assemble);
        assert!(
            app.emulator.is_some(),
            "le bouton du panneau vide fait ce qu'il dit : {:?}",
            app.status
        );
        let peint = paint(&mut app);
        assert!(
            !peint.contains(s.empty_assemble_hint),
            "et il disparaît une fois qu'il n'a plus lieu d'être"
        );
    }
}

/// La palette propose vraiment **toutes** les commandes.
///
/// Elle le promet — « toutes les commandes » — et le code le dit déjà :
/// une commande que la palette ne propose pas n'existe que pour qui
/// connaît déjà son raccourci. Rien ne l'imposait, et une action
/// ajoutée demain n'y entrerait pas d'elle-même.
#[test]
fn the_palette_offers_every_single_action() {
    let s = strings(Lang::Fr);
    // Tout allumé : les modules facultatifs ajoutent des entrées, et
    // c'est l'inventaire complet qu'on veut contrôler.
    let commandes = palette_commands(s, true, true);
    let proposées: std::collections::BTreeSet<String> = commandes
        .iter()
        .filter_map(|(_, c)| match c {
            PaletteCommand::Do(action) => Some(format!("{action:?}")),
            _ => None,
        })
        .collect();
    let manquantes: Vec<String> = Action::ALL
        .into_iter()
        .filter(|a| !proposées.contains(&format!("{a:?}")))
        .map(|a| format!("{a:?}"))
        .collect();
    assert!(
        manquantes.is_empty(),
        "{} action(s) absente(s) de la palette : {manquantes:?}",
        manquantes.len()
    );
    // Et aucune entrée ne porte un libellé vide : on ne cherche pas une
    // commande qui n'a pas de nom.
    for (libellé, _) in &commandes {
        assert!(
            !libellé.trim().is_empty(),
            "une entrée de la palette sans libellé ne se cherche pas"
        );
    }
}

/// L'état des outils est relevé une fois, et non à chaque image.
///
/// Savoir si `rizin` est complet demande de **le lancer** : `rizin` sans
/// `rz-ghidra` a le programme et pas la commande, et namsdec le lui
/// demande plutôt que de le supposer. Cette réponse était redemandée à
/// chaque image, si bien qu'ouvrir Préférences › Outils lançait `rizin`
/// soixante fois par seconde.
///
/// On l'a vu par un test instable : le contrôle de la fenêtre
/// d'installation demandait la réponse, la fenêtre la redemandait, et
/// les deux pouvaient différer sous la charge. Un test qui échoue au
/// hasard apprend à ignorer la suite.
#[test]
fn what_the_tools_answer_is_asked_once_and_kept() {
    let mut app = NamsdecApp::default();
    assert!(app.tools_ready.is_empty(), "rien n'est demandé sans raison");

    // Première demande : le relevé se remplit, pour tous les outils
    // d'un coup.
    let première = app.tool_availability("rizin-ghidra");
    assert_eq!(
        app.tools_ready.len(),
        namsdec_disasm::known::all().len(),
        "un seul relevé couvre tous les outils"
    );

    // Les demandes suivantes ne relancent rien : elles lisent. On le
    // vérifie en faussant le relevé — s'il était refait, la valeur
    // posée à la main disparaîtrait.
    app.tools_ready.insert(
        "rizin-ghidra".to_owned(),
        namsdec_disasm::known::Availability::Incomplete,
    );
    assert_eq!(
        app.tool_availability("rizin-ghidra"),
        namsdec_disasm::known::Availability::Incomplete,
        "la réponse gardée est celle qu'on lit"
    );

    // Et le relevé se refait quand on le jette : à la fin d'une
    // installation, à l'ouverture des préférences, ou sur demande.
    app.tools_ready.clear();
    assert_eq!(
        app.tool_availability("rizin-ghidra"),
        première,
        "jeté, il se refait et retrouve ce que la machine répond"
    );

    // L'ouverture des préférences le jette une fois : on a pu
    // installer un outil à la main dans un terminal entre-temps.
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    app.show_preferences = true;
    app.preferences_tab = PreferencesTab::Tools;
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    sortie.textures_delta.clear();

    // Et les images suivantes le gardent : c'est tout l'objet. Faussé à
    // la main, il doit survivre au dessin — s'il était refait par
    // image, la valeur posée disparaîtrait.
    app.tools_ready.insert(
        "objdump".to_owned(),
        namsdec_disasm::known::Availability::Incomplete,
    );
    for _ in 0..4 {
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    assert_eq!(
        app.tools_ready.get("objdump").copied(),
        Some(namsdec_disasm::known::Availability::Incomplete),
        "le relevé n'est pas refait à chaque image"
    );
}

/// Hors des menus, un bouton se voit sans qu'on le survole.
///
/// Le thème rend les boutons **plats au repos** — fond transparent,
/// liseré absent — et c'est voulu : sans cela la barre de menus et la
/// barre d'outils se ressemblent trop. Mais la règle s'appliquait à
/// *tous* les boutons, y compris ceux des fenêtres et des boîtes de
/// dialogue, qui ne se distinguaient donc d'une étiquette qu'au survol.
/// Dix-sept l'étaient : « Recharger », « Copier le rapport », « Envoyer »,
/// les croix de fermeture des champs de recherche…
///
/// `framed_button` pose le liseré. Ce contrôle lit le source parce que
/// c'est là qu'est la règle : ce qu'on peut cliquer doit se voir, et les
/// exceptions — la barre de menus et la barre d'outils, où la platitude
/// EST le dessin et où un liseré par bouton ferait une grille — sont
/// nommées.
///
/// La barre d'outils n'était pas nommée jusqu'ici, et elle passait
/// seulement parce que son bouton « Assembler » est écrit sur plusieurs
/// lignes, c'est-à-dire par le trou même que ce contrôle avait. Une
/// exception qui tient à un défaut n'est pas une exception.
#[test]
fn outside_the_menus_a_button_shows_itself_without_being_hovered() {
    let mut plats: Vec<(String, &str, String)> = Vec::new();
    for (fichier, source) in SOURCES_DE_L_INTERFACE {
        let mut fonction = "";
        for (n, ligne) in source.lines().enumerate() {
            // Le nom de la fonction englobante : les `fn` de premier niveau
            // dans un `impl` sont indentés de quatre espaces.
            // Les fonctions de premier niveau AUSSI : `tab_plus_menu` vit au
            // niveau du module, et le contrôle ne voyait que les méthodes.
            // Une exception qui ne peut pas être nommée n'en est pas une.
            if let Some(nom) = ligne
                .strip_prefix("fn ")
                .or_else(|| ligne.strip_prefix("pub fn "))
                .or_else(|| ligne.strip_prefix("pub(crate) fn "))
            {
                fonction = nom.split('(').next().unwrap_or("").trim();
            }
            if let Some(reste) = ligne.strip_prefix("    ")
                && let Some(nom) = reste
                    .trim_start_matches("pub(crate) ")
                    .trim_start_matches("pub ")
                    .strip_prefix("fn ")
            {
                fonction = nom.split('(').next().unwrap_or("").trim();
            }
            // Deux écritures, et la seconde avait été oubliée : `ui.button(…)`
            // sur une ligne, mais aussi la chaîne coupée en plusieurs lignes,
            // que rustfmt produit dès qu'on ajoute `.on_hover_text(…)` —
            //
            //     if ui
            //         .button(s.analysis_save)
            //         .on_hover_text(s.analysis_save_hint)
            //
            // Le contrôle ne voyait que la première, et sept boutons sont
            // passés au travers pendant des semaines : « Enregistrer… » et
            // « Enregistrer le vidage » dans l'analyse, « Vers l'entrée du
            // programme » dans le motif cyclique, « Fin d'entrée » deux fois,
            // « Retour » dans l'aide, « Rechercher une mise à jour ». Trois
            // d'entre eux ont été retrouvés sur une CAPTURE, posés à côté
            // d'un bouton encadré — c'est dire ce que le contrôle laissait
            // passer.
            // TROISIÈME écriture, et elle a coûté soixante boutons :
            // `egui::Button::new(…)` posé à la main, ce que fait
            // `add_enabled` quand il faut griser le bouton. Les deux boutons
            // de chaque rangée de Préférences › Raccourcis — « Touche
            // simple… » et « Séquence… », trente rangées — étaient dans ce
            // cas, et le contrôle ne les voyait pas.
            //
            // Un `Button::new` suivi d'un `.fill(` ou d'un `.stroke(` dans
            // les dix lignes qui suivent se voit déjà : c'est le bouton
            // plein des barres de dialogue, ou l'encadré lui-même. Dix et
            // non six : le libellé d'un bouton plein tient sur sept lignes
            // quand il porte une couleur, une police et une taille.
            let nu = ligne.contains("egui::Button::new(")
                && !source
                    .lines()
                    .skip(n)
                    .take(10)
                    .any(|d| d.contains(".fill(") || d.contains(".stroke("));
            let plat = ligne.contains("ui.button(")
                || nu
                || (ligne.trim_start().starts_with(".button(")
                    && source
                        .lines()
                        .nth(n.saturating_sub(1))
                        .is_some_and(|d| d.trim_end().ends_with("ui")));
            if plat && !matches!(fonction, "menu_bar_ui" | "toolbar_ui") {
                plats.push((
                    format!("{fichier}:{}", n + 1),
                    fonction,
                    ligne.trim().chars().take(70).collect(),
                ));
            }
        }
    }
    assert!(
        plats.is_empty(),
        "{} bouton(s) invisible(s) au repos — `framed_button` les encadre :\n  {}",
        plats.len(),
        plats
            .iter()
            .map(|(où, f, l)| format!("{où} ({f}) {l}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// SONDE : le bandeau déborde-t-il, et à quelles tailles ?
#[test]
fn sonde_bandeau_deborde() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    for (l, h) in [
        (1280.0_f32, 820.0_f32),
        (1600.0, 1000.0),
        (1920.0, 1230.0),
        (1920.0, 1600.0),
    ] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            show_sidebar: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        app.do_assemble();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(l, h),
            )),
            ..Default::default()
        };
        for _ in 0..4 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            sortie.textures_delta.clear();
        }
        let état = egui::scroll_area::State::load(&ctx, egui::Id::new("sidebar_banner"));
        eprintln!(
            "{l}x{h} : bandeau voulu {:.0}, onglet {:?}, offset {:?}",
            app.sidebar_banner_height,
            app.sidebar_tab_height,
            état.map(|e| e.offset.y),
        );
    }
}

/// L'état du processeur ne défile jamais, à aucune taille de fenêtre.
///
/// C'est ce qu'on regarde entre deux pas : devoir le faire défiler pour
/// voir R15 ou ZF coupe la lecture. Il partageait sa zone défilante avec
/// la prochaine instruction, laquelle est au-dessus — et une zone qui
/// défile coupe par le bas. Sur une fenêtre de 1280×820, la taille par
/// défaut, le compte ne tombait pas juste : c'étaient donc les registres
/// qui passaient sous la barre.
///
/// Vu à l'écran avant d'être mesuré, et c'est la leçon : le contrôle
/// d'alors regardait les colonnes, pas la barre.
#[test]
fn the_cpu_state_never_scrolls_at_any_window_size() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // De la fenêtre minimale à un grand écran, en passant par la taille
    // par défaut — celle où le défaut se voyait.
    for (largeur, hauteur) in [
        (800.0_f32, 520.0_f32),
        (1280.0, 820.0),
        (1600.0, 1000.0),
        (1920.0, 1230.0),
    ] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            show_sidebar: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        app.do_assemble();
        assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, hauteur),
            )),
            ..Default::default()
        };
        // Le partage se stabilise en quelques images : il se règle sur
        // ce que l'image précédente a mesuré.
        let mut entiers: std::collections::BTreeSet<&str> = Default::default();
        let mut rognés: Vec<String> = Vec::new();
        for _ in 0..6 {
            entiers.clear();
            rognés.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                let Some((nom, _)) = REGISTERS.iter().find(|(n, _)| *n == t.galley.text()) else {
                    continue;
                };
                let posé = egui::Rect::from_min_size(t.pos, t.galley.size());
                // Ce qui se voit : l'intersection avec le découpage. Un
                // demi-pixel de tolérance pour les arrondis.
                let vu = posé.intersect(clipped.clip_rect);
                if vu.height() >= posé.height() - 0.5 {
                    entiers.insert(nom);
                } else if vu.height() > 0.0 {
                    rognés.push(format!(
                        "{nom} ({:.0} px sur {:.0})",
                        vu.height(),
                        posé.height()
                    ));
                }
            }
            sortie.textures_delta.clear();
        }
        assert!(
            rognés.is_empty(),
            "à {largeur}×{hauteur}, des registres sont coupés par une barre de \
             défilement : {rognés:?}"
        );
        for (nom, _) in REGISTERS {
            assert!(
                entiers.contains(nom),
                "à {largeur}×{hauteur}, {nom} ne se voit pas : l'état du processeur \
                 doit tenir en entier, sans défilement"
            );
        }
    }
}

/// « Base » et « Largeur » sont deux réglages jumeaux, et se lisent
/// comme tels.
///
/// Chaque liste prenait la largeur de ce qu'elle affichait : « Binaire »
/// et « 8 bits » donnaient deux boîtes de tailles différentes, et deux
/// flèches qui ne tombaient pas au même endroit. Les largeurs sont
/// maintenant mesurées sur les textes les plus longs — et dans les deux
/// langues, parce qu'un chiffre écrit en dur alignerait le français en
/// désalignant l'anglais.
#[test]
fn the_base_and_the_width_are_two_twins_and_read_as_twins() {
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            lang,
            show_calculator: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        // Où chaque texte est peint, une fois la fenêtre posée.
        let mut posés: Vec<(String, egui::Rect)> = Vec::new();
        for _ in 0..4 {
            posés.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    posés.push((
                        t.galley.text().to_owned(),
                        egui::Rect::from_min_size(t.pos, t.galley.size()),
                    ));
                }
            }
            sortie.textures_delta.clear();
        }
        let où = |quoi: &str| -> egui::Rect {
            posés
                .iter()
                .find(|(t, _)| t == quoi)
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("{lang:?} : « {quoi} » est peint"))
        };

        let base = où(s.calculator_base_label);
        let largeur = où(s.calculator_width_label);
        // Les deux étiquettes sur la même ligne — comparées par leur
        // MILIEU : « Largeur » a une hampe descendante et « Base » non,
        // si bien que leurs galettes n'ont pas la même hauteur et que
        // leurs bords hauts ne coïncident pas alors qu'elles sont
        // pourtant sur la même ligne.
        assert!(
            (base.center().y - largeur.center().y).abs() < 2.0,
            "{lang:?} : les deux étiquettes doivent être sur la même ligne — \
             {:.1} contre {:.1}",
            base.center().y,
            largeur.center().y
        );
        let valeur_base = où(app.calc_base.label(s));
        let valeur_largeur = où(&format!("{} bits", app.calc_width.bits()));
        let écart_base = valeur_base.left() - base.left();
        let écart_largeur = valeur_largeur.left() - largeur.left();
        assert!(
            (écart_base - écart_largeur).abs() < 1.0,
            "{lang:?} : la valeur doit être au même écart de son étiquette — \
             {écart_base:.1} px pour « {} », {écart_largeur:.1} px pour « {} »",
            s.calculator_base_label,
            s.calculator_width_label
        );

        // Et les deux flèches tombent au même endroit : c'est ce qui se
        // voit d'abord. egui les dessine en triangle, calées à droite de
        // la liste — deux listes de même largeur donnent donc deux
        // flèches au même écart de leur étiquette.
        let mut flèches: Vec<f32> = flèches_de_la_rangée(&ctx, &mut app, base.center().y);
        flèches.sort_by(f32::total_cmp);
        assert_eq!(
            flèches.len(),
            2,
            "{lang:?} : deux listes, donc deux flèches — {flèches:?}"
        );
        let vers_flèche_base = flèches[0] - base.left();
        let vers_flèche_largeur = flèches[1] - largeur.left();
        assert!(
            (vers_flèche_base - vers_flèche_largeur).abs() < 1.0,
            "{lang:?} : les deux flèches doivent tomber au même endroit — \
             {vers_flèche_base:.1} px contre {vers_flèche_largeur:.1} px"
        );
    }
}

/// Les abscisses des flèches de liste posées sur cette ligne.
///
/// egui dessine la flèche d'une `ComboBox` en triangle fermé de trois
/// points, calée à droite de la boîte. C'est le repère le plus proche du
/// bord droit de la liste, et donc le meilleur pour dire si deux listes
/// ont la même largeur.
fn flèches_de_la_rangée(ctx: &egui::Context, app: &mut NamsdecApp, ligne: f32) -> Vec<f32> {
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
    let mut x = Vec::new();
    for clipped in &sortie.shapes {
        if let egui::epaint::Shape::Path(chemin) = &clipped.shape
            && chemin.closed
            && chemin.points.len() == 3
        {
            let boîte = egui::Rect::from_points(&chemin.points);
            if (boîte.center().y - ligne).abs() < 12.0 {
                x.push(boîte.left());
            }
        }
    }
    sortie.textures_delta.clear();
    x
}

/// Une même phrase ne se répète pas sur un écran.
///
/// Au démarrage, « Aucun binaire assemblé. » s'affichait **trois fois**
/// en même temps : le bandeau des registres, l'onglet du panneau, et la
/// console. La première dit ce qui manque et propose le bouton ; les
/// deux autres ne faisaient que la répéter, là où elles pouvaient dire
/// ce qu'elles montreraient.
///
/// Deux fois est acceptable — un constat et son écho là où l'on
/// regarde — trois fois est du remplissage.
#[test]
fn no_sentence_is_said_three_times_on_one_screen() {
    for onglet in SidebarTab::ALL {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            show_sidebar: true,
            show_console: true,
            active_tab: onglet,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let entrée = écran_du_banc();
        let mut comptes: std::collections::BTreeMap<String, usize> = Default::default();
        for _ in 0..3 {
            comptes.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    let texte = t.galley.text().trim();
                    // Les phrases seulement : un nombre, un nom de
                    // registre ou un libellé d'onglet se répètent à bon
                    // droit.
                    if texte.len() > 20 && texte.contains(' ') {
                        *comptes.entry(texte.to_owned()).or_default() += 1;
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        for (phrase, combien) in &comptes {
            assert!(
                *combien < 3,
                "onglet {} : « {phrase} » est écrit {combien} fois sur le même écran",
                onglet.id()
            );
        }
    }
}

/// Le bandeau latéral ne coupe aucune ligne en deux.
///
/// Une zone qui défile se coupe, et c'est son métier — mais dans une
/// liste, où l'on fait défiler exprès. Dans le bandeau du haut, qui
/// n'est pas une liste, une demi-ligne tranchée au milieu ne se lit pas
/// comme un choix : elle se lit comme un défaut. C'était le cas de
/// « EAX = 0 » sur une fenêtre de 1280×820, la taille par défaut.
///
/// La mesure ne regarde que ce qui est **au-dessus** de l'onglet : en
/// dessous, la pile et la mémoire sont des listes, et leur dernière
/// ligne a le droit d'être coupée.
#[test]
fn the_sidebar_banner_never_cuts_a_line_in_half() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Les deux sortes de séance : un source qu'on assemble, et un
    // binaire qu'on ouvre. La seconde manquait, et c'est elle qui a
    // laissé passer le repli « Détail » rogné — la prochaine
    // instruction d'un binaire tient deux lignes de plus.
    let binaire = binaire_dépreuve("crackme-01");
    let séances: Vec<(&str, bool)> = match &binaire {
        Some(_) => vec![("source", false), ("binaire", true)],
        None => vec![("source", false)],
    };
    for (séance, depuis_binaire) in séances {
        // Les deux sortes de séance, et à toutes les tailles courantes.
        // Le cas du binaire à 1280×820 a longtemps été hors de portée :
        // une instruction de binaire porte deux lignes de plus, et
        // aucun partage n'y changeait rien. Il tient depuis que
        // « Prochaine instruction » se replie sur un binaire.
        for &(largeur, hauteur) in &[(1280.0_f32, 820.0_f32), (1440.0, 900.0), (1920.0, 1080.0)] {
            let ctx = egui::Context::default();
            let mut app = NamsdecApp {
                source: DEFAULT_SOURCE.to_owned(),
                show_sidebar: true,
                show_console: true,
                ..Default::default()
            };
            app.apply_theme(&ctx);
            if depuis_binaire {
                assert!(app.load_elf(binaire.as_ref().unwrap(), "crackme-01"));
            } else {
                app.do_assemble();
                assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
            }
            let entrée = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(largeur, hauteur),
                )),
                ..Default::default()
            };
            let mut coupés: Vec<String> = Vec::new();
            for _ in 0..6 {
                coupés.clear();
                let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
                // Le haut de l'onglet : ce qui est en dessous est une liste.
                let onglet =
                    egui::containers::panel::PanelState::load(&ctx, egui::Id::new("sidebar_tab"))
                        .map(|é| é.outer_rect.top())
                        .unwrap_or(f32::INFINITY);
                for clipped in &sortie.shapes {
                    if let egui::epaint::Shape::Text(t) = &clipped.shape {
                        let posé = egui::Rect::from_min_size(t.pos, t.galley.size());
                        if posé.bottom() > onglet {
                            continue;
                        }
                        let vu = posé.intersect(clipped.clip_rect);
                        // Coupée « en deux » : une part visible, mais pas
                        // toute. Rognée à néant, c'est du défilement.
                        if vu.height() > 0.5 && vu.height() < posé.height() - 0.5 {
                            coupés.push(format!(
                                "« {} » {:.0} px sur {:.0}",
                                t.galley.text().chars().take(24).collect::<String>(),
                                vu.height(),
                                posé.height()
                            ));
                        }
                    }
                }
                sortie.textures_delta.clear();
            }
            assert!(
                coupés.is_empty(),
                "séance « {séance} », à {largeur}×{hauteur}, le bandeau coupe des \
             lignes en deux : {coupés:?}"
            );
        }
    }
}

/// « Prochaine instruction » se replie sur un binaire, et pas sur un
/// source.
///
/// Une instruction de binaire porte deux lignes de plus — son adresse
/// absolue et son pseudo-code — et le bandeau n'avait plus la place de
/// finir sur une fenêtre de 1280×820. Repliée, la section laisse son
/// titre, qu'un clic rouvre.
///
/// Et elle reste dépliée devant un source qu'on assemble : c'est là
/// qu'on regarde l'instruction qui vient, pas le listing.
#[test]
fn the_next_instruction_folds_for_a_binary_and_not_for_a_source() {
    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
        assert!(
            app.next_instruction_open,
            "devant son propre programme, l'instruction qui vient est ce qu'on lit"
        );
    }

    let Some(octets) = binaire_dépreuve("crackme-01") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.next_instruction_open, "dépliée au départ");
    assert!(app.load_elf(&octets, "crackme-01"), "l'ELF se charge");
    assert!(
        !app.next_instruction_open,
        "un binaire replie la section : elle porte deux lignes de plus, \
         et la place manque"
    );

    // Mais ce n'est qu'un départ : rouverte, elle le reste — on ne
    // referme pas sous les doigts de quelqu'un qui vient d'ouvrir.
    app.next_instruction_open = true;
    let peint = paint(&mut app);
    assert!(
        peint.contains(strings(Lang::Fr).next_instruction),
        "le titre est toujours là :\n{peint}"
    );
    assert!(
        app.next_instruction_open,
        "dessiner ne referme pas ce que l'on a ouvert"
    );
}

/// Les fenêtres qu'on avait laissées ouvertes sont là au retour.
///
/// Fermer la barre d'outils la gardait fermée au lancement suivant ;
/// ouvrir la calculatrice, non. Ce qu'on a disposé autour de son code
/// est pourtant un réglage comme un autre, et il n'y a pas de raison
/// que la moitié des gestes survive à la séance et l'autre pas.
///
/// Le test passe par le fichier — instantané, puis reconstruction à
/// partir de lui — et non par une copie du champ : c'est le voyage
/// complet qui compte, et c'est lui qui casse quand on ajoute une
/// fenêtre en oubliant de la nommer dans la table.
///
/// Il dit aussi ce qui ne revient PAS. Préférences, « À propos » et la
/// saisie du programme s'ouvrent sur une question et se ferment sur la
/// réponse ; les retrouver au lancement serait répondre à une question
/// que personne n'a posée.
#[test]
fn the_windows_left_open_come_back_next_time() {
    for (nom, ouverte, ouvre) in FENÊTRES_REPRISES {
        let mut app = NamsdecApp {
            // Les deux fenêtres de module ne s'ouvrent que si leur
            // module est allumé — c'est le sens de leur reprise.
            show_decompiler_module: true,
            exercises: true,
            ..Default::default()
        };
        ouvre(&mut app);
        assert!(
            ouverte(&app),
            "« {nom} » doit s'ouvrir quand on le lui demande"
        );
        let revenue = NamsdecApp::from_settings(app.settings_snapshot());
        assert!(
            ouverte(&revenue),
            "« {nom} » était ouverte, elle doit revenir ouverte"
        );
    }

    // Et l'inverse : ce qui n'est pas d'établi ne revient pas.
    let mut app = NamsdecApp {
        show_preferences: true,
        show_about: true,
        show_input_dialog: true,
        ..Default::default()
    };
    app.show_calculator = true;
    let revenue = NamsdecApp::from_settings(app.settings_snapshot());
    assert!(
        revenue.show_calculator,
        "la calculatrice, elle, revient — sans quoi ce test ne prouve rien"
    );
    for (quoi, encore) in [
        ("les préférences", revenue.show_preferences),
        ("« À propos »", revenue.show_about),
        ("la saisie du programme", revenue.show_input_dialog),
    ] {
        assert!(
            !encore,
            "{quoi} ne doit pas se rouvrir tout seul au lancement"
        );
    }

    // Un nom venu d'une version plus récente ne fait pas de bruit.
    let _ = NamsdecApp::from_settings(Settings {
        open_windows: vec!["une_fenetre_du_futur".to_owned()],
        ..Settings::default()
    });
}

/// Deux fenêtres ouvertes ensemble ne se posent pas au même endroit.
///
/// Toutes partaient du centre exact du viewport, avec un pivot centré :
/// elles se superposaient AU PIXEL PRÈS, et celle du dessous devenait
/// introuvable — on la cherchait dans le menu Affichage en la croyant
/// fermée. Le défaut dormait tant qu'on les ouvrait une à une ; la
/// reprise de séance, qui peut en rouvrir douze, l'a mis au jour.
///
/// Le contrôle porte sur le CENTRE, et c'est le premier essai de ce test
/// qui l'a appris : comparer les coins haut-gauche ne voyait rien du
/// tout. Le pivot des fenêtres est `CENTER_CENTER` — posées au même
/// point, deux fenêtres de tailles différentes ont des coins différents
/// tout en étant parfaitement concentriques, c'est-à-dire empilées. Le
/// coin ne dit pas où la fenêtre a été posée ; le centre, si.
///
/// Le test a donc été éprouvé dans les deux sens : replacé sur le centre
/// de l'écran, il échoue en nommant les fenêtres confondues.
///
/// Et elles restent sur l'écran : un éventail qui pousse la dernière
/// fenêtre hors du viewport échange un défaut contre un pire.
#[test]
fn two_windows_opened_together_do_not_land_on_the_same_spot() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        // Les deux fenêtres de module ne s'ouvrent qu'avec lui.
        show_decompiler_module: true,
        exercises: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    for (_, _, ouvre) in FENÊTRES_REPRISES {
        ouvre(&mut app);
    }
    let entrée = écran_du_banc();
    let écran = entrée.screen_rect.expect("l'écran du banc");
    // Quatre images : une fenêtre dont la hauteur épouse son contenu ne
    // se pose pas du premier coup.
    for _ in 0..4 {
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }

    let mut posées: Vec<(&str, egui::Rect)> = Vec::new();
    for (nom, ouverte) in app.window_states() {
        if !ouverte {
            continue;
        }
        let Some(état) = egui::AreaState::load(&ctx, egui::Id::new(nom)) else {
            continue;
        };
        posées.push((nom, état.rect()));
    }
    assert!(
        posées.len() >= 10,
        "il faut plusieurs fenêtres pour que le test prouve quelque chose — {} \
         seulement : {:?}",
        posées.len(),
        posées.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );

    for (i, (nom_a, a)) in posées.iter().enumerate() {
        for (nom_b, b) in &posées[i + 1..] {
            let écart = a.center().distance(b.center());
            assert!(
                écart > 8.0,
                "« {nom_a} » et « {nom_b} » se posent au même endroit — \
                 centre {:?} contre {:?}",
                a.center(),
                b.center()
            );
        }
        assert!(
            écran.expand(1.0).contains_rect(*a),
            "« {nom_a} » doit tenir dans l'écran — {a:?} pour un écran de {écran:?}"
        );
    }
}

/// Aucun écran ne montre son balisage.
///
/// Les textes d'explication sont écrits dans une petite convention :
/// accents graves pour le code, doubles astérisques pour un passage
/// appuyé. `prose` les rend — mais il ne rendait que les accents graves,
/// et l'explication du motif cyclique s'affichait « une suite dont
/// \*\*toute tranche de quatre octets est unique\*\* », astérisques
/// comprises. Une capture l'a montré ; aucun test ne le demandait.
///
/// Le contrôle porte sur ce qui est PEINT et non sur les chaînes du
/// fichier de traduction : c'est le rendu qui est en cause, pas le
/// texte. Écrire `**` dans une nouvelle phrase est donc permis — ce qui
/// ne l'est pas, c'est de l'afficher à travers un chemin qui ne le rend
/// pas.
#[test]
fn no_screen_shows_its_own_markup() {
    for (nom, id, ouvre) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, id, 4);
        for (texte, _) in &lignes {
            assert!(
                !texte.contains("**"),
                "« {nom} » montre son balisage : {:?}",
                texte.chars().take(90).collect::<String>()
            );
        }
    }
}

/// Les points de suspension sont des points de suspension.
///
/// Soixante-huit libellés portaient le caractère « … » et dix portaient
/// trois points ASCII : « Motif cyclique... » dans le menu Outils juste
/// au-dessus de « Calculatrice… ». À l'écran, les deux ne font pas la
/// même largeur ni le même gris — c'est le genre d'écart qu'on ne sait
/// pas nommer en le voyant, et qui donne l'impression que le programme
/// a été écrit par deux personnes qui ne se parlaient pas.
///
/// Le contrôle lit le fichier des traductions plutôt que l'écran :
/// toutes les phrases ne sont pas affichées en même temps, et celle qui
/// ne l'est pas aujourd'hui le sera demain.
#[test]
fn an_ellipsis_is_written_as_an_ellipsis() {
    let source = include_str!("i18n.rs");
    let fautives: Vec<String> = source
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("...") && l.contains('"'))
        .map(|(n, l)| format!("i18n.rs:{} {}", n + 1, l.trim()))
        .collect();
    assert!(
        fautives.is_empty(),
        "{} libellé(s) avec trois points au lieu de « … » :\n  {}",
        fautives.len(),
        fautives.join("\n  ")
    );
}

/// Une section dépliée n'est jamais muette.
///
/// Au premier lancement, « Prochaine instruction » était dépliée — c'est
/// voulu devant un source — et ne peignait RIEN : un titre, un filet, et
/// deux cents pixels de blanc, entre deux voisines qui, elles,
/// annonçaient ce qu'elles montreraient. Le vide n'était pas un défaut
/// de mise en page mais un défaut de PAROLE : la section avait la place,
/// elle n'avait pas la phrase.
///
/// Le contrôle regarde ce qui est peint entre le titre d'une section et
/// le titre suivant. Replié, un titre n'a rien en dessous et c'est
/// normal : on ne regarde donc que les sections ouvertes, reconnues au
/// chevron « ▾ » qu'egui pose devant elles.
#[test]
fn an_open_section_always_has_something_to_say() {
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            lang,
            source: DEFAULT_SOURCE.to_owned(),
            show_sidebar: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);

        // Le bandeau seul : les titres des sections y sont, et le reste
        // de l'écran ne nous regarde pas ici.
        let titre = |quoi: &str| -> egui::Rect {
            lignes
                .iter()
                .find(|(t, _)| t == quoi)
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("{lang:?} : « {quoi} » est peint"))
        };
        let haut = titre(s.next_instruction);
        let bas = titre(s.cpu_state);
        assert!(
            haut.bottom() < bas.top(),
            "{lang:?} : « {} » est au-dessus de « {} »",
            s.next_instruction,
            s.cpu_state
        );
        let dedans = lignes.iter().any(|(t, r)| {
            r.top() > haut.bottom()
                && r.bottom() < bas.top()
                && r.left() > haut.left() - 40.0
                && !t.is_empty()
        });
        assert!(
            dedans,
            "{lang:?} : « {} » est dépliée et ne dit rien — {:.0} px de blanc \
             entre elle et « {} »",
            s.next_instruction,
            bas.top() - haut.bottom(),
            s.cpu_state
        );
    }
}

/// Deux mots identiques ne se touchent pas sur une même ligne.
///
/// Dans la fenêtre du graphe, le bouton qui remet le dessin à sa taille
/// s'appelait « 100 % » — comme l'affichage du grossissement posé vingt
/// pixels à sa gauche. L'un gris, l'autre encadré, et le même texte :
/// on lisait un bégaiement, et rien ne disait ce que le bouton faisait.
/// Il s'appelle maintenant « Taille réelle », du nom de son geste et non
/// de la valeur qu'il rétablit.
///
/// QUATRE SIGNES AU MOINS, et ce seuil vient d'un faux positif : à un
/// signe, la calculatrice déclenchait la règle des dizaines de fois, ses
/// soixante-quatre bits affichant « 0 » côte à côte. Un chiffre répété
/// dans un tableau est une donnée ; un mot répété à côté de lui-même est
/// une maladresse. Le seuil sépare les deux sans avoir à les nommer.
#[test]
fn two_identical_words_never_sit_side_by_side() {
    for (nom, id, ouvre) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, id, 4);
        for (i, (ta, ra)) in lignes.iter().enumerate() {
            for (tb, rb) in &lignes[i + 1..] {
                if ta != tb || ta.trim().chars().count() < 4 {
                    continue;
                }
                // Sur la même ligne : quatre pixels de tolérance, les
                // hampes et les jambages ne décalent pas un centre.
                if (ra.center().y - rb.center().y).abs() > 4.0 {
                    continue;
                }
                let trou = (rb.left() - ra.right())
                    .abs()
                    .min((ra.left() - rb.right()).abs());
                assert!(
                    trou >= 60.0,
                    "« {nom} » bégaie : « {ta} » deux fois sur la même ligne, \
                     à {trou:.0} px l'un de l'autre"
                );
            }
        }
    }
}

/// Ce qui se dit « chasse fixe » l'est vraiment.
///
/// `RichText::monospace()` et `RichText::small()` posent le MÊME champ,
/// `text_style` : enchaînés, le second efface le premier. Quatre
/// endroits écrivaient `.monospace().small()` en croyant demander du
/// code en petit, et obtenaient du proportionnel en petit — le JSON
/// d'exemple du décompilateur s'affichait accolades et indentation
/// désalignées, trente pixels sous un chemin en chasse fixe, dans la
/// même fenêtre. L'intention était écrite dans le code ; le résultat
/// peint disait le contraire.
///
/// Le contrôle lit le source, parce que la règle est là : un style de
/// texte s'écrase, donc on n'en enchaîne pas deux. `small_monospace`
/// pose un `FontId` explicite, qui dit la famille ET la taille.
#[test]
fn what_calls_itself_monospace_really_is() {
    let fautives: Vec<String> = SOURCES_DE_L_INTERFACE
        .iter()
        .flat_map(|(fichier, source)| {
            source
                .lines()
                .enumerate()
                .filter(|(_, l)| {
                    (l.contains(".monospace().small()") || l.contains(".small().monospace()"))
                        && !l.trim_start().starts_with("//")
                })
                .map(move |(n, l)| format!("{fichier}:{} {}", n + 1, l.trim()))
        })
        .collect();
    assert!(
        fautives.is_empty(),
        "{} texte(s) qui se croient en chasse fixe et ne le sont pas — \
         `small_monospace` le dit pour de bon :\n  {}",
        fautives.len(),
        fautives.join("\n  ")
    );
}

/// Une colonne de réglages garde le même rythme.
///
/// Dans Préférences › Général, huit cases à cocher du même rang se
/// suivaient avec deux respirations : 13,5 px entre les quatre
/// premières, 7,5 px entre les quatre dernières. La cause était un
/// `add_space(6.0)` posé devant les premières et oublié devant les
/// autres. Personne ne sait nommer ce défaut en le voyant ; tout le
/// monde voit que quelque chose cloche, sur l'écran où l'on regarde le
/// plus longtemps une colonne.
///
/// Le contrôle mesure les écarts entre les étiquettes successives et
/// demande qu'ils soient tous égaux. Un demi-pixel de tolérance : les
/// hampes et les jambages changent la hauteur d'une encre selon les
/// lettres qu'elle porte.
#[test]
fn a_column_of_settings_keeps_one_rhythm() {
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_preferences: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("preferences_window"), 4);
    // La DERNIÈRE ligne du texte, et la PREMIÈRE de l'étiquette : une
    // explication qui se replie sur deux lignes rend autant d'entrées
    // que de lignes, et prendre la première ferait compter l'écart
    // depuis le milieu du paragraphe. Ce faux positif a été rencontré
    // ici même : la première explication se replie, et l'écart qui la
    // suit paraissait deux fois plus grand que les autres.
    let bas_de = |quoi: &str| -> f32 {
        lignes
            .iter()
            .filter(|(t, _)| t == quoi)
            .map(|(_, r)| r.bottom())
            .fold(f32::NEG_INFINITY, f32::max)
    };
    let haut_de = |quoi: &str| -> f32 {
        lignes
            .iter()
            .filter(|(t, _)| t == quoi)
            .map(|(_, r)| r.top())
            .fold(f32::INFINITY, f32::min)
    };
    // Les sept réglages de la page Général, dans l'ordre où on les lit.
    let colonne = [
        (
            s.preferences_system_titlebar,
            s.preferences_system_titlebar_hint,
        ),
        (s.preferences_heat_legend, s.preferences_heat_legend_hint),
        (s.preferences_tooltips, s.preferences_tooltips_hint),
        (s.preferences_completion, s.preferences_completion_hint),
        (s.preferences_auto_indent, s.preferences_auto_indent_hint),
        (s.preferences_auto_pairs, s.preferences_auto_pairs_hint),
        (s.preferences_hover_help, s.preferences_hover_help_hint),
    ];
    let mut écarts: Vec<(String, f32)> = Vec::new();
    for paire in colonne.windows(2) {
        let bas = bas_de(paire[0].1);
        let haut = haut_de(paire[1].0);
        assert!(
            bas.is_finite() && haut.is_finite(),
            "« {} » et « {} » sont peints dans les préférences",
            paire[0].1,
            paire[1].0
        );
        écarts.push((paire[1].0.to_owned(), haut - bas));
    }
    let premier = écarts[0].1;
    let hors: Vec<String> = écarts
        .iter()
        .filter(|(_, e)| (e - premier).abs() > 0.5)
        .map(|(quoi, e)| format!("« {quoi} » à {e:.1} px"))
        .collect();
    assert!(
        hors.is_empty(),
        "la colonne change de rythme : {premier:.1} px partout, sauf {}",
        hors.join(", ")
    );
}

/// Deux boutons côte à côte ont la même hauteur.
///
/// Dans la barre des Préférences, « OK » faisait 25 px et « Quitter »,
/// posé huit pixels à sa droite, 27 : egui ajoute la largeur du trait à
/// la taille d'un bouton tracé, et « OK » est plein quand « Quitter »
/// est encadré. Deux pixels sur une rangée de deux boutons, c'est peu à
/// dire et très visible — l'un dépasse l'autre en haut ET en bas.
///
/// Le remède est un liseré de la couleur du fond : invisible, et il
/// rend la même hauteur. Le contrôle, lui, ne connaît ni l'un ni
/// l'autre : il relève les rectangles arrondis d'une taille de bouton,
/// et n'accepte pas deux voisins de hauteurs différentes.
#[test]
fn two_buttons_side_by_side_share_a_height() {
    for (nom, _, ouvre) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let entrée = écran_du_banc();
        let mut boîtes: Vec<egui::Rect> = Vec::new();
        for _ in 0..4 {
            boîtes.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                // Un rectangle arrondi, de la taille d'un bouton. Les
                // bornes écartent les fonds de fenêtre et les pastilles
                // d'indicateur, qui ne sont pas des boutons.
                //
                // VINGT PIXELS AU MOINS, et ce plancher vient d'un faux
                // positif : un article de la barre de menus fait dix-huit
                // pixels de haut et il est arrondi comme un bouton. Il
                // n'en est pas un — on ne lui demande pas de s'aligner
                // sur les boutons d'un formulaire, et la règle n'a rien
                // à dire entre deux barres différentes.
                if let egui::epaint::Shape::Rect(r) = &clipped.shape
                    && r.corner_radius.nw > 0
                    && (20.0..40.0).contains(&r.rect.height())
                    && (40.0..300.0).contains(&r.rect.width())
                {
                    boîtes.push(r.rect);
                }
            }
            sortie.textures_delta.clear();
        }
        for (i, a) in boîtes.iter().enumerate() {
            for b in &boîtes[i + 1..] {
                // Sur la même rangée : six pixels de tolérance, un
                // bouton plus haut que son voisin décale son centre.
                if (a.center().y - b.center().y).abs() > 6.0 {
                    continue;
                }
                let trou = (b.left() - a.right())
                    .abs()
                    .min((a.left() - b.right()).abs());
                if trou >= 40.0 {
                    continue;
                }
                assert!(
                    (a.height() - b.height()).abs() <= 0.5,
                    "« {nom} » : deux boutons voisins de hauteurs différentes — \
                     {:.1} px contre {:.1} px, à {trou:.0} px l'un de l'autre",
                    a.height(),
                    b.height()
                );
            }
        }
    }
}

/// Les titres du bandeau partent tous du même bord.
///
/// Le panneau latéral a trois zones, et elles avaient trois en-têtes
/// différents : « État du processeur » par l'en-tête repliable d'egui,
/// « Prochaine instruction » et « Détail » par `section_toggle`, « Pile »
/// par `section_header`. Trois dessins, deux chevrons — « ▾ » dessiné
/// par egui d'un côté, un « v » écrit de l'autre — et trois abscisses :
/// 1229,0, 1243,2 et 1247,0. Presque rien, et c'est exactement ce qui se
/// voit sans qu'on sache le nommer.
///
/// Un titre replié s'écarte légitimement : il porte un chevron devant
/// lui. Le contrôle demande donc que les titres À CHEVRON soient tous
/// alignés entre eux, et que celui qui n'en porte pas parte du bord du
/// panneau.
#[test]
fn the_banner_titles_all_start_at_one_edge() {
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_sidebar: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
    let gauche = |quoi: &str| -> f32 {
        lignes
            .iter()
            .filter(|(t, _)| t == quoi)
            .map(|(_, r)| r.left())
            .fold(f32::INFINITY, f32::min)
    };
    let à_chevron = [s.next_instruction, s.cpu_state];
    let bords: Vec<(&str, f32)> = à_chevron.iter().map(|t| (*t, gauche(t))).collect();
    for (quoi, x) in &bords {
        assert!(x.is_finite(), "« {quoi} » est peint dans le bandeau");
    }
    let premier = bords[0].1;
    let hors: Vec<String> = bords
        .iter()
        .filter(|(_, x)| (x - premier).abs() > 0.5)
        .map(|(quoi, x)| format!("« {quoi} » à {x:.1}"))
        .collect();
    assert!(
        hors.is_empty(),
        "les titres du bandeau ne partent pas du même bord : {premier:.1} px, \
         sauf {}",
        hors.join(", ")
    );
}

/// Les trois zones du panneau se soulignent de la même façon.
///
/// Un titre de zone porte un filet EN DESSOUS, et rien au-dessus. C'est
/// vrai de « Prochaine instruction » et de « État du processeur », qui
/// passent par `section_toggle` ; ce ne l'était pas de « Pile », que le
/// panneau d'onglet faisait précéder d'un filet à lui avant que
/// `section_header` n'en pose un second. Deux traits à trente-quatre
/// pixels l'un de l'autre, un seul titre entre eux : on lisait une
/// rayure sans savoir d'où elle venait, et la troisième zone ne
/// ressemblait pas aux deux autres.
///
/// Le contrôle demande qu'aucun filet ne passe juste AU-DESSUS d'un
/// titre de zone. « Juste » vaut vingt-quatre pixels : au-delà, le
/// filet appartient à ce qui précède.
#[test]
fn a_zone_title_is_underlined_and_not_boxed() {
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_sidebar: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = écran_du_banc();
    let mut filets: Vec<f32> = Vec::new();
    for _ in 0..4 {
        filets.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            // Un trait plein, pleine largeur, dans le panneau de droite.
            if let egui::epaint::Shape::Rect(r) = &clipped.shape
                && r.rect.height() <= 1.5
                && r.rect.width() > 200.0
                && r.rect.left() > 1200.0
            {
                filets.push(r.rect.top());
            }
        }
        sortie.textures_delta.clear();
    }
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
    // Les titres se cherchent DE HAUT EN BAS, chacun sous le précédent.
    // « Pile » est écrit deux fois à l'écran — l'onglet de la barre
    // d'outils et le titre de la zone — et prendre le plus haut des deux
    // faisait mesurer la barre d'outils : le contrôle passait alors même
    // avec le défaut remis, ce qui est la pire sorte de contrôle.
    let mut plancher = 0.0f32;
    for titre in [s.next_instruction, s.cpu_state, SidebarTab::Stack.label(s)] {
        let haut = lignes
            .iter()
            .filter(|(t, r)| t == titre && r.top() > plancher)
            .map(|(_, r)| r.top())
            .fold(f32::INFINITY, f32::min);
        assert!(haut.is_finite(), "« {titre} » est peint dans le bandeau");
        plancher = haut + 1.0;
        let dessus: Vec<f32> = filets
            .iter()
            .copied()
            .filter(|y| *y < haut && haut - *y < 24.0)
            .collect();
        assert!(
            dessus.is_empty(),
            "« {titre} » porte un filet au-dessus de lui, en y = {dessus:?} \
             pour un titre en {haut:.0} — les trois zones doivent se souligner \
             de la même façon"
        );
    }
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
    // laquelle commence au bord du panneau.
    let visée = egui::pos2(8.0, numéro.center().y);

    /// Les ronds pleins d'un rayon de point d'arrêt peints sur cette ligne.
    fn points(sortie: &egui::FullOutput, y: f32) -> Vec<egui::Color32> {
        let mut trouvés = Vec::new();
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Circle(c) = &clipped.shape
                && (c.radius - 4.0).abs() < 0.5
                && (c.center.y - y).abs() < 4.0
                && c.center.x < 40.0
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
            if let egui::epaint::Shape::Rect(r) = &clipped.shape
                && r.rect.left() < 4.0
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

/// Un panneau vide se dit toujours de la même façon.
///
/// Neuf panneaux employaient `empty_state` — un bloc centré : le
/// constat, puis ce qu'on verra là quand il y aura quelque chose.
/// Cinq fenêtres écrivaient à la main une phrase seule, alignée à
/// gauche, qui disait le constat et taisait le reste : la chronologie,
/// les points d'arrêt, les correctifs, le graphe, le décompilateur.
/// Deux formes pour un même rôle, et aucune assez majoritaire pour
/// servir de norme — c'est le bloc qui l'emporte, parce qu'il DIT
/// quelque chose de plus.
///
/// Le contrôle demande que chaque constat soit CENTRÉ dans sa fenêtre
/// et suivi de son explication. Centré : c'est ce qui distingue les
/// deux formes à l'œil, et c'est mesurable sans rien savoir du reste.
///
/// Une exception nommée : « Aucun décompilateur installé. » n'est pas
/// un état vide mais un mode d'emploi — le constat, puis le dossier où
/// déposer un fichier, puis un exemple de ce fichier. Centrer les deux
/// premières lignes et laisser le reste à gauche ferait pire que le
/// défaut qu'on corrige.
#[test]
fn an_empty_panel_always_says_it_the_same_way() {
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        let constats = [
            (s.no_binary_assembled, true),
            (s.timeline_empty, true),
            (s.breakpoints_empty, true),
            (s.patch_none_yet, true),
            (s.flow_empty, true),
            (s.decompiler_no_binary, true),
        ];
        for (nom, id, ouvre) in scènes() {
            let Some(id) = id else { continue };
            let ctx = egui::Context::default();
            let mut app = NamsdecApp {
                lang,
                ..Default::default()
            };
            app.apply_theme(&ctx);
            ouvre(&mut app);
            let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some(id), 4);
            for (constat, _) in constats.iter().filter(|(_, exigé)| *exigé) {
                let Some((_, place)) = lignes.iter().find(|(t, _)| t == constat) else {
                    continue;
                };
                // L'explication, juste dessous : le constat dit ce qui
                // manque, elle dit ce qu'on verra là quand il y aura
                // quelque chose.
                let dessous = lignes
                    .iter()
                    .filter(|(t, r)| {
                        t != constat && r.top() > place.bottom() && r.top() - place.bottom() < 40.0
                    })
                    .min_by(|a, b| a.1.top().total_cmp(&b.1.top()));
                let Some((_, détail)) = dessous else {
                    panic!(
                        "{lang:?} « {nom} » : « {constat} » ne dit que ce qui manque, \
                         pas ce que le panneau montrera"
                    )
                };
                // CENTRÉS L'UN SUR L'AUTRE, et non centrés dans la
                // fenêtre : la fenêtre d'analyse porte une liste de
                // sections à gauche, si bien que son bloc est centré
                // dans sa colonne et non dans son cadre — la mesurer
                // depuis le cadre accusait un panneau parfaitement
                // rangé, à cent quatre pixels du milieu. Deux lignes de
                // longueurs différentes qui partagent leur milieu sont
                // centrées ; deux lignes alignées à gauche ne le sont
                // pas. La règle n'a besoin de rien d'autre.
                let écart = (place.center().x - détail.center().x).abs();
                assert!(
                    écart < 6.0,
                    "{lang:?} « {nom} » : « {constat} » et son explication ne sont \
                     pas centrés l'un sur l'autre — {écart:.0} px d'écart, \
                     c'est la forme alignée à gauche"
                );
            }
        }
    }
}

/// Le plus grand vide vertical d'une fenêtre, toutes formes comptées.
///
/// Les textes ne suffisent pas : l'image du bouton de don, l'histogramme
/// d'analyse et le tracé de la chronologie sont du contenu, et les
/// compter pour rien accuserait des fenêtres pleines.
///
/// Les fonds de la fenêtre elle-même, eux, ne comptent pas : un
/// rectangle qui couvre tout le cadre remplirait le vide à lui seul.
fn plus_grand_vide(ctx: &egui::Context, app: &mut NamsdecApp, id: &str) -> (f32, egui::Rect) {
    let entrée = écran_du_banc();
    let mut bandes: Vec<(f32, f32)> = Vec::new();
    let mut cadre = egui::Rect::NOTHING;
    for _ in 0..5 {
        bandes.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        cadre = egui::AreaState::load(ctx, egui::Id::new(id))
            .map(|a| a.rect())
            .unwrap_or(egui::Rect::NOTHING);
        for clipped in &sortie.shapes {
            let large = cadre.x_range().expand(2.0);
            if !large.contains(clipped.clip_rect.left())
                || !large.contains(clipped.clip_rect.right())
            {
                continue;
            }
            let vu = clipped
                .shape
                .visual_bounding_rect()
                .intersect(clipped.clip_rect);
            if !vu.is_positive() {
                continue;
            }
            // Un fond de fenêtre n'est pas du contenu.
            if vu.width() > cadre.width() * 0.8 && vu.height() > cadre.height() * 0.8 {
                continue;
            }
            bandes.push((vu.top(), vu.bottom()));
        }
        sortie.textures_delta.clear();
    }
    bandes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut fusion: Vec<(f32, f32)> = Vec::new();
    for (haut, bas) in bandes {
        match fusion.last_mut() {
            Some(dernière) if haut <= dernière.1 => dernière.1 = dernière.1.max(bas),
            _ => fusion.push((haut, bas)),
        }
    }
    let trou = fusion
        .windows(2)
        .map(|p| p[1].0 - p[0].1)
        .fold(0.0f32, f32::max);
    (trou, cadre)
}

/// Une fenêtre s'ouvre à la taille de ce qu'elle montre.
///
/// Elles s'ouvraient à la taille de ce qu'elles montrent QUAND elles
/// montrent quelque chose. Sans binaire, le graphe prenait donc
/// quatre-vingt-douze pour cent de l'écran pour une phrase et son
/// explication — 1472 px de large, 380 px de vide entre le texte et le
/// bouton « OK » — et les registres étendus 520 x 620 pour deux lignes.
/// C'est l'écran du PREMIER lancement.
///
/// La mesure est le plus grand VIDE entre deux bandes de contenu, et
/// non le blanc sous le dernier texte : celui-ci vaut vingt-six pixels
/// dans toutes les fenêtres, la barre « OK » étant ancrée au bas du
/// cadre, et ne tomberait jamais. Elle compte toutes les formes et pas
/// seulement les textes : l'image du bouton de don, l'histogramme
/// d'analyse et le tracé de la chronologie sont du contenu.
///
/// Quatre-vingts pixels : la plus grande fenêtre saine en montre
/// soixante-onze, et c'est le motif cyclique, dont le champ de saisie
/// laisse de l'air autour de lui.
///
/// UNE EXCEPTION NOMMÉE : le graphe, programme chargé. C'est une toile
/// où l'on se déplace et où l'on zoome ; le blanc autour du dessin n'y
/// est pas du vide mais de la place pour bouger. À vide, en revanche,
/// il n'y a pas de toile et la règle s'applique.
#[test]
fn a_window_opens_at_the_size_of_what_it_shows() {
    for (nom, id, ouvre) in scènes() {
        let Some(id) = id else { continue };
        if nom.starts_with("graphe (en marche)") {
            continue;
        }
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let (trou, cadre) = plus_grand_vide(&ctx, &mut app, id);
        assert!(
            trou <= 80.0,
            "« {nom} » s'ouvre en {:.0}x{:.0} et laisse {trou:.0} px de vide au \
             milieu : elle est plus grande que ce qu'elle montre",
            cadre.width(),
            cadre.height()
        );
    }
}

/// Les colonnes du listing ne dansent pas d'une ligne à l'autre.
///
/// La colonne des étiquettes n'avait pas de largeur : une rangée sans
/// étiquette ne réservait rien, et les octets comme le texte de
/// l'instruction reculaient de cinquante-cinq pixels dès qu'une ligne en
/// portait une. Un listing dont les colonnes bougent ne se lit plus en
/// colonnes, et c'est tout ce qu'on lui demande.
///
/// Le contrôle relève les abscisses de la colonne des octets — repérable
/// à sa forme, des paires hexadécimales séparées par des espaces — et
/// demande qu'elles soient toutes les mêmes.
#[test]
fn the_listing_keeps_its_columns_from_line_to_line() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        show_disassembly: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("disassembly_window"), 5);

    // La colonne des octets : du texte qui n'est que des chiffres
    // hexadécimaux et des espaces, et qui en contient au moins quatre.
    let mut bords: Vec<(String, f32)> = Vec::new();
    for (texte, place) in &lignes {
        let net = texte.trim_end();
        if net.len() >= 4
            && net.chars().all(|c| c.is_ascii_hexdigit() || c == ' ')
            && net.contains(' ')
        {
            bords.push((net.to_owned(), place.left()));
        }
    }
    assert!(
        bords.len() >= 4,
        "il faut plusieurs lignes d'octets pour que le test prouve quelque \
         chose — {bords:?}"
    );
    let premier = bords[0].1;
    let hors: Vec<String> = bords
        .iter()
        .filter(|(_, x)| (x - premier).abs() > 1.0)
        .map(|(t, x)| format!("« {t} » à {x:.1}"))
        .collect();
    assert!(
        hors.is_empty(),
        "la colonne des octets commence à {premier:.1}, sauf {}",
        hors.join(", ")
    );
}

/// Un changement d'échelle redemande une image.
///
/// Deux captures du 25 septembre au soir montraient le panneau de droite
/// avec deux blocs sur les mêmes pixels : l'état du processeur et le
/// haut de l'onglet Mémoire. Mesuré SUR L'IMAGE : « RIP » et « Carte de
/// la mémoire » font 33 et 31 pixels de haut, et les textes superposés à
/// eux en font dix-huit. Ce n'est donc pas une mise en page qui se
/// recouvre — egui dessine une image à une seule échelle — mais une
/// bande restée d'un rendu précédent, plus petit, que rien n'a
/// repeinte : le facteur d'échelle de l'écran avait changé entre les
/// deux.
///
/// Trois hypothèses de mise en page avaient été essayées avant celle-là,
/// et le banc ne retrouvait le défaut à aucune taille. Il ne pouvait
/// pas : le banc dessine à une échelle et une seule.
///
/// On ne répare pas le compositeur ; on ne l'attend pas non plus.
#[test]
fn a_change_of_scale_asks_for_a_new_frame() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let écran = |échelle: f32| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        viewports: std::iter::once((
            egui::ViewportId::ROOT,
            egui::ViewportInfo {
                native_pixels_per_point: Some(échelle),
                ..Default::default()
            },
        ))
        .collect(),
        ..Default::default()
    };
    // Deux images à la même échelle : le compte est posé.
    for _ in 0..2 {
        let mut sortie = ctx.run_ui(écran(2.0), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    let posée = app.dernière_échelle;
    assert_eq!(posée, Some(2.0), "l'échelle courante est retenue");

    // Elle change : l'application doit redemander une image, sans quoi
    // ce que le compositeur a agrandi reste à l'écran.
    let mut sortie = ctx.run_ui(écran(3.0), |ui| app.draw(ui));
    sortie.textures_delta.clear();
    assert_eq!(
        app.dernière_échelle,
        Some(3.0),
        "et la nouvelle échelle est retenue à son tour"
    );
}

/// Une commande grisée dit pourquoi elle l'est.
///
/// `on_hover_text` ne s'affiche PAS sur un widget éteint — egui réserve
/// l'infobulle ordinaire aux widgets vivants. La barre d'outils portait
/// donc cinq boutons gris — Exécuter, Step Into, Step Over, Step Back,
/// Stop — qui ne disaient rien du tout au survol. C'est l'écran qu'on a
/// sous les yeux tant qu'on n'a pas assemblé, et une commande éteinte
/// qui se tait passe pour cassée.
///
/// Dire ce qui manque, c'est enseigner l'ordre des gestes : « Rien à
/// exécuter : assemblez d'abord le programme. » C'est ce que namsdec est
/// là pour faire.
///
/// Le contrôle lit le source, parce que la règle est là. Il porte sur
/// les conditions qui parlent de **l'état du programme** — `can_exec`,
/// `can_undo`, `self.running`, `self.emulator.is_some()` : ce sont
/// celles dont la réponse s'apprend. Une entrée grisée parce que la
/// liste des récents est vide se comprend sans phrase.
#[test]
fn a_greyed_command_says_why() {
    let source = include_str!("main.rs");
    let lignes: Vec<&str> = source.lines().collect();
    let conditions = [
        "add_enabled_ui(can_exec",
        "add_enabled_ui(can_undo",
        "add_enabled_ui(self.running",
        "add_enabled_ui(self.emulator.is_some()",
    ];
    let mut muettes = Vec::new();
    for (n, ligne) in lignes.iter().enumerate() {
        if !conditions.iter().any(|c| ligne.contains(c)) {
            continue;
        }
        // CHAQUE bouton du bloc, et non le bloc entier : un premier
        // essai se contentait d'un `on_disabled_hover_text` quelque part
        // dans les trente lignes qui suivent, et il passait au vert avec
        // le défaut remis — les voisins du bouton muet avaient le leur.
        //
        // On repère donc chaque `.clicked()` du bloc et l'on remonte
        // jusqu'au début de sa chaîne, c'est-à-dire jusqu'au `if` qui
        // l'ouvre. L'explication doit être dans cette chaîne-là.
        let fin = (n + 30).min(lignes.len());
        for (m, l) in lignes[n..fin].iter().enumerate() {
            if !l.trim_start().starts_with(".clicked()") && !l.contains(".clicked()") {
                continue;
            }
            let mut début = n + m;
            while début > n && !lignes[début].contains("if ") {
                début -= 1;
            }
            let chaîne = lignes[début..=(n + m)].join("\n");
            if !chaîne.contains("on_disabled_hover_text") {
                muettes.push(format!(
                    "main.rs:{} {} (sous « {} »)",
                    début + 1,
                    lignes[début].trim(),
                    ligne.trim()
                ));
            }
        }
    }
    assert!(
        muettes.is_empty(),
        "{} commande(s) grisée(s) qui ne disent pas pourquoi :\n  {}",
        muettes.len(),
        muettes.join("\n  ")
    );
}

/// namsdec répond à sa ligne de commande.
///
/// Il l'ignorait purement et simplement — alors que son propre lanceur
/// déclare `Exec=namsdec %f` et `MimeType=text/x-asm`, c'est-à-dire
/// qu'il se propose d'ouvrir les fichiers `.asm` du gestionnaire de
/// fichiers. « Ouvrir avec namsdec » ouvrait une fenêtre vide, ce qui
/// est pire que de ne rien proposer. Et `namsdec --version` lançait
/// l'interface graphique : un programme installé dans le PATH doit
/// savoir répondre à la question la plus simple qu'on lui pose.
///
/// Le contrôle porte sur la décision, séparée du reste pour cela : elle
/// ne touche ni au disque ni à l'écran.
#[test]
fn namsdec_answers_its_command_line() {
    use crate::Intention;
    let dit = |args: &[&str]| {
        let v: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        crate::intention(&v)
    };

    assert_eq!(
        dit(&[]),
        Intention::Ouvrir(None),
        "sans argument, on ouvre l'interface"
    );
    assert_eq!(
        dit(&["bonjour.asm"]),
        Intention::Ouvrir(Some(std::path::PathBuf::from("bonjour.asm"))),
        "avec un chemin, on l'ouvre — c'est ce que le lanceur promet"
    );

    for drapeau in ["--version", "-V"] {
        let Intention::Dire(texte) = dit(&[drapeau]) else {
            panic!("{drapeau} doit répondre sans ouvrir de fenêtre")
        };
        assert!(
            texte.contains(crate::APP_VERSION),
            "{drapeau} donne la version : {texte:?}"
        );
    }
    for drapeau in ["--help", "-h"] {
        let Intention::Dire(texte) = dit(&[drapeau]) else {
            panic!("{drapeau} doit répondre sans ouvrir de fenêtre")
        };
        assert!(
            texte.contains("--version") && texte.contains("FICHIER"),
            "{drapeau} dit ce que le programme accepte : {texte:?}"
        );
    }

    // Une faute de frappe n'est pas un nom de fichier : la prendre pour
    // tel donnerait « fichier introuvable : --verison », ce qui n'aide
    // personne.
    let Intention::Dire(texte) = dit(&["--verison"]) else {
        panic!("un argument inconnu ne s'ouvre pas comme un fichier")
    };
    assert!(
        texte.contains("--verison") && texte.contains("--help"),
        "il nomme l'argument fautif et dit où regarder : {texte:?}"
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

/// « Fermer le fichier » ne disparaît pas du menu.
///
/// Elle s'effaçait quand aucun fichier n'était ouvert — c'est-à-dire,
/// pour qui ouvre le menu Fichier en se demandant comment fermer,
/// exactement au mauvais moment. Une entrée qui s'efface laisse croire
/// qu'elle n'existe pas ; une entrée grise qui dit pourquoi enseigne
/// l'état du programme.
#[test]
fn the_close_entry_never_leaves_the_file_menu() {
    // Le menu vit dans `menus.rs` depuis le découpage. Lire `main.rs`
    // trouvait la table des libellés d'action, où « Fermer » figure
    // aussi — et où il n'y a évidemment aucun `add_enabled_ui`. Un
    // contrôle qui cherche le premier trouve presque toujours autre
    // chose.
    let source = include_str!("menus.rs");
    // L'entrée est posée sous un `add_enabled_ui`, et non sous un `if`
    // qui la ferait disparaître.
    let posée = source
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains("s.action_close_tab,"))
        .map(|(n, _)| n)
        .expect("l'entrée « Fermer le fichier » est dans le menu");
    let autour = source.lines().skip(posée.saturating_sub(12)).take(18);
    let autour: Vec<&str> = autour.collect();
    assert!(
        autour.iter().any(|l| l.contains("add_enabled_ui")),
        "elle est grisée, non retirée :\n{}",
        autour.join("\n")
    );
    assert!(
        autour.iter().any(|l| l.contains("why_nothing_to_close")),
        "et elle dit pourquoi elle est grise :\n{}",
        autour.join("\n")
    );
}

/// Toute action a un chemin qui se VOIT.
///
/// Elle a déjà un raccourci dans les deux jeux, et la palette les offre
/// toutes — mais la palette est un filet, pas un chemin : on l'ouvre
/// quand on sait déjà ce qu'on cherche. Un menu, lui, se parcourt.
/// C'est là qu'on découvre ce qu'un programme sait faire.
///
/// HUIT ACTIONS SUR TRENTE n'avaient que le clavier : dupliquer une
/// ligne, surligner les occurrences, aller à la suivante et à la
/// précédente, les renommer toutes, nommer une adresse, la commenter,
/// revenir au fichier précédent. Toutes utiles, toutes invisibles.
///
/// Ce contrôle est le pendant, du côté de l'œil, de « toute action a un
/// raccourci dans les deux jeux ». Il est né d'une série de défauts que
/// le banc ne pouvait pas voir — l'entrée « Fermer » absente du menu, la
/// barre d'onglets invisible, le « + » qui faisait autre chose — et qui
/// étaient tous la même chose : le programme savait faire, et rien ne le
/// disait.
#[test]
fn every_action_has_a_path_you_can_see() {
    let source = include_str!("menus.rs");
    let lignes: Vec<&str> = source.lines().collect();
    // Les bornes des deux fonctions qui portent ce qui se voit en
    // permanence : la barre de menus et la barre d'outils.
    //
    // La visibilité fait partie du nom : sorties dans leur module,
    // elles s'écrivent `pub(crate) fn`, et chercher `fn menu_bar_ui(`
    // en tête de ligne ne trouvait plus rien.
    let entête = |l: &str, nom: &str| {
        let l = l.trim_start();
        l.strip_prefix("pub(crate) ")
            .or_else(|| l.strip_prefix("pub "))
            .unwrap_or(l)
            .starts_with(&format!("fn {nom}("))
    };
    let borne = |nom: &str| -> (usize, usize) {
        let début = lignes
            .iter()
            .position(|l| entête(l, nom))
            .unwrap_or_else(|| panic!("la fonction {nom} existe"));
        let suivante = |l: &&str| {
            let t = l.trim_start();
            (l.starts_with("    ") || !l.starts_with(' '))
                && t.strip_prefix("pub(crate) ")
                    .or_else(|| t.strip_prefix("pub "))
                    .unwrap_or(t)
                    .starts_with("fn ")
        };
        let fin = lignes[début + 1..]
            .iter()
            .position(suivante)
            .map(|d| début + 1 + d)
            .unwrap_or(lignes.len());
        (début, fin)
    };
    let (m0, m1) = borne("menu_bar_ui");
    let (t0, t1) = borne("toolbar_ui");
    let visible = format!(
        "{}\n{}",
        lignes[m0..m1].join("\n"),
        lignes[t0..t1].join("\n")
    );

    let sans: Vec<String> = Action::ALL
        .into_iter()
        .filter(|action| !visible.contains(&format!("Action::{action:?}")))
        .map(|action| format!("{action:?}"))
        .collect();
    assert!(
        sans.is_empty(),
        "{} action(s) qu'on ne peut trouver qu'au clavier — un menu se \
         parcourt, un raccourci se devine :\n  {}",
        sans.len(),
        sans.join("\n  ")
    );
}

/// Une étiquette et sa liste se lisent sur la même ligne.
///
/// Elles ne l'étaient pas : le texte de la liste se posait trois pixels
/// et demi SOUS son étiquette, partout où les deux se suivent. Six
/// façons d'entourer la liste d'egui ont été essayées — imposer la
/// hauteur de la case, celle de la rangée, `add_sized`, une boîte
/// centrée, annuler l'espacement vertical, aligner la rangée par le
/// haut — et l'écart n'a pas bougé d'un pixel. Il est DANS `ComboBox`,
/// dont le cadre part de `available_rect_before_wrap()` et non du
/// centre de la rangée.
///
/// La sortie était d'écrire la nôtre, et c'est un gain au-delà de
/// l'alignement : c'est un bouton encadré comme les autres commandes,
/// au lieu d'un contrôle qui ne ressemblait à rien d'autre dans
/// l'application.
#[test]
fn a_label_and_its_list_read_on_one_line() {
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            lang,
            show_calculator: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
        let où = |quoi: &str| -> f32 {
            lignes
                .iter()
                .find(|(t, _)| t == quoi)
                .map(|(_, r)| r.center().y)
                .unwrap_or_else(|| panic!("{lang:?} : « {quoi} » est peint"))
        };
        for (étiquette, valeur) in [
            (s.calculator_base_label, app.calc_base.label(s).to_owned()),
            (
                s.calculator_width_label,
                format!("{} bits", app.calc_width.bits()),
            ),
        ] {
            let écart = (où(étiquette) - où(&valeur)).abs();
            assert!(
                écart < 1.0,
                "{lang:?} : « {étiquette} » et « {valeur} » doivent se lire sur la \
                 même ligne — {écart:.1} px d'écart"
            );
        }
    }
}

/// Une rangée de commandes se lit sur une seule ligne.
///
/// La barre de saisie de la console en portait quatre à des hauteurs
/// différentes : « Entrée : » à 1028, « Fin d'entrée » et « Envoyer » à
/// 1032,5, le champ à 1032,44. Quatre pixels et demi d'amplitude sur une
/// rangée, ce qui ne se nomme pas en le voyant mais se voit.
///
/// La cause est le deuxième piège d'egui : une rangée horizontale centre
/// chaque élément sur la hauteur qu'elle a À CET INSTANT et ne recale
/// pas ce qui est déjà posé. L'étiquette venant en premier se centrait
/// sur elle-même. Imposer sa propre hauteur n'y change rien — c'est
/// celle de la RANGÉE qu'il faut fixer, et le seul moyen est que le
/// premier élément soit déjà le plus haut.
///
/// Ce contrôle existe pour retenir le nombre mesuré qui le fait : 34
/// pixels, trouvé en essayant 27, 30, 34 et 36. Un nombre mesuré n'est
/// acceptable que si quelque chose le retient.
#[test]
fn a_row_of_commands_reads_on_one_line() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        show_console: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    app.do_step();
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
    let où = |quoi: &str| -> f32 {
        lignes
            .iter()
            .find(|(t, _)| t == quoi)
            .map(|(_, r)| r.center().y)
            .unwrap_or_else(|| panic!("« {quoi} » est peint dans la barre de saisie"))
    };
    let repère = où(s.console_send);
    for quoi in [s.console_input_label, s.console_eof] {
        let écart = (où(quoi) - repère).abs();
        assert!(
            écart < 1.0,
            "« {quoi} » doit se lire sur la même ligne que « {} » — \
             {écart:.1} px d'écart",
            s.console_send
        );
    }
}

/// Ce qui se clique change le pointeur.
///
/// Une étiquette qu'on peut cliquer ne ressemble à rien : c'est du
/// texte. Douze d'entre elles dans namsdec — un nom de registre qu'on
/// surveille, une valeur qu'on remplace, une ligne de pile où l'on va,
/// une zone mémoire, un module d'analyse, une ligne de profil, un
/// diagnostic de nasm — et aucune ne changeait le pointeur.
///
/// Les infobulles disaient déjà ce qu'un clic ferait. Mais il faut
/// s'être arrêté dessus pour les lire, et l'on ne s'arrête que sur ce
/// qu'on soupçonne d'être cliquable : la main, elle, se voit en
/// passant. egui ne la pose pas tout seul — il la réserve aux liens.
///
/// Le contrôle lit le source, parce que la règle est là : tout ce qui
/// prend `Sense::click()` doit annoncer sa main dans les lignes qui
/// suivent, ou passer par `cliquable`, qui le fait pour vous.
#[test]
fn what_can_be_clicked_changes_the_pointer() {
    let mut muets = Vec::new();
    for (nom, source) in [
        ("main.rs", include_str!("main.rs")),
        ("sidebar.rs", include_str!("sidebar.rs")),
    ] {
        let lignes: Vec<&str> = source.lines().collect();
        for (n, ligne) in lignes.iter().enumerate() {
            if !ligne.contains("sense(egui::Sense::click())") {
                continue;
            }
            // La chaîne qui suit : huit lignes suffisent pour un
            // `.on_hover_*` et un `.clicked()`, et l'on ne déborde pas
            // sur le widget d'à côté.
            let suite = lignes[n..(n + 8).min(lignes.len())].join("\n");
            if !suite.contains("on_hover_cursor") {
                muets.push(format!("{nom}:{} {}", n + 1, lignes[n].trim()));
            }
        }
    }
    assert!(
        muets.is_empty(),
        "{} chose(s) qui se cliquent sans le dire — `cliquable` pose la main :\n  {}",
        muets.len(),
        muets.join("\n  ")
    );
}

/// Une poignée dit qu'elle se tire.
///
/// Trois bords se tirent dans namsdec : celui du panneau de droite,
/// celui qui partage ce panneau entre le bandeau et l'onglet, et celui
/// de la console. Rien ne les distingue d'un filet décoratif — ils sont
/// gris et larges d'un pixel comme les autres. Ce qui les annonce est le
/// POINTEUR, qui change de forme en passant dessus ; egui le fait, et
/// c'est le seul signe qu'on reçoive sans s'être arrêté.
///
/// Encore faut-il qu'il arrive jusque-là. Le lien « Registres étendus »
/// touchait le bord du panneau de l'onglet et réclamait la main :
/// mesuré, le pointeur disait `PointingHand` sur cette poignée-là et
/// `ResizeVertical` sur celle de la console, qui n'a rien sous elle. Une
/// poignée qui ment est pire qu'une poignée muette — on croit cliquer
/// sur quelque chose et l'on déplace une cloison.
#[test]
fn a_handle_says_that_it_can_be_pulled() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        show_sidebar: true,
        show_console: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    app.do_step();
    let entrée = écran_du_banc();
    for _ in 0..4 {
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    let haut_de = |id: &str| -> f32 {
        egui::containers::panel::PanelState::load(&ctx, egui::Id::new(id))
            .map(|p| p.outer_rect.top())
            .unwrap_or_else(|| panic!("le panneau « {id} » est posé"))
    };

    /// Ce que le pointeur devient quand on le pose là.
    fn pointeur(
        ctx: &egui::Context,
        app: &mut NamsdecApp,
        entrée: &egui::RawInput,
        place: egui::Pos2,
    ) -> egui::CursorIcon {
        let survol = egui::RawInput {
            events: vec![egui::Event::PointerMoved(place)],
            ..entrée.clone()
        };
        let mut sortie = ctx.run_ui(survol, |ui| app.draw(ui));
        sortie.textures_delta.clear();
        // La seconde image : la première pose le pointeur, la seconde
        // dit ce que les widgets en ont fait.
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        let curseur = sortie.platform_output.cursor_icon;
        sortie.textures_delta.clear();
        curseur
    }

    for (nom, place) in [
        (
            "le partage du panneau",
            egui::pos2(1400.0, haut_de("sidebar_tab")),
        ),
        (
            "le haut de la console",
            egui::pos2(600.0, haut_de("console")),
        ),
    ] {
        assert_eq!(
            pointeur(&ctx, &mut app, &entrée, place),
            egui::CursorIcon::ResizeVertical,
            "{nom} se tire, et le pointeur doit le dire"
        );
    }

    // Et ailleurs, il ne dit rien : une poignée partout n'est pas une
    // poignée.
    assert_eq!(
        pointeur(&ctx, &mut app, &entrée, egui::pos2(400.0, 300.0)),
        egui::CursorIcon::Default,
        "au milieu du code, le pointeur reste ce qu'il est"
    );
}

/// Le bandeau ne laisse pas un trou entre ses deux zones.
///
/// L'état du processeur était ancré au bas de sa région : tout le blanc
/// que le bandeau ne prenait pas tombait ENTRE le contenu de « Prochaine
/// instruction » et lui. Quatre cent cinquante-six pixels sur l'écran
/// d'accueil, où le bandeau n'a qu'une phrase à dire — la première zone
/// paraissait cassée, et non vide.
///
/// Trois tentatives de rééquilibrage avaient échoué avant. Aucune
/// n'avait vu la vraie cause, qui n'était pas dans le partage : le
/// panneau croyait que la POIGNÉE AVAIT ÉTÉ TIRÉE dès que la première
/// image ne rendait pas la hauteur demandée — ce qui arrive toujours,
/// puisque cette image-là ne connaît encore ni la hauteur du bandeau ni
/// celle de l'état du processeur. La part de l'onglet était donc gelée
/// à son plancher pour toute la séance, et le bandeau gardait tout le
/// reste. Une poignée ne bouge que sous un doigt : c'est la seule
/// question à poser, et elle n'était pas posée.
///
/// Désancrer l'état du processeur a aussi été essayé, et mesuré : vingt-
/// six pixels de trou au lieu de trente-huit. Douze pixels ne valent pas
/// de reprendre le risque qui avait fait échouer la deuxième tentative —
/// posé à la suite, il débordait sur la vue du dessous dès que le compte
/// ne tombait pas juste. Il reste ancré.
#[test]
fn the_banner_leaves_no_hole_between_its_two_zones() {
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_sidebar: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 5);
    // La dernière ligne de « Prochaine instruction », et le titre qui
    // suit : entre les deux, il ne doit y avoir que de l'air normal.
    let titre = lignes
        .iter()
        .filter(|(t, r)| t == s.cpu_state && r.left() > 1200.0)
        .map(|(_, r)| r.top())
        .fold(f32::INFINITY, f32::min);
    let bas_du_bandeau = lignes
        .iter()
        .filter(|(_, r)| r.left() > 1200.0 && r.bottom() < titre)
        .map(|(_, r)| r.bottom())
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        titre.is_finite() && bas_du_bandeau.is_finite(),
        "les deux zones du bandeau sont peintes"
    );
    let trou = titre - bas_du_bandeau;
    assert!(
        trou < 80.0,
        "« {} » doit suivre ce qui le précède, non flotter tout en bas : \
         {trou:.0} px de blanc",
        s.cpu_state
    );
}

/// Les octets d'une instruction se récrivent là où on les lit.
///
/// La vue mémoire se modifiait déjà ; l'onglet Octets, non — alors que
/// c'est là qu'on lit ce qu'on voudrait changer. On voyait le `je` qu'il
/// faudrait inverser et il fallait passer par une autre fenêtre.
///
/// Le même chemin que les trois gestes du patcheur : la fenêtre
/// Correctifs les liste et sait les défaire. Écrire à la main n'est pas
/// un autre mécanisme, c'est la même chose dite autrement.
///
/// LA LONGUEUR NE CHANGE PAS, et c'est refusé plutôt que rattrapé :
/// récrire une instruction plus courte ou plus longue déplacerait tout
/// ce qui suit, donc toutes les adresses, donc tous les sauts.
#[test]
fn the_bytes_of_an_instruction_can_be_rewritten_where_they_are_read() {
    // Ce que la saisie accepte, et ce qu'elle refuse.
    assert_eq!(crate::lire_octets("48 31 c0"), Some(vec![0x48, 0x31, 0xc0]));
    assert_eq!(
        crate::lire_octets("4831c0"),
        Some(vec![0x48, 0x31, 0xc0]),
        "on recopie ce qu'on lit ailleurs, et ailleurs n'a pas toujours d'espaces"
    );
    assert_eq!(crate::lire_octets("48,31,c0"), Some(vec![0x48, 0x31, 0xc0]));
    assert_eq!(crate::lire_octets(""), None, "rien n'est pas zéro octet");
    assert_eq!(
        crate::lire_octets("48 3"),
        None,
        "un nombre impair de chiffres ne dit pas ce qu'on voulait"
    );
    assert_eq!(
        crate::lire_octets("48 zz"),
        None,
        "et « zz » n'est pas un octet"
    );

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    let Some(state) = &app.assemble_state else {
        panic!("le programme du banc s'assemble")
    };
    let instruction = state.disasm[1].clone();
    let adresse = instruction.address;
    let avant = instruction.bytes.clone();

    // Une longueur différente est refusée, et le dit.
    app.write_bytes_at(adresse, "90");
    assert!(
        matches!(&app.status, Status::Error(_)),
        "récrire avec un autre nombre d'octets est refusé"
    );
    let après_refus = app.assemble_state.as_ref().unwrap().disasm[1].bytes.clone();
    assert_eq!(après_refus, avant, "et rien n'a bougé");

    // La même longueur passe, et le changement se voit là où on l'a
    // écrit — puis dans la fenêtre Correctifs, qui sait le défaire.
    let offset = namsdec_re::imports::file_offset(
        &app.assemble_state.as_ref().unwrap().output.file,
        adresse,
    )
    .expect("l'adresse a une place dans le fichier") as usize;
    let nops: String = vec!["90"; avant.len()].join(" ");
    app.write_bytes_at(adresse, &nops);
    // Le contrôle porte sur le FICHIER et non sur le désassemblage :
    // cinq octets remplacés par cinq `nop` ne font plus une instruction
    // de cinq octets mais cinq d'un octet, ce qui est exactement ce
    // qu'on voulait et pas ce qu'un test naïf attend.
    let fichier = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(
        &fichier[offset..offset + avant.len()],
        vec![0x90; avant.len()].as_slice(),
        "les octets écrits sont ceux qui sont dans le fichier"
    );
    assert_eq!(
        app.patches.len(),
        1,
        "et le changement est dans la liste de ce qu'on peut défaire"
    );

    // Défait, on retrouve exactement ce qu'il y avait.
    app.undo_patch(0);
    let fichier = &app.assemble_state.as_ref().unwrap().output.file;
    assert_eq!(
        &fichier[offset..offset + avant.len()],
        avant.as_slice(),
        "défaire rend les octets d'origine"
    );
    assert!(app.patches.is_empty());
}

/// Le graphe dit ce que chaque bloc fait, quand il le sait.
///
/// namsdec expliquait chaque instruction et jamais un ENSEMBLE : ce que
/// fait une boucle, ce qu'une suite de `cmp`/`je` vérifie au total.
///
/// La règle du module est qu'une explication fausse est pire qu'une
/// absence d'explication. Ce contrôle vérifie donc les deux moitiés :
/// que le graphe DIT quelque chose là où c'est certain, et qu'il se TAIT
/// sur ce qu'il ne sait pas.
#[test]
fn the_flow_says_what_a_block_does_when_it_knows() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    for _ in 0..40 {
        app.do_step();
    }
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 5);
    let écrit: Vec<&str> = lignes.iter().map(|(t, _)| t.as_str()).collect();

    // Le programme du banc écrit son message : le bloc qui le fait doit
    // le dire, parce que son numéro d'appel est posé juste avant.
    assert!(
        écrit.iter().any(|t| t.contains("demande au noyau : write")),
        "le graphe nomme l'appel système dont le numéro est dans le bloc — {écrit:?}"
    );
    // Et sa boucle est une boucle, qui a tourné.
    assert!(
        écrit.iter().any(|t| t.starts_with("boucle")),
        "le graphe reconnaît la boucle — {écrit:?}"
    );

    // Ce qu'il ne sait pas, il ne le dit pas : aucune phrase ne peut
    // apparaître pour un bloc dont les instructions ne se résument pas.
    // On le vérifie sur le module, qui est la seule porte d'entrée.
    assert_eq!(
        namsdec_re::explain::phrase(
            &namsdec_re::explain::Bloc {
                instructions: &["add rax,rcx".to_owned()],
                exit: namsdec_re::cfg::Exit::Fallthrough,
                boucle: false,
                passages: Some(3),
            },
            true
        ),
        None,
        "un bloc dont on ne sait rien dire n'a pas de phrase, même s'il a tourné"
    );
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
    let peint = paint(&mut app);
    assert!(
        peint.contains("a tourné 4 fois"),
        "et cela s'écrit sous le but :\n{peint}"
    );
}

/// Le pseudo-code se voit dans le déroulement, et remplace le graphe.
///
/// Deux lectures de la même fonction : sa forme, ou son texte. L'une ne
/// remplace pas l'autre chez personne, d'où une case et non un choix
/// imposé — mais quand la case est cochée, c'est bien le texte qu'on
/// voit, et non un graphe avec du texte à côté.
#[test]
fn the_flow_window_can_show_the_pseudo_code() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Une boucle : `mov ecx, 3` puis un `dec`/`jnz` qui remonte. C'est
    // la forme la plus courante, et celle qui doit se dire « répéter ».
    let source = "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                  boucle:\n    dec ecx\n    jnz boucle\n    hlt\n";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        show_flow: true,
        flow_pseudo: true,
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
    let peint: Vec<&String> = lignes.iter().map(|(t, _)| t).collect();
    assert!(
        peint.iter().any(|t| t.contains("répéter")),
        "la boucle se dit « répéter » : {peint:?}"
    );
    assert!(
        peint.iter().any(|t| t.contains("tant que")),
        "et sa condition se lit : {peint:?}"
    );
    // Le corps de la boucle, dit en clair et non en assembleur.
    assert!(
        peint.iter().any(|t| t.contains("ECX = ECX")),
        "le corps est traduit : {peint:?}"
    );
}

/// Et la case décochée rend le graphe.
///
/// Sans quoi le pseudo-code ne serait pas une vue de plus mais un
/// remplacement — et le graphe, lui, montre ce qu'aucun texte ne montre :
/// la forme d'ensemble d'une fonction qu'on découvre.
#[test]
fn unchecking_the_box_gives_the_graph_back() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let source = "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                  boucle:\n    dec ecx\n    jnz boucle\n    hlt\n";
    let mesure = |pseudo: bool| -> Vec<String> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            show_flow: true,
            flow_pseudo: pseudo,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
        lignes.into_iter().map(|(t, _)| t).collect()
    };
    let graphe = mesure(false);
    assert!(
        graphe.iter().any(|t| t.contains("dec ecx")),
        "le graphe montre les instructions telles quelles : {graphe:?}"
    );
    assert!(
        !graphe.iter().any(|t| t.contains("répéter")),
        "et aucun mot de structure : {graphe:?}"
    );
}

/// Le choix de la vue se retrouve à la séance suivante.
///
/// C'est la règle du dépôt : un réglage qui se voit dans l'interface
/// part dans `settings.json` à l'instant où on le règle. Celui-ci tient
/// à la personne — on lit un graphe pour repérer, un pseudo-code pour
/// comprendre — et non au binaire qu'on ouvre.
#[test]
fn the_chosen_flow_view_survives_the_session() {
    let app = NamsdecApp {
        flow_pseudo: true,
        ..Default::default()
    };
    let réglages = app.settings_snapshot();
    assert!(réglages.flow_pseudo, "le choix part dans les réglages");
    let relu = NamsdecApp::from_settings(réglages);
    assert!(relu.flow_pseudo, "et il en revient");
}

/// Une commande qui n'agirait sur rien est grisée, et dit pourquoi.
///
/// Le pseudo-code est du TEXTE. Le zoom, lui, agit sur un dessin : les
/// cinq commandes d'échelle de la fenêtre du déroulement — moins, plus,
/// « Taille réelle », « Tout voir », et le pourcentage — n'y feraient
/// rien du tout. Elles restaient pourtant actives, à l'identique, dans
/// les deux vues : le relevé de la barre donnait exactement la même
/// liste de part et d'autre de la case.
///
/// Une commande qui ne fait rien ment, et c'est le genre de mensonge
/// qu'on ne signale pas — on croit avoir mal cliqué. Grisées plutôt
/// qu'effacées : une commande qui disparaît laisse croire qu'elle
/// n'existe pas, une commande grise qui dit pourquoi enseigne l'état du
/// programme. C'est la règle déjà posée pour « Fermer le fichier ».
///
/// Le contrôle lit le source, parce que c'est là qu'est la règle : les
/// commandes d'échelle vivent sous un `add_enabled_ui` qui dépend de la
/// vue, et chacune porte sa propre explication — une seule pour tout le
/// bloc passerait au vert avec le défaut remis.
#[test]
fn the_scale_commands_are_greyed_when_there_is_nothing_to_scale() {
    let source = include_str!("flow.rs");
    let lignes: Vec<&str> = source.lines().collect();
    let garde = lignes
        .iter()
        .position(|l| l.contains("ui.add_enabled_ui(dessin,"))
        .expect("les commandes d'échelle sont sous une garde qui suit la vue");
    // Les cinq commandes sont dedans, et chacune dit pourquoi elle est
    // grise. La fenêtre de vingt-cinq lignes suffit : au-delà, c'est
    // qu'elles n'y sont plus.
    let bloc: Vec<&str> = lignes.iter().skip(garde).take(45).copied().collect();
    for commande in [
        "\"−\"",
        "\"+\"",
        "s.flow_zoom_reset",
        "s.flow_fit",
        "self.flow_zoom * 100.0",
    ] {
        assert!(
            bloc.iter().any(|l| l.contains(commande)),
            "{commande} doit être sous la garde :\n{}",
            bloc.join("\n")
        );
    }
    let explications = bloc
        .iter()
        .filter(|l| l.contains("on_disabled_hover_text"))
        .count();
    assert_eq!(
        explications,
        5,
        "chacune des cinq dit pourquoi elle est grise, et non une pour \
         tout le bloc :\n{}",
        bloc.join("\n")
    );
}

/// Et ce qu'on regarde peut s'emporter, quelle que soit la vue.
///
/// Les deux boutons d'export nomment le graphe dans leur libellé : ils
/// ne mentent pas, mais ils ne rendent pas ce qu'on a sous les yeux
/// quand c'est le texte qu'on lit. Le troisième bouton n'apparaît donc
/// que là où il a un sens.
#[test]
fn the_pseudo_code_can_be_copied_when_it_is_what_we_read() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let source = "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                  boucle:\n    dec ecx\n    jnz boucle\n    hlt\n";
    let barre = |pseudo: bool| -> Vec<String> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            show_flow: true,
            flow_pseudo: pseudo,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
        lignes.into_iter().map(|(t, _)| t).collect()
    };
    let s = strings(Lang::Fr);
    let texte = barre(true);
    assert!(
        texte.iter().any(|t| t == s.flow_copy_pseudo),
        "le bouton manque là où il sert : {texte:?}"
    );
    let graphe = barre(false);
    assert!(
        !graphe.iter().any(|t| t == s.flow_copy_pseudo),
        "et il ne traîne pas là où il n'a rien à copier : {graphe:?}"
    );
    // L'aide au glisser suit la même règle : elle décrit un geste qui
    // n'existe pas sur du texte.
    assert!(
        graphe.iter().any(|t| t == s.flow_pan_hint),
        "l'aide au glisser est là pour le dessin : {graphe:?}"
    );
    assert!(
        !texte.iter().any(|t| t == s.flow_pan_hint),
        "et pas pour le texte : {texte:?}"
    );
}

/// Un texte sans structure le dit, au lieu de laisser chercher.
///
/// Une fonction sans saut rend un pseudo-code qui n'a rien d'un
/// pseudo-code : les instructions, dites en clair, et c'est tout. Sans
/// un mot, on cherche le « si » qui n'y est pas et l'on croit la vue en
/// panne.
///
/// L'état vide écrit d'abord pour ce cas ne pouvait pas s'afficher : il
/// demandait un graphe vide, que la fenêtre écarte trente lignes plus
/// haut. Un état vide qu'on n'atteint jamais est un message qu'on
/// croit avoir écrit.
#[test]
fn a_pseudo_code_without_structure_says_so() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let peint = |source: &str| -> Vec<String> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            show_flow: true,
            flow_pseudo: true,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
        lignes.into_iter().map(|(t, _)| t).collect()
    };
    // Sans saut : la mention est là.
    let plat = peint("section .text\nglobal _start\n_start:\n    nop\n    nop\n    hlt\n");
    assert!(
        plat.iter().any(|t| t.contains("aucun saut")),
        "le texte sans structure le dit : {plat:?}"
    );
    // Avec une boucle : elle n'a plus lieu d'être.
    let structuré = peint(
        "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
         boucle:\n    dec ecx\n    jnz boucle\n    hlt\n",
    );
    assert!(
        structuré.iter().any(|t| t.contains("répéter")),
        "la boucle est reconnue : {structuré:?}"
    );
    assert!(
        !structuré.iter().any(|t| *t == s.flow_pseudo_flat),
        "et la mention ne traîne pas : {structuré:?}"
    );
}

/// L'indentation du pseudo-code se VOIT, et les lignes ne se marchent
/// pas dessus.
///
/// Un texte structuré dont les crans ne se distinguent pas ne vaut pas
/// mieux qu'un listing : c'est l'indentation qui dit ce qui dépend de
/// quoi, et c'est elle qu'on lit avant les mots. Elle est posée en
/// mesurant quatre espaces de la police à chasse fixe — deviner une
/// largeur de caractère marche jusqu'au premier changement de police ou
/// de zoom.
#[test]
fn the_pseudo_code_indentation_can_be_seen() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Trois niveaux : la boucle, le test dedans, et son corps.
    let source = "section .text\nglobal _start\n_start:\n    mov rcx, 5\n\
                  boucle:\n    cmp rcx, 2\n    je suivant\n    add rax, rcx\n\
                  suivant:\n    dec rcx\n    jnz boucle\n    hlt\n";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        show_flow: true,
        flow_pseudo: true,
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);

    let où = |quoi: &str| -> egui::Rect {
        lignes
            .iter()
            .find(|(t, _)| t.contains(quoi))
            .map(|(_, r)| *r)
            .unwrap_or_else(|| {
                panic!(
                    "« {quoi} » est peint : {:?}",
                    lignes.iter().map(|(t, _)| t).collect::<Vec<_>>()
                )
            })
    };
    let boucle = où("répéter");
    let test = où("si ");
    let corps = où("RAX = RAX +");
    // Chaque niveau rentre par rapport au précédent, et d'un cran qui
    // se voit : sous dix pixels, on ne le distingue pas.
    assert!(
        test.left() > boucle.left() + 10.0,
        "le test est en retrait de la boucle : {} vs {}",
        test.left(),
        boucle.left()
    );
    assert!(
        corps.left() > test.left() + 10.0,
        "et son corps l'est encore : {} vs {}",
        corps.left(),
        test.left()
    );
    // Les crans sont RÉGULIERS : un retrait qui varie se lit comme une
    // erreur de mise en page, non comme une structure.
    let premier = test.left() - boucle.left();
    let second = corps.left() - test.left();
    assert!(
        (premier - second).abs() < 1.0,
        "les deux crans doivent être égaux : {premier} puis {second}"
    );
}

/// Le déroulement dit ce qu'il sait de la FONCTION, pas seulement de
/// ses blocs.
///
/// Chaque boîte porte déjà une phrase quand namsdec sait la dire. Il
/// manquait le cran au-dessus : devant une fonction sans symboles, la
/// première question n'est pas « que fait ce bloc » mais « que prend-elle
/// et que rend-elle ». La convention System V répond pour les
/// arguments, et les retours répondent pour la valeur — à condition que
/// TOUS les chemins s'accordent.
///
/// La phrase ne paraît que sur une FONCTION : « lit RDI » n'a aucun sens
/// devant un programme entier, qui en contient plusieurs.
#[test]
fn the_flow_window_says_what_the_whole_function_does() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Une fonction qui compare ses deux premiers arguments et rend 0
    // ou 1 : le squelette de toute vérification de mot de passe.
    // Elle finit par `ret` et non par `hlt` : une fonction rend la
    // main, elle n'arrête pas la machine — et c'est le `ret` qui dit
    // qu'une valeur est rendue.
    let source = "section .text\nglobal _start\n_start:\n\
                  \x20   cmp rdi, rsi\n    jne different\n\
                  \x20   mov eax, 1\n    ret\ndifferent:\n    xor eax, eax\n    ret\n";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
    let peint: Vec<&String> = lignes.iter().map(|(t, _)| t).collect();
    assert!(
        peint.iter().any(|t| t.contains("RDI et RSI")),
        "les arguments lus se disent : {peint:?}"
    );
    assert!(
        peint.iter().any(|t| t.contains("0 ou 1")),
        "et ce que la fonction rend : {peint:?}"
    );
}

/// Et il se tait quand il n'aurait que le banal à dire.
///
/// « Contient une boucle » devant un dessin où la boucle se voit
/// n'apprend rien, et une phrase de trop fait cesser de lire celles qui
/// apprennent quelque chose.
#[test]
fn the_flow_window_keeps_quiet_when_the_drawing_already_says_it() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let source = "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                  boucle:\n    dec ecx\n    jnz boucle\n    hlt\n";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
    let peint: Vec<&String> = lignes.iter().map(|(t, _)| t).collect();
    assert!(
        !peint.iter().any(|t| t.contains("contient une boucle")),
        "le dessin le montre déjà : {peint:?}"
    );
}

/// Ce qui change le pointeur se clique VRAIMENT.
///
/// `cliquable` fait deux choses : elle rend une étiquette sensible au
/// clic, et elle met le curseur en main au survol. La seconde est une
/// promesse — dans toute interface, une main annonce que quelque chose
/// va se produire. Une étiquette qui prend la main sans rien faire au
/// clic enseigne à ne plus croire le pointeur, et c'est alors tous les
/// autres liens de l'application qui cessent d'être lus.
///
/// Le défaut a été commis ici même : le portrait de fonction, en tête
/// du déroulement, a été posé avec `cliquable` par simple habitude de
/// frappe. Rien ne l'a signalé — il s'affichait bien, l'infobulle
/// marchait, et seul le curseur mentait.
///
/// Le contrôle lit le source parce que la règle y est : une réponse de
/// `cliquable` dont on ne lit jamais `clicked()` est une main promise
/// pour rien.
#[test]
fn what_turns_the_cursor_into_a_hand_really_answers_a_click() {
    let mut muettes = Vec::new();
    for (fichier, source) in SOURCES_DE_L_INTERFACE {
        let lignes: Vec<&str> = source.lines().collect();
        for (n, ligne) in lignes.iter().enumerate() {
            // La définition elle-même, et les mentions en commentaire,
            // ne sont pas des appels.
            if !ligne.contains("cliquable(")
                || ligne.trim_start().starts_with("//")
                || ligne.contains("fn cliquable")
            {
                continue;
            }
            // Par la VARIABLE qui garde la réponse, et non par un
            // nombre de lignes.
            //
            // Le premier jet cherchait `clicked()` dans les vingt lignes
            // suivantes, et accusait à tort le bandeau des registres :
            // son infobulle fait soixante lignes — le rôle du registre,
            // sa valeur dans toutes les bases, sa provenance — avant que
            // le clic ne soit lu. Un contrôle trop étroit accuse ; un
            // contrôle trop large absout. Le nom de la variable ne se
            // trompe ni dans un sens ni dans l'autre.
            let gardée = ligne
                .split_once("let ")
                .and_then(|(_, reste)| reste.split_once(" ="))
                .map(|(nom, _)| nom.trim().trim_start_matches("mut ").to_owned());
            let répond = match &gardée {
                // Une ligne qui parle de la variable ET lit un clic.
                // Chercher « nom.clicked() » tel quel ratait
                // `réponse.on_hover_text(…).clicked()`, qui est la forme
                // la plus courante : l'infobulle s'intercale entre la
                // réponse et la lecture du clic.
                Some(nom) => lignes
                    .iter()
                    .skip(n)
                    .any(|l| l.contains(nom.as_str()) && l.contains(".clicked()")),
                // Sans variable, la chaîne se lit sur place : quelques
                // lignes suffisent, le temps d'une infobulle courte.
                None => lignes
                    .iter()
                    .skip(n)
                    .take(8)
                    .any(|l| l.contains(".clicked()")),
            };
            if !répond {
                muettes.push(format!("{fichier}:{} {}", n + 1, ligne.trim()));
            }
        }
    }
    assert!(
        muettes.is_empty(),
        "{} main(s) promise(s) pour rien — le curseur annonce un clic \
         qui ne fait rien :\n  {}",
        muettes.len(),
        muettes.join("\n  ")
    );
}

/// Aucun contrôle ne demande à la machine ce qu'il doit vérifier.
///
/// Ce défaut a coûté quatre CI rouges d'affilée, et il était passé quatre
/// fois sous le nez d'une relecture, parce qu'il a l'air d'une précaution :
/// « le test s'adapte à la machine plutôt que de supposer ». Mais un
/// contrôle qui lit l'état de la machine pour choisir son assertion dit
/// deux choses différentes selon le poste — et le pire n'est pas qu'il
/// échoue ailleurs. Le pire est qu'il **passe sans rien vérifier** là où
/// tout est installé : `a_working_tool_is_not_announced_as_missing_something`
/// n'éprouvait jamais le cas « incomplet », le seul qui l'intéressait.
///
/// La règle est donc : l'état se **sème**, il ne se lit pas. Un test qui
/// veut les deux cas les sème tous les deux.
///
/// Deux formes restent légitimes, et sont nommées ici une par une :
/// vérifier une **relation** qui tient sur toute machine (si le programme
/// est introuvable, la fiche doit dire « absent »), et vérifier la
/// **mécanique du relevé** lui-même (qu'il soit demandé une fois et
/// gardé). Un `return` en tête parce que nasm manque n'est pas concerné :
/// se taire n'est pas s'adapter.
#[test]
fn no_control_asks_the_machine_what_it_should_be_checking() {
    let source = include_str!("tests.rs");

    // Les contrôles qui ont le droit de lire l'état réel, et pourquoi.
    // Allonger cette liste demande une raison écrite, et c'est le but :
    // le prochain qui voudra « s'adapter à la machine » devra dire ici en
    // quoi son cas ressemble à ces deux-là.
    const AUTORISÉS: [(&str, &str); 2] = [
        (
            "the_tools_say_whether_this_machine_has_them",
            "vérifie une RELATION vraie partout : programme introuvable ⇒ fiche « absent »",
        ),
        (
            "what_the_tools_answer_is_asked_once_and_kept",
            "vérifie la mécanique du relevé, non son contenu",
        ),
    ];

    // Ce qui dénonce une lecture d'état : les trois façons de la demander.
    const LECTURES: [&str; 3] = ["availability_in(", ".availability()", "tool_availability("];

    // Et ce contrôle-ci, qui porte forcément les trois motifs puisqu'il
    // les cherche. Un contrôle qui lit sa propre source se dénonce
    // toujours lui-même ; le dire ici vaut mieux que de tordre les
    // motifs pour qu'ils ne se ressemblent plus.
    const MOI: &str = "no_control_asks_the_machine_what_it_should_be_checking";

    let mut fautifs: Vec<String> = Vec::new();
    // Découpé sur `#[test]` : chaque morceau porte une fonction de test et
    // son corps. Le premier morceau est l'avant-propos du fichier.
    for morceau in source.split("#[test]\n").skip(1) {
        let Some(nom) = morceau
            .split_once("fn ")
            .and_then(|(_, reste)| reste.split_once('('))
            .map(|(nom, _)| nom.trim())
        else {
            continue;
        };
        if nom == MOI || AUTORISÉS.iter().any(|(permis, _)| *permis == nom) {
            continue;
        }
        // Le corps seul : la documentation d'un test peut très bien
        // parler de `availability_in` sans l'appeler.
        let corps = morceau
            .split_once('{')
            .map(|(_, reste)| reste)
            .unwrap_or(morceau);
        for lecture in LECTURES {
            if corps.contains(lecture) {
                fautifs.push(format!("{nom} appelle {lecture}"));
            }
        }
    }

    assert!(
        fautifs.is_empty(),
        "{} contrôle(s) qui lisent la machine au lieu de semer leur état :\n  {}\n\n\
         Semez l'état (`app.tools_ready = …`), et semez-le APRÈS la première image : \
         ouvrir les préférences ou la fenêtre d'installation jette le relevé par \
         conception, et l'image suivante le refait en lançant vraiment l'outil.",
        fautifs.len(),
        fautifs.join("\n  ")
    );

    // Et la liste blanche ne pourrit pas : un nom qu'on y laisse après
    // avoir renommé ou supprimé le test fait croire que la règle a une
    // exception qu'elle n'a plus.
    for (nom, raison) in AUTORISÉS {
        assert!(
            source.contains(&format!("fn {nom}(")),
            "{nom} n'existe plus — retirez-le de la liste (« {raison} »)"
        );
    }
}

/// Le guide que le README annonce existe, et c'est celui de la version.
///
/// Le lien pointait vers `guide-namsdec-0.2.5-beta.3.pdf`, un fichier
/// que trois montées de version avaient emporté : un lien mort en tête
/// du seul endroit où l'on va chercher le mode d'emploi. Personne ne le
/// remarque en écrivant `sed -i` sur un numéro de version — d'où ce
/// contrôle, qui le remarque.
#[test]
fn the_guide_the_readme_points_to_is_the_one_of_this_version() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = match std::fs::read_to_string(racine.join("README.md")) {
        Ok(texte) => texte,
        Err(_) => return,
    };
    let attendu = format!("docs/guide-namsdec-{}.pdf", env!("CARGO_PKG_VERSION"));
    assert!(
        readme.contains(&attendu),
        "le README doit annoncer « {attendu} » — le guide de CETTE version"
    );
    // Et le fichier est là : un lien vers un PDF absent ne vaut pas mieux
    // qu'un lien vers le mauvais.
    assert!(
        racine.join(&attendu).is_file(),
        "{attendu} doit exister — régénérez-le avec `python3 docs/guide.py`"
    );
    // Aucun autre guide ne traîne : deux PDF dans le dossier, et l'on ne
    // sait plus lequel décrit ce qu'on a sous les yeux.
    let autres: Vec<String> = std::fs::read_dir(racine.join("docs"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entrée| entrée.file_name().to_string_lossy().into_owned())
        .filter(|nom| nom.starts_with("guide-namsdec-") && nom.ends_with(".pdf"))
        .filter(|nom| !attendu.ends_with(nom.as_str()))
        .collect();
    assert!(
        autres.is_empty(),
        "un seul guide à la fois : {autres:?} décrivent une autre version"
    );
}

/// Les deux scripts disent ce qu'ils vont faire AVANT de le faire.
///
/// L'aide existait, mais il fallait la demander : on lançait
/// `./install.sh` et l'on découvrait après coup qu'il avait touché au
/// fichier du shell et épinglé au dock. Un relevé des réglages retenus en
/// tête, avec l'option qui annule chacun, laisse le temps de couper
/// court — et évite d'avoir à relire `--help` pour savoir ce qui se
/// passe.
///
/// Le contrôle porte sur le comportement, non sur la mise en forme : il
/// lance un essai à blanc et exige que chaque réglage nommé s'y lise,
/// avec l'option qui le retire.
#[test]
fn the_scripts_say_what_they_will_do_before_doing_it() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !racine.join("install.sh").is_file() {
        return;
    }
    let dossier = tempfile::tempdir().expect("un dossier temporaire");

    for (nom, attendus) in [
        (
            "install.sh",
            vec![
                "Ce qui va être fait",
                "-h pour toutes les options",
                "--no-exercises",
                "--no-desktop",
                "--no-path",
                "--no-dock",
            ],
        ),
        (
            "uninstall.sh",
            vec!["Ce qui va partir", "-h pour toutes les options", "--purge"],
        ),
    ] {
        let sortie = std::process::Command::new("bash")
            .arg(racine.join(nom))
            .args(["--dry-run", "--no-color", "--prefix"])
            .arg(dossier.path().join("share"))
            .arg("--bin-dir")
            .arg(dossier.path().join("bin"))
            .output();
        let Ok(sortie) = sortie else {
            eprintln!("bash est introuvable : test ignoré");
            return;
        };
        assert!(
            sortie.status.success(),
            "{nom} --dry-run doit aboutir :\n{}",
            String::from_utf8_lossy(&sortie.stderr)
        );
        let dit = String::from_utf8_lossy(&sortie.stdout);
        for attendu in attendus {
            assert!(
                dit.contains(attendu),
                "{nom} doit annoncer « {attendu} » :\n{dit}"
            );
        }
        // Et le cadre ne doit pas être de travers : ses trois lignes font
        // la même largeur. Un titre accentué la décalait, parce que
        // `printf '%-*s'` compte les octets et non les caractères.
        let bords: Vec<&str> = dit
            .lines()
            .filter(|ligne| ligne.contains('┌') || ligne.contains('└') || ligne.contains('│'))
            .collect();
        assert!(bords.len() >= 3, "{nom} doit porter un cadre :\n{dit}");
        let largeurs: std::collections::BTreeSet<usize> =
            bords.iter().map(|ligne| ligne.chars().count()).collect();
        assert_eq!(
            largeurs.len(),
            1,
            "{nom} : les lignes du cadre doivent faire la même largeur, vu {largeurs:?}\n{dit}"
        );
    }
}

/// Les scripts d'installation disent toutes les options qu'ils
/// acceptent.
///
/// Une option qui marche mais que `--help` ne nomme pas n'existe pas :
/// personne ne la devinera. Le contrôle lit le `case` qui traite les
/// arguments et vérifie que chaque motif se retrouve dans l'aide.
#[test]
fn the_install_scripts_describe_every_option_they_accept() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for nom in ["install.sh", "uninstall.sh"] {
        let source = std::fs::read_to_string(racine.join(nom)).expect(nom);
        // Le bloc d'aide : entre `cat <<AIDE` et son terminateur.
        let aide = source
            .split_once("cat <<AIDE")
            .and_then(|(_, reste)| reste.split_once("\nAIDE\n"))
            .map(|(bloc, _)| bloc.to_owned())
            .unwrap_or_else(|| panic!("{nom} doit porter un bloc d'aide"));

        // Les motifs du `case` : `--purge)`, `-n|--dry-run)`, …
        let mut options: Vec<String> = Vec::new();
        for ligne in source.lines() {
            let taillée = ligne.trim();
            let Some((motifs, _)) = taillée.split_once(')') else {
                continue;
            };
            if !motifs.starts_with('-') {
                continue;
            }
            for motif in motifs.split('|') {
                // `--prefix=*` et `--prefix` sont la même option.
                let motif = motif.trim().trim_end_matches("=*");
                if motif.starts_with('-') {
                    options.push(motif.to_owned());
                }
            }
        }
        assert!(
            options.len() >= 5,
            "{nom} : le relevé ne trouve presque aucune option ({}) — il \
             regarde au mauvais endroit",
            options.len()
        );
        let muettes: Vec<&String> = options.iter().filter(|o| !aide.contains(*o)).collect();
        assert!(
            muettes.is_empty(),
            "{nom} accepte {} option(s) que son aide ne nomme pas — \
             une option qu'on ne peut pas deviner n'existe pas : {muettes:?}",
            muettes.len()
        );
    }
}

/// Un essai à blanc n'écrit RIEN.
///
/// `--dry-run` est une promesse plus forte qu'une commodité : on
/// l'emploie précisément quand on n'ose pas lancer la vraie commande.
/// Un seul geste qui y échappe la rend pire qu'absente.
///
/// Le défaut a été commis en écrivant ces options : le dock et la ligne
/// de PATH étaient modifiés pour de bon en essai à blanc, parce qu'ils
/// ne passaient pas par la fonction qui sait se taire.
///
/// Le contrôle lit le source : toute commande qui écrit doit passer par
/// `faire`/`retirer`, ou vivre sous une garde qui teste `$ESSAI`.
#[test]
fn a_dry_run_writes_nothing() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for nom in ["install.sh", "uninstall.sh"] {
        let source = std::fs::read_to_string(racine.join(nom)).expect(nom);
        let lignes: Vec<&str> = source.lines().collect();
        let mut nues = Vec::new();
        for (n, ligne) in lignes.iter().enumerate() {
            let taillée = ligne.trim();
            if taillée.starts_with('#') {
                continue;
            }
            // Ce qui touche au disque, au bureau ou au shell.
            let écrit = taillée.starts_with("rm ")
                || taillée.starts_with("rm -")
                || taillée.starts_with("install -")
                || taillée.starts_with("mkdir ")
                || taillée.starts_with("gsettings set")
                || taillée.starts_with("cargo build")
                || taillée.contains(">> \"$rc\"");
            if !écrit {
                continue;
            }
            // Sous une garde qui teste l'essai à blanc ? On regarde les
            // quinze lignes au-dessus : c'est la portée d'un `if` dans
            // ces scripts.
            let gardée = lignes[n.saturating_sub(15)..n]
                .iter()
                .any(|l| l.contains("$ESSAI"));
            if !gardée {
                nues.push(format!("{nom}:{} {taillée}", n + 1));
            }
        }
        assert!(
            nues.is_empty(),
            "{} geste(s) qui écriraient malgré --dry-run :\n  {}",
            nues.len(),
            nues.join("\n  ")
        );
    }
}

/// L'installation, pour de vrai, de bout en bout.
///
/// Les contrôles précédents lisent le script ; celui-ci le LANCE, et
/// regarde ce qui est posé sur le disque. C'est la seule façon de savoir
/// qu'`install.sh` marche — un script qui se relit bien peut très bien
/// ne rien installer.
///
/// Il n'écrit RIEN dans le profil de l'utilisateur : `--prefix` et
/// `--bin-dir` l'envoient dans un dossier temporaire, `--no-dock` et
/// `--no-path` lui interdisent de toucher au bureau et au shell. C'est
/// précisément ce que ces options ont rendu possible — sans elles, ce
/// test ne pouvait pas exister, et il est resté ouvert des semaines pour
/// cette raison.
///
/// `--no-build` évite de rappeler cargo depuis cargo : le binaire est
/// remplacé par un fichier quelconque, puisque ce qu'on mesure ici est
/// l'installation, non la compilation.
#[test]
fn installing_really_puts_everything_in_place() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !racine.join("install.sh").is_file() {
        return;
    }
    // Un binaire de remplacement, là où le script ira le chercher. On
    // ne touche à `target/` que s'il n'y a rien : écraser celui de la
    // compilation en cours serait une mauvaise surprise.
    let faux = racine.join("target/debug/namsdec-gui");
    if faux.exists() {
        // Le vrai est là : tant mieux, on l'installera lui.
    } else {
        let Some(parent) = faux.parent() else { return };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        if std::fs::write(&faux, b"#!/bin/sh\nexit 0\n").is_err() {
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&faux, std::fs::Permissions::from_mode(0o755));
        }
    }

    let dossier = tempfile::tempdir().expect("un dossier temporaire");
    let prefix = dossier.path().join("share");
    let bin = dossier.path().join("bin");

    let sortie = std::process::Command::new("bash")
        .arg(racine.join("install.sh"))
        .args([
            "--no-build",
            "--debug",
            "--no-dock",
            "--no-path",
            "--no-color",
            "--prefix",
        ])
        .arg(&prefix)
        .arg("--bin-dir")
        .arg(&bin)
        .output();
    let Ok(sortie) = sortie else {
        eprintln!("bash est introuvable : test ignoré");
        return;
    };
    assert!(
        sortie.status.success(),
        "l'installation a échoué :\n{}\n{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    // Ce que le script promet d'avoir posé, et rien de moins.
    for attendu in [
        bin.join("namsdec"),
        prefix.join("namsdec/exercises/namsdec.json"),
        // Les séries greffées, et non le seul parcours livré : le
        // script les pose par une boucle sur `plugins/exercices/`, et
        // une boucle qui ne trouve rien s'exécute sans rien dire.
        prefix.join("namsdec/exercises/atelier.json"),
        prefix.join("icons/hicolor/scalable/apps/namsdec.svg"),
        prefix.join("applications/namsdec.desktop"),
    ] {
        assert!(
            attendu.exists(),
            "{} manque après l'installation :\n{}",
            attendu.display(),
            String::from_utf8_lossy(&sortie.stdout)
        );
    }

    // Le lanceur pointe vers le binaire qu'on vient de poser, et non
    // vers celui d'ailleurs : un `.desktop` qui lance autre chose est
    // pire qu'un lanceur absent.
    let lanceur =
        std::fs::read_to_string(prefix.join("applications/namsdec.desktop")).expect("le lanceur");
    assert!(
        lanceur.contains(&format!("Exec={}/namsdec", bin.display())),
        "le lanceur doit pointer vers le binaire installé :\n{lanceur}"
    );

    // Et la série livrée est bien celle du dépôt, non un fichier vide.
    let pack = namsdec_exercises::pack::read(prefix.join("namsdec/exercises/namsdec.json"))
        .expect("la série installée se lit");
    assert_eq!(pack.id, namsdec_exercises::BUILTIN_ID);

    // La désinstallation défait exactement cela.
    let sortie = std::process::Command::new("bash")
        .arg(racine.join("uninstall.sh"))
        .args(["--no-color", "--prefix"])
        .arg(&prefix)
        .arg("--bin-dir")
        .arg(&bin)
        .output()
        .expect("la désinstallation tourne");
    assert!(
        sortie.status.success(),
        "la désinstallation a échoué :\n{}",
        String::from_utf8_lossy(&sortie.stderr)
    );
    for parti in [
        bin.join("namsdec"),
        prefix.join("icons/hicolor/scalable/apps/namsdec.svg"),
        prefix.join("applications/namsdec.desktop"),
    ] {
        assert!(!parti.exists(), "{} devait partir", parti.display());
    }
    // Les exercices, eux, restent : on désinstalle souvent pour
    // réinstaller, et une série qu'on a écrite ne se perd pas au passage.
    assert!(
        prefix.join("namsdec/exercises/namsdec.json").exists(),
        "les séries d'exercices sont gardées sans --purge"
    );
    assert!(
        prefix.join("namsdec/exercises/atelier.json").exists(),
        "l'atelier aussi est gardé sans --purge"
    );

    // Et l'atelier installé est bien celui du dépôt, avec ses deux
    // paliers : un fichier posé mais vide passerait les lignes
    // ci-dessus sans rien apprendre.
    let atelier = namsdec_exercises::pack::read(prefix.join("namsdec/exercises/atelier.json"))
        .expect("l'atelier installé se lit");
    assert!(
        atelier.exercises.len() >= 18,
        "l'atelier installé en compte {}",
        atelier.exercises.len()
    );
    assert!(
        atelier
            .exercises
            .iter()
            .any(|e| e.level == namsdec_exercises::Level::Mastery),
        "l'atelier monte jusqu'au palier Maîtrise"
    );
}

/// Un listing déjà mis en page ne se refait pas, et ne se re-hache pas.
///
/// Un binaire ouvert fait quinze mille lignes. La mise en page complète
/// coûte trois cents millisecondes ; le cache la rend une fois pour
/// toutes, et c'est déjà le cas depuis longtemps.
///
/// Ce qui ne l'était pas : la CLÉ de ce cache était un condensé du texte
/// entier, recalculé à chaque appel — deux fois par frame, une fois pour
/// la gouttière et une fois pour le champ de texte. Mesuré : 1,2 ms le
/// condensé, contre 11 µs la comparaison directe des mêmes octets. Cent
/// treize fois moins cher, et plus sûr : une égalité exacte ne peut pas
/// collisionner.
///
/// Le contrôle mesure le coût d'un appel servi par le cache. Le seuil
/// est large — une machine chargée mesure mal — mais il attrape le
/// retour au condensé, qui coûte cent fois le reste.
#[test]
fn an_already_laid_out_listing_costs_almost_nothing() {
    let ctx = egui::Context::default();
    let app = NamsdecApp::default();
    app.apply_theme(&ctx);
    // De la taille d'un binaire ordinaire : six cent mille octets.
    let mut listing = String::new();
    for i in 0..15_000 {
        listing.push_str(&format!(
            "{:#010x}  mov rax, {}   ; ligne {i}\n",
            0x400000 + i * 4,
            i % 97
        ));
    }
    assert!(listing.len() > 500_000, "{} octets", listing.len());

    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let mut servi = std::time::Duration::MAX;
    let mut sortie = ctx.run_ui(input, |ui| {
        // Le premier appel pose tout ; les suivants sont servis.
        let _ = crate::editor::layout_code(ui, app.theme, &listing);
        for _ in 0..20 {
            let début = std::time::Instant::now();
            let _ = crate::editor::layout_code(ui, app.theme, &listing);
            servi = servi.min(début.elapsed());
        }
    });
    sortie.textures_delta.clear();
    // Le meilleur des vingt : c'est celui qui mesure le code, les autres
    // mesurent aussi ce que la machine faisait à côté.
    assert!(
        servi < std::time::Duration::from_micros(400),
        "un appel servi par le cache coûte {servi:?} — le condensé du \
         texte entier est-il revenu ? Il vaut 1,2 ms à lui seul."
    );
}

/// La frame qui POSE le texte est celle qui le dit, et non celle d'à
/// côté.
///
/// Une frame lente a deux causes, et le message ne doit pas les
/// confondre : une mise en page qu'on paie UNE fois — trois cents
/// millisecondes à l'ouverture d'un binaire — ou un coût qui
/// reviendrait à chaque image. Crier « frame lente » pour la première
/// fait chercher une fuite là où il n'y a qu'un chargement.
///
/// La distinction se devinait en comparant la longueur du texte d'une
/// frame à l'autre. Cela marche quand le texte change et que l'éditeur
/// le pose dans la même frame — le cas ordinaire —, et se trompe
/// exactement quand il ne faut pas : un fichier choisi dans un dialogue
/// arrive parfois après le dessin, et c'est la frame SUIVANTE qui paie.
/// La longueur y avait déjà été notée.
///
/// Le cache de mise en page compte ses poses : il sait, lui, s'il a
/// payé. Le contrôle vérifie que ce compte n'avance que lorsqu'une pose
/// a vraiment lieu.
#[test]
fn the_frame_that_lays_out_the_text_is_the_one_that_says_so() {
    let ctx = egui::Context::default();
    let app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let texte = "mov rax, 1\nmov rdi, 2\nsyscall\n".repeat(200);
    let autre = "nop\n".repeat(200);

    let mut relevé = Vec::new();
    let mut sortie = ctx.run_ui(input, |ui| {
        let avant = crate::editor::poses_du_texte();
        let _ = crate::editor::layout_code(ui, app.theme, &texte);
        relevé.push(("première pose", crate::editor::poses_du_texte() - avant));

        let avant = crate::editor::poses_du_texte();
        let _ = crate::editor::layout_code(ui, app.theme, &texte);
        let _ = crate::editor::layout_code(ui, app.theme, &texte);
        relevé.push(("relectures", crate::editor::poses_du_texte() - avant));

        let avant = crate::editor::poses_du_texte();
        let _ = crate::editor::layout_code(ui, app.theme, &autre);
        relevé.push(("texte changé", crate::editor::poses_du_texte() - avant));
    });
    sortie.textures_delta.clear();

    assert_eq!(relevé[0], ("première pose", 1), "{relevé:?}");
    assert_eq!(
        relevé[1],
        ("relectures", 0),
        "relire du texte déjà posé ne coûte pas une pose : {relevé:?}"
    );
    assert_eq!(
        relevé[2],
        ("texte changé", 1),
        "un texte nouveau se pose : {relevé:?}"
    );
}

/// La recherche de mise à jour se laisse retirer, et l'est par une seule
/// option.
///
/// La pile TLS qu'elle traîne — ureq, rustls, ring — pèse 1,6 Mo sur un
/// binaire de 16, soit dix pour cent, pour une fonction qu'on emploie
/// une fois par mois. C'est le cas d'un empaqueteur de distribution :
/// les mises à jour sont son travail, pas celui de l'application.
///
/// L'option est activée PAR DÉFAUT — l'application livrée ne change
/// pas. Le contrôle vérifie les deux moitiés de cette promesse : que
/// l'option existe, et qu'elle est dans les défauts. Une option
/// d'allègement qu'on oublierait d'activer livrerait une application
/// amputée sans que personne ne s'en aperçoive avant l'utilisateur.
#[test]
fn the_update_check_can_be_left_out_by_one_option() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let gui = std::fs::read_to_string(racine.join("Cargo.toml")).expect("Cargo.toml du gui");
    assert!(
        gui.contains("reseau = [\"namsdec-update/reseau\"]"),
        "l'option doit passer la consigne au crate qui porte le réseau :\n{gui}"
    );
    assert!(
        gui.contains("default = [\"reseau\"]"),
        "et être activée par défaut — l'application livrée ne change pas :\n{gui}"
    );

    let update =
        std::fs::read_to_string(racine.join("../namsdec-update/Cargo.toml")).expect("Cargo.toml");
    assert!(
        update.contains("ureq = { workspace = true, optional = true }"),
        "la pile réseau doit être optionnelle là où elle est déclarée :\n{update}"
    );

    // Et le workspace n'active pas les défauts du crate à la place du
    // gui : sans cela, l'option ne servirait à rien, puisque `ureq`
    // entrerait quoi qu'on demande.
    let ws = std::fs::read_to_string(racine.join("../../Cargo.toml")).expect("Cargo.toml racine");
    assert!(
        ws.contains(
            "namsdec-update = { path = \"crates/namsdec-update\", default-features = false }"
        ),
        "le workspace doit laisser le gui décider :\n{ws}"
    );

    // Ce qui ne dépend PAS du réseau reste disponible sans l'option :
    // comparer deux versions, lire une réponse, rendre un verdict. Le
    // vérifier ici garde la frontière à sa place.
    let lib = std::fs::read_to_string(racine.join("../namsdec-update/src/lib.rs")).expect("lib.rs");
    let bornées = lib.matches("#[cfg(feature = \"reseau\")]").count();
    assert!(
        bornées <= 3,
        "{bornées} éléments bornés au réseau — seule la requête doit \
         l'être, le reste se teste et se lit sans lui"
    );
}

/// La case « Pseudo-code » fait ce qu'elle annonce — au CLIC, et non
/// par un état posé à la main.
///
/// Tous les contrôles d'interface de ce fichier fabriquent l'état —
/// `flow_pseudo: true` — puis mesurent ce qui est peint. Ils vérifient
/// donc que la VUE est juste, jamais que le GESTE y mène : on peut
/// débrancher la case sans qu'un seul d'entre eux rougisse.
///
/// C'est ce qu'`egui_kittest` apporte, et la seule raison de le garder.
/// Évalué le 30 septembre 2026 contre le banc maison : à longueur
/// égale, il ne raccourcit rien — le mandat disait qu'alors il sortait.
/// Mais il clique, et le banc ne sait pas le faire. Les captures non
/// plus : sous Wayland, les frappes et les clics simulés n'atteignent
/// pas l'application, ce qui a été vérifié deux fois.
///
/// L'emploi est donc borné : kittest pour les GESTES, le banc pour la
/// mise en page. Mélanger les deux ferait perdre au banc ce qu'il a de
/// mieux — il mesure des pixels, là où l'arbre d'accessibilité ne
/// connaît que des noms.
#[test]
fn clicking_the_pseudo_code_box_really_switches_the_view() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        source: "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                 boucle:\n    dec ecx\n    jnz boucle\n    hlt\n"
            .to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.do_assemble();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui(move |ui| app.draw(ui));
    harness.run();

    assert!(
        harness.query_by_label_contains("répéter").is_none(),
        "au départ, c'est le graphe qu'on voit"
    );
    harness.get_by_label(s.flow_pseudo).click();
    harness.run();
    assert!(
        harness.query_by_label_contains("répéter").is_some(),
        "cocher la case montre le pseudo-code"
    );
    // Et le chemin du retour : une bascule qui ne rebascule pas est une
    // porte à sens unique, ce qui ne se voit qu'en essayant.
    harness.get_by_label(s.flow_pseudo).click();
    harness.run();
    assert!(
        harness.query_by_label_contains("répéter").is_none(),
        "décocher la rend au graphe"
    );
}

/// Les bascules du menu Affichage font ce qu'elles annoncent.
///
/// Quatre cases commandent ce qu'on voit : la barre d'outils, le panneau
/// latéral, la barre de statut, la console. Aucun contrôle ne vérifiait
/// qu'elles MARCHENT — ils posaient l'état (`show_toolbar: false`) et
/// mesuraient ce qui était peint, ce qui passe au vert même si la case
/// est débranchée.
///
/// Celui-ci ouvre le menu, clique, et regarde. C'est plus lent et c'est
/// le seul qui réponde à la question qu'on se pose vraiment : « est-ce
/// que ça marche ? »
#[test]
fn the_view_menu_toggles_really_toggle() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let s = strings(Lang::Fr);
    let mut app = NamsdecApp::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui(move |ui| app.draw(ui));
    harness.run();

    // Chaque bascule, avec un témoin de la zone commandée — non la case
    // elle-même : c'est la zone qui doit partir, pas la case.
    //
    // On COMPTE les occurrences plutôt que d'en exiger une seule : un
    // même libellé sert souvent à deux endroits — « Assembler » est
    // dans la barre d'outils ET dans le menu qui la commande. Exiger
    // l'unicité ferait échouer le contrôle sur une coïncidence de
    // vocabulaire, ce qui n'apprend rien sur la bascule.
    let combien = |harness: &Harness<'_>, quoi: &str| -> usize {
        harness.get_all_by_label_contains(quoi).count()
    };
    // UNE seule bascule, et c'est un choix.
    //
    // Les trois autres se heurtent à deux limites qu'il vaut mieux
    // connaître que contourner :
    //
    //   - l'arbre d'accessibilité n'expose que ce qu'egui pose comme
    //     widget. La barre de statut et le panneau latéral, peints à la
    //     main, n'y figurent pas — leurs témoins sont introuvables ;
    //   - kittest cherche dans TOUT l'arbre, sans notion de « dans le
    //     menu ouvert ». « Console » nomme à la fois l'entrée du menu et
    //     l'onglet du bas, et la requête refuse de choisir.
    //
    // Un contrôle juste sur une bascule vaut mieux qu'un contrôle
    // fragile sur quatre. Le banc, lui, mesure ce qui est peint et
    // restreint son relevé à une fenêtre : les deux outils ne couvrent
    // pas la même moitié du problème.
    {
        let (bascule, témoin) = (s.view_toolbar, s.action_assemble);
        let avant = combien(&harness, témoin);
        assert!(avant > 0, "« {témoin} » doit être là au départ");

        harness.get_by_label(s.menu_view).click();
        harness.run();
        harness.get_by_label(bascule).click();
        harness.run();
        assert!(
            combien(&harness, témoin) < avant,
            "décocher « {bascule} » doit faire partir « {témoin} » \
             (toujours {} occurrence(s))",
            combien(&harness, témoin)
        );

        // Et le retour : une bascule qui ne rebascule pas est une porte
        // à sens unique, ce qui ne se voit qu'en essayant.
        harness.get_by_label(s.menu_view).click();
        harness.run();
        harness.get_by_label(bascule).click();
        harness.run();
        assert_eq!(
            combien(&harness, témoin),
            avant,
            "et la recocher doit le ramener"
        );
    }
}

/// Le profil d'un gros binaire se dessine en quelques dizaines de
/// millisecondes, non en douze secondes.
///
/// Il en prenait DOUZE. Le classement compte cinq cents lignes, et pour
/// chacune le panneau demandait « quelle est l'étiquette au-dessus ? »
/// en remontant le listing ligne par ligne. Devant un binaire sans
/// symboles — le cas ordinaire en rétro-ingénierie — il n'y a aucune
/// étiquette : la recherche parcourait donc les soixante mille lignes,
/// cinq cents fois, à chaque image. L'application devenait inutilisable
/// au moment précis où l'on avait besoin d'elle.
///
/// Les étiquettes sont relevées UNE fois, et la recherche est binaire.
/// Le pire cas — aucune étiquette — est devenu le plus rapide.
///
/// Le seuil est large : une machine chargée mesure mal, et ce qu'on
/// veut attraper est un retour au parcours complet, qui coûte cent
/// cinquante fois plus.
#[test]
fn a_big_binarys_profile_draws_in_tens_of_milliseconds() {
    let chemin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/binaires/crackme-01");
    let Ok(octets) = std::fs::read(&chemin) else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.load_binary(octets, "crackme-01");
    assert!(
        app.source.lines().count() > 20_000,
        "le binaire d'essai doit être gros : {} lignes",
        app.source.lines().count()
    );
    // On le fait courir, pour que le profil ait de quoi classer.
    for _ in 0..2000 {
        if app.emulator.as_mut().map(|e| e.step()).is_none() {
            break;
        }
    }
    assert!(
        app.line_visits().len() > 100,
        "le classement doit être fourni : {} lignes",
        app.line_visits().len()
    );

    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(2300.0, 1390.0),
        )),
        ..Default::default()
    };
    app.console_tab = ConsoleTab::Profile;
    // Le meilleur de plusieurs tours : le premier paie la mise en page
    // du listing, les suivants disent le coût en régime — celui qu'on
    // ressent en déplaçant la souris.
    let mut régime = std::time::Duration::MAX;
    for _ in 0..4 {
        let début = std::time::Instant::now();
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
        régime = régime.min(début.elapsed());
    }
    assert!(
        régime < std::time::Duration::from_millis(600),
        "une image du profil coûte {régime:?} — la recherche d'étiquette \
         remonte-t-elle à nouveau tout le listing ? Elle valait douze \
         secondes à elle seule."
    );
}

/// Les séquences que la documentation annonce sont celles qui marchent.
///
/// Le README promettait `Espace s i` pour avancer d'un pas et
/// `Espace n i` pour enjamber un appel. Le jeu de raccourcis, lui, fait
/// `Espace p i` et `Espace p o` depuis qu'il range les séquences par
/// famille. Personne ne s'en était aperçu, et la seule façon de le
/// découvrir était d'essayer : on lit le mode d'emploi, on tape ce qu'il
/// dit, il ne se passe rien — et l'on croit avoir mal compris le
/// programme, non la page.
///
/// Le contrôle lit les séquences ANNONCÉES dans le README et le manuel,
/// et vérifie que le jeu Fake Vim-GDB les porte vraiment. Une
/// documentation qui ment coûte plus qu'une documentation absente.
#[test]
fn the_sequences_the_docs_promise_are_the_ones_that_work() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let jeu = keybindings::KeyBindings::vim();

    // Toutes les séquences du jeu, écrites comme la documentation les
    // écrit : « Espace p i ».
    let connues: std::collections::BTreeSet<String> = jeu
        .map
        .values()
        .filter_map(|accord| match accord {
            keybindings::Chord::Leader(touches) => Some(
                touches
                    .as_slice()
                    .iter()
                    .map(|k| match k {
                        egui::Key::Space => "Espace".to_owned(),
                        egui::Key::Slash => "/".to_owned(),
                        autre => format!("{autre:?}").to_lowercase(),
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            _ => None,
        })
        .collect();
    assert!(
        connues.len() > 10,
        "le relevé ne trouve presque aucune séquence ({}) — il regarde \
         au mauvais endroit",
        connues.len()
    );

    // Ce que les pages promettent : « Espace x y », en gras ou non.
    let motif = regex_maison;
    let mut promises = Vec::new();
    for page in ["README.md", "docs/guide.py"] {
        let Ok(texte) = std::fs::read_to_string(racine.join(page)) else {
            continue;
        };
        for séquence in motif(&texte) {
            promises.push((page, séquence));
        }
    }
    assert!(
        promises.len() > 3,
        "le relevé ne trouve presque aucune promesse ({}) — il regarde \
         au mauvais endroit",
        promises.len()
    );
    let menteuses: Vec<String> = promises
        .iter()
        .filter(|(_, s)| !connues.contains(s))
        .map(|(page, s)| format!("{page} : « {s} »"))
        .collect();
    assert!(
        menteuses.is_empty(),
        "{} séquence(s) annoncée(s) qui n'existent pas dans le jeu — \
         on lit le mode d'emploi, on tape, il ne se passe rien :\n  {}\n\
         Celles qui existent : {connues:?}",
        menteuses.len(),
        menteuses.join("\n  ")
    );
}

/// Les séquences « Espace x y » d'une page, relevées à la main.
///
/// À la main parce qu'une seule expression régulière servirait ici, et
/// qu'une dépendance pour trois lignes se paierait sur chaque
/// compilation.
fn regex_maison(texte: &str) -> Vec<String> {
    let mut trouvées = Vec::new();
    let nettoyé = texte.replace("**", "").replace(" puis ", " ");
    for (position, _) in nettoyé.match_indices("Espace ") {
        let suite = &nettoyé[position..];
        let mots: Vec<&str> = suite.split_whitespace().take(3).collect();
        if mots.len() < 3 {
            continue;
        }
        // Deux lettres ou signes courts après « Espace » : au-delà,
        // c'est une phrase, non une séquence.
        // Une lettre ASCII ou une barre oblique. Sans cette borne, le
        // relevé prenait le « … » de la phrase qui EXPLIQUE ce qu'est une
        // séquence, et accusait le manuel de promettre « Espace … ».
        let bon = |m: &str| {
            let m = m.trim_end_matches([',', '.', ':', ')', '`', '<', '»']);
            m.chars().count() == 1 && m.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
        };
        if bon(mots[1]) && bon(mots[2]) {
            let taille = |m: &str| {
                m.trim_end_matches([',', '.', ':', ')', '`', '<', '»'])
                    .to_owned()
            };
            trouvées.push(format!("Espace {} {}", taille(mots[1]), taille(mots[2])));
        }
    }
    trouvées.sort();
    trouvées.dedup();
    trouvées
}

/// Le désassemblage se lit aussi dans la syntaxe du système.
///
/// `mov rax, [rbx+8]` d'un côté, `movq 8(%rbx), %rax` de l'autre : les
/// deux disent la même chose et ne se ressemblent pas. Ce n'est pas un
/// détail de goût — dès qu'on sort de namsdec, un rapport d'`objdump`,
/// une session `gdb` ou un tutoriel trouvé en ligne écrivent tout dans
/// l'autre, et la traduction de tête est la marche où l'on abandonne.
///
/// C'est un moteur de plus dans la liste existante, et non un réglage
/// ailleurs : le choix se fait là où l'on choisit déjà ce qu'on lit.
#[test]
fn the_disassembly_reads_in_the_systems_syntax_too() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let peint = |moteur: &str| -> Vec<String> {
        let mut app = NamsdecApp {
            source: "section .text\nglobal _start\n_start:\n    mov rax, rbx\n    hlt\n".to_owned(),
            show_disassembly: true,
            disassembler: moteur.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("disassembly_window"), 4);
        lignes.into_iter().map(|(t, _)| t).collect()
    };

    // Le choix est offert là où l'on choisit déjà le moteur.
    let nasm = peint("nasm");
    assert!(
        nasm.iter().any(|t| t == s.disassembly_system),
        "le moteur système doit figurer dans la liste : {nasm:?}"
    );
    assert!(
        nasm.iter().any(|t| t.contains("rax")),
        "NASM écrit les registres nus : {nasm:?}"
    );

    // Et il montre bien l'autre syntaxe.
    let système = peint("system");
    assert!(
        système.iter().any(|t| t.contains("%rax")),
        "le système préfixe les registres d'un % : {système:?}"
    );
    assert!(
        !système.iter().any(|t| t.contains("mov rax")),
        "et n'écrit plus la forme NASM : {système:?}"
    );
}

/// Le choix du moteur se retrouve à la séance suivante.
#[test]
fn the_chosen_disassembler_survives_the_session() {
    let app = NamsdecApp {
        disassembler: "system".to_owned(),
        ..Default::default()
    };
    let réglages = app.settings_snapshot();
    assert_eq!(réglages.disassembler, "system");
    assert_eq!(NamsdecApp::from_settings(réglages).disassembler, "system");
}

/// On récrit une instruction en assembleur, pas seulement en octets.
///
/// Corriger un binaire demandait de connaître l'encodage : pour
/// neutraliser un test, il fallait savoir que `nop` s'écrit `90`. C'est
/// une marche de plus, et elle n'apprend rien — l'assembleur est là pour
/// éviter d'écrire des octets à la main.
///
/// Un seul champ accepte les deux, et la règle est dite : ce qui peut se
/// lire en octets se lit en octets. Sans cet ordre, `fadd` — quatre
/// lettres qui sont aussi quatre chiffres hexadécimaux — changerait de
/// sens selon l'humeur du programme.
#[test]
fn an_instruction_can_be_rewritten_in_assembly() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    // L'assembleur rend bien les octets attendus, adresse comprise :
    // un saut s'écrit en relatif, et ne produit pas les mêmes octets
    // selon l'endroit d'où l'on saute.
    assert_eq!(
        namsdec_asm::assemble_one("mov eax, 1", 0x401000).expect("mov s'assemble"),
        vec![0xB8, 0x01, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        namsdec_asm::assemble_one("nop", 0x401000).expect("nop s'assemble"),
        vec![0x90]
    );
    let depuis_1000 = namsdec_asm::assemble_one("jmp 0x401010", 0x401000).expect("saut");
    let depuis_2000 = namsdec_asm::assemble_one("jmp 0x401010", 0x402000).expect("saut");
    assert_ne!(
        depuis_1000, depuis_2000,
        "un saut est relatif : les octets dépendent d'où l'on part"
    );

    // Et ce qui ne s'assemble pas le dit, avec le message de nasm.
    let erreur = namsdec_asm::assemble_one("mvo eax, 1", 0x401000)
        .expect_err("un mnémonique inexistant doit être refusé");
    let namsdec_asm::AsmError::Failed { diagnostics } = &erreur else {
        panic!("l'erreur doit porter les diagnostics de nasm : {erreur}");
    };
    assert!(
        !diagnostics.is_empty(),
        "et nasm doit dire pourquoi : {erreur}"
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
