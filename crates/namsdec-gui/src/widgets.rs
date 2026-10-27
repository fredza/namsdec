//! Les petites briques d'interface partagées : barres de validation,
//! barres de dialogue, boutons (texte, action, icône) et libellés
//! d'action.
//!
//! Aucune ne connaît l'état de l'application : chacune reçoit ce qu'elle
//! dessine. Sorties de `main.rs` pour l'alléger — elles servent partout
//! et n'y partageaient rien avec le reste.

use super::*;

/// La barre « OK » d'une fenêtre qui n'a rien à annuler.
///
/// Posée en panneau bas, et c'est ce qui compte : le contenu prend la
/// place qui reste, le bouton garde la sienne. Ajouté au corps de la
/// fenêtre, il se faisait pousser hors de l'écran par un contenu long —
/// c'est ce qui était arrivé au plan d'installation, dont la croix
/// elle-même sortait du cadre.
///
/// Rend `true` quand il faut fermer. Le séparateur au-dessus est léger :
/// une barre d'action doit se voir sans découper la fenêtre en deux.
/// La même barre, mais posée dans le flux — pour une fenêtre dont la
/// hauteur épouse son contenu.
///
/// Un panneau bas est ancré en bas du cadre de la fenêtre. Quand ce
/// cadre est mesuré sur le contenu, les deux se poursuivent : le
/// contenu déborde sous le panneau, la fenêtre grandit d'autant, le
/// panneau redescend, et l'équilibre trouvé laisse la dernière ligne
/// *sous* le bouton. Ce n'est pas un réglage à ajuster, c'est la
/// géométrie du montage — la calculatrice écrivait « Survolez une
/// fonction… » à travers son OK.
///
/// Posée en dernier dans le flux, la barre n'a plus rien à quoi
/// s'ancrer et suit simplement ce qui la précède. À n'employer que là :
/// dans une fenêtre haute, un contenu long la repousserait hors de
/// l'écran, ce qui est la raison d'être du panneau.
pub(crate) fn ok_bar_inline(ui: &mut egui::Ui, theme: &'static Theme, s: &Strings) -> bool {
    ui.add_space(8.0);
    let fermer = dialog_bar(ui, theme, s.dialog_ok, None).is_some();
    ui.add_space(2.0);
    fermer
}

pub(crate) fn ok_bar(ui: &mut egui::Ui, theme: &'static Theme, s: &Strings, id: &str) -> bool {
    let mut fermer = false;
    egui::Panel::bottom(egui::Id::new(format!("{id}_ok")))
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            // Du blanc, et non un filet. Une fenêtre qui porte déjà un
            // trait sous son titre et un autre au-dessus de son bouton
            // se lit comme un formulaire : trois bandes empilées. C'est
            // l'espace qui sépare, et il sépare aussi bien.
            ui.add_space(8.0);
            if dialog_bar(ui, theme, s.dialog_ok, None).is_some() {
                fermer = true;
            }
            ui.add_space(2.0);
        });
    fermer
}

/// Ce qu'une boîte de dialogue a reçu comme réponse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Le bouton principal, ou Entrée.
    Ok,
    /// Le second bouton, ou Échap, ou la croix.
    Cancel,
}

/// La barre d'action d'une boîte de dialogue : toujours la même.
///
/// Elles ne se ressemblaient pas d'une fenêtre à l'autre — OK et Quitter
/// ici, Fermer et Recommencer là, une croix seule ailleurs, et cette
/// croix sortait de l'écran dès que la fenêtre était haute. Or c'est
/// l'exception qui fait chercher : une barre d'action se trouve
/// d'autant plus vite qu'elle est au même endroit partout.
///
/// Trois règles tenues ici :
///
/// - le bouton principal à droite, plein, en couleur d'accent ;
/// - le second à sa gauche, seulement quand il y a vraiment quelque
///   chose à annuler — un bouton qui ne défait rien ment, et il vaut
///   mieux n'en pas mettre ;
/// - **Entrée vaut le principal, Échap vaut l'autre.** Relevés avant les
///   boutons et pris à la file : sans cela, un champ de texte de la même
///   fenêtre les aurait déjà consommés.
///
/// `annuler` porte le libellé du second bouton, ou `None` pour n'en pas
/// avoir. Dans ce dernier cas Échap répond quand même `Cancel` : fermer
/// une boîte qui ne fait que renseigner n'annule rien, mais on doit
/// pouvoir la fermer d'une touche.
pub(crate) fn dialog_bar(
    ui: &mut egui::Ui,
    theme: &Theme,
    principal: &str,
    annuler: Option<&str>,
) -> Option<Answer> {
    dialog_bar_hinted(ui, theme, principal, annuler, None)
}

