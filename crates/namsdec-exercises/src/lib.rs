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
        // Un parcours, pas une poignée : seize étapes, et chacune
        // s'appuie sur les précédentes.
        assert_eq!(pack.exercises.len(), 16);
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
