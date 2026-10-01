//! Nommer l'empaqueteur, au lieu de le deviner.
//!
//! L'entropie dit « compressé, chiffré, ou empaqueté » : c'est juste, et
//! c'est frustrant. Un cran plus loin, la plupart des empaqueteurs se
//! **nomment** eux-mêmes — ils laissent un nom de section, un nombre
//! magique, une chaîne de version, parce qu'il leur faut bien se
//! retrouver pour se déballer. Les reconnaître change « il y a quelque
//! chose d'illisible ici » en « c'est de l'UPX, et `upx -d` le rendra ».
//!
//! Lecture seule, bornée, et **sans deviner** : on ne nomme un
//! empaqueteur que sur une marque qui n'appartient qu'à lui. Un gros
//! bloc à 7,9 bits n'est pas une marque — c'est ce que l'entropie dit
//! déjà, et le dire deux fois n'ajoute rien.

/// Ce qui a permis de reconnaître l'empaqueteur.
///
/// Montré avec le nom, toujours : « c'est de l'UPX » sans dire pourquoi
/// demande de faire confiance, et un outil pédagogique n'a pas à
/// demander ça. La preuve se vérifie à la main, dans le vidage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clue {
    /// Une section porte un nom qui n'est à personne d'autre.
    Section(String),
    /// Une suite d'octets reconnaissable, et où elle se trouve.
    Marker { text: String, offset: usize },
}

impl Clue {
    /// La preuve, écrite pour être relue dans le vidage.
    pub fn label(&self, lang: &str) -> String {
        let fr = lang.starts_with("fr");
        match self {
            Clue::Section(nom) => {
                if fr {
                    format!("section « {nom} »")
                } else {
                    format!("section “{nom}”")
                }
            }
            Clue::Marker { text, offset } => {
                if fr {
                    format!("« {text} » à {offset:#x}")
                } else {
                    format!("“{text}” at {offset:#x}")
                }
            }
        }
    }
}

/// Un empaqueteur reconnu, et ce qui l'a trahi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Le nom tel qu'on le cherchera : `UPX`, `PyInstaller`.
    pub name: &'static str,
    /// Ce qu'on en fait, dit en une phrase — c'est la vraie valeur de la
    /// reconnaissance.
    pub fr: &'static str,
    pub en: &'static str,
    pub clue: Clue,
}

impl Found {
    pub fn text(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.fr
        } else {
            self.en
        }
    }
}

/// Une marque à chercher : les octets, et où les chercher.
struct Marque {
    name: &'static str,
    bytes: &'static [u8],
    /// La position exacte, quand la marque n'en a qu'une.
    ///
    /// Un nombre magique posé à un décalage fixe se vérifie là et
    /// nulle part ailleurs : le chercher dans tout le fichier le
    /// trouverait par hasard dans des données, et nommerait un
    /// empaqueteur à tort. `None` quand la marque est un nom propre
    /// qu'on peut rencontrer n'importe où.
    at: Option<usize>,
    fr: &'static str,
    en: &'static str,
}

/// Les marques qu'on cherche dans le fichier.
///
/// Chacune est un nom propre : `UPX!` n'est le nombre magique de rien
/// d'autre, `MEI\x0c\x0b\x0a\x0b\x0e` est le cookie que PyInstaller pose
/// en queue de fichier. Rien de générique ici — pas de « packed », pas
/// de « compressed » : ces mots-là sont dans un programme sur dix sans
/// rien vouloir dire.
const MARQUES: &[Marque] = &[
    Marque {
        name: "UPX",
        bytes: b"UPX!",
        at: None,
        fr: "Le plus répandu des empaqueteurs libres. `upx -d fichier` rend \
             l'original : c'est de la compression, non du chiffrement, et il \
             sait se défaire lui-même.",
        en: "The most widespread free packer. `upx -d file` gives the original \
             back: this is compression, not encryption, and it knows how to \
             undo itself.",
    },
    Marque {
        name: "PyInstaller",
        bytes: b"MEI\x0c\x0b\x0a\x0b\x0e",
        at: None,
        fr: "Ce n'est pas du code machine à lire : c'est un interpréteur Python \
             et une archive de `.pyc` collés ensemble. `pyinstxtractor` sort \
             l'archive, et c'est le bytecode Python qu'on décompile ensuite.",
        en: "This is not machine code to read: it is a Python interpreter and an \
             archive of `.pyc` files glued together. `pyinstxtractor` extracts \
             the archive, and it is Python bytecode one decompiles next.",
    },
    Marque {
        name: "AppImage",
        // Le nombre magique du type 2, posé dans les trois octets que
        // l'en-tête ELF laisse libres juste après le sien. Cherché **à
        // cette place-là** et non n'importe où : le mot « AppImage » se
        // trouve dans le texte d'un programme sur dix, ces trois
        // octets-là nulle part ailleurs.
        bytes: b"AI\x02",
        at: Some(8),
        fr: "Une image SquashFS collée derrière un petit lanceur. \
             `--appimage-extract` l'ouvre, et le vrai programme est dedans — \
             ce qu'on lit ici n'est que le lanceur.",
        en: "A SquashFS image glued behind a small launcher. \
             `--appimage-extract` opens it, and the real program is inside — \
             what one reads here is only the launcher.",
    },
];

