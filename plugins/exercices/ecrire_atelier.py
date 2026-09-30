# -*- coding: utf-8 -*-
"""Écrit la seconde série d'exercices, et rien d'autre.

    python3 plugins/exercices/ecrire_atelier.py plugins/exercices/atelier.json

Un fichier Python plutôt qu'un JSON tapé à la main : les programmes
portent des retours à la ligne et des guillemets, et les échapper à la
main est le meilleur moyen de livrer un exercice qui ne s'assemble pas.
C'est donc ce fichier-ci qu'on corrige, et le JSON qu'on régénère.

Les budgets, en bas, sont MESURÉS et non devinés : ils viennent de ce
qu'exécute chaque solution livrée. Le test
`the_workshop_exercises_are_complete_and_reach_their_goal` assemble les
dix-huit, les exécute, et refuse un but que le code donné n'atteint pas
— un exercice dont l'énoncé ment apprend la mauvaise chose.
"""
import json, sys

E = []

def exo(id, fr, en, desc_fr, desc_en, code, goal, par, level):
    E.append({
        "id": id,
        "title": {"fr": fr, "en": en},
        "description": {"fr": desc_fr, "en": desc_en},
        "code": code,
        "goal": goal,
        "par": par,
        "level": level,
    })

# ─────────────────────────────────────────────────────────────────────
#  STRUCTURES — ce qu'on construit
# ─────────────────────────────────────────────────────────────────────

exo("matrice", "Une matrice, ligne par ligne",
    "A matrix, row by row",
    "Un tableau à deux dimensions n'existe pas : il y a un tableau, et une multiplication. `[base + ligne*largeur + colonne]` est tout ce que le processeur connaît d'une matrice. À l'arrêt, RAX doit valoir 45 — la somme de la diagonale.",
    "A two-dimensional array does not exist: there is an array, and a multiplication. `[base + row*width + column]` is all the processor knows about a matrix. At the halt, RAX must hold 45 — the sum of the diagonal.",
    """; Une matrice 3x3, et sa diagonale.
;
; But : à l'arrêt, RAX vaut 45 — 1 + 5 + 9, mais en octets : 5 + 15 + 25.
;
; Un tableau à deux dimensions n'existe pas pour le processeur. Il y a
; des octets à la file, et une multiplication : l'élément (ligne,
; colonne) vit à `base + ligne*largeur + colonne`.
;
; La diagonale est le cas où ligne == colonne, donc l'indice avance de
; `largeur + 1` à chaque pas. C'est pourquoi on la parcourt sans deux
; compteurs.

default rel
global _start

section .data
    m       dq  5, 10, 15
            dq  20, 15, 30
            dq  35, 40, 25
    largeur equ 3

section .text
_start:
    lea rsi, [m]
    xor rax, rax            ; la somme
    xor rcx, rcx            ; l'indice dans la diagonale : 0, 4, 8

diagonale:
    add rax, [rsi + rcx*8]
    add rcx, largeur + 1    ; le pas qui reste sur la diagonale
    cmp rcx, largeur * largeur
    jb diagonale

    hlt
""",
    ["rax == 0x2d"], None, "structures")

exo("chaine_longueur", "La longueur d'une chaîne, quatre octets à la fois",
    "String length, four bytes at a time",
    "Lire une chaîne octet par octet coûte un tour de boucle par caractère. En lisant quatre octets d'un coup et en cherchant le zéro dedans, on divise le nombre de tours par quatre — c'est l'idée de toutes les `strlen` de bibliothèque. À l'arrêt, RAX doit valoir 12.",
    "Reading a string byte by byte costs one loop turn per character. Reading four bytes at once and looking for the zero inside divides the turns by four — that is the idea behind every library `strlen`. At the halt, RAX must hold 12.",
    """; La longueur d'une chaîne, sans lire octet par octet.
;
; But : à l'arrêt, RAX vaut 12 — la longueur de « Bonjour tout ».
;
; La version naïve lit un octet, compare à zéro, recommence : un tour
; par caractère. Celle-ci lit quatre octets d'un coup et cherche le zéro
; dedans — quatre fois moins de tours.
;
; Le repérage du zéro dans un mot est le tour de force : `x - 0x01010101`
; passe une retenue là où un octet valait zéro, et `~x` dit lesquels
; avaient le bit haut à zéro. Le ET des deux ne garde que les octets
; nuls. C'est l'astuce de toutes les strlen de bibliothèque.

default rel
global _start

section .data
    texte db "Bonjour tout", 0

section .text
_start:
    lea rsi, [texte]
    xor rax, rax

boucle:
    mov ecx, [rsi + rax]    ; quatre octets d'un coup
    mov edx, ecx
    sub edx, 0x01010101     ; une retenue là où un octet valait 0
    not ecx
    and edx, ecx
    and edx, 0x80808080     ; le bit haut de chaque octet nul
    jnz trouve
    add rax, 4
    jmp boucle

trouve:
    ; Le premier octet nul dans ces quatre-là : son rang est celui du
    ; premier bit allumé, divisé par huit.
    bsf edx, edx
    shr edx, 3
    add rax, rdx

    hlt
""",
    ["rax == 0xc"], None, "structures")

