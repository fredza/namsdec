//! Aller chercher un outil, et le mettre en état de servir.
//!
//! namsdec dit jusqu'ici ce qu'il faut taper, et laisse taper. Ce module
//! propose de le taper pour vous — ce qui change la nature de l'acte :
//! on ne décrit plus une commande, on **télécharge et l'on exécute du
//! code venu d'Internet**. Trois principes le gouvernent, et ils sont
//! plus importants que le code lui-même.
//!
//! **Tout est montré avant d'être fait.** Une recette est une suite
//! d'étapes, chacune une commande complète avec ses arguments. Elles
//! sont affichées, dans l'ordre, avant qu'aucune ne parte — et chacune
//! est réaffichée au moment où elle part, avec ce qu'elle répond.
//!
//! **Rien ne sort du dossier de namsdec.** Pas de `sudo`, pas
//! d'installation à l'échelle du système, rien dans `/usr`. Tout va dans
//! `~/.local/share/namsdec/tools/`, et se retire en effaçant ce dossier.
//!
//! **Rien n'est embarqué.** Les téléchargements passent par `curl`,
//! l'extraction par `tar`, la compilation par `meson` ou `cmake` :
//! namsdec appelle les outils du système plutôt que de les réécrire,
//! exactement comme il fait pour `nasm` et `ld`. Le binaire n'en grossit
//! pas d'un octet, et ce qui se passe est vérifiable avec les mêmes
//! commandes, tapées à la main.

use std::path::{Path, PathBuf};

/// Une étape d'une recette : une commande, et ce qu'elle fait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Ce que cette étape accomplit, en une phrase.
    pub about_fr: String,
    pub about_en: String,
    pub command: String,
    pub args: Vec<String>,
    /// Le dossier depuis lequel lancer la commande, s'il importe.
    pub cwd: Option<PathBuf>,
}

impl Step {
    /// La commande, telle qu'on la taperait soi-même.
    pub fn line(&self) -> String {
        format!("{} {}", self.command, self.args.join(" "))
    }
}

/// Ce qu'il faut faire pour avoir un outil, et ce qu'il en coûte.
#[derive(Debug, Clone)]
pub struct Recipe {
    /// L'outil concerné, par son identifiant de plugin.
    pub tool: &'static str,
    pub steps: Vec<Step>,
    /// Les programmes qu'il faut avoir avant de commencer.
    pub requires: Vec<&'static str>,
    /// Ce que l'affaire coûte, dit franchement.
    pub cost_fr: &'static str,
    pub cost_en: &'static str,
    /// Le programme qui doit exister à la fin, et qu'on déclarera.
    pub produces: PathBuf,
    /// D'où vient ce qu'on télécharge, pour que ce soit vérifiable.
    pub source: &'static str,
    /// Ce que cet outil apporte, et ce qu'il ne fait pas.
    ///
    /// Une fenêtre d'installation qui ne dit que « voulez-vous
    /// installer ? » demande un acte de foi. Celle-ci explique ce qu'on
    /// va gagner — c'est aussi la seule occasion d'apprendre la
    /// différence entre désassembler et décompiler.
    pub teaches_fr: &'static str,
    pub teaches_en: &'static str,
}

/// Où les outils installés par namsdec vivent.
///
/// Sous son propre dossier de données : ce que namsdec a posé, namsdec
/// peut le reprendre, et rien n'est allé se mêler aux paquets du
/// système.
pub fn tools_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("tools")
}

/// La version de RetDec qu'on installe.
///
/// Épinglée plutôt que « la dernière » : une recette qui suit
/// automatiquement ce qu'un dépôt publie exécute demain autre chose que
/// ce qu'elle a montré aujourd'hui.
const RETDEC_VERSION: &str = "v5.0";
/// Le nom exact du fichier publié.
///
/// Relevé sur la page des versions, pas deviné : la première écriture de
/// cette recette portait `retdec-v5.0-linux-64b.tar.xz`, qui n'existe
/// pas — l'installation aurait téléchargé une page d'erreur.
const RETDEC_ARCHIVE: &str = "RetDec-v5.0-Linux-Release.tar.xz";

