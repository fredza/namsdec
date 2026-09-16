//! Le panneau latéral : ce que le programme fait au processeur.
//!
//! Le bandeau fixe (registres, indicateurs, vectoriels, pile x87) et les
//! quatre onglets — la pile, la mémoire, le tas, la rétro-ingénierie.
//! Tout ce qui *montre* un état, par opposition à ce qui le modifie.
//!
//! Les fonctions libres du bas — lecture d'une adresse tapée, vidages
//! hexadécimaux, mise en forme d'un flottant — ne servent qu'ici, et se
//! testent sans écran.

use super::*;

impl NamsdecApp {
    pub(crate) fn sidebar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // RE est masqué par défaut (activable dans Préférences > Modules) :
        // s'il vient d'être caché pendant qu'il était actif, on retombe
        // sur la pile plutôt que de laisser un panneau vide.
        if self.active_tab == SidebarTab::Re && !self.show_re_tab {
            self.active_tab = SidebarTab::Stack;
        }

        // Tout l'état du processeur — registres et flags, registres
        // vectoriels, pile x87 — dans **une seule** zone qui défile,
        // bornée à la moitié du panneau.
        //
        // Trois zones bornées séparément ne suffisaient pas : chacune
        // tenait dans sa moitié, mais leurs hauteurs s'ajoutaient et
        // poussaient la rangée d'onglets hors du panneau, sans aucune
        // barre pour aller la rechercher. Une seule zone rend la chose
        // impossible : ce qui dépasse défile, et les onglets restent là
        // où on va les chercher.
        let visible = ui.available_height();
        egui::ScrollArea::vertical()
            .id_salt("cpu_state")
            .max_height((visible * 0.5).max(120.0))
            .show(ui, |ui| {
                // `show_unindented` : le retrait qu'ajoute un en-tête
                // repliable vaut une vingtaine de pixels, et une valeur
                // de registre fait seize chiffres — avec le retrait, la
                // ligne débordait du panneau.
                //
                // `open(Some(...))` plutôt que `default_open(true)` : le
                // premier impose l'état à chaque frame, le second ne vaut
                // qu'à la toute première ouverture et laisse ensuite egui
                // ressortir ce qu'il a retenu de la session précédente.
                let banner = egui::CollapsingHeader::new(s.cpu_state)
                    .open(Some(self.cpu_state_open))
                    .show_unindented(ui, |ui| self.cpu_state_ui(ui));
                // L'état imposé prive l'en-tête de son propre clic : on
                // le reprend ici, sinon le bandeau ne se replierait plus.
                if banner.header_response.clicked() {
                    self.cpu_state_open = !self.cpu_state_open;
                }
                self.xmm_ui(ui);
                self.x87_ui(ui);
            });

