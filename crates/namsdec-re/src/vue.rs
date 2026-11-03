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
    /// Un champ où l'on tape, et ce que sa validation demande.
    ///
    /// **Le texte en cours de frappe appartient à celui qui peint**,
    /// comme l'état d'ouverture d'un [`Bloc::Repliable`] : le module
    /// n'apprend ce qui est écrit qu'au moment où l'on valide. C'est ce
    /// qui permet à `vue()` de n'emprunter `self` qu'en lecture — un
    /// `TextEdit` veut un `&mut String`, et le lui donner aurait rendu
    /// la vue dépendante du geste en cours au lieu de ce que le panneau
    /// MONTRE.
    ///
    /// Compté avant d'être écrit, comme le reste du vocabulaire :
    /// **treize champs dans sept modules** — « aller à » dans le graphe
    /// d'appels et dans l'éditeur binaire, un filtre dans les outils et
    /// les figures, un nom à donner dans les fonctions, une tranche à
    /// vider, une adresse de détour, des octets à écrire. Douze sont
    /// d'une seule ligne ; le treizième — le code d'un détour — est un
    /// éditeur de plusieurs lignes, et attendra sa propre forme plutôt
    /// qu'un drapeau de plus ici.
    Champ(Champ),
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
    /// Des valeurs qu'une interface peint à sa façon.
    ///
    /// C'est ici que la frontière gagne le plus. Le profil d'entropie
    /// décide « une colonne par pixel, et la plus haute des tranches
    /// qu'elle recouvre » — une décision qui n'a de sens qu'une fois la
    /// largeur connue, c'est-à-dire dans l'interface et nulle part
    /// ailleurs. Le module, lui, n'a qu'une chose à dire : voici une
    /// valeur par tranche.
    Dessin(Dessin),
    /// Un groupe en retrait, sans titre.
    ///
    /// Le retrait dit un rattachement : ces lignes-ci développent la
    /// précédente. Sans lui, une liste de détails se lit comme une
    /// suite de faits indépendants.
    Retrait(Vec<Bloc>),
    /// Une chose qu'on vérifie, ce qu'elle vaut, et ce que ça change.
    ///
    /// **Comptée avant d'être écrite, comme le reste : seize lignes de
    /// ce genre, et TROIS marques différentes pour la même donnée.**
    /// « NX est-il là ? » s'écrivait d'une pastille ronde peinte au
    /// `painter` dans le panneau Protections, d'un `√` ou d'un `×` dans
    /// « En bref », et des mêmes glyphes mais d'un autre vert dans la
    /// fiche du panneau latéral — l'un lisant la teinte du thème, les
    /// deux autres l'écrivant en dur. Trois modules avaient chacun
    /// choisi comment se peint un verdict, ce qui est exactement ce que
    /// la frontière doit leur retirer.
    ///
    /// Ce que le module dit, et c'est tout : le NOM de ce qu'on
    /// vérifie, son ÉTAT en mots — « complet », « partiel »,
    /// « dépouillé », `0x401000` —, le VERDICT, et POURQUOI cela
    /// compte. La marque, sa couleur, et la place où se lit le pourquoi
    /// sont de la présentation.
    ///
    /// Le verdict est un [`Ton`] et non un booléen : un RELRO
    /// « partiel » n'est ni bon ni mauvais, et le dire en vrai/faux
    /// obligeait `protections` à trancher — il peignait donc « partiel »
    /// en rouge, au même rang qu'absent.
    Verdict {
        nom: Ligne,
        état: Ligne,
        verdict: Ton,
        /// Ce que cet état change concrètement. Une case rouge sans
        /// explication n'apprend rien à qui découvre — et c'est la
        /// moitié de ce que ce projet a à offrir.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        pourquoi: Option<String>,
    },
}

