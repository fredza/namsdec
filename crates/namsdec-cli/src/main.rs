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
//! * **et surtout, la comparaison de deux versions d'un binaire** :
//!   `--contre` dit ce qui a changé dans l'ANALYSE — une protection qui
//!   disparaît, une chaîne qui apparaît, une fonction qui grossit —,
//!   rubrique par rubrique, et le code de retour le dit aussi. Un `diff`
//!   d'octets, lui, répond « le fichier a changé à l'offset 0x15604 »,
//!   ce qui n'apprend rien.
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
    /// L'autre version, à comparer avec celle qu'on analyse.
    ///
    /// C'EST LA PREMIÈRE VERSION, et le fichier de la ligne est la
    /// seconde : « namsdec-cli programme.v2 --contre programme.v1 » se
    /// lit « analyse v2, contre v1 », et le rapport dit v1 → v2. Dans
    /// l'autre sens, les `+` et les `-` seraient à l'envers de ce qu'on
    /// a dans la tête.
    contre: Option<PathBuf>,
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
    /// Les deux analyses comparées diffèrent.
    ///
    /// **Comme `diff`**, qui rend 1 quand les fichiers ne se
    /// ressemblent pas : en CI, ce qui intéresse est qu'une analyse ait
    /// bougé — une protection qui disparaît entre deux versions doit
    /// pouvoir arrêter une chaîne, sans qu'on relise un rapport pour
    /// s'en apercevoir.
    ///
    /// 4 ET NON 1, parce que 1 dit déjà « la ligne de commande est
    /// fautive ». Confondre les deux ferait passer une faute de frappe
    /// pour une régression du binaire.
    pub const ÇA_A_CHANGÉ: u8 = 4;
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

    // LA COMPARAISON, qui est une autre question : non pas « que dit ce
    // fichier ? » mais « qu'est-ce qui a changé depuis l'autre ? ». Elle
    // vient avant les autres sorties parce qu'elle les remplace toutes.
    if let Some(ancien) = demande.contre.clone() {
        // ET CE QU'ELLE REMPLACE, ON LE DIT. Sans cela, « --rubriques
        // --contre » comparait en IGNORANT `--rubriques` : l'option
        // était là, elle ne faisait rien, et rien ne le disait. C'est
        // le même défaut qu'un format ignoré en silence.
        let inutile = if demande.lister {
            Some("--rubriques")
        } else if demande.sans_prose {
            // Une comparaison n'a pas de prose d'ouverture : l'option
            // décrit ce qu'elle fait déjà, et une option qui ne change
            // rien ferait croire qu'elle change quelque chose.
            Some("--sans-prose")
        } else {
            None
        };
        if let Some(inutile) = inutile {
            return Err(t(
                &demande.langue,
                &format!(
                    "namsdec-cli : « {inutile} » ne veut rien dire avec « --contre », qui compare deux analyses. Les deux séparément, ou l'un des deux."
                ),
                &format!(
                    "namsdec-cli: `{inutile}` means nothing with `--against`, which compares two analyses. Either separately, or just one of them."
                ),
            ));
        }
        return comparer(&demande, &chemin, &octets, &ancien, &titre);
    }

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

