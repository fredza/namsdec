//! Thèmes de l'interface, sous forme de données plutôt que de constantes
//! éparpillées dans le code de rendu.
//!
//! Un thème est une palette nommée ; tout ce qui dessine (éditeur,
//! explorateur, barres) lit ses couleurs ici. C'est ce qui permet d'en
//! ajouter d'autres sans toucher au rendu — un thème installé plus tard
//! n'est qu'un `Theme` de plus dans la liste.

use egui::Color32;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Palette complète d'un thème. Volontairement plate et explicite : un
/// champ par rôle, pas de dérivation automatique, pour qu'un thème
/// importé d'ailleurs (tokyonight, gruvbox…) se transcrive directement.
pub struct Theme {
    /// Identifiant stable, utilisé pour retrouver un thème (et le
    /// persister le jour où les préférences le seront).
    pub id: &'static str,
    pub name: &'static str,
    pub dark: bool,

    /// Fond de l'éditeur et des zones de saisie.
    pub bg: Color32,
    /// Fond des panneaux (barre latérale, explorateur).
    pub panel: Color32,
    /// Fond des barres (outils, statut) — légèrement détaché du panneau.
    pub surface: Color32,
    pub text: Color32,
    /// Texte secondaire, guides d'arborescence, numéros de ligne.
    pub dim: Color32,

    pub accent: Color32,
    pub good: Color32,
    pub warn: Color32,
    pub bad: Color32,

    // Coloration syntaxique NASM.
    pub mnemonic: Color32,
    pub number: Color32,
    pub label: Color32,
    pub comment: Color32,

    // Explorateur de fichiers.
    pub folder: Color32,
    pub folder_open: Color32,
    pub folder_text: Color32,
    /// Filets et titre de l'en-tête de l'explorateur.
    pub header: Color32,
    pub selection: Color32,
}

impl Theme {
    /// Traduit la palette en `Visuals` egui. Tout ce qui n'est pas peint à
    /// la main (boutons, menus, fenêtres) en découle.
    pub fn visuals(&self) -> egui::Visuals {
        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.panel_fill = self.panel;
        visuals.window_fill = self.surface;
        visuals.extreme_bg_color = self.bg;
        visuals.faint_bg_color = self.surface;
        visuals.selection.bg_fill = self.selection;
        visuals.override_text_color = Some(self.text);
        visuals.hyperlink_color = self.accent;

        visuals.window_corner_radius = egui::CornerRadius::from(10);
        visuals.menu_corner_radius = egui::CornerRadius::from(8);
        let radius = egui::CornerRadius::from(6);
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = radius;
        }

        // Boutons plats au repos : seul le survol colore le fond, sinon la
        // barre de menus et la barre d'outils se ressemblent trop.
        visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

        visuals
    }
}

/// VS Code Dark+ — le thème d'origine de l'application.
pub static VSCODE_DARK: Theme = Theme {
    id: "vscode-dark",
    name: "VS Code Dark+",
    dark: true,
    bg: Color32::from_rgb(0x1e, 0x1e, 0x1e),
    panel: Color32::from_rgb(0x25, 0x25, 0x26),
    surface: Color32::from_rgb(0x2d, 0x2d, 0x2d),
    text: Color32::from_rgb(0xd4, 0xd4, 0xd4),
    dim: Color32::from_rgb(0x85, 0x85, 0x85),
    accent: Color32::from_rgb(0x37, 0x94, 0xff),
    good: Color32::from_rgb(0x89, 0xd1, 0x85),
    warn: Color32::from_rgb(0xdc, 0xdc, 0xaa),
    bad: Color32::from_rgb(0xf4, 0x87, 0x71),
    mnemonic: Color32::from_rgb(0x56, 0x9c, 0xd6),
    number: Color32::from_rgb(0xb5, 0xce, 0xa8),
    label: Color32::from_rgb(0x4e, 0xc9, 0xb0),
    comment: Color32::from_rgb(0x6a, 0x99, 0x55),
    folder: Color32::from_rgb(0x7d, 0xd3, 0xfc),
    folder_open: Color32::from_rgb(0x38, 0xbd, 0xf8),
    folder_text: Color32::from_rgb(0x9c, 0xdc, 0xfe),
    header: Color32::from_rgb(0xe0, 0x9a, 0x5a),
    selection: Color32::from_rgb(0x37, 0x94, 0xff),
};

