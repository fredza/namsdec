//! Les thèmes venus d'ailleurs : un fichier JSON, chargé au démarrage.
//!
//! Un thème est une palette et rien d'autre — aucun code, aucune
//! permission, rien à exécuter. C'est ce qui permet d'en accepter un
//! depuis le disque sans se demander ce qu'il va faire de la machine :
//! au pire, il est illisible, et il est refusé en le disant.
//!
//! Le format reprend les champs de [`Theme`] un pour un, en couleurs
//! écrites `"#rrggbb"`. Tous sont obligatoires : un thème à moitié défini
//! donnerait une interface à moitié lisible, et mieux vaut le refuser à
//! l'installation que le découvrir à l'usage.

use std::path::Path;

use egui::Color32;
use serde::Deserialize;
use thiserror::Error;

use crate::Theme;

/// `Display` en anglais, comme les autres erreurs des crates métier : la
/// traduction se fait dans l'interface.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("cannot read the theme file: {0}")]
    Io(#[from] std::io::Error),
    #[error("this is not a valid theme file: {0}")]
    Format(#[from] serde_json::Error),
    #[error("{field} is not a colour like \"#1e1e1e\": {value}")]
    Colour { field: &'static str, value: String },
    #[error("the identifier {0} is already taken")]
    DuplicateId(String),
    #[error("the identifier must be made of letters, digits or dashes: {0}")]
    BadId(String),
}

/// Le fichier tel qu'il est écrit. Les couleurs y sont du texte ; elles
/// deviennent des `Color32` à la relecture, avec le nom du champ fautif
/// quand l'une d'elles est mal écrite.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    id: String,
    name: String,
    dark: bool,
    bg: String,
    panel: String,
    surface: String,
    text: String,
    dim: String,
    accent: String,
    good: String,
    warn: String,
    bad: String,
    changed: String,
    mnemonic: String,
    number: String,
    label: String,
    comment: String,
    // Ces trois rôles sont arrivés après le format : un thème écrit
    // avant eux reste valide, et ils reprennent la couleur la plus
    // proche de celles qu'il donne. C'est la seule dérivation
    // automatique de tout le crate, et elle n'existe que pour ne pas
    // casser les thèmes déjà installés.
    #[serde(default)]
    register: Option<String>,
    #[serde(default)]
    string: Option<String>,
    #[serde(default)]
    directive: Option<String>,
    folder: String,
    folder_open: String,
    folder_text: String,
    header: String,
    selection: String,
}

/// Lit un thème depuis un fichier, sans l'installer.
///
/// Sert aussi bien au chargement du démarrage qu'à la vérification d'un
/// fichier qu'on s'apprête à installer : rien n'est copié ni enregistré
/// tant que la lecture n'a pas réussi.
pub fn read(path: impl AsRef<Path>) -> Result<Theme, PluginError> {
    let text = std::fs::read_to_string(path)?;
    parse(&text)
}

/// Le même travail, à partir du texte du fichier.
pub fn parse(text: &str) -> Result<Theme, PluginError> {
    let file: ThemeFile = serde_json::from_str(text)?;
    if file.id.is_empty()
        || !file
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PluginError::BadId(file.id));
    }

    // `Theme` porte des `&'static str` : un thème vit aussi longtemps que
    // l'application, et il n'y en a qu'une poignée. Les fuir une fois est
    // plus honnête qu'un `Cow` ou un `Arc` traînés partout dans le rendu
    // pour un texte qui ne sera jamais libéré.
    Ok(Theme {
        id: file.id.leak(),
        name: file.name.leak(),
        dark: file.dark,
        bg: colour("bg", &file.bg)?,
        panel: colour("panel", &file.panel)?,
        surface: colour("surface", &file.surface)?,
        text: colour("text", &file.text)?,
        dim: colour("dim", &file.dim)?,
        accent: colour("accent", &file.accent)?,
        good: colour("good", &file.good)?,
        warn: colour("warn", &file.warn)?,
        bad: colour("bad", &file.bad)?,
        changed: colour("changed", &file.changed)?,
        mnemonic: colour("mnemonic", &file.mnemonic)?,
        number: colour("number", &file.number)?,
        label: colour("label", &file.label)?,
        comment: colour("comment", &file.comment)?,
        register: match &file.register {
            Some(value) => colour("register", value)?,
            None => colour("text", &file.text)?,
        },
        string: match &file.string {
            Some(value) => colour("string", value)?,
            None => colour("number", &file.number)?,
        },
        directive: match &file.directive {
            Some(value) => colour("directive", value)?,
            None => colour("mnemonic", &file.mnemonic)?,
        },
        folder: colour("folder", &file.folder)?,
        folder_open: colour("folder_open", &file.folder_open)?,
        folder_text: colour("folder_text", &file.folder_text)?,
        header: colour("header", &file.header)?,
        selection: colour("selection", &file.selection)?,
    })
}

