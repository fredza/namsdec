//! Les fenêtres internes : ouvrir, agrandir, refermer, retrouver.
//!
//! La boîte « À propos », les barres de dialogue, la pastille de
//! fermeture, l'ordre d'empilement, et la place où une fenêtre revient.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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

    app.apply_theme(&ctx);
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
        .memory(|m| m.area_rect(egui::Id::unique("flow_window")))
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
        .memory(|m| m.area_rect(egui::Id::unique("flow_window")))
        .expect("toujours ouverte");
    assert!(
        agrandie.width() > rect.width() && agrandie.height() > rect.height(),
        "{agrandie:?} n'est pas plus grande que {rect:?}"
    );
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
    assert_eq!(SITE_URL, "https://fredza.github.io/namsdec");
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
    let manifeste = include_str!("../../../../Cargo.toml");
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
        // SIX CENTS OCTETS, RAMENÉS SUR UNE FRONTIÈRE DE CARACTÈRE.
        // Sans `floor_char_boundary`, la borne tombait un jour au
        // milieu d'un « î » et le contrôle s'arrêtait sur un
        // découpage impossible — un défaut qui dormait là et ne se
        // réveillait qu'au premier changement de longueur du code
        // au-dessus. Il n'avait rien à dire des fenêtres.
        let suite = &tout[position..tout.floor_char_boundary(position + 600)];
        // Certaines fenêtres nomment leur identifiant par une constante,
        // parce que plusieurs endroits s'y réfèrent ; le reste l'écrit
        // sur place. On résout la constante en relisant sa déclaration,
        // plutôt qu'en tenant ici une liste qu'il faudrait allonger à
        // chaque fenêtre — et qu'on oublierait.
        let id = suite
            .find("egui::Id::unique(\"")
            .map(|i| &suite[i + "egui::Id::unique(\"".len()..])
            .and_then(|reste| reste.find('"').map(|fin| &reste[..fin]))
            .or_else(|| {
                let i = suite.find("egui::Id::unique(")? + "egui::Id::unique(".len();
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
                .position(|id| id == egui::LayerId::new(egui::Order::Middle, egui::Id::unique(nom)))
                .unwrap_or_else(|| panic!("la fenêtre « {nom} » est dessinée"))
        })
    };
    assert!(
        rang("calculator_window") > rang("preferences_window"),
        "la dernière ouverte est au-dessus"
    );
}

