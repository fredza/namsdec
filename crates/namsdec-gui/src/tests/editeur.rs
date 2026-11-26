//! L'éditeur : le clavier, le curseur, le texte.
//!
//! Les modes de vim, les mouvements, le compte qui répète, les paires qui
//! se ferment, la complétion, le surlignage et le renommage, les occurrences
//! et leurs coordonnées. Ce qui concerne le *listing* d'un binaire est dans
//! [`super::listing`].
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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

/// Le mode VISUEL existe, et `v` y mène depuis le mode Normal.
///
/// Il manquait : `v` ne faisait rien, et l'on ne pouvait sélectionner
/// qu'à la souris ou en Insertion. Les mouvements y étendent la
/// sélection depuis l'ancre posée à l'entrée, `y` copie, `d` coupe,
/// Échap renonce.
#[test]
fn visual_mode_selects_and_yanks() {
    let frappe = |app: &mut NamsdecApp, ctx: &egui::Context, key: egui::Key| {
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
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    };

    let mut app = NamsdecApp::default();
    app.set_source("abcdef\nghijkl\n".to_owned());
    app.cursor_byte = 0;
    let ctx = egui::Context::default();

    // `v` ouvre le mode, et pose l'ancre là où le curseur est.
    frappe(&mut app, &ctx, egui::Key::V);
    assert_eq!(app.editor_mode, EditorMode::Visual, "`v` ouvre le Visuel");
    assert_eq!(app.visual_anchor, Some(0), "l'ancre est sous le curseur");
    // Un seul caractère, et non zéro : le Visuel de vim est inclusif.
    assert_eq!(app.visual_span(), Some(0..1));

    // Deux `l` étendent la sélection vers la droite.
    frappe(&mut app, &ctx, egui::Key::L);
    frappe(&mut app, &ctx, egui::Key::L);
    assert_eq!(
        app.visual_span(),
        Some(0..3),
        "les mouvements étendent au lieu de porter seulement le curseur"
    );

    // `y` copie et revient en Normal, sans toucher au texte.
    frappe(&mut app, &ctx, egui::Key::Y);
    assert_eq!(app.editor_mode, EditorMode::Normal, "`y` referme le Visuel");
    assert_eq!(app.visual_anchor, None, "et l'ancre ne traîne pas");
    assert_eq!(app.source, "abcdef\nghijkl\n", "copier ne coupe pas");
}

/// `d` en Visuel retire la sélection — y compris sur plusieurs lignes.
#[test]
fn visual_mode_cuts_across_lines() {
    let frappe = |app: &mut NamsdecApp, ctx: &egui::Context, key: egui::Key| {
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    };

    let mut app = NamsdecApp::default();
    app.set_source("abc\ndef\nghi\n".to_owned());
    app.cursor_byte = 0;
    let ctx = egui::Context::default();

    frappe(&mut app, &ctx, egui::Key::V);
    // Une ligne plus bas : la sélection traverse une fin de ligne, ce
    // qui est le cas ordinaire et celui que `place_spans` refuse de
    // situer d'un seul tenant.
    frappe(&mut app, &ctx, egui::Key::J);
    let étendue = app.visual_span().expect("une sélection");
    assert!(
        app.source[étendue.clone()].contains('\n'),
        "elle traverse une fin de ligne : {:?}",
        &app.source[étendue]
    );

    frappe(&mut app, &ctx, egui::Key::D);
    assert_eq!(app.editor_mode, EditorMode::Normal);
    assert_eq!(app.source, "ef\nghi\n", "la sélection est retirée");
}

