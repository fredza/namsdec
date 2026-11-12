//! Les exercices : des programmes à trous, avec ce qu'on cherche à y
//! comprendre.
//!
//! Module à part, et chargé seulement si on le demande. Quelqu'un qui
//! écrit son propre code n'a pas besoin qu'on lui propose des devoirs :
//! l'application ne montre rien de tout cela tant que la case n'est pas
//! cochée dans les préférences, et cette crate n'a alors aucun effet sur
//! le reste.
//!
//! Deux sources : la série livrée avec l'application, et les séries
//! installées depuis le disque — un fichier JSON, comme un thème (voir
//! [`pack`]). Les deux donnent le même type, pour que l'interface n'ait
//! pas à savoir d'où vient ce qu'elle affiche.
//!
//! Pas de correction automatique : un exercice pose un but en clair — « à
//! l'arrêt, RAX doit valoir 15 » — et c'est le bandeau des registres qui
//! répond. Vérifier à la place de l'utilisateur lui retirerait le geste
//! qu'il est venu apprendre.

/// La version de ce module, telle que Cargo la connaît.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod pack;
pub mod reponse;

/// Un texte dans les deux langues de l'interface.
///
/// Comme pour les fiches d'instructions : l'énoncé d'un exercice *est*
/// son texte, il ne peut pas être traduit à l'affichage par quelqu'un
/// d'autre. Une série écrite dans une seule langue donne le même texte
/// des deux côtés — c'est à celui qui l'écrit de décider, pas à nous de
/// refuser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub fr: String,
    pub en: String,
}

impl Text {
    /// Le même texte quelle que soit la langue.
    pub fn same(text: &str) -> Self {
        Self {
            fr: text.to_owned(),
            en: text.to_owned(),
        }
    }

    pub fn get(&self, lang: &str) -> &str {
        if lang.starts_with("fr") {
            &self.fr
        } else {
            &self.en
        }
    }
}

/// Le palier d'un exercice.
///
/// Quatre, et nommés par ce qu'on y fait plutôt que par un chiffre : un
/// « niveau 3 » ne dit pas ce qui attend, « Structures » si. Le parcours
/// les traverse dans l'ordre, et aucun exercice ne revient à un palier
/// qu'on a quitté — c'est ce que vérifie le test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Ce que le processeur est : registres, indicateurs, sauts.
    Discovery,
    /// Ce qu'il manipule : mémoire, données, adresses.
    Practice,
    /// Ce qu'on construit avec : boucles écrites autrement, fonctions,
    /// pile, macros.
    Structures,
    /// Ce qui l'entoure : le noyau, le vectoriel, et ce qui casse.
    Mastery,
}

impl Level {
    /// Son nom, dans la langue demandée.
    pub fn label(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match (self, fr) {
            (Level::Discovery, true) => "Découverte",
            (Level::Discovery, false) => "Discovery",
            (Level::Practice, true) => "Pratique",
            (Level::Practice, false) => "Practice",
            (Level::Structures, true) => "Structures",
            (Level::Structures, false) => "Structures",
            (Level::Mastery, true) => "Maîtrise",
            (Level::Mastery, false) => "Mastery",
        }
    }

    /// Les quatre, dans l'ordre.
    pub const ALL: [Level; 4] = [
        Level::Discovery,
        Level::Practice,
        Level::Structures,
        Level::Mastery,
    ];

