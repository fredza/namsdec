//! Les outils qu'on connaît, prêts à être déclarés.
//!
//! Écrire un fichier JSON à la main pour brancher `rizin` est une
//! barrière absurde : les arguments qui marchent sont connus, et ils
//! sont les mêmes pour tout le monde. namsdec porte donc leur
//! description, et l'installation se réduit à un clic — qui **écrit un
//! fichier**, rien de plus.
//!
//! Ce que namsdec ne fait pas, et ne fera pas : télécharger et installer
//! l'outil lui-même. Exécuter du code au nom de quelqu'un n'est pas une
//! commodité, c'est une décision. Il dit donc si l'outil est là, ce
//! qu'il faut taper pour l'obtenir, et laisse taper.

use std::path::{Path, PathBuf};

use crate::plugin::Plugin;

/// Ce qu'on sait d'un outil sur cette machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Le programme est là, et il fait ce qu'on attend.
    Ready,
    /// Le programme est là, mais il lui manque une pièce — `rizin` sans
    /// `rz-ghidra` ne sait pas décompiler, et le dire vaut mieux que de
    /// laisser essayer.
    Incomplete,
    /// Pas de programme de ce nom dans le PATH.
    Missing,
}

/// Un outil connu : ce qu'il est, comment on l'appelle, comment on
/// l'obtient.
#[derive(Debug, Clone)]
pub struct Known {
    pub plugin: Plugin,
    /// Une phrase sur ce qu'il rend : des instructions, du pseudo-C.
    pub about_fr: &'static str,
    pub about_en: &'static str,
    /// La pièce dont il a besoin en plus, quand il y en a une.
    pub needs_fr: Option<&'static str>,
    pub needs_en: Option<&'static str>,
}