exo("tri_insertion", "Trier huit valeurs",
    "Sorting eight values",
    "Le tri par insertion est celui qu'on fait à la main avec un jeu de cartes : on prend la suivante et on la remet à sa place en décalant ce qui dépasse. Lent sur un million d'éléments, imbattable sur huit. À l'arrêt, RAX doit valoir la plus petite valeur et RBX la plus grande.",
    "Insertion sort is the one you do by hand with playing cards: take the next one and put it back in place, shifting what sticks out. Slow on a million elements, unbeatable on eight. At the halt, RAX must hold the smallest value and RBX the largest.",
    """; Trier huit valeurs par insertion.
;
; But : à l'arrêt, RAX vaut 3 (la plus petite) et RBX vaut 91 (la plus
; grande) — donc le tableau est trié.
;
; Le tri par insertion est celui qu'on fait avec un jeu de cartes : on
; prend la suivante, et on la remet à sa place en décalant vers la
; droite tout ce qui est plus grand.
;
; Lent quand il y a un million d'éléments — le nombre d'opérations
; croît comme le carré — et imbattable sur huit : pas d'appel, pas de
; mémoire supplémentaire, et la boucle intérieure tient dans le cache.

default rel
global _start

section .data
    t       dq 42, 7, 91, 3, 58, 12, 77, 25
    combien equ 8

section .text
_start:
    lea rsi, [t]
    mov rcx, 1              ; on part du deuxième : le premier est trié

suivante:
    mov rax, [rsi + rcx*8]  ; la carte qu'on tient
    mov rdx, rcx            ; où l'on cherche sa place

place:
    test rdx, rdx
    jz posee
    cmp [rsi + rdx*8 - 8], rax
    jbe posee
    ; Plus grand : il descend d'un cran.
    mov r8, [rsi + rdx*8 - 8]
    mov [rsi + rdx*8], r8
    dec rdx
    jmp place

posee:
    mov [rsi + rdx*8], rax
    inc rcx
    cmp rcx, combien
    jb suivante

    ; Le tableau est trié : le premier est le plus petit, le dernier le
    ; plus grand.
    mov rax, [rsi]
    mov rbx, [rsi + (combien-1)*8]

    hlt
""",
    ["rax == 0x3", "rbx == 0x5b"], None, "structures")

exo("recursion", "Une fonction qui s'appelle elle-même",
    "A function that calls itself",
    "La récursion n'est pas une construction du langage : c'est un `call` vers sa propre étiquette, et une pile qui garde les retours. Chaque appel coûte huit octets de pile — et c'est pour cela qu'une récursion trop profonde finit par écraser quelque chose. À l'arrêt, RAX doit valoir 120.",
    "Recursion is not a language construct: it is a `call` to one's own label, and a stack that keeps the returns. Each call costs eight bytes of stack — which is why too deep a recursion ends up overwriting something. At the halt, RAX must hold 120.",
    """; La factorielle, en s'appelant soi-même.
;
; But : à l'arrêt, RAX vaut 120 — 5!.
;
; La récursion n'est pas une construction du langage : c'est un `call`
; vers sa propre étiquette. Ce qui la rend possible est la PILE — chaque
; appel y pose son adresse de retour, et `ret` la reprend.
;
; Regardez l'onglet Pile pendant que ça descend : cinq adresses de
; retour s'y empilent, puis se dépilent. Huit octets par appel, et c'est
; pour cela qu'une récursion trop profonde finit par écraser ce qui est
; en dessous.

global _start

section .text
_start:
    mov rdi, 5
    call fact
    hlt

; Rend RDI! dans RAX.
fact:
    cmp rdi, 1
    ja descendre
    mov rax, 1              ; le cas qui arrête tout
    ret

descendre:
    push rdi                ; on garde le n du niveau courant
    dec rdi
    call fact
    pop rdi
    imul rax, rdi           ; n * (n-1)!
    ret
""",
    ["rax == 0x78"], None, "structures")

