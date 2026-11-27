//! Le banc de mise en page : les règles qui valent pour tout l'écran.
//!
//! Un invariant par contrôle, éprouvé sur TOUTES les scènes du banc —
//! rien ne se recouvre, rien n'est coupé, chaque texte ressort de son fond,
//! une ellipse s'écrit comme une ellipse. Les scènes elles-mêmes et leurs
//! outils de mesure sont restés dans `tests.rs`, puisque tout le monde s'en
//! sert.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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
        // Le style de l'application, et pas seulement ses couleurs : les
        // espacements et les polices d'egui par défaut ne sont pas les
        // siens, et l'on mesurerait une largeur qui n'existe nulle part.
        NamsdecApp::default().apply_theme(&ctx);
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
            active_tab: SidebarTab::Cpu,
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

/// Et AUCUN TEXTE de l'onglet Mémoire n'est coupé à droite.
///
/// Le vidage défile horizontalement, et il le doit : seize octets et
/// leur transcription ne se replient pas à la largeur d'un panneau.
/// Mais les PHRASES qui l'accompagnent héritaient du même réglage, et
/// deux explications sur trois partaient hors de l'écran — « 1 octet,
/// puis un zéro : c'est là que `write` s… ».
///
/// Mesuré au sens de l'invariant « rien n'est coupé » : l'encre
/// comparée à l'encre ∩ découpage.
#[test]
fn no_sentence_of_the_memory_tab_is_cut_off() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Memory,
        next_instruction_open: false,
        source: "global _start
section .text
_start:
    mov rax, 60
    xor rdi, rdi
    syscall
