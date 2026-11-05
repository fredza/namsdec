//! Les barres d'icônes des bords : montrer et cacher les panneaux.
//!
//! **Demandé à l'écran, capture annotée** (4 novembre 2026) : trois
//! rectangles jaunes autour des barres d'icônes de CLion — celle de
//! gauche, celle de droite, celle du bas — et deux flèches, « Afficher
//! l'explorateur » sur l'icône du coin et « cacher l'explorateur » sur
//! le bouton du panneau.
//!
//! **CE QU'ELLES APPORTENT N'EST PAS UNE COMMANDE DE PLUS.** Tout ce
//! qu'elles font était déjà au menu Affichage et dans les raccourcis.
//! Ce qu'elles ajoutent est de RENDRE VISIBLE CE QUI EXISTE : un
//! panneau fermé ne laissait aucune trace à l'écran, et l'on ne ferme
//! pas ce qu'on ne sait pas rouvrir. L'icône reste là, allumée ou
//! éteinte, et dit à la fois que la chose existe et où elle est rangée.
//!
//! Chaque icône est du côté où son panneau s'ouvre : l'explorateur et
//! les exercices à gauche, la console en bas à gauche — comme dans
//! JetBrains, où le groupe du bas de la barre gauche tient les panneaux
//! du bas —, le panneau du processeur à droite. Une icône qui ouvre un
//! panneau à l'autre bout de l'écran apprend à ne plus la croire.

use super::*;

/// La largeur d'une barre : une icône et ses marges.
pub(crate) const BARRE_LARGEUR: f32 = 34.0;
/// Le côté du carré cliquable.
const CASE: f32 = 26.0;

/// Ce qu'une icône de barre commande.
struct Outil {
    /// Ce qu'elle dessine.
    dessin: fn(&egui::Painter, egui::Rect, egui::Color32),
    /// Le panneau est-il ouvert ?
    ouvert: bool,
    /// Ce que dit l'infobulle, selon l'état.
    montrer: &'static str,
    cacher: &'static str,
    /// Le raccourci, s'il y en a un : LU dans le tableau, jamais
    /// recopié.
    action: Option<Action>,
}

impl NamsdecApp {
    /// La barre de gauche : l'explorateur, les exercices, la console.
    pub(crate) fn barre_gauche_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let mut haut = vec![Outil {
            dessin: crate::icons::icon_folder,
            ouvert: self.show_explorer,
            montrer: s.rail_explorer_show,
            cacher: s.rail_explorer_hide,
            action: Some(Action::ToggleExplorer),
        }];
        // Les exercices ne paraissent que si le module est allumé : une
        // icône éteinte pour une chose absente ne se distingue pas d'une
        // chose fermée.
        if self.exercises {
            haut.push(Outil {
                dessin: crate::icons::icon_book,
                ouvert: self.show_exercises,
                montrer: s.rail_exercises_show,
                cacher: s.rail_exercises_hide,
                action: None,
            });
        }
        let bas = [Outil {
            dessin: crate::icons::icon_terminal,
            ouvert: self.show_console,
            montrer: s.rail_console_show,
            cacher: s.rail_console_hide,
            action: Some(Action::ToggleConsole),
        }];

        let mut bascules = Vec::new();
        for (rang, outil) in haut.iter().enumerate() {
            if self.case_d_outil(ui, outil) {
                bascules.push(rang);
            }
        }
        // LE GROUPE DU BAS, EN BAS : c'est ce qui dit, sans un mot, que
        // ce panneau-là s'ouvre en bas de l'écran. JetBrains en a fait
        // une convention ; une icône posée n'importe où redevient une
        // liste de commandes.
        let reste = ui.available_height() - (CASE + 6.0) * bas.len() as f32;
        if reste > 0.0 {
            ui.add_space(reste);
        }
        for (rang, outil) in bas.iter().enumerate() {
            if self.case_d_outil(ui, outil) {
                bascules.push(haut.len() + rang);
            }
        }