exo("pile_cadre", "Le cadre de pile, à la main",
    "The stack frame, by hand",
    "Une fonction qui a besoin de place la prend sur la pile : `push rbp`, `mov rbp, rsp`, `sub rsp, n`. RBP devient l'ancre à laquelle tout se réfère — c'est ce qu'un débogueur suit pour remonter les appels. À l'arrêt, RAX doit valoir 30.",
    "A function that needs room takes it from the stack: `push rbp`, `mov rbp, rsp`, `sub rsp, n`. RBP becomes the anchor everything refers to — it is what a debugger follows to walk the calls. At the halt, RAX must hold 30.",
    """; Le cadre de pile, monté à la main.
;
; But : à l'arrêt, RAX vaut 30.
;
; Une fonction qui a besoin de place pour ses variables la prend sur la
; pile. Le montage est toujours le même :
;
;     push rbp          ; on garde le cadre de l'appelant
;     mov  rbp, rsp     ; RBP ancre le nôtre
;     sub  rsp, n       ; n octets pour nous
;
; Dès lors, `[rbp-8]` est notre première variable, `[rbp+16]` le premier
; argument passé sur la pile, et `[rbp]` le cadre d'au-dessus. C'est
; cette chaîne que namsdec suit dans « Chaîne des appels ».
;
; Le démontage est `leave` — une instruction pour `mov rsp, rbp` suivi
; de `pop rbp`.

global _start

section .text
_start:
    mov rdi, 10
    mov rsi, 20
    call somme
    hlt

somme:
    push rbp
    mov rbp, rsp
    sub rsp, 16             ; deux variables locales

    mov [rbp-8], rdi        ; on range les arguments chez nous
    mov [rbp-16], rsi

    mov rax, [rbp-8]
    add rax, [rbp-16]

    leave                   ; mov rsp, rbp ; pop rbp
    ret
""",
    ["rax == 0x1e"], None, "structures")

exo("liste_chainee", "Suivre une liste chaînée",
    "Following a linked list",
    "Un maillon porte sa valeur et l'adresse du suivant ; le dernier porte zéro. Parcourir, c'est donc lire, sauter, recommencer — chaque saut est une adresse que le processeur ne peut pas prévoir, et c'est pour cela qu'une liste est plus lente qu'un tableau. À l'arrêt, RAX doit valoir 100.",
    "A node holds its value and the address of the next; the last holds zero. Walking it means read, jump, repeat — each jump is an address the processor cannot predict, which is why a list is slower than an array. At the halt, RAX must hold 100.",
    """; Suivre une liste chaînée.
;
; But : à l'arrêt, RAX vaut 100 — la somme des quatre maillons.
;
; Un maillon porte deux choses : sa valeur, et l'adresse du suivant. Le
; dernier porte zéro à la place de l'adresse, et c'est ainsi qu'on sait
; s'arrêter.
;
; Le parcours est donc : lire la valeur, aller à l'adresse, recommencer.
; Chaque saut mène quelque part que le processeur ne pouvait pas
; prévoir — il ne peut ni précharger, ni deviner. C'est toute la
; différence avec un tableau, où le suivant est toujours à côté.

default rel
global _start

section .data
    ; valeur, adresse du suivant
    a dq 10, b
    b dq 20, c
    c dq 30, d
    d dq 40, 0

section .text
_start:
    lea rsi, [a]
    xor rax, rax

maillon:
    test rsi, rsi
    jz fini
    add rax, [rsi]          ; la valeur
    mov rsi, [rsi + 8]      ; le suivant
    jmp maillon

fini:
    hlt
""",
    ["rax == 0x64"], None, "structures")

exo("chercher_octet", "Chercher un octet dans un tampon",
    "Finding a byte in a buffer",
    "`scasb` compare AL à l'octet pointé par RDI et avance : avec `repne`, c'est une boucle entière en une instruction. RCX compte à rebours, et ce qu'il reste dit où l'on s'est arrêté. À l'arrêt, RAX doit valoir 5 — la position du caractère cherché.",
    "`scasb` compares AL to the byte at RDI and advances: with `repne`, that is a whole loop in one instruction. RCX counts down, and what remains says where it stopped. At the halt, RAX must hold 5 — the position of the character sought.",
    """; Chercher un octet, avec une seule instruction de boucle.
;
; But : à l'arrêt, RAX vaut 5 — la position du « , » dans « Salut, toi ».
;
; `scasb` fait trois choses : comparer AL à l'octet en [RDI], avancer
; RDI d'un, et poser les indicateurs. Avec le préfixe `repne`, elle
; recommence tant que ce n'est pas égal ET que RCX n'est pas nul.
;
; RCX compte à rebours. Ce qu'il en reste à l'arrêt dit combien
; d'octets n'ont pas été lus — d'où la position, par soustraction.
;
; C'est la forme la plus ancienne du parcours de chaîne, et elle tient
; encore : une instruction, pas de saut, pas de prédiction à rater.

default rel
global _start

section .data
    texte   db "Salut, toi", 0
    combien equ 10

section .text
_start:
    lea rdi, [texte]
    mov rcx, combien
    mov al, ','
    cld                     ; on avance, on ne recule pas

    repne scasb

    ; RDI pointe juste APRÈS l'octet trouvé : la position est donc
    ; la distance au début, moins un.
    lea rax, [texte]
    sub rdi, rax
    mov rax, rdi
    dec rax

    hlt
""",
    ["rax == 0x5"], None, "structures")

