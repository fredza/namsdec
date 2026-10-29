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

/// Aucun raccourci n'est posé en dur ailleurs que dans la table.
///
/// C'est l'invariant que la palette de commandes violait, seule, et
/// depuis le début : son Ctrl+Maj+P vivait dans un `consume_shortcut` au
/// milieu de la boucle d'affichage. Conséquences, toutes invisibles
/// depuis le code qui le posait — absent des deux jeux, absent du
/// tableau de Préférences, absent du fichier exporté, absent du manuel
/// qui lit `keybindings.rs`. Le seul raccourci qu'on ne pouvait pas
/// déplacer était celui qui sert à trouver tous les autres.
///
/// Deux fichiers ont le droit d'en fabriquer : `keybindings.rs`, qui
/// POSE les jeux, et `settings.rs`, qui les relit depuis le fichier de
/// l'utilisateur.
#[test]
fn aucun_raccourci_n_est_pose_en_dur_hors_de_la_table() {
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let permis = ["keybindings.rs", "settings.rs"];
    let mut fautifs = Vec::new();
    let mut lus = 0;

    let mut à_voir = vec![dossier];
    while let Some(chemin) = à_voir.pop() {
        for entrée in std::fs::read_dir(&chemin).expect("le source du greffon se lit") {
            let entrée = entrée.expect("une entrée de dossier").path();
            if entrée.is_dir() {
                à_voir.push(entrée);
                continue;
            }
            if entrée.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let nom = entrée
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_owned();
            if permis.contains(&nom.as_str()) {
                continue;
            }
            // Les contrôles fabriquent des frappes pour les jouer :
            // c'est leur métier.
            if entrée.components().any(|c| c.as_os_str() == "tests")
                || nom == "tests.rs"
                || nom == "banc.rs"
            {
                continue;
            }
            let texte = std::fs::read_to_string(&entrée).expect("un fichier source se lit");
            lus += 1;
            for (n, ligne) in texte.lines().enumerate() {
                // Une ligne ne se compte qu'une fois, même quand elle
                // porte les deux motifs — `consume_shortcut(&…::new(`
                // les porte tous les deux, et un rapport qui dit deux
                // fois la même ligne se lit mal.
                if ["consume_shortcut(", "KeyboardShortcut::new("]
                    .iter()
                    .any(|motif| ligne.contains(motif))
                {
                    fautifs.push(format!("{nom}:{} {}", n + 1, ligne.trim()));
                }
            }
        }
    }

    assert!(lus > 20, "le balayage doit voir tout le greffon : {lus}");
    assert!(
        fautifs.is_empty(),
        "{} frappe(s) posée(s) hors de la table des raccourcis — elles \
         n'apparaissent ni dans Préférences, ni dans le jeu exporté, ni \
         dans le manuel :\n  {}",
        fautifs.len(),
        fautifs.join("\n  ")
    );
}

/// La palette écrit, à droite de chaque commande, la frappe qui la
/// déclenche.
///
/// La palette est l'endroit où l'on cherche une action dont on ne
/// connaît pas le raccourci : c'est donc le seul endroit où l'apprendre
/// au bon moment. Sans cela, les trente-deux frappes du programme
/// n'existent que dans le tableau de Préférences — qu'on n'ouvre pas en
/// travaillant — et la palette reste le chemin le plus court pour
/// toujours.
///
/// Le relevé porte sur ce qui est PEINT, et vérifie la place : la frappe
/// est sur la même ligne que son libellé, et à sa droite.
#[test]
fn la_palette_ecrit_le_raccourci_de_chaque_commande() {
    let mut app = NamsdecApp {
        command_palette: Some(CommandPalette::default()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let s = strings(app.lang);
    let fr = app.lang == Lang::Fr;

    let (_, lignes) = ce_qui_est_écrit_sur(
        &ctx,
        &mut app,
        Some("command_palette"),
        3,
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 1600.0),
            )),
            ..Default::default()
        },
    );
    assert!(
        !lignes.is_empty(),
        "la palette n'est pas peinte — un contrôle qui ne voit pas son \
         sujet ne mesure rien"
    );

    // Trois actions aux trois formes de frappe : une séquence, une
    // touche de fonction, un Ctrl+Maj.
    let bindings = app.keybindings.clone();
    for (action, libellé) in [
        (Action::Run, s.action_run),
        (Action::Stop, s.action_stop),
        (Action::Assemble, s.action_assemble),
    ] {
        let frappe = bindings
            .map
            .get(&action)
            .expect("l'action a un raccourci")
            .display(&ctx, fr);
        let gauche = lignes
            .iter()
            .find(|(t, _)| t == libellé)
            .unwrap_or_else(|| panic!("« {libellé} » doit être peint dans la palette"));
        let droite = lignes
            .iter()
            .find(|(t, r)| t == &frappe && (r.center().y - gauche.1.center().y).abs() < 4.0)
            .unwrap_or_else(|| {
                panic!(
                    "« {frappe} » doit être écrit sur la ligne de « {libellé} » : {:?}",
                    lignes
                        .iter()
                        .filter(|(_, r)| (r.center().y - gauche.1.center().y).abs() < 4.0)
                        .collect::<Vec<_>>()
                )
            });
        assert!(
            droite.1.left() > gauche.1.right(),
            "« {frappe} » va à droite du libellé, sans le recouvrir : \
             {:?} contre {:?}",
            droite.1,
            gauche.1
        );
    }
}

