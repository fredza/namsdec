; Parcourir une chaîne.
;
; But : à l'arrêt, RCX doit contenir la longueur de `message`, soit 5.
;
; Une chaîne en mémoire n'est qu'une suite d'octets ; ce qui en marque
; la fin est une convention. Ici, un zéro final — celui que le C
; appelle « terminateur ».
;
; `movzx` lit un octet et l'élargit avec des zéros : sans lui, écrire
; dans `al` laisserait intact le reste de RAX, et la comparaison
; porterait sur une valeur qu'on n'a pas lue.

global _start

section .data
message: db "salut", 0

section .text
_start:
    mov rsi, message        ; où l'on en est de la lecture
    xor rcx, rcx            ; combien d'octets lus

parcours:
    movzx eax, byte [rsi]   ; l'octet courant, élargi par des zéros
    test al, al             ; est-il nul ?
    jz fini
    inc rsi                 ; l'octet suivant
    inc rcx
    jmp parcours

fini:
    hlt
