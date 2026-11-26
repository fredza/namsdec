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
    // L'ÉCRAN COMPTE LES BARRES D'ICÔNES EN PLUS : le panneau latéral
    // ne peut pas dépasser la moitié de la place de travail, et les
    // barres en prennent trente-quatre de chaque côté. Sans ces
    // soixante-huit pixels, un panneau réglé à 620 se faisait rogner à
    // 606 — ce que le contrôle a vu, et il avait raison de le voir.
    let largeur_des_barres = crate::barres::BARRE_LARGEUR * 2.0;
    let screen = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(1280.0 + largeur_des_barres, 1400.0),
    );
    let wide = 620.0;
    let mut app = NamsdecApp::default();
    app.do_assemble();
    assert!(app.emulator.is_some(), "il faut un processeur à montrer");

    let ctx = egui::Context::default();
    let id_panneau = crate::panneaux::id_racine(&ctx, "sidebar");
    ctx.data_mut(|d| {
        d.insert_persisted(
            id_panneau,
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
            crate::panneaux::état(&ctx, "sidebar")
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
        let part = crate::panneaux::état(&ctx, "sidebar_tab")
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
                let rect = cadre_du_texte(t);
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
    // qui pointe sur un mot, le bandeau dit le dernier pas —, donc on ne
    // retient que ce qui est SOUS LA RANGÉE D'ONGLETS du panneau : c'est
    // là que vit la vue choisie.
    //
    // La rangée a remplacé l'en-tête « État du processeur » le 26
    // octobre 2026 : elle est posée en tête du panneau, juste au-dessus
    // de ce qu'elle commande, et l'onglet souligné EST le titre. Le
    // repère change donc de nom, pas de rôle.
    let mut registres: Vec<egui::Pos2> = Vec::new();
    let mut rangée_y = f32::MAX;
    let mut barres: Vec<egui::Rect> = Vec::new();
    for _ in 0..4 {
        let mut trouvés: Vec<(&'static str, egui::Pos2)> = Vec::new();
        rangée_y = f32::MAX;
        barres.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        // Deux passes : l'ordre de peinture ne dit pas l'ordre à
        // l'écran — le panneau de l'onglet est posé avant le bandeau
        // qu'il surmonte —, donc on relève d'abord les repères, on
        // décide ensuite.
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => match text.galley.text() {
                    "RAX" => trouvés.push(("RAX", text.pos)),
                    "R15" => trouvés.push(("R15", text.pos)),
                    "RIP" => trouvés.push(("RIP", text.pos)),
                    // L'onglet dans le PANNEAU : la rangée n'est plus
                    // dans la barre d'outils, et c'est elle le repère.
                    // La barre d'icônes de droite décale le panneau
                    // vers la gauche : le repère se prend sur la
                    // largeur du panneau, barre comprise.
                    t if t == SidebarTab::Cpu.label(strings(Lang::Fr))
                        && text.pos.x > 1280.0 - SIDEBAR_WIDTH - crate::barres::BARRE_LARGEUR =>
                    {
                        rangée_y = rangée_y.min(text.pos.y);
                    }
                    _ => {}
                },
                egui::epaint::Shape::Rect(r) if r.rect.width() < 20.0 && r.rect.height() > 40.0 => {
                    barres.push(r.rect);
                }
                _ => {}
            }
        }
        registres = trouvés
            .into_iter()
            .filter(|(_, pos)| pos.y > rangée_y)
            .map(|(_, pos)| pos)
            .collect();
        output.textures_delta.clear();
    }
    assert!(
        rangée_y.is_finite(),
        "la rangée d'onglets doit être peinte en tête du panneau : c'est \
         le repère de tout ce contrôle"
    );
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
    // Le bord droit du panneau n'est plus celui de l'écran : la barre
    // d'icônes de droite est posée dehors, et le panneau commence
    // trente-quatre pixels plus à gauche.
    let bord = 1280.0 - crate::barres::BARRE_LARGEUR;
    let droite = *colonnes.iter().next_back().unwrap() as f32;
    assert!(
        droite > bord - SIDEBAR_WIDTH / 2.0,
        "la seconde colonne doit occuper la moitié droite du panneau : {colonnes:?}"
    );
    // Et aucune barre de défilement SOUS la rangée, là où vit l'état du
    // processeur.
    let fautives: Vec<_> = barres
        .iter()
        .filter(|r| r.min.x > bord - SIDEBAR_WIDTH && r.min.y > rangée_y)
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
    // AU MODE AVANCÉ, de l'hexadécimal — comme ce qu'on lit à l'écran.
    assert_eq!(parse_register_value("1f", false), Some(0x1f));
    assert_eq!(parse_register_value("0x1F", false), Some(0x1f));
    assert_eq!(parse_register_value("dead_beef", false), Some(0xdeadbeef));
    // Un moins donne le complément à deux : « mets -1 » est la façon
    // dont on le pense.
    assert_eq!(parse_register_value("-1", false), Some(u64::MAX));
    assert_eq!(parse_register_value("", false), None);
    assert_eq!(parse_register_value("zz", false), None);

    // AU NIVEAU DÉBUTANT, ce qu'on tape est lu comme ce qu'on lit : le
    // volet y écrit « 55 » en clair, et le champ s'ouvre de même. Lire
    // 0x55 sous une saisie de « 55 » serait comprendre autre chose que
    // ce qu'on montre — relevé le 16 novembre 2026 en même temps que
    // le focus qui empêchait toute validation.
    assert_eq!(parse_register_value("55", true), Some(55));
    assert_eq!(parse_register_value("10", true), Some(10));
    // Et `0x` tranche des deux côtés : c'est le préfixe qui dit la
    // base, et celui qui l'écrit sait ce qu'il demande.
    assert_eq!(parse_register_value("0x55", true), Some(0x55));
    assert_eq!(parse_register_value("-1", true), Some(u64::MAX));
    // Ce qui n'est pas un nombre décimal n'en devient pas un.
    assert_eq!(parse_register_value("ff", true), None);

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
                crate::panneaux::état(&ctx, "sidebar_tab").map(|état| état.outer_rect)
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
        let état = egui::scroll_area::State::load(&ctx, egui::Id::unique("sidebar_banner"));
        eprintln!(
            "{l}x{h} : bandeau voulu {:.0}, onglet {:?}, offset {:?}",
            app.sidebar_banner_height,
            app.sidebar_tab_height,
            état.map(|e| e.clamped_offset().y),
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
                let posé = cadre_du_texte(t);
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
    // La dernière ligne de « Prochaine instruction », et ce qui suit :
    // entre les deux, il ne doit y avoir que de l'air normal.
    //
    // Ce qui suit est la RANGÉE D'ONGLETS depuis le 26 octobre 2026 —
    // elle a pris la place de l'en-tête « État du processeur », qu'elle
    // rendait redondant. Le trou se mesure donc jusqu'à son premier
    // onglet.
    let titre = lignes
        .iter()
        .filter(|(t, r)| t == SidebarTab::Cpu.label(s) && r.left() > 1200.0)
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
        SidebarTab::Cpu.label(s)
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
        let id_panneau = crate::panneaux::id_racine(&ctx, "sidebar");
        ctx.data_mut(|d| d.insert_persisted(id_panneau, état));
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

/// « Prévoir avant d'avancer » : la réponse attend le pas.
///
/// **Le défaut corrigé n'est pas un manque, c'est un excès.** Devant une
/// instruction pas encore exécutée, le bandeau disait déjà ce qu'elle
/// allait faire — on appuyait sur « Pas à pas » pour vérifier une
/// réponse qu'on venait de lire, et l'on n'avait jamais l'occasion de se
/// tromper.
///
/// Trois états mesurés dans le même contrôle, et il faut les trois : un
/// contrôle qui ne regarderait que le mode allumé passerait aussi si la
/// réponse avait disparu pour tout le monde.
#[test]
fn la_réponse_attend_le_pas_au_niveau_débutant() {
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rax, 1
mov rbx, 2
hlt
"
        .to_owned(),
        debutant: true,
        prevoir: true,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    // Un pas : on est arrêté SUR `mov rbx, 2`, qui n'a pas encore été
    // exécuté — et `mov rax, 1` vient de l'être, ce qui donne les deux
    // moitiés du chemin d'un coup.
    app.do_step();

    // Ce que l'instruction dit en clair vient du même moteur que le
    // bandeau : on ne le recopie pas à la main, sinon le contrôle
    // passerait au vert le jour où le pseudo-code changerait de forme.
    let réponse = {
        let cpu = app.emulator.as_ref().unwrap().cpu();
        let rip = cpu.regs.rip;
        let octets = cpu.mem.peek_bytes(rip, 16).unwrap();
        let instr = namsdec_disasm::decode_one(octets, rip).unwrap();
        namsdec_disasm::pseudo::of(&instr, "fr")
    };
    // `mov rbx, 2` s'assemble en `mov ebx, 2` — nasm sait qu'écrire dans
    // EBX met à zéro le haut de RBX, et c'est deux octets de moins. La
    // réponse attendue vient donc du moteur, et non d'une chaîne écrite
    // de mémoire.
    assert_eq!(réponse, "EBX = 2", "la réponse à retenir");

    // 1. DÉBUTANT, PRÉVISION ALLUMÉE : l'énoncé entier, pas la réponse.
    let peint = paint(&mut app);
    assert!(
        peint.contains("mov rbx"),
        "l'instruction reste à l'écran — c'est l'énoncé : {peint}"
    );
    assert!(
        peint.contains(s.foresee_question),
        "la question est posée : {peint}"
    );
    assert!(
        peint.contains(s.foresee_reveal),
        "et « Montrer » la donne sans attendre : {peint}"
    );
    assert!(
        !peint.contains(&réponse),
        "mais « {réponse} » ne doit pas être là : {peint}"
    );
    // ET LE CONSTAT, qui est la raison de retenir : l'instruction qu'on
    // vient d'exécuter dit ce qu'elle annonçait, à côté de ce qui a
    // changé.
    let annoncé = app
        .last_step
        .as_ref()
        .and_then(|pas| pas.annonce.clone())
        .expect("le pas qu'on vient de faire annonçait quelque chose");
    assert_eq!(annoncé, "EAX = 1", "et celle du pas déjà fait");
    assert!(
        peint.contains(s.foresee_said) && peint.contains(&annoncé),
        "ce que le pas précédent disait se relit : {peint}"
    );
    // EN DÉCIMAL, et c'est voulu depuis le 10 novembre 2026 (A 9.87) :
    // sous le million, un nombre s'écrit au niveau débutant comme le
    // programme qu'on vient de taper l'écrivait. Ce contrôle attendait
    // « RAX 0x0 -> 0x1 » et c'est LUI qui a dit que la règle avait
    // changé de place — on le met à jour plutôt que de l'assouplir,
    // sans quoi il ne distinguerait plus les deux écritures.
    assert!(
        peint.contains("RAX 0 -> 1"),
        "et ce qui a vraiment changé est juste à côté : {peint}"
    );

    // 2. DÉBUTANT, PRÉVISION ÉTEINTE : namsdec se comporte comme avant.
    app.prevoir = false;
    let peint = paint(&mut app);
    assert!(
        peint.contains(&réponse),
        "éteinte, la réponse revient tout de suite : {peint}"
    );
    assert!(
        !peint.contains(s.foresee_question),
        "et la question ne se pose plus : {peint}"
    );
    assert!(
        !peint.contains(s.foresee_said),
        "ni le rappel de ce que ça disait, qui n'aurait plus d'objet : {peint}"
    );

    // 3. NIVEAU AVANCÉ, PRÉVISION ALLUMÉE : elle ne s'impose pas.
    //
    // C'est une décision, et non un oubli : un exercice imposé à qui se
    // sert de l'outil tous les jours serait une taxe.
    app.prevoir = true;
    app.debutant = false;
    let peint = paint(&mut app);
    assert!(
        peint.contains(&réponse) && !peint.contains(s.foresee_question),
        "au niveau avancé, la prévision ne retient rien : {peint}"
    );
}

/// « Montrer » donne la réponse, et ne la donne que pour cette
/// instruction-là.
///
/// Le bouton est cliqué pour de vrai : poser le champ à la main aurait
/// mesuré la logique — déjà éprouvée dans `crate::prevision` — et non le
/// bouton, qui est ce dont on se sert.
#[test]
fn montrer_donne_la_réponse_sans_attendre_le_pas() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    // Ce que le bandeau retient, et ce qu'un clic doit rendre. Vérifié
    // par le contrôle voisin, qui le tire du moteur plutôt que de la
    // mémoire : `mov rbx, 2` s'assemble en `mov ebx, 2`.
    const RÉPONSE: &str = "EBX = 2";

    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rax, 1
mov rbx, 2
hlt
"
        .to_owned(),
        debutant: true,
        prevoir: true,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();
    // LE BATTEMENT D'ORANGE EST ÉTEINT, et c'est ce qui rend ce contrôle
    // déterministe : une valeur qui vient de changer s'éclaire puis
    // retombe, et redemande une image tant qu'elle brille — `Harness::run`
    // s'arrête alors au bout de quatre en disant que l'interface ne se
    // stabilise pas. Ce n'est pas un défaut de l'application ; c'est une
    // animation qui n'a rien à voir avec ce qu'on mesure ici.
    app.values_changed_at = None;

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui(move |ui| app.draw(ui));
    harness.run();

    // `query_all` ET NON `get_all` : le second PANIQUE quand il ne
    // trouve rien, et « ne rien trouver » est précisément ce que ce
    // contrôle doit pouvoir constater au départ.
    let combien = |harness: &Harness<'_>, quoi: &str| -> usize {
        harness.query_all_by_label_contains(quoi).count()
    };
    assert_eq!(
        combien(&harness, RÉPONSE),
        0,
        "la réponse est retenue au départ"
    );
    assert!(
        combien(&harness, s.foresee_reveal) > 0,
        "et « Montrer » est là"
    );

    harness.get_by_label(s.foresee_reveal).click();
    harness.run();
    assert!(combien(&harness, RÉPONSE) > 0, "un clic suffit à l'obtenir");
    // Et la question ne se repose pas tant qu'on n'a pas avancé : sans
    // l'adresse retenue, le bandeau recacherait la réponse à l'image
    // suivante — ce qui se verrait comme un clignotement, et non comme
    // un défaut de logique.
    harness.run();
    assert!(
        combien(&harness, RÉPONSE) > 0,
        "et elle tient d'une image à l'autre"
    );
}

/// Réinitialiser remet la question : on recommence, donc on reprévoit.
///
/// **Ce que ce contrôle empêche est un faux positif, le genre le plus
/// coûteux :** l'adresse révélée appartient au programme qu'on
/// regardait. Deux programmes différents ont des instructions
/// différentes à la même adresse — et le point d'entrée, lui, est
/// presque toujours le même. Sans la remise à zéro, « Prévoir avant
/// d'avancer » donnait la réponse d'une instruction qu'on n'avait
/// jamais vue, et sa première impression était qu'il ne marchait pas.
#[test]
fn réinitialiser_repose_la_question() {
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rax, 1
hlt
"
        .to_owned(),
        debutant: true,
        prevoir: true,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // La réponse demandée sur la première instruction.
    let rip = app.emulator.as_ref().unwrap().cpu().regs.rip;
    app.prevision_révélée = Some(rip);
    let peint = paint(&mut app);
    assert!(
        peint.contains("EAX = 1") && !peint.contains(s.foresee_question),
        "la réponse est montrée : {peint}"
    );

    // Et « Réinitialiser » la retire : le programme repart du point
    // d'entrée, et la question se pose de nouveau.
    app.do_reset();
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.rip,
        rip,
        "on est revenu au point d'entrée"
    );
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.foresee_question) && !peint.contains("EAX = 1"),
        "et la question est reposée : {peint}"
    );
}

