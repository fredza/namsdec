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

/// Les six indicateurs qu'on lit après une opération, dans l'ordre où le
/// processeur les range. Partagés par les pastilles du bandeau et le
/// récit du dernier pas : les deux doivent parler des mêmes.
const FLAG_BITS: [(&str, RFlags); 6] = [
    ("CF", RFlags::CF),
    ("PF", RFlags::PF),
    ("AF", RFlags::AF),
    ("ZF", RFlags::ZF),
    ("SF", RFlags::SF),
    ("OF", RFlags::OF),
];

impl NamsdecApp {
    pub(crate) fn sidebar_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // RE est masqué par défaut (activable dans Préférences > Modules) :
        // s'il vient d'être caché pendant qu'il était actif, on retombe
        // sur la pile plutôt que de laisser un panneau vide.
        if self.active_tab == SidebarTab::Re && !self.show_re_tab {
            self.active_tab = SidebarTab::Stack;
        }

        // Au-dessus de tout, et hors de toute zone qui défile : c'est ce
        // qu'on regarde entre deux pas, et il ne doit jamais falloir
        // aller le chercher.
        self.next_instruction_ui(ui);
        self.last_step_ui(ui);
        self.advice_ui(ui);

        // Les registres et les flags sont posés en entier, sans zone de
        // défilement : leur hauteur est connue — dix-huit valeurs sur
        // autant de colonnes que le panneau en tient — et une barre au
        // milieu de l'état du processeur oblige à faire défiler pour
        // voir R15 ou ZF, en plein pas à pas.
        //
        // `show_unindented` : le retrait qu'ajoute un en-tête repliable
        // vaut une vingtaine de pixels, et une valeur de registre fait
        // seize chiffres — avec le retrait, la ligne débordait du
        // panneau.
        //
        // `open(Some(...))` plutôt que `default_open(true)` : le premier
        // impose l'état à chaque frame, le second ne vaut qu'à la toute
        // première ouverture et laisse ensuite egui ressortir ce qu'il a
        // retenu de la session précédente.
        let banner = egui::CollapsingHeader::new(s.cpu_state)
            .open(Some(self.cpu_state_open))
            .show_unindented(ui, |ui| self.cpu_state_ui(ui));
        // L'état imposé prive l'en-tête de son propre clic : on le
        // reprend ici, sinon le bandeau ne se replierait plus.
        if banner.header_response.clicked() {
            self.cpu_state_open = !self.cpu_state_open;
        }

