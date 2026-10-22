//! La calculatrice : son clavier, ses bases, ses lectures.
//!
//! Le calcul lui-même se contrôle dans `calc.rs` ; ici, ce qu'on voit et
//! ce qu'on frappe.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// La calculatrice ne laisse pas de vide sous sa dernière ligne.
///
/// Une hauteur figée laissait cent pixels entre la dernière ligne et le
/// bouton : un appareil ne se termine pas par du vide. La fenêtre
/// épouse la hauteur de son contenu.
#[test]
fn the_calculator_ends_where_its_content_ends() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_calculator: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };
    let mut bas_du_texte = f32::MIN;
    for _ in 0..3 {
        bas_du_texte = f32::MIN;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                // Ce que la calculatrice écrit, et rien d'autre : le
                // reste de la fenêtre principale est ailleurs.
                let dans = t.pos.x > 400.0 && t.pos.x < 1200.0;
                if dans {
                    bas_du_texte = bas_du_texte.max(t.pos.y + t.galley.size().y);
                }
            }
        }
        sortie.textures_delta.clear();
    }

    let fenêtre = egui::AreaState::load(&ctx, egui::Id::new("calculator_window"))
        .expect("la calculatrice est affichée")
        .rect();
    let vide = fenêtre.bottom() - bas_du_texte;
    assert!(
        vide < 40.0,
        "la fenêtre traîne {vide} pixels de vide sous sa dernière ligne"
    );
}

/// L'expression se tape dans l'écran, au-dessus du résultat.
///
/// Un champ de saisie encadré, posé au-dessus avec son étiquette
/// « Expression », faisait un formulaire. Sur une calculatrice, la ligne
/// du haut de l'écran *est* le champ : c'est là que les chiffres
/// arrivent, et le résultat s'écrit dessous.
#[test]
fn the_expression_is_typed_inside_the_screen_above_the_result() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_calculator: true,
        calc_input: "0xff & 0x0f".to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };

    let mut textes: Vec<(String, egui::Rect)> = Vec::new();
    let mut écran: Option<egui::Rect> = None;
    for _ in 0..3 {
        textes.clear();
        écran = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            match &clipped.shape {
                egui::epaint::Shape::Text(t) => {
                    textes.push((
                        t.galley.text().to_owned(),
                        egui::Rect::from_min_size(t.pos, t.galley.size()),
                    ));
                }
                // Le cadre de l'écran : la seule forme à cette largeur.
                // Deux pixels de tolérance, parce que le liseré déborde
                // du rectangle qu'on lui a donné.
                egui::epaint::Shape::Rect(cadre)
                    if (cadre.rect.width() - (CALC_LARGEUR + 20.0)).abs() < 3.0 =>
                {
                    écran = Some(cadre.rect);
                }
                _ => {}
            }
        }
        sortie.textures_delta.clear();
    }
    let écran = écran.expect("l'écran a son cadre");

    let où = |quoi: &str| -> egui::Rect {
        textes
            .iter()
            .find(|(t, _)| t == quoi)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("« {quoi} » doit être peint"))
    };
    let expression = où("0xff & 0x0f");
    // `0xff & 0x0f` vaut 15, écrit en hexadécimal comme le veut la base
    // par défaut.
    let résultat = où("0xf");

    assert!(
        écran.contains_rect(expression.shrink(1.0)),
        "l'expression est dans l'écran : {expression:?} dans {écran:?}"
    );
    assert!(
        écran.contains_rect(résultat.shrink(1.0)),
        "et le résultat aussi : {résultat:?}"
    );
    assert!(
        expression.bottom() <= résultat.top() + 1.0,
        "l'expression est au-dessus du résultat : {expression:?} puis {résultat:?}"
    );
    // En grand, et pas à la taille d'un champ ordinaire.
    assert!(
        expression.height() > 20.0,
        "l'expression se lit en grand : {} de haut",
        expression.height()
    );
    // Les deux calées à gauche, au même bord : on écrit une expression,
    // qui se lit de gauche à droite comme la ligne de code d'où elle
    // sort.
    assert!(
        (expression.left() - résultat.left()).abs() < 4.0,
        "les deux lignes partagent un bord gauche : {expression:?} et {résultat:?}"
    );
}