/// Échap renonce sans rien couper : c'est la seule sortie sûre.
#[test]
fn escape_leaves_visual_mode_without_cutting() {
    let mut app = NamsdecApp::default();
    app.set_source("abcdef\n".to_owned());
    app.cursor_byte = 0;
    let ctx = egui::Context::default();
    for key in [egui::Key::V, egui::Key::L, egui::Key::Escape] {
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    }
    assert_eq!(app.editor_mode, EditorMode::Normal);
    assert_eq!(app.visual_anchor, None);
    assert_eq!(app.source, "abcdef\n", "rien n'a été touché");
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

/// L'ÉDITEUR, lui, ne garde aucune barre à demeure.
///
/// C'est l'exception à l'invariant ci-dessus, et elle est demandée :
/// « la barre de défilement dans l'éditeur n'est pas nécessaire »
/// (20 octobre 2026). Elle prenait sa place deux fois — la largeur
/// qu'elle réserve en permanence, et un liseré clair le long du code,
/// à demeure, pour une information qu'on a déjà : la gouttière numérote
/// les lignes, la barre d'état dit où est le curseur.
///
/// Ailleurs, l'invariant d'origine tient toujours : une liste qui
/// continue sous le bord doit le dire. Dans un éditeur où l'on navigue
/// au clavier, c'est un meuble.
///
/// Mesuré sur le CÔTÉ ÉDITEUR de l'écran seulement — le panneau
/// latéral, lui, garde les siennes, et les compter ici ferait passer le
/// contrôle pour rien.
#[test]
fn the_editor_keeps_no_scroll_bar_at_rest() {
    let barres_côté_éditeur = |largeur_écran: f32| -> Vec<(f32, f32)> {
        let long = format!(
            "global _start\nsection .text\n_start:\n{}    hlt\n",
            "    nop ; une ligne volontairement longue pour déborder à droite aussi\n".repeat(200)
        );
        let mut app = NamsdecApp {
            source: long,
            active_tab: SidebarTab::Cpu,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur_écran, 900.0),
            )),
            ..Default::default()
        };
        // Le bord DROIT de l'éditeur, relevé et non deviné : c'est le
        // découpage le plus large qui parte du bord gauche de l'écran et
        // descende sur toute la hauteur. Une fraction choisie à la main
        // aurait laissé le contrôle passer au vert sans rien regarder —
        // c'est ce qui est arrivé en l'écrivant.
        let mut barres = Vec::new();
        for _ in 0..3 {
            barres.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            let bord = output
                .shapes
                .iter()
                .filter(|c| c.clip_rect.min.x < 5.0 && c.clip_rect.height() > 200.0)
                .map(|c| c.clip_rect.max.x)
                .fold(0.0f32, f32::max);
            assert!(
                bord > largeur_écran * 0.3,
                "le bord de l'éditeur n'a pas été trouvé ({bord}) — le relevé ne \
                 regarde rien"
            );
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Rect(r) = &clipped.shape {
                    let étroite_et_haute = r.rect.width() < 20.0 && r.rect.height() > 60.0;
                    // Au moins deux pixels de large : le filet qui sépare
                    // l'éditeur du panneau latéral en fait un, et ce
                    // n'est pas une barre.
                    if étroite_et_haute && r.rect.width() >= 2.0 && r.rect.max.x <= bord {
                        barres.push((r.rect.min.x.round(), r.rect.width().round()));
                    }
                }
            }
            output.textures_delta.clear();
        }
        barres
    };

    for largeur in [1280.0, 1600.0] {
        let barres = barres_côté_éditeur(largeur);
        assert!(
            barres.is_empty(),
            "à {largeur}px, l'éditeur ne doit porter aucune barre au repos : {barres:?}"
        );
    }
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
    app.apply_theme(&ctx);
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
    // Le curseur descend d'une ligne et avance de quatre colonnes : les
    // gestes du mode Normal, ceux de vim. C'est l'APPLICATION qui porte
    // sa position — en mode Normal, le champ de texte n'est pas même
    // dessiné (voir `vue_listing`), et la barre d'état restait figée sur
    // « Lig 1, Col 1 » du temps où elle la lisait dans son état.
    app.move_in_normal_mode_times(&ctx, Move::Down, 1);
    app.move_in_normal_mode_times(&ctx, Move::Right, 4);
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

