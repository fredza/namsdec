//! Wrapper autour de la chaîne d'outils externe : `nasm` assemble, `ld`
//! lie, et on relit l'ELF produit pour savoir quoi charger en mémoire et
//! où. En sortent aussi les diagnostics (erreurs/avertissements reliés aux
//! lignes source) et la table ligne source -> adresse.
//!
//! Le format est `elf64`, et pas le `bin` plat des débuts, parce que le
//! listing de nasm compte ses décalages **à partir du début de chaque
//! section**. Sur une source à plusieurs sections — le cas de tout
//! programme Linux ordinaire — la première ligne de `.data` et la première
//! de `.text` recevaient donc la même adresse, et la gouttière comme les
//! points d'arrêt désignaient la mauvaise ligne. L'ELF lié donne l'adresse
//! réelle de chaque section, ce qui recale ces décalages ; il donne aussi
//! le point d'entrée, au lieu de supposer que le programme commence au
//! premier octet.

pub mod elf;

use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;
use thiserror::Error;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub line: u32,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AssembleOutput {
    /// L'image mémoire du programme lié : les sections chargeables mises
    /// bout à bout, trous compris.
    pub bytes: Vec<u8>,
    /// Adresse à laquelle `bytes` doit être chargé.
    pub load_addr: u64,
    /// Point d'entrée déclaré par l'ELF — la valeur de RIP au départ.
    pub entry: u64,
    /// Les sections chargeables, pour que l'appelant sache lesquelles sont
    /// du code (à désassembler) et lesquelles sont des données (à ne
    /// surtout pas décoder comme des instructions).
    pub sections: Vec<Section>,
    pub diagnostics: Vec<Diagnostic>,
    /// ligne source (1-indexée) -> adresse absolue de la première
    /// instruction qu'elle génère (adresse de sa section + décalage lu
    /// dans le listing nasm).
    pub line_to_addr: BTreeMap<u32, u64>,
}

/// Une section chargeable, telle que l'appelant en a besoin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub addr: u64,
    pub size: usize,
    pub executable: bool,
    /// La section s'écrit-elle ? `.data` et `.bss` oui, `.text` et
    /// `.rodata` non.
    pub writable: bool,
}

#[derive(Debug, Clone)]
pub struct NasmInfo {
    pub version: String,
}

