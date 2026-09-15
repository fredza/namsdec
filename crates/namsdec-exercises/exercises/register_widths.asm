BITS 64
section .text

    mov rax, 0x1122334455667788
    mov eax, 0xAABBCCDD  ; écrire EAX remet à zéro les 32 bits hauts de RAX
    mov rbx, 0x1122334455667788
    mov bx, 0xBEEF        ; écrire BX préserve le reste de RBX
    hlt

; But : observer, pas à pas, la différence entre écrire un registre 32 bits
; (RAX -> 0x00000000AABBCCDD) et un registre 16 bits (RBX -> ...667788 devient ...66BEEF).