"
        .to_owned(),
        ..Default::default()
    };
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    app.apply_theme(&ctx);
    app.do_assemble();
    app.memory_format = MemoryFormat::Text;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1200.0),
        )),
        ..Default::default()
    };
    let mut coupés: Vec<String> = Vec::new();
    for _ in 0..4 {
        coupés.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            let egui::epaint::Shape::Text(t) = &clipped.shape else {
                continue;
            };
            // LIGNE PAR LIGNE, et non galley par galley.
            //
            // C'est ce qui a d'abord fait passer ce contrôle pour rien.
            // Tant que le vidage était peint d'un bloc, ses phrases et
            // sa ligne d'octets ne faisaient qu'UNE galley, dont le
            // texte commence par « 0x… » — et le filtre « les lignes de
            // vidage ont le droit de dépasser » les écartait toutes
            // ensemble. La phrase coupée n'était même pas regardée.
            for ligne in t.galley.rows.iter() {
                let texte = ligne.text();
                // Une ligne de VIDAGE a le droit de dépasser : c'est
                // pour elle que la zone défile horizontalement.
                if texte.trim_start().starts_with("0x") {
                    continue;
                }
                // Une phrase : plusieurs mots.
                if texte.split_whitespace().count() < 4 {
                    continue;
                }
                let encre = ligne
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
        coupés.is_empty(),
        "des phrases de l'onglet Mémoire sont coupées : {coupés:#?}"
    );
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
        include_str!("../i18n.rs"),
        include_str!("../main.rs"),
        include_str!("../sidebar.rs"),
        include_str!("../errors.rs"),
        include_str!("../roles.rs"),
        include_str!("../readings.rs"),
        include_str!("../nasm_errors.rs"),
        include_str!("../condition.rs"),
        include_str!("../../../namsdec-re/src/strings.rs"),
        include_str!("../../../namsdec-re/src/protections.rs"),
        include_str!("../../../namsdec-re/src/xrefs_module.rs"),
        include_str!("../../../namsdec-re/src/gadgets_module.rs"),
        include_str!("../../../namsdec-re/src/imports_module.rs"),
        include_str!("../../../namsdec-re/src/libc.rs"),
        include_str!("../../../namsdec-re/src/binary_info.rs"),
        include_str!("../../../namsdec-disasm/src/pseudo.rs"),
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
            kind: Default::default(),
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
                textes.push((t.galley.text().to_owned(), cadre_du_texte(t)));
            }
        }
        sortie.textures_delta.clear();
    }

    let fenêtre = egui::AreaState::load(&ctx, egui::Id::unique("decompiler_window"))
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
            rect = egui::AreaState::load(&ctx, egui::Id::unique("calculator_window"))
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
            let Some(état) = crate::panneaux::état(&ctx, nom) else {
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
            // L'onglet de la vue, qui en est le titre depuis qu'il est
            // posé juste au-dessus d'elle.
            SidebarTab::Cpu.label(strings(Lang::Fr)),
            strings(Lang::Fr).tab_stack,
        ] {
            assert!(
                peint.contains(attendu),
                "« {attendu} » manque à la taille {taille:?}"
            );
        }
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
        let id_panneau = crate::panneaux::id_racine(&ctx, "sidebar");
        ctx.data_mut(|data| {
            data.insert_persisted(
                id_panneau,
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
            let cadre = id
                .and_then(|id| egui::AreaState::load(&ctx, egui::Id::unique(id)).map(|a| a.rect()));
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
                    coupés.push((t.galley.text().to_owned(), cadre_du_texte(t)));
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
                    let place = cadre_du_texte(t);
                    if dans.is_some_and(|c| !c.contains_rect(place.shrink(1.0))) {
                        continue;
                    }
                    // CE QUI EST DÉCOUPÉ N'EST PAS PEINT, donc n'a pas
                    // de contraste.
                    //
                    // Une zone défilante remet à egui toutes ses
                    // lignes, y compris celles qui sortent par le bas,
                    // et c'est egui qui les coupe à l'affichage. Le
                    // relevé, lui, voyait leur forme et lui cherchait
                    // un fond — en le prenant, le cas échéant, sous un
                    // panneau ancré plus bas, qui recouvre justement
                    // cette place-là.
                    //
                    // Mesuré le 20 novembre 2026 : le parcours passe de
                    // cent dix-sept étapes à cent vingt, la liste des
                    // exercices s'allonge de trois rangées, et le
                    // « Charger » de la vingtième tombe hors de la
                    // fenêtre, pile sur le bouton d'accent de la barre
                    // du bas. Deux virgule zéro sept de contraste pour
                    // un texte que personne ne voit — et, à l'inverse,
                    // cent dix-sept étapes qui passaient par chance.
                    // Un relevé qui dépend du nombre de lignes d'une
                    // liste fait douter de tous les autres.
                    if !clipped.clip_rect.intersects(place) {
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
        let cadre = egui::AreaState::load(&ctx, egui::Id::unique(id))
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
    // Le style de l'application, et pas seulement ses couleurs : les
    // espacements et les polices d'egui par défaut ne sont pas les
    // siens, et l'on mesurerait une largeur qui n'existe nulle part.
    NamsdecApp::default().apply_theme(&ctx);
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
        // Le bouton LANCE : depuis P2, l'assemblage tourne dans un fil
        // à part et l'interface continue de vivre pendant ce temps. Ce
        // que fait ici l'attente, l'application le fait en plusieurs
        // images, par `tick_assemble`.
        app.attendre_l_assemblage();
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
            // QUATRIÈME écriture, et elle coûte onze boutons :
            // `ui.small_button(`. Trouvée par un relevé des formes, qui
            // mesurait les hauteurs de commandes et s'est étonné d'en
            // voir à quinze pixels — « RAX », « RSP », « RIP » dans le
            // motif cyclique, à soixante pixels de trois boutons
            // encadrés de vingt-sept. Un petit bouton reste un bouton :
            // il doit se voir sans qu'on le survole.
            let petit = ligne.contains("ui.small_button(")
                || (ligne.trim_start().starts_with(".small_button(")
                    && source
                        .lines()
                        .nth(n.saturating_sub(1))
                        .is_some_and(|d| d.trim_end().ends_with("ui")));
            let plat = ligne.contains("ui.button(")
                || petit
                || nu
                || (ligne.trim_start().starts_with(".button(")
                    && source
                        .lines()
                        .nth(n.saturating_sub(1))
                        .is_some_and(|d| d.trim_end().ends_with("ui")));
            // DANS UN MENU, UN BOUTON PLAT EST LA RÈGLE, et c'est à la
            // fonction qu'on reconnaît le menu — le relevé lit du
            // texte, il ne sait pas qu'une ligne est dans la fermeture
            // d'un `MenuButton`. Les trois nommées ici n'ouvrent que
            // des menus : la barre de menus, la barre d'outils (son
            // menu de débordement), la rangée d'onglets (le « +N »
            // qui liste les onglets cachés), et le menu du clic droit
            // sur un onglet — lequel n'est une fonction à part que pour
            // pouvoir être nommé ici.
            if plat
                && !matches!(
                    fonction,
                    "menu_bar_ui" | "toolbar_ui" | "rangée_d_onglets_repliée" | "menu_d_un_onglet"
                )
            {
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
            // ET L'ASTÉRISQUE SIMPLE, depuis le 10 novembre 2026.
            //
            // Le contrôle ne regardait que les doubles, et cinquante
            // passages des fiches emploient les simples : « le
            // débordement *non signé* », « laisse RAX *intacte* ». La
            // fenêtre d'aide les montrait donc telles quelles depuis
            // toujours, sans que personne le voie — jusqu'à ce qu'une
            // bande les mette au premier écran, en gros.
            //
            // LA MULTIPLICATION RESTE PERMISE, et c'est la raison de
            // la forme exacte du motif : on ne refuse qu'une paire qui
            // se comporte comme du balisage — ouvrante collée à ce qui
            // suit, fermante collée à ce qui précède. « RAX * 3 » a un
            // blanc de chaque côté et passe.
            assert!(
                !paire_d_emphase(texte),
                "« {nom} » montre une emphase non rendue : {:?}",
                texte.chars().take(90).collect::<String>()
            );
        }
    }
}

/// Le texte porte-t-il une paire d'astérisques qui se lit comme du
/// balisage ?
///
/// Les règles de Markdown, réduites à ce qu'on emploie : l'ouvrante
/// est collée au mot qui suit, la fermante au mot qui précède, et
/// l'emphase ne passe pas la ligne.
fn paire_d_emphase(texte: &str) -> bool {
    let octets = texte.as_bytes();
    let mut depuis = 0usize;
    while let Some(relatif) = texte.get(depuis..).and_then(|q| q.find('*')) {
        let ouvre = depuis + relatif;
        if !octets
            .get(ouvre + 1)
            .is_some_and(|c| !c.is_ascii_whitespace() && *c != b'*')
        {
            depuis = ouvre + 1;
            continue;
        }
        let mut curseur = ouvre + 1;
        while let Some(relatif) = texte.get(curseur..).and_then(|q| q.find('*')) {
            let candidat = curseur + relatif;
            if texte[ouvre + 1..candidat].contains('\n') {
                break;
            }
            if octets
                .get(candidat - 1)
                .is_some_and(|c| !c.is_ascii_whitespace())
            {
                return true;
            }
            curseur = candidat + 1;
        }
        depuis = ouvre + 1;
    }
    false
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
    let source = include_str!("../i18n.rs");
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
        // L'onglet, et non le titre de la vue : celui-ci a disparu avec
        // le déplacement de la rangée en tête du panneau, où l'onglet
        // souligné dit déjà ce qu'on regarde.
        let bas = titre(SidebarTab::Cpu.label(s));
        assert!(
            haut.bottom() < bas.top(),
            "{lang:?} : « {} » est au-dessus de « {} »",
            s.next_instruction,
            SidebarTab::Cpu.label(s)
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
    // Les huit réglages de la page Général, dans l'ordre où on les lit.
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
        (s.preferences_prereleases, s.preferences_prereleases_hint),
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
/// Tout caractère que l'interface peint existe dans la police.
///
/// Un caractère absent ne manque pas discrètement : egui peint un
/// **carré vide** à sa place. Les polices qu'il embarque — Ubuntu et
/// deux polices d'émojis — n'ont ni les petits triangles, ni la coche
/// `\u{2713}`, ni la croix `\u{2717}`, ni les flèches `\u{2192}`,
/// `\u{25b6}`, `\u{25c0}`, `\u{21a9}`. Tous étaient employés quelque
/// part, et se voyaient en carré.
///
/// Le contrôle ne lit aucune liste écrite d'avance : il regarde ce qui
/// est **peint**, sur toutes les scènes du banc. Ce qui existe et qu'on
/// peut employer : `\u{2039}` `\u{203a}` `\u{00ab}` `\u{00bb}` `+`
/// `\u{2212}` `\u{00d7}` `\u{221a}` `\u{2022}` `\u{25cb}` `\u{2026}`
/// `\u{21ba}`.
#[test]
fn tout_caractere_peint_existe_dans_la_police() {
    // La police PROPORTIONNELLE, quelle que soit celle du texte, et
    // c'est mesuré : interrogée sur la famille à chasse fixe,
    // `has_glyph` répond « non » pour tout, jusqu'au signe plus. La
    // question reste la bonne — la chaîne de repli de la chasse fixe
    // contient celle de la proportionnelle —, elle est seulement un peu
    // sévère.
    let police = egui::FontId::proportional(13.0);
    let mut absents: Vec<(String, char, String)> = Vec::new();
    for (nom, _, ouvre) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let entrée = écran_du_banc();
        let mut textes: Vec<String> = Vec::new();
        for _ in 0..3 {
            textes.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    textes.push(t.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        for texte in &textes {
            for c in texte.chars() {
                if c.is_ascii() || c.is_whitespace() {
                    continue;
                }
                if !ctx.fonts_mut(|f| f.has_glyph(&police, c)) {
                    absents.push((nom.clone(), c, texte.clone()));
                }
            }
        }
    }
    absents.sort();
    absents.dedup_by(|a, b| a.1 == b.1);
    assert!(
        absents.is_empty(),
        "des caractères se peignent en carré vide — la police ne les a pas : {}",
        absents
            .iter()
            .map(|(nom, c, texte)| format!("{nom} : U+{:04X} {c} dans « {texte} »", *c as u32))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

#[test]
fn two_buttons_side_by_side_share_a_height() {
    for (nom, _, ouvre) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let entrée = écran_du_banc();
        // AVEC LA ZONE OÙ CHAQUE BOÎTE EST PEINTE.
        //
        // Deux boutons ne sont voisins que dans la même zone. Sans ce
        // second terme, la règle rapprochait une entrée de la liste de
        // navigation d'une fenêtre d'analyse — colonne de gauche, 21 px
        // — d'un bouton du panneau de droite, trente-trois pixels plus
        // loin et dans un autre panneau : deux choses qui ne se
        // comparent pas, et que personne ne lit comme une rangée. Le
        // rectangle de rognage dit de quelle zone une forme vient, et
        // c'est egui qui le pose, non nous.
        let mut boîtes: Vec<(egui::Rect, egui::Rect)> = Vec::new();
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
                    boîtes.push((r.rect, clipped.clip_rect));
                }
            }
            sortie.textures_delta.clear();
        }
        for (i, (a, zone_a)) in boîtes.iter().enumerate() {
            for (b, zone_b) in &boîtes[i + 1..] {
                // Pas la même zone : pas la même rangée.
                if zone_a != zone_b {
                    continue;
                }
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
    let gauche = |lignes: &[(String, egui::Rect)], quoi: &str| -> f32 {
        lignes
            .iter()
            .filter(|(t, _)| t == quoi)
            .map(|(_, r)| r.left())
            .fold(f32::INFINITY, f32::min)
    };

    // LA RANGÉE D'ONGLETS a remplacé les six titres de vue le 26
    // octobre 2026 : c'est elle, maintenant, qui dit en tête du panneau
    // ce qu'on regarde. Son premier onglet part donc du bord du
    // panneau, et il en part TOUJOURS DU MÊME — quelle que soit la vue
    // choisie, puisque la rangée ne change pas d'ordre.
    let mut bords: Vec<(&str, f32)> = Vec::new();
    for onglet in SidebarTab::ALL {
        if onglet == SidebarTab::Re {
            continue; // absent tant qu'on ne l'a pas demandé
        }
        app.active_tab = onglet;
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
        let premier = SidebarTab::Cpu.label(s);
        let x = gauche(&lignes, premier);
        assert!(
            x.is_finite(),
            "la rangée porte « {premier} » en tête du panneau, vue {onglet:?} \
             choisie"
        );
        bords.push((onglet.label(s), x));
    }
    let premier = bords[0].1;
    let hors: Vec<String> = bords
        .iter()
        .filter(|(_, x)| (x - premier).abs() > 0.5)
        .map(|(quoi, x)| format!("vue « {quoi} » à {x:.1}"))
        .collect();
    assert!(
        hors.is_empty(),
        "la rangée ne part pas du même bord selon la vue choisie : \
         {premier:.1} px, sauf {}",
        hors.join(", ")
    );

    // Et le titre À CHEVRON du bandeau s'écarte de ce bord, mais de peu :
    // la largeur du chevron, pas davantage. C'est ce qui tient les deux
    // rangées du panneau sur un même bord à l'œil.
    app.active_tab = SidebarTab::Cpu;
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
    let bandeau = gauche(&lignes, s.next_instruction);
    assert!(bandeau.is_finite(), "le bandeau écrit son titre");
    let écart = bandeau - premier;
    assert!(
        (0.0..=24.0).contains(&écart),
        "le titre à chevron s'écarte du bord de la largeur d'un chevron, \
         et non de {écart:.1} px"
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
    // UNE SEULE ZONE À TITRE depuis le 26 octobre 2026 : le bandeau. La
    // vue, elle, est annoncée par la RANGÉE D'ONGLETS posée juste
    // au-dessus d'elle, qui porte sa propre marque — un trait SOUS
    // l'onglet actif, et non au-dessus d'un titre. Deux idiomes, deux
    // mesures : celui-ci ne vaut que pour les titres de zone.
    let titre = s.next_instruction;
    let haut = lignes
        .iter()
        .filter(|(t, _)| t == titre)
        .map(|(_, r)| r.top())
        .fold(f32::INFINITY, f32::min);
    assert!(haut.is_finite(), "« {titre} » est peint dans le bandeau");
    let dessus: Vec<f32> = filets
        .iter()
        .copied()
        .filter(|y| *y < haut && haut - *y < 24.0)
        .collect();
    assert!(
        dessus.is_empty(),
        "« {titre} » porte un filet au-dessus de lui, en y = {dessus:?} pour \
         un titre en {haut:.0} — une zone se souligne, elle ne s'encadre pas"
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
        ("main.rs", include_str!("../main.rs")),
        ("sidebar.rs", include_str!("../sidebar.rs")),
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
        crate::panneaux::état(&ctx, id)
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

/// Un contrôle qui mesure une géométrie pose d'abord le style.
///
/// C'est la règle écrite de A 7.6, et elle est enfin tenue par autre
/// chose qu'une bonne intention. Quatre défauts d'affichage étaient
/// passés sous des tests qui paraissaient les couvrir, et l'un d'eux
/// tenait à cela : le test ne posait pas le thème, donc mesurait les
/// espacements et les polices d'egui par défaut — des valeurs qui
/// n'existent nulle part dans l'application. Une largeur juste sous ce
/// style-là peut être fausse sous le vrai.
///
/// La règle ne vaut que pour les contrôles qui **mesurent** : chercher un
/// mot dans ce qui est peint ne demande pas le bon espacement, et
/// l'exiger de trois cents tests serait du bruit. Ce qui la déclenche,
/// c'est de lire une position, une taille de galley, un rectangle de
/// découpe ou une forme peinte.
///
/// `apply_theme` suffit, sur l'application du test ou sur une jetable
/// (`NamsdecApp::default().apply_theme(&ctx)`) quand le test dessine un
/// composant isolé. Le banc de mise en page, lui, le fait déjà pour
/// toutes ses scènes.
#[test]
fn a_control_that_measures_geometry_sets_the_style_first() {
    // Ce qui compte comme une mesure : on lit la géométrie de ce qui a
    // été peint, et non son texte.
    const MESURES: [&str; 6] = [
        ".galley.size()",
        "clip_rect",
        ".pos.x",
        ".pos.y",
        "Shape::Rect",
        "Shape::Circle",
    ];

    // Ce contrôle-ci porte les motifs puisqu'il les cherche.
    const MOI: &str = "a_control_that_measures_geometry_sets_the_style_first";

    let mut fautifs: Vec<String> = Vec::new();
    // Les vingt thèmes et cette page : depuis le découpage, lire la seule
    // source où l'on se trouve reviendrait à ne regarder qu'un
    // vingtième des contrôles.
    for (fichier, source) in SOURCES_DU_BANC {
        for morceau in source.split("#[test]\n").skip(1) {
            let Some(nom) = morceau
                .split_once("fn ")
                .and_then(|(_, reste)| reste.split_once('('))
                .map(|(nom, _)| nom.trim())
            else {
                continue;
            };
            if nom == MOI {
                continue;
            }
            let corps = morceau
                .split_once('{')
                .map(|(_, reste)| reste)
                .unwrap_or(morceau);
            // Et il s'arrête à SON accolade fermante, en colonne zéro.
            // Sans cela, le morceau court jusqu'au `#[test]` suivant et
            // s'approprie les fonctions libres posées entre les deux :
            // le relevé a désigné `le_binaire_fabrique_est_un_vrai_binaire`
            // pour un `clip_rect` qui appartenait à `ce_qui_est_écrit`,
            // trente lignes plus bas.
            let corps = corps.split_once("\n}\n").map(|(c, _)| c).unwrap_or(corps);
            // Il faut qu'il peigne : un test qui parle de `clip_rect` sans
            // jamais faire tourner une image ne mesure rien.
            if !corps.contains("run_ui(") {
                continue;
            }
            if corps.contains("apply_theme") {
                continue;
            }
            // `paint` et `peindre` posent le style eux-mêmes.
            if corps.contains("paint(&mut") || corps.contains("peindre(&mut") {
                continue;
            }
            let mesure = MESURES.iter().find(|motif| corps.contains(**motif));
            if let Some(motif) = mesure {
                fautifs.push(format!(
                    "{fichier} : {nom} lit {motif} sans avoir posé le style"
                ));
            }
        }
    }

    assert!(
        fautifs.is_empty(),
        "{} contrôle(s) qui mesurent sous un style qui n'est pas celui de \
         l'application :\n  {}\n\n\
         Ajoutez `app.apply_theme(&ctx);` après la création du contexte — ou \
         `NamsdecApp::default().apply_theme(&ctx);` si le test dessine un \
         composant isolé.",
        fautifs.len(),
        fautifs.join("\n  ")
    );
}

/// Toute abstention nomme sa raison.
///
/// Un contrôle qui rend la main au lieu de juger — un `return;` nu — ne
/// vérifie rien, et il passe au VERT en ne vérifiant rien. C'est
/// souvent juste : sans nasm, il n'y a rien à assembler, et se taire
/// vaut mieux qu'échouer sur l'absence d'un outil.
///
/// Mais il faut que la raison se LISE, et c'est tout ce que cette règle
/// demande. 7.6 réclamait « un contrôle qui dit ce qui n'est pas
/// testé » : le voici, et il tient à l'envers de ce qu'on croirait —
/// non pas compter ce qui manque, mais refuser qu'un silence n'ait pas
/// de nom.
///
/// Le relevé du 22 octobre 2026 donnait 228 abstentions sur 557
/// contrôles. Rangées par raison, trois n'en avaient aucune, et les
/// trois étaient des défauts :
///   - deux s'abstenaient sur l'absence d'un fichier DU DÉPÔT
///     (`docs/guide.py`, `src/`) — ce qui n'est pas un état de la
///     machine, mais le dépôt qui a bougé : elles crient maintenant ;
///   - la troisième — `the_install_button_brings_its_plan_to_the_front`
///     — s'abstenait PARTOUT, y compris ici, parce qu'elle semait son
///     état avant la première image et qu'ouvrir les préférences jette
///     le relevé. Elle n'avait jamais rien vérifié.
///
/// Trente lignes d'amont, et non trois : l'appel qui justifie
/// l'abstention peut être loin du `return`, un `compile_c` portant
/// vingt lignes de source C entre les deux.
#[test]
fn every_abstention_names_its_reason() {
    // Les raisons de se taire, et ce qui les nomme dans le code.
    //
    // Allonger cette liste demande une raison écrite, et c'est le but :
    // le prochain qui voudra une abstention nouvelle devra dire ici
    // pourquoi se taire est la bonne réponse. Ce qu'on refuse n'est pas
    // le silence, c'est le silence anonyme.
    const RAISONS: [(&str, &str); 10] = [
        ("find_nasm", "sans assembleur, rien à assembler"),
        ("find_linker", "sans éditeur de liens, rien à lier"),
        (
            "binaire_fabriqué",
            "le binaire d'épreuve demande nasm et ld",
        ),
        (
            "binaire_avec_place_libre",
            "le binaire qui déclare sa place libre demande nasm et ld",
        ),
        ("compile_c", "pas de compilateur C sur cette machine"),
        (
            "dossier_dépreuve",
            "le corpus de chacun, qui n'est pas versionné",
        ),
        ("assemble_state", "rien n'a été assemblé, donc rien à lire"),
        (
            "install_plan",
            "sans dossier de données, la fenêtre d'installation se referme seule",
        ),
        ("program_path", "l'outil n'est pas installé"),
        (
            "bash",
            "sans bash, un script d'installation ne se lance pas",
        ),
    ];

    // Les mêmes, sous une forme que le code écrit autrement.
    const AUSSI: [&str; 5] = [
        "\"/bin/",
        "\"/usr/bin",
        "data_dir()",
        "external_abyss",
        ".diagnostics",
    ];

    // Ce contrôle-ci porte les motifs puisqu'il les cherche.
    const MOI: &str = "every_abstention_names_its_reason";

    let mut muets: Vec<String> = Vec::new();
    let mut abstentions = 0usize;
    for (fichier, source) in SOURCES_DU_BANC {
        for morceau in source.split("#[test]\n").skip(1) {
            let Some(nom) = morceau
                .split_once("fn ")
                .and_then(|(_, reste)| reste.split_once('('))
                .map(|(nom, _)| nom.trim())
            else {
                continue;
            };
            if nom == MOI {
                continue;
            }
            let corps = morceau
                .split_once('{')
                .map(|(_, reste)| reste)
                .unwrap_or(morceau);
            let corps = corps.split_once("\n}\n").map(|(c, _)| c).unwrap_or(corps);
            let lignes: Vec<&str> = corps.lines().collect();
            // TOUTES les abstentions du contrôle, et non la première :
            // n'en regarder qu'une les fait sortir du bois l'une après
            // l'autre, et la dernière reste anonyme sans que rien ne le
            // dise.
            for (rang, _) in lignes
                .iter()
                .enumerate()
                .filter(|(_, l)| l.trim() == "return;")
            {
                abstentions += 1;
                let amont = lignes[rang.saturating_sub(30)..=rang].join("\n");
                let nommée = RAISONS.iter().any(|(motif, _)| amont.contains(motif))
                    || AUSSI.iter().any(|motif| amont.contains(motif));
                if !nommée {
                    muets.push(format!(
                        "{fichier} : {nom}\n      …{}",
                        lignes[rang.saturating_sub(2)..=rang]
                            .join("\n      ")
                            .trim_start()
                    ));
                }
            }
        }
    }

    assert!(
        abstentions > 100,
        "le relevé ne trouve presque aucune abstention ({abstentions}) — il \
         lit au mauvais endroit, et cette règle ne garde plus rien"
    );
    assert!(
        muets.is_empty(),
        "{} abstention(s) sans raison nommée — un contrôle qui rend la main \
         sans dire pourquoi passe au vert en ne vérifiant rien :\n  {}\n\n\
         Trois issues, dans cet ordre : SEMER ce qui manque plutôt que de \
         s'en passer ; ÉCHOUER si ce qui manque est un fichier du dépôt ; ou \
         inscrire la raison dans RAISONS, avec sa phrase.",
        muets.len(),
        muets.join("\n  ")
    );
}

/// Aucun contrôle ne lit un chemin de la machine de quelqu'un.
///
/// Trouvé le 22 octobre 2026 en comptant les abstentions : deux
/// contrôles lisaient `/home/fred/CrackMe/crackme2`, le fichier qui
/// avait servi à signaler le défaut de l'éditeur binaire. Ils
/// s'abstenaient partout ailleurs — et l'un des deux était la
/// RÉTENTION de ce défaut. Elle ne retenait rien sur aucune autre
/// machine, et sur la CI non plus, tout en passant au vert.
///
/// C'est la même faute que « demander à la machine ce qu'on devrait
/// semer », vue du côté de l'entrée au lieu de l'assertion : un
/// contrôle dont l'entrée n'existe que chez une personne ne protège
/// qu'elle.
///
/// `/bin/ls` n'est pas visé : il est là sur toute machine POSIX, et le
/// coureur de la CI l'a. Ce qui est visé est le dossier de quelqu'un.
///
/// UN CHEMIN INVENTÉ POUR L'EXEMPLE — un rapport d'anomalie qui ne doit
/// pas trahir le chemin d'un client, une fiche d'outil qui montre une
/// installation ailleurs — s'écrit sous `/home/quelquun/`. Il n'est
/// jamais lu, il est raconté, et la convention le dit d'un coup d'œil.
#[test]
fn no_control_reads_a_path_from_someones_home() {
    // Les racines qui n'appartiennent qu'à une personne.
    const PERSONNELS: [&str; 3] = ["/home/", "/root/", "/Users/"];

    // Le chemin qu'on invente pour l'exemple, et qu'on ne lit pas.
    const INVENTÉ: &str = "/home/quelquun/";

    // Ce contrôle-ci porte les motifs puisqu'il les cherche.
    const MOI: &str = "no_control_reads_a_path_from_someones_home";

    let mut fautifs: Vec<String> = Vec::new();
    for (fichier, source) in SOURCES_DU_BANC {
        for morceau in source.split("#[test]\n").skip(1) {
            let Some(nom) = morceau
                .split_once("fn ")
                .and_then(|(_, reste)| reste.split_once('('))
                .map(|(nom, _)| nom.trim())
            else {
                continue;
            };
            if nom == MOI {
                continue;
            }
            // Le corps seul, et jusqu'à sa propre accolade : un récit
            // peut raconter le chemin qui a servi à chercher, et ce
            // récit est même ce qui doit rester.
            let corps = morceau
                .split_once('{')
                .map(|(_, reste)| reste)
                .unwrap_or(morceau);
            let corps = corps.split_once("\n}\n").map(|(c, _)| c).unwrap_or(corps);
            for racine in PERSONNELS {
                let mut reste = corps;
                while let Some(at) = reste.find(racine) {
                    let depuis = &reste[at..];
                    if !depuis.starts_with(INVENTÉ) {
                        let fin = depuis.find('"').unwrap_or(depuis.len().min(48));
                        fautifs.push(format!("{fichier} : {nom} lit {}", &depuis[..fin]));
                    }
                    reste = &reste[at + racine.len()..];
                }
            }
        }
    }

    assert!(
        fautifs.is_empty(),
        "{} contrôle(s) dont l'entrée n'existe que sur une machine — ils \
         s'abstiennent partout ailleurs, EN PASSANT AU VERT :\n  {}\n\n\
         Fabriquez l'entrée (`binaire_fabriqué`), ou semez-la dans un \
         dossier temporaire. Un chemin inventé pour l'exemple s'écrit sous \
         « {INVENTÉ} », et ne se lit pas.",
        fautifs.len(),
        fautifs.join("\n  ")
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
    // Les contrôles qui ont le droit de lire l'état réel, et pourquoi.
    // Allonger cette liste demande une raison écrite, et c'est le but :
    // le prochain qui voudra « s'adapter à la machine » devra dire ici en
    // quoi son cas ressemble à ceux-là.
    const AUTORISÉS: [(&str, &str); 3] = [
        (
            "the_tools_say_whether_this_machine_has_them",
            "vérifie une RELATION vraie partout : programme introuvable ⇒ fiche « absent »",
        ),
        (
            "what_the_tools_answer_is_asked_once_and_kept",
            "vérifie la mécanique du relevé, non son contenu",
        ),
        (
            "a_located_binary_makes_a_tool_ready",
            "vérifie l'override de localisation, et n'affirme rien sur la machine",
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
    // Les vingt thèmes et cette page, et non la seule source où l'on se
    // trouve : voir [`SOURCES_DU_BANC`].
    for (fichier, source) in SOURCES_DU_BANC {
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
            // parler de `availability_in` sans l'appeler. Et le corps
            // s'arrête à son accolade de colonne zéro, pour ne pas
            // s'approprier les fonctions libres qui le suivent.
            let corps = morceau
                .split_once('{')
                .map(|(_, reste)| reste)
                .unwrap_or(morceau);
            let corps = corps.split_once("\n}\n").map(|(c, _)| c).unwrap_or(corps);
            for lecture in LECTURES {
                if corps.contains(lecture) {
                    fautifs.push(format!("{fichier} : {nom} appelle {lecture}"));
                }
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
            SOURCES_DU_BANC
                .iter()
                .any(|(_, source)| source.contains(&format!("fn {nom}("))),
            "{nom} n'existe plus — retirez-le de la liste (« {raison} »)"
        );
    }
}

/// Déplier les outils de la vue mémoire ne coupe aucun titre en deux.
///
/// Signalé sur capture le 14 octobre 2026 : « Écrire ici » s'écrivait
/// par-dessus la première ligne d'octets du vidage. La mesure image par
/// image a donné la cause : le FONDU d'ouverture d'egui rogne le corps du
/// repli à sa hauteur du moment, si bien que son dernier titre est coupé
/// en deux pendant que le vidage, lui, a déjà pris sa nouvelle place.
/// Relevé : l'encre d'« Écrire ici » s'arrête à 601,8 là où le texte va
/// jusqu'à 615,2, et le vidage reprend à 608. Trois images — invisibles à
/// soixante hertz, très visibles quand une image coûte trois cents
/// millisecondes sur un gros binaire.
///
/// Le contrôle regarde donc TOUTES les images qui suivent le clic, et
/// non la dernière : c'est entre les deux que le défaut vivait. Et il
/// mesure l'ENCRE — ce qui est peint intersecté avec ce qui le découpe —
/// car un titre coupé est un titre dont il manque la moitié, pas un
/// titre absent.
#[test]
fn unfolding_the_memory_tools_never_cuts_a_title() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        active_tab: SidebarTab::Memory,
        show_sidebar: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    for _ in 0..8 {
        app.do_step();
    }
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1100.0));
    // Le texte peint, et l'encre qu'il en reste une fois découpé.
    let image =
        |app: &mut NamsdecApp, events: Vec<egui::Event>| -> Vec<(String, egui::Rect, egui::Rect)> {
            let entrée = egui::RawInput {
                screen_rect: Some(écran),
                events,
                ..Default::default()
            };
            let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
            let mut vus = Vec::new();
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    for rangée in &texte.galley.rows {
                        let ligne = rangée.rect().translate(texte.pos.to_vec2());
                        vus.push((
                            rangée.row.text().to_owned(),
                            ligne,
                            ligne.intersect(clipped.clip_rect),
                        ));
                    }
                }
            }
            sortie.textures_delta.clear();
            vus
        };
    for _ in 0..3 {
        image(&mut app, Vec::new());
    }
    let textes = image(&mut app, Vec::new());
    // L'en-tête EST peint, ou c'est un défaut : c'est lui qu'on vient
    // déplier, et sans lui le contrôle ne mesure rien. Il s'abstenait
    // ici, donc il se serait tu le jour où la section aurait changé de
    // nom — exactement le jour où il sert.
    let (_, entête, _) = textes
        .iter()
        .find(|(t, _, _)| t.contains("Outils mémoire"))
        .unwrap_or_else(|| {
            panic!(
                "« Outils mémoire » n'est pas peint — la section a changé de \
                 nom, ou elle a disparu de l'onglet Mémoire. Textes vus : {:?}",
                textes.iter().map(|(t, _, _)| t).collect::<Vec<_>>()
            )
        });
    let cible = entête.center();
    let clic = |pressed| egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    image(&mut app, vec![egui::Event::PointerMoved(cible), clic(true)]);
    image(&mut app, vec![clic(false)]);

    // Les titres des outils, dans les images qui suivent : chacun est
    // peint en entier, ou pas du tout. Jamais à moitié.
    const TITRES: [&str; 3] = ["Carte de la mémoire", "Chercher en mémoire", "Écrire ici"];
    let mut vu_déplié = false;
    for rang in 0..10 {
        let textes = image(&mut app, Vec::new());
        for (titre, ligne, encre) in &textes {
            if !TITRES.iter().any(|t| titre.contains(t)) {
                continue;
            }
            if encre.height() <= 0.5 {
                // Pas encore peint : le repli n'est pas ouvert.
                continue;
            }
            vu_déplié = true;
            assert!(
                (encre.height() - ligne.height()).abs() < 0.5,
                "image {rang} : « {titre} » est coupé — peint sur {:.1} px, \
                 visible sur {:.1}",
                ligne.height(),
                encre.height()
            );
        }
    }
    assert!(
        vu_déplié,
        "les outils se sont bien dépliés — sans quoi le contrôle ne \
         prouverait rien"
    );
}

/// Rien ne devient cliquable sans le dire.
///
/// « Comment savoir si un texte est cliquable ou pas ? » — la question
/// de Fred, le 15 octobre, et elle n'a de réponse durable que si la
/// marque est posée à UN seul endroit. Sinon chaque panneau neuf
/// réinvente sa façon, et l'on se retrouve avec des textes qui mènent
/// quelque part sans rien montrer, à côté d'autres qui ne mènent nulle
/// part et qui sont bleus.
///
/// La règle : une étiquette ne devient cliquable que par `cliquable`
/// (interface) ou `clickable_label` (panneaux d'analyse), qui posent la
/// main du pointeur et le trait au survol. Une RANGÉE entière, elle,
/// passe par `interact` et doit appeler `souligne_au_survol` dans les
/// lignes qui suivent.
///
/// Le contrôle lit les sources : c'est la seule liste qui ne puisse pas
/// se désynchroniser du code.
#[test]
fn nothing_becomes_clickable_without_saying_so() {
    /// Les lignes où `Sense::click()` est posé sur autre chose qu'une
    /// étiquette — un dessin, un bouton peint à la main, une image —, et
    /// qui portent donc leur marque autrement : un cadre, une pastille,
    /// une touche.
    fn est_un_dessin(contexte: &str) -> bool {
        contexte.contains("allocate_exact_size")
            || contexte.contains("allocate_painter")
            || contexte.contains("allocate_response")
            || contexte.contains("egui::Image::new")
            || contexte.contains("ui.interact(")
    }

    let mut fautifs = Vec::new();
    // Les sources de l'interface ET celles des panneaux d'analyse : la
    // règle vaut des deux côtés, et c'est tout l'intérêt de l'écrire
    // deux fois plutôt que de laisser chaque panneau décider.
    //
    // Celles des panneaux sont LUES sur le disque, comme le fait déjà
    // `everything_the_emulator_runs_has_a_sheet` : les inclure une à une
    // reviendrait à tenir une liste que personne ne mettrait à jour, et
    // c'est justement le panneau neuf qu'on veut attraper.
    let racine = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../namsdec-re/src"));
    let mut panneaux: Vec<(String, String)> = Vec::new();
    for entrée in std::fs::read_dir(racine).expect("les sources des panneaux") {
        let chemin = entrée.expect("une entrée").path();
        if chemin.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let nom = chemin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        panneaux.push((nom, std::fs::read_to_string(&chemin).expect("lisible")));
    }
    assert!(
        panneaux.len() > 20,
        "les sources des panneaux doivent être lues : {}",
        panneaux.len()
    );
    let panneaux: Vec<(&str, &str)> = panneaux
        .iter()
        .map(|(nom, source)| (nom.as_str(), source.as_str()))
        .collect();
    for (nom, source) in SOURCES_DE_L_INTERFACE.iter().chain(panneaux.iter()) {
        let lignes: Vec<&str> = source.lines().collect();
        for (rang, ligne) in lignes.iter().enumerate() {
            let pose_un_clic = ligne.contains("Sense::click()");
            if !pose_un_clic {
                continue;
            }
            // Ce qui précède et ce qui suit : une pose de clic s'écrit
            // sur plusieurs lignes, et la marque se lit juste après.
            let avant = lignes[rang.saturating_sub(8)..=rang].join("\n");
            let après = lignes[rang..(rang + 10).min(lignes.len())].join("\n");
            let entouré = format!("{avant}\n{après}");
            // Les deux helpers eux-mêmes, et les dessins.
            if entouré.contains("souligne_au_survol")
                || entouré.contains("cliquable(ui")
                || entouré.contains("clickable_label(")
                || est_un_dessin(&entouré)
            {
                continue;
            }
            fautifs.push(format!("{nom}:{} — {}", rang + 1, ligne.trim()));
        }
    }
    assert!(
        fautifs.is_empty(),
        "ces textes se cliquent sans le dire — passez par `cliquable` / `clickable_label`, \
         ou appelez `souligne_au_survol` si c'est une rangée :\n{}",
        fautifs.join("\n")
    );
}

/// Une fenêtre tient dans l'écran — jusqu'au plus petit que
/// l'application accepte.
///
/// Le banc dessine sur 1600x1100, et c'est là que toutes les règles de
/// mise en page ont été éprouvées. Mais l'application déclare accepter
/// 800x520 (`MIN_SCREEN`), et personne ne regardait ce format : à
/// 800x520, trois fenêtres dépassaient l'écran EN HAUT, c'est-à-dire du
/// mauvais côté — la calculatrice de 236 px, « À propos » de 79, les
/// registres étendus de 68. Posées au centre avec un pivot centré, une
/// fenêtre plus haute que l'écran répartit son débord moitié en haut,
/// moitié en bas ; et ces trois-là n'ont pas de barre de titre d'egui
/// mais leur propre bandeau, qui porte la croix de fermeture. Le
/// bandeau hors de vue, la fenêtre ne se déplace plus et ne se ferme
/// plus à la souris.
///
/// La calculatrice portait même, en commentaire, l'intention inverse :
/// « Jamais plus haute que l'écran : une fenêtre qui le dépasse pose son
/// bouton hors de vue ». Elle borne bien sa hauteur par
/// `max_height`, mais `max_height` borne ce que la POIGNÉE permet, non
/// ce que le contenu prend : un contenu qui ne défile pas pousse le
/// cadre au-delà, et la fenêtre mesure 756 px là où elle s'était
/// promis 460.
///
/// On mesure le cadre et non le contenu : une fenêtre dont le cadre
/// tient dans l'écran a son bandeau atteignable, et le reste est
/// l'affaire des invariants de découpage.
#[test]
fn a_window_fits_the_smallest_screen_the_app_accepts() {
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, MIN_SCREEN.into());
    let mut dehors: Vec<String> = Vec::new();
    for (nom, id, ouvre) in scènes() {
        let Some(id) = id else { continue };
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        };
        // Douze images : une fenêtre dont la hauteur épouse son contenu
        // n'a pas fini de se poser avant la neuvième.
        let mut cadre = egui::Rect::NOTHING;
        for _ in 0..12 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            cadre = egui::AreaState::load(&ctx, egui::Id::unique(id))
                .map(|a| a.rect())
                .unwrap_or(egui::Rect::NOTHING);
            sortie.textures_delta.clear();
        }
        let manque = egui::vec2(
            (cadre.right() - écran.right()).max(0.0) + (écran.left() - cadre.left()).max(0.0),
            (cadre.bottom() - écran.bottom()).max(0.0) + (écran.top() - cadre.top()).max(0.0),
        );
        if manque.x > 1.0 || manque.y > 1.0 {
            dehors.push(format!(
                "« {nom} » : {:.0}x{:.0} posée en ({:.0},{:.0}) — {:.0} px hors de \
                 l'écran en largeur et {:.0} px en hauteur",
                cadre.width(),
                cadre.height(),
                cadre.left(),
                cadre.top(),
                manque.x,
                manque.y
            ));
        }
    }
    assert!(
        dehors.is_empty(),
        "à {:.0}x{:.0}, le plus petit écran que l'application accepte, \
         {} fenêtre(s) sortent de l'écran :\n  {}",
        MIN_SCREEN[0],
        MIN_SCREEN[1],
        dehors.len(),
        dehors.join("\n  ")
    );
}

/// Une fenêtre qui montre un contenu de taille connue ne défile pas
/// quand l'écran a la place.
///
/// Le revers de l'invariant précédent. Trois fenêtres ont dû recevoir
/// une hauteur d'ouverture ÉCRITE, parce qu'une zone qui défile ne
/// déborde jamais et qu'une fenêtre qui ne déborde pas ne grandit pas :
/// il faut lui dire ce qu'elle a à montrer. Un nombre écrit à la main
/// est un nombre qui vieillit — le jour où les registres étendus
/// gagneront une ligne, la fenêtre ne la montrera plus et se mettra à
/// défiler sur un écran qui avait la place. Ce contrôle est ce qui
/// l'empêche, et c'est lui que nomme le commentaire de
/// `VECTEURS_HAUTEUR`.
///
/// Trois fenêtres et pas toutes : l'analyse, l'aide, le désassemblage ou
/// la console montrent des listes sans longueur connue, et elles
/// DOIVENT défiler. Celles-ci montrent un contenu de taille fixe — un
/// appareil, vingt-quatre registres, une boîte de renseignements.
#[test]
fn a_window_of_known_content_does_not_scroll_when_there_is_room() {
    let entrée = écran_du_banc();
    let mut défilent: Vec<String> = Vec::new();
    for (nom, id, ouvre) in scènes() {
        let Some(id) = id else { continue };
        if !nom.starts_with("calculatrice")
            && !nom.starts_with("à propos")
            && !nom.starts_with("registres étendus")
        {
            continue;
        }
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        let mut barres: Vec<f32> = Vec::new();
        for _ in 0..12 {
            let cadre = egui::AreaState::load(&ctx, egui::Id::unique(id))
                .map(|a| a.rect())
                .unwrap_or(egui::Rect::NOTHING);
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            barres.clear();
            for clipped in &sortie.shapes {
                let large = cadre.x_range().expand(2.0);
                if !large.contains(clipped.clip_rect.left())
                    || !large.contains(clipped.clip_rect.right())
                {
                    continue;
                }
                // Une barre de défilement : étroite et haute. Le même
                // relevé que `a_scroll_bar_shows_up_when_there_is_…`,
                // qui ne regardait que les panneaux.
                if let egui::epaint::Shape::Rect(r) = &clipped.shape
                    && r.rect.width() < 20.0
                    && r.rect.height() > 60.0
                {
                    barres.push(r.rect.height());
                }
            }
            sortie.textures_delta.clear();
        }
        if !barres.is_empty() {
            défilent.push(format!(
                "« {nom} » montre {} barre(s) de défilement ({} px) sur un écran \
                 de {:.0}x{:.0} : sa hauteur d'ouverture écrite ne couvre plus ce \
                 qu'elle a à montrer",
                barres.len(),
                barres
                    .iter()
                    .map(|h| format!("{h:.0}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                entrée.screen_rect.unwrap().width(),
                entrée.screen_rect.unwrap().height(),
            ));
        }
    }
    assert!(défilent.is_empty(), "{}", défilent.join("\n  "));
}

/// Chaque scène d'onglet montre bien l'onglet qu'elle nomme.
///
/// Trouvé par un relevé des tailles de police, et c'est ce qui le rend
/// intéressant : il cherchait pourquoi le gros bouton d'action — le
/// seul texte en 14 pt de l'application — n'était peint par aucune
/// scène. Parce que la scène qui aurait dû le peindre ne montrait pas
/// ce qu'elle disait montrer. L'onglet RE est masqué par défaut
/// (Préférences > Modules), et `sidebar_ui` rabat l'onglet actif sur le
/// processeur quand on le demande sans l'avoir allumé — en silence,
/// comme il se doit pour quelqu'un qui vient d'éteindre le module.
///
/// Les deux moitiés du banc mesuraient donc l'onglet Processeur sous le
/// nom de l'onglet RE. Tous les invariants de scène passaient : ils
/// regardaient un onglet, simplement pas celui qu'ils nommaient.
///
/// Le contrôle demande à l'application ce qu'elle montre APRÈS avoir
/// dessiné, et non ce qu'on lui a demandé de montrer — c'est tout
/// l'écart entre les deux qui était passé inaperçu.
#[test]
fn chaque_scene_d_onglet_montre_bien_son_onglet() {
    let entrée = écran_du_banc();
    for onglet in SidebarTab::ALL {
        let nom = format!("application / {}", onglet.id());
        let Some((_, _, ouvre)) = scènes().into_iter().find(|(n, _, _)| *n == nom) else {
            panic!("la scène « {nom} » doit exister dans le banc");
        };
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        for _ in 0..3 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            sortie.textures_delta.clear();
        }
        assert_eq!(
            app.active_tab, onglet,
            "« {nom} » dit montrer {:?} et montre {:?} : la scène mesure un autre \
             onglet que celui qu'elle nomme",
            onglet, app.active_tab
        );
    }
}

/// Chaque scène de vue montre bien la vue qu'elle nomme.
///
/// Le même piège que pour les onglets du panneau, et il s'est refermé
/// une fois déjà : une vue indisponible — la décompilation, dont le
/// module s'allume dans les préférences — fait retomber la zone
/// centrale sur la source, en silence. La scène « vue / decompiler »
/// mesurait alors l'éditeur sous un autre nom, et aucun invariant ne
/// rougissait.
///
/// On le demande à l'application APRÈS qu'elle a dessiné, et non à la
/// scène qui l'a réglée : c'est la seule façon de voir un repli
/// silencieux.
#[test]
fn chaque_scene_de_vue_montre_bien_sa_vue() {
    let entrée = écran_du_banc();
    for vue in VueCentrale::ALL {
        let nom = format!("vue / {}", vue.id());
        let Some((_, _, ouvre)) = scènes().into_iter().find(|(n, _, _)| *n == nom) else {
            panic!("la scène « {nom} » doit exister dans le banc");
        };
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        for _ in 0..3 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            sortie.textures_delta.clear();
        }
        assert_eq!(
            app.vue_centrale, vue,
            "« {nom} » dit montrer {:?} et montre {:?} : la scène mesure une autre \
             vue que celle qu'elle nomme",
            vue, app.vue_centrale
        );
    }
}

/// Les commandes de la barre d'outils ont toutes la même hauteur.
///
/// Mesuré dans une seule rangée, celle qu'on a sous les yeux toute la
/// journée : « Assembler » 25 px, puis « Exécuter », « Step Into »,
/// « Step Over », « Step Back », « Stop » et « Réinitialiser » à
/// 18 px, puis les cinq onglets à 25 px. Les commandes dont on se sert
/// le plus étaient les plus petites de la barre, et sept pixels
/// séparaient deux voisines immédiates.
///
/// La cause tenait en un mot : `icon_button` demandait
/// `interact_size.y` — la hauteur d'un widget quelconque, 18 px —
/// quand un bouton de cette application vaut son texte plus deux fois
/// son rembourrage, soit 25. Un bouton à icône n'est pas une autre
/// sorte de bouton.
///
/// Dix-huit pixels, c'est aussi six de moins que la cible de
/// vingt-quatre que recommandent les règles d'accessibilité pour une
/// zone qu'on vise à la souris.
///
/// La mesure porte sur les rectangles peints de la barre, et non sur
/// les formes des icônes qu'ils contiennent : on groupe par bande
/// horizontale, et l'on exige une seule hauteur par bande.
#[test]
fn every_command_of_the_toolbar_has_the_same_height() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = écran_du_banc();
    let mut hauteurs: Vec<(f32, f32)> = Vec::new();
    for _ in 0..4 {
        hauteurs.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Rect(r) = &clipped.shape {
                // La barre d'outils : la bande sous la barre de menus,
                // au-dessus de l'éditeur. On prend les rectangles de la
                // taille d'un bouton, et rien d'autre.
                let dans_la_barre = r.rect.top() > 24.0 && r.rect.bottom() < 70.0;
                // Entre 24 et 300 px de large : au-delà, c'est le fond
                // de la barre elle-même, qui court sur tout l'écran et
                // n'est pas une commande.
                if dans_la_barre
                    && (24.0..300.0).contains(&r.rect.width())
                    && r.rect.height() > 12.0
                {
                    hauteurs.push((r.rect.left(), r.rect.height()));
                }
            }
        }
        sortie.textures_delta.clear();
    }
    hauteurs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut vues: Vec<f32> = hauteurs.iter().map(|(_, h)| *h).collect();
    vues.dedup();
    assert!(
        !vues.is_empty(),
        "le relevé ne trouve aucune commande dans la barre d'outils : \
         il ne mesure plus ce qu'il croit mesurer"
    );
    assert_eq!(
        vues.len(),
        1,
        "la barre d'outils mêle {} hauteurs de commande : {:?}",
        vues.len(),
        hauteurs
            .iter()
            .map(|(x, h)| format!("x={x:.0} → {h:.0} px"))
            .collect::<Vec<_>>()
    );
    assert!(
        vues[0] >= 24.0,
        "une commande de la barre d'outils fait {:.0} px de haut : \
         sous les vingt-quatre que l'on vise à la souris",
        vues[0]
    );
}

/// Toute taille de texte peinte appartient à un jeu déclaré.
///
/// Un inventaire, et non une règle de plus : l'application n'a pas de
/// table de styles — chaque endroit écrit sa taille, ou laisse egui
/// choisir la sienne. Rien n'empêchait donc un 15 pt de paraître dans
/// un coin, et deux l'ont fait : le titre d'un module d'analyse
/// grossissait de treize à quinze dès qu'on cliquait dessus — même
/// texte, même fenêtre, même place —, et la page Outils des
/// préférences ouvrait deux points au-dessus des quatre autres.
///
/// Les deux avaient été relevés en septembre et rangés comme
/// « inatteignables par le banc ». Ils ne l'étaient pas : ils étaient
/// peints cinquante-deux fois, dans vingt-cinq scènes. Ce qui manquait
/// n'était pas une scène, c'était ce contrôle-ci.
///
/// Le jeu déclaré, et pourquoi chaque taille y est :
///   * **9** — le détail d'un état vide, l'explication sous une
///     phrase, le libellé d'un travail en cours. `TextStyle::Small`.
///   * **13** — tout le reste : prose, étiquettes, valeurs, titres de
///     section, libellés de boutons, titres de fenêtres.
///     `TextStyle::Body`, `Button` et `Monospace`.
///   * **12** et **14** — la vignette d'un thème, et elle seule : son
///     nom en 14, son « actif » en 12. `theme_card_width` calcule la
///     largeur des vignettes sur ces deux tailles exactement ; les
///     changer sans la changer décalerait les boutons « Retirer ».
///   * **14** aussi pour le gros bouton d'action du panneau RE, le seul
///     bouton de l'application qui porte une icône et une phrase.
///   * **28** — le nom « namsdec » dans « À propos », une fois.
///
/// Les tailles de la CHASSE FIXE ne sont pas ici : la calculatrice et
/// le graphe sont deux appareils qui comptent en colonnes de
/// caractères, et leurs tailles tiennent à leur géométrie, pas à une
/// hiérarchie de texte. C'est un relevé à part, et il reste à faire.
#[test]
fn every_text_size_belongs_to_a_declared_set() {
    const DÉCLARÉES: &[f32] = &[9.0, 12.0, 13.0, 14.0, 28.0];
    let entrée = écran_du_banc();
    let mut étrangères: Vec<String> = Vec::new();
    for (nom, id, ouvre) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        ouvre(&mut app);
        for image in 0..6 {
            let cadre = id
                .and_then(|id| egui::AreaState::load(&ctx, egui::Id::unique(id)).map(|a| a.rect()));
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            if image < 5 {
                sortie.textures_delta.clear();
                continue;
            }
            for clipped in &sortie.shapes {
                if cadre.is_some_and(|c| {
                    let large = c.x_range().expand(2.0);
                    !large.contains(clipped.clip_rect.left())
                        || !large.contains(clipped.clip_rect.right())
                }) {
                    continue;
                }
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                // Ce qui se voit : une ligne rognée à néant par un
                // ascenseur n'est pas peinte, et n'a pas à être jugée.
                if t.galley.rows.iter().all(|l| {
                    l.glyphs.is_empty()
                        || !l
                            .rect_without_leading_space()
                            .translate(t.pos.to_vec2())
                            .intersect(clipped.clip_rect)
                            .is_positive()
                }) {
                    continue;
                }
                for section in &t.galley.job.sections {
                    let police = &section.format.font_id;
                    if police.family != egui::FontFamily::Proportional {
                        continue;
                    }
                    if DÉCLARÉES.iter().any(|d| (d - police.size).abs() < 0.01) {
                        continue;
                    }
                    let texte: String = t.galley.text().chars().take(40).collect();
                    let ligne = format!("{:.1} pt — « {nom} » : {texte:?}", police.size);
                    if !étrangères.contains(&ligne) {
                        étrangères.push(ligne);
                    }
                }
            }
            sortie.textures_delta.clear();
        }
    }
    étrangères.sort();
    assert!(
        étrangères.is_empty(),
        "{} taille(s) de texte hors du jeu déclaré {DÉCLARÉES:?} :\n  {}",
        étrangères.len(),
        étrangères.join("\n  ")
    );
}

/// Et les panneaux ne se recouvrent pas non plus sur le plus petit
/// écran.
///
/// Le banc dessine sur 1600x1100 ; l'application accepte 800x520, et
/// c'est là, et là seulement, que la barre d'outils et la rangée
/// d'onglets du panneau latéral se peignaient L'UNE SUR L'AUTRE :
/// « Step Back », « Stop » et « Réinitialiser » sous « Processeur »,
/// « Pile » et « Mémoire », des lettres sur des lettres. Une
/// disposition de droite à gauche ne recule pas devant le manque de
/// place : elle part du bord droit et peint par-dessus.
///
/// Vu sur une capture, et seulement là. La règle existait depuis des
/// mois (`no_two_texts_of_a_scene_ever_overlap`) ; ce qui manquait
/// n'était pas la règle, c'était la taille.
///
/// LES SCÈNES À FENÊTRE SONT HORS DE CE CONTRÔLE, et c'est une limite
/// assumée plutôt qu'un oubli. Une fenêtre flottante couvre ce qu'il y
/// a derrière elle — c'est son métier — et il faut donc savoir
/// distinguer les couches. Deux façons ont été essayées et mesurées :
/// trier sur la LARGEUR du découpage, qui marche à 1600 px où une
/// fenêtre est plus étroite que l'écran et ne marche plus à 800 px où
/// elle l'occupe presque entièrement ; ranger chaque texte sous le
/// cadre de la fenêtre qui le contient, qui échoue parce qu'egui laisse
/// à une fenêtre un cadre parfois plus petit que ce qu'elle peint — les
/// phrases des préférences retombaient dans le fond. Le découpage ne
/// suffit pas non plus : la rangée d'onglets, posée de droite à gauche,
/// a le sien, distinct de celui de la barre d'outils, alors que les
/// deux sont dans le même panneau.
///
/// Restent les scènes sans fenêtre, où tout est panneau et où la
/// question ne se pose pas. C'est là que vit le défaut, et c'est tout
/// ce que ce contrôle prétend couvrir.
/// À 800 x 520, les onglets du panneau sont peints EN ENTIER.
///
/// Quand la barre d'outils n'a plus la place de les porter au bout de
/// sa rangée, ils passent à la ligne (voir `toolbar_ui`). Ce repli n'a
/// qu'un invariant jusqu'ici — ne rien recouvrir —, et c'est
/// l'invariant d'un évitement : une rangée qui ne se peint pas du tout
/// ne recouvre rien non plus.
///
/// Il en fallait donc un qui exige la PRÉSENCE. Le 25 octobre 2026, une
/// capture a fait croire une demi-journée que ces cinq onglets étaient
/// invisibles à cette taille, et le banc disait le contraire ; c'est le
/// banc qui avait raison — la capture montrait une bande laissée par un
/// redimensionnement, que rien n'avait repeinte. Un invariant écrit
/// alors aurait tranché en une minute.
///
/// La mesure compare chaque étiquette à ELLE-MÊME au large, où rien ne
/// la rogne : un onglet repoussé hors du bord n'est pas absent, il est
/// rogné, et six pixels d'encre passent pour un onglet entier.
#[test]
fn the_sidebar_tabs_are_painted_whole_on_the_smallest_screen() {
    /// L'encre de chaque onglet du panneau, sur un écran donné.
    fn encres(largeur: f32, hauteur: f32) -> Vec<(SidebarTab, egui::Rect)> {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, hauteur),
            )),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        let s = strings(app.lang);
        let (_, lignes) = ce_qui_est_écrit_sur(&ctx, &mut app, None, 4, entrée);
        let mut encres = Vec::new();
        for onglet in SidebarTab::ALL {
            // L'onglet RE est masqué par défaut : il n'a pas à être là.
            if onglet == SidebarTab::Re {
                continue;
            }
            for (texte, rect) in &lignes {
                if texte == onglet.label(s) {
                    encres.push((onglet, *rect));
                }
            }
        }
        encres
    }

    let s = strings(Lang::Fr);
    let large = encres(1600.0, 1100.0);
    let petit = encres(MIN_SCREEN[0], MIN_SCREEN[1]);
    for onglet in SidebarTab::ALL {
        if onglet == SidebarTab::Re {
            continue;
        }
        let nom = onglet.label(s);
        let au_large = large
            .iter()
            .find(|(o, _)| *o == onglet)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("l'onglet « {nom} » doit être peint au large"));
        let posés: Vec<egui::Rect> = petit
            .iter()
            .filter(|(o, _)| *o == onglet)
            .map(|(_, r)| *r)
            .collect();
        assert_eq!(
            posés.len(),
            1,
            "l'onglet « {nom} » doit être peint une fois à {} x {} : la \
             rangée de repli existe pour cela",
            MIN_SCREEN[0],
            MIN_SCREEN[1]
        );
        let rect = posés[0];
        assert!(
            rect.width() >= au_large.width() - 0.5,
            "l'onglet « {nom} » est rogné à l'étroit : {:.0} px d'encre \
             contre {:.0} au large",
            rect.width(),
            au_large.width()
        );
        assert!(
            rect.right() <= MIN_SCREEN[0] && rect.left() >= 0.0,
            "l'onglet « {nom} » sort de l'écran : {rect:?}"
        );
        assert!(
            rect.bottom() <= MIN_SCREEN[1] && rect.top() >= 0.0,
            "l'onglet « {nom} » sort de l'écran par le haut ou le bas : \
             {rect:?}"
        );
    }
}

