; Un tableau à deux dimensions.
;
; But : à l'arrêt, RAX doit valoir 60 — l'élément de la ligne 1, colonne
; 2 — et RBX 450, la somme de toute la matrice.
;
; La mémoire est plate : un tableau à deux dimensions n'existe pas pour
; le processeur. On le range ligne par ligne, et l'on calcule l'adresse
; soi-même : base + (ligne × largeur + colonne) × taille.
;
; C'est de là que vient une règle de performance qu'on retrouve partout :
; parcourir une matrice ligne par ligne va plus vite que colonne par
; colonne, parce que les octets voisins arrivent ensemble dans le cache.
; Le même algorithme, deux ordres, deux vitesses.

default rel
global _start

%define LARGEUR 3

section .data
    ; Trois lignes de trois colonnes.
    matrice dq 10, 20, 30
            dq 40, 50, 60
            dq 70, 80, 90

section .text
_start:
    ; L'élément [1][2] : ligne 1, colonne 2.
    lea rsi, [matrice]
    mov rcx, 1              ; la ligne
    mov rdx, 2              ; la colonne
    imul rcx, LARGEUR       ; ligne x largeur
    add rcx, rdx            ; + colonne
    mov rax, [rsi + rcx*8]  ; x 8, la taille d'un élément

    ; Et la somme de tout, ligne par ligne.
    xor rbx, rbx
    xor rcx, rcx
parcours:
    cmp rcx, 9
    jae fini
    add rbx, [rsi + rcx*8]
    inc rcx
    jmp parcours

fini:
    hlt
