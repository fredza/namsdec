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

    /// La même chose, avec un délai qu'on choisit — et qui est **tenu**,
    /// jusque dans la descendance de l'outil.
    ///
    /// Elle ne rend que le TEXTE : pour qui a besoin de savoir si
    /// l'outil a réussi, c'est [`Plugin::run_on`] et son
    /// [`ToolOutput`].
    ///
    /// Un désassembleur répond en un souffle ; un décompilateur peut
    /// réfléchir une demi-minute, et parfois ne jamais rendre la main.
    /// On le surveille donc vraiment : passé le délai, le processus est
    /// tué, et l'on dit qu'il a été tué plutôt que d'attendre pour
    /// toujours.
    pub fn run_within(&self, bytes: &[u8], délai: Duration) -> Result<String, PluginError> {
        self.run_on(&Cible::brute(bytes), délai)
            .map(|rendu| rendu.text)
    }

    /// La même chose, en disant à l'outil ce que namsdec sait du binaire.
    ///
    /// Un outil qui lit l'ELF n'a besoin que de `{file}` et ignore le
    /// reste. Un outil qui ne le lit pas — et c'est le cas du premier
    /// désassembleur que quiconque écrit — reçoit avec `{text}` les
    /// octets exécutables seuls, et avec `{base}` l'adresse où ils
    /// vivent. Ni l'un ni l'autre n'a à changer pour cela : c'est la
    /// fiche qui dit ce qu'il veut recevoir.
    pub fn run_on(&self, cible: &Cible<'_>, délai: Duration) -> Result<ToolOutput, PluginError> {
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
        // Le groupe de processus, et c'est la correction du délai.
        //
        // On tuait l'enfant DIRECT, puis on attendait les fils qui
        // lisent ses sorties. Un descendant qui garde les tubes ouverts
        // — un `sleep` lancé en héritant des sorties, ce que font
        // couramment les scripts d'installation — laissait donc ces
        // lecteurs attendre, et leur `join` n'était soumis à aucun
        // délai : un délai demandé de 100 ms rendait la main au bout de
        // deux secondes. Mis dans son propre groupe, l'outil emmène sa
        // descendance quand on l'arrête.
        let mut commande = Command::new(&self.command);
        commande
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            commande.process_group(0);
        }
        let mut enfant = commande.spawn()?;

        // ON LIT PENDANT QUE L'OUTIL ÉCRIT, et c'est indispensable.
        //
        // Un tube a un tampon d'une soixantaine de kilo-octets. Plein,
        // l'écriture suivante BLOQUE l'outil — qui ne finit donc jamais,
        // pendant que nous l'attendons sagement. On tenait là un
        // interblocage parfait : `objdump -d /bin/ls` rend plusieurs
        // méga-octets, et namsdec le déclarait « arrêté au bout de cent
        // vingt secondes » alors qu'objdump avait fait son travail en
        // une fraction de seconde et attendait qu'on le lise.
        //
        // Deux fils, un par tube. Ils gardent au plus [`MAX_OUTPUT`]
        // octets et **continuent de vider** le reste : la borne est
        // maintenant une borne de MÉMOIRE et pas seulement d'affichage.
        // `read_to_end` lisait tout avant de tronquer, ce qui revenait
        // à promettre quatre méga-octets en en consommant autant que
        // l'outil voulait bien en écrire.
        let sortie_std = enfant.stdout.take().map(Lecteur::lancer);
        let sortie_err = enfant.stderr.take().map(Lecteur::lancer);

        // On surveille à petits pas plutôt que d'attendre : c'est la
        // seule façon, sans bibliothèque supplémentaire, de reprendre la
        // main sur un outil qui ne la rend pas.
        let début = std::time::Instant::now();
        let mut trop_long = false;
        let mut statut = None;
        loop {
            match enfant.try_wait()? {
                // Le statut est GARDÉ. Jeté, il laissait un outil qui
                // écrit « tool failed » et sort avec le code 7 rendre un
                // `Ok` indiscernable d'un succès.
                Some(fini) => {
                    statut = Some(fini);
                    break;
                }
                None if début.elapsed() >= délai => {
                    tuer_le_groupe(&mut enfant);
                    let _ = enfant.wait();
                    trop_long = true;
                    break;
                }
                None => std::thread::sleep(Duration::from_millis(20)),
            }
        }
        // Les fils rejoints DANS TOUS LES CAS, y compris après un
        // arrêt — mais avec une échéance, et c'est la seconde moitié de
        // la correction : un descendant qui aurait échappé au groupe
        // (un `setsid`, par exemple) tiendrait sinon le tube ouvert
        // pour toujours. Passé le délai de grâce, on prend ce qui a été
        // lu jusque-là et l'on s'en va ; le tampon est partagé, donc
        // rien n'est perdu de ce qui était arrivé.
        let std_out = sortie_std
            .map(|lecteur| lecteur.récolter(GRÂCE))
            .unwrap_or_default();
        let std_err = sortie_err
            .map(|lecteur| lecteur.récolter(GRÂCE))
            .unwrap_or_default();
        if trop_long {
            return Err(PluginError::TooLong(délai.as_secs()));
        }
        let code = statut.and_then(|fini| fini.code());
        if écrit_un_fichier {
            // Ce que l'outil a écrit dans son fichier fait foi. S'il n'a
            // rien écrit — il a refusé, il s'est plaint —, la sortie
            // d'erreur dit pourquoi, et c'est elle qu'on montre.
            //
            // Lu BORNÉ comme les tubes : un fichier de sortie d'un
            // gigaoctet était lu en entier avant d'être tronqué pour
            // l'affichage.
            if let Ok(fichier) = Récolte::lire_fichier(&sortie)
                && !fichier.octets.is_empty()
            {
                return Ok(ToolOutput::choisir(code, fichier, std_err, Source::File));
            }
        }
        Ok(ToolOutput::choisir(code, std_out, std_err, Source::Stdout))
    }
}

