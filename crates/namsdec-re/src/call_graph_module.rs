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
use crate::{ReContext, ReModule, ReRequest, ReSection, clickable_address, framed_button};
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
    /// Le nom en cours de frappe pour chaque fonction, par adresse.
    édition: HashMap<u64, String>,
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

    fn section(&self) -> ReSection {
        ReSection::Code
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
        // Les tampons de frappe d'un autre binaire n'ont plus de sens :
        // ses adresses ne sont pas celles-ci.
        self.édition.clear();
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
            if framed_button(ui, ctx.t("Y aller", "Go")).clicked() || validé {
                match lire_adresse(&self.aller_à) {
                    Some(addr) => {
                        // Une adresse TAPÉE dans ce champ, et non une
                        // adresse cliquée : on la cherche dans le code,
                        // c'est l'objet de ce panneau.
                        self.demande = Some(ReRequest::GoTo {
                            addr,
                            target: crate::GoTarget::Listing,
                        });
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

    /// L'arbre, dessiné comme un vrai diagramme : une boîte peinte et
    /// cliquable par nœud, reliée à son parent par un connecteur fléché.
    ///
    /// Peint sur un canevas unique (`allocate_painter`) plutôt qu'empilé
    /// en widgets : c'est ce qui en fait un graphe et non une liste. Une
    /// première version posait des boutons dans une colonne et traçait
    /// des traits par-dessus ; aux couleurs discrètes du thème, cela ne
    /// se lisait pas comme un graphe. Ici les boîtes ont un fond et un
    /// cadre francs, et les liens portent une flèche — on voit qui appelle
    /// qui.
    fn dessiner_arbre(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let nœuds = self.arbre();
        if nœuds.is_empty() {
            ui.weak(ctx.t(
                "Pas de racine d'où partir : ni point d'entrée ni `main` connus.",
                "No root to start from: neither entry point nor `main` is known.",
            ));
            return;
        }

        // Les couleurs, relevées du thème avant d'emprunter `ui` pour le
        // canevas. Celles d'un graphe doivent se voir : on ne prend pas
        // les fonds estompés qui servent aux fiches.
        let v = ui.visuals();
        let couleur_texte = v.text_color();
        let couleur_faible = v.weak_text_color();
        let couleur_accent = v.hyperlink_color;
        let couleur_cadre = v.widgets.noninteractive.fg_stroke.color;
        let fond_boîte = v.extreme_bg_color;
        let fond_accent = v.selection.bg_fill.gamma_multiply(0.45);
        let fond_survol = v.widgets.hovered.bg_fill;
        let couleur_trait = couleur_texte.gamma_multiply(0.55);

        let police = egui::FontId::monospace(12.0);
        let hauteur_ligne = ui.ctx().fonts_mut(|f| f.row_height(&police));
        let glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(&police, '0')).max(1.0);
        const PAD_X: f32 = 8.0;
        const PAD_Y: f32 = 4.0;
        const INDENT_X: f32 = 26.0;
        let hauteur_boîte = hauteur_ligne + PAD_Y * 2.0;
        let pas_vertical = hauteur_boîte + 8.0;

        // Poser chaque boîte : étiquette, rectangle local, et ce qu'elle vise.
        struct Boîte {
            rect: egui::Rect,
            label: String,
            addr: u64,
            accent: bool,
            renvoi: bool,
        }
        let mut boîtes: Vec<Boîte> = Vec::with_capacity(nœuds.len());
        let mut largeur_max = 0.0f32;
        for (i, nœud) in nœuds.iter().enumerate() {
            let f = &self.functions[nœud.fonction];
            let accent = matches!(f.origin, Origin::Entry | Origin::Main);
            let nom = ctx.display_name(f.start, &f.name());
            let label = if nœud.renvoi {
                // « ↺ » et non « ↩ » : la seconde flèche n'existe pas
                // dans les polices qu'egui embarque, et se peignait en
                // carré vide.
                format!("\u{21ba} {nom}  {:#x}", f.start)
            } else {
                format!("{nom}  {:#x}", f.start)
            };
            let largeur = label.chars().count() as f32 * glyphe + PAD_X * 2.0;
            let x = nœud.profondeur as f32 * INDENT_X;
            let y = i as f32 * pas_vertical;
            largeur_max = largeur_max.max(x + largeur);
            boîtes.push(Boîte {
                rect: egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::vec2(largeur, hauteur_boîte),
                ),
                label,
                addr: f.start,
                accent,
                renvoi: nœud.renvoi,
            });
        }
        let hauteur_totale = nœuds.len() as f32 * pas_vertical;

        let (réponse, peintre) = ui.allocate_painter(
            egui::vec2(largeur_max.max(120.0), hauteur_totale.max(40.0)),
            egui::Sense::click(),
        );
        let origine = réponse.rect.min.to_vec2();
        let décalé = |r: egui::Rect| r.translate(origine);
        let pointeur = réponse.hover_pos();

        // Les connecteurs d'abord : ils passent sous les boîtes. Un rail
        // vertical dans la gouttière de l'enfant, un coude, une flèche qui
        // entre dans la boîte appelée.
        let trait_ = egui::Stroke::new(1.5, couleur_trait);
        for (i, nœud) in nœuds.iter().enumerate() {
            let Some(parent) = nœud.parent else { continue };
            let rp = décalé(boîtes[parent].rect);
            let rn = décalé(boîtes[i].rect);
            let x_rail = rp.left() + 11.0;
            let y_enfant = rn.center().y;
            peintre.line_segment(
                [
                    egui::pos2(x_rail, rp.bottom()),
                    egui::pos2(x_rail, y_enfant),
                ],
                trait_,
            );
            peintre.line_segment(
                [
                    egui::pos2(x_rail, y_enfant),
                    egui::pos2(rn.left(), y_enfant),
                ],
                trait_,
            );
            flèche(&peintre, egui::pos2(rn.left(), y_enfant), couleur_trait);
        }

        // Puis les boîtes, et le clic qui mène à l'adresse.
        let mut demande: Option<ReRequest> = None;
        let mut sur_une_boîte = false;
        for boîte in &boîtes {
            let rect = décalé(boîte.rect);
            let survol = pointeur.is_some_and(|p| rect.contains(p));
            sur_une_boîte |= survol;
            let fond = if survol {
                fond_survol
            } else if boîte.accent {
                fond_accent
            } else {
                fond_boîte
            };
            peintre.rect_filled(rect, 4.0, fond);
            peintre.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(
                    if boîte.accent { 2.0 } else { 1.0 },
                    if boîte.accent {
                        couleur_accent
                    } else {
                        couleur_cadre
                    },
                ),
                egui::StrokeKind::Inside,
            );
            let couleur = if boîte.renvoi {
                couleur_faible
            } else {
                couleur_texte
            };
            peintre.text(
                egui::pos2(rect.left() + PAD_X, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &boîte.label,
                police.clone(),
                couleur,
            );
            if survol && réponse.clicked() {
                // Vers le CODE, sans demander : une boîte de ce graphe
                // est une fonction, et une fonction se lit désassemblée.
                // Le choix se pose pour une adresse écrite — une chaîne,
                // une table, un bourrage peuvent être l'un ou l'autre —,
                // pas pour celle-ci.
                demande = Some(ReRequest::GoTo {
                    addr: boîte.addr,
                    target: crate::GoTarget::Listing,
                });
            }
        }

        // Le curseur dit que les boîtes mènent quelque part.
        if sur_une_boîte {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
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

    /// Le détail de chaque fonction montrée : son nom (qu'on peut
    /// changer), son code binaire et son pseudo-code.
    fn dessiner_détails(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let mut demande: Option<ReRequest> = None;
        // On copie les rangs : le corps emprunte `self` pour lire le
        // désassemblage, et le module ne peut donc pas s'écrire dedans.
        let montrées = self.montrées.clone();
        for rang in montrées {
            let f = self.functions[rang].clone();
            // Les instructions d'abord : la tranche est liée à `ctx`, pas
            // à `self`, donc l'emprunt de `self` se termine ici et laisse
            // la place au tampon d'édition juste après.
            let instrs = self.instructions(ctx, &f);
            let nom_courant = ctx.display_name(f.start, &f.name());
            let défaut = f.name();
            let tampon = self
                .édition
                .entry(f.start)
                .or_insert_with(|| nom_courant.clone());
            let titre = format!(
                "{}   {:#010x}   {} {}",
                nom_courant,
                f.start,
                f.size(),
                ctx.t("octets", "bytes"),
            );
            // Rebaptisée à la main : un léger fond la distingue des
            // fonctions encore anonymes, dans une liste qui peut en
            // compter des centaines.
            let mut riche = egui::RichText::new(titre).monospace();
            if ctx.user_name(f.start).is_some() {
                riche = riche.background_color(ui.visuals().selection.bg_fill.gamma_multiply(0.25));
            }
            egui::CollapsingHeader::new(riche)
                .id_salt(("call_graph_detail", f.start))
                .default_open(false)
                .show(ui, |ui| {
                    // Renommer : on comprend une fonction, on lui donne un
                    // nom. « Défaut » rend son `sub_xxxx` — se dédire fait
                    // partie de nommer.
                    ui.horizontal(|ui| {
                        ui.label(ctx.t("Nom :", "Name:"));
                        let champ = ui.add(
                            egui::TextEdit::singleline(tampon)
                                .desired_width(200.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        let validé =
                            champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if framed_button(ui, ctx.t("Renommer", "Rename")).clicked() || validé {
                            demande = Some(ReRequest::Rename {
                                addr: f.start,
                                name: tampon.clone(),
                            });
                        }
                        if framed_button(ui, ctx.t("Défaut", "Default"))
                            .on_hover_text(
                                ctx.t("Rendre son nom d'origine", "Restore its original name"),
                            )
                            .clicked()
                        {
                            *tampon = défaut.clone();
                            demande = Some(ReRequest::Rename {
                                addr: f.start,
                                name: String::new(),
                            });
                        }
                    });
                    if let Some(cible) = clickable_address(ui, ctx, f.start) {
                        demande = Some(ReRequest::GoTo {
                            addr: f.start,
                            target: cible,
                        });
                    }
                    ui.add_space(4.0);
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

/// Une petite flèche pleine, pointant vers la droite, à l'entrée d'une
/// boîte appelée : c'est ce qui fait d'un trait un appel orienté.
fn flèche(peintre: &egui::Painter, bout: egui::Pos2, couleur: egui::Color32) {
    const T: f32 = 5.0;
    let arrière = bout - egui::vec2(T, 0.0);
    peintre.add(egui::Shape::convex_polygon(
        vec![
            bout,
            arrière + egui::vec2(0.0, T * 0.6),
            arrière - egui::vec2(0.0, T * 0.6),
        ],
        couleur,
        egui::Stroke::NONE,
    ));
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
        t.parse::<u64>()
            .ok()
            .or_else(|| u64::from_str_radix(t, 16).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn contexte<'a>(
        disasm: &'a [namsdec_disasm::DecodedInstruction],
        cpu: &'a Cpu,
    ) -> ReContext<'a> {
        ReContext {
            cpu,
            disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
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

    /// Un nom donné par l'utilisateur prend la place du nom par défaut,
    /// et l'absence de table rend le nom par défaut.
    #[test]
    fn a_user_name_replaces_the_default() {
        let mut noms = std::collections::BTreeMap::new();
        noms.insert(0x401000u64, "vérifie_clé".to_owned());
        let bytes: &[u8] = &[0x90, 0xc3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x401000);
        let cpu = Cpu::new(Memory::new(0x401000, 0x100));
        let avec = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: Some(&noms),
        };
        assert_eq!(avec.display_name(0x401000, "sub_401000"), "vérifie_clé");
        // Une adresse non nommée garde son défaut, même quand la table existe.
        assert_eq!(avec.display_name(0x402000, "sub_402000"), "sub_402000");
        // `user_name` dit s'il y a un nom donné — c'est ce qui sert à le
        // montrer partout (via `clickable_address`) et à le mettre en
        // évidence. Nommé : Some ; anonyme ou sans table : None.
        assert_eq!(avec.user_name(0x401000), Some("vérifie_clé"));
        assert_eq!(avec.user_name(0x402000), None);
        // Sans table, le défaut partout.
        let sans = ReContext {
            names: None,
            ..avec
        };
        assert_eq!(sans.display_name(0x401000, "sub_401000"), "sub_401000");
        assert_eq!(sans.user_name(0x401000), None);
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
