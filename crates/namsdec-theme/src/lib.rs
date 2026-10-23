//! Thèmes de l'interface, sous forme de données plutôt que de constantes
//! éparpillées dans le code de rendu.
//!
//! Un thème est une palette nommée ; tout ce qui dessine (éditeur,
//! explorateur, barres) lit ses couleurs ici. C'est ce qui permet d'en
//! ajouter d'autres sans toucher au rendu — un thème installé plus tard
//! n'est qu'un `Theme` de plus dans la liste.

pub mod plugin;

use std::sync::RwLock;

use egui::Color32;

pub use plugin::PluginError;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Palette complète d'un thème. Volontairement plate et explicite : un
/// champ par rôle, pas de dérivation automatique, pour qu'un thème
/// importé d'ailleurs (tokyonight, gruvbox…) se transcrive directement.
///
/// `Debug` sert aux tests et aux messages d'erreur d'installation.
#[derive(Debug)]
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
    /// Ce qui vient de changer — les registres qu'une instruction a
    /// modifiés. Une couleur à part, et **vive** : elle doit sauter aux
    /// yeux à chaque pas au milieu de seize valeurs figées, là où
    /// `warn` a le rôle plus tiède d'un avertissement qu'on peut lire
    /// plus tard.
    pub changed: Color32,

    // Coloration syntaxique NASM.
    pub mnemonic: Color32,
    pub number: Color32,
    pub label: Color32,
    pub comment: Color32,
    /// Les registres : `rax`, `xmm0`, `cl`… Ils forment la moitié d'une
    /// ligne d'assembleur et n'avaient aucune couleur à eux.
    pub register: Color32,
    /// Le texte entre guillemets, `db "salut", 10`.
    pub string: Color32,
    /// Les directives (`section`, `global`, `db`) et le préprocesseur
    /// (`%include`, `%define`) : ce qui s'adresse à nasm et non au
    /// processeur.
    pub directive: Color32,

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

        // L'ALERTE ET L'ERREUR SONT CELLES DU THÈME, et pas celles
        // d'egui. Elles manquaient, et personne ne les voyait manquer :
        // un module d'analyse qui dit « Cet outil n'est pas sur cette
        // machine. » employait `visuals().warn_fg_color`, c'est-à-dire
        // l'orange vif d'egui (#FF8F00), au milieu d'une fenêtre où tout
        // autre avertissement est en #DCDCAA. L'erreur était pire — le
        // rouge pur d'egui, 3,44 de contraste sur le fond des fenêtres,
        // contre 5,61 pour celui du thème.
        //
        // Dix sites de `namsdec-re` passent par ces deux couleurs, dans
        // les trois thèmes livrés. Deux lignes ici les redressent tous,
        // là où dix corrections les auraient laissés diverger au
        // prochain module.
        visuals.warn_fg_color = self.warn;
        visuals.error_fg_color = self.bad;

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
    changed: Color32::from_rgb(0xff, 0xd7, 0x00),
    mnemonic: Color32::from_rgb(0x56, 0x9c, 0xd6),
    number: Color32::from_rgb(0xb5, 0xce, 0xa8),
    label: Color32::from_rgb(0x4e, 0xc9, 0xb0),
    comment: Color32::from_rgb(0x6a, 0x99, 0x55),
    register: Color32::from_rgb(0x9c, 0xdc, 0xfe),
    string: Color32::from_rgb(0xce, 0x91, 0x78),
    directive: Color32::from_rgb(0xc5, 0x86, 0xc0),
    folder: Color32::from_rgb(0x7d, 0xd3, 0xfc),
    folder_open: Color32::from_rgb(0x38, 0xbd, 0xf8),
    folder_text: Color32::from_rgb(0x9c, 0xdc, 0xfe),
    header: Color32::from_rgb(0xe0, 0x9a, 0x5a),
    // `editor.selectionBackground` de Dark+, et non `focusBorder` : le
    // bleu vif qui était là ne laissait lire aucun texte posé dessus —
    // 2,07 de contraste, là où le bleu sombre en donne 5,7. Le thème
    // dit « VS Code Dark+ », et c'est bien celle-ci que VS Code emploie.
    selection: Color32::from_rgb(0x26, 0x4f, 0x78),
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
    changed: Color32::from_rgb(0xd9, 0x5f, 0x00),
    mnemonic: Color32::from_rgb(0x00, 0x00, 0xff),
    number: Color32::from_rgb(0x09, 0x86, 0x58),
    label: Color32::from_rgb(0x26, 0x7f, 0x99),
    comment: Color32::from_rgb(0x00, 0x80, 0x00),
    register: Color32::from_rgb(0x00, 0x70, 0xc1),
    string: Color32::from_rgb(0xa3, 0x15, 0x15),
    directive: Color32::from_rgb(0xaf, 0x00, 0xdb),
    folder: Color32::from_rgb(0x1a, 0x85, 0xb8),
    folder_open: Color32::from_rgb(0x00, 0x6b, 0x9c),
    folder_text: Color32::from_rgb(0x0a, 0x53, 0x94),
    header: Color32::from_rgb(0xb5, 0x6a, 0x1f),
    // Même raison, et la couleur de Light+ : un bleu pâle, sur lequel
    // l'encre sombre du thème se lit (11,0), quand le bleu franc qui
    // était là n'en donnait que 2,65.
    selection: Color32::from_rgb(0xad, 0xd6, 0xff),
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
    // Plus clair que le `#565f89` d'origine, et c'est un écart assumé :
    // ce gris-bleu ne donne que 2,4 de contraste sur le fond des
    // panneaux, où il porte le nom de l'onglet, les libellés secondaires
    // et les commentaires du programme. Un commentaire qui explique et
    // qu'on doit deviner ne sert à rien.
    dim: Color32::from_rgb(0x7a, 0x83, 0xad),
    accent: Color32::from_rgb(0x7a, 0xa2, 0xf7),
    good: Color32::from_rgb(0x9e, 0xce, 0x6a),
    warn: Color32::from_rgb(0xe0, 0xaf, 0x68),
    bad: Color32::from_rgb(0xf7, 0x76, 0x8e),
    changed: Color32::from_rgb(0xff, 0x9e, 0x64),
    mnemonic: Color32::from_rgb(0xbb, 0x9a, 0xf7),
    number: Color32::from_rgb(0xff, 0x9e, 0x64),
    label: Color32::from_rgb(0x7d, 0xcf, 0xff),
    // Même valeur que `dim`, et pour la même raison.
    comment: Color32::from_rgb(0x7a, 0x83, 0xad),
    register: Color32::from_rgb(0x2a, 0xc3, 0xde),
    string: Color32::from_rgb(0x9e, 0xce, 0x6a),
    directive: Color32::from_rgb(0xf7, 0x76, 0x8e),
    folder: Color32::from_rgb(0x7d, 0xcf, 0xff),
    folder_open: Color32::from_rgb(0x2a, 0xc3, 0xde),
    folder_text: Color32::from_rgb(0x7d, 0xcf, 0xff),
    header: Color32::from_rgb(0xff, 0x9e, 0x64),
    selection: Color32::from_rgb(0x3d, 0x59, 0xa1),
};

