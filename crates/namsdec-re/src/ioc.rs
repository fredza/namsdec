//! Les indicateurs : ce qu'un chercheur recopiera dans son rapport.
//!
//! Le panneau des chaînes **liste** ; il ne **qualifie** pas. Or parmi
//! deux cents chaînes, cinq comptent : l'adresse qu'un programme
//! contacte, le chemin qu'il écrit, la clé de registre qu'il pose. Ce
//! sont les « indicateurs de compromission » — IoC dans la littérature —
//! et c'est la première chose qu'on cherche dans un échantillon, parce
//! que c'est la seule qu'on puisse aller vérifier ailleurs : dans des
//! journaux réseau, sur un autre poste, dans une base publique.
//!
//! Le module est de la pure lecture : il ne contacte rien, ne résout
//! aucun nom, ne juge personne. Il reconnaît des **formes** dans du
//! texte déjà trouvé.
//!
//! ## Ce qu'il ne promet pas
//!
//! Une forme n'est pas une intention. `/etc/passwd` dans un binaire peut
//! être un message d'aide ; `8.8.8.8` peut être un exemple dans une page
//! de manuel compilée. Les règles sont donc écrites pour **se taire
//! plutôt que de se tromper** : une liste de domaines de premier niveau
//! connus, et non « tout ce qui contient un point » — sans quoi
//! `libc.so.6` passerait pour un nom de domaine et la liste serait
//! inutilisable.

/// La nature d'un indicateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// Une adresse complète, avec son protocole.
    Url,
    /// Une adresse IPv4 en quatre nombres.
    Ip,
    /// Un nom d'hôte, sans protocole.
    Domain,
    /// Une adresse de courrier.
    Email,
    /// Un chemin absolu dans le système de fichiers.
    Path,
    /// Une clé de registre Windows.
    RegistryKey,
}

/// Les natures dans l'ordre où on les lit : du plus parlant au plus
/// banal.
///
/// Une URL désigne quelque chose d'extérieur et de vérifiable ; un
/// chemin absolu se trouve dans tous les programmes du monde. C'est donc
/// l'URL d'abord.
pub const ALL: [Kind; 6] = [
    Kind::Url,
    Kind::Ip,
    Kind::Domain,
    Kind::Email,
    Kind::Path,
    Kind::RegistryKey,
];

impl Kind {
    pub fn label(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match (self, fr) {
            (Kind::Url, true) => "Adresse web",
            (Kind::Url, false) => "URL",
            (Kind::Ip, true) => "Adresse IP",
            (Kind::Ip, false) => "IP address",
            (Kind::Domain, true) => "Nom de domaine",
            (Kind::Domain, false) => "Domain name",
            (Kind::Email, true) => "Courriel",
            (Kind::Email, false) => "Email",
            (Kind::Path, true) => "Chemin",
            (Kind::Path, false) => "Path",
            (Kind::RegistryKey, true) => "Clé de registre",
            (Kind::RegistryKey, false) => "Registry key",
        }
    }
}

/// Un indicateur trouvé : sa nature, son texte, et d'où il vient.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Indicator {
    pub kind: Kind,
    pub value: String,
    /// L'adresse de la chaîne qui le portait, pour y retourner.
    pub addr: u64,
}

/// Les protocoles qu'on reconnaît en tête d'URL.
const SCHEMES: &[&str] = &[
    "http://",
    "https://",
    "ftp://",
    "ftps://",
    "sftp://",
    "tftp://",
    "smb://",
    "file://",
    "ws://",
    "wss://",
    "ldap://",
    "irc://",
    "ssh://",
    "telnet://",
    "gopher://",
];