        ui.separator();
        ui.horizontal_wrapped(|ui| {
            for tab in SidebarTab::ALL {
                if tab == SidebarTab::Re && !self.show_re_tab {
                    continue;
                }
                if ui
                    .selectable_label(self.active_tab == tab, tab.label(s))
                    .clicked()
                {
                    self.active_tab = tab;
                }
            }
        });
        ui.separator();
        // Chaque vue pose sa propre zone de défilement : un vidage
        // mémoire défile dans les deux sens (seize octets et leur
        // transcription, c'est plus large que le panneau), mais une
        // commande — le champ Adresse — reste bornée par le panneau.
        // Une zone de défilement horizontale commune les étirait toutes
        // les deux à la largeur du vidage.
        match self.active_tab {
            SidebarTab::Stack => self.stack_ui(ui),
            SidebarTab::Memory => self.memory_ui(ui),
            SidebarTab::Heap => self.heap_ui(ui),
            SidebarTab::Re => self.re_ui(ui),
        }
    }

    /// Le bandeau fixe : les registres, puis les flags en pastilles.
    fn cpu_state_ui(&self, ui: &mut egui::Ui) {
        self.registers_ui(ui);
        ui.add_space(6.0);
        self.flags_ui(ui);
    }

    /// Les seize registres SSE, repliés : un programme sur deux n'en
    /// touche aucun, et ils prennent seize lignes de plus.
    ///
    /// Ce qu'on montre d'eux est leur moitié basse — celle où vit un
    /// flottant scalaire — suivie de sa valeur décimale, parce que
    /// personne ne lit `4014000000000000` comme « 5 ». Le reste est dans
    /// l'infobulle.
    fn xmm_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            return;
        };
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        let changed = changed_registers(self.regs_before.as_ref(), regs);
        let fold =
            egui::CollapsingHeader::new(egui::RichText::new(s.sse_registers).color(theme.dim))
                .id_salt("xmm_registers")
                .open(Some(self.sse_open))
                .show(ui, |ui| {
                    egui::Grid::new("xmm_grid")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            for (name, reg) in XMM_REGISTERS {
                                let value = regs.get_ymm(reg);
                                let color = if changed.contains(&name) {
                                    theme.changed
                                } else {
                                    ui.visuals().text_color()
                                };
                                let low = value[0] as u64;
                                let shown = vector_name(name, value);
                                ui.colored_label(color, egui::RichText::new(shown).monospace());
                                ui.colored_label(
                                    color,
                                    egui::RichText::new(format!("{low:016x}")).monospace(),
                                )
                                .on_hover_text(xmm_tooltip(s, value));
                                // La lecture décimale n'a de sens que
                                // si le registre contient *un* nombre :
                                // lire le bas d'un vecteur de quatre
                                // flottants simples comme un double
                                // donne un nombre qui n'existe nulle
                                // part. Le détail est dans l'infobulle.
                                let scalar = value[1] == 0 && value[0] >> 64 == 0;
                                let reading = if scalar {
                                    short_float(f64::from_bits(low))
                                } else {
                                    String::new()
                                };
                                ui.colored_label(
                                    theme.dim,
                                    egui::RichText::new(reading).monospace(),
                                );
                                ui.end_row();
                            }
                        });
                });
        if fold.header_response.clicked() {
            self.sse_open = !self.sse_open;
        }
    }

    fn registers_ui(&self, ui: &mut egui::Ui) {
        let Some(emulator) = &self.emulator else {
            ui.label(strings(self.lang).no_binary_assembled);
            return;
        };
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        let changed = changed_registers(self.regs_before.as_ref(), regs);
        // Dix-sept lignes d'affilée, c'est haut. Le panneau élargi, elles
        // tiennent sur deux colonnes et le bandeau retrouve la moitié de
        // sa hauteur — celui qui garde le panneau étroit garde une
        // colonne, qui défile.
        let pairs = register_columns(ui.available_width());
        egui::Grid::new("registers_grid")
            // Deux cellules par paire, plus une cellule vide entre les
            // paires : c'est elle qui sépare les colonnes.
            .num_columns(3 * pairs - 1)
            .striped(true)
            .show(ui, |ui| {
                // Entre la valeur d'une paire et le nom de la suivante,
                // l'espacement ordinaire de la grille ne suffit pas :
                // seize chiffres suivis de trois lettres se lisent comme
                // un seul mot (« …003c RBX »). Une cellule vide, large de
                // trois espacements, sépare les deux colonnes.
                let gap = ui.spacing().item_spacing.x * 3.0;
                let mut in_row = 0;
                for (index, (name, value)) in REGISTERS
                    .iter()
                    .map(|&(name, reg)| (name, regs.get64(reg)))
                    .chain(std::iter::once(("RIP", regs.rip)))
                    .enumerate()
                {
                    // RIP n'est pas un registre comme un autre ici :
                    // c'est lui que la gouttière suit, et c'est la ligne
                    // qu'on cherche des yeux après chaque pas. Il change
                    // à chaque instruction, le mettre en avant pour ça
                    // n'apprendrait rien.
                    let color = if name == "RIP" {
                        theme.accent
                    } else if changed.contains(&name) {
                        theme.changed
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(color, egui::RichText::new(name).monospace());
                    ui.colored_label(
                        color,
                        egui::RichText::new(format!("{value:016x}")).monospace(),
                    );
                    in_row += 1;
                    if in_row == pairs {
                        ui.end_row();
                        in_row = 0;
                    } else if index != REGISTERS.len() {
                        // `allocate_space` et non `add_space` : dans une
                        // grille, la seconde avance un curseur qui
                        // n'existe pas (egui le refuse). Celle-ci réserve
                        // une vraie cellule, vide et large de `gap`.
                        ui.allocate_space(egui::vec2(gap, 0.0));
                    }
                }
                // RIP peut rester seul sur sa ligne : elle se ferme quand
                // même, sinon la grille avale la dernière.
                if in_row != 0 {
                    ui.end_row();
                }
            });
    }

    fn flags_ui(&self, ui: &mut egui::Ui) {
        let Some(emulator) = &self.emulator else {
            return;
        };
        let f = emulator.cpu().flags;
        let theme = self.theme;
        ui.horizontal_wrapped(|ui| {
            for (name, bit) in [
                ("CF", RFlags::CF),
                ("PF", RFlags::PF),
                ("AF", RFlags::AF),
                ("ZF", RFlags::ZF),
                ("SF", RFlags::SF),
                ("OF", RFlags::OF),
            ] {
                flag_chip(ui, theme, name, f.contains(bit));
            }
        });
    }

    fn stack_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            ui.label(s.no_binary_assembled);
            return;
        };
        self.call_chain_ui(ui, emulator.as_ref());
        ui.separator();

        let addr = emulator.cpu().regs.get64(Reg64::Rsp).saturating_sub(16);
        let dump = safe_hexdump(&emulator.cpu().mem, addr, 64, s.address_out_of_range);
        hexdump_ui(ui, "stack_dump", &dump);
    }

    /// Le tas : ce que `brk` a donné au programme, et ce qu'il en reste.
    ///
    /// Un débogueur montre la pile parce qu'elle est sous RSP, où il sait
    /// regarder. Le tas, lui, n'a pas de registre : sans le noyau, on ne
    /// sait pas où il commence ni où il s'arrête. L'émulateur, qui tient
    /// la frontière lui-même, peut la montrer — c'est tout l'intérêt.
    fn heap_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let Some(emulator) = &self.emulator else {
            ui.label(s.no_binary_assembled);
            return;
        };
        let heap = emulator.heap();
        let room = heap.limit().saturating_sub(heap.start());
        // Le vidage est calculé tant que l'émulateur est emprunté : les
        // boutons, eux, écrivent dans `self` et viennent après.
        let dump = safe_hexdump(
            &emulator.cpu().mem,
            heap.start(),
            (heap.len() as usize).min(HEAP_DUMP_BYTES),
            s.address_out_of_range,
        );

        let bound = |ui: &mut egui::Ui, label: &str, addr: u64, colour: egui::Color32| {
            ui.colored_label(theme.dim, label);
            ui.colored_label(
                colour,
                egui::RichText::new(format!("{addr:#x}")).monospace(),
            );
            ui.end_row();
        };
        egui::Grid::new("heap_bounds")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                bound(ui, s.heap_start, heap.start(), theme.text);
                // La rupture est la seule des trois qui bouge : elle
                // porte donc la couleur de ce qui vient de changer.
                bound(ui, s.heap_break, heap.current(), theme.changed);
                bound(ui, s.heap_limit, heap.limit(), theme.text);
                ui.colored_label(theme.dim, s.heap_used);
                ui.monospace(format!("{} / {} {}", heap.len(), room, s.unit_bytes));
                ui.end_row();
            });

        // La jauge dit d'un coup d'œil ce que trois adresses hexadécimales
        // demandent de soustraire.
        let filled = if room == 0 {
            0.0
        } else {
            heap.len() as f32 / room as f32
        };
        ui.add(egui::ProgressBar::new(filled).desired_height(6.0));
        ui.add_space(2.0);
        ui.colored_label(theme.dim, egui::RichText::new(s.heap_hint).small());
        ui.separator();

        if heap.is_empty() {
            ui.colored_label(theme.dim, s.heap_empty);
            return;
        }
        if framed_button(ui, theme, s.heap_inspect).clicked() {
            self.mem_addr = heap.start();
            self.mem_addr_input = format!("{:#x}", heap.start());
            self.active_tab = SidebarTab::Memory;
        }
        ui.add_space(4.0);
        hexdump_ui(ui, "heap_dump", &dump);
    }

    /// La pile du coprocesseur : huit cases en anneau, du sommet vers le
    /// bas.
    ///
    /// Montrer `st0` à `st7` plutôt que l'ordre physique, c'est montrer
    /// ce que le programme écrit : `fadd st2` compte à partir du sommet,
    /// et c'est de là que vient la moitié des erreurs quand on découvre
    /// cette machine.
    fn x87_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            return;
        };
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        let changed = changed_registers(self.regs_before.as_ref(), regs);
        let fold = egui::CollapsingHeader::new(egui::RichText::new(s.x87_stack).color(theme.dim))
            .id_salt("x87_stack")
            .open(Some(self.x87_open))
            .show(ui, |ui| {
                egui::Grid::new("x87_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        for (index, name) in X87_SLOTS.iter().enumerate() {
                            let occupied = regs.x87.is_occupied(index);
                            let color = if changed.contains(name) {
                                theme.changed
                            } else if occupied {
                                ui.visuals().text_color()
                            } else {
                                theme.dim
                            };
                            ui.colored_label(color, egui::RichText::new(*name).monospace());
                            let text = if occupied {
                                format!("{}", regs.x87.get(index))
                            } else {
                                s.x87_empty.to_owned()
                            };
                            ui.colored_label(color, egui::RichText::new(text).monospace())
                                .on_hover_text(s.x87_hint);
                            ui.end_row();
                        }
                    });
            });
        if fold.header_response.clicked() {
            self.x87_open = !self.x87_open;
        }
    }

    /// Les zones surveillées, avec leur valeur courante et de quoi les
    /// retirer.
    ///
    /// Là où un point d'arrêt dit « arrête-toi là », celui-ci dit
    /// « arrête-toi quand *ça* change » — sans qu'on ait à deviner quelle
    /// instruction le fera. C'est ce qu'un débogueur ordinaire rend cher
    /// et qu'un interpréteur offre pour rien.
    fn watchpoints_ui(&mut self, ui: &mut egui::Ui, watched: &[(u64, u64)]) {
        if self.watchpoints.is_empty() {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        ui.separator();
        ui.label(egui::RichText::new(s.watchpoints_title).color(theme.dim));

        let mut remove = None;
        for &(addr, value) in watched {
            ui.horizontal(|ui| {
                if ui
                    .small_button("×")
                    .on_hover_text(s.watchpoints_remove)
                    .clicked()
                {
                    remove = Some(addr);
                }
                ui.colored_label(
                    theme.changed,
                    egui::RichText::new(format!("{addr:#x}")).monospace(),
                );
                ui.monospace(format!("= {value:#x}"));
            });
        }
        if let Some(addr) = remove {
            self.watchpoints.retain(|&(a, _)| a != addr);
            self.sync_watchpoints();
        }
    }

    /// La chaîne des appels, du plus récent au plus ancien — l'ordre d'un
    /// débogueur, parce que c'est l'appel en cours qui intéresse d'abord.
    ///
    /// Chaque cadre porte la ligne d'où part l'appel : c'est ce qui
    /// transforme une suite d'adresses en une phrase lisible, « appelé
    /// depuis la ligne 12 ».
    fn call_chain_ui(&self, ui: &mut egui::Ui, emulator: &dyn Backend) {
        let s = strings(self.lang);
        let theme = self.theme;
        ui.label(egui::RichText::new(s.call_chain).color(theme.dim));

        let line_of = |addr: u64| {
            self.assemble_state
                .as_ref()
                .and_then(|state| state.addr_to_line.get(&addr).copied())
        };
        let position = |addr: u64| match line_of(addr) {
            Some(line) => format!("{addr:#x}  ({} {line})", s.call_chain_line),
            None => format!("{addr:#x}"),
        };

        // Le sommet : là où l'exécution se trouve en ce moment.
        let rip = emulator.cpu().regs.rip;
        ui.horizontal(|ui| {
            ui.colored_label(theme.accent, "▸");
            ui.monospace(position(rip));
        });

        let frames = emulator.frames();
        if frames.is_empty() {
            ui.colored_label(theme.dim, s.call_chain_empty);
            return;
        }
        for frame in frames.iter().rev() {
            ui.horizontal(|ui| {
                ui.colored_label(theme.dim, "└");
                ui.monospace(position(frame.call_site));
            });
        }
    }

    fn memory_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            ui.label(s.no_binary_assembled);
            return;
        };
        // Le champ est posé *avant* la zone de défilement, et non dedans :
        // à l'intérieur, il se serait étiré à la largeur du vidage.
        let theme = self.theme;
        let regs = emulator.cpu().regs.clone();
        let typed = eval_address(&self.mem_addr_input, Some(&regs));
        let mut go = false;
        let mut watch = None;
        ui.horizontal(|ui| {
            ui.label(s.memory_address_label);
            let response = ui
                .add(
                    egui::TextEdit::singleline(&mut self.mem_addr_input)
                        .id(memory_address_id())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(address_field_width(ui)),
                )
                .on_hover_text(s.memory_address_hint);
            // Le refus se voit sur le champ lui-même, peint par-dessus :
            // le liseré d'egui change avec le focus, celui-ci ne dépend
            // que de ce qui est écrit.
            if typed.is_none() {
                ui.painter().rect_stroke(
                    response.rect,
                    ui.visuals().widgets.inactive.corner_radius,
                    egui::Stroke::new(1.0, theme.bad),
                    egui::StrokeKind::Inside,
                );
            }
            // Entrée vaut Go : c'est le geste du clavier, et il évite
            // d'aller chercher le bouton à la souris.
            go |= response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.add_enabled_ui(typed.is_some(), |ui| {
                go |= icon_square_button(ui, icon_go)
                    .on_hover_text(s.memory_go_hint)
                    .clicked();
                // Surveiller l'adresse *écrite*, et non celle affichée :
                // on tape « rsi », on clique, et c'est le tampon qu'on
                // regarde — sans avoir à valider d'abord.
                if icon_square_button(ui, icon_watch)
                    .on_hover_text(s.memory_watch_hint)
                    .clicked()
                    && let Some(addr) = typed
                {
                    watch = Some(addr);
                }
            });
        });
        // Rien n'a bougé tant que l'adresse n'est pas validée : le vidage
        // ne saute plus à chaque frappe, et une saisie invalide le laisse
        // là où il est au lieu de le renvoyer au début du programme.
        if go {
            self.mem_addr = validated_address(self.mem_addr, &self.mem_addr_input, Some(&regs));
        }
        if typed.is_none() {
            ui.colored_label(theme.bad, s.memory_address_invalid);
        }

        // Comment lire ces octets : bruts, par mots de huit, ou comme du
        // texte. Le même endroit dit trois choses différentes selon ce
        // qu'on y cherche — un tampon, une pile, une chaîne.
        ui.horizontal_wrapped(|ui| {
            for format in MemoryFormat::ALL {
                if ui
                    .selectable_label(self.memory_format == format, format.label(s))
                    .clicked()
                {
                    self.memory_format = format;
                }
            }
        });

        // Le vidage est calculé *avant* la liste : celle-ci peut retirer
        // une zone, donc emprunter `self` en écriture, et l'émulateur
        // n'est plus disponible à ce moment-là.
        let dump = read_memory(
            &emulator.cpu().mem,
            self.mem_addr,
            self.memory_format,
            s.address_out_of_range,
        );
        let watched: Vec<(u64, u64)> = emulator
            .watchpoints()
            .iter()
            .map(|point| (point.addr, point.value()))
            .collect();
        self.watchpoints_ui(ui, &watched);
        hexdump_ui(ui, "memory_dump", &dump);

        // Poser la zone en dernier : `sync_watchpoints` emprunte
        // l'émulateur en écriture, et il est lu jusqu'ici.
        if let Some(addr) = watch
            && !self.watchpoints.iter().any(|&(a, _)| a == addr)
        {
            // Huit octets : la taille d'un mot, celle qu'on surveille
            // neuf fois sur dix.
            self.watchpoints.push((addr, 8));
            self.sync_watchpoints();
        }
    }

    fn re_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            ui.label(s.no_binary_assembled);
            return;
        };
        let ctx = ReContext {
            cpu: emulator.cpu(),
            disasm: &state.disasm,
            source: &self.source,
            lang: self.lang.code(),
        };
        // Défilement vertical seulement : les modules posent des widgets
        // ordinaires, qui prennent la largeur qu'on leur laisse. Dans une
        // zone qui défile aussi horizontalement, cette largeur n'a plus
        // de fin.
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Les modules écrivent ce qu'ils ont trouvé — une adresse, une
            // chaîne, une liste d'appelants — et rien ne borne ce que ça
            // mesure. Sans cette limite, le panneau adopte la largeur du
            // plus long et pousse l'éditeur hors de la fenêtre.
            ui.set_max_width(ui.available_width());
            for module in self.re_registry.iter_mut() {
                ui.group(|ui| {
                    ui.strong(module.title(ctx.lang));
                    module.ui(ui, &ctx);
                });
            }
        });
    }
}

