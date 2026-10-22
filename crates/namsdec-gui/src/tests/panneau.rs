//! Le panneau latéral : sa largeur, ses onglets, son partage.
//!
//! Ce qui vaut pour le panneau lui-même, quel que soit l'onglet qu'il
//! montre.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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

/// Les onglets du panneau restent atteignables, quoi qu'on déplie.
///
/// Le bandeau, les registres vectoriels et la pile x87 étaient bornés
/// chacun de son côté : chacun tenait dans sa moitié de panneau, mais
/// leurs hauteurs s'ajoutaient et la rangée d'onglets passait sous le
/// bord — sans barre de défilement pour aller la rechercher.
#[test]
fn nothing_unfolded_can_push_the_tabs_out_of_the_panel() {
    let mut app = NamsdecApp {
        // La vue du processeur, et un programme chargé pour que chaque
        // section ait quelque chose à montrer.
        active_tab: SidebarTab::Cpu,
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
        active_tab: SidebarTab::Cpu,
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

/// La part de l'onglet se fige à la poignée — VERS LE HAUT —, et se
/// rend au partage automatique.
///
/// Une poignée tirée une fois qui ne se reprendrait plus, c'est un
/// réglage perdu pour de bon : la case d'Affichage doit rendre au
/// panneau le partage qu'il calcule lui-même, comme pour la console.
///
/// LA RÈGLE A CHANGÉ DE SENS le 20 octobre 2026, et ce contrôle avec
/// elle. Il exigeait que la hauteur figée soit gardée « même quand le
/// bandeau se replie et libère de la place ». C'était précisément le
/// défaut : sur un panneau de 2 460 px, une poignée tirée trois
/// semaines plus tôt — et enregistrée dans les réglages — laissait
/// deux mille pixels de vide entre un bandeau replié et un onglet
/// Mémoire écrasé. « Encore l'onglet mémoire — ILLISIBLE ».
///
/// La hauteur retenue est donc un PLANCHER : on peut agrandir l'onglet
/// au détriment du bandeau, qui défile ; on ne peut plus le rétrécir
/// pour laisser du vide, ce qui n'est pas un réglage.
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
        // Le bandeau DÉPLIÉ, sans quoi il ne demande presque rien et le
        // partage donne déjà tout à l'onglet : il n'y aurait plus de
        // place pour éprouver le gel vers le haut.
        next_instruction_open: true,
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let partagée = rendre(&mut app, &ctx);

    // Figée PLUS PETIT, et le bandeau se replie : la place libérée
    // revient à l'onglet. C'est le défaut corrigé — ce qui ne doit pas
    // tenir, c'est un vide entre les deux.
    app.sidebar_tab_height = Some(SIDEBAR_TAB_MIN);
    app.next_instruction_open = false;
    let repliée = rendre(&mut app, &ctx);
    assert!(
        repliée > partagée + 10.0,
        "le bandeau replié rend sa place à l'onglet au lieu de la garder vide : \
         {repliée} contre {partagée}"
    );

    // Figée PLUS GRAND que ce que le partage donnerait, bandeau déplié :
    // c'est le geste qu'on fait pour un vidage mémoire long, et le
    // bandeau cède — il défile.
    app.next_instruction_open = true;
    let demandée = partagée + 40.0;
    app.sidebar_tab_height = Some(demandée);
    let figée = rendre(&mut app, &ctx);
    assert!(
        figée > partagée + 20.0,
        "une part figée plus grande que le partage est tenue : {figée} contre \
         {partagée} (demandé {demandée})"
    );

    // Rendue au partage : elle suit ce que le bandeau laisse.
    app.sidebar_tab_height = None;
    app.next_instruction_open = false;
    let rendue = rendre(&mut app, &ctx);
    assert!(
        rendue > partagée,
        "l'état replié rend sa place à l'onglet : {rendue} contre {partagée}"
    );
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
    let binaire = binaire_fabriqué();
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
                assert!(app.load_elf(binaire.as_ref().unwrap(), NOM_FABRIQUÉ));
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

/// Le panneau latéral ne se recouvre jamais, même à l'étroit.
///
/// Le défaut, sur capture de Fred : dans l'onglet Mémoire, l'état du
/// processeur s'écrivait par-dessus le haut de l'onglet — « Adresse »,
/// « Chercher », « Suivre » et « Lire en » sous les registres, illisible.
///
/// La cause est contre-intuitive, et c'est la mesure qui l'a donnée.
/// Quand le contenu d'un panneau egui dépasse la place qu'on lui a
/// laissée, egui ramène la TAILLE du rectangle au maximum demandé en
/// gardant le bord du CONTENU, non celui de la place disponible : le
/// rectangle glisse vers le bas, et le panneau ne rend au parent que ce
/// qui reste visible. Relevé ici : le panneau peignait de 361 à 539 et
/// n'en rendait que de 403 à 539 — l'état du processeur, placé sur ce
/// que le parent avait rendu, s'écrivait sur les quarante-deux pixels
/// de différence.
///
/// Les conditions comptent, et aucune n'est exotique : un panneau court
/// (console ouverte sur une fenêtre ordinaire), une part d'onglet
/// agrandie à la main du temps où le panneau était haut, et un onglet
/// dont les commandes prennent de la place. Le contrôle les réunit, et
/// passe les cinq onglets.
#[test]
fn le_panneau_lateral_ne_se_recouvre_jamais_meme_a_l_etroit() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    for onglet in SidebarTab::ALL {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        if !app.load_elf(&octets, NOM_FABRIQUÉ) {
            return;
        }
        app.show_sidebar = true;
        app.show_re_tab = true;
        app.active_tab = onglet;
        // La poignée a été tirée quand le panneau était haut : la part
        // gardée pour l'onglet ne tient plus dans ce qui reste.
        app.sidebar_tab_height = Some(543.0);
        let état = egui::containers::panel::PanelState {
            outer_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(380.0, 543.0)),
        };
        ctx.data_mut(|d| d.insert_persisted(egui::Id::new("sidebar_tab"), état));
        // Un panneau court : une fenêtre ordinaire avec la console
        // ouverte.
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 820.0),
            )),
            ..Default::default()
        };
        let mut lignes: Vec<(String, egui::Rect)> = Vec::new();
        for _ in 0..4 {
            lignes.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    for ligne in &t.galley.rows {
                        if ligne.glyphs.is_empty() {
                            continue;
                        }
                        // Ce qui se voit, et non ce qui est peint : une
                        // ligne sortie d'une zone défilante est bien
                        // dans la liste des formes, mais rognée à néant.
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
        assert!(
            lignes.len() > 20,
            "onglet {} : le panneau n'écrit presque rien, le contrôle ne prouverait rien",
            onglet.id()
        );
        for (i, (texte_a, a)) in lignes.iter().enumerate() {
            for (texte_b, b) in &lignes[i + 1..] {
                // Deux pixels de tolérance : les jambages d'une ligne
                // frôlent les hampes de la suivante sans se gêner.
                assert!(
                    !a.shrink(2.0).intersects(b.shrink(2.0)),
                    "onglet {} : « {texte_a} » et « {texte_b} » se recouvrent\n  {a:?}\n  {b:?}",
                    onglet.id()
                );
            }
        }
    }
}
