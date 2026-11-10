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
                let rect = cadre_du_texte(text);
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

/// Les onglets du panneau se choisissent EN TÊTE DU PANNEAU, juste
/// au-dessus de ce qu'ils commandent.
///
/// Trois endroits en trois ans, et chacun pour une raison mesurée :
/// au fond du panneau d'abord — la rangée montait et descendait avec le
/// bandeau, on visait une cible qui bouge ; dans la barre d'outils
/// ensuite — mais une étiquette séparée de son contenu ne se lit plus
/// comme son étiquette, et six onglets ne tenaient pas dans la barre en
/// dessous de mille pixels, où ils la faisaient se dédoubler de
/// trente-et-un pixels pour toute la séance ; en tête du panneau enfin,
/// le 26 octobre 2026, où ils touchent ce qu'ils gouvernent et ne
/// bougent plus.
///
/// On mesure les deux : l'ordre lu de gauche à droite, et la place —
/// dans le panneau, sous le bandeau, au-dessus du contenu.
#[test]
fn the_sidebar_tabs_live_at_the_head_of_the_panel() {
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
    // Dans le panneau, et non dans la barre : à droite de l'écran, et
    // SOUS la barre d'outils.
    assert!(
        x[0] > 1280.0 - SIDEBAR_WIDTH,
        "la rangée est dans le panneau : {x:?}"
    );
    // Un programme en marche pour que la vue ait du contenu à montrer :
    // « RAX » est ce qu'on cherche sous la rangée.
    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        app.source = DEFAULT_SOURCE.to_owned();
        app.do_assemble();
        app.do_step();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, None, 4);
        let y = |quoi: &str| -> f32 {
            lignes
                .iter()
                .filter(|(t, r)| t == quoi && r.left() > 1280.0 - SIDEBAR_WIDTH)
                .map(|(_, r)| r.top())
                .fold(f32::INFINITY, f32::min)
        };
        let barre = lignes
            .iter()
            .filter(|(t, _)| t == s.action_reset)
            .map(|(_, r)| r.bottom())
            .fold(0.0_f32, f32::max);
        let rangée = y(s.tab_cpu);
        let contenu = y("RAX");
        assert!(
            rangée.is_finite() && contenu.is_finite(),
            "la rangée et le contenu de la vue sont peints : rangée \
             {rangée}, contenu {contenu}"
        );
        assert!(
            rangée > barre,
            "la rangée est sous la barre d'outils : {rangée:.0} contre \
             {barre:.0}"
        );
        assert!(
            rangée < contenu,
            "la rangée est au-dessus de ce qu'elle commande : {rangée:.0} \
             contre {contenu:.0}"
        );
    }
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
                case = Some(cadre_du_texte(t));
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

/// LE MESSAGE D'ÉTAT SE COUPE AU BORD, ET SE RELIT AU SURVOL.
///
/// Relevé sur la capture du 10 novembre 2026, sur l'Établi : la note
/// d'un binaire lié dynamiquement — quatre lignes — partait du bord
/// gauche de la frise et passait PAR DESSUS le bouton de la Scène,
/// puis par dessus RIP, puis hors de l'écran. Mesuré à 1600 px de
/// large : l'encre allait jusqu'à x = 2824.
///
/// LA CAUSE N'ÉTAIT PAS DANS LA FRISE mais dans une habitude d'egui :
/// une étiquette posée dans un `ui.horizontal` est en `Extend`, elle
/// s'allonge au-delà de la place qu'on lui laisse au lieu de s'y
/// couper. La barre d'état du mode avancé s'en gardait elle-même, en
/// posant `wrap_mode` avant d'appeler le message ; la frise de
/// l'Établi, écrite plus tard, ne le savait pas. La règle est donc
/// passée DANS `status_message_ui`, où elle n'a plus à être sue.
///
/// Et l'infobulle en dépendait : elle ne paraît que si l'on a coupé,
/// et l'on ne coupait jamais. Un message qui déborde l'écran sans
/// infobulle n'est lisible NULLE PART.
#[test]
fn le_message_d_etat_se_coupe_au_bord_de_la_frise() {
    const LARGEUR: f32 = 1600.0;
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(LARGEUR, 1000.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.debutant = true;
    // La note la plus longue de l'interface, celle de la capture.
    app.status = Status::Note(|s| s.binary_dynamic);
    let début = strings(app.lang)
        .binary_dynamic
        .chars()
        .take(30)
        .collect::<String>();

    let mut encre: Option<egui::Rect> = None;
    let mut coupé = false;
    for _ in 0..3 {
        encre = None;
        coupé = false;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && t.galley.text().starts_with(&début)
                // La bande du résultat redit la même note en entier,
                // et c'est sa place : elle a la largeur pour cela. On
                // ne garde que celle de la frise, tout en haut.
                && t.pos.y < 100.0
            {
                encre = Some(cadre_du_texte(t));
                coupé = t.galley.elided;
            }
        }
        sortie.textures_delta.clear();
    }

    let encre = encre.expect("la note doit se peindre dans la frise de l'Établi");
    assert!(
        encre.right() <= LARGEUR,
        "le message d'état déborde l'écran : il va jusqu'à x = {:.0} pour {LARGEUR:.0} de large",
        encre.right()
    );
    assert!(
        coupé,
        "une note de quatre lignes NE TIENT PAS sur la frise : elle doit être coupée \
         (et donc relisible au survol), et `galley.elided` dit qu'elle ne l'est pas"
    );
}

