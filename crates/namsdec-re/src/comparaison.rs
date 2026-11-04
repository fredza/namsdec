//! Deux versions d'un binaire, comparées par leur ANALYSE.
//!
//! **C'est la question qu'on se pose vraiment devant deux versions d'un
//! programme :** une protection a-t-elle disparu ? une chaîne est-elle
//! apparue ? une fonction a-t-elle grossi ? Un `diff` d'octets répond
//! « le fichier a changé à l'offset 0x15604 », ce qui n'apprend rien :
//! recompiler sans rien changer déplace déjà tout. Un `diff` des deux
//! sorties JSON répond mieux, mais en montrant des `"texte":` et des
//! accolades — on lit la forme du rapport, pas le fond.
//!
//! Ce module compare ce que namsdec COMPREND : rubrique par rubrique,
//! ligne par ligne, telles que le rapport les écrit.
//!
//! **Trois décisions, et elles font tout le caractère du résultat.**
//!
//! 1. **On compare des lignes, pas des arbres.** Une vue est un arbre de
//!    blocs ; le comparer nœud par nœud dirait « le troisième bloc de la
//!    deuxième section a changé », ce qui ne se lit pas. Les lignes du
//!    rapport, elles, sont ce qu'un humain lit — et c'est [`rapport::lignes`]
//!    qui les écrit, le même écrivain que le rapport lui-même.
//! 2. **L'ordre ne compte pas, le contenu compte.** Deux analyses se
//!    comparent comme deux SACS de lignes : une ligne qui a seulement
//!    changé de place n'est ni ajoutée ni retirée. Sans cela, une
//!    fonction de plus au milieu d'une liste triée aurait déplacé tout
//!    ce qui la suit, et le rapport aurait annoncé trois cents
//!    changements pour un.
//! 3. **Les rubriques gardent leur ordre**, celui dans lequel on lit un
//!    binaire inconnu — ce que le fichier dit de lui-même, puis son
//!    code, puis ce qu'il dit, puis les octets. Trier par nombre de
//!    changements mettrait les chaînes en tête de tout, à chaque fois.
//!
//! **Ce que cela ne fait pas :** rien n'est interprété. « NX est passé
//! de oui à non » n'est pas qualifié de régression, et aucun score n'est
//! calculé. Le module dit ce qui a changé, à celui qui sait ce qu'il
//! cherche — c'est la même règle que les signaux du panneau d'analyse,
//! qui n'additionnent rien.

use crate::rapport::{self, Panneau};

/// Ce qu'une rubrique est devenue d'une version à l'autre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum État {
    /// Elle n'était pas là, elle y est — un empaqueteur reconnu, une
    /// règle YARA qui mord, un outil qui a enfin quelque chose à dire.
    Apparue,
    /// Elle y était, elle n'y est plus.
    Disparue,
    /// Les deux versions la remplissent, et pas de la même façon.
    Changée,
}

/// Une ligne qui a changé de camp.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Changement {
    /// Présente dans la seconde version seulement.
    Ajout(String),
    /// Présente dans la première seulement.
    Retrait(String),
}

impl Changement {
    pub fn texte(&self) -> &str {
        match self {
            Self::Ajout(texte) | Self::Retrait(texte) => texte,
        }
    }

    /// La marque qui porte le sens SANS la couleur — un fichier texte
    /// n'en a pas, et qui distingue mal le rouge du vert lit encore le
    /// signe. Les mêmes que `diff`, pour la même raison.
    pub fn marque(&self) -> char {
        match self {
            Self::Ajout(_) => '+',
            Self::Retrait(_) => '-',
        }
    }
}

/// Une rubrique, et ce qui lui est arrivé.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Rubrique {
    pub id: String,
    pub titre: String,
    pub état: État,
    /// Les lignes qui ont changé. Vide quand la rubrique entière est
    /// apparue ou a disparu — dans ce cas, c'est elle qui est le
    /// changement, et recopier ses trois cents lignes n'apprendrait
    /// rien de plus.
    pub changements: Vec<Changement>,
}

