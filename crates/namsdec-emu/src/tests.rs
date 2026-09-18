//! Les tests de l'interpréteur : ils passent par le vrai `nasm` et le
//! vrai `ld`, et vérifient ce que les instructions font au processeur
//! simulé — registres, indicateurs, mémoire, pile.
//!
//! Sortis de `lib.rs` pour la même raison qu'on sort le reste : deux
//! mille cinq cents lignes de tests au bas d'un fichier de code en
//! rendent les deux moitiés plus difficiles à lire.

use super::*;

/// Une instruction que l'interpréteur ne sait pas exécuter laisse RIP
/// où il est. Sans ça, réessayer avançait d'un cran à chaque fois et
/// l'exécution finissait par décoder les données comme du code.
#[test]
fn a_failed_step_leaves_rip_on_the_offending_instruction() {
    let mut mem = namsdec_core::Memory::new(0x400000, 0x1000);
    // `in al, dx` (EC) : parfaitement décodable, et jamais exécutée
    // ici — il n'y a pas de ports d'entrée-sortie dans une machine
    // qui n'a pas de matériel.
    mem.load(0x400000, &[0xec]).unwrap();
    let mut cpu = namsdec_core::Cpu::new(mem);
    cpu.regs.rip = 0x400000;
    let mut emu = Emulator::new(cpu);

    for attempt in 1..=3 {
        let err = emu.step().expect_err("`in` n'est pas supportée");
        assert!(
            matches!(err, EmuError::UnsupportedInstruction { .. }),
            "obtenu {err:?}"
        );
        assert_eq!(
            emu.cpu.regs.rip, 0x400000,
            "RIP a bougé à la tentative {attempt}"
        );
    }
}
use namsdec_core::Reg64;

const BASE: u64 = 0x400000;

/// Assemble `asm` avec `namsdec-asm`, le charge en mémoire et exécute
/// jusqu'à l'arrêt (halt, point d'arrêt ou limite de pas). Test
/// d'intégration "golden" : il passe par le vrai `nasm` externe.
fn run_program(asm: &str, breakpoints: &[u64]) -> Emulator {
    let mut emu = load_only(asm, breakpoints);
    emu.run(10_000).expect("exécution");
    emu
}

/// Le programme que tout le monde écrit en premier : il parle, puis
/// il sort. C'est exactement ce que `write` et `exit` servent à faire.
#[test]
fn a_program_can_write_to_stdout_and_exit_with_a_code() {
    let emu = run_program(
        "\
section .data
message db \"salut\", 10

section .text
global _start
_start:
mov rax, 1
mov rdi, 1
mov rsi, message
mov rdx, 6
syscall

mov rax, 60
mov rdi, 3
syscall
",
        &[],
    );

    assert_eq!(emu.output(), b"salut\n");
    assert_eq!(emu.exit_code(), Some(3));
    assert!(emu.cpu.halted, "`exit` arrête la machine");
}

/// `write` rend le nombre d'octets écrits, comme le noyau — un
/// programme peut donc le vérifier, et le `hlt` juste après laisse la
/// valeur observable dans RAX.
#[test]
fn write_returns_the_number_of_bytes_written() {
    let emu = run_program(
        "\
section .data
message db \"salut\", 10

section .text
global _start
_start:
mov rax, 1
mov rdi, 1
mov rsi, message
mov rdx, 6
syscall
hlt
",
        &[],
    );

    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
    assert_eq!(emu.output(), b"salut\n");
}

/// Un programme qui lit son entrée : tant qu'on ne lui a rien donné,
/// il attend — sans avancer d'un pouce, et sans bloquer l'interface.
/// Dès que l'entrée arrive, le même `syscall` repart et la lit.
#[test]
fn read_waits_for_input_then_consumes_it() {
    const PROGRAM: &str = "\
section .bss
tampon resb 32

section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, tampon
mov rdx, 32
syscall
hlt
";
    let mut emu = load_only(PROGRAM, &[]);

    // Le programme court jusqu'au `read`, puis s'arrête là.
    assert_eq!(emu.run(100).unwrap(), RunStop::NeedsInput);
    let waiting_at = emu.cpu.regs.rip;
    assert!(!emu.cpu.halted, "attendre n'est pas finir");

    // Insister ne sert à rien et ne casse rien : RIP ne bouge pas.
    assert_eq!(emu.step().unwrap(), StepResult::NeedsInput);
    assert_eq!(emu.cpu.regs.rip, waiting_at);

    emu.provide_input(b"bonjour\n");
    assert_eq!(emu.run(100).unwrap(), RunStop::Halted);

    // `read` a rendu le nombre d'octets lus...
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 8);
    // ...et les a bien déposés dans le tampon du programme.
    let buf = emu.cpu.regs.get64(Reg64::Rsi);
    assert_eq!(emu.cpu.mem.bytes_at(buf, 8).unwrap(), b"bonjour\n");
    assert_eq!(emu.pending_input(), 0, "tout a été consommé");
}

/// Une entrée close rend une fin de fichier (0) au lieu d'attendre —
/// l'équivalent d'un Ctrl+D. Sans ça, un programme qui lit jusqu'à la
/// fin de son entrée attendrait pour toujours.
#[test]
fn a_closed_input_reads_as_end_of_file() {
    let mut emu = load_only(
        "\
section .bss
tampon resb 8

section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, tampon
mov rdx, 8
syscall
hlt
",
        &[],
    );

    emu.close_input();
    assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rax),
        0,
        "attendu une fin de fichier"
    );
}

/// `read` rend ce qu'il a sous la main plutôt que d'attendre de quoi
/// remplir tout le tampon — c'est pourquoi un programme sérieux boucle
/// sur `read`.
#[test]
fn read_returns_what_is_available_without_filling_the_buffer() {
    let mut emu = load_only(
        "\
section .bss
tampon resb 64

section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, tampon
mov rdx, 64
syscall
hlt
",
        &[],
    );

    emu.provide_input(b"court");
    assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 5);
}

/// Lire depuis autre chose que l'entrée standard n'a pas de sens ici :
/// même réponse que le noyau pour un descripteur jamais ouvert.
#[test]
fn reading_from_an_unknown_file_descriptor_returns_ebadf() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 0
mov rdi, 7
mov rsi, 0
mov rdx, 8
syscall
hlt
",
        &[],
    );

    assert_eq!(emu.cpu.regs.get64(Reg64::Rax) as i64, -9);
}

/// `hlt` arrête la machine sans que le programme ait rendu quoi que ce
/// soit : il n'y a pas de code de sortie à inventer.
#[test]
fn halting_is_not_exiting() {
    let emu = run_program("section .text\n    mov rax, 1\n    hlt\n", &[]);
    assert!(emu.cpu.halted);
    assert_eq!(emu.exit_code(), None);
}

/// Il n'y a pas de système de fichiers derrière cet émulateur : tout
/// descripteur autre que 1 et 2 reçoit `-EBADF`, la réponse qu'aurait
/// donnée le noyau. Un programme en assembleur est censé savoir lire
/// ce retour négatif — c'est la convention, pas un détail d'émulation.
#[test]
fn writing_to_an_unknown_file_descriptor_returns_ebadf() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 1
mov rdi, 7
mov rsi, 0
mov rdx, 1
syscall
hlt
",
        &[],
    );

    assert_eq!(emu.cpu.regs.get64(Reg64::Rax) as i64, -9, "attendu -EBADF");
    assert!(emu.output().is_empty(), "rien ne doit avoir été écrit");
}

/// Un appel système que namsdec ne simule pas s'arrête en le disant.
/// Mieux vaut cet aveu qu'un code d'erreur inventé, qui laisserait
/// croire que l'appel a eu lieu.
#[test]
fn an_unimplemented_syscall_says_so() {
    // 59 = `execve` : remplacer le programme courant par un autre n'a
    // aucun sens dans une machine qui n'a ni système de fichiers ni
    // second processus.
    let mut emu = load_only(
        "section .text\n    global _start\n_start:\n    mov rax, 59\n    syscall\n    hlt\n",
        &[],
    );
    let entry = emu.cpu.regs.rip;

    let err = emu.run(100).expect_err("execve n'est pas simulé");
    match err {
        EmuError::UnsupportedSyscall { number, .. } => assert_eq!(number, 59),
        other => panic!("attendu UnsupportedSyscall, obtenu {other:?}"),
    }
    assert!(
        emu.cpu.regs.rip > entry,
        "les instructions avant le syscall se sont bien exécutées"
    );
    assert!(!emu.cpu.halted, "un appel refusé n'arrête pas la machine");
}

/// Un pointeur qui sort de la fenêtre mémoire simulée est une erreur,
/// pas une occasion de lire ce qui traîne à côté.
#[test]
fn writing_from_an_address_outside_memory_fails() {
    let mut emu = load_only(
        "\
section .text
global _start
_start:
mov rax, 1
mov rdi, 1
mov rsi, 0xdeadbeef
mov rdx, 4
syscall
hlt
",
        &[],
    );

    let err = emu
        .run(100)
        .expect_err("l'adresse est hors de la mémoire simulée");
    assert!(matches!(err, EmuError::Mem(_)), "obtenu {err:?}");
    assert!(emu.output().is_empty());
}

/// Un `write` de zéro octet est valide et ne regarde même pas le
/// pointeur — y compris quand celui-ci ne pointe nulle part.
#[test]
fn writing_zero_bytes_is_allowed_and_writes_nothing() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 1
mov rdi, 1
mov rsi, 0xdeadbeef
mov rdx, 0
syscall
hlt
",
        &[],
    );

    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
    assert!(emu.output().is_empty());
}

/// Charge sans exécuter — pour les programmes dont on veut observer
/// l'échec plutôt que le résultat.
/// `rep movsb` recopie une chaîne, un octet par pas — et le pas à
/// pas montre RSI, RDI et RCX avancer, ce qu'une boucle atomique
/// aurait caché.
#[test]
fn rep_movsb_copies_one_byte_per_step() {
    let mut emu = load_only(
        "\
section .data
source db \"salut\", 0

section .bss
cible resb 8

section .text
global _start
_start:
mov rsi, source
mov rdi, cible
mov rcx, 6
rep movsb
hlt
",
        &[],
    );
    // Les trois `mov`, puis la copie.
    for _ in 0..3 {
        emu.step().unwrap();
    }
    let (rsi, rdi) = (
        emu.cpu.regs.get64(Reg64::Rsi),
        emu.cpu.regs.get64(Reg64::Rdi),
    );
    let rip = emu.cpu.regs.rip;

    emu.step().unwrap();
    assert_eq!(
        emu.cpu.regs.rip, rip,
        "l'instruction se rejoue tant que RCX reste"
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 5, "un élément par pas");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), rsi + 1);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdi), rdi + 1);
    assert_eq!(
        emu.cpu.mem.read_u8(rdi).unwrap(),
        b's',
        "le premier octet est passé"
    );

    // Jusqu'au bout.
    emu.run(100).unwrap();
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0);
    assert_eq!(
        emu.cpu.mem.bytes_at(rdi, 6).unwrap(),
        b"salut\0",
        "la chaîne entière, terminateur compris"
    );
    assert!(emu.cpu.halted);
}

/// `rep` avec RCX à zéro ne fait *rien* — pas même une itération.
/// C'est la nuance qui distingue un compteur nul d'un compteur à un,
/// et le piège de tous les premiers `rep`.
#[test]
fn rep_with_a_zero_count_does_nothing_at_all() {
    let mut emu = load_only(
        "\
section .bss
cible resb 4

section .text
global _start
_start:
mov rdi, cible
mov rax, 0x41
xor rcx, rcx
rep stosb
hlt
",
        &[],
    );
    emu.run(100).unwrap();
    let cible = emu.cpu.regs.get64(Reg64::Rdi);
    assert_eq!(emu.cpu.mem.read_u8(cible).unwrap(), 0, "rien n'a été écrit");
    assert!(emu.cpu.halted);
}

