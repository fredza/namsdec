//! La fenêtre du déroulement : le graphe des blocs du programme.
//!
//! Un listing se lit de haut en bas ; un programme, non. Cette fenêtre
//! montre les blocs et les sauts qui les relient, avec ce que chacun
//! fait quand namsdec sait le dire.
//!
//! Sortie de `main.rs` le 29 septembre 2026 : le fichier passait les
//! vingt mille lignes, et le graphe en est la partie la plus autonome —
//! il ne parle qu'au `Cfg` et au thème.

use super::*;

impl NamsdecApp {
    /// Le graphe du programme en DOT, tel que la fenêtre le montre —
    /// la fonction seule ou le programme entier, selon la case cochée.
    ///
    /// Sorti de la fenêtre pour que l'enregistrement le lise aussi : un
    /// bouton qui n'écrit pas le même graphe que celui qu'on regarde
    /// serait un piège.
    pub(crate) fn flow_dot(&self) -> Option<String> {
        let state = self.assemble_state.as_ref()?;
        let entier = namsdec_re::cfg::build(&state.disasm);
        if entier.blocks.is_empty() {
            return None;
        }
        let ancre = self
            .emulator
            .as_ref()
            .and_then(|e| entier.block_at(e.cpu().regs.rip))
            .unwrap_or(0);
        let cfg = if self.flow_current_only && entier.blocks.len() > 1 {
            entier.subgraph(&entier.function_around(ancre))
        } else {
            entier
        };
        (!cfg.blocks.is_empty()).then(|| cfg.to_dot())
    }

    pub(crate) fn flow_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_flow {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = self.show_flow;
        let mut fermer_ok = false;
        let agrandie = self.window_maximized(Self::FLOW_WINDOW);
        // Ce que la pastille a demandé : appliqué une fois la fenêtre
        // refermée, car tant qu'elle se dessine, ce qu'elle montre est
        // emprunté à `self`.
        let mut bascule = false;
        let mut fenêtre = egui::Window::new(s.flow_title)
            .id(egui::Id::new("flow_window"))
            .open(&mut open)
            .collapsible(false);
        // Agrandie ou libre : deux réglages qui ne se mélangent pas. Les
        // poser à la suite ne marchait pas — `min_size` et `default_size`
        // écrasaient ce que `fixed_rect` venait de fixer, et la fenêtre
        // gardait sa taille d'origine en changeant seulement de coin.
        let taille_voulue = {
            let écran = ctx.viewport_rect().size();
            window_size(
                self.assemble_state.is_some(),
                egui::vec2(écran.x * 0.92, écran.y * 0.88),
                egui::vec2(540.0, 220.0),
            )
        };
        fenêtre = if agrandie {
            Self::fill_the_screen(fenêtre, ctx)
        } else {
            fenêtre
                // Un graphe de blocs n'est pas une boîte de dialogue : il
                // se lit à la taille de l'écran. Sept cent soixante
                // pixels de large montraient trois boîtes sur mille.
                .resizable(true)
                .default_size(taille_voulue)
                .min_size(window_size(
                    self.assemble_state.is_some(),
                    egui::vec2(640.0, 420.0),
                    // Le plancher suit la taille d'ouverture : laissé à
                    // 640 x 420, il imposait sa hauteur à une fenêtre
                    // qui n'a qu'une phrase à montrer.
                    egui::vec2(420.0, 180.0),
                ))
                .pivot(egui::Align2::CENTER_CENTER)
                // La taille lui est passée : chargée, elle remplit
                // l'écran, et une fenêtre qui le remplit garde le
                // centre — un cran la ferait sortir.
                .default_pos(default_window_pos_sized(ctx, "flow", taille_voulue))
        };
        let rendue = fenêtre.title_bar(false).show(ctx, |ui| {
            if chrome::window_head(ui, self.theme, s, s.flow_title) {
                fermer_ok = true;
            }
            if ok_bar(ui, self.theme, strings(self.lang), "flow") {
                fermer_ok = true;
            }
            self.flow_body_ui(ui);
        });
        self.show_flow = open && !fermer_ok;
        if let Some(rendue) = rendue
            && title_bar_maximize_button(
                ctx,
                Self::FLOW_WINDOW,
                rendue.response.rect,
                theme,
                s,
                agrandie,
            )
        {
            bascule = true;
        }
        if bascule {
            self.toggle_maximized(Self::FLOW_WINDOW);
        }
    }

