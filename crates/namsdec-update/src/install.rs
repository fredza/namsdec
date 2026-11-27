//! Prendre la version publiée et se remplacer par elle — **après
//! accord**.
//!
//! Longtemps ce crate n'a su que REGARDER : demander à GitHub la
//! dernière version, la comparer à la sienne, dire ce qu'il en est, et
//! s'arrêter là. L'en-tête du module disait pourquoi — « un atelier qui
//! se réécrit tout seul pendant qu'on travaille dedans n'est pas un
//! atelier ». Cette phrase visait l'automatisme, pas le service : ce
//! qui était refusé, c'est qu'un programme se remplace **sans qu'on le
//! lui ait demandé**.
//!
//! Ce module fait donc le travail, et rien ne s'y déclenche seul :
//!
//! 1. personne ne télécharge quoi que ce soit avant un clic qui dit
//!    oui, après avoir lu ce qui va se passer ;
//! 2. la somme SHA-256 publiée à côté du binaire est vérifiée **avant**
//!    que quoi que ce soit ne bouge ;
//! 3. ce qui est installé doit être un exécutable ELF, pas une page
//!    d'erreur arrivée avec le code 200 ;
//! 4. l'ancienne version est gardée à côté de la neuve ;
//! 5. et rien n'est remplacé en place : le fichier neuf est écrit à
//!    côté, puis **renommé** par-dessus l'ancien.
//!
//! **Ce que la somme prouve, et ce qu'elle ne prouve pas.** Elle vient
//! de la même publication que le binaire : elle démontre que les
//! octets sont arrivés entiers, pas qu'ils sont honnêtes. Ce qui tient
//! la provenance ici, c'est TLS et le compte GitHub du dépôt — le jour
//! où les publications seront signées, c'est la signature qui prendra
//! ce rôle, et c'est la place prévue pour elle.

use std::path::{Path, PathBuf};

use crate::Release;

/// Un fichier attaché à une version publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    /// L'adresse de téléchargement directe.
    pub url: String,
    /// Sa taille annoncée, en octets. Elle sert à dire ce qu'on
    /// s'apprête à prendre *avant* de le prendre.
    pub size: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// Cette publication ne porte pas de binaire pour cette machine.
    #[error("this release has no file named {0}")]
    NoAsset(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("HTTP {0}")]
    Status(u16),
    /// Le fichier `.sha256` n'a pas la forme que `sha256sum` écrit.
    #[error("the published checksum is unreadable")]
    BadChecksumFile,
    /// Les octets reçus ne sont pas ceux qui étaient annoncés.
    #[error("checksum mismatch: expected {expected}, got {got}")]
    Checksum { expected: String, got: String },
    /// Ce qui est arrivé n'est pas un exécutable.
    #[error("what was downloaded is not an executable")]
    NotAnExecutable,
    /// Le dossier d'installation ne s'écrit pas : paquet d'une
    /// distribution, binaire posé par root, disque en lecture seule.
    #[error("{0} cannot be written to")]
    ReadOnly(PathBuf),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// Arrêté par celui qui l'avait demandé.
    #[error("cancelled")]
    Cancelled,
}

/// Le nom du binaire publié pour une version, sur cette machine.
///
/// Un seul couple pour l'instant — c'est ce que `release.yml`
/// construit, et prétendre en servir d'autres ferait chercher des
/// fichiers qui n'existent pas. Ailleurs, [`binary_asset`] ne trouve
/// rien et l'interface propose le lien du navigateur.
pub fn asset_name(version: &str) -> String {
    format!("namsdec-{version}-{}-{}", std::env::consts::ARCH, "linux")
}

/// Le binaire de cette version, parmi ses fichiers attachés.
pub fn binary_asset(release: &Release) -> Option<&Asset> {
    let attendu = asset_name(&release.version);
    release.assets.iter().find(|asset| asset.name == attendu)
}

