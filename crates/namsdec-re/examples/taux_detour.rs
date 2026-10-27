//! Mesurer le TAUX DE REFUS du détour sur des binaires réels.
//!
//! La question que ce banc tranche, et qu'aucune lecture du code ne
//! tranchait : `plan_detour` refuse les instructions qui dépendent de
//! leur position quand il ne sait pas les réécrire, et
//! `mov rax, [rip+0x2e4f]` est la forme NORMALE d'un accès à une
//! globale en code compilé. La crainte était que le détour refuse la
//! plupart des endroits réels sans que personne ne le sache.
//!
//! Un chiffre le dit. On prend chaque adresse d'instruction de chaque
//! section exécutable, on y planifie un détour, et on compte les
//! réussites contre les refus, par motif.
//!
//! `cargo run -p namsdec-re --example taux_detour -- tests/binaires/*`
//!
//! CE QUE LA MESURE NE DIT PAS. Le balayage est linéaire depuis le
//! début de chaque section exécutable : si une table de données y est
//! enchâssée, elle est décodée comme du code, et les sites qu'on en
//! tire ne sont pas des sites réels. On ne corrige pas, on le dit —
//! l'alternative serait de ne mesurer que les fonctions nommées, donc
//! de ne rien mesurer sur un binaire dépouillé, qui est précisément le
//! cas qui nous intéresse.

use namsdec_re::{elf_scan, patch, slack};