/// Combien de paires nom/valeur tiennent côte à côte dans `width`.
///
/// Une paire mesure un peu moins de 175 px (trois lettres, seize chiffres
/// hexadécimaux en chasse fixe, l'espacement de la grille) ; on n'en met
/// deux que s'il reste de la marge, plutôt que de déborder pour quelques
/// pixels — un débordement, dans un panneau, ce n'est pas du texte coupé,
/// c'est la largeur du panneau qui change.
pub(crate) fn register_columns(width: f32) -> usize {
    const PAIR: f32 = 175.0;
    /// Le blanc entre les deux paires, plus la marge du panneau.
    const GAP: f32 = 30.0;
    if width >= 2.0 * PAIR + GAP { 2 } else { 1 }
}

/// Une pastille de flag. Allumée, elle prend la couleur « bon » du thème ;
/// éteinte, elle reste un contour discret. Le 0/1 est gardé à côté du
/// nom : c'est la valeur du flag, et c'est elle qu'on apprend à lire.
///
/// Peinte à la main sur une place réservée d'avance, et non montée en
/// `Frame` : dans une rangée qui se replie, egui décide du passage à la
/// ligne d'après la taille annoncée par l'élément. Un `Frame` ne l'annonce
/// qu'une fois son contenu posé — les six pastilles restaient donc sur une
/// seule ligne, qui dépassait du panneau.
/// Les noms des registres dont la valeur a changé depuis la photographie
/// prise avant la dernière avancée.
///
/// Sans photographie — programme tout juste assemblé, jamais exécuté —
/// personne n'a changé : le bandeau reste uni plutôt que de tout
/// souligner.
pub(crate) fn changed_registers(
    before: Option<&namsdec_core::Registers>,
    now: &namsdec_core::Registers,
) -> Vec<&'static str> {
    let Some(before) = before else {
        return Vec::new();
    };
    REGISTERS
        .iter()
        .filter(|&&(_, reg)| before.get64(reg) != now.get64(reg))
        .map(|&(name, _)| name)
        .chain(
            XMM_REGISTERS
                .iter()
                .filter(|&&(_, reg)| before.get_ymm(reg) != now.get_ymm(reg))
                .map(|&(name, _)| name),
        )
        .chain(
            X87_SLOTS
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(i, name)| {
                    // Une case qui se vide ou se remplit a changé autant qu'une
                    // case dont la valeur bouge : c'est tout l'intérêt d'une pile.
                    let moved = before.x87.get(i) != now.x87.get(i)
                        || before.x87.is_occupied(i) != now.x87.is_occupied(i);
                    moved.then_some(name)
                }),
        )
        .collect()
}