/// Et la somme qui l'accompagne : le même nom, suivi de `.sha256`.
pub fn checksum_asset(release: &Release) -> Option<&Asset> {
    let attendu = format!("{}.sha256", asset_name(&release.version));
    release.assets.iter().find(|asset| asset.name == attendu)
}

/// Lit un fichier `.sha256` tel que `sha256sum` l'écrit :
/// `<64 chiffres hexadécimaux>  <nom du fichier>`.
///
/// Seul le premier mot compte, et il doit faire exactement
/// soixante-quatre caractères hexadécimaux. Une page d'erreur HTML,
/// une redirection rendue en texte ou un fichier vide tombent ici —
/// et c'est voulu : une somme qu'on n'a pas su lire ne doit jamais
/// valoir « bon ».
pub fn parse_checksum(texte: &str) -> Option<[u8; 32]> {
    let premier = texte.split_whitespace().next()?;
    if premier.len() != 64 || !premier.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, place) in out.iter_mut().enumerate() {
        *place = u8::from_str_radix(&premier[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Est-ce un exécutable ELF 64 bits ?
///
/// Quatre octets de magie, puis la classe et le type. C'est un garde-fou
/// contre ce qui ARRIVE SANS ERREUR sans être le fichier voulu : une
/// page de connexion d'un portail Wi-Fi, un message d'erreur du proxy,
/// un objet vide. La somme l'attraperait aussi — mais elle n'est connue
/// que si le `.sha256` a pu être lu, et ce contrôle-ci ne dépend de
/// rien.
pub fn looks_like_elf(début: &[u8]) -> bool {
    if début.len() < 18 || &début[..4] != b"\x7fELF" {
        return false;
    }
    // Classe 2 : 64 bits. Le type vit sur deux octets en 16..18, et
    // vaut 2 (exécutable) ou 3 (objet partagé — ce qu'un binaire Rust
    // lié en PIE est, et c'est le cas normal ici).
    let type_ = u16::from_le_bytes([début[16], début[17]]);
    début[4] == 2 && (type_ == 2 || type_ == 3)
}

/// Le programme qui tourne, dont c'est le fichier.
pub fn current_exe() -> Result<PathBuf, InstallError> {
    std::env::current_exe().map_err(InstallError::Io)
}

/// Peut-on remplacer ce fichier-ci ?
///
/// C'est le DOSSIER qu'on interroge, et non le fichier : remplacer un
/// programme, c'est écrire un nouveau fichier à côté puis le renommer
/// par-dessus, et les deux gestes demandent le droit d'écrire dans le
/// dossier. Un binaire posé par la distribution dans `/usr/bin` y
/// échoue, et c'est exactement le cas où la mise à jour n'a pas à se
/// proposer : c'est le gestionnaire de paquets qui s'en charge.
///
/// Éprouvé en écrivant pour de bon, et non lu dans les permissions :
/// un dossier peut être à nous, marqué accessible en écriture, et
/// refuser quand même — montage en lecture seule, attribut immuable,
/// politique du noyau. Le seul test qui ne mente pas est l'essai.
pub fn replaceable(cible: &Path) -> Result<(), InstallError> {
    let dossier = cible
        .parent()
        .ok_or_else(|| InstallError::ReadOnly(cible.to_path_buf()))?;
    let essai = dossier.join(format!(".namsdec-essai-{}", std::process::id()));
    match std::fs::File::create(&essai) {
        Ok(_) => {
            let _ = std::fs::remove_file(&essai);
            Ok(())
        }
        Err(_) => Err(InstallError::ReadOnly(dossier.to_path_buf())),
    }
}

/// Où le fichier neuf est écrit en attendant d'être mis en place.
///
/// **Dans le dossier de destination**, et non dans `/tmp` : un renommage
/// n'est atomique qu'à l'intérieur d'un même système de fichiers, et
/// `/tmp` en est souvent un autre. Le nom commence par un point et
/// porte le numéro du processus — deux namsdec qui se mettraient à jour
/// en même temps ne se marcheraient pas dessus.
pub fn staging_path(cible: &Path) -> PathBuf {
    let dossier = cible.parent().unwrap_or(Path::new("."));
    let nom = cible
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "namsdec".to_owned());
    dossier.join(format!(".{nom}.neuf.{}", std::process::id()))
}

/// Et où l'ancienne version est gardée.
pub fn backup_path(cible: &Path) -> PathBuf {
    let mut nom = cible
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "namsdec".to_owned());
    nom.push_str(".ancien");
    cible.parent().unwrap_or(Path::new(".")).join(nom)
}

/// Met le fichier neuf à la place de l'ancien, en gardant celui-ci.
///
/// **L'ancien est COPIÉ, puis le neuf est RENOMMÉ par-dessus.** L'ordre
/// compte : un renommage est atomique, de sorte qu'à aucun instant la
/// place du programme ne contient autre chose que l'ancien ou le
/// nouveau — jamais un fichier à moitié écrit, même si le courant
/// s'arrête au milieu. Et si le renommage échoue, rien n'a bougé.
///
/// **On n'écrit pas DANS le fichier qui tourne.** Le noyau le refuse
/// (`ETXTBSY`) tant que le programme est en mémoire. Le renommer, lui,
/// est permis : le processus en cours garde son inode ouvert et
/// continue de tourner sur l'ancien code jusqu'à ce qu'on le relance.
/// C'est ce qui permet de se mettre à jour sans se fermer d'abord.
pub fn install_over(neuf: &Path, cible: &Path) -> Result<PathBuf, InstallError> {
    let sauvegarde = backup_path(cible);
    if cible.exists() {
        std::fs::copy(cible, &sauvegarde)?;
    }
    rendre_exécutable(neuf)?;
    if let Err(souci) = std::fs::rename(neuf, cible) {
        // Rien n'a été remplacé : la cible n'a pas été touchée. On
        // retire seulement ce qu'on avait posé.
        let _ = std::fs::remove_file(neuf);
        return Err(InstallError::Io(souci));
    }
    Ok(sauvegarde)
}

/// Les droits d'un programme : lisible et exécutable par tous,
/// modifiable par son propriétaire.
fn rendre_exécutable(fichier: &Path) -> Result<(), InstallError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(fichier, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    let _ = fichier;
    Ok(())
}

/// Ce qu'un téléchargement raconte pendant qu'il dure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub reçus: u64,
    /// Ce que le serveur annonce, quand il l'annonce.
    pub total: Option<u64>,
}

