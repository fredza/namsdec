//! La rangée des vues : ce qu'elle montre, et où vit ce qu'elle ne
//! montre pas.
//!
//! Le désassemblage a quitté sa fenêtre pour la rangée le 25 octobre
//! 2026. Il peut encore en sortir — c'est la seule façon de le lire à
//! côté de la source —, et c'est ce va-et-vient qui est ici sous
//! contrôle : une vue doit être quelque part, jamais aux deux endroits
//! ni nulle part.

use super::*;

/// L'écran du banc, assez grand pour que rien ne soit à l'étroit.
fn entrée() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    }
}

/// Tout ce qui est écrit à l'écran, en quatre images.
fn écrits(ctx: &egui::Context, app: &mut NamsdecApp) -> Vec<String> {
    let (_, textes) = ce_qui_est_écrit_sur(ctx, app, None, 4, entrée());
    textes.into_iter().map(|(texte, _)| texte).collect()
}

/// La rangée ne porte pas d'onglet pour une vue qui n'y est plus.
///
/// Un onglet qui ne mène à rien est pire qu'un onglet absent : on
/// clique, et c'est la fenêtre déjà ouverte qui aurait dû répondre.
///
/// Mesuré pour les huit, et sur ce qui est ÉCRIT : l'étiquette de
/// l'onglet est un texte court — « Déroulement » —, le bandeau de la
/// fenêtre en porte un autre — « Déroulement du programme ». Détachée,
/// la vue ne doit plus écrire le premier nulle part, et doit écrire le
/// second.
#[test]
fn la_rangee_ne_montre_pas_l_onglet_d_une_vue_detachee() {
    let s = strings(Lang::Fr);
    for vue in VueCentrale::ALL {
        if !vue.détachable() {
            continue;
        }
        let étiquette = vue.label(s);
        let ctx = egui::Context::default();
        // La décompilation n'existe qu'avec son module : sans lui, elle
        // n'a pas d'onglet, et ce n'est pas elle qu'on mesurerait.
        let mut app = NamsdecApp {
            show_decompiler_module: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        let avant = écrits(&ctx, &mut app);
        assert_eq!(
            avant.iter().filter(|t| *t == étiquette).count(),
            1,
            "la rangée porte l'onglet « {étiquette} », une fois : {avant:?}"
        );

        let ctx = egui::Context::default();
        // La décompilation n'existe qu'avec son module : sans lui, elle
        // n'a pas d'onglet, et ce n'est pas elle qu'on mesurerait.
        let mut app = NamsdecApp {
            show_decompiler_module: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        app.détacher_vue(vue);
        let après = écrits(&ctx, &mut app);
        // Quatre vues portent le même nom court et le même titre
        // entier — « Chronologie », « Points d'arrêt »,
        // « Registres étendus », « Désassemblage » : le bandeau de leur
        // fenêtre écrit alors exactement ce qu'écrivait leur onglet, et
        // le seul compte qui ait un sens est « une fois, pas deux ».
        // Pour les quatre autres, l'onglet disparaît tout à fait.
        let attendu = usize::from(vue.titre(s) == étiquette);
        assert_eq!(
            après.iter().filter(|t| *t == étiquette).count(),
            attendu,
            "détachée, {vue:?} ne garde pas d'onglet dans la rangée — \
             « {étiquette} » doit s'écrire {attendu} fois : {après:?}"
        );
        assert!(
            après.iter().any(|t| t == vue.titre(s)),
            "et elle s'écrit au bandeau de sa fenêtre, sous son titre \
             entier « {} » : {après:?}",
            vue.titre(s)
        );
    }
}

/// La source, elle, ne se détache pas.
///
/// C'est la vue qui reste quand les autres partent : une rangée vide ne
/// mènerait nulle part, et la zone centrale n'aurait plus rien à
/// montrer.
#[test]
fn la_source_ne_quitte_jamais_la_rangee() {
    assert!(!VueCentrale::Source.détachable());
    let mut app = NamsdecApp::default();
    app.détacher_vue(VueCentrale::Source);
    assert_eq!(app.vue_centrale, VueCentrale::Source);
}

/// Le listing est le même des deux côtés.
///
/// C'est tout l'intérêt d'avoir un seul corps : la vue en onglet et la
/// vue en fenêtre montrent les mêmes adresses, et le jour où l'une
/// divergera, c'est ici qu'on le verra.
#[test]
fn le_desassemblage_dit_la_meme_chose_dans_la_rangee_et_dans_sa_fenetre() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    /// Les adresses écrites : huit chiffres hexadécimaux, ce que la
    /// première colonne du listing peint.
    fn adresses(textes: &[String]) -> Vec<String> {
        let mut vues: Vec<String> = textes
            .iter()
            .filter(|t| t.len() == 8 && t.chars().all(|c| c.is_ascii_hexdigit()))
            .cloned()
            .collect();
        vues.sort();
        vues.dedup();
        vues
    }

    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.source = PROGRAMME_DU_BANC.to_owned();
    app.do_assemble();
    app.montrer_vue(VueCentrale::Disassembly);
    let dans_la_rangée = adresses(&écrits(&ctx, &mut app));

    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.source = PROGRAMME_DU_BANC.to_owned();
    app.do_assemble();
    app.détacher_vue(VueCentrale::Disassembly);
    let dans_la_fenêtre = adresses(&écrits(&ctx, &mut app));

    assert!(
        !dans_la_rangée.is_empty(),
        "la rangée montre bien un listing"
    );
    // La fenêtre est plus petite que la zone centrale : elle en montre
    // moins, mais ce qu'elle montre doit être le même listing.
    let communes = dans_la_fenêtre
        .iter()
        .filter(|a| dans_la_rangée.contains(a))
        .count();
    assert_eq!(
        communes,
        dans_la_fenêtre.len(),
        "la fenêtre montre des adresses que la rangée ignore :\n  \
         fenêtre {dans_la_fenêtre:?}\n  rangée {dans_la_rangée:?}"
    );
}

/// Les huit vues détachables font le même aller-retour.
///
/// Le désassemblage a essuyé les plâtres le 25 octobre ; les sept
/// autres ont suivi le même jour, et c'est justement quand un chemin se
/// répète huit fois qu'un seul des huit se met à mentir. On les éprouve
/// toutes, et une neuvième vue entrera dans ce contrôle sans qu'on y
/// pense.
#[test]
fn chaque_vue_fait_l_aller_retour_entre_la_rangee_et_sa_fenetre() {
    for vue in VueCentrale::ALL {
        if !vue.détachable() {
            continue;
        }
        // La décompilation n'existe qu'avec son module : sans lui, elle
        // n'a pas d'onglet, et ce n'est pas elle qu'on mesurerait.
        let mut app = NamsdecApp {
            show_decompiler_module: true,
            ..Default::default()
        };

        app.montrer_vue(vue);
        assert_eq!(app.vue_centrale, vue, "{vue:?} se choisit dans la rangée");
        assert!(!app.vue_détachée(vue));

        app.détacher_vue(vue);
        assert!(
            app.vue_détachée(vue),
            "{vue:?} détachée est dans sa fenêtre"
        );
        assert!(
            !app.vue_dans_la_rangée(vue),
            "{vue:?} détachée n'a plus d'onglet : une vue est à un seul \
             endroit"
        );
        assert_eq!(
            app.vue_centrale,
            VueCentrale::Source,
            "la zone centrale retombe sur la source : l'onglet qu'on \
             regardait vient de partir"
        );

        // Le menu Outils ne la rattache pas d'autorité : elle est
        // déjà sous les yeux, et la ramener défairait le côte à côte.
        app.montrer_vue(vue);
        assert_eq!(app.vue_centrale, VueCentrale::Source);

        // Fermer la fenêtre, c'est ce que fait sa croix.
        app.poser_fenêtre_de_vue(vue, false);
        assert!(app.vue_dans_la_rangée(vue), "fermée, {vue:?} est rendue");
        app.montrer_vue(vue);
        assert_eq!(
            app.vue_centrale, vue,
            "revenue dans la rangée, {vue:?} se choisit de nouveau"
        );
    }
}

/// La rangée tient dans le plus petit écran que l'application accepte.
///
/// Neuf onglets demandent huit cent quarante pixels — mesurés, pas
/// estimés — et la zone centrale n'en offre que quatre cents à
/// 800 x 520, le plus petit écran que l'application accepte : la rangée
/// passe donc à la ligne, trois fois. C'est ce qu'on éprouve ici.
///
/// LE PIÈGE, et ce contrôle y est tombé d'abord : un onglet repoussé
/// hors du bord n'est pas ABSENT, il est ROGNÉ. Sans passage à la
/// ligne, « Correctifs » se peignait sur six pixels de large, par
/// dessus le panneau latéral, et un contrôle qui demandait seulement
/// « est-il peint, et reste-t-il dans l'écran ? » répondait oui deux
/// fois. Un seuil deviné ne mesure rien.
///
/// On compare donc chaque onglet à LUI-MÊME au large, où rien ne le
/// rogne : la même étiquette doit occuper la même encre. Un onglet à
/// moitié peint est pire qu'un onglet absent, parce qu'on croit
/// l'avoir lu.
#[test]
fn la_rangee_des_vues_tient_dans_le_plus_petit_ecran() {
    /// L'encre de chaque étiquette de la rangée, sur un écran donné.
    fn encres(largeur: f32, hauteur: f32) -> Vec<(VueCentrale, egui::Rect)> {
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, hauteur),
            )),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        // La décompilation n'existe qu'avec son module : sans lui, elle
        // n'a pas d'onglet, et ce n'est pas elle qu'on mesurerait.
        let mut app = NamsdecApp {
            show_decompiler_module: true,
            ..Default::default()
        };
        app.apply_theme(&ctx);
        // UNE VUE DÉTACHABLE EST CHOISIE, sans quoi « Détacher » n'est
        // pas peint du tout : la source ne se détache pas, et le bouton
        // ne s'adresse qu'à la vue active. Le contrôle l'a appris en
        // rougissant — il mesurait la rangée du cas où elle n'a pas de
        // bouton.
        app.montrer_vue(VueCentrale::Disassembly);
        let s = strings(app.lang);
        let (_, lignes) = ce_qui_est_écrit_sur(&ctx, &mut app, None, 4, entrée);
        let mut encres = Vec::new();
        for vue in VueCentrale::ALL {
            for (texte, rect) in &lignes {
                if texte == vue.label(s) {
                    encres.push((vue, *rect));
                }
            }
        }
        encres
    }

    let s = strings(Lang::Fr);
    // Au large, rien ne rogne : c'est la référence.
    let large = encres(1600.0, 1100.0);
    let petit = encres(MIN_SCREEN[0], MIN_SCREEN[1]);

    for vue in VueCentrale::ALL {
        let étiquette = vue.label(s);
        let au_large = large
            .iter()
            .find(|(v, _)| *v == vue)
            .map(|(_, r)| *r)
            .unwrap_or_else(|| panic!("l'onglet « {étiquette} » doit être peint au large"));
        let posées: Vec<egui::Rect> = petit
            .iter()
            .filter(|(v, _)| *v == vue)
            .map(|(_, r)| *r)
            .collect();
        assert_eq!(
            posées.len(),
            1,
            "l'onglet « {étiquette} » doit être peint une fois à {} x {} : \
             une vue sans onglet n'existe plus pour qui regarde l'écran",
            MIN_SCREEN[0],
            MIN_SCREEN[1]
        );
        let rect = posées[0];
        assert!(
            rect.width() >= au_large.width() - 0.5,
            "l'onglet « {étiquette} » est rogné à l'étroit : {:.0} px \
             d'encre contre {:.0} au large. La rangée doit passer à la \
             ligne, non raboter ses onglets",
            rect.width(),
            au_large.width()
        );
        assert!(
            rect.right() <= MIN_SCREEN[0] && rect.left() >= 0.0,
            "l'onglet « {étiquette} » sort de l'écran : {rect:?} pour une \
             largeur de {}",
            MIN_SCREEN[0]
        );
        assert!(
            rect.bottom() <= MIN_SCREEN[1] && rect.top() >= 0.0,
            "l'onglet « {étiquette} » sort de l'écran par le haut ou le \
             bas : {rect:?}"
        );
    }

    // Et la commande qui va avec : « Détacher » ne se fait pas rogner
    // non plus — une commande à moitié peinte ne se clique pas. Elle
    // descend d'une ligne quand le bout de la rangée ne lui laisse pas
    // la place, ce qui est le cas ici.
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            MIN_SCREEN.into(),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_decompiler_module: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.montrer_vue(VueCentrale::Disassembly);
    let (_, lignes) = ce_qui_est_écrit_sur(&ctx, &mut app, None, 4, entrée);
    let bouton: Vec<egui::Rect> = lignes
        .iter()
        .filter(|(texte, _)| texte == s.view_detach)
        .map(|(_, rect)| *rect)
        .collect();
    assert_eq!(
        bouton.len(),
        1,
        "« {} » doit être peint une fois : c'est le seul chemin vers le \
         côte à côte",
        s.view_detach
    );
    assert!(
        bouton[0].right() <= MIN_SCREEN[0],
        "« {} » sort de l'écran : {:?}",
        s.view_detach,
        bouton[0]
    );
}

