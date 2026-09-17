//! Ce que GitHub répond vraiment, et ce que namsdec en fait.
//!
//! La réponse enregistrée ici vient de l'API du dépôt, le jour de la
//! première publication, réduite aux champs qui servent. Un JSON inventé
//! prouve qu'on sait lire ce qu'on a écrit ; celui-ci prouve qu'on sait
//! lire ce que GitHub envoie.

const RÉPONSE: &str = include_str!("github-releases.json");

/// La version publiée se lit, avec son étiquette, son adresse et le fait
/// qu'elle est une préversion.
#[test]
fn the_real_answer_is_read() {
    let liste = namsdec_update::parse_releases(RÉPONSE).expect("la réponse est lisible");
    assert!(!liste.is_empty(), "au moins une version publiée");
    let première = &liste[0];
    assert!(première.tag.starts_with('v'), "{}", première.tag);
    assert_eq!(première.version, première.tag.trim_start_matches('v'));
    assert!(
        première
            .url
            .starts_with("https://github.com/fredza/namsdec/releases/"),
        "{}",
        première.url
    );
    // Une étiquette à tiret est publiée comme préversion : c'est le
    // workflow qui le décide, et l'API le confirme.
    assert_eq!(première.prerelease, première.tag.contains('-'));
}

/// Et le verdict rendu est celui qu'on attend, dans les trois cas.
#[test]
fn the_verdict_from_the_real_answer_is_the_expected_one() {
    let liste = namsdec_update::parse_releases(RÉPONSE).expect("la réponse est lisible");
    let publiée = namsdec_update::pick_latest(&liste, true).expect("une version proposable");

    // Une version plus ancienne se voit proposer celle-ci.
    assert!(matches!(
        namsdec_update::verdict("0.0.1", publiée.clone()),
        namsdec_update::Verdict::Newer(_)
    ));
    // La même est à jour.
    assert_eq!(
        namsdec_update::verdict(&publiée.version, publiée.clone()),
        namsdec_update::Verdict::UpToDate
    );
    // Et compiler le dépôt met devant ce qui est publié, ce qui se dit.
    assert!(matches!(
        namsdec_update::verdict("99.0.0", publiée.clone()),
        namsdec_update::Verdict::Ahead(_)
    ));

    // Tant que seule une préversion est publiée, une version définitive
    // n'a rien à se voir proposer : c'est la règle du canal, et c'est
    // aussi pourquoi `releases/latest` — qui ignore les préversions —
    // répondait 404 sur ce dépôt.
    if liste.iter().all(|release| release.prerelease) {
        assert!(namsdec_update::pick_latest(&liste, false).is_none());
    }
}
