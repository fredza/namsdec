//! La fenêtre sans décoration du système : sa barre de titre, ses trois
//! boutons, et de quoi la déplacer et la redimensionner.
//!
//! Pourquoi la dessiner soi-même : la barre de titre du système ajoute
//! une bande de trente pixels au-dessus d'une application qui a déjà sa
//! barre de menus. En la dessinant, les menus *sont* la barre de titre —
//! on y gagne une rangée, et l'ensemble a l'air d'une seule pièce.
//!
//! Ce qu'il faut reprendre à sa charge quand on la dessine soi-même :
//! déplacer la fenêtre, la maximiser au double-clic, et la
//! redimensionner par ses bords. Rien de tout cela n'est gratuit, et
//! c'est pourquoi le réglage existe : Préférences ▸ Général rend la
//! barre du système à qui la préfère.

use eframe::egui;

use crate::i18n::Strings;
use namsdec_theme::Theme;

/// Le rayon d'un bouton, et l'air autour de lui.
const BUTTON_RADIUS: f32 = 6.5;
/// L'écart entre deux pastilles, et l'air qu'on laisse au bord de la
/// fenêtre. Sans ce dernier, la fermeture se peint à cheval sur le
/// cadre.
const BUTTON_SPACING: f32 = 10.0;
const EDGE_MARGIN: f32 = 10.0;

/// L'épaisseur de la zone qui saisit un bord pour redimensionner.
///
/// Huit pixels : assez pour viser à la souris sans loupe, assez peu pour
/// ne pas manger le clic d'un widget posé au bord.
const RESIZE_EDGE: f32 = 8.0;

/// Ce que fait un bouton de fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowButton {
    Minimize,
    Maximize,
    Close,
}

impl WindowButton {
    /// Les trois, de gauche à droite : fermer, réduire, agrandir.
    ///
    /// C'est l'ordre des pastilles de macOS, et il va avec leur place —
    /// au bord gauche de la fenêtre. La fermeture occupe le coin, donc
    /// l'angle que la souris atteint sans viser : on la lance vers le
    /// haut à gauche, elle bute, elle y est. Les deux autres sont
    /// derrière elle, ce qui les met hors de portée d'un geste trop
    /// rapide — l'inverse de ce qu'on veut pour un bouton qui ferme.
    ///
    /// L'ordre compte plus que le côté : c'est la constance de la place
    /// qui fait qu'on finit par cliquer sans regarder.
    pub const ALL: [WindowButton; 3] = [
        WindowButton::Close,
        WindowButton::Minimize,
        WindowButton::Maximize,
    ];

    /// Sa couleur, celle des pastilles qu'on reconnaît sans les lire.
    pub fn colour(self) -> egui::Color32 {
        match self {
            WindowButton::Minimize => egui::Color32::from_rgb(0xf5, 0xbf, 0x4f),
            WindowButton::Maximize => egui::Color32::from_rgb(0x62, 0xc5, 0x54),
            WindowButton::Close => egui::Color32::from_rgb(0xed, 0x6a, 0x5e),
        }
    }

    fn hint(self, s: &Strings) -> &'static str {
        match self {
            WindowButton::Minimize => s.window_minimize,
            WindowButton::Maximize => s.window_maximize,
            WindowButton::Close => s.window_close,
        }
    }
}

