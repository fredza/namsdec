//! Les fenêtres qui montrent un binaire : l'analyse, le désassemblage,
//! la chronologie, les registres étendus, les points d'arrêt, le motif
//! cyclique et les correctifs.
//!
//! Toutes ont la même façon : un titre, un contenu qui se défile, une
//! barre « OK » ancrée en bas. Les tenir ensemble est ce qui permet de
//! voir qu'elles se ressemblent — et de remarquer, le jour où l'une
//! diverge, qu'elle diverge.
//!
//! Sorties de  le 29 septembre 2026.

use super::*;

impl NamsdecApp {
    /// Lance un désassembleur extérieur sur le binaire ouvert.
    ///
    /// On lui donne trois choses, et il prend celles que sa fiche
    /// nomme : le fichier entier, les octets exécutables seuls, et
    /// l'adresse où ceux-ci sont chargés. Un outil qui ne lit pas l'ELF
    /// — DZF, ou le premier désassembleur que quiconque écrit — n'a
    /// ainsi plus à désassembler l'en-tête comme du code, ni à adresser
    /// son listing à partir de zéro.
    ///
    /// C'est namsdec qui sait lire l'ELF. Le lui faire réapprendre à
    /// chaque outil serait le contraire d'un format de plugin.
    fn lancer_désassembleur(
        &self,
        plugin: &namsdec_disasm::plugin::Plugin,
        state: &AssembleState,
    ) -> String {
        let fichier = if state.output.file.is_empty() {
            &state.output.bytes
        } else {
            &state.output.file
        };
        // La première section exécutable : celle où vit le programme.
        // S'il y en a plusieurs, c'est celle du point d'entrée qu'on
        // veut, et elle vient en premier dans tous les ELF ordinaires.
        let code = state
            .output
            .sections
            .iter()
            .find(|section| section.executable && section.size > 0)
            .and_then(|section| {
                let début = section.addr.checked_sub(state.output.load_addr)? as usize;
                state.output.bytes.get(début..début + section.size)
            });
        let cible = namsdec_disasm::plugin::Cible {
            fichier,
            code,
            base: code
                .and_then(|_| {
                    state
                        .output
                        .sections
                        .iter()
                        .find(|s| s.executable && s.size > 0)
                        .map(|s| s.addr)
                })
                .unwrap_or(state.output.load_addr),
        };
        plugin
            .run_on(&cible, namsdec_disasm::plugin::TIMEOUT)
            // Le message brut, comme avant : la traduction de
            // `namsdec-theme::PluginError` ne couvre pas celle-ci, et
            // inventer une correspondance dirait autre chose que l'outil.
            .unwrap_or_else(|e| format!("{e}"))
    }

