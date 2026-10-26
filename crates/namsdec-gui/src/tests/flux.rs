//! La fenêtre du déroulement : le graphe et le pseudo-code.
//!
//! La forme du programme, ce que chaque bloc fait quand namsdec sait le
//! dire, et la bascule entre les deux vues.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// Le panneau de désassemblage s'ouvre sur le pseudo-code : lire
/// « RAX = RAX + RCX » avant « add rax, rcx » est le chemin naturel.
#[test]
fn the_disassembly_panel_opens_on_pseudo_code() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_disassembly: true,
        source: "\
global _start
section .text
_start:
mov rax, 0
mov rcx, 5
boucle:
add rax, rcx
dec rcx
jnz boucle
hlt
"
        .to_owned(),
        ..Default::default()
    };
    // Le moteur par défaut est le pseudo-code, et non la syntaxe
    // machine : c'est le premier qu'on veut lire.
    assert_eq!(app.disassembler, PSEUDO_ID);
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
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
    assert!(painted.contains("Désassemblage"), "{painted}");
    assert!(painted.contains("RAX = RAX + RCX"), "{painted}");
    assert!(painted.contains("RCX = RCX − 1"), "{painted}");
    assert!(
        painted.contains("si différent, aller à"),
        "le saut dit sa condition : {painted}"
    );
    // Les étiquettes structurent le listing.
    assert!(painted.contains("boucle:"), "{painted}");

    // L'autre moteur montre la syntaxe qu'on écrit.
    app.disassembler = NASM_ID.to_owned();
    let painted = peindre(&mut app);
    assert!(painted.contains("add rax, rcx"), "{painted}");
    assert!(!painted.contains("RAX = RAX + RCX"), "{painted}");
}

/// La boîte s'élargit pour ce qu'elle porte, entre deux bornes.
///
/// Une largeur commune à tout le graphe : les colonnes restent alignées,
/// ce qui est la moitié de ce qu'un tel dessin sert à montrer.
#[test]
fn flow_boxes_take_the_width_their_text_needs() {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };

    let mesure = |cfg: &namsdec_re::cfg::Cfg| {
        let mut large = 0.0;
        for _ in 0..2 {
            let mut sortie = ctx.run_ui(input.clone(), |ui| {
                let police = egui::FontId::monospace(11.0);
                large = flow_box_width(ui, cfg, strings(Lang::Fr), &police, 1.0);
            });
            sortie.textures_delta.clear();
        }
        large
    };

    // Un programme aux instructions courtes, et le même avec une
    // instruction longue : le second doit demander plus de place.
    let court = namsdec_re::cfg::build(&disassemble_code(
        &namsdec_asm::assemble(
            "section .text\nglobal _start\n_start:\nnop\nhlt\n",
            BASE_ADDR,
        )
        .expect("l'assemblage"),
    ));
    let long = namsdec_re::cfg::build(&disassemble_code(
        &namsdec_asm::assemble(
            "section .text\nglobal _start\n_start:\nmov qword [rax+rbx*8+0x11223344], 0x55667788\nhlt\n",
            BASE_ADDR,
        )
        .expect("l'assemblage"),
    ));

    let (étroit, large) = (mesure(&court), mesure(&long));
    assert!(
        large > étroit,
        "une instruction plus longue demande une boîte plus large : {large} contre {étroit}"
    );
    // Et jamais au-delà des bornes : une seule instruction bavarde ne
    // doit pas étaler le graphe entier.
    assert!((220.0..=560.0).contains(&large), "{large}");
    assert!((220.0..=560.0).contains(&étroit), "{étroit}");
}