/// Les fenêtres internes se ferment par la même pastille que la grande.
///
/// egui pose sa croix à lui : deux traits maigres, à droite, de la
/// couleur du texte. Dans une application dont la fenêtre principale se
/// ferme par un rond rouge posé à gauche, cela faisait deux grammaires
/// pour le même geste — et celle qu'on voit le moins fermait le plus de
/// choses. Ce test dit qu'il n'en reste qu'une, et qu'elle est bien la
/// même.
///
/// Une fenêtre peut en porter une SECONDE — la verte, qui agrandit —,
/// et elle est alors à droite de la rouge, comme sur la barre de titre.
/// Ce qui est interdit, c'est une fermeture qui ne serait pas la
/// première pastille, ou qui ne serait pas rouge.
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
        let sous_la_barre = |app: &mut NamsdecApp| -> Vec<(f32, [i32; 3])> {
            let mut vues = Vec::new();
            for _ in 0..3 {
                vues.clear();
                let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
                for clipped in &output.shapes {
                    if let egui::epaint::Shape::Circle(cercle) = &clipped.shape
                        && cercle.center.y > 40.0
                        && (cercle.radius - 6.5).abs() < 0.01
                    {
                        vues.push((cercle.center.x, teinte(cercle.fill)));
                    }
                }
                output.textures_delta.clear();
            }
            // De gauche à droite : c'est l'ordre qui porte le sens.
            vues.sort_by(|a, b| a.0.total_cmp(&b.0));
            vues
        };

        assert!(
            sous_la_barre(&mut app).is_empty(),
            "{nom} : fermée, rien ne traîne"
        );

        ouvre(&mut app);
        let vues = sous_la_barre(&mut app);
        assert!(
            !vues.is_empty(),
            "{nom} : il faut une pastille de fermeture"
        );
        // Une seule, sauf là où une seconde est voulue — et nommée ici,
        // pour qu'une pastille qui apparaîtrait ailleurs sans qu'on
        // l'ait demandée fasse échouer ce contrôle.
        let attendues = if nom == "analyse" { 2 } else { 1 };
        assert_eq!(
            vues.len(),
            attendues,
            "{nom} : {attendues} pastille(s) attendue(s) : {vues:?}"
        );
        // Le fondu laisse quelques unités d'écart après division : on
        // juge la teinte, pas l'arrondi.
        let écart = vues[0]
            .1
            .iter()
            .zip(attendue.iter())
            .map(|(a, b)| (a - b).abs())
            .max()
            .unwrap();
        assert!(
            écart <= 8,
            "{nom} : la PREMIÈRE pastille doit être celle de la fermeture, {:?} contre \
             {attendue:?}",
            vues[0].1
        );
        // La seconde, quand il y en a une, est celle qui agrandit : une
        // fenêtre qui porterait deux rouges en serait illisible.
        if let Some((_, teinte_zoom)) = vues.get(1) {
            let verte = teinte(chrome::WindowButton::Maximize.colour());
            let écart = teinte_zoom
                .iter()
                .zip(verte.iter())
                .map(|(a, b)| (a - b).abs())
                .max()
                .unwrap();
            assert!(
                écart <= 8,
                "{nom} : la seconde pastille doit être celle qui agrandit, {teinte_zoom:?} \
                 contre {verte:?}"
            );
        }
    }
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
    // Le style de l'application, et pas seulement ses couleurs : les
    // espacements et les polices d'egui par défaut ne sont pas les
    // siens, et l'on mesurerait une largeur qui n'existe nulle part.
    NamsdecApp::default().apply_theme(&ctx);
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

