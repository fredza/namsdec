//! La mémoire : les onglets Mémoire, Pile et Tas.
//!
//! Les régions et leurs droits, la convention d'appel lue dans la pile,
//! la rupture du tas, la recherche d'un motif, l'écriture à la main, et qui
//! a écrit où.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// Et l'affichage d'une frame entière avec une saisie invalide ne
/// déplace rien de lui-même : il faut valider (Entrée ou le bouton).
#[test]
fn typing_alone_does_not_move_the_memory_view() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Memory,
        mem_addr: 0x401000,
        mem_addr_input: "pas une adresse".to_owned(),
        ..Default::default()
    };
    app.do_assemble();

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

    assert_eq!(
        app.mem_addr, 0x401000,
        "le vidage reste sur la dernière adresse validée"
    );
}

/// Le champ Adresse comprend les registres, et une petite
/// arithmétique autour : c'est ce qui permet de suivre un pointeur
/// qu'on a sous les yeux dans le bandeau, sans recopier seize
/// chiffres à la main.
#[test]
fn the_address_field_understands_registers_and_arithmetic() {
    let mut regs = namsdec_core::Registers::default();
    regs.set64(Reg64::Rsp, 0x40ff00);
    regs.set64(Reg64::Rsi, 0x401000);
    regs.set64(Reg64::Rcx, 4);
    regs.rip = 0x400020;
    let at = |expr: &str| eval_address(expr, Some(&regs));

    // Ce qui marchait déjà marche toujours.
    assert_eq!(at("0x401000"), Some(0x401000));
    assert_eq!(at("401000"), Some(0x401000), "le 0x reste facultatif");

    // Les registres, RIP compris.
    assert_eq!(at("rsp"), Some(0x40ff00));
    assert_eq!(at("RSI"), Some(0x401000), "la casse est indifférente");
    assert_eq!(at("rip"), Some(0x400020));

    // L'arithmétique, en hexadécimal de bout en bout.
    assert_eq!(
        at("rsi+10"),
        Some(0x401010),
        "les décalages aussi sont hexa"
    );
    assert_eq!(at("rsp-8"), Some(0x40fef8));
    assert_eq!(
        at(" rsi + rcx * 4 "),
        Some(0x401010),
        "l'indexation d'un tableau"
    );
    assert_eq!(at("rsp-8+8"), Some(0x40ff00));

    // Et ce qui n'est pas une adresse est refusé, plutôt que deviné.
    assert_eq!(at("truc"), None);
    assert_eq!(at("rsi+"), None, "une somme inachevée");
    assert_eq!(at(""), None);
    assert_eq!(at("rax*"), None);

    // Sans processeur chargé, les registres n'ont pas de valeur : le
    // champ le dit au lieu d'inventer un zéro.
    assert_eq!(eval_address("rsp", None), None);
    assert_eq!(eval_address("0x401000", None), Some(0x401000));
}

/// Les quatre lectures de la mémoire disent quatre choses différentes
/// des mêmes octets.
#[test]
fn the_memory_formats_read_the_same_bytes_differently() {
    let mut mem = namsdec_core::Memory::new(BASE_ADDR, 0x1000);
    // « Hi », un octet nul, puis un mot de huit octets valant 1.
    mem.load(BASE_ADDR, b"Hi\0\0\0\0\0\0\x01\0\0\0\0\0\0\0")
        .unwrap();

    let bytes = read_memory(
        &mem,
        BASE_ADDR,
        MemoryFormat::Bytes,
        "hors bornes",
        Lang::Fr,
    );
    assert!(bytes.contains("48 69"), "l'hexadécimal brut : {bytes}");

    let words = read_memory(
        &mem,
        BASE_ADDR,
        MemoryFormat::Words,
        "hors bornes",
        Lang::Fr,
    );
    let second = words.lines().nth(1).expect("un deuxième mot");
    assert!(
        second.ends_with("  1") && second.contains("0000000000000001"),
        "le mot suivant, en hexa et en décimal : {second}"
    );

    let text = read_memory(&mem, BASE_ADDR, MemoryFormat::Text, "hors bornes", Lang::Fr);
    let première = text.lines().next().expect("une ligne");
    assert!(
        première.contains("Hi") && !première.contains("48"),
        "le texte s'arrête au premier nul, sans hexadécimal : {text}"
    );
    assert!(
        text.contains("puis un zéro"),
        "et il dit où `write` s'arrêterait : {text}"
    );

    // Et les pointeurs : le mot de huit octets qui vaut 1 ne pointe
    // nulle part, mais celui qu'on pose ici mène à « Hi ».
    mem.map(
        BASE_ADDR..BASE_ADDR + 0x1000,
        namsdec_core::Perms::DATA,
        "test",
    );
    mem.write_u64(BASE_ADDR + 0x100, BASE_ADDR).unwrap();
    let chaîne = read_memory(
        &mem,
        BASE_ADDR + 0x100,
        MemoryFormat::Chain,
        "hors bornes",
        Lang::Fr,
    );
    let première = chaîne.lines().next().expect("une ligne");
    assert!(
        première.contains(&format!("{BASE_ADDR:016x}")) && première.contains("\"Hi\""),
        "l'adresse suivie jusqu'à son texte : {première}"
    );

    // Hors de la fenêtre simulée, les quatre le disent.
    for format in MemoryFormat::ALL {
        assert_eq!(
            read_memory(&mem, 0xdead_beef, format, "hors bornes", Lang::Fr),
            "hors bornes"
        );
    }
}

