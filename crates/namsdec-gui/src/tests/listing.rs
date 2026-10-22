//! Le listing : désassemblage, étiquettes, octets, correctifs.
//!
//! Ce qu'on lit d'un binaire et ce qu'on y écrit — les octets d'une
//! instruction, l'éditeur hexadécimal, un saut inversé, une instruction
//! réécrite —, plus les diagnostics de l'assembleur.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// Une saisie qui n'est pas une adresse laisse le vidage là où il
/// est. Avant, la moindre frappe de trop — `0x40_1000`, un `g` qui
/// traîne, un champ à moitié effacé — renvoyait l'affichage au début
/// du programme sans un mot d'explication.
#[test]
fn an_unparsable_address_keeps_the_one_on_screen() {
    assert_eq!(validated_address(0x401000, "0x400500", None), 0x400500);
    assert_eq!(
        validated_address(0x401000, "400500", None),
        0x400500,
        "le 0x est facultatif"
    );
    assert_eq!(validated_address(0x401000, "truc", None), 0x401000);
    assert_eq!(validated_address(0x401000, "0x40_1000", None), 0x401000);
    assert_eq!(validated_address(0x401000, "", None), 0x401000);
}

/// Un assemblage qui échoue amène ses diagnostics sous les yeux :
/// le panneau bas s'ouvre sur l'onglet qui les porte. La barre de
/// statut dit « Erreur d'assemblage » — encore faut-il pouvoir lire
/// laquelle. Et quand l'erreur est corrigée, la place revient à la
/// console, d'où repart l'exécution.
#[test]
fn a_failed_assembly_brings_up_its_diagnostics() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    mov rax, pasunregistre\n".to_owned(),
        show_console: false,
        ..Default::default()
    };

    app.do_assemble();
    assert!(
        matches!(app.status, Status::AssembleFailed),
        "{:?}",
        app.status
    );
    assert!(app.show_console, "le panneau bas doit s'ouvrir");
    assert_eq!(app.console_tab, ConsoleTab::Diagnostics);

    app.source = "global _start\nsection .text\n_start:\n    hlt\n".to_owned();
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    assert_eq!(
        app.console_tab,
        ConsoleTab::Output,
        "plus de diagnostic : la console reprend la place"
    );
}

/// Ouvrir un binaire doit mener quelque part : un vrai programme
/// passé par nasm, relu comme des octets bruts, doit remplir
/// l'éditeur d'un listing, donner la correspondance ligne <-> adresse
/// dont dépend la gouttière, et produire un émulateur capable
/// d'avancer.
#[test]
fn opening_a_raw_binary_disassembles_and_boots_the_emulator() {
    let Some(_nasm) = namsdec_asm::find_nasm() else {
        // Pas de nasm ici : ce test vérifie un aller-retour à travers
        // l'assembleur réel, il n'a rien à prouver sans lui.
        return;
    };
    let output = namsdec_asm::assemble(
        "BITS 64\nsection .text\n    mov rax, 7\n    hlt\n",
        BASE_ADDR,
    )
    .expect("le programme de test doit s'assembler");

    let mut app = NamsdecApp::default();
    app.load_binary(output.bytes, "programme");

    assert!(
        app.source.contains("mov"),
        "le listing désassemblé doit remplir l'éditeur, obtenu : {:?}",
        app.source
    );
    assert_eq!(
        app.current_file, None,
        "le tampon est reconstruit : pas de fichier à réécrire dessus"
    );

    let state = app.assemble_state.as_ref().expect("un état chargé");
    assert_eq!(
        state.output.line_to_addr.get(&1).copied(),
        Some(BASE_ADDR),
        "la première ligne du listing est la première instruction"
    );
    assert_eq!(
        app.current_line(),
        Some(1),
        "RIP au départ doit se refléter sur la ligne 1 de la gouttière"
    );

    app.do_step();
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax),
        7,
        "le binaire chargé doit s'exécuter pas à pas comme un programme assemblé"
    );
}

/// Un fichier plus gros que la mémoire simulée est tronqué plutôt que
/// rejeté — et le statut le dit, au lieu de laisser croire que tout a
/// été chargé.
#[test]
fn oversized_binaries_are_truncated_not_rejected() {
    let mut app = NamsdecApp::default();
    app.load_binary(vec![0x90; MEM_SIZE + 1024], "programme"); // que des `nop`

    match app.status {
        Status::BinaryLoaded { bytes, truncated } => {
            assert_eq!(bytes, MEM_SIZE);
            assert!(truncated);
        }
        _ => panic!("un binaire trop gros doit tout de même se charger"),
    }
}

/// LE LISTING NOMME CE QU'UN APPEL CALCULÉ VISE.
///
/// `call qword [rip+0x2f3a]` ne porte pas sa destination : elle est
/// dans une case mémoire, et le désassembleur s'arrête là. Sur les
/// seize binaires d'épreuve, relevé le 20 octobre 2026, **un
/// branchement sur dix est dans ce cas** — 16 260 contre 147 301
/// directs. Le graphe d'appels y perdait l'arête, la découverte de
/// fonctions la cible, et la ligne à l'écran ne disait rien.
///
/// Le contrôle part d'un vrai binaire et vérifie deux choses : qu'au
/// moins un appel calculé porte désormais un nom, et que ce nom est
/// celui d'une fonction que le binaire emprunte vraiment — non une
/// invention.
#[test]
fn the_listing_names_what_a_computed_call_targets() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    if !app.load_elf(&octets, "ls") {
        return;
    }
    let empruntées: std::collections::BTreeSet<String> = namsdec_re::imports::scan(&octets)
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert!(!empruntées.is_empty(), "/bin/ls emprunte des fonctions");

    // Les lignes du listing qui portent un `call` calculé ET un nom.
    let nommées: Vec<&str> = app
        .source
        .lines()
        .filter(|l| l.contains("call") && l.contains('[') && l.contains("; -> "))
        .collect();
    assert!(
        !nommées.is_empty(),
        "aucun appel calculé n'est nommé — le listing ne dit rien de ce qu'ils visent"
    );
    // Et ce qui est nommé l'est JUSTEMENT : au moins une de ces lignes
    // doit nommer une fonction que le binaire emprunte pour de vrai.
    let juste = nommées.iter().any(|ligne| {
        ligne
            .rsplit("; -> ")
            .next()
            .is_some_and(|nom| empruntées.contains(nom.trim()))
    });
    assert!(
        juste,
        "les noms doivent venir de la table de liaison, pas d'une invention : {:?}",
        &nommées[..nommées.len().min(5)]
    );
}

/// L'onglet Octets montre ce que l'assembleur a produit : une ligne par
/// instruction, son adresse, sa longueur, ses octets — et la ligne source
/// d'où elle vient.
///
/// C'est là qu'on voit, sans qu'on ait à l'expliquer, ce que coûte ce
/// qu'on écrit — et ce que nasm choisit parfois pour vous : `mov rax, 1`
/// devient `mov eax, 1` et tient en cinq octets, puisque le haut de RAX
/// sera mis à zéro de toute façon. Il faut un négatif pour voir la forme
/// à sept octets, et une grande constante pour celle à dix.
#[test]
fn the_bytes_tab_shows_what_each_line_produced() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Bytes,
        source: "global _start\nsection .text\n_start:\n\
             \x20   mov rax, 1\n    mov rax, -1\n\
             \x20   mov rax, 0x1122334455667788\n    xor eax, eax\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1400.0),
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

    // Les octets eux-mêmes.
    assert!(
        painted.contains("b8 01 00 00 00"),
        "mov rax, 1 tient en cinq octets : {painted}"
    );
    assert!(painted.contains("48 c7 c0 ff ff ff ff"), "{painted}");
    assert!(
        painted.contains("48 b8 88 77 66 55 44 33 22 11"),
        "{painted}"
    );
    assert!(painted.contains("31 c0"), "{painted}");
    // Les longueurs, qui sont tout l'intérêt : 5, 7, 10, 2.
    for longueur in [" 5|", " 7|", "10|", " 2|"] {
        assert!(
            painted.contains(longueur),
            "longueur {longueur} : {painted}"
        );
    }
    // Et le lien avec la source : la ligne de chaque instruction, collée
    // à son adresse.
    assert!(painted.contains("  4 00400000"), "{painted}");
}

/// Cliquer un diagnostic pose le curseur sur sa ligne : lire le message
/// et retrouver l'endroit ne sont pas deux gestes.
#[test]
fn clicking_a_diagnostic_moves_the_cursor_to_its_line() {
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    hlt\n".to_owned(),
        ..Default::default()
    };
    // Comme si l'on avait cliqué le diagnostic de la ligne 3.
    app.goto_line = Some(3);

    let ctx = egui::Context::default();
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

    assert!(app.goto_line.is_none(), "la demande est consommée");
    // Le curseur est celui de l'APPLICATION : en mode Normal, le champ
    // de texte n'est pas dessiné, et c'est elle qui tient la position
    // (voir `vue_listing`).
    // « global _start\n » = 14 octets, « section .text\n » = 14.
    assert_eq!(app.cursor_byte, 28, "le curseur est au début de la ligne 3");
    assert_eq!(app.cursor_pos, (3, 1), "et la barre d'état le dit");
}

/// La ligne fautive se voit dans l'éditeur, là où l'on écrit — pas
/// seulement dans la console.
#[test]
fn the_offending_line_is_marked_in_the_editor() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    let attendu = namsdec_theme::by_id("vscode-dark").bad.gamma_multiply(0.18);
    let bandes = |source: &str| -> Vec<(f32, egui::Color32)> {
        let mut app = NamsdecApp {
            source: source.to_owned(),
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
        let mut trouvées = Vec::new();
        for _ in 0..2 {
            trouvées.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Rect(r) = &clipped.shape {
                    // Le bandeau d'une ligne fautive : sa teinte le
                    // distingue de tout le reste.
                    if r.fill == attendu {
                        trouvées.push((r.rect.min.y, r.fill));
                    }
                }
            }
            output.textures_delta.clear();
        }
        trouvées
    };

    let propre = "global _start\nsection .text\n_start:\n    hlt\n";
    assert!(
        bandes(propre).is_empty(),
        "un programme correct ne porte aucune marque"
    );

    let fautif = "global _start\nsection .text\n_start:\n    mov rax, ebx\n    hlt\n";
    let marquées = bandes(fautif);
    assert_eq!(marquées.len(), 1, "une seule ligne fautive : {marquées:?}");
}

/// Une adresse qu'aucune étiquette ne couvre reste un nombre : on ne
/// rattache pas `_start+4000` à une étiquette qui s'arrête bien avant.
#[test]
fn an_address_past_its_label_is_not_named() {
    use namsdec_asm::elf::Symbol;
    let symbols = vec![
        Symbol {
            name: "tampon".to_owned(),
            addr: 0x401000,
            size: 4,
        },
        Symbol {
            name: "suite".to_owned(),
            addr: 0x401010,
            size: 0,
        },
    ];
    assert_eq!(symbol_at(&symbols, 0x401000).as_deref(), Some("tampon"));
    assert_eq!(symbol_at(&symbols, 0x401002).as_deref(), Some("tampon+2"));
    // Au-delà de sa taille déclarée, l'étiquette ne dit plus rien.
    assert_eq!(symbol_at(&symbols, 0x401004), None);
    // Avant la première, non plus.
    assert_eq!(symbol_at(&symbols, 0x400fff), None);
    // Sans taille déclarée, une étiquette couvre jusqu'à la suivante —
    // ici il n'y en a pas, donc son seul octet.
    assert_eq!(symbol_at(&symbols, 0x401010).as_deref(), Some("suite"));
    assert_eq!(symbol_at(&symbols, 0x401011), None);
}

