//! Le contrat JSON entre DZF et namsdec — côté lecture.
//!
//! DZF (le désassembleur-décompilateur maison, bâti à part en C++23) reçoit
//! de namsdec les octets exécutables et leur adresse, et rend une
//! décompilation en JSON. Ce module décrit **ce que namsdec attend de ce
//! JSON**, en types serde : c'est le contrat sous sa forme exécutable, et
//! la spec lisible qui l'accompagne (`docs/contrat-dzf.md`) dit la même
//! chose en français.
//!
//! **Tolérant par principe.** DZF grandira ; namsdec ne doit pas casser à
//! chaque champ nouveau. Donc : tout champ optionnel a un défaut, les
//! champs inconnus sont **ignorés** (serde le fait par défaut, et l'on se
//! garde bien de `deny_unknown_fields`), et un `version` permet de savoir
//! à qui l'on parle. Une seule chose est obligatoire : qu'une fonction
//! porte son **adresse**, car c'est par elle que tout se relie — le
//! listing, les noms donnés à la main, les renvois.
//!
//! Les adresses sont des **adresses virtuelles** (u64), les mêmes que
//! celles du listing et de la table de noms. C'est ce qui permet qu'une
//! fonction rebaptisée dans l'arbre d'appels le soit aussi dans la
//! décompilation DZF.

use serde::Deserialize;

/// Ce que DZF rend pour un binaire entier.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DzfOutput {
    /// La version du contrat que DZF a suivie. 0 si DZF ne la dit pas —
    /// auquel cas on lit au mieux, sans rien exiger de neuf.
    #[serde(default)]
    pub version: u32,
    /// Les fonctions décompilées, dans l'ordre où DZF les donne.
    #[serde(default)]
    pub functions: Vec<DzfFunction>,
    /// Ce que DZF n'a pas su faire : une fonction trop emmêlée, une
    /// instruction inconnue. Affiché tel quel, pour ne pas faire croire
    /// que l'absence est un vide.
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

/// Une fonction décompilée.
#[derive(Debug, Clone, Deserialize)]
pub struct DzfFunction {
    /// L'adresse virtuelle d'entrée. **Obligatoire** : c'est la clé qui
    /// relie cette fonction au reste de namsdec.
    pub address: u64,
    /// Le nom que DZF propose (`sub_401020`, ou mieux s'il a deviné). Un
    /// nom donné à la main dans namsdec le remplace à l'affichage —
    /// DZF n'a pas à le savoir.
    #[serde(default)]
    pub name: Option<String>,
    /// La signature, sur une ligne, quand DZF la reconstitue
    /// (`int sub_401020(int64 a1)`). Facultative.
    #[serde(default)]
    pub signature: Option<String>,
    /// Le pseudo-code, en C-isme, sur plusieurs lignes. Le cœur : c'est
    /// ce qu'on lit. Vide si DZF n'a rendu que la signature.
    #[serde(default)]
    pub pseudocode: String,
    /// La taille de la fonction en octets, si DZF la connaît.
    #[serde(default)]
    pub size: Option<u64>,
    /// Les adresses que cette fonction appelle, pour recouper avec le
    /// graphe d'appels de namsdec. Facultatif.
    #[serde(default)]
    pub calls: Vec<u64>,
}

/// Lit la sortie de DZF. Rend une erreur seulement sur un JSON mal formé
/// ou une fonction sans adresse — jamais sur un champ nouveau qu'on ne
/// connaît pas.
pub fn parse(json: &str) -> Result<DzfOutput, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXEMPLE: &str = r#"{
        "version": 1,
        "functions": [
            {
                "address": 4198432,
                "name": "verifie_cle",
                "signature": "int verifie_cle(char *saisie)",
                "pseudocode": "int verifie_cle(char *saisie) {\n  return strcmp(saisie, \"42\") == 0;\n}",
                "size": 48,
                "calls": [4198400]
            }
        ],
        "diagnostics": ["sub_401500 : trop de chemins, abandonnée"]
    }"#;

    #[test]
    fn un_exemple_complet_se_lit() {
        let out = parse(EXEMPLE).expect("JSON valide");
        assert_eq!(out.version, 1);
        assert_eq!(out.functions.len(), 1);
        let f = &out.functions[0];
        assert_eq!(f.address, 4198432);
        assert_eq!(f.name.as_deref(), Some("verifie_cle"));
        assert!(f.pseudocode.contains("strcmp"));
        assert_eq!(f.size, Some(48));
        assert_eq!(f.calls, vec![4198400]);
        assert_eq!(out.diagnostics.len(), 1);
    }

    /// Le strict minimum : une fonction avec sa seule adresse. Tout le
    /// reste prend son défaut.
    #[test]
    fn le_minimum_suffit() {
        let out = parse(r#"{"functions":[{"address":4096}]}"#).expect("minimal");
        assert_eq!(out.version, 0);
        let f = &out.functions[0];
        assert_eq!(f.address, 4096);
        assert_eq!(f.name, None);
        assert!(f.pseudocode.is_empty());
        assert!(f.calls.is_empty());
    }

    /// Un champ que namsdec ne connaît pas encore (DZF a grandi) est
    /// IGNORÉ, pas une erreur : le contrat tient dans le temps.
    #[test]
    fn un_champ_inconnu_est_ignore() {
        let json = r#"{"version":9,"functions":[{"address":1,"pseudocode":"x","confiance":0.9,"tags":["a"]}],"nouveaute":true}"#;
        let out = parse(json).expect("les champs inconnus ne cassent rien");
        assert_eq!(out.version, 9);
        assert_eq!(out.functions[0].pseudocode, "x");
    }

    /// Une fonction sans adresse est une erreur : sans elle, rien ne se
    /// relie. C'est la seule exigence dure du contrat.
    #[test]
    fn une_fonction_sans_adresse_est_refusee() {
        assert!(parse(r#"{"functions":[{"pseudocode":"orpheline"}]}"#).is_err());
    }

    /// Un JSON vide, ou sans fonctions, se lit sans erreur : DZF peut
    /// n'avoir rien trouvé, et ce n'est pas un plantage.
    #[test]
    fn le_vide_se_lit() {
        assert_eq!(parse("{}").expect("vide").functions.len(), 0);
    }
}