/// Le délai de grâce accordé aux fils de lecture après l'arrêt de
/// l'outil : le temps que les tubes se ferment, et pas davantage.
const GRÂCE: Duration = Duration::from_millis(500);

/// Arrête l'outil **et sa descendance**.
///
/// Sous Unix, l'enfant a été placé dans son propre groupe : un signal
/// envoyé au groupe (un PID négatif) atteint tout ce qu'il a lancé.
/// Ailleurs, on fait ce qu'on peut — l'enfant direct.
fn tuer_le_groupe(enfant: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let pid = enfant.id() as i32;
        // SAFETY: `kill(2)` sur un groupe dont nous sommes le parent.
        // Le processus n'a pas encore été attendu, donc son PID est
        // toujours le sien et ne peut pas avoir été réattribué.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    let _ = enfant.kill();
}

/// Ce qu'un fil de lecture a récolté sur un tube ou dans un fichier.
#[derive(Debug, Default, Clone)]
struct Récolte {
    /// Les octets gardés : au plus [`MAX_OUTPUT`].
    octets: Vec<u8>,
    /// Vrai si l'outil en a écrit davantage.
    tronqué: bool,
}

impl Récolte {
    /// Le contenu d'un fichier, borné de la même façon qu'un tube.
    fn lire_fichier(path: &Path) -> std::io::Result<Self> {
        use std::io::Read;
        let mut octets = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_OUTPUT as u64 + 1)
            .read_to_end(&mut octets)?;
        let tronqué = octets.len() > MAX_OUTPUT;
        octets.truncate(MAX_OUTPUT);
        Ok(Self { octets, tronqué })
    }

    /// Le texte, coupé sur une frontière UTF-8 valide.
    ///
    /// C'est ici que tenait la panique : `String::truncate(4 Mio)` sur
    /// un texte dont le quatre-millionième octet tombait au milieu d'un
    /// « é » interrompait le fil, et le résultat du travail était perdu.
    /// On coupe les OCTETS, et seulement là où une séquence se termine.
    fn texte(&self) -> String {
        let mut texte = String::from_utf8_lossy(frontière(&self.octets)).into_owned();
        if self.tronqué {
            texte.push_str("\n… (sortie coupée à 4 Mio)");
        }
        texte
    }
}

/// Les octets jusqu'à la dernière séquence UTF-8 complète.
///
/// Une séquence tronquée en fin de tampon est écartée ; une vraie
/// erreur au milieu est laissée à `from_utf8_lossy`, qui la remplace
/// sans rien interrompre.
fn frontière(octets: &[u8]) -> &[u8] {
    match std::str::from_utf8(octets) {
        Ok(_) => octets,
        Err(e) if e.error_len().is_none() => &octets[..e.valid_up_to()],
        Err(_) => octets,
    }
}