/// Les domaines de premier niveau qu'on accepte.
///
/// Une liste et non une règle, et c'est tout le sujet : « deux mots
/// séparés d'un point » décrit `libc.so.6`, `printf.c`, `a.out` et la
/// moitié des chaînes d'un binaire. La liste tient les cas qu'on
/// rencontre vraiment, et laisse passer le reste — se taire vaut mieux
/// que remplir la liste de bruit.
const TLDS: &[&str] = &[
    "com", "net", "org", "edu", "gov", "mil", "int", "info", "biz", "name", "pro", "io", "co",
    "me", "tv", "cc", "ru", "su", "cn", "jp", "kr", "in", "br", "mx", "de", "fr", "uk", "it", "es",
    "nl", "be", "ch", "at", "se", "no", "fi", "dk", "pl", "cz", "ro", "ua", "tr", "il", "ir", "pk",
    "vn", "th", "id", "my", "sg", "hk", "tw", "au", "nz", "ca", "us", "za", "ng", "ke", "eg", "ma",
    "dz", "tn", "sn", "ci", "cm", "xyz", "top", "site", "online", "club", "shop", "store", "tech",
    "cloud", "dev", "app", "ai", "sh", "gg", "ly", "to", "ws", "tk", "ml", "ga", "cf", "pw", "cat",
    "eu", "asia", "onion",
];

/// Les préfixes d'une clé de registre Windows.
const REGISTRY: &[&str] = &[
    "HKEY_LOCAL_MACHINE",
    "HKEY_CURRENT_USER",
    "HKEY_CLASSES_ROOT",
    "HKEY_USERS",
    "HKEY_CURRENT_CONFIG",
    "HKLM\\",
    "HKCU\\",
    "SOFTWARE\\Microsoft\\",
    "SYSTEM\\CurrentControlSet\\",
];

/// Un chemin plus court que cela n'en est pas un : `/a` est une option
/// de ligne de commande, un format d'affichage, un reste de quelque
/// chose. Deux segments nommés, c'est le minimum pour désigner un
/// endroit.
const MIN_PATH_SEGMENTS: usize = 2;

/// Les indicateurs portés par une chaîne.
///
/// Une même chaîne peut en porter plusieurs — un message de journal
/// contient souvent un chemin **et** une adresse — et chacun est rendu
/// séparément, parce que c'est séparément qu'on les recopie.
///
/// ```
/// use namsdec_re::ioc::{self, Kind};
/// let trouvés = ioc::in_text("GET http://exemple.ru/a vers 10.0.0.7", 0x1000);
/// let natures: Vec<Kind> = trouvés.iter().map(|i| i.kind).collect();
/// assert_eq!(natures, vec![Kind::Url, Kind::Ip]);
/// ```
pub fn in_text(text: &str, addr: u64) -> Vec<Indicator> {
    let mut out: Vec<Indicator> = Vec::new();
    let mut ajoute = |kind: Kind, value: String| {
        if !out.iter().any(|i| i.kind == kind && i.value == value) {
            out.push(Indicator { kind, value, addr });
        }
    };

    // Les URL d'abord, et on retient leur hôte : sans cela
    // `http://exemple.com/a` rendrait aussi le domaine `exemple.com`,
    // c'est-à-dire la même information deux fois.
    let mut hôtes: Vec<String> = Vec::new();
    for schéma in SCHEMES {
        let mut reste = text;
        while let Some(début) = reste.find(schéma) {
            let après = &reste[début + schéma.len()..];
            let fin = après
                .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '`'))
                .unwrap_or(après.len());
            let corps = après[..fin].trim_end_matches(['.', ',', ')', ';']);
            if !corps.is_empty() {
                // L'hôte : ce qui précède le premier `/`, débarrassé
                // d'un éventuel `utilisateur:mot@` et du port.
                let hôte: String = corps
                    .split(['/', '?', '#'])
                    .next()
                    .unwrap_or(corps)
                    .rsplit('@')
                    .next()
                    .unwrap_or(corps)
                    .split(':')
                    .next()
                    .unwrap_or(corps)
                    .to_owned();
                hôtes.push(hôte);
                ajoute(Kind::Url, format!("{schéma}{corps}"));
            }
            // On avance, toujours : `fin` est une frontière de
            // caractère (elle vient de `find`), mais elle vaut zéro
            // quand le schéma est suivi d'un séparateur — et rester sur
            // place tournerait sans fin. Un caractère entier alors, non
            // un octet : couper au milieu d'un `é` paniquerait, et une
            // chaîne tordue est exactement ce qu'un fichier hostile
            // contient.
            let avance = if fin > 0 {
                fin
            } else {
                match après.chars().next() {
                    Some(c) => c.len_utf8(),
                    None => break,
                }
            };
            reste = &après[avance..];
        }
    }

    for mot in mots(text) {
        if let Some(ip) = ipv4(mot) {
            ajoute(Kind::Ip, ip);
            // Une adresse n'est pas aussi un domaine : `1.2.3.4` ne doit
            // pas ressortir deux fois sous deux noms.
            continue;
        }
        if let Some(courriel) = email(mot) {
            ajoute(Kind::Email, courriel);
            continue;
        }
        if let Some(domaine) = domain(mot) {
            // Sauf s'il vient d'une URL déjà relevée.
            if !hôtes.iter().any(|h| h == &domaine) {
                ajoute(Kind::Domain, domaine);
            }
            continue;
        }
        if let Some(clé) = registry(mot) {
            ajoute(Kind::RegistryKey, clé);
            continue;
        }
        if let Some(chemin) = path(mot) {
            ajoute(Kind::Path, chemin);
        }
    }
    out
}

