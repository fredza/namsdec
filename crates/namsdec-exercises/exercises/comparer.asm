; Comparer, et choisir.
;
; But : R8 doit contenir le plus grand des deux nombres.
;
; Puis changez -3 en 3 et recommencez : `jg` lit SF et OF, pour des
; nombres signés ; `ja` lirait CF, pour des non signés. Avec -3, les
; deux ne donnent pas la même réponse — c'est l'erreur classique.

global _start

section .text
_start:
    mov rax, -3
    mov rbx, 2

    cmp rax, rbx
    jg  rax_plus_grand
    mov r8, rbx
    jmp fin
rax_plus_grand:
    mov r8, rax
fin:

    ; À essayer ensuite : la même chose sans branchement, avec
    ; `cmp rax, rbx` puis `cmovl rax, rbx`.
    hlt
