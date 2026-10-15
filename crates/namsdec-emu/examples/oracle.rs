//! L'oracle : ce que le VRAI processeur répond, face à `namsdec-emu`.
//!
//! Treize mille lignes de sémantique écrites à la main ne se vérifient
//! pas en les relisant. Un indicateur calculé de travers ne plante pas :
//! il rend une mauvaise réponse, en silence, et le programme émulé part
//! dans une branche qu'il n'aurait pas prise. C'est le seul endroit du
//! projet où une erreur peut vivre longtemps sans rien dire.
//!
//! Le remède est ancien et ne demande aucune intelligence : on fait
//! faire la même chose aux deux, et l'on compare. Pour chaque cas —
//!
//!   1. on sème les seize registres généraux et les indicateurs ;
//!   2. on bâtit un petit programme qui pose cet état, exécute
//!      l'instruction, puis s'arrête sur un `int3` ;
//!   3. `gdb` le lance sur le processeur de la machine et relève ce
//!      qu'il trouve ;
//!   4. `namsdec-emu` part du même état et exécute les mêmes octets ;
//!   5. on compare registre par registre, indicateur par indicateur.
//!
//! Ce qui en fait un oracle et non un test de plus : il n'y a **aucune
//! réponse écrite à la main**. On ne dit jamais ce que `adc` devrait
//! donner — on le demande au matériel. Un cas qu'on n'avait pas prévu
//! est donc traité comme les autres, et c'est précisément ceux-là qui
//! trouvent les défauts.
//!
//! Outil de développement, et qui LIT LA MACHINE : il demande `nasm`,
//! `ld` et `gdb`, et ne vaut que sur x86-64. Il n'a donc pas sa place
//! dans la suite qui tourne sur l'intégration continue, et n'y est pas.
//!
//! ```text
//! cargo run -p namsdec-emu --example oracle
//! cargo run -p namsdec-emu --example oracle -- "adc rax, rbx"
//! cargo run -p namsdec-emu --example oracle -- --graines 200
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use namsdec_core::{Cpu, Memory, RFlags, Reg64, RegXmm};
use namsdec_emu::Emulator;

/// Là où le programme d'épreuve est chargé, des deux côtés.
const BASE: u64 = 0x400000;

/// Les seize registres, dans l'ordre de l'encodage x86 — celui de
/// `Reg64`, et celui que `gdb` emploie aussi.
const NOMS: [&str; 16] = [
    "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15",
];

const REGS: [Reg64; 16] = [
    Reg64::Rax,
    Reg64::Rcx,
    Reg64::Rdx,
    Reg64::Rbx,
    Reg64::Rsp,
    Reg64::Rbp,
    Reg64::Rsi,
    Reg64::Rdi,
    Reg64::R8,
    Reg64::R9,
    Reg64::R10,
    Reg64::R11,
    Reg64::R12,
    Reg64::R13,
    Reg64::R14,
    Reg64::R15,
];

/// Les quatre registres vectoriels semés.
const XMMS: [RegXmm; 4] = [RegXmm::Xmm0, RegXmm::Xmm1, RegXmm::Xmm2, RegXmm::Xmm3];

/// Les six indicateurs que l'arithmétique recalcule.
///
/// TF, IF et DF ne sont pas comparés : le noyau en dispose à sa guise au
/// lancement d'un processus, et aucune des instructions éprouvées ici ne
/// les touche. Les comparer ferait échouer tous les cas sur une
/// différence qui n'appartient pas à l'instruction.
const INDICATEURS: [(&str, RFlags); 6] = [
    ("CF", RFlags::CF),
    ("PF", RFlags::PF),
    ("AF", RFlags::AF),
    ("ZF", RFlags::ZF),
    ("SF", RFlags::SF),
    ("OF", RFlags::OF),
];