/// `"#rrggbb"` — le dièse est facultatif, la casse indifférente.
fn colour(field: &'static str, value: &str) -> Result<Color32, PluginError> {
    let digits = value.trim().trim_start_matches('#');
    let bad = || PluginError::Colour {
        field,
        value: value.to_owned(),
    };
    if digits.len() != 6 {
        return Err(bad());
    }
    let component =
        |from: usize| u8::from_str_radix(&digits[from..from + 2], 16).map_err(|_| bad());
    Ok(Color32::from_rgb(
        component(0)?,
        component(2)?,
        component(4)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABYSS: &str = include_str!("../tests/abyss.json");

    #[test]
    fn a_complete_file_becomes_a_theme() {
        let theme = parse(ABYSS).expect("le thème d'exemple doit se lire");
        assert_eq!(theme.id, "abyss");
        assert_eq!(theme.bg, Color32::from_rgb(0x00, 0x0c, 0x18));
        assert!(theme.dark);
    }

    /// Le thème d'exemple doit passer la même exigence que les thèmes
    /// d'origine : sa couleur de changement doit se détacher du texte
    /// ordinaire, sans quoi un registre modifié passe inaperçu.
    ///
    /// C'est le défaut qu'il avait : un rose de la même clarté que son
    /// bleu, « différent » sur le papier et invisible à l'écran.
    #[test]
    fn the_sample_theme_makes_a_change_stand_out() {
        let theme = parse(ABYSS).unwrap();
        let from_text = crate::perceptual_distance(theme.changed, theme.text);
        assert!(
            from_text > 240.0,
            "le changement doit se distinguer du texte ({from_text:.0})"
        );
    }

    /// Le thème d'exemple doit rendre le code lisible comme du code —
    /// la même exigence que les thèmes d'origine.
    #[test]
    fn the_sample_theme_tells_the_code_apart() {
        crate::tests::check_code_colours(&parse(ABYSS).unwrap());
    }

    /// Le dièse est facultatif et la casse indifférente : un thème
    /// recopié depuis un site web ne doit pas être refusé pour un détail
    /// d'écriture.
    #[test]
    fn colours_are_read_leniently_but_checked() {
        assert_eq!(
            colour("bg", "#1E1E1E").unwrap(),
            Color32::from_rgb(30, 30, 30)
        );
        assert_eq!(
            colour("bg", "1e1e1e").unwrap(),
            Color32::from_rgb(30, 30, 30)
        );
        assert_eq!(
            colour("bg", " #1e1e1e ").unwrap(),
            Color32::from_rgb(30, 30, 30)
        );

        // Mais ce qui n'est pas une couleur est refusé en nommant le
        // champ : c'est ce qui rend le message utile.
        let err = colour("accent", "bleu").unwrap_err();
        assert!(
            matches!(
                &err,
                PluginError::Colour {
                    field: "accent",
                    ..
                }
            ),
            "{err}"
        );
        assert!(colour("bg", "#12345").is_err(), "trop court");
        assert!(colour("bg", "#gggggg").is_err(), "pas hexadécimal");
    }

    /// Un champ oublié n'est pas comblé en silence : une interface à
    /// moitié lisible est pire qu'un refus franc à l'installation.
    #[test]
    fn a_missing_field_is_refused() {
        let incomplete = ABYSS.replace("\"accent\"", "\"accent_oublie\"");
        let err = parse(&incomplete).unwrap_err();
        assert!(matches!(err, PluginError::Format(_)), "{err}");
    }

    #[test]
    fn an_identifier_must_be_usable_as_a_key() {
        let odd = ABYSS.replace("\"abyss\"", "\"a byss/../\"");
        assert!(matches!(parse(&odd), Err(PluginError::BadId(_))));
    }
}
