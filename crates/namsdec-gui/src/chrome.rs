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
        let couleur = bouton.colour();
        // Éteinte tant que la souris n'est pas dessus : trois pastilles
        // vives en permanence tirent l'œil hors du code, qui est ce
        // qu'on est venu lire.
        let remplissage = if réponse.hovered() {
            couleur
        } else {
            couleur.gamma_multiply(0.55)
        };
        ui.painter()
            .circle_filled(centre, BUTTON_RADIUS, remplissage);
        // Le signe n'apparaît qu'au survol, comme sur les pastilles
        // qu'on connaît : la couleur suffit à désigner, le signe
        // confirme.
        if réponse.hovered() {
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
                    let carré =
                        egui::Rect::from_center_size(centre, egui::vec2(bras * 1.7, bras * 1.7));
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

/// Rend la zone vide de la barre de menus déplaçable, et le double-clic
/// maximisant — les deux gestes qu'on fait sans y penser sur une barre
/// de titre. Y écrit aussi ce qu'on y cherche : le nom du fichier.
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
    // Le nom du fichier, centré dans ce qui reste entre les menus et les
    // pastilles. Une barre de titre sans titre laisse deviner, à l'alt-
    // tab comme au réveil, sur quoi l'on travaillait.
    let texte = if modifié {
        format!("{titre} •")
    } else {
        titre.to_owned()
    };
    let galley = ui.painter().layout_no_wrap(
        texte,
        egui::TextStyle::Body.resolve(ui.style()),
        if modifié { theme.changed } else { theme.dim },
    );
    if galley.size().x < bande.width() {
        ui.painter()
            .galley(bande.center() - galley.size() * 0.5, galley, theme.dim);
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
    let r = rect.width() * 0.42;
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