/// CE QUI A CHANGÉ SE DIT EN TOUTES LETTRES, AU NIVEAU DÉBUTANT.
///
/// La bande qui explique le dernier pas est l'endroit où l'on apprend
/// — « lire, prévoir, avancer, CONSTATER ». Elle ne constatait rien de
/// lisible à qui débute : « RCX 0x3 -> 0x2 » demande de savoir lire
/// l'hexadécimal, et « ZF 0 -> 1 » de savoir ce qu'est ZF. Deux choses
/// qu'on vient précisément apprendre.
///
/// Deux changements, et ils ne valent QUE sous ce niveau : au-dessus,
/// l'hexadécimal est la langue du métier et une phrase par indicateur
/// à chaque pas serait du bruit. Le contrôle mesure donc les deux
/// niveaux, sans quoi il ne prouverait pas que la règle s'arrête.
///
/// L'ÉTAT EST SEMÉ À LA MAIN. Faire tourner un vrai programme jusqu'à
/// ce qu'un indicateur bascule demanderait nasm, et ce qu'on vérifie
/// ici est une PHRASE, pas une exécution. Ce qui invalide ce semis :
/// le jour où `last_step_ui` lira autre chose que `regs_before`,
/// `flags_before` et `last_step`.
#[test]
fn au_niveau_debutant_le_dernier_pas_se_dit_en_toutes_lettres() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;

    for débutant in [true, false] {
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
        app.debutant = débutant;

        // L'AVANT : RCX à 3, aucun indicateur posé.
        let mut avant = namsdec_core::Registers::default();
        avant.set64(Reg64::Rcx, 3);
        app.regs_before = Some(avant);
        app.flags_before = Some(RFlags::empty());

        // L'APRÈS : RCX à 2, et le zéro posé. Les deux ensemble, parce
        // qu'un `dec` écrit bien l'un et l'autre.
        let mut mem = Memory::new(BASE, 0x1000);
        mem.load(BASE, &[0xf4]).expect("un hlt se charge");
        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = BASE;
        cpu.regs.set64(Reg64::Rcx, 2);
        cpu.flags = RFlags::ZF;
        app.emulator = Some(Box::new(namsdec_emu::Emulator::new(cpu)));
        app.last_step = Some(DernierPas {
            addr: BASE,
            texte: "dec rcx".to_owned(),
            annonce: None,
        });

        let mut peints = String::new();
        for _ in 0..3 {
            peints.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    peints.push_str(t.galley.text());
                    peints.push('|');
                }
            }
            sortie.textures_delta.clear();
        }

        let (_, rôle_du_zéro) = crate::sidebar::flag_words("ZF", app.lang);
        if débutant {
            assert!(
                peints.contains("RCX 3 -> 2"),
                "au niveau débutant, un petit nombre s'écrit en décimal — \
                 le programme disait `mov rcx, 3`, pas `mov rcx, 0x3`"
            );
            assert!(
                peints.contains(rôle_du_zéro),
                "et l'indicateur qui vient d'être posé se dit en toutes lettres : \
                 « {rôle_du_zéro} » n'est pas peint"
            );
        } else {
            assert!(
                peints.contains("RCX 0x3 -> 0x2"),
                "au-dessus du niveau débutant, rien ne change : l'hexadécimal est la \
                 langue du métier"
            );
            assert!(
                !peints.contains(rôle_du_zéro),
                "et une phrase par indicateur à chaque pas y serait du bruit"
            );
        }
    }
}

