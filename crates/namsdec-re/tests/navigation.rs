//! Cliquer une adresse d'un panneau d'analyse mène où l'on veut.
//!
//! Un panneau d'analyse ne fait que **désigner des endroits** : une
//! entrée de liaison, une zone libre, une figure, un fragment. La seule
//! chose qu'on veuille en faire est d'aller voir — et jusqu'ici il
//! fallait relever l'adresse à la main et la retaper dans « aller à ».
//!
//! Le clic DEMANDE où : dans le code désassemblé, ou dans les octets.
//! Les deux répondent à des questions différentes, et le panneau ne
//! sait pas laquelle on se pose.
//!
//! Le contrôle clique pour de vrai : il peint le panneau, cherche où
//! l'adresse a été posée, envoie un appui et un relâchement à cet
//! endroit, puis cherche l'entrée du menu et la clique à son tour.
//! Vérifier que le code « poserait une demande si on cliquait » ne
//! prouverait rien — c'est justement de savoir si l'on PEUT cliquer
//! qu'il s'agit.

mod commun;

use commun::{BASE, germe_piégé};
use namsdec_core::{Cpu, Memory};
use namsdec_re::{GoTarget, ReContext, ReModule, ReRequest};

/// Peint le panneau, clique à `cible`, choisit `entrée_du_menu`, et rend
/// ce que le module demande.
///
/// Deux clics, et c'est le geste réel : le premier ouvre le menu des
/// destinations, le second en choisit une. Entre les deux, le contrôle
/// cherche où l'entrée a été peinte — elle est dans une couche
/// flottante, posée là où egui l'a mise, et une position devinée
/// cliquerait à côté.
///
/// Huit images : egui pose ses identifiants à la première, le clic
/// demande un appui puis un relâchement, et le menu ne se peint qu'une
/// fois le premier clic reçu.
fn clique_et_choisit(
    module: &mut dyn ReModule,
    ctx: &ReContext,
    cible: egui::Pos2,
    entrée_du_menu: &str,
) -> Option<ReRequest> {
    let egui_ctx = egui::Context::default();
    let base = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 2000.0),
        )),
        ..Default::default()
    };
    let mut demande = None;
    let mut dans_le_menu: Option<egui::Pos2> = None;
    for étape in 0..8 {
        let mut entrée = base.clone();
        let appui = |entrée: &mut egui::RawInput, où: egui::Pos2, pressed: bool| {
            if pressed {
                entrée.events.push(egui::Event::PointerMoved(où));
            }
            entrée.events.push(egui::Event::PointerButton {
                pos: où,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        };
        match (étape, dans_le_menu) {
            (1, _) => appui(&mut entrée, cible, true),
            (2, _) => appui(&mut entrée, cible, false),
            (4, Some(où)) => appui(&mut entrée, où, true),
            (5, Some(où)) => appui(&mut entrée, où, false),
            _ => {}
        }
        let mut sortie = egui_ctx.run_ui(entrée, |ui| module.ui(ui, ctx));
        if dans_le_menu.is_none() {
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape
                    && t.galley.text().starts_with(entrée_du_menu)
                {
                    let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                    let visible = rect.intersect(clipped.clip_rect);
                    if visible.width() > 0.0 && visible.height() > 0.0 {
                        dans_le_menu = Some(visible.center());
                    }
                }
            }
        }
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
        names: None,
        patches: &[],
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
        // Les deux destinations, sur la même adresse : le menu n'aurait
        // aucun intérêt si la seconde entrée faisait ce que fait la
        // première.
        // Les entrées disent désormais OÙ elles mènent, et non plus
        // seulement dans quelle vue : « mais où ? », signalé à l'écran
        // le 20 octobre. Le contrôle les cherche donc par leur début.
        for (entrée, attendu) in [
            ("Dans le listing", GoTarget::Listing),
            ("Dans les octets", GoTarget::Bytes),
        ] {
            let demande = clique_et_choisit(module.as_mut(), &ctx, cible, entrée);
            assert_eq!(
                demande,
                Some(ReRequest::GoTo {
                    addr: attendue,
                    target: attendu
                }),
                "{nom} : cliquer « {écrite} » puis « {entrée} » doit demander d'y aller"
            );
        }
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
        names: None,
        patches: &[],
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

/// Un texte cliquable est SOULIGNÉ AU REPOS, et davantage au survol.
///
/// « Comment savoir si un texte est cliquable ou pas ? » La main du
/// pointeur le dit, mais seulement une fois qu'on est déjà dessus, et
/// sur un texte elle se confond avec le curseur de sélection. Le trait
/// est le signe que tout le monde connaît.
///
/// Le contrôle a changé de sens le 20 octobre, et c'est le défaut qu'il
/// retient : il exigeait auparavant qu'il n'y ait AUCUN trait au repos.
/// Un signe qui n'apparaît qu'une fois qu'on est dessus ne sert qu'à
/// celui qui savait déjà — on ne survole pas les textes au hasard pour
/// voir lesquels se cliquent. Ce qu'on exige désormais : un trait au
/// repos, et un trait PLUS VISIBLE au survol.
///
/// Mesuré sur ce qui est PEINT, et sur l'opacité réelle du trait. Une
/// marque qu'on vérifierait dans le code ne dirait pas si elle arrive à
/// l'écran.
#[test]
fn a_clickable_text_underlines_itself_at_rest_and_more_under_the_pointer() {
    let binaire = germe_piégé();
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x120], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    let mut module = namsdec_re::ImportsModule::default();
    module.on_binary_loaded(&ctx);

    // Où une adresse est peinte, et les traits horizontaux comptés à
    // cet endroit-là — les filets de séparation du panneau ne doivent
    // pas compter pour la marque qu'on cherche.
    let Some(cible) = où_est_une_adresse(&mut module, &ctx) else {
        return;
    };
    let au_repos = traits_sous(&mut module, &ctx, cible, None);
    let au_survol = traits_sous(&mut module, &ctx, cible, Some(cible));
    assert!(
        !au_repos.is_empty(),
        "le texte se déclare SANS qu'on le survole : aucun trait peint au repos"
    );
    assert!(
        !au_survol.is_empty(),
        "et il se déclare toujours au survol : aucun trait peint"
    );
    // Au repos le trait est estompé ; au survol il est plein. C'est ce
    // qui garde un panneau calme tout en déclarant ce qui se clique.
    let plus_fort_au_repos = au_repos.iter().map(|c| c.a()).max().unwrap_or(0);
    let plus_fort_au_survol = au_survol.iter().map(|c| c.a()).max().unwrap_or(0);
    assert!(
        plus_fort_au_survol > plus_fort_au_repos,
        "le survol renforce le trait : opacité {plus_fort_au_repos} au repos, \
         {plus_fort_au_survol} au survol"
    );
}

/// Le premier texte qui ressemble à une adresse, et où il est peint.
fn où_est_une_adresse(module: &mut dyn ReModule, ctx: &ReContext) -> Option<egui::Pos2> {
    let egui_ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 2000.0),
        )),
        ..Default::default()
    };
    let mut vu = None;
    for _ in 0..2 {
        vu = None;
        let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, ctx));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape
                && t.galley.text().starts_with("0x")
                && vu.is_none()
            {
                let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                let visible = rect.intersect(clipped.clip_rect);
                if visible.is_positive() {
                    vu = Some(visible.center());
                }
            }
        }
        sortie.textures_delta.clear();
        let _ = module.take_request();
    }
    vu
}

