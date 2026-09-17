; Un nombre négatif, vu de près.
;
; But : à l'arrêt, RAX doit valoir -5 (0xFFFFFFFFFFFFFFFB), RBX 5, et RCX
; 1 — le drapeau « RAX était négatif ».
;
; Il n'y a pas de bit « moins » : un nombre négatif est le complément à
; deux de son opposé — on inverse tous les bits et on ajoute un. C'est ce
; que fait `neg`, et c'est pourquoi -1 s'écrit avec tous les bits à un.
;
; L'avantage est énorme : l'addition fonctionne sans rien changer.
; 5 + (-5) donne zéro par le simple jeu des retenues, et le processeur
; n'a qu'un seul additionneur.
;
; Survolez RAX : namsdec en donne la lecture signée et non signée. Le
; même motif de bits vaut -5 ou 18 446 744 073 709 551 611, selon la
; question qu'on pose.

global _start

section .text
_start:
    mov rax, 5
    neg rax                 ; 5 devient -5

    mov rbx, rax
    neg rbx                 ; et revient à 5

    ; « Est-il négatif ? » se lit sur le bit de poids fort.
    mov rcx, rax
    shr rcx, 63             ; le bit de signe, ramené en bas

    hlt