/// Les mots d'une chaîne, au sens de ce module.
///
/// On coupe sur l'espace et sur la ponctuation qui ne peut pas faire
/// partie d'un indicateur. Le point, le tiret, la barre oblique et la
/// barre inverse en font partie, eux : ce sont les caractères dont les
/// indicateurs sont faits.
fn mots(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '"' | '\''
                    | '`'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | ','
                    | ';'
                    | '|'
                    | '*'
                    | '<'
                    | '>'
                    | '='
            )
    })
    .map(|mot| mot.trim_matches(|c: char| matches!(c, '.' | ':' | '!' | '?')))
    .filter(|mot| !mot.is_empty())
}

/// Une adresse IPv4, si ce mot en est une.
///
/// Quatre nombres de 0 à 255 et rien d'autre autour. Un numéro de
/// version à quatre composantes a exactement la même forme, et c'est une
/// ambiguïté qu'aucune règle ne lève : le panneau dit donc « indicateur »
/// et non « adresse contactée », et c'est au lecteur de trancher.
fn ipv4(mot: &str) -> Option<String> {
    // Un port éventuel ne gêne pas : c'est l'adresse qu'on relève.
    let corps = mot.split(':').next()?;
    let parties: Vec<&str> = corps.split('.').collect();
    if parties.len() != 4 {
        return None;
    }
    for partie in &parties {
        if partie.is_empty() || partie.len() > 3 || !partie.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if partie.parse::<u16>().ok()? > 255 {
            return None;
        }
    }
    Some(corps.to_owned())
}

/// Une adresse de courrier, si ce mot en est une.
fn email(mot: &str) -> Option<String> {
    let (avant, après) = mot.split_once('@')?;
    if avant.is_empty() || avant.contains('/') || avant.contains('@') {
        return None;
    }
    domain(après).map(|_| mot.to_owned())
}

/// Un nom de domaine, si ce mot en est un.
///
/// Au moins deux étiquettes, la dernière dans [`TLDS`], et des
/// étiquettes faites de ce dont un nom d'hôte est fait. C'est la règle
/// stricte qui empêche `libc.so.6` d'apparaître comme un domaine.
fn domain(mot: &str) -> Option<String> {
    let corps = mot.split('/').next()?.trim_end_matches('.');
    let étiquettes: Vec<&str> = corps.split('.').collect();
    if étiquettes.len() < 2 {
        return None;
    }
    let tld = étiquettes.last()?.to_ascii_lowercase();
    if !TLDS.contains(&tld.as_str()) {
        return None;
    }
    for étiquette in &étiquettes {
        if étiquette.is_empty() || étiquette.len() > 63 {
            return None;
        }
        if !étiquette
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return None;
        }
    }
    // Une étiquette de premier niveau est faite de lettres ; `1.2.3.in`
    // ne serait pas un domaine, et un segment tout en chiffres juste
    // avant trahit presque toujours un numéro de version.
    if étiquettes[étiquettes.len() - 2]
        .bytes()
        .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some(corps.to_owned())
}