/// Tous les outils que namsdec sait déclarer.
///
/// Les arguments ne sont pas inventés : chacun est celui qui donne le
/// résultat attendu, et les commentaires disent pourquoi il est là.
pub fn all() -> Vec<Known> {
    vec![
        Known {
            plugin: Plugin {
                id: "objdump".to_owned(),
                name: "objdump — désassemblage (Intel)".to_owned(),
                command: "objdump".to_owned(),
                // `--no-show-raw-insn` enlève les octets : on les a déjà
                // dans l'onglet Octets, et ils doublent la largeur.
                args: vec![
                    "-d".to_owned(),
                    "-M".to_owned(),
                    "intel".to_owned(),
                    "--no-show-raw-insn".to_owned(),
                    "{file}".to_owned(),
                ],
                install: Some("sudo dnf install binutils".to_owned()),
            },
            about_fr: "Le désassembleur des binutils : sûr, partout, et il connaît les sections.",
            about_en: "The binutils disassembler: reliable, everywhere, and it knows sections.",
            needs_fr: None,
            needs_en: None,
        },
        Known {
            plugin: Plugin {
                id: "ndisasm".to_owned(),
                name: "ndisasm — désassemblage brut".to_owned(),
                command: "ndisasm".to_owned(),
                args: vec!["-b".to_owned(), "64".to_owned(), "{file}".to_owned()],
                install: Some("sudo dnf install nasm".to_owned()),
            },
            about_fr: "Celui de nasm. Il lit les octets d'affilée, sans se soucier des sections : \
                       utile devant du code sans en-tête, trompeur devant un ELF.",
            about_en: "The one from nasm. It reads bytes straight through, with no regard for \
                       sections: useful for headerless code, misleading on an ELF.",
            needs_fr: None,
            needs_en: None,
        },
        Known {
            plugin: Plugin {
                id: "radare2".to_owned(),
                name: "radare2 — pseudo-code (pdc)".to_owned(),
                command: "r2".to_owned(),
                // `-N` saute le fichier de configuration personnel : ce
                // qu'on voit ici ne doit pas dépendre des réglages de
                // quelqu'un. `scr.color=0` retire les couleurs, qui
                // arriveraient sous forme de codes d'échappement.
                // `aaa` analyse, `s entry0` va au point d'entrée, `af`
                // définit la fonction si elle manque, `pdc` la rend en
                // pseudo-code.
                args: vec![
                    "-N".to_owned(),
                    "-q".to_owned(),
                    "-e".to_owned(),
                    "scr.color=0".to_owned(),
                    "-c".to_owned(),
                    "aaa; s entry0; af; pdc".to_owned(),
                    "--".to_owned(),
                    "{file}".to_owned(),
                ],
                install: Some("sudo dnf install radare2".to_owned()),
            },
            about_fr: "Le pseudo-code intégré de radare2 : moins fidèle qu'un vrai décompilateur, \
                       et il ne demande aucun greffon.",
            about_en: "radare2's built-in pseudo-code: less faithful than a real decompiler, and \
                       it needs no plugin.",
            needs_fr: None,
            needs_en: None,
        },
        Known {
            plugin: Plugin {
                id: "rizin-ghidra".to_owned(),
                name: "Rizin + rz-ghidra — décompilation (pdg)".to_owned(),
                command: "rizin".to_owned(),
                // Même script que pour radare2, mais `pdg` : c'est le
                // décompilateur de Ghidra, apporté par le greffon
                // rz-ghidra. Sans lui, rizin répond que la commande
                // n'existe pas.
                args: vec![
                    "-N".to_owned(),
                    "-q".to_owned(),
                    "-e".to_owned(),
                    "scr.color=0".to_owned(),
                    "-c".to_owned(),
                    "aaa; s entry0; af; pdg".to_owned(),
                    "--".to_owned(),
                    "{file}".to_owned(),
                ],
                install: Some("sudo dnf install rizin".to_owned()),
            },
            about_fr: "Le décompilateur de Ghidra, porté dans Rizin. C'est le meilleur résultat \
                       qu'on obtienne sans installer Ghidra lui-même.",
            about_en: "Ghidra's decompiler, ported into Rizin. This is the best result one gets \
                       without installing Ghidra itself.",
            needs_fr: Some("le greffon rz-ghidra, sans lequel la commande pdg n'existe pas"),
            needs_en: Some("the rz-ghidra plugin, without which the pdg command does not exist"),
        },
        Known {
            plugin: Plugin {
                id: "retdec".to_owned(),
                name: "RetDec — décompilation en C".to_owned(),
                command: "retdec-decompiler".to_owned(),
                // RetDec écrit dans un fichier, d'où `{out}` : namsdec
                // lui en fournit un et lit ce qu'il y a mis.
                // `--cleanup` efface les fichiers de travail qu'il sème
                // à côté.
                args: vec![
                    "--cleanup".to_owned(),
                    "-o".to_owned(),
                    "{out}".to_owned(),
                    "{file}".to_owned(),
                ],
                install: Some("github.com/avast/retdec/releases".to_owned()),
            },
            about_fr: "Le décompilateur d'Avast. Lent — comptez la minute — et le seul de cette \
                       liste à rendre du C compilable.",
            about_en: "Avast's decompiler. Slow — count a minute — and the only one here that \
                       returns compilable C.",
            needs_fr: None,
            needs_en: None,
        },
        // DZF est dans la liste bien que ses fonctions soient encore
        // minimales, et c'est voulu : un outil qu'on développe se
        // reconnaît d'abord à ce qu'il est absent de la machine. La
        // fiche dit alors où le prendre, et le jour où il est bâti,
        // namsdec le voit sans qu'on ait rien à déclarer.
        //
        // Il ne lit pas l'ELF, et n'a pas à l'apprendre : il reçoit
        // `{text}` — les octets exécutables seuls — et `{base}`, leur
        // adresse. C'est namsdec qui découpe. Cette fiche-ci est la
        // même que `plugins/fonctionnalites/dzf.json`, qui reste le
        // moyen de l'installer à la main ou d'en changer les options.
        Known {
            plugin: Plugin {
                id: "dzf".to_owned(),
                name: "DZF — désassemblage x86-64 (maison)".to_owned(),
                command: "x64-disassembler".to_owned(),
                args: vec![
                    "--base".to_owned(),
                    "{base}".to_owned(),
                    "{text}".to_owned(),
                ],
                install: Some(
                    "cd DZF/x64-disassembler && cmake -S . -B build && cmake --build build"
                        .to_owned(),
                ),
            },
            about_fr: "Désassembleur x86-64 écrit à la main en C++23, sans aucune dépendance. \
                       En cours de développement : il ne couvre pas encore tout le jeu \
                       d'instructions, et il n'a pas à savoir lire un ELF — namsdec lui donne \
                       les octets exécutables et leur adresse.",
            about_en: "An x86-64 disassembler written by hand in C++23, with no dependencies. \
                       Under development: it does not yet cover the whole instruction set, and \
                       it need not know how to read an ELF — namsdec hands it the executable \
                       bytes and their address.",
            needs_fr: None,
            needs_en: None,
        },
    ]
}

