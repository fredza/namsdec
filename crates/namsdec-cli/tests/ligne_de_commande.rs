//! Ce que la commande promet, éprouvé sur le vrai programme.
//!
//! Chaque contrôle lance le binaire construit — pas une fonction
//! interne : la moitié de ce qu'une commande promet est dans sa lecture
//! d'arguments et ses codes de retour, et aucun appel de bibliothèque
//! ne les regarde.
//!
//! **Et chacun distingue les deux états qu'il nomme.** Un contrôle qui
//! vérifie « le fichier vide échoue » sans vérifier qu'un vrai fichier
//! réussit ne regarde rien : il passerait aussi si la commande
//! échouait toujours.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Le binaire tel que cargo vient de le construire.
fn commande() -> Command {
    Command::new(env!("CARGO_BIN_EXE_namsdec-cli"))
}

/// Un vrai binaire du dépôt — pas un fichier de la machine.
fn binaire_d_épreuve() -> Option<PathBuf> {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/binaires/crackme-01");
    chemin.exists().then_some(chemin)
}

fn sortie(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn plainte(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

/// Le rapport dit tout ce que les panneaux disent, prose comprise.
#[test]
fn le_rapport_markdown_porte_la_prose_et_les_panneaux() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let out = commande().arg(&binaire).output().expect("la commande part");
    assert_eq!(code(&out), 0, "{}", plainte(&out));
    let md = sortie(&out);

    // La prose d'ouverture — ce qu'on lit en premier.
    assert!(
        md.starts_with("# Analyse du binaire — crackme-01"),
        "{}",
        &md[..80.min(md.len())]
    );
    // Et les rubriques, qui sont le gros de l'analyse.
    assert!(
        md.contains("## Ce que chaque panneau montre — crackme-01")
            || md.contains("# Ce que chaque panneau montre — crackme-01")
    );
    for attendu in [
        "## En bref",
        "## Protections du binaire",
        "**NX**",
        "Profil d'entropie",
    ] {
        assert!(md.contains(attendu), "le rapport porte « {attendu} »");
    }

    // AUCUN OUTIL EXTÉRIEUR N'A TOURNÉ, et le rapport ne parle donc pas
    // de la machine qui l'a produit : « radare2 n'est pas installé ici »
    // n'apprend rien à qui lira ce rapport ailleurs.
    for outil in ["radare2", "Ghidra", "RetDec"] {
        assert!(!md.contains(outil), "{outil} n'a rien à dire de ce fichier");
    }
}

/// Le même fond, lisible par un autre programme — et deux fois le même.
///
/// **La reproductibilité est la promesse, pas un détail** : tout
/// l'intérêt de la sortie JSON est de comparer deux analyses, et une
/// sortie qui bougerait d'un appel à l'autre ferait du diff un bruit.
#[test]
fn le_json_est_le_même_à_chaque_appel() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let lire = || {
        let out = commande()
            .args(["-f", "json"])
            .arg(&binaire)
            .output()
            .expect("la commande part");
        assert_eq!(code(&out), 0, "{}", plainte(&out));
        sortie(&out)
    };
    let premier = lire();
    assert_eq!(
        premier,
        lire(),
        "deux analyses du même fichier se ressemblent"
    );

    let relu: serde_json::Value = serde_json::from_str(&premier).expect("du JSON qui se relit");
    let panneaux = relu.as_array().expect("une liste de panneaux");
    assert!(panneaux.len() >= 8, "{} panneaux", panneaux.len());
    assert!(
        panneaux
            .iter()
            .any(|p| p["id"] == "protections" && p["vue"]["blocs"].is_array()),
        "les protections y sont, avec leur vue"
    );
}

