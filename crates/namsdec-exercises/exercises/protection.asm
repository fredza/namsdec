; Ce qu'on n'a pas le droit de faire.
;
; But : comprendre les trois refus. Exécutez, lisez le message en bas,
; puis commentez la ligne fautive et recommencez avec la suivante.
;
; L'onglet Mémoire affiche, au-dessus du vidage, la zone de l'adresse
; regardée et ses droits — .text en lecture-exécution, .data en
; lecture-écriture.

default rel
global _start

section .data
    tampon times 8 db 0

section .text
_start:
    ; 1. Écrire dans son propre code.
    mov byte [_start], 0x90

    ; 2. Exécuter ses données.
    jmp tampon

    ; 3. Écrire là où il n'y a rien.
    mov rax, 0x407000
    mov qword [rax], 1

    hlt
