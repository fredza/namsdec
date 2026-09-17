; Multiplier et diviser sans multiplier.
;
; But : à l'arrêt, RAX doit valoir 40, RBX 5, RCX 0xFFFFFFFFFFFFFFFE et
; RDX 0x7FFFFFFFFFFFFFFF.
;
; Décaler à gauche d'un cran, c'est multiplier par deux ; à droite,
; diviser par deux. Un processeur le fait en un souffle là où `mul`
; demande plusieurs cycles, et c'est pourquoi un compilateur remplace
; `x * 8` par `x << 3` sans qu'on le lui demande.
;
; Mais il y a deux décalages à droite, et confondre les deux coûte cher :
; `shr` fait entrer des zéros — bon pour un nombre non signé —, `sar`
; recopie le bit de signe — bon pour un nombre signé. Sur -4, `sar` rend
; -2 ; `shr` rend un nombre positif énorme. Les deux sont ici, côte à
; côte.

global _start

section .text
_start:
    mov rax, 5
    shl rax, 3              ; 5 x 8 = 40

    mov rbx, 40
    shr rbx, 3              ; 40 / 8 = 5

    mov rcx, -4
    sar rcx, 1              ; -4 / 2 = -2, le signe est conservé

    mov rdx, -2
    shr rdx, 1              ; le même décalage, sans le signe

    hlt