/// Les indicateurs que l'architecture laisse INDÉFINIS après cette
/// instruction, et qu'il ne faut donc pas comparer.
///
/// C'est ce qui sépare un oracle utile d'un oracle qui crie au loup. Le
/// manuel d'Intel distingue trois sorts pour un indicateur : calculé,
/// inchangé, ou **indéfini**. Indéfini ne veut pas dire « vaut n'importe
/// quoi » : il veut dire que le processeur a le droit d'y mettre ce
/// qu'il veut, que deux modèles peuvent différer, et qu'un programme qui
/// le lit a déjà tort. Comparer ces bits-là reviendrait à reprocher à
/// namsdec de ne pas être ce processeur-ci — ce qu'aucun émulateur ne
/// peut ni ne doit être.
///
/// La liste vient du volume 2 du manuel d'Intel, instruction par
/// instruction. Les silences sont volontaires : `add`, `sub`, `adc`,
/// `sbb`, `cmp`, `neg`, `inc`, `dec`, `and`, `or`, `xor`, `test`
/// définissent TOUT ce qu'elles touchent, et c'est pour cela qu'on les
/// compare en entier.
fn indéfinis(instruction: &str) -> RFlags {
    let mnémonique = instruction
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    // Un décalage ou une rotation « de 1 » est un cas à part : OF y est
    // DÉFINI. Au-delà de un, il ne l'est plus — et c'est la distinction
    // que les trois quarts des émulateurs ratent.
    let de_un = instruction.trim_end().ends_with(", 1");

    match mnémonique.as_str() {
        // CF et OF disent si le résultat déborde de la moitié basse ;
        // les quatre autres sont indéfinis.
        "mul" | "imul" => RFlags::SF | RFlags::ZF | RFlags::AF | RFlags::PF,
        // CF prend le bit sorti ; SF, ZF et PF suivent le résultat ; AF
        // est indéfini, et OF ne vaut que pour un décalage de 1.
        "shl" | "sal" | "shr" | "sar" => {
            let mut masque = RFlags::AF;
            if !de_un {
                masque |= RFlags::OF;
            }
            masque
        }
        // Une rotation ne touche QUE CF et OF, et OF seulement pour 1.
        "rol" | "ror" | "rcl" | "rcr" => {
            if de_un {
                RFlags::empty()
            } else {
                RFlags::OF
            }
        }
        // Comme les décalages, AF en plus — et le décalage « de 1 » de
        // shld/shrd s'écrit avec un troisième opérande.
        "shld" | "shrd" => {
            let mut masque = RFlags::AF;
            if !instruction.trim_end().ends_with(", 1") {
                masque |= RFlags::OF;
            }
            masque
        }
        // Seul ZF est défini : il dit que la source était nulle, auquel
        // cas la destination elle-même est indéfinie — mais cela, c'est
        // au banc de registres de le savoir, pas à nous.
        "bsf" | "bsr" => RFlags::CF | RFlags::OF | RFlags::SF | RFlags::AF | RFlags::PF,
        // CF et ZF sont définis ; les quatre autres, non.
        "lzcnt" | "tzcnt" => RFlags::OF | RFlags::SF | RFlags::AF | RFlags::PF,
        // CF prend le bit visé ; ZF n'est pas touché ; le reste est
        // indéfini.
        "bt" | "bts" | "btr" | "btc" => RFlags::OF | RFlags::SF | RFlags::AF | RFlags::PF,
        // Tout est indéfini après une division.
        "div" | "idiv" => RFlags::all(),
        // BMI1 : SF et ZF suivent le résultat, CF et OF ont chacune leur
        // règle, AF et PF sont indéfinis. C'est par ce dernier point que
        // l'oracle a d'abord cru voir quatre défauts là où il n'y en
        // avait qu'un — mais ce dernier était réel, et c'est `bzhi`.
        "andn" | "blsi" | "blsr" | "blsmsk" | "bzhi" => RFlags::AF | RFlags::PF,
        // `bextr` : seul ZF est défini.
        "bextr" => RFlags::CF | RFlags::OF | RFlags::SF | RFlags::AF | RFlags::PF,
        _ => RFlags::empty(),
    }
}

