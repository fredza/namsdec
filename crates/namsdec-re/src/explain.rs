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

// ─────────────────────────────────────────────────────────────────────
// Une FONCTION, et non plus un bloc
// ─────────────────────────────────────────────────────────────────────

/// Ce qu'on sait dire d'une fonction entière, sans rien inventer.
///
/// Un bloc se résume en une phrase ; une fonction, non — elle fait
/// plusieurs choses, et les dire toutes d'un trait donnerait un
/// paragraphe qu'on ne lit pas. D'où un **portrait** : quelques
/// constats séparés, chacun certain ou absent.
///
/// Ce qui est dit se lit dans les instructions. Ce qui demanderait de
/// suivre une valeur à travers la mémoire, ou de supposer qu'un chemin
/// est pris, n'est pas dit du tout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Portrait {
    /// Les registres d'argument que la fonction LIT avant de les
    /// écrire, dans l'ordre de la convention System V.
    pub arguments: Vec<&'static str>,
    /// Les valeurs que la fonction rend, quand chaque `ret` est précédé
    /// d'une constante posée dans RAX. Triées, sans doublon.
    pub rend: Vec<u64>,
    /// Les appels système faits, nommés.
    pub syscalls: Vec<&'static str>,
    /// Combien de boucles elle contient.
    pub boucles: usize,
    /// Combien de fois elle appelle ailleurs.
    pub appels: usize,
    /// Ne fait-elle rien d'observable — que des `nop` et un retour ?
    pub vide: bool,
}

/// Les registres d'argument de la convention System V, dans l'ordre.
const ARGUMENTS: [(&str, &[&str]); 6] = [
    ("RDI", &["rdi", "edi", "di", "dil"]),
    ("RSI", &["rsi", "esi", "si", "sil"]),
    ("RDX", &["rdx", "edx", "dx", "dl"]),
    ("RCX", &["rcx", "ecx", "cx", "cl"]),
    ("R8", &["r8", "r8d", "r8w", "r8b"]),
    ("R9", &["r9", "r9d", "r9w", "r9b"]),
];

/// Le portrait d'une fonction, lu dans son graphe.
///
/// `cfg` est le graphe de la fonction seule — celui que donne
/// `subgraph` sur `function_around`. Passer le programme entier ferait
/// compter les boucles et les retours de tout le monde.
pub fn portrait(cfg: &crate::cfg::Cfg) -> Portrait {
    let mut p = Portrait {
        boucles: crate::structure::têtes_de_boucle(cfg).len(),
        ..Default::default()
    };
    if cfg.blocks.is_empty() {
        return p;
    }

    // Les arguments : lus AVANT d'être écrits, et dans le bloc d'entrée
    // seulement.
    //
    // Ailleurs, on ne saurait pas par où l'on est arrivé : un registre
    // lu au fond d'une branche peut très bien avoir été posé par le
    // chemin qui y mène. Le bloc d'entrée, lui, s'exécute toujours en
    // premier — ce qu'il lit sans l'avoir écrit vient donc de
    // l'appelant. On en rate quand un argument n'est consommé que plus
    // loin ; on n'en invente aucun.
    for (nom, formes) in ARGUMENTS {
        if lu_avant_d_être_écrit(&cfg.blocks[0].instructions, formes) {
            p.arguments.push(nom);
        }
    }

    for bloc in &cfg.blocks {
        // Les appels système : le numéro doit être posé dans le bloc
        // même, comme pour une boîte isolée.
        if let Some(position) = bloc.instructions.iter().position(|i| i.trim() == "syscall")
            && let Some(numéro) = bloc.instructions[..position]
                .iter()
                .rev()
                .find_map(|i| valeur_posée_dans_rax(i))
            && let Some(nom) = crate::table::syscall_name(numéro)
            && !p.syscalls.contains(&nom)
        {
            p.syscalls.push(nom);
        }
        p.appels += bloc
            .instructions
            .iter()
            .filter(|i| i.trim_start().starts_with("call "))
            .count();
    }

    // Ce qu'elle rend : chaque `ret` doit être précédé, dans SON bloc,
    // d'une constante posée dans RAX que rien n'écrase ensuite.
    //
    // Un seul `ret` qui ne dit pas d'où vient sa valeur suffit à se
    // taire : « rend 0 ou 1 » serait faux s'il existe un troisième
    // chemin qui rend autre chose.
    let retours: Vec<&crate::cfg::Block> = cfg
        .blocks
        .iter()
        .filter(|b| b.exit == Exit::Return)
        .collect();
    if !retours.is_empty() {
        let mut valeurs = Vec::new();
        let mut toutes_connues = true;
        for bloc in &retours {
            match constante_rendue(&bloc.instructions) {
                Some(valeur) => {
                    if !valeurs.contains(&valeur) {
                        valeurs.push(valeur);
                    }
                }
                None => toutes_connues = false,
            }
        }
        if toutes_connues {
            valeurs.sort_unstable();
            p.rend = valeurs;
        }
    }

    p.vide = cfg
        .blocks
        .iter()
        .all(|b| b.instructions.iter().all(|i| est_sans_effet(i)));
    p
}

