//! L'entropie : ce qui distingue du texte, du code, et du chiffré.
//!
//! Un octet pris au hasard dans un fichier n'apprend rien. La
//! *distribution* des octets, elle, dit beaucoup : du texte anglais
//! n'emploie qu'une cinquantaine de valeurs sur 256 et en répète
//! quelques-unes sans cesse ; du code x86 a ses propres habitudes
//! — `48` devant presque toutes les instructions 64 bits, `e8` devant
//! chaque appel ; des données compressées ou chiffrées, elles, n'ont
//! aucune habitude, et c'est justement ce qui les trahit.
//!
//! L'entropie de Shannon met un nombre là-dessus : **combien de bits
//! d'information porte un octet, en moyenne**. Zéro quand tous les
//! octets sont identiques, huit quand ils sont parfaitement imprévisibles.
//!
//! C'est la mesure qu'on lance en premier sur un binaire inconnu, parce
//! qu'elle répond en une image à « où est le vrai code ? » — un
//! empaqueteur laisse un petit déchiffreur en clair et un gros bloc à 7,9.

/// L'entropie de Shannon d'un bloc d'octets, en bits par octet.
///
/// Entre 0 et 8. Un bloc vide vaut zéro : il n'y a aucune information
/// dans rien, et c'est la seule réponse qui ne suppose pas.
pub fn shannon(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut compte = [0usize; 256];
    for b in bytes {
        compte[*b as usize] += 1;
    }
    let total = bytes.len() as f64;
    -compte
        .iter()
        .filter(|n| **n > 0)
        .map(|n| {
            let p = *n as f64 / total;
            p * p.log2()
        })
        .sum::<f64>()
}

/// Les seuils qui séparent les verdicts, et la mesure qui les justifie.
///
/// Ils étaient écrits en clair dans le `match` de [`Verdict::of`], et la
/// ligne de repère du panneau en recopiait un troisième. Trois nombres
/// dispersés que personne ne pouvait régler sans relire l'algorithme —
/// et qu'un lecteur pouvait prendre pour des vérités plutôt que pour des
/// repères. Les voici réunis, chacun avec ce qu'on a mesuré pour le
/// poser.
///
/// Aucun n'est une frontière de la nature : ce sont des bornes
/// *ordinaires*, prises sur des fichiers ordinaires. Un petit bloc de
/// code très dense franchit [`DENSE_CODE`] sans être chiffré pour
/// autant, et c'est pourquoi le panneau dit « une piste, non un
/// verdict ».
pub mod seuils {
    /// En dessous, presque rien ne varie : du remplissage, des zéros,
    /// une table d'adresses alignées. Mesuré : une page de zéros vaut 0,
    /// une table de pointeurs consécutifs tourne autour de 1.
    pub const PLAT: f64 = 1.5;

    /// En dessous, la distribution d'un texte lisible : l'ASCII
    /// n'emploie qu'une cinquantaine de valeurs sur 256. Mesuré : du
    /// français ou de l'anglais ordinaire tourne entre 4,0 et 4,5.
    pub const TEXTE: f64 = 5.0;

    /// En dessous, du code machine ordinaire. Mesuré : le
    /// `.text` d'un binaire compilé sans optimisation agressive tourne
    /// entre 5,5 et 6,5.
    pub const CODE: f64 = 6.8;

    /// Au-delà, il n'y a plus de structure à voir : compressé, chiffré,
    /// ou déjà empaqueté. C'est le repère que le profil trace en
    /// travers de l'histogramme, et celui que le panneau « Signaux »
    /// relève. Mesuré : un bloc `gzip` ou AES tourne entre 7,9 et 8,0 ;
    /// entre [`CODE`] et ce seuil, on ne sait pas trancher.
    pub const EMPAQUETÉ: f64 = 7.2;

    /// Le chiffre qu'on cite quand on prévient : un bloc de code dense
    /// atteint cette valeur sans rien cacher. Il ne sert à aucune
    /// décision — seulement à la phrase qui tempère les autres.
    pub const DENSE_CODE: f64 = 6.5;
}

/// Ce qu'une valeur d'entropie suggère.
///
/// Des seuils, donc des approximations — et il faut le dire : un petit
/// bloc de code bien dense monte à 6,5 sans être chiffré pour autant.
/// C'est une piste, pas un verdict, et le libellé le formule ainsi. Les
/// bornes et leur justification sont dans [`seuils`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Presque rien ne varie : du remplissage, des zéros, une table.
    Flat,
    /// La distribution d'un texte lisible.
    Text,
    /// Celle d'un code machine ordinaire.
    Code,
    /// Trop régulier pour du code, trop varié pour du texte.
    Mixed,
    /// Aucune habitude : compressé, chiffré, ou déjà empaqueté.
    Random,
}

