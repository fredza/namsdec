//! Le panneau des fragments : ce que NX laisse quand même faire.
//!
//! Il existe pour expliquer les protections d'à côté, pas pour armer
//! qui que ce soit : voir qu'un programme de dix lignes contient déjà
//! des fragments enchaînables, sans que personne les ait écrits, est ce
//! qui fait comprendre pourquoi une pile non exécutable ne suffit pas —
//! et pourquoi RELRO, le canari et IBT existent.

use crate::gadgets::{self, Gadget};
use crate::vue::{Bloc, Champ, Fragment, Vue};
use crate::{ReContext, ReModule, ReRequest, ReSection};

/// Ce que le panneau demande, par son nom.
const ACTION_CHERCHER: &str = "fragments:chercher";
/// « fragments:vider » et non « fragments:chercher:vider » : un champ
/// validé rend son action suivie de la valeur tapée, et le second nom
/// aurait été indiscernable du mot « vider » cherché parmi les
/// fragments.
const ACTION_VIDER: &str = "fragments:vider";
const ACTION_PREMIÈRE: &str = "fragments:page:première";
const ACTION_AVANT: &str = "fragments:page:avant";
const ACTION_APRÈS: &str = "fragments:page:après";
const ACTION_DERNIÈRE: &str = "fragments:page:dernière";

/// Combien on en montre par page.
///
/// Quarante tiennent sous les yeux sans faire défiler la fenêtre ;
/// au-delà, une page cesse d'être une page.
const PAR_PAGE: usize = 40;

/// Combien on en récolte en tout.
///
/// La liste s'arrêtait à quarante, sans suite ni moyen d'en voir
/// d'autres : on lisait « 40 montrés » et le binaire semblait n'en
/// contenir que quarante. Il en porte des milliers — c'est même
/// l'argument de ce panneau. On en garde donc de quoi parcourir, et la
/// borne reste, pour que la récolte ne dure pas une seconde sur un gros
/// fichier.
const RÉCOLTE: usize = 2_000;

#[derive(Default)]
pub struct GadgetsModule {
    /// Ce que le panneau demande à l'application : aller voir une
    /// adresse. Relevé après le dessin, jamais pendant.
    demande: Option<ReRequest>,
    found: Vec<Gadget>,
    /// Combien il y en avait en tout, avant la coupe.
    total: usize,
    /// Ce qu'on cherche parmi eux : on ne parcourt pas une liste de
    /// fragments, on y cherche celui qui met un argument en place.
    query: String,
    /// La page qu'on regarde, à partir de zéro.
    page: usize,
}

impl GadgetsModule {
    /// Les fragments que la recherche laisse passer.
    ///
    /// EN LECTURE SEULE : `ui()` filtrait et bornait la page pendant
    /// qu'il dessinait, ce qu'une vue ne peut pas faire — elle
    /// n'emprunte `self` que pour lire.
    fn montrés(&self) -> Vec<&Gadget> {
        let cherché = self.query.to_lowercase();
        self.found
            .iter()
            .filter(|gadget| cherché.is_empty() || gadget.text().to_lowercase().contains(&cherché))
            .collect()
    }

    /// La tranche qu'on regarde, et combien il y a de pages.
    fn fenêtre(&self) -> (usize, usize, usize) {
        let combien = self.montrés().len();
        let pages = combien.div_ceil(PAR_PAGE).max(1);
        let page = self.page.min(pages - 1);
        let début = page * PAR_PAGE;
        (début, (début + PAR_PAGE).min(combien), pages)
    }

    /// Ce que le panneau fait de ce qu'on lui demande.
    fn agit(&mut self, action: &str) {
        if action == ACTION_VIDER {
            self.chercher(String::new());
            return;
        }
        if let Some(cherché) = action.strip_prefix(&format!("{ACTION_CHERCHER}:")) {
            self.chercher(cherché.to_owned());
            return;
        }
        let (_, _, pages) = self.fenêtre();
        match action {
            ACTION_PREMIÈRE => self.page = 0,
            ACTION_AVANT => self.page = self.page.saturating_sub(1),
            ACTION_APRÈS => self.page = (self.page + 1).min(pages - 1),
            ACTION_DERNIÈRE => self.page = pages - 1,
            _ => {}
        }
    }

