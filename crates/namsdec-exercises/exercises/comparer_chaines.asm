; Deux chaînes sont-elles les mêmes ?
;
; But : à l'arrêt, RAX doit valoir 1 — les deux premières chaînes sont
; identiques — et RBX 0, car la troisième diffère.
;
; `cmpsb` compare l'octet que RSI désigne à celui que RDI désigne, avance
; les deux, et pose les indicateurs. `repe` répète *tant que* les octets
; sont égaux et que RCX n'est pas épuisé : à la sortie, `ZF` dit si l'on
; s'est arrêté sur une différence ou parce qu'on avait tout comparé.
;
; C'est `memcmp` en trois instructions. La vraie `memcmp` d'une
; bibliothèque C fait la même chose seize octets à la fois, avec SSE —
; l'étape « Quatre additions d'un coup » montre comment.

default rel
global _start

section .data
    un    db "salut"
    deux  db "salut"
    trois db "salue"

section .text
_start:
    ; « salut » contre « salut »
    lea rsi, [un]
    lea rdi, [deux]
    mov rcx, 5
    cld
    repe cmpsb
    sete al                 ; ZF posé : tout était égal
    movzx rax, al

    ; « salut » contre « salue »
    lea rsi, [un]
    lea rdi, [trois]
    mov rcx, 5
    repe cmpsb
    sete bl
    movzx rbx, bl

    hlt
