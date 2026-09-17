//! Ce que namsdec sait dire d'un binaire, en texte.
//!
//! Outil de développement : il montre ce que les modules d'analyse
//! trouvent, sans passer par l'interface. C'est avec lui qu'on mesure ce
//! qui manque encore devant un binaire réel.
//!
//! `cargo run -p namsdec-re --example explore -- /chemin/du/binaire`

fn main() {
    let Some(chemin) = std::env::args().nth(1) else {
        eprintln!("usage : explore <binaire>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&chemin).expect("le fichier se lit");
    let elf = namsdec_re::elf_scan::scan(&bytes).expect("un ELF64");

    println!("== {chemin} : {} octets", bytes.len());
    println!("point d'entrée : {:#x}", elf.entry);
    println!("\n== sections exécutables");
    for section in elf.sections.iter().filter(|s| s.execute && !s.nobits) {
        println!(
            "  {:<16} {:#010x}  {:>8} octets",
            section.name, section.addr, section.size
        );
    }

    println!("\n== fonctions empruntées (PLT)");
    let imports = namsdec_re::imports::scan(&bytes);
    println!("  {} entrées", imports.len());
    for import in imports.iter().take(20) {
        println!("  {:#010x}  {}", import.plt, import.name);
    }

    // Le désassemblage des sections exécutables, comme le fait la GUI.
    let mut disasm = Vec::new();
    for section in elf.sections.iter().filter(|s| s.execute && !s.nobits) {
        let début = section.offset as usize;
        let fin = (début + section.size as usize).min(bytes.len());
        if fin > début {
            disasm.extend(namsdec_disasm::disassemble(
                &bytes[début..fin],
                section.addr,
            ));
        }
    }
    println!(
        "\n== fonctions retrouvées ({} instructions décodées)",
        disasm.len()
    );
    let fonctions = namsdec_re::functions::discover(&disasm, elf.entry);
    println!("  {} fonctions", fonctions.len());
    match namsdec_re::functions::main_of(&disasm, elf.entry) {
        Some(main) => println!("  main : {main:#x}"),
        None => println!("  main : introuvable"),
    }
    for f in namsdec_re::functions::by_importance(&fonctions)
        .into_iter()
        .take(12)
    {
        println!(
            "  {:<12} {:#010x}  {:>6} octets  {:>3} appelants  {:>3} appels",
            f.name(),
            f.start,
            f.size(),
            f.callers,
            f.calls.len()
        );
    }

    println!("\n== chaînes (sections de données)");
    let chaînes = namsdec_re::strings::scan_elf(&bytes).unwrap_or_default();
    println!("  {} trouvées", chaînes.len());
    for trouvée in chaînes
        .iter()
        .filter(|f| {
            let bas = f.text.to_lowercase();
            ["password", "correct", "incorrect", "enter", "flag"]
                .iter()
                .any(|mot| bas.contains(mot))
        })
        .take(20)
    {
        println!("  {:#010x}  {:?}", trouvée.addr, trouvée.text);
    }

    println!("\n== les chaînes telles que le code les emploie");
    let littérales = namsdec_re::literals::scan(&disasm, &elf.sections, &bytes);
    println!("  {} chaînes découpées par le code", littérales.len());
    for l in littérales
        .iter()
        .filter(|l| {
            let bas = l.text.to_lowercase();
            ["password", "correct", "enter", "error", "flag"]
                .iter()
                .any(|m| bas.contains(m))
        })
        .take(12)
    {
        println!(
            "  {:#010x} ({:>3}) {:?}   employée en {:#010x}",
            l.addr, l.len, l.text, l.from
        );
    }

    // Qui désigne ces chaînes ? C'est la question d'un crackme.
    println!("\n== qui charge l'adresse de ces chaînes ?");
    let plage = elf
        .sections
        .iter()
        .filter(|s| !s.nobits)
        .fold(u64::MAX..0, |acc, s| {
            acc.start.min(s.addr)..acc.end.max(s.addr + s.size)
        });
    let xrefs = namsdec_re::xrefs::scan(&disasm, plage);
    for cible in [0x4afcu64, 0x5f6b, 0x5fa6] {
        let refs = xrefs.to(cible);
        println!("  {cible:#010x} : {} référence(s)", refs.len());
        for r in refs.iter().take(4) {
            let dans = fonctions
                .iter()
                .find(|f| r.from >= f.start && r.from <= f.last)
                .map(|f| f.name())
                .unwrap_or_else(|| "?".into());
            println!("      depuis {:#010x} ({dans}) {:?}", r.from, r.kind);
        }
    }
}