/// Ce qui a changé entre deux analyses.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Comparaison {
    /// Les rubriques qui ont changé, dans l'ordre de lecture d'un
    /// binaire. Les identiques n'y sont pas.
    pub rubriques: Vec<Rubrique>,
    /// Combien de rubriques les deux versions remplissaient pareil.
    ///
    /// **Ce nombre est la moitié utile du rapport**, et il est dit en
    /// clair : « dix-sept rubriques identiques » est ce qui permet de
    /// croire aux deux qui ont changé. Sans lui, on ne sait pas si l'on
    /// regarde un détail ou le seul endroit où l'on a regardé.
    pub identiques: usize,
}

impl Comparaison {
    /// Les deux analyses disent-elles la même chose ?
    pub fn rien_ne_change(&self) -> bool {
        self.rubriques.is_empty()
    }
}

/// Ce qui a changé d'une analyse à l'autre.
pub fn comparer(avant: &[Panneau], après: &[Panneau]) -> Comparaison {
    let mut comparaison = Comparaison::default();

    // L'ORDRE EST CELUI DE LA SECONDE VERSION, puis ce qui n'y est plus.
    // On regarde ce qu'un fichier est devenu : son rapport à lui est le
    // plan de lecture, et les disparitions se disent après, là où elles
    // étaient.
    for (rang, panneau) in après.iter().enumerate() {
        match avant.iter().find(|autre| autre.id == panneau.id) {
            None => comparaison.rubriques.push(Rubrique {
                id: panneau.id.clone(),
                titre: panneau.titre.clone(),
                état: État::Apparue,
                changements: Vec::new(),
            }),
            Some(ancien) => {
                let changements = lignes_changées(ancien, panneau);
                if changements.is_empty() {
                    comparaison.identiques += 1;
                } else {
                    comparaison.rubriques.push(Rubrique {
                        id: panneau.id.clone(),
                        titre: panneau.titre.clone(),
                        état: État::Changée,
                        changements,
                    });
                }
            }
        }
        // Les rubriques disparues, à leur place : celles que la première
        // version portait entre celle-ci et la suivante. Sans cela,
        // elles s'entassaient toutes à la fin, loin de ce dont elles
        // parlent.
        let suivante = après.get(rang + 1).map(|p| p.id.as_str());
        for disparue in disparues_entre(avant, après, &panneau.id, suivante) {
            comparaison.rubriques.push(disparue);
        }
    }

    // Et celles qui disparaissaient AVANT la première rubrique de la
    // seconde version — ou toutes, si la seconde n'en a aucune.
    let première = après.first().map(|p| p.id.as_str());
    let mut avant_tout = disparues_avant(avant, après, première);
    avant_tout.extend(comparaison.rubriques);
    comparaison.rubriques = avant_tout;
    comparaison
}

/// Les rubriques que la première version portait et que la seconde n'a
/// plus, situées entre deux rubriques de celle-ci.
fn disparues_entre(
    avant: &[Panneau],
    après: &[Panneau],
    ici: &str,
    suivante: Option<&str>,
) -> Vec<Rubrique> {
    let début = avant.iter().position(|p| p.id == ici);
    let Some(début) = début else {
        return Vec::new();
    };
    avant
        .iter()
        .skip(début + 1)
        .take_while(|p| suivante != Some(p.id.as_str()))
        .filter(|p| !après.iter().any(|autre| autre.id == p.id))
        .map(disparue)
        .collect()
}

/// Celles qui disparaissaient avant la première rubrique retenue.
fn disparues_avant(
    avant: &[Panneau], après: &[Panneau], première: Option<&str>
) -> Vec<Rubrique> {
    avant
        .iter()
        .take_while(|p| première != Some(p.id.as_str()))
        .filter(|p| !après.iter().any(|autre| autre.id == p.id))
        .map(disparue)
        .collect()
}