/// Le champ « Adresse » a la largeur d'une adresse, pas celle du
/// panneau. Un champ de saisie egui réclame par défaut toute la place
/// disponible : sur un panneau large, celui-ci s'étirait sur des
/// centaines de pixels pour dix-huit caractères.
#[test]
fn the_address_field_is_as_wide_as_an_address_not_as_the_panel() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));

    // La même vue dans un panneau étroit, puis dans un panneau deux
    // fois plus large.
    let field_width = |sidebar_width: f32| {
        let mut app = NamsdecApp {
            active_tab: SidebarTab::Memory,
            ..Default::default()
        };
        app.do_assemble();
        let ctx = egui::Context::default();
        ctx.data_mut(|d| {
            d.insert_persisted(
                egui::Id::new("sidebar"),
                egui::containers::PanelState {
                    outer_rect: egui::Rect::from_min_max(
                        egui::pos2(screen.right() - sidebar_width, 0.0),
                        egui::pos2(screen.right(), screen.bottom()),
                    ),
                },
            );
        });
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            output.textures_delta.clear();
        }
        let sidebar = egui::containers::PanelState::load(&ctx, egui::Id::new("sidebar"))
            .expect("le panneau latéral est affiché")
            .outer_rect;
        let field = ctx
            .read_response(memory_address_id())
            .expect("le champ Adresse est affiché")
            .rect;
        assert!(
            field.right() <= sidebar.right() + 1.0,
            "le champ ne doit pas dépasser le panneau ({} contre {})",
            field.right(),
            sidebar.right()
        );
        (sidebar.width(), field.width())
    };

    let (narrow_panel, narrow_field) = field_width(300.0);
    let (wide_panel, wide_field) = field_width(620.0);

    assert!(
        wide_panel > narrow_panel,
        "les deux panneaux doivent différer"
    );
    assert_eq!(
        narrow_field, wide_field,
        "le champ doit garder sa taille quand le panneau s'élargit"
    );
    assert!(
        wide_field < wide_panel * 0.6,
        "et rester un champ d'adresse, pas une barre : {wide_field} px"
    );
}

/// L'onglet Tas montre la frontière que `brk` déplace. Aucun registre
/// ne la porte : sans l'émulateur qui la tient, ni l'utilisateur ni un
/// débogueur ne sauraient où la chercher.
#[test]
fn the_heap_tab_shows_the_break_the_program_moved() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Heap,
        source: "\
section .text
global _start
_start:
mov rax, 12
xor rdi, rdi
syscall
mov rdi, rax
add rdi, 4096
mov rax, 12
syscall
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();

    let heap = app.emulator.as_ref().unwrap().heap();
    assert_eq!(heap.len(), 0, "rien n'est demandé avant de tourner");

    for _ in 0..10 {
        app.do_step();
    }
    let heap = app.emulator.as_ref().unwrap().heap();
    assert_eq!(heap.len(), 4096, "le programme a pris 4 Kio");

    let ctx = egui::Context::default();
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
                painted.push('\n');
            }
        }
        output.textures_delta.clear();
    }

    for expected in [
        format!("{:#x}", heap.start()),
        format!("{:#x}", heap.current()),
        format!("{:#x}", heap.limit()),
    ] {
        assert!(
            painted.contains(&expected),
            "l'onglet doit montrer {expected} ; peint : {painted}"
        );
    }
    assert!(
        painted.contains("4096 / "),
        "il doit dire ce qui est pris sur ce qui reste"
    );
}

