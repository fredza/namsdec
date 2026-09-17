; Ce qu'une fonction a le droit d'abîmer.
;
; But : à l'arrêt, RBX doit valoir 7 — la valeur qu'on lui avait donnée
; *avant* l'appel — et l'onglet Pile ne doit signaler aucune entorse.
;
; La convention System V partage les registres en deux : ceux qu'une
; fonction peut écraser librement (RAX, RCX, RDX, RSI, RDI, R8 à R11) et
; ceux qu'elle doit rendre intacts (RBX, RBP, R12 à R15). Personne ne
; l'impose — le processeur s'en moque — mais tout le monde s'y tient,
; sinon plus rien ne s'appelle.
;
; La fonction ci-dessous triche : elle écrase RBX. namsdec le remarque et
; le dit dans l'onglet Pile, sous « Entorses à la convention ».
;
; À faire : rendez-la honnête. Empilez RBX au début, dépilez-le à la fin
; — et regardez l'entorse disparaître.

global _start

section .text
_start:
    mov rbx, 7              ; une valeur qui doit survivre à l'appel
    mov rdi, 3
    call triche
    hlt

; Elle calcule juste, mais elle abîme RBX au passage.
triche:
    mov rbx, 99             ; un registre qu'elle devait rendre intact
    mov rax, rdi
    add rax, rbx
    ret
