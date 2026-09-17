; Ce qu'un tampon trop petit laisse faire.
;
; But : comprendre, en le voyant, ce qu'un débordement écrase — et
; pourquoi toutes les protections des binaires modernes existent.
;
; Le programme lit jusqu'à 64 octets dans un tampon qui en contient 16,
; posé sur la pile. Rien ne vérifie la taille : c'est le bogue, et il a
; l'air de rien.
;
; La marche à suivre :
;   1. Outils > Motif cyclique, longueur 64, « Vers l'entrée du
;      programme », puis Exécuter.
;   2. Le `ret` part dans le décor. Regardez RIP : il contient quatre
;      lettres du motif.
;   3. Rouvrez le motif cyclique, cliquez RIP, lisez le décalage : c'est
;      la distance, en octets, entre le début du tampon et l'adresse de
;      retour.
;   4. Onglet Pile, « Adresses de retour dans la pile » : avant de lire,
;      il y en a une ; après, elle a disparu.
;
; Rien de tout cela n'est un exploit : c'est la mesure d'un dégât. Les
; protections que namsdec sait détecter — canari de pile, NX, PIE — sont
; les réponses à ce qu'on voit ici.

default rel
global _start

section .text
_start:
    call lire_le_nom
    hlt

lire_le_nom:
    sub rsp, 16             ; le tampon : seize octets, pas un de plus
    mov rax, 0              ; read
    mov rdi, 0              ; l'entrée standard
    mov rsi, rsp            ; où écrire : le tampon
    mov rdx, 64             ; combien : quatre fois trop
    syscall

    add rsp, 16
    ret                     ; par où repart-on, au juste ?
