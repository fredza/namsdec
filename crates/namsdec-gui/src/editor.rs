//! L'éditeur : le code, sa gouttière, ses onglets, et les trois barres
//! qui s'y ouvrent — chercher, renommer, commenter.
//!
//! C'est la partie de namsdec où l'on passe ses journées, et la plus
//! dense : un  d'egui ne sait ni numéroter les lignes, ni
//! porter des points d'arrêt, ni colorer une syntaxe, et tout cela se
//! dessine à côté de lui en restant aligné au pixel près.
//!
//! Sorti de  le 29 septembre 2026.

use super::*;

impl NamsdecApp {
    /// La ligne source où le programme est arrêté, s'il l'est.
    pub(crate) fn current_line(&self) -> Option<u32> {
        let emulator = self.emulator.as_ref()?;
        let state = self.assemble_state.as_ref()?;
        state.addr_to_line.get(&emulator.cpu().regs.rip).copied()
    }

    /// Pose le curseur au début d'une ligne, et y amène la vue.
    ///
    /// Appelé au rendu de l'éditeur, une fois seulement : le curseur
    /// appartient à l'utilisateur, on ne le lui reprend pas à chaque
    /// frame.
    pub(crate) fn apply_goto_line(&mut self, ctx: &egui::Context) {
        // La position de lecture d'une séance précédente : la vue y est
        // déjà allée (voir la gouttière), le curseur l'y suit, et c'est
        // tout — pas de changement de mode, pas de clavier pris.
        //
        // Posé ICI et non à la reprise elle-même : l'état du champ de
        // texte n'existe qu'une fois le champ dessiné, et c'est
        // précisément ce que cette fonction attend pour s'exécuter —
        // elle est appelée après lui, pour cette raison-là.
        if let Some(ligne) = self.reprise_ligne.take() {
            let octet = ligne_début(&self.source, ligne.saturating_sub(1) as usize).unwrap_or(0);
            if set_cursor_byte(ctx, &self.source, octet) {
                self.cursor_byte = octet;
                self.cursor_pos = (ligne as usize, 1);
            }
        }
        let Some(line) = self.goto_line.take() else {
            return;
        };
        let offset: usize = self
            .source
            .lines()
            .take(line.saturating_sub(1) as usize)
            .map(|l| l.chars().count() + 1)
            .sum();
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(offset),
                )));
            state.store(ctx, editor_id());
        }
        self.editor_mode = EditorMode::Insert;
        ctx.memory_mut(|m| m.request_focus(editor_id()));
    }

    /// Replace le curseur là où la dernière modification l'attend.
    pub(crate) fn apply_pending_cursor(&mut self, ctx: &egui::Context) {
        if let Some(étendue) = self.pending_selection.take() {
            let début = byte_to_char(&self.source, étendue.start.min(self.source.len()));
            let fin = byte_to_char(&self.source, étendue.end.min(self.source.len()));
            if let Some(mut état) = egui::TextEdit::load_state(ctx, editor_id()) {
                état
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(début),
                        egui::text::CCursor::new(fin),
                    )));
                état.store(ctx, editor_id());
            }
            ctx.memory_mut(|m| m.request_focus(editor_id()));
            return;
        }
        let Some(at) = self.pending_cursor.take() else {
            return;
        };
        set_cursor_byte(ctx, &self.source, at.min(self.source.len()));
    }
    /// Un vrai éditeur : gouttière avec numéros de ligne et points d'arrêt
    /// cliquables (plus de saisie de numéro à la main), ligne courante en
    /// surbrillance. La gouttière et le `TextEdit` partagent la même
    /// `ScrollArea` pour rester synchronisés au défilement ; le
    /// retour-à-la-ligne automatique du `TextEdit` est désactivé
    /// (`desired_width(INFINITY)` + défilement horizontal disponible) pour
    /// qu'une ligne logique corresponde toujours à une seule ligne visuelle
    /// — sinon la gouttière se décale sur une ligne trop longue.
    pub(crate) fn editor_ui(&mut self, ui: &mut egui::Ui) {
        // Avant de dessiner : la barre d'état, peinte plus tôt dans la
        // frame, lit ce qu'on calcule ici.
        self.refresh_fix();
        let theme = self.theme;
        // Les fichiers ouverts, en tête : on passe de l'un à l'autre
        // sans rien perdre de ce qu'on faisait sur les autres.
        self.tabs_bar_ui(ui);
        self.disk_changed_ui(ui);
        // La barre de recherche, au-dessus du code : elle pousse
        // l'éditeur d'une ligne plutôt que de flotter par-dessus, pour
        // ne jamais couvrir ce qu'on vient de trouver.
        self.find_ui(ui);
        self.rename_ui(ui);
        self.note_ui(ui);
        // Modes façon (n)vim : `i`/`I` pour passer en Insertion, `Échap`
        // pour revenir en Normal — désactivé pendant qu'une autre capture
        // de texte est active (palette de commandes, rebind de raccourci)
        // pour ne pas leur voler leurs propres touches.
        let mut just_entered_insert = false;
        if self.command_palette.is_none() && self.rebinding.is_none() && !self.explorer_focused {
            // Les touches déjà prises par un raccourci ne sont plus
            // disponibles ici : le `i` qui termine `Espace s i` a servi à
            // demander un pas, pas à ouvrir la saisie.
            let consumed = self.dispatcher.consumed().to_vec();
            let g_armé = self.g_pending;
            let compte_armé = self.count_pending;
            let (enter_insert, exit_insert, mouvement, armer_g, désarmer_g, compte) =
                ui.ctx().input(|inp| {
                    let mut enter = false;
                    let mut exit = false;
                    let mut mouvement = None;
                    let mut armer_g = false;
                    let mut désarmer_g = false;
                    // Le compteur en cours de frappe : `None` tant
                    // qu'aucun chiffre n'est venu.
                    let mut compte = compte_armé;
                    for event in &inp.events {
                        if let egui::Event::Key {
                            key,
                            pressed: true,
                            repeat: false,
                            modifiers,
                            ..
                        } = event
                        {
                            // `Maj+I` doit rester "I" (vim l'utilise pour insérer
                            // en début de ligne) — seuls Ctrl/Alt/Cmd distinguent
                            // ça d'un raccourci sans rapport, comme Ctrl+Alt+I
                            // (Step Into).
                            let plain = !modifiers.ctrl && !modifiers.alt && !modifiers.mac_cmd;
                            match key {
                                egui::Key::I if plain && !consumed.contains(key) => enter = true,
                                egui::Key::Escape => exit = true,
                                // Les déplacements, seulement en mode Normal
                                // et seulement s'ils n'ont pas déjà servi à
                                // un raccourci.
                                egui::Key::H if plain && !consumed.contains(key) => {
                                    mouvement = Some(Move::Left);
                                }
                                egui::Key::L if plain && !consumed.contains(key) => {
                                    mouvement = Some(Move::Right);
                                }
                                egui::Key::K if plain && !consumed.contains(key) => {
                                    mouvement = Some(Move::Up);
                                }
                                egui::Key::J if plain && !consumed.contains(key) => {
                                    mouvement = Some(Move::Down);
                                }
                                // LES CHIFFRES SONT UN COMPTEUR, comme
                                // dans vim : `5j` descend de cinq
                                // lignes. Ils s'accumulent jusqu'au
                                // mouvement qui les consomme.
                                //
                                // Le `0` fait exception quand rien ne
                                // le précède : seul, il va en début de
                                // ligne — c'est son rôle dans vim, et il
                                // ne devient un chiffre qu'une fois
                                // qu'un autre l'a ouvert (`10j`).
                                egui::Key::Num0 if plain && compte.is_some() => {
                                    compte = Some(borne_du_compte(compte, 0));
                                }
                                egui::Key::Num0 if plain => mouvement = Some(Move::LineStart),
                                _ if plain
                                    && !consumed.contains(key)
                                    && chiffre_de(*key).is_some() =>
                                {
                                    compte = chiffre_de(*key).map(|c| borne_du_compte(compte, c));
                                }
                                egui::Key::Home => mouvement = Some(Move::LineStart),
                                egui::Key::End => mouvement = Some(Move::LineEnd),
                                // `G` : la fin du fichier, comme dans vim.
                                egui::Key::G if plain && modifiers.shift => {
                                    mouvement = Some(Move::FileEnd);
                                }
                                // `g g` : le début. Deux frappes de la même
                                // touche, comme dans vim, et donc un témoin
                                // à garder d'une image à l'autre — `G` est
                                // la même touche avec Maj, il n'y a pas de
                                // conflit.
                                egui::Key::G if plain && !modifiers.shift => {
                                    if g_armé {
                                        mouvement = Some(Move::FileStart);
                                        désarmer_g = true;
                                    } else {
                                        armer_g = true;
                                    }
                                }
                                // Les mouvements par mot : ce qu'on réclame
                                // en premier avec l'habitude de vim. Avancer
                                // caractère par caractère dans
                                // `mov qword [rbp-0x18], rax` use la patience.
                                egui::Key::W if plain && !consumed.contains(key) => {
                                    mouvement = Some(Move::WordForward);
                                }
                                egui::Key::B if plain && !consumed.contains(key) => {
                                    mouvement = Some(Move::WordBack);
                                }
                                egui::Key::E if plain && !consumed.contains(key) => {
                                    mouvement = Some(Move::WordEnd);
                                }
                                _ => {}
                            }
                        }
                    }
                    (enter, exit, mouvement, armer_g, désarmer_g, compte)
                });
            // `g` seul attend son second `g` ; tout le reste le désarme.
            // Sans cela, `g` puis `j` sauterait au début du fichier à la
            // prochaine frappe de `g`, une minute plus tard.
            self.g_pending = if désarmer_g { false } else { armer_g };
            self.count_pending = compte;
            // Se déplacer avant de changer de mode : `i` entre en
            // Insertion là où le curseur est, non là où il était.
            if self.editor_mode == EditorMode::Normal
                && let Some(mouvement) = mouvement
            {
                // Le compteur est consommé par le mouvement, et par lui
                // seul : `5` puis `i` entre en Insertion sans rien
                // répéter, comme dans vim.
                let fois = self.count_pending.take().unwrap_or(1);
                self.move_in_normal_mode_times(ui.ctx(), mouvement, fois);
            }
            // Entrer en Insertion efface le compteur : ce qu'on va taper
            // est du texte, pas une répétition.
            if enter_insert || exit_insert {
                self.count_pending = None;
            }
            // Échap éteint aussi le surlignage, avant de changer de
            // mode : c'est le geste qui annule ce qu'on vient de
            // demander, et l'attendre d'un second raccourci serait une
            // chose de plus à retenir.
            //
            // Sans rien annoncer, en revanche : on appuie sur Échap pour
            // quitter l'insertion, et lire « Surlignage effacé » là où
            // l'on attend « NORMAL » répond à côté. L'effacement se voit
            // — les marques disparaissent —, il n'a pas à être dit.
            if exit_insert && self.tracked.is_some() {
                self.forget_matches();
            }
            match self.editor_mode {
                EditorMode::Normal if enter_insert => {
                    self.editor_mode = EditorMode::Insert;
                    just_entered_insert = true;
                }
                EditorMode::Insert if exit_insert => self.editor_mode = EditorMode::Normal,
                _ => {}
            }
        }

        // La ligne courante d'exécution n'est plus annoncée par un bandeau
        // au-dessus de l'éditeur : elle a sa place dans la barre de
        // statut, avec la position du curseur.
        let current_line = self.current_line();
        let editor_bg = ui.visuals().extreme_bg_color;
        let font_id = egui::TextStyle::Monospace.resolve(ui.style());

        // Gouttière et texte peints sur UN seul fond continu (celui du
        // `TextEdit`, `extreme_bg_color`) plutôt que le `panel_fill` du
        // panneau englobant, et le `TextEdit` est rendu sans cadre
        // (`Frame::NONE`) pour ne pas repeindre un fond/une bordure
        // différents par-dessus.
        // Mesurées avant d'entrer dans la zone de défilement : ce sont
        // les dimensions *visibles*, celles que le contenu doit remplir
        // pour qu'aucune barre n'ait lieu d'être.
        let visible = ui.available_size();
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        // Un pas vient d'avancer : on tient le défilement horizontal à
        // zéro quelques images, pour que le code revienne sous les yeux et
        // y reste malgré l'élan d'une molette lancée. Le compteur retombe
        // ensuite, et l'on redéfile à la main librement.
        let suivre = self.editor_follow > 0;
        self.editor_follow = self.editor_follow.saturating_sub(1);
        egui::Frame::new().fill(editor_bg).show(ui, |ui| {
            let mut zone = egui::ScrollArea::both()
                .id_salt("editeur_scroll")
                .auto_shrink([false, false]);
            if suivre {
                zone = zone.horizontal_scroll_offset(0.0);
            }
            zone.show(ui, |ui| {
                // Calculé UNE fois ici, en dehors du `TextEdit`, pour que
                // la gouttière connaisse la position Y *exacte* de
                // chaque ligne telle que le texte la rendra réellement,
                // plutôt que de deviner une hauteur de ligne générique
                // (`ui.text_style_height`) qui dérivait de la vraie
                // mise en page ligne après ligne — c'est ce décalage
                // cumulatif qui désynchronisait gouttière et texte.
                let galley = layout_code(ui, theme, &self.source);

                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // `GUTTER_PAD` est l'air entre le dernier chiffre
                    // et la première colonne de code. Il fait partie
                    // de la gouttière plutôt que d'être un
                    // `item_spacing` : l'espacement horizontal est mis
                    // à zéro juste au-dessus pour que le `TextEdit`
                    // commence exactement où la gouttière s'arrête.
                    const GUTTER_PAD: f32 = 16.0;
                    const GUTTER_WIDTH: f32 = 38.0 + GUTTER_PAD;
                    // Combien de fois chaque ligne a tourné. C'est le
                    // profil du programme : la boucle qu'on croyait
                    // écrire a-t-elle tourné cinq fois, ou une seule ?
                    // La branche qu'on croyait prendre a-t-elle
                    // seulement été atteinte ? Rien d'autre à l'écran
                    // ne répond à ça.
                    let passages = self.line_visits();
                    // Le plus grand nombre de passages : c'est lui qui
                    // donne l'échelle. Une ligne exécutée mille fois
                    // dans un programme qui en compte mille n'est pas
                    // chaude — elle est la normale.
                    let plus_chaude = passages.values().copied().max().unwrap_or(0);
                    let (gutter_rect, gutter_resp) = ui.allocate_exact_size(
                        egui::vec2(GUTTER_WIDTH, galley.rect.height().max(1.0)),
                        egui::Sense::click_and_drag(),
                    );
                    let click_y = gutter_resp.interact_pointer_pos().map(|p| p.y);
                    let mut clicked_line = None;
                    // Les lignes dont nasm s'est plaint : la console
                    // les liste, mais c'est ici qu'on écrit, et une
                    // erreur qu'on ne voit pas là où l'on tape ne
                    // sert qu'à moitié.
                    let fautives: BTreeMap<u32, Severity> = self
                        .diagnostics
                        .iter()
                        .filter(|d| d.line > 0)
                        .map(|d| (d.line, d.severity))
                        .collect();
                    let largeur_ligne = gutter_rect.width() + ui.available_width();
                    // ALLER À UNE LIGNE, C'EST L'AMENER SOUS LES YEUX.
                    //
                    // Poser le curseur ne suffisait pas, et c'est le
                    // défaut qui faisait croire que les adresses des
                    // panneaux d'analyse ne menaient nulle part : egui
                    // ne défile vers le curseur d'un champ de texte que
                    // lorsque la sélection change PENDANT que le champ
                    // a le focus. Quand c'est nous qui posons le
                    // curseur dans l'état du champ, il n'a rien changé
                    // à l'image suivante — donc aucun défilement. On
                    // lisait la ligne 60 749 dans la barre d'état,
                    // tandis que le listing restait à la ligne 1.
                    //
                    // Centrée, et non simplement rendue visible : on
                    // vient lire ce qu'il y a AUTOUR de cette ligne.
                    if let Some(ligne) = self.goto_line.or(self.reprise_ligne)
                        && let Some(row) = galley.rows.get(ligne.saturating_sub(1) as usize)
                    {
                        ui.scroll_to_rect(
                            row.rect().translate(gutter_rect.min.to_vec2()),
                            Some(egui::Align::Center),
                        );
                    }
                    // La ligne sous le pointeur, s'il est dans la
                    // gouttière. Rien à l'écran ne disait qu'on
                    // pouvait cliquer là : le manuel l'écrit, la
                    // gouttière se taisait. Un point fantôme le dit
                    // maintenant, comme le fait n'importe quel
                    // éditeur — et c'est là que ça s'apprend.
                    //
                    // La position du POINTEUR, et non
                    // `gutter_resp.hover_pos()` : la gouttière vit
                    // dans une zone de défilement, laquelle s'inscrit
                    // elle-même comme sensible au glisser pour se
                    // faire tirer. Elle passe donc devant, et la
                    // gouttière n'est jamais « survolée » au sens
                    // d'egui — c'est pour cela que l'infobulle du
                    // nombre de passages ne paraissait pas non plus.
                    // Une bande de gouttière est une géométrie : on
                    // la traite comme telle.
                    let survolée = ui
                        .ctx()
                        .pointer_latest_pos()
                        .filter(|p| gutter_rect.contains(*p) && ui.clip_rect().contains(*p))
                        .and_then(|p| ligne_sous(&galley, gutter_rect, p));
                    // Les lignes à l'écran, trouvées par dichotomie.
                    //
                    // Peindre un numéro pour chacune des quinze mille
                    // lignes d'un binaire coûtait une frame entière,
                    // alors que trente sont à l'écran : on s'en tient
                    // depuis longtemps à ce que la zone montre. Mais on
                    // les ÉCARTAIT une par une, et c'était le vrai
                    // prix — neuf millisecondes par image sur un
                    // listing d'un million de lignes, pour en retenir
                    // soixante (mesuré le 14 octobre 2026), soit une
                    // image entière à soixante hertz dépensée à ne
                    // rien peindre.
                    //
                    // Le `intersects` plus bas reste : il ne coûte rien
                    // sur une soixantaine de rangées, et il garde la
                    // peinture juste si l'une venait à sortir de la
                    // tranche.
                    let visible_rect = ui.clip_rect();
                    let tranche =
                        rangées_visibles(&galley, gutter_rect.min.to_vec2(), visible_rect);
                    let première_visible = tranche.start as u32 + 1;
                    let dernière_visible = tranche.end as u32;
                    // Les occurrences de la recherche sont relevées une
                    // fois, et non à chaque image (voir
                    // `refresh_find_hits`).
                    self.refresh_find_hits();
                    let painter = ui.painter();

                    // Les occurrences de la recherche, peintes sous
                    // le texte : une barre qui compte « 3/7 » sans
                    // montrer où ils sont ne sert qu'à moitié.
                    //
                    // Celles qui sont à l'écran, et elles seules : les
                    // autres se peindraient hors de la vue, et les
                    // situer coûtait deux parcours complets du texte
                    // chacune (voir `text::Place`).
                    if let Some(find) = &self.find {
                        let vues =
                            places_entre(&self.find_hits, première_visible, dernière_visible);
                        for rang in vues {
                            let place = &self.find_hits[rang];
                            // Une occurrence à cheval sur deux lignes
                            // n'existe pas ici : une recherche ne
                            // contient pas de retour à la ligne.
                            let Some(rect) = rect_de_place(&galley, place) else {
                                continue;
                            };
                            let rect = rect.translate(
                                gutter_rect.min.to_vec2() + egui::vec2(gutter_rect.width(), 0.0),
                            );
                            let teinte = if rang == find.current {
                                theme.accent.gamma_multiply(0.45)
                            } else {
                                theme.accent.gamma_multiply(0.18)
                            };
                            painter.rect_filled(rect, 2.0, teinte);
                        }
                    }
                    for (rang, row) in galley.rows[tranche.clone()].iter().enumerate() {
                        let line = (tranche.start + rang + 1) as u32;
                        let row_rect = row.rect().translate(gutter_rect.min.to_vec2());
                        if !row_rect.intersects(visible_rect) {
                            continue;
                        }

                        if gutter_resp.clicked()
                            && let Some(y) = click_y
                            && row_rect.y_range().contains(y)
                        {
                            clicked_line = Some(line);
                        }

                        // La ligne où le processeur est arrêté, en
                        // premier : une bande pâle sur toute la
                        // largeur. La flèche de la gouttière et le
                        // numéro en couleur d'accent la disaient
                        // déjà, mais tous deux sont à GAUCHE — on
                        // suit du regard la colonne du code, et il
                        // fallait revenir en bord d'écran pour
                        // retrouver où l'on en était. Une bande se
                        // voit sans qu'on la cherche.
                        //
                        // Douze pour cent : assez pour distinguer la
                        // ligne d'un coup d'œil, assez peu pour que
                        // la coloration syntaxique se lise dessus
                        // sans rien perdre.
                        if current_line == Some(line) {
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    row_rect.min,
                                    egui::vec2(largeur_ligne, row_rect.height()),
                                ),
                                0.0,
                                theme.accent.gamma_multiply(0.12),
                            );
                        }
                        // Le bandeau des diagnostics ensuite, sous le
                        // reste.
                        if let Some(severity) = fautives.get(&line) {
                            let teinte = if *severity == Severity::Error {
                                theme.bad
                            } else {
                                theme.warn
                            };
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    row_rect.min,
                                    egui::vec2(largeur_ligne, row_rect.height()),
                                ),
                                0.0,
                                // Assez pour se voir, assez peu pour
                                // laisser lire le code par-dessus.
                                teinte.gamma_multiply(0.18),
                            );
                        }

                        let is_current = current_line == Some(line);
                        let point = egui::pos2(gutter_rect.left() + 8.0, row_rect.center().y);
                        if self.breakpoint_lines.contains(&line) {
                            painter.circle_filled(point, 4.0, theme.bad);
                        } else if survolée == Some(line) {
                            // Pâle, et au même endroit que le vrai :
                            // il montre ce qui va se passer, il ne
                            // prétend pas que c'est déjà fait.
                            painter.circle_filled(point, 4.0, theme.bad.gamma_multiply(0.38));
                        }
                        if is_current {
                            let arrow_rect = egui::Rect::from_center_size(
                                egui::pos2(gutter_rect.left() + 20.0, row_rect.center().y),
                                egui::vec2(10.0, 10.0),
                            );
                            icon_run(painter, arrow_rect, theme.accent);
                        }
                        let color = if is_current {
                            theme.accent
                        } else {
                            ui.visuals().weak_text_color()
                        };
                        painter.text(
                            egui::pos2(gutter_rect.right() - GUTTER_PAD, row_rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            line.to_string(),
                            font_id.clone(),
                            color,
                        );
                        // La chaleur de la ligne : une bande à
                        // l'extrême gauche, d'autant plus vive que la
                        // ligne a tourné souvent. Un nombre par ligne
                        // se lit une ligne à la fois ; une bande de
                        // couleur se lit d'un coup d'œil, et c'est
                        // ainsi qu'on repère la boucle qui coûte.
                        if let Some(count) = passages.get(&line) {
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(gutter_rect.left(), row_rect.top()),
                                    egui::vec2(HEAT_WIDTH, row_rect.height()),
                                ),
                                0.0,
                                heat_colour(theme, *count, plus_chaude),
                            );
                        }
                    }
                    // Combien de fois, au survol : la bande dit
                    // « souvent », le nombre dit « mille deux cent
                    // trois ». Quel que soit le mode de l'éditeur —
                    // on regarde un programme bien plus souvent
                    // qu'on ne le tape.
                    // La main sur la gouttière : le pointeur dit
                    // « ceci se clique » avant même qu'on ait lu quoi
                    // que ce soit.
                    if survolée.is_some() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if self.show_tooltips
                        && let Some(ligne) = survolée
                    {
                        // Le nombre de passages s'il existe — la
                        // bande dit « souvent », le nombre dit « mille
                        // deux cent trois ». Sinon, ce que le clic
                        // ferait : c'est la question qu'on se pose la
                        // première fois, et la seule fois qui compte.
                        let phrases = strings(self.lang);
                        let texte = match passages.get(&ligne) {
                            Some(count) if self.lang == Lang::Fr => {
                                format!("Ligne {ligne} : exécutée {count} fois")
                            }
                            Some(count) => format!("Line {ligne}: run {count} times"),
                            None if self.breakpoint_lines.contains(&ligne) => {
                                phrases.gutter_clear_breakpoint.to_owned()
                            }
                            None => phrases.gutter_set_breakpoint.to_owned(),
                        };
                        gutter_resp.clone().show_tooltip_text(texte);
                    }
                    if let Some(line) = clicked_line {
                        if self.breakpoint_lines.contains(&line) {
                            self.breakpoint_lines.remove(&line);
                        } else {
                            self.breakpoint_lines.insert(line);
                        }
                        self.sync_breakpoints();
                    }

                    // La largeur de repli que propose egui est
                    // délibérément ignorée : voir `layout_code`.
                    //
                    // LA MISE EN PAGE QUE LA GOUTTIÈRE VIENT DE FAIRE,
                    // PRÊTÉE AU CHAMP. Le cache de `layout_code`
                    // répondrait aussi, mais sa clé est le texte
                    // lui-même : sur un listing de seize mégaoctets,
                    // reconnaître le texte coûte une milliseconde et
                    // demie, et c'était payé DEUX fois par image —
                    // mesuré le 14 octobre 2026, c'était la moitié du
                    // temps d'une image.
                    //
                    // Le prêt n'est consenti qu'en mode Normal, où le
                    // champ n'est pas interactif : egui ne peut alors
                    // pas toucher au tampon, donc rien n'a pu changer
                    // entre les deux appels. L'adresse et la longueur
                    // du texte sont vérifiées quand même — c'est ce qui
                    // rend le prêt faux s'il cesse d'être vrai, au lieu
                    // de le rendre silencieusement périmé.
                    let prêt = (self.editor_mode == EditorMode::Normal).then(|| {
                        (
                            self.source.as_ptr() as usize,
                            self.source.len(),
                            galley.clone(),
                        )
                    });
                    let mut layouter =
                        |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
                            let texte = buf.as_str();
                            if let Some((adresse, longueur, galley)) = &prêt
                                && texte.as_ptr() as usize == *adresse
                                && texte.len() == *longueur
                            {
                                return galley.clone();
                            }
                            layout_code(ui, theme, texte)
                        };
                    // `.show()` plutôt que `ui.add()` : c'est la seule
                    // forme qui donne accès au curseur, dont la barre
                    // de statut affiche la position.
                    // La largeur demandée est celle du code, ou celle
                    // du panneau s'il est plus large — et non l'infini.
                    // Demander l'infini faisait croire à la zone de
                    // défilement qu'il y avait toujours quelque chose à
                    // atteindre à droite : sa barre horizontale restait
                    // affichée même sur trois lignes de vingt
                    // caractères.
                    let width = editor_width(galley.rect.width(), ui.available_width());
                    // Avant le `TextEdit` : les touches que la liste
                    // de propositions veut sont retirées de la file,
                    // sans quoi l'éditeur les appliquerait d'abord.
                    self.claim_completion_keys(ui);
                    let output = egui::TextEdit::multiline(&mut self.source)
                        .id(editor_id())
                        .frame(egui::Frame::NONE)
                        .desired_width(width)
                        .desired_rows(editor_rows(visible.y, row_height))
                        .code_editor()
                        .interactive(self.editor_mode == EditorMode::Insert)
                        .layouter(&mut layouter)
                        .show(ui);
                    // Les occurrences suivies, peintes *sous* le
                    // texte : par-dessus, même translucide, le fond
                    // ternirait les lettres — or c'est le mot qu'on
                    // cherche à lire, pas le rectangle.
                    self.paint_matches(ui, &output);
                    // Les commentaires qu'on a écrits, à droite de
                    // leur ligne : c'est là qu'on les lit, et non
                    // dans un panneau à côté qu'il faudrait mettre
                    // en face du code de mémoire.
                    self.paint_notes(ui, &output);
                    // En mode Normal, le `TextEdit` n'est pas
                    // interactif : egui ne peint aucun caret et ne
                    // rend plus de `cursor_range`. La position du
                    // curseur n'était donc visible nulle part — ni
                    // dans le code, ni dans la barre de statut, qui
                    // restait figée sur « Lig 1, Col 1 ». On la lit
                    // dans l'état conservé du champ, et on la peint
                    // soi-même : un bloc plein, comme vim, là où
                    // l'insertion reprendra.
                    if self.editor_mode == EditorMode::Normal
                        && let Some(state) = egui::TextEdit::load_state(ui.ctx(), editor_id())
                        && let Some(range) = state.cursor.char_range()
                    {
                        let primary = range.primary;
                        let (ligne, colonne, octet) = self.cursor_place(primary.index.0);
                        self.cursor_pos = (ligne, colonne);
                        self.cursor_byte = octet;
                        // La rangée EST la ligne — le listing ne se
                        // replie pas —, donc la position se prend
                        // directement. `pos_from_cursor` répondrait
                        // aussi, en comptant les caractères de toutes
                        // les rangées depuis la première : un parcours
                        // d'un million de rangées par image.
                        let place = output.galley.pos_from_layout_cursor(
                            &egui::epaint::text::cursor::LayoutCursor {
                                row: ligne.saturating_sub(1),
                                column: egui::text::CharIndex(colonne.saturating_sub(1)),
                            },
                        );
                        // Monospace : la largeur d'une espace est
                        // celle de n'importe quel caractère, et vaut
                        // mieux que de mesurer le glyphe visé — il
                        // n'y en a aucun en fin de ligne.
                        let largeur = ui.ctx().fonts_mut(|f| f.glyph_width(&font_id, ' '));
                        let bloc = egui::Rect::from_min_size(
                            output.galley_pos + place.left_top().to_vec2(),
                            egui::vec2(largeur, place.height()),
                        );
                        // Translucide : le caractère sous le bloc
                        // doit rester lisible. L'inversion de vim
                        // demanderait de repeindre le glyphe.
                        ui.painter()
                            .rect_filled(bloc, 1.0, self.theme.accent.gamma_multiply(0.45));
                    }
                    if let Some(range) = output.cursor_range {
                        let (ligne, colonne, octet) = self.cursor_place(range.primary.index.0);
                        self.cursor_pos = (ligne, colonne);
                        self.cursor_byte = octet;
                        // La sélection, pour « surligner les
                        // occurrences » : elle l'emporte sur le
                        // curseur quand il y en a une.
                        let (a, b) = (range.primary.index.0, range.secondary.index.0);
                        self.editor_selection = (a != b).then(|| {
                            char_to_byte(&self.source, a.min(b))
                                ..char_to_byte(&self.source, a.max(b))
                        });
                        // Entrée garde l'indentation de la ligne
                        // qu'on quitte. egui insère un `\n` nu ; on
                        // ajoute les blancs derrière, et l'on avance
                        // le curseur d'autant.
                        // Les paires : posées après coup, comme
                        // l'indentation, parce que le `TextEdit` a
                        // déjà écrit ce qu'on vient de taper et que
                        // c'est de là qu'on raisonne.
                        if self.auto_pairs
                            && output.response.response.has_focus()
                            && range.is_empty()
                        {
                            let at = self.cursor_byte;
                            match pair_edit(&self.source, at) {
                                PairEdit::Close(fermant) => {
                                    self.source.insert(at, fermant);
                                    // Le curseur ne bouge pas : il
                                    // reste entre les deux, qui est
                                    // le seul endroit utile.
                                    set_cursor_byte(ui.ctx(), &self.source, at);
                                }
                                PairEdit::StepOver => {
                                    // Retirer le fermant qu'on vient
                                    // d'écrire revient à traverser
                                    // celui qui était déjà là.
                                    let début = at
                                        - self.source[..at]
                                            .chars()
                                            .next_back()
                                            .map_or(0, char::len_utf8);
                                    self.source.replace_range(début..at, "");
                                    set_cursor_byte(ui.ctx(), &self.source, at);
                                }
                                PairEdit::None => {}
                            }
                        }
                        // Le retour arrière emporte la paire vide
                        // qu'il rouvrirait sans cela.
                        if self.auto_pairs
                            && output.response.response.has_focus()
                            && range.is_empty()
                            && ui.input(|i| i.key_pressed(egui::Key::Backspace))
                            && backspace_clears_pair(&self.source, self.cursor_byte)
                        {
                            let at = self.cursor_byte;
                            let fin =
                                at + self.source[at..].chars().next().map_or(0, char::len_utf8);
                            self.source.replace_range(at..fin, "");
                        }
                        if self.auto_indent
                            && output.response.response.has_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && range.is_empty()
                        {
                            let at = self.cursor_byte;
                            // Le `\n` vient d'être posé : l'indentation
                            // se lit sur la ligne d'avant.
                            let indent = newline_indent(
                                &self.source[..at.saturating_sub(1)],
                                at.saturating_sub(1),
                            );
                            let blancs = indent.trim_start_matches('\n');
                            if !blancs.is_empty() && at <= self.source.len() {
                                self.source.insert_str(at, blancs);
                                let cible = byte_to_char(&self.source, at + blancs.len());
                                if let Some(mut state) =
                                    egui::TextEdit::load_state(ui.ctx(), editor_id())
                                {
                                    state.cursor.set_char_range(Some(
                                        egui::text::CCursorRange::one(egui::text::CCursor::new(
                                            cible,
                                        )),
                                    ));
                                    state.store(ui.ctx(), editor_id());
                                }
                            }
                        }
                    }
                    self.completion_ui(ui, &output);
                    self.apply_fix_if_confirmed();
                    self.hover_help_ui(ui, &output);
                    // Après le `TextEdit` : son état existe, et c'est
                    // lui qu'on déplace.
                    self.apply_goto_line(&ui.ctx().clone());
                    self.apply_pending_cursor(&ui.ctx().clone());
                    let response = output.response.response;
                    if just_entered_insert {
                        response.request_focus();
                    }
                    if response.clicked() {
                        // Cliquer dans le code reprend le focus clavier
                        // à l'explorateur.
                        self.explorer_focused = false;
                    }
                    // Conditionne l'armement de la touche leader
                    // (Espace, ...) : sinon "Espace" taperait un
                    // espace *et* armerait la séquence à chaque mot
                    // pendant la frappe.
                    self.editor_focused = response.has_focus();
                });
            });
        });
    }

    /// L'avertissement d'un fichier changé sous nos pieds.
    ///
    /// Non bloquant : une fenêtre modale au retour d'un `git checkout`
    /// interromprait pour une question qui n'est pas urgente. Deux
    /// boutons suffisent — relire le disque, ou garder ce qu'on a écrit
    /// et l'enregistrer quand on voudra.
    pub(crate) fn disk_changed_ui(&mut self, ui: &mut egui::Ui) {
        if !self.disk_changed {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        egui::Frame::new()
            .fill(theme.warn.gamma_multiply(0.12))
            .inner_margin(egui::Margin::symmetric(8, 5))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme.warn, s.disk_changed);
                    if framed_button(ui, theme, s.disk_reload).clicked() {
                        self.reload_from_disk();
                    }
                    if framed_button(ui, theme, s.disk_keep).clicked() {
                        // Garder, c'est accepter la version du disque
                        // comme point de comparaison : sans cela,
                        // l'avertissement reviendrait à chaque regard.
                        self.file_mtime = self
                            .current_file
                            .as_deref()
                            .and_then(file_mtime)
                            .or(self.file_mtime);
                        self.disk_changed = false;
                    }
                });
            });
        ui.add_space(2.0);
    }

    /// La barre des fichiers ouverts.
    ///
    /// Un onglet par fichier, le nom raccourci quand il est long — mais
    /// par le milieu, jamais par la fin : `long_nom_de_prog…` et
    /// `long_nom_de_prog2…` se ressemblent trop, alors que
    /// `long_n…rog.asm` et `long_n…og2.asm` se distinguent. Le point dit
    /// ce qui n'est pas enregistré, et la croix ferme.
    ///
    /// Un seul fichier ouvert : pas de barre. Elle n'apprend rien tant
    /// qu'il n'y a rien à choisir, et l'éditeur gagne sa hauteur.
    /// La barre d'onglets : une vraie barre, et non des pastilles.
    ///
    /// Elle était faite d'un `selectable_label` arrondi suivi d'une
    /// croix détachée — trois objets qui flottaient, sans lien visible
    /// entre eux ni avec l'éditeur. Un onglet, c'est autre chose : un
    /// rectangle franc, collé à ce qu'il montre, dont le bord supérieur
    /// dit lequel est devant. C'est ce que fait tout éditeur, et ce que
    /// l'œil sait lire sans qu'on le lui explique.
    ///
    /// Dessinée à la main plutôt que composée de widgets : il faut un
    /// fond qui rejoint celui de l'éditeur, un liseré d'accent sur deux
    /// pixels, et une croix à l'intérieur du rectangle. Aucun widget
    /// d'egui ne donne les trois.
    pub(crate) fn tabs_bar_ui(&mut self, ui: &mut egui::Ui) {
        // TOUJOURS, y compris sur un tampon « sans titre ».
        //
        // Elle n'apparaissait qu'avec DEUX fichiers ouverts : rien,
        // jamais, ne disait qu'on pouvait en ouvrir plusieurs. Premier
        // remède, insuffisant : la montrer dès le premier fichier
        // OUVERT. Or l'écran d'accueil n'en a pas — il porte un tampon
        // « sans titre » —, si bien que la barre restait invisible
        // exactement là où elle enseigne quelque chose.
        let theme = self.theme;
        let s = strings(self.lang);
        let mut aller = None;
        let mut fermer = None;
        let mut demande = None;
        let ctx_raccourcis = ui.ctx().clone();
        let neuf_infobulle = format!(
            "{} — {}",
            shortcut_hint(&self.keybindings, &ctx_raccourcis, s.file_new, Action::New),
            s.tab_open_another
        );

        const HAUTEUR: f32 = 30.0;
        const MARGE: f32 = 12.0;
        const CROIX: f32 = 16.0;
        // Le fond de la barre : celui des panneaux, plus sombre que
        // l'éditeur. C'est le contraste entre les deux qui fait que
        // l'onglet actif paraît posé PAR-DESSUS l'éditeur.
        let fond_barre = theme.panel;
        let fond_actif = ui.visuals().extreme_bg_color;
        let police = egui::TextStyle::Body.resolve(ui.style());

        let étiquettes = self.tab_labels();
        let actif = self.active_tab_index;
        egui::ScrollArea::horizontal()
            .id_salt("tabs_bar")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (rang, (nom, modifié)) in étiquettes.into_iter().enumerate() {
                        let court = raccourcir_nom(&nom, 18);
                        let galette = ui.painter().layout_no_wrap(
                            court.clone(),
                            police.clone(),
                            egui::Color32::PLACEHOLDER,
                        );
                        let largeur = MARGE + galette.size().x + 6.0 + CROIX + MARGE * 0.5;
                        let (rect, réponse) = ui.allocate_exact_size(
                            egui::vec2(largeur, HAUTEUR),
                            egui::Sense::click(),
                        );
                        let ici = rang == actif;
                        let peintre = ui.painter();
                        peintre.rect_filled(
                            rect,
                            0.0,
                            if ici {
                                fond_actif
                            } else if réponse.hovered() {
                                mélange(fond_barre, fond_actif, 0.5)
                            } else {
                                fond_barre
                            },
                        );
                        if ici {
                            // Le liseré du dessus : c'est lui qui dit
                            // « celui-ci », plus vite qu'une nuance de
                            // fond.
                            peintre.rect_filled(
                                egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0)),
                                0.0,
                                theme.accent,
                            );
                        }
                        peintre.galley(
                            egui::pos2(
                                rect.left() + MARGE,
                                rect.center().y - galette.size().y / 2.0,
                            ),
                            galette,
                            if ici { theme.text } else { theme.dim },
                        );
                        // La croix, dans le rectangle : c'est l'onglet
                        // qu'on ferme, et non un bouton qui traîne à
                        // côté.
                        let croix = egui::Rect::from_center_size(
                            egui::pos2(rect.right() - MARGE * 0.5 - CROIX / 2.0, rect.center().y),
                            egui::vec2(CROIX, CROIX),
                        );
                        let sur_croix = ui.interact(
                            croix,
                            egui::Id::new(("tab_close", rang)),
                            egui::Sense::click(),
                        );
                        let peintre = ui.painter();
                        if modifié && !sur_croix.hovered() {
                            // Le point du non-enregistré prend la place
                            // de la croix : les deux au même endroit ne
                            // tiendraient pas, et c'est ce que font les
                            // éditeurs — le point devient croix quand on
                            // s'en approche.
                            peintre.circle_filled(croix.center(), 3.5, theme.warn);
                        } else {
                            if sur_croix.hovered() {
                                peintre.rect_filled(
                                    croix,
                                    3.0,
                                    mélange(fond_actif, theme.text, 0.18),
                                );
                            }
                            let trait_ = egui::Stroke::new(
                                1.4,
                                if sur_croix.hovered() {
                                    theme.text
                                } else {
                                    theme.dim
                                },
                            );
                            let c = croix.center();
                            let d = 4.0;
                            peintre.line_segment(
                                [egui::pos2(c.x - d, c.y - d), egui::pos2(c.x + d, c.y + d)],
                                trait_,
                            );
                            peintre.line_segment(
                                [egui::pos2(c.x - d, c.y + d), egui::pos2(c.x + d, c.y - d)],
                                trait_,
                            );
                        }
                        if sur_croix.on_hover_text(s.tab_close_hint).clicked() {
                            fermer = Some(rang);
                        } else if réponse.on_hover_text(&nom).clicked() {
                            aller = Some(rang);
                        }
                    }
                    // Le « + », à la même hauteur que les onglets : un
                    // programme neuf, et rien d'autre.
                    //
                    // Il a d'abord mené au sélecteur de fichiers, puis à
                    // un menu de deux entrées. Les deux fois c'était
                    // trop : dans une barre d'onglets, « + » veut dire
                    // « un de plus », un point c'est tout. Ouvrir un
                    // fichier est un autre geste, et il a sa place dans
                    // le menu Fichier.
                    ui.add_space(4.0);
                    let (plus, réponse) =
                        ui.allocate_exact_size(egui::vec2(HAUTEUR, HAUTEUR), egui::Sense::click());
                    let peintre = ui.painter();
                    if réponse.hovered() {
                        peintre.rect_filled(
                            plus.shrink(5.0),
                            3.0,
                            mélange(fond_barre, fond_actif, 0.6),
                        );
                    }
                    let trait_plus = egui::Stroke::new(
                        1.5,
                        if réponse.hovered() {
                            theme.text
                        } else {
                            theme.dim
                        },
                    );
                    let c = plus.center();
                    let d = 5.0;
                    peintre.line_segment(
                        [egui::pos2(c.x - d, c.y), egui::pos2(c.x + d, c.y)],
                        trait_plus,
                    );
                    peintre.line_segment(
                        [egui::pos2(c.x, c.y - d), egui::pos2(c.x, c.y + d)],
                        trait_plus,
                    );
                    if réponse.on_hover_text(&neuf_infobulle).clicked() {
                        demande = Some(Action::New);
                    }
                });
            });
        if let Some(action) = demande {
            self.dispatch(action);
        }
        if let Some(rang) = aller {
            self.go_to_tab(rang);
        }
        if let Some(rang) = fermer {
            self.close_tab(rang);
        }
    }

    /// Où le curseur est : ligne, colonne et octet — relevés une fois.
    ///
    /// Les trois demandaient chacun un parcours du texte depuis le
    /// début, deux fois par image (le caret peint, puis la barre
    /// d'état). Sur un listing de seize mégaoctets, c'est une
    /// quarantaine de millisecondes par image pour une position qui, le
    /// plus souvent, n'a pas bougé d'un caractère.
    ///
    /// La clé est le rang du caractère, la longueur du texte et le
    /// nombre de poses du listing : le texte ne peut pas avoir changé
    /// sans qu'il ait été reposé (voir `poses_du_texte`).
    pub(crate) fn cursor_place(&mut self, char_index: usize) -> (usize, usize, usize) {
        let clé = (char_index, self.source.len(), poses_du_texte());
        if let Some(vu) = &self.cursor_place_vu
            && vu.clé == clé
        {
            return vu.place;
        }
        let place = line_col_byte(&self.source, char_index);
        self.cursor_place_vu = Some(CurseurVu { place, clé });
        place
    }

    /// Relève les occurrences de la recherche — et seulement quand il y
    /// a lieu.
    ///
    /// Elles étaient recomptées DEUX fois par image, une fois par la
    /// barre et une fois par la peinture, et chaque relevé recopiait
    /// tout le texte en minuscules. Puis chaque occurrence était située
    /// par deux parcours complets (voir `text::Place`). Mesuré le 14
    /// octobre 2026 sur crackme-01 — 62 386 lignes, 24 081 fois « mov » :
    /// 3 735 ms par image, barre ouverte, contre 0,8 ms fermée. Sur un
    /// binaire de quatre mégaoctets, l'image ne venait plus du tout —
    /// Ctrl+F dans un binaire gelait donc namsdec, et c'est exactement
    /// le geste qu'on fait en premier dans un listing qu'on ne connaît
    /// pas.
    ///
    /// La clé dit ce dont le relevé dépend : la requête, la longueur du
    /// texte, et le nombre de fois que le listing a été POSÉ — lequel
    /// augmente quand le texte change, puisque c'est le texte lui-même
    /// qui sert de clé au cache de mise en page (voir
    /// `editor::poses_du_texte`). Une comparaison de plus sur seize
    /// mégaoctets à chaque image n'apprendrait rien de mieux.
    pub(crate) fn refresh_find_hits(&mut self) {
        let Some(find) = &self.find else {
            self.find_hits.clear();
            self.find_hits_clé = None;
            return;
        };
        let clé = (find.query.clone(), self.source.len(), poses_du_texte());
        if self.find_hits_clé.as_ref() == Some(&clé) {
            return;
        }
        self.find_hits = find_places(&self.source, &find.query);
        self.find_hits_clé = Some(clé);
        RELEVÉS_DE_RECHERCHE.with(|n| n.set(n.get() + 1));
    }

    /// La barre de recherche : ce qu'on cherche, combien de fois on le
    /// trouve, et de quoi passer d'une occurrence à l'autre.
    pub(crate) fn find_ui(&mut self, ui: &mut egui::Ui) {
        if self.find.is_none() {
            return;
        }
        // Le compte vient du relevé gardé d'une image à l'autre : le
        // refaire ici recopiait tout le texte en minuscules à chaque
        // image, barre ouverte (voir `refresh_find_hits`).
        self.refresh_find_hits();
        let combien = self.find_hits.len();
        let Some(find) = &mut self.find else {
            return;
        };
        let s = strings(self.lang);
        let theme = self.theme;
        let mut query = std::mem::take(&mut find.query);
        let mut current = find.current;
        let just_opened = std::mem::take(&mut find.just_opened);
        let mut fermer = false;
        let mut aller = just_opened && !query.is_empty();

        if combien == 0 {
            current = 0;
        } else {
            current %= combien;
        }

        ui.horizontal(|ui| {
            let champ = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .id(find_field_id())
                    .hint_text(s.find_placeholder)
                    .desired_width(220.0),
            );
            if just_opened {
                champ.request_focus();
            }
            // Entrée avance, Maj+Entrée recule : les deux gestes d'une
            // barre de recherche, sans quitter le clavier.
            if champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let arrière = ui.input(|i| i.modifiers.shift);
                if combien > 0 {
                    current = if arrière {
                        (current + combien - 1) % combien
                    } else {
                        (current + 1) % combien
                    };
                }
                aller = true;
                champ.request_focus();
            }
            if framed_button(ui, theme, "<")
                .on_hover_text(s.find_previous)
                .clicked()
                && combien > 0
            {
                current = (current + combien - 1) % combien;
                aller = true;
            }
            if framed_button(ui, theme, ">")
                .on_hover_text(s.find_next)
                .clicked()
                && combien > 0
            {
                current = (current + 1) % combien;
                aller = true;
            }
            if combien == 0 {
                ui.colored_label(
                    if query.is_empty() {
                        theme.dim
                    } else {
                        theme.bad
                    },
                    if query.is_empty() {
                        String::new()
                    } else {
                        s.find_none.to_owned()
                    },
                );
            } else {
                ui.colored_label(theme.dim, format!("{}/{}", current + 1, combien));
            }
            if framed_button(ui, theme, "x").clicked()
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                fermer = true;
            }
        });

        if fermer {
            self.find = None;
            return;
        }
        if let Some(find) = &mut self.find {
            find.query = query;
            find.current = current;
        }
        if aller {
            self.goto_match();
        }
    }

    /// La barre de renommage, quand elle est ouverte.
    pub(crate) fn rename_ui(&mut self, ui: &mut egui::Ui) {
        let Some(mut nouveau) = self.rename.clone() else {
            return;
        };
        let s = strings(self.lang);
        let theme = self.theme;
        let just_opened = std::mem::take(&mut self.rename_just_opened);
        let combien = self.tracked_spans.len();
        let mut appliquer = false;
        let mut annuler = false;

        ui.horizontal(|ui| {
            ui.colored_label(theme.good, format!("{combien} ×"));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut nouveau)
                    .id(egui::Id::new("rename_field"))
                    .hint_text(s.rename_placeholder)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(220.0),
            );
            if just_opened {
                champ.request_focus();
                // Tout sélectionné : on remplace le nom, on ne le
                // complète pas.
                if let Some(mut état) =
                    egui::TextEdit::load_state(ui.ctx(), egui::Id::new("rename_field"))
                {
                    état
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(nouveau.chars().count()),
                        )));
                    état.store(ui.ctx(), egui::Id::new("rename_field"));
                }
            }
            if champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                appliquer = true;
            }
            if framed_button(ui, theme, s.rename_apply).clicked() {
                appliquer = true;
            }
            if framed_button(ui, theme, "x").clicked()
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                annuler = true;
            }
        });

        if appliquer {
            self.apply_rename(&nouveau);
        } else if annuler {
            self.rename = None;
        } else {
            self.rename = Some(nouveau);
        }
    }

    /// La barre de saisie d'une note, quand elle est ouverte.
    pub(crate) fn note_ui(&mut self, ui: &mut egui::Ui) {
        let Some((genre, adresse, mut texte)) = self.note_edit.clone() else {
            return;
        };
        let s = strings(self.lang);
        let theme = self.theme;
        let just_opened = std::mem::take(&mut self.note_just_opened);
        let mut appliquer = false;
        let mut annuler = false;

        ui.horizontal(|ui| {
            ui.colored_label(
                theme.accent,
                egui::RichText::new(format!("{adresse:#x}")).monospace(),
            );
            ui.colored_label(
                theme.dim,
                match genre {
                    NoteKind::Name => s.action_name_address,
                    NoteKind::Comment => s.action_comment_address,
                },
            );
            // Relevées avant le champ : un `TextEdit` qui traite Entrée
            // la retire de la file, et la guetter après lui ne verrait
            // rien. Même piège que partout ailleurs.
            let entrée = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let échap = ui.input(|i| i.key_pressed(egui::Key::Escape));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut texte)
                    .id(egui::Id::new("note_field"))
                    .hint_text(match genre {
                        NoteKind::Name => s.notes_name_placeholder,
                        NoteKind::Comment => s.notes_comment_placeholder,
                    })
                    .font(egui::TextStyle::Monospace)
                    .desired_width(ui.available_width() - 120.0),
            );
            if just_opened {
                champ.request_focus();
            }
            if entrée && (champ.has_focus() || champ.lost_focus()) {
                appliquer = true;
            }
            if échap {
                annuler = true;
            }
            if framed_button(ui, theme, s.dialog_ok).clicked() {
                appliquer = true;
            }
        });

        if appliquer {
            let vide = texte.trim().is_empty();
            match genre {
                NoteKind::Name => self.notes.set_name(adresse, &texte),
                NoteKind::Comment => self.notes.set_comment(adresse, &texte),
            }
            self.save_notes();
            self.note_edit = None;
            self.status = if vide {
                Status::Note(|s| s.notes_cleared)
            } else {
                match genre {
                    NoteKind::Name => Status::Note(|s| s.notes_named),
                    NoteKind::Comment => Status::Note(|s| s.notes_commented),
                }
            };
        } else if annuler {
            self.note_edit = None;
        } else {
            self.note_edit = Some((genre, adresse, texte));
        }
    }
}

