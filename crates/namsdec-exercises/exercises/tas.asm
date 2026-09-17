; Demander de la mémoire au noyau.
;
; But : à l'arrêt, R8 doit contenir l'adresse rendue par le premier
; `brk`, R9 une adresse plus haute, et RAX l'octet 0x42 relu dans la
; mémoire obtenue.
;
; Un programme naît avec son code, ses données et sa pile. Le reste — le
; tas — se demande : `brk(0)` rend la limite actuelle, `brk(nouvelle)`
; la déplace. Entre l'ancienne et la nouvelle, la mémoire est à vous.
;
; C'est sur cet appel que repose tout `malloc`, et c'est le seul appel
; « mémoire » qu'un programme écrit à la main utilise sans bibliothèque.
; L'onglet Tas montre les trois bornes : début, rupture, plafond.

global _start

section .text
_start:
    ; Où en est la rupture ?
    mov rax, 12             ; brk
    mov rdi, 0              ; 0 : ne rien déplacer, seulement demander
    syscall
    mov r8, rax             ; la limite actuelle

    ; Quatre kilo-octets de plus.
    lea rdi, [r8 + 0x1000]
    mov rax, 12
    syscall
    mov r9, rax             ; la nouvelle limite

    ; Et la mémoire obtenue s'écrit comme n'importe quelle autre.
    mov byte [r8], 0x42
    movzx rax, byte [r8]

    hlt
