//! La documentation des instructions que namsdec sait exécuter.
//!
//! Une fiche par *famille* plutôt qu'une par mnémonique : `je`, `jz`,
//! `jne` et les trente autres sauts conditionnels ne diffèrent que par la
//! condition, et trente fiches presque identiques se liraient moins bien
//! qu'une seule qui le dit.
//!
//! Cette table est aussi la liste de ce que l'interpréteur connaît :
//! l'autocomplétion la lit, plutôt que d'entretenir sa propre liste à
//! côté. Une instruction ajoutée au moteur sans fiche ne sera donc pas
//! proposée — ce qui vaut mieux que de la proposer sans savoir la
//! décrire.

mod table;

use std::collections::BTreeMap;
use std::sync::OnceLock;

pub use table::ENTRIES;

/// Un texte dans les deux langues de l'interface. Contrairement aux
/// messages d'erreur — anglais dans les crates métier, traduits par
/// l'interface —, une fiche *est* son texte : elle ne peut pas être
/// « traduite » à l'affichage, il faut l'avoir écrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    pub fr: &'static str,
    pub en: &'static str,
}

impl Text {
    /// Le texte dans la langue demandée, désignée par son code ISO.
    pub fn get(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.fr
        } else {
            self.en
        }
    }
}

/// Ce que la fiche décrit.
///
/// Apprendre l'assembleur, c'est apprendre deux langages à la fois :
/// celui du processeur — les instructions — et celui de l'assembleur, qui
/// n'exécute rien mais décide de ce qui sera écrit dans le fichier. Les
/// confondre est la première difficulté : `mov` s'exécute, `db` non.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Une instruction du processeur.
    Instruction,
    /// Une directive de NASM : elle produit des octets ou range le
    /// programme, elle ne s'exécute pas.
    Directive,
    /// Le préprocesseur : ce qui est remplacé *avant* l'assemblage.
    Preprocessor,
    /// Une notion : ce qu'il faut avoir compris pour lire les autres
    /// fiches. Un registre, une section, la pile, un appel système —
    /// des mots qu'aucune instruction ne définit et que tout le monde
    /// emploie.
    Concept,
}

/// La fiche d'une famille d'instructions, d'une directive ou d'une
/// construction du préprocesseur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub kind: Kind,
    /// Les mnémoniques que la fiche couvre, en minuscules.
    pub names: &'static [&'static str],
    pub title: Text,
    pub body: Text,
    /// Les indicateurs que l'instruction écrit, séparés par des espaces
    /// (« CF ZF »), ou une chaîne vide quand elle n'en touche aucun.
    /// Sans mot autour : c'est l'interface qui fait la phrase.
    pub flags: &'static str,
    /// Une ou deux lignes de NASM, telles qu'on les écrirait.
    pub example: &'static str,
}

/// Toutes les fiches, dans l'ordre où elles sont écrites — celui d'un
/// manuel : les familles se suivent.
///
/// Rendues pour que l'interface puisse en faire un index consultable :
/// une fiche qu'on ne peut lire qu'en écrivant l'instruction dans son
/// code n'est pas une documentation, c'est une infobulle.
pub fn all() -> &'static [Entry] {
    ENTRIES
}

