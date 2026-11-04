//! `namsdec-cli` — l'analyse de namsdec, sans fenêtre.
//!
//! **Ce que cette commande existe pour rendre possible.** Jusqu'ici, ce
//! que namsdec comprend d'un binaire n'existait que peint à l'écran :
//! pour le lire, il fallait lancer l'application, ouvrir un fichier,
//! cliquer dans vingt-deux panneaux. La frontière P7 a sorti l'analyse
//! de l'interface — `rapport::analyse(octets, lang)` rend tout sans
//! peindre un seul pixel —, et cette commande est ce qui en découle :
//!
//! * un rapport Markdown qu'on colle dans un compte rendu, un ticket ou
//!   un support de cours, sans capture d'écran ;
//! * le même fond en JSON, pour un autre programme ;
//! * **et surtout, de quoi comparer deux versions d'un binaire** : une
//!   protection qui disparaît, une chaîne qui apparaît, une fonction qui
//!   grossit. Un `diff` de deux analyses, et non de deux tas d'octets —
//!   qui, lui, n'apprend rien. C'est la raison d'être de la sortie JSON
//!   stable.
//!
//! **Ce qu'elle n'est pas, et il faut le dire :** elle ne remplace pas
//! l'application. Elle repart des octets, donc elle ne porte rien de ce
//! qu'un utilisateur aurait fait — pas de tranche choisie dans le
//! vidage, pas de détour calculé, pas de nom donné à une fonction, pas
//! de sortie d'outil extérieur (`objdump`, `radare2`, Ghidra ne sont
//! jamais lancés ici, et les panneaux qui les portent restent muets).
//! C'est le prix de la reproductibilité : deux appels sur le même
//! fichier rendent le même texte, ce qu'aucune session interactive ne
//! garantit.
//!
//! **Et elle lie egui sans l'employer.** `namsdec-re` en dépend encore,
//! parce que le trait `ReModule` porte toujours `fn ui(…)` ; aucune
//! surface n'est créée, aucune image n'est rendue, et le programme
//! tourne sans écran. S'en défaire demande de déménager les vingt-deux
//! `ui()` dans `namsdec-gui` — c'est un autre lot, noté dans
//! `TRAVAUX.md`.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// Ce que la ligne de commande demande.
#[derive(Debug, Default)]
struct Demande {
    fichier: Option<PathBuf>,
    format: Format,
    sortie: Option<PathBuf>,
    langue: String,
    titre: Option<String>,
    /// Ne garder que ces rubriques, par leur identifiant.
    rubriques: Vec<String>,
    /// Lister les rubriques que ce fichier remplit, et s'arrêter là.
    lister: bool,
    /// Sauter la prose d'ouverture, et n'écrire que les panneaux.
    sans_prose: bool,
    aide: bool,
    version: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
enum Format {
    #[default]
    Markdown,
    Json,
}

/// Les codes de retour, parce qu'un programme qui tourne en CI est lu
/// par un autre programme avant de l'être par quelqu'un.
mod code {
    /// Le rapport est écrit.
    pub const SUCCÈS: u8 = 0;
    /// La ligne de commande est fautive — option inconnue, fichier
    /// absent de la ligne, valeur manquante.
    pub const EMPLOI: u8 = 1;
    /// Le fichier ne se lit pas.
    pub const ILLISIBLE: u8 = 2;
    /// Il se lit, et il n'y a rien à analyser : fichier vide, ou aucune
    /// rubrique qui se remplisse.
    ///
    /// C'EST UN ÉTAT À PART, et non un succès avec un rapport maigre :
    /// en CI, « l'analyse n'a rien trouvé » doit pouvoir arrêter une
    /// chaîne, sans quoi un artefact tronqué passe pour analysé — avec
    /// un titre, des tableaux et des sections qui disent toutes zéro.
    pub const RIEN_À_DIRE: u8 = 3;
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match exécuter(&args) {
        Ok(code) => ExitCode::from(code),
        Err(plainte) => {
            eprintln!("{plainte}");
            ExitCode::from(code::EMPLOI)
        }
    }
}

fn exécuter(args: &[String]) -> Result<u8, String> {
    let demande = analyse_args(args)?;

    if demande.version {
        println!("namsdec-cli {}", env!("CARGO_PKG_VERSION"));
        return Ok(code::SUCCÈS);
    }
    if demande.aide || args.is_empty() {
        print!("{}", aide(&demande.langue));
        return Ok(code::SUCCÈS);
    }

    let Some(chemin) = demande.fichier.clone() else {
        return Err(t(
            &demande.langue,
            "namsdec-cli : il manque le fichier à analyser. « namsdec-cli --aide » dit comment.",
            "namsdec-cli: no file to analyse. Try `namsdec-cli --help`.",
        ));
    };

    let octets = match std::fs::read(&chemin) {
        Ok(octets) => octets,
        Err(erreur) => {
            // LE CHEMIN ET LA CAUSE, LES DEUX. « Permission denied »
            // sans le fichier ne dit pas lequel, et un script qui en
            // analyse trente ne le retrouve pas.
            eprintln!(
                "{}",
                t(
                    &demande.langue,
                    &format!(
                        "namsdec-cli : {} ne se lit pas — {erreur}",
                        chemin.display()
                    ),
                    &format!("namsdec-cli: cannot read {} — {erreur}", chemin.display()),
                )
            );
            return Ok(code::ILLISIBLE);
        }
    };

    // UN FICHIER VIDE N'EST PAS UN FICHIER ANALYSÉ, et c'est le cas
    // qu'une CI rencontre vraiment : un artefact tronqué, un
    // téléchargement coupé, un chemin qui pointait sur un fichier
    // créé et jamais rempli. L'analyse, elle, ne s'en plaint pas —
    // elle rend dix-huit rubriques qui disent toutes « 0 octet », «
    // aucun fichier chargé », « pas d'en-tête ELF ». Un rapport pareil
    // PASSE POUR UNE ANALYSE : il a un titre, des tableaux, des
    // sections. Le laisser sortir avec un code 0 était le seul vrai
    // mensonge de cette commande.
    if octets.is_empty() {
        eprintln!(
            "{}",
            t(
                &demande.langue,
                &format!(
                    "namsdec-cli : {} est vide — 0 octet, il n'y a rien à analyser.",
                    chemin.display()
                ),
                &format!(
                    "namsdec-cli: {} is empty — 0 bytes, nothing to analyse.",
                    chemin.display()
                ),
            )
        );
        return Ok(code::RIEN_À_DIRE);
    }

    let titre = demande.titre.clone().unwrap_or_else(|| {
        chemin
            .file_name()
            .map(|nom| nom.to_string_lossy().into_owned())
            .unwrap_or_else(|| chemin.display().to_string())
    });

    // LE CAS COURT D'ABORD : lister les rubriques ne demande pas
    // d'écrire un rapport, et c'est ce qu'on fait avant de choisir
    // `--rubrique`.
    if demande.lister {
        let panneaux = namsdec_re::rapport::analyse(&octets, &demande.langue);
        if panneaux.is_empty() {
            return Ok(rien_à_dire(&demande.langue, &chemin));
        }
        // `--format` VAUT ICI AUSSI. Il serait ignoré en silence
        // autrement, et une option qui ne fait rien là où on la pose
        // est pire qu'une option absente.
        let liste = match demande.format {
            Format::Markdown => {
                // Un identifiant, une tabulation, un titre : de quoi un
                // `cut -f1`.
                let mut lignes = String::new();
                for panneau in &panneaux {
                    lignes.push_str(&format!("{}\t{}\n", panneau.id, panneau.titre));
                }
                lignes
            }
            Format::Json => {
                let liste: Vec<serde_json::Value> = panneaux
                    .iter()
                    .map(|panneau| serde_json::json!({ "id": panneau.id, "titre": panneau.titre }))
                    .collect();
                let mut texte =
                    serde_json::to_string_pretty(&liste).unwrap_or_else(|_| "[]".to_owned());
                texte.push('\n');
                texte
            }
        };
        écrire(&demande, liste)?;
        return Ok(code::SUCCÈS);
    }

    // DEUX CHEMINS, ET NON UN AVEC UN FILTRE. Sans `--rubrique` ni
    // `--sans-prose`, le rapport Markdown est celui de l'application
    // entier, prose comprise, et il vient d'un seul appel qui ne
    // désassemble qu'une fois. Dès qu'on filtre, la prose ne vaut plus
    // — elle parlerait de rubriques qu'on vient d'écarter.
    let texte =
        if demande.format == Format::Json || demande.sans_prose || !demande.rubriques.is_empty() {
            let mut panneaux = namsdec_re::rapport::analyse(&octets, &demande.langue);
            if panneaux.is_empty() {
                return Ok(rien_à_dire(&demande.langue, &chemin));
            }
            if !demande.rubriques.is_empty() {
                let avant = panneaux.len();
                panneaux.retain(|panneau| demande.rubriques.iter().any(|id| id == &panneau.id));
                if panneaux.is_empty() {
                    // ON DIT CE QUI EXISTAIT. Une rubrique mal orthographiée
                    // rendait un rapport vide sans un mot, et l'on cherchait
                    // le défaut dans le binaire.
                    return Err(t(
                        &demande.langue,
                        &format!(
                            "namsdec-cli : aucune des rubriques demandées n'est dans ce fichier \
                         ({avant} y sont). « --rubriques » les liste."
                        ),
                        &format!(
                            "namsdec-cli: none of the requested sections is in this file \
                         ({avant} are). `--sections` lists them."
                        ),
                    ));
                }
            }
            match demande.format {
                Format::Json => namsdec_re::rapport::json(&panneaux),
                Format::Markdown => namsdec_re::rapport::markdown(&panneaux, &titre),
            }
        } else {
            match namsdec_re::rapport::rapport_complet(&octets, &demande.langue, &titre) {
                Some(texte) => texte,
                None => return Ok(rien_à_dire(&demande.langue, &chemin)),
            }
        };

    écrire(&demande, texte)?;
    Ok(code::SUCCÈS)
}

/// Le fichier se lit, et namsdec n'en comprend rien.
fn rien_à_dire(langue: &str, chemin: &std::path::Path) -> u8 {
    eprintln!(
        "{}",
        t(
            langue,
            &format!(
                "namsdec-cli : rien à dire de {} — aucune rubrique ne le remplit.",
                chemin.display()
            ),
            &format!(
                "namsdec-cli: nothing to say about {} — no section applies.",
                chemin.display()
            ),
        )
    );
    code::RIEN_À_DIRE
}

/// Sur la sortie standard, ou dans le fichier demandé.
fn écrire(demande: &Demande, texte: String) -> Result<(), String> {
    match &demande.sortie {
        None => {
            // `print!` PANIQUE SI LE TUYAU SE FERME, et
            // `namsdec-cli … | head` le ferme toujours. Un programme
            // qui se termine par « failed printing to stdout » au lieu
            // de se taire est un programme cassé.
            let mut sortie = std::io::stdout();
            match sortie
                .write_all(texte.as_bytes())
                .and_then(|()| sortie.flush())
            {
                Ok(()) => Ok(()),
                Err(erreur) if erreur.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
                Err(erreur) => Err(format!("namsdec-cli : {erreur}")),
            }
        }
        Some(chemin) => std::fs::write(chemin, texte).map_err(|erreur| {
            t(
                &demande.langue,
                &format!(
                    "namsdec-cli : {} ne s'écrit pas — {erreur}",
                    chemin.display()
                ),
                &format!("namsdec-cli: cannot write {} — {erreur}", chemin.display()),
            )
        }),
    }
}

/// Les arguments, lus une fois.
///
/// Les noms sont français, et l'anglais est accepté partout : le projet
/// parle les deux langues à l'écran, une commande qui n'en parlerait
/// qu'une se refuserait à la moitié de ceux qui l'emploient.
fn analyse_args(args: &[String]) -> Result<Demande, String> {
    let mut demande = Demande {
        langue: langue_par_défaut(args),
        ..Demande::default()
    };
    let mut reste = args.iter();
    // Après `--`, tout est un chemin — y compris un fichier qui
    // s'appelle `--format`.
    let mut plus_d_options = false;

    while let Some(arg) = reste.next() {
        let mut valeur = |nom: &str| -> Result<String, String> {
            reste.next().cloned().ok_or_else(|| {
                t(
                    &demande.langue,
                    &format!("namsdec-cli : « {nom} » attend une valeur."),
                    &format!("namsdec-cli: `{nom}` expects a value."),
                )
            })
        };
        match arg.as_str() {
            _ if plus_d_options => demande.fichier = Some(PathBuf::from(arg)),
            "--" => plus_d_options = true,
            "--aide" | "--help" | "-h" => demande.aide = true,
            "--version" | "-V" => demande.version = true,
            "--rubriques" | "--sections" => demande.lister = true,
            "--sans-prose" | "--no-prose" => demande.sans_prose = true,
            "--format" | "-f" => {
                let demandé = valeur(arg)?;
                demande.format = match demandé.as_str() {
                    "markdown" | "md" => Format::Markdown,
                    "json" => Format::Json,
                    autre => {
                        return Err(t(
                            &demande.langue,
                            &format!(
                                "namsdec-cli : format inconnu « {autre} » — markdown ou json."
                            ),
                            &format!("namsdec-cli: unknown format `{autre}` — markdown or json."),
                        ));
                    }
                };
            }
            "--sortie" | "--output" | "-o" => demande.sortie = Some(PathBuf::from(valeur(arg)?)),
            "--langue" | "--lang" | "-l" => demande.langue = valeur(arg)?,
            "--titre" | "--title" | "-t" => demande.titre = Some(valeur(arg)?),
            "--rubrique" | "--section" | "-s" => demande.rubriques.push(valeur(arg)?),
            autre if autre.starts_with('-') && autre.len() > 1 => {
                return Err(t(
                    &demande.langue,
                    &format!(
                        "namsdec-cli : option inconnue « {autre} ». \
                         « namsdec-cli --aide » les liste."
                    ),
                    &format!(
                        "namsdec-cli: unknown option `{autre}`. \
                         Try `namsdec-cli --help`."
                    ),
                ));
            }
            // DEUX FICHIERS, ET L'ON S'ARRÊTE. Le second écrasait
            // silencieusement le premier, et `namsdec-cli a.bin b.bin`
            // n'analysait que b.bin en disant le contraire par son
            // titre.
            _ if demande.fichier.is_some() => {
                return Err(t(
                    &demande.langue,
                    "namsdec-cli : un seul fichier à la fois.",
                    "namsdec-cli: one file at a time.",
                ));
            }
            _ => demande.fichier = Some(PathBuf::from(arg)),
        }
    }
    Ok(demande)
}

/// La langue des messages, avant que les arguments ne soient lus.
///
/// Il faut la connaître pour se plaindre d'un argument fautif, donc
/// avant de pouvoir compter sur la lecture des arguments : on la
/// cherche d'abord dans la ligne de commande, puis dans
/// l'environnement, et le français reste le défaut — c'est la langue
/// du projet.
fn langue_par_défaut(args: &[String]) -> String {
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if matches!(arg.as_str(), "--langue" | "--lang" | "-l")
            && let Some(langue) = args.next()
        {
            return langue.clone();
        }
    }
    match std::env::var("LANG") {
        Ok(lang) if lang.starts_with("en") => "en".to_owned(),
        _ => "fr".to_owned(),
    }
}

