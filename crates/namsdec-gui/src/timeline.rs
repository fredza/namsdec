//! La barre de chronologie : où l'on en est dans le temps du programme.
//!
//! La fenêtre « Chronologie » montre déjà la liste des pas, et répond
//! très bien à « qu'est-ce qui s'est passé ». Elle ne répond pas à
//! l'autre question, qui vient avant : **où j'en suis**. Cinq mille pas
//! dans une zone défilante ne donnent aucune idée de la forme du
//! parcours — on ne voit ni qu'on est au début, ni qu'une boucle vient
//! d'occuper les quatre cinquièmes du chemin.
//!
//! Une barre le dit d'un coup d'oeil, et sert en même temps de poignée :
//! on clique où l'on veut revenir. C'est ce qu'un lecteur de vidéo fait
//! avec une piste, et c'est le bon geste ici aussi.
//!
//! Ce fichier tient le **regroupement** — comment cinq mille pas
//! deviennent deux cents colonnes sans mentir —, qui se teste sans écran.

use namsdec_emu::{Step, StepKind};

/// Une colonne de la barre : ce qui s'est passé pendant la tranche de
/// temps qu'elle couvre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    /// Le premier pas de la tranche, en rang depuis le début.
    pub first: usize,
    /// Combien de pas elle couvre.
    pub count: usize,
    /// Ce qui s'est passé de plus notable dedans.
    pub kind: StepKind,
}

/// Le poids d'un genre de pas : plus il est grand, plus il mérite d'être
/// celui qu'on montre quand plusieurs pas tombent dans la même colonne.
///
/// Un `call` perdu au milieu de mille instructions ordinaires est
/// exactement ce qu'on cherche des yeux ; l'inverse n'est jamais vrai.
/// Rendre la moyenne, ou le premier venu, effacerait l'information.
fn weight(kind: StepKind) -> u8 {
    match kind {
        StepKind::Plain => 0,
        StepKind::Jump => 1,
        // Une écriture passe devant un saut : dans un crackme, c'est
        // souvent la seule trace de ce que le programme fabrique.
        StepKind::Write => 2,
        StepKind::Return => 3,
        StepKind::Call => 4,
        // L'appel système en tête : c'est le seul endroit où un
        // programme touche au monde extérieur, et donc le seul où il
        // peut faire quelque chose d'irréversible.
        StepKind::Syscall => 5,
    }
}

/// Regroupe un parcours en `width` colonnes au plus.
///
/// Quand il y a moins de pas que de colonnes, chaque pas a la sienne :
/// c'est le cas du pas à pas, où l'on veut voir chaque avancée.
pub fn columns(trace: &[Step], width: usize) -> Vec<Column> {
    if trace.is_empty() || width == 0 {
        return Vec::new();
    }
    let par_colonne = trace.len().div_ceil(width).max(1);
    trace
        .chunks(par_colonne)
        .enumerate()
        .map(|(i, tranche)| Column {
            first: i * par_colonne,
            count: tranche.len(),
            // Le plus notable de la tranche, non le premier : c'est lui
            // qui porte l'information.
            kind: tranche
                .iter()
                .map(|pas| pas.kind)
                .max_by_key(|kind| weight(*kind))
                .unwrap_or(StepKind::Plain),
        })
        .collect()
}

/// Le rang du pas visé quand on clique à `fraction` de la largeur.
///
/// `fraction` va de 0 à 1. Rend un rang depuis le début, borné au
/// dernier pas — cliquer sur le bord droit revient à « maintenant », non
/// à un pas qui n'existe pas.
pub fn step_at(trace_len: usize, fraction: f32) -> usize {
    if trace_len == 0 {
        return 0;
    }
    let brut = (fraction.clamp(0.0, 1.0) * trace_len as f32) as usize;
    brut.min(trace_len - 1)
}

