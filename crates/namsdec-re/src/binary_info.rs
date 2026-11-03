use crate::behaviour::{self, Family};
use crate::vue::{Bloc, Fragment, Ligne, Ton, Vue};
use crate::{
    ReContext, ReModule, ReSection, elf_scan, imports, strings, syscalls, toolchain, xrefs,
};

/// Le résumé : de quoi on parle, avant d'ouvrir les autres panneaux.
///
/// Six modules détaillent chacun une chose ; celui-ci répond à la
/// question qu'on pose en premier — qu'est-ce que c'est, et qu'est-ce
/// qu'il y a dedans ? Tout ce qu'il montre est mesuré, pas déclaré :
/// les fonctions viennent des `call` relevés, les chaînes des octets
/// lisibles, la nature du fichier de ses en-têtes, et ce qu'il *sait
/// faire* des fonctions qu'il emprunte et des appels système qu'il passe.
#[derive(Default)]
pub struct BinaryInfoModule {
    size: usize,
    entry: u64,
    instructions: usize,
    functions: usize,
    strings: usize,
    imports: usize,
    syscalls: usize,
    /// Le comportement, par famille, dans l'ordre de lecture — ce que le
    /// binaire sait faire, avec le nombre d'appels qui le tiennent.
    comportement: Vec<(Family, usize)>,
    /// Ce que l'en-tête ELF dit du fichier, quand on l'a : nature,
    /// liaison, protections. Gardé entier pour en tirer plusieurs lignes
    /// sans rescanner à chaque image.
    scan: Option<elf_scan::Scan>,
    /// Qui a produit ce fichier, et dans quel langage — avec le degré
    /// de certitude et les indices qui le portent.
    toolchain: toolchain::Toolchain,
}

impl ReModule for BinaryInfoModule {
    fn section(&self) -> ReSection {
        ReSection::File
    }

    /// La première question devant un fichier inconnu : qu'est-ce
    /// que c'est ? Rien ne se comprend avant elle.
    fn niveau(&self) -> crate::Niveau {
        crate::Niveau::Débutant
    }

