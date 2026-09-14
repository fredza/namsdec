//! Wrapper autour du binaire `nasm` externe : assemblage, diagnostics
//! (erreurs/avertissements reliés aux lignes source) et table
//! ligne source -> adresse, issue du listing `nasm -l`.

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
    pub bytes: Vec<u8>,
    pub diagnostics: Vec<Diagnostic>,
    /// ligne source (1-indexée) -> adresse absolue de la première
    /// instruction qu'elle génère (base_addr + décalage du listing nasm).
    pub line_to_addr: BTreeMap<u32, u64>,
}

#[derive(Debug, Clone)]
pub struct NasmInfo {
    pub version: String,
}

#[derive(Debug, Error)]
pub enum AsmError {
    #[error("nasm est introuvable dans le PATH — installez-le (ex : `sudo dnf install nasm`)")]
    NasmNotFound,
    #[error("échec de l'assemblage")]
    Failed { diagnostics: Vec<Diagnostic> },
    #[error("erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),
}

/// Détecte la présence de `nasm` dans le PATH et sa version.
pub fn find_nasm() -> Option<NasmInfo> {
    let output = Command::new("nasm").arg("-v").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.lines().next().unwrap_or("nasm").trim().to_string();
    Some(NasmInfo { version })
}

/// Assemble `source` (format `nasm -f bin`) et charge le résultat comme
/// s'il devait être placé à `base_addr` en mémoire.
pub fn assemble(source: &str, base_addr: u64) -> Result<AssembleOutput, AsmError> {
    find_nasm().ok_or(AsmError::NasmNotFound)?;

    let dir = tempfile::tempdir()?;
    let asm_path = dir.path().join("input.asm");
    let bin_path = dir.path().join("out.bin");
    let listing_path = dir.path().join("listing.lst");
    fs::write(&asm_path, source)?;

    let output = Command::new("nasm")
        .current_dir(dir.path())
        .args([
            "-f",
            "bin",
            "-l",
            "listing.lst",
            "-o",
            "out.bin",
            "input.asm",
        ])
        .output()?;

    let diagnostics = parse_diagnostics(&String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        return Err(AsmError::Failed { diagnostics });
    }

    let bytes = fs::read(&bin_path)?;
    let listing = fs::read_to_string(&listing_path).unwrap_or_default();
    let line_to_addr = parse_listing(&listing, base_addr);

    Ok(AssembleOutput {
        bytes,
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
    Regex::new(r"^\s*(?P<line>\d+)\s+(?:(?P<addr>[0-9A-Fa-f]{8})\s+[0-9A-Fa-f]+\s*)?").unwrap()
});

fn parse_listing(listing: &str, base_addr: u64) -> BTreeMap<u32, u64> {
    let mut map = BTreeMap::new();
    for line in listing.lines() {
        let Some(caps) = LISTING_RE.captures(line) else {
            continue;
        };
        let Some(addr_hex) = caps.name("addr") else {
            continue;
        };
        let Ok(line_no) = caps["line"].parse::<u32>() else {
            continue;
        };
        let Ok(offset) = u64::from_str_radix(addr_hex.as_str(), 16) else {
            continue;
        };
        map.insert(line_no, base_addr + offset);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUM_LOOP_ASM: &str = "\
BITS 64
org 0x400000

    mov rax, 0
    mov rcx, 5
sum_loop:
    add rax, rcx
    dec rcx
    jnz sum_loop
    hlt
";

    #[test]
    fn assembles_sum_loop_and_reports_line_addresses() {
        let out = assemble(SUM_LOOP_ASM, 0x400000).expect("nasm doit être installé pour ce test");
        assert!(out.diagnostics.is_empty());
        // mov rax,0 / mov rcx,5 (5 octets chacun, nasm optimise en mov e*x)
        // add rax,rcx / dec rcx (3 octets chacun) / jnz (2) / hlt (1) = 19 octets
        assert_eq!(out.bytes.len(), 19);
        assert_eq!(out.line_to_addr[&4], 0x400000);
        assert_eq!(out.line_to_addr[&7], 0x40000A); // sum_loop:
        assert_eq!(out.line_to_addr[&10], 0x400012); // hlt
    }

    #[test]
    fn reports_error_diagnostic_with_line_number() {
        let src = "BITS 64\norg 0x400000\n    mov rax, notaregister\n";
        let err = assemble(src, 0x400000).expect_err("devrait échouer");
        match err {
            AsmError::Failed { diagnostics } => {
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].line, 3);
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

    #[test]
    fn parses_listing_addresses() {
        let listing = "\
     1                                  BITS 64
     2                                  org 0x400000
     3
     4 00000000 B800000000                  mov rax, 0
     6                                  sum_loop:
     7 0000000A 4801C8                      add rax, rcx
";
        let map = parse_listing(listing, 0x400000);
        assert_eq!(map[&4], 0x400000);
        assert_eq!(map[&7], 0x40000A);
        assert!(!map.contains_key(&6)); // ligne d'étiquette seule : pas d'adresse propre
    }
}
