//! Le châssis : barre de titre, barre d'état, barre d'outils, menus.
//!
//! Ce qui entoure le travail sans en faire partie. Les fichiers ouverts
//! et leurs onglets sont dans [`super::fichiers`] ; les commandes, dans
//! [`super::commandes`].
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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
    app.apply_theme(&ctx);
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
        app.apply_theme(&ctx);
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
    let source = include_str!("../menus.rs");
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
