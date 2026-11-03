//! La fenêtre d'analyse et les modules d'ingénierie inverse.
//!
//! Protections, chaînes, appels importés, empreinte, boucles, fragments,
//! carte et vidage, espace libre, provenance d'une valeur dans le code. Ce
//! qui se mesure sur un binaire posé, non sur un programme qui tourne —
//! celui-là est dans [`super::execution`].
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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
    assert_eq!(shown, ["Processeur", "Pile", "Mémoire", "Tas", "Octets"]);

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

/// LA FICHE DE L'ONGLET RE tient dans le panneau, sans rien couper.
///
/// L'onglet ne portait qu'un bouton — « Analyse du binaire… » — et deux
/// mille pixels de vide : tout ce qu'il savait vivait derrière une
/// fenêtre qu'il fallait ouvrir. Il porte maintenant la fiche
/// d'identité du fichier, et c'est précisément ce qui le met en danger :
/// un panneau étroit, des valeurs longues.
///
/// Première écriture, en grille à deux colonnes : la taille du fichier
/// coupée, « (GNU) 16.2.1 » tronqué, le canari hors du bord, et la
/// suite de la ligne du compilateur peinte PAR-DESSUS les protections.
/// Mesuré ici, au sens de l'invariant « rien n'est coupé » : l'encre
/// comparée à l'encre ∩ découpage, rangée par rangée.
#[test]
fn the_re_tab_card_fits_without_cutting_anything() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        next_instruction_open: false,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    if !app.load_elf(&octets, "ls") {
        return;
    }
    // L'onglet est posé APRÈS le chargement : ouvrir un binaire ramène
    // le panneau sur « Processeur », et le poser avant ne tenait pas —
    // le relevé mesurait alors l'état du processeur en croyant mesurer
    // la fiche.
    // Et l'onglet RE n'existe que si le module est activé : sans cela
    // le panneau retombe sur « Processeur », et le relevé mesurait
    // l'état du processeur en croyant mesurer la fiche.
    app.show_re_tab = true;
    app.active_tab = SidebarTab::Re;
    app.next_instruction_open = false;
    // Les deux largeurs qui comptent : celle d'un écran ordinaire, et
    // une plus étroite — c'est là que les valeurs longues débordent.
    for largeur in [1280.0, 1100.0] {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, 1000.0),
            )),
            ..Default::default()
        };
        let mut coupés: Vec<String> = Vec::new();
        let mut vu_la_fiche = false;
        for _ in 0..4 {
            coupés.clear();
            vu_la_fiche = false;
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                if t.galley.text().contains("Compilateur") || t.galley.text().contains("Langage") {
                    vu_la_fiche = true;
                }
                for rangée in t.galley.rows.iter() {
                    let texte = rangée.text();
                    if texte.trim().is_empty() {
                        continue;
                    }
                    let encre = rangée
                        .rect_without_leading_space()
                        .translate(t.pos.to_vec2());
                    if !encre.is_positive() {
                        continue;
                    }
                    let vu = encre.intersect(clipped.clip_rect);
                    if vu.width() + 1.0 < encre.width() {
                        coupés.push(format!(
                            "« {} » : {:.0} px peints sur {:.0}",
                            texte.chars().take(40).collect::<String>(),
                            vu.width(),
                            encre.width()
                        ));
                    }
                }
            }
            output.textures_delta.clear();
        }
        assert!(
            vu_la_fiche,
            "à {largeur}px, la fiche n'est pas peinte — le relevé ne mesure rien"
        );
        assert!(
            coupés.is_empty(),
            "à {largeur}px, des textes sont coupés : {coupés:#?}"
        );
    }
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

