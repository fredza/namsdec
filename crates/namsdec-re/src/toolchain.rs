//! Qui a produit ce binaire, et dans quel langage était-il écrit.
//!
//! C'est la première question qu'on pose devant un fichier inconnu, et
//! la plus rentable : un binaire Go ne se lit pas comme un binaire C,
//! un binaire Rust n'a pas les mêmes symboles, et savoir qu'on a du
//! C++ devant soi change la façon dont on lit un `call` indirect —
//! c'est peut-être une table virtuelle.
//!
//! ## Le résultat porte une certitude, et c'est tout l'objet
//!
//! Un outil qui affiche « compilateur : GCC » sans dire d'où il le
//! tient apprend à croire plutôt qu'à lire. Chaque conclusion vient
//! donc avec :
//!
//! - un **degré** — de « c'est écrit dans le fichier » à « un seul
//!   indice faible » ;
//! - les **indices** eux-mêmes, nommés, avec ce qu'on a vu et où.
//!
//! La distinction n'est pas cosmétique. La section `.comment` porte la
//! version exacte du compilateur, écrite par le compilateur lui-même :
//! c'est une déclaration, et elle est fiable tant que personne ne l'a
//! réécrite. Un symbole `_ZN` est une déduction : il dit « quelque
//! chose a produit du code au nom décoré à la façon d'Itanium », ce que
//! font C++, Rust et quelques autres. Les deux ne valent pas la même
//! chose, et les mélanger en un verdict unique serait mentir par
//! omission.
//!
//! ## Ce que ce module NE fait pas
//!
//! Il ne cherche pas à être complet. Les langages listés sont ceux dont
//! on croise des binaires sur un bureau Linux ; le reste tombe dans
//! « indéterminé », ce qui est une réponse honnête et non un échec.
//!
//! Il ne résiste pas non plus à quelqu'un qui VEUT tromper : `.comment`
//! s'écrit, les symboles s'effacent, les chaînes se remplacent. C'est
//! écrit dans ce que le panneau affiche, parce qu'un lecteur qui ne le
//! sait pas tirerait de « Go 1.22 » une confiance qu'aucun octet ne
//! justifie.

use crate::elf_scan::Scan;

/// Ce qu'on a conclu, et sur quoi.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Toolchain {
    /// Le langage, s'il se laisse nommer.
    pub language: Option<Finding>,
    /// Le compilateur et sa version, s'il se laisse nommer.
    pub compiler: Option<Finding>,
    /// Tout ce qui a été relevé, y compris ce qui n'a pas servi à
    /// conclure — c'est en lisant les indices qu'on apprend à les
    /// reconnaître soi-même.
    pub evidence: Vec<Evidence>,
}

/// Une conclusion, avec son degré de certitude.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Ce qu'on nomme : « Rust », « GCC 14.2.1 ».
    pub name: String,
    /// À quel point on y croit.
    pub confidence: Confidence,
}

/// À quel point une conclusion est tenue.
///
/// L'ordre est celui de la force : `Declared` l'emporte sur tout le
/// reste, et deux indices moyens ne font pas une déclaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Un seul indice, et il peut s'expliquer autrement.
    Weak,
    /// Plusieurs indices concordants, ou un indice qui ne trompe guère.
    Likely,
    /// Des indices qui ne laissent pas de place au doute raisonnable.
    Strong,
    /// Le fichier le DIT, dans un champ prévu pour cela — `.comment`,
    /// `.note.go.buildid`. Fiable tant que personne ne l'a réécrit.
    Declared,
}

impl Confidence {
    /// Le mot qu'on écrit à l'écran.
    pub fn label(self, lang: &str) -> &'static str {
        let fr = lang.starts_with("fr");
        match self {
            Self::Declared if fr => "déclaré par le fichier",
            Self::Declared => "declared by the file",
            Self::Strong if fr => "quasi certain",
            Self::Strong => "near certain",
            Self::Likely if fr => "probable",
            Self::Likely => "likely",
            Self::Weak if fr => "un seul indice",
            Self::Weak => "a single clue",
        }
    }
}

/// Un indice relevé dans le fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// Où on l'a vu : « section .comment », « symbole », « DT_NEEDED ».
    pub where_: String,
    /// Ce qu'on y a lu.
    pub what: String,
    /// Ce qu'on en tire.
    pub means: String,
}