/// La recette de RetDec : une version publiée, décompressée.
///
/// Pas de compilation : RetDec demande une demi-heure et huit gigaoctets
/// de mémoire pour se construire, alors que ses auteurs publient un
/// paquet tout fait. On prend le paquet.
fn retdec(data: &Path) -> Recipe {
    let racine = tools_dir(data);
    let archive = racine.join(RETDEC_ARCHIVE);
    // L'archive n'a pas de dossier racine : elle contient `bin/`,
    // `share/`… directement. Sans un dossier à nous où l'extraire, elle
    // se répandrait dans celui des outils.
    let destination = racine.join("retdec");
    Recipe {
        tool: "retdec",
        requires: vec!["curl", "tar", "sha256sum"],
        cost_fr: "180 Mo à télécharger, et 1,5 Go une fois décompressé — dont 1,1 Go de \
                  signatures et de motifs, qui sont ce qui permet à RetDec de reconnaître les \
                  fonctions de bibliothèque. Deux à cinq minutes selon la ligne. Rien à compiler.",
        cost_en: "180 MB to download, and 1.5 GB once unpacked — 1.1 GB of it signatures and \
                  patterns, which are what lets RetDec recognise library functions. Two to five \
                  minutes depending on your line. Nothing to compile.",
        source: "https://github.com/avast/retdec/releases",
        teaches_fr: "Un décompilateur ne traduit pas les instructions une à une : il reconstruit \
                     des boucles, des conditions, des variables — la forme qu'avait le programme \
                     avant d'être compilé. RetDec rend du C qu'un compilateur accepte. Ce qu'il \
                     ne rend pas : les noms d'origine, les commentaires, et la certitude que \
                     c'était écrit ainsi. Un décompilateur propose une lecture, il ne restitue \
                     pas la source.",
        teaches_en: "A decompiler does not translate instructions one by one: it rebuilds loops, \
                     conditions and variables — the shape the program had before it was compiled. \
                     RetDec returns C a compiler accepts. What it does not return: the original \
                     names, the comments, and any certainty that it was written that way. A \
                     decompiler offers a reading; it does not restore the source.",
        produces: racine.join("retdec/bin/retdec-decompiler"),
        steps: vec![
            Step {
                about_fr: "Créer le dossier qui recevra RetDec".into(),
                about_en: "Create the directory that will hold RetDec".into(),
                command: "mkdir".into(),
                args: vec!["-p".into(), destination.display().to_string()],
                cwd: None,
            },
            // Un téléchargement interrompu laisse une archive à demi
            // écrite, que `tar` refusera plus loin sans qu'on sache
            // pourquoi.
            Step {
                about_fr: "Effacer une archive laissée par une tentative précédente".into(),
                about_en: "Remove any archive left by an earlier attempt".into(),
                command: "rm".into(),
                args: vec!["-f".into(), archive.display().to_string()],
                cwd: None,
            },
            Step {
                about_fr: format!("Télécharger RetDec {RETDEC_VERSION} depuis GitHub"),
                about_en: format!("Download RetDec {RETDEC_VERSION} from GitHub"),
                command: "curl".into(),
                args: vec![
                    // `-L` suit les redirections de GitHub, `-f` fait
                    // échouer sur une page d'erreur au lieu de
                    // l'enregistrer comme si c'était l'archive.
                    "-fL".into(),
                    // `-#` remplace le tableau de chiffres par une
                    // jauge d'une ligne, réécrite avec un retour
                    // chariot : c'est ce que le journal sait afficher.
                    "-#".into(),
                    "--proto".into(),
                    "=https".into(),
                    "-o".into(),
                    archive.display().to_string(),
                    format!(
                        "https://github.com/avast/retdec/releases/download/{RETDEC_VERSION}/{RETDEC_ARCHIVE}"
                    ),
                ],
                cwd: None,
            },
            Step {
                // L'empreinte n'est pas comparée à une valeur écrite
                // ici : personne ne l'a vérifiée, et une constante
                // inventée serait pire qu'une absence. Elle est
                // **affichée**, pour être comparée à celle que publie le
                // projet.
                about_fr: "Calculer l'empreinte de ce qui a été téléchargé".into(),
                about_en: "Compute the fingerprint of what was downloaded".into(),
                command: "sha256sum".into(),
                args: vec![archive.display().to_string()],
                cwd: None,
            },
            Step {
                about_fr: "Extraire l'archive".into(),
                about_en: "Extract the archive".into(),
                command: "tar".into(),
                args: vec![
                    "xf".into(),
                    archive.display().to_string(),
                    "-C".into(),
                    destination.display().to_string(),
                ],
                cwd: None,
            },
            Step {
                about_fr: "Effacer l'archive, qui ne sert plus".into(),
                about_en: "Remove the archive, now useless".into(),
                command: "rm".into(),
                args: vec!["-f".into(), archive.display().to_string()],
                cwd: None,
            },
        ],
    }
}

