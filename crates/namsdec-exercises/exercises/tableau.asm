; Un tableau, et son indice.
;
; But : à l'arrêt, RAX doit contenir la somme des quatre nombres, 100.
;
; `[base + indice*taille]` est l'adressage qui fait exister les
; tableaux : le processeur calcule l'adresse pour vous, à condition que
; la taille soit 1, 2, 4 ou 8. C'est pour cela que les tableaux de
; structures ont des tailles arrondies.
;
; Arrêté sur le `add`, regardez le bandeau : il montre l'adresse que
; `[nombres + rcx*8]` désigne *avant* de la lire, et la valeur qui s'y
; trouve.

global _start

section .data
nombres: dq 10, 20, 30, 40
taille:  equ 4              ; combien d'éléments — calculé à l'assemblage

section .text
_start:
    xor rax, rax            ; la somme
    xor rcx, rcx            ; l'indice

somme:
    add rax, [nombres + rcx*8]
    inc rcx
    cmp rcx, taille
    jb somme                ; `jb` : comparaison non signée, un indice
                            ; n'est jamais négatif

    hlt
