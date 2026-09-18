//! Les appels système Linux x86-64 : leur nom, et ce que chaque registre
//! y transporte.
//!
//! Un `syscall` ne dit rien à qui le lit pour la première fois : le
//! numéro est dans RAX, les arguments dans RDI, RSI, RDX, R10, R8, R9,
//! et rien à l'écran ne le rappelle. C'est une convention à connaître —
//! autant la donner.
//!
//! La table est volontairement courte : les appels qu'un programme
//! d'apprentissage utilise, plus ceux qu'on croise en lisant un binaire.

/// Un appel système : son nom, et le nom de ses arguments dans l'ordre
/// où la convention les range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Syscall {
    pub number: u64,
    pub name: &'static str,
    /// Les arguments, dans l'ordre RDI, RSI, RDX, R10, R8, R9.
    pub args: &'static [&'static str],
    /// namsdec l'exécute-t-il vraiment ? Le dire évite la surprise d'un
    /// programme qui s'arrête au milieu.
    pub simulated: bool,
}

/// Les registres qui portent les arguments d'un appel système, dans
/// l'ordre. Ce n'est **pas** la convention des appels de fonction : le
/// quatrième passe par R10, pas par RCX, que `syscall` écrase.
pub const ARGUMENT_REGISTERS: [&str; 6] = ["RDI", "RSI", "RDX", "R10", "R8", "R9"];

/// L'appel système numéro `number`, s'il est dans la table.
pub fn syscall(number: u64) -> Option<Syscall> {
    let (name, args): (&'static str, &'static [&'static str]) = match number {
        0 => ("read", &["fd", "buf", "count"]),
        1 => ("write", &["fd", "buf", "count"]),
        2 => ("open", &["chemin", "options", "droits"]),
        3 => ("close", &["fd"]),
        4 => ("stat", &["chemin", "statbuf"]),
        5 => ("fstat", &["fd", "statbuf"]),
        8 => ("lseek", &["fd", "décalage", "origine"]),
        9 => (
            "mmap",
            &["adresse", "taille", "droits", "options", "fd", "décalage"],
        ),
        10 => ("mprotect", &["adresse", "taille", "droits"]),
        11 => ("munmap", &["adresse", "taille"]),
        12 => ("brk", &["rupture"]),
        13 => ("rt_sigaction", &["signal", "action", "ancienne"]),
        16 => ("ioctl", &["fd", "requête", "argument"]),
        19 => ("readv", &["fd", "vecteur", "nombre"]),
        20 => ("writev", &["fd", "vecteur", "nombre"]),
        22 => ("pipe", &["tubes"]),
        23 => (
            "select",
            &["nfds", "lecture", "écriture", "exception", "délai"],
        ),
        24 => ("sched_yield", &[]),
        32 => ("dup", &["fd"]),
        35 => ("nanosleep", &["durée", "restant"]),
        39 => ("getpid", &[]),
        41 => ("socket", &["domaine", "type", "protocole"]),
        42 => ("connect", &["fd", "adresse", "taille"]),
        57 => ("fork", &[]),
        59 => ("execve", &["chemin", "arguments", "environnement"]),
        60 => ("exit", &["code"]),
        61 => ("wait4", &["pid", "statut", "options", "ressources"]),
        62 => ("kill", &["pid", "signal"]),
        63 => ("uname", &["buf"]),
        79 => ("getcwd", &["buf", "taille"]),
        83 => ("mkdir", &["chemin", "droits"]),
        87 => ("unlink", &["chemin"]),
        96 => ("gettimeofday", &["heure", "fuseau"]),
        89 => ("readlink", &["chemin", "buf", "taille"]),
        97 => ("getrlimit", &["ressource", "limite"]),
        102 => ("getuid", &[]),
        104 => ("getgid", &[]),
        107 => ("geteuid", &[]),
        108 => ("getegid", &[]),
        110 => ("getppid", &[]),
        158 => ("arch_prctl", &["code", "adresse"]),
        186 => ("gettid", &[]),
        202 => ("futex", &["adresse", "opération", "valeur", "délai"]),
        218 => ("set_tid_address", &["adresse"]),
        228 => ("clock_gettime", &["horloge", "temps"]),
        262 => ("newfstatat", &["dossier", "chemin", "statbuf", "options"]),
        267 => ("readlinkat", &["dossier", "chemin", "buf", "taille"]),
        273 => ("set_robust_list", &["liste", "taille"]),
        302 => (
            "prlimit64",
            &["pid", "ressource", "nouvelle", "ancienne"],
        ),
        334 => ("rseq", &["rseq", "taille", "options", "signature"]),
        201 => ("time", &["résultat"]),
        318 => ("getrandom", &["buf", "taille", "options"]),
        231 => ("exit_group", &["code"]),
        257 => ("openat", &["dossier", "chemin", "options", "droits"]),
        _ => return None,
    };
    Some(Syscall {
        number,
        name,
        args,
        // Ceux que l'interpréteur exécute pour de vrai.
        //
        // Trois familles. Ceux qui *font* le travail — lire, écrire,
        // sortir, demander de la mémoire. Ceux qui *renseignent* sans
        // rien changer au monde, et sur lesquels s'arrêter empêcherait
        // seulement le programme d'avancer. Et ceux du **démarrage**
        // d'une bibliothèque C — `arch_prctl` pour installer la base FS,
        // `fstat` pour choisir comment tamponner la sortie, `prlimit64`
        // pour connaître sa pile : un binaire lié statiquement en fait
        // une vingtaine avant d'arriver à `main`, et il suffit d'un seul
        // manquant pour qu'il n'y arrive jamais.
        simulated: matches!(
            number,
            0 | 1
                | 5
                | 9
                | 10
                | 11
                | 12
                | 16
                | 19
                | 20
                | 24
                | 35
                | 39
                | 60
                | 63
                | 89
                | 96
                | 97
                | 102
                | 104
                | 107
                | 108
                | 110
                | 158
                | 186
                | 202
                | 218
                | 228
                | 231
                | 262
                | 267
                | 273
                | 302
                | 318
                | 334
        ),
    })
}

/// Le nom seul, pour les listes qui n'ont pas la place du reste.
pub fn syscall_name(number: u64) -> Option<&'static str> {
    syscall(number).map(|call| call.name)
}

