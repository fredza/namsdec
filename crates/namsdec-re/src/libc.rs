//! Ce que font les fonctions qu'un binaire emprunte le plus souvent.
//!
//! Lire une liste de noms suppose de les connaître ; c'est vrai de
//! `malloc`, ça l'est moins de `__printf_chk` ou de `__ctype_b_loc`.
//! Une phrase par fonction, et le mot qu'on cherche vraiment quand on
//! lit un binaire : est-ce là qu'on peut se faire déborder ?
//!
//! La liste est volontairement courte : les fonctions qu'on rencontre
//! dans presque tout programme C, et celles dont l'usage est un point
//! d'attention connu. Ce qui n'y figure pas n'est pas expliqué plutôt
//! que d'être deviné.

/// Ce qu'on peut dire d'une fonction empruntée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    pub fr: &'static str,
    pub en: &'static str,
    /// Une fonction qui écrit sans connaître la taille de la
    /// destination, ou qui exécute ce qu'on lui donne. Ce n'est pas un
    /// verdict sur le programme : c'est l'endroit qu'on regarde en
    /// premier.
    pub watch: bool,
    /// Le nom de ses paramètres, dans l'ordre où l'ABI les passe.
    ///
    /// Gardés tels que les pages de manuel les écrivent — `dest`, `src`,
    /// `n`, `fd` — dans les deux langues : ce sont des identifiants,
    /// comme `rdi`, et c'est sous ces noms-là qu'on les retrouvera
    /// ailleurs. Vide quand la signature ne se résume pas ainsi.
    pub params: &'static [&'static str],
}

impl Note {
    pub fn text(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.fr
        } else {
            self.en
        }
    }
}

const fn note(fr: &'static str, en: &'static str) -> Note {
    Note {
        fr,
        en,
        watch: false,
        params: &[],
    }
}

const fn watch(fr: &'static str, en: &'static str) -> Note {
    Note {
        fr,
        en,
        watch: true,
        params: &[],
    }
}

/// La même note, avec le nom de ses paramètres.
const fn args(note: Note, params: &'static [&'static str]) -> Note {
    Note { params, ..note }
}

/// Ce que fait cette fonction, si namsdec le sait.
///
/// Le nom est nettoyé de ce que l'édition de liens y ajoute : le suffixe
/// `@plt` du listing, et l'habillage `__…_chk` que pose `FORTIFY_SOURCE`
/// — une variante qui vérifie la taille de la destination, et qui fait
/// donc le même travail que la fonction d'origine.
///
/// ```
/// let note = namsdec_re::libc::explain("__strcpy_chk@plt").unwrap();
/// assert!(note.fr.contains("Copie"));
/// assert!(note.watch, "strcpy écrit sans borne, habillée ou non");
/// ```
pub fn explain(name: &str) -> Option<Note> {
    let name = name.split('@').next().unwrap_or(name);
    if let Some(note) = TABLE.iter().find(|(n, _)| *n == name) {
        return Some(note.1);
    }
    // `__printf_chk` fait ce que fait `printf` : on le dit plutôt que de
    // ne rien dire.
    let dépouillé = name.trim_start_matches('_');
    let dépouillé = dépouillé.strip_suffix("_chk").unwrap_or(dépouillé);
    TABLE
        .iter()
        .find(|(n, _)| *n == dépouillé)
        .map(|(_, note)| *note)
}

