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

/// La seconde version du binaire d'épreuve : une chaîne changée EN
/// PLACE, à longueur égale — ce qu'un correctif fait vraiment.
///
/// `None` quand la chaîne n'y est pas : on s'abstient plutôt que de
/// passer pour la mauvaise raison.
fn seconde_version(dans: &Path) -> Option<PathBuf> {
    let binaire = binaire_d_épreuve()?;
    let mut octets = std::fs::read(&binaire).expect("le binaire se lit");
    let place = octets
        .windows(AVANT.len())
        .position(|fenêtre| fenêtre == AVANT)?;
    octets[place..place + AVANT.len()].copy_from_slice(APRÈS);
    let chemin = dans.join("crackme-01.v2");
    std::fs::write(&chemin, &octets).expect("la seconde version s'écrit");
    Some(chemin)
}

/// `--contre` dit ce qui a changé dans l'ANALYSE, rubrique par rubrique.
///
/// **C'est ce que la commande existait pour rendre possible, et qu'elle
/// ne faisait pas encore** : on s'en sortait par un `diff` sur deux
/// sorties JSON, ce qui marche et se lit mal — on y voit la forme du
/// rapport (`"texte":`, des accolades) plutôt que le fond.
#[test]
fn comparer_dit_ce_qui_a_changé_et_ce_qui_n_a_pas_changé() {
    let atelier = tempfile::tempdir().expect("un dossier temporaire");
    let (Some(v1), Some(v2)) = (binaire_d_épreuve(), seconde_version(atelier.path())) else {
        return;
    };

    let out = commande()
        .arg(&v2)
        .arg("--contre")
        .arg(&v1)
        .output()
        .expect("la commande part");
    // LE CODE DE RETOUR PORTE LA RÉPONSE, comme `diff` : en CI, on n'a
    // pas à relire le rapport pour savoir que quelque chose a bougé.
    assert_eq!(code(&out), 4, "{}", plainte(&out));
    let md = sortie(&out);

    // Les deux noms en tête, et dans le bon sens.
    assert!(
        md.starts_with("# Ce qui a changé — crackme-01 → crackme-01.v2"),
        "{}",
        &md[..90.min(md.len())]
    );
    // La chaîne changée, des deux côtés.
    assert!(
        md.contains(&format!("- | 00015604 | {} |", lisible(AVANT)))
            && md.contains(&format!("+ | 00015604 | {} |", lisible(APRÈS))),
        "la ligne qui a changé se dit des deux côtés : {md}"
    );
    // ET CE QUI N'A PAS CHANGÉ EST COMPTÉ, ce qui est la moitié utile
    // du rapport : « quinze rubriques disent la même chose » est ce qui
    // permet de croire aux quatre qui ont changé.
    assert!(
        md.contains("disent la même chose"),
        "le rapport dit combien de rubriques se taisent : {md}"
    );
    // Les protections n'ont pas bougé : elles ne sont donc pas listées.
    assert!(
        !md.contains("## Protections du binaire"),
        "une rubrique identique n'a rien à dire ici : {md}"
    );
}

/// Deux fois le même fichier : rien n'a changé, et le code le dit.
///
/// **Le témoin contraire du contrôle précédent.** Sans lui, celui-ci
/// passerait aussi si la commande annonçait des changements à tout
/// propos — ce qui est le défaut le plus probable d'un comparateur.
#[test]
fn comparer_un_fichier_avec_lui_même_ne_trouve_rien() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let out = commande()
        .arg(&binaire)
        .arg("-c")
        .arg(&binaire)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 0, "rien n'a changé : {}", plainte(&out));
    let md = sortie(&out);
    assert!(
        md.contains("pas une qui diffère"),
        "et le rapport le dit en toutes lettres : {md}"
    );
    // L'accord en nombre est tenu : « 1 rubrique(s) ont changé » est la
    // marque d'une phrase qu'aucun humain n'a relue.
    assert!(!md.contains("(s)"), "{md}");
}