pub(crate) fn editor_id() -> egui::Id {
    egui::Id::new("editeur")
}

/// Le mot qui entoure `byte` dans `text`.
///
/// « Mot » au sens de l'assembleur : lettres, chiffres, et le souligné et
/// le point qui font partie des noms (`_start`, `.boucle`). Le curseur
/// posé juste après un mot compte comme dedans — c'est là qu'il se trouve
/// quand on vient de le taper.
pub(crate) fn word_at(text: &str, byte: usize) -> &str {
    pub(crate) fn is_word(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '.'
    }
    let byte = byte.min(text.len());
    // `$` et `$$` ne sont pas des mots au sens des lettres, mais ce sont
    // bien des choses sur lesquelles on demande de l'aide : l'adresse
    // courante et le début de la section.
    if let Some(dollars) = dollars_at(text, byte) {
        return dollars;
    }
    // Reculer jusqu'au début du mot, avancer jusqu'à sa fin.
    let start = text[..byte]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word(*c))
        .map(|(i, _)| i)
        .last()
        .unwrap_or(byte);
    let end = byte
        + text[byte..]
            .char_indices()
            .take_while(|(_, c)| is_word(*c))
            .map(|(i, c)| i + c.len_utf8())
            .last()
            .unwrap_or(0);
    // Un `%` devant fait partie du nom : `%macro`, `%define`. Le
    // préprocesseur est un langage à part entière, et c'est là qu'on
    // demande ce qu'il fait.
    let start = if text[..start].ends_with('%') {
        start - 1
    } else {
        start
    };
    // Un point *devant* fait partie du nom (`.boucle`) ; un point
    // derrière est de la ponctuation — « … avant idiv. » dans un
    // commentaire ne demande pas d'aide sur « idiv. ».
    text[start..end].trim_end_matches('.')
}

