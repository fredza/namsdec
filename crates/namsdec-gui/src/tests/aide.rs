//! Le manuel, les fiches et les mots de l'interface.
//!
//! Ce que namsdec *dit* : les fiches d'instruction et de directive, le
//! manuel et sa recherche, le survol qui explique, le guide et les deux
//! langues. Ce qui vérifie que tout ce qui s'exécute a une fiche est ici
//! aussi : c'est une promesse de documentation, non de moteur.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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

/// Le sommaire des instructions montre TOUS les mnémoniques, pas une
/// tête de famille par fiche.
///
/// Le sommaire n'affichait que `names[0]` de chaque fiche : `jmp` se
/// voyait, mais `je`, `jne`, `jz`… — pourtant documentés par la même
/// famille — restaient invisibles tant qu'on ne savait pas les chercher.
/// Cent treize têtes de famille pour six cent soixante-quatorze
/// mnémoniques : cinq cent soixante et un instructions nasm présentes
/// dans la doc mais absentes de l'écran. « Toutes les instructions nasm
/// doivent être présentes », et ce contrôle l'exige nom par nom.
#[test]
fn the_instruction_index_shows_every_mnemonic_not_just_the_family_head() {
    let mut app = NamsdecApp {
        show_help: true,
        help_index_kind: namsdec_doc::Kind::Instruction,
        ..Default::default()
    };
    app.help = None;
    let painted = paint(&mut app);
    let vus: std::collections::BTreeSet<&str> = painted.split('|').collect();

    // La fenêtre d'aide ne montre qu'une page à la fois : on ne peut pas
    // exiger les six cent soixante-quatorze noms d'un coup. Mais parmi les
    // familles VISIBLES, au moins une doit montrer TOUS ses noms — avant,
    // le sommaire n'en montrait qu'un par fiche, donc aucune famille à
    // plusieurs noms n'aurait pu être complète.
    let mut une_famille_complète = false;
    let mut une_famille_multi_vue = false;
    for entry in namsdec_doc::all() {
        if entry.kind != namsdec_doc::Kind::Instruction || entry.names.len() < 2 {
            continue;
        }
        // Visible = son premier nom est peint.
        if !vus.contains(entry.names[0]) {
            continue;
        }
        une_famille_multi_vue = true;
        if entry.names.iter().all(|nom| vus.contains(nom)) {
            une_famille_complète = true;
            break;
        }
    }
    assert!(
        une_famille_multi_vue,
        "aucune famille à plusieurs noms n'est visible : test non concluant\n{painted}"
    );
    assert!(
        une_famille_complète,
        "une famille visible ne montre pas tous ses noms — le sommaire tait \
         encore les mnémoniques qui ne sont pas en tête de fiche\n{painted}"
    );

    // Et le compte annoncé est celui des mnémoniques, non des fiches : il
    // dit « toutes présentes », et il est en tête, jamais masqué.
    let combien = namsdec_doc::mnemonics().len();
    assert!(
        painted.contains(&format!("{combien} ")),
        "le sommaire doit annoncer les {combien} mnémoniques :\n{painted}"
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

/// L'exemple de chaque fiche d'instruction s'assemble, et namsdec
/// l'exécute.
///
/// Sans compte écrit ici : il y en a un par fiche, la table en porte
/// cent quatre-vingt-neuf, et un nombre recopié dans un commentaire ne
/// suit pas le code qui le dément — c'est la leçon que ce fichier a
/// déjà payée deux fois.
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
        \x20   texte db \"abc\", 0\n\
        \x20   ancien dw 0\n    vers_zero dw 0x0f7f\n\
        \x20   env times 28 db 0\n    rapide dd 0x9fc0\n\
        \x20   align 16\n    zone times 512 db 0\n\nsection .text\n_start:\n";
    let epilogue = "\n    hlt\nplus_grand:\nfin:\nnul:\nfini:\nidentiques:\n\
        non_comparable:\nétait_déjà_posé:\npas_de_transaction:\n    hlt\n";

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
                // Chaque ligne est peinte pour elle-même (voir
                // `vue_listing`) : on vise celle qui porte `movzx`, et
                // non la quatrième rangée d'une galère unique.
                let (pos, galley) = texts
                    .iter()
                    .rfind(|(_, g)| g.text().contains("movzx"))
                    .expect("la ligne du movzx doit être dessinée");
                let colonne = galley.text().find("movzx").unwrap_or(0) as f32;
                let largeur = galley.rect.width() / galley.text().chars().count().max(1) as f32;
                pointer = Some(egui::pos2(
                    pos.x + (colonne + 2.0) * largeur,
                    pos.y + galley.rect.center().y,
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

/// Aucune phrase affichée ne porte l'indentation du source.
///
/// UN DÉFAUT QUI NE SE VOIT QUE SUR L'IMAGE, et qui revient à chaque
/// fois qu'une longue chaîne est repliée dans le code sans `\` de
/// continuation : les espaces du retrait entrent DANS le texte. À
/// l'écran, la phrase s'ouvre d'un trou de vingt-six espaces, ou se
/// replie en deux blocs qu'on lit dans le désordre — c'est ce que
/// montrait l'éditeur binaire le 28 octobre 2026, et quatorze phrases
/// d'`upx_module` avaient le même.
///
/// LA RÈGLE DISTINGUE LA PROSE DE L'ALIGNEMENT, et la longueur ne
/// suffit pas à les séparer : le retrait incrusté fait de dix à
/// vingt-six espaces, l'alignement volontaire de dix à seize. Écrit
/// sur la seule longueur, ce contrôle accusait trois colonnes
/// parfaitement légitimes.
///
/// Ce qui les sépare est ce qu'il y a de part et d'autre. Des espaces
/// qui mènent à un `;` alignent un commentaire d'assembleur sur sa
/// colonne. Des espaces qui suivent un mot hexadécimal séparent les
/// colonnes d'un listing. Partout ailleurs, dix espaces entre deux
/// mots sont un accident du source.
#[test]
fn aucune_phrase_ne_porte_l_indentation_du_source() {
    let racine = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let mut coupables = Vec::new();
    let mut à_voir = vec![racine.join("crates")];
    while let Some(dossier) = à_voir.pop() {
        let Ok(entrées) = std::fs::read_dir(&dossier) else {
            continue;
        };
        for entrée in entrées.filter_map(Result::ok) {
            let chemin = entrée.path();
            if chemin.is_dir() {
                à_voir.push(chemin);
                continue;
            }
            if chemin.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let Ok(texte) = std::fs::read_to_string(&chemin) else {
                continue;
            };
            for (numéro, ligne) in texte.lines().enumerate() {
                // Hors d'un littéral, rien à dire ; et une chaîne qui
                // porte des retours à la ligne met en page quelque
                // chose — un listing, une bannière —, ce n'est pas de
                // la prose.
                if !ligne.contains('"') || ligne.contains("\\n") {
                    continue;
                }
                let octets: Vec<char> = ligne.chars().collect();
                let mut début = None;
                for (index, &c) in octets.iter().enumerate() {
                    if c == ' ' {
                        début.get_or_insert(index);
                    } else {
                        // Le mot qui précède : hexadécimal, ce sont
                        // les colonnes d'un listing.
                        let avant: String = début
                            .map(|d| {
                                octets[..d]
                                    .iter()
                                    .rev()
                                    .take_while(|c| !c.is_whitespace() && **c != '"')
                                    .collect::<Vec<_>>()
                                    .into_iter()
                                    .rev()
                                    .collect()
                            })
                            .unwrap_or_default();
                        let colonne_de_données =
                            avant.len() >= 4 && avant.chars().all(|c| c.is_ascii_hexdigit());
                        if let Some(d) = début.take()
                            && index - d >= 10
                            && d > 0
                            && !matches!(octets[d - 1], ' ' | '"')
                            && c != '"'
                            // Un commentaire d'assembleur posé sur sa
                            // colonne : les espaces y sont le propos.
                            && c != ';'
                            && !colonne_de_données
                        {
                            coupables.push(format!(
                                "{}:{} — {} espaces",
                                chemin.strip_prefix(racine).unwrap_or(&chemin).display(),
                                numéro + 1,
                                index - d
                            ));
                        }
                        début = None;
                    }
                }
            }
        }
    }

    // ET LE MÊME DÉFAUT SOUS SON AUTRE FORME : une chaîne écrite sur
    // plusieurs lignes du SOURCE, sans le `\` qui dit à Rust de manger
    // le retour à la ligne et le retrait qui suit. La phrase emporte
    // alors un saut de ligne ET vingt-cinq espaces. Deux infobulles de
    // la barre de statut en étaient là, et le contrôle ci-dessus ne les
    // voyait pas : il regarde une ligne à la fois, et le retrait y est
    // en TÊTE, là où il est ordinaire.
    //
    // La signature est nette : une ligne très en retrait, qui est de la
    // prose, qui ne porte qu'UN guillemet — celui qui ferme —, et dont
    // la précédente ne finit pas par une continuation.
    let mut suites = Vec::new();
    let mut à_voir = vec![racine.join("crates")];
    while let Some(dossier) = à_voir.pop() {
        let Ok(entrées) = std::fs::read_dir(&dossier) else {
            continue;
        };
        for entrée in entrées.filter_map(Result::ok) {
            let chemin = entrée.path();
            if chemin.is_dir() {
                à_voir.push(chemin);
                continue;
            }
            if chemin.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let Ok(texte) = std::fs::read_to_string(&chemin) else {
                continue;
            };
            let lignes: Vec<&str> = texte.lines().collect();
            for (numéro, ligne) in lignes.iter().enumerate() {
                if numéro == 0 {
                    continue;
                }
                let retrait = ligne.len() - ligne.trim_start().len();
                let corps = ligne.trim_end();
                if retrait < 14
                    || !corps.ends_with('"')
                    || corps.matches('"').count() != 1
                    || corps.split_whitespace().count() < 4
                {
                    continue;
                }
                // Le premier caractère est une lettre : du code finit
                // par une parenthèse, une accolade, une virgule.
                if !ligne
                    .trim_start()
                    .starts_with(|c: char| c.is_alphabetic() || c == '«' || c == '\u{201c}')
                {
                    continue;
                }
                if lignes[numéro - 1].trim_end().ends_with('\\') {
                    continue;
                }
                suites.push(format!(
                    "{}:{}",
                    chemin.strip_prefix(racine).unwrap_or(&chemin).display(),
                    numéro + 1
                ));
            }
        }
    }
    coupables.extend(suites);

    assert!(
        coupables.is_empty(),
        "ces phrases portent le retrait de leur source, et il se voit à l'écran : {coupables:#?}"
    );
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
        exercices, 138,
        "le parcours livré compte cent trente-huit exercices"
    );
    assert!(
        readme.contains("parcours de cent trente-huit exercices"),
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
        // La douzième, arrivée le 4 novembre 2026. Le tableau du README
        // l'oubliait, et rien ne le disait : ce contrôle vérifiait que
        // chaque crate ANNONCÉE existe, jamais que chaque crate du
        // dépôt est annoncée. Les deux sens comptent — une crate
        // absente du tableau est une crate que personne ne sait
        // chercher.
        "namsdec-cli",
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

    // ET LE SENS INVERSE, qui manquait : chaque crate du dépôt est dans
    // le tableau. Sans lui, la liste ci-dessus serait exacte le jour où
    // on l'écrit et muette sur la crate suivante — c'est exactement ce
    // qui est arrivé à `namsdec-cli`, née le matin même et absente du
    // README toute la journée.
    let crates = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../"));
    let mut oubliées = Vec::new();
    for entrée in std::fs::read_dir(crates).expect("le dossier des crates se lit") {
        let chemin = entrée.expect("une entrée lisible").path();
        if !chemin.join("Cargo.toml").is_file() {
            continue;
        }
        let nom = chemin
            .file_name()
            .expect("un nom de dossier")
            .to_string_lossy()
            .into_owned();
        if !readme.contains(&format!("`{nom}`")) {
            oubliées.push(nom);
        }
    }
    assert!(
        oubliées.is_empty(),
        "le tableau des crates du README n'annonce pas : {oubliées:?}"
    );
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
    // Ouvrir un fichier RÉPOND — « introuvable », ce qui est la vérité
    // — depuis le 28 octobre 2026, et le README ne dit plus le
    // contraire. Ce qui reste refusé est ce qui n'a pas de réponse
    // vraie : il n'y a ni second processus, ni réseau.
    assert!(
        namsdec_re::table::is_simulated(2) && namsdec_re::table::is_simulated(257),
        "ouvrir un fichier répond, et le README le dit"
    );
    for refusé in [59u64, 41, 42] {
        assert!(
            !namsdec_re::table::is_simulated(refusé),
            "l'appel {refusé} n'a pas de réponse vraie ici"
        );
    }
    for nom in ["getpid", "getrandom", "sched_yield", "arch_prctl", "mmap"] {
        assert!(readme.contains(nom), "le README annonce {nom}");
    }
}

/// LE SITE MONTRE DES IMAGES QUI EXISTENT, ET LE GUIDE DE CETTE
/// VERSION.
///
/// Un site est la seule partie du dépôt que personne ne compile : une
/// image renommée, un PDF qui change de numéro, et la page affiche un
/// cadre vide sans que rien ne tombe. Ce contrôle lui rend ce que le
/// reste du dépôt a — quelqu'un qui dit non.
///
/// Il ne juge pas la mise en page : il vérifie que ce que les pages
/// DÉSIGNENT est là.
#[test]
fn le_site_ne_montre_rien_qui_manque() {
    let docs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let guide = format!("guide-namsdec-{}.pdf", env!("CARGO_PKG_VERSION"));
    let mut vues = std::collections::BTreeSet::new();

    for (nom, autre) in [("index.html", "en.html"), ("en.html", "index.html")] {
        let page =
            std::fs::read_to_string(docs.join(nom)).unwrap_or_else(|_| panic!("docs/{nom} est là"));

        // Chaque image désignée existe vraiment.
        let mut reste = page.as_str();
        let mut images = 0;
        while let Some(i) = reste.find("src=\"images/") {
            reste = &reste[i + "src=\"".len()..];
            let fin = reste.find('"').expect("un attribut fermé");
            let fichier = &reste[..fin];
            assert!(
                docs.join(fichier).is_file(),
                "docs/{nom} montre « {fichier} », qui n'existe pas. \
                 Les captures se refont par \
                 `cargo test -p namsdec-gui -- --ignored releve_du_site`"
            );
            vues.insert(fichier.to_owned());
            images += 1;
            reste = &reste[fin..];
        }
        assert!(images >= 4, "docs/{nom} ne montre que {images} capture(s)");

        // Le guide est celui de la version qu'on compile.
        assert!(
            page.contains(&guide),
            "docs/{nom} ne mène pas à « {guide} »"
        );
        assert!(docs.join(&guide).is_file(), "le guide annoncé n'est pas là");

        // L'autre langue se trouve depuis celle-ci.
        assert!(page.contains(autre), "docs/{nom} ne mène pas à {autre}");

        // Et le site dit qui l'écrit, dans les deux langues.
        assert!(
            page.contains(APP_AUTHOR),
            "docs/{nom} ne nomme pas l'auteur"
        );

        // La feuille de style qu'elles partagent.
        assert!(page.contains("style.css"));
    }

    assert!(docs.join("style.css").is_file(), "docs/style.css est là");
    // `.nojekyll` : sans lui, GitHub Pages passe le dossier à Jekyll,
    // qui écarte ce qui commence par un souligné et transforme ce
    // qu'on n'avait pas demandé. Le site est du HTML servi tel quel.
    assert!(docs.join(".nojekyll").is_file(), "docs/.nojekyll est là");

    // ET LES DEUX PAGES MONTRENT LA MÊME CHOSE. Une page anglaise qui
    // perd la moitié des captures est la même dérive que la page
    // anglaise qui perd la moitié de son texte.
    for (nom, autre) in [("index.html", "en.html"), ("en.html", "index.html")] {
        let a = std::fs::read_to_string(docs.join(nom)).expect("page");
        let b = std::fs::read_to_string(docs.join(autre)).expect("page");
        for image in &vues {
            assert_eq!(
                a.contains(image.as_str()),
                b.contains(image.as_str()),
                "« {image} » n'est que dans une des deux pages"
            );
        }
    }
}

/// LA PAGE ANGLAISE DIT LA MÊME CHOSE QUE LA FRANÇAISE.
///
/// Le dépôt est public, et la moitié de ceux qui tomberont dessus ne
/// lisent pas le français. Une page anglaise qui dérive en résumé est
/// pire qu'une absence : elle promet une traduction et donne un
/// dépliant. C'est arrivé dès le premier jour — la moitié anglaise
/// faisait 495 lignes contre 1 007, et personne ne l'aurait vu sans
/// qu'on le dise.
///
/// Ce contrôle ne compare pas des phrases : elles sont dans deux
/// langues. Il compare ce qui se vérifie — les nombres, les noms, les
/// liens — et la TAILLE, qui est le seul signe qu'une page se soit
/// vidée de sa substance.
#[test]
fn les_deux_pages_du_readme_disent_la_meme_chose() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fr = std::fs::read_to_string(racine.join("README.md")).expect("README.md");
    let en = std::fs::read_to_string(racine.join("README.en.md")).expect("README.en.md");

    // Chacune mène à l'autre : une page qu'on ne trouve pas n'existe pas.
    assert!(
        fr.contains("README.en.md"),
        "le français ne mène pas à l'anglais"
    );
    assert!(
        en.contains("(README.md)"),
        "l'anglais ne ramène pas au français"
    );

    // Les nombres qu'on peut vérifier, des deux côtés.
    let fiches = namsdec_doc::count();
    assert!(
        en.contains(&format!("{fiches} instructions")),
        "la page anglaise annonce un autre nombre de fiches que les {fiches} existantes"
    );
    let guide = format!("docs/guide-namsdec-{}.pdf", env!("CARGO_PKG_VERSION"));
    for (nom, page) in [("français", &fr), ("anglais", &en)] {
        assert!(
            page.contains(&guide),
            "le README {nom} n'annonce pas « {guide} »"
        );
    }

    // Le tableau des crates, des deux côtés : c'est par là qu'on entre
    // dans le code, et une page qui l'oublie laisse son lecteur dehors.
    let crates = racine.join("crates");
    for entrée in std::fs::read_dir(&crates).expect("le dossier des crates se lit") {
        let chemin = entrée.expect("une entrée lisible").path();
        if !chemin.join("Cargo.toml").is_file() {
            continue;
        }
        let nom = chemin
            .file_name()
            .expect("un nom")
            .to_string_lossy()
            .into_owned();
        assert!(
            en.contains(&format!("`{nom}`")),
            "le tableau des crates de la page anglaise oublie {nom}"
        );
    }

    // ET ELLE N'EST PAS UN RÉSUMÉ. Les deux langues ne font pas le même
    // nombre de lignes — l'anglais est plus court à dire —, mais un
    // écart de plus d'un quart n'est plus une différence de langue,
    // c'est une section qui manque.
    let (l_fr, l_en) = (fr.lines().count(), en.lines().count());
    assert!(
        l_en * 4 >= l_fr * 3,
        "la page anglaise fait {l_en} lignes contre {l_fr} : il y manque des sections, \
         ce n'est pas une différence de langue"
    );
}

/// LE NOM DE L'AUTEUR SE LIT DANS LA BOÎTE, ET C'EST CELUI DE LA
/// LICENCE.
///
/// Un nom écrit deux fois finit par ne plus dire la même chose — c'est
/// exactement ce qui était arrivé à l'adresse de don, en dur dans le
/// binaire et pointant vers un domaine de documentation. Ici la source
/// est la LICENCE : c'est elle qui engage, l'interface ne fait que la
/// citer.
#[test]
fn la_boite_a_propos_nomme_l_auteur_de_la_licence() {
    let licence = include_str!("../../../../LICENSE");
    assert!(
        licence.contains(APP_AUTHOR),
        "le nom affiché « {APP_AUTHOR} » n'est pas celui de la LICENCE"
    );
    // Et il est peint, dans les deux langues : une boîte « À propos »
    // qui ne nomme personne en anglais ne nomme personne.
    for lang in [Lang::Fr, Lang::En] {
        let mut app = NamsdecApp {
            show_about: true,
            lang,
            ..Default::default()
        };
        let painted = paint(&mut app);
        assert!(
            painted.contains(APP_AUTHOR),
            "« À propos » ne nomme pas l'auteur en {lang:?} : {painted}"
        );
        assert!(
            painted.contains(strings(lang).about_author),
            "la phrase qui l'introduit manque en {lang:?}"
        );
    }
}

/// LES PRÉFÉRENCES MÈNENT AUX VERSIONS, ET NON À UNE PAGE DE PAIEMENT.
///
/// Le champ qui s'y trouvait servait à qui PUBLIE namsdec — une
/// personne — et occupait une place que cherche quiconque ouvre les
/// préférences : où sont les versions. Le réglage reste lu dans
/// `settings.json` pour qui en avait posé un, il ne s'offre plus ici.
#[test]
fn les_preferences_menent_aux_versions_publiees() {
    let source = include_str!("../preferences.rs");
    assert!(
        !source.contains("self.donation_url"),
        "les préférences proposent encore l'adresse de don"
    );
    assert!(
        source.contains("releases_url"),
        "les préférences ne mènent pas aux versions publiées"
    );
    // Et le lien mène au dépôt de namsdec, non à une page inventée.
    let url = namsdec_update::releases_url(namsdec_update::OWNER, namsdec_update::REPO);
    assert!(url.contains("namsdec"), "{url}");
    assert!(url.starts_with("https://"), "{url}");

    // ET PLUS AUCUN TEXTE N'Y ENVOIE. La boîte « À propos » disait
    // encore « posez la vôtre dans Préférences › Général » : un mode
    // d'emploi vers un endroit qui ne l'offre plus est pire qu'un
    // silence, parce qu'on le suit.
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        assert!(
            !s.about_donate_unset.contains("Préférences")
                && !s.about_donate_unset.contains("Preferences"),
            "le message du don renvoie encore aux préférences en {lang:?}"
        );
        assert!(
            s.about_donate_unset.contains("settings.json"),
            "il faut bien dire où l'adresse se pose, en {lang:?}"
        );
    }
}

/// CE QU'UNE AUTRE DISTRIBUTION EMPLOIE, ET QUE CELLE-CI N'EMPLOIE PAS.
///
/// `what_this_machine_runs_can_be_explained` lit les binaires de la
/// machine où il tourne : il ne voit donc que ce que cette
/// distribution-là a compilé. Sur Fedora il passait au vert pendant que
/// la CI — Ubuntu 24.04 — nommait cinq instructions sans fiche :
/// `bnd`, `crc32`, `pclmullqlqdq`, `rdrand` et `rdseed`. Un contrôle
/// qui ne regarde qu'une machine ne peut pas dire ce qui manque pour
/// les autres.
///
/// Celui-ci les retient PAR LEUR NOM. Il ne dépend d'aucun binaire,
/// donc il dit la même chose partout — et la prochaine liste trouvée
/// ailleurs vient s'y ajouter au lieu de se reperdre entre deux
/// journaux de CI.
#[test]
fn ce_qu_une_autre_distribution_emploie_a_sa_fiche() {
    // Relevées sur Ubuntu 24.04 le 26 novembre 2026, dans soixante
    // binaires de `/usr/bin`, avec le nombre de fois qu'elles y
    // revenaient.
    let ailleurs = [
        ("bnd", 39),
        ("crc32", 32),
        ("pclmullqlqdq", 4),
        ("rdrand", 3),
        ("rdseed", 12),
    ];
    let sans_fiche: Vec<&str> = ailleurs
        .iter()
        .filter(|(nom, _)| namsdec_doc::lookup(nom).is_none())
        .map(|(nom, _)| *nom)
        .collect();
    assert!(
        sans_fiche.is_empty(),
        "ces instructions reviennent dans les binaires d'une distribution \
         courante sans pouvoir s'expliquer : {sans_fiche:?}"
    );
    // ET LA FICHE DIT CE QU'ON EN FAIT. Une fiche qui décrit une
    // instruction sans dire si namsdec l'exécute promet une exécution
    // qui n'aura pas lieu — c'est la règle que
    // `every_documented_example_assembles_and_runs` applique déjà, et
    // ces cinq-là sont précisément celles qu'on lit sans les écrire.
    for (nom, _) in ailleurs {
        let fiche = namsdec_doc::lookup(nom).expect("la fiche existe");
        assert!(
            fiche.body.fr.contains("namsdec"),
            "la fiche de {nom} ne dit pas ce que namsdec en fait"
        );
        assert!(
            fiche.body.en.contains("namsdec"),
            "the {nom} card does not say what namsdec does with it"
        );
    }
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

/// Le journal des versions parle bien de la version qu'on compile.
///
/// Un journal en retard d'une version est pire que pas de journal : il
/// dit que rien n'a changé alors que tout a bougé.
#[test]
fn the_changelog_covers_the_version_being_built() {
    let changelog = include_str!("../../../../CHANGELOG.md");
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
    let readme = include_str!("../../../../README.md");
    assert!(readme.contains("CHANGELOG.md"));
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
    const LES_MÊMES: [&str; 33] = [
        // « Arguments » s'écrit pareil des deux côtés de la Manche, et
        // c'est le mot qu'emploient tous les outils.
        "console_args_toggle",
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
        "bug_absent",
        "re_bref_protections",
        "shortcuts_action_col",
        "abi_arguments",
        "about_build_word",
        "dialog_ok",
        "bytes_instructions",
        "disassembly_nasm",
        "disassembly_pseudo",
        "filter_markdown",
        // « JSON » est un sigle : il ne se traduit nulle part.
        "filter_json",
        "preferences_tab_modules",
        "preferences_tab_plugins",
        "tab_diagnostics",
        "tab_output",
        "tab_re",
        // L'onglet de l'éditeur : « Source » s'écrit pareil dans les
        // deux langues.
        "view_source",
        "view_console",
        "x87_empty",
        // Le mot est le même des deux côtés de la Manche.
        "calculator_base_label",
        "reading_octal",
        "reading_ascii",
        // Une expression de calcul : `0xff & 0x0f` ne se traduit pas.
        "calculator_hint_text",
    ];

    let source = include_str!("../i18n.rs");
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

/// Le guide de l'utilisateur dit la vérité sur la version qu'il décrit.
///
/// Un guide qui annonce une autre version que celle du programme, ou
/// quarante-sept exercices quand il y en a cent huit, est pire
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
    //
    // `modules/` est désormais SEUL à porter les outils : il y en avait
    // deux, et c'est le dossier qui disait ce qu'une fiche rendait. La
    // fiche le dit elle-même, par son champ `kind`.
    for dossier in ["modules/", "themes/", "exercises/"] {
        assert!(
            sommaire.contains(dossier),
            "le mode d'emploi doit nommer « {dossier} » : c'est là que namsdec regarde"
        );
    }
    // Et le code lit bien celui-là, sous ce nom.
    let data = data_dir().expect("un dossier de données");
    assert_eq!(
        crate::tools::decompilers_dir().expect("modules"),
        data.join("modules")
    );
    // Les deux noms d'avant restent cités : ce qu'on y avait posé est
    // recopié vers `modules/`, et le mode d'emploi doit dire où
    // retrouver ce qui ne l'a pas été.
    for ancien in ["disassemblers/", "decompilers/"] {
        assert!(
            sommaire.contains(ancien),
            "le mode d'emploi doit encore nommer « {ancien} » : c'est là que les \
             fiches d'avant sont reprises"
        );
    }
}

/// Le manuel nomme chaque panneau d'analyse comme l'écran le nomme.
///
/// Le lecteur cherche dans l'interface le nom qu'il vient de lire. Le
/// manuel écrivait « Gadgets », « Fonctions empruntées », « Chaînes »,
/// « Figures », « Protections », « Place libre » — l'écran affiche
/// « Fragments réutilisables (ROP) », « Fonctions importées », « Chaînes
/// lisibles », « Figures du code », « Protections du binaire », « Place
/// libre (bourrage) ». Six panneaux introuvables sous le nom annoncé.
///
/// Le contrôle lit les titres dans le registre lui-même : un module
/// nouveau, ou renommé, y entre sans qu'on pense à mettre ce test à jour.
#[test]
fn le_manuel_nomme_les_panneaux_comme_l_interface() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // `docs/guide.py` est DANS le dépôt : son absence n'est pas un état
    // de la machine dont il faudrait s'accommoder, c'est le dépôt qui a
    // bougé. S'abstenir alors, c'est se taire au seul moment où ce
    // contrôle aurait quelque chose à dire.
    let guide = std::fs::read_to_string(racine.join("docs/guide.py"))
        .expect("docs/guide.py — le manuel est versionné avec le code");
    let mut app = NamsdecApp::default();
    let mut manquants: Vec<&'static str> = Vec::new();
    for module in app.re_registry.iter_mut() {
        let titre = module.title("fr");
        if !guide.contains(titre) {
            manquants.push(titre);
        }
    }
    assert!(
        manquants.is_empty(),
        "{} panneau(x) que le manuel nomme autrement que l'écran : {manquants:?}\n\n\
         Le lecteur cherche dans l'interface le nom qu'il vient de lire. Le manuel \
         disait « Gadgets », « Fonctions empruntées », « Chaînes » — l'écran affiche \
         « Fragments réutilisables (ROP) », « Fonctions importées », « Chaînes lisibles ». \
         Employez le titre exact du module dans docs/guide.py.",
        manquants.len()
    );
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

/// Les rubriques d'une même famille se suivent, et les familles sont
/// dans l'ordre où l'on lit un binaire.
///
/// La colonne pose un intertitre quand la famille change. Un module
/// rangé hors de son groupe ferait donc écrire deux fois le même
/// intertitre — « Le code », puis trois autres familles, puis « Le
/// code » encore —, ce qui est pire qu'une liste à plat : on croirait
/// à deux choses différentes.
///
/// Le contrôle tient aussi l'ordre des familles, qui n'est pas une
/// préférence de rangement : c'est celui dans lequel on lit un binaire
/// inconnu — ce que le fichier dit de lui-même, son code, ce qu'on peut
/// en rendre lisible, ce qu'il dit, ce qu'il demande, puis ses octets.
#[test]
fn les_rubriques_d_une_meme_famille_se_suivent() {
    use namsdec_re::ReSection;

    let app = NamsdecApp::default();
    let entrées = app.re_registry.nav_entries("fr");
    assert!(entrées.len() > 15, "le registre est peuplé : {entrées:?}");

    let mut familles: Vec<ReSection> = Vec::new();
    for (id, _, section) in &entrées {
        if familles.last() == Some(section) {
            continue;
        }
        assert!(
            !familles.contains(section),
            "« {id} » est rangé hors de sa famille « {} » : la colonne écrirait \
             deux fois cet intertitre",
            section.label("fr")
        );
        familles.push(*section);
    }
    let mut dans_l_ordre = familles.clone();
    dans_l_ordre.sort();
    assert_eq!(
        familles, dans_l_ordre,
        "les familles ne se suivent pas dans l'ordre déclaré par ReSection"
    );
    assert!(
        familles.len() >= 5,
        "la colonne compte bien des familles : {familles:?}"
    );
}