    /// Le graphe lui-même, sans son cadre : un seul corps pour l'onglet
    /// de la rangée et pour la fenêtre détachée.
    ///
    /// Ce que le corps ne peut pas faire lui-même, il le demande :
    /// enregistrer le DOT ouvre une boîte de fichier, et copier le
    /// pseudo-code relit le programme — deux appels de méthode, donc
    /// deux emprunts de `self`, impossibles tant que le dessin tient le
    /// graphe. Ils sont relevés à la fin, comme ailleurs.
    pub(crate) fn flow_body_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let mut enregistrer_dot = false;
        let mut copier_pseudo = false;
        // Un bloc, et il a sa raison : ce qu'on montre est EMPRUNTÉ au
        // programme assemblé pendant tout le dessin, et les deux
        // demandes ci-dessous appellent des méthodes — c'est-à-dire
        // reprennent `self` en entier. L'emprunt se termine à
        // l'accolade, et elles passent après.
        {
            let Some(state) = &self.assemble_state else {
                empty_state(ui, theme, s.flow_empty, Some(s.empty_flow), None);
                return;
            };
            let entier = namsdec_re::cfg::build(&state.disasm);
            if entier.blocks.is_empty() {
                empty_state(ui, theme, s.flow_empty, Some(s.empty_flow), None);
                return;
            }
            // Le bloc d'où l'on regarde : celui où le programme en
            // est, ou son point d'entrée tant que rien n'a tourné.
            let ancre = self
                .emulator
                .as_ref()
                .and_then(|e| entier.block_at(e.cpu().regs.rip))
                .unwrap_or(0);
            // La fonction seule, ou tout. Un vrai sous-graphe : le
            // dessin, la mise en page et la recherche de route
            // travaillent dessus sans rien savoir de la restriction.
            let restreint = self.flow_current_only && entier.blocks.len() > 1;
            let cfg = if restreint {
                entier.subgraph(&entier.function_around(ancre))
            } else {
                entier.clone()
            };
            if cfg.blocks.is_empty() {
                empty_state(ui, theme, s.flow_empty, Some(s.empty_flow), None);
                return;
            }
            ui.colored_label(theme.dim, s.flow_hint);
            flow_legend_ui(ui, theme, s);

            // Ce qu'on sait dire de la fonction ENTIÈRE, quand c'est
            // une fonction qu'on regarde. Sur le programme entier, la
            // question n'a pas de sens : « lit RDI » ne veut rien dire
            // d'un ensemble de fonctions qui en prennent chacune les
            // leurs.
            //
            // Le portrait ne parle que de ce qui ne se lit pas sur le
            // dessin — les arguments, les appels système, la valeur
            // rendue —, et se tait quand il n'aurait que du banal à
            // dire. Une phrase de plus qui répète le graphe ferait
            // cesser de lire celles qui apprennent quelque chose.
            if restreint
                && let Some(portrait) = namsdec_re::explain::phrase_de_fonction(
                    &namsdec_re::explain::portrait(&cfg),
                    self.lang == Lang::Fr,
                )
            {
                ui.horizontal_wrapped(|ui| {
                    // « • » et non « ≡ » : le signe d'identité n'existe
                    // pas dans les polices qu'egui embarque, et se
                    // peignait en carré vide — une puce qui ressemble
                    // à un défaut d'affichage.
                    ui.colored_label(theme.accent, "\u{2022}");
                    // Une étiquette ordinaire, et non `cliquable` : le
                    // curseur en main annonce un clic, et ce clic ne
                    // ferait rien. Ce qui change le pointeur doit se
                    // cliquer, sans quoi l'on apprend à ne plus croire
                    // le pointeur.
                    ui.label(&portrait).on_hover_text(s.flow_portrait_hint);
                });
            }

            // « Comment arrive-t-on là ? » — la question qu'on pose
            // devant un programme qui refuse un mot de passe : on
            // connaît l'adresse du succès, on cherche la route.
            let mut route = None;
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, s.flow_goal_label);
                ui.add(
                    egui::TextEdit::singleline(&mut self.flow_goal)
                        .hint_text(s.flow_goal_hint)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(120.0),
                );
                if !self.flow_goal.is_empty() && small_framed_button(ui, theme, "x").clicked() {
                    self.flow_goal.clear();
                }
            });
            if let Some(visée) = sidebar::eval_address(&self.flow_goal, None) {
                match namsdec_re::route::to(&cfg, visée) {
                    Some(trouvée) => {
                        flow_route_ui(ui, theme, s, &cfg, &trouvée, self.lang);
                        route = Some(trouvée);
                    }
                    None => {
                        ui.colored_label(theme.warn, s.flow_no_route);
                        // La route se cherche dans ce qu'on montre.
                        // Si elle existe ailleurs dans le programme,
                        // le dire vaut mieux que de laisser croire
                        // qu'il n'y en a aucune — l'adresse visée est
                        // souvent dans une autre fonction, et c'est
                        // même tout l'intérêt de la question.
                        if restreint && namsdec_re::route::to(&entier, visée).is_some() {
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(theme.dim, s.flow_route_elsewhere);
                                if framed_button(ui, theme, s.flow_show_whole).clicked() {
                                    self.flow_current_only = false;
                                }
                            });
                        }
                    }
                }
            }
            let sur_la_route: std::collections::BTreeSet<usize> = route
                .as_ref()
                .map(|r| {
                    r.stages
                        .iter()
                        .map(|é| é.block)
                        .chain(std::iter::once(r.target))
                        .collect()
                })
                .unwrap_or_default();
            ui.separator();

            // Le zoom à la molette, comme sur une carte : un
            // programme un peu long ne tient pas à l'échelle d'une
            // ligne lisible, et l'on veut d'abord voir sa forme.
            let survolé = ui.rect_contains_pointer(ui.max_rect());
            if survolé {
                let molette = ui.input(|i| i.smooth_scroll_delta.y);
                if molette.abs() > 0.5 && ui.input(|i| i.modifiers.ctrl) {
                    let avant = self.flow_zoom;
                    self.flow_zoom = (self.flow_zoom * (1.0 + molette * 0.002))
                        .clamp(FLOW_ZOOM_MIN, FLOW_ZOOM_MAX);
                    // Zoomer sous le pointeur, comme sur une carte :
                    // ce qu'on vise reste sous la souris. Zoomer
                    // depuis le coin faisait fuir le bloc qu'on
                    // regardait dès le deuxième cran.
                    if let Some(souris) = ui.input(|i| i.pointer.interact_pos()) {
                        let facteur = self.flow_zoom / avant;
                        let vue = ui.min_rect().min.to_vec2();
                        let sous_souris = self.flow_offset + (souris.to_vec2() - vue);
                        self.flow_offset = sous_souris * facteur - (souris.to_vec2() - vue);
                        self.flow_offset.x = self.flow_offset.x.max(0.0);
                        self.flow_offset.y = self.flow_offset.y.max(0.0);
                    }
                }
            }

            // ── 2.8 : « Tout voir » à la touche F ────────────────
            //
            // Le bouton existait ; la touche manquait. `f` comme
            // « fit », et seulement quand aucun champ ne retient les
            // frappes — la case d'adresse est juste au-dessus.
            if survolé
                && ui.ctx().memory(|m| m.focused()).is_none()
                && ui.input(|i| i.key_pressed(egui::Key::F))
            {
                self.flow_fit = true;
            }

            // Et les mêmes commandes à portée de clic : la molette
            // avec Ctrl ne s'invente pas, et rien ne l'annonçait.
            ui.horizontal_wrapped(|ui| {
                // Le zoom agit sur un DESSIN. Le pseudo-code est du
                // texte : ces cinq commandes n'y feraient rien, et une
                // commande qui ne fait rien ment. Grisées plutôt
                // qu'effacées — une commande qui disparaît laisse croire
                // qu'elle n'existe pas, une commande grise qui dit
                // pourquoi enseigne l'état du programme.
                let dessin = !self.flow_pseudo;
                ui.add_enabled_ui(dessin, |ui| {
                    if framed_button(ui, theme, "−")
                        .on_hover_text(s.flow_zoom_out)
                        .on_disabled_hover_text(s.flow_no_zoom_here)
                        .clicked()
                    {
                        self.flow_zoom =
                            (self.flow_zoom / 1.25).clamp(FLOW_ZOOM_MIN, FLOW_ZOOM_MAX);
                    }
                    ui.colored_label(theme.dim, format!("{:.0} %", self.flow_zoom * 100.0))
                        .on_disabled_hover_text(s.flow_no_zoom_here);
                    if framed_button(ui, theme, "+")
                        .on_hover_text(s.flow_zoom_in)
                        .on_disabled_hover_text(s.flow_no_zoom_here)
                        .clicked()
                    {
                        self.flow_zoom =
                            (self.flow_zoom * 1.25).clamp(FLOW_ZOOM_MIN, FLOW_ZOOM_MAX);
                    }
                    if framed_button(ui, theme, s.flow_zoom_reset)
                        .on_disabled_hover_text(s.flow_no_zoom_here)
                        .clicked()
                    {
                        self.flow_zoom = 1.0;
                        self.flow_offset = egui::Vec2::ZERO;
                    }
                    if framed_button(ui, theme, s.flow_fit)
                        .on_disabled_hover_text(s.flow_no_zoom_here)
                        .clicked()
                    {
                        self.flow_fit = true;
                    }
                });
                // Le graphe s'emporte : dans une note, un rapport,
                // un cours. DOT plutôt qu'une image — il se lit à
                // l'œil, et `dot -Tsvg` en fait une image quand on en
                // veut une.
                if framed_button(ui, theme, s.flow_copy_dot)
                    .on_hover_text(s.flow_copy_dot_hint)
                    .clicked()
                {
                    ui.ctx().copy_text(cfg.to_dot());
                    self.status = Status::Note(|s| s.flow_dot_copied);
                }
                // Et sur le disque : un graphe se garde. Celui
                // d'avant le correctif, celui d'après, celui qu'on
                // joint au compte rendu — le presse-papier ne
                // survit pas à la copie suivante.
                if framed_button(ui, theme, s.flow_save_dot)
                    .on_hover_text(s.flow_save_dot_hint)
                    .clicked()
                {
                    enregistrer_dot = true;
                }
                // Et le texte s'emporte aussi, quand c'est lui qu'on
                // regarde. Les deux boutons d'à côté nomment le graphe
                // dans leur libellé : ils ne mentent pas, mais ils ne
                // rendent pas ce qu'on a sous les yeux.
                if self.flow_pseudo {
                    copier_pseudo = framed_button(ui, theme, s.flow_copy_pseudo)
                        .on_hover_text(s.flow_copy_pseudo_hint)
                        .clicked();
                }
                ui.separator();
                ui.checkbox(&mut self.flow_current_only, s.flow_current_only);
                // Deux lectures de la même fonction : sa forme, ou son
                // texte. Le graphe dit par où l'on passe, le pseudo-code
                // dit ce qui se passe — et l'un n'a jamais remplacé
                // l'autre chez personne.
                ui.checkbox(&mut self.flow_pseudo, s.flow_pseudo)
                    .on_hover_text(s.flow_pseudo_hint);
                // Et laquelle. Le nom quand le binaire en porte un,
                // l'adresse sinon : « vérifie_mdp » se retient, pas
                // `0x1d491`. L'adresse reste montrée à côté — c'est
                // elle qu'on tape dans « Comment arrive-t-on à », et
                // c'est elle qui lève l'ambiguïté quand la fermeture
                // par sauts se trompe, faute de symboles.
                if restreint && let Some(entrée) = cfg.blocks.first() {
                    let nom = named_at(&self.notes, &state.output.symbols, entrée.start);
                    let texte = match &nom {
                        Some(nom) => format!("{nom}  {:#x}", entrée.start),
                        None => format!("{:#x}", entrée.start),
                    };
                    ui.colored_label(theme.dim, egui::RichText::new(texte).monospace())
                        .on_hover_text(s.flow_function_hint);
                }
                ui.separator();
                // Combien on en montre, et combien il y en a : sur un
                // binaire ordinaire, l'écart est tout le sujet.
                ui.colored_label(
                    theme.dim,
                    if restreint {
                        format!(
                            "{} / {} {}",
                            cfg.blocks.len(),
                            entier.blocks.len(),
                            s.flow_blocks
                        )
                    } else {
                        format!("{} {}", cfg.blocks.len(), s.flow_blocks)
                    },
                );
                if dessin {
                    ui.separator();
                    ui.weak(s.flow_pan_hint);
                }
            });
            ui.add_space(4.0);

            // « Tout voir » : le zoom qui fait entrer le graphe
            // entier dans la place qu'on a. Il se calcule ici, où
            // cette place est enfin connue — la demande, elle, a été
            // faite au clic.
            let vue = ui.available_size();
            if self.flow_fit {
                self.flow_fit = false;
                let naturel = flow_extent(ui, &cfg, s, 1.0);
                let facteur = (vue.x / naturel.x.max(1.0))
                    .min(vue.y / naturel.y.max(1.0))
                    .clamp(FLOW_ZOOM_MIN, FLOW_ZOOM_MAX);
                self.flow_zoom = facteur;
                self.flow_offset = egui::Vec2::ZERO;
            }
            let zoom = self.flow_zoom;
            // Où le programme en est : le bloc qui contient RIP est
            // mis en avant, comme la ligne courante dans l'éditeur.
            let ici = self
                .emulator
                .as_ref()
                .and_then(|e| cfg.block_at(e.cpu().regs.rip));
            let lignes = &state.addr_to_line;
            // Ce qu'on sait dire de chaque bloc. Calculé ici parce
            // que seul l'appelant connaît ce que l'exécution a
            // compté : le dessin, lui, ne sait rien de l'émulateur.
            let passages: BTreeMap<u64, u64> = self
                .emulator
                .as_ref()
                .map(|e| e.visits().iter().map(|(a, c)| (*a, *c)).collect())
                .unwrap_or_default();
            let phrases: Vec<Option<String>> = cfg
                .blocks
                .iter()
                .enumerate()
                .map(|(index, bloc)| {
                    // Une boucle : une arête qui part d'ici vers
                    // soi-même ou vers plus haut. C'est la seule
                    // définition qu'on puisse tenir sans calculer
                    // les dominateurs, et elle ne se trompe pas dans
                    // le sens qui compte — elle ne dit « boucle »
                    // que s'il y a vraiment un retour en arrière.
                    let boucle = cfg.edges.iter().any(|arête| {
                        arête.from == index && cfg.blocks[arête.to].start <= bloc.start
                    });
                    namsdec_re::explain::phrase(
                        &namsdec_re::explain::Bloc {
                            instructions: &bloc.instructions,
                            exit: bloc.exit,
                            boucle,
                            passages: passages.get(&bloc.start).copied(),
                        },
                        self.lang == Lang::Fr,
                    )
                })
                .collect();
            if self.flow_pseudo {
                let ligne_courante = self.emulator.as_ref().map(|e| e.cpu().regs.rip);
                flow_pseudo_ui(ui, theme, s, &cfg, &state.disasm, self.lang, ligne_courante);
                return;
            }
            let sortie = egui::ScrollArea::both()
                .auto_shrink([false, false])
                .scroll_offset(self.flow_offset)
                .show(ui, |ui| {
                    flow_graph_ui(
                        ui,
                        theme,
                        s,
                        &cfg,
                        ici,
                        lignes,
                        zoom,
                        &sur_la_route,
                        &phrases,
                    )
                });
            let (destination, tiré) = sortie.inner;
            // La ligne visée se pose ici même : `goto_line` est un
            // champ, et l'écrire ne reprend pas `self` en entier — le
            // graphe reste emprunté à côté, sans conflit.
            if let Some(ligne) = destination {
                self.goto_line = Some(ligne);
            }
            // Le glisser déplace la vue, comme sur une carte. On
            // repart de ce que la zone défilante dit d'elle-même :
            // les barres et la molette la déplacent aussi, et les
            // trois doivent parler du même point.
            self.flow_offset = sortie.state.offset - tiré;
        }
        if copier_pseudo && let Some(state) = &self.assemble_state {
            let entier = namsdec_re::cfg::build(&state.disasm);
            let ancre = self
                .emulator
                .as_ref()
                .and_then(|e| entier.block_at(e.cpu().regs.rip))
                .unwrap_or(0);
            let cfg = if self.flow_current_only && entier.blocks.len() > 1 {
                entier.subgraph(&entier.function_around(ancre))
            } else {
                entier
            };
            let lignes = namsdec_re::structure::pseudo_code(&cfg, &state.disasm, self.lang.code());
            ui.ctx().copy_text(namsdec_re::structure::en_texte(&lignes));
            self.status = Status::Note(|s| s.flow_pseudo_copied);
        }
        if enregistrer_dot {
            self.spawn_file_dialog(FileDialogKind::ExportDot);
        }
    }
}