/// La même, avec une infobulle sur le second bouton.
///
/// Séparée pour que le cas ordinaire reste court : la plupart des boîtes
/// n'ont rien à expliquer de plus que le mot écrit sur le bouton. Quand
/// ce mot ne suffit pas — « Recommencer » vaut un quart d'heure de
/// compilation —, le dire au survol vaut mieux que de l'allonger.
pub(crate) fn dialog_bar_hinted(
    ui: &mut egui::Ui,
    theme: &Theme,
    principal: &str,
    annuler: Option<&str>,
    infobulle: Option<&str>,
) -> Option<Answer> {
    let mut réponse = None;
    // Avant les boutons : les touches se prennent à la file, et un champ
    // de la même fenêtre les aurait sinon déjà mangées.
    ui.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
            réponse = Some(Answer::Ok);
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
            réponse = Some(Answer::Cancel);
        }
    });
    // La rangée reçoit une hauteur, et c'est tout le sujet.
    //
    // `with_layout` aurait donné à cette rangée *toute* la place qui
    // restait, parce qu'une disposition horizontale calée en
    // `Align::Center` doit connaître la hauteur dans laquelle centrer :
    // elle prend donc celle de son cadre entier. Dans un panneau bas,
    // dont la hauteur est justement mesurée sur son contenu, cela se
    // referme en boucle — le panneau redonne à la frame suivante la
    // hauteur qu'il vient de lire, augmentée des marges qui l'entourent,
    // et grandit de cinq pixels par image. C'est ce qu'on voyait :
    // la fenêtre « montait » tant qu'on remuait la souris, puisque
    // chaque mouvement vaut une image de plus.
    //
    // Une hauteur imposée coupe la boucle : la rangée ne lit plus rien
    // de son parent. C'est le même remède que pour la ligne de la
    // console, dont la largeur s'emballait de la même manière.
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            // Le liseré est de la couleur du fond du bouton : on ne le voit
            // pas, et il sert quand même. egui ajoute la largeur du trait à
            // la taille d'un bouton tracé — « Quitter » faisait donc 27 px
            // de haut et « OK », posé huit pixels à sa gauche, 25. Deux
            // pixels sur une rangée de deux boutons, c'est peu à dire et
            // très visible : l'un dépasse l'autre en haut ET en bas.
            let ok = egui::Button::new(egui::RichText::new(principal).color(theme.bg))
                .fill(theme.accent)
                .stroke(egui::Stroke::new(1.0, theme.accent))
                .min_size(egui::vec2(96.0, 0.0));
            if ui.add(ok).clicked() {
                réponse = Some(Answer::Ok);
            }
            if let Some(annuler) = annuler {
                let autre = egui::Button::new(annuler)
                    .stroke(egui::Stroke::new(1.0, theme.dim))
                    .min_size(egui::vec2(96.0, 0.0));
                let cliqué = match infobulle {
                    Some(texte) => ui.add(autre).on_hover_text(texte).clicked(),
                    None => ui.add(autre).clicked(),
                };
                if cliqué {
                    réponse = Some(Answer::Cancel);
                }
            }
        },
    );
    réponse
}

