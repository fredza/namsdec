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

/// Les fenêtres qui savent s'agrandir : leur **clé d'agrandissement**,
/// puis l'**identifiant egui** de la fenêtre.
///
/// **Deux identifiants pour une fenêtre, et c'est une chausse-trappe de
/// ce code** : la clé (`EXERCISES_WINDOW` vaut « exercises ») n'est pas
/// l'identifiant de la fenêtre (« exercises_window_v2 »). Les confondre
/// fait basculer un drapeau que personne ne lit — le bouton paraît
/// mort. Les deux sont donc écrits côte à côte, une fois, ici.
pub(crate) const FENÊTRES_AGRANDISSABLES: &[(&str, &str)] = &[
    ("calculator", "calculator_window"),
    ("exercises", "exercises_window_v2"),
    ("disassembly", "disassembly_window"),
    ("timeline", "timeline_window"),
    ("analysis", "analysis_window"),
    ("flow", "flow_window"),
    ("decompiler", "decompiler_window"),
];

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

    /// La zone de travail : tout l'écran, sauf les barres.
    ///
    /// Une fenêtre agrandie à `viewport_rect()` recouvre la barre de
    /// menus et la barre d'état — on ne peut plus ouvrir un menu, et
    /// l'on ne sait plus où l'on en est. C'est supportable pour une
    /// toile qu'on regarde (le graphe), et ça ne l'est pas pour un
    /// panneau d'où l'on repart sans cesse vers le listing.
    ///
    /// `zone_de_travail` est ce qui restait à l'image précédente, une
    /// fois les barres posées — le contexte, lui, ne connaît que
    /// l'écran entier. On n'en garde que le BORD HAUT, et l'on prend
    /// tout le reste.
    ///
    /// Le bord haut, parce que c'est là qu'est la barre de menus : une
    /// fenêtre qui la recouvre coupe l'accès à tout ce qu'on pourrait
    /// vouloir faire ensuite. Le bas, non : la barre d'état et la
    /// console sont de la lecture, pas des commandes, et agrandir doit
    /// agrandir — une fenêtre qui s'étale moins haut qu'avant n'a pas
    /// l'air agrandie, elle a l'air déplacée.
    pub(crate) fn work_area_rect(&self, ctx: &egui::Context) -> egui::Rect {
        let tout = ctx.viewport_rect();
        if !self.zone_de_travail.is_positive() {
            return tout;
        }
        egui::Rect::from_min_max(
            egui::pos2(tout.left(), self.zone_de_travail.top()),
            tout.max,
        )
    }

    /// Rend cette fenêtre agrandissable DANS LA ZONE DE TRAVAIL.
    pub(crate) fn maximizable_in_work_area<'a>(
        &self,
        fenêtre: egui::Window<'a>,
        id: &'static str,
        ctx: &egui::Context,
    ) -> egui::Window<'a> {
        if !self.window_maximized(id) {
            return self.restaurer_si_demandé(fenêtre, id);
        }
        let place = self.work_area_rect(ctx);
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
            self.restaurer_si_demandé(fenêtre, id)
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

    /// Déplace le curseur du mode Normal `fois` fois — le compteur de
    /// vim, `5j` —, et l'écrit dans l'état du champ : c'est lui qui le
    /// porte, même quand il n'a pas le focus.
    ///
    /// La répétition est faite dans `move_cursor_times`, qui ne découpe
    /// le texte qu'une fois : c'est ce qui permet à `5j` de coûter ce
    /// que coûte `j`.
    pub(crate) fn move_in_normal_mode_times(
        &mut self,
        ctx: &egui::Context,
        mouvement: Move,
        fois: u32,
    ) {
        // LE CURSEUR EST CELUI DE L'APPLICATION, et non celui du champ
        // de texte. En mode Normal, le champ n'est pas même dessiné —
        // c'est la vue ligne à ligne qui peint le listing (voir
        // `vue_listing`) —, et son état n'existe pas tant qu'on n'est
        // pas entré en Insertion : y lire le curseur revenait à ne pas
        // bouger du tout.
        let at = byte_to_char(&self.source, self.cursor_byte);
        let cible = move_cursor_times(&self.source, at, mouvement, fois);
        // Et il est écrit dans le champ s'il existe : entrer en
        // Insertion doit reprendre là où la lecture s'est arrêtée.
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(cible),
                )));
            state.store(ctx, editor_id());
        }
        // Ligne, colonne et octet en une passe, et gardés : voir
        // `cursor_place`.
        let (ligne, colonne, octet) = self.cursor_place(cible);
        self.cursor_pos = (ligne, colonne);
        self.cursor_byte = octet;
        // Le listing suit : se déplacer hors de l'écran sans que la vue
        // bouge reviendrait à ne pas se déplacer.
        self.follow_execution();
    }

    /// Applique ce qu'un clic sur le bouton d'agrandissement demande.
    ///
    /// **Restaurer demande plus que d'oublier l'agrandissement**, et
    /// c'est ce qui manquait : `fixed_pos` et `fixed_size` ÉCRIVENT
    /// dans la mémoire de la fenêtre, et les `default_*` qui les
    /// remplacent au retour ne valent que la première fois. La fenêtre
    /// restait donc pleine page, et le bouton paraissait mort —
    /// signalé à l'écran le 4 novembre 2026, capture à l'appui.
    ///
    /// On note donc la fenêtre comme « à restaurer », et la prochaine
    /// image lui réimpose le rectangle qu'elle avait avant de grandir.
    pub(crate) fn toggle_maximized(&mut self, id: &'static str) {
        if self.maximized_windows.remove(id) {
            self.à_restaurer.insert(id);
        } else {
            self.maximized_windows.insert(id);
        }
    }

    /// Retient où chaque fenêtre agrandissable se tient quand elle n'est
    /// PAS agrandie.
    ///
    /// Relevé à chaque image, avant que les fenêtres ne soient
    /// construites : c'est la seule mesure qui dise où l'utilisateur
    /// l'avait mise et de quelle taille il l'avait faite. Une fenêtre
    /// agrandie, ou qu'on est en train de restaurer, n'est pas relevée
    /// — on écraserait justement ce qu'on cherche à lui rendre.
    pub(crate) fn retenir_les_fenetres_normales(&mut self, ctx: &egui::Context) {
        for (clé, id) in FENÊTRES_AGRANDISSABLES {
            if self.maximized_windows.contains(clé) || self.à_restaurer.contains(clé) {
                continue;
            }
            if let Some(état) = egui::AreaState::load(ctx, egui::Id::new(*id)) {
                let rect = état.rect();
                if rect.is_positive() {
                    self.rects_avant_agrandissement
                        .insert(clé, (rect, état.pivot));
                }
            }
        }
    }

    /// Réimpose à une fenêtre, le temps d'UNE image, le rectangle
    /// qu'elle avait avant de grandir.
    ///
    /// Une seule image suffit : egui retient ce qu'on vient de lui
    /// imposer, et la fenêtre repart de là — redimensionnable comme
    /// avant. L'imposer plus longtemps la figerait.
    pub(crate) fn restaurer_si_demandé<'a>(
        &self,
        fenêtre: egui::Window<'a>,
        id: &'static str,
    ) -> egui::Window<'a> {
        if !self.à_restaurer.contains(id) {
            return fenêtre;
        }
        let Some((rect, pivot)) = self.rects_avant_agrandissement.get(id).copied() else {
            return fenêtre;
        };
        // LA TAILLE RELEVÉE, SANS RIEN RETRANCHER — et c'est l'inverse
        // de `fill_the_screen`, qui enlève les marges du cadre.
        //
        // La différence tient à ce qu'on mesure : là-bas, on vise une
        // ZONE (l'écran) et l'on calcule le contenu qui la remplira
        // exactement ; ici, on rend à la fenêtre une taille qu'on a
        // relevée SUR ELLE-MÊME. La retrancher d'un cadre la rétrécit à
        // chaque aller-retour — vingt-quatre pixels par voyage, mesuré.
        let taille = rect.size();
        // LA POSITION EXPRIMÉE DANS LE PIVOT DE LA FENÊTRE, et non dans
        // le nôtre.
        //
        // egui garde une position de PIVOT et recalcule le coin haut
        // gauche à partir de lui : `coin = pivot_pos − facteur ×
        // taille`. Le pivot employé est celui que la FENÊTRE déclare —
        // presque toutes disent « centre » —, et il est réécrit à
        // chaque image. Imposer ici un pivot « coin haut gauche »
        // tenait une image, puis la fenêtre sautait d'un demi-format
        // quand elle reprenait le sien : restaurée en (120, 20) au lieu
        // de (580, 380), mesuré.
        //
        // On lit donc son pivot, et on lui rend la position qui, DANS
        // CE PIVOT, remet le coin où il était.
        let ancre = rect.min
            + egui::vec2(
                pivot.x().to_factor() * taille.x,
                pivot.y().to_factor() * taille.y,
            );
        fenêtre
            .resizable(false)
            // SANS BORNAGE, LE TEMPS DE CETTE IMAGE — et c'est le point
            // qui a demandé la mesure.
            //
            // egui ramène une fenêtre dans l'écran en se servant de la
            // taille qu'il CONNAÎT à cet instant : celle de l'image
            // précédente, c'est-à-dire la pleine page. Une fenêtre de
            // 920 × 720 remise en (580, 380) était donc bornée comme si
            // elle en faisait 1576 × 1076 — elle ne tenait plus, et
            // egui la collait au coin : (24, 24). La taille revenait,
            // la position non, et c'est ce qu'on lisait à l'écran.
            //
            // La taille est posée dans la même image, donc ce bornage
            // n'a plus lieu dès l'image suivante : on le coupe ici, et
            // nulle part ailleurs.
            .constrain(false)
            .fixed_pos(ancre)
            .fixed_size(taille)
    }

    /// Les restaurations ont eu lieu : on les oublie.
    ///
    /// À la FIN de l'image, et non au début : la fenêtre est construite
    /// entre les deux, et c'est elle qui avait besoin du drapeau.
    pub(crate) fn fin_des_restaurations(&mut self) {
        self.à_restaurer.clear();
    }
}