/// LE VOLET DE LA MACHINE DIT CE QU'IL MONTRE, AU MODE SIMPLE.
///
/// La pile portait sa légende depuis toujours — « en grisé, au-dessus
/// de RSP : déjà dépilé ». Les registres et les indicateurs n'en
/// avaient aucune : dix-sept noms de trois lettres, sept sigles de
/// deux, et pas un mot pour dire ce que c'est. Dans le mode dont la
/// règle est que chaque bande se présente elle-même et que rien ne se
/// survole pour savoir ce que c'est, c'était le seul endroit qui
/// montrait de l'état brut sans le nommer.
///
/// ET LA PLACE COMPTE AUTANT QUE LE TEXTE : celle des registres doit
/// précéder la liste. La première capture la posait sous les dix-sept
/// lignes, c'est-à-dire qu'on lisait dix-sept lignes incompréhensibles
/// PUIS ce qu'elles étaient.
///
/// Au mode avancé, rien : le panneau latéral y est déjà serré, et ces
/// phrases seraient du bruit devant quelqu'un qui sait.
#[test]
fn au_mode_simple_le_volet_de_la_machine_se_presente() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;

    for simple in [true, false] {
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
        app.debutant = simple;
        let mut mem = Memory::new(BASE, 0x1000);
        mem.load(BASE, &[0xf4]).expect("un hlt se charge");
        let mut cpu = Cpu::new(mem);
        cpu.regs.rip = BASE;
        app.emulator = Some(Box::new(namsdec_emu::Emulator::new(cpu)));

        let s = strings(app.lang);
        let mut légende: Option<egui::Rect> = None;
        let mut premier_registre: Option<egui::Rect> = None;
        let mut indicateurs = false;
        let début = s.etabli_regs_legend.chars().take(30).collect::<String>();
        for _ in 0..3 {
            légende = None;
            premier_registre = None;
            indicateurs = false;
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                let texte = t.galley.text();
                if texte.starts_with(&début) {
                    légende = Some(cadre_du_texte(t));
                }
                if texte == "RAX" {
                    premier_registre = Some(cadre_du_texte(t));
                }
                if texte.starts_with(&s.etabli_flags_legend.chars().take(30).collect::<String>()) {
                    indicateurs = true;
                }
            }
            sortie.textures_delta.clear();
        }

        if !simple {
            assert!(
                légende.is_none() && !indicateurs,
                "au mode avancé, le panneau latéral ne porte aucune de ces deux phrases"
            );
            continue;
        }
        let légende = légende.expect("au mode simple, les registres disent ce qu'ils sont");
        let rax = premier_registre.expect("et RAX se peint toujours");
        assert!(
            indicateurs,
            "les sept indicateurs aussi disent ce qu'ils sont"
        );
        assert!(
            légende.bottom() <= rax.top() + 0.5,
            "une légende se lit AVANT ce qu'elle explique : elle est à y = {:.0}, \
             et RAX — la première des dix-sept lignes — à y = {:.0}",
            légende.top(),
            rax.top()
        );
    }
}

