; D'un texte à un nombre.
;
; But : à l'arrêt, RAX doit valoir 4096 — la valeur du texte "4096" — et
; RBX 1, le drapeau « tout était bien un chiffre ».
;
; Le clavier ne donne pas de nombres : il donne des caractères. "4096"
; est la suite d'octets 0x34 0x30 0x39 0x36, et rien dedans ne vaut
; quatre mille quatre-vingt-seize. La conversion est une règle de trois
; répétée : résultat = résultat × 10 + (caractère − '0').
;
; C'est l'exact inverse de « Afficher un nombre », et c'est ce que fait
; `atoi`. La différence avec `strtol`, qui lui est préféré partout, est
; ici : `atoi` ne dit pas si le texte était un nombre. Le drapeau RBX
; est ce que `atoi` ne rend pas.
;
; À faire ensuite : remplacez un chiffre du texte par une lettre et
; vérifiez que RBX tombe à zéro au lieu de rendre n'importe quoi.

default rel
global _start

section .data
    texte db "4096", 0

section .text
_start:
    lea rsi, [texte]
    xor rax, rax            ; le nombre en construction
    mov rbx, 1              ; jusqu'ici, tout va bien

parcours:
    movzx rcx, byte [rsi]   ; un octet, élargi avec des zéros
    test rcx, rcx
    jz fini                 ; le zéro final : c'est tout

    cmp rcx, '0'
    jb pas_un_chiffre
    cmp rcx, '9'
    ja pas_un_chiffre

    sub rcx, '0'            ; du caractère au chiffre : 0x34 -> 4
    imul rax, 10            ; on décale d'un rang…
    add rax, rcx            ; …et l'on pose le nouveau

    inc rsi
    jmp parcours

pas_un_chiffre:
    xor rbx, rbx            ; le texte n'était pas un nombre

fini:
    hlt
