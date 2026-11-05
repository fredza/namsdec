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

/// Une erreur : un disque plein, avec une barre couchée.
///
/// Le disque plutôt qu'un cercle vide : à quatorze pixels, un contour
/// d'un pixel et demi disparaît sur un fond sombre, et c'est le signe
/// qu'on doit voir du coin de l'œil.
pub(crate) fn icon_error(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let rayon = rect.width() * 0.46;
    painter.circle_filled(rect.center(), rayon, color);
    // La barre est creusée dans le disque : du fond, et non une
    // couleur — l'icône se pose sur le panneau comme sur le survol.
    let demi = egui::vec2(rayon * 0.52, rayon * 0.16);
    painter.rect_filled(
        egui::Rect::from_center_size(rect.center(), demi * 2.0),
        0.0,
        egui::Color32::from_black_alpha(190),
    );
}

/// Un avertissement : un triangle plein, pointe en haut.
pub(crate) fn icon_warning(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let demi = rect.width() * 0.5;
    let centre = rect.center();
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(centre.x, centre.y - demi),
            egui::pos2(centre.x + demi, centre.y + demi * 0.78),
            egui::pos2(centre.x - demi, centre.y + demi * 0.78),
        ],
        color,
        egui::Stroke::NONE,
    ));
    // Le point d'exclamation, creusé lui aussi.
    let sombre = egui::Color32::from_black_alpha(190);
    painter.rect_filled(
        egui::Rect::from_center_size(
            egui::pos2(centre.x, centre.y + demi * 0.02),
            egui::vec2(demi * 0.22, demi * 0.62),
        ),
        0.0,
        sombre,
    );
    painter.circle_filled(
        egui::pos2(centre.x, centre.y + demi * 0.56),
        demi * 0.13,
        sombre,
    );
}

/// Un chevron : deux segments, pointe en haut ou en bas.
///
/// DESSINÉ, ET NON ÉCRIT. « ↑ » et « ↓ » se sont peints en carrés vides
/// sur le compteur de problèmes — la police de l'interface ne les porte
/// pas, et un caractère absent ne se voit qu'à la capture. Les icônes
/// de la barre d'outils sont dessinées pour la même raison.
pub(crate) fn icon_chevron(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    vers_le_bas: bool,
) {
    let demi = rect.width() * 0.3;
    let centre = rect.center();
    let sens = if vers_le_bas { 1.0 } else { -1.0 };
    let hauteur = demi * 0.62;
    painter.add(egui::Shape::line(
        vec![
            egui::pos2(centre.x - demi, centre.y - sens * hauteur),
            egui::pos2(centre.x, centre.y + sens * hauteur),
            egui::pos2(centre.x + demi, centre.y - sens * hauteur),
        ],
        egui::Stroke::new(1.4, color),
    ));
}

/// Un dossier : une chemise à onglet.
///
/// UN SEUL CONTOUR, et non un rectangle plus un trait : les deux
/// dessins séparés laissaient l'onglet flotter au-dessus de la boîte,
/// et l'on voyait un couvercle mal posé. La silhouette entière se suit
/// d'un trait, comme on la dessinerait à la main.
pub(crate) fn icon_folder(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.3, color);
    let largeur = rect.width() * 0.84;
    let hauteur = rect.height() * 0.64;
    let g = rect.center().x - largeur / 2.0;
    let h = rect.center().y - hauteur / 2.0;
    let épaule = h + hauteur * 0.24;
    painter.add(egui::Shape::closed_line(
        vec![
            egui::pos2(g, h + hauteur),
            egui::pos2(g, épaule),
            egui::pos2(g + largeur * 0.36, épaule),
            egui::pos2(g + largeur * 0.46, h),
            egui::pos2(g + largeur, h),
            egui::pos2(g + largeur, h + hauteur),
        ],
        trait_,
    ));
}

/// Un terminal : un cadre, une invite et un curseur.
pub(crate) fn icon_terminal(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.3, color);
    let côté = rect.width() * 0.82;
    let cadre = egui::Rect::from_center_size(rect.center(), egui::vec2(côté, côté * 0.82));
    painter.rect_stroke(cadre, 1.5, trait_, egui::StrokeKind::Inside);
    // Le chevron de l'invite, puis le trait de la ligne tapée.
    let g = cadre.left() + côté * 0.2;
    let y = cadre.center().y;
    painter.add(egui::Shape::line(
        vec![
            egui::pos2(g, y - côté * 0.16),
            egui::pos2(g + côté * 0.16, y),
            egui::pos2(g, y + côté * 0.16),
        ],
        trait_,
    ));
    painter.line_segment(
        [
            egui::pos2(g + côté * 0.26, y + côté * 0.16),
            egui::pos2(cadre.right() - côté * 0.18, y + côté * 0.16),
        ],
        trait_,
    );
}

