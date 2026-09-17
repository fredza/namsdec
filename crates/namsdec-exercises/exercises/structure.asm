; Un enregistrement en mémoire.
;
; But : à l'arrêt, RAX doit valoir 30 — l'âge du deuxième — et RBX 3,
; le nombre d'enregistrements dont le champ « actif » vaut 1.
;
; Une structure n'existe pas pour le processeur : c'est une suite
; d'octets, et un nom pour chaque décalage depuis le début. `struc` de
; nasm ne fait rien d'autre — il compte les octets à votre place et
; vous rend `Personne.age` au lieu de `+8`.
;
; Un tableau de structures se parcourt alors comme n'importe quel
; tableau, à ceci près que le pas n'est plus 8 mais la taille de la
; structure entière : base + indice × taille + décalage du champ.
; C'est exactement ce que fait un compilateur C derrière `gens[i].age`.
;
; À faire ensuite : ajoutez un champ après `actif` et regardez la
; taille changer toute seule — c'est tout l'intérêt de `struc`.

default rel
global _start

struc Personne
    .id     resq 1          ; +0
    .age    resq 1          ; +8
    .actif  resq 1          ; +16
endstruc

section .data
    ; Quatre personnes, à la file. Pas de séparateur : la taille suffit.
    gens:
        dq 1, 20, 1
        dq 2, 30, 1
        dq 3, 40, 0
        dq 4, 50, 1
    combien equ 4

section .text
_start:
    lea rsi, [gens]

    ; L'âge du deuxième : indice 1, champ .age.
    mov rcx, 1
    imul rcx, Personne_size
    mov rax, [rsi + rcx + Personne.age]

    ; Combien sont actifs ?
    xor rbx, rbx
    xor rcx, rcx            ; l'indice
    xor rdx, rdx            ; le décalage en octets, qui avance par pas
parcours:
    cmp rcx, combien
    jae fini
    cmp qword [rsi + rdx + Personne.actif], 1
    jne suivant
    inc rbx
suivant:
    inc rcx
    add rdx, Personne_size
    jmp parcours

fini:
    hlt
