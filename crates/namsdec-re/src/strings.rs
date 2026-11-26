use crate::vue::{Bloc, Fragment, Ligne, Vue};
use crate::{ReContext, ReModule, ReSection, framed_small_button};

/// Recopier une rubrique d'indicateurs d'un clic, par sa clé.
const ACTION_COPIER: &str = "chaînes:copier:";

/// Les chaînes lisibles trouvées dans la mémoire du programme.
///
/// C'est la première chose qu'on cherche dans un binaire qu'on n'a pas
/// écrit : un message d'erreur, un nom de fichier, une clé. Elles disent
/// souvent ce que le code fait avant qu'on ait lu une seule instruction.
#[derive(Default)]
pub struct StringsModule {
    found: Vec<Found>,
    /// Les chaînes que le code découpe lui-même : adresse **et**
    /// longueur, prises dans les instructions qui s'en servent.
    literals: Vec<crate::literals::Literal>,
    /// Qui désigne quoi : c'est ce qui fait la différence entre une
    /// liste de chaînes et une piste à suivre.
    xrefs: crate::xrefs::Xrefs,
    /// Ce qu'on cherche parmi elles. Deux cents chaînes se parcourent
    /// mal ; on sait en général ce qu'on veut y trouver.
    query: String,
    /// Les indicateurs tirés des chaînes : ce qu'on recopiera dans un
    /// rapport. Calculés une fois au chargement, comme le reste.
    iocs: Vec<crate::ioc::Indicator>,
    /// N'afficher que celles que le code désigne.
    ///
    /// Une chaîne à laquelle personne ne fait référence est souvent du
    /// texte mort — un reste de bibliothèque, un nom de section. Celles
    /// qu'une instruction va chercher sont, elles, des pistes.
    pointed_only: bool,
    /// Le texte à poser dans le presse-papier, quand on l'a demandé.
    à_copier: Option<String>,
}

impl StringsModule {
    /// Les indicateurs trouvés, pour qui veut les lire sans l'interface
    /// — le rapport s'en sert.
    pub fn indicators(&self) -> &[crate::ioc::Indicator] {
        &self.iocs
    }
}

/// Une chaîne trouvée : où elle commence, et ce qu'elle dit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub addr: u64,
    pub text: String,
}

/// En dessous de quatre caractères, tout ressemble à une chaîne : deux
/// octets imprimables au milieu d'un nombre suffiraient.
const MIN_LEN: usize = 4;

/// De quoi remplir un panneau sans le noyer. Un binaire entier en contient
/// des milliers, et ce n'est pas une liste de mille entrées qu'on lit.
const MAX_FOUND: usize = 200;

/// Les chaînes d'un ELF, cherchées **là où elles se trouvent**.
///
/// Un binaire commence par du code, et du code contient à foison des
/// suites d'octets imprimables qui ne veulent rien dire : `[A\A]A^A_]`
/// n'est pas une chaîne, c'est une fin de fonction. Chercher depuis le
/// premier octet remplissait donc le panneau de bruit et s'arrêtait
/// avant d'atteindre `.rodata`, là où sont les vrais messages.
///
/// On ne lit donc que les sections qui portent des données : ni le code,
/// ni celles qui n'occupent rien dans le fichier. C'est ce que fait
/// `strings -d`, et pour la même raison.
///
/// Rend `None` quand ce n'est pas un ELF : l'appelant retombe alors sur
/// [`scan`], qui lit tout.
pub fn scan_elf(bytes: &[u8]) -> Option<Vec<Found>> {
    let elf = crate::elf_scan::scan(bytes)?;
    let mut out = Vec::new();
    for section in &elf.sections {
        if section.execute || section.nobits || section.size == 0 {
            continue;
        }
        let début = section.offset as usize;
        let fin = début.checked_add(section.size as usize)?;
        let Some(contenu) = bytes.get(début..fin) else {
            continue;
        };
        // L'adresse virtuelle quand la section en a une ; sinon sa
        // position dans le fichier, qui reste un repère utilisable.
        let base = if section.addr != 0 {
            section.addr
        } else {
            section.offset
        };
        for found in scan(contenu, base) {
            out.push(found);
            if out.len() >= MAX_FOUND {
                return Some(out);
            }
        }
    }
    Some(out)
}