/// Une boîte est assez haute pour tout ce qu'elle écrit.
///
/// Elle comptait l'en-tête et les huit instructions montrées, mais pas
/// la ligne « … 4 » qui dit combien restent : celle-ci se peignait sous
/// le cadre, à côté du bloc d'en dessous.
#[test]
fn a_block_is_tall_enough_for_the_line_that_counts_the_rest() {
    const LIGNES_MAX: usize = 8;
    // La règle telle que le dessin l'applique : l'en-tête, ce qu'on
    // montre, et la ligne de reste quand il y en a un.
    let lignes = |combien: usize| 1 + combien.min(LIGNES_MAX) + usize::from(combien > LIGNES_MAX);

    // Un bloc court : l'en-tête et ses instructions, rien de plus.
    assert_eq!(lignes(3), 4);
    // Pile la limite : rien n'est caché, donc pas de ligne de reste.
    assert_eq!(lignes(LIGNES_MAX), LIGNES_MAX + 1);
    // Une de trop : la ligne « … 1 » doit avoir sa place.
    assert_eq!(lignes(LIGNES_MAX + 1), LIGNES_MAX + 2);
    // Et le bloc de la capture : douze instructions, huit montrées.
    assert_eq!(lignes(12), 10);
}

/// L'en-tête d'un bloc et « le programme est ici » ne se recouvrent pas.
///
/// Les deux étaient peints aux deux bouts de la même ligne sans que
/// personne ne mesure la place : sur une boîte étroite, le libellé
/// venait s'imprimer par-dessus l'adresse, et l'on ne lisait plus ni
/// l'un ni l'autre. La largeur des boîtes leur réserve désormais de quoi
/// tenir côte à côte.
#[test]
fn a_block_header_and_the_here_label_never_overlap() {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };

    // Le cas le plus serré : un programme dont toutes les instructions
    // sont courtes, où rien d'autre ne force la boîte à s'élargir.
    let cfg = namsdec_re::cfg::build(&disassemble_code(
        &namsdec_asm::assemble(
            "section .text\nglobal _start\n_start:\nnop\nhlt\n",
            BASE_ADDR,
        )
        .expect("l'assemblage"),
    ));

    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        let mut large = 0.0;
        let mut glyphe = 0.0;
        for _ in 0..2 {
            let mut sortie = ctx.run_ui(input.clone(), |ui| {
                let police = egui::FontId::monospace(11.0);
                large = flow_box_width(ui, &cfg, s, &police, 1.0);
                glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(&police, ' '));
            });
            sortie.textures_delta.clear();
        }
        // Ce qui tient dans la boîte, marges déduites — le même calcul
        // que le dessin.
        let colonnes = ((large - 12.0) / glyphe).floor() as usize;
        // « 0x0001d491  ? » puis le libellé, avec de quoi les séparer.
        let nécessaire = 16 + s.flow_here.chars().count() + 3;
        assert!(
            colonnes >= nécessaire,
            "{lang:?} : {colonnes} colonnes pour {nécessaire} nécessaires — \
             « {} » recouvrirait l'adresse",
            s.flow_here
        );
    }
}

/// Le pseudo-code doit couvrir ce qu'on lit vraiment dans un binaire.
///
/// Mesuré sur un binaire du système, pas sur des cas choisis : ce qui
/// n'est pas traduit tombe dans le repli NASM, ce qui reste honnête mais
/// n'apprend rien. Le seuil est large — il attrape une régression, pas
/// une variation de quelques instructions.
#[test]
fn the_pseudo_code_covers_a_real_binary() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes, "programme");
    let state = app.assemble_state.as_ref().expect("chargé");

    let mut total = 0usize;
    let mut traduites = 0usize;
    for instruction in &state.disasm {
        let Some(decoded) = namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
        else {
            continue;
        };
        total += 1;
        // Le repli rend la forme NASM telle quelle : c'est à cela qu'on
        // reconnaît une instruction qu'on ne sait pas dire.
        if namsdec_disasm::pseudo::of(&decoded, "fr") != namsdec_disasm::format_one(&decoded) {
            traduites += 1;
        }
    }
    assert!(total > 5_000, "le binaire doit être gros : {total}");
    let part = traduites as f32 / total as f32;
    assert!(
        part > 0.95,
        "seulement {traduites} instructions sur {total} se disent en clair ({:.0} %)",
        part * 100.0
    );
}

