//! Un panneau par outil extérieur : Ghidra, RetDec, radare2, objdump,
//! ndisasm.
//!
//! La fenêtre d'analyse ne portait qu'une rubrique de décompilation,
//! celle de DZF — le moteur maison, encore en chantier. Les outils que
//! namsdec sait pourtant déclarer et lancer n'avaient, eux, qu'une
//! fenêtre à part, avec une liste déroulante : pour comparer ce que
//! Ghidra et RetDec disent d'une même fonction, il fallait relancer en
//! changeant de choix, et la sortie précédente était perdue.
//!
//! Ici, chacun a sa rubrique. On les lance l'un après l'autre, et l'on
//! revient lire celle qu'on veut : **chaque panneau garde sa sortie**.
//!
//! Trois règles, les mêmes que pour la fenêtre des décompilateurs, et
//! elles sont le sujet de ce module autant que le pseudo-code qu'il
//! affiche :
//!
//! - **Rien ne s'exécute tout seul.** Ouvrir la rubrique ne lance rien ;
//!   l'outil ne part qu'au clic sur « Lancer ».
//! - **La commande est annoncée avant d'être lancée**, en toutes
//!   lettres : c'est ce qu'on s'apprête à exécuter sur sa propre
//!   machine.
//! - **namsdec n'installe rien à votre place.** Si l'outil manque, il
//!   dit ce qu'il faut taper, et laisse taper.

use namsdec_disasm::known::{Availability, Known};

use crate::vue::{Bloc, Champ, Fragment, Ton, Vue};
use crate::{ReContext, ReModule, ReRequest, ReResult, ReSection};

/// Ce que le panneau demande, par son nom.
///
/// Des noms et non les demandes elles-mêmes : la vue reste du texte,
/// et c'est le module qui sait ce qu'« agir » veut dire chez lui.
const ACTION_LANCER: &str = "outil:lancer";
const ACTION_FILTRE: &str = "outil:filtre";
/// « outil:vider » et NON « outil:filtre:vider » : un champ validé
/// rend son action suivie de la valeur tapée, séparée par un
/// deux-points. Le second nom aurait donc été indiscernable du mot
/// « vider » cherché dans la sortie.
const ACTION_VIDER: &str = "outil:vider";
const ACTION_PAGE_AVANT: &str = "outil:page:avant";
const ACTION_PAGE_APRÈS: &str = "outil:page:après";

/// Combien de lignes une page de sortie porte.
///
/// La fenêtre d'analyse a sa propre zone de défilement et le panneau
/// n'en pose pas une seconde — deux zones imbriquées ne défilent bien
/// ni l'une ni l'autre. Un désassemblage d'objdump fait des dizaines de
/// milliers de lignes : on les peint donc par pages.
const PAR_PAGE: usize = 300;

/// Le panneau d'un outil extérieur.
pub struct ToolModule {
    /// La fiche de l'outil : son nom, sa commande, ses arguments, la
    /// façon de l'obtenir.
    known: Known,
    /// Le titre de la rubrique, dans les deux langues.
    titre_fr: &'static str,
    titre_en: &'static str,
    /// Le nom court, celui que la colonne de navigation écrit.
    court: &'static str,
    /// Ce que namsdec sait de l'outil sur cette machine. C'est
    /// l'application qui le dit — elle seule connaît le dossier où
    /// namsdec installe ce qu'il bâtit.
    disponible: Availability,
    /// Un travail est-il en cours ?
    en_cours: bool,
    /// Ce que l'outil a rendu, découpé en lignes.
    lignes: Vec<String>,
    /// Son refus, le cas échéant.
    erreur: Option<String>,
    /// Ce qu'on cherche dans la sortie.
    filtre: String,
    filtre_appliqué: String,
    /// Les rangs des lignes qui passent le filtre.
    visibles: Vec<usize>,
    /// La page qu'on regarde.
    page: usize,
    demande: Option<ReRequest>,
}