/// Une vue éteinte n'a pas d'onglet.
///
/// La décompilation est un module qu'on allume dans les préférences :
/// son onglet n'existe que s'il mène à quelque chose. C'est le même
/// principe que pour une vue détachée — un onglet qui ne mène à rien
/// est pire qu'un onglet absent —, et c'est ce que le menu Outils
/// fait déjà de son côté avec son entrée.
#[test]
fn la_rangee_ne_montre_pas_l_onglet_d_un_module_eteint() {
    let s = strings(Lang::Fr);
    let étiquette = VueCentrale::Decompiler.label(s);

    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(
        !app.show_decompiler_module,
        "le module est éteint par défaut, et c'est tout le sujet"
    );
    let éteint = écrits(&ctx, &mut app);
    assert!(
        !éteint.iter().any(|t| t == étiquette),
        "module éteint, « {étiquette} » ne doit pas avoir d'onglet : {éteint:?}"
    );
    assert!(
        !app.vue_dans_la_rangée(VueCentrale::Decompiler),
        "et `montrer_vue` doit refuser de la choisir"
    );

    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        show_decompiler_module: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    let allumé = écrits(&ctx, &mut app);
    assert!(
        allumé.iter().any(|t| t == étiquette),
        "module allumé, l'onglet « {étiquette} » paraît : {allumé:?}"
    );
}

