//! Le panneau du détour : ajouter du code à un binaire sans le décaler.
//!
//! Il existe pour expliquer une idée, et cette idée est belle : on ne
//! peut pas agrandir un exécutable sans fausser toutes ses adresses,
//! mais on peut le **détourner** par un trou qu'il contient déjà.
//! Comprendre cela, c'est comprendre du même coup pourquoi un binaire
//! n'est pas plein, pourquoi une adresse relative n'est pas une adresse,
//! et pourquoi un correctif se montre avant de se poser.
//!
//! Rien ne s'écrit ici. Le panneau calcule un **plan**, le montre en
//! entier — ce qui sera recouvert, ce qui sera écrit, et où —, et c'est
//! seulement sur un geste explicite que l'application le pose. Les deux
//! changements partent sous un même numéro de groupe, et se défont
//! ensemble par la fenêtre Correctifs.

use crate::patch::{self, Detour, DetourRefusé};
use crate::slack::{self, Slack};
use crate::vue::{Bloc, Fragment, Ton, Vue};
use crate::{ReContext, ReModule, ReRequest, ReSection, framed_button, framed_small_button};

/// Comment l'utilisateur écrit le code à ajouter.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ModeAjout {
    /// En assembleur, assemblé par nasm à l'adresse où il atterrira.
    ///
    /// Le défaut, parce que c'est la façon dont on PENSE le geste :
    /// « écris 0x42 à cette adresse », pas « pose 41 c6 03 42 ».
    #[default]
    Assembleur,
    /// En octets hexadécimaux, pour qui les a déjà.
    ///
    /// Gardé parce que nasm peut manquer, et parce qu'on veut parfois
    /// poser une suite d'octets précise sans passer par un assembleur
    /// qui choisirait un encodage équivalent mais différent.
    Octets,
}

#[derive(Default)]
pub struct DetourModule {
    /// Ce que le panneau demande à l'application. Relevé après le
    /// dessin, jamais pendant.
    demande: Option<ReRequest>,
    /// L'adresse visée, telle qu'elle est tapée.
    site: String,
    /// Le code à ajouter, en assembleur ou en octets.
    code: String,
    mode: ModeAjout,
    /// Les places libres exécutables, la plus grande d'abord. Les autres
    /// ne sont pas offertes : un détour vers des données ne s'exécute
    /// pas, et proposer un choix qui échoue toujours n'apprend rien.
    trous: Vec<Slack>,
    /// Laquelle est choisie.
    choix: usize,
    /// Le dernier plan calculé, ou la raison du refus. `None` tant qu'on
    /// n'a rien demandé : un plan ne se calcule pas à chaque image, il
    /// lance nasm.
    plan: Option<Result<Box<Detour>, DetourRefusé>>,
    /// L'assembleur a refusé le texte — ses diagnostics, tels quels.
    erreur_asm: Option<String>,
    /// nasm a-t-il été trouvé ? Relevé une fois, au chargement.
    nasm: bool,
}

/// Poser le détour que le plan décrit.
const ACTION_POSER: &str = "détour:poser";