        // Les vectoriels ne sont plus ici : seize registres SSE et huit
        // emplacements x87, c'est vingt-quatre lignes de plus dans un
        // panneau qui en a déjà dix-huit. Dépliés, ils poussaient la
        // rangée d'onglets hors du panneau ; bornés, ils défilaient au
        // milieu de l'état du processeur. Ils ont maintenant une fenêtre
        // à eux, où ils tiennent en entier.
        self.vectors_link_ui(ui);

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
            SidebarTab::Bytes => self.bytes_ui(ui),
            SidebarTab::Re => self.re_ui(ui),
        }
    }

    /// Le bandeau fixe : les registres, puis les flags en pastilles.
    fn cpu_state_ui(&mut self, ui: &mut egui::Ui) {
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
    pub(crate) fn xmm_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            return;
        };
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        let changed = changed_registers(self.regs_before.as_ref(), regs);

        ui.strong(s.sse_registers);
        // Autant de colonnes que la fenêtre en tient : seize registres
        // d'affilée font seize lignes, et une fenêtre large les pose sur
        // deux ou trois — sans jamais rien à faire défiler.
        let entry = register_pair_width(ui) + 70.0;
        let columns = register_columns(ui.available_width() - 2.0, entry, 16.0);
        egui::Grid::new("xmm_grid")
            .num_columns(3 * columns)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                for (index, (name, reg)) in XMM_REGISTERS.into_iter().enumerate() {
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
                    // La lecture décimale n'a de sens que si le registre
                    // contient *un* nombre : lire le bas d'un vecteur de
                    // quatre flottants simples comme un double donne un
                    // nombre qui n'existe nulle part. Le détail est dans
                    // l'infobulle.
                    let scalar = value[1] == 0 && value[0] >> 64 == 0;
                    let reading = if scalar {
                        short_float(f64::from_bits(low))
                    } else {
                        String::new()
                    };
                    ui.colored_label(theme.dim, egui::RichText::new(reading).monospace());
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
                ui.end_row();
            });
    }

    /// Ce que le dernier pas a fait : l'instruction, et ce qu'elle a
    /// changé — avec la valeur d'avant.
    ///
    /// Le bandeau des registres colore ce qui a bougé ; savoir *qu'*une
    /// valeur a changé ne dit pas d'où elle vient. `RCX 0x5 → 0x4` sous
    /// un `dec rcx`, c'est l'instruction et son effet dans la même
    /// phrase, et c'est comme cela qu'on apprend ce que fait une
    /// instruction.
    fn last_step_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let (Some((addr, text)), Some(emulator)) = (&self.last_step, &self.emulator) else {
            return;
        };
        let Some(before) = &self.regs_before else {
            return;
        };
        let now = &emulator.cpu().regs;
        // RIP est écarté : il change à chaque instruction, le dire à
        // chaque pas noierait ce qui compte.
        let mut changes: Vec<String> = REGISTERS
            .iter()
            .filter(|&&(_, reg)| before.get64(reg) != now.get64(reg))
            .map(|&(name, reg)| format!("{name} {:#x} → {:#x}", before.get64(reg), now.get64(reg)))
            .collect();
        // Les indicateurs, un par un : après un `cmp`, ce sont eux le
        // résultat — l'instruction ne touche aucun registre.
        if let Some(flags_before) = self.flags_before {
            let flags_now = emulator.cpu().flags;
            for (name, bit) in FLAG_BITS {
                if flags_before.contains(bit) != flags_now.contains(bit) {
                    changes.push(format!(
                        "{name} {} → {}",
                        u8::from(flags_before.contains(bit)),
                        u8::from(flags_now.contains(bit))
                    ));
                }
            }
        }

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.last_step);
            ui.colored_label(
                theme.dim,
                egui::RichText::new(format!("{addr:#x}")).monospace(),
            );
            ui.label(egui::RichText::new(text).monospace());
        });
        if changes.is_empty() {
            // Une instruction peut ne rien changer de visible — un saut,
            // un `nop`. Le dire vaut mieux que de laisser croire qu'on a
            // oublié de regarder.
            ui.colored_label(theme.dim, s.last_step_nothing);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for change in changes {
                ui.colored_label(theme.changed, egui::RichText::new(change).monospace());
            }
        });
    }

    /// L'appel système sur le point d'être fait, écrit comme on
    /// l'écrirait en C : `write(fd = 1, buf = 0x402000 → "salut\n",
    /// count = 6)`.
    ///
    /// Le numéro est dans RAX, les arguments dans RDI, RSI, RDX, R10, R8
    /// et R9 — et surtout pas RCX, que `syscall` écrase, ce qui est
    /// l'erreur qu'on fait en venant des appels de fonction.
    fn syscall_ui(&self, ui: &mut egui::Ui, cpu: &namsdec_core::Cpu) {
        let s = strings(self.lang);
        let theme = self.theme;
        let number = cpu.regs.get64(Reg64::Rax);
        let Some(call) = namsdec_re::table::syscall(number) else {
            ui.colored_label(theme.dim, format!("{} {number}", s.syscall_unknown));
            return;
        };

        let mut text = format!("{}(", call.name);
        for (index, name) in call.args.iter().enumerate() {
            let Some(reg) = SYSCALL_ARGS.get(index) else {
                break;
            };
            let value = cpu.regs.get64(*reg);
            if index > 0 {
                text.push_str(", ");
            }
            text.push_str(&format!("{name} = {value:#x}"));
            // Un tampon qu'on peut lire vaut mieux que son adresse :
            // c'est le texte que le programme est sur le point d'écrire.
            if let Some(extrait) = peek_text(cpu, name, value, &cpu.regs) {
                text.push_str(&format!(" → {extrait}"));
            }
        }
        text.push(')');

        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.accent, egui::RichText::new(text).monospace());
            if !call.simulated {
                ui.colored_label(theme.warn, s.syscall_not_simulated);
            }
        });
    }

    /// Le conseil qui accompagne une erreur d'exécution.
    ///
    /// Le message d'erreur constate — « interdit d'écrire à 0x401000 :
    /// elle est dans .text ». Le conseil dit par où reprendre, et c'est
    /// lui qu'on cherche quand on apprend. Il tient dans le panneau,
    /// sous l'instruction qui a échoué, plutôt que dans la barre de
    /// statut, qui n'a de place que pour une ligne tronquée.
    fn advice_ui(&mut self, ui: &mut egui::Ui) {
        let Some(advice) = self.error_advice else {
            return;
        };
        let theme = self.theme;
        ui.add_space(4.0);
        egui::Frame::new()
            .fill(theme.bad.gamma_multiply(0.12))
            .inner_margin(egui::Margin::symmetric(8, 6))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                ui.colored_label(theme.text, advice);
            });
    }

    /// La ligne qui mène aux registres étendus.
    ///
    /// Un lien plutôt qu'un repli : ce que l'on déplie dans un panneau,
    /// il faut bien le poser quelque part, et il n'y a pas la place. La
    /// fenêtre, elle, s'agrandit.
    fn vectors_link_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        if self.emulator.is_none() {
            return;
        }
        ui.add_space(2.0);
        let link = ui.add(
            egui::Label::new(egui::RichText::new(s.vectors_open).color(theme.dim))
                .sense(egui::Sense::click()),
        );
        if link.on_hover_text(s.vectors_hint).clicked() {
            self.show_vectors = true;
        }
    }

    /// Ce que l'instruction sur le point de s'exécuter va manipuler.
    ///
    /// Le bandeau des registres dit où en est la machine ; celui-ci dit
    /// ce qui va lui arriver. Entre les deux, tout le travail que le
    /// débutant fait de tête et rate une fois sur deux : `[rsi+rcx*8+16]`
    /// désigne *cette* adresse-là, et il y a *cette* valeur dedans.
    ///
    /// Posé sous les registres et au-dessus des vectoriels : il se lit
    /// avec les valeurs qu'il cite, juste au-dessus.
    fn next_instruction_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let Some(emulator) = &self.emulator else {
            return;
        };
        if emulator.cpu().halted {
            ui.add_space(4.0);
            ui.colored_label(theme.dim, s.next_instruction_done);
            return;
        }
        let cpu = emulator.cpu();
        let rip = cpu.regs.rip;
        // Seize octets : la plus longue instruction x86 possible. Moins
        // que cela en fin de mémoire, et le décodage échoue tout seul.
        let window = cpu.mem.peek_bytes(rip, 16).or_else(|_| {
            cpu.mem
                .peek_bytes(rip, (cpu.mem.range().end - rip) as usize)
        });
        let Ok(window) = window else {
            return;
        };
        let Some(instr) = namsdec_disasm::decode_one(window, rip) else {
            return;
        };
        let text = namsdec_disasm::format_one(&instr);

        ui.add_space(4.0);
        ui.colored_label(theme.dim, s.next_instruction);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                theme.accent,
                egui::RichText::new(format!("{rip:#x}")).monospace(),
            );
            ui.label(egui::RichText::new(&text).monospace().strong());
        });
        // La fiche de l'instruction, en une ligne : « Copier une valeur ».
        // Le reste est sous F1 — ici, c'est le rappel qui évite d'y aller.
        let mnemonic = text.split([' ', '\t']).next().unwrap_or_default();
        if let Some(entry) = namsdec_doc::lookup(mnemonic) {
            ui.colored_label(theme.dim, entry.title.get(self.lang.code()));
        }
        // Un `syscall` : lequel, et avec quoi ?
        //
        // L'instruction ne porte aucun opérande — tout est dans les
        // registres, selon une convention qu'il faut connaître par cœur
        // ou chercher ailleurs. C'est le genre de chose qu'un atelier
        // d'apprentissage doit dire à l'endroit où la question se pose.
        if text.trim() == "syscall" {
            self.syscall_ui(ui, cpu);
        }

        // Un saut conditionnel : sera-t-il pris, et pourquoi ?
        //
        // « Quel saut après quelle comparaison » est l'endroit précis où
        // l'on se trompe en apprenant — d'autant que `ja` et `jg` disent
        // tous les deux « plus grand » et ne répondent pas la même chose
        // sur les mêmes octets. La règle est affichée avec le verdict :
        // sans elle, on apprend le résultat, pas la raison.
        let code = instr.condition_code();
        if let Some(taken) = namsdec_emu::condition_met(code, cpu.flags) {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    if taken { theme.good } else { theme.dim },
                    if taken {
                        s.jump_taken
                    } else {
                        s.jump_not_taken
                    },
                );
                if let Some(rule) = namsdec_emu::condition_rule(code) {
                    ui.colored_label(theme.dim, egui::RichText::new(rule).monospace());
                }
                if let Some(signed) = namsdec_emu::condition_signed(code) {
                    ui.colored_label(
                        theme.dim,
                        if signed {
                            s.jump_signed
                        } else {
                            s.jump_unsigned
                        },
                    );
                }
            });
        }

        let lines = namsdec_emu::explain::operands(cpu, &instr);
        if lines.is_empty() {
            return;
        }
        // Les étiquettes du programme, pour nommer les adresses : le
        // désassembleur ne connaît que des nombres, et « tampon+2 » se
        // relit dans le source, pas « 0x401002 ».
        let symbols = self
            .assemble_state
            .as_ref()
            .map(|state| state.output.symbols.as_slice())
            .unwrap_or_default();
        egui::Grid::new("next_instruction_grid")
            .num_columns(3)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                for line in lines {
                    ui.colored_label(theme.dim, egui::RichText::new(line.text).monospace());
                    ui.label(egui::RichText::new(line.value).monospace());
                    let named = line.addr.and_then(|addr| symbol_at(symbols, addr));
                    ui.colored_label(
                        theme.accent,
                        egui::RichText::new(named.unwrap_or_default()).monospace(),
                    );
                    ui.end_row();
                }
            });
        ui.separator();
    }

    fn registers_ui(&mut self, ui: &mut egui::Ui) {
        let Some(emulator) = &self.emulator else {
            ui.label(strings(self.lang).no_binary_assembled);
            return;
        };
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        let changed = changed_registers(self.regs_before.as_ref(), regs);
        // Dix-huit valeurs d'affilée, c'est haut. Le panneau élargi,
        // elles tiennent sur deux colonnes ou plus, et l'état du
        // processeur se lit d'un coup d'œil au lieu de défiler.
        //
        // Deux pixels de garde : une grille plus large que le panneau
        // ferait *grandir* le panneau, qui en offrirait davantage à la
        // frame suivante — le panneau s'élargissait alors sans fin.
        let spacing = ui.spacing().item_spacing.x;
        let available = ui.available_width() - 2.0;
        let pair = register_pair_width(ui);
        // Le blanc minimal entre deux paires : sans lui, seize chiffres
        // suivis de trois lettres se lisent comme un seul mot
        // (« …003c RBX »). La cellule vide vaut deux espacements, et la
        // grille en pose un de chaque côté.
        let min_gap = spacing * 2.0;
        let pairs = register_columns(available, pair, min_gap + 2.0 * spacing);
        // Le reste de la largeur part dans les blancs entre colonnes :
        // la grille s'appuie alors sur les deux bords du panneau au lieu
        // de se tasser à gauche.
        let gap = if pairs > 1 {
            let filled = pairs as f32 * pair + (2 * pairs - 2) as f32 * spacing;
            ((available - filled) / (pairs - 1) as f32).max(min_gap)
        } else {
            min_gap
        };
        // Relevé dans la boucle, appliqué après : la grille emprunte
        // `self` en lecture, la surveillance l'écrit.
        let mut clicked: Option<&'static str> = None;
        let s = strings(self.lang);
        let lang = self.lang;
        egui::Grid::new("registers_grid")
            // Deux cellules par paire, plus une cellule vide entre les
            // paires : c'est elle qui sépare les colonnes.
            .num_columns(3 * pairs - 1)
            // Sans quoi egui donne à chaque colonne la largeur d'un
            // bouton — quarante pixels — même pour trois lettres : la
            // grille réclamait plus que le panneau, qui s'élargissait
            // pour la contenir, ce qui lui en donnait encore plus.
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
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
                    // Le nom du registre est cliquable : il pose une
                    // surveillance. « Arrête-toi quand RAX change » n'a
                    // pas de point d'arrêt possible — on ignore où
                    // l'écriture aura lieu, c'est justement la question.
                    // Un point d'arrêt se pose dans la gouttière, celui-ci
                    // se pose sur le registre lui-même.
                    let watched = self.watched_registers.contains(&name);
                    let étiquette = egui::RichText::new(name).monospace();
                    let étiquette = if watched {
                        étiquette.underline()
                    } else {
                        étiquette
                    };
                    let response = ui
                        .add(egui::Label::new(étiquette.color(color)).sense(egui::Sense::click()));
                    // Le rôle du registre, d'abord : le processeur ne le
                    // connaît pas — RBX et R12 lui sont identiques —, et
                    // rien dans le code ne le rappelle. Puis ce que le
                    // clic ferait.
                    let response = response.on_hover_ui(|ui| {
                        ui.strong(name);
                        ui.label(roles::of(lang, name));
                        ui.add_space(4.0);
                        ui.weak(if name == "RIP" {
                            s.register_rip_hint
                        } else if watched {
                            s.register_unwatch_hint
                        } else {
                            s.register_watch_hint
                        });
                    });
                    // RIP ne se surveille pas : il change à chaque
                    // instruction, s'arrêter dessus n'apprendrait rien.
                    if name != "RIP" && response.clicked() {
                        clicked = Some(name);
                    }
                    // La valeur porte toutes ses lectures : combien ça
                    // fait en décimal, quel caractère, quels octets en
                    // mémoire. C'est la question qui fait ouvrir une
                    // calculatrice ailleurs.
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{value:016x}"))
                                .monospace()
                                .color(color),
                        )
                        .sense(egui::Sense::hover()),
                    )
                    .on_hover_text(readings::of(lang, value));
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

        if let Some(name) = clicked {
            if !self.watched_registers.remove(name) {
                self.watched_registers.insert(name);
            }
            self.sync_watchpoints();
        }
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
        // Tout l'onglet dans une zone qui défile : la chaîne des appels,
        // l'aide-mémoire de la convention et les entorses relevées font
        // déjà plus d'une hauteur de panneau, et le vidage de la pile
        // vient après. Sans cela, ce qui dépasse est simplement coupé.
        egui::ScrollArea::vertical()
            .id_salt("stack_tab")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // La pile d'abord : c'est le sujet de l'onglet. La
                // chaîne des appels et l'aide-mémoire de la convention
                // l'expliquent, mais ils passaient devant elle et la
                // repoussaient hors du panneau.
                self.stack_words_ui(ui, emulator.as_ref());
                ui.separator();
                self.call_chain_ui(ui, emulator.as_ref());
                ui.separator();
                self.abi_ui(ui, emulator.as_ref());
            });
    }

    /// La pile, mot par mot, avec ses repères.
    ///
    /// Seize octets par ligne d'hexadécimal ne montrent pas une pile :
    /// une pile se lit par mots de huit octets, et ce qui compte, ce
    /// sont RSP et RBP — où commence ce qui est à moi, où commence le
    /// cadre de l'appelant. Ce qui est *au-dessus* de RSP a été dépilé :
    /// les octets sont encore là, mais ils ne m'appartiennent plus, et
    /// le premier `push` venu les écrasera. On les montre en grisé, pour
    /// que cela se voie plutôt que de s'apprendre à ses dépens.
    fn stack_words_ui(&self, ui: &mut egui::Ui, emulator: &dyn Backend) {
        let s = strings(self.lang);
        let lang = self.lang;
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        let rsp = regs.get64(Reg64::Rsp);
        let rbp = regs.get64(Reg64::Rbp);
        let symbols = self
            .assemble_state
            .as_ref()
            .map(|state| state.output.symbols.as_slice())
            .unwrap_or_default();
        // Les zones de code : une valeur qui pointe dedans est une
        // adresse de retour, pas un nombre.
        let code: Vec<std::ops::Range<u64>> = self
            .assemble_state
            .as_ref()
            .map(|state| {
                state
                    .output
                    .sections
                    .iter()
                    .filter(|section| section.executable)
                    .map(|section| section.addr..section.addr + section.size as u64)
                    .collect()
            })
            .unwrap_or_default();

        // Deux mots au-dessus de RSP — ce qui vient d'être dépilé — et
        // dix en dessous : de quoi voir un cadre d'appel entier.
        let first = rsp.saturating_sub(16);
        egui::Grid::new("stack_words")
            .num_columns(3)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                for index in 0..12u64 {
                    let addr = first + index * 8;
                    let Ok(value) = emulator.cpu().mem.peek_bytes(addr, 8) else {
                        continue;
                    };
                    let value = u64::from_le_bytes(value.try_into().unwrap_or_default());
                    let dépilé = addr < rsp;
                    let colour = if dépilé {
                        theme.dim
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.colored_label(
                        colour,
                        egui::RichText::new(format!("{addr:#010x}")).monospace(),
                    );
                    ui.colored_label(
                        colour,
                        egui::RichText::new(format!("{value:016x}")).monospace(),
                    )
                    .on_hover_text(readings::of(lang, value));
                    // Le repère : d'abord ce que l'adresse *est* pour le
                    // processeur, puis ce que la valeur veut dire.
                    let mut note = String::new();
                    if addr == rsp {
                        note.push_str("← RSP");
                    }
                    if addr == rbp && rbp != 0 {
                        if !note.is_empty() {
                            note.push_str(", ");
                        }
                        note.push_str("← RBP");
                    }
                    if code.iter().any(|range| range.contains(&value)) {
                        if !note.is_empty() {
                            note.push_str(" · ");
                        }
                        note.push_str(s.stack_return_address);
                        if let Some(label) = symbol_at(symbols, value) {
                            note.push_str(&format!(" ({label})"));
                        }
                    }
                    ui.colored_label(
                        if addr == rsp || addr == rbp {
                            theme.accent
                        } else {
                            theme.dim
                        },
                        note,
                    );
                    ui.end_row();
                }
            });
        ui.weak(s.stack_above_hint);
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
    pub(crate) fn x87_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            return;
        };
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        let changed = changed_registers(self.regs_before.as_ref(), regs);

        ui.strong(s.x87_stack);
        let entry = register_pair_width(ui);
        let columns = register_columns(ui.available_width() - 2.0, entry, 16.0);
        egui::Grid::new("x87_grid")
            .num_columns(2 * columns)
            .min_col_width(0.0)
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
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
                ui.end_row();
            });
    }

    /// La convention d'appel : par où passent les arguments, où revient
    /// le résultat, ce qu'une sous-routine doit rendre intact — et ce
    /// qu'elle n'a pas rendu.
    ///
    /// Les deux premières lignes sont un aide-mémoire, celui qu'on
    /// cherche dans un tableau au dos d'un livre. La troisième est ce
    /// qu'aucun livre ne peut donner : la vérification, faite sur *ce*
    /// programme, en comparant l'avant et l'après de chaque appel.
    fn abi_ui(&self, ui: &mut egui::Ui, emulator: &dyn Backend) {
        let s = strings(self.lang);
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        ui.label(egui::RichText::new(s.abi_title).color(theme.dim));

        // Des lignes qui se replient, et non une grille : six registres
        // et leurs valeurs ne tiennent pas sur une ligne de panneau, et
        // une cellule de grille qui se replie déborde sur la ligne
        // suivante — le texte s'y imprimait par-dessus.
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.abi_arguments);
            for (index, (name, reg)) in namsdec_emu::ARGUMENTS.iter().enumerate() {
                let value = regs.get64(*reg);
                // Le numéro de l'argument compte autant que le nom du
                // registre : c'est lui qu'on cherche.
                ui.colored_label(theme.register, egui::RichText::new(*name).monospace())
                    .on_hover_text(format!("{} {}", index + 1, s.abi_arguments));
                ui.colored_label(
                    if value == 0 { theme.dim } else { theme.text },
                    egui::RichText::new(format!("{value:#x}")).monospace(),
                );
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.abi_result);
            ui.colored_label(
                theme.text,
                egui::RichText::new(format!("RAX {:#x}", regs.get64(Reg64::Rax))).monospace(),
            );
        });
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.abi_preserved);
            for (name, _) in namsdec_emu::PRESERVED {
                ui.colored_label(theme.dim, egui::RichText::new(name).monospace());
            }
        });

        let entorses = emulator.breaches();
        if entorses.is_empty() {
            return;
        }
        ui.add_space(4.0);
        ui.colored_label(theme.warn, s.abi_breaches);
        for entorse in entorses {
            let quoi = if entorse.register == "RSP" {
                s.abi_breach_stack.to_owned()
            } else {
                format!("{} {}", s.abi_breach_line, entorse.register)
            };
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    theme.warn,
                    egui::RichText::new(format!("{:#x}", entorse.entry)).monospace(),
                );
                ui.colored_label(theme.warn, quoi);
                // La ligne source quand on la connaît, l'adresse sinon :
                // « appelée depuis la ligne 14 » se cherche des yeux,
                // « appelée depuis 0x400014 » beaucoup moins.
                let depuis = match self
                    .assemble_state
                    .as_ref()
                    .and_then(|state| state.addr_to_line.get(&entorse.call_site).copied())
                {
                    Some(line) => format!("({} {} {line})", s.abi_called_from, s.call_chain_line),
                    None => format!("({} {:#x})", s.abi_called_from, entorse.call_site),
                };
                ui.colored_label(theme.dim, depuis);
            });
        }
    }

    /// Ce que l'assembleur a produit : une ligne par instruction, avec
    /// son adresse, ses octets, et la ligne source d'où elle vient.
    ///
    /// C'est la moitié du métier qu'aucun bandeau de registres ne montre.
    /// On y voit d'un coup d'œil pourquoi `mov eax, 1` fait cinq octets
    /// et `mov rax, 1` dix, ce que coûte un préfixe REX, et que
    /// `xor eax, eax` tient en deux octets là où `mov eax, 0` en demande
    /// cinq — trois choses qu'on peut lire dix fois sans les retenir, et
    /// qu'une colonne de chiffres apprend en une.
    fn bytes_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let Some(state) = &self.assemble_state else {
            ui.label(s.no_binary_assembled);
            return;
        };
        let rip = self.emulator.as_ref().map(|e| e.cpu().regs.rip);

        let total: usize = state.disasm.iter().map(|i| i.bytes.len()).sum();
        ui.colored_label(
            theme.dim,
            format!(
                "{} {} · {} {}",
                state.disasm.len(),
                s.bytes_instructions,
                total,
                s.bytes_octets
            ),
        );
        ui.add_space(4.0);

        // Verticale seulement : `scroll_to_me` recentre *les deux* axes,
        // et suivre l'exécution faisait alors glisser la vue vers la
        // droite, cachant l'adresse et le numéro de ligne — les deux
        // colonnes qu'on vient lire.
        egui::ScrollArea::vertical()
            .id_salt("bytes_tab")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Un panneau egui adopte la largeur de son contenu, et
                // dix octets en hexadécimal suivis d'un texte
                // d'instruction en font beaucoup : sans cette borne,
                // l'onglet poussait le panneau — et l'éditeur avec.
                let largeur = ui.available_width();
                ui.set_max_width(largeur);
                for instruction in &state.disasm {
                    let here = rip == Some(instruction.address);
                    let colour = if here {
                        theme.accent
                    } else {
                        ui.visuals().text_color()
                    };
                    let octets = instruction
                        .bytes
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let ligne = state
                        .addr_to_line
                        .get(&instruction.address)
                        .map(|l| format!("{l:>3}"))
                        .unwrap_or_else(|| "   ".to_owned());
                    // Deux étiquettes seulement, et la seconde se
                    // tronque : dans une rangée horizontale, chaque
                    // étiquette réclame sa largeur entière, et quatre
                    // colonnes côte à côte poussaient le panneau. Le
                    // préfixe — ligne, adresse, longueur — a une largeur
                    // fixe ; tout le reste tient dans ce qui subsiste.
                    let response = ui.horizontal(|ui| {
                        ui.set_max_width(largeur);
                        ui.colored_label(
                            if here { theme.accent } else { theme.dim },
                            egui::RichText::new(format!("{ligne} {:08x}", instruction.address))
                                .monospace(),
                        );
                        ui.colored_label(
                            theme.register,
                            egui::RichText::new(format!("{:2}", instruction.bytes.len()))
                                .monospace(),
                        )
                        .on_hover_text(s.bytes_length_hint);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("{octets}  {}", instruction.text))
                                    .monospace()
                                    .color(colour),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&instruction.text);
                    });
                    // Suivre l'exécution : la ligne courante revient sous
                    // les yeux à chaque pas, sans quoi il faudrait la
                    // chercher à la main dans un long programme.
                    if here && self.follow_rip {
                        response.response.scroll_to_me(Some(egui::Align::Center));
                    }
                }
            });
        self.follow_rip = false;
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
        // Ce que l'adresse regardée *est* : quelle zone, avec quels
        // droits. La protection devient ainsi une chose qu'on lit avant
        // de se faire refuser un accès, et non seulement une punition.
        let mem = &emulator.cpu().mem;
        let quartier = region_line(s, mem, self.mem_addr);
        // En octets, le vidage est mis en forme ici pour pouvoir colorer
        // ce qui vient de changer ; les deux autres formats restent du
        // texte simple.
        let coloré = (self.memory_format == MemoryFormat::Bytes)
            .then(|| {
                let taille =
                    (mem.range().end.saturating_sub(self.mem_addr) as usize).min(MEMORY_WINDOW);
                mem.peek_bytes(self.mem_addr, taille).ok().map(|bytes| {
                    hexdump_job(ui, theme, bytes, self.mem_addr, self.mem_before.as_deref())
                })
            })
            .flatten();
        let dump = read_memory(
            mem,
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
        ui.colored_label(theme.dim, quartier);
        match coloré {
            Some(job) => {
                egui::ScrollArea::both()
                    .id_salt("memory_dump")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
                    });
            }
            None => hexdump_ui(ui, "memory_dump", &dump),
        }

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
            self.re_modules_ui(ui);
        });
    }

    /// Les modules de rétro-ingénierie, les uns sous les autres.
    ///
    /// Posés ici sans zone de défilement : l'onglet du panneau et la
    /// fenêtre d'analyse ont chacun la leur, et deux zones imbriquées ne
    /// défilent bien ni l'une ni l'autre.
    pub(crate) fn re_modules_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            ui.label(s.no_binary_assembled);
            return;
        };
        let ctx = ReContext {
            cpu: emulator.cpu(),
            disasm: &state.disasm,
            source: &self.source,
            binary: &state.output.file,
            lang: self.lang.code(),
        };
        for module in self.re_registry.iter_mut() {
            ui.group(|ui| {
                ui.strong(module.title(ctx.lang));
                module.ui(ui, &ctx);
            });
        }
    }
}

