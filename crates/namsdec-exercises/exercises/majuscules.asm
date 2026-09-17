; Mettre un texte en majuscules.
;
; But : à l'arrêt, la console doit afficher SALUT et RAX valoir 6 — les
; cinq lettres et le saut de ligne.
;
; Une lettre minuscule et sa majuscule ne diffèrent que d'un bit :
; 'a' vaut 0x61, 'A' vaut 0x41, et 0x20 est le seul bit qui change. C'est
; un choix délibéré de la table ASCII, fait pour que la conversion tienne
; en une instruction — `and` avec 0xDF, ou `sub` de 32.
;
; Attention à ce qui n'est pas une lettre : le saut de ligne vaut 0x0A, et
; lui retirer 32 donnerait n'importe quoi. Le programme ne traite donc que
; ce qui est entre 'a' et 'z' — et c'est exactement ce que fait
; `toupper` de la bibliothèque C.

default rel
global _start

section .data
    texte db "salut", 10
    taille equ $ - texte

section .text
_start:
    lea rsi, [texte]
    xor rcx, rcx

boucle:
    cmp rcx, taille
    jae ecrire

    movzx rax, byte [rsi + rcx]
    cmp al, 'a'
    jb suivant              ; avant 'a' : on n'y touche pas
    cmp al, 'z'
    ja suivant              ; après 'z' non plus
    and al, 0xdf            ; le bit de la casse, effacé
    mov [rsi + rcx], al

suivant:
    inc rcx
    jmp boucle

ecrire:
    mov rax, 1              ; write
    mov rdi, 1
    lea rsi, [texte]
    mov rdx, taille
    syscall

    hlt
