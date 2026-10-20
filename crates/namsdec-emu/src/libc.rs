//! Une poignée de fonctions de bibliothèque, écrites en Rust.
//!
//! namsdec ne charge aucune bibliothèque : il n'est pas un éditeur de
//! liens dynamique, et le code de `printf` n'est nulle part dans la
//! mémoire simulée. Le chargeur pose donc une **borne** à la place de
//! chaque fonction qu'un binaire emprunte — et quand l'exécution arrive
//! sur l'une d'elles, c'est ici qu'on répond.
//!
//! Ce n'est pas une libc. C'est ce qu'il faut pour qu'un programme
//! ordinaire — celui qu'on trouve dans un exercice de rétro-ingénierie —
//! aille jusqu'au bout : afficher, lire, comparer des chaînes, demander
//! de la mémoire. Chaque fonction fournie l'est **en le disant** : la
//! liste se lit dans `PROVIDED`, et ce qui n'y est pas arrête l'exécution
//! en se nommant, plutôt que de rendre une valeur inventée qui ferait
//! croire à un programme cassé.
//!
//! Trois règles tenues d'un bout à l'autre :
//!
//! - **Les droits s'appliquent.** Un `strcpy` vers `.rodata` échoue ici
//!   comme il échouerait sur la vraie machine : les accès passent par la
//!   mémoire simulée et ses zones. C'est la moitié de l'intérêt.
//! - **Tout est réversible.** Ce qu'une fonction écrit est journalisé
//!   comme le ferait une instruction, et « pas en arrière » la défait.
//! - **Rien n'est inventé.** Une fonction qu'on ne sait pas faire n'est
//!   pas approchée : elle s'arrête et se nomme.

use namsdec_core::Reg64;

use crate::{EmuError, Emulator};

/// Ce que l'appel a fait du flot d'exécution.
pub(crate) enum Flow {
    /// La fonction a rendu la main à son appelant : il faut dépiler
    /// l'adresse de retour.
    Returned,
    /// La fonction a elle-même décidé où aller — c'est le cas de
    /// `__libc_start_main`, qui saute dans `main`.
    Jumped,
    /// La machine s'arrête : `exit` est passé par là.
    Halted,
}

/// Les fonctions que namsdec fournit, par ordre alphabétique.
///
/// C'est la liste que l'interface montre, et c'est elle qui décide : ce
/// qui n'y est pas s'arrête en se nommant.
pub const PROVIDED: &[&str] = &[
    "_ITM_deregisterTMCloneTable",
    "_ITM_registerTMCloneTable",
    "_Unwind_ForcedUnwind",
    "_Unwind_RaiseException",
    "_Unwind_Resume",
    "__assert_fail",
    "__ctype_b_loc",
    "__ctype_tolower_loc",
    "__ctype_toupper_loc",
    "__cxa_atexit",
    "__cxa_finalize",
    "__errno_location",
    "__gmon_start__",
    "__libc_start_main",
    "__stack_chk_fail",
    "__xpg_strerror_r",
    "_exit",
    "abort",
    "abs",
    "aligned_alloc",
    "atexit",
    "atoi",
    "atol",
    "atoll",
    "bcmp",
    "calloc",
    "clearerr",
    "clock",
    "closedir",
    "dl_iterate_phdr",
    "exit",
    "fclose",
    "fdopen",
    "feof",
    "ferror",
    "fflush",
    "fflush_unlocked",
    "fgets",
    "fileno",
    "fileno_unlocked",
    "fopen",
    "fopen64",
    "fprintf",
    "fputc",
    "fputs",
    "fread",
    "fread_unlocked",
    "free",
    "freopen",
    "fseek",
    "fseeko",
    "fseeko64",
    "ftell",
    "ftello",
    "ftello64",
    "fwrite",
    "getauxval",
    "getchar",
    "getegid",
    "getenv",
    "geteuid",
    "getgid",
    "getpid",
    "getppid",
    "gets",
    "getuid",
    "index",
    "isalnum",
    "isalpha",
    "isascii",
    "isatty",
    "isblank",
    "iscntrl",
    "isdigit",
    "isgraph",
    "islower",
    "isprint",
    "ispunct",
    "isspace",
    "isupper",
    "isxdigit",
    "malloc",
    "malloc_usable_size",
    "memchr",
    "memcmp",
    "memcpy",
    "memmem",
    "memmove",
    "mempcpy",
    "memrchr",
    "memset",
    "opendir",
    "perror",
    "posix_memalign",
    "prctl",
    "printf",
    "pthread_attr_destroy",
    "pthread_attr_getguardsize",
    "pthread_attr_getstack",
    "pthread_attr_init",
    "pthread_getattr_np",
    "pthread_getspecific",
    "pthread_key_create",
    "pthread_mutex_destroy",
    "pthread_mutex_init",
    "pthread_mutex_lock",
    "pthread_mutex_unlock",
    "pthread_once",
    "pthread_self",
    "pthread_setspecific",
    "ptrace",
    "putchar",
    "puts",
    "raise",
    "rand",
    "read",
    "readdir",
    "readdir64",
    "realloc",
    "rewind",
    "rindex",
    "scanf",
    "setbuf",
    "setlinebuf",
    "setvbuf",
    "sigaction",
    "sigaddset",
    "sigemptyset",
    "signal",
    "sigprocmask",
    "sleep",
    "snprintf",
    "sprintf",
    "srand",
    "sscanf",
    "stpcpy",
    "stpncpy",
    "strcasecmp",
    "strcasestr",
    "strcat",
    "strchr",
    "strchrnul",
    "strcmp",
    "strcpy",
    "strcspn",
    "strdup",
    "strerror",
    "strlen",
    "strncasecmp",
    "strncat",
    "strncmp",
    "strncpy",
    "strnlen",
    "strpbrk",
    "strrchr",
    "strspn",
    "strstr",
    "strtok",
    "strtok_r",
    "strtol",
    "strtoll",
    "strtoul",
    "strtoull",
    "syscall",
    "sysconf",
    "time",
    "tmpfile",
    "tolower",
    "toupper",
    "usleep",
    "write",
];

/// namsdec sait-il faire cette fonction ?
///
/// Une seule source de vérité : les enveloppes d'appels système, et la
/// liste des fonctions écrites ici. C'est ce que l'interface montre après
/// avoir chargé un binaire — et donc ce qui répond à « est-ce que ce
/// programme-là va tourner ? ».
pub fn provides(name: &str) -> bool {
    let court = canonical(name);
    wrapper(court).is_some() || PROVIDED.contains(&court)
}

/// Cette fonction attend-elle l'entrée standard ?
///
/// Posée avant l'appel, comme pour `read` : le pas rend la main, et
/// l'utilisateur tape sa ligne. Sans cela, un `fgets` sur une file vide
/// rendrait « fin de fichier » à un programme qui attend une réponse.
pub(crate) fn reads_input(name: &str) -> bool {
    matches!(
        name,
        "fgets" | "gets" | "getchar" | "read" | "scanf" | "__isoc99_scanf"
    )
}

/// Ce nom-là désigne-t-il une fonction qui réclame une entrée ?
///
/// Publique, et prenant le nom tel que l'éditeur de liens l'a écrit :
/// l'interface s'en sert pour ouvrir la ligne de saisie dès qu'un binaire
/// est chargé, sans attendre qu'il se bloque dessus. Un programme lié
/// dynamiquement ne fait aucun `syscall` de lecture — il appelle `fgets`
/// ou `scanf` —, et rien ne disait donc qu'il demanderait quelque chose.
pub fn name_reads_input(name: &str) -> bool {
    reads_input(canonical(name))
}

/// Le nom, dépouillé de ce que l'éditeur de liens y ajoute.
///
/// `strcpy@GLIBC_2.2.5`, `__strcpy_chk`, `puts@plt` : trois façons de
/// nommer la même chose. On ne fournit pas les variantes « _chk », qui
/// vérifient une taille connue à la compilation, mais leur cousine
/// ordinaire fait exactement le même travail.
fn canonical(name: &str) -> &str {
    let name = name.split('@').next().unwrap_or(name);
    // `__isoc99_scanf` hier, `__isoc23_scanf` depuis la glibc 2.38 : la
    // norme C change de millésime, le préfixe suit, et la fonction fait
    // toujours la même chose. On les retire tous plutôt que d'attendre le
    // prochain — sans quoi un binaire compilé cette année s'arrête sur un
    // « scanf » que namsdec fournit pourtant.
    let name = name
        .strip_prefix("__isoc99_")
        .or_else(|| name.strip_prefix("__isoc23_"))
        .or_else(|| name.strip_prefix("__isoc11_"))
        .unwrap_or(name);
    // `__snprintf_chk`, `__fread_chk`, `__realpath_chk` : l'habillage de
    // FORTIFY_SOURCE. Il ajoute un argument de taille devant les autres
    // et fait, pour le reste, le travail de la fonction d'origine.
    //
    // Les deux marques se retirent ENSEMBLE, et la règle vaut mieux que
    // la liste : il y avait ici dix correspondances écrites à la main —
    // `__printf` vers `printf`, et neuf autres —, si bien que chaque
    // nouvelle fonction fortifiée demandait une ligne de plus, et
    // `__fread_chk` s'arrêtait faute de l'avoir écrite. Le préfixe `__`
    // n'est retiré QUE si le suffixe `_chk` était là : `__errno_location`
    // et `__ctype_b_loc` ne sont pas des habillages, ce sont des
    // fonctions à part entière.
    match name.strip_suffix("_chk") {
        Some(nu) => nu.strip_prefix("__").unwrap_or(nu),
        None => name,
    }
}

