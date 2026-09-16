; Parler au système.
;
; But : la console doit afficher le message, et le programme se terminer
; avec le code 0.
;
; Un appel système : le numéro dans RAX, les arguments dans RDI, RSI,
; RDX — la même convention que pour un appel ordinaire, à ceci près que
; c'est le noyau qui répond. namsdec en simule quatre : read (0),
; write (1), brk (12) et exit (60).

global _start

section .data
    message  db "Je parle au processeur !", 10
    longueur equ $ - message

section .text
_start:
    mov rax, 1              ; write
    mov rdi, 1              ; sortie standard
    mov rsi, message        ; quoi
    mov rdx, longueur       ; combien
    syscall

    ; À faire : terminer proprement avec exit (60) et le code 0,
    ; au lieu du `hlt` ci-dessous.
    hlt
