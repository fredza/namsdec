//! Explorateur de fichiers façon neo-tree (nvim) : curseur de sélection,
//! navigation au clavier, connecteurs d'arborescence, icônes de dossier
//! ouvert/fermé et filtre incrémental.
//!
//! Le rendu part d'une **liste aplatie** des lignes visibles, reconstruite
//! à chaque frame depuis le disque et l'ensemble des dossiers dépliés. Ce
//! détour — plutôt qu'un parcours récursif qui dessine au fil de l'eau —
//! est ce qui rend possibles la sélection, la navigation clavier et le
//! défilement automatique : tout devient un simple index dans un `Vec`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use eframe::egui;

const INDENT: f32 = 16.0;
const ROW_HEIGHT: f32 = 22.0;
const LEFT_PAD: f32 = 6.0;
/// Largeur réservée à l'icône/au badge, pour que les noms s'alignent que
/// la ligne soit un dossier ou un fichier.
const ICON_W: f32 = 22.0;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(0xe0, 0x9a, 0x5a);
const FOLDER_ICON: egui::Color32 = egui::Color32::from_rgb(0x2a, 0xc3, 0xde);
const FOLDER_TEXT: egui::Color32 = egui::Color32::from_rgb(0x7d, 0xcf, 0xff);

pub enum ExplorerAction {
    None,
    OpenFile(PathBuf),
    /// `Échap` : rendre le focus clavier à l'éditeur.
    Defocus,
}

#[derive(Default)]
pub struct FileExplorer {
    pub root: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    selected: usize,
    filter: String,
    request_filter_focus: bool,
    scroll_to_selection: bool,
}

/// Une ligne visible de l'arborescence, à plat.
struct Row {
    path: PathBuf,
    name: String,
    depth: usize,
    is_dir: bool,
    is_expanded: bool,
    /// Pour chaque niveau `d` : reste-t-il des frères après l'ancêtre de ce
    /// niveau ? Sert à savoir où tracer les `│` de continuation et si le
    /// connecteur de la ligne est un `├` ou un `└`. Le dernier élément
    /// concerne la ligne elle-même.
    guides: Vec<bool>,
}