#[test]
fn panels_do_not_overlap_on_the_smallest_screen() {
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            MIN_SCREEN.into(),
        )),
        ..Default::default()
    };
    let mut vues = 0;
    for (nom, id, prépare) in scènes() {
        if id.is_some() {
            continue;
        }
        vues += 1;
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        prépare(&mut app);
        let (_, textes) = ce_qui_est_écrit_sur(&ctx, &mut app, None, 4, entrée.clone());
        for (i, (texte_a, a)) in textes.iter().enumerate() {
            for (texte_b, b) in &textes[i + 1..] {
                // Deux pixels de tolérance : les jambages d'une ligne
                // frôlent les hampes de la suivante sans se gêner.
                assert!(
                    !a.shrink(2.0).intersects(b.shrink(2.0)),
                    "à {:.0}x{:.0}, « {nom} » peint « {texte_a} » sur « {texte_b} »\
                     \n  {a:?}\n  {b:?}",
                    MIN_SCREEN[0],
                    MIN_SCREEN[1],
                );
            }
        }
    }
    assert!(
        vues >= 5,
        "le contrôle n'a regardé que {vues} scène(s) sans fenêtre : \
         il ne mesure plus ce qu'il croit mesurer"
    );
}

/// Un profil neuf ouvre sur l'onglet que le code annonce.
///
/// `SidebarTab::Cpu` porte en commentaire « en tête, et vue par
/// défaut » ; la lecture des réglages répondait `Stack`. Sur un profil
/// neuf — le premier écran d'un nouveau venu — l'onglet choisi était
/// donc la pile, qui n'a rien à montrer tant qu'on n'a pas assemblé,
/// et le premier onglet de la rangée n'était pas celui qui était
/// allumé. Vu sur la capture du premier lancement.
///
/// Le contrôle part des réglages VIDES et non d'un `NamsdecApp` nu :
/// c'est le chemin que prend un profil neuf, et c'est là qu'était
/// l'écart.
#[test]
fn a_fresh_profile_opens_on_the_tab_the_code_announces() {
    let app = NamsdecApp::from_settings(Settings::default());
    assert_eq!(
        app.active_tab,
        SidebarTab::Cpu,
        "un profil neuf doit ouvrir sur l'onglet que `SidebarTab::Cpu` \
         annonce comme la vue par défaut"
    );
}

/// Sur la page Plugins, ce qui parle des thèmes est écrit avec les
/// thèmes.
///
/// « Aucun thème installé pour l'instant — les quatre thèmes d'origine
/// ne se retirent pas. » était peinte en DERNIER, sous le titre
/// « Séries d'exercices » et sous l'explication du fichier d'une série.
/// Une phrase rangée sous la mauvaise section ne se mesure pas : elle
/// se voit. Celle-ci a été trouvée sur une capture.
///
/// Le contrôle compare deux ordonnées peintes, ce qui est tout ce dont
/// il s'agit : la phrase des thèmes au-dessus du titre des exercices.
#[test]
fn what_speaks_of_themes_is_written_with_the_themes() {
    for lang in [Lang::Fr, Lang::En] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            lang,
            show_preferences: true,
            preferences_tab: PreferencesTab::Plugins,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let s = strings(lang);
        let (_, textes) = ce_qui_est_écrit(&ctx, &mut app, Some("preferences_window"), 4);
        let y_de = |cherché: &str| {
            textes
                .iter()
                .find(|(t, _)| t.starts_with(cherché))
                .map(|(_, r)| r.top())
        };
        let (Some(thèmes), Some(exercices)) = (
            y_de(s.plugins_none_installed),
            y_de(s.exercises_packs_title),
        ) else {
            panic!("la page Plugins doit peindre les deux sections");
        };
        assert!(
            thèmes < exercices,
            "« {} » est peinte à y={thèmes:.0}, sous « {} » à y={exercices:.0} : \
             elle parle des thèmes et se range avec les exercices",
            s.plugins_none_installed,
            s.exercises_packs_title
        );
    }
}

/// Un repli s'annonce par un chevron, jamais par une lettre.
///
/// Les trois en-têtes repliables du panneau écrivaient « v » et « > » —
/// deux lettres de l'alphabet. Sur la capture, le « v » gris posé devant
/// « Prochaine instruction » ne se lit pas comme un signe : c'est une
/// lettre orpheline, et rien ne dit que le titre se replie. Le défaut
/// est de ceux qu'aucune mesure ne trouve, parce que tout est aligné,
/// contrasté et entier — il faut avoir regardé l'écran.
///
/// On demande donc les chevrons d'egui, U+23F7 et U+23F5. Le choix
/// n'est pas libre : la police ne porte ni ▾, ni ▼, ni ⌄: interrogée
/// par `has_glyph`, elle répond non à tous les trois, et un caractère
/// absent se peint en carré vide.
#[test]
fn a_fold_is_announced_by_a_chevron_never_by_a_letter() {
    for ouvert in [true, false] {
        let ctx = egui::Context::default();
        NamsdecApp::default().apply_theme(&ctx);
        let mut état = ouvert;
        let mut écrits: Vec<String> = Vec::new();
        let mut sortie = ctx.run_ui(egui::RawInput::default(), |ui| {
            section_toggle(ui, DEFAULT_THEME, "Prochaine instruction", &mut état);
        });
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                écrits.push(t.galley.text().to_owned());
            }
        }
        sortie.textures_delta.clear();
        let attendu = if ouvert { "⏷" } else { "⏵" };
        assert!(
            écrits.iter().any(|texte| texte == attendu),
            "le repli {} doit s'annoncer par « {attendu} » — écrit : {écrits:?}",
            if ouvert { "ouvert" } else { "fermé" }
        );
        for lettre in ["v", ">", "V", "<"] {
            assert!(
                !écrits.iter().any(|texte| texte == lettre),
                "« {lettre} » n'est pas un chevron : c'est une lettre, et elle \
                 se lit comme telle"
            );
        }
        // Et le caractère existe vraiment dans la police : un chevron
        // absent se peint en carré vide, ce qui est pire que la lettre.
        let police = egui::FontId::proportional(13.0);
        for c in attendu.chars() {
            assert!(
                ctx.fonts_mut(|f| f.has_glyph(&police, c)),
                "la police n'a pas U+{:04X} — il se peindrait en carré vide",
                c as u32
            );
        }
    }
}

