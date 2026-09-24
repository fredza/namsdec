//! Optional real-process tests. Set NAMSDEC_DZF_BRIDGE_MANIFEST to require DZF.
use namsdec_disasm::plugin;
use std::{path::PathBuf, process::Command};

#[test]
fn dzf_bridge_preserves_partial_diagnostics_and_translates_elf() {
    let Some(manifest) = std::env::var_os("NAMSDEC_DZF_BRIDGE_MANIFEST") else {
        eprintln!("DZF bridge not configured; set NAMSDEC_DZF_BRIDGE_MANIFEST");
        return;
    };
    let tool = plugin::read(&PathBuf::from(manifest)).expect("valid DZF bridge manifest");
    assert_eq!(tool.id, "dzf-decompile");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("sample.asm");
    let object = dir.path().join("sample.o");
    for (body, status) in [
        ("xor eax,eax\nor eax,1\nand eax,3\nret", "TRANSLATED_REGIONS"),
        ("nop\nud2", "PARTIAL_OR_FAILED"),
    ] {
        std::fs::write(&source, format!("bits 64\nsection .text\nglobal _start\n_start:\n{body}\n")).unwrap();
        assert!(Command::new("nasm").args(["-f", "elf64"]).arg(&source).arg("-o").arg(&object).status().unwrap().success());
        let result = tool.run(&std::fs::read(&object).unwrap()).expect("bridge process");
        assert!(result.contains(&format!("Status: {status}")), "{result}");
        if status == "TRANSLATED_REGIONS" {
            assert!(result.contains("flags_logic32"), "{result}");
        } else {
            assert!(result.contains("DZF diagnostics"), "{result}");
        }
    }
    let refused = tool.run(b"MZ not ELF").unwrap();
    assert!(refused.contains("Status: REFUSED"), "{refused}");
}
