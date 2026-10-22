//! Ce qu'on ne refait pas : les relevés gardés et la dichotomie.
//!
//! Des contrôles qui comptent — des poses de texte, des relevés, des
//! rangées —, non des millisecondes : une durée dirait autre chose sur une
//! autre machine.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

// ============================================================================
//  CE QU'ON NE PARCOURT PLUS : les relevés gardés, et la dichotomie
//
//  Quatre défauts mesurés le 14 octobre 2026 avaient la même forme : un
//  travail proportionnel au LISTING ENTIER, refait à chaque image, pour
//  en montrer trente lignes. Les contrôles qui suivent retiennent la
//  correction par ce qui se compte — des relevés, des rangées — et non
//  par des millisecondes, qui diraient autre chose sur une autre
//  machine.
// ============================================================================

/// Un binaire ouvert fait quinze mille lignes, et l'éditeur doit rester
/// utilisable dessus.
///
/// Deux choses coûtaient une frame entière : recolorer tout le texte à
/// chaque passage — deux fois par frame, pour la gouttière puis pour le
/// champ — et peindre un numéro de ligne pour chacune des quinze mille,
/// quand trente sont à l'écran.
#[test]
fn a_large_listing_still_draws_fast() {
    let Ok(bytes) = std::fs::read("/bin/ls") else {
        eprintln!("/bin/ls introuvable : test ignoré");
        return;
    };
    let mut app = NamsdecApp::default();
    app.load_binary(bytes, "programme");
    assert!(
        app.source.lines().count() > 5_000,
        "le listing doit être gros pour que la mesure ait un sens : {}",
        app.source.lines().count()
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
    // Une frame pour chauffer les caches, puis la mesure.
    let mut premier = ctx.run_ui(input.clone(), |ui| app.draw(ui));
    premier.textures_delta.clear();

    let start = std::time::Instant::now();
    const FRAMES: u32 = 5;
    for _ in 0..FRAMES {
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        output.textures_delta.clear();
    }
    let par_frame = start.elapsed() / FRAMES;
    // Large : en debug la mesure est plusieurs fois plus lente qu'en
    // release, et une machine chargée l'est encore plus. Le seuil
    // attrape une régression d'ordre de grandeur — celle qui rendrait
    // l'éditeur saccadé — pas une variation de quelques pour cent.
    assert!(
        par_frame < std::time::Duration::from_millis(100),
        "une frame prend {par_frame:?} sur {} lignes",
        app.source.lines().count()
    );
}

/// Une frame lente ne dit pas la même chose selon ce qui l'a ralentie.
///
/// Ouvrir un binaire de soixante mille lignes demande une demi-seconde
/// de mise en page — **une fois**. Le cache porte ensuite, et les frames
/// suivantes tiennent en deux millisecondes. Crier « frame lente » sans
/// le dire faisait chercher une fuite là où il n'y avait qu'un
/// chargement.
#[test]
fn a_slow_frame_says_whether_it_will_come_back() {
    use std::time::Duration;

    let étapes = [
        ("éditeur", Duration::from_millis(440)),
        ("panneaux", Duration::from_millis(2)),
    ];
    let vite = Duration::from_millis(3);
    let lente = Duration::from_millis(450);

    // En marche normale, rien : l'instrumentation est silencieuse.
    assert_eq!(slow_frame_note(vite, &étapes, false), None);
    assert_eq!(slow_frame_note(vite, &étapes, true), None);

    // Lente, et le texte vient de changer : c'est une mise en page.
    let note = slow_frame_note(lente, &étapes, true).expect("il y a à dire");
    assert!(note.contains("mise en page"), "{note}");
    assert!(note.contains("payé une fois"), "{note}");
    assert!(note.contains("éditeur 440ms"), "{note}");

    // Lente sans que rien n'ait changé : là, c'est un vrai reproche.
    let note = slow_frame_note(lente, &étapes, false).expect("il y a à dire");
    assert!(note.contains("frame lente"), "{note}");
    assert!(!note.contains("payé une fois"), "{note}");

    // Les étapes trop courtes ne noient pas le message, et l'absence de
    // toute étape mesurée se dit plutôt que de laisser une parenthèse
    // vide.
    let note = slow_frame_note(lente, &[], false).expect("il y a à dire");
    assert!(note.contains("hors des étapes mesurées"), "{note}");
}

/// L'état des outils est relevé une fois, et non à chaque image.
///
/// Savoir si `rizin` est complet demande de **le lancer** : `rizin` sans
/// `rz-ghidra` a le programme et pas la commande, et namsdec le lui
/// demande plutôt que de le supposer. Cette réponse était redemandée à
/// chaque image, si bien qu'ouvrir Préférences › Outils lançait `rizin`
/// soixante fois par seconde.
///
/// On l'a vu par un test instable : le contrôle de la fenêtre
/// d'installation demandait la réponse, la fenêtre la redemandait, et
/// les deux pouvaient différer sous la charge. Un test qui échoue au
/// hasard apprend à ignorer la suite.
#[test]
fn what_the_tools_answer_is_asked_once_and_kept() {
    let mut app = NamsdecApp::default();
    assert!(app.tools_ready.is_empty(), "rien n'est demandé sans raison");

    // Première demande : le relevé se remplit, pour tous les outils
    // d'un coup.
    let première = app.tool_availability("rizin-ghidra");
    // Les outils connus, PLUS upx. Il n'est pas dans `known::all()` et
    // ne doit pas y être : cette liste porte les moteurs de
    // désassemblage qu'on peut activer, et upx n'en est pas un — il
    // écrit un fichier déballé. Son état se relève pourtant au même
    // moment, pour que le panneau « UPX » le sache sans parcourir le
    // PATH soixante fois par seconde.
    assert_eq!(
        app.tools_ready.len(),
        namsdec_disasm::known::all().len() + 1,
        "un seul relevé couvre tous les outils, upx compris"
    );
    assert!(
        app.tools_ready.contains_key("upx"),
        "et upx en fait partie : {:?}",
        app.tools_ready.keys().collect::<Vec<_>>()
    );

    // Les demandes suivantes ne relancent rien : elles lisent. On le
    // vérifie en faussant le relevé — s'il était refait, la valeur
    // posée à la main disparaîtrait.
    app.tools_ready.insert(
        "rizin-ghidra".to_owned(),
        namsdec_disasm::known::Availability::Incomplete,
    );
    assert_eq!(
        app.tool_availability("rizin-ghidra"),
        namsdec_disasm::known::Availability::Incomplete,
        "la réponse gardée est celle qu'on lit"
    );

    // Et le relevé se refait quand on le jette : à la fin d'une
    // installation, à l'ouverture des préférences, ou sur demande.
    app.tools_ready.clear();
    assert_eq!(
        app.tool_availability("rizin-ghidra"),
        première,
        "jeté, il se refait et retrouve ce que la machine répond"
    );

    // L'ouverture des préférences le jette une fois : on a pu
    // installer un outil à la main dans un terminal entre-temps.
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    app.show_preferences = true;
    app.preferences_tab = PreferencesTab::Tools;
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    sortie.textures_delta.clear();

    // Et les images suivantes le gardent : c'est tout l'objet. Faussé à
    // la main, il doit survivre au dessin — s'il était refait par
    // image, la valeur posée disparaîtrait.
    app.tools_ready.insert(
        "objdump".to_owned(),
        namsdec_disasm::known::Availability::Incomplete,
    );
    for _ in 0..4 {
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    assert_eq!(
        app.tools_ready.get("objdump").copied(),
        Some(namsdec_disasm::known::Availability::Incomplete),
        "le relevé n'est pas refait à chaque image"
    );
}

/// Ouvrir un binaire ne met JAMAIS son listing en page d'un bloc.
///
/// C'était le contrôle du « une seule pose », et il a tenu tant que le
/// listing passait par un champ de texte : 561 ms à la première image sur
/// crackme-01 — 62 386 lignes —, puis 2,4 ms, le cache de `layout_code`
/// y veillant. Une régression qui aurait reposé à chaque image aurait
/// rendu l'application inutilisable.
///
/// La vue ligne à ligne (voir `vue_listing`) va plus loin : elle ne pose
/// PAS le fichier, même une fois. Ce qui se vérifie maintenant est donc
/// plus fort, et plus simple à dire — aucune galère peinte ne porte plus
/// de deux rangées, et le compteur de poses ne bouge pas d'une image à
/// l'autre.
///
/// Mesuré le 14 octobre 2026, même binaire, même écran : la première
/// image passe de 343 ms à 12,5 ms, et le régime de 410 µs à 185 µs.
#[test]
fn opening_a_binary_never_lays_out_the_whole_listing() {
    let Some(octets) = binaire_fabriqué_long() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    let lignes = app.source.lines().count();
    assert!(lignes > 10_000, "un listing de {lignes} lignes");
    assert_eq!(
        app.editor_mode,
        EditorMode::Normal,
        "on ouvre un binaire pour le lire"
    );

    let mut plus_grande = 0usize;
    let mut poses = Vec::new();
    for _ in 0..6 {
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                plus_grande = plus_grande.max(texte.galley.rows.len());
            }
        }
        poses.push(crate::editor::poses_du_texte());
        sortie.textures_delta.clear();
    }
    assert!(
        plus_grande <= 2,
        "la plus grande galère peinte porte {plus_grande} rangées : le listing          est remis en page d'un bloc quelque part"
    );
    // Les images se suivent sans rien reposer : le compteur ne bouge
    // plus une fois la première passée (il compte la mesure du pas, qui
    // pose une ligne d'un caractère).
    assert_eq!(
        poses.last(),
        poses.get(1),
        "le nombre de poses change d'une image à l'autre : {poses:?}"
    );
}