        for rang in bascules {
            match rang {
                0 => self.show_explorer = !self.show_explorer,
                _ if rang == haut.len() => self.show_console = !self.show_console,
                _ => self.show_exercises = !self.show_exercises,
            }
        }
    }

    /// La barre de droite : le panneau du processeur.
    pub(crate) fn barre_droite_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let outil = Outil {
            dessin: crate::icons::icon_chip,
            ouvert: self.show_sidebar,
            montrer: s.rail_sidebar_show,
            cacher: s.rail_sidebar_hide,
            action: Some(Action::ToggleSidebar),
        };
        if self.case_d_outil(ui, &outil) {
            self.show_sidebar = !self.show_sidebar;
        }
    }

    /// Une case de barre : l'icône, son fond si le panneau est ouvert,
    /// et ce qu'elle dit au survol.
    fn case_d_outil(&self, ui: &mut egui::Ui, outil: &Outil) -> bool {
        let theme = self.theme;
        let (rect, réponse) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), CASE + 6.0),
            egui::Sense::click(),
        );
        let case = egui::Rect::from_center_size(rect.center(), egui::vec2(CASE, CASE));
        // OUVERT SE VOIT AU FOND, et non à la seule couleur du trait :
        // un dessin gris clair contre un dessin gris foncé, à cette
        // taille, ne dit rien de ce qui est ouvert. C'est la règle des
        // onglets — celui qu'on regarde porte une marque, pas une
        // nuance.
        if outil.ouvert {
            ui.painter()
                .rect_filled(case, 4.0, mélange(theme.panel, theme.text, 0.12));
        } else if réponse.hovered() {
            ui.painter()
                .rect_filled(case, 4.0, mélange(theme.panel, theme.text, 0.06));
        }
        if réponse.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let dessin =
            egui::Rect::from_center_size(case.center(), egui::vec2(CASE * 0.62, CASE * 0.62));
        (outil.dessin)(
            ui.painter(),
            dessin,
            if outil.ouvert { theme.text } else { theme.dim },
        );
        // L'infobulle dit le GESTE, et non le nom : « Afficher
        // l'explorateur » quand il est fermé, « Cacher l'explorateur »
        // quand il est ouvert. Ce sont les mots de la consigne, et ce
        // sont ceux qu'on cherche — on ne survole pas une icône pour
        // apprendre son nom, mais pour savoir ce qu'un clic fera.
        réponse
            .on_hover_text(self.phrase_d_outil(ui.ctx(), outil))
            .clicked()
    }

    /// Ce que l'infobulle d'une case dit, selon l'état du panneau.
    ///
    /// À part, pour être mesurable : une infobulle d'egui ne se peint
    /// pas dans un banc sans écran — aucun de nos contrôles n'en a
    /// jamais relevé une. Ce qui peut se tromper ici n'est pas le
    /// dessin de la bulle mais le CHOIX de la phrase — annoncer
    /// « Afficher » sur un panneau déjà ouvert —, et c'est ce choix
    /// qu'on mesure.
    fn phrase_d_outil(&self, ctx: &egui::Context, outil: &Outil) -> String {
        let phrase = if outil.ouvert {
            outil.cacher
        } else {
            outil.montrer
        };
        // Le raccourci est LU dans le tableau, jamais recopié : réglé
        // autrement, l'infobulle le dit autrement.
        match outil
            .action
            .and_then(|action| self.keybindings.map.get(&action))
        {
            Some(chord) => format!("{phrase} ({})", chord.display(ctx, self.lang == Lang::Fr)),
            None => phrase.to_owned(),
        }
    }
}

#[cfg(test)]
impl NamsdecApp {
    /// Ce que dirait l'icône de l'explorateur, dans l'état où l'on est.
    pub(crate) fn phrase_de_l_explorateur(&self, ctx: &egui::Context) -> String {
        let s = strings(self.lang);
        self.phrase_d_outil(
            ctx,
            &Outil {
                dessin: crate::icons::icon_folder,
                ouvert: self.show_explorer,
                montrer: s.rail_explorer_show,
                cacher: s.rail_explorer_hide,
                action: Some(Action::ToggleExplorer),
            },
        )
    }
}
