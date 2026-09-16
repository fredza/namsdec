; Maximum de deux nombres.
;
; But : à l'arrêt, RAX doit contenir le plus grand des deux (12). Changez
; les valeurs de départ et devinez le résultat avant d'exécuter.

global _start

section .text
_start:
    mov rax, 7              ; premier nombre
    mov rbx, 12             ; second nombre

    cmp rax, rbx
    jge deja_le_plus_grand
    mov rax, rbx            ; RBX est plus grand : on le copie
deja_le_plus_grand:

    ; À essayer ensuite : refaire la même chose sans branchement, avec
    ; `cmp rax, rbx` suivi de `cmovl rax, rbx`.
    hlt