/// Rend chaque scène du banc en image, pour qu'on puisse la REGARDER.
///
/// « Il y a trois endroits où mesurer : le banc, la capture, et
/// l'application en marche. » La capture manquait pour tout ce qui
/// n'apparaît qu'au clavier — la palette de commandes, les suites d'une
/// séquence armée —, et elle manquait pour une raison qui ne se répare
/// pas côté namsdec : **sous Wayland, l'injection de touches par XTEST
/// n'atteint pas la fenêtre**, et GNOME ne donne pas le focus à une
/// fenêtre qu'on vient de lancer. Vérifié avec un témoin : `xdotool
/// type` n'écrit rien dans l'éditeur.
///
/// Ici il n'y a ni serveur d'affichage ni compositeur : egui_kittest
/// rend la scène par wgpu sur lavapipe — Vulkan logiciel — et l'on
/// écrit le PNG. Pas d'écran, pas de carte graphique, pas de droits, et
/// le même résultat sur toute machine.
///
/// IGNORÉ PAR DÉFAUT : il écrit une trentaine de fichiers et compile un
/// pilote graphique entier. Il se demande :
///
/// ```text
/// cargo test -p namsdec-gui -- --ignored --nocapture captures_de_toutes_les_scenes
/// ```
///
/// Les images vont dans `target/captures/`, qui n'est pas suivi.
#[test]
#[ignore = "écrit des images ; se demande explicitement"]
fn captures_de_toutes_les_scenes() {
    use egui_kittest::Harness;

    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/captures");
    std::fs::create_dir_all(&dossier).expect("le dossier des captures se crée");

    // NE RENDRE QUE CE QU'ON REGARDE.
    //
    // Les cent vingt-deux scènes prennent plusieurs minutes et un
    // pilote graphique logiciel entier. Quand on vient de toucher une
    // seule mise en page, on la rend seule :
    //
    // ```text
    // CAPTURES=établi cargo test -p namsdec-gui -- --ignored --nocapture captures_de_toutes_les_scenes
    // ```
    //
    // Sans la variable, tout est rendu — un filtre qui s'oublierait
    // ferait un banc qui ne regarde plus rien.
    let filtre = std::env::var("CAPTURES").unwrap_or_default();
    // ET LE THÈME SE DEMANDE AUSSI.
    //
    // Les scènes rendent l'application telle qu'un profil neuf la
    // porte, donc dans un seul thème. Or une palette ne se juge pas
    // sur ses nombres : le banc mesure les contrastes de chaque thème
    // sur chaque onglet, et il a raison, mais il ne dit rien de ce que
    // l'œil reçoit — un vert de givre au milieu de seize registres, ou
    // deux ors qui se ressemblent une fois peints côte à côte.
    //
    // ```text
    // CAPTURES_THEME=noel CAPTURES=établi cargo test -p namsdec-gui -- --ignored --nocapture captures_de_toutes_les_scenes
    // ```
    //
    // Un identifiant inconnu est REFUSÉ plutôt que remplacé : `by_id`
    // retombe sur le thème par défaut, et l'on jugerait alors une
    // autre palette que celle qu'on croit regarder.
    let thème_demandé = std::env::var("CAPTURES_THEME").unwrap_or_default();
    if !thème_demandé.is_empty() {
        assert_eq!(
            namsdec_theme::by_id(&thème_demandé).id,
            thème_demandé,
            "CAPTURES_THEME : « {thème_demandé} » n'est pas un thème connu"
        );
    }

    let mut écrites = 0;
    let mut refusées = Vec::new();
    for (nom, _, prépare) in scènes() {
        if !filtre.is_empty() && !nom.contains(&filtre) {
            continue;
        }
        let mut app = NamsdecApp::default();
        // La scène se pose UNE fois, avant la première image : `prépare`
        // écrit dans l'application, et la rejouer à chaque image
        // remettrait l'état à zéro sous ce qu'on regarde. C'est ce que
        // fait le relevé du banc, et pour la même raison.
        prépare(&mut app);
        // L'ÉCLAT DES VALEURS QUI VIENNENT DE CHANGER EST ÉTEINT, et
        // c'est ce qui rend ces images comparables.
        //
        // `flash_fraction` lit une horloge MURALE
        // (`Instant::elapsed`) : l'orange tient `FLASH_SECONDS`
        // secondes après un pas d'exécution, puis s'estompe. La scène
        // avance de huit pas dans `prépare`, et l'on rend six images
        // aussitôt après — à la vitesse de la machine. Selon qu'elle
        // est chargée ou non, RCX se peignait donc en orange plein ou
        // en gris, et `vue / flow` différait D'UN TIRAGE À L'AUTRE
        // SANS QUE LE CODE CHANGE. Mesuré en rendant deux fois de
        // suite : une capture sur 122 bougeait toute seule.
        //
        // Une image qui change d'elle-même ne prouve rien de ce qu'on
        // lui demande — et pire, elle fait douter des 121 autres. Le
        // banc compare des MISES EN PAGE, pas des animations à
        // mi-course.
        app.values_changed_at = None;
        if !thème_demandé.is_empty() {
            app.theme = namsdec_theme::by_id(&thème_demandé);
        }
        // Relevé AVANT le déplacement d'`app` : `theme` est un
        // `&'static Theme`, donc il survit à la closure.
        let attendu = app.theme;
        let taille = écran_du_banc()
            .screen_rect
            .map(|r| r.size())
            .unwrap_or(egui::vec2(1600.0, 1100.0));
        // LE THÈME, et c'est tout le sujet : `NamsdecApp::default()`
        // porte sa palette, mais rien ne l'a posée dans le contexte.
        // Sans ce `apply_theme`, les images rendues ici montraient les
        // `Visuals` d'egui — fond gris 27, boutons gris 60, espacements
        // par défaut — et non l'application. Les 107 captures du
        // 27 octobre 2026 ont toutes été jugées sur cette interface-là,
        // qui n'existe nulle part : le reste du banc, lui, passe par
        // `chassis`, qui l'appelle depuis toujours.
        //
        // Posé DANS la closure parce qu'`app` y est déplacée, et à
        // chaque image parce que `set_visuals` ne prend qu'à la
        // suivante : le `Ui` reçu tient déjà le style de l'image en
        // cours.
        let mut harness = Harness::builder()
            .with_size(taille)
            .wgpu()
            .build_ui(move |ui| {
                app.apply_theme(ui.ctx());
                app.draw(ui)
            });
        // Plusieurs images : une surface ancrée ne se place qu'après
        // s'être mesurée, et une fenêtre est AMENÉE à sa taille.
        for _ in 0..6 {
            harness.step();
        }
        // Et l'on vérifie que la palette est bien celle du thème, au
        // lieu de le croire : c'est l'écart exact qui a rendu les
        // captures trompeuses, et il revient si quelqu'un remet
        // `build_ui(move |ui| app.draw(ui))`.
        let fond = harness.ctx.style_of(egui::Theme::Dark).visuals.panel_fill;
        assert_eq!(
            fond, attendu.panel,
            "{nom} : rendue sans le thème — fond {fond:?}"
        );
        // DEUX RENDUS, ET LE PREMIER EST JETÉ.
        //
        // L'ATLAS DE GLYPHES SE CONSTRUIT EN PEIGNANT. Une scène qui
        // apporte beaucoup de caractères neufs — la note d'un binaire
        // lié dynamiquement en apporte plus de deux cents — fait
        // grandir la texture des polices à la dernière image, et le
        // premier rendu emploie encore l'ancienne : les glyphes déjà
        // posés y pointent sur les mauvaises cases. Vu le 10 novembre
        // 2026 sur « établi / une note longue » : le code de l'éditeur
        // s'y peignait en charabia, tout le reste de l'image étant
        // juste. Le deuxième rendu, lui, voit l'atlas à jour.
        //
        // C'est le même piège que les six images ci-dessus, d'un cran
        // plus tard : on les fait pour que la MISE EN PAGE se stabilise,
        // et celle-ci pour que la TEXTURE la rattrape. Une image qui
        // bouge seule fait douter de toutes les autres.
        let _ = harness.render();
        harness.step();
        match harness.render() {
            Ok(image) => {
                let fichier = dossier.join(format!("{}.png", nom.replace(['/', ' '], "_")));
                image
                    .save(&fichier)
                    .unwrap_or_else(|e| panic!("{} : {e}", fichier.display()));
                écrites += 1;
            }
            Err(e) => refusées.push(format!("{nom} : {e}")),
        }
    }

    eprintln!("{écrites} image(s) dans {}", dossier.display());
    assert!(
        refusées.is_empty(),
        "{} scène(s) n'ont pas pu être rendues :\n  {}",
        refusées.len(),
        refusées.join("\n  ")
    );
    // Sans filtre, le banc doit rendre tout ce qu'il a : une liste de
    // scènes qui se viderait sans bruit ferait un banc vert qui ne
    // regarde rien. Avec filtre, on a demandé moins — mais au moins
    // une, sinon c'est le nom qu'on a mal écrit.
    if filtre.is_empty() {
        assert!(écrites > 20, "le banc a plus de vingt scènes : {écrites}");
    } else {
        assert!(écrites > 0, "aucune scène ne s'appelle « {filtre} »");
    }
}

/// Les mêmes scènes, au plus petit écran que l'application accepte.
///
/// LE BANC DESSINE SUR 1600x1100, et c'est un écran confortable où
/// presque rien ne manque de place. Les défauts d'ergonomie vivent à
/// l'autre bout : à 800x520, soixante-dix pixels pris par des barres
/// d'icônes ne sont plus un détail, une rangée d'onglets se replie, et
/// un panneau latéral qui garde sa largeur mange la moitié de
/// l'éditeur.
///
/// Les mesures du banc couvrent déjà ce format — elles y ont trouvé
/// des textes superposés et des fenêtres hors de l'écran. Ce qu'elles
/// ne donnent pas, c'est le COUP D'ŒIL : la place qui reste pour
/// travailler, et si l'écran a encore l'air d'un atelier.
#[test]
#[ignore = "écrit des images ; se demande explicitement"]
fn captures_du_petit_ecran() {
    use egui_kittest::Harness;

    let dossier =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/captures-petit");
    std::fs::create_dir_all(&dossier).expect("le dossier des captures se crée");
    let taille = egui::vec2(MIN_SCREEN[0], MIN_SCREEN[1]);

    let mut écrites = 0;
    for (nom, _, prépare) in scènes() {
        let mut app = NamsdecApp::default();
        prépare(&mut app);
        app.values_changed_at = None;
        let mut harness = Harness::builder()
            .with_size(taille)
            .wgpu()
            .build_ui(move |ui| {
                app.apply_theme(ui.ctx());
                app.draw(ui)
            });
        for _ in 0..6 {
            harness.step();
        }
        // DEUX RENDUS, ET LE PREMIER EST JETÉ.
        //
        // L'ATLAS DE GLYPHES SE CONSTRUIT EN PEIGNANT. Une scène qui
        // apporte beaucoup de caractères neufs — la note d'un binaire
        // lié dynamiquement en apporte plus de deux cents — fait
        // grandir la texture des polices à la dernière image, et le
        // premier rendu emploie encore l'ancienne : les glyphes déjà
        // posés y pointent sur les mauvaises cases. Vu le 10 novembre
        // 2026 sur « établi / une note longue » : le code de l'éditeur
        // s'y peignait en charabia, tout le reste de l'image étant
        // juste. Le deuxième rendu, lui, voit l'atlas à jour.
        //
        // C'est le même piège que les six images ci-dessus, d'un cran
        // plus tard : on les fait pour que la MISE EN PAGE se stabilise,
        // et celle-ci pour que la TEXTURE la rattrape. Une image qui
        // bouge seule fait douter de toutes les autres.
        let _ = harness.render();
        harness.step();
        if let Ok(image) = harness.render() {
            let fichier = dossier.join(format!("{}.png", nom.replace(['/', ' '], "_")));
            image.save(&fichier).expect("l'image s'écrit");
            écrites += 1;
        }
    }
    eprintln!("{écrites} image(s) dans {}", dossier.display());
    assert!(écrites > 20, "le banc a plus de vingt scènes : {écrites}");
}

/// Aucun texte peint ne dit « 1 » suivi d'un pluriel.
///
/// Trouvé en REGARDANT les captures : « 1 fonctions retrouvées » dans
/// l'analyse, « 1 passages par ligne » dans la légende de chaleur. Un
/// compte qui ne s'accorde pas est la faute la plus visible d'une
/// interface française, et c'est le cas à UNE qu'on ne voit jamais en
/// développant — on essaie toujours sur un programme qui en a douze.
///
/// Le relevé balaie toutes les scènes du banc et lit ce qui est PEINT.
/// Les mots invariables sont listés : « 1 fois », « 1 pas », « 1 octet
/// de plus » sont corrects.
#[test]
fn aucun_compte_a_un_ne_porte_un_pluriel() {
    // Les mots français qui finissent par s ou x au singulier. La liste
    // est courte, et elle vaut mieux qu'un contrôle qui crie au loup.
    const INVARIABLES: &[&str] = &[
        "fois",
        "pas",
        "mois",
        "temps",
        "corps",
        "cours",
        "sens",
        "processus",
        "relais",
        "jamais",
        "plus",
        "moins",
        "après",
        "très",
        "souris",
        "choix",
        "prix",
        "faux",
        "deux",
        "ceux",
        "os",
        "bis",
        "alias",
        "index",
        "parcours",
        "concours",
        "discours",
        "univers",
        "travers",
        "divers",
        "gras",
        "bas",
        "dessous",
        "dessus",
        "puis",
        "depuis",
        "vers",
        "lors",
        "hors",
        "dans",
        "sous",
        "sans",
    ];

    let mut fautifs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (nom, _, prépare) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        prépare(&mut app);
        // Sans identifiant : on veut TOUT ce qui est peint, fenêtres
        // comprises, et non le contenu d'une seule.
        let (_, textes) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
        for (texte, _) in &textes {
            let mots: Vec<&str> = texte.split_whitespace().collect();
            for paire in mots.windows(2) {
                if paire[0] != "1" {
                    continue;
                }
                let mot = paire[1]
                    .trim_matches(|c: char| !c.is_alphabetic())
                    .to_lowercase();
                if mot.len() < 3 || !(mot.ends_with('s') || mot.ends_with('x')) {
                    continue;
                }
                if INVARIABLES.contains(&mot.as_str()) {
                    continue;
                }
                fautifs.insert(format!("« {nom} » : « {texte} »"));
            }
        }
    }

    assert!(
        fautifs.is_empty(),
        "{} compte(s) à un qui portent un pluriel :\n  {}",
        fautifs.len(),
        fautifs.iter().cloned().collect::<Vec<_>>().join("\n  ")
    );
}

/// Un état vide ne s'étire jamais sur toute la largeur d'un écran.
///
/// Il prenait la place offerte, quelle qu'elle soit. Dans le panneau
/// latéral — trois cents pixels — c'est juste ; dans une vue de la
/// rangée — mille deux cents —, la phrase qui explique ce qu'est un
/// décompilateur s'étirait d'un bord à l'autre sur une seule ligne. Un
/// œil qui revient à la ligne perd son rang au-delà d'une quinzaine de
/// mots : c'est la raison pour laquelle un livre n'est pas composé sur
/// la largeur de sa page.
///
/// Le relevé mesure l'ENCRE des phrases d'état vide, dans la vue la
/// plus large que l'application propose.
#[test]
fn un_etat_vide_tient_dans_une_largeur_de_lecture() {
    // Deux vues larges, et deux états vides différents : celui qui
    // attend un binaire, celui qui attend un décompilateur.
    for (nom, poser) in [
        (
            "décompilation",
            (|app: &mut NamsdecApp| {
                app.show_decompiler_module = true;
                app.montrer_vue(VueCentrale::Decompiler);
            }) as fn(&mut NamsdecApp),
        ),
        ("analyse", |app| {
            app.montrer_vue(VueCentrale::Analysis);
        }),
    ] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        // Panneau et console repliés : la vue prend alors toute la
        // largeur, et c'est le cas qui déborde.
        app.show_sidebar = false;
        app.show_console = false;
        poser(&mut app);

        let (_, lignes) = ce_qui_est_écrit_sur(
            &ctx,
            &mut app,
            None,
            3,
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1900.0, 1000.0),
                )),
                ..Default::default()
            },
        );
        // Une phrase, et non un titre ni une valeur : au moins six mots.
        let longues: Vec<&(String, egui::Rect)> = lignes
            .iter()
            .filter(|(t, _)| t.split_whitespace().count() >= 6)
            .collect();
        assert!(
            !longues.is_empty(),
            "« {nom} » n'écrit aucune phrase — le contrôle ne mesure rien"
        );
        const LARGEUR_DE_LECTURE: f32 = 560.0;
        let débordent: Vec<String> = longues
            .iter()
            .filter(|(_, r)| r.width() > LARGEUR_DE_LECTURE)
            .map(|(t, r)| format!("{:.0} px — « {t} »", r.width()))
            .collect();
        assert!(
            débordent.is_empty(),
            "« {nom} » : {} phrase(s) plus larges que {LARGEUR_DE_LECTURE:.0} px :\n  {}",
            débordent.len(),
            débordent.join("\n  ")
        );
    }
}

/// Une rangée d'onglets marque son onglet actif par un TRAIT SOUS LUI,
/// jamais par un fond.
///
/// Quatre rangées d'onglets dans namsdec : les vues de la zone
/// centrale, les onglets du panneau latéral, les pages des préférences,
/// les onglets de la console. `rangée_d_onglets` prévenait, en
/// commentaire, du jour où « l'application aurait trois idiomes
/// d'onglets au lieu de deux » — elle en avait trois, et son inventaire
/// ne les comptait pas : les préférences et la console passaient par
/// `selectable_label`, qui peint le bleu PLEIN de la sélection.
///
/// Ce que cela donnait à l'écran : dans la fenêtre des préférences,
/// l'onglet « Général » et le bouton « OK » étaient peints de la même
/// façon — un sélecteur et une commande qu'on ne distingue plus. Vu sur
/// capture le 28 octobre 2026.
///
/// LE CONTRÔLE DEMANDE LES DEUX MOITIÉS, et c'est voulu : l'absence du
/// fond, et la PRÉSENCE du trait. « Rien ne recouvre l'onglet actif »
/// serait vrai aussi d'un onglet qui ne se marque pas du tout.
#[test]
fn a_row_of_tabs_underlines_its_active_tab_and_never_fills_it() {
    // Chaque rangée : la scène qui la montre, et l'étiquette de l'onglet
    // qui y est actif. Déclarées ici, parce qu'une rangée nouvelle qui
    // oublierait ce contrôle est précisément ce qu'on vient de vivre.
    /// Une rangée d'onglets du banc : ce qu'on en dit, l'étiquette qui
    /// doit y être active, et la façon de l'amener à l'écran.
    type Rangée = (&'static str, &'static str, Box<dyn Fn(&mut NamsdecApp)>);
    let rangées: Vec<Rangée> = vec![
        (
            "les pages des préférences",
            "Général",
            Box::new(|app: &mut NamsdecApp| {
                app.show_preferences = true;
                app.preferences_tab = PreferencesTab::General;
            }),
        ),
        (
            "les onglets de la console",
            "Console",
            Box::new(|app: &mut NamsdecApp| {
                app.show_console = true;
                app.console_tab = ConsoleTab::Output;
            }),
        ),
        (
            "les onglets du panneau latéral",
            "Processeur",
            Box::new(|app: &mut NamsdecApp| {
                app.show_sidebar = true;
                app.active_tab = SidebarTab::Cpu;
            }),
        ),
        (
            "les familles de la calculatrice",
            calc::FAMILIES[0].1,
            Box::new(|app: &mut NamsdecApp| {
                app.show_calculator = true;
                app.calc_family = calc::FAMILIES[0].0.to_owned();
            }),
        ),
    ];

    for (quoi, actif, prépare) in rangées {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        prépare(&mut app);
        let theme = app.theme;

        // Les formes pleines, relevées sur la même image que les textes :
        // comparer un relevé de formes à un relevé de textes pris à deux
        // images d'écart ferait mesurer deux mises en page.
        // DOUZE IMAGES, et c'est la fenêtre qui les demande : egui fait
        // APPARAÎTRE une `Area` en fondu, et sur les premières images
        // tout ce qu'elle peint est teinté de son opacité. Le trait sous
        // « Général » sortait en #1B4A7F — l'accent à moitié — et le
        // contrôle concluait qu'il n'y en avait pas. Quatre images
        // suffisent à poser une fenêtre, pas à la rendre opaque.
        let entrée = écran_du_banc();
        let mut pleins: Vec<(egui::Rect, egui::Color32)> = Vec::new();
        for _ in 0..12 {
            pleins.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Rect(r) = &clipped.shape
                    && r.fill.a() > 0
                {
                    pleins.push((r.rect, r.fill));
                }
            }
            sortie.textures_delta.clear();
        }
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
        let étiquette = lignes
            .iter()
            .find(|(t, _)| t == actif)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("{quoi} : « {actif} » doit être peint"));

        // PAS DE FOND. Le bleu de la sélection derrière l'étiquette, à
        // deux pixels près — une marge, pas une tolérance : le fond d'un
        // `selectable_label` déborde l'encre de son texte.
        let fonds: Vec<String> = pleins
            .iter()
            .filter(|(rect, fill)| {
                *fill == theme.selection && rect.expand(2.0).contains_rect(étiquette)
            })
            .map(|(rect, _)| format!("{rect:?}"))
            .collect();
        assert!(
            fonds.is_empty(),
            "{quoi} : « {actif} » est peint sur un fond de sélection {fonds:?} — \
             un onglet actif se souligne, il ne se remplit pas (sinon il a \
             l'apparence d'un bouton d'action)"
        );

        // LE TRAIT, SOUS LUI. Un filet de deux pixels, de l'accent du
        // thème, qui commence sous le bas de l'étiquette et couvre son
        // abscisse.
        let traits: Vec<&(egui::Rect, egui::Color32)> = pleins
            .iter()
            .filter(|(rect, fill)| {
                *fill == theme.accent
                    && rect.height() <= 3.0
                    && rect.top() >= étiquette.bottom() - 2.0
                    && rect.top() - étiquette.bottom() < 14.0
                    && rect.left() <= étiquette.left()
                    && rect.right() >= étiquette.right()
            })
            .collect();
        assert!(
            !traits.is_empty(),
            "{quoi} : « {actif} » (encre {étiquette:?}) n'a aucun trait sous lui — \
             c'est à ce trait qu'on voit quel onglet on regarde"
        );
    }
}