/// UNE PARENTHÈSE TAPÉE EN POSE UNE, PAS CENT.
///
/// Le contrôle d'à côté éprouve `pair_edit`, c'est-à-dire la DÉCISION :
/// « faut-il fermer ici ? ». Il répondait juste, et l'éditeur était
/// pourtant inutilisable — rien ne liait la pose à une frappe, si bien
/// qu'elle se refaisait à chaque image tant que le curseur restait
/// derrière un ouvrant. Taper `(` donnait `()`, puis `(())`, puis
/// `((()))`, à soixante fermantes par seconde. Signalé sur capture le
/// 21 octobre 2026, dans un fichier C.
///
/// Celui-ci éprouve le GESTE, sur plusieurs images : c'est la seule
/// façon de voir un défaut qui ne se produit qu'à la deuxième. Une
/// décision juste jouée au mauvais moment reste un défaut.
#[test]
fn typing_one_parenthesis_inserts_one_closing_not_a_hundred() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        auto_pairs: true,
        editor_mode: EditorMode::Insert,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.set_source(String::new());

    let image = |app: &mut NamsdecApp, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 900.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    };

    // Le champ doit avoir le clavier, sans quoi rien n'est tapé.
    image(&mut app, vec![]);
    ctx.memory_mut(|m| m.request_focus(editor_id()));
    image(&mut app, vec![]);

    // UNE frappe.
    image(&mut app, vec![egui::Event::Text("(".to_owned())]);
    // Puis dix images où l'on ne touche à rien — c'est là que le
    // défaut se produisait.
    for _ in 0..10 {
        image(&mut app, vec![]);
    }

    let fermantes = app.source.matches(')').count();
    assert!(
        fermantes <= 1,
        "une parenthèse tapée en pose au plus une : obtenu {fermantes} dans {:?}",
        app.source
    );
    assert!(
        app.source.starts_with('('),
        "et l'ouvrante est bien là : {:?}",
        app.source
    );
}

/// LE COMPTEUR DE RÉPÉTITION N'EXISTE QU'EN MODE NORMAL.
///
/// Rien ne le disait : taper « 8 » en Insertion écrivait le chiffre ET
/// armait le compteur, si bien que la barre d'état affichait
/// « INSERT ×8 » pendant qu'on tapait du texte. Vu sur capture le
/// 21 octobre 2026, à côté d'un autre défaut — c'est souvent ainsi
/// qu'on les trouve.
#[test]
fn the_repeat_count_only_exists_in_normal_mode() {
    let ctx = egui::Context::default();
    let frappe = |app: &mut NamsdecApp, key: egui::Key| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 900.0),
            )),
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    };

    // En NORMAL, le chiffre arme le compteur : c'est le `5` de `5j`.
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    frappe(&mut app, egui::Key::Num5);
    assert_eq!(
        app.count_pending,
        Some(5),
        "en mode Normal, un chiffre compte"
    );

    // En INSERTION, le même chiffre est du texte, et rien d'autre.
    let mut app = NamsdecApp {
        editor_mode: EditorMode::Insert,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    frappe(&mut app, egui::Key::Num8);
    assert_eq!(
        app.count_pending, None,
        "en Insertion, « 8 » est un huit — pas une répétition"
    );
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
    if let Some(octets) = binaire_fabriqué() {
        let mut app = NamsdecApp::default();
        assert!(app.load_elf(&octets, NOM_FABRIQUÉ));
        let listing = app.source.clone();
        app.insert_at_cursor(COLLÉ);
        app.replace_all_with(COLLÉ);
        assert_eq!(app.source, listing, "un listing ne se colle pas dedans");
    }
}

/// Les occurrences d'une recherche se situent en une passe, et juste.
#[test]
fn an_occurrence_knows_its_line_and_column() {
    let texte = "mov rax, 1\nmov rbx, 2\n  mov rcx, rax\n";
    let étendues: Vec<std::ops::Range<usize>> = find_matches(texte, "mov")
        .into_iter()
        .map(|at| at..at + 3)
        .collect();
    let places = place_spans(texte, &étendues);
    assert_eq!(places.len(), 3, "trois fois « mov »");
    assert_eq!((places[0].ligne, places[0].colonne), (1, 0));
    assert_eq!((places[1].ligne, places[1].colonne), (2, 0));
    // Deux espaces d'indentation : la colonne les compte.
    assert_eq!((places[2].ligne, places[2].colonne), (3, 2));
    assert!(
        places.iter().all(|place| place.largeur == 3),
        "trois caractères chacune"
    );
}