    /// L'identifiant écrit dans un fichier de série.
    pub fn id(self) -> &'static str {
        match self {
            Level::Discovery => "discovery",
            Level::Practice => "practice",
            Level::Structures => "structures",
            Level::Mastery => "mastery",
        }
    }

    /// Lu depuis un fichier de série. Ce qu'on ne reconnaît pas retombe
    /// sur le premier palier : une série mal étiquetée reste utilisable.
    pub fn by_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|level| level.id() == id)
            .unwrap_or(Level::Discovery)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    /// Identifiant stable, indépendant de la langue et du titre.
    pub id: String,
    pub title: Text,
    /// Ce qu'on cherche à comprendre, et le but à atteindre.
    pub description: Text,
    /// Le programme de départ, à charger dans l'éditeur.
    ///
    /// IL LUI MANQUE QUELQUE CHOSE, et c'est tout le sujet. Mesuré le
    /// 11 novembre 2026 : sur cent une étapes qui posent un but,
    /// QUATRE-VINGT-DIX-SEPT l'atteignaient déjà telles qu'elles sont
    /// livrées. On chargeait l'étape, on cliquait « Exécuter », la
    /// pastille passait au vert — sans avoir rien écrit. C'étaient des
    /// démonstrations commentées présentées comme des devoirs.
    ///
    /// Un exercice qui pose un but laisse donc un trou : une ligne
    /// « ; À FAIRE : … » à la place de ce qu'on vient apprendre à
    /// écrire. Ce qui n'a rien à faire écrire le dit en n'ayant pas de
    /// but — l'interface l'annonce alors comme une démonstration.
    pub starter_code: String,
    /// Le programme COMPLET, celui qui atteint le but.
    ///
    /// Trois choses en dépendent, et c'est pourquoi il est obligatoire
    /// dès qu'il y a un trou :
    ///
    /// - un contrôle l'exécute et vérifie qu'il atteint vraiment le but,
    ///   dans le budget annoncé. Une solution écrite à la main dans un
    ///   fichier JSON est une affirmation ; passée à l'épreuve de
    ///   l'émulateur, c'est un fait ;
    /// - le même contrôle vérifie que le programme de DÉPART, lui, ne
    ///   l'atteint pas. C'est ce qui empêche un exercice de redevenir
    ///   une démonstration sans qu'on le voie ;
    /// - l'interface peut la montrer quand on la demande. Chercher sans
    ///   jamais pouvoir regarder la réponse n'apprend rien à qui est
    ///   bloqué.
    ///
    /// `None` pour une démonstration ou un exercice de lecture : il n'y
    /// a pas de programme à écrire.
    pub solution: Option<String>,
    /// Le but, en conditions vérifiables — `rax == 15`, `rdx == 3`.
    ///
    /// Toutes doivent tenir à l'arrêt du programme. La grammaire est
    /// celle des points d'arrêt conditionnels : une valeur, une
    /// comparaison, une valeur. Vide quand l'exercice est une
    /// démonstration à regarder plutôt qu'un but à atteindre — dire
    /// « atteint » d'un exercice qui ne demande rien n'apprendrait
    /// rien.
    pub goal: Vec<String>,
    /// Le budget : atteindre le but en ce nombre d'instructions
    /// **exécutées**, ou moins.
    ///
    /// Ce n'est pas une note, c'est une mesure. Un programme juste peut
    /// être deux fois plus court, et c'est en cherchant comment qu'on
    /// apprend `lea`, les décalages, ou qu'une boucle de cinq tours se
    /// remplace par une formule. Le budget est celui de la solution
    /// livrée : l'égaler est normal, descendre en dessous est le vrai
    /// exercice.
    ///
    /// `None` quand il n'y a rien à mesurer — un exercice qui attend une
    /// entrée, ou qui montre un refus.
    pub par: Option<u64>,
    /// Le palier auquel il appartient.
    pub level: Level,
    /// La question d'un exercice de LECTURE, et sa réponse.
    ///
    /// `None` pour les cent huit exercices d'écriture : on y répond en
    /// écrivant du code, et le but dit si l'on a réussi. Un exercice de
    /// lecture, lui, donne un programme COMPLET et demande ce qu'il
    /// fait — on lit, on répond, puis on exécute, et la machine
    /// tranche. Voir [`reponse`].
    pub question: Option<Question>,
}