/// Cherche les suites d'octets imprimables dans `bytes`, avec l'adresse
/// où chacune commence.
///
/// Fonction libre et non méthode : c'est elle qu'on teste, et elle n'a
/// besoin de rien d'autre que d'octets.
pub fn scan(bytes: &[u8], base: u64) -> Vec<Found> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut start = base;
    for (offset, byte) in bytes.iter().enumerate() {
        // Imprimable au sens de l'ASCII, plus la tabulation : le reste
        // couperait les chaînes en deux ou les remplirait de carrés.
        let printable = (0x20..0x7f).contains(byte) || *byte == b'\t';
        if printable {
            if current.is_empty() {
                // `base` vient du fichier (l'adresse d'une section) et
                // `offset` du parcours : leur somme débordait en *debug*
                // sur un en-tête qui annonce une adresse proche de
                // `u64::MAX`. Une chaîne dont l'adresse déborde existe
                // tout de même dans le fichier ; elle est gardée, avec
                // l'adresse repliée, plutôt que perdue.
                start = base.wrapping_add(offset as u64);
            }
            current.push(*byte as char);
            continue;
        }
        if current.len() >= MIN_LEN {
            out.push(Found {
                addr: start,
                text: std::mem::take(&mut current),
            });
            if out.len() >= MAX_FOUND {
                return out;
            }
        } else {
            current.clear();
        }
    }
    if current.len() >= MIN_LEN {
        out.push(Found {
            addr: start,
            text: current,
        });
    }
    out
}

impl ReModule for StringsModule {
    fn section(&self) -> ReSection {
        ReSection::Says
    }

    /// Ce que le programme dit en clair : le premier endroit où
    /// l'on trouve quelque chose, et souvent le plus parlant.
    fn niveau(&self) -> crate::Niveau {
        crate::Niveau::Débutant
    }