exo("table_sauts", "Une table de sauts, sans comparaison",
    "A jump table, without comparison",
    "Cinq cas, cinq comparaisons — ou une multiplication et un saut indirect. C'est ce que fait un compilateur d'un `switch` dense, et c'est ce qu'on voit dans un binaire sous la forme d'un `jmp [table + rax*8]` qu'aucun désassembleur ne sait suivre. À l'arrêt, RAX doit valoir 400.",
    "Five cases, five comparisons — or one multiplication and an indirect jump. That is what a compiler makes of a dense `switch`, and what one sees in a binary as a `jmp [table + rax*8]` that no disassembler can follow. At the halt, RAX must hold 400.",
    """; Une table de sauts : cinq cas, aucune comparaison.
;
; But : à l'arrêt, RAX vaut 400 — le cas numéro 3.
;
; Cinq cas demandent cinq comparaisons... ou une multiplication et un
; saut indirect. La table porte les adresses ; l'indice les atteint.
;
; C'est ce qu'un compilateur fait d'un `switch` dense, et c'est ce qu'on
; retrouve dans un binaire sous la forme d'un `jmp [table + rax*8]`.
; Aucun désassembleur ne sait où il mène — la cible se calcule, elle ne
; s'écrit pas. namsdec le signale plutôt que d'inventer une flèche.
;
; La borne compte : un indice hors table sauterait vers ce qui se trouve
; après elle, c'est-à-dire n'importe où. C'est une faille classique.

default rel
global _start

section .data
    table dq cas0, cas1, cas2, cas3, cas4
    combien equ 5

section .text
_start:
    mov rcx, 3              ; essayez 0, 1, 2, 3, 4
    cmp rcx, combien        ; la borne : sans elle, tout est possible
    jae hors_table

    lea rdx, [table]
    jmp [rdx + rcx*8]

cas0:
    mov rax, 100
    jmp fin
cas1:
    mov rax, 200
    jmp fin
cas2:
    mov rax, 300
    jmp fin
cas3:
    mov rax, 400
    jmp fin
cas4:
    mov rax, 500
    jmp fin

hors_table:
    xor rax, rax

fin:
    hlt
""",
    ["rax == 0x190"], None, "structures")


# ─────────────────────────────────────────────────────────────────────
#  MASTERY — ce qui l'entoure
# ─────────────────────────────────────────────────────────────────────

exo("sse_somme", "Quatre additions en une instruction",
    "Four additions in one instruction",
    "Un registre XMM porte seize octets, donc quatre entiers de trente-deux bits. `paddd` les additionne tous d'un coup. C'est le calcul vectoriel dans sa forme la plus simple, et c'est ce qui rend un filtre d'image ou un produit scalaire quatre fois plus rapide. À l'arrêt, RAX doit valoir 10.",
    "An XMM register holds sixteen bytes, so four thirty-two-bit integers. `paddd` adds them all at once. That is vector computation at its simplest, and what makes an image filter or a dot product four times faster. At the halt, RAX must hold 10.",
    """; Quatre additions en une instruction.
;
; But : à l'arrêt, RAX vaut 10 — le premier élément de la somme.
;
; Un registre XMM porte seize octets : quatre entiers de trente-deux
; bits, ou deux de soixante-quatre, ou seize octets. `paddd` additionne
; les quatre doubles mots élément par élément, d'un seul ordre.
;
; Les débordements NE SE PROPAGENT PAS d'un élément au suivant : chacun
; boucle chez lui. C'est ce qui distingue une addition vectorielle d'une
; addition de nombres larges.
;
; L'onglet « Registres étendus » du panneau montre les XMM pendant le
; pas à pas.

default rel
global _start

section .data
    align 16
    a dd 1, 2, 3, 4
    b dd 9, 8, 7, 6

section .text
_start:
    movdqa xmm0, [a]
    movdqa xmm1, [b]
    paddd  xmm0, xmm1       ; quatre additions, une instruction

    ; Ce qui en sort : 10, 10, 10, 10.
    movd eax, xmm0

    hlt
""",
    ["rax == 0xa"], None, "mastery")

