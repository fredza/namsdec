//! Le module « Fonctions » : par où commencer à lire.
//!
//! Devant un binaire dépouillé, la première question n'est pas « que
//! fait cette instruction » mais « où est le programme ». Deux cent
//! quarante kilo-octets de `.text` dont la quasi-totalité appartient à
//! la bibliothèque standard : ce qu'on cherche tient en quelques
//! centaines d'octets, et rien ne dit lesquels.
//!
//! Ce module répond en trois temps : le `main`, retrouvé par la
//! convention de démarrage ; les fonctions qu'on sait nommer ; et le
//! nombre d'appels qui mènent à chacune, parce qu'une fonction appelée
//! de quarante endroits est un utilitaire, et que celle qui n'est
//! appelée qu'une fois est le programme.

use std::collections::BTreeMap;

use crate::functions::{Function, Origin, by_importance, discover_with};
use crate::{ReContext, ReModule, ReRequest, ReSection, clickable_address, framed};

#[derive(Default)]
pub struct FunctionsModule {
    found: Vec<Function>,
    main: Option<u64>,
    entry: u64,
    /// Ce que le panneau demande à l'application : aller voir une
    /// adresse, ou rebaptiser une fonction. Relevé après le dessin.
    demande: Option<ReRequest>,
    /// La fonction dont on est en train de taper le nom.
    ///
    /// Une seule à la fois : une liste de quarante champs de saisie
    /// pèse, et un nom long y serait coupé sans qu'on puisse le lire.
    /// Au repos, le nom est une étiquette qu'on clique pour la changer.
    en_édition: Option<u64>,
    /// Celle dont le champ doit prendre le clavier à la prochaine
    /// image : on vient de l'ouvrir, et taper doit marcher tout de
    /// suite.
    à_focaliser: Option<u64>,
    /// Le nom en cours de frappe, par fonction.
    ///
    /// À part du nom retenu : tant qu'on tape, rien n'est posé — sans
    /// quoi « vérifie_ » deviendrait le nom de la fonction au premier
    /// caractère, et chaque lettre écrirait dans les notes du binaire.
    édition: BTreeMap<u64, String>,
}

/// Combien de fonctions la liste montre avant de s'arrêter.
///
/// Trois cents lignes ne se lisent pas : on montre les plus employées,
/// et le compte total dit ce qu'on ne montre pas.
const MONTRÉES: usize = 40;