/// Ce qu'un module donne à dessiner, en valeurs.
///
/// Un enum fermé et non un sac d'octets : les formes sont connues —
/// elles ont été comptées avant d'être écrites — et une variante
/// nouvelle doit obliger chaque peintre à dire ce qu'il en fait. Un
/// `serde_json::Value` aurait laissé passer n'importe quoi en silence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Dessin {
    /// Un profil : une valeur par tranche, et ce que pèse une tranche.
    ///
    /// L'échelle n'est pas dite ici : c'est au peintre de savoir que
    /// l'entropie va de zéro à huit bits par octet. Le module donne des
    /// mesures, pas une mise en page.
    Profil {
        valeurs: Vec<f64>,
        /// Combien d'octets du fichier chaque valeur résume.
        taille_tranche: usize,
        /// La taille mesurée, en octets.
        ///
        /// Dite, et non déduite de `valeurs.len() * taille_tranche` :
        /// la DERNIÈRE tranche est plus courte que les autres, et la
        /// déduction plaçait la fin du fichier cinquante-six octets
        /// trop loin. La règle d'adresses sous le profil s'en trouvait
        /// graduée sur une taille qui n'existe pas — vu sur la capture,
        /// `0x2300` au lieu de `0x22c8`.
        total: usize,
    },
    /// Un graphe orienté : des boîtes, et qui mène à qui.
    ///
    /// **Rien du placement n'est dit** — ni position, ni taille, ni
    /// indentation, ni le pas qui sépare deux boîtes. Ce sont des
    /// pixels, et les pixels dépendent de la surface dont on dispose et
    /// de ce que mesure un caractère : l'interface le sait, le module
    /// non. Ce qu'il dit est la STRUCTURE — des nœuds dans l'ordre où
    /// on les lit, leur distance à la racine, et les arêtes entre eux.
    Graphe {
        /// Dans l'ordre de lecture : la racine, puis sa descendance.
        nœuds: Vec<Nœud>,
        /// `(de, à)`, par rang dans `nœuds`.
        ///
        /// Dites, et non déduites des profondeurs : deux nœuds voisins
        /// de profondeurs 1 et 2 ne sont pas forcément parent et
        /// enfant, et c'est l'appel qui est le fait — pas la place que
        /// le nœud occupe dans la liste.
        arêtes: Vec<(usize, usize)>,
    },
}

/// Une boîte d'un graphe : ce qu'elle est, ce qu'elle dit, où elle mène.
///
/// Les trois compartiments d'une classe UML, parce que c'est la forme
/// que le graphe d'appels a prise : ce que la fonction EST, son nom, et
/// ce qu'on en mesure. Le dessin — le cadre à angles droits, le filet
/// entre les compartiments, les guillemets du stéréotype — appartient
/// au peintre ; ici il n'y a que du texte et un fait.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Nœud {
    /// Ce que la boîte EST, en un mot : « entrée », « main »,
    /// « renvoi ». Déjà traduit — c'est du contenu, pas un drapeau.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stéréotype: Option<String>,
    pub titre: String,
    /// Les lignes du compartiment du bas : ce qu'on mesure de ce nœud.
    pub corps: Vec<String>,
    /// Où mène un clic sur la boîte, quand il mène quelque part.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mène_à: Option<u64>,
    /// Combien de pas depuis la racine.
    pub profondeur: usize,
    /// `Fort` : par où l'on entre dans le programme — ça se voit en
    /// premier. `Faible` : déjà déplié ailleurs, et non redéployé ici.
    /// `Normal` : une fonction comme les autres.
    pub ton: Ton,
}