exo("sse_comparer", "Comparer seize octets d'un coup",
    "Comparing sixteen bytes at once",
    "`pcmpeqb` compare seize octets et rend un masque : tout à un là où c'est égal. `pmovmskb` résume ce masque en seize bits dans un registre ordinaire, où un saut conditionnel peut enfin le lire. C'est le cœur de toutes les fonctions de chaîne modernes. À l'arrêt, RAX doit valoir 4.",
    "`pcmpeqb` compares sixteen bytes and returns a mask: all ones where equal. `pmovmskb` sums that mask into sixteen bits in an ordinary register, where a conditional jump can finally read it. That is the core of every modern string function. At the halt, RAX must hold 4.",
    """; Comparer seize octets d'un coup, et en tirer une réponse.
;
; But : à l'arrêt, RAX vaut 4 — la position du premier octet qui diffère.
;
; Trois instructions, et chacune fait une chose qu'on ne peut pas faire
; autrement :
;
;   pcmpeqb   compare seize octets et rend un MASQUE — tout à un là où
;             c'est égal, tout à zéro ailleurs. Aucun indicateur n'est
;             touché : un masque n'est pas une condition.
;   pmovmskb  résume ce masque : un bit par octet, dans un registre
;             ordinaire. C'est le pont entre le vectoriel et les sauts.
;   bsf       trouve le premier bit allumé.
;
; Le `not` entre les deux retourne la question : on cherche le premier
; octet QUI DIFFÈRE, donc le premier zéro du masque d'égalité.

default rel
global _start

section .data
    align 16
    x db "abcdefghijklmnop"
    y db "abcdXfghijklmnop"

section .text
_start:
    movdqu xmm0, [x]
    movdqu xmm1, [y]
    pcmpeqb xmm0, xmm1      ; le masque d'égalité
    pmovmskb eax, xmm0      ; seize bits, un par octet

    not eax                 ; on veut le premier qui DIFFÈRE
    and eax, 0xffff         ; seize octets, seize bits utiles
    bsf eax, eax            ; le premier bit allumé

    hlt
""",
    ["rax == 0x4"], None, "mastery")

exo("syscall_write", "Écrire, vraiment",
    "Writing, for real",
    "Un programme ne sait pas écrire : il demande au noyau. RAX porte le numéro de l'appel, RDI RSI RDX les arguments, et `syscall` passe la main. C'est la seule frontière que le processeur ne franchit pas seul, et elle se voit dans tout binaire. La console montre le texte ; à l'arrêt, RAX doit valoir 14.",
    "A program cannot write: it asks the kernel. RAX holds the call number, RDI RSI RDX the arguments, and `syscall` hands over. It is the one boundary the processor does not cross alone, and it shows in every binary. The console shows the text; at the halt, RAX must hold 14.",
    """; Demander au noyau d'écrire.
;
; But : à l'arrêt, RAX vaut 14 — le nombre d'octets que le noyau dit
; avoir écrits. Regardez la console : le texte y est.
;
; Un programme ne sait pas écrire. Il ne sait que poser des valeurs dans
; des registres et exécuter `syscall` :
;
;   RAX   le numéro de l'appel      1 = write
;   RDI   le premier argument       1 = la sortie standard
;   RSI   le second                 où sont les octets
;   RDX   le troisième              combien
;
; Au retour, RAX porte la réponse : ce qui a été écrit, ou un nombre
; négatif si le noyau a refusé.
;
; La convention des appels système N'EST PAS celle des fonctions : RCX
; et R11 sont écrasés par `syscall` lui-même, et le quatrième argument
; passe par R10 et non RCX.

default rel
global _start

section .data
    message db "Bonjour, noyau", 10
    combien equ $ - message

section .text
_start:
    mov rax, 1              ; write
    mov rdi, 1              ; la sortie standard
    lea rsi, [message]
    mov rdx, combien
    syscall

    ; RAX porte maintenant ce que le noyau a écrit : 15 octets, le
    ; retour à la ligne compris.
    sub rax, 1              ; sans le retour à la ligne : 14

    hlt
""",
    ["rax == 0xe"], None, "mastery")

