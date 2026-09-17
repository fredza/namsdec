; Lire ce qu'on vous donne.
;
; But : tapez quelque chose dans la console, appuyez sur Envoyer, et
; regardez RAX — le noyau y met le nombre d'octets lus, saut de ligne
; compris. Le tampon, lui, se regarde dans l'onglet Mémoire.
;
; `read(0, tampon, taille)` lit *au plus* `taille` octets et rend ce
; qu'il a pu lire — jamais plus, souvent moins. Un programme qui suppose
; avoir tout reçu du premier coup est un programme qui se trompera : sur
; un fichier, sur un tube, sur une connexion.
;
; Zéro en retour veut dire « fin d'entrée », et non « erreur » : c'est ce
; que rend la console quand on clique « Fin d'entrée ».
;
; À faire, une fois que cela marche : bouclez tant que `read` rend autre
; chose que zéro, et comptez le total.

default rel
global _start

section .bss
    tampon resb 64

section .text
_start:
    mov rax, 0              ; read
    mov rdi, 0              ; l'entrée standard
    lea rsi, [tampon]       ; où mettre ce qu'on lit
    mov rdx, 64             ; au plus soixante-quatre octets
    syscall                 ; RAX : combien ont vraiment été lus

    ; Le premier octet reçu, pour l'avoir sous les yeux.
    movzx rbx, byte [tampon]

    hlt