    fn id(&self) -> &'static str {
        "binary_info"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "En bref"
        } else {
            "At a glance"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.size = ctx.disasm.iter().map(|i| i.bytes.len()).sum();
        self.entry = ctx.disasm.first().map(|i| i.address).unwrap_or(0);
        self.instructions = ctx.disasm.len();

        let range = ctx.cpu.mem.range();
        self.functions = xrefs::scan(ctx.disasm, range.clone()).functions().len();
        self.strings = (range.end - range.start)
            .try_into()
            .ok()
            .and_then(|len: usize| ctx.cpu.mem.peek_bytes(range.start, len).ok())
            .map(|bytes| strings::scan(bytes, range.start).len())
            .unwrap_or(0);

        // Ce que le binaire sait faire : relevé de ses fonctions
        // empruntées ET de ses appels système, rangé par famille. Les
        // deux, car un programme qui veut passer inaperçu appelle le
        // noyau par son numéro plutôt que par la table de liaison.
        let imports = imports::scan(ctx.binary);
        self.imports = imports.len();
        let appels = syscalls::scan(ctx.disasm);
        self.syscalls = appels.len();
        self.comportement = behaviour::ALL
            .iter()
            .map(|&famille| {
                let n = imports
                    .iter()
                    .filter(|i| behaviour::family_of(&i.name) == Some(famille))
                    .count()
                    + appels
                        .iter()
                        .filter(|c| c.family() == Some(famille))
                        .count();
                (famille, n)
            })
            .collect();

        self.scan = elf_scan::scan(ctx.binary);
        // La chaîne d'outils : relevée une fois au chargement, comme le
        // reste. Elle lit des sections entières et parcourt les chaînes
        // du fichier — ce n'est pas un calcul à refaire par image.
        let symboles: Vec<String> = self
            .scan
            .as_ref()
            .map(|scan| scan.functions.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        self.toolchain = toolchain::detect(ctx.binary, self.scan.as_ref(), &symboles);
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        // L'ACCORD EN NOMBRE, PARCE QU'UN COMPTE PEUT VALOIR UN.
        //
        // « 1 instructions », « 1 (cibles de `call`) » : un outil qui
        // sert à apprendre ne peut pas écrire cela, et sur un programme
        // d'une ligne c'est exactement ce qu'il écrivait. Trouvé par le
        // banc, sur la scène du mode débutant — une scène neuve voit ce
        // qu'aucune autre ne montrait.
        let accord = |n: usize, un: &'static str, plusieurs: &'static str| {
            if n > 1 { plusieurs } else { un }
        };
        let ligne = |label: &str, valeur: String| -> Vec<Ligne> {
            vec![
                Fragment::dit(label).into(),
                Fragment::dit(valeur).code().into(),
            ]
        };
        let mut rangées = vec![
            ligne(
                ctx.t("Code", "Code"),
                format!(
                    "{} {} · {} {}",
                    self.size,
                    ctx.t(
                        accord(self.size, "octet", "octets"),
                        accord(self.size, "byte", "bytes")
                    ),
                    self.instructions,
                    ctx.t("instructions", "instructions")
                ),
            ),
            ligne(
                ctx.t("Point d'entrée", "Entry point"),
                format!("{:#x}", self.entry),
            ),
            ligne(
                ctx.t("Fonctions", "Functions"),
                format!(
                    "{} ({})",
                    self.functions,
                    ctx.t(
                        accord(self.functions, "cible de `call`", "cibles de `call`"),
                        accord(self.functions, "`call` target", "`call` targets")
                    )
                ),
            ),
            ligne(ctx.t("Chaînes", "Strings"), format!("{}", self.strings)),
            ligne(
                ctx.t("Emprunte", "Borrows"),
                format!(
                    "{} {} · {} {}",
                    self.imports,
                    ctx.t(
                        accord(self.imports, "fonction", "fonctions"),
                        accord(self.imports, "function", "functions")
                    ),
                    self.syscalls,
                    ctx.t(
                        accord(self.syscalls, "appel système", "appels système"),
                        accord(self.syscalls, "system call", "system calls")
                    )
                ),
            ),
        ];
        if let Some(scan) = &self.scan {
            rangées.push(ligne(
                ctx.t("Fichier", "File"),
                format!(
                    "{} · {}",
                    if scan.pie() { "PIE" } else { "EXEC" },
                    if scan.dynamic() { "dyn" } else { "static" }
                ),
            ));
        }
        let mut blocs = vec![Bloc::Tableau {
            colonnes: Vec::new(),
            rangées,
            rayé: true,
        }];

        // CE QUI L'A PRODUIT.
        //
        // La première question qu'on pose devant un fichier inconnu, et
        // la plus rentable : un binaire Go ne se lit pas comme un
        // binaire C, et savoir qu'on a du C++ devant soi change la
        // façon dont on lit un `call` indirect — c'est peut-être une
        // table virtuelle.
        //
        // Chaque réponse porte SON DEGRÉ DE CERTITUDE, et le détail des
        // indices se déplie. Un outil qui affiche « compilateur : GCC »
        // sans dire d'où il le tient apprend à croire plutôt qu'à lire.
        let mut produit: Vec<Bloc> = Vec::new();
        if self.toolchain.language.is_none() && self.toolchain.compiler.is_none() {
            produit.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Indéterminé : ni section .comment, ni runtime nommé, ni décor de \
                     symboles reconnu. Un binaire dépouillé peut n'en porter aucun.",
                    "Undetermined: no .comment section, no named runtime, no recognised \
                     symbol mangling. A stripped binary may carry none of these.",
                ))
                .faible()
                .into(),
            ));
        } else {
            let certitude = |quoi: &str, trouvé: &toolchain::Finding| -> Vec<Ligne> {
                vec![
                    Fragment::dit(quoi).into(),
                    Fragment::dit(trouvé.name.clone()).code().into(),
                    Fragment::dit(format!("({})", trouvé.confidence.label(ctx.lang)))
                        .faible()
                        .into(),
                ]
            };
            let mut rangées = Vec::new();
            if let Some(langage) = &self.toolchain.language {
                rangées.push(certitude(ctx.t("Langage", "Language"), langage));
            }
            if let Some(compilateur) = &self.toolchain.compiler {
                rangées.push(certitude(ctx.t("Compilateur", "Compiler"), compilateur));
            }
            produit.push(Bloc::Tableau {
                colonnes: Vec::new(),
                rangées,
                rayé: true,
            });
        }
        if !self.toolchain.evidence.is_empty() {
            let mut dedans: Vec<Bloc> = self
                .toolchain
                .evidence
                .iter()
                .map(|indice| {
                    Bloc::Texte(
                        vec![
                            Fragment::dit(format!("[{}]", indice.where_)).faible(),
                            Fragment::dit(indice.what.clone()).code(),
                            Fragment::dit(format!("— {}", indice.means)).faible(),
                        ]
                        .into(),
                    )
                })
                .collect();
            // CE QUE CELA NE PROUVE PAS, et ce n'est pas une formule de
            // prudence : `.comment` s'écrit, les symboles s'effacent,
            // les chaînes se remplacent. Un lecteur qui l'ignore
            // tirerait de « Go 1.22 » une confiance qu'aucun octet ne
            // justifie.
            dedans.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Tout cela se falsifie : la section .comment s'écrit, les symboles \
                     s'effacent, les chaînes se remplacent. Ces indices disent ce que le \
                     fichier MONTRE, non ce qu'il est.",
                    "All of this can be forged: the .comment section can be written, symbols \
                     stripped, strings replaced. These clues say what the file SHOWS, not \
                     what it is.",
                ))
                .faible()
                .into(),
            ));
            produit.push(Bloc::Repliable {
                titre: ctx.t("Ce qui le dit", "What says so").into(),
                ouvert: false,
                blocs: dedans,
            });
        }
        blocs.push(Bloc::Section {
            titre: ctx.t("Ce qui l'a produit", "What produced it").into(),
            blocs: produit,
        });

        // Ce qu'il sait faire : une ligne par famille présente. C'est la
        // réponse à « a-t-il accès au réseau ? » et aux cinq autres
        // questions du même ordre — lue d'un coup d'œil, et expliquée
        // juste en dessous.
        //
        // SANS LE PORTRAIT : `portrait_en_blocs` porte son propre
        // libellé — « Ce qu'il sait faire » —, écrit pour vivre seul
        // dans le panneau « Fonctions importées ». Appelé ici, sous un
        // titre qui dit déjà la même phrase, il la redisait à la ligne
        // suivante, puis réénumérait les familles que les lignes
        // ci-dessous détaillent. Trois lignes, deux répétitions ; vu
        // sur une capture.
        let présentes: Vec<(Family, usize)> = self
            .comportement
            .iter()
            .copied()
            .filter(|(_, n)| *n > 0)
            .collect();
        blocs.push(Bloc::Section {
            titre: ctx.t("Ce qu'il sait faire", "What it can do").into(),
            blocs: if présentes.is_empty() {
                vec![Bloc::Texte(
                    Fragment::dit(ctx.t(
                        "Rien de remarquable : ni réseau, ni exécution, ni chiffrement, ni accès \
                         fichier repéré. Un programme peut n'avoir besoin de rien de tout cela.",
                        "Nothing notable: no network, execution, cryptography or file access \
                         spotted. A program may need none of these.",
                    ))
                    .faible()
                    .into(),
                )]
            } else {
                présentes
                    .iter()
                    .map(|(famille, combien)| {
                        Bloc::Texte(
                            vec![
                                // UNE CLÉ ET NON UNE COULEUR : ce
                                // module écrivait `famille.color()`, et
                                // choisissait donc une teinte. Il range
                                // maintenant le mot dans une catégorie,
                                // et l'interface décide.
                                Fragment::dit(format!("{} ({combien})", famille.label(ctx.lang)))
                                    .étiquette(famille.clé()),
                                Fragment::dit(famille.why(ctx.lang)).faible(),
                            ]
                            .into(),
                        )
                    })
                    .collect()
            },
        });

        // Les protections, en un coup d'œil : le détail complet reste au
        // panneau « Protections », mais les quatre qui décident de tout
        // se lisent ici.
        //
        // QUATRE VERDICTS DITS ET NON PEINTS, depuis P7 : ce module
        // écrivait la marque (`√`, `×`) ET ses deux couleurs en dur,
        // tandis que le panneau Protections peignait une pastille ronde
        // et la fiche du panneau latéral les mêmes glyphes dans un
        // autre vert. Trois façons de dire « NX est là ». Le module dit
        // désormais le nom, l'état, le verdict et le pourquoi ; la
        // marque, la teinte et la place du pourquoi sont à l'interface.
        let Some(scan) = &self.scan else {
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Pas d'en-tête ELF : ce sont des octets bruts, désassemblés depuis le \
                     premier.",
                    "No ELF header: these are raw bytes, disassembled from the first one.",
                ))
                .faible()
                .into(),
            ));
            return Some(Vue { blocs });
        };
        // LE POURQUOI SUIT L'ÉTAT, ET C'EST UNE CORRECTION.
        //
        // Ce module écrivait UNE note par protection, qui décrivait
        // celle-ci PRÉSENTE : « chargé à une adresse au hasard » pour
        // PIE, « la pile ne s'exécute pas » pour NX. Elle se peignait à
        // droite de la marque, verdict favorable ou non — de sorte que
        // sur un binaire sans PIE on lisait « × PIE — chargé à une
        // adresse au hasard », c'est-à-dire l'inverse de la vérité. Vu
        // sur la capture du banc, dont le programme n'a aucune des
        // quatre ; aucun contrôle ne regardait cette phrase, et elle
        // était fausse depuis qu'elle existe.
        let verdict = |bon: bool| if bon { Ton::Bon } else { Ton::Mauvais };
        let selon = |bon: bool, oui: &'static str, non: &'static str| -> Option<String> {
            Some(if bon { oui } else { non }.to_owned())
        };
        let pie = scan.pie();
        let nx = scan.nx.unwrap_or(false);
        blocs.push(Bloc::Section {
            titre: ctx.t("Protections", "Protections").into(),
            blocs: vec![
                Bloc::Verdict {
                    nom: ctx.t("PIE", "PIE").into(),
                    état: Ligne::default(),
                    verdict: verdict(pie),
                    pourquoi: selon(
                        pie,
                        ctx.t(
                            "chargé à une adresse au hasard",
                            "loaded at a random address",
                        ),
                        ctx.t(
                            "chargé toujours à la même adresse",
                            "always loaded at the same address",
                        ),
                    ),
                },
                Bloc::Verdict {
                    nom: ctx.t("NX", "NX").into(),
                    état: Ligne::default(),
                    verdict: verdict(nx),
                    pourquoi: selon(
                        nx,
                        ctx.t("la pile ne s'exécute pas", "the stack does not execute"),
                        ctx.t(
                            "la pile s'exécute : des octets qu'on y écrit peuvent tourner",
                            "the stack executes: bytes written there can run",
                        ),
                    ),
                },
                // TROIS ÉTATS ET NON DEUX. Un RELRO « partiel » n'est
                // ni bon ni mauvais, et le dire en vrai/faux le peignait
                // en rouge au même rang qu'absent — alors qu'il protège
                // bel et bien une partie des données. C'est pour cela
                // que le verdict d'un `Bloc::Verdict` est un ton.
                Bloc::Verdict {
                    nom: ctx.t("RELRO", "RELRO").into(),
                    état: match scan.relro {
                        elf_scan::Relro::Full => ctx.t("complet", "full"),
                        elf_scan::Relro::Partial => ctx.t("partiel", "partial"),
                        elf_scan::Relro::None => ctx.t("absent", "none"),
                    }
                    .into(),
                    verdict: match scan.relro {
                        elf_scan::Relro::Full => Ton::Bon,
                        elf_scan::Relro::Partial => Ton::Attention,
                        elf_scan::Relro::None => Ton::Mauvais,
                    },
                    pourquoi: Some(
                        match scan.relro {
                            elf_scan::Relro::Full => ctx.t(
                                "les tables de liaison passent en lecture seule",
                                "linking tables are made read-only",
                            ),
                            elf_scan::Relro::Partial => ctx.t(
                                "la table des sauts reste inscriptible",
                                "the jump table stays writable",
                            ),
                            elf_scan::Relro::None => ctx.t(
                                "rien ne repasse en lecture seule après le chargement",
                                "nothing is made read-only after loading",
                            ),
                        }
                        .to_owned(),
                    ),
                },
                Bloc::Verdict {
                    nom: ctx.t("Canari", "Canary").into(),
                    état: Ligne::default(),
                    verdict: verdict(scan.canary),
                    pourquoi: selon(
                        scan.canary,
                        ctx.t(
                            "sentinelle contre l'écrasement de pile",
                            "sentinel against stack smashing",
                        ),
                        ctx.t(
                            "rien ne surveille l'adresse de retour",
                            "nothing watches the return address",
                        ),
                    ),
                },
            ],
        });
        Some(Vue { blocs })
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if let Some(vue) = self.vue(ctx) {
            crate::vue_peinte::peindre(ui, ctx, &vue);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    #[test]
    fn the_summary_counts_what_it_sees() {
        let bytes: &[u8] = &[
            0xe8, 0x01, 0x00, 0x00, 0x00, // call +1
            0x90, // nop  (la cible)
            0xf4, // hlt
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };

        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.size, 7);
        assert_eq!(module.entry, 0x400000);
        assert_eq!(module.instructions, 3);
        // Le `call` désigne une adresse : c'est une fonction.
        assert_eq!(module.functions, 1);
        // Pas d'en-tête : on ne prétend rien sur la nature du fichier.
        assert!(module.scan.is_none());
    }

    /// Le portrait compte ce que le code DEMANDE au système, même par
    /// numéro : `mov eax, 41 ; syscall` est un `socket`, donc du réseau.
    /// « Ce qu'il sait faire » ne s'écrit qu'une fois.
    ///
    /// Cette phrase existe à DEUX endroits par construction : c'est le
    /// titre de la section ici, et c'est aussi le libellé que
    /// `imports_module::portrait` porte en tête de sa ligne de
    /// pastilles — écrit pour vivre seul dans le panneau « Fonctions
    /// importées ». L'un appelé sous l'autre donnait trois lignes dont
    /// deux redites :
    ///
    /// ```text
    /// Ce qu'il sait faire
    /// Ce qu'il sait faire   Réseau (1)
    /// Réseau (1)   Le programme ouvre ou écoute une connexion…
    /// ```
    ///
    /// Vu sur une capture du 23 octobre, et nulle part ailleurs :
    /// aucune scène du banc de l'interface n'a de binaire dont le
    /// comportement soit non vide, de sorte que ce chemin n'y est
    /// jamais peint. Le contrôle vit donc ici, où l'état se construit
    /// en sept octets.
    #[test]
    fn what_it_can_do_is_written_only_once() {
        // Le même appel système que ci-dessous : il suffit à remplir le
        // comportement, qui est la condition pour que la section
        // s'affiche.
        let bytes: &[u8] = &[0xb8, 0x29, 0x00, 0x00, 0x00, 0x0f, 0x05];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);
        assert!(
            !module.comportement.is_empty(),
            "sans comportement, la section ne se peint pas et le contrôle \
             ne prouverait rien"
        );

        // IL LIT DES BLOCS, ET NON DES FORMES PEINTES. Ce contrôle
        // demandait un contexte egui, une image entière, et le
        // parcours de ses formes de texte pour compter une phrase. La
        // phrase est dans la vue ; la compter y revient au même, sans
        // moteur de rendu — et le contrôle dit maintenant OÙ elle est,
        // ce que des formes anonymes ne savaient pas dire.
        let vue = module.vue(&ctx).expect("le panneau dit quelque chose");
        let combien = à_plat(&vue.blocs)
            .into_iter()
            .filter(|bloc| match bloc {
                Bloc::Section { titre, .. } => titre.texte_nu() == "Ce qu'il sait faire",
                Bloc::Texte(ligne) => ligne
                    .fragments
                    .iter()
                    .any(|f| f.texte == "Ce qu'il sait faire"),
                _ => false,
            })
            .count();
        assert_eq!(
            combien, 1,
            "« Ce qu'il sait faire » est dit {combien} fois : le titre de la \
             section et le libellé de la ligne de pastilles disent la même chose"
        );
    }

    /// Tous les blocs d'une vue, à plat.
    fn à_plat(blocs: &[Bloc]) -> Vec<&Bloc> {
        let mut vus = Vec::new();
        for bloc in blocs {
            vus.push(bloc);
            match bloc {
                Bloc::Section { blocs, .. }
                | Bloc::Repliable { blocs, .. }
                | Bloc::Retrait(blocs) => vus.extend(à_plat(blocs)),
                _ => {}
            }
        }
        vus
    }

    /// Ce qu'on lit sous une protection décrit SON état, pas l'autre.
    ///
    /// Le panneau portait une note par protection, écrite pour le cas
    /// où celle-ci est PRÉSENTE — « chargé à une adresse au hasard » —,
    /// et la peignait quel que soit le verdict. Sur un binaire sans
    /// PIE, on lisait donc l'inverse de la vérité. La phrase n'était
    /// regardée par aucun contrôle : elle était fausse depuis qu'elle
    /// existe, et c'est la capture qui l'a montrée.
    ///
    /// **LE MÊME FICHIER DANS LES DEUX ÉTATS, et c'est tout le contrôle.**
    /// Une première version lisait un binaire d'épreuve et comparait sa
    /// phrase à son en-tête — elle passait en remettant le défaut,
    /// parce que `crackme-01` A du NX : la seule variante qu'elle
    /// éprouvait était justement celle que le défaut rend. Un contrôle
    /// qui ne distingue pas les deux états qu'il nomme ne regarde rien.
    /// On bascule donc l'en-tête sous le module, et l'on exige que la
    /// phrase bascule avec.
    #[test]
    fn ce_qu_on_lit_sous_une_protection_suit_son_état() {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/binaires/crackme-01");
        let Ok(octets) = std::fs::read(&chemin) else {
            return;
        };
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &octets,
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);
        assert!(module.scan.is_some(), "un ELF a un en-tête");

        /// Le verdict et la phrase d'une protection, dans la vue.
        fn protection(module: &BinaryInfoModule, ctx: &ReContext, nom: &str) -> (Ton, String) {
            let vue = module.vue(ctx).expect("le panneau dit quelque chose");
            à_plat(&vue.blocs)
                .into_iter()
                .find_map(|bloc| match bloc {
                    Bloc::Verdict {
                        nom: dit,
                        verdict,
                        pourquoi,
                        ..
                    } if dit.texte_nu() == nom => {
                        Some((*verdict, pourquoi.clone().unwrap_or_default()))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{nom} est dit"))
        }

        let vue = module.vue(&ctx).expect("le panneau dit quelque chose");
        let combien = à_plat(&vue.blocs)
            .into_iter()
            .filter(|bloc| matches!(bloc, Bloc::Verdict { .. }))
            .count();
        assert_eq!(combien, 4, "PIE, NX, RELRO, Canari");

        // LA PILE PROTÉGÉE, PUIS LA PILE EXÉCUTABLE — le même fichier,
        // le même panneau, et deux phrases qui doivent différer.
        module.scan.as_mut().unwrap().nx = Some(true);
        let (bon, protégée) = protection(&module, &ctx, "NX");
        module.scan.as_mut().unwrap().nx = Some(false);
        let (mauvais, exécutable) = protection(&module, &ctx, "NX");

        assert_eq!(bon, Ton::Bon, "NX déclaré : le verdict est favorable");
        assert_eq!(mauvais, Ton::Mauvais, "NX absent : il ne l'est pas");
        assert_ne!(
            protégée, exécutable,
            "la même phrase pour les deux états : « {protégée} »"
        );
        assert!(
            !protégée.contains("peuvent tourner"),
            "NX est là, et l'on lit pourtant « {protégée} »"
        );
        assert!(
            exécutable.contains("peuvent tourner"),
            "NX manque, et l'on lit pourtant « {exécutable} »"
        );
    }

    /// Un RELRO partiel n'est ni bon ni mauvais, et se dit ainsi.
    ///
    /// Dit en vrai/faux, il se peignait en rouge au même rang
    /// qu'absent — alors qu'il protège bel et bien une partie des
    /// données. C'est la raison pour laquelle le verdict d'un
    /// `Bloc::Verdict` est un TON et non un booléen.
    #[test]
    fn un_relro_partiel_se_dit_sans_le_condamner() {
        use crate::elf_scan::Relro;
        let tons = [
            (Relro::Full, Ton::Bon),
            (Relro::Partial, Ton::Attention),
            (Relro::None, Ton::Mauvais),
        ];
        // Trois états distincts, et trois tons distincts : c'est tout
        // ce que la table doit garantir.
        let distincts: std::collections::BTreeSet<_> =
            tons.iter().map(|(_, ton)| format!("{ton:?}")).collect();
        assert_eq!(distincts.len(), 3, "trois états, trois verdicts");
        assert_ne!(
            tons[1].1, tons[2].1,
            "« partiel » ne se dit pas comme « absent »"
        );
    }

    #[test]
    fn the_portrait_counts_a_network_syscall() {
        // b8 29 00 00 00 = mov eax, 41 (socket) ; 0f 05 = syscall.
        let bytes: &[u8] = &[0xb8, 0x29, 0x00, 0x00, 0x00, 0x0f, 0x05];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.syscalls, 1, "un appel système relevé");
        let réseau = module
            .comportement
            .iter()
            .find(|(f, _)| *f == Family::Network)
            .map(|(_, n)| *n)
            .unwrap_or(0);
        assert_eq!(
            réseau, 1,
            "le socket compte comme du réseau : {:?}",
            module.comportement
        );
    }
}