impl Verdict {
    /// Le verdict qui correspond à cette entropie.
    ///
    /// Les bornes, et ce qu'on a mesuré pour les poser, sont dans
    /// [`seuils`] : les lire là plutôt qu'ici permet de les régler sans
    /// relire ce `match`.
    pub fn of(entropy: f64) -> Self {
        match entropy {
            e if e < seuils::PLAT => Verdict::Flat,
            e if e < seuils::TEXTE => Verdict::Text,
            e if e < seuils::CODE => Verdict::Code,
            e if e < seuils::EMPAQUETÉ => Verdict::Mixed,
            _ => Verdict::Random,
        }
    }

    /// Le verdict en un mot, pour une colonne de tableau.
    ///
    /// La phrase entière — « uniforme — remplissage, zéros, une table »
    /// — explique, et c'est ce qu'il faut sous la légende ; dans une
    /// table à cinq colonnes, elle court au-delà du bord du panneau.
    pub fn short(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Verdict::Flat, true) => "uniforme",
            (Verdict::Flat, false) => "flat",
            (Verdict::Text, true) => "texte",
            (Verdict::Text, false) => "text",
            (Verdict::Code, true) => "code",
            (Verdict::Code, false) => "code",
            (Verdict::Mixed, true) => "mêlé",
            (Verdict::Mixed, false) => "mixed",
            (Verdict::Random, true) => "aléa",
            (Verdict::Random, false) => "random",
        }
    }

    pub fn label(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Verdict::Flat, true) => "uniforme — remplissage, zéros, une table",
            (Verdict::Flat, false) => "flat — padding, zeros, a table",
            (Verdict::Text, true) => "du texte, ou des données très régulières",
            (Verdict::Text, false) => "text, or very regular data",
            (Verdict::Code, true) => "du code machine",
            (Verdict::Code, false) => "machine code",
            (Verdict::Mixed, true) => "mêlé — du code et des données",
            (Verdict::Mixed, false) => "mixed — code and data",
            (Verdict::Random, true) => "compressé, chiffré, ou empaqueté",
            (Verdict::Random, false) => "compressed, encrypted, or packed",
        }
    }
}

/// Une tranche de fichier, et ce qu'elle contient.
#[derive(Debug, Clone, Copy)]
pub struct Window {
    /// Le décalage du début de la tranche dans le fichier.
    pub offset: usize,
    pub len: usize,
    pub entropy: f64,
}

/// Découpe `bytes` en tranches et mesure chacune.
///
/// Une seule entropie pour tout un fichier ne sert à rien : elle mélange
/// l'en-tête, le code, les chaînes et le remplissage, et rend une moyenne
/// qui ne décrit aucun d'entre eux. C'est le *profil* qu'on veut — la
/// suite des mesures, où une bosse saute aux yeux.
///
/// `count` tranches au plus, d'au moins `MIN_WINDOW` octets : mesurer
/// l'entropie de trente-deux octets rendrait un nombre sans valeur,
/// puisqu'il n'y a pas de quoi voir une habitude.
pub fn profile(bytes: &[u8], count: usize) -> Vec<Window> {
    if bytes.is_empty() || count == 0 {
        return Vec::new();
    }
    let taille = bytes.len().div_ceil(count).max(MIN_WINDOW);
    bytes
        .chunks(taille)
        .enumerate()
        .map(|(i, tranche)| Window {
            offset: i * taille,
            len: tranche.len(),
            entropy: shannon(tranche),
        })
        .collect()
}

/// La taille de tranche qui convient à un fichier de cette taille.
///
/// **Compartimenter selon la taille du binaire**, et non découper en
/// soixante-quatre quoi qu'il arrive : sur un fichier d'un méga-octet,
/// soixante-quatre tranches font seize kilo-octets chacune, et un
/// déchiffreur de deux cents octets se noie dans la moyenne de sa
/// tranche. Le profil disait alors « rien à signaler » là où il y avait
/// tout à voir.
///
/// La règle : viser [`TRANCHES_VISÉES`] tranches, arrondir la taille à
/// la puissance de deux au-dessus — un découpage en 4 096 octets se lit
/// et se retrouve dans un vidage, un découpage en 3 277 non —, et ne
/// jamais descendre sous [`MIN_WINDOW`], en deçà de quoi la mesure ne
/// veut rien dire.
pub fn window_size(len: usize) -> usize {
    if len == 0 {
        return MIN_WINDOW;
    }
    let visée = len.div_ceil(TRANCHES_VISÉES).max(MIN_WINDOW);
    // La puissance de deux au-dessus, bornée : au-delà, les tranches
    // redeviennent des moyennes aveugles.
    visée.next_power_of_two().min(MAX_WINDOW)
}

