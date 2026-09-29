//! Les désassembleurs venus d'ailleurs.
//!
//! namsdec en livre deux — le pseudo-code et la syntaxe NASM —, et l'on
//! peut en ajouter d'autres sans toucher au programme : `objdump`,
//! `ndisasm`, `radare2`, ou celui qu'on écrit soi-même. Un fichier JSON
//! décrit l'outil, namsdec lui passe le binaire et montre ce qu'il
//! répond.
//!
//! **Ce que cela suppose est dit franchement** : contrairement à un
//! thème, qui n'est qu'une palette, un désassembleur de ce genre *est un
//! programme qui s'exécute*. Installer un tel plugin revient à lancer la
//! commande qu'il nomme. namsdec ne va donc pas les chercher tout seul,
//! ne les exécute que sur demande explicite, et les borne : un temps
//! maximum, une sortie maximum, et le binaire passé par un fichier
//! temporaire plutôt que par un interpréteur de commandes.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `Display` en anglais, comme les autres erreurs des crates métier.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("the tool took longer than {0} seconds and was stopped")]
    TooLong(u64),
    #[error("cannot read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("this is not a valid disassembler file: {0}")]
    Format(#[from] serde_json::Error),
    #[error("the identifier must be made of letters, digits or dashes: {0}")]
    BadId(String),
    #[error("the command is empty")]
    NoCommand,
    #[error("the command does not say where the file goes: add {{file}} to its arguments")]
    NoFilePlaceholder,
}

/// Un désassembleur décrit par un fichier.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Plugin {
    /// Identifiant stable : lettres, chiffres et tirets.
    pub id: String,
    /// Le nom montré dans le choix du moteur.
    pub name: String,
    /// Le programme à lancer.
    pub command: String,
    /// Ses arguments. L'un d'eux doit contenir `{file}`, remplacé par le
    /// chemin du binaire — sans quoi l'outil ne saurait pas quoi lire.
    ///
    /// Un argument peut aussi contenir `{out}` : namsdec y met alors le
    /// chemin d'un fichier de sortie, et lit ce fichier une fois l'outil
    /// terminé. C'est nécessaire pour ceux qui écrivent leur résultat
    /// dans un fichier plutôt que sur la sortie standard — RetDec, par
    /// exemple, à qui l'on passe `-o {out}`.
    pub args: Vec<String>,
    /// Comment installer l'outil, quand il n'est pas là.
    ///
    /// namsdec ne l'installe pas lui-même : télécharger et exécuter du
    /// code au nom de quelqu'un n'est pas une commodité, c'est une
    /// décision. Il dit ce qu'il faut taper, et laisse taper.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install: Option<String>,
}

/// Ce qu'on tolère d'un outil extérieur : il analyse un binaire, il ne
/// tourne pas en fond.
const TIMEOUT: Duration = Duration::from_secs(10);
/// Au-delà, ce n'est plus un listing qu'on lit, c'est un vidage.
const MAX_OUTPUT: usize = 4 * 1024 * 1024;

/// Lit un fichier de description.
pub fn read(path: &Path) -> Result<Plugin, PluginError> {
    let plugin: Plugin = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    if plugin.id.is_empty()
        || !plugin
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(PluginError::BadId(plugin.id));
    }
    if plugin.command.trim().is_empty() {
        return Err(PluginError::NoCommand);
    }
    if !plugin.args.iter().any(|arg| arg.contains("{file}")) {
        return Err(PluginError::NoFilePlaceholder);
    }
    Ok(plugin)
}

/// Tous les désassembleurs décrits dans ce dossier, triés par nom.
///
/// Un fichier illisible est ignoré plutôt que fatal : un plugin abîmé ne
/// doit pas empêcher les autres de servir.
pub fn read_dir(dir: &Path) -> Vec<Plugin> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<Plugin> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| read(&entry.path()).ok())
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found.dedup_by(|a, b| a.id == b.id);
    found
}

impl Plugin {
    /// Lance l'outil sur ces octets et rend ce qu'il a écrit.
    ///
    /// Le binaire part par un fichier temporaire, et les arguments sont
    /// passés tels quels au programme : aucun interpréteur de commandes
    /// n'est en jeu, donc rien de ce que contient un nom de fichier ne
    /// peut être relu comme une commande.
    pub fn run(&self, bytes: &[u8]) -> Result<String, PluginError> {
        self.run_within(bytes, TIMEOUT)
    }