impl FileExplorer {
    /// Affiche l'arborescence. `focused` indique si l'explorateur a le
    /// focus clavier (sinon les touches sont ignorées et laissées à
    /// l'éditeur).
    pub fn ui(&mut self, ui: &mut egui::Ui, focused: bool, filter_hint: &str) -> ExplorerAction {
        let Some(root) = self.root.clone() else {
            return ExplorerAction::None;
        };

        let everything = self.build_rows(&root);
        let total = everything.len();
        let rows = filter_rows(everything, &self.filter);
        if self.selected >= rows.len() {
            self.selected = rows.len().saturating_sub(1);
        }

        let filter_focused = self.header_ui(ui, rows.len(), total, filter_hint);

        let mut action = if focused && !filter_focused {
            self.handle_keys(ui, &rows)
        } else {
            ExplorerAction::None
        };

        let mut toggle: Option<PathBuf> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, row) in rows.iter().enumerate() {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ROW_HEIGHT),
                        egui::Sense::click(),
                    );

                    if response.clicked() {
                        self.selected = index;
                        if row.is_dir {
                            toggle = Some(row.path.clone());
                        } else {
                            action = ExplorerAction::OpenFile(row.path.clone());
                        }
                    }

                    let is_selected = index == self.selected;
                    if is_selected && self.scroll_to_selection {
                        ui.scroll_to_rect(rect, None);
                    }
                    if ui.is_rect_visible(rect) {
                        paint_row(ui, rect, row, is_selected, response.hovered());
                    }
                }
            });
        self.scroll_to_selection = false;

        if let Some(path) = toggle {
            self.toggle(&path);
        }

        action
    }

    fn toggle(&mut self, path: &Path) {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_path_buf());
        }
    }

    /// Raccourcis de neo-tree : `j`/`k` (ou flèches) pour se déplacer,
    /// `l`/`Entrée` pour ouvrir ou déplier, `h` pour replier ou remonter au
    /// parent, `g`/`G` pour les extrémités, `/` pour filtrer, `Échap` pour
    /// vider le filtre puis rendre le focus à l'éditeur.
    fn handle_keys(&mut self, ui: &egui::Ui, rows: &[Row]) -> ExplorerAction {
        let keys: Vec<(egui::Key, egui::Modifiers)> = ui.ctx().input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, *modifiers)),
                    _ => None,
                })
                .collect()
        });

        let mut action = ExplorerAction::None;
        for (key, modifiers) in keys {
            if modifiers.ctrl || modifiers.alt || modifiers.mac_cmd {
                continue;
            }
            match key {
                egui::Key::J | egui::Key::ArrowDown => self.move_selection(1, rows.len()),
                egui::Key::K | egui::Key::ArrowUp => self.move_selection(-1, rows.len()),
                egui::Key::G => {
                    // `G` (majuscule) va en bas, `g` en haut — comme dans vim.
                    self.selected = if modifiers.shift { rows.len().saturating_sub(1) } else { 0 };
                    self.scroll_to_selection = true;
                }
                egui::Key::Home => {
                    self.selected = 0;
                    self.scroll_to_selection = true;
                }
                egui::Key::End => {
                    self.selected = rows.len().saturating_sub(1);
                    self.scroll_to_selection = true;
                }
                egui::Key::L | egui::Key::Enter | egui::Key::O | egui::Key::ArrowRight => {
                    if let Some(row) = rows.get(self.selected) {
                        if row.is_dir {
                            // La racine reste toujours dépliée : la replier
                            // ne laisserait qu'une ligne et aucun moyen
                            // évident de revenir en arrière.
                            if row.depth > 0 {
                                self.toggle(&row.path);
                            }
                        } else {
                            action = ExplorerAction::OpenFile(row.path.clone());
                        }
                    }
                }
                egui::Key::H | egui::Key::ArrowLeft => self.collapse_or_go_to_parent(rows),
                egui::Key::Slash => self.request_filter_focus = true,
                egui::Key::Escape => {
                    if self.filter.is_empty() {
                        action = ExplorerAction::Defocus;
                    } else {
                        self.filter.clear();
                    }
                }
                _ => {}
            }
        }
        action
    }

    fn move_selection(&mut self, delta: isize, len: usize) {
        if len == 0 {
            return;
        }
        let next = self.selected as isize + delta;
        self.selected = next.clamp(0, len as isize - 1) as usize;
        self.scroll_to_selection = true;
    }

    fn collapse_or_go_to_parent(&mut self, rows: &[Row]) {
        let Some(row) = rows.get(self.selected) else {
            return;
        };
        if row.is_dir && row.is_expanded {
            self.toggle(&row.path.clone());
            return;
        }
        // Sinon on remonte : la première ligne au-dessus dont la profondeur
        // est strictement plus faible est le dossier parent.
        let depth = row.depth;
        if let Some(parent) = (0..self.selected).rev().find(|&i| rows[i].depth < depth) {
            self.selected = parent;
            self.scroll_to_selection = true;
        }
    }

    /// En-tête : titre centré entre deux filets, puis la boîte de filtre
    /// avec son compteur `visibles/total`. Renvoie `true` si le champ de
    /// filtre a le focus clavier (auquel cas `j`/`k` doivent s'y taper au
    /// lieu de déplacer la sélection).
    fn header_ui(&mut self, ui: &mut egui::Ui, shown: usize, total: usize, hint: &str) -> bool {
        ui.add_space(6.0);

        let title = "Explorer";
        let font = egui::FontId::monospace(13.0);
        let title_width = ui
            .painter()
            .layout_no_wrap(title.to_owned(), font.clone(), ACCENT)
            .size()
            .x;
        let side = ((ui.available_width() - title_width - 16.0) * 0.5).max(4.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            rule(ui, side);
            ui.colored_label(ACCENT, title);
            rule(ui, side);
        });

        let mut filter_focused = false;
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, ACCENT))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(6, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (chevron, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 12.0), egui::Sense::hover());
                    paint_triangle(ui.painter(), chevron.center(), false, FOLDER_TEXT);

                    let count = format!("{shown}/{total}");
                    let count_width = ui
                        .painter()
                        .layout_no_wrap(count.clone(), font.clone(), ACCENT)
                        .size()
                        .x;
                    let field_width = (ui.available_width() - count_width - 10.0).max(24.0);

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.filter)
                            .frame(egui::Frame::NONE)
                            .hint_text(hint)
                            .desired_width(field_width),
                    );
                    if self.request_filter_focus {
                        self.request_filter_focus = false;
                        response.request_focus();
                    }
                    filter_focused = response.has_focus();

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(ui.visuals().weak_text_color(), count);
                    });
                });
            });

        ui.add_space(6.0);
        filter_focused
    }

    fn build_rows(&self, root: &Path) -> Vec<Row> {
        let mut rows = vec![Row {
            path: root.to_path_buf(),
            name: root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| root.display().to_string()),
            depth: 0,
            is_dir: true,
            is_expanded: true,
            guides: Vec::new(),
        }];
        let mut guides = Vec::new();
        self.collect(root, &mut guides, &mut rows);
        rows
    }

    fn collect(&self, dir: &Path, guides: &mut Vec<bool>, rows: &mut Vec<Row>) {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return;
        };
        // Dossiers d'abord, puis ordre alphabétique insensible à la casse.
        let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| {
            (
                !e.path().is_dir(),
                e.file_name().to_string_lossy().to_lowercase(),
            )
        });

        for (index, entry) in entries.iter().enumerate() {
            let path = entry.path();
            let is_dir = path.is_dir();
            let is_expanded = is_dir && self.expanded.contains(&path);

            guides.push(index + 1 < entries.len());
            rows.push(Row {
                name: entry.file_name().to_string_lossy().into_owned(),
                depth: guides.len(),
                is_dir,
                is_expanded,
                guides: guides.clone(),
                path: path.clone(),
            });
            if is_expanded {
                self.collect(&path, guides, rows);
            }
            guides.pop();
        }
    }
}

