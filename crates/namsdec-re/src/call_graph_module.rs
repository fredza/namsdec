//! Le module « Graphe d'appels » : qui appelle qui, et ce qu'il y a dedans.
//!
//! Les autres modules disent *où* sont les fonctions (le module
//! « Fonctions ») et *ce qu'elles font de remarquable* (figures, appels
//! système). Celui-ci montre leur **structure** : à partir du point
//! d'entrée, l'arbre des appels, à la façon d'un diagramme UML — une
//! boîte par fonction, un trait vers chacune de celles qu'elle appelle.
//! C'est la carte qu'on lit avant de descendre dans une fonction.
//!
//! Trois choses s'y font :
//!
//! - **Aller à une adresse.** On tape une adresse, on y va dans le
//!   listing — et chaque boîte de l'arbre est elle-même un raccourci.
//! - **Voir l'arbre des appels.** Déplié depuis `_start`/`main`, chaque
//!   fonction menant à celles qu'elle appelle. Une fonction déjà vue
//!   ailleurs est marquée d'un renvoi plutôt que redéployée : c'est un
//!   arbre, pas une boucle infinie.
//! - **Lire le détail d'une fonction.** Sous l'arbre, chaque fonction se
//!   déplie sur son **code binaire** (les octets, bruts) et son
//!   **pseudo-code** (ce que chaque instruction fait, en clair).

use crate::functions::{Function, Origin, by_importance, discover};
use crate::{ReContext, ReModule, ReRequest, clickable_address};
use std::collections::HashMap;

/// Combien de fonctions l'arbre et le détail montrent, au plus.
///
/// Un arbre de trois cents boîtes n'est plus une carte, c'est un mur. On
/// garde les plus employées — celles par où l'on entre dans un binaire —
/// et le compte total dit ce qu'on ne montre pas.
const MAX_FONCTIONS: usize = 40;

/// Jusqu'où l'arbre descend. Au-delà, on marque « … » : une pile d'appels
/// de douze niveaux ne se lit plus, et la profondeur seule n'apprend rien.
const MAX_PROFONDEUR: usize = 8;

/// Combien de nœuds l'arbre dessine en tout, renvois compris.
const MAX_NŒUDS: usize = 120;

/// Combien d'instructions une rubrique de détail montre avant de couper.
const MAX_INSTRUCTIONS: usize = 200;

/// Un pas d'indentation dans l'arbre, en pixels.
const INDENT: f32 = 22.0;

/// Un nœud de l'arbre, aplati dans l'ordre où on le dessine.
struct Nœud {
    /// Rang de la fonction dans `self.functions`.
    fonction: usize,
    /// Profondeur dans l'arbre (0 = la racine).
    profondeur: usize,
    /// Le nœud parent, dans cette liste aplatie.
    parent: Option<usize>,
    /// Un renvoi : la fonction est déjà dépliée ailleurs. On la montre,
    /// mais on ne redéploie pas ses appels.
    renvoi: bool,
}

#[derive(Default)]
pub struct CallGraphModule {
    /// Ce que le panneau demande à l'application. Relevé après le dessin.
    demande: Option<ReRequest>,
    functions: Vec<Function>,
    /// De l'adresse de début d'une fonction à son rang dans `functions`.
    par_adresse: HashMap<u64, usize>,
    entry: u64,
    main: Option<u64>,
    /// Les rangs des fonctions montrées, les plus employées d'abord.
    montrées: Vec<usize>,
    /// Ce qu'on a tapé dans « aller à une adresse ».
    aller_à: String,
    /// Dit quand l'adresse tapée ne se lit pas, pour l'écrire sous le champ.
    aller_à_erreur: bool,
}

