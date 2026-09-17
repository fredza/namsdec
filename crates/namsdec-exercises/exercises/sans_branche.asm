; Choisir sans sauter.
;
; But : à l'arrêt, RAX doit valoir 20 — le plus grand des deux nombres —
; obtenu sans un seul saut conditionnel.
;
; Un processeur moderne devine la suite d'un saut avant de la connaître.
; Quand il devine juste, le saut est gratuit ; quand il se trompe, il
; jette une quinzaine d'étapes déjà commencées. Sur une donnée
; imprévisible — un nombre au hasard, une entrée utilisateur — il se
; trompe une fois sur deux.
;
; `cmovcc` répond à cela : la comparaison a lieu, mais il n'y a rien à
; deviner, donc rien à jeter. Le prix est que les deux valeurs sont
; calculées, et c'est un marché qui ne vaut que si le calcul est court.
;
; Les deux versions sont ici : celle qui saute est en commentaire.
; Comparez le compteur d'instructions.

global _start

section .text
_start:
    mov rax, 10
    mov rbx, 20

    cmp rax, rbx
    cmovl rax, rbx          ; si RAX < RBX, prendre RBX. Sans saut.

    ; La version qui saute, à essayer :
    ;   cmp rax, rbx
    ;   jge fin
    ;   mov rax, rbx
    ; fin:

    hlt