/// `Display` en anglais, variantes structurées : la traduction se fait
/// dans `namsdec-gui` (voir `namsdec_core::MemError`).
#[derive(Debug, Error)]
pub enum AsmError {
    #[error("nasm was not found in PATH")]
    NasmNotFound,
    #[error("ld was not found in PATH")]
    LinkerNotFound,
    #[error("assembly failed")]
    Failed { diagnostics: Vec<Diagnostic> },
    /// L'édition de liens a échoué — typiquement un symbole indéfini, que
    /// nasm ne pouvait pas voir seul.
    #[error("linking failed: {message}")]
    LinkFailed { message: String },
    /// `ld` a produit quelque chose que le lecteur ELF ne comprend pas.
    #[error("the linked program could not be read back")]
    UnreadableProgram,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Détecte la présence de `nasm` dans le PATH et sa version. Sondé une
/// fois, retenu ensuite — même raison que `find_linker`, dont la
/// documentation détaille la contrepartie.
///
/// La copie rendue à chaque appel est une petite chaîne de version, sans
/// commune mesure avec le `nasm -v` qu'elle remplace.
pub fn find_nasm() -> Option<NasmInfo> {
    static NASM: LazyLock<Option<NasmInfo>> = LazyLock::new(probe_nasm);
    NASM.clone()
}

fn probe_nasm() -> Option<NasmInfo> {
    let output = Command::new("nasm").arg("-v").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.lines().next().unwrap_or("nasm").trim().to_string();
    Some(NasmInfo { version })
}

/// Détecte la présence de `ld` dans le PATH. Sondé une fois, retenu
/// ensuite.
///
/// Le sondage lance un processus, et `assemble` le refaisait à chaque
/// appel : un `ld --version` par Assembler, soit plusieurs par minute
/// pour une réponse qui ne change pas — une chaîne d'outils n'apparaît ni
/// ne disparaît en cours de session.
///
/// Contrepartie assumée : `ld` installé pendant que namsdec tourne n'est
/// vu qu'au lancement suivant. C'est déjà le cas de nasm, dont l'interface
/// retient au démarrage la version trouvée.
pub fn find_linker() -> bool {
    static LINKER: LazyLock<bool> = LazyLock::new(probe_linker);
    *LINKER
}

fn probe_linker() -> bool {
    Command::new("ld")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

/// Assemble `source`, le lie, et rend de quoi le charger en mémoire.
///
/// `base_addr` est imposé à `ld` comme adresse de `.text` : la fenêtre
/// mémoire simulée est fixe, le programme doit atterrir dedans plutôt
/// qu'à l'adresse que l'éditeur de liens aurait choisie de son côté.
pub fn assemble(source: &str, base_addr: u64) -> Result<AssembleOutput, AsmError> {
    find_nasm().ok_or(AsmError::NasmNotFound)?;
    if !find_linker() {
        return Err(AsmError::LinkerNotFound);
    }

    let dir = tempfile::tempdir()?;
    let asm_path = dir.path().join("input.asm");
    let listing_path = dir.path().join("listing.lst");
    let program_path = dir.path().join("program");
    fs::write(&asm_path, source)?;

    let output = Command::new("nasm")
        .current_dir(dir.path())
        .args([
            "-f",
            "elf64",
            "-l",
            "listing.lst",
            "-o",
            "input.o",
            "input.asm",
        ])
        .output()?;

    let mut diagnostics = parse_diagnostics(&String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(AsmError::Failed { diagnostics });
    }

    let linked = Command::new("ld")
        .current_dir(dir.path())
        .args([
            "-Ttext".to_owned(),
            format!("{base_addr:#x}"),
            "-o".to_owned(),
            "program".to_owned(),
            "input.o".to_owned(),
        ])
        .output()?;
    if !linked.status.success() {
        return Err(AsmError::LinkFailed {
            message: String::from_utf8_lossy(&linked.stderr).trim().to_owned(),
        });
    }
    // L'éditeur de liens peut réussir *en avertissant* — un programme
    // sans `_start`, par exemple, pour lequel il choisit une entrée par
    // défaut. Ces avertissements-là ont longtemps été jetés ici, et le
    // programme démarrait en silence à un endroit que personne n'avait
    // choisi. Ils rejoignent maintenant ceux de nasm, sans numéro de
    // ligne : ils parlent du programme entier, pas d'une ligne.
    diagnostics.extend(
        String::from_utf8_lossy(&linked.stderr)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| Diagnostic {
                line: 0,
                severity: Severity::Warning,
                message: line.to_owned(),
            }),
    );

    let elf = elf::parse(&fs::read(&program_path)?).ok_or(AsmError::UnreadableProgram)?;
    let (load_addr, bytes) = elf.image().ok_or(AsmError::UnreadableProgram)?;

    let listing = fs::read_to_string(&listing_path).unwrap_or_default();
    let line_to_addr = parse_listing(&listing, &elf);

    Ok(AssembleOutput {
        bytes,
        load_addr,
        entry: elf.entry,
        sections: elf
            .sections
            .iter()
            .map(|s| Section {
                name: s.name.clone(),
                addr: s.addr,
                size: s.size,
                executable: s.executable,
                writable: s.writable,
            })
            .collect(),
        diagnostics,
        line_to_addr,
    })
}

static DIAGNOSTIC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[^:]+:(?P<line>\d+): (?P<sev>error|warning)(?:\s*\[[^\]]*\])?: (?P<msg>.*)$")
        .unwrap()
});

fn parse_diagnostics(stderr: &str) -> Vec<Diagnostic> {
    stderr
        .lines()
        .filter_map(|line| {
            let caps = DIAGNOSTIC_RE.captures(line)?;
            let severity = if &caps["sev"] == "error" {
                Severity::Error
            } else {
                Severity::Warning
            };
            Some(Diagnostic {
                line: caps["line"].parse().ok()?,
                severity,
                message: caps["msg"].to_string(),
            })
        })
        .collect()
}

