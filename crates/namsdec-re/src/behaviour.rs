//! Ce que le binaire *fait* : les familles de comportement.
//!
//! Les imports et les appels système sont **listés** ailleurs, et une
//! liste suppose qu'on connaisse les noms. Mais la question d'un
//! chercheur n'est pas « quelles fonctions » : c'est « qu'est-ce que ce
//! binaire fait ». Trois familles allumées — réseau, exécution,
//! anti-débogage — sont déjà un portrait, et se lisent d'un coup d'œil
//! là où cinquante noms demandent de les connaître un par un.
//!
//! Une seule table pour les deux panneaux, et c'est voulu : un `socket`
//! est un `socket`, qu'il arrive par la table de liaison ou par le
//! numéro 41 dans RAX. Le nom est la clé commune, et `table.rs` donne
//! déjà celui de chaque appel système.
//!
//! Ce que la table **ne fait pas** : conclure. Une famille n'est pas une
//! accusation — `open` et `unlink` sont dans tous les programmes du
//! monde. Elle dit seulement où regarder, et chaque famille porte la
//! phrase qui explique ce que sa présence change.

/// Une famille de comportement : ce qu'un groupe de fonctions permet au
/// programme de faire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    /// Le programme vérifie qu'on ne l'observe pas, ou gêne qui essaie.
    AntiDebug,
    /// Il lance autre chose que lui-même.
    Execution,
    /// Il parle au réseau.
    Network,
    /// Il change les droits de sa propre mémoire, ou en demande.
    Memory,
    /// Il chiffre, déchiffre, ou brouille.
    Crypto,
    /// Il lit, écrit, efface des fichiers.
    Files,
}

/// Les familles dans l'ordre où on les regarde : du plus rare au plus
/// banal.
///
/// Ce n'est pas un classement par gravité — il n'y en a pas — mais par
/// **pouvoir d'information**. `open` ne distingue aucun programme d'un
/// autre ; `ptrace` en distingue très peu. C'est donc l'anti-débogage
/// qu'on montre en premier, et les fichiers en dernier.
pub const ALL: [Family; 6] = [
    Family::AntiDebug,
    Family::Execution,
    Family::Network,
    Family::Memory,
    Family::Crypto,
    Family::Files,
];

impl Family {
    /// Le nom de la famille, dans la langue demandée.
    pub fn label(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match (self, fr) {
            (Family::Network, true) => "Réseau",
            (Family::Network, false) => "Network",
            (Family::Execution, true) => "Exécution",
            (Family::Execution, false) => "Execution",
            (Family::AntiDebug, true) => "Anti-débogage",
            (Family::AntiDebug, false) => "Anti-debugging",
            (Family::Crypto, true) => "Chiffrement",
            (Family::Crypto, false) => "Cryptography",
            (Family::Files, true) => "Fichiers",
            (Family::Files, false) => "Files",
            (Family::Memory, true) => "Mémoire",
            (Family::Memory, false) => "Memory",
        }
    }