/// Le rapport n'est plus partiel, et il s'écrit aussi en JSON.
///
/// **C'EST L'ÉTAPE QUI JUSTIFIE P7.** Le rapport écrit à la main
/// couvrait six rubriques sur vingt-deux : ni l'entropie, ni les
/// chaînes, ni les appels système, ni les fragments ROP, ni le graphe
/// d'appels n'en sortaient — ce que namsdec en comprenait n'existait
/// que PEINT. Il porte maintenant ce que chaque panneau montre, tiré
/// des mêmes vues que l'écran : une source unique, donc un rapport qui
/// ne peut plus prendre du retard sur ce qu'on voit.
///
/// Et le même fond en JSON, pour un autre programme. L'extension
/// choisie décide — un format qu'on demande en nommant son fichier,
/// plutôt qu'un bouton de plus dans une barre qui en porte déjà trois.
#[test]
fn le_rapport_porte_ce_que_les_panneaux_montrent() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes, "programme");

    let md = app.analysis_report().expect("un binaire est chargé");
    // La prose soignée reste en tête : elle raconte, là où les
    // panneaux énumèrent.
    assert!(md.starts_with("# Analyse du binaire"));
    // Et ce qu'elle ne disait pas y est maintenant. Trois rubriques
    // choisies parce qu'AUCUNE n'était dans le rapport d'avant.
    for panneau in ["## Entropie", "## Chaînes lisibles", "## Appels système"] {
        assert!(md.contains(panneau), "le rapport porte « {panneau} »");
    }
    // Le profil d'entropie se DIT, faute de se dessiner : un rapport
    // ne peint pas, et il ne fait pas semblant.
    assert!(
        md.contains("Profil d'entropie"),
        "ce qui se dessinait se dit : {}",
        &md[..md.len().min(200)]
    );

    // LES PANNEAUX QUI PARLENT DE LA MACHINE SONT ÉCARTÉS : aucun
    // outil extérieur n'a tourné, et « radare2 n'est pas installé
    // ici » n'apprend rien à qui lira le rapport ailleurs.
    assert!(
        !md.contains("Décompilation par radare2"),
        "un outil qui n'a rien rendu n'a pas de rubrique"
    );

    // LE MÊME FOND, EN JSON, et c'est l'extension qui décide.
    let dir = tempfile::tempdir().unwrap();
    let json = dir.path().join("analyse.json");
    app.export_report(&json);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    let texte = std::fs::read_to_string(&json).unwrap();
    let relu: serde_json::Value = serde_json::from_str(&texte).expect("du JSON valable");
    let panneaux = relu.as_array().expect("une liste de panneaux");
    assert!(panneaux.len() >= 8, "{} panneaux", panneaux.len());
    assert!(
        panneaux
            .iter()
            .any(|p| p["id"] == "entropy" && !p["vue"]["blocs"].as_array().unwrap().is_empty()),
        "l'entropie y est, avec ses blocs"
    );

    // Et le Markdown reste le défaut : c'est l'extension qui change
    // tout, et rien d'autre.
    let md_path = dir.path().join("analyse.md");
    app.export_report(&md_path);
    assert!(
        std::fs::read_to_string(&md_path)
            .unwrap()
            .starts_with("# Analyse du binaire")
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
    // Le style de l'application avant toute mesure : ce test lit
    // désormais la position d'un en-tête pour aller cliquer dessus, et
    // une galley mesurée sous le style d'egui n'est pas la nôtre.
    app.apply_theme(&ctx);
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
    //
    // CE CONTRÔLE PASSAIT POUR LA MAUVAISE RAISON. Il cherchait chaque
    // titre dans ce qui est peint, et la ligne de titres séparés par des
    // virgules — celle qu'il dit refuser — les contenait tous. Il
    // n'aurait rougi que le jour où l'on aurait retiré les deux.
    //
    // La carte ne donne plus que le compte ; les titres sont dans le
    // repli, qui est fermé. On l'OUVRE donc, d'un clic à l'endroit où
    // son en-tête est peint, et l'on regarde ce qu'il y a dedans. Ce
    // que le contrôle vérifie est désormais ce qu'il annonce.
    let s = strings(app.lang);
    assert!(peint.contains(s.modules_re_detail), "{peint}");
    assert!(
        !peint.contains(&app.re_registry.titles(app.lang.code()).join(", ")),
        "les titres ne doivent pas être entassés en une ligne de virgules :\n{peint}"
    );

    let en_tête = {
        let mut trouvé = None;
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.modules_ui(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape
                && texte.galley.text().starts_with(s.modules_re_detail)
            {
                trouvé = Some(egui::Rect::from_min_size(texte.pos, texte.galley.size()).center());
            }
        }
        sortie.textures_delta.clear();
        trouvé.expect("l'en-tête du repli des modules d'analyse est peint")
    };
    // Un clic : la pression et le relâchement au même endroit, dans la
    // même image, comme egui les reçoit d'une souris.
    let clic = egui::RawInput {
        events: vec![
            egui::Event::PointerMoved(en_tête),
            egui::Event::PointerButton {
                pos: en_tête,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos: en_tête,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ],
        ..input.clone()
    };
    let mut sortie = ctx.run_ui(clic, |ui| app.modules_ui(ui));
    sortie.textures_delta.clear();
    // Le repli s'ouvre par une animation : quelques images pour qu'il
    // ait sa hauteur entière.
    let mut déplié = String::new();
    for _ in 0..12 {
        déplié.clear();
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.modules_ui(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                déplié.push_str(texte.galley.text());
                déplié.push('|');
            }
        }
        sortie.textures_delta.clear();
    }
    for titre in app.re_registry.titles(app.lang.code()) {
        assert!(
            déplié.contains(titre),
            "le module d'analyse « {titre} » doit se lire une fois le repli \
             ouvert :\n{déplié}"
        );
    }

    // Et la section de ce qui vient de fichiers, même vide : son absence
    // et « aucun greffon » ne se disent pas de la même façon.
    assert!(peint.contains(s.modules_plugins), "{peint}");
}

/// La fenêtre d'analyse respire, et sa colonne s'aligne.
///
/// Sans marge, le texte se colle au cadre de la fenêtre et à la ligne
/// qui sépare les deux colonnes : un tableau donne alors l'impression
/// d'être coupé. Et treize libellés centrés n'ont plus de bord commun —
/// l'œil doit chercher chacun au lieu de descendre une liste.
#[test]
fn the_analysis_window_has_margins_and_a_left_aligned_column() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
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
    //
    // Les noms de COLONNE, et non les titres : « Décompilation par
    // RetDec » s'écrit « RetDec » dans deux cents pixels, et le
    // panneau garde son titre entier.
    let titres: Vec<&str> = app
        .re_registry
        .nav_entries("fr")
        .into_iter()
        .map(|(_, titre, _)| titre)
        .collect();
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

/// Dans l'onglet Mémoire, « Lire en : » ne se peint pas sur « Suivre : ».
///
/// Les deux réglages tenaient sur une seule rangée enroulée. Sur un
/// panneau étroit elle débordait, et `horizontal_wrapped` donnait à
/// l'élément reporté — « Lire en : » — une galère démesurée (257 px pour
/// neuf caractères) peinte à l'abscisse de départ, par-dessus « Suivre : »
/// et les boutons de suivi. Deux rangées franches règlent l'affaire ; ce
/// test vérifie qu'elles ne partagent plus la même ligne.
#[test]
fn lire_en_ne_se_peint_pas_sur_suivre() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_sidebar: true,
        active_tab: SidebarTab::Memory,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    for _ in 0..4 {
        app.do_step();
    }
    let scr = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(2000.0, 1200.0));
    let mut posés: Vec<(String, egui::Rect)> = Vec::new();
    // Le partage de la barre latérale se cale en une quinzaine d'images ;
    // on lit la dernière, posée.
    for _ in 0..18 {
        posés.clear();
        let mut sortie = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(scr),
                ..Default::default()
            },
            |ui| app.draw(ui),
        );
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
    let s = strings(Lang::Fr);
    let rect = |quoi: &str| -> egui::Rect {
        posés
            .iter()
            .find(|(t, _)| t == quoi)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("« {quoi} » est peint"))
    };
    let suivre = rect(s.memory_follow);
    let lire = rect(s.memory_format_label);
    assert!(
        !suivre.shrink(2.0).intersects(lire.shrink(2.0)),
        "« {} » et « {} » se recouvrent : {suivre:?} contre {lire:?}",
        s.memory_follow,
        s.memory_format_label
    );
}