/// Ouvrir un binaire en donne un listing structuré : les fonctions
/// portent leur nom, et chaque saut dit où il va.
#[test]
fn an_opened_binary_gets_its_labels_back() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        eprintln!("nasm ou ld absent : test ignoré");
        return;
    }
    // Un vrai programme, assemblé et lié, puis rouvert comme un binaire
    // qu'on n'aurait pas écrit.
    let out = namsdec_asm::assemble(
        "\
global _start
section .text
_start:
call sous
hlt
sous:
mov rax, 1
ret
",
        0x400000,
    )
    .expect("assemblage");

    let mut app = NamsdecApp::default();
    assert!(
        app.load_elf(&out.file, "programme"),
        "l'ELF doit se charger"
    );

    // Les étiquettes du source sont revenues dans le listing.
    assert!(
        app.source.contains("_start:"),
        "le point d'entrée porte son nom : {}",
        app.source
    );
    assert!(
        app.source.contains("sous:"),
        "la sous-routine aussi : {}",
        app.source
    );
    // Et l'appel dit qui il appelle.
    assert!(
        app.source.contains("; -> sous"),
        "le `call` doit nommer sa cible : {}",
        app.source
    );

    // La gouttière reste juste : la ligne marquée pendant l'exécution
    // est celle de l'instruction, pas celle d'une étiquette.
    let state = app.assemble_state.as_ref().unwrap();
    let ligne_entree = state.addr_to_line[&out.entry];
    let texte_ligne = app
        .source
        .lines()
        .nth(ligne_entree as usize - 1)
        .expect("la ligne existe");
    assert!(
        texte_ligne.starts_with("call"),
        "la ligne du point d'entrée porte son instruction, pas l'étiquette : {texte_ligne:?}"
    );
}

/// F12 sur une étiquette mène à l'endroit où elle est définie.
#[test]
fn going_to_a_label_lands_on_its_definition() {
    let source = "\
global _start
section .text
_start:
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    hlt
";
    // La fonction pure, d'abord : c'est elle qui décide.
    assert_eq!(label_line(source, "boucle"), Some(5));
    assert_eq!(label_line(source, "_start"), Some(3));
    // Ce qui n'est pas une définition n'en est pas une : le `boucle` du
    // `jnz` ne compte pas, et un nom plus long non plus.
    assert_eq!(label_line(source, "bouclette"), None);
    assert_eq!(label_line(source, "hlt"), None);

    let mut app = NamsdecApp {
        source: source.to_owned(),
        // Le curseur sur le `boucle` du `jnz`, en ligne 7.
        cursor_byte: source.find("jnz boucle").unwrap() + 4,
        ..Default::default()
    };
    app.dispatch(Action::GoToLabel);
    assert_eq!(app.goto_line, Some(5), "F12 doit mener à `boucle:`");

    // Sur un mot qui n'est pas une étiquette, l'outil le dit plutôt que
    // de ne rien faire.
    app.goto_line = None;
    app.cursor_byte = source.find("mov rcx").unwrap();
    app.dispatch(Action::GoToLabel);
    assert_eq!(app.goto_line, None);
    assert!(
        matches!(app.status, Status::Note(_)),
        "une touche qui ne répond pas laisse croire qu'elle n'existe pas : {:?}",
        app.status
    );
}

/// Les octets de l'instruction sont découpés par rôle : un préfixe n'est
/// pas un opcode, et un immédiat encore moins.
#[test]
fn the_bytes_of_an_instruction_are_split_by_role() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
mov rax, rbx
mov eax, 42
syscall
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    // Les octets sont passés sous le repli « Détail » : ils découpent
    // l'instruction par champs, ce qu'on regarde en s'arrêtant, non à
    // chaque pas — le bandeau n'avait plus la place de les montrer sans
    // couper une ligne en deux sur une fenêtre par défaut. Le repli
    // s'ouvre de lui-même devant un appel ou un saut ; devant un `mov`,
    // c'est à nous de l'ouvrir.
    app.next_detail_open = true;

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
        let mut textes = Vec::new();
        for _ in 0..2 {
            textes.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(text) = &clipped.shape {
                    textes.push(text.galley.text().to_owned());
                }
            }
            output.textures_delta.clear();
        }
        textes
    };

    // `mov rax, rbx` s'encode 48 89 d8 : le REX.W à part, l'opcode à
    // part, l'octet d'adressage à part.
    let textes = peindre(&mut app);
    assert!(textes.iter().any(|t| t == "48"), "{textes:?}");
    assert!(textes.iter().any(|t| t == "89"), "{textes:?}");
    assert!(textes.iter().any(|t| t == "d8"), "{textes:?}");
    // Et surtout : pas d'un seul tenant.
    assert!(!textes.iter().any(|t| t == "48 89 d8"), "{textes:?}");

    // `mov eax, 42` : l'immédiat forme un groupe à lui seul.
    app.do_step();
    let textes = peindre(&mut app);
    assert!(
        textes.iter().any(|t| t == "2a 00 00 00"),
        "les quatre octets de l'immédiat vont ensemble : {textes:?}"
    );
}

/// Sur un long programme, l'onglet Octets ramène la ligne courante sous
/// les yeux à chaque pas.
///
/// Il ne dessine plus que les rangées visibles — trente sur quinze
/// mille —, ce qui veut dire qu'il ne peut plus compter sur egui pour
/// faire défiler jusqu'à une rangée qu'il n'a pas dessinée : la position
/// se calcule.
#[test]
fn the_bytes_tab_follows_execution_in_a_long_program() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Bytes,
        source: format!(
            "global _start\nsection .text\n_start:\n{}    hlt\n",
            "    nop\n".repeat(400)
        ),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    for _ in 0..300 {
        app.do_step();
    }
    let rip = app.emulator.as_ref().unwrap().cpu().regs.rip;

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
    // Quelques images : le partage du panneau se cale en deux ou
    // trois, et la vue ne cesse de suivre que lorsqu'elle est arrivée.
    for _ in 0..4 {
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
        painted.contains(&format!("{rip:08x}")),
        "l'instruction courante ({rip:#x}) doit être à l'écran"
    );
    // Et la première, restée loin au-dessus, ne l'est pas : la vue a
    // bien suivi plutôt que de tout dessiner.
    assert!(
        !painted.contains("00400000"),
        "le début du programme est hors de la vue : {painted}"
    );
}

/// « Nouveau » donne un programme qui tient debout, pas une page
/// blanche : devant un assembleur, elle est un mur.
#[test]
fn a_new_program_is_ready_to_assemble() {
    let mut app = NamsdecApp {
        source: "de quoi salir le tampon".to_owned(),
        ..Default::default()
    };

    // Il y a quelque chose à perdre : on demande avant.
    app.dispatch(Action::New);
    assert_eq!(app.pending, Some(Pending::New));
    assert_eq!(app.source, "de quoi salir le tampon");

    // Une fois accordé, le squelette est posé — et il est propre.
    app.pending = None;
    app.perform(Pending::New);
    assert_eq!(app.source, NEW_PROGRAM);
    assert!(!app.buffer_modified(), "rien à enregistrer d'emblée");
    assert!(app.current_file.is_none());

    // Les trois choses qu'on n'invente pas devant une page blanche.
    for attendu in ["global _start", "section .text", "_start:"] {
        assert!(app.source.contains(attendu), "{}", app.source);
    }
    // Et il sort proprement, plutôt que de laisser le processeur
    // continuer dans ce qui suit.
    assert!(app.source.contains("mov rax, 60"), "{}", app.source);

    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        app.do_assemble();
        assert!(
            matches!(app.status, Status::AssembleOk),
            "le squelette doit s'assembler : {:?}",
            app.status
        );
        app.do_run();
        let ctx = egui::Context::default();
        for _ in 0..10 {
            if !app.running {
                break;
            }
            app.tick_run(&ctx);
        }
        assert!(
            matches!(app.status, Status::Exited { code: 0 }),
            "et rendre la main avec le code 0 : {:?}",
            app.status
        );
    }
}

/// Le désassemblage se copie, étiquettes comprises.
///
/// C'est ce qu'on colle dans un compte rendu ou un message — pas le
/// source, que l'autre a peut-être déjà.
#[test]
fn the_disassembly_can_be_taken_away() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Rien à copier tant que rien n'est assemblé.
    assert!(NamsdecApp::default().listing_text().is_none());

    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
call sous
hlt
sous:
mov rax, 1
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let listing = app.listing_text().expect("un programme assemblé");
    assert!(listing.contains("_start:"), "{listing}");
    assert!(listing.contains("sous:"), "{listing}");
    assert!(
        listing.contains("; -> sous"),
        "l'appel nomme sa cible : {listing}"
    );
    // Les octets y sont, en colonne.
    assert!(listing.contains("c3"), "le `ret` et ses octets : {listing}");
    // Et l'instruction est écrite comme on l'écrit — ici telle que nasm
    // l'a encodée : `mov rax, 1` devient `mov eax, 1`, cinq octets, le
    // haut de RAX étant mis à zéro de toute façon.
    assert!(listing.contains("mov eax, 1"), "{listing}");
}

/// Un désassembleur extérieur ne s'exécute que sur demande, et le
/// panneau dit ce qu'il va lancer.
///
/// C'est la différence avec un thème : un thème est une palette, un
/// désassembleur de ce genre est un programme. namsdec lit ce qui est
/// déclaré, l'annonce, et attend un clic.
#[test]
fn an_external_disassembler_runs_only_when_asked() {
    let dossier = tempfile::tempdir().expect("dossier temporaire");
    std::fs::write(
        dossier.path().join("cat.json"),
        r#"{"id":"cat","name":"cat","command":"cat","args":["{file}"]}"#,
    )
    .unwrap();

    let plugins = namsdec_disasm::plugin::read_dir(dossier.path());
    assert_eq!(plugins.len(), 1);

    let mut app = NamsdecApp {
        show_disassembly: true,
        disassembler_plugins: plugins,
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    app.do_assemble();
    app.disassembler = "cat".to_owned();

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
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

    // Rien n'a été lancé : le panneau annonce ce qu'il ferait.
    assert!(app.plugin_output.is_none(), "rien ne s'exécute tout seul");
    assert!(painted.contains("programme extérieur"), "{painted}");
    assert!(painted.contains("cat"), "{painted}");
    assert!(painted.contains("Lancer"), "{painted}");
}

/// Le listing d'un binaire du système nomme ce qu'il appelle.
///
/// Sans cela, lire /bin/ls revient à suivre des `call 0x…` vers des
/// adresses muettes. Les entrées de la table de liaison portent donc une
/// étiquette, comme n'importe quel symbole du programme.
#[test]
fn the_listing_labels_the_functions_a_binary_calls() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes, "programme");

    let state = app.assemble_state.as_ref().expect("chargé");
    let noms: Vec<&str> = state
        .output
        .symbols
        .iter()
        .map(|symbol| symbol.name.as_str())
        .collect();
    assert!(noms.contains(&"malloc@plt"), "{} symboles", noms.len());
    assert!(noms.contains(&"free@plt"));
    // Et elles sont dans le listing, là où l'œil les cherche.
    assert!(app.source.contains("malloc@plt:"), "l'étiquette est posée");

    // Chaque étiquette tombe sur une instruction du listing : une
    // étiquette entre deux lignes n'existerait nulle part.
    for symbol in &state.output.symbols {
        assert!(
            state.addr_to_line.contains_key(&symbol.addr),
            "{} pointe {:#x}, qui n'est aucune ligne",
            symbol.name,
            symbol.addr
        );
    }
}

