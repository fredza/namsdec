//! L'oracle du coprocesseur : le mot de contrôle, confronté au vrai x87.
//!
//! L'oracle général (`examples/oracle.rs`) sème les seize registres
//! généraux et compare une instruction à la fois. Le x87 ne se mesure pas
//! comme ça : ce qu'on veut éprouver n'est pas une instruction isolée mais
//! un RÉGLAGE — quatre bits du mot de contrôle qui changent la réponse de
//! `fistp`, deux autres qui changent la précision de `fadd` —, et un
//! réglage ne se voit qu'au travers d'une petite suite d'instructions.
//!
//! D'où un protocole plus simple, et plus honnête : **un seul programme**,
//! qui règle, calcule, et range chaque résultat dans un tampon qu'il écrit
//! sur sa sortie standard. Le même programme tourne deux fois — sur le
//! processeur de la machine, puis dans `namsdec-emu` —, et l'on compare
//! les deux sorties mot pour mot. namsdec simule `write` et `exit`, c'est
//! ce qui permet au programme d'être rigoureusement le même des deux
//! côtés : aucune version « adaptée » à l'émulateur, donc aucun endroit où
//! glisser une complaisance.
//!
//! **Les écarts attendus sont déclarés d'avance**, dans la table des cas.
//! C'est ce qui sépare une mesure d'une illustration : un cas marqué
//! `Identique` qui diffère est un défaut, et un cas marqué `Écart` qui
//! coïncide est une simplification qu'on peut retirer du recensement. Les
//! deux sont des découvertes ; une table remplie après coup n'en produit
//! aucune.
//!
//! Outil de développement, et qui LIT LA MACHINE : il demande `nasm`,
//! `ld`, et un processeur x86-64 doté d'un x87 — c'est-à-dire tous. Il n'a
//! donc pas sa place dans la suite qui tourne sur l'intégration continue,
//! et n'y est pas. Les valeurs qu'il a relevées, elles, sont figées dans
//! les contrôles de `src/tests.rs`, qui tournent partout.
//!
//! ```text
//! cargo run -p namsdec-emu --example oracle_x87
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use namsdec_core::{Cpu, Memory, Reg64};
use namsdec_emu::{Backend, Emulator, Simplification};

/// Là où le programme d'épreuve est chargé côté namsdec.
const BASE: u64 = 0x400000;

/// Ce qu'on attend d'un cas, décidé AVANT de lancer la mesure.
enum Verdict {
    /// namsdec doit rendre exactement ce que le processeur rend.
    Identique,
    /// Un écart est attendu, et il a une raison nommée — celle que
    /// l'émulateur annonce à l'utilisateur.
    Écart(Simplification),
}