/// Les sections dont la seule PRÉSENCE nomme un langage.
///
/// Elles sont posées par la chaîne d'outils du langage et par elle
/// seule : rien d'autre n'écrit un `.gopclntab`.
/// `.gcc_except_table` N'EST PAS dans cette liste, et l'avoir essayé
/// valait la leçon : elle porte les tables de déroulement de pile, que
/// GCC pose pour `throw` — et que rustc pose aussi pour `panic!`.
/// `ripgrep`, qui est écrit en Rust, se voyait donc classer en C++ avec
/// la mention « quasi certain ». Une section partagée par deux langages
/// ne nomme ni l'un ni l'autre.
const SECTIONS_QUI_NOMMENT: &[(&str, &str, &str)] = &[
    (".gopclntab", "Go", "la table de lignes du runtime Go"),
    (".note.go.buildid", "Go", "l'empreinte de build de Go"),
    (".go.buildinfo", "Go", "les informations de build de Go"),
    (
        ".rustc",
        "Rust",
        "les métadonnées que rustc dépose dans ses rlib",
    ),
    (
        ".swift5_typeref",
        "Swift",
        "les métadonnées de types de Swift 5",
    ),
];

/// Les bibliothèques réclamées qui nomment un langage.
const BIBLIOTHÈQUES_QUI_NOMMENT: &[(&str, &str, &str)] = &[
    ("libstdc++", "C++", "la bibliothèque standard C++ de GNU"),
    ("libc++", "C++", "la bibliothèque standard C++ de LLVM"),
    (
        "libgfortran",
        "Fortran",
        "le support d'exécution de Fortran",
    ),
    ("libphobos", "D", "la bibliothèque standard de D"),
    ("libswiftCore", "Swift", "le coeur d'exécution de Swift"),
    ("libHSrts", "Haskell", "le runtime de GHC"),
    ("libgo", "Go", "le runtime de gccgo"),
    ("libobjc", "Objective-C", "le runtime d'Objective-C"),
];

/// Les symboles qui nomment un langage dès qu'ils paraissent.
const SYMBOLES_QUI_NOMMENT: &[(&str, &str, &str)] = &[
    ("__cxa_throw", "C++", "le lancement d'une exception C++"),
    ("__gxx_personality_v0", "C++", "le dérouleur de pile de C++"),
    ("_Znwm", "C++", "`operator new(unsigned long)`"),
    ("_ZdlPv", "C++", "`operator delete(void*)`"),
    (
        "rust_eh_personality",
        "Rust",
        "le dérouleur de pile de Rust",
    ),
    (
        "__rust_alloc",
        "Rust",
        "l'allocateur que rustc appelle par son nom",
    ),
    ("runtime.main", "Go", "le point d'entrée du runtime Go"),
    ("nimFrame", "Nim", "la trace d'appel de Nim"),
    ("NimMain", "Nim", "le démarrage d'un programme Nim"),
    ("caml_main", "OCaml", "le démarrage d'un programme OCaml"),
    ("_D4core", "D", "le runtime de D"),
];

/// Combien d'octets de `.comment` on lit au plus.
///
/// La section fait quelques dizaines d'octets dans un binaire ordinaire
/// et quelques centaines dans un binaire lié à partir de beaucoup
/// d'objets. La borne est là pour qu'un fichier qui ment ne fasse pas
/// lire un mébioctet.
const COMMENT_MAX: usize = 4 * 1024;