/// La carte mémoire dépliée ne se peint pas sur le bandeau.
///
/// Un binaire aux régions nombreuses — les sections d'un programme lié
/// dynamiquement — dépliait une carte si haute que le contenu de la vue
/// Mémoire dépassait la place que le panneau latéral lui réserve. Le
/// partage divergeait alors : la vue remontait par-dessus ce qui la
/// précède, ou filait sous le bas de la fenêtre. La carte défile
/// désormais dans sa propre zone bornée, et la vue est dessinée dans un
/// enfant de taille fixe.
///
/// Le contrôle visait les registres, qui étaient alors ancrés entre le
/// bandeau et la vue ; ils ont leur propre onglet depuis le 13 octobre
/// 2026 — ce qui éteint cette famille de défauts à la racine. Il vise
/// maintenant ce qui partage encore le panneau avec la vue : le
/// bandeau.
#[test]
fn la_carte_depliee_ne_couvre_pas_le_bandeau() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_sidebar: true,
        active_tab: SidebarTab::Memory,
        next_instruction_open: false,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    for _ in 0..4 {
        app.do_step();
    }
    let scr = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(2000.0, 1183.0));
    let rend = |app: &mut NamsdecApp, ev: Vec<egui::Event>| -> Vec<(String, egui::Rect)> {
        let mut txts = Vec::new();
        let mut sortie = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(scr),
                events: ev,
                ..Default::default()
            },
            |ui| app.draw(ui),
        );
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                txts.push((
                    t.galley.text().to_owned(),
                    egui::Rect::from_min_size(t.pos, t.galley.size()),
                ));
            }
        }
        sortie.textures_delta.clear();
        txts
    };
    let clic = |app: &mut NamsdecApp, t: &[(String, egui::Rect)], début: &str| -> bool {
        let Some(r) = t
            .iter()
            .find(|(x, _)| x.starts_with(début))
            .map(|(_, r)| *r)
        else {
            return false;
        };
        let pos = r.center();
        rend(
            app,
            vec![
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        true
    };
    // Poser la barre, puis déplier « Outils mémoire » et « Carte de la
    // mémoire », en laissant chaque fois le partage se recaler.
    let mut t = rend(&mut app, vec![]);
    for _ in 0..5 {
        t = rend(&mut app, vec![]);
    }
    assert!(
        clic(&mut app, &t, s.memory_tools),
        "« Outils mémoire » est là"
    );
    for _ in 0..6 {
        t = rend(&mut app, vec![]);
    }
    assert!(
        clic(&mut app, &t, "Carte de la"),
        "« Carte de la mémoire » est là"
    );
    for _ in 0..8 {
        t = rend(&mut app, vec![]);
    }

    // La carte est bien dépliée : des lignes de région (« 0x…-0x… »)
    // sont peintes.
    let lignes_carte: Vec<egui::Rect> = t
        .iter()
        .filter(|(x, _)| x.starts_with("0x") && x.contains("-0x"))
        .map(|(_, r)| *r)
        .collect();
    assert!(
        !lignes_carte.is_empty(),
        "la carte devrait montrer des régions une fois dépliée"
    );

    // Rien de ce que le bandeau écrit ne partage sa place avec une
    // ligne de la carte.
    //
    // Les registres ne sont plus au-dessus : ils ont leur propre
    // onglet depuis le 13 octobre 2026, et c'est justement ce qui
    // éteint cette famille de défauts. Reste le bandeau — ce qui vient,
    // le dernier pas —, qui partage encore le panneau avec la vue.
    let bandeau: Vec<(String, egui::Rect)> = t
        .iter()
        .filter(|(x, r)| [s.next_instruction, s.last_step].contains(&x.trim()) && r.left() < 1700.0)
        .cloned()
        .collect();
    assert!(
        !bandeau.is_empty(),
        "le bandeau écrit quelque chose au-dessus de la vue"
    );
    for (nom, zone) in &bandeau {
        for ligne in &lignes_carte {
            assert!(
                !zone.shrink(2.0).intersects(ligne.shrink(2.0)),
                "la carte se peint sur « {nom} » : {zone:?} ∩ {ligne:?}"
            );
        }
    }
    use egui::containers::panel::PanelState;
    let tab = PanelState::load(&ctx, egui::Id::new("sidebar_tab"))
        .map(|z| z.outer_rect)
        .expect("l'onglet est un panneau");
    // Et l'onglet tient dans la fenêtre : sans la carte bornée, le
    // partage divergeait dans l'autre sens — l'onglet filait sous le bas
    // de l'écran (mesuré : bas à 1650 px pour une fenêtre de 1183).
    assert!(
        tab.bottom() <= 1183.0 + 2.0,
        "l'onglet déborde sous la fenêtre : tab={tab:?}"
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
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
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

/// Le panneau de la place libre montre le bourrage d'un vrai binaire.
///
/// C'est une analyse en lecture seule, dans la lignée du panneau des
/// fragments ROP : là où celui-ci montre qu'un binaire contient déjà du
/// code réutilisable, celui-ci montre qu'il contient de la place
/// inutilisée. Les deux apprennent qu'un exécutable n'est pas le bloc
/// figé qu'on croit.
///
/// Un ELF réel en a forcément : l'éditeur de liens aligne tout sur seize
/// octets, et laisse du bourrage partout. Le vérifier sur un binaire du
/// système plutôt que sur un tampon fabriqué, c'est éprouver que la
/// lecture des en-têtes tient sur un vrai fichier.
#[test]
fn the_free_space_panel_shows_a_real_binarys_padding() {
    // Un binaire du système fait l'affaire : tous en portent.
    let Ok(octets) = std::fs::read("/bin/true") else {
        return;
    };
    if namsdec_re::elf_scan::scan(&octets).is_none() {
        return; // pas un ELF 64 bits ici : rien à éprouver.
    }
    let trous = namsdec_re::slack::find(&octets);
    assert!(
        !trous.is_empty(),
        "un vrai binaire a forcément du bourrage d'alignement"
    );

    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "true"), "l'ELF se charge");
    app.show_analysis = true;

    // Le module existe, et sous son identifiant stable.
    let ids: Vec<&str> = app
        .re_registry
        .entries("fr")
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(ids.contains(&"slack"), "le module existe : {ids:?}");

    app.analysis_module = "slack".to_owned();
    let peint = paint(&mut app);
    // La plus grande zone, telle qu'elle est mesurée, doit se lire : le
    // panneau montre bien ce que l'analyse trouve, et non une valeur de
    // son cru.
    let (_, plus_grande) = namsdec_re::slack::totals(&trous);
    assert!(
        peint.contains(&format!("{plus_grande}")),
        "la plus grande zone ({plus_grande} octets) doit se lire :\n{peint}"
    );
    // Et le bourrage est nommé, pas seulement chiffré : c'est ce qui
    // apprend quelque chose.
    assert!(
        peint.contains("nop") || peint.contains("zéro") || peint.contains("int3"),
        "l'octet de bourrage est nommé :\n{peint}"
    );
    // La section qui porte une zone se lit : c'est elle qui raconte
    // l'histoire — une place en fin de `.text` n'est pas une place entre
    // deux entrées de table. Au moins une zone d'un vrai binaire tombe
    // dans une section nommée.
    if let Some(nom) = trous.iter().find_map(|t| t.section.as_deref()) {
        assert!(
            peint.contains(nom),
            "la section « {nom} » qui porte une zone doit se lire :\n{peint}"
        );
    }
}

/// Le rapport d'analyse porte l'empreinte du fichier.
///
/// C'est la première ligne d'un compte rendu : de quel octet parle-t-on ?
/// Deux personnes qui lisent « le crackme » n'ont pas forcément le même
/// fichier sous les yeux.
#[test]
fn the_analysis_gives_the_fingerprint_of_the_file() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");

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
    let Some(octets) = binaire_fabriqué() else {
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
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    app.show_analysis = true;

    // La colonne porte « Tout » et le nom de chaque module — le nom de
    // colonne, qui est plus court que le titre pour les outils
    // extérieurs.
    let titres: Vec<&str> = app
        .re_registry
        .nav_entries("fr")
        .into_iter()
        .map(|(_, titre, _)| titre)
        .collect();
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

/// Le vidage s'enregistre, au format que `diff` sait lire.
///
/// Un vidage sert à être comparé : à celui qu'on a sous les yeux, à
/// celui d'un autre outil, à la même analyse faite la semaine dernière.
#[test]
fn the_dump_can_be_saved_in_a_shape_one_can_compare() {
    let s = strings(Lang::Fr);
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
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

/// Le vidage laisse écrire, et ce qu'il écrit devient un patch qu'on
/// peut défaire.
///
/// Le vidage montrait ; il laisse maintenant changer, ce qui est la
/// suite naturelle : on a repéré la constante qu'on veut voir
/// autrement, et l'on n'avait aucun moyen de la poser.
#[test]
fn the_dump_can_write_bytes_and_the_change_is_a_patch() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    assert!(app.patches.is_empty());

    // Ce qu'un module demande, l'application le fait : c'est le chemin
    // que suit le bouton « Écrire » du vidage.
    let offset = 0x1000;
    let nouveaux = vec![0x90u8, 0x90, 0x90, 0x90];
    let avant = octets[offset..offset + nouveaux.len()].to_vec();
    assert_ne!(avant, nouveaux, "de quoi voir la différence");
    app.write_patch(offset, nouveaux.len(), nouveaux.clone(), 0);

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
    app.write_patch(offset, nouveaux.len(), nouveaux.clone(), 0);
    assert_eq!(app.patches.len(), 1, "et toujours un seul");

    // Et il se défait comme les autres.
    app.undo_patch(0);
    assert!(app.patches.is_empty());
    let rendu = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(&rendu[offset..offset + avant.len()], avant.as_slice());
}

/// L'onglet RE dit ce qu'il est AVANT d'avoir quelque chose à montrer.
///
/// Deux règles, et la seconde est venue d'une capture.
///
/// LA PREMIÈRE, ancienne : les sept modules empilés dessous faisaient
/// deux mille pixels à parcourir dans une colonne de trois cents. La
/// fenêtre d'analyse montre exactement les mêmes, en grand et tout
/// dépliés : c'est là qu'on les lit.
///
/// LA SECONDE : sans binaire, l'onglet ne montrait qu'un bouton GRIS et
/// pas un mot. C'était le seul onglet du panneau sans état vide — les
/// six autres disent ce qu'ils attendent et offrent le bouton qui y
/// mène. Et c'est l'onglet qui porte tout le travail de
/// rétro-ingénierie : celui qu'un nouveau venu ouvre par curiosité.
///
/// Aucune mesure ne l'avait demandé. Il a fallu regarder l'image.
#[test]
fn the_re_tab_explains_itself_before_it_has_anything_to_show() {
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        show_re_tab: true,
        active_tab: SidebarTab::Re,
        ..Default::default()
    };
    let peint = paint(&mut app);

    // Rien d'assemblé : l'onglet explique, et propose la porte.
    assert!(
        peint.contains(s.no_binary_assembled),
        "il doit dire ce qui manque :\n{peint}"
    );
    assert!(
        peint.contains(s.empty_re_hint),
        "et ce qu'il est — son détail à lui, celui qui parle d'un \
         binaire qu'on n'a pas écrit :\n{peint}"
    );
    assert!(
        peint.contains(s.action_assemble),
        "et la porte qui y mène :\n{peint}"
    );
    // Et PAS le bouton gris : un état vide qui porte sa propre action
    // n'a pas besoin d'une seconde commande éteinte sous elle.
    assert!(
        !peint.contains(s.analysis_open),
        "le bouton éteint n'a plus de raison d'être :\n{peint}"
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

/// Une écriture dans l'éditeur binaire rafraîchit l'analyse.
///
/// On avait cru à un défaut : `load_elf`, le chemin du patch, n'appelle
/// pas `on_binary_loaded`. Mais il appelle `rebuild_emulator`, qui
/// dispatche vers `load_process` ou `rebuild_from_assembly` — et ces
/// deux-là, eux, notifient les modules. L'analyse se refait donc bien
/// après chaque patch. Ce contrôle le prouve, et tombe si l'on casse un
/// jour cette chaîne : il écrit un octet et vérifie que l'empreinte
/// SHA-256, calculée au chargement, a changé.
#[test]
fn une_ecriture_rafraichit_l_analyse() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source:
            "global _start\nsection .text\n_start:\n    mov rax, 60\n    mov rdi, 7\n    syscall\n"
                .to_owned(),
        show_analysis: true,
        analysis_module: "digest".to_owned(),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    app.do_assemble();

    // L'empreinte peinte par le module « Empreintes » : soixante-quatre
    // caractères hexadécimaux.
    let sha = |app: &mut NamsdecApp| -> Option<String> {
        let mut trouvée = None;
        for _ in 0..3 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1500.0, 1200.0),
                    )),
                    ..Default::default()
                },
                |ui| app.draw(ui),
            );
            for clipped in &out.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    let t = texte.galley.text();
                    if t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()) {
                        trouvée = Some(t.to_owned());
                    }
                }
            }
            out.textures_delta.clear();
        }
        trouvée
    };

    let avant = sha(&mut app).expect("une empreinte avant le patch");

    // On écrit sur un octet du code — l'immédiat 7 de « mov rdi, 7 » — en
    // le passant à 8. Changer un octet de code garde un ELF valide.
    let off = app.assemble_state.as_ref().and_then(|st| {
        st.output
            .file
            .iter()
            .enumerate()
            .skip(0x400)
            .find(|&(_, &b)| b == 0x07)
            .map(|(i, _)| i)
    });
    let Some(off) = off else {
        return; // pas d'octet repère ici : rien à prouver sans risque
    };
    app.write_patch(off, 1, vec![0x08], 0);
    assert!(
        matches!(app.status, Status::Note(_)),
        "le patch doit s'appliquer : {:?}",
        app.status
    );

    let après = sha(&mut app).expect("une empreinte après le patch");
    assert_ne!(
        avant, après,
        "l'empreinte doit changer après une écriture — l'analyse était restée périmée"
    );
}

