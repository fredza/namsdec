; Largeurs de registres.
;
; But : observer, pas à pas, que RAX perd sa moitié haute et que RBX la
; garde. Avancez avec Step Into et regardez le bandeau des registres.

global _start

section .text
_start:
    mov rax, 0x1122334455667788
    mov eax, 0xAABBCCDD     ; RAX devient 0x00000000AABBCCDD

    mov rbx, 0x1122334455667788
    mov bx, 0xBEEF          ; RBX devient 0x112233445566BEEF

    mov rcx, 0x1122334455667788
    mov cl, 0x42            ; RCX devient 0x1122334455667742
    hlt