impl ReModule for FunctionsModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Code
    }

    fn id(&self) -> &'static str {
        "functions"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Fonctions"
        } else {
            "Functions"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        // Le point d'entrée déclaré par le fichier, quand on l'a : c'est
        // de lui qu'on remonte jusqu'à `main`.
        self.entry = crate::elf_scan::scan(ctx.binary)
            .map(|elf| elf.entry)
            .or_else(|| ctx.disasm.first().map(|i| i.address))
            .unwrap_or(0);
        // Les appels dont la destination se CALCULE comptent comme les
        // autres, quand on a su la retrouver : chacun était une fonction
        // que la découverte ne voyait pas, et une arête perdue.
        let imports = crate::imports::scan(ctx.binary);
        let résolus: Vec<(u64, u64)> = crate::indirect::resolve(ctx.disasm, ctx.binary, &imports)
            .into_iter()
            .filter(|r| r.appel)
            .filter_map(|r| Some((r.at, r.target?)))
            .collect();
        self.found = discover_with(ctx.disasm, self.entry, &résolus);
        self.main = crate::functions::main_of(ctx.disasm, self.entry);
        // Un autre binaire, d'autres fonctions : les noms en cours de
        // frappe ne parlent plus de rien.
        self.édition.clear();
        self.en_édition = None;
        self.à_focaliser = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.found.is_empty() {
            ui.label(ctx.t(
                "Rien à découper : aucun code n'est chargé.",
                "Nothing to carve up: no code is loaded.",
            ));
            return;
        }

        match self.main {
            Some(main) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(ctx.t("Le programme commence à", "The program starts at"));
                    ui.monospace(format!("{main:#010x}"));
                });
                ui.weak(ctx.t(
                    "Trouvé par la convention de démarrage : le code d'amorçage passe \
                     l'adresse de `main` à `__libc_start_main`, dans RDI. C'est une \
                     convention, pas une loi — un programme peut s'en passer.",
                    "Found through the startup convention: the bootstrap code hands the \
                     address of `main` to `__libc_start_main`, in RDI. That is a \
                     convention, not a law — a program may do without it.",
                ));
            }
            None => {
                ui.label(ctx.t(
                    "Le `main` n'a pas été retrouvé : ce programme ne suit pas la \
                     convention de démarrage de la bibliothèque C.",
                    "No `main` was found: this program does not follow the C library's \
                     startup convention.",
                ));
            }
        }
        ui.add_space(4.0);
        ui.weak(format!(
            "{} {}",
            self.found.len(),
            ctx.t(
                "fonctions retrouvées — les cibles d'appel, et rien d'autre : ce qu'on \
                 n'appelle que par pointeur échappe à ce relevé.",
                "functions recovered — call targets, and nothing else: what is only \
                 called through a pointer escapes this survey.",
            )
        ));
        ui.add_space(4.0);

        // RENOMMER SE FAIT ICI, et le nom donné vaut partout : c'est la
        // même table que le listing, l'arbre d'appels et les
        // références croisées — les « notes » du binaire, gardées d'une
        // séance à l'autre. Comprendre ce que fait `sub_1d491` et ne
        // pas pouvoir l'appeler `vérifie_mdp` dans la liste même des
        // fonctions était le défaut : il fallait passer par l'arbre
        // d'appels pour nommer ce qu'on lisait ici.
        ui.weak(ctx.t(
            "Cliquez un nom pour le changer, et validez par Entrée. Il vaut partout — \
             listing, arbre d'appels, références croisées — et survit à la fermeture ; \
             la flèche de retour rend son nom d'origine.",
            "Click a name to change it, and press Enter. It holds everywhere — \
             listing, call tree, cross-references — and survives closing; the back \
             arrow restores the original one.",
        ));
        ui.add_space(4.0);

        // Relevé pendant le dessin, posé après : la grille emprunte
        // `self`, et le module ne peut donc pas s'écrire dedans.
        let mut demande: Option<ReRequest> = None;
        let rangées: Vec<Function> = by_importance(&self.found)
            .into_iter()
            .take(MONTRÉES)
            .cloned()
            .collect();
        // Une rangée par fonction, et non une grille : dans une
        // grille, egui donne à chaque cellule ce qui reste de la
        // largeur une fois les autres colonnes posées — le champ du
        // nom se retrouvait rogné à vingt-quatre pixels dès que la
        // fenêtre d'analyse était étroite, ce qui est précisément sa
        // taille par défaut une fois la colonne de navigation prise.
        // Une rangée, elle, donne à chacun la largeur qu'on lui dit.
        let place = ui.available_width();
        let étroit = place < 560.0;
        // Ce qu'on retranche est la place des autres colonnes, mesurée
        // sur le pire cas : l'adresse sur dix chiffres, le bouton, et
        // « 12 appels ». Trop peu, et la rangée court au-delà du bord.
        let largeur_nom = (place - if étroit { 260.0 } else { 340.0 }).clamp(70.0, 220.0);
        let hauteur = ui.spacing().interact_size.y;
        for fonction in &rangées {
            let défaut = fonction.name();
            let nom = ctx.display_name(fonction.start, &défaut);
            let rebaptisée = ctx.user_name(fonction.start).is_some();
            let en_édition = self.en_édition == Some(fonction.start);
            let à_focaliser = self.à_focaliser == Some(fonction.start);
            let tampon = self
                .édition
                .entry(fonction.start)
                .or_insert_with(|| nom.clone());
            let mut ouvrir = None;
            let mut fermer = false;
            ui.horizontal(|ui| {
                if en_édition {
                    let champ = egui::TextEdit::singleline(tampon)
                        .desired_width(largeur_nom)
                        .font(egui::TextStyle::Monospace);
                    let réponse = ui.add_sized([largeur_nom, hauteur], champ);
                    if à_focaliser {
                        réponse.request_focus();
                    }
                    // Entrée pose le nom ; perdre le champ des yeux le
                    // pose aussi — on ne perd pas ce qu'on vient de
                    // taper parce qu'on a cliqué ailleurs. Échap se
                    // dédit.
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        *tampon = nom.clone();
                        fermer = true;
                    } else if réponse.lost_focus() {
                        demande = Some(ReRequest::Rename {
                            addr: fonction.start,
                            name: tampon.clone(),
                        });
                        fermer = true;
                    }
                } else {
                    // Le nom au repos est une étiquette, non un champ :
                    // quarante champs de saisie dans une liste pèsent,
                    // et surtout un nom long y serait coupé sans qu'on
                    // puisse le lire. Tronquée, l'étiquette promet
                    // l'infobulle — et le clic ouvre la saisie.
                    let mut riche = egui::RichText::new(&nom).monospace();
                    if rebaptisée {
                        riche = riche
                            .background_color(ui.visuals().selection.bg_fill.gamma_multiply(0.25));
                    }
                    if matches!(fonction.origin, Origin::Main | Origin::Entry) {
                        riche = riche.strong();
                    }
                    // Aligné à gauche, et dans toute la largeur de la
                    // colonne : `add_sized` CENTRE ce qu'on lui donne,
                    // et les noms se retrouvaient en escalier, chacun
                    // flottant au milieu de sa case. Une colonne de
                    // noms se lit par son bord gauche.
                    let réponse = ui
                        .allocate_ui_with_layout(
                            egui::vec2(largeur_nom, hauteur),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                // La largeur est IMPOSÉE : sans elle,
                                // la case se rétrécit sur le texte, et
                                // la flèche de retour comme l'adresse
                                // suivent le nom en escalier au lieu
                                // de former une colonne.
                                ui.set_min_width(largeur_nom);
                                let réponse = ui.add(
                                    egui::Label::new(riche)
                                        .truncate()
                                        .sense(egui::Sense::click()),
                                );
                                // Le trait au survol, mais pas la main :
                                // ce clic-ci ne mène nulle part, il ouvre
                                // la saisie du nom — d'où le curseur de
                                // texte, posé plus bas.
                                crate::souligne_au_survol(ui, &réponse);
                                réponse
                            },
                        )
                        .inner
                        .on_hover_cursor(egui::CursorIcon::Text)
                        .on_hover_text(format!(
                            "{nom}\n{}",
                            ctx.t(
                                "Cliquez pour renommer — le nom vaut partout, et survit à la \
                                 fermeture",
                                "Click to rename — the name holds everywhere, and survives \
                                 closing",
                            )
                        ));
                    if réponse.clicked() {
                        *tampon = nom.clone();
                        ouvrir = Some(fonction.start);
                    }
                }
                // Se dédire fait partie de nommer : le bouton rend son
                // `sub_xxxx` à la fonction. Éteint quand il n'y a rien
                // à défaire.
                if ui
                    .add_enabled(rebaptisée, framed(ui, "\u{21ba}"))
                    .on_hover_text(ctx.t("Rendre son nom d'origine", "Restore its original name"))
                    .clicked()
                {
                    *tampon = défaut.clone();
                    demande = Some(ReRequest::Rename {
                        addr: fonction.start,
                        name: String::new(),
                    });
                }
                // L'adresse mène au listing, comme partout ailleurs :
                // lire « 0x1d491 » sans pouvoir y aller était
                // s'arrêter au milieu du geste.
                if let Some(cible) = clickable_address(ui, ctx, fonction.start) {
                    demande = Some(ReRequest::GoTo {
                        addr: fonction.start,
                        target: cible,
                    });
                }
                if !étroit {
                    ui.weak(format!("{} {}", fonction.size(), ctx.t("octets", "bytes")));
                }
                let appels = format!(
                    "{} {}",
                    fonction.callers,
                    if fonction.callers > 1 {
                        ctx.t("appels", "calls")
                    } else {
                        ctx.t("appel", "call")
                    }
                );
                ui.weak(appels);
            });
            if let Some(addr) = ouvrir {
                self.en_édition = Some(addr);
                self.à_focaliser = Some(addr);
            } else if à_focaliser {
                // Le focus a été demandé à l'image précédente : il est
                // posé, on n'y revient plus — sinon le champ reprendrait
                // le clavier à chaque image.
                self.à_focaliser = None;
            }
            if fermer {
                self.en_édition = None;
                self.à_focaliser = None;
            }
        }
        if demande.is_some() {
            self.demande = demande;
        }
        if self.found.len() > MONTRÉES {
            ui.weak(format!(
                "… {} {}",
                self.found.len() - MONTRÉES,
                ctx.t("de plus", "more")
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// Le module nomme le point de départ, et ne prétend rien quand la
    /// convention n'est pas suivie.
    #[test]
    fn the_module_names_where_to_start_or_says_it_cannot() {
        // Un démarrage en règle : lea rdi, [rip+0x10] puis call.
        let bytes: &[u8] = &[
            0xF3, 0x0F, 0x1E, 0xFA, 0x48, 0x8D, 0x3D, 0x10, 0x00, 0x00, 0x00, 0xFF, 0x15, 0x00,
            0x20, 0x00, 0x00, 0xF4,
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = FunctionsModule::default();
        module.on_binary_loaded(&ctx);
        // `lea` à 0x1004, longue de sept octets : 0x100b + 0x10.
        assert_eq!(module.main, Some(0x101b));
        assert!(!module.found.is_empty());
    }
}
