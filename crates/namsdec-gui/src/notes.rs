//! Ce qu'on a compris d'un binaire, et qu'il ne dit pas lui-même.
//!
//! Un binaire dépouillé n'a que des adresses. On passe une soirée à
//! comprendre que `0x1d491` vérifie le mot de passe, et le lendemain il
//! n'y a plus que `0x1d491`. Ce module garde le nom qu'on a trouvé — et
//! les commentaires qu'on a écrits en chemin.
//!
//! **Rangé à part du binaire, jamais dedans.** On n'écrit pas dans
//! l'exécutable de quelqu'un pour y noter ce qu'on en pense : les notes
//! vivent dans le dossier de namsdec, et le binaire reste l'octet près
//! tel qu'on l'a reçu.
//!
//! **La clé est le contenu, non le chemin.** Un fichier déplacé ou
//! renommé retrouve ses notes ; un binaire recompilé n'hérite pas de
//! celles d'un autre. C'est la même leçon que pour les points d'arrêt :
//! un repère qui désigne autre chose que ce qu'on croit est pire qu'un
//! repère absent.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Les noms et les commentaires d'un binaire.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notes {
    /// Adresse -> nom donné à la main.
    ///
    /// Ce nom l'emporte sur le symbole quand il y en a un : quelqu'un qui
    /// renomme `sub_1d491` en `vérifie_mdp` veut voir le second.
    #[serde(default)]
    pub names: BTreeMap<u64, String>,
    /// Adresse -> commentaire, montré en fin de ligne.
    #[serde(default)]
    pub comments: BTreeMap<u64, String>,
}

impl Notes {
    /// Y a-t-il quelque chose à garder ?
    ///
    /// Sert à ne pas laisser un fichier vide derrière soi : ouvrir un
    /// binaire sans rien y écrire ne doit rien créer.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.comments.is_empty()
    }

    /// Le nom d'une adresse, s'il en a reçu un.
    pub fn name(&self, addr: u64) -> Option<&str> {
        self.names.get(&addr).map(String::as_str)
    }

    /// Le commentaire d'une adresse.
    pub fn comment(&self, addr: u64) -> Option<&str> {
        self.comments.get(&addr).map(String::as_str)
    }

    /// Pose un nom, ou l'efface si le texte est vide.
    ///
    /// Effacer en laissant le champ vide, plutôt qu'un second geste : on
    /// retire un nom comme on le corrige, et demander une commande
    /// séparée pour cela ferait chercher.
    pub fn set_name(&mut self, addr: u64, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            self.names.remove(&addr);
        } else {
            self.names.insert(addr, name.to_owned());
        }
    }

    /// Pose un commentaire, ou l'efface si le texte est vide.
    pub fn set_comment(&mut self, addr: u64, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            self.comments.remove(&addr);
        } else {
            self.comments.insert(addr, text.to_owned());
        }
    }
}

/// L'empreinte d'un binaire : ce qui sert de nom à son fichier de notes.
///
/// FNV-1a sur tout le contenu, et la taille avec lui. Écrit à la main
/// plutôt que pris d'une bibliothèque, pour deux raisons : ce n'est pas
/// de la cryptographie — personne n'essaie de tromper personne —, et
/// surtout le résultat doit rester **le même d'une version à l'autre**.
/// `DefaultHasher` ne le garantit pas, et des notes qui changent de
/// fichier à la prochaine mise à jour de Rust seraient perdues sans que
/// rien ne le dise.
pub fn fingerprint(bytes: &[u8]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    // La taille dans le nom : deux fichiers de tailles différentes ne
    // peuvent pas se retrouver dans le même, quelle que soit la chance
    // du hachage.
    format!("{:016x}-{}", hash, bytes.len())
}

/// Le fichier de notes d'un binaire, dans le dossier donné.
pub fn path_in(dir: &Path, fingerprint: &str) -> PathBuf {
    dir.join(format!("{fingerprint}.json"))
}

/// Lit les notes d'un binaire. Des notes vides si le fichier n'existe
/// pas — c'est un binaire qu'on ouvre pour la première fois, non une
/// erreur.
pub fn load(dir: &Path, fingerprint: &str) -> Notes {
    std::fs::read_to_string(path_in(dir, fingerprint))
        .ok()
        .and_then(|texte| serde_json::from_str(&texte).ok())
        .unwrap_or_default()
}