/// Toute fenêtre qui s'ouvre porte un bouton pour la fermer.
///
/// La croix de la barre de titre ne suffit pas : elle est petite, elle
/// est loin quand la fenêtre est haute, et il lui est arrivé de sortir
/// de l'écran. Un bouton dans le corps de la fenêtre se trouve toujours,
/// et il se trouve au même endroit partout.
#[test]
fn every_window_carries_a_way_out() {
    let Some(octets) = binaire_fabriqué() else {
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
        assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
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
/// introuvable — on la cherchait dans le menu Outils en la croyant
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
        let Some(état) = egui::AreaState::load(&ctx, egui::Id::unique(nom)) else {
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

/// Une fenêtre qui revient n'est jamais enterrée.
///
/// Le contrôle voisin — deux fenêtres ne se posent pas au même endroit —
/// passait, et pourtant la capture d'une séance rouverte montrait trois
/// fenêtres empilées dont la première était illisible. Il mesurait la
/// distance entre CENTRES, et huit pixels lui suffisaient : un éventail
/// qui tenait tout entier dans deux cent soixante pixels de diagonale le
/// satisfaisait pendant que la moitié des fenêtres disparaissait sous
/// l'autre. Mesuré sur l'écran de cette machine, 3840 × 2460 : dix-huit
/// paires se recouvraient à cent pour cent.
///
/// Ce qu'on vérifie ici est ce qui décide vraiment du sort d'une
/// fenêtre : sa BARRE DE TITRE reste-t-elle saisissable ? Une fenêtre
/// dont le titre est entièrement couvert par celles du dessus est
/// introuvable — on la cherche dans le menu Outils en la croyant
/// fermée, et l'y rappeler ne fait que la reposer au même endroit.
///
/// Trois précautions, et chacune vient d'un faux résultat :
///
/// * **L'ordre de peinture, et non celui de la table.** Une fenêtre
///   n'est cachée que par ce qui se peint APRÈS elle. `layer_ids()` le
///   donne, du fond vers le dessus.
/// * **Un identifiant revient sur plusieurs couches.** Sans
///   dédoublonner, chaque fenêtre se trouve couverte par elle-même, et
///   le relevé annonce douze fenêtres enterrées sur douze.
/// * **Pas en dessous de 1920 × 1200.** À 1600 × 1100, la fenêtre
///   d'analyse mesure 1100 × 1000 et celle des exercices 920 × 720 :
///   elles couvrent à elles deux la presque totalité de l'écran, et
///   aucun placement ne peut y sauver douze fenêtres. Ce qui se corrige
///   là est la taille de ces deux fenêtres, pas l'éventail — et c'est un
///   autre contrôle.
///
/// Éprouvé en remettant l'ancienne formule — un pas unique sur une
/// diagonale, écrêté à vingt-quatre pixels : le test échoue alors en
/// nommant les fenêtres enterrées.
#[test]
fn no_window_that_comes_back_is_buried_under_the_others() {
    // Chaque écran avec ce qu'on peut lui garantir. Sur les deux grands,
    // AUCUNE fenêtre enterrée ; en dessous, au plus une par demi-douzaine
    // — car la fenêtre d'analyse mesure à elle seule 1100 × 1000 et celle
    // des exercices 920 × 720 : à 1280 × 800, elles couvrent presque tout,
    // et ce qui se corrige alors est LEUR taille, non le placement.
    for (larg, haut, toléré) in [
        (1280.0_f32, 800.0_f32, 3_usize),
        (1920.0, 1200.0, 2),
        (2560.0, 1440.0, 0),
        (3840.0, 2460.0, 0),
    ] {
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
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(larg, haut),
            )),
            ..Default::default()
        };
        // Vingt images : egui AMÈNE une fenêtre à sa taille au lieu de
        // l'y poser, et une fenêtre dont la hauteur épouse son contenu
        // met plusieurs images à se fixer.
        for _ in 0..20 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            sortie.textures_delta.clear();
        }

        // Les fenêtres dans l'ordre où elles se peignent, le fond
        // d'abord — et une seule fois chacune.
        let mut ordre: Vec<egui::Id> = Vec::new();
        for id in ctx.memory(|m| m.layer_ids().map(|couche| couche.id).collect::<Vec<_>>()) {
            ordre.retain(|déjà| *déjà != id);
            ordre.push(id);
        }
        let mut posées: Vec<(&str, egui::Rect)> = Vec::new();
        for id in &ordre {
            for (nom, ouverte) in app.window_states() {
                if ouverte
                    && egui::Id::unique(nom) == *id
                    && let Some(état) = egui::AreaState::load(&ctx, *id)
                {
                    posées.push((nom, état.rect()));
                }
            }
        }
        assert!(
            posées.len() >= 10,
            "il faut plusieurs fenêtres pour que le test prouve quelque chose — \
             {} seulement",
            posées.len()
        );

        // La hauteur de la poignée : la bande du haut, celle qu'on
        // saisit pour déplacer une fenêtre.
        const POIGNÉE: f32 = 28.0;
        // Ce qu'il faut en voir pour pouvoir la saisir sans tâtonner.
        // Trente-deux points, et non les quarante-huit des cibles
        // tactiles : on saisit une barre de titre à la souris, et un
        // carré de trente-deux se vise sans peine. Le seuil sépare
        // bien les deux cas — l'ancienne formule laissait ZÉRO.
        const ATTRAPABLE: f32 = 32.0;
        let mut enterrées = Vec::new();
        for (i, (nom, cadre)) in posées.iter().enumerate() {
            let titre =
                egui::Rect::from_min_max(cadre.min, egui::pos2(cadre.max.x, cadre.min.y + POIGNÉE));
            // La plus large tranche de titre qu'aucune fenêtre du dessus
            // ne mord. On retranche les intervalles couverts et l'on
            // garde le plus long morceau — et non, comme au premier
            // essai, en reculant seulement les deux bords : cette
            // mesure-là ignore les trous du milieu, et déclarait
            // introuvable une fenêtre dont la moitié centrale du titre
            // était parfaitement visible.
            let mut couverts: Vec<(f32, f32)> = posées[i + 1..]
                .iter()
                .filter(|(_, dessus)| {
                    dessus.min.y <= titre.min.y + 4.0 && dessus.max.y >= titre.min.y
                })
                .map(|(_, dessus)| (dessus.min.x.max(titre.min.x), dessus.max.x.min(titre.max.x)))
                .filter(|(a, b)| b > a)
                .collect();
            couverts.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut libre: f32 = 0.0;
            let mut bord = titre.min.x;
            for (début, fin) in couverts {
                libre = libre.max(début - bord);
                bord = bord.max(fin);
            }
            libre = libre.max(titre.max.x - bord);
            if libre < ATTRAPABLE {
                enterrées.push(format!("{nom} ({libre:.0} px de poignée)"));
            }
        }
        assert!(
            enterrées.len() <= toléré,
            "à {larg}×{haut}, {} fenêtres rouvertes sont introuvables au lieu \
             des {toléré} tolérées : {enterrées:?}",
            enterrées.len()
        );
    }
}