/// Les trois pastilles, à poser en bout de la barre de menus.
///
/// Rend le bouton cliqué, s'il y en a un : l'appelant décide quoi en
/// faire, parce que fermer la fenêtre passe chez namsdec par la garde du
/// travail non enregistré.
///
/// Les positions sont calculées, pas empilées : la dernière pastille
/// doit s'arrêter à `EDGE_MARGIN` du bord, et un enchaînement de cases
/// allouées la poussait dehors — la fermeture se retrouvait à cheval
/// sur le cadre de la fenêtre.
pub fn window_buttons_ui(ui: &mut egui::Ui, s: &Strings, maximized: bool) -> Option<WindowButton> {
    let mut cliqué = None;
    let combien = WindowButton::ALL.len() as f32;
    let largeur =
        combien * BUTTON_RADIUS * 2.0 + (combien - 1.0) * BUTTON_SPACING + EDGE_MARGIN * 2.0;
    let (bande, _) = ui.allocate_exact_size(
        egui::vec2(largeur, BUTTON_RADIUS * 2.0 + 6.0),
        egui::Sense::hover(),
    );
    for (rang, bouton) in WindowButton::ALL.into_iter().enumerate() {
        // Le premier à `EDGE_MARGIN` du bord gauche de la bande, le
        // dernier à la même distance du bord droit : l'écart entre les
        // trois est alors le même, et rien ne déborde.
        let centre = egui::pos2(
            bande.left()
                + EDGE_MARGIN
                + BUTTON_RADIUS
                + rang as f32 * (BUTTON_RADIUS * 2.0 + BUTTON_SPACING),
            bande.center().y,
        );
        let cible = egui::Rect::from_center_size(
            centre,
            egui::vec2(BUTTON_RADIUS * 2.0 + BUTTON_SPACING, bande.height()),
        );
        let réponse = ui.interact(
            cible,
            ui.id().with(("window_button", rang)),
            egui::Sense::click(),
        );
        paint_pastille(ui, centre, bouton, réponse.hovered());
        let infobulle = if bouton == WindowButton::Maximize && maximized {
            s.window_restore
        } else {
            bouton.hint(s)
        };
        if réponse.on_hover_text(infobulle).clicked() {
            cliqué = Some(bouton);
        }
    }
    cliqué
}

/// Peint une pastille — le rond de couleur, et son signe au survol.
///
/// Sortie de la barre de titre parce que les fenêtres internes de
/// namsdec portent la même : une seule façon de dessiner un bouton de
/// fenêtre dans toute l'application, et un seul endroit à corriger.
fn paint_pastille(ui: &egui::Ui, centre: egui::Pos2, bouton: WindowButton, survolée: bool) {
    let couleur = bouton.colour();
    // Éteinte tant que la souris n'est pas dessus : des pastilles vives
    // en permanence tirent l'œil hors du code, qui est ce qu'on est venu
    // lire.
    let remplissage = if survolée {
        couleur
    } else {
        couleur.gamma_multiply(0.55)
    };
    ui.painter()
        .circle_filled(centre, BUTTON_RADIUS, remplissage);
    // Le signe n'apparaît qu'au survol, comme sur les pastilles qu'on
    // connaît : la couleur suffit à désigner, le signe confirme.
    if !survolée {
        return;
    }
    let trait_ = egui::Stroke::new(1.4, egui::Color32::from_black_alpha(190));
    let bras = BUTTON_RADIUS * 0.45;
    match bouton {
        WindowButton::Minimize => {
            ui.painter().line_segment(
                [
                    centre - egui::vec2(bras, 0.0),
                    centre + egui::vec2(bras, 0.0),
                ],
                trait_,
            );
        }
        WindowButton::Maximize => {
            let carré = egui::Rect::from_center_size(centre, egui::vec2(bras * 1.7, bras * 1.7));
            ui.painter()
                .rect_stroke(carré, 1.0, trait_, egui::StrokeKind::Inside);
        }
        WindowButton::Close => {
            ui.painter().line_segment(
                [
                    centre - egui::vec2(bras, bras),
                    centre + egui::vec2(bras, bras),
                ],
                trait_,
            );
            ui.painter().line_segment(
                [
                    centre - egui::vec2(bras, -bras),
                    centre + egui::vec2(bras, -bras),
                ],
                trait_,
            );
        }
    }
}

/// La hauteur de la barre de titre d'une fenêtre interne.
const HEAD_HEIGHT: f32 = 24.0;