/// Un champ de saisie : ce qu'il demande, et de quoi il a l'air.
///
/// `action` est le nom de ce que la validation demande — la valeur
/// tapée le suit, séparée par un deux-points : « graphe:aller:0x401136 ».
/// Un NOM et non la demande elle-même, pour la même raison que partout
/// ailleurs dans ce fichier : la vue reste du texte, et c'est le module
/// qui sait ce qu'« aller » veut dire chez lui.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Champ {
    pub action: String,
    /// Ce qui s'écrit à gauche du champ.
    pub étiquette: Ligne,
    /// Ce que le champ contient — et ce qui remplace ce qu'on a tapé
    /// quand le module en change d'avis.
    ///
    /// C'est ainsi que « Défaut » rend son nom d'origine à une fonction
    /// sans que le module tienne le tampon de frappe : il propose une
    /// autre valeur, et l'interface voit que la proposition a changé.
    pub valeur: String,
    /// Ce qui s'écrit en gris dans un champ vide : un EXEMPLE de ce
    /// qu'on y attend, jamais une valeur qu'on croirait déjà posée.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub indice: Option<String>,
    /// En chasse fixe : une adresse, des octets, un nom de symbole.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub code: bool,
    /// Combien de caractères le champ doit pouvoir montrer.
    ///
    /// Des CARACTÈRES et non des pixels : le module sait ce qu'on écrit
    /// dans son champ — « 0x401136 » en fait huit —, l'interface sait
    /// ce que ses caractères mesurent. Les largeurs en dur des modules
    /// d'avant P7 — 90, 110, 120, 140, 160, 200, 220 pixels — étaient
    /// sept réglages pour une seule question.
    pub caractères: usize,
    /// Ce qui se pose à droite du champ, sur la même rangée.
    ///
    /// Des boutons le plus souvent : celui qui porte la MÊME action que
    /// le champ le valide, tout comme la touche Entrée ; les autres
    /// font ce que leur action dit. Pas de règle de rang — « le premier
    /// valide » serait une convention invisible dans les données.
    ///
    /// Mais un fragment SANS action y tient aussi sa place, et c'est le
    /// panneau des fonctions qui l'a demandé : la rangée qu'on y édite
    /// porte le champ du nom, la flèche qui se dédit, l'adresse qui
    /// mène au listing, la taille et le nombre d'appels. Sans quoi une
    /// ligne perdrait la moitié de ce qu'elle dit dès qu'on clique
    /// dedans.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub boutons: Vec<Fragment>,
    /// Ce champ vient de paraître pour qu'on y écrive : il prend le
    /// clavier.
    ///
    /// Un FAIT du module — « je viens d'ouvrir ceci à votre demande »
    /// —, et non une manœuvre : c'est le même genre d'énoncé que
    /// l'`ouvert` d'un repliable, qui ne dit que la position de départ.
    /// L'interface en tire le focus, et seulement à la première image
    /// où le champ existe ; le redemander à chaque image reprendrait le
    /// clavier à qui serait allé ailleurs.
    ///
    /// Sans lui, cliquer un nom de fonction ouvrait une case où il
    /// fallait ensuite cliquer pour taper — deux gestes là où il y en
    /// avait un.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub clavier: bool,
}