/// Les touches du clavier sont de vrais boutons, et le bloc est centré.
///
/// Le thème rend le fond des boutons transparent au repos et leur liseré
/// invisible : sans fond posé à la main, le clavier n'était qu'une
/// trentaine de caractères flottant dans du vide, qu'on ne pensait même
/// pas à cliquer. Et aligné à gauche dans une fenêtre large, il tombait
/// contre un bord avec du vide en face — une page, et non un appareil.
#[test]
fn the_keypad_is_made_of_real_buttons_and_is_centred() {
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_calculator: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };

    // Les touches : des rectangles à la taille posée, et rien d'autre
    // dans la fenêtre n'a cette taille-là.
    let mut touches: Vec<(egui::Rect, egui::Color32)> = Vec::new();
    for _ in 0..3 {
        touches.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Rect(cadre) = &clipped.shape
                && (cadre.rect.width() - TOUCHE_L).abs() < 0.5
                && (cadre.rect.height() - TOUCHE_H).abs() < 0.5
            {
                touches.push((cadre.rect, cadre.fill));
            }
        }
        sortie.textures_delta.clear();
    }

    assert_eq!(touches.len(), 30, "cinq rangées de six touches");
    // La teinte rendue à son opacité pleine : egui fait apparaître une
    // fenêtre en fondu, et ses formes arrivent en alpha prémultiplié
    // tant que le fondu dure. On juge la couleur, non l'avancement
    // d'une animation.
    let teinte = |c: egui::Color32| -> [i32; 3] {
        let a = i32::from(c.a()).max(1);
        [
            i32::from(c.r()) * 255 / a,
            i32::from(c.g()) * 255 / a,
            i32::from(c.b()) * 255 / a,
        ]
    };
    let fond_panneau = teinte(app.theme.surface);
    for (rect, fond) in &touches {
        assert!(fond.a() > 0, "une touche invisible : {rect:?}");
        // Assez loin du fond pour se voir : c'est ce qui manquait quand
        // les touches étaient peintes à vingt pour cent d'opacité —
        // elles prenaient la couleur du panneau, à peine plus claires,
        // et le clavier n'avait plus l'air d'un clavier.
        let couleur = teinte(*fond);
        let distance: i32 = couleur
            .iter()
            .zip(fond_panneau.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(
            distance >= 60,
            "une touche doit se détacher franchement du panneau : {couleur:?} contre \
             {fond_panneau:?}"
        );
    }

    // Trois familles, trois teintes : les chiffres, les opérateurs et ce
    // qui efface ne se ressemblent pas. Des mélanges timides donnaient
    // six nuances du même bleu sombre, où il fallait lire chaque touche
    // pour la trouver.
    let familles: std::collections::BTreeSet<[i32; 3]> =
        touches.iter().map(|(_, fond)| teinte(*fond)).collect();
    assert!(
        familles.len() >= 2,
        "les chiffres et les opérateurs n'ont pas la même couleur : {familles:?}"
    );
    let écart = |a: &[i32; 3], b: &[i32; 3]| -> i32 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
    };
    let familles: Vec<[i32; 3]> = familles.into_iter().collect();
    let plus_proches = familles
        .iter()
        .enumerate()
        .flat_map(|(i, a)| familles[i + 1..].iter().map(move |b| écart(a, b)))
        .min()
        .unwrap_or(0);
    assert!(
        plus_proches >= 40,
        "deux familles doivent se distinguer d'un coup d'œil : {familles:?}"
    );

    // Centré dans la fenêtre : le milieu du bloc et celui de la fenêtre
    // tombent au même endroit.
    let fenêtre = egui::AreaState::load(&ctx, egui::Id::new("calculator_window"))
        .expect("la calculatrice est affichée");
    let gauche = touches
        .iter()
        .map(|(r, _)| r.left())
        .fold(f32::MAX, f32::min);
    let droite = touches
        .iter()
        .map(|(r, _)| r.right())
        .fold(f32::MIN, f32::max);
    let milieu_du_bloc = (gauche + droite) / 2.0;
    let milieu_fenêtre = fenêtre.rect().center().x;
    assert!(
        (milieu_du_bloc - milieu_fenêtre).abs() < 12.0,
        "le clavier doit être centré : {milieu_du_bloc} contre {milieu_fenêtre}"
    );

    // La grille de bits est centrée elle aussi : ses deux index
    // encadrent la rangée, et leur milieu tombe au milieu de la fenêtre.
    let mut index: std::collections::BTreeMap<String, (f32, f32)> =
        std::collections::BTreeMap::new();
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    for clipped in &sortie.shapes {
        if let egui::epaint::Shape::Text(texte) = &clipped.shape {
            let mot = texte.galley.text();
            if ["63", "32"].contains(&mot) {
                index.insert(
                    mot.to_owned(),
                    (texte.pos.x, texte.pos.x + texte.galley.size().x),
                );
            }
        }
    }
    sortie.textures_delta.clear();
    let (gauche_index, _) = index["63"];
    let (_, droite_index) = index["32"];
    assert!(
        ((gauche_index + droite_index) / 2.0 - milieu_fenêtre).abs() < 16.0,
        "la grille de bits doit être centrée : {gauche_index}..{droite_index}"
    );

    // Six colonnes : les touches d'une rangée se suivent sans se
    // chevaucher, et chaque rangée en compte six.
    let mut par_rangée: std::collections::BTreeMap<i32, usize> = std::collections::BTreeMap::new();
    for (rect, _) in &touches {
        *par_rangée.entry(rect.top() as i32).or_default() += 1;
    }
    assert_eq!(par_rangée.len(), 5, "cinq rangées : {par_rangée:?}");
    for (haut, combien) in &par_rangée {
        assert_eq!(*combien, 6, "six touches à la hauteur {haut}");
    }
}

