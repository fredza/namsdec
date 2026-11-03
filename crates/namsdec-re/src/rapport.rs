//! Ce que la frontière permet : sortir l'analyse de l'application.
//!
//! **C'est l'étape qui justifie P7, et la seule qui le prouve.** Tant
//! qu'un module n'avait qu'une sortie — `ui()`, qui dessine —, ce que
//! namsdec comprend d'un binaire n'existait que peint à l'écran : ni
//! exportable, ni lisible par un autre programme, ni éprouvable
//! autrement qu'en mesurant des pixels. Chaque module rend aujourd'hui
//! une [`Vue`], et il suffit de les parcourir.
//!
//! Trois sorties, et la même source pour les trois :
//!
//! * [`markdown`] — le rapport que l'on colle dans un compte rendu. Il
//!   dit tout ce que les panneaux disent, explications comprises :
//!   `report::markdown` écrivait une prose soignée mais ne couvrait que
//!   six rubriques sur vingt-deux ;
//! * [`json`] — la même chose pour un autre programme. Ce qui n'est pas
//!   dit ne pèse rien : une vue de trois cents lignes ne porte pas
//!   autant de `"copiable": false` que de contenu ;
//! * [`analyse`] — tout cela **sans peindre une seule image**, à partir
//!   des octets. Aucun contexte egui, aucune surface, aucun thème.
//!
//! **Ce que cela ne fait pas, et il faut le dire :** `namsdec-re`
//! dépend toujours d'egui, parce que le trait `ReModule` porte encore
//! `fn ui(&mut self, ui: &mut egui::Ui, …)`. S'en défaire demanderait
//! de déménager les vingt-deux `ui()` dans `namsdec-gui` — c'est un
//! autre lot. Ici, « sans interface » veut dire : aucune image n'est
//! rendue, et le résultat ne dépend d'aucun pixel.

use crate::vue::{Bloc, Dessin, Ligne, Ton, Vue};
use crate::{ReContext, ReModuleRegistry};

/// Ce qu'un panneau montre, avec de quoi le nommer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Panneau {
    pub id: String,
    pub titre: String,
    pub vue: Vue,
}

/// Tout ce que les panneaux montrent de ce binaire, dans leur ordre.
///
/// Les panneaux muets sont écartés : un module qui n'a rien à dire d'un
/// fichier donné — pas d'empaqueteur, pas de règle YARA — ne mérite pas
/// une rubrique vide dans un rapport, qui ferait chercher ce qui n'y
/// est pas.
///
/// LES TROIS PANNEAUX PARTAGÉS Y SONT AUSSI, pour la part d'eux-mêmes
/// qui est de la compréhension : l'inspecteur de l'éditeur binaire, les
/// mesures du vidage, le plan d'un détour. Leur instrument — une grille
/// d'octets, un formulaire — n'a rien à faire dans un rapport.
pub fn panneaux(registry: &mut ReModuleRegistry, ctx: &ReContext) -> Vec<Panneau> {
    for module in registry.iter_mut() {
        module.on_binary_loaded(ctx);
    }
    registry
        .iter_mut()
        .filter(|module| module.dans_un_rapport(ctx))
        .filter_map(|module| {
            let vue = module.vue(ctx)?;
            (!vue.est_vide()).then(|| Panneau {
                id: module.id().to_owned(),
                titre: module.title(ctx.lang).to_owned(),
                vue,
            })
        })
        .collect()
}

/// Le rapport complet, en Markdown ordinaire.
///
/// Du Markdown et non un format à nous : il se colle tel quel dans un
/// ticket, un courriel ou un support de cours, et se lit même sans être
/// rendu.
pub fn markdown(panneaux: &[Panneau], titre: &str) -> String {
    let mut out = format!("# {titre}\n");
    for panneau in panneaux {
        out.push_str(&format!("\n## {}\n\n", panneau.titre));
        écrire_blocs(&mut out, &panneau.vue.blocs, 0);
    }
    out
}

