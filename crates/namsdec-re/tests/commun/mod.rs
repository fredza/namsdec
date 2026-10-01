//! Ce que les contrôles de ce dossier partagent : un ELF d'épreuve.
//!
//! Fabriqué ici et non lu sur le disque. Un contrôle qui part de
//! `/bin/true` et s'abstient quand il ne le trouve pas passe sans rien
//! prouver sur une machine qui n'en a pas — et la CI n'est pas la
//! machine de qui l'écrit. Celui-ci tourne partout de la même façon.

/// L'adresse où namsdec pose ce qu'il charge.
#[allow(dead_code)]
pub const BASE: u64 = 0x400000;

/// Un ELF64 complet, fabriqué pour mener l'analyse jusque dans ses
/// tables.
///
/// Rien d'exécutable au sens strict — on ne l'exécute jamais — mais tout
/// ce qu'un analyseur suit : deux segments (dont un inscriptible *et*
/// exécutable, pour que le panneau des signaux ait quelque chose à
/// dire), une table de sections, une table de symboles dynamiques, ses
/// chaînes, deux relocations de liaison et les deux entrées de `.plt`
/// qui leur répondent, du code, et des données qui portent un indicateur.
///
/// Les décalages sont écrits en clair : c'est un fichier, pas une
/// structure de données, et le lire se fait en regardant les nombres.
#[allow(dead_code)]
pub fn germe() -> Vec<u8> {
    const TEXT: usize = 0x100;
    const PLT: usize = 0x120;
    const RELA: usize = 0x140;
    const DYNSYM: usize = 0x180;
    const DYNSTR: usize = 0x1d0;
    const GOT: usize = 0x1f0;
    const RODATA: usize = 0x200;
    const SHSTR: usize = 0x300;
    const SHOFF: usize = 0x400;
    const TAILLE: usize = SHOFF + 9 * 64;

    let mut f = vec![0u8; TAILLE];
    let pose = |at: usize, octets: &[u8], f: &mut Vec<u8>| {
        f[at..at + octets.len()].copy_from_slice(octets);
    };

    // --- L'en-tête ------------------------------------------------
    pose(0, b"\x7fELF\x02\x01\x01\x00", &mut f);
    pose(0x10, &2u16.to_le_bytes(), &mut f); // ET_EXEC
    pose(0x12, &0x3eu16.to_le_bytes(), &mut f); // x86-64
    pose(0x14, &1u32.to_le_bytes(), &mut f);
    pose(0x18, &(BASE + TEXT as u64).to_le_bytes(), &mut f); // e_entry
    pose(0x20, &0x40u64.to_le_bytes(), &mut f); // e_phoff
    pose(0x28, &(SHOFF as u64).to_le_bytes(), &mut f); // e_shoff
    pose(0x34, &64u16.to_le_bytes(), &mut f); // e_ehsize
    pose(0x36, &56u16.to_le_bytes(), &mut f); // e_phentsize
    pose(0x38, &2u16.to_le_bytes(), &mut f); // e_phnum
    pose(0x3a, &64u16.to_le_bytes(), &mut f); // e_shentsize
    pose(0x3c, &9u16.to_le_bytes(), &mut f); // e_shnum
    pose(0x3e, &8u16.to_le_bytes(), &mut f); // e_shstrndx

    // --- Les segments ---------------------------------------------
    let segment = |index: usize, kind: u32, flags: u32, f: &mut Vec<u8>| {
        let at = 0x40 + index * 56;
        f[at..at + 4].copy_from_slice(&kind.to_le_bytes());
        f[at + 4..at + 8].copy_from_slice(&flags.to_le_bytes());
        f[at + 0x08..at + 0x10].copy_from_slice(&0u64.to_le_bytes()); // p_offset
        f[at + 0x10..at + 0x18].copy_from_slice(&BASE.to_le_bytes()); // p_vaddr
        f[at + 0x20..at + 0x28].copy_from_slice(&(TAILLE as u64).to_le_bytes()); // p_filesz
        f[at + 0x28..at + 0x30].copy_from_slice(&(TAILLE as u64).to_le_bytes()); // p_memsz
        f[at + 0x30..at + 0x38].copy_from_slice(&0x1000u64.to_le_bytes());
    };
    // Inscriptible, lisible *et* exécutable : de quoi donner au panneau
    // des signaux une ligne à relever sur un fichier valide.
    segment(0, 1, 7, &mut f); // PT_LOAD rwx
    segment(1, 0x6474_e551, 6, &mut f); // PT_GNU_STACK rw — NX en place

    // --- Le code --------------------------------------------------
    //
    // `lea rdi, [rip+…]` / `call` vers la première entrée de liaison /
    // `mov eax, 101` / `syscall` / `ret`. De quoi faire travailler le
    // désassembleur, les références croisées, le graphe de flot, les
    // appels système et la détection de fonctions.
    pose(
        TEXT,
        &[
            0x48, 0x8d, 0x3d, 0xf6, 0x00, 0x00, 0x00, // lea rdi, [rip+0xf6]
            0xe8, 0x14, 0x00, 0x00, 0x00, // call .plt
            0xb8, 0x65, 0x00, 0x00, 0x00, // mov eax, 101 (ptrace)
            0x0f, 0x05, // syscall
            0xc3, // ret
        ],
        &mut f,
    );

    // --- Les entrées de liaison -----------------------------------
    //
    // `jmp [rip + d]`, où `d` mène à l'emplacement que l'éditeur de
    // liens remplira : c'est ce chemin-là que `imports::scan` suit pour
    // nommer un appel.
    for (index, slot) in [GOT, GOT + 8].into_iter().enumerate() {
        let at = PLT + index * 16;
        let après = (BASE + at as u64) + 6;
        let d = (BASE + slot as u64).wrapping_sub(après) as u32;
        f[at] = 0xff;
        f[at + 1] = 0x25;
        f[at + 2..at + 6].copy_from_slice(&d.to_le_bytes());
        // Le reste de l'entrée : des `nop`, comme un vrai éditeur de
        // liens y laisse du bourrage.
        for octet in f[at + 6..at + 16].iter_mut() {
            *octet = 0x90;
        }
    }

    // --- Les relocations ------------------------------------------
    for (index, (slot, sym)) in [(GOT, 1u64), (GOT + 8, 2u64)].into_iter().enumerate() {
        let at = RELA + index * 24;
        f[at..at + 8].copy_from_slice(&(BASE + slot as u64).to_le_bytes());
        // R_X86_64_JUMP_SLOT (7), porté par le symbole `sym`.
        f[at + 8..at + 16].copy_from_slice(&((sym << 32) | 7).to_le_bytes());
    }

    // --- Les symboles et leurs noms -------------------------------
    pose(DYNSTR, b"\0socket\0ptrace\0", &mut f);
    for (index, name) in [(1usize, 1u32), (2, 8)] {
        let at = DYNSYM + index * 24;
        f[at..at + 4].copy_from_slice(&name.to_le_bytes());
        f[at + 4] = 0x12; // GLOBAL FUNC
        f[at + 8..at + 16].copy_from_slice(&0u64.to_le_bytes());
    }

    // --- Des données qui portent quelque chose à trouver ----------
    //
    // Une de chaque nature d'indicateur : c'est ce qui permet au
    // contrôle des panneaux de voir toutes les rubriques, et au fuzzing
    // de descendre jusque dans la reconnaissance des formes.
    pose(
        RODATA,
        b"POST https://c2.exemple.xyz/depot HTTP/1.1\0\
/var/tmp/.cache/charge\0\
HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\0\
admin@exemple.ru\0\
203.0.113.42\0",
        &mut f,
    );

    // --- Les noms de section, puis la table -----------------------
    let noms: &[&str] = &[
        "",
        ".text",
        ".plt",
        ".rela.plt",
        ".dynsym",
        ".dynstr",
        ".got.plt",
        ".rodata",
        ".shstrtab",
    ];
    let mut table = Vec::new();
    let mut positions = Vec::new();
    for nom in noms {
        positions.push(table.len());
        table.extend_from_slice(nom.as_bytes());
        table.push(0);
    }
    pose(SHSTR, &table, &mut f);

    // (type, flags, addr/offset, taille, link, entsize)
    let sections: &[(u32, u64, usize, usize, u32, u64)] = &[
        (0, 0, 0, 0, 0, 0),
        (1, 0x6, TEXT, 0x20, 0, 0),       // .text   PROGBITS AX
        (1, 0x6, PLT, 0x20, 0, 16),       // .plt    PROGBITS AX
        (4, 0x2, RELA, 0x30, 4, 24),      // .rela.plt RELA -> .dynsym
        (11, 0x2, DYNSYM, 0x48, 5, 24),   // .dynsym DYNSYM -> .dynstr
        (3, 0x2, DYNSTR, 0x20, 0, 0),     // .dynstr STRTAB
        (1, 0x3, GOT, 0x10, 0, 8),        // .got.plt PROGBITS WA
        (1, 0x2, RODATA, 0x100, 0, 0),    // .rodata PROGBITS A
        (3, 0, SHSTR, table.len(), 0, 0), // .shstrtab STRTAB
    ];
    for (index, (kind, flags, offset, size, link, entsize)) in sections.iter().enumerate() {
        let at = SHOFF + index * 64;
        f[at..at + 4].copy_from_slice(&(positions[index] as u32).to_le_bytes());
        f[at + 0x04..at + 0x08].copy_from_slice(&kind.to_le_bytes());
        f[at + 0x08..at + 0x10].copy_from_slice(&flags.to_le_bytes());
        // Une section allouée porte une adresse ; les autres non.
        let addr = if flags & 2 != 0 {
            BASE + *offset as u64
        } else {
            0
        };
        f[at + 0x10..at + 0x18].copy_from_slice(&addr.to_le_bytes());
        f[at + 0x18..at + 0x20].copy_from_slice(&(*offset as u64).to_le_bytes());
        f[at + 0x20..at + 0x28].copy_from_slice(&(*size as u64).to_le_bytes());
        f[at + 0x28..at + 0x2c].copy_from_slice(&link.to_le_bytes());
        f[at + 0x30..at + 0x38].copy_from_slice(&8u64.to_le_bytes());
        f[at + 0x38..at + 0x40].copy_from_slice(&entsize.to_le_bytes());
    }
    f
}