/// Le déroulement s'ouvre par « Espace g » et montre la forme du
/// programme.
///
/// Un listing se lit de haut en bas ; un programme, non. La fenêtre
/// dessine ce qu'on reconstitue de tête en lisant : les blocs, les
/// sauts pris et non pris, et la boucle qui remonte.
#[test]
fn the_flow_window_draws_the_shape_of_the_program() {
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
    call fin
    hlt
fin:
    ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    app.do_step();

    // Le raccourci : `Espace g`, sous le même préfixe que l'explorateur.
    let chord = app
        .keybindings
        .map
        .get(&Action::ShowFlow)
        .expect("le raccourci existe");
    assert_eq!(
        *chord,
        crate::keybindings::Chord::leader(&[egui::Key::Space, egui::Key::V, egui::Key::G]),
        "« Espace g » ouvre le déroulement"
    );

    app.dispatch(Action::ShowFlow);
    // Le raccourci MONTRE LA VUE, et depuis le 25 octobre 2026 la vue
    // est un onglet de la rangée : « Espace v g » la choisit au lieu
    // d'ouvrir une fenêtre. Ce qui suit vaut pour les deux lieux — le
    // graphe se peint le même, et c'est tout l'objet d'un corps unique.
    assert_eq!(
        app.vue_centrale,
        VueCentrale::Flow,
        "l'action montre le déroulement"
    );
    assert!(
        !app.show_flow,
        "et elle ne le détache pas : une vue est quelque part, et la \
         rangée est son lieu par défaut"
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
    let mut peint = String::new();
    let mut traits = 0usize;
    let mut flèches = 0usize;
    for _ in 0..2 {
        peint.clear();
        traits = 0;
        flèches = 0;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => {
                    peint.push_str(text.galley.text());
                    peint.push('|');
                }
                // Les flèches : un trait pour le corps, un triangle pour
                // la pointe.
                egui::epaint::Shape::LineSegment { .. } => traits += 1,
                egui::epaint::Shape::Path(path) if path.points.len() == 3 => flèches += 1,
                _ => {}
            }
        }
        output.textures_delta.clear();
    }

    // La vue se nomme à l'écran : dans la rangée, c'est son onglet qui
    // le fait — « Déroulement » —, le titre entier (« Déroulement du
    // programme ») restant au bandeau de la fenêtre quand on la
    // détache, et en infobulle sur l'onglet.
    assert!(
        peint.contains(VueCentrale::Flow.label(strings(Lang::Fr))),
        "{peint}"
    );
    // Les instructions du programme sont dans les boîtes.
    for attendu in ["dec rcx", "hlt"] {
        assert!(peint.contains(attendu), "« {attendu} » manque :\n{peint}");
    }
    // Et le bloc où le programme se trouve est signalé.
    assert!(
        peint.contains(strings(Lang::Fr).flow_here),
        "le bloc courant doit se voir :\n{peint}"
    );
    // Des flèches, et pas qu'une : le saut pris, le saut non pris,
    // l'appel, le retour de l'appel.
    assert!(traits >= 4, "trop peu de traits : {traits}");
    assert!(flèches >= 4, "trop peu de pointes : {flèches}");

    // Sans programme assemblé, la fenêtre dit quoi faire plutôt que de
    // rester vide.
    let mut vierge = NamsdecApp {
        show_flow: true,
        ..Default::default()
    };
    let peint = paint(&mut vierge);
    assert!(peint.contains(strings(Lang::Fr).flow_empty), "{peint}");
}

/// Le graphe se lit comme le programme : entrée en haut, boucle qui
/// remonte, deux chemins après un test.
#[test]
fn the_flow_graph_says_what_the_listing_hides() {
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
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    let state = app.assemble_state.as_ref().expect("assemblé");
    let cfg = namsdec_re::cfg::build(&state.disasm);

    // Trois blocs : l'entrée, le corps de la boucle, la sortie.
    assert!(cfg.blocks.len() >= 3, "{:?}", cfg.blocks);
    let niveaux = cfg.layers();
    assert_eq!(niveaux[0], 0, "l'entrée est en haut");

    // Le test ouvre bien deux chemins, et l'un des deux remonte.
    let corps = cfg
        .blocks
        .iter()
        .position(|b| b.exit == namsdec_re::cfg::Exit::Branch)
        .expect("un bloc se termine par un saut conditionnel");
    let pris = cfg
        .edges
        .iter()
        .find(|e| e.from == corps && e.kind == namsdec_re::cfg::Kind::Taken)
        .expect("le chemin du saut pris");
    assert!(
        cfg.blocks[pris.to].start <= cfg.blocks[corps].start,
        "la boucle remonte"
    );
    assert!(
        cfg.edges
            .iter()
            .any(|e| e.from == corps && e.kind == namsdec_re::cfg::Kind::NotTaken),
        "et l'autre chemin continue"
    );
}