impl DetourModule {
    /// Le plan : ce qui est recouvert, ce qui part, ce qui le remplace.
    ///
    /// La part de ce panneau qui est de la COMPRÉHENSION, et la seule
    /// qui passe la frontière — voir le commentaire de `ui`. Elle rend
    /// `None` tant qu'on n'a rien calculé : il n'y a alors pas de plan
    /// à dire.
    fn vue_du_plan(&self, ctx: &ReContext) -> Option<Vue> {
        let plan = self.plan.as_ref()?;
        let detour = match plan {
            Err(refus) => {
                return Some(Vue::neuve().texte(
                    Fragment::dit(refus.message(ctx.lang.starts_with("fr"))).ton(Ton::Attention),
                ));
            }
            Ok(detour) => detour,
        };

        let mut dedans = vec![
            // Ce qui part, et ce qui le remplace. La question que tout
            // le monde se pose devant un correctif binaire est
            // « qu'est-ce que je perds ? » — on y répond d'abord.
            Bloc::Texte(
                format!(
                    "{} {} {}",
                    ctx.t("Recouvert :", "Covered:"),
                    detour.recouvert,
                    ctx.t("octets", "bytes"),
                )
                .into(),
            ),
        ];
        dedans.extend(blocs_d_instructions(
            ctx,
            &detour.aller.before,
            detour.aller.addr,
            ctx.t("ce qui part", "what goes"),
        ));
        dedans.extend(blocs_d_instructions(
            ctx,
            &detour.aller.after,
            detour.aller.addr,
            ctx.t("ce qui le remplace", "what replaces it"),
        ));
        dedans.push(Bloc::Texte(
            vec![
                Fragment::dit(ctx.t("Le corps, écrit en", "The body, written at")),
                Fragment::dit(format!("{:#x}", detour.corps.addr))
                    .code()
                    .mène_à(detour.corps.addr),
            ]
            .into(),
        ));
        dedans.extend(blocs_d_instructions(
            ctx,
            &detour.corps.after,
            detour.corps.addr,
            ctx.t("le détour", "the detour"),
        ));

        // Les instructions déplacées ont pu être RÉÉCRITES, et c'est la
        // chose la plus instructive du panneau : le dire quand c'est
        // arrivé, parce que sinon personne ne remarque que les octets
        // ont changé sans que l'instruction change.
        if detour.déplacé != detour.aller.before[..detour.déplacé.len()] {
            dedans.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Les instructions déplacées ont été relocalisées : leur \
                     écart a été recalculé pour qu'elles visent toujours la \
                     même chose depuis leur nouvelle adresse. L'instruction \
                     ne change pas ; ses octets, si.",
                    "The moved instructions were relocated: their offset was \
                     recomputed so they still target the same thing from \
                     their new address. The instruction does not change; its \
                     bytes do.",
                ))
                .faible()
                .into(),
            ));
        }
        dedans.push(Bloc::Texte(
            Fragment::dit(ctx.t("Poser le détour", "Place the detour"))
                .bouton(ACTION_POSER)
                .survol(ctx.t(
                    "Les deux changements, ensemble — et défaisables ensemble \
                     par la fenêtre Correctifs",
                    "Both changes, together — and undoable together from the \
                     Patches window",
                ))
                .into(),
        ));
        Some(Vue {
            blocs: vec![Bloc::Section {
                titre: ctx.t("Le plan", "The plan").into(),
                blocs: dedans,
            }],
        })
    }

    /// Les octets de l'ajout, assemblés ou décodés selon le mode.
    ///
    /// L'adresse compte : `assemble_one` pose un `[ORG]`, donc du code
    /// qui s'adresse relativement est assemblé pour l'endroit où il
    /// atterrira vraiment. C'est pour cela que la place libre se choisit
    /// AVANT d'assembler.
    fn octets_de_l_ajout(&self, adresse: u64) -> Result<Vec<u8>, String> {
        let texte = self.code.trim();
        if texte.is_empty() {
            // Un détour sans ajout est légitime : il déplace du code et
            // le ramène, sans rien changer. C'est le cas d'école qu'on
            // veut pouvoir poser pour voir le mécanisme seul.
            return Ok(Vec::new());
        }
        match self.mode {
            ModeAjout::Assembleur => {
                namsdec_asm::assemble_one(texte, adresse).map_err(|e| phrase_de_l_asm(&e))
            }
            ModeAjout::Octets => décoder_hex(texte),
        }
    }

    /// Calcule le plan, et le garde. Ne touche à rien du binaire.
    fn calculer(&mut self, ctx: &ReContext) {
        self.plan = None;
        self.erreur_asm = None;

        let Some(site_vaddr) = adresse_lue(&self.site) else {
            // VIDE et ILLISIBLE ne sont pas le même défaut, et les
            // confondre coûtait cher : « attendu de l'hexadécimal »
            // devant un champ vide fait chercher une faute de frappe
            // dans ce qu'on n'a pas écrit.
            self.erreur_asm = Some(
                if self.site.trim().is_empty() {
                    ctx.t(
                        "Le champ « Détourner depuis » est vide : donnez l'adresse de \
                         l'instruction à détourner, ou cliquez « ici » pour prendre \
                         celle où l'exécution en est.",
                        "The “Detour from” field is empty: give the address of the \
                         instruction to detour, or click “here” to take the one \
                         execution has reached.",
                    )
                } else {
                    ctx.t(
                        "Adresse illisible — attendu de l'hexadécimal, par exemple 0x401136.",
                        "Unreadable address — hexadecimal expected, for instance 0x401136.",
                    )
                }
                .to_owned(),
            );
            return;
        };
        let Some(place) = self.trous.get(self.choix).cloned() else {
            return;
        };
        let Some(site_offset) = crate::imports::file_offset(ctx.binary, site_vaddr) else {
            self.erreur_asm = Some(
                ctx.t(
                    "Cette adresse n'est dans aucun segment chargé du fichier.",
                    "This address is in no loaded segment of the file.",
                )
                .to_owned(),
            );
            return;
        };

        let ajout = match self.octets_de_l_ajout(place.vaddr) {
            Ok(octets) => octets,
            Err(message) => {
                self.erreur_asm = Some(message);
                return;
            }
        };

        self.plan = Some(
            patch::plan_detour(
                ctx.binary,
                64,
                site_offset as usize,
                site_vaddr,
                &place,
                &ajout,
                ctx.patches,
            )
            .map(Box::new),
        );
    }
}