/// Chaque vue n'a qu'un seul corps, et il sert aux deux lieux.
///
/// La règle est structurelle, et c'est ce qui la rend tenable : le
/// corps prend un `&mut Ui` et ne sait pas où il peint ; l'enveloppe
/// est soit la fenêtre, soit l'onglet. Le jour où l'un des deux lieux
/// se mettrait à peindre autre chose — une variante « vite faite » pour
/// l'onglet —, les deux divergeraient en silence, et c'est le genre
/// d'écart qu'on ne voit qu'en comparant deux captures.
///
/// On compte donc les appels dans les sources de l'interface : au
/// moins deux pour chaque corps, et l'un d'eux est dans `vues.rs`.
#[test]
fn chaque_vue_a_un_seul_corps_pour_ses_deux_lieux() {
    let dispatch = SOURCES_DE_L_INTERFACE
        .iter()
        .find(|(nom, _)| *nom == "vues.rs")
        .map(|(_, source)| *source)
        .expect("vues.rs est dans la liste des sources de l'interface");
    for vue in VueCentrale::ALL {
        if !vue.détachable() {
            continue;
        }
        let corps = format!("{}_body_ui(", vue.id());
        let appels: usize = SOURCES_DE_L_INTERFACE
            .iter()
            .map(|(_, source)| source.matches(&corps).count())
            .sum();
        // Trois au moins : la déclaration, l'appel de la fenêtre et
        // celui de la rangée.
        assert!(
            appels >= 3,
            "« {corps} » n'est appelé que {appels} fois : une vue doit \
             avoir un corps, et ce corps doit servir à la fenêtre comme \
             à l'onglet"
        );
        // Le désassemblage passe par `disassembly_view_ui`, qui relève
        // ce qu'il montre avant de peindre ; les autres sont appelés
        // directement. Dans les deux cas, la rangée doit nommer la vue.
        assert!(
            dispatch.contains(&corps) || dispatch.contains(&format!("{}_view_ui(", vue.id())),
            "la rangée ne peint pas {vue:?} : `vues.rs` ne nomme ni son \
             corps ni sa vue"
        );
    }
}