/// Les thèmes fournis avec l'application. Un thème installé ensuite
/// viendra simplement s'ajouter à cette liste.
pub static BUILT_IN: &[&Theme] = &[&VSCODE_DARK, &VSCODE_LIGHT, &TOKYO_NIGHT];

/// À quel point deux couleurs se distinguent à l'œil, sur 0 à ~765.
///
/// Une distance RGB brute ment : deux couleurs de teintes différentes
/// mais de même clarté s'y ressemblent peu, alors qu'à l'écran elles se
/// confondent. Cette approximation-ci (« redmean ») pèse les composantes
/// selon la sensibilité de l'œil, et c'est elle qui a fait voir que le
/// rose d'Abyss ne se détachait pas de son bleu : deux pastels de même
/// clarté, distincts sur le papier, indiscernables du coin de l'œil.
pub fn perceptual_distance(a: Color32, b: Color32) -> f32 {
    let (ar, ag, ab) = (a.r() as f32, a.g() as f32, a.b() as f32);
    let (br, bg, bb) = (b.r() as f32, b.g() as f32, b.b() as f32);
    let red_mean = (ar + br) / 2.0;
    let (dr, dg, db) = (ar - br, ag - bg, ab - bb);
    ((2.0 + red_mean / 256.0) * dr * dr
        + 4.0 * dg * dg
        + (2.0 + (255.0 - red_mean) / 256.0) * db * db)
        .sqrt()
}

