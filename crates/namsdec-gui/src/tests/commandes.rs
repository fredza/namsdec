//! La palette, les menus, les raccourcis.
//!
//! Toute action doit se trouver — par la palette, par un menu, par une
//! touche — et dire pourquoi quand elle est grisée. Le clavier de l'éditeur
//! est dans [`super::editeur`].
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// La palette ne propose pas la vue RE tant qu'elle est masquée.
///
/// Elle la proposait toujours, et s'y rendre *allumait* le réglage —
/// enregistré dans la foulée. Un aller-retour dans la palette
/// suffisait donc à faire réapparaître l'onglet pour de bon, sans
/// être passé par Préférences.
#[test]
fn the_palette_offers_the_re_view_only_when_it_is_visible() {
    let s = strings(Lang::Fr);
    let labels = |show_re_tab| -> Vec<String> {
        palette_commands(s, show_re_tab, false)
            .into_iter()
            .map(|(label, _)| label)
            .collect()
    };
    let entry = format!("{}: {}", s.menu_view, s.tab_re);

    assert!(
        !labels(false).contains(&entry),
        "onglet masqué : la commande n'a pas à être offerte"
    );
    assert!(labels(true).contains(&entry), "onglet visible : elle l'est");

    // Et s'y rendre ne touche plus au réglage.
    let mut app = NamsdecApp {
        show_re_tab: false,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.execute_palette_command(&ctx, PaletteCommand::GoTab(SidebarTab::Re));
    assert!(
        !app.show_re_tab,
        "aller quelque part ne doit pas activer une vue désactivée"
    );
}

/// La palette de commandes montre tout ce qu'elle a, sans barre de
/// défilement, tant que l'écran le permet — et ses lignes occupent
/// toute sa largeur.
///
/// « Tant que l'écran le permet » est la clause qui compte : une
/// quarantaine de commandes ne tiennent pas dans huit cents pixels, et
/// une barre a alors sa place. Ce qu'on refuse, c'est la barre sur un
/// écran qui aurait pu tout montrer.
///
/// Elle avait une hauteur figée de 340 px : la liste défilait dans une
/// fenêtre à moitié vide, et sa barre tombait au milieu parce que la
/// zone se serrait à gauche.
#[test]
fn the_command_palette_shows_everything_it_can_without_scrolling() {
    let mut app = NamsdecApp {
        command_palette: Some(CommandPalette::default()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            // Assez haut pour les tenir toutes, quel qu'en soit le
            // nombre : une hauteur en dur devenait fausse dès qu'on
            // ajoutait une commande, et le test tombait alors sans rien
            // révéler. Ce qu'on vérifie n'est pas qu'elles tiennent dans
            // 1400 px, c'est qu'elles ne défilent pas quand il y a la
            // place.
            egui::vec2(
                1280.0,
                palette_commands(strings(Lang::Fr), true, true).len() as f32 * 40.0 + 200.0,
            ),
        )),
        ..Default::default()
    };

    let mut barres: Vec<egui::Rect> = Vec::new();
    let mut premiere = None;
    let mut derniere = None;
    for _ in 0..3 {
        barres.clear();
        premiere = None;
        derniere = None;
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(text) => {
                    let s = strings(Lang::Fr);
                    if text.galley.text() == s.action_assemble {
                        premiere = Some(text.pos);
                    }
                    if text.galley.text() == s.file_quit {
                        derniere = Some(text.pos);
                    }
                }
                egui::epaint::Shape::Rect(r) if r.rect.width() < 20.0 && r.rect.height() > 40.0 => {
                    barres.push(r.rect);
                }
                _ => {}
            }
        }
        output.textures_delta.clear();
    }

    // La première et la dernière commande sont toutes les deux peintes :
    // rien n'est resté sous le bord.
    assert!(premiere.is_some(), "la première commande doit être visible");
    assert!(
        derniere.is_some(),
        "la dernière commande aussi : c'est tout l'objet du changement"
    );
    // Et aucune barre de défilement dans la palette — elle est centrée,
    // donc au milieu de l'écran.
    let fautives: Vec<_> = barres
        .iter()
        .filter(|r| r.min.x > 400.0 && r.max.x < 900.0)
        .collect();
    assert!(
        fautives.is_empty(),
        "la palette ne doit pas défiler quand tout tient : {fautives:?}"
    );
}

