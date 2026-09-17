; Une fonction qui s'appelle elle-même.
;
; But : à l'arrêt, RAX doit contenir 5! = 120.
;
; La récursion ne demande rien de particulier au processeur : `call`
; empile une adresse de retour, et rien n'interdit d'appeler la fonction
; où l'on se trouve déjà. Ce qui la rend possible, c'est la pile — chaque
; appel y garde ce qui lui appartient, et le dépile en revenant.
;
; Regardez « Chaîne des appels » dans l'onglet Pile pendant que vous
; avancez : elle s'allonge à chaque descente et se raccourcit à chaque
; retour. C'est exactement l'image qu'on se fait de la récursion, sauf
; qu'ici on la voit.
;
; Et c'est aussi ce qui explique le débordement de pile : cinq appels
; tiennent, cinq millions, non.

global _start

section .text
_start:
    mov rdi, 5              ; l'argument, selon la convention
    call factorielle
    hlt

; factorielle(n) : si n <= 1, rendre 1 ; sinon n * factorielle(n - 1).
factorielle:
    cmp rdi, 1
    jbe un                  ; le cas d'arrêt : sans lui, on descend sans fin

    push rdi                ; n doit survivre à l'appel
    dec rdi
    call factorielle
    pop rdi
    imul rax, rdi           ; n * factorielle(n - 1)
    ret

un:
    mov rax, 1
    ret
