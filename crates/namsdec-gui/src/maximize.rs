//! Une fenêtre qui prend tout l'écran, et qui sait revenir.
//!
//! Le geste est banal ailleurs et manquait ici : un désassemblage de
//! quinze mille lignes dans une fenêtre de six cents pixels se lit mal.
//! Toute fenêtre de l'établi peut donc s'agrandir, par son bouton ou
//! par un double-clic sur son bandeau.
//!
//! « Agrandie » est un état retenu par fenêtre et par session : ce qu'on
//! avait mis en grand la veille l'est encore au réveil.

use super::*;

impl NamsdecApp {
    /// La fenêtre du déroulement : le programme, tel qu'il se parcourt.
    ///
    /// Un listing se lit de haut en bas ; un programme, non. Les sauts
    /// le plient — une boucle remonte, un test ouvre deux chemins, un
    /// appel part et revient. Ce dessin est la forme que l'on
    /// reconstitue de tête en lisant, et qu'un débutant ne voit pas
    /// encore.
    /// Cette fenêtre est-elle agrandie ?
    pub(crate) fn window_maximized(&self, id: &'static str) -> bool {
        self.maximized_windows.contains(id)
    }

    /// Règle une fenêtre pour qu'elle occupe tout l'écran.
    ///
    /// `fixed_size` donne la taille du *contenu* : le cadre de la fenêtre
    /// s'ajoute autour. Sans retrancher ses marges, la fenêtre dépassait
    /// de l'écran et se faisait rogner. Et `pivot` n'a plus lieu d'être —
    /// c'est un coin qu'on vise, non un centre.
    pub(crate) fn fill_the_screen<'a>(
        fenêtre: egui::Window<'a>,
        ctx: &egui::Context,
    ) -> egui::Window<'a> {
        let place = Self::maximized_rect(ctx);
        let marge = ctx.style_of(ctx.theme()).spacing.window_margin;
        let cadre = egui::vec2(
            f32::from(marge.left + marge.right),
            f32::from(marge.top + marge.bottom),
        );
        fenêtre
            .resizable(false)
            .pivot(egui::Align2::LEFT_TOP)
            .fixed_pos(place.min)
            .fixed_size(place.size() - cadre)
    }

    /// Le rectangle d'une fenêtre agrandie : tout ce qu'il y a.
    ///
    /// Sans marge : une fenêtre agrandie qui laisse voir douze pixels de
    /// ce qu'elle recouvre n'a pas l'air agrandie, elle a l'air mal
    /// posée.
    pub(crate) fn maximized_rect(ctx: &egui::Context) -> egui::Rect {
        ctx.viewport_rect()
    }

    /// Rend cette fenêtre agrandissable : elle prend tout l'écran quand
    /// on le lui a demandé, et garde ses réglages sinon.
    ///
    /// À poser en dernier sur la fenêtre, après ses tailles : `fixed_*`
    /// et `default_*` se disputent la même place, et le dernier écrit
    /// gagne.
    pub(crate) fn maximizable<'a>(
        &self,
        fenêtre: egui::Window<'a>,
        id: &'static str,
        ctx: &egui::Context,
    ) -> egui::Window<'a> {
        if self.window_maximized(id) {
            Self::fill_the_screen(fenêtre, ctx)
        } else {
            fenêtre
        }
    }

    /// Pose le bouton d'agrandissement dans le bandeau d'une fenêtre qui
    /// vient d'être dessinée, et applique ce qu'on en a fait.
    pub(crate) fn place_maximize_button<T>(
        &mut self,
        ctx: &egui::Context,
        id: &'static str,
        rendue: Option<egui::InnerResponse<Option<T>>>,
    ) {
        let agrandie = self.window_maximized(id);
        if let Some(rendue) = rendue
            && title_bar_maximize_button(
                ctx,
                id,
                rendue.response.rect,
                self.theme,
                strings(self.lang),
                agrandie,
            )
        {
            self.toggle_maximized(id);
        }
    }

    /// Déplace le curseur du mode Normal, et l'écrit dans l'état du
    /// champ — c'est lui qui le porte, même quand il n'a pas le focus.
    pub(crate) fn move_in_normal_mode(&mut self, ctx: &egui::Context, mouvement: Move) {
        let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) else {
            return;
        };
        let at = state
            .cursor
            .char_range()
            .map_or(0, |range| range.primary.index.0);
        let cible = move_cursor(&self.source, at, mouvement);
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(cible),
            )));
        state.store(ctx, editor_id());
        self.cursor_pos = line_col(&self.source, cible);
        self.cursor_byte = char_to_byte(&self.source, cible);
        // Le listing suit : se déplacer hors de l'écran sans que la vue
        // bouge reviendrait à ne pas se déplacer.
        self.follow_execution();
    }

    /// Applique ce qu'un clic sur le bouton d'agrandissement demande.
    pub(crate) fn toggle_maximized(&mut self, id: &'static str) {
        if !self.maximized_windows.remove(id) {
            self.maximized_windows.insert(id);
        }
    }
}