/// Les fonctions qui ne sont qu'une **enveloppe** autour d'un appel
/// système, avec le numéro qu'elles appellent.
///
/// `open`, `close`, `mmap`, `poll` : la fonction C ne fait rien d'autre
/// que ranger ses arguments et exécuter `syscall`. Les faire passer par
/// la même porte que l'instruction évite d'écrire deux fois le même
/// travail — et laisse au noyau simulé le soin de dire non quand il ne
/// sait pas faire, en nommant l'appel.
fn wrapper(name: &str) -> Option<u64> {
    Some(match name {
        "mmap" | "mmap64" => 9,
        "mprotect" => 10,
        "munmap" => 11,
        "open" | "open64" => 2,
        "openat" => 257,
        "close" => 3,
        "lseek" | "lseek64" => 8,
        "fstat" | "fstat64" => 5,
        "stat" | "stat64" => 4,
        "ioctl" => 16,
        "fcntl" => 72,
        "dup" => 32,
        "poll" => 7,
        "sigaction" | "rt_sigaction" | "signal" => 13,
        "sigprocmask" | "pthread_sigmask" => 14,
        "sigaltstack" => 131,
        "getpid" => 39,
        "gettid" => 186,
        "getuid" => 102,
        "geteuid" => 107,
        "getgid" => 104,
        "getegid" => 108,
        "getppid" => 110,
        "getrandom" => 318,
        "nanosleep" => 35,
        "sched_yield" => 24,
        "uname" => 63,
        "readlink" => 89,
        "getcwd" => 79,
        "brk" | "sbrk" => 12,
        "writev" => 20,
        "readv" => 19,
        "clock_gettime" => 228,
        "gettimeofday" => 96,
        "time" => 201,
        "futex" => 202,
        "prlimit64" => 302,
        "getrlimit" => 97,
        "exit_group" => 231,
        _ => return None,
    })
}