exo("debordement_signe", "Le débordement, et pourquoi il se voit",
    "Overflow, and why it shows",
    "Additionner deux grands positifs donne un négatif : c'est le débordement signé, et OF est le seul à le dire. CF, lui, parle du non signé. Les deux indicateurs répondent à deux questions différentes sur la même addition, et confondre les deux est l'erreur qui coûte le plus cher. À l'arrêt, RAX doit valoir 3.",
    "Adding two large positives gives a negative: that is signed overflow, and OF is the only one that says so. CF speaks of unsigned. The two flags answer two different questions about the same addition, and confusing them is the costliest mistake. At the halt, RAX must hold 3.",
    """; Le débordement, et les deux indicateurs qui n'en disent pas la même
; chose.
;
; But : à l'arrêt, RAX vaut 3 — les deux débordements ont été vus.
;
; Une même addition pose DEUX questions :
;
;   CF  « le résultat tient-il en non signé ? »
;   OF  « le résultat tient-il en signé ? »
;
; Elles n'ont pas la même réponse, et c'est tout le sujet. 0x7fff...ff
; plus 1 déborde en signé — le plus grand positif devient le plus petit
; négatif — sans rien déborder en non signé. Et 0xffff...ff plus 1
; déborde en non signé sans rien déborder en signé : -1 plus 1 fait
; zéro, ce qui est juste.
;
; `seto` et `setc` posent 1 ou 0 selon l'indicateur, sans saut.

global _start

section .text
_start:
    xor rax, rax

    ; Débordement SIGNÉ, et lui seul.
    mov rbx, 0x7fffffffffffffff
    add rbx, 1
    seto cl                 ; OF : oui
    setc dl                 ; CF : non
    movzx rcx, cl
    add rax, rcx

    ; Débordement NON SIGNÉ, et lui seul.
    mov rbx, 0xffffffffffffffff
    add rbx, 1
    setc cl                 ; CF : oui
    movzx rcx, cl
    add rax, rcx

    ; Deux additions, deux débordements, deux indicateurs différents.
    add rax, 1

    hlt
""",
    ["rax == 0x3"], None, "mastery")

exo("multiplication_large", "Multiplier plus large que le registre",
    "Multiplying wider than the register",
    "Le produit de deux nombres de soixante-quatre bits en demande cent vingt-huit. `mul` rend donc deux registres : RDX porte le haut, RAX le bas. Ignorer RDX, c'est perdre la moitié du résultat sans que rien ne le dise. À l'arrêt, RDX doit valoir 1 et RAX 0.",
    "The product of two sixty-four-bit numbers needs a hundred and twenty-eight. `mul` therefore returns two registers: RDX holds the high half, RAX the low. Ignoring RDX loses half the result with nothing to say so. At the halt, RDX must hold 1 and RAX 0.",
    """; Multiplier plus large que le registre.
;
; But : à l'arrêt, RDX vaut 1 et RAX vaut 0.
;
; Le produit de deux nombres de n bits en demande 2n. `mul` le sait :
; elle rend RDX:RAX, cent vingt-huit bits à cheval sur deux registres.
;
; Ici 2^32 fois 2^32 fait exactement 2^64 : rien dans RAX, un dans RDX.
; Un programme qui ne lirait que RAX conclurait que le produit est nul,
; et rien ne l'avertirait — c'est une des façons les plus discrètes de
; se tromper en assembleur.
;
; `imul` à deux opérandes, elle, ne rend que la moitié basse : c'est la
; forme qu'on emploie quand on SAIT que le résultat tient.

global _start

section .text
_start:
    mov rax, 0x100000000    ; 2^32
    mov rbx, 0x100000000    ; 2^32

    mul rbx                 ; RDX:RAX = RAX * RBX

    ; RAX vaut 0, RDX vaut 1 : le produit est 2^64, et il vit
    ; entièrement dans la moitié haute.

    hlt
""",
    ["rdx == 0x1", "rax == 0x0"], None, "mastery")

exo("decalage_division", "Diviser sans diviser",
    "Dividing without dividing",
    "`div` coûte des dizaines de cycles ; un décalage en coûte un. Pour une puissance de deux, les deux donnent le même résultat — sur les non signés. Sur les signés, le décalage arithmétique arrondit vers le bas et la division vers zéro, et les deux ne s'accordent pas sur les négatifs. À l'arrêt, RAX doit valoir 1.",
    "`div` costs dozens of cycles; a shift costs one. For a power of two, both give the same result — on unsigned. On signed, the arithmetic shift rounds down and the division towards zero, and the two disagree on negatives. At the halt, RAX must hold 1.",
    """; Diviser sans diviser, et le piège du signe.
;
; But : à l'arrêt, RAX vaut 1 — les deux méthodes ont bien divergé.
;
; `div` coûte des dizaines de cycles ; `shr` en coûte un. Pour une
; puissance de deux, les deux donnent le même résultat... sur les NON
; SIGNÉS.
;
; Sur les signés, elles divergent : `sar` décale en gardant le signe,
; donc arrondit vers le BAS — -7 sur 2 fait -4. `idiv` arrondit vers
; ZÉRO — -7 sur 2 fait -3. Un écart de un, sur un nombre négatif, et
; c'est pour cela qu'un compilateur ajoute une correction avant de
; décaler un signé.
;
; C'est aussi pourquoi on voit, dans un binaire, `shr` suivi d'un `add`
; mystérieux : la correction est là.

global _start

section .text
_start:
    ; Par décalage : arrondi vers le bas.
    mov rax, -7
    sar rax, 1              ; -4
    mov r8, rax

    ; Par division : arrondi vers zéro.
    mov rax, -7
    cqo                     ; étend le signe dans RDX — sans quoi c'est faux
    mov rcx, 2
    idiv rcx                ; -3
    mov r9, rax

    ; L'écart entre les deux vaut exactement un.
    mov rax, r9
    sub rax, r8

    hlt
""",
    ["rax == 0x1"], None, "mastery")

