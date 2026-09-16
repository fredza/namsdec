//! Les exercices : des programmes à trous, avec ce qu'on cherche à y
//! comprendre.
//!
//! Module à part, et chargé seulement si on le demande. Quelqu'un qui
//! écrit son propre code n'a pas besoin qu'on lui propose des devoirs :
//! l'application ne montre rien de tout cela tant que la case n'est pas
//! cochée dans les préférences, et cette crate n'a alors aucun effet sur
//! le reste.
//!
//! Deux sources : la série livrée avec l'application, et les séries
//! installées depuis le disque — un fichier JSON, comme un thème (voir
//! [`pack`]). Les deux donnent le même type, pour que l'interface n'ait
//! pas à savoir d'où vient ce qu'elle affiche.
//!
//! Pas de correction automatique : un exercice pose un but en clair — « à
//! l'arrêt, RAX doit valoir 15 » — et c'est le bandeau des registres qui
//! répond. Vérifier à la place de l'utilisateur lui retirerait le geste
//! qu'il est venu apprendre.

pub mod pack;

/// Un texte dans les deux langues de l'interface.
///
/// Comme pour les fiches d'instructions : l'énoncé d'un exercice *est*
/// son texte, il ne peut pas être traduit à l'affichage par quelqu'un
/// d'autre. Une série écrite dans une seule langue donne le même texte
/// des deux côtés — c'est à celui qui l'écrit de décider, pas à nous de
/// refuser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub fr: String,
    pub en: String,
}

impl Text {
    /// Le même texte quelle que soit la langue.
    pub fn same(text: &str) -> Self {
        Self {
            fr: text.to_owned(),
            en: text.to_owned(),
        }
    }

    pub fn get(&self, lang: &str) -> &str {
        if lang.starts_with("fr") {
            &self.fr
        } else {
            &self.en
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    /// Identifiant stable, indépendant de la langue et du titre.
    pub id: String,
    pub title: Text,
    /// Ce qu'on cherche à comprendre, et le but à atteindre.
    pub description: Text,
    /// Le programme de départ, à charger dans l'éditeur.
    pub starter_code: String,
}

/// Une série d'exercices : celle qui est livrée, ou une installée depuis
/// le disque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub exercises: Vec<Exercise>,
}

/// L'identifiant de la série livrée avec l'application.
pub const BUILTIN_ID: &str = "namsdec";

/// Les exercices livrés avec l'application, dans l'ordre où les faire.
pub fn builtin_pack() -> Pack {
    Pack {
        id: BUILTIN_ID.to_owned(),
        name: "namsdec".to_owned(),
        exercises: vec![
            Exercise {
                id: "sum_loop".to_owned(),
                title: Text {
                    fr: "Somme d'une boucle".to_owned(),
                    en: "Sum of a loop".to_owned(),
                },
                description: Text {
                    fr: "Additionner 5+4+3+2+1 avec une boucle add/dec/jnz. À l'arrêt, RAX doit valoir 15.".to_owned(),
                    en: "Add 5+4+3+2+1 with an add/dec/jnz loop. At the halt, RAX must hold 15.".to_owned(),
                },
                starter_code: include_str!("../exercises/sum_loop.asm").to_owned(),
            },
            Exercise {
                id: "register_widths".to_owned(),
                title: Text {
                    fr: "Largeurs de registres".to_owned(),
                    en: "Register widths".to_owned(),
                },
                description: Text {
                    fr: "Écrire un registre 32 bits met à zéro le haut de RAX ; écrire 16 ou 8 bits le préserve. Avancez pas à pas et regardez RAX dans le bandeau.".to_owned(),
                    en: "Writing a 32-bit register zeroes the upper half of RAX; writing 16 or 8 bits preserves it. Step through it and watch RAX in the banner.".to_owned(),
                },
                starter_code: include_str!("../exercises/register_widths.asm").to_owned(),
            },
            Exercise {
                id: "max_of_two".to_owned(),
                title: Text {
                    fr: "Maximum de deux nombres".to_owned(),
                    en: "Maximum of two numbers".to_owned(),
                },
                description: Text {
                    fr: "Comparer deux valeurs avec cmp/jge et garder la plus grande. Essayez ensuite de refaire la même chose avec cmovg, sans branchement.".to_owned(),
                    en: "Compare two values with cmp/jge and keep the larger. Then try the same thing with cmovg, without a branch.".to_owned(),
                },
                starter_code: include_str!("../exercises/max_of_two.asm").to_owned(),
            },
        ],
    }
}

impl Pack {
    /// L'exercice qui porte cet identifiant.
    pub fn by_id(&self, id: &str) -> Option<&Exercise> {
        self.exercises.iter().find(|e| e.id == id)
    }
}

/// Cherche un exercice dans plusieurs séries à la fois, par son
/// identifiant *complet* — « série/exercice ».
///
/// Complet parce que deux séries peuvent nommer leur premier exercice de
/// la même façon, et que celui qu'on a cliqué est celui qu'on veut.
pub fn find<'a>(packs: &'a [Pack], full_id: &str) -> Option<&'a Exercise> {
    let (pack_id, exercise_id) = full_id.split_once('/')?;
    packs.iter().find(|p| p.id == pack_id)?.by_id(exercise_id)
}

/// L'identifiant complet d'un exercice dans sa série.
pub fn full_id(pack: &Pack, exercise: &Exercise) -> String {
    format!("{}/{}", pack.id, exercise.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_exercise_is_complete_and_stops() {
        let pack = builtin_pack();
        assert_eq!(pack.exercises.len(), 3);
        for exercise in &pack.exercises {
            assert!(!exercise.id.is_empty());
            assert!(!exercise.title.fr.is_empty() && !exercise.title.en.is_empty());
            assert!(!exercise.description.fr.is_empty());
            assert!(!exercise.description.en.is_empty());
            // Un exercice est un programme complet, pas un extrait : il
            // déclare son point d'entrée, sans quoi l'éditeur de liens
            // en choisit un à sa place — et le programme démarre là où
            // personne ne l'a demandé.
            assert!(
                exercise.starter_code.contains("global _start"),
                "{} ne déclare pas son point d'entrée",
                exercise.id
            );
            assert!(exercise.starter_code.contains("_start:"));
            // Et il doit s'arrêter, sinon « Exécuter » ne rend jamais la
            // main.
            assert!(
                exercise.starter_code.contains("hlt") || exercise.starter_code.contains("exit"),
                "{} ne s'arrête pas",
                exercise.id
            );
        }
    }

    #[test]
    fn an_exercise_is_found_by_its_full_identifier() {
        let packs = vec![builtin_pack()];
        let found = find(&packs, "namsdec/sum_loop").expect("l'exercice existe");
        assert_eq!(found.id, "sum_loop");
        assert_eq!(full_id(&packs[0], found), "namsdec/sum_loop");
        // Le nom seul ne suffit pas : deux séries peuvent le porter.
        assert!(find(&packs, "sum_loop").is_none());
        assert!(find(&packs, "autre/sum_loop").is_none());
    }
}