/// Le graphe nomme la fonction quand le binaire porte ses symboles.
///
/// Il n'en donnait que l'adresse : « vérifie_mdp » se retient, `0x1d491`
/// non. L'adresse reste affichée à côté — c'est elle qu'on tape dans
/// « Comment arrive-t-on à », et elle lève l'ambiguïté quand la
/// fermeture par sauts se trompe, faute de symboles.
#[test]
fn the_flow_window_names_the_function_it_shows() {
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    call vérifie
    hlt
vérifie:
    mov rax, 1
    ret
"
        .to_owned(),
        show_flow: true,
        flow_current_only: true,
        ..Default::default()
    };
    app.do_assemble();
    // Sans nasm, rien à vérifier.
    let Some(état) = app.assemble_state.as_ref() else {
        return;
    };
    if état.output.symbols.is_empty() {
        return;
    }

    // Le symbole se retrouve bien à son adresse : c'est ce dont dépend
    // l'affichage.
    let vérifie = état
        .output
        .symbols
        .iter()
        .find(|symbole| symbole.name == "vérifie");
    let Some(vérifie) = vérifie else {
        return;
    };
    assert_eq!(
        symbol_at(&état.output.symbols, vérifie.addr).as_deref(),
        Some("vérifie"),
        "l'adresse de la fonction doit rendre son nom"
    );

    // Et au milieu de la fonction, le nom porte son décalage : on sait
    // où l'on est, et dans quoi.
    let dedans = symbol_at(&état.output.symbols, vérifie.addr + 1);
    assert!(
        dedans
            .as_deref()
            .is_some_and(|nom| nom.starts_with("vérifie+")),
        "au milieu, le nom porte son décalage : {dedans:?}"
    );
}

/// Le graphe s'écrit dans un fichier, et pas seulement dans le
/// presse-papier.
///
/// Un graphe se garde : celui d'avant le correctif, celui d'après,
/// celui qu'on joint au compte rendu. Le presse-papier, lui, ne survit
/// pas à la copie suivante.
#[test]
fn the_graph_goes_to_a_file_and_not_only_to_the_clipboard() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let chemin = dir.path().join("graphe.dot");

    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    // Sans programme, rien à écrire — et on le dit plutôt que d'écrire
    // un fichier vide.
    app.export_dot(&chemin);
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
    assert!(!chemin.exists(), "aucun fichier n'est créé pour rien");

    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.export_dot(&chemin);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    let dot = std::fs::read_to_string(&chemin).expect("le fichier est écrit");
    assert!(
        dot.starts_with("digraph"),
        "du DOT, et pas autre chose :\n{dot}"
    );
    assert!(dot.contains("->") || dot.contains("label"), "{dot}");
    // Et c'est bien le graphe qu'on regarde : la case « la fonction
    // seule » change ce qui est écrit.
    assert_eq!(
        dot,
        app.flow_dot().expect("le même graphe qu'à l'écran"),
        "le fichier et l'écran doivent montrer le même graphe"
    );
}