fn disparue(panneau: &Panneau) -> Rubrique {
    Rubrique {
        id: panneau.id.clone(),
        titre: panneau.titre.clone(),
        état: État::Disparue,
        changements: Vec::new(),
    }
}

/// Les lignes qui ont changé entre deux versions d'une même rubrique.
///
/// Deux SACS de lignes, et non deux suites : une ligne qui a seulement
/// changé de place n'est ni ajoutée ni retirée. C'est ce qui garde le
/// rapport lisible quand une fonction de plus décale tout ce qui la
/// suit dans une liste triée.
///
/// Les retraits d'abord, les ajouts ensuite, chacun dans l'ordre de sa
/// propre version : on lit « ce qui n'y est plus », puis « ce qui y
/// est ».
fn lignes_changées(avant: &Panneau, après: &Panneau) -> Vec<Changement> {
    use std::collections::HashMap;

    let compter = |panneau: &Panneau| -> HashMap<String, usize> {
        let mut compte = HashMap::new();
        for ligne in rapport::lignes(panneau) {
            *compte.entry(ligne).or_insert(0) += 1;
        }
        compte
    };
    let (mut restant_avant, mut restant_après) = (compter(avant), compter(après));
    // Ce que les deux portent autant de fois s'annule. Une ligne
    // présente trois fois d'un côté et une fois de l'autre laisse deux
    // retraits : les comptes comptent, parce qu'un tableau peut porter
    // deux fois la même valeur.
    for (ligne, combien) in &mut restant_avant {
        if let Some(aussi) = restant_après.get_mut(ligne) {
            let commun = (*combien).min(*aussi);
            *combien -= commun;
            *aussi -= commun;
        }
    }

    let mut changements = Vec::new();
    let tirer = |panneau: &Panneau,
                 restant: &mut HashMap<String, usize>,
                 faire: fn(String) -> Changement,
                 dans: &mut Vec<Changement>| {
        for ligne in rapport::lignes(panneau) {
            if let Some(combien) = restant.get_mut(&ligne)
                && *combien > 0
            {
                *combien -= 1;
                dans.push(faire(ligne));
            }
        }
    };
    tirer(
        avant,
        &mut restant_avant,
        Changement::Retrait,
        &mut changements,
    );
    tirer(
        après,
        &mut restant_après,
        Changement::Ajout,
        &mut changements,
    );
    changements
}

