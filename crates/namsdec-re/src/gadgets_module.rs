//! Le panneau des fragments : ce que NX laisse quand même faire.
//!
//! Il existe pour expliquer les protections d'à côté, pas pour armer
//! qui que ce soit : voir qu'un programme de dix lignes contient déjà
//! des fragments enchaînables, sans que personne les ait écrits, est ce
//! qui fait comprendre pourquoi une pile non exécutable ne suffit pas —
//! et pourquoi RELRO, le canari et IBT existent.

use crate::gadgets::{self, Gadget};
use crate::{
    ReContext, ReModule, ReRequest, ReSection, clickable_address, framed, framed_small_button,
};

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
    /// La recherche à laquelle cette page correspond.
    ///
    /// Quand elle change, on revient en première page : rester page
    /// sept d'une liste qui n'en compte plus qu'une, c'est se retrouver
    /// devant rien sans comprendre pourquoi.
    query_appliquée: String,
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

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.found = gadgets::from_disasm(ctx.disasm, RÉCOLTE);
        self.total = self.found.len();
        self.page = 0;
        self.query.clear();
        self.query_appliquée.clear();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.weak(ctx.t(
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
        ));
        if self.found.is_empty() {
            ui.add_space(4.0);
            ui.weak(ctx.t(
                "Aucun fragment ici : le programme est trop court.",
                "No fragments here: the program is too short.",
            ));
            return;
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(ctx.t("chercher — « pop rdi »…", "search — \"pop rdi\"…"))
                    .desired_width(160.0),
            );
            if !self.query.is_empty() && framed_small_button(ui, "x").clicked() {
                self.query.clear();
            }
        });
        let query = self.query.to_lowercase();
        let montrés: Vec<&Gadget> = self
            .found
            .iter()
            .filter(|gadget| query.is_empty() || gadget.text().to_lowercase().contains(&query))
            .collect();

        // Une recherche qui change ramène en première page.
        if self.query != self.query_appliquée {
            self.query_appliquée = self.query.clone();
            self.page = 0;
        }
        let pages = montrés.len().div_ceil(PAR_PAGE).max(1);
        self.page = self.page.min(pages - 1);
        let début = self.page * PAR_PAGE;
        let fin = (début + PAR_PAGE).min(montrés.len());

        // Ce qu'on a sous les yeux, et de quoi : « 41 à 80 sur 1 274 ».
        // Un simple « 40 montrés » ne dit pas qu'il y a une suite, et
        // c'est précisément ce qu'on reprochait à ce panneau.
        ui.horizontal_wrapped(|ui| {
            if montrés.is_empty() {
                ui.label(ctx.t("Aucun fragment ne correspond", "No fragment matches"));
            } else {
                ui.label(format!(
                    "{}–{} {} {}",
                    début + 1,
                    fin,
                    ctx.t("sur", "of"),
                    montrés.len()
                ));
            }
            if !query.is_empty() {
                ui.weak(format!(
                    "({} {} {})",
                    self.found.len(),
                    ctx.t("fragments en tout", "fragments in all"),
                    if self.total >= RÉCOLTE {
                        ctx.t("— récolte bornée", "— harvest capped")
                    } else {
                        ""
                    }
                ));
            } else if self.total >= RÉCOLTE {
                ui.weak(ctx.t(
                    "— la récolte s'arrête là, le binaire en contient davantage",
                    "— the harvest stops there, the binary holds more",
                ));
            }
        });

        // La navigation : première, précédente, la page, suivante,
        // dernière. Les boutons s'éteignent au bout plutôt que de
        // disparaître — une rangée qui change de forme se relit à
        // chaque fois.
        if pages > 1 {
            // « « ‹ › » » et non des triangles : ceux-ci n'existent
            // pas dans les polices qu'egui embarque, et se peignaient
            // en carré vide — un bouton qui ne montre rien ne se
            // clique pas.
            ui.horizontal(|ui| {
                let première = self.page == 0;
                let dernière = self.page + 1 >= pages;
                if ui
                    .add_enabled(!première, framed(ui, "\u{00ab}"))
                    .on_hover_text(ctx.t("Première page", "First page"))
                    .clicked()
                {
                    self.page = 0;
                }
                if ui
                    .add_enabled(!première, framed(ui, "\u{2039}"))
                    .on_hover_text(ctx.t("Page précédente", "Previous page"))
                    .clicked()
                {
                    self.page -= 1;
                }
                ui.label(format!(
                    "{} {} / {}",
                    ctx.t("page", "page"),
                    self.page + 1,
                    pages
                ));
                if ui
                    .add_enabled(!dernière, framed(ui, "\u{203a}"))
                    .on_hover_text(ctx.t("Page suivante", "Next page"))
                    .clicked()
                {
                    self.page += 1;
                }
                if ui
                    .add_enabled(!dernière, framed(ui, "\u{00bb}"))
                    .on_hover_text(ctx.t("Dernière page", "Last page"))
                    .clicked()
                {
                    self.page = pages - 1;
                }
            });
        }
        // Relevé pendant le dessin, posé après : la grille emprunte
        // `self`, et le module ne peut donc pas s'écrire dedans.
        let mut demande: Option<ReRequest> = None;
        egui::Grid::new("gadgets_grid")
            .num_columns(2)
            .striped(true)
            .min_col_width(0.0)
            .show(ui, |ui| {
                for gadget in &montrés[début..fin] {
                    if let Some(cible) = clickable_address(ui, ctx, gadget.addr) {
                        demande = Some(ReRequest::GoTo {
                            addr: gadget.addr,
                            target: cible,
                        });
                    }
                    // Tronqué plutôt que débordant : un fragment de
                    // quatre instructions fait soixante-dix caractères,
                    // et la fenêtre d'analyse réduite n'en montre que
                    // quarante. L'infobulle rend le reste, et le texte
                    // ne court plus hors du panneau.
                    let texte = gadget.text();
                    ui.add(
                        egui::Label::new(egui::RichText::new(&texte).monospace())
                            .truncate()
                            .sense(egui::Sense::hover()),
                    )
                    .on_hover_text(egui::RichText::new(&texte).monospace());
                    ui.end_row();
                }
            });
        if demande.is_some() {
            self.demande = demande;
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
        };
        let mut module = GadgetsModule::default();
        module.on_binary_loaded(&ctx);
        module.page = 2;
        module.query = "pop rsi".to_owned();
        let texte = textes(&mut module, &ctx).join(" | ");
        assert_eq!(module.page, 0, "la recherche ramène en tête : {texte}");
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