/// LA LÉGENDE DE LA PILE NE SE LIT PAS EN POUSSANT UNE BARRE.
///
/// Les mots de pile défilent de côté, et pour une bonne raison : une
/// valeur coupée au bord n'est pas une valeur abrégée, c'est une
/// valeur fausse. Mais la phrase qui explique le grisé — « au-dessus
/// de RSP : déjà dépilé » — était peinte DANS cette zone, en dernière
/// ligne. Elle n'y repliait pas, allongeait la course du défilement
/// pour son seul compte, et la capture du 10 novembre 2026 la montrait
/// coupée au milieu d'un mot, dans le volet de l'Établi.
///
/// On mesure donc qu'elle tient dans ce qu'on en voit : son encre et
/// son encre rognée par le découpage sont le même rectangle.
///
/// L'ÉMULATEUR EST POSÉ À LA MAIN, sans passer par nasm : ce qui est
/// mesuré ici est une mise en page, et elle ne doit pas dépendre de ce
/// qui est installé sur la machine. Si `Backend` change au point que
/// cette pose ne compile plus, c'est ce commentaire qui dira pourquoi
/// elle existait.
#[test]
fn la_legende_de_la_pile_tient_dans_le_volet() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;
    const TAILLE: usize = 0x4000;

    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1400.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.debutant = true;

    let mut mem = Memory::new(BASE, TAILLE);
    mem.load(BASE, &[0xf4]).expect("un hlt se charge");
    // UNE PILE LARGE, et c'est la moitié du contrôle.
    //
    // Une pile de zéros donne des lignes courtes, la zone qui défile
    // de côté n'a rien à faire défiler, et la légende se replie bien
    // à l'aise : le défaut ne paraît pas. Ce que montrait la capture,
    // c'est une pile d'environnement — un mot qui pointe sur une
    // chaîne, que namsdec suit et écrit en clair. La ligne devient
    // deux fois plus large que le volet, et c'est à CETTE largeur que
    // tout ce qui partage la zone se replie, légende comprise.
    const CHAÎNE: u64 = BASE + 0x1000;
    mem.load(CHAÎNE, b"HOME=/home/quelquun/avec/un/chemin/tres/long\0")
        .expect("la chaîne se charge");
    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = BASE;
    let rsp = BASE + 0x2000;
    cpu.regs.set64(Reg64::Rsp, rsp);
    for mot in 0..4 {
        cpu.mem
            .write_u64(rsp + mot * 8, CHAÎNE)
            .expect("le mot de pile s'écrit");
    }
    app.emulator = Some(Box::new(namsdec_emu::Emulator::new(cpu)));

    let s = strings(app.lang);
    let mut encre: Option<egui::Rect> = None;
    let mut vue: Option<egui::Rect> = None;
    for _ in 0..3 {
        encre = None;
        vue = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && t.galley
                    .text()
                    .starts_with(&s.stack_above_hint.chars().take(20).collect::<String>())
            {
                let r = cadre_du_texte(t);
                encre = Some(r);
                vue = Some(r.intersect(clipped.clip_rect));
            }
        }
        sortie.textures_delta.clear();
    }

    let encre = encre.expect("la légende du grisé se peint sous la pile");
    let vue = vue.expect("et elle a un découpage");
    assert!(
        vue.width() >= encre.width() - 0.5,
        "la légende de la pile est coupée : {:.0} px peints, {:.0} px visibles \
         — elle doit se replier dans le volet, non défiler de côté",
        encre.width(),
        vue.width()
    );
}