exo("endianness", "Retourner les octets",
    "Swapping the bytes",
    "Le x86 range le poids faible en premier ; les protocoles réseau font l'inverse. `bswap` retourne les octets d'un registre en une instruction — c'est ce que fait tout programme qui lit un en-tête TCP ou un fichier venu d'ailleurs. À l'arrêt, RAX doit valoir 0x78563412.",
    "x86 stores the low byte first; network protocols do the opposite. `bswap` reverses a register's bytes in one instruction — what every program reading a TCP header or a file from elsewhere does. At the halt, RAX must hold 0x78563412.",
    """; Retourner les octets, et pourquoi il le faut.
;
; But : à l'arrêt, RAX vaut 0x78563412.
;
; Le x86 est « petit-boutiste » : le poids faible est rangé en premier.
; 0x12345678 en mémoire s'écrit 78 56 34 12. Les protocoles réseau, eux,
; sont gros-boutistes : le poids fort d'abord.
;
; `bswap` retourne les quatre (ou huit) octets d'un registre en une
; instruction. C'est ce que fait tout programme qui lit un en-tête TCP,
; un fichier PNG, ou n'importe quoi venu d'une autre machine.
;
; Dans un binaire, un `bswap` est donc un indice : il y a du réseau, ou
; un format de fichier, pas loin.
;
; L'onglet Octets montre l'ordre réel en mémoire ; le bandeau montre la
; valeur. Les deux ne se ressemblent pas, et c'est normal.

global _start

section .text
_start:
    mov eax, 0x12345678
    bswap eax

    hlt
""",
    ["rax == 0x78563412"], None, "mastery")

exo("popcount", "Compter les bits allumés",
    "Counting the bits that are on",
    "Compter les bits un à un demande soixante-quatre tours ; `popcnt` le fait en un cycle. La question revient partout — un masque de droits, une somme de contrôle, une distance entre deux empreintes — et c'est pour cela qu'elle a fini par avoir son instruction. À l'arrêt, RAX doit valoir 24.",
    "Counting bits one by one takes sixty-four turns; `popcnt` does it in one cycle. The question comes up everywhere — a permission mask, a checksum, a distance between two fingerprints — which is why it ended up with its own instruction. At the halt, RAX must hold 24.",
    """; Compter les bits allumés, de deux façons.
;
; But : à l'arrêt, RAX vaut 24 — les deux méthodes s'accordent.
;
; La question revient partout : combien de droits dans ce masque,
; combien de bits diffèrent entre deux empreintes, combien de cases
; occupées dans cette carte. Elle a fini par avoir son instruction.
;
; La méthode de Kernighan, en dessous, mérite d'être connue : `x & (x-1)`
; éteint le bit allumé le plus bas, et rien d'autre. On boucle donc
; autant de fois qu'il y a de bits, non soixante-quatre fois.
;
; `popcnt` fait la même chose en un cycle. Quand une instruction existe,
; elle gagne toujours — mais savoir ce qu'elle remplace aide à la
; reconnaître dans un binaire qui ne l'emploie pas.

global _start

section .text
_start:
    mov rbx, 0xdeadbeef     ; vingt-quatre bits allumés — comptez-les

    ; Par l'instruction.
    popcnt rax, rbx

    ; Par la méthode de Kernighan, pour vérifier.
    mov rdx, rbx
    xor rcx, rcx

kernighan:
    test rdx, rdx
    jz compte
    mov r8, rdx
    dec r8
    and rdx, r8             ; éteint le bit le plus bas
    inc rcx
    jmp kernighan

compte:
    ; Les deux doivent s'accorder : si ce n'est pas le cas, RAX finit
    ; à zéro et l'exercice le dit.
    cmp rax, rcx
    je fin
    xor rax, rax

fin:
    hlt
""",
    ["rax == 0x18"], None, "mastery")