/// Filtrée, la palette rétrécit : quatre thèmes ne se lisent pas au
/// milieu d'une fenêtre de huit cents pixels.
#[test]
fn the_command_palette_shrinks_to_what_it_shows() {
    let cadre = |query: &str| -> f32 {
        let mut app = NamsdecApp {
            command_palette: Some(CommandPalette {
                query: query.to_owned(),
                ..Default::default()
            }),
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
        let mut hauteur = 0.0f32;
        for _ in 0..3 {
            hauteur = 0.0;
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Rect(r) = &clipped.shape {
                    // Le cadre de la palette : large, et centré.
                    if r.rect.width() > 400.0 && r.rect.width() < 700.0 {
                        hauteur = hauteur.max(r.rect.height());
                    }
                }
            }
            output.textures_delta.clear();
        }
        hauteur
    };

    let tout = cadre("");
    let filtre = cadre("Thème");
    assert!(tout > 400.0, "la palette entière est haute : {tout}");
    assert!(
        filtre < tout / 2.0,
        "filtrée, elle doit se replier : {filtre} contre {tout}"
    );
}

/// La palette promet « toutes les commandes » : elle les a toutes.
///
/// Une action qui n'y figure pas n'existe que pour qui connaît déjà son
/// raccourci — c'est-à-dire pour personne, le premier jour.
#[test]
fn the_palette_offers_every_action_there_is() {
    let s = strings(Lang::Fr);
    let commandes = palette_commands(s, true, true);
    for action in Action::ALL {
        assert!(
            commandes
                .iter()
                .any(|(_, command)| matches!(command, PaletteCommand::Do(a) if *a == action)),
            "{action:?} n'est proposée nulle part dans la palette"
        );
    }
    // Et chaque entrée porte un libellé : une ligne vide ne se cherche
    // pas.
    for (label, _) in &commandes {
        assert!(!label.trim().is_empty());
    }
}

/// Réinitialiser remet tout ce qui décrivait la course précédente.
///
/// Le processeur repartait bien du point d'entrée, mais le reste de
/// l'écran ne bougeait pas : les diagnostics du binaire restaient
/// affichés avec leur compte, l'onglet restait ouvert dessus, et le
/// listing ne quittait pas la ligne où l'exécution s'était arrêtée. Le
/// bouton avait l'air de n'avoir rien fait.
#[test]
fn reset_clears_what_the_previous_run_left_behind() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&octets, "ls"), "l'ELF se charge");

    // Une séance ordinaire : on avance, puis on frappe Assembler par
    // habitude — ce que namsdec refuse sur un listing de binaire.
    for _ in 0..50 {
        app.dispatch(Action::StepInto);
    }
    app.dispatch(Action::Assemble);
    app.console_tab = ConsoleTab::Diagnostics;
    app.follow_rip = false;
    let entree = namsdec_asm::elf::parse(&octets).expect("un ELF").entry;
    assert!(
        matches!(app.status, Status::Error(_)),
        "le refus de réassembler est bien affiché"
    );
    assert_ne!(
        app.emulator.as_ref().expect("un émulateur").cpu().regs.rip,
        entree,
        "l'exécution a quitté le point d'entrée"
    );

    app.dispatch(Action::Reset);

    assert_eq!(
        app.emulator.as_ref().expect("un émulateur").cpu().regs.rip,
        entree,
        "le programme repart de son point d'entrée"
    );
    assert!(
        matches!(app.status, Status::Idle),
        "obtenu {:?}",
        app.status
    );
    assert!(
        app.diagnostics.is_empty(),
        "les diagnostics du binaire s'en vont comme au chargement : {:?}",
        app.diagnostics
    );
    assert_eq!(
        app.console_tab,
        ConsoleTab::Output,
        "l'onglet ne reste pas ouvert sur des diagnostics qui n'existent plus"
    );
    assert!(app.follow_rip, "et le listing retourne au point d'entrée");
    assert_eq!(app.run_steps_taken, 0);
}