/// Le graphe dit ce que chaque bloc fait, quand il le sait.
///
/// namsdec expliquait chaque instruction et jamais un ENSEMBLE : ce que
/// fait une boucle, ce qu'une suite de `cmp`/`je` vérifie au total.
///
/// La règle du module est qu'une explication fausse est pire qu'une
/// absence d'explication. Ce contrôle vérifie donc les deux moitiés :
/// que le graphe DIT quelque chose là où c'est certain, et qu'il se TAIT
/// sur ce qu'il ne sait pas.
#[test]
fn the_flow_says_what_a_block_does_when_it_knows() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    for _ in 0..40 {
        app.do_step();
    }
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 5);
    let écrit: Vec<&str> = lignes.iter().map(|(t, _)| t.as_str()).collect();

    // Le programme du banc écrit son message : le bloc qui le fait doit
    // le dire, parce que son numéro d'appel est posé juste avant.
    assert!(
        écrit.iter().any(|t| t.contains("demande au noyau : write")),
        "le graphe nomme l'appel système dont le numéro est dans le bloc — {écrit:?}"
    );
    // Et sa boucle est une boucle, qui a tourné.
    assert!(
        écrit.iter().any(|t| t.starts_with("boucle")),
        "le graphe reconnaît la boucle — {écrit:?}"
    );

    // Ce qu'il ne sait pas, il ne le dit pas : aucune phrase ne peut
    // apparaître pour un bloc dont les instructions ne se résument pas.
    // On le vérifie sur le module, qui est la seule porte d'entrée.
    assert_eq!(
        namsdec_re::explain::phrase(
            &namsdec_re::explain::Bloc {
                instructions: &["add rax,rcx".to_owned()],
                exit: namsdec_re::cfg::Exit::Fallthrough,
                boucle: false,
                passages: Some(3),
            },
            true
        ),
        None,
        "un bloc dont on ne sait rien dire n'a pas de phrase, même s'il a tourné"
    );
}

/// Le pseudo-code se voit dans le déroulement, et remplace le graphe.
///
/// Deux lectures de la même fonction : sa forme, ou son texte. L'une ne
/// remplace pas l'autre chez personne, d'où une case et non un choix
/// imposé — mais quand la case est cochée, c'est bien le texte qu'on
/// voit, et non un graphe avec du texte à côté.
#[test]
fn the_flow_window_can_show_the_pseudo_code() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Une boucle : `mov ecx, 3` puis un `dec`/`jnz` qui remonte. C'est
    // la forme la plus courante, et celle qui doit se dire « répéter ».
    let source = "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                  boucle:\n    dec ecx\n    jnz boucle\n    hlt\n";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        show_flow: true,
        flow_pseudo: true,
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
    let peint: Vec<&String> = lignes.iter().map(|(t, _)| t).collect();
    assert!(
        peint.iter().any(|t| t.contains("répéter")),
        "la boucle se dit « répéter » : {peint:?}"
    );
    assert!(
        peint.iter().any(|t| t.contains("tant que")),
        "et sa condition se lit : {peint:?}"
    );
    // Le corps de la boucle, dit en clair et non en assembleur.
    assert!(
        peint.iter().any(|t| t.contains("ECX = ECX")),
        "le corps est traduit : {peint:?}"
    );
}

/// Et la case décochée rend le graphe.
///
/// Sans quoi le pseudo-code ne serait pas une vue de plus mais un
/// remplacement — et le graphe, lui, montre ce qu'aucun texte ne montre :
/// la forme d'ensemble d'une fonction qu'on découvre.
#[test]
fn unchecking_the_box_gives_the_graph_back() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let source = "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                  boucle:\n    dec ecx\n    jnz boucle\n    hlt\n";
    let mesure = |pseudo: bool| -> Vec<String> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            show_flow: true,
            flow_pseudo: pseudo,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
        lignes.into_iter().map(|(t, _)| t).collect()
    };
    let graphe = mesure(false);
    assert!(
        graphe.iter().any(|t| t.contains("dec ecx")),
        "le graphe montre les instructions telles quelles : {graphe:?}"
    );
    assert!(
        !graphe.iter().any(|t| t.contains("répéter")),
        "et aucun mot de structure : {graphe:?}"
    );
}

/// Le choix de la vue se retrouve à la séance suivante.
///
/// C'est la règle du dépôt : un réglage qui se voit dans l'interface
/// part dans `settings.json` à l'instant où on le règle. Celui-ci tient
/// à la personne — on lit un graphe pour repérer, un pseudo-code pour
/// comprendre — et non au binaire qu'on ouvre.
#[test]
fn the_chosen_flow_view_survives_the_session() {
    let app = NamsdecApp {
        flow_pseudo: true,
        ..Default::default()
    };
    let réglages = app.settings_snapshot();
    assert!(réglages.flow_pseudo, "le choix part dans les réglages");
    let relu = NamsdecApp::from_settings(réglages);
    assert!(relu.flow_pseudo, "et il en revient");
}