/// Le clavier écrit ce que l'évaluateur lit.
///
/// C'est toute la règle : une touche qui produirait une expression que
/// la calculatrice refuse serait une touche qui ment. En hexadécimal,
/// taper 1 puis F doit donner trente et un — non mille neuf cent.
#[test]
fn the_keypad_writes_what_the_evaluator_reads() {
    // Une frappe après l'autre, comme au clavier de l'écran.
    let tape = |base: CalcBase, touches: &[&str]| -> String {
        let mut expression = String::new();
        for touche in touches {
            expression.push_str(&écriture_de_touche(touche, base, &expression));
        }
        expression
    };

    // Le préfixe arrive avec le premier chiffre, et une seule fois.
    assert_eq!(tape(CalcBase::Hex, &["1", "F"]), "0x1F");
    assert_eq!(calc::eval("0x1F", None), Ok(31));
    assert_eq!(tape(CalcBase::Bin, &["1", "1", "1", "1", "1"]), "0b11111");
    assert_eq!(calc::eval("0b11111", None), Ok(31));
    assert_eq!(tape(CalcBase::Oct, &["3", "7"]), "0o37");
    // Le décimal n'a pas de préfixe : c'est le défaut de tout le monde.
    assert_eq!(tape(CalcBase::Dec, &["3", "1"]), "31");

    // Après un opérateur, un nouveau nombre commence, donc un nouveau
    // préfixe.
    assert_eq!(tape(CalcBase::Hex, &["1", "F", "&", "F"]), "0x1F&0xF");
    assert_eq!(calc::eval("0x1F&0xF", None), Ok(15));

    // Les opérateurs et les parenthèses s'écrivent tels quels.
    assert_eq!(
        tape(CalcBase::Hex, &["(", "1", ")", "<<", "4"]),
        "(0x1)<<0x4"
    );
    assert_eq!(calc::eval("(0x1)<<0x4", None), Ok(16));
}

/// Un chiffre n'est proposé que s'il existe dans la base.
///
/// Un clavier qui propose `9` en binaire ment sur ce qu'on peut taper,
/// et l'on apprend l'erreur en la commettant.
#[test]
fn a_digit_is_offered_only_where_it_exists() {
    assert!(CalcBase::Hex.accepte('f') && CalcBase::Hex.accepte('F'));
    assert!(CalcBase::Dec.accepte('9') && !CalcBase::Dec.accepte('A'));
    assert!(CalcBase::Oct.accepte('7') && !CalcBase::Oct.accepte('8'));
    assert!(CalcBase::Bin.accepte('1') && !CalcBase::Bin.accepte('2'));
}