/// Libellé d'une action, pour le tableau de raccourcis de Préférences.
pub(crate) fn action_label(s: &Strings, action: Action) -> &'static str {
    match action {
        Action::ToggleExplorer => s.action_toggle_explorer,
        Action::Assemble => s.action_assemble,
        Action::Run => s.action_run,
        Action::StepInto => s.action_step_into,
        Action::StepOver => s.action_step_over,
        Action::StepBack => s.action_step_back,
        Action::Stop => s.action_stop,
        Action::Reset => s.action_reset,
        Action::Save => s.action_save,
        Action::OpenFile => s.file_open_file,
        Action::OpenFolder => s.file_open_folder,
        Action::InstructionHelp => s.action_instruction_help,
        Action::GoToLabel => s.action_goto_label,
        Action::Find => s.action_find,
        Action::New => s.file_new,
        Action::ShowFlow => s.flow_open,
        Action::NextTab => s.action_next_tab,
        Action::PreviousTab => s.action_previous_tab,
        Action::CloseTab => s.action_close_tab,
        Action::DuplicateLine => s.action_duplicate_line,
        Action::HighlightMatches => s.action_highlight_matches,
        Action::RenameMatches => s.action_rename_matches,
        Action::NameAddress => s.action_name_address,
        Action::CommentAddress => s.action_comment_address,
        Action::NextMatch => s.action_next_match,
        Action::PreviousMatch => s.action_previous_match,
        Action::RunToCursor => s.action_run_to_cursor,
        Action::RunToCall => s.action_run_to_call,
        Action::RunToReturn => s.action_run_to_return,
        Action::RunToSyscall => s.action_run_to_syscall,
        Action::ToggleCalculator => s.calculator_open,
        Action::ToggleCommandPalette => s.palette_action,
        Action::ShowPreferences => s.file_preferences,
        Action::ToggleConsole => s.palette_toggle_console,
        Action::ToggleSidebar => s.palette_toggle_sidebar,
    }
}

/// Texte d'info-bulle pour un bouton lié à une action : le libellé plus,
/// s'il existe, son raccourci courant — lu en direct dans `KeyBindings`
/// pour qu'un changement dans Préférences se reflète aussitôt.
pub(crate) fn shortcut_hint(
    bindings: &KeyBindings,
    lang: Lang,
    ctx: &egui::Context,
    label: &str,
    action: Action,
) -> String {
    match bindings.map.get(&action) {
        Some(chord) => format!("{label} ({})", chord.display(ctx, lang == Lang::Fr)),
        None => label.to_string(),
    }
}