    /// La même chose, avec un délai qu'on choisit — et qui est **tenu**.
    ///
    /// Un désassembleur répond en un souffle ; un décompilateur peut
    /// réfléchir une demi-minute, et parfois ne jamais rendre la main.
    /// On le surveille donc vraiment : passé le délai, le processus est
    /// tué, et l'on dit qu'il a été tué plutôt que d'attendre pour
    /// toujours.
    pub fn run_within(&self, bytes: &[u8], délai: Duration) -> Result<String, PluginError> {
        let dir = tempfile::tempdir()?;
        let file: PathBuf = dir.path().join("program");
        std::fs::write(&file, bytes)?;
        // Le fichier où l'outil écrira, s'il en veut un. Dans le même
        // dossier temporaire : il disparaît avec lui.
        let sortie: PathBuf = dir.path().join("sortie");
        let écrit_un_fichier = self.args.iter().any(|arg| arg.contains("{out}"));

        let args: Vec<String> = self
            .args
            .iter()
            .map(|arg| {
                arg.replace("{file}", &file.to_string_lossy())
                    .replace("{out}", &sortie.to_string_lossy())
            })
            .collect();
        let mut enfant = Command::new(&self.command)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // On surveille à petits pas plutôt que d'attendre : c'est la
        // seule façon, sans bibliothèque supplémentaire, de reprendre la
        // main sur un outil qui ne la rend pas.
        let début = std::time::Instant::now();
        loop {
            match enfant.try_wait()? {
                Some(_) => break,
                None if début.elapsed() >= délai => {
                    let _ = enfant.kill();
                    let _ = enfant.wait();
                    return Err(PluginError::TooLong(délai.as_secs()));
                }
                None => std::thread::sleep(Duration::from_millis(50)),
            }
        }
        let output = enfant.wait_with_output()?;
        if écrit_un_fichier {
            // Ce que l'outil a écrit dans son fichier fait foi. S'il n'a
            // rien écrit — il a refusé, il s'est plaint —, la sortie
            // d'erreur dit pourquoi, et c'est elle qu'on montre.
            match std::fs::read(&sortie) {
                Ok(contenu) if !contenu.is_empty() => {
                    return Ok(borner(&contenu, &output.stderr));
                }
                _ => return Ok(borner(&output.stdout, &output.stderr)),
            }
        }
        Ok(borner(&output.stdout, &output.stderr))
    }
}

/// Ce qu'on garde de la sortie d'un outil.
///
/// La sortie d'erreur remplace la sortie ordinaire quand celle-ci est
/// vide : beaucoup d'outils y écrivent leur refus, et un panneau vide
/// n'apprendrait rien.
fn borner(stdout: &[u8], stderr: &[u8]) -> String {
    let mut texte = String::from_utf8_lossy(stdout).into_owned();
    if texte.len() > MAX_OUTPUT {
        texte.truncate(MAX_OUTPUT);
        texte.push_str("\n…");
    }
    if texte.trim().is_empty() {
        texte = String::from_utf8_lossy(stderr).into_owned();
    }
    texte
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, contenu: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contenu).unwrap();
        path
    }

    #[test]
    fn a_plugin_describes_the_tool_to_run() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            dir.path(),
            "objdump.json",
            r#"{"id":"objdump","name":"objdump",
                "command":"objdump","args":["-d","{file}"]}"#,
        );
        let plugin = read(&path).expect("un plugin valide");
        assert_eq!(plugin.id, "objdump");
        assert_eq!(plugin.args, ["-d", "{file}"]);
    }

    /// Ce qui ne peut pas marcher est refusé à la lecture, pas découvert
    /// au premier clic.
    #[test]
    fn what_cannot_work_is_refused_up_front() {
        let dir = tempfile::tempdir().unwrap();
        let sans_fichier = write(
            dir.path(),
            "a.json",
            r#"{"id":"a","name":"a","command":"objdump","args":["-d"]}"#,
        );
        assert!(matches!(
            read(&sans_fichier),
            Err(PluginError::NoFilePlaceholder)
        ));

        let sans_commande = write(
            dir.path(),
            "b.json",
            r#"{"id":"b","name":"b","command":"","args":["{file}"]}"#,
        );
        assert!(matches!(read(&sans_commande), Err(PluginError::NoCommand)));

        let mauvais_id = write(
            dir.path(),
            "c.json",
            r#"{"id":"c d","name":"c","command":"x","args":["{file}"]}"#,
        );
        assert!(matches!(read(&mauvais_id), Err(PluginError::BadId(_))));
    }

    /// Un fichier abîmé est ignoré, les autres servent quand même.
    #[test]
    fn a_broken_file_does_not_take_the_others_down() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "cassé.json", "{ ceci n'est pas du JSON");
        write(
            dir.path(),
            "bon.json",
            r#"{"id":"bon","name":"bon","command":"objdump","args":["{file}"]}"#,
        );
        let found = read_dir(dir.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "bon");
    }

    /// Et l'outil est réellement lancé sur les octets qu'on lui donne.
    #[test]
    fn the_tool_receives_the_binary() {
        let dir = tempfile::tempdir().unwrap();
        // `cat` lit le fichier et le réécrit : de quoi vérifier le tour
        // complet sans dépendre d'un désassembleur particulier.
        let path = write(
            dir.path(),
            "cat.json",
            r#"{"id":"cat","name":"cat","command":"cat","args":["{file}"]}"#,
        );
        let plugin = read(&path).unwrap();
        let sortie = plugin.run(b"salut").expect("cat existe partout");
        assert_eq!(sortie, "salut");
    }
}