/// « MONTRER » SE DÉFAIT : LA RÉPONSE SE RECACHE.
///
/// Demandé à l'écran le 10 novembre 2026, capture à l'appui : « mets
/// un btn de cacher/afficher ». « Montrer » donnait la réponse et il
/// n'y avait plus de retour — elle restait jusqu'au pas suivant, et
/// l'on ne pouvait plus se reposer la question. Un geste qui ne se
/// défait pas se prend avec hésitation, et l'hésitation est exactement
/// ce que ce mode cherche à lever.
///
/// LE CYCLE ENTIER EST MESURÉ — caché, montré, caché — parce que c'est
/// lui qu'on a demandé. Vérifier que le bouton existe ne dirait pas
/// qu'il rend la question.
#[test]
fn la_reponse_montree_se_recache() {
    use namsdec_core::{Cpu, Memory};
    const BASE: u64 = 0x40_0000;

    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        debutant: true,
        prevoir: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    // Un `mov` à l'arrêt : le pseudo-code en dit plus que
    // l'instruction, donc il y a bien une réponse à retenir.
    let mut mem = Memory::new(BASE, 0x1000);
    mem.load(BASE, &[0xb8, 0x05, 0x00, 0x00, 0x00])
        .expect("mov eax, 5 se charge");
    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = BASE;
    app.emulator = Some(Box::new(namsdec_emu::Emulator::new(cpu)));

    let s = strings(app.lang);
    let peints = |app: &mut NamsdecApp| {
        let mut tout = String::new();
        for _ in 0..3 {
            tout.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    tout.push_str(t.galley.text());
                    tout.push('|');
                }
            }
            sortie.textures_delta.clear();
        }
        tout
    };

    // 1. LA QUESTION EST POSÉE : « Montrer », pas « Cacher ».
    let vu = peints(&mut app);
    assert!(
        vu.contains(s.foresee_reveal),
        "la question attend : « {} » doit être là",
        s.foresee_reveal
    );
    assert!(
        !vu.contains(s.foresee_hide),
        "et rien n'est encore montré : « {} » n'a rien à faire là",
        s.foresee_hide
    );

    // 2. ON MONTRE : la réponse paraît, et « Cacher » avec elle.
    app.prevision_révélée = Some(BASE);
    let vu = peints(&mut app);
    assert!(
        vu.contains(s.foresee_hide),
        "la réponse montrée, le retour doit être offert : « {} »",
        s.foresee_hide
    );
    assert!(
        !vu.contains(s.foresee_reveal),
        "et « {} » n'a plus de sens : c'est déjà fait",
        s.foresee_reveal
    );

    // 3. ON RECACHE : on retrouve la question.
    app.prevision_révélée = None;
    let vu = peints(&mut app);
    assert!(
        vu.contains(s.foresee_reveal) && !vu.contains(s.foresee_hide),
        "et la question se repose, telle qu'elle était"
    );
}

