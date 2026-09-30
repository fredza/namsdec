//! Savoir s'il existe une version plus récente, et où la prendre.
//!
//! Le dépôt publie ses versions sur GitHub ; namsdec demande la
//! dernière, compare avec la sienne, et dit ce qu'il en est. Rien de
//! plus : **aucun téléchargement, aucune installation automatique**. Un
//! atelier qui se réécrit tout seul pendant qu'on travaille dedans
//! n'est pas un atelier, et la mise à jour d'un binaire installé à la
//! main se fait à la main — `install.sh` est là pour ça.
//!
//! Rien n'est envoyé : la requête ne porte ni identifiant, ni numéro de
//! version, ni quoi que ce soit sur la machine. Elle n'est faite que
//! lorsqu'on clique.

#[cfg(feature = "reseau")]
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Le dépôt d'où viennent les versions.
pub const OWNER: &str = "fredza";
pub const REPO: &str = "namsdec";

/// Ce que GitHub dit d'une version publiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Le nom de l'étiquette, tel quel : `v0.2.1-beta.2`.
    pub tag: String,
    /// Le numéro seul, sans le `v` : ce qui se compare.
    pub version: String,
    /// Le nom donné à la version, quand il y en a un.
    pub name: String,
    /// La page de la version, à ouvrir dans un navigateur.
    pub url: String,
    /// Une préversion au sens de GitHub — la case cochée en publiant.
    pub prerelease: bool,
}

/// Ce qu'on apprend en comparant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Rien de plus récent : c'est la version publiée la plus haute.
    UpToDate,
    /// Une version plus récente existe.
    Newer(Release),
    /// La version qui tourne est *plus haute* que la dernière publiée —
    /// le cas de quelqu'un qui compile le dépôt lui-même. Le dire est
    /// plus honnête que de prétendre qu'il est à jour.
    Ahead(Release),
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// La requête n'a pas abouti : pas de réseau, GitHub injoignable,
    /// délai dépassé.
    #[error("network error: {0}")]
    Network(String),
    /// GitHub a répondu autre chose qu'un succès.
    #[error("HTTP {0}")]
    Status(u16),
    /// Ni dépôt ni version à cette adresse.
    ///
    /// C'est aussi ce que GitHub répond d'un dépôt **privé** interrogé
    /// sans jeton : de l'extérieur, un dépôt qu'on n'a pas le droit de
    /// voir et un dépôt qui n'existe pas se ressemblent — et c'est
    /// voulu. Le dire ainsi évite de chercher une panne là où il n'y a
    /// qu'une porte fermée.
    #[error("repository or release not found (a private repository answers this too)")]
    NotFound,
    /// La réponse n'a pas la forme attendue.
    #[error("unexpected response")]
    Malformed,
    /// Le dépôt n'a encore rien publié.
    #[error("no release published")]
    NoRelease,
}

/// Combien de temps on attend avant d'abandonner.
///
/// Court : la réponse n'est pas ce pour quoi on a ouvert le programme,
/// et une attente de trente secondes ferait croire à un gel.
#[cfg(feature = "reseau")]
const TIMEOUT: Duration = Duration::from_secs(8);

/// L'adresse de l'API pour les versions publiées.
///
/// La liste, et non `releases/latest` : **ce dernier ignore les
/// préversions**. Tant qu'un dépôt ne publie que des bêtas, il répond
/// 404 comme s'il n'avait jamais rien publié — ce qui est exactement ce
/// qui est arrivé ici, et ce qui aurait fait dire au programme qu'il
/// n'existait aucune version alors qu'elle était là.
pub fn releases_url(owner: &str, repo: &str) -> String {
    format!("https://api.github.com/repos/{owner}/{repo}/releases?per_page=20")
}