/// Un listing déjà mis en page ne se refait pas, et ne se re-hache pas.
///
/// Un binaire ouvert fait quinze mille lignes. La mise en page complète
/// coûte trois cents millisecondes ; le cache la rend une fois pour
/// toutes, et c'est déjà le cas depuis longtemps.
///
/// Ce qui ne l'était pas : la CLÉ de ce cache était un condensé du texte
/// entier, recalculé à chaque appel — deux fois par frame, une fois pour
/// la gouttière et une fois pour le champ de texte. Mesuré : 1,2 ms le
/// condensé, contre 11 µs la comparaison directe des mêmes octets. Cent
/// treize fois moins cher, et plus sûr : une égalité exacte ne peut pas
/// collisionner.
///
/// Le contrôle mesure le coût d'un appel servi par le cache. Le seuil
/// est large — une machine chargée mesure mal — mais il attrape le
/// retour au condensé, qui coûte cent fois le reste.
#[test]
fn an_already_laid_out_listing_costs_almost_nothing() {
    let ctx = egui::Context::default();
    let app = NamsdecApp::default();
    app.apply_theme(&ctx);
    // De la taille d'un binaire ordinaire : six cent mille octets.
    let mut listing = String::new();
    for i in 0..15_000 {
        listing.push_str(&format!(
            "{:#010x}  mov rax, {}   ; ligne {i}\n",
            0x400000 + i * 4,
            i % 97
        ));
    }
    assert!(listing.len() > 500_000, "{} octets", listing.len());

    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let mut servi = std::time::Duration::MAX;
    let mut sortie = ctx.run_ui(input, |ui| {
        // Le premier appel pose tout ; les suivants sont servis.
        let _ = crate::editor::layout_code(ui, app.theme, &listing);
        for _ in 0..20 {
            let début = std::time::Instant::now();
            let _ = crate::editor::layout_code(ui, app.theme, &listing);
            servi = servi.min(début.elapsed());
        }
    });
    sortie.textures_delta.clear();
    // Le meilleur des vingt : c'est celui qui mesure le code, les autres
    // mesurent aussi ce que la machine faisait à côté.
    assert!(
        servi < std::time::Duration::from_micros(400),
        "un appel servi par le cache coûte {servi:?} — le condensé du \
         texte entier est-il revenu ? Il vaut 1,2 ms à lui seul."
    );
}