/// Ne garde que les lignes dont le nom contient le filtre — plus leurs
/// dossiers parents, sans quoi les correspondances apparaîtraient
/// détachées de l'arborescence.
fn filter_rows(rows: Vec<Row>, filter: &str) -> Vec<Row> {
    if filter.is_empty() {
        return rows;
    }
    let needle = filter.to_lowercase();

    let mut keep = vec![false; rows.len()];
    for i in 0..rows.len() {
        if !rows[i].name.to_lowercase().contains(&needle) {
            continue;
        }
        keep[i] = true;
        let mut depth = rows[i].depth;
        for j in (0..i).rev() {
            if rows[j].depth < depth {
                keep[j] = true;
                depth = rows[j].depth;
                if depth == 0 {
                    break;
                }
            }
        }
    }

    rows.into_iter()
        .zip(keep)
        .filter_map(|(row, keep)| keep.then_some(row))
        .collect()
}

fn paint_row(ui: &egui::Ui, rect: egui::Rect, row: &Row, selected: bool, hovered: bool) {
    let painter = ui.painter();

    if selected {
        let fill = ui.visuals().selection.bg_fill;
        painter.rect_filled(rect, 0, fill.gamma_multiply(0.45));
        // Le petit repère vertical au bord gauche, comme neo-tree.
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
            0,
            fill,
        );
    } else if hovered {
        painter.rect_filled(rect, 0, ui.visuals().widgets.hovered.weak_bg_fill);
    }

    paint_guides(painter, rect, row, ui.visuals().weak_text_color().gamma_multiply(0.4));

    let icon_left = rect.left() + LEFT_PAD + row.depth as f32 * INDENT;
    let icon_center = egui::pos2(icon_left + ICON_W * 0.5, rect.center().y);
    let font = egui::FontId::monospace(13.0);

    if row.is_dir {
        paint_folder_icon(painter, icon_center, FOLDER_ICON, row.is_expanded);
    } else {
        let (badge, color) = file_badge(&row.name);
        let galley = painter.layout_no_wrap(badge, egui::FontId::monospace(11.0), color);
        let pos = egui::pos2(
            icon_center.x - galley.size().x * 0.5,
            icon_center.y - galley.size().y * 0.5,
        );
        painter.galley(pos, galley, color);
    }

    let text_color = if row.is_dir {
        FOLDER_TEXT
    } else {
        ui.visuals().text_color()
    };
    painter.text(
        egui::pos2(icon_left + ICON_W + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &row.name,
        font,
        text_color,
    );
}