/// La barre de titre d'une fenêtre interne, taillée comme celle de la
/// fenêtre principale : la pastille de fermeture au coin haut gauche, le
/// titre au centre, un filet dessous.
///
/// Pourquoi ne pas garder celle d'egui : elle pose une croix maigre à
/// droite, dessinée au trait du texte. Dans une application dont la
/// fenêtre principale se ferme par une pastille rouge posée à gauche,
/// cela fait deux grammaires pour le même geste — et c'est celle qu'on
/// voit le moins qui ferme le plus de choses. Une seule ici, celle de la
/// grande fenêtre.
///
/// Rend `true` si l'on a cliqué la pastille. L'appelant en fait ce qu'il
/// veut : fermer tout court pour une fenêtre d'affichage, annuler pour
/// une boîte qui pose une question.
///
/// La fenêtre se déplace en tirant n'importe où (egui bascule en
/// « glisser partout » dès qu'on lui retire sa barre de titre), donc
/// cette barre-ci n'a pas à s'en charger.
pub fn window_head(ui: &mut egui::Ui, theme: &'static Theme, s: &Strings, titre: &str) -> bool {
    let largeur = ui.available_width();
    let (bande, _) = ui.allocate_exact_size(egui::vec2(largeur, HEAD_HEIGHT), egui::Sense::hover());
    let centre = egui::pos2(bande.left() + EDGE_MARGIN + BUTTON_RADIUS, bande.center().y);
    let cible = egui::Rect::from_center_size(
        centre,
        egui::vec2(BUTTON_RADIUS * 2.0 + BUTTON_SPACING, bande.height()),
    );
    let réponse = ui.interact(
        cible,
        ui.id().with("window_head_close"),
        egui::Sense::click(),
    );
    paint_pastille(ui, centre, WindowButton::Close, réponse.hovered());

    // Le titre au centre de la bande entière, non de ce qui reste à
    // droite de la pastille : décalé de six pixels, il se lirait de
    // travers dans une fenêtre dont on connaît le milieu.
    let galley = ui.painter().layout_no_wrap(
        titre.to_owned(),
        egui::TextStyle::Body.resolve(ui.style()),
        theme.text,
    );
    let place = bande.width() - (EDGE_MARGIN + BUTTON_RADIUS * 2.0 + BUTTON_SPACING) * 2.0;
    if galley.size().x < place {
        ui.painter().galley(
            egui::pos2(
                bande.center().x - galley.size().x * 0.5,
                bande.center().y - galley.size().y * 0.5,
            ),
            galley,
            theme.text,
        );
    }

    // Pas de filet sous le titre : une fenêtre qui en porte un ici et un
    // autre au-dessus de son bouton se lit comme un formulaire, trois
    // bandes empilées. Le bandeau se distingue déjà par sa pastille et
    // par le blanc qui le suit, et c'est assez.
    ui.add_space(6.0);
    réponse.on_hover_text(s.window_close).clicked()
}

/// L'écart entre le dernier menu et le nom du fichier, en largeurs
/// d'espace.
///
/// Trois tabulations de quatre espaces : assez pour qu'on ne lise pas
/// « Aide sans titre » comme un sixième menu, assez peu pour que le nom
/// reste rattaché à la barre plutôt que perdu au milieu.
const TITLE_GAP_SPACES: f32 = 12.0;