/// Une commande qui n'agirait sur rien est grisée, et dit pourquoi.
///
/// Le pseudo-code est du TEXTE. Le zoom, lui, agit sur un dessin : les
/// cinq commandes d'échelle de la fenêtre du déroulement — moins, plus,
/// « Taille réelle », « Tout voir », et le pourcentage — n'y feraient
/// rien du tout. Elles restaient pourtant actives, à l'identique, dans
/// les deux vues : le relevé de la barre donnait exactement la même
/// liste de part et d'autre de la case.
///
/// Une commande qui ne fait rien ment, et c'est le genre de mensonge
/// qu'on ne signale pas — on croit avoir mal cliqué. Grisées plutôt
/// qu'effacées : une commande qui disparaît laisse croire qu'elle
/// n'existe pas, une commande grise qui dit pourquoi enseigne l'état du
/// programme. C'est la règle déjà posée pour « Fermer le fichier ».
///
/// Le contrôle lit le source, parce que c'est là qu'est la règle : les
/// commandes d'échelle vivent sous un `add_enabled_ui` qui dépend de la
/// vue, et chacune porte sa propre explication — une seule pour tout le
/// bloc passerait au vert avec le défaut remis.
#[test]
fn the_scale_commands_are_greyed_when_there_is_nothing_to_scale() {
    let source = include_str!("../flow.rs");
    let lignes: Vec<&str> = source.lines().collect();
    let garde = lignes
        .iter()
        .position(|l| l.contains("ui.add_enabled_ui(dessin,"))
        .expect("les commandes d'échelle sont sous une garde qui suit la vue");
    // Les cinq commandes sont dedans, et chacune dit pourquoi elle est
    // grise. La fenêtre de vingt-cinq lignes suffit : au-delà, c'est
    // qu'elles n'y sont plus.
    let bloc: Vec<&str> = lignes.iter().skip(garde).take(45).copied().collect();
    for commande in [
        "\"−\"",
        "\"+\"",
        "s.flow_zoom_reset",
        "s.flow_fit",
        "self.flow_zoom * 100.0",
    ] {
        assert!(
            bloc.iter().any(|l| l.contains(commande)),
            "{commande} doit être sous la garde :\n{}",
            bloc.join("\n")
        );
    }
    let explications = bloc
        .iter()
        .filter(|l| l.contains("on_disabled_hover_text"))
        .count();
    assert_eq!(
        explications,
        5,
        "chacune des cinq dit pourquoi elle est grise, et non une pour \
         tout le bloc :\n{}",
        bloc.join("\n")
    );
}

/// Et ce qu'on regarde peut s'emporter, quelle que soit la vue.
///
/// Les deux boutons d'export nomment le graphe dans leur libellé : ils
/// ne mentent pas, mais ils ne rendent pas ce qu'on a sous les yeux
/// quand c'est le texte qu'on lit. Le troisième bouton n'apparaît donc
/// que là où il a un sens.
#[test]
fn the_pseudo_code_can_be_copied_when_it_is_what_we_read() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let source = "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                  boucle:\n    dec ecx\n    jnz boucle\n    hlt\n";
    let barre = |pseudo: bool| -> Vec<String> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            show_flow: true,
            flow_pseudo: pseudo,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
        lignes.into_iter().map(|(t, _)| t).collect()
    };
    let s = strings(Lang::Fr);
    let texte = barre(true);
    assert!(
        texte.iter().any(|t| t == s.flow_copy_pseudo),
        "le bouton manque là où il sert : {texte:?}"
    );
    let graphe = barre(false);
    assert!(
        !graphe.iter().any(|t| t == s.flow_copy_pseudo),
        "et il ne traîne pas là où il n'a rien à copier : {graphe:?}"
    );
    // L'aide au glisser suit la même règle : elle décrit un geste qui
    // n'existe pas sur du texte.
    assert!(
        graphe.iter().any(|t| t == s.flow_pan_hint),
        "l'aide au glisser est là pour le dessin : {graphe:?}"
    );
    assert!(
        !texte.iter().any(|t| t == s.flow_pan_hint),
        "et pas pour le texte : {texte:?}"
    );
}