/// La recette de Rizin et de son greffon.
///
/// Ici il faut compiler : ni Rizin ni `rz-ghidra` ne publient de paquet
/// qu'on puisse poser dans un dossier personnel, et le paquet de la
/// distribution demanderait `sudo` — que namsdec ne prendra pas.
///
/// Les deux se construisent dans le préfixe de namsdec, donc sans
/// toucher au système. `rz-ghidra` a besoin de trouver le Rizin qu'on
/// vient de poser : d'où les variables d'environnement passées par
/// `env`, plutôt qu'un réglage global qu'il faudrait défaire ensuite.
fn rizin(data: &Path) -> Recipe {
    let racine = tools_dir(data);
    let préfixe = racine.join("rizin");
    let sources = racine.join("src");
    let pkgconfig = préfixe.join("lib64/pkgconfig");
    let pkgconfig2 = préfixe.join("lib/pkgconfig");
    Recipe {
        tool: "rizin-ghidra",
        requires: vec!["git", "meson", "ninja", "cmake", "make"],
        cost_fr: "Une compilation : comptez dix à vingt minutes et quelques centaines de \
                  mégaoctets. Rien n'est installé hors du dossier de namsdec.",
        cost_en: "A build: count ten to twenty minutes and a few hundred megabytes. Nothing is \
                  installed outside namsdec's own directory.",
        source: "https://github.com/rizinorg/rizin et https://github.com/rizinorg/rz-ghidra",
        teaches_fr: "Rizin lit et analyse un binaire ; c'est son greffon rz-ghidra qui décompile, \
                     avec le moteur de Ghidra — celui de la NSA, publié en 2019, et la référence \
                     libre du domaine. D'où les deux constructions : l'outil, puis la pièce qui \
                     lui manque. C'est aussi pourquoi « Rizin est installé » ne veut pas dire \
                     « on peut décompiler ».",
        teaches_en: "Rizin reads and analyses a binary; its rz-ghidra plugin is what decompiles, \
                     using Ghidra's engine — the NSA's, released in 2019, and the field's free \
                     reference. Hence the two builds: the tool, then the piece it lacks. It is \
                     also why « Rizin is installed » does not mean « we can decompile ».",
        produces: préfixe.join("bin/rizin"),
        steps: vec![
            Step {
                about_fr: "Créer le dossier des sources".into(),
                about_en: "Create the sources directory".into(),
                command: "mkdir".into(),
                args: vec!["-p".into(), sources.display().to_string()],
                cwd: None,
            },
            // `git clone` refuse un dossier qui n'est pas vide, et
            // s'arrête à la première ligne. Une tentative précédente —
            // interrompue, ou faite avec d'autres versions — laissait
            // donc l'installation échouer aussitôt, trop vite pour qu'on
            // voie autre chose qu'un bouton qui revient.
            //
            // Effacer plutôt que réutiliser : ce qui est là vient
            // peut-être d'autres étiquettes, et un dépôt à demi mis à
            // jour est plus difficile à comprendre qu'un dépôt neuf.
            Step {
                about_fr: "Effacer les sources d'une tentative précédente".into(),
                about_en: "Remove sources from any earlier attempt".into(),
                command: "rm".into(),
                args: vec![
                    "-rf".into(),
                    sources.join("rizin").display().to_string(),
                    sources.join("rz-ghidra").display().to_string(),
                ],
                cwd: None,
            },
            Step {
                // `--branch` : une version publiée, jamais la branche
                // par défaut. Rizin et rz-ghidra sont deux dépôts que
                // seule leur numérotation apparie : prendre les deux
                // HEAD donnait un Rizin d'hier et un greffon du mois
                // dernier, et le greffon ne compilait plus contre lui —
                // `rz_core_annotated_code_print` avait gagné un
                // paramètre entre les deux.
                //
                // `--depth 1` : on veut le code, pas dix ans d'histoire.
                // `--recursive` : Rizin porte ses dépendances en
                // sous-modules, et sans elles la configuration échoue.
                about_fr: "Récupérer les sources de Rizin".into(),
                about_en: "Fetch Rizin's sources".into(),
                command: "git".into(),
                args: vec![
                    "clone".into(),
                    "--depth".into(),
                    "1".into(),
                    "--branch".into(),
                    RIZIN_TAG.into(),
                    "--recursive".into(),
                    "https://github.com/rizinorg/rizin".into(),
                    sources.join("rizin").display().to_string(),
                ],
                cwd: None,
            },
            Step {
                about_fr: "Configurer la construction de Rizin".into(),
                about_en: "Configure Rizin's build".into(),
                command: "meson".into(),
                args: vec![
                    "setup".into(),
                    "build".into(),
                    format!("--prefix={}", préfixe.display()),
                    "--buildtype=release".into(),
                ],
                cwd: Some(sources.join("rizin")),
            },
            Step {
                about_fr: "Construire et poser Rizin".into(),
                about_en: "Build and place Rizin".into(),
                command: "ninja".into(),
                args: vec!["-C".into(), "build".into(), "install".into()],
                cwd: Some(sources.join("rizin")),
            },
            Step {
                about_fr: "Récupérer les sources du greffon rz-ghidra".into(),
                about_en: "Fetch the rz-ghidra plugin's sources".into(),
                command: "git".into(),
                args: vec![
                    "clone".into(),
                    "--depth".into(),
                    "1".into(),
                    "--branch".into(),
                    RZ_GHIDRA_TAG.into(),
                    "--recursive".into(),
                    "https://github.com/rizinorg/rz-ghidra".into(),
                    sources.join("rz-ghidra").display().to_string(),
                ],
                cwd: None,
            },
            Step {
                about_fr: "Configurer le greffon, pour le Rizin qu'on vient de poser".into(),
                about_en: "Configure the plugin, against the Rizin just placed".into(),
                command: "env".into(),
                args: vec![
                    format!(
                        "PKG_CONFIG_PATH={}:{}",
                        pkgconfig.display(),
                        pkgconfig2.display()
                    ),
                    "cmake".into(),
                    "-S".into(),
                    ".".into(),
                    "-B".into(),
                    "build".into(),
                    format!("-DCMAKE_INSTALL_PREFIX={}", préfixe.display()),
                    "-DCMAKE_BUILD_TYPE=Release".into(),
                    "-DBUILD_CUTTER_PLUGIN=OFF".into(),
                ],
                cwd: Some(sources.join("rz-ghidra")),
            },
            Step {
                about_fr: "Construire et poser le greffon".into(),
                about_en: "Build and place the plugin".into(),
                command: "cmake".into(),
                args: vec![
                    "--build".into(),
                    "build".into(),
                    "--target".into(),
                    "install".into(),
                ],
                cwd: Some(sources.join("rz-ghidra")),
            },
            // Un gigaoctet et demi de sources dont plus personne n'a
            // besoin : l'outil est posé ailleurs, dans son propre
            // dossier. Les garder ne servirait qu'à une reconstruction
            // qui, de toute façon, recommence par les effacer.
            Step {
                about_fr: "Effacer les sources, devenues inutiles".into(),
                about_en: "Remove the sources, now useless".into(),
                command: "rm".into(),
                args: vec![
                    "-rf".into(),
                    sources.join("rizin").display().to_string(),
                    sources.join("rz-ghidra").display().to_string(),
                ],
                cwd: None,
            },
        ],
    }
}

/// Les versions de Rizin et de son greffon, appariées.
///
/// Elles vont ensemble et ne se remontent qu'ensemble : rz-ghidra suit
/// l'API de Rizin, qui change entre deux versions mineures. Prendre les
/// branches par défaut des deux dépôts revenait à tirer à pile ou face
/// sur leur écart du jour.
const RIZIN_TAG: &str = "v0.9.1";
const RZ_GHIDRA_TAG: &str = "v0.9.0";