/// Appelle la fonction `name`, arguments lus dans les registres.
pub(crate) fn call(emu: &mut Emulator, name: &str) -> Result<Flow, EmuError> {
    let court = canonical(name);
    // Une simple enveloppe ? Alors c'est le noyau simulé qui répond. La
    // convention diffère d'un cran : le quatrième argument d'un appel
    // système passe par R10 et non par RCX.
    if let Some(numéro) = wrapper(court) {
        let quatrième = emu.cpu.regs.get64(Reg64::Rcx);
        emu.cpu.regs.set64(Reg64::R10, quatrième);
        emu.cpu.regs.set64(Reg64::Rax, numéro);
        emu.syscall()?;
        return Ok(Flow::Returned);
    }
    match court {
        // --- Le démarrage et la fin ------------------------------------
        "__libc_start_main" => return libc_start_main(emu),
        "exit" | "_exit" => {
            let code = emu.cpu.regs.get64(Reg64::Rdi) as u8;
            emu.exit_code = Some(code as i64);
            emu.cpu.halted = true;
            return Ok(Flow::Halted);
        }
        "abort" => {
            // `abort` ne rend pas la main : le programme meurt sur
            // SIGABRT, et le code de sortie d'un shell le dit 134.
            emu.exit_code = Some(134);
            emu.cpu.halted = true;
            return Ok(Flow::Halted);
        }
        "__stack_chk_fail" => {
            // Le canari a changé : un tampon a débordé. C'est un
            // diagnostic, pas un plantage ordinaire — et c'est
            // exactement ce qu'on cherchait à provoquer, souvent.
            emu.output
                .extend_from_slice(b"*** stack smashing detected ***\n");
            emu.exit_code = Some(134);
            emu.cpu.halted = true;
            return Ok(Flow::Halted);
        }
        "__assert_fail" => {
            let message = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let fichier = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let ligne = emu.cpu.regs.get64(Reg64::Rdx);
            let texte = format!(
                "assertion échouée : {} ({}:{ligne})\n",
                String::from_utf8_lossy(&message),
                String::from_utf8_lossy(&fichier)
            );
            emu.output.extend_from_slice(texte.as_bytes());
            emu.exit_code = Some(134);
            emu.cpu.halted = true;
            return Ok(Flow::Halted);
        }
        // Les fonctions du démarrage qui n'ont rien à faire sans
        // bibliothèque : les dire réussies est la vérité.
        "__cxa_atexit"
        | "atexit"
        | "__cxa_finalize"
        | "__gmon_start__"
        | "_ITM_registerTMCloneTable"
        | "_ITM_deregisterTMCloneTable" => set(emu, 0),

        // --- Écrire ----------------------------------------------------
        "puts" => {
            let texte = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let n = texte.len() + 1;
            emu.output.extend_from_slice(&texte);
            emu.output.push(b'\n');
            set(emu, n as u64);
        }
        // `putchar(c)` et `fputc(c, flux)` portent tous deux le caractère
        // dans le premier registre : le flux, lui, ne peut être que la
        // sortie.
        "putchar" | "fputc" => {
            let c = emu.cpu.regs.get64(Reg64::Rdi) as u8;
            emu.output.push(c);
            set(emu, c as u64);
        }
        "fputs" => {
            let texte = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let n = texte.len();
            emu.output.extend_from_slice(&texte);
            set(emu, n as u64);
        }
        // `printf` et ses cousines. Les variantes « _chk » intercalent un
        // drapeau devant le format ; `fprintf` un flux, qui ne peut être
        // que la sortie, faute d'autre part où écrire.
        "printf" | "fprintf" => {
            let saut = usize::from(court == "fprintf") + usize::from(name.contains("_chk"));
            let mut args = VarArgs::new(emu, saut);
            let format = cstr(emu, args.integer(emu))?;
            let texte = format_printf(emu, &format, &mut args)?;
            let n = texte.len();
            emu.output.extend_from_slice(&texte);
            set(emu, n as u64);
        }
        // `sprintf(buf, format, …)` et `snprintf(buf, taille, format, …)`
        // écrivent dans la mémoire du programme, et non sur la sortie.
        "sprintf" | "snprintf" => {
            let dest = emu.cpu.regs.get64(Reg64::Rdi);
            let borné = court == "snprintf";
            let saut = 1 + usize::from(borné) + usize::from(name.contains("_chk")) * 2;
            let taille = if borné {
                emu.cpu.regs.get64(Reg64::Rsi)
            } else {
                u64::MAX
            };
            let mut args = VarArgs::new(emu, saut);
            let format = cstr(emu, args.integer(emu))?;
            let mut texte = format_printf(emu, &format, &mut args)?;
            let complet = texte.len();
            // `snprintf` tronque et termine toujours par un zéro ; il rend
            // pourtant la longueur qu'il **aurait** écrite, et c'est là
            // que se cachent la moitié des débordements.
            if borné && taille > 0 {
                texte.truncate((taille - 1) as usize);
            }
            texte.push(0);
            if taille > 0 {
                put(emu, dest, &texte)?;
            }
            set(emu, complet as u64);
        }
        "fwrite" => {
            // `fwrite(ptr, taille, nombre, flux)` : tout va sur la sortie,
            // il n'y a pas d'autre flux ici.
            let ptr = emu.cpu.regs.get64(Reg64::Rdi);
            let taille = emu.cpu.regs.get64(Reg64::Rsi);
            let nombre = emu.cpu.regs.get64(Reg64::Rdx);
            let total = taille.saturating_mul(nombre);
            if total > 0 {
                let octets = emu.cpu.mem.bytes_at(ptr, total as usize)?.to_vec();
                emu.output.extend_from_slice(&octets);
            }
            set(emu, nombre);
        }
        "perror" => {
            let texte = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            emu.output.extend_from_slice(&texte);
            emu.output.extend_from_slice(b": erreur\n");
            set(emu, 0);
        }
        "fflush" => set(emu, 0),
        "write" => {
            let fd = emu.cpu.regs.get64(Reg64::Rdi);
            let buf = emu.cpu.regs.get64(Reg64::Rsi);
            let n = emu.cpu.regs.get64(Reg64::Rdx);
            let écrit = emu.write(fd, buf, n)?;
            set(emu, écrit as u64);
        }

        // --- Lire ------------------------------------------------------
        "read" => {
            let fd = emu.cpu.regs.get64(Reg64::Rdi);
            let buf = emu.cpu.regs.get64(Reg64::Rsi);
            let n = emu.cpu.regs.get64(Reg64::Rdx);
            let lu = emu.read(fd, buf, n)?;
            set(emu, lu as u64);
        }
        "fgets" | "gets" => {
            // `fgets(buf, taille, flux)` ; `gets(buf)` — celui qu'aucun
            // programme ne devrait employer, et qu'on rencontre dans tous
            // les exercices de débordement.
            let buf = emu.cpu.regs.get64(Reg64::Rdi);
            let taille = if court == "fgets" {
                emu.cpu.regs.get64(Reg64::Rsi)
            } else {
                u64::MAX
            };
            let Some(ligne) = take_line(emu, taille) else {
                // Fin de fichier : `fgets` rend NULL.
                set(emu, 0);
                return Ok(Flow::Returned);
            };
            put(emu, buf, &ligne)?;
            set(emu, buf);
        }
        "getchar" => {
            let Some(octet) = take_byte(emu) else {
                // EOF vaut -1.
                set(emu, u64::MAX);
                return Ok(Flow::Returned);
            };
            set(emu, octet as u64);
        }
        "scanf" | "__isoc99_scanf" => {
            let mut args = VarArgs::new(emu, 0);
            let format = cstr(emu, args.integer(emu))?;
            let lus = scan(emu, &format, &mut args)?;
            set(emu, lus);
        }
        // `sscanf` lit dans une chaîne qu'on lui donne, et non sur
        // l'entrée. C'est la façon dont un crackme découpe ce qu'on
        // vient de taper — « %d-%d-%d » sur un numéro de série — et il
        // manquait, alors que `scanf` était là depuis le début.
        "sscanf" => {
            let mut args = VarArgs::new(emu, 0);
            let source = cstr(emu, args.integer(emu))?;
            let format = cstr(emu, args.integer(emu))?;
            let lus = scan_dans(emu, &source, &format, &mut args)?;
            set(emu, lus);
        }

        // --- Les chaînes -----------------------------------------------
        "strlen" | "strnlen" => {
            let texte = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let n = if court == "strnlen" {
                (texte.len() as u64).min(emu.cpu.regs.get64(Reg64::Rsi))
            } else {
                texte.len() as u64
            };
            set(emu, n);
        }
        "strcmp" | "strcasecmp" | "strncmp" => {
            let a = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let b = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let (mut a, mut b) = (a, b);
            if court == "strncmp" {
                let n = emu.cpu.regs.get64(Reg64::Rdx) as usize;
                a.truncate(n);
                b.truncate(n);
            }
            if court == "strcasecmp" {
                a.make_ascii_lowercase();
                b.make_ascii_lowercase();
            }
            set(emu, compare(&a, &b));
        }
        "strcpy" | "strncpy" => {
            let dest = emu.cpu.regs.get64(Reg64::Rdi);
            let mut source = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            source.push(0);
            if court == "strncpy" {
                let n = emu.cpu.regs.get64(Reg64::Rdx) as usize;
                source.resize(n, 0);
            }
            put(emu, dest, &source)?;
            set(emu, dest);
        }
        "strcat" | "strncat" => {
            let dest = emu.cpu.regs.get64(Reg64::Rdi);
            let déjà = cstr(emu, dest)?;
            let mut source = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            if court == "strncat" {
                source.truncate(emu.cpu.regs.get64(Reg64::Rdx) as usize);
            }
            source.push(0);
            put(emu, dest + déjà.len() as u64, &source)?;
            set(emu, dest);
        }
        "strchr" | "strrchr" => {
            let base = emu.cpu.regs.get64(Reg64::Rdi);
            let texte = cstr(emu, base)?;
            let cherché = emu.cpu.regs.get64(Reg64::Rsi) as u8;
            // Le zéro final compte : `strchr(s, 0)` rend la fin de chaîne.
            let position = if court == "strchr" {
                texte.iter().position(|o| *o == cherché)
            } else {
                texte.iter().rposition(|o| *o == cherché)
            }
            .or_else(|| (cherché == 0).then_some(texte.len()));
            set(emu, position.map(|p| base + p as u64).unwrap_or(0));
        }
        "strstr" => {
            let base = emu.cpu.regs.get64(Reg64::Rdi);
            let botte = cstr(emu, base)?;
            let aiguille = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let position = if aiguille.is_empty() {
                Some(0)
            } else {
                botte
                    .windows(aiguille.len())
                    .position(|fenêtre| fenêtre == aiguille)
            };
            set(emu, position.map(|p| base + p as u64).unwrap_or(0));
        }
        // Les trois fonctions qui découpent une chaîne sur un jeu de
        // caractères. `strcspn(ligne, "\n")` est la façon canonique de
        // retirer le saut de ligne que `fgets` laisse derrière lui.
        "strspn" | "strcspn" => {
            let texte = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let jeu = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let dedans = court == "strspn";
            let n = texte
                .iter()
                .position(|o| jeu.contains(o) != dedans)
                .unwrap_or(texte.len());
            set(emu, n as u64);
        }
        "strpbrk" => {
            let base = emu.cpu.regs.get64(Reg64::Rdi);
            let texte = cstr(emu, base)?;
            let jeu = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let position = texte.iter().position(|o| jeu.contains(o));
            set(emu, position.map(|p| base + p as u64).unwrap_or(0));
        }
        "strdup" => {
            let mut texte = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            texte.push(0);
            let Some(bloc) = emu.libc_alloc(texte.len() as u64) else {
                set(emu, 0);
                return Ok(Flow::Returned);
            };
            put(emu, bloc, &texte)?;
            set(emu, bloc);
        }
        // `strtoul`, `strtoll`, `strtoull` font le même travail que
        // `strtol` : même grammaire, même second argument, et seule la
        // largeur du résultat change — or elle tient dans RAX de la
        // même façon. Les séparer aurait recopié trente lignes pour
        // écrire quatre fois la même chose.
        "atoi" | "atol" | "atoll" | "strtol" | "strtoul" | "strtoll" | "strtoull" => {
            let texte = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let avec_base = court.starts_with("strto");
            let base = if avec_base {
                emu.cpu.regs.get64(Reg64::Rdx) as u32
            } else {
                10
            };
            let (valeur, lus) = parse_int(&texte, if base == 0 { 10 } else { base });
            if avec_base {
                // Le second argument reçoit où l'analyse s'est arrêtée.
                let fin = emu.cpu.regs.get64(Reg64::Rsi);
                if fin != 0 {
                    let après = emu.cpu.regs.get64(Reg64::Rdi) + lus as u64;
                    let octets = après.to_le_bytes();
                    emu.record_memory(fin, 8);
                    emu.cpu.mem.write_bytes(fin, &octets)?;
                }
            }
            set(emu, valeur as u64);
        }
        // LES CLASSES DE CARACTÈRES, au complet.
        //
        // Elles manquaient toutes, et c'est le genre d'absence qui
        // arrête un programme en pleine vérification de mot de passe :
        // « ce caractère est-il un chiffre ? » est la première question
        // qu'une validation se pose. Le compilateur les écrit en appel
        // quand il ne peut pas les développer en ligne.
        //
        // `is*` rend un entier NON NUL, pas forcément 1 : c'est ce que
        // la norme dit, et un programme qui compare à 1 est déjà cassé
        // sur une vraie libc.
        "isalpha" | "isdigit" | "isalnum" | "isspace" | "isupper" | "islower" | "ispunct"
        | "isprint" | "isxdigit" | "iscntrl" | "isgraph" | "isblank" | "isascii" => {
            let brut = emu.cpu.regs.get64(Reg64::Rdi);
            // L'argument est un `int` : EOF (-1) n'est aucune classe, et
            // tout ce qui dépasse un octet non plus.
            let c = u8::try_from(brut & 0xff).unwrap_or(0);
            let hors_ascii = brut > 0xff;
            let vrai = !hors_ascii
                && match court {
                    "isalpha" => c.is_ascii_alphabetic(),
                    "isdigit" => c.is_ascii_digit(),
                    "isalnum" => c.is_ascii_alphanumeric(),
                    "isspace" => c.is_ascii_whitespace() || c == 0x0b,
                    "isupper" => c.is_ascii_uppercase(),
                    "islower" => c.is_ascii_lowercase(),
                    "ispunct" => c.is_ascii_punctuation(),
                    "isprint" => c.is_ascii_graphic() || c == b' ',
                    "isxdigit" => c.is_ascii_hexdigit(),
                    "iscntrl" => c.is_ascii_control(),
                    "isgraph" => c.is_ascii_graphic(),
                    "isblank" => c == b' ' || c == b'\t',
                    _ => c.is_ascii(),
                };
            set(emu, u64::from(vrai));
        }
        "toupper" | "tolower" => {
            let c = emu.cpu.regs.get64(Reg64::Rdi) as u8;
            let rendu = if court == "toupper" {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            set(emu, rendu as u64);
        }
        "abs" => {
            let v = emu.cpu.regs.get64(Reg64::Rdi) as i32;
            set(emu, v.unsigned_abs() as u64);
        }

        // --- La mémoire ------------------------------------------------
        "memcpy" | "mempcpy" | "memmove" => {
            let dest = emu.cpu.regs.get64(Reg64::Rdi);
            let source = emu.cpu.regs.get64(Reg64::Rsi);
            let n = emu.cpu.regs.get64(Reg64::Rdx);
            if n > 0 {
                let octets = emu.cpu.mem.bytes_at(source, n as usize)?.to_vec();
                put(emu, dest, &octets)?;
            }
            set(emu, if court == "mempcpy" { dest + n } else { dest });
        }
        "memset" => {
            let dest = emu.cpu.regs.get64(Reg64::Rdi);
            let octet = emu.cpu.regs.get64(Reg64::Rsi) as u8;
            let n = emu.cpu.regs.get64(Reg64::Rdx);
            if n > 0 {
                put(emu, dest, &vec![octet; n as usize])?;
            }
            set(emu, dest);
        }
        "memcmp" | "bcmp" => {
            let n = emu.cpu.regs.get64(Reg64::Rdx) as usize;
            let a = emu
                .cpu
                .mem
                .bytes_at(emu.cpu.regs.get64(Reg64::Rdi), n)?
                .to_vec();
            let b = emu
                .cpu
                .mem
                .bytes_at(emu.cpu.regs.get64(Reg64::Rsi), n)?
                .to_vec();
            set(emu, compare(&a, &b));
        }
        "memchr" => {
            let base = emu.cpu.regs.get64(Reg64::Rdi);
            let cherché = emu.cpu.regs.get64(Reg64::Rsi) as u8;
            let n = emu.cpu.regs.get64(Reg64::Rdx) as usize;
            let octets = emu.cpu.mem.bytes_at(base, n)?.to_vec();
            let position = octets.iter().position(|o| *o == cherché);
            set(emu, position.map(|p| base + p as u64).unwrap_or(0));
        }
        "malloc" => {
            let taille = emu.cpu.regs.get64(Reg64::Rdi);
            let bloc = emu.libc_alloc(taille).unwrap_or(0);
            set(emu, bloc);
        }
        "calloc" => {
            let taille = emu
                .cpu
                .regs
                .get64(Reg64::Rdi)
                .saturating_mul(emu.cpu.regs.get64(Reg64::Rsi));
            let bloc = emu.libc_alloc(taille).unwrap_or(0);
            if bloc != 0 && taille > 0 {
                // `calloc` rend de la mémoire **à zéro** : c'est toute sa
                // différence avec `malloc`.
                put(emu, bloc, &vec![0u8; taille as usize])?;
            }
            set(emu, bloc);
        }
        "realloc" => {
            let ancien = emu.cpu.regs.get64(Reg64::Rdi);
            let taille = emu.cpu.regs.get64(Reg64::Rsi);
            let Some(bloc) = emu.libc_alloc(taille) else {
                set(emu, 0);
                return Ok(Flow::Returned);
            };
            if ancien != 0 {
                let avant = emu.block_size(ancien).min(taille);
                if avant > 0 {
                    let octets = emu.cpu.mem.bytes_at(ancien, avant as usize)?.to_vec();
                    put(emu, bloc, &octets)?;
                }
            }
            set(emu, bloc);
        }
        "aligned_alloc" => {
            let taille = emu.cpu.regs.get64(Reg64::Rsi);
            let bloc = emu.libc_alloc(taille).unwrap_or(0);
            set(emu, bloc);
        }
        "posix_memalign" => {
            let où = emu.cpu.regs.get64(Reg64::Rdi);
            let taille = emu.cpu.regs.get64(Reg64::Rdx);
            let bloc = emu.libc_alloc(taille).unwrap_or(0);
            emu.record_memory(où, 8);
            emu.cpu.mem.write_bytes(où, &bloc.to_le_bytes())?;
            set(emu, if bloc == 0 { 12 } else { 0 });
        }
        "malloc_usable_size" => {
            let taille = emu.block_size(emu.cpu.regs.get64(Reg64::Rdi));
            set(emu, taille);
        }
        // Rendre la mémoire ne la reprend pas : l'allocateur de namsdec
        // est un curseur qui monte. Un programme qui relit ce qu'il a
        // libéré retrouve donc ses données — un vrai `malloc` en fait
        // souvent autant, et c'est exactement ce qui rend le bogue si
        // difficile à voir.
        "free" => set(emu, 0),

        // --- Le reste --------------------------------------------------
        "__errno_location" => {
            let adresse = emu.errno_slot();
            set(emu, adresse);
        }
        "prctl" => {
            // `prctl` règle mille détails d'un processus : son nom, sa
            // graine de hasard, son droit de laisser un fichier de
            // vidange. Rien de tout cela n'existe ici.
            //
            // On répond « c'est fait ». C'est vrai au sens qui compte :
            // il n'y avait rien à régler. Le seul usage qu'en fait un
            // crackme est `PR_SET_DUMPABLE` à zéro, juste avant de se
            // dédoubler — une façon d'interdire qu'on l'attache. Ce
            // verrou-là ne ferme rien sur une machine qui n'a qu'un
            // processus.
            //
            // Ce que cela coûte : un `PR_GET_…` qui attend une réponse
            // dans un pointeur ne recevra rien. On sait dire oui, on ne
            // sait pas raconter un processus qui n'existe pas.
            set(emu, 0);
        }
        "ptrace" => {
            // Le programme demande si quelqu'un le regarde. C'est
            // l'anti-débogage le plus répandu : il s'attache à lui-même
            // par `PTRACE_TRACEME`, et si l'appel échoue, c'est qu'un
            // débogueur l'a devancé.
            //
            // namsdec répond zéro — « personne ne te regarde ». C'est un
            // mensonge, et il est délibéré : l'outil est fait pour
            // observer des programmes qui préféreraient ne pas l'être, et
            // s'arrêter sur la question reviendrait à leur donner raison.
            //
            // Ce que cela coûte : un programme qui se servirait vraiment
            // de `ptrace` pour piloter un fils n'en tirerait rien. On ne
            // sait pas tracer, on sait seulement dire qu'on ne trace pas.
            set(emu, 0);
        }
        "getenv" => {
            // L'environnement que le chargeur a posé sur la pile n'est pas
            // relu ici : namsdec n'en promet aucun, et une variable
            // absente est une réponse parfaitement ordinaire.
            set(emu, 0);
        }
        "sysconf" => {
            // `_SC_PAGESIZE` vaut 30 ; le reste, on ne le sait pas.
            let quoi = emu.cpu.regs.get64(Reg64::Rdi);
            set(emu, if quoi == 30 { 4096 } else { u64::MAX });
        }
        "rand" => {
            let valeur = emu.pseudo_random_u32();
            set(emu, valeur as u64);
        }
        "srand" => set(emu, 0),
        // Les fils d'exécution : il n'y en a qu'un, et il est là.
        "pthread_self" => {
            let moi = emu.cpu.regs.fs_base;
            set(emu, moi);
        }
        "pthread_attr_init"
        | "pthread_attr_destroy"
        | "pthread_mutex_lock"
        | "pthread_mutex_unlock"
        | "pthread_mutex_init"
        | "pthread_mutex_destroy"
        | "pthread_once"
        | "pthread_key_create"
        | "pthread_setspecific" => set(emu, 0),
        "pthread_getspecific" => set(emu, 0),
        // « Où est ma pile ? » — la bibliothèque standard de Rust le
        // demande pour poser une garde contre le débordement. Répondre
        // « je ne sais pas » est prévu : elle s'en passe.
        "pthread_getattr_np" | "pthread_attr_getstack" | "pthread_attr_getguardsize" => {
            set(emu, 38)
        }
        // Parcourir les objets chargés : il n'y en a aucun.
        "dl_iterate_phdr" => set(emu, 0),
        "getauxval" => {
            // Seule la taille de page se connaît d'ici.
            let quoi = emu.cpu.regs.get64(Reg64::Rdi);
            set(emu, if quoi == 6 { 4096 } else { 0 });
        }
        "strerror" | "__xpg_strerror_r" => {
            // Pas de table de messages : un texte unique, mais vrai.
            set(emu, 0);
        }
        // --- Les chaînes, la suite ------------------------------------
        //
        // Ce lot vient d'un COMPTE et non d'une intuition : les imports
        // des onze binaires d'épreuve et de la collection personnelle,
        // moins ce que namsdec fournit déjà. Ce qui suit est ce que ce
        // relevé a nommé, plus ses voisins immédiats — ajouter `strtok`
        // sans `strtok_r` reviendrait à s'arrêter au milieu.
        "strncasecmp" => {
            let a = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let b = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let n = emu.cpu.regs.get64(Reg64::Rdx) as usize;
            let coupe = |v: &[u8]| -> Vec<u8> {
                v.iter().take(n).map(|o| o.to_ascii_lowercase()).collect()
            };
            set(emu, compare(&coupe(&a), &coupe(&b)));
        }
        // `stpcpy` est `strcpy` qui rend la FIN plutôt que le début :
        // c'est ce qui permet d'enchaîner des copies sans reparcourir.
        "stpcpy" | "stpncpy" => {
            let dst = emu.cpu.regs.get64(Reg64::Rdi);
            let src = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let mut octets = src.clone();
            if court == "stpncpy" {
                let n = emu.cpu.regs.get64(Reg64::Rdx) as usize;
                octets.truncate(n);
                octets.resize(n, 0);
            } else {
                octets.push(0);
            }
            put(emu, dst, &octets)?;
            // La fin, c'est le zéro final — ou le bout pour `stpncpy`.
            let fin = dst + src.len().min(octets.len()) as u64;
            set(emu, fin);
        }
        "memmem" => {
            let foin = emu.cpu.regs.get64(Reg64::Rdi);
            let n = emu.cpu.regs.get64(Reg64::Rsi) as usize;
            let aiguille_at = emu.cpu.regs.get64(Reg64::Rdx);
            let m = emu.cpu.regs.get64(Reg64::Rcx) as usize;
            let foin_octets = emu.cpu.mem.bytes_at(foin, n)?.to_vec();
            let aiguille = emu.cpu.mem.bytes_at(aiguille_at, m)?.to_vec();
            // Une aiguille vide se trouve au début : c'est ce que dit la
            // page de manuel, et c'est le cas qu'on oublie.
            let trouvé = if aiguille.is_empty() {
                Some(0)
            } else {
                foin_octets
                    .windows(aiguille.len())
                    .position(|f| f == aiguille)
            };
            set(emu, trouvé.map(|p| foin + p as u64).unwrap_or(0));
        }
        "strcasestr" => {
            let foin = cstr(emu, emu.cpu.regs.get64(Reg64::Rdi))?;
            let aiguille = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let bas = |v: &[u8]| -> Vec<u8> { v.iter().map(|o| o.to_ascii_lowercase()).collect() };
            let (f, a) = (bas(&foin), bas(&aiguille));
            let trouvé = if a.is_empty() {
                Some(0)
            } else if a.len() > f.len() {
                None
            } else {
                f.windows(a.len()).position(|w| w == a)
            };
            set(
                emu,
                trouvé
                    .map(|p| emu.cpu.regs.get64(Reg64::Rdi) + p as u64)
                    .unwrap_or(0),
            );
        }
        // `strchrnul` rend le zéro final plutôt que NULL : glibc
        // l'emploie partout en interne, et un binaire compilé contre
        // elle s'y arrête.
        "strchrnul" | "index" | "rindex" => {
            let base = emu.cpu.regs.get64(Reg64::Rdi);
            let texte = cstr(emu, base)?;
            let cherché = emu.cpu.regs.get64(Reg64::Rsi) as u8;
            let position = if court == "rindex" {
                texte.iter().rposition(|o| *o == cherché)
            } else {
                texte.iter().position(|o| *o == cherché)
            };
            let rendu = match (position, court) {
                (Some(p), _) => base + p as u64,
                // Pas trouvé : `strchrnul` rend la fin, les deux autres
                // rendent rien.
                (None, "strchrnul") => base + texte.len() as u64,
                (None, _) => 0,
            };
            set(emu, rendu);
        }
        // `strtok` DÉCOUPE EN PLACE et retient où il en était. Ce qu'il
        // retient vit dans la mémoire simulée (voir `strtok_slot`) pour
        // que « pas en arrière » le défasse comme le reste.
        "strtok" | "strtok_r" => {
            let case = if court == "strtok_r" {
                emu.cpu.regs.get64(Reg64::Rdx)
            } else {
                emu.strtok_slot()
            };
            let donné = emu.cpu.regs.get64(Reg64::Rdi);
            let départ = if donné != 0 {
                donné
            } else if case != 0 {
                emu.cpu.mem.read_u64(case)?
            } else {
                0
            };
            if départ == 0 {
                set(emu, 0);
                return Ok(Flow::Returned);
            }
            let séparateurs = cstr(emu, emu.cpu.regs.get64(Reg64::Rsi))?;
            let texte = cstr(emu, départ)?;
            // On saute les séparateurs de tête, puis on coupe au premier
            // suivant — que l'on remplace par un zéro, en place.
            let début = texte
                .iter()
                .position(|o| !séparateurs.contains(o))
                .unwrap_or(texte.len());
            if début >= texte.len() {
                if case != 0 {
                    emu.record_memory(case, 8);
                    emu.cpu.mem.write_bytes(case, &0u64.to_le_bytes())?;
                }
                set(emu, 0);
                return Ok(Flow::Returned);
            }
            let fin = texte[début..]
                .iter()
                .position(|o| séparateurs.contains(o))
                .map(|p| début + p);
            let jeton = départ + début as u64;
            let suite = match fin {
                Some(fin) => {
                    let at = départ + fin as u64;
                    emu.record_memory(at, 1);
                    emu.cpu.mem.write_bytes(at, &[0])?;
                    at + 1
                }
                None => départ + texte.len() as u64,
            };
            if case != 0 {
                emu.record_memory(case, 8);
                emu.cpu.mem.write_bytes(case, &suite.to_le_bytes())?;
            }
            set(emu, jeton);
        }
        // LES TABLES DE CLASSES DE CARACTÈRES.
        //
        // glibc ne compile pas `isalpha(c)` en appel : elle le compile
        // en lecture de table — `(*__ctype_b_loc())[c] & _ISalpha`. La
        // fonction rend donc un pointeur VERS un pointeur vers une
        // table indexée de -128 à 255, et c'est ce double niveau qui
        // surprend. Sans elle, tout programme qui emploie les macros de
        // `<ctype.h>` s'arrête au premier caractère examiné.
        //
        // La table est bâtie une fois dans la mémoire simulée, et les
        // trois fonctions partagent le même travail.
        "__ctype_b_loc" | "__ctype_tolower_loc" | "__ctype_toupper_loc" => {
            let adresse = table_ctype(emu, court)?;
            set(emu, adresse);
        }
        // --- Le temps, l'identité, le terminal ------------------------
        //
        // Tous rendent une valeur STABLE : le même programme doit donner
        // deux fois le même résultat, sans quoi revenir en arrière ne
        // voudrait plus rien dire. C'est la même règle que pour `rand`,
        // dont la suite est reproductible ici.
        "time" => {
            // Une date fixe, et assumée : le 20 octobre 2026 à minuit
            // UTC. Un programme qui l'affiche affichera toujours la
            // même, et c'est ce qu'on veut d'un atelier.
            const QUAND: u64 = 1_792_195_200;
            let où = emu.cpu.regs.get64(Reg64::Rdi);
            if où != 0 {
                emu.record_memory(où, 8);
                emu.cpu.mem.write_bytes(où, &QUAND.to_le_bytes())?;
            }
            set(emu, QUAND);
        }
        "clock" => set(emu, emu.ticks),
        "getpid" => set(emu, 4242),
        "getppid" => set(emu, 4241),
        "getuid" | "geteuid" | "getgid" | "getegid" => set(emu, 1000),
        // Pas de terminal : la sortie de namsdec est une console, mais
        // ce n'est pas un tty, et répondre « oui » ferait prendre au
        // programme un chemin de couleurs et de curseurs qu'il ne
        // pourrait pas tenir.
        "isatty" => {
            set(emu, 0);
            let case = emu.errno_slot();
            if case != 0 {
                // ENOTTY : la réponse honnête, et celle qu'un programme
                // regarde après un `isatty` négatif.
                emu.record_memory(case, 8);
                emu.cpu.mem.write_bytes(case, &25u64.to_le_bytes())?;
            }
        }
        // Poser un gestionnaire de signal : accepté et retenu nulle
        // part. namsdec ne livre aucun signal — il n'y a ni noyau ni
        // horloge pour en envoyer —, donc le gestionnaire ne servirait
        // jamais. Rendre « l'ancien était le comportement par défaut »
        // est exact, et laisse le programme continuer.
        "signal" | "sigaction" | "sigemptyset" | "sigaddset" | "sigprocmask" => set(emu, 0),
        // `raise`, en revanche, ARRIVE : le programme se l'envoie à
        // lui-même, et il n'y a pas de gestionnaire pour l'attraper.
        "raise" => {
            let signal = emu.cpu.regs.get64(Reg64::Rdi);
            emu.output.extend_from_slice(
                format!("*** le programme s'est envoyé le signal {signal} ***\n").as_bytes(),
            );
            emu.exit_code = Some(128 + (signal as i64 & 0x7f));
            emu.cpu.halted = true;
            return Ok(Flow::Halted);
        }
        // --- LES FICHIERS, ET CE QU'ON EN DIT -------------------------
        //
        // namsdec N'A PAS DE SYSTÈME DE FICHIERS. Il n'ouvre rien sur le
        // disque de la personne : un programme analysé est un programme
        // dont on ne sait rien, et lui donner la main sur les fichiers
        // serait une décision, pas une commodité.
        //
        // Deux réponses étaient possibles, et le choix se justifie. Ne
        // pas fournir `fopen` du tout arrête le programme en nommant la
        // fonction — exact, mais il s'arrête AVANT d'avoir montré ce
        // qu'il fait quand le fichier manque, qui est souvent la moitié
        // intéressante : « clé introuvable », un message, un chemin de
        // repli. Répondre « introuvable » n'invente RIEN — c'est la
        // vérité, il n'y a pas de fichier — et laisse le programme
        // suivre sa branche d'erreur.
        //
        // La première fois, la console le dit en toutes lettres, pour
        // qu'on ne prenne pas un refus de namsdec pour un refus du
        // système.
        "fopen" | "fopen64" | "freopen" | "fdopen" | "tmpfile" | "opendir" => {
            sans_fichiers(emu, court);
            let case = emu.errno_slot();
            if case != 0 {
                // ENOENT : « ce fichier n'existe pas ».
                emu.record_memory(case, 8);
                emu.cpu.mem.write_bytes(case, &2u64.to_le_bytes())?;
            }
            set(emu, 0);
        }
        // Les suites logiques : on ne peut pas être arrivé ici avec un
        // flux ouvert par nous, puisqu'on n'en ouvre aucun. Elles
        // répondent donc ce que la norme dit d'un flux qui ne donne
        // rien, sans rien inventer.
        "fread" | "__fread_chk" | "fread_unlocked" => set(emu, 0),
        "fclose" | "closedir" | "fflush_unlocked" => set(emu, 0),
        // Fin de fichier : vrai, puisqu'il n'y a rien à lire.
        "feof" => set(emu, 1),
        "ferror" => set(emu, 0),
        "clearerr" | "rewind" | "setbuf" | "setvbuf" | "setlinebuf" => set(emu, 0),
        // Le descripteur des trois flux standards, et rien d'autre.
        "fileno" | "fileno_unlocked" => set(emu, 1),
        "fseek" | "fseeko" | "fseeko64" => set(emu, u64::MAX),
        "ftell" | "ftello" | "ftello64" => set(emu, u64::MAX),
        "readdir" | "readdir64" => set(emu, 0),
        "memrchr" => {
            let base = emu.cpu.regs.get64(Reg64::Rdi);
            let cherché = emu.cpu.regs.get64(Reg64::Rsi) as u8;
            let n = emu.cpu.regs.get64(Reg64::Rdx) as usize;
            let octets = emu.cpu.mem.bytes_at(base, n)?.to_vec();
            let position = octets.iter().rposition(|o| *o == cherché);
            set(emu, position.map(|p| base + p as u64).unwrap_or(0));
        }
        // Le chemin de la panique d'un programme Rust ou C++ : dérouler
        // la pile demanderait d'interpréter les tables d'exception, ce
        // que namsdec ne fait pas. Le programme s'arrête donc là, en le
        // disant — et le pas à pas montre exactement d'où venait la
        // panique.
        "_Unwind_RaiseException" | "_Unwind_Resume" | "_Unwind_ForcedUnwind" => {
            emu.output.extend_from_slice(
                "*** le programme a paniqué — namsdec ne déroule pas la pile ***\n".as_bytes(),
            );
            emu.exit_code = Some(101);
            emu.cpu.halted = true;
            return Ok(Flow::Halted);
        }
        "sleep" | "usleep" => set(emu, 0),
        "syscall" => {
            // L'appel système brut : les arguments glissent d'un cran,
            // et le numéro passe dans RAX.
            let numéro = emu.cpu.regs.get64(Reg64::Rdi);
            let a = emu.cpu.regs.get64(Reg64::Rsi);
            let b = emu.cpu.regs.get64(Reg64::Rdx);
            let c = emu.cpu.regs.get64(Reg64::Rcx);
            let d = emu.cpu.regs.get64(Reg64::R8);
            let e = emu.cpu.regs.get64(Reg64::R9);
            emu.cpu.regs.set64(Reg64::Rax, numéro);
            emu.cpu.regs.set64(Reg64::Rdi, a);
            emu.cpu.regs.set64(Reg64::Rsi, b);
            emu.cpu.regs.set64(Reg64::Rdx, c);
            // Le quatrième argument d'un appel système passe par R10 :
            // c'est *la* différence avec un appel de fonction, et elle
            // vaut d'être faite ici aussi.
            emu.cpu.regs.set64(Reg64::R10, d);
            emu.cpu.regs.set64(Reg64::R8, e);
            emu.syscall()?;
        }
        _ => {
            return Err(EmuError::UnsupportedLibrary {
                backend: Emulator::NAME,
                name: name.to_owned(),
            });
        }
    }
    Ok(Flow::Returned)
}

/// `__libc_start_main(main, argc, argv, …)` : ce qui se passe entre
/// `_start` et la première ligne écrite par quelqu'un.
///
/// La vraie fonction met en place la bibliothèque, les fonctions de
/// sortie, les locales, puis appelle `main` et passe ce qu'il rend à
/// `exit`. Ici, tout ce qui compte est le dernier point : `main` reçoit
/// `argc` et `argv` tels que le chargeur les a posés, et son retour
/// devient le code de sortie.
fn libc_start_main(emu: &mut Emulator) -> Result<Flow, EmuError> {
    let main = emu.cpu.regs.get64(Reg64::Rdi);
    let argc = emu.cpu.regs.get64(Reg64::Rsi);
    let argv = emu.cpu.regs.get64(Reg64::Rdx);
    // L'environnement suit `argv` : autant de pointeurs, puis un zéro.
    let envp = argv + (argc + 1) * 8;

    // L'adresse de retour de `main` : une borne à nous, qui transforme
    // son résultat en `exit`. C'est exactement ce que fait la vraie
    // fonction, et cela rend le « pas à pas » juste jusqu'au bout.
    let retour = emu.main_return_addr();
    let rsp = emu.cpu.regs.get64(Reg64::Rsp);
    // On écrase l'adresse de retour de `__libc_start_main` : elle ne
    // servira jamais, la vraie fonction ne revenant pas non plus.
    emu.record_memory(rsp, 8);
    emu.cpu.mem.write_bytes(rsp, &retour.to_le_bytes())?;

    emu.cpu.regs.set64(Reg64::Rdi, argc);
    emu.cpu.regs.set64(Reg64::Rsi, argv);
    emu.cpu.regs.set64(Reg64::Rdx, envp);
    emu.cpu.regs.rip = main;
    Ok(Flow::Jumped)
}

/// Le retour de `main` : son résultat devient le code de sortie.
pub(crate) fn main_returned(emu: &mut Emulator) -> Flow {
    let code = emu.cpu.regs.get64(Reg64::Rax) as u8;
    emu.exit_code = Some(code as i64);
    emu.cpu.halted = true;
    Flow::Halted
}

/// Bâtit, une fois, la table que `__ctype_*_loc` désigne, et rend
/// l'adresse du POINTEUR vers elle.
///
/// Deux niveaux, et c'est la source de toutes les confusions : la
/// fonction rend `unsigned short **`, un pointeur sur un pointeur. Le
/// second pointe au MILIEU d'un tableau de 384 entrées, de sorte que
/// l'index -128 soit valide — glibc indexe par un `int` qui peut être
/// négatif, EOF compris.
///
/// Les trois tables sont bâties ensemble et gardées : les rebâtir à
/// chaque appel ferait grandir le tas à chaque caractère examiné.
fn table_ctype(emu: &mut Emulator, laquelle: &str) -> Result<u64, EmuError> {
    /// Les bits de glibc, dans son ordre à elle — et il est en
    /// GROS-BOUTISTE dans le `unsigned short`, ce qui est une
    /// bizarrerie historique de la bibliothèque et non une erreur ici.
    const UPPER: u16 = 1 << 8;
    const LOWER: u16 = 1 << 9;
    const ALPHA: u16 = 1 << 10;
    const DIGIT: u16 = 1 << 11;
    const XDIGIT: u16 = 1 << 12;
    const SPACE: u16 = 1 << 13;
    const PRINT: u16 = 1 << 14;
    const GRAPH: u16 = 1 << 15;
    const BLANK: u16 = 1 << 0;
    const CNTRL: u16 = 1 << 1;
    const PUNCT: u16 = 1 << 2;
    const ALNUM: u16 = 1 << 3;

    if emu.ctype_at == 0 {
        // Trois tableaux de 384 entrées : classes (2 octets), minuscules
        // et majuscules (4 octets chacun), plus trois pointeurs.
        let total = 384 * 2 + 384 * 4 * 2 + 3 * 8;
        // Le tas plein : on rend zéro, comme `malloc`. Un programme qui
        // déréférence un pointeur nul s'arrêtera en le disant, ce qui
        // est plus clair qu'une erreur venue d'ici.
        let Some(base) = emu.libc_alloc(total) else {
            return Ok(0);
        };
        let classes = base + 3 * 8;
        let vers_bas = classes + 384 * 2;
        let vers_haut = vers_bas + 384 * 4;
        let mut octets = Vec::with_capacity(total as usize);
        // Les trois pointeurs, chacun décalé de 128 entrées pour que
        // l'index négatif tombe juste.
        octets.extend_from_slice(&(classes + 128 * 2).to_le_bytes());
        octets.extend_from_slice(&(vers_bas + 128 * 4).to_le_bytes());
        octets.extend_from_slice(&(vers_haut + 128 * 4).to_le_bytes());
        for i in 0..384i32 {
            let c = i - 128;
            let mut bits = 0u16;
            if (0..=255).contains(&c) {
                let o = c as u8;
                if o.is_ascii_uppercase() {
                    bits |= UPPER | ALPHA | ALNUM | GRAPH | PRINT;
                }
                if o.is_ascii_lowercase() {
                    bits |= LOWER | ALPHA | ALNUM | GRAPH | PRINT;
                }
                if o.is_ascii_digit() {
                    bits |= DIGIT | ALNUM | GRAPH | PRINT | XDIGIT;
                }
                if o.is_ascii_hexdigit() {
                    bits |= XDIGIT;
                }
                if o.is_ascii_whitespace() || o == 0x0b {
                    bits |= SPACE;
                }
                if o == b' ' || o == b'\t' {
                    bits |= BLANK;
                }
                if o == b' ' {
                    bits |= PRINT;
                }
                if o.is_ascii_punctuation() {
                    bits |= PUNCT | GRAPH | PRINT;
                }
                if o.is_ascii_control() {
                    bits |= CNTRL;
                }
            }
            // Gros-boutiste, comme glibc le range.
            octets.extend_from_slice(&bits.to_be_bytes());
        }
        for (vers_minuscule, _) in [(true, ()), (false, ())] {
            for i in 0..384i32 {
                let c = i - 128;
                let rendu = if (0..=255).contains(&c) {
                    let o = c as u8;
                    i32::from(if vers_minuscule {
                        o.to_ascii_lowercase()
                    } else {
                        o.to_ascii_uppercase()
                    })
                } else {
                    c
                };
                octets.extend_from_slice(&rendu.to_le_bytes());
            }
        }
        emu.record_memory(base, octets.len());
        emu.cpu.mem.write_bytes(base, &octets)?;
        emu.ctype_at = base;
    }
    let rang = match laquelle {
        "__ctype_tolower_loc" => 1,
        "__ctype_toupper_loc" => 2,
        _ => 0,
    };
    Ok(emu.ctype_at + rang * 8)
}

/// Dit, une fois, que namsdec n'a pas de système de fichiers.
///
/// Une fois et pas davantage : un programme qui essaie d'ouvrir trois
/// cents fichiers noierait sa propre sortie sous le même message.
fn sans_fichiers(emu: &mut Emulator, quoi: &str) {
    if emu.dit_sans_fichiers {
        return;
    }
    emu.dit_sans_fichiers = true;
    emu.output.extend_from_slice(
        format!(
            "*** namsdec n'a pas de système de fichiers : `{quoi}` répond « introuvable ». \
             Le programme suit sa branche d'erreur, ce qui est souvent ce qu'on veut voir. ***\n"
        )
        .as_bytes(),
    );
}

/// Pose le résultat dans RAX, là où la convention d'appel le veut.
fn set(emu: &mut Emulator, value: u64) {
    emu.cpu.regs.set64(Reg64::Rax, value);
}

/// Lit une chaîne terminée par zéro dans la mémoire simulée.
///
/// Les droits s'appliquent : lire une chaîne dans une zone illisible
/// échoue ici comme sur la vraie machine.
fn cstr(emu: &Emulator, addr: u64) -> Result<Vec<u8>, EmuError> {
    /// Une chaîne plus longue que cela n'en est pas une : c'est un
    /// pointeur qui ne pointe nulle part, et le dire vaut mieux que de
    /// parcourir la mémoire entière.
    const MAX: usize = 1 << 20;
    let mut octets = Vec::new();
    let mut at = addr;
    loop {
        let octet = emu.cpu.mem.read_u8(at)?;
        if octet == 0 {
            return Ok(octets);
        }
        octets.push(octet);
        at += 1;
        if octets.len() >= MAX {
            return Ok(octets);
        }
    }
}

/// Écrit des octets, droits vérifiés et écriture journalisée.
fn put(emu: &mut Emulator, addr: u64, octets: &[u8]) -> Result<(), EmuError> {
    emu.record_memory(addr, octets.len());
    emu.cpu.mem.write_bytes(addr, octets)?;
    Ok(())
}

/// La comparaison de `strcmp` : négatif, nul, positif.
fn compare(a: &[u8], b: &[u8]) -> u64 {
    match a.cmp(b) {
        std::cmp::Ordering::Less => (-1i64) as u64,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Lit un entier en tête de chaîne : la valeur, et combien de caractères
/// ont été consommés.
fn parse_int(texte: &[u8], base: u32) -> (i64, usize) {
    let mut at = 0;
    while at < texte.len() && (texte[at] as char).is_ascii_whitespace() {
        at += 1;
    }
    let début = at;
    let négatif = texte.get(at) == Some(&b'-');
    if négatif || texte.get(at) == Some(&b'+') {
        at += 1;
    }
    let chiffres = at;
    let mut valeur: i64 = 0;
    while let Some(c) = texte.get(at) {
        let Some(chiffre) = (*c as char).to_digit(base) else {
            break;
        };
        valeur = valeur
            .saturating_mul(base as i64)
            .saturating_add(chiffre as i64);
        at += 1;
    }
    if at == chiffres {
        // Rien lu : la chaîne ne commençait pas par un nombre.
        return (0, début);
    }
    (if négatif { -valeur } else { valeur }, at)
}

/// Les arguments d'une fonction variadique, lus dans l'ordre où la
/// convention d'appel les a rangés.
///
/// x86-64 ne pousse pas les arguments sur la pile : les six premiers
/// entiers passent par RDI, RSI, RDX, RCX, R8 et R9, les huit premiers
/// flottants par XMM0 à XMM7, et le reste seulement va sur la pile. Un
/// `printf` doit donc compter les deux familles séparément — c'est ce que
/// fait ce petit curseur, et c'est aussi pourquoi `printf("%s", 1.5)`
/// affiche n'importe quoi plutôt que rien.
struct VarArgs {
    entier: usize,
    vectoriel: usize,
    /// La prochaine adresse à lire sur la pile, une fois les registres
    /// épuisés. L'adresse de retour occupe le sommet : les arguments
    /// commencent juste au-dessus.
    pile: u64,
}

impl VarArgs {
    /// `saut` : combien d'arguments entiers *fixes* précèdent le format.
    fn new(emu: &Emulator, saut: usize) -> Self {
        Self {
            entier: saut,
            vectoriel: 0,
            pile: emu.cpu.regs.get64(Reg64::Rsp) + 8,
        }
    }

    fn integer(&mut self, emu: &Emulator) -> u64 {
        const REGISTRES: [Reg64; 6] = [
            Reg64::Rdi,
            Reg64::Rsi,
            Reg64::Rdx,
            Reg64::Rcx,
            Reg64::R8,
            Reg64::R9,
        ];
        let valeur = match REGISTRES.get(self.entier) {
            Some(registre) => emu.cpu.regs.get64(*registre),
            None => {
                let valeur = emu.cpu.mem.peek_u64(self.pile).unwrap_or(0);
                self.pile += 8;
                valeur
            }
        };
        self.entier += 1;
        valeur
    }

    fn double(&mut self, emu: &Emulator) -> f64 {
        const REGISTRES: [namsdec_core::RegXmm; 8] = [
            namsdec_core::RegXmm::Xmm0,
            namsdec_core::RegXmm::Xmm1,
            namsdec_core::RegXmm::Xmm2,
            namsdec_core::RegXmm::Xmm3,
            namsdec_core::RegXmm::Xmm4,
            namsdec_core::RegXmm::Xmm5,
            namsdec_core::RegXmm::Xmm6,
            namsdec_core::RegXmm::Xmm7,
        ];
        let bits = match REGISTRES.get(self.vectoriel) {
            Some(registre) => emu.cpu.regs.get_xmm(*registre) as u64,
            None => {
                let valeur = emu.cpu.mem.peek_u64(self.pile).unwrap_or(0);
                self.pile += 8;
                valeur
            }
        };
        self.vectoriel += 1;
        f64::from_bits(bits)
    }
}

/// Met un format en forme, comme `printf`.
///
/// Ce qui est couvert : `%d %i %u %x %X %o %c %s %p %f %e %g %%`, les
/// tailles `l`, `ll`, `h`, `hh`, `z` et `t`, la largeur, la précision, et
/// les drapeaux `-`, `0`, `+` et l'espace. Ce qui ne l'est pas — `%n`,
/// les positions explicites `%1$s` — s'écrit tel quel, ce qui se voit
/// tout de suite dans la sortie.
fn format_printf(emu: &Emulator, format: &[u8], args: &mut VarArgs) -> Result<Vec<u8>, EmuError> {
    let mut sortie = Vec::new();
    let mut at = 0;
    while at < format.len() {
        if format[at] != b'%' {
            sortie.push(format[at]);
            at += 1;
            continue;
        }
        let début = at;
        at += 1;
        if format.get(at) == Some(&b'%') {
            sortie.push(b'%');
            at += 1;
            continue;
        }

        // Les drapeaux.
        let (mut gauche, mut zéros, mut signe, mut espace) = (false, false, false, false);
        while let Some(c) = format.get(at) {
            match c {
                b'-' => gauche = true,
                b'0' => zéros = true,
                b'+' => signe = true,
                b' ' => espace = true,
                b'#' => {}
                _ => break,
            }
            at += 1;
        }
        // La largeur, éventuellement donnée par un argument.
        let mut largeur = 0usize;
        if format.get(at) == Some(&b'*') {
            largeur = args.integer(emu) as usize;
            at += 1;
        } else {
            while let Some(c) = format.get(at).filter(|c| c.is_ascii_digit()) {
                largeur = largeur * 10 + (c - b'0') as usize;
                at += 1;
            }
        }
        // La précision.
        let mut précision = None;
        if format.get(at) == Some(&b'.') {
            at += 1;
            let mut valeur = 0usize;
            if format.get(at) == Some(&b'*') {
                valeur = args.integer(emu) as usize;
                at += 1;
            } else {
                while let Some(c) = format.get(at).filter(|c| c.is_ascii_digit()) {
                    valeur = valeur * 10 + (c - b'0') as usize;
                    at += 1;
                }
            }
            précision = Some(valeur);
        }
        // La taille : elle ne change que la façon de tronquer.
        let mut taille = 8u32;
        loop {
            match format.get(at) {
                Some(b'l') | Some(b'z') | Some(b't') | Some(b'j') => {
                    taille = 8;
                    at += 1;
                }
                Some(b'h') => {
                    taille = if taille == 2 { 1 } else { 2 };
                    at += 1;
                }
                _ => break,
            }
        }

        let Some(conversion) = format.get(at).copied() else {
            // Un format tronqué : on le recopie tel quel plutôt que
            // d'inventer.
            sortie.extend_from_slice(&format[début..]);
            break;
        };
        at += 1;

        let mut morceau: Vec<u8> = match conversion {
            b'd' | b'i' => {
                let brut = args.integer(emu);
                let valeur = match taille {
                    1 => brut as i8 as i64,
                    2 => brut as i16 as i64,
                    4 => brut as i32 as i64,
                    _ => brut as i64,
                };
                let mut texte = valeur.abs().to_string();
                if valeur < 0 {
                    texte.insert(0, '-');
                } else if signe {
                    texte.insert(0, '+');
                } else if espace {
                    texte.insert(0, ' ');
                }
                texte.into_bytes()
            }
            b'u' => {
                let brut = args.integer(emu);
                let valeur = match taille {
                    1 => brut as u8 as u64,
                    2 => brut as u16 as u64,
                    4 => brut as u32 as u64,
                    _ => brut,
                };
                valeur.to_string().into_bytes()
            }
            b'x' => format!("{:x}", args.integer(emu)).into_bytes(),
            b'X' => format!("{:X}", args.integer(emu)).into_bytes(),
            b'o' => format!("{:o}", args.integer(emu)).into_bytes(),
            b'p' => {
                let valeur = args.integer(emu);
                if valeur == 0 {
                    b"(nil)".to_vec()
                } else {
                    format!("{valeur:#x}").into_bytes()
                }
            }
            b'c' => vec![args.integer(emu) as u8],
            b's' => {
                let adresse = args.integer(emu);
                if adresse == 0 {
                    b"(null)".to_vec()
                } else {
                    let mut texte = cstr(emu, adresse)?;
                    if let Some(max) = précision {
                        texte.truncate(max);
                    }
                    texte
                }
            }
            b'f' | b'F' => format!("{:.*}", précision.unwrap_or(6), args.double(emu)).into_bytes(),
            b'e' | b'E' => format!("{:e}", args.double(emu)).into_bytes(),
            b'g' | b'G' => format!("{}", args.double(emu)).into_bytes(),
            autre => {
                // Une conversion qu'on ne connaît pas : elle repart telle
                // quelle, visible dans la sortie plutôt que devinée.
                let mut brut = format[début..at].to_vec();
                brut.pop();
                brut.push(autre);
                brut
            }
        };

        // La précision d'un entier est un nombre minimal de chiffres.
        if let Some(max) = précision
            && matches!(conversion, b'd' | b'i' | b'u' | b'x' | b'X' | b'o')
            && morceau.len() < max
        {
            let manque = max - morceau.len();
            let mut rempli = vec![b'0'; manque];
            rempli.extend_from_slice(&morceau);
            morceau = rempli;
        }
        // Puis la largeur du champ.
        if morceau.len() < largeur {
            let manque = largeur - morceau.len();
            if gauche {
                morceau.extend(std::iter::repeat_n(b' ', manque));
            } else {
                let bourre = if zéros && !matches!(conversion, b's' | b'c') {
                    b'0'
                } else {
                    b' '
                };
                let mut rempli = vec![bourre; manque];
                // Un signe reste devant les zéros de remplissage.
                if bourre == b'0' && matches!(morceau.first(), Some(b'-') | Some(b'+')) {
                    rempli.insert(0, morceau.remove(0));
                }
                rempli.extend_from_slice(&morceau);
                morceau = rempli;
            }
        }
        sortie.extend_from_slice(&morceau);
    }
    Ok(sortie)
}

/// Lit ce que le format demande sur l'entrée standard, comme `scanf`.
///
/// Couvre `%s`, `%d`, `%u`, `%x`, `%c` et les espaces. C'est ce qu'un
/// programme qui demande un mot de passe emploie, et il n'en faut pas
/// plus. Rend le nombre de conversions réussies, comme le vrai.
fn scan(emu: &mut Emulator, format: &[u8], args: &mut VarArgs) -> Result<u64, EmuError> {
    let Some(ligne) = take_line_raw(emu) else {
        // Fin de fichier : `scanf` rend EOF.
        return Ok(u64::MAX);
    };
    scan_dans(emu, &ligne, format, args)
}

/// Le même travail, mais sur une chaîne qu'on a déjà — c'est `sscanf`.
///
/// Séparé de `scan` plutôt que recopié : la grammaire des `%` est la
/// seule chose délicate ici, et deux copies divergeraient au premier
/// format ajouté. Ce qui change entre les deux tient en une ligne — d'où
/// viennent les octets.
fn scan_dans(
    emu: &mut Emulator,
    source: &[u8],
    format: &[u8],
    args: &mut VarArgs,
) -> Result<u64, EmuError> {
    let mut reste: &[u8] = source;
    let mut réussies = 0u64;
    let mut at = 0;
    while at < format.len() {
        let c = format[at];
        if c.is_ascii_whitespace() {
            while reste.first().is_some_and(|o| o.is_ascii_whitespace()) {
                reste = &reste[1..];
            }
            at += 1;
            continue;
        }
        if c != b'%' {
            if reste.first() == Some(&c) {
                reste = &reste[1..];
                at += 1;
                continue;
            }
            break;
        }
        at += 1;
        // La largeur maximale, quand elle est donnée.
        let mut largeur = usize::MAX;
        let mut lu = 0usize;
        while let Some(chiffre) = format.get(at).filter(|c| c.is_ascii_digit()) {
            lu = lu * 10 + (chiffre - b'0') as usize;
            at += 1;
        }
        if lu > 0 {
            largeur = lu;
        }
        while matches!(format.get(at), Some(b'l') | Some(b'h') | Some(b'z')) {
            at += 1;
        }
        let Some(conversion) = format.get(at).copied() else {
            break;
        };
        at += 1;

        if conversion != b'c' {
            while reste.first().is_some_and(|o| o.is_ascii_whitespace()) {
                reste = &reste[1..];
            }
        }
        let destination = args.integer(emu);
        match conversion {
            b's' => {
                let fin = reste
                    .iter()
                    .position(|o| o.is_ascii_whitespace())
                    .unwrap_or(reste.len())
                    .min(largeur);
                if fin == 0 {
                    break;
                }
                let mut mot = reste[..fin].to_vec();
                reste = &reste[fin..];
                mot.push(0);
                put(emu, destination, &mot)?;
                réussies += 1;
            }
            b'c' => {
                let Some(octet) = reste.first().copied() else {
                    break;
                };
                reste = &reste[1..];
                put(emu, destination, &[octet])?;
                réussies += 1;
            }
            b'd' | b'i' | b'u' | b'x' => {
                let base = if conversion == b'x' { 16 } else { 10 };
                let (valeur, consommés) = parse_int(reste, base);
                if consommés == 0 {
                    break;
                }
                reste = &reste[consommés..];
                let octets = (valeur as i32).to_le_bytes();
                emu.record_memory(destination, 4);
                emu.cpu.mem.write_bytes(destination, &octets)?;
                réussies += 1;
            }
            _ => break,
        }
    }
    Ok(réussies)
}

/// Retire une ligne de l'entrée, terminée par un zéro et bornée à
/// `taille` octets — ce que `fgets` promet.
fn take_line(emu: &mut Emulator, taille: u64) -> Option<Vec<u8>> {
    let mut ligne = take_line_raw(emu)?;
    // `fgets` garde le saut de ligne : c'est ce qui permet de savoir si
    // la ligne était complète, et c'est l'octet qu'un programme oublie de
    // retirer une fois sur deux.
    ligne.push(b'\n');
    if taille != u64::MAX && ligne.len() as u64 >= taille {
        ligne.truncate(taille.saturating_sub(1) as usize);
    }
    ligne.push(0);
    Some(ligne)
}

/// La ligne suivante de l'entrée, sans son saut de ligne. `None` à la fin
/// de l'entrée.
fn take_line_raw(emu: &mut Emulator) -> Option<Vec<u8>> {
    if emu.input.is_empty() {
        return None;
    }
    let fin = emu
        .input
        .iter()
        .position(|o| *o == b'\n')
        .unwrap_or(emu.input.len());
    let ligne: Vec<u8> = emu.input.iter().take(fin).copied().collect();
    // Le saut de ligne part avec elle.
    let combien = (fin + 1).min(emu.input.len());
    let consommés: Vec<u8> = emu.input.iter().take(combien).copied().collect();
    emu.input.drain(..combien);
    if let Some(undo) = emu.history.back_mut() {
        undo.consumed_input.extend(consommés);
    }
    Some(ligne)
}

/// Un octet de l'entrée, ou `None` à la fin.
fn take_byte(emu: &mut Emulator) -> Option<u8> {
    let octet = emu.input.pop_front()?;
    if let Some(undo) = emu.history.back_mut() {
        undo.consumed_input.push(octet);
    }
    Some(octet)
}