/// `std` fait travailler les chaînes à reculons, `cld` les remet à
/// l'endroit — c'est tout ce que le drapeau de direction fait, et on
/// ne le voit que sur une instruction de chaîne.
#[test]
fn the_direction_flag_reverses_a_string_operation() {
    let mut emu = load_only(
        "\
section .bss
cible resb 8

section .text
global _start
_start:
mov rdi, cible
add rdi, 3          ; on part de la fin
mov rax, 0x41
mov rcx, 4
std
rep stosb           ; écrit vers les adresses décroissantes
cld
hlt
",
        &[],
    );
    emu.run(200).unwrap();
    let base = emu.cpu.regs.get64(Reg64::Rdi).wrapping_add(1);
    assert_eq!(
        emu.cpu.mem.bytes_at(base, 4).unwrap(),
        b"AAAA",
        "les quatre octets sont remplis, de la fin vers le début"
    );
    assert!(
        !emu.cpu.flags.contains(RFlags::DF),
        "`cld` a remis la direction à l'endroit"
    );
}

/// `repne scasb` cherche un octet : il s'arrête sur l'égalité, sans
/// attendre que RCX s'épuise. C'est ainsi qu'on mesure une chaîne
/// terminée par un zéro.
#[test]
fn repne_scasb_stops_on_the_byte_it_was_looking_for() {
    let mut emu = load_only(
        "\
section .data
texte db \"abc\", 0

section .text
global _start
_start:
mov rdi, texte
xor rax, rax        ; on cherche l'octet nul
mov rcx, 16
repne scasb
hlt
",
        &[],
    );
    emu.run(200).unwrap();
    // RDI s'est arrêté juste après le zéro, et RCX porte ce qui
    // restait : la longueur s'en déduit.
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rcx),
        12,
        "quatre octets parcourus"
    );
    assert!(
        emu.cpu.flags.contains(RFlags::ZF),
        "arrêté sur une égalité, pas sur l'épuisement du compteur"
    );
}

/// Le pas arrière défait une itération à la fois, la mémoire copiée
/// comprise : chaque élément est un pas, donc chaque élément se
/// défait.
#[test]
fn stepping_back_undoes_one_string_element_at_a_time() {
    let mut emu = load_only(
        "\
section .data
source db \"ab\"

section .bss
cible resb 4

section .text
global _start
_start:
mov rsi, source
mov rdi, cible
mov rcx, 2
rep movsb
hlt
",
        &[],
    );
    for _ in 0..3 {
        emu.step().unwrap();
    }
    let cible = emu.cpu.regs.get64(Reg64::Rdi);
    emu.step().unwrap();
    emu.step().unwrap();
    assert_eq!(emu.cpu.mem.bytes_at(cible, 2).unwrap(), b"ab");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0);

    emu.step_back();
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 1, "une itération défaite");
    assert_eq!(
        emu.cpu.mem.bytes_at(cible, 2).unwrap(),
        b"a\0",
        "et l'octet qu'elle avait écrit est repris"
    );
}

/// Un point de surveillance arrête l'exécution quand la mémoire
/// change sous lui — sans qu'on ait eu à deviner quelle instruction
/// le ferait, ce qui est tout l'intérêt.
#[test]
fn a_watchpoint_stops_the_run_when_the_value_changes() {
    let mut emu = load_only(
        "\
section .data
compteur dq 0

section .text
global _start
_start:
mov rax, 1
mov rbx, 2
mov rcx, 3
mov [compteur], rcx     ; c'est ici que ça change
mov rdx, 4
hlt
",
        &[],
    );
    let compteur = namsdec_asm::assemble(
        "section .data\n    compteur dq 0\nsection .text\n    global _start\n_start:\n    hlt\n",
        BASE,
    )
    .unwrap()
    .sections
    .iter()
    .find(|s| s.name == ".data")
    .expect("une section .data")
    .addr;

    emu.set_watchpoints(&[(compteur, 8)]);
    assert_eq!(emu.watchpoints().len(), 1);
    assert_eq!(
        emu.watchpoints()[0].value(),
        0,
        "armé sur la valeur du moment"
    );

    let stop = emu.run(100).expect("exécution");
    match stop {
        RunStop::Watchpoint { addr, old, new } => {
            assert_eq!(addr, compteur);
            assert_eq!((old, new), (0, 3), "la valeur d'avant et celle d'après");
        }
        other => panic!("attendu un arrêt sur surveillance, obtenu {other:?}"),
    }
    // Arrêté *après* l'instruction fautive, pas à la fin du programme.
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rdx),
        0,
        "`mov rdx, 4` n'a pas eu lieu"
    );
    assert!(!emu.cpu.halted);

    // Relancé, il va jusqu'au bout : la zone ne change plus.
    assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
}

/// Une écriture qui ne change rien n'arrête rien : `mov [x], 0` sur
/// une case déjà nulle n'apprend rien à personne.
#[test]
fn writing_the_same_value_does_not_trigger_a_watchpoint() {
    let mut emu = load_only(
        "\
section .bss
case resq 1

section .text
global _start
_start:
xor rax, rax
mov [case], rax
hlt
",
        &[],
    );
    let case = BASE + 0x1000; // .bss, la seule donnée
    emu.set_watchpoints(&[(case, 8)]);
    assert_eq!(emu.run(100).unwrap(), RunStop::Halted, "rien n'a changé");
}

/// Le retour en arrière réarme les zones surveillées : il défait, il
/// ne déclenche pas.
#[test]
fn stepping_back_rearms_the_watchpoints_instead_of_firing_them() {
    let mut emu = load_only(
        "\
section .bss
case resq 1

section .text
global _start
_start:
mov rax, 5
mov [case], rax
hlt
",
        &[],
    );
    let case = BASE + 0x1000;
    emu.set_watchpoints(&[(case, 8)]);
    assert!(matches!(
        emu.run(100).unwrap(),
        RunStop::Watchpoint { old: 0, new: 5, .. }
    ));

    // On revient avant l'écriture : la zone est réarmée sur zéro, et
    // rien ne s'est déclenché au passage.
    assert!(emu.step_back());
    assert_eq!(emu.watchpoints()[0].value(), 0);

    // Et rejouer le même pas la redéclenche.
    assert!(matches!(
        emu.run(100).unwrap(),
        RunStop::Watchpoint { old: 0, new: 5, .. }
    ));
}

/// La chaîne des appels se lit directement : l'émulateur voit passer
/// les `call` et les `ret`, il n'a pas à deviner les cadres en
/// suivant RBP — ce que le code d'un débutant ne monte pas.
#[test]
fn the_call_chain_follows_calls_and_returns() {
    let mut emu = load_only(
        "\
section .text
global _start
_start:
call externe
hlt

externe:
call interne
ret

interne:
mov rax, 1
ret
",
        &[],
    );
    assert!(emu.frames().is_empty(), "au départ, personne n'est appelé");

    emu.step().unwrap(); // call externe
    assert_eq!(emu.frames().len(), 1);
    assert_eq!(
        emu.frames()[0].call_site,
        BASE,
        "le `call` est la première instruction"
    );

    emu.step().unwrap(); // call interne
    assert_eq!(emu.frames().len(), 2, "deux appels imbriqués");
    let inner = emu.frames()[1];
    assert_eq!(inner.entry, emu.cpu.regs.rip, "on est entré dans `interne`");
    assert!(inner.call_site > emu.frames()[0].call_site);

    emu.step().unwrap(); // mov rax, 1
    emu.step().unwrap(); // ret
    assert_eq!(emu.frames().len(), 1, "le retour dépile son cadre");
    emu.step().unwrap(); // ret
    assert!(emu.frames().is_empty(), "de retour au programme principal");

    // Un `ret` de trop ne dépile rien qui n'existe pas.
    emu.cpu.regs.rip = inner.return_addr;
    assert!(emu.frames().is_empty());
}

/// Et le pas arrière remet la chaîne comme elle était : défaire un
/// `call` retire son cadre, défaire un `ret` le remet.
#[test]
fn stepping_back_restores_the_call_chain() {
    let mut emu = load_only(
        "\
section .text
global _start
_start:
call externe
hlt

externe:
mov rax, 1
ret
",
        &[],
    );
    emu.step().unwrap(); // call
    let called = emu.frames().to_vec();
    assert_eq!(called.len(), 1);

    emu.step().unwrap(); // mov
    emu.step().unwrap(); // ret
    assert!(emu.frames().is_empty());

    emu.step_back(); // défait le ret
    assert_eq!(emu.frames(), called.as_slice(), "le cadre est revenu");

    emu.step_back(); // défait le mov
    emu.step_back(); // défait le call
    assert!(emu.frames().is_empty(), "et il repart avec son `call`");
}

/// Lire et écrire *en mémoire* par un opérande — le pain quotidien
/// dès qu'on manipule un tableau.
///
/// Aucune de ces formes ne fonctionnait : iced réclame le registre de
/// segment pour calculer une adresse virtuelle, et l'interpréteur ne
/// savait pas quoi en faire. Il refusait donc l'instruction avant
/// même de toucher à la mémoire — et rien ne le disait, faute d'un
/// test qui déréférence quoi que ce soit.
#[test]
fn memory_operands_are_addressable_in_every_usual_form() {
    let emu = run_program(
        "\
section .data
table dq 10, 20, 30, 40

section .bss
case resq 1

section .text
global _start
_start:
mov rax, [table]            ; adresse absolue
mov rsi, table
mov rbx, [rsi + 8]          ; base + décalage
mov rcx, 2
mov rdx, [rsi + rcx*8]      ; base + index mis à l'échelle
mov r8, 99
mov [case], r8              ; écriture absolue
mov r9, [case]              ; relecture
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 10, "[table]");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 20, "[rsi + 8]");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 30, "[rsi + rcx*8]");
    assert_eq!(emu.cpu.regs.get64(Reg64::R9), 99, "ce qu'on vient d'écrire");
}

/// Le pas arrière défait ce que le dernier pas a fait : registres,
/// flags, et la mémoire écrite au passage.
#[test]
fn stepping_back_undoes_registers_flags_and_memory() {
    let mut emu = load_only(
        "\
section .bss
case resq 1

section .text
global _start
_start:
mov rax, 7
mov [case], rax
cmp rax, 7          ; ZF = 1
hlt
",
        &[],
    );
    for _ in 0..3 {
        emu.step().unwrap();
    }
    let case = emu.cpu.regs.rip;
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 7);
    assert!(emu.cpu.flags.contains(RFlags::ZF), "le cmp a levé ZF");

    // On défait le `cmp` : ZF retombe, RIP recule.
    assert!(emu.step_back());
    assert!(
        !emu.cpu.flags.contains(RFlags::ZF),
        "ZF est revenu comme avant"
    );
    assert!(emu.cpu.regs.rip < case);

    // Puis l'écriture en mémoire, et enfin le `mov rax, 7` : RAX
    // redevient nul.
    assert!(emu.step_back());
    assert!(emu.step_back());
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0, "RAX est revenu à zéro");

    // Plus rien à défaire au départ du programme.
    assert!(!emu.can_step_back());
    assert!(!emu.step_back());
}

/// Ce qu'un `mov [adresse], valeur` a écrit est bien remis en place,
/// et pas seulement les registres.
#[test]
fn stepping_back_restores_the_bytes_that_were_overwritten() {
    let mut emu = load_only(
        "\
section .data
case dq 0x1122334455667788

section .text
global _start
_start:
mov rax, 0
mov [case], rax
hlt
",
        &[],
    );
    let addr = {
        // La case est la seule donnée : elle est au début de .data.
        let out = namsdec_asm::assemble(
            "section .data\n    case dq 0x1122334455667788\nsection .text\n    global _start\n_start:\n    hlt\n",
            BASE,
        )
        .unwrap();
        out.sections
            .iter()
            .find(|s| s.name == ".data")
            .expect("une section .data")
            .addr
    };

    emu.step().unwrap(); // mov rax, 0
    emu.step().unwrap(); // mov [case], rax
    assert_eq!(
        emu.cpu.mem.read_u64(addr).unwrap(),
        0,
        "la case est écrasée"
    );

    assert!(emu.step_back());
    assert_eq!(
        emu.cpu.mem.read_u64(addr).unwrap(),
        0x1122334455667788,
        "les octets d'origine sont revenus"
    );
}