impl ReModule for DetourModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Bytes
    }

    fn id(&self) -> &'static str {
        "detour"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Détourner (ajouter du code)"
        } else {
            "Detour (add code)"
        }
    }

    fn nav_title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Détourner"
        } else {
            "Detour"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.trous = slack::find(ctx.binary)
            .into_iter()
            .filter(|t| t.executable)
            .collect();
        self.choix = 0;
        // LE CHAMP EST PRÉREMPLI D'UNE VRAIE ADRESSE.
        //
        // Il montrait un EXEMPLE en gris — « 0x401136 » —, et un exemple
        // en gris dans un champ se lit comme une valeur. On clique
        // « Calculer le plan », et l'on reçoit « Adresse illisible —
        // attendu de l'hexadécimal, par exemple 0x401136 » en regard
        // d'un champ qui semble porter exactement cela. Vu à l'écran le
        // 16 octobre 2026 : la personne avait d'ailleurs tapé l'adresse
        // dans la boîte du CODE, la seule qui parût vide.
        //
        // Préremplie, la première instruction du listing n'est pas un
        // exemple : c'est un point de départ qu'on peut calculer tel
        // quel, et qu'on remplace en tapant par-dessus.
        if let Some(première) = ctx.disasm.first() {
            self.site = format!("{:#x}", première.address);
        }
        // Un plan calculé sur l'image d'avant ne vaut plus rien : le
        // garder à l'écran laisserait poser des octets d'après un
        // fichier qui a changé.
        self.plan = None;
        self.erreur_asm = None;
        self.nasm = namsdec_asm::find_nasm().is_some();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        ui.weak(ctx.t(
            "Ajouter du code à un exécutable sans le décaler. Un octet de plus \
             fausserait toutes les adresses qui suivent — alors on ne pousse \
             rien : on pose un saut vers un trou que le binaire contient déjà, \
             on y écrit le code neuf, on y rejoue les instructions recouvertes, \
             et on revient. Le fichier garde sa taille exacte.",
            "Adding code to an executable without shifting it. One extra byte \
             would falsify every address that follows — so nothing is pushed: a \
             jump is placed towards a gap the binary already contains, the new \
             code is written there, the covered instructions are replayed, and \
             control comes back. The file keeps its exact size.",
        ));

        if self.trous.is_empty() {
            ui.add_space(6.0);
            ui.weak(ctx.t(
                "Aucune place libre exécutable dans ce fichier — rien à détourner \
                 vers. Le panneau « Place libre » montre ce qu'il y a.",
                "No executable slack in this file — nowhere to detour to. The \
                 “Free space” panel shows what there is.",
            ));
            return;
        }

        ui.add_space(8.0);

        // --- L'endroit visé ---------------------------------------
        //
        // REPLIABLE, et c'est ce qui tient la largeur. Une rangée
        // `horizontal` ne recule pas devant le manque de place : elle
        // dépasse, et la largeur disponible qu'elle laisse derrière
        // elle est celle de son débordement — si bien que les textes
        // SUIVANTS se replient sur une largeur qu'on ne voit pas. Le
        // même défaut avait été corrigé sur la rangée d'onglets du
        // processeur le 23 octobre 2026.
        ui.horizontal_wrapped(|ui| {
            ui.label(ctx.t("Détourner depuis", "Detour from"));
            ui.add(
                egui::TextEdit::singleline(&mut self.site)
                    .desired_width(110.0)
                    // Le gris dit « exemple » et non « valeur » : seule
                    // la forme le disait, et la forme d'une adresse est
                    // une adresse.
                    .hint_text(ctx.t("ex. 0x401136", "e.g. 0x401136"))
                    .font(egui::TextStyle::Monospace),
            );
            // L'adresse où l'émulateur en est : c'est presque toujours
            // celle qu'on veut, et la retaper à la main est une corvée
            // qui fait recopier de travers.
            if framed_small_button(ui, ctx.t("ici", "here"))
                .on_hover_text(ctx.t(
                    "Prendre l'adresse où l'exécution en est",
                    "Take the address execution has reached",
                ))
                .clicked()
            {
                self.site = format!("{:#x}", ctx.cpu.regs.rip);
            }
        });

        // --- La place libre où atterrir ----------------------------
        ui.add_space(4.0);
        // L'ÉTIQUETTE AU-DESSUS, ET NON À CÔTÉ. Dans une rangée
        // repliable, `available_width` rend la largeur de la RANGÉE et
        // non ce qu'il en reste — c'est ce qui permet à un enfant de
        // décider s'il passe à la ligne. La liste déroulante y lisait
        // donc 220 et s'ajoutait aux 110 de l'étiquette : la rangée
        // débordait de 107 px, le `max_rect` du panneau grandissait
        // d'autant, et tous les textes SUIVANTS se repliaient sur une
        // largeur qu'on ne voit pas — jusqu'au message de refus.
        // Au-dessus, la largeur lue est la vraie.
        ui.label(ctx.t("vers la place libre", "towards the slack"));
        {
            let ui = &mut *ui;
            let étiquette = self
                .trous
                .get(self.choix)
                .map(|t| étiquette_du_trou(t, ctx))
                .unwrap_or_default();
            // UN PLAFOND, ET NON UNE LARGEUR. 230 px en dur dans un
            // panneau plus étroit n'est pas seulement un choix trop
            // large : la liste déroulante allonge la rangée, la rangée
            // allonge la largeur disponible, et TOUT CE QUI SUIT se
            // replie sur une largeur qui n'est pas visible. Mesuré à
            // 220 px : cinq textes coupés, dont le message de refus,
            // alors qu'ils se replient tous correctement. Lire la place
            // offerte comme un maximum ne reboucle pas — le contenu en
            // prend moins, jamais plus.
            egui::ComboBox::from_id_salt("detour_slack")
                .selected_text(étiquette.clone())
                .width(ui.available_width().clamp(90.0, 230.0))
                // TRONQUER, sans quoi le plafond ne sert à rien : une
                // liste déroulante prend la largeur de son texte si
                // celui-ci dépasse ce qu'on lui a dit. « 0x00401100 ·
                // 256 octets · .text » demande deux cent vingt pixels à
                // lui seul, et la rangée en débordait de cent sept.
                .truncate()
                .show_ui(ui, |ui| {
                    for (rang, trou) in self.trous.iter().enumerate() {
                        ui.selectable_value(&mut self.choix, rang, étiquette_du_trou(trou, ctx));
                    }
                })
                .response
                // ET LE TEXTE TRONQUÉ SE LIT ENCORE EN ENTIER. Une
                // troncature sans survol cache une information qu'on
                // n'a pas d'autre moyen d'obtenir : ici l'adresse, la
                // taille et la section du trou où le code atterrira.
                .on_hover_text(étiquette);
        }

        // --- Le code à ajouter -------------------------------------
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(ctx.t("Code à ajouter", "Code to add"));
            ui.selectable_value(
                &mut self.mode,
                ModeAjout::Assembleur,
                ctx.t("assembleur", "assembly"),
            );
            ui.selectable_value(&mut self.mode, ModeAjout::Octets, ctx.t("octets", "bytes"));
        });
        if self.mode == ModeAjout::Assembleur && !self.nasm {
            ui.weak(ctx.t(
                "nasm est introuvable : l'assembleur ne répondra pas. Passez par \
                 les octets, ou installez-le.",
                "nasm not found: the assembler will not answer. Use bytes \
                 instead, or install it.",
            ));
        }
        ui.add(
            egui::TextEdit::multiline(&mut self.code)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .code_editor()
                .hint_text(match self.mode {
                    ModeAjout::Assembleur => "mov r11, 0x404030\nmov byte [r11], 0x42",
                    ModeAjout::Octets => "41 bb 30 40 40 00 41 c6 03 42",
                }),
        );
        ui.weak(ctx.t(
            "Laissé vide, le détour déplace le code et le ramène sans rien \
             changer — de quoi voir le mécanisme seul.",
            "Left empty, the detour moves the code and brings it back unchanged \
             — enough to see the mechanism alone.",
        ));

        // --- Calculer, puis montrer avant de poser -----------------
        ui.add_space(8.0);
        if framed_button(ui, ctx.t("Calculer le plan", "Compute the plan"))
            .on_hover_text(ctx.t(
                "Rien n'est écrit : le plan se montre d'abord",
                "Nothing is written: the plan is shown first",
            ))
            .clicked()
        {
            self.calculer(ctx);
        }

        if let Some(message) = &self.erreur_asm {
            ui.add_space(6.0);
            ui.colored_label(ui.visuals().error_fg_color, message);
        }

        // LE PLAN PASSE LA FRONTIÈRE, LE FORMULAIRE RESTE PEINT.
        //
        // C'est la même coupe que pour l'éditeur binaire, le vidage et
        // les chaînes, et ici elle était écrite d'avance : `vue.rs`
        // dit, à propos des treize champs du projet, que « le
        // treizième — le code d'un détour — est un éditeur de
        // plusieurs lignes, et attendra sa propre forme ». Ce panneau
        // cumule les TROIS formes qui manquent au vocabulaire — une
        // liste déroulante, deux étiquettes à bascule, un éditeur
        // multiligne —, et toutes les trois n'ont qu'un usage : le
        // sien. C'est le signal d'arrêt que `vue.rs` pose en tête.
        //
        // Mais ce que ce panneau COMPREND, lui, passe : le plan. Ce
        // qui est recouvert, ce qui part, ce qui le remplace, le corps
        // écrit ailleurs, et l'avertissement sur les instructions
        // relocalisées. C'est exactement ce qu'un rapport veut d'un
        // détour — et il n'a que faire du formulaire qui a servi à le
        // calculer.
        let mut demande = None;
        if let Some(vue) = self.vue_du_plan(ctx) {
            let gestes = crate::vue_peinte::peindre_tout(ui, ctx, &vue);
            if gestes.demande.is_some() {
                demande = gestes.demande;
            }
            if gestes.actions.iter().any(|a| a == ACTION_POSER)
                && let Some(Ok(detour)) = &self.plan
            {
                demande = Some(ReRequest::PatchDetour {
                    detour: detour.clone(),
                });
            }
        }
        if demande.is_some() {
            self.demande = demande;
            // Le plan vient d'être posé ou l'on part ailleurs : dans les
            // deux cas il ne décrit plus ce qui est dans le fichier.
            // `on_binary_loaded` le remettra à neuf au rechargement.
            self.erreur_asm = None;
        }
    }
}