/// La casse ne compte pas, et la colonne se compte en CARACTÈRES.
#[test]
fn an_occurrence_counts_characters_and_ignores_case() {
    // « é » pèse deux octets : une colonne comptée en octets poserait le
    // surlignage un caractère trop loin.
    let texte = "; prépare\n    MOV rax, 1\n";
    let places = find_places(texte, "mov");
    assert_eq!(places.len(), 1, "« MOV » répond à « mov »");
    assert_eq!(places[0].ligne, 2);
    assert_eq!(places[0].colonne, 4);
    let places = find_places(texte, "pare");
    assert_eq!(places[0].colonne, 5, "après « ; pré », soit cinq signes");
}

/// Une étendue à cheval sur deux lignes n'est pas située.
///
/// Elle n'arrive pas d'une recherche — on n'y tape pas de retour à la
/// ligne — mais un surlignage qui traverserait une fin de ligne serait
/// faux, et c'est le genre de faux qu'on ne voit qu'une fois peint.
#[test]
fn a_span_across_two_lines_is_not_placed() {
    let texte = "un\ndeux\n";
    // Deux étendues au même départ : l'une tient sur sa ligne, l'autre
    // la traverse.
    let places = place_spans(texte, &[0..2, 0..5]);
    assert_eq!(places.len(), 1, "une seule des deux est située");
    assert_eq!(places[0].octets, 0..2, "celle qui tient sur sa ligne");
}

/// La ligne du curseur se prend autour de lui, et c'est la bonne.
#[test]
fn the_line_around_a_byte_is_the_line_it_sits_on() {
    let texte = "première\ndeuxième\ntroisième";
    // Les positions sont en OCTETS, et « è » en pèse deux : le retour à
    // la ligne de la première est au neuvième octet, non au huitième.
    assert_eq!(ligne_autour(texte, 0), "première");
    assert_eq!(
        ligne_autour(texte, 9),
        "première",
        "sur le retour à la ligne"
    );
    assert_eq!(ligne_autour(texte, 10), "deuxième");
    assert_eq!(ligne_autour(texte, texte.len()), "troisième");
    // Et le même résultat que `lines().nth()`, qui est ce qu'elle
    // remplace.
    for (rang, ligne) in texte.lines().enumerate() {
        let octet = ligne_début(texte, rang).expect("la ligne existe");
        assert_eq!(ligne_autour(texte, octet), ligne, "ligne {rang}");
    }
}

/// Ligne, colonne et octet en une passe disent la même chose que les
/// trois fonctions qu'ils remplacent.
#[test]
fn line_column_and_byte_agree_with_what_they_replace() {
    let texte = "mov rax, 1\n; é accentué\nhlt\n";
    for rang in 0..=texte.chars().count() + 2 {
        let (ligne, colonne, octet) = line_col_byte(texte, rang);
        assert_eq!((ligne, colonne), line_col(texte, rang), "rang {rang}");
        assert_eq!(octet, char_to_byte(texte, rang), "rang {rang}");
    }
}

/// Le compteur de vim : `5j` descend de cinq lignes, et coûte un `j`.
#[test]
fn a_count_repeats_a_movement() {
    let texte: String = (1..=20).map(|n| format!("ligne {n}\n")).collect();
    let cinq_fois = move_cursor_times(&texte, 0, Move::Down, 5);
    let un_par_un = (0..5).fold(0, |at, _| move_cursor(&texte, at, Move::Down));
    assert_eq!(cinq_fois, un_par_un, "cinq fois `j`, c'est `5j`");
    assert_eq!(line_col_byte(&texte, cinq_fois).0, 6, "de la 1 à la 6");
    // Buté : on s'arrête là où le texte s'arrête, sans faire les neuf
    // mille autres tours.
    let fin = move_cursor_times(&texte, 0, Move::Down, 9999);
    assert_eq!(
        fin,
        move_cursor_times(&texte, 0, Move::Down, 100),
        "la dernière ligne est la dernière, qu'on demande cent ou dix mille"
    );
    assert_eq!(
        fin,
        texte.chars().count(),
        "et c'est la fin du texte, le dernier retour à la ligne compris"
    );
    // Les mots aussi.
    let ligne = "mov qword [rbp-0x18], rax";
    assert_eq!(
        move_cursor_times(ligne, 0, Move::WordForward, 3),
        (0..3).fold(0, |at, _| move_cursor(ligne, at, Move::WordForward))
    );
}