/// Rend la zone vide de la barre de menus déplaçable, et le double-clic
/// maximisant — les deux gestes qu'on fait sans y penser sur une barre
/// de titre. Y écrit aussi ce qu'on y cherche : le nom du fichier.
///
/// Le nom est posé **à la suite des menus**, non au centre de la bande.
/// Centré, il se déplaçait à chaque fois qu'on ouvrait un panneau ou
/// qu'on redimensionnait la fenêtre : un repère qui bouge n'en est pas
/// un. Aligné sur la fin du dernier menu, il est toujours au même
/// endroit, et l'œil le trouve sans le chercher.
pub fn title_drag_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    titre: &str,
    modifié: bool,
    maximized: bool,
) {
    let reste = ui.available_size_before_wrap();
    if reste.x <= 0.0 {
        return;
    }
    let (bande, réponse) = ui.allocate_exact_size(reste, egui::Sense::click_and_drag());
    let texte = if modifié {
        format!("{titre} •")
    } else {
        titre.to_owned()
    };
    let police = egui::TextStyle::Body.resolve(ui.style());
    // L'écart est mesuré dans la police courante, non posé en pixels :
    // le réglage de zoom et le thème changent la taille du texte, et un
    // blanc figé se retrouverait deux fois trop grand ou invisible.
    let espace = ui.ctx().fonts_mut(|f| f.glyph_width(&police, ' '));
    let galley = ui.painter().layout_no_wrap(
        texte,
        police,
        if modifié { theme.changed } else { theme.dim },
    );
    let départ = bande.left() + espace * TITLE_GAP_SPACES;
    // Écrit seulement s'il tient : sur une fenêtre étroite, mieux vaut
    // pas de titre qu'un titre qui passe sous les pastilles.
    if départ + galley.size().x < bande.right() {
        ui.painter().galley(
            egui::pos2(départ, bande.center().y - galley.size().y * 0.5),
            galley,
            theme.dim,
        );
    }
    if réponse.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if réponse.double_clicked() {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
}

/// Les huit prises de redimensionnement, sur les bords et les coins.
///
/// Posées dans une couche au-dessus du reste mais *derrière* le
/// pointeur : elles ne peignent rien, elles ne font que changer le
/// curseur et transmettre le geste au système.
pub fn resize_edges_ui(ctx: &egui::Context, _theme: &'static Theme) {
    let écran = ctx.viewport_rect();
    let coin = RESIZE_EDGE * 2.0;
    // Les coins d'abord : ils recouvrent les bords, et c'est ce qu'on
    // veut — viser un coin doit donner le coin, pas le bord.
    let prises: [(egui::Rect, egui::ResizeDirection, egui::CursorIcon); 8] = [
        (
            egui::Rect::from_min_size(écran.left_top(), egui::vec2(coin, coin)),
            egui::ResizeDirection::NorthWest,
            egui::CursorIcon::ResizeNorthWest,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(écran.right() - coin, écran.top()),
                egui::vec2(coin, coin),
            ),
            egui::ResizeDirection::NorthEast,
            egui::CursorIcon::ResizeNorthEast,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(écran.left(), écran.bottom() - coin),
                egui::vec2(coin, coin),
            ),
            egui::ResizeDirection::SouthWest,
            egui::CursorIcon::ResizeSouthWest,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(écran.right() - coin, écran.bottom() - coin),
                egui::vec2(coin, coin),
            ),
            egui::ResizeDirection::SouthEast,
            egui::CursorIcon::ResizeSouthEast,
        ),
        (
            egui::Rect::from_min_size(écran.left_top(), egui::vec2(écran.width(), RESIZE_EDGE)),
            egui::ResizeDirection::North,
            egui::CursorIcon::ResizeNorth,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(écran.left(), écran.bottom() - RESIZE_EDGE),
                egui::vec2(écran.width(), RESIZE_EDGE),
            ),
            egui::ResizeDirection::South,
            egui::CursorIcon::ResizeSouth,
        ),
        (
            egui::Rect::from_min_size(écran.left_top(), egui::vec2(RESIZE_EDGE, écran.height())),
            egui::ResizeDirection::West,
            egui::CursorIcon::ResizeWest,
        ),
        (
            egui::Rect::from_min_size(
                egui::pos2(écran.right() - RESIZE_EDGE, écran.top()),
                egui::vec2(RESIZE_EDGE, écran.height()),
            ),
            egui::ResizeDirection::East,
            egui::CursorIcon::ResizeEast,
        ),
    ];

    for (index, (rect, direction, curseur)) in prises.into_iter().enumerate() {
        let aire = egui::Area::new(egui::Id::new(("resize_edge_area", index)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .interactable(true);
        aire.show(ctx, |ui| {
            let (_, r) = ui.allocate_exact_size(rect.size(), egui::Sense::drag());
            if r.hovered() || r.dragged() {
                ui.ctx().set_cursor_icon(curseur);
            }
            if r.drag_started() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
            }
        });
    }
}

/// Les trois marques des portes du projet, dessinées.
///
/// Dessinées et non chargées : une image par logo, c'est trois fichiers
/// à garder à jour, un décodeur à embarquer, et des couleurs qui jurent
/// avec le thème. Ces trois-là tiennent en quelques primitives, prennent
/// la couleur qu'on leur donne, et restent nettes à toutes les tailles.
/// Ce sont des signes, pas des reproductions : un globe pour le site, le
/// visage de Discord, le chat de GitHub.
pub fn icon_globe(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.3, color);
    let centre = rect.center();
    let r = rect.width() * 0.5;
    painter.circle_stroke(centre, r, stroke);
    // Le méridien : une ellipse vue de face, deux arcs qui se rejoignent
    // aux pôles.
    for sens in [-1.0f32, 1.0] {
        let points: Vec<egui::Pos2> = (0..=12)
            .map(|i| {
                let t = -1.0 + i as f32 / 6.0;
                egui::pos2(
                    centre.x + sens * (1.0 - t * t).max(0.0).sqrt() * r * 0.55,
                    centre.y + t * r,
                )
            })
            .collect();
        painter.add(egui::Shape::line(points, stroke));
    }
    // L'équateur et un parallèle : deux traits qui disent la sphère.
    for part in [0.0f32, -0.45] {
        let y = centre.y + part * r;
        let demi = (1.0 - part * part).max(0.0).sqrt() * r;
        painter.line_segment(
            [
                egui::pos2(centre.x - demi, y),
                egui::pos2(centre.x + demi, y),
            ],
            stroke,
        );
    }
}

