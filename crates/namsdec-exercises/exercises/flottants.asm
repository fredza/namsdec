; Calculer avec des nombres à virgule.
;
; But : à l'arrêt, `resultat` doit contenir 3.75, et RAX ses octets —
; 0x400E000000000000. Survolez RAX : namsdec en donne la lecture
; décimale.
;
; Les entiers vivent dans RAX et ses frères ; les flottants, dans XMM0 à
; XMM15. `movsd` déplace un double (64 bits), `addsd` en additionne deux,
; et le `s` de `sd` veut dire *scalar* : un seul nombre, alors que la
; même instruction sans `s` en traiterait deux d'un coup.
;
; Ouvrez « Registres étendus » dans le bandeau : XMM0 y est montré en
; hexadécimal et en décimal, parce que personne ne lit 400E000000000000
; comme « 3,75 ».

default rel
global _start

section .data
    a         dq 1.5
    b         dq 2.25
    resultat  dq 0.0

section .text
_start:
    movsd xmm0, [a]         ; 1.5
    addsd xmm0, [b]         ; + 2.25
    movsd [resultat], xmm0  ; = 3.75

    mov rax, [resultat]     ; les octets du double, pour les voir

    hlt