#[cfg(feature = "reseau")]
mod réseau {
    use super::*;
    use std::io::{Read, Write};

    /// Combien d'octets on lit d'un coup. Assez gros pour ne pas
    /// appeler le noyau à chaque poignée d'octets, assez petit pour
    /// que l'arrêt demandé soit pris en compte tout de suite.
    const MORCEAU: usize = 64 * 1024;

    fn agent() -> ureq::Agent {
        // Le même agent que la recherche de version, et le même
        // silence : la requête ne porte rien sur la machine.
        ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(300)))
            .user_agent(concat!("namsdec/", env!("CARGO_PKG_VERSION")))
            .build()
            .into()
    }

    fn souci(e: ureq::Error) -> InstallError {
        match e {
            ureq::Error::StatusCode(code) => InstallError::Status(code),
            autre => InstallError::Network(autre.to_string()),
        }
    }

    /// Lit le petit fichier de somme publié à côté du binaire.
    pub fn fetch_checksum(url: &str) -> Result<[u8; 32], InstallError> {
        let mut réponse = agent().get(url).call().map_err(souci)?;
        let texte = réponse
            .body_mut()
            .read_to_string()
            .map_err(|e| InstallError::Network(e.to_string()))?;
        parse_checksum(&texte).ok_or(InstallError::BadChecksumFile)
    }

    /// Télécharge le binaire vers `vers`, en vérifiant la somme **au
    /// fil de l'eau**.
    ///
    /// L'empreinte se calcule sur ce qui passe, et non en relisant le
    /// fichier ensuite : vingt-trois mégaoctets ne sont ni gardés en
    /// mémoire ni relus. Si la somme ne tombe pas juste, le fichier
    /// est effacé avant de rendre la main — un binaire faux ne traîne
    /// pas à côté du bon.
    ///
    /// `avance` est appelée à chaque morceau ; qu'elle rende `false`
    /// arrête tout. C'est ainsi que le bouton « Arrêter » agit, sans
    /// qu'aucun fil n'ait à être tué.
    pub fn download(
        url: &str,
        attendu: [u8; 32],
        vers: &Path,
        avance: &mut dyn FnMut(Progress) -> bool,
    ) -> Result<(), InstallError> {
        let mut réponse = agent().get(url).call().map_err(souci)?;
        let total = réponse
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        let mut corps = réponse.body_mut().as_reader();
        let mut fichier = std::fs::File::create(vers)?;
        let mut empreinte = namsdec_core::digest::Sha256::new();
        let mut tampon = vec![0u8; MORCEAU];
        let mut reçus = 0u64;
        let mut début = Vec::new();

        let échoue = |souci: InstallError, vers: &Path| {
            let _ = std::fs::remove_file(vers);
            souci
        };

        loop {
            let lus = match corps.read(&mut tampon) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => return Err(échoue(InstallError::Network(e.to_string()), vers)),
            };
            if début.len() < 64 {
                début.extend_from_slice(&tampon[..lus.min(64)]);
            }
            if let Err(e) = fichier.write_all(&tampon[..lus]) {
                return Err(échoue(InstallError::Io(e), vers));
            }
            empreinte.update(&tampon[..lus]);
            reçus += lus as u64;
            if !avance(Progress { reçus, total }) {
                return Err(échoue(InstallError::Cancelled, vers));
            }
        }
        if let Err(e) = fichier.flush() {
            return Err(échoue(InstallError::Io(e), vers));
        }
        drop(fichier);

        let obtenu = empreinte.finish();
        if obtenu != attendu {
            return Err(échoue(
                InstallError::Checksum {
                    expected: namsdec_core::digest::hex(&attendu),
                    got: namsdec_core::digest::hex(&obtenu),
                },
                vers,
            ));
        }
        if !looks_like_elf(&début) {
            return Err(échoue(InstallError::NotAnExecutable, vers));
        }
        Ok(())
    }
}

