//! Le panneau « Protections » : ce que le binaire oppose à qui l'attaque.
//!
//! C'est la première chose qu'on regarde d'un binaire inconnu, et la
//! dernière qu'on vérifie sur le sien. Chaque ligne dit l'état, et ce que
//! cet état change concrètement — une case rouge sans explication
//! n'apprend rien à qui découvre.

use crate::elf_scan::{self, Kind, Relro, Scan};
use crate::vue::{Bloc, Fragment, Ton, Vue};
use crate::{ReContext, ReModule, ReSection};

#[derive(Default)]
pub struct ProtectionsModule {
    scan: Option<Scan>,
    /// Le fichier n'était pas un ELF64 : on le dit, plutôt que de
    /// laisser un panneau vide qu'on croirait cassé.
    unreadable: bool,
}

impl ProtectionsModule {
    /// L'analyse, pour qui veut la lire sans passer par l'interface.
    pub fn scan(&self) -> Option<&Scan> {
        self.scan.as_ref()
    }
}

impl ReModule for ProtectionsModule {
    fn section(&self) -> ReSection {
        ReSection::File
    }

    fn id(&self) -> &'static str {
        "protections"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Protections du binaire"
        } else {
            "Binary protections"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        if ctx.binary.is_empty() {
            self.scan = None;
            self.unreadable = false;
            return;
        }
        self.scan = elf_scan::scan(ctx.binary);
        self.unreadable = self.scan.is_none();
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        let Some(scan) = &self.scan else {
            return Some(
                Vue::neuve().texte(
                    Fragment::dit(if self.unreadable {
                        ctx.t(
                            "Ce fichier n'est pas un ELF64 : rien à analyser ici.",
                            "This file is not an ELF64: nothing to analyse here.",
                        )
                    } else {
                        ctx.t("Aucun binaire chargé.", "No binary loaded.")
                    })
                    .faible(),
                ),
            );
        };

        // L'identité du fichier, d'abord : le reste ne se lit qu'en
        // sachant de quoi on parle.
        let point = || Fragment::dit("·").faible();
        let mut blocs = vec![Bloc::Texte(
            vec![
                Fragment::dit(match scan.kind {
                    Kind::Executable => ctx.t("Exécutable", "Executable"),
                    Kind::Shared => ctx.t("Exécutable relogeable", "Relocatable executable"),
                    Kind::Relocatable => ctx.t("Objet à lier", "Relocatable object"),
                    Kind::Core => ctx.t("Image mémoire (core)", "Core dump"),
                    Kind::Other(_) => ctx.t("Type inconnu", "Unknown type"),
                })
                .fort(),
                point(),
                Fragment::dit(scan.machine),
                point(),
                Fragment::dit(format!("{} {:#x}", ctx.t("entrée", "entry"), scan.entry)),
                point(),
                Fragment::dit(if scan.dynamic() {
                    ctx.t("lié dynamiquement", "dynamically linked")
                } else {
                    ctx.t("lié statiquement", "statically linked")
                }),
            ]
            .into(),
        )];

        // LES VERDICTS, DITS ET NON PEINTS.
        //
        // Ce module peignait une pastille ronde au `painter`, choisie
        // ici, avec ses deux couleurs écrites en dur ; « En bref »
        // peignait les mêmes faits avec un `√` et un `×`, et la fiche
        // du panneau latéral les mêmes glyphes dans un troisième vert.
        // Le module dit maintenant le nom, l'état, le verdict et le
        // pourquoi — la marque, la teinte, et la place où se lit le
        // pourquoi sont décidées une fois, dans le peintre.
        let dit = |nom: &str, état: &str, verdict: Ton, pourquoi: &str| Bloc::Verdict {
            nom: nom.to_owned().into(),
            état: état.to_owned().into(),
            verdict,
            pourquoi: Some(pourquoi.to_owned()),
        };
        let oui_non = |bon: bool| if bon { Ton::Bon } else { Ton::Mauvais };

        blocs.push(dit(
            ctx.t("PIE", "PIE"),
            if scan.pie() {
                ctx.t("oui", "yes")
            } else {
                ctx.t("non", "no")
            },
            oui_non(scan.pie()),
            if scan.pie() {
                ctx.t(
                    "Le programme est posé à une adresse différente à chaque \
                     exécution : les adresses lues ici ne seront pas celles de \
                     la prochaine fois.",
                    "The program lands at a different address every run: the \
                     addresses read here will not be next run's.",
                )
            } else {
                ctx.t(
                    "Les adresses sont écrites en dur : celles du désassemblage \
                     sont celles qu'il aura, à chaque exécution.",
                    "Addresses are fixed: the ones in the disassembly are the \
                     ones it will have, every run.",
                )
            },
        ));

        // TROIS ÉTATS ET NON DEUX. « Non déclaré » n'est pas « pile
        // exécutable » : le fichier ne dit rien, et c'est le noyau qui
        // tranche. Le peindre en rouge au même rang qu'une pile
        // franchement exécutable affirmait ce que personne ne sait.
        blocs.push(dit(
            "NX",
            match scan.nx {
                Some(true) => ctx.t("pile non exécutable", "non-executable stack"),
                Some(false) => ctx.t("pile exécutable", "executable stack"),
                None => ctx.t("non déclaré", "not declared"),
            },
            match scan.nx {
                Some(true) => Ton::Bon,
                Some(false) => Ton::Mauvais,
                None => Ton::Attention,
            },
            match scan.nx {
                Some(true) => ctx.t(
                    "Du code déposé sur la pile ne s'exécutera pas : il faut \
                     réutiliser celui du programme (ROP) plutôt qu'en injecter.",
                    "Code written onto the stack will not run: one has to reuse \
                     the program's own (ROP) instead of injecting any.",
                ),
                Some(false) => ctx.t(
                    "La pile est exécutable : des octets qu'on y écrit peuvent \
                     être exécutés tels quels.",
                    "The stack is executable: bytes written there can be run as \
                     they are.",
                ),
                None => ctx.t(
                    "Le fichier ne déclare pas de segment de pile : le noyau \
                     décide seul, et il ne l'a pas toujours rendue non \
                     exécutable.",
                    "The file declares no stack segment: the kernel decides \
                     alone, and it has not always made it non-executable.",
                ),
            },
        ));

        // Et de même ici : un RELRO partiel protège bel et bien une
        // partie des données, et se peignait pourtant comme absent.
        blocs.push(dit(
            "RELRO",
            match scan.relro {
                Relro::Full => ctx.t("complet", "full"),
                Relro::Partial => ctx.t("partiel", "partial"),
                Relro::None => ctx.t("absent", "none"),
            },
            match scan.relro {
                Relro::Full => Ton::Bon,
                Relro::Partial => Ton::Attention,
                Relro::None => Ton::Mauvais,
            },
            match scan.relro {
                Relro::Full => ctx.t(
                    "Les tables de liaison sont résolues au démarrage puis \
                     passées en lecture seule : une écriture arbitraire ne peut \
                     plus les détourner.",
                    "Linking tables are resolved at startup then made read-only: \
                     an arbitrary write can no longer hijack them.",
                ),
                Relro::Partial => ctx.t(
                    "Une partie des données est protégée, mais la table des \
                     sauts reste inscriptible — c'est elle qu'on détourne pour \
                     rediriger un appel de bibliothèque.",
                    "Some data is protected, but the jump table stays writable — \
                     that is what gets hijacked to redirect a library call.",
                ),
                Relro::None => ctx.t(
                    "Rien n'est repassé en lecture seule après le chargement.",
                    "Nothing is made read-only after loading.",
                ),
            },
        ));

        blocs.push(dit(
            ctx.t("Canari de pile", "Stack canary"),
            if scan.canary {
                ctx.t("présent", "present")
            } else {
                ctx.t("absent", "absent")
            },
            oui_non(scan.canary),
            if scan.canary {
                ctx.t(
                    "Une valeur secrète est posée avant l'adresse de retour et \
                     vérifiée avant de s'en servir : un débordement qui écrase \
                     l'une écrase l'autre, et le programme s'arrête.",
                    "A secret value sits before the return address and is checked \
                     before use: an overflow that smashes one smashes the other, \
                     and the program aborts.",
                )
            } else {
                ctx.t(
                    "Rien ne surveille l'adresse de retour : un débordement de \
                     tampon sur la pile la remplace sans que personne ne le voie.",
                    "Nothing watches the return address: a stack buffer overflow \
                     replaces it unnoticed.",
                )
            },
        ));

        blocs.push(dit(
            "IBT",
            if scan.cet.ibt {
                ctx.t("demandé", "requested")
            } else {
                ctx.t("non demandé", "not requested")
            },
            oui_non(scan.cet.ibt),
            if scan.cet.ibt {
                ctx.t(
                    "Un saut indirect ne peut atterrir que sur un `endbr64` : \
                     les morceaux de code choisis au milieu d'une instruction \
                     ne sont plus des cibles valables.",
                    "An indirect jump may only land on an `endbr64`: code \
                     fragments picked mid-instruction are no longer valid \
                     targets.",
                )
            } else {
                ctx.t(
                    "N'importe quelle adresse peut être la cible d'un saut \
                     indirect, y compris au milieu d'une instruction.",
                    "Any address can be the target of an indirect jump, \
                     including mid-instruction.",
                )
            },
        ));

        if scan.cet.shadow_stack {
            blocs.push(dit(
                ctx.t("Pile fantôme", "Shadow stack"),
                ctx.t("demandée", "requested"),
                Ton::Bon,
                ctx.t(
                    "Le processeur garde sa propre copie des adresses de retour \
                     et compare : écraser celle de la pile ne suffit plus.",
                    "The processor keeps its own copy of return addresses and \
                     compares: overwriting the one on the stack is no longer \
                     enough.",
                ),
            ));
        }

        if !scan.fortified.is_empty() {
            blocs.push(dit(
                "FORTIFY",
                &format!(
                    "{} {}",
                    scan.fortified.len(),
                    ctx.t("fonctions", "functions")
                ),
                Ton::Bon,
                ctx.t(
                    "Certaines fonctions de copie ont été remplacées par des \
                     versions qui connaissent la taille de la destination.",
                    "Some copy functions were replaced by versions that know the \
                     size of their destination.",
                ),
            ));
        }

        let rwx = scan.writable_executable();
        if !rwx.is_empty() {
            blocs.push(dit(
                ctx.t("Segment W+X", "W+X segment"),
                &rwx.iter()
                    .map(|s| format!("{:#x}", s.vaddr))
                    .collect::<Vec<_>>()
                    .join(", "),
                Ton::Mauvais,
                ctx.t(
                    "Une zone est à la fois inscriptible et exécutable : on peut \
                     y écrire du code et sauter dedans, sans rien contourner.",
                    "A region is both writable and executable: one can write code \
                     there and jump into it, with nothing to bypass.",
                ),
            ));
        }

        blocs.push(dit(
            ctx.t("Symboles", "Symbols"),
            &if scan.stripped {
                ctx.t("dépouillé", "stripped").to_owned()
            } else {
                scan.symbol_count.to_string()
            },
            oui_non(!scan.stripped),
            if scan.stripped {
                ctx.t(
                    "Les noms ont été retirés : les fonctions n'ont plus que \
                     leur adresse.",
                    "Names were removed: functions have nothing but their address.",
                )
            } else {
                ctx.t(
                    "Les noms sont là : les fonctions se lisent par leur nom \
                     plutôt que par leur adresse.",
                    "The names are there: functions read by name rather than by \
                     address.",
                )
            },
        ));

        // Ce que le chargeur ira chercher ailleurs : autant de code qui
        // s'exécutera dans le processus sans être dans le fichier.
        if !scan.needed.is_empty() {
            blocs.push(Bloc::Section {
                titre: ctx
                    .t("Bibliothèques réclamées", "Required libraries")
                    .into(),
                blocs: scan
                    .needed
                    .iter()
                    .map(|lib| Bloc::Texte(Fragment::dit(lib).code().into()))
                    .collect(),
            });
        }

        // UN CHEMIN DE RECHERCHE EST UN VERDICT, et il l'était déjà
        // sans le dire : le module écrivait la valeur puis, dessous, en
        // gris, pourquoi elle inquiète. C'est exactement la forme d'un
        // verdict — un nom, un état, une raison —, et le dire ainsi lui
        // donne la marque et la teinte des autres.
        for (nom, valeur) in [
            (ctx.t("RUNPATH", "RUNPATH"), &scan.runpath),
            (ctx.t("RPATH", "RPATH"), &scan.rpath),
        ] {
            if let Some(valeur) = valeur {
                blocs.push(Bloc::Verdict {
                    nom: nom.into(),
                    état: Fragment::dit(valeur).code().into(),
                    verdict: Ton::Attention,
                    pourquoi: Some(
                        ctx.t(
                            "Un chemin de recherche inscrit dans le binaire : qui peut \
                             écrire dedans peut y placer sa propre bibliothèque.",
                            "A search path baked into the binary: whoever can write there \
                             can drop their own library in it.",
                        )
                        .to_owned(),
                    ),
                });
            }
        }

        if let Some(build_id) = &scan.build_id {
            let survol = ctx.t(
                "Deux fichiers qui la partagent sont le même programme, \
                 compilé une seule fois.",
                "Two files sharing it are the same program, built once.",
            );
            blocs.push(Bloc::Texte(
                vec![
                    Fragment::dit(ctx.t("Empreinte de build", "Build ID")).survol(survol),
                    Fragment::dit(build_id).code().survol(survol),
                ]
                .into(),
            ));
        }

        // Les fonctions nommées : par où commencer la lecture.
        if !scan.functions.is_empty() {
            blocs.push(Bloc::Repliable {
                titre: format!(
                    "{} ({})",
                    ctx.t("Fonctions", "Functions"),
                    scan.functions.len()
                )
                .into(),
                ouvert: false,
                blocs: vec![Bloc::Tableau {
                    colonnes: Vec::new(),
                    rangées: scan
                        .functions
                        .iter()
                        .map(|function| {
                            vec![
                                Fragment::dit(format!("{:#x}", function.addr)).code().into(),
                                Fragment::dit(&function.name).code().into(),
                                Fragment::dit(if function.size > 0 {
                                    format!("{} {}", function.size, ctx.t("o", "B"))
                                } else {
                                    String::new()
                                })
                                .faible()
                                .into(),
                            ]
                        })
                        .collect(),
                    rayé: true,
                }],
            });
        }

        // Les sections : le découpage fin, celui que l'éditeur de liens
        // a choisi — plus précis que les segments, qui les regroupent
        // par droits.
        if !scan.sections.is_empty() {
            blocs.push(Bloc::Repliable {
                titre: format!(
                    "{} ({})",
                    ctx.t("Sections", "Sections"),
                    scan.sections.len()
                )
                .into(),
                ouvert: false,
                blocs: vec![Bloc::Tableau {
                    colonnes: Vec::new(),
                    rangées: scan
                        .sections
                        .iter()
                        .map(|section| {
                            vec![
                                Fragment::dit(&section.name).code().into(),
                                Fragment::dit(format!(
                                    "{}{}",
                                    if section.write { 'w' } else { '-' },
                                    if section.execute { 'x' } else { '-' }
                                ))
                                .code()
                                .into(),
                                Fragment::dit(if section.addr > 0 {
                                    format!("{:#x}", section.addr)
                                } else {
                                    String::new()
                                })
                                .code()
                                .into(),
                                Fragment::dit(format!("{} {}", section.size, ctx.t("o", "B")))
                                    .code()
                                    .into(),
                            ]
                        })
                        .collect(),
                    rayé: true,
                }],
            });
        }

        // Et la carte de ce qui sera projeté, droits compris : c'est
        // elle qui explique pourquoi telle adresse est lisible et telle
        // autre pas.
        blocs.push(Bloc::Section {
            titre: ctx.t("Segments", "Segments").into(),
            blocs: vec![Bloc::Tableau {
                colonnes: Vec::new(),
                rangées: scan
                    .segments
                    .iter()
                    .map(|segment| {
                        // Un segment inscriptible ET exécutable est ce
                        // qu'on cherche du regard ; il a sa ligne de
                        // verdict plus haut, et ici sa teinte.
                        let droits = segment.rights();
                        let inquiétant = droits.contains('w') && droits.contains('x');
                        vec![
                            Fragment::dit(segment.kind).code().into(),
                            if inquiétant {
                                Fragment::dit(droits).code().ton(Ton::Mauvais)
                            } else {
                                Fragment::dit(droits).code()
                            }
                            .into(),
                            Fragment::dit(format!("{:#x}", segment.vaddr)).code().into(),
                            Fragment::dit(format!("{} {}", segment.memsz, ctx.t("o", "B")))
                                .code()
                                .into(),
                        ]
                    })
                    .collect(),
                rayé: true,
            }],
        });
        Some(Vue { blocs })
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if let Some(vue) = self.vue(ctx) {
            crate::vue_peinte::peindre(ui, ctx, &vue);
        }
    }
}