/// La palette garde sa largeur, quoi que ses libellés contiennent.
///
/// Elle fait 520 px, le raccourci est collé au bord droit, et la liste
/// accueille aussi les étiquettes du programme — lesquelles viennent
/// d'un binaire et ne sont bornées par rien : un nom décoré de C++ fait
/// couramment quatre-vingts caractères. Un libellé qui prend la largeur
/// de son texte élargit la fenêtre entière, et la palette saute d'une
/// frappe à l'autre selon le fichier ouvert.
///
/// LE RELEVÉ PORTE SUR LE CADRE, et c'est le deuxième essai : mesurer
/// l'encre ne voyait rien, parce que ce qui déborde n'est pas rogné —
/// c'est la fenêtre qui cède. Le contrôle est éprouvé en remettant le
/// défaut : `wrap_mode(Extend)` sur la ligne, et le cadre passe de 520 à
/// 606 px.
#[test]
fn la_palette_garde_sa_largeur_quel_que_soit_le_libelle() {
    // La largeur du cadre, avec et sans une étiquette démesurée dans le
    // programme ouvert : c'est l'écart entre les deux qui est le défaut,
    // et une largeur absolue seule ne le dirait pas.
    let cadre = |source: &str| -> (f32, Vec<String>) {
        let mut app = NamsdecApp {
            command_palette: Some(CommandPalette::default()),
            source: source.to_owned(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 1600.0),
            )),
            ..Default::default()
        };
        let mut largeur = 0.0f32;
        let mut coupées = Vec::new();
        for _ in 0..3 {
            coupées.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            let frame = egui::AreaState::load(&ctx, egui::Id::new("command_palette"))
                .expect("la palette est affichée")
                .rect();
            largeur = frame.width();
            for clipped in &output.shapes {
                let egui::epaint::Shape::Text(text) = &clipped.shape else {
                    continue;
                };
                // Ce que peint la palette, et non ce qui se trouve
                // dessous.
                if !(frame.x_range().expand(2.0).contains(text.pos.x)
                    && frame.y_range().expand(2.0).contains(text.pos.y))
                {
                    continue;
                }
                let droite = text.pos.x + text.galley.size().x;
                if droite > clipped.clip_rect.right() + 0.5 {
                    coupées.push(format!(
                        "« {} » demande jusqu'à {droite:.2} px pour un bord à {:.2}",
                        text.galley.text(),
                        clipped.clip_rect.right()
                    ));
                }
            }
            output.textures_delta.clear();
        }
        (largeur, coupées)
    };

    let (nue, _) = cadre("section .text\n_start:\n  nop\n");
    // Un nom décoré de C++, tel qu'un binaire en porte.
    let (chargée, coupées) = cadre(
        "section .text\n_ZNSt6vectorIiSaIiEE13_M_insert_auxEN9__gnu_cxx17__normal_iteratorIPiS1_EERKi:\n  nop\n",
    );

    assert!(
        (nue - 520.0).abs() < 2.0,
        "la palette fait 520 px de large : {nue}"
    );
    assert!(
        (chargée - nue).abs() < 2.0,
        "une étiquette au nom démesuré élargit la palette : {chargée} \
         contre {nue} px"
    );
    assert!(
        coupées.is_empty(),
        "{} ligne(s) de la palette perdent du texte sous le bord :\n  {}",
        coupées.len(),
        coupées.join("\n  ")
    );
}