impl CallGraphModule {
    /// L'arbre des appels, aplati dans l'ordre de dessin (pré-ordre).
    ///
    /// Depuis la racine — le point d'entrée, ou `main` à défaut —, on
    /// descend par les appels. Une fonction déjà posée devient un renvoi :
    /// l'arbre reste fini même quand le code boucle sur lui-même.
    fn arbre(&self) -> Vec<Nœud> {
        let mut nœuds = Vec::new();
        let Some(&racine) = self
            .main
            .and_then(|m| self.par_adresse.get(&m))
            .or_else(|| self.entry_index())
            .or_else(|| self.montrées.first())
        else {
            return nœuds;
        };
        let mut vus = std::collections::HashSet::new();
        self.descendre(racine, 0, None, &mut nœuds, &mut vus);
        nœuds
    }

    fn entry_index(&self) -> Option<&usize> {
        self.par_adresse.get(&self.entry)
    }

    fn descendre(
        &self,
        fonction: usize,
        profondeur: usize,
        parent: Option<usize>,
        nœuds: &mut Vec<Nœud>,
        vus: &mut std::collections::HashSet<usize>,
    ) {
        if nœuds.len() >= MAX_NŒUDS {
            return;
        }
        let renvoi = !vus.insert(fonction);
        let moi = nœuds.len();
        nœuds.push(Nœud {
            fonction,
            profondeur,
            parent,
            renvoi,
        });
        // Un renvoi ne se redéploie pas, et l'on s'arrête à la profondeur
        // où l'arbre cesse d'apprendre.
        if renvoi || profondeur >= MAX_PROFONDEUR {
            return;
        }
        for cible in self.functions[fonction].calls.clone() {
            if let Some(&rang) = self.par_adresse.get(&cible) {
                self.descendre(rang, profondeur + 1, Some(moi), nœuds, vus);
            }
        }
    }

    /// Les instructions d'une fonction, prises dans le désassemblage trié.
    fn instructions<'a>(
        &self,
        ctx: &'a ReContext,
        f: &Function,
    ) -> &'a [namsdec_disasm::DecodedInstruction] {
        let début = ctx.disasm.partition_point(|i| i.address < f.start);
        let fin = ctx.disasm.partition_point(|i| i.address <= f.last);
        ctx.disasm.get(début..fin).unwrap_or(&[])
    }
}

impl ReModule for CallGraphModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn id(&self) -> &'static str {
        "call_graph"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Graphe d'appels"
        } else {
            "Call graph"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.entry = crate::elf_scan::scan(ctx.binary)
            .map(|elf| elf.entry)
            .or_else(|| ctx.disasm.first().map(|i| i.address))
            .unwrap_or(0);
        self.functions = discover(ctx.disasm, self.entry);
        self.main = crate::functions::main_of(ctx.disasm, self.entry);
        self.par_adresse = self
            .functions
            .iter()
            .enumerate()
            .map(|(rang, f)| (f.start, rang))
            .collect();
        // Les plus employées d'abord : ce sont les utilitaires et les
        // points d'entrée, par où l'on commence à lire.
        let ordre = by_importance(&self.functions);
        self.montrées = ordre
            .into_iter()
            .take(MAX_FONCTIONS)
            .filter_map(|f| self.par_adresse.get(&f.start).copied())
            .collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.weak(ctx.t(
            "Un programme n'est pas une liste d'instructions mais un réseau de \
             fonctions qui s'appellent. Voici ce réseau, déplié depuis le point \
             d'entrée : chaque boîte est une fonction, chaque trait un appel. On \
             clique une boîte pour aller la voir dans le listing ; on déplie une \
             fonction, plus bas, pour lire ses octets et ce qu'ils font.",
            "A program is not a list of instructions but a web of functions that \
             call one another. Here is that web, unfolded from the entry point: \
             each box is a function, each line a call. Click a box to go to it in \
             the listing; unfold a function, below, to read its bytes and what \
             they do.",
        ));

        if self.functions.is_empty() {
            ui.add_space(6.0);
            ui.weak(ctx.t(
                "Rien à tracer : aucun code n'est chargé.",
                "Nothing to trace: no code is loaded.",
            ));
            return;
        }

        ui.add_space(8.0);
        self.aller_à_une_adresse(ui, ctx);

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        ui.strong(ctx.t("L'arbre des appels", "The call tree"));
        ui.add_space(4.0);
        self.dessiner_arbre(ui, ctx);

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(6.0);
        ui.strong(ctx.t("Le détail des fonctions", "Function details"));
        ui.weak(ctx.t(
            "Pour chaque fonction, ses octets et leur traduction.",
            "For each function, its bytes and their translation.",
        ));
        ui.add_space(4.0);
        self.dessiner_détails(ui, ctx);
    }
}