    fn id(&self) -> &'static str {
        "strings"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Chaînes lisibles"
        } else {
            "Readable strings"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        let range = ctx.cpu.mem.range();
        let len = (range.end - range.start) as usize;
        // Le fichier d'abord, quand on l'a : ses sections disent où sont
        // les données, et chercher ailleurs ne rapporte que du code qui
        // ressemble à du texte. À défaut — un tampon brut, un programme
        // qu'on vient d'assembler — on lit la mémoire telle quelle.
        self.found = match scan_elf(ctx.binary) {
            Some(trouvées) => {
                // Les adresses viennent des sections — des adresses
                // virtuelles. Le listing, lui, part parfois du premier
                // octet du fichier : il faut alors les ramener là où le
                // code les désigne, sans quoi aucune référence croisée
                // ne tomberait juste.
                let base = ctx.disasm.first().map(|first| first.address);
                trouvées
                    .into_iter()
                    .map(|found| match base {
                        Some(base) if !range.contains(&found.addr) => {
                            match crate::imports::file_offset(ctx.binary, found.addr) {
                                Some(offset) => Found {
                                    addr: base + offset,
                                    ..found
                                },
                                None => found,
                            }
                        }
                        _ => found,
                    })
                    .collect()
            }
            None => match ctx.cpu.mem.peek_bytes(range.start, len) {
                Ok(bytes) => scan(bytes, range.start),
                Err(_) => Vec::new(),
            },
        };
        // « Mot de passe » quelque part dans les données ne dit rien ;
        // « mot de passe, lu ici » dit par où entrer.
        self.xrefs = crate::xrefs::scan(ctx.disasm, range);
        // Et les chaînes que le **code** découpe, pour les langages qui
        // ne terminent pas les leurs par un zéro : sans elles, un
        // binaire Rust ou Go ne montre que des pâtés illisibles.
        self.literals = match crate::elf_scan::scan(ctx.binary) {
            Some(elf) => crate::literals::scan(ctx.disasm, &elf.sections, ctx.binary),
            None => Vec::new(),
        };
        // Et ce qu'il y a de vérifiable là-dedans : adresses, chemins,
        // clés. Les chaînes découpées par le code comptent autant que
        // les autres — en Rust et en Go, c'est là que se trouve l'URL.
        let mut tout: Vec<Found> = self.found.clone();
        tout.extend(self.literals.iter().map(|littérale| Found {
            addr: littérale.addr,
            text: littérale.text.clone(),
        }));
        self.iocs = crate::ioc::from_strings(&tout);
    }

    /// Tout ce que ce panneau montre, dit sans savoir qui le peint.
    ///
    /// **ELLE EST COMPLÈTE, ET C'EST CE QUI COMPTE** : un rapport ou
    /// un export reçoit les indicateurs, les chaînes découpées par le
    /// code et la liste entière. Ce que `ui` ajoute par-dessus — deux
    /// zones qui défilent et la rangée de filtre — ne change pas ce
    /// qui est montré, seulement combien on en voit d'un coup.
    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        let mut blocs = self.blocs_des_indicateurs(ctx);
        blocs.extend(self.blocs_des_littérales(ctx));
        if self.found.is_empty() {
            blocs.push(Bloc::Texte(
                ctx.t("Aucune chaîne trouvée.", "No string found.").into(),
            ));
            return Some(Vue { blocs });
        }
        blocs.push(Bloc::Texte(self.compte(ctx).into()));
        blocs.extend(self.blocs_des_chaînes(ctx));
        Some(Vue { blocs })
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        // Les indicateurs tout en haut : parmi deux cents chaînes, ce
        // sont les cinq qu'on recopiera. Ouverts d'office quand il y en
        // a — c'est la réponse qu'on venait chercher.
        let indicateurs = self.blocs_des_indicateurs(ctx);
        if !indicateurs.is_empty() {
            let gestes = crate::vue_peinte::peindre_tout(ui, ctx, &Vue { blocs: indicateurs });
            for action in &gestes.actions {
                self.agit(action);
            }
            ui.add_space(4.0);
        }

        // CE QUE LA PLACE OFFRE, ET NON UN CHIFFRE.
        //
        // Deux cent vingt pixels en dur : la liste s'arrêtait à onze
        // lignes et laissait deux cent cinquante pixels de vide
        // dessous, dans une fenêtre qui en tenait quarante. On faisait
        // défiler treize chaînes dans un hublot, devant un écran libre.
        // Vu sur une capture. Le plancher garde l'ancien comportement
        // là où la place manque : ce module se montre aussi dans des
        // cadres courts.
        //
        // LA ZONE QUI DÉFILE RESTE ICI, et c'est une décision de P7 :
        // une vue dit CE QUI EST MONTRÉ, pas combien on en voit à la
        // fois. Lui apprendre les hublots en ferait un second moteur de
        // mise en page — et un rapport, lui, ne défile pas.
        let littérales = self.blocs_des_littérales(ctx);
        if !littérales.is_empty() {
            egui::ScrollArea::vertical()
                .id_salt("strings_literals_scroll")
                .max_height(ui.available_height().max(220.0))
                .show(ui, |ui| {
                    crate::vue_peinte::peindre(ui, ctx, &Vue { blocs: littérales });
                });
            ui.add_space(4.0);
        }

        if self.found.is_empty() {
            crate::vue_peinte::peindre(
                ui,
                ctx,
                &Vue::neuve().texte(ctx.t("Aucune chaîne trouvée.", "No string found.")),
            );
            return;
        }

        // LA RANGÉE DE FILTRE RESTE PEINTE, et c'est l'autre décision.
        // La case à cocher est la SEULE de toute la frontière — celle
        // de l'éditeur binaire est dans le module qui reste peint —, et
        // `vue.rs` dit en tête pourquoi une forme à usage unique est un
        // signal d'arrêt. Une case qu'on coche est de l'interaction, et
        // non de la compréhension : ce qu'un rapport veut de ce
        // panneau, ce sont les chaînes, pas le réglage qui a servi à
        // les trier.
        ui.horizontal(|ui| {
            crate::aligne_la_rangée(ui);
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(ctx.t("chercher…", "search…"))
                    .desired_width(140.0),
            );
            if !self.query.is_empty() && framed_small_button(ui, "x").clicked() {
                self.query.clear();
            }
            ui.checkbox(
                &mut self.pointed_only,
                ctx.t("désignées seulement", "pointed at only"),
            )
            .on_hover_text(ctx.t(
                "Une chaîne que personne ne va chercher est souvent du texte mort.",
                "A string nobody goes looking for is usually dead text.",
            ));
        });

        let mut blocs = vec![Bloc::Texte(self.compte(ctx).into())];
        blocs.extend(self.blocs_des_chaînes(ctx));
        egui::ScrollArea::vertical()
            .id_salt("re_strings")
            .max_height(ui.available_height().max(220.0))
            // Toute la largeur du panneau : sans cela, la zone se réduit
            // à la plus longue ligne *déjà* affichée, et les chaînes
            // suivantes se tronquent bien avant le bord.
            .auto_shrink([false, false])
            .show(ui, |ui| {
                crate::vue_peinte::peindre(ui, ctx, &Vue { blocs });
            });

        if let Some(texte) = self.à_copier.take() {
            ui.ctx().copy_text(texte);
        }
    }
}