/// Défaire un `write` reprend ce que le programme avait affiché, et
/// défaire un `read` lui rend ce qu'il avait consommé — sans quoi le
/// rejeu se bloquerait sur une entrée vide.
#[test]
fn stepping_back_rewinds_the_console_too() {
    let mut emu = load_only(
        "\
section .bss
tampon resb 8

section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, tampon
mov rdx, 3
syscall             ; read
mov rax, 1
mov rdi, 1
mov rsi, tampon
mov rdx, 3
syscall             ; write
hlt
",
        &[],
    );
    emu.provide_input(b"oui");
    emu.run(100).unwrap();
    assert_eq!(emu.output(), b"oui");
    assert_eq!(emu.pending_input(), 0);

    // Défaire le `write` : la sortie se vide.
    while emu.output() == b"oui" {
        assert!(emu.step_back());
    }
    assert!(emu.output().is_empty(), "la sortie est reprise");

    // Continuer jusqu'à défaire le `read` : l'entrée revient.
    while emu.pending_input() == 0 && emu.can_step_back() {
        emu.step_back();
    }
    assert_eq!(emu.pending_input(), 3, "les octets lus sont rendus");
}

/// Le lot d'instructions qui manquait pour aller au-delà des
/// premiers exercices : multiplication, division, décalages,
/// extensions de taille, `setcc`. Chacune est vérifiée sur son
/// résultat *et* sur les flags qu'un cours fait lire juste après.
#[test]
fn multiplication_and_division_land_in_the_right_registers() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 7
mov rbx, 6
mul rbx             ; RDX:RAX = 42
mov rcx, rax

mov rax, 100
xor rdx, rdx        ; la moitié haute du dividende doit être nette
mov rbx, 7
div rbx             ; RAX = 14, RDX = 2
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 42, "7 * 6");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 14, "100 / 7");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 2, "et son reste");
}

/// `imul` à deux opérandes garde le produit dans la destination, et
/// signale par CF/OF qu'il a dû le tronquer.
#[test]
fn imul_reports_a_truncated_product() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov eax, -3
imul eax, 5         ; -15, tient dans 32 bits
mov ebx, eax

mov eax, 0x40000000
imul eax, 4         ; 2^32 : ne tient pas
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get32(Reg64::Rbx) as i32, -15);
    assert!(
        emu.cpu.flags.contains(RFlags::CF) && emu.cpu.flags.contains(RFlags::OF),
        "le produit tronqué doit lever CF et OF"
    );
}

/// Une division par zéro est l'exception du processeur, pas un
/// résultat inventé — et l'adresse fautive vient avec.
#[test]
fn dividing_by_zero_is_reported_as_the_processor_would() {
    let mut emu = load_only(
        "\
section .text
global _start
_start:
mov rax, 5
xor rdx, rdx
xor rbx, rbx
div rbx
hlt
",
        &[],
    );
    let err = emu.run(50).expect_err("division par zéro");
    assert!(
        matches!(err, EmuError::DivideByZero { .. }),
        "obtenu {err:?}"
    );
    // Et le quotient qui déborde est une faute distincte.
    let mut emu = load_only(
        "\
section .text
global _start
_start:
mov rdx, 1          ; dividende = 2^64, quotient trop grand
xor rax, rax
mov rbx, 1
div rbx
hlt
",
        &[],
    );
    let err = emu.run(50).expect_err("quotient hors bornes");
    assert!(
        matches!(err, EmuError::DivideOverflow { .. }),
        "obtenu {err:?}"
    );
}

/// Décalages : le résultat, et le dernier bit sorti dans CF.
#[test]
fn shifts_move_the_bits_and_the_carry() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 5
shl rax, 3          ; 40
mov rbx, rax

mov rcx, 0x81
shr rcx, 1          ; 0x40, CF = 1 (le bit sorti)
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 40, "5 << 3");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0x40);
    assert!(emu.cpu.flags.contains(RFlags::CF), "le bit 0 sort dans CF");
}

/// `sar` recopie le bit de signe : c'est ce qui en fait une division
/// signée par une puissance de deux, là où `shr` ferait entrer des
/// zéros et changerait le signe.
#[test]
fn sar_keeps_the_sign_where_shr_loses_it() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, -16
sar rax, 2          ; -4
mov rbx, rax

mov rcx, -16
shr rcx, 2          ; un très grand nombre positif
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rbx) as i64, -4);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), (-16i64 as u64) >> 2);
}

/// `movzx` complète par des zéros, `movsx` par le bit de signe : la
/// même valeur d'octet donne deux nombres différents, et c'est
/// exactement la leçon.
#[test]
fn movzx_and_movsx_disagree_on_purpose() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov al, 0xf0
movzx rbx, al       ; 240
mov al, 0xf0
movsx rcx, al       ; -16
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 240);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx) as i64, -16);
}

/// `setcc` met un booléen dans un octet, sans branchement — et
/// `neg`/`not`, que le complément à deux réclame dès le premier cours.
#[test]
fn setcc_neg_and_not_do_their_arithmetic() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 3
cmp rax, 3
sete bl             ; 1
cmp rax, 4
setg cl             ; 0

mov rdx, 5
neg rdx             ; -5
mov rsi, 0
not rsi             ; tous les bits à 1
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::Rbx), 1, "3 == 3");
    assert_eq!(emu.cpu.regs.get8l(Reg64::Rcx), 0, "3 > 4 est faux");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdx) as i64, -5);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), u64::MAX);
}

/// `loop`, `xchg` et `leave` : la boucle idiomatique, l'échange, et
/// le démontage du cadre de pile qu'un prologue a monté.
#[test]
fn loop_xchg_and_leave_behave_as_taught() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rcx, 4
xor rax, rax
somme:
add rax, rcx
loop somme          ; 4+3+2+1 = 10

mov rbx, 1
mov rdx, 2
xchg rbx, rdx

push rbp
mov rbp, rsp
sub rsp, 32
leave               ; RSP et RBP reviennent où ils étaient
hlt
",
        &[],
    );
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rax),
        10,
        "la boucle a bien tourné"
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 2, "échangés");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 1);
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rsp),
        BASE + 0x8000,
        "`leave` défait le prologue en entier, `push rbp` compris"
    );
}

/// `adc` additionne la retenue : c'est ainsi qu'on additionne plus
/// large qu'un registre.
#[test]
fn adc_carries_across_registers() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, -1         ; 0xffff_ffff_ffff_ffff
mov rbx, 1
add rax, rbx        ; déborde : CF = 1
mov rcx, 0
adc rcx, 0          ; la retenue remonte
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 1, "la retenue est passée");
}

/// Un `read` dont le tampon tombe hors de la fenêtre mémoire échoue —
/// et l'entrée déjà fournie doit rester dans la file. `step` rembobine
/// RIP pour rejouer le `syscall` à l'identique ; si les octets avaient
/// été consommés au passage, l'utilisateur devait retaper ce qu'il
/// venait d'envoyer, et le rejeu se bloquait sur une file vide.
#[test]
fn a_read_that_faults_keeps_the_input_it_could_not_write() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    let mut emu = load_only(
        "\
section .text
global _start
_start:
mov rax, 0
mov rdi, 0
mov rsi, 0xdeadbeef     ; tampon hors de la fenêtre simulée
mov rdx, 4
syscall
hlt
",
        &[],
    );
    emu.provide_input(b"oui\n");

    let err = emu.run(100).expect_err("le tampon est hors bornes");
    assert!(matches!(err, EmuError::Mem(_)), "obtenu {err:?}");
    assert_eq!(
        emu.pending_input(),
        4,
        "l'entrée doit être intacte pour le rejeu"
    );
    // Et le `syscall` est toujours sous RIP, prêt à être rejoué.
    assert_eq!(
        emu.cpu.mem.bytes_at(emu.cpu.regs.rip, 2).unwrap(),
        &[0x0f, 0x05],
        "RIP doit être resté sur le syscall"
    );
}

/// Un `pop` dont l'écriture échoue laisse RSP là où il était. Il
/// avançait avant l'écriture : chaque rejeu — que `step` provoque en
/// rembobinant RIP — dépilait un mot de plus, et la pile se dévidait
/// silencieusement, huit octets par tentative.
#[test]
fn a_failed_pop_leaves_rsp_where_it_was() {
    let mut mem = namsdec_core::Memory::new(0x400000, 0x1000);
    // `pop qword [rax]` (8F 00), avec RAX hors de la fenêtre.
    mem.load(0x400000, &[0x8f, 0x00]).unwrap();
    let mut cpu = namsdec_core::Cpu::new(mem);
    cpu.regs.rip = 0x400000;
    cpu.regs.set64(Reg64::Rsp, 0x400800);
    cpu.regs.set64(Reg64::Rax, 0xdeadbeef);
    let mut emu = Emulator::new(cpu);

    for attempt in 1..=3 {
        emu.step().expect_err("la destination est hors bornes");
        assert_eq!(
            emu.cpu.regs.get64(Reg64::Rsp),
            0x400800,
            "RSP a bougé à la tentative {attempt}"
        );
        assert_eq!(emu.cpu.regs.rip, 0x400000, "et RIP aussi");
    }
}

fn load_only(asm: &str, breakpoints: &[u64]) -> Emulator {
    let out = namsdec_asm::assemble(asm, BASE).expect("assemblage (nasm et ld requis)");
    let mut mem = Memory::new(BASE, 0x10000);
    // L'ELF dit où il veut être chargé et par où commencer : sur un
    // programme à plusieurs sections, ce ne sont pas la même adresse.
    mem.load(out.load_addr, &out.bytes).unwrap();

    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = out.entry;
    cpu.regs.set64(Reg64::Rsp, BASE + 0x8000);

    let mut emu = Emulator::new(cpu);
    emu.set_heap(out.load_addr + out.bytes.len() as u64, BASE + 0x7000);
    emu.breakpoints.extend(breakpoints.iter().copied());
    emu
}

/// Comme `run_program`, mais sans exécuter quoi que ce soit — pour les
/// tests qui doivent piloter `step`/`step_over` à la main. Renvoie
/// aussi la sortie d'assemblage pour retrouver les adresses par ligne.
fn load_program(asm: &str) -> (Emulator, namsdec_asm::AssembleOutput) {
    let out = namsdec_asm::assemble(asm, BASE).expect("assemblage (nasm et ld requis)");
    let mut mem = Memory::new(BASE, 0x10000);
    mem.load(out.load_addr, &out.bytes).unwrap();

    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = out.entry;
    cpu.regs.set64(Reg64::Rsp, BASE + 0x8000);

    let mut emu = Emulator::new(cpu);
    // Le tas entre l'image du programme et la pile, comme le fait
    // l'interface quand elle charge un programme.
    emu.set_heap(out.load_addr + out.bytes.len() as u64, BASE + 0x7000);
    (emu, out)
}

const SUM_LOOP_ASM: &str = "\
BITS 64
section .text
mov rax, 0
mov rcx, 5
sum_loop:
add rax, rcx
dec rcx
jnz sum_loop
hlt
";

#[test]
fn sum_loop_computes_15_and_halts() {
    let emu = run_program(SUM_LOOP_ASM, &[]);
    assert!(emu.cpu.halted);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 15);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 0);
    assert!(emu.cpu.flags.contains(RFlags::ZF));
}

#[test]
fn breakpoint_stops_before_halt() {
    // sum_loop: est à 0x40000A (voir namsdec-asm::tests::assembles_sum_loop_*)
    let emu = run_program(SUM_LOOP_ASM, &[0x40000A]);
    assert!(!emu.cpu.halted);
    assert_eq!(emu.cpu.regs.rip, 0x40000A);
}