/// L'état comparé : seize registres et six indicateurs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct État {
    gpr: [u64; 16],
    flags: RFlags,
    /// `ymm0`, en quatre mots de 64 bits, du bas vers le haut.
    ///
    /// Deux cent cinquante-six bits et non cinq cent douze : ce
    /// processeur-ci n'a pas AVX-512 (voir le commentaire de
    /// `répertoire_vectoriel`), et relever un `zmm` qui n'existe pas
    /// n'apprendrait rien.
    ///
    /// Comparé même pour une instruction entière : si `add rax, rbx`
    /// touchait un registre vectoriel, on voudrait le savoir.
    ymm0: [u64; 4],
}

impl État {
    /// Ce qui diffère entre deux états, nommé.
    ///
    /// `indéfinis` dit les indicateurs que l'architecture ne garantit
    /// pas : on les saute, sans quoi l'oracle reprocherait à namsdec de
    /// n'être pas ce processeur-ci.
    fn écarts(&self, autre: &État, indéfinis: RFlags) -> Vec<String> {
        let mut écarts = Vec::new();
        for (i, nom) in NOMS.iter().enumerate() {
            if self.gpr[i] != autre.gpr[i] {
                écarts.push(format!(
                    "{nom} : matériel {:#018x}, namsdec {:#018x}",
                    self.gpr[i], autre.gpr[i]
                ));
            }
        }
        if self.ymm0 != autre.ymm0 {
            écarts.push(format!(
                "ymm0 : matériel {}, namsdec {}",
                en_hexa(&self.ymm0),
                en_hexa(&autre.ymm0)
            ));
        }
        for (nom, bit) in INDICATEURS {
            if indéfinis.contains(bit) {
                continue;
            }
            let m = self.flags.contains(bit);
            let n = autre.flags.contains(bit);
            if m != n {
                écarts.push(format!(
                    "{nom} : matériel {}, namsdec {}",
                    u8::from(m),
                    u8::from(n)
                ));
            }
        }
        écarts
    }
}

/// Un état de départ.
#[derive(Debug, Clone, Copy)]
struct Graine {
    gpr: [u64; 16],
    /// `ymm0` à `ymm3`, semés comme les généraux.
    ymm: [[u64; 4]; 4],
    /// Les indicateurs posés AVANT l'instruction. Ils comptent : `adc`
    /// et `sbb` lisent CF, et c'est justement là que les erreurs se
    /// cachent.
    flags: u64,
}

impl Graine {
    /// Des valeurs tirées d'un générateur simple, mais pas quelconques.
    ///
    /// Un tirage uniforme sur 64 bits ne tombe jamais sur zéro, ni sur
    /// un, ni sur la limite du signe — c'est-à-dire exactement les
    /// valeurs où les indicateurs basculent. On mêle donc les deux : des
    /// valeurs remarquables, et du tirage pour le reste.
    fn tirée(graine: &mut u64) -> Self {
        const REMARQUABLES: [u64; 10] = [
            0,
            1,
            u64::MAX,
            0x7fff_ffff_ffff_ffff,
            0x8000_0000_0000_0000,
            0xffff_ffff,
            0x8000_0000,
            0x7fff_ffff,
            0xff,
            0x80,
        ];
        let mut gpr = [0u64; 16];
        for case in gpr.iter_mut() {
            *case = match suivant(graine) % 3 {
                0 => REMARQUABLES[(suivant(graine) % REMARQUABLES.len() as u64) as usize],
                _ => suivant(graine),
            };
        }
        let mut ymm = [[0u64; 4]; 4];
        for registre in ymm.iter_mut() {
            for mot in registre.iter_mut() {
                *mot = match suivant(graine) % 3 {
                    0 => REMARQUABLES[(suivant(graine) % REMARQUABLES.len() as u64) as usize],
                    _ => suivant(graine),
                };
            }
        }
        Self {
            gpr,
            ymm,
            // Seuls CF, PF, AF, ZF, SF, OF sont semés, et le bit 1 est
            // toujours à un sur x86 : `popfq` le force de toute façon,
            // et l'y mettre nous-mêmes évite un faux écart.
            flags: 0x002 | (suivant(graine) & 0x8d5),
        }
    }
}

