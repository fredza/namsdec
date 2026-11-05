//! L'écran d'accueil : ce qu'on voit quand plus rien n'est ouvert.
//!
//! **Demandé à l'écran, capture à l'appui** (4 novembre 2026) : « quand
//! je quitte, j'aimerais que tout soit fermé, et une fenêtre comme la
//! suivante apparaisse — celle comme Zed ». La capture jointe montrait
//! l'accueil de Zed : le nom de l'outil, une liste « pour commencer »
//! avec les raccourcis en regard, et les projets récents.
//!
//! Fermer le dernier fichier vidait déjà le tampon — « fermer, ici,
//! c'est revenir à un tampon vierge » —, et l'on se retrouvait devant
//! un éditeur vide, sans rien qui dise quoi faire. C'est le pire état
//! d'une application : elle marche, elle n'a rien fait de mal, et elle
//! n'offre rien.
//!
//! **Ce qui est montré est ce qui MARCHE.** Chaque ligne agit, et
//! chaque raccourci affiché est celui du tableau des raccourcis — lu, et
//! non recopié : un raccourci réglé autrement se lit ici tel qu'on l'a
//! réglé. Les récents, eux, n'ont pas de raccourci et n'en affichent
//! pas : Zed leur en donne, mais il faudrait inventer cinq touches
//! globales qui ouvriraient un fichier au milieu d'une frappe ailleurs.

use super::*;

impl NamsdecApp {
    /// Plus rien n'est ouvert : ni fichier, ni texte, ni programme.
    ///
    /// Les trois ensemble, et pas seulement le texte : un fichier
    /// ouvert et vidé reste un fichier qu'on est en train d'écrire, et
    /// lui retirer son éditeur serait lui retirer son travail.
    pub(crate) fn rien_n_est_ouvert(&self) -> bool {
        // Le drapeau dit qu'on a FERMÉ ; les trois autres conditions
        // disent qu'il n'y a toujours rien. L'une sans les autres
        // mentirait : un drapeau qu'on oublierait de baisser cacherait
        // un fichier rouvert, et la seule vue du tampon vide ferait
        // disparaître l'éditeur sous les doigts de qui efface tout pour
        // recommencer.
        self.montrer_accueil
            && self.current_file.is_none()
            && self.source.trim().is_empty()
            && self.assemble_state.is_none()
    }

    /// L'accueil, à la place de l'éditeur.
    pub(crate) fn accueil_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let fr = self.lang == Lang::Fr;
        let ctx = ui.ctx().clone();
        // Ce qu'un clic demande, exécuté APRÈS la mise en page : agir
        // au milieu du dessin changerait sous nos pieds ce qu'on est en
        // train de peindre.
        let mut action = None;
        let mut ouvrir = None;

        ui.vertical_centered(|ui| {
            ui.add_space(64.0);
            ui.label(
                egui::RichText::new(APP_NAME)
                    .size(28.0)
                    .color(theme.text)
                    .strong(),
            );
            ui.label(egui::RichText::new(s.accueil_sous_titre).color(theme.dim));
            ui.add_space(28.0);
        });

        // UNE COLONNE CENTRÉE, et bornée : une liste de quatre lignes
        // étalée sur seize cents pixels ne se lit pas — l'œil va du
        // libellé au raccourci en traversant tout l'écran. Zed borne la
        // sienne de la même façon.
        let largeur = 420.0_f32.min(ui.available_width() - 32.0).max(220.0);
        let marge = ((ui.available_width() - largeur) / 2.0).max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(marge);
            // DE HAUT EN BAS, DIT EXPRESSÉMENT : on est dans une rangée,
            // et un `Ui` hérite de la mise en page de son parent. Sans
            // cette ligne, les quatre entrées se peignaient les unes SUR
            // les autres, côte à côte sur une seule ligne — vu à la
            // capture, illisible.
            ui.allocate_ui_with_layout(
                egui::vec2(largeur, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.label(
                        egui::RichText::new(s.accueil_commencer)
                            .color(theme.dim)
                            .small(),
                    );
                    ui.add_space(4.0);
                    for (étiquette, quoi) in [
                        (s.accueil_nouveau, Action::New),
                        (s.accueil_ouvrir, Action::OpenFile),
                        (s.accueil_palette, Action::ToggleCommandPalette),
                    ] {
                        let touches = self
                            .keybindings
                            .map
                            .get(&quoi)
                            .map(|chord| chord.display(&ctx, fr))
                            .unwrap_or_default();
                        if self.accueil_ligne(ui, étiquette, &touches) {
                            action = Some(quoi);
                        }
                    }
                    // Les exercices, quand le module est allumé : c'est la
                    // porte d'entrée de qui apprend, et elle n'a pas de
                    // raccourci.
                    if self.exercises && self.accueil_ligne(ui, s.exercises_open, "") {
                        self.show_exercises = true;
                    }

                    let récents = self.recent_files.clone();
                    if !récents.is_empty() {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new(s.accueil_recents)
                                .color(theme.dim)
                                .small(),
                        );
                        ui.add_space(4.0);
                        for chemin in &récents {
                            let nom = chemin
                                .file_name()
                                .map(|nom| nom.to_string_lossy().into_owned())
                                .unwrap_or_else(|| chemin.to_string_lossy().into_owned());
                            // Le dossier à droite, là où les raccourcis se
                            // tiennent : deux fichiers peuvent porter le
                            // même nom, et c'est même le cas courant d'un
                            // dossier d'exercices.
                            let dossier = chemin
                                .parent()
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            if self.accueil_ligne(ui, &nom, &dossier) {
                                ouvrir = Some(chemin.clone());
                            }
                        }
                    }
                },
            );
        });

        if let Some(action) = action {
            self.dispatch(action);
        }
        if let Some(chemin) = ouvrir {
            self.open_file(chemin);
        }
    }

    /// Une ligne de l'accueil : ce qu'on fait à gauche, la touche ou le
    /// dossier à droite.
    ///
    /// Toute la ligne se clique, et non le seul libellé : une cible de
    /// quatre cents pixels de large se vise sans y penser, un mot de
    /// soixante demande de viser.
    fn accueil_ligne(&self, ui: &mut egui::Ui, étiquette: &str, droite: &str) -> bool {
        let theme = self.theme;
        let hauteur =
            ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().button_padding.y * 2.0;
        let (rect, réponse) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), hauteur),
            egui::Sense::click(),
        );
        if réponse.hovered() {
            ui.painter().rect_filled(rect, 4.0, theme.surface);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let police = egui::TextStyle::Body.resolve(ui.style());
        ui.painter().text(
            rect.left_center() + egui::vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            étiquette,
            police.clone(),
            theme.text,
        );
        if !droite.is_empty() {
            ui.painter().text(
                rect.right_center() - egui::vec2(8.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                droite,
                police,
                theme.dim,
            );
        }
        réponse.clicked()
    }
}
