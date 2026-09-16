; La pile.
;
; But : à l'arrêt, RAX vaut 1 et RBX vaut 2 — malgré l'échange.
;
; Regardez RSP dans le bandeau à chaque pas : il descend de 8 à chaque
; `push`, remonte à chaque `pop`. L'onglet Pile montre ce qu'il y a
; autour de lui.

global _start

section .text
_start:
    mov rax, 1
    mov rbx, 2

    push rax                ; RSP descend de 8
    push rbx

    ; La pile rend dans l'ordre inverse : dernier posé, premier repris.
    pop rax
    pop rbx

    ; Donc ici RAX vaut 2 et RBX vaut 1 : l'échange a eu lieu.
    ; À faire : remettez-les à l'endroit, avec deux instructions.

    hlt