/// Combien de tranches on vise, quelle que soit la taille du fichier.
///
/// Assez pour qu'un bloc de quelques centaines d'octets ressorte dans
/// un binaire ordinaire, assez peu pour que le dessin reste une forme
/// et non un peigne.
pub const TRANCHES_VISÉES: usize = 512;

/// En deçà, la mesure ne veut rien dire : 256 octets, c'est déjà peu
/// pour juger d'une distribution sur 256 valeurs possibles.
pub const MIN_WINDOW: usize = 256;

/// Au-delà, une tranche n'est plus une tranche mais une moyenne.
pub const MAX_WINDOW: usize = 64 * 1024;

/// Découpe `bytes` en tranches de `window` octets et mesure chacune.
///
/// La forme que le panneau emploie : c'est la TAILLE de la tranche
/// qu'on choisit — elle se dit en octets, se lit dans un vidage et se
/// retrouve dans le fichier —, et non leur nombre.
pub fn profile_by(bytes: &[u8], window: usize) -> Vec<Window> {
    let taille = window.max(1);
    if bytes.is_empty() {
        return Vec::new();
    }
    bytes
        .chunks(taille)
        .enumerate()
        .map(|(i, tranche)| Window {
            offset: i * taille,
            len: tranche.len(),
            entropy: shannon(tranche),
        })
        .collect()
}

/// Les tranches voisines qui disent la même chose, réunies en plages.
///
/// C'est la forme qu'on lit : « de 0x0 à 0x2000, du code ; de 0x2000 à
/// 0x2c00, de l'aléa ». Trois cents tranches ne se lisent pas une à
/// une, et c'est pourtant là que sont les adresses qu'on veut.
///
/// Deux temps, et le second est indispensable depuis que les tranches
/// sont fines : réunir ce qui porte le même verdict, puis **absorber
/// les miettes**. Un gros bloc chiffré oscille autour du seuil
/// d'empaquetage — 7,13 puis 7,25 puis 7,16 —, et le premier temps,
/// seul, le découpait en quatorze plages d'une tranche là où il n'y a
/// qu'une chose à voir. Une plage sous [`MIN_TRANCHES`] tranches
/// rejoint donc sa voisine la plus proche en entropie, et le verdict
/// de la plage ainsi formée se relit sur sa moyenne.
pub fn ranges(windows: &[Window]) -> Vec<Range> {
    let mut plages: Vec<Range> = Vec::new();
    for fenêtre in windows {
        let verdict = Verdict::of(fenêtre.entropy);
        match plages.last_mut() {
            Some(plage) if plage.verdict == verdict => {
                plage.len += fenêtre.len;
                plage.somme += fenêtre.entropy * fenêtre.len as f64;
                plage.haute = plage.haute.max(fenêtre.entropy);
            }
            _ => plages.push(Range {
                offset: fenêtre.offset,
                len: fenêtre.len,
                verdict,
                somme: fenêtre.entropy * fenêtre.len as f64,
                haute: fenêtre.entropy,
            }),
        }
    }

    // Les miettes rejoignent leur voisine, et ce que l'absorption rend
    // identique se réunit à son tour : une miette avalée change la
    // moyenne de sa voisine, donc parfois son verdict, et deux plages
    // qui disent désormais la même chose n'ont plus de raison d'être
    // deux.
    let tranche = windows.first().map_or(1, |f| f.len.max(1));
    let minimum = tranche * MIN_TRANCHES;
    loop {
        let avant = plages.len();
        réunir_les_pareilles(&mut plages);
        absorber_la_plus_petite(&mut plages, minimum);
        if plages.len() == avant {
            break;
        }
    }
    plages
}

/// Réunit les plages voisines qui portent le même verdict.
fn réunir_les_pareilles(plages: &mut Vec<Range>) {
    let mut rang = 1;
    while rang < plages.len() {
        if plages[rang].verdict == plages[rang - 1].verdict {
            let suivante = plages.remove(rang);
            let plage = &mut plages[rang - 1];
            plage.len += suivante.len;
            plage.somme += suivante.somme;
            plage.haute = plage.haute.max(suivante.haute);
            plage.verdict = Verdict::of(plage.entropy());
        } else {
            rang += 1;
        }
    }
}