/// La frame qui POSE le texte est celle qui le dit, et non celle d'à
/// côté.
///
/// Une frame lente a deux causes, et le message ne doit pas les
/// confondre : une mise en page qu'on paie UNE fois — trois cents
/// millisecondes à l'ouverture d'un binaire — ou un coût qui
/// reviendrait à chaque image. Crier « frame lente » pour la première
/// fait chercher une fuite là où il n'y a qu'un chargement.
///
/// La distinction se devinait en comparant la longueur du texte d'une
/// frame à l'autre. Cela marche quand le texte change et que l'éditeur
/// le pose dans la même frame — le cas ordinaire —, et se trompe
/// exactement quand il ne faut pas : un fichier choisi dans un dialogue
/// arrive parfois après le dessin, et c'est la frame SUIVANTE qui paie.
/// La longueur y avait déjà été notée.
///
/// Le cache de mise en page compte ses poses : il sait, lui, s'il a
/// payé. Le contrôle vérifie que ce compte n'avance que lorsqu'une pose
/// a vraiment lieu.
#[test]
fn the_frame_that_lays_out_the_text_is_the_one_that_says_so() {
    let ctx = egui::Context::default();
    let app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let texte = "mov rax, 1\nmov rdi, 2\nsyscall\n".repeat(200);
    let autre = "nop\n".repeat(200);

    let mut relevé = Vec::new();
    let mut sortie = ctx.run_ui(input, |ui| {
        let avant = crate::editor::poses_du_texte();
        let _ = crate::editor::layout_code(ui, app.theme, &texte);
        relevé.push(("première pose", crate::editor::poses_du_texte() - avant));

        let avant = crate::editor::poses_du_texte();
        let _ = crate::editor::layout_code(ui, app.theme, &texte);
        let _ = crate::editor::layout_code(ui, app.theme, &texte);
        relevé.push(("relectures", crate::editor::poses_du_texte() - avant));

        let avant = crate::editor::poses_du_texte();
        let _ = crate::editor::layout_code(ui, app.theme, &autre);
        relevé.push(("texte changé", crate::editor::poses_du_texte() - avant));
    });
    sortie.textures_delta.clear();

    assert_eq!(relevé[0], ("première pose", 1), "{relevé:?}");
    assert_eq!(
        relevé[1],
        ("relectures", 0),
        "relire du texte déjà posé ne coûte pas une pose : {relevé:?}"
    );
    assert_eq!(
        relevé[2],
        ("texte changé", 1),
        "un texte nouveau se pose : {relevé:?}"
    );
}

