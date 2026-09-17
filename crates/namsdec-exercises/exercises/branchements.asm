; Choisir parmi plusieurs : la table de saut.
;
; But : à l'arrêt, RAX doit valoir 300 — la troisième branche, celle que
; l'indice 2 désigne.
;
; Une suite de `cmp` et de `je` coûte un test par branche : la dixième
; possibilité se paie dix comparaisons. Une table de saut n'en coûte
; aucune — on range les adresses des branches dans un tableau, et l'on
; saute à celle que l'indice désigne. C'est ce qu'un compilateur produit
; pour un `switch` dense, et c'est pourquoi un `switch` de vingt cas
; n'est pas vingt fois plus lent qu'un `if`.
;
; Le prix à payer : rien ne vérifie l'indice. Un indice hors bornes lit
; une adresse au hasard et saute dedans — c'est très exactement ce que
; les protections des binaires modernes cherchent à rendre impossible.
;
; Changez l'indice et relancez : le saut suit.

default rel
global _start

section .data
    branches dq cas_zero, cas_un, cas_deux

section .text
_start:
    mov rcx, 2              ; l'indice : essayez 0, 1, puis 2
    lea rdx, [branches]
    jmp [rdx + rcx*8]       ; le saut, sans une seule comparaison

cas_zero:
    mov rax, 100
    jmp fin
cas_un:
    mov rax, 200
    jmp fin
cas_deux:
    mov rax, 300

fin:
    hlt