impl StringsModule {
    /// Les indicateurs, par nature, et prêts à être recopiés.
    ///
    /// Rangés par nature et non par adresse : un rapport se tient par
    /// rubriques — « les adresses contactées », « les chemins écrits »
    /// — et c'est sous cette forme qu'on les recopie. L'adresse de la
    /// chaîne reste à côté, pour y retourner.
    fn blocs_des_indicateurs(&self, ctx: &ReContext) -> Vec<Bloc> {
        use crate::ioc;
        if self.iocs.is_empty() {
            return Vec::new();
        }
        // L'accord au singulier : « 1 indicateurs » se lit mal, et un
        // seul indicateur est le cas ordinaire d'un binaire du système.
        let seul = self.iocs.len() == 1;
        let mut dedans = vec![Bloc::Texte(
            Fragment::dit(ctx.t(
                "Une forme reconnue, non une intention : ce sont les seules chaînes qu'on \
                 puisse aller vérifier ailleurs — dans un journal réseau, sur un autre \
                 poste. Un chemin ou une adresse peut aussi n'être qu'un exemple compilé \
                 dans un message d'aide.",
                "A recognised shape, not an intent: these are the only strings one can go \
                 and check elsewhere — in a network log, on another machine. A path or an \
                 address may also be a mere example compiled into a help message.",
            ))
            .faible()
            .into(),
        )];
        for nature in ioc::ALL {
            let miens: Vec<&ioc::Indicator> =
                self.iocs.iter().filter(|i| i.kind == nature).collect();
            if miens.is_empty() {
                continue;
            }
            dedans.push(Bloc::Texte(
                vec![
                    Fragment::dit(nature.label(ctx.lang)).fort(),
                    Fragment::dit(format!("({})", miens.len())).faible(),
                    Fragment::dit(ctx.t("copier", "copy"))
                        .bouton(format!("{ACTION_COPIER}{}", nature.label("en")))
                        .survol(ctx.t(
                            "Toute la rubrique, une par ligne.",
                            "The whole heading, one per line.",
                        )),
                ]
                .into(),
            ));
            dedans.push(Bloc::Retrait(vec![Bloc::Tableau {
                colonnes: Vec::new(),
                rangées: miens
                    .iter()
                    .map(|indicateur| {
                        vec![
                            Fragment::dit(format!("{:08x}", indicateur.addr))
                                .code()
                                .into(),
                            Fragment::dit(&indicateur.value).code().into(),
                        ]
                    })
                    .collect(),
                rayé: false,
            }]));
        }
        vec![Bloc::Repliable {
            titre: format!(
                "{} {}",
                self.iocs.len(),
                match (ctx.lang.starts_with("fr"), seul) {
                    (true, true) => "indicateur à vérifier ailleurs",
                    (true, false) => "indicateurs à vérifier ailleurs",
                    (false, true) => "indicator to check elsewhere",
                    (false, false) => "indicators to check elsewhere",
                }
            )
            .into(),
            ouvert: true,
            blocs: dedans,
        }]
    }

