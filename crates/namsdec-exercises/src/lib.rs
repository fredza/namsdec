//! Les exercices : des programmes à trous, avec ce qu'on cherche à y
//! comprendre.
//!
//! Module à part, et chargé seulement si on le demande. Quelqu'un qui
//! écrit son propre code n'a pas besoin qu'on lui propose des devoirs :
//! l'application ne montre rien de tout cela tant que la case n'est pas
//! cochée dans les préférences, et cette crate n'a alors aucun effet sur
//! le reste.
//!
//! Deux sources : la série livrée avec l'application, et les séries
//! installées depuis le disque — un fichier JSON, comme un thème (voir
//! [`pack`]). Les deux donnent le même type, pour que l'interface n'ait
//! pas à savoir d'où vient ce qu'elle affiche.
//!
//! Pas de correction automatique : un exercice pose un but en clair — « à
//! l'arrêt, RAX doit valoir 15 » — et c'est le bandeau des registres qui
//! répond. Vérifier à la place de l'utilisateur lui retirerait le geste
//! qu'il est venu apprendre.

pub mod pack;

/// Un texte dans les deux langues de l'interface.
///
/// Comme pour les fiches d'instructions : l'énoncé d'un exercice *est*
/// son texte, il ne peut pas être traduit à l'affichage par quelqu'un
/// d'autre. Une série écrite dans une seule langue donne le même texte
/// des deux côtés — c'est à celui qui l'écrit de décider, pas à nous de
/// refuser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub fr: String,
    pub en: String,
}

impl Text {
    /// Le même texte quelle que soit la langue.
    pub fn same(text: &str) -> Self {
        Self {
            fr: text.to_owned(),
            en: text.to_owned(),
        }
    }

    pub fn get(&self, lang: &str) -> &str {
        if lang.starts_with("fr") {
            &self.fr
        } else {
            &self.en
        }
    }
}

/// Le palier d'un exercice.
///
/// Quatre, et nommés par ce qu'on y fait plutôt que par un chiffre : un
/// « niveau 3 » ne dit pas ce qui attend, « Structures » si. Le parcours
/// les traverse dans l'ordre, et aucun exercice ne revient à un palier
/// qu'on a quitté — c'est ce que vérifie le test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Ce que le processeur est : registres, indicateurs, sauts.
    Discovery,
    /// Ce qu'il manipule : mémoire, données, adresses.
    Practice,
    /// Ce qu'on construit avec : boucles écrites autrement, fonctions,
    /// pile, macros.
    Structures,
    /// Ce qui l'entoure : le noyau, le vectoriel, et ce qui casse.
    Mastery,
}

impl Level {
    /// Son nom, dans la langue demandée.
    pub fn label(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match (self, fr) {
            (Level::Discovery, true) => "Découverte",
            (Level::Discovery, false) => "Discovery",
            (Level::Practice, true) => "Pratique",
            (Level::Practice, false) => "Practice",
            (Level::Structures, true) => "Structures",
            (Level::Structures, false) => "Structures",
            (Level::Mastery, true) => "Maîtrise",
            (Level::Mastery, false) => "Mastery",
        }
    }

    /// Les quatre, dans l'ordre.
    pub const ALL: [Level; 4] = [
        Level::Discovery,
        Level::Practice,
        Level::Structures,
        Level::Mastery,
    ];

    /// L'identifiant écrit dans un fichier de série.
    pub fn id(self) -> &'static str {
        match self {
            Level::Discovery => "discovery",
            Level::Practice => "practice",
            Level::Structures => "structures",
            Level::Mastery => "mastery",
        }
    }

    /// Lu depuis un fichier de série. Ce qu'on ne reconnaît pas retombe
    /// sur le premier palier : une série mal étiquetée reste utilisable.
    pub fn by_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|level| level.id() == id)
            .unwrap_or(Level::Discovery)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    /// Identifiant stable, indépendant de la langue et du titre.
    pub id: String,
    pub title: Text,
    /// Ce qu'on cherche à comprendre, et le but à atteindre.
    pub description: Text,
    /// Le programme de départ, à charger dans l'éditeur.
    pub starter_code: String,
    /// Le but, en conditions vérifiables — `rax == 15`, `rdx == 3`.
    ///
    /// Toutes doivent tenir à l'arrêt du programme. La grammaire est
    /// celle des points d'arrêt conditionnels : une valeur, une
    /// comparaison, une valeur. Vide quand l'exercice est une
    /// démonstration à regarder plutôt qu'un but à atteindre — dire
    /// « atteint » d'un exercice qui ne demande rien n'apprendrait
    /// rien.
    pub goal: Vec<String>,
    /// Le budget : atteindre le but en ce nombre d'instructions
    /// **exécutées**, ou moins.
    ///
    /// Ce n'est pas une note, c'est une mesure. Un programme juste peut
    /// être deux fois plus court, et c'est en cherchant comment qu'on
    /// apprend `lea`, les décalages, ou qu'une boucle de cinq tours se
    /// remplace par une formule. Le budget est celui de la solution
    /// livrée : l'égaler est normal, descendre en dessous est le vrai
    /// exercice.
    ///
    /// `None` quand il n'y a rien à mesurer — un exercice qui attend une
    /// entrée, ou qui montre un refus.
    pub par: Option<u64>,
    /// Le palier auquel il appartient.
    pub level: Level,
}

/// Une série d'exercices : celle qui est livrée, ou une installée depuis
/// le disque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub exercises: Vec<Exercise>,
}

/// L'identifiant de la série livrée avec l'application.
pub const BUILTIN_ID: &str = "namsdec";

