//! L'état du processeur : registres, indicateurs, bandeau.
//!
//! Le bandeau et son onglet, la grille des registres, les sept
//! indicateurs, les registres vectoriels, et la prochaine instruction
//! résolue avant qu'elle ne s'exécute.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

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

/// L'état du processeur est un ONGLET, et la vue choisie a tout le
/// panneau.
///
/// Les registres avaient un bandeau ancré entre le haut du panneau et
/// la vue, au nom d'un principe juste — ce qu'on surveille entre deux
/// pas ne doit pas se chercher. Le prix était un partage à trois qui ne
/// tombait jamais juste : trois correctifs successifs pour la même
/// famille de recouvrements, et au zoom de travail de Fred, près de
/// soixante pour cent du panneau pris aux autres vues.
///
/// Ce contrôle tient les deux bouts du changement : le processeur est
/// la vue par défaut et montre bien ses registres ; et dès qu'on en
/// choisit une autre, ils ne sont plus peints — c'est ce qui rend toute
/// la place à celle qu'on regarde.
#[test]
fn l_etat_du_processeur_est_un_onglet_et_la_vue_choisie_a_tout_le_panneau() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    assert_eq!(
        NamsdecApp::default().active_tab,
        SidebarTab::Cpu,
        "le processeur est la vue qu'on trouve en ouvrant"
    );

    let peint = |onglet: SidebarTab| -> (Vec<String>, f32) {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp::default();
        app.apply_theme(&ctx);
        assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
        app.show_sidebar = true;
        app.active_tab = onglet;
        let entrée = écran_du_banc();
        let mut textes: Vec<String> = Vec::new();
        // Quelques images : le partage entre le bandeau et la vue se
        // cale en deux ou trois.
        for _ in 0..5 {
            textes.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    textes.push(t.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        let part = egui::containers::panel::PanelState::load(&ctx, egui::Id::new("sidebar_tab"))
            .expect("la vue a sa part du panneau")
            .outer_rect
            .height();
        (textes, part)
    };

    let (processeur, part_cpu) = peint(SidebarTab::Cpu);
    for registre in ["RAX", "R15", "RIP"] {
        assert!(
            processeur.iter().any(|t| t == registre),
            "la vue du processeur montre {registre} : {processeur:?}"
        );
    }

    let (mémoire, part_mémoire) = peint(SidebarTab::Memory);
    for registre in ["RAX", "R15"] {
        assert!(
            !mémoire.iter().any(|t| t == registre),
            "la vue Mémoire ne porte plus les registres — c'est ce qui lui rend la place"
        );
    }
    // Et la place rendue est réelle : la vue Mémoire en reçoit au moins
    // autant que la vue du processeur, là où elle en recevait le reste.
    assert!(
        part_mémoire >= part_cpu - 1.0,
        "la vue Mémoire a au moins la part du processeur : {part_mémoire} contre {part_cpu}"
    );
    assert!(
        part_mémoire >= SIDEBAR_TAB_MIN,
        "et de quoi montrer quelque chose : {part_mémoire}"
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

/// LE BANDEAU NE GARDE PAS DE VIDE au-dessus de l'onglet.
///
/// C'est le grief « encore l'onglet mémoire — ILLISIBLE », et il se
/// mesure. Sur un panneau haut, « Prochaine instruction » replié
/// demande une cinquantaine de pixels ; l'onglet en recevait 307, et
/// les deux mille du milieu étaient du vide. La cause : la hauteur
/// retenue d'un geste de poignée — enregistrée dans les réglages, donc
/// gardée d'une séance à l'autre — servait de PLAFOND à l'onglet au
/// lieu de servir de plancher, et le bandeau empochait le reste.
///
/// Le contrôle part exactement de cet état : une hauteur retenue
/// petite, un bandeau replié, un panneau haut.
#[test]
fn the_banner_keeps_no_empty_space_above_the_tab() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Memory,
        next_instruction_open: false,
        // Ce qu'une poignée tirée il y a trois semaines avait laissé.
        sidebar_tab_height: Some(307.0),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1800.0),
        )),
        ..Default::default()
    };
    // Plusieurs images : le partage se mesure sur ce que le bandeau a
    // demandé à l'image d'avant, et met donc un tour à se poser.
    let mut bandeau: Option<f32> = None;
    let mut titres: Vec<f32> = Vec::new();
    for _ in 0..6 {
        bandeau = None;
        titres.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                let vu = rect.intersect(clipped.clip_rect);
                if !vu.is_positive() {
                    continue;
                }
                match t.galley.text().trim() {
                    "Prochaine instruction" => bandeau = Some(vu.bottom()),
                    // « Mémoire » paraît DEUX fois : l'onglet de la
                    // barre d'outils, tout en haut, et le titre de la
                    // vue, sous le bandeau. C'est le second qu'on
                    // mesure — le premier avait fait passer ce contrôle
                    // pour rien, avec un « vide » négatif.
                    "Mémoire" => titres.push(vu.top()),
                    _ => {}
                }
            }
        }
        output.textures_delta.clear();
    }
    let Some(bandeau) = bandeau else {
        panic!("le relevé n'a pas trouvé le bandeau — il ne mesure rien");
    };
    let Some(titre) = titres
        .iter()
        .copied()
        .filter(|haut| *haut > bandeau)
        .min_by(f32::total_cmp)
    else {
        panic!(
            "le relevé n'a pas trouvé le titre de la vue sous le bandeau ({bandeau}) — \
             il ne mesure rien ; vus : {titres:?}"
        );
    };
    let vide = titre - bandeau;
    assert!(
        vide < 90.0,
        "le bandeau replié garde {vide:.0} px de vide avant le titre de l'onglet : \
         la place qu'il n'emploie pas revient à l'onglet"
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

/// Un registre se pose à la main : « et si RCX valait 1 ? » se répond en
/// une seconde au lieu de se réécrire.
///
/// C'est le `set $rax = …` de gdb. Le pas suivant part de là, et la valeur
/// posée entre dans l'histoire du programme : revenir en arrière restaure
/// ce qui était vrai avant chaque pas, la main comprise. Les indicateurs,
/// eux, ne se posent pas à la main — ce sont de simples témoins de ce que
/// la dernière instruction a produit.
#[test]
fn a_register_is_set_by_hand() {
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

    // Le `jz` lit ce que le `dec` a posé : RCX a atteint 0, ZF est levé,
    // le saut est pris. On arrive sur `fini:`.
    app.do_step(); // jz : ZF posé, le saut est pris
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
        texte.starts_with("hlt"),
        "le saut devait être pris, on attend `hlt` : « {texte} »"
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

/// Le battement d'orange décroît avec le temps, pas avec les frames.
#[test]
fn the_flash_fades_over_real_time() {
    use std::time::{Duration, Instant};
    // Rien ne clignote tant que rien n'a tourné.
    assert_eq!(crate::sidebar::flash_fraction(None), 0.0);
    // Juste après un changement : presque plein.
    assert!(crate::sidebar::flash_fraction(Some(Instant::now())) > 0.9);
    // Passé le battement : retombé à zéro.
    let vieux = Instant::now() - Duration::from_secs_f32(crate::sidebar::FLASH_SECONDS + 0.1);
    assert_eq!(crate::sidebar::flash_fraction(Some(vieux)), 0.0);
}

/// Tous les indicateurs s'affichent, DF compris.
///
/// DF manquait : « std »/« cld » le posent et les instructions de chaîne
/// le lisent, mais le bandeau n'en montrait que six. « Tout doit
/// s'afficher. »
#[test]
fn every_flag_is_shown_including_df() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    std\n    cld\n    hlt\n".to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();
    let peint = paint(&mut app);
    for nom in ["CF", "PF", "AF", "ZF", "SF", "DF", "OF"] {
        assert!(
            peint.split('|').any(|mot| mot.starts_with(nom)),
            "l'indicateur {nom} doit s'afficher :\n{peint}"
        );
    }
}

/// Les indicateurs vivent dans la barre de titre et se présentent en entier.
///
/// Ils flottaient sous les registres, et le survol d'une pastille ne
/// donnait que « cliquez pour basculer ». Ils sont maintenant dans la
/// barre de titre — toujours visible —, et chaque survol développe le nom,
/// le rôle et l'état : les sept se lisent, au lieu de seulement se voir.
#[test]
fn the_flags_carry_a_title_and_describe_each_state() {
    // Le descriptif, sans écran : nom développé, rôle, état posé/retombé.
    let zf_on = crate::sidebar::flag_description("ZF", true, Lang::Fr);
    assert!(zf_on.contains("zéro") && zf_on.contains("posé"), "{zf_on}");
    let zf_off = crate::sidebar::flag_description("ZF", false, Lang::Fr);
    assert!(zf_off.contains("retombé"), "{zf_off}");
    let cf_en = crate::sidebar::flag_description("CF", true, Lang::En);
    assert!(cf_en.contains("carry") && cf_en.contains("set"), "{cf_en}");
    // Les sept ont un nom développé — aucun ne reste muet.
    for nom in ["CF", "PF", "AF", "ZF", "SF", "DF", "OF"] {
        let d = crate::sidebar::flag_description(nom, false, Lang::Fr);
        assert!(
            d.lines().count() >= 2 && d.starts_with(nom),
            "{nom} doit se décrire en entier : {d}"
        );
    }

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    mov rax, 1\n    hlt\n".to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();
    let peint = paint(&mut app);
    // Les sept pastilles se lisent dans la barre de titre.
    for nom in ["CF", "PF", "AF", "ZF", "SF", "DF", "OF"] {
        assert!(
            peint.split('|').any(|mot| mot.starts_with(nom)),
            "l'indicateur {nom} doit s'afficher dans la barre de titre :\n{peint}"
        );
    }
}

/// Une valeur qui vient de changer clignote en orange.
///
/// Un registre ou un indicateur qui bascule sans bouger à l'écran se
/// rate ; un bref éclat orange le désigne. On le vérifie juste après le
/// pas, quand l'éclat est au plus fort : l'indicateur changé tire vers
/// l'orange du thème, un autre non.
#[test]
fn a_changed_value_flashes_orange() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    mov rcx, 1\n    dec rcx\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step(); // mov rcx, 1
    app.do_step(); // dec rcx -> 0, ZF passe de 0 à 1 : il doit clignoter

    let theme = app.theme;
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    // Couleur de chaque galley, par son texte.
    let mut couleurs: std::collections::HashMap<String, egui::Color32> = Default::default();
    for _ in 0..2 {
        couleurs.clear();
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let c = t
                    .galley
                    .job
                    .sections
                    .first()
                    .map(|s| s.format.color)
                    .filter(|c| *c != egui::Color32::PLACEHOLDER)
                    .unwrap_or(t.fallback_color);
                couleurs.insert(t.galley.text().to_owned(), c);
            }
        }
        sortie.textures_delta.clear();
    }

    let dist = |a: egui::Color32, b: egui::Color32| {
        let d = |x: u8, y: u8| (i32::from(x) - i32::from(y)).pow(2);
        d(a.r(), b.r()) + d(a.g(), b.g()) + d(a.b(), b.b())
    };
    // ZF vient de passer à 1 : sa pastille tire vers l'orange du thème,
    // plus que vers le vert d'un indicateur levé au repos.
    let zf = couleurs.get("ZF 1").copied().unwrap_or_else(|| {
        panic!(
            "la pastille « ZF 1 » doit être peinte : {:?}",
            couleurs.keys()
        )
    });
    assert!(
        dist(zf, theme.changed) < dist(zf, theme.good),
        "ZF vient de changer : il doit clignoter en orange, pas rester vert"
    );
    // CF n'a pas changé : il ne clignote pas (il reste grisé, loin de
    // l'orange).
    if let Some(cf) = couleurs.get("CF 0").copied() {
        assert!(
            dist(cf, theme.changed) > dist(cf, theme.dim),
            "CF n'a pas changé : il ne doit pas clignoter"
        );
    }
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
    // De la taille par défaut — celle où le défaut se voyait — à un
    // grand écran.
    //
    // La fenêtre minimale (800×520) n'y est plus, et c'est un choix
    // qu'il faut dire : l'état du processeur est une VUE depuis le
    // 13 octobre 2026, et non plus une bande ancrée qui prenait sa
    // hauteur aux autres. Sur un panneau de trois cents pixels, seize
    // registres ne tiennent pas, et la vue défile — comme la pile, la
    // mémoire et les octets. Ce qu'on refuse n'a pas changé : qu'elle
    // soit coupée là où elle a la place de tenir.
    for (largeur, hauteur) in [(1280.0_f32, 820.0_f32), (1600.0, 1000.0), (1920.0, 1230.0)] {
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

    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.next_instruction_open, "dépliée au départ");
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
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