/// Ouvrir un binaire montre son code, et non son en-tête décodé comme
/// des instructions.
///
/// Les premières lignes d'un ELF ouvert étaient du charabia : le
/// désassemblage partait de l'octet zéro, donc de `\x7fELF`, des tables
/// de relocation et des chaînes de symboles. C'est pourtant la première
/// chose qu'on voit en ouvrant un fichier.
#[test]
fn opening_a_binary_shows_its_code_not_its_header() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes, "programme");
    let state = app.assemble_state.as_ref().expect("chargé");

    // Le listing commence à une section exécutable, et non à l'octet
    // zéro du fichier — qui porte « \x7fELF », pas des instructions.
    let elf_entier = namsdec_re::elf_scan::scan(&state.output.file).expect("un ELF");
    let exécutables: Vec<&namsdec_re::elf_scan::SectionInfo> = elf_entier
        .sections
        .iter()
        .filter(|s| s.execute && !s.nobits && s.size > 0)
        .collect();
    let première = state.disasm.first().expect("du code").address;
    assert!(
        exécutables
            .iter()
            .any(|s| première >= s.addr && première < s.addr + s.size),
        "le listing ne commence pas dans du code : {première:#x}"
    );

    // Et tout ce qu'il montre tombe dans une section exécutable du
    // fichier — c'est la définition de « c'est du code ».
    // Les adresses sont celles que le fichier déclare, désormais : la
    // fenêtre mémoire s'adapte au programme au lieu de le tasser à
    // `BASE_ADDR`, ce qui décalait tout.
    let plages: Vec<(u64, u64)> = exécutables
        .iter()
        .map(|s| (s.addr, s.addr + s.size))
        .collect();
    for instruction in &state.disasm {
        assert!(
            plages
                .iter()
                .any(|(début, fin)| (*début..*fin).contains(&instruction.address)),
            "{:#x} n'est dans aucune section exécutable",
            instruction.address
        );
    }

    // Les entrées de liaison y sont toujours nommées.
    assert!(app.source.contains("malloc@plt:"), "les étiquettes restent");
}

/// Assembler se grise sur un listing de binaire, et revient dès qu'on
/// touche au texte.
///
/// Le refus existait déjà, mais il arrivait après le clic : on frappait
/// Assembler, on récoltait une erreur rouge, et rien ne l'avait annoncé.
/// Un bouton qui échouera à tous les coups doit se voir avant, comme
/// Exécuter et Step Into se grisent quand plus rien ne tourne.
#[test]
fn assembling_is_greyed_out_on_a_binary_listing() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(
        app.can_assemble(),
        "sur un source ordinaire, rien n'empêche d'assembler"
    );

    assert!(app.load_elf(&octets, "ls"), "l'ELF se charge");
    assert!(
        !app.can_assemble(),
        "le listing d'un binaire ouvert n'est pas un source"
    );

    // Réinitialiser ne rend pas un désassemblage assemblable : il remet
    // la machine à son point d'entrée, pas le texte à l'état de source.
    app.dispatch(Action::Reset);
    assert!(!app.can_assemble(), "Réinitialiser ne change rien à cela");

    // La porte de sortie est celle qu'annonce le message : modifier le
    // texte. La première frappe suffit.
    app.source.push_str("\n; à moi\n");
    assert!(
        app.can_assemble(),
        "dès que le texte est touché, c'est un source comme un autre"
    );

    // Et le raccourci, lui, continue de passer par le refus expliqué :
    // un geste qui ne vient pas d'un bouton visible mérite sa raison.
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "ls"));
    app.dispatch(Action::Assemble);
    assert!(
        matches!(&app.status, Status::Error(e) if e == strings(app.lang).binary_no_reassemble),
        "obtenu {:?}",
        app.status
    );
}

/// Une instruction trop longue est coupée, pas laissée à déborder.
///
/// Les boîtes avaient une largeur fixe et le texte n'était jamais
/// mesuré : « jne near 000000000001D903h » sortait de son cadre et se
/// posait sur la boîte voisine. Sur un binaire réel, le dessin devenait
/// illisible avant même d'être grand.
#[test]
fn a_long_instruction_is_cut_rather_than_spilled() {
    assert_eq!(flow_ellipsis("mov rax, rbx", 20), "mov rax, rbx");
    // Pile la largeur : rien à couper.
    assert_eq!(flow_ellipsis("mov rax, rbx", 12), "mov rax, rbx");
    // Un caractère de trop : on coupe, et l'on montre qu'on a coupé.
    let coupé = flow_ellipsis("jne near 000000000001D903h", 12);
    assert_eq!(coupé.chars().count(), 12, "la coupe tient dans la boîte");
    assert!(coupé.ends_with('…'), "et se voit : {coupé}");
    assert!(coupé.starts_with("jne near"), "{coupé}");
    // Les caractères accentués comptent pour un, pas pour leurs octets.
    assert_eq!(flow_ellipsis("éééééé", 3), "éé…");
}

/// Choisir un octet dans l'éditeur ne fait pas enfler la fenêtre.
///
/// L'inspecteur qui s'ouvre sous l'octet choisi était une `egui::Grid`,
/// et toute rangée imbriquée — grille ou `ui.horizontal` — réserve la
/// largeur disponible. Dans une fenêtre qui s'ajuste à son contenu,
/// c'est une boucle : la fenêtre grandit pour contenir la rangée, ce
/// qui agrandit la disponible, et l'on repart. On mesurait 1100 px,
/// puis 2878, puis deux de plus à chaque image — la fenêtre, centrée,
/// finissait par déborder de l'écran et le contenu se coupait à gauche.
/// Ce test choisit un octet et vérifie que la fenêtre ne bouge plus.
#[test]
fn choisir_un_octet_ne_fait_pas_enfler_la_fenetre() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    app.show_analysis = true;
    app.analysis_module = "hex_editor".to_owned();

    let scr = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1100.0));
    let rendre = |app: &mut NamsdecApp, ev: Vec<egui::Event>| -> Option<egui::Rect> {
        let mut r7f = None;
        let mut sortie = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(scr),
                events: ev,
                ..Default::default()
            },
            |ui| app.draw(ui),
        );
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape
                && texte.galley.text() == "7f"
                && r7f.is_none()
            {
                r7f = Some(egui::Rect::from_min_size(texte.pos, texte.galley.size()));
            }
        }
        sortie.textures_delta.clear();
        r7f
    };
    let largeur = |ctx: &egui::Context| {
        egui::AreaState::load(ctx, egui::Id::new("analysis_window"))
            .map(|a| a.rect().width())
            .expect("la fenêtre est affichée")
    };

    // On laisse la fenêtre se poser, et l'on note où est le premier octet
    // (0x7f, le début du nombre magique de l'ELF) pour le viser.
    let mut cible = None;
    for _ in 0..4 {
        if let Some(r) = rendre(&mut app, vec![]) {
            cible = Some(r);
        }
    }
    let avant = largeur(&ctx);
    let pos = cible.expect("un octet 0x7f est affiché").center();

    for ev in [
        egui::Event::PointerMoved(pos),
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
    ] {
        rendre(&mut app, vec![ev]);
    }
    // Plusieurs images après le clic : si la boucle existait, la fenêtre
    // aurait déjà bondi d'un bon millier de pixels.
    for _ in 0..6 {
        rendre(&mut app, vec![]);
    }
    let après = largeur(&ctx);
    assert!(
        après <= avant + 2.0,
        "la fenêtre a enflé en choisissant un octet : {avant} px puis {après} px"
    );
}

/// Patcher un binaire : inverser un saut, et le retrouver dans le
/// fichier exporté.
///
/// C'est le geste qui manquait le plus. On ouvre un crackme, on voit le
/// `je` qu'il faudrait inverser, et l'on ne pouvait rien en faire :
/// réassembler le listing est refusé — à juste titre, un désassemblage
/// n'est pas un source.
#[test]
fn a_jump_can_be_inverted_and_the_change_reaches_the_exported_file() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");

    // La ligne d'un saut conditionnel : on la cherche dans le listing,
    // comme on la chercherait des yeux.
    let état = app.assemble_state.as_ref().expect("chargé");
    let (adresse, ligne, avant) = état
        .disasm
        .iter()
        .find_map(|instruction| {
            let court = instruction
                .bytes
                .first()
                .is_some_and(|o| (0x70..=0x7f).contains(o));
            let ligne = état.addr_to_line.get(&instruction.address)?;
            court.then(|| (instruction.address, *ligne, instruction.bytes.clone()))
        })
        .expect("un crackme a bien un saut conditionnel");
    app.cursor_pos = (ligne as usize, 1);

    // Ce que le geste ferait, dit avant de cliquer.
    let (vu, origine, nouveaux) = app
        .patch_preview(PatchGeste::Inverser)
        .expect("l'inversion est possible sur cette ligne");
    assert_eq!(vu, adresse);
    assert_eq!(origine, avant);
    assert_eq!(nouveaux[0], avant[0] ^ 1, "un bit, et un seul");
    assert_eq!(nouveaux.len(), avant.len(), "la longueur ne bouge pas");

    // Appliqué : le binaire est rechargé avec les octets changés.
    app.apply_patch(PatchGeste::Inverser);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    assert_eq!(app.patches.len(), 1);

    let patché = &app.assemble_state.as_ref().expect("rechargé").output.file;
    let offset = app.patches[0].offset;
    assert_eq!(
        &patché[offset..offset + nouveaux.len()],
        nouveaux.as_slice(),
        "les octets changés sont dans l'image"
    );
    assert_ne!(patché[offset], octets[offset], "et ils ont bien changé");

    // Et le fichier exporté les porte : c'est tout l'intérêt.
    let dossier = tempfile::tempdir().unwrap();
    let chemin = dossier.path().join("patché");
    app.export_binary(&chemin);
    let écrit = std::fs::read(&chemin).expect("le binaire exporté");
    assert_eq!(écrit.len(), octets.len(), "même taille, à l'octet près");
    assert_eq!(&écrit[offset..offset + nouveaux.len()], nouveaux.as_slice());

    // Défait : on retrouve le fichier d'origine, octet pour octet.
    app.undo_patch(0);
    assert!(app.patches.is_empty());
    let rendu = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(
        &rendu[offset..offset + avant.len()],
        avant.as_slice(),
        "un patch qu'on ne peut pas défaire n'est pas un outil d'analyse"
    );
}

/// Neutraliser une instruction garde la longueur du fichier.
///
/// C'est la règle qui rend l'opération sûre : un octet de plus ou de
/// moins décalerait tout ce qui suit, et chaque adresse du fichier
/// deviendrait fausse.
#[test]
fn neutralising_an_instruction_keeps_every_other_address() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");

    let état = app.assemble_state.as_ref().expect("chargé");
    let (ligne, longueur) = état
        .disasm
        .iter()
        .find_map(|instruction| {
            let ligne = état.addr_to_line.get(&instruction.address)?;
            (instruction.bytes.len() >= 3).then_some((*ligne, instruction.bytes.len()))
        })
        .expect("une instruction d'au moins trois octets");
    app.cursor_pos = (ligne as usize, 1);

    app.apply_patch(PatchGeste::Neutraliser);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);

    let patché = app
        .assemble_state
        .as_ref()
        .expect("rechargé")
        .output
        .file
        .clone();
    assert_eq!(patché.len(), octets.len(), "le fichier garde sa taille");
    let offset = app.patches[0].offset;
    assert_eq!(
        &patché[offset..offset + longueur],
        vec![0x90u8; longueur].as_slice(),
        "autant de nop que d'octets remplacés"
    );
    // Et rien d'autre n'a bougé : c'est ce qui garantit que toutes les
    // autres adresses restent justes.
    let différents = patché
        .iter()
        .zip(octets.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(différents, longueur, "seuls ces octets-là ont changé");
}

