; Quatre additions d'un coup.
;
; But : à l'arrêt, RAX doit valoir 11 — la première somme — et RBX 44, la
; dernière.
;
; Un registre XMM tient 128 bits, soit quatre entiers de 32 bits. `paddd`
; les additionne **tous les quatre en même temps** : le `p` dit *packed*,
; le `d` dit *doubleword*. Là où une boucle ferait quatre tours, une
; instruction suffit.
;
; C'est le principe de tout le calcul vectoriel — image, son, matrices —
; et la raison pour laquelle un même programme peut aller quatre fois
; plus vite sans changer d'algorithme.
;
; `movdqa` demande une adresse alignée sur seize octets : c'est à quoi
; sert le `align 16` ci-dessous. Retirez-le pour voir ce que le
; processeur en dit.

default rel
global _start

section .data
    align 16
    gauche  dd 1, 2, 3, 4
    droite  dd 10, 20, 30, 40
    somme   dd 0, 0, 0, 0

section .text
_start:
    movdqa xmm0, [gauche]   ; les quatre nombres d'un coup
    paddd  xmm0, [droite]   ; les quatre additions d'un coup
    movdqa [somme], xmm0

    mov eax, [somme]        ; 1 + 10
    mov ebx, [somme + 12]   ; 4 + 40

    hlt