/// La palette mène aux étiquettes du programme, par leur nom.
///
/// Sur deux cents lignes, « où est donc `affiche_nombre` ? » se répond
/// en tapant son nom — comme on ouvre un fichier dans un éditeur. Rien
/// de nouveau à retenir : c'est la palette qu'on connaît déjà.
#[test]
fn the_palette_leads_to_the_labels_of_the_program() {
    let source = "global _start\nsection .text\n_start:\n    call affiche\n    hlt\n\
                  affiche:\n.local:\n    ret\n";
    let étiquettes = source_labels(source);
    assert_eq!(
        étiquettes,
        vec![("_start".to_owned(), 3), ("affiche".to_owned(), 6)],
        "les étiquettes locales ne comptent pas, ni les lignes indentées"
    );

    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: source.to_owned(),
        command_palette: Some(CommandPalette {
            query: "affiche".to_owned(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    let mut peint = String::new();
    for _ in 0..2 {
        peint.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                peint.push_str(text.galley.text());
                peint.push('|');
            }
        }
        output.textures_delta.clear();
    }
    assert!(
        peint.contains("Aller à : affiche (ligne 6)"),
        "l'étiquette cherchée doit être proposée :\n{peint}"
    );

    // Et la commande y emmène.
    app.execute_palette_command(&ctx, PaletteCommand::GoLine(6));
    assert_eq!(app.goto_line, Some(6), "la palette pose le curseur là-bas");
}

/// Sur un écran ordinaire, la palette défile — et c'est le bon repli.
///
/// Elle porte cinquante-deux commandes : aucune fenêtre courante ne les
/// tient toutes. Le test voisin vérifie qu'on ne défile pas *quand il y a
/// la place* ; celui-ci vérifie l'autre moitié, qui est le cas réel :
/// quand il n'y en a pas, la liste défile au lieu de couper la fin.
#[test]
fn on_a_real_screen_the_palette_scrolls_rather_than_cutting_the_end() {
    let mut app = NamsdecApp {
        command_palette: Some(CommandPalette::default()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    // La hauteur d'un écran d'ordinateur portable ordinaire.
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };

    let s = strings(Lang::Fr);
    let mut première = None;
    let mut fond = 0.0f32;
    for _ in 0..3 {
        première = None;
        fond = 0.0;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                if texte.galley.text() == s.action_assemble {
                    première = Some(texte.pos);
                }
                fond = fond.max(texte.pos.y);
            }
        }
        sortie.textures_delta.clear();
    }

    // La première commande est là : la liste commence bien au début.
    assert!(première.is_some(), "la première commande doit être visible");
    // Et rien n'est peint sous le bord de l'écran — ce qui ne tient pas
    // est derrière la barre de défilement, non coupé dans le vide.
    assert!(
        fond <= 900.0,
        "quelque chose est peint sous le bord de l'écran, en y={fond}"
    );
}

/// Échap ne déclenche jamais l'action secondaire d'une fenêtre.
///
/// La barre de dialogue répond `Cancel` à Échap. Tant que le second
/// bouton est un renoncement, c'est juste — mais la fenêtre
/// d'installation y avait mis « Recommencer », si bien qu'Échap lançait
/// une recompilation de vingt minutes au lieu de fermer.
///
/// La règle est donc : la barre porte le geste principal et le
/// renoncement, jamais une troisième action. Celle-ci se pose à côté et
/// se clique exprès.
#[test]
fn escape_never_triggers_a_second_action() {
    let mut app = NamsdecApp {
        show_installer_module: true,
        install_plan: Some("rizin-ghidra".to_owned()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1100.0));
    let mut sortie = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        },
        |ui| app.draw(ui),
    );
    sortie.textures_delta.clear();
    if app.install_plan.is_none() {
        return;
    }

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
    let mut sortie = ctx.run_ui(échap, |ui| app.draw(ui));
    sortie.textures_delta.clear();

    assert!(app.install_plan.is_none(), "Échap referme");
    assert!(
        app.install_job.is_none(),
        "et surtout ne lance rien : une recompilation de vingt minutes \
         ne se déclenche pas par la touche qui veut dire « je m'en vais »"
    );
}