/// VS Code Light+.
pub static VSCODE_LIGHT: Theme = Theme {
    id: "vscode-light",
    name: "VS Code Light+",
    dark: false,
    bg: Color32::WHITE,
    panel: Color32::from_rgb(0xf3, 0xf3, 0xf3),
    surface: Color32::from_rgb(0xe8, 0xe8, 0xe8),
    text: Color32::from_rgb(0x1e, 0x1e, 0x1e),
    dim: Color32::from_rgb(0x6e, 0x6e, 0x6e),
    accent: Color32::from_rgb(0x00, 0x5f, 0xb8),
    good: Color32::from_rgb(0x38, 0x8a, 0x34),
    warn: Color32::from_rgb(0xb8, 0x85, 0x00),
    bad: Color32::from_rgb(0xcd, 0x31, 0x31),
    mnemonic: Color32::from_rgb(0x00, 0x00, 0xff),
    number: Color32::from_rgb(0x09, 0x86, 0x58),
    label: Color32::from_rgb(0x26, 0x7f, 0x99),
    comment: Color32::from_rgb(0x00, 0x80, 0x00),
    folder: Color32::from_rgb(0x1a, 0x85, 0xb8),
    folder_open: Color32::from_rgb(0x00, 0x6b, 0x9c),
    folder_text: Color32::from_rgb(0x0a, 0x53, 0x94),
    header: Color32::from_rgb(0xb5, 0x6a, 0x1f),
    selection: Color32::from_rgb(0x00, 0x5f, 0xb8),
};

/// Tokyo Night — le thème nvim de référence.
pub static TOKYO_NIGHT: Theme = Theme {
    id: "tokyo-night",
    name: "Tokyo Night",
    dark: true,
    bg: Color32::from_rgb(0x1a, 0x1b, 0x26),
    panel: Color32::from_rgb(0x1f, 0x23, 0x35),
    surface: Color32::from_rgb(0x24, 0x28, 0x3b),
    text: Color32::from_rgb(0xc0, 0xca, 0xf5),
    dim: Color32::from_rgb(0x56, 0x5f, 0x89),
    accent: Color32::from_rgb(0x7a, 0xa2, 0xf7),
    good: Color32::from_rgb(0x9e, 0xce, 0x6a),
    warn: Color32::from_rgb(0xe0, 0xaf, 0x68),
    bad: Color32::from_rgb(0xf7, 0x76, 0x8e),
    mnemonic: Color32::from_rgb(0xbb, 0x9a, 0xf7),
    number: Color32::from_rgb(0xff, 0x9e, 0x64),
    label: Color32::from_rgb(0x7d, 0xcf, 0xff),
    comment: Color32::from_rgb(0x56, 0x5f, 0x89),
    folder: Color32::from_rgb(0x7d, 0xcf, 0xff),
    folder_open: Color32::from_rgb(0x2a, 0xc3, 0xde),
    folder_text: Color32::from_rgb(0x7d, 0xcf, 0xff),
    header: Color32::from_rgb(0xff, 0x9e, 0x64),
    selection: Color32::from_rgb(0x3d, 0x59, 0xa1),
};

/// Les thèmes fournis avec l'application. Un thème installé ensuite
/// viendra simplement s'ajouter à cette liste.
pub static BUILT_IN: &[&Theme] = &[&VSCODE_DARK, &VSCODE_LIGHT, &TOKYO_NIGHT];

/// Retrouve un thème par identifiant, en retombant sur le thème par défaut
/// si l'identifiant est inconnu — un thème désinstallé ne doit pas empêcher
/// l'application de démarrer.
pub fn by_id(id: &str) -> &'static Theme {
    BUILT_IN
        .iter()
        .copied()
        .find(|theme| theme.id == id)
        .unwrap_or(&VSCODE_DARK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_resolvable() {
        for theme in BUILT_IN {
            assert!(
                std::ptr::eq(by_id(theme.id), *theme),
                "l'identifiant « {} » doit retrouver exactement son thème",
                theme.id
            );
        }
    }

    #[test]
    fn unknown_id_falls_back_instead_of_panicking() {
        assert_eq!(by_id("thème-désinstallé").id, VSCODE_DARK.id);
    }
}