/// Ce que le fichier dit de sa chaîne d'outils.
///
/// `bytes` est le fichier entier — on y lit le contenu des sections —,
/// `scan` ce qu'on en a déjà tiré, et `symbols` les noms qu'on connaît
/// (ceux de la table de symboles et ceux de la table de liaison).
pub fn detect(bytes: &[u8], scan: Option<&Scan>, symbols: &[String]) -> Toolchain {
    let mut out = Toolchain::default();
    // Tout ce que `.comment` déclare. UN binaire en porte souvent
    // PLUSIEURS — `target/debug/namsdec-gui` y range GCC (pour les
    // objets C de ses dépendances), rustc, et l'éditeur de liens LLD.
    // Garder le premier venu donnait « GCC 16.2.1 » pour un programme
    // Rust, ce qui est vrai de trois fichiers objets et faux du
    // programme. On choisit à la fin, quand le langage est connu.
    let mut déclarés: Vec<String> = Vec::new();

    // --- 0. Un script dit son interprète sur sa première ligne ------
    //
    // Ce n'est pas un binaire, et c'est justement pourquoi cela vaut la
    // peine : les trois quarts des « binaires » non reconnus d'un
    // /usr/bin sont des scripts, et répondre « indéterminé » devant un
    // `#!/usr/bin/python3` serait un comble. La ligne est une
    // DÉCLARATION, au sens propre : le noyau la lit et la croit.
    if let Some(interprète) = shebang(bytes) {
        let langage = langage_de_l_interprète(&interprète);
        out.evidence.push(Evidence {
            where_: "première ligne".to_owned(),
            what: format!("#!{interprète}"),
            means: "le noyau lit cette ligne et lance cet interprète".to_owned(),
        });
        out.language = Some(Finding {
            name: langage,
            confidence: Confidence::Declared,
        });
        out.compiler = Some(Finding {
            name: interprète,
            confidence: Confidence::Declared,
        });
        return out;
    }

    // --- 1. Ce que le fichier DÉCLARE -------------------------------
    //
    // `.comment` est écrit par le compilateur lui-même et porte sa
    // version exacte. C'est la seule source qui donne un numéro, et
    // c'est pour cela qu'elle vient en premier.
    if let Some(scan) = scan
        && let Some(texte) = lire_comment(bytes, scan)
    {
        for morceau in texte.split('\0').filter(|m| !m.trim().is_empty()) {
            let morceau = morceau.trim();
            out.evidence.push(Evidence {
                where_: "section .comment".to_owned(),
                what: morceau.to_owned(),
                means: "le compilateur y écrit son nom et sa version".to_owned(),
            });
            if let Some(nom) = compilateur_de(morceau) {
                déclarés.push(nom);
            }
        }
    }

    // --- 2. Les sections qui nomment un langage ---------------------
    let mut votes: Vec<(String, Confidence, String)> = Vec::new();
    if let Some(scan) = scan {
        for section in &scan.sections {
            for (motif, langage, pourquoi) in SECTIONS_QUI_NOMMENT {
                if section.name == *motif {
                    out.evidence.push(Evidence {
                        where_: format!("section {}", section.name),
                        what: (*pourquoi).to_owned(),
                        means: format!("produit par la chaîne d'outils {langage}"),
                    });
                    votes.push((
                        (*langage).to_owned(),
                        Confidence::Strong,
                        format!("la section {}", section.name),
                    ));
                }
            }
        }
        // --- 3. Les bibliothèques réclamées -------------------------
        for lib in &scan.needed {
            for (motif, langage, pourquoi) in BIBLIOTHÈQUES_QUI_NOMMENT {
                if lib.contains(motif) {
                    out.evidence.push(Evidence {
                        where_: "DT_NEEDED".to_owned(),
                        what: lib.clone(),
                        means: format!("{pourquoi} — donc {langage}"),
                    });
                    votes.push((
                        (*langage).to_owned(),
                        Confidence::Strong,
                        format!("la bibliothèque {lib}"),
                    ));
                }
            }
        }
    }

    // --- 4. Les symboles --------------------------------------------
    for symbole in symbols {
        for (motif, langage, pourquoi) in SYMBOLES_QUI_NOMMENT {
            if symbole == motif || symbole.starts_with(motif) {
                out.evidence.push(Evidence {
                    where_: "symbole".to_owned(),
                    what: symbole.clone(),
                    means: format!("{pourquoi} — donc {langage}"),
                });
                votes.push((
                    (*langage).to_owned(),
                    Confidence::Strong,
                    format!("le symbole {symbole}"),
                ));
            }
        }
    }

    // Le décor de noms de Rust : `_ZN…17h<16 chiffres hexadécimaux>E`
    // pour l'ancien, `_R` pour le format v0. Le hachage final est ce
    // qui distingue Rust de C++, qui emploie le même préfixe `_ZN`.
    let rust_legacy = symbols.iter().filter(|s| est_rust_legacy(s)).count();
    let rust_v0 = symbols.iter().filter(|s| s.starts_with("_R")).count();
    if rust_legacy > 0 {
        out.evidence.push(Evidence {
            where_: "décor des symboles".to_owned(),
            what: format!("{rust_legacy} symboles en `_ZN…17h<hachage>E`"),
            means: "le décor hérité de rustc : un hachage de seize chiffres en fin de nom"
                .to_owned(),
        });
        votes.push((
            "Rust".to_owned(),
            Confidence::Strong,
            format!("{rust_legacy} symboles au décor de rustc"),
        ));
    }
    if rust_v0 > 0 {
        out.evidence.push(Evidence {
            where_: "décor des symboles".to_owned(),
            what: format!("{rust_v0} symboles en `_R…`"),
            means: "le décor v0 de rustc".to_owned(),
        });
        votes.push((
            "Rust".to_owned(),
            Confidence::Strong,
            format!("{rust_v0} symboles au décor v0"),
        ));
    }
    // `_ZN` SANS hachage Rust : C++, ou un autre langage qui emploie le
    // décor d'Itanium. Indice moyen, et pas davantage.
    let itanium = symbols
        .iter()
        .filter(|s| s.starts_with("_Z") && !s.starts_with("_ZN") || s.starts_with("_ZN"))
        .filter(|s| !est_rust_legacy(s))
        .count();
    if itanium > 0 && rust_legacy == 0 && rust_v0 == 0 {
        out.evidence.push(Evidence {
            where_: "décor des symboles".to_owned(),
            what: format!("{itanium} symboles en `_Z…`"),
            means: "le décor d'Itanium, employé par C++ (et quelques autres)".to_owned(),
        });
        votes.push((
            "C++".to_owned(),
            Confidence::Likely,
            format!("{itanium} symboles au décor d'Itanium"),
        ));
    }

    // --- 5. Les chaînes qui trahissent ------------------------------
    for (motif, langage, pourquoi, force) in [
        (
            "/rustc/",
            "Rust",
            "un chemin de la bibliothèque standard de Rust",
            Confidence::Strong,
        ),
        // Les distributions REMAPPENT les chemins de compilation —
        // Fedora écrit `/usr/share/cargo/registry/…` et
        // `library/std/src/…` là où rustc écrirait `/rustc/<empreinte>/`.
        // Chercher la seule forme d'origine faisait classer `ripgrep` et
        // `eza`, dépouillés et empaquetés, parmi les programmes C. Les
        // trois formes sont donc cherchées.
        (
            "library/std/src/",
            "Rust",
            "un chemin de la bibliothèque standard de Rust, remappé par la distribution",
            Confidence::Strong,
        ),
        (
            "cargo/registry/",
            "Rust",
            "un chemin du registre de paquets de cargo",
            Confidence::Strong,
        ),
        (
            "called `Option::unwrap()` on a `None` value",
            "Rust",
            "un message de panique de la bibliothèque standard de Rust",
            Confidence::Strong,
        ),
        (
            "goroutine ",
            "Go",
            "le vocabulaire du runtime Go",
            Confidence::Likely,
        ),
        (
            "basic_string",
            "C++",
            "le nom décoré de `std::string`",
            Confidence::Likely,
        ),
        (
            "go1.",
            "Go",
            "la version du compilateur Go, telle que son runtime la porte",
            Confidence::Likely,
        ),
        (
            "PyInstaller",
            "Python",
            "un exécutable produit par PyInstaller",
            Confidence::Strong,
        ),
        (
            "GHC ",
            "Haskell",
            "la bannière du compilateur GHC",
            Confidence::Likely,
        ),
        (
            "zig 0.",
            "Zig",
            "la bannière de version du compilateur Zig",
            Confidence::Likely,
        ),
    ] {
        if let Some(vu) = chaîne_contenant(bytes, motif) {
            out.evidence.push(Evidence {
                where_: "chaîne lisible".to_owned(),
                what: vu.clone(),
                means: format!("{pourquoi} — donc {langage}"),
            });
            votes.push((langage.to_owned(), force, format!("la chaîne « {vu} »")));
        }
    }

    // --- 6. Le C, qui ne laisse aucune trace de lui-même ------------
    //
    // Aucun langage n'est plus discret : pas de décor de noms, pas de
    // runtime nommé, pas de section à lui. Ce qui le désigne est donc
    // NÉGATIF — la libc réclamée, et rien d'autre qui parle. C'est une
    // déduction par élimination, et elle est marquée comme telle.
    if votes.is_empty()
        && let Some(scan) = scan
        && let Some(libc) = scan
            .needed
            .iter()
            .find(|lib| lib.starts_with("libc.so") || lib.starts_with("libc-"))
    {
        out.evidence.push(Evidence {
            where_: "DT_NEEDED".to_owned(),
            what: libc.clone(),
            means: "la bibliothèque C, et aucun runtime d'un autre langage".to_owned(),
        });
        votes.push((
            "C".to_owned(),
            Confidence::Likely,
            "la libc seule, sans autre runtime".to_owned(),
        ));
    }

    // --- 7. Et s'il n'y a rien ? ------------------------------------
    //
    // Un ELF sans libc, sans `.comment`, sans symbole décoré et qui
    // passe ses appels système à la main est le portrait de ce que
    // namsdec produit lui-même : de l'assembleur assemblé par nasm puis
    // lié par ld. C'est une conclusion, pas un aveu d'ignorance — mais
    // elle reste faible, parce que du C compilé sans bibliothèque
    // standard a exactement la même allure.
    if votes.is_empty()
        && let Some(scan) = scan
        && scan.needed.is_empty()
        && scan.interpreter.is_none()
        && out.compiler.is_none()
    {
        out.evidence.push(Evidence {
            where_: "absence".to_owned(),
            what: "ni bibliothèque réclamée, ni section .comment, ni symbole décoré".to_owned(),
            means: "le portrait d'un programme écrit directement en assembleur — ou de \
                    code compilé sans bibliothèque standard"
                .to_owned(),
        });
        votes.push((
            "assembleur".to_owned(),
            Confidence::Weak,
            "l'absence de toute trace de bibliothèque".to_owned(),
        ));
    }

    out.language = conclure(&votes);

    // LE COMPILATEUR DÉCLARÉ QUI CORRESPOND AU LANGAGE.
    //
    // Entre « GCC 16.2.1 » et « rustc 1.99.0 » dans le même `.comment`,
    // c'est le langage conclu qui tranche — et il a été conclu sur des
    // indices indépendants de `.comment`, donc il n'y a pas de
    // circularité. Faute de correspondance, le premier déclaré, qui
    // reste une information vraie sur le fichier.
    if !déclarés.is_empty() {
        let attendu = out.as_deref_name().and_then(mot_du_compilateur);
        let choisi = attendu
            .and_then(|mot| {
                déclarés
                    .iter()
                    .find(|ligne| ligne.to_ascii_lowercase().contains(mot))
            })
            .or_else(|| déclarés.first());
        if let Some(ligne) = choisi {
            out.compiler = Some(Finding {
                name: ligne.clone(),
                confidence: Confidence::Declared,
            });
        }
    }

    // Le compilateur peut aussi se déduire du langage quand `.comment`
    // n'a rien dit : rustc pour Rust, le compilateur Go pour Go. Ce
    // n'est pas une tautologie — c'est l'information qu'on cherchait, et
    // elle est honnêtement marquée comme déduite.
    if out.compiler.is_none()
        && let Some(langage) = &out.language
    {
        let déduit = match langage.name.as_str() {
            "Rust" => Some("rustc"),
            "Go" => Some("le compilateur Go (gc)"),
            "Swift" => Some("swiftc"),
            "Nim" => Some("nim (via un compilateur C)"),
            _ => None,
        };
        if let Some(nom) = déduit {
            out.compiler = Some(Finding {
                name: nom.to_owned(),
                // Jamais davantage que ce qui a servi à conclure au
                // langage : une déduction ne gagne pas en force en
                // changeant de sujet.
                confidence: langage.confidence.min(Confidence::Likely),
            });
        }
    }
    out
}

