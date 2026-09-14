BITS 64
org 0x400000

    mov rax, 0
    mov rcx, 5
sum_loop:
    add rax, rcx
    dec rcx
    jnz sum_loop
    hlt

; But : à l'arrêt, RAX doit contenir 5+4+3+2+1 = 15.