/// Le compte d'appels des imports tombe juste sur un binaire **ouvert
/// par l'application**.
///
/// Le contrôle le plus proche de l'usage, et celui qui manquait. Les
/// contrôles de `namsdec-re` reconstruisaient le listing à leur façon ;
/// celui-ci prend celui que `load_elf` produit vraiment — des sections
/// exécutables posées à leur adresse virtuelle, et non le fichier
/// désassemblé d'un bloc depuis son premier octet.
///
/// La différence n'était pas théorique : le panneau annonçait 105
/// fonctions importées et **zéro** appel vers chacune, sans que rien ne
/// le signale. Un compte faux se lit comme un compte vrai.
#[test]
fn the_imported_calls_are_counted_on_a_binary_the_app_opened() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "ls"), "l'ELF se charge");
    let state = app.assemble_state.as_ref().expect("un état d'assemblage");

    let mut module = namsdec_re::ImportsModule::default();
    let cpu = app.emulator.as_ref().expect("un émulateur").cpu();
    let ctx = namsdec_re::ReContext {
        cpu,
        disasm: &state.disasm,
        source: "",
        binary: &state.output.file,
        lang: "fr",
        names: None,
        patches: &[],
    };
    namsdec_re::ReModule::on_binary_loaded(&mut module, &ctx);

    assert!(!module.imports().is_empty(), "des imports sont nommés");
    let total: usize = module.calls().iter().sum();
    assert!(
        total > 50,
        "un binaire de cette taille appelle ses imports — zéro veut dire que la \
         conversion d'adresse ne tombe pas juste : {} imports, {total} appels",
        module.imports().len()
    );
}