impl Toolchain {
    /// Le nom du langage conclu, s'il y en a un.
    fn as_deref_name(&self) -> Option<&str> {
        self.language.as_ref().map(|f| f.name.as_str())
    }
}

/// L'interprète qu'un script réclame sur sa première ligne.
///
/// `#!/usr/bin/env python3` comme `#!/bin/sh` : on garde la ligne
/// entière après le `#!`, qui est ce que le noyau lit.
fn shebang(bytes: &[u8]) -> Option<String> {
    let reste = bytes.strip_prefix(b"#!")?;
    // Bornée : une première ligne de cent mille octets n'est pas un
    // shebang, c'est un fichier qui essaie quelque chose.
    let fin = reste
        .iter()
        .take(256)
        .position(|o| *o == b'\n' || *o == b'\r')?;
    let ligne = std::str::from_utf8(&reste[..fin]).ok()?.trim();
    (!ligne.is_empty()).then(|| ligne.to_owned())
}

/// Le langage que nomme un interprète.
///
/// Sur le nom du programme, et non sur le chemin : `/usr/bin/env
/// python3`, `/usr/bin/python3.13` et `python` sont le même langage.
fn langage_de_l_interprète(ligne: &str) -> String {
    let mots: Vec<&str> = ligne.split_whitespace().collect();
    // `env` passe la main au mot suivant, qui est le vrai interprète.
    let programme = mots
        .iter()
        .map(|mot| mot.rsplit('/').next().unwrap_or(mot))
        .find(|nom| *nom != "env" && !nom.starts_with('-'))
        .unwrap_or("");
    let sans_version = programme.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    match sans_version {
        "python" => "Python",
        "perl" => "Perl",
        "ruby" => "Ruby",
        "node" => "JavaScript (Node)",
        "sh" | "bash" | "dash" | "zsh" | "ksh" | "fish" => "script shell",
        "lua" => "Lua",
        "php" => "PHP",
        "tclsh" | "wish" => "Tcl",
        "awk" | "gawk" | "mawk" => "awk",
        _ => return format!("script ({programme})"),
    }
    .to_owned()
}

