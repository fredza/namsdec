//! Les icônes dessinées à la main de la barre d'outils et des contrôles.
//!
//! Chacune reçoit un peintre, un rectangle et une couleur, et ne connaît
//! rien d'autre : ni l'état de l'application, ni le thème. Ce sont des
//! dessins purs, et c'est pourquoi ils se testent au pixel sans écran.
//! Sortis de `main.rs` pour l'alléger — ils n'y partageaient rien avec
//! le reste.

use super::*;

/// Surveiller : un œil. Deux arcs et une pupille — de quoi se
/// reconnaître à quatorze pixels sans ressembler au reste.
pub(crate) fn icon_watch(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.3, color);
    let centre = rect.center();
    let half = rect.width() * 0.5;
    // Les deux paupières, chacune une suite de segments.
    for sign in [-1.0, 1.0] {
        let points: Vec<egui::Pos2> = (0..=8)
            .map(|i| {
                let t = -1.0 + i as f32 * 0.25;
                egui::pos2(
                    centre.x + t * half,
                    centre.y + sign * (1.0 - t * t) * half * 0.62,
                )
            })
            .collect();
        painter.add(egui::Shape::line(points, stroke));
    }
    painter.circle_stroke(centre, half * 0.26, stroke);
}

/// « Aller à » : une flèche qui bute sur un montant, comme la fin d'une
/// course. Distincte de la flèche de Step Over, qui enjambe.
pub(crate) fn icon_go(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5, color);
    let y = rect.center().y;
    let right = rect.right() - 1.0;
    painter.line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(right - 2.5, y)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(right - 5.0, y - 3.0), egui::pos2(right - 2.0, y)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(right - 5.0, y + 3.0), egui::pos2(right - 2.0, y)],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(right, rect.top() + 1.0),
            egui::pos2(right, rect.bottom() - 1.0),
        ],
        stroke,
    );
}

pub(crate) fn icon_run(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let r = rect.shrink(1.0);
    painter.add(egui::Shape::convex_polygon(
        vec![
            r.left_top(),
            r.left_bottom(),
            egui::pos2(r.right(), r.center().y),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

pub(crate) fn icon_stop(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    painter.rect_filled(rect.shrink(2.0), 1.0, color);
}

pub(crate) fn icon_step_into(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let x = rect.center().x;
    let stroke = egui::Stroke::new(1.6, color);
    painter.line_segment(
        [
            egui::pos2(x, rect.top()),
            egui::pos2(x, rect.bottom() - 3.0),
        ],
        stroke,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(x - 3.0, rect.bottom() - 5.0),
            egui::pos2(x + 3.0, rect.bottom() - 5.0),
            egui::pos2(x, rect.bottom()),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

/// Le pas arrière : une flèche qui remonte, exactement l'inverse de celle
/// de Step Into, qui descend dans l'instruction.
pub(crate) fn icon_step_back(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let x = rect.center().x;
    painter.line_segment(
        [
            egui::pos2(x, rect.bottom() - 1.0),
            egui::pos2(x, rect.top() + 1.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x - 3.5, rect.top() + 4.5),
            egui::pos2(x, rect.top() + 1.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(x + 3.5, rect.top() + 4.5),
            egui::pos2(x, rect.top() + 1.0),
        ],
        stroke,
    );
}

/// La réinitialisation : une flèche qui fait presque le tour et revient
/// d'où elle part. L'ouverture est en haut, là où l'œil attend la
/// boucle ; la pointe marque le sens de la course.
pub(crate) fn icon_reset(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let centre = rect.center();
    let rayon = rect.width().min(rect.height()) / 2.0 - 1.5;

    // Un arc de trois cents degrés : le tour est fait, mais on voit
    // encore qu'il s'agit d'un cycle et non d'un disque.
    let depart = -std::f32::consts::PI * 0.35;
    let arrivee = depart + std::f32::consts::PI * 1.7;
    let points: Vec<egui::Pos2> = (0..=28)
        .map(|i| {
            let t = depart + (arrivee - depart) * (i as f32 / 28.0);
            centre + egui::vec2(t.cos(), t.sin()) * rayon
        })
        .collect();
    painter.add(egui::Shape::line(points, stroke));

    // La pointe, au bout de l'arc : deux traits en chevron, l'un et
    // l'autre pris sur la tangente et la normale du cercle en ce point,
    // pour qu'elle suive la courbe au lieu de la couper.
    let bout = centre + egui::vec2(arrivee.cos(), arrivee.sin()) * rayon;
    let tangente = egui::vec2(-arrivee.sin(), arrivee.cos());
    let normale = egui::vec2(arrivee.cos(), arrivee.sin());
    let taille = 3.5;
    painter.line_segment(
        [bout, bout - tangente * taille + normale * taille * 0.8],
        stroke,
    );
    painter.line_segment(
        [bout, bout - tangente * taille - normale * taille * 0.8],
        stroke,
    );
}

/// Agrandir : le cadre qu'on veut voir occuper toute la place.
pub(crate) fn icon_maximize(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    painter.rect_stroke(
        rect.shrink(1.5),
        1.0,
        egui::Stroke::new(1.5, color),
        egui::StrokeKind::Inside,
    );
}

/// Rendre sa taille : deux cadres décalés, celui de devant plus petit.
pub(crate) fn icon_restore(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.4, color);
    let petit = rect.shrink(1.5);
    // Celui de derrière, décalé vers le haut à droite.
    let derrière = egui::Rect::from_min_size(
        petit.min + egui::vec2(3.0, -2.0),
        petit.size() - egui::vec2(1.0, 1.0),
    );
    painter.rect_stroke(derrière, 1.0, trait_, egui::StrokeKind::Inside);
    // Celui de devant : peint par-dessus, fond compris, pour que le
    // recouvrement se lise.
    painter.rect_filled(
        egui::Rect::from_min_size(petit.min + egui::vec2(0.0, 2.0), petit.size()),
        1.0,
        egui::Color32::TRANSPARENT,
    );
    painter.rect_stroke(
        egui::Rect::from_min_size(petit.min + egui::vec2(0.0, 2.0), petit.size()),
        1.0,
        trait_,
        egui::StrokeKind::Inside,
    );
}

pub(crate) fn icon_step_over(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let y = rect.center().y;
    let stroke = egui::Stroke::new(1.6, color);
    painter.line_segment(
        [
            egui::pos2(rect.left(), y),
            egui::pos2(rect.right() - 3.0, y),
        ],
        stroke,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(rect.right() - 5.0, y - 3.0),
            egui::pos2(rect.right() - 5.0, y + 3.0),
            egui::pos2(rect.right(), y),
        ],
        color,
        egui::Stroke::NONE,
    ));
}
