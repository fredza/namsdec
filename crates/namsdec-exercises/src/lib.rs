//! Bibliothèque d'exercices NASM embarqués. Pas de logique de correction —
//! juste de quoi peupler un menu "Exercices" qui pré-remplit l'éditeur.

pub struct Exercise {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub starter_code: &'static str,
}

pub fn builtin_exercises() -> Vec<Exercise> {
    vec![
        Exercise {
            id: "sum_loop",
            title: "Somme d'une boucle",
            description: "Additionner 5+4+3+2+1 avec une boucle add/dec/jnz.",
            starter_code: include_str!("../exercises/sum_loop.asm"),
        },
        Exercise {
            id: "register_widths",
            title: "Largeurs de registres",
            description:
                "Observer la différence entre écrire un registre 32 bits (qui remet à zéro le haut de RAX) et un registre 16 bits (qui le préserve).",
            starter_code: include_str!("../exercises/register_widths.asm"),
        },
        Exercise {
            id: "max_of_two",
            title: "Maximum de deux nombres",
            description: "Comparer deux valeurs avec cmp/jge et garder la plus grande.",
            starter_code: include_str!("../exercises/max_of_two.asm"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_three_builtin_exercises_with_non_empty_source() {
        let exercises = builtin_exercises();
        assert_eq!(exercises.len(), 3);
        for ex in &exercises {
            assert!(!ex.id.is_empty());
            assert!(!ex.title.is_empty());
            assert!(ex.starter_code.contains("BITS 64"));
        }
    }
}