/// Un grand bouton d'action : pleine largeur, rempli de la couleur
/// d'accent, une icône et un mot.
///
/// Pour les endroits où il n'y a **qu'une** chose à faire. Un bouton
/// ordinaire y est un rectangle gris de la taille de son texte, posé en
/// haut d'une colonne : on le prend pour une étiquette, et l'on cherche
/// ailleurs la commande qu'on a sous les yeux. Celui-ci occupe la
/// largeur, porte la couleur que le thème réserve à ce qui compte, et
/// se voit sans être lu.
///
/// Rendu désactivé quand il n'y a rien à faire, plutôt que caché : un
/// bouton qui disparaît laisse croire que la fonction n'existe pas.
pub(crate) fn big_action_button(
    ui: &mut egui::Ui,
    theme: &Theme,
    icon: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
    label: &str,
) -> egui::Response {
    const HAUTEUR: f32 = 42.0;
    const TAILLE_ICÔNE: f32 = 18.0;
    const ÉCART: f32 = 10.0;

    let largeur = ui.available_width();
    let (rect, réponse) =
        ui.allocate_exact_size(egui::vec2(largeur, HAUTEUR), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return réponse;
    }

    let actif = ui.is_enabled();
    // Le survol éclaircit, le clic assombrit : la même grammaire que les
    // boutons d'egui, avec la couleur du thème.
    let fond = match (
        actif,
        réponse.is_pointer_button_down_on(),
        réponse.hovered(),
    ) {
        (false, _, _) => theme.dim.gamma_multiply(0.25),
        (true, true, _) => theme.accent.gamma_multiply(0.80),
        (true, false, true) => theme.accent.gamma_multiply(1.15),
        (true, false, false) => theme.accent,
    };
    let encre = if actif { theme.bg } else { theme.dim };

    ui.painter().rect_filled(rect, 8.0, fond);

    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), egui::FontId::proportional(14.0), encre);
    // Icône et texte comptés ensemble, puis centrés d'un bloc : centrer
    // chacun de son côté laisse le texte de travers dès que le libellé
    // change de langue.
    let bloc = TAILLE_ICÔNE + ÉCART + galley.size().x;
    // Dans un panneau étroit, l'icône cède la place au mot : une icône
    // seule ne dit pas ce que le bouton fait, un mot tronqué non plus,
    // et entre les deux c'est le mot qui en dit le plus.
    if bloc > rect.width() - 8.0 {
        ui.painter().galley(
            egui::pos2(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            encre,
        );
        return réponse;
    }
    let gauche = rect.center().x - bloc * 0.5;
    let case = egui::Rect::from_min_size(
        egui::pos2(gauche, rect.center().y - TAILLE_ICÔNE * 0.5),
        egui::vec2(TAILLE_ICÔNE, TAILLE_ICÔNE),
    );
    icon(ui.painter(), case, encre);
    ui.painter().galley(
        egui::pos2(
            case.right() + ÉCART,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        encre,
    );
    réponse
}

/// Bouton de barre d'outils avec une icône dessinée directement plutôt
/// qu'un glyphe Unicode — "●", puis "↓"/"→"/"🔨" se sont tous révélés
/// absents de la police par défaut d'egui et retombaient sur des carrés
/// vides ou des glyphes de repli erratiques (voir commit 5d95e09 et le
/// retour utilisateur qui a suivi). Peindre la forme soi-même élimine le
/// risque une fois pour toutes.
pub(crate) fn icon_button(
    ui: &mut egui::Ui,
    icon: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
    label: &str,
) -> egui::Response {
    let icon_size = 14.0;
    let gap = 6.0;
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(13.0),
        egui::Color32::PLACEHOLDER,
    );
    let padding = ui.spacing().button_padding;
    // LA HAUTEUR D'UN BOUTON, et non celle d'un widget quelconque.
    //
    // `interact_size.y` vaut dix-huit : c'est la hauteur que donne egui
    // à ce qu'on ne lui a pas décrit. Un bouton de cette application
    // vaut son texte plus deux fois son rembourrage — vingt-cinq —, et
    // c'est ce que mesurent « Assembler » et les cinq onglets de la même
    // rangée. Les six commandes qu'on emploie le plus — exécuter, les
    // trois pas, arrêter, réinitialiser — étaient donc les plus petites
    // de la barre, sept pixels sous leurs voisines immédiates, et six
    // sous les vingt-quatre que l'on vise à la souris.
    //
    // Un bouton à icône n'est pas une autre sorte de bouton.
    let hauteur = (galley.size().y + padding.y * 2.0).max(ui.spacing().interact_size.y);
    let desired_size = egui::vec2(icon_size + gap + galley.size().x + padding.x * 2.0, hauteur);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + padding.x, rect.center().y - icon_size / 2.0),
            egui::vec2(icon_size, icon_size),
        );
        icon(ui.painter(), icon_rect, visuals.text_color());
        let text_pos = egui::pos2(
            icon_rect.right() + gap,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(text_pos, galley, visuals.text_color());
    }

    response
}

