; Lire et écrire en mémoire.
;
; But : R8 doit valoir 0x44332211 et R9 l'octet 0x11.
;
; Ouvrez l'onglet Mémoire et tapez « tampon » — non, tapez son adresse,
; ou « rdi » après la ligne 14 : le champ accepte les registres. Vous
; verrez les octets changer au fur et à mesure des pas.

default rel
global _start

section .data
    tampon  times 16 db 0
    valeurs dd 0x44332211, 0x88776655

section .text
_start:
    mov eax, [valeurs]      ; quatre octets, d'un coup
    mov [tampon], eax
    mov r8d, [tampon]

    movzx r9, byte [tampon] ; un seul octet, élargi par des zéros

    mov rdi, tampon
    mov byte [rdi + 4], 0xFF    ; adressage indexé : base + décalage
    hlt
