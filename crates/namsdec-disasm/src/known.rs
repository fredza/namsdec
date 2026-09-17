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

use std::path::PathBuf;

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
                install: Some("sudo dnf install rizin rizin-plugin-ghidra".to_owned()),
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
    ]
}

/// Le programme existe-t-il dans le PATH ?
///
/// On cherche à la main plutôt que d'appeler `which` : lancer un
/// programme pour savoir si l'on peut en lancer un autre serait un
/// détour, et un `which` absent ferait croire que rien n'est installé.
pub fn program_path(command: &str) -> Option<PathBuf> {
    let chemin = std::path::Path::new(command);
    if chemin.is_absolute() {
        return chemin.is_file().then(|| chemin.to_path_buf());
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidat = dir.join(command);
        candidat.is_file().then_some(candidat)
    })
}

impl Known {
    /// Ce qu'on sait de cet outil sur cette machine.
    pub fn availability(&self) -> Availability {
        let Some(chemin) = program_path(&self.plugin.command) else {
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
            assert!(
                p.args.iter().any(|a| a.contains("{file}")),
                "{} ne dit pas où va le binaire",
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
}
