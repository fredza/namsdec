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
    /// UN COUP DE POUCE, entre la consigne et la solution.
    ///
    /// Sans lui, l'aide est tout ou rien : on relit la consigne, on ne
    /// trouve pas, et le seul recours est le programme entier — ce qui
    /// met fin à l'exercice. Un indice dit où chercher sans dire quoi
    /// écrire.
    ///
    /// Facultatif, et pour une raison mesurée : quatre-vingt-cinq des
    /// cent huit étapes en ont un SANS QU'ON L'ÉCRIVE — « la solution
    /// livrée emploie `add`, `dec`, `jnz` », calculé en comparant la
    /// solution à ce qu'on a tapé. Ce champ sert aux vingt-sept autres, dont
    /// le trou ne demande aucune instruction nouvelle : une valeur à
    /// trouver, un ordre à remettre, une répétition à voir. Ce sont les
    /// plus subtiles, et c'est là qu'un indice compte le plus.
    pub hint: Option<Text>,
    /// LA THÉORIE : ce qu'il faut avoir compris AVANT de s'y mettre.
    ///
    /// Ni la consigne, ni l'indice, et c'est ce qui la distingue des
    /// deux. La consigne dit ce qu'on demande ; l'indice dit où
    /// chercher quand on est bloqué. La théorie, elle, ne parle pas de
    /// CET exercice : elle dit la mécanique du processeur sur laquelle
    /// il repose — ce qu'est une boucle quand le langage n'en a pas, ce
    /// qu'un indicateur retient, pourquoi écrire `eax` efface le haut
    /// de `rax`. On la lit avant d'essayer, et elle resservira à dix
    /// autres exercices.
    ///
    /// Elle cite entre accents graves les mots qui ont une fiche —
    /// `dec`, `ZF`, `rsp` —, et l'interface en fait des renvois
    /// cliquables sans qu'on ait à tenir une liste à côté. C'est le
    /// même mécanisme que `namsdec_doc::related`, et pour la même
    /// raison : une liste écrite à la main est une liste qui vieillit.
    ///
    /// Obligatoire dans la série livrée — un contrôle le vérifie —,
    /// facultative dans le format : une série tierce peut ne pas en
    /// avoir, et refuser de la charger pour cela serait fermer la porte
    /// à ce que le format est censé ouvrir.
    pub theorie: Option<Text>,
    /// La question d'un exercice de LECTURE, et sa réponse.
    ///
    /// `None` pour les cent huit exercices d'écriture et les sept
    /// démonstrations : on y répond en écrivant du code, ou en
    /// regardant tourner, et le but dit si l'on a réussi. Un exercice de
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
        // Un parcours, pas une poignée : cent trente-huit étapes, et
        // chacune s'appuie sur les précédentes.
        assert_eq!(pack.exercises.len(), 138);
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

#[cfg(test)]
mod theorie_tests {
    use super::*;

    fn pack() -> Pack {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packs");
        builtin_pack_in(&chemin).expect("la série livrée est lisible")
    }

    /// Les mots cités entre accents graves dans un texte.
    ///
    /// Le même découpage que l'interface : les tranches impaires d'un
    /// `split('`')`, dont on ne garde que le premier mot — « `cmp rax,
    /// rbx` » cite `cmp`. Écrire ici un second découpage, qui
    /// divergerait du premier, ferait un contrôle qui vérifie autre
    /// chose que ce qu'on voit.
    fn mots_cités(texte: &str) -> Vec<String> {
        texte
            .split('`')
            .enumerate()
            .filter(|(i, _)| i % 2 == 1)
            .filter_map(|(_, morceau)| morceau.split_whitespace().next())
            .map(|mot| {
                mot.trim_end_matches([',', '.', ':', ';', '?', '!'])
                    .to_ascii_lowercase()
            })
            .filter(|mot| !mot.is_empty())
            .collect()
    }