/// « Détacher » garde sa gouttière au bord de la zone de travail.
///
/// Une disposition de droite à gauche part du bord et n'en laisse rien
/// : sur la capture du 25 octobre 2026, le liseré du bouton touchait le
/// trait du panneau latéral, et deux cadres qui se touchent se lisent
/// comme un seul. La place du bouton est donc retenue AVEC sa marge.
///
/// La mesure se prend sur le CADRE du bouton — pas sur son texte, qui
/// est à l'intérieur — et la borne est la zone de travail que
/// l'application se donne, non un nombre réécrit ici : `MARGE` est la
/// même constante des deux côtés.
///
/// Le bouton est aussi PETIT : un bouton de pleine taille fait 25 px
/// dans une rangée qui en fait 26, et pèse alors plus que les onglets
/// qu'il accompagne.
#[test]
fn le_bouton_detacher_garde_sa_gouttiere_et_reste_petit() {
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    app.montrer_vue(VueCentrale::Disassembly);
    let s = strings(app.lang);
    let mut cadre: Option<egui::Rect> = None;
    let mut encre: Option<egui::Rect> = None;
    for _ in 0..3 {
        cadre = None;
        encre = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        // Le texte d'abord : c'est lui qui désigne le bouton.
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && t.galley.text() == s.view_detach
            {
                encre = Some(egui::Rect::from_min_size(t.pos, t.galley.size()));
            }
        }
        // Puis le plus petit rectangle qui le contient : son cadre.
        if let Some(encre) = encre {
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Rect(r) = &clipped.shape
                    && r.rect.contains(encre.center())
                    && r.rect.width() < encre.width() + 40.0
                    && (cadre.is_none_or(|c: egui::Rect| r.rect.width() < c.width()))
                {
                    cadre = Some(r.rect);
                }
            }
        }
        sortie.textures_delta.clear();
    }
    let encre = encre.expect("« Détacher » est peint dans la rangée");
    let cadre = cadre.expect("« Détacher » a un liseré : c'est un bouton, pas une étiquette");
    let gouttière = app.zone_de_travail.right() - cadre.right();
    assert!(
        gouttière >= crate::widgets::ONGLET_MARGE - 0.5,
        "« {} » touche le bord : {:.1} px de gouttière pour {:.0} attendus \
         — cadre {cadre:?}, zone de travail {:?}",
        s.view_detach,
        gouttière,
        crate::widgets::ONGLET_MARGE,
        app.zone_de_travail
    );
    assert!(
        cadre.contains_rect(encre),
        "le liseré doit entourer le texte : cadre {cadre:?}, texte {encre:?}"
    );
    // Petit : la rangée fait 26 px, et le bouton doit y tenir avec de
    // l'air au-dessus et au-dessous.
    assert!(
        cadre.height() <= 20.0,
        "« {} » est un bouton PETIT : {:.0} px de haut dans une rangée de 26",
        s.view_detach,
        cadre.height()
    );
}

/// ON RENOMME UNE FONCTION INCONNUE DEPUIS LA RANGÉE.
///
/// C'est le geste de rétro-ingénierie le plus ordinaire : un binaire
/// dépouillé ne donne que `sub_40008e`, on comprend ce qu'elle fait, on
/// lui donne un nom, et tout le reste de l'application l'emploie —
/// listing, arbre d'appels, références croisées, déroulement.
///
/// IL TENAIT À UN FIL en passant de la fenêtre à l'onglet, et c'est
/// pourquoi ce contrôle existe : un module d'analyse ne renomme pas
/// lui-même, il DEMANDE (`ReRequest::Rename`), et quelqu'un doit
/// relever sa demande après le dessin. La fenêtre le faisait ; la
/// rangée devait apprendre à le faire (`vue_centrale_ui`). Sans cette
/// ligne, le champ de saisie s'ouvre, on tape, on valide, et il ne se
/// passe RIEN — le pire des défauts, parce qu'il ne dit pas son nom.
///
/// Le geste est refait en entier : le clic sur l'étiquette, la frappe,
/// la validation. Rien n'est appelé en coulisse.
///
/// AUCUNE ÉCRITURE SUR LE DISQUE : l'empreinte des notes est effacée
/// avant le geste, de sorte que `save_notes` s'abstient. Un contrôle
/// n'écrit pas dans le dossier de données de l'utilisateur.
#[test]
fn on_renomme_une_fonction_inconnue_depuis_la_rangee() {
    let Some(octets) = binaire_fabriqué() else {
        return;
    };
    let ctx = egui::Context::default();
    let mut app = NamsdecApp::default();
    app.apply_theme(&ctx);
    assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "l'ELF se charge");
    app.notes_fingerprint = None;
    app.analysis_module = "functions".to_owned();
    app.montrer_vue(VueCentrale::Analysis);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        ..Default::default()
    };

    // Le nom sans nom : celui que namsdec fabrique faute de symbole.
    let mut cible: Option<(u64, egui::Pos2)> = None;
    for _ in 0..3 {
        cible = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && let Some(hexa) = t.galley.text().strip_prefix("sub_")
                && let Ok(adresse) = u64::from_str_radix(hexa, 16)
            {
                cible = Some((
                    adresse,
                    egui::Rect::from_min_size(t.pos, t.galley.size()).center(),
                ));
            }
        }
        sortie.textures_delta.clear();
    }
    let (adresse, où) = cible.expect(
        "la rubrique « Fonctions » doit montrer au moins une fonction sans \
         nom : c'est elle qu'on renomme",
    );
    assert_eq!(
        app.notes.name(adresse),
        None,
        "elle n'a pas encore de nom, c'est tout le sujet"
    );

    // Le clic sur l'étiquette ouvre la saisie.
    let clic = egui::RawInput {
        events: vec![
            egui::Event::PointerMoved(où),
            egui::Event::PointerButton {
                pos: où,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos: où,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ],
        ..entrée.clone()
    };
    let mut sortie = ctx.run_ui(clic, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    // Une image de plus : le champ demande le clavier à son ouverture,
    // et egui le lui donne à l'image suivante.
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    sortie.textures_delta.clear();
    assert!(
        ctx.memory(|m| m.focused()).is_some(),
        "le champ de saisie doit avoir le clavier : sans lui, la frappe \
         part dans l'éditeur"
    );

    // Tout sélectionner, écrire par-dessus, valider.
    const NOM: &str = "vérifie_mdp";
    let frappe = egui::RawInput {
        events: vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Text(NOM.to_owned()),
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            },
        ],
        ..entrée.clone()
    };
    let mut sortie = ctx.run_ui(frappe, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    // Et une image pour que la demande, relevée après le dessin, ait été
    // exécutée — elle l'est dans la même, mais on ne le suppose pas.
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    sortie.textures_delta.clear();

    assert_eq!(
        app.notes.name(adresse),
        Some(NOM),
        "le nom donné dans la rangée doit atteindre les notes : c'est la \
         demande du module, relevée après le dessin"
    );
    // Et il se lit à l'écran, là où l'on vient de le taper.
    let écrit = écrits(&ctx, &mut app);
    assert!(
        écrit.iter().any(|t| t == NOM),
        "le nom doit paraître dans la rubrique : {écrit:?}"
    );
}

