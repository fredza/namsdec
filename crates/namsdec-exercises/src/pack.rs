//! Les séries d'exercices venues d'ailleurs : un fichier JSON, chargé au
//! démarrage.
//!
//! Comme un thème, une série est de la donnée et rien d'autre : du texte
//! et des programmes en assembleur. Rien n'y est exécuté à l'ouverture —
//! le programme d'un exercice ne tourne que si on le charge dans
//! l'éditeur et qu'on appuie sur Exécuter, comme celui qu'on aurait tapé
//! soi-même.
//!
//! Le format, aussi court que possible :
//!
//! ```json
//! {
//!   "id": "mes-exos",
//!   "name": "Mes exercices",
//!   "exercises": [
//!     {
//!       "id": "echange",
//!       "title": "Échanger deux registres",
//!       "description": { "fr": "Sans passer par un troisième.", "en": "Without a third one." },
//!       "code": "BITS 64\nsection .text\n    xor rax, rbx\n    hlt\n",
//!       "goal": ["rax == 2", "rbx == 1"],
//!       "par": 6,
//!       "level": "practice"
//!     }
//!   ]
//! }
//! ```
//!
//! `level` range l'exercice dans un palier — « discovery », « practice »,
//! « structures », « mastery » —, et le parcours les traverse dans cet
//! ordre.
//!
//! `par` est le budget : le nombre d'instructions exécutées qu'il faut
//! tenir. Facultatif lui aussi — il n'a de sens qu'avec un but.
//!
//! `goal` est facultatif : les conditions — la grammaire des points
//! d'arrêt, **tout en hexadécimal** — que l'état du processeur doit
//! vérifier pour que l'exercice soit réussi. Sans lui, l'exercice se
//! regarde ; avec lui, namsdec dit « atteint » ou « pas encore » au lieu
//! de laisser vérifier à l'œil.
//!
//! Un titre ou un énoncé s'écrit au choix en texte simple — le même dans
//! les deux langues — ou en objet `{ "fr": …, "en": … }`. Traduire est un
//! travail ; ne pas le faire ne doit pas empêcher d'écrire une série.

use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use crate::{Exercise, Level, Pack, Text};

