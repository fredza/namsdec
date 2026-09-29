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

/// La version de ce module, telle que Cargo la connaît.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
            if fr {
                "Les instructions"
            } else {
                "Instructions"
            },
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

/// Une fiche trouvée par une recherche, et le nom qui a correspondu.
///
/// Le nom compte autant que la fiche : une fiche couvre souvent
/// plusieurs mnémoniques — `push` et `pop` partagent la leur — et
/// n'afficher que le premier répondait « push » à qui cherchait « pop ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found {
    pub entry: &'static Entry,
    /// Le nom sous lequel la fiche a été trouvée, tel qu'on l'a cherché.
    pub name: &'static str,
    /// D'autant plus petit que la correspondance est franche : sert au
    /// classement, et n'a pas d'autre sens.
    rang: u8,
}

/// Les fiches qui répondent à `query`, la plus franche d'abord.
///
/// Le classement est ce qui manquait. Chercher « pop » rendait quatre
/// fiches dans l'ordre du manuel — `push`, `pushf`, `popcnt`, `%push` —
/// et celle qu'on voulait n'était nulle part en tête, alors qu'elle
/// portait le nom exact.
///
/// Quatre degrés, du plus franc au plus lâche :
///
/// 1. un nom **exact** ;
/// 2. un nom qui **commence** par ce qu'on a tapé ;
/// 3. un nom qui le **contient** ;
/// 4. le **titre** qui le contient.
///
/// À rang égal, l'ordre du manuel est gardé : les familles s'y suivent,
/// et les bousculer par ordre alphabétique n'apprendrait rien.
pub fn search(query: &str, lang: &str) -> Vec<Found> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Found> = Vec::new();
    for entry in all() {
        // Le meilleur des noms de la fiche : c'est lui qu'on montrera,
        // et c'est lui qui décide du rang.
        let meilleur = entry
            .names
            .iter()
            .map(|name| {
                let rang = if *name == query {
                    0
                } else if name.starts_with(&query) {
                    1
                } else if name.contains(&query) {
                    2
                } else {
                    u8::MAX
                };
                (rang, *name)
            })
            .min_by_key(|(rang, _)| *rang);
        let (rang, name) = match meilleur {
            Some((rang, name)) if rang != u8::MAX => (rang, name),
            // Aucun nom ne correspond : reste le titre, qui répond à
            // « comment ça s'appelle, déjà ? » plutôt qu'à un nom précis.
            _ if entry.title.get(lang).to_lowercase().contains(&query) => (3, entry.names[0]),
            _ => continue,
        };
        out.push(Found { entry, name, rang });
    }
    // `sort_by_key` est stable : à rang égal, l'ordre du manuel tient.
    out.sort_by_key(|trouvé| trouvé.rang);
    out
}

#[cfg(test)]
mod search_tests {
    use super::*;

    /// Chercher « pop » rend d'abord la fiche de `pop`, sous son nom.
    ///
    /// Elle rendait quatre fiches dans l'ordre du manuel — `push`,
    /// `pushf`, `popcnt`, `%push` — et celle qu'on voulait n'était nulle
    /// part en tête. Pire : elle s'y affichait sous le nom « push »,
    /// parce que c'est le premier de la fiche. On cherchait `pop`, on
    /// lisait `push`.
    #[test]
    fn searching_for_pop_answers_pop() {
        let trouvées = search("pop", "fr");
        assert!(!trouvées.is_empty(), "« pop » doit trouver quelque chose");
        assert_eq!(
            trouvées[0].name,
            "pop",
            "et la première doit porter le nom cherché : {:?}",
            trouvées.iter().map(|t| t.name).collect::<Vec<_>>()
        );
        assert!(
            trouvées[0].entry.names.contains(&"pop"),
            "la fiche couvre bien pop"
        );
    }

    /// Un nom exact passe devant un nom qui commence pareil.
    #[test]
    fn an_exact_name_comes_before_one_that_merely_starts_alike() {
        let trouvées = search("mov", "fr");
        assert_eq!(trouvées[0].name, "mov");
        // `movsb`, `movzx` et les autres suivent, mais après.
        assert!(
            trouvées.len() > 1,
            "les cousines sont là aussi : {:?}",
            trouvées.iter().map(|t| t.name).collect::<Vec<_>>()
        );
    }

