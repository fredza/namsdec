//! La table des fiches, une par famille d'instructions.
//!
//! Écrite à la main et rangée par sujet : déplacer, calculer, brancher,
//! puis les chaînes, le x87 et le vectoriel. L'ordre n'a d'importance
//! que pour qui relit le fichier — la recherche passe par un index.

use crate::{Entry, Text};

pub const ENTRIES: &[Entry] = &[
    Entry {
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
        names: &["pushf", "pushfq", "popf", "popfq"],
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
        example: "mov rdx, a\nmulx haut, bas, b",
    },
    Entry {
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
        example: "andn rax, masque, valeur",
    },
    Entry {
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
        names: &["nop", "pause", "endbr64", "mfence", "lfence", "sfence"],
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
        names: &["fstsw", "fnstsw", "fstcw", "fnstcw", "fldcw"],
        title: Text {
            fr: "Le mot d'état, le mot de contrôle",
            en: "The status and control words",
        },
        body: Text {
            fr: "`fstsw` rend le mot d'état — où vivent les bits de condition et le numéro du sommet. `fstcw`/`fldcw` lisent et écrivent le mot de contrôle, qui fixe la précision et le mode d'arrondi ; namsdec calcule toujours au plus proche, et accepte l'écriture sans en tenir compte.",
            en: "`fstsw` returns the status word — home of the condition bits and the top-of-stack index. `fstcw`/`fldcw` read and write the control word, which sets precision and rounding mode; namsdec always rounds to nearest, and accepts the write without acting on it.",
        },
        flags: "",
        example: "fstsw ax",
    },
    Entry {
        names: &[
            "finit", "fninit", "ffree", "ffreep", "fincstp", "fdecstp", "fnop", "fwait",
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
        names: &[
            "movaps", "movapd", "movups", "movupd", "movdqa", "movdqu", "vmovaps", "vmovapd",
            "vmovups", "vmovupd", "vmovdqa", "vmovdqu",
        ],
        title: Text {
            fr: "Déplacer tout le registre",
            en: "Move the whole register",
        },
        body: Text {
            fr: "Copient 128 ou 256 bits d'un coup. Les formes en `a` exigent une adresse alignée sur la taille du transfert, les formes en `u` non — sur un processeur récent la différence de vitesse a disparu, l'exception d'alignement non.",
            en: "Copy 128 or 256 bits at once. The `a` forms require an address aligned to the transfer size, the `u` forms do not — on a recent processor the speed difference is gone, the alignment fault is not.",
        },
        flags: "",
        example: "movaps xmm0, [vecteur]\nvmovups ymm1, [non_aligné]",
    },
    Entry {
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
        names: &[
            "xorps", "xorpd", "pxor", "andps", "andpd", "pand", "orps", "orpd", "por", "andnps",
            "andnpd", "pandn", "vxorps", "vxorpd", "vpxor", "vandps", "vandpd", "vpand", "vorps",
            "vorpd", "vpor", "vandnps", "vandnpd", "vpandn",
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
];
