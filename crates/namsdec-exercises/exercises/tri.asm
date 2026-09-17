; Trier un tableau, et voir ce que ça coûte.
;
; But : à l'arrêt, RAX doit valoir 11 — le plus petit, en tête — et RBX
; 95, le plus grand, en queue.
;
; Le tri à bulles : on parcourt le tableau, on échange deux voisins mal
; rangés, et on recommence tant qu'on a échangé quelque chose. C'est le
; tri le plus simple à écrire et le plus lent qui soit — six éléments se
; paient une trentaine de comparaisons.
;
; Ouvrez l'onglet **Profil** de la console après l'exécution : la
; comparaison intérieure porte à elle seule la plus grosse part du
; temps. C'est ce que dit la notation O(n²) — et on le voit, ligne par
; ligne, au lieu de le croire sur parole.
;
; À faire ensuite : arrêtez le parcours un cran plus tôt à chaque tour
; (le dernier élément est déjà à sa place) et regardez la part de la
; comparaison baisser dans le profil.

default rel
global _start

section .data
    nombres dq 42, 17, 93, 11, 68, 95
    combien equ 6

section .text
_start:
    lea rsi, [nombres]

tour:
    xor r8, r8              ; a-t-on échangé pendant ce tour ?
    xor rcx, rcx            ; l'indice

parcours:
    mov rdx, rcx
    inc rdx
    cmp rdx, combien
    jae fin_tour            ; plus de voisin à droite

    mov rax, [rsi + rcx*8]
    mov r9, [rsi + rdx*8]
    cmp rax, r9
    jbe suivant             ; déjà dans l'ordre

    ; L'échange : deux écritures, aucune variable temporaire en mémoire.
    mov [rsi + rcx*8], r9
    mov [rsi + rdx*8], rax
    mov r8, 1

suivant:
    inc rcx
    jmp parcours

fin_tour:
    test r8, r8
    jnz tour                ; un échange : il faut refaire un tour

    ; Le résultat : le plus petit en tête, le plus grand en queue.
    mov rax, [rsi]
    mov rbx, [rsi + (combien - 1)*8]
    hlt