#[test]
fn push_pop_round_trip() {
    let asm = "\
BITS 64
section .text
mov rax, 0x1234
push rax
mov rax, 0
pop rbx
hlt
";
    let emu = run_program(asm, &[]);
    assert!(emu.cpu.halted);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 0x1234);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0);
}

#[test]
fn lea_computes_address_without_touching_memory() {
    let asm = "\
BITS 64
section .text
mov rbx, 0x400000
lea rax, [rbx + 0x10]
hlt
";
    let emu = run_program(asm, &[]);
    assert!(emu.cpu.halted);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0x400010);
}

#[test]
fn inc_dec_preserve_carry_flag() {
    let asm = "\
BITS 64
section .text
mov rax, 0xFFFFFFFFFFFFFFFF
add rax, 2       ; positionne CF=1
inc rbx          ; ne doit pas modifier CF
hlt
";
    let emu = run_program(asm, &[]);
    assert!(emu.cpu.halted);
    assert!(emu.cpu.flags.contains(RFlags::CF));
    assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 1);
}

const CALL_RET_ASM: &str = "\
BITS 64
section .text
mov rax, 1
call add_five
hlt
add_five:
add rax, 5
ret
";

#[test]
fn call_then_ret_round_trips_and_updates_rax() {
    let emu = run_program(CALL_RET_ASM, &[]);
    assert!(emu.cpu.halted);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
}

#[test]
fn step_into_a_call_enters_the_subroutine() {
    let (mut emu, out) = load_program(CALL_RET_ASM);
    let call_addr = out.line_to_addr[&4];
    while emu.cpu.regs.rip != call_addr {
        emu.step().unwrap();
    }
    let subroutine_addr = out.line_to_addr[&7]; // "add rax, 5"
    emu.step().unwrap();
    assert_eq!(emu.cpu.regs.rip, subroutine_addr);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 1); // pas encore exécuté
}

#[test]
fn step_over_a_call_runs_the_whole_subroutine() {
    let (mut emu, out) = load_program(CALL_RET_ASM);
    let call_addr = out.line_to_addr[&4];
    while emu.cpu.regs.rip != call_addr {
        emu.step().unwrap();
    }
    let after_call_addr = out.line_to_addr[&5]; // hlt
    let result = emu.step_over(1000).unwrap();
    assert_eq!(result, StepOverResult::SteppedOverCall);
    assert_eq!(emu.cpu.regs.rip, after_call_addr);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 6);
}

#[test]
fn step_over_a_non_call_instruction_behaves_like_step() {
    let (mut emu, _out) = load_program(SUM_LOOP_ASM);
    let result = emu.step_over(1000).unwrap();
    assert_eq!(result, StepOverResult::Stepped);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0); // "mov rax, 0" tout juste exécuté
}
/// Un tas neuf : le programme demande où il commence, le fait
/// grandir, et écrit dans ce qu'il vient d'obtenir. C'est le cœur de
/// ce qu'un `malloc` fait pour vous.
#[test]
fn brk_grows_the_heap_and_the_new_memory_is_usable() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 12         ; brk(0) : ne demande rien, lit la rupture
xor rdi, rdi
syscall
mov r12, rax        ; le début du tas

mov rdi, rax
add rdi, 4096
mov rax, 12         ; brk(fin + 4096) : 4 Kio de plus
syscall
mov r13, rax

mov byte [r12], 42  ; la mémoire obtenue s'écrit
movzx r14, byte [r12]
hlt
",
        &[],
    );

    let start = emu.cpu.regs.get64(Reg64::R12);
    assert_eq!(start % PAGE, 0, "la rupture part sur une page");
    assert_eq!(start, emu.heap().start());
    assert_eq!(
        emu.cpu.regs.get64(Reg64::R13),
        start + 4096,
        "brk rend la rupture obtenue, pas un code d'erreur"
    );
    assert_eq!(emu.heap().len(), 4096);
    assert_eq!(emu.cpu.regs.get64(Reg64::R14), 42, "le tas est utilisable");
}

/// Le tas ne mord ni sur le programme, ni sur la pile : hors de ces
/// bornes, `brk` refuse — et refuser, pour lui, c'est rendre la
/// rupture inchangée.
#[test]
fn a_brk_outside_the_heap_is_refused_and_leaves_the_break_alone() {
    let mut emu = Emulator::new(Cpu::new(Memory::new(BASE, 0x10000)));
    emu.set_heap(BASE + 0x1000, BASE + 0x2000);

    assert_eq!(emu.set_break(0), BASE + 0x1000, "brk(0) ne déplace rien");
    assert_eq!(
        emu.set_break(BASE + 0x2001),
        BASE + 0x1000,
        "au-dessus du plafond : refusé"
    );
    assert_eq!(
        emu.set_break(BASE + 0x0800),
        BASE + 0x1000,
        "sous le programme : refusé"
    );
    assert_eq!(
        emu.set_break(BASE + 0x2000),
        BASE + 0x2000,
        "le plafond lui-même s'atteint"
    );
}

/// Rendre de la mémoire, c'est la perdre : le noyau reprend les pages
/// et n'en rendra que des vierges. Un programme qui rétrécit son tas
/// puis le rouvre ne doit pas y retrouver ses données.
#[test]
fn giving_the_heap_back_forgets_what_was_in_it() {
    let mut emu = Emulator::new(Cpu::new(Memory::new(BASE, 0x10000)));
    emu.set_heap(BASE + 0x1000, BASE + 0x3000);
    emu.set_break(BASE + 0x2000);
    emu.cpu.mem.write_u64(BASE + 0x1000, 0x1122_3344).unwrap();

    emu.set_break(BASE + 0x1000); // rendu
    emu.set_break(BASE + 0x2000); // repris
    assert_eq!(emu.cpu.mem.read_u64(BASE + 0x1000).unwrap(), 0);
}

/// Sans tas installé, `brk` se comporte comme sous un système à court
/// de mémoire : il rend la rupture, il ne la déplace pas.
#[test]
fn without_a_heap_brk_hands_out_nothing() {
    let mut emu = Emulator::new(Cpu::new(Memory::new(BASE, 0x10000)));
    let top = BASE + 0x10000;
    assert_eq!(emu.set_break(0), top);
    assert_eq!(emu.set_break(top + 0x1000), top);
    assert_eq!(emu.heap().len(), 0);
}

/// Revenir en arrière sur un `brk` rend la mémoire au système.
#[test]
fn stepping_back_undoes_a_brk() {
    let mut emu = load_only(
        "\
section .text
global _start
_start:
mov rax, 12
xor rdi, rdi
syscall
mov rdi, rax
add rdi, 4096
mov rax, 12
syscall
hlt
",
        &[],
    );

    let start = emu.heap().current();
    for _ in 0..20 {
        if emu.heap().current() != start {
            break;
        }
        emu.step().unwrap();
    }
    assert_eq!(emu.heap().current(), start + 4096, "le tas a grandi");

    assert!(emu.step_back());
    assert_eq!(
        emu.heap().current(),
        start,
        "défaire le pas rend aussi la mémoire"
    );
}
/// Diviser un nombre négatif demande d'abord d'étendre son signe dans
/// RDX : sans `cqo`, `idiv` divise par un reste oublié.
#[test]
fn sign_extension_prepares_a_signed_division() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, -17
cqo                 ; RDX:RAX = -17 sur 128 bits
mov rbx, 5
idiv rbx            ; RAX = -3, RDX = -2

mov r13, rax        ; le quotient, mis à l'abri
mov r14, rdx        ; et le reste

mov r8b, -3
movsx r9, r8b       ; comparaison : la même idée, sur un registre
mov al, -5
cbw                 ; AL -> AX
mov r10w, ax
mov eax, -6
cdqe                ; EAX -> RAX
mov r11, rax
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::R13) as i64, -3, "quotient");
    assert_eq!(emu.cpu.regs.get64(Reg64::R14) as i64, -2, "reste");
    assert_eq!(emu.cpu.regs.get64(Reg64::R9) as i64, -3);
    assert_eq!(emu.cpu.regs.get16(Reg64::R10) as i16, -5, "cbw");
    assert_eq!(emu.cpu.regs.get64(Reg64::R11) as i64, -6, "cdqe");
}

/// Les rotations font ressortir par le bas ce qui sort par le haut —
/// et `rcl`/`rcr` font passer le bit par la retenue.
#[test]
fn rotations_bring_back_what_a_shift_would_lose() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov al, 10000001b
rol al, 1           ; 00000011b, CF = 1
mov bl, al

mov cl, 00000011b
ror cl, 1           ; 10000001b, CF = 1

stc
mov dl, 00000001b
rcl dl, 1           ; la retenue entre par le bas : 00000011b

mov sil, 11111111b
rol sil, 8          ; un tour complet : rien ne change
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::Rbx), 0b0000_0011, "rol");
    assert_eq!(emu.cpu.regs.get8l(Reg64::Rcx), 0b1000_0001, "ror");
    assert_eq!(emu.cpu.regs.get8l(Reg64::Rdx), 0b0000_0011, "rcl");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rsi) & 0xff,
        0xff,
        "un tour entier"
    );
    assert!(
        emu.cpu.flags.contains(RFlags::CF),
        "le bit sorti reste dans la retenue"
    );
}

/// Le bit désigné : `bt` le lit, `bts` le pose, et sur une zone
/// mémoire le décalage n'est pas borné par l'opérande — c'est ce qui
/// permet de traiter un tableau de bits.
#[test]
fn the_bit_instructions_find_set_and_count_bits() {
    let emu = run_program(
        "\
section .bss
drapeaux resb 32

section .text
global _start
_start:
mov rax, 1000b
bt rax, 3           ; CF = 1
setc r8b
bts rax, 4          ; rax = 11000b
btr rax, 3          ; rax = 10000b
btc rax, 0          ; rax = 10001b
mov r9, rax

mov rcx, 100        ; le centième bit : treizième octet, bit 4
bts [drapeaux], rcx
movzx r10, byte [drapeaux + 12]

mov rax, 0x100
bsf rbx, rax        ; 8
bsr rdx, rax        ; 8
mov rax, 0xff00
popcnt rsi, rax     ; 8
tzcnt rdi, rax      ; 8
lzcnt r11, rax      ; 48
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1, "bt a mis CF");
    assert_eq!(emu.cpu.regs.get64(Reg64::R9), 0b1_0001);
    assert_eq!(emu.cpu.regs.get64(Reg64::R10), 1 << 4, "le centième bit");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rbx), 8, "bsf");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 8, "bsr");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), 8, "popcnt");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdi), 8, "tzcnt");
    assert_eq!(emu.cpu.regs.get64(Reg64::R11), 48, "lzcnt");
}

/// `xadd` et `cmpxchg` sont les deux instructions sur lesquelles
/// reposent les compteurs et les verrous : elles lisent et écrivent
/// dans le même souffle.
#[test]
fn exchange_instructions_swap_and_compare() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 10
mov rbx, 3
xadd rax, rbx       ; rax = 13, rbx = 10
mov r11, rax        ; la somme, avant que cmpxchg ne reprenne RAX

mov rcx, 5
mov rax, 5
mov rdx, 42
cmpxchg rcx, rdx    ; rax == rcx : rcx prend 42, ZF = 1
setz r8b

mov rax, 0          ; cette fois la comparaison échoue
mov rsi, 7
cmpxchg rsi, rdx    ; rax reçoit 7, rsi ne bouge pas
setz r9b

mov r10d, 0x11223344
bswap r10d          ; 0x44332211
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::R11), 13, "xadd : la somme");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rbx),
        10,
        "xadd : l'ancienne valeur"
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 42, "l'échange a eu lieu");
    assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), 7, "l'échange a échoué");
    assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 0);
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rax),
        7,
        "un cmpxchg qui échoue rend la valeur trouvée"
    );
    assert_eq!(emu.cpu.regs.get32(Reg64::R10), 0x4433_2211, "bswap");
}