#[cfg(feature = "reseau")]
pub use réseau::{download, fetch_checksum};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_checksum_file_is_read_the_way_sha256sum_writes_it() {
        let ligne = "f63ed0dd8d0e55b10ab6a131be270428dcfc25cb85d4aa1c204e33d51079baa5  namsdec-1.0.1-beta.1-x86_64-linux\n";
        let somme = parse_checksum(ligne).expect("une somme lisible");
        assert_eq!(
            namsdec_core::digest::hex(&somme),
            "f63ed0dd8d0e55b10ab6a131be270428dcfc25cb85d4aa1c204e33d51079baa5"
        );
        // Et ce qui n'en est pas une n'en devient pas une.
        assert!(parse_checksum("").is_none());
        assert!(parse_checksum("<!DOCTYPE html>").is_none(), "une page web");
        assert!(parse_checksum(&"ab".repeat(31)).is_none(), "trop court");
        assert!(
            parse_checksum(&format!("{}zz", "ab".repeat(31))).is_none(),
            "pas hexadécimal"
        );
    }

    /// Un ELF se reconnaît, et une page d'erreur arrivée avec le code
    /// 200 ne passe pas pour un programme.
    #[test]
    fn only_an_executable_passes_for_one() {
        let mut elf = vec![0u8; 64];
        elf[..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[16] = 3; // objet partagé : ce qu'un binaire Rust en PIE est
        assert!(looks_like_elf(&elf));
        elf[16] = 2; // exécutable
        assert!(looks_like_elf(&elf));

        elf[16] = 1; // objet relogeable : pas un programme qu'on lance
        assert!(!looks_like_elf(&elf));
        assert!(!looks_like_elf(b"<!DOCTYPE html><html>Not Found"));
        assert!(!looks_like_elf(b"\x7fELF"), "trop court pour qu'on sache");
        assert!(!looks_like_elf(&[]));
    }

    /// Le binaire et sa somme se retrouvent parmi les fichiers d'une
    /// version, et une version qui n'en porte pas se dit telle.
    #[test]
    fn the_binary_and_its_checksum_are_found_among_the_files() {
        let asset = |nom: &str| Asset {
            name: nom.to_owned(),
            url: format!("https://example.invalid/{nom}"),
            size: 10,
        };
        let nom = asset_name("1.0.1-beta.1");
        let release = Release {
            tag: "v1.0.1-beta.1".to_owned(),
            version: "1.0.1-beta.1".to_owned(),
            name: "namsdec 1.0.1-beta.1".to_owned(),
            url: String::new(),
            prerelease: true,
            assets: vec![
                asset("namsdec-1.0.1-beta.1-aarch64-macos"),
                asset(&nom),
                asset(&format!("{nom}.sha256")),
            ],
        };
        assert_eq!(
            binary_asset(&release).map(|a| a.name.as_str()),
            Some(&nom[..])
        );
        assert_eq!(
            checksum_asset(&release).map(|a| a.name.clone()),
            Some(format!("{nom}.sha256"))
        );

        // Une publication sans binaire pour cette machine : on ne
        // propose rien plutôt que de télécharger le fichier d'une
        // autre.
        let ailleurs = Release {
            assets: vec![asset("namsdec-1.0.1-beta.1-aarch64-macos")],
            ..release
        };
        assert!(binary_asset(&ailleurs).is_none());
    }

    /// L'installation remplace, garde l'ancien, et laisse les droits
    /// d'un programme.
    #[test]
    fn installing_replaces_the_file_and_keeps_the_old_one() {
        let dossier = tempfile::tempdir().expect("un dossier d'essai");
        let cible = dossier.path().join("namsdec");
        std::fs::write(&cible, b"ancienne version").unwrap();
        let neuf = staging_path(&cible);
        std::fs::write(&neuf, b"nouvelle version").unwrap();

        let sauvegarde = install_over(&neuf, &cible).expect("l'installation");
        assert_eq!(std::fs::read(&cible).unwrap(), b"nouvelle version");
        assert_eq!(std::fs::read(&sauvegarde).unwrap(), b"ancienne version");
        assert!(!neuf.exists(), "le fichier d'attente a été mis en place");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&cible).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755, "{mode:o}");
        }
    }

    /// Et un dossier où l'on ne peut pas écrire se dit tel, au lieu
    /// d'échouer au dernier moment.
    #[test]
    fn a_folder_one_cannot_write_to_says_so() {
        let dossier = tempfile::tempdir().expect("un dossier d'essai");
        let cible = dossier.path().join("namsdec");
        std::fs::write(&cible, b"programme").unwrap();
        assert!(replaceable(&cible).is_ok());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dossier.path(), std::fs::Permissions::from_mode(0o555))
                .unwrap();
            // ROOT IGNORE LES PERMISSIONS, et ce contrôle n'a alors
            // rien à mesurer. Le dire et passer vaut mieux que
            // d'échouer sur une machine où le test est bâti en root :
            // on le saurait par un rouge qui n'apprend rien.
            let en_root = std::fs::File::create(dossier.path().join("témoin")).is_ok();
            if !en_root {
                assert!(
                    matches!(replaceable(&cible), Err(InstallError::ReadOnly(_))),
                    "un dossier en lecture seule doit être refusé avant tout téléchargement"
                );
            }
            std::fs::set_permissions(dossier.path(), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
    }
}
