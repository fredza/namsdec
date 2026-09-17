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
            Step {
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
        ],
    }
}

/// La recette d'un outil, quand namsdec sait l'installer.
pub fn recipe_for(tool: &str, data_dir: &Path) -> Option<Recipe> {
    match tool {
        "retdec" => Some(retdec(data_dir)),
        "rizin-ghidra" => Some(rizin(data_dir)),
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
        for outil in ["retdec", "rizin-ghidra"] {
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
        for outil in ["retdec", "rizin-ghidra"] {
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
}
