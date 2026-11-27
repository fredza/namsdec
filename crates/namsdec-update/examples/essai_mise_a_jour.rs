//! Éprouve la mise à jour **pour de vrai**, sans toucher au programme
//! installé.
//!
//! Les contrôles de ce crate vérifient chaque morceau séparément : une
//! somme se lit, un ELF se reconnaît, un fichier se remplace, un
//! dossier en lecture seule se dit. Ce qu'ils ne peuvent pas faire,
//! c'est aller chercher la vraie publication sur GitHub — un test qui
//! dépend du réseau échoue le jour où la CI n'en a pas, et apprend
//! alors quelque chose sur le réseau plutôt que sur le programme.
//!
//! D'où cet exemple, qui se lance à la main :
//!
//! ```text
//! cargo run -p namsdec-update --example essai_mise_a_jour
//! ```
//!
//! Il fait l'enchaînement complet — chercher la dernière version, lire
//! sa somme, télécharger le binaire, le vérifier, le mettre en place —
//! mais **dans un dossier temporaire**, sur un leurre qu'il crée
//! lui-même. Le namsdec qui tourne n'est jamais touché, et le dossier
//! disparaît à la fin.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use namsdec_update::install;

    let préversions = std::env::args().any(|a| a == "--preversions" || a == "--prereleases");
    println!(
        "Recherche de la dernière version publiée{}…",
        if préversions {
            " (préversions comprises)"
        } else {
            ""
        }
    );
    let release =
        namsdec_update::fetch_latest(namsdec_update::OWNER, namsdec_update::REPO, préversions)?;
    println!(
        "  {} — {} fichier(s) attaché(s){}",
        release.tag,
        release.assets.len(),
        if release.prerelease {
            ", préversion"
        } else {
            ""
        }
    );

    let binaire = install::binary_asset(&release).ok_or_else(|| {
        format!(
            "aucun fichier nommé {}",
            install::asset_name(&release.version)
        )
    })?;
    let somme =
        install::checksum_asset(&release).ok_or("aucune somme publiée à côté du binaire")?;
    println!(
        "  binaire : {} ({:.1} Mo)",
        binaire.name,
        binaire.size as f64 / 1_000_000.0
    );

    let attendue = install::fetch_checksum(&somme.url)?;
    println!("  somme publiée : {}", namsdec_core::digest::hex(&attendue));

    // LE LEURRE : un faux programme, dans un dossier qui s'efface. Tout
    // ce qui suit se passe là, et nulle part ailleurs.
    let dossier = tempfile::tempdir()?;
    let cible = dossier.path().join("namsdec");
    std::fs::write(&cible, b"ceci n'est pas namsdec")?;
    install::replaceable(&cible)?;

    let neuf = install::staging_path(&cible);
    let début = std::time::Instant::now();
    let mut dernier = 0u64;
    install::download(&binaire.url, attendue, &neuf, &mut |avance| {
        // Une ligne par mégaoctet : de quoi voir que ça avance sans
        // noyer le terminal.
        if avance.reçus - dernier >= 4_000_000 {
            dernier = avance.reçus;
            println!(
                "  reçus {:.1} Mo{}",
                avance.reçus as f64 / 1_000_000.0,
                match avance.total {
                    Some(total) => format!(" sur {:.1}", total as f64 / 1_000_000.0),
                    None => String::new(),
                }
            );
        }
        true
    })?;
    println!(
        "  téléchargé et vérifié en {:.1} s",
        début.elapsed().as_secs_f64()
    );

    let sauvegarde = install::install_over(&neuf, &cible)?;
    let posé = std::fs::read(&cible)?;
    println!(
        "  mis en place : {} octets, somme {}",
        posé.len(),
        namsdec_core::digest::hex(&namsdec_core::digest::sha256(&posé))
    );
    println!(
        "  ancien gardé : {} ({:?})",
        sauvegarde.display(),
        String::from_utf8_lossy(&std::fs::read(&sauvegarde)?)
    );
    assert_eq!(
        namsdec_core::digest::sha256(&posé),
        attendue,
        "ce qui est en place doit être ce qui était annoncé"
    );
    assert!(
        install::looks_like_elf(&posé),
        "et ce doit être un programme"
    );
    println!("\nL'enchaînement complet tient, et rien hors du dossier temporaire n'a bougé.");
    Ok(())
}