/// Demande à GitHub la dernière version publiée.
///
/// `allow_prerelease` dit si les bêtas comptent : qui en fait tourner
/// une veut savoir qu'une bêta plus récente existe ; qui a une version
/// définitive n'a pas à se voir proposer un essai.
///
/// Bloquant : à appeler depuis un thread, jamais depuis la boucle de
/// rendu. Rien n'est envoyé d'autre que la requête elle-même.
#[cfg(feature = "reseau")]
pub fn fetch_latest(
    owner: &str,
    repo: &str,
    allow_prerelease: bool,
) -> Result<Release, UpdateError> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        // GitHub refuse les requêtes sans agent, et le nôtre dit qui
        // demande — sans rien de plus sur la machine.
        .user_agent(concat!("namsdec/", env!("CARGO_PKG_VERSION")))
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .get(&releases_url(owner, repo))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => UpdateError::NotFound,
            ureq::Error::StatusCode(code) => UpdateError::Status(code),
            autre => UpdateError::Network(autre.to_string()),
        })?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    pick_latest(&parse_releases(&body)?, allow_prerelease).ok_or(UpdateError::NoRelease)
}

/// Lit la liste des versions publiées.
///
/// Les brouillons sont écartés : ils ne sont visibles que de ceux qui
/// peuvent les publier, et proposer une version que personne ne peut
/// prendre n'aide pas.
pub fn parse_releases(body: &str) -> Result<Vec<Release>, UpdateError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| UpdateError::Malformed)?;
    let liste = value.as_array().ok_or(UpdateError::Malformed)?;
    Ok(liste
        .iter()
        .filter(|entrée| {
            !entrée
                .get("draft")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .filter_map(|entrée| read_release(entrée).ok())
        .collect())
}

/// La plus haute des versions, selon ce qu'on accepte.
pub fn pick_latest(releases: &[Release], allow_prerelease: bool) -> Option<Release> {
    releases
        .iter()
        // Une préversion n'est proposée qu'à qui en fait déjà tourner
        // une : on ne pousse personne vers un essai sans le lui avoir
        // demandé.
        .filter(|release| allow_prerelease || !release.prerelease)
        .max_by(|a, b| compare(&a.version, &b.version))
        .cloned()
}

/// Lit la réponse de GitHub.
///
/// Séparée de la requête pour être vérifiable sans réseau : c'est ici
/// que se trouve tout ce qui peut mal tourner une fois les octets
/// arrivés.
pub fn parse_release(body: &str) -> Result<Release, UpdateError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| UpdateError::Malformed)?;
    read_release(&value)
}

/// Une version, lue dans l'objet que GitHub en donne.
fn read_release(value: &serde_json::Value) -> Result<Release, UpdateError> {
    // GitHub répond 404 avec un corps JSON quand rien n'est publié : la
    // forme est correcte, l'étiquette manque.
    let tag = value
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or(UpdateError::NoRelease)?
        .to_owned();
    let version = tag.trim_start_matches(['v', 'V']).to_owned();
    if version.is_empty() {
        return Err(UpdateError::Malformed);
    }
    Ok(Release {
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&tag)
            .to_owned(),
        url: value
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned(),
        prerelease: value
            .get("prerelease")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        tag,
        version,
    })
}

/// Compare la version qui tourne à celle qui est publiée.
pub fn verdict(current: &str, latest: Release) -> Verdict {
    match compare(&latest.version, current) {
        std::cmp::Ordering::Greater => Verdict::Newer(latest),
        std::cmp::Ordering::Equal => Verdict::UpToDate,
        std::cmp::Ordering::Less => Verdict::Ahead(latest),
    }
}

/// L'ordre SemVer de deux numéros de version.
///
/// Majeur, mineur, correctif, puis la préversion — et c'est là que les
/// comparaisons naïves se trompent : `0.2.1` est **plus récente** que
/// `0.2.1-beta.2`, et `beta.10` vient après `beta.2` parce qu'un
/// identifiant tout en chiffres se compare comme un nombre.
pub fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    let (a_nombres, a_pre) = split(a);
    let (b_nombres, b_pre) = split(b);
    match a_nombres.cmp(&b_nombres) {
        std::cmp::Ordering::Equal => compare_prerelease(a_pre.as_deref(), b_pre.as_deref()),
        autre => autre,
    }
}