/// La largeur que l'éditeur réclame : celle du code, ou celle de la place
/// disponible s'il est plus étroit.
///
/// Surtout pas l'infini, qui était demandé jusqu'ici : la zone de
/// défilement croyait alors toujours avoir quelque chose à atteindre à
/// droite, et gardait sa barre horizontale sous trois lignes de vingt
/// caractères. Prendre au moins la place disponible garde en revanche le
/// clic actif sur toute la largeur, jusqu'au bord droit.
pub(crate) fn editor_width(code: f32, available: f32) -> f32 {
    code.max(available)
}

/// Le chiffre qu'une touche porte, s'il s'agit d'un chiffre.
///
/// Les touches du pavé numérique comptent autant que celles du haut du
/// clavier : taper `5j` d'une main sur le pavé est exactement le même
/// geste.
fn chiffre_de(key: egui::Key) -> Option<u32> {
    match key {
        egui::Key::Num0 => Some(0),
        egui::Key::Num1 => Some(1),
        egui::Key::Num2 => Some(2),
        egui::Key::Num3 => Some(3),
        egui::Key::Num4 => Some(4),
        egui::Key::Num5 => Some(5),
        egui::Key::Num6 => Some(6),
        egui::Key::Num7 => Some(7),
        egui::Key::Num8 => Some(8),
        egui::Key::Num9 => Some(9),
        _ => None,
    }
}