/// LE REPÈRE DE MESURE DES TEXTES, ÉPROUVÉ SUR UN TEXTE ALIGNÉ À DROITE.
///
/// `cadre_du_texte` existe pour une raison précise, et une raison qui
/// ne se voit pas : `TextShape::pos` n'est le coin haut-gauche que pour
/// une galley alignée à GAUCHE. Alignée à droite — tout ce qu'une
/// rangée `right_to_left` pose, et egui en met partout —, `pos` est le
/// bord DROIT.
///
/// Dix-sept relevés du banc mesuraient `from_min_size(pos, size)`. Ils
/// déplaçaient donc chaque étiquette alignée à droite d'une largeur de
/// texte VERS LA DROITE, c'est-à-dire vers le bord où un recouvrement
/// se produit : ils rataient les recouvrements de ce côté-là, et
/// pouvaient en inventer ailleurs. Le défaut s'est révélé en accusant
/// « RIP 0x400000 » d'être coupé dans la frise de l'Établi alors qu'il
/// ne l'était pas.
///
/// Ce contrôle prend ce même RIP — le seul texte aligné à droite dont
/// on connaisse la place exacte — et vérifie les deux moitiés : que le
/// repère le pose DANS l'écran, et que la forme naïve l'en sortait.
/// Sans la seconde moitié, le contrôle passerait aussi bien si egui
/// changeait d'avis et alignait tout à gauche.
#[test]
fn le_cadre_d_un_texte_suit_son_alignement() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;
    const LARGEUR: f32 = 1600.0;

    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(LARGEUR, 1000.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.debutant = true;
    let mut mem = Memory::new(BASE, 0x1000);
    mem.load(BASE, &[0xf4]).expect("un hlt se charge");
    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = BASE;
    app.emulator = Some(Box::new(namsdec_emu::Emulator::new(cpu)));

    let mut juste: Option<egui::Rect> = None;
    let mut naïf: Option<egui::Rect> = None;
    let mut aligné_a_droite = false;
    for _ in 0..3 {
        juste = None;
        naïf = None;
        aligné_a_droite = false;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && t.galley.text().starts_with("RIP ")
            {
                juste = Some(cadre_du_texte(t));
                naïf = Some(egui::Rect::from_min_size(t.pos, t.galley.size()));
                aligné_a_droite = t.galley.job.halign == egui::Align::Max;
            }
        }
        sortie.textures_delta.clear();
    }

    let juste = juste.expect("RIP se peint dans la frise de l'Établi");
    let naïf = naïf.expect("et l'on sait aussi ce que la forme naïve en aurait dit");
    assert!(
        aligné_a_droite,
        "RIP est posé par une rangée `right_to_left` : sans cela, ce contrôle ne \
         mesure plus ce pour quoi il existe"
    );
    assert!(
        juste.right() <= LARGEUR,
        "le repère doit poser RIP DANS l'écran : il va jusqu'à x = {:.0} pour \
         {LARGEUR:.0} de large",
        juste.right()
    );
    assert!(
        naïf.right() > LARGEUR,
        "et la forme naïve doit, elle, l'en sortir — sinon le repère ne corrige rien \
         et le contrôle ne regarde pas ce qu'il croit : naïf {naïf:?}, juste {juste:?}"
    );
}

/// LE NOM DU FICHIER S'ABRÈGE, ET GARDE SON EXTENSION.
///
/// Demandé à l'écran le 10 novembre 2026, une flèche verte sur le nom
/// de la barre du haut : « si le nom est trop long le concatener ou
/// premiers caractères puis … ». Un nom de fichier n'a pas de limite,
/// et celui-ci ouvre une rangée qui porte ensuite cinq commandes :
/// laissé entier, il les repousse hors de la barre.
///
/// PAR LE MILIEU, et c'est le point : couper par la droite perdrait
/// l'extension, qui dit de quoi le fichier est fait, et deux fichiers
/// d'un même projet ne se distinguent souvent que par leur début.
///
/// Le contrôle mesure les deux bouts — la fonction qui abrège, puis ce
/// qui est PEINT, avec le nom entier au survol. La première sans la
/// seconde laisserait un nom juste qu'on n'affiche pas.
#[test]
fn le_nom_du_fichier_s_abrege_dans_la_barre() {
    use crate::etabli::{NOM_MAX, nom_abrégé};

    // 1. LA RÈGLE, sur les trois cas qui comptent.
    assert_eq!(
        nom_abrégé("court.asm", NOM_MAX),
        "court.asm",
        "un nom qui tient ne se touche pas"
    );
    let long = "un_nom_de_travail_vraiment_tres_long.asm";
    let abrégé = nom_abrégé(long, NOM_MAX);
    assert!(
        abrégé.chars().count() <= NOM_MAX,
        "l'abrégé tient dans la limite : « {abrégé} »"
    );
    assert!(
        abrégé.ends_with(".asm"),
        "et garde son extension : « {abrégé} »"
    );
    assert!(
        long.starts_with(&abrégé[..abrégé.find('…').expect("le signe de coupure")]),
        "le début est celui du vrai nom : « {abrégé} »"
    );
    // Une « extension » qui n'en est pas une ne se garde pas : au-delà
    // de six caractères, c'est la fin du nom, et la garder reviendrait
    // à ne presque rien couper.
    let faux = "archive_de_travail.sauvegarde_du_jeudi";
    assert!(
        !nom_abrégé(faux, NOM_MAX).ends_with("jeudi"),
        "une fin de nom n'est pas une extension"
    );

    // 2. ET C'EST BIEN CE QUI EST PEINT, le nom entier au survol.
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.debutant = true;
    app.current_file = Some(std::path::PathBuf::from("/tmp").join(long));

    let mut vu = false;
    let mut entier = false;
    for _ in 0..3 {
        vu = false;
        entier = false;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let texte = t.galley.text();
                if texte.starts_with(&abrégé) {
                    vu = true;
                }
                if texte.contains(long) {
                    entier = true;
                }
            }
        }
        sortie.textures_delta.clear();
    }
    assert!(vu, "la barre du haut doit porter le nom abrégé");
    assert!(
        !entier,
        "et jamais le nom entier : c'est lui qui repoussait les commandes"
    );
}