/// Ce registre est-il lu avant d'être écrit ?
///
/// Écrit veut dire : premier opérande d'une instruction qui affecte, et
/// c'est le seul cas qu'on traite — un `xchg`, un `mul` implicite ou une
/// écriture par la mémoire ne comptent pas comme une écriture connue, et
/// l'on préfère alors ne rien conclure.
fn lu_avant_d_être_écrit(instructions: &[String], formes: &[&str]) -> bool {
    for instruction in instructions {
        let texte = instruction.trim().to_ascii_lowercase();
        let Some((mnémonique, reste)) = texte.split_once(' ') else {
            continue;
        };
        let opérandes: Vec<&str> = reste.split(',').map(str::trim).collect();
        // La source d'abord : `mov rdi, rdi` lit avant d'écrire.
        if opérandes
            .iter()
            .skip(1)
            .any(|o| formes.iter().any(|f| mentionne(o, f)))
        {
            return true;
        }
        let premier = opérandes.first().copied().unwrap_or_default();
        if !formes.iter().any(|f| mentionne(premier, f)) {
            continue;
        }
        // Le premier opérande : écrit par `mov` et `lea`, lu ET écrit
        // par tout le reste (`add rdi, 8` lit RDI), lu seulement par les
        // instructions qui ne rendent rien.
        return !matches!(
            mnémonique,
            "mov" | "movzx" | "movsx" | "movsxd" | "lea" | "xor"
        );
    }
    false
}

/// Ce texte d'opérande parle-t-il de ce registre ?
///
/// Un test de sous-chaîne prendrait `rdi` dans `[rdi+8]`, ce qui est
/// juste, mais aussi dans un nom de symbole. On borne donc aux
/// caractères qui ne peuvent pas faire partie d'un identifiant.
fn mentionne(opérande: &str, registre: &str) -> bool {
    let mut reste = opérande;
    while let Some(position) = reste.find(registre) {
        let avant = reste[..position].chars().next_back();
        let après = reste[position + registre.len()..].chars().next();
        let bord = |c: Option<char>| !c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if bord(avant) && bord(après) {
            return true;
        }
        reste = &reste[position + registre.len()..];
    }
    false
}

/// La constante que ce bloc laisse dans RAX avant de rendre la main.
fn constante_rendue(instructions: &[String]) -> Option<u64> {
    let mut valeur = None;
    for instruction in instructions {
        let texte = instruction.trim().to_ascii_lowercase();
        if let Some(posée) = valeur_posée_dans_rax(&texte) {
            valeur = Some(posée);
            continue;
        }
        // `xor eax, eax` met RAX à zéro, et c'est la façon dont tout le
        // monde écrit « rend 0 ».
        if texte == "xor eax,eax" || texte == "xor rax,rax" || texte == "xor eax, eax" {
            valeur = Some(0);
            continue;
        }
        // Toute autre écriture de RAX : on ne sait plus.
        if écrit_rax(&texte) {
            valeur = None;
        }
    }
    valeur
}

/// Cette instruction écrit-elle dans RAX autrement qu'en y posant une
/// constante ?
fn écrit_rax(texte: &str) -> bool {
    let Some((mnémonique, reste)) = texte.split_once(' ') else {
        // `cqo`, `cdq` touchent RDX ; `ret`, `nop`, rien.
        return false;
    };
    if matches!(mnémonique, "cmp" | "test" | "push" | "jmp" | "call") {
        return false;
    }
    let premier = reste.split(',').next().unwrap_or_default().trim();
    matches!(premier, "rax" | "eax" | "ax" | "al" | "ah")
}

/// Une instruction qui ne change rien d'observable.
fn est_sans_effet(instruction: &str) -> bool {
    let texte = instruction.trim();
    texte == "nop" || texte == "ret" || texte == "leave" || texte.starts_with("nop ")
}

/// Une énumération qui se lit : « A, B et C », et non « A, B, C ».
///
/// Le dernier séparateur fait la différence entre une phrase et une
/// liste de courses, et ce module écrit des phrases.
fn énumère(éléments: &[&str], fr: bool) -> String {
    match éléments {
        [] => String::new(),
        [seul] => (*seul).to_owned(),
        [début @ .., dernier] => {
            let et = if fr { " et " } else { " and " };
            format!("{}{et}{dernier}", début.join(", "))
        }
    }
}