/// Toute action a un raccourci, dans chacun des deux jeux.
///
/// Une action sans raccourci n'est pas forcément inaccessible — la
/// plupart ont aussi un bouton ou une entrée de menu —, mais elle est
/// invisible dans Préférences › Raccourcis, où l'on vient justement
/// voir ce que namsdec sait faire. Et changer de jeu ne doit pas faire
/// disparaître une commande : les deux couvrent donc les mêmes trente.
#[test]
fn every_action_has_a_shortcut_in_both_presets() {
    for preset in crate::keybindings::Preset::ALL {
        let jeu = KeyBindings::preset(preset);
        let sans: Vec<Action> = Action::ALL
            .into_iter()
            .filter(|a| !jeu.map.contains_key(a))
            .collect();
        assert!(
            sans.is_empty(),
            "jeu « {} » : {} action(s) sans raccourci — {sans:?}",
            preset.id(),
            sans.len()
        );
        // Et deux actions ne partagent pas la même frappe : la seconde
        // ne se déclencherait jamais.
        let mut vus: std::collections::BTreeMap<String, Action> = Default::default();
        for (action, chord) in &jeu.map {
            let touches = format!("{chord:?}");
            if let Some(déjà) = vus.insert(touches.clone(), *action) {
                panic!(
                    "jeu « {} » : {déjà:?} et {action:?} partagent {touches}",
                    preset.id()
                );
            }
        }
    }
}

/// Le jeu de raccourcis s'exporte **en entier**, à plat.
///
/// `keymap.json` ne garde que les écarts, et c'est ce qu'on veut d'un
/// fichier de réglages : il suit le jeu quand celui-ci gagne une
/// action. Mais un fichier de deux lignes ne dit pas ce que namsdec
/// sait faire, et l'on n'avait rien à quoi se raccrocher pour partir
/// d'une liste complète.
#[test]
fn the_whole_shortcut_set_can_be_exported_flat() {
    let dir = tempfile::tempdir().expect("un dossier temporaire");
    let chemin = dir.path().join("keymap.json");
    let mut app = NamsdecApp::default();
    app.export_keymap(&chemin);
    assert!(matches!(app.status, Status::Note(_)), "{:?}", app.status);

    let écrit = std::fs::read_to_string(&chemin).expect("le fichier est écrit");
    let relu: settings::Keymap = serde_json::from_str(&écrit).expect("du JSON valide");
    assert_eq!(
        relu.shortcuts.len(),
        Action::ALL.len(),
        "les trente actions y sont, une par une"
    );
    // Et il décrit exactement le jeu courant : relu, il rend les mêmes
    // raccourcis.
    assert_eq!(relu.bindings().map, app.keybindings.map);
}

/// `keymap.json` modifié pendant que namsdec tourne est repris.
///
/// C'est un fichier qu'on donne à éditer ; il n'était relu qu'au
/// lancement suivant. On l'éditait, on essayait la touche, il ne se
/// passait rien — et rien ne disait pourquoi.
#[test]
fn a_keymap_edited_on_disk_is_picked_up_while_running() {
    let mut app = NamsdecApp::default();
    let avant = app.keybindings.map.clone();

    // Le fichier n'a pas de date connue : la première visite en prend
    // une sans rien changer.
    app.keymap_mtime = None;
    app.keymap_checked = None;
    app.reload_keymap_if_changed();

    // Une capture de raccourci en cours suspend la reprise : changer le
    // jeu sous les doigts de quelqu'un qui appuie serait la dernière
    // chose à faire.
    app.rebinding = Some(keybindings::Rebinding::new(
        Action::Assemble,
        keybindings::RebindMode::Simple,
    ));
    app.keymap_checked = None;
    app.keymap_mtime = Some(std::time::SystemTime::UNIX_EPOCH);
    app.reload_keymap_if_changed();
    assert_eq!(
        app.keymap_mtime,
        Some(std::time::SystemTime::UNIX_EPOCH),
        "pendant une capture, on ne regarde même pas"
    );
    app.rebinding = None;

    // Et l'on ne se réveille pas plus d'une fois par seconde.
    app.keymap_checked = Some(std::time::Instant::now());
    app.keymap_mtime = Some(std::time::SystemTime::UNIX_EPOCH);
    app.reload_keymap_if_changed();
    assert_eq!(
        app.keymap_mtime,
        Some(std::time::SystemTime::UNIX_EPOCH),
        "l'intervalle est respecté : pas d'accès au disque à chaque image"
    );
    assert_eq!(app.keybindings.map, avant);
}