exo("canari", "Le canari de pile, et ce qu'il protège",
    "The stack canary, and what it protects",
    "Une valeur secrète posée devant l'adresse de retour, et vérifiée avant de revenir : si elle a changé, quelque chose a débordé. C'est la protection qu'on rencontre dans presque tout binaire compilé aujourd'hui, et la première qu'on identifie en lisant un prologue. À l'arrêt, RAX doit valoir 1.",
    "A secret value placed in front of the return address and checked before returning: if it changed, something overflowed. It is the protection found in nearly every binary compiled today, and the first one identifies when reading a prologue. At the halt, RAX must hold 1.",
    """; Le canari de pile, monté à la main.
;
; But : à l'arrêt, RAX vaut 1 — le canari est intact.
;
; Le principe tient en trois gestes : poser une valeur secrète juste
; devant l'adresse de retour, faire son travail, vérifier qu'elle n'a
; pas bougé avant de revenir. Un débordement de tampon qui écrase
; l'adresse de retour écrase forcément le canari d'abord.
;
; Dans un vrai binaire, la valeur vient de `fs:[0x28]` — une case que le
; noyau remplit, différente à chaque lancement. C'est ce `mov rax,
; fs:[0x28]` en tête de fonction qui trahit la protection, et c'est la
; première chose qu'on reconnaît en lisant un prologue.
;
; Essayez de faire déborder le tampon : changez le 8 en 24 dans le
; `mov qword [rsp+...]`, et regardez RAX passer à zéro.

global _start

section .text
_start:
    sub rsp, 32             ; un tampon de 24 octets, plus le canari
    mov rbx, 0xdeadbeefcafe ; le secret — en vrai, il vient du noyau
    mov [rsp+24], rbx       ; posé juste devant ce qu'il protège

    ; Le travail : on écrit dans le tampon, sans déborder.
    mov qword [rsp], 1
    mov qword [rsp+8], 2
    mov qword [rsp+16], 3

    ; La vérification, avant de rendre la main.
    mov rax, [rsp+24]
    cmp rax, rbx
    je intact
    xor rax, rax            ; le canari a chanté : on ne revient pas
    jmp fin

intact:
    mov rax, 1

fin:
    add rsp, 32
    hlt
""",
    ["rax == 0x1"], None, "mastery")

exo("code_auto_modifiant", "Un programme qui se récrit",
    "A program that rewrites itself",
    "Rien n'empêche un programme d'écrire dans son propre code — sauf les droits de la page. C'est la technique des paquets chiffrés, et c'est pourquoi une section exécutable et inscriptible est un signal en analyse. namsdec refuse l'écriture et le dit, comme la vraie machine. À l'arrêt, RAX doit valoir 42.",
    "Nothing stops a program from writing into its own code — except the page rights. It is the technique of packed binaries, and why a section both executable and writable is a signal in analysis. namsdec refuses the write and says so, like the real machine. At the halt, RAX must hold 42.",
    """; Un programme qui se récrit — et le mur qu'il rencontre.
;
; But : à l'arrêt, RAX vaut 42.
;
; Rien dans le processeur n'empêche d'écrire à une adresse qui contient
; du code. Ce qui l'empêche, ce sont les DROITS de la page : `.text`
; est lisible et exécutable, pas inscriptible.
;
; C'est pourquoi une section à la fois exécutable et inscriptible est un
; signal en analyse : un paquet chiffré se déchiffre lui-même en
; mémoire, et il lui faut ce droit-là. namsdec le dit dans Analyse >
; Protections.
;
; Ici on fait l'inverse, et c'est la façon honnête : la valeur vit dans
; `.data`, qui s'écrit. Décommentez la ligne marquée pour voir namsdec
; refuser l'écriture dans `.text` — avec l'adresse et la raison, comme
; la vraie machine.

default rel
global _start

section .data
    valeur dq 0

section .text
_start:
    ; Ceci marche : .data s'écrit.
    mov qword [valeur], 42
    mov rax, [valeur]

    ; Ceci serait refusé : .text ne s'écrit pas.
    ; lea rdi, [_start]
    ; mov qword [rdi], 0x90909090

    hlt
""",
    ["rax == 0x2a"], None, "mastery")


# Les budgets, mesurés sur la solution livrée — non devinés.
#
# `par` est le nombre d'instructions qu'exécute le programme tel qu'il
# est donné : l'égaler est normal, descendre en dessous est l'exercice.
# Il reste à None là où il n'y a rien à chercher : un appel système ne
# se raccourcit pas, et une démonstration de trois instructions non plus.
MESURÉ = {
    "matrice": 16,
    "chaine_longueur": 29,
    "tri_insertion": 175,
    "recursion": 39,
    "pile_cadre": 13,
    "liste_chainee": 25,
    "chercher_octet": 15,
    "table_sauts": 8,
    "sse_somme": 5,
    "sse_comparer": 8,
    "debordement_signe": 14,
    "decalage_division": 11,
    "popcount": 177,
    "canari": 12,
}
for _e in E:
    _e["par"] = MESURÉ.get(_e["id"])

json.dump({"id": "atelier", "name": "Atelier — intermédiaire et avancé", "exercises": E},
          open(sys.argv[1], "w"), ensure_ascii=False, indent=1)
print(f"{len(E)} exercices écrits")