/// Les registres passent sur une colonne de plus quand le panneau est
/// court — et seulement quand cela sert.
///
/// Demandé par Fred, après avoir vu l'état du processeur manger les
/// trois cinquièmes du panneau : « les registres sur trois colonnes
/// quand le panneau est court ».
///
/// Ce qui bloque une colonne de plus n'est pas la largeur des valeurs —
/// elles font seize chiffres, c'est irréductible — mais le BLANC entre
/// les colonnes : quatre espacements pour que « …003c RBX » ne se lise
/// pas comme un seul mot. Sur un panneau court, ce confort coûte une
/// colonne, c'est-à-dire deux rangées. On le rend donc, mais seulement
/// là où il le faut.
///
/// Les trois cas, et le troisième est le plus important : serrer pour
/// rien serait aussi un défaut.
#[test]
fn les_registres_gagnent_une_colonne_quand_le_panneau_est_court() {
    use crate::sidebar::register_layout;

    // Une paire mesure 157 px, l'espacement 8 : mesuré dans le thème du
    // banc, et les nombres de ce contrôle en dépendent.
    let (pair, spacing, ligne) = (157.0, 8.0, 21.0);
    // 491 px de large : deux colonnes au blanc confortable (il en
    // faudrait 535), trois au blanc serré (il en faut 487).
    let large = 491.0;

    // Haut : on respire, deux colonnes, blanc confortable.
    let (colonnes, gap) = register_layout(large, pair, spacing, 1000.0, ligne, 16);
    assert_eq!(colonnes, 2, "au large, le confort passe avant");
    assert_eq!(gap, spacing * 2.0);

    // Court : trois colonnes, blanc serré. Onze rangées à vingt et un
    // pixels ne tiennent pas dans deux cents.
    let (colonnes, gap) = register_layout(large, pair, spacing, 200.0, ligne, 16);
    assert_eq!(
        colonnes, 3,
        "sur un panneau court, une colonne de plus vaut mieux qu'un blanc"
    );
    assert!(gap < spacing * 2.0, "et le blanc se resserre : {gap}");
    // Et le compte y est : six rangées plus trois lignes tiennent dans
    // deux cents pixels, là où onze n'y tenaient pas.
    assert!((16usize.div_ceil(colonnes) + 3) as f32 * ligne <= 200.0);

    // Court mais étroit : serrer ne gagnerait rien, donc on ne serre
    // pas. Un blanc rétréci sans colonne gagnée est une perte sèche.
    let étroit = 360.0;
    let (colonnes, gap) = register_layout(étroit, pair, spacing, 200.0, ligne, 16);
    assert_eq!(colonnes, 2, "la largeur ne permet pas davantage");
    assert_eq!(gap, spacing * 2.0, "donc le blanc reste celui du confort");
}