/// Le programme existe-t-il dans le PATH ?
///
/// On cherche à la main plutôt que d'appeler `which` : lancer un
/// programme pour savoir si l'on peut en lancer un autre serait un
/// détour, et un `which` absent ferait croire que rien n'est installé.
pub fn program_path(command: &str) -> Option<PathBuf> {
    program_path_in(command, None)
}

/// Comme [`program_path`], mais en regardant d'abord là où namsdec pose
/// ce qu'il installe lui-même.
///
/// Un outil bâti par le module d'installation vit dans
/// `<tools>/<outil>/bin/`, qui n'est dans le PATH de personne. Son chemin
/// n'était retenu que par la déclaration faite en fin d'installation : une
/// installation interrompue, ou dont une étape tardive échouait, laissait
/// donc sur le disque un programme parfaitement fonctionnel que namsdec
/// déclarait « absent ». On regarde le disque plutôt que de se fier à ce
/// qu'on a noté.
pub fn program_path_in(command: &str, tools: Option<&Path>) -> Option<PathBuf> {
    let chemin = std::path::Path::new(command);
    if chemin.is_absolute() {
        return chemin.is_file().then(|| chemin.to_path_buf());
    }
    if let Some(tools) = tools
        && let Some(trouvé) = installed_program(command, tools)
    {
        return Some(trouvé);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidat = dir.join(command);
        candidat.is_file().then_some(candidat)
    })
}

/// Les paquets qu'une commande d'installation nomme.
///
/// `"sudo dnf install rizin"` rend `["rizin"]`. Rien pour ce qui n'est
/// pas une commande de gestionnaire — l'adresse d'une page de
/// téléchargement, par exemple : il n'y a alors aucun nom à vérifier.
///
/// Sert au contrôle ci-dessous, et il a sa raison d'être : une recette
/// annonçait `rizin-plugin-ghidra`, qui n'existe dans aucun dépôt
/// Fedora. Comme dnf refuse la transaction entière, la commande
/// n'installait même pas Rizin — et rien ne le disait.
pub fn packages_named(install: &str) -> Vec<&str> {
    let mots: Vec<&str> = install.split_whitespace().collect();
    // D'abord : est-ce seulement une commande de gestionnaire ?
    //
    // Sans cette question, `cmake -S . -B build` était lu comme un
    // `pacman -S` et l'on partait chercher des paquets nommés « . » et
    // « build » dans les dépôts. Le verbe seul ne suffit pas à
    // reconnaître une installation — c'est le gestionnaire qui la
    // nomme, et une ligne qui n'en cite aucun n'annonce aucun paquet.
    const GESTIONNAIRES: [&str; 8] = [
        "dnf", "yum", "apt", "apt-get", "pacman", "zypper", "apk", "brew",
    ];
    if !mots.iter().any(|mot| GESTIONNAIRES.contains(mot)) {
        return Vec::new();
    }
    // Après `install`, `-S`, `add` : le verbe du gestionnaire.
    let Some(rang) = mots
        .iter()
        .position(|mot| matches!(*mot, "install" | "add" | "-S"))
    else {
        return Vec::new();
    };
    mots[rang + 1..]
        .iter()
        .copied()
        // Les options ne sont pas des paquets.
        .filter(|mot| !mot.starts_with('-'))
        .collect()
}