/// Une adresse venue d'un panneau d'analyse mène à sa ligne du listing.
///
/// Le bout qui manque au canal de navigation : les panneaux savent dire
/// « va voir là », et c'est ici que le listing s'y rend. Sur une adresse
/// que le listing porte, on va exactement dessus ; sur une adresse qui
/// tombe entre deux lignes — une zone de bourrage, le milieu d'une
/// instruction qu'un saut vise exprès — on va à la ligne juste avant,
/// parce qu'un clic qui ne fait rien passe pour un défaut.
#[test]
fn an_address_from_an_analysis_panel_reaches_its_line() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "ls"), "l'ELF se charge");

    let (adresse, ligne) = {
        let state = app.assemble_state.as_ref().expect("un état");
        // Une instruction prise au milieu du listing : elle a une ligne,
        // et ce n'est pas la première.
        let instruction = state.disasm[state.disasm.len() / 2].address;
        let ligne = *state
            .addr_to_line
            .get(&instruction)
            .expect("une instruction du listing a une ligne");
        (instruction, ligne)
    };

    // On suit RIP au départ : c'est le réglage ordinaire.
    app.follow_rip = true;
    app.show_address(adresse);
    assert_eq!(app.goto_line, Some(ligne), "on va à la ligne de l'adresse");
    assert!(
        !app.follow_rip,
        "on cesse de suivre RIP : sinon le pas suivant ramènerait le listing là où \
         l'exécution en est, et le clic paraîtrait n'avoir rien fait"
    );

    // Une adresse qui tombe à l'intérieur d'une instruction : la ligne
    // juste avant. C'est exactement ce que vise un désalignement.
    app.goto_line = None;
    app.show_address(adresse + 1);
    assert_eq!(
        app.goto_line,
        Some(ligne),
        "une adresse au milieu d'une instruction mène à sa ligne"
    );

    // Et une adresse qui n'est dans aucune partie désassemblée le dit,
    // plutôt que de ne rien faire.
    app.goto_line = None;
    app.show_address(0);
    assert_eq!(app.goto_line, None, "on ne va nulle part");
    assert!(
        matches!(app.status, Status::Error(_)),
        "et on le dit : {:?}",
        app.status
    );
}