/// La palette propose vraiment **toutes** les commandes.
///
/// Elle le promet — « toutes les commandes » — et le code le dit déjà :
/// une commande que la palette ne propose pas n'existe que pour qui
/// connaît déjà son raccourci. Rien ne l'imposait, et une action
/// ajoutée demain n'y entrerait pas d'elle-même.
#[test]
fn the_palette_offers_every_single_action() {
    let s = strings(Lang::Fr);
    // Tout allumé : les modules facultatifs ajoutent des entrées, et
    // c'est l'inventaire complet qu'on veut contrôler.
    let commandes = palette_commands(s, true, true);
    let proposées: std::collections::BTreeSet<String> = commandes
        .iter()
        .filter_map(|(_, c)| match c {
            PaletteCommand::Do(action) => Some(format!("{action:?}")),
            _ => None,
        })
        .collect();
    let manquantes: Vec<String> = Action::ALL
        .into_iter()
        .filter(|a| !proposées.contains(&format!("{a:?}")))
        .map(|a| format!("{a:?}"))
        .collect();
    assert!(
        manquantes.is_empty(),
        "{} action(s) absente(s) de la palette : {manquantes:?}",
        manquantes.len()
    );
    // Et aucune entrée ne porte un libellé vide : on ne cherche pas une
    // commande qui n'a pas de nom.
    for (libellé, _) in &commandes {
        assert!(
            !libellé.trim().is_empty(),
            "une entrée de la palette sans libellé ne se cherche pas"
        );
    }
}

/// Une commande grisée dit pourquoi elle l'est.
///
/// `on_hover_text` ne s'affiche PAS sur un widget éteint — egui réserve
/// l'infobulle ordinaire aux widgets vivants. La barre d'outils portait
/// donc cinq boutons gris — Exécuter, Step Into, Step Over, Step Back,
/// Stop — qui ne disaient rien du tout au survol. C'est l'écran qu'on a
/// sous les yeux tant qu'on n'a pas assemblé, et une commande éteinte
/// qui se tait passe pour cassée.
///
/// Dire ce qui manque, c'est enseigner l'ordre des gestes : « Rien à
/// exécuter : assemblez d'abord le programme. » C'est ce que namsdec est
/// là pour faire.
///
/// Le contrôle lit le source, parce que la règle est là. Il porte sur
/// les conditions qui parlent de **l'état du programme** — `can_exec`,
/// `can_undo`, `self.running`, `self.emulator.is_some()` : ce sont
/// celles dont la réponse s'apprend. Une entrée grisée parce que la
/// liste des récents est vide se comprend sans phrase.
#[test]
fn a_greyed_command_says_why() {
    let source = include_str!("../main.rs");
    let lignes: Vec<&str> = source.lines().collect();
    let conditions = [
        "add_enabled_ui(can_exec",
        "add_enabled_ui(can_undo",
        "add_enabled_ui(self.running",
        "add_enabled_ui(self.emulator.is_some()",
    ];
    let mut muettes = Vec::new();
    for (n, ligne) in lignes.iter().enumerate() {
        if !conditions.iter().any(|c| ligne.contains(c)) {
            continue;
        }
        // CHAQUE bouton du bloc, et non le bloc entier : un premier
        // essai se contentait d'un `on_disabled_hover_text` quelque part
        // dans les trente lignes qui suivent, et il passait au vert avec
        // le défaut remis — les voisins du bouton muet avaient le leur.
        //
        // On repère donc chaque `.clicked()` du bloc et l'on remonte
        // jusqu'au début de sa chaîne, c'est-à-dire jusqu'au `if` qui
        // l'ouvre. L'explication doit être dans cette chaîne-là.
        let fin = (n + 30).min(lignes.len());
        for (m, l) in lignes[n..fin].iter().enumerate() {
            if !l.trim_start().starts_with(".clicked()") && !l.contains(".clicked()") {
                continue;
            }
            let mut début = n + m;
            while début > n && !lignes[début].contains("if ") {
                début -= 1;
            }
            let chaîne = lignes[début..=(n + m)].join("\n");
            if !chaîne.contains("on_disabled_hover_text") {
                muettes.push(format!(
                    "main.rs:{} {} (sous « {} »)",
                    début + 1,
                    lignes[début].trim(),
                    ligne.trim()
                ));
            }
        }
    }
    assert!(
        muettes.is_empty(),
        "{} commande(s) grisée(s) qui ne disent pas pourquoi :\n  {}",
        muettes.len(),
        muettes.join("\n  ")
    );
}

