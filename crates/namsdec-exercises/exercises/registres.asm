; Largeurs de registres.
;
; But : avancer pas à pas et regarder RAX, RBX, RCX dans le bandeau.
; Écrire un registre 32 bits met à zéro la moitié haute ; écrire 16 ou
; 8 bits laisse le reste intact. C'est la source de bogue numéro un
; quand on débute.

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