/// Les deux étiquettes que la recette de rz-ghidra va chercher.
///
/// Exposées pour qu'on puisse les vérifier : un dépôt qui retire une
/// étiquette — cela arrive, une version reprise, un tag déplacé — fait
/// échouer la recette à sa sixième étape, après dix minutes de
/// compilation de Rizin. Le dire d'abord coûte une requête.
pub fn rizin_tags() -> (&'static str, &'static str) {
    (RIZIN_TAG, RZ_GHIDRA_TAG)
}

/// Le dépôt de DZF, et la branche qu'on en prend.
///
/// DZF est le désassembleur-décompilateur maison : il vit dans son propre
/// dépôt, jamais en dépendance de namsdec. Il n'a pas encore de version
/// publiée — c'est un outil qu'on développe —, donc on prend `main`
/// plutôt qu'une étiquette. Le jour où il en aura une, c'est elle qu'on
/// épinglera, comme pour Rizin, pour qu'une installation donne demain ce
/// qu'elle a montré aujourd'hui.
const DZF_REPO: &str = "https://github.com/fredza/DZF";
const DZF_BRANCH: &str = "main";

/// La recette de DZF : cloner le dépôt, le construire, poser les deux
/// binaires qui servent à namsdec.
///
/// `cmake` n'installe que la bibliothèque de DZF, pas ses exécutables :
/// c'est pourquoi la recette ne finit pas par `--target install` comme
/// celle de Rizin, mais copie elle-même `x64-disassembler` (le listing)
/// et `dzf-plugin-engine` (la décompilation JSON) dans le dossier où
/// namsdec cherche ce qu'il a posé — `<tools>/dzf/bin/`.
///
/// `-DDZF_BUILD_TESTS=OFF` : DZF porte une centaine d'exécutables de test
/// qu'on n'a aucune raison de construire ici. On ne veut que les deux
/// binaires de l'intégration.
fn dzf(data: &Path) -> Recipe {
    let racine = tools_dir(data);
    let sources = racine.join("src");
    let dépôt = sources.join("DZF");
    let bin = racine.join("dzf/bin");
    Recipe {
        tool: "dzf",
        requires: vec!["git", "cmake", "make", "g++"],
        cost_fr: "Une compilation en C++23 : quelques minutes et quelques dizaines de mégaoctets. \
                  Rien n'est installé hors du dossier de namsdec.",
        cost_en: "A C++23 build: a few minutes and a few tens of megabytes. Nothing is installed \
                  outside namsdec's own directory.",
        source: "https://github.com/fredza/DZF",
        teaches_fr: "DZF est un désassembleur x86-64 écrit à la main en C++23, sans aucune \
                     dépendance — un outil qu'on développe, et qu'on lit pour comprendre comment \
                     une instruction se décode. Il ne lit pas l'en-tête ELF : namsdec lui passe \
                     les octets exécutables et leur adresse, et c'est tout ce dont il a besoin.",
        teaches_en: "DZF is an x86-64 disassembler written by hand in C++23, with no dependencies \
                     — a tool under development, and one you read to understand how an instruction \
                     is decoded. It does not read the ELF header: namsdec hands it the executable \
                     bytes and their address, and that is all it needs.",
        produces: bin.join("x64-disassembler"),
        steps: vec![
            Step {
                about_fr: "Créer le dossier des sources".into(),
                about_en: "Create the sources directory".into(),
                command: "mkdir".into(),
                args: vec!["-p".into(), sources.display().to_string()],
                cwd: None,
            },
            // Effacer d'abord, comme pour Rizin : `git clone` refuse un
            // dossier non vide, et un dépôt à demi cloné d'une tentative
            // interrompue ferait échouer l'étape suivante sans qu'on
            // sache pourquoi.
            Step {
                about_fr: "Effacer les sources d'une tentative précédente".into(),
                about_en: "Remove sources from any earlier attempt".into(),
                command: "rm".into(),
                args: vec!["-rf".into(), dépôt.display().to_string()],
                cwd: None,
            },
            // `--depth 1` : on veut le code, pas l'histoire. Pas de
            // `--recursive` : DZF n'a aucun sous-module, c'est l'un de
            // ses partis pris.
            Step {
                about_fr: "Récupérer les sources de DZF".into(),
                about_en: "Fetch DZF's sources".into(),
                command: "git".into(),
                args: vec![
                    "clone".into(),
                    "--depth".into(),
                    "1".into(),
                    "--branch".into(),
                    DZF_BRANCH.into(),
                    DZF_REPO.into(),
                    dépôt.display().to_string(),
                ],
                cwd: None,
            },
            Step {
                about_fr: "Configurer la construction de DZF".into(),
                about_en: "Configure DZF's build".into(),
                command: "cmake".into(),
                args: vec![
                    "-S".into(),
                    "x64-disassembler".into(),
                    "-B".into(),
                    "build".into(),
                    "-DCMAKE_BUILD_TYPE=Release".into(),
                    "-DDZF_BUILD_TESTS=OFF".into(),
                ],
                cwd: Some(dépôt.clone()),
            },
            Step {
                about_fr: "Construire DZF".into(),
                about_en: "Build DZF".into(),
                command: "cmake".into(),
                args: vec!["--build".into(), "build".into(), "--parallel".into()],
                cwd: Some(dépôt.clone()),
            },
            Step {
                about_fr: "Créer le dossier du binaire".into(),
                about_en: "Create the binary's directory".into(),
                command: "mkdir".into(),
                args: vec!["-p".into(), bin.display().to_string()],
                cwd: None,
            },
            // Les deux binaires que namsdec emploie : le désassembleur,
            // moteur possible du listing, et le moteur de décompilation
            // JSON. `cmake` les laisse dans `build/` sans les installer,
            // on les prend donc à la main.
            Step {
                about_fr: "Poser les binaires de DZF".into(),
                about_en: "Place DZF's binaries".into(),
                command: "cp".into(),
                args: vec![
                    dépôt.join("build/x64-disassembler").display().to_string(),
                    dépôt.join("build/dzf-plugin-engine").display().to_string(),
                    bin.display().to_string(),
                ],
                cwd: None,
            },
            // Les sources, devenues inutiles : les binaires sont posés
            // ailleurs, et une reconstruction recommencerait de toute
            // façon par les effacer.
            Step {
                about_fr: "Effacer les sources, devenues inutiles".into(),
                about_en: "Remove the sources, now useless".into(),
                command: "rm".into(),
                args: vec!["-rf".into(), dépôt.display().to_string()],
                cwd: None,
            },
        ],
    }
}