/// Les touches s'écrivent à l'écran comme le manuel les écrit.
///
/// Le générateur du guide traduit les noms d'egui depuis toujours —
/// « Espace », « / », « Maj » — parce que « Slash » dans une phrase ne
/// dit pas sur quoi appuyer. L'écran, lui, affichait le nom brut : on
/// lisait « Espace puis / » dans le PDF et « Space Slash » dans
/// l'application, pour la même frappe. Une notation de trop est une
/// notation qu'il faut traduire de tête au moment où l'on cherche son
/// raccourci.
///
/// Les deux tables sont maintenant confrontées l'une à l'autre.
#[test]
fn les_touches_s_ecrivent_comme_le_manuel_les_ecrit() {
    use crate::keybindings::{Chord, key_name, keys_display};
    use egui::Key;

    // La table du manuel, lue dans le générateur : c'est la source, et
    // elle ne doit pas se recopier à la main.
    let guide = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide.py"),
    )
    .expect("docs/guide.py — le manuel est versionné avec le code");
    let début = guide
        .find("def _touche(nom):")
        .expect("le manuel traduit les noms de touches");
    let fin = guide[début..]
        .find("\n\n\n")
        .map(|d| début + d)
        .unwrap_or(guide.len());
    let table = &guide[début..fin];

    // Chaque touche que le manuel traduit doit se lire pareil à l'écran.
    for (nom_egui, attendu) in [
        ("Space", "Espace"),
        ("Slash", "/"),
        ("Semicolon", ";"),
        ("Comma", ","),
        ("Period", "."),
        ("Minus", "-"),
        ("Equals", "="),
        ("OpenBracket", "["),
        ("CloseBracket", "]"),
    ] {
        assert!(
            table.contains(&format!("\"{nom_egui}\": \"{attendu}\""))
                || table.contains(&format!(
                    "\"{nom_egui}\": \"{}\"",
                    attendu.replace('\\', "\\\\")
                )),
            "le manuel n'écrit plus {nom_egui} « {attendu} » — les deux \
             tables ont divergé : {table}"
        );
        let touche = Key::from_name(nom_egui).expect("egui connaît cette touche");
        assert_eq!(
            key_name(touche, true),
            attendu,
            "l'écran doit écrire {nom_egui} comme le manuel"
        );
    }

    // Et la séquence entière s'assemble pareil des deux côtés : une
    // espace entre les touches, les lettres en minuscule. Le guide
    // écrivait « Espace puis S puis I » — plus explicite dans une
    // phrase, et introuvable à l'écran.
    let assemblage = {
        let début = guide
            .find("def raccourcis():")
            .expect("le manuel tire ses raccourcis du code");
        let fin = guide[début..]
            .find("\ndef ")
            .map(|d| début + d)
            .unwrap_or(guide.len());
        &guide[début..fin]
    };
    assert!(
        !assemblage.contains("\" puis \""),
        "le manuel sépare encore les touches d'une séquence par « puis » : l'écran écrit « Espace s i »"
    );
    assert!(
        assemblage.contains("_minuscule("),
        "le manuel doit écrire les lettres d'une séquence en minuscule, comme l'écran"
    );

    // La forme complète, telle qu'on la lit dans la prose du projet.
    assert_eq!(
        keys_display(&[Key::Space, Key::S, Key::I], true),
        "Espace s i",
        "les lettres d'une séquence sont en minuscule : une séquence \
         ignore les modificateurs, un « S » majuscule annoncerait un Maj \
         inutile"
    );
    assert_eq!(keys_display(&[Key::Y, Key::Y], true), "y y");
    assert_eq!(
        keys_display(&[Key::Space, Key::Slash, Key::Slash], true),
        "Espace / /"
    );

    // Et « Maj », pas « Shift », dans la version française.
    let ctx = egui::Context::default();
    let assembler = Chord::Simple(egui::KeyboardShortcut::new(
        egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
        Key::B,
    ));
    assert_eq!(assembler.display(&ctx, true), "Ctrl+Maj+B");
    assert_eq!(assembler.display(&ctx, false), "Ctrl+Shift+B");

    // Aucun raccourci des deux jeux ne laisse passer un nom
    // d'identificateur en français.
    let interdits = ["Shift", "Slash", "Space ", "Escape", "Backslash"];
    for preset in crate::keybindings::Preset::ALL {
        for (action, chord) in &crate::keybindings::KeyBindings::preset(preset).map {
            let écrit = chord.display(&ctx, true);
            for mot in interdits {
                assert!(
                    !écrit.contains(mot),
                    "{action:?} s'écrit « {écrit} » : « {mot} » n'est pas \
                     du français"
                );
            }
        }
    }
}