/// Le visage de Discord : une tête arrondie, deux yeux, deux pattes.
pub fn icon_discord(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let centre = rect.center();
    let l = rect.width();
    let h = rect.height() * 0.62;
    // Le corps : un rectangle très arrondi, plus large que haut.
    let corps = egui::Rect::from_center_size(centre, egui::vec2(l * 0.92, h));
    painter.rect_filled(corps, h * 0.48, color);
    // Les deux pattes du bas, qui donnent sa silhouette au visage.
    for sens in [-1.0f32, 1.0] {
        painter.circle_filled(
            egui::pos2(centre.x + sens * l * 0.34, corps.bottom() - h * 0.06),
            h * 0.26,
            color,
        );
    }
    // Les yeux, creusés dans la couleur du fond pour rester lisibles
    // quelle que soit la teinte du bouton.
    for sens in [-1.0f32, 1.0] {
        painter.circle_filled(
            egui::pos2(centre.x + sens * l * 0.19, centre.y),
            h * 0.16,
            egui::Color32::from_black_alpha(210),
        );
    }
}

/// Le chat de GitHub, ramené à ce qui le fait reconnaître : une tête
/// ronde, deux oreilles, une queue.
pub fn icon_github(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let centre = rect.center();
    // 0,37 et non 0,42 : les oreilles et la queue montent à 1,35 fois ce
    // rayon, si bien qu'à 0,42 le chat dépassait de sa case d'un huitième
    // en haut comme en bas — assez pour mordre le texte du lien voisin.
    // Le rayon est donc choisi pour que ces extrémités-là, et non le
    // cercle de la tête, touchent le bord.
    let r = rect.width() * 0.37;
    // Les oreilles d'abord : elles dépassent de la tête.
    for sens in [-1.0f32, 1.0] {
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(centre.x + sens * r * 0.75, centre.y - r * 0.55),
                egui::pos2(centre.x + sens * r * 0.35, centre.y - r * 1.35),
                egui::pos2(centre.x + sens * r * 0.05, centre.y - r * 0.75),
            ],
            color,
            egui::Stroke::NONE,
        ));
    }
    painter.circle_filled(centre, r, color);
    // La queue : le trait qui descend à gauche et revient.
    let stroke = egui::Stroke::new(1.4, color);
    painter.add(egui::Shape::line(
        vec![
            egui::pos2(centre.x - r * 0.55, centre.y + r * 0.85),
            egui::pos2(centre.x - r * 0.9, centre.y + r * 1.2),
            egui::pos2(centre.x - r * 0.35, centre.y + r * 1.35),
        ],
        stroke,
    ));
    // Les yeux, comme pour Discord : creusés plutôt que peints.
    for sens in [-1.0f32, 1.0] {
        painter.circle_filled(
            egui::pos2(centre.x + sens * r * 0.35, centre.y - r * 0.05),
            r * 0.17,
            egui::Color32::from_black_alpha(210),
        );
    }
}

/// Une loupe : un cercle et son manche.
///
/// Dessinée plutôt qu'écrite, comme les autres marques de l'interface.
/// L'emoji « 🔍 » n'est dans aucune des polices qu'egui charge par
/// défaut : il se peindrait en carré vide, ce qu'un test du projet
/// attrape — et il aurait raison, un champ de recherche annoncé par un
/// rectangle n'annonce rien.
pub fn icon_search(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.4, color);
    // Le verre occupe les deux tiers hauts-gauche, le manche descend en
    // diagonale vers le bas-droit : c'est la forme qu'on reconnaît sans
    // la regarder.
    let r = rect.width() * 0.32;
    let centre = egui::pos2(rect.left() + r + 1.0, rect.top() + r + 1.0);
    painter.circle_stroke(centre, r, trait_);
    let départ = centre + egui::vec2(r * 0.72, r * 0.72);
    painter.line_segment([départ, rect.right_bottom() - egui::vec2(1.0, 1.0)], trait_);
}