/// Les noms de section qui nomment leur empaqueteur.
const SECTIONS: &[(&str, &str, &str, &str)] = &[
    (
        "UPX0",
        "UPX",
        "La section vide où UPX dépliera le programme à l'exécution.",
        "The empty section where UPX will unpack the program at run time.",
    ),
    (
        "UPX1",
        "UPX",
        "La section qui porte le programme compressé et le déballeur.",
        "The section carrying the compressed program and the unpacker.",
    ),
];

/// Ce que namsdec reconnaît dans ce fichier, un nom par empaqueteur.
///
/// Un même empaqueteur laisse souvent plusieurs marques ; on n'en garde
/// qu'une — la première trouvée — parce que trois lignes qui disent
/// « UPX » n'apprennent rien de plus que la première.
///
/// La recherche est bornée : le fichier n'est parcouru qu'une fois par
/// marque, et les sections viennent de l'analyse ELF déjà faite.
pub fn detect(bytes: &[u8]) -> Vec<Found> {
    let mut trouvés: Vec<Found> = Vec::new();

    // Les sections d'abord : une section nommée est une marque qu'on ne
    // peut pas confondre avec du contenu qui ressemblerait par hasard.
    if let Some(elf) = crate::elf_scan::scan(bytes) {
        for section in &elf.sections {
            if let Some((nom, empaqueteur, fr, en)) =
                SECTIONS.iter().find(|(nom, ..)| *nom == section.name)
                && !trouvés.iter().any(|f| f.name == *empaqueteur)
            {
                trouvés.push(Found {
                    name: empaqueteur,
                    fr,
                    en,
                    clue: Clue::Section((*nom).to_owned()),
                });
            }
        }
    }

    for marque in MARQUES {
        if trouvés.iter().any(|f| f.name == marque.name) {
            continue;
        }
        let trouvée = match marque.at {
            // À sa place, ou nulle part : un décalage fixe se vérifie,
            // il ne se cherche pas.
            Some(at) => bytes
                .get(at..at + marque.bytes.len())
                .is_some_and(|fenêtre| fenêtre == marque.bytes)
                .then_some(at),
            None => position(bytes, marque.bytes),
        };
        if let Some(offset) = trouvée {
            trouvés.push(Found {
                name: marque.name,
                fr: marque.fr,
                en: marque.en,
                clue: Clue::Marker {
                    text: lisible(marque.bytes),
                    offset,
                },
            });
        }
    }
    trouvés
}

/// Où `aiguille` apparaît dans `botte`, la première fois.
///
/// Écrit à la main et non par une bibliothèque : la recherche naïve
/// suffit pour quatre marques de huit octets, et elle n'ajoute aucune
/// dépendance à un module qui doit rester lisible.
fn position(botte: &[u8], aiguille: &[u8]) -> Option<usize> {
    if aiguille.is_empty() || botte.len() < aiguille.len() {
        return None;
    }
    botte
        .windows(aiguille.len())
        .position(|fenêtre| fenêtre == aiguille)
}