/// La clarté d'une couleur, telle que l'œil la reçoit — de 0 (noir) à
/// 1 (blanc).
///
/// Ce n'est pas la moyenne des composantes, et ce n'est pas non plus
/// leur moyenne pondérée : il faut d'abord défaire la correction gamma
/// que l'écran applique. Sans cela le gris moyen `#808080` compte pour
/// la moitié du blanc, alors qu'il n'en renvoie que le cinquième. C'est
/// l'erreur qui faisait choisir du gris clair sur du bleu vif.
pub fn relative_luminance(c: Color32) -> f32 {
    let droit = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * droit(c.r()) + 0.7152 * droit(c.g()) + 0.0722 * droit(c.b())
}

/// Le contraste entre deux couleurs, de 1 (identiques) à 21 (noir et
/// blanc).
///
/// C'est la mesure des recommandations d'accessibilité : 4,5 pour un
/// texte courant, 3 pour un gros titre ou un élément d'interface. En
/// dessous de 3, un texte se devine plus qu'il ne se lit ; en dessous
/// de 2, il disparaît dès que l'écran n'est pas de face.
///
/// Les deux couleurs doivent être opaques : un contraste ne se calcule
/// qu'entre ce qu'on voit, et une encre translucide se compose d'abord
/// avec son fond.
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (x, y) = (relative_luminance(a), relative_luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Les couleurs qui doivent distinguer le code les unes des autres, avec
/// le nom de leur rôle.
///
/// Le texte ordinaire en fait partie : c'est lui que portent les
/// registres, les opérandes, tout ce qui n'est ni mnémonique, ni nombre,
/// ni étiquette, ni commentaire.
pub fn code_colours(theme: &Theme) -> [(&'static str, Color32); 8] {
    [
        ("text", theme.text),
        ("mnemonic", theme.mnemonic),
        ("number", theme.number),
        ("label", theme.label),
        ("comment", theme.comment),
        ("register", theme.register),
        ("string", theme.string),
        ("directive", theme.directive),
    ]
}

/// Les thèmes venus d'un fichier, ajoutés au fil des installations.
///
/// Un `RwLock` et non un `OnceLock` : on peut en installer un pendant que
/// l'application tourne, et la liste doit alors s'allonger sous les yeux
/// de l'utilisateur plutôt qu'au lancement suivant.
static INSTALLED: RwLock<Vec<&'static Theme>> = RwLock::new(Vec::new());

/// Ajoute un thème lu depuis un fichier. Son identifiant doit être libre :
/// deux thèmes qui répondent au même nom rendraient `by_id` — et donc le
/// thème enregistré dans les réglages — imprévisible.
pub fn install(theme: Theme) -> Result<&'static Theme, PluginError> {
    if all().iter().any(|known| known.id == theme.id) {
        return Err(PluginError::DuplicateId(theme.id.to_owned()));
    }
    // Même raison que dans `plugin::parse` : un thème vit aussi longtemps
    // que l'application.
    let theme: &'static Theme = Box::leak(Box::new(theme));
    INSTALLED
        .write()
        .expect("la liste des thèmes installés")
        .push(theme);
    Ok(theme)
}

/// Retire un thème installé de la liste. Rend `false` s'il n'y était pas.
///
/// Les trois thèmes d'origine ne sont pas concernés : ils ne sont pas
/// dans cette liste-là, et rien ne peut les en retirer.
///
/// La palette elle-même reste en mémoire jusqu'à la fermeture — elle a
/// été fuitée à l'installation pour vivre aussi longtemps que
/// l'application, et quelque chose peut encore la désigner. Ce n'est pas
/// une négligence mais le prix, une fois pour toutes, de `&'static`.
pub fn uninstall(id: &str) -> bool {
    let mut installed = INSTALLED.write().expect("la liste des thèmes installés");
    let before = installed.len();
    installed.retain(|theme| theme.id != id);
    installed.len() != before
}

/// Tous les thèmes disponibles : ceux d'origine, puis les installés.
pub fn all() -> Vec<&'static Theme> {
    let mut themes: Vec<&'static Theme> = BUILT_IN.to_vec();
    themes.extend(
        INSTALLED
            .read()
            .expect("la liste des thèmes installés")
            .iter()
            .copied(),
    );
    themes
}