/// Un flottant écrit court, pour tenir dans une colonne : six décimales
/// au plus, sans les zéros qui traînent, et la notation scientifique dès
/// que le nombre sort de ce qu'on lit d'un coup d'œil — un flottant
/// simple précision relu comme un double, par exemple, donne un
/// dix-millième de rien qu'aucune écriture décimale ne rend lisible.
///
/// Un registre vide ne dit rien du tout : seize « = 0 » sur seize lignes
/// n'apprennent rien que les chiffres d'à côté ne disent déjà.
pub(crate) fn short_float(value: f64) -> String {
    if value == 0.0 {
        return String::new();
    }
    if !value.is_finite() {
        return format!("= {value}");
    }
    let magnitude = value.abs();
    if !(1e-4..1e12).contains(&magnitude) {
        return format!("= {value:.3e}");
    }
    let text = format!("{value:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("= {text}")
}

/// Le nom sous lequel montrer un registre vectoriel.
///
/// Le même registre en porte deux : `xmm3` ne désigne que sa moitié
/// basse, `ymm3` le tout. Écrire « YMM » dès que la moitié haute contient
/// quelque chose, c'est dire qu'il y a là plus que la ligne n'en montre —
/// et rappeler au passage qu'il s'agit des mêmes bits.
pub(crate) fn vector_name(name: &str, value: [u128; 2]) -> String {
    if value[1] == 0 {
        name.to_owned()
    } else {
        name.replacen("XMM", "YMM", 1)
    }
}

/// Tous les bits d'un registre vectoriel, puis leur lecture en flottants.
pub(crate) fn xmm_tooltip(s: &Strings, value: [u128; 2]) -> String {
    // La moitié haute n'est montrée que si elle sert : un registre resté
    // en 128 bits n'a pas à traîner seize zéros de plus.
    let wide = value[1] != 0;
    let halves = if wide { 2 } else { 1 };
    let doubles: Vec<String> = (0..halves * 2)
        .map(|lane| {
            format!(
                "{}",
                f64::from_bits((value[lane / 2] >> ((lane % 2) * 64)) as u64)
            )
        })
        .collect();
    let singles: Vec<String> = (0..halves * 4)
        .map(|lane| {
            format!(
                "{}",
                f32::from_bits((value[lane / 4] >> ((lane % 4) * 32)) as u32)
            )
        })
        .collect();
    let bits = if wide {
        format!("{:032x}{:032x}", value[1], value[0])
    } else {
        format!("{:032x}", value[0])
    };
    format!(
        "{}\n{}\n[{}]\n[{}]",
        s.sse_hint,
        bits,
        doubles.join(", "),
        singles.join(", ")
    )
}

pub(crate) fn flag_chip(ui: &mut egui::Ui, theme: &'static Theme, name: &str, on: bool) {
    let color = if on { theme.good } else { theme.dim };
    let galley = ui.painter().layout_no_wrap(
        format!("{name} {}", u8::from(on)),
        egui::TextStyle::Monospace.resolve(ui.style()),
        color,
    );
    let (rect, _) =
        ui.allocate_exact_size(galley.size() + egui::vec2(12.0, 4.0), egui::Sense::hover());
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::from(3),
        egui::Stroke::new(1.0, color),
        egui::StrokeKind::Inside,
    );
    let pos = rect.center() - 0.5 * galley.size();
    ui.painter().galley(pos, galley, color);
}