/// Cette étiquette existe-t-elle encore dans ce dépôt ?
///
/// `None` quand on ne peut pas conclure — pas de `git`, pas de réseau,
/// une erreur : on ne suppose pas qu'une étiquette a disparu parce que
/// personne n'a répondu. `Some(false)` veut dire « git a répondu, et il
/// ne la trouve pas ».
///
/// `ls-remote` plutôt qu'un clone : c'est une question, pas un
/// téléchargement, et elle se pose en une seconde.
pub fn tag_exists(repo: &str, tag: &str) -> Option<bool> {
    let git = crate::known::program_path_in("git", None)?;
    let sortie = std::process::Command::new(git)
        .args([
            "ls-remote",
            "--tags",
            "--exit-code",
            repo,
            &format!("refs/tags/{tag}"),
        ])
        .output()
        .ok()?;
    match sortie.status.code() {
        // 0 : trouvée. 2 : `--exit-code` dit qu'il n'y a pas de
        // correspondance, ce qui est une réponse et non une panne.
        Some(0) => Some(true),
        Some(2) => Some(false),
        // Tout le reste — réseau coupé, dépôt injoignable — ne dit rien
        // de l'étiquette.
        _ => None,
    }
}

/// La commande d'installation d'un outil, dite pour cette machine-ci.
///
/// Les outils connus annoncent leur paquet en `dnf`, faute de pouvoir
/// deviner la distribution à l'écriture. Devant un autre gestionnaire, la
/// ligne était fausse — et l'on ne s'en apercevait qu'en la tapant. On la
/// retraduit ici, en gardant les noms de paquets et en changeant ce qui
/// les précède.
///
/// Ce qui ne ressemble pas à une commande `dnf` est rendu tel quel : une
/// adresse de page de téléchargement n'a pas de gestionnaire.
pub fn install_command_here(annoncée: &str) -> String {
    const PRÉFIXE: &str = "sudo dnf install ";
    let Some(paquets) = annoncée.strip_prefix(PRÉFIXE) else {
        return annoncée.to_owned();
    };
    let paquets: Vec<&str> = paquets.split_whitespace().collect();
    if crate::known::program_path("dnf").is_some() {
        return annoncée.to_owned();
    }
    if crate::known::program_path("apt-get").is_some() {
        return format!("sudo apt-get install {}", paquets.join(" "));
    }
    if crate::known::program_path("pacman").is_some() {
        return format!("sudo pacman -S {}", paquets.join(" "));
    }
    if crate::known::program_path("zypper").is_some() {
        return format!("sudo zypper install {}", paquets.join(" "));
    }
    // Aucun gestionnaire connu : mieux vaut la ligne d'origine, qui dit
    // au moins quels paquets chercher, qu'une invention.
    annoncée.to_owned()
}

/// La commande qui poserait les prérequis manquants, pour cette machine.
///
/// namsdec ne la lance pas : elle demande `sudo`, et prendre les droits
/// de quelqu'un pour installer des paquets système n'est pas son rôle.
/// Mais la lui montrer toute faite lui épargne de chercher le nom du
/// paquet — qui n'est presque jamais celui du programme : `ninja` est
/// fourni par `ninja-build` sur Fedora comme sur Debian, et `sha256sum`
/// par `coreutils` partout.
///
/// `None` quand aucun gestionnaire connu n'est là : mieux vaut ne rien
/// dire qu'une commande qui ne marchera pas ici.
pub fn requirement_command(missing: &[&str]) -> Option<String> {
    let (gestionnaire, installer, paquet): (&str, &str, fn(&str) -> &str) =
        if crate::known::program_path("dnf").is_some() {
            ("dnf", "install", paquet_fedora)
        } else if crate::known::program_path("apt-get").is_some() {
            ("apt-get", "install", paquet_debian)
        } else if crate::known::program_path("pacman").is_some() {
            ("pacman", "-S", paquet_arch)
        } else if crate::known::program_path("zypper").is_some() {
            ("zypper", "install", paquet_fedora)
        } else {
            return None;
        };

    let mut paquets: Vec<&str> = missing.iter().map(|m| paquet(m)).collect();
    paquets.sort_unstable();
    paquets.dedup();
    if paquets.is_empty() {
        return None;
    }
    Some(format!(
        "sudo {gestionnaire} {installer} {}",
        paquets.join(" ")
    ))
}

fn paquet_fedora(programme: &str) -> &str {
    match programme {
        "ninja" => "ninja-build",
        "sha256sum" => "coreutils",
        "g++" | "gcc" => "gcc-c++",
        autre => autre,
    }
}

fn paquet_debian(programme: &str) -> &str {
    match programme {
        "ninja" => "ninja-build",
        "sha256sum" => "coreutils",
        "g++" | "gcc" => "g++",
        autre => autre,
    }
}