#[cfg(test)]
mod dzf_tests {
    use super::*;

    /// La fiche de DZF est valide, et namsdec sait la lire.
    ///
    /// DZF est un désassembleur x86-64 écrit à la main, sans dépendance :
    /// c'est le genre d'outil pour lequel ce format existe. Le test lit
    /// la fiche livrée et vérifie qu'elle passe les contrôles — un
    /// identifiant propre, une commande, et `{file}` quelque part, sans
    /// quoi l'outil ne saurait pas quoi lire.
    #[test]
    fn the_dzf_plugin_file_is_valid() {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/fonctionnalites/dzf.json");
        let plugin = read(&chemin).expect("la fiche de DZF doit être valide");
        assert_eq!(plugin.id, "dzf");
        assert!(
            plugin.args.iter().any(|a| a.contains("{file}")),
            "namsdec doit pouvoir dire où est le binaire : {:?}",
            plugin.args
        );
        // Et elle dit comment l'obtenir : un outil qu'on n'a pas et dont
        // on ne sait pas quoi faire n'est pas une proposition.
        assert!(plugin.install.is_some());
    }

    /// Et s'il est bâti sur cette machine, il répond vraiment.
    ///
    /// Le test ne suppose pas qu'il est là : DZF vit dans un autre dépôt,
    /// et la suite doit passer sur une machine qui ne l'a pas. Quand il
    /// est là, en revanche, on va jusqu'au bout — c'est la seule façon de
    /// savoir que le branchement tient.
    #[test]
    fn dzf_disassembles_real_bytes_when_it_is_built() {
        // DZF vit dans son propre dépôt : `NAMSDEC_DZF` dit où trouver
        // son binaire, et à défaut on regarde à côté de celui-ci. Sans
        // lui, le test se tait — il n'a rien à juger.
        let binaire = match std::env::var_os("NAMSDEC_DZF") {
            Some(ailleurs) => std::path::PathBuf::from(ailleurs),
            None => std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../DZF/x64-disassembler/build/x64-disassembler"),
        };
        if !binaire.is_file() {
            return;
        }
        let plugin = Plugin {
            id: "dzf".to_owned(),
            name: "DZF".to_owned(),
            command: binaire.display().to_string(),
            args: vec![
                "--base".to_owned(),
                "0x401000".to_owned(),
                "{file}".to_owned(),
            ],
            install: None,
        };
        // `push rbp ; mov rbp, rsp ; mov rax, 42 ; pop rbp ; ret` — le
        // début de n'importe quelle fonction, et de quoi vérifier que les
        // adresses avancent du bon nombre d'octets.
        let octets: &[u8] = &[
            0x55, 0x48, 0x89, 0xe5, 0x48, 0xb8, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x5d, 0xc3,
        ];
        let listing = plugin.run(octets).expect("DZF doit répondre");
        for attendu in [
            "push rbp",
            "mov rbp, rsp",
            "mov rax, 0x2a",
            "pop rbp",
            "ret",
        ] {
            assert!(
                listing.contains(attendu),
                "« {attendu} » manque dans ce que DZF rend :\n{listing}"
            );
        }
        // Les adresses suivent la base demandée, et avancent de la taille
        // de chaque instruction.
        assert!(listing.contains("401000"), "{listing}");
        assert!(
            listing.contains("40100f"),
            "le `ret` est au bon endroit :\n{listing}"
        );
    }
}