/// La rangée « Moteur : » est alignée, et le niveau débutant n'en
/// propose que trois.
///
/// **Deux consignes écrites sur une capture d'écran** (4 novembre
/// 2026) : « ça doit être aligné », trait souligné sous la rangée, et
/// « comme moteur de décompil en mode débutant, le choix doit être
/// pseudo-code, nasm et objdump (Intel) ».
///
/// L'alignement d'abord, parce qu'il se mesure : dans une rangée egui,
/// chaque élément est centré sur la hauteur que la rangée a **à cet
/// instant**. L'étiquette arrive la première, sur une rangée haute de
/// quinze pixels ; les choix sont des boutons de vingt-trois, et la
/// rangée grandit sous eux. L'étiquette restait cinq pixels plus haut
/// que ce qu'elle annonce.
#[test]
fn la_rangee_du_moteur_est_alignee_et_le_debutant_en_a_trois() {
    let s = strings(Lang::Fr);
    let rendre = |app: &mut NamsdecApp| -> Vec<(String, egui::Pos2)> {
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        let mut vus = Vec::new();
        for _ in 0..3 {
            vus.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    vus.push((texte.galley.text().to_owned(), texte.pos));
                }
            }
            sortie.textures_delta.clear();
        }
        vus
    };

    // 1. ALIGNÉE : l'étiquette et les choix sur la même ligne de base.
    let mut app = NamsdecApp {
        vue_centrale: VueCentrale::Disassembly,
        ..Default::default()
    };
    let vus = rendre(&mut app);
    let y = |quoi: &str| -> f32 {
        vus.iter()
            .find(|(texte, _)| texte == quoi)
            .unwrap_or_else(|| panic!("« {quoi} » doit être peint"))
            .1
            .y
    };
    let étiquette = y(s.disassembly_engine);
    for choix in [s.disassembly_pseudo, s.disassembly_nasm] {
        assert!(
            (y(choix) - étiquette).abs() < 1.0,
            "« {} » est à {étiquette:.1}, « {choix} » à {:.1} : la rangée n'est pas alignée",
            s.disassembly_engine,
            y(choix)
        );
    }

    // 2. TROIS AU NIVEAU DÉBUTANT. La syntaxe du système est celle de
    //    gdb et d'as : elle suppose qu'on sache déjà ce qu'on cherche.
    let mut app = NamsdecApp {
        vue_centrale: VueCentrale::Disassembly,
        debutant: true,
        ..Default::default()
    };
    let textes: Vec<String> = rendre(&mut app).into_iter().map(|(t, _)| t).collect();
    assert!(
        textes.iter().any(|t| t == s.disassembly_pseudo)
            && textes.iter().any(|t| t == s.disassembly_nasm),
        "le pseudo-code et NASM restent : {textes:?}"
    );
    assert!(
        !textes.iter().any(|t| t == s.disassembly_system),
        "la syntaxe du système ne se propose pas au débutant : {textes:?}"
    );

    // 3. ET CE QU'ON REGARDE RESTE LISTÉ : un moteur choisi au niveau
    //    avancé puis retrouvé au niveau débutant garde sa place, sans
    //    quoi la rangée n'aurait plus rien de sélectionné et l'on ne
    //    saurait plus d'où vient ce qu'on lit.
    let mut app = NamsdecApp {
        vue_centrale: VueCentrale::Disassembly,
        debutant: true,
        disassembler: "system".to_owned(),
        ..Default::default()
    };
    let textes: Vec<String> = rendre(&mut app).into_iter().map(|(t, _)| t).collect();
    assert!(
        textes.iter().any(|t| t == s.disassembly_system),
        "le moteur en cours reste listé, même au-dessus du niveau : {textes:?}"
    );
}