/// L'infobulle de l'export compte les raccourcis au lieu de les deviner.
///
/// Elle annonçait « les trente raccourcis » en dur, et il y en avait
/// trente-deux : le nombre avait été juste un jour, puis deux actions
/// sont arrivées et personne n'a relu la phrase. Un chiffre figé dans
/// une chaîne de langue vieillit sans bruit — il n'a ni compilateur ni
/// contrôle pour le rappeler.
///
/// Aucune chaîne ne doit donc écrire ce nombre en mots : il se compte
/// sur `Action::ALL`, à l'affichage.
#[test]
fn l_infobulle_de_l_export_compte_les_raccourcis() {
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        assert!(
            s.shortcuts_export_hint.contains("{}"),
            "{lang:?} : l'infobulle de l'export doit recevoir le compte, \
             et non le porter en dur : « {} »",
            s.shortcuts_export_hint
        );
        let écrit = s
            .shortcuts_export_hint
            .replace("{}", &Action::ALL.len().to_string());
        assert!(
            écrit.contains(&Action::ALL.len().to_string()),
            "{lang:?} : le compte doit s'écrire : « {écrit} »"
        );
        // Et aucun nombre de raccourcis écrit en mots, dans aucune
        // chaîne : c'est la forme que le compilateur ne voit pas
        // vieillir.
        for (nom, valeur) in [
            ("shortcuts_export_hint", s.shortcuts_export_hint),
            ("shortcuts_both", s.shortcuts_both),
            ("shortcuts_saved_hint", s.shortcuts_saved_hint),
        ] {
            for mot in ["trente", "thirty", "quarante", "forty"] {
                assert!(
                    !valeur.to_lowercase().contains(mot),
                    "{nom} fige un compte en mots (« {mot} ») : « {valeur} »"
                );
            }
        }
    }
}