/// Ce qu'on a appris d'un seul site.
enum Issue {
    /// Un détour s'y pose.
    Posé,
    /// Il n'y en a pas : le motif, tel que le module le nomme.
    Refus(&'static str, Option<String>),
}

fn main() {
    let chemins: Vec<String> = std::env::args().skip(1).collect();
    if chemins.is_empty() {
        eprintln!("usage : taux_detour <binaire>…");
        std::process::exit(2);
    }

    let mut total_sites = 0usize;
    let mut total_posés = 0usize;
    let mut total_motifs: std::collections::BTreeMap<&'static str, usize> = Default::default();
    let mut total_mnémoniques: std::collections::BTreeMap<String, usize> = Default::default();
    // Ce que la relocalisation a rapporté : les sites dont la zone
    // recouverte porte au moins une instruction relative. Avant 4.17
    // ils étaient TOUS refusés ; aujourd'hui une partie passe, et c'est
    // la seule façon de chiffrer ce qu'a valu le travail.
    let mut total_relatifs = 0usize;
    let mut total_relatifs_posés = 0usize;

    for chemin in &chemins {
        let Ok(bytes) = std::fs::read(chemin) else {
            eprintln!("-- {chemin} : illisible, ignoré");
            continue;
        };
        let Some(scan) = elf_scan::scan(&bytes) else {
            eprintln!("-- {chemin} : pas un ELF x86-64, ignoré");
            continue;
        };

        let trous: Vec<slack::Slack> = slack::find(&bytes)
            .into_iter()
            .filter(|t| t.executable)
            .collect();

        let mut sites = 0usize;
        let mut posés = 0usize;
        let mut motifs: std::collections::BTreeMap<&'static str, usize> = Default::default();
        let mut relatifs = 0usize;
        let mut relatifs_posés = 0usize;

        // Les plages de code à balayer. Les sections exécutables quand
        // le fichier en déclare ; sinon ses segments exécutables, parce
        // qu'un binaire DÉPOUILLÉ n'a pas d'en-têtes de section et que
        // c'est justement le cas qui nous intéresse : sans ce recours,
        // deux des onze binaires rendaient « zéro site », ce qui se lit
        // comme « rien à mesurer » alors que c'était « rien de mesuré ».
        let mut plages: Vec<(usize, usize, u64)> = scan
            .sections
            .iter()
            .filter(|s| s.execute && s.size > 0 && !s.nobits)
            .map(|s| (s.offset as usize, s.size as usize, s.addr))
            .collect();
        if plages.is_empty() {
            plages = scan
                .segments
                .iter()
                .filter(|s| s.kind == "LOAD" && s.execute && s.filesz > 0)
                .map(|s| (s.offset as usize, s.filesz as usize, s.vaddr))
                .collect();
        }

        for (début, taille, addr_début) in plages {
            let fin = (début + taille).min(bytes.len());
            if début >= fin {
                continue;
            }
            let code = &bytes[début..fin];

            use iced_x86::{Decoder, DecoderOptions};
            let mut décodeur = Decoder::with_ip(64, code, addr_début, DecoderOptions::NONE);
            while décodeur.can_decode() {
                let instr = décodeur.decode();
                if instr.is_invalid() {
                    continue;
                }
                let site_vaddr = instr.ip();
                let site_offset = début + (site_vaddr - addr_début) as usize;

                sites += 1;
                let relatif = zone_relative(&bytes, site_offset, site_vaddr);
                if relatif {
                    relatifs += 1;
                }

                match essayer(&bytes, site_offset, site_vaddr, &trous) {
                    Issue::Posé => {
                        posés += 1;
                        if relatif {
                            relatifs_posés += 1;
                        }
                    }
                    Issue::Refus(motif, mnémonique) => {
                        *motifs.entry(motif).or_default() += 1;
                        if let Some(m) = mnémonique {
                            *total_mnémoniques.entry(m).or_default() += 1;
                        }
                    }
                }
            }
        }

        let taux = pourcent(posés, sites);
        println!("== {chemin}");
        println!(
            "   {sites} sites  •  {posés} posés ({taux:.1} %)  •  {} trous exécutables",
            trous.len()
        );
        if relatifs > 0 {
            println!(
                "   dont {relatifs} sites à code relatif : {relatifs_posés} posés ({:.1} %) — ce que la relocalisation a rapporté",
                pourcent(relatifs_posés, relatifs)
            );
        }
        for (motif, n) in &motifs {
            println!(
                "     refus {motif:<22} {n:>6}  ({:.1} %)",
                pourcent(*n, sites)
            );
        }
        println!();

        total_sites += sites;
        total_posés += posés;
        total_relatifs += relatifs;
        total_relatifs_posés += relatifs_posés;
        for (motif, n) in motifs {
            *total_motifs.entry(motif).or_default() += n;
        }
    }

    println!("== TOTAL sur {} binaires", chemins.len());
    println!(
        "   {total_sites} sites  •  {total_posés} posés ({:.1} %)",
        pourcent(total_posés, total_sites)
    );
    if total_relatifs > 0 {
        println!(
            "   {total_relatifs} sites à code relatif  •  {total_relatifs_posés} posés ({:.1} %)",
            pourcent(total_relatifs_posés, total_relatifs)
        );
    }
    for (motif, n) in &total_motifs {
        println!(
            "     refus {motif:<22} {n:>6}  ({:.1} %)",
            pourcent(*n, total_sites)
        );
    }
    if !total_mnémoniques.is_empty() {
        println!("\n   les instructions qui font refuser, les dix premières :");
        let mut classement: Vec<_> = total_mnémoniques.into_iter().collect();
        classement.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (mnémonique, n) in classement.into_iter().take(10) {
            println!("     {mnémonique:<16} {n:>6}");
        }
    }
}

/// Planifier un détour à cet endroit, en essayant chaque place libre.
///
/// L'ordre et l'arrêt reprennent ceux de l'exemple `detour` : un refus
/// qui porte sur le SITE ne changera pas d'une place à l'autre, donc on
/// s'arrête ; un refus qui porte sur la PLACE mérite qu'on essaie la
/// suivante.
fn essayer(bytes: &[u8], site_offset: usize, site_vaddr: u64, trous: &[slack::Slack]) -> Issue {
    let mut dernier = Issue::Refus("AucunePlaceLibre", None);
    for trou in trous {
        match patch::plan_detour(bytes, 64, site_offset, site_vaddr, trou, &[], &[]) {
            Ok(_) => return Issue::Posé,
            Err(refus) => {
                let (motif, mnémonique, dépend_du_site) = classer(&refus);
                dernier = Issue::Refus(motif, mnémonique);
                if dépend_du_site {
                    break;
                }
            }
        }
    }
    dernier
}

/// Le nom du motif, l'instruction fautive s'il y en a une, et si le
/// refus dépend du site plutôt que de la place libre.
fn classer(refus: &patch::DetourRefusé) -> (&'static str, Option<String>, bool) {
    use patch::DetourRefusé as R;
    match refus {
        R::PlaceNonExécutable => ("PlaceNonExécutable", None, false),
        R::PlaceTropPetite { .. } => ("PlaceTropPetite", None, false),
        R::HorsFichier => ("HorsFichier", None, true),
        R::SiteIndécodable { .. } => ("SiteIndécodable", None, true),
        R::TropLoin => ("TropLoin", None, false),
        R::CodeDéplacéRelatif { mnémonique, .. } => {
            ("CodeDéplacéRelatif", Some(mnémonique.clone()), true)
        }
        R::PortéeTropCourte {
            mnémonique, champ, ..
        } => (
            "PortéeTropCourte",
            Some(format!("{mnémonique} (rel{})", champ * 8)),
            true,
        ),
        R::ChampIntrouvable { mnémonique, .. } => {
            ("ChampIntrouvable", Some(mnémonique.clone()), true)
        }
        // Cet exemple mesure un fichier NEUF, sans aucun correctif
        // posé : il passe `&[]`, et ce refus ne peut donc pas tomber.
        // Il est nommé quand même plutôt que balayé par `_` — un refus
        // de plus ajouté un jour et oublié ici se compterait en silence
        // dans la colonne de la place libre.
        R::SurUnCorrectif { .. } => ("SurUnCorrectif", None, true),
    }
}

/// La zone que le `jmp` recouvrirait porte-t-elle du code relatif ?
///
/// Décodé comme `plan_detour` le fait — des instructions entières
/// jusqu'à couvrir les cinq octets du saut —, parce qu'un chiffre qui
/// compterait autre chose ne dirait rien de ce qu'on mesure.
fn zone_relative(bytes: &[u8], site_offset: usize, site_vaddr: u64) -> bool {
    use iced_x86::{Decoder, DecoderOptions, OpKind};
    let Some(reste) = bytes.get(site_offset..) else {
        return false;
    };
    let mut décodeur = Decoder::with_ip(64, reste, site_vaddr, DecoderOptions::NONE);
    let mut couvert = 0usize;
    while couvert < 5 {
        if !décodeur.can_decode() {
            return false;
        }
        let instr = décodeur.decode();
        if instr.is_invalid() {
            return false;
        }
        if instr.is_ip_rel_memory_operand()
            || (0..instr.op_count()).any(|i| {
                matches!(
                    instr.op_kind(i),
                    OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64
                )
            })
        {
            return true;
        }
        couvert += instr.len();
    }
    false
}

fn pourcent(part: usize, tout: usize) -> f64 {
    if tout == 0 {
        0.0
    } else {
        part as f64 * 100.0 / tout as f64
    }
}
