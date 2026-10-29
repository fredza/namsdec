//! La table des fiches, une par famille d'instructions.
//!
//! Écrite à la main et rangée par sujet : déplacer, calculer, brancher,
//! puis les chaînes, le x87 et le vectoriel — et à la fin le langage de
//! NASM lui-même, directives et préprocesseur, qui ne s'exécute pas mais
//! décide de ce qui sera écrit. L'ordre n'a d'importance que pour qui
//! relit le fichier : la recherche passe par un index.

use crate::{Entry, Kind, Text};

pub const ENTRIES: &[Entry] = &[
    Entry {
        kind: Kind::Instruction,
        names: &["mov"],
        title: Text {
            fr: "Copier une valeur",
            en: "Copy a value",
        },
        body: Text {
            fr: "Copie la source dans la destination, sans jamais toucher aux indicateurs. Deux pièges de largeur : écrire un registre 32 bits met à zéro les 32 bits hauts du registre 64, alors qu'écrire 8 ou 16 bits laisse le reste intact.",
            en: "Copies the source into the destination and never touches the flags. Two width traps: writing a 32-bit register zeroes the upper 32 bits of the 64-bit one, while writing 8 or 16 bits leaves the rest alone.",
        },
        flags: "",
        example: "mov rax, 5\nmov eax, 5      ; RAX vaut 5 : le haut est effacé",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["movzx", "movsx", "movsxd"],
        title: Text {
            fr: "Élargir une valeur",
            en: "Widen a value",
        },
        body: Text {
            fr: "Copient une valeur étroite dans un registre plus large. `movzx` complète par des zéros, `movsx` recopie le bit de signe — c'est la différence entre lire un octet comme 0 à 255 ou comme -128 à 127. `movsxd` est la forme 32 vers 64.",
            en: "Copy a narrow value into a wider register. `movzx` fills with zeroes, `movsx` copies the sign bit — the difference between reading a byte as 0..255 or as -128..127. `movsxd` is the 32-to-64 form.",
        },
        flags: "",
        example: "movzx eax, byte [buf]   ; 0xff -> 255\nmovsx eax, byte [buf]   ; 0xff -> -1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["lea"],
        title: Text {
            fr: "Calculer une adresse",
            en: "Compute an address",
        },
        body: Text {
            fr: "Rend l'adresse qu'un accès mémoire aurait calculée, sans lire la mémoire. Ce qui en fait aussi la façon la plus courte d'écrire `a + b*4 + 8` en une instruction, sans toucher aux indicateurs.",
            en: "Yields the address a memory access would have computed, without reading memory. Which also makes it the shortest way to write `a + b*4 + 8` in one instruction, flags untouched.",
        },
        flags: "",
        example: "lea rax, [rbx + rcx*4 + 8]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["xchg"],
        title: Text {
            fr: "Échanger",
            en: "Exchange",
        },
        body: Text {
            fr: "Échange les deux opérandes. Sur une opérande mémoire, l'échange est atomique même sans préfixe `lock` — le seul cas où il l'est d'office.",
            en: "Swaps the two operands. On a memory operand the exchange is atomic even without a `lock` prefix — the only instruction where that is implicit.",
        },
        flags: "",
        example: "xchg rax, rbx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["xadd"],
        title: Text {
            fr: "Échanger puis additionner",
            en: "Exchange and add",
        },
        body: Text {
            fr: "La destination reçoit la somme, la source reçoit l'ancienne destination. Avec `lock`, c'est le compteur partagé de tous les programmes concurrents : on incrémente et on apprend la valeur d'avant, en une fois.",
            en: "The destination gets the sum, the source gets the old destination. With `lock`, this is every concurrent program's shared counter: increment and learn the previous value in one go.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "lock xadd [compteur], rax   ; rax reçoit l'ancienne valeur",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cmpxchg"],
        title: Text {
            fr: "Comparer et échanger",
            en: "Compare and exchange",
        },
        body: Text {
            fr: "Compare l'accumulateur à la destination. S'ils sont égaux, la source y est écrite et ZF vaut 1 ; sinon l'accumulateur reçoit ce qui s'y trouvait, de quoi réessayer avec la bonne valeur de départ. C'est la brique de tous les verrous.",
            en: "Compares the accumulator with the destination. If equal, the source is written there and ZF is 1; otherwise the accumulator receives what was found, ready for another try from the right starting value. The building block of every lock.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "mov rax, attendu\ncmpxchg [verrou], rbx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["bswap", "movbe"],
        title: Text {
            fr: "Retourner les octets",
            en: "Reverse the bytes",
        },
        body: Text {
            fr: "Inversent l'ordre des octets : c'est le passage entre l'ordre du x86 (petit-boutiste) et celui des formats réseau ou de fichiers (gros-boutiste). `movbe` fait la même chose en lisant ou en écrivant la mémoire au passage.",
            en: "Reverse byte order: the bridge between the x86's little-endian layout and the big-endian one of network and file formats. `movbe` does the same while reading or writing memory.",
        },
        flags: "",
        example: "mov eax, 0x11223344\nbswap eax               ; 0x44332211",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["push", "pop"],
        title: Text {
            fr: "Empiler, dépiler",
            en: "Push, pop",
        },
        body: Text {
            fr: "`push` fait descendre RSP de huit octets puis y écrit ; `pop` lit et le fait remonter. La pile descend : la « dernière » valeur est à l'adresse la plus basse, et c'est RSP qui la désigne.",
            en: "`push` lowers RSP by eight bytes then writes there; `pop` reads and raises it. The stack grows down: the most recent value sits at the lowest address, and RSP points at it.",
        },
        flags: "",
        example: "push rbp\npop rbp",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["pushf", "pushfq", "pushfd", "popf", "popfq", "popfd"],
        title: Text {
            fr: "Ranger les indicateurs",
            en: "Save the flags",
        },
        body: Text {
            fr: "Empilent ou dépilent le registre des indicateurs. C'est ainsi qu'une routine emprunte CF ou ZF sans perturber le calcul de celle qui l'appelle.",
            en: "Push or pop the flags register. This is how a routine borrows CF or ZF without disturbing its caller's calculation.",
        },
        flags: "CF PF AF ZF SF OF (au dépilement)",
        example: "pushfq\n; … un calcul qui écrase les indicateurs\npopfq",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["add", "sub"],
        title: Text {
            fr: "Additionner, soustraire",
            en: "Add, subtract",
        },
        body: Text {
            fr: "L'addition et la soustraction, sur 8 à 64 bits. CF signale le débordement *non signé* (une retenue), OF le débordement *signé* : les deux répondent à des questions différentes, et c'est l'instruction de saut qui choisit laquelle écouter.",
            en: "Addition and subtraction, 8 to 64 bits. CF reports *unsigned* overflow (a carry), OF reports *signed* overflow: two different questions, and the following jump decides which one to listen to.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "add rax, rbx\nsub rax, 1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["adc", "sbb"],
        title: Text {
            fr: "Additionner avec la retenue",
            en: "Add with carry",
        },
        body: Text {
            fr: "Comme `add` et `sub`, mais en tenant compte de CF. C'est ainsi qu'on additionne plus large qu'un registre : le morceau bas d'abord avec `add`, les suivants avec `adc`.",
            en: "Like `add` and `sub`, but taking CF into account. This is how you add numbers wider than a register: the low part with `add`, the rest with `adc`.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "add rax, rcx\nadc rdx, rbx        ; la retenue remonte",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["inc", "dec"],
        title: Text {
            fr: "Incrémenter, décrémenter",
            en: "Increment, decrement",
        },
        body: Text {
            fr: "Ajoutent ou retranchent 1. Elles laissent CF *intacte*, contrairement à `add reg, 1` : c'est le détail qui permet de compter au milieu d'une addition multi-registres sans en perdre la retenue.",
            en: "Add or subtract 1. They leave CF *untouched*, unlike `add reg, 1`: the detail that lets you count in the middle of a multi-register addition without losing its carry.",
        },
        flags: "PF AF ZF SF OF",
        example: "inc rcx\ndec rdx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["neg"],
        title: Text {
            fr: "Changer de signe",
            en: "Negate",
        },
        body: Text {
            fr: "Remplace la valeur par son opposé, c'est-à-dire par 0 moins elle. CF vaut 1 sauf si l'opérande était nulle.",
            en: "Replaces the value with its negation, that is, 0 minus it. CF is 1 unless the operand was zero.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "neg rax",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cmp"],
        title: Text {
            fr: "Comparer",
            en: "Compare",
        },
        body: Text {
            fr: "Soustrait sans rien écrire : seuls les indicateurs restent. La comparaison qu'il faut ensuite lire dépend du signe des nombres — `ja`/`jb` pour des non signés, `jg`/`jl` pour des signés.",
            en: "Subtracts without writing anything: only the flags remain. Which jump to read them with depends on signedness — `ja`/`jb` for unsigned, `jg`/`jl` for signed.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "cmp rax, rbx\njg plus_grand",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["mul", "imul"],
        title: Text {
            fr: "Multiplier",
            en: "Multiply",
        },
        body: Text {
            fr: "`mul` multiplie sans signe, `imul` avec. Le produit de deux nombres de n bits en fait 2n : la forme à un opérande range donc la moitié haute dans RDX. Les formes à deux et trois opérandes, elles, tronquent — c'est ce qu'on veut presque toujours.",
            en: "`mul` multiplies unsigned, `imul` signed. The product of two n-bit numbers has 2n bits: the one-operand form therefore puts the high half in RDX. The two- and three-operand forms truncate, which is nearly always what you want.",
        },
        flags: "CF OF",
        example: "imul rax, rbx, 3    ; rax = rbx * 3\nmul rbx             ; RDX:RAX = RAX * RBX",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["div", "idiv"],
        title: Text {
            fr: "Diviser",
            en: "Divide",
        },
        body: Text {
            fr: "Divisent RDX:RAX par l'opérande : le quotient va dans RAX, le reste dans RDX. Il faut donc préparer RDX — `xor rdx, rdx` avant `div`, `cqo` avant `idiv` —, faute de quoi on divise par un reste oublié. Une division par zéro, ou un quotient trop grand, arrête le programme.",
            en: "Divide RDX:RAX by the operand: quotient into RAX, remainder into RDX. RDX must therefore be prepared — `xor rdx, rdx` before `div`, `cqo` before `idiv` — or you divide by a forgotten remainder. Dividing by zero, or a quotient too large, faults.",
        },
        flags: "",
        example: "mov rax, -17\ncqo\nidiv rbx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["pext", "pdep"],
        title: Text {
            fr: "Ramasser et éparpiller des bits",
            en: "Gather and scatter bits",
        },
        body: Text {
            fr: "Deux inverses l'une de l'autre, et un masque dit lesquels.\n\n`pext` **ramasse** : elle prend les bits que le masque désigne, dans l'ordre, et les tasse tout en bas du résultat. `pdep` **éparpille** : elle prend les bits bas de la source et les pose aux places que le masque désigne. Avec le même masque, `pdep` défait exactement ce que `pext` a fait.\n\nElles remplacent des boucles entières : extraire les champs d'un mot d'état, recomposer une adresse dont les morceaux sont éclatés, passer d'un index dense à un index creux. Aucune des deux ne touche aux indicateurs, ce qui permet de les glisser au milieu d'un calcul.\n\nLe sens se retient par le mot : **ext**ract ramasse vers le bas, **dep**osit repose en place.",
            en: "Two inverses of each other, and a mask says which bits.\n\n`pext` **gathers**: it takes the bits the mask names, in order, and packs them at the bottom of the result. `pdep` **scatters**: it takes the low bits of the source and places them where the mask says. With the same mask, `pdep` undoes exactly what `pext` did.\n\nThey replace whole loops: pulling the fields out of a status word, rebuilding an address whose pieces are scattered, turning a dense index into a sparse one. Neither touches the flags, so they slot into the middle of a computation.\n\nThe names carry the sense: **ext**ract packs down, **dep**osit puts back.",
        },
        flags: "",
        example: "mov rbx, 0x12345678\nmov rcx, 0xFF00FF00   ; on retient deux octets\npext rax, rbx, rcx    ; tassés : 0x1256\npdep rdx, rax, rcx    ; et remis : 0x12005600",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["mulx"],
        title: Text {
            fr: "Multiplier sans indicateurs",
            en: "Multiply without flags",
        },
        body: Text {
            fr: "Multiplie RDX par la source et range les deux moitiés du produit là où on le lui dit, sans toucher à un seul indicateur. C'est ce qui permet de la glisser au milieu d'une addition en chaîne.",
            en: "Multiplies RDX by the source and puts the two halves of the product wherever it is told, without touching a single flag. Which is what lets it sit in the middle of a carry chain.",
        },
        flags: "",
        example: "mov rdx, [a]          ; le multiplicande, implicite\nmulx r9, r8, [b]      ; r9:r8 = a * b",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["and", "or", "xor", "not"],
        title: Text {
            fr: "Opérations bit à bit",
            en: "Bitwise operations",
        },
        body: Text {
            fr: "Les quatre opérations logiques, bit par bit. `and` sert à masquer, `or` à poser des bits, `xor` à les retourner — et `xor rax, rax` est la façon la plus courte de mettre un registre à zéro. `not` inverse tout sans toucher aux indicateurs.",
            en: "The four logical operations, bit by bit. `and` masks, `or` sets bits, `xor` flips them — and `xor rax, rax` is the shortest way to zero a register. `not` inverts everything without touching the flags.",
        },
        flags: "CF PF ZF SF OF",
        example: "and rax, 0xff       ; ne garder que l'octet bas\nxor rax, rax        ; mettre à zéro",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["test"],
        title: Text {
            fr: "Tester des bits",
            en: "Test bits",
        },
        body: Text {
            fr: "Fait un `and` sans écrire le résultat : seuls les indicateurs restent. `test rax, rax` est la façon idiomatique de demander « est-ce nul ? », et `test al, 1` « ce bit est-il posé ? ».",
            en: "Performs an `and` without writing the result: only the flags remain. `test rax, rax` is the idiomatic way to ask \"is it zero?\", and `test al, 1` \"is that bit set?\".",
        },
        flags: "CF PF ZF SF OF",
        example: "test rax, rax\njz nul",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["andn"],
        title: Text {
            fr: "ET avec le complément",
            en: "AND with complement",
        },
        body: Text {
            fr: "Calcule « non-source1 ET source2 » en une instruction, sans détruire ses sources. Utile pour effacer les bits d'un masque.",
            en: "Computes \"NOT source1 AND source2\" in one instruction, without destroying its sources. Handy to clear the bits of a mask.",
        },
        flags: "ZF SF",
        example: "mov rbx, [masque]\nandn rax, rbx, [valeur]   ; rax = ~masque & valeur",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["shl", "sal", "shr", "sar"],
        title: Text {
            fr: "Décaler",
            en: "Shift",
        },
        body: Text {
            fr: "Décalent les bits. À gauche (`shl`, `sal`) c'est une multiplication par deux ; à droite, `shr` fait entrer des zéros — division non signée — et `sar` recopie le bit de signe, ce qui divise correctement un nombre négatif. Le compte est masqué à 5 bits (6 en 64 bits) : décaler de 64 ne fait rien.",
            en: "Shift the bits. Left (`shl`, `sal`) is a multiplication by two; on the right, `shr` brings in zeroes — unsigned division — while `sar` copies the sign bit, which divides a negative number correctly. The count is masked to 5 bits (6 in 64-bit): shifting by 64 does nothing.",
        },
        flags: "CF PF ZF SF OF",
        example: "shl rax, 3          ; rax * 8\nsar rax, 1          ; rax / 2, signe compris",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["rol", "ror", "rcl", "rcr"],
        title: Text {
            fr: "Tourner",
            en: "Rotate",
        },
        body: Text {
            fr: "Ce qui sort d'un côté rentre de l'autre : rien n'est perdu, contrairement à un décalage. `rcl` et `rcr` font passer le bit par la retenue, ce qui donne un anneau d'un bit plus long. Elles ne touchent ni ZF ni SF.",
            en: "What leaves on one side comes back on the other: nothing is lost, unlike a shift. `rcl` and `rcr` route the bit through the carry, making the ring one bit longer. They touch neither ZF nor SF.",
        },
        flags: "CF OF",
        example: "rol al, 1\nrcr rax, 1          ; la retenue entre par le haut",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["shld", "shrd"],
        title: Text {
            fr: "Décalage double",
            en: "Double shift",
        },
        body: Text {
            fr: "La destination se décale, et les bits qui entrent viennent de la source au lieu d'être des zéros. C'est ainsi qu'on décale une valeur plus large qu'un registre.",
            en: "The destination shifts, and the bits coming in are taken from the source instead of being zeroes. This is how you shift a value wider than a register.",
        },
        flags: "CF PF ZF SF OF",
        example: "shld rdx, rax, 4    ; les quatre bits hauts de rax entrent dans rdx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["shlx", "shrx", "sarx", "rorx"],
        title: Text {
            fr: "Décaler sans indicateurs",
            en: "Shift without flags",
        },
        body: Text {
            fr: "Les mêmes décalages, avec une destination séparée et sans toucher un seul indicateur — ce qui permet de les glisser au milieu d'un calcul dont les indicateurs servent ailleurs. `rorx` ajoute la rotation.",
            en: "The same shifts, with a separate destination and without touching a single flag — which lets them sit in the middle of a calculation whose flags matter elsewhere. `rorx` adds the rotation.",
        },
        flags: "",
        example: "shlx rax, rbx, rcx  ; rbx décalé de rcx, dans rax",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["bt", "bts", "btr", "btc"],
        title: Text {
            fr: "Le bit désigné",
            en: "The named bit",
        },
        body: Text {
            fr: "Copient dans CF le bit dont on donne le numéro, puis le laissent (`bt`), le posent (`bts`), l'effacent (`btr`) ou le retournent (`btc`). Sur une zone mémoire avec un numéro en registre, le décalage n'est pas borné par l'opérande : c'est ce qui permet de traiter un tableau de bits.",
            en: "Copy the numbered bit into CF, then leave it (`bt`), set it (`bts`), clear it (`btr`) or flip it (`btc`). On a memory operand with the offset in a register, the offset is not bounded by the operand size: that is what lets you address a whole bit array.",
        },
        flags: "CF",
        example: "bts qword [drapeaux], rax\njc était_déjà_posé",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["bsf", "bsr"],
        title: Text {
            fr: "Chercher un bit",
            en: "Scan for a bit",
        },
        body: Text {
            fr: "Rendent le numéro du bit posé le plus bas (`bsf`) ou le plus haut (`bsr`). Si la source est nulle, ZF vaut 1 et la destination n'est pas définie — d'où `tzcnt`/`lzcnt`, qui rendent une réponse dans tous les cas.",
            en: "Return the index of the lowest (`bsf`) or highest (`bsr`) set bit. If the source is zero, ZF is 1 and the destination is undefined — hence `tzcnt`/`lzcnt`, which always answer.",
        },
        flags: "ZF",
        example: "bsf rax, rbx        ; le premier bit posé de rbx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["tzcnt", "lzcnt"],
        title: Text {
            fr: "Compter les zéros",
            en: "Count zeroes",
        },
        body: Text {
            fr: "Comptent les zéros avant le premier bit posé, par le bas (`tzcnt`) ou par le haut (`lzcnt`). Une source nulle rend la largeur de l'opérande, et pose CF — pas de cas particulier à traiter.",
            en: "Count the zeroes before the first set bit, from the bottom (`tzcnt`) or the top (`lzcnt`). A zero source returns the operand width and sets CF — no special case to handle.",
        },
        flags: "CF ZF",
        example: "tzcnt rax, rbx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["popcnt"],
        title: Text {
            fr: "Compter les bits posés",
            en: "Count set bits",
        },
        body: Text {
            fr: "Rend le nombre de bits à 1. Une instruction pour ce qui demandait une boucle — et la brique de toute distance de Hamming.",
            en: "Returns the number of bits set to 1. One instruction for what used to be a loop — and the building block of every Hamming distance.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "popcnt rax, rbx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["blsi", "blsr", "blsmsk"],
        title: Text {
            fr: "Le bit bas isolé",
            en: "The lowest bit, isolated",
        },
        body: Text {
            fr: "Trois variations sur `x & -x` : `blsi` garde le bit posé le plus bas, `blsr` l'efface, `blsmsk` fabrique le masque de tout ce qui est en dessous. De quoi parcourir les bits d'un masque un par un.",
            en: "Three variations on `x & -x`: `blsi` keeps the lowest set bit, `blsr` clears it, `blsmsk` builds the mask of everything below. Enough to walk a mask's bits one at a time.",
        },
        flags: "CF ZF SF",
        example: "blsr rax, rbx       ; rbx sans son bit bas",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["bextr", "bzhi"],
        title: Text {
            fr: "Extraire des bits",
            en: "Extract bits",
        },
        body: Text {
            fr: "`bextr` prend un champ de bits — départ et longueur dans l'opérande de contrôle — et le ramène en bas. `bzhi` efface tout à partir du bit indiqué. Deux instructions pour ce qui demandait un décalage et un masque.",
            en: "`bextr` takes a bit field — start and length in the control operand — and brings it down. `bzhi` clears everything from the given bit up. Two instructions for what took a shift and a mask.",
        },
        flags: "ZF",
        example: "mov rcx, 0x0804     ; huit bits à partir du quatrième\nbextr rax, rbx, rcx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cbw", "cwde", "cdqe"],
        title: Text {
            fr: "Élargir l'accumulateur",
            en: "Widen the accumulator",
        },
        body: Text {
            fr: "Recopient le bit de signe de l'accumulateur dans sa moitié haute : AL vers AX, AX vers EAX, EAX vers RAX. La même valeur, écrite plus large.",
            en: "Copy the accumulator's sign bit into its upper half: AL to AX, AX to EAX, EAX to RAX. The same value, written wider.",
        },
        flags: "",
        example: "mov al, -5\ncbw                 ; AX vaut -5",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cwd", "cdq", "cqo"],
        title: Text {
            fr: "Préparer une division signée",
            en: "Prepare a signed division",
        },
        body: Text {
            fr: "Remplissent le registre voisin du bit de signe de l'accumulateur — RDX pour RAX. À faire avant chaque `idiv` : sans elles, on divise par un reste oublié.",
            en: "Fill the neighbouring register with the accumulator's sign bit — RDX for RAX. To be done before every `idiv`: without them, you divide by a forgotten remainder.",
        },
        flags: "",
        example: "mov rax, -17\ncqo\nidiv rbx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["jmp"],
        title: Text {
            fr: "Sauter",
            en: "Jump",
        },
        body: Text {
            fr: "Continue ailleurs, sans condition. La cible peut être une étiquette, un registre ou une case mémoire — c'est ainsi que se compile une table de branchements.",
            en: "Continue elsewhere, unconditionally. The target can be a label, a register or a memory slot — which is how a jump table compiles.",
        },
        flags: "",
        example: "jmp fin\njmp [table + rax*8]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "je", "jz", "jne", "jnz", "jb", "jnae", "jc", "jnb", "jae", "jnc", "jbe", "jna", "ja",
            "jnbe", "jl", "jnge", "jge", "jnl", "jle", "jng", "jg", "jnle", "js", "jns", "jo",
            "jno", "jp", "jnp",
        ],
        title: Text {
            fr: "Sauter si…",
            en: "Jump if…",
        },
        body: Text {
            fr: "Sautent selon les indicateurs laissés par la comparaison précédente. Deux familles à ne pas mélanger : `ja`/`jb` lisent CF et valent pour des nombres *non signés*, `jg`/`jl` lisent SF et OF et valent pour des *signés*. Plusieurs noms désignent souvent le même saut (`je` et `jz`, `jb`, `jc` et `jnae`).",
            en: "Jump depending on the flags left by the previous comparison. Two families not to be mixed up: `ja`/`jb` read CF and apply to *unsigned* numbers, `jg`/`jl` read SF and OF and apply to *signed* ones. Several names often mean the same jump (`je` and `jz`, or `jb`, `jc` and `jnae`).",
        },
        flags: "",
        example: "cmp rax, rbx\nja plus_grand       ; non signé\njg plus_grand       ; signé",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["loop", "loope", "loopz", "loopne", "loopnz"],
        title: Text {
            fr: "Boucler sur RCX",
            en: "Loop on RCX",
        },
        body: Text {
            fr: "Décrémentent RCX et sautent tant qu'il n'est pas nul. `loope` s'arrête aussi quand ZF tombe, `loopne` quand il se lève : de quoi chercher dans une suite en comptant les tours. Elles ne touchent aucun indicateur.",
            en: "Decrement RCX and jump while it is not zero. `loope` also stops when ZF drops, `loopne` when it rises: enough to search a sequence while counting the passes. They touch no flags.",
        },
        flags: "",
        example: "mov rcx, 10\nboucle:\n    loop boucle",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["jrcxz", "jecxz"],
        title: Text {
            fr: "Sauter si le compteur est nul",
            en: "Jump if the counter is zero",
        },
        body: Text {
            fr: "Sautent si RCX (ou ECX) vaut zéro, sans le décrémenter ni toucher aux indicateurs. Posée avant une boucle `loop`, elle évite les 2^64 tours que ferait un compteur déjà nul.",
            en: "Jump if RCX (or ECX) is zero, without decrementing it or touching the flags. Placed before a `loop`, it avoids the 2^64 passes an already-zero counter would make.",
        },
        flags: "",
        example: "jrcxz fini\nboucle:\n    loop boucle",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["call", "ret"],
        title: Text {
            fr: "Appeler, revenir",
            en: "Call, return",
        },
        body: Text {
            fr: "`call` empile l'adresse de l'instruction suivante puis saute ; `ret` dépile cette adresse et y retourne. Tout le reste — quels registres sont préservés, où passent les arguments — est une convention, pas une règle du processeur.",
            en: "`call` pushes the address of the following instruction then jumps; `ret` pops that address and goes back. Everything else — which registers are preserved, where arguments go — is a convention, not a rule of the processor.",
        },
        flags: "",
        example: "call somme\n; …\nsomme:\n    ret",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["enter", "leave"],
        title: Text {
            fr: "Le cadre de pile",
            en: "The stack frame",
        },
        body: Text {
            fr: "`enter` empile RBP, l'installe sur RSP et réserve la place demandée ; `leave` défait exactement cela. Deux instructions pour le prologue et l'épilogue qu'on écrit sinon à la main.",
            en: "`enter` pushes RBP, points it at RSP and reserves the requested room; `leave` undoes exactly that. Two instructions for the prologue and epilogue you would otherwise write by hand.",
        },
        flags: "",
        example: "enter 32, 0\n; … 32 octets de variables locales\nleave\nret",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["hlt"],
        title: Text {
            fr: "Arrêter la machine",
            en: "Halt",
        },
        body: Text {
            fr: "Arrête l'exécution. Sur un vrai système c'est une instruction privilégiée ; ici, c'est la façon de finir un programme sans passer par `exit`, et l'interface le signale comme un arrêt et non comme une sortie.",
            en: "Stops execution. On a real system it is a privileged instruction; here it is the way to end a program without going through `exit`, and the interface reports a halt rather than an exit.",
        },
        flags: "",
        example: "hlt",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "nop", "pause", "endbr64", "endbr32", "mfence", "lfence", "sfence",
        ],
        title: Text {
            fr: "Ne rien faire",
            en: "Do nothing",
        },
        body: Text {
            fr: "Instructions sans effet sur les registres. `nop` remplit ; `pause` fait patienter une boucle d'attente active ; `endbr64` marque une cible de saut indirect légitime, ce que les compilateurs posent en tête de fonction ; les barrières ordonnent les accès mémoire sur un vrai processeur — ici, rien à ordonner.",
            en: "Instructions with no effect on the registers. `nop` fills; `pause` throttles a spin loop; `endbr64` marks a legitimate indirect-jump target, which compilers place at the top of every function; the fences order memory accesses on a real processor — here there is nothing to order.",
        },
        flags: "",
        example: "nop\nendbr64",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "rdsspq",
            "rdsspd",
            "incsspq",
            "incsspd",
            "saveprevssp",
            "clrssbsy",
        ],
        title: Text {
            fr: "La pile fantôme",
            en: "The shadow stack",
        },
        body: Text {
            fr: "Un second exemplaire des adresses de retour, que le processeur tient à part et que le programme ne peut pas écrire. À chaque `call`, l'adresse y est copiée ; à chaque `ret`, les deux sont comparées — si la pile ordinaire a été récrite, elles diffèrent, et le processeur arrête tout. C'est la réponse matérielle au débordement de tampon, et elle rend inopérante la technique qui consiste à écraser une adresse de retour.\n\n`rdssp` lit le pointeur de cette pile, `incssp` l'avance, `saveprevssp` et `clrssbsy` servent au changement de contexte. Sur une machine sans le dispositif — la plupart —, elles ne font **rien** : `rdssp` laisse son registre inchangé, et c'est ainsi qu'une bibliothèque teste la présence du dispositif. Elle met zéro dans un registre, lit, et regarde si c'est toujours zéro. namsdec fait comme ces machines-là.",
            en: "A second copy of the return addresses, kept aside by the processor and unwritable by the program. Every `call` copies the address there; every `ret` compares the two — if the ordinary stack was rewritten they differ, and the processor stops everything. This is the hardware answer to the buffer overflow, and it defeats the technique of smashing a return address.\n\n`rdssp` reads that stack's pointer, `incssp` advances it, `saveprevssp` and `clrssbsy` serve context switches. On a machine without the feature — most of them — they do **nothing**: `rdssp` leaves its register untouched, and that is how a library tests for the feature. It zeroes a register, reads, and checks whether it is still zero. namsdec behaves like those machines.",
        },
        flags: "",
        example: "xor eax, eax\n    rdsspq rax\n    test rax, rax",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["int3", "ud2"],
        title: Text {
            fr: "S'arrêter exprès",
            en: "Stop on purpose",
        },
        body: Text {
            fr: "`int3` est le point d'arrêt qu'un programme se pose à lui-même — c'est l'octet qu'un débogueur écrit dans le code. `ud2` est une instruction invalide volontaire, que les compilateurs posent là où le programme ne doit jamais passer.",
            en: "`int3` is the breakpoint a program sets for itself — the byte a debugger writes into the code. `ud2` is a deliberately invalid instruction, which compilers place where control must never reach.",
        },
        flags: "",
        example: "int3\nud2",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["syscall"],
        title: Text {
            fr: "Appeler le système",
            en: "Call the system",
        },
        body: Text {
            fr: "Passe la main au noyau : le numéro dans RAX, les arguments dans RDI, RSI, RDX, R10, R8, R9, et la réponse dans RAX. namsdec en simule quatre — `read` (0), `write` (1), `brk` (12) et `exit` (60) — et s'arrête en le disant sur tous les autres.",
            en: "Hands over to the kernel: number in RAX, arguments in RDI, RSI, RDX, R10, R8, R9, answer in RAX. namsdec simulates four — `read` (0), `write` (1), `brk` (12) and `exit` (60) — and stops with a message on any other.",
        },
        flags: "",
        example: "mov rax, 1          ; write\nmov rdi, 1          ; sortie standard\nsyscall",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "seta", "setae", "setb", "setbe", "setc", "setnc", "sete", "setg", "setge", "setl",
            "setle", "setne", "setno", "setnp", "setns", "setnz", "seto", "setp", "sets", "setz",
        ],
        title: Text {
            fr: "Poser un booléen",
            en: "Set a boolean",
        },
        body: Text {
            fr: "Écrivent 1 ou 0 dans un octet selon la même condition qu'un saut. C'est ainsi qu'on obtient une valeur — à additionner, à masquer — plutôt qu'un branchement.",
            en: "Write 1 or 0 into a byte according to the same condition as a jump. This is how you get a value — to add, to mask — instead of a branch.",
        },
        flags: "",
        example: "cmp rax, rbx\nsetg al             ; al = 1 si rax > rbx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "cmova", "cmovae", "cmovb", "cmovbe", "cmovc", "cmove", "cmovg", "cmovge", "cmovl",
            "cmovle", "cmovnc", "cmovne", "cmovno", "cmovnp", "cmovns", "cmovnz", "cmovo", "cmovp",
            "cmovs", "cmovz",
        ],
        title: Text {
            fr: "Copier si…",
            en: "Move if…",
        },
        body: Text {
            fr: "Copient la source si la condition est vraie, et ne font rien sinon. Un branchement en moins : le processeur ne peut pas se tromper de prédiction sur ce qu'il n'a pas à prédire.",
            en: "Copy the source if the condition holds, and do nothing otherwise. One branch fewer: the processor cannot mispredict what it does not have to predict.",
        },
        flags: "",
        example: "cmp rax, rbx\ncmovg rax, rbx      ; garder le plus petit",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["clc", "stc", "cmc", "cld", "std"],
        title: Text {
            fr: "Poser un indicateur",
            en: "Set a flag",
        },
        body: Text {
            fr: "Posent, effacent ou retournent la retenue (`clc`, `stc`, `cmc`) et le sens de parcours des instructions de chaînes (`cld` vers le haut, `std` vers le bas). Une routine qui met `std` doit remettre `cld` avant de rendre la main.",
            en: "Set, clear or flip the carry (`clc`, `stc`, `cmc`) and the direction of the string instructions (`cld` upward, `std` downward). A routine that sets `std` must restore `cld` before returning.",
        },
        flags: "CF DF",
        example: "cld\nrep movsb",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["lahf", "sahf"],
        title: Text {
            fr: "Les indicateurs dans AH",
            en: "The flags in AH",
        },
        body: Text {
            fr: "Font passer les indicateurs bas entre AH et le registre d'état. `sahf` est la seconde moitié du `fstsw ax` / `sahf` par lequel on lit une comparaison du x87 avec un saut ordinaire.",
            en: "Move the low flags between AH and the flags register. `sahf` is the second half of the `fstsw ax` / `sahf` pair that reads an x87 comparison with an ordinary jump.",
        },
        flags: "CF PF AF ZF SF",
        example: "fstsw ax\nsahf\nja plus_grand",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["movsb", "movsw", "movsq"],
        title: Text {
            fr: "Copier un élément",
            en: "Copy one element",
        },
        body: Text {
            fr: "Copient un élément de [RSI] vers [RDI] et avancent les deux pointeurs — d'un, deux, quatre ou huit octets selon le suffixe, en arrière si DF est posé. Avec `rep` devant, c'est la copie de bloc du x86. Attention : `movsd` désigne aussi une instruction SSE, et c'est la présence d'opérandes qui les distingue.",
            en: "Copy one element from [RSI] to [RDI] and advance both pointers — by one, two, four or eight bytes depending on the suffix, backwards if DF is set. With `rep` in front, this is the x86's block copy. Careful: `movsd` is also an SSE instruction, and it is the operands that tell them apart.",
        },
        flags: "",
        example: "mov rcx, 100\nrep movsb           ; copier 100 octets",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["stosb", "stosw", "stosd", "stosq"],
        title: Text {
            fr: "Écrire un élément",
            en: "Store one element",
        },
        body: Text {
            fr: "Écrivent l'accumulateur en [RDI] et avancent RDI. Avec `rep`, c'est le remplissage d'une zone — l'équivalent de `memset`.",
            en: "Write the accumulator to [RDI] and advance RDI. With `rep`, this fills a region — the equivalent of `memset`.",
        },
        flags: "",
        example: "mov al, 0\nmov rcx, 64\nrep stosb           ; effacer 64 octets",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["lodsb", "lodsw", "lodsd", "lodsq"],
        title: Text {
            fr: "Lire un élément",
            en: "Load one element",
        },
        body: Text {
            fr: "Chargent [RSI] dans l'accumulateur et avancent RSI. `rep` n'a pas de sens devant elles : on relirait la même chose dans le même registre.",
            en: "Load [RSI] into the accumulator and advance RSI. `rep` makes no sense in front: you would reload into the same register.",
        },
        flags: "",
        example: "lodsb\ntest al, al",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["scasb", "scasw", "scasd", "scasq"],
        title: Text {
            fr: "Chercher un élément",
            en: "Scan for an element",
        },
        body: Text {
            fr: "Comparent l'accumulateur à [RDI] et avancent RDI. Avec `repne`, c'est la recherche d'un octet dans une zone — la boucle de `strlen` quand on cherche le zéro final.",
            en: "Compare the accumulator with [RDI] and advance RDI. With `repne`, this searches a region for a byte — `strlen`'s loop when looking for the final zero.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "mov al, 0\nmov rcx, 256\nrepne scasb",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cmpsb", "cmpsw", "cmpsd", "cmpsq"],
        title: Text {
            fr: "Comparer deux éléments",
            en: "Compare two elements",
        },
        body: Text {
            fr: "Comparent [RSI] à [RDI] et avancent les deux. Avec `repe`, c'est la comparaison de deux blocs : on s'arrête à la première différence.",
            en: "Compare [RSI] with [RDI] and advance both. With `repe`, this compares two blocks: it stops at the first difference.",
        },
        flags: "CF PF AF ZF SF OF",
        example: "mov rcx, 16\nrepe cmpsb\nje identiques",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["rep", "repe", "repz", "repne", "repnz"],
        title: Text {
            fr: "Répéter",
            en: "Repeat",
        },
        body: Text {
            fr: "Préfixes qui répètent l'instruction de chaîne qui suit tant que RCX n'est pas nul. `repe`/`repz` s'arrêtent en plus quand ZF tombe, `repne`/`repnz` quand il se lève. namsdec exécute un élément par pas, pour qu'on puisse regarder la boucle tourner.",
            en: "Prefixes that repeat the following string instruction while RCX is not zero. `repe`/`repz` also stop when ZF drops, `repne`/`repnz` when it rises. namsdec runs one element per step, so you can watch the loop go round.",
        },
        flags: "",
        example: "mov rcx, 100\nrep movsb",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["xlatb"],
        title: Text {
            fr: "Traduire par une table",
            en: "Translate through a table",
        },
        body: Text {
            fr: "Remplace AL par l'octet lu à [RBX + AL]. Une table de correspondance en une instruction — le chiffrement du pauvre, et la conversion de codage des années 80.",
            en: "Replaces AL with the byte at [RBX + AL]. A lookup table in one instruction — the poor man's cipher, and the 1980s way to convert encodings.",
        },
        flags: "",
        example: "mov rbx, table\nmov al, 2\nxlatb               ; al = table[2]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["lock"],
        title: Text {
            fr: "Rendre une opération indivisible",
            en: "Make an operation indivisible",
        },
        body: Text {
            fr: "Un préfixe, et non une instruction : il s'écrit devant `add`, `xchg`, `inc`, `cmpxchg` et quelques autres, et garantit que la lecture, le calcul et l'écriture se font d'un seul tenant. Sans lui, deux cœurs qui incrémentent le même compteur peuvent perdre une incrémentation. C'est la brique de tous les verrous.",
            en: "A prefix, not an instruction: it is written before `add`, `xchg`, `inc`, `cmpxchg` and a few others, and guarantees the read, the computation and the write happen as one. Without it, two cores incrementing the same counter can lose one increment. It is the brick every lock is built from.",
        },
        flags: "selon l'instruction préfixée",
        example: "lock inc qword [compteur]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cs", "ds", "es", "ss", "fs", "gs", "segment-prefix"],
        title: Text {
            fr: "Les préfixes de segment",
            en: "Segment prefixes",
        },
        body: Text {
            fr: "`fs:` et `gs:` changent la base à laquelle une adresse se rapporte. En 64 bits, `CS`, `DS`, `ES` et `SS` ont une base nulle — les écrire ne change rien —, mais `FS` et `GS` restent utilisés : Linux met sous `FS` les données propres au fil d'exécution, dont le canari de pile, et c'est pourquoi on lit `mov rax, [fs:0x28]` au début de tant de fonctions. namsdec ne simule pas cette base : il montre le préfixe et refuse l'accès plutôt que de rendre une adresse fausse.",
            en: "`fs:` and `gs:` change the base an address is relative to. In 64-bit mode `CS`, `DS`, `ES` and `SS` have a zero base — writing them changes nothing — but `FS` and `GS` remain in use: Linux puts thread-local data under `FS`, the stack canary among it, which is why `mov rax, [fs:0x28]` opens so many functions. namsdec does not simulate that base: it shows the prefix and refuses the access rather than returning a wrong address.",
        },
        flags: "aucun",
        example: "; namsdec ne les simule pas : mov rax, [fs:0x28]\n    nop",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["o16", "a32", "size-prefix"],
        title: Text {
            fr: "Les préfixes de taille",
            en: "Size prefixes",
        },
        body: Text {
            fr: "Deux octets qui changent la taille de ce qui suit : `66` rend les opérandes de 16 bits, `67` les adresses de 32 bits. Un désassembleur les montre seuls — `o16`, `a32` — quand ils précèdent quelque chose qu'il ne sait pas nommer, ou quand ils ne servent qu'à allonger une instruction pour l'aligner. C'est souvent ce dernier cas : un `nop` de sept octets n'est qu'un `nop` habillé de préfixes.",
            en: "Two bytes that change the size of what follows: `66` makes operands 16-bit, `67` makes addresses 32-bit. A disassembler shows them alone — `o16`, `a32` — when they precede something it cannot name, or when they only pad an instruction for alignment. That last case is the common one: a seven-byte `nop` is just a `nop` dressed in prefixes.",
        },
        flags: "aucun",
        example: "o16 nop            ; le même nop, un octet de plus",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["fbld", "fbstp"],
        title: Text {
            fr: "Le décimal codé binaire du 8087",
            en: "The 8087's packed decimal",
        },
        body: Text {
            fr: "`fbld` charge un nombre écrit un chiffre décimal par demi-octet — dix-huit chiffres et un signe, sur dix octets — et `fbstp` le réécrit ainsi. C'est l'héritage des machines comptables, où arrondir en binaire était inacceptable. On les rencontre encore dans du vieux code et dans les bibliothèques de conversion. namsdec ne les simule pas : il les nomme et les explique, comme le reste de ce qu'on lit sans l'exécuter.",
            en: "`fbld` loads a number written one decimal digit per nibble — eighteen digits and a sign, in ten bytes — and `fbstp` writes one back. It is the legacy of accounting machines, where rounding in binary was unacceptable. They still turn up in old code and in conversion libraries. namsdec does not simulate them: it names and explains them, like the rest of what one reads without running it.",
        },
        flags: "aucun",
        example: "; namsdec ne les simule pas : fbstp [resultat]\n    nop",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["pshuflw", "pshufhw", "vpshuflw", "vpshufhw"],
        title: Text {
            fr: "Remettre des mots dans l'ordre qu'on veut",
            en: "Put words in whatever order one wants",
        },
        body: Text {
            fr: "La même idée que `pshufd`, mais sur des mots de 16 bits : `pshuflw` réarrange les quatre mots bas d'un registre et laisse les hauts, `pshufhw` fait l'inverse. Le troisième opérande dit, deux bits par mot, d'où chacun vient. On les rencontre dans les routines de texte et de traitement d'images, là où l'ordre des octets est ce qu'on manipule.",
            en: "The same idea as `pshufd`, on 16-bit words: `pshuflw` rearranges the low four words of a register and leaves the high ones, `pshufhw` does the opposite. The third operand says, two bits per word, where each one comes from. They turn up in text and image routines, where the order of bytes is the thing being handled.",
        },
        flags: "aucun",
        example: "pshuflw xmm0, xmm1, 0x1b\n    pshufhw xmm2, xmm0, 0x39",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pextrw", "pinsrw", "pextrb", "pinsrb", "pextrd", "pextrq", "pinsrd", "pinsrq",
            "vpextrw", "vpinsrw", "vpextrb", "vpinsrb", "vpextrd", "vpextrq", "vpinsrd", "vpinsrq",
        ],
        title: Text {
            fr: "Prendre ou poser un seul élément d'un vecteur",
            en: "Take or put a single element of a vector",
        },
        body: Text {
            fr: "Un registre vectoriel tient huit mots ou seize octets ; ces instructions en lisent ou en écrivent **un seul**, désigné par un numéro écrit dans l'instruction. `pextrw` sort un mot vers un registre ordinaire, `pinsrw` en pose un. C'est ainsi qu'une routine SIMD rend son résultat au code qui l'appelle — après `pcmpeqb` et `pmovmskb`, par exemple.",
            en: "A vector register holds eight words or sixteen bytes; these read or write exactly **one** of them, picked by a number written into the instruction. `pextrw` moves a word out to an ordinary register, `pinsrw` puts one in. That is how a SIMD routine hands its result back to the code calling it — after `pcmpeqb` and `pmovmskb`, for instance.",
        },
        flags: "aucun",
        example: "pextrw eax, xmm0, 3\n    pinsrw xmm1, eax, 0",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "cvtdq2ps",
            "cvtdq2pd",
            "cvtps2dq",
            "cvttps2dq",
            "cvtpd2dq",
            "cvttpd2dq",
        ],
        title: Text {
            fr: "Convertir des paquets d'entiers et de flottants",
            en: "Convert packs of integers and floats",
        },
        body: Text {
            fr: "Les mêmes conversions que `cvtsi2ss` et compagnie, mais sur tout un vecteur à la fois : `dq` veut dire entiers de 32 bits, `ps` flottants simples, `pd` flottants doubles. Le `t` de `cvttps2dq` marque la troncature — on coupe la partie décimale — là où l'absence de `t` arrondit selon le mode en vigueur. C'est la différence entre `(int)x` en C et `lrint(x)`, et elle se paie en résultats qui diffèrent d'une unité.",
            en: "The same conversions as `cvtsi2ss` and friends, but on a whole vector at a time: `dq` means 32-bit integers, `ps` single floats, `pd` double floats. The `t` in `cvttps2dq` marks truncation — the fractional part is cut — where without it the current rounding mode applies. That is the difference between `(int)x` in C and `lrint(x)`, and it costs results that differ by one.",
        },
        flags: "aucun",
        example: "; namsdec ne les simule pas : cvttps2dq xmm0, xmm1\n    nop",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cpuid"],
        title: Text {
            fr: "Interroger le processeur",
            en: "Query the processor",
        },
        body: Text {
            fr: "Rend, selon la feuille demandée dans EAX, le nom du fabricant et la liste des extensions disponibles. namsdec se nomme et n'annonce aucune extension : il ne peut pas en promettre sans mentir.",
            en: "Returns, depending on the leaf requested in EAX, the vendor name and the list of available extensions. namsdec names itself and announces no extension: it cannot promise one without lying.",
        },
        flags: "",
        example: "xor eax, eax\ncpuid",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["rdtsc", "rdtscp"],
        title: Text {
            fr: "Lire l'horloge",
            en: "Read the clock",
        },
        body: Text {
            fr: "Rendent dans EDX:EAX le compteur de cycles du processeur. namsdec y met le nombre d'instructions exécutées : c'est le seul temps qui ait un sens ici, et il recule avec le pas arrière.",
            en: "Return the processor's cycle counter in EDX:EAX. namsdec puts the number of instructions executed there: the only time that means anything here, and it rewinds with the step-back.",
        },
        flags: "",
        example: "rdtsc\nmov r8, rax",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["fld", "fild"],
        title: Text {
            fr: "Empiler un nombre",
            en: "Push a number",
        },
        body: Text {
            fr: "Font descendre la pile d'un cran et écrivent la valeur au sommet : `fld` depuis un flottant (32, 64 ou 80 bits), `fild` depuis un entier. Tout le x87 travaille sur cette pile de huit cases, désignées par rapport au sommet.",
            en: "Lower the stack by one slot and write the value at the top: `fld` from a float (32, 64 or 80 bits), `fild` from an integer. All of the x87 works on this eight-slot stack, named relative to the top.",
        },
        flags: "",
        example: "fld qword [x]\nfild dword [n]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "fld1", "fldz", "fldpi", "fldl2e", "fldl2t", "fldlg2", "fldln2",
        ],
        title: Text {
            fr: "Empiler une constante",
            en: "Push a constant",
        },
        body: Text {
            fr: "Empilent une constante que le coprocesseur connaît par cœur : 1, 0, pi, et les logarithmes dont les fonctions transcendantes ont besoin. Plus précis, et plus court, que de les charger depuis la mémoire.",
            en: "Push a constant the coprocessor knows by heart: 1, 0, pi, and the logarithms the transcendental functions need. More precise, and shorter, than loading them from memory.",
        },
        flags: "",
        example: "fldpi\nfld1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["fst", "fstp", "fist", "fistp", "fisttp"],
        title: Text {
            fr: "Ranger le sommet",
            en: "Store the top",
        },
        body: Text {
            fr: "Écrivent le sommet en mémoire ou dans une autre case. Les formes en `p` dépilent ensuite — ce sont elles qu'on utilise presque toujours, sous peine de laisser la pile pleine. `fist` arrondit au plus proche, `fisttp` tranche vers zéro.",
            en: "Write the top to memory or to another slot. The `p` forms then pop — and those are the ones you nearly always want, or the stack fills up. `fist` rounds to nearest, `fisttp` truncates towards zero.",
        },
        flags: "",
        example: "fstp qword [résultat]\nfistp dword [entier]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["fxch"],
        title: Text {
            fr: "Échanger deux cases",
            en: "Exchange two slots",
        },
        body: Text {
            fr: "Échange le sommet avec une autre case. Sur une machine à pile, c'est ce qui remplace le choix d'un registre.",
            en: "Swaps the top with another slot. On a stack machine, this is what replaces choosing a register.",
        },
        flags: "",
        example: "fxch st1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "fadd", "faddp", "fiadd", "fsub", "fsubp", "fsubr", "fsubrp", "fisub", "fisubr",
            "fmul", "fmulp", "fimul", "fdiv", "fdivp", "fdivr", "fdivrp", "fidiv", "fidivr",
        ],
        title: Text {
            fr: "Calculer sur la pile",
            en: "Compute on the stack",
        },
        body: Text {
            fr: "Les quatre opérations. Les formes en `p` dépilent après avoir écrit — c'est ainsi qu'une pile se consomme. Les formes en `r` inversent les opérandes (`fsubr` fait source moins destination), les formes en `i` lisent un entier en mémoire.",
            en: "The four operations. The `p` forms pop after writing — that is how a stack gets consumed. The `r` forms reverse the operands (`fsubr` computes source minus destination), and the `i` forms read an integer from memory.",
        },
        flags: "",
        example: "fld qword [a]\nfld qword [b]\nfaddp               ; a + b, et la pile redescend",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "fchs", "fabs", "fsqrt", "frndint", "fscale", "fprem", "fprem1",
        ],
        title: Text {
            fr: "Transformer le sommet",
            en: "Transform the top",
        },
        body: Text {
            fr: "Changent le signe, prennent la valeur absolue, la racine, l'arrondi à l'entier, l'échelle par une puissance de deux, ou le reste d'une division. Le sommet est remplacé, la pile ne bouge pas.",
            en: "Change the sign, take the absolute value, the square root, the rounding to an integer, the scale by a power of two, or the remainder of a division. The top is replaced, the stack does not move.",
        },
        flags: "",
        example: "fld qword [x]\nfabs\nfsqrt",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "fsin", "fcos", "fsincos", "fptan", "fpatan", "fyl2x", "f2xm1",
        ],
        title: Text {
            fr: "Les fonctions transcendantes",
            en: "The transcendental functions",
        },
        body: Text {
            fr: "Sinus, cosinus, tangente, arc tangente, logarithme et exponentielle — le coprocesseur les calcule lui-même, ce qui était son grand intérêt en 1980. `fsincos` rend les deux d'un coup, `fptan` empile un 1 derrière le résultat, héritage de l'époque où elle rendait un rapport.",
            en: "Sine, cosine, tangent, arctangent, logarithm and exponential — the coprocessor computes them itself, which was its whole point in 1980. `fsincos` returns both at once, and `fptan` pushes a 1 behind its result, a leftover from when it returned a ratio.",
        },
        flags: "",
        example: "fldpi\nfld1\nfld1\nfaddp\nfdivp               ; pi/2\nfsin                ; 1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "fcom", "fcomp", "fcompp", "fucom", "fucomp", "fucompp", "ftst",
        ],
        title: Text {
            fr: "Comparer, à l'ancienne",
            en: "Compare, the old way",
        },
        body: Text {
            fr: "Comparent le sommet et rangent la réponse dans le mot d'état du coprocesseur, et non dans les indicateurs du processeur. Il faut donc l'aller chercher : `fstsw ax` puis `sahf`, et seulement ensuite un saut. `ftst` compare à zéro.",
            en: "Compare the top and put the answer in the coprocessor's status word, not in the processor's flags. So you have to go and fetch it: `fstsw ax`, then `sahf`, and only then a jump. `ftst` compares against zero.",
        },
        flags: "",
        example: "fcom st1\nfstsw ax\nsahf\nja plus_grand",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["fcomi", "fcomip", "fucomi", "fucomip"],
        title: Text {
            fr: "Comparer, directement",
            en: "Compare, directly",
        },
        body: Text {
            fr: "La même comparaison, mais écrite dans ZF, PF et CF du processeur : un saut ordinaire suit, sans passer par AX. Arrivées avec le Pentium Pro, elles rendent le `fstsw ax` inutile.",
            en: "The same comparison, but written into the processor's ZF, PF and CF: an ordinary jump follows, with no detour through AX. Introduced with the Pentium Pro, they make `fstsw ax` unnecessary.",
        },
        flags: "ZF PF CF",
        example: "fcomip st1\nja plus_grand",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["fxam"],
        title: Text {
            fr: "Examiner le sommet",
            en: "Examine the top",
        },
        body: Text {
            fr: "Range dans le mot d'état la *nature* du sommet — nombre ordinaire, zéro, infini, NaN, case vide — et son signe. La façon de distinguer un zéro d'une pile vide.",
            en: "Puts the *nature* of the top into the status word — ordinary number, zero, infinity, NaN, empty slot — and its sign. The way to tell a zero from an empty stack.",
        },
        flags: "",
        example: "fxam\nfstsw ax",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["fstsw", "fnstsw", "fstcw", "fnstcw", "fldcw"],
        title: Text {
            fr: "Le mot d'état, le mot de contrôle",
            en: "The status and control words",
        },
        body: Text {
            fr: "`fstsw` rend le mot d'état — où vivent les bits de condition et le numéro du sommet. `fstcw`/`fldcw` lisent et écrivent le mot de contrôle, qui fixe la précision (bits 8-9) et le mode d'arrondi (bits 10-11) ; il vaut 0x037f au repos, et `finit` le remet là. Dans namsdec, ce qu'on y pose compte : l'arrondi gouverne `fist`, `fistp` et `frndint`, et la précision à 24 bits arrondit chaque opération de base au format simple. Ce qui reste simplifié est recensé sous la notion « arrondi ».",
            en: "`fstsw` returns the status word — home of the condition bits and the top-of-stack index. `fstcw`/`fldcw` read and write the control word, which sets precision (bits 8-9) and rounding mode (bits 10-11); it reads 0x037f at rest, and `finit` puts it back there. In namsdec what you write there counts: the rounding mode governs `fist`, `fistp` and `frndint`, and 24-bit precision rounds every basic operation to single format. What remains simplified is listed under the «\u{a0}rounding\u{a0}» concept.",
        },
        flags: "",
        example: "fnstcw [ancien]      ; garder le réglage\n    fldcw [vers_zero]    ; 0x0f7f : arrondi vers zéro\n    fld qword [x]        ; 1,5\n    fistp qword [entier] ; 1 — au plus proche, ce serait 2\n    fldcw [ancien]       ; remettre comme on l'a trouvé",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "finit", "fninit", "ffree", "ffreep", "fincstp", "fdecstp", "fnop", "fwait", "wait",
        ],
        title: Text {
            fr: "Tenir la pile",
            en: "Housekeeping",
        },
        body: Text {
            fr: "Remettent le coprocesseur à neuf (`finit`), libèrent une case (`ffree`) ou font tourner l'anneau sans rien écrire (`fincstp`, `fdecstp`). `fwait` attendait le coprocesseur, du temps où il était une puce à part.",
            en: "Reset the coprocessor (`finit`), free a slot (`ffree`) or rotate the ring without writing (`fincstp`, `fdecstp`). `fwait` used to wait for the coprocessor, back when it was a separate chip.",
        },
        flags: "",
        example: "finit",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["movss", "vmovss", "vmovsd"],
        title: Text {
            fr: "Déplacer un seul flottant",
            en: "Move a single float",
        },
        body: Text {
            fr: "Copient un flottant — simple pour `ss`, double pour `sd` — sans toucher au reste du registre… sauf en venant de la mémoire, où tout le reste passe à zéro. Cette asymétrie est une source d'erreurs classique. La forme AVX à trois opérandes dit explicitement d'où vient le haut.",
            en: "Copy one float — single for `ss`, double for `sd` — without touching the rest of the register… except when coming from memory, where everything else is zeroed. That asymmetry is a classic source of bugs. The three-operand AVX form says explicitly where the upper part comes from.",
        },
        flags: "",
        example: "movsd xmm0, [x]     ; le haut de xmm0 passe à zéro\nmovsd xmm0, xmm1    ; le haut de xmm0 est préservé",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "movaps",
            "movapd",
            "movups",
            "movupd",
            "movdqa",
            "movdqu",
            "vmovaps",
            "vmovapd",
            "vmovups",
            "vmovupd",
            "vmovdqa",
            "vmovdqu",
            "vmovdqu8",
            "vmovdqu16",
            "vmovdqu32",
            "vmovdqu64",
            "vmovdqa32",
            "vmovdqa64",
        ],
        title: Text {
            fr: "Déplacer tout le registre",
            en: "Move the whole register",
        },
        body: Text {
            fr: "Copient 128 ou 256 bits d'un coup. Les formes en `a` exigent une adresse alignée sur la taille du transfert, les formes en `u` non — sur un processeur récent la différence de vitesse a disparu, l'exception d'alignement non. Le chiffre final des formes AVX-512 — `vmovdqu8`, `vmovdqu64` — ne dit ni la taille du transfert ni celle du registre : il dit ce qu'est un ÉLÉMENT pour le masque qui peut les accompagner. Sans masque, elles déplacent exactement ce que déplace `vmovdqu`.",
            en: "Copy 128 or 256 bits at once. The `a` forms require an address aligned to the transfer size, the `u` forms do not — on a recent processor the speed difference is gone, the alignment fault is not. The trailing number on the AVX-512 forms — `vmovdqu8`, `vmovdqu64` — gives neither the transfer size nor the register size: it says what an ELEMENT is for the mask that may come with them. Without a mask they move exactly what `vmovdqu` moves.",
        },
        flags: "",
        example: "movaps xmm0, [vecteur]\nvmovups ymm1, [non_aligné]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["movd", "movq", "vmovd", "vmovq"],
        title: Text {
            fr: "Passer d'un fichier à l'autre",
            en: "Cross between register files",
        },
        body: Text {
            fr: "Font passer 32 ou 64 bits entre un registre général et un registre vectoriel, dans un sens ou dans l'autre. Ce qui entre dans un registre vectoriel efface tout le reste.",
            en: "Move 32 or 64 bits between a general register and a vector register, in either direction. Whatever enters a vector register clears everything else.",
        },
        flags: "",
        example: "movq xmm0, rax\nmovd eax, xmm0",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "addss", "addsd", "subss", "subsd", "mulss", "mulsd", "divss", "divsd", "vaddss",
            "vaddsd", "vsubss", "vsubsd", "vmulss", "vmulsd", "vdivss", "vdivsd",
        ],
        title: Text {
            fr: "Calculer sur un flottant",
            en: "Compute on one float",
        },
        body: Text {
            fr: "Les quatre opérations sur le seul élément du bas ; le reste du registre ne bouge pas. C'est la forme « scalaire », celle qu'un compilateur produit pour un calcul ordinaire en double.",
            en: "The four operations on the bottom element only; the rest of the register does not move. This is the \"scalar\" form, the one a compiler emits for an ordinary calculation in double precision.",
        },
        flags: "",
        example: "movsd xmm0, [a]\naddsd xmm0, [b]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "addps", "addpd", "subps", "subpd", "mulps", "mulpd", "divps", "divpd", "vaddps",
            "vaddpd", "vsubps", "vsubpd", "vmulps", "vmulpd", "vdivps", "vdivpd",
        ],
        title: Text {
            fr: "Calculer sur tout le registre",
            en: "Compute on the whole register",
        },
        body: Text {
            fr: "Les mêmes opérations, sur tous les éléments à la fois : quatre flottants simples ou deux doubles par 128 bits, le double en AVX. C'est toute la raison d'être de ces registres.",
            en: "The same operations, on every element at once: four singles or two doubles per 128 bits, twice that in AVX. This is the whole point of these registers.",
        },
        flags: "",
        example: "vmovaps ymm0, [huit_valeurs]\nvaddps ymm0, ymm0, ymm1     ; huit additions",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "minss", "minsd", "maxss", "maxsd", "minps", "minpd", "maxps", "maxpd", "vminss",
            "vminsd", "vmaxss", "vmaxsd", "vminps", "vminpd", "vmaxps", "vmaxpd",
        ],
        title: Text {
            fr: "Le plus petit, le plus grand",
            en: "Minimum, maximum",
        },
        body: Text {
            fr: "Rendent le plus petit ou le plus grand des deux. Attention : dès que la comparaison est fausse — égalité, NaN —, c'est la *source* qui l'emporte. L'ordre des opérandes compte donc, contrairement à ce que le nom laisse croire.",
            en: "Return the smaller or the larger of the two. Careful: as soon as the comparison is false — equal values, a NaN — the *source* wins. So operand order matters, whatever the name suggests.",
        },
        flags: "",
        example: "maxsd xmm0, xmm1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "sqrtss", "sqrtsd", "sqrtps", "sqrtpd", "vsqrtss", "vsqrtsd", "vsqrtps", "vsqrtpd",
        ],
        title: Text {
            fr: "Racine carrée",
            en: "Square root",
        },
        body: Text {
            fr: "La racine carrée, d'un élément ou de tous. Le matériel la calcule directement, au bit près.",
            en: "The square root, of one element or of all of them. The hardware computes it directly, correctly rounded.",
        },
        flags: "",
        example: "sqrtsd xmm0, xmm1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "comiss", "comisd", "ucomiss", "ucomisd", "vcomiss", "vcomisd", "vucomiss", "vucomisd",
        ],
        title: Text {
            fr: "Comparer des flottants",
            en: "Compare floats",
        },
        body: Text {
            fr: "Comparent les éléments du bas et répondent dans ZF, PF et CF — et non dans SF et OF. C'est pourquoi le code flottant saute avec `ja` et `jb`, les tests *non signés* : après un `comisd`, `jg` ne veut rien dire. Un NaN pose les trois indicateurs d'un coup, ce qu'aucune comparaison entière ne produit.",
            en: "Compare the bottom elements and answer in ZF, PF and CF — not in SF and OF. Which is why floating-point code jumps with `ja` and `jb`, the *unsigned* tests: after a `comisd`, `jg` means nothing. A NaN sets all three flags at once, something no integer comparison ever does.",
        },
        flags: "ZF PF CF",
        example: "comisd xmm0, xmm1\nja plus_grand\njp non_comparable   ; un NaN",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "cmpps",
            "cmppd",
            "cmpss",
            "cmpeqsd",
            "cmpltsd",
            "cmplesd",
            "cmpunordsd",
            "cmpneqsd",
            "cmpnltsd",
            "cmpnlesd",
            "cmpordsd",
            "cmpeqss",
            "cmpltss",
            "cmpless",
            "cmpneqss",
            "cmpnltss",
            "cmpnless",
            "cmpeqps",
            "cmpltps",
            "cmpleps",
            "cmpneqps",
            "cmpeqpd",
            "cmpltpd",
            "cmplepd",
            "cmpneqpd",
            "cmpnltpd",
            "cmpnlepd",
            "vcmpps",
            "vcmppd",
            "vcmpss",
            "vcmpsd",
        ],
        title: Text {
            fr: "Comparer des flottants, en masque",
            en: "Compare floats, into a mask",
        },
        body: Text {
            fr: "L'autre façon de comparer : au lieu de poser des indicateurs comme `comisd`, celle-ci écrit le **résultat dans le registre** — que des uns si c'est vrai, que des zéros sinon. Un masque, donc, qu'on applique ensuite avec `andpd` ou `blendvpd` pour choisir sans brancher, sur plusieurs valeurs à la fois.\n\nLe prédicat est un octet immédiat (`cmppd xmm0, xmm1, 5`), mais l'assembleur accepte des noms : `cmpeq` égal, `cmplt` plus petit, `cmple` plus petit ou égal, `cmpneq` différent, `cmpnlt` **non** plus petit, `cmpord`/`cmpunord` comparable ou non. Le `n` change tout quand un NaN traîne : `nlt` n'est pas `ge`, puisqu'un NaN n'est ni plus petit ni plus grand.",
            en: "The other way to compare: instead of setting flags like `comisd`, this one writes the **result into the register** — all ones if true, all zeros otherwise. A mask, then, applied afterwards with `andpd` or `blendvpd` to choose without branching, on several values at once.\n\nThe predicate is an immediate byte (`cmppd xmm0, xmm1, 5`), but the assembler accepts names: `cmpeq` equal, `cmplt` less than, `cmple` less or equal, `cmpneq` not equal, `cmpnlt` **not** less than, `cmpord`/`cmpunord` comparable or not. That `n` matters as soon as a NaN is around: `nlt` is not `ge`, since a NaN is neither smaller nor larger.",
        },
        flags: "",
        example: "cmpltpd xmm0, xmm1   ; masque : xmm0 < xmm1 ?\nandpd xmm2, xmm0     ; ne garde que ce que le masque laisse",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cvtsi2ss", "cvtsi2sd", "vcvtsi2ss", "vcvtsi2sd"],
        title: Text {
            fr: "D'un entier vers un flottant",
            en: "Integer to float",
        },
        body: Text {
            fr: "Convertissent un entier signé, pris dans un registre général ou en mémoire, en flottant dans le bas du registre vectoriel.",
            en: "Convert a signed integer, taken from a general register or memory, into a float in the bottom of the vector register.",
        },
        flags: "",
        example: "mov rax, 5\ncvtsi2sd xmm0, rax",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "cvttss2si",
            "cvttsd2si",
            "cvtss2si",
            "cvtsd2si",
            "vcvttss2si",
            "vcvttsd2si",
            "vcvtss2si",
            "vcvtsd2si",
        ],
        title: Text {
            fr: "D'un flottant vers un entier",
            en: "Float to integer",
        },
        body: Text {
            fr: "Convertissent dans l'autre sens. Les formes en `tt` *tranchent* vers zéro — c'est ce que fait un cast en C —, les autres arrondissent au plus proche. Hors bornes ou sur un NaN, le résultat est « l'entier indéfini » : le plus petit négatif de la largeur demandée.",
            en: "Convert the other way. The `tt` forms *truncate* towards zero — what a C cast does — while the others round to nearest. Out of range, or on a NaN, the result is the \"integer indefinite\" value: the most negative number of the requested width.",
        },
        flags: "",
        example: "cvttsd2si rax, xmm0",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["cvtss2sd", "cvtsd2ss", "vcvtss2sd", "vcvtsd2ss"],
        title: Text {
            fr: "Changer de précision",
            en: "Change precision",
        },
        body: Text {
            fr: "Passent d'un flottant simple à un double et retour. La conversion vers le simple perd des chiffres : c'est le moment où un calcul « qui marchait » cesse de tomber juste.",
            en: "Move from single to double precision and back. The conversion down loses digits: the moment a calculation that \"worked\" stops coming out exact.",
        },
        flags: "",
        example: "cvtss2sd xmm0, xmm1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "xorps", "xorpd", "pxor", "andps", "andpd", "pand", "orps", "orpd", "por", "andnps",
            "andnpd", "pandn", "vxorps", "vxorpd", "vpxor", "vandps", "vandpd", "vpand", "vorps",
            "vorpd", "vpor", "vandnps", "vandnpd", "vpandn", "vpxord", "vpxorq", "vpandd",
            "vpandq", "vpord", "vporq", "vpandnd", "vpandnq",
        ],
        title: Text {
            fr: "Bit à bit sur un vecteur",
            en: "Bitwise on a vector",
        },
        body: Text {
            fr: "Les opérations logiques sur toute la largeur du registre. Elles servent autant à combiner des masques de comparaison qu'à manipuler le bit de signe d'un flottant — et `pxor xmm0, xmm0` est la façon habituelle de remettre un registre à zéro.",
            en: "The logical operations across the whole register. They serve as much to combine comparison masks as to flip a float's sign bit — and `pxor xmm0, xmm0` is the usual way to zero a register.",
        },
        flags: "",
        example: "pxor xmm0, xmm0     ; mise à zéro\nandps xmm1, [masque_signe]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "paddb", "paddw", "paddd", "paddq", "psubb", "psubw", "psubd", "psubq", "vpaddb",
            "vpaddw", "vpaddd", "vpaddq", "vpsubb", "vpsubw", "vpsubd", "vpsubq",
        ],
        title: Text {
            fr: "Additionner des entiers en parallèle",
            en: "Add integers in parallel",
        },
        body: Text {
            fr: "Additionnent ou soustraient élément par élément : seize octets, huit mots, quatre doubles mots ou deux quadruples mots d'un seul ordre. Les débordements ne se propagent pas d'un élément au suivant — chacun boucle chez lui.",
            en: "Add or subtract element by element: sixteen bytes, eight words, four doublewords or two quadwords in one instruction. Overflow does not carry from one element into the next — each one wraps on its own.",
        },
        flags: "",
        example: "paddb xmm0, xmm1    ; seize additions d'octets",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pcmpeqb", "pcmpeqw", "pcmpeqd", "pcmpgtb", "pcmpgtw", "pcmpgtd", "vpcmpeqb",
            "vpcmpeqw", "vpcmpeqd", "vpcmpgtb", "vpcmpgtw", "vpcmpgtd",
        ],
        title: Text {
            fr: "Comparer en parallèle",
            en: "Compare in parallel",
        },
        body: Text {
            fr: "Comparent élément par élément et rendent un *masque* : tout à 1 là où c'est vrai, tout à 0 ailleurs. Aucun indicateur n'est touché ; c'est ce masque qu'on combine ensuite avec `pand`, ou qu'on résume avec `pmovmskb`. `pcmpgt` compare en signé.",
            en: "Compare element by element and return a *mask*: all ones where true, all zeroes elsewhere. No flag is touched; the mask is what you then combine with `pand`, or summarise with `pmovmskb`. `pcmpgt` compares signed.",
        },
        flags: "",
        example: "pcmpeqb xmm0, xmm1\npmovmskb eax, xmm0",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["pcmpeqq", "pcmpgtq", "vpcmpeqq", "vpcmpgtq"],
        title: Text {
            fr: "Comparer deux nombres de 64 bits à la fois",
            en: "Compare two 64-bit numbers at a time",
        },
        body: Text {
            fr: "Les mêmes masques que `pcmpeqd`, sur des éléments deux fois plus larges. SSE4.1 a ajouté l'égalité, SSE4.2 la comparaison signée — le jeu s'est construit par couches, et la date d'une instruction se lit dans ce qui manque autour d'elle.",
            en: "The same masks as `pcmpeqd`, on elements twice as wide. SSE4.1 added equality, SSE4.2 the signed comparison — the instruction set was built in layers, and an instruction's age shows in what is missing around it.",
        },
        flags: "",
        example: "pcmpeqq xmm0, xmm1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pminub", "pmaxub", "pminsb", "pmaxsb", "pminuw", "pmaxuw", "pminsw", "pmaxsw",
            "pminud", "pmaxud", "pminsd", "pmaxsd", "vpminub", "vpmaxub", "vpminsb", "vpmaxsb",
            "vpminuw", "vpmaxuw", "vpminsw", "vpmaxsw", "vpminud", "vpmaxud", "vpminsd", "vpmaxsd",
        ],
        title: Text {
            fr: "Le plus petit, le plus grand, seize à la fois",
            en: "The smallest, the largest, sixteen at a time",
        },
        body: Text {
            fr: "Rendent élément par élément le plus petit — ou le plus grand — des deux. Le signe change tout : `0x80` vaut 128 pour `pmaxub` et −128 pour `pmaxsb`, si bien que les deux rendent des vecteurs différents sur les mêmes octets.\n\nOn les rencontre bien au-delà du calcul : `pminub` contre un vecteur nul, suivi de `pcmpeqb`, est la façon dont la bibliothèque C cherche la fin d'une chaîne seize octets d'un coup. C'est la raison pour laquelle elles apparaissent dans à peu près tout binaire compilé, même sans une seule ligne de code vectoriel écrite à la main.",
            en: "Return, element by element, the smaller — or the larger — of two. Sign changes everything: `0x80` is 128 to `pmaxub` and −128 to `pmaxsb`, so the two return different vectors from the same bytes.\n\nThey reach well beyond arithmetic: `pminub` against a zero vector, followed by `pcmpeqb`, is how the C library finds the end of a string sixteen bytes at a time. That is why they appear in nearly every compiled binary, without a single hand-written line of vector code.",
        },
        flags: "",
        example: "pminub xmm0, xmm1   ; le plus petit octet de chaque paire",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["vpternlogd", "vpternlogq"],
        title: Text {
            fr: "Une table de vérité, en une instruction",
            en: "A truth table, in one instruction",
        },
        body: Text {
            fr: "Rien n'y ressemble ailleurs dans le jeu. Pour chaque position de bit, elle prend le bit de la destination, celui de la première source et celui de la seconde — trois bits, donc huit combinaisons. L'immédiat de huit bits donne le résultat de chacune : son bit numéro `abc` dit ce que vaut la sortie quand les trois entrées valent `a`, `b`, `c`.\n\nAutrement dit, **l'immédiat EST la fonction booléenne**. N'importe quel opérateur à trois entrées s'écrit ainsi, et c'est pourquoi un compilateur peut remplacer trois instructions logiques par celle-ci.\n\nPour composer son immédiat, on part de trois constantes : `0xf0` ne rend que la destination, `0xcc` que la première source, `0xaa` que la seconde. On les combine ensuite avec les opérateurs qu'on veut — `0xf0 ^ 0xcc ^ 0xaa` donne `0x96`, le ou-exclusif des trois. `0xca` est un multiplexeur : si la destination, alors la première source, sinon la seconde.\n\nNoter que la destination est **lue** avant d'être écrite, ce qui est rare pour une instruction vectorielle.",
            en: "Nothing else in the instruction set resembles it. For each bit position it takes the destination bit, the first source bit and the second — three bits, so eight combinations. The eight-bit immediate gives the result of each: its bit number `abc` says what the output is when the three inputs are `a`, `b`, `c`.\n\nIn other words, **the immediate IS the boolean function**. Any three-input operator can be written this way, which is why a compiler can replace three logical instructions with this one.\n\nTo build the immediate, start from three constants: `0xf0` returns only the destination, `0xcc` only the first source, `0xaa` only the second. Then combine them with the operators you want — `0xf0 ^ 0xcc ^ 0xaa` gives `0x96`, the exclusive-or of all three. `0xca` is a multiplexer: if the destination, then the first source, else the second.\n\nNote that the destination is **read** before being written, which is rare for a vector instruction.",
        },
        flags: "",
        example: "vpternlogd zmm0, zmm1, zmm2, 0x96   ; xor des trois\nvpternlogd zmm0, zmm1, zmm2, 0xca   ; un multiplexeur",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["vpermb", "vpermd", "vpermw"],
        title: Text {
            fr: "Permuter selon un index calculé",
            en: "Permute by a computed index",
        },
        body: Text {
            fr: "Chaque voie du résultat va chercher la voie que **désigne** l'index de même rang. Avec un registre d'index, l'ordre n'est pas figé dans l'instruction : il peut être calculé, ce qui fait de l'opération une **table de consultation** à seize entrées en un seul coup.\n\nC'est la différence avec les permutations à immédiat (`vpermq zmm, zmm, 0x1b`), figées à la compilation et répétées sur chaque voie de 256 bits — huit bits ne peuvent pas nommer huit positions parmi huit. Les deux formes portent le même nom : seul le type du troisième opérande les distingue, et les confondre donne un résultat qui a l'air plausible.\n\nAttention à l'ordre : l'index est la **première** source, la donnée la seconde — l'inverse de ce qu'on lit de gauche à droite.",
            en: "Each lane of the result fetches the lane **named** by the index of the same rank. With an index register the order is not frozen in the instruction: it can be computed, which makes the operation a **lookup table** of sixteen entries in one go.\n\nThat is the difference from the immediate forms (`vpermq zmm, zmm, 0x1b`), frozen at compile time and repeated on each 256-bit lane — eight bits cannot name eight positions out of eight. Both forms share a name: only the third operand's type tells them apart, and confusing them yields a plausible-looking result.\n\nMind the order: the index is the **first** source, the data the second — the opposite of the reading order.",
        },
        flags: "",
        example: "vpermd zmm0, zmm1, zmm2   ; zmm1 dit d'où vient chaque mot",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["valignd", "valignq"],
        title: Text {
            fr: "Une fenêtre à cheval sur deux vecteurs",
            en: "A window straddling two vectors",
        },
        body: Text {
            fr: "On met les deux sources bout à bout — la seconde en bas, la première en haut — et l'on prend la tranche qui commence `imm` mots plus loin. C'est le `palignr` de SSE, mais par mots de 32 ou 64 bits et sur toute la largeur du registre au lieu d'une voie de 128.\n\nL'usage : faire **glisser une fenêtre** sur un flux sans relire la mémoire. Un filtre ou une convolution a besoin, à chaque pas, des mêmes données décalées d'un cran ; `valignd` les fabrique à partir des deux vecteurs déjà chargés.",
            en: "The two sources are placed end to end — the second low, the first high — and the slice starting `imm` words further along is taken. It is SSE's `palignr`, but by 32- or 64-bit words and across the whole register instead of a 128-bit lane.\n\nThe use: **sliding a window** over a stream without rereading memory. A filter or a convolution needs, at each step, the same data shifted by one; `valignd` builds it from the two vectors already loaded.",
        },
        flags: "",
        example: "valignd zmm0, zmm1, zmm2, 1   ; la fenêtre avance d'un mot",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["vshufi32x4", "vshufi64x2", "vshuff32x4", "vshuff64x2"],
        title: Text {
            fr: "Choisir des tranches de 128 bits",
            en: "Pick 128-bit slices",
        },
        body: Text {
            fr: "AVX-512 travaille à trois échelles : le bit, la voie de 32 ou 64 bits, et la **tranche de 128**. Celle-ci est à la troisième : deux bits d'immédiat par tranche du résultat. Les deux tranches basses viennent de la première source, les deux hautes de la seconde.\n\nC'est l'outil des transpositions et des réarrangements en gros blocs, là où `vpermd` travaillerait mot par mot pour le même effet, en plus cher.",
            en: "AVX-512 works at three scales: the bit, the 32- or 64-bit lane, and the **128-bit slice**. This one is at the third: two immediate bits per result slice. The two low slices come from the first source, the two high ones from the second.\n\nIt is the tool for transposes and coarse rearrangements, where `vpermd` would work word by word for the same effect, more expensively.",
        },
        flags: "",
        example: "vshufi32x4 zmm0, zmm1, zmm2, 0x4e   ; les tranches échangées",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "vpsllvd", "vpsllvq", "vpsrlvd", "vpsrlvq", "vpsravd", "vpsravq",
        ],
        title: Text {
            fr: "Décaler chaque voie de son propre compte",
            en: "Shift each lane by its own count",
        },
        body: Text {
            fr: "`vpsrad` décale toutes les voies d'autant ; ici le second vecteur porte **un compte par voie**. On traite donc seize champs de largeurs différentes à la fois, ce qui est l'opération centrale du décodage de formats compactés.\n\nUn piège : un compte supérieur ou égal à la largeur donne **zéro** pour les décalages logiques, et le **signe répété** pour l'arithmétique. Ce n'est pas une troncature du compte, contrairement aux décalages scalaires qui le prennent modulo la largeur — `shl rax, 64` ne décale pas du tout, `vpsllvd` avec un compte de 64 donne zéro.",
            en: "`vpsrad` shifts every lane by the same amount; here the second vector carries **one count per lane**. Sixteen fields of different widths are handled at once, which is the core operation of unpacking bit-packed formats.\n\nA trap: a count greater than or equal to the width gives **zero** for logical shifts, and the **repeated sign** for arithmetic ones. It is not a truncation of the count, unlike scalar shifts which take it modulo the width — `shl rax, 64` does not shift at all, while `vpsllvd` with a count of 64 gives zero.",
        },
        flags: "",
        example: "vpsravd zmm0, zmm1, zmm2   ; chaque mot a son décalage",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "vplzcntd", "vplzcntq", "vpopcntb", "vpopcntw", "vpopcntd", "vpopcntq",
        ],
        title: Text {
            fr: "Compter les bits, voie par voie",
            en: "Count bits, lane by lane",
        },
        body: Text {
            fr: "`vplzcnt` compte les zéros **de tête** de chaque voie, `vpopcnt` les bits **à un**. Ce que `lzcnt` et `popcnt` font pour un registre, seize fois d'un coup.\n\nUne voie nulle rend la largeur entière pour `vplzcnt` — trente-deux pour un mot de 32 bits —, ce qui est la réponse juste et non un cas limite : il y a bien trente-deux zéros devant le premier un qui n'existe pas.\n\n`vplzcnt` sert à normaliser des flottants à la main et à calculer des logarithmes entiers ; `vpopcnt` à mesurer des distances de Hamming et à compter des correspondances dans un masque.",
            en: "`vplzcnt` counts each lane's **leading** zeros, `vpopcnt` its **set** bits. What `lzcnt` and `popcnt` do for one register, sixteen times at once.\n\nA zero lane yields the full width for `vplzcnt` — thirty-two for a 32-bit word — which is the right answer and not an edge case: there really are thirty-two zeros before the first one that does not exist.\n\n`vplzcnt` serves to normalise floats by hand and to compute integer logarithms; `vpopcnt` to measure Hamming distances and count matches in a mask.",
        },
        flags: "",
        example: "vpopcntd zmm0, zmm1   ; combien de bits à un, par mot",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["vpconflictd", "vpconflictq"],
        title: Text {
            fr: "Où sont les collisions",
            en: "Where the collisions are",
        },
        body: Text {
            fr: "Instruction singulière, qui ne sert qu'à une chose — mais elle la résout entièrement.\n\nQuand on veut écrire seize valeurs à seize adresses calculées (une **dispersion**), deux adresses égales se marchent dessus : la seconde écriture écrase la première, et le résultat dépend de l'ordre. `vpconflict` dit **d'avance** où sont les doublons, et le code peut alors les traiter en plusieurs passes au lieu de renoncer à vectoriser.\n\nChaque voie reçoit un masque de bits : le bit `j` est mis si la voie `j`, **plus basse que celle-ci**, porte la même valeur. La voie 0 reçoit donc toujours zéro — rien ne la précède.",
            en: "A singular instruction, serving only one purpose — but solving it entirely.\n\nWhen writing sixteen values to sixteen computed addresses (a **scatter**), two equal addresses collide: the second write overwrites the first, and the result depends on ordering. `vpconflict` says **in advance** where the duplicates are, so the code can handle them in several passes instead of giving up on vectorising.\n\nEach lane receives a bitmask: bit `j` is set if lane `j`, **lower than this one**, holds the same value. Lane 0 therefore always receives zero — nothing precedes it.",
        },
        flags: "",
        example: "vpconflictd zmm0, zmm1   ; les doublons, avant d'écrire",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["pblendw", "vpblendw"],
        title: Text {
            fr: "Mélanger deux vecteurs, mot par mot",
            en: "Blend two vectors, word by word",
        },
        body: Text {
            fr: "Chaque mot de seize bits du résultat vient de la **première** source si le bit correspondant de l'immédiat est à zéro, de la **seconde** s'il est à un. Huit bits, huit mots — et c'est là qu'est le piège.\n\nSur 256 bits il y a seize mots pour huit bits de contrôle : le motif **se répète** sur chaque moitié de 128 bits. Ce n'est pas une bizarrerie gratuite, c'est l'histoire : l'instruction vient de SSE, où le registre n'avait qu'une moitié, et AVX l'a élargi sans élargir l'immédiat.\n\nÀ retenir aussi : zéro prend la première source, un prend la seconde — l'inverse de ce que l'intuition souffle.",
            en: "Each sixteen-bit word of the result comes from the **first** source when the matching immediate bit is zero, from the **second** when it is one. Eight bits, eight words — and that is where the trap is.\n\nOver 256 bits there are sixteen words for eight control bits: the pattern **repeats** on each 128-bit half. Not an arbitrary quirk but history: the instruction comes from SSE, where the register had only one half, and AVX widened it without widening the immediate.\n\nAlso worth remembering: zero takes the first source, one takes the second — the opposite of what intuition suggests.",
        },
        flags: "",
        example: "vpblendw ymm0, ymm1, ymm2, 0b10101010\n; un mot sur deux vient de ymm2, et le motif\n; repart à l'identique au neuvième",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["pshufb", "vpshufb"],
        title: Text {
            fr: "Choisir ses octets un par un",
            en: "Pick one's bytes one by one",
        },
        body: Text {
            fr: "Chaque octet du résultat est l'octet de la source que **désigne** l'octet de même rang du contrôle — et il est nul si le bit de poids fort de ce contrôle-là est mis. Seize recherches dans une table de seize entrées, en une instruction.\n\nC'est l'outil des conversions : hexadécimal, base64, UTF-8, et tout ce qui transforme un octet en un autre selon une table. Contrairement à `pshufd`, l'ordre n'est pas gravé dans un immédiat : il est dans un registre, donc il peut être calculé.",
            en: "Each byte of the result is the byte of the source **named** by the control byte of the same rank — and it is zero if that control byte's top bit is set. Sixteen lookups in a sixteen-entry table, in one instruction.\n\nIt is the conversion tool: hexadecimal, base64, UTF-8, anything mapping a byte to another through a table. Unlike `pshufd`, the order is not frozen in an immediate: it sits in a register, so it can be computed.",
        },
        flags: "",
        example: "pshufb xmm0, xmm1   ; xmm1 dit d'où vient chaque octet",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["palignr", "vpalignr"],
        title: Text {
            fr: "Une fenêtre à cheval sur deux vecteurs",
            en: "A window straddling two vectors",
        },
        body: Text {
            fr: "Met les deux sources bout à bout — la première en haut —, décale le tout du nombre d'octets donné, et garde les seize du bas. Elle donne donc une fenêtre de seize octets qui commence où l'on veut dans trente-deux.\n\nC'est ainsi qu'un `memcpy` traite une adresse qui ne tombe pas rond : deux lectures alignées, et `palignr` recompose entre les deux, sans jamais faire de lecture non alignée.",
            en: "Puts the two sources end to end — the first on top —, shifts the whole by the given number of bytes, and keeps the low sixteen. It yields a sixteen-byte window starting wherever one wants within thirty-two.\n\nThat is how a `memcpy` handles an address that does not fall round: two aligned loads, and `palignr` recomposes between them, never making an unaligned read.",
        },
        flags: "",
        example: "palignr xmm0, xmm1, 4",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pavgb", "pavgw", "psadbw", "pabsb", "pabsw", "pabsd", "vpavgb", "vpavgw", "vpsadbw",
            "vpabsb", "vpabsw", "vpabsd",
        ],
        title: Text {
            fr: "Moyenne, écart, valeur absolue",
            en: "Average, difference, absolute value",
        },
        body: Text {
            fr: "Trois mesures que le jeu rend en une instruction parce que l'image et le son les demandent partout. `pavgb` fait la moyenne arrondie au supérieur, sans débordement possible. `psadbw` additionne les écarts absolus de huit octets et range le total dans un seul nombre — une mesure de ressemblance entre deux blocs, et aussi la façon la plus courte de compter les octets d'un masque. `pabsb` prend la valeur absolue ; la plus petite valeur négative n'a pas d'opposé qui tienne dans sa voie, et reste elle-même.",
            en: "Three measures the instruction set gives in one step, because image and sound ask for them everywhere. `pavgb` averages, rounding up, with no overflow possible. `psadbw` sums the absolute differences of eight bytes into a single number — a likeness measure between two blocks, and also the shortest way to count the bytes of a mask. `pabsb` takes the absolute value; the most negative value has no opposite fitting its lane, and stays itself.",
        },
        flags: "",
        example: "psadbw xmm0, xmm1   ; la somme des huit écarts",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "kmovb", "kmovw", "kmovd", "kmovq", "kandb", "kandw", "kandd", "kandq", "korb", "korw",
            "kord", "korq", "kxorb", "kxorw", "kxord", "kxorq", "kandnb", "kandnw", "kandnd",
            "kandnq", "kxnorb", "kxnorw", "kxnord", "kxnorq", "knotb", "knotw", "knotd", "knotq",
            "kaddb", "kaddw", "kaddd", "kaddq", "kshiftlb", "kshiftlw", "kshiftld", "kshiftlq",
            "kshiftrb", "kshiftrw", "kshiftrd", "kshiftrq", "kortestb", "kortestw", "kortestd",
            "kortestq", "ktestb", "ktestw", "ktestd", "ktestq", "kunpckbw", "kunpckwd", "kunpckdq",
        ],
        title: Text {
            fr: "Les registres de masque d'AVX-512",
            en: "The AVX-512 mask registers",
        },
        body: Text {
            fr: "Huit registres — `k0` à `k7` — qui portent **un bit par voie** du vecteur. C'est la grande idée d'AVX-512 : là où SSE et AVX écrivaient un masque dans un registre vectoriel (tout à 1 là où c'est vrai) et le combinaient avec `pand`, AVX-512 le range à part, en bits, et chaque instruction peut s'y référer pour ne travailler que sur les voies retenues.\n\n`kmov` est le pont : une comparaison écrit `k1`, `kmovd eax, k1` le ramène dans un registre ordinaire, et le code branche dessus. C'est l'instruction AVX-512 la plus fréquente dans un vrai binaire. `kortest` et `ktest` posent les indicateurs sans rien écrire — `kortestd k1, k1` suivi d'un `jz` est la façon dont une boucle demande « rien trouvé dans ces soixante-quatre octets ? ».\n\nLa lettre finale donne la largeur : `b` huit bits, `w` seize, `d` trente-deux, `q` soixante-quatre. Elle **borne** le résultat : `knotb` ne retourne que huit bits.\n\n`k0` a une particularité : employé comme masque d'une autre instruction, il veut dire « aucun masque, toutes les voies ». Écrit ou lu explicitement, c'est un registre comme les autres.",
            en: "Eight registers — `k0` to `k7` — holding **one bit per lane** of the vector. That is the big idea of AVX-512: where SSE and AVX wrote a mask into a vector register (all ones where true) and combined it with `pand`, AVX-512 keeps it apart, in bits, and any instruction can refer to it to work only on the selected lanes.\n\n`kmov` is the bridge: a comparison writes `k1`, `kmovd eax, k1` brings it back into an ordinary register, and the code branches on it. It is the most frequent AVX-512 instruction in a real binary. `kortest` and `ktest` set the flags without writing anything — `kortestd k1, k1` followed by `jz` is how a loop asks \"nothing found in these sixty-four bytes?\".\n\nThe trailing letter gives the width: `b` eight bits, `w` sixteen, `d` thirty-two, `q` sixty-four. It **bounds** the result: `knotb` flips eight bits only.\n\n`k0` is special: used as another instruction's mask it means \"no mask, every lane\". Written or read explicitly, it is a register like the others.",
        },
        flags: "ZF CF",
        example: "vptestmb k1, ymm0, ymm1\nkmovd eax, k1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "vptestmb",
            "vptestmw",
            "vptestmd",
            "vptestmq",
            "vptestnmb",
            "vptestnmw",
            "vptestnmd",
            "vptestnmq",
            "vpcmpb",
            "vpcmpw",
            "vpcmpd",
            "vpcmpq",
            "vpcmpub",
            "vpcmpuw",
            "vpcmpud",
            "vpcmpuq",
        ],
        title: Text {
            fr: "Comparer dans un masque",
            en: "Compare into a mask",
        },
        body: Text {
            fr: "Ce qui REMPLIT un registre de masque. `vptestmb k1, ymm2, ymm3` pose un bit par octet, là où le ET des deux n'est pas nul ; la forme `nm` fait l'inverse. Elles remplacent à elles seules le couple `pcmpeqb` + `pmovmskb` des jeux précédents.\n\n`vpcmp*` va plus loin : les jeux d'avant n'avaient que « égal » et « plus grand », et l'on composait le reste à la main. Ici le **prédicat** est un immédiat — 0 égal, 1 plus petit, 2 plus petit ou égal, 4 différent, 5 pas plus petit, 6 plus grand —, et le `u` du nom dit que les éléments sont lus sans signe. Un désassembleur écrit souvent l'alias plutôt que l'immédiat : `vpcmpltub` est `vpcmpub …, 1`.",
            en: "What FILLS a mask register. `vptestmb k1, ymm2, ymm3` sets one bit per byte where the AND of the two is non-zero; the `nm` form does the opposite. On their own they replace the `pcmpeqb` + `pmovmskb` pair of earlier sets.\n\n`vpcmp*` goes further: earlier sets only had \"equal\" and \"greater\", and the rest was composed by hand. Here the **predicate** is an immediate — 0 equal, 1 less, 2 less or equal, 4 not equal, 5 not less, 6 greater — and the `u` in the name says elements are read unsigned. A disassembler often writes the alias rather than the immediate: `vpcmpltub` is `vpcmpub …, 1`.",
        },
        flags: "",
        example: "vpcmpub k1, ymm0, ymm1, 1   ; les octets plus petits",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pcmpistri",
            "pcmpistrm",
            "pcmpestri",
            "pcmpestrm",
            "vpcmpistri",
            "vpcmpistrm",
            "vpcmpestri",
            "vpcmpestrm",
        ],
        title: Text {
            fr: "Une boucle de chaîne en une instruction",
            en: "A whole string loop in one instruction",
        },
        body: Text {
            fr: "Comparent seize octets à seize autres selon l'une de **quatre questions**, et disent où. C'est ainsi que la bibliothèque C fait `strlen`, `strchr`, `strcmp` et `strstr` — d'où leur présence dans à peu près tout binaire, et l'intérêt de savoir les lire.\n\nTout est dans l'immédiat, et c'est l'instruction la plus paramétrée du jeu. Les bits 3 et 2 posent la QUESTION : `00` « cet élément est-il dans l'ensemble ? » (`strchr`), `01` « est-il dans l'un de ces intervalles ? » (une classe de caractères), `10` « les deux chaînes sont-elles égales, élément par élément ? » (`strcmp`, et `strlen` en comparant une chaîne à elle-même), `11` « la première apparaît-elle ici dans la seconde ? » (`strstr`). Les bits 5 et 4 donnent la polarité — `11` retourne la réponse **seulement là où la chaîne est valide**, et c'est celle qui sert à `strlen`. Les bits 1 et 0 disent la largeur et le signe des éléments ; le bit 6, la forme de la sortie.\n\n`i` veut dire *implicit* : les longueurs se lisent dans les données, une chaîne s'arrêtant à son premier élément nul. `e` veut dire *explicit* : elles arrivent dans EAX et EDX. Les formes en `i` rendent un **index** dans ECX — et **seize** quand il n'y a rien, ce qui fait sortir de la boucle ; celles en `m` rendent un **masque** dans XMM0, registre qu'elles écrasent sans le nommer.",
            en: "Compare sixteen bytes against sixteen others under one of **four questions**, and say where. This is how the C library does `strlen`, `strchr`, `strcmp` and `strstr` — hence their presence in nearly every binary, and the value of being able to read them.\n\nEverything is in the immediate, and it is the most parameterised instruction of the set. Bits 3 and 2 set the QUESTION: `00` \"is this element in the set?\" (`strchr`), `01` \"is it within one of these ranges?\" (a character class), `10` \"are the two strings equal, element by element?\" (`strcmp`, and `strlen` by comparing a string with itself), `11` \"does the first appear here in the second?\" (`strstr`). Bits 5 and 4 give polarity — `11` flips the answer **only where the string is valid**, and that is the one `strlen` uses. Bits 1 and 0 give element width and sign; bit 6, the shape of the output.\n\n`i` means *implicit*: lengths are read from the data, a string ending at its first null element. `e` means *explicit*: they arrive in EAX and EDX. The `i` forms return an **index** in ECX — and **sixteen** when there is nothing, which is what breaks the loop; the `m` forms return a **mask** in XMM0, a register they clobber without naming it.",
        },
        flags: "CF ZF SF OF",
        example: "pcmpistri xmm0, xmm0, 0x38  ; la longueur de la chaîne",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["ptest", "vptest"],
        title: Text {
            fr: "Deux questions sur tout un vecteur",
            en: "Two questions about a whole vector",
        },
        body: Text {
            fr: "La seule instruction vectorielle entière qui écrive des indicateurs. ZF dit que la seconde source n'a **aucun** bit en commun avec la première ; CF, qu'elle n'en a aucun **hors** de la première.\n\nAprès une comparaison vectorielle, c'est ainsi qu'on demande « aucune voie ne correspond » (`jz`) et « toutes correspondent » (`jc`) sans jamais ramener le vecteur dans un registre général — là où un `pmovmskb` suivi d'un `test` coûtait deux instructions et un aller-retour entre deux fichiers de registres.",
            en: "The only packed-integer instruction that writes flags. ZF says the second source has **no** bit in common with the first; CF, that it has none **outside** the first.\n\nAfter a vector comparison, that is how one asks \"no lane matches\" (`jz`) and \"all match\" (`jc`) without ever bringing the vector back into a general register — where a `pmovmskb` plus a `test` cost two instructions and a trip between two register files.",
        },
        flags: "ZF CF",
        example: "ptest xmm0, xmm0\njz nul",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "movntdq",
            "movntdqa",
            "movntps",
            "movntpd",
            "movnti",
            "vmovntdq",
            "vmovntdqa",
            "vmovntps",
            "vmovntpd",
        ],
        title: Text {
            fr: "Écrire sans salir le cache",
            en: "Write without dirtying the cache",
        },
        body: Text {
            fr: "« Non temporel » veut dire : je ne relirai pas ceci de sitôt. Le processeur écrit alors sans garder la donnée en cache, et sans la relire d'abord — ce qui, sur un gros tampon, évite d'évincer tout ce qui servait encore. C'est le choix que fait un `memset` dès que la zone dépasse la taille du cache.\n\nÀ l'émulation, il n'y a pas de cache : ce sont des déplacements comme les autres, et namsdec les exécute comme tels. La différence est réelle sur la machine, invisible dans les registres.",
            en: "\"Non-temporal\" means: I will not read this again soon. The processor writes without keeping the data in cache, and without reading it first — which, on a large buffer, avoids evicting everything still in use. It is the choice a `memset` makes as soon as the area outgrows the cache.\n\nUnder emulation there is no cache: these are ordinary moves, and namsdec runs them as such. The difference is real on the machine, invisible in the registers.",
        },
        flags: "",
        example: "movntdq [rdi], xmm0",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "psllw", "pslld", "psllq", "psrlw", "psrld", "psrlq", "psraw", "psrad", "vpsllw",
            "vpslld", "vpsllq", "vpsrlw", "vpsrld", "vpsrlq", "vpsraw", "vpsrad",
        ],
        title: Text {
            fr: "Décaler en parallèle",
            en: "Shift in parallel",
        },
        body: Text {
            fr: "Décalent chaque élément du même nombre de bits. Au-delà de la largeur d'un élément le résultat est nul — et non « ce qui reste », comme sur un registre général.",
            en: "Shift every element by the same number of bits. Beyond an element's width the result is zero — not \"whatever is left\", as on a general register.",
        },
        flags: "",
        example: "psllq xmm0, 4",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["pslldq", "psrldq", "vpslldq", "vpsrldq"],
        title: Text {
            fr: "Décaler par octets",
            en: "Shift by bytes",
        },
        body: Text {
            fr: "Décalent le registre entier d'un nombre d'*octets*, et non de bits. En AVX, chaque moitié de 128 bits pour son compte : la largeur a doublé, pas le décalage.",
            en: "Shift the whole register by a number of *bytes*, not bits. In AVX, each 128-bit half on its own: the width doubled, the shift did not.",
        },
        flags: "",
        example: "pslldq xmm0, 4      ; tout remonte de quatre octets",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "paddsb", "paddsw", "paddusb", "paddusw", "psubsb", "psubsw", "psubusb", "psubusw",
            "vpaddsb", "vpaddsw", "vpaddusb", "vpaddusw", "vpsubsb", "vpsubsw", "vpsubusb",
            "vpsubusw",
        ],
        title: Text {
            fr: "Additionner sans déborder",
            en: "Add without wrapping",
        },
        body: Text {
            fr: "Additionnent ou soustraient élément par élément, mais **saturent** au lieu de boucler : 250 + 10 sur un octet non signé donne 255, pas 4. C'est la différence entre un pixel qui devient blanc et un pixel qui devient noir — et la raison pour laquelle ces instructions peuplent le traitement d'image et de son. Le `s` central dit signé, le `us` non signé. namsdec les exécute : comparez `paddb` et `paddusb` sur les mêmes octets, la différence se voit dans le registre.",
            en: "Add or subtract element by element, but **saturate** instead of wrapping: 250 + 10 on an unsigned byte gives 255, not 4. That is the difference between a pixel turning white and a pixel turning black — and why these instructions fill image and audio code. The inner `s` means signed, `us` unsigned. namsdec runs them: compare `paddb` and `paddusb` on the same bytes and the difference shows in the register.",
        },
        flags: "",
        example: "paddusb xmm0, xmm1  ; 250 + 10 -> 255, jamais 4",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pmullw",
            "pmulhw",
            "pmulhuw",
            "pmulld",
            "pmuludq",
            "pmuldq",
            "pmulhrsw",
            "vpmullw",
            "vpmulhw",
            "vpmulhuw",
            "vpmulld",
            "vpmuludq",
            "vpmuldq",
            "vpmulhrsw",
        ],
        title: Text {
            fr: "Multiplier en parallèle",
            en: "Multiply in parallel",
        },
        body: Text {
            fr: "Le produit de deux nombres de *n* bits en tient 2*n* : il ne rentre pas dans l'élément d'où il sort. D'où la famille — `pmullw` garde la moitié basse, `pmulhw` la moitié haute, et il faut les deux pour le produit entier. `pmuludq` prend l'autre chemin : il ne multiplie qu'un élément sur deux, mais garde les 64 bits du résultat. namsdec les exécute, et c'est le seul moyen de vérifier qu'on a bien compris laquelle des deux moitiés on garde.",
            en: "The product of two *n*-bit numbers needs 2*n*: it does not fit in the element it came from. Hence the family — `pmullw` keeps the low half, `pmulhw` the high half, and you need both for the whole product. `pmuludq` takes the other road: it multiplies only every other element, but keeps all 64 bits of the result. namsdec runs them, which is the only way to check one has understood which half is kept.",
        },
        flags: "",
        example: "pmullw xmm0, xmm1   ; huit produits, moitié basse",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["pmaddwd", "pmaddubsw", "vpmaddwd", "vpmaddubsw"],
        title: Text {
            fr: "Multiplier et additionner d'un coup",
            en: "Multiply and add in one step",
        },
        body: Text {
            fr: "Multiplient élément par élément, puis additionnent les résultats **deux par deux** — la brique du produit scalaire, et donc de presque tout le calcul matriciel. La largeur double au passage, ce qui laisse la place à la somme : seize octets deviennent huit mots, huit mots deviennent quatre doubles mots. `pmaddubsw` porte en plus une asymétrie que son nom n'annonce pas : sa première source est non signée, la seconde signée — de quoi multiplier des pixels par des coefficients négatifs. namsdec les exécute.",
            en: "Multiply element by element, then add the results **in pairs** — the building block of the dot product, and so of nearly all matrix work. The width doubles on the way, leaving room for the sum: sixteen bytes become eight words, eight words become four doublewords. `pmaddubsw` adds an asymmetry its name does not announce: its first source is unsigned, the second signed — enough to multiply pixels by negative coefficients. namsdec runs them.",
        },
        flags: "",
        example: "pmaddwd xmm0, xmm1  ; 8 produits -> 4 sommes",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "phaddw", "phaddd", "phaddsw", "phsubw", "phsubd", "phsubsw", "vphaddw", "vphaddd",
            "vphaddsw", "vphsubw", "vphsubd", "vphsubsw",
        ],
        title: Text {
            fr: "Additionner voisin à voisin",
            en: "Add neighbour to neighbour",
        },
        body: Text {
            fr: "Additionnent les éléments **côte à côte dans le même registre**, au lieu d'un registre sur l'autre. C'est le geste qui manquait pour totaliser un vecteur : quatre nombres, deux `phaddd`, et la somme est en bas. Le `s` de `phaddsw` sature au lieu de boucler. namsdec les exécute.",
            en: "Add elements **side by side within one register**, rather than across two. This is the move that was missing to total a vector: four numbers, two `phaddd`, and the sum sits at the bottom. The `s` in `phaddsw` saturates instead of wrapping. namsdec runs them.",
        },
        flags: "",
        example: "phaddd xmm0, xmm0