/// Une place libre, dite en une ligne.
fn étiquette_du_trou(trou: &Slack, ctx: &ReContext) -> String {
    format!(
        "{:#010x} · {} {} · {}",
        trou.vaddr,
        trou.len,
        ctx.t("octets", "bytes"),
        trou.section.as_deref().unwrap_or("—"),
    )
}

/// Les octets, désassemblés, sous un intertitre.
///
/// Montrer les octets seuls ne dit rien à personne ; montrer
/// l'instruction sans les octets cache ce qui change quand une
/// relocalisation réécrit un écart. Les deux, donc, côte à côte.
fn blocs_d_instructions(_ctx: &ReContext, octets: &[u8], base: u64, titre: &str) -> Vec<Bloc> {
    let lignes = namsdec_disasm::disassemble(octets, base);
    if lignes.is_empty() {
        return vec![
            Bloc::Texte(Fragment::dit(titre).faible().into()),
            Bloc::Texte(Fragment::dit(crate::digest::hex(octets)).code().into()),
        ];
    }
    vec![
        Bloc::Texte(Fragment::dit(titre).faible().into()),
        Bloc::Tableau {
            colonnes: Vec::new(),
            rangées: lignes
                .iter()
                .map(|ligne| {
                    vec![
                        Fragment::dit(format!("{:#010x}", ligne.address))
                            .code()
                            .into(),
                        Fragment::dit(crate::digest::hex(&ligne.bytes))
                            .code()
                            .into(),
                        Fragment::dit(&ligne.text).code().into(),
                    ]
                })
                .collect(),
            rayé: true,
        },
    ]
}