impl CallGraphModule {
    /// Le champ « aller à une adresse », et ce qu'il comprend.
    fn aller_à_une_adresse(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.horizontal(|ui| {
            ui.label(ctx.t("Aller à :", "Go to:"));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut self.aller_à)
                    .hint_text("0x401000")
                    .desired_width(140.0)
                    .font(egui::TextStyle::Monospace),
            );
            let validé = champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button(ctx.t("Y aller", "Go")).clicked() || validé {
                match lire_adresse(&self.aller_à) {
                    Some(addr) => {
                        self.demande = Some(ReRequest::GoTo { addr });
                        self.aller_à_erreur = false;
                    }
                    None => self.aller_à_erreur = !self.aller_à.trim().is_empty(),
                }
            }
        });
        if self.aller_à_erreur {
            ui.weak(ctx.t(
                "Cette adresse ne se lit pas : attendez un nombre, en hexadécimal \
                 (0x401000) ou en décimal.",
                "That address does not read: a number is expected, in hex \
                 (0x401000) or decimal.",
            ));
        }
    }

    /// L'arbre, dessiné avec egui : une boîte cliquable par nœud, et des
    /// traits de liaison peints entre parent et enfant.
    fn dessiner_arbre(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let nœuds = self.arbre();
        if nœuds.is_empty() {
            ui.weak(ctx.t(
                "Pas de racine d'où partir : ni point d'entrée ni `main` connus.",
                "No root to start from: neither entry point nor `main` is known.",
            ));
            return;
        }

        let couleur_trait = ui.visuals().weak_text_color();
        let mut demande: Option<ReRequest> = None;
        // Le rectangle de chaque boîte, pour tracer les liens ensuite.
        let mut rects: Vec<egui::Rect> = Vec::with_capacity(nœuds.len());

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for nœud in &nœuds {
                let f = &self.functions[nœud.fonction];
                let réponse = ui
                    .horizontal(|ui| {
                        ui.add_space(nœud.profondeur as f32 * INDENT);
                        boîte_fonction(ui, f, nœud.renvoi)
                    })
                    .inner;
                if réponse.clicked() {
                    demande = Some(ReRequest::GoTo { addr: f.start });
                }
                rects.push(réponse.rect);
            }
        });

        // Les traits, par-dessus la gouttière de gauche (jamais sur une
        // boîte : l'enfant est indenté d'un cran de plus que son rail).
        let peintre = ui.painter();
        let trait_ = egui::Stroke::new(1.0, couleur_trait);
        for (i, nœud) in nœuds.iter().enumerate() {
            let Some(parent) = nœud.parent else { continue };
            let rp = rects[parent];
            let rn = rects[i];
            let x_rail = rp.left() + 9.0;
            let y_enfant = rn.center().y;
            peintre.line_segment(
                [egui::pos2(x_rail, rp.bottom()), egui::pos2(x_rail, y_enfant)],
                trait_,
            );
            peintre.line_segment(
                [egui::pos2(x_rail, y_enfant), egui::pos2(rn.left(), y_enfant)],
                trait_,
            );
        }

        if demande.is_some() {
            self.demande = demande;
        }

        if self.functions.len() > self.montrées.len() {
            ui.add_space(4.0);
            ui.weak(format!(
                "… {} {}",
                self.functions.len() - self.montrées.len(),
                ctx.t(
                    "fonctions de plus, moins employées, non tracées",
                    "more functions, less used, not traced"
                )
            ));
        }
    }

    /// Le détail de chaque fonction montrée : code binaire et pseudo-code.
    fn dessiner_détails(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let mut demande: Option<ReRequest> = None;
        // On copie les rangs : la grille emprunte `self` pour lire le
        // désassemblage, et le module ne peut donc pas s'écrire dedans.
        let montrées = self.montrées.clone();
        for rang in montrées {
            let f = self.functions[rang].clone();
            let titre = format!(
                "{}   {:#010x}   {} {}",
                f.name(),
                f.start,
                f.size(),
                ctx.t("octets", "bytes"),
            );
            egui::CollapsingHeader::new(egui::RichText::new(titre).monospace())
                .id_salt(("call_graph_detail", f.start))
                .default_open(false)
                .show(ui, |ui| {
                    if clickable_address(ui, ctx, f.start) {
                        demande = Some(ReRequest::GoTo { addr: f.start });
                    }
                    ui.add_space(4.0);
                    let instrs = self.instructions(ctx, &f);
                    if instrs.is_empty() {
                        ui.weak(ctx.t(
                            "Aucune instruction désassemblée dans cette plage.",
                            "No disassembled instruction in this range.",
                        ));
                        return;
                    }
                    rubrique_code(ui, ctx, instrs);
                });
        }
        if demande.is_some() {
            self.demande = demande;
        }
    }
}