/// Un générateur xorshift : court, reproductible, et sans dépendance.
///
/// Reproductible est le mot important — un oracle qui trouve un défaut
/// doit pouvoir le remontrer, et une graine qu'on peut réécrire suffit.
fn suivant(état: &mut u64) -> u64 {
    *état ^= *état << 13;
    *état ^= *état >> 7;
    *état ^= *état << 17;
    *état
}

/// Le petit programme d'épreuve, en assembleur.
///
/// L'ordre n'est pas libre : `popfq` a besoin d'une pile, donc `rsp`
/// d'abord ; les indicateurs ensuite ; et les registres en dernier,
/// parce qu'un `mov` ne touche aucun indicateur et ne défait donc pas
/// ce que `popfq` vient de poser.
fn programme(graine: &Graine, instruction: &str) -> String {
    let mut asm = String::from("bits 64\nsection .text\nglobal _start\n_start:\n");
    asm.push_str("    mov rsp, pile_haut\n");
    // Les vecteurs en premier : `vmovdqu` ne touche aucun indicateur,
    // donc l'ordre est libre, mais les poser avant `popfq` évite d'avoir
    // à le redémontrer à chaque relecture.
    for registre in 0..4 {
        asm.push_str(&format!("    vmovdqu ymm{registre}, [semis{registre}]\n"));
    }
    asm.push_str(&format!("    mov rax, {:#x}\n", graine.flags));
    asm.push_str("    push rax\n    popfq\n");
    for (i, nom) in NOMS.iter().enumerate() {
        // `rsp` porte la pile : le semer l'écraserait, et l'instruction
        // éprouvée n'a pas à en dépendre.
        if *nom == "rsp" {
            continue;
        }
        asm.push_str(&format!("    mov {nom}, {:#x}\n", graine.gpr[i]));
    }
    asm.push_str(&format!("    {instruction}\n"));
    // `ymm0` rangé en mémoire pour que le débogueur puisse le lire :
    // relever un registre vectoriel par `info registers` dépend du
    // format que gdb choisit, et ce format change d'une version à
    // l'autre. Une adresse et huit octets, non.
    asm.push_str("    vmovdqu [resultat], ymm0\n");
    // `int3` arrête net et rend la main au débogueur, sans rien changer
    // aux indicateurs.
    asm.push_str("    int3\n");

    asm.push_str("\nsection .data\nalign 32\n");
    for (registre, valeurs) in graine.ymm.iter().enumerate() {
        asm.push_str(&format!("semis{registre}:\n"));
        for mot in valeurs {
            asm.push_str(&format!("    dq {mot:#x}\n"));
        }
    }
    asm.push_str("resultat:\n    times 32 db 0\n");
    asm.push_str("\nsection .bss\n    resb 4096\npile_haut:\n");
    asm
}

/// Assemble et lie le programme d'épreuve. Rend son chemin.
fn bâtir(dossier: &Path, asm: &str) -> Result<PathBuf, String> {
    let source = dossier.join("essai.asm");
    let objet = dossier.join("essai.o");
    let programme = dossier.join("essai");
    std::fs::write(&source, asm).map_err(|e| e.to_string())?;

    let nasm = Command::new("nasm")
        .args(["-f", "elf64"])
        .arg(&source)
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
        .arg(&programme)
        .output()
        .map_err(|e| format!("ld introuvable : {e}"))?;
    if !ld.status.success() {
        return Err(format!(
            "ld refuse : {}",
            String::from_utf8_lossy(&ld.stderr).trim()
        ));
    }
    Ok(programme)
}

