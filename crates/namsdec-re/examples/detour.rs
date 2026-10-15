//! Poser un détour sur un binaire, hors de l'interface.
//!
//! Outil de développement, et preuve de bout en bout : il choisit la
//! place libre, planifie le détour, l'applique et écrit le fichier. Ce
//! qu'on vérifie ensuite n'est pas du ressort de namsdec — on lance le
//! binaire corrigé, et on le relit avec un désassembleur tiers.
//!
//! `cargo run -p namsdec-re --example detour -- <binaire> <adresse> <octets-hex> <sortie>`
//!
//! Les octets de l'ajout sont donnés tout assemblés, en hexadécimal :
//! cet exemple ne dépend pas d'un assembleur, et la règle vaut pour le
//! module lui-même.

use namsdec_re::{imports, patch, slack};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [chemin, adresse, ajout_hex, sortie] = match args.as_slice() {
        [a, b, c, d] => [a, b, c, d],
        _ => {
            eprintln!("usage : detour <binaire> <adresse> <octets-hex> <sortie>");
            std::process::exit(2);
        }
    };

    let bytes = std::fs::read(chemin).expect("le fichier se lit");
    let site_vaddr = u64::from_str_radix(adresse.trim_start_matches("0x"), 16)
        .expect("une adresse en hexadécimal");
    let ajout = décoder_hex(ajout_hex);

    let site_offset =
        imports::file_offset(&bytes, site_vaddr).expect("cette adresse est dans un segment chargé");

    // La place libre : la première exécutable qui soit assez grande.
    // `slack::find` trie par taille, donc la première qui passe est la
    // plus grande — et c'est bien ce qu'on veut, pour laisser de la
    // marge à un second détour.
    let trous = slack::find(&bytes);
    println!("== {} trous de bourrage trouvés", trous.len());
    for trou in trous.iter().take(5) {
        println!(
            "  {:#010x}  {:>6} octets  remplissage {:#04x}  {}  {}",
            trou.vaddr,
            trou.len,
            trou.filler,
            if trou.executable {
                "exécutable"
            } else {
                "données   "
            },
            trou.section.as_deref().unwrap_or("(sans section)")
        );
    }

    let mut posé = false;
    for trou in trous.iter().filter(|t| t.executable) {
        match patch::plan_detour(&bytes, 64, site_offset as usize, site_vaddr, trou, &ajout) {
            Ok(detour) => {
                println!(
                    "\n== détour depuis {:#x} vers {:#x}",
                    site_vaddr, trou.vaddr
                );
                println!(
                    "  recouvert : {} octets  →  {}",
                    detour.recouvert,
                    en_hex(&detour.déplacé)
                );
                println!("  aller     : {}", en_hex(&detour.aller.after));
                println!("  corps     : {}", en_hex(&detour.corps.after));

                let mut image = bytes.clone();
                patch::apply(&mut image, detour.aller.offset, &detour.aller.after)
                    .expect("l'aller s'applique");
                patch::apply(&mut image, detour.corps.offset, &detour.corps.after)
                    .expect("le corps s'applique");
                std::fs::write(sortie, &image).expect("le fichier s'écrit");
                println!("\nécrit : {sortie}");
                posé = true;
                break;
            }
            Err(refus) => {
                // Un refus qui porte sur le SITE ne changera pas d'une
                // place libre à l'autre : le répéter pour chacune
                // noierait la seule ligne utile.
                let dépend_du_site = matches!(
                    refus,
                    patch::DetourRefusé::CodeDéplacéRelatif { .. }
                        | patch::DetourRefusé::SiteIndécodable { .. }
                );
                println!("  {}", refus.message(true));
                if dépend_du_site {
                    break;
                }
            }
        }
    }

    if !posé {
        eprintln!("\naucune place libre ne convient");
        std::process::exit(1);
    }
}

fn décoder_hex(texte: &str) -> Vec<u8> {
    let propre: String = texte.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    propre
        .as_bytes()
        .chunks(2)
        .map(|paire| {
            u8::from_str_radix(std::str::from_utf8(paire).unwrap(), 16).expect("hexadécimal")
        })
        .collect()
}

fn en_hex(octets: &[u8]) -> String {
    octets
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