/// Un programme ne peut pas écrire dans son propre code, ni exécuter ses
/// données, ni toucher ce qui n'est projeté nulle part — et chaque refus
/// se lit en français, avec l'adresse et la zone en cause.
///
/// C'est la leçon que namsdec ne savait pas donner : « pourquoi ça
/// segfault » restait invisible, le programme écrivait dans `.text` et
/// continuait comme si de rien n'était.
#[test]
fn a_program_cannot_write_its_own_code_or_run_its_data() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let faute = |source: &str| -> String {
        let mut app = NamsdecApp {
            source: source.to_owned(),
            ..Default::default()
        };
        app.do_assemble();
        for _ in 0..40 {
            app.do_step();
            if let Status::Error(message) = &app.status {
                return message.clone();
            }
        }
        String::new()
    };

    // Écrire dans le code.
    let message =
        faute("global _start\nsection .text\n_start:\n    mov byte [_start], 0x90\n    hlt\n");
    assert!(
        message.contains("interdit d'écrire") && message.contains(".text"),
        "obtenu : {message}"
    );

    // Sauter dans les données.
    let message = faute(
        "global _start\nsection .data\n    tampon times 8 db 0\n\
         section .text\n_start:\n    jmp tampon\n",
    );
    assert!(
        message.contains("interdit d'exécuter") && message.contains(".data"),
        "obtenu : {message}"
    );

    // Écrire là où il n'y a rien : le défaut de segmentation, dit en
    // clair.
    let message = faute(
        "global _start\nsection .text\n_start:\n    mov rax, 0x407000\n\
         \x20   mov qword [rax], 1\n    hlt\n",
    );
    assert!(message.contains("rien n'est projeté"), "obtenu : {message}");

    // Et un programme honnête tourne toujours : il écrit dans ses
    // données, lit son code, et s'arrête.
    let message = faute(
        "global _start\nsection .data\n    n dq 0\nsection .text\n_start:\n\
         \x20   mov qword [n], 42\n    mov rax, [n]\n    hlt\n",
    );
    assert!(
        message.is_empty(),
        "un programme correct ne doit rien déclencher : {message}"
    );
}

/// Le tas n'existe que jusqu'à la rupture : au-delà, il n'y a rien, même
/// si l'adresse est dans la fenêtre. C'est ce qui apprend à demander la
/// place avant de s'en servir.
#[test]
fn the_heap_exists_only_up_to_the_break() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "global _start\nsection .text\n_start:\n\
             \x20   mov rax, 12\n    xor rdi, rdi\n    syscall\n\
             \x20   mov r12, rax\n\
             \x20   mov qword [r12], 1      ; rien n'est encore alloué\n    hlt\n"
            .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    for _ in 0..20 {
        app.do_step();
    }
    let Status::Error(message) = &app.status else {
        panic!(
            "écrire avant d'avoir demandé doit échouer : {:?}",
            app.status
        );
    };
    assert!(message.contains("rien n'est projeté"), "obtenu : {message}");

    // Avec le brk qui va bien, la même écriture passe.
    app.source = "global _start\nsection .text\n_start:\n\
         \x20   mov rax, 12\n    xor rdi, rdi\n    syscall\n    mov r12, rax\n\
         \x20   lea rdi, [rax + 4096]\n    mov rax, 12\n    syscall\n\
         \x20   mov qword [r12], 1\n    hlt\n"
        .to_owned();
    app.do_assemble();
    for _ in 0..20 {
        app.do_step();
    }
    assert!(
        matches!(app.status, Status::Halted),
        "obtenu : {:?}",
        app.status
    );
}