/// Un vidage mémoire, dans une zone qui défile des deux côtés : seize
/// octets et leur transcription font bien plus large que le panneau,
/// et la ligne était jusqu'ici coupée net sans moyen d'aller voir la
/// suite.
pub(crate) fn hexdump_ui(ui: &mut egui::Ui, id: &str, dump: &str) {
    egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Sans `Extend`, seize octets et leur transcription se
            // replient à la largeur du panneau : une ligne sur deux part
            // à la ligne et le vidage devient illisible. C'est pour ça
            // que la zone défile aussi horizontalement.
            ui.add(
                egui::Label::new(egui::RichText::new(dump).monospace())
                    .wrap_mode(egui::TextWrapMode::Extend),
            );
        });
}

pub(crate) fn safe_hexdump(
    mem: &namsdec_core::Memory,
    addr: u64,
    len: usize,
    out_of_range: &str,
) -> String {
    let range = mem.range();
    if addr < range.start || addr >= range.end {
        return out_of_range.to_string();
    }
    let len = len.min((range.end - addr) as usize);
    match mem.bytes_at(addr, len) {
        Ok(bytes) => hexdump(bytes, addr),
        Err(_) => out_of_range.to_string(),
    }
}

pub(crate) fn hexdump(bytes: &[u8], base_addr: u64) -> String {
    let mut out = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let addr = base_addr + (i * 16) as u64;
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        out.push_str(&format!("{addr:08x}  {:<47}  {ascii}\n", hex.join(" ")));
    }
    out
}