/// Fait rejoindre à la plus petite plage trop courte sa voisine la plus
/// proche en entropie. Une seule à la fois : chaque fusion change le
/// voisinage, et la suivante doit se décider sur le nouveau.
fn absorber_la_plus_petite(plages: &mut Vec<Range>, minimum: usize) {
    if plages.len() < 2 {
        return;
    }
    // La plus petite d'abord : absorber dans l'ordre du fichier ferait
    // grossir la première par la gauche, quel que soit son voisinage.
    let Some(rang) = plages
        .iter()
        .enumerate()
        .filter(|(_, plage)| plage.len < minimum)
        .min_by_key(|(_, plage)| plage.len)
        .map(|(rang, _)| rang)
    else {
        return;
    };
    // Celle des deux voisines dont la moyenne est la plus proche : une
    // miette de 7,25 appartient au bloc d'aléa d'à côté, non à la zone
    // plate d'en face.
    let avant = rang.checked_sub(1);
    let après = (rang + 1 < plages.len()).then_some(rang + 1);
    let écart = |voisine: Option<usize>| -> Option<f64> {
        voisine.map(|v| (plages[v].entropy() - plages[rang].entropy()).abs())
    };
    let cible = match (écart(avant), écart(après)) {
        (Some(a), Some(b)) if a <= b => avant,
        (Some(_), Some(_)) => après,
        (Some(_), None) => avant,
        (None, Some(_)) => après,
        (None, None) => None,
    };
    let Some(cible) = cible else {
        return;
    };
    let miette = plages.remove(rang);
    let cible = if cible > rang { cible - 1 } else { cible };
    let voisine = &mut plages[cible];
    voisine.offset = voisine.offset.min(miette.offset);
    voisine.len += miette.len;
    voisine.somme += miette.somme;
    voisine.haute = voisine.haute.max(miette.haute);
    voisine.verdict = Verdict::of(voisine.entropy());
}

/// Combien de tranches il faut pour qu'une plage en soit une.
///
/// En deçà, c'est une oscillation autour d'un seuil, non un morceau de
/// fichier : la nommer à part n'apprend rien et noie les vraies.
pub const MIN_TRANCHES: usize = 4;

/// Une suite de tranches qui disent la même chose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub offset: usize,
    pub len: usize,
    pub verdict: Verdict,
    /// La somme des entropies pondérées par la longueur : sert à rendre
    /// la moyenne sans la recalculer.
    somme: f64,
    /// La tranche la plus dense de la plage.
    pub haute: f64,
}

impl Range {
    /// L'entropie moyenne de la plage.
    pub fn entropy(&self) -> f64 {
        if self.len == 0 {
            return 0.0;
        }
        self.somme / self.len as f64
    }