    /// Chaque exercice livré porte sa théorie, dans les deux langues.
    ///
    /// Obligatoire ici, facultative dans le format : une série tierce
    /// peut ne pas en avoir, et refuser de la charger pour cela serait
    /// fermer la porte que le format est censé ouvrir. Mais la série
    /// LIVRÉE est le parcours, et un parcours dont une étape sur cent
    /// dix-sept n'explique pas sur quoi elle repose a un trou que
    /// personne ne verra — on ne lit pas les cent trente-huit d'affilée.
    #[test]
    fn chaque_exercice_livre_porte_sa_theorie() {
        let pack = pack();
        let mut sans: Vec<&str> = Vec::new();
        for exercice in &pack.exercises {
            match &exercice.theorie {
                Some(t) if !t.fr.trim().is_empty() && !t.en.trim().is_empty() => {}
                _ => sans.push(&exercice.id),
            }
        }
        assert!(
            sans.is_empty(),
            "{} exercice(s) sans théorie dans les deux langues : {sans:?}",
            sans.len()
        );
    }

    /// La théorie dit autre chose que la consigne.
    ///
    /// C'EST LA RAISON D'ÊTRE DU CHAMP, et le seul moyen de la retenir.
    /// La consigne dit ce qu'on demande ; la théorie dit la mécanique
    /// sur laquelle l'exercice repose, et elle doit resservir ailleurs.
    /// Recopier l'une dans l'autre donnerait cent trente-huit redites que
    /// personne ne relirait, et le repli serait un repli vide.
    ///
    /// La mesure est grossière et honnête : on compte les mots communs.
    /// Au-delà de la moitié, ce n'est plus un autre texte.
    #[test]
    fn la_theorie_ne_recopie_pas_la_consigne() {
        let pack = pack();
        let mut fautifs: Vec<String> = Vec::new();
        for exercice in &pack.exercises {
            let Some(théorie) = &exercice.theorie else {
                continue;
            };
            for (langue, th, desc) in [
                ("fr", théorie.fr.as_str(), exercice.description.fr.as_str()),
                ("en", théorie.en.as_str(), exercice.description.en.as_str()),
            ] {
                let mots_consigne: std::collections::BTreeSet<String> = desc
                    .split_whitespace()
                    .map(|m| m.to_lowercase())
                    .filter(|m| m.len() > 4)
                    .collect();
                let mots_théorie: Vec<String> = th
                    .split_whitespace()
                    .map(|m| m.to_lowercase())
                    .filter(|m| m.len() > 4)
                    .collect();
                if mots_théorie.is_empty() {
                    continue;
                }
                let communs = mots_théorie
                    .iter()
                    .filter(|m| mots_consigne.contains(*m))
                    .count();
                let part = communs as f32 / mots_théorie.len() as f32;
                if part > 0.5 {
                    fautifs.push(format!(
                        "{} ({langue}) : {:.0} % des mots viennent de la consigne",
                        exercice.id,
                        part * 100.0
                    ));
                }
            }
        }
        assert!(
            fautifs.is_empty(),
            "la théorie recopie la consigne au lieu d'expliquer la mécanique :\n  {}",
            fautifs.join("\n  ")
        );
    }

    /// Chaque théorie mène quelque part.
    ///
    /// L'interface fait un renvoi cliquable de tout mot cité entre
    /// accents graves QUI A UNE FICHE. Une théorie qui n'en cite aucune
    /// est une impasse : on la lit, on ne sait pas où aller ensuite, et
    /// les cent quatre-vingt-neuf fiches restent fermées. Une par
    /// théorie suffit — ce n'est pas un quota, c'est une porte.
    ///
    /// Éprouvé en retirant les accents graves d'une théorie : il la
    /// nomme.
    #[test]
    fn chaque_theorie_cite_au_moins_une_fiche() {
        let pack = pack();
        let mut impasses: Vec<String> = Vec::new();
        for exercice in &pack.exercises {
            let Some(théorie) = &exercice.theorie else {
                continue;
            };
            for (langue, texte) in [("fr", théorie.fr.as_str()), ("en", théorie.en.as_str())] {
                let mène = mots_cités(texte)
                    .iter()
                    .any(|mot| namsdec_doc::lookup(mot).is_some());
                if !mène {
                    impasses.push(format!("{} ({langue})", exercice.id));
                }
            }
        }
        assert!(
            impasses.is_empty(),
            "{} théorie(s) ne citent aucune fiche : on les lit, et il n'y a \
             nulle part où aller ensuite — {impasses:?}",
            impasses.len()
        );
    }