fn paquet_arch(programme: &str) -> &str {
    match programme {
        "sha256sum" => "coreutils",
        "g++" | "gcc" => "gcc",
        autre => autre,
    }
}

/// La recette d'un outil, quand namsdec sait l'installer.
pub fn recipe_for(tool: &str, data_dir: &Path) -> Option<Recipe> {
    match tool {
        "retdec" => Some(retdec(data_dir)),
        "rizin-ghidra" => Some(rizin(data_dir)),
        // DZF vit dans son propre dépôt : namsdec sait le cloner et le
        // construire, chez l'utilisateur, sans rien installer à l'échelle
        // du système.
        "dzf" => Some(dzf(data_dir)),
        // Les autres viennent des paquets du système : les installer
        // demanderait `sudo`, que namsdec ne prendra pas.
        _ => None,
    }
}

/// Les prérequis qui manquent sur cette machine.
pub fn missing_requirements(recipe: &Recipe) -> Vec<&'static str> {
    recipe
        .requires
        .iter()
        .filter(|programme| crate::known::program_path(programme).is_none())
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ce qu'une recette promet, elle le dit : d'où ça vient, ce que ça
    /// coûte, ce qu'il faut avoir.
    #[test]
    fn a_recipe_says_where_it_goes_and_what_it_costs() {
        let data = Path::new("/tmp/namsdec-test");
        for outil in ["retdec", "rizin-ghidra", "dzf"] {
            let recette = recipe_for(outil, data).expect("une recette");
            assert!(!recette.steps.is_empty());
            assert!(recette.source.starts_with("https://"), "{}", recette.source);
            assert!(!recette.cost_fr.is_empty() && !recette.cost_en.is_empty());
            assert!(!recette.requires.is_empty());
            // Tout ce qu'elle produit reste sous le dossier de namsdec.
            assert!(
                recette.produces.starts_with(tools_dir(data)),
                "{}",
                recette.produces.display()
            );
        }
    }

    /// Aucune étape ne demande les droits d'administrateur, et aucune ne
    /// passe par un interpréteur de commandes.
    ///
    /// C'est la limite que ce module s'impose : ce qu'il fait, il le fait
    /// chez l'utilisateur, et ce qu'il lance est exactement ce qu'il a
    /// montré — pas une chaîne qu'un interpréteur relirait.
    #[test]
    fn nothing_asks_for_root_and_nothing_goes_through_a_shell() {
        let data = Path::new("/tmp/namsdec-test");
        for outil in ["retdec", "rizin-ghidra", "dzf"] {
            let recette = recipe_for(outil, data).expect("une recette");
            for étape in &recette.steps {
                assert_ne!(étape.command, "sudo", "{}", étape.line());
                assert_ne!(étape.command, "su", "{}", étape.line());
                assert!(
                    !matches!(étape.command.as_str(), "sh" | "bash" | "zsh"),
                    "{}",
                    étape.line()
                );
                // Ni redirection, ni tube, ni substitution : ces signes
                // n'auraient de sens que pour un interpréteur, et il n'y
                // en a pas.
                for argument in &étape.args {
                    assert!(
                        !argument.contains('|')
                            && !argument.contains('>')
                            && !argument.contains("$("),
                        "{argument} ressemble à du shell"
                    );
                }
            }
        }
    }

    /// Ce qu'on télécharge vient d'un dépôt nommé, en HTTPS, et à une
    /// version épinglée.
    #[test]
    fn what_is_downloaded_is_pinned_and_over_https() {
        let recette = recipe_for("retdec", Path::new("/tmp/x")).expect("retdec");
        let téléchargement = recette
            .steps
            .iter()
            .find(|é| é.command == "curl")
            .expect("une étape de téléchargement");
        let url = téléchargement.args.last().expect("une adresse");
        assert!(url.starts_with("https://github.com/avast/retdec/"), "{url}");
        assert!(
            url.contains(RETDEC_VERSION),
            "la version est épinglée : {url}"
        );
        assert!(
            téléchargement.args.iter().any(|a| a == "=https"),
            "et rien d'autre que HTTPS n'est accepté"
        );
        // L'empreinte de ce qui arrive est calculée, pour être comparée.
        assert!(
            recette.steps.iter().any(|é| é.command == "sha256sum"),
            "l'empreinte doit être calculée"
        );
    }

    /// Un outil qu'on ne sait pas installer le dit, plutôt que d'inventer
    /// une recette.
    #[test]
    fn an_unknown_tool_has_no_recipe() {
        assert!(recipe_for("objdump", Path::new("/tmp/x")).is_none());
        assert!(recipe_for("ce-qui-nexiste-pas", Path::new("/tmp/x")).is_none());
    }

    /// DZF se clone depuis son dépôt, se construit, et les deux binaires
    /// de l'intégration se posent là où namsdec les cherchera.
    #[test]
    fn dzf_is_cloned_built_and_its_binaries_are_placed() {
        let data = Path::new("/tmp/namsdec-test");
        let recette = recipe_for("dzf", data).expect("une recette pour DZF");

        // Le binaire déclaré est le désassembleur, sous le dossier d'outils.
        assert!(
            recette.produces.ends_with("dzf/bin/x64-disassembler"),
            "{}",
            recette.produces.display()
        );
        assert!(recette.produces.starts_with(tools_dir(data)));

        // On va bien chercher le dépôt de DZF, en HTTPS.
        let clone = recette
            .steps
            .iter()
            .find(|é| é.command == "git" && é.args.iter().any(|a| a == "clone"))
            .expect("une étape de clonage");
        assert!(
            clone.args.iter().any(|a| a == DZF_REPO),
            "le dépôt cloné est celui de DZF : {}",
            clone.line()
        );
        assert!(DZF_REPO.starts_with("https://"), "{DZF_REPO}");

        // On construit avec cmake, sans construire les tests de DZF.
        let config = recette
            .steps
            .iter()
            .find(|é| é.command == "cmake" && é.args.iter().any(|a| a == "-S"))
            .expect("une étape de configuration");
        assert!(
            config.args.iter().any(|a| a == "-DDZF_BUILD_TESTS=OFF"),
            "on ne construit pas les tests de DZF : {}",
            config.line()
        );

        // Les DEUX binaires de l'intégration sont copiés : le listing et
        // le moteur de décompilation.
        let copie = recette
            .steps
            .iter()
            .find(|é| é.command == "cp")
            .expect("une étape de copie");
        assert!(
            copie.args.iter().any(|a| a.ends_with("x64-disassembler"))
                && copie.args.iter().any(|a| a.ends_with("dzf-plugin-engine")),
            "les deux binaires sont posés : {}",
            copie.line()
        );
    }
}