/// Le compteur est borné, et la borne tient.
///
/// Sans elle, un doigt resté sur une touche déborderait l'entier du
/// compteur, et le mouvement partirait là où personne ne l'a demandé.
#[test]
fn a_count_cannot_grow_without_bound() {
    let mut compte = None;
    for _ in 0..12 {
        compte = Some(crate::editor::borne_du_compte(compte, 9));
    }
    assert_eq!(compte, Some(9999));
}

/// La ligne qu'on lisait revient avec le binaire — et le mode ne change
/// pas pour autant.
#[test]
fn the_line_we_were_reading_comes_back() {
    let Some(octets) = binaire_fabriqué_long() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    let lignes = app.source.lines().count();
    assert!(lignes > 1_000, "un listing de {lignes} lignes");
    // Ce que les réglages de la séance précédente porteraient.
    app.saved_settings.listing_line = Some(4_012);
    app.restore_session_marks();
    assert_eq!(
        app.reprise_ligne,
        Some(4_012),
        "la ligne de lecture est reprise"
    );
    assert_eq!(
        app.cursor_pos.0, 4_012,
        "et la barre d'état le dit dès la première image, avant que \
         l'éditeur se dessine"
    );
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(input, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    assert_eq!(
        app.editor_mode,
        EditorMode::Normal,
        "on rouvre pour LIRE : « INSERT » au premier regard serait une surprise"
    );
    assert!(
        app.reprise_ligne.is_none(),
        "la reprise est consommée, et ne reprend pas le curseur à chaque image"
    );
    // Et une ligne qui n'existe plus — binaire plus court — est écartée
    // plutôt que de poser le curseur n'importe où.
    app.saved_settings.listing_line = Some(lignes as u32 + 10);
    app.restore_session_marks();
    assert!(
        app.reprise_ligne.is_none(),
        "hors du listing, la reprise est écartée"
    );
}

/// Le compteur en attente se voit dans la barre d'état.
///
/// Taper `5` puis hésiter laisse l'éditeur dans un état qui change ce
/// que la touche suivante va faire. Un état qui agit et ne se voit pas
/// est un piège — c'est la règle du projet pour le mode, elle vaut pour
/// le compteur.
#[test]
fn a_pending_count_is_shown() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 800.0),
        )),
        ..Default::default()
    };
    // Ce que la barre d'état peint, mot par mot.
    let peints = |app: &mut NamsdecApp| -> Vec<String> {
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        let textes = sortie
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        sortie.textures_delta.clear();
        textes
    };
    // Un nombre qu'aucun numéro de ligne de la gouttière ne porte : « 12 »
    // en est un, et c'est ce qui faisait passer ce contrôle pour rien.
    let sans = peints(&mut app);
    assert!(
        !sans.iter().any(|texte| texte == "×4242"),
        "rien à montrer tant qu'aucun chiffre n'est tapé : {sans:?}"
    );
    app.count_pending = Some(4242);
    let avec = peints(&mut app);
    // « ×4242 » et non « 4242 » : le nombre seul ne disait pas ce qu'il
    // était, et il se lisait comme un numéro de ligne.
    assert!(
        avec.iter().any(|texte| texte == "×4242"),
        "le compteur en attente est peint, et dit qu'il compte : {avec:?}"
    );
}

