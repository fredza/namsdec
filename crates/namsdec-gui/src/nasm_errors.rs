//! Ce que les messages de nasm et de l'éditeur de liens veulent dire.
//!
//! Un débutant est bloqué au moment précis où l'assembleur refuse, et ce
//! qu'il lit alors est une phrase anglaise de six mots écrite pour
//! quelqu'un qui connaît déjà la réponse : « invalid combination of
//! opcode and operands ». Le message brut reste — c'est lui qu'on colle
//! dans une recherche, et il ne faut pas le cacher —, mais une phrase
//! dessous dit ce qui se passe et par où commencer.
//!
//! La reconnaissance se fait sur un morceau stable du message, pas sur
//! la phrase entière : nasm y insère des noms, des nombres et une
//! traduction, et une correspondance exacte ne survivrait pas à une
//! version de plus.

use crate::i18n::Lang;

/// Une explication : ce qui se passe, puis quoi faire.
struct Explanation {
    /// Le morceau du message de nasm qui identifie le cas, en minuscules.
    needle: &'static str,
    fr: &'static str,
    en: &'static str,
}

const EXPLANATIONS: &[Explanation] = &[
    Explanation {
        needle: "instruction expected",
        fr: "Ce mot n'est pas une instruction connue : une faute de frappe (`mv` pour `mov`), ou une étiquette à laquelle il manque ses deux-points. Une ligne d'assembleur commence par une étiquette, une instruction ou une directive, jamais par autre chose.",
        en: "That word is not a known instruction: a typo (`mv` for `mov`), or a label missing its colon. A line of assembly starts with a label, an instruction or a directive, never with anything else.",
    },
    Explanation {
        needle: "initialize memory in bss",
        fr: "`.bss` est la section des données *non initialisées* : elle n'occupe rien dans le fichier, seulement en mémoire. On y réserve de la place avec `resb`, `resw`, `resq` — pas avec `db`, qui écrit des octets. Ce que vous donnez ici est ignoré.",
        en: "`.bss` is the section of *uninitialised* data: it takes no room in the file, only in memory. Space is reserved there with `resb`, `resw`, `resq` — not with `db`, which writes bytes. What you gave here is ignored.",
    },
    Explanation {
        needle: "exceeds bounds",
        fr: "La valeur ne tient pas dans la taille demandée : 300 ne tient pas dans un octet (0 à 255), et un déplacement d'adresse ne tient que sur 32 bits. Choisissez une taille plus large, ou passez par un registre.",
        en: "The value does not fit the size asked for: 300 does not fit in a byte (0 to 255), and an address displacement only holds 32 bits. Use a wider size, or go through a register.",
    },
    Explanation {
        needle: "times value",
        fr: "`times` répète ce qui suit, et ne peut donc pas répéter un nombre négatif de fois. Le calcul qui donne ce nombre est faux — souvent un `$ - étiquette` où l'étiquette est déclarée après, ou dans une autre section.",
        en: "`times` repeats what follows, so it cannot repeat a negative number of times. The expression giving that number is wrong — often a `$ - label` where the label comes later, or lives in another section.",
    },
    Explanation {
        needle: "unterminated string",
        fr: "Il manque le guillemet fermant. NASM lit jusqu'à la fin de la ligne et ce qui suit devient une chaîne, d'où les erreurs qui s'enchaînent en dessous.",
        en: "The closing quote is missing. NASM reads to the end of the line and what follows becomes part of the string, hence the errors that pile up below.",
    },
    Explanation {
        needle: "inconsistently redefined",
        fr: "Cette étiquette est définie deux fois. Chaque nom ne désigne qu'un endroit : renommez l'une des deux, ou supprimez la copie — on tombe souvent dessus après avoir dupliqué un bloc.",
        en: "That label is defined twice. Each name designates one place: rename one of them, or remove the copy — this usually turns up after duplicating a block.",
    },
    Explanation {
        needle: "invalid 64-bit effective address",
        fr: "Cette forme d'adresse n'existe pas. Le facteur d'échelle ne peut être que 1, 2, 4 ou 8 — `[rbx + rcx*3]` est refusé —, et l'on ne peut pas mettre deux bases. Décomposez le calcul avec un `lea` ou une addition.",
        en: "That address form does not exist. The scale can only be 1, 2, 4 or 8 — `[rbx + rcx*3]` is refused — and there cannot be two bases. Break the computation up with a `lea` or an addition.",
    },
    Explanation {
        needle: "relocation truncated to fit",
        fr: "L'éditeur de liens ne peut pas écrire cette adresse : elle ne tient pas dans les 32 bits que l'instruction lui laisse. C'est le signe d'un adressage absolu là où il faut du relatif — `default rel` en tête du fichier, ou `lea rax, [étiquette]` plutôt que `mov rax, étiquette`.",
        en: "The linker cannot write that address: it does not fit in the 32 bits the instruction leaves it. That is the sign of absolute addressing where relative is needed — `default rel` at the top of the file, or `lea rax, [label]` instead of `mov rax, label`.",
    },
    Explanation {
        needle: "multiple definition",
        fr: "Ce symbole est défini dans deux fichiers à la fois. Un seul programme, un seul `_start` : c'est l'éditeur de liens qui le dit, et il ne peut pas choisir à votre place.",
        en: "That symbol is defined in two files at once. One program, one `_start`: the linker is saying so, and it cannot choose for you.",
    },
    Explanation {
        needle: "not defined",
        fr: "Ce nom n'existe nulle part dans le programme : une étiquette mal orthographiée, ou une section oubliée. Vérifiez l'orthographe, et que l'étiquette est bien suivie de deux-points là où elle est définie.",
        en: "That name exists nowhere in the program: a misspelt label, or a forgotten section. Check the spelling, and that the label is followed by a colon where it is defined.",
    },
    Explanation {
        needle: "invalid combination of opcode and operands",
        fr: "L'instruction existe, mais pas avec ces opérandes-là. Les causes habituelles : deux opérandes en mémoire (le x86 n'en accepte qu'une), des tailles qui ne correspondent pas (`mov rax, ebx`), ou un immédiat là où il faut un registre. F1 sur l'instruction montre ses formes.",
        en: "The instruction exists, but not with those operands. The usual causes: two memory operands (x86 allows only one), mismatched sizes (`mov rax, ebx`), or an immediate where a register is required. F1 on the instruction shows its forms.",
    },
    Explanation {
        needle: "invalid operand sizes",
        fr: "Les deux opérandes n'ont pas la même taille, ou celle de la mémoire n'est pas dite. `movzx rax, [rbx]` ne dit pas combien d'octets lire : écrivez `movzx rax, byte [rbx]`. Et `mov rax, ebx` mélange 64 et 32 bits.",
        en: "The two operands do not have the same size, or memory's size is not stated. `movzx rax, [rbx]` does not say how many bytes to read: write `movzx rax, byte [rbx]`. And `mov rax, ebx` mixes 64 and 32 bits.",
    },
    Explanation {
        needle: "operation size not specified",
        fr: "L'adresse seule ne dit pas combien d'octets écrire. Précisez-le : `mov byte [x], 1`, `mov qword [x], 1`. NASM refuse de deviner, et il a raison — un octet et huit octets, ce n'est pas le même programme.",
        en: "The address alone does not say how many bytes to write. Say it: `mov byte [x], 1`, `mov qword [x], 1`. NASM refuses to guess, and rightly so — one byte and eight bytes are not the same program.",
    },
    Explanation {
        needle: "high byte register",
        fr: "AH, BH, CH et DH ne peuvent pas cohabiter avec R8 à R15, SIL, DIL, SPL ou BPL dans la même instruction : ces registres-là exigent un préfixe REX, qui rend les octets hauts inaccessibles. Passez par AL, ou par un autre registre.",
        en: "AH, BH, CH and DH cannot share an instruction with R8–R15, SIL, DIL, SPL or BPL: those registers need a REX prefix, which makes the high bytes unreachable. Go through AL, or another register.",
    },
    Explanation {
        needle: "signed dword value exceeds bounds",
        fr: "Une écriture en mémoire ne peut pas porter un immédiat de plus de 32 bits. Passez par un registre : `mov rax, 0x1122334455667788` puis `mov [x], rax`.",
        en: "A memory write cannot carry an immediate wider than 32 bits. Go through a register: `mov rax, 0x1122334455667788` then `mov [x], rax`.",
    },
    Explanation {
        needle: "implicit default abs",
        fr: "nasm demande qu'on dise quel adressage on veut. Ajoutez `default rel` en tête du fichier : c'est l'adressage relatif à RIP, celui qu'on écrit aujourd'hui et qu'exige un exécutable relogeable.",
        en: "nasm wants you to say which addressing you mean. Add `default rel` at the top of the file: RIP-relative addressing, what everyone writes today and what a relocatable executable requires.",
    },
    Explanation {
        needle: "invalid effective address",
        fr: "Cette expression d'adresse n'est pas calculable par le processeur. Une adresse s'écrit `[base + index*n + décalage]`, où n vaut 1, 2, 4 ou 8, et où base et index sont des registres — pas deux additions de registres multipliés.",
        en: "The processor cannot compute that address expression. An address is `[base + index*n + displacement]`, where n is 1, 2, 4 or 8, and base and index are registers — not two scaled registers added together.",
    },
    Explanation {
        needle: "not supported in 64-bit mode",
        fr: "Cette instruction n'existe qu'en 16 ou 32 bits : `pusha`, `popa`, `into`, les formes 32 bits de `push`… Le mode 64 bits les a retirées. Cherchez l'équivalent moderne — souvent une suite de `push` explicites.",
        en: "That instruction only exists in 16- or 32-bit mode: `pusha`, `popa`, `into`, the 32-bit forms of `push`… 64-bit mode dropped them. Look for the modern equivalent — often a sequence of explicit pushes.",
    },
    Explanation {
        needle: "label or instruction expected",
        fr: "nasm ne reconnaît pas le premier mot de la ligne. Souvent une faute de frappe dans un mnémonique, une étiquette sans deux-points, ou une directive écrite sans son `%`.",
        en: "nasm does not recognise the first word on the line. Often a typo in a mnemonic, a label without its colon, or a directive written without its `%`.",
    },
    Explanation {
        needle: "symbol `_start' is undefined",
        fr: "L'éditeur de liens cherche `_start` et ne le trouve pas. Il faut les deux lignes : `global _start` pour le rendre visible, et `_start:` là où le programme commence.",
        en: "The linker looks for `_start` and cannot find it. Both lines are needed: `global _start` to make it visible, and `_start:` where the program begins.",
    },
    Explanation {
        needle: "entry symbol _start",
        fr: "Le programme ne déclare pas où il commence, et l'éditeur de liens a choisi une adresse à votre place. Ajoutez `global _start` et l'étiquette `_start:`.",
        en: "The program does not declare where it starts, so the linker picked an address for you. Add `global _start` and the `_start:` label.",
    },
    Explanation {
        needle: "undefined reference",
        fr: "Un nom est utilisé mais défini nulle part à l'édition de liens. Si c'est une fonction de la bibliothèque C, namsdec ne l'édite pas avec : il lie seul, avec `ld`, et ne connaît que les quatre appels système qu'il simule.",
        en: "A name is used but defined nowhere at link time. If it is a C library function, namsdec does not link against it: it links alone, with `ld`, and knows only the four system calls it simulates.",
    },
    Explanation {
        needle: "comma expected",
        fr: "Il manque une virgule entre deux opérandes, ou il y en a une de trop. En assembleur, la destination vient d'abord, puis la source : `mov rax, rbx` met RBX dans RAX.",
        en: "A comma is missing between two operands, or there is one too many. In assembly the destination comes first, then the source: `mov rax, rbx` puts RBX into RAX.",
    },
    Explanation {
        needle: "expression syntax error",
        fr: "L'expression ne se lit pas : une parenthèse qui manque, un opérateur en trop, ou un nombre mal écrit — en hexadécimal, NASM veut `0x1f` ou `1Fh`.",
        en: "The expression does not parse: a missing parenthesis, an extra operator, or a badly written number — in hexadecimal, NASM wants `0x1f` or `1Fh`.",
    },
    Explanation {
        needle: "redefined",
        fr: "Ce nom est défini deux fois. Deux étiquettes ne peuvent pas porter le même nom dans un fichier ; pour une étiquette locale à une routine, préfixez-la d'un point : `.boucle`.",
        en: "That name is defined twice. Two labels cannot share a name in one file; for a label local to a routine, prefix it with a dot: `.boucle`.",
    },
];