/// Les indicateurs se posent aussi à la main, et se rangent sur la
/// pile — ce qu'il faut pour écrire une routine qui ne perturbe pas
/// l'état de celle qui l'appelle.
#[test]
fn flags_can_be_set_saved_and_restored() {
    let emu = run_program(
        "\
section .text
global _start
_start:
stc
pushfq              ; la retenue part sur la pile
clc
setc r8b            ; 0
popfq
setc r9b            ; 1 : elle est revenue

cmc                 ; et elle se retourne
setc r10b           ; 0

stc
lahf                ; les indicateurs dans AH
mov dl, ah          ; (AH ne se lit pas dans une instruction à REX)
movzx r11, dl
clc
sahf                ; et retour
setc r12b           ; 1
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 0);
    assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 1, "popfq a rendu la retenue");
    assert_eq!(emu.cpu.regs.get8l(Reg64::R10), 0, "cmc l'a retournée");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::R11) & 0x03,
        0x03,
        "lahf : CF posée, et le bit 1 toujours à 1"
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::R12), 1, "sahf l'a rendue");
}

/// Boucles conditionnelles, saut sur RCX nul, cadre de pile par
/// `enter`, et la table de `xlatb`.
#[test]
fn loops_frames_and_tables() {
    let emu = run_program(
        "\
section .data
table db 'a', 'b', 'c', 'd'

section .text
global _start
_start:
mov rcx, 10
xor rax, rax
compte:
inc rax
cmp rax, 4
loopne compte       ; s'arrête sur l'égalité, RCX non nul
mov r13, rax        ; mis à l'abri : xlatb reprendra RAX
mov r14, rcx        ; de même pour RCX, remis à zéro plus bas

xor rcx, rcx
jrcxz vide          ; RCX est nul : on saute
mov rbx, 1          ; sautée
vide:
mov rbx, 2

mov rbp, rsp
enter 32, 0         ; cadre de 32 octets
mov r8, rsp
mov r9, rbp
leave

mov rbx, table
mov al, 2
xlatb               ; AL = table[2] = 'c'
mov r10b, al
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::R13), 4, "loopne s'arrête sur ZF");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::R14),
        6,
        "et RCX a compté les tours"
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::R10), b'c', "xlatb a lu la table");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::R9) - emu.cpu.regs.get64(Reg64::R8),
        32,
        "enter a réservé 32 octets sous RBP"
    );
}

/// `shld`/`shrd` décalent une valeur plus large qu'un registre : les
/// bits qui entrent viennent de l'autre moitié, et non de zéros.
#[test]
fn double_shifts_carry_bits_across_registers() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 0xff00000000000000
mov rbx, 0x8000000000000000
shld rax, rbx, 4    ; rax = 0xf000...008
mov r8, rax

mov rcx, 0x0f
mov rdx, 0xf0
shrd rcx, rdx, 4    ; rcx = 0x0000...00 | (0xf0 << 60)
mov r9, rcx
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::R8), 0xf000_0000_0000_0008, "shld");
    assert_eq!(emu.cpu.regs.get64(Reg64::R9), 0x0000_0000_0000_0000, "shrd");
}

/// `rdtsc` compte les instructions exécutées : un temps qui a un sens
/// ici, et qui revient en arrière avec le pas arrière.
#[test]
fn rdtsc_counts_the_instructions_and_steps_back() {
    let mut emu = load_only(
        "\
section .text
global _start
_start:
rdtsc
mov r8, rax
rdtsc
mov r9, rax
hlt
",
        &[],
    );
    emu.run(100).expect("exécution");
    let first = emu.cpu.regs.get64(Reg64::R8);
    let second = emu.cpu.regs.get64(Reg64::R9);
    assert_eq!(first, 1, "le premier rdtsc est la première instruction");
    assert_eq!(second, 3, "deux instructions plus loin");
    while emu.can_step_back() {
        emu.step_back();
    }
    assert_eq!(emu.ticks, 0, "revenir au départ remet le compteur à zéro");
}
/// AH, BH, CH, DH : l'octet haut des quatre registres historiques.
/// Un vestige du 8086, mais tous les cours d'assembleur passent par
/// lui — et il ne se confond pas avec l'octet bas.
#[test]
fn the_high_byte_registers_are_not_the_low_ones() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 0x1122
mov ah, 0xff        ; seul l'octet haut change
mov bl, al          ; l'octet bas est intact
mov cl, ah
mov dx, ax
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0xff22, "ah seul a changé");
    assert_eq!(emu.cpu.regs.get8l(Reg64::Rbx), 0x22, "al n'a pas bougé");
    assert_eq!(emu.cpu.regs.get8l(Reg64::Rcx), 0xff, "ah se lit");
    assert_eq!(emu.cpu.regs.get16(Reg64::Rdx), 0xff22);
}

/// Ce qui ne simule rien mais doit tout de même passer : les barrières
/// mémoire, `pause`, et le `endbr64` que tout compilateur pose en tête
/// de fonction depuis que les processeurs vérifient les sauts.
#[test]
fn the_instructions_that_do_nothing_still_run() {
    let emu = run_program(
        "\
section .data
mot dd 0x11223344

section .text
global _start
_start:
endbr64
pause
mfence
lfence
sfence
movbe eax, [mot]    ; 0x44332211 : les octets à l'envers
mov r8d, eax
xchg r8, r9
hlt
",
        &[],
    );
    assert_eq!(
        emu.cpu.regs.get64(Reg64::R9),
        0x4433_2211,
        "movbe, puis xchg"
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::R8), 0);
}

/// `int3` arrête le programme là où il est — c'est le point d'arrêt
/// qu'un programme se pose à lui-même — et `ud2` échoue exprès.
#[test]
fn int3_stops_and_ud2_faults() {
    let mut emu = load_only(
        "\
section .text
global _start
_start:
mov rax, 1
int3
mov rax, 2
ud2
",
        &[],
    );
    assert_eq!(emu.run(100).unwrap(), RunStop::Halted);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 1, "arrêté sur l'int3");

    // On repart : l'arrêt n'est pas la fin du programme.
    emu.cpu.halted = false;
    let err = emu.run(100).expect_err("ud2 doit échouer");
    assert!(
        matches!(err, EmuError::InvalidInstruction { .. }),
        "obtenu {err:?}"
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 2);
}
/// Les instructions de bits récentes : elles font en une fois ce qui
/// demandait un masque et deux décalages, sans toucher aux
/// indicateurs qu'un calcul en cours pourrait attendre.
#[test]
fn the_modern_bit_instructions_shift_extract_and_multiply() {
    let emu = run_program(
        "\
section .text
global _start
_start:
mov rax, 0xff
mov rcx, 4
shlx rbx, rax, rcx      ; 0xff0
shrx rdx, rax, rcx      ; 0x0f
mov rsi, -16
sarx rdi, rsi, rcx      ; -1 : le signe est recopié
rorx r8, rax, 4         ; 0xf00000000000000f

mov r9, 0b1011000
blsi r10, r9            ; le bit bas posé : 0b1000
blsr r11, r9            ; le même, effacé : 0b1010000
andn r12, r9, rax       ; ~r9 & 0xff = 0b10100111

mov rax, 0x1234
mov rcx, 0x0804         ; huit bits (0x08) à partir du quatrième (0x04)
bextr r13, rax, rcx     ; 0x23
mov rcx, 8
bzhi r14, rax, rcx      ; on ne garde que les huit bas : 0x34

mov rdx, 0x1_0000_0000
mov r15, 4
mulx rbx, rsi, r15      ; rdx * r15 = 0x4_0000_0000
hlt
",
        &[],
    );
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rbx) as i64,
        0,
        "mulx : la moitié haute"
    );
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rsi),
        0x4_0000_0000,
        "mulx : la basse"
    );
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rdi) as i64,
        -1,
        "sarx recopie le signe"
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::R8), 0xf000_0000_0000_000f, "rorx");
    assert_eq!(emu.cpu.regs.get64(Reg64::R10), 0b1000, "blsi");
    assert_eq!(emu.cpu.regs.get64(Reg64::R11), 0b101_0000, "blsr");
    assert_eq!(emu.cpu.regs.get64(Reg64::R12), 0b1010_0111, "andn");
    assert_eq!(emu.cpu.regs.get64(Reg64::R13), 0x23, "bextr");
    assert_eq!(emu.cpu.regs.get64(Reg64::R14), 0x34, "bzhi");
}

/// `cpuid` ne peut annoncer aucune extension sans mentir : il se nomme
/// et s'arrête là.
#[test]
fn cpuid_says_who_it_is_and_promises_nothing() {
    let emu = run_program(
        "\
section .text
global _start
_start:
xor eax, eax
cpuid
mov r8, rbx
mov r9, rcx
mov r10, rdx
mov r11, rax

mov eax, 1
cpuid                   ; aucune extension annoncée
mov r12, rdx
hlt
",
        &[],
    );
    let mut vendor = Vec::new();
    for part in [Reg64::R8, Reg64::R10, Reg64::R9] {
        vendor.extend_from_slice(&(emu.cpu.regs.get32(part)).to_le_bytes());
    }
    assert_eq!(&vendor, b"namsdec emu\0", "obtenu {vendor:?}");
    assert_eq!(emu.cpu.regs.get64(Reg64::R11), 0, "aucune feuille au-delà");
    assert_eq!(emu.cpu.regs.get64(Reg64::R12), 0, "aucun bit de capacité");
}
/// Le calcul flottant du x86-64 : il se fait dans les registres XMM,
/// un nombre à la fois, et les résultats se comparent avec des tests
/// *non signés* parce que `comisd` écrit dans CF et ZF.
#[test]
fn scalar_floating_point_computes_and_compares() {
    let emu = run_program(
        "\
section .data
deux    dq 2.0
trois   dq 3.0
demi    dd 0.5

section .text
global _start
_start:
movsd xmm0, [deux]
movsd xmm1, [trois]
addsd xmm0, xmm1        ; 5.0
movq rax, xmm0

movsd xmm2, [trois]
mulsd xmm2, xmm2        ; 9.0
sqrtsd xmm2, xmm2       ; 3.0
movq rbx, xmm2

movss xmm3, [demi]
addss xmm3, xmm3        ; 1.0 en simple précision
movd ecx, xmm3

movsd xmm4, [deux]
comisd xmm4, [trois]    ; 2 < 3 : CF = 1
setb r8b
comisd xmm4, [deux]     ; égal : ZF = 1
sete r9b

mov r11, 5
cvtsi2sd xmm5, r11      ; 5 -> 5.0
cvttsd2si rdx, xmm5     ; et retour
pxor xmm6, xmm6         ; la façon habituelle de remettre à zéro
movq rsi, xmm6
hlt
",
        &[],
    );
    assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rax)), 5.0, "addsd");
    assert_eq!(
        f64::from_bits(emu.cpu.regs.get64(Reg64::Rbx)),
        3.0,
        "sqrtsd"
    );
    assert_eq!(
        f32::from_bits(emu.cpu.regs.get32(Reg64::Rcx)),
        1.0,
        "addss en simple précision"
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1, "comisd : plus petit");
    assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 1, "comisd : égal");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 5, "cvttsd2si");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rsi), 0, "pxor remet à zéro");
}

/// Un NaN n'est ni plus petit, ni égal, ni plus grand : `comisd` le
/// dit en posant les trois indicateurs d'un coup — « non ordonné ».
#[test]
fn a_nan_comparison_is_unordered() {
    let emu = run_program(
        "\
section .data
pasnombre dq 0x7ff8000000000000
un        dq 1.0

section .text
global _start
_start:
movsd xmm0, [pasnombre]
movsd xmm1, [un]
comisd xmm0, xmm1
setp r8b                ; PF : non ordonné
setz r9b
setc r10b
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1, "PF signale le NaN");
    assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 1);
    assert_eq!(emu.cpu.regs.get8l(Reg64::R10), 1);
}