/// Les bornes du grossissement du déroulement.
///
/// Au-delà, les boîtes deviennent des traits ; en deçà, on ne lit plus
/// rien. Entre les deux, on voit soit la forme, soit les instructions —
/// et c'est exactement le choix qu'on veut laisser.
const FLOW_ZOOM_MIN: f32 = 0.4;
const FLOW_ZOOM_MAX: f32 = 2.0;

/// Le pseudo-code structuré de la fonction montrée.
///
/// Une colonne de texte en chasse fixe, indentée par la structure. Les
/// mots qui la portent — « si », « sinon », « tant que », « répéter » —
/// sont en couleur d'accent : ils ne viennent d'aucune instruction, ils
/// viennent du GRAPHE, et cette différence est tout ce qu'il y a à
/// comprendre ici.
///
/// La ligne où le processeur se trouve est surlignée, comme dans
/// l'éditeur — sans quoi le pseudo-code serait un texte mort à côté d'un
/// listing vivant.
pub(crate) fn flow_pseudo_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    cfg: &namsdec_re::cfg::Cfg,
    disasm: &[namsdec_disasm::DecodedInstruction],
    lang: Lang,
    rip: Option<u64>,
) {
    let lignes = namsdec_re::structure::pseudo_code(cfg, disasm, lang.code());
    if lignes.is_empty() {
        empty_state(ui, theme, s.flow_pseudo_empty, None, None);
        return;
    }
    // Une fonction sans saut rend un texte qui n'a aucune structure :
    // rien que les instructions, dites en clair. Le dire évite de
    // chercher un « si » qui n'existe pas et de croire que la vue est
    // en panne.
    //
    // L'état vide ci-dessus, lui, ne se déclenche que sur un graphe
    // vide — que la fenêtre a déjà écarté plus haut. Il est donc mort,
    // et c'est cette ligne-ci qui dit ce qu'il croyait dire.
    if lignes.iter().all(|l| !l.est_structure) {
        ui.colored_label(theme.dim, s.flow_pseudo_flat);
        ui.add_space(4.0);
    }
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Un cran de quatre espaces, mesuré et non deviné : la
            // largeur d'un caractère dépend de la police et du zoom.
            let police = egui::TextStyle::Monospace.resolve(ui.style());
            let cran = ui
                .painter()
                .layout_no_wrap("    ".to_owned(), police, egui::Color32::PLACEHOLDER)
                .size()
                .x;
            ui.spacing_mut().item_spacing.y = 2.0;
            for ligne in &lignes {
                ui.horizontal(|ui| {
                    ui.add_space(cran * ligne.profondeur as f32);
                    let ici = rip.is_some() && ligne.adresse == rip;
                    let couleur = if ici {
                        theme.accent
                    } else if ligne.est_étiquette {
                        theme.dim
                    } else if ligne.est_structure {
                        // Un mot de structure : il vient du graphe, non
                        // d'une instruction. Demandé à la ligne et non
                        // déduit d'une adresse absente — un `si` qui a
                        // absorbé sa comparaison porte l'adresse de
                        // celle-ci, et se peignait donc en texte
                        // ordinaire le jour où il l'a eue.
                        theme.warn
                    } else {
                        theme.text
                    };
                    let texte = egui::RichText::new(&ligne.texte).monospace().color(couleur);
                    let texte = if ici { texte.strong() } else { texte };
                    ui.label(texte);
                });
            }
        });
}