/// Ce que la commande existe pour rendre possible : voir ce qui a changé
/// entre deux versions d'un binaire.
///
/// Un `diff` d'octets dirait « le fichier a changé à l'offset 0x15604 »,
/// ce qui n'apprend rien. Un diff d'ANALYSES dit « cette chaîne-là a
/// changé », et c'est ce qu'on cherchait.
///
/// **Ce que la première écriture de ce contrôle a appris, et qui valait
/// la peine :** j'avais AJOUTÉ la chaîne à la fin du fichier, et
/// l'analyse ne la voyait pas — à juste titre. Le panneau des chaînes
/// lit les SECTIONS que l'en-tête déclare, pas le fichier entier :
/// chercher ailleurs ne rapporterait que du code qui ressemble à du
/// texte. Des octets collés après la dernière section n'appartiennent à
/// rien, et un outil qui les lirait mentirait sur leur place. On modifie
/// donc une chaîne EN PLACE, à longueur égale — ce qu'un correctif fait
/// vraiment.
#[test]
fn deux_versions_se_comparent_par_leur_analyse() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let mut octets = std::fs::read(&binaire).expect("le binaire se lit");
    // Une chaîne que le fichier porte vraiment, dans une section
    // déclarée. Absente, le contrôle ne prouverait rien : on s'abstient
    // plutôt que de passer pour la mauvaise raison.
    let Some(place) = octets
        .windows(AVANT.len())
        .position(|fenêtre| fenêtre == AVANT)
    else {
        return;
    };
    octets[place..place + AVANT.len()].copy_from_slice(APRÈS);

    let atelier = tempfile::tempdir().expect("un dossier temporaire");
    let v2 = atelier.path().join("crackme-01.v2");
    std::fs::write(&v2, &octets).expect("la seconde version s'écrit");

    let analyse = |chemin: &Path| {
        let out = commande()
            .args(["-f", "json", "-s", "strings"])
            .arg(chemin)
            .output()
            .expect("la commande part");
        assert_eq!(code(&out), 0, "{}", plainte(&out));
        sortie(&out)
    };
    let avant = analyse(&binaire);
    let après = analyse(&v2);

    assert_ne!(avant, après, "l'analyse voit la différence");
    let lisible = |texte: &[u8]| String::from_utf8_lossy(texte).into_owned();
    assert!(
        après.contains(&lisible(APRÈS)) && !après.contains(&lisible(AVANT)),
        "la chaîne modifiée se lit dans la seconde analyse"
    );
    assert!(
        avant.contains(&lisible(AVANT)) && !avant.contains(&lisible(APRÈS)),
        "et l'ancienne dans la première — sans quoi le contrôle ne prouverait rien"
    );

    // ET LE RESTE NE BOUGE PAS. C'est la moitié utile d'un diff
    // d'analyses : si chaque rapport différait partout, celui-ci ne
    // montrerait pas où regarder.
    let empreintes = |chemin: &Path| {
        let out = commande()
            .args(["-f", "json", "-s", "protections"])
            .arg(chemin)
            .output()
            .expect("la commande part");
        sortie(&out)
    };
    assert_eq!(
        empreintes(&binaire),
        empreintes(&v2),
        "ce qui n'a pas changé se lit pareil"
    );
}

/// La chaîne qu'on modifie d'une version à l'autre, à longueur égale.
const AVANT: &[u8] = b"/dev/null";
const APRÈS: &[u8] = b"/dev/nulX";

/// Un fichier vide ne passe pas pour un fichier analysé.
///
/// C'est le défaut que la mesure a trouvé, et il valait le détour :
/// l'analyse d'un fichier de zéro octet rend dix-huit rubriques qui
/// disent toutes « 0 octet », « aucun fichier chargé ». Avec un titre,
/// des tableaux et des sections, **ce rapport a l'air d'une analyse** —
/// et en CI, un artefact tronqué passait donc pour analysé.
#[test]
fn le_fichier_vide_s_arrête_sur_son_propre_code() {
    let atelier = tempfile::tempdir().expect("un dossier temporaire");
    let vide = atelier.path().join("vide.bin");
    std::fs::write(&vide, b"").expect("le fichier vide s'écrit");

    let out = commande().arg(&vide).output().expect("la commande part");
    assert_eq!(code(&out), 3, "le fichier vide a son code à lui");
    assert!(sortie(&out).is_empty(), "et il n'écrit pas de rapport");
    assert!(plainte(&out).contains("vide"), "{}", plainte(&out));

    // LE TÉMOIN CONTRAIRE, sans quoi ce contrôle passerait aussi si la
    // commande échouait sur tout.
    if let Some(binaire) = binaire_d_épreuve() {
        let out = commande().arg(&binaire).output().expect("la commande part");
        assert_eq!(code(&out), 0, "un vrai binaire, lui, s'analyse");
    }
}