/// La largeur masque l'affichage, elle ne change pas le calcul.
///
/// Le processeur travaille sur soixante-quatre bits quoi qu'on affiche ;
/// ce qu'on choisit ici, c'est combien de bits on veut *lire*. Un `and
/// eax, 0x1f` travaille sur trente-deux, et lire son résultat sur
/// soixante-quatre fait compter des zéros qui ne veulent rien dire.
#[test]
fn the_width_masks_the_reading_not_the_computation() {
    assert_eq!(CalcWidth::W64.mask(), u64::MAX, "et non un débordement");
    assert_eq!(CalcWidth::W32.mask(), 0xffff_ffff);
    assert_eq!(CalcWidth::W16.mask(), 0xffff);
    assert_eq!(CalcWidth::W8.mask(), 0xff);

    let valeur = calc::eval("0 - 1", None).expect("le calcul passe");
    assert_eq!(valeur, u64::MAX, "le calcul déborde comme un registre");
    assert_eq!(
        valeur & CalcWidth::W8.mask(),
        0xff,
        "et la lecture se borne"
    );
}

/// La base et la largeur se retrouvent au retour.
///
/// Ce sont des réglages : quelqu'un qui travaille en hexadécimal sur
/// trente-deux bits ne veut pas le redire à chaque lancement.
#[test]
fn the_calculator_remembers_its_base_and_width() {
    let mut app = NamsdecApp {
        calc_base: CalcBase::Bin,
        calc_width: CalcWidth::W16,
        ..Default::default()
    };
    let écrits = app.settings_snapshot();
    assert_eq!(écrits.calc_base, "bin");
    assert_eq!(écrits.calc_width, "16");

    app = NamsdecApp::from_settings(écrits);
    assert_eq!(app.calc_base, CalcBase::Bin);
    assert_eq!(app.calc_width, CalcWidth::W16);

    // Un fichier qui ne les connaît pas retombe sur l'hexadécimal et
    // soixante-quatre bits : la base d'un désassemblage, et la largeur
    // de la machine simulée.
    assert_eq!(CalcBase::by_id("n'importe quoi"), CalcBase::Hex);
    assert_eq!(CalcWidth::by_id("n'importe quoi"), CalcWidth::W64);
}

/// La calculatrice montre son clavier, ses bits et ses fonctions.
#[test]
fn the_calculator_shows_its_keypad_its_bits_and_its_functions() {
    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        show_calculator: true,
        calc_input: "0x1f".to_owned(),
        ..Default::default()
    };
    let peint = paint(&mut app);

    // Le résultat, dans la base choisie.
    assert!(peint.contains("0x1f"), "{peint}");
    // Le clavier : des chiffres et des opérateurs de code, non des
    // symboles mathématiques qu'il faudrait retraduire.
    for touche in ["7", "F", "&", "<<", "0x"] {
        assert!(
            peint.contains(&format!("{touche}|")),
            "la touche {touche} :\n{peint}"
        );
    }
    // Les SIX FAMILLES s'annoncent toutes : c'est l'onglet qui dit où
    // chercher, et une famille qu'on ne voit pas est une famille qui
    // n'existe pas pour celui qui lit.
    for (_, court, _) in calc::FAMILIES {
        assert!(
            peint.contains(court),
            "la famille « {court} » doit s'annoncer :\n{peint}"
        );
    }
    // Et chacune montre ses fonctions quand on l'ouvre — par leur nom,
    // les arguments étant dans l'infobulle : écrits sur la pastille,
    // ils doublaient sa largeur.
    //
    // Une par famille, choisie parmi les nouvelles : ce sont celles qui
    // manquaient pour calculer en bits.
    for (famille, attendue) in [
        ("bits", "mask"),
        ("ordre", "bitrev"),
        ("signe", "twos"),
        ("compter", "parity"),
        ("tourner", "sar"),
        ("calcul", "mulhi"),
    ] {
        let mut app = NamsdecApp {
            show_calculator: true,
            calc_family: famille.to_owned(),
            ..Default::default()
        };
        let peint = paint(&mut app);
        assert!(
            peint.contains(&format!("{attendue}|")),
            "« {attendue} » doit paraître sous « {famille} » :\n{peint}"
        );
    }
    // Une famille à la fois : ce qui n'est pas ouvert n'est pas peint,
    // sans quoi les quarante-sept pastilles reviendraient toutes et
    // avec elles les douze lignes qui débordaient de l'écran.
    let mut app = NamsdecApp {
        show_calculator: true,
        calc_family: "bits".to_owned(),
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(peint.contains("setbit|"), "la famille ouverte :\n{peint}");
    assert!(
        !peint.contains("mulhi|"),
        "une autre famille ne doit pas être peinte :\n{peint}"
    );
    // Et la ligne qui invite à les survoler.
    assert!(peint.contains(s.calculator_pick_function), "{peint}");
    // Les index de la grille de bits.
    assert!(peint.contains("63|"), "{peint}");
    assert!(peint.contains("32|"), "{peint}");
}

