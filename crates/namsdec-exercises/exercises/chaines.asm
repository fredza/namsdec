; Copier sans boucle : les instructions de chaîne.
;
; But : à l'arrêt, `copie` doit contenir « salut » et RCX valoir 0.
;
; `movsb` copie l'octet que RSI désigne vers celui que RDI désigne, puis
; avance les deux — d'un octet vers le haut si le drapeau de direction
; est effacé (`cld`), vers le bas s'il est posé (`std`). `rep` répète
; l'opération RCX fois en décomptant.
;
; Six instructions de chaîne suivent la même règle : `movs` copie,
; `stos` écrit, `lods` lit, `scas` cherche, `cmps` compare. Elles
; expliquent pourquoi RSI et RDI s'appellent *source index* et
; *destination index*.
;
; Avancez pas à pas et regardez RSI, RDI et RCX dans le bandeau : c'est
; toute la mécanique, et elle se voit.

default rel
global _start

section .data
message db "salut", 0

section .bss
copie resb 8

section .text
_start:
    lea rsi, [message]      ; d'où l'on copie
    lea rdi, [copie]        ; où l'on copie
    mov rcx, 6              ; cinq lettres et le zéro final
    cld                     ; vers les adresses hautes

    rep movsb               ; et voilà la boucle, en une instruction

    ; De quoi vérifier sans l'onglet Mémoire : la première lettre et la
    ; dernière.
    movzx rax, byte [copie]
    movzx rbx, byte [copie + 4]

    hlt
