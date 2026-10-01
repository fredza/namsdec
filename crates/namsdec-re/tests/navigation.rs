//! Cliquer une adresse d'un panneau d'analyse mène au listing.
//!
//! Un panneau d'analyse ne fait que **désigner des endroits** : une
//! entrée de liaison, une zone libre, une figure, un fragment. La seule
//! chose qu'on veuille en faire est d'aller voir — et jusqu'ici il
//! fallait relever l'adresse à la main et la retaper dans « aller à ».
//!
//! Le contrôle clique pour de vrai : il peint le panneau, cherche où
//! l'adresse a été posée, et envoie un appui et un relâchement à cet
//! endroit. Vérifier que le code « poserait une demande si on cliquait »
//! ne prouverait rien — c'est justement de savoir si l'on PEUT cliquer
//! qu'il s'agit.

mod commun;

use commun::{BASE, germe_piégé};
use namsdec_core::{Cpu, Memory};
use namsdec_re::{ReContext, ReModule, ReRequest};

/// Peint le panneau, clique à `cible`, et rend ce que le module demande.
///
/// Trois images : la première pose les identifiants qu'egui garde d'une
/// image sur l'autre, la deuxième porte l'appui, la troisième le
/// relâchement — un clic n'existe qu'une fois le bouton relevé.
fn clique(module: &mut dyn ReModule, ctx: &ReContext, cible: egui::Pos2) -> Option<ReRequest> {
    let egui_ctx = egui::Context::default();
    let base = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 2000.0),
        )),
        ..Default::default()
    };
    let mut demande = None;
    for étape in 0..3 {
        let mut entrée = base.clone();
        match étape {
            1 => {
                entrée.events.push(egui::Event::PointerMoved(cible));
                entrée.events.push(egui::Event::PointerButton {
                    pos: cible,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            2 => entrée.events.push(egui::Event::PointerButton {
                pos: cible,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }),
            _ => {}
        }
        let mut sortie = egui_ctx.run_ui(entrée, |ui| module.ui(ui, ctx));
        sortie.textures_delta.clear();
        if let Some(posée) = module.take_request() {
            demande = Some(posée);
        }
    }
    demande
}

/// Où un texte a été peint, la première fois qu'il l'est.
fn où_est(module: &mut dyn ReModule, ctx: &ReContext, texte: &str) -> Option<egui::Pos2> {
    let egui_ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 2000.0),
        )),
        ..Default::default()
    };
    let mut trouvé = None;
    for _ in 0..2 {
        trouvé = None;
        let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, ctx));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && t.galley.text() == texte
                && trouvé.is_none()
            {
                let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                // Le centre, et rogné au découpage : un point hors du
                // rectangle de découpe ne reçoit aucun clic.
                let visible = rect.intersect(clipped.clip_rect);
                if visible.width() > 0.0 && visible.height() > 0.0 {
                    trouvé = Some(visible.center());
                }
            }
        }
        sortie.textures_delta.clear();
        // La demande posée par un survol éventuel ne doit pas fausser la
        // suite.
        let _ = module.take_request();
    }
    trouvé
}

/// Chaque panneau qui désigne une adresse mène à cette adresse.
///
/// Quatre panneaux d'un coup : ce qui compte est que le geste soit **le
/// même partout**. Une adresse qui mène quelque part dans l'un et nulle
/// part dans l'autre coûterait plus cher que si aucune ne menait nulle
/// part — on apprend à cliquer, et le clic doit tenir sa promesse.
#[test]
fn clicking_an_address_asks_to_go_there() {
    let binaire = germe_piégé();
    // Le listing de l'application : les sections exécutables à leur
    // adresse. Ici, `.text` seule.
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x120], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
    };

    let panneaux: Vec<(&str, Box<dyn ReModule>)> = vec![
        ("imports", Box::new(namsdec_re::ImportsModule::default())),
        ("place libre", Box::new(namsdec_re::SlackModule::default())),
        ("fragments", Box::new(namsdec_re::GadgetsModule::default())),
        ("figures", Box::new(namsdec_re::PatternsModule::default())),
    ];

    let mut éprouvés = 0;
    for (nom, mut module) in panneaux {
        module.on_binary_loaded(&ctx);
        // L'adresse telle que le panneau l'écrit : c'est le texte qu'on
        // cherchera à l'écran.
        let egui_ctx = egui::Context::default();
        let mut adresses: Vec<String> = Vec::new();
        for _ in 0..2 {
            adresses.clear();
            let mut sortie = egui_ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 2000.0),
                    )),
                    ..Default::default()
                },
                |ui| module.ui(ui, &ctx),
            );
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    let texte = t.galley.text();
                    if texte.starts_with("0x") && texte.len() == 10 {
                        adresses.push(texte.to_owned());
                    }
                }
            }
            sortie.textures_delta.clear();
            let _ = module.take_request();
        }
        let Some(écrite) = adresses.first().cloned() else {
            // Un panneau qui n'a rien à montrer sur ce fichier : il n'y
            // a pas d'adresse à cliquer, et ce n'est pas un échec.
            continue;
        };
        let attendue = u64::from_str_radix(écrite.trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| panic!("{nom} : « {écrite} » est une adresse"));

        let cible = où_est(module.as_mut(), &ctx, &écrite)
            .unwrap_or_else(|| panic!("{nom} : « {écrite} » est peinte quelque part"));
        let demande = clique(module.as_mut(), &ctx, cible);
        assert_eq!(
            demande,
            Some(ReRequest::GoTo { addr: attendue }),
            "{nom} : cliquer « {écrite} » doit demander d'y aller"
        );
        éprouvés += 1;
    }
    assert!(
        éprouvés >= 3,
        "au moins trois panneaux montrent une adresse sur ce fichier : {éprouvés}"
    );
}

/// Sans clic, aucun panneau ne demande quoi que ce soit.
///
/// La moitié qui compte : un panneau qui poserait sa demande au simple
/// dessin ferait sauter le listing tout seul, à chaque image.
#[test]
fn drawing_alone_asks_for_nothing() {
    let binaire = germe_piégé();
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x120], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
    };
    let egui_ctx = egui::Context::default();
    for mut module in [
        Box::new(namsdec_re::ImportsModule::default()) as Box<dyn ReModule>,
        Box::new(namsdec_re::SlackModule::default()),
        Box::new(namsdec_re::GadgetsModule::default()),
        Box::new(namsdec_re::PatternsModule::default()),
    ] {
        module.on_binary_loaded(&ctx);
        for _ in 0..4 {
            let mut sortie = egui_ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 2000.0),
                    )),
                    ..Default::default()
                },
                |ui| module.ui(ui, &ctx),
            );
            sortie.textures_delta.clear();
            assert_eq!(
                module.take_request(),
                None,
                "dessiner ne demande rien : {}",
                module.id()
            );
        }
    }
}