/// Un panneau vide qui dit ce qui manque, et propose ce qui le corrige.
///
/// La règle vient de l'explorateur, qui l'appliquait seul : « un panneau
/// simplement vide se lit comme cassé ». Ailleurs, six panneaux du
/// bandeau latéral disaient « Aucun binaire assemblé. » et s'arrêtaient
/// là — un constat, pas une porte. Or c'est exactement l'écran qu'on a
/// sous les yeux la première fois qu'on ouvre namsdec.
///
/// Trois étages, et les deux derniers sont facultatifs : la phrase qui
/// dit ce qui manque, celle qui dit ce qu'on verra là une fois le
/// programme lancé, et le bouton qui y mène. Rend `true` quand on a
/// cliqué.
pub(crate) fn empty_state(
    ui: &mut egui::Ui,
    theme: &Theme,
    phrase: &str,
    détail: Option<&str>,
    action: Option<&str>,
) -> bool {
    let mut cliqué = false;
    ui.add_space(10.0);
    // Toute la largeur, et le texte replié dedans : centrer une phrase
    // longue sans la replier la ferait déborder du panneau.
    ui.vertical_centered(|ui| {
        // BORNÉ À UNE LARGEUR DE LECTURE, et centré dedans.
        //
        // L'état vide prenait toute la place offerte. Dans le panneau
        // latéral, trois cents pixels : parfait. Dans une vue de la
        // rangée, mille deux cents — et la phrase qui explique ce
        // qu'est un décompilateur s'étirait sur une seule ligne d'un
        // bout à l'autre de l'écran. Un œil qui revient à la ligne
        // perd son rang au-delà d'une quinzaine de mots ; c'est la
        // raison pour laquelle un livre n'est pas composé sur la
        // largeur de sa page.
        //
        // Le minimum garde les panneaux étroits inchangés : ils
        // offrent moins que cette borne, et c'est leur largeur qui
        // commande.
        const LARGEUR_DE_LECTURE: f32 = 560.0;
        ui.set_max_width(ui.available_width().min(LARGEUR_DE_LECTURE) - 16.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.colored_label(theme.text, phrase);
        if let Some(détail) = détail {
            ui.add_space(4.0);
            ui.colored_label(theme.dim, egui::RichText::new(détail).small());
        }
        if let Some(action) = action {
            ui.add_space(10.0);
            let bouton = egui::Button::new(egui::RichText::new(action).color(theme.bg))
                .fill(theme.accent)
                .corner_radius(6.0);
            cliqué = ui.add(bouton).clicked();
        }
    });
    ui.add_space(10.0);
    cliqué
}

/// Un bouton carré qui ne porte qu'une icône, pour les commandes serrées
/// contre un champ — là où un libellé prendrait la place de la saisie.
pub(crate) fn icon_square_button(
    ui: &mut egui::Ui,
    icon: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
) -> egui::Response {
    let side = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        // Un cadre, toujours : au repos, le thème rend le fond des
        // boutons transparent et le liseré invisible — deux icônes
        // posées à côté d'un champ passaient alors pour de la décoration.
        // Un bouton doit se voir sans qu'on ait à le survoler.
        let stroke = if visuals.bg_stroke.width > 0.0 {
            visuals.bg_stroke
        } else {
            egui::Stroke::new(1.0, ui.visuals().weak_text_color())
        };
        ui.painter().rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            stroke,
            egui::StrokeKind::Inside,
        );
        // Trois pixels, et non cinq : sur un bouton de dix-huit, cinq
        // ne laissaient que huit pixels à l'icône — de quoi faire une
        // tache, pas un dessin. Le cadre reste lisible.
        icon(ui.painter(), rect.shrink(3.0), visuals.text_color());
    }
    response
}

/// Un bouton **encadré**, pour les endroits où il doit se voir sans
/// qu'on ait à le survoler.
///
/// Le thème met les boutons à plat au repos — c'est ce qui empêche la
/// barre de menus et la barre d'outils de se ressembler. Dans une boîte
/// de dialogue, l'effet se retourne contre nous : « Ajouter un thème… »
/// se lisait comme une phrase, et personne n'allait cliquer dessus.
pub(crate) fn framed_button(ui: &mut egui::Ui, theme: &Theme, label: &str) -> egui::Response {
    ui.add(framed(theme, label))
}

/// Le même, en petit — pour ce qui se glisse dans une rangée dense.
///
/// `ui.small_button` existait pour cela et posait un bouton PLAT : onze
/// dans l'application, et personne ne les voyait parce que le contrôle
/// de source ne connaissait que trois écritures de bouton sur quatre.
/// « RAX », « RSP » et « RIP », dans le motif cyclique, se lisaient
/// comme trois étiquettes à soixante pixels de trois boutons encadrés.
///
/// Petit veut dire : le rembourrage vertical d'egui en moins, donc une
/// rangée qui reste serrée. Il ne veut pas dire invisible — c'est tout
/// l'objet de celui-ci.
pub(crate) fn small_framed_button(
    ui: &mut egui::Ui,
    theme: &Theme,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .small()
            .stroke(egui::Stroke::new(1.0, theme.dim))
            .corner_radius(egui::CornerRadius::from(6)),
    )
}

/// Le bouton encadré, non encore posé.
///
/// `framed_button` l'ajoute tout de suite ; celui-ci le rend, pour les
/// endroits qui ont besoin de le poser eux-mêmes — `add_enabled`, par
/// exemple, qui grise un bouton tant qu'une autre capture est en cours.
/// C'est le cas des deux boutons de chaque rangée de Préférences ›
/// Raccourcis, « Touche simple… » et « Séquence… », qui n'avaient donc
/// pas de liseré : trente rangées de deux boutons qu'on ne distinguait
/// d'une étiquette qu'au survol.
pub(crate) fn framed<'a>(theme: &Theme, label: impl Into<egui::WidgetText>) -> egui::Button<'a> {
    egui::Button::new(label)
        .stroke(egui::Stroke::new(1.0, theme.dim))
        .corner_radius(egui::CornerRadius::from(6))
}