/// Et à l'écran, les registres se rangent vraiment sur une colonne de
/// plus quand le panneau raccourcit.
///
/// Le contrôle d'à côté éprouve la règle ; celui-ci éprouve qu'elle est
/// branchée. On compte les abscisses distinctes des noms de registres :
/// c'est le nombre de colonnes, mesuré sur ce qui est peint.
#[test]
fn a_l_ecran_les_registres_se_rangent_sur_une_colonne_de_plus() {
    let colonnes_à = |hauteur: f32| -> usize {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            source: DEFAULT_SOURCE.to_owned(),
            show_sidebar: true,
            active_tab: SidebarTab::Cpu,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        app.do_assemble();
        // Un panneau élargi à la main : à la largeur par défaut, deux
        // colonnes sont tout ce que la place permet, serré ou non.
        let état = egui::containers::panel::PanelState {
            outer_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(545.0, hauteur)),
        };
        ctx.data_mut(|d| d.insert_persisted(egui::Id::new("sidebar"), état));
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(2304.0, hauteur),
            )),
            ..Default::default()
        };
        let mut abscisses: std::collections::BTreeSet<i32> = Default::default();
        // Le partage se cale en quelques images, et la largeur du
        // panneau avec : on lit la dernière.
        for _ in 0..12 {
            abscisses.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape
                    && REGISTERS.iter().any(|(n, _)| *n == t.galley.text())
                {
                    abscisses.insert(t.pos.x.round() as i32);
                }
            }
            sortie.textures_delta.clear();
        }
        abscisses.len()
    };

    let au_large = colonnes_à(1392.0);
    let à_l_étroit = colonnes_à(560.0);
    assert!(
        à_l_étroit > au_large,
        "le panneau court doit gagner une colonne : {au_large} au large, \
         {à_l_étroit} à l'étroit"
    );
    assert_eq!(à_l_étroit, 3, "et trois est ce que la largeur permet");
}