/// L'onglet Mémoire dit à quelle zone appartient l'adresse regardée, et
/// avec quels droits : on lit la protection avant de se la voir opposer.
#[test]
fn the_memory_tab_names_the_region_and_its_rights() {
    let s = strings(Lang::Fr);
    let mut mem = namsdec_core::Memory::new(BASE_ADDR, 0x4000);
    // Sans zone déclarée, rien à dire : un binaire brut n'a pas de
    // sections, et inventer des droits serait mentir.
    assert_eq!(region_line(s, &mem, BASE_ADDR), "");

    mem.map(
        BASE_ADDR..BASE_ADDR + 0x1000,
        namsdec_core::Perms::TEXT,
        ".text",
    );
    mem.map(
        BASE_ADDR + 0x1000..BASE_ADDR + 0x2000,
        namsdec_core::Perms::DATA,
        ".data",
    );
    assert_eq!(
        region_line(s, &mem, BASE_ADDR),
        ".text · lecture, exécution"
    );
    assert_eq!(
        region_line(s, &mem, BASE_ADDR + 0x1000),
        ".data · lecture, écriture"
    );
    assert_eq!(
        region_line(s, &mem, BASE_ADDR + 0x3000),
        "rien n'est projeté ici"
    );
}

/// L'onglet Pile montre la convention d'appel — par où passent les
/// arguments, ce qu'une routine doit rendre — et nomme celles qui ne l'ont
/// pas respectée.
#[test]
fn the_stack_tab_teaches_the_calling_convention_and_checks_it() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Stack,
        source: "\
global _start

section .text
_start:
    mov rdi, 1
    mov rsi, 2
    mov rbx, 7
    call malpolie
    hlt

malpolie:
    mov rbx, 42             ; empruntée, jamais rendue
    ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    for _ in 0..20 {
        app.do_step();
    }

    let ctx = egui::Context::default();
    // Une fenêtre très haute : l'onglet montre d'abord la pile
    // elle-même, puis les adresses de retour, puis l'aide-mémoire de la
    // convention — c'est lui qu'on vérifie ici. Ce qui dépasse est
    // maintenant coupé par la zone de défilement de l'onglet, comme il
    // se doit : pour le lire, il faut donc la place de l'afficher.
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 2400.0),
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

    // L'aide-mémoire : les six registres d'arguments, dans l'ordre.
    assert!(painted.contains("RDI|0x1|RSI|0x2"), "{painted}");
    assert!(painted.contains("Convention d'appel"), "{painted}");
    // Et la vérification, sur ce programme-ci.
    assert!(painted.contains("Entorses relevées"), "{painted}");
    assert!(painted.contains("n'a pas rendu"), "{painted}");
    assert!(painted.contains("RBX"), "{painted}");
    // Et d'où partait l'appel, en ligne de source — pas en hexadécimal.
    assert!(painted.contains("appelée depuis ligne 8"), "{painted}");
}

/// Les octets qui viennent de changer ressortent dans le vidage, comme un
/// registre dans le bandeau : c'est ce qui répond à « qu'est-ce que cette
/// instruction a écrit ? » sans relire seize octets un à un.
#[test]
fn the_memory_dump_highlights_what_just_changed() {
    let ctx = egui::Context::default();
    let theme = namsdec_theme::by_id("vscode-dark");
    let mut job = None;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let avant = [0u8; 8];
        let maintenant = [0u8, 0, 0x42, 0, 0, 0, 0, 0];
        job = Some(hexdump_job(ui, theme, &maintenant, 0x400000, Some(&avant)));
    });
    output.textures_delta.clear();
    let job = job.expect("un vidage");

    // Le troisième octet, et lui seul, porte la couleur du changement —
    // deux fois : en hexadécimal et dans la colonne de texte.
    let changés: Vec<&str> = job
        .sections
        .iter()
        .filter(|section| section.format.color == theme.changed)
        .map(|section| &job.text[section.byte_range.start.0..section.byte_range.end.0])
        .collect();
    assert_eq!(changés, ["42 ", "B"], "obtenu {changés:?}");

    // Sans point de comparaison — au chargement, avant le premier pas —
    // rien n'est mis en avant.
    let mut job = None;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        job = Some(hexdump_job(ui, theme, &[0u8, 0, 0x42], 0x400000, None));
    });
    output.textures_delta.clear();
    assert!(
        job.expect("un vidage")
            .sections
            .iter()
            .all(|section| section.format.color != theme.changed)
    );
}