/// Une clé de registre Windows, si ce mot en est une.
fn registry(mot: &str) -> Option<String> {
    let majuscule = mot.to_ascii_uppercase();
    REGISTRY
        .iter()
        .any(|préfixe| majuscule.starts_with(&préfixe.to_ascii_uppercase()))
        .then(|| mot.to_owned())
}

/// Un chemin absolu, si ce mot en est un.
///
/// Les chemins Unix et les chemins Windows, avec leur lettre de lecteur.
/// Relatifs exclus : `a/b` est aussi bien un chemin qu'un morceau de
/// phrase, et relever les deux noierait la liste.
fn path(mot: &str) -> Option<String> {
    let unix = mot.starts_with('/');
    let windows = {
        let octets = mot.as_bytes();
        octets.len() > 3
            && octets[0].is_ascii_alphabetic()
            && octets[1] == b':'
            && (octets[2] == b'\\' || octets[2] == b'/')
    };
    if !unix && !windows {
        return None;
    }
    let segments = mot
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .count();
    // Le lecteur `C:` compte comme un segment : il faut donc un de plus.
    let minimum = if windows {
        MIN_PATH_SEGMENTS + 1
    } else {
        MIN_PATH_SEGMENTS
    };
    (segments >= minimum).then(|| mot.to_owned())
}