/// Les `│`, `├` et `└` de l'arborescence, tracés en segments plutôt qu'en
/// caractères : la police par défaut d'egui ne garantit pas ces glyphes.
fn paint_guides(painter: &egui::Painter, rect: egui::Rect, row: &Row, color: egui::Color32) {
    if row.depth == 0 {
        return;
    }
    let stroke = egui::Stroke::new(1.0, color);
    let guide_x = |depth: usize| rect.left() + LEFT_PAD + depth as f32 * INDENT + INDENT * 0.5;

    // Continuations verticales des ancêtres qui ont encore des frères.
    for depth in 0..row.depth - 1 {
        if row.guides[depth] {
            painter.vline(guide_x(depth), rect.y_range(), stroke);
        }
    }

    // Connecteur de la ligne elle-même : `├` s'il reste des frères après
    // elle, `└` si c'est la dernière.
    let own = row.depth - 1;
    let x = guide_x(own);
    let center_y = rect.center().y;
    let bottom = if row.guides[own] { rect.bottom() } else { center_y };
    painter.vline(x, egui::Rangef::new(rect.top(), bottom), stroke);
    painter.hline(
        egui::Rangef::new(x, rect.left() + LEFT_PAD + row.depth as f32 * INDENT + 2.0),
        center_y,
        stroke,
    );
}

fn paint_triangle(painter: &egui::Painter, center: egui::Pos2, down: bool, color: egui::Color32) {
    let s = 4.0;
    let points = if down {
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

/// Dossier fermé plein, dossier ouvert avec sa face avant inclinée — c'est
/// l'icône elle-même qui dit l'état, comme dans neo-tree, plutôt qu'un
/// chevron séparé qui alourdirait chaque ligne.
fn paint_folder_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32, open: bool) {
    let r = egui::Rect::from_center_size(center, egui::vec2(14.0, 11.0));
    painter.rect_filled(
        egui::Rect::from_min_size(egui::pos2(r.left(), r.top()), egui::vec2(6.0, 3.0)),
        1,
        color,
    );

    if open {
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(r.left(), r.top() + 2.0), egui::pos2(r.right() - 3.0, r.bottom())),
            1,
            color.gamma_multiply(0.5),
        );
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(r.left() + 2.5, r.bottom()),
                egui::pos2(r.right(), r.bottom()),
                egui::pos2(r.right() - 2.5, r.top() + 4.5),
                egui::pos2(r.left(), r.top() + 4.5),
            ],
            color,
            egui::Stroke::NONE,
        ));
    } else {
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(r.left(), r.top() + 2.0), r.max),
            2,
            color,
        );
    }
}

