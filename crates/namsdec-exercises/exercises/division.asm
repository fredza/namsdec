; Diviser, et ce que le processeur lit sans le dire.
;
; But : à l'arrêt, RAX doit contenir 7 (le quotient de 45 par 6) et RDX
; 3 (le reste).
;
; `div` ne divise pas RAX : il divise le nombre de 128 bits formé par
; RDX en haut et RAX en bas. Ce qui traîne dans RDX fait donc partie du
; dividende, même si personne ne l'a voulu — et si le quotient ne tient
; pas dans 64 bits, le processeur refuse de continuer.
;
; Ici RDX porte encore le reste d'un calcul précédent. Exécutez, lisez
; le message, puis mettez RDX à zéro avant la division.

global _start

section .text
_start:
    mov rdx, 99             ; un reste oublié là par le calcul d'avant
    mov rax, 45             ; le dividende... la moitié basse, du moins
    mov rcx, 6              ; le diviseur

    div rcx                 ; RAX = quotient, RDX = reste

    hlt