/// Les indicateurs portés par une liste de chaînes, sans doublon.
///
/// Une même URL apparaît souvent dans plusieurs chaînes ; elle n'est
/// relevée qu'une fois, avec la **première** adresse où on l'a vue —
/// c'est là qu'on ira lire. Rangés par nature, dans l'ordre de [`ALL`],
/// puis par texte : c'est la forme d'un rapport.
pub fn from_strings(strings: &[crate::strings::Found]) -> Vec<Indicator> {
    let mut out: Vec<Indicator> = Vec::new();
    for chaîne in strings {
        for indicateur in in_text(&chaîne.text, chaîne.addr) {
            if !out
                .iter()
                .any(|déjà| déjà.kind == indicateur.kind && déjà.value == indicateur.value)
            {
                out.push(indicateur);
            }
        }
    }
    out.sort_by(|a, b| {
        let rang = |k: Kind| ALL.iter().position(|n| *n == k).unwrap_or(ALL.len());
        rang(a.kind)
            .cmp(&rang(b.kind))
            .then_with(|| a.value.cmp(&b.value))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valeurs(text: &str, kind: Kind) -> Vec<String> {
        in_text(text, 0)
            .into_iter()
            .filter(|i| i.kind == kind)
            .map(|i| i.value)
            .collect()
    }

    /// Les cinq formes que le relevé demandait se reconnaissent.
    #[test]
    fn the_five_shapes_are_recognised() {
        assert_eq!(
            valeurs(
                "telecharge https://mauvais.example.ru/charge.bin maintenant",
                Kind::Url
            ),
            vec!["https://mauvais.example.ru/charge.bin"]
        );
        assert_eq!(
            valeurs("se connecte a 192.168.1.254:4444", Kind::Ip),
            vec!["192.168.1.254"]
        );
        assert_eq!(
            valeurs("resout c2.exemple.xyz", Kind::Domain),
            vec!["c2.exemple.xyz"]
        );
        assert_eq!(
            valeurs("ecrit /var/tmp/.cache/x", Kind::Path),
            vec!["/var/tmp/.cache/x"]
        );
        assert_eq!(
            valeurs(
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
                Kind::RegistryKey
            ),
            vec!["HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run"]
        );
        assert_eq!(
            valeurs("ecrire a admin@exemple.com", Kind::Email),
            vec!["admin@exemple.com"]
        );
    }

    /// Ce qui ressemble à un indicateur sans en être un reste dehors.
    ///
    /// C'est la moitié qui décide de l'utilité du module : une liste où
    /// `libc.so.6` passe pour un nom de domaine ne se lit pas, et on
    /// cesse de l'ouvrir.
    #[test]
    fn what_merely_looks_like_one_stays_out() {
        for texte in [
            "libc.so.6",
            "ld-linux-x86-64.so.2",
            "printf.c",
            "a.out",
            "monfichier.txt",
            "version 2.38.1",
            "Makefile.am",
            "GCC: (GNU) 14.2.1",
            "%s/%s",
            "/",
            "/a",
            "src/main.rs",
        ] {
            assert!(
                in_text(texte, 0).is_empty(),
                "« {texte} » n'est pas un indicateur : {:?}",
                in_text(texte, 0)
            );
        }
    }

    /// Un numéro de version à quatre composantes a la forme d'une
    /// adresse, et le module le dit comme tel.
    ///
    /// Aucune règle ne lève l'ambiguïté : `1.2.3.4` est un numéro de
    /// version pour l'un et une adresse pour l'autre. On le relève, et
    /// c'est le panneau qui dit « indicateur », non « adresse
    /// contactée » — se taire ferait manquer de vraies adresses, qui ont
    /// exactement cette forme.
    #[test]
    fn a_four_part_version_has_the_shape_of_an_address() {
        assert_eq!(valeurs("1.2.3.4", Kind::Ip), vec!["1.2.3.4"]);
        // Mais cinq composantes, non : ce n'est plus une adresse.
        assert!(in_text("1.2.3.4.5", 0).is_empty());
        // Ni un nombre hors bornes.
        assert!(in_text("256.1.1.1", 0).is_empty());
        assert!(in_text("10.0.0.300", 0).is_empty());
    }

    /// L'hôte d'une URL n'est pas relevé une seconde fois comme domaine.
    #[test]
    fn a_url_host_is_not_also_reported_as_a_domain() {
        let trouvés = in_text("POST http://exemple.com/envoi", 0);
        assert_eq!(trouvés.len(), 1, "{trouvés:?}");
        assert_eq!(trouvés[0].kind, Kind::Url);
    }

    /// Une chaîne peut porter plusieurs indicateurs, et chacun compte.
    #[test]
    fn one_string_can_carry_several() {
        let trouvés = in_text(
            "journal: envoi de /etc/shadow vers http://vol.example.ru/d depuis 10.1.2.3",
            0x4010,
        );
        let natures: Vec<Kind> = trouvés.iter().map(|i| i.kind).collect();
        assert!(natures.contains(&Kind::Url), "{trouvés:?}");
        assert!(natures.contains(&Kind::Ip), "{trouvés:?}");
        assert!(natures.contains(&Kind::Path), "{trouvés:?}");
        // Et chacun garde l'adresse de la chaîne qui le portait.
        assert!(trouvés.iter().all(|i| i.addr == 0x4010));
    }

    /// Sur une liste de chaînes, les doublons tombent et l'ordre est
    /// celui d'un rapport.
    #[test]
    fn a_list_of_strings_yields_a_report_order() {
        use crate::strings::Found;
        let chaînes = vec![
            Found {
                addr: 0x30,
                text: "/opt/charge/x".into(),
            },
            Found {
                addr: 0x10,
                text: "http://a.example.com/1".into(),
            },
            Found {
                addr: 0x20,
                text: "encore http://a.example.com/1".into(),
            },
            Found {
                addr: 0x40,
                text: "10.0.0.1".into(),
            },
        ];
        let trouvés = from_strings(&chaînes);
        assert_eq!(
            trouvés.iter().map(|i| i.kind).collect::<Vec<_>>(),
            vec![Kind::Url, Kind::Ip, Kind::Path],
            "l'URL d'abord, le chemin en dernier : {trouvés:?}"
        );
        // Le doublon n'apparaît qu'une fois, avec la première adresse.
        let url = &trouvés[0];
        assert_eq!(url.addr, 0x10);
    }

    /// Chaque nature a un nom dans les deux langues.
    #[test]
    fn each_kind_has_a_name_in_both_languages() {
        for nature in ALL {
            assert!(!nature.label("fr").is_empty());
            assert!(!nature.label("en").is_empty());
        }
    }

    /// Aucune chaîne ne fait tomber la recherche, même tordue.
    #[test]
    fn no_string_breaks_the_search() {
        for texte in [
            "",
            "http://",
            "https://///",
            "@@@@",
            "....",
            "C:\\",
            "HKCU\\",
            "/////////",
            "a@b@c@d",
            &"x".repeat(5000),
            "é@é.com",
        ] {
            let _ = in_text(texte, 0);
        }
    }
}