impl ToolModule {
    /// Le panneau d'un outil connu, avec le titre de sa rubrique.
    pub fn new(
        known: Known,
        titre_fr: &'static str,
        titre_en: &'static str,
        court: &'static str,
    ) -> Self {
        Self {
            known,
            titre_fr,
            titre_en,
            court,
            disponible: Availability::Missing,
            en_cours: false,
            lignes: Vec::new(),
            erreur: None,
            filtre: String::new(),
            filtre_appliqué: String::new(),
            visibles: Vec::new(),
            page: 0,
            demande: None,
        }
    }

    /// Le panneau de l'outil que namsdec connaît sous cet identifiant.
    ///
    /// `None` quand aucun ne porte ce nom : la liste des outils connus
    /// est une donnée, et une faute de frappe ne doit pas donner un
    /// panneau muet.
    pub fn connu(
        id: &str,
        titre_fr: &'static str,
        titre_en: &'static str,
        court: &'static str,
    ) -> Option<Self> {
        let known = namsdec_disasm::known::all()
            .into_iter()
            .find(|outil| outil.plugin.id == id)?;
        Some(Self::new(known, titre_fr, titre_en, court))
    }

    /// L'identifiant de l'outil que ce panneau lance.
    pub fn tool_id(&self) -> &str {
        &self.known.plugin.id
    }

    /// La sortie telle qu'elle est, pour qui veut la lire sans
    /// l'interface.
    pub fn lines(&self) -> &[String] {
        &self.lignes
    }

    /// La tranche de lignes qu'on regarde, et combien il y a de pages.
    ///
    /// EN LECTURE SEULE, et c'est ce qui a demandé le plus de soin en
    /// portant ce panneau : `ui()` écrivait `self.visibles`,
    /// `self.filtre_appliqué` et `self.page` PENDANT qu'il dessinait.
    /// Une vue n'emprunte `self` qu'en lecture ; filtrer et borner la
    /// page se font donc là où l'on agit, et non là où l'on décrit.
    fn fenêtre(&self) -> (usize, usize, usize) {
        let pages = self.visibles.len().div_ceil(PAR_PAGE).max(1);
        let page = self.page.min(pages - 1);
        let début = page * PAR_PAGE;
        let fin = (début + PAR_PAGE).min(self.visibles.len());
        (début, fin, pages)
    }

    /// Ce que le panneau fait de ce qu'on lui demande.
    ///
    /// À part de `ui()` pour être éprouvé sans peindre : filtrer et
    /// tourner une page sont des comportements, pas des dessins.
    fn agit(&mut self, action: &str) {
        if action == ACTION_LANCER {
            self.en_cours = true;
            self.erreur = None;
            self.demande = Some(ReRequest::RunTool {
                tool: self.known.plugin.id.clone(),
            });
            return;
        }
        if action == ACTION_VIDER {
            self.filtrer(String::new());
            return;
        }
        if let Some(cherché) = action.strip_prefix(&format!("{ACTION_FILTRE}:")) {
            self.filtrer(cherché.to_owned());
            return;
        }
        let (_, _, pages) = self.fenêtre();
        match action {
            ACTION_PAGE_AVANT => self.page = self.page.saturating_sub(1),
            ACTION_PAGE_APRÈS => self.page = (self.page + 1).min(pages - 1),
            _ => {}
        }
    }

    /// Ne garder que les lignes qui portent ce mot, et revenir en tête.
    fn filtrer(&mut self, cherché: String) {
        let bas = cherché.to_lowercase();
        self.visibles = if bas.is_empty() {
            (0..self.lignes.len()).collect()
        } else {
            self.lignes
                .iter()
                .enumerate()
                .filter(|(_, ligne)| ligne.to_lowercase().contains(&bas))
                .map(|(rang, _)| rang)
                .collect()
        };
        self.filtre = cherché;
        self.filtre_appliqué = self.filtre.clone();
        self.page = 0;
    }

    fn poser(&mut self, texte: &str) {
        self.lignes = texte.lines().map(str::to_owned).collect();
        self.erreur = None;
        self.filtre.clear();
        self.filtre_appliqué.clear();
        self.visibles = (0..self.lignes.len()).collect();
        self.page = 0;
    }
}