/// La marque, écrite de façon à pouvoir la chercher dans le vidage.
///
/// Les octets imprimables tels quels, les autres en hexadécimal : un
/// cookie comme celui de PyInstaller est à moitié du texte, et le
/// montrer entièrement en hexadécimal le rendrait méconnaissable.
fn lisible(marque: &[u8]) -> String {
    marque
        .iter()
        .map(|b| {
            if (0x20..0x7f).contains(b) {
                (*b as char).to_string()
            } else {
                format!("\\x{b:02x}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un fichier ordinaire n'est pas accusé d'être empaqueté.
    ///
    /// C'est la moitié qui compte : nommer un empaqueteur à tort ferait
    /// chercher un déballeur qui n'existe pas, et perdre une soirée.
    #[test]
    fn an_ordinary_file_is_not_accused() {
        assert!(detect(&[]).is_empty());
        assert!(detect(&[0u8; 4096]).is_empty());
        assert!(detect(b"un programme ordinaire, compresse de rien").is_empty());
        // Le mot d'un format, cité au milieu d'un texte, n'est pas ce
        // format : c'est pourquoi la marque d'AppImage est cherchée à sa
        // place et pas ailleurs.
        assert!(detect(b"ce programme sait lire un AppImage et un .deb").is_empty());
        if let Ok(vrai) = std::fs::read("/bin/true") {
            assert!(detect(&vrai).is_empty(), "/bin/true n'est pas empaqueté");
        }
    }

    /// Le nombre magique d'UPX le nomme, et dit où il est.
    #[test]
    fn the_upx_magic_names_it_and_says_where() {
        let mut fichier = vec![0u8; 256];
        fichier.extend_from_slice(b"UPX!");
        fichier.extend_from_slice(&[0u8; 64]);

        let trouvés = detect(&fichier);
        assert_eq!(trouvés.len(), 1, "{trouvés:?}");
        assert_eq!(trouvés[0].name, "UPX");
        assert_eq!(
            trouvés[0].clue,
            Clue::Marker {
                text: "UPX!".to_owned(),
                offset: 256
            }
        );
        // La preuve se relit dans le vidage : l'adresse y est.
        assert!(trouvés[0].clue.label("fr").contains("0x100"));
        assert!(trouvés[0].text("fr").contains("upx -d"));
        assert!(trouvés[0].text("en").contains("upx -d"));
    }

    /// Un cookie à moitié binaire reste reconnaissable à l'écrit.
    #[test]
    fn a_half_binary_cookie_stays_readable() {
        let mut fichier = b"programme".to_vec();
        fichier.extend_from_slice(b"MEI\x0c\x0b\x0a\x0b\x0e");
        let trouvés = detect(&fichier);
        assert_eq!(trouvés.len(), 1);
        assert_eq!(trouvés[0].name, "PyInstaller");
        let Clue::Marker { text, .. } = &trouvés[0].clue else {
            panic!("une marque : {:?}", trouvés[0].clue);
        };
        assert_eq!(text, "MEI\\x0c\\x0b\\x0a\\x0b\\x0e");
    }

    /// Deux marques du même empaqueteur ne le nomment qu'une fois.
    #[test]
    fn the_same_packer_is_named_only_once() {
        let mut fichier = b"UPX!".to_vec();
        fichier.extend_from_slice(b"............UPX!");
        let trouvés = detect(&fichier);
        assert_eq!(trouvés.len(), 1, "{trouvés:?}");
    }

    /// Un nombre magique posé à sa place est reconnu ; le même, ailleurs,
    /// ne l'est pas.
    ///
    /// C'est tout l'objet du décalage fixe : trois octets se rencontrent
    /// par hasard dans n'importe quelles données, et nommer un
    /// empaqueteur à tort ferait chercher un déballeur qui n'existe pas.
    #[test]
    fn a_magic_number_counts_only_where_it_belongs() {
        let mut à_sa_place = vec![0u8; 32];
        à_sa_place[8..11].copy_from_slice(b"AI\x02");
        let trouvés = detect(&à_sa_place);
        assert_eq!(trouvés.len(), 1, "{trouvés:?}");
        assert_eq!(trouvés[0].name, "AppImage");

        let mut ailleurs = vec![0u8; 32];
        ailleurs[20..23].copy_from_slice(b"AI\x02");
        assert!(detect(&ailleurs).is_empty(), "les mêmes octets, ailleurs");
    }

    /// Rien ne fait tomber la recherche, quelle que soit la taille.
    #[test]
    fn no_size_breaks_the_search() {
        for taille in [0usize, 1, 2, 3, 7, 8, 9, 255] {
            let _ = detect(&vec![0x55u8; taille]);
        }
    }
}