/// Le mot qu'on cherche dans `.comment` pour un langage donné.
fn mot_du_compilateur(langage: &str) -> Option<&'static str> {
    match langage {
        "Rust" => Some("rustc"),
        "Go" => Some("go"),
        "C++" | "C" => None,
        _ => None,
    }
}

/// Le vote qui l'emporte : la force d'abord, le nombre d'indices
/// ensuite.
///
/// Deux indices faibles ne valent pas un indice fort — c'est la règle, et
/// elle est délibérée. Un seul `_ZN` rend « C++, probable » ; dix n'y
/// changent rien, parce qu'ils disent tous la même chose. Ce qui
/// renforce une conclusion, c'est qu'un indice d'une AUTRE nature la
/// confirme, et c'est ce que compte le nombre de sources distinctes.
fn conclure(votes: &[(String, Confidence, String)]) -> Option<Finding> {
    let mut par_langage: std::collections::BTreeMap<&str, (Confidence, usize)> =
        std::collections::BTreeMap::new();
    for (langage, force, _) in votes {
        let entrée = par_langage
            .entry(langage.as_str())
            .or_insert((*force, 0usize));
        entrée.0 = entrée.0.max(*force);
        entrée.1 += 1;
    }
    let (nom, (force, combien)) = par_langage
        .into_iter()
        .max_by_key(|(_, (force, combien))| (*force, *combien))?;
    // Deux sources distinctes qui concordent font monter d'un cran — mais
    // jamais jusqu'à « déclaré », qui n'appartient qu'à ce que le fichier
    // écrit lui-même.
    let force = if combien >= 2 && force < Confidence::Strong {
        Confidence::Strong
    } else {
        force
    };
    Some(Finding {
        name: nom.to_owned(),
        confidence: force,
    })
}