/// Écrit les notes. Efface le fichier quand il n'y a plus rien à garder :
/// un dossier qui se remplit de fichiers vides finit par cacher ceux qui
/// comptent.
pub fn save(dir: &Path, fingerprint: &str, notes: &Notes) -> std::io::Result<()> {
    let chemin = path_in(dir, fingerprint);
    if notes.is_empty() {
        // Pas d'erreur si le fichier n'était pas là : on voulait qu'il
        // n'y soit pas, et c'est le cas.
        if chemin.exists() {
            std::fs::remove_file(&chemin)?;
        }
        return Ok(());
    }
    let json =
        serde_json::to_string_pretty(notes).map_err(|e| std::io::Error::other(e.to_string()))?;
    // Atomique : ce sont les noms que l'analyste a donnés aux fonctions,
    // parfois des heures de lecture. Les reperdre sur une écriture coupée
    // serait le pire des trois.
    crate::write_atomically(&chemin, json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Poser un nom, le corriger, l'effacer.
    #[test]
    fn a_name_is_set_corrected_and_cleared() {
        let mut notes = Notes::default();
        assert!(notes.is_empty());

        notes.set_name(0x1d491, "verifie_mdp");
        assert_eq!(notes.name(0x1d491), Some("verifie_mdp"));
        assert!(!notes.is_empty());

        notes.set_name(0x1d491, "  verifie_le_mot_de_passe  ");
        assert_eq!(
            notes.name(0x1d491),
            Some("verifie_le_mot_de_passe"),
            "les blancs autour ne font pas partie du nom"
        );

        // Vider le champ efface : on retire un nom comme on le corrige,
        // et demander une commande séparée ferait chercher.
        notes.set_name(0x1d491, "   ");
        assert_eq!(notes.name(0x1d491), None);
        assert!(notes.is_empty());
    }

    /// Les commentaires suivent la même règle.
    #[test]
    fn comments_follow_the_same_rule() {
        let mut notes = Notes::default();
        notes.set_comment(0x401000, "le canari est lu ici");
        assert_eq!(notes.comment(0x401000), Some("le canari est lu ici"));
        notes.set_comment(0x401000, "");
        assert!(notes.is_empty());
    }

    /// L'empreinte suit le contenu, non le chemin.
    ///
    /// C'est ce qui fait qu'un binaire déplacé retrouve ses notes, et
    /// qu'un binaire recompilé n'hérite pas de celles d'un autre.
    #[test]
    fn the_fingerprint_follows_the_content() {
        let a = fingerprint(b"\x7fELF ... un programme");
        let b = fingerprint(b"\x7fELF ... un programme");
        assert_eq!(a, b, "le même contenu donne la même empreinte");

        let autre = fingerprint(b"\x7fELF ... un autre programme");
        assert_ne!(a, autre);

        // Deux contenus de même longueur mais différents.
        assert_ne!(fingerprint(b"aaaa"), fingerprint(b"aaab"));
        // Et la longueur entre dans le nom : deux tailles différentes ne
        // peuvent pas se retrouver dans le même fichier.
        assert!(fingerprint(b"aaaa").ends_with("-4"));
        assert!(fingerprint(b"").ends_with("-0"));
    }

    /// L'empreinte ne change pas d'une version à l'autre.
    ///
    /// Elle est écrite à la main pour cela : `DefaultHasher` ne le
    /// garantit pas, et des notes qui changent de fichier à la prochaine
    /// mise à jour de Rust seraient perdues sans que rien ne le dise. La
    /// valeur est donc gravée ici — si ce test tombe, c'est que la
    /// formule a bougé, et que les notes de tout le monde avec.
    #[test]
    fn the_fingerprint_never_changes_between_versions() {
        assert_eq!(fingerprint(b"namsdec"), "a0f9f657318cfc9c-7");
        assert_eq!(fingerprint(b""), "cbf29ce484222325-0");
    }

    /// Les notes font l'aller-retour par le disque.
    #[test]
    fn notes_survive_a_trip_through_the_disk() {
        let dossier = tempfile::tempdir().unwrap();
        let empreinte = fingerprint(b"un binaire");

        // Rien encore : ce n'est pas une erreur, c'est un binaire qu'on
        // ouvre pour la première fois.
        assert!(load(dossier.path(), &empreinte).is_empty());

        let mut notes = Notes::default();
        notes.set_name(0x1d491, "vérifie_mdp");
        notes.set_comment(0x1d4a0, "compare octet par octet");
        save(dossier.path(), &empreinte, &notes).unwrap();

        let relues = load(dossier.path(), &empreinte);
        assert_eq!(relues, notes);
        assert_eq!(relues.name(0x1d491), Some("vérifie_mdp"));

        // Et vidées, le fichier disparaît : un dossier qui se remplit de
        // fichiers vides finit par cacher ceux qui comptent.
        save(dossier.path(), &empreinte, &Notes::default()).unwrap();
        assert!(!path_in(dossier.path(), &empreinte).exists());
    }

    /// Un fichier illisible ne fait pas tomber l'ouverture.
    ///
    /// Notes à demi écrites, disque plein, fichier tripoté à la main :
    /// on repart de zéro plutôt que de refuser d'ouvrir le binaire. On
    /// perd des annotations, ce qui est fâcheux ; refuser de travailler
    /// serait pire.
    #[test]
    fn a_broken_file_does_not_stop_the_binary_from_opening() {
        let dossier = tempfile::tempdir().unwrap();
        let empreinte = fingerprint(b"x");
        std::fs::write(path_in(dossier.path(), &empreinte), "{ pas du json").unwrap();
        assert!(load(dossier.path(), &empreinte).is_empty());
    }
}