/// La fenêtre des patchs dit ce qu'elle ferait avant qu'on clique.
#[test]
fn the_patch_window_says_what_it_would_do_before_one_clicks() {
    let s = strings(Lang::Fr);
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    app.show_patches = true;

    let peint = paint(&mut app);
    assert!(peint.contains(s.patch_title), "{peint}");
    assert!(peint.contains(s.patch_invert), "{peint}");
    assert!(peint.contains(s.patch_neutralise), "{peint}");
    assert!(
        peint.contains(s.patch_none_yet),
        "et l'on n'a encore rien changé :\n{peint}"
    );
}

/// Le programme assemblé sort de l'atelier : un ELF exécutable, celui
/// que la ligne de commande aurait donné.
///
/// Un atelier qui ne laisse rien sortir apprend à se servir de lui, pas
/// de l'assembleur. Les deux commandes lancées sont affichées avec, pour
/// pouvoir refaire le travail sans namsdec.
#[test]
fn the_assembled_program_can_leave_the_workbench() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let chemin = dir.path().join("programme");

    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
    mov rax, 60
    xor rdi, rdi
    syscall
"
        .to_owned(),
        ..Default::default()
    };
    // Rien d'assemblé : rien à écrire, et le message le dit.
    app.export_binary(&chemin);
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
    assert!(!chemin.exists());

    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.export_binary(&chemin);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);

    // Un vrai ELF, et exécutable.
    let octets = std::fs::read(&chemin).unwrap();
    assert!(octets.starts_with(b"\x7fELF"), "un ELF");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&chemin).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "exécutable : {mode:o}");
    }
    // Et il tourne pour de vrai, avec le code qu'on lui a écrit.
    let sortie = lance_ce_qui_vient_d_etre_ecrit(&chemin);
    assert_eq!(sortie.code(), Some(0));

    // Les deux commandes sont dites, et elles sont celles qu'on
    // taperait.
    let commandes = namsdec_asm::build_commands("programme.asm", "programme", BASE_ADDR);
    assert!(commandes[0].contains("nasm -f elf64"));
    assert!(commandes[1].contains(&format!("-Ttext {BASE_ADDR:#x}")));
    app.console_tab = ConsoleTab::Diagnostics;
    app.show_console = true;
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).assemble_commands),
        "{painted}"
    );
    assert!(painted.contains("nasm -f elf64"), "{painted}");
}

/// Ouvrir un fichier objet — un `.o` — se lit et s'analyse, mais ne
/// s'exécute pas, et namsdec dit pourquoi.
///
/// Il s'en plaignait jusqu'ici en parlant de mémoire : « le binaire
/// dépasse la fenêtre simulée », devant un fichier de 560 octets. La
/// vraie raison est que `ld` n'est pas passé — aucune section n'a
/// d'adresse, il n'y a pas de point d'entrée.
#[test]
fn an_unlinked_object_reads_but_does_not_run() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let asm = dir.path().join("t.asm");
    let objet = dir.path().join("t.o");
    std::fs::write(
        &asm,
        "global _start\nsection .text\n_start:\n    mov rax, 60\n    xor rdi, rdi\n    syscall\n",
    )
    .unwrap();
    let ok = std::process::Command::new("nasm")
        .args(["-f", "elf64", "-o"])
        .arg(&objet)
        .arg(&asm)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return;
    }

    let mut app = NamsdecApp::default();
    app.load_binary(std::fs::read(&objet).unwrap(), "objet.o");

    // Il se lit : le désassemblage est là, avec l'étiquette.
    let state = app.assemble_state.as_ref().expect("chargé");
    assert!(!state.disasm.is_empty(), "le code est désassemblé");
    assert!(app.source.contains("_start:"), "{}", app.source);
    assert!(state.output.symbols.iter().any(|s| s.name == "_start"));

    // Il ne s'exécute pas, et le message dit pourquoi et quoi faire.
    assert!(app.emulator.is_none(), "rien à exécuter");
    let Status::Error(message) = &app.status else {
        panic!("attendu une explication, obtenu {:?}", app.status);
    };
    assert!(message.contains("ld -Ttext"), "{message}");
    assert!(
        !message.contains("mémoire"),
        "la mémoire n'y est pour rien : {message}"
    );
}

/// Le panneau de désassemblage se cherche et suit l'exécution.
#[test]
fn the_disassembly_panel_searches_and_follows() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        show_disassembly: true,
        source: "\
global _start
section .text
_start:
mov rax, 1
mov rbx, 2
call sous
hlt
sous:
mov rcx, 3
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

    // Sans recherche, tout est là — en pseudo-code, et sur les
    // registres que nasm a réellement encodés : `mov rax, 1` tient en
    // cinq octets sous la forme `mov eax, 1`.
    let painted = peindre(&mut app);
    assert!(painted.contains("EAX = 1"), "{painted}");
    assert!(painted.contains("ECX = 3"), "{painted}");
    assert!(
        painted.contains("appeler"),
        "l'appel se dit en clair : {painted}"
    );

    // Une recherche filtre.
    app.disassembly_query = "ECX".to_owned();
    let painted = peindre(&mut app);
    assert!(painted.contains("ECX = 3"), "{painted}");
    // Le compte des lignes retenues, et la disparition de ce qui ne
    // correspond pas. (`EAX = 1` reste visible ailleurs : le bandeau du
    // panneau latéral dit l'instruction courante, filtre ou pas.)
    assert!(painted.contains("1 / 6"), "{painted}");
    assert!(
        !painted.contains("appeler"),
        "le `call` ne correspond pas à la recherche : {painted}"
    );
    // Et les octets sont là, à côté : c'est ce qui distingue un
    // désassemblage d'une traduction.
    assert!(painted.contains("b9 03 00 00 00"), "{painted}");

    // Chercher une étiquette la trouve, bien qu'elle ne soit pas dans le
    // texte de l'instruction.
    app.disassembly_query = "sous".to_owned();
    let painted = peindre(&mut app);
    assert!(painted.contains("sous:"), "{painted}");
}

/// Le moteur choisi revient à la session suivante, et le panneau sait
/// aller à une fonction sans qu'on fasse défiler.
#[test]
fn the_disassembly_panel_remembers_and_navigates() {
    // Le réglage voyage comme les autres.
    let app = NamsdecApp {
        disassembler: NASM_ID.to_owned(),
        ..Default::default()
    };
    let json = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(json.contains("\"disassembler\":\"nasm\""), "{json}");
    let relu: crate::settings::Settings = serde_json::from_str(&json).unwrap();
    assert_eq!(NamsdecApp::from_settings(relu).disassembler, NASM_ID);

    // Un fichier d'une version plus ancienne n'a pas le champ : on
    // revient au pseudo-code, celui qu'on lit en premier.
    let ancien: crate::settings::Settings =
        serde_json::from_str(r#"{"version":1,"theme":"vscode-dark","language":"fr"}"#).unwrap();
    assert_eq!(ancien.disassembler, PSEUDO_ID);

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Et la liste des étiquettes mène où on lui demande.
    let mut app = NamsdecApp {
        show_disassembly: true,
        source: format!(
            "global _start\nsection .text\n_start:\n{}call sous\nhlt\nsous:\nret\n",
            "nop\n".repeat(200)
        ),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let sous = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|s| s.name == "sous")
        .expect("l'étiquette existe")
        .addr;

    app.disassembly_target = Some(sous);
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1000.0),
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
        painted.contains("sous:"),
        "la cible doit être à l'écran : {painted}"
    );
    assert!(
        painted.contains("revenir à l'appelant"),
        "et ce qu'elle contient : {painted}"
    );
}

/// Lire du code « en Texte » doit s'expliquer, pas laisser deviner.
///
/// À `_start`, les octets sont `b8 01 00 00 00` : deux illisibles, puis
/// un zéro qui termine la chaîne. On lisait donc « <b8><01> » et « \0 »,
/// sans un mot pour dire que ce n'était pas du texte, que le zéro
/// arrêtait la lecture, ni où se trouvait le message qu'on cherchait.
#[test]
fn reading_code_as_text_says_why_it_looks_like_nothing() {
    let mut mem = namsdec_core::Memory::new(BASE_ADDR, 0x1000);
    // `mov eax, 1`, puis de quoi remplir, puis un vrai message.
    mem.load(BASE_ADDR, b"\xb8\x01\x00\x00\x00").unwrap();
    mem.load(BASE_ADDR + 0x20, b"salut, monde\n\0").unwrap();

    let lu = read_memory(&mem, BASE_ADDR, MemoryFormat::Text, "hors bornes", Lang::Fr);
    // Ce que `write` enverrait : deux octets, montrés tels quels.
    assert!(lu.contains("<b8><01>"), "{lu}");
    assert!(
        lu.contains("2 octets, puis un zéro"),
        "le zéro qui arrête la lecture doit être nommé : {lu}"
    );
    assert!(
        lu.contains("ne forment pas du texte"),
        "et l'on doit dire que ce n'en est pas : {lu}"
    );
    // Et le message qui suit, avec son adresse : c'est la question
    // suivante, et elle n'avait pas de réponse.
    assert!(
        lu.contains(&format!("{:#010x}", BASE_ADDR + 0x20)) && lu.contains("salut, monde"),
        "le texte lisible de la fenêtre doit être listé : {lu}"
    );

    // Sur du vrai texte, rien de tout cela : la chaîne, et c'est tout.
    let lu = read_memory(
        &mem,
        BASE_ADDR + 0x20,
        MemoryFormat::Text,
        "hors bornes",
        Lang::Fr,
    );
    assert!(lu.contains("salut, monde"), "{lu}");
    assert!(
        !lu.contains("ne forment pas du texte"),
        "ici c'en est, et il ne faut pas dire le contraire : {lu}"
    );
    assert!(
        !lu.contains("Texte lisible dans les octets qui suivent"),
        "et rien à chercher ailleurs quand on est déjà dessus : {lu}"
    );

    // Un premier octet nul : la chaîne est vide, et c'est une réponse.
    let lu = read_memory(
        &mem,
        BASE_ADDR + 0x100,
        MemoryFormat::Text,
        "hors bornes",
        Lang::Fr,
    );
    assert!(lu.contains("Chaîne vide"), "{lu}");
}