/// La table elle-même : nom exact, puis phrase.
static TABLE: &[(&str, Note)] = &[
    // Mémoire.
    (
        "malloc",
        args(
            note(
                "Réserve un bloc de mémoire sur le tas et rend son adresse, ou zéro.",
                "Reserves a block of heap memory and returns its address, or zero.",
            ),
            &["size"],
        ),
    ),
    (
        "calloc",
        args(
            note(
                "Comme malloc, mais le bloc est rempli de zéros.",
                "Like malloc, but the block comes filled with zeroes.",
            ),
            &["nmemb", "size"],
        ),
    ),
    (
        "realloc",
        args(
            note(
                "Change la taille d'un bloc ; l'adresse peut changer avec.",
                "Changes a block's size; the address may change with it.",
            ),
            &["ptr", "size"],
        ),
    ),
    (
        "free",
        args(
            note(
                "Rend un bloc au tas. L'adresse reste dans le registre, mais ne désigne plus rien.",
                "Gives a block back to the heap. The address stays in the register but designates nothing.",
            ),
            &["ptr"],
        ),
    ),
    (
        "memcpy",
        args(
            note(
                "Copie n octets d'un endroit à un autre ; les zones ne doivent pas se chevaucher.",
                "Copies n bytes from one place to another; the regions must not overlap.",
            ),
            &["dest", "src", "n"],
        ),
    ),
    (
        "memmove",
        args(
            note(
                "Comme memcpy, mais correct même si les zones se chevauchent.",
                "Like memcpy, but correct even when the regions overlap.",
            ),
            &["dest", "src", "n"],
        ),
    ),
    (
        "memset",
        args(
            note(
                "Remplit n octets avec la même valeur.",
                "Fills n bytes with the same value.",
            ),
            &["s", "c", "n"],
        ),
    ),
    (
        "memcmp",
        args(
            note(
                "Compare n octets, et rend le signe de leur différence.",
                "Compares n bytes and returns the sign of their difference.",
            ),
            &["s1", "s2", "n"],
        ),
    ),
    // Chaînes.
    (
        "strlen",
        args(
            note(
                "Compte les octets jusqu'au zéro final — sans lui, elle ne s'arrête pas.",
                "Counts bytes up to the final zero — without one, it does not stop.",
            ),
            &["s"],
        ),
    ),
    (
        "strcpy",
        args(
            watch(
                "Copie une chaîne jusqu'à son zéro, sans savoir ce qui tient dans la destination.",
                "Copies a string up to its zero, with no idea what fits in the destination.",
            ),
            &["dest", "src"],
        ),
    ),
    (
        "strcat",
        args(
            watch(
                "Ajoute une chaîne à la fin d'une autre, sans borne non plus.",
                "Appends a string to another, with no bound either.",
            ),
            &["dest", "src"],
        ),
    ),
    (
        "strncpy",
        args(
            note(
                "Copie au plus n octets — et ne pose pas le zéro final si la source les remplit tous.",
                "Copies at most n bytes — and writes no final zero if the source fills them all.",
            ),
            &["dest", "src", "n"],
        ),
    ),
    (
        "strcmp",
        args(
            note(
                "Compare deux chaînes, et rend le signe de leur différence.",
                "Compares two strings and returns the sign of their difference.",
            ),
            &["s1", "s2"],
        ),
    ),
    (
        "strncmp",
        args(
            note(
                "Compare au plus n octets de deux chaînes.",
                "Compares at most n bytes of two strings.",
            ),
            &["s1", "s2", "n"],
        ),
    ),
    (
        "strdup",
        args(
            note(
                "Copie une chaîne dans un bloc neuf du tas, qu'il faudra rendre.",
                "Copies a string into a fresh heap block, which must be given back.",
            ),
            &["s"],
        ),
    ),
    (
        "strstr",
        args(
            note(
                "Cherche une chaîne dans une autre et rend où elle commence.",
                "Looks for one string inside another and returns where it starts.",
            ),
            &["haystack", "needle"],
        ),
    ),
    (
        "strchr",
        args(
            note(
                "Cherche un octet dans une chaîne.",
                "Looks for one byte inside a string.",
            ),
            &["s", "c"],
        ),
    ),
    // Entrées et sorties.
    (
        "printf",
        args(
            note(
                "Écrit sur la sortie standard en suivant un format. Le format lui-même est lu comme du code.",
                "Writes to standard output following a format. The format itself is read as code.",
            ),
            &["format", "..."],
        ),
    ),
    (
        "fprintf",
        args(
            note(
                "Comme printf, vers un flux choisi.",
                "Like printf, to a chosen stream.",
            ),
            &["stream", "format", "..."],
        ),
    ),
    (
        "sprintf",
        args(
            watch(
                "Écrit le résultat dans un tampon, sans savoir ce qui y tient.",
                "Writes the result into a buffer, with no idea what fits.",
            ),
            &["str", "format", "..."],
        ),
    ),
    (
        "snprintf",
        args(
            note(
                "Comme sprintf, mais bornée : elle écrit au plus n octets.",
                "Like sprintf, but bounded: it writes at most n bytes.",
            ),
            &["str", "size", "format", "..."],
        ),
    ),
    (
        "puts",
        args(
            note(
                "Écrit une chaîne et un saut de ligne sur la sortie standard.",
                "Writes a string and a newline to standard output.",
            ),
            &["s"],
        ),
    ),
    (
        "fputs",
        args(
            note(
                "Écrit une chaîne sur un flux, sans saut de ligne.",
                "Writes a string to a stream, with no newline.",
            ),
            &["s", "stream"],
        ),
    ),
    (
        "gets",
        args(
            watch(
                "Lit une ligne sans aucune borne. Retirée du langage C : sa présence date le binaire, ou l'explique.",
                "Reads a line with no bound whatsoever. Removed from C: its presence dates the binary, or explains it.",
            ),
            &["s"],
        ),
    ),
    (
        "fgets",
        args(
            note(
                "Lit au plus n-1 octets d'une ligne, et pose le zéro final.",
                "Reads at most n-1 bytes of a line and writes the final zero.",
            ),
            &["s", "size", "stream"],
        ),
    ),
    (
        "scanf",
        args(
            watch(
                "Lit selon un format ; `%s` sans largeur n'a pas de borne.",
                "Reads following a format; `%s` without a width has no bound.",
            ),
            &["format", "..."],
        ),
    ),
    (
        "sscanf",
        args(
            note(
                "Comme scanf, en lisant dans une chaîne plutôt que sur l'entrée.",
                "Like scanf, reading from a string instead of the input.",
            ),
            &["str", "format", "..."],
        ),
    ),
    (
        "fopen",
        args(
            note(
                "Ouvre un fichier et rend un flux.",
                "Opens a file and returns a stream.",
            ),
            &["path", "mode"],
        ),
    ),
    (
        "fclose",
        args(note("Ferme un flux.", "Closes a stream."), &["stream"]),
    ),
    (
        "fread",
        args(
            note(
                "Lit des éléments depuis un flux.",
                "Reads items from a stream.",
            ),
            &["ptr", "size", "nmemb", "stream"],
        ),
    ),
    (
        "fwrite",
        args(
            note(
                "Écrit des éléments sur un flux.",
                "Writes items to a stream.",
            ),
            &["ptr", "size", "nmemb", "stream"],
        ),
    ),
    (
        "open",
        args(
            note(
                "Ouvre un fichier et rend un descripteur — l'appel système, sans le flux.",
                "Opens a file and returns a descriptor — the system call, without the stream.",
            ),
            &["path", "flags"],
        ),
    ),
    (
        "read",
        args(
            note(
                "Lit au plus n octets depuis un descripteur.",
                "Reads at most n bytes from a descriptor.",
            ),
            &["fd", "buf", "count"],
        ),
    ),
    (
        "write",
        args(
            note(
                "Écrit n octets sur un descripteur.",
                "Writes n bytes to a descriptor.",
            ),
            &["fd", "buf", "count"],
        ),
    ),
    (
        "close",
        args(
            note("Ferme un descripteur.", "Closes a descriptor."),
            &["fd"],
        ),
    ),
    // Le système.
    (
        "exit",
        args(
            note(
                "Termine le programme avec un code, après avoir vidé les flux.",
                "Ends the program with a code, after flushing the streams.",
            ),
            &["status"],
        ),
    ),
    (
        "abort",
        note(
            "Termine le programme sur un signal, sans rien vider.",
            "Ends the program on a signal, flushing nothing.",
        ),
    ),
    (
        "system",
        args(
            watch(
                "Fait exécuter une commande par le shell. Ce qu'on lui passe est du code.",
                "Has the shell run a command. Whatever is passed to it is code.",
            ),
            &["command"],
        ),
    ),
    (
        "execve",
        args(
            watch(
                "Remplace le programme en cours par un autre.",
                "Replaces the running program with another one.",
            ),
            &["path", "argv", "envp"],
        ),
    ),
    (
        "fork",
        note(
            "Duplique le processus ; les deux reprennent au même endroit.",
            "Duplicates the process; both resume at the same place.",
        ),
    ),
    (
        "getenv",
        args(
            note(
                "Lit une variable d'environnement : une entrée que l'appelant contrôle.",
                "Reads an environment variable: an input the caller controls.",
            ),
            &["name"],
        ),
    ),
    (
        "setuid",
        args(
            watch(
                "Change l'identité du processus. Un binaire qui l'appelle mérite un second regard.",
                "Changes the process identity. A binary calling it deserves a second look.",
            ),
            &["uid"],
        ),
    ),
    (
        "getuid",
        note(
            "Rend l'identité réelle du processus.",
            "Returns the process's real identity.",
        ),
    ),
    // Autour.
    (
        "atoi",
        args(
            note(
                "Convertit une chaîne en entier, sans dire si elle n'en était pas un.",
                "Converts a string to an integer, without saying if it was not one.",
            ),
            &["nptr"],
        ),
    ),
    (
        "strtol",
        args(
            note(
                "Convertit une chaîne en entier, et dit où elle s'est arrêtée.",
                "Converts a string to an integer, and says where it stopped.",
            ),
            &["nptr", "endptr", "base"],
        ),
    ),
    (
        "qsort",
        args(
            note(
                "Trie un tableau en appelant la fonction de comparaison qu'on lui passe.",
                "Sorts an array by calling the comparison function it is given.",
            ),
            &["base", "nmemb", "size", "compar"],
        ),
    ),
    (
        "rand",
        note(
            "Rend un nombre pseudo-aléatoire — prévisible si la graine l'est.",
            "Returns a pseudo-random number — predictable if the seed is.",
        ),
    ),
    (
        "srand",
        args(
            note(
                "Choisit la graine de rand : la même graine rend la même suite.",
                "Chooses rand's seed: the same seed gives the same sequence.",
            ),
            &["seed"],
        ),
    ),
    (
        "time",
        args(
            note(
                "L'heure courante, en secondes.",
                "The current time, in seconds.",
            ),
            &["tloc"],
        ),
    ),
    (
        "stack_chk_fail",
        note(
            "Appelée quand le canari de pile a été écrasé : le programme s'arrête là, exprès.",
            "Called when the stack canary has been overwritten: the program stops there, on purpose.",
        ),
    ),
    (
        "ctype_b_loc",
        note(
            "La table qui dit, pour chaque octet, s'il est une lettre, un chiffre, un espace.",
            "The table saying, for each byte, whether it is a letter, a digit, a space.",
        ),
    ),
    // Le support d'exécution, qu'on trouve dans presque tout binaire
    // C++ ou Rust — et que la liste ne nommait pas, si bien que les
    // deux seules entrées d'un crackme en Rust restaient muettes.
    (
        "__libc_start_main",
        args(
            note(
                "Le vrai démarrage : la glibc prépare l'environnement, puis appelle `main` \
                 et passe sa valeur de retour à `exit`. C'est d'elle qu'on remonte pour \
                 retrouver `main` dans un binaire dépouillé.",
                "The real start-up: the C library prepares the environment, then calls \
                 `main` and hands its return value to `exit`. It is the one you follow \
                 back to find `main` in a stripped binary.",
            ),
            &["main", "argc", "argv", "init", "fini", "rtld_fini"],
        ),
    ),
    (
        "__cxa_finalize",
        args(
            note(
                "Exécute ce qui restait à faire en sortant : les destructeurs globaux et \
                 les fonctions enregistrées par `atexit`. Appelée au déchargement d'une \
                 bibliothèque comme à la fin du programme.",
                "Runs what was left to do on the way out: global destructors and the \
                 functions registered with `atexit`. Called when unloading a library as \
                 well as at the end of the program.",
            ),
            &["d"],
        ),
    ),
    (
        "__cxa_atexit",
        args(
            note(
                "Enregistre une fonction à rappeler à la fin du programme — c'est ainsi \
                 qu'un objet global fait exécuter son destructeur.",
                "Registers a function to call back at the end of the program — this is how \
                 a global object gets its destructor run.",
            ),
            &["func", "arg", "dso_handle"],
        ),
    ),
    (
        "_Unwind_Resume",
        note(
            "Reprend le déroulement de la pile après un bloc de nettoyage : c'est la \
             mécanique des exceptions C++ et des paniques Rust. Sa présence dit que le \
             programme sait remonter les appels en défaisant ce qu'il a fait.",
            "Resumes stack unwinding after a cleanup block: this is the machinery behind \
             C++ exceptions and Rust panics. Its presence says the program knows how to \
             walk back up the calls undoing what it did.",
        ),
    ),
];