/// Badge `[X]` coloré selon l'extension — même principe visuel que
/// l'arborescence de référence (qui affiche `[T]` pour les `.toml`), sans
/// dépendre d'une police d'icônes type nerd-font.
fn file_badge(name: &str) -> (String, egui::Color32) {
    if name.eq_ignore_ascii_case("LICENSE") {
        return ("[L]".to_owned(), egui::Color32::from_rgb(0x4e, 0xc9, 0xb0));
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .or_else(|| name.eq_ignore_ascii_case("Cargo.lock").then(|| "toml".to_owned()));

    match ext.as_deref() {
        Some("toml") => ("[T]".to_owned(), egui::Color32::from_rgb(0xdc, 0x98, 0x2b)),
        Some("rs") => ("[R]".to_owned(), egui::Color32::from_rgb(0xde, 0x76, 0x56)),
        Some("asm" | "nasm" | "s") => ("[A]".to_owned(), egui::Color32::from_rgb(0x56, 0x9c, 0xd6)),
        Some("md") => ("[M]".to_owned(), egui::Color32::from_rgb(0x9c, 0xa3, 0xaf)),
        Some(other) => (
            format!("[{}]", other.chars().next().unwrap_or('?').to_ascii_uppercase()),
            egui::Color32::from_rgb(0x8a, 0x8a, 0x8a),
        ),
        None => ("[F]".to_owned(), egui::Color32::from_rgb(0x8a, 0x8a, 0x8a)),
    }
}

fn rule(ui: &mut egui::Ui, len: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(len.max(0.0), 8.0), egui::Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, egui::Stroke::new(1.0, ACCENT));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, depth: usize) -> Row {
        Row {
            path: PathBuf::from(name),
            name: name.to_owned(),
            depth,
            is_dir: depth == 0,
            is_expanded: false,
            guides: vec![false; depth],
        }
    }

    #[test]
    fn filter_keeps_matches_and_their_parents() {
        let rows = vec![
            row("racine", 0),
            row("crates", 1),
            row("namsdec-core", 2),
            row("lib.rs", 3),
            row("docs", 1),
            row("notes.md", 2),
        ];

        let kept: Vec<String> = filter_rows(rows, "lib")
            .into_iter()
            .map(|r| r.name)
            .collect();

        // "lib.rs" correspond ; ses trois ancêtres sont conservés pour que
        // la ligne reste rattachée à l'arbre. "docs"/"notes.md" disparaissent.
        assert_eq!(kept, ["racine", "crates", "namsdec-core", "lib.rs"]);
    }

    #[test]
    fn empty_filter_keeps_everything() {
        let rows = vec![row("racine", 0), row("a", 1), row("b", 1)];
        assert_eq!(filter_rows(rows, "").len(), 3);
    }

    /// Fait tourner une frame d'egui avec de vraies frappes clavier.
    /// `run_ui` renvoie un `FullOutput` dont `textures_delta` doit être
    /// vidé, sinon egui panique volontairement à sa destruction.
    fn frame(
        ctx: &egui::Context,
        keys: &[egui::Key],
        mut logic: impl FnMut(&mut egui::Ui),
    ) {
        let events = keys
            .iter()
            .map(|key| egui::Event::Key {
                key: *key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect();
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| logic(ui));
        output.textures_delta.clear();
    }

    fn names(explorer: &FileExplorer, root: &Path) -> Vec<String> {
        explorer
            .build_rows(root)
            .into_iter()
            .map(|r| r.name)
            .collect()
    }

    /// Navigation clavier de bout en bout sur une vraie arborescence :
    /// `j` descend, `l` déplie un dossier puis ouvre un fichier.
    #[test]
    fn keyboard_navigation_expands_then_opens_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "").unwrap();
        std::fs::write(root.join("Cargo.toml"), "").unwrap();

        let ctx = egui::Context::default();
        let mut explorer = FileExplorer {
            root: Some(root.clone()),
            ..Default::default()
        };

        // Dossier fermé au départ : sous la racine, `src` puis `Cargo.toml`
        // (dossiers d'abord), et pas encore `main.rs`.
        assert_eq!(
            names(&explorer, &root)[1..],
            ["src", "Cargo.toml"].map(|s| s.to_owned()),
            "les enfants de la racine, dossiers d'abord"
        );

        // `j` descend de la racine vers `src`.
        frame(&ctx, &[egui::Key::J], |ui| {
            explorer.ui(ui, true, "");
        });
        assert_eq!(explorer.selected, 1);

        // `l` déplie `src`.
        frame(&ctx, &[egui::Key::L], |ui| {
            explorer.ui(ui, true, "");
        });
        assert!(
            explorer.expanded.contains(&root.join("src")),
            "`l` sur un dossier doit le déplier"
        );
        assert_eq!(
            names(&explorer, &root)[1..],
            ["src", "main.rs", "Cargo.toml"].map(|s| s.to_owned()),
            "main.rs apparaît sous src une fois déplié"
        );

        // `j` puis `l` sur `main.rs` doit demander son ouverture.
        frame(&ctx, &[egui::Key::J], |ui| {
            explorer.ui(ui, true, "");
        });
        let mut opened = None;
        frame(&ctx, &[egui::Key::L], |ui| {
            if let ExplorerAction::OpenFile(path) = explorer.ui(ui, true, "") {
                opened = Some(path);
            }
        });
        assert_eq!(opened.as_deref(), Some(root.join("src/main.rs").as_path()));
    }

    /// Sans le focus, l'explorateur doit ignorer les touches — sinon il
    /// volerait `j`/`k`/`l` à l'éditeur.
    #[test]
    fn keys_are_ignored_when_not_focused() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir(root.join("src")).unwrap();

        let ctx = egui::Context::default();
        let mut explorer = FileExplorer {
            root: Some(root),
            ..Default::default()
        };

        frame(&ctx, &[egui::Key::J], |ui| {
            explorer.ui(ui, false, "");
        });

        assert_eq!(explorer.selected, 0);
    }
}
