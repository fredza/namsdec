//! Ce qu'on peut dire d'un bloc **sans rien inventer**.
//!
//! namsdec explique déjà chaque instruction. Il n'expliquait pas un
//! ENSEMBLE : ce que fait une boucle, ce qu'une suite de `cmp`/`je`
//! vérifie au total. C'est ce qui le distingue le plus des autres
//! outils, et c'est aussi le plus difficile à faire honnêtement.
//!
//! La règle est donc une seule, et elle commande tout le module : **une
//! explication fausse est pire qu'une absence d'explication**. Chaque
//! reconnaisseur ci-dessous ne répond que s'il est certain, et rend
//! `None` au moindre doute. Rien n'est deviné, rien n'est approché : ce
//! qui est dit se lit dans les instructions du bloc ou dans ce que
//! l'exécution a compté.
//!
//! C'est pour cela que le module se tait souvent. Un bloc sans phrase
//! n'est pas un échec — c'est un bloc dont on ne sait rien dire de plus
//! que ce que ses instructions disent déjà.

use crate::cfg::Exit;

/// Ce qu'on sait d'un bloc au moment d'en parler.
pub struct Bloc<'a> {
    /// Le texte de chaque instruction, dans l'ordre, tel que le
    /// désassembleur l'écrit.
    pub instructions: &'a [String],
    pub exit: Exit,
    /// Le bloc se reboucle-t-il — vers lui-même ou vers plus haut ?
    pub boucle: bool,
    /// Combien de fois il a tourné, si le programme a tourné. `None`
    /// quand rien n'a encore été exécuté : on ne dit alors rien du
    /// nombre, plutôt que d'écrire « zéro fois » à un programme qui
    /// n'a pas commencé.
    pub passages: Option<u64>,
}

/// La phrase qui décrit ce bloc, s'il y en a une.
///
/// L'ordre des reconnaisseurs va du plus précis au plus général : un
/// bloc qui ne fait que des `nop` est d'abord un bloc qui ne fait rien,
/// avant d'être une boucle ou quoi que ce soit d'autre.
pub fn phrase(bloc: &Bloc, fr: bool) -> Option<String> {
    let tours = tours(bloc, fr);
    if bloc.instructions.is_empty() {
        return None;
    }
    if bloc.instructions.iter().all(|i| i.trim() == "nop") {
        return Some(join(
            if fr { "ne fait rien" } else { "does nothing" },
            tours,
        ));
    }
    if let Some(quoi) = appel_système(bloc, fr) {
        return Some(join(&quoi, tours));
    }
    if let Some(quoi) = comparaison(bloc, fr) {
        return Some(join(&quoi, tours));
    }
    if bloc.boucle {
        return Some(join(if fr { "boucle" } else { "loops" }, tours));
    }
    if bloc.exit == Exit::Return {
        return Some(join(
            if fr {
                "rend la main à l'appelant"
            } else {
                "hands control back to the caller"
            },
            tours,
        ));
    }
    if let Some(quoi) = préparation(bloc, fr) {
        return Some(join(&quoi, tours));
    }
    None
}

/// « , et a tourné N fois » — ou rien.
///
/// Rien quand le programme n'a pas tourné, et rien pour un seul
/// passage : « a tourné une fois » n'apprend rien à qui regarde un
/// programme qui vient de s'exécuter une fois.
fn tours(bloc: &Bloc, fr: bool) -> Option<String> {
    match bloc.passages {
        Some(n) if n > 1 && fr => Some(format!("a tourné {n} fois")),
        Some(n) if n > 1 => Some(format!("ran {n} times")),
        _ => None,
    }
}

fn join(quoi: &str, tours: Option<String>) -> String {
    match tours {
        Some(t) => format!("{quoi}, {t}"),
        None => quoi.to_owned(),
    }
}