/// Un fil qui vide un tube dans un tampon partagé.
///
/// Partagé, et c'est la raison d'être du type : le fil peut être
/// abandonné sans que ce qu'il avait déjà lu disparaisse avec lui.
struct Lecteur {
    tampon: std::sync::Arc<std::sync::Mutex<Récolte>>,
    fini: std::sync::mpsc::Receiver<()>,
}

impl Lecteur {
    fn lancer(mut flux: impl std::io::Read + Send + 'static) -> Self {
        let tampon = std::sync::Arc::new(std::sync::Mutex::new(Récolte::default()));
        let (dit_fini, fini) = std::sync::mpsc::channel();
        let à_remplir = std::sync::Arc::clone(&tampon);
        std::thread::spawn(move || {
            let mut bloc = [0u8; 64 * 1024];
            loop {
                let lu = match flux.read(&mut bloc) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                let Ok(mut récolte) = à_remplir.lock() else {
                    break;
                };
                let place = MAX_OUTPUT.saturating_sub(récolte.octets.len());
                let prend = lu.min(place);
                récolte.octets.extend_from_slice(&bloc[..prend]);
                if prend < lu {
                    // Au-delà de la borne on CONTINUE DE LIRE sans
                    // garder : s'arrêter remplirait le tube, et l'outil
                    // resterait bloqué sur son écriture suivante.
                    récolte.tronqué = true;
                }
            }
            let _ = dit_fini.send(());
        });
        Self { tampon, fini }
    }

    /// Ce que le fil a lu, en lui laissant au plus `grâce` pour finir.
    fn récolter(self, grâce: Duration) -> Récolte {
        let _ = self.fini.recv_timeout(grâce);
        self.tampon
            .lock()
            .map(|récolte| récolte.clone())
            .unwrap_or_default()
    }
}

/// Ce qu'un outil extérieur a rendu — et dans quel état il l'a rendu.
///
/// Une chaîne seule ne suffisait pas : un programme qui écrit
/// « tool failed » sur sa sortie d'erreur puis sort avec le code 7
/// donnait `Ok("tool failed\n")`, que rien ne distinguait d'un
/// résultat normal. Le texte reste ce qu'on affiche ; le statut dit ce
/// qu'on en pense, et la provenance d'où il vient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    /// Ce qu'on montre.
    pub text: String,
    /// Le code de sortie, ou `None` si l'outil a été arrêté par un
    /// signal plutôt que terminé de lui-même.
    pub status: Option<i32>,
    /// Vrai si l'outil en a écrit davantage que ce qui est gardé.
    pub truncated: bool,
    /// D'où vient `text`.
    pub source: Source,
}

/// D'où vient le texte qu'un outil a rendu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Sa sortie ordinaire.
    Stdout,
    /// Sa sortie d'erreur — ce qui est le cas ordinaire d'un refus.
    Stderr,
    /// Le fichier qu'on lui avait désigné par `{out}`.
    File,
}

impl std::fmt::Display for ToolOutput {
    /// Le texte, et rien d'autre : ce qu'un panneau peint.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl ToolOutput {
    /// L'outil a-t-il réussi ? `None` veut dire qu'on ne sait pas.
    pub fn succeeded(&self) -> bool {
        self.status == Some(0)
    }

    /// La sortie d'erreur remplace la sortie ordinaire quand celle-ci
    /// est vide : beaucoup d'outils y écrivent leur refus, et un
    /// panneau vide n'apprendrait rien.
    ///
    /// Elle est bornée COMME L'AUTRE, ce qui n'était pas le cas : le
    /// repli se faisait après la troncature, et cinq méga-octets de
    /// stderr revenaient en entier malgré le plafond annoncé.
    fn choisir(status: Option<i32>, ordinaire: Récolte, erreur: Récolte, source: Source) -> Self {
        let texte = ordinaire.texte();
        if texte.trim().is_empty() {
            return Self {
                text: erreur.texte(),
                status,
                truncated: erreur.tronqué,
                source: Source::Stderr,
            };
        }
        Self {
            text: texte,
            status,
            truncated: ordinaire.tronqué,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, contenu: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contenu).unwrap();
        path
    }