/// La fenêtre des registres étendus montre les masques d'AVX-512 —
/// mais seulement quand il y en a.
///
/// En binaire et non en hexadécimal : ce qu'on y lit est « quelles
/// voies », un bit par élément, et c'est la forme même de la réponse.
/// Le `0x0f` d'un `kmovd` ne dit pas qu'il s'agit des quatre premiers
/// octets ; `00001111` le dit.
///
/// Et seulement quand il y en a : un programme qui n'emploie pas
/// AVX-512 n'a pas à lire huit lignes de zéros.
#[test]
fn the_vector_window_shows_the_avx512_masks_when_there_are_any() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);

    // Sans masque posé : rien ne doit apparaître.
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\nmov rax, 1\nhlt\n".to_owned(),
        show_vectors: true,
        ..Default::default()
    };
    app.do_assemble();
    let peint = paint(&mut app);
    assert!(
        !peint.contains(s.mask_registers),
        "sans AVX-512, pas huit lignes de zéros :\n{peint}"
    );

    // Avec : les huit registres, et la valeur en binaire.
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov eax, 0b1111
kmovd k1, eax
hlt
"
        .to_owned(),
        show_vectors: true,
        ..Default::default()
    };
    app.do_assemble();
    app.do_run();
    for _ in 0..200 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    let peint = paint(&mut app);
    assert!(peint.contains(s.mask_registers), "le titre :\n{peint}");
    assert!(peint.contains("k1|"), "les huit registres :\n{peint}");
    assert!(
        peint.contains("00000000000000000000000000001111"),
        "la valeur en binaire, où l'on LIT les quatre voies :\n{peint}"
    );
}