/// `Display` en anglais, comme les autres erreurs des crates métier : la
/// traduction se fait dans l'interface.
#[derive(Debug, Error)]
pub enum PackError {
    #[error("cannot read the exercise file: {0}")]
    Io(#[from] std::io::Error),
    #[error("this is not a valid exercise file: {0}")]
    Format(#[from] serde_json::Error),
    #[error("the identifier must be made of letters, digits or dashes: {0}")]
    BadId(String),
    #[error("the pack has no exercise")]
    Empty,
    #[error("the exercise {0} has no code")]
    NoCode(String),
    #[error("two exercises share the identifier {0}")]
    DuplicateId(String),
}

/// Un texte du fichier : une chaîne, ou un objet par langue.
#[derive(Deserialize)]
#[serde(untagged)]
enum TextField {
    Same(String),
    PerLanguage { fr: String, en: String },
}

impl From<TextField> for Text {
    fn from(field: TextField) -> Self {
        match field {
            TextField::Same(text) => Text::same(&text),
            TextField::PerLanguage { fr, en } => Text { fr, en },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExerciseFile {
    id: String,
    title: TextField,
    description: TextField,
    /// Le programme de départ. Nommé `code` et non `starter_code` :
    /// c'est un fichier qu'un humain écrit à la main.
    code: String,
    /// Le but, en conditions — `["rax == 15"]`. Facultatif : une série
    /// écrite avant que cela existe reste lisible, et un exercice qui
    /// se regarde n'a pas de but à vérifier.
    #[serde(default)]
    goal: Vec<String>,
    /// Le budget d'instructions exécutées. Facultatif.
    #[serde(default)]
    par: Option<u64>,
    /// Le palier : « discovery », « practice », « structures » ou
    /// « mastery ». Facultatif — sans lui, l'exercice est au premier,
    /// ce qui reste vrai d'une série qu'on découvre.
    #[serde(default)]
    level: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackFile {
    id: String,
    name: String,
    exercises: Vec<ExerciseFile>,
}

/// Un identifiant sert de nom de dossier et d'entrée de menu : on s'en
/// tient à ce qui ne surprend personne.
fn check_id(id: &str) -> Result<(), PackError> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PackError::BadId(id.to_owned()));
    }
    Ok(())
}

/// Lit une série depuis un fichier, sans l'installer.
///
/// Sert aussi bien au chargement du démarrage qu'à la vérification d'un
/// fichier qu'on s'apprête à installer : rien n'est copié tant que la
/// lecture n'a pas réussi.
pub fn read(path: impl AsRef<Path>) -> Result<Pack, PackError> {
    let text = std::fs::read_to_string(path)?;
    parse(&text)
}

/// Le même travail, à partir du texte du fichier.
pub fn parse(text: &str) -> Result<Pack, PackError> {
    let file: PackFile = serde_json::from_str(text)?;
    check_id(&file.id)?;
    if file.exercises.is_empty() {
        return Err(PackError::Empty);
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut exercises = Vec::with_capacity(file.exercises.len());
    for exercise in file.exercises {
        check_id(&exercise.id)?;
        if exercise.code.trim().is_empty() {
            return Err(PackError::NoCode(exercise.id));
        }
        if !seen.insert(exercise.id.clone()) {
            return Err(PackError::DuplicateId(exercise.id));
        }
        exercises.push(Exercise {
            id: exercise.id,
            title: exercise.title.into(),
            description: exercise.description.into(),
            starter_code: exercise.code,
            goal: exercise.goal,
            par: exercise.par,
            level: exercise
                .level
                .as_deref()
                .map(Level::by_id)
                .unwrap_or(Level::Discovery),
        });
    }

    Ok(Pack {
        id: file.id,
        name: file.name,
        exercises,
    })
}

#[cfg(test)]
mod goal_tests {
    use super::*;

    /// Le but se lit depuis le fichier, et son absence n'est pas une
    /// erreur : une série écrite avant que cela existe reste lisible.
    #[test]
    fn a_pack_may_or_may_not_carry_goals() {
        let avec = parse(
            r#"{"id":"s","name":"S","exercises":[
                 {"id":"e","title":"T","description":"D","code":"hlt\n",
                  "goal":["rax == 1","rbx == 2"]}]}"#,
        )
        .expect("série lisible");
        assert_eq!(avec.exercises[0].goal, ["rax == 1", "rbx == 2"]);

        let sans = parse(
            r#"{"id":"s","name":"S","exercises":[
                 {"id":"e","title":"T","description":"D","code":"hlt\n"}]}"#,
        )
        .expect("série lisible sans but");
        assert!(sans.exercises[0].goal.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "id": "mes-exos",
        "name": "Mes exercices",
        "exercises": [
            {
                "id": "echange",
                "title": "Échanger deux registres",
                "description": { "fr": "Sans troisième registre.", "en": "Without a third register." },
                "code": "BITS 64\nsection .text\n    hlt\n"
            }
        ]
    }"#;

    #[test]
    fn a_pack_reads_both_shapes_of_text() {
        let pack = parse(SAMPLE).expect("série valide");
        assert_eq!(pack.id, "mes-exos");
        assert_eq!(pack.name, "Mes exercices");
        assert_eq!(pack.exercises.len(), 1);
        let exercise = &pack.exercises[0];
        // Un titre écrit une seule fois vaut pour les deux langues.
        assert_eq!(exercise.title.get("fr"), "Échanger deux registres");
        assert_eq!(exercise.title.get("en"), "Échanger deux registres");
        // Un énoncé traduit garde ses deux versions.
        assert_eq!(exercise.description.get("en"), "Without a third register.");
        assert!(exercise.starter_code.contains("hlt"));
    }

    #[test]
    fn a_pack_that_would_show_nothing_is_refused() {
        let empty = SAMPLE.replace(r#""exercises": ["#, r#""exercises": [], "ignore": ["#);
        assert!(matches!(parse(&empty), Err(PackError::Format(_))));

        let no_exercise = r#"{"id":"x","name":"X","exercises":[]}"#;
        assert!(matches!(parse(no_exercise), Err(PackError::Empty)));

        let no_code = SAMPLE.replace(r#"BITS 64\nsection .text\n    hlt\n"#, "   ");
        assert!(matches!(parse(&no_code), Err(PackError::NoCode(_))));
    }

    #[test]
    fn identifiers_are_checked_because_they_become_file_names() {
        let bad = SAMPLE.replace("mes-exos", "../ailleurs");
        assert!(matches!(parse(&bad), Err(PackError::BadId(_))));

        let twice = SAMPLE.replace(
            r#""exercises": ["#,
            r#""exercises": [
            {"id":"echange","title":"A","description":"B","code":"BITS 64\nhlt\n"},"#,
        );
        assert!(matches!(parse(&twice), Err(PackError::DuplicateId(_))));
    }
}