    /// Ce que la présence de cette famille change, pour qui lit le
    /// binaire.
    ///
    /// Jamais un verdict : ce qu'il faut aller regarder, et pourquoi.
    pub fn why(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match (self, fr) {
            (Family::Network, true) => {
                "Le programme ouvre une connexion : il peut recevoir des ordres ou \
                 sortir des données. Les chaînes diront vers où."
            }
            (Family::Network, false) => {
                "The program opens a connection: it can receive orders or send data out. \
                 The strings will say where to."
            }
            (Family::Execution, true) => {
                "Le programme en lance un autre. Ce qu'il lance est souvent dans une \
                 chaîne, parfois construit à l'exécution."
            }
            (Family::Execution, false) => {
                "The program launches another one. What it launches is often in a string, \
                 sometimes built at run time."
            }
            (Family::AntiDebug, true) => {
                "Le programme regarde si on le regarde. Son comportement sous débogueur \
                 n'est donc pas celui qu'il a seul — et c'est précisément pourquoi \
                 namsdec le simule au lieu de l'exécuter."
            }
            (Family::AntiDebug, false) => {
                "The program checks whether it is being watched. Its behaviour under a \
                 debugger is therefore not the one it has alone — which is exactly why \
                 namsdec simulates it instead of running it."
            }
            (Family::Crypto, true) => {
                "Des données sont chiffrées ou brouillées. Si l'entropie est haute \
                 quelque part, c'est probablement là."
            }
            (Family::Crypto, false) => {
                "Data is encrypted or scrambled. If entropy is high somewhere, that is \
                 probably where."
            }
            (Family::Files, true) => {
                "Le programme touche au système de fichiers : lire, écrire, effacer. \
                 Banal en soi — ce sont les chemins qui comptent."
            }
            (Family::Files, false) => {
                "The program touches the file system: read, write, delete. Ordinary in \
                 itself — the paths are what matter."
            }
            (Family::Memory, true) => {
                "Le programme demande de la mémoire ou en change les droits. Rendre une \
                 page exécutable après y avoir écrit, c'est du code qui se déballe."
            }
            (Family::Memory, false) => {
                "The program asks for memory or changes its rights. Making a page \
                 executable after writing to it is code unpacking itself."
            }
        }
    }

    /// La couleur sous laquelle la famille se montre.
    ///
    /// Du même nuancier que le reste de l'interface (VS Code Dark+), et
    /// stable : une famille garde sa couleur d'un panneau à l'autre,
    /// sans quoi la couleur n'apprendrait rien.
    pub fn color(self) -> egui::Color32 {
        match self {
            Family::AntiDebug => egui::Color32::from_rgb(0xf4, 0x87, 0x71),
            Family::Execution => egui::Color32::from_rgb(0xce, 0x91, 0x78),
            Family::Network => egui::Color32::from_rgb(0x56, 0x9c, 0xd6),
            Family::Memory => egui::Color32::from_rgb(0xd7, 0xba, 0x7d),
            Family::Crypto => egui::Color32::from_rgb(0xc5, 0x86, 0xc0),
            Family::Files => egui::Color32::from_rgb(0x4e, 0xc9, 0xb0),
        }
    }
}

