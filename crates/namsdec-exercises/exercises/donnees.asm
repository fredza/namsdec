; Déclarer des données, et voir ce que ça produit.
;
; But : à l'arrêt, RAX doit valoir 0x44332211, RBX 5 et RCX 0x0201.
;
; `db` pose un octet, `dw` deux, `dd` quatre, `dq` huit — et ce sont les
; mêmes noms que les tailles d'opérande (`byte`, `word`, `dword`,
; `qword`). `times` répète, `equ` donne un nom à une valeur, et `$`
; désigne l'adresse courante : `$ - message` est donc la longueur de ce
; qui précède, calculée par l'assembleur et non par le programme.
;
; Ouvrez l'onglet Octets : il montre, ligne par ligne, ce que chaque
; directive a réellement écrit dans le fichier. C'est la seule façon de
; vérifier qu'on a compris — et de découvrir le petit-boutisme là où on
; ne l'attendait pas.

default rel
global _start

section .data
    octets    db 0x11, 0x22, 0x33, 0x44
    mot       dw 0x0201
    message   db "salut"
    longueur  equ $ - message        ; l'assembleur compte pour nous
    remplis   times 4 db 0xff

section .text
_start:
    mov eax, [octets]       ; quatre octets d'un coup : 0x44332211
    mov rbx, longueur       ; une constante, pas une lecture mémoire
    movzx rcx, word [mot]

    hlt