/// Ceux que l'interpréteur exécute vraiment.
pub fn is_simulated(number: u64) -> bool {
    syscall(number).is_some_and(|call| call.simulated)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les trois appels d'un premier programme, avec leurs arguments
    /// nommés : c'est ce qui manque le plus en lisant un `syscall`.
    #[test]
    fn the_first_calls_one_writes_are_named() {
        let write = syscall(1).expect("write est dans la table");
        assert_eq!(write.name, "write");
        assert_eq!(write.args, ["fd", "buf", "count"]);
        assert!(write.simulated);

        assert_eq!(syscall_name(60), Some("exit"));
        assert_eq!(syscall(60).unwrap().args, ["code"]);
        // Sans argument, et c'est une information en soi.
        assert!(syscall(39).unwrap().args.is_empty());
    }

    /// Ceux que namsdec ne simule pas doivent le dire : un programme qui
    /// s'arrête au milieu sans explication est un programme incompris.
    #[test]
    fn unsimulated_calls_say_so() {
        assert!(is_simulated(1) && is_simulated(60));
        // Ce qui demanderait un vrai système : un système de fichiers,
        // d'autres processus, un réseau.
        assert!(!is_simulated(2), "ouvrir un fichier n'est pas simulé");
        assert!(!is_simulated(59), "execve non plus");
        assert!(syscall(2).is_some(), "mais ils sont nommés");
        // Ceux du démarrage d'une bibliothèque C, eux, le sont.
        assert!(is_simulated(158), "arch_prctl installe la base FS");
        assert!(is_simulated(9), "mmap donne de la mémoire");
        assert!(syscall(4242).is_none());
    }

    /// Le quatrième argument passe par R10, pas par RCX : `syscall`
    /// écrase RCX, et c'est l'erreur qu'on fait en venant des appels de
    /// fonction.
    #[test]
    fn the_fourth_argument_goes_through_r10() {
        assert_eq!(ARGUMENT_REGISTERS[3], "R10");
        assert!(!ARGUMENT_REGISTERS.contains(&"RCX"));
    }
}
