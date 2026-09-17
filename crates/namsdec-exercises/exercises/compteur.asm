; `loop` : la boucle que le processeur connaît par cœur.
;
; But : à l'arrêt, RAX doit valoir 15 — la même somme que « Une boucle
; qui compte », écrite autrement.
;
; `loop etiquette` décrémente RCX et saute si le résultat n'est pas nul :
; deux instructions en une, et RCX y est imposé. Elle date d'une époque
; où chaque octet comptait.
;
; Elle a pourtant disparu du code moderne : sur les processeurs
; d'aujourd'hui, `dec rcx` suivi de `jnz` va *plus vite* que `loop`, qui
; n'a jamais été optimisée. Comparez le compteur d'instructions des deux
; versions — il dit le nombre d'instructions, pas le temps, et c'est
; justement la limite de cette mesure-là.

global _start

section .text
_start:
    xor rax, rax
    mov rcx, 5

boucle:
    add rax, rcx
    loop boucle             ; dec rcx + jnz boucle, en une instruction

    hlt