/// Le compteur, un chiffre de plus — borné.
///
/// Quatre chiffres : `9999j` traverse n'importe quel listing, et au-delà
/// le nombre ne veut plus rien dire. La borne n'est pas qu'une prudence
/// de calcul : sans elle, un doigt resté sur une touche accumulerait un
/// compteur de dix chiffres qui déborderait son entier, et le mouvement
/// partirait on ne sait où.
pub(crate) fn borne_du_compte(compte: Option<u32>, chiffre: u32) -> u32 {
    (compte.unwrap_or(0) * 10 + chiffre).min(9999)
}

/// La mise en page du code, telle que la gouttière **et** l'éditeur la
/// voient — c'est le fait qu'ils partagent celle-ci qui les garde alignés.
///
/// La largeur de repli est volontairement infinie : dans un éditeur de
/// code, une ligne trop longue part sur la droite (la zone défile dans les
/// deux sens) plutôt que de se replier. Une ligne repliée compterait
/// plusieurs lignes rendues pour un seul numéro, et la règle de gauche —
/// un numéro par ligne rendue — se décalait d'autant : c'est ce qui
/// arrivait dès que l'éditeur devenait étroit.
pub(crate) fn layout_code(
    ui: &egui::Ui,
    theme: &'static Theme,
    text: &str,
) -> std::sync::Arc<egui::Galley> {
    // La coloration d'un binaire désassemblé — quinze mille lignes —
    // prend une quarantaine de millisecondes, et la pose du texte six
    // fois plus. C'est demandé au moins deux fois par frame : une fois
    // pour la gouttière, une fois par le champ de texte. Tant que ni le
    // texte, ni le thème, ni la taille de la police ne changent, le
    // résultat est le même : on le garde.
    //
    // LE TEXTE LUI-MÊME SERT DE CLÉ, et non son condensé. Hacher six
    // cent mille octets prend 1,2 ms — deux fois par frame, soit un
    // huitième d'un budget de seize millisecondes, dépensé à ne rien
    // apprendre. Les comparer prend 11 µs, cent treize fois moins, et
    // répond mieux : une égalité exacte ne peut pas collisionner.
    //
    // Le prix est une copie du texte à côté du galley. Elle est
    // négligeable devant lui : quinze mille lignes de glyphes pèsent
    // bien plus que les octets qui les ont produites.
    thread_local! {
        static CACHE: std::cell::RefCell<Option<Cache>> =
            const { std::cell::RefCell::new(None) };
    }

    RECONNAISSANCES.with(|n| n.set(n.get() + 1));
    // Le zoom change la taille de la police sans toucher au texte.
    let taille = egui::TextStyle::Monospace.resolve(ui.style()).size;
    let thème = theme as *const Theme as usize;

    if let Some(galley) = CACHE.with(|cache| {
        cache
            .borrow()
            .as_ref()
            .filter(|gardé| gardé.thème == thème && gardé.taille == taille && gardé.texte == text)
            .map(|gardé| gardé.galley.clone())
    }) {
        return galley;
    }

    let job = syntax::highlight(ui, theme, text, f32::INFINITY);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    POSES.with(|n| n.set(n.get() + 1));
    CACHE.with(|cache| {
        *cache.borrow_mut() = Some(Cache {
            texte: text.to_owned(),
            thème,
            taille,
            galley: galley.clone(),
        })
    });
    galley
}

