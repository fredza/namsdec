//! Répondre à une question en LISANT, et savoir si l'on avait juste.
//!
//! **Ce que ce module corrige :** les cent huit exercices font écrire du
//! code. Douze d'entre eux disent où regarder dans namsdec — c'est déjà
//! lire —, mais leur but s'atteint en exécutant le programme livré :
//! rien ne vérifie qu'on a lu quoi que ce soit. On apprend `shl` en
//! l'écrivant, jamais à dire ce qu'il VA faire, et c'est pourtant le
//! seul geste qu'on ait devant un binaire qu'on n'a pas écrit.
//!
//! Un exercice de lecture porte donc une **question** et sa réponse. On
//! lit, on répond, puis on exécute — et la machine tranche. C'est le
//! même chemin que « prévoir avant d'avancer » dans l'interface, à
//! l'échelle d'un programme entier.
//!
//! **La réponse s'écrit comme on veut.** Quelqu'un qui a compris que
//! `shl rax, 3` multiplie par huit écrira « 32 », « 0x20 », « 20h » ou
//! « 0b100000 » selon ce qu'il a dans la tête, et refuser trois de ces
//! quatre écritures n'enseignerait que notre propre notation. On compare
//! donc des NOMBRES, pas des chaînes — et à défaut de nombre, des mots
//! rognés et sans casse, pour les questions dont la réponse est `jne`.
//!
//! **Et la réponse attendue est écrite comme une condition** — `rax ==
//! 0x20` —, la même grammaire que les buts. Deux raisons, et la seconde
//! est la vraie : l'auteur de la série nomme ainsi ce qu'il interroge,
//! et surtout un contrôle peut **exécuter le programme et vérifier que
//! la machine donne bien cette réponse-là**. Une réponse écrite à la
//! main dans un fichier JSON est une affirmation ; passée à l'épreuve de
//! l'émulateur, c'est un fait.

/// Ce que vaut la réponse donnée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Rien n'a encore été écrit. On ne dit pas « faux » à qui n'a pas
    /// répondu : c'est la différence entre se tromper et réfléchir.
    Vide,
    Juste,
    Faux,
}

impl Verdict {
    pub fn est_juste(self) -> bool {
        self == Self::Juste
    }
}

/// La valeur attendue, telle qu'elle est écrite dans la série.
///
/// `rax == 0x20` donne `0x20` : ce qui est à droite du `==` est la
/// réponse, ce qui est à gauche dit ce qu'on interroge. Sans `==`, toute
/// la chaîne est la réponse — une série peut interroger un mnémonique
/// (`jne`) ou un nom de section.
pub fn attendue(réponse: &str) -> &str {
    match réponse.split_once("==") {
        Some((_, droite)) => droite.trim(),
        None => réponse.trim(),
    }
}

/// La réponse donnée vaut-elle celle qu'on attendait ?
pub fn verdict(donnée: &str, réponse: &str) -> Verdict {
    let donnée = donnée.trim();
    if donnée.is_empty() {
        return Verdict::Vide;
    }
    let attendue = attendue(réponse);
    let juste = match (nombre(donnée), nombre(attendue)) {
        // DEUX NOMBRES SE COMPARENT COMME DES NOMBRES : « 32 »,
        // « 0x20 » et « 0b100000 » sont la même réponse, et celui qui
        // la donne en binaire a compris la même chose.
        (Some(a), Some(b)) => a == b,
        // Sinon, des mots : rognés, sans casse, et c'est tout. On ne
        // devine pas plus loin — « j'allais dire jne » n'est pas une
        // réponse qu'un programme peut reconnaître.
        _ => donnée.eq_ignore_ascii_case(attendue),
    };
    if juste { Verdict::Juste } else { Verdict::Faux }
}