/// Un indicateur levé se voit à sa TACHE, et non au seul trait qui
/// l'entoure.
///
/// Les sept témoins de la barre de titre — CF, PF, AF, ZF, SF, DF, OF —
/// étaient des pastilles à CONTOUR, tracé de la couleur de leur propre
/// texte. Sept traits clairs côte à côte, aussi marqués que ce qu'ils
/// encadrent, pour sept valeurs qui sont à zéro la plupart du temps :
/// le cadre criait, le contenu chuchotait. Et un indicateur à un ne se
/// distinguait d'un indicateur à zéro que par la teinte du trait et du
/// chiffre, à la taille d'un caractère.
///
/// Ils sont maintenant creusés dans le fond de l'éditeur, et celui qui
/// est levé teinte ce creux. Le contrôle demande les deux : aucun
/// contour, et un fond qui DIFFÈRE selon l'état — sans quoi « plus de
/// contour » serait vrai aussi de sept pastilles toutes identiques.
#[test]
fn a_raised_flag_shows_as_a_patch_and_not_as_an_outline() {
    /// Le fond et les contours peints sous « ZF {valeur} », à l'endroit
    /// où la barre de titre l'écrit.
    fn relevé(zf: bool) -> (Vec<egui::Color32>, Vec<egui::Color32>) {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        app.source = PROGRAMME_DU_BANC.to_owned();
        app.do_assemble();
        // `cmp rcx, rcx` pose ZF ; quelques pas du programme du banc le
        // laissent retombé. On prend donc l'état par la machine
        // elle-même plutôt que de le supposer.
        for _ in 0..8 {
            app.do_step();
        }
        if let Some(emulator) = app.emulator.as_mut() {
            let mut f = emulator.cpu().flags;
            f.set(namsdec_core::RFlags::ZF, zf);
            emulator.cpu_mut().flags = f;
        }
        let entrée = écran_du_banc();
        let mut fonds = Vec::new();
        let mut contours = Vec::new();
        let mut encre = None;
        for _ in 0..4 {
            fonds.clear();
            contours.clear();
            encre = None;
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            // L'encre de la pastille d'abord : c'est elle qui dit où
            // regarder. LA PLUS HAUTE des « ZF … » : le bandeau du
            // panneau latéral écrit les mêmes sept témoins plus bas, et
            // prendre la dernière venue faisait mesurer celui-là — un
            // texte sans pastille, donc sans fond, donc un contrôle qui
            // échoue en désignant le mauvais endroit.
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape
                    && t.galley.job.text.starts_with("ZF ")
                {
                    let rect = cadre_du_texte(t);
                    if encre.is_none_or(|haut: egui::Rect| rect.top() < haut.top()) {
                        encre = Some(rect);
                    }
                }
            }
            if let Some(encre) = encre {
                for clipped in &sortie.shapes {
                    if let egui::epaint::Shape::Rect(r) = &clipped.shape
                        && r.rect.expand(2.0).contains_rect(encre)
                        && r.rect.width() < 100.0
                    {
                        if r.fill.a() > 0 {
                            fonds.push(r.fill);
                        }
                        if r.stroke.width > 0.0 && r.stroke.color.a() > 0 {
                            contours.push(r.stroke.color);
                        }
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        assert!(encre.is_some(), "« ZF » est peint dans la barre de titre");
        (fonds, contours)
    }

    let (fond_bas, contour_bas) = relevé(false);
    let (fond_haut, contour_haut) = relevé(true);

    assert!(
        contour_bas.is_empty() && contour_haut.is_empty(),
        "la pastille de ZF est encore cerclée d'un trait — retombée {contour_bas:?}, \
         levée {contour_haut:?} ; sept contours clairs dans une barre de titre \
         pèsent plus que les sept valeurs qu'ils encadrent"
    );
    assert!(
        !fond_bas.is_empty() && !fond_haut.is_empty(),
        "la pastille de ZF n'a aucun fond — retombée {fond_bas:?}, levée {fond_haut:?} ; \
         sans creux, les sept témoins se lisent comme une seule phrase"
    );
    assert_ne!(
        fond_bas, fond_haut,
        "ZF levé et ZF retombé se peignent sur le MÊME fond : l'état d'un \
         indicateur doit se voir à la tache, et non au seul chiffre"
    );
}

/// Le libellé d'un bouton encadré commence par une capitale.
///
/// Une règle de CASSE, et c'est une règle de forme comme les autres :
/// sur une même fenêtre, « Copier » voisinait avec « sommaire » — le
/// seul bouton de l'application dont le libellé s'ouvrait en
/// minuscule. Ce n'est pas une faute de langue, c'est une rupture de
/// rythme : l'œil parcourt une rangée de commandes en s'appuyant sur
/// leurs capitales, et celle qui manque se lit comme du texte courant.
///
/// Mesuré sur le RENDU et non sur `i18n` : un cadre de la couleur
/// estompée du thème EST un bouton encadré (c'est ce que dit
/// `a_framed_button_draws_the_outline_a_flat_one_does_not`), et l'on
/// relève ce qui est écrit dedans. Un libellé nouveau est donc jugé
/// sans qu'on ait à l'inscrire nulle part.
#[test]
fn a_framed_button_opens_its_label_with_a_capital() {
    let entrée = écran_du_banc();
    let mut fautes: Vec<String> = Vec::new();
    for lang in [Lang::Fr, Lang::En] {
        for (nom, _, ouvre) in scènes() {
            let ctx = egui::Context::default();
            let mut app = NamsdecApp {
                lang,
                ..Default::default()
            };
            app.apply_theme(&ctx);
            ouvre(&mut app);
            let theme = app.theme;
            // DOUZE IMAGES : egui fait APPARAÎTRE une fenêtre en fondu,
            // et tant qu'il dure, tout ce qu'elle peint est teinté de
            // son opacité. Le cadre d'un bouton n'est alors plus
            // `theme.dim` mais `theme.dim` à moitié, et le relevé ne
            // reconnaît aucun bouton dans aucune fenêtre — un contrôle
            // au vert qui ne regarde rien.
            for image in 0..12 {
                let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
                if image < 11 {
                    sortie.textures_delta.clear();
                    continue;
                }
                let cadres: Vec<egui::Rect> = sortie
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Rect(r)
                            if r.stroke.width >= 1.0 && r.stroke.color == theme.dim =>
                        {
                            Some(r.rect)
                        }
                        _ => None,
                    })
                    .collect();
                for clipped in &sortie.shapes {
                    let egui::epaint::Shape::Text(t) = &clipped.shape else {
                        continue;
                    };
                    let encre = cadre_du_texte(t);
                    if !cadres.iter().any(|c| c.expand(1.0).contains_rect(encre)) {
                        continue;
                    }
                    // LE GRAPHE N'EST PAS UNE RANGÉE DE BOUTONS. Ses
                    // blocs portent le même cadre estompé, et le relevé
                    // y voyait vingt « boutons » nommés `mov eax,1`. Un
                    // bouton écrit en PROPORTIONNELLE : le code d'un
                    // bloc est en chasse fixe, et c'est ce qui les
                    // sépare sans rien connaître du graphe.
                    if t.galley.job.sections.iter().any(|section| {
                        section.format.font_id.family != egui::FontFamily::Proportional
                    }) {
                        continue;
                    }
                    // Et un bouton ne porte qu'UN texte : un bloc du
                    // graphe en empile plusieurs, dont sa phrase
                    // d'explication, qui est bien en proportionnelle.
                    let dans_le_cadre = cadres
                        .iter()
                        .find(|c| c.expand(1.0).contains_rect(encre))
                        .copied();
                    if let Some(cadre) = dans_le_cadre {
                        let combien = sortie
                            .shapes
                            .iter()
                            .filter(|c| match &c.shape {
                                egui::epaint::Shape::Text(autre) => {
                                    cadre.expand(1.0).contains_rect(egui::Rect::from_min_size(
                                        autre.pos,
                                        autre.galley.size(),
                                    ))
                                }
                                _ => false,
                            })
                            .count();
                        if combien > 1 {
                            continue;
                        }
                    }
                    let texte = t.galley.text();
                    let Some(première) = texte.chars().next() else {
                        continue;
                    };
                    // Seules les lettres sont jugées : une commande peut
                    // s'ouvrir sur un chiffre, un signe ou une icône.
                    if !première.is_alphabetic() || première.is_uppercase() {
                        continue;
                    }
                    let faute = format!("« {texte} » — {nom} ({lang:?})");
                    if !fautes.contains(&faute) {
                        fautes.push(faute);
                    }
                }
                sortie.textures_delta.clear();
            }
        }
    }
    fautes.sort();
    assert!(
        fautes.is_empty(),
        "{} libellé(s) de bouton encadré s'ouvrent en minuscule :\n  {}",
        fautes.len(),
        fautes.join("\n  ")
    );
}

/// Les cartes d'une liste partagent une largeur.
///
/// La page Modules des préférences empile dix `module_card`. Chacune
/// épousait son texte : « nasm détecté — NASM version 3.02 compiled on
/// Jul 14 2026 » en faisait une de 355 px à côté d'une de 175, et le
/// bord droit de la page descendait en dents de scie. Ce ne sont pas
/// dix objets de tailles différentes, c'est une liste — et une liste
/// s'aligne. Les vignettes de thèmes de la page voisine partagent déjà
/// une largeur calculée une fois.
///
/// Mesuré sur les cadres peints. Le trait d'un `Frame::group` n'est PAS
/// une couleur du thème — c'est `widgets.noninteractive.bg_stroke`,
/// qu'egui pose lui-même ; le relevé la demande donc au style plutôt
/// que de la nommer, sans quoi il ne trouve aucune carte. Et la hauteur
/// les sépare des boutons encadrés, qui portent le même trait sur une
/// seule ligne.
#[test]
fn the_cards_of_a_list_share_one_width() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.show_preferences = true;
    app.preferences_tab = PreferencesTab::Modules;
    let trait_du_cadre = ctx
        .style_of(egui::Theme::Dark)
        .visuals
        .widgets
        .noninteractive
        .bg_stroke
        .color;
    let entrée = écran_du_banc();
    let mut largeurs: Vec<f32> = Vec::new();
    // Douze images : la fenêtre des préférences apparaît en fondu, et
    // tant qu'il dure, le trait d'un cadre n'est plus `theme.dim` mais
    // `theme.dim` à moitié — le relevé ne verrait aucune carte.
    for _ in 0..12 {
        largeurs.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Rect(r) = &clipped.shape
                && r.stroke.width >= 1.0
                && r.stroke.color == trait_du_cadre
                && r.rect.height() > 40.0
            {
                largeurs.push(r.rect.width());
            }
        }
        sortie.textures_delta.clear();
    }
    assert!(
        largeurs.len() >= 8,
        "la page Modules empile au moins huit cartes, relevé {largeurs:?}"
    );
    let mut distinctes = largeurs.clone();
    distinctes.sort_by(f32::total_cmp);
    distinctes.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    assert_eq!(
        distinctes.len(),
        1,
        "les {} cartes de la page Modules ont {} largeurs différentes : {distinctes:?} — \
         une liste s'aligne sur un bord droit, elle n'épouse pas chaque phrase",
        largeurs.len(),
        distinctes.len()
    );
}

/// Le vidage de l'éditeur binaire emploie la place qu'on lui donne.
///
/// IL NE LE FAISAIT PAS, et de deux façons qui s'additionnaient. Le
/// nombre de lignes était écrit d'avance — vingt-deux —, donc le même
/// sur un écran de mille pixels et sur un de deux mille quatre cents :
/// la moitié du panneau restait blanche en grand, et le vidage
/// débordait en petit, avec un second ascenseur à côté du sien.
///
/// Et le nombre annoncé n'était même pas celui qu'on voyait :
/// `show_rows` compte ses lignes à `hauteur + item_spacing.y`, l'écart
/// n'était remis à zéro qu'à l'intérieur de la closure — trop tard —,
/// si bien que la zone réservait vingt lignes de place pour en peindre
/// quinze. « Vingt-deux » en montrait seize, et les six manquantes
/// restaient en blanc au bas de la zone.
///
/// CE QUE CE CONTRÔLE MESURE est donc le seul fait qui les attrape
/// toutes les deux : le nombre de lignes PEINTES, qui doit croître
/// avec la hauteur de l'écran, et remplir ce qu'on lui a donné.
#[test]
fn le_vidage_binaire_emploie_la_place_qu_on_lui_donne() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };

    /// Les adresses de la première colonne : six chiffres hexadécimaux
    /// et rien d'autre. Les compter, c'est compter les lignes peintes.
    fn lignes_peintes(formes: &[egui::epaint::ClippedShape]) -> (usize, f32) {
        let mut vues = 0;
        let mut bas_des_lignes = f32::MIN;
        let mut bas_de_la_zone = f32::MIN;
        for clipped in formes {
            if let egui::Shape::Text(t) = &clipped.shape {
                let texte = t.galley.text();
                if texte.len() == 6 && texte.chars().all(|c| c.is_ascii_hexdigit()) {
                    // Ce qui sort de la découpe n'est pas à l'écran.
                    let encre = cadre_du_texte(t);
                    if clipped.clip_rect.intersects(encre) {
                        vues += 1;
                        bas_des_lignes = bas_des_lignes.max(encre.bottom());
                        // La découpe d'une adresse EST celle de la zone
                        // du vidage : c'est elle qui dit jusqu'où la
                        // zone descend.
                        bas_de_la_zone = bas_de_la_zone.max(clipped.clip_rect.bottom());
                    }
                }
            }
        }
        (vues, (bas_de_la_zone - bas_des_lignes).max(0.0))
    }

    let compte = |hauteur: f32| -> (usize, f32) {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        app.load_elf(&octets, NOM_FABRIQUÉ);
        app.show_analysis = true;
        app.analysis_module = "hex_editor".to_owned();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, hauteur),
            )),
            predicted_dt: 0.5,
            ..Default::default()
        };
        let mut formes = Vec::new();
        for _ in 0..6 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            formes = std::mem::take(&mut sortie.shapes);
            sortie.textures_delta.clear();
        }
        lignes_peintes(&formes)
    };

    // TROIS HAUTEURS, et non deux : à deux, ce contrôle passait encore
    // avec le nombre de lignes refigé. Un vidage figé à vingt-deux en
    // montre moins quand l'écran est petit — non parce qu'il se serre,
    // mais parce qu'il est COUPÉ —, et la différence suffisait à le
    // faire paraître adaptatif. Entre deux écrans où ses vingt-deux
    // lignes tiennent, en revanche, il plafonne ; c'est ce plafond que
    // la troisième mesure attrape.
    let mesures: Vec<(usize, f32)> = [700.0, 1100.0, 1500.0].into_iter().map(compte).collect();
    let compté: Vec<usize> = mesures.iter().map(|(n, _)| *n).collect();
    assert!(
        compté[0] > 0,
        "le vidage doit peindre des lignes : {compté:?}"
    );
    assert!(
        compté[2] > compté[1] && compté[1] > compté[0],
        "le vidage doit montrer plus d'octets quand on lui donne plus de hauteur, \
         et il plafonne : {compté:?} lignes à 700, 1100 et 1500 px"
    );
    // ET PAS DE BLANC AU BAS DE LA ZONE. C'est l'autre moitié du
    // défaut, et elle ne se voit pas dans un compte : `show_rows`
    // réservait vingt pixels par ligne pour en peindre quinze, si bien
    // que le quart du bas de la zone restait vide alors qu'il restait
    // des octets à montrer. Deux lignes de tolérance : la dernière peut
    // ne pas tomber juste.
    for (hauteur, (lignes, blanc)) in [700.0, 1100.0, 1500.0].iter().zip(&mesures) {
        assert!(
            *blanc < 40.0,
            "à {hauteur} px de haut, {blanc:.0} pixels de blanc au bas de la zone \
             du vidage, sous {lignes} lignes — il reste des octets à y montrer"
        );
    }
}

/// Quand la place y est, choisir un octet ne rétrécit pas le vidage.
///
/// L'inspecteur — position, lectures, boutons — se posait SOUS le
/// vidage et lui prenait quatre cents pixels, dans un panneau dont le
/// quart droit restait vide. Cliquer un octet coûtait donc la moitié de
/// ce qu'on regardait, au moment précis où l'on en avait besoin.
///
/// Il va maintenant dans une colonne à droite dès qu'il y a la largeur
/// — ce qui suppose deux choses : que le panneau ne soit plus borné à
/// une largeur de LECTURE, qui est faite pour de la prose et non pour
/// une grille, et que le vidage garde sa hauteur. C'est cette seconde
/// moitié qu'on mesure ici ; la première la rend possible.
#[test]
fn choisir_un_octet_ne_retrecit_pas_le_vidage_quand_la_place_y_est() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };

    /// Les lignes peintes, et où se trouve le premier octet cliquable.
    fn relève(formes: &[egui::epaint::ClippedShape]) -> (usize, Option<egui::Pos2>) {
        let mut lignes = 0;
        let mut premier = None;
        for clipped in formes {
            if let egui::Shape::Text(t) = &clipped.shape {
                let texte = t.galley.text();
                let encre = cadre_du_texte(t);
                if !clipped.clip_rect.intersects(encre) {
                    continue;
                }
                if texte.len() == 6 && texte.chars().all(|c| c.is_ascii_hexdigit()) {
                    lignes += 1;
                    // Un octet de la grille se trouve à droite des deux
                    // colonnes d'adresses, sur la même ligne.
                    if premier.is_none() && texte == "000000" {
                        premier = Some(egui::pos2(encre.right() + 120.0, encre.center().y));
                    }
                }
            }
        }
        (lignes, premier)
    }

    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.load_elf(&octets, NOM_FABRIQUÉ);
    app.show_analysis = true;
    app.analysis_module = "hex_editor".to_owned();
    // Agrandie : c'est là que la largeur existe, et c'est le cas qu'on
    // veut tenir. Serrée, l'inspecteur se remet dessous, et c'est bien.
    app.toggle_maximized(NamsdecApp::ANALYSIS_WINDOW);

    let écran = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1900.0, 1200.0),
        )),
        predicted_dt: 0.5,
        ..Default::default()
    };
    let mut formes = Vec::new();
    for _ in 0..6 {
        let mut sortie = ctx.run_ui(écran.clone(), |ui| app.draw(ui));
        formes = std::mem::take(&mut sortie.shapes);
        sortie.textures_delta.clear();
    }
    let (avant, cible) = relève(&formes);
    let cible = cible.expect("le vidage peint ses adresses");
    assert!(avant > 20, "le vidage doit être grand ici : {avant}");

    // Le clic sur un octet, puis quelques images pour que le panneau
    // s'installe.
    for pressed in [true, false] {
        let clic = egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: cible,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            }],
            ..écran.clone()
        };
        let mut sortie = ctx.run_ui(clic, |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    for _ in 0..4 {
        let mut sortie = ctx.run_ui(écran.clone(), |ui| app.draw(ui));
        formes = std::mem::take(&mut sortie.shapes);
        sortie.textures_delta.clear();
    }
    let (après, _) = relève(&formes);
    assert!(
        après + 2 >= avant,
        "choisir un octet a coûté {} lignes de vidage ({avant} puis {après}) : \
         l'inspecteur est repassé sous la grille au lieu de se poser à côté",
        avant.saturating_sub(après)
    );
}

/// LES CAPTURES DU SITE, rendues depuis le dépôt.
///
/// `cargo test -p namsdec-gui -- --ignored releve_du_site`
///
/// Le site montre l'application : il lui faut donc des images, et des
/// images qui ne vieillissent pas en silence. Celles-ci se refont d'une
/// commande, sur les scènes que le banc déclare déjà — ce qui est
/// capturé est donc ce qui est mesuré ailleurs, et non une pose
/// choisie un bon jour.
///
/// Rendues **sans écran** : `egui_kittest` passe par lavapipe, donc ni
/// serveur d'affichage, ni carte graphique, ni droits. Deux rendus par
/// scène, et le premier est jeté — l'atlas de glyphes se construit en
/// peignant, et la première image emploie encore l'ancien.
#[test]
#[ignore]
fn releve_du_site() {
    use egui_kittest::Harness;
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/images");
    std::fs::create_dir_all(&dossier).expect("dossier");
    // Ce que le site montre, et le nom du fichier qu'il attend.
    let voulues = [
        ("établi", "etabli"),
        ("établi (en marche)", "etabli-en-marche"),
        ("application / accueil, plus rien d'ouvert", "accueil"),
        ("application / cpu (en marche)", "processeur"),
        ("vue / points d'arrêt posés", "points-d-arret"),
        ("analyse / hex_editor (en marche)", "editeur-binaire"),
        ("analyse / binary_info (en marche)", "protections"),
        ("analyse / call_graph (en marche)", "graphe-d-appels"),
        // L'ENTROPIE : elle se lit sur n'importe quel fichier, même
        // court, parce qu'elle mesure les octets et non le code.
        ("analyse / entropy (en marche)", "entropie"),
    ];
    let mut écrites = 0;
    for (nom, _, ouvre) in scènes() {
        let Some((_, fichier)) = voulues.iter().find(|(v, _)| *v == nom) else {
            continue;
        };
        let mut app = NamsdecApp::default();
        ouvre(&mut app);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1600.0, 1100.0))
            .wgpu()
            .build_ui(move |ui| {
                app.apply_theme(ui.ctx());
                app.draw(ui)
            });
        for _ in 0..8 {
            harness.step();
        }
        let _ = harness.render();
        harness.step();
        harness
            .render()
            .expect("rendu")
            .save(dossier.join(format!("{fichier}.png")))
            .expect("écriture");
        println!("écrit {fichier}.png");
        écrites += 1;
    }
    assert!(écrites > 0, "aucune scène rendue : les noms ont changé");

    // LES PANNEAUX AVANCÉS, SUR UN BINAIRE QUI A DE QUOI LES REMPLIR.
    //
    // Les scènes du banc tournent sur le programme du banc : vingt
    // lignes, un saut, une chaîne. Les fragments ROP y répondent « le
    // programme est trop court », les références croisées « une
    // boucle » — on montrerait des panneaux vides, ce qui dessert
    // mieux qu'on ne montre. Le crackme fabriqué, lui, a des appels,
    // des chaînes et une comparaison de mot de passe : c'est le plus
    // petit sujet qui ait quelque chose à dire.
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    for (module, fichier) in [
        ("xrefs", "references-croisees"),
        ("functions", "fonctions"),
        ("slack", "place-libre"),
    ] {
        let mut app = NamsdecApp::default();
        if !app.load_elf(&octets, NOM_FABRIQUÉ) {
            continue;
        }
        app.show_analysis = true;
        app.analysis_module = module.to_owned();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1600.0, 1100.0))
            .wgpu()
            .build_ui(move |ui| {
                app.apply_theme(ui.ctx());
                app.draw(ui)
            });
        for _ in 0..8 {
            harness.step();
        }
        let _ = harness.render();
        harness.step();
        harness
            .render()
            .expect("rendu")
            .save(dossier.join(format!("{fichier}.png")))
            .expect("écriture");
        println!("écrit {fichier}.png");
    }
}

