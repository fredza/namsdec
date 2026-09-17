; Échanger deux registres.
;
; But : à l'arrêt, RAX doit valoir 2 et RBX 1 — sans passer par un
; troisième registre.
;
; `xchg` le fait en une instruction, et c'est la bonne réponse. Mais
; l'autre manière mérite d'être vue une fois : `xor` appliqué trois fois
; échange deux valeurs sans rien emprunter, parce que a xor b xor b
; redonne a. C'est une curiosité, pas un conseil — sur un processeur
; moderne, `xchg` est plus rapide et bien plus lisible.
;
; Essayez les deux, et regardez le compteur d'instructions : c'est lui
; qui tranche.

global _start

section .text
_start:
    mov rax, 1
    mov rbx, 2

    xchg rax, rbx           ; une instruction, et c'est fait

    ; L'autre chemin, à essayer en commentant celui du dessus :
    ;   xor rax, rbx
    ;   xor rbx, rax
    ;   xor rax, rbx

    hlt
