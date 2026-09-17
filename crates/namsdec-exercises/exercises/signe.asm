; Le même nombre, deux lectures.
;
; But : à l'arrêt, RCX doit valoir 1 et RDX 0.
;
; Un registre ne contient que des bits : c'est la *comparaison* qui
; décide s'ils forment un nombre signé ou non. 0xFFFFFFFFFFFFFFFF est à
; la fois -1 et le plus grand entier non signé qui soit.
;
; `ja` lit CF (comparaison non signée), `jg` lit SF et OF (comparaison
; signée). Sur les mêmes octets, l'un saute et l'autre pas.

global _start

section .text
_start:
    mov rax, -1             ; soit 0xFFFFFFFFFFFFFFFF
    mov rbx, 1
    xor rcx, rcx
    xor rdx, rdx

    cmp rax, rbx
    ja plus_grand_non_signe ; non signé : -1 est le plus grand de tous
    jmp suite
plus_grand_non_signe:
    mov rcx, 1

suite:
    cmp rax, rbx
    jg plus_grand_signe     ; signé : -1 est bien plus petit que 1
    jmp fin
plus_grand_signe:
    mov rdx, 1

fin:
    hlt
