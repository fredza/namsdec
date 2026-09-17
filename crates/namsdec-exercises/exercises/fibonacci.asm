; Fibonacci, sans appel et sans tableau.
;
; But : à l'arrêt, RAX doit valoir 55 — le dixième terme de la suite.
;
; La suite de Fibonacci se définit par récurrence, et c'est l'exemple
; qu'on donne toujours pour la récursion. En assembleur, elle s'écrit
; sans un seul `call` : deux registres suffisent à garder les deux termes
; précédents, et `xadd` les échange et les additionne d'un coup.
;
; La version récursive existe aussi, et elle est catastrophique : elle
; recalcule les mêmes termes des milliers de fois. Écrivez-la si vous
; voulez voir le compteur d'instructions exploser — c'est la meilleure
; démonstration de ce que coûte un mauvais algorithme, et elle tient en
; six lignes.

global _start

section .text
_start:
    xor rax, rax            ; F(0)
    mov rbx, 1              ; F(1)
    mov rcx, 10             ; le terme voulu

boucle:
    test rcx, rcx
    jz fini

    xadd rax, rbx           ; RAX <- RAX + RBX, RBX <- l'ancien RAX
    dec rcx
    jmp boucle

fini:
    hlt