/// La fenêtre d'analyse ne grandit pas avec son contenu, quelle que
/// soit la rubrique.
///
/// Vu sur capture de Fred, rubrique « Chaînes lisibles » : la fenêtre
/// prenait tout l'écran et la colonne de navigation sortait par la
/// gauche. Les chemins que cette rubrique montre font parfois cent
/// caractères — un chemin de source Rust avec son empreinte de
/// version — et, peints dans une zone qui ne défilait qu'en hauteur,
/// ils poussaient la fenêtre. Mesuré : 1 100 px de large sur « En
/// bref », 1 831 sur « Chaînes ».
///
/// Pire que la taille : elle y RESTAIT. egui garde la taille d'une
/// fenêtre, et revenir sur « En bref » ne la rendait pas.
///
/// Le contrôle passe TOUTES les rubriques à la file, et vérifie après
/// chacune que la fenêtre a gardé la taille qu'elle avait au départ.
/// Une rubrique nouvelle y entre sans qu'on y pense.
#[test]
fn la_fenetre_d_analyse_ne_grandit_pas_avec_son_contenu() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    if !app.load_elf(&octets, NOM_FABRIQUÉ) {
        return;
    }
    app.show_analysis = true;
    let écran = egui::vec2(1920.0, 1200.0);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, écran)),
        ..Default::default()
    };
    let dessine = |app: &mut NamsdecApp| -> egui::Rect {
        // Six images : egui amène une fenêtre à sa taille au lieu de
        // l'y poser, et c'est la taille posée qu'on mesure.
        for _ in 0..6 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            sortie.textures_delta.clear();
        }
        egui::AreaState::load(&ctx, egui::Id::new("analysis_window"))
            .map(|a| a.rect())
            .expect("la fenêtre d'analyse est affichée")
    };

    let départ = dessine(&mut app);
    assert!(
        départ.width() < écran.x - 100.0,
        "au départ, la fenêtre laisse voir ce qu'il y a derrière : {départ:?}"
    );

    let rubriques: Vec<String> = app
        .re_registry
        .entries("fr")
        .iter()
        .map(|(id, _)| (*id).to_owned())
        .collect();
    let mut fautives: Vec<String> = Vec::new();
    for id in rubriques {
        app.analysis_module = id.clone();
        let cadre = dessine(&mut app);
        if cadre.width() > départ.width() + 2.0 || cadre.height() > départ.height() + 2.0 {
            fautives.push(format!(
                "« {id} » la pousse à {:.0}x{:.0}",
                cadre.width(),
                cadre.height()
            ));
        }
    }
    assert!(
        fautives.is_empty(),
        "la fenêtre doit garder sa taille, et ce qui dépasse doit défiler : {}",
        fautives.join(", ")
    );

    // Et elle est rendue telle qu'on l'avait laissée : une fenêtre qui
    // grandit sans revenir est pire qu'une fenêtre trop grande.
    app.analysis_module.clear();
    let fin = dessine(&mut app);
    assert!(
        (fin.width() - départ.width()).abs() < 2.0,
        "revenue sur « Tout », la fenêtre a la taille du départ : {départ:?} puis {fin:?}"
    );
}