static LISTING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?P<line>\d+)\s+(?:(?P<addr>[0-9A-Fa-f]{8})\s+[0-9A-Fa-f]+\s*)?(?P<text>.*)$")
        .unwrap()
});

/// Repère une directive `section .xxx` (ou `segment .xxx`) dans le texte
/// d'une ligne du listing.
static SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(?:section|segment)\s+(?P<name>[.\w$]+)").unwrap());

/// Relie chaque ligne source à une adresse absolue.
///
/// Les décalages du listing repartent de zéro **à chaque section** : lus
/// tels quels, la première ligne de `.data` et la première de `.text`
/// tombaient sur la même adresse. On suit donc les directives `section`
/// au fil du listing pour savoir dans laquelle on se trouve, et on ajoute
/// l'adresse que l'éditeur de liens a donnée à cette section.
///
/// Une ligne d'une section dont l'ELF ne parle pas (une section vide, que
/// `ld` a éliminée) est laissée de côté plutôt que rattachée au hasard.
fn parse_listing(listing: &str, elf: &elf::Elf) -> BTreeMap<u32, u64> {
    let mut map = BTreeMap::new();
    // nasm démarre dans `.text` tant qu'aucune directive n'a été vue.
    let mut section_addr = elf.section_addr(".text");

    for line in listing.lines() {
        let Some(caps) = LISTING_RE.captures(line) else {
            continue;
        };
        if let Some(section) = caps
            .name("text")
            .and_then(|t| SECTION_RE.captures(t.as_str()))
        {
            section_addr = elf.section_addr(&section["name"]);
            continue;
        }

        let (Some(addr_hex), Some(base)) = (caps.name("addr"), section_addr) else {
            continue;
        };
        let Ok(line_no) = caps["line"].parse::<u32>() else {
            continue;
        };
        let Ok(offset) = u64::from_str_radix(addr_hex.as_str(), 16) else {
            continue;
        };
        // Une ligne qui s'étale sur plusieurs lignes de listing (une
        // longue chaîne `db`) n'y garde que son premier décalage.
        map.entry(line_no).or_insert(base + offset);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un programme sans `_start` se lie quand même : `ld` avertit et
    /// choisit une entrée. L'avertissement doit remonter, sans quoi le
    /// programme démarre à un endroit que personne n'a choisi.
    #[test]
    fn a_linker_warning_becomes_a_diagnostic() {
        if find_nasm().is_none() || !find_linker() {
            return;
        }
        let sans_start = "section .text\n    mov rax, 1\n    hlt\n";
        let out = assemble(sans_start, 0x400000).expect("le lien réussit malgré tout");
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.line == 0 && d.severity == Severity::Warning),
            "obtenu {:?}",
            out.diagnostics
        );

        // Le même programme complet ne dit plus rien.
        let complet = "global _start\nsection .text\n_start:\n    mov rax, 1\n    hlt\n";
        let out = assemble(complet, 0x400000).expect("assemblage");
        assert!(out.diagnostics.is_empty(), "obtenu {:?}", out.diagnostics);
    }

    /// Un programme complet : il déclare son point d'entrée, comme tout
    /// ce que namsdec livre — sans quoi l'éditeur de liens avertit.
    const SUM_LOOP_ASM: &str = "\
global _start

section .text
_start:
    mov rax, 0
    mov rcx, 5
boucle:
    add rax, rcx
    dec rcx
    jnz boucle
    hlt
";

    /// Un programme à plusieurs sections, comme en écrit tout le monde
    /// sous Linux.
    const HELLO_ASM: &str = "\
section .data
    message db \"salut\", 10
    longueur equ $ - message

section .text
    global _start
_start:
    mov rax, 1
    mov rsi, message
    mov rdx, longueur
    hlt