/// L'icône de l'analyse : un engrenage, et une clé dedans.
///
/// C'est celle de Desdec, l'autre atelier de la maison. Reprendre le
/// même dessin n'est pas une coquetterie : deux programmes qui font le
/// même geste — ouvrir un binaire et le démonter — gagnent à le dire de
/// la même façon, et l'on reconnaît un bouton avant de lire son
/// libellé.
///
/// Dessinée plutôt qu'importée : une image au trait perd tout à seize
/// pixels, et surtout elle ne suit pas la couleur du thème. Ici le trait
/// est celui qu'on lui passe, et il change avec le thème comme le reste.
pub fn icon_analysis(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    // L'engrenage et la clé de Desdec — mais côte à côte, et non l'un
    // dans l'autre.
    //
    // Superposés, comme ils l'étaient, la clé tenait dans le moyeu :
    // quatre pixels pour un manche et une fourche, ce qui donnait une
    // rayure en travers d'un rouage. Chacun sur sa diagonale, chacun a
    // la moitié de la place, et les deux se reconnaissent.
    let côté = rect.width().min(rect.height());
    let trait_ = egui::Stroke::new((côté * 0.09).clamp(1.0, 1.6), color);

    // L'engrenage, en haut à gauche. Six dents et non huit : à trois
    // pixels de rayon, huit traits se rejoignent en un disque.
    let moyeu = rect.min + egui::vec2(côté * 0.34, côté * 0.34);
    let r = côté * 0.18;
    painter.circle_stroke(moyeu, r, trait_);
    for dent in 0..6 {
        let angle = std::f32::consts::TAU * dent as f32 / 6.0;
        let direction = egui::vec2(angle.cos(), angle.sin());
        painter.line_segment(
            [moyeu + direction * r, moyeu + direction * (r * 1.7)],
            trait_,
        );
    }

    // La clé, en bas à droite — mais sur l'AUTRE diagonale. Alignée
    // sur celle de l'engrenage, elle en prolongeait les dents et les
    // deux se lisaient comme une clé de porte : un panneton et un
    // anneau. Croisée, chacune garde sa forme.
    let bout = rect.min + egui::vec2(côté * 0.94, côté * 0.52);
    let tête = rect.min + egui::vec2(côté * 0.52, côté * 0.94);
    painter.line_segment([bout, tête], trait_);
    let direction = (tête - bout).normalized();
    let travers = egui::vec2(-direction.y, direction.x) * (côté * 0.13);
    let gauche = tête - direction * (côté * 0.14) + travers;
    let droite = tête - direction * (côté * 0.14) - travers;
    painter.line_segment([gauche, droite], trait_);
    painter.line_segment([gauche, gauche + direction * (côté * 0.14)], trait_);
    painter.line_segment([droite, droite + direction * (côté * 0.14)], trait_);
}

/// L'icône de la calculatrice : un boîtier, un écran, quatre touches.
pub fn icon_calculator(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    // Dessinée pour douze pixels, et c'est tout le sujet.
    //
    // Le premier dessin superposait deux rectangles tracés au trait de
    // 1,2 px dans un boîtier de cinq pixels de large : le contour seul
    // en mangeait deux et demi, l'écran s'y peignait plein, et les
    // quatre touches débordaient du boîtier. Rendue, l'icône était une
    // tache blanche avec un trou — « elle ne ressemble à rien », et
    // c'était exact.
    //
    // Trois règles en sont sorties, et elles valent pour toute icône de
    // cette taille : un seul trait tracé (le boîtier), ce qui est petit
    // est PLEIN plutôt que tracé, et l'espace entre deux marques ne
    // descend jamais sous l'épaisseur du trait.
    let trait_ = egui::Stroke::new(1.0, color);
    // Plus haut que large, comme l'objet — mais pas au point d'être une
    // barre : les trois quarts de la largeur laissent trois colonnes de
    // touches, qui sont ce qui fait reconnaître une calculatrice.
    let boîtier = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() * 0.76, rect.height() * 0.96),
    );
    painter.rect_stroke(boîtier, 1.5, trait_, egui::StrokeKind::Inside);

    let marge = (boîtier.width() * 0.16).max(1.0);
    let dedans = boîtier.shrink(marge);

    // L'écran : une barre pleine. Tracée, à cette taille, elle se
    // remplit toute seule de son propre contour.
    let écran = egui::Rect::from_min_max(
        dedans.left_top(),
        egui::pos2(dedans.right(), dedans.top() + dedans.height() * 0.26),
    );
    painter.rect_filled(écran, 0.5, color);

    // Trois colonnes, deux rangées : la grille est ce qu'on reconnaît.
    // Des points pleins, espacés d'au moins l'épaisseur du trait, sans
    // quoi ils se rejoignent en un pâté.
    let clavier = egui::Rect::from_min_max(
        egui::pos2(dedans.left(), écran.bottom() + marge * 0.7),
        dedans.right_bottom(),
    );
    let rayon = (clavier.width() * 0.09).max(0.6);
    for colonne in 0..3 {
        let x = clavier.left() + rayon + (clavier.width() - rayon * 2.0) * colonne as f32 / 2.0;
        for rangée in 0..2 {
            let y = clavier.top() + rayon + (clavier.height() - rayon * 2.0) * rangée as f32;
            painter.circle_filled(egui::pos2(x, y), rayon, color);
        }
    }
}