/// La vignette d'un thème, de largeur imposée.
///
/// Imposée, et non « toute la place » : les boutons « Retirer » qui la
/// suivent doivent s'aligner d'une ligne à l'autre, donc toutes les
/// vignettes doivent finir à la même abscisse — celle que dicte le nom
/// le plus long (voir `theme_card_width`).
pub(crate) fn theme_card(
    ui: &mut egui::Ui,
    theme: &Theme,
    active: bool,
    active_word: &str,
    width: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 44.0), egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, active);
        let painter = ui.painter();
        painter.rect(
            rect,
            8,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        let preview = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.center().y - 14.0),
            egui::vec2(58.0, 28.0),
        );
        painter.rect_filled(preview, 4, theme.bg);
        painter.rect_filled(
            egui::Rect::from_min_size(preview.min, egui::vec2(14.0, preview.height())),
            4,
            theme.panel,
        );
        for (i, color) in [theme.accent, theme.good, theme.warn, theme.bad]
            .iter()
            .enumerate()
        {
            painter.circle_filled(
                egui::pos2(preview.left() + 23.0 + i as f32 * 9.0, preview.center().y),
                3.0,
                *color,
            );
        }
        painter.rect_stroke(
            preview,
            4,
            egui::Stroke::new(1.0, theme.dim),
            egui::StrokeKind::Inside,
        );

        painter.text(
            egui::pos2(preview.right() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            theme.name,
            egui::FontId::proportional(14.0),
            visuals.text_color(),
        );
        if active {
            painter.text(
                egui::pos2(rect.right() - 10.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                active_word,
                egui::FontId::proportional(12.0),
                theme.accent,
            );
        }
    }

    response
}

/// La largeur commune des vignettes : de quoi loger la plus longue des
/// étiquettes — le nom, et « actif » qui le suit quand le thème l'est.
///
/// Mesurée sur les polices réelles plutôt qu'estimée : un thème installé
/// peut s'appeler « Solarized Light Extra Contrast », et les boutons
/// doivent rester alignés.
pub(crate) fn theme_card_width(ui: &egui::Ui, themes: &[&Theme], active_word: &str) -> f32 {
    let text_width = |text: &str, size: f32| {
        ui.painter()
            .layout_no_wrap(
                text.to_owned(),
                egui::FontId::proportional(size),
                egui::Color32::PLACEHOLDER,
            )
            .size()
            .x
    };
    let widest_name = themes
        .iter()
        .map(|theme| text_width(theme.name, 14.0))
        .fold(0.0_f32, f32::max);
    // Vignette (8 + 58) + espace (10) + nom + espace + « actif » + marge.
    76.0 + widest_name + 12.0 + text_width(active_word, 12.0) + 10.0
}