/// L'accueil : ce qu'on voit quand on a fermé le dernier fichier.
///
/// **Demandé à l'écran, capture à l'appui** (4 novembre 2026) : « quand
/// je quitte, j'aimerais que tout soit fermé, et une fenêtre comme la
/// suivante apparaisse — celle comme Zed ».
///
/// Quatre choses à retenir, et la deuxième est celle qui fait mal si on
/// l'oublie : l'accueil ne doit PAS se déduire du tampon vide, sans quoi
/// il apparaîtrait au milieu d'une frappe — on sélectionne tout, on
/// efface pour recommencer, et l'éditeur disparaîtrait sous les doigts.
#[test]
fn l_accueil_parait_quand_on_a_ferme_et_jamais_sous_les_doigts() {
    let s = strings(Lang::Fr);
    let peindre = |app: &mut NamsdecApp| -> Vec<String> {
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        let mut vus = Vec::new();
        for _ in 0..3 {
            vus.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    vus.push(texte.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        vus
    };

    // 1. ON FERME LE DERNIER FICHIER : l'accueil prend la place.
    let mut app = NamsdecApp {
        source: "mov rax, 1\n".to_owned(),
        recent_files: vec![std::path::PathBuf::from("/tmp/boucle.asm")],
        ..Default::default()
    };
    app.close_tab_now(app.active_tab_index);
    assert!(
        app.rien_n_est_ouvert(),
        "fermer le dernier mène à l'accueil"
    );
    let vus = peindre(&mut app);
    for attendu in [
        s.accueil_commencer,
        s.accueil_nouveau,
        s.accueil_ouvrir,
        s.accueil_palette,
        s.accueil_recents,
    ] {
        assert!(
            vus.iter().any(|t| t == attendu),
            "« {attendu} » doit être à l'accueil : {vus:?}"
        );
    }
    assert!(
        vus.iter().any(|t| t == "boucle.asm"),
        "le fichier récent est offert : {vus:?}"
    );

    // 2. ET JAMAIS SOUS LES DOIGTS. Effacer tout son texte n'est pas
    //    fermer : l'éditeur reste, sans quoi on perdrait l'endroit où
    //    l'on est en train d'écrire.
    let mut app = NamsdecApp {
        source: String::new(),
        ..Default::default()
    };
    assert!(
        !app.rien_n_est_ouvert(),
        "un tampon vidé n'est pas un fichier fermé"
    );
    let vus = peindre(&mut app);
    assert!(
        !vus.iter().any(|t| t == s.accueil_commencer),
        "l'accueil ne paraît pas sous les doigts de qui efface tout"
    );

    // 3. CE QUI EST MONTRÉ EST CE QUI MARCHE : « Nouveau fichier »
    //    rend un éditeur, et sans laisser l'onglet qu'on vient de
    //    fermer revenir tout seul dans la barre.
    let mut app = NamsdecApp {
        source: "mov rax, 1\n".to_owned(),
        ..Default::default()
    };
    app.close_tab_now(app.active_tab_index);
    app.new_program();
    assert!(
        !app.rien_n_est_ouvert() && !app.source.trim().is_empty(),
        "« Nouveau fichier » rend un éditeur"
    );
    assert!(
        app.tabs.is_empty(),
        "et un seul : le tampon fermé ne revient pas comme onglet"
    );

    // 4. LES RACCOURCIS AFFICHÉS SONT CEUX DU TABLEAU, lus et non
    //    recopiés : réglé autrement, l'accueil le dit autrement.
    let mut app = NamsdecApp {
        source: "mov rax, 1\n".to_owned(),
        ..Default::default()
    };
    app.keybindings.map.insert(
        Action::New,
        crate::keybindings::Chord::Simple(egui::KeyboardShortcut::new(
            egui::Modifiers::CTRL | egui::Modifiers::ALT,
            egui::Key::K,
        )),
    );
    app.close_tab_now(app.active_tab_index);
    let vus = peindre(&mut app);
    assert!(
        vus.iter().any(|t| t.contains('K') && t.contains("Alt")),
        "le raccourci réglé se lit à l'accueil : {vus:?}"
    );
}

/// Le compteur de problèmes : ce qu'il compte, et où il mène.
///
/// **Demandé à l'écran, capture à l'appui** (4 novembre 2026) : le
/// badge de CLion, « erreurs et warning », en haut à droite de
/// l'éditeur — un disque rouge et son nombre, un triangle orange et le
/// sien, et deux chevrons pour sauter au problème suivant.
#[test]
fn le_compteur_de_problemes_compte_et_mene_au_probleme() {
    let diagnostic = |ligne: u32, erreur: bool| namsdec_asm::Diagnostic {
        line: ligne,
        severity: if erreur {
            namsdec_asm::Severity::Error
        } else {
            namsdec_asm::Severity::Warning
        },
        message: "pour la mesure".to_owned(),
        kind: None,
    };
    // Les chevrons sont des TRACÉS : c'est à eux qu'on reconnaît le
    // compteur quand il n'a aucun nombre à écrire.
    let chevrons_de_la_rangée = |app: &mut NamsdecApp| -> usize {
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        let mut combien = 0;
        for _ in 0..3 {
            combien = 0;
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Path(tracé) = &clipped.shape {
                    // Les chevrons sont de simples traits : le triangle
                    // de l'avertissement est un tracé lui aussi, mais
                    // rempli.
                    if tracé.fill != egui::Color32::TRANSPARENT {
                        continue;
                    }
                    let haut = tracé.visual_bounding_rect();
                    if haut.center().y > 90.0 && haut.center().y < 125.0 && haut.center().x > 700.0
                    {
                        combien += 1;
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        combien
    };
    let nombres_de_la_rangée = |app: &mut NamsdecApp| -> Vec<String> {
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1100.0),
            )),
            ..Default::default()
        };
        let mut vus = Vec::new();
        for _ in 0..3 {
            vus.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    // La rangée des vues, et elle seule : un « 2 » dans
                    // le listing ou dans un registre ne dit rien du
                    // compteur.
                    if texte.pos.y > 90.0 && texte.pos.y < 125.0 && texte.pos.x > 700.0 {
                        vus.push(texte.galley.text().to_owned());
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        vus
    };

    // 1. LES DEUX NOMBRES, chacun le sien.
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        diagnostics: vec![
            diagnostic(3, true),
            diagnostic(7, true),
            diagnostic(5, false),
        ],
        ..Default::default()
    };
    assert_eq!(app.compte_des_problemes(), (2, 1));
    let vus = nombres_de_la_rangée(&mut app);
    assert!(
        vus.iter().any(|t| t == "2") && vus.iter().any(|t| t == "1"),
        "deux erreurs et un avertissement se lisent au compteur : {vus:?}"
    );
    assert_eq!(
        chevrons_de_la_rangée(&mut app),
        2,
        "et deux chevrons, pour aller d'un problème à l'autre"
    );

    // 2. RIEN À COMPTER, RIEN À L'ÉCRAN. Un compteur à zéro en
    //    permanence est un endroit de l'écran qu'on apprend à ne plus
    //    regarder, et c'est celui qu'il faut voir le jour où il n'est
    //    plus à zéro.
    // SUR UNE VUE DÉTACHABLE, et c'est ce qui donne sa force à la
    // mesure : la rangée n'appelle ses commandes que lorsqu'elle leur a
    // réservé de la place, et sans problème à compter c'est « Détacher »
    // qui la retient. Sur la vue Source, qui ne se détache pas, la
    // rangée n'appellerait rien et l'on prouverait seulement qu'on n'a
    // rien demandé.
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        vue_centrale: VueCentrale::Disassembly,
        ..Default::default()
    };
    assert_eq!(app.compte_des_problemes(), (0, 0));
    assert_eq!(
        chevrons_de_la_rangée(&mut app),
        0,
        "sans problème, pas de compteur du tout — pas même ses chevrons"
    );

    // 3. LES CHEVRONS MÈNENT AU PROBLÈME, ET BOUCLENT.
    let mut app = NamsdecApp {
        diagnostics: vec![
            diagnostic(3, true),
            diagnostic(7, true),
            diagnostic(3, false),
        ],
        ..Default::default()
    };
    app.cursor_pos = (1, 1);
    app.aller_au_probleme(Problèmes::Suivant);
    assert_eq!(
        app.goto_line,
        Some(3),
        "le premier problème après la ligne 1"
    );
    app.cursor_pos = (3, 1);
    app.aller_au_probleme(Problèmes::Suivant);
    assert_eq!(
        app.goto_line,
        Some(7),
        "la ligne d'après, et non la suivante dans la liste"
    );
    app.cursor_pos = (7, 1);
    app.aller_au_probleme(Problèmes::Suivant);
    assert_eq!(app.goto_line, Some(3), "le dernier mène au premier");
    app.cursor_pos = (3, 1);
    app.aller_au_probleme(Problèmes::Précédent);
    assert_eq!(app.goto_line, Some(7), "et le premier au dernier");

    // 4. ET LE COMPTE MÈNE À LA LISTE, sur le bon onglet : ouvrir la
    //    console sans en changer montrerait la sortie du programme à
    //    qui vient de cliquer sur un compte d'erreurs.
    app.show_console = false;
    app.console_tab = ConsoleTab::Output;
    app.aller_au_probleme(Problèmes::Liste);
    assert!(app.show_console, "la liste s'ouvre");
    assert_eq!(
        app.console_tab,
        ConsoleTab::Diagnostics,
        "sur les problèmes"
    );
}

/// Les barres d'icônes des bords : elles montrent, elles cachent, et
/// elles disent ce qu'un clic fera.
///
/// **Demandé à l'écran, capture annotée** (4 novembre 2026) : trois
/// rectangles jaunes autour des barres d'icônes de CLion, et deux
/// flèches — « Afficher l'explorateur » sur l'icône du coin, « cacher
/// l'explorateur » sur le tiret du panneau.
#[test]
fn les_barres_d_icones_montrent_et_cachent() {
    let s = strings(Lang::Fr);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1100.0));
    // L'HORLOGE AVANCE D'UNE IMAGE À L'AUTRE, et c'est ce qui fait
    // paraître les infobulles : elles ont un délai, que le temps figé
    // d'un banc n'atteint jamais.
    let mut horloge = 0.0_f64;
    // Une frappe de souris : on pose le pointeur, on appuie, on
    // relâche. Plusieurs images, parce qu'une infobulle attend son
    // délai et qu'un clic se lit au relâchement.
    // L'APPUI ET LE RELÂCHEMENT SONT DEUX IMAGES. egui décide d'un clic
    // au relâchement, sur un widget qu'il a vu appuyé à l'image d'avant :
    // les deux dans la même image ne font rien, et le contrôle croyait
    // que la case ne répondait pas.
    let mut sortir = |app: &mut NamsdecApp,
                      ctx: &egui::Context,
                      pointeur: Option<egui::Pos2>,
                      cliquer: bool|
     -> Vec<String> {
        let mut événements = Vec::new();
        if let Some(pos) = pointeur {
            événements.push(egui::Event::PointerMoved(pos));
            if cliquer {
                événements.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
            }
        }
        horloge += 0.5;
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            events: événements,
            time: Some(horloge),
            focused: true,
            ..Default::default()
        };
        let mut vus = Vec::new();
        let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                vus.push(texte.galley.text().to_owned());
            }
        }
        sortie.textures_delta.clear();
        vus
    };

    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        ..Default::default()
    };
    app.apply_theme(&ctx);
    sortir(&mut app, &ctx, None, false);
    sortir(&mut app, &ctx, None, false);

    // 1. LES ICÔNES SONT DU CÔTÉ DE LEUR PANNEAU. On les repère à
    //    leurs tracés : une icône de barre est un dessin, jamais un
    //    mot — c'est tout l'intérêt d'une barre de trente-quatre
    //    pixels.
    let mut icônes: Vec<egui::Pos2> = Vec::new();
    let entrée = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
    for clipped in &sortie.shapes {
        let cadre = match &clipped.shape {
            egui::epaint::Shape::Path(tracé) => tracé.visual_bounding_rect(),
            egui::epaint::Shape::LineSegment { points, .. } => {
                egui::Rect::from_two_pos(points[0], points[1])
            }
            _ => continue,
        };
        // LA CASE, ET NON LA BARRE ENTIÈRE : le trait qui sépare la
        // barre de l'éditeur tombe à son bord droit, à mi-hauteur de
        // l'écran — relevé à x = 33,5, y = 565 —, et il se faisait
        // compter pour une icône. Le contrôle croyait alors que
        // l'icône de la console était en bas alors qu'elle était en
        // haut : c'est le trait qu'il mesurait.
        if cadre.center().x < crate::barres::BARRE_LARGEUR - 6.0 {
            icônes.push(cadre.center());
        }
    }
    sortie.textures_delta.clear();
    assert!(
        !icônes.is_empty(),
        "la barre de gauche porte ses icônes : {icônes:?}"
    );
    let dossier = icônes
        .iter()
        .copied()
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .expect("au moins une icône");
    let console = icônes
        .iter()
        .copied()
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .expect("au moins une icône");
    assert!(
        console.y > écran.height() * 0.5,
        "l'icône de la console est EN BAS de la barre, comme le panneau \
         qu'elle ouvre : {console:?}"
    );

    // 2. L'INFOBULLE DIT LE GESTE, et change avec l'état. On ne survole
    //    pas une icône pour apprendre son nom, mais pour savoir ce
    //    qu'un clic fera.
    //
    //    MESURÉE À SA SOURCE, et non peinte : une infobulle d'egui ne
    //    paraît pas dans un banc sans écran — huit images, le pointeur
    //    immobile sur la case, le délai largement dépassé, et rien.
    //    Aucun contrôle du projet n'en a jamais relevé une. Ce qui peut
    //    se tromper ici n'est pas le dessin de la bulle mais le CHOIX
    //    de la phrase.
    let dit = app.phrase_de_l_explorateur(&ctx);
    assert!(
        dit.starts_with(s.rail_explorer_show),
        "fermé, l'icône annonce « {} » et non « {dit} »",
        s.rail_explorer_show
    );

    // 3. ET LE CLIC OUVRE.
    assert!(!app.show_explorer, "l'explorateur est fermé au départ");
    let mut cliquer = |app: &mut NamsdecApp| {
        sortir(app, &ctx, Some(dossier), true);
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            events: vec![egui::Event::PointerButton {
                pos: dossier,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            focused: true,
            ..Default::default()
        };
        let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
        sortie.textures_delta.clear();
    };
    cliquer(&mut app);
    assert!(app.show_explorer, "un clic sur l'icône ouvre l'explorateur");
    let dit = app.phrase_de_l_explorateur(&ctx);
    assert!(
        dit.starts_with(s.rail_explorer_hide),
        "ouvert, elle annonce « {} » et non « {dit} »",
        s.rail_explorer_hide
    );
    // ET LE RACCOURCI EST LU, NON RECOPIÉ, comme à l'accueil.
    app.keybindings.map.insert(
        Action::ToggleExplorer,
        crate::keybindings::Chord::Simple(egui::KeyboardShortcut::new(
            egui::Modifiers::CTRL | egui::Modifiers::ALT,
            egui::Key::J,
        )),
    );
    let dit = app.phrase_de_l_explorateur(&ctx);
    assert!(
        dit.contains('J') && dit.contains("Alt"),
        "le raccourci réglé se lit à l'infobulle : {dit}"
    );
    cliquer(&mut app);
    assert!(!app.show_explorer, "et un autre le referme");

    // 4. ÉTEINTES AU MENU, AUCUNE TRACE : on retrouve l'écran d'avant.
    app.show_rails = false;
    sortir(&mut app, &ctx, None, false);
    let entrée = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
    let reste = sortie.shapes.iter().any(|clipped| match &clipped.shape {
        egui::epaint::Shape::Path(tracé) => {
            tracé.visual_bounding_rect().center().x < crate::barres::BARRE_LARGEUR
        }
        _ => false,
    });
    sortie.textures_delta.clear();
    assert!(!reste, "éteintes, les barres ne laissent rien");
}