/// Retrouve un thème par identifiant, en retombant sur le thème par défaut
/// si l'identifiant est inconnu — un thème désinstallé ne doit pas empêcher
/// l'application de démarrer.
pub fn by_id(id: &str) -> &'static Theme {
    all()
        .into_iter()
        .find(|theme| theme.id == id)
        .unwrap_or(&VSCODE_DARK)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le seuil à partir duquel une couleur se remarque sans qu'on la
    /// cherche. Tokyo Night est le plus serré des thèmes d'origine
    /// (253) : descendre en dessous, c'est repasser du côté d'Abyss et
    /// de son rose invisible (229).
    const CHANGED_APART_FROM_TEXT: f32 = 240.0;

    /// Deux rôles du code doivent rester distinguables. Le plus serré
    /// des thèmes d'origine est VS Code Dark+, entre son texte et ses
    /// nombres (84).
    pub(crate) const CODE_ROLES_APART: f32 = 80.0;
    /// Et chacun doit se détacher du fond de l'éditeur. Le plus serré
    /// est le commentaire de Tokyo Night (232) — un commentaire a le
    /// droit d'être discret, pas d'être illisible.
    pub(crate) const CODE_OFF_BACKGROUND: f32 = 225.0;

    /// Un thème doit rendre le code lisible *comme du code* : quatre
    /// rôles qui ne se confondent pas, et aucun qui se noie dans le fond.
    ///
    /// C'est ce qui manquait au thème d'exemple : son mnémonique et son
    /// commentaire, deux bleus sombres de même clarté, étaient à 42 l'un
    /// de l'autre. `mov` avait la couleur d'un commentaire, et la ligne
    /// ne se lisait plus.
    pub(crate) fn check_code_colours(theme: &Theme) {
        let roles = code_colours(theme);
        for (name, colour) in roles {
            let off = perceptual_distance(colour, theme.bg);
            assert!(
                off > CODE_OFF_BACKGROUND,
                "{} : « {name} » se noie dans le fond de l'éditeur ({off:.0})",
                theme.id
            );
        }
        for (i, (a, ca)) in roles.iter().enumerate() {
            for (b, cb) in &roles[i + 1..] {
                let apart = perceptual_distance(*ca, *cb);
                assert!(
                    apart > CODE_ROLES_APART,
                    "{} : « {a} » et « {b} » se ressemblent trop ({apart:.0})",
                    theme.id
                );
            }
        }
    }

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

    /// La couleur du changement doit être *vive* : c'est elle qui doit
    /// attraper l'œil au milieu de seize registres figés. On exige donc
    /// qu'elle soit plus saturée et plus contrastée sur son panneau que
    /// l'avertissement, dont le rôle est de se laisser lire plus tard.
    #[test]
    fn the_change_colour_is_livelier_than_the_warning() {
        /// Écart entre la composante la plus forte et la plus faible :
        /// une couleur grise vaut zéro, une couleur pure vaut 255.
        fn saturation(c: Color32) -> i32 {
            let (r, g, b) = (c.r() as i32, c.g() as i32, c.b() as i32);
            r.max(g).max(b) - r.min(g).min(b)
        }
        /// Distance à la couleur du panneau : le contraste, en gros.
        fn distance(a: Color32, b: Color32) -> i32 {
            (a.r() as i32 - b.r() as i32).abs()
                + (a.g() as i32 - b.g() as i32).abs()
                + (a.b() as i32 - b.b() as i32).abs()
        }

        for theme in BUILT_IN {
            assert!(
                saturation(theme.changed) > saturation(theme.warn),
                "{} : le changement doit être plus saturé que l'avertissement",
                theme.id
            );
            assert!(
                distance(theme.changed, theme.panel) > 200,
                "{} : et bien détaché du fond du panneau",
                theme.id
            );
            // Et surtout : détaché du texte ordinaire. C'est là que le
            // rose d'Abyss échouait — même clarté que son bleu, donc
            // invisible sans le chercher, alors que les deux couleurs
            // sont « différentes » sur le papier.
            let from_text = perceptual_distance(theme.changed, theme.text);
            assert!(
                from_text > CHANGED_APART_FROM_TEXT,
                "{} : le changement doit se distinguer du texte ordinaire ({from_text:.0})",
                theme.id
            );
            assert_ne!(
                theme.changed, theme.accent,
                "{} : à ne pas confondre avec RIP, qui porte l'accent",
                theme.id
            );
        }
    }

    /// Installer puis retirer : la liste revient à ce qu'elle était, et
    /// les thèmes d'origine ne sont jamais concernés.
    #[test]
    fn an_installed_theme_can_be_taken_back_out() {
        let file = include_str!("../tests/abyss.json").replace("\"abyss\"", "\"abyss-jetable\"");
        let theme = plugin::parse(&file).expect("thème de test");
        let before = all().len();

        install(theme).expect("installation");
        assert_eq!(all().len(), before + 1);
        assert_eq!(by_id("abyss-jetable").id, "abyss-jetable");

        assert!(uninstall("abyss-jetable"), "il était bien installé");
        assert_eq!(all().len(), before, "la liste revient à son état");
        assert_eq!(
            by_id("abyss-jetable").id,
            VSCODE_DARK.id,
            "et l'identifiant retombe sur le thème par défaut"
        );

        // Retirer deux fois, ou retirer un thème d'origine, ne fait rien.
        assert!(!uninstall("abyss-jetable"));
        assert!(
            !uninstall(VSCODE_DARK.id),
            "un thème d'origine n'est pas retirable"
        );
        assert_eq!(all().len(), before);
    }

    /// Les trois thèmes d'origine tiennent la même exigence que celle
    /// qu'on impose aux thèmes installés.
    #[test]
    fn every_built_in_theme_tells_the_code_apart() {
        for theme in BUILT_IN {
            check_code_colours(theme);
        }
    }

    #[test]
    fn unknown_id_falls_back_instead_of_panicking() {
        assert_eq!(by_id("thème-désinstallé").id, VSCODE_DARK.id);
    }

    /// L'alerte et l'erreur d'egui sont celles du thème.
    ///
    /// Un thème qui ne les pose pas laisse egui peindre son orange et
    /// son rouge par défaut. Ce n'est pas une nuance : `warn_fg_color`
    /// et `error_fg_color` sont lus par dix endroits de `namsdec-re`,
    /// et ils y voisinent avec des textes peints, eux, à la palette du
    /// thème. Le contrôle vaut pour TOUS les thèmes livrés — c'est
    /// justement ce qu'un oubli dans `visuals()` fait perdre d'un coup.
    #[test]
    fn every_theme_paints_its_own_warning_and_its_own_error() {
        for theme in BUILT_IN {
            let visuals = theme.visuals();
            assert_eq!(
                visuals.warn_fg_color, theme.warn,
                "« {} » laisse egui choisir la couleur d'un avertissement",
                theme.name
            );
            assert_eq!(
                visuals.error_fg_color, theme.bad,
                "« {} » laisse egui choisir la couleur d'une erreur",
                theme.name
            );
        }
    }
}