/// L'ÉDITEUR BINAIRE ÉCRIT VRAIMENT, sur un binaire ouvert.
///
/// Signalé le 21 octobre : « dans l'éditeur binaire on ne sait rien
/// modifier ». Le contrôle part d'un vrai binaire, écrit un octet par
/// le chemin que le bouton « Écrire » emprunte, et regarde trois
/// choses : que l'octet a changé, que le correctif est retenu — donc
/// défaisable —, et que le reste du fichier n'a pas bougé.
#[test]
fn the_binary_editor_really_writes_a_byte() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    if !app.load_elf(&octets, "ls") {
        return;
    }
    // Un octet du code, pas de l'en-tête : changer `e_ident` ferait
    // refuser le rechargement, et l'on mesurerait ce refus au lieu de
    // l'écriture.
    let (offset, avant) = {
        let state = app.assemble_state.as_ref().expect("un état");
        let scan = namsdec_re::elf_scan::scan(&state.output.file).expect("un ELF");
        let texte = scan
            .sections
            .iter()
            .find(|s| s.name == ".text" && s.size > 16)
            .expect("une section de code");
        let at = texte.offset as usize + 8;
        (at, state.output.file[at])
    };
    let neuf = avant ^ 0xff;

    app.write_patch(offset, vec![neuf], 0);

    let state = app.assemble_state.as_ref().expect("un état après écriture");
    assert_eq!(
        state.output.file[offset], neuf,
        "l'octet doit avoir changé — c'est tout l'objet de l'éditeur"
    );
    assert_eq!(
        state.output.file.len(),
        octets.len(),
        "et le fichier garde sa taille exacte"
    );
    // Le correctif est retenu : sans cela, rien ne se défait.
    assert_eq!(app.patches.len(), 1, "le correctif est inscrit");
    assert_eq!(app.patches[0].before, vec![avant]);
    assert_eq!(app.patches[0].after, vec![neuf]);
    // Et le reste du fichier n'a pas bougé d'un octet.
    let changés = state
        .output
        .file
        .iter()
        .zip(&octets)
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(changés, 1, "un seul octet change, et c'est celui-là");

    // Écrire la MÊME valeur ne crée pas de correctif : une liste de
    // choses à défaire qui n'ont rien fait n'aide personne.
    app.write_patch(offset, vec![neuf], 0);
    assert_eq!(app.patches.len(), 1, "rien de neuf, rien d'inscrit");
}

/// L'ÉDITEUR BINAIRE SUR LE FICHIER QUI A SERVI À LE SIGNALER.
///
/// Un essai de reproduction, et il vise un fichier NOMMÉ plutôt qu'un
/// binaire du système : « /home/fred/CrackMe/crackme2 ». S'il n'est pas
/// là, le contrôle s'abstient — mais il a servi à chercher.
#[test]
fn the_binary_editor_on_the_reported_file() {
    let Ok(octets) = std::fs::read("/home/fred/CrackMe/crackme2") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "crackme2"), "ce binaire s'ouvre");
    let state = app.assemble_state.as_ref().expect("un état");
    println!(
        "crackme2 : {} octets dans output.file, {} instructions, from_binary {}",
        state.output.file.len(),
        state.disasm.len(),
        state.from_binary.is_some()
    );
    let offset = 0x1000usize.min(state.output.file.len().saturating_sub(1));
    let avant = state.output.file[offset];
    // Et l'octet ZÉRO, celui du nombre magique : l'écrire rend le
    // fichier illisible, et le correctif était alors jeté EN SILENCE.
    let mut muet = NamsdecApp::default();
    assert!(muet.load_elf(&octets, "crackme2"));
    muet.write_patch(0, vec![0x42], 0);
    println!(
        "octet 0 : statut {:?}, correctifs {}",
        muet.status,
        muet.patches.len()
    );
    app.write_patch(offset, vec![avant ^ 0xff], 0);
    let state = app.assemble_state.as_ref().expect("un état");
    assert_eq!(
        state.output.file[offset],
        avant ^ 0xff,
        "le bouton « Écrire » change l'octet, sur ce fichier-là aussi"
    );
    assert_eq!(app.patches.len(), 1);
}

/// LA GRILLE PREND-ELLE LE CLAVIER QUAND L'ÉDITEUR L'A ?
///
/// Le parcours de la capture : l'éditeur en mode INSERTION — donc son
/// champ de texte a le clavier, et le garde —, la fenêtre d'analyse
/// ouverte sur l'éditeur binaire, et l'on clique un octet pour taper
/// dedans. La grille n'écoute que si AUCUN champ n'a le focus.
///
/// C'est l'application ENTIÈRE qui est dessinée ici, et non le panneau
/// seul : mesuré seul, il n'y a aucun champ de texte dans l'image, et
/// la question ne se pose pas. C'est précisément ce qui rendait les
/// mesures précédentes rassurantes et vaines.
#[test]
fn the_hex_grid_takes_the_keyboard_from_the_editor() {
    let Ok(octets) = std::fs::read("/home/fred/CrackMe/crackme2") else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    if !app.load_elf(&octets, "crackme2") {
        return;
    }
    // L'ÉMULATEUR, sans quoi la fenêtre d'analyse ne dessine AUCUN
    // panneau : `re_modules_ui` s'arrête sur son état vide. Le premier
    // essai l'avait oublié et concluait « reproduit » sur un panneau
    // qui n'existait pas.
    app.rebuild_emulator();
    assert!(app.emulator.is_some(), "l'émulateur est là");
    app.show_re_tab = true;
    app.show_analysis = true;
    app.analysis_module = "hex_editor".to_owned();
    app.editor_mode = EditorMode::Insert;

    let image = |app: &mut NamsdecApp, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        output.textures_delta.clear();
    };

    // L'éditeur prend le clavier, comme après un `i`.
    image(&mut app, vec![]);
    ctx.memory_mut(|m| m.request_focus(editor_id()));
    image(&mut app, vec![]);
    println!("focus avant le clic : {:?}", ctx.memory(|m| m.focused()));

    // Où une cellule hexadécimale est peinte, DANS la fenêtre d'analyse.
    let mut cible = None;
    for _ in 0..2 {
        cible = None;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| app.draw(ui));
        // DANS LA FENÊTRE D'ANALYSE, et nulle part ailleurs : l'onglet
        // « Octets » du panneau latéral et le listing peignent eux
        // aussi des paires hexadécimales, et le premier relevé cliquait
        // dans le panneau latéral en croyant cliquer dans la grille.
        let fenêtre = egui::AreaState::load(&ctx, egui::Id::new("analysis_window"))
            .and_then(|a| a.rect().into());
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let texte = t.galley.text();
                if texte.len() == 2
                    && texte.chars().all(|c| c.is_ascii_hexdigit())
                    && cible.is_none()
                {
                    let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                    let vu = rect.intersect(clipped.clip_rect);
                    let dedans = fenêtre.is_none_or(|f: egui::Rect| f.contains(vu.center()));
                    if vu.is_positive() && dedans {
                        cible = Some(vu.center());
                    }
                }
            }
        }
        output.textures_delta.clear();
    }
    let Some(cible) = cible else {
        println!("aucune cellule peinte — rien à mesurer");
        return;
    };

    for pressé in [true, false] {
        let mut events = Vec::new();
        if pressé {
            events.push(egui::Event::PointerMoved(cible));
        }
        events.push(egui::Event::PointerButton {
            pos: cible,
            button: egui::PointerButton::Primary,
            pressed: pressé,
            modifiers: egui::Modifiers::NONE,
        });
        image(&mut app, events);
    }
    assert!(
        ctx.memory(|m| m.focused()).is_none(),
        "cliquer un octet rend le clavier à la grille — egui s'en charge"
    );

    let avant = app
        .assemble_state
        .as_ref()
        .map(|s| s.output.file.clone())
        .unwrap_or_default();
    for key in [egui::Key::Num4, egui::Key::Num2] {
        image(
            &mut app,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    // Une image de plus : la demande du panneau est relevée par
    // l'application APRÈS le dessin de la fenêtre, donc elle s'applique
    // à l'image suivante.
    image(&mut app, vec![]);
    let après = app
        .assemble_state
        .as_ref()
        .map(|s| s.output.file.clone())
        .unwrap_or_default();
    let changés = avant.iter().zip(&après).filter(|(a, b)| a != b).count();

    // La grille a bien reçu la frappe et demandé l'écriture — c'était
    // mesuré pas à pas. Mais l'octet visé est le ZÉRO, celui du `7f` du
    // nombre magique : l'écrire rend le fichier illisible, `load_elf`
    // refuse de le relire, et le correctif n'est pas posé.
    //
    // C'EST JUSTE, ET CELA DOIT SE DIRE. Avant le 21 octobre 2026, on
    // sortait de `write_patch` sans rien changer et SANS RIEN DIRE :
    // l'octet reprenait sa valeur à l'image suivante, et l'on tapait
    // dans le vide. « Dans l'éditeur binaire on ne sait rien
    // modifier. »
    assert_eq!(
        changés, 0,
        "l'en-tête n'est pas écrit : namsdec ne saurait plus lire le fichier"
    );
    let s = strings(app.lang);
    match &app.status {
        Status::Error(message) => assert_eq!(
            message, s.patch_breaks_file,
            "le refus se dit, et il dit POURQUOI"
        ),
        autre => panic!(
            "un correctif refusé doit le dire — c'est tout le grief du \
             21 octobre. Statut : {autre:?}"
        ),
    }
}

/// Mais pas sur un listing venu d'un binaire.
///
/// Celui-ci est reconstruit, et y insérer une ligne décalerait toutes les
/// correspondances ligne <-> adresse dont dépendent la gouttière, les
/// points d'arrêt et le suivi de RIP.
#[test]
fn a_binary_listing_does_not_duplicate_lines() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ));
    let avant = app.source.clone();
    app.cursor_byte = 40;
    app.duplicate_line();
    assert_eq!(app.source, avant, "le listing ne doit pas bouger");
}

/// Un désassembleur maison déclaré devient un moteur de listing.
///
/// C'est tout le point du format : DZF est écrit à la main, dans un autre
/// dépôt, en C++ sans dépendance — et namsdec doit pouvoir s'en servir
/// sans rien savoir de lui d'autre que la commande à lancer.
#[test]
fn a_home_made_disassembler_can_become_the_listing_engine() {
    let fiche = data_dir()
        .map(|dir| dir.join("disassemblers/dzf.json"))
        .filter(|p| p.is_file());
    // DZF vit ailleurs : sur une machine qui ne l'a pas déclaré, il n'y a
    // rien à vérifier, et le test le dit plutôt que d'échouer.
    let Some(fiche) = fiche else { return };

    let plugin = namsdec_disasm::plugin::read(&fiche).expect("la fiche déclarée doit être lisible");
    assert_eq!(plugin.id, "dzf");
    // Le programme qu'elle nomme doit exister, sans quoi la déclarer
    // n'aurait servi qu'à promettre.
    let programme = std::path::Path::new(&plugin.command);
    if !programme.is_file() {
        return;
    }
    // Et il rend un listing sur les octets d'une vraie fonction.
    let octets: &[u8] = &[0x55, 0x48, 0x89, 0xe5, 0x5d, 0xc3];
    let listing = plugin.run(octets).expect("DZF doit répondre");
    for attendu in ["push rbp", "mov rbp, rsp", "pop rbp", "ret"] {
        assert!(
            listing.contains(attendu),
            "« {attendu} » manque :\n{listing}"
        );
    }
}