/// Ce que le vrai processeur a fait, relevé par `gdb`.
///
/// On le lance en lot, sans fichier d'initialisation (`-nx`) : celui de
/// l'utilisateur pourrait changer le format des registres, et le relevé
/// deviendrait illisible pour des raisons qui n'ont rien à voir avec
/// l'instruction.
fn sous_le_matériel(programme: &Path) -> Result<État, String> {
    let sortie = Command::new("gdb")
        .args([
            "-batch",
            "-nx",
            "-ex",
            "run",
            "-ex",
            "info registers",
            // Un repère, pour que le relevé du vecteur ne se confonde
            // pas avec celui des registres : les deux sont faits de
            // nombres en hexadécimal, et les mélanger donnerait un état
            // faux sans rien signaler.
            "-ex",
            "echo ===VECTEUR===\n",
            "-ex",
            "x/4gx &resultat",
        ])
        .arg(programme)
        .output()
        .map_err(|e| format!("gdb introuvable : {e}"))?;
    let texte = String::from_utf8_lossy(&sortie.stdout);
    let (avant, après) = match texte.split_once("===VECTEUR===") {
        Some(deux) => deux,
        None => (texte.as_ref(), ""),
    };

    // Les quatre mots du vecteur, dans l'ordre où `x/4gx` les donne :
    // du bas vers le haut, comme la mémoire les porte.
    let mut ymm0 = [0u64; 4];
    let mut lus = 0usize;
    for ligne in après.lines() {
        let Some((_, droite)) = ligne.split_once(':') else {
            continue;
        };
        for mot in droite.split_whitespace() {
            let Some(hexa) = mot.strip_prefix("0x") else {
                continue;
            };
            if let (Ok(valeur), true) = (u64::from_str_radix(hexa, 16), lus < 4) {
                ymm0[lus] = valeur;
                lus += 1;
            }
        }
    }
    if lus < 4 {
        return Err(format!(
            "gdb n'a pas rendu les 32 octets du résultat :\n{}",
            après.trim()
        ));
    }
    let texte = avant;

    let mut valeurs: BTreeMap<&str, u64> = BTreeMap::new();
    let mut eflags: Option<u64> = None;
    for ligne in texte.lines() {
        let mut mots = ligne.split_whitespace();
        let (Some(nom), Some(brut)) = (mots.next(), mots.next()) else {
            continue;
        };
        let Some(hexa) = brut.strip_prefix("0x") else {
            continue;
        };
        let Ok(valeur) = u64::from_str_radix(hexa, 16) else {
            continue;
        };
        if nom == "eflags" {
            eflags = Some(valeur);
        } else if NOMS.contains(&nom) {
            valeurs.insert(NOMS.iter().find(|n| **n == nom).unwrap(), valeur);
        }
    }

    let eflags = eflags.ok_or_else(|| {
        format!(
            "gdb n'a pas rendu les indicateurs — le programme n'a peut-être pas \
             atteint son int3 :\n{}",
            texte.trim()
        )
    })?;
    let mut gpr = [0u64; 16];
    for (i, nom) in NOMS.iter().enumerate() {
        gpr[i] = *valeurs
            .get(nom)
            .ok_or_else(|| format!("gdb n'a pas rendu {nom}"))?;
    }
    Ok(État {
        gpr,
        flags: RFlags::from_bits_truncate(eflags),
        ymm0,
    })
}

/// Quatre mots de 64 bits, écrits comme on lit un registre : du haut
/// vers le bas.
fn en_hexa(mots: &[u64; 4]) -> String {
    format!(
        "{:016x}_{:016x}_{:016x}_{:016x}",
        mots[3], mots[2], mots[1], mots[0]
    )
}

