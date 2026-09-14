//! Explorateur de fichiers façon VS Code : un dossier racine choisi par
//! l'utilisateur, une arborescence relue à chaque affichage (pas de cache
//! — les projets NASM restent petits, la simplicité l'emporte), et un
//! fichier cliqué qui se charge dans l'éditeur. N'importe quel fichier
//! peut être ouvert, pas seulement les `.asm` : c'est le comportement réel
//! de VS Code, qui ne masque pas les fichiers non reconnus.

use std::path::{Path, PathBuf};

use eframe::egui;

#[derive(Default)]
pub struct FileExplorer {
    pub root: Option<PathBuf>,
}

impl FileExplorer {
    /// Affiche l'arborescence à partir de `root`. Renvoie le fichier
    /// cliqué, s'il y en a un — à charger dans l'éditeur par l'appelant.
    pub fn ui(&self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let root = self.root.clone()?;
        let mut clicked = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            dir_contents_ui(ui, &root, &mut clicked);
        });
        clicked
    }
}

fn dir_contents_ui(ui: &mut egui::Ui, dir: &Path, clicked: &mut Option<PathBuf>) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    // Dossiers d'abord, puis ordre alphabétique dans chaque groupe.
    entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            egui::CollapsingHeader::new(name)
                .id_salt(&path)
                .default_open(false)
                .show(ui, |ui| dir_contents_ui(ui, &path, clicked));
        } else if ui.selectable_label(false, name).clicked() {
            *clicked = Some(path);
        }
    }
}
