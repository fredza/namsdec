; Chercher une valeur dans un tableau.
;
; But : à l'arrêt, RAX doit valoir 3 — la position de 40 dans le tableau
; — et RBX 1, le drapeau « trouvé ».
;
; Une recherche linéaire, c'est trois choses : un indice qui avance, une
; comparaison, et deux sorties — on a trouvé, ou on est arrivé au bout.
; Rater la seconde donne une boucle qui lit au-delà du tableau, et c'est
; ainsi qu'on lit la mémoire du voisin.
;
; Essayez de chercher une valeur absente : la boucle doit s'arrêter
; proprement, avec RBX à zéro. C'est le cas qu'on oublie.
;
; À faire ensuite : arrêtez-vous *avant* la fin quand la valeur est
; trouvée, plutôt que de parcourir tout le tableau.

default rel
global _start

section .data
    nombres dq 10, 20, 30, 40, 50
    combien equ 5

section .text
_start:
    lea rsi, [nombres]
    mov rdi, 40             ; ce qu'on cherche
    xor rcx, rcx            ; l'indice
    xor rbx, rbx            ; trouvé ? pas encore

parcours:
    cmp rcx, combien
    jae fini                ; arrivé au bout sans rien trouver

    cmp [rsi + rcx*8], rdi
    je trouve

    inc rcx
    jmp parcours

trouve:
    mov rax, rcx            ; la position
    mov rbx, 1              ; et le drapeau

fini:
    hlt
