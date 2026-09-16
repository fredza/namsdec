; Appeler une sous-routine, et lui rendre ce qu'on lui doit.
;
; But : RAX doit valoir 7 à l'arrêt, et l'onglet Pile ne doit signaler
; aucune entorse à la convention.
;
; Les six premiers arguments passent par RDI, RSI, RDX, RCX, R8, R9 ;
; le résultat revient dans RAX. RBX, RBP et R12 à R15 sont prêtés à
; l'appelé, qui doit les rendre intacts — namsdec le vérifie.

global _start

section .text
_start:
    mov rbx, 99             ; à nous : la routine doit le rendre tel quel
    mov rdi, 3              ; premier argument
    mov rsi, 4              ; second argument
    call somme
    hlt

somme:
    ; À corriger : cette routine écrase RBX sans le rendre. Ajoutez le
    ; `push rbx` / `pop rbx` qui manque, et l'entorse disparaîtra de
    ; l'onglet Pile.
    mov rbx, rdi
    add rbx, rsi
    mov rax, rbx
    ret