";

    #[test]
    fn assembles_sum_loop_and_reports_line_addresses() {
        let out = assemble(SUM_LOOP_ASM, 0x400000).expect("nasm et ld doivent être installés");
        assert!(out.diagnostics.is_empty(), "obtenu {:?}", out.diagnostics);
        // mov rax,0 / mov rcx,5 (5 octets chacun, nasm optimise en mov e*x)
        // add rax,rcx / dec rcx (3 octets chacun) / jnz (2) / hlt (1).
        assert_eq!(out.line_to_addr[&5], 0x400000);
        assert_eq!(out.line_to_addr[&8], 0x40000A); // la première ligne de la boucle
        assert_eq!(out.line_to_addr[&11], 0x400012); // hlt
        assert_eq!(out.entry, 0x400000, "`_start` est au début du code");
    }

    /// La régression qui rendait le pas à pas inexploitable : les
    /// décalages du listing repartant de zéro à chaque section, la
    /// première ligne de `.data` et la première de `.text` recevaient la
    /// même adresse, et la gouttière montrait la mauvaise ligne.
    #[test]
    fn lines_from_different_sections_get_different_addresses() {
        let out = assemble(HELLO_ASM, 0x400000).expect("nasm et ld doivent être installés");

        let data_line = out.line_to_addr[&2]; // message db "salut", 10
        let first_code_line = out.line_to_addr[&8]; // mov rax, 1

        assert_ne!(
            data_line, first_code_line,
            "une ligne de données et une ligne de code ne peuvent pas être à la même adresse"
        );
        assert_eq!(
            first_code_line, 0x400000,
            "`.text` est placée à l'adresse demandée"
        );
        assert_eq!(
            out.entry, first_code_line,
            "l'exécution démarre à `_start`, pas au premier octet chargé"
        );

        // Et la ligne de données tombe bien dans `.data`.
        let data = out
            .sections
            .iter()
            .find(|s| s.name == ".data")
            .expect("le programme a une section .data");
        assert!(
            (data.addr..data.addr + data.size as u64).contains(&data_line),
            "{data_line:#x} devrait être dans .data ({:#x}..)",
            data.addr
        );
        assert!(
            !data.executable,
            ".data ne doit pas être marquée exécutable"
        );
    }

    /// L'appelant doit pouvoir distinguer le code des données : décoder
    /// une chaîne de caractères comme des instructions ne produit que du
    /// bruit, et c'est ce qui envoyait l'exécution dans le décor.
    #[test]
    fn code_and_data_sections_are_told_apart() {
        let out = assemble(HELLO_ASM, 0x400000).unwrap();
        let text = out.sections.iter().find(|s| s.name == ".text").unwrap();
        assert!(text.executable);
        assert!(
            out.sections
                .iter()
                .any(|s| s.name == ".data" && !s.executable)
        );
    }

    /// Le format `elf64` est du 64 bits par construction : une source qui
    /// ne dit pas `BITS 64` n'a plus à échouer sur « instruction not
    /// supported in 16-bit mode », comme c'était le cas avec `-f bin`.
    #[test]
    fn a_source_without_a_bits_directive_assembles_as_64_bit() {
        let out = assemble(HELLO_ASM, 0x400000).expect("doit s'assembler sans `BITS 64` explicite");
        assert!(
            out.diagnostics.is_empty(),
            "aucun diagnostic attendu, obtenu : {:?}",
            out.diagnostics
        );
        // `mov rsi, message` sort en REX.W (0x48) suivi d'un immédiat 64
        // bits, une forme qui n'existe pas en 16 bits.
        assert!(
            out.bytes.windows(2).any(|w| w == [0x48, 0xbe]),
            "attendu un `mov rsi, imm64` en REX.W"
        );
    }

    /// Un symbole qui n'existe pas n'est vu ni par nasm ni par le listing :
    /// c'est l'éditeur de liens qui le refuse, et son échec doit remonter
    /// comme tel plutôt que de passer pour un programme valide.
    #[test]
    fn an_undefined_symbol_is_reported_as_a_link_failure() {
        // `extern` promet à nasm que le symbole existera ailleurs : il
        // laisse donc passer, et c'est ld qui découvre qu'il n'existe
        // nulle part.
        let err = assemble(
            "extern fonction_inexistante\nsection .text\n    call fonction_inexistante\n",
            0x400000,
        )
        .expect_err("l'édition de liens doit échouer");
        match err {
            AsmError::LinkFailed { message } => {
                assert!(
                    !message.is_empty(),
                    "l'échec doit porter la raison donnée par ld"
                )
            }
            other => panic!("attendu LinkFailed, obtenu {other:?}"),
        }
    }

    /// Même chose pour `nasm`, à la version rendue près : le cache doit
    /// dire ce que dirait le sondage, et ne le lancer qu'une fois.
    #[test]
    fn nasm_is_probed_once_and_then_remembered() {
        let found = find_nasm().map(|info| info.version);
        assert_eq!(
            found,
            probe_nasm().map(|info| info.version),
            "le cache doit dire la vérité"
        );

        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            assert_eq!(find_nasm().map(|info| info.version), found);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "mille appels ont pris {elapsed:?} — le processus est relancé à chaque fois"
        );
    }

    /// Le sondage de `ld` ne doit avoir lieu qu'une fois : c'est tout
    /// l'intérêt du cache, et mille appels ne peuvent pas lancer mille
    /// processus sans que la durée le dise.
    #[test]
    fn the_linker_is_probed_once_and_then_remembered() {
        // Le premier appel sonde pour de bon, les suivants non.
        let found = find_linker();
        assert_eq!(found, probe_linker(), "le cache doit dire la vérité");

        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            assert_eq!(find_linker(), found);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "mille appels ont pris {elapsed:?} — le processus est relancé à chaque fois"
        );
    }

    #[test]
    fn reports_error_diagnostic_with_line_number() {
        let src = "section .text
    mov rax, notaregister
";
        let err = assemble(src, 0x400000).expect_err("devrait échouer");
        match err {
            AsmError::Failed { diagnostics } => {
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].line, 2);
                assert_eq!(diagnostics[0].severity, Severity::Error);
            }
            other => panic!("attendu AsmError::Failed, obtenu {other:?}"),
        }
    }

    #[test]
    fn parses_diagnostic_line_format() {
        let diags = parse_diagnostics("bad.asm:3: error: symbol `notaregister' not defined\n");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 3);
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(diags[0].message, "symbol `notaregister' not defined");
    }

    /// Le décodage du listing seul, sans passer par nasm : chaque section
    /// repart de zéro, et c'est l'adresse donnée par l'ELF qui la recale.
    #[test]
    fn listing_offsets_are_rebased_per_section() {
        let elf = elf::Elf {
            entry: 0x400000,
            sections: vec![
                elf::Section {
                    name: ".text".to_owned(),
                    addr: 0x400000,
                    data: Vec::new(),
                    size: 0x20,
                    executable: true,
                    writable: false,
                },
                elf::Section {
                    name: ".data".to_owned(),
                    addr: 0x401000,
                    data: Vec::new(),
                    size: 0x10,
                    executable: false,
                    writable: true,
                },
            ],
        };
        let listing = "\
     1                                  section .data
     2 00000000 73616C75740A                message db \"salut\", 10
     3                                  section .text
     4 00000000 B800000000                  mov rax, 0
     5 0000000A 4801C8                      add rax, rcx
";
        let map = parse_listing(listing, &elf);

        assert_eq!(map[&2], 0x401000, "la ligne de données suit .data");
        assert_eq!(map[&4], 0x400000, "la ligne de code suit .text");
        assert_eq!(map[&5], 0x40000A);
        assert!(
            !map.contains_key(&1),
            "une directive seule n'a pas d'adresse"
        );
    }

    /// Le lecteur ELF sur un fichier qui n'en est pas un : il doit
    /// renoncer proprement, pas paniquer sur un index hors bornes.
    #[test]
    fn the_elf_reader_rejects_anything_else() {
        assert!(elf::parse(b"").is_none());
        assert!(elf::parse(b"#!/bin/sh\necho salut\n").is_none());
        // Bon nombre magique, mais tronqué juste après.
        assert!(elf::parse(b"\x7fELF\x02\x01").is_none());
    }
}