/// Ce que le gestionnaire local sait de ce paquet.
///
/// `Some(false)` veut dire « il a répondu, et il ne le connaît pas » :
/// c'est le seul cas qui compte, celui d'un nom annoncé qui n'existe
/// nulle part. `None` couvre tout le reste — pas de gestionnaire connu,
/// pas de cache, une erreur : on ne conclut rien de l'absence de réponse.
///
/// Interrogé **sans réseau** (`--cacheonly`) et sans droits : un contrôle
/// qui exigerait l'un ou l'autre ne serait pas lancé, donc ne servirait
/// à rien.
pub fn package_known(name: &str) -> Option<bool> {
    let dnf = program_path_in("dnf", None)?;
    let sortie = std::process::Command::new(dnf)
        .args(["-q", "--cacheonly", "repoquery", "--qf", "%{name}", name])
        .output()
        .ok()?;
    if !sortie.status.success() {
        return None;
    }
    let texte = String::from_utf8_lossy(&sortie.stdout);
    // Une sortie vide veut dire « connais pas ». Encore faut-il que le
    // cache existe : sans lui, dnf répond vide pour tout, et l'on
    // conclurait que rien n'existe.
    if texte.trim().is_empty() {
        let témoin = std::process::Command::new(program_path_in("dnf", None)?)
            .args(["-q", "--cacheonly", "repoquery", "--qf", "%{name}", "bash"])
            .output()
            .ok()?;
        let cache_vivant = String::from_utf8_lossy(&témoin.stdout).contains("bash");
        return cache_vivant.then_some(false);
    }
    Some(true)
}

/// Le programme, s'il est dans le dossier d'outils de namsdec.
///
/// Un niveau de répertoire par outil, un `bin` dedans : c'est ce que les
/// recettes construisent, et l'on n'explore pas plus loin.
fn installed_program(command: &str, tools: &Path) -> Option<PathBuf> {
    let candidat = |dossier: &Path| {
        let programme = dossier.join("bin").join(command);
        programme.is_file().then_some(programme)
    };
    std::fs::read_dir(tools)
        .ok()?
        .filter_map(Result::ok)
        .find_map(|entrée| candidat(&entrée.path()))
}

impl Known {
    /// Ce qu'on sait de cet outil sur cette machine.
    pub fn availability(&self) -> Availability {
        self.availability_in(None)
    }

    /// La fiche à enregistrer, avec le chemin du programme **résolu**.
    ///
    /// La fiche embarquée nomme son programme tout court — « rizin » —,
    /// ce qui est juste tant qu'on parle de celui du système. Mais
    /// namsdec sait en installer un dans son propre dossier, et celui-là
    /// n'est dans le PATH de personne.
    ///
    /// Sans cette résolution, s'en servir après l'avoir installé écrivait
    /// « rizin » : la machine en a peut-être un autre, plus ancien, sans
    /// le greffon — et l'on obtenait « la commande pdg n'existe pas »
    /// juste après une installation qui avait pourtant réussi. C'est
    /// exactement ce qui se passait sur une Fedora livrant rizin 0.7.4.
    pub fn resolved_plugin(&self, tools: Option<&Path>) -> Plugin {
        let mut plugin = self.plugin.clone();
        // Seulement celui que namsdec a posé : si le programme vient du
        // système, son nom nu reste la bonne réponse — il suivra les
        // mises à jour de la machine, et le figer en chemin absolu
        // casserait à la première.
        if let Some(tools) = tools
            && let Some(chemin) = installed_program(&plugin.command, tools)
        {
            plugin.command = chemin.display().to_string();
        }
        plugin
    }