/// `--rubrique` restreint la comparaison, et c'est là qu'elle sert le
/// plus.
///
/// « Une protection a-t-elle disparu ? » est la question qu'on pose en
/// CI. Les empreintes, elles, changent à chaque compilation : sans
/// restriction, le rapport dirait toujours que quelque chose a bougé, et
/// le code de retour ne vaudrait plus rien.
#[test]
fn comparer_une_seule_rubrique_répond_à_la_question_de_la_ci() {
    let atelier = tempfile::tempdir().expect("un dossier temporaire");
    let (Some(v1), Some(v2)) = (binaire_d_épreuve(), seconde_version(atelier.path())) else {
        return;
    };

    // Les protections sont les mêmes : le code dit 0, et la chaîne
    // d'intégration continue.
    let out = commande()
        .args(["-s", "protections"])
        .arg(&v2)
        .arg("-c")
        .arg(&v1)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 0, "{}", sortie(&out));

    // Les chaînes, elles, ont changé : le même appel sur une autre
    // rubrique rend 4. C'est la paire qui prouve que le code de retour
    // regarde vraiment la rubrique demandée.
    let out = commande()
        .args(["-s", "strings"])
        .arg(&v2)
        .arg("-c")
        .arg(&v1)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 4, "{}", plainte(&out));
    assert!(sortie(&out).contains(&lisible(APRÈS)), "{}", sortie(&out));
}

/// Comparer avec un fichier qui ne se lit pas, ou qui est vide.
///
/// Comparer une analyse à rien rendrait « tout a disparu » — vrai, et
/// trompeur : le défaut est que l'autre fichier est vide, et c'est cela
/// qu'il faut dire.
#[test]
fn comparer_avec_rien_se_plaint_au_lieu_de_tout_dire_disparu() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    let atelier = tempfile::tempdir().expect("un dossier temporaire");

    let absent = atelier.path().join("jamais-écrit.bin");
    let out = commande()
        .arg(&binaire)
        .arg("-c")
        .arg(&absent)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 2);
    assert!(
        plainte(&out).contains("jamais-écrit.bin"),
        "{}",
        plainte(&out)
    );

    let vide = atelier.path().join("vide.bin");
    std::fs::write(&vide, b"").expect("le fichier vide s'écrit");
    let out = commande()
        .arg(&binaire)
        .arg("-c")
        .arg(&vide)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 3);
    assert!(sortie(&out).is_empty(), "et rien n'est écrit");
    assert!(
        plainte(&out).contains("rien à comparer"),
        "{}",
        plainte(&out)
    );
}

/// La comparaison se relit par un autre programme.
#[test]
fn la_comparaison_se_rend_aussi_en_json() {
    let atelier = tempfile::tempdir().expect("un dossier temporaire");
    let (Some(v1), Some(v2)) = (binaire_d_épreuve(), seconde_version(atelier.path())) else {
        return;
    };
    let out = commande()
        .args(["-f", "json"])
        .arg(&v2)
        .arg("-c")
        .arg(&v1)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 4, "{}", plainte(&out));
    let relue: serde_json::Value = serde_json::from_str(&sortie(&out)).expect("du JSON");
    assert!(
        relue["identiques"].as_u64().is_some_and(|n| n >= 8),
        "ce qui ne change pas est compté : {relue}"
    );
    let rubriques = relue["rubriques"].as_array().expect("une liste");
    assert!(
        rubriques
            .iter()
            .any(|r| r["id"] == "strings" && r["état"] == "Changée"),
        "et la rubrique qui a changé est nommée : {relue}"
    );
}

/// Une option qui ne servirait à rien se dit, au lieu d'être ignorée.
///
/// `--rubriques --contre` comparait en IGNORANT `--rubriques` : l'option
/// était là, elle ne faisait rien, et rien ne le disait. C'est le même
/// défaut qu'un format ignoré en silence — et il se répète partout où
/// deux options se rencontrent sans se contredire franchement.
#[test]
fn une_option_qui_ne_sert_à_rien_avec_contre_se_dit() {
    let Some(binaire) = binaire_d_épreuve() else {
        return;
    };
    for inutile in ["--rubriques", "--sans-prose"] {
        let out = commande()
            .arg(inutile)
            .arg(&binaire)
            .arg("-c")
            .arg(&binaire)
            .output()
            .expect("la commande part");
        assert_eq!(code(&out), 1, "{inutile} : {}", sortie(&out));
        assert!(
            plainte(&out).contains(inutile) && plainte(&out).contains("--contre"),
            "la plainte nomme les deux : {}",
            plainte(&out)
        );
    }

    // Et séparément, chacune marche : sans ce témoin, le contrôle
    // passerait aussi si la commande refusait ces options partout.
    for seule in ["--rubriques", "--sans-prose"] {
        let out = commande()
            .arg(seule)
            .arg(&binaire)
            .output()
            .expect("la commande part");
        assert_eq!(code(&out), 0, "{seule} : {}", plainte(&out));
    }
}