#[cfg(test)]
mod reachability {
    use super::*;

    /// Ce qu'une recette télécharge existe vraiment.
    ///
    /// La première écriture de la recette de RetDec nommait un fichier
    /// qui n'a jamais été publié : `curl` aurait rapporté une page
    /// d'erreur, `tar` s'en serait plaint, et l'utilisateur aurait cru à
    /// un défaut de sa machine. Une adresse écrite de mémoire n'est pas
    /// une adresse ; celle-ci est vérifiée.
    ///
    /// Le test ne tourne que si le réseau répond, et ne télécharge rien :
    /// il demande les en-têtes. Sans réseau, il se tait plutôt que
    /// d'échouer — un test qui casse dans un train n'apprend rien.
    #[test]
    fn what_a_recipe_downloads_really_exists() {
        let Some(curl) = crate::known::program_path("curl") else {
            return;
        };
        let data = Path::new("/tmp/namsdec-verif");
        for outil in ["retdec"] {
            let recette = recipe_for(outil, data).expect("une recette");
            let Some(étape) = recette.steps.iter().find(|é| é.command == "curl") else {
                continue;
            };
            let url = étape.args.last().expect("une adresse").clone();
            let sortie = std::process::Command::new(&curl)
                // `-I` : les en-têtes seulement. On vérifie qu'il y a
                // quelque chose au bout, pas qu'on peut le rapatrier.
                .args(["-fsIL", "--max-time", "20", "--proto", "=https", &url])
                .output();
            let Ok(sortie) = sortie else { return };
            if !sortie.status.success() && sortie.stdout.is_empty() {
                // Pas de réseau : on ne conclut rien.
                return;
            }
            let entêtes = String::from_utf8_lossy(&sortie.stdout);
            assert!(
                entêtes.contains("200"),
                "{url} ne répond pas 200 : la recette téléchargerait une page d'erreur\n{entêtes}"
            );
        }
    }

    /// Et les dépôts qu'une recette clone existent aussi.
    #[test]
    fn what_a_recipe_clones_really_exists() {
        let Some(git) = crate::known::program_path("git") else {
            return;
        };
        let data = Path::new("/tmp/namsdec-verif");
        let recette = recipe_for("rizin-ghidra", data).expect("une recette");
        for étape in recette.steps.iter().filter(|é| é.command == "git") {
            let dépôt = étape
                .args
                .iter()
                .find(|a| a.starts_with("https://"))
                .expect("une adresse de dépôt");
            let sortie = std::process::Command::new(&git)
                .args(["ls-remote", "--exit-code", dépôt, "HEAD"])
                .output();
            let Ok(sortie) = sortie else { return };
            // Sans réseau, `git` se plaint autrement qu'en disant que le
            // dépôt n'existe pas : on ne conclut que sur ce cas-là.
            let erreur = String::from_utf8_lossy(&sortie.stderr).to_lowercase();
            if erreur.contains("could not resolve") || erreur.contains("network") {
                return;
            }
            assert!(
                sortie.status.success(),
                "{dépôt} est introuvable : la recette échouerait au clonage\n{erreur}"
            );
        }
    }
    /// Une recette se rejoue sur un dossier déjà occupé.
    ///
    /// `git clone` refuse un dossier qui n'est pas vide et s'arrête à la
    /// première ligne : une tentative précédente — interrompue, ou faite
    /// avec d'autres étiquettes — faisait échouer l'installation en une
    /// fraction de seconde, trop vite pour qu'on voie autre chose qu'un
    /// bouton qui revient.
    #[test]
    fn a_recipe_can_be_run_again_over_what_a_previous_attempt_left() {
        let data = std::path::Path::new("/tmp/namsdec-test");
        for outil in ["rizin-ghidra", "retdec"] {
            let recette = recipe_for(outil, data).expect("une recette");
            // Ce qu'une tentative précédente a pu laisser doit être
            // effacé avant d'être réécrit.
            let efface: Vec<&Step> = recette
                .steps
                .iter()
                .filter(|étape| étape.command == "rm")
                .collect();
            assert!(
                !efface.is_empty(),
                "{outil} : rien n'efface ce qu'une tentative précédente a laissé"
            );

            // Et cet effacement précède ce qui écrit.
            let premier_rm = recette
                .steps
                .iter()
                .position(|é| é.command == "rm")
                .expect("une étape d'effacement");
            for (rang, étape) in recette.steps.iter().enumerate() {
                if étape.command == "git" && étape.args.first().map(String::as_str) == Some("clone")
                {
                    assert!(
                        premier_rm < rang,
                        "{outil} : on efface après avoir cloné, ce qui ne sert à rien"
                    );
                }
            }

            // Ce qu'on efface reste sous le dossier d'outils : une
            // recette ne touche à rien d'autre.
            let outils = tools_dir(data);
            for étape in efface {
                for cible in étape.args.iter().filter(|a| !a.starts_with('-')) {
                    assert!(
                        std::path::Path::new(cible).starts_with(&outils),
                        "{outil} : {cible} est hors de {}",
                        outils.display()
                    );
                }
            }
        }
    }