    /// Comme [`Known::availability`], en regardant aussi ce que namsdec a
    /// installé lui-même sous `tools`.
    pub fn availability_in(&self, tools: Option<&Path>) -> Availability {
        let Some(chemin) = program_path_in(&self.plugin.command, tools) else {
            return Availability::Missing;
        };
        // Rizin sans rz-ghidra : le programme est là, `pdg` n'y est pas.
        // On le lui demande plutôt que de le supposer — et l'on ne lit
        // que sa réponse, sans lui donner de fichier à analyser.
        if self.plugin.id == "rizin-ghidra" {
            return match std::process::Command::new(chemin)
                .args(["-N", "-q", "-c", "pdg?", "--", "malloc://16"])
                .output()
            {
                Ok(sortie) => {
                    let texte = String::from_utf8_lossy(&sortie.stdout).to_lowercase();
                    let erreur = String::from_utf8_lossy(&sortie.stderr).to_lowercase();
                    if erreur.contains("cannot find") || erreur.contains("unknown command") {
                        Availability::Incomplete
                    } else if texte.contains("pdg") {
                        Availability::Ready
                    } else {
                        Availability::Incomplete
                    }
                }
                Err(_) => Availability::Incomplete,
            };
        }
        Availability::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque outil connu est déclaré correctement : sans quoi son
    /// installation en un clic poserait un fichier que namsdec refuse.
    #[test]
    fn every_known_tool_describes_itself_properly() {
        for outil in all() {
            let p = &outil.plugin;
            assert!(!p.id.is_empty() && !p.name.is_empty(), "{p:?}");
            // `{file}` ou `{text}` : le contrat en offre deux, et le
            // second est le bon pour un outil qui ne lit pas l'ELF —
            // namsdec lui découpe les octets exécutables et lui donne
            // leur adresse. Exiger `{file}` seul rendait la liste
            // fermée à ces outils-là.
            assert!(
                p.args
                    .iter()
                    .any(|a| a.contains("{file}") || a.contains("{text}")),
                "{} ne dit pas où vont les octets à désassembler",
                p.id
            );
            assert!(
                p.install.is_some(),
                "{} ne dit pas comment on l'obtient",
                p.id
            );
            assert!(!outil.about_fr.is_empty() && !outil.about_en.is_empty());
        }
    }

    /// RetDec écrit dans un fichier : sa description doit le dire, sinon
    /// namsdec lirait une sortie standard vide et croirait à un échec.
    #[test]
    fn retdec_is_declared_as_writing_to_a_file() {
        let retdec = all()
            .into_iter()
            .find(|outil| outil.plugin.id == "retdec")
            .expect("RetDec est connu");
        assert!(
            retdec.plugin.args.iter().any(|a| a.contains("{out}")),
            "{:?}",
            retdec.plugin.args
        );
    }

    /// Un programme absent est dit absent — et l'on ne cherche pas à
    /// deviner par un autre chemin.
    #[test]
    fn a_missing_program_is_reported_as_missing() {
        assert_eq!(program_path("ce-programme-nexiste-pas-du-tout"), None);
        // Et l'un de ceux que tout système possède est trouvé.
        assert!(program_path("sh").is_some());
    }

    /// Ce que namsdec a installé lui-même, il doit le retrouver.
    ///
    /// Le programme est posé sous `<tools>/<outil>/bin/`, qui n'est dans
    /// le PATH de personne. Tant que la recherche s'en tenait au PATH, une
    /// installation qu'une étape tardive avait fait échouer laissait sur
    /// le disque un outil complet et fonctionnel, que namsdec déclarait
    /// « absent » — et qu'il proposait de réinstaller.
    #[test]
    fn a_tool_namsdec_installed_itself_is_found() {
        let racine = tempfile::tempdir().expect("un dossier temporaire");
        let tools = racine.path();
        let bin = tools.join("retdec").join("bin");
        std::fs::create_dir_all(&bin).expect("l'arborescence");
        let programme = bin.join("retdec-decompiler");
        std::fs::write(&programme, b"").expect("le programme");

        assert_eq!(
            program_path_in("retdec-decompiler", Some(tools)),
            Some(programme),
            "il est là où namsdec l'a mis"
        );
        // Sans le dossier, on ne connaît toujours que le PATH.
        assert_eq!(program_path("retdec-decompiler"), None);
        // Et l'on ne remonte pas un programme qui n'y est pas.
        assert_eq!(program_path_in("outil-qui-nexiste-pas", Some(tools)), None);
    }
}

#[cfg(test)]
mod resolution_tests {
    use super::*;

    /// S'en servir après l'avoir installé enregistre **son** chemin.
    ///
    /// Le cas qui s'est vraiment produit : Fedora livre rizin 0.7.4, sans
    /// le greffon rz-ghidra. namsdec en installe un 0.9.1 dans son propre
    /// dossier, avec le greffon — puis, en cliquant « S'en servir »,
    /// écrivait « rizin » tout court dans la fiche. La commande partait
    /// alors chercher celui du PATH, c'est-à-dire l'autre, et répondait
    /// que `pdg` n'existe pas. Une installation réussie donnait donc un
    /// outil cassé.
    #[test]
    fn using_a_self_installed_tool_records_its_own_path() {
        let dossier = tempfile::tempdir().unwrap();
        let outils = dossier.path();
        let bin = outils.join("rizin").join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("rizin"), b"#!/bin/sh\n").unwrap();