/// Les noms exacts, et la famille de chacun.
///
/// Volontairement une table et non des règles : « tout ce qui commence
/// par `exec` » attraperait `execute_query` d'une bibliothèque de bases
/// de données, et nommer une famille à tort est pire que de n'en nommer
/// aucune. Les quelques préfixes qu'on accepte sont plus bas, et chacun
/// est justifié.
const TABLE: &[(&str, Family)] = &[
    // --- Réseau ---------------------------------------------------
    ("socket", Family::Network),
    ("socketpair", Family::Network),
    ("connect", Family::Network),
    ("bind", Family::Network),
    ("listen", Family::Network),
    ("accept", Family::Network),
    ("accept4", Family::Network),
    ("send", Family::Network),
    ("sendto", Family::Network),
    ("sendmsg", Family::Network),
    ("recv", Family::Network),
    ("recvfrom", Family::Network),
    ("recvmsg", Family::Network),
    ("shutdown", Family::Network),
    ("getpeername", Family::Network),
    ("getsockname", Family::Network),
    ("setsockopt", Family::Network),
    ("getsockopt", Family::Network),
    ("gethostbyname", Family::Network),
    ("getaddrinfo", Family::Network),
    ("freeaddrinfo", Family::Network),
    ("inet_addr", Family::Network),
    ("inet_aton", Family::Network),
    ("inet_ntoa", Family::Network),
    ("inet_pton", Family::Network),
    ("inet_ntop", Family::Network),
    ("htons", Family::Network),
    ("htonl", Family::Network),
    ("ntohs", Family::Network),
    ("ntohl", Family::Network),
    // --- Exécution ------------------------------------------------
    ("execve", Family::Execution),
    ("execveat", Family::Execution),
    ("execl", Family::Execution),
    ("execle", Family::Execution),
    ("execlp", Family::Execution),
    ("execv", Family::Execution),
    ("execvp", Family::Execution),
    ("execvpe", Family::Execution),
    ("fexecve", Family::Execution),
    ("system", Family::Execution),
    ("popen", Family::Execution),
    ("pclose", Family::Execution),
    ("fork", Family::Execution),
    ("vfork", Family::Execution),
    ("clone", Family::Execution),
    ("clone3", Family::Execution),
    ("posix_spawn", Family::Execution),
    ("posix_spawnp", Family::Execution),
    ("daemon", Family::Execution),
    ("setsid", Family::Execution),
    ("dlopen", Family::Execution),
    ("dlsym", Family::Execution),
    // --- Anti-débogage / évasion ----------------------------------
    ("ptrace", Family::AntiDebug),
    ("prctl", Family::AntiDebug),
    ("personality", Family::AntiDebug),
    ("process_vm_readv", Family::AntiDebug),
    ("process_vm_writev", Family::AntiDebug),
    ("seccomp", Family::AntiDebug),
    // Ce qui n'y est **pas**, et pourquoi : `raise`, `alarm`,
    // `sigaltstack`, `rt_sigaction`. Un programme qui pose un
    // gestionnaire de signaux ne se défend de personne — il range ses
    // affaires avant de mourir. `/bin/ls` appelle `raise`, et la ligne
    // « Anti-débogage » s'allumait dessus : un tableau de bord qui crie
    // sur `ls` n'est plus lu du tout. Une famille doit **distinguer**,
    // sans quoi elle ne vaut rien.
    // --- Chiffrement ----------------------------------------------
    ("crypt", Family::Crypto),
    ("crypt_r", Family::Crypto),
    ("getrandom", Family::Crypto),
    ("srand", Family::Crypto),
    ("rand", Family::Crypto),
    ("random", Family::Crypto),
    ("arc4random", Family::Crypto),
    // --- Fichiers / persistance -----------------------------------
    ("open", Family::Files),
    ("openat", Family::Files),
    ("creat", Family::Files),
    ("fopen", Family::Files),
    ("freopen", Family::Files),
    ("unlink", Family::Files),
    ("unlinkat", Family::Files),
    ("remove", Family::Files),
    ("rename", Family::Files),
    ("renameat", Family::Files),
    ("chmod", Family::Files),
    ("fchmod", Family::Files),
    ("chown", Family::Files),
    ("mkdir", Family::Files),
    ("rmdir", Family::Files),
    ("opendir", Family::Files),
    ("readdir", Family::Files),
    ("symlink", Family::Files),
    ("link", Family::Files),
    ("readlink", Family::Files),
    ("readlinkat", Family::Files),
    ("truncate", Family::Files),
    ("ftruncate", Family::Files),
    ("chdir", Family::Files),
    ("chroot", Family::Files),
    ("utimensat", Family::Files),
    // --- Mémoire --------------------------------------------------
    ("mmap", Family::Memory),
    ("mmap2", Family::Memory),
    ("mprotect", Family::Memory),
    ("mremap", Family::Memory),
    ("munmap", Family::Memory),
    ("memfd_create", Family::Memory),
    ("shmat", Family::Memory),
    ("shmget", Family::Memory),
    ("madvise", Family::Memory),
    ("pkey_mprotect", Family::Memory),
];