fn t(langue: &str, fr: &str, en: &str) -> String {
    if langue.starts_with("fr") { fr } else { en }.to_owned()
}

fn aide(langue: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    if langue.starts_with("fr") {
        format!(
            "namsdec-cli {version} — l'analyse de namsdec, sans fenêtre.

EMPLOI
  namsdec-cli [options] <fichier>

OPTIONS
  -f, --format <markdown|json>  La forme du rapport. Markdown par défaut.
  -o, --sortie <chemin>         Où l'écrire. La sortie standard par défaut.
  -l, --langue <fr|en>          La langue du rapport. Celle du système, sinon fr.
  -t, --titre <texte>           Le titre du rapport. Le nom du fichier par défaut.
      --rubriques               Lister les rubriques que ce fichier remplit,
                                avec leur identifiant, et s'arrêter là.
                                « -f json » les rend en JSON.
  -s, --rubrique <id>           N'écrire que cette rubrique. Répétable.
      --sans-prose              Les panneaux seuls, sans le récit d'ouverture.
  -h, --aide                    Ceci.
  -V, --version                 Le numéro de version.
  --                            Tout ce qui suit est un chemin.

CODES DE RETOUR
  0  le rapport est écrit
  1  la ligne de commande est fautive
  2  le fichier ne se lit pas
  3  il se lit, mais il n'y a rien à analyser (vide, ou aucune rubrique)

EXEMPLES
  namsdec-cli programme                       le rapport, à l'écran
  namsdec-cli programme -o rapport.md         le rapport, dans un fichier
  namsdec-cli programme -f json -o a.json     de quoi comparer deux versions :
  namsdec-cli programme.v2 -f json -o b.json  puis « diff a.json b.json », qui
                                              dit ce qui a changé dans l'ANALYSE
                                              et non dans les octets
  namsdec-cli programme --rubriques           ce dont ce fichier a de quoi parler
  namsdec-cli programme -s protections        une seule rubrique

CE QUE CETTE COMMANDE NE FAIT PAS
  Elle repart des octets : elle ne porte rien d'une session — pas de nom
  donné à une fonction, pas de détour calculé. Et elle ne lance aucun
  outil extérieur : les rubriques objdump, radare2, Ghidra et retdec
  restent muettes. C'est ce qui rend son rapport reproductible.
"
        )
    } else {
        format!(
            "namsdec-cli {version} — namsdec's analysis, without a window.

USAGE
  namsdec-cli [options] <file>

OPTIONS
  -f, --format <markdown|json>  Report shape. Markdown by default.
  -o, --output <path>           Where to write it. Standard output by default.
  -l, --lang <fr|en>            Report language. The system's, else fr.
  -t, --title <text>            Report title. The file name by default.
      --sections                List the sections this file fills, with their
                                identifiers, and stop there. `-f json`
                                renders them as JSON.
  -s, --section <id>            Write only this section. Repeatable.
      --no-prose                Panels only, without the opening narrative.
  -h, --help                    This.
  -V, --version                 Version number.
  --                            Everything after this is a path.

EXIT CODES
  0  the report is written
  1  the command line is wrong
  2  the file cannot be read
  3  it reads, but there is nothing to analyse (empty, or no section)

EXAMPLES
  namsdec-cli program                       the report, on screen
  namsdec-cli program -o report.md          the report, into a file
  namsdec-cli program -f json -o a.json     how to compare two versions:
  namsdec-cli program.v2 -f json -o b.json  then `diff a.json b.json`, which
                                            tells what changed in the ANALYSIS,
                                            not in the bytes
  namsdec-cli program --sections            what this file has to talk about
  namsdec-cli program -s protections        a single section

WHAT THIS COMMAND DOES NOT DO
  It starts from the bytes: it carries nothing from a session — no name
  given to a function, no computed detour. And it runs no external
  tool: the objdump, radare2, Ghidra and retdec sections stay silent.
  That is what makes its report reproducible.
"
        )
    }
}