        let rizin = all()
            .into_iter()
            .find(|k| k.plugin.id == "rizin-ghidra")
            .expect("la fiche rizin existe");

        // Sans dossier d'outils : le nom nu, qui suivra le système.
        assert_eq!(rizin.resolved_plugin(None).command, "rizin");

        // Avec : le chemin de celui que namsdec a posé.
        let résolu = rizin.resolved_plugin(Some(outils));
        assert_eq!(
            résolu.command,
            bin.join("rizin").display().to_string(),
            "c'est celui-là qu'il faut lancer, pas celui du PATH"
        );
        // Le reste de la fiche ne bouge pas : seul le programme change.
        assert_eq!(résolu.args, rizin.plugin.args);
        assert_eq!(résolu.id, rizin.plugin.id);
    }

    /// Un outil que namsdec n'a pas installé garde son nom nu.
    ///
    /// Le figer en chemin absolu casserait à la première mise à jour de
    /// la machine : `/usr/bin/objdump` déplacé, et la fiche ne vaut plus
    /// rien. Le nom nu, lui, suit.
    #[test]
    fn a_system_tool_keeps_its_bare_name() {
        let dossier = tempfile::tempdir().unwrap();
        for outil in all() {
            let résolu = outil.resolved_plugin(Some(dossier.path()));
            assert_eq!(
                résolu.command, outil.plugin.command,
                "{} : rien d'installé, rien à résoudre",
                outil.plugin.id
            );
        }
    }
}

#[cfg(test)]
mod package_tests {
    use super::*;

    /// Les noms de paquets se lisent dans la commande annoncée.
    #[test]
    fn the_packages_are_read_from_the_command() {
        assert_eq!(packages_named("sudo dnf install rizin"), vec!["rizin"]);
        assert_eq!(
            packages_named("sudo dnf install -y rizin rizin-devel"),
            vec!["rizin", "rizin-devel"],
            "les options ne sont pas des paquets"
        );
        assert_eq!(packages_named("sudo pacman -S nasm"), vec!["nasm"]);
        // Une page de téléchargement ne nomme aucun paquet : il n'y a
        // rien à vérifier, et prétendre le contraire crierait au loup.
        assert!(packages_named("github.com/avast/retdec/releases").is_empty());
        assert!(packages_named("").is_empty());
        // Une ligne de compilation n'annonce aucun paquet, et son `-S`
        // n'est pas celui de pacman. Sans cette distinction, namsdec
        // allait demander aux dépôts s'ils connaissent un paquet
        // nommé « . ».
        assert!(
            packages_named("cd DZF/x64-disassembler && cmake -S . -B build && cmake --build build")
                .is_empty(),
            "cmake -S n'est pas pacman -S"
        );
    }

    /// Chaque paquet annoncé existe vraiment dans les dépôts.
    ///
    /// C'est le contrôle qui manquait. Une recette annonçait
    /// `rizin-plugin-ghidra`, qui n'existe dans aucun dépôt Fedora — et
    /// comme dnf refuse la transaction entière, la commande n'installait
    /// même pas Rizin. Personne ne pouvait le savoir avant de la taper.
    ///
    /// Le test se tait quand il ne peut pas conclure : pas de dnf, pas de
    /// cache, une erreur. `None` n'est pas un échec — c'est une absence
    /// de réponse, et l'on n'en tire rien.
    #[test]
    fn every_announced_package_exists() {
        let mut fautifs: Vec<String> = Vec::new();
        let mut vérifiés = 0usize;
        for outil in all() {
            let Some(install) = &outil.plugin.install else {
                continue;
            };
            for paquet in packages_named(install) {
                match package_known(paquet) {
                    Some(true) => vérifiés += 1,
                    Some(false) => {
                        fautifs.push(format!("{} annonce « {paquet} »", outil.plugin.id))
                    }
                    None => {}
                }
            }
        }
        assert!(
            fautifs.is_empty(),
            "{} paquet(s) annoncé(s) qui n'existent pas ({vérifiés} vérifiés) :\n{}",
            fautifs.len(),
            fautifs.join("\n")
        );
    }
}