/// Les préfixes acceptés, et pourquoi chacun l'est.
///
/// Un préfixe range des dizaines de noms d'un coup, au risque d'en
/// ranger un qui n'a rien à voir. Ceux-ci sont ceux dont le préfixe
/// *est* le nom d'une bibliothèque : `EVP_`, `SSL_`, `curl_` ne
/// commencent rien d'autre.
const PREFIXES: &[(&str, Family)] = &[
    // OpenSSL : l'interface moderne, et les primitives directes.
    ("EVP_", Family::Crypto),
    ("AES_", Family::Crypto),
    ("DES_", Family::Crypto),
    ("RC4", Family::Crypto),
    ("RSA_", Family::Crypto),
    ("MD5", Family::Crypto),
    ("SHA1", Family::Crypto),
    ("SHA256", Family::Crypto),
    ("SHA512", Family::Crypto),
    ("HMAC", Family::Crypto),
    ("BF_", Family::Crypto),
    ("RAND_", Family::Crypto),
    ("CRYPTO_", Family::Crypto),
    ("gcry_", Family::Crypto),
    ("mbedtls_", Family::Crypto),
    ("sodium_", Family::Crypto),
    ("crypto_", Family::Crypto),
    // TLS et HTTP : du réseau, même quand le chiffrement s'y mêle.
    ("SSL_", Family::Network),
    ("TLS_", Family::Network),
    ("curl_", Family::Network),
];

/// La famille de cette fonction, si namsdec la connaît.
///
/// Le nom est nettoyé comme ailleurs : le suffixe `@plt` que le listing
/// ajoute, l'habillage `__…_chk` de `FORTIFY_SOURCE`, et le préfixe
/// `__isoc99_` qu'une glibc récente pose sur les fonctions de lecture
/// formatée. Trois habillages, un seul nom dessous.
///
/// ```
/// use namsdec_re::behaviour::{self, Family};
/// assert_eq!(behaviour::family_of("socket@plt"), Some(Family::Network));
/// assert_eq!(behaviour::family_of("ptrace"), Some(Family::AntiDebug));
/// assert_eq!(behaviour::family_of("EVP_DecryptUpdate"), Some(Family::Crypto));
/// // Et ce qu'on ne connaît pas n'est pas rangé au hasard.
/// assert_eq!(behaviour::family_of("printf"), None);
/// ```
pub fn family_of(name: &str) -> Option<Family> {
    let name = clean(name);
    if let Some((_, family)) = TABLE.iter().find(|(n, _)| *n == name) {
        return Some(*family);
    }
    PREFIXES
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
        .map(|(_, family)| *family)
}

/// Le nom sans ce que l'édition de liens y ajoute.
fn clean(name: &str) -> &str {
    let name = name.split('@').next().unwrap_or(name);
    let name = name.strip_prefix("__isoc99_").unwrap_or(name);
    // `__strcpy_chk` fait le travail de `strcpy` ; de même pour les
    // variantes `_sse2`, `_avx` qu'une glibc expose parfois.
    match name.strip_prefix("__") {
        Some(reste) => reste.strip_suffix("_chk").unwrap_or(reste),
        None => name,
    }
}

/// La famille d'un appel système, par son numéro.
///
/// Passe par `table::syscall`, qui nomme déjà l'appel : un numéro n'a
/// pas de famille à lui, c'est ce qu'il désigne qui en a une.
pub fn family_of_syscall(number: u64) -> Option<Family> {
    family_of(crate::table::syscall(number)?.name)
}

