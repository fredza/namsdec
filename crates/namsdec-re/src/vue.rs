//! Ce qu'un panneau a à montrer, dit sans savoir comment on le peint.
//!
//! Jusqu'ici, un module de `namsdec-re` n'avait qu'une sortie : `ui()`,
//! qui dessine en egui. Ce que namsdec comprend d'un binaire n'existait
//! donc que peint à l'écran — ni exportable, ni lisible par un autre
//! programme, ni éprouvable autrement qu'en mesurant des pixels.
//!
//! **Ce vocabulaire est dérivé d'un relevé, pas imaginé.** Les vingt
//! `*_module.rs` ont été comptés avant qu'une ligne soit écrite ici :
//! deux cent trente appels de texte sous cinq emphases, des tableaux
//! dans onze modules sur vingt, dix-huit boutons, quatorze adresses
//! cliquables, et du dessin propre dans **quatre modules seulement**.
//! D'où un vocabulaire court — texte, tableau, jauge, trait, attente —
//! et pas un de plus : une « vue » assez riche pour tout décrire serait
//! un second HTML, c'est-à-dire un second moteur de rendu à maintenir.
//!
//! Les quatre qui dessinent — entropie, graphe d'appels, éditeur
//! binaire, décompilation — exposeront leurs **données** et non un
//! dessin : le dessin reste à l'interface, qui est son métier.
//!
//! Ce fichier ne connaît pas egui, et ne doit jamais le connaître.
//! C'est ce qui fait de lui une frontière plutôt qu'une couche de plus.

use serde::{Deserialize, Serialize};

/// Ce qu'un panneau montre, de haut en bas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Vue {
    pub blocs: Vec<Bloc>,
}

impl Vue {
    pub fn neuve() -> Self {
        Self::default()
    }

    /// Ajoute un bloc, et se rend — pour écrire une vue d'un trait.
    #[must_use]
    pub fn bloc(mut self, bloc: Bloc) -> Self {
        self.blocs.push(bloc);
        self
    }

    /// Une ligne de texte ordinaire, forme la plus courante de loin.
    #[must_use]
    pub fn texte(self, fragments: impl Into<Ligne>) -> Self {
        self.bloc(Bloc::Texte(fragments.into()))
    }

    pub fn est_vide(&self) -> bool {
        self.blocs.is_empty()
    }
}

/// Un morceau de panneau.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Bloc {
    /// Une ligne de texte, faite d'un ou plusieurs fragments.
    Texte(Ligne),
    /// Un tableau. `colonnes` vide veut dire « pas d'en-tête » : c'est
    /// le cas de la moitié des grilles du projet, qui alignent deux
    /// colonnes « nom / valeur » sans les titrer.
    Tableau {
        colonnes: Vec<Ligne>,
        rangées: Vec<Vec<Ligne>>,
        /// Une rangée sur deux teintée. Un tableau de trois lignes n'en
        /// a pas besoin ; un de quarante, si.
        rayé: bool,
    },
    /// Une part de quelque chose, entre 0 et 1, et ce qu'elle mesure.
    Jauge { part: f32, texte: Ligne },
    /// Un séparateur.
    Trait,
    /// Quelque chose est en train de se faire, et on dit quoi.
    Attente(Ligne),
    /// Un intertitre et ce qu'il introduit.
    Section { titre: Ligne, blocs: Vec<Bloc> },
    /// Ce qui se replie derrière son titre.
    ///
    /// L'état d'ouverture appartient à celui qui peint, pas au module :
    /// `ouvert` ne dit que la position de DÉPART — « une famille de
    /// douze endroits se parcourt, une de mille se demande ». Le module
    /// reste donc sans mémoire de ce que l'utilisateur a déplié, ce qui
    /// est le cas aujourd'hui dans `patterns_module` et qu'il serait
    /// dommage de perdre.
    Repliable {
        titre: Ligne,
        ouvert: bool,
        blocs: Vec<Bloc>,
    },
    /// Du code source, tel qu'un décompilateur l'a rendu.
    ///
    /// Le TEXTE est la donnée ; la coloration et les adresses qu'on y
    /// clique sont de la présentation, et c'est l'interface qui s'en
    /// charge — elle seule sait quelles couleurs elle emploie et vers
    /// quoi une adresse mène chez elle. Un rapport en reçoit du code
    /// lisible, ce qui est exactement ce qu'il veut.
    Code(String),
    /// Un groupe en retrait, sans titre.
    ///
    /// Le retrait dit un rattachement : ces lignes-ci développent la
    /// précédente. Sans lui, une liste de détails se lit comme une
    /// suite de faits indépendants.
    Retrait(Vec<Bloc>),
}

/// Une ligne : des fragments mis bout à bout.
///
/// Un type à part plutôt qu'un `Vec<Fragment>` nu, pour qu'une ligne
/// puisse se construire à partir d'une simple chaîne — ce qui est le
/// cas de la grande majorité d'entre elles.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ligne {
    pub fragments: Vec<Fragment>,
}

impl Ligne {
    #[must_use]
    pub fn puis(mut self, fragment: Fragment) -> Self {
        self.fragments.push(fragment);
        self
    }

