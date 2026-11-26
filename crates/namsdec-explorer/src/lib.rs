//! Explorateur de fichiers façon neo-tree (nvim) : curseur de sélection,
//! navigation au clavier, connecteurs d'arborescence, icônes de dossier
//! ouvert/fermé et filtre incrémental.
//!
//! Crate à part entière, et pas un module de l'interface : l'explorateur
//! ne connaît ni l'éditeur, ni les préférences, ni la langue (ses libellés
//! lui sont passés). Il se présente comme une **fenêtre flottante
//! indépendante**, invoquée directement par son raccourci — il n'est
//! accroché à aucun panneau de la mise en page, et le fermer ne réorganise
//! rien autour de lui.
//!
//! Le rendu part d'une **liste aplatie** des lignes visibles, reconstruite
//! à chaque frame depuis le disque et l'ensemble des dossiers dépliés. Ce
//! détour — plutôt qu'un parcours récursif qui dessine au fil de l'eau —
//! est ce qui rend possibles la sélection, la navigation clavier et le
//! défilement automatique : tout devient un simple index dans un `Vec`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use namsdec_theme::Theme;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const INDENT: f32 = 16.0;
const ROW_HEIGHT: f32 = 22.0;
const LEFT_PAD: f32 = 6.0;
/// Largeur réservée à l'icône/au badge, pour que les noms s'alignent que
/// la ligne soit un dossier ou un fichier.
const ICON_W: f32 = 22.0;

/// Ce que l'explorateur demande à l'application hôte après une frame.
pub enum ExplorerAction {
    None,
    OpenFile(PathBuf),
    /// Aucun dossier ouvert et l'utilisateur en réclame un — c'est l'hôte
    /// qui ouvre le sélecteur natif, l'explorateur n'a pas à dépendre de
    /// `rfd`.
    PickRoot,
    /// `Échap` : refermer l'explorateur et rendre le focus à l'éditeur.
    Close,
    /// Un fichier vient d'être renommé sur le disque (l'explorateur a déjà
    /// fait le `rename`). L'hôte en est averti parce que lui seul sait si
    /// c'est le fichier ouvert dans l'éditeur, dont le chemin de
    /// sauvegarde doit alors suivre.
    Renamed {
        from: PathBuf,
        to: PathBuf,
    },
}

/// Ce qu'une frame de l'explorateur demande à l'hôte.
pub struct ExplorerResponse {
    pub action: ExplorerAction,
    /// Un clic à l'intérieur vient de lui donner le focus clavier.
    /// Renvoyé plutôt que décidé ici : c'est l'hôte qui arbitre le focus
    /// entre l'explorateur et son éditeur.
    pub focus_gained: bool,
}

/// Libellés fournis par l'hôte : la traduction reste de son côté.
pub struct Labels<'a> {
    pub title: &'a str,
    pub filter_hint: &'a str,
    pub empty: &'a str,
    pub open_folder: &'a str,
    /// Invite du champ de chemin, en haut de la fenêtre.
    pub path_hint: &'a str,
    /// Infobulles des raccourcis de lieux et du bouton « remonter ».
    pub go_parent: &'a str,
    pub go_root: &'a str,
    pub go_home: &'a str,
    pub go_current_file: &'a str,
    /// Renommage : invite du champ, et modèle du message d'échec. Le
    /// `{}` y est remplacé par la raison donnée par le système.
    pub rename_hint: &'a str,
    pub rename_failed: &'a str,
    pub rename_exists: &'a str,
    pub path_not_a_directory: &'a str,
}

/// Durée pendant laquelle le contenu d'un dossier déjà lu est réutilisé.
///
/// Sans elle, l'arborescence était relue **à chaque frame** : autant
/// d'appels à `read_dir`, plus un `stat` par entrée pour trier les
/// dossiers en premier, soixante fois par seconde. C'est tenable sur le
/// dossier d'un projet, et plus du tout dès qu'on ouvre l'explorateur sur
/// `/`, `/usr/lib` ou un montage réseau — là où l'utilisateur peut
/// désormais aller.
///
/// Deux secondes gardent l'affichage vivant (un fichier créé dans un autre
/// terminal apparaît sans rien faire) pour un coût redevenu négligeable ;
/// les opérations de l'explorateur lui-même, elles, vident le cache
/// aussitôt plutôt que d'attendre l'expiration.
const CACHE_TTL: Duration = Duration::from_secs(2);

/// Le contenu d'un dossier tel qu'il a été lu, et quand.
struct CachedDir {
    entries: Vec<Entry>,
    read_at: Instant,
}

/// Une entrée de dossier, réduite à ce que l'affichage utilise — le tri
/// (dossiers d'abord) est fait une fois à la lecture, pas à chaque frame.
#[derive(Clone)]
struct Entry {
    path: PathBuf,
    name: String,
    is_dir: bool,
}

/// Le renommage en cours : quelle ligne, et le nom tel qu'il est tapé.
struct Renaming {
    path: PathBuf,
    name: String,
    /// Le champ vient d'apparaître et doit réclamer le focus clavier.
    request_focus: bool,
}

#[derive(Default)]
pub struct FileExplorer {
    pub root: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    selected: usize,
    filter: String,
    request_filter_focus: bool,
    scroll_to_selection: bool,
    /// Contenu des dossiers déjà lus (voir `CACHE_TTL`).
    cache: HashMap<PathBuf, CachedDir>,
    /// Le champ de chemin, quand il est ouvert : la racine s'y tape à la
    /// main. `None` tant que l'utilisateur ne l'a pas ouvert.
    path_edit: Option<String>,
    /// Le champ de chemin vient d'être ouvert et réclame le clavier —
    /// une fois, à la frame suivante (même patron que
    /// `request_filter_focus`).
    request_path_focus: bool,
    renaming: Option<Renaming>,
    /// Dernière opération échouée (renommage, chemin invalide), affichée
    /// sous l'en-tête jusqu'à l'action suivante.
    error: Option<String>,
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
    /// Ouvre l'explorateur sur un dossier donné — typiquement celui que
    /// l'hôte a retrouvé dans ses réglages au lancement.
    pub fn with_root(root: Option<PathBuf>) -> Self {
        Self {
            root,
            ..Default::default()
        }
    }