/// Les frappes du menu Affichage s'alignent en colonne, et le menu ne
/// grandit pas pour autant.
///
/// `ui.checkbox` n'a pas de `shortcut_text` : la frappe est posée par un
/// `Atom::grow` entre le libellé et elle, ce qui la pousse au bord droit
/// de la case. Deux choses à mesurer, et une seule saute aux yeux.
///
/// CE QU'ON VOIT : les frappes doivent former une COLONNE. « À droite du
/// libellé » ne suffit pas — sans l'atome qui pousse, chaque frappe se
/// colle à son libellé et la colonne se disloque, « Panneau latéral »
/// étant deux fois plus long que « Console ». C'est l'alignement qui
/// permet de descendre le menu des yeux.
///
/// CE QU'ON NE VOIT PAS : une case à cocher occupe toute la largeur du
/// menu, et la largeur du menu se calcule sur son contenu le plus large.
/// **Un enfant qui réclame toute la place offerte dans un parent qui se
/// dimensionne sur ses enfants est la boucle la plus connue d'egui** —
/// la surface grandit de quelques pixels par image, sans fin. Le relevé
/// suit l'abscisse de la frappe sur vingt images ; alignée à droite,
/// elle se déplace d'autant que le menu s'élargit.
#[test]
fn les_frappes_du_menu_affichage_s_alignent_sans_elargir_le_menu() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let s = strings(Lang::Fr);
    let ctx_frappes = egui::Context::default();
    let jeu = KeyBindings::default();
    let frappe_de = |action| {
        jeu.map
            .get(&action)
            .expect("cette bascule a un raccourci")
            .display(&ctx_frappes, true)
    };
    let console = frappe_de(Action::ToggleConsole);
    let panneau = frappe_de(Action::ToggleSidebar);

    let mut app = NamsdecApp::default();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_ui(move |ui| app.draw(ui));
    harness.run();
    harness.get_by_label(s.menu_view).click();
    harness.run();

    // Tout ce qui est peint et qui nous intéresse, image par image.
    let relever = |harness: &Harness<'_>| -> Vec<(String, egui::Rect)> {
        let mut vus = Vec::new();
        for clipped in &harness.output().shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                vus.push((
                    t.galley.text().to_owned(),
                    egui::Rect::from_min_size(t.pos, t.galley.size()).intersect(clipped.clip_rect),
                ));
            }
        }
        vus
    };
    let premier = relever(&harness);
    let trouver = |quoi: &str| -> egui::Rect {
        premier
            .iter()
            .find(|(t, _)| t == quoi)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| {
                panic!(
                    "« {quoi} » doit être peint dans le menu ouvert : {:?}",
                    premier.iter().map(|(t, _)| t).collect::<Vec<_>>()
                )
            })
    };

    let mut droites = Vec::new();
    for (frappe, libellé) in [(&console, s.view_console), (&panneau, s.view_sidebar)] {
        let écrite = trouver(frappe);
        // Le libellé est peint DEUX FOIS pour la console — l'entrée du
        // menu et l'onglet du bas, à sept cents pixels de là. On prend
        // celui de la ligne de la frappe : c'est celui dont on parle.
        let texte = premier
            .iter()
            .find(|(t, r)| t == libellé && (r.center().y - écrite.center().y).abs() < 4.0)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| {
                panic!("« {libellé} » doit être peint sur la ligne de « {frappe} »")
            });
        assert!(
            écrite.left() > texte.right(),
            "« {frappe} » va à droite de « {libellé} », sans le recouvrir : \
             {écrite:?} contre {texte:?}"
        );
        droites.push((frappe.clone(), écrite.right()));
    }

    let écart = (droites[0].1 - droites[1].1).abs();
    assert!(
        écart < 1.0,
        "les frappes du menu doivent s'aligner à droite, en colonne : \
         {} finit à {:.2} px et {} à {:.2} — {écart:.2} px d'écart",
        droites[0].0,
        droites[0].1,
        droites[1].0,
        droites[1].1
    );

    // Vingt images, et les huit dernières au même endroit. Vingt parce
    // qu'egui AMÈNE une surface à sa taille au lieu de l'y poser : une
    // mise en place qui converge n'est pas une dérive.
    let mut abscisses = Vec::new();
    for _ in 0..20 {
        harness.step();
        if let Some((_, r)) = relever(&harness).into_iter().find(|(t, _)| t == &console) {
            abscisses.push(r.left());
        }
    }
    assert!(
        abscisses.len() >= 12,
        "la frappe doit rester peinte tout du long : {} image(s)",
        abscisses.len()
    );
    let fin = &abscisses[abscisses.len() - 8..];
    let dérive = fin
        .iter()
        .fold(0.0f32, |pire, x| pire.max((x - fin[0]).abs()));
    assert!(
        dérive < 0.5,
        "le menu s'élargit image après image — la frappe dérive de \
         {dérive:.2} px : {fin:?}"
    );
}

/// Les deux bascules d'affichage basculent, et reviennent.
///
/// Une bascule qui ne rebascule pas est une porte à sens unique : on
/// cache la console d'une frappe, et il faut le menu pour la retrouver.
/// C'est le défaut qu'on ne voit pas en écrivant le code, parce qu'on
/// essaie toujours l'aller.
#[test]
fn les_bascules_d_affichage_vont_et_reviennent() {
    let mut app = NamsdecApp::default();
    for (action, lire) in [
        (
            Action::ToggleConsole,
            (|a: &NamsdecApp| a.show_console) as fn(&NamsdecApp) -> bool,
        ),
        (Action::ToggleSidebar, |a: &NamsdecApp| a.show_sidebar),
    ] {
        let départ = lire(&app);
        app.dispatch(action);
        assert_ne!(lire(&app), départ, "{action:?} doit basculer");
        app.dispatch(action);
        assert_eq!(
            lire(&app),
            départ,
            "{action:?} doit revenir d'où elle vient"
        );
    }
}