/// Un fichier qui ne se lit pas le dit, avec son nom et la cause.
#[test]
fn un_fichier_absent_a_son_code_et_sa_cause() {
    let atelier = tempfile::tempdir().expect("un dossier temporaire");
    let absent = atelier.path().join("jamais-écrit.bin");
    let out = commande().arg(&absent).output().expect("la commande part");
    assert_eq!(code(&out), 2);
    let plainte = plainte(&out);
    assert!(plainte.contains("jamais-écrit.bin"), "{plainte}");
    assert!(
        plainte.contains("No such file") || plainte.contains("os error 2"),
        "{plainte}"
    );
}

/// Une rubrique mal nommée se plaint, au lieu de rendre un rapport vide.
///
/// Sans cela, `--rubrique protection` (au singulier) rendait un rapport
/// de zéro panneau sans un mot, et l'on cherchait le défaut dans le
/// binaire.
#[test]
fn une_rubrique_inconnue_se_plaint() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let out = commande()
        .args(["-s", "protection"])
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 1);
    assert!(sortie(&out).is_empty(), "et rien n'est écrit");
    assert!(plainte(&out).contains("--rubriques"), "{}", plainte(&out));

    // Au pluriel, c'est le bon nom, et il rend une rubrique et une seule.
    let out = commande()
        .args(["-f", "json", "-s", "protections"])
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 0, "{}", plainte(&out));
    let relu: serde_json::Value = serde_json::from_str(&sortie(&out)).expect("du JSON");
    let panneaux = relu.as_array().expect("une liste");
    assert_eq!(panneaux.len(), 1, "une rubrique demandée, une rendue");
    assert_eq!(panneaux[0]["id"], "protections");
}

/// Les rubriques se listent avant de se choisir, avec leur identifiant.
#[test]
fn les_rubriques_se_listent_avec_leur_identifiant() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let out = commande()
        .arg("--rubriques")
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 0, "{}", plainte(&out));
    let liste = sortie(&out);

    // UN IDENTIFIANT ET UN TITRE PAR LIGNE, séparés par une tabulation :
    // c'est ce qui rend la liste lisible par `cut -f1`.
    let lignes: Vec<&str> = liste.lines().collect();
    assert!(lignes.len() >= 8, "{} rubriques", lignes.len());
    for ligne in &lignes {
        let (id, titre) = ligne
            .split_once('\t')
            .unwrap_or_else(|| panic!("« {ligne} » tient en deux champs"));
        assert!(!id.is_empty() && !titre.is_empty(), "« {ligne} »");
        assert!(
            !id.contains(' '),
            "un identifiant s'écrit sans blanc : « {id} »"
        );
    }
    // Et ces identifiants sont ceux que `--rubrique` accepte.
    let premier = lignes[0].split('\t').next().unwrap();
    let out = commande()
        .args(["-s", premier])
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(
        code(&out),
        0,
        "« {premier} » se redonne tel quel : {}",
        plainte(&out)
    );

    // ET `--format json` VAUT ICI AUSSI : il serait ignoré en silence
    // autrement, et une option qui ne fait rien là où on la pose est
    // pire qu'une option absente.
    let out = commande()
        .args(["--rubriques", "-f", "json"])
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 0, "{}", plainte(&out));
    let relu: serde_json::Value = serde_json::from_str(&sortie(&out)).expect("du JSON");
    let liste = relu.as_array().expect("une liste");
    assert_eq!(
        liste.len(),
        lignes.len(),
        "les mêmes rubriques, autrement dites"
    );
    assert_eq!(liste[0]["id"], premier);
}