/// Rien ne s'ouvre par-dessus la barre d'état.
///
/// C'est la seule bande qui dise en permanence où l'on en est : le
/// mode, la ligne, la colonne, l'état de l'assemblage. La fiche
/// d'instruction s'ancrait à vingt-quatre pixels du bas — la barre en
/// fait vingt-huit —, si bien qu'elle couvrait « Lig 1, Col 1 » et
/// l'indicateur de nasm au moment précis où l'on regarde une
/// instruction.
///
/// Vu sur une capture, et nulle part ailleurs : aucune mesure ne
/// demandait « cette fenêtre laisse-t-elle la barre d'état lisible ? ».
///
/// La règle vaut pour ce qui se POSE tout seul : une fenêtre qu'on
/// traîne va où l'on veut. Elle couvre donc les deux surfaces qui
/// choisissent leur place en bas — la fiche et le panneau des suites.
#[test]
fn rien_ne_s_ouvre_par_dessus_la_barre_d_etat() {
    for (nom, avec_console, poser) in [
        (
            "fiche d'instruction",
            false,
            (|app: &mut NamsdecApp| app.show_help = true) as fn(&mut NamsdecApp),
        ),
        ("fiche d'instruction / console ouverte", true, |app| {
            app.show_help = true
        }),
        ("suites d'une séquence", false, |app| {
            app.dispatcher.arm(&[egui::Key::Space]);
            app.leader_since = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
        }),
    ] {
        let id = if nom.starts_with("suites") {
            "leader_hints"
        } else {
            "instruction_help"
        };
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            show_console: avec_console,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        poser(&mut app);

        let hauteur = 900.0;
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, hauteur),
            )),
            ..Default::default()
        };
        // Plusieurs images : une fenêtre est AMENÉE à sa place, et les
        // panneaux du bas ne connaissent leur hauteur qu'une fois posés.
        let mut cadre = None;
        let mut barres = 0.0f32;
        for _ in 0..6 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            sortie.textures_delta.clear();
            cadre = egui::AreaState::load(&ctx, egui::Id::unique(id)).map(|a| a.rect());
            barres = app.hauteur_des_barres_du_bas(&ctx);
        }
        let cadre = cadre.unwrap_or_else(|| panic!("« {nom} » doit être affichée"));
        assert!(
            barres > 10.0,
            "« {nom} » : la barre d'état doit avoir une hauteur — {barres}"
        );
        let haut_des_barres = hauteur - barres;
        assert!(
            cadre.bottom() <= haut_des_barres + 0.5,
            "« {nom} » descend à {:.2} px ; les barres du bas commencent à \
             {haut_des_barres:.2}",
            cadre.bottom()
        );
    }
}