/// Une boîte de fonction, cliquable, à la façon d'un nœud UML.
fn boîte_fonction(ui: &mut egui::Ui, f: &Function, renvoi: bool) -> egui::Response {
    let nom = f.name();
    let accent = matches!(f.origin, Origin::Entry | Origin::Main);
    let mut texte = egui::RichText::new(format!("{nom}  {:#x}", f.start)).monospace();
    if accent {
        texte = texte.strong();
    }
    if renvoi {
        // Un renvoi : la fonction est dépliée ailleurs. La flèche le dit,
        // et la couleur affaiblie l'éloigne du reste.
        texte = egui::RichText::new(format!("↩ {nom}  {:#x}", f.start))
            .monospace()
            .color(ui.visuals().weak_text_color());
    }
    let bouton = egui::Button::new(texte)
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(1.0, ui.visuals().weak_text_color()))
        .corner_radius(egui::CornerRadius::from(4));
    let infobulle = if renvoi {
        "Déjà dépliée plus haut — cliquer pour aller la voir"
    } else {
        "Aller à cette fonction dans le listing"
    };
    ui.add(bouton)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(infobulle)
}

/// La rubrique de code d'une fonction : par instruction, les octets puis
/// le pseudo-code.
///
/// Deux colonnes dans une grille : à gauche l'adresse et les octets bruts
/// — le **code binaire**, ce qui est vraiment dans le fichier —, à droite
/// ce que l'instruction fait, en clair. C'est la même ligne lue deux
/// fois : telle que la machine la reçoit, puis telle qu'on la comprend.
fn rubrique_code(
    ui: &mut egui::Ui,
    ctx: &ReContext,
    instrs: &[namsdec_disasm::DecodedInstruction],
) {
    egui::Grid::new(("call_graph_code", instrs[0].address))
        .num_columns(3)
        .striped(true)
        .min_col_width(0.0)
        .show(ui, |ui| {
            ui.weak(ctx.t("adresse", "address"));
            ui.weak(ctx.t("octets", "bytes"));
            ui.weak(ctx.t("pseudo-code", "pseudo-code"));
            ui.end_row();
            for instr in instrs.iter().take(MAX_INSTRUCTIONS) {
                ui.monospace(format!("{:#010x}", instr.address));
                ui.monospace(octets_hex(&instr.bytes));
                // Le pseudo-code quand on sait le rendre, l'instruction
                // elle-même sinon : mieux vaut le NASM que rien.
                let pseudo = namsdec_disasm::decode_one(&instr.bytes, instr.address)
                    .map(|d| namsdec_disasm::pseudo::of(&d, ctx.lang))
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(|| instr.text.clone());
                ui.monospace(pseudo);
                ui.end_row();
            }
        });
    if instrs.len() > MAX_INSTRUCTIONS {
        ui.weak(format!(
            "… {} {}",
            instrs.len() - MAX_INSTRUCTIONS,
            ctx.t("instructions de plus", "more instructions")
        ));
    }
}

