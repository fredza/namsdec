; Afficher un nombre : la synthèse.
;
; But : la console doit afficher 1234, et RAX valoir 4 — le nombre de
; chiffres écrits.
;
; Un ordinateur ne connaît pas les chiffres, il connaît les nombres.
; Afficher 1234, c'est le découper : diviser par dix rend 123 et le
; reste 4, redemander rend 12 et 3, et ainsi jusqu'à zéro. Les chiffres
; sortent donc **à l'envers** — on les empile au fur et à mesure, et la
; pile les rend dans le bon ordre.
;
; Tout ce que le parcours a montré se retrouve ici : une boucle, `div`
; et son RDX, la pile, l'arithmétique sur les caractères ('0' + chiffre)
; et un appel système. C'est le premier programme utile qu'on écrit.

default rel
global _start

section .bss
tampon resb 20

section .text
_start:
    mov rax, 1234           ; le nombre à écrire
    lea rdi, [tampon]
    mov rcx, 10             ; la base
    xor r8, r8              ; combien de chiffres

decouper:
    xor rdx, rdx            ; RDX:RAX — le haut doit être nul
    div rcx                 ; RAX = quotient, RDX = reste
    add rdx, '0'            ; le chiffre, en caractère
    push rdx                ; à l'envers : la pile remettra l'ordre
    inc r8
    test rax, rax
    jnz decouper

ranger:
    pop rdx                 ; le chiffre de poids fort en premier
    mov [rdi], dl
    inc rdi
    dec r8
    jnz ranger

    ; Ce que l'on vient d'écrire, moins le tampon, donne la longueur.
    lea rsi, [tampon]
    mov rdx, rdi
    sub rdx, rsi            ; combien de caractères

    mov rax, 1              ; write
    mov rdi, 1              ; sortie standard
    syscall                 ; RAX reçoit le nombre d'octets écrits

    hlt