/// La barre d'icônes de la console : relancer, arrêter, effacer.
///
/// **Demandé à l'écran, capture à l'appui** (4 novembre 2026) : les
/// trois commandes de la fenêtre Run de JetBrains, dans le panneau
/// lui-même.
///
/// Le clic est simulé pour de vrai — pointeur, appui, relâchement —,
/// parce que c'est le CÂBLAGE qui peut se tromper : une icône qui
/// dessine bien et ne fait rien se relit vingt fois sans qu'on voie
/// l'erreur.
#[test]
fn la_barre_de_la_console_efface_sans_rien_remettre_a_zero() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1100.0));
    let ctx = egui::Context::default();
    let mut app = NamsdecApp {
        source: PROGRAMME_DU_BANC.to_owned(),
        show_console: true,
        ..Default::default()
    };
    app.apply_theme(&ctx);
    app.do_assemble();
    for _ in 0..20 {
        app.do_step();
    }
    assert!(
        app.emulator
            .as_ref()
            .is_some_and(|e| !e.output().is_empty()),
        "le programme du banc écrit : c'est ce qu'on va effacer"
    );
    let rip_avant = app.emulator.as_ref().map(|e| e.cpu().regs.rip);

    let image = |app: &mut NamsdecApp, events: Vec<egui::Event>| -> Vec<egui::Rect> {
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            events,
            focused: true,
            ..Default::default()
        };
        let mut dessins = Vec::new();
        let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            // La rangée d'onglets de la console : ses icônes sont des
            // traits et des tracés, au bord droit.
            let cadre = match &clipped.shape {
                egui::epaint::Shape::Path(t) => t.visual_bounding_rect(),
                egui::epaint::Shape::LineSegment { points, .. } => {
                    egui::Rect::from_two_pos(points[0], points[1])
                }
                _ => continue,
            };
            if cadre.center().y > écran.height() * 0.6 && cadre.center().x > écran.width() * 0.7 {
                dessins.push(cadre);
            }
        }
        sortie.textures_delta.clear();
        dessins
    };
    let dessins = image(&mut app, Vec::new());
    image(&mut app, Vec::new());
    let dessins = if dessins.is_empty() {
        image(&mut app, Vec::new())
    } else {
        dessins
    };
    assert!(
        !dessins.is_empty(),
        "la rangée de la console porte ses icônes"
    );
    // LA CORBEILLE EST LA PLUS À DROITE DES TROIS, le tiret et le
    // bouton des arguments mis à part : on la repère à sa place, qui
    // est justement ce que la consigne réglait.
    let bande: Vec<&egui::Rect> = dessins
        .iter()
        .filter(|r| r.height() < 20.0 && r.width() < 20.0)
        .collect();
    assert!(
        bande.len() >= 3,
        "trois commandes au moins : relancer, arrêter, effacer — {bande:?}"
    );

    // Le clic sur la corbeille : elle est à gauche du bouton des
    // arguments, donc parmi les icônes, la troisième en partant de la
    // gauche de ce groupe.
    let mut xs: Vec<f32> = bande.iter().map(|r| r.center().x).collect();
    xs.sort_by(f32::total_cmp);
    let corbeille = bande
        .iter()
        .map(|r| r.center())
        .rfind(|p| (p.x - xs[2]).abs() < 12.0)
        .expect("la corbeille est repérée");
    let clic = |app: &mut NamsdecApp, pressé: bool| {
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            events: vec![
                egui::Event::PointerMoved(corbeille),
                egui::Event::PointerButton {
                    pos: corbeille,
                    button: egui::PointerButton::Primary,
                    pressed: pressé,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            focused: true,
            ..Default::default()
        };
        let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
        sortie.textures_delta.clear();
    };
    clic(&mut app, true);
    clic(&mut app, false);

    // EFFACER N'EST PAS RÉINITIALISER. C'est l'écran de la console
    // qu'on vide ; le processeur n'a pas bougé d'une instruction, et
    // c'est la seule chose qui pouvait être mal câblée ici — « effacer »
    // qui remettrait le programme à zéro ferait perdre une exécution
    // qu'on vient peut-être de passer dix minutes à mener.
    assert!(
        app.emulator.as_ref().is_some_and(|e| e.output().is_empty()),
        "la corbeille efface ce que le programme a écrit"
    );
    assert_eq!(
        app.emulator.as_ref().map(|e| e.cpu().regs.rip),
        rip_avant,
        "et ne remet rien à zéro : le processeur est où on l'a laissé"
    );
}