/// Combien de paires nom/valeur tiennent côte à côte dans `width`, une
/// paire mesurant `pair` et le blanc entre deux paires `gap`.
///
/// Mesuré, et non estimé : la largeur d'une paire dépend de la police de
/// chasse fixe et de sa taille, que le thème et le réglage de zoom
/// changent tous les deux. Une constante en pixels donnait une colonne
/// unique sur un panneau qui en tenait deux — et le bandeau d'état, deux
/// fois trop haut, réclamait une barre de défilement.
///
/// On n'ajoute une colonne que si elle tient *entièrement* : un
/// débordement, dans un panneau, ce n'est pas du texte coupé, c'est la
/// largeur du panneau qui change.
pub(crate) fn register_columns(width: f32, pair: f32, gap: f32) -> usize {
    if pair <= 0.0 {
        return 1;
    }
    let columns = ((width + gap) / (pair + gap)).floor() as usize;
    // Quatre au plus : au-delà, l'œil ne suit plus la ligne jusqu'à la
    // valeur, et le panneau n'est de toute façon pas fait pour ça.
    columns.clamp(1, 4)
}

/// La largeur d'une paire « nom de registre + valeur sur seize chiffres »,
/// dans la police que l'interface utilise réellement.
pub(crate) fn register_pair_width(ui: &egui::Ui) -> f32 {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let width = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
            .rect
            .width()
    };
    // « RFLAGS » n'est pas dans la grille ; le nom le plus large y fait
    // trois caractères, et la valeur seize.
    width("RAX") + ui.spacing().item_spacing.x + width("0000000000000000")
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