/// Le portrait, dit en une phrase — ou rien du tout.
///
/// Rien du tout quand il n'y a que du banal à dire : « contient une
/// boucle » pour une fonction qui en a une et rien d'autre de
/// remarquable n'apprend à personne ce que le graphe montre déjà.
pub fn phrase_de_fonction(p: &Portrait, fr: bool) -> Option<String> {
    if p.vide {
        return Some(if fr {
            "Cette fonction ne fait rien.".to_owned()
        } else {
            "This function does nothing.".to_owned()
        });
    }
    let mut morceaux: Vec<String> = Vec::new();
    if !p.arguments.is_empty() {
        let liste = énumère(&p.arguments, fr);
        morceaux.push(if fr {
            format!("lit {liste}")
        } else {
            format!("reads {liste}")
        });
    }
    if p.boucles > 0 {
        morceaux.push(match (p.boucles, fr) {
            (1, true) => "contient une boucle".to_owned(),
            (1, false) => "contains one loop".to_owned(),
            (n, true) => format!("contient {n} boucles"),
            (n, false) => format!("contains {n} loops"),
        });
    }
    if !p.syscalls.is_empty() {
        let liste = énumère(&p.syscalls, fr);
        morceaux.push(if fr {
            format!("demande au noyau : {liste}")
        } else {
            format!("asks the kernel: {liste}")
        });
    }
    if p.appels > 0 {
        morceaux.push(match (p.appels, fr) {
            (1, true) => "appelle une autre fonction".to_owned(),
            (1, false) => "calls another function".to_owned(),
            (n, true) => format!("appelle {n} fois ailleurs"),
            (n, false) => format!("calls out {n} times"),
        });
    }
    match p.rend.len() {
        0 => {}
        1 => morceaux.push(if fr {
            format!("rend toujours {}", p.rend[0])
        } else {
            format!("always returns {}", p.rend[0])
        }),
        _ => {
            let liste: Vec<String> = p.rend.iter().map(|v| v.to_string()).collect();
            let liste = liste.join(if fr { " ou " } else { " or " });
            morceaux.push(if fr {
                format!("rend {liste}")
            } else {
                format!("returns {liste}")
            });
        }
    }
    // Se taire quand le seul constat est celui que le graphe montre
    // déjà : « contient une boucle » devant un dessin où la boucle se
    // voit n'apprend rien à personne. Un appel système ou des arguments
    // lus, en revanche, ne se lisent nulle part ailleurs — un seul
    // suffit alors à parler.
    let banal = p.arguments.is_empty() && p.syscalls.is_empty() && p.rend.is_empty();
    if banal {
        return None;
    }
    let mut phrase = morceaux.join(", ");
    phrase.push('.');
    let mut lettres = phrase.chars();
    Some(match lettres.next() {
        Some(première) => première.to_uppercase().collect::<String>() + lettres.as_str(),
        None => phrase,
    })
}

#[cfg(test)]
mod tests_fonction {
    use super::*;

    fn portrait_de(bytes: &[u8]) -> Portrait {
        portrait(&crate::cfg::build(&namsdec_disasm::disassemble(
            bytes, 0x401000,
        )))
    }

    fn phrase_de(bytes: &[u8]) -> Option<String> {
        phrase_de_fonction(&portrait_de(bytes), true)
    }

    /// Les arguments qu'une fonction lit se voient.
    ///
    /// C'est la première question qu'on se pose devant une fonction
    /// sans symboles : combien en prend-elle ? La convention System V
    /// répond — RDI, RSI, RDX, RCX, R8, R9 —, à condition de ne
    /// compter que ceux qui sont LUS avant d'être écrits.
    #[test]
    fn the_arguments_a_function_reads_are_visible() {
        // cmp rdi, rsi / jne / mov eax,1 / ret / xor eax,eax / ret
        let deux = portrait_de(&[
            0x48, 0x39, 0xF7, 0x75, 0x07, 0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3, 0x31, 0xC0, 0xC3,
        ]);
        assert_eq!(deux.arguments, vec!["RDI", "RSI"]);
        // Et une fonction qui ÉCRIT RDI avant de s'en servir n'en prend
        // pas : `mov rdi, 1` puis `syscall` est un appelant, non une
        // fonction à un argument.
        let aucun = portrait_de(&[0xBF, 0x01, 0x00, 0x00, 0x00, 0x48, 0x01, 0xFF, 0xC3]);
        assert!(aucun.arguments.is_empty(), "{aucun:?}");
    }