/// La couleur d'une flèche, selon ce qu'elle veut dire.
fn flow_edge_colour(theme: &'static Theme, kind: namsdec_re::cfg::Kind) -> egui::Color32 {
    use namsdec_re::cfg::Kind;
    match kind {
        // Le vert et le rouge d'une condition, comme partout ailleurs
        // dans namsdec : ce qui se fait, ce qui ne se fait pas.
        Kind::Taken => theme.good,
        Kind::NotTaken => theme.bad,
        Kind::Call => theme.accent,
        Kind::Next => theme.dim,
    }
}

/// La légende des flèches : quatre traits et leur sens.
fn flow_legend_ui(ui: &mut egui::Ui, theme: &'static Theme, s: &Strings) {
    use namsdec_re::cfg::Kind;
    ui.horizontal_wrapped(|ui| {
        for (kind, texte) in [
            (Kind::Taken, s.flow_legend_taken),
            (Kind::NotTaken, s.flow_legend_not_taken),
            (Kind::Call, s.flow_legend_call),
        ] {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 10.0), egui::Sense::hover());
            ui.painter().line_segment(
                [
                    egui::pos2(rect.left(), rect.center().y),
                    egui::pos2(rect.right(), rect.center().y),
                ],
                egui::Stroke::new(2.0, flow_edge_colour(theme, kind)),
            );
            ui.colored_label(theme.dim, texte);
            ui.add_space(6.0);
        }
        ui.colored_label(theme.dim, format!("? {}", s.flow_legend_indirect));
    });
}