    /// Affiche la fenêtre flottante. `focused` indique si l'explorateur a
    /// le focus clavier.
    ///
    /// L'explorateur se comporte comme une fenêtre surgissante : `Échap`
    /// la referme, et un clic en dehors aussi.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &Theme,
        labels: &Labels,
        focused: bool,
        current_file: Option<&Path>,
    ) -> ExplorerResponse {
        let frame = egui::Frame::new()
            .fill(theme.panel)
            .stroke(egui::Stroke::new(
                1.0,
                if focused {
                    theme.header
                } else {
                    theme.dim.gamma_multiply(0.6)
                },
            ))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(8))
            // Ombre portée propre plutôt que celle du style courant : elle
            // doit détacher l'explorateur de l'éditeur qu'il recouvre, y
            // compris sur un thème clair où l'ombre par défaut d'egui est
            // presque invisible.
            .shadow(egui::epaint::Shadow {
                offset: [0, 6],
                blur: 18,
                spread: 0,
                color: egui::Color32::from_black_alpha(96),
            });

        let mut action = ExplorerAction::None;

        // Un panneau haut, comme l'arbre de fichiers d'un éditeur, plutôt
        // qu'une boîte ajustée à son contenu : il occupe la hauteur
        // disponible sous les barres d'outils, quelle que soit la taille
        // de la fenêtre.
        const TOP: f32 = 104.0;
        let height = (ctx.viewport_rect().height() - TOP - 60.0).max(240.0);

        let window = egui::Window::new("namsdec_explorer")
            .id(egui::Id::unique("namsdec_explorer"))
            .title_bar(false)
            .frame(frame)
            .resizable(true)
            .default_pos(egui::pos2(18.0, TOP))
            .default_size(egui::vec2(300.0, height))
            .show(ctx, |ui| {
                action = self.ui(ui, theme, labels, focused, current_file);
            });

        // Le rectangle de la *fenêtre* entière, marge comprise — pas
        // `ui_contains_pointer()` du contenu, qui exclurait la marge
        // intérieure et refermerait donc l'explorateur sur un clic posé
        // sur son bord. `GRAB_MARGIN` étend encore un peu la zone, pour
        // couvrir la poignée de redimensionnement qui déborde du cadre :
        // attraper le bord pour redimensionner ne doit pas fermer.
        const GRAB_MARGIN: f32 = 6.0;
        let inside = window
            .as_ref()
            .map(|w| w.response.rect)
            .zip(ctx.input(|i| i.pointer.interact_pos()))
            .is_some_and(|(rect, pos)| rect.expand(GRAB_MARGIN).contains(pos));

        let pressed = ctx.input(|i| i.pointer.any_pressed());
        if pressed && !inside {
            // Un clic ailleurs referme, comme n'importe quelle fenêtre
            // surgissante — et prime sur ce que le contenu aurait pu
            // renvoyer, qui ne peut de toute façon pas venir d'un clic
            // hors de lui.
            action = ExplorerAction::Close;
        }

        ExplorerResponse {
            action,
            focus_gained: pressed && inside,
        }
    }

    /// Le contenu seul, sans la fenêtre — utilisable tel quel dans un
    /// panneau si l'hôte préfère, et c'est ce que pilotent les tests.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        labels: &Labels,
        focused: bool,
        current_file: Option<&Path>,
    ) -> ExplorerAction {
        let Some(root) = self.root.clone() else {
            // Un panneau simplement vide se lit comme cassé : on dit ce
            // qui manque et on propose l'action qui le corrige.
            ui.add_space(12.0);
            let mut action = ExplorerAction::None;
            ui.vertical_centered(|ui| {
                ui.label(labels.empty);
                ui.add_space(6.0);
                if ui.button(labels.open_folder).clicked() {
                    action = ExplorerAction::PickRoot;
                }
            });
            // Garde la fenêtre à sa taille au lieu de la laisser se
            // rétracter autour de deux lignes de texte.
            ui.allocate_space(ui.available_size());
            return action;
        };

        let everything = self.build_rows(&root);
        let total = everything.len();
        let rows = filter_rows(everything, &self.filter);
        if self.selected >= rows.len() {
            self.selected = rows.len().saturating_sub(1);
        }

        let filter_focused = self.header_ui(ui, theme, rows.len(), total, labels, current_file);

        // Tant qu'un champ est ouvert — filtre, chemin, nouveau nom — les
        // touches lui appartiennent : `j` doit s'écrire dans le nom du
        // fichier, pas déplacer la sélection sous le champ.
        let typing = filter_focused || self.path_edit.is_some() || self.renaming.is_some();
        let mut action = if focused && !typing {
            self.handle_keys(ui, &rows)
        } else {
            ExplorerAction::None
        };

        let mut toggle: Option<PathBuf> = None;
        let mut rename_done = false;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, row) in rows.iter().enumerate() {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ROW_HEIGHT),
                        egui::Sense::click(),
                    );

                    let renaming_here = self.renaming.as_ref().is_some_and(|r| r.path == row.path);

                    if response.clicked() && !renaming_here {
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
                        paint_row(ui, theme, rect, row, is_selected, response.hovered());
                    }

                    if renaming_here {
                        // Le champ se pose par-dessus la ligne déjà peinte,
                        // à l'emplacement exact du nom : l'icône et les
                        // connecteurs restent visibles, on édite le nom là
                        // où on le lit.
                        let left = rect.left() + LEFT_PAD + row.depth as f32 * INDENT + ICON_W;
                        let field = egui::Rect::from_min_max(
                            egui::pos2(left, rect.top() + 1.0),
                            egui::pos2(rect.right() - 4.0, rect.bottom() - 1.0),
                        );
                        let renaming = self
                            .renaming
                            .as_mut()
                            .expect("renaming_here vient d'être vérifié");
                        let response = ui.put(
                            field,
                            egui::TextEdit::singleline(&mut renaming.name)
                                .hint_text(labels.rename_hint)
                                .font(egui::TextStyle::Monospace),
                        );
                        if std::mem::take(&mut renaming.request_focus) {
                            response.request_focus();
                        }
                        // Entrée valide ; tout le reste (Échap, un clic
                        // ailleurs) abandonne — un renommage à moitié tapé
                        // ne doit jamais partir sur le disque par accident.
                        if response.lost_focus() {
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                rename_done = true;
                            } else {
                                self.renaming = None;
                            }
                        }
                    }
                }
            });
        self.scroll_to_selection = false;

        if let Some(path) = toggle {
            self.toggle(&path);
        }
        if rename_done {
            action = self.commit_rename(labels);
        }

        action
    }

    /// Change de racine. Tout ce qui décrivait l'ancienne — sélection,
    /// filtre, contenu lu — est remis à zéro : ces valeurs ne veulent plus
    /// rien dire ailleurs.
    fn set_root(&mut self, path: PathBuf) {
        self.root = Some(path);
        self.selected = 0;
        self.filter.clear();
        self.scroll_to_selection = true;
        self.error = None;
        self.path_edit = None;
        self.invalidate();
    }

    /// Remonte d'un cran : c'est ce qui permet de sortir du dossier
    /// ouvert au départ et d'atteindre n'importe où sur le disque, en
    /// remontant jusqu'à `/`.
    ///
    /// L'ancienne racine est dépliée au passage, pour qu'on retrouve d'où
    /// l'on vient au lieu de se perdre dans une liste de frères.
    fn go_to_parent(&mut self) {
        let Some(current) = self.root.clone() else {
            return;
        };
        // `/` n'a pas de parent : on y est déjà tout en haut.
        let Some(parent) = current.parent().map(Path::to_path_buf) else {
            return;
        };
        self.set_root(parent);
        self.expanded.insert(current);
    }

    /// Valide le chemin tapé à la main. Un chemin qui n'est pas un dossier
    /// laisse la racine inchangée et le dit, plutôt que d'ouvrir sur du
    /// vide.
    fn apply_path_edit(&mut self, labels: &Labels) {
        let Some(text) = self.path_edit.take() else {
            return;
        };
        let path = expand_tilde(text.trim());
        if path.as_os_str().is_empty() {
            return;
        }
        if path.is_dir() {
            self.set_root(path);
        } else {
            self.error = Some(
                labels
                    .path_not_a_directory
                    .replace("{}", &path.display().to_string()),
            );
        }
    }

    /// Ouvre le champ de renommage sur la ligne sélectionnée.
    ///
    /// La racine en est exclue : la renommer déplacerait le dossier dans
    /// lequel on est en train de naviguer, et l'explorateur se retrouverait
    /// pointé sur un chemin qui n'existe plus.
    fn start_rename(&mut self, rows: &[Row]) {
        let Some(row) = rows.get(self.selected).filter(|r| r.depth > 0) else {
            return;
        };
        self.error = None;
        self.renaming = Some(Renaming {
            path: row.path.clone(),
            name: row.name.clone(),
            request_focus: true,
        });
    }

    /// Applique le renommage en cours. Rend `Renamed` pour que l'hôte
    /// suive le fichier s'il l'a ouvert dans l'éditeur.
    fn commit_rename(&mut self, labels: &Labels) -> ExplorerAction {
        let Some(renaming) = self.renaming.take() else {
            return ExplorerAction::None;
        };
        let new_name = renaming.name.trim();
        let unchanged = renaming
            .path
            .file_name()
            .is_some_and(|n| n == std::ffi::OsStr::new(new_name));
        if new_name.is_empty() || unchanged {
            return ExplorerAction::None;
        }
        // Renommer n'est pas déplacer : un séparateur ferait sortir le
        // fichier de son dossier, ce que cette commande ne promet pas.
        if new_name.contains(std::path::MAIN_SEPARATOR) || new_name == ".." || new_name == "." {
            self.error = Some(labels.rename_failed.replace("{}", new_name));
            return ExplorerAction::None;
        }
        let Some(target) = renaming.path.parent().map(|p| p.join(new_name)) else {
            return ExplorerAction::None;
        };
        // `fs::rename` écrase sa cible sans rien dire sous Unix : sans ce
        // garde-fou, renommer `a.asm` en `b.asm` détruirait un `b.asm`
        // existant. La vérification laisse une fenêtre de course
        // théorique, qu'un `renameat2(RENAME_NOREPLACE)` fermerait au prix
        // d'une dépendance à libc — disproportionné ici, où le seul autre
        // acteur est l'utilisateur lui-même.
        if target.exists() {
            self.error = Some(labels.rename_exists.replace("{}", new_name));
            return ExplorerAction::None;
        }

        match std::fs::rename(&renaming.path, &target) {
            Ok(()) => {
                // Un dossier renommé garde son état déplié sous son
                // nouveau nom, sinon l'arborescence se replierait sous les
                // pieds de l'utilisateur.
                if self.expanded.remove(&renaming.path) {
                    self.expanded.insert(target.clone());
                }
                self.invalidate();
                ExplorerAction::Renamed {
                    from: renaming.path,
                    to: target,
                }
            }
            Err(e) => {
                self.error = Some(labels.rename_failed.replace("{}", &e.to_string()));
                ExplorerAction::None
            }
        }
    }

    fn toggle(&mut self, path: &Path) {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_path_buf());
        }
    }

    /// Raccourcis de neo-tree : `j`/`k` (ou flèches) pour se déplacer,
    /// `l`/`Entrée` pour ouvrir ou déplier, `h` pour replier ou remonter au
    /// parent, `g`/`G` pour les extrémités, `/` pour filtrer, `Échap` pour
    /// vider le filtre puis refermer l'explorateur.
    ///
    /// S'y ajoutent `r` pour renommer la ligne sélectionnée et
    /// `Retour arrière` (ou `-`) pour sortir du dossier courant vers son
    /// parent — de proche en proche jusqu'à `/`.
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
                    self.selected = if modifiers.shift {
                        rows.len().saturating_sub(1)
                    } else {
                        0
                    };
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
                egui::Key::Backspace | egui::Key::Minus => self.go_to_parent(),
                egui::Key::R => self.start_rename(rows),
                egui::Key::Slash => self.request_filter_focus = true,
                egui::Key::Escape => {
                    // Un premier Échap annule le filtre en cours (sinon on
                    // perdrait la recherche en même temps que la fenêtre) ;
                    // sans filtre, il referme.
                    if self.filter.is_empty() {
                        action = ExplorerAction::Close;
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
        // Sur la racine, il n'y a rien à replier au-dessus : replier la
        // ligne 0 ne laisserait qu'elle et aucun moyen de continuer. C'est
        // le moment de sortir du dossier, ce que la touche veut dire de
        // toute façon — « vers la gauche », donc vers le parent.
        if row.depth == 0 {
            self.go_to_parent();
            return;
        }
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
    fn header_ui(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        shown: usize,
        total: usize,
        labels: &Labels,
        current_file: Option<&Path>,
    ) -> bool {
        ui.add_space(4.0);

        let font = egui::FontId::monospace(13.0);
        let title_width = ui
            .painter()
            .layout_no_wrap(labels.title.to_owned(), font.clone(), theme.header)
            .size()
            .x;
        let side = ((ui.available_width() - title_width - 16.0) * 0.5).max(4.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            rule(ui, theme, side);
            ui.colored_label(theme.header, labels.title);
            rule(ui, theme, side);
        });

        self.path_bar_ui(ui, theme, labels, current_file);

        let mut filter_focused = false;
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, theme.header))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(6, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (chevron, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 12.0), egui::Sense::hover());
                    paint_triangle(ui.painter(), chevron.center(), false, theme.folder_text);

                    let count = format!("{shown}/{total}");
                    let count_width = ui
                        .painter()
                        .layout_no_wrap(count.clone(), font.clone(), theme.header)
                        .size()
                        .x;
                    let field_width = (ui.available_width() - count_width - 10.0).max(24.0);

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.filter)
                            .frame(egui::Frame::NONE)
                            .hint_text(labels.filter_hint)
                            .desired_width(field_width),
                    );
                    if self.request_filter_focus {
                        self.request_filter_focus = false;
                        response.request_focus();
                    }
                    filter_focused = response.has_focus();

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(theme.dim, count);
                    });
                });
            });

        // Un échec (renommage refusé, chemin inexistant) reste affiché
        // jusqu'à l'action suivante : dans une fenêtre sans barre de
        // statut, c'est le seul endroit où l'utilisateur peut apprendre
        // pourquoi il ne s'est rien passé.
        if let Some(error) = self.error.clone() {
            ui.add_space(2.0);
            ui.colored_label(theme.bad, error);
        }

        ui.add_space(6.0);
        filter_focused
    }

    /// La ligne au-dessus du filtre : les raccourcis de lieux, puis la
    /// racine courante — affichée, et éditable d'un clic pour taper un
    /// chemin quelconque.
    fn path_bar_ui(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        labels: &Labels,
        current_file: Option<&Path>,
    ) {
        // Des libellés ASCII plutôt que des flèches ou des pictogrammes :
        // la police par défaut d'egui ne garantit pas ces glyphes, et un
        // carré de repli serait pire qu'un `..` que tout le monde lit.
        // L'infobulle, elle, est traduite.
        let mut commit_path = false;
        let mut close_path_edit = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            if ui
                .small_button("..")
                .on_hover_text(labels.go_parent)
                .clicked()
            {
                self.go_to_parent();
            }
            if ui.small_button("/").on_hover_text(labels.go_root).clicked() {
                self.set_root(PathBuf::from("/"));
            }
            let home = std::env::var_os("HOME").map(PathBuf::from);
            if ui
                .add_enabled(home.is_some(), egui::Button::new("~").small())
                .on_hover_text(labels.go_home)
                .clicked()
                && let Some(home) = home
            {
                self.set_root(home);
            }
            // Le dossier du fichier ouvert dans l'éditeur : l'explorateur
            // ne connaît pas l'éditeur, l'hôte lui passe donc le chemin.
            let file_dir = current_file.and_then(Path::parent).map(Path::to_path_buf);
            if ui
                .add_enabled(file_dir.is_some(), egui::Button::new("@").small())
                .on_hover_text(labels.go_current_file)
                .clicked()
                && let Some(dir) = file_dir
            {
                self.set_root(dir);
            }

            match &mut self.path_edit {
                Some(text) => {
                    let response = ui.add(
                        egui::TextEdit::singleline(text)
                            .hint_text(labels.path_hint)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width()),
                    );
                    // Le focus se demande une fois, à l'ouverture.
                    // Redemandé à chaque frame, le champ le reprenait
                    // aussitôt qu'on l'avait donné ailleurs : cliquer
                    // dans le code ne suffisait plus à en sortir, et
                    // chaque touche continuait d'atterrir ici.
                    if self.request_path_focus {
                        self.request_path_focus = false;
                        response.request_focus();
                    }
                    if response.lost_focus() {
                        commit_path = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if !commit_path {
                            // Partir ailleurs referme le champ, comme on
                            // abandonne une étiquette qu'on éditait.
                            close_path_edit = true;
                        }
                    }
                }
                None => {
                    let shown = self
                        .root
                        .as_ref()
                        .map(|r| elide_left(&r.display().to_string(), 34))
                        .unwrap_or_default();
                    let response = ui.add(
                        egui::Label::new(egui::RichText::new(shown).monospace().color(theme.dim))
                            .sense(egui::Sense::click())
                            .truncate(),
                    );
                    if response.on_hover_text(labels.path_hint).clicked() {
                        self.path_edit = Some(
                            self.root
                                .as_ref()
                                .map(|r| r.display().to_string())
                                .unwrap_or_default(),
                        );
                        self.request_path_focus = true;
                    }
                }
            }
        });

        if commit_path {
            self.apply_path_edit(labels);
        } else if close_path_edit || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            // Échap — ou un clic ailleurs — abandonne la saisie sans
            // toucher à la racine.
            self.path_edit = None;
        }
    }

    fn build_rows(&mut self, root: &Path) -> Vec<Row> {
        let mut rows = vec![Row {
            path: root.to_path_buf(),
            // À la racine du disque, `file_name()` ne rend rien : le chemin
            // entier fait alors office de nom, sinon la première ligne
            // serait vide.
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

    fn collect(&mut self, dir: &Path, guides: &mut Vec<bool>, rows: &mut Vec<Row>) {
        self.ensure_cached(dir);
        let count = self.cache.get(dir).map_or(0, |c| c.entries.len());

        for index in 0..count {
            // Relu du cache à chaque tour plutôt qu'emprunté sur toute la
            // boucle : la récursion en dessous a besoin de `&mut self` pour
            // remplir le cache des sous-dossiers.
            let Some(entry) = self
                .cache
                .get(dir)
                .and_then(|c| c.entries.get(index).cloned())
            else {
                break;
            };
            let is_expanded = entry.is_dir && self.expanded.contains(&entry.path);

            guides.push(index + 1 < count);
            rows.push(Row {
                name: entry.name,
                depth: guides.len(),
                is_dir: entry.is_dir,
                is_expanded,
                guides: guides.clone(),
                path: entry.path.clone(),
            });
            if is_expanded {
                self.collect(&entry.path, guides, rows);
            }
            guides.pop();
        }
    }

    /// Relit `dir` si son contenu n'est pas en cache ou a dépassé son
    /// `CACHE_TTL` ; ne touche pas au disque autrement.
    fn ensure_cached(&mut self, dir: &Path) {
        let fresh = self
            .cache
            .get(dir)
            .is_some_and(|c| c.read_at.elapsed() < CACHE_TTL);
        if fresh {
            return;
        }
        self.cache.insert(
            dir.to_path_buf(),
            CachedDir {
                entries: read_dir_sorted(dir),
                read_at: Instant::now(),
            },
        );
    }

    /// Force la relecture au prochain affichage — après une opération de
    /// l'explorateur lui-même (un renommage), dont le résultat doit être
    /// visible tout de suite et pas dans deux secondes.
    fn invalidate(&mut self) {
        self.cache.clear();
    }
}

/// Raccourcit un chemin trop long par la gauche : c'est la fin — le
/// dossier où l'on est — qui porte l'information, pas la racine.
fn elide_left(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_owned();
    }
    let skip = count - max_chars + 1;
    format!("…{}", text.chars().skip(skip).collect::<String>())
}

/// `~` et `~/quelque/part` désignent le dossier personnel — la forme que
/// tout le monde tape, et que le champ de chemin doit donc comprendre.
fn expand_tilde(text: &str) -> PathBuf {
    let Some(rest) = text.strip_prefix('~') else {
        return PathBuf::from(text);
    };
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return PathBuf::from(text);
    };
    match rest.strip_prefix('/') {
        Some(relative) => home.join(relative),
        // `~` seul, ou `~autrechose` qu'on ne cherche pas à résoudre en
        // dossier d'un autre utilisateur.
        None if rest.is_empty() => home,
        None => PathBuf::from(text),
    }
}