/// L'icône « effacer le dernier caractère » : la flèche à gauche avec
/// sa croix, comme sur tous les claviers.
///
/// Dessinée et non écrite : le caractère `←` n'est pas dans la police
/// d'egui, et le test qui vérifie chaque glyphe de l'interface l'a
/// attrapé avant qu'il n'arrive à l'écran sous forme de carré vide.
pub fn icon_backspace(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.3, color);
    let h = rect.height() * 0.62;
    let l = rect.width() * 0.86;
    let centre = rect.center();
    let pointe = egui::pos2(centre.x - l * 0.5, centre.y);
    let épaule = centre.x - l * 0.5 + h * 0.5;

    // Le contour : une pointe à gauche, un rectangle à droite.
    painter.add(egui::Shape::closed_line(
        vec![
            pointe,
            egui::pos2(épaule, centre.y - h * 0.5),
            egui::pos2(centre.x + l * 0.5, centre.y - h * 0.5),
            egui::pos2(centre.x + l * 0.5, centre.y + h * 0.5),
            egui::pos2(épaule, centre.y + h * 0.5),
        ],
        trait_,
    ));
    // La croix dedans.
    let bras = h * 0.22;
    let croix = egui::pos2(centre.x + l * 0.16, centre.y);
    painter.line_segment(
        [
            croix - egui::vec2(bras, bras),
            croix + egui::vec2(bras, bras),
        ],
        trait_,
    );
    painter.line_segment(
        [
            croix - egui::vec2(bras, -bras),
            croix + egui::vec2(bras, -bras),
        ],
        trait_,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une icône reste dans la case qu'on lui donne.
    ///
    /// Rien ne l'y oblige : le peintre accepte n'importe quelle
    /// coordonnée, et une icône qui déborde de deux pixels passe sur le
    /// bouton voisin sans que personne ne s'en plaigne — sauf l'œil,
    /// plus tard, sur une capture d'écran.
    #[test]
    fn an_icon_stays_inside_the_box_it_is_given() {
        let ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(200.0, 100.0),
            )),
            ..Default::default()
        };
        let case = egui::Rect::from_min_size(egui::pos2(40.0, 20.0), egui::vec2(16.0, 16.0));
        // Ce que le trait lui-même occupe : un segment épais posé sur le
        // bord en dépasse de la moitié de son épaisseur, et une courbe
        // d'un peu plus. Un pixel et demi de tolérance sur une icône de
        // seize laisse passer la géométrie du trait et arrête ce qu'on
        // cherche : une forme posée à côté de sa case.
        let permis = case.expand(1.5);

        /// Le nom d'une icône, et la fonction qui la dessine.
        type Dessin = (&'static str, fn(&egui::Painter, egui::Rect, egui::Color32));
        let dessins: [Dessin; 6] = [
            ("analyse", icon_analysis),
            ("retour arrière", icon_backspace),
            ("calculatrice", icon_calculator),
            ("recherche", icon_search),
            ("globe", icon_globe),
            ("github", icon_github),
        ];

        for (nom, dessine) in dessins {
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| {
                dessine(ui.painter(), case, egui::Color32::WHITE);
            });
            for clipped in &sortie.shapes {
                let emprise = clipped.shape.visual_bounding_rect();
                if !emprise.is_positive() {
                    continue;
                }
                assert!(
                    permis.contains_rect(emprise),
                    "l'icône « {nom} » déborde : {emprise:?} hors de {permis:?}"
                );
            }
            sortie.textures_delta.clear();
        }
    }
}