/// La calculatrice se prend juste à gauche de l'onglet Pile.
///
/// C'est là qu'on s'en sert : on lit une valeur dans la pile ou dans un
/// registre, et l'on veut la même dans une autre base. Elle était dans
/// le menu Outils, à trois gestes de l'endroit où le besoin naît.
#[test]
fn the_calculator_sits_just_left_of_the_stack_tab() {
    let s = strings(Lang::Fr);
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);

    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };
    let mut touches = Vec::new();
    let mut pile: Option<f32> = None;
    for _ in 0..2 {
        touches.clear();
        pile = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            match &clipped.shape {
                // Les six touches de l'icône : des points, les seuls
                // cercles aussi petits de la barre d'outils.
                // La barre d'outils occupe la deuxième rangée : « Pile »
                // est aussi le titre du panneau, plus bas, et c'est le
                // titre qu'on attrapait.
                egui::epaint::Shape::Circle(cercle)
                    if cercle.radius < 1.5 && cercle.center.y < 60.0 =>
                {
                    touches.push(cercle.center);
                }
                egui::epaint::Shape::Text(texte)
                    if texte.galley.text() == s.tab_stack && texte.pos.y < 60.0 =>
                {
                    pile = Some(texte.pos.x);
                }
                _ => {}
            }
        }
        sortie.textures_delta.clear();
    }

    let pile = pile.expect("l'onglet Pile est peint");
    assert_eq!(touches.len(), 6, "les six touches de la calculatrice");
    for point in &touches {
        assert!(
            point.x < pile,
            "la calculatrice est à gauche de « {} » : {} contre {pile}",
            s.tab_stack,
            point.x
        );
    }
    // Dans la barre d'outils, non perdue ailleurs dans la fenêtre.
    let haut = touches.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    assert!(
        haut > 30.0,
        "dans la barre d'outils, non dans les menus : {haut}"
    );
}

/// La calculatrice s'ouvre, calcule, et montre toutes les bases.
///
/// Le test regarde ce qui est **peint** : l'évaluateur est déjà couvert
/// au caractère près ailleurs, ce qu'on vérifie ici est que la fenêtre
/// montre bien ce qu'il rend.
#[test]
fn the_calculator_shows_every_reading_of_what_is_typed() {
    let mut app = NamsdecApp {
        show_calculator: true,
        calc_input: "0xff & 0x0f".to_owned(),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
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

    // 0xff & 0x0f fait 15 : on doit le lire dans chaque base, sans avoir
    // à cliquer quoi que ce soit.
    for attendu in ["0xf", "15", "0o17", "00001111"] {
        assert!(
            peint.contains(attendu),
            "« {attendu} » manque dans la calculatrice :\n{peint}"
        );
    }
}

/// Une expression fautive se dit en clair, et ne fait rien tomber.
#[test]
fn the_calculator_names_what_went_wrong() {
    let ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 900.0),
        )),
        ..Default::default()
    };
    // Chacune des fautes qu'on fait en tapant, y compris les états
    // intermédiaires d'une frappe.
    for (expression, attendu) in [
        ("1 / 0", strings(Lang::Fr).calc_divide_by_zero),
        ("(1", strings(Lang::Fr).calc_unbalanced),
        ("1 +", strings(Lang::Fr).calc_missing),
    ] {
        let mut app = NamsdecApp {
            show_calculator: true,
            calc_input: expression.to_owned(),
            ..Default::default()
        };
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
        assert!(
            peint.contains(attendu),
            "« {expression} » devrait dire « {attendu} » :\n{peint}"
        );
    }

    // Et une fenêtre vide ne reproche rien : elle vient de s'ouvrir.
    let mut app = NamsdecApp {
        show_calculator: true,
        ..Default::default()
    };
    let mut peint = String::new();
    for _ in 0..2 {
        peint.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                peint.push_str(texte.galley.text());
            }
        }
        sortie.textures_delta.clear();
    }
    assert!(
        !peint.contains(strings(Lang::Fr).calc_missing),
        "rien de tapé, rien à reprocher :\n{peint}"
    );
}