/// Les exercices livrés avec l'application, **dans l'ordre où les faire**.
///
/// Un parcours, et non une collection : chacun suppose le précédent et
/// n'introduit qu'une chose. Registres, boucle, comparaison, mémoire,
/// pile, appels, système, protection — c'est l'ordre dans lequel un
/// programme en assembleur se construit, et celui dans lequel on bute
/// sur les difficultés.
pub fn builtin_pack() -> Pack {
    Pack {
        id: BUILTIN_ID.to_owned(),
        name: "namsdec".to_owned(),
        exercises: vec![
            Exercise {
                id: "registres".to_owned(),
                title: Text {
                    fr: "Registres et largeurs".to_owned(),
                    en: "Registers and widths".to_owned(),
                },
                description: Text {
                    fr: "Écrire un registre 32 bits met à zéro la moitié haute de RAX ; écrire 16 ou 8 bits la laisse. Avancez pas à pas et regardez le bandeau : c'est la source de bogue numéro un quand on débute.".to_owned(),
                    en: "Writing a 32-bit register zeroes the upper half of RAX; writing 16 or 8 bits leaves it. Step through it and watch the banner: this is the number one source of bugs when starting out.".to_owned(),
                },
                starter_code: include_str!("../exercises/registres.asm").to_owned(),
                goal: Vec::new(),
                par: None,
                level: Level::Discovery,
            },
            Exercise {
                id: "echange".to_owned(),
                title: Text {
                    fr: "Échanger deux registres".to_owned(),
                    en: "Swapping two registers".to_owned(),
                },
                description: Text {
                    fr: "`xchg` le fait en une instruction. L'autre chemin — trois `xor` — échange sans rien emprunter, parce que a xor b xor b redonne a : une curiosité à avoir vue, pas un conseil. Essayez les deux et regardez le compteur d'instructions : c'est lui qui tranche.".to_owned(),
                    en: "`xchg` does it in one instruction. The other way — three `xor`s — swaps without borrowing anything, because a xor b xor b gives a back: a curiosity worth seeing once, not advice. Try both and watch the instruction count: that is what settles it.".to_owned(),
                },
                starter_code: include_str!("../exercises/echange.asm").to_owned(),
                goal: vec!["rax == 2".to_owned(), "rbx == 1".to_owned()],
                par: Some(4),
                level: Level::Discovery,
            },
            Exercise {
                id: "zero".to_owned(),
                title: Text {
                    fr: "Mettre à zéro, trois façons".to_owned(),
                    en: "Zeroing, three ways".to_owned(),
                },
                description: Text {
                    fr: "`mov rax, 0` écrit le zéro en toutes lettres : sept octets. `xor eax, eax` en fait deux, et met bien RAX entier à zéro — écrire dans une moitié 32 bits efface la haute. C'est pour cela qu'un binaire compilé est rempli de `xor` là où le source disait « = 0 ». Ouvrez l'onglet Octets et comparez.".to_owned(),
                    en: "`mov rax, 0` spells the zero out: seven bytes. `xor eax, eax` takes two, and does zero all of RAX — writing a 32-bit half clears the upper one. That is why a compiled binary is full of `xor` where the source said \"= 0\". Open the Bytes tab and compare.".to_owned(),
                },
                starter_code: include_str!("../exercises/zero.asm").to_owned(),
                goal: vec!["rax == 0".to_owned(), "rbx == 0".to_owned(), "rcx == 0".to_owned()],
                par: Some(4),
                level: Level::Discovery,
            },
            Exercise {
                id: "boucle".to_owned(),
                title: Text {
                    fr: "Une boucle qui compte".to_owned(),
                    en: "A loop that counts".to_owned(),
                },
                description: Text {
                    fr: "Additionner 5+4+3+2+1. `dec` laisse une trace dans les indicateurs, `jnz` la lit : c'est toute la mécanique d'une boucle. À l'arrêt, RAX doit valoir 15.".to_owned(),
                    en: "Add 5+4+3+2+1. `dec` leaves a trace in the flags and `jnz` reads it: that is the whole machinery of a loop. At the halt, RAX must hold 15.".to_owned(),
                },
                starter_code: include_str!("../exercises/boucle.asm").to_owned(),
                goal: vec!["rax == 0xf".to_owned()],
                par: Some(18),
                level: Level::Discovery,
            },
            Exercise {
                id: "compteur".to_owned(),
                title: Text {
                    fr: "La boucle que le processeur connaît".to_owned(),
                    en: "The loop the processor knows".to_owned(),
                },
                description: Text {
                    fr: "`loop` décrémente RCX et saute si le résultat n'est pas nul : deux instructions en une, et RCX imposé. Elle date d'une époque où chaque octet comptait — et elle a disparu du code moderne, parce que `dec` suivi de `jnz` va plus vite. Le compteur d'instructions ne le dit pas : il compte les instructions, pas le temps, et c'est justement sa limite.".to_owned(),
                    en: "`loop` decrements RCX and jumps if the result is not zero: two instructions in one, with RCX imposed. It dates from a time when every byte counted — and it has vanished from modern code, because `dec` then `jnz` is faster. The instruction count will not tell you: it counts instructions, not time, and that is exactly its limit.".to_owned(),
                },
                starter_code: include_str!("../exercises/compteur.asm").to_owned(),
                goal: vec!["rax == 0xf".to_owned()],
                par: Some(13),
                level: Level::Discovery,
            },
            Exercise {
                id: "bits".to_owned(),
                title: Text {
                    fr: "Les bits, un par un".to_owned(),
                    en: "Bits, one at a time".to_owned(),
                },
                description: Text {
                    fr: "Décaler, masquer, poser, tester. `shl` multiplie par deux autant de fois qu'on le lui demande, `and` garde des bits, `or` en pose, `test` demande « ce bit est-il mis ? » sans rien abîmer. Survolez RAX : sa lecture binaire montre les bits bouger.".to_owned(),
                    en: "Shift, mask, set, test. `shl` doubles as many times as asked, `and` keeps bits, `or` sets them, `test` asks \"is this bit set?\" without damaging anything. Hover RAX: its binary reading shows the bits move.".to_owned(),
                },
                starter_code: include_str!("../exercises/bits.asm").to_owned(),
                goal: vec!["rax == 0x28".to_owned(), "rbx == 1".to_owned()],
                par: Some(8),
                level: Level::Discovery,
            },
            Exercise {
                id: "comparer".to_owned(),
                title: Text {
                    fr: "Comparer, et choisir".to_owned(),
                    en: "Compare, and choose".to_owned(),
                },
                description: Text {
                    fr: "Garder le plus grand de deux nombres. Puis changez -3 en 3 : `jg` lit SF et OF pour des nombres signés, `ja` lirait CF pour des non signés — et avec un négatif, les deux ne répondent pas la même chose.".to_owned(),
                    en: "Keep the larger of two numbers. Then change -3 into 3: `jg` reads SF and OF for signed numbers, `ja` would read CF for unsigned ones — and with a negative value the two do not answer the same.".to_owned(),
                },
                starter_code: include_str!("../exercises/comparer.asm").to_owned(),
                goal: Vec::new(),
                par: None,
                level: Level::Discovery,
            },
            Exercise {
                id: "signe".to_owned(),
                title: Text {
                    fr: "Signé ou non signé".to_owned(),
                    en: "Signed or unsigned".to_owned(),
                },
                description: Text {
                    fr: "Les mêmes bits, deux lectures : 0xFFFFFFFFFFFFFFFF est -1 et le plus grand entier non signé. `ja` lit CF, `jg` lit SF et OF — sur la même comparaison, l'un saute et l'autre pas. À l'arrêt, RCX doit valoir 1 et RDX 0.".to_owned(),
                    en: "The same bits, two readings: 0xFFFFFFFFFFFFFFFF is -1 and the largest unsigned integer. `ja` reads CF, `jg` reads SF and OF — on the same comparison, one jumps and the other does not. At the halt, RCX must hold 1 and RDX 0.".to_owned(),
                },
                starter_code: include_str!("../exercises/signe.asm").to_owned(),
                goal: vec!["rcx == 1".to_owned(), "rdx == 0".to_owned()],
                par: Some(11),
                level: Level::Discovery,
            },
            Exercise {
                id: "decalages".to_owned(),
                title: Text {
                    fr: "Multiplier sans multiplier".to_owned(),
                    en: "Multiplying without multiplying".to_owned(),
                },
                description: Text {
                    fr: "Décaler d'un cran à gauche multiplie par deux, à droite divise par deux — et un compilateur remplace `x * 8` par `x << 3` sans qu'on le lui demande. Mais il y a deux décalages à droite : `shr` fait entrer des zéros, `sar` recopie le bit de signe. Sur -4, l'un rend -2, l'autre un nombre positif énorme.".to_owned(),
                    en: "Shifting left by one multiplies by two, right divides by two — and a compiler turns `x * 8` into `x << 3` unasked. But there are two right shifts: `shr` brings in zeroes, `sar` copies the sign bit. On -4, one gives -2, the other a huge positive number.".to_owned(),
                },
                starter_code: include_str!("../exercises/decalages.asm").to_owned(),
                goal: vec!["rax == 0x28".to_owned(), "rbx == 5".to_owned()],
                par: Some(9),
                level: Level::Practice,
            },
            Exercise {
                id: "multiplier".to_owned(),
                title: Text {
                    fr: "Multiplier et diviser".to_owned(),
                    en: "Multiply and divide".to_owned(),
                },
                description: Text {
                    fr: "Les deux opérations qui travaillent sur une paire de registres : `mul` écrit RDX:RAX, `div` la lit. Retirez le `xor rdx, rdx` pour voir ce qui arrive quand on oublie la moitié haute du dividende — c'est l'erreur classique.".to_owned(),
                    en: "The two operations that work on a register pair: `mul` writes RDX:RAX, `div` reads it. Remove the `xor rdx, rdx` to see what happens when the high half of the dividend is forgotten — the classic mistake.".to_owned(),
                },
                starter_code: include_str!("../exercises/multiplier.asm").to_owned(),
                goal: vec!["rax == 7".to_owned(), "rdx == 1".to_owned()],
                par: Some(8),
                level: Level::Practice,
            },
            Exercise {
                id: "division".to_owned(),
                title: Text {
                    fr: "Diviser, et ce qu'on oublie".to_owned(),
                    en: "Divide, and what gets forgotten".to_owned(),
                },
                description: Text {
                    fr: "`div` ne divise pas RAX mais RDX:RAX — ce qui traîne dans RDX fait partie du dividende. Le programme part avec un reste oublié dans RDX : exécutez, lisez le refus, puis mettez RDX à zéro. À l'arrêt, RAX doit valoir 7 et RDX 3.".to_owned(),
                    en: "`div` does not divide RAX but RDX:RAX — whatever sits in RDX is part of the dividend. The program starts with a leftover in RDX: run it, read the refusal, then zero RDX. At the halt, RAX must hold 7 and RDX 3.".to_owned(),
                },
                starter_code: include_str!("../exercises/division.asm").to_owned(),
                goal: vec!["rax == 7".to_owned(), "rdx == 3".to_owned()],
                par: Some(5),
                level: Level::Practice,
            },
            Exercise {
                id: "memoire".to_owned(),
                title: Text {
                    fr: "Lire et écrire en mémoire".to_owned(),
                    en: "Read and write memory".to_owned(),
                },
                description: Text {
                    fr: "Quatre octets d'un coup, un seul octet élargi par des zéros, et l'adressage indexé base + décalage. Ouvrez l'onglet Mémoire et tapez « rdi » dans le champ d'adresse pour suivre ce qui change.".to_owned(),
                    en: "Four bytes at once, a single byte widened with zeroes, and indexed addressing base + offset. Open the Memory tab and type \"rdi\" in the address field to follow what changes.".to_owned(),
                },
                starter_code: include_str!("../exercises/memoire.asm").to_owned(),
                goal: vec!["r8 == 0x44332211".to_owned(), "r9 == 0x11".to_owned()],
                par: Some(7),
                level: Level::Practice,
            },
            Exercise {
                id: "tailles".to_owned(),
                title: Text {
                    fr: "La même adresse, quatre lectures".to_owned(),
                    en: "One address, four readings".to_owned(),
                },
                description: Text {
                    fr: "Une adresse ne dit pas combien d'octets lire : c'est l'instruction qui décide — `byte`, `word`, `dword`, `qword`. Et ce que chacune rapporte redit le petit-boutisme sous un autre angle : deux octets lus à la même adresse donnent 0x2211, pas 0x1122. Notez au passage que `mov ecx` efface le haut de RCX, et `movzx` élargit avec des zéros.".to_owned(),
                    en: "An address does not say how many bytes to read: the instruction does — `byte`, `word`, `dword`, `qword`. And what each brings back restates little-endianness from another angle: two bytes at the same address give 0x2211, not 0x1122. Note in passing that `mov ecx` clears the top of RCX, and `movzx` widens with zeroes.".to_owned(),
                },
                starter_code: include_str!("../exercises/tailles.asm").to_owned(),
                goal: vec!["rax == 0x11".to_owned(), "rbx == 0x2211".to_owned(), "rcx == 0x44332211".to_owned(), "rdx == 0x8877665544332211".to_owned()],
                par: Some(5),
                level: Level::Practice,
            },
            Exercise {
                id: "boutisme".to_owned(),
                title: Text {
                    fr: "L'ordre des octets".to_owned(),
                    en: "The order of the bytes".to_owned(),
                },
                description: Text {
                    fr: "Un entier de 64 bits se range en mémoire du poids faible au poids fort. L'octet à l'adresse la plus basse n'est donc pas celui qu'on écrit en premier. Ouvrez la fenêtre Mémoire sur `valeur` et comparez : RCX doit valoir 0x88 et RDX 0x11.".to_owned(),
                    en: "A 64-bit integer is stored least significant byte first. The byte at the lowest address is therefore not the one written first. Open the Memory window on `valeur` and compare: RCX must hold 0x88 and RDX 0x11.".to_owned(),
                },
                starter_code: include_str!("../exercises/boutisme.asm").to_owned(),
                goal: vec!["rcx == 0x88".to_owned(), "rdx == 0x11".to_owned()],
                par: Some(4),
                level: Level::Practice,
            },
            Exercise {
                id: "donnees".to_owned(),
                title: Text {
                    fr: "Déclarer des données".to_owned(),
                    en: "Declaring data".to_owned(),
                },
                description: Text {
                    fr: "`db`, `dw`, `dd`, `dq` posent des octets ; `times` répète, `equ` nomme une valeur, `$` est l'adresse courante — d'où `$ - message` pour une longueur calculée à l'assemblage. Ouvrez l'onglet Octets : il montre ce que chaque directive a vraiment écrit. À l'arrêt, RAX doit valoir 0x44332211, RBX 5 et RCX 0x0201.".to_owned(),
                    en: "`db`, `dw`, `dd`, `dq` lay down bytes; `times` repeats, `equ` names a value, `$` is the current address — hence `$ - message` for a length worked out at assembly time. Open the Bytes tab: it shows what each directive really wrote. At the halt, RAX must hold 0x44332211, RBX 5 and RCX 0x0201.".to_owned(),
                },
                starter_code: include_str!("../exercises/donnees.asm").to_owned(),
                goal: vec!["rax == 0x44332211".to_owned(), "rbx == 5".to_owned(), "rcx == 0x201".to_owned()],
                par: Some(4),
                level: Level::Practice,
            },
            Exercise {
                id: "tableau".to_owned(),
                title: Text {
                    fr: "Un tableau, et son indice".to_owned(),
                    en: "An array, and its index".to_owned(),
                },
                description: Text {
                    fr: "`[base + indice*taille]` est l'adressage qui fait exister les tableaux. Arrêté sur le `add`, le bandeau montre l'adresse que le calcul désigne et la valeur qui s'y trouve, avant même de la lire.".to_owned(),
                    en: "`[base + index*size]` is the addressing that makes arrays exist. Stopped on the `add`, the banner shows the address the computation points at and the value sitting there, before it is even read.".to_owned(),
                },
                starter_code: include_str!("../exercises/tableau.asm").to_owned(),
                goal: vec!["rax == 0x64".to_owned()],
                par: Some(19),
                level: Level::Practice,
            },
            Exercise {
                id: "recherche".to_owned(),
                title: Text {
                    fr: "Chercher dans un tableau".to_owned(),
                    en: "Searching an array".to_owned(),
                },
                description: Text {
                    fr: "Trois choses : un indice qui avance, une comparaison, et **deux** sorties — trouvé, ou arrivé au bout. Rater la seconde donne une boucle qui lit au-delà du tableau, et c'est ainsi qu'on lit la mémoire du voisin. Cherchez une valeur absente pour voir si votre boucle s'arrête proprement : c'est le cas qu'on oublie.".to_owned(),
                    en: "Three things: an index that advances, a comparison, and **two** ways out — found, or run off the end. Missing the second gives a loop that reads past the array, which is how one ends up reading the neighbour's memory. Search for a value that is not there to see whether your loop stops cleanly: that is the case one forgets.".to_owned(),
                },
                starter_code: include_str!("../exercises/recherche.asm").to_owned(),
                goal: vec!["rax == 3".to_owned(), "rbx == 1".to_owned()],
                par: Some(29),
                level: Level::Practice,
            },
            Exercise {
                id: "adresses".to_owned(),
                title: Text {
                    fr: "Calculer une adresse".to_owned(),
                    en: "Working out an address".to_owned(),
                },
                description: Text {
                    fr: "`mov` lit la mémoire, `lea` calcule seulement l'adresse. Le processeur fait base + indice × taille + décalage sans rien toucher — ce qui fait de `lea rax, [rdi + rdi*4]` la multiplication par cinq la moins chère du jeu d'instructions. À l'arrêt, RAX doit valoir 45 et RCX 30.".to_owned(),
                    en: "`mov` reads memory, `lea` only computes the address. The processor does base + index × scale + displacement without touching anything — which makes `lea rax, [rdi + rdi*4]` the cheapest multiply-by-five in the instruction set. At the halt, RAX must hold 45 and RCX 30.".to_owned(),
                },
                starter_code: include_str!("../exercises/adresses.asm").to_owned(),
                goal: vec!["rax == 0x2d".to_owned(), "rcx == 0x1e".to_owned()],
                par: Some(6),
                level: Level::Practice,
            },
            Exercise {
                id: "chaine".to_owned(),
                title: Text {
                    fr: "Parcourir une chaîne".to_owned(),
                    en: "Walk a string".to_owned(),
                },
                description: Text {
                    fr: "Une chaîne n'est qu'une suite d'octets ; ce qui en marque la fin est une convention — ici un zéro final. `movzx` lit un octet en l'élargissant par des zéros, pour que la comparaison porte bien sur ce qu'on a lu.".to_owned(),
                    en: "A string is just a run of bytes; what marks its end is a convention — here a trailing zero. `movzx` reads a byte widening it with zeroes, so the comparison is about what was actually read.".to_owned(),
                },
                starter_code: include_str!("../exercises/chaine.asm").to_owned(),
                goal: vec!["rcx == 5".to_owned()],
                par: Some(36),
                level: Level::Structures,
            },
            Exercise {
                id: "chaines".to_owned(),
                title: Text {
                    fr: "Copier sans boucle".to_owned(),
                    en: "Copying without a loop".to_owned(),
                },
                description: Text {
                    fr: "`rep movsb` copie RCX octets de RSI vers RDI, en une instruction. C'est d'elle que RSI et RDI tirent leur nom — *source index*, *destination index* —, et `cld` dit dans quel sens on avance. Avancez pas à pas : les trois registres bougent ensemble. À l'arrêt, RAX doit valoir 's' (0x73) et RBX 't' (0x74).".to_owned(),
                    en: "`rep movsb` copies RCX bytes from RSI to RDI in one instruction. That is where RSI and RDI get their names — source index, destination index — and `cld` says which way to walk. Step through it: the three registers move together. At the halt, RAX must hold 's' (0x73) and RBX 't' (0x74).".to_owned(),
                },
                starter_code: include_str!("../exercises/chaines.asm").to_owned(),
                goal: vec!["rax == 0x73".to_owned(), "rbx == 0x74".to_owned()],
                par: Some(13),
                level: Level::Structures,
            },
            Exercise {
                id: "comparer_chaines".to_owned(),
                title: Text {
                    fr: "Deux chaînes sont-elles les mêmes ?".to_owned(),
                    en: "Are two strings the same?".to_owned(),
                },
                description: Text {
                    fr: "`repe cmpsb` compare octet par octet tant qu'ils sont égaux ; à la sortie, `ZF` dit si l'on s'est arrêté sur une différence ou parce qu'il n'y avait plus rien à comparer. C'est `memcmp` en trois instructions — la vraie en fait seize à la fois, avec SSE.".to_owned(),
                    en: "`repe cmpsb` compares byte by byte while they are equal; on exit, `ZF` says whether one stopped on a difference or ran out of bytes. This is `memcmp` in three instructions — the real one does sixteen at a time, with SSE.".to_owned(),
                },
                starter_code: include_str!("../exercises/comparer_chaines.asm").to_owned(),
                goal: vec!["rax == 1".to_owned(), "rbx == 0".to_owned()],
                par: Some(22),
                level: Level::Structures,
            },
            Exercise {
                id: "macros".to_owned(),
                title: Text {
                    fr: "Avant l'assemblage : le préprocesseur".to_owned(),
                    en: "Before assembly: the preprocessor".to_owned(),
                },
                description: Text {
                    fr: "`%define` remplace un mot, `%macro` recopie des lignes, `%rep` répète un bloc — tout cela sur le texte, avant la moindre traduction. Une macro n'est pas un appel : ni `call`, ni `ret`, ni pile, et l'onglet Octets le prouve. À l'arrêt, RAX doit valoir 30 et RBX 12.".to_owned(),
                    en: "`%define` replaces a word, `%macro` copies lines in, `%rep` repeats a block — all on the text, before a single translation. A macro is not a call: no `call`, no `ret`, no stack, and the Bytes tab proves it. At the halt, RAX must hold 30 and RBX 12.".to_owned(),
                },
                starter_code: include_str!("../exercises/macros.asm").to_owned(),
                goal: vec!["rax == 0x1e".to_owned(), "rbx == 0xc".to_owned()],
                par: Some(8),
                level: Level::Structures,
            },
            Exercise {
                id: "branchements".to_owned(),
                title: Text {
                    fr: "Choisir parmi plusieurs".to_owned(),
                    en: "Choosing among many".to_owned(),
                },
                description: Text {
                    fr: "Une suite de `cmp`/`je` coûte un test par branche ; une table de saut n'en coûte aucun — on range les adresses dans un tableau et l'on saute à celle que l'indice désigne. C'est ce qu'un compilateur produit pour un `switch` dense. Le prix : rien ne vérifie l'indice, et c'est exactement ce que les protections modernes cherchent à rendre impossible.".to_owned(),
                    en: "A chain of `cmp`/`je` costs one test per branch; a jump table costs none — the addresses live in an array and one jumps to the entry the index picks. That is what a compiler emits for a dense `switch`. The price: nothing checks the index, which is precisely what modern hardening tries to make impossible.".to_owned(),
                },
                starter_code: include_str!("../exercises/branchements.asm").to_owned(),
                goal: vec!["rax == 0x12c".to_owned()],
                par: Some(6),
                level: Level::Structures,
            },
            Exercise {
                id: "pile".to_owned(),
                title: Text {
                    fr: "La pile".to_owned(),
                    en: "The stack".to_owned(),
                },
                description: Text {
                    fr: "`push` descend RSP de huit et y écrit, `pop` fait l'inverse. La pile rend dans l'ordre inverse : deux `push` suivis de deux `pop` échangent deux registres. À vous de les remettre à l'endroit.".to_owned(),
                    en: "`push` lowers RSP by eight and writes there, `pop` does the opposite. The stack gives back in reverse: two pushes followed by two pops swap two registers. Put them back the right way round.".to_owned(),
                },
                starter_code: include_str!("../exercises/pile.asm").to_owned(),
                goal: vec!["rax == 1".to_owned(), "rbx == 2".to_owned()],
                par: Some(9),
                level: Level::Structures,
            },
            Exercise {
                id: "appels".to_owned(),
                title: Text {
                    fr: "Appeler, et rendre ce qu'on doit".to_owned(),
                    en: "Call, and give back what you borrowed".to_owned(),
                },
                description: Text {
                    fr: "Les arguments passent par RDI et RSI, le résultat revient dans RAX. Mais cette routine écrase RBX sans le rendre : l'onglet Pile le signale. Ajoutez le `push`/`pop` qui manque et l'entorse disparaît.".to_owned(),
                    en: "Arguments travel in RDI and RSI, the result comes back in RAX. But this routine keeps RBX without giving it back: the Stack tab says so. Add the missing push/pop and the breach disappears.".to_owned(),
                },
                starter_code: include_str!("../exercises/appels.asm").to_owned(),
                goal: vec!["rax == 7".to_owned()],
                par: Some(9),
                level: Level::Structures,
            },
            Exercise {
                id: "convention".to_owned(),
                title: Text {
                    fr: "Ce qu'une fonction a le droit d'abîmer".to_owned(),
                    en: "What a function may damage".to_owned(),
                },
                description: Text {
                    fr: "La convention partage les registres en deux : ceux qu'une fonction écrase librement, et ceux qu'elle doit rendre intacts — RBX, RBP, R12 à R15. Personne ne l'impose, le processeur s'en moque, mais tout le monde s'y tient, sinon plus rien ne s'appelle. Celle-ci triche : namsdec le dit dans l'onglet Pile. Rendez-la honnête et regardez l'entorse disparaître.".to_owned(),
                    en: "The convention splits the registers in two: those a function may clobber freely, and those it must hand back intact — RBX, RBP, R12 to R15. Nobody enforces it, the processor could not care less, but everyone abides by it, or nothing can call anything. This one cheats: namsdec says so in the Stack tab. Make it honest and watch the breach disappear.".to_owned(),
                },
                starter_code: include_str!("../exercises/convention.asm").to_owned(),
                goal: vec!["rbx == 7".to_owned()],
                par: Some(10),
                level: Level::Structures,
            },
            Exercise {
                id: "recursion".to_owned(),
                title: Text {
                    fr: "Une fonction qui s'appelle elle-même".to_owned(),
                    en: "A function that calls itself".to_owned(),
                },
                description: Text {
                    fr: "Rien de particulier n'est demandé au processeur : `call` empile une adresse de retour, et rien n'interdit d'appeler la fonction où l'on est déjà. C'est la pile qui rend cela possible — et c'est aussi ce qui explique le débordement de pile. Regardez la chaîne des appels s'allonger. À l'arrêt, RAX doit contenir 5! = 120.".to_owned(),
                    en: "Nothing special is asked of the processor: `call` pushes a return address, and nothing forbids calling the function one is already in. The stack is what makes it possible — and what explains stack overflow. Watch the call chain grow. At the halt, RAX must hold 5! = 120.".to_owned(),
                },
                starter_code: include_str!("../exercises/recursion.asm").to_owned(),
                goal: vec!["rax == 0x78".to_owned()],
                par: Some(39),
                level: Level::Structures,
            },
            Exercise {
                id: "cadre".to_owned(),
                title: Text {
                    fr: "Un cadre de pile".to_owned(),
                    en: "A stack frame".to_owned(),
                },
                description: Text {
                    fr: "`push rbp` / `mov rbp, rsp` / `sub rsp, 16` : trois instructions qui donnent à une fonction sa place à elle. Les variables locales se lisent par rapport à RBP, qui ne bouge pas, et non par rapport à RSP, qui bouge à chaque `push`. `leave` défait le tout. À l'arrêt, RAX doit valoir 7 et l'onglet Pile ne rien signaler.".to_owned(),
                    en: "`push rbp` / `mov rbp, rsp` / `sub rsp, 16`: three instructions that give a function room of its own. Locals are read relative to RBP, which stays put, not RSP, which moves on every push. `leave` undoes it all. At the halt, RAX must hold 7 and the Stack tab must report nothing.".to_owned(),
                },
                starter_code: include_str!("../exercises/cadre.asm").to_owned(),
                goal: vec!["rax == 7".to_owned()],
                par: Some(11),
                level: Level::Structures,
            },
            Exercise {
                id: "sortie".to_owned(),
                title: Text {
                    fr: "Parler au système".to_owned(),
                    en: "Talk to the system".to_owned(),
                },
                description: Text {
                    fr: "`write` pour afficher, `exit` pour finir : le numéro dans RAX, les arguments dans RDI, RSI, RDX. Le programme s'arrête pour l'instant sur un `hlt` — remplacez-le par un vrai exit(0).".to_owned(),
                    en: "`write` to print, `exit` to finish: the number in RAX, the arguments in RDI, RSI, RDX. The program stops on a `hlt` for now — replace it with a real exit(0).".to_owned(),
                },
                starter_code: include_str!("../exercises/sortie.asm").to_owned(),
                goal: Vec::new(),
                par: None,
                level: Level::Mastery,
            },
            Exercise {
                id: "entree".to_owned(),
                title: Text {
                    fr: "Lire ce qu'on vous donne".to_owned(),
                    en: "Reading what you are given".to_owned(),
                },
                description: Text {
                    fr: "`read` lit *au plus* ce qu'on lui demande et rend ce qu'il a pu lire — jamais plus, souvent moins, et zéro veut dire « fin d'entrée ». Tapez quelque chose dans la console, envoyez, et regardez RAX : un programme qui suppose avoir tout reçu du premier coup est un programme qui se trompera.".to_owned(),
                    en: "`read` reads *at most* what it is asked for and returns what it managed — never more, often less, and zero means end of input. Type something in the console, send it, and watch RAX: a program assuming it got everything in one go is a program that will be wrong.".to_owned(),
                },
                starter_code: include_str!("../exercises/entree.asm").to_owned(),
                goal: vec![],
                par: None,
                level: Level::Mastery,
            },
            Exercise {
                id: "tas".to_owned(),
                title: Text {
                    fr: "Demander de la mémoire".to_owned(),
                    en: "Asking for memory".to_owned(),
                },
                description: Text {
                    fr: "Un programme naît avec son code, ses données et sa pile ; le reste se demande. `brk(0)` rend la limite du tas, `brk(plus haut)` la déplace, et entre les deux la mémoire est à vous. C'est là-dessus que repose tout `malloc`. L'onglet Tas montre les trois bornes bouger.".to_owned(),
                    en: "A program is born with its code, its data and its stack; the rest is asked for. `brk(0)` returns the heap limit, `brk(higher)` moves it, and what lies between is yours. Every `malloc` rests on this. The Heap tab shows the three bounds move.".to_owned(),
                },
                starter_code: include_str!("../exercises/tas.asm").to_owned(),
                goal: vec!["rax == 0x42".to_owned()],
                par: Some(11),
                level: Level::Mastery,
            },
            Exercise {
                id: "flottants".to_owned(),
                title: Text {
                    fr: "Calculer à virgule".to_owned(),
                    en: "Computing with decimals".to_owned(),
                },
                description: Text {
                    fr: "Les entiers vivent dans RAX, les flottants dans XMM0. `movsd` déplace un double, `addsd` en additionne deux, et le `s` veut dire « un seul » — la même instruction sans lui en traiterait deux d'un coup. À l'arrêt, RAX porte les octets de 3.75 ; survolez-le pour la lecture décimale, ou ouvrez « Registres étendus ».".to_owned(),
                    en: "Integers live in RAX, floats in XMM0. `movsd` moves a double, `addsd` adds two, and the `s` means \"just one\" — the same instruction without it would handle two at once. At the halt, RAX holds the bytes of 3.75; hover it for the decimal reading, or open \"Extended registers\".".to_owned(),
                },
                starter_code: include_str!("../exercises/flottants.asm").to_owned(),
                goal: vec!["rax == 0x400e000000000000".to_owned()],
                par: Some(5),
                level: Level::Mastery,
            },
            Exercise {
                id: "simd".to_owned(),
                title: Text {
                    fr: "Quatre additions d'un coup".to_owned(),
                    en: "Four additions at once".to_owned(),
                },
                description: Text {
                    fr: "Un registre XMM tient quatre entiers de 32 bits, et `paddd` les additionne tous les quatre en même temps — le `p` dit *packed*. C'est le principe de tout le calcul vectoriel, et la raison pour laquelle un même algorithme peut aller quatre fois plus vite. `movdqa` exige un alignement sur seize octets : retirez le `align 16` pour voir ce que le processeur en dit.".to_owned(),
                    en: "An XMM register holds four 32-bit integers, and `paddd` adds all four at once — the `p` means packed. That is the principle behind all vector computing, and why the same algorithm can run four times faster. `movdqa` demands sixteen-byte alignment: remove the `align 16` to see what the processor makes of it.".to_owned(),
                },
                starter_code: include_str!("../exercises/simd.asm").to_owned(),
                goal: vec!["rax == 0xb".to_owned(), "rbx == 0x2c".to_owned()],
                par: Some(6),
                level: Level::Mastery,
            },
            Exercise {
                id: "afficher".to_owned(),
                title: Text {
                    fr: "Afficher un nombre".to_owned(),
                    en: "Printing a number".to_owned(),
                },
                description: Text {
                    fr: "La synthèse du parcours : une boucle, `div` et son RDX, la pile, l'arithmétique des caractères et un appel système. Un ordinateur ne connaît pas les chiffres : afficher 1234, c'est le diviser par dix jusqu'à épuisement — les chiffres sortent à l'envers, et la pile les remet à l'endroit. La console doit afficher 1234.".to_owned(),
                    en: "The course's synthesis: a loop, `div` and its RDX, the stack, character arithmetic and a system call. A computer does not know digits: printing 1234 means dividing by ten until nothing is left — the digits come out backwards, and the stack puts them right. The console must show 1234.".to_owned(),
                },
                starter_code: include_str!("../exercises/afficher.asm").to_owned(),
                goal: vec!["rax == 4".to_owned()],
                par: Some(59),
                level: Level::Mastery,
            },
            Exercise {
                id: "debordement".to_owned(),
                title: Text {
                    fr: "Ce qu'un tampon trop petit laisse faire".to_owned(),
                    en: "What a too-small buffer lets through".to_owned(),
                },
                description: Text {
                    fr: "Seize octets de tampon, soixante-quatre octets lus : le `ret` repart dans le décor. Engendrez un motif cyclique (Outils), donnez-le à manger au programme, puis cliquez RIP dans la fenêtre du motif — le décalage vous dit exactement ce qui a été écrasé. C'est la mesure d'un dégât, et la raison d'être du canari de pile.".to_owned(),
                    en: "A sixteen-byte buffer, sixty-four bytes read: the `ret` goes off into the weeds. Generate a cyclic pattern (Tools), feed it to the program, then click RIP in the pattern window — the offset tells you exactly what was overwritten. This is measuring damage, and the reason the stack canary exists.".to_owned(),
                },
                starter_code: include_str!("../exercises/debordement.asm").to_owned(),
                goal: Vec::new(),
                par: None,
                level: Level::Mastery,
            },
            Exercise {
                id: "protection".to_owned(),
                title: Text {
                    fr: "Ce qu'on n'a pas le droit de faire".to_owned(),
                    en: "What you are not allowed to do".to_owned(),
                },
                description: Text {
                    fr: "Trois refus à comprendre : écrire dans son propre code, exécuter ses données, écrire là où rien n'est projeté. Exécutez, lisez le message, commentez la ligne fautive, recommencez.".to_owned(),
                    en: "Three refusals to understand: writing into your own code, executing your data, writing where nothing is mapped. Run it, read the message, comment out the offending line, run it again.".to_owned(),
                },
                starter_code: include_str!("../exercises/protection.asm").to_owned(),
                goal: Vec::new(),
                par: None,
                level: Level::Mastery,
            },
        ],
    }
}