/// `1.2.3-beta.2+build` → `([1, 2, 3], Some("beta.2"))`.
///
/// Les champs manquants valent zéro : `0.2` se compare comme `0.2.0`,
/// ce qu'on écrit couramment dans une étiquette.
fn split(version: &str) -> ([u64; 3], Option<String>) {
    let sans_build = version.split('+').next().unwrap_or(version);
    let (nombres, pre) = match sans_build.split_once('-') {
        Some((nombres, pre)) => (nombres, Some(pre.to_owned())),
        None => (sans_build, None),
    };
    let mut out = [0u64; 3];
    for (index, morceau) in nombres.split('.').take(3).enumerate() {
        out[index] = morceau.trim().parse().unwrap_or(0);
    }
    (out, pre)
}

/// L'ordre de deux préversions, `None` valant « version définitive ».
fn compare_prerelease(a: Option<&str>, b: Option<&str>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (None, None) => Ordering::Equal,
        // Une version sans préversion l'emporte : `1.0.0` vient après
        // `1.0.0-rc.1`, quoi qu'en dise l'ordre alphabétique.
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => {
            let mut a = a.split('.');
            let mut b = b.split('.');
            loop {
                match (a.next(), b.next()) {
                    (None, None) => return Ordering::Equal,
                    // Moins d'identifiants l'emporte : `beta` vient
                    // avant `beta.1`.
                    (None, Some(_)) => return Ordering::Less,
                    (Some(_), None) => return Ordering::Greater,
                    (Some(x), Some(y)) => {
                        let ordre = match (x.parse::<u64>(), y.parse::<u64>()) {
                            // Deux nombres se comparent comme des
                            // nombres : `beta.10` après `beta.2`.
                            (Ok(x), Ok(y)) => x.cmp(&y),
                            // Un nombre vient avant du texte.
                            (Ok(_), Err(_)) => Ordering::Less,
                            (Err(_), Ok(_)) => Ordering::Greater,
                            (Err(_), Err(_)) => x.cmp(y),
                        };
                        if ordre != Ordering::Equal {
                            return ordre;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    /// L'ordre des versions, y compris là où une comparaison de chaînes
    /// se tromperait.
    #[test]
    fn versions_compare_the_way_semver_says() {
        assert_eq!(compare("0.2.1", "0.2.0"), Ordering::Greater);
        assert_eq!(compare("0.3.0", "0.2.9"), Ordering::Greater);
        assert_eq!(compare("1.0.0", "0.99.99"), Ordering::Greater);
        assert_eq!(compare("0.2.1", "0.2.1"), Ordering::Equal);
        // Le piège : une version définitive vient après sa préversion.
        assert_eq!(compare("0.2.1", "0.2.1-beta.2"), Ordering::Greater);
        assert_eq!(compare("0.2.1-beta.2", "0.2.1"), Ordering::Less);
        // Et les préversions entre elles se comparent par identifiants,
        // les nombres comme des nombres.
        assert_eq!(compare("0.2.1-beta.10", "0.2.1-beta.2"), Ordering::Greater);
        assert_eq!(compare("0.2.1-beta.2", "0.2.1-alpha.9"), Ordering::Greater);
        assert_eq!(compare("0.2.1-rc.1", "0.2.1-beta.9"), Ordering::Greater);
        assert_eq!(compare("0.2.1-beta", "0.2.1-beta.1"), Ordering::Less);
        // La métadonnée de compilation ne compte pas.
        assert_eq!(compare("0.2.1+a", "0.2.1+b"), Ordering::Equal);
        // Un numéro écrit court se lit quand même.
        assert_eq!(compare("0.3", "0.2.9"), Ordering::Greater);
    }

    /// La réponse de GitHub, telle qu'elle arrive.
    #[test]
    fn a_release_is_read_from_what_github_answers() {
        let body = r#"{
            "tag_name": "v0.3.0",
            "name": "namsdec 0.3.0",
            "html_url": "https://github.com/fredza/namsdec/releases/tag/v0.3.0",
            "prerelease": false,
            "assets": []
        }"#;
        let release = parse_release(body).expect("une version lisible");
        assert_eq!(release.tag, "v0.3.0");
        // Le `v` de l'étiquette ne fait pas partie du numéro.
        assert_eq!(release.version, "0.3.0");
        assert!(release.url.ends_with("/v0.3.0"));
        assert!(!release.prerelease);
    }

    /// Un 404 se dit pour ce qu'il est : personne à cette adresse — ou
    /// une porte fermée, ce qui se ressemble vu de l'extérieur.
    #[test]
    fn a_missing_repository_says_what_it_means() {
        let message = UpdateError::NotFound.to_string();
        assert!(message.contains("private"), "{message}");
    }

    /// Ce qui n'est pas une version ne devient pas une version.
    #[test]
    fn anything_else_is_refused_rather_than_guessed() {
        // Le corps que GitHub rend quand rien n'est publié.
        let vide = r#"{"message":"Not Found","status":"404"}"#;
        assert!(matches!(parse_release(vide), Err(UpdateError::NoRelease)));
        assert!(matches!(
            parse_release("pas du json"),
            Err(UpdateError::Malformed)
        ));
        assert!(matches!(
            parse_release(r#"{"tag_name":"v"}"#),
            Err(UpdateError::Malformed)
        ));
    }

    /// Les trois réponses possibles, y compris celle qu'on oublie :
    /// tourner *devant* la dernière version publiée, ce qui arrive à
    /// quiconque compile le dépôt.
    #[test]
    fn the_verdict_says_which_of_the_three_it_is() {
        let release = |v: &str| Release {
            tag: format!("v{v}"),
            version: v.to_owned(),
            name: v.to_owned(),
            url: String::new(),
            prerelease: v.contains('-'),
        };
        assert!(matches!(
            verdict("0.2.1-beta.2", release("0.3.0")),
            Verdict::Newer(_)
        ));
        assert_eq!(
            verdict("0.2.1-beta.2", release("0.2.1-beta.2")),
            Verdict::UpToDate
        );
        assert!(matches!(
            verdict("0.3.0", release("0.2.1-beta.2")),
            Verdict::Ahead(_)
        ));
    }

    /// La liste des versions, telle que GitHub la rend — et la règle
    /// qui décide laquelle proposer.
    ///
    /// `releases/latest` ne suffisait pas : il **ignore les
    /// préversions**, et répond 404 pour un dépôt qui n'a publié que des
    /// bêtas. Le programme aurait dit « aucune version publiée » devant
    /// une version bien présente.
    #[test]
    fn a_list_of_releases_is_read_and_the_right_one_chosen() {
        // La forme exacte que rend l'API, telle qu'on l'a relevée sur le
        // dépôt : un tableau, le plus récent en tête.
        let body = r#"[
            {"tag_name":"v0.3.0-beta.1","prerelease":true,"draft":false,
             "name":"beta 1","html_url":"https://example.invalid/b1"},
            {"tag_name":"v0.2.9","prerelease":false,"draft":false,
             "name":"0.2.9","html_url":"https://example.invalid/029"},
            {"tag_name":"v9.9.9","prerelease":false,"draft":true,
             "name":"brouillon","html_url":"https://example.invalid/draft"}
        ]"#;
        let liste = parse_releases(body).expect("une liste lisible");
        // Le brouillon est écarté : personne d'autre ne peut le prendre.
        assert_eq!(liste.len(), 2, "{liste:?}");

        // Qui fait tourner une bêta veut savoir qu'une bêta plus récente
        // existe.
        let pour_beta = pick_latest(&liste, true).expect("une version");
        assert_eq!(pour_beta.tag, "v0.3.0-beta.1");
        // Qui a une version définitive ne se voit pas proposer d'essai.
        let pour_stable = pick_latest(&liste, false).expect("une version");
        assert_eq!(pour_stable.tag, "v0.2.9");

        // Et une liste vide ne rend rien plutôt que d'inventer.
        assert!(pick_latest(&[], true).is_none());
        assert!(parse_releases("{}").is_err());
    }

    /// L'adresse interrogée est la liste, pas `latest` — et elle demande
    /// une page bornée.
    #[test]
    fn the_address_asks_for_the_list() {
        let url = releases_url(OWNER, REPO);
        assert!(url.starts_with("https://api.github.com/repos/fredza/namsdec/releases"));
        assert!(url.contains("per_page="), "{url}");
        assert!(!url.contains("/latest"), "{url}");
    }
}