/// Un mot de huit octets par ligne, lu en petit-boutien comme le
/// processeur le fait — c'est la vue qui rend une pile lisible.
pub(crate) fn words_dump(bytes: &[u8], base_addr: u64) -> String {
    let mut out = String::new();
    for (i, chunk) in bytes.as_chunks::<8>().0.iter().enumerate() {
        let word = u64::from_le_bytes(*chunk);
        let addr = base_addr + (i * 8) as u64;
        out.push_str(&format!("{addr:016x}  {word:016x}  {word}\n"));
    }
    out
}

/// Le texte jusqu'au premier octet nul — ce qu'un `write` enverrait.
/// Les octets qui ne forment pas un caractère lisible sont montrés tels
/// quels, entre chevrons, plutôt que remplacés par un point : leur valeur
/// est justement ce qu'on vient vérifier.
pub(crate) fn text_dump(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let mut out = String::new();
    for &b in &bytes[..end] {
        match b {
            b'\n' => out.push_str("\\n\n"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(b as char),
            other => out.push_str(&format!("<{other:02x}>")),
        }
    }
    if end == 0 {
        out.push_str("(vide)");
    } else if end < bytes.len() {
        out.push_str("\n\\0");
    }
    out
}

/// Les mêmes octets, lus de trois façons.
///
/// Le vidage hexadécimal montre la mémoire telle qu'elle est ; les mots
/// de huit octets la montrent telle que la pile la range (adresses de
/// retour, pointeurs, `push`) ; le texte, telle qu'un `write` la
/// enverra. Choisir la lecture, c'est déjà savoir ce qu'on cherche.
pub(crate) fn read_memory(
    mem: &namsdec_core::Memory,
    addr: u64,
    format: MemoryFormat,
    out_of_range: &str,
) -> String {
    let range = mem.range();
    if addr < range.start || addr >= range.end {
        return out_of_range.to_string();
    }
    let available = (range.end - addr) as usize;
    match format {
        MemoryFormat::Bytes => safe_hexdump(mem, addr, 128, out_of_range),
        MemoryFormat::Words => {
            let len = (16 * 8).min(available & !7);
            match mem.bytes_at(addr, len) {
                Ok(bytes) => words_dump(bytes, addr),
                Err(_) => out_of_range.to_string(),
            }
        }
        MemoryFormat::Text => {
            let len = 256.min(available);
            match mem.bytes_at(addr, len) {
                Ok(bytes) => text_dump(bytes),
                Err(_) => out_of_range.to_string(),
            }
        }
    }
}

/// Évalue une expression d'adresse : un nombre, un registre, ou une
/// somme des deux — `0x401000`, `rsp`, `rsi+10`, `rip-8`, `rbx+rcx*4`.
///
/// C'est le `x/` de gdb réduit à ce qui sert : suivre un pointeur qu'on a
/// sous les yeux dans le bandeau, sans recopier seize chiffres à la main.
/// Les registres viennent de `regs` ; sans processeur chargé, seuls les
/// nombres sont acceptés.
///
/// **Tout est hexadécimal**, décalages compris — une seule règle, la même
/// que le vidage d'en dessous. `rsi+10` avance donc de seize octets.
pub(crate) fn eval_address(expr: &str, regs: Option<&namsdec_core::Registers>) -> Option<u64> {
    let expr = expr.trim().to_ascii_lowercase();
    if expr.is_empty() {
        return None;
    }

    let mut total: u64 = 0;
    let mut add = true;
    let mut term = String::new();
    // Les termes se lisent au fil des signes ; le dernier se solde à la
    // sortie de la boucle.
    for c in expr.chars().chain(std::iter::once('+')) {
        match c {
            '+' | '-' => {
                let value = eval_term(&term, regs)?;
                total = if add {
                    total.wrapping_add(value)
                } else {
                    total.wrapping_sub(value)
                };
                add = c == '+';
                term.clear();
            }
            ' ' | '\t' => {}
            _ => term.push(c),
        }
    }
    Some(total)
}

/// Un terme : un facteur, ou deux multipliés (`rcx*4`, l'indexation d'un
/// tableau telle qu'on l'écrit dans `[rbx + rcx*4]`).
pub(crate) fn eval_term(term: &str, regs: Option<&namsdec_core::Registers>) -> Option<u64> {
    let mut product: u64 = 1;
    let mut seen = false;
    for factor in term.split('*') {
        product = product.wrapping_mul(eval_factor(factor, regs)?);
        seen = true;
    }
    seen.then_some(product)
}

pub(crate) fn eval_factor(factor: &str, regs: Option<&namsdec_core::Registers>) -> Option<u64> {
    let factor = factor.trim();
    if factor.is_empty() {
        return None;
    }
    if let Some(reg) = register_by_name(factor) {
        return regs.map(|r| r.get64(reg));
    }
    if factor == "rip" {
        return regs.map(|r| r.rip);
    }
    let digits = factor
        .strip_prefix("0x")
        .or_else(|| factor.strip_prefix("0h"))
        .unwrap_or(factor);
    u64::from_str_radix(digits, 16).ok()
}

/// L'adresse à montrer après validation : celle qui vient d'être tapée si
/// elle en est une, sinon celle qu'on montrait déjà.
///
/// La règle tient dans cette ligne, et c'est elle qui a changé : le vidage
/// retombait sur le début du programme dès que la saisie ne s'analysait
/// pas — une frappe de trop, et l'affichage sautait ailleurs sans rien
/// dire.
pub(crate) fn validated_address(
    current: u64,
    typed: &str,
    regs: Option<&namsdec_core::Registers>,
) -> u64 {
    eval_address(typed, regs).unwrap_or(current)
}

/// La largeur du champ « Adresse » : celle de ce qu'il peut contenir, et
/// non celle du panneau.
///
/// Un champ de saisie egui réclame par défaut toute la place disponible.
/// Pour une ligne de commande, c'est ce qu'on veut ; pour une adresse —
/// « 0x » suivi de seize chiffres au plus — ça donnait un champ long
/// comme le panneau, vide aux trois quarts, dont on ne savait plus ce
/// qu'il attendait.
pub(crate) fn address_field_width(ui: &egui::Ui) -> f32 {
    const MAX_CHARS: f32 = 18.0;
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let digit = ui.fonts_mut(|f| f.glyph_width(&font_id, '0'));
    digit * MAX_CHARS + 12.0
}

/// Le champ « Adresse » de l'onglet Mémoire porte un identifiant fixe :
/// c'est par lui qu'un test retrouve sa largeur.
pub(crate) fn memory_address_id() -> egui::Id {
    egui::Id::new("memory_address")
}
