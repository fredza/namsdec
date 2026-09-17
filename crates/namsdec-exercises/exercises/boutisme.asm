; L'ordre des octets en mémoire.
;
; But : à l'arrêt, RCX doit contenir 0x88 et RDX 0x11 — comprendre
; pourquoi, c'est comprendre le petit-boutisme.
;
; Un entier de 64 bits occupe huit octets, et x86-64 les range du moins
; significatif au plus significatif. L'octet d'adresse la plus basse est
; donc celui de poids faible : ce qu'on lit à `valeur` n'est pas le
; premier chiffre du nombre écrit ici, c'est le dernier.
;
; Ouvrez la fenêtre Mémoire sur `valeur` : les huit octets s'y lisent à
; l'envers du nombre, et c'est normal.

default rel
global _start

section .data
valeur: dq 0x1122334455667788

section .text
_start:
    mov rax, [valeur]           ; le nombre entier, remis à l'endroit
    movzx ecx, byte [valeur]    ; le premier octet en mémoire
    movzx edx, byte [valeur + 7] ; le dernier

    hlt
