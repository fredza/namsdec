//! Ce que les binaires d'épreuve empruntent, et que namsdec ne fournit pas.
//!
//! La question « quelles fonctions ajouter ? » se répond par un compte,
//! non par une intuition : on relève les imports de tout un corpus, on
//! retire ce qui est déjà fourni, et l'on classe le reste par le nombre
//! de binaires qui le réclament. Un symbole demandé par un seul fichier
//! ne vaut pas celui que huit réclament.
fn main() {
    // `provides` ET NON `PROVIDED` : la liste brute ignore que
    // `canonical` ramène `__isoc23_scanf` à `scanf` et `__snprintf_chk`
    // à `snprintf`, et que les enveloppes d'appels système — `mmap`,
    // `mprotect` — passent par une autre porte. Comparée à la liste
    // seule, la mesure annonçait onze fonctions manquantes qui sont
    // fournies. Une seule source de vérité, et c'est celle que
    // l'exécution emploie.
    // Combien de BINAIRES réclament chaque symbole, et lesquels.
    let mut compte: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    let mut lus = 0usize;
    // Les deux corpus se recouvrent : `tests/binaires/` et la collection
    // personnelle portent les mêmes fichiers. Compter deux fois le même
    // octet gonflerait le classement sans rien apprendre.
    let mut déjà_vus: std::collections::BTreeSet<[u8; 32]> = Default::default();
    for chemin in std::env::args().skip(1) {
        let Ok(octets) = std::fs::read(&chemin) else {
            continue;
        };
        if !déjà_vus.insert(namsdec_re::digest::sha256(&octets)) {
            continue;
        }
        let nom = std::path::Path::new(&chemin)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or(chemin.clone());
        let imports = namsdec_re::imports::scan(&octets);
        if imports.is_empty() {
            continue;
        }
        lus += 1;
        let mut vus = std::collections::BTreeSet::new();
        for import in imports {
            // Le nom nettoyé de ce que l'édition de liens y ajoute.
            let propre = import
                .name
                .split('@')
                .next()
                .unwrap_or(&import.name)
                .to_owned();
            if propre.is_empty() || namsdec_emu::libc::provides(&propre) {
                continue;
            }
            if vus.insert(propre.clone()) {
                compte.entry(propre).or_default().push(nom.clone());
            }
        }
    }
    let mut classé: Vec<(String, Vec<String>)> = compte.into_iter().collect();
    classé.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    println!("{lus} binaires liés dynamiquement, distincts\n");
    // Ce qu'on pourrait fournir, et ce qu'on ne fournira pas : un
    // symbole décoré en C++ ou une fonction de Qt, de SDL, d'OpenGL
    // n'est pas une fonction de bibliothèque C — l'écrire en Rust
    // reviendrait à réimplémenter une bibliothèque entière, ce que
    // namsdec ne fera pas. Les séparer évite de lire un classement où
    // les deux se mêlent.
    let tierce = |nom: &str| {
        nom.starts_with("_Z")
            || nom.starts_with("SDL_")
            || nom.starts_with("IMG_")
            || nom.starts_with("gl")
            || nom.starts_with("Qt")
            || nom.starts_with("qt_")
            || nom.starts_with("__gxx")
            || nom.starts_with("__cxa_t")
    };
    println!("=== CE QU'ON POURRAIT FOURNIR ===");
    for (nom, qui) in classé.iter().filter(|(n, _)| !tierce(n)) {
        println!("{:3}  {nom}   ({})", qui.len(), qui.join(", "));
    }
    let autres = classé.iter().filter(|(n, _)| tierce(n)).count();
    println!("\n=== et {autres} symboles de bibliothèques tierces (C++, Qt, SDL, OpenGL) ===");
}