/// Les fiches vers lesquelles celle-ci renvoie.
///
/// Rien n'est saisi à la main : une fiche cite déjà entre accents
/// graves les mots dont elle parle — `cmp`, `RSP`, `%define` — et ceux
/// qui ont une fiche en deviennent des renvois. Écrire la liste à part
/// la condamnerait à vieillir ; la déduire du texte la tient à jour.
///
/// ```
/// let cmp = namsdec_doc::lookup("cmp").unwrap();
/// let renvois = namsdec_doc::related(cmp, "fr");
/// // `ja`/`jb` mènent à la fiche des sauts conditionnels, citée une
/// // seule fois même si le texte en nomme quatre variantes.
/// assert!(renvois.iter().any(|e| e.names.contains(&"ja")));
/// // Une fiche ne se renvoie jamais à elle-même.
/// assert!(!renvois.iter().any(|e| std::ptr::eq(*e, cmp)));
/// ```
pub fn related(entry: &'static Entry, lang: &str) -> Vec<&'static Entry> {
    let mut vus: Vec<&'static Entry> = Vec::new();
    let texte = entry.body.get(lang);
    for (i, morceau) in texte.split('`').enumerate() {
        // Un texte entre accents graves occupe les tranches impaires.
        if i % 2 == 0 {
            continue;
        }
        // `cmp rax, rbx` cite une instruction : le premier mot suffit.
        let mot = morceau.split_whitespace().next().unwrap_or(morceau);
        let mot = mot.trim_end_matches([',', '.', ':', ';']);
        let Some(autre) = lookup(mot) else { continue };
        if std::ptr::eq(autre, entry) || vus.iter().any(|v| std::ptr::eq(*v, autre)) {
            continue;
        }
        vus.push(autre);
    }
    vus
}

/// Le manuel entier en Markdown, dans une langue.
///
/// Ce que namsdec explique n'a pas à rester dans namsdec : un fichier
/// qu'on lit sans lui, qu'on imprime, qu'on distribue à une classe. Les
/// fiches sortent dans l'ordre de la table, groupées par famille.
///
/// ```
/// let manuel = namsdec_doc::markdown("fr");
/// assert!(manuel.starts_with("# "));
/// assert!(manuel.contains("## Les notions"));
/// ```
pub fn markdown(lang: &str) -> String {
    let fr = lang == "fr";
    let mut out = String::new();
    out.push_str(if fr {
        "# Le manuel de namsdec

"
    } else {
        "# The namsdec manual

"
    });
    out.push_str(&if fr {
        format!(
            "{} fiches : les instructions que namsdec exécute, les              directives de NASM, le préprocesseur, et les notions que              tout le reste suppose.
",
            count()
        )
    } else {
        format!(
            "{} sheets: the instructions namsdec runs, NASM's              directives, the preprocessor, and the concepts everything              else takes for granted.
",
            count()
        )
    });

    for (kind, titre) in [
        (Kind::Concept, if fr { "Les notions" } else { "Concepts" }),
        (
            Kind::Instruction,
            if fr { "Les instructions" } else { "Instructions" },
        ),
        (
            Kind::Directive,
            if fr {
                "Les directives de NASM"
            } else {
                "NASM directives"
            },
        ),
        (
            Kind::Preprocessor,
            if fr {
                "Le préprocesseur"
            } else {
                "The preprocessor"
            },
        ),
    ] {
        out.push_str(&format!("\n## {titre}\n"));
        for entry in ENTRIES.iter().filter(|e| e.kind == kind) {
            out.push_str(&format!(
                "\n### {} — {}\n\n",
                entry.names[0],
                entry.title.get(lang)
            ));
            if entry.names.len() > 1 {
                out.push_str(&format!(
                    "*{} {}*\n\n",
                    if fr { "Aussi :" } else { "Also:" },
                    entry.names[1..].join(", ")
                ));
            }
            out.push_str(entry.body.get(lang));
            out.push('\n');
            if kind == Kind::Instruction {
                out.push_str(&format!(
                    "\n{} {}\n",
                    if fr {
                        "Indicateurs écrits :"
                    } else {
                        "Flags written:"
                    },
                    if entry.flags.is_empty() {
                        if fr { "aucun" } else { "none" }
                    } else {
                        entry.flags
                    }
                ));
            }
            out.push_str(&format!("\n```nasm\n{}\n```\n", entry.example));
        }
    }
    out
}

/// Combien de fiches namsdec porte, instructions et directives
/// confondues. Le README l'annonce ; un test le compare.
pub fn count() -> usize {
    ENTRIES.len()
}

/// La fiche qui décrit ce mnémonique, quelle que soit sa casse.
pub fn lookup(mnemonic: &str) -> Option<&'static Entry> {
    let lower = mnemonic.to_ascii_lowercase();
    index().get(lower.as_str()).copied()
}

/// Tous les mnémoniques documentés, triés — c'est-à-dire tout ce que
/// namsdec sait exécuter et nommer.
///
/// Les instructions seulement : une directive n'a rien à faire dans une
/// liste de propositions en tête d'instruction, et l'autocomplétion en
/// propose déjà par ailleurs, au bon endroit.
pub fn mnemonics() -> &'static [&'static str] {
    static ALL: OnceLock<Vec<&'static str>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut names: Vec<&'static str> = ENTRIES
            .iter()
            .filter(|entry| entry.kind == Kind::Instruction)
            .flat_map(|entry| entry.names.iter().copied())
            .collect();
        names.sort_unstable();
        names
    })
}

fn index() -> &'static BTreeMap<&'static str, &'static Entry> {
    static INDEX: OnceLock<BTreeMap<&'static str, &'static Entry>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut map = BTreeMap::new();
        for entry in ENTRIES {
            for name in entry.names {
                map.insert(*name, entry);
            }
        }
        map
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un mnémonique ne peut pas être décrit deux fois : la seconde fiche
    /// masquerait la première sans que rien ne le dise.
    #[test]
    fn no_mnemonic_is_described_twice() {
        let mut seen = std::collections::BTreeSet::new();
        for entry in ENTRIES {
            for name in entry.names {
                assert!(seen.insert(*name), "« {name} » est décrit deux fois");
            }
        }
    }

    /// Une fiche sans nom, sans titre ou sans exemple n'aide personne.
    #[test]
    fn every_entry_is_complete() {
        for entry in ENTRIES {
            let first = entry.names.first().expect("une fiche sans mnémonique");
            assert!(!entry.title.fr.is_empty(), "{first} : titre français vide");
            assert!(!entry.title.en.is_empty(), "{first} : titre anglais vide");
            assert!(!entry.body.fr.is_empty(), "{first} : texte français vide");
            assert!(!entry.body.en.is_empty(), "{first} : texte anglais vide");
            assert!(!entry.example.is_empty(), "{first} : pas d'exemple");
            for name in entry.names {
                assert_eq!(
                    *name,
                    name.to_ascii_lowercase(),
                    "les mnémoniques s'écrivent en minuscules"
                );
            }
        }
    }

    #[test]
    fn a_mnemonic_is_found_whatever_its_case() {
        let entry = lookup("MOV").expect("mov doit être documentée");
        assert!(entry.names.contains(&"mov"));
        assert_eq!(lookup("mov"), Some(entry));
        assert_eq!(lookup("ceci-n-existe-pas"), None);
    }

    /// La liste que lit l'autocomplétion.
    #[test]
    fn the_mnemonic_list_is_sorted_and_covers_the_basics() {
        let all = mnemonics();
        assert!(all.windows(2).all(|pair| pair[0] < pair[1]), "triée");
        for expected in ["mov", "syscall", "addsd", "vaddps", "fld", "rep"] {
            assert!(all.contains(&expected), "« {expected} » manque");
        }
    }
}