/// Un `syscall` dont le numéro est écrit DANS CE BLOC.
///
/// Dans ce bloc et nulle part ailleurs : un `mov eax, 1` posé trois
/// blocs plus haut peut très bien avoir été écrasé par un chemin qu'on
/// ne voit pas d'ici. C'est exactement le genre de raccourci qui
/// produirait une phrase fausse.
fn appel_système(bloc: &Bloc, fr: bool) -> Option<String> {
    let position = bloc
        .instructions
        .iter()
        .position(|i| i.trim() == "syscall")?;
    let numéro = bloc.instructions[..position]
        .iter()
        .rev()
        .find_map(|i| valeur_posée_dans_rax(i))?;
    let nom = crate::table::syscall_name(numéro)?;
    Some(if fr {
        format!("demande au noyau : {nom}")
    } else {
        format!("asks the kernel: {nom}")
    })
}

/// `mov eax, 1` ou `mov rax, 1` — et la valeur.
fn valeur_posée_dans_rax(texte: &str) -> Option<u64> {
    let (mnémonique, reste) = texte.trim().split_once(' ')?;
    if mnémonique != "mov" {
        return None;
    }
    let (cible, valeur) = reste.split_once(',')?;
    if !matches!(cible.trim().to_ascii_lowercase().as_str(), "eax" | "rax") {
        return None;
    }
    nombre(valeur.trim())
}

/// Un nombre tel que le désassembleur l'écrit : `1`, `3Ch`, `0x3c`.
fn nombre(texte: &str) -> Option<u64> {
    let texte = texte.trim();
    if let Some(hexa) = texte.strip_prefix("0x") {
        return u64::from_str_radix(hexa, 16).ok();
    }
    if let Some(hexa) = texte.strip_suffix('h').or_else(|| texte.strip_suffix('H')) {
        return u64::from_str_radix(hexa, 16).ok();
    }
    texte.parse().ok()
}

/// `cmp a, b` suivi d'un saut conditionnel : ce que le bloc vérifie.
///
/// Les deux doivent se suivre — une instruction entre les deux peut
/// changer les indicateurs, et l'on ne saurait plus de quoi le saut
/// parle.
fn comparaison(bloc: &Bloc, fr: bool) -> Option<String> {
    if bloc.exit != Exit::Branch || bloc.instructions.len() < 2 {
        return None;
    }
    let saut = bloc.instructions.last()?.trim();
    let avant = bloc.instructions[bloc.instructions.len() - 2].trim();
    let (mnémonique, opérandes) = avant.split_once(' ')?;
    if !matches!(mnémonique, "cmp" | "test") {
        return None;
    }
    let (gauche, droite) = opérandes.split_once(',')?;
    let condition = condition(saut.split_whitespace().next()?, fr)?;
    let (gauche, droite) = (gauche.trim().to_uppercase(), droite.trim().to_uppercase());
    Some(if fr {
        format!("compare {gauche} et {droite}, et part {condition}")
    } else {
        format!("compares {gauche} and {droite}, and goes {condition}")
    })
}

/// Ce que dit un saut conditionnel, en toutes lettres.
///
/// Seulement ceux dont le sens ne dépend pas du signe supposé des
/// opérandes : `jg` et `ja` veulent dire « plus grand », l'un pour des
/// nombres signés et l'autre non, et confondre les deux est le genre
/// d'erreur qu'un débutant met une heure à trouver. On les nomme donc
/// séparément, ou l'on se tait.
fn condition(mnémonique: &str, fr: bool) -> Option<&'static str> {
    Some(match (mnémonique, fr) {
        ("je" | "jz", true) => "si c'est égal",
        ("je" | "jz", false) => "if equal",
        ("jne" | "jnz", true) => "si c'est différent",
        ("jne" | "jnz", false) => "if different",
        ("jg" | "jnle", true) => "si c'est plus grand (signé)",
        ("jg" | "jnle", false) => "if greater (signed)",
        ("jl" | "jnge", true) => "si c'est plus petit (signé)",
        ("jl" | "jnge", false) => "if less (signed)",
        ("ja" | "jnbe", true) => "si c'est plus grand (non signé)",
        ("ja" | "jnbe", false) => "if above (unsigned)",
        ("jb" | "jnae", true) => "si c'est plus petit (non signé)",
        ("jb" | "jnae", false) => "if below (unsigned)",
        _ => return None,
    })
}