/// Les suites possibles sont lues dans le jeu actif, et nulle part
/// ailleurs.
///
/// Le panneau pourrait montrer des noms de familles — `p` « les pas »,
/// `v` « voir » — comme le font les comentaires qui les ont posées. Ce
/// serait une seconde table à tenir en accord avec la première, et elle
/// mentirait dès le premier raccourci déplacé. Les touches QUI RESTENT
/// et l'action au bout ne peuvent pas mentir : elles sont le jeu.
#[test]
fn les_suites_possibles_sortent_du_jeu_et_suivent_ce_qu_on_y_change() {
    use crate::keybindings::{Chord, continuations};
    use egui::Key;

    let jeu = KeyBindings::preset(keybindings::Preset::Vim);

    // Rien d'armé, rien à montrer.
    assert!(continuations(&jeu, &[]).is_empty());

    // `Espace v` mène aux vues, et à elles seules.
    let vues = continuations(&jeu, &[Key::Space, Key::V]);
    let actions: Vec<Action> = vues.iter().map(|(_, a)| *a).collect();
    for attendue in [
        Action::ToggleExplorer,
        Action::ShowFlow,
        Action::ShowPreferences,
        Action::ToggleConsole,
        Action::ToggleSidebar,
    ] {
        assert!(
            actions.contains(&attendue),
            "{attendue:?} est sous « Espace v » : {actions:?}"
        );
    }
    // Et rien d'autre : une seule touche reste à taper, et c'est la
    // bonne.
    for (reste, action) in &vues {
        assert_eq!(reste.len(), 1, "{action:?} : il reste {reste:?}");
        assert_eq!(
            jeu.map.get(action),
            Some(&Chord::leader(&[Key::Space, Key::V, reste[0]])),
            "{action:?} doit s'atteindre par ce qui est montré"
        );
    }

    // Une séquence complète n'a pas de suite : elle vient de partir.
    assert!(continuations(&jeu, &[Key::Y, Key::Y]).is_empty());

    // Et tout cela SUIT ce qu'on déplace : un raccourci changé change
    // ce que le panneau montre, à l'image suivante.
    let mut déplacé = jeu.clone();
    déplacé.map.insert(
        Action::ToggleCalculator,
        Chord::leader(&[Key::Space, Key::V, Key::X]),
    );
    assert!(
        continuations(&déplacé, &[Key::Space, Key::V])
            .iter()
            .any(|(reste, a)| *a == Action::ToggleCalculator && reste == &[Key::X]),
        "le panneau montre le jeu, pas une table écrite à côté"
    );

    // La règle du délai se lit sans horloge.
    assert!(!keybindings::hint_is_due(
        keybindings::LEADER_HINT_DELAY - std::time::Duration::from_millis(1)
    ));
    assert!(keybindings::hint_is_due(keybindings::LEADER_HINT_DELAY));
}

/// Quand une séquence est armée et qu'on hésite, le programme dit quoi
/// taper.
///
/// La barre d'état disait « touche suivante attendue… » sans dire
/// LESQUELLES. C'est l'endroit exact où un outil qui sert à apprendre
/// doit parler : on vient de taper `Espace`, on ne sait plus ce qui
/// suit, et le seul recours était d'abandonner la séquence pour ouvrir
/// Préférences.
///
/// Le relevé est de bout en bout : la frappe entre par les évènements,
/// le dispatcher s'arme, et l'on mesure ce qui est PEINT. La date
/// d'armement est reculée d'une seconde pleine — on ne compare pas deux
/// instants pris à deux lignes d'intervalle, ce qui passerait ici et
/// tomberait sur une machine chargée.
#[test]
fn une_sequence_armee_dit_ce_qu_on_peut_taper_ensuite() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let s = strings(Lang::Fr);
    let entrée = |events: Vec<egui::Event>| egui::RawInput {
        events,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let frappe = |key| {
        vec![
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };

    // Espace arme la séquence — par le vrai chemin, celui des touches.
    let mut sortie = ctx.run_ui(entrée(frappe(egui::Key::Space)), |ui| app.draw(ui));
    sortie.textures_delta.clear();
    assert_eq!(
        app.dispatcher.pending(),
        [egui::Key::Space],
        "Espace doit armer la séquence"
    );

    // Juste armée, rien ne doit paraître : qui tape vite ne voit rien
    // passer.
    // TROIS IMAGES, et c'est une règle d'egui qu'il vaut mieux
    // connaître : une surface ancrée ne peut pas se placer tant qu'elle
    // ignore sa taille — elle doit se mesurer d'abord. egui la dessine
    // donc INVISIBLE la première fois et redemande une image. Mesurer
    // une seule image ne montre rien, et ce n'est pas un défaut.
    let peint = |app: &mut NamsdecApp, ctx: &egui::Context| -> Vec<String> {
        let mut textes = Vec::new();
        for _ in 0..3 {
            textes.clear();
            let mut sortie = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1400.0, 900.0),
                    )),
                    ..Default::default()
                },
                |ui| app.draw(ui),
            );
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    textes.push(t.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        textes
    };

    // Puis on recule l'armement d'une seconde pleine : l'hésitation est
    // franche, et la mesure ne dépend pas de la vitesse de la machine.
    app.leader_since = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
    let après = peint(&mut app, &ctx);

    // Chaque suite de `Espace` est annoncée, touches ET action.
    let suites = keybindings::continuations(&app.keybindings, &[egui::Key::Space]);
    assert!(
        suites.len() > 4,
        "le jeu vim a plusieurs familles sous Espace : {}",
        suites.len()
    );
    for (reste, action) in &suites {
        let touches = keybindings::keys_display(reste, true);
        assert!(
            après.iter().any(|t| t == &touches),
            "« {touches} » doit être montré : {après:?}"
        );
        assert!(
            après.iter().any(|t| t == action_label(s, *action)),
            "« {} » doit être nommée en face de « {touches} »",
            action_label(s, *action)
        );
    }
}

