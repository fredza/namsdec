//! Les exercices : des programmes à trous, avec ce qu'on cherche à y
//! comprendre.
//!
//! Module à part, et chargé seulement si on le demande. Quelqu'un qui
//! écrit son propre code n'a pas besoin qu'on lui propose des devoirs :
//! l'application ne montre rien de tout cela tant que la case n'est pas
//! cochée dans les préférences, et cette crate n'a alors aucun effet sur
//! le reste.
//!
//! Pas de correction automatique : un exercice pose un but en clair — « à
//! l'arrêt, RAX doit valoir 15 » — et c'est le bandeau des registres qui
//! répond. Vérifier à la place de l'utilisateur lui retirerait le geste
//! qu'il est venu apprendre.

/// Un texte dans les deux langues de l'interface.
///
/// Comme pour les fiches d'instructions : l'énoncé d'un exercice *est*
/// son texte, il ne peut pas être traduit à l'affichage par quelqu'un
/// d'autre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    pub fr: &'static str,
    pub en: &'static str,
}

impl Text {
    pub fn get(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.fr
        } else {
            self.en
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Exercise {
    /// Identifiant stable, indépendant de la langue et du titre.
    pub id: &'static str,
    pub title: Text,
    /// Ce qu'on cherche à comprendre, et le but à atteindre.
    pub description: Text,
    /// Le programme de départ, à charger dans l'éditeur.
    pub starter_code: &'static str,
}

/// Les exercices livrés avec l'application, dans l'ordre où les faire.
pub fn builtin_exercises() -> Vec<Exercise> {
    vec![
        Exercise {
            id: "sum_loop",
            title: Text {
                fr: "Somme d'une boucle",
                en: "Sum of a loop",
            },
            description: Text {
                fr: "Additionner 5+4+3+2+1 avec une boucle add/dec/jnz. À l'arrêt, RAX doit valoir 15.",
                en: "Add 5+4+3+2+1 with an add/dec/jnz loop. At the halt, RAX must hold 15.",
            },
            starter_code: include_str!("../exercises/sum_loop.asm"),
        },
        Exercise {
            id: "register_widths",
            title: Text {
                fr: "Largeurs de registres",
                en: "Register widths",
            },
            description: Text {
                fr: "Écrire un registre 32 bits met à zéro le haut de RAX ; écrire 16 ou 8 bits le préserve. Avancez pas à pas et regardez RAX dans le bandeau.",
                en: "Writing a 32-bit register zeroes the upper half of RAX; writing 16 or 8 bits preserves it. Step through it and watch RAX in the banner.",
            },
            starter_code: include_str!("../exercises/register_widths.asm"),
        },
        Exercise {
            id: "max_of_two",
            title: Text {
                fr: "Maximum de deux nombres",
                en: "Maximum of two numbers",
            },
            description: Text {
                fr: "Comparer deux valeurs avec cmp/jge et garder la plus grande. Essayez ensuite de refaire la même chose avec cmovg, sans branchement.",
                en: "Compare two values with cmp/jge and keep the larger. Then try the same thing with cmovg, without a branch.",
            },
            starter_code: include_str!("../exercises/max_of_two.asm"),
        },
    ]
}

/// L'exercice qui porte cet identifiant.
pub fn by_id(id: &str) -> Option<Exercise> {
    builtin_exercises().into_iter().find(|e| e.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_exercise_is_complete_and_assemblable_looking() {
        let exercises = builtin_exercises();
        assert_eq!(exercises.len(), 3);
        for exercise in &exercises {
            assert!(!exercise.id.is_empty());
            assert!(!exercise.title.fr.is_empty() && !exercise.title.en.is_empty());
            assert!(!exercise.description.fr.is_empty());
            assert!(!exercise.description.en.is_empty());
            // Un exercice est un programme, pas un extrait : il doit
            // pouvoir être assemblé tel quel.
            assert!(exercise.starter_code.contains("BITS 64"));
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
    fn identifiers_are_unique_and_findable() {
        let mut seen = std::collections::BTreeSet::new();
        for exercise in builtin_exercises() {
            assert!(seen.insert(exercise.id), "{} en double", exercise.id);
            assert_eq!(by_id(exercise.id).map(|e| e.id), Some(exercise.id));
        }
        assert!(by_id("n-existe-pas").is_none());
    }
}