/// La même chose pour un autre programme.
pub fn json(panneaux: &[Panneau]) -> String {
    serde_json::to_string_pretty(panneaux).unwrap_or_else(|_| "[]".to_owned())
}

/// Un décalage de deux espaces par niveau de retrait.
fn marge(retrait: usize) -> String {
    "  ".repeat(retrait)
}

/// Une ligne, lisible — c'est-à-dire avec les blancs que l'écran
/// donnait gratuitement.
///
/// `Ligne::texte_nu` colle les fragments bout à bout, et c'est juste :
/// une ligne de vue n'est pas une phrase, c'est une suite de morceaux
/// que la mise en page SÉPARE. À l'écran, egui pose son espacement
/// entre deux étiquettes voisines ; dans un fichier, personne ne le
/// fait, et l'on lisait « Pas de canari de pile—aucun témoin posé » ou
/// « [section .comment]rustc 1.97.1— le compilateur y écrit son nom ».
///
/// On recolle donc avec un blanc, après avoir rogné chaque morceau :
/// certains fragments portent déjà un espace de tête, écrit du temps où
/// le module posait lui-même son rythme.
fn lisible(ligne: &Ligne) -> String {
    ligne
        .fragments
        .iter()
        .map(|f| f.texte.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn écrire_blocs(out: &mut String, blocs: &[Bloc], retrait: usize) {
    for bloc in blocs {
        écrire_bloc(out, bloc, retrait);
    }
}

fn écrire_bloc(out: &mut String, bloc: &Bloc, retrait: usize) {
    let m = marge(retrait);
    match bloc {
        Bloc::Texte(ligne) => {
            let texte = lisible(ligne);
            if !texte.trim().is_empty() {
                out.push_str(&format!("{m}{texte}\n\n"));
            }
        }
        Bloc::Attente(ligne) => out.push_str(&format!("{m}… {}\n\n", lisible(ligne))),
        Bloc::Trait => out.push_str("---\n\n"),
        Bloc::Jauge { part, texte } => out.push_str(&format!(
            "{m}{} — {:.0} %\n\n",
            lisible(texte),
            part * 100.0
        )),
        Bloc::Section { titre, blocs } => {
            out.push_str(&format!("\n{m}### {}\n\n", lisible(titre)));
            écrire_blocs(out, blocs, retrait);
        }
        Bloc::Repliable { titre, blocs, .. } => {
            // DÉPLIÉ, TOUJOURS. Un repliable dit où l'on peut alléger
            // l'écran ; un rapport qu'on relit six mois plus tard n'a
            // pas de clic à offrir, et ce qui y serait caché serait
            // perdu.
            out.push_str(&format!("\n{m}#### {}\n\n", lisible(titre)));
            écrire_blocs(out, blocs, retrait);
        }
        Bloc::Retrait(blocs) => écrire_blocs(out, blocs, retrait + 1),
        Bloc::Tableau {
            colonnes, rangées, ..
        } => écrire_tableau(out, colonnes, rangées, retrait),
        Bloc::Verdict {
            nom,
            état,
            verdict,
            pourquoi,
        } => {
            // La même marque qu'à l'écran, et pour la même raison :
            // elle porte le verdict SANS la couleur, qu'un fichier
            // Markdown n'a de toute façon pas.
            let marque = match verdict {
                Ton::Bon => '\u{221a}',
                Ton::Mauvais => '\u{00d7}',
                _ => '\u{2022}',
            };
            let état = lisible(état);
            let espace = if état.is_empty() { "" } else { " " };
            out.push_str(&format!("{m}- {marque} **{}**{espace}{état}", lisible(nom)));
            match pourquoi {
                Some(texte) => out.push_str(&format!(" — {texte}\n")),
                None => out.push('\n'),
            }
        }
        Bloc::Champ(champ) => {
            // UN CHAMP EST UNE COMMANDE, et un rapport n'en a pas. Mais
            // ce qu'il CONTIENT dit souvent de quoi le panneau parle —
            // la tranche du vidage, l'adresse d'un détour —, et le
            // taire ferait lire des mesures sans savoir sur quoi.
            if !champ.valeur.trim().is_empty() {
                let étiquette = lisible(&champ.étiquette);
                let étiquette = if étiquette.is_empty() {
                    champ.action.clone()
                } else {
                    étiquette
                };
                out.push_str(&format!("{m}{étiquette} {}\n\n", champ.valeur));
            }
        }
        Bloc::Code(texte) => out.push_str(&format!("```\n{texte}\n```\n\n")),
        Bloc::Dessin(dessin) => {
            // UN RAPPORT NE DESSINE PAS, et il ne fait pas semblant : il
            // dit ce que le dessin mesure, et combien. Un profil
            // d'entropie en art ASCII serait moins juste que la phrase
            // qui le résume, et un graphe d'appels en texte est
            // illisible.
            let phrase = match dessin {
                Dessin::Profil {
                    valeurs,
                    taille_tranche,
                    total,
                } => {
                    let moyenne = if valeurs.is_empty() {
                        0.0
                    } else {
                        valeurs.iter().sum::<f64>() / valeurs.len() as f64
                    };
                    let haute = valeurs.iter().cloned().fold(0.0f64, f64::max);
                    format!(
                        "Profil d'entropie : {} tranches de {taille_tranche} octets sur \
                         {total} ; moyenne {moyenne:.2}, maximum {haute:.2} bits par octet.",
                        valeurs.len()
                    )
                }
                Dessin::Graphe { nœuds, arêtes } => format!(
                    "Graphe d'appels : {} fonctions, {} appels — {}.",
                    nœuds.len(),
                    arêtes.len(),
                    nœuds
                        .iter()
                        .map(|n| n.titre.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            };
            out.push_str(&format!("{m}{phrase}\n\n"));
        }
    }
}

fn écrire_tableau(out: &mut String, colonnes: &[Ligne], rangées: &[Vec<Ligne>], retrait: usize) {
    if rangées.is_empty() && colonnes.is_empty() {
        return;
    }
    let combien = rangées
        .iter()
        .map(Vec::len)
        .chain(std::iter::once(colonnes.len()))
        .max()
        .unwrap_or(0)
        .max(1);
    let m = marge(retrait);
    let cellule = |ligne: Option<&Ligne>| {
        // LE TUYAU EST LA SYNTAXE DU TABLEAU : une cellule qui en
        // contient un couperait la rangée en deux, et un tableau
        // Markdown mal formé ne se rend plus du tout. Une barre
        // verticale pleine passe pour un tuyau à l'œil, et ne casse
        // rien.
        ligne
            .map(lisible)
            .unwrap_or_default()
            .replace('|', "\u{2502}")
            .replace('\n', " ")
    };
    // SANS EN-TÊTE, UN TABLEAU MARKDOWN N'EXISTE PAS : la syntaxe en
    // exige un. La moitié des tableaux du projet alignent « nom /
    // valeur » sans les titrer — on leur en donne un vide, qui se rend
    // comme une bande grise sans texte, plutôt que de perdre
    // l'alignement en écrivant des lignes de texte.
    let titres: Vec<String> = (0..combien)
        .map(|rang| cellule(colonnes.get(rang)))
        .collect();
    out.push_str(&format!("{m}| {} |\n", titres.join(" | ")));
    out.push_str(&format!("{m}|{}\n", " --- |".repeat(combien)));
    for rangée in rangées {
        let cellules: Vec<String> = (0..combien).map(|rang| cellule(rangée.get(rang))).collect();
        out.push_str(&format!("{m}| {} |\n", cellules.join(" | ")));
    }
    out.push('\n');
}

/// L'analyse d'un fichier, **sans peindre une seule image**.
///
/// C'est la preuve que la frontière servait à quelque chose : on donne
/// des octets, on reçoit ce que namsdec en comprend. Aucun contexte
/// egui, aucune surface, aucun thème, et rien qui dépende d'un pixel.
///
/// Le désassemblage se tire du **segment exécutable** que l'en-tête
/// déclare, parce que tout ne se lit pas dans le fichier : un programme
/// qui parle au noyau par son numéro n'emprunte rien, et seule la suite
/// des instructions le dit. Faute d'en-tête ELF, on lit le fichier
/// entier depuis son premier octet — c'est ce que fait l'application
/// devant un tampon brut.
pub fn analyse(octets: &[u8], lang: &str) -> Vec<Panneau> {
    use namsdec_core::{Cpu, Memory};

    let scan = crate::elf_scan::scan(octets);
    // Le segment chargeable qui s'exécute : c'est là qu'est le code, et
    // son adresse virtuelle est celle que porteront les instructions.
    let (base, début, fin) = scan
        .as_ref()
        .and_then(|scan| {
            let segment = scan
                .segments
                .iter()
                .filter(|s| s.rights().contains('x') && s.filesz > 0)
                .max_by_key(|s| s.filesz)?;
            let début = usize::try_from(segment.offset).ok()?;
            let fin = début.checked_add(usize::try_from(segment.filesz).ok()?)?;
            (fin <= octets.len()).then_some((segment.vaddr, début, fin))
        })
        .unwrap_or((0x400000, 0, octets.len()));
    let code = &octets[début..fin];
    let disasm = namsdec_disasm::disassemble(code, base);
    // Une mémoire qui porte vraiment le code : plusieurs panneaux la
    // lisent — les chaînes, le résumé — et une mémoire vide leur ferait
    // dire « aucune » là où il y en a.
    let mut mem = Memory::new(base, code.len().max(1));
    let _ = mem.load(base, code);
    let cpu = Cpu::new(mem);
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: octets,
        lang,
        names: None,
        patches: &[],
    };
    let mut registry = ReModuleRegistry::standard();
    panneaux(&mut registry, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binaire_d_épreuve() -> Option<Vec<u8>> {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/binaires/crackme-01");
        std::fs::read(&chemin).ok()
    }

    /// Un binaire s'analyse entièrement sans qu'une image soit rendue.
    ///
    /// **C'EST L'ÉTAPE QUI JUSTIFIE P7**, et le contrôle qui la retient.
    /// Avant la frontière, obtenir ceci demandait un contexte egui, une
    /// surface, un thème, et la collecte des formes de texte peintes —
    /// c'est-à-dire de lire une image pour savoir ce qu'un programme a
    /// compris.
    #[test]
    fn un_binaire_s_analyse_sans_peindre() {
        let Some(octets) = binaire_d_épreuve() else {
            return;
        };
        let panneaux = analyse(&octets, "fr");
        assert!(
            panneaux.len() >= 8,
            "un vrai binaire remplit plusieurs panneaux : {}",
            panneaux.len()
        );

        // LES PANNEAUX QUI PARLENT DE LA MACHINE SONT ÉCARTÉS : aucun
        // outil extérieur n'a tourné, et « radare2 n'est pas installé
        // ici » n'apprend rien à qui lira le rapport ailleurs.
        for outil in ["objdump", "ghidra", "radare2", "retdec", "ndisasm"] {
            assert!(
                !panneaux.iter().any(|p| p.id == outil),
                "{outil} n'a rien à dire de ce fichier"
            );
        }

        // ET CE QU'ILS DISENT EST CE QU'ON VOYAIT À L'ÉCRAN.
        let md = markdown(&panneaux, "crackme-01");
        for attendu in ["# crackme-01", "## En bref", "## Protections du binaire"] {
            assert!(md.contains(attendu), "le rapport porte « {attendu} »");
        }
        // Les protections s'y lisent avec leur marque et leur raison,
        // comme dans le panneau.
        assert!(
            md.contains("**NX**") && md.contains("**RELRO**"),
            "les verdicts sont dits"
        );
        // Et les mesures qui n'existaient qu'en pixels y sont en
        // toutes lettres.
        assert!(
            md.contains("Profil d'entropie"),
            "le profil d'entropie se dit, faute de se dessiner"
        );
    }

    /// Le même fond, lisible par un autre programme.
    #[test]
    fn la_même_analyse_se_relit_en_json() {
        let Some(octets) = binaire_d_épreuve() else {
            return;
        };
        let panneaux = analyse(&octets, "fr");
        let texte = json(&panneaux);
        let relus: Vec<Panneau> = serde_json::from_str(&texte).expect("ce qu'on écrit se relit");
        assert_eq!(relus.len(), panneaux.len());
        assert_eq!(relus[0].id, panneaux[0].id);
        assert_eq!(relus[0].vue, panneaux[0].vue, "l'aller-retour est fidèle");

        // CE QUI N'EST PAS DIT NE PÈSE RIEN : un fragment ordinaire ne
        // porte ni `copiable`, ni `bouton`, ni `inactif` dans le JSON.
        // Sans cette règle, une vue de trois cents lignes porterait
        // autant de `false` que de contenu.
        assert!(
            !texte.contains("\"copiable\": false"),
            "les faux par défaut ne sont pas écrits"
        );
    }

    /// Les fragments d'une ligne se lisent séparés.
    ///
    /// À l'écran, egui pose son espacement entre deux étiquettes
    /// voisines, et le module n'a rien à écrire. Dans un fichier,
    /// personne ne le fait : collés, les morceaux donnaient « Pas de
    /// canari de pile—aucun témoin posé ». Vu en lisant le rapport,
    /// pas en relisant le code.
    #[test]
    fn les_morceaux_d_une_ligne_ne_se_collent_pas() {
        use crate::vue::Fragment;
        let ligne: Ligne = vec![
            Fragment::dit("Pas de canari de pile"),
            Fragment::dit("—"),
            // Avec un espace de tête, comme en portent les fragments
            // écrits du temps où le module posait son rythme.
            Fragment::dit(" aucun témoin posé"),
        ]
        .into();
        assert_eq!(
            lisible(&ligne),
            "Pas de canari de pile — aucun témoin posé",
            "un blanc entre les morceaux, et un seul"
        );
        // Et la vue, elle, ne change pas : `texte_nu` colle, parce
        // qu'une ligne de vue n'est pas une phrase.
        assert_eq!(ligne.texte_nu(), "Pas de canari de pile— aucun témoin posé");
    }

    /// Le rapport est du Markdown qui tient debout.
    ///
    /// Un tableau Markdown sans ligne de séparation ne se rend pas du
    /// tout — et la moitié des tableaux du projet n'ont pas d'en-tête,
    /// ce qui est précisément le cas qu'on risque d'oublier.
    #[test]
    fn les_tableaux_du_rapport_tiennent_debout() {
        let Some(octets) = binaire_d_épreuve() else {
            return;
        };
        let md = markdown(&analyse(&octets, "fr"), "crackme-01");
        let lignes: Vec<&str> = md.lines().collect();
        let mut tableaux = 0;
        for (rang, ligne) in lignes.iter().enumerate() {
            let tuyaux = ligne.trim_start();
            if !tuyaux.starts_with('|') {
                continue;
            }
            // Une rangée de tableau est soit l'en-tête (suivie de la
            // séparation), soit précédée d'une rangée.
            let avant = rang.checked_sub(1).map(|r| lignes[r].trim_start());
            let après = lignes.get(rang + 1).map(|l| l.trim_start());
            let séparation =
                |l: Option<&str>| l.is_some_and(|l| l.starts_with('|') && l.contains("---"));
            if séparation(après) {
                tableaux += 1;
                // Le nombre de colonnes de l'en-tête et de la
                // séparation doit coïncider, sans quoi rien ne se rend.
                let colonnes = tuyaux.matches('|').count();
                assert_eq!(
                    colonnes,
                    après.unwrap().matches('|').count(),
                    "en-tête et séparation s'accordent : {tuyaux}"
                );
                continue;
            }
            assert!(
                avant.is_some_and(|l| l.starts_with('|')),
                "une rangée de tableau sans en-tête ni séparation ne se rend pas : {tuyaux}"
            );
        }
        assert!(tableaux >= 3, "le rapport porte des tableaux : {tableaux}");
    }
}