/// Tout ce qui écrit le texte de l'éditeur le dit.
///
/// Plusieurs relevés coûteux sont gardés d'une image à l'autre et ne
/// valent que pour un texte donné : la place du curseur, les occurrences
/// d'une recherche, les instructions que la fenêtre de désassemblage
/// retient. Leur clé porte la RÉVISION du texte (voir `texte_a_changé`),
/// et une écriture qui ne la ferait pas avancer les laisserait peindre
/// ce qui n'est plus là.
///
/// Le listing tenait ce rôle avant : il était mis en page d'un bloc, et
/// le nombre de poses disait quand le texte avait bougé. La vue qui ne
/// pose que les lignes visibles le lui retire — d'où ce contrôle, qui
/// prend la relève de ce qu'egui garantissait tout seul.
#[test]
fn tout_ce_qui_ecrit_le_texte_le_dit() {
    // Les trois portes, et elles seules : les deux méthodes qui
    // incrémentent la révision, et le champ de texte d'egui, qui écrit
    // dans le tampon lui-même et le signale par `response.changed()`.
    const PORTES: &[&str] = &[
        "        self.source = texte;",
        "        &mut self.source",
        "                        egui::TextEdit::multiline(&mut self.source)",
    ];
    // Ce qui compte comme une écriture : l'affectation, et les méthodes
    // de `String` qui changent son contenu.
    let écritures = [
        "self.source =",
        "self.source.insert",
        "self.source.push",
        "self.source.replace_range",
        "self.source.clear",
        "self.source.truncate",
        "self.source.retain",
        "self.source.remove",
        "&mut self.source",
    ];
    let mut fautives = Vec::new();
    for (nom, source) in SOURCES_DE_L_INTERFACE {
        for (rang, ligne) in source.lines().enumerate() {
            if !écritures.iter().any(|é| ligne.contains(é)) {
                continue;
            }
            if PORTES.contains(&ligne) {
                continue;
            }
            fautives.push(format!("{nom}:{} — {}", rang + 1, ligne.trim()));
        }
    }
    assert!(
        fautives.is_empty(),
        "{} écriture(s) du texte hors des portes connues : la révision \
         n'avancera pas, et les relevés gardés peindront ce qui n'est \
         plus là. Passer par `set_source` ou `source_mut` :\n  {}",
        fautives.len(),
        fautives.join("\n  ")
    );
}

/// Les deux vues posent le texte AU MÊME ENDROIT.
///
/// C'est l'invariant qui rend la vue ligne à ligne acceptable : on passe
/// de l'une à l'autre en entrant en Insertion, et un décalage d'un pixel
/// ferait sauter tout le code sous les yeux à chaque `i`. La géométrie
/// calculée doit donc tomber là où egui pose ses rangées — ce qui n'allait
/// pas de soi : la hauteur de police dit 15,125 là où les rangées se
/// suivent de 15,0, et les abscisses de glyphes sont calées au pixel
/// (`x_offset(1)` vaut 7 quand la largeur d'un caractère vaut 7,83).
///
/// D'où le partage : le PAS se calcule, les ABSCISSES se lisent dans le
/// galley de la ligne.
#[test]
fn both_views_lay_the_text_in_the_same_place() {
    let source: String = (1..=12)
        .map(|n| format!("    mov rax, {n}        ; ligne {n}\n"))
        .collect();
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    // Ce que chaque ligne de code occupe à l'écran, quelle que soit la
    // vue : la vue ligne à ligne peint une galère par ligne, le champ de
    // texte une seule pour tout.
    let posées = |app: &mut NamsdecApp| -> Vec<(String, egui::Rect)> {
        let mut vues = Vec::new();
        for _ in 0..3 {
            vues.clear();
            let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                let egui::epaint::Shape::Text(texte) = &clipped.shape else {
                    continue;
                };
                for rangée in &texte.galley.rows {
                    let contenu = rangée.row.text().to_owned();
                    if !contenu.contains("mov rax") {
                        continue;
                    }
                    vues.push((contenu, rangée.rect().translate(texte.pos.to_vec2())));
                }
            }
            sortie.textures_delta.clear();
        }
        vues.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        vues
    };

    app.editor_mode = EditorMode::Normal;
    let ligne_à_ligne = posées(&mut app);
    app.editor_mode = EditorMode::Insert;
    let champ = posées(&mut app);

    assert!(
        ligne_à_ligne.len() >= 10,
        "la vue ligne à ligne peint les lignes : {}",
        ligne_à_ligne.len()
    );
    assert_eq!(
        ligne_à_ligne.len(),
        champ.len(),
        "les deux vues peignent le même nombre de lignes"
    );
    for ((texte_a, a), (texte_b, b)) in ligne_à_ligne.iter().zip(&champ) {
        assert_eq!(texte_a, texte_b, "les mêmes lignes, dans le même ordre");
        // Un demi-pixel : egui cale ses rangées sur le pixel, et les deux
        // vues doivent tomber sur le même.
        assert!(
            (a.left() - b.left()).abs() < 0.5 && (a.top() - b.top()).abs() < 0.5,
            "« {texte_a} » est posée en {a:?} par la vue ligne à ligne et en \
             {b:?} par le champ de texte"
        );
    }
}