/// Le texte d'une suite d'octets, pour le comparer à ce qui est écrit.
fn lisible(octets: &[u8]) -> String {
    String::from_utf8_lossy(octets).into_owned()
}

/// **Une protection qui disparaît se voit** — le cas pour lequel toute
/// cette commande existe.
///
/// On rend la pile exécutable dans la seconde version : un seul bit du
/// segment `PT_GNU_STACK`, ce qu'un éditeur hexadécimal fait en dix
/// secondes et qu'un `diff` d'octets rapporterait comme « un octet a
/// changé à l'offset 0x…c4 ». L'analyse, elle, en tire trois phrases —
/// NX absent, un segment W+X, et un signal qui dit ce que cela change.
#[test]
fn une_protection_qui_disparaît_se_voit() {
    let Some(v1) = binaire_d_épreuve() else {
        return;
    };
    let mut octets = std::fs::read(&v1).expect("le binaire se lit");

    // L'en-tête des segments, puis celui de la pile. Les décalages sont
    // ceux d'un ELF 64 bits petit-boutien, et le binaire d'épreuve en
    // est un — le contrôle s'abstient si ce n'en est plus un.
    let lire_u64 = |o: &[u8], d: usize| u64::from_le_bytes(o[d..d + 8].try_into().unwrap());
    let lire_u16 = |o: &[u8], d: usize| u16::from_le_bytes(o[d..d + 2].try_into().unwrap());
    let lire_u32 = |o: &[u8], d: usize| u32::from_le_bytes(o[d..d + 4].try_into().unwrap());
    if octets.len() < 0x40 || &octets[..4] != b"\x7fELF" || octets[4] != 2 {
        return;
    }
    let phoff = lire_u64(&octets, 0x20) as usize;
    let (taille, combien) = (
        lire_u16(&octets, 0x36) as usize,
        lire_u16(&octets, 0x38) as usize,
    );
    /// `PT_GNU_STACK` : le segment qui ne porte aucun octet et dont les
    /// seuls droits décident si la pile s'exécute.
    const PT_GNU_STACK: u32 = 0x6474_e551;
    let mut touché = false;
    for rang in 0..combien {
        let base = phoff + rang * taille;
        if base + 8 > octets.len() {
            return;
        }
        if lire_u32(&octets, base) == PT_GNU_STACK {
            // Le bit d'exécution, et rien d'autre : la pile devient
            // exécutable, le reste du fichier ne bouge pas.
            let drapeaux = lire_u32(&octets, base + 4) | 1;
            octets[base + 4..base + 8].copy_from_slice(&drapeaux.to_le_bytes());
            touché = true;
            break;
        }
    }
    if !touché {
        return;
    }

    let atelier = tempfile::tempdir().expect("un dossier temporaire");
    let v2 = atelier.path().join("crackme-01.nx");
    std::fs::write(&v2, &octets).expect("la seconde version s'écrit");

    let out = commande()
        .args(["-s", "protections"])
        .arg(&v2)
        .arg("-c")
        .arg(&v1)
        .output()
        .expect("la commande part");
    assert_eq!(code(&out), 4, "{}", plainte(&out));
    let md = sortie(&out);
    // LE VERDICT DES DEUX CÔTÉS, et c'est ce qui rend le rapport
    // utilisable : on lit ce qui était, puis ce qui est.
    assert!(
        md.contains("- - √ **NX**") && md.contains("+ - × **NX**"),
        "le verdict de NX a changé de marque : {md}"
    );
    assert!(
        md.contains("+ | GNU_STACK | rwx |"),
        "et les droits du segment le disent : {md}"
    );
    // ET CE QUE CELA CHANGE, qui est la raison de lire un rapport
    // plutôt qu'un bit.
    assert!(
        md.contains("Segment W+X"),
        "une zone inscriptible ET exécutable est apparue : {md}"
    );
}
