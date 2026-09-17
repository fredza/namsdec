; Calculer une adresse sans aller la chercher.
;
; But : à l'arrêt, RAX doit valoir 45, RBX l'adresse de `table`, et RCX
; le troisième nombre du tableau, 30.
;
; `mov rax, [table + rcx*8]` *lit* la mémoire ; `lea rax, [table +
; rcx*8]` calcule la même adresse et s'arrête là. Le processeur fait le
; calcul d'adressage — base + indice × taille + décalage — sans toucher
; à la mémoire, et c'est ce qui fait de `lea` la multiplication la moins
; chère du jeu d'instructions : `lea rax, [rdi + rdi*4]` vaut rdi × 5, en
; une instruction et sans multiplieur.
;
; Arrêté sur un `lea`, regardez le bandeau : il montre l'adresse que le
; crochet désigne, avant que quoi que ce soit ne soit lu.

default rel
global _start

section .data
    table dq 10, 20, 30, 40

section .text
_start:
    mov rdi, 9
    lea rax, [rdi + rdi*4]  ; 9 x 5 = 45, sans `imul`

    lea rbx, [table]        ; l'adresse, pas le contenu

    mov rcx, 2
    mov rcx, [table + rcx*8] ; le troisième : base + 2 x 8

    hlt