/// « Base » et « Largeur » sont deux réglages jumeaux, et se lisent
/// comme tels.
///
/// Chaque liste prenait la largeur de ce qu'elle affichait : « Binaire »
/// et « 8 bits » donnaient deux boîtes de tailles différentes, et deux
/// flèches qui ne tombaient pas au même endroit. Les largeurs sont
/// maintenant mesurées sur les textes les plus longs — et dans les deux
/// langues, parce qu'un chiffre écrit en dur alignerait le français en
/// désalignant l'anglais.
#[test]
fn the_base_and_the_width_are_two_twins_and_read_as_twins() {
    for lang in [Lang::Fr, Lang::En] {
        let s = strings(lang);
        let ctx = egui::Context::default();
        let mut app = NamsdecApp {
            lang,
            show_calculator: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        // Où chaque texte est peint, une fois la fenêtre posée.
        let mut posés: Vec<(String, egui::Rect)> = Vec::new();
        for _ in 0..4 {
            posés.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    posés.push((
                        t.galley.text().to_owned(),
                        egui::Rect::from_min_size(t.pos, t.galley.size()),
                    ));
                }
            }
            sortie.textures_delta.clear();
        }
        let où = |quoi: &str| -> egui::Rect {
            posés
                .iter()
                .find(|(t, _)| t == quoi)
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("{lang:?} : « {quoi} » est peint"))
        };

        let base = où(s.calculator_base_label);
        let largeur = où(s.calculator_width_label);
        // Les deux étiquettes sur la même ligne — comparées par leur
        // MILIEU : « Largeur » a une hampe descendante et « Base » non,
        // si bien que leurs galettes n'ont pas la même hauteur et que
        // leurs bords hauts ne coïncident pas alors qu'elles sont
        // pourtant sur la même ligne.
        assert!(
            (base.center().y - largeur.center().y).abs() < 2.0,
            "{lang:?} : les deux étiquettes doivent être sur la même ligne — \
             {:.1} contre {:.1}",
            base.center().y,
            largeur.center().y
        );
        let valeur_base = où(app.calc_base.label(s));
        let valeur_largeur = où(&format!("{} bits", app.calc_width.bits()));
        let écart_base = valeur_base.left() - base.left();
        let écart_largeur = valeur_largeur.left() - largeur.left();
        assert!(
            (écart_base - écart_largeur).abs() < 1.0,
            "{lang:?} : la valeur doit être au même écart de son étiquette — \
             {écart_base:.1} px pour « {} », {écart_largeur:.1} px pour « {} »",
            s.calculator_base_label,
            s.calculator_width_label
        );

        // Et les deux flèches tombent au même endroit : c'est ce qui se
        // voit d'abord. egui les dessine en triangle, calées à droite de
        // la liste — deux listes de même largeur donnent donc deux
        // flèches au même écart de leur étiquette.
        let mut flèches: Vec<f32> = flèches_de_la_rangée(&ctx, &mut app, base.center().y);
        flèches.sort_by(f32::total_cmp);
        assert_eq!(
            flèches.len(),
            2,
            "{lang:?} : deux listes, donc deux flèches — {flèches:?}"
        );
        let vers_flèche_base = flèches[0] - base.left();
        let vers_flèche_largeur = flèches[1] - largeur.left();
        assert!(
            (vers_flèche_base - vers_flèche_largeur).abs() < 1.0,
            "{lang:?} : les deux flèches doivent tomber au même endroit — \
             {vers_flèche_base:.1} px contre {vers_flèche_largeur:.1} px"
        );
    }
}