/// Un processeur : un carré et ses broches.
pub(crate) fn icon_chip(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.3, color);
    let côté = rect.width() * 0.56;
    let corps = egui::Rect::from_center_size(rect.center(), egui::vec2(côté, côté));
    painter.rect_stroke(corps, 1.0, trait_, egui::StrokeKind::Inside);
    let broche = rect.width() * 0.14;
    for part in 0..3 {
        let t = -0.3 + part as f32 * 0.3;
        // Les quatre côtés, deux broches chacun : moins ne se
        // reconnaîtrait pas, plus se brouillerait à cette taille.
        painter.line_segment(
            [
                egui::pos2(corps.left() + côté * (t + 0.5), corps.top()),
                egui::pos2(corps.left() + côté * (t + 0.5), corps.top() - broche),
            ],
            trait_,
        );
        painter.line_segment(
            [
                egui::pos2(corps.left() + côté * (t + 0.5), corps.bottom()),
                egui::pos2(corps.left() + côté * (t + 0.5), corps.bottom() + broche),
            ],
            trait_,
        );
        painter.line_segment(
            [
                egui::pos2(corps.left(), corps.top() + côté * (t + 0.5)),
                egui::pos2(corps.left() - broche, corps.top() + côté * (t + 0.5)),
            ],
            trait_,
        );
        painter.line_segment(
            [
                egui::pos2(corps.right(), corps.top() + côté * (t + 0.5)),
                egui::pos2(corps.right() + broche, corps.top() + côté * (t + 0.5)),
            ],
            trait_,
        );
    }
}

/// Les exercices : un livre ouvert.
pub(crate) fn icon_book(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.3, color);
    let largeur = rect.width() * 0.86;
    let hauteur = rect.height() * 0.64;
    let centre = rect.center();
    for sens in [-1.0_f32, 1.0] {
        painter.add(egui::Shape::line(
            vec![
                egui::pos2(centre.x, centre.y - hauteur * 0.36),
                egui::pos2(centre.x + sens * largeur * 0.42, centre.y - hauteur * 0.5),
                egui::pos2(centre.x + sens * largeur * 0.42, centre.y + hauteur * 0.36),
                egui::pos2(centre.x, centre.y + hauteur * 0.5),
            ],
            trait_,
        ));
    }
    // La reliure, qui fait les deux pages plutôt qu'un losange.
    painter.line_segment(
        [
            egui::pos2(centre.x, centre.y - hauteur * 0.36),
            egui::pos2(centre.x, centre.y + hauteur * 0.5),
        ],
        trait_,
    );
}

/// Relancer : la flèche circulaire de la réinitialisation, mais qui
/// porte une pointe pleine — on repart, on ne remet pas à zéro.
pub(crate) fn icon_rerun(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.4, color);
    let centre = rect.center();
    let rayon = rect.width() * 0.34;
    // Un arc ouvert en haut à droite, par où la pointe entre.
    let points: Vec<egui::Pos2> = (0..=22)
        .map(|i| {
            let t = -0.42 + i as f32 * (1.76 / 22.0);
            let a = t * std::f32::consts::TAU;
            egui::pos2(centre.x + rayon * a.sin(), centre.y - rayon * a.cos())
        })
        .collect();
    painter.add(egui::Shape::line(points, trait_));
    // La pointe, pleine : c'est elle qui dit « ça repart ».
    let pointe = egui::pos2(centre.x + rayon * 0.72, centre.y - rayon * 0.72);
    let c = rect.width() * 0.13;
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(pointe.x + c, pointe.y + c * 0.2),
            egui::pos2(pointe.x - c * 0.6, pointe.y + c * 1.3),
            egui::pos2(pointe.x - c * 0.2, pointe.y - c * 1.1),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

/// Effacer : une corbeille — un couvercle, un corps.
pub(crate) fn icon_trash(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let trait_ = egui::Stroke::new(1.3, color);
    let largeur = rect.width() * 0.56;
    let centre = rect.center();
    let haut = centre.y - rect.height() * 0.26;
    // Le couvercle, et sa petite anse.
    painter.line_segment(
        [
            egui::pos2(centre.x - largeur * 0.7, haut),
            egui::pos2(centre.x + largeur * 0.7, haut),
        ],
        trait_,
    );
    painter.line_segment(
        [
            egui::pos2(centre.x - largeur * 0.24, haut - rect.height() * 0.1),
            egui::pos2(centre.x + largeur * 0.24, haut - rect.height() * 0.1),
        ],
        trait_,
    );
    // Le corps, légèrement évasé vers le haut : une corbeille, et non
    // une boîte.
    painter.add(egui::Shape::line(
        vec![
            egui::pos2(centre.x - largeur * 0.52, haut + 2.0),
            egui::pos2(centre.x - largeur * 0.4, centre.y + rect.height() * 0.3),
            egui::pos2(centre.x + largeur * 0.4, centre.y + rect.height() * 0.3),
            egui::pos2(centre.x + largeur * 0.52, haut + 2.0),
        ],
        trait_,
    ));
}