    /// Une sortie plus grosse qu'un tube revient quand même.
    ///
    /// Un tube a un tampon d'une soixantaine de kilo-octets : plein,
    /// l'écriture suivante bloque l'outil. Tant qu'on attendait sa fin
    /// avant de le lire, l'outil attendait qu'on le lise — un
    /// interblocage parfait, que seul le délai finissait par rompre.
    /// `objdump -d /bin/ls` rend plusieurs méga-octets : namsdec le
    /// déclarait « arrêté au bout de cent vingt secondes » alors
    /// qu'objdump avait fini en une fraction de seconde.
    ///
    /// `cat` pour éprouver cela : il est partout, et il rend
    /// exactement ce qu'on lui donne.
    #[test]
    fn an_output_bigger_than_a_pipe_still_comes_back() {
        if crate::known::program_path("cat").is_none() {
            return;
        }
        let plugin = Plugin {
            id: "cat".to_owned(),
            name: "cat".to_owned(),
            command: "cat".to_owned(),
            args: vec!["{file}".to_owned()],
            install: None,
        };
        // Un méga-octet : seize fois le tampon d'un tube.
        let gros = vec![b'A'; 1024 * 1024];
        let début = std::time::Instant::now();
        let rendu = plugin
            .run_within(&gros, Duration::from_secs(20))
            .expect("cat rend ce qu'on lui donne");
        assert_eq!(rendu.len(), gros.len(), "tout est revenu");
        assert!(
            début.elapsed() < Duration::from_secs(5),
            "et sans attendre le délai : {:?}",
            début.elapsed()
        );
    }