/// Le profil d'un gros binaire se dessine en quelques dizaines de
/// millisecondes, non en douze secondes.
///
/// Il en prenait DOUZE. Le classement compte cinq cents lignes, et pour
/// chacune le panneau demandait « quelle est l'étiquette au-dessus ? »
/// en remontant le listing ligne par ligne. Devant un binaire sans
/// symboles — le cas ordinaire en rétro-ingénierie — il n'y a aucune
/// étiquette : la recherche parcourait donc les soixante mille lignes,
/// cinq cents fois, à chaque image. L'application devenait inutilisable
/// au moment précis où l'on avait besoin d'elle.
///
/// Les étiquettes sont relevées UNE fois, et la recherche est binaire.
/// Le pire cas — aucune étiquette — est devenu le plus rapide.
///
/// Le seuil est large : une machine chargée mesure mal, et ce qu'on
/// veut attraper est un retour au parcours complet, qui coûte cent
/// cinquante fois plus.
#[test]
fn a_big_binarys_profile_draws_in_tens_of_milliseconds() {
    let chemin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/binaires/crackme-01");
    let Ok(octets) = std::fs::read(&chemin) else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.load_binary(octets, "crackme-01");
    assert!(
        app.source.lines().count() > 20_000,
        "le binaire d'essai doit être gros : {} lignes",
        app.source.lines().count()
    );
    // On le fait courir, pour que le profil ait de quoi classer.
    for _ in 0..2000 {
        if app.emulator.as_mut().map(|e| e.step()).is_none() {
            break;
        }
    }
    assert!(
        app.line_visits().len() > 100,
        "le classement doit être fourni : {} lignes",
        app.line_visits().len()
    );

    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(2300.0, 1390.0),
        )),
        ..Default::default()
    };
    app.console_tab = ConsoleTab::Profile;
    // Le meilleur de plusieurs tours : le premier paie la mise en page
    // du listing, les suivants disent le coût en régime — celui qu'on
    // ressent en déplaçant la souris.
    let mut régime = std::time::Duration::MAX;
    for _ in 0..4 {
        let début = std::time::Instant::now();
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
        régime = régime.min(début.elapsed());
    }
    assert!(
        régime < std::time::Duration::from_millis(600),
        "une image du profil coûte {régime:?} — la recherche d'étiquette \
         remonte-t-elle à nouveau tout le listing ? Elle valait douze \
         secondes à elle seule."
    );
}