/// Le même ELF, avec la figure classique de l'anti-désassemblage posée
/// dans son `.text`.
///
/// `allow(dead_code)` : chaque contrôle d'intégration compile ce module
/// pour son compte, et aucun ne se sert de tout. Sans cela, celui qui
/// n'emploie que `germe` échouerait sur ce qu'un autre emploie.
///
/// ```text
///     eb 01        jmp $+3       ← enjambe l'octet-piège
///     e8           db 0xE8       ← un opcode de `call`, qui avale la suite
///     31 c0        xor eax,eax   ← le vrai code
///     48 ff c0     inc rax
///     c3           ret
/// ```
///
/// La lecture ligne à ligne prend `e8` pour un `call`, engloutit quatre
/// octets d'adresse et repart de travers ; le processeur, lui, saute
/// par-dessus le piège. Un seul octet, et tout un désassemblage ment.
#[allow(dead_code)]
pub fn germe_piégé() -> Vec<u8> {
    const TEXT: usize = 0x100;
    let mut f = germe();
    let piège: &[u8] = &[
        0xEB, 0x01, // jmp $+3
        0xE8, // l'octet-piège
        0x31, 0xC0, // xor eax,eax
        0x48, 0xFF, 0xC0, // inc rax
        0xC3, // ret
    ];
    // Le reste de `.text` devient des `nop` : ce qui s'y trouvait
    // décoderait après le `ret` et brouillerait la lecture du flot.
    for octet in f[TEXT..TEXT + 0x20].iter_mut() {
        *octet = 0x90; // nop
    }
    // Et un `nop` juste AVANT `.text`, dans le bourrage.
    //
    // Sans lui, la figure est invisible — et pour une raison qui vaut
    // d'être dite. Le listing part du premier octet du FICHIER : il
    // traverse l'en-tête et les tables, décodés comme du code, et arrive
    // dans `.text` sur une frontière quelconque. Mesuré ici : il entrait
    // un octet trop tôt, lisait `00 eb` puis `01 e8`, et le `jmp` du
    // piège n'existait tout simplement pas pour lui.
    //
    // Un `nop` d'un octet remet la lecture en phase à `.text`. Ce n'est
    // pas un artifice de test : c'est exactement ce que fait du bourrage
    // à `0x90` dans un vrai binaire, et c'est pourquoi un éditeur de
    // liens en pose.
    f[TEXT - 1] = 0x90;
    f[TEXT..TEXT + piège.len()].copy_from_slice(piège);
    f
}