/// Un diagnostic déjà affiché suit la langue quand on la change.
///
/// Ils gardaient celle où ils étaient nés : basculer en anglais laissait
/// du français dans l'interface, et les retraduire aurait demandé de
/// réassembler. Ceux qui viennent de namsdec portent maintenant leur
/// motif et non leur phrase, laquelle se reforme au dessin.
#[test]
fn a_diagnostic_already_shown_follows_the_language() {
    // Un programme dont l'exécution sort par la fin : le lint le dit.
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rax, 1
"
        .to_owned(),
        lang: Lang::Fr,
        ..Default::default()
    };
    app.do_assemble();
    let avec_motif: Vec<_> = app
        .diagnostics
        .iter()
        .filter(|d| d.kind.is_some())
        .collect();
    // Sans nasm sur la machine, rien n'a été assemblé : le test se tait.
    if avec_motif.is_empty() {
        return;
    }

    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let peindre = |app: &mut NamsdecApp| -> String {
        app.console_tab = ConsoleTab::Diagnostics;
        let mut peint = String::new();
        for _ in 0..2 {
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

    let en_français = peindre(&mut app);
    assert!(
        en_français.contains("l'exécution peut sortir"),
        "le diagnostic doit se lire en français :\n{en_français}"
    );
    let _ = s;

    // On bascule, **sans réassembler** : c'est tout le point.
    app.lang = Lang::En;
    let en_anglais = peindre(&mut app);
    assert!(
        en_anglais.contains("execution can run off the end"),
        "et se relire en anglais sans réassembler :\n{en_anglais}"
    );
    assert!(
        !en_anglais.contains("l'exécution peut sortir"),
        "sans laisser du français derrière :\n{en_anglais}"
    );
}

/// Les messages de nasm, eux, ne se retraduisent pas — et c'est dit.
///
/// Ils arrivent rédigés, en anglais. Les réécrire demanderait de
/// comprendre ce que nasm a voulu dire, ce qui n'est pas le travail de
/// namsdec : il les montre tels quels, et explique à côté quand il sait.
#[test]
fn a_message_from_nasm_carries_no_pattern() {
    let venu_de_nasm = namsdec_asm::Diagnostic {
        line: 3,
        severity: namsdec_asm::Severity::Error,
        message: "symbol `foo' undefined".to_owned(),
        kind: None,
    };
    assert!(
        venu_de_nasm.kind.is_none(),
        "un message rédigé ailleurs n'a pas de motif à reformer"
    );
}

/// Nommer une adresse, et retrouver ce nom au lancement suivant.
///
/// Le geste central de la rétro-ingénierie : on passe une soirée à
/// comprendre que `0x1d491` vérifie le mot de passe, et sans cela il n'y
/// a plus que `0x1d491` le lendemain.
#[test]
fn a_name_given_to_an_address_comes_back_later() {
    let dossier = tempfile::tempdir().unwrap();
    let octets: &[u8] = b"\x7fELF un binaire d'essai";
    let empreinte = notes::fingerprint(octets);

    let mut notes = notes::Notes::default();
    notes.set_name(0x1d491, "vérifie_mdp");
    notes.set_comment(0x1d4a0, "compare octet par octet");
    notes::save(dossier.path(), &empreinte, &notes).unwrap();

    // Le même contenu, sous un autre nom de fichier : les notes suivent.
    let relues = notes::load(dossier.path(), &notes::fingerprint(octets));
    assert_eq!(relues.name(0x1d491), Some("vérifie_mdp"));
    assert_eq!(relues.comment(0x1d4a0), Some("compare octet par octet"));

    // Un binaire recompilé, lui, n'hérite de rien : ses notes
    // désigneraient d'autres instructions, ce qui est pire que rien.
    let recompilé = notes::load(dossier.path(), &notes::fingerprint(b"\x7fELF autre chose"));
    assert!(
        recompilé.is_empty(),
        "un autre contenu ne doit pas hériter des notes"
    );
}

/// Le nom posé à la main l'emporte sur le symbole du binaire.
///
/// C'est toute la raison de l'avoir écrit : quelqu'un qui renomme
/// `sub_1d491` en `vérifie_mdp` veut voir le second.
#[test]
fn a_hand_written_name_wins_over_the_symbol() {
    let mut app = NamsdecApp {
        source: "\
section .text
global _start
_start:
    mov rax, 1
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    let Some(état) = app.assemble_state.as_ref() else {
        return;
    };
    let Some(start) = état
        .output
        .symbols
        .iter()
        .find(|symbole| symbole.name == "_start")
        .map(|symbole| symbole.addr)
    else {
        return;
    };

    let symboles = app
        .assemble_state
        .as_ref()
        .map(|état| état.output.symbols.clone())
        .unwrap_or_default();

    // Le symbole, d'abord.
    assert_eq!(
        named_at(&app.notes, &symboles, start).as_deref(),
        Some("_start")
    );

    // Puis le nom posé.
    app.notes.set_name(start, "le_vrai_début");
    assert_eq!(
        named_at(&app.notes, &symboles, start).as_deref(),
        Some("le_vrai_début"),
        "le nom écrit à la main doit primer"
    );

    // Retiré, on retombe sur le symbole — et non sur rien.
    app.notes.set_name(start, "");
    assert_eq!(
        named_at(&app.notes, &symboles, start).as_deref(),
        Some("_start")
    );
}

/// Nommer une ligne qui n'a pas d'adresse le dit, plutôt que d'ouvrir un
/// champ qui n'écrirait nulle part.
#[test]
fn naming_a_line_without_an_address_says_so() {
    let mut app = NamsdecApp {
        source: "; rien que du commentaire\n".to_owned(),
        ..Default::default()
    };
    app.cursor_pos = (1, 1);
    app.start_note(NoteKind::Name);
    assert!(app.note_edit.is_none(), "aucun champ ne doit s'ouvrir");
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
}

/// Les colonnes du listing ne dansent pas d'une ligne à l'autre.
///
/// La colonne des étiquettes n'avait pas de largeur : une rangée sans
/// étiquette ne réservait rien, et les octets comme le texte de
/// l'instruction reculaient de cinquante-cinq pixels dès qu'une ligne en
/// portait une. Un listing dont les colonnes bougent ne se lit plus en
/// colonnes, et c'est tout ce qu'on lui demande.
///
/// Le contrôle relève les abscisses de la colonne des octets — repérable
/// à sa forme, des paires hexadécimales séparées par des espaces — et
/// demande qu'elles soient toutes les mêmes.
#[test]
fn the_listing_keeps_its_columns_from_line_to_line() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        show_disassembly: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("disassembly_window"), 5);

    // La colonne des octets : du texte qui n'est que des chiffres
    // hexadécimaux et des espaces, et qui en contient au moins quatre.
    let mut bords: Vec<(String, f32)> = Vec::new();
    for (texte, place) in &lignes {
        let net = texte.trim_end();
        if net.len() >= 4
            && net.chars().all(|c| c.is_ascii_hexdigit() || c == ' ')
            && net.contains(' ')
        {
            bords.push((net.to_owned(), place.left()));
        }
    }
    assert!(
        bords.len() >= 4,
        "il faut plusieurs lignes d'octets pour que le test prouve quelque \
         chose — {bords:?}"
    );
    let premier = bords[0].1;
    let hors: Vec<String> = bords
        .iter()
        .filter(|(_, x)| (x - premier).abs() > 1.0)
        .map(|(t, x)| format!("« {t} » à {x:.1}"))
        .collect();
    assert!(
        hors.is_empty(),
        "la colonne des octets commence à {premier:.1}, sauf {}",
        hors.join(", ")
    );
}

/// Les octets d'une instruction se récrivent là où on les lit.
///
/// La vue mémoire se modifiait déjà ; l'onglet Octets, non — alors que
/// c'est là qu'on lit ce qu'on voudrait changer. On voyait le `je` qu'il
/// faudrait inverser et il fallait passer par une autre fenêtre.
///
/// Le même chemin que les trois gestes du patcheur : la fenêtre
/// Correctifs les liste et sait les défaire. Écrire à la main n'est pas
/// un autre mécanisme, c'est la même chose dite autrement.
///
/// LA LONGUEUR NE CHANGE PAS, et c'est refusé plutôt que rattrapé :
/// récrire une instruction plus courte ou plus longue déplacerait tout
/// ce qui suit, donc toutes les adresses, donc tous les sauts.
#[test]
fn the_bytes_of_an_instruction_can_be_rewritten_where_they_are_read() {
    // Ce que la saisie accepte, et ce qu'elle refuse.
    assert_eq!(crate::lire_octets("48 31 c0"), Some(vec![0x48, 0x31, 0xc0]));
    assert_eq!(
        crate::lire_octets("4831c0"),
        Some(vec![0x48, 0x31, 0xc0]),
        "on recopie ce qu'on lit ailleurs, et ailleurs n'a pas toujours d'espaces"
    );
    assert_eq!(crate::lire_octets("48,31,c0"), Some(vec![0x48, 0x31, 0xc0]));
    assert_eq!(crate::lire_octets(""), None, "rien n'est pas zéro octet");
    assert_eq!(
        crate::lire_octets("48 3"),
        None,
        "un nombre impair de chiffres ne dit pas ce qu'on voulait"
    );
    assert_eq!(
        crate::lire_octets("48 zz"),
        None,
        "et « zz » n'est pas un octet"
    );

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    let Some(state) = &app.assemble_state else {
        panic!("le programme du banc s'assemble")
    };
    let instruction = state.disasm[1].clone();
    let adresse = instruction.address;
    let avant = instruction.bytes.clone();

    // Une longueur différente est refusée, et le dit.
    app.write_bytes_at(adresse, "90");
    assert!(
        matches!(&app.status, Status::Error(_)),
        "récrire avec un autre nombre d'octets est refusé"
    );
    let après_refus = app.assemble_state.as_ref().unwrap().disasm[1].bytes.clone();
    assert_eq!(après_refus, avant, "et rien n'a bougé");

    // La même longueur passe, et le changement se voit là où on l'a
    // écrit — puis dans la fenêtre Correctifs, qui sait le défaire.
    let offset = namsdec_re::imports::file_offset(
        &app.assemble_state.as_ref().unwrap().output.file,
        adresse,
    )
    .expect("l'adresse a une place dans le fichier") as usize;
    let nops: String = vec!["90"; avant.len()].join(" ");
    app.write_bytes_at(adresse, &nops);
    // Le contrôle porte sur le FICHIER et non sur le désassemblage :
    // cinq octets remplacés par cinq `nop` ne font plus une instruction
    // de cinq octets mais cinq d'un octet, ce qui est exactement ce
    // qu'on voulait et pas ce qu'un test naïf attend.
    let fichier = &app.assemble_state.as_ref().expect("rechargé").output.file;
    assert_eq!(
        &fichier[offset..offset + avant.len()],
        vec![0x90; avant.len()].as_slice(),
        "les octets écrits sont ceux qui sont dans le fichier"
    );
    assert_eq!(
        app.patches.len(),
        1,
        "et le changement est dans la liste de ce qu'on peut défaire"
    );

    // Défait, on retrouve exactement ce qu'il y avait.
    app.undo_patch(0);
    let fichier = &app.assemble_state.as_ref().unwrap().output.file;
    assert_eq!(
        &fichier[offset..offset + avant.len()],
        avant.as_slice(),
        "défaire rend les octets d'origine"
    );
    assert!(app.patches.is_empty());
}