/// AU NIVEAU DÉBUTANT, LE VOLET DE LA MACHINE ÉCRIT LES NOMBRES EN
/// CLAIR.
///
/// Le défaut tenait sur une seule capture, le 16 novembre 2026
/// (`établi / un exercice de lecture`) : l'énoncé demande « Que vaut
/// RAX à l'arrêt ? », la bonne réponse est 55, et le volet affichait
/// `0000000000000037`. L'exercice portait sa propre réponse à l'écran,
/// dans une écriture que celui qui apprend ne sait pas encore lire —
/// pendant que la bande du bas disait déjà « RCX 3 → 2 ».
#[test]
fn at_beginner_level_registers_are_written_plainly() {
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rax, 55
    hlt
"
        .to_owned(),
        debutant: true,
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();

    let peint = paint(&mut app);
    assert!(
        peint.contains("55"),
        "la valeur se lit en clair :\n{peint:.400}"
    );
    assert!(
        !peint.contains("0000000000000037"),
        "et pas en hexadécimal :\n{peint:.400}"
    );
    // L'ALIGNEMENT EST GARDÉ : seize colonnes, remplies d'espaces
    // invisibles plutôt que des zéros estompés de l'hexadécimal. Sans
    // lui, les colonnes de la grille dansent dès qu'une valeur passe
    // de 9 à 10.
    assert!(
        peint.contains(&format!("{:>16}", 55)),
        "aligné à droite comme l'hexadécimal qu'il remplace"
    );
    // UNE ADRESSE RESTE EN HEXADÉCIMAL : au-dessus du million, la
    // règle de `nombre_lisible` rend la main à l'hexadécimal, et c'est
    // ce que RSP et RIP montrent.
    let rsp = app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rsp);
    assert!(rsp >= 1_000_000, "la pile vit haut : {rsp:#x}");
    assert!(
        peint.contains(&format!("{rsp:016x}")),
        "les adresses gardent l'hexadécimal :\n{peint:.400}"
    );

    // AU NIVEAU AVANCÉ, RIEN NE CHANGE : l'hexadécimal est la langue
    // du métier, et c'est elle qu'on compare au vidage et à gdb.
    app.debutant = false;
    let peint = paint(&mut app);
    assert!(
        peint.contains("0000000000000037"),
        "le mode avancé garde ses seize chiffres :\n{peint:.400}"
    );
}