impl Pack {
    /// L'exercice qui porte cet identifiant.
    pub fn by_id(&self, id: &str) -> Option<&Exercise> {
        self.exercises.iter().find(|e| e.id == id)
    }
}

/// Cherche un exercice dans plusieurs séries à la fois, par son
/// identifiant *complet* — « série/exercice ».
///
/// Complet parce que deux séries peuvent nommer leur premier exercice de
/// la même façon, et que celui qu'on a cliqué est celui qu'on veut.
pub fn find<'a>(packs: &'a [Pack], full_id: &str) -> Option<&'a Exercise> {
    let (pack_id, exercise_id) = full_id.split_once('/')?;
    packs.iter().find(|p| p.id == pack_id)?.by_id(exercise_id)
}

/// L'identifiant complet d'un exercice dans sa série.
pub fn full_id(pack: &Pack, exercise: &Exercise) -> String {
    format!("{}/{}", pack.id, exercise.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_exercise_is_complete_and_stops() {
        let pack = builtin_pack();
        // Un parcours, pas une poignée : trente-six étapes, et chacune
        // s'appuie sur les précédentes.
        assert_eq!(pack.exercises.len(), 36);
        for exercise in &pack.exercises {
            assert!(!exercise.id.is_empty());
            assert!(!exercise.title.fr.is_empty() && !exercise.title.en.is_empty());
            assert!(!exercise.description.fr.is_empty());
            assert!(!exercise.description.en.is_empty());
            // Un exercice est un programme complet, pas un extrait : il
            // déclare son point d'entrée, sans quoi l'éditeur de liens
            // en choisit un à sa place — et le programme démarre là où
            // personne ne l'a demandé.
            assert!(
                exercise.starter_code.contains("global _start"),
                "{} ne déclare pas son point d'entrée",
                exercise.id
            );
            assert!(exercise.starter_code.contains("_start:"));
            // Et il doit s'arrêter, sinon « Exécuter » ne rend jamais la
            // main.
            assert!(
                exercise.starter_code.contains("hlt") || exercise.starter_code.contains("exit"),
                "{} ne s'arrête pas",
                exercise.id
            );
        }
    }

    /// Les paliers ne reculent jamais : le parcours va de la découverte
    /// à la maîtrise, et rien n'y revient en arrière.
    #[test]
    fn the_course_never_goes_back_down_a_level() {
        let pack = builtin_pack();
        let mut niveaux = pack.exercises.iter().map(|exercise| exercise.level);
        let mut précédent = niveaux.next().expect("au moins un exercice");
        for niveau in niveaux {
            assert!(
                niveau >= précédent,
                "un exercice de niveau {précédent:?} précède un {niveau:?}"
            );
            précédent = niveau;
        }
        // Et les quatre paliers sont représentés : un parcours qui n'en
        // aurait que trois n'aurait pas besoin du quatrième.
        for niveau in Level::ALL {
            assert!(
                pack.exercises
                    .iter()
                    .any(|exercise| exercise.level == niveau),
                "aucun exercice en {niveau:?}"
            );
        }
    }

    #[test]
    fn an_exercise_is_found_by_its_full_identifier() {
        let packs = vec![builtin_pack()];
        let found = find(&packs, "namsdec/boucle").expect("l'exercice existe");
        assert_eq!(found.id, "boucle");
        assert_eq!(full_id(&packs[0], found), "namsdec/boucle");
        // Le nom seul ne suffit pas : deux séries peuvent le porter.
        assert!(find(&packs, "boucle").is_none());
        assert!(find(&packs, "autre/boucle").is_none());
    }
}
