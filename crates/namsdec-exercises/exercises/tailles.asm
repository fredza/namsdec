; La même adresse, quatre lectures différentes.
;
; But : à l'arrêt, RAX doit valoir 0x11, RBX 0x2211, RCX 0x44332211 et
; RDX 0x8877665544332211.
;
; Une adresse ne dit pas combien d'octets lire : c'est l'instruction qui
; le décide. `byte`, `word`, `dword`, `qword` — un, deux, quatre, huit —
; et l'assembleur l'exige dès que le registre ne suffit pas à trancher.
;
; Remarquez ce que chaque lecture rapporte : les octets sont rangés du
; poids faible au poids fort, donc lire deux octets à partir de la même
; adresse donne 0x2211 et non 0x1122. C'est le petit-boutisme, revu
; sous un autre angle.
;
; Et `movzx` contre `mov` : le premier élargit avec des zéros, le second
; laisserait intact le haut du registre.

default rel
global _start

section .data
    huit dq 0x8877665544332211

section .text
_start:
    movzx rax, byte [huit]      ; un octet
    movzx rbx, word [huit]      ; deux
    mov   ecx, [huit]           ; quatre — et le haut de RCX s'efface
    mov   rdx, [huit]           ; huit

    hlt