/// La largeur qu'il faut aux boîtes pour que leur texte y tienne.
///
/// Une largeur commune à tout le graphe, et non une par boîte : les
/// colonnes restent alignées, ce qui est la moitié de ce qu'un tel dessin
/// sert à montrer. Bornée des deux côtés — en deçà on ne lit plus rien,
/// au-delà une seule instruction bavarde étalerait le graphe entier.
pub(crate) fn flow_box_width(
    ui: &egui::Ui,
    cfg: &namsdec_re::cfg::Cfg,
    s: &Strings,
    police: &egui::FontId,
    zoom: f32,
) -> f32 {
    const LIGNES_MAX: usize = 8;
    let plus_longue = cfg
        .blocks
        .iter()
        .flat_map(|bloc| bloc.instructions.iter().take(LIGNES_MAX))
        .map(|texte| texte.chars().count())
        .max()
        .unwrap_or(24)
        // L'en-tête doit tenir lui aussi — et il porte, sur le bloc où
        // le programme en est, « le programme est ici » à sa droite. Les
        // deux étaient peints sans se demander la place : sur une boîte
        // étroite, le libellé venait recouvrir l'adresse.
        .max(ENTÊTE_COLONNES + s.flow_here.chars().count() + 3);
    let glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(police, ' ')).max(1.0);
    (plus_longue as f32 * glyphe + 12.0 * zoom).clamp(220.0 * zoom, 560.0 * zoom)
}