    /// Un paquet qui n'existe pas ne doit pas être promis.
    ///
    /// La fiche de Rizin annonçait « sudo dnf install rizin
    /// rizin-plugin-ghidra ». Le second paquet n'existe dans aucun dépôt
    /// Fedora : la commande échouait en bloc, sans rien installer — pas
    /// même Rizin, dnf refusant la transaction entière. Le greffon
    /// rz-ghidra n'est empaqueté nulle part ; c'est justement pourquoi
    /// namsdec sait le bâtir.
    #[test]
    fn the_rizin_entry_promises_only_a_package_that_exists() {
        let rizin = crate::known::all()
            .into_iter()
            .find(|connu| connu.plugin.id == "rizin-ghidra")
            .expect("la fiche de Rizin");
        let commande = rizin.plugin.install.expect("une façon de l'obtenir");
        assert!(
            !commande.contains("rizin-plugin-ghidra"),
            "ce paquet n'existe pas : {commande}"
        );
        assert!(commande.contains("rizin"), "{commande}");
        // Et la recette de construction reste le seul chemin vers le
        // greffon : elle doit exister pour cet outil.
        assert!(
            recipe_for("rizin-ghidra", std::path::Path::new("/tmp")).is_some(),
            "sans paquet, il faut une recette"
        );
    }

    /// La commande d'installation se dit dans le gestionnaire qu'on a.
    #[test]
    fn an_install_line_speaks_the_local_package_manager() {
        let dite = install_command_here("sudo dnf install radare2");
        assert!(dite.starts_with("sudo "), "{dite}");
        assert!(dite.contains("radare2"), "le paquet ne change pas : {dite}");
        // Le gestionnaire, lui, est celui de cette machine.
        for (outil, attendu) in [
            ("dnf", "dnf install"),
            ("apt-get", "apt-get install"),
            ("pacman", "pacman -S"),
            ("zypper", "zypper install"),
        ] {
            if crate::known::program_path(outil).is_some() {
                assert!(dite.contains(attendu), "{outil} attendu dans {dite}");
                break;
            }
        }
        // Ce qui n'est pas une commande de paquets passe tel quel.
        assert_eq!(
            install_command_here("github.com/avast/retdec/releases"),
            "github.com/avast/retdec/releases"
        );
    }

    /// Les prérequis manquants deviennent une commande qu'on peut coller.
    ///
    /// Le nom du paquet n'est presque jamais celui du programme : c'est
    /// tout l'intérêt de la proposer toute faite plutôt que de laisser
    /// chercher « ninja » dans un gestionnaire qui l'appelle autrement.
    #[test]
    fn missing_requirements_become_a_command_one_can_paste() {
        let Some(commande) = requirement_command(&["meson", "ninja"]) else {
            // Aucun gestionnaire connu sur cette machine : rien à dire,
            // et c'est ce que la fonction répond.
            return;
        };
        assert!(commande.starts_with("sudo "), "{commande}");
        assert!(commande.contains("meson"), "{commande}");
        // ninja est fourni par ninja-build sur Fedora comme sur Debian.
        assert!(
            commande.contains("ninja"),
            "le paquet qui fournit ninja doit y être : {commande}"
        );
        // Rien à installer, rien à dire.
        assert_eq!(requirement_command(&[]), None);
    }
}

#[cfg(test)]
mod tag_tests {
    use super::*;

    /// Les deux étiquettes épinglées existent encore.
    ///
    /// Elles vont ensemble et ne se remontent qu'ensemble : rz-ghidra
    /// suit l'API de Rizin, qui change entre deux versions mineures. Si
    /// l'une disparaît du dépôt — version reprise, tag déplacé —, la
    /// recette échoue à sa sixième étape, après dix minutes de
    /// compilation de Rizin. Autant le savoir ici.
    ///
    /// Le test se tait sans réseau : `None` n'est pas un échec, c'est
    /// une absence de réponse.
    #[test]
    fn both_pinned_tags_still_exist() {
        let (rizin, ghidra) = rizin_tags();
        for (dépôt, étiquette) in [
            ("https://github.com/rizinorg/rizin", rizin),
            ("https://github.com/rizinorg/rz-ghidra", ghidra),
        ] {
            if let Some(false) = tag_exists(dépôt, étiquette) {
                panic!(
                    "{dépôt} n'a plus l'étiquette {étiquette} : la recette échouerait \
                     après dix minutes de compilation"
                );
            }
        }
    }

    /// Et une étiquette inventée est bien vue comme absente.
    ///
    /// Sans quoi le contrôle ci-dessus passerait toujours, quoi qu'il
    /// arrive — un test qui ne peut pas échouer ne vérifie rien.
    #[test]
    fn a_made_up_tag_is_reported_missing() {
        let réponse = tag_exists(
            "https://github.com/rizinorg/rizin",
            "v0.0.0-cette-version-n-existe-pas",
        );
        assert!(
            réponse != Some(true),
            "une étiquette inventée ne doit pas être trouvée"
        );
    }
}