/// Un bloc qui ne fait que poser des constantes dans des registres.
///
/// C'est la préparation d'un appel, et la reconnaître évite de lire
/// quatre `mov` pour comprendre qu'il ne s'y passe rien d'autre.
fn préparation(bloc: &Bloc, fr: bool) -> Option<String> {
    let mut posées = Vec::new();
    for instruction in bloc.instructions {
        let (mnémonique, reste) = instruction.trim().split_once(' ')?;
        if mnémonique != "mov" {
            return None;
        }
        let (cible, valeur) = reste.split_once(',')?;
        let valeur = nombre(valeur.trim())?;
        posées.push(format!("{} = {valeur}", cible.trim().to_uppercase()));
    }
    if posées.is_empty() {
        return None;
    }
    Some(if fr {
        format!("prépare {}", posées.join(", "))
    } else {
        format!("sets up {}", posées.join(", "))
    })
}

/// Un fait d'exécution : quelque chose que le programme A FAIT, compté
/// et non déduit.
///
/// « Ce que le programme a fait à la place » est la moitié qui manquait
/// aux échecs d'exercice. Elle demande le profil, et elle demande
/// surtout de ne rien dire qu'on n'ait compté : « la boucle s'est
/// arrêtée trop tôt » est une interprétation, « la boucle a tourné
/// quatre fois » est une mesure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fait {
    /// Une boucle, et combien de tours elle a faits.
    Boucle {
        adresse: u64,
        tours: u64,
    },
    /// Un saut conditionnel dont une seule des deux suites a été
    /// atteinte — et l'on sait laquelle, parce qu'elle n'a pas d'autre
    /// entrée que celle-là.
    SautToujoursPris {
        adresse: u64,
    },
    SautJamaisPris {
        adresse: u64,
    },
}