/// Un nombre écrit comme on l'écrit vraiment.
///
/// Les quatre notations qu'on rencontre : décimale, `0x` et le `h` final
/// des assembleurs, `0b`, et le signe moins — `mov rax, -1` est une
/// réponse qu'on écrit `-1` et que la machine garde en
/// `0xffffffffffffffff`. Les deux doivent valoir, donc un nombre négatif
/// est ramené à sa forme sur soixante-quatre bits : c'est la même valeur
/// dans le registre, et refuser l'une des deux écritures serait
/// interroger notre notation plutôt que la machine.
///
/// Les blancs et les traits bas séparant les groupes de chiffres sont
/// ignorés : `0xdead_beef` et `0xdeadbeef` sont le même nombre.
pub fn nombre(texte: &str) -> Option<u64> {
    let texte: String = texte
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .collect();
    let (négatif, corps) = match texte.strip_prefix('-') {
        Some(reste) => (true, reste),
        None => (false, texte.as_str()),
    };
    let bas = corps.to_ascii_lowercase();
    let valeur = if let Some(hexa) = bas.strip_prefix("0x") {
        u64::from_str_radix(hexa, 16).ok()?
    } else if let Some(binaire) = bas.strip_prefix("0b") {
        u64::from_str_radix(binaire, 2).ok()?
    } else if let Some(hexa) = bas.strip_suffix('h') {
        u64::from_str_radix(hexa, 16).ok()?
    } else {
        bas.parse::<u64>().ok()?
    };
    Some(if négatif {
        (valeur as i64).checked_neg()? as u64
    } else {
        valeur
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La réponse attendue se lit dans la condition.
    #[test]
    fn ce_qui_est_à_droite_du_double_égal_est_la_réponse() {
        assert_eq!(attendue("rax == 0x20"), "0x20");
        assert_eq!(attendue("[tampon+2] == 0x44"), "0x44");
        // Sans condition, toute la chaîne : une série peut interroger
        // autre chose qu'un registre.
        assert_eq!(attendue("jne"), "jne");
    }

    /// LA MÊME RÉPONSE, ÉCRITE DE QUATRE FAÇONS, EST LA MÊME RÉPONSE.
    ///
    /// C'est la décision qui fait tout ce module : refuser « 32 » parce
    /// que la série a écrit « 0x20 » n'enseignerait que notre notation.
    #[test]
    fn quatre_écritures_du_même_nombre_valent_toutes() {
        for écriture in ["32", "0x20", "0X20", "20h", "0b100000", " 32 ", "0b10_0000"] {
            assert!(
                verdict(écriture, "rax == 0x20").est_juste(),
                "« {écriture} » est la bonne réponse"
            );
        }
        assert_eq!(verdict("33", "rax == 0x20"), Verdict::Faux);
        // Et « 0x32 » n'est pas « 32 » : la base compte, c'est tout
        // l'intérêt de l'écrire.
        assert_eq!(verdict("0x32", "rax == 32"), Verdict::Faux);
    }

    /// Un nombre négatif et sa forme sur soixante-quatre bits sont la
    /// même valeur dans le registre.
    #[test]
    fn moins_un_et_ses_seize_f_sont_la_même_réponse() {
        assert!(verdict("-1", "rax == 0xffffffffffffffff").est_juste());
        assert!(verdict("0xffffffffffffffff", "rax == -1").est_juste());
        assert!(verdict("-8", "rax == 0xfffffffffffffff8").est_juste());
    }

    /// Rien d'écrit n'est pas une erreur.
    ///
    /// « Faux » devant un champ vide dirait à quelqu'un qui réfléchit
    /// qu'il s'est trompé, et c'est exactement le moment où l'on
    /// abandonne.
    #[test]
    fn un_champ_vide_ne_vaut_pas_faux() {
        assert_eq!(verdict("", "rax == 1"), Verdict::Vide);
        assert_eq!(verdict("   ", "rax == 1"), Verdict::Vide);
    }

    /// Une réponse en mots : rognée, sans casse, et rien de plus.
    #[test]
    fn une_réponse_en_mots_se_compare_sans_la_casse() {
        assert!(verdict("jne", "jne").est_juste());
        assert!(verdict(" JNE ", "jne").est_juste());
        assert_eq!(verdict("je", "jne"), Verdict::Faux);
        // Et l'on ne devine pas : une phrase autour de la réponse n'est
        // pas la réponse. Un programme qui accepterait « c'est jne »
        // accepterait aussi « ce n'est pas jne ».
        assert_eq!(verdict("c'est jne", "jne"), Verdict::Faux);
    }

    /// Ce qui n'est pas un nombre ne devient pas zéro.
    #[test]
    fn ce_qui_n_est_pas_un_nombre_n_en_est_pas_un() {
        assert_eq!(nombre("zéro"), None);
        assert_eq!(nombre(""), None);
        assert_eq!(nombre("0x"), None);
        assert_eq!(nombre("12g"), None);
        // Et un dépassement n'est pas un nombre : `u64::MAX + 1` ne
        // vaut pas zéro.
        assert_eq!(nombre("0x1_0000_0000_0000_0000"), None);
        assert_eq!(nombre("0"), Some(0));
    }
}