/// Les octets d'une instruction, en hexadécimal, espacés par deux.
fn octets_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lit une adresse tapée à la main : hexadécimale (`0x…`) ou décimale.
fn lire_adresse(texte: &str) -> Option<u64> {
    let t = texte.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        // Un nombre nu : décimal d'abord, car c'est ce qu'on tape quand on
        // en écrit un ; hexadécimal en repli, car une adresse l'est
        // souvent même sans le `0x`.
        t.parse::<u64>().ok().or_else(|| u64::from_str_radix(t, 16).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn contexte<'a>(disasm: &'a [namsdec_disasm::DecodedInstruction], cpu: &'a Cpu) -> ReContext<'a> {
        ReContext {
            cpu,
            disasm,
            source: "",
            binary: &[],
            lang: "fr",
        }
    }

    /// Une adresse se lit en hexadécimal avec ou sans `0x`, et en décimal.
    #[test]
    fn an_address_reads_in_hex_or_decimal() {
        assert_eq!(lire_adresse("0x401000"), Some(0x401000));
        assert_eq!(lire_adresse("  0X10 "), Some(0x10));
        assert_eq!(lire_adresse("4096"), Some(4096));
        // Sans `0x`, un nombre qui n'est pas décimal se relit en hexa.
        assert_eq!(lire_adresse("ff"), Some(0xff));
        assert_eq!(lire_adresse("zzz"), None);
        assert_eq!(lire_adresse(""), None);
    }

    /// L'arbre part de la racine, descend par les appels, et reste fini
    /// même quand une fonction s'appelle elle-même.
    #[test]
    fn the_tree_unfolds_from_the_root_and_stays_finite() {
        // Deux fonctions : à 0x1000, un `call` vers 0x1000 lui-même
        // (récursion), puis `ret`. L'arbre ne doit pas boucler.
        // e8 .. .. .. .. = call rel32 ; c3 = ret.
        let bytes: &[u8] = &[0xE8, 0xFB, 0xFF, 0xFF, 0xFF, 0xC3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = contexte(&disasm, &cpu);
        let mut module = CallGraphModule::default();
        module.on_binary_loaded(&ctx);
        let arbre = module.arbre();
        assert!(!arbre.is_empty(), "la racine au moins doit être là");
        // La récursion est marquée d'un renvoi, pas redéployée.
        assert!(
            arbre.iter().any(|n| n.renvoi),
            "un appel récursif devient un renvoi"
        );
        assert!(arbre.len() <= MAX_NŒUDS);
    }

    /// Les instructions d'une fonction sont prises dans sa plage, et pas
    /// au-delà.
    #[test]
    fn a_functions_instructions_are_bounded_to_its_range() {
        let bytes: &[u8] = &[0x90, 0x90, 0xC3]; // nop ; nop ; ret
        let disasm = namsdec_disasm::disassemble(bytes, 0x2000);
        let cpu = Cpu::new(Memory::new(0x2000, 0x100));
        let ctx = contexte(&disasm, &cpu);
        let module = CallGraphModule::default();
        let f = Function {
            start: 0x2000,
            last: 0x2001,
            callers: 0,
            calls: vec![],
            origin: Origin::Called,
        };
        let instrs = module.instructions(&ctx, &f);
        // 0x2000 et 0x2001 appartiennent à la fonction, 0x2002 (ret) non.
        assert_eq!(instrs.len(), 2, "deux instructions dans [0x2000, 0x2001]");
        assert_eq!(instrs[0].address, 0x2000);
    }
}