    /// Deux exercices n'ont jamais la même théorie.
    ///
    /// Un copier-coller d'une étape à l'autre se verrait tout de suite
    /// en les lisant d'affilée — sauf que personne ne les lit
    /// d'affilée. C'est exactement le genre de chose qu'un contrôle
    /// attrape et qu'un relecteur manque.
    #[test]
    fn deux_exercices_nont_jamais_la_meme_theorie() {
        let pack = pack();
        let mut vues: std::collections::BTreeMap<&str, &str> = Default::default();
        let mut doublons: Vec<String> = Vec::new();
        for exercice in &pack.exercises {
            let Some(théorie) = &exercice.theorie else {
                continue;
            };
            if let Some(premier) = vues.insert(théorie.fr.as_str(), &exercice.id) {
                doublons.push(format!("{} et {}", premier, exercice.id));
            }
        }
        assert!(
            doublons.is_empty(),
            "des théories sont recopiées d'un exercice à l'autre : {doublons:?}"
        );
    }

    /// Une théorie s'explique, et elle se lit d'un trait.
    ///
    /// LES DEUX BORNES VIENNENT D'UNE MESURE, non d'un goût, et elles
    /// ont changé deux fois : le jour où la théorie a quitté la bande
    /// de l'exercice pour une fenêtre à elle, puis le jour où elle a
    /// été complétée pour qui débute.
    ///
    /// Le plancher était à quatre cents caractères, ce qui laissait
    /// passer une mécanique MENTIONNÉE au lieu d'être expliquée. Le
    /// compte l'a dit : sur les cent trente-huit théories livrées,
    /// quatre-vingt-dix-sept ne montraient aucun exemple instrumenté
    /// et cinquante et une pas un seul nombre. Un débutant à qui l'on
    /// explique `movzx` sans jamais écrire ce que devient `0xFF` n'a
    /// pas reçu d'explication : il a reçu un vocabulaire. Les cent
    /// trente-huit portent maintenant chacune un passage chiffré —
    /// voir le contrôle d'à côté —, et cela coûte de quatre à six
    /// cents caractères. D'où le plancher à mille deux cents.
    ///
    /// Au-dessus de deux mille quatre cents, on ne lit plus d'un
    /// trait : la fenêtre se met à défiler, et un texte qu'on lit
    /// AVANT de commencer doit tenir sous les yeux. Les deux cent
    /// soixante-seize textes — cent trente-huit dans deux langues —
    /// tiennent entre 1274 et 2004 caractères ; les bornes laissent
    /// de la place sans ouvrir la porte au chapitre de cours.
    ///
    /// ET LE PLAFOND N'EST PAS LA VRAIE BORNE. Ce qui décide est la
    /// hauteur rendue, que `banc::aucune_theorie_ne_demande_de_defiler`
    /// mesure dans l'image à 1280x800 — quatre théories y défilaient
    /// encore après l'élargissement de la fiche, dont une de 1708
    /// caractères qui avait un paragraphe de trop. Les caractères se
    /// comptent ici parce que c'est instantané ; la hauteur se mesure
    /// là-bas parce que c'est la vérité.
    ///
    /// ET LA FORME COMPTE AUTANT QUE LA LONGUEUR : une idée par
    /// paragraphe, séparés par une ligne vide, que la fenêtre rend
    /// avec de l'air entre eux. Un bloc de huit cents caractères d'un
    /// seul tenant est un mur, quelle que soit sa qualité. Les textes
    /// livrés en comptent de cinq à neuf.
    #[test]
    fn une_theorie_s_explique_et_se_lit_d_un_trait() {
        let pack = pack();
        let mut hors_bornes: Vec<String> = Vec::new();
        for exercice in &pack.exercises {
            let Some(théorie) = &exercice.theorie else {
                continue;
            };
            for (langue, texte) in [("fr", théorie.fr.as_str()), ("en", théorie.en.as_str())] {
                let n = texte.chars().count();
                if !(1200..=2400).contains(&n) {
                    hors_bornes.push(format!("{} ({langue}) : {n} caractères", exercice.id));
                }
                // Au moins deux paragraphes : un seul bloc est un mur.
                if texte.split("\n\n").filter(|p| !p.trim().is_empty()).count() < 2 {
                    hors_bornes.push(format!(
                        "{} ({langue}) : un seul paragraphe — une idée par paragraphe",
                        exercice.id
                    ));
                }
            }
        }
        assert!(
            hors_bornes.is_empty(),
            "des théories ne se lisent pas d'un trait (1200 à 2400 caractères, \
             au moins deux paragraphes) :\n  {}",
            hors_bornes.join("\n  ")
        );
    }