/// La pile se lit par mots, avec ses repères : où est RSP, où est RBP,
/// et laquelle de ces valeurs est une adresse de retour.
#[test]
fn the_stack_shows_rsp_rbp_and_the_return_address() {
    let mut app = NamsdecApp {
        // La vue de la pile, demandée : depuis que l'état du processeur
        // est un onglet, c'est lui qu'on trouve en ouvrant.
        active_tab: SidebarTab::Stack,
        source: "\
global _start
section .text
_start:
call sous
hlt
sous:
push rbp
mov rbp, rsp
ret
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step(); // call  → l'adresse de retour est empilée
    app.do_step(); // push rbp
    app.do_step(); // mov rbp, rsp

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
        painted.contains("<- RSP"),
        "RSP doit être repéré : {painted}"
    );
    assert!(
        painted.contains("<- RBP") || painted.contains("RSP, <- RBP"),
        "RBP aussi, une fois le cadre posé : {painted}"
    );
    assert!(
        painted.contains("adresse de code"),
        "l'adresse de retour empilée par `call` doit être nommée : {painted}"
    );
    // Et elle porte l'étiquette de l'endroit d'où l'on vient.
    assert!(
        painted.contains("_start+"),
        "l'adresse de retour pointe juste après le call, dans _start : {painted}"
    );
}

/// L'onglet Mémoire nomme ce qu'on regarde : l'étiquette du source, la
/// zone et ses droits — pas seulement une adresse.
#[test]
fn the_memory_tab_names_what_it_shows() {
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
message: db \"salut\", 10
section .text
_start:
hlt
"
        .to_owned(),
        active_tab: SidebarTab::Memory,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // On va voir l'étiquette, comme on le ferait en tapant son nom.
    let message = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|symbol| symbol.name == "message")
        .expect("l'étiquette est dans l'ELF")
        .addr;
    app.mem_addr = message;

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
        painted.contains("message · "),
        "l'étiquette doit précéder la zone : {painted}"
    );
    assert!(
        painted.contains("écriture"),
        "et les droits de .data avec : {painted}"
    );
}

/// L'onglet Mémoire dit qui a écrit à l'adresse qu'on regarde.
#[test]
fn the_memory_tab_says_who_wrote_there() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        active_tab: SidebarTab::Memory,
        source: "\
global _start
section .data
case: dq 0
section .text
_start:
mov rax, 7
mov [case], rax
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let case = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|s| s.name == "case")
        .expect("l'étiquette existe")
        .addr;
    app.mem_addr = case;

    // Avant l'exécution, personne n'a écrit : rien à dire.
    assert!(app.last_memory_write_line(case).is_none());

    let ctx = egui::Context::default();
    app.do_run();
    for _ in 0..10 {
        if !app.running {
            break;
        }
        app.tick_run(&ctx);
    }

    let texte = app
        .last_memory_write_line(case)
        .expect("le `mov [case], rax` a écrit là");
    assert!(texte.contains("ligne 7"), "{texte}");

    // Et cela se peint dans l'onglet.
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
    assert!(painted.contains("Écrit au pas"), "{painted}");
}

/// Chercher en mémoire : « où est cette chaîne ? » ne se répond pas en
/// faisant défiler un vidage.
///
/// Les trois écritures sont celles qu'on a sous les yeux : un texte
/// qu'on vient d'écrire, des octets relevés dans un vidage, une valeur
/// lue dans un registre.
#[test]
fn memory_can_be_searched_the_three_ways_one_writes_a_needle() {
    // Le texte, tel quel.
    assert_eq!(SearchKind::Text.needle("salut"), Some(b"salut".to_vec()));
    // Les octets, avec ou sans espaces, avec ou sans 0x.
    assert_eq!(
        SearchKind::Bytes.needle("48 89 d8"),
        Some(vec![0x48, 0x89, 0xd8])
    );
    assert_eq!(SearchKind::Bytes.needle("0x4889"), Some(vec![0x48, 0x89]));
    // Un nombre impair de chiffres ne désigne pas des octets.
    assert_eq!(SearchKind::Bytes.needle("489"), None);
    assert_eq!(SearchKind::Bytes.needle("zz"), None);
    // Une valeur : petit-boutiste, sur ses octets utiles.
    assert_eq!(SearchKind::Value.needle("0x1234"), Some(vec![0x34, 0x12]));
    assert_eq!(SearchKind::Value.needle("255"), Some(vec![0xff]));
    // Rien à chercher n'est pas une recherche.
    assert_eq!(SearchKind::Text.needle("   "), None);

    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start

section .data
message: db \"salut\", 0

section .text
_start:
    hlt
"
        .to_owned(),
        active_tab: SidebarTab::Memory,
        mem_search: "salut".to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Et la chaîne se trouve là où le programme l'a écrite.
    let emulator = app.emulator.as_ref().expect("chargé");
    let trouvés = namsdec_emu::telescope::search(&emulator.cpu().mem, b"salut", 24);
    assert_eq!(trouvés.len(), 1, "{trouvés:?}");
    let symbols = &app.assemble_state.as_ref().unwrap().output.symbols;
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "message" && s.addr == trouvés[0]),
        "l'adresse trouvée est celle de `message`"
    );
}

