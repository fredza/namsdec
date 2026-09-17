; Les quatre indicateurs, un par un.
;
; But : à l'arrêt, RAX doit valoir 1, RBX 1, RCX 1 et RDX 1 — chacun
; portant l'indicateur qu'on vient de provoquer.
;
; Un calcul laisse quatre traces qui comptent. `ZF` : le résultat est
; nul. `SF` : son bit de poids fort est à un. `CF` : il y a eu une
; retenue *au-delà* du registre — c'est le débordement des non signés.
; `OF` : le signe du résultat est impossible — c'est celui des signés.
;
; Les quatre sont provoqués ici exprès, et `setcc` les recopie dans un
; registre pour qu'on les voie. Avancez pas à pas et regardez les
; pastilles du bandeau changer : c'est la seule façon de ne plus les
; confondre.

global _start

section .text
_start:
    ; ZF : 3 - 3 fait zéro.
    mov rax, 3
    sub rax, 3
    setz al
    movzx rax, al

    ; SF : 0 - 1 est négatif.
    mov rbx, 0
    sub rbx, 1
    sets bl
    movzx rbx, bl

    ; CF : 0 - 1 emprunte au-delà du registre.
    mov rcx, 0
    sub rcx, 1
    setc cl
    movzx rcx, cl

    ; OF : le plus grand entier signé, plus un, devient négatif.
    mov rdx, 0x7fffffffffffffff
    add rdx, 1
    seto dl
    movzx rdx, dl

    hlt