/// Un texte sans structure le dit, au lieu de laisser chercher.
///
/// Une fonction sans saut rend un pseudo-code qui n'a rien d'un
/// pseudo-code : les instructions, dites en clair, et c'est tout. Sans
/// un mot, on cherche le « si » qui n'y est pas et l'on croit la vue en
/// panne.
///
/// L'état vide écrit d'abord pour ce cas ne pouvait pas s'afficher : il
/// demandait un graphe vide, que la fenêtre écarte trente lignes plus
/// haut. Un état vide qu'on n'atteint jamais est un message qu'on
/// croit avoir écrit.
#[test]
fn a_pseudo_code_without_structure_says_so() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let peint = |source: &str| -> Vec<String> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            show_flow: true,
            flow_pseudo: true,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
        lignes.into_iter().map(|(t, _)| t).collect()
    };
    // Sans saut : la mention est là.
    let plat = peint("section .text\nglobal _start\n_start:\n    nop\n    nop\n    hlt\n");
    assert!(
        plat.iter().any(|t| t.contains("aucun saut")),
        "le texte sans structure le dit : {plat:?}"
    );
    // Avec une boucle : elle n'a plus lieu d'être.
    let structuré = peint(
        "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
         boucle:\n    dec ecx\n    jnz boucle\n    hlt\n",
    );
    assert!(
        structuré.iter().any(|t| t.contains("répéter")),
        "la boucle est reconnue : {structuré:?}"
    );
    assert!(
        !structuré.iter().any(|t| *t == s.flow_pseudo_flat),
        "et la mention ne traîne pas : {structuré:?}"
    );
}

/// L'indentation du pseudo-code se VOIT, et les lignes ne se marchent
/// pas dessus.
///
/// Un texte structuré dont les crans ne se distinguent pas ne vaut pas
/// mieux qu'un listing : c'est l'indentation qui dit ce qui dépend de
/// quoi, et c'est elle qu'on lit avant les mots. Elle est posée en
/// mesurant quatre espaces de la police à chasse fixe — deviner une
/// largeur de caractère marche jusqu'au premier changement de police ou
/// de zoom.
#[test]
fn the_pseudo_code_indentation_can_be_seen() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Trois niveaux : la boucle, le test dedans, et son corps.
    let source = "section .text\nglobal _start\n_start:\n    mov rcx, 5\n\
                  boucle:\n    cmp rcx, 2\n    je suivant\n    add rax, rcx\n\
                  suivant:\n    dec rcx\n    jnz boucle\n    hlt\n";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        show_flow: true,
        flow_pseudo: true,
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);

    let où = |quoi: &str| -> egui::Rect {
        lignes
            .iter()
            .find(|(t, _)| t.contains(quoi))
            .map(|(_, r)| *r)
            .unwrap_or_else(|| {
                panic!(
                    "« {quoi} » est peint : {:?}",
                    lignes.iter().map(|(t, _)| t).collect::<Vec<_>>()
                )
            })
    };
    let boucle = où("répéter");
    let test = où("si ");
    let corps = où("RAX = RAX +");
    // Chaque niveau rentre par rapport au précédent, et d'un cran qui
    // se voit : sous dix pixels, on ne le distingue pas.
    assert!(
        test.left() > boucle.left() + 10.0,
        "le test est en retrait de la boucle : {} vs {}",
        test.left(),
        boucle.left()
    );
    assert!(
        corps.left() > test.left() + 10.0,
        "et son corps l'est encore : {} vs {}",
        corps.left(),
        test.left()
    );
    // Les crans sont RÉGULIERS : un retrait qui varie se lit comme une
    // erreur de mise en page, non comme une structure.
    let premier = test.left() - boucle.left();
    let second = corps.left() - test.left();
    assert!(
        (premier - second).abs() < 1.0,
        "les deux crans doivent être égaux : {premier} puis {second}"
    );
}

