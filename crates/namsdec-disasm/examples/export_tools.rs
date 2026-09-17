//! Écrit les descriptions des outils connus dans `plugins/fonctionnalites/`.
//!
//! Les fichiers du dépôt sont ainsi *engendrés* par le code qui les
//! connaît : impossible qu'un argument diverge entre ce que namsdec
//! lance et ce que le dépôt propose de copier à la main.
//!
//! `cargo run -p namsdec-disasm --example export_tools`

fn main() -> std::io::Result<()> {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins");
    let dossier = racine.join("fonctionnalites");
    std::fs::create_dir_all(&dossier)?;
    for outil in namsdec_disasm::known::all() {
        let chemin = dossier.join(format!("{}.json", outil.plugin.id));
        let json = serde_json::to_string_pretty(&outil.plugin)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(&chemin, format!("{json}\n"))?;
        println!("écrit : {}", chemin.display());
    }
    Ok(())
}