/// Le bouton d'agrandissement, posé dans le bandeau d'une fenêtre.
///
/// `egui::Window` ne laisse rien mettre dans son bandeau : la croix y est
/// posée par son propre agencement, hors d'atteinte. On dessine donc le
/// bouton par-dessus, une fois la fenêtre rendue — mais à la géométrie
/// exacte de la croix, et non à vue de nez.
///
/// Cette géométrie est celle qu'egui emploie pour ses propres boutons de
/// bandeau : une case carrée de la hauteur d'une ligne de titre, dans
/// laquelle l'icône est centrée à `icon_width`. La case allouée est plus
/// grande que l'icône, faute de quoi les boutons paraîtraient décalés
/// vers le haut à côté d'un titre plus haut qu'eux. Le nôtre se range
/// d'une case à gauche, séparé par l'écart ordinaire entre éléments.
pub(crate) fn title_bar_maximize_button(
    ctx: &egui::Context,
    id: &'static str,
    fenêtre: egui::Rect,
    theme: &'static Theme,
    s: &Strings,
    agrandie: bool,
) -> bool {
    let style = ctx.style_of(ctx.theme());
    let icône_côté = style.spacing.icon_width;
    // La hauteur d'une ligne de titre : c'est elle qui donne sa taille à
    // la case, non l'icône.
    let case_côté = ctx.fonts_mut(|f| f.row_height(&egui::TextStyle::Heading.resolve(&style)));
    let écart = style.spacing.item_spacing.x;
    let marge_droite = f32::from(style.spacing.window_margin.right);
    let marge_haut = f32::from(style.spacing.window_margin.top);

    // La case de la croix, puis la nôtre juste à sa gauche.
    let croix = egui::Rect::from_min_size(
        egui::pos2(
            fenêtre.right() - marge_droite - case_côté,
            fenêtre.top() + marge_haut,
        ),
        egui::vec2(case_côté, case_côté),
    );
    let case = croix.translate(egui::vec2(-(case_côté + écart), 0.0));
    let dessin = egui::Rect::from_center_size(case.center(), egui::Vec2::splat(icône_côté));

    // Le double-clic sur le bandeau fait la même chose, partout ailleurs.
    // Il est libre ici : ces fenêtres ne se replient pas, et c'est le
    // repli qu'egui lui donne d'ordinaire.
    let bandeau = egui::Rect::from_min_max(fenêtre.min, egui::pos2(croix.left(), croix.bottom()));
    let double = ctx.input(|i| {
        i.pointer
            .button_double_clicked(egui::PointerButton::Primary)
            && i.pointer
                .interact_pos()
                .is_some_and(|p| bandeau.contains(p))
    });

    let mut cliqué = double;
    egui::Area::new(egui::Id::new(("maximize", id)))
        .order(egui::Order::Foreground)
        .fixed_pos(case.min)
        .show(ctx, |ui| {
            let (rect, réponse) = ui.allocate_exact_size(case.size(), egui::Sense::click());
            let couleur = if réponse.hovered() {
                theme.text
            } else {
                theme.dim
            };
            if réponse.hovered() {
                ui.painter()
                    .rect_filled(rect, 3.0, theme.dim.gamma_multiply(0.25));
            }
            // L'icône au centre de la case, à la taille des icônes
            // d'egui : la croix voisine est peinte exactement ainsi.
            let dessin = egui::Rect::from_center_size(rect.center(), dessin.size());
            if agrandie {
                icon_restore(ui.painter(), dessin, couleur);
            } else {
                icon_maximize(ui.painter(), dessin, couleur);
            }
            let quoi = if agrandie {
                s.window_restore
            } else {
                s.window_maximize
            };
            cliqué |= réponse.on_hover_text(quoi).clicked();
        });
    cliqué
}

/// Une ligne de la vue d'ensemble des modules : nom, version, point coloré
/// de statut, et une courte description de ce que fait ce module là,
/// maintenant.
pub(crate) fn module_card(
    ui: &mut egui::Ui,
    name: &str,
    version: &str,
    active: bool,
    color: egui::Color32,
    detail: &str,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            status_dot(ui, if active { color } else { egui::Color32::GRAY });
            ui.strong(name);
            ui.weak(format!("v{version}"));
        });
        ui.label(detail);
    });
    ui.add_space(4.0);
}

/// La hauteur d'une ligne d'une rangée d'onglets plats.
///
/// Plus basse que celle des fichiers (30) : une rangée d'onglets plats
/// est subordonnée à ce qui la surmonte, et l'œil doit le voir avant de
/// lire.
pub(crate) const ONGLET_HAUTEUR: f32 = 26.0;
/// De chaque côté d'une étiquette, et la gouttière qu'une commande
/// garde au bord.
pub(crate) const ONGLET_MARGE: f32 = 12.0;