/// Les numéros de la gouttière suivent les lignes source, une pour une.
///
/// Le numéro `i` est celui de la i-ème ligne POSÉE (`galley.rows`), et non
/// de la i-ème ligne du texte. Les deux coïncident tant qu'aucune ligne ne
/// se replie — c'est pourquoi le code pose le texte avec une largeur de
/// repli infinie. Qu'on la rende finie, et une ligne trop longue
/// deviendrait deux rangées pour un seul numéro : tous les numéros
/// d'après glisseraient d'un cran sous leur ligne, et la ligne courante
/// s'éclairerait à côté de l'instruction qu'elle désigne.
///
/// On met donc une ligne volontairement très longue en tête, et l'on
/// vérifie que les numéros peints restent consécutifs, à pas constant.
#[test]
fn les_numeros_de_la_gouttiere_suivent_les_lignes_source() {
    let ctx = egui::Context::default();
    // Une ligne de commentaire longue à déborder : à largeur de repli
    // finie, elle couvrirait plusieurs rangées.
    let longue = format!("    ; {}", "x".repeat(400));
    let source = format!(
        "global _start\nsection .text\n_start:\n{longue}\n    mov rax, 60\n    xor edi, edi\n    syscall\n"
    );
    let lignes_source = source.lines().count();
    let mut app = NamsdecApp {
        source,
        ..Default::default()
    };
    app.apply_theme(&ctx);

    let (_, peint) = ce_qui_est_écrit(&ctx, &mut app, None, 3);
    // Les numéros de la gouttière : des entiers peints tout à gauche.
    let mut numéros: Vec<(u32, f32)> = peint
        .iter()
        .filter(|(_, r)| r.left() < 80.0)
        .filter_map(|(txt, r)| txt.trim().parse::<u32>().ok().map(|n| (n, r.center().y)))
        .collect();
    numéros.sort_by(|a, b| a.1.total_cmp(&b.1));
    numéros.dedup();

    assert!(
        numéros.len() >= 5,
        "la gouttière devrait numéroter au moins cinq lignes : {numéros:?}"
    );
    // Rangés par hauteur, les numéros se suivent de 1 en 1 : aucune rangée
    // sans numéro (une ligne repliée) ne s'est glissée entre deux.
    for paire in numéros.windows(2) {
        assert_eq!(
            paire[1].0,
            paire[0].0 + 1,
            "les numéros sautent — un repli a décalé la numérotation : {numéros:?}"
        );
    }
    // Le premier numéro visible est 1 : on voit le haut du fichier.
    assert_eq!(numéros.first().unwrap().0, 1, "{numéros:?}");
    // Et le dernier numéro ne dépasse pas le nombre de lignes source : pas
    // de rangée fantôme en trop.
    assert!(
        numéros.last().unwrap().0 as usize <= lignes_source + 1,
        "un numéro dépasse le nombre de lignes source ({lignes_source}) : {numéros:?}"
    );
    // Le pas en hauteur est constant : chaque numéro est une rangée sous
    // le précédent, jamais deux (ce que ferait une ligne repliée).
    let pas: Vec<f32> = numéros.windows(2).map(|p| p[1].1 - p[0].1).collect();
    let pas0 = pas[0];
    for p in &pas {
        assert!(
            (p - pas0).abs() < 1.0,
            "le pas entre numéros varie — une ligne s'est repliée : {pas:?}"
        );
    }
}

/// Le JSON de DZF, livré au panneau, s'y affiche en pseudo-C.
///
/// C'est le câblage de bout en bout : l'application reçoit la sortie du
/// décompilateur (ici fournie à la main, en attendant que DZF la produise
/// vraiment), `ingest_dzf_json` la lit et la livre au module, qui la rend.
#[test]
fn le_json_de_dzf_livre_au_panneau_s_y_affiche() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_analysis: true,
        analysis_module: "dzf_decompiler".to_owned(),
        ..Default::default()
    };
    // Un binaire chargé d'abord (la fenêtre d'analyse en a besoin), PUIS la
    // livraison — sans quoi `on_binary_loaded` l'effacerait.
    app.do_assemble();
    let json =
        r#"{"functions":[{"address":4096,"name":"demo","pseudocode":"int demo() { return 1; }"}]}"#;
    app.dzf_wanted = true;
    app.ingest_dzf_json(json);

    let peint = paint(&mut app);
    assert!(
        peint.contains("demo"),
        "le nom de la fonction décompilée :\n{peint}"
    );
    assert!(
        peint.contains("int demo") || peint.contains("return 1"),
        "le pseudo-code :\n{peint}"
    );
}

/// Demandée depuis le panneau DZF, une sortie qui n'est pas du JSON le dit
/// — un clic ne reste pas sans réponse.
#[test]
fn une_sortie_non_dzf_demandee_le_dit() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: DEFAULT_SOURCE.to_owned(),
        show_analysis: true,
        analysis_module: "dzf_decompiler".to_owned(),
        lang: Lang::Fr,
        ..Default::default()
    };
    app.do_assemble();
    app.dzf_wanted = true;
    // Du pseudo-code de rizin, pas du JSON.
    app.ingest_dzf_json("int main() {\n  return 0;\n}");
    let peint = paint(&mut app);
    assert!(
        peint.contains("pas rendu de JSON DZF"),
        "l'échec doit se dire :\n{peint}"
    );
}

/// L'éditeur binaire dispose le fichier en grille hexadécimale.
///
/// Un vrai ELF, la fenêtre d'analyse ouverte sur l'éditeur : on doit y
/// lire les décalages, les octets — la magie `7f` en tête —, et la
/// colonne de texte où « ELF » se lit en clair. C'est la forme de tout
/// éditeur hexadécimal ; sans elle, il n'y a rien à éditer.
#[test]
fn l_editeur_binaire_dispose_le_fichier_en_grille() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n    mov rax, 60\n    xor edi, edi\n    syscall\n"
            .to_owned(),
        show_analysis: true,
        analysis_module: "hex_editor".to_owned(),
        system_titlebar: false,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let scr = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 1100.0));
    let mut peint = String::new();
    for _ in 0..3 {
        peint.clear();
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(scr),
                ..Default::default()
            },
            |ui| app.draw(ui),
        );
        for c in &out.shapes {
            if let egui::epaint::Shape::Text(texte) = &c.shape {
                peint.push_str(texte.galley.text());
                peint.push('|');
            }
        }
        out.textures_delta.clear();
    }
    assert!(
        peint.contains("Éditeur binaire"),
        "le titre du panneau :\n{peint}"
    );
    assert!(
        peint.contains("00000000"),
        "le décalage de la première ligne :\n{peint}"
    );
    assert!(peint.contains("7f"), "la magie ELF en tête :\n{peint}");
    assert!(
        peint.contains("ELF"),
        "la colonne de texte lit « ELF » :\n{peint}"
    );
}

/// Le désassemblage se lit aussi dans la syntaxe du système.
///
/// `mov rax, [rbx+8]` d'un côté, `movq 8(%rbx), %rax` de l'autre : les
/// deux disent la même chose et ne se ressemblent pas. Ce n'est pas un
/// détail de goût — dès qu'on sort de namsdec, un rapport d'`objdump`,
/// une session `gdb` ou un tutoriel trouvé en ligne écrivent tout dans
/// l'autre, et la traduction de tête est la marche où l'on abandonne.
///
/// C'est un moteur de plus dans la liste existante, et non un réglage
/// ailleurs : le choix se fait là où l'on choisit déjà ce qu'on lit.
#[test]
fn the_disassembly_reads_in_the_systems_syntax_too() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let peint = |moteur: &str| -> Vec<String> {
        let mut app = NamsdecApp {
            source: "section .text\nglobal _start\n_start:\n    mov rax, rbx\n    hlt\n".to_owned(),
            show_disassembly: true,
            disassembler: moteur.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let (_, lignes) = ce_qui_est_écrit(&ctx, &mut app, Some("disassembly_window"), 4);
        lignes.into_iter().map(|(t, _)| t).collect()
    };

    // Le choix est offert là où l'on choisit déjà le moteur.
    let nasm = peint("nasm");
    assert!(
        nasm.iter().any(|t| t == s.disassembly_system),
        "le moteur système doit figurer dans la liste : {nasm:?}"
    );
    assert!(
        nasm.iter().any(|t| t.contains("rax")),
        "NASM écrit les registres nus : {nasm:?}"
    );

    // Et il montre bien l'autre syntaxe.
    let système = peint("system");
    assert!(
        système.iter().any(|t| t.contains("%rax")),
        "le système préfixe les registres d'un % : {système:?}"
    );
    assert!(
        !système.iter().any(|t| t.contains("mov rax")),
        "et n'écrit plus la forme NASM : {système:?}"
    );
}

/// Le choix du moteur se retrouve à la séance suivante.
#[test]
fn the_chosen_disassembler_survives_the_session() {
    let app = NamsdecApp {
        disassembler: "system".to_owned(),
        ..Default::default()
    };
    let réglages = app.settings_snapshot();
    assert_eq!(réglages.disassembler, "system");
    assert_eq!(NamsdecApp::from_settings(réglages).disassembler, "system");
}

/// On récrit une instruction en assembleur, pas seulement en octets.
///
/// Corriger un binaire demandait de connaître l'encodage : pour
/// neutraliser un test, il fallait savoir que `nop` s'écrit `90`. C'est
/// une marche de plus, et elle n'apprend rien — l'assembleur est là pour
/// éviter d'écrire des octets à la main.
///
/// Un seul champ accepte les deux, et la règle est dite : ce qui peut se
/// lire en octets se lit en octets. Sans cet ordre, `fadd` — quatre
/// lettres qui sont aussi quatre chiffres hexadécimaux — changerait de
/// sens selon l'humeur du programme.
#[test]
fn an_instruction_can_be_rewritten_in_assembly() {
    if namsdec_asm::find_nasm().is_none() {
        return;
    }
    // L'assembleur rend bien les octets attendus, adresse comprise :
    // un saut s'écrit en relatif, et ne produit pas les mêmes octets
    // selon l'endroit d'où l'on saute.
    assert_eq!(
        namsdec_asm::assemble_one("mov eax, 1", 0x401000).expect("mov s'assemble"),
        vec![0xB8, 0x01, 0x00, 0x00, 0x00]
    );
    assert_eq!(
        namsdec_asm::assemble_one("nop", 0x401000).expect("nop s'assemble"),
        vec![0x90]
    );
    let depuis_1000 = namsdec_asm::assemble_one("jmp 0x401010", 0x401000).expect("saut");
    let depuis_2000 = namsdec_asm::assemble_one("jmp 0x401010", 0x402000).expect("saut");
    assert_ne!(
        depuis_1000, depuis_2000,
        "un saut est relatif : les octets dépendent d'où l'on part"
    );

    // Et ce qui ne s'assemble pas le dit, avec le message de nasm.
    let erreur = namsdec_asm::assemble_one("mvo eax, 1", 0x401000)
        .expect_err("un mnémonique inexistant doit être refusé");
    let namsdec_asm::AsmError::Failed { diagnostics } = &erreur else {
        panic!("l'erreur doit porter les diagnostics de nasm : {erreur}");
    };
    assert!(
        !diagnostics.is_empty(),
        "et nasm doit dire pourquoi : {erreur}"
    );
}

