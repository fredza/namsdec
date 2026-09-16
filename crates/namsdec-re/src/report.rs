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

fn t(lang: &str, fr: &str, en: &str) -> String {
    if lang.starts_with("fr") { fr } else { en }.to_owned()
}

/// Le rapport complet d'un binaire analysé.
pub fn markdown(scan: &Scan, lang: &str, name: &str) -> String {
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
        let report = markdown(&scan, "fr", "ls");

        assert!(report.starts_with("# Analyse du binaire — ls"));
        for attendu in ["PIE", "NX", "RELRO", "Canari de pile", "Segments", "LOAD"] {
            assert!(report.contains(attendu), "« {attendu} » manque :\n{report}");
        }
        assert!(
            report.contains("Ce que cela change"),
            "la colonne d'explication est le cœur du rapport"
        );
        assert!(
            report.contains("namsdec (MIT)"),
            "le rapport dit d'où il vient, et sous quelle licence"
        );
        // Et la version anglaise dit autre chose que la française.
        let english = markdown(&scan, "en", "ls");
        assert_ne!(report, english);
        assert!(english.starts_with("# Binary analysis — ls"));
    }
}