/// Dans le panneau des suites, les noms d'actions s'alignent en
/// colonne.
///
/// Les restes de séquence n'ont pas tous la même longueur — et un
/// raccourci qu'on déplace peut n'en laisser qu'une touche là où son
/// voisin en laisse deux. Posés à la suite, les noms commençaient
/// chacun où finissait sa touche, et une colonne qui ondule ne se
/// descend pas des yeux. C'est le seul service que ce panneau rende :
/// se lire d'un coup d'œil.
#[test]
fn les_noms_d_actions_du_panneau_des_suites_s_alignent() {
    use crate::keybindings::Chord;

    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    // Un reste d'UNE touche à côté de restes de deux : le cas qui
    // disloque la colonne, et il suffit d'un raccourci déplacé pour
    // l'obtenir.
    app.keybindings.map.insert(
        Action::ToggleCalculator,
        Chord::leader(&[egui::Key::Space, egui::Key::K]),
    );
    app.dispatcher.arm(&[egui::Key::Space]);
    app.leader_since = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));

    let (cadre, lignes) = ce_qui_est_écrit_sur(
        &ctx,
        &mut app,
        Some("leader_hints"),
        3,
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 1000.0),
            )),
            ..Default::default()
        },
    );
    cadre.expect("le panneau des suites est affiché");
    let s = strings(Lang::Fr);
    let suites = keybindings::continuations(&app.keybindings, &[egui::Key::Space]);
    assert!(
        suites.iter().any(|(reste, _)| reste.len() == 1),
        "la scène doit contenir le cas qui disloque : un reste d'une touche"
    );

    let mut gauches: Vec<(String, f32)> = Vec::new();
    for (_, action) in &suites {
        let nom = action_label(s, *action);
        let posé = lignes
            .iter()
            .find(|(t, _)| t == nom)
            .unwrap_or_else(|| panic!("« {nom} » doit être peint dans le panneau"));
        gauches.push((nom.to_owned(), posé.1.left()));
    }
    let premier = gauches[0].1;
    let fautifs: Vec<&(String, f32)> = gauches
        .iter()
        .filter(|(_, x)| (x - premier).abs() > 0.5)
        .collect();
    assert!(
        fautifs.is_empty(),
        "les noms d'actions doivent partir de la même abscisse ({premier:.2}) : \
         {fautifs:?}"
    );
}

