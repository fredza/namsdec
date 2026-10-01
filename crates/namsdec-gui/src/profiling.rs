//! Le diagnostic des images lentes : mesurer chaque étape d'une frame, et
//! écrire sur la sortie d'erreur celle qui a gelé l'interface.
//!
//! Un gel intermittent ne se reproduit pas sous un profileur ; il faut
//! donc qu'il laisse une trace de lui-même quand il passe. Sorti de
//! `main.rs` pour l'alléger — c'est un sujet à part.

use super::*;

/// Au-delà de ce seuil, une frame n'est plus une saccade : la fenêtre a
/// cessé de répondre assez longtemps pour que l'utilisateur le voie, et
/// le compositeur finit par proposer de forcer l'application à quitter.
///
/// Un gel ne laisse aucune trace une fois passé — surtout un gel
/// intermittent, qu'on ne peut pas provoquer à la demande sous un
/// profileur. La frame lente s'écrit donc sur la sortie d'erreur avec le
/// détail de ce qui l'a occupée, pour que le prochain gel désigne son
/// coupable au lieu d'être à reproduire.
const SLOW_FRAME: Duration = Duration::from_millis(120);

/// Mesure une étape de la frame et l'ajoute au relevé.
pub(crate) fn timed<R>(
    spans: &mut Vec<(&'static str, Duration)>,
    name: &'static str,
    body: impl FnOnce() -> R,
) -> R {
    let start = Instant::now();
    let out = body();
    spans.push((name, start.elapsed()));
    out
}

/// Ce qu'il y a à dire d'une frame, s'il y a quelque chose à dire.
///
/// Rien tant qu'elle tient dans son budget : en marche normale cette
/// instrumentation est silencieuse.
///
/// `texte_changé` distingue les deux sortes de lenteur, et la distinction
/// n'est pas cosmétique. Ouvrir un binaire de soixante mille lignes
/// demande une demi-seconde de mise en page, **une fois** : le cache
/// porte ensuite, et les frames suivantes tiennent en deux
/// millisecondes. Crier « frame lente » sans le dire laissait croire à
/// un coût permanent, et l'on cherchait une fuite là où il n'y avait
/// qu'un chargement.
pub(crate) fn slow_frame_note(
    total: Duration,
    spans: &[(&'static str, Duration)],
    texte_changé: bool,
) -> Option<String> {
    if total < SLOW_FRAME {
        return None;
    }
    let detail: Vec<String> = spans
        .iter()
        .filter(|(_, d)| *d >= Duration::from_millis(1))
        .map(|(name, d)| format!("{name} {}ms", d.as_millis()))
        .collect();
    let detail = if detail.is_empty() {
        "hors des étapes mesurées".to_owned()
    } else {
        detail.join(", ")
    };
    Some(if texte_changé {
        format!(
            "namsdec: mise en page — {}ms ({detail}). Le texte vient de changer : \
             c'est payé une fois, les frames suivantes n'en gardent rien.",
            total.as_millis()
        )
    } else {
        format!(
            "namsdec: frame lente — {}ms au total ({detail})",
            total.as_millis()
        )
    })
}

/// Écrit sur la sortie d'erreur ce que `slow_frame_note` a trouvé à dire.
pub(crate) fn report_slow_frame(
    total: Duration,
    spans: &[(&'static str, Duration)],
    texte_changé: bool,
) {
    if let Some(note) = slow_frame_note(total, spans, texte_changé) {
        eprintln!("{note}");
    }
}