    /// Un outil qui ne rend jamais la main est arrêté, et on le dit.
    #[test]
    fn a_tool_that_never_returns_is_stopped() {
        if crate::known::program_path("sleep").is_none() {
            return;
        }
        // Un outil qui ne fait rien pendant trente secondes : c'est
        // exactement ce qu'un décompilateur parti en boucle ressemble
        // à faire, et il ne faut pas l'attendre.
        let plugin = Plugin {
            id: "sleep".to_owned(),
            name: "sleep".to_owned(),
            command: "sleep".to_owned(),
            args: vec!["30".to_owned()],
            install: None,
        };
        let début = std::time::Instant::now();
        let rendu = plugin.run_within(b"peu importe", Duration::from_secs(1));
        assert!(
            matches!(rendu, Err(PluginError::TooLong(1))),
            "il faut le dire, et non attendre : {rendu:?}"
        );
        assert!(
            début.elapsed() < Duration::from_secs(5),
            "et le dire tout de suite après le délai : {:?}",
            début.elapsed()
        );
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
        assert!(réponse.text.contains("base=0x401000"), "{réponse}");
        assert!(réponse.text.contains("CODE"), "{réponse}");
        assert!(
            !réponse.text.contains("en-tete"),
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
        assert!(réponse.text.contains("tout le fichier"), "{réponse}");
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
mod bornes_tests {
    use super::*;

    /// Une fiche qui lance `sh -c` sur un script qu'on écrit.
    ///
    /// `{file}` est exigé par les contrôles de `read`, et il est bien
    /// là : le script le reçoit et l'ignore. C'est le seul moyen
    /// d'éprouver des sorties précises — volumes, encodages, codes de
    /// retour — sans dépendre de ce qui est installé sur la machine.
    fn fiche(script: &str) -> Plugin {
        Plugin {
            id: "epreuve".to_owned(),
            name: "épreuve".to_owned(),
            command: "sh".to_owned(),
            args: vec![
                "-c".to_owned(),
                script.to_owned(),
                "--".to_owned(),
                "{file}".to_owned(),
            ],
            install: None,
        }
    }

    fn sh_existe() -> bool {
        crate::known::program_path("sh").is_some()
    }

    /// Quatre méga-octets moins un octet de « A », puis un « é » : la
    /// coupe ne doit pas tomber au milieu du caractère.
    ///
    /// C'est la reproduction, retournée en contrôle. `borner` tronquait
    /// la `String` à un INDICE D'OCTET ; quand cet indice tombait entre
    /// les deux octets d'un « é », `String::truncate` paniquait, et le
    /// fil de travail mourait avec le résultat.
    #[test]
    fn a_cut_never_falls_inside_a_character() {
        if !sh_existe() {
            return;
        }
        // 4 Mio - 1 octet de « A », puis « é » (deux octets) : la borne
        // tombe exactement entre les deux octets du « é ».
        let plugin = fiche(
            r"printf 'A%.0s' $(seq 1 4194303); printf '\303\251'; printf 'B%.0s' $(seq 1 100)",
        );
        let rendu = plugin
            .run_within(b"peu importe", Duration::from_secs(30))
            .expect("l'outil répond");
        assert!(
            rendu.starts_with("AAAA"),
            "le début est là : {:?}",
            &rendu[..rendu.len().min(8)]
        );
        assert!(rendu.contains("sortie coupée"), "et la coupe est annoncée");
        // La seule chose qui compte vraiment : on est arrivé ici. Avant,
        // la panique se produisait dans `borner`.
    }

    /// Cinq méga-octets sur la SEULE sortie d'erreur reviennent bornés.
    ///
    /// Le repli sur stderr se faisait après la troncature, sans lui
    /// appliquer la même limite : le plafond annoncé de 4 Mio ne valait
    /// que pour la sortie ordinaire.
    #[test]
    fn standard_error_obeys_the_same_cap() {
        if !sh_existe() {
            return;
        }
        let plugin = fiche("printf 'E%.0s' $(seq 1 5242880) >&2");
        let rendu = plugin
            .run_on(&Cible::brute(b"peu importe"), Duration::from_secs(60))
            .expect("l'outil répond");
        assert_eq!(rendu.source, Source::Stderr, "rien sur la sortie ordinaire");
        assert!(
            rendu.text.len() <= MAX_OUTPUT + 64,
            "borné comme l'autre : {} octets",
            rendu.text.len()
        );
        assert!(rendu.truncated, "et il le dit");
    }

    /// Le code de sortie est gardé, et un échec reste un échec.
    ///
    /// `try_wait` rendait le statut, et on le jetait : un programme qui
    /// écrit « tool failed » puis sort avec 7 donnait `Ok("tool
    /// failed\n")`, que rien ne distinguait d'un résultat normal.
    #[test]
    fn the_exit_status_survives() {
        if !sh_existe() {
            return;
        }
        let rendu = fiche("echo 'tool failed' >&2; exit 7")
            .run_on(&Cible::brute(b"peu importe"), Duration::from_secs(10))
            .expect("l'outil répond");
        assert_eq!(rendu.status, Some(7), "le code est là");
        assert!(!rendu.succeeded(), "et il ne vaut pas un succès");
        assert!(rendu.text.contains("tool failed"), "le texte reste visible");

        let bon = fiche("echo ok")
            .run_on(&Cible::brute(b"peu importe"), Duration::from_secs(10))
            .expect("l'outil répond");
        assert_eq!(bon.status, Some(0));
        assert!(bon.succeeded());
    }

    /// Un descendant qui garde les tubes ouverts ne prolonge plus
    /// l'attente.
    ///
    /// On tuait l'enfant direct, puis on attendait les fils de lecture —
    /// et ce `join` n'était soumis à aucun délai. Un `sleep` lancé en
    /// héritant des sorties les tenait ouvertes : un délai demandé de
    /// 100 ms rendait la main au bout de deux secondes. L'enfant est
    /// désormais seul dans son groupe, et c'est le GROUPE qu'on arrête.
    #[test]
    fn a_grandchild_cannot_outlive_the_deadline() {
        if !sh_existe() {
            return;
        }
        // Un petit-fils qui dort cinq secondes en héritant des sorties,
        // et un père qui ne finit pas non plus.
        let plugin = fiche("sleep 5 & sleep 5");
        let début = std::time::Instant::now();
        let rendu = plugin.run_within(b"peu importe", Duration::from_millis(100));
        let mis = début.elapsed();
        assert!(
            matches!(rendu, Err(PluginError::TooLong(0))),
            "l'arrêt est annoncé : {rendu:?}"
        );
        // Le délai, plus la grâce accordée aux fils, plus de la marge
        // pour une machine chargée — et très en deçà des cinq secondes
        // du petit-fils.
        assert!(
            mis < Duration::from_secs(3),
            "la main revient en {mis:?}, pas en cinq secondes"
        );
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
