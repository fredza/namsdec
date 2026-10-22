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
    let gauche = |lignes: &[(String, egui::Rect)], quoi: &str| -> f32 {
        lignes
            .iter()
            .filter(|(t, _)| t == quoi)
            .map(|(_, r)| r.left())
            .fold(f32::INFINITY, f32::min)
    };

    // Les titres de vue, qui ne portent pas de chevron : les six
    // partent du bord du panneau, et du même.
    let mut bords: Vec<(&str, f32)> = Vec::new();
    for onglet in SidebarTab::ALL {
        if onglet == SidebarTab::Re {
            continue; // absent tant qu'on ne l'a pas demandé
        }
        app.active_tab = onglet;
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
        let titre = onglet.title(s);
        let x = gauche(&lignes, titre);
        assert!(x.is_finite(), "« {titre} » est peint en tête de sa vue");
        bords.push((titre, x));
    }
    let premier = bords[0].1;
    let hors: Vec<String> = bords
        .iter()
        .filter(|(_, x)| (x - premier).abs() > 0.5)
        .map(|(quoi, x)| format!("« {quoi} » à {x:.1}"))
        .collect();
    assert!(
        hors.is_empty(),
        "les titres de vue ne partent pas du même bord : {premier:.1} px, sauf {}",
        hors.join(", ")
    );

    // Et le titre À CHEVRON du bandeau s'écarte de ce bord, mais de peu :
    // la largeur du chevron, pas davantage.
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
    let mut plancher = 0.0f32;
    // Deux zones depuis que l'état du processeur est une vue comme les
    // autres : le bandeau, puis le titre de la vue choisie — ici celle
    // du processeur, qui est la vue par défaut.
    for titre in [s.next_instruction, SidebarTab::Cpu.title(s)] {
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
            // parler de `availability_in` sans l'appeler.
            let corps = morceau
                .split_once('{')
                .map(|(_, reste)| reste)
                .unwrap_or(morceau);
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
    let Some((_, entête, _)) = textes.iter().find(|(t, _, _)| t.contains("Outils mémoire"))
    else {
        return;
    };
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