    /// Ce que le code découpe lui-même : la vérité du programme, là où
    /// le scan des octets n'est qu'une supposition.
    fn blocs_des_littérales(&self, ctx: &ReContext) -> Vec<Bloc> {
        if self.literals.is_empty() {
            return Vec::new();
        }
        vec![Bloc::Repliable {
            titre: format!(
                "{} {}",
                self.literals.len(),
                ctx.t(
                    "chaînes découpées par le code",
                    "strings carved out by the code"
                )
            )
            .into(),
            ouvert: true,
            blocs: vec![
                Bloc::Texte(
                    Fragment::dit(ctx.t(
                        "En Rust, en Go, en Zig, une chaîne n'a pas de zéro final : c'est une \
                         adresse et une longueur. La découpe est donc dans le code, pas dans \
                         les données — « employée en » dit où aller lire.",
                        "In Rust, Go or Zig a string has no final zero: it is an address and \
                         a length. The cut is therefore in the code, not in the data — \
                         « used at » says where to go and read.",
                    ))
                    .faible()
                    .into(),
                ),
                Bloc::Tableau {
                    colonnes: Vec::new(),
                    rangées: self
                        .literals
                        .iter()
                        .map(|littérale| {
                            vec![
                                Fragment::dit(format!("{:#010x}", littérale.addr))
                                    .code()
                                    .into(),
                                Fragment::dit(format!("{:?}", littérale.text)).into(),
                                Fragment::dit(format!(
                                    "{} {:#010x}",
                                    ctx.t("employée en", "used at"),
                                    littérale.from
                                ))
                                .faible()
                                .into(),
                            ]
                        })
                        .collect(),
                    rayé: true,
                },
            ],
        }]
    }

    /// Les chaînes que le filtre laisse passer.
    fn montrées(&self) -> Vec<&Found> {
        let cherché = self.query.to_lowercase();
        self.found
            .iter()
            .filter(|found| cherché.is_empty() || found.text.to_lowercase().contains(&cherché))
            .filter(|found| !self.pointed_only || !self.xrefs.to(found.addr).is_empty())
            .collect()
    }

    /// Combien on en montre, et sur combien.
    fn compte(&self, ctx: &ReContext) -> String {
        if self.query.is_empty() && !self.pointed_only {
            format!(
                "{} {}",
                self.found.len(),
                ctx.t(
                    "chaînes d'au moins quatre caractères",
                    "strings of at least four characters"
                )
            )
        } else {
            format!(
                "{} / {} {}",
                self.montrées().len(),
                self.found.len(),
                ctx.t("chaînes", "strings")
            )
        }
    }

    /// La liste elle-même : l'adresse, qui la désigne, et le texte.
    fn blocs_des_chaînes(&self, ctx: &ReContext) -> Vec<Bloc> {
        vec![Bloc::Tableau {
            colonnes: Vec::new(),
            rangées: self
                .montrées()
                .iter()
                .map(|found| {
                    // Qui la désigne ? Une chaîne à laquelle personne
                    // ne fait référence est souvent du texte mort — un
                    // reste de bibliothèque, un nom de section.
                    let vers = self.xrefs.to(found.addr);
                    let désignée: Ligne = match vers.first() {
                        Some(première) => Fragment::dit(format!("<- {:#x}", première.from))
                            .faible()
                            .survol(if vers.len() > 1 {
                                format!("{} {}", vers.len(), ctx.t("références", "references"))
                            } else {
                                ctx.t("désignée ici", "pointed at here").to_owned()
                            })
                            .into(),
                        None => Ligne::default(),
                    };
                    vec![
                        Fragment::dit(format!("{:08x}", found.addr)).code().into(),
                        désignée,
                        // TRONQUÉE À LA LARGEUR QUI RESTE, et le module
                        // n'a plus à le dire : une chaîne peut faire
                        // trois cents caractères. Le peintre borne la
                        // colonne la plus large d'un tableau et rend le
                        // reste au survol.
                        Fragment::dit(&found.text).code().into(),
                    ]
                })
                .collect(),
            rayé: false,
        }]
    }