    /// Le texte seul, sans les tons ni les liens — pour un rapport, une
    /// recherche, ou un contrôle qui veut savoir ce qui est écrit.
    pub fn texte_nu(&self) -> String {
        self.fragments
            .iter()
            .map(|f| f.texte.as_str())
            .collect::<Vec<_>>()
            .join("")
    }
}

impl From<Fragment> for Ligne {
    fn from(fragment: Fragment) -> Self {
        Self {
            fragments: vec![fragment],
        }
    }
}

impl From<&str> for Ligne {
    fn from(texte: &str) -> Self {
        Fragment::dit(texte).into()
    }
}

impl From<String> for Ligne {
    fn from(texte: String) -> Self {
        Fragment::dit(texte).into()
    }
}

impl From<Vec<Fragment>> for Ligne {
    fn from(fragments: Vec<Fragment>) -> Self {
        Self { fragments }
    }
}

/// Un morceau de ligne : son texte, et ce qu'il est.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub texte: String,
    pub ton: Ton,
    /// L'adresse où ce fragment mène, s'il y mène. L'interface en fait
    /// un lien vers le listing ; un rapport n'en fait rien de spécial.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mène_à: Option<u64>,
    /// Se copie d'un clic. Une empreinte sert à être collée quelque
    /// part, pas à être recopiée à la main.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub copiable: bool,
    /// Ce qui se lit au survol, quand il y a davantage à dire.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub survol: Option<String>,
    /// Ce fragment est un **bouton**, et voici ce qu'il demande.
    ///
    /// Un identifiant que le module reconnaît, et non la demande
    /// elle-même : la vue reste du texte et des noms, sérialisable et
    /// lisible par qui ne connaît pas namsdec. C'est le module qui
    /// traduit « identifier » en ce qu'il faut faire — lui seul le
    /// sait, et une autre interface fera la même traduction avec ses
    /// propres moyens.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub action: Option<String>,
    /// Ce fragment est un **bouton encadré** et non un lien.
    ///
    /// Par défaut une action fait un lien — du texte qu'on clique,
    /// comme une adresse —, parce que c'est la forme la plus fréquente
    /// et la plus discrète. Un bouton encadré dit « ceci va LANCER
    /// quelque chose », et c'est un autre message : `upx` le mérite,
    /// le nom d'une fonction qu'on déplie non.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub bouton: bool,
    /// La catégorie à laquelle ce fragment appartient, par son nom.
    ///
    /// Un NOM et non une couleur, pour la même raison que l'action est
    /// un nom : `behaviour::Family::clé()` donne « réseau »,
    /// « crypto », « anti-debug », et c'est l'interface qui décide de
    /// quelle teinte elle peint ce mot-là. Un module qui écrivait
    /// `famille.color()` choisissait une couleur — ce que la frontière
    /// doit lui retirer.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub étiquette: Option<String>,
    /// Un bouton qu'on ne peut pas presser maintenant.
    ///
    /// Grisé et NON caché, parce que ce sont deux messages différents :
    /// un bouton absent fait chercher, un bouton grisé fait lire la
    /// raison. C'est pourquoi `survol` compte double ici.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub inactif: bool,
}

