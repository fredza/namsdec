; Un cadre de pile, et des variables locales.
;
; But : à l'arrêt, RAX doit valoir 7 et RSP être revenu là où il était
; avant l'appel — l'onglet Pile ne doit signaler aucune entorse.
;
; Une fonction qui a besoin de place la prend sur la pile : `push rbp`
; met de côté le cadre de l'appelant, `mov rbp, rsp` pose le sien, et
; `sub rsp, N` réserve N octets. Les variables locales se lisent alors
; par rapport à RBP — `[rbp - 8]`, `[rbp - 16]` — et non par rapport à
; RSP, qui bouge dès qu'on empile quoi que ce soit.
;
; `leave` défait le tout en une instruction : `mov rsp, rbp` puis
; `pop rbp`. C'est pour cela qu'on la rencontre juste avant `ret`.

global _start

section .text
_start:
    call somme_locale
    hlt

somme_locale:
    push rbp
    mov rbp, rsp
    sub rsp, 16             ; deux variables de huit octets

    mov qword [rbp - 8], 3  ; la première
    mov qword [rbp - 16], 4 ; la seconde

    mov rax, [rbp - 8]
    add rax, [rbp - 16]     ; 3 + 4

    leave                   ; mov rsp, rbp ; pop rbp
    ret