/// LES HUIT REGISTRES HAUTS NE PRENNENT PAS LA MOITIÉ DU VOLET POUR NE
/// RIEN DIRE.
///
/// R8 à R15 sont à zéro dans toutes les captures du mode simple, et
/// occupent huit rangées sur dix-sept — pendant que la pile, en
/// dessous, se défile faute de place. Vingt étapes du parcours sur cent
/// dix-sept les emploient.
#[test]
fn the_high_registers_show_up_when_the_program_uses_them() {
    let programme =
        |corps: &str| format!("section .text\nglobal _start\n_start:\n    {corps}\n    hlt\n");
    let mut app = NamsdecApp {
        source: programme("mov rax, 1"),
        debutant: true,
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();
    // DANS LA GRILLE : le nom suivi de sa valeur. « R12 » tout court
    // paraît ailleurs — la convention d'appel le range parmi les
    // registres à rendre intacts —, et le chercher seul mesurerait
    // autre chose que ce qu'on veut.
    let dans_la_grille = |peint: &str, nom: &str| peint.contains(&format!("|{nom}|{:>16}|", 0));
    let peint = paint(&mut app);
    assert!(
        !dans_la_grille(&peint, "R12"),
        "repliés tant qu'ils n'ont rien à dire :\n{peint:.400}"
    );
    let s = strings(app.lang);
    assert!(
        peint.contains(s.registers_high_show),
        "et la ligne qui les rend est là :\n{peint:.400}"
    );

    // ON LES DEMANDE : ils paraissent, et le réglage part dans
    // `settings.json` comme tous les autres.
    app.registres_hauts = true;
    let peint = paint(&mut app);
    assert!(dans_la_grille(&peint, "R12"), "demandés, ils sont là");
    assert!(
        serde_json::to_string(&app.settings_snapshot())
            .unwrap()
            .contains("\"registres_hauts\":true"),
        "et le choix se retient"
    );

    // ET UN PROGRAMME QUI EN EMPLOIE UN LES DÉPLIE TOUS, SANS QU'ON
    // DEMANDE RIEN : les huit ensemble, sinon la grille se
    // réorganiserait registre par registre sous les yeux de celui qui
    // avance d'un pas.
    let mut app = NamsdecApp {
        source: programme("mov r9, 3"),
        debutant: true,
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();
    let peint = paint(&mut app);
    assert!(
        peint.contains(&format!("|R9|{:>16}|", 3)) && dans_la_grille(&peint, "R12"),
        "un seul en jeu, et les huit paraissent :\n{peint:.400}"
    );
    assert!(
        !peint.contains(s.registers_high_show),
        "et la ligne n'a plus rien à offrir"
    );

    // AU NIVEAU AVANCÉ, LES SEIZE SONT TOUJOURS LÀ : celui qui
    // travaille veut ses registres à la même place d'une séance à
    // l'autre, même vides.
    let mut app = NamsdecApp {
        source: programme("mov rax, 1"),
        debutant: false,
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();
    let peint = paint(&mut app);
    assert!(
        peint.contains(&format!("|R12|{:016x}|", 0)),
        "le mode avancé les montre toujours, en hexadécimal :\n{peint:.400}"
    );
}

/// OÙ L'ON EN EST DANS LE TEMPS DU PROGRAMME.
///
/// La question que le mode simple ne traitait pas : on avançait pas à
/// pas sans savoir combien de pas on avait faits, ni combien de fois la
/// boucle avait tourné — alors que c'est la question même de plusieurs
/// étapes du parcours.
#[test]
fn the_beginner_bench_says_how_far_the_program_has_got() {
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    hlt
"
        .to_owned(),
        debutant: true,
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();
    app.do_step();
    let peint = paint(&mut app);
    assert!(
        peint.contains("Pas 2 depuis le début"),
        "le compte des pas :\n{peint:.600}"
    );
    assert!(
        !peint.contains("passage"),
        "et rien sur les passages tant qu'on n'est passé qu'une fois :\n{peint:.600}"
    );

    // Deux tours de boucle : le `dec` a été exécuté deux fois, et c'est
    // ce que dit la ligne.
    app.do_step();
    app.do_step();
    let peint = paint(&mut app);
    assert!(
        peint.contains("passage 2 par cette ligne"),
        "le tour de boucle :\n{peint:.600}"
    );

    // AU NIVEAU AVANCÉ, RIEN : la chronologie et sa barre répondent
    // déjà à cette question, et une ligne de plus à chaque pas devant
    // quelqu'un qui n'a rien demandé est du bruit.
    app.debutant = false;
    let peint = paint(&mut app);
    assert!(
        !peint.contains("depuis le début"),
        "le mode avancé a sa chronologie :\n{peint:.600}"
    );
}

/// LES LÉGENDES DU VOLET TIENNENT EN DEUX LIGNES.
///
/// La règle vient de A 9.87 et le guide la porte depuis : à trois
/// lignes, la pile tombait sous la ligne de flottaison — une légende
/// qui chasse ce qu'elle accompagne ne vaut pas mieux que pas de
/// légende. Rien ne la retenait pourtant, et la phrase des registres
/// est passée à QUATRE lignes le 16 novembre 2026, le jour où elle a
/// dû dire que les nombres sont écrits en clair.
#[test]
fn les_legendes_du_volet_tiennent_en_deux_lignes() {
    for lang in [Lang::Fr, Lang::En] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            debutant: true,
            lang,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        app.do_assemble();
        let s = strings(lang);
        let mut vues: Vec<(String, usize)> = Vec::new();
        for _ in 0..3 {
            vues.clear();
            let mut sortie = ctx.run_ui(écran_du_banc(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    let texte = t.galley.text();
                    if texte == s.etabli_regs_legend || texte == s.etabli_flags_legend {
                        vues.push((texte.to_owned(), t.galley.rows.len()));
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        assert_eq!(vues.len(), 2, "les deux légendes sont peintes ({lang:?})");
        for (texte, lignes) in vues {
            assert!(
                lignes <= 2,
                "« {texte} » tient en {lignes} lignes, et la pile tombe sous le bord"
            );
        }
    }
}

/// POSER UN REGISTRE À LA MAIN MARCHE VRAIMENT — éprouvé en cliquant,
/// en tapant et en validant.
///
/// Le contrôle d'à côté vérifiait la LECTURE de ce qu'on tape, et il
/// passait au vert pendant que le geste entier ne faisait rien : le
/// champ redemandait le focus à chaque image, si bien qu'il ne le
/// perdait jamais — et `lost_focus()`, qui valide la saisie, n'arrivait
/// jamais non plus. « La modification des registres n'est pas
/// effective » (16 novembre 2026).
///
/// C'est la même leçon que la case « ne plus demander » du 11 novembre :
/// un contrôle qui appelle la méthode serait passé au vert ; celui qui
/// clique tombe dessus du premier coup.
#[test]
fn un_registre_se_pose_vraiment_a_la_main() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use namsdec_core::{Cpu, Memory};

    const BASE: u64 = 0x40_0000;
    let mut mem = Memory::new(BASE, 0x1000);
    mem.load(BASE, &[0xf4]).expect("un hlt se charge");
    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = BASE;
    cpu.regs.set64(Reg64::Rcx, 5);
    let app = NamsdecApp {
        debutant: true,
        emulator: Some(Box::new(namsdec_emu::Emulator::new(cpu))),
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui_state(|ui, app: &mut NamsdecApp| app.draw(ui), app);
    harness.run();

    // La valeur se clique : au niveau débutant elle s'écrit en clair,
    // et c'est ce texte-là qu'on vise.
    let cible = format!("{:>16}", 5);
    harness.get_by_label(cible.as_str()).click();
    harness.run();
    assert!(
        harness.state().editing_register.is_some(),
        "le champ s'ouvre sur la valeur cliquée"
    );

    // ON TAPE VRAIMENT, et c'est le sujet : « fais pour que le registre
    // soit enregistré par Entrée ». Poser la chaîne dans l'état de
    // l'application sauterait le chemin du clavier, c'est-à-dire
    // exactement ce qui ne marchait pas — le champ reprenait le focus
    // à chaque image, et les frappes allaient quelque part sans que la
    // validation n'arrive jamais.
    //
    // Le champ s'ouvre sur la valeur d'aujourd'hui (« 5 ») : on
    // l'efface d'un retour arrière avant d'écrire la nouvelle.
    harness.key_press(egui::Key::Backspace);
    harness.run();
    harness
        .input_mut()
        .events
        .push(egui::Event::Text("12".to_owned()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.run();

    let app = harness.state();
    assert!(
        app.editing_register.is_none(),
        "le champ se referme une fois validé"
    );
    assert_eq!(
        app.emulator
            .as_ref()
            .map(|e| e.cpu().regs.get64(Reg64::Rcx)),
        Some(12),
        "ET LA VALEUR EST POSÉE — en décimal, comme le volet l'écrit"
    );
}

/// LE NOM DU FICHIER DONNE SON CHEMIN AU PRESSE-PAPIERS.
///
/// Demandé le 16 novembre 2026 : « tronquer le nom du fichier ouvert
/// […] avec une infobulle donnant le chemin complet et l'enregistrer
/// dans le presse-papier ». L'infobulle MONTRE le chemin depuis le
/// 11 novembre ; elle ne le donne pas, et on le recopiait en le lisant.
#[test]
fn le_nom_du_fichier_copie_son_chemin() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let chemin = std::path::PathBuf::from("/tmp/un-dossier-assez-long/mon-programme.asm");
    let app = NamsdecApp {
        debutant: true,
        current_file: Some(chemin.clone()),
        ..Default::default()
    };
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui_state(|ui, app: &mut NamsdecApp| app.draw(ui), app);
    harness.run();

    // Le nom est abrégé s'il est long, et c'est lui qu'on clique.
    let abrégé = crate::etabli::nom_abrégé("mon-programme.asm", crate::etabli::NOM_MAX);
    harness.get_by_label(abrégé.as_str()).click();
    harness.run();

    // Ce que l'application a mis dans le presse-papiers : le statut le
    // dit aussi, et c'est lui qu'on lit — la commande de copie est
    // consommée par le dos d'egui dès la frame suivante.
    // LA NOTE EST LA PREUVE : `path_copied` n'est posé QUE par la
    // branche qui vient d'appeler `copy_text` avec le chemin complet.
    // La commande de copie elle-même est consommée par le dos d'egui à
    // la frame suivante, et ne se relit pas d'ici.
    let app = harness.state();
    let note = match &app.status {
        Status::Note(texte) => texte(strings(app.lang)),
        autre => panic!("le clic doit répondre : {autre:?}"),
    };
    assert_eq!(
        note,
        strings(app.lang).path_copied,
        "le clic met le chemin complet dans le presse-papiers"
    );
}