/// On écrit en mémoire à la main, y compris là où le programme n'aurait
/// pas le droit — c'est ce que fait un débogueur —, et le programme,
/// lui, continue d'obéir aux règles.
#[test]
fn memory_is_written_by_hand_where_the_program_could_not() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
default rel
global _start

section .data
tampon: times 8 db 0

section .text
_start:
    mov al, [tampon]
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    let tampon = app
        .assemble_state
        .as_ref()
        .unwrap()
        .output
        .symbols
        .iter()
        .find(|s| s.name == "tampon")
        .expect("l'étiquette existe")
        .addr;

    // Ce qu'on écrit, le programme le lit.
    app.poke_memory(tampon, b"\x42");
    app.do_step();
    assert_eq!(
        app.emulator.as_ref().unwrap().cpu().regs.get64(Reg64::Rax) & 0xff,
        0x42
    );

    // Et l'on écrit même dans le code, que le programme ne pourrait pas
    // toucher : c'est l'observateur qui écrit, pas lui.
    let code = app.assemble_state.as_ref().unwrap().output.entry;
    app.poke_memory(code, &[0x90]);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);
    assert_eq!(
        app.emulator
            .as_ref()
            .unwrap()
            .cpu()
            .mem
            .peek_bytes(code, 1)
            .unwrap(),
        &[0x90]
    );

    // Hors de la mémoire simulée, en revanche, il n'y a rien à écrire.
    app.poke_memory(0xdead_beef, &[0]);
    assert!(matches!(app.status, Status::Error(_)), "{:?}", app.status);
}

/// La pile dit toutes ses adresses de retour, pas seulement celles des
/// douze mots affichés : après un débordement, la question est
/// « laquelle a été écrasée ».
#[test]
fn the_stack_lists_every_return_address_it_carries() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start

section .text
_start:
    call une
    hlt
une:
    call deux
    ret
deux:
    ret
"
        .to_owned(),
        active_tab: SidebarTab::Stack,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step(); // call une
    app.do_step(); // call deux

    // Deux appels imbriqués : deux adresses de retour sur la pile, la
    // plus récente en premier, chacune nommée par l'étiquette de son
    // appelant.
    let emulator = app.emulator.as_ref().expect("chargé");
    let symbols = app.assemble_state.as_ref().unwrap().output.symbols.clone();
    let trouvées = return_addresses(emulator.cpu(), &app.notes, &symbols, 32);
    assert_eq!(trouvées.len(), 2, "{trouvées:?}");
    assert!(trouvées[0].2.starts_with("une"), "{trouvées:?}");
    assert!(trouvées[1].2.starts_with("_start"), "{trouvées:?}");
    assert!(trouvées[0].0 < trouvées[1].0, "de RSP vers le haut");

    // Écrasée par un débordement, elle disparaît de la liste : c'est
    // exactement ce qu'on vient y lire.
    app.poke_memory(trouvées[0].0, b"aaaabbbb");
    let emulator = app.emulator.as_ref().expect("chargé");
    let après = return_addresses(emulator.cpu(), &app.notes, &symbols, 32);
    assert_eq!(après.len(), 1, "{après:?}");
}

/// L'onglet Mémoire suit ce qu'on lui demande : la pile ou le code,
/// pas à pas, sans qu'on retape l'adresse à chaque instruction.
#[test]
fn the_memory_tab_follows_the_register_it_is_told_to() {
    // Alignée sur seize octets : un vidage qui danse d'une colonne à
    // l'autre à chaque pas ne se lit pas.
    let mut regs = namsdec_core::Registers::new();
    regs.set64(Reg64::Rsp, 0x40fef8);
    regs.rip = 0x400007;
    assert_eq!(MemFollow::Fixed.address(&regs), None);
    assert_eq!(MemFollow::Rsp.address(&regs), Some(0x40fef0));
    assert_eq!(MemFollow::Rip.address(&regs), Some(0x400000));

    // Et le choix survit au lancement suivant, comme tout réglage.
    let mut app = NamsdecApp {
        memory_follow: MemFollow::Rsp,
        ..Default::default()
    };
    let écrit = serde_json::to_string(&app.settings_snapshot()).unwrap();
    assert!(écrit.contains("\"memory_follow\":\"rsp\""), "{écrit}");
    app.memory_follow = MemFollow::by_id("rip");
    assert_eq!(app.memory_follow, MemFollow::Rip);
    // Un identifiant inconnu ne fait pas suivre n'importe quoi.
    assert_eq!(MemFollow::by_id("n'importe quoi"), MemFollow::Fixed);
}

