; Une boucle qui compte.
;
; But : à l'arrêt, RAX doit contenir 5+4+3+2+1 = 15.
;
; `dec` pose ZF quand le résultat est nul, et `jnz` lit ce ZF. C'est
; toute la mécanique d'une boucle : une opération qui laisse une trace
; dans les indicateurs, un saut qui la lit.

global _start

section .text
_start:
    mov rax, 0
    mov rcx, 5
boucle:
    add rax, rcx
    dec rcx
    jnz boucle

    hlt