/// La place que prendrait le graphe entier à ce zoom.
///
/// Sert au bouton « Tout voir », qui en déduit le facteur : on mesure
/// avant de dessiner plutôt que de dessiner pour voir si ça dépasse.
fn flow_extent(ui: &egui::Ui, cfg: &namsdec_re::cfg::Cfg, s: &Strings, zoom: f32) -> egui::Vec2 {
    const LIGNES_MAX: usize = 8;
    let niveaux = cfg.layers();
    let profondeur = niveaux.iter().max().copied().unwrap_or(0) + 1;
    let mut par_rangée: Vec<Vec<usize>> = vec![Vec::new(); profondeur];
    for (index, niveau) in niveaux.iter().enumerate() {
        par_rangée[*niveau].push(index);
    }
    let police = egui::FontId::monospace(11.0 * zoom);
    let hauteur_ligne = ui.ctx().fonts_mut(|f| f.row_height(&police));
    let large = flow_box_width(ui, cfg, s, &police, zoom);
    let marge = 18.0 * zoom;

    let mut largeur = 0.0f32;
    let mut hauteur = 0.0f32;
    for rangée in &par_rangée {
        let mut x = 0.0f32;
        let mut haut = 0.0f32;
        for index in rangée {
            let lignes = cfg.blocks[*index].instructions.len().min(LIGNES_MAX) + 1;
            haut = haut.max(lignes as f32 * hauteur_ligne + 10.0 * zoom);
            x += large + marge;
        }
        largeur = largeur.max(x);
        hauteur += haut + marge * 2.0;
    }
    egui::vec2(largeur.max(100.0), hauteur.max(100.0))
}

/// Le texte, coupé à la largeur de la boîte, avec de quoi le dire.
///
/// Couper vaut mieux que laisser déborder : une instruction trop longue
/// se posait sur la boîte voisine, et les deux devenaient illisibles.
pub(crate) fn flow_ellipsis(texte: &str, colonnes: usize) -> String {
    if texte.chars().count() <= colonnes {
        return texte.to_owned();
    }
    let gardé: String = texte.chars().take(colonnes.saturating_sub(1)).collect();
    format!("{gardé}…")
}