/// L'explication qui correspond à ce message, s'il y en a une.
pub fn explain(lang: Lang, message: &str) -> Option<&'static str> {
    let lower = message.to_lowercase();
    EXPLANATIONS
        .iter()
        .find(|e| lower.contains(e.needle))
        .map(|e| if lang == Lang::Fr { e.fr } else { e.en })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les messages tels que nasm et ld les écrivent vraiment — relevés
    /// en les provoquant, pas inventés.
    #[test]
    fn the_messages_one_actually_gets_are_explained() {
        for message in [
            "symbol `mesage' not defined",
            "invalid combination of opcode and operands",
            "operation size not specified",
            "invalid operand sizes for instruction",
            "cannot use high byte register in this instruction",
            "signed dword value exceeds bounds",
            "implicit DEFAULT ABS is deprecated [-w+implicit-abs-deprecated]",
            "invalid effective address",
            "instruction not supported in 64-bit mode",
            "label or instruction expected at start of line",
            "/usr/bin/ld: warning: cannot find entry symbol _start; defaulting to 0000000000400080",
            "undefined reference to `printf'",
            "label `boucle' inconsistently redefined",
            // Relevés sur nasm 3.02 en provoquant chaque faute.
            "instruction expected, found `mv rax'",
            "attempt to initialize memory in BSS section `.bss': ignored [-w+other]",
            "byte exceeds bounds [-w+number-overflow]",
            "dword displacement exceeds bounds [-w+number-overflow]",
            "TIMES value -1 is negative",
            "unterminated string (missing `\"') [-w+pp-open-string]",
            "invalid 64-bit effective address",
            "/usr/bin/ld: relocation truncated to fit: R_X86_64_32S against `.data'",
            "/usr/bin/ld: multiple definition of `_start'",
        ] {
            assert!(
                explain(Lang::Fr, message).is_some(),
                "sans explication : {message}"
            );
            assert!(explain(Lang::En, message).is_some(), "{message}");
        }
    }

    /// Chaque explication doit dire quoi faire, pas seulement traduire
    /// le message : c'est ce qui la distingue d'un dictionnaire.
    #[test]
    fn every_explanation_says_what_to_do() {
        for explanation in EXPLANATIONS {
            for texte in [explanation.fr, explanation.en] {
                assert!(
                    texte.len() > 80,
                    "trop courte pour expliquer quoi que ce soit : {texte}"
                );
            }
            assert_ne!(
                explanation.fr, explanation.en,
                "les deux langues doivent dire la même chose autrement"
            );
            // Le repère se cherche en minuscules : un `needle` qui en
            // contient d'autres ne trouverait jamais rien.
            assert_eq!(
                explanation.needle,
                explanation.needle.to_lowercase(),
                "le repère doit être en minuscules"
            );
        }
    }

    /// Un message inconnu n'invente rien : mieux vaut le message brut
    /// seul qu'une explication qui tombe à côté.
    #[test]
    fn an_unknown_message_gets_no_invented_explanation() {
        assert_eq!(
            explain(Lang::Fr, "quelque chose de tout à fait nouveau"),
            None
        );
    }
}
