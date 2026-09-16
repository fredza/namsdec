; Somme d'une boucle.
;
; But : à l'arrêt, RAX doit contenir 5+4+3+2+1 = 15.

global _start

section .text
_start:
    mov rax, 0
    mov rcx, 5
boucle:
    add rax, rcx
    dec rcx
    jnz boucle          ; tant que RCX n'est pas nul

    ; `hlt` arrête la machine simulée en laissant les registres en
    ; place — c'est ce qui permet de lire le résultat. Un vrai programme
    ; Linux finirait par l'appel système `exit`.
    hlt