/// Ce que namsdec fait des mêmes octets, depuis le même état.
///
/// Une seule instruction est exécutée : celle qu'on éprouve. Le reste du
/// programme d'épreuve — poser les registres, poser les indicateurs — est
/// remplacé ici par un appel direct, pour que l'écart constaté ne puisse
/// venir que de l'instruction visée.
fn sous_namsdec(octets: &[u8], graine: &Graine, rsp: u64) -> Result<État, String> {
    let mut mem = Memory::new(BASE, 0x10000);
    mem.load(BASE, octets).map_err(|e| format!("{e:?}"))?;
    let mut cpu = Cpu::new(mem);
    for (i, reg) in REGS.iter().enumerate() {
        cpu.regs.set64(*reg, graine.gpr[i]);
    }
    // Les quatre registres vectoriels, montés en tranches de 128 bits :
    // c'est ainsi que `namsdec-core` les range, le bas d'abord.
    for (registre, valeurs) in graine.ymm.iter().enumerate() {
        cpu.regs.set_zmm(
            XMMS[registre],
            [
                valeurs[0] as u128 | ((valeurs[1] as u128) << 64),
                valeurs[2] as u128 | ((valeurs[3] as u128) << 64),
                0,
                0,
            ],
        );
    }
    // `rsp` vaut ce que le programme d'épreuve lui a donné : la même
    // valeur des deux côtés, sans quoi toute instruction qui le lit
    // paraîtrait fautive.
    cpu.regs.set64(Reg64::Rsp, rsp);
    cpu.flags = RFlags::from_bits_truncate(graine.flags);
    cpu.regs.rip = BASE;

    let mut emu = Emulator::new(cpu);
    emu.step().map_err(|e| format!("namsdec refuse : {e:?}"))?;

    let mut gpr = [0u64; 16];
    for (i, reg) in REGS.iter().enumerate() {
        gpr[i] = emu.cpu.regs.get64(*reg);
    }
    let zmm0 = emu.cpu.regs.get_zmm(XMMS[0]);
    Ok(État {
        gpr,
        flags: emu.cpu.flags,
        ymm0: [
            zmm0[0] as u64,
            (zmm0[0] >> 64) as u64,
            zmm0[1] as u64,
            (zmm0[1] >> 64) as u64,
        ],
    })
}

/// Les octets d'une instruction, assemblés à l'adresse où elle tournera.
fn octets_de(dossier: &Path, instruction: &str) -> Result<Vec<u8>, String> {
    let source = dossier.join("une.asm");
    let binaire = dossier.join("une.bin");
    std::fs::write(&source, format!("bits 64\norg {BASE:#x}\n{instruction}\n"))
        .map_err(|e| e.to_string())?;
    let nasm = Command::new("nasm")
        .args(["-f", "bin"])
        .arg(&source)
        .arg("-o")
        .arg(&binaire)
        .output()
        .map_err(|e| format!("nasm introuvable : {e}"))?;
    if !nasm.status.success() {
        return Err(format!(
            "nasm refuse « {instruction} » : {}",
            String::from_utf8_lossy(&nasm.stderr).trim()
        ));
    }
    std::fs::read(&binaire).map_err(|e| e.to_string())
}

/// Les instructions éprouvées par défaut.
///
/// Choisies pour ce qu'elles font aux INDICATEURS, pas pour leur
/// résultat : c'est là que la sémantique écrite à la main se trompe.
/// `adc` et `sbb` lisent CF en plus de l'écrire ; `inc` et `dec` sont les
/// seules arithmétiques à NE PAS toucher CF ; `shl`/`shr` ont une règle
/// particulière pour OF et laissent les indicateurs intacts quand le
/// décalage est nul ; `bt` et `bsf` ont chacune leur exception.
fn répertoire() -> Vec<&'static str> {
    vec![
        "add rax, rbx",
        "add al, bl",
        "adc rax, rbx",
        "adc eax, ebx",
        "sub rax, rbx",
        "sbb rax, rbx",
        "sbb al, bl",
        "cmp rax, rbx",
        "inc rax",
        "dec rax",
        "neg rax",
        "and rax, rbx",
        "or rax, rbx",
        "xor rax, rbx",
        "test rax, rbx",
        "not rax",
        "shl rax, 1",
        "shl rax, cl",
        "shr rax, 1",
        "shr rax, cl",
        "sar rax, cl",
        "rol rax, cl",
        "ror rax, cl",
        "rcl rax, 1",
        "rcr rax, 1",
        "imul rax, rbx",
        "mul rbx",
        "bt rax, rbx",
        "bts rax, rbx",
        "btr rax, rbx",
        "bsf rax, rbx",
        "bsr rax, rbx",
        "popcnt rax, rbx",
        "lzcnt rax, rbx",
        "tzcnt rax, rbx",
        "xchg rax, rbx",
        "movzx eax, bl",
        "movsx rax, bl",
        "movsxd rax, ebx",
        "cmovz rax, rbx",
        "setz al",
        "bswap rax",
        "cqo",
        "cdqe",
        "lea rax, [rbx+rcx*4+0x10]",
        "shld rax, rbx, cl",
        "shrd rax, rbx, cl",
        "cmc",
        "clc",
        "stc",
    ]
}