/// Le déroulement dit ce qu'il sait de la FONCTION, pas seulement de
/// ses blocs.
///
/// Chaque boîte porte déjà une phrase quand namsdec sait la dire. Il
/// manquait le cran au-dessus : devant une fonction sans symboles, la
/// première question n'est pas « que fait ce bloc » mais « que prend-elle
/// et que rend-elle ». La convention System V répond pour les
/// arguments, et les retours répondent pour la valeur — à condition que
/// TOUS les chemins s'accordent.
///
/// La phrase ne paraît que sur une FONCTION : « lit RDI » n'a aucun sens
/// devant un programme entier, qui en contient plusieurs.
#[test]
fn the_flow_window_says_what_the_whole_function_does() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Une fonction qui compare ses deux premiers arguments et rend 0
    // ou 1 : le squelette de toute vérification de mot de passe.
    // Elle finit par `ret` et non par `hlt` : une fonction rend la
    // main, elle n'arrête pas la machine — et c'est le `ret` qui dit
    // qu'une valeur est rendue.
    let source = "section .text\nglobal _start\n_start:\n\
                  \x20   cmp rdi, rsi\n    jne different\n\
                  \x20   mov eax, 1\n    ret\ndifferent:\n    xor eax, eax\n    ret\n";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
    let peint: Vec<&String> = lignes.iter().map(|(t, _)| t).collect();
    assert!(
        peint.iter().any(|t| t.contains("RDI et RSI")),
        "les arguments lus se disent : {peint:?}"
    );
    assert!(
        peint.iter().any(|t| t.contains("0 ou 1")),
        "et ce que la fonction rend : {peint:?}"
    );
}

/// Et il se tait quand il n'aurait que le banal à dire.
///
/// « Contient une boucle » devant un dessin où la boucle se voit
/// n'apprend rien, et une phrase de trop fait cesser de lire celles qui
/// apprennent quelque chose.
#[test]
fn the_flow_window_keeps_quiet_when_the_drawing_already_says_it() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let source = "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                  boucle:\n    dec ecx\n    jnz boucle\n    hlt\n";
    let mut app = NamsdecApp {
        source: source.to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.do_assemble();
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("flow_window"), 4);
    let peint: Vec<&String> = lignes.iter().map(|(t, _)| t).collect();
    assert!(
        !peint.iter().any(|t| t.contains("contient une boucle")),
        "le dessin le montre déjà : {peint:?}"
    );
}

/// La case « Pseudo-code » fait ce qu'elle annonce — au CLIC, et non
/// par un état posé à la main.
///
/// Tous les contrôles d'interface de ce fichier fabriquent l'état —
/// `flow_pseudo: true` — puis mesurent ce qui est peint. Ils vérifient
/// donc que la VUE est juste, jamais que le GESTE y mène : on peut
/// débrancher la case sans qu'un seul d'entre eux rougisse.
///
/// C'est ce qu'`egui_kittest` apporte, et la seule raison de le garder.
/// Évalué le 30 septembre 2026 contre le banc maison : à longueur
/// égale, il ne raccourcit rien — le mandat disait qu'alors il sortait.
/// Mais il clique, et le banc ne sait pas le faire. Les captures non
/// plus : sous Wayland, les frappes et les clics simulés n'atteignent
/// pas l'application, ce qui a été vérifié deux fois.
///
/// L'emploi est donc borné : kittest pour les GESTES, le banc pour la
/// mise en page. Mélanger les deux ferait perdre au banc ce qu'il a de
/// mieux — il mesure des pixels, là où l'arbre d'accessibilité ne
/// connaît que des noms.
#[test]
fn clicking_the_pseudo_code_box_really_switches_the_view() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        source: "section .text\nglobal _start\n_start:\n    mov ecx, 3\n\
                 boucle:\n    dec ecx\n    jnz boucle\n    hlt\n"
            .to_owned(),
        show_flow: true,
        ..Default::default()
    };
    app.do_assemble();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui(move |ui| app.draw(ui));
    harness.run();

    assert!(
        harness.query_by_label_contains("répéter").is_none(),
        "au départ, c'est le graphe qu'on voit"
    );
    harness.get_by_label(s.flow_pseudo).click();
    harness.run();
    assert!(
        harness.query_by_label_contains("répéter").is_some(),
        "cocher la case montre le pseudo-code"
    );
    // Et le chemin du retour : une bascule qui ne rebascule pas est une
    // porte à sens unique, ce qui ne se voit qu'en essayant.
    harness.get_by_label(s.flow_pseudo).click();
    harness.run();
    assert!(
        harness.query_by_label_contains("répéter").is_none(),
        "décocher la rend au graphe"
    );
}
