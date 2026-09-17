; Compter les bits à un.
;
; But : à l'arrêt, RAX doit valoir 5 — le nombre de bits à un dans
; 0b10110101 — et RBX la même chose, obtenue en une instruction.
;
; La boucle est évidente : regarder le bit du bas, décaler, recommencer.
; Soixante-quatre tours pour un registre plein.
;
; `popcnt` fait le même travail en une instruction, parce que le
; matériel sait le faire directement. C'est l'exemple type de ce qu'on ne
; devine pas en lisant un cours d'algorithmique : la meilleure solution
; n'est pas un meilleur algorithme, c'est une instruction qui existe.
;
; Comparez les deux compteurs. Et retenez `bsf`, `bsr`, `tzcnt`,
; `lzcnt` : la même famille, pour trouver le premier ou le dernier bit à
; un.

global _start

section .text
_start:
    mov rdx, 0b10110101     ; cinq bits à un
    xor rax, rax
    mov rcx, rdx

boucle:
    test rcx, rcx
    jz fait
    mov r8, rcx
    and r8, 1               ; le bit du bas
    add rax, r8
    shr rcx, 1              ; au suivant
    jmp boucle

fait:
    popcnt rbx, rdx         ; la même chose, en une instruction

    hlt
