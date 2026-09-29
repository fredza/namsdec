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
    #[error(
        "the command does not say where the file goes: add {{file}} or {{text}} to its arguments"
    )]
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
    ///
    /// Deux autres marqueurs servent les outils qui **ne lisent pas
    /// l'ELF** — et il y en a, à commencer par celui qu'on écrit
    /// soi-même le premier jour :
    ///
    /// - `{text}` reçoit les octets EXÉCUTABLES seuls, sans en-tête ni
    ///   données. Sans lui, un tel outil désassemble l'en-tête ELF comme
    ///   du code, et les vingt premières lignes de son listing ne
    ///   veulent rien dire ;
    /// - `{base}` reçoit l'adresse à laquelle ces octets sont chargés,
    ///   en hexadécimal — sans quoi tout le listing serait adressé à
    ///   partir de zéro, et aucun saut ne tomberait juste.
    ///
    /// C'est namsdec qui sait lire l'ELF ; il n'y a pas de raison de
    /// demander à chaque outil de réapprendre à le faire.
    pub args: Vec<String>,
    /// Comment installer l'outil, quand il n'est pas là.
    ///
    /// namsdec ne l'installe pas lui-même : télécharger et exécuter du
    /// code au nom de quelqu'un n'est pas une commodité, c'est une
    /// décision. Il dit ce qu'il faut taper, et laisse taper.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install: Option<String>,
}

/// Ce que namsdec sait du binaire, et qu'il peut passer à l'outil.
///
/// Séparé des octets bruts parce que tous les outils n'en veulent pas
/// autant : celui qui lit l'ELF se passe de tout le reste, celui qui
/// décode des octets a besoin qu'on lui dise lesquels et à quelle
/// adresse. Une seule structure, et la fiche choisit.
#[derive(Debug, Clone, Copy)]
pub struct Cible<'a> {
    /// Le binaire entier, tel qu'il est sur le disque.
    pub fichier: &'a [u8],
    /// Les octets exécutables seuls, quand on sait les isoler.
    pub code: Option<&'a [u8]>,
    /// L'adresse à laquelle ces octets sont chargés.
    pub base: u64,
}

impl<'a> Cible<'a> {
    /// Des octets dont on ne sait rien d'autre.
    pub fn brute(bytes: &'a [u8]) -> Self {
        Self {
            fichier: bytes,
            code: None,
            base: 0,
        }
    }
}

