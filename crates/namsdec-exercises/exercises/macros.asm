; Ce qui se passe avant l'assemblage.
;
; But : à l'arrêt, RAX doit valoir 30 et RBX 12.
;
; Le préprocesseur de NASM travaille sur le *texte*, avant que la
; moindre instruction ne soit traduite. `%define` remplace un mot,
; `%macro` remplace un appel par les lignes qu'on lui a données, `%rep`
; répète un bloc. Rien de tout cela n'existe à l'exécution : le
; processeur ne voit que le résultat.
;
; Ouvrez l'onglet Octets après avoir assemblé : les trois `add` du `%rep`
; y sont, un par un. Une macro n'est pas un appel — il n'y a ni `call`,
; ni `ret`, ni pile : le code est recopié à chaque emploi.

global _start

%define DIX 10

%macro ajoute_deux_fois 1
    add rax, %1
    add rax, %1
%endmacro

section .text
_start:
    mov rax, DIX            ; le préprocesseur a écrit « mov rax, 10 »
    ajoute_deux_fois DIX    ; deux `add` recopiés ici

    mov rbx, 0
%rep 3
    add rbx, 4              ; trois fois la même ligne, écrites d'avance
%endrep

    hlt