/// Ce que l'assembleur a répondu, mis en une phrase.
///
/// Les diagnostics de nasm arrivent déjà rédigés, en anglais, et il n'y a
/// pas à les réécrire — on les rend tels quels, avec leur numéro de
/// ligne, parce que c'est ce qui permet de corriger. Seul le cas « nasm
/// introuvable » se dit dans la langue de l'interface : c'est le seul où
/// le message vient de namsdec et non de l'outil.
fn phrase_de_l_asm(erreur: &namsdec_asm::AsmError) -> String {
    match erreur {
        namsdec_asm::AsmError::NasmNotFound => {
            "nasm est introuvable — passez par les octets, ou installez-le.".to_owned()
        }
        namsdec_asm::AsmError::Failed { diagnostics } => diagnostics
            .iter()
            .map(|d| format!("ligne {} : {}", d.line, d.message))
            .collect::<Vec<_>>()
            .join("\n"),
        autre => autre.to_string(),
    }
}

/// Une adresse hexadécimale, avec ou sans `0x`.
fn adresse_lue(texte: &str) -> Option<u64> {
    let propre = texte.trim();
    let propre = propre
        .strip_prefix("0x")
        .or_else(|| propre.strip_prefix("0X"))
        .unwrap_or(propre);
    if propre.is_empty() {
        return None;
    }
    u64::from_str_radix(propre, 16).ok()
}

