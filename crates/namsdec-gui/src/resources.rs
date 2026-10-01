//! Les dossiers de données et les ressources installées : où namsdec
//! pose ce qu'il écrit, et comment il relit greffons et thèmes posés à
//! côté. S'y ajoutent les deux libellés de région, qui vivaient là.
//!
//! Sortis de `main.rs` pour l'alléger.

use super::*;

/// Les désassembleurs installés : un fichier JSON par outil, dans le
/// dossier des plugins.
///
/// Rien n'est lancé ici — on lit seulement ce qui est déclaré. Un
/// désassembleur extérieur *est un programme* : il ne s'exécute que
/// lorsqu'on le choisit, et le panneau le dit avant.
pub(crate) fn load_disassembler_plugins() -> Vec<namsdec_disasm::plugin::Plugin> {
    let Some(dir) = data_dir().map(|dir| dir.join("disassemblers")) else {
        return Vec::new();
    };
    namsdec_disasm::plugin::read_dir(&dir)
}

/// Où vivent les thèmes installés : à côté des réglages, dans le dossier
/// de données de l'utilisateur.
pub(crate) fn plugins_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("themes"))
}

/// Le dossier de données de l'application : ce que l'utilisateur y ajoute
/// — thèmes, séries d'exercices — y reste d'une version à l'autre.
pub(crate) fn data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })?;
    Some(base.join("namsdec"))
}

/// Les sections qu'un binaire ordinaire porte, et qui méritent d'être
/// nommées telles quelles dans la carte mémoire et dans les messages de
/// faute.
///
/// Un programme qu'on vient d'écrire en a trois ou quatre ; un exécutable
/// venu du disque en a vingt, et savoir qu'on est tombé dans `.got` ou
/// dans `.plt` plutôt que dans « des données » est souvent toute la
/// réponse à la question qu'on se pose.
const SECTIONS_CONNUES: &[&str] = &[
    ".text",
    ".data",
    ".bss",
    ".rodata",
    ".init",
    ".fini",
    ".plt",
    ".plt.sec",
    ".plt.got",
    ".got",
    ".got.plt",
    ".init_array",
    ".fini_array",
    ".data.rel.ro",
    ".dynamic",
    ".dynsym",
    ".dynstr",
    ".interp",
    ".eh_frame",
    ".eh_frame_hdr",
    ".gcc_except_table",
    ".tdata",
    ".tbss",
    ".rela.dyn",
    ".rela.plt",
    ".gnu.hash",
];

/// Le nom sous lequel annoncer une section dans un message de faute.
///
/// Les noms usuels passent tels quels ; une section au nom exotique est
/// annoncée par ce qu'elle *est* — du code, des données, de la donnée
/// en lecture seule. Le message doit pouvoir être écrit dans les deux
/// langues, et un nom venu du fichier ne peut pas l'être : il n'a pas de
/// durée de vie statique, et il ne se traduit pas.
pub(crate) fn region_label(section: &namsdec_asm::Section) -> &'static str {
    region_label_named(&section.name, section.executable, section.writable)
}

/// Même chose, depuis les faits plutôt que depuis la structure : le
/// chargeur travaille sur les sections de l'ELF, qui ne sont pas du même
/// type que celles d'un programme assemblé mais portent les mêmes noms.
pub(crate) fn region_label_named(name: &str, executable: bool, writable: bool) -> &'static str {
    if let Some(connu) = SECTIONS_CONNUES.iter().find(|connu| **connu == name) {
        return connu;
    }
    if executable {
        ".text"
    } else if writable {
        ".data"
    } else {
        ".rodata"
    }
}

/// Où vivent les noms et les commentaires qu'on pose sur un binaire.
///
/// À côté du binaire, jamais dedans : on n'écrit pas dans l'exécutable
/// de quelqu'un pour y noter ce qu'on en pense.
pub(crate) fn notes_dir() -> Option<PathBuf> {
    data_dir().map(|base| base.join("notes"))
}

/// Recopie le fichier de thème dans le dossier des plugins, pour qu'il
/// revienne au lancement suivant. Un fichier déjà installé est écrasé :
/// c'est la version qu'on vient de choisir qui fait foi.
pub(crate) fn copy_into_plugins(path: &Path) -> std::io::Result<PathBuf> {
    copy_into_dir(path, plugins_dir())
}

/// Recopie un fichier dans un dossier de données, en le créant au besoin.
/// Un fichier déjà installé est écrasé : c'est la version qu'on vient de
/// choisir qui fait foi.
pub(crate) fn copy_into_dir(path: &Path, dir: Option<PathBuf>) -> std::io::Result<PathBuf> {
    let dir = dir.ok_or_else(|| {
        std::io::Error::other("aucun dossier de données (ni XDG_DATA_HOME ni HOME)")
    })?;
    std::fs::create_dir_all(&dir)?;
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("le chemin ne nomme pas un fichier"))?;
    let destination = dir.join(name);
    // Se recopier sur soi-même n'a pas de sens, et `copy` s'en plaindrait.
    if path != destination {
        std::fs::copy(path, &destination)?;
    }
    Ok(destination)
}

/// Charge les thèmes déjà installés. Appelé une fois au démarrage.
///
/// Un fichier illisible n'empêche pas les autres de se charger : il est
/// nommé dans le compte rendu, et l'application démarre quand même —
/// perdre son éditeur parce qu'un thème a mal vieilli serait absurde.
pub(crate) fn load_installed_themes() -> (BTreeMap<String, PathBuf>, Vec<String>) {
    let mut files = BTreeMap::new();
    let mut failures = Vec::new();
    let Some(dir) = plugins_dir() else {
        return (files, failures);
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return (files, failures);
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    // Ordre stable : la liste des thèmes ne doit pas changer d'ordre d'un
    // lancement à l'autre.
    paths.sort();
    for path in paths {
        match namsdec_theme::plugin::read(&path).and_then(namsdec_theme::install) {
            Ok(theme) => {
                files.insert(theme.id.to_owned(), path);
            }
            Err(e) => failures.push(format!("{} : {e}", path.display())),
        }
    }
    (files, failures)
}