    /// Une théorie montre un exemple CHIFFRÉ, et pas seulement une règle.
    ///
    /// **Le défaut qu'il attrape, et le compte qui l'a trouvé.** La
    /// fenêtre de théorie promet, dans son propre commentaire, « un
    /// exemple avec de vrais nombres » — c'est l'une des trois
    /// raisons pour lesquelles elle a quitté la bande de quatre-vingts
    /// pixels. Or sur les cent trente-huit théories livrées,
    /// quatre-vingt-dix-sept ne citaient aucune instruction avec ses
    /// opérandes, et cinquante et une ne portaient pas un chiffre. La
    /// promesse était tenue par vingt-neuf d'entre elles.
    ///
    /// Une règle énoncée — « lire un octet laisse le haut du registre
    /// tel qu'il était » — est juste et ne s'attrape pas. La même
    /// règle avec `0xFF` d'un côté et `0x00000000000000FF` de
    /// l'autre se retient, parce qu'on peut la REFAIRE. C'est toute
    /// la différence entre savoir et avoir vu.
    ///
    /// **La mesure est la plus grossière possible, et c'est
    /// voulu** : un chiffre quelque part dans le texte. Un contrôle
    /// plus fin — un décompte de valeurs hexadécimales, une forme
    /// imposée — vérifierait une façon d'écrire plutôt qu'un exemple,
    /// et le premier auteur qui écrirait son exemple autrement le
    /// ferait tomber pour rien. Le plancher de longueur du contrôle
    /// d'à côté fait le reste du travail.
    ///
    /// Éprouvé en retirant le passage chiffré d'une théorie : il la
    /// nomme, dans la langue où le passage manque.
    #[test]
    fn chaque_theorie_montre_un_exemple_chiffre() {
        let pack = pack();
        let mut sans_exemple: Vec<String> = Vec::new();
        for exercice in &pack.exercises {
            let Some(théorie) = &exercice.theorie else {
                continue;
            };
            for (langue, texte) in [("fr", théorie.fr.as_str()), ("en", théorie.en.as_str())] {
                if !texte.chars().any(|c| c.is_ascii_digit()) {
                    sans_exemple.push(format!("{} ({langue})", exercice.id));
                }
            }
        }
        assert!(
            sans_exemple.is_empty(),
            "{} théorie(s) n'énoncent qu'une règle, sans un seul nombre à \
             refaire : {sans_exemple:?}",
            sans_exemple.len()
        );
    }
}