/// La fenêtre d'analyse s'agrandit par sa pastille verte, et revient.
///
/// Elle avait perdu ce geste le 13 octobre, et pour une bonne raison :
/// agrandie, elle recouvrait tout, et cliquer une adresse emmenait le
/// listing derrière un panneau opaque. Cette raison-là est tombée — le
/// saut au listing efface la fenêtre, et le saut aux octets reste
/// dedans. Un vidage hexadécimal et un graphe d'appels se lisent mieux
/// en grand.
///
/// Le contrôle clique la pastille pour de vrai, et la cherche là où
/// elle est peinte plutôt qu'à une position calculée : un bouton posé
/// ailleurs qu'on ne croit est précisément ce qu'on veut attraper.
#[test]
fn the_analysis_window_grows_by_its_green_pastille_and_comes_back() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.show_analysis = true;
    let base = écran_du_banc();

    let verte = chrome::WindowButton::Maximize.colour();
    // Où la pastille verte est peinte. Trois images : egui amène la
    // fenêtre à sa taille, et une pastille relevée sur la première
    // bougerait encore.
    let où_est_la_verte = |app: &mut NamsdecApp| -> Option<egui::Pos2> {
        let mut vue = None;
        for _ in 0..3 {
            vue = None;
            let mut sortie = ctx.run_ui(base.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Circle(cercle) = &clipped.shape
                    && cercle.center.y > 40.0
                    && (cercle.radius - 6.5).abs() < 0.01
                    // La teinte, non l'opacité : la fenêtre arrive en
                    // fondu, et la pastille est alors plus sombre.
                    && cercle.fill.g() > cercle.fill.r()
                    && cercle.fill.g() > cercle.fill.b()
                {
                    let _ = verte;
                    vue = Some(cercle.center);
                }
            }
            sortie.textures_delta.clear();
        }
        vue
    };
    let clique = |app: &mut NamsdecApp, où: egui::Pos2| {
        let mut presse = base.clone();
        presse.events.push(egui::Event::PointerMoved(où));
        presse.events.push(egui::Event::PointerButton {
            pos: où,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        ctx.run_ui(presse, |ui| app.draw(ui)).textures_delta.clear();
        let mut relâche = base.clone();
        relâche.events.push(egui::Event::PointerButton {
            pos: où,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        ctx.run_ui(relâche, |ui| app.draw(ui))
            .textures_delta
            .clear();
    };

    let pastille = où_est_la_verte(&mut app).expect("la pastille verte est peinte");
    let avant = ctx
        .memory(|m| m.area_rect(egui::Id::new("analysis_window")))
        .expect("la fenêtre est ouverte");

    clique(&mut app, pastille);
    assert!(
        app.window_maximized(NamsdecApp::ANALYSIS_WINDOW),
        "le clic sur la pastille verte doit agrandir la fenêtre d'analyse"
    );
    for _ in 0..12 {
        ctx.run_ui(base.clone(), |ui| app.draw(ui))
            .textures_delta
            .clear();
    }
    let agrandie = ctx
        .memory(|m| m.area_rect(egui::Id::new("analysis_window")))
        .expect("toujours ouverte");
    assert!(
        agrandie.width() > avant.width() && agrandie.height() >= avant.height(),
        "agrandie, elle doit prendre plus de place : {agrandie:?} contre {avant:?}"
    );

    // Et elle revient : une fenêtre qui s'agrandit sans savoir
    // rapetisser est un piège, pas un geste.
    let pastille = où_est_la_verte(&mut app).expect("la pastille est toujours là");
    clique(&mut app, pastille);
    assert!(
        !app.window_maximized(NamsdecApp::ANALYSIS_WINDOW),
        "le second clic doit la restaurer"
    );
}

/// Le texte de l'analyse n'est pas coupé par son propre ascenseur.
///
/// LE DÉFAUT, ET POURQUOI IL A ÉCHAPPÉ À UN CONTRÔLE PENDANT DES
/// SEMAINES. La largeur de lecture se prenait sur toute la place
/// disponible, ascenseur vertical compris ; dans l'onglet — plus étroit
/// que la largeur de lecture, donc là où la borne ne mord pas — les
/// paragraphes se posaient une dizaine de pixels trop larges, se
/// coupaient au milieu d'un mot (« l'ad », « dan », « à ce ») et
/// réclamaient une barre HORIZONTALE pour trois caractères.
///
/// Le premier contrôle écrit pour lui cherchait le TEXTE COUPÉ, et il
/// passait au vert avec le défaut remis — à vingt-six largeurs, avec un
/// binaire chargé, à l'échelle de cet écran. Il a été retiré plutôt que
/// gardé pour l'ornement, et la question est restée ouverte.
///
/// CELUI-CI CHERCHE LA BARRE, et c'est tout ce qui change. Le texte
/// coupé est le SYMPTÔME ; la barre horizontale est le mécanisme — le
/// contenu déborde la vue de la largeur exacte de l'ascenseur. C'est la
/// même mesure qui a tranché sur le vidage hexadécimal le même jour, et
/// pour la même raison : l'ascenseur vertical d'une zone egui se pose à
/// l'intérieur de sa largeur, pas à côté.
///
/// Le module est « En bref » : sa prose se replie à la largeur de
/// lecture, donc elle ne déborde JAMAIS d'elle-même. Un panneau dont le
/// contenu est large de lui-même — les chaînes et leurs chemins de cent
/// caractères — a le droit à sa barre, et n'aurait rien prouvé.
#[test]
fn the_analysis_text_is_not_cut_by_its_own_scrollbar() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    // Des largeurs où la zone centrale reste PLUS ÉTROITE que la
    // largeur de lecture : au-delà, la borne mord et la soustraction
    // est sans effet — c'est exactement pourquoi le défaut ne se voyait
    // pas dans la fenêtre, qui est plus large.
    for (largeur, hauteur) in [(1100.0_f32, 800.0_f32), (1280.0, 820.0), (1400.0, 900.0)] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        if !app.load_elf(&octets, NOM_FABRIQUÉ) {
            return;
        }
        app.show_re_tab = true;
        app.vue_centrale = VueCentrale::Analysis;
        app.analysis_module = "binary_info".to_owned();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, hauteur),
            )),
            ..Default::default()
        };
        let mut barres: Vec<egui::Rect> = Vec::new();
        for _ in 0..3 {
            barres.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                // La poignée d'un ascenseur horizontal : basse, large,
                // et dans la zone centrale. Reconnue à ses MESURES et
                // non à sa couleur — les thèmes changent, la
                // silhouette non.
                if let egui::epaint::Shape::Rect(r) = &clipped.shape
                    && r.rect.height() < 14.0
                    && r.rect.width() > 200.0
                    && r.rect.top() > hauteur * 0.2
                    && r.rect.bottom() < hauteur * 0.8
                {
                    barres.push(r.rect);
                }
            }
            output.textures_delta.clear();
        }
        assert!(
            barres.is_empty(),
            "à {largeur}x{hauteur}, « En bref » déborde sa vue et réclame une barre \
             horizontale : {barres:?}"
        );
    }
}

/// Un listing qui tient en entier ne cache pas son début.
///
/// Vu sur une capture : la vue du désassemblage montrait les neuf
/// dernières instructions d'un programme qui en compte treize, avec six
/// cents pixels de vide en dessous. `_start` et les trois instructions
/// qui suivent étaient poussés hors du cadre — sur un programme de
/// treize lignes, dans une vue qui en tient trente-huit.
///
/// La cause : egui ne ramène pas dans ses bornes une position de
/// défilement posée à la main. « Trois lignes de contexte au-dessus de
/// l'instruction courante » est juste sur un long listing et absurde
/// sur un court, où il n'y a rien à faire défiler du tout.
///
/// Le relevé compte les ADRESSES peintes : c'est la seule mesure qui
/// réponde à la question qu'on se pose vraiment — « voit-on le
/// programme ? ».
#[test]
fn un_listing_qui_tient_en_entier_ne_cache_pas_son_debut() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    for (nom, vue) in [
        ("la vue du désassemblage", Some(VueCentrale::Disassembly)),
        ("l'onglet Octets", None),
    ] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        app.source = PROGRAMME_DU_BANC.to_owned();
        app.do_assemble();
        // Huit pas : on est au milieu du programme, donc la vue a une
        // raison de vouloir suivre.
        for _ in 0..8 {
            app.do_step();
        }
        match vue {
            Some(vue) => app.montrer_vue(vue),
            None => {
                app.show_sidebar = true;
                app.active_tab = SidebarTab::Bytes;
            }
        }

        let attendues = app
            .assemble_state
            .as_ref()
            .map(|s| s.disasm.len())
            .expect("le programme est assemblé");
        assert!(
            attendues > 8,
            "le programme d'épreuve doit avoir de quoi défiler : {attendues}"
        );

        // Les adresses, d'où qu'elles viennent : la vue les peint
        // seules, l'onglet Octets les colle au numéro de ligne
        // (« 8 00400000 »). On cherche donc le MOT, et non l'étiquette
        // entière — un premier essai filtrait sur l'étiquette et
        // comptait zéro adresse dans l'onglet, ce qui aurait fait
        // passer le contrôle pour un défaut qu'il n'avait pas vu.
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
        let adresses: std::collections::BTreeSet<String> = lignes
            .iter()
            .flat_map(|(t, _)| t.split_whitespace())
            .filter(|mot| mot.len() == 8 && mot.chars().all(|c| c.is_ascii_hexdigit()))
            .map(|mot| mot.to_owned())
            .collect();
        assert_eq!(
            adresses.len(),
            attendues,
            "{nom} : {} instruction(s) peinte(s) sur {attendues} — le début \
             du programme est caché alors que tout y tient : {adresses:?}",
            adresses.len()
        );
    }
}