impl ReModule for ToolModule {
    fn section(&self) -> ReSection {
        ReSection::Decompile
    }

    fn id(&self) -> &'static str {
        // Un identifiant stable et statique, comme les autres panneaux.
        // Celui de l'outil est une `String` venue de sa fiche : on la
        // retrouve, et l'on rend la constante qui lui correspond.
        match self.known.plugin.id.as_str() {
            "objdump" => "tool_objdump",
            "ndisasm" => "tool_ndisasm",
            "radare2" => "tool_radare2",
            "rizin-ghidra" => "tool_ghidra",
            "retdec" => "tool_retdec",
            _ => "tool_autre",
        }
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.titre_fr
        } else {
            self.titre_en
        }
    }

    /// Le nom de l'outil, tout court : c'est ainsi qu'on le cherche
    /// dans la colonne, et le panneau garde son titre entier.
    fn nav_title(&self, _lang: &str) -> &'static str {
        self.court
    }

    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn on_binary_loaded(&mut self, _ctx: &ReContext) {
        // Un autre binaire : la sortie d'avant ne le concerne plus. La
        // laisser à l'écran ferait lire le désassemblage d'un programme
        // en croyant lire celui d'un autre.
        self.lignes.clear();
        self.visibles.clear();
        self.erreur = None;
        self.en_cours = false;
        self.page = 0;
    }

    fn deliver(&mut self, result: &ReResult) {
        match result {
            ReResult::ToolState { tool, availability } if *tool == self.known.plugin.id => {
                self.disponible = *availability;
            }
            ReResult::ToolOutput { tool, text } if *tool == self.known.plugin.id => {
                self.en_cours = false;
                match text {
                    Ok(texte) => self.poser(texte),
                    Err(message) => self.erreur = Some(message.clone()),
                }
            }
            _ => {}
        }
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        let fr = ctx.lang.starts_with("fr");
        let mut blocs = vec![
            Bloc::Texte(
                Fragment::dit(if fr {
                    self.known.about_fr
                } else {
                    self.known.about_en
                })
                .faible()
                .into(),
            ),
            // CE QU'ON S'APPRÊTE À LANCER, en toutes lettres. Un
            // panneau qui exécute un programme sur la machine de
            // quelqu'un doit pouvoir se lire avant d'être cliqué — et
            // la commande se COPIE, parce qu'on la colle dans un
            // terminal pour la voir faire.
            Bloc::Texte(
                vec![
                    Fragment::dit(ctx.t("La commande :", "The command:")).faible(),
                    Fragment::dit(format!(
                        "{} {}",
                        self.known.plugin.command,
                        self.known.plugin.args.join(" ")
                    ))
                    .code()
                    .copiable(),
                ]
                .into(),
            ),
            Bloc::Texte(
                Fragment::dit(ctx.t(
                    "`{file}` reçoit le binaire ouvert, par un fichier temporaire. Aucun \
                     interpréteur de commandes n'est en jeu.",
                    "`{file}` gets the open binary, through a temporary file. No shell is \
                     involved.",
                ))
                .faible()
                .into(),
            ),
        ];

        match self.disponible {
            Availability::Ready => {}
            Availability::Incomplete => {
                blocs.push(Bloc::Texte(
                    Fragment::dit(ctx.t(
                        "L'outil est là, mais il lui manque une pièce :",
                        "The tool is here, but a piece is missing:",
                    ))
                    .ton(Ton::Attention)
                    .into(),
                ));
                if let Some(manque) = if fr {
                    self.known.needs_fr
                } else {
                    self.known.needs_en
                } {
                    blocs.push(Bloc::Retrait(vec![Bloc::Texte(
                        Fragment::dit(manque).faible().into(),
                    )]));
                }
            }
            Availability::Missing => {
                blocs.push(Bloc::Texte(
                    Fragment::dit(ctx.t(
                        "Cet outil n'est pas sur cette machine.",
                        "This tool is not on this machine.",
                    ))
                    .ton(Ton::Attention)
                    .into(),
                ));
                if let Some(install) = &self.known.plugin.install {
                    blocs.push(Bloc::Texte(
                        vec![
                            Fragment::dit(ctx.t("Pour l'obtenir :", "To get it:")).faible(),
                            Fragment::dit(install).code().copiable(),
                        ]
                        .into(),
                    ));
                }
                blocs.push(Bloc::Texte(
                    Fragment::dit(ctx.t(
                        "namsdec ne l'installe pas à votre place : exécuter du code au nom de \
                         quelqu'un n'est pas une commodité, c'est une décision. Outils › \
                         Désassembleurs en dit plus.",
                        "namsdec does not install it for you: running code on someone's behalf \
                         is not a convenience, it is a decision. Tools › Disassemblers says \
                         more.",
                    ))
                    .faible()
                    .into(),
                ));
            }
        }

        let lançable = self.disponible != Availability::Missing && !self.en_cours;
        let mut commande = vec![
            Fragment::dit(if self.lignes.is_empty() {
                ctx.t("Lancer", "Run")
            } else {
                ctx.t("Relancer", "Run again")
            })
            .bouton(ACTION_LANCER)
            .inactif_si(!lançable)
            .survol(ctx.t(
                "Lancer cet outil sur le binaire ouvert, et lire sa sortie ici.",
                "Run this tool on the open binary, and read its output here.",
            )),
        ];
        if !self.lignes.is_empty() {
            commande.push(
                Fragment::dit(format!(
                    "{} {}",
                    self.lignes.len(),
                    ctx.t("lignes", "lines")
                ))
                .faible(),
            );
        }
        blocs.push(Bloc::Texte(commande.into()));
        if self.en_cours {
            blocs.push(Bloc::Attente(
                Fragment::dit(ctx.t(
                    "L'outil travaille — un décompilateur peut réfléchir une minute.",
                    "The tool is working — a decompiler may think for a minute.",
                ))
                .faible()
                .into(),
            ));
        }

        if let Some(message) = &self.erreur {
            blocs.push(Bloc::Texte(Fragment::dit(message).ton(Ton::Mauvais).into()));
        }
        if self.lignes.is_empty() {
            return Some(Vue { blocs });
        }

        // CHERCHER DANS LA SORTIE : un désassemblage d'`objdump` fait
        // cinquante mille lignes, et ce qu'on y cherche tient en un
        // mot.
        //
        // LE FILTRE S'APPLIQUE À LA VALIDATION, et non à chaque
        // frappe : c'est ce que `Bloc::Champ` permet — le tampon de
        // frappe vit chez celui qui peint, et le module n'apprend ce
        // qui est écrit qu'au moment où l'on valide. Sur cinquante
        // mille lignes, refiltrer à chaque lettre était de toute façon
        // du travail jeté à la touche suivante.
        let mut champ = Champ::pour(ACTION_FILTRE)
            .étiquette(Fragment::dit(ctx.t("Chercher :", "Find:")).faible())
            .valeur(&self.filtre)
            .indice(ctx.t("un nom, une adresse…", "a name, an address…"))
            .caractères(28);
        if !self.filtre.is_empty() {
            champ = champ.bouton(
                Fragment::dit("x")
                    .bouton(ACTION_VIDER)
                    .survol(ctx.t("Vider la recherche", "Clear the search")),
            );
        }
        blocs.push(Bloc::Champ(champ));

        let (début, fin, pages) = self.fenêtre();
        let mut barre = vec![if self.visibles.is_empty() {
            Fragment::dit(ctx.t("Aucune ligne ne correspond", "No line matches"))
        } else {
            Fragment::dit(format!(
                "{}–{} {} {}",
                début + 1,
                fin,
                ctx.t("sur", "of"),
                self.visibles.len()
            ))
        }];
        if pages > 1 {
            barre.push(
                Fragment::dit("\u{2039}")
                    .bouton(ACTION_PAGE_AVANT)
                    .inactif_si(self.page == 0)
                    .survol(ctx.t("Page précédente", "Previous page")),
            );
            barre.push(Fragment::dit(format!(
                "{} {} / {}",
                ctx.t("page", "page"),
                self.page + 1,
                pages
            )));
            barre.push(
                Fragment::dit("\u{203a}")
                    .bouton(ACTION_PAGE_APRÈS)
                    .inactif_si(self.page + 1 >= pages)
                    .survol(ctx.t("Page suivante", "Next page")),
            );
        }
        blocs.push(Bloc::Texte(barre.into()));

        // La sortie : du CODE, et c'est tout ce que le module en dit.
        // Les adresses qu'on y clique, la coloration et la largeur à
        // ne pas dépasser sont de la présentation — le peintre les
        // décide, pour ce panneau comme pour celui de la
        // décompilation.
        blocs.extend(
            self.visibles[début..fin]
                .iter()
                .map(|rang| Bloc::Code(self.lignes[*rang].clone())),
        );
        Some(Vue { blocs })
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let Some(vue) = self.vue(ctx) else {
            return;
        };
        let gestes = crate::vue_peinte::peindre_tout(ui, ctx, &vue);
        if gestes.demande.is_some() {
            self.demande = gestes.demande;
        }
        for action in &gestes.actions {
            self.agit(action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn contexte<'a>(cpu: &'a Cpu) -> ReContext<'a> {
        ReContext {
            cpu,
            disasm: &[],
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        }
    }

    fn peindre(module: &mut ToolModule, ctx: &ReContext) -> String {
        let egui_ctx = egui::Context::default();
        let mut textes: Vec<String> = Vec::new();
        for _ in 0..2 {
            textes.clear();
            let mut sortie = egui_ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ui| module.ui(ui, ctx),
            );
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    textes.push(texte.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        textes.join(" | ")
    }

    /// Chaque outil connu a son panneau, et son panneau dit ce qu'il
    /// lancerait — sans rien lancer.
    #[test]
    fn a_panel_says_what_it_would_run_without_running_it() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu(
            "objdump",
            "Désassemblage par objdump",
            "Disassembly by objdump",
            "objdump",
        )
        .expect("objdump est un outil connu");
        assert_eq!(module.id(), "tool_objdump");
        assert_eq!(module.title("fr"), "Désassemblage par objdump");
        assert_eq!(module.title("en"), "Disassembly by objdump");
        // La colonne, elle, écrit le nom tout court : vingt-cinq
        // rubriques sur deux cents pixels ne tiennent pas autrement.
        assert_eq!(module.nav_title("fr"), "objdump");

        let peint = peindre(&mut module, &ctx);
        // La commande est écrite avant d'être lancée.
        assert!(peint.contains("objdump -d -M intel"), "{peint}");
        // Et rien n'a été lancé : aucune sortie, aucune demande.
        assert!(module.lines().is_empty());
        assert!(module.demande.is_none());
    }

    /// Un outil absent le dit, et dit comment l'obtenir — plutôt que de
    /// laisser cliquer un bouton qui ne peut pas marcher.
    #[test]
    fn a_missing_tool_says_how_to_get_it() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu(
            "retdec",
            "Décompilation par RetDec",
            "Decompilation by RetDec",
            "RetDec",
        )
        .expect("RetDec est un outil connu");
        // L'état par défaut est « absent » : c'est l'application qui dit
        // le contraire, après avoir regardé le disque.
        let peint = peindre(&mut module, &ctx);
        assert!(peint.contains("n'est pas sur cette machine"), "{peint}");
        assert!(peint.contains("github.com/avast/retdec"), "{peint}");
    }

    /// Chercher et tourner les pages se font sans peindre.
    ///
    /// Le panneau écrivait `self.visibles`, `self.filtre_appliqué` et
    /// `self.page` PENDANT qu'il dessinait, de sorte que ces trois
    /// comportements ne s'éprouvaient qu'en rendant une image. Ils
    /// sont maintenant dans `agit`, et la vue n'emprunte `self` qu'en
    /// lecture.
    ///
    /// L'ACTION EST PROPOSÉE PAR LA VUE, et non inventée ici : c'est
    /// ce qui fait tomber le contrôle si le nom change d'un côté
    /// seulement.
    #[test]
    fn chercher_et_tourner_les_pages_se_font_sans_peindre() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu(
            "objdump",
            "Désassemblage par objdump",
            "Disassembly by objdump",
            "objdump",
        )
        .expect("objdump est un outil connu");
        module.deliver(&ReResult::ToolState {
            tool: "objdump".to_owned(),
            availability: Availability::Ready,
        });
        // Sept cents lignes : de quoi avoir trois pages de trois
        // cents, et de quoi que le filtre serve.
        let sortie: String = (0..700)
            .map(|n| {
                if n % 7 == 0 {
                    format!("  {n:>4}: 48 89 e5    mov rbp, rsp  ; aiguille\n")
                } else {
                    format!("  {n:>4}: 90          nop\n")
                }
            })
            .collect();
        module.deliver(&ReResult::ToolOutput {
            tool: "objdump".to_owned(),
            text: Ok(sortie),
        });
        assert_eq!(module.lines().len(), 700);

        /// Combien de lignes de code la vue montre, et ce qu'elle dit
        /// de la page.
        fn regarde(module: &ToolModule, ctx: &ReContext) -> (usize, Vec<String>) {
            let vue = module.vue(ctx).expect("le panneau dit quelque chose");
            let code = vue
                .blocs
                .iter()
                .filter(|bloc| matches!(bloc, Bloc::Code(_)))
                .count();
            let textes = vue
                .blocs
                .iter()
                .filter_map(|bloc| match bloc {
                    Bloc::Texte(ligne) => Some(ligne.texte_nu()),
                    _ => None,
                })
                .collect();
            (code, textes)
        }

        let (code, textes) = regarde(&module, &ctx);
        assert_eq!(code, PAR_PAGE, "une page pleine");
        assert!(
            textes
                .iter()
                .any(|t| t.contains("1–300") && t.contains("700")),
            "la barre dit où l'on est : {textes:?}"
        );

        // TOURNER UNE PAGE, par l'action que la vue propose.
        let vue = module.vue(&ctx).expect("le panneau dit quelque chose");
        let suivante = vue
            .blocs
            .iter()
            .filter_map(|bloc| match bloc {
                Bloc::Texte(ligne) => Some(ligne),
                _ => None,
            })
            .flat_map(|ligne| &ligne.fragments)
            .find(|f| f.texte == "\u{203a}")
            .and_then(|f| f.action.clone())
            .expect("la page suivante se demande");
        module.agit(&suivante);
        let (_, textes) = regarde(&module, &ctx);
        assert!(
            textes.iter().any(|t| t.contains("301–600")),
            "on a tourné : {textes:?}"
        );

        // CHERCHER. Cent lignes sur sept cents portent l'aiguille, et
        // la page revient en tête — rester à la page deux d'un
        // résultat qui n'en a qu'une montrerait un panneau vide.
        module.agit(&format!("{ACTION_FILTRE}:aiguille"));
        let (code, textes) = regarde(&module, &ctx);
        assert_eq!(code, 100, "une ligne sur sept");
        assert!(
            textes.iter().any(|t| t.contains("1–100")),
            "le filtre ramène en tête : {textes:?}"
        );

        // ET « VIDER » N'EST PAS UNE RECHERCHE. Le bouton porte son
        // propre nom ; taper le mot cherche le mot.
        module.agit(&format!("{ACTION_FILTRE}:vider"));
        let (code, _) = regarde(&module, &ctx);
        assert_eq!(code, 0, "aucune ligne ne porte « vider »");
        module.agit(ACTION_VIDER);
        let (code, _) = regarde(&module, &ctx);
        assert_eq!(code, PAR_PAGE, "le bouton rend toute la sortie");
    }

    /// Ce que l'outil rend arrive par le canal des résultats, et le
    /// panneau le garde : on lance Ghidra, puis RetDec, et l'on revient
    /// lire le premier.
    #[test]
    fn the_output_arrives_and_stays() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu(
            "radare2",
            "Décompilation par radare2",
            "Decompilation by radare2",
            "radare2",
        )
        .expect("radare2 est un outil connu");
        module.deliver(&ReResult::ToolState {
            tool: "radare2".to_owned(),
            availability: Availability::Ready,
        });
        // Ce qui s'adresse à un autre outil ne le concerne pas.
        module.deliver(&ReResult::ToolOutput {
            tool: "objdump".to_owned(),
            text: Ok("rien pour moi".to_owned()),
        });
        assert!(module.lines().is_empty());

        module.deliver(&ReResult::ToolOutput {
            tool: "radare2".to_owned(),
            text: Ok("entry0 ()\n    call 0x401030\n".to_owned()),
        });
        assert_eq!(module.lines().len(), 2);
        let peint = peindre(&mut module, &ctx);
        assert!(peint.contains("call 0x401030"), "{peint}");
        assert!(peint.contains("1–2 sur 2"), "{peint}");

        // Et un refus s'affiche tel quel, plutôt que de laisser le
        // panneau muet.
        module.deliver(&ReResult::ToolOutput {
            tool: "radare2".to_owned(),
            text: Err("l'outil a été arrêté au bout de 120 secondes".to_owned()),
        });
        let peint = peindre(&mut module, &ctx);
        assert!(peint.contains("arrêté au bout de"), "{peint}");
    }

    /// Une sortie aux lignes très longues reste dans le panneau.
    ///
    /// Une ligne d'`objdump` fait parfois deux cents caractères.
    /// Peinte sans borne, elle court hors du panneau — et la fenêtre
    /// d'analyse ne défile pas en largeur, donc ce qui dépasse est
    /// perdu, quand il ne fait pas enfler la fenêtre.
    #[test]
    fn a_very_long_line_stays_inside_the_panel() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(&cpu);
        let mut module = ToolModule::connu("objdump", "objdump", "objdump", "objdump")
            .expect("objdump est un outil connu");
        module.deliver(&ReResult::ToolState {
            tool: "objdump".to_owned(),
            availability: Availability::Ready,
        });
        let longue = format!(
            "  401030:\t{}\t{}",
            "48 8d 3d 10 00 00 00 ".repeat(8),
            "lea rdi, [rip + 0x10] ; un commentaire très long qui n'en finit pas de finir"
        );
        module.deliver(&ReResult::ToolOutput {
            tool: "objdump".to_owned(),
            text: Ok(longue.clone()),
        });

        let largeur = 600.0;
        let egui_ctx = egui::Context::default();
        let mut dehors: Vec<(String, f32)> = Vec::new();
        for _ in 0..2 {
            dehors.clear();
            let mut sortie = egui_ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(largeur, 1600.0),
                    )),
                    ..Default::default()
                },
                |ui| module.ui(ui, &ctx),
            );
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    for ligne in &t.galley.rows {
                        if ligne.glyphs.is_empty() {
                            continue;
                        }
                        let posée = ligne
                            .rect_without_leading_space()
                            .translate(t.pos.to_vec2());
                        if posée.right() > largeur + 0.5 {
                            dehors.push((t.galley.text().to_owned(), posée.right()));
                        }
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        assert!(
            dehors.is_empty(),
            "du texte est posé hors du panneau : {dehors:?}"
        );
    }

    /// Le bouton « Lancer » pose une demande, et c'est l'application qui
    /// lance : un module ne sort jamais du bac à sable lui-même.
    #[test]
    fn running_is_asked_for_never_done_here() {
        let mut module =
            ToolModule::connu("objdump", "objdump", "objdump", "objdump").expect("connu");
        module.deliver(&ReResult::ToolState {
            tool: "objdump".to_owned(),
            availability: Availability::Ready,
        });
        // On simule le clic en posant la demande comme le bouton le
        // fait : ce qu'on vérifie ici est le CONTRAT — une demande
        // nommée, relevée une fois, et rien d'autre.
        module.demande = Some(ReRequest::RunTool {
            tool: "objdump".to_owned(),
        });
        assert_eq!(
            module.take_request(),
            Some(ReRequest::RunTool {
                tool: "objdump".to_owned()
            })
        );
        assert_eq!(module.take_request(), None, "relevée une seule fois");
    }
}