/// Ce que l'exécution a fait de ce graphe.
///
/// Rien n'est rendu tant que rien n'a tourné : un programme qui n'a pas
/// commencé n'a pas de branche à raconter.
///
/// LA PRÉCAUTION QUI COMPTE : on ne conclut sur une branche que si le
/// bloc visé n'a **pas d'autre entrée**. Sinon son compteur mélange les
/// passages venus d'ailleurs, et dire « ce saut a toujours été pris »
/// serait faux exactement dans les cas intéressants — ceux où plusieurs
/// chemins se rejoignent.
pub fn ce_qui_a_tourné(
    cfg: &crate::cfg::Cfg,
    passages: &std::collections::BTreeMap<u64, u64>,
) -> Vec<Fait> {
    use crate::cfg::Kind;
    let mut faits = Vec::new();
    if passages.is_empty() {
        return faits;
    }
    let tours =
        |index: usize| -> u64 { passages.get(&cfg.blocks[index].start).copied().unwrap_or(0) };
    let entrées = |index: usize| -> usize { cfg.edges.iter().filter(|a| a.to == index).count() };
    for (index, bloc) in cfg.blocks.iter().enumerate() {
        // Une boucle : une arête d'ici vers soi-même ou vers plus haut.
        if cfg
            .edges
            .iter()
            .any(|a| a.from == index && cfg.blocks[a.to].start <= bloc.start)
            && tours(index) > 1
        {
            faits.push(Fait::Boucle {
                adresse: bloc.start,
                tours: tours(index),
            });
            continue;
        }
        if bloc.exit != crate::cfg::Exit::Branch {
            continue;
        }
        let pris = cfg
            .edges
            .iter()
            .find(|a| a.from == index && a.kind == Kind::Taken)
            .map(|a| a.to);
        let pas_pris = cfg
            .edges
            .iter()
            .find(|a| a.from == index && a.kind == Kind::NotTaken)
            .map(|a| a.to);
        let (Some(pris), Some(pas_pris)) = (pris, pas_pris) else {
            continue;
        };
        // Le bloc lui-même doit avoir tourné : sans cela on parlerait
        // d'un saut que le programme n'a jamais rencontré.
        if tours(index) == 0 || entrées(pris) != 1 || entrées(pas_pris) != 1 {
            continue;
        }
        match (tours(pris), tours(pas_pris)) {
            (p, 0) if p > 0 => faits.push(Fait::SautToujoursPris { adresse: bloc.last }),
            (0, np) if np > 0 => faits.push(Fait::SautJamaisPris { adresse: bloc.last }),
            _ => {}
        }
    }
    faits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bloc(instructions: &[&str], exit: Exit, boucle: bool, passages: Option<u64>) -> Vec<String> {
        let _ = (exit, boucle, passages);
        instructions.iter().map(|i| (*i).to_owned()).collect()
    }

    fn dit(
        instructions: &[&str],
        exit: Exit,
        boucle: bool,
        passages: Option<u64>,
    ) -> Option<String> {
        let textes = bloc(instructions, exit, boucle, passages);
        phrase(
            &Bloc {
                instructions: &textes,
                exit,
                boucle,
                passages,
            },
            true,
        )
    }

    #[test]
    fn un_bloc_de_nop_ne_fait_rien() {
        assert_eq!(
            dit(&["nop", "nop"], Exit::Fallthrough, false, None).as_deref(),
            Some("ne fait rien")
        );
    }

    #[test]
    fn un_appel_système_se_nomme_quand_son_numéro_est_là() {
        assert_eq!(
            dit(&["mov eax,1", "syscall"], Exit::Fallthrough, false, None).as_deref(),
            Some("demande au noyau : write")
        );
        // En hexadécimal, comme le désassembleur l'écrit.
        assert_eq!(
            dit(&["mov eax,3Ch", "syscall"], Exit::Stop, false, None).as_deref(),
            Some("demande au noyau : exit")
        );
    }

    #[test]
    fn un_appel_système_sans_numéro_ici_ne_se_nomme_pas() {
        // Le numéro vient d'ailleurs : un `mov` posé trois blocs plus
        // haut peut avoir été écrasé par un chemin qu'on ne voit pas.
        // On se tait plutôt que de deviner.
        assert_eq!(dit(&["syscall"], Exit::Stop, false, None), None);
        assert_eq!(
            dit(&["mov rbx,1", "syscall"], Exit::Stop, false, None),
            None
        );
    }

    #[test]
    fn une_comparaison_se_dit_en_toutes_lettres() {
        assert_eq!(
            dit(
                &["cmp rax,rbx", "jne short 400021h"],
                Exit::Branch,
                false,
                None
            )
            .as_deref(),
            Some("compare RAX et RBX, et part si c'est différent")
        );
    }

    #[test]
    fn une_comparaison_interrompue_ne_se_dit_pas() {
        // Une instruction entre les deux peut changer les indicateurs :
        // on ne sait plus de quoi le saut parle.
        assert_eq!(
            dit(
                &["cmp rax,rbx", "add rcx,1", "jne short 400021h"],
                Exit::Branch,
                false,
                None
            ),
            None
        );
    }

    #[test]
    fn un_saut_dont_le_signe_est_ambigu_ne_se_dit_pas_à_moitié() {
        // `jg` et `ja` veulent tous deux dire « plus grand », l'un signé
        // et l'autre non. Les nommer séparément, ou se taire.
        let signé = dit(&["cmp rax,rbx", "jg short 1h"], Exit::Branch, false, None);
        let non_signé = dit(&["cmp rax,rbx", "ja short 1h"], Exit::Branch, false, None);
        assert!(signé.as_deref().unwrap().contains("(signé)"));
        assert!(non_signé.as_deref().unwrap().contains("(non signé)"));
        assert_ne!(signé, non_signé);
    }

    #[test]
    fn une_boucle_dit_combien_de_fois_elle_a_tourné() {
        assert_eq!(
            dit(&["dec rcx", "jnz short 1h"], Exit::Branch, true, Some(20)).as_deref(),
            Some("boucle, a tourné 20 fois")
        );
        // Un seul passage n'apprend rien.
        assert_eq!(
            dit(&["dec rcx", "jnz short 1h"], Exit::Branch, true, Some(1)).as_deref(),
            Some("boucle")
        );
        // Et rien n'a tourné : on ne dit pas « zéro fois » à un
        // programme qui n'a pas commencé.
        assert_eq!(
            dit(&["dec rcx", "jnz short 1h"], Exit::Branch, true, None).as_deref(),
            Some("boucle")
        );
    }

    #[test]
    fn une_préparation_se_résume() {
        assert_eq!(
            dit(&["mov rdi,1", "mov rdx,8"], Exit::Fallthrough, false, None).as_deref(),
            Some("prépare RDI = 1, RDX = 8")
        );
    }

    #[test]
    fn ce_quon_ne_sait_pas_dire_ne_se_dit_pas() {
        // Le cas qui compte le plus : trois instructions ordinaires, et
        // le module se tait. Un bloc sans phrase n'est pas un échec.
        assert_eq!(
            dit(
                &["add rax,rcx", "xor rdx,rdx", "shl rax,2"],
                Exit::Fallthrough,
                false,
                None
            ),
            None
        );
        assert_eq!(dit(&[], Exit::Fallthrough, false, None), None);
    }

    use crate::cfg::{Block, Cfg, Edge, Kind};
    use std::collections::BTreeMap;

    fn b(start: u64, last: u64, exit: Exit) -> Block {
        Block {
            start,
            last,
            instructions: vec!["nop".to_owned()],
            exit,
        }
    }

    /// Un test simple : un bloc qui se reboucle, et un qui branche vers
    /// deux suites dont chacune n'a que lui pour entrée.
    fn graphe() -> Cfg {
        Cfg {
            blocks: vec![
                b(0x10, 0x14, Exit::Branch),
                b(0x20, 0x24, Exit::Branch),
                b(0x30, 0x34, Exit::Return),
                b(0x40, 0x44, Exit::Return),
            ],
            edges: vec![
                // Le bloc 0 se reboucle sur lui-même, et tombe dans 1.
                Edge {
                    from: 0,
                    to: 0,
                    kind: Kind::Taken,
                },
                Edge {
                    from: 0,
                    to: 1,
                    kind: Kind::NotTaken,
                },
                // Le bloc 1 branche vers 2 ou 3, qui n'ont que lui.
                Edge {
                    from: 1,
                    to: 2,
                    kind: Kind::Taken,
                },
                Edge {
                    from: 1,
                    to: 3,
                    kind: Kind::NotTaken,
                },
            ],
        }
    }

    #[test]
    fn rien_na_tourné_ne_raconte_rien() {
        assert!(ce_qui_a_tourné(&graphe(), &BTreeMap::new()).is_empty());
    }

    #[test]
    fn une_boucle_dit_ses_tours_et_une_branche_dit_son_côté() {
        let passages: BTreeMap<u64, u64> = [(0x10, 5), (0x20, 1), (0x30, 1)].into_iter().collect();
        let faits = ce_qui_a_tourné(&graphe(), &passages);
        assert!(faits.contains(&Fait::Boucle {
            adresse: 0x10,
            tours: 5
        }));
        assert!(faits.contains(&Fait::SautToujoursPris { adresse: 0x24 }));
    }

    #[test]
    fn un_bloc_à_plusieurs_entrées_ne_dit_rien_de_sa_branche() {
        // LA PRÉCAUTION QUI COMPTE : le compteur d'un bloc où plusieurs
        // chemins se rejoignent mélange les passages venus d'ailleurs.
        // Dire « ce saut a toujours été pris » serait faux exactement
        // dans les cas intéressants.
        let mut cfg = graphe();
        cfg.edges.push(Edge {
            from: 0,
            to: 2,
            kind: Kind::Next,
        });
        let passages: BTreeMap<u64, u64> = [(0x10, 5), (0x20, 1), (0x30, 1)].into_iter().collect();
        let faits = ce_qui_a_tourné(&cfg, &passages);
        assert!(
            !faits
                .iter()
                .any(|f| matches!(f, Fait::SautToujoursPris { .. })),
            "{faits:?}"
        );
        // La boucle, elle, se compte toujours : son propre compteur ne
        // dépend de personne.
        assert!(faits.iter().any(|f| matches!(f, Fait::Boucle { .. })));
    }

    #[test]
    fn un_saut_jamais_rencontré_ne_dit_rien() {
        // Le bloc du saut n'a pas tourné : parler de sa branche serait
        // parler d'un saut que le programme n'a jamais vu.
        let passages: BTreeMap<u64, u64> = [(0x10, 5)].into_iter().collect();
        let faits = ce_qui_a_tourné(&graphe(), &passages);
        assert!(
            faits.iter().all(|f| matches!(f, Fait::Boucle { .. })),
            "{faits:?}"
        );
    }
}