/// La tranche visible des occurrences se trouve par dichotomie, et c'est
/// la même que celle qu'un parcours complet désignerait.
#[test]
fn the_visible_occurrences_are_found_without_walking_the_others() {
    let texte: String = (1..=200).map(|n| format!("mov rax, {n}\n")).collect();
    let places = find_places(&texte, "mov");
    assert_eq!(places.len(), 200);
    for (première, dernière) in [(1u32, 1u32), (1, 30), (57, 61), (190, 220), (300, 400)] {
        let par_dichotomie = places_entre(&places, première, dernière);
        let à_la_main: Vec<usize> = places
            .iter()
            .enumerate()
            .filter(|(_, place)| place.ligne >= première && place.ligne <= dernière)
            .map(|(rang, _)| rang)
            .collect();
        assert_eq!(
            par_dichotomie.clone().collect::<Vec<_>>(),
            à_la_main,
            "lignes {première} à {dernière}"
        );
    }
}

/// Les occurrences de la recherche ne sont relevées qu'UNE fois, barre
/// ouverte, quelles que soient les images qui passent.
///
/// C'est le contrôle qui retient le défaut du 14 octobre 2026 : sur
/// crackme-01 — 62 386 lignes, 24 081 fois « mov » —, l'image prenait
/// 3 735 ms, barre ouverte, contre 0,8 ms fermée. Chaque image relevait
/// tout, deux fois, puis situait chaque occurrence par deux parcours
/// complets du texte.
#[test]
fn the_search_hits_are_gathered_once_and_kept() {
    let Some(octets) = binaire_fabriqué_long() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    // Une image d'abord, pour que la mise en page soit payée.
    // Le delta de textures se VIDE ; l'écraser par un neuf jetterait
    // l'ancien avec ses poses dedans, et `TexturesDelta` panique quand on
    // le laisse tomber plein.
    ctx.run_ui(input.clone(), |ui| app.draw(ui))
        .textures_delta
        .clear();
    app.find = Some(Find {
        query: "mov".to_owned(),
        current: 0,
        just_opened: false,
    });
    let avant = crate::editor::relevés_de_recherche();
    for _ in 0..6 {
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    let relevés = crate::editor::relevés_de_recherche() - avant;
    assert_eq!(
        relevés, 1,
        "six images, {relevés} relevés — la requête n'a pas changé"
    );
    assert!(
        app.find_hits.len() > 10_000,
        "et le relevé a bien trouvé : {} occurrences",
        app.find_hits.len()
    );

    // La requête change : là, il faut relever à nouveau — et une fois.
    if let Some(find) = &mut app.find {
        find.query = "call".to_owned();
    }
    let avant = crate::editor::relevés_de_recherche();
    for _ in 0..4 {
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    assert_eq!(
        crate::editor::relevés_de_recherche() - avant,
        1,
        "une requête nouvelle, un relevé"
    );
}

/// La fenêtre de désassemblage ne redécode pas le binaire à chaque
/// image.
///
/// Elle composait une ligne — un décodage et deux chaînes — pour CHAQUE
/// instruction du programme, à chaque image, alors que sa zone n'en
/// montre qu'une quarantaine. Sur un binaire d'un million
/// d'instructions, l'ouvrir revenait à la fermer.
#[test]
fn the_disassembly_window_decodes_once_per_question() {
    let Some(octets) = binaire_fabriqué_long() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    app.show_disassembly = true;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let avant = crate::windows::relevés_de_désassemblage();
    for _ in 0..6 {
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    let relevés = crate::windows::relevés_de_désassemblage() - avant;
    assert_eq!(
        relevés, 1,
        "six images, {relevés} relevés — rien n'a changé de la question"
    );
    assert!(
        !app.disassembly_rows.is_empty(),
        "et la fenêtre a bien de quoi montrer"
    );

    // Un filtre posé : une question nouvelle, un relevé de plus.
    app.disassembly_query = "mov".to_owned();
    let avant = crate::windows::relevés_de_désassemblage();
    for _ in 0..4 {
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    assert_eq!(
        crate::windows::relevés_de_désassemblage() - avant,
        1,
        "un filtre nouveau, un relevé"
    );
    assert!(
        app.disassembly_rows.len() < app.assemble_state.as_ref().unwrap().disasm.len(),
        "et le filtre filtre"
    );
}

/// La tranche de rangées visibles, trouvée par dichotomie, est celle
/// qu'un parcours complet du galley désignerait.
///
/// C'est le contrôle de la correction du 14 octobre 2026 : écarter les
/// rangées une par une coûtait neuf millisecondes par image sur un
/// listing d'un million de lignes, pour en retenir soixante. La
/// dichotomie ne vaut que si elle retient EXACTEMENT les mêmes.
#[test]
fn the_visible_rows_are_the_same_the_dichotomy_finds_them() {
    let texte: String = (1..=400).map(|n| format!("    mov rax, {n}\n")).collect();
    let ctx = egui::Context::default();
    let app = NamsdecApp::default();
    app.apply_theme(&ctx);
    let theme = app.theme;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 600.0),
        )),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(input, |ui| {
        let galley = layout_code(ui, theme, &texte);
        let origine = egui::vec2(40.0, 12.0);
        for (haut, bas) in [
            (0.0, 100.0),
            (12.0, 612.0),
            (300.0, 320.0),
            (-500.0, 0.0),
            (5_000.0, 9_000.0),
        ] {
            let vue = egui::Rect::from_min_max(egui::pos2(0.0, haut), egui::pos2(900.0, bas));
            let tranche = rangées_visibles(&galley, origine, vue);
            let à_la_main: Vec<usize> = galley
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| row.rect().translate(origine).intersects(vue))
                .map(|(rang, _)| rang)
                .collect();
            let par_dichotomie: Vec<usize> = galley.rows[tranche.clone()]
                .iter()
                .enumerate()
                .filter(|(rang, row)| {
                    let _ = rang;
                    row.rect().translate(origine).intersects(vue)
                })
                .map(|(rang, _)| tranche.start + rang)
                .collect();
            assert_eq!(par_dichotomie, à_la_main, "vue {haut}..{bas}");
        }
    });
    sortie.textures_delta.clear();
}