/// Une liste qui tient dans la place qu'on lui laisse n'en cache aucune
/// ligne.
///
/// Vu sur une capture : « Chaînes lisibles » annonçait treize chaînes et
/// en montrait onze et demie, avec deux cent cinquante pixels de vide
/// dessous. On faisait défiler treize lignes dans un hublot, devant un
/// écran libre. La cause était un `max_height(220.0)` en dur, et le
/// vidage du fichier avait le même avec 320.
///
/// Aucune mesure ne le demandait : le banc vérifie qu'une liste ne
/// déborde pas, jamais qu'elle occupe ce qu'on lui donne.
///
/// Le relevé compte les ADRESSES peintes, et les compare au nombre que
/// le module annonce lui-même — on ne recopie pas le chiffre ici, il
/// dépend du binaire.
#[test]
fn une_liste_qui_tient_ne_cache_aucune_ligne() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.source = PROGRAMME_DU_BANC.to_owned();
    app.do_assemble();
    app.analysis_module = "strings".to_owned();
    app.montrer_vue(VueCentrale::Analysis);

    let (_, lignes) = ce_qui_est_écrit_sur(
        &ctx,
        &mut app,
        None,
        4,
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        },
    );
    let peint: Vec<&str> = lignes.iter().map(|(t, _)| t.as_str()).collect();

    // Ce que le module annonce : « 13 chaînes d'au moins quatre
    // caractères ». On lit SON chiffre, on ne le recopie pas.
    let annoncé = peint
        .iter()
        .find_map(|t| {
            t.contains("chaînes")
                .then(|| t.split_whitespace().next())
                .flatten()
                .and_then(|mot| mot.parse::<usize>().ok())
        })
        .expect("le module annonce combien de chaînes il a trouvées");
    assert!(
        annoncé > 8,
        "le programme d'épreuve doit en porter assez pour que la \
         question se pose : {annoncé}"
    );

    // Et ce qu'il montre : une adresse par chaîne, huit chiffres
    // hexadécimaux, chacune dans sa propre étiquette.
    let adresses = peint
        .iter()
        .filter(|t| t.len() == 8 && t.chars().all(|c| c.is_ascii_hexdigit()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        adresses.len(),
        annoncé,
        "le module annonce {annoncé} chaînes et en montre {} — la liste \
         se replie alors que la place ne manque pas : {adresses:?}",
        adresses.len()
    );
}

/// LE MODE DÉBUTANT ALLÈGE LA COLONNE, SANS RIEN DÉSINSTALLER.
///
/// namsdec sert à apprendre l'assembleur et la rétro-ingénierie.
/// Vingt-cinq rubriques devant quelqu'un qui ouvre son premier binaire
/// lui demandent de savoir ce qu'il cherche avant d'avoir appris à
/// chercher ; neuf se parcourent.
///
/// Trois choses à tenir, et chacune est une façon de se tromper :
/// la liste RACCOURCIT vraiment ; ce qui reste est ce qui se lit sans
/// rien savoir ; et la rubrique OUVERTE ne disparaît jamais, même
/// au-dessus du niveau — on y arrive par un lien d'un autre panneau, et
/// l'escamoter laisserait un panneau à l'écran sans moyen d'en sortir.
#[test]
fn le_mode_debutant_allege_la_colonne_sans_rien_desinstaller() {
    let mut app = NamsdecApp::default();
    let lang = "fr";
    let entier = app.re_registry.nav_entries(lang);
    let débutant = app
        .re_registry
        .nav_entries_jusqu_a(lang, namsdec_re::Niveau::Débutant);
    assert!(
        débutant.len() * 2 < entier.len(),
        "le mode débutant doit vraiment alléger : {} sur {}",
        débutant.len(),
        entier.len()
    );
    // Ce qui reste : ce qu'on regarde pour apprendre. Les identifiants
    // et non les titres — un titre change avec la langue.
    let ids: Vec<&str> = débutant.iter().map(|(id, _, _)| *id).collect();
    for attendu in [
        "binary_info",
        "map",
        "functions",
        "call_graph",
        "xrefs",
        "strings",
        "syscalls",
        "imports",
        "hex_editor",
    ] {
        assert!(
            ids.contains(&attendu),
            "« {attendu} » se lit sans rien savoir d'avance : {ids:?}"
        );
    }
    // Et ce qui part : ce qui suppose qu'on sache déjà ce qu'on cherche.
    for écarté in ["entropy", "gadgets", "yara", "detour", "slack"] {
        assert!(
            !ids.contains(&écarté),
            "« {écarté} » demande de savoir ce qu'on cherche : {ids:?}"
        );
    }

    // La rubrique ouverte reste listée, même au-dessus du niveau.
    let avec_entropie =
        app.re_registry
            .nav_entries_pour(lang, namsdec_re::Niveau::Débutant, "entropy");
    assert!(
        avec_entropie.iter().any(|(id, _, _)| *id == "entropy"),
        "une rubrique ouverte ne disparaît pas sous les pieds"
    );
    assert_eq!(
        avec_entropie.len(),
        débutant.len() + 1,
        "et elle n'en ramène aucune autre"
    );

    // Enfin, ce que la colonne PEINT vraiment : le relevé, et non la
    // liste. Une liste juste et une colonne qui montre autre chose
    // seraient deux choses différentes.
    app.debutant = true;
    app.show_analysis = true;
    app.analysis_module.clear();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, textes) = ce_qui_est_écrit(&ctx, &mut app, Some("analysis_window"), 3);
    let peints: Vec<&str> = textes.iter().map(|(t, _)| t.as_str()).collect();
    let écrit = |quoi: &str| peints.contains(&quoi);
    assert!(écrit("Chaînes lisibles"), "{peints:?}");
    assert!(
        !écrit("Entropie"),
        "l'entropie n'est pas dans la colonne du débutant : {peints:?}"
    );
    // « Tout » existe à tous les niveaux : c'est le compte rendu.
    assert!(écrit(strings(Lang::Fr).analysis_all), "{peints:?}");
}
