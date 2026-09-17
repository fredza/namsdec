; Mettre un registre à zéro : trois façons, un seul gagnant.
;
; But : à l'arrêt, RAX, RBX et RCX doivent tous valoir 0 — ce qui est
; facile. L'exercice est ailleurs : regardez l'onglet Octets et comparez
; ce que chaque ligne a coûté en octets.
;
; `mov rax, 0` est la façon évidente : sept octets, parce que le zéro est
; écrit en toutes lettres dans l'instruction. `xor eax, eax` en fait
; deux, et met bien RAX entier à zéro — écrire dans la moitié basse d'un
; registre 32 bits efface la haute, souvenez-vous de la première étape.
; `sub rax, rax` en fait trois.
;
; C'est pour cela qu'un binaire compilé est rempli de `xor` là où le code
; source disait `= 0`. Le résultat est le même ; la place prise, non.

global _start

section .text
_start:
    mov rax, 0              ; sept octets
    xor ebx, ebx            ; deux octets, et RBX entier est à zéro
    sub rcx, rcx            ; trois octets

    hlt