/// Cliquer l'adresse d'un panneau d'analyse emmène VRAIMENT au listing.
///
/// Au clic, et non par un état posé à la main : c'est tout l'objet de
/// ce contrôle. Le grief disait « dans figures de code et références
/// croisées on ne sait pas aller aux adresses, pourtant l'infobulle du
/// pointeur dit aller à cette adresse ». Le canal marchait pourtant —
/// le clic posait bien la demande, et l'application la relevait —,
/// mais deux choses manquaient au bout : le listing ne défilait pas, et
/// la fenêtre d'analyse restait devant. On lisait donc le numéro de la
/// ligne dans la barre d'état, derrière un panneau opaque.
///
/// Le clic DEMANDE désormais où aller : le contrôle passe donc par le
/// menu, comme la main.
#[test]
fn clicking_an_address_in_a_panel_really_lands_in_the_listing() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    if !app.load_elf(&octets, "ls") {
        return;
    }
    app.show_analysis = true;
    app.analysis_module = "xrefs".to_owned();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1600.0, 1000.0))
        .build_ui_state(|ui, app: &mut NamsdecApp| app.draw(ui), app);
    harness.run();
    {
        let adresses: Vec<_> = harness.get_all_by_label_contains("0x00").collect();
        assert!(
            !adresses.is_empty(),
            "le panneau des références croisées montre des adresses"
        );
        adresses[0].click();
    }
    harness.run();
    // Le menu des destinations, et l'entrée qui mène au code.
    harness.get_by_label_contains("Dans le listing").click();
    harness.run();

    assert!(
        !harness.state().show_analysis,
        "la fenêtre d'analyse s'efface : elle couvrait ce qu'on venait de demander à voir"
    );
    assert!(
        matches!(harness.state().status, Status::WentTo(_)),
        "et la barre d'état dit où l'on va : {:?}",
        harness.state().status
    );
    assert!(
        harness.state().cursor_pos.0 > 1,
        "le curseur est posé sur la ligne de l'adresse, et non restée en tête"
    );
}

/// Et l'autre destination : les OCTETS de cette adresse.
///
/// Le clic sur une adresse emmenait au listing sans rien demander, ce
/// qui est la bonne réponse pour une fonction et la mauvaise pour une
/// chaîne, une table ou un bourrage — on voulait voir les octets, on
/// atterrissait sur un désassemblage de données. Le contrôle prend le
/// second chemin et vérifie les trois choses qui le distinguent du
/// premier : la fenêtre d'analyse RESTE ouverte (la destination est
/// dedans), le panneau devient l'éditeur binaire, et l'adresse cliquée
/// y est écrite — on vient de la cliquer, la retrouver est ce qui dit
/// qu'on est au bon endroit.
#[test]
fn choosing_the_bytes_opens_the_binary_editor_where_the_address_is() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    if !app.load_elf(&octets, "ls") {
        return;
    }
    app.show_analysis = true;
    app.analysis_module = "xrefs".to_owned();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1600.0, 1000.0))
        .build_ui_state(|ui, app: &mut NamsdecApp| app.draw(ui), app);
    harness.run();
    {
        let adresses: Vec<_> = harness.get_all_by_label_contains("0x00").collect();
        assert!(!adresses.is_empty(), "le panneau montre des adresses");
        adresses[0].click();
    }
    harness.run();
    harness.get_by_label_contains("Dans les octets").click();
    harness.run();

    assert!(
        harness.state().show_analysis,
        "la fenêtre reste ouverte : ce qu'on va voir est dedans"
    );
    assert_eq!(
        harness.state().analysis_module,
        "hex_editor",
        "et le panneau est devenu l'éditeur binaire"
    );
    // Quelle adresse : celle que la barre d'état nomme, et c'est elle
    // qu'on doit retrouver écrite dans l'éditeur.
    let Status::WentTo(addr) = harness.state().status else {
        panic!(
            "la barre d'état dit où l'on va : {:?}",
            harness.state().status
        );
    };
    let écrite = format!("{addr:#010x}");
    assert!(
        harness.get_all_by_label_contains(&écrite).next().is_some(),
        "« {écrite} » doit se retrouver dans l'éditeur binaire"
    );
}

/// Et le listing **défile** jusqu'à cette ligne.
///
/// Le bout qui manquait vraiment : poser le curseur ne déplace pas la
/// vue. egui ne défile vers le curseur d'un champ de texte que lorsque
/// la sélection change pendant que le champ a le focus — or c'est nous
/// qui l'écrivons dans l'état du champ, et à l'image suivante il n'a
/// rien changé. On lisait donc « Lig 60749 » dans la barre d'état
/// pendant que le listing restait sur la ligne 1, et chaque adresse
/// cliquée d'un panneau d'analyse passait pour un clic mort.
///
/// Mesuré sur la galère du code : si la vue a défilé, le texte est
/// peint bien au-dessus du haut de l'éditeur.
#[test]
fn going_to_a_line_really_scrolls_the_listing_to_it() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    if !app.load_elf(&octets, "ls") {
        return;
    }
    let lignes = app.source.lines().count() as u32;
    // Un binaire assez long pour qu'il y ait quelque part où aller :
    // sur un /bin/ls minuscule, le contrôle n'apprendrait rien.
    if lignes < 400 {
        return;
    }
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = écran_du_banc();

    // Les numéros que la gouttière écrit : c'est la réponse la plus
    // directe à « où en est la vue ». Le contrôle lisait le haut de la
    // galère du code — il n'y en a plus qu'une par ligne visible, et
    // c'est précisément ce qu'on vérifie ici.
    let numéros = |app: &mut NamsdecApp| -> Vec<u32> {
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        let mut vus = Vec::new();
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape
                && let Ok(n) = texte.galley.text().trim().parse::<u32>()
            {
                vus.push(n);
            }
        }
        sortie.textures_delta.clear();
        vus
    };

    for _ in 0..3 {
        numéros(&mut app);
    }
    let avant = numéros(&mut app);
    assert!(
        avant.contains(&1),
        "sans rien demander, le listing est à son début : {avant:?}"
    );

    let cible = lignes - 20;
    app.goto_line = Some(cible);
    // Quelques images : la zone de défilement se place, puis la
    // gouttière écrit ce qui est à l'écran.
    let mut après = Vec::new();
    for _ in 0..4 {
        après = numéros(&mut app);
    }
    assert!(
        après.contains(&cible),
        "le listing doit montrer la ligne {cible} : la gouttière écrit {après:?}"
    );
    assert!(
        !après.contains(&1),
        "et la ligne 1 n'y est plus : {après:?}"
    );
}

/// Rebaptiser une fonction se voit **dans le listing**.
///
/// Le nom paraissait partout ailleurs — arbre d'appels, références
/// croisées, « prochaine instruction » — et nulle part là où l'on passe
/// ses journées. Le listing est du texte, écrit une fois au chargement ;
/// il se récrit donc quand un nom change, et ses points d'arrêt suivent
/// leur adresse plutôt que leur numéro de ligne, puisqu'une étiquette
/// nouvelle décale tout ce qui la suit.
#[test]
fn renaming_a_function_shows_up_in_the_listing() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    if !app.load_elf(&octets, "ls") {
        return;
    }
    // Une instruction du milieu, et la ligne où le listing la montre.
    let (adresse, ligne) = {
        let state = app.assemble_state.as_ref().expect("un état");
        let adresse = state.disasm[state.disasm.len() / 2].address;
        let ligne = *state.addr_to_line.get(&adresse).expect("une ligne");
        (adresse, ligne)
    };
    // Un point d'arrêt plus bas : il doit rester sur SON instruction.
    let (arrêt_addr, arrêt_ligne) = {
        let state = app.assemble_state.as_ref().expect("un état");
        let addr = state.disasm[state.disasm.len() / 2 + 20].address;
        (addr, *state.addr_to_line.get(&addr).expect("une ligne"))
    };
    app.breakpoint_lines.insert(arrêt_ligne);
    assert!(
        !app.source.contains("vérifie_mdp"),
        "le nom n'est pas déjà là"
    );

    app.notes.set_name(adresse, "vérifie_mdp");
    app.refresh_listing_names();

    assert!(
        app.source.contains("vérifie_mdp:"),
        "le listing porte l'étiquette donnée"
    );
    let state = app.assemble_state.as_ref().expect("un état");
    let nouvelle = *state
        .addr_to_line
        .get(&adresse)
        .expect("l'adresse a toujours une ligne");
    assert!(
        nouvelle > ligne,
        "l'étiquette insérée décale la ligne : {ligne} → {nouvelle}"
    );
    // La ligne d'avant est bien l'étiquette, et la ligne rendue est
    // bien l'instruction.
    let lignes: Vec<&str> = app.source.lines().collect();
    assert_eq!(lignes[nouvelle as usize - 2].trim(), "vérifie_mdp:");
    // Le point d'arrêt a suivi son adresse.
    let arrêt_nouvelle = *state
        .addr_to_line
        .get(&arrêt_addr)
        .expect("le point d'arrêt a toujours une ligne");
    assert!(
        app.breakpoint_lines.contains(&arrêt_nouvelle),
        "le point d'arrêt suit son instruction : {arrêt_ligne} → {arrêt_nouvelle}, \
         il reste {:?}",
        app.breakpoint_lines
    );

    // Et se dédire rend le listing d'avant.
    app.notes.set_name(adresse, "");
    app.refresh_listing_names();
    assert!(
        !app.source.contains("vérifie_mdp"),
        "le nom retiré s'en va aussi du listing"
    );
    let state = app.assemble_state.as_ref().expect("un état");
    assert_eq!(
        state.addr_to_line.get(&adresse).copied(),
        Some(ligne),
        "et tout revient à sa place"
    );
}

/// UN CORRECTIF REFUSÉ LE DIT, et ne laisse pas croire qu'il a été posé.
///
/// Écrire sur l'en-tête — l'octet zéro porte le `7f` du nombre magique —
/// rend le fichier illisible par namsdec, qui refuse alors de le relire.
/// Le refus est juste : on ne désassemble pas ce qu'on ne sait plus lire.
///
/// Mais il ne se DISAIT pas. On sortait de `write_patch` sans rien
/// changer et sans un mot : l'octet reprenait sa valeur à l'image
/// suivante, et l'on tapait dans le vide. C'est la réponse au grief
/// « dans l'éditeur binaire on ne sait rien modifier » du 21 octobre
/// 2026, dont le parcours complet est éprouvé par
/// `the_hex_grid_takes_the_keyboard_from_the_editor`.
///
/// Ce contrôle-ci ne demande ni binaire du système ni nasm : il vaut
/// aussi sur une machine d'intégration.
#[test]
fn a_refused_patch_says_so() {
    let octets = elf_minimal();
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "minimal"), "l'ELF minimal se charge");
    let s = strings(app.lang);

    // D'abord ce qui MARCHE : un octet du code, loin de l'en-tête.
    let dans_le_code = octets.len() - 1;
    let avant = octets[dans_le_code];
    app.write_patch(dans_le_code, vec![avant ^ 0xff], 0);
    assert_eq!(
        app.assemble_state.as_ref().expect("un état").output.file[dans_le_code],
        avant ^ 0xff,
        "un octet du code s'écrit"
    );
    assert_eq!(app.patches.len(), 1, "et le correctif est inscrit");
    assert!(
        matches!(app.status, Status::Note(_)),
        "une écriture réussie se dit aussi"
    );

    // Puis ce qui est REFUSÉ : le nombre magique.
    let image = app
        .assemble_state
        .as_ref()
        .expect("un état")
        .output
        .file
        .clone();
    app.write_patch(0, vec![0x42], 0);
    assert_eq!(
        app.assemble_state.as_ref().expect("un état").output.file,
        image,
        "le fichier n'a pas changé d'un octet"
    );
    assert_eq!(app.patches.len(), 1, "et rien n'est inscrit à défaire");
    match &app.status {
        Status::Error(message) => assert_eq!(
            message, s.patch_breaks_file,
            "le refus se dit, et il dit pourquoi"
        ),
        autre => panic!("un correctif refusé doit le DIRE. Statut : {autre:?}"),
    }
}