/// Le panneau des suites ne recouvre jamais la barre d'état ni la
/// console.
///
/// Il s'ancre en bas à gauche, et c'est là que vivent les deux panneaux
/// qui disent où l'on en est. Un décalage en dur suffisait tant que la
/// console restait fermée ; dès qu'on l'ouvre, le panneau se poserait
/// par-dessus ce qu'on venait d'y lire. La hauteur est donc relue à
/// l'image.
#[test]
fn le_panneau_des_suites_reste_au_dessus_des_barres_du_bas() {
    for avec_console in [false, true] {
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            show_console: avec_console,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1400.0, 900.0),
            )),
            ..Default::default()
        };
        let mut sortie = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::Space,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..entrée.clone()
            },
            |ui| app.draw(ui),
        );
        sortie.textures_delta.clear();
        app.leader_since = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));

        // Trois images : les panneaux du bas prennent leur hauteur à la
        // première, le panneau la lit à la suivante.
        let mut cadre = None;
        let mut barres = Vec::new();
        for _ in 0..3 {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            sortie.textures_delta.clear();
            cadre = egui::AreaState::load(&ctx, egui::Id::new("leader_hints")).map(|a| a.rect());
            barres.clear();
            for id in ["statusbar", "console"] {
                if let Some(p) = egui::containers::panel::PanelState::load(&ctx, egui::Id::new(id))
                {
                    barres.push((id, p.outer_rect.height()));
                }
            }
        }
        let cadre = cadre.expect("le panneau des suites est affiché");
        let pris: f32 = barres
            .iter()
            .filter(|(id, _)| *id == "statusbar" || avec_console)
            .map(|(_, h)| h)
            .sum();
        let haut_des_barres = 900.0 - pris;
        assert!(
            cadre.bottom() <= haut_des_barres + 0.5,
            "console {avec_console} : le panneau descend à {:.2} px, les \
             barres commencent à {haut_des_barres:.2} ({barres:?})",
            cadre.bottom()
        );
    }
}

/// Les suites se rangent dans l'ordre où elles s'écrivent.
///
/// Trouvé en REGARDANT la capture, et par rien d'autre : le tri portait
/// sur `Key::name()`, où la barre oblique s'appelle « Slash » — elle se
/// rangeait donc entre `s` et `u`, au milieu des lettres, alors qu'on la
/// voit écrite `/`. Aucune mesure ne l'avait demandé, et une colonne
/// dont l'ordre ne se devine pas se relit ligne à ligne au lieu de se
/// parcourir.
#[test]
fn les_suites_se_rangent_dans_l_ordre_ou_elles_s_ecrivent() {
    let jeu = KeyBindings::default();
    let suites = keybindings::continuations(&jeu, &[egui::Key::Space]);
    let écrites: Vec<String> = suites
        .iter()
        .map(|(reste, _)| keybindings::keys_display(reste, true))
        .collect();

    let mut rangées = écrites.clone();
    rangées.sort();
    assert_eq!(
        écrites, rangées,
        "la colonne doit se parcourir dans l'ordre où elle s'écrit"
    );
    // Et le cas qui l'a révélé : la ponctuation ne se glisse pas entre
    // deux lettres.
    let première_lettre = écrites
        .iter()
        .position(|t| t.starts_with(|c: char| c.is_ascii_alphabetic()))
        .expect("il y a des lettres");
    let dernière_ponctuation = écrites
        .iter()
        .rposition(|t| !t.starts_with(|c: char| c.is_ascii_alphabetic()));
    if let Some(fin) = dernière_ponctuation {
        assert!(
            fin < première_lettre,
            "« {} » est rangée au milieu des lettres : {écrites:?}",
            écrites[fin]
        );
    }
}

/// La légende de chaleur accorde son unité à son dernier repère.
///
/// Le relevé général des comptes ne pouvait pas l'attraper : le chiffre
/// et l'unité sont DEUX étiquettes peintes séparément, et il ne lit
/// qu'un texte à la fois. C'est la capture qui l'a montré — « Chaleur :
/// ▪ 1 passages par ligne », sur le programme d'accueil, c'est-à-dire
/// sur le premier écran venu.
#[test]
fn la_legende_de_chaleur_accorde_son_unite() {
    use crate::listing::heat_legend_steps;

    // Un seul passage : un seul repère, et l'unité au singulier.
    assert_eq!(heat_legend_steps(1), vec![1]);
    // Plusieurs : le dernier repère est le maximum.
    let plusieurs = heat_legend_steps(1000);
    assert!(plusieurs.len() > 1 && plusieurs.last() != Some(&1));

    let s = strings(Lang::Fr);
    assert_eq!(s.heat_legend_hint_one, "passage par ligne");
    assert_eq!(s.heat_legend_hint, "passages par ligne");
    assert_ne!(
        s.heat_legend_hint_one, s.heat_legend_hint,
        "les deux formes doivent différer, sinon l'accord ne sert à rien"
    );
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        assert!(!s.heat_legend_hint_one.trim().is_empty());
        assert!(!s.heat_legend_hint.trim().is_empty());
    }
}