/// L'anglais fait la même chose, et le dit en anglais.
#[test]
fn les_noms_anglais_mènent_au_même_endroit() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let fr = commande()
        .args(["--langue", "fr", "--rubriques"])
        .arg(&binaire)
        .output()
        .expect("la commande part");
    let en = commande()
        .args(["--lang", "en", "--sections"])
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&fr), 0);
    assert_eq!(code(&en), 0);

    // Les mêmes rubriques, dans le même ordre, sous d'autres titres.
    let ids = |texte: &str| -> Vec<String> {
        texte
            .lines()
            .filter_map(|l| l.split('\t').next().map(str::to_owned))
            .collect()
    };
    assert_eq!(
        ids(&sortie(&fr)),
        ids(&sortie(&en)),
        "les identifiants ne se traduisent pas"
    );
    assert!(sortie(&fr).contains("En bref"), "{}", sortie(&fr));
    assert!(sortie(&en).contains("At a glance"), "{}", sortie(&en));

    // Et l'aide suit la langue demandée.
    let aide = commande()
        .args(["-l", "en", "--help"])
        .output()
        .expect("la commande part");
    assert!(sortie(&aide).contains("USAGE"), "{}", sortie(&aide));
    let aide = commande()
        .args(["-l", "fr", "--aide"])
        .output()
        .expect("la commande part");
    assert!(sortie(&aide).contains("EMPLOI"), "{}", sortie(&aide));
}

/// Le rapport s'écrit où on le demande.
#[test]
fn la_sortie_va_dans_le_fichier_demandé() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let atelier = tempfile::tempdir().expect("un dossier temporaire");
    let rapport = atelier.path().join("rapport.md");
    let out = commande()
        .arg("-o")
        .arg(&rapport)
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 0, "{}", plainte(&out));
    assert!(sortie(&out).is_empty(), "rien sur la sortie standard");
    let écrit = std::fs::read_to_string(&rapport).expect("le rapport est là");
    assert!(
        écrit.contains("## Protections du binaire"),
        "et il est entier"
    );

    // Un dossier qui n'existe pas se dit, plutôt que de se perdre.
    let out = commande()
        .arg("-o")
        .arg(atelier.path().join("nulle-part/rapport.md"))
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 1);
    assert!(
        plainte(&out).contains("ne s'écrit pas"),
        "{}",
        plainte(&out)
    );
}

/// Un tuyau qui se ferme ne fait pas paniquer la commande.
///
/// `namsdec-cli programme | head -1` ferme la sortie au premier saut de
/// ligne. Avec `print!`, le programme se terminait sur « failed printing
/// to stdout » — un rapport de panique Rust au bout d'un pipe
/// parfaitement normal.
#[test]
fn un_tuyau_fermé_ne_fait_pas_paniquer() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{} {} | head -1",
            shell_quote(env!("CARGO_BIN_EXE_namsdec-cli")),
            shell_quote(&binaire.to_string_lossy())
        ))
        .output()
        .expect("sh part");
    assert_eq!(code(&out), 0, "{}", plainte(&out));
    assert!(
        sortie(&out).contains("Analyse du binaire"),
        "la première ligne passe"
    );
    assert!(
        !plainte(&out).contains("panicked"),
        "aucune panique : {}",
        plainte(&out)
    );
}

fn shell_quote(texte: &str) -> String {
    format!("'{}'", texte.replace('\'', r"'\''"))
}

/// Deux fichiers sur la ligne, et l'on s'arrête.
///
/// Le second écrasait le premier en silence : `namsdec-cli a.bin b.bin`
/// analysait b.bin en disant « a.bin » nulle part, et le titre du
/// rapport était le seul indice.
#[test]
fn deux_fichiers_ne_passent_pas() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let out = commande()
        .arg(&binaire)
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 1);
    assert!(
        plainte(&out).contains("un seul fichier"),
        "{}",
        plainte(&out)
    );
}