impl Champ {
    /// Un champ qui demandera `action` quand on le validera.
    pub fn pour(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            // Vingt caractères : de quoi écrire une adresse de
            // soixante-quatre bits et son `0x`, ce qui est le contenu
            // le plus fréquent de ces champs.
            caractères: 20,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn étiquette(mut self, étiquette: impl Into<Ligne>) -> Self {
        self.étiquette = étiquette.into();
        self
    }

    #[must_use]
    pub fn valeur(mut self, valeur: impl Into<String>) -> Self {
        self.valeur = valeur.into();
        self
    }

    #[must_use]
    pub fn indice(mut self, indice: impl Into<String>) -> Self {
        self.indice = Some(indice.into());
        self
    }

    #[must_use]
    pub fn code(mut self) -> Self {
        self.code = true;
        self
    }

    #[must_use]
    pub fn caractères(mut self, combien: usize) -> Self {
        self.caractères = combien;
        self
    }

    /// Ajoute un bouton à droite du champ.
    #[must_use]
    pub fn bouton(mut self, bouton: Fragment) -> Self {
        self.boutons.push(bouton);
        self
    }

    /// Ce champ vient de paraître pour qu'on y écrive.
    #[must_use]
    pub fn clavier(mut self) -> Self {
        self.clavier = true;
        self
    }
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
    /// La page qui fait AUTORITÉ sur ce que ce fragment nomme.
    ///
    /// Une seule dans tout le crate — `man 2 write` sous la fiche d'un
    /// appel système —, et c'est délibérément une DONNÉE et non une
    /// forme. Un relevé à un seul usage est ailleurs un signal d'arrêt
    /// (voir l'éditeur binaire, sa case à cocher et ses vingt-six
    /// touches) ; la différence est qu'une adresse de page n'apprend
    /// rien au vocabulaire sur la façon de peindre. Elle se range à
    /// côté de `mène_à` : l'une mène dans le fichier, l'autre hors de
    /// namsdec, et les deux sont du texte qu'un rapport veut garder —
    /// « nous n'avons pas recopié `man 2 write`, voici où il est ».
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub renvoi: Option<String>,
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
    /// Ce fragment s'écrit en **chasse fixe** : une adresse, un octet,
    /// un mnémonique, une empreinte, un nom de symbole.
    ///
    /// UN CHAMP À PART, ET NON UN TON, parce que ce sont deux axes et
    /// non un seul : une signature de fonction est en chasse fixe ET
    /// atténuée, un nom de fonction en chasse fixe ET fort. Tant que
    /// « code » était un ton, `.code().faible()` ne disait que
    /// « faible » — le second appel écrasait le premier, en silence.
    /// Trois endroits l'écrivaient, dont deux dans le panneau de
    /// décompilation : une signature C s'y peignait en chasse
    /// proportionnelle, ce qu'aucune relecture n'avait attrapé.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub chasse_fixe: bool,
    /// Ce fragment porte quelque chose que l'utilisateur a fait **à la
    /// main** — un nom qu'il a donné, un octet qu'il a changé.
    ///
    /// Un FAIT et non un fond : dans une liste qui peut compter des
    /// centaines de fonctions, ce qu'on a nommé soi-même doit se
    /// retrouver d'un coup d'œil, et c'est l'interface qui décide
    /// comment — un fond léger aujourd'hui. Deux panneaux le font déjà
    /// de leur côté (le graphe d'appels et les fonctions), chacun avec
    /// sa teinte écrite sur place : c'est exactement ce que la
    /// frontière doit leur retirer.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub à_la_main: bool,
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
    ///
    /// Se cumule avec un ton, et c'est tout l'intérêt :
    /// `.code().faible()` dit bien les deux.
    #[must_use]
    pub fn code(mut self) -> Self {
        self.chasse_fixe = true;
        self
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

    /// La page qui fait autorité sur ce que ce fragment nomme.
    #[must_use]
    pub fn renvoi(mut self, url: impl Into<String>) -> Self {
        self.renvoi = Some(url.into());
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

    /// Marque ce fragment comme venant de la main de l'utilisateur.
    #[must_use]
    pub fn à_la_main(mut self) -> Self {
        self.à_la_main = true;
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
        assert!(!json.contains("à_la_main"), "{json}");
        assert!(!json.contains("chasse_fixe"), "{json}");
    }

    /// Un champ et un graphe font l'aller-retour comme le reste.
    ///
    /// Les deux formes ajoutées pour le graphe d'appels : un champ de
    /// saisie, et un dessin qui n'est plus une courbe mais des boîtes.
    /// Si l'aller-retour en perdait une part, la vue ne sortirait plus
    /// de namsdec entière — ce qui est la seule raison d'être de ce
    /// fichier.
    #[test]
    fn un_champ_et_un_graphe_font_l_aller_retour() {
        let vue = Vue::neuve()
            .bloc(Bloc::Champ(
                Champ::pour("graphe:aller")
                    .étiquette("Aller à :")
                    .indice("0x401000")
                    .code()
                    .caractères(20)
                    .bouton(Fragment::dit("Y aller").bouton("graphe:aller")),
            ))
            .bloc(Bloc::Dessin(Dessin::Graphe {
                nœuds: vec![
                    Nœud {
                        stéréotype: Some("entrée".to_owned()),
                        titre: "_start".to_owned(),
                        corps: vec!["0x401000 · 32 octets".to_owned()],
                        mène_à: Some(0x401000),
                        profondeur: 0,
                        ton: Ton::Fort,
                    },
                    Nœud {
                        titre: "sub_401136".to_owned(),
                        profondeur: 1,
                        ..Nœud::default()
                    },
                ],
                arêtes: vec![(0, 1)],
            }))
            .bloc(Bloc::Repliable {
                titre: "sub_401136".into(),
                ouvert: false,
                blocs: vec![Bloc::Code("return 0;".to_owned())],
            });
        let json = serde_json::to_string(&vue).expect("la vue se sérialise");
        let relue: Vue = serde_json::from_str(&json).expect("et se relit");
        assert_eq!(vue, relue);
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