    /// Ce que le panneau fait de ce qu'on lui demande.
    ///
    /// Une seule chose pour l'instant : recopier une rubrique
    /// d'indicateurs. Posé dans `à_copier`, pris par `ui` — copier
    /// demande le contexte egui, que le comportement n'a pas, et c'est
    /// ce qui permet de l'éprouver sans peindre.
    fn agit(&mut self, action: &str) {
        let Some(clé) = action.strip_prefix(ACTION_COPIER) else {
            return;
        };
        let Some(nature) = crate::ioc::ALL.into_iter().find(|n| n.label("en") == clé) else {
            return;
        };
        self.à_copier = Some(
            self.iocs
                .iter()
                .filter(|i| i.kind == nature)
                .map(|i| i.value.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La liste des chaînes s'aligne à gauche, quelle que soit leur
    /// longueur.
    ///
    /// Elles flottaient au milieu du panneau, à des abscisses
    /// différentes selon leur longueur : `add_sized` centre son contenu
    /// dans la case qu'il alloue, et dire à l'étiquette de s'aligner à
    /// gauche ne suffisait pas à l'en empêcher. Sur un écran large, la
    /// liste devenait illisible — on cherchait chaque ligne au lieu de
    /// descendre la colonne.
    #[test]
    fn the_list_lines_up_on_the_left_whatever_the_length() {
        use namsdec_core::{Cpu, Memory};

        // Deux chaînes de longueurs très différentes, posées en
        // mémoire : c'est l'écart de longueur qui révélait le
        // centrage.
        let mut mémoire = Memory::new(0x400000, 0x1000);
        let données = b"court\0une chaine beaucoup plus longue que la premiere\0";
        mémoire.poke(0x400000, données).expect("la mémoire accepte");
        let cpu = Cpu::new(mémoire);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            // Pas d'ELF : le module lit alors la mémoire telle quelle.
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = StringsModule::default();
        module.on_binary_loaded(&ctx);
        assert!(
            module.found.len() >= 2,
            "les deux chaînes sont trouvées : {:?}",
            module.found
        );

        // Peintes dans une fenêtre large : c'est là que le centrage se
        // voyait.
        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            ..Default::default()
        };
        let mut posés: Vec<(String, f32)> = Vec::new();
        for _ in 0..2 {
            posés.clear();
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, &ctx));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    posés.push((texte.galley.text().to_owned(), texte.pos.x));
                }
            }
            sortie.textures_delta.clear();
        }

        let où = |quoi: &str| -> f32 {
            posés
                .iter()
                .find(|(t, _)| t == quoi)
                .map(|(_, x)| *x)
                .unwrap_or_else(|| panic!("« {quoi} » doit être peinte : {posés:?}"))
        };
        let courte = où("court");
        let longue = où("une chaine beaucoup plus longue que la premiere");
        assert!(
            (courte - longue).abs() < 0.5,
            "deux chaînes de longueurs différentes commencent au même endroit : \
             {courte} et {longue}"
        );
    }

    /// Sur un vrai binaire, les chaînes cherchées dans les données
    /// valent celles qu'on lirait avec `strings -d` — et non les
    /// fragments de code qui leur ressemblent.
    ///
    /// C'est ce qui manquait : en partant du premier octet du fichier,
    /// les deux cents premières « chaînes » étaient des fins de
    /// fonctions, et la recherche s'arrêtait avant d'atteindre les vrais
    /// messages.
    #[test]
    fn a_real_binary_yields_its_data_not_its_code() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        let trouvées = scan_elf(&bytes).expect("un ELF");
        assert!(!trouvées.is_empty());
        // L'interpréteur dynamique est une chaîne, et elle est dans une
        // section de données : c'est le repère le plus sûr qui soit.
        assert!(
            trouvées.iter().any(|found| found.text.contains("ld-linux")),
            "les vraies chaînes ne sont pas atteintes"
        );
        // Et les fragments de code ne sont plus là : aucune adresse
        // trouvée ne tombe dans une section exécutable.
        let elf = crate::elf_scan::scan(&bytes).expect("un ELF");
        for found in &trouvées {
            for section in elf.sections.iter().filter(|s| s.execute) {
                let fin = section.addr + section.size;
                assert!(
                    !(section.addr..fin).contains(&found.addr),
                    "« {} » vient de {}, qui est du code",
                    found.text,
                    section.name
                );
            }
        }
    }

    /// Le filtre « désignées seulement » sépare les pistes du texte
    /// mort : une chaîne que personne ne va chercher n'apprend rien.
    #[test]
    fn showing_only_the_strings_the_code_points_at() {
        // lea rax, [rel 0x401000] ; hlt — le code désigne la première
        // chaîne, pas la seconde.
        let code = [0x48, 0x8d, 0x05, 0xf9, 0x0f, 0x00, 0x00, 0xf4];
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        let xrefs = crate::xrefs::scan(&disasm, 0x400000..0x402000);

        let trouvées = [
            Found {
                addr: 0x401000,
                text: "désignée".to_owned(),
            },
            Found {
                addr: 0x401100,
                text: "orpheline".to_owned(),
            },
        ];
        let désignées: Vec<&Found> = trouvées
            .iter()
            .filter(|found| !xrefs.to(found.addr).is_empty())
            .collect();
        assert_eq!(désignées.len(), 1, "{désignées:?}");
        assert_eq!(désignées[0].text, "désignée");
    }

    /// Une chaîne occupe la largeur qu'on lui donne, et se tronque au
    /// bord du panneau — pas trois centimètres avant.
    ///
    /// `truncate` seul coupe à la largeur *idéale* du texte et laisse le
    /// reste de la ligne vide : dans un panneau de quatre cents pixels,
    /// les chaînes s'arrêtaient au milieu.
    #[test]
    fn a_string_uses_the_width_it_is_given() {
        let mut module = StringsModule {
            found: vec![
                Found {
                    addr: 0x401000,
                    text: "abcd".to_owned(),
                },
                Found {
                    addr: 0x401010,
                    text: "une chaîne bien plus longue que la précédente, pour voir".to_owned(),
                },
            ],
            ..Default::default()
        };
        let mem = namsdec_core::Memory::new(0x400000, 0x100);
        let cpu = namsdec_core::Cpu::new(mem);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 600.0),
            )),
            ..Default::default()
        };
        let mut largeurs: Vec<(String, f32)> = Vec::new();
        for _ in 0..2 {
            largeurs.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| {
                let re = ReContext {
                    cpu: &cpu,
                    disasm: &[],
                    source: "",
                    binary: &[],
                    lang: "fr",
                    names: None,
                    patches: &[],
                };
                module.ui(ui, &re);
            });
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(forme) = &clipped.shape {
                    let texte = forme.galley.text().to_owned();
                    if texte.starts_with("abcd") || texte.starts_with("une chaîne") {
                        largeurs.push((texte, forme.galley.size().x));
                    }
                }
            }
            output.textures_delta.clear();
        }
        let longue = largeurs
            .iter()
            .find(|(texte, _)| texte.starts_with("une chaîne"))
            .unwrap_or_else(|| panic!("la longue chaîne n'est pas peinte : {largeurs:?}"));
        // Elle doit occuper l'essentiel des quatre cents pixels, moins
        // l'adresse et les marges.
        assert!(
            longue.1 > 200.0,
            "la chaîne s'arrête à {} pixels sur 400",
            longue.1
        );
    }

    /// Ce qui n'est pas un ELF se lit quand même, du premier octet au
    /// dernier : un tampon brut n'a pas de sections à consulter.
    #[test]
    fn a_raw_buffer_is_still_read_whole() {
        assert!(scan_elf(b"pas un ELF, juste des octets").is_none());
        let trouvées = scan(b"\x00\x01salut tout le monde\x00", 0x1000);
        assert_eq!(trouvées.len(), 1);
        assert_eq!(trouvées[0].text, "salut tout le monde");
        assert_eq!(trouvées[0].addr, 0x1002);
    }

    #[test]
    fn a_run_of_printable_bytes_is_a_string() {
        let bytes = b"\x00\x01salut, monde\n\x00xy\x00mot de passe";
        let found = scan(bytes, 0x400000);
        let texts: Vec<&str> = found.iter().map(|f| f.text.as_str()).collect();
        // « xy » est trop court pour compter ; le saut de ligne coupe.
        assert_eq!(texts, ["salut, monde", "mot de passe"]);
        assert_eq!(
            found[0].addr, 0x400002,
            "l'adresse est celle du premier octet"
        );
    }

    #[test]
    fn a_program_full_of_text_does_not_fill_the_panel_for_ever() {
        let mut bytes = Vec::new();
        for _ in 0..(MAX_FOUND * 2) {
            bytes.extend_from_slice(b"abcd\x00");
        }
        assert_eq!(scan(&bytes, 0).len(), MAX_FOUND);
    }
}