    /// Ce qu'une fonction rend n'est dit que si TOUS ses retours le
    /// disent.
    ///
    /// « Rend 0 ou 1 » est faux dès qu'il existe un troisième chemin
    /// dont on ne sait pas ce qu'il laisse dans RAX. Un seul `ret`
    /// muet suffit donc à se taire sur la valeur.
    #[test]
    fn a_return_value_is_said_only_when_every_path_says_it() {
        let deux = portrait_de(&[
            0x48, 0x39, 0xF7, 0x75, 0x07, 0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3, 0x31, 0xC0, 0xC3,
        ]);
        assert_eq!(deux.rend, vec![0, 1]);

        // Le cas qui compte : UN chemin pose une constante, l'AUTRE non.
        // Dire « rend 1 » serait faux, et c'est le genre de phrase qu'on
        // croit sur parole devant un binaire qu'on ne connaît pas.
        //
        //   cmp rdi, rsi / jne autre / mov eax,1 / ret
        //   autre: add rax, rdi / ret
        let mélangé = portrait_de(&[
            0x48, 0x39, 0xF7, // cmp rdi, rsi
            0x75, 0x07, // jne +7
            0xB8, 0x01, 0x00, 0x00, 0x00, // mov eax, 1
            0xC3, // ret
            0x48, 0x01, 0xF8, // add rax, rdi
            0xC3, // ret
        ]);
        assert!(
            mélangé.rend.is_empty(),
            "un seul retour muet suffit à se taire : {mélangé:?}"
        );

        // add rax, rdi / ret : la valeur rendue dépend de l'argument.
        let inconnu = portrait_de(&[0x48, 0x01, 0xF8, 0xC3]);
        assert!(inconnu.rend.is_empty(), "{inconnu:?}");
        assert!(
            !phrase_de(&[0x48, 0x01, 0xF8, 0xC3])
                .unwrap_or_default()
                .contains("rend"),
            "rien ne doit être dit de la valeur rendue"
        );
    }

    /// Une fonction vide le dit, et c'est utile.
    ///
    /// Une fonction qui ne fait rien est presque toujours un reste :
    /// une vérification retirée, un correctif déjà posé. C'est ce qu'on
    /// cherche dans un binaire modifié.
    #[test]
    fn a_function_that_does_nothing_says_so() {
        let phrase = phrase_de(&[0x90, 0x90, 0xC3]).expect("une phrase");
        assert!(phrase.contains("ne fait rien"), "{phrase}");
    }

    /// Ce que le graphe montre déjà n'est pas répété.
    ///
    /// « Contient une boucle » devant un dessin où la boucle se voit
    /// n'apprend rien. Le portrait ne parle que de ce qui ne se lit pas
    /// ailleurs : les arguments, les appels système, la valeur rendue.
    #[test]
    fn what_the_graph_already_shows_is_not_repeated() {
        // mov ecx,3 / dec ecx / jnz / ret : une boucle, et rien d'autre
        // à en dire.
        let seule = &[0xB9, 0x03, 0x00, 0x00, 0x00, 0xFF, 0xC9, 0x75, 0xFC, 0xC3][..];
        assert_eq!(portrait_de(seule).boucles, 1);
        assert_eq!(phrase_de(seule), None, "il n'y a rien à ajouter au dessin");
    }

    /// Un appel système se dit, même seul.
    ///
    /// Il ne se lit nulle part ailleurs : le graphe montre un bloc, non
    /// ce que ce bloc demande au noyau.
    #[test]
    fn a_system_call_is_worth_saying_on_its_own() {
        // mov eax,1 / mov edi,1 / syscall / ret
        let phrase = phrase_de(&[
            0xB8, 0x01, 0x00, 0x00, 0x00, 0xBF, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x05, 0xC3,
        ])
        .expect("une phrase");
        assert!(phrase.contains("write"), "{phrase}");
    }

    /// L'énumération se lit comme une phrase.
    #[test]
    fn a_list_reads_like_a_sentence() {
        assert_eq!(énumère(&["RDI", "RSI"], true), "RDI et RSI");
        assert_eq!(énumère(&["A", "B", "C"], false), "A, B and C");
        assert_eq!(énumère(&["seul"], true), "seul");
        assert_eq!(énumère(&[], true), "");
    }

    /// Les deux langues disent la même chose.
    #[test]
    fn both_languages_draw_the_same_portrait() {
        let p = portrait_de(&[
            0x48, 0x39, 0xF7, 0x75, 0x07, 0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3, 0x31, 0xC0, 0xC3,
        ]);
        let fr = phrase_de_fonction(&p, true).expect("français");
        let en = phrase_de_fonction(&p, false).expect("anglais");
        assert!(fr.contains("Lit RDI et RSI"), "{fr}");
        assert!(en.contains("Reads RDI and RSI"), "{en}");
        assert_ne!(fr, en, "la traduction ne doit pas être oubliée");
    }

    /// Un graphe vide ne fait rien tomber.
    #[test]
    fn an_empty_graph_draws_no_portrait() {
        let p = portrait(&crate::cfg::Cfg::default());
        assert_eq!(p, Portrait::default());
    }
}