/// Les instructions vectorielles éprouvées, et jusqu'où cette machine
/// permet d'aller.
///
/// **Ce processeur n'a pas AVX-512.** Un Core Ultra 7 258V (Lunar Lake)
/// expose `avx`, `avx2`, `avx_vnni`, `bmi1`, `bmi2`, `f16c` — et rien
/// en `avx512`. Intel l'a retiré des cœurs hybrides grand public. Donc
/// la partie la plus récente de `namsdec-emu`, celle des registres de
/// 512 bits et des masques, est précisément celle que cette machine ne
/// peut PAS confronter au matériel. Le dire est le plus utile de ce que
/// cet oracle apprend : ce n'est pas que namsdec s'y trompe, c'est
/// qu'on n'en sait rien ici.
///
/// Pour l'éprouver quand même, il faudrait un oracle qui n'est pas ce
/// processeur : Intel SDE, qui émule AVX-512 sur n'importe quel x86, ou
/// QEMU avec un modèle de processeur qui l'annonce. L'un comme l'autre
/// se branchent à la place de `gdb` dans `sous_le_matériel`, et c'est
/// pour cela que cette fonction est seule à connaître l'outil.
fn répertoire_vectoriel() -> Vec<&'static str> {
    vec![
        // Ce que le déplacement doit préserver, et ce qu'il efface : un
        // écrit en VEX de 128 bits met à ZÉRO la moitié haute du
        // registre de 256. C'est la règle qu'on oublie, et elle se voit
        // ici du premier coup.
        "vmovdqa xmm0, xmm1",
        "vmovdqu ymm0, ymm1",
        "vpxor xmm0, xmm1, xmm2",
        "vpxor ymm0, ymm1, ymm2",
        "vpand ymm0, ymm1, ymm2",
        "vpor ymm0, ymm1, ymm2",
        "vpaddb ymm0, ymm1, ymm2",
        "vpaddw ymm0, ymm1, ymm2",
        "vpaddd ymm0, ymm1, ymm2",
        "vpaddq ymm0, ymm1, ymm2",
        "vpsubb ymm0, ymm1, ymm2",
        "vpsubd ymm0, ymm1, ymm2",
        // Les saturations : là où l'arithmétique s'arrête au bord au
        // lieu de repasser par zéro.
        "vpaddsb ymm0, ymm1, ymm2",
        "vpaddusb ymm0, ymm1, ymm2",
        "vpsubsw ymm0, ymm1, ymm2",
        "vpminub ymm0, ymm1, ymm2",
        "vpmaxub ymm0, ymm1, ymm2",
        "vpminsd ymm0, ymm1, ymm2",
        "vpcmpeqb ymm0, ymm1, ymm2",
        "vpcmpgtd ymm0, ymm1, ymm2",
        "vpmullw ymm0, ymm1, ymm2",
        "vpmuludq ymm0, ymm1, ymm2",
        "vpsllw ymm0, ymm1, 3",
        "vpsrld ymm0, ymm1, 5",
        "vpsraw ymm0, ymm1, 2",
        "vpshufb ymm0, ymm1, ymm2",
        "vpunpcklbw ymm0, ymm1, ymm2",
        "vpunpckhdq ymm0, ymm1, ymm2",
        "vperm2i128 ymm0, ymm1, ymm2, 0x21",
        "vpblendw ymm0, ymm1, ymm2, 0xa5",
        "vpavgb ymm0, ymm1, ymm2",
        "vpsadbw ymm0, ymm1, ymm2",
        "vpabsd ymm0, ymm1",
        "vpmovmskb eax, ymm1",
        "vptest ymm1, ymm2",
        // Les bits, en général : BMI1 et BMI2 sont de la même époque.
        "andn rax, rbx, rcx",
        "blsi rax, rbx",
        "blsr rax, rbx",
        "bzhi rax, rbx, rcx",
        "pext rax, rbx, rcx",
        "pdep rax, rbx, rcx",
        "shlx rax, rbx, rcx",
        "sarx rax, rbx, rcx",
        "mulx rax, rbx, rcx",
    ]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut graines = 24usize;
    let mut instructions: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--graines" => {
                i += 1;
                graines = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(24);
            }
            autre => instructions.push(autre.to_owned()),
        }
        i += 1;
    }
    if instructions.is_empty() {
        instructions = répertoire()
            .into_iter()
            .chain(répertoire_vectoriel())
            .map(str::to_owned)
            .collect();
    }

    let dossier = match tempdir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("impossible de préparer un dossier de travail : {e}");
            std::process::exit(2);
        }
    };

    println!(
        "== oracle : {} instruction(s) × {graines} graine(s)\n",
        instructions.len()
    );

    let mut éprouvées = 0usize;
    let mut cas = 0usize;
    let mut fautives: Vec<String> = Vec::new();
    let mut écartées: Vec<String> = Vec::new();

    for instruction in &instructions {
        let octets = match octets_de(&dossier, instruction) {
            Ok(o) => o,
            Err(message) => {
                écartées.push(format!("{instruction} — {message}"));
                continue;
            }
        };

        let mut graine = 0x2545_f491_4f6c_dd1du64;
        let mut écarts_vus: Vec<String> = Vec::new();
        let mut refusée: Option<String> = None;

        for _ in 0..graines {
            let semis = Graine::tirée(&mut graine);
            let asm = programme(&semis, instruction);
            let programme_bâti = match bâtir(&dossier, &asm) {
                Ok(p) => p,
                Err(message) => {
                    refusée = Some(message);
                    break;
                }
            };
            let matériel = match sous_le_matériel(&programme_bâti) {
                Ok(é) => é,
                Err(message) => {
                    refusée = Some(message);
                    break;
                }
            };
            // `rsp` tel que le matériel l'a laissé : le programme
            // d'épreuve l'a posé lui-même, et namsdec doit partir de là.
            let nôtre = match sous_namsdec(&octets, &semis, matériel.gpr[4]) {
                Ok(é) => é,
                Err(message) => {
                    refusée = Some(message);
                    break;
                }
            };
            cas += 1;

            let écarts = matériel.écarts(&nôtre, indéfinis(instruction));
            if !écarts.is_empty() && écarts_vus.len() < 3 {
                écarts_vus.push(format!(
                    "    graine rax={:#x} rbx={:#x} rcx={:#x} flags={:#x}\n      {}",
                    semis.gpr[0],
                    semis.gpr[3],
                    semis.gpr[1],
                    semis.flags,
                    écarts.join("\n      ")
                ));
            }
        }

        if let Some(message) = refusée {
            écartées.push(format!("{instruction} — {message}"));
            continue;
        }
        éprouvées += 1;
        if écarts_vus.is_empty() {
            println!("  ok    {instruction}");
        } else {
            println!("  ÉCART {instruction}");
            for ligne in &écarts_vus {
                println!("{ligne}");
            }
            fautives.push(instruction.clone());
        }
    }

    println!(
        "\n== {éprouvées} instruction(s) éprouvée(s), {cas} cas, {} écart(s)",
        fautives.len()
    );
    if !écartées.is_empty() {
        println!(
            "\n== {} écartée(s), faute de pouvoir les poser :",
            écartées.len()
        );
        for ligne in &écartées {
            println!("  {ligne}");
        }
    }
    if !fautives.is_empty() {
        println!("\nà regarder : {}", fautives.join(", "));
        std::process::exit(1);
    }
}

/// Un dossier de travail à soi, sans dépendance.
///
/// On ne l'efface pas : quand l'oracle trouve un écart, le programme
/// d'épreuve qui l'a produit est la première chose qu'on veut relire.
fn tempdir() -> std::io::Result<PathBuf> {
    let base = std::env::temp_dir().join(format!("namsdec-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&base)?;
    Ok(base)
}