/// Deux versions d'un binaire, comparées par leur analyse.
///
/// **C'est ce que cette commande existait pour rendre possible, et
/// qu'elle ne faisait pas encore.** On s'en sortait par un `diff` sur
/// deux sorties JSON : cela marche, et cela se lit mal — on y voit des
/// `"texte":` et des accolades, c'est-à-dire la forme du rapport et non
/// le fond. Ici, c'est `namsdec_re::comparaison` qui compare ce que
/// namsdec COMPREND, rubrique par rubrique.
fn comparer(
    demande: &Demande,
    chemin: &std::path::Path,
    octets: &[u8],
    ancien: &std::path::Path,
    titre: &str,
) -> Result<u8, String> {
    let anciens_octets = match std::fs::read(ancien) {
        Ok(octets) => octets,
        Err(erreur) => {
            eprintln!(
                "{}",
                t(
                    &demande.langue,
                    &format!(
                        "namsdec-cli : {} ne se lit pas — {erreur}",
                        ancien.display()
                    ),
                    &format!("namsdec-cli: cannot read {} — {erreur}", ancien.display()),
                )
            );
            return Ok(code::ILLISIBLE);
        }
    };
    // LES DEUX FICHIERS DOIVENT AVOIR QUELQUE CHOSE À DIRE. Comparer une
    // analyse à rien rendrait « tout a disparu », ce qui est vrai et
    // trompeur : le défaut est que le second fichier est vide.
    if anciens_octets.is_empty() {
        eprintln!(
            "{}",
            t(
                &demande.langue,
                &format!(
                    "namsdec-cli : {} est vide — il n'y a rien à comparer.",
                    ancien.display()
                ),
                &format!(
                    "namsdec-cli: {} is empty — nothing to compare.",
                    ancien.display()
                ),
            )
        );
        return Ok(code::RIEN_À_DIRE);
    }

    let mut avant = namsdec_re::rapport::analyse(&anciens_octets, &demande.langue);
    let mut après = namsdec_re::rapport::analyse(octets, &demande.langue);
    if avant.is_empty() || après.is_empty() {
        let muet = if avant.is_empty() { ancien } else { chemin };
        return Ok(rien_à_dire(&demande.langue, muet));
    }
    // `--rubrique` RESTREINT LA COMPARAISON, et c'est là qu'elle sert le
    // plus : « ce qui a changé dans les protections » est la question
    // qu'on pose en CI, et les empreintes d'un binaire recompilé
    // changent toujours.
    if !demande.rubriques.is_empty() {
        let garder = |panneaux: &mut Vec<namsdec_re::rapport::Panneau>| {
            panneaux.retain(|p| demande.rubriques.iter().any(|id| id == &p.id));
        };
        garder(&mut avant);
        garder(&mut après);
        if avant.is_empty() && après.is_empty() {
            return Err(t(
                &demande.langue,
                "namsdec-cli : aucune des rubriques demandées n'est dans ces deux fichiers. « --rubriques » les liste.",
                "namsdec-cli: none of the requested sections is in either file. `--sections` lists them.",
            ));
        }
    }

    let comparaison = namsdec_re::comparaison::comparer(&avant, &après);
    let nom = |chemin: &std::path::Path| {
        chemin
            .file_name()
            .map(|nom| nom.to_string_lossy().into_owned())
            .unwrap_or_else(|| chemin.display().to_string())
    };
    let texte = match demande.format {
        Format::Json => namsdec_re::comparaison::json(&comparaison),
        Format::Markdown => {
            namsdec_re::comparaison::markdown(&comparaison, &nom(ancien), titre, &demande.langue)
        }
    };
    écrire(demande, texte)?;
    // LE CODE DE RETOUR PORTE LA RÉPONSE, et c'est tout l'intérêt en
    // CI : on n'a pas à relire le rapport pour savoir si quelque chose a
    // bougé.
    Ok(if comparaison.rien_ne_change() {
        code::SUCCÈS
    } else {
        code::ÇA_A_CHANGÉ
    })
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
            "--contre" | "--against" | "-c" => demande.contre = Some(PathBuf::from(valeur(arg)?)),
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
  -c, --contre <fichier>        Comparer : dire ce qui a changé dans l'analyse
                                depuis CE fichier-là. L'ancien d'abord dans le
                                rapport, le nouveau ensuite.
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
  0  le rapport est écrit (en comparaison : rien n'a changé)
  1  la ligne de commande est fautive
  2  le fichier ne se lit pas
  3  il se lit, mais il n'y a rien à analyser (vide, ou aucune rubrique)
  4  en comparaison seulement : les deux analyses diffèrent — comme « diff »,
     qui rend 1 pour cela. En CI, c'est ce code qui arrête la chaîne quand une
     protection a disparu, sans qu'on relise le rapport.

EXEMPLES
  namsdec-cli programme                       le rapport, à l'écran
  namsdec-cli programme -o rapport.md         le rapport, dans un fichier
  namsdec-cli prog.v2 --contre prog.v1        ce qui a changé dans l'analyse,
                                              rubrique par rubrique
  namsdec-cli prog.v2 -c prog.v1 -s protections
                                              la seule question qui compte en
                                              CI : une protection a-t-elle
                                              disparu ? (les empreintes, elles,
                                              changent à chaque compilation)
  namsdec-cli programme -f json -o a.json     l'analyse entière, pour un autre
                                              programme
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
  -c, --against <file>          Compare: say what changed in the analysis
                                since THAT file. The older one comes first in
                                the report, the newer one second.
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
  0  the report is written (when comparing: nothing changed)
  1  the command line is wrong
  2  the file cannot be read
  3  it reads, but there is nothing to analyse (empty, or no section)
  4  when comparing only: the two analyses differ — like `diff`, which returns
     1 for that. In CI, this is the code that stops the pipeline when a
     protection is gone, without anyone reading the report.

EXAMPLES
  namsdec-cli program                       the report, on screen
  namsdec-cli program -o report.md          the report, into a file
  namsdec-cli prog.v2 --against prog.v1     what changed in the analysis,
                                            section by section
  namsdec-cli prog.v2 -c prog.v1 -s protections
                                            the only question that matters in
                                            CI: is a protection gone? (the
                                            fingerprints change on every build)
  namsdec-cli program -f json -o a.json     the whole analysis, for another
                                            program
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