    /// Le panneau de désassemblage : le programme, lu par le moteur
    /// qu'on a choisi.
    ///
    /// Deux moteurs sont livrés — le pseudo-code, qui dit l'instruction
    /// en clair, et la syntaxe NASM, celle qu'on écrit. D'autres
    /// s'ajoutent en posant un fichier de description dans le dossier
    /// des plugins : namsdec appelle alors l'outil et montre ce qu'il
    /// répond.
    pub(crate) fn disassembly_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_disassembly {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let mut open = true;
        let mut fermer_ok = false;
        let mut aller: Option<u32> = None;
        let mut vise: Option<u64> = None;
        let mut revenir_au_début = false;
        // Relevée avant la fenêtre : elle emprunte `self` en écriture.
        let addr_to_line = self
            .assemble_state
            .as_ref()
            .map(|state| state.addr_to_line.clone())
            .unwrap_or_default();

        let fenêtre_disassembly_window = egui::Window::new(s.disassembly_title)
            .id(egui::Id::new("disassembly_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size(window_size(
                self.assemble_state.is_some(),
                egui::vec2(680.0, 560.0),
                egui::vec2(540.0, 280.0),
            ))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "disassembly"));
        let rendue_disassembly_window = self
            .maximizable(fenêtre_disassembly_window, Self::DISASSEMBLY_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.disassembly_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "disassembly") {
                    fermer_ok = true;
                }
                // Le choix du moteur, en tête : c'est lui qui décide de
                // tout ce qu'on lit en dessous.
                ui.horizontal_wrapped(|ui| {
                    ui.label(s.disassembly_engine);
                    ui.selectable_value(
                        &mut self.disassembler,
                        PSEUDO_ID.to_owned(),
                        s.disassembly_pseudo,
                    );
                    ui.selectable_value(
                        &mut self.disassembler,
                        NASM_ID.to_owned(),
                        s.disassembly_nasm,
                    );
                    ui.selectable_value(
                        &mut self.disassembler,
                        SYSTEM_ID.to_owned(),
                        s.disassembly_system,
                    )
                    .on_hover_text(s.disassembly_system_hint);
                    for plugin in &self.disassembler_plugins {
                        ui.selectable_value(
                            &mut self.disassembler,
                            plugin.id.clone(),
                            &plugin.name,
                        )
                        .on_hover_text(format!("{} {}", s.disassembly_external, plugin.command));
                    }
                });
                ui.weak(if self.disassembler == PSEUDO_ID {
                    s.disassembly_pseudo_hint
                } else if self.disassembler == SYSTEM_ID {
                    s.disassembly_system_hint
                } else {
                    s.disassembly_nasm_hint
                });
                ui.separator();

                let Some(state) = &self.assemble_state else {
                    empty_state(
                        ui,
                        theme,
                        s.no_binary_assembled,
                        Some(s.empty_disassembly),
                        None,
                    );
                    return;
                };
                if state.disasm.is_empty() {
                    empty_state(
                        ui,
                        theme,
                        s.no_binary_assembled,
                        Some(s.empty_disassembly),
                        None,
                    );
                    return;
                }

                // Un moteur extérieur : on l'appelle sur demande, et
                // l'on garde sa réponse. Le relancer à chaque frame
                // reviendrait à lancer un programme soixante fois par
                // seconde.
                if let Some(plugin) = self
                    .disassembler_plugins
                    .iter()
                    .find(|plugin| plugin.id == self.disassembler)
                {
                    let déjà = self
                        .plugin_output
                        .as_ref()
                        .filter(|(id, _)| *id == plugin.id)
                        .map(|(_, texte)| texte.clone());
                    match déjà {
                        Some(texte) => {
                            egui::ScrollArea::both()
                                .id_salt("plugin_output")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(texte).monospace())
                                            .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                });
                        }
                        None => {
                            ui.weak(format!("{} {}", s.disassembly_will_run, plugin.command));
                            if framed_button(ui, theme, s.disassembly_run).clicked() {
                                let texte = self.lancer_désassembleur(plugin, state);
                                self.plugin_output = Some((plugin.id.clone(), texte));
                            }
                        }
                    }
                    return;
                }
                let rip = self.emulator.as_ref().map(|e| e.cpu().regs.rip);
                let symbols = &state.output.symbols;
                // La colonne des étiquettes, sur une largeur FIXE.
                //
                // Elle n'en avait pas : une rangée sans étiquette ne
                // réservait rien, et les octets comme le texte de
                // l'instruction reculaient de cinquante-cinq pixels dès
                // qu'une ligne en portait une. Un listing dont les
                // colonnes dansent d'une ligne à l'autre ne se lit plus
                // en colonnes, et c'est tout ce qu'on lui demande.
                //
                // Vingt signes au plus : au-delà, un nom long pousserait
                // le code hors de la fenêtre pour le confort d'une seule
                // ligne. Zéro quand il n'y a aucun symbole, auquel cas la
                // colonne disparaît au lieu de laisser un blanc.
                let colonne_étiquette = symbols
                    .iter()
                    .map(|s| s.name.chars().count() + 1)
                    .max()
                    .unwrap_or(0)
                    .min(20);
                let pseudo = self.disassembler == PSEUDO_ID;
                let syntaxe = if self.disassembler == SYSTEM_ID {
                    namsdec_disasm::Syntax::System
                } else {
                    namsdec_disasm::Syntax::Nasm
                };

                // Ce qu'on cherche : une étiquette, un registre, une
                // adresse. Le listing d'un binaire fait quinze mille
                // lignes, et l'on sait ce qu'on y cherche.
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.disassembly_query)
                            .hint_text(s.find_placeholder)
                            .desired_width(200.0),
                    );
                    if !self.disassembly_query.is_empty() && framed_button(ui, theme, "x").clicked()
                    {
                        self.disassembly_query.clear();
                    }
                });
                let query = self.disassembly_query.to_lowercase();

                // Où aller : les étiquettes du programme, dans l'ordre
                // des adresses. Sur un binaire, c'est par là qu'on entre
                // — et faire défiler quinze mille lignes pour trouver
                // une fonction n'est pas une façon de lire.
                if !symbols.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(s.disassembly_goto_label);
                        egui::ComboBox::from_id_salt("disassembly_symbols")
                            .selected_text(s.disassembly_choose)
                            .show_ui(ui, |ui| {
                                for symbol in symbols {
                                    if ui.selectable_label(false, &symbol.name).clicked() {
                                        vise = Some(symbol.addr);
                                    }
                                }
                            });
                    });
                }

                // Le texte de chaque ligne est calculé ici, pour que la
                // recherche porte sur ce qu'on lit et non sur la forme
                // d'un autre moteur.
                let lignes: Vec<Ligne> = state
                    .disasm
                    .iter()
                    // Demandé d'aller quelque part : la liste commence
                    // là. Un défilement calculé dépend de la hauteur
                    // exacte d'une rangée et de ce que la fenêtre laisse
                    // voir — deux grandeurs qu'on ne connaît qu'après
                    // avoir dessiné, et qui font tomber à côté.
                    .skip_while(|instruction| {
                        self.disassembly_target
                            .is_some_and(|addr| instruction.address < addr)
                    })
                    .map(|instruction| Ligne {
                        address: instruction.address,
                        octets: instruction
                            .bytes
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<Vec<_>>()
                            .join(" "),
                        texte: namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
                            .map(|decoded| {
                                if pseudo {
                                    namsdec_disasm::pseudo::of(&decoded, lang)
                                } else {
                                    namsdec_disasm::format_one_in(&decoded, syntaxe)
                                }
                            })
                            .unwrap_or_else(|| instruction.text.clone()),
                    })
                    .filter(|ligne| {
                        query.is_empty()
                            || ligne.texte.to_lowercase().contains(&query)
                            || format!("{:08x}", ligne.address).contains(&query)
                            || symbols.iter().any(|symbol| {
                                symbol.addr == ligne.address
                                    && symbol.name.to_lowercase().contains(&query)
                            })
                    })
                    .collect();
                if !query.is_empty() {
                    ui.weak(format!("{} / {}", lignes.len(), state.disasm.len()));
                }
                if let Some(addr) = self.disassembly_target {
                    ui.horizontal(|ui| {
                        let nom = symbols
                            .iter()
                            .find(|s| s.addr == addr)
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| format!("{addr:#x}"));
                        ui.colored_label(theme.accent, format!("{} {nom}", s.disassembly_from));
                        if framed_button(ui, theme, s.disassembly_all).clicked() {
                            revenir_au_début = true;
                        }
                    });
                }

                let row_height =
                    ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y;
                // Suivre l'exécution, comme l'onglet Octets — sauf
                // pendant une recherche, où c'est elle qui commande.
                let suivie =
                    (query.is_empty() && self.disassembly_target.is_none() && self.follow_rip)
                        .then(|| {
                            rip.and_then(|rip| lignes.iter().position(|ligne| ligne.address == rip))
                        })
                        .flatten();
                // Bornée à la place que la fenêtre laisse : une zone qui
                // se croit plus haute déborde, et ce qui dépasse est
                // coupé — y compris les lignes vers lesquelles on vient
                // de demander à aller.
                let mut zone = egui::ScrollArea::vertical()
                    .id_salt("disassembly_rows")
                    .max_height(ui.available_height().max(120.0))
                    .auto_shrink([false, false]);
                if let Some(index) = suivie {
                    // Trois lignes de contexte au-dessus, et non un
                    // centrage : la hauteur visible n'est pas connue
                    // avant que la zone existe, et un centrage calculé
                    // sur une hauteur approximative tombe à côté.
                    const CONTEXTE: usize = 3;
                    zone = zone
                        .vertical_scroll_offset(index.saturating_sub(CONTEXTE) as f32 * row_height);
                }
                zone.show_rows(ui, row_height, lignes.len(), |ui, rows| {
                    for ligne in &lignes[rows] {
                        let ici = rip == Some(ligne.address);
                        let rangée = ui
                            .horizontal(|ui| {
                                ui.colored_label(
                                    if ici { theme.accent } else { theme.dim },
                                    egui::RichText::new(format!("{:08x}", ligne.address))
                                        .monospace(),
                                );
                                // L'étiquette sur la rangée de son
                                // instruction, et non sur une ligne à
                                // elle : toutes les rangées gardent la
                                // même hauteur, ce dont la zone a besoin
                                // pour savoir où elle en est.
                                let étiquette = symbols
                                    .iter()
                                    .find(|s| s.addr == ligne.address)
                                    .map(|s| format!("{}:", s.name))
                                    .unwrap_or_default();
                                if colonne_étiquette > 0 {
                                    ui.colored_label(
                                        theme.label,
                                        egui::RichText::new(format!(
                                            "{étiquette:<colonne_étiquette$}"
                                        ))
                                        .monospace(),
                                    );
                                }
                                // Les octets : ce qui distingue un
                                // désassemblage d'une traduction. Sur une
                                // largeur fixe, sinon les colonnes
                                // dansent d'une ligne à l'autre.
                                ui.colored_label(
                                    theme.dim,
                                    egui::RichText::new(format!("{:<23}", ligne.octets))
                                        .monospace(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&ligne.texte).monospace().color(
                                            if ici {
                                                theme.accent
                                            } else {
                                                ui.visuals().text_color()
                                            },
                                        ),
                                    )
                                    .truncate(),
                                );
                            })
                            .response;
                        // Un clic mène à la ligne source : le listing
                        // montre ce que le programme *est*, l'éditeur ce
                        // qu'on a écrit, et l'on passe de l'un à l'autre.
                        if rangée
                            .interact(egui::Sense::click())
                            .on_hover_text(s.disassembly_goto)
                            .clicked()
                        {
                            aller = addr_to_line.get(&ligne.address).copied();
                        }
                    }
                });
            });
        self.place_maximize_button(ctx, Self::DISASSEMBLY_WINDOW, rendue_disassembly_window);
        if let Some(ligne) = aller {
            self.goto_line = Some(ligne);
        }
        // Posée pour la frame suivante, puis oubliée : sans quoi la vue
        // resterait collée là et l'on ne pourrait plus la faire défiler.
        if let Some(addr) = vise {
            self.disassembly_target = Some(addr);
        }
        if revenir_au_début {
            self.disassembly_target = None;
        }
        self.show_disassembly = open && !fermer_ok;
    }

    /// La chronologie : le chemin parcouru, et de quoi y revenir.
    ///
    /// « Comment suis-je arrivé là ? » est la question qu'on pose devant
    /// un état qu'on n'attendait pas, et aucun compteur n'y répond : il
    /// faut l'ordre des pas. Revenir en arrière d'un pas à la fois y
    /// répond mal aussi — on cherche un endroit, pas un pas.
    ///
    /// Chaque ligne est un pas qu'on peut vraiment défaire : cliquer
    /// dessus rembobine le programme jusque-là, mémoire et registres
    /// compris.
    /// Le motif cyclique : « de combien ça déborde ».
    ///
    /// On remplit l'entrée d'une suite dont toute tranche de quatre
    /// octets est unique, on laisse le programme déborder, et la valeur
    /// qu'on retrouve dans RIP dit d'elle-même à quelle distance du
    /// début du tampon se trouvait l'adresse de retour. Le calcul est
    /// mécanique — c'est le `cyclic` de pwndbg, et il n'y a aucune
    /// raison de le faire à la main.
    pub(crate) fn cyclic_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_cyclic {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = true;
        let mut fermer_ok = false;
        // Relevés avant la fenêtre, qui emprunte `self` en écriture.
        let registres: Vec<(&'static str, u64)> = self
            .emulator
            .as_ref()
            .map(|emulator| {
                let regs = &emulator.cpu().regs;
                vec![
                    ("RIP", regs.rip),
                    ("RSP", regs.get64(Reg64::Rsp)),
                    ("RAX", regs.get64(Reg64::Rax)),
                ]
            })
            .unwrap_or_default();
        let mut vers_entrée = None;

        egui::Window::new(s.cyclic_title)
            // Même raison qu'ailleurs : le titre change avec la langue,
            // l'identifiant ne doit pas.
            .id(egui::Id::new("cyclic_window"))
            .open(&mut open)
            .resizable(true)
            .default_size([560.0, 400.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "cyclic"))
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.cyclic_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "cyclic") {
                    fermer_ok = true;
                }
                ui.label(prose(ui, theme, s.cyclic_explain));
                ui.separator();

                // Engendrer.
                ui.horizontal(|ui| {
                    ui.colored_label(theme.dim, s.cyclic_length);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyclic_len)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(60.0),
                    );
                });
                // Une longueur qui ne se lit pas ne fait pas disparaître
                // le motif : elle retombe sur cent.
                let combien = self
                    .cyclic_len
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(100)
                    .clamp(1, 4096);
                let motif =
                    String::from_utf8_lossy(&namsdec_re::cyclic::pattern(combien)).into_owned();
                ui.add(
                    egui::Label::new(egui::RichText::new(&motif).monospace())
                        .wrap_mode(egui::TextWrapMode::Wrap),
                );
                ui.horizontal_wrapped(|ui| {
                    if framed_button(ui, theme, s.cyclic_copy).clicked() {
                        ui.ctx().copy_text(motif.clone());
                        self.status = Status::Note(|s| s.cyclic_copied);
                    }
                    // Le motif sert à être *mangé* par le programme :
                    // l'envoyer dans l'entrée évite le détour par le
                    // presse-papier.
                    if framed_button(ui, self.theme, s.cyclic_to_input)
                        .on_hover_text(s.cyclic_to_input_hint)
                        .clicked()
                    {
                        vers_entrée = Some(motif.clone());
                    }
                });

                ui.separator();
                // Retrouver.
                ui.colored_label(theme.dim, s.cyclic_find_label);
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.cyclic_find)
                            .font(egui::TextStyle::Monospace)
                            // Assez large pour son indication : 180 px
                            // la coupaient au milieu, et une indication
                            // s'affiche justement avant qu'on ait tapé
                            // quoi que ce soit — c'est le moment où
                            // l'on en a besoin. Une infobulle ne
                            // remplacerait pas ce qui doit se lire sans
                            // qu'on cherche.
                            .desired_width(280.0)
                            .hint_text(s.cyclic_find_hint),
                    );
                    // Les registres où l'on retrouve le motif après un
                    // débordement : il n'y a pas à les recopier.
                    for (name, value) in &registres {
                        if ui
                            .small_button(*name)
                            .on_hover_text(format!("{value:#x}"))
                            .clicked()
                        {
                            self.cyclic_find = format!("{value:#x}");
                        }
                    }
                });
                match cyclic_offset(&self.cyclic_find) {
                    Some(offset) => {
                        ui.colored_label(
                            theme.good,
                            format!("{} {offset} ({offset:#x})", s.cyclic_offset),
                        );
                        ui.colored_label(theme.dim, s.cyclic_offset_hint);
                    }
                    None if self.cyclic_find.trim().is_empty() => {}
                    None => {
                        ui.colored_label(theme.warn, s.cyclic_not_found);
                    }
                }
            });

        if let Some(motif) = vers_entrée {
            self.console_input = motif;
            self.console_tab = ConsoleTab::Output;
            self.show_console = true;
            self.status = Status::Note(|s| s.cyclic_sent);
        }
        self.show_cyclic = open && !fermer_ok;
    }

    pub(crate) fn timeline_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_timeline {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = true;
        let mut fermer_ok = false;
        let mut revenir = None;
        let mut aller = None;

        // Relevé avant la fenêtre : elle emprunte `self` en écriture
        // pour poser le retour en arrière.
        // Le rang est celui du chemin entier, même quand on n'en montre
        // qu'une partie : « revenir à ce pas » doit rembobiner du bon
        // nombre de pas, filtre ou pas filtre.
        let pas: Vec<(usize, namsdec_emu::Step)> = self
            .emulator
            .as_ref()
            .map(|emulator| {
                emulator
                    .trace()
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, pas)| {
                        !self.timeline_notable_only || pas.kind != namsdec_emu::StepKind::Plain
                    })
                    .collect()
            })
            .unwrap_or_default();

        let fenêtre_timeline_window = egui::Window::new(s.timeline_title)
            .id(egui::Id::new("timeline_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size(window_size(
                self.emulator.is_some(),
                egui::vec2(520.0, 420.0),
                egui::vec2(540.0, 220.0),
            ))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "timeline"));
        let rendue_timeline_window = self
            .maximizable(fenêtre_timeline_window, Self::TIMELINE_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.timeline_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "timeline") {
                    fermer_ok = true;
                }
                if pas.is_empty() {
                    // La même forme que les neuf autres panneaux : le
                    // constat, puis ce que la fenêtre montrera. Une
                    // phrase seule alignée à gauche disait le premier et
                    // taisait le second.
                    if self.timeline_notable_only {
                        empty_state(ui, self.theme, s.timeline_nothing_notable, None, None);
                    } else {
                        empty_state(
                            ui,
                            self.theme,
                            s.timeline_empty,
                            Some(s.empty_timeline),
                            None,
                        );
                    }
                    return;
                }
                ui.weak(s.timeline_hint);
                // La barre, avant la liste : elle répond à « où j'en
                // suis », question qui vient avant « qu'est-ce qui s'est
                // passé ». Le filtre ne la touche pas — c'est le parcours
                // entier qu'elle montre, sans quoi sa forme mentirait.
                if let Some(index) = timeline_bar(ui, theme, self.timeline_trace()) {
                    let total = self.timeline_trace().len();
                    revenir = Some(timeline::rewind_to(total, index));
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.timeline_notable_only, s.timeline_notable);
                });
                ui.separator();
                let row_height =
                    ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y;
                // Le nombre de pas du chemin, et non celui des lignes
                // montrées : c'est de lui que se compte le recul.
                let total = self
                    .emulator
                    .as_ref()
                    .map(|emulator| emulator.trace().len())
                    .unwrap_or(0);
                // Cinq mille pas : on ne dessine que ce qui est à
                // l'écran, comme pour l'onglet Octets.
                // Collée au bas : le dernier pas est celui qu'on regarde
                // entre deux avancées, et le chercher à chaque fois
                // reviendrait à faire défiler cinq mille lignes.
                egui::ScrollArea::vertical()
                    .id_salt("timeline_rows")
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show_rows(ui, row_height, pas.len(), |ui, rows| {
                        for (index, pas) in &pas[rows] {
                            let addr = &pas.addr;
                            // Le rang par rapport à maintenant : le
                            // dernier pas est « -1 », celui d'avant
                            // « -2 ». C'est ainsi qu'on le pense.
                            let recul = total - index;
                            let ligne = self
                                .assemble_state
                                .as_ref()
                                .and_then(|state| state.addr_to_line.get(addr).copied());
                            let texte = self
                                .assemble_state
                                .as_ref()
                                .and_then(|state| {
                                    state
                                        .disasm
                                        .iter()
                                        .find(|instruction| instruction.address == *addr)
                                })
                                .map(|instruction| instruction.text.clone())
                                .unwrap_or_default();
                            let réponse = ui
                                .horizontal(|ui| {
                                    ui.colored_label(theme.dim, format!("-{recul}"));
                                    // Ce que le pas a fait, en une
                                    // marque : c'est ce qu'on cherche des
                                    // yeux en parcourant un chemin.
                                    let (marque, couleur, quoi) = step_mark(s, pas.kind, theme);
                                    ui.colored_label(couleur, marque).on_hover_text(quoi);
                                    ui.monospace(format!("{addr:#010x}"));
                                    match ligne {
                                        Some(ligne) => {
                                            ui.colored_label(theme.accent, format!("{ligne}"))
                                        }
                                        None => ui.colored_label(theme.dim, " "),
                                    };
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(texte).monospace())
                                            .truncate(),
                                    );
                                })
                                .response;
                            let réponse = réponse
                                .interact(egui::Sense::click())
                                .on_hover_text(s.timeline_go_back);
                            if réponse.clicked() {
                                revenir = Some(recul);
                            }
                            // Le clic droit mène à la ligne dans
                            // l'éditeur, sans toucher à l'exécution :
                            // lire le code d'un pas et rembobiner
                            // jusqu'à lui sont deux gestes différents.
                            if réponse.secondary_clicked() {
                                aller = ligne;
                            }
                        }
                    });
            });
        self.place_maximize_button(ctx, Self::TIMELINE_WINDOW, rendue_timeline_window);

        if let Some(recul) = revenir {
            self.rewind(recul);
        }
        if let Some(ligne) = aller {
            self.goto_line = Some(ligne);
        }
        self.show_timeline = open && !fermer_ok;
    }

    /// Les points d'arrêt, avec leur condition.
    ///
    /// Un point d'arrêt se pose d'un clic dans la gouttière ; sa
    /// condition, elle, s'écrit — et il faut bien un endroit pour cela.
    /// La liste dit aussi lesquels sont posés, ce que la gouttière ne
    /// montre que ligne par ligne.
    pub(crate) fn breakpoints_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_breakpoints {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut open = true;
        let mut fermer_ok = false;
        let mut retirer = None;
        egui::Window::new(s.breakpoints_title)
            .id(egui::Id::new("breakpoints_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size(window_size(
                !self.breakpoint_lines.is_empty(),
                egui::vec2(480.0, 320.0),
                egui::vec2(540.0, 220.0),
            ))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "breakpoints"))
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.breakpoints_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "breakpoints") {
                    fermer_ok = true;
                }
                if self.breakpoint_lines.is_empty() {
                    empty_state(
                        ui,
                        self.theme,
                        s.breakpoints_empty,
                        Some(s.empty_breakpoints),
                        None,
                    );
                    return;
                }
                ui.weak(s.breakpoints_hint);
                ui.separator();
                let lignes: Vec<u32> = self.breakpoint_lines.iter().copied().collect();
                for ligne in lignes {
                    ui.horizontal(|ui| {
                        ui.colored_label(theme.accent, format!("{} {ligne}", s.breakpoints_line));
                        let texte = self.breakpoint_conditions.entry(ligne).or_default();
                        let réponse = ui.add(
                            egui::TextEdit::singleline(texte)
                                .id(egui::Id::new(("bp_cond", ligne)))
                                .hint_text(s.breakpoints_condition_hint)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(180.0),
                        );
                        // Le refus se voit sur le champ : une condition
                        // qu'on ne sait pas lire ne filtre rien, et il
                        // vaut mieux le dire que de laisser croire.
                        if !texte.trim().is_empty() && condition::parse(texte).is_none() {
                            ui.painter().rect_stroke(
                                réponse.rect,
                                ui.visuals().widgets.inactive.corner_radius,
                                egui::Stroke::new(1.0, theme.bad),
                                egui::StrokeKind::Inside,
                            );
                            ui.colored_label(theme.bad, s.breakpoints_condition_bad);
                        }
                        if framed_button(ui, theme, "x")
                            .on_hover_text(s.breakpoints_remove)
                            .clicked()
                        {
                            retirer = Some(ligne);
                        }
                    });
                }
            });
        if let Some(ligne) = retirer {
            self.breakpoint_lines.remove(&ligne);
            self.breakpoint_conditions.remove(&ligne);
            self.sync_breakpoints();
        }
        self.show_breakpoints = open && !fermer_ok;
    }

    /// La fenêtre d'analyse : tous les modules de rétro-ingénierie, avec
    /// la place de s'exprimer.
    ///
    /// Les mêmes modules qu'à l'onglet RE, au même instant et sur le même
    /// binaire — mais l'onglet fait trois cent quatre-vingts pixels de
    /// large et cent cinquante de haut, ce qui suffit pour une liste de
    /// registres et pas pour une carte de segments.
    pub(crate) fn analysis_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_analysis {
            return;
        }
        let s = strings(self.lang);
        let mut open = true;
        let mut fermer_ok = false;
        let fenêtre_analysis_window = egui::Window::new(s.analysis_title)
            // Un identifiant explicite : le titre change avec la langue,
            // et la fenêtre perdrait sa position en passant à l'anglais.
            .id(egui::Id::new("analysis_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // Sept modules à la suite, dont un histogramme et plusieurs
            // tableaux : la fenêtre part large ET haute. 620 px de large
            // coupaient les tables de fonctions au milieu d'une colonne,
            // et l'on relisait un rapport en le faisant défiler dans les
            // deux sens.
            //
            // Proportionnée à l'écran, comme celle des exercices : un
            // chiffre en dur déborde d'un petit écran et se perd sur un
            // grand.
            .default_size(analysis_window_size(ctx))
            .min_size([560.0, 420.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "analysis"));
        // Pas de bouton d'agrandissement ici, et c'est délibéré : cette
        // fenêtre-là sert à DÉSIGNER des endroits du listing, et le
        // premier geste qui suit une adresse cliquée est d'aller la
        // voir. Agrandie, elle recouvrait tout — le clic emmenait bien
        // le listing à l'adresse, derrière un panneau opaque, et l'on
        // concluait que « ça ne marche pas ». Elle s'ouvre déjà aux
        // deux tiers de l'écran et se redimensionne à la main.
        fenêtre_analysis_window.title_bar(false).show(ctx, |ui| {
            if chrome::window_head(ui, self.theme, s, s.analysis_title) {
                fermer_ok = true;
            }
            if ok_bar(ui, self.theme, strings(self.lang), "analysis") {
                fermer_ok = true;
            }
            // Ce qu'on a compris d'un binaire ne vaut que si on peut
            // le transmettre : le rapport part en Markdown, prêt à
            // coller dans un compte rendu ou un support de cours.
            //
            // La rangée est encadrée d'air : collée sous le filet du
            // bandeau, elle passait pour une seconde barre de titre
            // plutôt que pour deux boutons.
            ui.add_space(6.0);
            egui::Frame::NONE
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 0,
                    bottom: 0,
                })
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if framed_button(ui, self.theme, s.analysis_copy).clicked()
                            && let Some(report) = self.analysis_report()
                        {
                            ui.ctx().copy_text(report);
                            self.status = Status::Note(|s| s.analysis_copied);
                        }
                        // Et sur le disque : un compte rendu se
                        // garde, se relit, s'attache à un courriel.
                        // Le presse-papier ne survit pas à la
                        // fenêtre suivante.
                        if framed_button(ui, self.theme, s.analysis_save)
                            .on_hover_text(s.analysis_save_hint)
                            .clicked()
                        {
                            self.spawn_file_dialog(FileDialogKind::ExportReport);
                        }
                        // Le vidage a son propre enregistrement :
                        // ce n'est pas un rapport, c'est le fichier
                        // lui-même, et il sert à être comparé.
                        if self.analysis_module == "dump"
                            && framed_button(ui, self.theme, s.dump_save)
                                .on_hover_text(s.dump_save_hint)
                                .clicked()
                        {
                            self.spawn_file_dialog(FileDialogKind::ExportDump);
                        }
                        ui.separator();
                        ui.weak(s.analysis_hint);
                    });
                });
            ui.add_space(8.0);

            // Une colonne de navigation à gauche, le module à
            // droite. Les sept modules empilés les uns sous les
            // autres n'employaient que la hauteur : on relisait une
            // page de deux mille pixels dans une fenêtre large de
            // mille, en cherchant un titre à l'ascenseur. Ici, on
            // choisit, et le module a toute la largeur.
            let lang = self.lang.code();
            // Les noms de colonne, qui ne sont pas toujours les titres :
            // « Décompilation par RetDec » passe sur deux lignes dans deux
            // cents pixels, et cinq entrées de ce genre repoussent les
            // dernières sous le bord.
            let titres = self.re_registry.nav_entries(lang);
            let theme = self.theme;
            egui::Panel::left(egui::Id::new("analysis_nav"))
                .resizable(true)
                .default_size(200.0)
                .min_size(150.0)
                .max_size(340.0)
                // Un fond léger et une marge : les deux colonnes se
                // lisent alors comme deux zones, et non comme un
                // texte coupé par un trait. Sans la marge de droite,
                // les libellés touchaient la ligne de séparation.
                .frame(
                    egui::Frame::NONE
                        .fill(theme.dim.gamma_multiply(0.07))
                        .inner_margin(egui::Margin {
                            left: 8,
                            right: 10,
                            top: 8,
                            bottom: 8,
                        }),
                )
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("analysis_nav_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // Justifié à gauche : les entrées
                            // prennent toute la largeur — on clique
                            // la ligne, non le mot — et leur texte
                            // commence au même endroit. Centrés,
                            // treize libellés de longueurs
                            // différentes n'ont plus de bord commun,
                            // et l'œil doit chercher chacun.
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::LEFT),
                                |ui| {
                                    // Serré, et c'est la place qui
                                    // le demande : vingt-cinq
                                    // rubriques — les cinq outils
                                    // extérieurs comprises — ne
                                    // tiennent dans la colonne que
                                    // si chaque entrée s'en tient à
                                    // sa ligne. Au-delà, la liste
                                    // défile, et l'on cherche à
                                    // l'ascenseur une rubrique dont
                                    // on ignore l'existence.
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
                                    // « Tout » en tête : c'est la
                                    // forme d'un compte rendu, celle
                                    // qu'on copie.
                                    let tout = self.analysis_module.is_empty();
                                    if ui
                                        .add(egui::Button::selectable(tout, s.analysis_all))
                                        .clicked()
                                    {
                                        self.analysis_module.clear();
                                    }
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);
                                    for (id, titre) in &titres {
                                        let choisi = self.analysis_module == *id;
                                        if ui
                                            .add(egui::Button::selectable(choisi, *titre))
                                            .clicked()
                                        {
                                            self.analysis_module = (*id).to_owned();
                                        }
                                    }
                                },
                            );
                        });
                });

            // Le contenu, avec la même marge : collé au trait, un
            // tableau donne l'impression d'être coupé.
            egui::Frame::NONE
                .inner_margin(egui::Margin {
                    left: 14,
                    right: 6,
                    top: 8,
                    bottom: 4,
                })
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("analysis_content")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // Une largeur de lecture, et non toute
                            // la place : sur un écran large, une
                            // table à deux colonnes posait la
                            // seconde à mille pixels de la première,
                            // et l'on suivait la ligne du doigt. Ce
                            // que fait tout volet de détail.
                            ui.set_max_width(ui.available_width().min(ANALYSIS_CONTENT_MAX));
                            let choisi = self.analysis_module.clone();
                            let seul = (!choisi.is_empty()).then_some(choisi.as_str());
                            self.re_modules_ui(ui, seul);
                        });
                });
        });
        // L'état de la fenêtre AVANT de relever les demandes des
        // panneaux : l'une d'elles — aller voir une adresse — ferme
        // cette fenêtre, et l'écrire après l'aurait rouverte aussitôt.
        self.show_analysis = open && !fermer_ok;
        self.apply_re_requests();
    }

    /// Les registres étendus, dans une fenêtre à eux.
    ///
    /// Seize registres vectoriels de deux cent cinquante-six bits et huit
    /// emplacements x87 ne tiennent pas dans un panneau qui porte déjà
    /// dix-huit registres entiers : dans le panneau, ils poussaient la
    /// rangée d'onglets dehors, et bornés par une zone de défilement ils
    /// se lisaient par tranches de trois lignes. Ici ils tiennent en
    /// entier, et la fenêtre s'agrandit si l'on veut.
    pub(crate) fn vectors_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_vectors {
            return;
        }
        let s = strings(self.lang);
        let mut open = true;
        let mut fermer_ok = false;
        egui::Window::new(s.vectors_title)
            .id(egui::Id::new("vectors_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_size(window_size(
                self.emulator.is_some(),
                // Seize XMM et huit ST : un contenu de taille fixe et
                // connue, qui tenait dans 374 x 381 pixels quand la
                // fenêtre en demandait 520 x 620.
                egui::vec2(440.0, 480.0),
                // Plus courte que les autres fenêtres vides : son
                // explication tient sur une ligne quand celles du
                // désassemblage et de la chronologie en prennent trois.
                egui::vec2(540.0, 220.0),
            ))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "vectors"))
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.vectors_title) {
                    fermer_ok = true;
                }
                if ok_bar(ui, self.theme, strings(self.lang), "vectors") {
                    fermer_ok = true;
                }
                if self.emulator.is_none() {
                    empty_state(
                        ui,
                        self.theme,
                        s.no_binary_assembled,
                        Some(s.empty_vectors),
                        None,
                    );
                    return;
                }
                // Aucune zone de défilement : les vingt-quatre valeurs
                // se posent sur autant de colonnes que la fenêtre en
                // tient, et tiennent donc en entier. C'est toute la
                // raison d'être de cette fenêtre.
                self.xmm_ui(ui);
                ui.add_space(10.0);
                self.x87_ui(ui);
            });
        self.show_vectors = open && !fermer_ok;
    }

    /// La fenêtre des patchs : ce qu'on peut changer, et ce qu'on a
    /// changé.
    ///
    /// On ouvre un crackme, on voit le `je` qu'il faudrait inverser, et
    /// jusqu'ici l'on ne pouvait rien en faire. Réassembler le listing
    /// est refusé — à juste titre, un désassemblage n'est pas un
    /// source. Écrire les octets à leur place, si.
    pub(crate) fn patches_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_patches {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut fermer = false;
        let mut geste: Option<PatchGeste> = None;
        let mut défaire: Option<usize> = None;
        let inversion = self.patch_preview(PatchGeste::Inverser);
        let neutralisation = self.patch_preview(PatchGeste::Neutraliser);
        let forçage = self.patch_preview(PatchGeste::Forcer);

        egui::Window::new(s.patch_title)
            .id(egui::Id::new(PATCHES_WINDOW))
            .resizable(true)
            .collapsible(false)
            .title_bar(false)
            .default_size(window_size(
                !self.patches.is_empty(),
                egui::vec2(560.0, 420.0),
                egui::vec2(560.0, 330.0),
            ))
            .min_size([460.0, 300.0])
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "patches"))
            .show(ctx, |ui| {
                if chrome::window_head(ui, theme, s, s.patch_title) {
                    fermer = true;
                }
                if ok_bar(ui, theme, s, "patches") {
                    fermer = true;
                }
                ui.add_space(6.0);
                ui.weak(s.patch_hint);
                ui.add_space(8.0);

                // Ce que les trois gestes feraient à la ligne du curseur,
                // écrit avant qu'on clique : on ne change pas les octets
                // de quelqu'un à l'aveugle.
                let mut bouton = |ui: &mut egui::Ui,
                                  libellé: &str,
                                  aperçu: &Option<(u64, Vec<u8>, Vec<u8>)>,
                                  quoi: PatchGeste| {
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(aperçu.is_some(), |ui| {
                            if framed_button(ui, theme, libellé).clicked() {
                                geste = Some(quoi);
                            }
                        });
                        match aperçu {
                            Some((addr, avant, après)) => {
                                ui.monospace(format!(
                                    "{addr:#x} : {} -> {}",
                                    namsdec_re::digest::hex(avant),
                                    namsdec_re::digest::hex(après)
                                ));
                            }
                            None => {
                                ui.weak(s.patch_nothing_here);
                            }
                        }
                    });
                };
                bouton(ui, s.patch_invert, &inversion, PatchGeste::Inverser);
                bouton(ui, s.patch_force, &forçage, PatchGeste::Forcer);
                bouton(
                    ui,
                    s.patch_neutralise,
                    &neutralisation,
                    PatchGeste::Neutraliser,
                );

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                if self.patches.is_empty() {
                    empty_state(
                        ui,
                        self.theme,
                        s.patch_none_yet,
                        Some(s.empty_patches),
                        None,
                    );
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} {}",
                            self.patches.len(),
                            s.patch_applied_count
                        ))
                        .strong(),
                    );
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("patch_list")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (rang, patch) in self.patches.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui.small_button(s.patch_undo).clicked() {
                                        défaire = Some(rang);
                                    }
                                    // Sans adresse — un patch venu du
                                    // vidage désigne une position dans
                                    // le fichier, non une instruction —
                                    // on écrit la position, précédée
                                    // d'un plus pour qu'on ne la
                                    // confonde pas avec une adresse.
                                    let où = if patch.addr == 0 {
                                        format!("+{:#x}", patch.offset)
                                    } else {
                                        format!("{:#x}", patch.addr)
                                    };
                                    ui.monospace(format!(
                                        "{où} : {} -> {}",
                                        namsdec_re::digest::hex(&patch.before),
                                        namsdec_re::digest::hex(&patch.after)
                                    ));
                                });
                            }
                        });
                    ui.add_space(6.0);
                    ui.weak(s.patch_export_hint);
                }
            });

        if let Some(quoi) = geste {
            self.apply_patch(quoi);
        }
        if let Some(rang) = défaire {
            self.undo_patch(rang);
        }
        if fermer {
            self.show_patches = false;
        }
    }
}