/// Lit un dossier, trié dossiers d'abord puis par nom insensible à la
/// casse. Un dossier illisible (droits) rend une liste vide plutôt qu'une
/// erreur : l'explorateur doit pouvoir traverser `/` sans s'arrêter au
/// premier `/root` interdit.
fn read_dir_sorted(dir: &Path) -> Vec<Entry> {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read_dir
        .filter_map(|e| e.ok())
        .map(|e| {
            // `file_type()` est déjà connu du `read_dir` sur les systèmes
            // de fichiers courants : contrairement à `path().is_dir()`, il
            // n'ajoute pas un `stat` par entrée. Seuls les liens
            // symboliques en demandent un, pour qu'un lien vers un dossier
            // s'ouvre bien comme un dossier.
            let is_dir = match e.file_type() {
                Ok(t) if t.is_symlink() => e.path().is_dir(),
                Ok(t) => t.is_dir(),
                Err(_) => false,
            };
            Entry {
                path: e.path(),
                name: e.file_name().to_string_lossy().into_owned(),
                is_dir,
            }
        })
        .collect();
    // `sort_by_cached_key` : la clé (dont une minuscule allouée) est
    // calculée une fois par entrée, pas à chaque comparaison.
    entries.sort_by_cached_key(|e| (!e.is_dir, e.name.to_lowercase()));
    entries
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

fn paint_row(
    ui: &egui::Ui,
    theme: &Theme,
    rect: egui::Rect,
    row: &Row,
    selected: bool,
    hovered: bool,
) {
    let painter = ui.painter();

    if selected {
        painter.rect_filled(rect, 0, theme.selection.gamma_multiply(0.45));
        // Le petit repère vertical au bord gauche, comme neo-tree.
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(3.0, rect.height())),
            0,
            theme.accent,
        );
    } else if hovered {
        painter.rect_filled(rect, 0, theme.surface);
    }

    paint_guides(painter, rect, row, theme.dim.gamma_multiply(0.7));

    let icon_left = rect.left() + LEFT_PAD + row.depth as f32 * INDENT;
    let icon_center = egui::pos2(icon_left + ICON_W * 0.5, rect.center().y);
    let font = egui::FontId::monospace(13.0);

    if row.is_dir {
        let color = if row.is_expanded {
            theme.folder_open
        } else {
            theme.folder
        };
        paint_folder_icon(painter, icon_center, color, row.is_expanded);
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
        theme.folder_text
    } else {
        theme.text
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
    for (depth, encore) in row.guides.iter().enumerate().take(row.depth - 1) {
        if *encore {
            painter.vline(guide_x(depth), rect.y_range(), stroke);
        }
    }

    // Connecteur de la ligne elle-même : `├` s'il reste des frères après
    // elle, `└` si c'est la dernière.
    let own = row.depth - 1;
    let x = guide_x(own);
    let center_y = rect.center().y;
    let bottom = if row.guides[own] {
        rect.bottom()
    } else {
        center_y
    };
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
    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

/// Dossier fermé plein, dossier ouvert avec sa face avant inclinée — c'est
/// l'icône elle-même qui dit l'état, comme dans neo-tree, plutôt qu'un
/// chevron séparé qui alourdirait chaque ligne.
fn paint_folder_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: egui::Color32,
    open: bool,
) {
    let r = egui::Rect::from_center_size(center, egui::vec2(14.0, 11.0));
    painter.rect_filled(
        egui::Rect::from_min_size(egui::pos2(r.left(), r.top()), egui::vec2(6.0, 3.0)),
        1,
        color,
    );

    if open {
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(r.left(), r.top() + 2.0),
                egui::pos2(r.right() - 3.0, r.bottom()),
            ),
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
/// dépendre d'une police d'icônes type nerd-font. Ces couleurs ne suivent
/// volontairement pas le thème : comme les devicons de nvim, elles
/// identifient un type de fichier et restent les mêmes partout.
fn file_badge(name: &str) -> (String, egui::Color32) {
    if name.eq_ignore_ascii_case("LICENSE") {
        return ("[L]".to_owned(), egui::Color32::from_rgb(0x4e, 0xc9, 0xb0));
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .or_else(|| {
            name.eq_ignore_ascii_case("Cargo.lock")
                .then(|| "toml".to_owned())
        });

    match ext.as_deref() {
        Some("toml") => ("[T]".to_owned(), egui::Color32::from_rgb(0xdc, 0x98, 0x2b)),
        Some("rs") => ("[R]".to_owned(), egui::Color32::from_rgb(0xde, 0x76, 0x56)),
        Some("asm" | "nasm" | "s") => ("[A]".to_owned(), egui::Color32::from_rgb(0x56, 0x9c, 0xd6)),
        Some("md") => ("[M]".to_owned(), egui::Color32::from_rgb(0x9c, 0xa3, 0xaf)),
        Some(other) => (
            format!(
                "[{}]",
                other.chars().next().unwrap_or('?').to_ascii_uppercase()
            ),
            egui::Color32::from_rgb(0x8a, 0x8a, 0x8a),
        ),
        None => ("[F]".to_owned(), egui::Color32::from_rgb(0x8a, 0x8a, 0x8a)),
    }
}

fn rule(ui: &mut egui::Ui, theme: &Theme, len: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(len.max(0.0), 8.0), egui::Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.center().y,
        egui::Stroke::new(1.0, theme.header),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABELS: Labels<'static> = Labels {
        title: "Explorer",
        filter_hint: "",
        empty: "",
        open_folder: "",
        path_hint: "",
        go_parent: "",
        go_root: "",
        go_home: "",
        go_current_file: "",
        rename_hint: "",
        rename_failed: "échec : {}",
        rename_exists: "{} existe déjà",
        path_not_a_directory: "{} n'est pas un dossier",
    };

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
    fn frame(ctx: &egui::Context, keys: &[egui::Key], mut logic: impl FnMut(&mut egui::Ui)) {
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

    fn names(explorer: &mut FileExplorer, root: &Path) -> Vec<String> {
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
        let theme = &namsdec_theme::TOKYO_NIGHT;
        let mut explorer = FileExplorer {
            root: Some(root.clone()),
            ..Default::default()
        };

        // Dossier fermé au départ : sous la racine, `src` puis `Cargo.toml`
        // (dossiers d'abord), et pas encore `main.rs`.
        assert_eq!(
            names(&mut explorer, &root)[1..],
            ["src", "Cargo.toml"].map(|s| s.to_owned()),
            "les enfants de la racine, dossiers d'abord"
        );

        // `j` descend de la racine vers `src`.
        frame(&ctx, &[egui::Key::J], |ui| {
            explorer.ui(ui, theme, &LABELS, true, None);
        });
        assert_eq!(explorer.selected, 1);

        // `l` déplie `src`.
        frame(&ctx, &[egui::Key::L], |ui| {
            explorer.ui(ui, theme, &LABELS, true, None);
        });
        assert!(
            explorer.expanded.contains(&root.join("src")),
            "`l` sur un dossier doit le déplier"
        );
        assert_eq!(
            names(&mut explorer, &root)[1..],
            ["src", "main.rs", "Cargo.toml"].map(|s| s.to_owned()),
            "main.rs apparaît sous src une fois déplié"
        );

        // `j` puis `l` sur `main.rs` doit demander son ouverture.
        frame(&ctx, &[egui::Key::J], |ui| {
            explorer.ui(ui, theme, &LABELS, true, None);
        });
        let mut opened = None;
        frame(&ctx, &[egui::Key::L], |ui| {
            if let ExplorerAction::OpenFile(path) = explorer.ui(ui, theme, &LABELS, true, None) {
                opened = Some(path);
            }
        });
        assert_eq!(opened.as_deref(), Some(root.join("src/main.rs").as_path()));
    }

    /// `Échap` referme l'explorateur — mais pas avant d'avoir annulé un
    /// filtre en cours, sinon la fenêtre disparaîtrait au moment où
    /// l'utilisateur voulait seulement effacer sa recherche.
    #[test]
    fn escape_clears_the_filter_first_then_closes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("main.asm"), "").unwrap();

        let ctx = egui::Context::default();
        let theme = &namsdec_theme::TOKYO_NIGHT;
        let mut explorer = FileExplorer {
            root: Some(root),
            filter: "asm".to_owned(),
            ..Default::default()
        };

        let mut closed = false;
        frame(&ctx, &[egui::Key::Escape], |ui| {
            if let ExplorerAction::Close = explorer.ui(ui, theme, &LABELS, true, None) {
                closed = true;
            }
        });
        assert!(explorer.filter.is_empty(), "le filtre doit être annulé");
        assert!(!closed, "ce premier Échap ne devait pas refermer");

        let ctx = egui::Context::default();
        frame(&ctx, &[egui::Key::Escape], |ui| {
            if let ExplorerAction::Close = explorer.ui(ui, theme, &LABELS, true, None) {
                closed = true;
            }
        });
        assert!(closed, "sans filtre, Échap referme");
    }

    /// Fait tourner une frame complète de `show()` — fenêtre comprise —
    /// avec les évènements bruts donnés, comme les recevrait eframe.
    fn window_frame(
        ctx: &egui::Context,
        explorer: &mut FileExplorer,
        events: Vec<egui::Event>,
    ) -> ExplorerResponse {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            events,
            ..Default::default()
        };
        let mut response = None;
        let mut output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            response = Some(explorer.show(&ctx, &namsdec_theme::TOKYO_NIGHT, &LABELS, true, None));
        });
        output.textures_delta.clear();
        response.expect("show() doit avoir tourné")
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn click_at(pos: egui::Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    /// Le chemin réel de `Échap`, à travers la fenêtre flottante et non
    /// juste le contenu : c'est celui que l'application emprunte.
    #[test]
    fn escape_closes_the_floating_window() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.asm"), "").unwrap();
        let ctx = egui::Context::default();
        let mut explorer = FileExplorer::with_root(Some(dir.path().to_path_buf()));

        // Une première frame pour que la fenêtre existe et se place.
        window_frame(&ctx, &mut explorer, Vec::new());

        let response = window_frame(&ctx, &mut explorer, vec![key(egui::Key::Escape)]);
        assert!(
            matches!(response.action, ExplorerAction::Close),
            "Échap doit refermer l'explorateur"
        );
    }

    /// Un clic en dehors referme ; un clic dedans donne le focus et ne
    /// referme surtout pas.
    #[test]
    fn a_click_outside_closes_but_a_click_inside_focuses() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.asm"), "").unwrap();
        let ctx = egui::Context::default();
        let mut explorer = FileExplorer::with_root(Some(dir.path().to_path_buf()));

        window_frame(&ctx, &mut explorer, Vec::new());

        // Loin à droite : dans l'éditeur, hors de la fenêtre (posée en
        // haut à gauche).
        let outside = window_frame(&ctx, &mut explorer, click_at(egui::pos2(1000.0, 600.0)));
        assert!(
            matches!(outside.action, ExplorerAction::Close),
            "un clic en dehors doit refermer"
        );
        assert!(!outside.focus_gained);

        // Dans l'en-tête de la fenêtre, au-dessus des lignes de fichiers.
        let inside = window_frame(&ctx, &mut explorer, click_at(egui::pos2(60.0, 120.0)));
        assert!(
            !matches!(inside.action, ExplorerAction::Close),
            "un clic à l'intérieur ne doit pas refermer"
        );
        assert!(
            inside.focus_gained,
            "un clic à l'intérieur doit donner le focus clavier"
        );
    }

    /// Sortir du dossier ouvert au départ, c'est ce qui donne accès au
    /// reste du disque : `Retour arrière` recale la racine sur le parent,
    /// en dépliant celui d'où l'on vient pour ne pas perdre le fil.
    #[test]
    fn backspace_climbs_to_the_parent_folder() {
        let dir = tempfile::tempdir().unwrap();
        let start = dir.path().join("projet");
        std::fs::create_dir(&start).unwrap();
        std::fs::write(start.join("main.asm"), "").unwrap();

        let ctx = egui::Context::default();
        let mut explorer = FileExplorer::with_root(Some(start.clone()));

        frame(&ctx, &[egui::Key::Backspace], |ui| {
            explorer.ui(ui, &namsdec_theme::TOKYO_NIGHT, &LABELS, true, None);
        });

        assert_eq!(explorer.root.as_deref(), Some(dir.path()));
        assert!(
            explorer.expanded.contains(&start),
            "le dossier qu'on vient de quitter doit rester déplié"
        );
    }

    /// On doit pouvoir remonter jusqu'à `/` — et s'y arrêter, sans tourner
    /// en rond ni perdre la racine.
    #[test]
    fn climbing_stops_at_the_filesystem_root() {
        let dir = tempfile::tempdir().unwrap();
        let mut explorer = FileExplorer::with_root(Some(dir.path().to_path_buf()));

        // Largement plus de crans qu'il n'en faut depuis un dossier
        // temporaire : les derniers ne doivent rien casser.
        for _ in 0..40 {
            explorer.go_to_parent();
        }

        assert_eq!(explorer.root.as_deref(), Some(Path::new("/")));
    }

    /// `r` ouvre le champ, et valider renomme vraiment sur le disque —
    /// l'hôte en étant averti, puisque c'est peut-être le fichier ouvert
    /// dans l'éditeur.
    #[test]
    fn r_opens_the_rename_field_and_confirming_renames_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("ancien.asm"), "hlt").unwrap();

        let ctx = egui::Context::default();
        let mut explorer = FileExplorer::with_root(Some(root.clone()));

        // `j` descend de la racine sur le fichier, `r` ouvre le champ.
        frame(&ctx, &[egui::Key::J], |ui| {
            explorer.ui(ui, &namsdec_theme::TOKYO_NIGHT, &LABELS, true, None);
        });
        frame(&ctx, &[egui::Key::R], |ui| {
            explorer.ui(ui, &namsdec_theme::TOKYO_NIGHT, &LABELS, true, None);
        });

        let renaming = explorer
            .renaming
            .as_mut()
            .expect("`r` doit ouvrir le champ de renommage");
        assert_eq!(renaming.name, "ancien.asm", "le champ part du nom actuel");
        renaming.name = "nouveau.asm".to_owned();

        let action = explorer.commit_rename(&LABELS);

        assert!(
            root.join("nouveau.asm").exists(),
            "le fichier doit avoir été renommé"
        );
        assert!(!root.join("ancien.asm").exists());
        match action {
            ExplorerAction::Renamed { from, to } => {
                assert_eq!(from, root.join("ancien.asm"));
                assert_eq!(to, root.join("nouveau.asm"));
            }
            _ => panic!("l'hôte doit être averti du renommage"),
        }
    }

    /// `fs::rename` écrase sa cible sans rien dire : renommer par-dessus
    /// un fichier existant doit être refusé, pas détruire son contenu.
    #[test]
    fn renaming_never_overwrites_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("a.asm"), "aaa").unwrap();
        std::fs::write(root.join("b.asm"), "bbb").unwrap();

        let mut explorer = FileExplorer::with_root(Some(root.clone()));
        explorer.renaming = Some(Renaming {
            path: root.join("a.asm"),
            name: "b.asm".to_owned(),
            request_focus: false,
        });

        let action = explorer.commit_rename(&LABELS);

        assert!(matches!(action, ExplorerAction::None));
        assert_eq!(
            std::fs::read_to_string(root.join("b.asm")).unwrap(),
            "bbb",
            "le fichier visé ne doit surtout pas avoir été écrasé"
        );
        assert!(root.join("a.asm").exists(), "et l'original reste en place");
        assert!(explorer.error.is_some(), "le refus doit être expliqué");
    }

    /// Renommer n'est pas déplacer : un séparateur dans le nom sortirait
    /// le fichier de son dossier.
    #[test]
    fn a_name_with_a_separator_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("a.asm"), "").unwrap();

        let mut explorer = FileExplorer::with_root(Some(root.clone()));
        explorer.renaming = Some(Renaming {
            path: root.join("a.asm"),
            name: "../ailleurs.asm".to_owned(),
            request_focus: false,
        });

        assert!(matches!(
            explorer.commit_rename(&LABELS),
            ExplorerAction::None
        ));
        assert!(root.join("a.asm").exists());
        assert!(!dir.path().join("ailleurs.asm").exists());
    }

    /// Le champ de chemin rend le clavier quand on va ailleurs.
    ///
    /// Il redemandait le focus à *chaque* frame tant qu'il était ouvert :
    /// une fois cliqué, plus rien n'atteignait l'éditeur — chaque touche
    /// retombait dans le champ, frame après frame, et seuls Entrée ou
    /// Échap en sortaient.
    #[test]
    fn the_path_field_releases_the_keyboard_when_focus_goes_elsewhere() {
        let dir = tempfile::tempdir().unwrap();
        let mut explorer = FileExplorer::with_root(Some(dir.path().to_path_buf()));
        let ctx = egui::Context::default();

        // Ouvert : il prend le clavier, une fois. Deux frames, le temps
        // qu'egui enregistre qui l'avait au début de la précédente —
        // c'est de là qu'il tire la perte de focus.
        explorer.path_edit = Some(dir.path().display().to_string());
        explorer.request_path_focus = true;
        window_frame(&ctx, &mut explorer, vec![]);
        let field = ctx.memory(|m| m.focused()).expect("le champ a le clavier");
        window_frame(&ctx, &mut explorer, vec![]);
        assert_eq!(ctx.memory(|m| m.focused()), Some(field), "il le garde");

        // L'utilisateur clique ailleurs (l'éditeur, une autre fenêtre) :
        // egui donne le focus à autre chose.
        ctx.memory_mut(|m| m.surrender_focus(field));
        window_frame(&ctx, &mut explorer, vec![]);

        assert!(
            explorer.path_edit.is_none(),
            "partir ailleurs doit refermer le champ"
        );
        assert!(
            ctx.memory(|m| m.focused()).is_none(),
            "et lui faire lâcher le clavier"
        );
    }

    /// Un chemin tapé à la main qui ne mène pas à un dossier laisse la
    /// racine où elle est, et le dit.
    #[test]
    fn a_path_that_is_not_a_directory_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let mut explorer = FileExplorer::with_root(Some(root.clone()));

        explorer.path_edit = Some("/ce/chemin/n/existe/pas".to_owned());
        explorer.apply_path_edit(&LABELS);

        assert_eq!(explorer.root.as_deref(), Some(root.as_path()));
        assert!(explorer.error.is_some());

        // Et un vrai dossier, lui, est accepté.
        explorer.path_edit = Some(dir.path().display().to_string());
        explorer.apply_path_edit(&LABELS);
        assert_eq!(explorer.root.as_deref(), Some(dir.path()));
    }

    /// L'arborescence était relue à chaque frame : soixante `read_dir` par
    /// seconde, intenable sur un gros dossier — et c'est justement là que
    /// la navigation libre mène. Le contenu lu doit être réutilisé, et le
    /// rester jusqu'à ce qu'on l'invalide.
    #[test]
    fn the_tree_is_not_re_read_on_every_frame() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("a.asm"), "").unwrap();

        let mut explorer = FileExplorer::with_root(Some(root.clone()));
        assert_eq!(names(&mut explorer, &root)[1..], ["a.asm".to_owned()]);

        // Un fichier apparu entre deux frames : dans les deux secondes de
        // `CACHE_TTL`, il n'a pas à provoquer une relecture (ce test
        // s'exécute bien en deçà).
        std::fs::write(root.join("b.asm"), "").unwrap();
        assert_eq!(
            names(&mut explorer, &root)[1..],
            ["a.asm".to_owned()],
            "le contenu en cache doit être réutilisé tel quel"
        );

        // Les opérations de l'explorateur, elles, le rendent visible tout
        // de suite.
        explorer.invalidate();
        assert_eq!(
            names(&mut explorer, &root)[1..],
            ["a.asm".to_owned(), "b.asm".to_owned()]
        );
    }

    #[test]
    fn a_tilde_path_points_at_the_home_folder() {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(home) = home {
            assert_eq!(expand_tilde("~"), home);
            assert_eq!(expand_tilde("~/Projects"), home.join("Projects"));
        }
        // Un chemin absolu ordinaire traverse sans être touché.
        assert_eq!(expand_tilde("/usr/lib"), PathBuf::from("/usr/lib"));
    }

    /// Sans le focus, l'explorateur doit ignorer les touches — sinon il
    /// volerait `j`/`k`/`l` à l'éditeur.
    #[test]
    fn keys_are_ignored_when_not_focused() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir(root.join("src")).unwrap();

        let ctx = egui::Context::default();
        let theme = &namsdec_theme::TOKYO_NIGHT;
        let mut explorer = FileExplorer {
            root: Some(root),
            ..Default::default()
        };

        frame(&ctx, &[egui::Key::J], |ui| {
            explorer.ui(ui, theme, &LABELS, false, None);
        });

        assert_eq!(explorer.selected, 0);
    }
}