/// Ce qu'on tolère d'un outil extérieur : il analyse un binaire, il ne
/// tourne pas en fond.
///
/// Exposé parce qu'un appelant qui passe une [`Cible`] emploie `run_on`
/// et doit pouvoir dire le même délai que `run` — sans le recopier.
pub const TIMEOUT: Duration = Duration::from_secs(10);
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
    // `{file}` ou `{text}` : le binaire entier, ou son code seul. L'un
    // des deux, sans quoi l'outil ne saurait pas quoi lire.
    if !plugin
        .args
        .iter()
        .any(|arg| arg.contains("{file}") || arg.contains("{text}"))
    {
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
        self.run_on(&Cible::brute(bytes), délai)
    }

    /// La même chose, en disant à l'outil ce que namsdec sait du binaire.
    ///
    /// Un outil qui lit l'ELF n'a besoin que de `{file}` et ignore le
    /// reste. Un outil qui ne le lit pas — et c'est le cas du premier
    /// désassembleur que quiconque écrit — reçoit avec `{text}` les
    /// octets exécutables seuls, et avec `{base}` l'adresse où ils
    /// vivent. Ni l'un ni l'autre n'a à changer pour cela : c'est la
    /// fiche qui dit ce qu'il veut recevoir.
    pub fn run_on(&self, cible: &Cible<'_>, délai: Duration) -> Result<String, PluginError> {
        let dir = tempfile::tempdir()?;
        let file: PathBuf = dir.path().join("program");
        std::fs::write(&file, cible.fichier)?;
        // Le code seul, quand la fiche le demande. Faute de savoir le
        // découper, on lui donne le binaire entier : c'est ce qu'il
        // recevait avant, donc au pire rien ne change pour lui.
        let text: PathBuf = dir.path().join("text");
        if self.args.iter().any(|arg| arg.contains("{text}")) {
            std::fs::write(&text, cible.code.unwrap_or(cible.fichier))?;
        }
        // Le fichier où l'outil écrira, s'il en veut un. Dans le même
        // dossier temporaire : il disparaît avec lui.
        let sortie: PathBuf = dir.path().join("sortie");
        let écrit_un_fichier = self.args.iter().any(|arg| arg.contains("{out}"));

        let args: Vec<String> = self
            .args
            .iter()
            .map(|arg| {
                arg.replace("{file}", &file.to_string_lossy())
                    .replace("{text}", &text.to_string_lossy())
                    .replace("{base}", &format!("{:#x}", cible.base))
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

    /// `{text}` suffit : un outil qui ne veut que du code est accepté.
    ///
    /// La règle disait « il faut `{file}` ». Elle écartait exactement les
    /// outils pour lesquels `{text}` a été ajouté — ceux qui ne lisent
    /// pas l'ELF et n'ont que faire du fichier entier.
    #[test]
    fn asking_only_for_the_code_is_enough() {
        let dir = tempfile::tempdir().unwrap();
        let fiche = write(
            dir.path(),
            "t.json",
            r#"{"id":"t","name":"t","command":"x","args":["--base","{base}","{text}"]}"#,
        );
        assert!(read(&fiche).is_ok(), "{:?}", read(&fiche));
    }

    /// L'outil reçoit le CODE, et non le fichier entier.
    ///
    /// C'est tout l'objet de `{text}` : un désassembleur qui décode des
    /// octets bruts commençait par décoder l'en-tête ELF, et ses vingt
    /// premières lignes ne voulaient rien dire. On vérifie donc ce qui
    /// arrive vraiment sur le disque, et non ce que la substitution
    /// prétend écrire.
    #[test]
    fn the_tool_receives_the_code_and_its_address() {
        let dir = tempfile::tempdir().unwrap();
        let fiche = write(
            dir.path(),
            "cat.json",
            r#"{"id":"cat","name":"cat","command":"sh","args":["-c","echo base={base}; cat \"$0\"","{text}"]}"#,
        );
        let plugin = read(&fiche).expect("la fiche est valide");
        let cible = Cible {
            fichier: b"\x7fELF--en-tete--CODE",
            code: Some(b"CODE"),
            base: 0x401000,
        };
        let réponse = plugin
            .run_on(&cible, Duration::from_secs(5))
            .expect("l'outil répond");
        assert!(réponse.contains("base=0x401000"), "{réponse}");
        assert!(réponse.contains("CODE"), "{réponse}");
        assert!(
            !réponse.contains("en-tete"),
            "l'en-tête ne doit pas partir avec le code : {réponse}"
        );
    }

    /// Faute de savoir découper, l'outil reçoit ce qu'il recevait avant.
    ///
    /// Un plugin ne doit pas cesser de marcher parce que namsdec n'a pas
    /// su isoler le code — un objet sans section exécutable, un format
    /// qu'il ne connaît pas. Au pire, rien ne change pour lui.
    #[test]
    fn without_a_code_section_the_whole_file_is_passed() {
        let dir = tempfile::tempdir().unwrap();
        let fiche = write(
            dir.path(),
            "cat2.json",
            r#"{"id":"cat2","name":"cat2","command":"cat","args":["{text}"]}"#,
        );
        let plugin = read(&fiche).expect("la fiche est valide");
        let réponse = plugin
            .run_on(&Cible::brute(b"tout le fichier"), Duration::from_secs(5))
            .expect("l'outil répond");
        assert!(réponse.contains("tout le fichier"), "{réponse}");
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
    /// identifiant propre, une commande, et de quoi savoir quoi lire.
    ///
    /// DZF ne lit pas l'en-tête ELF, et n'a pas à l'apprendre : sa fiche
    /// demande `{text}` — les octets exécutables seuls — et `{base}` —
    /// l'adresse où ils vivent. Sans eux, il désassemblait l'en-tête
    /// comme du code et adressait tout son listing à partir de zéro.
    #[test]
    fn the_dzf_plugin_file_is_valid() {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/fonctionnalites/dzf.json");
        let plugin = read(&chemin).expect("la fiche de DZF doit être valide");
        assert_eq!(plugin.id, "dzf");
        assert!(
            plugin.args.iter().any(|a| a.contains("{text}")),
            "DZF reçoit le code seul, non le fichier entier : {:?}",
            plugin.args
        );
        assert!(
            plugin.args.iter().any(|a| a.contains("{base}")),
            "et l'adresse où ce code est chargé : {:?}",
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
