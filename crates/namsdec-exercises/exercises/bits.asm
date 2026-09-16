; Les bits, un par un.
;
; But : à l'arrêt, RAX doit valoir 40 et RBX 1.
;
; Décaler à gauche multiplie par deux, décaler à droite divise — et
; c'est ce que fait le processeur quand vous écrivez une multiplication
; par une puissance de deux. `and` sert à garder des bits, `or` à en
; poser, `xor` à en retourner : les trois outils avec lesquels on range
; plusieurs informations dans un seul registre.
;
; Survolez la valeur de RAX dans le bandeau : sa lecture binaire montre
; les bits bouger.

global _start

section .text
_start:
    mov rax, 5
    shl rax, 3              ; 5 × 8 = 40 — trois décalages, trois × 2

    ; Le bit 0 de RAX : 40 est pair, il vaut donc 0.
    mov rbx, rax
    and rbx, 1              ; ne garder que le bit 0

    ; Le poser, puis vérifier qu'il y est.
    or rbx, 1               ; RBX vaut 1

    ; `test` est un `and` qui ne garde pas son résultat : il ne pose que
    ; les indicateurs. C'est ainsi qu'on demande « ce bit est-il mis ? »
    ; sans abîmer la valeur.
    test rax, 8             ; le bit 3 de 40 (101000 en binaire) est mis
    jz rate                 ; ZF=1 voudrait dire « pas mis »

    hlt

rate:
    ; On n'arrive ici que si le test s'est mal passé.
    mov rbx, -1
    hlt