/// Le dessin du graphe.
///
/// Rend la ligne où aller si l'on a cliqué un bloc, et de combien la
/// main a tiré le graphe : c'est la fenêtre qui tient la position de la
/// vue, puisque les barres de défilement et les boutons la déplacent
/// aussi.
///
/// La mise en page tient en deux règles : un bloc se range au niveau de
/// son plus court chemin depuis l'entrée, et les blocs d'un même niveau
/// s'alignent sur une rangée. Cela ne donne pas le plus joli graphe du
/// monde — les vrais outils y mettent des heures de calcul —, mais cela
/// donne le bon : l'entrée en haut, le sens de lecture vers le bas, et
/// les boucles qui remontent, visiblement.
#[allow(clippy::too_many_arguments)]
fn flow_graph_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    cfg: &namsdec_re::cfg::Cfg,
    ici: Option<usize>,
    lignes: &BTreeMap<u64, u32>,
    zoom: f32,
    route: &std::collections::BTreeSet<usize>,
    // Ce qu'on sait dire de chaque bloc, ou rien. Calculé par
    // l'appelant, qui seul connaît ce que l'exécution a compté.
    phrases: &[Option<String>],
) -> (Option<u32>, egui::Vec2) {
    use namsdec_re::cfg::Exit;

    let niveaux = cfg.layers();
    let profondeur = niveaux.iter().max().copied().unwrap_or(0) + 1;
    // Combien de blocs par rangée, et le rang de chacun dans la sienne.
    let mut par_rangée: Vec<Vec<usize>> = vec![Vec::new(); profondeur];
    for (index, niveau) in niveaux.iter().enumerate() {
        par_rangée[*niveau].push(index);
    }

    let police = egui::FontId::monospace(11.0 * zoom);
    let hauteur_ligne = ui.ctx().fonts_mut(|f| f.row_height(&police));
    // La largeur d'une boîte suit ce qu'elle contient. Fixée à deux cent
    // soixante pixels, elle laissait « jne near 000000000001D903h » sortir
    // de son cadre et se poser sur la boîte voisine — sur un vrai binaire,
    // le dessin devenait illisible avant même d'être grand.
    let large = flow_box_width(ui, cfg, s, &police, zoom);
    let marge = 18.0 * zoom;
    let glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(&police, ' ')).max(1.0);
    // Ce qui tient dans une boîte, marges déduites.
    let colonnes = (((large - 12.0 * zoom) / glyphe).floor() as usize).max(8);

    // La hauteur d'un bloc dépend de ce qu'il contient : on borne à
    // huit lignes, au-delà le dessin ne montrerait plus la forme.
    const LIGNES_MAX: usize = 8;
    let hauteur_de = |index: usize| {
        let combien = cfg.blocks[index].instructions.len();
        // L'en-tête, les instructions montrées, et la ligne « … n » qui
        // dit ce qu'on ne montre pas. Cette dernière manquait au compte :
        // elle se peignait sous le cadre, à côté du bloc suivant.
        let dit = phrases.get(index).and_then(|p| p.as_ref()).is_some();
        let lignes =
            1 + combien.min(LIGNES_MAX) + usize::from(combien > LIGNES_MAX) + usize::from(dit);
        lignes as f32 * hauteur_ligne + 10.0 * zoom
    };

    // Poser les boîtes, rangée par rangée.
    let mut rects: Vec<egui::Rect> = vec![egui::Rect::NOTHING; cfg.blocks.len()];
    let mut y = 0.0f32;
    let mut largeur_totale = 0.0f32;
    for rangée in &par_rangée {
        let mut x = 0.0f32;
        let mut haut = 0.0f32;
        for index in rangée {
            let h = hauteur_de(*index);
            rects[*index] = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(large, h));
            x += large + marge;
            haut = haut.max(h);
        }
        largeur_totale = largeur_totale.max(x);
        y += haut + marge * 2.0;
    }

    let (réponse, painter) = ui.allocate_painter(
        egui::vec2(largeur_totale.max(100.0), y.max(100.0)),
        egui::Sense::click_and_drag(),
    );
    // De combien la main a tiré le graphe pendant cette frame.
    let tiré = réponse.drag_delta();
    let origine = réponse.rect.min.to_vec2();
    let décalé = |rect: egui::Rect| rect.translate(origine);

    // Les flèches d'abord : elles passent sous les boîtes.
    for edge in &cfg.edges {
        let (de, vers) = (décalé(rects[edge.from]), décalé(rects[edge.to]));
        let couleur = flow_edge_colour(theme, edge.kind);
        let remonte = rects[edge.to].top() <= rects[edge.from].top();
        let départ = egui::pos2(de.center().x, de.bottom());
        let arrivée = egui::pos2(vers.center().x, vers.top());
        if remonte {
            // Une boucle : on la fait passer par le côté, sinon elle
            // traverse tout ce qui la sépare de sa cible.
            let côté = de.right() + marge * 0.6;
            let retour = egui::pos2(vers.center().x, vers.bottom());
            for segment in [
                [
                    egui::pos2(de.right(), de.center().y),
                    egui::pos2(côté, de.center().y),
                ],
                [egui::pos2(côté, de.center().y), egui::pos2(côté, retour.y)],
                [egui::pos2(côté, retour.y), retour],
            ] {
                painter.line_segment(segment, egui::Stroke::new(1.5 * zoom, couleur));
            }
            flow_arrow_head(&painter, retour, egui::vec2(0.0, -1.0), couleur, zoom);
        } else {
            painter.line_segment([départ, arrivée], egui::Stroke::new(1.5 * zoom, couleur));
            flow_arrow_head(&painter, arrivée, egui::vec2(0.0, 1.0), couleur, zoom);
        }
    }

    // Puis les boîtes.
    let mut aller = None;
    for (index, bloc) in cfg.blocks.iter().enumerate() {
        let rect = décalé(rects[index]);
        let courant = ici == Some(index);
        // Un bloc de la route demandée se voit : c'est le chemin qu'on
        // vient de chercher, et il doit se suivre des yeux.
        let sur_route = route.contains(&index);
        let fond = if courant {
            theme.accent.gamma_multiply(0.22)
        } else if sur_route {
            theme.good.gamma_multiply(0.16)
        } else {
            theme.dim.gamma_multiply(0.10)
        };
        painter.rect_filled(rect, 4.0 * zoom, fond);
        painter.rect_stroke(
            rect,
            4.0 * zoom,
            egui::Stroke::new(
                if courant || sur_route { 2.0 } else { 1.0 },
                if courant {
                    theme.accent
                } else if sur_route {
                    theme.good
                } else {
                    theme.dim
                },
            ),
            egui::StrokeKind::Inside,
        );

        // L'adresse du bloc en tête, puis ses instructions.
        let mut ligne_y = rect.top() + 4.0 * zoom;
        let entête = match bloc.exit {
            Exit::Indirect => format!("{:#010x}  ?", bloc.start),
            _ => format!("{:#010x}", bloc.start),
        };
        painter.text(
            egui::pos2(rect.left() + 6.0 * zoom, ligne_y),
            egui::Align2::LEFT_TOP,
            entête.clone(),
            police.clone(),
            if courant { theme.accent } else { theme.dim },
        );
        ligne_y += hauteur_ligne;
        for (rang, texte) in bloc.instructions.iter().enumerate() {
            if rang == LIGNES_MAX {
                painter.text(
                    egui::pos2(rect.left() + 6.0 * zoom, ligne_y),
                    egui::Align2::LEFT_TOP,
                    format!("… {}", bloc.instructions.len() - LIGNES_MAX),
                    police.clone(),
                    theme.dim,
                );
                break;
            }
            painter.text(
                egui::pos2(rect.left() + 6.0 * zoom, ligne_y),
                egui::Align2::LEFT_TOP,
                flow_ellipsis(texte, colonnes),
                police.clone(),
                theme.text,
            );
            ligne_y += hauteur_ligne;
        }
        // CE QUE LE BLOC FAIT, en une phrase, sous ses instructions.
        //
        // namsdec explique chaque instruction depuis longtemps ; il
        // n'expliquait pas un ENSEMBLE. Une phrase par bloc, quand on
        // sait en dire une — et le module se tait le reste du temps,
        // parce qu'une explication fausse est pire qu'une absence
        // d'explication.
        if let Some(phrase) = phrases.get(index).and_then(|p| p.as_ref()) {
            painter.text(
                egui::pos2(rect.left() + 6.0 * zoom, rect.bottom() - 4.0 * zoom),
                egui::Align2::LEFT_BOTTOM,
                flow_ellipsis(phrase, colonnes),
                police.clone(),
                theme.good,
            );
        }

        // « le programme est ici », à droite de l'en-tête. La largeur
        // des boîtes réserve de quoi les mettre côte à côte : sans cela
        // le libellé venait recouvrir l'adresse dès que la boîte était
        // étroite, et l'on ne lisait plus ni l'un ni l'autre.
        if courant {
            painter.text(
                egui::pos2(rect.right() - 6.0 * zoom, rect.top() + 4.0 * zoom),
                egui::Align2::RIGHT_TOP,
                s.flow_here,
                police.clone(),
                theme.accent,
            );
        }

        // Un clic emmène à la ligne du bloc dans l'éditeur : le dessin
        // sert à s'orienter, pas à remplacer le code.
        if réponse.clicked()
            && let Some(pos) = réponse.interact_pointer_pos()
            && rect.contains(pos)
        {
            aller = lignes.get(&bloc.start).copied();
        }
    }
    réponse.on_hover_text(s.flow_goto);
    (aller, tiré)
}

