; Un tableau local, sur la pile.
;
; But : à l'arrêt, RAX doit valoir 6 — la somme de trois nombres rangés
; dans un tableau qui n'existe que le temps de la fonction.
;
; Les données d'une fonction n'ont pas à vivre dans `.data` : ce qui ne
; sert que pendant l'appel se range sur la pile, et disparaît en
; revenant. `sub rsp, 24` réserve trois emplacements, `[rsp]`, `[rsp+8]`
; et `[rsp+16]` les désignent, `add rsp, 24` les rend.
;
; C'est ce que fait un compilateur pour un tableau local, et c'est
; pourquoi un tableau local trop grand fait déborder la pile là où le
; même tableau en `.data` ne poserait aucun problème.
;
; Regardez RSP descendre puis remonter dans le bandeau, et l'onglet Pile
; montrer les trois valeurs apparaître.

global _start

section .text
_start:
    call somme_locale
    hlt

somme_locale:
    sub rsp, 24             ; trois emplacements de huit octets

    mov qword [rsp], 1
    mov qword [rsp + 8], 2
    mov qword [rsp + 16], 3

    mov rax, [rsp]
    add rax, [rsp + 8]
    add rax, [rsp + 16]

    add rsp, 24             ; on rend ce qu'on a pris
    ret