/// Une rangée d'onglets PLATS, qui passe à la ligne quand la place
/// manque.
///
/// Deux endroits la peignent — les vues de la zone centrale, et les
/// vues du panneau latéral —, et c'est le même corps pour les deux :
/// ce sont la même chose, des façons de regarder un même objet, et
/// elles doivent se lire pareil. Le jour où l'une des deux divergerait,
/// l'application aurait trois idiomes d'onglets au lieu de deux.
///
/// Les deux idiomes, justement : l'onglet de FICHIER est un rectangle
/// plein, marqué EN HAUT, avec sa croix — c'est un document qu'on ouvre
/// et qu'on ferme ; l'onglet de VUE est une étiquette plate, soulignée
/// EN BAS, qui ne se ferme pas. Marques opposées, formes opposées : on
/// ne les confond pas, même du coin de l'œil.
///
/// `réserve` est la place gardée libre à droite, sur toutes les lignes,
/// et `commande` ce qu'on y pose — posé DANS la première ligne, et non
/// après coup : `ui.put` ramène le curseur au rectangle qu'on lui donne,
/// de sorte qu'une commande posée après les lignes faisait repartir
/// tout ce qui suit du bas de la PREMIÈRE. Mesuré : à 800 x 520, la
/// deuxième ligne d'onglets se peignait sur le « Moteur : » du
/// désassemblage.
///
/// Les étiquettes sont MESURÉES AVANT D'ÊTRE POSÉES : une ligne ne se
/// mesure pas pendant qu'on la peint — ce qui déborde serait déjà peint
/// dehors, rogné au bord, et un onglet rogné se lit comme un onglet
/// entier. Rien n'est repris de l'image précédente : la place
/// disponible et la largeur des textes ne sont pas produites par la
/// rangée, donc rien ne peut se reboucler.
pub(crate) fn rangée_d_onglets(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    onglets: &[(&str, Option<&str>)],
    actif: usize,
    réserve: f32,
    commande: impl FnOnce(&mut egui::Ui),
) -> Option<usize> {
    let police = egui::TextStyle::Body.resolve(ui.style());
    let galettes: Vec<std::sync::Arc<egui::Galley>> = onglets
        .iter()
        .map(|(libellé, _)| {
            ui.painter().layout_no_wrap(
                (*libellé).to_owned(),
                police.clone(),
                egui::Color32::PLACEHOLDER,
            )
        })
        .collect();

    let dispo = (ui.available_width() - réserve).max(0.0);
    let mut lignes: Vec<Vec<usize>> = vec![Vec::new()];
    let mut large = 0.0_f32;
    for (rang, galette) in galettes.iter().enumerate() {
        let onglet = galette.size().x + ONGLET_MARGE * 2.0;
        if !lignes.last().is_some_and(|l| l.is_empty()) && large + onglet > dispo {
            lignes.push(Vec::new());
            large = 0.0;
        }
        lignes.last_mut().expect("une ligne au moins").push(rang);
        large += onglet;
    }

    let mut choisi = None;
    let mut commande = Some(commande);
    for (numéro, ligne) in lignes.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for rang in ligne {
                let galette = &galettes[*rang];
                let ici = *rang == actif;
                let (rect, réponse) = ui.allocate_exact_size(
                    egui::vec2(galette.size().x + ONGLET_MARGE * 2.0, ONGLET_HAUTEUR),
                    egui::Sense::click(),
                );
                let peintre = ui.painter();
                if réponse.hovered() && !ici {
                    peintre.rect_filled(rect, 0.0, mélange(theme.panel, theme.text, 0.06));
                }
                peintre.galley(
                    egui::pos2(
                        rect.left() + ONGLET_MARGE,
                        rect.center().y - galette.size().y / 2.0,
                    ),
                    galette.clone(),
                    if ici { theme.text } else { theme.dim },
                );
                if ici {
                    peintre.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(rect.left(), rect.bottom() - 2.0),
                            egui::vec2(rect.width(), 2.0),
                        ),
                        0.0,
                        theme.accent,
                    );
                }
                if réponse.clicked() {
                    choisi = Some(*rang);
                }
                let réponse = réponse.on_hover_cursor(egui::CursorIcon::PointingHand);
                // L'infobulle seulement quand l'étiquette a dû
                // raccourcir ce qu'elle nomme : une infobulle qui répète
                // l'étiquette apprend qu'il est inutile de survoler.
                if let Some(infobulle) = onglets[*rang].1 {
                    réponse.on_hover_text(infobulle);
                }
            }
            // La commande, au bord droit de la première ligne : sa
            // gouttière d'abord, puisqu'on part du bord.
            if numéro == 0
                && réserve > 0.0
                && let Some(commande) = commande.take()
            {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(ONGLET_MARGE);
                    commande(ui);
                });
            }
        });
    }
    choisi
}