/// Toute action a un chemin qui se VOIT.
///
/// Elle a déjà un raccourci dans les deux jeux, et la palette les offre
/// toutes — mais la palette est un filet, pas un chemin : on l'ouvre
/// quand on sait déjà ce qu'on cherche. Un menu, lui, se parcourt.
/// C'est là qu'on découvre ce qu'un programme sait faire.
///
/// HUIT ACTIONS SUR TRENTE n'avaient que le clavier : dupliquer une
/// ligne, surligner les occurrences, aller à la suivante et à la
/// précédente, les renommer toutes, nommer une adresse, la commenter,
/// revenir au fichier précédent. Toutes utiles, toutes invisibles.
///
/// Ce contrôle est le pendant, du côté de l'œil, de « toute action a un
/// raccourci dans les deux jeux ». Il est né d'une série de défauts que
/// le banc ne pouvait pas voir — l'entrée « Fermer » absente du menu, la
/// barre d'onglets invisible, le « + » qui faisait autre chose — et qui
/// étaient tous la même chose : le programme savait faire, et rien ne le
/// disait.
#[test]
fn every_action_has_a_path_you_can_see() {
    let source = include_str!("../menus.rs");
    let lignes: Vec<&str> = source.lines().collect();
    // Les bornes des deux fonctions qui portent ce qui se voit en
    // permanence : la barre de menus et la barre d'outils.
    //
    // La visibilité fait partie du nom : sorties dans leur module,
    // elles s'écrivent `pub(crate) fn`, et chercher `fn menu_bar_ui(`
    // en tête de ligne ne trouvait plus rien.
    let entête = |l: &str, nom: &str| {
        let l = l.trim_start();
        l.strip_prefix("pub(crate) ")
            .or_else(|| l.strip_prefix("pub "))
            .unwrap_or(l)
            .starts_with(&format!("fn {nom}("))
    };
    let borne = |nom: &str| -> (usize, usize) {
        let début = lignes
            .iter()
            .position(|l| entête(l, nom))
            .unwrap_or_else(|| panic!("la fonction {nom} existe"));
        let suivante = |l: &&str| {
            let t = l.trim_start();
            (l.starts_with("    ") || !l.starts_with(' '))
                && t.strip_prefix("pub(crate) ")
                    .or_else(|| t.strip_prefix("pub "))
                    .unwrap_or(t)
                    .starts_with("fn ")
        };
        let fin = lignes[début + 1..]
            .iter()
            .position(suivante)
            .map(|d| début + 1 + d)
            .unwrap_or(lignes.len());
        (début, fin)
    };
    let (m0, m1) = borne("menu_bar_ui");
    let (t0, t1) = borne("toolbar_ui");
    let visible = format!(
        "{}\n{}",
        lignes[m0..m1].join("\n"),
        lignes[t0..t1].join("\n")
    );

    let sans: Vec<String> = Action::ALL
        .into_iter()
        .filter(|action| !visible.contains(&format!("Action::{action:?}")))
        .map(|action| format!("{action:?}"))
        .collect();
    assert!(
        sans.is_empty(),
        "{} action(s) qu'on ne peut trouver qu'au clavier — un menu se \
         parcourt, un raccourci se devine :\n  {}",
        sans.len(),
        sans.join("\n  ")
    );
}