/// `addps` fait quatre additions d'un seul ordre — c'est toute la
/// raison d'être des registres larges.
#[test]
fn packed_arithmetic_computes_four_at_once() {
    let emu = run_program(
        "\
section .data
align 16
quatre dd 1.0, 2.0, 3.0, 4.0
unites dd 1.0, 1.0, 1.0, 1.0
sortie times 16 db 0

section .text
global _start
_start:
movaps xmm0, [quatre]
movaps xmm1, [unites]
addps xmm0, xmm1        ; 2, 3, 4, 5
movaps [sortie], xmm0

movss xmm2, [sortie]
movd eax, xmm2
movss xmm3, [sortie + 12]
movd ebx, xmm3
hlt
",
        &[],
    );
    assert_eq!(f32::from_bits(emu.cpu.regs.get32(Reg64::Rax)), 2.0);
    assert_eq!(f32::from_bits(emu.cpu.regs.get32(Reg64::Rbx)), 5.0);
}

/// Le haut d'un registre XMM ne bouge pas quand on n'écrit que son
/// bas — sauf en venant de la mémoire, où `movsd` met le reste à
/// zéro. Une asymétrie du jeu d'instructions, et une source d'erreurs
/// classique.
#[test]
fn a_scalar_move_only_touches_what_it_must() {
    let emu = run_program(
        "\
section .data
align 16
motifs dq 0x1111111111111111, 0x2222222222222222
un     dq 1.0
sortie times 16 db 0

section .text
global _start
_start:
movaps xmm0, [motifs]
movsd xmm0, [un]        ; depuis la mémoire : le haut passe à zéro
movaps [sortie], xmm0
mov rax, [sortie]
mov rbx, [sortie + 8]

movaps xmm1, [motifs]
movsd xmm2, [un]
movsd xmm1, xmm2        ; entre registres : le haut est préservé
movaps [sortie], xmm1
mov rcx, [sortie]
mov rdx, [sortie + 8]
hlt
",
        &[],
    );
    assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rax)), 1.0);
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rbx),
        0,
        "la mémoire efface le haut"
    );
    assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rcx)), 1.0);
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rdx),
        0x2222_2222_2222_2222,
        "un registre le laisse en place"
    );
}
/// SSE entier : seize additions d'un ordre, une comparaison qui rend
/// un masque, et `pmovmskb` qui ramène le résultat dans un registre
/// général — la chaîne complète d'un `memchr` vectoriel.
#[test]
fn packed_integers_add_compare_and_report() {
    let emu = run_program(
        "\
section .data
align 16
a       times 16 db 1
b       times 16 db 2
cible   db 'n','a','m','s','d','e','c',0,0,0,0,0,0,0,0,0
lettres times 16 db 'm'
sortie  times 16 db 0

section .text
global _start
_start:
movdqa xmm0, [a]
movdqa xmm1, [b]
paddb xmm0, xmm1        ; seize fois 3
movdqa [sortie], xmm0
movzx rax, byte [sortie]

movdqa xmm2, [cible]
movdqa xmm3, [lettres]
pcmpeqb xmm2, xmm3      ; le 'm' de « nams » : troisième octet
pmovmskb ebx, xmm2

movdqa xmm4, [a]
psllq xmm4, 4           ; chaque moitié décalée de quatre bits
movq rcx, xmm4

movdqa xmm5, [a]
punpcklbw xmm5, xmm5    ; des octets entrelacés avec eux-mêmes
movq rdx, xmm5
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 3, "paddb");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rbx),
        0b100,
        "pcmpeqb puis pmovmskb"
    );
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rcx),
        0x0101_0101_0101_0101u64 << 4,
        "psllq décale les deux moitiés"
    );
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rdx),
        0x0101_0101_0101_0101,
        "punpcklbw"
    );
}

/// `pshufd` remet les quatre éléments dans l'ordre qu'on veut, et
/// `movhlps` ramène le haut d'un registre en bas — deux façons
/// d'atteindre ce que les instructions scalaires ne voient pas.
#[test]
fn shuffles_reach_the_upper_half() {
    let emu = run_program(
        "\
section .data
align 16
quatre dd 10, 20, 30, 40
sortie times 16 db 0

section .text
global _start
_start:
movdqa xmm0, [quatre]
pshufd xmm1, xmm0, 00_01_10_11b   ; ordre inversé : 40, 30, 20, 10
movdqa [sortie], xmm1
mov eax, [sortie]
mov ebx, [sortie + 12]

movhlps xmm2, xmm0      ; le haut de xmm0 (30, 40) passe en bas
movq rcx, xmm2
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get32(Reg64::Rax), 40, "pshufd a inversé");
    assert_eq!(emu.cpu.regs.get32(Reg64::Rbx), 10);
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rcx),
        (40u64 << 32) | 30,
        "movhlps"
    );
}
/// Le x87 : une pile, et non des registres nommés. `fld` empile, les
/// opérations consomment le sommet, et le résultat redescend en
/// mémoire. C'est la machine que décrivent les cours d'assembleur
/// écrits avant SSE.
#[test]
fn the_x87_stack_computes_and_stores() {
    let emu = run_program(
        "\
section .data
deux    dq 2.0
trois   dq 3.0
sept    dd 7
sortie  dq 0.0
entier  dq 0
etendu  dt 0.0

section .text
global _start
_start:
fld qword [deux]        ; st0 = 2
fld qword [trois]       ; st0 = 3, st1 = 2
faddp                   ; st1 += st0, puis dépile : st0 = 5
fstp qword [sortie]     ; la pile est vide
mov rax, [sortie]

fild dword [sept]       ; 7, converti depuis un entier
fld qword [deux]
fdivp                   ; st1 / st0 = 3.5
fistp qword [entier]    ; arrondi au plus proche pair : 4
mov rbx, [entier]

fld qword [trois]
fstp tword [etendu]     ; aller-retour par le format 80 bits
fld tword [etendu]
fstp qword [sortie]
mov rcx, [sortie]
hlt
",
        &[],
    );
    assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rax)), 5.0, "faddp");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rbx),
        4,
        "fistp arrondit 3,5 vers 4"
    );
    assert_eq!(
        f64::from_bits(emu.cpu.regs.get64(Reg64::Rcx)),
        3.0,
        "le format étendu se relit"
    );
    assert_eq!(emu.cpu.regs.x87.depth(), 0, "tout a été dépilé");
}

/// Le x87 ne touche pas aux indicateurs du processeur : il répond
/// dans son mot d'état, qu'il faut aller chercher. D'où le
/// `fstsw ax` / `sahf` du code d'époque — et les `fcomi`, venus plus
/// tard, qui s'en passent.
#[test]
fn an_x87_comparison_answers_in_its_own_status_word() {
    let emu = run_program(
        "\
section .data
deux  dq 2.0
trois dq 3.0

section .text
global _start
_start:
fld qword [deux]
fld qword [trois]       ; st0 = 3, st1 = 2
fcom st1                ; 3 > 2 : ni C0 ni C3
fstsw ax
sahf                    ; C0 -> CF, C3 -> ZF
seta r8b                ; strictement plus grand

fcomip st1              ; la même comparaison, dans les indicateurs
seta r9b

fldz
ftst                    ; zéro contre zéro : C3
fstsw ax
sahf
sete r10b
hlt
",
        &[],
    );
    assert_eq!(emu.cpu.regs.get8l(Reg64::R8), 1, "fstsw puis sahf");
    assert_eq!(emu.cpu.regs.get8l(Reg64::R9), 1, "fcomip directement");
    assert_eq!(emu.cpu.regs.get8l(Reg64::R10), 1, "ftst voit le zéro");
}

/// Les constantes et les fonctions du coprocesseur : c'est avec
/// elles qu'on écrit un calcul trigonométrique sans table.
#[test]
fn the_coprocessor_knows_its_constants() {
    let emu = run_program(
        "\
section .data
sortie dq 0.0

section .text
global _start
_start:
fldpi
fstp qword [sortie]
mov rax, [sortie]

fldpi
fld1
fld1
faddp                   ; 2
fdivp                   ; pi / 2
fsin                    ; sin(pi/2) = 1
fstp qword [sortie]
mov rbx, [sortie]

fld1
fchs                    ; -1
fabs                    ; 1
fstp qword [sortie]
mov rcx, [sortie]
hlt
",
        &[],
    );
    assert_eq!(
        f64::from_bits(emu.cpu.regs.get64(Reg64::Rax)),
        std::f64::consts::PI
    );
    assert!(
        (f64::from_bits(emu.cpu.regs.get64(Reg64::Rbx)) - 1.0).abs() < 1e-12,
        "sin(pi/2)"
    );
    assert_eq!(f64::from_bits(emu.cpu.regs.get64(Reg64::Rcx)), 1.0);
}
/// AVX : les mêmes registres, deux fois plus larges. Huit additions
/// flottantes d'un seul ordre.
#[test]
fn avx_computes_eight_at_a_time() {
    let emu = run_program(
        "\
section .data
align 32
huit   dd 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0
uns    dd 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0
sortie times 32 db 0

section .text
global _start
_start:
vmovaps ymm0, [huit]
vmovaps ymm1, [uns]
vaddps ymm2, ymm0, ymm1     ; 2, 3, 4, 5, 6, 7, 8, 9
vmovaps [sortie], ymm2
mov eax, [sortie]           ; le premier
mov ebx, [sortie + 28]      ; le dernier

; AVX ne détruit pas ses sources : la destination est à part.
vmovaps [sortie], ymm0
mov ecx, [sortie]
hlt
",
        &[],
    );
    assert_eq!(f32::from_bits(emu.cpu.regs.get32(Reg64::Rax)), 2.0);
    assert_eq!(f32::from_bits(emu.cpu.regs.get32(Reg64::Rbx)), 9.0);
    assert_eq!(
        f32::from_bits(emu.cpu.regs.get32(Reg64::Rcx)),
        1.0,
        "la source est intacte"
    );
}

/// La règle qui surprend tout le monde : écrire dans `xmm3` avec une
/// instruction AVX remet à zéro la moitié haute de `ymm3`, alors que
/// la même écriture en SSE la laisse intacte. Ce sont pourtant les
/// mêmes bits.
#[test]
fn a_vex_write_clears_the_upper_half_where_sse_leaves_it() {
    let emu = run_program(
        "\
section .data
align 32
huit   dd 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0
uns    dd 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0
sortie times 32 db 0

section .text
global _start
_start:
vmovaps ymm3, [huit]
movaps xmm3, [uns]          ; SSE : le haut de ymm3 survit
vextractf128 xmm4, ymm3, 1
vmovaps [sortie], xmm4
mov eax, [sortie]           ; 5.0

vmovaps ymm3, [huit]
vmovaps xmm3, [uns]         ; AVX 128 bits : le haut passe à zéro
vextractf128 xmm5, ymm3, 1
vmovaps [sortie], xmm5
mov ebx, [sortie]           ; 0

vmovaps ymm6, [huit]
vzeroupper                  ; et voilà comment on nettoie avant du SSE
vextractf128 xmm7, ymm6, 1
vmovaps [sortie], xmm7
mov ecx, [sortie]
hlt
",
        &[],
    );
    assert_eq!(
        f32::from_bits(emu.cpu.regs.get32(Reg64::Rax)),
        5.0,
        "une écriture SSE laisse le haut"
    );
    assert_eq!(
        emu.cpu.regs.get32(Reg64::Rbx),
        0,
        "une écriture VEX l'efface"
    );
    assert_eq!(emu.cpu.regs.get32(Reg64::Rcx), 0, "vzeroupper aussi");
}