/// La provenance remonte plusieurs pas, et non un seul.
///
/// « D'où vient ce que RAX contient ? » est la question qu'on pose
/// devant chaque comparaison d'un programme qu'on ne connaît pas, et
/// une seule réponse ne suffit presque jamais : le pas qui a écrit RAX
/// lisait RCX, et c'est RCX qu'il faut alors remonter. On le faisait à
/// la main, un survol après l'autre.
#[test]
fn the_provenance_goes_back_several_steps() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    // Une chaîne volontairement nette : RDX vient de RCX, qui vient de
    // RBX, qui vient d'une constante.
    let mut app = NamsdecApp {
        source: "\
global _start
section .text
_start:
    mov rbx, 7
    mov rcx, rbx
    mov rdx, rcx
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let chaîne = app.provenance_chain("RDX", 4);
    assert!(
        chaîne.len() >= 3,
        "trois maillons au moins : RDX, RCX, RBX — {chaîne:?}"
    );
    // Chaque maillon nomme le registre qu'il explique, et l'instruction
    // qui l'a écrit.
    assert!(chaîne[0].starts_with("RDX"), "{:?}", chaîne[0]);
    assert!(chaîne[0].contains("mov rdx, rcx"), "{:?}", chaîne[0]);
    assert!(chaîne[1].starts_with("RCX"), "{:?}", chaîne[1]);
    assert!(chaîne[1].contains("mov rcx, rbx"), "{:?}", chaîne[1]);
    assert!(chaîne[2].starts_with("RBX"), "{:?}", chaîne[2]);
    // `mov ebx, 7` et non `mov rbx, 7` : nasm choisit la forme 32 bits
    // pour un petit immédiat, et l'écriture d'un registre de 32 bits met
    // à zéro la moitié haute du registre de 64. C'est le même effet en
    // deux octets de moins, et c'est ce que le désassemblage montre.
    assert!(chaîne[2].contains("bx, 7"), "{:?}", chaîne[2]);

    // Bornée : on demande quatre maillons, on n'en reçoit pas cinq.
    assert!(app.provenance_chain("RDX", 2).len() <= 2);
    // Un registre que personne n'a écrit n'a pas de provenance.
    assert!(app.provenance_chain("R15", 4).is_empty());
}

/// La chaîne s'arrête à la mémoire, et le dit.
///
/// Une valeur qui vient d'un `[rbp-8]` ne se remonte pas par son
/// registre de base : cela mènerait à la mise en place du cadre de
/// pile, pas à la valeur. Mieux vaut s'arrêter en le disant que de
/// conduire ailleurs avec assurance.
#[test]
fn the_chain_stops_at_memory_and_says_so() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
valeur dq 42
section .text
_start:
    mov rax, [valeur]
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_run();
    for _ in 0..50 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }

    let chaîne = app.provenance_chain("RAX", 4);
    assert!(chaîne.len() >= 2, "{chaîne:?}");
    assert!(chaîne[0].contains("mov rax,"), "{:?}", chaîne[0]);
    assert_eq!(
        chaîne[1], s.provenance_from_memory,
        "la piste s'arrête là, et le dit"
    );
}