phaddd xmm0, xmm0   ; la somme des quatre",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pmovzxbw",
            "pmovzxbd",
            "pmovzxbq",
            "pmovzxwd",
            "pmovzxwq",
            "pmovzxdq",
            "pmovsxbw",
            "pmovsxbd",
            "pmovsxbq",
            "pmovsxwd",
            "pmovsxwq",
            "pmovsxdq",
            "vpmovzxbw",
            "vpmovzxbd",
            "vpmovzxbq",
            "vpmovzxwd",
            "vpmovzxwq",
            "vpmovzxdq",
            "vpmovsxbw",
            "vpmovsxbd",
            "vpmovsxbq",
            "vpmovsxwd",
            "vpmovsxwq",
            "vpmovsxdq",
        ],
        title: Text {
            fr: "Élargir chaque élément",
            en: "Widen every element",
        },
        body: Text {
            fr: "Ce que `movzx` et `movsx` font pour un nombre, ceux-ci le font pour tout un vecteur : huit octets deviennent huit mots, `zx` en complétant par des zéros, `sx` en recopiant le bit de signe. Seule la moitié basse de la source est lue — elle occupe deux fois plus de place à l'arrivée. namsdec les exécute : c'est l'inverse exact de `packuswb`, et on peut faire l'aller-retour pour s'en convaincre.",
            en: "What `movzx` and `movsx` do for one number, these do for a whole vector: eight bytes become eight words, `zx` filling with zeroes, `sx` copying the sign bit. Only the low half of the source is read — it takes twice the room on arrival. namsdec runs them: it is the exact inverse of `packuswb`, and the round trip can be made to prove it.",
        },
        flags: "",
        example: "pmovzxbw xmm0, xmm1 ; 8 octets -> 8 mots",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["lddqu", "vlddqu"],
        title: Text {
            fr: "Charger seize octets mal alignés",
            en: "Load sixteen unaligned bytes",
        },
        body: Text {
            fr: "Fait ce que fait `movdqu` — lire seize octets où qu'ils tombent —, mais en allant chercher davantage et en taillant dedans, ce qui était plus rapide sur certains processeurs d'alors. Sur les machines d'aujourd'hui il n'y a plus de différence ; on le rencontre surtout dans du code compilé il y a quinze ans. namsdec l'exécute comme `movdqu`, ce qu'elle est devenue.",
            en: "Does what `movdqu` does — read sixteen bytes wherever they fall — but by fetching more and cutting inside, which was faster on some processors of the time. On today's machines there is no difference left; it mostly turns up in code compiled fifteen years ago. namsdec runs it as `movdqu`, which is what it has become.",
        },
        flags: "",
        example: "lddqu xmm0, [rsi]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "prefetcht0",
            "prefetcht1",
            "prefetcht2",
            "prefetchnta",
            "prefetchw",
            "prefetchwt1",
            "prefetch",
        ],
        title: Text {
            fr: "Prévenir le cache",
            en: "Warn the cache",
        },
        body: Text {
            fr: "Demandent d'amener une ligne de cache **avant** qu'on en ait besoin. Elles ne lisent rien, n'écrivent rien, ne peuvent pas provoquer de faute : au pire elles ne servent à rien. Le chiffre dit jusqu'à quel niveau de cache remonter, `nta` signifie « je ne relirai pas ceci » et évite d'évincer le reste. C'est l'instruction la plus fréquente de ce manuel qui ne fait, littéralement, rien d'observable. namsdec la laisse donc passer sans rien faire — ce qui est fidèle, puisqu'il n'a pas de cache à préchauffer — plutôt que de s'arrêter sur une instruction sans effet.",
            en: "Ask for a cache line to be fetched **before** it is needed. They read nothing, write nothing, and cannot fault: at worst they are useless. The digit says how far up the cache to bring it, `nta` means \"I will not read this again\" and avoids evicting the rest. It is the most frequent instruction in this manual that does, literally, nothing observable. namsdec therefore lets it pass doing nothing — which is faithful, having no cache to warm — rather than stopping on an instruction with no effect.",
        },
        flags: "",
        example: "prefetcht0 [rsi + 64]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["xgetbv", "xsetbv", "xsave", "xrstor", "xsaveopt", "xsavec"],
        title: Text {
            fr: "Demander ce que ce processeur sait faire",
            en: "Ask what this processor can do",
        },
        body: Text {
            fr: "`xgetbv` lit quelles extensions le système d'exploitation a autorisées — car un processeur peut connaître l'AVX sans que le noyau accepte d'en sauvegarder les registres. C'est la deuxième moitié du test que fait tout programme qui choisit son chemin rapide au démarrage, `cpuid` étant la première. `xsave` et `xrstor` rangent et restaurent ces registres-là. namsdec ne les simule pas : il n'y a pas de noyau derrière lui pour répondre.",
            en: "`xgetbv` reads which extensions the operating system has enabled — a processor can know AVX without the kernel agreeing to save its registers. It is the second half of the check every program makes when picking its fast path at startup, `cpuid` being the first. `xsave` and `xrstor` store and restore those registers. namsdec does not simulate them: there is no kernel behind it to answer.",
        },
        flags: "",
        example: "xor ecx, ecx\nxgetbv          ; EDX:EAX = ce qui est permis",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "vmcall", "vmmcall", "vmlaunch", "vmresume", "vmxon", "vmxoff", "vmread", "vmwrite",
        ],
        title: Text {
            fr: "Parler à l'hyperviseur",
            en: "Call the hypervisor",
        },
        body: Text {
            fr: "L'équivalent de `syscall`, un étage plus bas : le système invité appelle la machine qui l'héberge. `vmcall` est la forme Intel, `vmmcall` la forme AMD. Hors d'une machine virtuelle, elles lèvent une exception — les rencontrer dans un binaire ordinaire signale du code qui teste s'il est observé. namsdec ne les simule pas — il n'héberge personne.",
            en: "The equivalent of `syscall`, one storey down: the guest system calls the machine hosting it. `vmcall` is the Intel form, `vmmcall` the AMD one. Outside a virtual machine they raise an exception — finding them in an ordinary binary signals code checking whether it is being watched. namsdec does not simulate them — it hosts nobody.",
        },
        flags: "",
        example: "vmcall",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pmovmskb",
            "movmskps",
            "movmskpd",
            "vpmovmskb",
            "vmovmskps",
            "vmovmskpd",
        ],
        title: Text {
            fr: "Résumer les bits de signe",
            en: "Gather the sign bits",
        },
        body: Text {
            fr: "Rassemblent le bit de signe de chaque élément dans un registre général : un bit par octet, par flottant simple ou par double. C'est ainsi qu'on ramène le résultat d'une comparaison vectorielle là où un saut conditionnel peut le lire.",
            en: "Gather each element's sign bit into a general register: one bit per byte, per single or per double. This is how a vector comparison's result gets somewhere a conditional jump can read it.",
        },
        flags: "",
        example: "pcmpeqb xmm0, xmm1\npmovmskb eax, xmm0\ntest eax, eax",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "pshufd", "shufps", "shufpd", "vpshufd", "vshufps", "vshufpd",
        ],
        title: Text {
            fr: "Remettre dans l'ordre",
            en: "Shuffle",
        },
        body: Text {
            fr: "Réordonnent les éléments selon un immédiat, deux bits par élément. `shufps` prend les deux éléments bas dans la première source et les deux hauts dans la seconde — d'où son usage pour transposer. En AVX, chaque moitié de 128 bits est mélangée séparément.",
            en: "Reorder the elements according to an immediate, two bits per element. `shufps` takes its two low elements from the first source and its two high ones from the second — hence its use for transposing. In AVX each 128-bit half is shuffled separately.",
        },
        flags: "",
        example: "pshufd xmm0, xmm1, 00_01_10_11b  ; ordre inversé",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["movhlps", "movlhps", "vmovhlps", "vmovlhps"],
        title: Text {
            fr: "Échanger les moitiés",
            en: "Swap the halves",
        },
        body: Text {
            fr: "Font passer la moitié haute d'un registre en bas, ou l'inverse. Le seul moyen, en 128 bits, d'atteindre ce que les instructions scalaires ne voient pas.",
            en: "Move a register's upper half down, or the other way. In 128 bits, the only way to reach what the scalar instructions cannot see.",
        },
        flags: "",
        example: "movhlps xmm0, xmm1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "punpcklbw",
            "punpcklwd",
            "punpckldq",
            "punpcklqdq",
            "punpckhbw",
            "punpckhwd",
            "punpckhdq",
            "punpckhqdq",
            "unpcklps",
            "unpcklpd",
            "unpckhps",
            "unpckhpd",
            "vpunpcklbw",
            "vpunpcklwd",
            "vpunpckldq",
            "vpunpcklqdq",
            "vpunpckhbw",
            "vpunpckhwd",
            "vpunpckhdq",
            "vpunpckhqdq",
            "vunpcklps",
            "vunpcklpd",
            "vunpckhps",
            "vunpckhpd",
        ],
        title: Text {
            fr: "Entrelacer",
            en: "Interleave",
        },
        body: Text {
            fr: "Prennent une moitié de chaque source et en entrelacent les éléments. C'est ainsi qu'on élargit des octets en mots — en entrelaçant avec des zéros — et qu'on transpose une matrice.",
            en: "Take one half of each source and interleave their elements. This is how you widen bytes into words — by interleaving with zeroes — and how you transpose a matrix.",
        },
        flags: "",
        example: "punpcklbw xmm0, xmm1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "vbroadcastss",
            "vbroadcastsd",
            "vpbroadcastb",
            "vpbroadcastw",
            "vpbroadcastd",
            "vpbroadcastq",
        ],
        title: Text {
            fr: "Diffuser une valeur",
            en: "Broadcast a value",
        },
        body: Text {
            fr: "Recopient un seul élément dans tout le registre. C'est ainsi qu'on multiplie un vecteur entier par un scalaire : il faut d'abord que le scalaire soit partout.",
            en: "Copy one element across the whole register. This is how you multiply a whole vector by a scalar: the scalar has to be everywhere first.",
        },
        flags: "",
        example: "vbroadcastss ymm0, [facteur]\nvmulps ymm1, ymm1, ymm0",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "vinsertf128",
            "vinserti128",
            "vextractf128",
            "vextracti128",
            "vperm2f128",
            "vperm2i128",
            "vpermq",
            "vpermpd",
        ],
        title: Text {
            fr: "Déplacer les moitiés de 128 bits",
            en: "Move the 128-bit halves",
        },
        body: Text {
            fr: "Les deux moitiés d'un registre 256 bits ne se parlent pas d'elles-mêmes : la plupart des instructions AVX travaillent dans chaque moitié séparément. Ces instructions-là sont celles qui les nomment — en insérer une, en extraire une, ou les permuter.",
            en: "The two halves of a 256-bit register do not talk to each other: most AVX instructions work inside each half separately. These are the ones that name them — insert one, extract one, or permute them.",
        },
        flags: "",
        example: "vextractf128 xmm0, ymm1, 1   ; la moitié haute\nvperm2f128 ymm0, ymm1, ymm1, 1",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["vzeroupper", "vzeroall"],
        title: Text {
            fr: "Nettoyer avant du SSE",
            en: "Clean up before SSE",
        },
        body: Text {
            fr: "Remettent à zéro la moitié haute de tous les registres vectoriels. À appeler avant de repasser à du code SSE : sur un vrai processeur, mélanger les deux jeux sans cela coûte une pénalité à chaque transition.",
            en: "Zero the upper half of every vector register. To be called before going back to SSE code: on a real processor, mixing the two instruction sets without it costs a penalty at every transition.",
        },
        flags: "",
        example: "vzeroupper\nret",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["movsd"],
        title: Text {
            fr: "movsd : deux instructions, un seul nom",
            en: "movsd: two instructions, one name",
        },
        body: Text {
            fr: "NASM accepte ce nom pour deux choses sans rapport : sans opérande, c'est l'instruction de chaîne qui copie quatre octets de [RSI] vers [RDI] ; avec deux opérandes, c'est le déplacement d'un flottant double entre registres vectoriels. Ce sont les opérandes qui tranchent, et rien d'autre.",
            en: "NASM accepts this name for two unrelated things: with no operand, it is the string instruction copying four bytes from [RSI] to [RDI]; with two operands, it moves one double-precision float between vector registers. The operands decide, and nothing else.",
        },
        flags: "",
        example: "rep movsd           ; la chaîne : quatre octets à la fois\nmovsd xmm0, [x]     ; le flottant",
    },
    Entry {
        kind: Kind::Directive,
        names: &["section", "segment"],
        title: Text {
            fr: "Découper le programme",
            en: "Split the program",
        },
        body: Text {
            fr: "Ouvre une section : `.text` pour le code, `.data` pour les données écrites d'avance, `.bss` pour celles qu'on veut à zéro. Chacune reçoit ses droits — namsdec refuse d'écrire dans `.text` et d'exécuter `.data`, comme un vrai système.",
            en: "Opens a section: `.text` for code, `.data` for data written in advance, `.bss` for data you want zeroed. Each gets its rights — namsdec refuses to write into `.text` or execute `.data`, like a real system.",
        },
        flags: "",
        example: "section .data\n    total dq 0\n\nsection .text",
    },
    Entry {
        kind: Kind::Directive,
        names: &["global", "extern", "common", "static", "required"],
        title: Text {
            fr: "Ce qui sort, ce qui entre",
            en: "What leaves, what comes in",
        },
        body: Text {
            fr: "`global` rend un nom visible depuis l'extérieur — c'est ainsi que `ld` trouve `_start`, sans quoi il choisit un point d'entrée à votre place en avertissant. `extern` annonce un nom défini ailleurs.",
            en: "`global` makes a name visible from outside — this is how `ld` finds `_start`; without it, it picks an entry point for you and says so. `extern` announces a name defined elsewhere.",
        },
        flags: "",
        example: "global _start\nextern printf",
    },
    Entry {
        kind: Kind::Directive,
        names: &["db", "dw", "dd", "dq", "dt", "do", "dy", "dz"],
        title: Text {
            fr: "Écrire des octets",
            en: "Write bytes",
        },
        body: Text {
            fr: "Posent des valeurs dans le fichier : `db` un octet, `dw` deux, `dd` quatre, `dq` huit, `dt` dix. Ce ne sont pas des instructions — rien ne s'exécute, les octets sont simplement là. `db` accepte aussi du texte, un caractère par octet.",
            en: "Put values into the file: `db` one byte, `dw` two, `dd` four, `dq` eight, `dt` ten. These are not instructions — nothing executes, the bytes are simply there. `db` also takes text, one character per byte.",
        },
        flags: "",
        example: "message db \"salut\", 10, 0\nnombres dd 1, 2, 3\npi      dq 3.14159",
    },
    Entry {
        kind: Kind::Directive,
        names: &[
            "resb", "resw", "resd", "resq", "rest", "reso", "resy", "resz",
        ],
        title: Text {
            fr: "Réserver sans écrire",
            en: "Reserve without writing",
        },
        body: Text {
            fr: "Réservent de la place sans rien y mettre : le fichier ne grossit pas, la mémoire est à zéro au démarrage. C'est ce qui distingue `.bss` de `.data` — un tampon d'un mégaoctet coûte un mégaoctet de fichier avec `db`, et rien du tout avec `resb`.",
            en: "Reserve room without putting anything in it: the file does not grow, and the memory is zero at startup. That is what separates `.bss` from `.data` — a one-megabyte buffer costs a megabyte of file with `db`, and nothing at all with `resb`.",
        },
        flags: "",
        example: "section .bss\n    tampon_lu resb 1024\n    restant resq 1",
    },
    Entry {
        kind: Kind::Directive,
        names: &["times"],
        title: Text {
            fr: "Répéter",
            en: "Repeat",
        },
        body: Text {
            fr: "Répète ce qui suit un nombre de fois. Avec `db`, c'est la façon de remplir une zone ; avec une instruction, de poser du remplissage. Le nombre peut être une expression, y compris `$` — c'est ainsi qu'on complète un secteur de démarrage à 512 octets.",
            en: "Repeats what follows a number of times. With `db` it fills a region; with an instruction it lays down padding. The count can be an expression, `$` included — that is how a boot sector is padded to 512 bytes.",
        },
        flags: "",
        example: "remplissage times 16 db 0\n            times 4 nop",
    },
    Entry {
        kind: Kind::Directive,
        names: &["equ"],
        title: Text {
            fr: "Donner un nom à une valeur",
            en: "Name a value",
        },
        body: Text {
            fr: "Associe un nom à une valeur, calculée à l'assemblage. Contrairement à `db`, rien n'est écrit en mémoire : le nom est remplacé par sa valeur partout où il paraît. C'est la constante de l'assembleur.",
            en: "Binds a name to a value, computed at assembly time. Unlike `db`, nothing is written to memory: the name is replaced by its value wherever it appears. This is the assembler's constant.",
        },
        flags: "",
        example: "phrase  db \"salut\", 10\ntaille  equ $ - phrase",
    },
    Entry {
        kind: Kind::Directive,
        names: &["$", "$$"],
        title: Text {
            fr: "Où en est l'assembleur",
            en: "Where the assembler is",
        },
        body: Text {
            fr: "`$` vaut l'adresse de la ligne courante, `$$` celle du début de la section. `$ - message` donne donc le nombre d'octets écrits depuis l'étiquette : la longueur d'une chaîne, sans la compter à la main ni la corriger quand on la change.",
            en: "`$` is the address of the current line, `$$` the start of the section. So `$ - message` gives the number of bytes written since that label: the length of a string, without counting by hand or fixing it when the string changes.",
        },
        flags: "",
        example: "phrase  db \"salut\", 10\ntaille  equ $ - phrase",
    },
    Entry {
        kind: Kind::Directive,
        names: &[
            "byte", "word", "dword", "qword", "tword", "oword", "yword", "zword",
        ],
        title: Text {
            fr: "Dire la taille",
            en: "State the size",
        },
        body: Text {
            fr: "Quand l'adresse seule ne dit pas combien d'octets écrire — `mov [x], 1` : un octet ? huit ? — c'est à vous de le préciser. NASM refuse plutôt que de deviner, et il a raison.",
            en: "When the address alone does not say how many bytes to write — `mov [x], 1`: one byte? eight? — you have to say. NASM refuses rather than guess, and it is right to.",
        },
        flags: "",
        example: "mov byte [buf], 1\nmov qword [n], 42\nmovzx eax, word [buf]",
    },
    Entry {
        kind: Kind::Directive,
        names: &["default", "rel", "abs", "wrt", "seg", "strict"],
        title: Text {
            fr: "Adressage relatif ou absolu",
            en: "Relative or absolute addressing",
        },
        body: Text {
            fr: "`default rel` fait calculer les adresses par rapport à RIP : c'est ce qu'écrit tout le monde aujourd'hui, et ce qu'exige un exécutable relogeable. Sans cette ligne, nasm 3 avertit à chaque accès mémoire par étiquette.",
            en: "`default rel` makes addresses count from RIP: this is what everyone writes today, and what a relocatable executable requires. Without that line, nasm 3 warns on every label-based memory access.",
        },
        flags: "",
        example: "default rel\n\nsection .text\n    mov rax, [valeur]",
    },
    Entry {
        kind: Kind::Directive,
        names: &["align", "alignb"],
        title: Text {
            fr: "Aligner",
            en: "Align",
        },
        body: Text {
            fr: "Avance jusqu'à la prochaine adresse multiple de n. Indispensable avant `movaps` ou `vmovaps`, qui exigent 16 ou 32 octets d'alignement et faute sinon — sur un vrai processeur, pas ici.",
            en: "Advances to the next address that is a multiple of n. Needed before `movaps` or `vmovaps`, which require 16- or 32-byte alignment and fault otherwise — on a real processor, not here.",
        },
        flags: "",
        example: "align 32\nquatre dd 1.0, 2.0, 3.0, 4.0",
    },
    Entry {
        kind: Kind::Directive,
        names: &["bits", "use16", "use32", "use64"],
        title: Text {
            fr: "Largeur du mode",
            en: "Mode width",
        },
        body: Text {
            fr: "Dit à l'assembleur en quel mode le code tournera : 16, 32 ou 64 bits. Avec `nasm -f elf64`, c'est déjà 64 bits — `BITS 64` n'y ajoute rien, et on peut l'omettre.",
            en: "Tells the assembler which mode the code will run in: 16, 32 or 64 bits. With `nasm -f elf64` it is already 64 — `BITS 64` adds nothing and can be left out.",
        },
        flags: "",
        example: "bits 64",
    },
    Entry {
        kind: Kind::Directive,
        names: &["incbin"],
        title: Text {
            fr: "Inclure un fichier tel quel",
            en: "Include a file as is",
        },
        body: Text {
            fr: "Recopie un fichier dans la sortie, octet pour octet : une image, une police, une table. Rien n'est interprété.",
            en: "Copies a file into the output, byte for byte: an image, a font, a table. Nothing is interpreted.",
        },
        flags: "",
        example: "table incbin \"table.bin\"",
    },
    Entry {
        kind: Kind::Instruction,
        names: &["notrack"],
        title: Text {
            fr: "Sauter sans vérifier",
            en: "Jump without checking",
        },
        body: Text {
            fr: "Un préfixe posé devant un saut indirect pour dire au processeur de ne pas exiger d'`endbr64` à l'arrivée. Il n'a de sens que sur une machine où IBT est actif : le compilateur le met là où il sait que la cible est sûre — une table de sauts qu'il a produite lui-même, par exemple. En lire un dans un binaire, c'est voir où les défenses matérielles sont volontairement levées.",
            en: "A prefix placed before an indirect jump to tell the processor not to require an `endbr64` at the destination. It only means anything on a machine where IBT is on: the compiler puts it where it knows the target is safe — a jump table it produced itself, say. Reading one in a binary shows where the hardware defences are deliberately lifted.",
        },
        flags: "",
        example: "notrack jmp rax",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "in", "out", "insb", "insw", "insd", "outsb", "outsw", "outsd",
        ],
        title: Text {
            fr: "Parler aux ports d'entrée-sortie",
            en: "Talk to the I/O ports",
        },
        body: Text {
            fr: "Les instructions qui parlent au matériel par les ports d'entrée-sortie, hors de la mémoire. Un programme utilisateur n'y a pas droit : sous Linux, elles font une faute de protection générale. On ne les rencontre que dans un noyau, un pilote ou un chargeur d'amorçage — ou parce qu'on lit des données comme du code, ce qui arrive vite au milieu d'une section qui n'en est pas une. namsdec ne les simule pas : on les lit, on ne les exécute pas ici.",
            en: "The instructions that talk to hardware through the I/O ports, outside memory. A user program is not allowed near them: on Linux they raise a general protection fault. One only meets them in a kernel, a driver or a bootloader — or because data is being read as code, which happens quickly in the middle of a section that is not one. namsdec does not simulate them: they are read here, not run.",
        },
        flags: "",
        example: "in al, dx",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "movhps", "movlps", "movhpd", "movlpd", "vmovhps", "vmovlps", "vmovhpd", "vmovlpd",
        ],
        title: Text {
            fr: "Ne bouger qu'une moitié",
            en: "Move one half only",
        },
        body: Text {
            fr: "Lisent ou écrivent huit octets dans la moitié *haute* (`movhps`) ou *basse* (`movlps`) d'un registre vectoriel, en laissant l'autre intacte. C'est ainsi qu'on charge deux morceaux venus d'endroits différents dans un même registre — et c'est pourquoi on les voit par paires dans le code des compilateurs. C'est aussi ce qui les distingue de `movsd`, qui efface le haut en lisant la mémoire : deux d'entre elles mises bout à bout recopient seize octets sans exiger le moindre alignement, recette de tous les `memcpy` courts du monde.",
            en: "Read or write eight bytes into the *upper* (`movhps`) or *lower* (`movlps`) half of a vector register, leaving the other untouched. That is how two pieces from different places end up in one register — which is why they come in pairs in compiler output. It is also what sets them apart from `movsd`, which wipes the upper half when reading memory: two of them back to back copy sixteen bytes with no alignment requirement at all, the recipe behind every short `memcpy` there is.",
        },
        flags: "",
        example: "movlps xmm0, [vecteur]\nmovhps xmm0, [vecteur + 8]",
    },
    Entry {
        kind: Kind::Instruction,
        names: &[
            "packuswb",
            "packsswb",
            "packssdw",
            "packusdw",
            "vpackuswb",
            "vpacksswb",
            "vpackssdw",
            "vpackusdw",
        ],
        title: Text {
            fr: "Rétrécir en empaquetant",
            en: "Pack down to a smaller size",
        },
        body: Text {
            fr: "Prennent deux registres de valeurs larges et en font un seul de valeurs deux fois plus étroites, en saturant ce qui ne tient pas — `packuswb` ramène des mots de 16 bits à des octets non signés, en bornant à 0 et 255 plutôt qu'en tronquant. C'est l'opération qui termine presque tout traitement d'image ou de son vectorisé. namsdec l'exécute : la source est toujours lue comme signée, même quand la destination ne l'est pas, et `packuswb` ramène donc tout négatif à zéro.",
            en: "Take two registers of wide values and make one of values half as wide, saturating what does not fit — `packuswb` brings 16-bit words down to unsigned bytes, clamping to 0 and 255 rather than truncating. It is the operation that ends almost every vectorised image or sound routine. namsdec runs it: the source is always read as signed even when the destination is not, so `packuswb` brings anything negative down to zero.",
        },
        flags: "",
        example: "packuswb xmm0, xmm1",
    },
    Entry {
        kind: Kind::Directive,
        names: &["struc", "endstruc", "istruc", "at", "iend"],
        title: Text {
            fr: "Décrire une structure",
            en: "Describe a structure",
        },
        body: Text {
            fr: "`struc` ouvre une description de structure : chaque champ y réserve sa place avec `resb`, `resq`… et devient une constante — son décalage depuis le début. On écrit ensuite `[rsi + point.x]` plutôt que `[rsi + 8]`, et l'on peut réordonner les champs sans relire tout le programme. `istruc` … `at` … `iend` en pose une, remplie, dans les données.",
            en: "`struc` opens a structure description: each field reserves its room with `resb`, `resq`… and becomes a constant — its offset from the start. One then writes `[rsi + point.x]` rather than `[rsi + 8]`, and can reorder the fields without re-reading the whole program. `istruc` … `at` … `iend` lays a filled one down in the data.",
        },
        flags: "",
        example: "struc point\n    .x: resq 1\n    .y: resq 1\nendstruc",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &[
            "%define",
            "%idefine",
            "%xdefine",
            "%ixdefine",
            "%undef",
            "%assign",
            "%iassign",
            "%clear",
        ],
        title: Text {
            fr: "Remplacer avant d'assembler",
            en: "Replace before assembling",
        },
        body: Text {
            fr: "Le préprocesseur remplace du texte *avant* que l'assembleur ne voie quoi que ce soit. `%define` prend des arguments, comme une macro du C ; `%assign` calcule sa valeur tout de suite. Rien de tout cela n'existe encore à l'exécution.",
            en: "The preprocessor replaces text *before* the assembler sees anything. `%define` takes arguments, like a C macro; `%assign` computes its value right away. None of it still exists at run time.",
        },
        flags: "",
        example: "%define TAILLE 64\n%define double(x) ((x) * 2)\n    mov rax, TAILLE",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &[
            "%macro",
            "%endmacro",
            "%imacro",
            "%unmacro",
            "%unimacro",
            "%exitmacro",
            "%rotate",
        ],
        title: Text {
            fr: "Écrire une fois, poser partout",
            en: "Write once, place anywhere",
        },
        body: Text {
            fr: "Une macro est du texte à recopier, avec des arguments numérotés `%1`, `%2`. Elle ne coûte pas d'appel — les instructions sont écrites sur place — mais elle ne partage rien non plus : deux usages font deux copies.",
            en: "A macro is text to be copied, with numbered arguments `%1`, `%2`. It costs no call — the instructions are written in place — but it shares nothing either: two uses make two copies.",
        },
        flags: "",
        example: "%macro afficher 2\n    mov rsi, %1\n    mov rdx, %2\n%endmacro",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &["%include", "%pathsearch", "%depend", "%use"],
        title: Text {
            fr: "Inclure un autre fichier",
            en: "Include another file",
        },
        body: Text {
            fr: "Recopie un fichier source à cet endroit, avant l'assemblage. C'est ainsi qu'on partage des macros et des constantes entre plusieurs programmes.",
            en: "Copies a source file in at this point, before assembly. This is how macros and constants are shared between programs.",
        },
        flags: "",
        example: "%include \"macros.inc\"",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &[
            "%if", "%ifdef", "%ifndef", "%ifmacro", "%ifctx", "%ifidn", "%ifidni", "%ifid",
            "%ifnum", "%ifstr", "%iftoken", "%ifempty", "%elif", "%else", "%endif",
        ],
        title: Text {
            fr: "Assembler ou non",
            en: "Assemble, or not",
        },
        body: Text {
            fr: "Choisit ce qui sera assemblé. Ce qui est écarté n'existe pas dans le fichier produit — ce n'est pas un `if` d'exécution, c'est une décision prise à l'assemblage.",
            en: "Chooses what gets assembled. What is left out does not exist in the produced file — this is not a run-time `if`, it is a decision made at assembly time.",
        },
        flags: "",
        example: "%ifdef DEBUG\n    mov rax, 1\n%endif",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &["%rep", "%endrep", "%exitrep"],
        title: Text {
            fr: "Répéter du texte",
            en: "Repeat text",
        },
        body: Text {
            fr: "Répète un bloc au moment de l'assemblage — là où `times` répète une seule ligne. Utile pour dérouler une boucle ou fabriquer une table.",
            en: "Repeats a block at assembly time — where `times` repeats a single line. Useful to unroll a loop or build a table.",
        },
        flags: "",
        example: "%rep 4\n    nop\n%endrep",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &["%error", "%warning", "%fatal"],
        title: Text {
            fr: "Se plaindre à l'assemblage",
            en: "Complain at assembly time",
        },
        body: Text {
            fr: "Fait dire quelque chose à l'assembleur, et arrêter au besoin. C'est le seul moyen de vérifier une hypothèse avant que le programme n'existe.",
            en: "Makes the assembler say something, and stop if needed. It is the only way to check an assumption before the program exists.",
        },
        flags: "",
        example: "%ifndef TAILLE\n%error \"TAILLE n'est pas définie\"\n%endif",
    },
    Entry {
        kind: Kind::Directive,
        names: &["absolute"],
        title: Text {
            fr: "Décrire sans rien produire",
            en: "Describe without producing anything",
        },
        body: Text {
            fr: "Ouvre une zone où les étiquettes reçoivent une adresse sans qu'aucun octet ne sorte : on décrit la forme de quelque chose — un en-tête, un bloc que le noyau remplira — pour pouvoir écrire `[rsi + champ]` plutôt que `[rsi + 8]`. `struc` est bâti là-dessus.",
            en: "Opens a region where labels get addresses without a single byte being emitted: one describes the shape of something — a header, a block the kernel will fill — so as to write `[rsi + field]` rather than `[rsi + 8]`. `struc` is built on it.",
        },
        flags: "",
        example: "absolute 0\n    .compteur: resq 1\n    .taille:   resq 1\nsection .text",
    },
    Entry {
        kind: Kind::Directive,
        names: &["cpu", "float", "list", "warning", "dollarhex"],
        title: Text {
            fr: "Régler l'assembleur",
            en: "Set the assembler up",
        },
        body: Text {
            fr: "Des directives qui ne produisent rien et changent la façon dont NASM lit la suite. `cpu` refuse ce que le processeur visé ne connaît pas — utile pour ne pas écrire par mégarde une instruction absente de la machine cible. `warning` allume ou éteint une famille d'avertissements, `float` règle l'arrondi des constantes, `list` interrompt le listing le temps d'un bloc.",
            en: "Directives that produce nothing and change how NASM reads what follows. `cpu` refuses what the target processor does not know — handy for not writing, by accident, an instruction absent from the target machine. `warning` turns a family of warnings on or off, `float` sets the rounding of constants, `list` suspends the listing for a block.",
        },
        flags: "",
        example: "cpu x86-64\nsection .text",
    },
    Entry {
        kind: Kind::Directive,
        names: &[
            "prefix", "suffix", "gprefix", "gsuffix", "lprefix", "lsuffix",
        ],
        title: Text {
            fr: "Renommer tous les symboles d'un coup",
            en: "Rename every symbol at once",
        },
        body: Text {
            fr: "Ajoutent la même chose devant ou derrière chaque symbole produit. C'est ce qui permet de lier un module à un compilateur qui attend des noms décorés — un tiret bas devant, par exemple — sans retoucher le source ligne à ligne.",
            en: "Add the same thing before or after every symbol emitted. That is what lets a module be linked against a compiler expecting decorated names — a leading underscore, say — without touching the source line by line.",
        },
        flags: "",
        example: "; nasm -f elf64 --prefix _ programme.asm",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &[
            "%defstr",
            "%idefstr",
            "%deftok",
            "%ideftok",
            "%defalias",
            "%idefalias",
            "%undefalias",
        ],
        title: Text {
            fr: "Fabriquer des noms et des chaînes",
            en: "Make names and strings",
        },
        body: Text {
            fr: "`%defstr` transforme ce qu'on lui donne en chaîne entre guillemets, `%deftok` fait l'inverse — une chaîne redevient du texte que l'assembleur lira comme du code. `%defalias` donne un second nom à une macro, sans la recopier.",
            en: "`%defstr` turns what it is given into a quoted string, `%deftok` does the reverse — a string becomes text again, which the assembler reads as code. `%defalias` gives a macro a second name, without copying it.",
        },
        flags: "",
        example: "%defstr NOM  programme\nsection .text",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &["%strcat", "%strlen", "%substr"],
        title: Text {
            fr: "Travailler les chaînes avant l'assemblage",
            en: "Work on strings before assembly",
        },
        body: Text {
            fr: "Coller, mesurer, découper — pendant la lecture du source, pas à l'exécution. Ce qui en sort est une constante comme une autre : `%strlen` donne un nombre qu'on peut mettre dans un `db`, sans qu'aucune instruction n'ait à compter quoi que ce soit.",
            en: "Join, measure, cut — while the source is being read, not at run time. What comes out is a constant like any other: `%strlen` gives a number one can put in a `db`, with no instruction having to count anything.",
        },
        flags: "",
        example: "%strlen TAILLE 'salut'\nsection .data\n    taille_du_mot: db TAILLE",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &["%push", "%pop", "%repl", "%$", "%$$"],
        title: Text {
            fr: "La pile de contextes",
            en: "The context stack",
        },
        body: Text {
            fr: "De quoi écrire des macros qui s'emboîtent — un `%push` ouvre un contexte, un `%pop` le referme — et donner à leurs étiquettes des noms qui ne se marchent pas dessus. Sans cela, deux appels d'une même macro dans un même fichier définiraient deux fois l'étiquette.",
            en: "What lets macros nest — a `%push` opens a context, a `%pop` closes it — and gives their labels names that do not tread on each other. Without it, two uses of the same macro in one file would define the same label twice.",
        },
        flags: "",
        example: "%push boucle\n%$debut:\n%pop\nsection .text",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &["%arg", "%local", "%stacksize"],
        title: Text {
            fr: "Nommer ce qui est sur la pile",
            en: "Name what is on the stack",
        },
        body: Text {
            fr: "Donnent des noms aux arguments reçus sur la pile et aux variables locales d'une fonction, plutôt que d'écrire `[rbp + 16]` et de recompter à chaque modification. Hérité des conventions 16 et 32 bits, où tout passait par la pile ; en 64 bits, les six premiers arguments arrivent dans des registres.",
            en: "Give names to arguments received on the stack and to a function's local variables, rather than writing `[rbp + 16]` and recounting on every change. Inherited from the 16- and 32-bit conventions, where everything went through the stack; in 64-bit code the first six arguments arrive in registers.",
        },
        flags: "",
        example: "%stacksize flat64\nsection .text",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &["%note", "%pragma", "%line", "%!"],
        title: Text {
            fr: "Parler à l'assembleur, ou de lui",
            en: "Talk to the assembler, or about it",
        },
        body: Text {
            fr: "`%note` écrit une remarque dans le listing, `%pragma` règle un détail de l'assemblage sans erreur si NASM ne le connaît pas, `%line` dit de quel fichier et de quelle ligne vient ce qui suit — ce dont se servent les programmes qui produisent de l'assembleur, pour que les messages d'erreur désignent leur source à eux. `%!` lit une variable d'environnement.",
            en: "`%note` writes a remark into the listing, `%pragma` sets an assembly detail without erroring if NASM does not know it, `%line` says which file and line what follows comes from — which programs generating assembly use, so error messages point at *their* source. `%!` reads an environment variable.",
        },
        flags: "",
        example: "%pragma list nomacro\nsection .text",
    },
    Entry {
        kind: Kind::Directive,
        names: &["org"],
        title: Text {
            fr: "Dire où le code sera chargé",
            en: "Say where the code will be loaded",
        },
        body: Text {
            fr: "Fixe l'adresse de départ que l'assembleur suppose pour calculer les étiquettes. Elle sert aux formats sans en-tête — un secteur d'amorçage, une ROM —, là où personne ne dira au programme où il a été mis. Dans un ELF, c'est l'éditeur de liens qui décide : `org` n'y a pas sa place.",
            en: "Sets the starting address the assembler assumes when working out labels. It is for formats with no header — a boot sector, a ROM — where nobody will tell the program where it was put. In an ELF the linker decides: `org` has no place there.",
        },
        flags: "",
        example: "; nasm -f bin : org 0x7c00",
    },
    Entry {
        kind: Kind::Preprocessor,
        // Les fiches se cherchent en minuscules ; NASM, lui, écrit ces
        // noms en majuscules — le corps de la fiche les donne tels
        // quels, et F1 sur l'un d'eux tombe ici quand même.
        names: &[
            "macros-prédéfinies",
            "__?line?__",
            "__?file?__",
            "__?bits?__",
            "__?date?__",
            "__?time?__",
            "__?nasm_version_id?__",
            "__?output_format?__",
        ],
        title: Text {
            fr: "Ce que NASM sait de lui-même",
            en: "What NASM knows about itself",
        },
        body: Text {
            fr: "NASM définit quelques macros avant de lire la première ligne : `__?FILE?__` et `__?LINE?__` disent où l'on se trouve dans la source, `__?BITS?__` le mode en cours, `__?DATE?__` et `__?TIME?__` le moment de l'assemblage, `__?NASM_VERSION_ID?__` la version, `__?OUTPUT_FORMAT?__` le format demandé. Elles servent à laisser une trace dans le binaire, ou à écrire une macro qui s'adapte au mode. Les points d'interrogation font partie du nom : ils empêchent qu'un programme les redéfinisse par mégarde.",
            en: "NASM defines a few macros before reading the first line: `__?FILE?__` and `__?LINE?__` say where one is in the source, `__?BITS?__` the current mode, `__?DATE?__` and `__?TIME?__` the moment of assembly, `__?NASM_VERSION_ID?__` the version, `__?OUTPUT_FORMAT?__` the requested format. They serve to leave a trace in the binary, or to write a macro that adapts to the mode. The question marks are part of the name: they stop a program from redefining them by accident.",
        },
        flags: "",
        example: "mov eax, __?LINE?__     ; le numéro de cette ligne-ci\n    mov ecx, __?BITS?__     ; 64",
    },
    Entry {
        kind: Kind::Preprocessor,
        names: &["altreg", "smartalign", "fp", "ifunc", "masm", "paquets"],
        title: Text {
            fr: "Les paquets de macros livrés avec NASM",
            en: "The macro packages shipped with NASM",
        },
        body: Text {
            fr: "`%use` charge un paquet écrit par les auteurs de NASM, sans fichier à trouver. `altreg` donne aux registres leurs noms numérotés (`r0` pour `RAX`, `r1` pour `RCX`…), `smartalign` choisit le remplissage d'`align` selon le processeur visé, `fp` fournit de quoi écrire des flottants remarquables (`__?Infinity?__`, `__?NaN?__`), `ifunc` des fonctions entières que le préprocesseur calcule (`ilog2`), `masm` imite quelques tournures de l'assembleur de Microsoft. On ne paie que ce qu'on demande : sans `%use`, rien de tout cela n'existe.",
            en: "`%use` loads a package written by NASM's authors, with no file to find. `altreg` gives registers their numbered names (`r0` for `RAX`, `r1` for `RCX`…), `smartalign` picks `align`'s padding according to the targeted processor, `fp` provides notable floating-point values (`__?Infinity?__`, `__?NaN?__`), `ifunc` integer functions the preprocessor computes (`ilog2`), `masm` imitates a few turns of phrase from Microsoft's assembler. One pays only for what one asks: without `%use`, none of it exists.",
        },
        flags: "",
        example: "%use altreg\n    mov r0, 1       ; r0, c'est RAX",
    },
    // --- Les notions -------------------------------------------
    // Ce qu'aucune instruction ne définit et que toutes supposent :
    // un registre, une section, la pile, un appel système. Un
    // manuel d'instructions qui n'explique pas ses propres mots
    // laisse le lecteur les deviner.
    Entry {
        kind: Kind::Concept,
        names: &[
            "chargement",
            "chargeur",
            "relocation",
            "bibliotheque",
            "bibliothèque",
            "libc",
            "ld.so",
            "tls",
        ],
        title: Text {
            fr: "Ce qui se passe avant la première instruction",
            en: "What happens before the first instruction",
        },
        body: Text {
            fr: "Entre « lancer un programme » et sa première instruction, le noyau a déjà beaucoup travaillé. Il lit les **segments** de l'ELF — et non ses sections — et les projette en pages entières avec leurs droits ; il met le `.bss` à zéro ; il pose une **pile** qui porte `argc`, `argv`, l'environnement et le *vecteur auxiliaire*, par lequel le programme apprend où sont ses propres en-têtes et d'où vient son hasard. Pour un exécutable indépendant de la position, l'éditeur de liens dynamique applique ensuite les **relocations** : sans elles, les tables de pointeurs du programme sont nulles. Enfin un bloc de mémoire **locale au fil d'exécution** (TLS) est posé, et la base `FS` installée dessus — c'est là que vit le canari de pile, lu par `fs:0x28`.\n\nnamsdec fait tout cela, et s'arrête là où commence le travail de `ld.so` : **aucune bibliothèque n'est chargée**. Il fournit lui-même une poignée de fonctions, écrites en Rust — afficher, lire, comparer des chaînes, demander de la mémoire —, de quoi faire tourner la plupart des programmes ; les autres s'arrêtent en se nommant. Un binaire lié statiquement, lui, n'emprunte rien et s'exécute entier.",
            en: "Between «\u{a0}run a program\u{a0}» and its first instruction, the kernel has already done a lot. It reads the ELF's **segments** — not its sections — and maps them as whole pages with their permissions; it zeroes `.bss`; it lays out a **stack** carrying `argc`, `argv`, the environment and the *auxiliary vector*, through which the program learns where its own headers are and where its randomness comes from. For a position-independent executable, the dynamic linker then applies the **relocations**: without them the program's pointer tables are all zero. Finally a block of **thread-local storage** (TLS) is laid down and the `FS` base installed on it — that is where the stack canary lives, read through `fs:0x28`.\n\nnamsdec does all of this, and stops where `ld.so`'s work begins: **no library is loaded**. It provides a handful of functions itself, written in Rust — printing, reading, comparing strings, asking for memory — enough to run most programs; the others stop by naming themselves. A statically linked binary borrows nothing and runs in full.",
        },
        flags: "",
        example: "; le canari, lu par le prologue d'une fonction protégée\n    ; mov rax, [fs:0x28]\n    nop",
    },
    Entry {
        kind: Kind::Concept,
        names: &["assemblage", "nasm", "ld", "editeur-de-liens", "lier"],
        title: Text {
            fr: "D'un texte à un programme : nasm, puis ld",
            en: "From text to program: nasm, then ld",
        },
        body: Text {
            fr: "Deux outils, deux étapes. **`nasm`** traduit le texte en *fichier objet* (`.o`) : les instructions y sont déjà en octets, mais les adresses ne sont pas encore décidées — une étiquette n'y est qu'un nom dans une table. **`ld`** assemble les objets, choisit où chaque section ira en mémoire, remplace les noms par des adresses et écrit l'ELF exécutable. C'est pourquoi un `.o` s'ouvre et se lit dans namsdec mais ne s'exécute pas : personne ne lui a encore dit où il vivrait.",
            en: "Two tools, two steps. **`nasm`** turns the text into an *object file* (`.o`): the instructions are already bytes, but no address has been decided — a label there is just a name in a table. **`ld`** puts the objects together, picks where each section will live in memory, replaces the names with addresses and writes the ELF executable. That is why a `.o` opens and reads in namsdec but does not run: nobody has told it where it would live yet.",
        },
        flags: "",
        example: "; nasm -f elf64 -o p.o p.asm ; ld -Ttext 0x400000 -o p p.o\n    nop",
    },
    Entry {
        kind: Kind::Concept,
        names: &["debordement", "débordement", "overflow"],
        title: Text {
            fr: "Le débordement de tampon",
            en: "Buffer overflow",
        },
        body: Text {
            fr: "Un tampon a une taille ; l'écriture, elle, n'en a pas connaissance. Lire soixante-quatre octets dans seize, c'est écrire quarante-huit octets sur ce qui suit — et sur la pile, ce qui suit est l'adresse de retour que `call` y a déposée. Le `ret` repart alors où l'on a écrit. Le **canari de pile** est la réponse : une valeur posée juste avant l'adresse de retour, vérifiée avant de s'en servir. L'exercice « débordement » le montre, et le motif cyclique mesure de combien ça dépasse.",
            en: "A buffer has a size; the write knows nothing of it. Reading sixty-four bytes into sixteen means writing forty-eight bytes over what follows — and on the stack, what follows is the return address `call` put there. The `ret` then leaves for wherever one wrote. The **stack canary** is the answer: a value placed just before the return address and checked before using it. The \"overflow\" exercise shows this, and the cyclic pattern measures by how much.",
        },
        flags: "",
        example: "    sub rsp, 16     ; seize octets de tampon\n    add rsp, 16",
    },
    Entry {
        kind: Kind::Concept,
        names: &["motif", "cyclique", "debruijn", "cyclic"],
        title: Text {
            fr: "Le motif cyclique",
            en: "The cyclic pattern",
        },
        body: Text {
            fr: "Une suite dont toute tranche de quatre octets n'apparaît qu'une fois — `aaaabaaacaaa…`. Donnée à manger à un programme qui déborde, elle se retrouve là où le débordement a écrit ; les quatre octets lus dans `RIP` disent alors, à eux seuls, à quelle distance du début du tampon se trouvait ce qui a été écrasé. C'est une suite de De Bruijn, et celle de namsdec est octet pour octet celle de pwntools : un décalage trouvé ici vaut ailleurs. Outils ▸ Motif cyclique.",
            en: "A sequence in which every four-byte slice appears exactly once — `aaaabaaacaaa…`. Fed to a program that overflows, it ends up wherever the overflow wrote; the four bytes read from `RIP` then say by themselves how far from the start of the buffer the overwritten thing was. It is a De Bruijn sequence, and namsdec's is byte for byte pwntools': an offset found here holds elsewhere. Tools ▸ Cyclic pattern.",
        },
        flags: "",
        example: "; Outils > Motif cyclique, puis « Vers l'entrée du programme »\n    nop",
    },
    Entry {
        kind: Kind::Concept,
        names: &["plt", "got", "liaison", "import"],
        title: Text {
            fr: "La table de liaison (PLT, GOT)",
            en: "The linkage table (PLT, GOT)",
        },
        body: Text {
            fr: "Un programme qui appelle `malloc` ne sait pas où elle sera : la bibliothèque est chargée ailleurs à chaque exécution. Il appelle donc une petite fonction à lui — une entrée de la **PLT** — qui saute à l'adresse écrite dans la **GOT**, remplie par l'éditeur de liens au chargement. C'est pour cela qu'un `call` vers la bibliothèque C reste, dans le binaire, un appel vers le binaire lui-même. namsdec retrouve le nom de chaque entrée : fenêtre Analyse ▸ Fonctions importées, et `malloc@plt` dans le listing.",
            en: "A program calling `malloc` cannot know where it will be: the library is loaded elsewhere on every run. So it calls a small function of its own — a **PLT** entry — which jumps to the address written in the **GOT**, filled in by the linker at load time. That is why a call into the C library remains, inside the binary, a call into the binary itself. namsdec recovers each entry's name: Analysis ▸ Imported functions, and `malloc@plt` in the listing.",
        },
        flags: "",
        example: "; Dans un binaire ouvert : call 0x401030 -> malloc@plt\n    nop",
    },
    Entry {
        kind: Kind::Concept,
        names: &["pwndbg", "gdb", "debogueur", "débogueur"],
        title: Text {
            fr: "Ce qu'un débogueur fait, et ce que namsdec en fait",
            en: "What a debugger does, and what namsdec does with it",
        },
        body: Text {
            fr: "Un débogueur pilote un vrai processus : il pose des points d'arrêt, lit la mémoire, modifie un registre. namsdec, lui, **simule** le processeur — d'où le retour en arrière, que gdb rend laborieux. Les gestes, eux, sont les mêmes, et portent ici les mêmes noms : `telescope` (suivre un pointeur) est la lecture « Pointeurs » de l'onglet Mémoire, `vmmap` sa carte, `search` sa recherche, `patch` son « Écrire ici » ; `cyclic` est dans Outils, `dumpargs` s'affiche devant chaque `call`, `retaddr` dans l'onglet Pile, `nextcall` dans Outils, `context` sous « Copier l'état », `checksec` dans la fenêtre d'analyse. Apprendre ici, c'est apprendre là-bas.",
            en: "A debugger drives a real process: it sets breakpoints, reads memory, changes a register. namsdec **simulates** the processor instead — hence stepping backwards, which gdb makes laborious. The gestures are the same, and carry the same names here: `telescope` (following a pointer) is the Memory tab's \"pointers\" reading, `vmmap` its map, `search` its search, `patch` its \"write here\"; `cyclic` lives in Tools, `dumpargs` shows before every `call`, `retaddr` in the Stack tab, `nextcall` in Tools, `context` under \"copy the state\", `checksec` in the analysis window. Learning here is learning there.",
        },
        flags: "",
        example: "; Outils > Motif cyclique, puis Édition > Copier l'état\n    nop",
    },
    Entry {
        kind: Kind::Concept,
        names: &["registre"],
        title: Text {
            fr: "Registre",
            en: "Register",
        },
        body: Text {
            fr: "Une case de mémoire à l'intérieur du processeur, la plus rapide qui soit : il y en a seize pour les entiers, chacune de 64 bits, et tout calcul passe par elles. `RAX`, `RBX`, `RCX`… et leurs moitiés : écrire dans `EAX` (32 bits) met à zéro le haut de `RAX`, écrire dans `AX` (16 bits) ou `AL` (8 bits) le laisse. C'est la source de bogue numéro un quand on débute.",
            en: "A slot of memory inside the processor, the fastest there is: sixteen of them for integers, 64 bits each, and every computation goes through them. `RAX`, `RBX`, `RCX`… and their halves: writing to `EAX` (32 bits) zeroes the top of `RAX`, writing to `AX` (16 bits) or `AL` (8 bits) leaves it. That is the number one source of bugs when starting out.",
        },
        flags: "",
        example: "mov rax, 1\nmov eax, 1      ; met aussi le haut de RAX à zéro",
    },
    Entry {
        kind: Kind::Concept,
        names: &["indicateur", "flag", "flags", "rflags"],
        title: Text {
            fr: "Indicateur",
            en: "Flag",
        },
        body: Text {
            fr: "Un bit que le processeur met à jour après un calcul, et que le saut suivant lit. `ZF` dit que le résultat est nul, `SF` qu'il est négatif, `CF` qu'il y a eu une retenue (comparaison non signée), `OF` un débordement (comparaison signée). On ne les écrit presque jamais à la main : on les produit avec `cmp` ou `test`, et on les consomme avec un `jcc`.",
            en: "A bit the processor updates after a computation, which the next jump reads. `ZF` says the result is zero, `SF` that it is negative, `CF` that there was a carry (unsigned comparison), `OF` an overflow (signed comparison). One almost never writes them by hand: they are produced with `cmp` or `test` and consumed by a `jcc`.",
        },
        flags: "",
        example: "cmp rax, rbx\nje egaux        ; saute si ZF vaut 1\negaux:",
    },
    Entry {
        kind: Kind::Concept,
        names: &["pile", "stack"],
        title: Text {
            fr: "La pile",
            en: "The stack",
        },
        body: Text {
            fr: "Une zone de mémoire où l'on empile et dépile, et qui grandit vers les **adresses basses** : `push` soustrait 8 de `RSP` puis écrit, `pop` lit puis ajoute 8. `call` y dépose l'adresse de retour, `ret` la reprend. Ce qui est au-dessus de `RSP` a été dépilé : les octets sont encore là, mais le prochain `push` les écrase.",
            en: "A region of memory one pushes onto and pops from, growing towards **lower addresses**: `push` subtracts 8 from `RSP` then writes, `pop` reads then adds 8. `call` leaves the return address there, `ret` takes it back. What sits above `RSP` has been popped: the bytes are still there, but the next push overwrites them.",
        },
        flags: "",
        example: "push rax\npop rbx         ; RBX reçoit ce que RAX contenait",
    },
    Entry {
        kind: Kind::Concept,
        names: &["tas", "heap", "brk"],
        title: Text {
            fr: "Le tas",
            en: "The heap",
        },
        body: Text {
            fr: "La mémoire qu'un programme demande au noyau pendant qu'il tourne, par l'appel système `brk` : une frontière qu'on déplace, et tout ce qui est en deçà appartient au programme. C'est sur cette frontière que sont bâtis les `malloc` des bibliothèques — il n'y a rien d'autre dessous.",
            en: "The memory a program asks the kernel for while running, through the `brk` system call: a boundary one moves, and everything below it belongs to the program. The `malloc` of the libraries is built on that boundary — there is nothing else underneath.",
        },
        flags: "",
        example: "mov rax, 12\nmov rdi, 0      ; brk(0) rend la frontière courante\nsyscall",
    },
    Entry {
        kind: Kind::Concept,
        names: &["notion-section", "sections"],
        title: Text {
            fr: "Section",
            en: "Section",
        },
        body: Text {
            fr: "Un morceau du programme, rangé selon ce qu'on en fait : `.text` contient le code et s'exécute sans pouvoir être écrit, `.data` les données modifiables, `.rodata` les constantes, `.bss` la place réservée mais non initialisée — elle ne pèse rien dans le fichier. Écrire dans une section en lecture seule est refusé à l'exécution.",
            en: "A piece of the program, filed by what one does with it: `.text` holds the code and runs without being writable, `.data` the changeable data, `.rodata` the constants, `.bss` space reserved but uninitialised — it weighs nothing in the file. Writing to a read-only section is refused at run time.",
        },
        flags: "",
        example: "section .data\nmessage: db \"salut\", 10\n\nsection .text",
    },
    Entry {
        kind: Kind::Concept,
        names: &["etiquette", "label"],
        title: Text {
            fr: "Étiquette",
            en: "Label",
        },
        body: Text {
            fr: "Un nom donné à une adresse. L'assembleur la remplace par un nombre : `jmp boucle` devient un saut vers une adresse, et `mov rsi, message` charge l'adresse de la donnée — pas la donnée elle-même. Pour lire ce qu'il y a dedans, il faut des crochets : `mov al, [message]`.",
            en: "A name given to an address. The assembler replaces it by a number: `jmp boucle` becomes a jump to an address, and `mov rsi, message` loads the address of the data — not the data. To read what is in it, brackets are needed: `mov al, [message]`.",
        },
        flags: "",
        example: "mov rsi, message   ; l'adresse\nmov al, [message]  ; l'octet\nmessage: db 65",
    },
    Entry {
        kind: Kind::Concept,
        names: &["appel", "notion-syscall", "appel système", "noyau"],
        title: Text {
            fr: "Appel système",
            en: "System call",
        },
        body: Text {
            fr: "La seule façon pour un programme de parler au monde extérieur : lire, écrire, obtenir de la mémoire, s'arrêter. Le numéro va dans `RAX`, les arguments dans `RDI`, `RSI`, `RDX`, `R10`, `R8`, `R9` — et non `RCX`, que l'instruction `syscall` écrase —, et la réponse revient dans `RAX`.",
            en: "The only way a program has of talking to the outside world: read, write, get memory, stop. The number goes in `RAX`, the arguments in `RDI`, `RSI`, `RDX`, `R10`, `R8`, `R9` — not `RCX`, which the `syscall` instruction clobbers — and the answer comes back in `RAX`.",
        },
        flags: "",
        example: "mov rax, 60     ; exit\nxor edi, edi    ; code 0\nsyscall",
    },
    Entry {
        kind: Kind::Concept,
        names: &["convention", "abi", "system v"],
        title: Text {
            fr: "Convention d'appel",
            en: "Calling convention",
        },
        body: Text {
            fr: "L'accord entre programmes sur qui met quoi où : les six premiers arguments dans `RDI`, `RSI`, `RDX`, `RCX`, `R8`, `R9`, le résultat dans `RAX`. `RBX`, `RBP` et `R12` à `R15` doivent être rendus intacts par la fonction appelée ; les autres, l'appelant les considère perdus. Le processeur n'en sait rien : rien ne l'impose, c'est un accord.",
            en: "The agreement between programs about who puts what where: the first six arguments in `RDI`, `RSI`, `RDX`, `RCX`, `R8`, `R9`, the result in `RAX`. `RBX`, `RBP` and `R12`–`R15` must be handed back untouched by the callee; the caller writes the others off. The processor knows nothing of this: nothing enforces it, it is an agreement.",
        },
        flags: "",
        example: "mov rdi, 1      ; premier argument\ncall ecrire\nhlt\necrire:\nret",
    },
    Entry {
        kind: Kind::Concept,
        names: &["adressage", "crochets"],
        title: Text {
            fr: "Adressage mémoire",
            en: "Memory addressing",
        },
        body: Text {
            fr: "Ce qu'il y a entre crochets : `[base + indice*taille + décalage]`. Le processeur fait le calcul pour vous, à condition que la taille soit 1, 2, 4 ou 8 — c'est ce qui fait exister les tableaux. Sans crochets, on manipule le nombre lui-même ; avec, on manipule ce qui se trouve à cette adresse.",
            en: "What goes between brackets: `[base + index*size + displacement]`. The processor does the arithmetic for you, provided the size is 1, 2, 4 or 8 — that is what makes arrays exist. Without brackets one handles the number itself; with them, what sits at that address.",
        },
        flags: "",
        example: "mov rax, [rsi + rcx*8]",
    },
    Entry {
        kind: Kind::Concept,
        names: &["boutisme", "endianness", "petit-boutiste"],
        title: Text {
            fr: "Ordre des octets",
            en: "Byte order",
        },
        body: Text {
            fr: "Sur x86, un nombre est écrit en mémoire **octet de poids faible en premier** : `0x1122` occupe les octets `22 11`. C'est pourquoi un vidage hexadécimal ne se lit pas comme le nombre qu'on a écrit, et pourquoi une chaîne, elle, se lit normalement — ses octets ne forment pas un nombre.",
            en: "On x86 a number is written to memory **least significant byte first**: `0x1122` takes up the bytes `22 11`. That is why a hex dump does not read like the number one wrote, and why a string does read normally — its bytes do not make up a number.",
        },
        flags: "",
        example: "dw 0x1122       ; en mémoire : 22 11",
    },
    Entry {
        kind: Kind::Concept,
        names: &["immediat", "immédiat"],
        title: Text {
            fr: "Valeur immédiate",
            en: "Immediate value",
        },
        body: Text {
            fr: "Un nombre écrit dans l'instruction elle-même, plutôt que lu ailleurs. Il fait partie des octets du code : c'est pour cela qu'il est limité — une écriture en mémoire n'accepte pas d'immédiat de plus de 32 bits, et il faut passer par un registre.",
            en: "A number written inside the instruction itself, rather than read from elsewhere. It is part of the code bytes: that is why it is limited — a memory write takes no immediate wider than 32 bits, and one has to go through a register.",
        },
        flags: "",
        example: "mov rax, 42     ; 42 est l'immédiat",
    },
    Entry {
        kind: Kind::Concept,
        names: &["point d'arret", "breakpoint"],
        title: Text {
            fr: "Point d'arrêt",
            en: "Breakpoint",
        },
        body: Text {
            fr: "Une ligne où l'exécution s'arrête pour qu'on regarde. Dans namsdec, un clic dans la gouttière la pose ; une condition la rend plus utile encore — dans une boucle de mille tours, on ne veut pas s'arrêter mille fois mais au tour où quelque chose va de travers.",
            en: "A line where execution stops so one can look. In namsdec a click in the gutter sets one; a condition makes it more useful still — in a thousand-turn loop one does not want to stop a thousand times, but at the turn where something goes wrong.",
        },
        flags: "",
        example: "; clic dans la gouttière, puis Outils > Points d'arrêt",
    },
    Entry {
        kind: Kind::Concept,
        names: &["desassemblage", "désassemblage", "disassembly"],
        title: Text {
            fr: "Désassemblage",
            en: "Disassembly",
        },
        body: Text {
            fr: "L'opération inverse de l'assemblage : retrouver les instructions à partir des octets. Elle n'est jamais parfaite — rien ne distingue du code de données dans un fichier, et commencer la lecture un octet plus loin donne d'autres instructions, parfaitement valables et complètement fausses.",
            en: "The reverse of assembling: recovering instructions from bytes. It is never perfect — nothing distinguishes code from data in a file, and starting one byte further along yields other instructions, perfectly valid and completely wrong.",
        },
        flags: "",
        example: "; Outils > Désassemblage",
    },
    Entry {
        kind: Kind::Concept,
        names: &["protection", "nx", "pie", "relro", "canari"],
        title: Text {
            fr: "Protections d'un binaire",
            en: "Binary hardening",
        },
        body: Text {
            fr: "Les défenses qu'un programme emporte avec lui : `NX` (la pile ne s'exécute pas), `PIE` (les adresses changent à chaque exécution), `RELRO` (les tables de liaison passent en lecture seule), le canari de pile (une valeur secrète devant l'adresse de retour). Aucune n'empêche un bogue ; toutes rendent son exploitation plus difficile.",
            en: "The defences a program carries with it: `NX` (the stack does not execute), `PIE` (addresses change on every run), `RELRO` (linking tables become read-only), the stack canary (a secret value in front of the return address). None prevents a bug; all make exploiting one harder.",
        },
        flags: "",
        example: "; Outils > Analyse du binaire",
    },
    Entry {
        kind: Kind::Concept,
        names: &["elf", "binaire"],
        title: Text {
            fr: "Un binaire ELF",
            en: "An ELF binary",
        },
        body: Text {
            fr: "Le format des exécutables sous Linux : un en-tête qui dit par où commencer, des segments que le noyau projette en mémoire avec leurs droits, et des tables — symboles, relocations — dont beaucoup disparaissent quand le binaire est dépouillé. Ce qui reste suffit à l'exécuter ; ce qui manque est ce qu'on reconstitue en le lisant.",
            en: "The executable format on Linux: a header saying where to start, segments the kernel maps into memory with their rights, and tables — symbols, relocations — many of which disappear when the binary is stripped. What remains is enough to run it; what is missing is what one reconstructs by reading it.",
        },
        flags: "",
        example: "; Outils > Analyse du binaire",
    },
    Entry {
        kind: Kind::Concept,
        names: &[
            "arrondi",
            "rounding",
            "precision",
            "précision",
            "mot-de-controle",
            "control-word",
        ],
        title: Text {
            fr: "L'arrondi, la précision, et ce que namsdec simplifie",
            en: "Rounding, precision, and what namsdec simplifies",
        },
        body: Text {
            fr: "Quatre modes, et le programme choisit. Les bits 10 et 11 du **mot de contrôle** du x87 disent comment arrondir : **au plus proche** — et l'exacte moitié vers le pair, 00 —, **vers moins l'infini** (01), **vers plus l'infini** (10), **vers zéro** (11). Aucun n'est plus juste que les autres : `fistp` de 3,5 rend 4 au plus proche et 3 par troncature, et les deux sont corrects. C'est la première chose qui surprend quand on compare deux programmes qui calculent « la même chose ».\n\nLes bits 8 et 9 fixent la **précision** : 24 bits de mantisse (le format simple), 53 (le double), ou 64 — le format étendu du x87, et le réglage par défaut. Le mot au repos vaut donc 0x037f, et `finit` le remet là. C'est la raison d'une vieille bizarrerie : un même calcul écrit en C pouvait changer de dernier chiffre selon qu'une valeur intermédiaire restait dans la pile du coprocesseur — à 64 bits — ou passait par la mémoire en 53. SSE a réglé la question en calculant toujours à la largeur du format.\n\n**Ce que namsdec simule, et ce qu'il simplifie.** Le mot de contrôle est un vrai registre : ce qu'on y pose se relit, et l'arrondi gouverne `fist`, `fistp` et `frndint` — `fisttp` tronche toujours, c'est sa raison d'être. À 53 bits, namsdec **est** le matériel : il calcule nativement en double. À 24 bits, il arrondit chaque opération de base au format simple. Restent sept simplifications, et les voici plutôt que de les taire : (1) la précision **étendue** est approximée par la double — le vrai x87 garde onze bits de mantisse de plus, et un calcul qui les exploite diverge ; (2) un `fstp tword` écrit un 80 bits dont les onze derniers bits sont nuls ; (3) les transcendantes — `fsin`, `fcos`, `fptan`, `fpatan`, `fyl2x`, `f2xm1` — passent par la bibliothèque de Rust et non par le polynôme du coprocesseur ; (4) à 24 bits, l'arrondi au format simple suit le plus proche même en mode dirigé ; (5) aucune exception flottante n'est levée, donc démasquer les bits 0 à 5 ne change rien ; (6) `fstsw` rend les bits de condition et le sommet, pas les bits d'exception ; (7) `fprem` rend le reste complet du premier coup, là où le processeur peut s'arrêter en chemin et le dire par C2.\n\nPendant une exécution, namsdec signale dans la console celles que le programme touche vraiment — une fois chacune.",
            en: "Four modes, and the program picks. Bits 10 and 11 of the x87 **control word** say how to round: **to nearest** — ties to even, 00 —, **toward minus infinity** (01), **toward plus infinity** (10), **toward zero** (11). None is more correct than the others: `fistp` of 3.5 gives 4 to nearest and 3 by truncation, and both are right. That is the first thing that surprises you when comparing two programs that compute «\u{a0}the same thing\u{a0}».\n\nBits 8 and 9 set the **precision**: 24 mantissa bits (single format), 53 (double), or 64 — the x87's extended format, and the default setting. The word at rest therefore reads 0x037f, and `finit` puts it back there. This explains an old oddity: one and the same C computation could change its last digit depending on whether an intermediate value stayed in the coprocessor stack — at 64 bits — or went through memory at 53. SSE settled the question by always computing at the format's width.\n\n**What namsdec simulates, and what it simplifies.** The control word is a real register: what you put there reads back, and the rounding mode governs `fist`, `fistp` and `frndint` — `fisttp` always truncates, that is its whole point. At 53 bits namsdec **is** the hardware: it computes natively in double. At 24 bits it rounds every basic operation to single format. Seven simplifications remain, and here they are rather than left unsaid: (1) **extended** precision is approximated by double — a real x87 keeps eleven more mantissa bits, and a computation that exploits them diverges; (2) an `fstp tword` writes an 80-bit value whose last eleven bits are zero; (3) the transcendentals — `fsin`, `fcos`, `fptan`, `fpatan`, `fyl2x`, `f2xm1` — go through Rust's library rather than the coprocessor's polynomial; (4) at 24 bits, rounding to single format follows nearest even in a directed mode; (5) no floating-point exception is raised, so unmasking bits 0 to 5 changes nothing; (6) `fstsw` returns the condition bits and the top index, not the exception bits; (7) `fprem` returns the complete remainder on the first try, where the processor may stop partway and say so through C2.\n\nDuring a run, namsdec reports in the console the ones the program actually touches — once each.",
        },
        flags: "",
        example: "fnstcw [ancien]\n    fldcw [vers_zero]    ; 0x0f7f : vers zéro\n    fld qword [x]        ; 1,5\n    fistp qword [entier] ; 1, et 2 au plus proche\n    fldcw [ancien]",
    },
];
