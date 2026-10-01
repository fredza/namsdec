//! L'anti-désassemblage, de bout en bout : un vrai ELF, le panneau, le
//! signal et le rapport.
//!
//! Les contrôles d'unité éprouvent la détection sur quelques octets ;
//! celui-ci éprouve **la chaîne entière** sur un fichier qui ressemble à
//! un fichier — en-tête, sections, et la figure posée dans `.text`. Sans
//! cela, rien ne dirait que le filtre qui écarte le bruit de `.rodata`
//! laisse passer ce qu'on cherche quand on le lui donne vraiment.

mod commun;

use commun::{BASE, germe_piégé};
use namsdec_core::{Cpu, Memory};
use namsdec_re::{ReContext, ReModule};

/// Le listing part du premier octet du fichier, comme dans
/// l'application.
fn lecture(binaire: &[u8]) -> Vec<namsdec_disasm::DecodedInstruction> {
    namsdec_disasm::disassemble(binaire, BASE)
}

/// La figure est vue dans un vrai ELF, et les deux lectures sont là.
#[test]
fn the_figure_is_seen_inside_a_real_elf() {
    let binaire = germe_piégé();
    let disasm = lecture(&binaire);
    let zones = namsdec_re::misalign::code_zones(&binaire, BASE);
    assert!(!zones.is_empty(), "le germe a des sections exécutables");

    let trouvés = namsdec_re::misalign::scan_in(&disasm, &zones);
    assert_eq!(trouvés.len(), 1, "{trouvés:?}");
    let m = &trouvés[0];
    assert_eq!(m.from, BASE + 0x100, "le saut, en tête de .text");
    assert_eq!(m.target, BASE + 0x103, "un octet après le piège");
    assert_eq!(m.offset, 1);
    assert!(
        m.linear.starts_with("call"),
        "le listing ment : {}",
        m.linear
    );
    // Quatre instructions montrées : les trois du piège, puis le
    // bourrage qui suit. Le module en montre un nombre fixe — il n'a pas
    // à deviner où le vrai code s'arrête.
    assert_eq!(
        m.flow.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        vec!["xor eax,eax", "inc rax", "ret", "nop"],
        "ce que le processeur lit"
    );
}

/// Le panneau « Figures du code » montre les deux lectures.
#[test]
fn the_shapes_panel_shows_both_readings() {
    let binaire = germe_piégé();
    let disasm = lecture(&binaire);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));

    for (lang, attendus) in [
        (
            "fr",
            [
                "Le listing est trompé",
                "le listing lit",
                "le processeur lit",
            ],
        ),
        (
            "en",
            [
                "The listing is being fooled",
                "the listing reads",
                "the processor reads",
            ],
        ),
    ] {
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &binaire,
            lang,
        };
        let mut module = namsdec_re::PatternsModule::default();
        module.on_binary_loaded(&ctx);
        assert_eq!(module.misalignments().len(), 1);

        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 1400.0),
            )),
            ..Default::default()
        };
        let mut textes: Vec<String> = Vec::new();
        for _ in 0..2 {
            textes.clear();
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, &ctx));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    textes.push(t.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        let tout = textes.join(" | ");
        for mot in attendus {
            assert!(tout.contains(mot), "{lang} : « {mot} » manque — {tout}");
        }
        // Et les deux lectures y sont, l'une et l'autre.
        assert!(tout.contains("xor eax,eax"), "{lang} : {tout}");
        assert!(
            textes.iter().any(|t| t.starts_with("call")),
            "{lang} : la lecture trompeuse aussi — {tout}"
        );
    }
}

/// Le signal le dit, et le rapport le porte avec les deux lectures.
#[test]
fn the_signal_and_the_report_both_carry_it() {
    let binaire = germe_piégé();
    let disasm = lecture(&binaire);

    let signaux = namsdec_re::signals::gather(&binaire, &disasm, "fr");
    let ligne = signaux
        .iter()
        .find(|s| s.fr == "Le listing est trompé")
        .unwrap_or_else(|| panic!("{signaux:?}"));
    assert_eq!(ligne.panel("fr"), "Figures du code");

    let scan = namsdec_re::elf_scan::scan(&binaire).expect("le germe est un ELF");
    let rapport = namsdec_re::report::markdown(&scan, &binaire, &disasm, "fr", "piège");
    assert!(rapport.contains("## Le listing est trompé"), "{rapport}");
    assert!(rapport.contains("le listing lit"), "{rapport}");
    assert!(
        rapport.contains("xor eax,eax ; inc rax ; ret"),
        "la vraie lecture, d'un bloc :\n{rapport}"
    );
    // Et en anglais, autre chose.
    let anglais = namsdec_re::report::markdown(&scan, &binaire, &disasm, "en", "trap");
    assert!(
        anglais.contains("## The listing is being fooled"),
        "{anglais}"
    );
}