impl Fragment {
    pub fn dit(texte: impl Into<String>) -> Self {
        Self {
            texte: texte.into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn ton(mut self, ton: Ton) -> Self {
        self.ton = ton;
        self
    }

    /// Ce qui s'écrit en chasse fixe : une adresse, un octet, un
    /// mnémonique, une empreinte.
    #[must_use]
    pub fn code(self) -> Self {
        self.ton(Ton::Code)
    }

    /// Ce qui s'efface : une explication, un commentaire, un « rien à
    /// montrer ».
    #[must_use]
    pub fn faible(self) -> Self {
        self.ton(Ton::Faible)
    }

    #[must_use]
    pub fn fort(self) -> Self {
        self.ton(Ton::Fort)
    }

    #[must_use]
    pub fn mène_à(mut self, adresse: u64) -> Self {
        self.mène_à = Some(adresse);
        self
    }

    #[must_use]
    pub fn copiable(mut self) -> Self {
        self.copiable = true;
        self
    }

    #[must_use]
    pub fn survol(mut self, texte: impl Into<String>) -> Self {
        self.survol = Some(texte.into());
        self
    }

    /// En fait un **lien** qui demande `quoi` quand on le clique.
    #[must_use]
    pub fn agit(mut self, quoi: impl Into<String>) -> Self {
        self.action = Some(quoi.into());
        self
    }

    /// En fait un **bouton encadré** qui demande `quoi`.
    #[must_use]
    pub fn bouton(self, quoi: impl Into<String>) -> Self {
        let mut fragment = self.agit(quoi);
        fragment.bouton = true;
        fragment
    }

    /// Range ce fragment dans une catégorie, par son nom.
    #[must_use]
    pub fn étiquette(mut self, nom: impl Into<String>) -> Self {
        self.étiquette = Some(nom.into());
        self
    }

    /// Le même bouton, hors d'usage pour l'instant.
    #[must_use]
    pub fn inactif_si(mut self, oui: bool) -> Self {
        self.inactif = oui;
        self
    }
}

/// Ce qu'un fragment est, du point de vue du sens et non de la couleur.
///
/// `Bon`, `Attention` et `Mauvais` disent un VERDICT, pas une teinte :
/// c'est l'interface qui choisit ses couleurs, et un rapport en texte
/// peut en faire tout autre chose. Nommer la couleur ici aurait figé le
/// thème dans les données.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ton {
    #[default]
    Normal,
    /// Ce qui accompagne sans être lu en premier.
    Faible,
    /// Ce qui doit être lu en premier.
    Fort,
    /// Ce qui s'écrit en chasse fixe.
    Code,
    /// Un verdict favorable.
    Bon,
    /// Un verdict qui appelle à regarder.
    Attention,
    /// Un verdict défavorable.
    Mauvais,
    /// Un mot qu'on retrouve ailleurs dans l'application.
    ///
    /// Le nom d'un registre dans une fiche de fonction, par exemple :
    /// il n'est pas cliquable, mais il désigne quelque chose que le
    /// bandeau du processeur montre, et le voir ressortir évite de
    /// relire la ligne pour le trouver. « Repère » et non « lien » : un
    /// lien se clique, celui-ci se regarde.
    Repère,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une vue se sérialise et se relit à l'identique.
    ///
    /// C'est toute la raison d'être de ce fichier : ce qu'un panneau
    /// montre doit pouvoir sortir de namsdec. Si l'aller-retour perd
    /// quelque chose, la frontière ne sert à rien.
    #[test]
    fn une_vue_fait_l_aller_retour_par_json() {
        let vue = Vue::neuve()
            .texte("Rien à signaler")
            .bloc(Bloc::Tableau {
                colonnes: vec!["Quoi".into(), "Valeur".into()],
                rangées: vec![vec![
                    "SHA-256".into(),
                    Fragment::dit("abcd").code().copiable().into(),
                ]],
                rayé: true,
            })
            .bloc(Bloc::Jauge {
                part: 0.75,
                texte: Fragment::dit("entropie").faible().into(),
            })
            .bloc(Bloc::Trait)
            .bloc(Bloc::Section {
                titre: "Ce qu'il emprunte".into(),
                blocs: vec![Bloc::Texte(
                    Fragment::dit("printf").code().mène_à(0x401000).into(),
                )],
            });
        let json = serde_json::to_string(&vue).expect("la vue se sérialise");
        let relue: Vue = serde_json::from_str(&json).expect("et se relit");
        assert_eq!(vue, relue);
    }

    /// Le texte nu d'une ligne est ce qu'elle dit, sans la façon de le
    /// dire — ce qu'un rapport ou une recherche lit.
    #[test]
    fn le_texte_nu_oublie_la_presentation() {
        let ligne: Ligne = vec![
            Fragment::dit("appelle "),
            Fragment::dit("sub_401000").code().mène_à(0x401000),
        ]
        .into();
        assert_eq!(ligne.texte_nu(), "appelle sub_401000");
    }

    /// Ce qui n'est pas dit ne pèse rien dans le JSON.
    ///
    /// Une vue de quelques centaines de lignes porterait autrement
    /// autant de `"copiable": false` et de `"mène_à": null` que de
    /// contenu, et c'est ce contenu qu'on veut relire.
    #[test]
    fn un_fragment_ordinaire_ne_porte_rien_d_inutile() {
        let json = serde_json::to_string(&Fragment::dit("salut")).expect("sérialisé");
        assert!(!json.contains("mène_à"), "{json}");
        assert!(!json.contains("copiable"), "{json}");
        assert!(!json.contains("survol"), "{json}");
        assert!(!json.contains("action"), "{json}");
        assert!(!json.contains("inactif"), "{json}");
        assert!(!json.contains("bouton"), "{json}");
        assert!(!json.contains("étiquette"), "{json}");
    }

    /// Un bouton dit ce qu'il demande, par un nom — et non par la
    /// demande elle-même.
    ///
    /// C'est ce qui garde la vue lisible hors de namsdec : un rapport
    /// ou une autre interface voit « ce texte est un bouton nommé
    /// identifier », et décide ce qu'il en fait.
    #[test]
    fn un_bouton_porte_le_nom_de_ce_qu_il_demande() {
        let bouton = Fragment::dit("Identifier")
            .bouton("upx:lister")
            .survol("Rien n'est écrit")
            .inactif_si(true);
        let json = serde_json::to_string(&bouton).expect("sérialisé");
        let relu: Fragment = serde_json::from_str(&json).expect("relu");
        assert_eq!(relu.action.as_deref(), Some("upx:lister"));
        assert!(relu.bouton, "encadré : il lance quelque chose");
        assert!(relu.inactif);
        // Et une action ordinaire fait un lien, pas un bouton.
        assert!(!Fragment::dit("printf").agit("fiche:printf").bouton);
        assert_eq!(relu.texte, "Identifier");
    }
}