/// Chercher une suite d'octets dans la mémoire du programme.
///
/// C'est le premier réflexe devant un programme qui compare quelque
/// chose : on a lu une chaîne, on veut savoir où elle est écrite, et
/// c'est de là qu'on remonte à ce qui la lit.
#[test]
fn a_byte_pattern_is_found_in_the_running_memory() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start
section .data
mot db \"HVUHADN\", 0
section .text
_start:
hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    let mem = &app.emulator.as_ref().expect("un émulateur").cpu().mem;

    // L'adresse que l'assembleur a donnée à l'étiquette : c'est elle
    // qu'on doit retrouver, et non une adresse voisine.
    let attendue = app
        .assemble_state
        .as_ref()
        .expect("assemblé")
        .output
        .symbols
        .iter()
        .find(|s| s.name == "mot")
        .map(|s| s.addr)
        .expect("l'étiquette « mot » existe");

    // Le texte.
    let trouvés = sidebar::search_memory(mem, &memfind::parse("\"HVUHADN\"").unwrap());
    assert!(
        trouvés.contains(&attendue),
        "« HVUHADN » doit être en {attendue:#x} : {trouvés:x?}"
    );

    // Les mêmes octets en hexadécimal, et avec un joker au milieu.
    for écriture in ["48 56 55 48", "48 56 ?? 48", "\"HV\" ?? 48"] {
        let trouvés = sidebar::search_memory(mem, &memfind::parse(écriture).unwrap());
        assert!(
            trouvés.contains(&attendue),
            "« {écriture} » doit trouver {attendue:#x} : {trouvés:x?}"
        );
    }

    // Ce qui n'y est pas ne se trouve pas — et ne fait rien tomber.
    let trouvés = sidebar::search_memory(mem, &memfind::parse("\"PASDEDANS\"").unwrap());
    assert!(trouvés.is_empty(), "{trouvés:x?}");
}

/// La mémoire montre sa carte : où commence le code, où finit la pile,
/// ce que le tas occupe, avec les droits de chacun.
///
/// C'est le `vmmap` de pwndbg. Un refus d'accès n'apprend rien à qui ne
/// sait pas où il se trouvait ; la carte le dit avant.
#[test]
fn the_memory_tab_shows_the_map_of_the_process() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut app = NamsdecApp {
        source: "\
global _start

section .data
    tampon times 8 db 0

section .text
_start:
    mov rax, 1
    hlt
"
        .to_owned(),
        active_tab: SidebarTab::Memory,
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);

    // Les zones existent et se lisent comme un vmmap : trois caractères
    // de droits par ligne.
    let emulator = app.emulator.as_ref().expect("chargé");
    let zones = emulator.cpu().mem.regions();
    assert!(zones.len() >= 3, "code, données, pile au moins : {zones:?}");
    let texte: Vec<String> = zones.iter().map(|z| perms_text(z.perms)).collect();
    assert!(texte.contains(&"r-x".to_owned()), "du code : {texte:?}");
    assert!(texte.contains(&"rw-".to_owned()), "des données : {texte:?}");
    // Les zones ne se chevauchent pas dans la carte, et sont triées.
    for paire in zones.windows(2) {
        assert!(
            paire[0].range.start <= paire[1].range.start,
            "la carte est triée : {zones:?}"
        );
    }

    // Et les outils qui la portent sont là, repliés : le vidage reste
    // ce qu'on voit en premier.
    let painted = paint(&mut app);
    assert!(
        painted.contains(strings(app.lang).memory_tools),
        "{painted}"
    );
    assert!(
        painted.contains("_start"),
        "l'étiquette de l'adresse regardée reste visible : {painted}"
    );
}

/// La pile suit ses pointeurs : un mot qui pointe une chaîne le dit, un
/// mot qui pointe du code montre l'instruction.
///
/// C'est le geste qu'on fait sinon à la main, une lecture après l'autre —
/// celui que pwndbg appelle *telescope*. Un mot de pile n'est presque
/// jamais un nombre.
#[test]
fn the_stack_follows_its_pointers() {
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
    lea rax, [message]
    push rax
    hlt
"
        .to_owned(),
        ..Default::default()
    };
    app.do_assemble();
    assert!(matches!(app.status, Status::AssembleOk), "{:?}", app.status);
    app.do_step();
    app.do_step();

    let painted = paint(&mut app);
    assert!(
        painted.contains("\"salut\""),
        "le mot empilé pointe une chaîne, et la pile le dit : {painted}"
    );

    // Et en mémoire, le format « Pointeurs » dit la même chose à partir
    // de n'importe quelle adresse.
    let emulator = app.emulator.as_ref().expect("chargé");
    let rsp = emulator.cpu().regs.get64(Reg64::Rsp);
    let dump = read_memory(
        &emulator.cpu().mem,
        rsp,
        MemoryFormat::Chain,
        "hors de la mémoire",
        Lang::Fr,
    );
    assert!(dump.contains("\"salut\""), "{dump}");
    assert_eq!(dump.lines().count(), 8, "huit mots, comme telescope");
}