/// Les instructions propres à AVX : recopier un élément partout, et
/// déplacer les moitiés de 128 bits, qui ne se parlent pas d'
/// elles-mêmes.
#[test]
fn avx_broadcasts_and_moves_its_halves() {
    let emu = run_program(
        "\
section .data
align 32
sept   dd 7.0
huit   dd 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0
quatre dq 10, 20, 30, 40
sortie times 32 db 0

section .text
global _start
_start:
vbroadcastss ymm0, [sept]   ; huit fois 7
vmovaps [sortie], ymm0
mov eax, [sortie + 16]

; Les deux moitiés échangées : 5,6,7,8 en bas.
vmovaps ymm1, [huit]
vperm2f128 ymm2, ymm1, ymm1, 0x01
vmovaps [sortie], ymm2
mov ebx, [sortie]

; Et les quatre quadruples mots remis dans l'ordre inverse.
vmovdqa ymm3, [quatre]
vpermq ymm4, ymm3, 00_01_10_11b
vmovdqa [sortie], ymm4
mov rcx, [sortie]

vpaddq ymm5, ymm3, ymm3     ; l'entier marche aussi sur 256 bits
vmovdqa [sortie], ymm5
mov rdx, [sortie + 24]
hlt
",
        &[],
    );
    assert_eq!(
        f32::from_bits(emu.cpu.regs.get32(Reg64::Rax)),
        7.0,
        "diffusé jusque dans la moitié haute"
    );
    assert_eq!(
        f32::from_bits(emu.cpu.regs.get32(Reg64::Rbx)),
        5.0,
        "vperm2f128 a échangé les moitiés"
    );
    assert_eq!(emu.cpu.regs.get64(Reg64::Rcx), 40, "vpermq a inversé");
    assert_eq!(emu.cpu.regs.get64(Reg64::Rdx), 80, "vpaddq sur 256 bits");
}

/// La convention d'appel, vérifiée : une sous-routine qui écrase un
/// registre confié — ou qui laisse la pile de travers — est signalée au
/// retour, en nommant lequel.
///
/// Ce n'est pas une faute d'exécution : le programme continue. C'est la
/// remarque qu'on ne peut pas se faire soi-même sans comparer l'avant et
/// l'après, et c'est ce qui casse le jour où quelqu'un d'autre appelle la
/// routine.
#[test]
fn a_subroutine_that_does_not_give_back_what_it_borrowed_is_named() {
    let emu = run_program(
        "\
global _start

section .text
_start:
    push rax                ; de quoi laisser jeter un mot à la dernière
    mov rbx, 1
    mov r12, 2
    call polie
    call malpolie
    call déséquilibrée
    hlt

polie:
    push rbx
    mov rbx, 99             ; empruntée, puis rendue
    pop rbx
    ret

malpolie:
    mov rbx, 42             ; écrasée, et jamais rendue
    ret

déséquilibrée:
    pop rax                 ; l'adresse de retour
    add rsp, 8              ; et un mot de l'appelant, jeté au passage
    push rax
    ret                     ; elle revient bien, mais la pile a bougé
",
        &[],
    );

    let entorses = emu.breaches();
    let registres: Vec<&str> = entorses.iter().map(|b| b.register).collect();
    assert!(
        registres.contains(&"RBX"),
        "la routine malpolie doit être signalée : {entorses:?}"
    );
    assert!(
        !registres.contains(&"R12"),
        "R12 n'a pas bougé : {entorses:?}"
    );
    // La routine polie ne figure nulle part.
    let polies = entorses
        .iter()
        .filter(|b| b.register == "RBX")
        .collect::<Vec<_>>();
    assert_eq!(polies.len(), 1, "une seule routine fautive : {entorses:?}");
}

/// Une pile qui ne revient pas où elle était : le `push` sans `pop`, la
/// faute la plus longue à chercher soi-même.
#[test]
fn a_stack_left_crooked_is_reported() {
    let mut emu = load_only(
        "\
global _start

section .text
_start:
    call oublieuse
    hlt

oublieuse:
    push rax                ; jamais dépilé
    ret
",
        &[],
    );
    // `ret` dépile le `rax` empilé au lieu de l'adresse de retour : le
    // programme part ailleurs. L'entorse est relevée au passage — et
    // c'est elle qui explique ce qui vient d'arriver.
    let _ = emu.run(100);
    assert!(
        emu.breaches().iter().any(|b| b.register == "RSP"),
        "obtenu {:?}",
        emu.breaches()
    );
}

/// Revenir en arrière sur un `ret` retire la remarque qu'il a produite.
#[test]
fn stepping_back_takes_back_the_remark() {
    let mut emu = load_only(
        "\
global _start

section .text
_start:
    call malpolie
    hlt

malpolie:
    mov rbx, 42
    ret
",
        &[],
    );
    emu.run(100).expect("exécution");
    assert_eq!(emu.breaches().len(), 1);
    while emu.can_step_back() {
        emu.step_back();
    }
    assert!(emu.breaches().is_empty(), "obtenu {:?}", emu.breaches());
}

/// Les conditions se lisent sans exécuter quoi que ce soit : c'est ce
/// qui permet à l'interface de dire, avant le pas, si le saut sera pris.
#[test]
fn conditions_can_be_read_off_the_flags() {
    use iced_x86::ConditionCode;

    // 5 - 5 : nul, donc ZF.
    let égal = RFlags::ZF | RFlags::PF;
    assert_eq!(condition_met(ConditionCode::e, égal), Some(true));
    assert_eq!(condition_met(ConditionCode::ne, égal), Some(false));
    assert_eq!(condition_met(ConditionCode::a, égal), Some(false));
    assert_eq!(condition_met(ConditionCode::ae, égal), Some(true));

    // 1 - 2 en non signé : emprunt, donc CF ; et en signé, SF ≠ OF.
    let plus_petit = RFlags::CF | RFlags::SF;
    assert_eq!(condition_met(ConditionCode::b, plus_petit), Some(true));
    assert_eq!(condition_met(ConditionCode::l, plus_petit), Some(true));
    assert_eq!(condition_met(ConditionCode::g, plus_petit), Some(false));

    // Le cas qui piège : 0xff comparé à 1. En non signé 0xff est le plus
    // grand ; en signé, c'est -1, donc le plus petit. Les indicateurs
    // sont les mêmes, les deux sauts répondent l'inverse l'un de
    // l'autre.
    let ff_contre_1 = RFlags::empty();
    assert_eq!(condition_met(ConditionCode::a, ff_contre_1), Some(true));
    assert_eq!(condition_met(ConditionCode::g, ff_contre_1), Some(true));

    // Sans condition, rien à dire.
    assert_eq!(condition_met(ConditionCode::None, égal), None);
    assert!(condition_rule(ConditionCode::None).is_none());

    // La règle est celle qu'on énonce, et le signe est dit là où il
    // change la réponse.
    assert_eq!(condition_rule(ConditionCode::a), Some("CF = 0 et ZF = 0"));
    assert_eq!(condition_rule(ConditionCode::g), Some("ZF = 0 et SF = OF"));
    assert_eq!(condition_signed(ConditionCode::g), Some(true));
    assert_eq!(condition_signed(ConditionCode::a), Some(false));
    assert_eq!(condition_signed(ConditionCode::e), None);
}

/// Le compteur de passages ne doit pas changer l'ordre de grandeur d'une
/// exécution, et sa carte ne doit pas grandir avec elle : une entrée par
/// adresse, pas par pas.
#[test]
fn counting_visits_stays_cheap() {
    let mut emu = load_only(
        "global _start\nsection .text\n_start:\nmov rcx, 2000000\nboucle:\ndec rcx\njnz boucle\nhlt\n",
        &[],
    );
    let start = std::time::Instant::now();
    emu.run(50_000_000).unwrap();
    let elapsed = start.elapsed();
    assert_eq!(
        emu.visits().get(&emu.cpu.regs.rip),
        None,
        "l'adresse d'arrêt n'a pas encore été exécutée deux fois"
    );
    // Quatre entrées au plus — le `mov`, le corps, le saut, le `hlt` —
    // quelle que soit la durée : c'est ce qui garde le coût borné.
    assert!(emu.visits().len() <= 8, "{}", emu.visits().len());
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "quatre millions d'instructions en {elapsed:?}"
    );
    println!("PERF {elapsed:?} pour ~4 000 000 d'instructions");
}

/// Le chemin parcouru se lit dans l'ordre, et se défait avec lui.
///
/// Le compteur de passages dit combien de fois une ligne a tourné ; la
/// trace dit dans quel ordre le programme est passé — et chacun de ses
/// pas doit être un pas qu'on peut vraiment défaire, sans quoi cliquer
/// dessus ne mènerait nulle part.
#[test]
fn the_trace_follows_the_path_and_unwinds_with_it() {
    let mut emu = load_only(
        "global _start\nsection .text\n_start:\nmov rcx, 2\nboucle:\ndec rcx\njnz boucle\nhlt\n",
        &[],
    );
    let départ = emu.cpu.regs.rip;

    emu.run(100).unwrap();
    let trace: Vec<Step> = emu.trace().iter().copied().collect();
    // Le premier pas est le point d'entrée, et la trace compte autant
    // d'entrées que d'instructions exécutées.
    assert_eq!(trace.first().map(|step| step.addr), Some(départ));
    assert_eq!(trace.len() as u64, emu.executed());
    // La boucle laisse sa marque : la même adresse revient.
    let corps = trace[1].addr;
    assert!(
        trace.iter().filter(|step| step.addr == corps).count() >= 2,
        "le corps de boucle passe plusieurs fois : {trace:x?}"
    );

    // Et revenir en arrière raccourcit le chemin d'autant.
    let avant = emu.trace().len();
    assert!(emu.step_back());
    assert_eq!(emu.trace().len(), avant - 1);
    // Ce qui reste est bien le début du chemin, pas une autre liste.
    assert_eq!(emu.trace().front().map(|step| step.addr), Some(départ));
}

/// Le chemin parcouru dit ce que chaque pas a fait, pas seulement où il
/// était.
///
/// Cent adresses se ressemblent toutes ; ce qu'on cherche dans une
/// chronologie, ce sont les moments où le programme a changé de
/// direction ou touché autre chose que ses registres.
#[test]
fn the_trace_says_what_each_step_did() {
    let mut emu = load_only(
        "global _start\nsection .data\ncase: dq 0\nsection .text\n_start:\n\
         mov rax, 1\ncmp rax, 2\nje ailleurs\nmov [case], rax\ncall sous\nhlt\n\
         sous:\nret\nailleurs:\nhlt\n",
        &[],
    );
    emu.run(50).unwrap();
    let kinds: Vec<StepKind> = emu.trace().iter().map(|step| step.kind).collect();

    // Un `je` qui ne saute pas est un pas ordinaire : le montrer comme
    // un saut ferait lire un chemin qui n'a pas été pris.
    assert!(
        !kinds.contains(&StepKind::Jump),
        "aucun saut n'a été pris : {kinds:?}"
    );
    assert!(
        kinds.contains(&StepKind::Write),
        "le `mov [case], rax` écrit"
    );
    assert!(kinds.contains(&StepKind::Call), "et le `call` appelle");
    assert!(kinds.contains(&StepKind::Return), "le `ret` revient");

    // Avec le saut pris, cette fois.
    let mut emu = load_only(
        "global _start\nsection .text\n_start:\nmov rax, 2\ncmp rax, 2\nje fin\nhlt\nfin:\nhlt\n",
        &[],
    );
    emu.run(50).unwrap();
    assert!(
        emu.trace().iter().any(|step| step.kind == StepKind::Jump),
        "le saut, lui, a bien été pris"
    );
}

/// « D'où vient cette valeur ? » : l'historique sait qui a écrit en
/// dernier dans un registre.
///
/// Le chercher à la main veut dire relire son programme à l'envers en
/// suivant un registre — et se tromper une fois sur deux quand il y a
/// une boucle.
#[test]
fn the_last_write_to_a_register_can_be_found() {
    let mut emu = load_only(
        "global _start\nsection .text\n_start:\n\
         mov rax, 1\nmov rbx, 2\nadd rax, rbx\nmov rcx, 9\nhlt\n",
        &[],
    );
    emu.run(50).unwrap();

    // RAX a été écrit par le `add`, troisième pas (rang 2).
    let rang = emu.last_write_to(Reg64::Rax).expect("RAX a été écrit");
    assert_eq!(rang, 2);
    assert_eq!(emu.trace()[rang].addr, emu.trace()[2].addr);

    // RBX, lui, n'a plus bougé depuis le deuxième pas.
    assert_eq!(emu.last_write_to(Reg64::Rbx), Some(1));
    // Et un registre auquel personne n'a touché n'a pas d'écriture.
    assert_eq!(emu.last_write_to(Reg64::R15), None);

    // Dans une boucle, c'est bien la *dernière* écriture qu'on trouve.
    let mut emu = load_only(
        "global _start\nsection .text\n_start:\nmov rcx, 3\nboucle:\ndec rcx\njnz boucle\nhlt\n",
        &[],
    );
    emu.run(50).unwrap();
    let rang = emu.last_write_to(Reg64::Rcx).expect("RCX a été écrit");
    // Le dernier `dec`, pas le `mov` du départ.
    assert!(rang > 0, "rang {rang}");
    assert_eq!(emu.trace()[rang].kind, StepKind::Plain);
}

