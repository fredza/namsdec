; Multiplier et diviser.
;
; But : à l'arrêt, RAX doit valoir 7 et RDX 1 (le reste de 43 ÷ 6).
;
; La multiplication et la division ne ressemblent à aucune autre
; opération : elles travaillent sur *deux* registres à la fois. `mul`
; écrit RDX:RAX — le haut et le bas d'un résultat de 128 bits. `div`
; lit la même paire et la divise, en rendant le quotient dans RAX et le
; reste dans RDX.
;
; D'où l'erreur classique : oublier de mettre RDX à zéro avant un `div`.
; Essayez de retirer le `xor rdx, rdx` ci-dessous — namsdec vous dira
; exactement ce qui s'est passé.

global _start

section .text
_start:
    mov rax, 6
    mov rbx, 7
    mul rbx                 ; RDX:RAX = 6 × 7 = 42 ; RDX reste à 0

    add rax, 1              ; 43, pour avoir un reste à la division

    xor rdx, rdx            ; la moitié haute du dividende, à zéro
    mov rbx, 6
    div rbx                 ; RAX = 43 ÷ 6 = 7, RDX = 43 mod 6 = 1

    hlt
