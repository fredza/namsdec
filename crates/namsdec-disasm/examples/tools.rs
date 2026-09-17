//! Ce que namsdec dit des outils d'analyse sur cette machine.
//!
//! `cargo run -p namsdec-disasm --example tools`

fn main() {
    for outil in namsdec_disasm::known::all() {
        let dispo = outil.availability();
        let chemin = namsdec_disasm::known::program_path(&outil.plugin.command);
        println!(
            "{:<40} {:<12} {}",
            outil.plugin.name,
            format!("{dispo:?}"),
            chemin.map(|p| p.display().to_string()).unwrap_or_else(|| {
                outil
                    .plugin
                    .install
                    .clone()
                    .unwrap_or_else(|| "—".to_owned())
            })
        );
    }
}