/// La même question posée à la mémoire : qui a écrit à cette adresse ?
///
/// Plus difficile encore à la main qu'avec un registre : une écriture
/// mémoire ne laisse rien de visible dans le code, il faut relire chaque
/// instruction en se demandant où elle pointait.
#[test]
fn the_last_write_to_an_address_can_be_found() {
    let mut emu = load_only(
        "global _start\nsection .data\ncase: dq 0\nautre: dq 0\nsection .text\n_start:\n\
         mov rax, 7\nmov [case], rax\nmov rbx, 9\nmov [autre], rbx\nmov qword [case], 1\nhlt\n",
        &[],
    );
    emu.run(50).unwrap();

    // L'adresse de `case` : deux écritures, on veut la dernière.
    let case = 0x401000;
    let rang = emu
        .last_write_to_memory(case)
        .expect("quelqu'un a écrit là");
    // Le dernier `mov qword [case], 1` est le cinquième pas (rang 4).
    assert_eq!(rang, 4, "trace : {:x?}", emu.trace());
    assert_eq!(emu.trace()[rang].kind, StepKind::Write);

    // Une adresse à laquelle personne n'a touché ne rend rien.
    assert_eq!(emu.last_write_to_memory(0x40ff00), None);
}

/// Les appels qui renseignent sans rien changer au monde : `getpid`,
/// `getuid`, `sched_yield`, `nanosleep`, `getrandom`.
///
/// Un programme qui les fait n'attend pas grand-chose, mais s'arrêter
/// dessus l'empêcherait d'aller plus loin — et c'est là qu'on perd un
/// débutant qui recopie un bout de code trouvé ailleurs.
#[test]
fn the_syscalls_that_only_tell_something_are_answered() {
    let programme = |numéro: u64| {
        format!(
            "\
section .text
global _start
_start:
mov rax, {numéro}
syscall
hlt
"
        )
    };

    // getpid : un numéro, et toujours le même — un atelier où l'on
    // revient en arrière doit être reproductible.
    let emu = run_program(&programme(39), &[]);
    let pid = emu.cpu.regs.get64(Reg64::Rax);
    assert_ne!(pid, 0, "un PID, pas zéro");
    let encore = run_program(&programme(39), &[]);
    assert_eq!(encore.cpu.regs.get64(Reg64::Rax), pid);

    // sched_yield et nanosleep : rien à faire, et c'est la bonne
    // réponse — il n'y a ni autre fil d'exécution ni horloge ici.
    for numéro in [24, 35] {
        let emu = run_program(&programme(numéro), &[]);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0, "appel {numéro}");
        assert!(emu.cpu.halted, "le programme est allé jusqu'au bout");
    }
    // getuid et geteuid répondent aussi.
    for numéro in [102, 107] {
        let emu = run_program(&programme(numéro), &[]);
        assert_eq!(emu.cpu.regs.get64(Reg64::Rax), pid, "appel {numéro}");
    }

    // getrandom remplit un tampon — d'octets reproductibles, et namsdec
    // ne prétend pas le contraire.
    let avec_tampon = "\
section .bss
tampon resb 16

section .text
global _start
_start:
mov rax, 318
mov rdi, tampon
mov rsi, 16
mov rdx, 0
syscall
hlt
";
    let emu = run_program(avec_tampon, &[]);
    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 16, "seize octets écrits");
    let tampon = emu.cpu.mem.peek_bytes(emu.cpu.regs.get64(Reg64::Rdi), 16);
    let octets = tampon.expect("le tampon est lisible").to_vec();
    assert!(octets.iter().any(|&b| b != 0), "quelque chose a été écrit");
    // Deux exécutions donnent les mêmes octets : c'est ce qu'on veut
    // d'un atelier, et ce qu'on ne veut surtout pas d'une clé.
    let encore = run_program(avec_tampon, &[]);
    let mêmes = encore
        .cpu
        .mem
        .peek_bytes(encore.cpu.regs.get64(Reg64::Rdi), 16)
        .unwrap()
        .to_vec();
    assert_eq!(octets, mêmes);
}

/// La comparaison qui écrit un masque, et non des indicateurs.
///
/// `comisd` pose CF et ZF, un nombre à la fois ; `cmppd` écrit le
/// résultat **dans le registre** — que des uns, ou que des zéros —, et
/// sur plusieurs valeurs à la fois. C'est ce qui permet de choisir sans
/// brancher : le masque sert ensuite d'interrupteur à `andpd`.
///
/// Les prédicats en `n` méritent leur propre vérification : `cmpnltsd`
/// n'est pas `cmpgesd`, parce qu'un NaN n'est ni plus petit ni plus
/// grand — il est *non comparable*, et tout prédicat en `n` répond vrai
/// devant lui.
#[test]
fn packed_comparison_writes_a_mask_and_a_nan_flips_the_n_predicates() {
    let emu = run_program(
        "\
section .data
paire   dq 1.0, 4.0
autre   dq 2.0, 2.0
nan     dq 0x7ff8000000000000
deux    dq 2.0

section .text
global _start
_start:
movupd xmm0, [paire]
movupd xmm1, [autre]
cmpltpd xmm0, xmm1      ; 1<2 vrai, 4<2 faux
movq rax, xmm0          ; le bas : que des uns
movhlps xmm2, xmm0
movq rbx, xmm2          ; le haut : que des zéros

; Le masque sert d'interrupteur : ce qu'il laisse passer, et rien d'autre.
movupd xmm3, [paire]
andpd xmm3, xmm0
movq rcx, xmm3          ; 1.0 conservé

; Un NaN n'est ni plus petit ni plus grand : `lt` est faux, `nlt` vrai.
movsd xmm4, [nan]
movsd xmm5, [deux]
movsd xmm6, xmm4
cmpltsd xmm6, xmm5
movq rdx, xmm6          ; faux : zéro
movsd xmm7, xmm4
cmpnltsd xmm7, xmm5
movq rsi, xmm7          ; vrai : que des uns
hlt
",
        &[],
    );
    let regs = &emu.cpu().regs;
    assert_eq!(
        regs.get64(Reg64::Rax),
        u64::MAX,
        "1 < 2 : le masque est plein"
    );
    assert_eq!(regs.get64(Reg64::Rbx), 0, "4 < 2 : le masque est vide");
    assert_eq!(
        f64::from_bits(regs.get64(Reg64::Rcx)),
        1.0,
        "le masque laisse passer ce qu'il désigne, et lui seul"
    );
    assert_eq!(regs.get64(Reg64::Rdx), 0, "NaN < 2 est faux");
    assert_eq!(
        regs.get64(Reg64::Rsi),
        u64::MAX,
        "mais « non plus petit » est vrai devant un NaN : ce n'est pas « plus grand ou égal »"
    );
}

/// `arch_prctl` installe la base FS, et `fs:[…]` s'en sert.
///
/// C'est le seul moyen, en espace utilisateur, de dire où vit la mémoire
/// locale au fil d'exécution. Un binaire lié statiquement le fait avant
/// d'appeler quoi que ce soit, et tout ce qui suit — le canari de pile en
/// tête — lit à travers FS. Sans cet appel, un programme moderne
/// s'arrêtait à sa première fonction protégée.
#[test]
fn arch_prctl_installs_the_fs_base_and_fs_reads_follow_it() {
    let emu = run_program(
        "\
section .bss
bloc: resb 64

section .text
global _start
_start:
; arch_prctl(ARCH_SET_FS, bloc)
mov rax, 158
mov rdi, 0x1002
mov rsi, bloc
syscall
; une valeur reconnaissable à fs:0x28, comme un canari
mov rcx, 0x1234567890abcdef
mov [bloc + 0x28], rcx
mov rdx, [fs:0x28]
hlt
",
        &[],
    );

    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rax),
        0,
        "l'appel doit réussir : {:?}",
        emu.cpu.regs.get64(Reg64::Rax)
    );
    assert_ne!(emu.cpu.regs.fs_base, 0, "la base est installée");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rdx),
        0x1234567890abcdef,
        "et `fs:0x28` lit bien 0x28 octets au-dessus d'elle"
    );
}

/// `mmap` donne de la mémoire vierge, utilisable, hors du tas.
///
/// C'est par lui que passe tout allocateur un peu sérieux : `brk` agrandit
/// le tas d'un seul tenant, `mmap` rend un bloc indépendant. Un programme
/// qui demande de la mémoire et écrit dedans doit pouvoir la relire.
#[test]
fn mmap_hands_out_usable_memory() {
    let mut emu = load_only(
        "\
section .text
global _start
_start:
; mmap(NULL, 8192, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0)
mov rax, 9
xor rdi, rdi
mov rsi, 8192
mov rdx, 3
mov r10, 0x22
mov r8, -1
xor r9, r9
syscall
mov rbx, rax
mov rdx, 0x4242424242424242
mov [rbx], rdx
mov rcx, [rbx]
hlt
",
        &[],
    );
    // L'arène est posée par celui qui charge : ici, juste au-dessus de
    // ce que le programme occupe.
    let fin = emu.cpu.mem.range().end;
    emu.set_mmap(fin - 0x8000, fin);
    emu.run(10_000).expect("exécution");

    let bloc = emu.cpu.regs.get64(Reg64::Rbx);
    assert!(
        bloc >= fin - 0x8000 && bloc < fin,
        "l'adresse rendue est dans l'arène : {bloc:#x}"
    );
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rcx),
        0x4242424242424242,
        "et ce qu'on y écrit se relit"
    );
    assert!(
        emu.mmap().len() >= 8192,
        "l'arène a bien donné les pages demandées"
    );
}

/// `writev` écrit ses morceaux dans l'ordre, comme `write` le ferait.
///
/// Une bibliothèque C s'en sert pour vider un tampon et ajouter la suite
/// en un seul appel — ce qui explique qu'un `printf` puisse ne jamais
/// passer par `write`.
#[test]
fn writev_writes_every_piece_in_order() {
    let emu = run_program(
        "\
section .data
un db \"bon\"
deux db \"jour\", 10
vecteur:
dq un
dq 3
dq deux
dq 5

section .text
global _start
_start:
mov rax, 20
mov rdi, 1
mov rsi, vecteur
mov rdx, 2
syscall
hlt
",
        &[],
    );

    assert_eq!(emu.output(), b"bonjour\n");
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rax),
        8,
        "et rend le total écrit"
    );
}

/// `fstat` décrit la sortie standard comme un terminal.
///
/// Une bibliothèque C interroge le descripteur avant d'écrire son premier
/// caractère : selon la réponse, elle tamponne par ligne ou par blocs.
/// Sans réponse, elle n'écrit rien du tout — et l'on cherche longtemps
/// pourquoi un `printf` ne sort pas.
#[test]
fn fstat_describes_standard_output() {
    let emu = run_program(
        "\
section .bss
statbuf: resb 144

section .text
global _start
_start:
mov rax, 5
mov rdi, 1
mov rsi, statbuf
syscall
mov ecx, [statbuf + 24]
hlt
",
        &[],
    );

    assert_eq!(emu.cpu.regs.get64(Reg64::Rax), 0, "l'appel réussit");
    // `S_IFCHR | 0620` : un périphérique caractère, ce qu'est un terminal.
    assert_eq!(
        emu.cpu.regs.get64(Reg64::Rcx) as u32,
        0x2000 | 0o620,
        "st_mode annonce un terminal"
    );
}