/// Les registres qui portent les six premiers arguments, dans l'ordre où
/// la convention System V les passe.
///
/// Ici plutôt qu'ailleurs parce que c'est ici que vivent les NOMS des
/// paramètres : « dest » ne veut rien dire tant qu'on ne sait pas qu'il
/// arrive par RDI, et les deux listes se lisent ensemble.
pub const ARGUMENT_REGISTERS: [&str; 6] = ["RDI", "RSI", "RDX", "RCX", "R8", "R9"];

/// Ce que le NOM d'une fonction dit de sa famille, quand on ne connaît
/// pas la fonction elle-même.
///
/// La table ci-dessus est volontairement courte, et le module l'assume :
/// ce qui n'y figure pas n'est pas expliqué plutôt que d'être deviné.
/// Mais il y a un entre-deux honnête — le PRÉFIXE d'un nom appartient à
/// une convention, et dire « ceci fait partie du support d'exécution
/// C++ » n'est pas une supposition sur ce que fait la fonction : c'est
/// une lecture de son nom.
///
/// Rendue à part de [`explain`], qui garde son contrat : une phrase sur
/// CETTE fonction, ou rien.
///
/// ```
/// let (famille, _) = namsdec_re::libc::family_hint("__cxa_guard_acquire").unwrap();
/// assert!(famille.contains("C++"));
/// assert!(namsdec_re::libc::family_hint("malloc").is_none());
/// ```
pub fn family_hint(name: &str) -> Option<(&'static str, &'static str)> {
    let name = name.split('@').next().unwrap_or(name);
    // Du plus précis au plus large : `__cxa_` avant `__`.
    const FAMILLES: &[(&str, &str, &str)] = &[
        (
            "__cxa_",
            "Support d'exécution C++ (ABI Itanium) : destructeurs globaux, exceptions, \
             initialisation des variables statiques.",
            "C++ runtime support (Itanium ABI): global destructors, exceptions, static \
             variable initialisation.",
        ),
        (
            "_Unwind_",
            "Le dérouleur de pile : il remonte les appels en défaisant ce qu'ils ont fait. \
             C'est la mécanique des exceptions C++ et des paniques Rust.",
            "The stack unwinder: it walks back up the calls undoing what they did. This is \
             the machinery behind C++ exceptions and Rust panics.",
        ),
        (
            "_ZN",
            "Un nom C++ ou Rust encodé (« mangled ») : le compilateur y a mis le chemin du \
             module et la signature pour que deux fonctions de même nom puissent coexister.",
            "A mangled C++ or Rust name: the compiler put the module path and the signature \
             in it so that two functions of the same name can coexist.",
        ),
        (
            "__gmon_",
            "Le support de profilage de gprof, posé par `-pg`.",
            "gprof's profiling support, put there by `-pg`.",
        ),
        (
            "_ITM_",
            "Un appel vers la mémoire transactionnelle de GCC. La glibc en fournit des \
             souches vides : presque toujours sans effet.",
            "A call into GCC's transactional memory. The C library provides empty stubs for \
             these: almost always without effect.",
        ),
        (
            "__libc_",
            "Les entrailles de la bibliothèque C, appelées par le code de démarrage plutôt \
             que par le programme lui-même.",
            "The C library's internals, called by the start-up code rather than by the \
             program itself.",
        ),
        (
            "pthread_",
            "La bibliothèque de fils d'exécution : ce programme en crée, en attend ou se \
             synchronise.",
            "The threading library: this program creates threads, waits for them, or \
             synchronises.",
        ),
    ];
    FAMILLES
        .iter()
        .find(|(préfixe, _, _)| name.starts_with(préfixe))
        .map(|(_, fr, en)| (*fr, *en))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_explained_through_what_the_linker_adds_to_it() {
        // Le suffixe du listing.
        assert!(explain("malloc@plt").is_some());
        // L'habillage de FORTIFY_SOURCE : la même fonction, vérifiée.
        let chk = explain("__snprintf_chk").expect("snprintf, en version bornée");
        assert_eq!(chk, explain("snprintf").unwrap());
        // Le canari, dont le nom vrai commence par deux tirets bas.
        assert!(explain("__stack_chk_fail").is_some());
        // Et ce qu'on ne connaît pas n'est pas inventé.
        assert!(explain("cap_to_text").is_none());
    }

    /// Les fonctions à regarder en premier le sont dans les deux
    /// langues, et pour la même raison.
    #[test]
    fn the_ones_to_watch_say_why_in_both_languages() {
        for name in ["gets", "strcpy", "strcat", "sprintf", "system"] {
            let note = explain(name).expect(name);
            assert!(note.watch, "{name} devrait être signalée");
            assert!(!note.fr.is_empty() && !note.en.is_empty());
            assert_ne!(note.fr, note.en);
        }
        // Et une fonction bornée ne l'est pas.
        assert!(!explain("snprintf").unwrap().watch);
        assert!(!explain("strncmp").unwrap().watch);
    }

    /// Les fonctions les plus courantes disent le nom de leurs
    /// paramètres : c'est ce qui transforme « rdi = 0x402000 » en
    /// « dest = 0x402000 ».
    #[test]
    fn the_common_ones_name_their_parameters() {
        assert_eq!(explain("strcpy").unwrap().params, ["dest", "src"]);
        assert_eq!(explain("read@plt").unwrap().params, ["fd", "buf", "count"]);
        // L'habillage de FORTIFY garde la signature de l'original.
        assert_eq!(
            explain("__memcpy_chk").unwrap().params,
            ["dest", "src", "n"]
        );
        // Les variadiques le disent plutôt que de mentir sur le nombre.
        assert_eq!(explain("printf").unwrap().params, ["format", "..."]);
        // Et une fonction sans signature connue n'en invente pas.
        assert!(explain("abort").unwrap().params.is_empty());
    }

    /// Chaque entrée dit quelque chose, dans les deux langues.
    #[test]
    fn every_entry_is_complete() {
        for (name, note) in TABLE {
            assert!(!note.fr.is_empty(), "{name} : rien en français");
            assert!(!note.en.is_empty(), "{name} : rien en anglais");
            assert_ne!(note.fr, note.en, "{name} : la même phrase deux fois");
        }
    }
}