struct Cas {
    /// Ce que le cas montre, en une ligne.
    quoi: &'static str,
    /// Le corps du cas. Il laisse son résultat au sommet de la pile x87
    /// (`reel`) ou dans RAX (`entier`) — voir `forme`.
    code: &'static str,
    /// Comment le résultat est rangé dans le tampon.
    forme: Forme,
    attendu: Verdict,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Forme {
    /// Le cas finit par un `fstp qword [sortie]` implicite.
    Reel,
    /// Le cas a laissé son résultat dans RAX.
    Entier,
}

/// Les cas, dans l'ordre où ils paraissent au rapport.
///
/// Choisis pour ce qui DÉPEND DU RÉGLAGE, et non pour redire ce que les
/// contrôles ordinaires vérifient déjà : 3,5 arrondi quatre fois, un
/// négatif en mode dirigé — là où « vers le bas » et « vers zéro » se
/// séparent —, `fisttp` qui ignore le réglage, et les trois précisions.
fn cas() -> Vec<Cas> {
    vec![
        Cas {
            quoi: "fnstcw après fldcw 0x0f7f — le mot relu",
            code: "fldcw [vers_zero]\n    mov qword [sortie], 0\n    fnstcw [sortie]\n    \
                   mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "fistp 3,5 au plus proche",
            code: "fldcw [au_plus_proche]\n    fld qword [trois_et_demi]\n    \
                   fistp qword [sortie]\n    mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "fistp 3,5 vers zéro",
            code: "fldcw [vers_zero]\n    fld qword [trois_et_demi]\n    \
                   fistp qword [sortie]\n    mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "fistp 3,5 vers le haut",
            code: "fldcw [vers_le_haut]\n    fld qword [trois_et_demi]\n    \
                   fistp qword [sortie]\n    mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "fistp 3,5 vers le bas",
            code: "fldcw [vers_le_bas]\n    fld qword [trois_et_demi]\n    \
                   fistp qword [sortie]\n    mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "fistp -3,5 vers le bas — là où « bas » et « zéro » se séparent",
            code: "fldcw [vers_le_bas]\n    fld qword [moins_trois_et_demi]\n    \
                   fistp qword [sortie]\n    mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "fisttp 3,5 vers le haut — fisttp tranche quand même",
            code: "fldcw [vers_le_haut]\n    fld qword [trois_et_demi]\n    \
                   fisttp qword [sortie]\n    mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "fistp dword 3,5 vers le bas — la forme 32 bits",
            code: "fldcw [vers_le_bas]\n    mov qword [sortie], 0\n    \
                   fld qword [trois_et_demi]\n    fistp dword [sortie]\n    mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "frndint 2,5 vers zéro",
            code: "fldcw [vers_zero]\n    fld qword [deux_et_demi]\n    frndint",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "frndint 2,5 au plus proche pair",
            code: "fldcw [au_plus_proche]\n    fld qword [deux_et_demi]\n    frndint",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "frndint 2,5 vers le haut",
            code: "fldcw [vers_le_haut]\n    fld qword [deux_et_demi]\n    frndint",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "frndint -2,5 vers le haut",
            code: "fldcw [vers_le_haut]\n    fld qword [moins_deux_et_demi]\n    frndint",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "fnstcw après finit",
            code: "fldcw [vers_zero]\n    finit\n    mov qword [sortie], 0\n    \
                   fnstcw [sortie]\n    mov rax, [sortie]",
            forme: Forme::Entier,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "1 + 2^-30 à 24 bits de mantisse",
            code: "fldcw [simple]\n    fld qword [un]\n    fadd qword [deux_moins_30]",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "1 + 2^-30 à la précision par défaut",
            code: "fldcw [etendue]\n    fld qword [un]\n    fadd qword [deux_moins_30]",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "racine de 2 à 24 bits de mantisse",
            code: "fldcw [simple]\n    fld1\n    fld1\n    faddp\n    fsqrt",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "racine de 2 à la précision par défaut",
            code: "fldcw [etendue]\n    fld1\n    fld1\n    faddp\n    fsqrt",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "(1 + 10^-18) - 1 à 53 bits — la précision que namsdec calcule",
            code: "fldcw [double]\n    fld qword [un]\n    fadd qword [tres_petit]\n    \
                   fsub qword [un]",
            forme: Forme::Reel,
            attendu: Verdict::Identique,
        },
        Cas {
            quoi: "(1 + 10^-18) - 1 à 64 bits — les onze bits que namsdec n'a pas",
            code: "fldcw [etendue]\n    fld qword [un]\n    fadd qword [tres_petit]\n    \
                   fsub qword [un]",
            forme: Forme::Reel,
            attendu: Verdict::Écart(Simplification::X87PrecisionEtendue),
        },
        // Celui-ci isole la LECTURE d'un 80 bits, et rien d'autre. La
        // valeur est posée octet par octet — 1 + 2^-63, qu'aucune
        // directive `dt` n'écrirait depuis un littéral décimal — et le
        // calcul se fait à 53 bits, où la différence 2^-63 est
        // exactement représentable. Le seul endroit où namsdec peut
        // perdre est donc la conversion à l'entrée : il la perd, et
        // `fsub` rend zéro là où le processeur rend 2^-63.
        Cas {
            quoi: "fld d'un vrai 80 bits : 1 + 2^-63, moins 1",
            code: "fldcw [double]\n    fld tword [un_et_un_bit]\n    fsub qword [un]",
            forme: Forme::Reel,
            attendu: Verdict::Écart(Simplification::X87FormatEtenduEnMemoire),
        },
    ]
}

/// Les données que les cas nomment. Un seul endroit : un cas qui invente
/// son propre `1.0` finirait par ne plus parler du même nombre que son
/// voisin.
const DONNÉES: &str = "\
section .data
au_plus_proche      dw 0x037f
vers_le_bas         dw 0x077f
vers_le_haut        dw 0x0b7f
vers_zero           dw 0x0f7f
simple              dw 0x007f
double              dw 0x027f
etendue             dw 0x037f
un                  dq 1.0
trois_et_demi       dq 3.5
moins_trois_et_demi dq -3.5
deux_et_demi        dq 2.5
moins_deux_et_demi  dq -2.5
tres_petit          dq 1.0e-18
deux_moins_30       dq 9.3132257461547852e-10
sortie              dq 0
etendu_en_memoire   dt 0.0
; 1 + 2^-63, écrit à la main : mantisse 0x8000000000000001, exposant
; 16383. Aucun littéral décimal ne donnerait cette valeur-là, et c'est
; justement celle qui tient dans un 80 bits et pas dans un double.
un_et_un_bit        db 0x01, 0, 0, 0, 0, 0, 0, 0x80, 0xff, 0x3f
";

/// Le programme d'épreuve : les cas à la suite, chacun rangeant son mot
/// dans le tampon, puis un `write` et un `exit`.
fn programme(cas: &[Cas]) -> String {
    let mut texte = String::from(DONNÉES);
    texte.push_str(&format!(
        "\nsection .bss\nreleve resq {}\n\nsection .text\nglobal _start\n_start:\n",
        cas.len()
    ));
    for (rang, c) in cas.iter().enumerate() {
        texte.push_str(&format!("    ; --- {rang} : {}\n    finit\n", c.quoi));
        texte.push_str("    ");
        texte.push_str(c.code);
        texte.push('\n');
        match c.forme {
            Forme::Reel => {
                texte.push_str(&format!("    fstp qword [releve + {rang} * 8]\n"));
            }
            Forme::Entier => {
                texte.push_str(&format!("    mov [releve + {rang} * 8], rax\n"));
            }
        }
    }
    texte.push_str(&format!(
        "\n    mov rax, 1\n    mov rdi, 1\n    mov rsi, releve\n    mov rdx, {}\n    syscall\n\
         \n    mov rax, 60\n    xor edi, edi\n    syscall\n",
        cas.len() * 8
    ));
    texte
}

/// Assemble, lie, et lance sur le processeur de la machine.
fn sous_le_matériel(dossier: &Path, source: &str, combien: usize) -> Result<Vec<u64>, String> {
    let asm = dossier.join("x87.asm");
    let objet = dossier.join("x87.o");
    let binaire = dossier.join("x87");
    std::fs::write(&asm, source).map_err(|e| e.to_string())?;

    let nasm = Command::new("nasm")
        .args(["-f", "elf64"])
        .arg(&asm)
        .arg("-o")
        .arg(&objet)
        .output()
        .map_err(|e| format!("nasm introuvable : {e}"))?;
    if !nasm.status.success() {
        return Err(format!(
            "nasm refuse : {}",
            String::from_utf8_lossy(&nasm.stderr).trim()
        ));
    }
    let ld = Command::new("ld")
        .arg(&objet)
        .arg("-o")
        .arg(&binaire)
        .output()
        .map_err(|e| format!("ld introuvable : {e}"))?;
    if !ld.status.success() {
        return Err(format!(
            "ld refuse : {}",
            String::from_utf8_lossy(&ld.stderr).trim()
        ));
    }
    let sortie = Command::new(&binaire)
        .output()
        .map_err(|e| format!("le programme d'épreuve ne se lance pas : {e}"))?;
    lire_mots(&sortie.stdout, combien, "le processeur")
}

/// Et le même programme, dans `namsdec-emu`.
fn sous_namsdec(source: &str, combien: usize) -> Result<(Vec<u64>, Vec<Simplification>), String> {
    let out = namsdec_asm::assemble(source, BASE).map_err(|e| format!("assemblage : {e}"))?;
    let mut mem = Memory::new(BASE, 0x10000);
    mem.load(out.load_addr, &out.bytes)
        .map_err(|e| format!("chargement : {e}"))?;
    let mut cpu = Cpu::new(mem);
    cpu.regs.rip = out.entry;
    cpu.regs.set64(Reg64::Rsp, BASE + 0x8000);
    let mut emu = Emulator::new(cpu);
    emu.set_heap(out.load_addr + out.bytes.len() as u64, BASE + 0x7000);
    emu.run(100_000).map_err(|e| format!("exécution : {e}"))?;
    let mots = lire_mots(emu.output(), combien, "namsdec")?;
    let touchées = emu.simplifications().iter().copied().collect();
    Ok((mots, touchées))
}

fn lire_mots(brut: &[u8], combien: usize, qui: &str) -> Result<Vec<u64>, String> {
    if brut.len() < combien * 8 {
        return Err(format!(
            "{qui} n'a rendu que {} octets sur {} — le programme s'est arrêté avant la fin",
            brut.len(),
            combien * 8
        ));
    }
    Ok((0..combien)
        .map(|rang| {
            let mut huit = [0u8; 8];
            huit.copy_from_slice(&brut[rang * 8..rang * 8 + 8]);
            u64::from_le_bytes(huit)
        })
        .collect())
}

/// Un mot, écrit comme le cas demande à être lu.
fn montrer(mot: u64, forme: Forme) -> String {
    match forme {
        Forme::Entier => format!("{} ({mot:#018x})", mot as i64),
        Forme::Reel => format!("{:?} ({mot:#018x})", f64::from_bits(mot)),
    }
}

/// Un dossier de travail à soi. On ne l'efface pas : quand un écart
/// paraît, le programme qui l'a produit est la première chose qu'on veut
/// relire.
fn tempdir() -> std::io::Result<PathBuf> {
    let base = std::env::temp_dir().join(format!("namsdec-oracle-x87-{}", std::process::id()));
    std::fs::create_dir_all(&base)?;
    Ok(base)
}

fn main() {
    let cas = cas();
    let source = programme(&cas);
    let dossier = match tempdir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("dossier de travail : {e}");
            std::process::exit(2);
        }
    };
    println!(
        "Programme d'épreuve : {}",
        dossier.join("x87.asm").display()
    );
    println!("{} cas, un seul programme, deux exécutions.\n", cas.len());

    let matériel = match sous_le_matériel(&dossier, &source, cas.len()) {
        Ok(mots) => mots,
        Err(e) => {
            eprintln!("côté processeur : {e}");
            std::process::exit(2);
        }
    };
    let (namsdec, touchées) = match sous_namsdec(&source, cas.len()) {
        Ok(résultat) => résultat,
        Err(e) => {
            eprintln!("côté namsdec : {e}");
            std::process::exit(2);
        }
    };

    let mut défauts = 0usize;
    let mut écarts_attendus = 0usize;
    let mut écarts_disparus = 0usize;
    for (rang, c) in cas.iter().enumerate() {
        let (m, n) = (matériel[rang], namsdec[rang]);
        let même = m == n;
        let étiquette = match (&c.attendu, même) {
            (Verdict::Identique, true) => "ok      ",
            (Verdict::Identique, false) => {
                défauts += 1;
                "DÉFAUT  "
            }
            (Verdict::Écart(_), false) => {
                écarts_attendus += 1;
                "écart   "
            }
            (Verdict::Écart(_), true) => {
                écarts_disparus += 1;
                "COÏNCIDE"
            }
        };
        println!("{étiquette} {rang:>2}. {}", c.quoi);
        if !même {
            println!("          processeur : {}", montrer(m, c.forme));
            println!("          namsdec    : {}", montrer(n, c.forme));
            if let Verdict::Écart(quoi) = &c.attendu {
                let annoncé = touchées.contains(quoi);
                println!(
                    "          attendu, et {} par l'émulateur ({})",
                    if annoncé { "annoncé" } else { "NON ANNONCÉ" },
                    quoi.cle()
                );
                if !annoncé {
                    défauts += 1;
                }
            }
        }
    }

    println!(
        "\n{} cas — {} identiques, {écarts_attendus} écart(s) attendu(s), \
         {défauts} défaut(s), {écarts_disparus} écart(s) disparu(s).",
        cas.len(),
        cas.len() - écarts_attendus - défauts - écarts_disparus
    );
    if écarts_disparus > 0 {
        println!(
            "Un écart qui coïncide est une bonne nouvelle : la simplification \
             correspondante peut sortir du recensement."
        );
    }
    println!(
        "Simplifications touchées par ce programme : {}",
        if touchées.is_empty() {
            "aucune".to_owned()
        } else {
            touchées
                .iter()
                .map(|s| s.cle())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    if défauts > 0 {
        std::process::exit(1);
    }
}