/// La couleur des traits horizontaux peints à la hauteur de `autour`.
///
/// La couleur et non le compte : depuis que le trait se voit au repos,
/// c'est son OPACITÉ qui distingue les deux états, et un compte ne la
/// dirait pas.
fn traits_sous(
    module: &mut dyn ReModule,
    ctx: &ReContext,
    autour: egui::Pos2,
    souris: Option<egui::Pos2>,
) -> Vec<egui::Color32> {
    let egui_ctx = egui::Context::default();
    let mut compte = Vec::new();
    for _ in 0..3 {
        let mut entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 2000.0),
            )),
            ..Default::default()
        };
        if let Some(p) = souris {
            entrée.events.push(egui::Event::PointerMoved(p));
        }
        let mut sortie = egui_ctx.run_ui(entrée, |ui| module.ui(ui, ctx));
        compte.clear();
        for clipped in &sortie.shapes {
            // Un trait d'egui est un rectangle plat ou un segment : on
            // prend les deux, à moins de dix pixels sous le texte.
            let cadre = clipped.shape.visual_bounding_rect();
            let plat = cadre.height() <= 2.0 && cadre.width() > 4.0;
            let sous_le_texte =
                (cadre.top() - autour.y).abs() < 12.0 && cadre.x_range().contains(autour.x);
            if plat && sous_le_texte {
                compte.push(couleur_du_trait(&clipped.shape));
            }
        }
        sortie.textures_delta.clear();
        let _ = module.take_request();
    }
    compte
}

/// La couleur d'un trait, quelle que soit la forme qui le porte.
fn couleur_du_trait(forme: &egui::epaint::Shape) -> egui::Color32 {
    match forme {
        egui::epaint::Shape::LineSegment { stroke, .. } => stroke.color,
        egui::epaint::Shape::Path(chemin) => match chemin.stroke.color {
            egui::epaint::ColorMode::Solid(couleur) => couleur,
            _ => egui::Color32::TRANSPARENT,
        },
        egui::epaint::Shape::Rect(rect) if rect.fill != egui::Color32::TRANSPARENT => rect.fill,
        egui::epaint::Shape::Rect(rect) => rect.stroke.color,
        _ => egui::Color32::TRANSPARENT,
    }
}
