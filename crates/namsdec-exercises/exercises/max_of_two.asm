BITS 64
org 0x400000

    mov rax, 7      ; premier nombre
    mov rbx, 12     ; second nombre
    cmp rax, rbx
    jge already_max
    mov rax, rbx    ; RBX est plus grand : on le copie dans RAX
already_max:
    hlt

; But : à l'arrêt, RAX doit contenir le plus grand des deux nombres (12).
; Essayez de changer les valeurs de départ et de deviner le résultat avant
; d'exécuter pas à pas.