    /// On trouve aussi par le titre, pour ce dont on ignore le nom.
    ///
    /// C'est la moitié du service : un manuel qu'on ne peut consulter
    /// qu'en sachant déjà le nom de ce qu'on cherche ne sert qu'à ceux
    /// qui n'en ont pas besoin.
    #[test]
    fn one_can_search_without_knowing_the_name() {
        let trouvées = search("pile", "fr");
        assert!(
            !trouvées.is_empty(),
            "« pile » doit mener quelque part : c'est ainsi qu'on cherche \
             ce dont on ignore le nom"
        );
    }

    /// Une recherche vide ne rend rien, et rien ne panique.
    #[test]
    fn nothing_breaks_on_anything() {
        assert!(search("", "fr").is_empty());
        assert!(search("   ", "fr").is_empty());
        for texte in ["zzzz", "%", "é", "MOV", "0x", "'"] {
            let _ = search(texte, "fr");
            let _ = search(texte, "en");
        }
        // Et la casse ne compte pas.
        assert_eq!(search("MOV", "fr")[0].name, search("mov", "fr")[0].name);
    }
}

/// L'adresse de la référence complète d'une instruction, quand il y en a
/// une.
///
/// Les fiches de namsdec tiennent en un écran : elles disent ce que
/// l'instruction fait, ce qu'elle écrit comme indicateurs, et donnent un
/// exemple qui s'assemble. C'est ce qu'on veut en écrivant du code.
///
/// Ce n'est pas ce qu'on veut quand une instruction se comporte
/// autrement qu'on l'espérait : il faut alors le détail — tous les
/// encodages, tous les modes, toutes les exceptions —, c'est-à-dire le
/// manuel d'Intel. Le recopier ici serait absurde : il fait deux mille
/// pages, et il est déjà mis en ligne, instruction par instruction.
///
/// `None` pour ce qui n'est pas une instruction du processeur : une
/// directive de NASM ou une notion n'ont rien à voir chez Intel, et y
/// renvoyer enverrait dans le vide.
pub fn reference_url(entry: &Entry) -> Option<String> {
    if entry.kind != Kind::Instruction {
        return None;
    }
    // felixcloutier.com met en ligne le manuel d'Intel, une page par
    // instruction, adressée par son mnémonique en majuscules.
    let nom = entry.names.first()?.to_ascii_uppercase();
    Some(format!("https://www.felixcloutier.com/x86/{nom}"))
}

#[cfg(test)]
mod reference_tests {
    use super::*;

    /// Une instruction renvoie vers le manuel d'Intel, une notion non.
    #[test]
    fn only_instructions_point_at_intels_manual() {
        let mov = lookup("mov").expect("la fiche de mov");
        let url = reference_url(mov).expect("une instruction a sa référence");
        assert!(url.ends_with("/MOV"), "{url}");
        assert!(url.starts_with("https://"), "{url}");

        // Une directive de NASM n'a rien à voir chez Intel : y renvoyer
        // enverrait dans le vide.
        for nom in ["db", "section"] {
            if let Some(fiche) = lookup(nom) {
                assert!(
                    reference_url(fiche).is_none(),
                    "« {nom} » n'est pas une instruction du processeur"
                );
            }
        }
    }

    /// Chaque instruction documentée a une référence, et une seule forme.
    #[test]
    fn every_instruction_has_one() {
        let mut vues = 0;
        for entry in all().iter().filter(|e| e.kind == Kind::Instruction) {
            let url = reference_url(entry)
                .unwrap_or_else(|| panic!("{:?} devrait avoir une référence", entry.names));
            assert!(
                !url.contains(' ') && url.starts_with("https://www.felixcloutier.com/x86/"),
                "adresse mal formée : {url}"
            );
            vues += 1;
        }
        assert!(vues > 50, "le manuel compte bien des instructions : {vues}");
    }
}