/// La zone qui contient cette adresse, et ses droits : « .data · lecture,
/// écriture ». Pour les adresses qui n'appartiennent à rien, on le dit —
/// c'est souvent la réponse qu'on cherchait.
pub(crate) fn region_line(s: &Strings, mem: &namsdec_core::Memory, addr: u64) -> String {
    if !mem.has_regions() {
        return String::new();
    }
    let Some(perms) = mem.perms_at(addr) else {
        return s.perms_unmapped.to_owned();
    };
    let mut droits = Vec::new();
    if perms.read {
        droits.push(s.perms_read);
    }
    if perms.write {
        droits.push(s.perms_write);
    }
    if perms.execute {
        droits.push(s.perms_execute);
    }
    match mem.region_name(addr) {
        Some(nom) => format!("{nom} · {}", droits.join(", ")),
        None => droits.join(", "),
    }
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
    match mem.peek_bytes(addr, len) {
        Ok(bytes) => hexdump(bytes, addr),
        Err(_) => out_of_range.to_string(),
    }
}

/// Le même vidage, mais en couleur : les octets qui ont changé depuis le
/// pas précédent ressortent.
///
/// C'est le tour qui marche le mieux sur les registres — voir d'un coup
/// d'œil ce que la dernière instruction a fait —, et la mémoire n'avait
/// pas de raison d'y échapper. Sans lui, on relit seize octets en
/// cherchant lequel a bougé.
pub(crate) fn hexdump_job(
    ui: &egui::Ui,
    theme: &'static Theme,
    bytes: &[u8],
    base_addr: u64,
    before: Option<&[u8]>,
) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let plain = egui::TextFormat {
        font_id: font.clone(),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let dim = egui::TextFormat {
        color: theme.dim,
        ..plain.clone()
    };
    let changed = egui::TextFormat {
        color: theme.changed,
        ..plain.clone()
    };
    let mut job = egui::text::LayoutJob::default();
    let a_changé = |index: usize, byte: u8| -> bool {
        before.is_some_and(|b| b.get(index).is_some_and(|old| *old != byte))
    };

    for (line, chunk) in bytes.chunks(16).enumerate() {
        let start = line * 16;
        job.append(
            &format!("{:08x}  ", base_addr + start as u64),
            0.0,
            dim.clone(),
        );
        for (i, byte) in chunk.iter().enumerate() {
            let format = if a_changé(start + i, *byte) {
                changed.clone()
            } else {
                plain.clone()
            };
            job.append(&format!("{byte:02x} "), 0.0, format);
        }
        // Les lignes incomplètes gardent la colonne de texte alignée.
        for _ in chunk.len()..16 {
            job.append("   ", 0.0, plain.clone());
        }
        job.append(" ", 0.0, plain.clone());
        for (i, byte) in chunk.iter().enumerate() {
            let lisible = if (0x20..0x7f).contains(byte) {
                *byte as char
            } else {
                '.'
            };
            let format = if a_changé(start + i, *byte) {
                changed.clone()
            } else {
                dim.clone()
            };
            job.append(&lisible.to_string(), 0.0, format);
        }
        job.append("\n", 0.0, plain.clone());
    }
    job
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
            match mem.peek_bytes(addr, len) {
                Ok(bytes) => words_dump(bytes, addr),
                Err(_) => out_of_range.to_string(),
            }
        }
        MemoryFormat::Text => {
            let len = 256.min(available);
            match mem.peek_bytes(addr, len) {
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

/// Les registres d'arguments d'un appel système, dans l'ordre. Jumeaux
/// des noms de `namsdec_re::table::ARGUMENT_REGISTERS`.
const SYSCALL_ARGS: [Reg64; 6] = [
    Reg64::Rdi,
    Reg64::Rsi,
    Reg64::Rdx,
    Reg64::R10,
    Reg64::R8,
    Reg64::R9,
];

/// Le texte pointé par un argument qui en porte un — le `buf` d'un
/// `write`, le chemin d'un `open`.
///
/// Rendu entre guillemets et écourté : on veut reconnaître ce qui va
/// être écrit, pas le relire en entier.
fn peek_text(
    cpu: &namsdec_core::Cpu,
    name: &str,
    addr: u64,
    regs: &namsdec_core::Registers,
) -> Option<String> {
    if !matches!(name, "buf" | "chemin") {
        return None;
    }
    // La longueur demandée quand elle existe (le `count` d'un write),
    // bornée à ce qui tient sur une ligne.
    let count = regs.get64(Reg64::Rdx).clamp(1, 24) as usize;
    let bytes = cpu.mem.peek_bytes(addr, count).ok()?;
    let readable: String = bytes
        .iter()
        .take_while(|&&b| b != 0)
        .map(|&b| match b {
            b'\n' => "\\n".to_owned(),
            b'\t' => "\\t".to_owned(),
            b if b.is_ascii_graphic() || b == b' ' => (b as char).to_string(),
            _ => "·".to_owned(),
        })
        .collect();
    (!readable.is_empty()).then(|| format!("\"{readable}\""))
}