/// Le contenu de `.comment`, borné.
fn lire_comment(bytes: &[u8], scan: &Scan) -> Option<String> {
    let section = scan
        .sections
        .iter()
        .find(|s| s.name == ".comment" && !s.nobits)?;
    let début = usize::try_from(section.offset).ok()?;
    let taille = usize::try_from(section.size).ok()?.min(COMMENT_MAX);
    let fin = début.checked_add(taille)?;
    let brut = bytes.get(début..fin)?;
    Some(String::from_utf8_lossy(brut).into_owned())
}

/// Le nom et la version d'un compilateur, lus dans une ligne de
/// `.comment`.
///
/// Les formes réelles : `GCC: (GNU) 14.2.1 20250110`, `clang version
/// 18.1.8`, `rustc version 1.82.0`. On garde la ligne telle quelle
/// plutôt que de la recomposer : elle est déjà faite pour être lue, et
/// la réécrire ferait perdre la date de compilation, qui sert.
fn compilateur_de(ligne: &str) -> Option<String> {
    let bas = ligne.to_ascii_lowercase();
    if bas.starts_with("gcc:") || bas.contains("gnu c") || bas.starts_with("gcc ") {
        return Some(ligne.trim_start_matches("GCC: ").trim().to_owned());
    }
    if bas.contains("clang") || bas.contains("llvm") {
        return Some(ligne.trim().to_owned());
    }
    if bas.contains("rustc") {
        return Some(ligne.trim().to_owned());
    }
    None
}

/// Le décor hérité de rustc : `_ZN` … `17h` + seize chiffres
/// hexadécimaux + `E`.
///
/// C'est ce hachage, et lui seul, qui distingue un symbole Rust d'un
/// symbole C++ : les deux emploient le décor d'Itanium, et le préfixe
/// ne dit donc rien à lui seul.
fn est_rust_legacy(symbole: &str) -> bool {
    let Some(reste) = symbole.strip_suffix('E') else {
        return false;
    };
    let Some(position) = reste.rfind("17h") else {
        return false;
    };
    let hachage = &reste[position + 3..];
    hachage.len() == 16 && hachage.bytes().all(|b| b.is_ascii_hexdigit())
}

/// La première chaîne lisible du fichier qui contient `motif`, rendue
/// RECENTRÉE sur ce qu'on y a trouvé.
///
/// Les deux parties comptent, et la seconde a été apprise à l'usage :
/// un binaire porte des chaînes de plusieurs centaines de caractères
/// collées bout à bout, et montrer les cent vingt PREMIERS donnait un
/// extrait où le motif ne paraissait même pas. « Voici l'indice » suivi
/// d'un texte où il est invisible ne vaut pas mieux que pas d'indice.
///
/// Le motif doit en outre être DÉLIMITÉ : `zig` au milieu d'un mot ne
/// dit rien, et c'est ainsi que `ripgrep` se voyait attribuer un indice
/// de chaîne d'outils Zig pris à l'intérieur d'un message d'erreur.
fn chaîne_contenant(bytes: &[u8], motif: &str) -> Option<String> {
    let aiguille = motif.as_bytes();
    let mut courante: Vec<u8> = Vec::new();
    for octet in bytes.iter().chain(std::iter::once(&0u8)) {
        if (0x20..0x7f).contains(octet) {
            courante.push(*octet);
            continue;
        }
        if let Some(à) = position_délimitée(&courante, aiguille) {
            return Some(extrait(&courante, à, aiguille.len()));
        }
        courante.clear();
    }
    None
}

/// Où `aiguille` paraît dans `foin`, à condition de ne pas être collée
/// à une lettre ou à un chiffre sur sa gauche.
///
/// Seulement à gauche : la plupart des motifs cherchés finissent déjà
/// par un séparateur — `go1.`, `zig 0.`, `/rustc/` — et exiger une
/// frontière à droite écarterait justement la version qui suit.
fn position_délimitée(foin: &[u8], aiguille: &[u8]) -> Option<usize> {
    if aiguille.is_empty() || foin.len() < aiguille.len() {
        return None;
    }
    (0..=foin.len() - aiguille.len()).find(|&début| {
        &foin[début..début + aiguille.len()] == aiguille
            && (début == 0 || !foin[début - 1].is_ascii_alphanumeric())
    })
}