    pub fn end(&self) -> usize {
        self.offset + self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La tranche grandit avec le fichier, et jamais n'importe comment.
    ///
    /// C'est tout l'objet du découpage adaptatif : soixante-quatre
    /// tranches sur un méga-octet noient un déchiffreur de deux cents
    /// octets dans une moyenne de seize kilo-octets.
    #[test]
    fn the_window_grows_with_the_file_and_stays_round() {
        // Un petit fichier garde la tranche la plus fine qui ait un
        // sens.
        assert_eq!(window_size(0), MIN_WINDOW);
        assert_eq!(window_size(1024), MIN_WINDOW);
        assert_eq!(window_size(100 * 1024), MIN_WINDOW);
        // Au-delà, elle double à mesure — et reste une puissance de
        // deux, qui se retrouve dans un vidage.
        for taille in [1 << 20, 4 << 20, 64 << 20, 512 << 20] {
            let fenêtre = window_size(taille);
            assert!(fenêtre.is_power_of_two(), "{taille} → {fenêtre}");
            assert!((MIN_WINDOW..=MAX_WINDOW).contains(&fenêtre), "{fenêtre}");
            let tranches = taille.div_ceil(fenêtre);
            assert!(
                tranches >= 64,
                "{taille} octets ne doivent pas tenir en {tranches} tranches"
            );
        }
        // Et jamais au-delà de la borne, même sur un fichier énorme.
        assert_eq!(window_size(usize::MAX / 2), MAX_WINDOW);
    }

    /// Les plages réunissent ce qui dit la même chose, sans trou.
    #[test]
    fn the_ranges_join_what_says_the_same_thing() {
        let mut octets = vec![0u8; 4096];
        let mut graine = 0x9e3779b97f4a7c15u64;
        for _ in 0..4096 {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            octets.push((graine >> 24) as u8);
        }
        let tranches = profile_by(&octets, 512);
        let plages = ranges(&tranches);
        assert!(plages.len() >= 2, "{plages:?}");
        // Elles se suivent et couvrent tout.
        assert_eq!(plages[0].offset, 0);
        for paire in plages.windows(2) {
            assert_eq!(paire[0].end(), paire[1].offset);
            // Et deux voisines ne disent jamais la même chose : sinon
            // c'est qu'il fallait les réunir.
            assert_ne!(paire[0].verdict, paire[1].verdict, "{plages:?}");
            // Aucune miette : une plage d'une tranche est une
            // oscillation autour d'un seuil, non un morceau de fichier.
            assert!(
                paire[0].len >= 512 * MIN_TRANCHES,
                "une plage trop courte : {:?}",
                paire[0]
            );
        }
        assert_eq!(plages.last().expect("une plage").end(), octets.len());
        // La première est plate, la dernière est de l'aléa.
        assert_eq!(plages[0].verdict, Verdict::Flat);
        assert_eq!(
            plages.last().expect("une plage").verdict,
            Verdict::Random,
            "{plages:?}"
        );
        // Et la moyenne d'une plage reste dans l'échelle.
        for plage in &plages {
            assert!((0.0..=8.0).contains(&plage.entropy()), "{plage:?}");
        }
    }

    /// Les seuils se suivent, et chacun rend le verdict qu'il annonce.
    ///
    /// Rassemblés dans un module, ils pouvaient se régler indépendamment
    /// les uns des autres — et un seuil « plat » au-dessus du seuil
    /// « texte » rendrait un `match` dont une branche ne serait jamais
    /// prise, sans que rien ne le dise. Ce contrôle est le garde-fou de
    /// ce réglage-là.
    #[test]
    fn the_thresholds_follow_one_another() {
        use seuils::*;
        // Nommés et rangés dans une suite : un `assert!(PLAT < TEXTE)`
        // compare deux constantes, et le compilateur le tranche avant
        // d'exécuter quoi que ce soit — ce n'est plus un contrôle.
        // Lus par la boucle, ils le redeviennent.
        let bornes = [
            ("PLAT", PLAT),
            ("TEXTE", TEXTE),
            ("CODE", CODE),
            ("EMPAQUETÉ", EMPAQUETÉ),
        ];
        for paire in bornes.windows(2) {
            let ((avant, bas), (après, haut)) = (paire[0], paire[1]);
            assert!(
                bas < haut,
                "{avant} ({bas}) doit rester sous {après} ({haut})"
            );
        }
        let plus_haut = bornes.last().expect("la suite n'est pas vide").1;
        assert!(plus_haut <= 8.0, "une entropie ne dépasse pas huit bits");
        // Le chiffre qu'on cite pour tempérer est bien dans la zone du
        // code : le citer au-delà du seuil d'empaquetage ferait dire au
        // panneau le contraire de ce qu'il veut dire.
        assert!(
            DENSE_CODE < plus_haut,
            "le repère du code dense reste sous celui de l'empaquetage"
        );

        // Et chaque seuil est bien la frontière qu'il prétend être :
        // juste en dessous, le verdict d'avant ; au niveau, le suivant.
        assert_eq!(Verdict::of(PLAT - 0.01), Verdict::Flat);
        assert_eq!(Verdict::of(PLAT), Verdict::Text);
        assert_eq!(Verdict::of(TEXTE), Verdict::Code);
        assert_eq!(Verdict::of(CODE), Verdict::Mixed);
        assert_eq!(Verdict::of(EMPAQUETÉ), Verdict::Random);
    }

    /// Les deux extrêmes, qui bornent tout le reste.
    #[test]
    fn the_two_extremes_are_zero_and_eight() {
        // Rien ne varie : aucune information.
        assert_eq!(shannon(&[0u8; 4096]), 0.0);
        assert_eq!(shannon(&[0x90u8; 100]), 0.0);
        // Les 256 valeurs une fois chacune : le maximum, exactement.
        let toutes: Vec<u8> = (0..=255u8).collect();
        assert!(
            (shannon(&toutes) - 8.0).abs() < 1e-9,
            "{}",
            shannon(&toutes)
        );
        // Et rien du tout vaut zéro, sans diviser par zéro au passage.
        assert_eq!(shannon(&[]), 0.0);
    }

    /// Deux valeurs équiprobables portent exactement un bit.
    ///
    /// C'est la définition même, et la vérifier sur un cas qu'on sait
    /// calculer de tête vaut mieux que de faire confiance à la formule.
    #[test]
    fn two_equal_values_carry_exactly_one_bit() {
        let mut bytes = vec![0u8; 512];
        bytes.extend(std::iter::repeat_n(1u8, 512));
        assert!((shannon(&bytes) - 1.0).abs() < 1e-9, "{}", shannon(&bytes));
    }

    /// Du texte, du code et du bruit ne se ressemblent pas.
    ///
    /// C'est tout l'objet du module : les distinguer sans les lire.
    #[test]
    fn text_code_and_noise_fall_into_different_verdicts() {
        let texte: Vec<u8> = "le petit chat boit du lait tous les matins dans la cuisine "
            .repeat(40)
            .into_bytes();
        assert_eq!(Verdict::of(shannon(&texte)), Verdict::Text);

        // Du remplissage : ce qu'on trouve entre deux sections.
        assert_eq!(Verdict::of(shannon(&[0u8; 2048])), Verdict::Flat);

        // Du bruit : une suite qui ne se répète pas avant longtemps.
        // Un générateur congruentiel suffit — on ne teste pas sa qualité,
        // seulement que l'entropie le voit comme imprévisible.
        let mut graine = 0x2545f4914f6cdd1du64;
        let bruit: Vec<u8> = (0..8192)
            .map(|_| {
                graine ^= graine << 13;
                graine ^= graine >> 7;
                graine ^= graine << 17;
                (graine >> 24) as u8
            })
            .collect();
        assert_eq!(Verdict::of(shannon(&bruit)), Verdict::Random);
    }

    /// Le profil découpe, et couvre tout le fichier sans trou.
    ///
    /// Un profil qui oublierait la fin cacherait exactement ce qu'on
    /// cherche : un bloc chiffré rangé en queue de fichier.
    #[test]
    fn the_profile_covers_the_whole_file() {
        let octets: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let tranches = profile(&octets, 16);
        assert!(!tranches.is_empty());
        let couvert: usize = tranches.iter().map(|w| w.len).sum();
        assert_eq!(couvert, octets.len(), "aucun octet ne doit manquer");
        // Et les tranches se suivent, sans recouvrement.
        for paire in tranches.windows(2) {
            assert_eq!(paire[0].offset + paire[0].len, paire[1].offset);
        }
    }

    /// Une bosse de bruit au milieu d'un fichier calme se voit.
    ///
    /// C'est le cas qui justifie le profil plutôt qu'une mesure unique :
    /// la moyenne du fichier entier ne dirait rien, la tranche le crie.
    #[test]
    fn a_block_of_noise_stands_out_in_the_profile() {
        let mut octets = vec![0u8; 4096];
        let mut graine = 0x9e3779b97f4a7c15u64;
        for i in 4096..8192 {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            octets.push((graine >> 24) as u8);
            let _ = i;
        }
        octets.extend(std::iter::repeat_n(0u8, 4096));

        let tranches = profile(&octets, 12);
        let plus_haute = tranches
            .iter()
            .max_by(|a, b| a.entropy.total_cmp(&b.entropy))
            .expect("il y a des tranches");
        assert!(
            plus_haute.entropy > 7.0,
            "la bosse doit ressortir : {plus_haute:?}"
        );
        // Et elle est bien au milieu, là où on l'a mise.
        assert!(
            (4096..8192).contains(&plus_haute.offset),
            "au mauvais endroit : {plus_haute:?}"
        );
        // Tandis que la moyenne du tout ne dirait rien de tranché.
        assert!(
            Verdict::of(shannon(&octets)) != Verdict::Random,
            "c'est justement pourquoi une mesure unique ne suffit pas"
        );
    }

    /// Aucune taille ne fait tomber le découpage.
    #[test]
    fn no_size_breaks_the_profile() {
        for taille in [0usize, 1, 2, 255, 256, 257, 1000, 4096] {
            let octets = vec![0x41u8; taille];
            for tranches in [0usize, 1, 7, 64, 1000] {
                let profil = profile(&octets, tranches);
                let couvert: usize = profil.iter().map(|w| w.len).sum();
                assert!(
                    couvert == taille || profil.is_empty(),
                    "taille {taille}, {tranches} tranches : {couvert} octets couverts"
                );
            }
        }
    }
}