/// Des octets écrits à la main, espacés comme on veut.
fn décoder_hex(texte: &str) -> Result<Vec<u8>, String> {
    let propre: String = texte.chars().filter(|c| !c.is_whitespace()).collect();
    if propre.chars().any(|c| !c.is_ascii_hexdigit()) {
        return Err("Octets illisibles — attendu de l'hexadécimal.".to_owned());
    }
    if !propre.len().is_multiple_of(2) {
        return Err("Nombre impair de chiffres — un octet en fait deux.".to_owned());
    }
    propre
        .as_bytes()
        .chunks(2)
        .map(|paire| {
            let texte = std::str::from_utf8(paire).map_err(|_| "Octets illisibles.".to_owned())?;
            u8::from_str_radix(texte, 16).map_err(|_| "Octets illisibles.".to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn contexte<'a>(binary: &'a [u8], cpu: &'a Cpu) -> ReContext<'a> {
        ReContext {
            cpu,
            disasm: &[],
            source: "",
            binary,
            lang: "fr",
            names: None,
            patches: &[],
        }
    }

    /// Un module frais ne propose rien et ne panique pas.
    #[test]
    fn a_fresh_module_offers_nothing() {
        let module = DetourModule::default();
        assert!(module.trous.is_empty());
        assert!(module.plan.is_none());
        assert_eq!(module.mode, ModeAjout::Assembleur);
    }

    /// Ce qui n'est pas un ELF le laisse vide.
    #[test]
    fn a_non_elf_leaves_it_empty() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let mut module = DetourModule::default();
        module.on_binary_loaded(&contexte(b"pas un elf", &cpu));
        assert!(module.trous.is_empty());
    }

    /// Seules les places EXÉCUTABLES sont offertes.
    ///
    /// Le panneau de lecture les montre toutes, parce que les voir
    /// apprend quelque chose. Celui-ci n'en offre que les utilisables :
    /// `slack::find` trie par taille, et les plus grands trous d'un
    /// binaire sont souvent dans les données — on proposerait donc en
    /// tête un choix qui échoue toujours.
    #[test]
    fn only_executable_slack_is_offered() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let mut module = DetourModule::default();
        // Un vrai ELF minimal n'est pas nécessaire : ce qu'on vérifie
        // est le filtre, et il porte sur ce que `find` a rendu.
        module.on_binary_loaded(&contexte(b"", &cpu));
        assert!(module.trous.iter().all(|t| t.executable));
    }

    /// Une adresse se lit avec ou sans `0x`, et rien d'autre ne passe.
    #[test]
    fn an_address_is_read_either_way() {
        assert_eq!(adresse_lue("0x401136"), Some(0x401136));
        assert_eq!(adresse_lue("401136"), Some(0x401136));
        assert_eq!(adresse_lue("0X401136"), Some(0x401136));
        assert_eq!(adresse_lue("  0x401136  "), Some(0x401136));
        assert_eq!(adresse_lue(""), None);
        assert_eq!(adresse_lue("0x"), None);
        assert_eq!(adresse_lue("pas une adresse"), None);
    }

    /// Les octets se donnent espacés comme on veut, et un nombre impair
    /// de chiffres est refusé plutôt que complété.
    ///
    /// Compléter serait le pire des deux : « 41c6034 » deviendrait
    /// « 41 c6 03 40 » ou « 41 c6 03 04 » selon le côté choisi, et
    /// l'utilisateur poserait des octets qu'il n'a pas écrits.
    #[test]
    fn bytes_are_read_spaced_and_odd_digits_are_refused() {
        assert_eq!(décoder_hex("41bb"), Ok(vec![0x41, 0xbb]));
        assert_eq!(décoder_hex("41 bb"), Ok(vec![0x41, 0xbb]));
        assert_eq!(décoder_hex(" 41  bb \n"), Ok(vec![0x41, 0xbb]));
        assert!(décoder_hex("41b").is_err(), "nombre impair");
        assert!(décoder_hex("41zz").is_err(), "pas de l'hexadécimal");
    }

    /// Un ajout vide est légitime : le détour se voit tout seul.
    #[test]
    fn an_empty_addition_is_allowed() {
        let module = DetourModule::default();
        assert_eq!(module.octets_de_l_ajout(0x1000), Ok(Vec::new()));
    }

    /// En mode octets, l'assembleur n'est pas appelé — donc nasm n'est
    /// pas nécessaire.
    ///
    /// Ce contrôle garde une promesse de l'interface : « passez par les
    /// octets » doit marcher sur une machine sans nasm. Il ne lit pas
    /// l'état de la machine pour autant — il vérifie que ce chemin
    /// n'emprunte pas l'autre.
    #[test]
    fn the_bytes_mode_does_not_need_an_assembler() {
        let module = DetourModule {
            mode: ModeAjout::Octets,
            code: "41 bb 0c 30 40 00".to_owned(),
            ..Default::default()
        };
        assert_eq!(
            module.octets_de_l_ajout(0x1000),
            Ok(vec![0x41, 0xbb, 0x0c, 0x30, 0x40, 0x00])
        );
    }

    /// Le plus petit ELF où une adresse se retrouve dans le fichier.
    ///
    /// Pas de table de sections : `imports::file_offset` se replie alors
    /// sur les segments, ce qui est le chemin des binaires dépouillés —
    /// donc des crackmes. Un seul `PT_LOAD`, qui projette tout le
    /// fichier à 0x400000.
    fn elf_d_un_segment(taille: usize) -> Vec<u8> {
        let mut f = vec![0u8; taille.max(0x80)];
        f[..8].copy_from_slice(b"\x7fELF\x02\x01\x01\x00");
        f[0x10..0x12].copy_from_slice(&2u16.to_le_bytes()); // ET_EXEC
        f[0x12..0x14].copy_from_slice(&0x3eu16.to_le_bytes()); // x86-64
        f[0x20..0x28].copy_from_slice(&0x40u64.to_le_bytes()); // e_phoff
        f[0x36..0x38].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
        f[0x38..0x3a].copy_from_slice(&1u16.to_le_bytes()); // e_phnum
        // Le segment : PT_LOAD, tout le fichier, projeté à 0x400000.
        let taille = f.len() as u64;
        f[0x40..0x44].copy_from_slice(&1u32.to_le_bytes()); // p_type
        f[0x44..0x48].copy_from_slice(&5u32.to_le_bytes()); // R+X
        f[0x48..0x50].copy_from_slice(&0u64.to_le_bytes()); // p_offset
        f[0x50..0x58].copy_from_slice(&0x400000u64.to_le_bytes()); // p_vaddr
        f[0x60..0x68].copy_from_slice(&taille.to_le_bytes()); // p_filesz
        f[0x68..0x70].copy_from_slice(&taille.to_le_bytes()); // p_memsz
        f
    }

    /// LE PANNEAU TIENT COMPTE DES CORRECTIFS DÉJÀ POSÉS.
    ///
    /// C'est un contrôle de BRANCHEMENT, et il vaut par ce qu'il
    /// attrape : `plan_detour` sait refuser d'écrire sur un correctif,
    /// mais seulement si on lui dit lesquels sont posés. Un `&[]` oublié
    /// ici — c'était la valeur de passage avant le 27 octobre 2026 —
    /// éteint la garde en entier, sans qu'aucun contrôle de `patch.rs`
    /// ne s'en aperçoive : ils lui passent, eux, ce qu'ils veulent
    /// éprouver.
    ///
    /// Il commence donc par montrer que le plan SORT quand rien n'est
    /// posé. Sans cette moitié, le contrôle passerait au vert pour la
    /// mauvaise raison — une adresse introuvable dans le fichier, un
    /// trou mal décrit, un champ mal lu laissent aussi `plan` sans
    /// `Ok`, et l'on croirait tenir la garde.
    #[test]
    fn the_panel_refuses_a_site_that_a_patch_already_holds() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let binaire = elf_d_un_segment(0x400);

        let neuf = || DetourModule {
            // 0x400100 : du zéro dans le fichier, donc des instructions
            // décodables (`add [rax], al`), et de quoi en couvrir cinq.
            site: "0x400100".to_owned(),
            mode: ModeAjout::Octets,
            code: "90".to_owned(),
            trous: vec![Slack {
                vaddr: 0x400200,
                offset: 0x200,
                len: 0x100,
                filler: 0x00,
                executable: true,
                section: None,
            }],
            ..Default::default()
        };

        // D'ABORD SANS RIEN DE POSÉ : le plan sort. C'est la moitié qui
        // donne son sens à l'autre.
        let mut module = neuf();
        module.calculer(&contexte(&binaire, &cpu));
        let plan = module.plan.as_ref().expect("un plan a été calculé");
        assert!(
            plan.is_ok(),
            "sur un fichier neuf, ce site se détourne : {:?}",
            plan.as_ref().err().map(|e| e.message(true))
        );
        let posé = plan.as_ref().unwrap().aller.clone();

        // PUIS AVEC CE MÊME ALLER DÉJÀ POSÉ : refusé, et nommément.
        let posés = [posé];
        let mut avec = contexte(&binaire, &cpu);
        avec.patches = &posés;
        let mut module = neuf();
        module.calculer(&avec);
        match module.plan.as_ref().expect("un refus est un plan aussi") {
            Err(DetourRefusé::SurUnCorrectif { à, correctif, .. }) => {
                assert_eq!(*à, 0x400100, "le site visé");
                assert_eq!(*correctif, 0x400100, "et le correctif qu'il heurte");
            }
            autre => panic!("attendu un refus pour correctif posé, reçu {autre:?}"),
        }
    }

    /// Un plan calculé devient caduc quand le fichier change.
    ///
    /// C'est le défaut qu'on ne verrait pas : le plan reste affiché, le
    /// bouton « Poser » reste cliquable, et il poserait des octets
    /// calculés sur une image qui n'est plus celle du fichier. Tout
    /// rechargement le jette.
    #[test]
    fn a_plan_is_dropped_when_the_file_changes() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let mut module = DetourModule {
            erreur_asm: Some("quelque chose".to_owned()),
            plan: Some(Err(DetourRefusé::PlaceNonExécutable)),
            ..Default::default()
        };
        module.on_binary_loaded(&contexte(b"", &cpu));
        assert!(
            module.plan.is_none(),
            "le plan ne survit pas au rechargement"
        );
        assert!(module.erreur_asm.is_none());
    }

    /// Ce qu'une grille du plan PEINT vraiment, à une largeur donnée.
    ///
    /// Relire le code ne dit pas si trois colonnes monospace tiennent
    /// dans une colonne d'écran partagé : egui ne borne rien de
    /// lui-même, et une grille déborde sans se plaindre. On relève donc
    /// les formes de texte et on les mesure, avec les précautions
    /// d'usage — le rectangle et non la position, la ligne et non la
    /// galley, et l'encre intersectée avec le découpage, seule façon de
    /// voir qu'un texte est coupé.
    fn largeur_perdue(octets: &[u8], largeur: f32) -> Vec<(String, f32)> {
        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, 1200.0),
            )),
            ..Default::default()
        };
        let cpu = Cpu::new(Memory::new(0x400000, 0x100));
        let ctx = contexte(b"", &cpu);
        let mut coupées = Vec::new();
        // Deux images : la première pose les identifiants et les
        // largeurs qu'egui garde d'une image sur l'autre.
        for _ in 0..2 {
            coupées.clear();
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| {
                // PAR LE PEINTRE COMMUN, depuis P7 : c'est lui qui
                // borne désormais la colonne la plus large d'un
                // tableau, et c'est lui qu'il faut mesurer.
                crate::vue_peinte::peindre(
                    ui,
                    &ctx,
                    &Vue {
                        blocs: blocs_d_instructions(&ctx, octets, 0x401000, "essai"),
                    },
                );
            });
            for clipped in &sortie.shapes {
                let egui::epaint::Shape::Text(t) = &clipped.shape else {
                    continue;
                };
                for ligne in &t.galley.rows {
                    if ligne.glyphs.is_empty() {
                        continue;
                    }
                    let posée = ligne
                        .rect_without_leading_space()
                        .translate(t.pos.to_vec2());
                    let encre = posée.intersect(clipped.clip_rect);
                    if encre.width() <= 0.0 {
                        continue;
                    }
                    let perdue = posée.width() - encre.width();
                    if perdue > 0.5 {
                        coupées.push((t.galley.text().to_owned(), perdue));
                    }
                }
            }
            sortie.textures_delta.clear();
        }
        coupées
    }

    /// Une instruction longue ne déborde pas la grille du plan, même
    /// dans une colonne étroite.
    ///
    /// `lock cmpxchg16b oword [rax+rbx*8+0x12345678]` fait dix octets et
    /// quarante-trois caractères : avec l'adresse et les octets en
    /// hexadécimal, la ligne demande plus de huit cents pixels en
    /// monospace. Dans une colonne de quatre cent vingt, il faut bien
    /// que quelque chose cède — et ce qui cède doit être un choix, non
    /// un texte coupé par un bord.
    #[test]
    fn a_long_instruction_does_not_overflow_the_plan_grid() {
        let longue = [0xF0, 0x48, 0x0F, 0xC7, 0x8C, 0xD8, 0x78, 0x56, 0x34, 0x12];
        let coupées = largeur_perdue(&longue, 420.0);
        assert!(
            coupées.is_empty(),
            "la grille du plan coupe du texte à 420 px : {coupées:?}"
        );
    }

    /// Et à large, rien ne bouge non plus — le contrôle ne doit pas
    /// passer par accident parce que la largeur suffisait.
    #[test]
    fn the_plan_grid_is_also_clean_when_wide() {
        let longue = [0xF0, 0x48, 0x0F, 0xC7, 0x8C, 0xD8, 0x78, 0x56, 0x34, 0x12];
        assert!(largeur_perdue(&longue, 1200.0).is_empty());
    }

    /// AUCUN REFUS NE SE FAIT COUPER PAR LE BORD DU PANNEAU.
    ///
    /// Les messages de `DetourRefusé` sont de la prose, et de la prose
    /// longue : ils nomment l'instruction, l'adresse, la taille du
    /// champ, et disent quoi faire. Un refus coupé est pire qu'un refus
    /// muet — on lit la moitié d'une explication et l'on croit avoir
    /// compris. Mesuré sur TOUTES les variantes, pour qu'un refus ajouté
    /// un jour ne passe pas au travers, et à la largeur la plus étroite
    /// que le panneau latéral prenne.
    #[test]
    fn no_refusal_is_cut_by_the_edge_of_the_panel() {
        let refus = [
            DetourRefusé::PlaceNonExécutable,
            DetourRefusé::PlaceTropPetite {
                besoin: 42,
                offert: 8,
            },
            DetourRefusé::HorsFichier,
            DetourRefusé::SiteIndécodable { à: 0x401136 },
            DetourRefusé::TropLoin,
            DetourRefusé::CodeDéplacéRelatif {
                à: 0x401136,
                mnémonique: "lea".to_owned(),
            },
            DetourRefusé::PortéeTropCourte {
                à: 0x401136,
                mnémonique: "jmp".to_owned(),
                champ: 1,
            },
            DetourRefusé::ChampIntrouvable {
                à: 0x401136,
                mnémonique: "call".to_owned(),
            },
            DetourRefusé::SurUnCorrectif {
                à: 0x401136,
                offset: 0x1136,
                correctif: 0x401131,
            },
        ];

        let egui_ctx = egui::Context::default();
        for largeur in [220.0f32, 420.0, 900.0] {
            let entrée = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(largeur, 2400.0),
                )),
                ..Default::default()
            };
            for cas in &refus {
                let cpu = Cpu::new(Memory::new(0x400000, 0x100));
                let ctx = contexte(b"", &cpu);
                let mut module = DetourModule {
                    plan: Some(Err(cas.clone())),
                    // UN TROU EST NÉCESSAIRE, et l'oubli a coûté un
                    // essai : sans place libre, `ui` écrit « aucune
                    // place libre exécutable » et REND LA MAIN. Le
                    // refus n'était jamais peint, et le contrôle passait
                    // au vert sans rien mesurer — y compris avec le
                    // repli du texte retiré exprès.
                    trous: vec![Slack {
                        vaddr: 0x401100,
                        offset: 0x1100,
                        len: 0x100,
                        filler: 0x00,
                        executable: true,
                        section: Some(".text".to_owned()),
                    }],
                    ..Default::default()
                };
                let attendu = cas.message(true);
                let mut coupées = Vec::new();
                let mut peint = false;
                // Deux images : la première pose ce qu'egui garde.
                for _ in 0..2 {
                    coupées.clear();
                    peint = false;
                    let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| {
                        module.ui(ui, &ctx);
                    });
                    for clipped in &sortie.shapes {
                        let egui::epaint::Shape::Text(t) = &clipped.shape else {
                            continue;
                        };
                        if t.galley.text() == attendu {
                            peint = true;
                        }
                        for ligne in &t.galley.rows {
                            if ligne.glyphs.is_empty() {
                                continue;
                            }
                            let posée = ligne
                                .rect_without_leading_space()
                                .translate(t.pos.to_vec2());
                            let encre = posée.intersect(clipped.clip_rect);
                            if encre.width() <= 0.0 {
                                continue;
                            }
                            if posée.width() - encre.width() > 0.5 {
                                coupées.push((
                                    t.galley.text().to_owned(),
                                    posée.width() - encre.width(),
                                ));
                            }
                        }
                    }
                    sortie.textures_delta.clear();
                }
                assert!(
                    peint,
                    "à {largeur:.0} px, le refus {cas:?} n'est pas peint du tout — \
                     un contrôle qui ne voit pas son sujet ne mesure rien"
                );
                assert!(
                    coupées.is_empty(),
                    "à {largeur:.0} px, le refus {cas:?} perd du texte au bord : {coupées:?}"
                );
            }
        }
    }

    /// Le titre du panneau existe dans les deux langues, et le nom de
    /// colonne est plus court.
    #[test]
    fn the_panel_names_itself_in_both_languages() {
        let module = DetourModule::default();
        assert!(module.title("fr").contains("Détourner"));
        assert!(module.title("en").contains("Detour"));
        assert!(
            module.nav_title("fr").len() < module.title("fr").len(),
            "la colonne a besoin du nom court"
        );
        assert_eq!(module.section(), ReSection::Bytes);
    }
}