/// La route vers une adresse : ce qu'il faut satisfaire, dans l'ordre.
///
/// C'est la lecture d'un programme qui refuse quelque chose : on part de
/// l'endroit où il accepte, et l'on remonte les conditions. Chaque ligne
/// nomme l'instruction qui décide et ce qu'elle doit répondre.
fn flow_route_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    cfg: &namsdec_re::cfg::Cfg,
    route: &namsdec_re::route::Route,
    lang: Lang,
) {
    use namsdec_re::route::Condition;
    ui.add_space(2.0);
    ui.colored_label(
        theme.good,
        format!(
            "{} {}",
            route.stages.len(),
            if route.stages.len() > 1 {
                s.flow_route_steps
            } else {
                s.flow_route_step
            }
        ),
    );
    egui::Grid::new("flow_route")
        .num_columns(3)
        .min_col_width(0.0)
        .striped(true)
        .show(ui, |ui| {
            for étape in &route.stages {
                ui.monospace(format!("{:#010x}", étape.addr));
                ui.monospace(&étape.decides);
                ui.colored_label(
                    match étape.condition {
                        Condition::Taken => theme.good,
                        Condition::NotTaken => theme.bad,
                        _ => theme.dim,
                    },
                    étape.condition.explain(lang.code()),
                );
                ui.end_row();
            }
            ui.monospace(format!("{:#010x}", cfg.blocks[route.target].start));
            ui.colored_label(theme.good, s.flow_route_arrival);
            ui.label("");
            ui.end_row();
        });
    if route.others {
        ui.colored_label(theme.dim, s.flow_route_others);
    }
    ui.add_space(2.0);
}

/// La pointe d'une flèche, à `bout`, dans la direction `sens`.
fn flow_arrow_head(
    painter: &egui::Painter,
    bout: egui::Pos2,
    sens: egui::Vec2,
    couleur: egui::Color32,
    zoom: f32,
) {
    let taille = 5.0 * zoom;
    let arrière = bout - sens * taille;
    let côté = egui::vec2(-sens.y, sens.x) * (taille * 0.6);
    painter.add(egui::Shape::convex_polygon(
        vec![bout, arrière + côté, arrière - côté],
        couleur,
        egui::Stroke::NONE,
    ));
}
