//! Explorateur de fichiers façon nvim-tree/neo-tree : arborescence dessinée
//! à la main (icônes de dossier, badges de fichier par extension, guides
//! d'indentation) plutôt que le `CollapsingHeader` générique d'egui, dont
//! le rendu ne laisse pas assez de contrôle pour ce style. Relu depuis le
//! disque à chaque affichage — pas de cache, les projets NASM restent
//! petits. N'importe quel fichier peut être ouvert, pas seulement les
//! `.asm` : c'est le comportement réel de VS Code/nvim-tree, qui ne
//! masquent pas les fichiers non reconnus.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use eframe::egui;

const INDENT: f32 = 18.0;
const ROW_HEIGHT: f32 = 22.0;
const FOLDER_COLOR: egui::Color32 = egui::Color32::from_rgb(0x7d, 0xd3, 0xfc);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x9a, 0x5a);

#[derive(Default)]
pub struct FileExplorer {
    pub root: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
}

impl FileExplorer {
    /// Affiche l'arborescence à partir de `root`. Renvoie le fichier
    /// cliqué, s'il y en a un — à charger dans l'éditeur par l'appelant.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let root = self.root.clone()?;
        let mut clicked = None;

        header_ui(ui, &root);

        egui::ScrollArea::vertical().show(ui, |ui| {
            root_row_ui(ui, &root);
            self.dir_rows_ui(ui, &root, 1, &mut clicked);
        });

        clicked
    }

    fn dir_rows_ui(&mut self, ui: &mut egui::Ui, dir: &Path, depth: u32, clicked: &mut Option<PathBuf>) {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };
        // Dossiers d'abord, puis ordre alphabétique dans chaque groupe.
        let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                let is_open = self.expanded.contains(&path);
                if dir_row_ui(ui, &name, depth, is_open) {
                    if is_open {
                        self.expanded.remove(&path);
                    } else {
                        self.expanded.insert(path.clone());
                    }
                }
                if self.expanded.contains(&path) {
                    self.dir_rows_ui(ui, &path, depth + 1, clicked);
                }
            } else if file_row_ui(ui, &name, depth) {
                *clicked = Some(path);
            }
        }
    }
}

fn header_ui(ui: &mut egui::Ui, root: &Path) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        rule(ui, 20.0);
        ui.colored_label(ACCENT, "Explorer");
        rule(ui, (ui.available_width() - 4.0).max(8.0));
    });

    let count = std::fs::read_dir(root).map(|d| d.count()).unwrap_or(0);
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, ACCENT))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(ui.visuals().weak_text_color(), count.to_string());
            });
        });
    ui.add_space(6.0);
}

fn rule(ui: &mut egui::Ui, len: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(len.max(0.0), 8.0), egui::Sense::hover());
    ui.painter().hline(rect.x_range(), rect.center().y, egui::Stroke::new(1.0, ACCENT));
}

fn root_row_ui(ui: &mut egui::Ui, root: &Path) {
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let icon_center = egui::pos2(rect.left() + 10.0, rect.center().y);
    paint_folder_icon(painter, icon_center, FOLDER_COLOR);
    painter.text(
        egui::pos2(rect.left() + 22.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::monospace(13.0),
        FOLDER_COLOR,
    );
}

fn dir_row_ui(ui: &mut egui::Ui, name: &str, depth: u32, is_open: bool) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return response.clicked();
    }

    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    paint_indent_guides(ui, rect, depth);

    let x0 = rect.left() + depth as f32 * INDENT;
    let painter = ui.painter();
    paint_disclosure_triangle(painter, egui::pos2(x0 + 6.0, rect.center().y), is_open, FOLDER_COLOR);
    paint_folder_icon(painter, egui::pos2(x0 + 20.0, rect.center().y), FOLDER_COLOR);
    painter.text(
        egui::pos2(x0 + 32.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::monospace(13.0),
        FOLDER_COLOR,
    );

    response.clicked()
}