    /// Une recherche qui change ramène en première page : rester page
    /// sept d'une liste qui n'en compte plus qu'une, c'est se
    /// retrouver devant rien sans comprendre pourquoi.
    fn chercher(&mut self, cherché: String) {
        self.query = cherché;
        self.page = 0;
    }
}

impl ReModule for GadgetsModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Bytes
    }

    fn id(&self) -> &'static str {
        "gadgets"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Fragments réutilisables (ROP)"
        } else {
            "Reusable fragments (ROP)"
        }
    }

    /// Le nom court dans la colonne : le titre entier y passait sur
    /// DEUX lignes.
    ///
    /// Vingt-trois pixels de plus pour une seule entrée, et c'est la
    /// dernière rubrique de la colonne — « Détourner » — qui tombait
    /// sous le bord. Mesuré, non supposé : le relevé de la fenêtre
    /// d'analyse donnait une hauteur de galley de 30 pour cette entrée
    /// et de 15 pour toutes les autres. Le panneau garde son titre
    /// entier, où la place ne manque pas. Le même mot sert dans les
    /// deux langues : « fragments » s'écrit pareil, et « ROP » est un
    /// sigle.
    fn nav_title(&self, _lang: &str) -> &'static str {
        "Fragments (ROP)"
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.found = gadgets::from_disasm(ctx.disasm, RÉCOLTE);
        self.total = self.found.len();
        self.page = 0;
        self.query.clear();
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        let mut blocs = vec![Bloc::Texte(
            Fragment::dit(ctx.t(
                "Une pile non exécutable (NX) empêche d'exécuter des octets qu'on y \
                 dépose. Ces fragments-là, eux, sont déjà dans le code : chacun finit \
                 par `ret`, et les enchaîner revient à écrire un programme avec les \
                 morceaux d'un autre. C'est ce que les autres protections — RELRO, le \
                 canari, IBT — cherchent à empêcher.",
                "A non-executable stack (NX) stops bytes written onto it from running. \
                 These fragments are already in the code: each ends with `ret`, and \
                 chaining them amounts to writing a program out of someone else's \
                 pieces. That is what the other protections — RELRO, the canary, IBT — \
                 are there to prevent.",
            ))
            .faible()
            .into(),
        )];
        if self.found.is_empty() {
            return Some(Vue {
                blocs: {
                    blocs.push(Bloc::Texte(
                        Fragment::dit(ctx.t(
                            "Aucun fragment ici : le programme est trop court.",
                            "No fragments here: the program is too short.",
                        ))
                        .faible()
                        .into(),
                    ));
                    blocs
                },
            });
        }

        let mut champ = Champ::pour(ACTION_CHERCHER)
            .valeur(&self.query)
            .indice(ctx.t("chercher — « pop rdi »…", "search — \"pop rdi\"…"))
            .caractères(22);
        if !self.query.is_empty() {
            champ = champ.bouton(
                Fragment::dit("x")
                    .bouton(ACTION_VIDER)
                    .survol(ctx.t("Vider la recherche", "Clear the search")),
            );
        }
        blocs.push(Bloc::Champ(champ));

        let montrés = self.montrés();
        let (début, fin, pages) = self.fenêtre();
        // Ce qu'on a sous les yeux, et de quoi : « 41 à 80 sur 1 274 ».
        // Un simple « 40 montrés » ne dit pas qu'il y a une suite, et
        // c'est précisément ce qu'on reprochait à ce panneau.
        let mut état = vec![if montrés.is_empty() {
            Fragment::dit(ctx.t("Aucun fragment ne correspond", "No fragment matches"))
        } else {
            Fragment::dit(format!(
                "{}–{} {} {}",
                début + 1,
                fin,
                ctx.t("sur", "of"),
                montrés.len()
            ))
        }];
        if !self.query.is_empty() {
            état.push(
                Fragment::dit(format!(
                    "({} {} {})",
                    self.found.len(),
                    ctx.t("fragments en tout", "fragments in all"),
                    if self.total >= RÉCOLTE {
                        ctx.t("— récolte bornée", "— harvest capped")
                    } else {
                        ""
                    }
                ))
                .faible(),
            );
        } else if self.total >= RÉCOLTE {
            état.push(
                Fragment::dit(ctx.t(
                    "— la récolte s'arrête là, le binaire en contient davantage",
                    "— the harvest stops there, the binary holds more",
                ))
                .faible(),
            );
        }
        blocs.push(Bloc::Texte(état.into()));

        // La navigation : première, précédente, la page, suivante,
        // dernière. Les boutons s'éteignent au bout plutôt que de
        // disparaître — une rangée qui change de forme se relit à
        // chaque fois.
        //
        // « « ‹ › » » et non des triangles : ceux-ci n'existent pas
        // dans les polices qu'egui embarque, et se peignaient en carré
        // vide — un bouton qui ne montre rien ne se clique pas.
        if pages > 1 {
            let première = self.page == 0;
            let dernière = self.page + 1 >= pages;
            blocs.push(Bloc::Texte(
                vec![
                    Fragment::dit("\u{00ab}")
                        .bouton(ACTION_PREMIÈRE)
                        .inactif_si(première)
                        .survol(ctx.t("Première page", "First page")),
                    Fragment::dit("\u{2039}")
                        .bouton(ACTION_AVANT)
                        .inactif_si(première)
                        .survol(ctx.t("Page précédente", "Previous page")),
                    Fragment::dit(format!(
                        "{} {} / {}",
                        ctx.t("page", "page"),
                        self.page + 1,
                        pages
                    )),
                    Fragment::dit("\u{203a}")
                        .bouton(ACTION_APRÈS)
                        .inactif_si(dernière)
                        .survol(ctx.t("Page suivante", "Next page")),
                    Fragment::dit("\u{00bb}")
                        .bouton(ACTION_DERNIÈRE)
                        .inactif_si(dernière)
                        .survol(ctx.t("Dernière page", "Last page")),
                ]
                .into(),
            ));
        }

        // TRONQUÉ PLUTÔT QUE DÉBORDANT, et le module n'a plus à le
        // dire : un fragment de quatre instructions fait soixante-dix
        // caractères, et la fenêtre d'analyse réduite n'en montre que
        // quarante. Le peintre borne la colonne la plus large d'un
        // tableau et rend le reste au survol — une fois, pour tous les
        // tableaux.
        blocs.push(Bloc::Tableau {
            colonnes: Vec::new(),
            rangées: montrés[début..fin]
                .iter()
                .map(|gadget| {
                    vec![
                        Fragment::dit(format!("{:#x}", gadget.addr))
                            .code()
                            .mène_à(gadget.addr)
                            .into(),
                        Fragment::dit(gadget.text()).code().into(),
                    ]
                })
                .collect(),
            rayé: true,
        });
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

    /// Ce que le panneau écrit, une galère par ligne.
    fn textes(module: &mut GadgetsModule, ctx: &ReContext) -> Vec<String> {
        let egui_ctx = egui::Context::default();
        let mut vus = Vec::new();
        // Deux images : la première pose la mise en page, la seconde la
        // peint pour de bon.
        for _ in 0..2 {
            vus.clear();
            let mut sortie = egui_ctx.run_ui(Default::default(), |ui| module.ui(ui, ctx));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    vus.push(texte.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        vus
    }

    /// La liste se parcourt page par page, au lieu de s'arrêter à
    /// quarante sans le dire.
    ///
    /// C'était le grief : « ici 40 affichés et… ». Rien ne disait qu'il
    /// y avait une suite, et rien ne permettait d'y aller — alors qu'un
    /// binaire ordinaire porte des milliers de fragments, ce qui est
    /// tout l'argument de ce panneau.
    #[test]
    fn the_list_can_be_walked_page_by_page() {
        // Cent fragments distincts : « pop rdi ; ret » répété donne cent
        // adresses, une par couple d'octets.
        let mut bytes = Vec::new();
        for _ in 0..100 {
            bytes.extend_from_slice(&[0x5f, 0xc3]);
        }
        let disasm = namsdec_disasm::disassemble(&bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x1000);
        mem.load(0x400000, &bytes).unwrap();
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

        let mut module = GadgetsModule::default();
        module.on_binary_loaded(&ctx);
        assert!(
            module.found.len() > PAR_PAGE,
            "la récolte dépasse une page : {}",
            module.found.len()
        );

        let première = textes(&mut module, &ctx).join(" | ");
        assert!(
            première.contains("page 1 / "),
            "la page et leur nombre sont écrits : {première}"
        );
        assert!(
            première.contains(&format!("1–{PAR_PAGE} sur")),
            "ce qu'on a sous les yeux est dit : {première}"
        );

        // La page suivante montre d'autres adresses que la première.
        let adresses = |texte: &str| -> Vec<String> {
            texte
                .split(" | ")
                .filter(|t| t.starts_with("0x"))
                .map(str::to_owned)
                .collect()
        };
        let avant = adresses(&première);
        assert_eq!(avant.len(), PAR_PAGE, "une page pleine : {avant:?}");
        module.page = 1;
        let seconde = textes(&mut module, &ctx).join(" | ");
        let après = adresses(&seconde);
        assert!(
            après.iter().all(|a| !avant.contains(a)),
            "la seconde page montre d'autres fragments : {après:?}"
        );
        assert!(seconde.contains("page 2 / "), "{seconde}");
    }

    /// Une recherche ramène en première page.
    ///
    /// Sinon l'on reste page sept d'une liste qui n'en compte plus
    /// qu'une : la grille est vide, et rien ne dit pourquoi.
    /// Une recherche qui change ramène en première page.
    ///
    /// **PAR LE CHEMIN RÉEL, depuis P7.** Ce contrôle posait
    /// `module.query` à la main, puis comptait sur le fait que `ui()`
    /// s'apercevrait du changement en dessinant. Le tri n'est plus
    /// dans le dessin : la vue PROPOSE l'action du champ, et c'est
    /// elle qu'on joue — ce qui éprouve le chemin entier et fait
    /// tomber le contrôle si le nom de l'action change d'un côté
    /// seulement.
    #[test]
    fn a_new_search_comes_back_to_the_first_page() {
        let mut bytes = Vec::new();
        for _ in 0..100 {
            bytes.extend_from_slice(&[0x5f, 0xc3]);
        }
        bytes.extend_from_slice(&[0x5e, 0xc3]); // pop rsi ; ret
        let disasm = namsdec_disasm::disassemble(&bytes, 0x400000);
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = GadgetsModule::default();
        module.on_binary_loaded(&ctx);

        // On s'en va page trois, par les boutons que la vue propose.
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
        module.agit(&suivante);
        assert_eq!(module.page, 2, "on est page trois");

        // Puis l'on cherche, par l'action du champ.
        let vue = module.vue(&ctx).expect("le panneau dit quelque chose");
        let champ = vue
            .blocs
            .iter()
            .find_map(|bloc| match bloc {
                Bloc::Champ(champ) => Some(champ.action.clone()),
                _ => None,
            })
            .expect("le panneau porte un champ de recherche");
        module.agit(&format!("{champ}:pop rsi"));
        assert_eq!(module.page, 0, "la recherche ramène en tête");

        let texte = textes(&mut module, &ctx).join(" | ");
        assert!(texte.contains("1–1 sur 1"), "{texte}");
    }

    /// Le filtre sert à trouver le fragment qu'on cherche — « pop rdi »
    /// met un argument en place, et c'est celui-là qu'on veut.
    #[test]
    fn the_filter_finds_the_fragment_one_is_after() {
        // pop rdi ; ret, puis pop rsi ; ret.
        let bytes: &[u8] = &[0x5f, 0xc3, 0x5e, 0xc3];
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

        let mut module = GadgetsModule::default();
        module.on_binary_loaded(&ctx);
        assert!(module.found.len() >= 2, "{:?}", module.found);

        // Le filtre est une simple recherche dans le texte du fragment :
        // c'est ce qu'on teste, le rendu n'y ajoutant rien.
        module.query = "pop rdi".to_owned();
        let montrés: Vec<&Gadget> = module
            .found
            .iter()
            .filter(|gadget| gadget.text().to_lowercase().contains(&module.query))
            .collect();
        assert_eq!(montrés.len(), 1, "{montrés:?}");
        assert_eq!(montrés[0].text(), "pop rdi ; ret");
    }
}