/// Le texte entier n'est relevé qu'une fois par texte.
///
/// La vue ligne à ligne a besoin de trois choses que les seules lignes
/// visibles ne donnent pas : où commence chaque ligne, quelle est la plus
/// longue, et ce que le fichier définit (voir `ReleveDuTexte`). Les
/// relever lit tout le fichier — c'est le prix juste une fois par texte,
/// et ce serait exactement le défaut qu'on vient de corriger si cela
/// revenait à chaque image.
///
/// En Insertion, le champ de texte reprend la main : il a sa propre
/// manière de reconnaître le texte (`layout_code`), et le relevé ne lui
/// sert pas.
#[test]
fn the_whole_text_is_surveyed_once_per_text() {
    let Some(octets) = binaire_fabriqué_long() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 900.0),
        )),
        ..Default::default()
    };
    let avant = crate::editor::relevés_du_texte();
    for _ in 0..6 {
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    let relevés = crate::editor::relevés_du_texte() - avant;
    assert_eq!(
        relevés, 1,
        "six images, {relevés} relevés du texte entier — un seul suffit"
    );

    // Le texte change : il faut relever à nouveau, et une seule fois.
    app.set_source(format!("{}\n; une ligne de plus\n", app.source));
    let avant = crate::editor::relevés_du_texte();
    for _ in 0..4 {
        let mut sortie = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
    }
    assert_eq!(
        crate::editor::relevés_du_texte() - avant,
        1,
        "un texte nouveau, un relevé"
    );
}