// Combien de fois on a demandé au cache de RECONNAÎTRE le texte.
//
// Reconnaître, c'est le comparer en entier : une milliseconde et demie
// sur un listing de seize mégaoctets. La gouttière le demande, puis le
// champ de texte — deux fois par image, pour la même réponse. Le
// compter est ce qui permet à un contrôle de retenir le prêt d'une
// image à l'autre (voir `editor_ui`, « LA MISE EN PAGE QUE LA
// GOUTTIÈRE VIENT DE FAIRE »).
thread_local! {
    static RECONNAISSANCES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Combien de fois le cache de mise en page a eu à reconnaître le texte.
#[cfg(test)]
pub(crate) fn reconnaissances_du_texte() -> u64 {
    RECONNAISSANCES.with(std::cell::Cell::get)
}

// Combien de fois les occurrences de la recherche ont été relevées.
//
// Un relevé parcourt tout le texte : c'est le prix juste quand la
// requête change, et un gel quand il revient à chaque image. Le compter
// est le seul moyen qu'un contrôle le voie — mesurer le temps d'une
// image dirait « lent » sur une machine chargée et « vite » sur une
// autre, alors que le nombre de relevés est le même partout.
thread_local! {
    static RELEVÉS_DE_RECHERCHE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Combien de fois les occurrences de la recherche ont été relevées.
#[cfg(test)]
pub(crate) fn relevés_de_recherche() -> u64 {
    RELEVÉS_DE_RECHERCHE.with(std::cell::Cell::get)
}

// Combien de fois le texte a été posé pour de bon.
//
// La mise en page complète d'un listing coûte trois cents
// millisecondes, une fois. La frame qui les paie n'est pas forcément
// celle où le texte a changé : un fichier choisi dans un dialogue
// arrive parfois après le dessin de l'éditeur, et c'est la frame
// SUIVANTE qui pose. Compter les poses dit exactement laquelle a payé,
// là où comparer des longueurs d'une frame à l'autre le devinait — et
// se trompait précisément dans ce cas-là, en criant « frame lente »
// pour un chargement.
thread_local! {
    // Un `//` et non un `///` : un commentaire de documentation ne peut
    // pas se poser sur une macro.
    static POSES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Combien de fois le texte a été posé depuis le début.
pub(crate) fn poses_du_texte() -> u64 {
    POSES.with(std::cell::Cell::get)
}

/// Ce que `layout_code` garde d'un appel à l'autre.
struct Cache {
    /// Le texte tel quel : c'est lui la clé, voir `layout_code`.
    texte: String,
    /// L'adresse du thème — ils sont statiques, donc elle les désigne.
    thème: usize,
    taille: f32,
    galley: std::sync::Arc<egui::Galley>,
}