/// Les captures de l'éditeur binaire, à quatre tailles.
///
/// À part des scènes du banc, et pour une raison écrite dans `scènes()` :
/// la plus intéressante — la fenêtre agrandie, seule disposition où
/// l'inspecteur se pose à côté du vidage — ne peut pas y entrer tant
/// que le tri des formes d'une fenêtre se fait sur la largeur de sa
/// découpe.
#[test]
#[ignore]
fn releve_hex() {
    use egui_kittest::Harness;
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/captures");
    let Some(source) = binaire_fabriqué() else {
        return;
    };
    for (étiquette, taille, clic, agrandie) in [
        ("petit", egui::vec2(1000.0, 700.0), false, false),
        ("grand", egui::vec2(2400.0, 1500.0), false, false),
        ("choisi", egui::vec2(1600.0, 1100.0), true, false),
        ("colonne", egui::vec2(1900.0, 1200.0), true, true),
    ] {
        let octets = source.clone();
        let mut app = NamsdecApp::default();
        app.load_elf(&octets, NOM_FABRIQUÉ);
        app.show_analysis = true;
        app.analysis_module = "hex_editor".to_owned();
        if agrandie {
            app.toggle_maximized(NamsdecApp::ANALYSIS_WINDOW);
        }
        let mut harness = Harness::builder()
            .with_size(taille)
            .wgpu()
            .build_ui(move |ui| {
                app.apply_theme(ui.ctx());
                app.draw(ui)
            });
        for _ in 0..6 {
            harness.step();
        }
        if clic {
            for pressed in [true, false] {
                harness.input_mut().events.push(egui::Event::PointerButton {
                    pos: egui::pos2(537.0, 452.0),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                });
            }
            for _ in 0..4 {
                harness.step();
            }
        }
        // DEUX RENDUS, ET LE PREMIER EST JETÉ.
        //
        // L'ATLAS DE GLYPHES SE CONSTRUIT EN PEIGNANT. Une scène qui
        // apporte beaucoup de caractères neufs — la note d'un binaire
        // lié dynamiquement en apporte plus de deux cents — fait
        // grandir la texture des polices à la dernière image, et le
        // premier rendu emploie encore l'ancienne : les glyphes déjà
        // posés y pointent sur les mauvaises cases. Vu le 10 novembre
        // 2026 sur « établi / une note longue » : le code de l'éditeur
        // s'y peignait en charabia, tout le reste de l'image étant
        // juste. Le deuxième rendu, lui, voit l'atlas à jour.
        //
        // C'est le même piège que les six images ci-dessus, d'un cran
        // plus tard : on les fait pour que la MISE EN PAGE se stabilise,
        // et celle-ci pour que la TEXTURE la rattrape. Une image qui
        // bouge seule fait douter de toutes les autres.
        let _ = harness.render();
        harness.step();
        let image = harness.render().expect("rendu");
        image
            .save(dossier.join(format!("hex-{étiquette}.png")))
            .expect("écriture");
        println!("écrit hex-{étiquette}");
    }
}

/// L'éditeur binaire met plus d'octets par ligne quand on lui donne
/// plus de largeur.
///
/// Le nombre était un RÉGLAGE — seize par défaut —, et la raison écrite
/// était la même crainte que pour la hauteur : lire la largeur offerte
/// serait lire une dimension que ce panneau fixe lui-même. Mesuré dans
/// l'onglet d'analyse : mille trois cent quarante-six pixels offerts,
/// six cent six pris — SEPT CENT QUARANTE de vide, plus que la grille
/// entière —, et la largeur relevée ne bouge pas d'une image à l'autre.
///
/// Ce qu'on mesure ici est la règle des colonnes : elle porte « 1f »
/// quand la ligne fait trente-deux octets, et s'arrête à « 0f » quand
/// elle en fait seize. C'est ce que l'œil voit, et non un réglage
/// interne.
#[test]
fn l_editeur_binaire_remplit_la_largeur_qu_on_lui_donne() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };

    let colonnes = |largeur: f32| -> bool {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        app.load_elf(&octets, NOM_FABRIQUÉ);
        app.vue_centrale = VueCentrale::Analysis;
        app.analysis_module = "hex_editor".to_owned();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, 1200.0),
            )),
            predicted_dt: 0.5,
            ..Default::default()
        };
        let mut formes = Vec::new();
        for _ in 0..6 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            formes = std::mem::take(&mut sortie.shapes);
            sortie.textures_delta.clear();
        }
        // « 1f » n'est peint que par la règle d'une ligne de
        // trente-deux octets : c'est le numéro de sa dernière colonne.
        formes.iter().any(|clipped| {
            if let egui::Shape::Text(t) = &clipped.shape {
                let encre = cadre_du_texte(t);
                t.galley.text() == "1f" && clipped.clip_rect.intersects(encre)
            } else {
                false
            }
        })
    };

    assert!(
        !colonnes(1100.0),
        "à 1100 px, trente-deux octets par ligne ne tiennent pas : la règle ne doit pas \
         aller jusqu'à « 1f »"
    );
    assert!(
        colonnes(2000.0),
        "à 2000 px, la place est là et la grille doit la prendre : la règle doit aller \
         jusqu'à « 1f »"
    );
}

/// AUCUNE PHRASE NE PERD DE HAUTEUR AU DÉCOUPAGE.
///
/// Le banc vérifiait déjà qu'une phrase n'est pas coupée en LARGEUR —
/// et seulement dans l'onglet Mémoire. La hauteur n'était regardée
/// nulle part, et c'est elle qui tombe quand une zone défilante reçoit
/// moins de place qu'elle n'en demande : la dernière ligne garde ses
/// jambages et perd ses hampes, ce qui donne un texte qu'on lit à moitié
/// sans savoir pourquoi.
///
/// **Trouvé en REGARDANT**, pas en mesurant : la capture du premier
/// lancement montrait la phrase d'accueil de la console tranchée par le
/// milieu, au-dessus de la barre d'état. Les mesures ne répondent qu'aux
/// questions qu'on leur pose, et personne n'avait posé celle-là.
#[test]
fn aucune_phrase_ne_perd_de_hauteur_au_decoupage() {
    let mut fautifs = Vec::new();
    for (nom, _, prépare) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        prépare(&mut app);
        app.values_changed_at = None;
        // UNE HAUTEUR FIGÉE À LA POIGNÉE EST UN CHOIX, PAS UN DÉFAUT.
        //
        // La console sait se replier jusqu'à ne plus montrer qu'une
        // demi-ligne, et c'est écrit noir sur blanc dans `console.rs` :
        // « une zone qui défile a le droit de couper ; ce qu'on
        // protège, c'est la rangée où l'on tape ». Imposer des lignes
        // entières a été essayé puis DÉFAIT sur mesure — la console
        // débordait sur ses voisins, et quatre invariants l'ont dit.
        // Ce contrôle-ci ne juge donc que les hauteurs que le programme
        // choisit lui-même.
        if app.console_height.is_some() {
            continue;
        }
        app.apply_theme(&ctx);
        let input = écran_du_banc();
        // LA DERNIÈRE IMAGE, ET ELLE SEULE.
        //
        // Une surface ancrée ne se place qu'après s'être mesurée, et
        // une fenêtre est AMENÉE à sa taille : pendant deux images, les
        // barres du bas n'ont pas encore la hauteur qu'elles auront, et
        // ce qu'elles portent passe sous le bord. Juger ces images-là
        // ferait rougir sur une mise en place qui converge — « Lig 1,
        // Col 1 » coupée à la deuxième image et entière à la
        // troisième. Ce qu'on refuse, c'est ce qui reste coupé quand
        // tout s'est posé.
        for image in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            if image < 2 {
                output.textures_delta.clear();
                continue;
            }
            // CE QU'UNE ZONE DIT QUAND ELLE N'A QUE ÇA À DIRE.
            //
            // C'est la règle que ce contrôle retient, et elle est plus
            // étroite que « rien n'est jamais coupé » — pour une
            // raison mesurée, après deux formulations trop larges.
            //
            // Une liste plus longue que sa fenêtre coupe forcément une
            // ligne à son bord : c'est ainsi qu'on SAIT qu'il y a une
            // suite, et vingt listes du programme le font à bon droit.
            // J'ai d'abord cru pouvoir les reconnaître à une autre
            // ligne peinte entièrement hors du cadre — la preuve du
            // débordement. Faux : une zone qui dépasse de MOINS d'une
            // ligne n'en a aucune, et deux listes parfaitement
            // normales se sont retrouvées accusées.
            //
            // Ce qui ne s'excuse jamais, en revanche, c'est une zone
            // dont l'unique ligne est coupée : elle montrait tout ce
            // qu'elle avait, et elle l'a montré à moitié. C'est le
            // défaut qu'on a vu sur la capture du premier lancement —
            // la console disant ce qu'elle montrera, tranchée par le
            // milieu.
            let mut lignes_par_cadre: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for clipped in &output.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                let cadre = format!("{:?}", clipped.clip_rect);
                for ligne in t.galley.rows.iter() {
                    if ligne
                        .rect_without_leading_space()
                        .translate(t.pos.to_vec2())
                        .is_positive()
                    {
                        *lignes_par_cadre.entry(cadre.clone()).or_insert(0) += 1;
                    }
                }
            }
            for clipped in &output.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                for ligne in t.galley.rows.iter() {
                    let texte = ligne.text();
                    // Une PHRASE : plusieurs mots. Un libellé court qui
                    // dépasse d'un pixel est une autre affaire, et la
                    // troncature horizontale a déjà son contrôle.
                    if texte.split_whitespace().count() < 4 {
                        continue;
                    }
                    let encre = ligne
                        .rect_without_leading_space()
                        .translate(t.pos.to_vec2());
                    if !encre.is_positive() {
                        continue;
                    }
                    let vu = encre.intersect(clipped.clip_rect);
                    // CE QUI EST ENTIÈREMENT HORS DU CADRE N'EST PAS
                    // COUPÉ : c'est une ligne qu'une zone défilante a
                    // fait sortir par le haut ou par le bas, et egui la
                    // pose quand même dans la liste des formes. Elle ne
                    // se voit pas, personne ne la lit à moitié, et
                    // l'accuser ferait crier ce contrôle sur toutes les
                    // listes du programme. Le défaut qu'on cherche est
                    // la ligne qu'on voit À MOITIÉ.
                    if vu.height() < 0.5 {
                        continue;
                    }
                    // La zone a plus d'une ligne : la coupure est le
                    // bord d'un défilement, pas un défaut de mise en
                    // page.
                    if lignes_par_cadre
                        .get(&format!("{:?}", clipped.clip_rect))
                        .is_some_and(|combien| *combien > 1)
                    {
                        continue;
                    }
                    // UN PIXEL DE TOLÉRANCE : une zone défilante pose
                    // son découpage au pixel près, et l'arrondi d'une
                    // hauteur de ligne ne fait pas un texte coupé.
                    if vu.height() + 1.0 < encre.height() {
                        fautifs.push(format!(
                            "{nom} : « {} » — {:.0} px peints sur {:.0}",
                            texte.chars().take(45).collect::<String>(),
                            vu.height().max(0.0),
                            encre.height()
                        ));
                    }
                }
            }
            output.textures_delta.clear();
        }
    }
    fautifs.sort();
    fautifs.dedup();
    assert!(
        fautifs.is_empty(),
        "des phrases sont coupées en hauteur :\n  {}",
        fautifs.join("\n  ")
    );
}

/// **UNE FENÊTRE AGRANDIE PUIS RESTAURÉE RETROUVE SA TAILLE.**
///
/// Signalé à l'écran le 4 novembre 2026, capture à l'appui : on agrandit
/// la fenêtre des exercices, on clique « Restaurer », et il ne se passe
/// rien — la fenêtre reste pleine page. « C'est identique sur tous les
/// écrans. »
///
/// La cause tient à egui : `fixed_pos` et `fixed_size` ÉCRIVENT dans la
/// mémoire de la fenêtre. Au retour, celle-ci est construite avec ses
/// `default_*`, qui ne valent que la PREMIÈRE fois — la position et la
/// taille retenues sont donc restées celles de la pleine page. Le
/// bouton fonctionnait, l'état basculait bien ; c'est la géométrie qui
/// ne revenait pas.
#[test]
fn une_fenetre_restauree_retrouve_sa_taille() {
    // LES SEPT FENÊTRES QUI S'AGRANDISSENT, et elles seules : les
    // autres n'ont pas de bouton. Deux identifiants par fenêtre, et
    // c'est une chausse-trappe de ce code — la clé d'agrandissement
    // (`EXERCISES_WINDOW` = « exercises ») n'est pas l'identifiant egui
    // de la fenêtre (« exercises_window_v2 »). Les confondre fait
    // basculer un drapeau que personne ne lit.
    let agrandissables: &[(&str, &'static str, &str)] = &[
        ("calculatrice", "calculator", "calculator_window"),
        ("exercices", "exercises", "exercises_window_v2"),
        ("désassemblage", "disassembly", "disassembly_window"),
        ("chronologie", "timeline", "timeline_window"),
        ("analyse", "analysis", "analysis_window"),
        ("déroulement", "flow", "flow_window"),
        ("décompilateur", "decompiler", "decompiler_window"),
    ];
    let mut fautifs = Vec::new();
    for (nom, clé, id) in agrandissables.iter().copied() {
        let Some((_, _, ouvre)) = fenêtres_ouvrables().into_iter().find(|(_, i, _)| *i == id)
        else {
            fautifs.push(format!("{nom} : aucune scène n'ouvre « {id} »"));
            continue;
        };
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        ouvre(&mut app);
        app.apply_theme(&ctx);
        let input = écran_du_banc();

        // Ce qu'elle mesure normalement, une fois posée.
        let mesure = |ctx: &egui::Context, app: &mut NamsdecApp| -> Option<egui::Rect> {
            for _ in 0..6 {
                let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                output.textures_delta.clear();
            }
            egui::AreaState::load(ctx, egui::Id::unique(id)).map(|a| a.rect())
        };
        let Some(avant) = mesure(&ctx, &mut app) else {
            continue;
        };

        // Agrandie : elle doit grandir, sans quoi le reste ne prouve
        // rien.
        app.toggle_maximized(clé);
        let Some(agrandie) = mesure(&ctx, &mut app) else {
            continue;
        };
        if agrandie.area() <= avant.area() {
            fautifs.push(format!("{nom} : agrandir ne l'agrandit pas"));
            continue;
        }

        // Restaurée : elle doit retrouver ce qu'elle avait.
        app.toggle_maximized(clé);
        let Some(après) = mesure(&ctx, &mut app) else {
            continue;
        };
        let écart = (après.size() - avant.size()).abs();
        if écart.x > 2.0 || écart.y > 2.0 {
            fautifs.push(format!(
                "{nom} : restaurée à {:.0}×{:.0}, elle faisait {:.0}×{:.0} \
                 (agrandie : {:.0}×{:.0})",
                après.width(),
                après.height(),
                avant.width(),
                avant.height(),
                agrandie.width(),
                agrandie.height()
            ));
        }
        // ET LÀ OÙ ELLE ÉTAIT, car agrandir la pose au coin haut
        // gauche : une fenêtre qui revient à sa taille mais reste
        // collée au coin n'est pas revenue.
        let bougé = (après.min - avant.min).abs();
        if bougé.x > 2.0 || bougé.y > 2.0 {
            fautifs.push(format!(
                "{nom} : restaurée en ({:.0}, {:.0}), elle était en ({:.0}, {:.0})",
                après.min.x, après.min.y, avant.min.x, avant.min.y
            ));
        }
    }
    assert!(
        fautifs.is_empty(),
        "des fenêtres ne reviennent pas de leur agrandissement :\n  {}",
        fautifs.join("\n  ")
    );
}

/// Une rangée d'onglets tient sur UNE ligne, quelle que soit la place.
///
/// **Trouvé en regardant l'écran le plus petit que l'application
/// accepte** (800x520, 5 novembre 2026) : les dix vues prenaient TROIS
/// lignes — quatre-vingt-seize pixels, de quoi peindre six lignes de
/// code — pour montrer dix onglets dont on en regarde un. Il restait
/// quatre lignes de programme à l'écran.
///
/// Le repli n'était pas un défaut d'implémentation : il était voulu, et
/// un contrôle veillait même à ce que les onglets repliés ne
/// recouvrent rien. Ce que personne n'avait mesuré, c'est CE QU'IL
/// COÛTE — et il coûte l'écran.
///
/// Trois choses à tenir, et la deuxième est celle qu'on oublie en
/// rangeant des onglets derrière un bouton.
#[test]
fn une_rangee_d_onglets_tient_sur_une_ligne() {
    let s = strings(Lang::Fr);
    let peindre = |largeur: f32, hauteur: f32, vue: VueCentrale| -> Vec<(String, f32)> {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            source: PROGRAMME_DU_BANC.to_owned(),
            vue_centrale: vue,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, hauteur),
            )),
            ..Default::default()
        };
        let mut vus = Vec::new();
        for _ in 0..3 {
            vus.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    vus.push((texte.galley.text().to_owned(), texte.pos.y));
                }
            }
            sortie.textures_delta.clear();
        }
        vus
    };

    // 1. UNE SEULE LIGNE. Les étiquettes de vues qui sont peintes le
    //    sont toutes à la même hauteur.
    let noms: Vec<&str> = VueCentrale::ALL.iter().map(|v| v.label(s)).collect();
    for (largeur, hauteur) in [(MIN_SCREEN[0], MIN_SCREEN[1]), (1600.0, 1100.0)] {
        let vus = peindre(largeur, hauteur, VueCentrale::Source);
        let mut hauteurs: Vec<f32> = vus
            .iter()
            .filter(|(texte, _)| noms.contains(&texte.as_str()))
            .map(|(_, y)| *y)
            .collect();
        hauteurs.sort_by(f32::total_cmp);
        hauteurs.dedup_by(|a, b| (*a - *b).abs() < 1.0);
        assert_eq!(
            hauteurs.len(),
            1,
            "à {largeur:.0}x{hauteur:.0}, la rangée des vues s'étale sur \
             {} lignes : {hauteurs:?}",
            hauteurs.len()
        );
    }

    // 2. CE QU'ON REGARDE RESTE VISIBLE. Une barre qui cache l'onglet
    //    en cours ne dit plus où l'on est — et c'est le piège exact
    //    d'une fenêtre d'onglets qui commence toujours au premier.
    let dernière = *VueCentrale::ALL.last().expect("au moins une vue");
    let vus = peindre(MIN_SCREEN[0], MIN_SCREEN[1], dernière);
    assert!(
        vus.iter().any(|(texte, _)| texte == dernière.label(s)),
        "la vue regardée doit rester dans la rangée : {vus:?}"
    );

    // 3. ET LE COMPTE DIT COMBIEN IL EN RESTE. « +5 » plutôt que trois
    //    points : le nombre se lit dans toutes les langues, et il dit
    //    ce que des points taisent.
    //
    //    LES VUES PRÉSENTES, et non les neuf du tableau : la rangée ne
    //    porte que celles que les modules allumés et le niveau
    //    laissent. Compter les neuf annonçait six cachées là où le
    //    bouton en disait cinq — et c'est le bouton qui avait raison.
    let app = NamsdecApp::default();
    let présentes = VueCentrale::ALL
        .into_iter()
        .filter(|vue| app.vue_dans_la_rangée(*vue) && app.vue_au_niveau(*vue))
        .count();
    let vus = peindre(MIN_SCREEN[0], MIN_SCREEN[1], VueCentrale::Source);
    let montrées = vus
        .iter()
        .filter(|(texte, _)| noms.contains(&texte.as_str()))
        .count();
    let cachées = présentes - montrées;
    assert!(cachées > 0, "à cette taille, tout ne tient pas : {vus:?}");
    assert!(
        vus.iter().any(|(texte, _)| texte == &format!("+{cachées}")),
        "le bouton annonce les {cachées} vues cachées : {vus:?}"
    );

    // 4. ET RIEN N'EST PERDU : le bouton s'ouvre, et les vues qui ne
    //    tenaient pas y sont toutes, nommées. C'est ce qui autorise la
    //    rangée à en cacher — sans cette liste, une vue sans onglet
    //    n'existerait plus pour qui regarde l'écran.
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let écran =
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(MIN_SCREEN[0], MIN_SCREEN[1]));
    let image = |app: &mut NamsdecApp, events: Vec<egui::Event>| -> Vec<String> {
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            events,
            focused: true,
            ..Default::default()
        };
        let mut textes = Vec::new();
        let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                textes.push(texte.galley.text().to_owned());
            }
        }
        sortie.textures_delta.clear();
        textes
    };
    // Le bouton se repère à son étiquette : on relève son rectangle à
    // la troisième image, la rangée étant alors posée.
    let mut bouton = None;
    for _ in 0..3 {
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            focused: true,
            ..Default::default()
        };
        let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape
                && texte.galley.text() == format!("+{cachées}")
            {
                bouton = Some(cadre_du_texte(texte).center());
            }
        }
        sortie.textures_delta.clear();
    }
    let bouton = bouton.expect("le bouton du débordement est peint");
    // L'APPUI ET LE RELÂCHEMENT SONT DEUX IMAGES : egui décide d'un
    // clic au relâchement, sur un widget qu'il a vu appuyé à l'image
    // d'avant.
    image(
        &mut app,
        vec![
            egui::Event::PointerMoved(bouton),
            egui::Event::PointerButton {
                pos: bouton,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let ouvert = image(
        &mut app,
        vec![egui::Event::PointerButton {
            pos: bouton,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    let ouvert = if ouvert.iter().filter(|t| noms.contains(&t.as_str())).count() > montrées {
        ouvert
    } else {
        image(&mut app, Vec::new())
    };
    let listées = ouvert.iter().filter(|t| noms.contains(&t.as_str())).count();
    assert!(
        listées >= présentes,
        "les {cachées} vues cachées doivent toutes être au menu : \
         {listées} nommées sur {présentes} : {ouvert:?}"
    );

    // 5. ET CE QU'ELLE ÉCRIT GARDE SON AIR.
    //
    // **Trouvé à la capture, et ce n'était PAS un recouvrement** : à
    // 800x520, le disque rouge du compteur de problèmes tombait à DEUX
    // PIXELS ET DEMI du chiffre de « +6 ». Rien ne se peignait sur rien
    // — la règle « deux textes ne se recouvrent jamais » n'avait rien à
    // dire, et pour cause —, mais à l'échelle 1 les deux se lisaient
    // comme un seul objet cassé, le disque paraissant avaler le
    // chiffre.
    //
    // C'est la règle des cadres qui se touchent, déjà écrite pour
    // « Détacher ». On mesure donc l'ÉCART, et non le recouvrement :
    // un invariant qui ne peut pas échouer ne prouve rien, et celui du
    // recouvrement ne le pouvait pas ici — la rangée alloue ses objets
    // l'un après l'autre.
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_FAUTIF.to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    let (mut mots, mut dessins): (Vec<(String, egui::Rect)>, Vec<egui::Rect>) =
        (Vec::new(), Vec::new());
    for _ in 0..3 {
        mots.clear();
        dessins.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(texte) => {
                    mots.push((texte.galley.text().to_owned(), cadre_du_texte(texte)));
                }
                egui::epaint::Shape::Circle(c) => dessins.push(egui::Rect::from_center_size(
                    c.center,
                    egui::vec2(c.radius, c.radius) * 2.0,
                )),
                egui::epaint::Shape::Path(p) => dessins.push(p.visual_bounding_rect()),
                _ => {}
            }
        }
        sortie.textures_delta.clear();
    }
    let bouton = mots
        .iter()
        .find(|(texte, _)| texte.starts_with('+') && texte[1..].parse::<usize>().is_ok())
        .map(|(_, r)| *r)
        .expect("le bouton du débordement est peint");
    let voisin = dessins
        .iter()
        .filter(|d| {
            (d.center().y - bouton.center().y).abs() < 10.0 && d.left() >= bouton.right() - 1.0
        })
        .map(|d| d.left())
        .min_by(f32::total_cmp);
    if let Some(gauche) = voisin {
        let écart = gauche - bouton.right();
        assert!(
            écart >= 6.0,
            "le compte des onglets cachés est collé à ce qui le suit : \
             {écart:.1} px. Deux objets voisins qui ne respirent pas se \
             lisent comme un seul"
        );
    }
}

/// Aucun survol ne déplace ce qui l'entoure.
///
/// **Signalé sur capture le 5 novembre 2026** : « des glitchs quand le
/// curseur se positionne sur un bouton ». Mesuré : au survol, la page
/// des Outils descendait de VINGT-QUATRE pixels.
///
/// La chaîne est de celles qu'on ne devine pas, et aucune relecture du
/// code ne l'aurait donnée. Le contenu de cette page tient à trois
/// pixels près dans sa fenêtre ; la barre de défilement paraît alors
/// d'une image à l'autre, et la largeur offerte change d'autant. Trois
/// pixels suffisent pour qu'une rangée qui se replie — un avertissement
/// et son bouton — ne tienne plus sur une ligne : le bouton bascule à la
/// suivante, la page grandit d'une ligne, et tout ce qui suit descend.
/// **Le pointeur n'y est pour rien** ; il ne fait que déclencher l'image
/// où la bascule se produit.
///
/// Et un pixel de plus venait du bouton lui-même : son cadre n'a pas de
/// trait au repos et en reçoit un au survol, ce qu'egui compte dans la
/// taille. Le trait garde donc sa largeur au repos, et n'y perd que sa
/// couleur.
///
/// LA LISTE DES OUTILS EST SEMÉE, et non lue sur la machine : ce
/// contrôle passerait tout seul là où aucun outil n'est installé, et
/// c'est précisément la page longue qui l'intéresse.
#[test]
fn aucun_survol_ne_deplace_ce_qui_l_entoure() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Tools,
        ..Default::default()
    };
    // Tous les outils connus, donnés pour présents : la page est alors
    // la plus longue qu'elle sache être, celle où la barre de
    // défilement se décide à trois pixels près.
    app.tools_ready = namsdec_disasm::known::all()
        .into_iter()
        .map(|outil| {
            (
                outil.plugin.id.clone(),
                namsdec_disasm::known::Availability::Ready,
            )
        })
        .collect();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1300.0, 1000.0));
    let relever =
        |app: &mut NamsdecApp, pointeur: Option<egui::Pos2>| -> Vec<(String, egui::Rect)> {
            let mut vus = Vec::new();
            for _ in 0..4 {
                let events = match pointeur {
                    Some(pos) => vec![egui::Event::PointerMoved(pos)],
                    None => Vec::new(),
                };
                let entrée = egui::RawInput {
                    screen_rect: Some(écran),
                    events,
                    focused: true,
                    ..Default::default()
                };
                vus.clear();
                let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
                for clipped in &sortie.shapes {
                    if let egui::epaint::Shape::Text(t) = &clipped.shape {
                        vus.push((t.galley.text().to_owned(), cadre_du_texte(t)));
                    }
                }
                sortie.textures_delta.clear();
            }
            vus
        };

    let au_repos = relever(&mut app, None);
    // Les boutons de la page, repérés à leur libellé : on les survole
    // un par un, et rien ne doit bouger.
    let s = strings(Lang::Fr);
    let cibles: Vec<egui::Pos2> = au_repos
        .iter()
        .filter(|(texte, _)| {
            texte == s.tools_recheck
                || texte == s.tools_enable_decompilation
                || texte.starts_with("Ajouter")
                || texte.starts_with("Retirer")
        })
        .map(|(_, r)| r.center())
        .collect();
    assert!(
        cibles.len() >= 2,
        "la page doit porter des boutons à survoler : {cibles:?}"
    );

    for cible in cibles {
        let survolé = relever(&mut app, Some(cible));
        assert_eq!(
            survolé.len(),
            au_repos.len(),
            "le survol de {cible:?} ajoute ou retire du texte"
        );
        for ((texte, avant), (_, après)) in au_repos.iter().zip(survolé.iter()) {
            let écart = (avant.min - après.min).abs();
            assert!(
                écart.x < 0.5 && écart.y < 0.5,
                "le survol de {cible:?} déplace « {} » de {:.1} x {:.1} px",
                texte.chars().take(30).collect::<String>(),
                écart.x,
                écart.y
            );
        }
    }
}

