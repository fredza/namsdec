//! « Prévoir avant d'avancer » : lire, prévoir, avancer, constater.
//!
//! **Le défaut que ce module corrige n'est pas un manque, c'est un
//! excès.** Devant une instruction qui n'a pas encore été exécutée,
//! namsdec dit déjà tout : ce qu'elle fait en clair, ses opérandes
//! résolus, et — devant un saut conditionnel — s'il sera pris. On
//! appuie sur « Pas à pas » pour vérifier une réponse qu'on a lue.
//! Celui qui apprend n'a jamais eu l'occasion de se tromper, et se
//! tromper est la seule façon de savoir qu'on n'avait pas compris.
//!
//! Le travail consiste donc à **RETENIR**, pas à ajouter : l'adresse et
//! l'instruction restent à l'écran — c'est l'énoncé —, la réponse
//! attend qu'on ait avancé, et le bloc « Dernier pas » la révèle à côté
//! de ce qui a vraiment changé. Rien de neuf n'est calculé ; ce qui
//! existait change seulement de moment.
//!
//! **Et cela ne s'impose à personne.** Le réglage ne vaut qu'au niveau
//! débutant : un quiz imposé à qui se sert de l'outil tous les jours
//! serait une taxe, pas une leçon. Il vit dans `settings.json` comme
//! tout ce que l'utilisateur règle.
//!
//! Tout ce que ce module décide est de la **logique** — pas du dessin :
//! il s'éprouve sans peindre une image, et les contrôles du bas de ce
//! fichier n'ouvrent aucune fenêtre.

use namsdec_re::Niveau;

/// Ce que le bandeau fait de la réponse, à cet instant précis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Réponse {
    /// On la retient. L'instruction est montrée, la question est posée,
    /// et ce qu'elle va faire attend le pas — ou un clic sur
    /// « Montrer ».
    Retenue,
    /// On la montre, comme namsdec l'a toujours fait : le pseudo-code en
    /// clair, le détail, et le verdict d'un saut conditionnel.
    Montrée,
}

impl Réponse {
    pub(crate) fn est_retenue(self) -> bool {
        self == Self::Retenue
    }
}

/// Faut-il retenir ce que la prochaine instruction va faire ?
///
/// Trois conditions, et il faut les trois. Le réglage, parce que
/// personne n'a à subir un exercice qu'il n'a pas demandé ; le niveau
/// débutant, parce que c'est le seul où la question se pose ; et que la
/// réponse n'ait pas déjà été demandée POUR CETTE ADRESSE — sans ce
/// dernier point, « Montrer » ne tiendrait pas une image : le bandeau
/// se redessine soixante fois par seconde et recacherait aussitôt ce
/// qu'on vient de découvrir.
///
/// L'adresse, et non un simple drapeau : au pas suivant, la question se
/// repose d'elle-même. C'est ce qui fait de la prévision une habitude
/// plutôt qu'une case à cocher une fois pour toutes.
pub(crate) fn réponse(
    prévoir: bool, niveau: Niveau, rip: u64, révélée: Option<u64>
) -> Réponse {
    if prévoir && niveau == Niveau::Débutant && révélée != Some(rip) {
        Réponse::Retenue
    } else {
        Réponse::Montrée
    }
}

/// Ce que l'instruction exécutée annonçait, à relire après le pas.
///
/// `None` quand le pseudo-code ne dit rien de plus que l'instruction
/// elle-même — `hlt`, `nop`, un saut inconditionnel. Dans ce cas, le
/// bloc « Dernier pas » garde exactement ce qu'il a toujours montré :
/// mettre « ce que ça disait : hlt » à côté de « hlt » n'apprend rien,
/// et une ligne qui n'apprend rien dans un panneau que l'on regarde à
/// chaque pas est pire qu'une ligne absente.
pub(crate) fn annonce(texte: &str, clair: &str) -> Option<String> {
    let clair = clair.trim();
    (!clair.is_empty() && clair != texte.trim()).then(|| clair.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trois conditions, et il faut les trois.
    #[test]
    fn la_réponse_ne_se_retient_qu_au_niveau_débutant_et_sur_demande() {
        // Le cas pour lequel tout ceci existe.
        assert!(réponse(true, Niveau::Débutant, 0x1000, None).est_retenue());

        // Le réglage éteint : namsdec se comporte comme avant, et c'est
        // le défaut pour qui a déjà une configuration.
        assert!(!réponse(false, Niveau::Débutant, 0x1000, None).est_retenue());

        // Le niveau avancé : la réponse ne se retient jamais, réglage ou
        // pas. Un quiz imposé à qui se sert de l'outil tous les jours
        // serait une taxe.
        assert!(!réponse(true, Niveau::Avancé, 0x1000, None).est_retenue());
    }

    /// « Montrer » tient, et ne tient que pour cette instruction-là.
    ///
    /// C'EST LE POINT QUI SE CASSE LE PLUS FACILEMENT : un drapeau
    /// booléen aurait marché à l'écran et aurait éteint la prévision
    /// pour tout le reste de la séance, au premier clic.
    #[test]
    fn montrer_ne_vaut_que_pour_l_instruction_qu_on_regardait() {
        let révélée = Some(0x1000);
        assert!(!réponse(true, Niveau::Débutant, 0x1000, révélée).est_retenue());
        // Le pas suivant : la question se repose.
        assert!(réponse(true, Niveau::Débutant, 0x1004, révélée).est_retenue());
        // Et revenir en arrière — « Pas en arrière » existe — retrouve la
        // réponse déjà découverte, plutôt que de faire semblant de
        // l'ignorer.
        assert!(!réponse(true, Niveau::Débutant, 0x1000, révélée).est_retenue());
    }

    /// Ce qui n'apprend rien ne s'écrit pas.
    #[test]
    fn l_annonce_se_taît_quand_elle_répéterait_l_instruction() {
        assert_eq!(annonce("mov rax, 1", "rax ← 1"), Some("rax ← 1".to_owned()));
        // Le pseudo-code rend l'instruction telle quelle quand il n'a
        // rien à en dire de plus.
        assert_eq!(annonce("hlt", "hlt"), None);
        assert_eq!(annonce("nop", ""), None);
        // Et les blancs ne font pas une différence.
        assert_eq!(annonce(" hlt ", "hlt "), None);
    }
}