/// La comparaison, en Markdown ordinaire.
///
/// Les deux noms sont dits en tête : un rapport de comparaison qu'on
/// retrouve six mois plus tard doit dire ce qu'il comparait, sans quoi
/// « + NX : absent » ne se lit plus dans aucun sens.
pub fn markdown(comparaison: &Comparaison, avant: &str, après: &str, lang: &str) -> String {
    /// Le pluriel, en français comme en anglais : un « s » au-delà de un.
    fn s(combien: usize) -> &'static str {
        if combien > 1 { "s" } else { "" }
    }

    let fr = lang.starts_with("fr");
    let mut out = if fr {
        format!("# Ce qui a changé — {avant} → {après}\n\n")
    } else {
        format!("# What changed — {avant} → {après}\n\n")
    };

    // L'ACCORD EN NOMBRE, et il ne coûte qu'une fonction : « 1
    // rubrique(s) ont changé » est la marque d'une phrase qu'aucun
    // humain n'a relue, et ce rapport est fait pour être lu.
    let combien = comparaison.rubriques.len();
    if comparaison.rien_ne_change() {
        out.push_str(&if fr {
            format!(
                "Les deux analyses disent la même chose : {} rubrique{}, \
                 et pas une qui diffère.\n",
                comparaison.identiques,
                s(comparaison.identiques)
            )
        } else {
            format!(
                "Both analyses say the same thing: {} section{}, not one differs.\n",
                comparaison.identiques,
                s(comparaison.identiques)
            )
        });
        return out;
    }

    out.push_str(&if fr {
        format!(
            "{combien} rubrique{} {} changé, {} {} la même chose.\n",
            s(combien),
            if combien > 1 { "ont" } else { "a" },
            comparaison.identiques,
            if comparaison.identiques > 1 {
                "disent"
            } else {
                "dit"
            }
        )
    } else {
        format!(
            "{combien} section{} changed, {} say{} the same thing.\n",
            s(combien),
            comparaison.identiques,
            if comparaison.identiques == 1 { "s" } else { "" }
        )
    });

    for rubrique in &comparaison.rubriques {
        let état = match (rubrique.état, fr) {
            (État::Apparue, true) => "apparue",
            (État::Apparue, false) => "appeared",
            (État::Disparue, true) => "disparue",
            (État::Disparue, false) => "gone",
            (État::Changée, true) => "changée",
            (État::Changée, false) => "changed",
        };
        out.push_str(&format!("\n## {} — {état}\n\n", rubrique.titre));
        if rubrique.changements.is_empty() {
            // APPARUE OU DISPARUE : la rubrique EST le changement, et
            // recopier ses trois cents lignes noierait les autres.
            out.push_str(&if fr {
                format!(
                    "La rubrique entière, qui n'était pas dans l'autre version. \
                     « namsdec-cli … -s {} » la lit en détail.\n",
                    rubrique.id
                )
            } else {
                format!(
                    "The whole section, absent from the other version. \
                     `namsdec-cli … -s {}` reads it in full.\n",
                    rubrique.id
                )
            });
            continue;
        }
        out.push_str("```diff\n");
        for changement in &rubrique.changements {
            out.push_str(&format!("{} {}\n", changement.marque(), changement.texte()));
        }
        out.push_str("```\n");
    }
    out
}