fn file_row_ui(ui: &mut egui::Ui, name: &str, depth: u32) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return response.clicked();
    }

    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    paint_indent_guides(ui, rect, depth);

    let x0 = rect.left() + depth as f32 * INDENT;
    let font_id = egui::FontId::monospace(13.0);
    let (badge, badge_color) = file_badge(name);
    let painter = ui.painter();
    let badge_galley = painter.layout_no_wrap(badge, font_id.clone(), badge_color);
    let badge_pos = egui::pos2(x0, rect.center().y - badge_galley.size().y / 2.0);
    let badge_width = badge_galley.size().x;
    painter.galley(badge_pos, badge_galley, badge_color);
    painter.text(
        egui::pos2(x0 + badge_width + 6.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        font_id,
        ui.visuals().text_color(),
    );

    response.clicked()
}

fn paint_indent_guides(ui: &egui::Ui, row_rect: egui::Rect, depth: u32) {
    let color = ui.visuals().weak_text_color().gamma_multiply(0.35);
    let painter = ui.painter();
    for level in 1..=depth {
        let x = row_rect.left() + (level as f32 - 0.5) * INDENT;
        painter.vline(x, row_rect.y_range(), egui::Stroke::new(1.0, color));
    }
}

fn paint_disclosure_triangle(painter: &egui::Painter, center: egui::Pos2, open: bool, color: egui::Color32) {
    let s = 4.0;
    let points = if open {
        vec![
            egui::pos2(center.x - s, center.y - s * 0.6),
            egui::pos2(center.x + s, center.y - s * 0.6),
            egui::pos2(center.x, center.y + s * 0.8),
        ]
    } else {
        vec![
            egui::pos2(center.x - s * 0.6, center.y - s),
            egui::pos2(center.x - s * 0.6, center.y + s),
            egui::pos2(center.x + s * 0.8, center.y),
        ]
    };
    painter.add(egui::Shape::convex_polygon(points, color, egui::Stroke::NONE));
}

fn paint_folder_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let body = egui::Rect::from_center_size(center, egui::vec2(14.0, 10.0));
    painter.rect_filled(body, 2, color);
    let tab = egui::Rect::from_min_size(egui::pos2(body.left() + 1.0, body.top() - 3.0), egui::vec2(6.0, 3.0));
    painter.rect_filled(tab, 1, color);
}

/// Badge `[X]` coloré selon l'extension — le même principe visuel que
/// l'arborescence de référence (nvim-tree), sans dépendre d'une police
/// d'icônes : une lettre entre crochets suffit à distinguer les types de
/// fichiers courants d'un coup d'œil.
fn file_badge(name: &str) -> (String, egui::Color32) {
    if name.eq_ignore_ascii_case("LICENSE") {
        return ("[L]".to_string(), egui::Color32::from_rgb(0x4e, 0xc9, 0xb0));
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .or_else(|| name.eq_ignore_ascii_case("Cargo.lock").then(|| "toml".to_string()));

    match ext.as_deref() {
        Some("toml") => ("[T]".to_string(), egui::Color32::from_rgb(0xdc, 0x98, 0x2b)),
        Some("rs") => ("[R]".to_string(), egui::Color32::from_rgb(0xde, 0x76, 0x56)),
        Some("asm" | "nasm" | "s") => ("[A]".to_string(), ACCENT_CODE),
        Some("md") => ("[M]".to_string(), egui::Color32::from_rgb(0x9c, 0xa3, 0xaf)),
        Some(other) => (
            format!("[{}]", other.chars().next().unwrap_or('?').to_ascii_uppercase()),
            egui::Color32::from_rgb(0x8a, 0x8a, 0x8a),
        ),
        None => ("[F]".to_string(), egui::Color32::from_rgb(0x8a, 0x8a, 0x8a)),
    }
}

const ACCENT_CODE: egui::Color32 = egui::Color32::from_rgb(0x56, 0x9c, 0xd6);