/// Les bascules du menu Affichage font ce qu'elles annoncent.
///
/// Quatre cases commandent ce qu'on voit : la barre d'outils, le panneau
/// latéral, la barre de statut, la console. Aucun contrôle ne vérifiait
/// qu'elles MARCHENT — ils posaient l'état (`show_toolbar: false`) et
/// mesuraient ce qui était peint, ce qui passe au vert même si la case
/// est débranchée.
///
/// Celui-ci ouvre le menu, clique, et regarde. C'est plus lent et c'est
/// le seul qui réponde à la question qu'on se pose vraiment : « est-ce
/// que ça marche ? »
#[test]
fn the_view_menu_toggles_really_toggle() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let s = strings(Lang::Fr);
    let mut app = NamsdecApp::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui(move |ui| app.draw(ui));
    harness.run();

    // Chaque bascule, avec un témoin de la zone commandée — non la case
    // elle-même : c'est la zone qui doit partir, pas la case.
    //
    // On COMPTE les occurrences plutôt que d'en exiger une seule : un
    // même libellé sert souvent à deux endroits — « Assembler » est
    // dans la barre d'outils ET dans le menu qui la commande. Exiger
    // l'unicité ferait échouer le contrôle sur une coïncidence de
    // vocabulaire, ce qui n'apprend rien sur la bascule.
    let combien = |harness: &Harness<'_>, quoi: &str| -> usize {
        harness.get_all_by_label_contains(quoi).count()
    };
    // UNE seule bascule, et c'est un choix.
    //
    // Les trois autres se heurtent à deux limites qu'il vaut mieux
    // connaître que contourner :
    //
    //   - l'arbre d'accessibilité n'expose que ce qu'egui pose comme
    //     widget. La barre de statut et le panneau latéral, peints à la
    //     main, n'y figurent pas — leurs témoins sont introuvables ;
    //   - kittest cherche dans TOUT l'arbre, sans notion de « dans le
    //     menu ouvert ». « Console » nomme à la fois l'entrée du menu et
    //     l'onglet du bas, et la requête refuse de choisir.
    //
    // Un contrôle juste sur une bascule vaut mieux qu'un contrôle
    // fragile sur quatre. Le banc, lui, mesure ce qui est peint et
    // restreint son relevé à une fenêtre : les deux outils ne couvrent
    // pas la même moitié du problème.
    {
        let (bascule, témoin) = (s.view_toolbar, s.action_assemble);
        let avant = combien(&harness, témoin);
        assert!(avant > 0, "« {témoin} » doit être là au départ");

        harness.get_by_label(s.menu_view).click();
        harness.run();
        harness.get_by_label(bascule).click();
        harness.run();
        assert!(
            combien(&harness, témoin) < avant,
            "décocher « {bascule} » doit faire partir « {témoin} » \
             (toujours {} occurrence(s))",
            combien(&harness, témoin)
        );

        // Et le retour : une bascule qui ne rebascule pas est une porte
        // à sens unique, ce qui ne se voit qu'en essayant.
        harness.get_by_label(s.menu_view).click();
        harness.run();
        harness.get_by_label(bascule).click();
        harness.run();
        assert_eq!(
            combien(&harness, témoin),
            avant,
            "et la recocher doit le ramener"
        );
    }
}

/// Un raccourci qu'on a déplacé soi-même se voit, et dit ce qu'il a
/// remplacé.
///
/// Sans marque, le tableau ferait croire que le jeu livré propose ce
/// qu'on a soi-même posé — et l'on chercherait longtemps pourquoi la
/// même touche ne fait pas la même chose sur une autre machine.
#[test]
fn a_shortcut_one_moved_oneself_is_marked_as_such() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.show_preferences = true;
    app.preferences_tab = PreferencesTab::Shortcuts;

    // Rien de déplacé : aucune marque.
    let (_, avant) = ce_qui_est_écrit(&ctx, &mut app, Some("preferences_window"), 4);
    let s = strings(app.lang);
    assert!(
        !avant.iter().any(|(t, _)| t == s.shortcuts_changed),
        "un jeu intact ne porte aucune marque"
    );

    // On déplace « assembler » sur une frappe libre.
    app.keybindings.map.insert(
        Action::Assemble,
        crate::keybindings::Chord::Simple(egui::KeyboardShortcut::new(
            egui::Modifiers::ALT,
            egui::Key::Y,
        )),
    );
    let (_, après) = ce_qui_est_écrit(&ctx, &mut app, Some("preferences_window"), 4);
    let textes: Vec<&str> = après.iter().map(|(t, _)| t.as_str()).collect();
    assert!(
        textes.contains(&s.shortcuts_changed),
        "le déplacement se voit : {textes:?}"
    );
    // Et la nouvelle frappe est écrite, non celle du jeu.
    assert!(
        textes.iter().any(|t| t.contains('Y')),
        "la frappe posée est celle qui s'affiche : {textes:?}"
    );
}