/// Ce qu'un exercice de lecture demande, et ce qu'il attend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub texte: Text,
    /// La réponse, écrite comme une condition — `rax == 0x20`.
    ///
    /// La même grammaire que les buts, et pour une raison qui n'est pas
    /// d'élégance : un contrôle peut **exécuter le programme et
    /// vérifier que la machine donne bien cette réponse-là**. Une
    /// réponse écrite à la main dans un fichier JSON est une
    /// affirmation ; passée à l'épreuve de l'émulateur, c'est un fait.
    pub réponse: String,
}

/// Une série d'exercices : celle qui est livrée, ou une installée depuis
/// le disque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub exercises: Vec<Exercise>,
}

/// L'identifiant de la série livrée avec l'application.
pub const BUILTIN_ID: &str = "namsdec";

/// Le nom du fichier qui porte la série livrée.
///
/// Elle n'est plus compilée dans le programme : c'est un fichier de
/// données, installé à côté des thèmes et des désassembleurs, et lu au
/// démarrage comme n'importe quelle autre série. Trois raisons à cela —
/// le binaire ne porte plus cent Ko de texte et de programmes, une
/// correction d'énoncé ne demande plus de recompiler, et ce qu'on écrit
/// soi-même se range exactement comme ce qui est livré.
pub const BUILTIN_FILE: &str = "namsdec.json";

/// La série livrée, lue dans le dossier des séries.
///
/// Absente — première installation, dossier effacé —, ce n'est pas une
/// erreur fatale : l'application démarre, et le module des exercices dit
/// ce qu'il faut faire pour la retrouver.
pub fn builtin_pack_in(dir: &std::path::Path) -> Result<Pack, pack::PackError> {
    pack::read(dir.join(BUILTIN_FILE))
}

impl Pack {
    /// L'exercice qui porte cet identifiant.
    pub fn by_id(&self, id: &str) -> Option<&Exercise> {
        self.exercises.iter().find(|e| e.id == id)
    }
}

/// Cherche un exercice dans plusieurs séries à la fois, par son
/// identifiant *complet* — « série/exercice ».
///
/// Complet parce que deux séries peuvent nommer leur premier exercice de
/// la même façon, et que celui qu'on a cliqué est celui qu'on veut.
pub fn find<'a>(packs: &'a [Pack], full_id: &str) -> Option<&'a Exercise> {
    let (pack_id, exercise_id) = full_id.split_once('/')?;
    packs.iter().find(|p| p.id == pack_id)?.by_id(exercise_id)
}

