//! L'analyse, en texte : de quoi la sortir de l'outil.
//!
//! Ce qu'on comprend d'un binaire ne vaut que si on peut le transmettre —
//! dans un compte rendu, un message, un support de cours. Le rapport est
//! du Markdown ordinaire : il se colle tel quel, et se lit même sans être
//! rendu.
//!
//! Il dit la même chose que le panneau, explications comprises : un
//! tableau de « oui / non » sans ce qu'ils changent n'apprend rien à qui
//! le recevra.

use crate::elf_scan::{Kind, Relro, Scan};
use crate::{digest, imports};

fn t(lang: &str, fr: &str, en: &str) -> String {
    if lang.starts_with("fr") { fr } else { en }.to_owned()
}

/// Le rapport complet d'un binaire analysé.
///
/// `bytes` est le fichier lui-même : certaines sections du rapport se
/// lisent dans des tables que le résumé ne porte pas — les fonctions
/// empruntées, par exemple, qui vivent dans les relocations.
///
/// `disasm` est son désassemblage, parce que tout ne se lit pas dans le
/// fichier : un programme qui parle au noyau directement n'emprunte
/// rien, et seule la suite des instructions le dit. Vide quand on ne
/// l'a pas — le rapport perd alors les lignes qui en dépendent, et
/// n'invente rien à la place.
pub fn markdown(
    scan: &Scan,
    bytes: &[u8],
    disasm: &[namsdec_disasm::DecodedInstruction],
    lang: &str,
    name: &str,
) -> String {
    let fr = lang.starts_with("fr");
    let mut out = String::new();

    out.push_str(&format!(
        "# {} — {name}\n\n",
        t(lang, "Analyse du binaire", "Binary analysis")
    ));

    let kind = match scan.kind {
        Kind::Executable => t(
            lang,
            "exécutable à adresse fixe",
            "fixed-address executable",
        ),
        Kind::Shared => t(
            lang,
            "exécutable relogeable (PIE)",
            "relocatable executable (PIE)",
        ),
        Kind::Relocatable => t(lang, "objet à lier", "relocatable object"),
        Kind::Core => t(lang, "image mémoire", "core dump"),
        Kind::Other(v) => format!("type {v}"),
    };
    // De quel fichier parle-t-on ? C'est la question qu'un compte rendu
    // doit trancher en premier : deux personnes qui lisent « le
    // crackme » n'ont pas forcément le même octet sous les yeux. Avant
    // même le type et la machine, donc.
    out.push_str(&format!(
        "- {} : {} {}\n- SHA-256 : `{}`\n- CRC-32 : `{:08x}`\n",
        t(lang, "Taille", "Size"),
        bytes.len(),
        t(lang, "octets", "bytes"),
        digest::hex(&digest::sha256(bytes)),
        digest::crc32(bytes),
    ));
    out.push_str(&format!(
        "- {} : {kind}\n- {} : {}\n- {} : {:#x}\n- {} : {}\n",
        t(lang, "Type", "Type"),
        t(lang, "Machine", "Machine"),
        scan.machine,
        t(lang, "Point d'entrée", "Entry point"),
        scan.entry,
        t(lang, "Liens", "Linking"),
        match &scan.interpreter {
            Some(interp) => format!("{} ({interp})", t(lang, "dynamique", "dynamic")),
            None => t(lang, "statique", "static"),
        }
    ));
    if let Some(build_id) = &scan.build_id {
        out.push_str(&format!(
            "- {} : `{build_id}`\n",
            t(lang, "Empreinte de build", "Build ID")
        ));
    }

    // Les signaux juste après l'identité : c'est ce qu'on lit en
    // premier d'un rapport, comme on lit le panneau du même nom en
    // premier dans l'interface. La même fonction les produit — deux
    // synthèses qui diraient des choses différentes seraient pires
    // qu'une.
    let signaux = crate::signals::gather(bytes, disasm, lang);
    if !signaux.is_empty() {
        out.push_str(&format!("\n## {}\n\n", t(lang, "Signaux", "Signals")));
        out.push_str(&format!(
            "{}\n\n",
            t(
                lang,
                "*Aucune de ces lignes n'est une preuve, et rien n'est additionné en \
                 score : chacune dit un fait, ce que ce fait change, et le panneau où \
                 le vérifier.*",
                "*None of these lines is proof, and nothing is added up into a score: \
                 each says a fact, what that fact changes, and the panel where to check \
                 it.*",
            )
        ));
        for signal in &signaux {
            out.push_str(&format!(
                "- **{}** — `{}`\n  - {}\n  - {} : {}\n",
                signal.title(lang),
                signal.detail,
                signal.why(lang),
                t(lang, "à lire dans", "read it in"),
                signal.panel(lang),
            ));
        }
    }

    out.push_str(&format!(
        "\n## {}\n\n",
        t(lang, "Protections", "Protections")
    ));
    out.push_str(&format!(
        "| {} | {} | {} |\n|---|---|---|\n",
        t(lang, "Protection", "Protection"),
        t(lang, "État", "State"),
        t(lang, "Ce que cela change", "What it changes")
    ));

    let mut row = |name: &str, state: String, why: String| {
        out.push_str(&format!("| {name} | {state} | {why} |\n"));
    };

    row(
        "PIE",
        if scan.pie() {
            t(lang, "oui", "yes")
        } else {
            t(lang, "non", "no")
        },
        if scan.pie() {
            t(
                lang,
                "Adresses différentes à chaque exécution.",
                "Different addresses on every run.",
            )
        } else {
            t(
                lang,
                "Les adresses du désassemblage sont celles qu'il aura.",
                "The disassembly's addresses are the ones it will have.",
            )
        },
    );
    row(
        "NX",
        match scan.nx {
            Some(true) => t(lang, "pile non exécutable", "non-executable stack"),
            Some(false) => t(lang, "pile exécutable", "executable stack"),
            None => t(lang, "non déclaré", "not declared"),
        },
        match scan.nx {
            Some(true) => t(
                lang,
                "Il faut réutiliser le code du programme (ROP), pas en injecter.",
                "One must reuse the program's own code (ROP), not inject any.",
            ),
            Some(false) => t(
                lang,
                "Des octets écrits sur la pile peuvent être exécutés.",
                "Bytes written onto the stack can be executed.",
            ),
            None => t(
                lang,
                "Le noyau décide seul des droits de la pile.",
                "The kernel alone decides the stack's rights.",
            ),
        },
    );
    row(
        "RELRO",
        match scan.relro {
            Relro::Full => t(lang, "complet", "full"),
            Relro::Partial => t(lang, "partiel", "partial"),
            Relro::None => t(lang, "absent", "none"),
        },
        match scan.relro {
            Relro::Full => t(
                lang,
                "Les tables de liaison passent en lecture seule après le démarrage.",
                "Linking tables become read-only after startup.",
            ),
            Relro::Partial => t(
                lang,
                "La table des sauts reste inscriptible.",
                "The jump table stays writable.",
            ),
            Relro::None => t(
                lang,
                "Rien n'est repassé en lecture seule.",
                "Nothing is made read-only.",
            ),
        },
    );
    row(
        &t(lang, "Canari de pile", "Stack canary"),
        if scan.canary {
            t(lang, "présent", "present")
        } else {
            t(lang, "absent", "absent")
        },
        if scan.canary {
            t(
                lang,
                "Un débordement qui atteint l'adresse de retour arrête le programme.",
                "An overflow reaching the return address aborts the program.",
            )
        } else {
            t(
                lang,
                "Rien ne surveille l'adresse de retour.",
                "Nothing watches the return address.",
            )
        },
    );
    row(
        "IBT",
        if scan.cet.ibt {
            t(lang, "demandé", "requested")
        } else {
            t(lang, "non demandé", "not requested")
        },
        t(
            lang,
            "Un saut indirect doit atterrir sur un `endbr64`.",
            "An indirect jump must land on an `endbr64`.",
        ),
    );
    if scan.cet.shadow_stack {
        row(
            &t(lang, "Pile fantôme", "Shadow stack"),
            t(lang, "demandée", "requested"),
            t(
                lang,
                "Le processeur garde sa propre copie des adresses de retour.",
                "The processor keeps its own copy of return addresses.",
            ),
        );
    }
    if !scan.fortified.is_empty() {
        row(
            "FORTIFY",
            format!(
                "{} {}",
                scan.fortified.len(),
                t(lang, "fonctions", "functions")
            ),
            t(
                lang,
                "Des copies à bornes vérifiées remplacent les originales.",
                "Bounds-checked copies replace the originals.",
            ),
        );
    }
    row(
        &t(lang, "Symboles", "Symbols"),
        if scan.stripped {
            t(lang, "dépouillé", "stripped")
        } else {
            scan.symbol_count.to_string()
        },
        if scan.stripped {
            t(
                lang,
                "Les fonctions n'ont plus que leur adresse.",
                "Functions have nothing but their address.",
            )
        } else {
            t(
                lang,
                "Les fonctions se lisent par leur nom.",
                "Functions read by name.",
            )
        },
    );
    for segment in scan.writable_executable() {
        row(
            &t(lang, "Segment W+X", "W+X segment"),
            format!("{:#x}", segment.vaddr),
            t(
                lang,
                "Inscriptible **et** exécutable : on peut y écrire du code et sauter dedans.",
                "Writable **and** executable: one can write code there and jump into it.",
            ),
        );
    }

    if !scan.needed.is_empty() {
        out.push_str(&format!(
            "\n## {}\n\n",
            t(lang, "Bibliothèques réclamées", "Required libraries")
        ));
        for lib in &scan.needed {
            out.push_str(&format!("- `{lib}`\n"));
        }
    }
    // Ce que le binaire emprunte, nommément : c'est la première
    // question qu'on se pose devant du code qu'on n'a pas écrit.
    let empruntées = imports::scan(bytes);
    if !empruntées.is_empty() {
        // Ce qu'il sait faire avant ce qu'il emprunte : six cases se
        // lisent, cinquante noms se parcourent.
        let familles = crate::behaviour::families_of(empruntées.iter().map(|i| i.name.as_str()));
        if !familles.is_empty() {
            out.push_str(&format!(
                "\n## {}\n\n",
                t(lang, "Ce qu'il sait faire", "What it can do")
            ));
            for famille in familles {
                out.push_str(&format!(
                    "- **{}** — {}\n",
                    famille.label(lang),
                    famille.why(lang)
                ));
            }
        }
        out.push_str(&format!(
            "\n## {} ({})\n\n",
            t(lang, "Fonctions importées", "Imported functions"),
            empruntées.len()
        ));
        for import in &empruntées {
            out.push_str(&format!("- `{}` — {:#x}", import.name, import.plt));
            // Ce que fait la fonction, quand namsdec le sait : une liste
            // de noms suppose qu'on les connaisse déjà.
            if let Some(famille) = crate::behaviour::family_of(&import.name) {
                out.push_str(&format!(" — *{}*", famille.label(lang)));
            }
            if let Some(note) = crate::libc::explain(&import.name) {
                out.push_str(&format!(" — {}", note.text(lang)));
                if note.watch {
                    out.push_str(&format!(" **({})**", t(lang, "à regarder", "worth a look")));
                }
            }
            out.push('\n');
        }
    }
    // Ce que le binaire dit de lui-même : les chaînes les plus
    // parlantes. Vingt, pas deux cents — un rapport qu'on ne lit pas
    // n'en est plus un, et le panneau reste là pour la liste entière.
    let chaînes = crate::strings::scan_elf(bytes).unwrap_or_default();
    let parlantes: Vec<&crate::strings::Found> = chaînes
        .iter()
        // Les chaînes courtes d'un binaire sont surtout des fragments de
        // symboles ; celles qui portent un message font au moins huit
        // caractères, espace compris.
        .filter(|found| found.text.trim().len() >= 8 && found.text.contains(' '))
        .take(20)
        .collect();
    if !parlantes.is_empty() {
        out.push_str(&format!(
            "\n## {}\n\n",
            t(lang, "Ce qu'il dit", "What it says")
        ));
        for found in parlantes {
            // Le texte tel quel, entre accents graves : une chaîne peut
            // contenir des caractères qui, en Markdown, veulent dire
            // autre chose.
            out.push_str(&format!("- `{}`\n", found.text.replace('`', "'")));
        }
    }
    // Les indicateurs : ce que le destinataire du rapport pourra aller
    // vérifier de son côté, dans ses propres journaux. C'est la partie
    // du compte rendu qui sert à quelqu'un d'autre que son auteur.
    let indicateurs = crate::ioc::from_strings(&chaînes);
    if !indicateurs.is_empty() {
        out.push_str(&format!(
            "\n## {}\n\n",
            t(
                lang,
                "Indicateurs à vérifier ailleurs",
                "Indicators to check elsewhere"
            )
        ));
        for nature in crate::ioc::ALL {
            let miens: Vec<&crate::ioc::Indicator> =
                indicateurs.iter().filter(|i| i.kind == nature).collect();
            if miens.is_empty() {
                continue;
            }
            out.push_str(&format!("\n**{}**\n\n", nature.label(lang)));
            for indicateur in miens {
                out.push_str(&format!(
                    "- `{}` — {:#x}\n",
                    indicateur.value.replace('`', "'"),
                    indicateur.addr
                ));
            }
        }
    }
    for (label, value) in [("RUNPATH", &scan.runpath), ("RPATH", &scan.rpath)] {
        if let Some(value) = value {
            out.push_str(&format!("\n- {label} : `{value}`\n"));
        }
    }

    out.push_str(&format!("\n## {}\n\n", t(lang, "Segments", "Segments")));
    out.push_str("| | | | |\n|---|---|---|---|\n");
    for segment in &scan.segments {
        out.push_str(&format!(
            "| {} | `{}` | {:#x} | {} {} |\n",
            segment.kind,
            segment.rights(),
            segment.vaddr,
            segment.memsz,
            t(lang, "o", "B")
        ));
    }

    if !scan.sections.is_empty() {
        out.push_str(&format!("\n## {}\n\n", t(lang, "Sections", "Sections")));
        for section in &scan.sections {
            out.push_str(&format!(
                "- `{}` {:#x} — {} {}{}\n",
                section.name,
                section.addr,
                section.size,
                t(lang, "o", "B"),
                if section.execute {
                    t(lang, " (exécutable)", " (executable)")
                } else if section.write {
                    t(lang, " (inscriptible)", " (writable)")
                } else {
                    String::new()
                }
            ));
        }
    }

    if !scan.functions.is_empty() {
        out.push_str(&format!(
            "\n## {} ({})\n\n",
            t(lang, "Fonctions", "Functions"),
            scan.functions.len()
        ));
        for function in &scan.functions {
            out.push_str(&format!(
                "- {:#x} `{}`{}\n",
                function.addr,
                function.name,
                if function.size > 0 {
                    format!(" — {} {}", function.size, t(lang, "o", "B"))
                } else {
                    String::new()
                }
            ));
        }
    }

    out.push_str(&format!(
        "\n---\n\n{}\n",
        if fr {
            "Rapport produit par namsdec (MIT) — atelier d'apprentissage de l'assembleur NASM."
        } else {
            "Report produced by namsdec (MIT) — a workshop for learning NASM assembly."
        }
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le rapport dit ce que le panneau dit : l'état *et* ce qu'il
    /// change. Sans la seconde colonne, il n'apprendrait rien à celui à
    /// qui on l'envoie.
    #[test]
    fn the_report_carries_the_explanations_too() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        let Some(scan) = crate::elf_scan::scan(&bytes) else {
            eprintln!("/bin/ls n'est pas un ELF64 ici : test ignoré");
            return;
        };
        let disasm = namsdec_disasm::disassemble(&bytes[..bytes.len().min(0x10000)], 0x400000);
        let report = markdown(&scan, &bytes, &disasm, "fr", "ls");

        assert!(report.starts_with("# Analyse du binaire — ls"));
        // De quel fichier parle-t-on ? C'est la première chose qu'un
        // compte rendu doit trancher, et elle doit valoir celle que
        // `sha256sum` donne — sinon on ne pourrait pas les comparer.
        assert!(
            report.contains(&format!(
                "SHA-256 : `{}`",
                digest::hex(&digest::sha256(&bytes))
            )),
            "l'empreinte du fichier entier :\n{report}"
        );
        assert!(
            report.contains(&format!("{} octets", bytes.len())),
            "et sa taille exacte"
        );
        for attendu in [
            "PIE",
            "NX",
            "RELRO",
            "Canari de pile",
            "Segments",
            "LOAD",
            "Fonctions importées",
            "malloc",
            "Ce qu'il dit",
        ] {
            assert!(report.contains(attendu), "« {attendu} » manque :\n{report}");
        }
        assert!(
            report.contains("Ce que cela change"),
            "la colonne d'explication est le cœur du rapport"
        );
        // Ce qu'il sait faire : `ls` ouvre des dossiers, donc la famille
        // « Fichiers » est là, avec la phrase qui dit ce qu'elle change.
        assert!(
            report.contains("## Ce qu'il sait faire"),
            "le portrait par familles :\n{report}"
        );
        assert!(report.contains("**Fichiers**"), "{report}");
        // Et chaque signal relevé est dans le rapport, avec son fait et
        // le panneau où le vérifier : le rapport et le panneau disent la
        // même chose, puisque c'est la même fonction qui les produit.
        let signaux = crate::signals::gather(&bytes, &disasm, "fr");
        assert!(
            !signaux.is_empty(),
            "un binaire de cette taille a quelque chose à relever"
        );
        assert!(report.contains("## Signaux"), "{report}");
        for signal in &signaux {
            assert!(
                report.contains(signal.title("fr")),
                "« {} » manque au rapport :\n{report}",
                signal.title("fr")
            );
            assert!(
                report.contains(signal.panel("fr")),
                "le panneau de « {} » manque",
                signal.title("fr")
            );
        }
        assert!(
            report.contains("namsdec (MIT)"),
            "le rapport dit d'où il vient, et sous quelle licence"
        );
        // Et la version anglaise dit autre chose que la française.
        let english = markdown(&scan, &bytes, &disasm, "en", "ls");
        assert_ne!(report, english);
        assert!(english.starts_with("# Binary analysis — ls"));
    }
}