/// Une capture ne dépend pas de la scène rendue juste avant.
///
/// C'EST LE CONTRÔLE QUI A TROUVÉ LE CACHE DE GALLEYS PÉRIMÉ, le
/// 18 novembre 2026, et il reste parce qu'aucune mesure ne pouvait le
/// voir : les invariants du banc lisent `output.shapes`, où le texte
/// est JUSTE — ce sont les rectangles de glyphes, un cran plus bas, qui
/// désignaient les cases d'un atlas disparu. Le défaut ne se voyait
/// qu'à l'image, et seulement si l'on savait à quoi la comparer.
///
/// Le principe : rendre toutes les scènes à l'endroit, puis à l'envers,
/// et comparer octet à octet. Une scène dont l'image change selon ce
/// qui précède garde de l'état par-dessus son contexte — et ce qu'elle
/// montre alors n'est pas ce que le programme ferait. Six scènes sur
/// cent trente-six différaient ; elles peignaient le programme
/// d'accueil en charabia.
///
/// Ce n'est pas la même question que « rendre deux fois avant de
/// comparer » : deux rendus de SUITE étaient parfaitement identiques,
/// et l'image fausse était stable. L'ordre, lui, la faisait bouger.
///
/// IGNORÉ PAR DÉFAUT : deux rendus complets, une minute, et un pilote
/// graphique logiciel entier.
///
/// ```text
/// cargo test -p namsdec-gui --release -- --ignored --nocapture aucune_capture_ne_depend_de_celle_d_avant
/// ```
#[test]
#[ignore = "rend toutes les scènes deux fois ; se demande explicitement"]
fn aucune_capture_ne_depend_de_celle_d_avant() {
    use egui_kittest::Harness;

    let rendu = |envers: bool| {
        let mut images: std::collections::BTreeMap<String, Vec<u8>> = Default::default();
        let mut toutes = scènes();
        if envers {
            toutes.reverse();
        }
        for (nom, _, prépare) in toutes {
            let mut app = NamsdecApp::default();
            prépare(&mut app);
            app.values_changed_at = None;
            let taille = écran_du_banc()
                .screen_rect
                .map(|r| r.size())
                .unwrap_or(egui::vec2(1600.0, 1100.0));
            let mut harness = Harness::builder()
                .with_size(taille)
                .wgpu()
                .build_ui(move |ui| {
                    app.apply_theme(ui.ctx());
                    app.draw(ui)
                });
            for _ in 0..6 {
                harness.step();
            }
            // Le même pas de deux que les captures : l'atlas se
            // construit en peignant, et le premier rendu emploie
            // encore l'ancienne texture.
            let _ = harness.render();
            harness.step();
            if let Ok(image) = harness.render() {
                images.insert(nom, image.into_raw());
            }
        }
        images
    };

    let endroit = rendu(false);
    let envers = rendu(true);
    assert!(
        endroit.len() > 20,
        "le banc a plus de vingt scènes : {}",
        endroit.len()
    );

    let fautives: Vec<&String> = endroit
        .iter()
        .filter(|(nom, pixels)| envers.get(*nom).is_some_and(|autre| autre != *pixels))
        .map(|(nom, _)| nom)
        .collect();
    assert!(
        fautives.is_empty(),
        "{} scène(s) changent selon ce qui a été rendu avant — \
         elles gardent de l'état par-dessus leur contexte :\n  {}",
        fautives.len(),
        fautives
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// Le bouton de don se voit éteint quand il ne mène nulle part.
///
/// La fenêtre « À propos » écrit, sous l'image : « Tant qu'elle est
/// vide, ce bouton reste éteint plutôt que de mener nulle part. »
/// L'image, elle, était peinte presque à plein — `add_enabled_ui` fond
/// une forme de MOITIÉ vers le fond de la fenêtre, ce qui éteint un
/// gris et ne fait rien à un carré orange saturé : 134,6 de clarté
/// moyenne contre 153,3, douze pour cent. Personne ne lit douze pour
/// cent comme « éteint », et la phrase disait donc le contraire de ce
/// qu'on voyait.
///
/// Le contrôle lit la TEINTE de l'image peinte — le `fill` du
/// rectangle texturé — dans les deux états, et demande que l'éteinte
/// soit nettement plus sombre. Le facteur deux est le minimum de ce
/// qui se voit ; remettre la teinte blanche dans les deux cas fait
/// rougir cette ligne.
#[test]
fn le_bouton_de_don_eteint_se_voit_eteint() {
    let teinte = |adresse: &str| {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        app.show_about = true;
        app.donation_url = adresse.to_owned();
        let mut trouvée = None;
        for _ in 0..4 {
            trouvée = None;
            let mut sortie = ctx.run_ui(écran_du_banc(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                // Le rectangle texturé le plus bas de la fenêtre :
                // l'image du repas est sous le logo, qui est le seul
                // autre.
                if let egui::epaint::Shape::Rect(r) = &clipped.shape
                    && r.brush.is_some()
                    && trouvée.is_none_or(|(haut, _)| r.rect.top() > haut)
                {
                    trouvée = Some((r.rect.top(), r.fill));
                }
            }
            sortie.textures_delta.clear();
        }
        trouvée.expect("l'image du don est peinte").1
    };

    let allumée = teinte("https://exemple.invalide/don");
    let éteinte = teinte("");
    // La clarté de la teinte, canaux pondérés comme pour le contraste.
    let clarté = |c: egui::Color32| {
        0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b())
    };
    assert!(
        clarté(éteinte) * 2.0 <= clarté(allumée),
        "sans adresse de don, l'image est peinte presque aussi vive \
         qu'avec — teinte éteinte {éteinte:?} ({:.0}) contre allumée \
         {allumée:?} ({:.0}) —, alors que la phrase au-dessous annonce \
         un bouton éteint",
        clarté(éteinte),
        clarté(allumée),
    );
}

/// La bande de la ligne courante tient dans son cadre.
///
/// LE DÉFAUT, mesuré le 18 novembre 2026 sur la capture de la scène
/// « établi / le détail de l'instruction », à 1600x1100. La bande
/// prenait la hauteur de ce qu'elle portait, bornée à `LIGNE_MAX` — et
/// quand le contenu en demandait plus, egui lui donnait quand même la
/// place, EN DEHORS du cadre : le panneau occupait 734..924, et son
/// contenu était découpé sur 710..900. Vingt-quatre pixels au-dessus du
/// bord, là où le fond de l'éditeur est peint par-dessus.
///
/// Ce qui s'y perdait n'était pas un détail : la ligne qui NOMME
/// l'instruction, `0x400024 jne short boucle`, c'est-à-dire la raison
/// d'être de la bande. Elle était recouverte, et rien ne défilait pour
/// aller la chercher. Par le bas, « RCX 3 -> 2 » perdait seize pixels
/// sur dix-huit.
///
/// POURQUOI AUCUN AUTRE INVARIANT NE LE VOYAIT, et c'est la raison
/// d'écrire celui-ci. `aucune_phrase_ne_perd_de_hauteur_au_decoupage`
/// mesure une encre contre SON découpage — or le découpage était
/// complice, puisqu'il sortait du cadre avec le texte. Et
/// `no_two_texts_of_a_scene_ever_overlap` compare des textes entre eux,
/// alors que ce qui recouvrait ici est un RECTANGLE, le fond d'un
/// panneau peint après. Le défaut passait entre les deux.
///
/// LA RÈGLE : un panneau découpe son contenu À SON CADRE. Ce qui ne
/// tient pas défile, et la barre le dit.
///
/// Éprouvé en remettant le défaut — rendre à la bande son
/// `.max_size(LIGNE_MAX)` et lui ôter sa zone défilante — : il nomme la
/// scène et les vingt-quatre pixels.
#[test]
fn la_bande_de_la_ligne_tient_dans_son_cadre() {
    let mut fautifs = Vec::new();
    for (nom, _, prépare) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        prépare(&mut app);
        app.values_changed_at = None;
        // La bande n'existe qu'à l'Établi.
        if !app.debutant {
            continue;
        }
        app.apply_theme(&ctx);
        let input = écran_du_banc();
        // QUATRE IMAGES, ET LA DERNIÈRE SEULE. La bande se donne sa
        // hauteur à partir du relevé de l'image PRÉCÉDENTE : la
        // première n'en a pas et prend la borne. Une mise en place qui
        // converge n'est pas un défaut ; ce qu'on refuse, c'est ce qui
        // reste faux une fois tout posé.
        for image in 0..4 {
            let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            let cadre = crate::panneaux::état(&ctx, "etabli_ligne").map(|état| état.outer_rect);
            if image < 3 {
                sortie.textures_delta.clear();
                continue;
            }
            let Some(cadre) = cadre else { continue };
            // RECONNAÎTRE LE DÉCOUPAGE DE LA BANDE, et lui seul. Trois
            // marques, et il faut les trois :
            //
            // — il prend toute la largeur, à la barre de défilement
            //   près (seize pixels) ;
            // — il n'est pas plus haut que ce que la bande peut
            //   prendre, ce qui écarte l'éditeur et la bande du
            //   résultat, qui tiennent toute la largeur eux aussi ;
            // — il croise la bande, ce qui écarte la barre de statut
            //   et la rangée des vues, larges et basses mais
            //   ailleurs.
            //
            // Le tri ne peut pas se faire sur l'identifiant : une
            // forme peinte ne porte que son découpage.
            let mut découpages: Vec<egui::Rect> = Vec::new();
            for clipped in &sortie.shapes {
                if !matches!(clipped.shape, egui::epaint::Shape::Text(_)) {
                    continue;
                }
                let clip = clipped.clip_rect;
                let large = (clip.width() - cadre.width()).abs() <= 16.0;
                let basse = clip.height() <= crate::etabli::LIGNE_MAX + 2.0;
                let croise = clip.bottom() > cadre.top() && clip.top() < cadre.bottom();
                if large && basse && croise && !découpages.contains(&clip) {
                    découpages.push(clip);
                }
            }
            for clip in découpages {
                let dehors = (cadre.top() - clip.top()).max(clip.bottom() - cadre.bottom());
                if dehors > 1.0 {
                    fautifs.push(format!(
                        "{nom} : la bande occupe {:.1}..{:.1} et découpe son contenu \
                         sur {:.1}..{:.1} — {dehors:.1} px hors du cadre, recouverts \
                         par ce qui est peint après",
                        cadre.top(),
                        cadre.bottom(),
                        clip.top(),
                        clip.bottom(),
                    ));
                }
            }
            sortie.textures_delta.clear();
        }
    }
    assert!(
        fautifs.is_empty(),
        "la bande de la ligne courante déborde de son cadre — ce qui est \
         peint là n'est ni visible ni atteignable :\n  {}",
        fautifs.join("\n  ")
    );
}

/// UNE BARRE DE DÉFILEMENT ANNONCE QUELQUE CHOSE QU'ON PEUT ATTEINDRE.
///
/// **Le symptôme, vu sur capture avant d'être mesuré** : sur le premier
/// écran du mode débutant — un écran par ailleurs sombre et vide —, un
/// trait clair de soixante-douze pixels brillait au bord de la bande qui
/// dit ce que fait la ligne courante. Cette bande portait deux phrases
/// centrées, qui tenaient. Rien n'était hors de vue.
///
/// **La cause est une mesure reprise en entrée d'elle-même**, dans sa
/// forme la plus douce : la bande se donne la hauteur de son contenu,
/// donc la zone défilante reçoit exactement la hauteur de ce qu'elle
/// porte. « Le contenu tient-il ? » se répond alors par un flottant
/// contre lui-même, et egui tranche du mauvais côté. Se tailler juste
/// est précisément ce qui fait paraître la barre.
///
/// **La mesure, le 20 novembre 2026, sur les cent vingt-deux scènes** :
/// cinquante-six poignées remplissaient leur glissière à plus de 96 %.
/// Elles se séparaient nettement en deux familles, et c'est la
/// séparation qui a donné le seuil :
///
/// - **97 à 98 %** — les listes d'analyse (721 px sur 733), la
///   chronologie, le profil. Douze pixels hors de vue sur sept cent
///   trente-trois : c'est peu, c'est vrai, et la barre a raison de le
///   dire. Rien à corriger ;
/// - **99 à 101 %** — les trois écrans du mode débutant, et eux seuls.
///   Soixante-douze sur soixante-treize, cent cinquante-deux sur cent
///   cinquante-trois. Un pixel, c'est-à-dire rien.
///
/// Le seuil est donc à 99 %, et il vient de ce que la mesure a montré,
/// non d'un goût. Corrigé par [`LIGNE_JEU`] : deux pixels de jeu entre
/// ce que la bande porte et la hauteur qu'elle se donne. Éprouvé en les
/// retirant — le contrôle nomme les trois scènes.
#[test]
fn une_barre_de_defilement_annonce_quelque_chose_d_atteignable() {
    const PLEINE: f32 = 0.99;
    let mut fautives: Vec<String> = Vec::new();

    for (nom, _, prépare) in scènes() {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        prépare(&mut app);
        let entrée = egui::RawInput {
            predicted_dt: 0.5,
            ..écran_du_banc()
        };
        let mut formes = Vec::new();
        // Six images : une surface ancrée ne paraît qu'à la deuxième, et
        // la bande qui se mesure sur son contenu met trois images à se
        // poser.
        for _ in 0..6 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            formes = std::mem::take(&mut sortie.shapes);
            sortie.textures_delta.clear();
        }

        // GLISSIÈRE ET POIGNÉE SE RECONNAISSENT À LEUR FORME, et non à
        // leur couleur : un thème clair inverse les deux, et le banc
        // passe les six thèmes. Étroites, hautes, et l'une dans l'autre.
        let barres: Vec<egui::Rect> = formes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Rect(r)
                    if r.fill.a() > 0
                        && (4.0..14.0).contains(&r.rect.width())
                        && r.rect.height() > 24.0 =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();

        for glissière in &barres {
            for poignée in &barres {
                if poignée == glissière || poignée.height() > glissière.height() {
                    continue;
                }
                let dedans = (poignée.min.x - glissière.min.x).abs() < 2.0
                    && poignée.min.y >= glissière.min.y - 1.0
                    && poignée.max.y <= glissière.max.y + 1.0;
                let part = poignée.height() / glissière.height();
                if dedans && part > PLEINE {
                    fautives.push(format!(
                        "{nom} : une poignée de {:.0} px dans une glissière de {:.0} \
                         ({:.0} %) en {:?} — elle annonce ce que personne n'atteindra",
                        poignée.height(),
                        glissière.height(),
                        part * 100.0,
                        glissière.min
                    ));
                }
            }
        }
    }

    fautives.sort();
    fautives.dedup();
    assert!(
        fautives.is_empty(),
        "{} barre(s) de défilement ne servent à rien :\n  {}",
        fautives.len(),
        fautives.join("\n  ")
    );
}

/// UNE PHRASE NE SE COMPOSE PAS SUR LA LARGEUR DE LA FENÊTRE.
///
/// Un œil qui revient à la ligne perd son rang au-delà d'une quinzaine
/// de mots ; c'est la raison pour laquelle un livre n'est pas composé
/// sur la largeur de sa page. [`crate::widgets::LARGEUR_DE_LECTURE`]
/// existait pour cela depuis les états vides — et ne servait qu'à eux.
///
/// **Mesuré le 20 novembre 2026 sur les scènes du mode débutant**, en
/// comptant les caractères de chaque ligne RÉELLEMENT composée :
///
/// - l'énoncé de l'exercice : **278 caractères** sur une ligne pour
///   « Ce que la boucle a accumulé ». C'est le texte qu'un débutant lit
///   avant tout le reste ;
/// - l'explication d'une erreur de nasm : **269**. C'est le texte qu'il
///   lit au moment précis où il est bloqué ;
/// - l'énoncé d'un exercice d'écriture : **191** ;
/// - la phrase qui dit ce que fait le désassembleur en clair : **140**,
///   alors que son propre commentaire la disait « de deux ou trois
///   lignes ».
///
/// Les trois premières tombent à 99 une fois composées sur la largeur
/// de lecture, la quatrième à 97.
///
/// **Le seuil est à cent cinquante, et non à cent.** Cent serait
/// l'idéal typographique, et plusieurs bandes ne peuvent pas l'atteindre
/// sans grandir : celle de la ligne courante est plafonnée en hauteur,
/// et la resserrer y ramènerait la barre de défilement que
/// `une_barre_de_defilement_annonce_quelque_chose_d_atteignable`
/// interdit. Cent cinquante est la ligne au-delà de laquelle un texte
/// n'est plus lu mais parcouru ; les quatre défauts ci-dessus la
/// franchissaient tous, et ce qui reste tient à 137.
///
/// **Les textes TRONQUÉS n'en sont pas**, et c'est une correction du
/// relevé plutôt qu'un défaut : la note de la frise fait deux cent
/// trois caractères et n'en peint qu'une ligne suivie de « … », le
/// reste au survol. Un relevé qui compte le texte d'une galley élidée
/// mesure ce qui n'est pas peint — exactement l'erreur que le banc a
/// déjà corrigée sur le contraste.
#[test]
fn une_phrase_ne_se_compose_pas_sur_la_largeur_de_la_fenetre() {
    const TROP_LONG: usize = 150;
    let mut fautives: Vec<String> = Vec::new();

    for (nom, _, prépare) in scènes() {
        // LE MODE DÉBUTANT, et lui seul : c'est là que la règle
        // compte, et c'est là qu'on a mesuré. Le mode avancé montre des
        // listings, des vidages et des tables, où une ligne longue est
        // une donnée et non une phrase.
        if !nom.contains("établi") && !nom.contains("débutant") {
            continue;
        }
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        prépare(&mut app);
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

        for clipped in &formes {
            let egui::epaint::Shape::Text(t) = &clipped.shape else {
                continue;
            };
            // Ce qui est élidé n'est pas composé : une seule ligne est
            // peinte, et le reste se survole.
            if t.galley.elided {
                continue;
            }
            for ligne in &t.galley.rows {
                let texte = ligne.text();
                // UNE PHRASE, ET NON UNE LIGNE DE CODE NI UNE
                // ÉTIQUETTE. Huit espaces : un programme en assembleur
                // en a deux ou trois par ligne, une adresse et ses
                // octets aucun, et une phrase française en a une
                // dizaine dès qu'elle dépasse la moitié du seuil.
                if texte.chars().filter(|c| *c == ' ').count() < 8 {
                    continue;
                }
                let combien = texte.chars().count();
                if combien > TROP_LONG {
                    let début: String = texte.chars().take(60).collect();
                    fautives.push(format!(
                        "{nom} : {combien} caractères sur une ligne — « {début}… »"
                    ));
                }
            }
        }
    }

    fautives.sort();
    fautives.dedup();
    assert!(
        fautives.is_empty(),
        "{} phrase(s) composées sur plus de {TROP_LONG} caractères :\n  {}",
        fautives.len(),
        fautives.join("\n  ")
    );
}