/// Une fenêtre lisible autour de ce qu'on a trouvé.
fn extrait(foin: &[u8], à: usize, longueur: usize) -> String {
    const AUTOUR: usize = 40;
    let début = à.saturating_sub(AUTOUR);
    let fin = (à + longueur + AUTOUR).min(foin.len());
    let mut texte = String::from_utf8_lossy(&foin[début..fin]).into_owned();
    if début > 0 {
        texte.insert(0, '…');
    }
    if fin < foin.len() {
        texte.push('…');
    }
    texte
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le décor de rustc se distingue de celui de C++ par son hachage,
    /// et par rien d'autre.
    ///
    /// C'est le point délicat de tout ce module : les deux langages
    /// emploient le décor d'Itanium, et conclure « C++ » sur le seul
    /// `_ZN` ferait passer tout binaire Rust pour du C++.
    #[test]
    fn the_rust_hash_is_what_tells_it_from_cpp() {
        assert!(est_rust_legacy(
            "_ZN4core3fmt9Formatter3pad17h0123456789abcdefE"
        ));
        // Un vrai symbole C++ : même préfixe, pas de hachage.
        assert!(!est_rust_legacy("_ZNSt6vectorIiSaIiEE9push_backERKi"));
        // Un hachage trop court, ou qui n'est pas hexadécimal.
        assert!(!est_rust_legacy("_ZN4core17h0123E"));
        assert!(!est_rust_legacy("_ZN4core17hzzzzzzzzzzzzzzzzE"));
    }

    /// Une ligne de `.comment` rend le compilateur ET sa version.
    #[test]
    fn the_comment_section_gives_a_version() {
        assert_eq!(
            compilateur_de("GCC: (GNU) 14.2.1 20250110").as_deref(),
            Some("(GNU) 14.2.1 20250110")
        );
        assert_eq!(
            compilateur_de("clang version 18.1.8").as_deref(),
            Some("clang version 18.1.8")
        );
        assert_eq!(compilateur_de("une ligne quelconque"), None);
    }

    /// Deux indices de NATURE DIFFÉRENTE font monter la certitude ; dix
    /// indices de même nature, non.
    ///
    /// C'est la règle du module, et elle mérite d'être retenue par un
    /// contrôle : dix symboles `_ZN` disent tous la même chose, et
    /// répéter une observation n'en fait pas une preuve.
    #[test]
    fn two_sources_agreeing_raise_the_confidence() {
        let un_seul = conclure(&[("C++".to_owned(), Confidence::Likely, "le décor".to_owned())])
            .expect("une conclusion");
        assert_eq!(un_seul.confidence, Confidence::Likely);

        let deux = conclure(&[
            ("C++".to_owned(), Confidence::Likely, "le décor".to_owned()),
            ("C++".to_owned(), Confidence::Likely, "libstdc++".to_owned()),
        ])
        .expect("une conclusion");
        assert_eq!(deux.confidence, Confidence::Strong);

        // Mais jamais jusqu'à « déclaré » : cela n'appartient qu'à ce
        // que le fichier écrit lui-même.
        let beaucoup = conclure(&[
            ("Go".to_owned(), Confidence::Strong, "a".to_owned()),
            ("Go".to_owned(), Confidence::Strong, "b".to_owned()),
            ("Go".to_owned(), Confidence::Strong, "c".to_owned()),
        ])
        .expect("une conclusion");
        assert_eq!(beaucoup.confidence, Confidence::Strong);
    }

    /// Le binaire le plus proche de nous : celui du projet lui-même.
    ///
    /// Il est en Rust, et le fichier le dit de trois façons. Le contrôle
    /// s'abstient si le binaire n'est pas bâti — il ne prouverait rien
    /// de plus en échouant.
    #[test]
    fn a_rust_binary_is_recognised() {
        let candidats = ["/bin/rg", "/usr/bin/rg", "/bin/fd", "/usr/bin/eza"];
        for chemin in candidats {
            let Ok(octets) = std::fs::read(chemin) else {
                continue;
            };
            let scan = crate::elf_scan::scan(&octets);
            let symboles: Vec<String> = scan
                .as_ref()
                .map(|s| s.functions.iter().map(|f| f.name.clone()).collect())
                .unwrap_or_default();
            let trouvé = detect(&octets, scan.as_ref(), &symboles);
            if let Some(langage) = &trouvé.language
                && langage.name == "Rust"
            {
                assert!(
                    !trouvé.evidence.is_empty(),
                    "la conclusion s'appuie sur des indices nommés"
                );
                return;
            }
        }
    }

    /// Un binaire C du système : `.comment` le déclare, et c'est la
    /// seule source qui donne un numéro de version.
    #[test]
    fn a_system_binary_declares_its_compiler() {
        let Ok(octets) = std::fs::read("/bin/ls") else {
            return;
        };
        let scan = crate::elf_scan::scan(&octets);
        let trouvé = detect(&octets, scan.as_ref(), &[]);
        // `.comment` manque sur un binaire dépouillé par le paquetage :
        // ce n'est pas un défaut, et le contrôle ne l'exige pas. Ce
        // qu'il exige, c'est que la conclusion tirée de `.comment`
        // porte bien le degré « déclaré ».
        if let Some(compilateur) = &trouvé.compiler
            && trouvé
                .evidence
                .iter()
                .any(|e| e.where_ == "section .comment")
        {
            assert_eq!(compilateur.confidence, Confidence::Declared);
        }
    }

    /// Un script dit son langage sur sa première ligne, et c'est une
    /// déclaration.
    ///
    /// Les trois quarts des fichiers exécutables non reconnus d'un
    /// `/usr/bin` sont des scripts : répondre « indéterminé » devant un
    /// `#!/usr/bin/python3` serait un comble.
    #[test]
    fn a_shebang_names_the_language() {
        let cas = [
            ("#!/usr/bin/python3\nprint(1)\n", "Python"),
            ("#!/usr/bin/env python3.13\n", "Python"),
            ("#!/bin/sh\necho\n", "script shell"),
            ("#!/usr/bin/env node\n", "JavaScript (Node)"),
            ("#!/usr/bin/perl -w\n", "Perl"),
        ];
        for (texte, attendu) in cas {
            let trouvé = detect(texte.as_bytes(), None, &[]);
            let langage = trouvé.language.expect("un langage");
            assert_eq!(langage.name, attendu, "pour « {texte:?} »");
            assert_eq!(langage.confidence, Confidence::Declared);
        }
        // Et ce qui n'en est pas un n'en devient pas un.
        assert!(shebang(b"pas un script").is_none());
        assert!(shebang(b"#!").is_none(), "sans ligne, rien");
    }

    /// `.gcc_except_table` NE nomme PAS C++.
    ///
    /// C'est la régression d'un vrai défaut, trouvé en essayant le
    /// module sur de vrais binaires : la section porte les tables de
    /// déroulement de pile, que GCC pose pour `throw` et que rustc pose
    /// pour `panic!`. `ripgrep`, écrit en Rust, se voyait classer
    /// « C++, quasi certain ».
    #[test]
    fn an_unwind_table_names_no_language() {
        assert!(
            !SECTIONS_QUI_NOMMENT
                .iter()
                .any(|(nom, _, _)| *nom == ".gcc_except_table"),
            "une section que deux langages posent ne nomme ni l'un ni l'autre"
        );
    }

    /// Un motif collé au milieu d'un mot ne compte pas.
    ///
    /// Second défaut trouvé au même essai : `zig` cherché tel quel se
    /// trouvait à l'intérieur d'un message d'erreur de `ripgrep`, qui
    /// héritait donc d'un indice de chaîne d'outils Zig.
    #[test]
    fn a_pattern_glued_inside_a_word_does_not_count() {
        assert_eq!(position_délimitée(b"amigo1.0", b"go1."), None);
        assert_eq!(position_délimitée(b"/go1.22", b"go1."), Some(1));
        assert_eq!(position_délimitée(b"go1.22", b"go1."), Some(0));
        // Et l'extrait est RECENTRÉ sur ce qu'on a trouvé, non pris au
        // début : montrer cent vingt caractères où l'indice ne paraît
        // pas ne vaut pas mieux que pas d'indice.
        let long = format!("{}MOTIF{}", "a".repeat(200), "b".repeat(200));
        let vu = extrait(long.as_bytes(), 200, 5);
        assert!(vu.contains("MOTIF"), "{vu}");
        assert!(vu.len() < 120, "et il tient sur une ligne : {}", vu.len());
    }

    /// Plusieurs compilateurs déclarés : c'est celui du langage conclu
    /// qui gagne.
    ///
    /// Troisième défaut du même essai : un binaire Rust porte dans son
    /// `.comment` GCC (pour les objets C de ses dépendances), rustc et
    /// l'éditeur de liens. Garder le premier venu annonçait
    /// « GCC 16.2.1 » pour un programme Rust.
    #[test]
    fn the_declared_compiler_follows_the_language() {
        assert_eq!(mot_du_compilateur("Rust"), Some("rustc"));
        assert_eq!(mot_du_compilateur("C"), None);
    }

    /// Rien ne panique sur des octets quelconques.
    #[test]
    fn nothing_panics_on_arbitrary_bytes() {
        let cas: [&[u8]; 4] = [b"", b"\x7fELF", b"MZ\0\0", &[0xff; 512]];
        for cas in cas {
            let _ = detect(cas, None, &["_ZN".to_owned(), String::new()]);
        }
    }
}