/// La même chose pour un autre programme.
pub fn json(comparaison: &Comparaison) -> String {
    serde_json::to_string_pretty(comparaison).unwrap_or_else(|_| "{}".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vue::{Fragment, Vue};

    fn panneau(id: &str, lignes: &[&str]) -> Panneau {
        let mut vue = Vue::neuve();
        for ligne in lignes {
            vue = vue.texte(vec![Fragment::dit(*ligne)]);
        }
        Panneau {
            id: id.to_owned(),
            titre: id.to_uppercase(),
            vue,
        }
    }

    /// Deux analyses identiques ne disent rien, et le disent.
    #[test]
    fn rien_ne_change_quand_rien_ne_change() {
        let avant = vec![panneau("a", &["un", "deux"]), panneau("b", &["trois"])];
        let comparaison = comparer(&avant, &avant.clone());
        assert!(comparaison.rien_ne_change());
        assert_eq!(comparaison.identiques, 2, "et les deux sont comptées");
        let md = markdown(&comparaison, "v1", "v2", "fr");
        assert!(md.contains("2 rubriques, et pas une qui diffère"), "{md}");
    }

    /// Une ligne qui apparaît, une qui s'en va, et le reste se tait.
    #[test]
    fn une_ligne_qui_change_se_dit_des_deux_côtés() {
        let avant = vec![panneau("a", &["NX : oui", "RELRO : complet"])];
        let après = vec![panneau("a", &["NX : non", "RELRO : complet"])];
        let comparaison = comparer(&avant, &après);
        let rubrique = &comparaison.rubriques[0];
        assert_eq!(rubrique.état, État::Changée);
        assert_eq!(
            rubrique.changements,
            vec![
                Changement::Retrait("NX : oui".to_owned()),
                Changement::Ajout("NX : non".to_owned()),
            ],
            "ce qui n'a pas changé ne se dit pas"
        );
        let md = markdown(&comparaison, "v1", "v2", "fr");
        assert!(
            md.contains("- NX : oui") && md.contains("+ NX : non"),
            "{md}"
        );
        assert!(
            !md.contains("RELRO"),
            "et la ligne identique n'est pas recopiée : {md}"
        );
    }

    /// UNE LIGNE QUI A SEULEMENT CHANGÉ DE PLACE N'A PAS CHANGÉ.
    ///
    /// C'est la décision qui fait tout le caractère de ce module, et
    /// celle qui se casse si l'on compare deux suites plutôt que deux
    /// sacs : une fonction de plus au milieu d'une liste triée décale
    /// tout ce qui la suit, et le rapport annoncerait trois cents
    /// changements pour un.
    #[test]
    fn une_ligne_déplacée_n_est_pas_un_changement() {
        let avant = vec![panneau("f", &["sub_1000", "sub_3000", "sub_4000"])];
        let après = vec![panneau(
            "f",
            &["sub_1000", "sub_2000", "sub_3000", "sub_4000"],
        )];
        let comparaison = comparer(&avant, &après);
        assert_eq!(
            comparaison.rubriques[0].changements,
            vec![Changement::Ajout("sub_2000".to_owned())],
            "un ajout, et un seul"
        );
    }

    /// La même ligne deux fois d'un côté, une fois de l'autre : il en
    /// manque une, et c'est un changement.
    ///
    /// Un tableau peut porter deux fois la même valeur — deux fonctions
    /// de même taille, deux chaînes identiques à deux adresses. Comparer
    /// des ENSEMBLES au lieu de sacs aurait tout annulé.
    #[test]
    fn les_répétitions_se_comptent() {
        let avant = vec![panneau("s", &["« salut »", "« salut »"])];
        let après = vec![panneau("s", &["« salut »"])];
        let comparaison = comparer(&avant, &après);
        assert_eq!(
            comparaison.rubriques[0].changements,
            vec![Changement::Retrait("« salut »".to_owned())]
        );
    }

    /// Une rubrique qui apparaît, une qui disparaît — et chacune à sa
    /// place dans la liste.
    #[test]
    fn une_rubrique_entière_peut_apparaître_ou_disparaître() {
        let avant = vec![panneau("bref", &["x"]), panneau("upx", &["empaqueté"])];
        let après = vec![panneau("bref", &["x"]), panneau("yara", &["une règle"])];
        let comparaison = comparer(&avant, &après);
        let états: Vec<(&str, État)> = comparaison
            .rubriques
            .iter()
            .map(|r| (r.id.as_str(), r.état))
            .collect();
        // L'ORDRE EST POSITIONNEL, et c'est ce qui le rend lisible :
        // `upx` venait après « En bref » dans la première version, donc
        // sa disparition se dit là ; `yara` paraît ensuite, à sa place
        // dans la seconde. Entassées à la fin, les disparitions se
        // liraient loin de ce dont elles parlent.
        assert_eq!(
            états,
            vec![("upx", État::Disparue), ("yara", État::Apparue)],
            "celle qui reste identique n'est pas listée"
        );
        assert_eq!(comparaison.identiques, 1);

        // ET LA RUBRIQUE ENTIÈRE NE SE RECOPIE PAS : elle EST le
        // changement, et trois cents lignes noieraient le reste.
        assert!(comparaison.rubriques[0].changements.is_empty());
        let md = markdown(&comparaison, "v1", "v2", "fr");
        assert!(
            md.contains("-s yara"),
            "elle dit où la lire en détail : {md}"
        );
    }

    /// Ce qu'on écrit se relit, pour un autre programme.
    #[test]
    fn la_comparaison_se_relit_en_json() {
        let avant = vec![panneau("a", &["un"])];
        let après = vec![panneau("a", &["deux"])];
        let comparaison = comparer(&avant, &après);
        let texte = json(&comparaison);
        let relue: Comparaison = serde_json::from_str(&texte).expect("ce qu'on écrit se relit");
        assert_eq!(relue.rubriques.len(), 1);
        assert_eq!(
            relue.rubriques[0].changements,
            comparaison.rubriques[0].changements
        );
    }
}