/// De combien de pas reculer pour revenir au rang `index`.
///
/// La fenêtre compte le recul depuis maintenant — « -1 » est le dernier
/// pas —, et c'est ce que `step_back` attend.
pub fn rewind_to(trace_len: usize, index: usize) -> usize {
    trace_len.saturating_sub(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parcours(genres: &[StepKind]) -> Vec<Step> {
        genres
            .iter()
            .enumerate()
            .map(|(i, kind)| Step {
                addr: 0x400000 + i as u64,
                kind: *kind,
            })
            .collect()
    }

    /// Un parcours court donne une colonne par pas.
    ///
    /// C'est le cas du pas à pas : chaque avancée doit se voir, sans quoi
    /// la barre ne bougerait pas pendant qu'on avance.
    #[test]
    fn a_short_walk_gets_one_column_per_step() {
        let trace = parcours(&[StepKind::Plain; 5]);
        let colonnes = columns(&trace, 200);
        assert_eq!(colonnes.len(), 5);
        for (i, colonne) in colonnes.iter().enumerate() {
            assert_eq!(colonne.first, i);
            assert_eq!(colonne.count, 1);
        }
    }

    /// Un long parcours tient dans la largeur, et couvre tout.
    #[test]
    fn a_long_walk_fits_the_width_without_losing_a_step() {
        let trace = parcours(&[StepKind::Plain; 5000]);
        let colonnes = columns(&trace, 200);
        assert!(colonnes.len() <= 200, "{} colonnes", colonnes.len());
        let couvert: usize = colonnes.iter().map(|c| c.count).sum();
        assert_eq!(couvert, 5000, "aucun pas ne doit disparaître");
        // Et les colonnes se suivent sans trou ni recouvrement.
        for paire in colonnes.windows(2) {
            assert_eq!(paire[0].first + paire[0].count, paire[1].first);
        }
    }

    /// Un appel seul au milieu de mille pas ordinaires ne disparaît pas.
    ///
    /// C'est tout l'objet du regroupement : ce qu'on cherche des yeux est
    /// justement le rare. Prendre le premier pas de chaque tranche, ou
    /// une moyenne, l'effacerait — et la barre ne montrerait qu'une suite
    /// uniforme, c'est-à-dire rien.
    #[test]
    fn a_lone_call_survives_the_grouping() {
        let mut genres = vec![StepKind::Plain; 2000];
        genres[1234] = StepKind::Call;
        let trace = parcours(&genres);
        let colonnes = columns(&trace, 100);
        let appels: Vec<&Column> = colonnes
            .iter()
            .filter(|c| c.kind == StepKind::Call)
            .collect();
        assert_eq!(appels.len(), 1, "l'appel doit se voir : {appels:?}");
        // Et dans la bonne colonne.
        let appel = appels[0];
        assert!(
            (appel.first..appel.first + appel.count).contains(&1234),
            "{appel:?}"
        );
    }

    /// Un appel système passe devant un appel ordinaire.
    ///
    /// Les deux sont notables ; celui qui parle au monde extérieur l'est
    /// davantage, parce que c'est le seul endroit où un programme peut
    /// faire quelque chose d'irréversible.
    #[test]
    fn a_syscall_outranks_a_call() {
        let trace = parcours(&[StepKind::Call, StepKind::Syscall, StepKind::Plain]);
        let colonnes = columns(&trace, 1);
        assert_eq!(colonnes.len(), 1);
        assert_eq!(colonnes[0].kind, StepKind::Syscall);
    }

    /// Cliquer sur la barre désigne un pas qui existe.
    #[test]
    fn clicking_the_bar_always_lands_on_a_real_step() {
        assert_eq!(step_at(100, 0.0), 0);
        assert_eq!(step_at(100, 0.5), 50);
        // Le bord droit, c'est le dernier pas — non un pas au-delà.
        assert_eq!(step_at(100, 1.0), 99);
        // Et au-delà des bornes, rien ne déborde.
        assert_eq!(step_at(100, -1.0), 0);
        assert_eq!(step_at(100, 42.0), 99);
        // Un parcours vide ne fait rien tomber.
        assert_eq!(step_at(0, 0.5), 0);
    }

    /// Le recul se compte depuis maintenant.
    #[test]
    fn the_rewind_counts_back_from_now() {
        // Cent pas faits ; revenir au dernier, c'est reculer d'un.
        assert_eq!(rewind_to(100, 99), 1);
        // Revenir au tout premier, c'est reculer de cent.
        assert_eq!(rewind_to(100, 0), 100);
        // Et rien ne débordera jamais par le bas.
        assert_eq!(rewind_to(0, 5), 0);
    }

    /// Aucune taille ne fait tomber le regroupement.
    #[test]
    fn no_size_breaks_the_grouping() {
        for pas in [0usize, 1, 2, 99, 100, 101, 5000] {
            let trace = parcours(&vec![StepKind::Plain; pas]);
            for largeur in [0usize, 1, 7, 100, 10_000] {
                let colonnes = columns(&trace, largeur);
                let couvert: usize = colonnes.iter().map(|c| c.count).sum();
                assert!(
                    couvert == pas || colonnes.is_empty(),
                    "{pas} pas en {largeur} colonnes : {couvert} couverts"
                );
            }
        }
    }
}
