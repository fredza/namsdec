//! Retrouver un panneau par son NOM, et non par un `Id` deviné.
//!
//! Jusqu'à egui 0.36, un panneau portait l'`Id` qu'on lui donnait, et
//! `PanelState::load(ctx, Id::new("statusbar"))` rendait son état. En
//! 0.37, le constructeur prend un **sel** (`AsIdSalt`) : l'identité
//! réelle devient `parent.scope_id().with_salt(sel)`, elle dépend donc
//! du `Ui` qui montre le panneau, et `Panel::id` n'est pas public.
//!
//! Chercher sous l'ancien nom ne rend plus **rien, en silence** — et
//! douze endroits de namsdec en vivent : la barre d'état dit sa hauteur
//! pour que rien ne s'ouvre par-dessus, le panneau latéral rattrape une
//! largeur absurde enregistrée, l'onglet garde sa part, la console pose
//! sa saisie au bas. Tous auraient continué de compiler en ne mesurant
//! plus que du vide.
//!
//! On ne redevine donc pas la recette d'egui à chaque lecture : le
//! panneau note son identité à l'endroit — le seul — où le nom et le
//! `Ui` parent sont connus ensemble, et les lectures la retrouvent par
//! le nom.

/// La clé sous laquelle namsdec retient l'identité d'un panneau.
fn clé(nom: &str) -> egui::Id {
    egui::Id::unique(("namsdec/panneau", nom))
}

/// L'`Id` sous lequel egui rangera l'état du panneau `nom` montré dans
/// `parent` — et le retient, pour que le nom seul suffise ensuite.
///
/// À appeler avec le `Ui` qui montre le panneau, et nulle part
/// ailleurs : l'identité en dépend. Les quatre constructeurs ci-dessous
/// s'en chargent ; cette fonction-ci sert à qui a besoin de l'`Id`
/// AVANT de montrer le panneau, pour corriger ce qu'egui avait retenu.
pub(crate) fn id_dans(parent: &egui::Ui, nom: &str) -> egui::Id {
    let réel = parent.scope_id().with_salt(egui::IdSalt::new(nom));
    parent.ctx().data_mut(|d| d.insert_temp(clé(nom), réel));
    réel
}

/// `egui::Panel::top`, en retenant l'identité du panneau.
pub(crate) fn haut(parent: &egui::Ui, nom: &str) -> egui::Panel {
    id_dans(parent, nom);
    egui::Panel::top(nom)
}

/// `egui::Panel::bottom`, en retenant l'identité du panneau.
pub(crate) fn bas(parent: &egui::Ui, nom: &str) -> egui::Panel {
    id_dans(parent, nom);
    egui::Panel::bottom(nom)
}

/// `egui::Panel::left`, en retenant l'identité du panneau.
pub(crate) fn gauche(parent: &egui::Ui, nom: &str) -> egui::Panel {
    id_dans(parent, nom);
    egui::Panel::left(nom)
}

/// `egui::Panel::right`, en retenant l'identité du panneau.
pub(crate) fn droite(parent: &egui::Ui, nom: &str) -> egui::Panel {
    id_dans(parent, nom);
    egui::Panel::right(nom)
}

/// L'`Id` d'un panneau montré À LA RACINE — dans le `Ui` que
/// `Context::run_ui` donne, c'est-à-dire celui qu'`eframe` passe à
/// `App::ui`. Calculable AVANT la première image, donc avant que le
/// panneau n'ait rien noté.
///
/// C'est ce qu'il faut pour SEMER l'état qu'une session précédente
/// aurait laissé dans `app.ron` — une largeur absurde à rattraper. Il
/// ne vaut que pour les panneaux de la racine : un panneau montré dans
/// un autre a une portée plus profonde, et c'est `id` qu'il faut lui
/// demander, après une image.
///
/// Réservée aux contrôles : l'application, elle, montre ses panneaux
/// avant de lire leur état, donc `id` lui suffit toujours.
#[cfg(test)]
pub(crate) fn id_racine(ctx: &egui::Context, nom: &str) -> egui::Id {
    ctx.viewport_id()
        .root_ui_id()
        .with_salt(egui::IdSalt::new(nom))
}

/// L'`Id` sous lequel egui range l'état du panneau `nom`, s'il a été
/// montré au moins une fois dans cette session.
pub(crate) fn id(ctx: &egui::Context, nom: &str) -> Option<egui::Id> {
    ctx.data(|d| d.get_temp::<egui::Id>(clé(nom)))
}

/// L'état du panneau `nom` — sa géométrie de l'image précédente.
///
/// `None` tant que le panneau n'a pas paru : la toute première image,
/// ou un panneau que l'on a fermé. C'est au lecteur de dire ce qu'il
/// fait d'un panneau absent, et aucun ne se contente d'un zéro.
pub(crate) fn état(ctx: &egui::Context, nom: &str) -> Option<egui::containers::panel::PanelState> {
    egui::containers::panel::PanelState::load(ctx, id(ctx, nom)?)
}
