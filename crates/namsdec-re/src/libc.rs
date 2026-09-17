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
    }
}

const fn watch(fr: &'static str, en: &'static str) -> Note {
    Note {
        fr,
        en,
        watch: true,
    }
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
        note(
            "Réserve un bloc de mémoire sur le tas et rend son adresse, ou zéro.",
            "Reserves a block of heap memory and returns its address, or zero.",
        ),
    ),
    (
        "calloc",
        note(
            "Comme malloc, mais le bloc est rempli de zéros.",
            "Like malloc, but the block comes filled with zeroes.",
        ),
    ),
    (
        "realloc",
        note(
            "Change la taille d'un bloc ; l'adresse peut changer avec.",
            "Changes a block's size; the address may change with it.",
        ),
    ),
    (
        "free",
        note(
            "Rend un bloc au tas. L'adresse reste dans le registre, mais ne désigne plus rien.",
            "Gives a block back to the heap. The address stays in the register but designates nothing.",
        ),
    ),
    (
        "memcpy",
        note(
            "Copie n octets d'un endroit à un autre ; les zones ne doivent pas se chevaucher.",
            "Copies n bytes from one place to another; the regions must not overlap.",
        ),
    ),
    (
        "memmove",
        note(
            "Comme memcpy, mais correct même si les zones se chevauchent.",
            "Like memcpy, but correct even when the regions overlap.",
        ),
    ),
    (
        "memset",
        note(
            "Remplit n octets avec la même valeur.",
            "Fills n bytes with the same value.",
        ),
    ),
    (
        "memcmp",
        note(
            "Compare n octets, et rend le signe de leur différence.",
            "Compares n bytes and returns the sign of their difference.",
        ),
    ),
    // Chaînes.
    (
        "strlen",
        note(
            "Compte les octets jusqu'au zéro final — sans lui, elle ne s'arrête pas.",
            "Counts bytes up to the final zero — without one, it does not stop.",
        ),
    ),
    (
        "strcpy",
        watch(
            "Copie une chaîne jusqu'à son zéro, sans savoir ce qui tient dans la destination.",
            "Copies a string up to its zero, with no idea what fits in the destination.",
        ),
    ),
    (
        "strcat",
        watch(
            "Ajoute une chaîne à la fin d'une autre, sans borne non plus.",
            "Appends a string to another, with no bound either.",
        ),
    ),
    (
        "strncpy",
        note(
            "Copie au plus n octets — et ne pose pas le zéro final si la source les remplit tous.",
            "Copies at most n bytes — and writes no final zero if the source fills them all.",
        ),
    ),
    (
        "strcmp",
        note(
            "Compare deux chaînes, et rend le signe de leur différence.",
            "Compares two strings and returns the sign of their difference.",
        ),
    ),
    (
        "strncmp",
        note(
            "Compare au plus n octets de deux chaînes.",
            "Compares at most n bytes of two strings.",
        ),
    ),
    (
        "strdup",
        note(
            "Copie une chaîne dans un bloc neuf du tas, qu'il faudra rendre.",
            "Copies a string into a fresh heap block, which must be given back.",
        ),
    ),
    (
        "strstr",
        note(
            "Cherche une chaîne dans une autre et rend où elle commence.",
            "Looks for one string inside another and returns where it starts.",
        ),
    ),
    (
        "strchr",
        note(
            "Cherche un octet dans une chaîne.",
            "Looks for one byte inside a string.",
        ),
    ),
    // Entrées et sorties.
    (
        "printf",
        note(
            "Écrit sur la sortie standard en suivant un format. Le format lui-même est lu comme du code.",
            "Writes to standard output following a format. The format itself is read as code.",
        ),
    ),
    (
        "fprintf",
        note(
            "Comme printf, vers un flux choisi.",
            "Like printf, to a chosen stream.",
        ),
    ),
    (
        "sprintf",
        watch(
            "Écrit le résultat dans un tampon, sans savoir ce qui y tient.",
            "Writes the result into a buffer, with no idea what fits.",
        ),
    ),
    (
        "snprintf",
        note(
            "Comme sprintf, mais bornée : elle écrit au plus n octets.",
            "Like sprintf, but bounded: it writes at most n bytes.",
        ),
    ),
    (
        "puts",
        note(
            "Écrit une chaîne et un saut de ligne sur la sortie standard.",
            "Writes a string and a newline to standard output.",
        ),
    ),
    (
        "fputs",
        note(
            "Écrit une chaîne sur un flux, sans saut de ligne.",
            "Writes a string to a stream, with no newline.",
        ),
    ),
    (
        "gets",
        watch(
            "Lit une ligne sans aucune borne. Retirée du langage C : sa présence date le binaire, ou l'explique.",
            "Reads a line with no bound whatsoever. Removed from C: its presence dates the binary, or explains it.",
        ),
    ),
    (
        "fgets",
        note(
            "Lit au plus n-1 octets d'une ligne, et pose le zéro final.",
            "Reads at most n-1 bytes of a line and writes the final zero.",
        ),
    ),
    (
        "scanf",
        watch(
            "Lit selon un format ; `%s` sans largeur n'a pas de borne.",
            "Reads following a format; `%s` without a width has no bound.",
        ),
    ),
    (
        "sscanf",
        note(
            "Comme scanf, en lisant dans une chaîne plutôt que sur l'entrée.",
            "Like scanf, reading from a string instead of the input.",
        ),
    ),
    (
        "fopen",
        note(
            "Ouvre un fichier et rend un flux.",
            "Opens a file and returns a stream.",
        ),
    ),
    ("fclose", note("Ferme un flux.", "Closes a stream.")),
    (
        "fread",
        note(
            "Lit des éléments depuis un flux.",
            "Reads items from a stream.",
        ),
    ),
    (
        "fwrite",
        note(
            "Écrit des éléments sur un flux.",
            "Writes items to a stream.",
        ),
    ),
    (
        "open",
        note(
            "Ouvre un fichier et rend un descripteur — l'appel système, sans le flux.",
            "Opens a file and returns a descriptor — the system call, without the stream.",
        ),
    ),
    (
        "read",
        note(
            "Lit au plus n octets depuis un descripteur.",
            "Reads at most n bytes from a descriptor.",
        ),
    ),
    (
        "write",
        note(
            "Écrit n octets sur un descripteur.",
            "Writes n bytes to a descriptor.",
        ),
    ),
    (
        "close",
        note("Ferme un descripteur.", "Closes a descriptor."),
    ),
    // Le système.
    (
        "exit",
        note(
            "Termine le programme avec un code, après avoir vidé les flux.",
            "Ends the program with a code, after flushing the streams.",
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
        watch(
            "Fait exécuter une commande par le shell. Ce qu'on lui passe est du code.",
            "Has the shell run a command. Whatever is passed to it is code.",
        ),
    ),
    (
        "execve",
        watch(
            "Remplace le programme en cours par un autre.",
            "Replaces the running program with another one.",
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
        note(
            "Lit une variable d'environnement : une entrée que l'appelant contrôle.",
            "Reads an environment variable: an input the caller controls.",
        ),
    ),
    (
        "setuid",
        watch(
            "Change l'identité du processus. Un binaire qui l'appelle mérite un second regard.",
            "Changes the process identity. A binary calling it deserves a second look.",
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
        note(
            "Convertit une chaîne en entier, sans dire si elle n'en était pas un.",
            "Converts a string to an integer, without saying if it was not one.",
        ),
    ),
    (
        "strtol",
        note(
            "Convertit une chaîne en entier, et dit où elle s'est arrêtée.",
            "Converts a string to an integer, and says where it stopped.",
        ),
    ),
    (
        "qsort",
        note(
            "Trie un tableau en appelant la fonction de comparaison qu'on lui passe.",
            "Sorts an array by calling the comparison function it is given.",
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
        note(
            "Choisit la graine de rand : la même graine rend la même suite.",
            "Chooses rand's seed: the same seed gives the same sequence.",
        ),
    ),
    (
        "time",
        note(
            "L'heure courante, en secondes.",
            "The current time, in seconds.",
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
];

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
