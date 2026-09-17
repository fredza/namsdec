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
const BUTTON_SPACING: f32 = 10.0;

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
    /// Les trois, de gauche à droite. La fermeture à l'extrémité : c'est
    /// là que la main la cherche sous Linux comme sous Windows, et c'est
    /// aussi le bouton qu'on veut le plus loin des deux autres.
    pub const ALL: [WindowButton; 3] = [
        WindowButton::Minimize,
        WindowButton::Maximize,
        WindowButton::Close,
    ];

    /// Sa couleur, celle des pastilles qu'on reconnaît sans les lire.
    fn colour(self) -> egui::Color32 {
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
pub fn window_buttons_ui(ui: &mut egui::Ui, s: &Strings, maximized: bool) -> Option<WindowButton> {
    let mut cliqué = None;
    ui.horizontal(|ui| {
        for bouton in WindowButton::ALL {
            let (rect, réponse) = ui.allocate_exact_size(
                egui::vec2(BUTTON_RADIUS * 2.0 + BUTTON_SPACING, BUTTON_RADIUS * 2.0),
                egui::Sense::click(),
            );
            let centre = rect.center();
            let couleur = bouton.colour();
            // Éteinte tant que la souris n'est pas dessus : trois
            // pastilles vives en permanence tirent l'œil hors du code,
            // qui est ce qu'on est venu lire.
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
                        let carré = egui::Rect::from_center_size(
                            centre,
                            egui::vec2(bras * 1.7, bras * 1.7),
                        );
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
    });
    cliqué
}

/// Rend la zone vide de la barre de menus déplaçable, et le double-clic
/// maximisant — les deux gestes qu'on fait sans y penser sur une barre
/// de titre.
pub fn title_drag_ui(ui: &mut egui::Ui, maximized: bool) {
    let reste = ui.available_size_before_wrap();
    if reste.x <= 0.0 {
        return;
    }
    let (_, réponse) = ui.allocate_exact_size(reste, egui::Sense::click_and_drag());
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