/// Les familles présentes parmi ces noms, dans l'ordre de [`ALL`].
///
/// C'est la forme dont un panneau de synthèse a besoin : non pas « quels
/// noms », mais « lesquelles de ces six choses ce binaire sait-il
/// faire ».
pub fn families_of<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<Family> {
    let trouvées: std::collections::BTreeSet<Family> =
        names.into_iter().filter_map(family_of).collect();
    ALL.into_iter().filter(|f| trouvées.contains(f)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les cinq familles que le relevé demandait se reconnaissent, sur
    /// les noms qu'on rencontre vraiment.
    #[test]
    fn the_families_are_recognised_by_name() {
        for (nom, attendue) in [
            ("socket", Family::Network),
            ("connect@plt", Family::Network),
            ("recvfrom", Family::Network),
            ("execve", Family::Execution),
            ("system", Family::Execution),
            ("posix_spawn", Family::Execution),
            ("ptrace", Family::AntiDebug),
            ("prctl", Family::AntiDebug),
            ("EVP_EncryptInit_ex", Family::Crypto),
            ("SHA256_Update", Family::Crypto),
            ("open", Family::Files),
            ("unlink@plt", Family::Files),
            ("mprotect", Family::Memory),
            ("mmap", Family::Memory),
        ] {
            assert_eq!(family_of(nom), Some(attendue), "{nom}");
        }
    }

    /// Ce qu'on ne connaît pas reste sans famille.
    ///
    /// C'est la moitié qui compte : une table qui range tout rangerait
    /// mal, et une famille nommée à tort égare plus qu'une famille
    /// absente. `printf` n'est pas un comportement remarquable, et
    /// `execute_query` n'est pas de l'exécution de processus.
    #[test]
    fn what_is_unknown_stays_unnamed() {
        for nom in [
            "printf",
            "malloc",
            "strlen",
            "execute_query",
            "opendb",
            "networkIdle",
            "",
            // Poser un gestionnaire de signaux n'est pas se défendre
            // d'un débogueur : `/bin/ls` appelle `raise`, et la famille
            // s'allumait dessus. Une famille qui ne distingue rien ne
            // vaut rien.
            "raise",
            "alarm",
            "sigaltstack",
            "rt_sigaction",
        ] {
            assert_eq!(family_of(nom), None, "{nom}");
        }
    }

    /// Les habillages de l'édition de liens ne cachent pas le nom.
    #[test]
    fn the_linker_dressings_do_not_hide_the_name() {
        assert_eq!(family_of("open@plt"), Some(Family::Files));
        assert_eq!(family_of("__open_chk"), Some(Family::Files));
        assert_eq!(
            family_of("__isoc99_fscanf"),
            None,
            "toujours pas une famille"
        );
    }

    /// Un appel système trouve sa famille par son numéro, via la table
    /// qui le nomme.
    #[test]
    fn a_syscall_finds_its_family_through_its_number() {
        assert_eq!(family_of_syscall(41), Some(Family::Network), "socket");
        assert_eq!(family_of_syscall(59), Some(Family::Execution), "execve");
        assert_eq!(family_of_syscall(101), Some(Family::AntiDebug), "ptrace");
        assert_eq!(family_of_syscall(10), Some(Family::Memory), "mprotect");
        assert_eq!(family_of_syscall(1), None, "write n'est pas remarquable");
        assert_eq!(family_of_syscall(9999), None, "pas dans la table");
    }

    /// Le portrait d'un binaire : les familles présentes, dans l'ordre
    /// où on les regarde.
    #[test]
    fn the_portrait_lists_families_in_reading_order() {
        let noms = ["malloc", "open", "socket", "ptrace", "printf"];
        assert_eq!(
            families_of(noms),
            vec![Family::AntiDebug, Family::Network, Family::Files],
            "l'anti-débogage d'abord, les fichiers en dernier"
        );
        assert!(families_of(["printf"]).is_empty());
    }

    /// Chaque famille a un nom et une explication dans les deux langues,
    /// et une couleur qui n'est à personne d'autre.
    #[test]
    fn each_family_has_a_name_a_reason_and_its_own_colour() {
        let mut couleurs = Vec::new();
        for famille in ALL {
            for lang in ["fr", "en"] {
                assert!(!famille.label(lang).is_empty(), "{famille:?} {lang}");
                assert!(famille.why(lang).len() > 40, "{famille:?} {lang}");
            }
            assert_ne!(famille.label("fr"), famille.label("en"), "{famille:?}");
            couleurs.push(famille.color());
        }
        let uniques: std::collections::BTreeSet<_> =
            couleurs.iter().map(|c| c.to_array()).collect();
        assert_eq!(uniques.len(), ALL.len(), "une couleur par famille");
    }
}