/// L'identifiant complet d'un exercice dans sa série.
pub fn full_id(pack: &Pack, exercise: &Exercise) -> String {
    format!("{}/{}", pack.id, exercise.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La série livrée, lue dans le dépôt.
    ///
    /// Les tests lisent le fichier **du dépôt**, et jamais celui qui est
    /// installé chez l'utilisateur : ce qu'on vérifie ici, c'est ce que
    /// le dépôt livre, pas l'état d'une machine.
    fn builtin_pack() -> Pack {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packs");
        builtin_pack_in(&chemin).expect("la série livrée est lisible")
    }

    #[test]
    fn every_builtin_exercise_is_complete_and_stops() {
        let pack = builtin_pack();
        // Un parcours, pas une poignée : cent dix-sept étapes, et
        // chacune s'appuie sur les précédentes.
        assert_eq!(pack.exercises.len(), 117);
        for exercise in &pack.exercises {
            assert!(!exercise.id.is_empty());
            assert!(!exercise.title.fr.is_empty() && !exercise.title.en.is_empty());
            assert!(!exercise.description.fr.is_empty());
            assert!(!exercise.description.en.is_empty());
            // Un exercice est un programme complet, pas un extrait : il
            // déclare son point d'entrée, sans quoi l'éditeur de liens
            // en choisit un à sa place — et le programme démarre là où
            // personne ne l'a demandé.
            assert!(
                exercise.starter_code.contains("global _start"),
                "{} ne déclare pas son point d'entrée",
                exercise.id
            );
            assert!(exercise.starter_code.contains("_start:"));
            // Et il doit s'arrêter, sinon « Exécuter » ne rend jamais la
            // main.
            assert!(
                exercise.starter_code.contains("hlt") || exercise.starter_code.contains("exit"),
                "{} ne s'arrête pas",
                exercise.id
            );
        }
    }

    /// Les paliers ne reculent jamais : le parcours va de la découverte
    /// à la maîtrise, et rien n'y revient en arrière.
    #[test]
    fn the_course_never_goes_back_down_a_level() {
        let pack = builtin_pack();
        let mut niveaux = pack.exercises.iter().map(|exercise| exercise.level);
        let mut précédent = niveaux.next().expect("au moins un exercice");
        for niveau in niveaux {
            assert!(
                niveau >= précédent,
                "un exercice de niveau {précédent:?} précède un {niveau:?}"
            );
            précédent = niveau;
        }
        // Et les quatre paliers sont représentés : un parcours qui n'en
        // aurait que trois n'aurait pas besoin du quatrième.
        for niveau in Level::ALL {
            assert!(
                pack.exercises
                    .iter()
                    .any(|exercise| exercise.level == niveau),
                "aucun exercice en {niveau:?}"
            );
        }
    }

    /// Le parcours apprend aussi à LIRE un binaire, et pas seulement à
    /// en écrire un.
    ///
    /// Il ne l'a pas toujours fait : quatre-vingt-seize exercices, tous
    /// d'écriture, dans une application dont la moitié des panneaux
    /// sert à l'analyse. On apprenait `xor` en l'écrivant, jamais à le
    /// reconnaître dans un fichier qu'on n'a pas écrit — et c'est
    /// pourtant ce qu'on fait devant un binaire inconnu.
    ///
    /// CE QUI DISTINGUE CES EXERCICES-LÀ, et ce que ce contrôle mesure :
    /// leur énoncé nomme un panneau de namsdec. Un exercice de
    /// rétro-ingénierie qui ne dit pas OÙ REGARDER n'en est pas un ;
    /// c'est le geste qu'on vient apprendre, pas la syntaxe.
    #[test]
    fn le_parcours_apprend_a_lire_un_binaire_et_pas_seulement_a_en_ecrire() {
        let pack = builtin_pack();
        let rétro: Vec<&Exercise> = pack
            .exercises
            .iter()
            .filter(|e| e.id.starts_with("re_"))
            .collect();
        assert!(
            rétro.len() >= 10,
            "un seul exercice de lecture ne fait pas un apprentissage : {}",
            rétro.len()
        );

        // Sur plusieurs paliers : lire un fichier se commence tôt — une
        // signature et des chaînes se regardent avant de savoir ce
        // qu'est une pile —, et finit tard, sur ce qui résiste.
        let paliers: std::collections::BTreeSet<Level> = rétro.iter().map(|e| e.level).collect();
        assert!(
            paliers.len() >= 3,
            "la lecture d'un binaire s'apprend par paliers, pas d'un bloc : {paliers:?}"
        );

        // Et chacun dit où regarder.
        let panneaux = [
            "Analyse",
            "Désassemblage",
            "Éditeur binaire",
            "Chronologie",
            "Correctifs",
            "Mémoire",
            "Fragments",
            "Chaînes",
            "Entropie",
            "Fonctions importées",
            "Appels système",
        ];
        for exercice in &rétro {
            let texte = format!("{} {}", exercice.description.fr, exercice.starter_code);
            assert!(
                panneaux.iter().any(|p| texte.contains(p)),
                "{} ne dit pas où regarder dans namsdec",
                exercice.id
            );
        }
    }

    #[test]
    fn an_exercise_is_found_by_its_full_identifier() {
        let packs = vec![builtin_pack()];
        let found = find(&packs, "namsdec/boucle").expect("l'exercice existe");
        assert_eq!(found.id, "boucle");
        assert_eq!(full_id(&packs[0], found), "namsdec/boucle");
        // Le nom seul ne suffit pas : deux séries peuvent le porter.
        assert!(find(&packs, "boucle").is_none());
        assert!(find(&packs, "autre/boucle").is_none());
    }
}