/// Le clic droit sur un onglet ouvre vraiment son menu, et ce menu se
/// lit en entier.
///
/// Le test précédent (`le_menu_d_un_onglet_ferme_les_autres_un_cote_ou_tout`)
/// tient ce que les entrées FONT ; celui-ci tient qu'elles s'ouvrent au
/// clic droit, qu'elles sont toutes là, et que le menu tient dans
/// l'écran plutôt que de sortir par le bas ou par la droite.
#[test]
fn le_clic_droit_sur_un_onglet_ouvre_son_menu() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = NamsdecApp::default();
    for rang in 0..3 {
        let chemin = dir.path().join(format!("f{rang}.asm"));
        std::fs::write(&chemin, "global _start\nsection .text\n_start:\n    hlt\n").unwrap();
        app.open_file(chemin);
    }

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let base = écran_du_banc();
    let frame = |app: &mut NamsdecApp, input: egui::RawInput| {
        let mut sortie = ctx.run_ui(input, |ui| app.draw(ui));
        sortie.textures_delta.clear();
    };
    // Deux poses avant de viser : la barre d'onglets doit avoir sa
    // place avant qu'on clique dedans.
    for _ in 0..2 {
        frame(&mut app, base.clone());
    }

    // L'onglet du milieu, là où il est écrit.
    let (_, écrits) = ce_qui_est_écrit(&ctx, &mut app, None, 2);
    let onglet = écrits
        .iter()
        .find(|(texte, rect)| texte.contains("f1.asm") && rect.top() < 200.0)
        .map(|(_, rect)| *rect)
        .expect("l'onglet du milieu est peint dans la barre");
    let cible = onglet.center();

    let mut presse = base.clone();
    presse.events.push(egui::Event::PointerMoved(cible));
    presse.events.push(egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    frame(&mut app, presse);
    let mut relâche = base.clone();
    relâche.events.push(egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    frame(&mut app, relâche);

    // Le menu est ouvert : on relève ce qu'il écrit, sans bouger la
    // souris — le pointeur reste sur l'onglet.
    let mut tenu = base.clone();
    tenu.events.push(egui::Event::PointerMoved(cible));
    let mut lignes: Vec<(String, egui::Rect)> = Vec::new();
    for _ in 0..2 {
        lignes.clear();
        let mut sortie = ctx.run_ui(tenu.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                for ligne in &t.galley.rows {
                    if ligne.glyphs.is_empty() {
                        continue;
                    }
                    lignes.push((
                        ligne.text().trim().to_owned(),
                        egui::Rect::from_min_size(t.pos + ligne.pos.to_vec2(), ligne.size),
                    ));
                }
            }
        }
        sortie.textures_delta.clear();
    }

    let s = strings(app.lang);
    let écran = base.screen_rect.unwrap();
    let mut cadre_du_menu: Option<egui::Rect> = None;
    // Le menu de l'onglet du milieu : les cinq entrées, puisqu'il a des
    // voisins des deux côtés.
    for entrée in [
        s.tab_menu_close,
        s.tab_menu_close_others,
        s.tab_menu_close_all,
        s.tab_menu_close_left,
        s.tab_menu_close_right,
    ] {
        let posée = lignes
            .iter()
            .find(|(texte, _)| texte == entrée)
            .unwrap_or_else(|| {
                panic!(
                    "« {entrée} » doit être dans le menu : {:?}",
                    lignes.iter().map(|(t, _)| t).take(30).collect::<Vec<_>>()
                )
            });
        assert!(
            écran.contains_rect(posée.1),
            "« {entrée} » sort de l'écran : {:?} hors de {écran:?}",
            posée.1
        );
        cadre_du_menu = Some(match cadre_du_menu {
            Some(cadre) => cadre.union(posée.1),
            None => posée.1,
        });
    }
    let cadre_du_menu = cadre_du_menu.expect("le menu a au moins une entrée");

    // ET L'INFOBULLE DU NOM NE SE PEINT PAS PAR-DESSUS. Le pointeur est
    // resté sur l'onglet : sans précaution, egui montrerait le nom du
    // fichier dans une bulle posée sur les entrées du menu.
    let sur_le_menu: Vec<&egui::Rect> = lignes
        .iter()
        .filter(|(texte, rect)| texte == "f1.asm" && rect.intersects(cadre_du_menu))
        .map(|(_, rect)| rect)
        .collect();
    assert!(
        sur_le_menu.is_empty(),
        "l'infobulle du nom se peint sur le menu ({cadre_du_menu:?}) : {sur_le_menu:?}"
    );
}

/// DANS UNE FICHE QU'ON LIT, TIRER LE TEXTE NE DÉPLACE PAS LA FENÊTRE.
///
/// Le défaut est arrivé par ces mots : « lors de la consultation de
/// théorie, l'ascenseur ne fonctionnait pas, pas moyen d'aller plus
/// bas ». Mesuré, il disait autre chose : on tirait le texte pour
/// descendre, LA FENÊTRE descendait de cent pixels — exactement le
/// geste du pointeur — et le texte ne défilait pas d'une ligne.
///
/// La cause est dans egui : une fenêtre à qui l'on retire sa barre de
/// titre bascule en « glisser partout » (`WindowDrag::Anywhere`), et sa
/// propre documentation prévient que cela « interfère avec la sélection
/// et le glissement du contenu ». Les vingt-cinq fenêtres de namsdec
/// portent leur propre bandeau, donc toutes y sont.
///
/// ON TIRE EN PLUSIEURS POINTS, et non en un seul : sous le pointeur il
/// peut y avoir un mot cliquable, un label sélectionnable ou rien du
/// tout, et chacun répond autrement. Un contrôle qui viserait un point
/// unique mesurerait ce qu'il y a dessous, non la règle. Les deux
/// moitiés comptent : aucun point ne déplace la fenêtre, et au moins un
/// fait défiler — une fiche immobile dont rien ne descend serait le
/// même échec vu de celui qui lit.
///
/// IL A SERVI UNE SECONDE FOIS, et c'est pour cela qu'il tire en
/// plusieurs points. Les théories complétées du 23 novembre 2026
/// remplissent la colonne du milieu, et le glissement a cessé de
/// défiler sur trois des cinq théories essayées : un label d'egui est
/// sélectionnable par défaut, donc il prend le glissement pour
/// sélectionner et la zone ne voit rien. La promesse ne tenait que là
/// où le pointeur manquait le texte. Les labels d'une zone qu'on lit
/// ne sont plus sélectionnables — voir `zone_de_lecture`, qui en dit
/// le prix.
#[test]
fn tirer_le_texte_d_une_fiche_ne_deplace_pas_sa_fenetre() {
    type Fiche = (&'static str, &'static str, fn(&mut NamsdecApp));
    let fiches: Vec<Fiche> = vec![
        ("théorie", THEORIE_WINDOW, |app| {
            app.exercises = true;
            app.exercise_packs = vec![builtin_pack()];
            app.load_exercise_now(&format!("namsdec/{THEORIE_DU_BANC}"));
            app.show_theorie = true;
        }),
        ("fiche d'instruction", "instruction_help", |app| {
            app.show_help = true;
            app.open_help("cmp");
        }),
    ];
    // Le plus petit écran que l'application accepte : c'est là que les
    // fiches défilent, et une fiche qui tient en entier ne dit rien de
    // ce contrôle.
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, MIN_SCREEN.into());
    let mut fautes = Vec::new();
    for (nom, id, ouvre) in fiches {
        let mut déplacements = Vec::new();
        let mut a_défilé = false;
        let mut vu_un_ascenseur = false;
        // Cinq hauteurs dans le corps de la fiche, du quart au trois
        // quarts : on évite le bandeau en haut et la barre d'action en
        // bas, qui ne sont pas du texte qu'on lit.
        for part in [0.3f32, 0.4, 0.5, 0.6, 0.7] {
            let ctx = egui::Context::default();
            let mut app = NamsdecApp::default();
            app.apply_theme(&ctx);
            ouvre(&mut app);
            let base = egui::RawInput {
                screen_rect: Some(écran),
                ..Default::default()
            };
            let image = |ctx: &egui::Context, app: &mut NamsdecApp, events: Vec<egui::Event>| {
                let mut entrée = base.clone();
                entrée.events = events;
                let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
                sortie.textures_delta.clear();
                sortie
            };
            // Huit images : une fenêtre est AMENÉE à sa taille.
            let mut sortie = None;
            for _ in 0..8 {
                sortie = Some(image(&ctx, &mut app, Vec::new()));
            }
            let cadre = egui::AreaState::load(&ctx, egui::Id::unique(id))
                .map(|a| a.rect())
                .unwrap_or(egui::Rect::NOTHING);
            // Le pouce de l'ascenseur de CETTE fenêtre : le plus court
            // des rectangles étroits posés contre son bord droit.
            let pouce = |sortie: &egui::FullOutput| -> Option<egui::Rect> {
                sortie
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::epaint::Shape::Rect(r)
                            if r.rect.width() < 20.0
                                && r.rect.height() > 40.0
                                && cadre.contains(r.rect.center())
                                && r.rect.center().x > cadre.right() - 40.0 =>
                        {
                            Some(r.rect)
                        }
                        _ => None,
                    })
                    .min_by(|a, b| a.height().total_cmp(&b.height()))
            };
            let avant = pouce(sortie.as_ref().expect("une image"));
            vu_un_ascenseur |= avant.is_some();
            // On saisit le corps et l'on tire vers le haut : c'est le
            // geste de qui pousse la page pour voir la suite.
            let mut pointeur = egui::pos2(cadre.center().x, cadre.top() + cadre.height() * part);
            image(&ctx, &mut app, vec![egui::Event::PointerMoved(pointeur)]);
            image(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(pointeur),
                    egui::Event::PointerButton {
                        pos: pointeur,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                ],
            );
            let mut dernière = None;
            for _ in 0..4 {
                pointeur.y -= 25.0;
                dernière = Some(image(
                    &ctx,
                    &mut app,
                    vec![egui::Event::PointerMoved(pointeur)],
                ));
            }
            let après = egui::AreaState::load(&ctx, egui::Id::unique(id))
                .map(|a| a.rect())
                .unwrap_or(egui::Rect::NOTHING);
            let déplacée = (après.min - cadre.min).length();
            if déplacée > 1.0 {
                déplacements.push(format!(
                    "{:.0} % de haut en bas : {déplacée:.0} px",
                    part * 100.0
                ));
            }
            if let (Some(avant), Some(après)) =
                (avant, pouce(dernière.as_ref().expect("une image")))
                && (après.top() - avant.top()).abs() > 1.0
            {
                a_défilé = true;
            }
        }
        if !déplacements.is_empty() {
            fautes.push(format!(
                "« {nom} » : tirer le texte a DÉPLACÉ la fenêtre — {}",
                déplacements.join(", ")
            ));
        }
        if !vu_un_ascenseur {
            fautes.push(format!(
                "« {nom} » : aucun ascenseur à {:.0}x{:.0} — le contrôle ne \
                 mesure rien ; donnez-lui une fiche plus longue",
                écran.width(),
                écran.height()
            ));
        } else if !a_défilé {
            fautes.push(format!(
                "« {nom} » : aucun des cinq points tirés n'a fait défiler le texte"
            ));
        }
    }
    assert!(
        fautes.is_empty(),
        "une fiche qu'on lit se tire pour défiler, et non pour se déplacer :\n  {}",
        fautes.join("\n  ")
    );
}

/// SUR UN ÉCRAN DE TRAVAIL COURANT, AUCUNE THÉORIE NE DEMANDE DE
/// DÉFILER.
///
/// C'est une règle pédagogique avant d'être une règle de mise en page :
/// la théorie s'adresse à qui débute, et quelqu'un qui débute ne sait
/// pas encore qu'il manque un paragraphe sous le bord. Une explication
/// qu'il faut faire défiler est une explication qu'on lit à moitié.
///
/// La hauteur de ces fiches valait 55 % de l'écran — une fraction qui ne
/// regarde rien. À 1280x800 elle accordait 440 px là où la zone libre
/// entre les barres en offre 626, et la plus longue des cent trente-huit
/// théories en demande 498 : elle défilait pour cent quatre-vingt-six
/// pixels de place inutilisée. `place_pour_une_fiche` prend désormais la
/// place qu'il y a.
///
/// Ce que le contrôle mesure est l'ascenseur lui-même — un rectangle
/// étroit et haut contre le bord droit de la fenêtre —, c'est-à-dire ce
/// que l'œil voit, et non un calcul qu'on referait à l'identique.
///
/// Ce contrôle vieillira dans le bon sens : le jour où une théorie
/// nouvelle sera trop longue pour tenir d'un coup, il le dira, et c'est
/// la théorie qu'il faudra raccourcir.
///
/// CE JOUR EST ARRIVÉ, et il a fait les deux. Les cent trente-huit
/// théories ont été complétées pour qui débute — un passage chiffré
/// dans chacune, la médiane passant de 800 à 1597 caractères —, et
/// **quarante-neuf d'entre elles se sont mises à défiler**. La
/// réponse n'a pas été de les raccourcir toutes : la place manquait
/// en HAUTEUR, et la fiche faisait 640 px de large sur une zone de
/// travail qui en offre près du double. `largeur_pour_une_fiche` lui
/// donne désormais la largeur qu'il y a, et quarante-cinq des
/// quarante-neuf ont cessé de défiler sans qu'un mot ne change.
///
/// Les quatre dernières ont bel et bien été raccourcies, et elles ont
/// appris quelque chose : **le nombre de paragraphes coûte autant que
/// les caractères**. Une théorie de 1899 caractères en sept
/// paragraphes tient, là où une de 1708 en huit défile — chaque
/// paragraphe ajoute une dernière ligne partielle et l'air qui le
/// sépare du suivant. Resserrer un texte, ici, c'est d'abord fusionner
/// ce qui dit deux fois la même chose.
#[test]
fn aucune_theorie_ne_demande_de_defiler_sur_un_ecran_de_travail() {
    // Un portable courant, et non l'écran large du banc : c'est le
    // format où la règle a mordu.
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0));
    let pack = builtin_pack();
    let avec_théorie: Vec<String> = pack
        .exercises
        .iter()
        .filter(|exercice| exercice.theorie.is_some())
        .map(|exercice| exercice.id.clone())
        .collect();
    assert!(
        avec_théorie.len() > 100,
        "la série livrée porte plus de cent théories, pas {}",
        avec_théorie.len()
    );
    let mut défilent = Vec::new();
    for id in &avec_théorie {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        app.exercises = true;
        app.exercise_packs = vec![builtin_pack()];
        app.load_exercise_now(&format!("namsdec/{id}"));
        app.show_theorie = true;
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        };
        let mut sortie = None;
        // Huit images : une fenêtre est AMENÉE à sa taille.
        for _ in 0..8 {
            let mut image = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            image.textures_delta.clear();
            sortie = Some(image);
        }
        let cadre = egui::AreaState::load(&ctx, egui::Id::unique(THEORIE_WINDOW))
            .map(|a| a.rect())
            .unwrap_or(egui::Rect::NOTHING);
        let ascenseur = sortie
            .as_ref()
            .expect("une image")
            .shapes
            .iter()
            .any(|clipped| match &clipped.shape {
                egui::epaint::Shape::Rect(r) => {
                    r.rect.width() < 20.0
                        && r.rect.height() > 40.0
                        && cadre.contains(r.rect.center())
                        && r.rect.center().x > cadre.right() - 40.0
                }
                _ => false,
            });
        if ascenseur {
            défilent.push(id.clone());
        }
    }
    assert!(
        défilent.is_empty(),
        "à {:.0}x{:.0}, {} théorie(s) sur {} demandent de défiler — \
         une explication qu'on lit à moitié :\n  {}",
        écran.width(),
        écran.height(),
        défilent.len(),
        avec_théorie.len(),
        défilent.join(", ")
    );
}

/// DANS UNE FICHE, LA COULEUR D'APPEL NE VA QU'À CE QUI S'OUVRE.
///
/// La théorie promet, en toutes lettres et en bas de chaque fiche, que
/// « les mots en couleur ont leur propre fiche : un clic l'ouvre ».
/// Elle ne le tenait pas : un morceau de code entre accents graves était
/// peint de la couleur des mnémoniques qu'il ait une fiche ou non, et
/// seul un soulignement à un tiers d'opacité séparait les deux.
///
/// Mesuré sur les cent trente-huit théories livrées, dans les deux
/// langues : sur 1127 morceaux colorés, 284 — un quart — ne répondaient
/// pas au clic. `rax`, `al`, `db`, `0x88`, `section`… On clique, rien ne
/// vient, et l'on en conclut que les clics ne marchent pas ici. C'est la
/// même déception que l'ascenseur qui ne défile pas : une promesse faite
/// à l'œil, et non tenue.
///
/// Le contrôle relève les textes PEINTS, et non ce que le code croit
/// écrire : un mot qui nomme une instruction porte la couleur d'appel,
/// un mot qui n'en nomme aucune porte celle du texte.
#[test]
fn dans_une_fiche_la_couleur_d_appel_ne_va_qu_a_ce_qui_s_ouvre() {
    let ctx = egui::Context::default();
    let app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let theme = app.theme;
    // Deux morceaux, et un seul a une fiche. `rax` est le cas le plus
    // courant des 284 : un registre, que personne n'a jamais documenté
    // comme une instruction.
    assert!(
        namsdec_doc::lookup("mov").is_some(),
        "le témoin suppose que `mov` a une fiche"
    );
    assert!(
        namsdec_doc::lookup("rax")
            .filter(|e| e.kind == namsdec_doc::Kind::Instruction
                || e.kind == namsdec_doc::Kind::Concept)
            .is_none(),
        "le témoin suppose que `rax` n'en a pas"
    );
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 400.0),
        )),
        ..Default::default()
    };
    let mut couleurs: std::collections::BTreeMap<String, Vec<egui::Color32>> = Default::default();
    for _ in 0..3 {
        couleurs.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| {
            prose_avec_fiches_et_notions(
                ui,
                theme,
                Lang::Fr,
                "On écrit `mov` pour déplacer, et le résultat tombe dans `rax`.",
                true,
            );
        });
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                let écrit = texte.galley.job.text.trim().to_owned();
                if écrit == "mov" || écrit == "rax" {
                    couleurs
                        .entry(écrit)
                        .or_default()
                        .extend(texte.galley.job.sections.iter().map(|s| s.format.color));
                }
            }
        }
        sortie.textures_delta.clear();
    }
    let couleur = |mot: &str| -> egui::Color32 {
        *couleurs
            .get(mot)
            .and_then(|liste| liste.first())
            .unwrap_or_else(|| panic!("« {mot} » n'a pas été peint : {couleurs:?}"))
    };
    assert_eq!(
        couleur("mov"),
        theme.mnemonic,
        "« mov » a une fiche : il porte la couleur d'appel"
    );
    assert_ne!(
        couleur("rax"),
        theme.mnemonic,
        "« rax » n'a pas de fiche : il ne doit pas porter la couleur de ce qui s'ouvre, \
         sans quoi la fiche promet un clic qui ne vient pas"
    );
    assert_eq!(
        couleur("rax"),
        theme.text,
        "« rax » reste du code, en chasse fixe et de la couleur du texte"
    );
}

/// LE PROGRAMME D'ACCUEIL MONTRE LE BOUTON, ET NON UN RÉGLAGE.
///
/// C'est la toute première phrase qu'un débutant lit dans namsdec, et
/// elle l'envoyait dans « Préférences > Modules » pour trouver les
/// exercices. Le renvoi datait du temps où le module était éteint au
/// départ ; il est allumé par défaut depuis, et le bouton
/// « Exercices… » est à l'écran, en haut, au moment où l'on lit la
/// phrase. On ne fait pas chercher dans un panneau de réglages ce qui
/// est sous les yeux.
///
/// Le nombre d'étapes qu'elle annonce est gardé ailleurs, par
/// `le_programme_d_accueil_annonce_le_bon_nombre_d_exercices` : ce
/// contrôle-ci ne regarde que l'endroit où la phrase envoie.
#[test]
fn le_programme_d_accueil_montre_le_bouton_et_non_un_reglage() {
    // Le module est allumé au premier lancement : c'est ce qui rend le
    // renvoi vers les réglages fautif. Si cela changeait un jour, la
    // phrase devrait changer avec, et ce contrôle le dira.
    let réglages = Settings::default();
    assert!(
        réglages.exercises,
        "le module des exercices est allumé au départ — sinon, la phrase \
         d'accueil doit redire où l'allumer"
    );
    assert!(
        !DEFAULT_SOURCE.contains("Préférences"),
        "le programme d'accueil envoie dans les préférences pour quelque chose \
         qui est déjà à l'écran :\n{DEFAULT_SOURCE}"
    );
    let bouton = strings(Lang::Fr).exercises_open;
    assert!(
        DEFAULT_SOURCE.contains(bouton),
        "le programme d'accueil doit nommer le bouton « {bouton} » tel qu'il est \
         écrit dessus :\n{DEFAULT_SOURCE}"
    );
}
