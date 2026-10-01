//! Les étiquettes du source : trouver la ligne d'un libellé, l'étiquette
//! qui englobe une ligne, lister les étiquettes dans l'ordre.
//!
//! Des fonctions pures sur du texte, sans écran ni état — c'est pourquoi
//! elles vivent à part et se testent directement. Sorties de `main.rs`
//! pour l'alléger.

/// Le texte de la ligne `ligne`, en comptant à partir de 1, débarrassé
/// de son indentation.
///
/// Le profil aligne ses colonnes lui-même : garder les espaces du début
/// décalerait chaque ligne d'une profondeur qui n'a plus de sens une
/// fois sortie du programme.
pub(crate) fn source_line(source: &str, ligne: u32) -> String {
    source
        .lines()
        .nth(ligne.saturating_sub(1) as usize)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// La même chose, sur un source DÉJÀ DÉCOUPÉ.
///
/// `source_line` reparcourt le texte depuis le début à chaque appel :
/// c'est O(n), et sans conséquence tant qu'on le demande une fois. Le
/// profil le demandait pour cinq cents lignes d'un listing d'un million
/// d'octets, à chaque image — un gigaoctet parcouru par frame, dix-sept
/// secondes. Découper une fois et indexer coûte O(1) par ligne.
pub(crate) fn source_line_in(lignes: &[&str], ligne: u32) -> String {
    lignes
        .get(ligne.saturating_sub(1) as usize)
        .unwrap_or(&"")
        .trim()
        .to_owned()
}

/// L'étiquette sous laquelle vit la ligne `ligne`, s'il y en a une.
///
/// La plus proche au-dessus : c'est le bloc auquel la ligne appartient.
/// Dans un profil, « boucle: dec rcx » situe la ligne sans rien aller
/// chercher, là où un numéro seul oblige à retourner dans le programme.
/// Les étiquettes locales de nasm — celles qui commencent par un point
/// — ne comptent pas : elles ne portent pas de sens à elles seules.
pub(crate) fn enclosing_label(source: &str, ligne: u32) -> Option<String> {
    enclosing_label_at(&source_labels(source), ligne)
}

/// La même chose, à partir des étiquettes RELEVÉES UNE FOIS.
///
/// La version qui prend le texte le remonte ligne par ligne depuis la
/// ligne visée jusqu'à trouver une étiquette. Devant un binaire sans
/// symboles, il n'y en a AUCUNE : elle parcourt donc les soixante mille
/// lignes du listing, en découpant chacune, et cela pour chaque ligne du
/// profil. Cinq cents fois par image, vingt-quatre millisecondes
/// chacune — les douze secondes du profil sont là.
///
/// `source_labels` donne la liste des étiquettes avec leur ligne, en un
/// seul passage. Trouver celle qui précède devient une recherche
/// binaire, et le pire cas — aucune étiquette — devient le plus rapide.
pub(crate) fn enclosing_label_at(étiquettes: &[(String, u32)], ligne: u32) -> Option<String> {
    // La dernière étiquette dont la ligne est <= la nôtre. `partition_point`
    // rend le nombre d'éléments qui satisfont le prédicat, donc l'index
    // juste après celle qu'on cherche.
    let index = étiquettes.partition_point(|(_, l)| *l <= ligne);
    index.checked_sub(1).map(|i| étiquettes[i].0.clone())
}

/// Toutes les étiquettes du programme, avec leur ligne, dans l'ordre où
/// elles apparaissent.
///
/// Les mêmes règles que partout ici : en tête de ligne, suivies de deux
/// points, et les étiquettes locales de nasm — celles qui commencent par
/// un point — laissées de côté, puisqu'elles ne veulent rien dire hors
/// de leur bloc.
pub(crate) fn source_labels(source: &str) -> Vec<(String, u32)> {
    source
        .lines()
        .enumerate()
        .filter_map(|(index, texte)| {
            let texte = texte.split(';').next().unwrap_or("");
            if texte.starts_with([' ', '\t']) {
                return None;
            }
            let nom = texte.split_whitespace().next()?.strip_suffix(':')?;
            (!nom.is_empty() && !nom.starts_with('.')).then(|| (nom.to_owned(), index as u32 + 1))
        })
        .collect()
}

/// La ligne où l'étiquette `name` est définie, en comptant à partir de 1.
///
/// Une étiquette se déclare en tête de ligne, suivie de deux points :
/// `boucle:`. Ce qui apparaît ailleurs — dans un `jmp`, dans un
/// commentaire — n'est pas une définition et ne compte pas.
pub(crate) fn label_line(source: &str, name: &str) -> Option<u32> {
    source.lines().enumerate().find_map(|(index, line)| {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix(name)?;
        // `boucle:` et non `bouclette:` — le nom doit s'arrêter là.
        let rest = rest.trim_start();
        rest.starts_with(':').then_some(index as u32 + 1)
    })
}
