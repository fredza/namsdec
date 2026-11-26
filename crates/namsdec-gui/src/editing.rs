//! Les gestes d'édition, et les deux aides qui se posent par-dessus.
//!
//! Dupliquer une ligne, coller, tout remplacer — et la complétion et
//! l'aide au survol, qui ne modifient rien mais se dessinent dans le
//! texte et doivent donc connaître la même géométrie que lui.
//!
//! La complétion prend des touches au clavier avant l'éditeur ; c'est
//! pourquoi elle les RÉCLAME explicitement plutôt que de les intercepter
//! au vol — une touche prise en silence est une touche qu'on croit
//! cassée.

use super::*;

impl NamsdecApp {
    /// Recopie la ligne courante en dessous, et descend dessus.
    ///
    /// Sans effet sur un listing venu d'un binaire : celui-ci est
    /// reconstruit, et y insérer une ligne décalerait toutes les
    /// correspondances ligne <-> adresse dont dépendent la gouttière, les
    /// points d'arrêt et le suivi de RIP.
    pub(crate) fn duplicate_line(&mut self) {
        if self.buffer_came_from_a_binary() {
            return;
        }
        let (source, curseur) = duplicated_line(&self.source, self.cursor_byte);
        self.set_source(source);
        self.cursor_byte = curseur;
        self.pending_cursor = Some(curseur);
    }

    /// Le presse-papier, ou ce qui empêche de le lire.
    pub(crate) fn clipboard_text(&mut self) -> Option<String> {
        match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
            Ok(texte) => Some(texte),
            Err(e) => {
                self.status = Status::Error(if self.lang == Lang::Fr {
                    format!("presse-papiers inaccessible : {e}")
                } else {
                    format!("clipboard unavailable: {e}")
                });
                None
            }
        }
    }

    /// Colle le presse-papier **à l'endroit du curseur**, en remplaçant
    /// la sélection s'il y en a une.
    ///
    /// C'est ce que « Coller » veut dire, et ce que l'entrée du menu ne
    /// faisait pas : elle remplaçait tout le programme. On cherchait à
    /// recopier trois lignes d'un cours, on perdait son travail, et rien
    /// ne prévenait — pas même un intitulé qui l'aurait laissé deviner.
    ///
    /// Refusé sur un listing venu d'un binaire, comme la duplication de
    /// ligne : celui-ci est reconstruit, et y insérer du texte
    /// décalerait les correspondances ligne ↔ adresse dont dépendent la
    /// gouttière, les points d'arrêt et le suivi de RIP.
    pub(crate) fn paste_at_cursor(&mut self) {
        let Some(texte) = self.clipboard_text() else {
            return;
        };
        self.insert_at_cursor(&texte);
    }

    /// Insère ce texte à l'endroit du curseur, en remplaçant la
    /// sélection s'il y en a une.
    ///
    /// Séparé de la lecture du presse-papier, et c'est ce qui rend le
    /// geste vérifiable : un test qui passerait par le presse-papier
    /// écraserait celui de l'utilisateur, ce qui n'est pas plus
    /// acceptable que d'écrire dans ses réglages.
    pub(crate) fn insert_at_cursor(&mut self, texte: &str) {
        if self.buffer_came_from_a_binary() {
            return;
        }
        let étendue = match self.editor_selection.clone() {
            Some(étendue) if étendue.start != étendue.end => étendue,
            _ => self.cursor_byte..self.cursor_byte,
        };
        // Bornes sûres : la sélection et le curseur sont relevés au
        // rendu, et le texte a pu changer depuis.
        let début = étendue.start.min(self.source.len());
        let fin = étendue.end.clamp(début, self.source.len());
        if !self.source.is_char_boundary(début) || !self.source.is_char_boundary(fin) {
            return;
        }
        self.source_mut().replace_range(début..fin, texte);
        let après = début + texte.len();
        self.cursor_byte = après;
        self.pending_cursor = Some(après);
        self.editor_selection = None;
        self.status = Status::Idle;
    }

    /// Remplace **tout** le programme par le presse-papier.
    ///
    /// Séparé de « Coller », et nommé pour ce qu'il fait : c'est le
    /// geste de qui reprend un programme entier d'un cours ou d'un
    /// sujet. Il reste utile ; ce qui ne l'était pas, c'est qu'il se
    /// cache derrière le mot « Coller ».
    pub(crate) fn replace_all_from_clipboard(&mut self) {
        let Some(texte) = self.clipboard_text() else {
            return;
        };
        self.replace_all_with(&texte);
    }

    /// Remplace tout le programme par ce texte. Voir
    /// `replace_all_from_clipboard`, dont c'est la moitié vérifiable.
    pub(crate) fn replace_all_with(&mut self, texte: &str) {
        if self.buffer_came_from_a_binary() {
            return;
        }
        self.set_source(texte.to_owned());
        self.cursor_byte = 0;
        self.pending_cursor = Some(0);
        self.editor_selection = None;
        self.status = Status::Idle;
    }

    /// Calcule, si besoin, la correction proposée pour la ligne du
    /// curseur.
    ///
    /// Ce qui rend le calcul caduc : changer de ligne, changer le texte
    /// de cette ligne, changer la longueur du fichier — un `jmp` vers
    /// une étiquette qu'on vient d'écrire ailleurs doit cesser d'être
    /// signalé — ou changer de langue, puisque la phrase l'est.
    ///
    /// Une modification ailleurs dans le fichier qui n'en change pas la
    /// longueur passe à travers, et c'est assumé : renommer une
    /// étiquette sans changer le nombre de lettres laisse une
    /// proposition d'une frappe en retard, ce qui se corrige à la frappe
    /// suivante et ne coûte rien à personne.
    pub(crate) fn refresh_fix(&mut self) {
        let index = self.cursor_pos.0.saturating_sub(1);
        let clé = FixKey {
            index,
            // La ligne prise autour du curseur, et non comptée depuis le
            // début du fichier : la clé se construit à chaque image, et
            // la compter coûtait tout le texte qui précède.
            ligne: ligne_autour(&self.source, self.cursor_byte).to_owned(),
            taille: self.source.len(),
            lang: self.lang,
        };
        if self.fix.as_ref().is_some_and(|f| f.clé == clé) {
            return;
        }
        let correction = fixes::propose(&self.source, index, strings(self.lang));
        self.fix = Some(FixCache { clé, correction });
    }

    /// La correction proposée en ce moment, s'il y en a une.
    pub(crate) fn current_fix(&self) -> Option<&fixes::Correction> {
        self.fix.as_ref()?.correction.as_ref()
    }

    /// `Alt` seule vient-elle d'être enfoncée ?
    ///
    /// Relevé une fois par image, AVANT que l'éditeur se dessine : la
    /// liste de propositions l'emploie pour accepter, la correction de
    /// code pour confirmer, et les deux vues — le champ de texte et la
    /// vue ligne à ligne — doivent le voir pareil. Il vivait dans la
    /// liste de propositions, qui n'est dessinée qu'en Insertion : la
    /// correction annoncée dans la barre d'état ne pouvait alors plus
    /// être confirmée en mode Normal.
    ///
    /// Relevé avant tout retour anticipé : gardée enfoncée pendant
    /// qu'aucune liste n'est affichée, `Alt` serait lue comme une
    /// nouvelle frappe au moment où la liste revient.
    ///
    /// Et `Alt` *seule* : les raccourcis d'exécution sont `Ctrl+Alt+I` et
    /// `Ctrl+Alt+O`, qui accepteraient sinon la proposition au passage,
    /// en plus de faire leur travail.
    pub(crate) fn relever_alt(&mut self, ui: &egui::Ui) {
        let alt_alone = ui.input(|i| {
            let m = i.modifiers;
            m.alt && !m.ctrl && !m.shift && !m.command && !m.mac_cmd
        });
        self.alt_pressed = alt_alone && !self.alt_was_down;
        self.alt_was_down = alt_alone;
    }

    /// Alt vient d'être enfoncé et rien d'autre ne l'a pris : la
    /// correction s'applique.
    ///
    /// À appeler **après** la liste de propositions, qui a la priorité :
    /// elle est sous le curseur, on la regarde, et c'est elle qu'on
    /// croit confirmer. La correction, elle, est écrite dans la barre
    /// d'état — on la confirme quand plus rien ne la précède.
    pub(crate) fn apply_fix_if_confirmed(&mut self) {
        if !self.alt_pressed || self.suggestions.is_some() {
            return;
        }
        self.alt_pressed = false;
        let Some(cache) = self.fix.as_ref() else {
            return;
        };
        let Some(correction) = cache.correction.as_ref() else {
            return;
        };
        let index = cache.clé.index;
        let nouvelle = correction.ligne.clone();
        // Le curseur se pose au bout de la ligne réécrite : c'est là
        // qu'on était, et c'est de là qu'on repart.
        self.poser_la_ligne(index, &nouvelle);
    }

    /// Pose une ligne réécrite à la place de la ligne `index`.
    ///
    /// Le cœur de `apply_fix_if_confirmed`, sorti pour servir aussi à la
    /// liste des problèmes : celle-ci montre la correction à côté de
    /// l'erreur, et son bouton doit faire exactement ce que fait
    /// Alt+Entrée — même écriture, même curseur, même message.
    pub(crate) fn poser_la_ligne(&mut self, index: usize, nouvelle: &str) {
        let Some(début) = ligne_début(&self.source, index) else {
            return;
        };
        let fin = début
            + self.source[début..]
                .find('\n')
                .unwrap_or(self.source.len() - début);
        self.source_mut().replace_range(début..fin, nouvelle);
        self.pending_cursor = Some(byte_to_char(&self.source, début + nouvelle.len()));
        self.status = Status::Note(|s| s.fix_applied);
        // Le cache parle d'une ligne qui n'existe plus.
        self.fix = None;
    }

    /// Retire du clavier ce que la liste de propositions veut pour elle.
    ///
    /// À appeler **avant** le `TextEdit` : Tab, les flèches et Échap ont
    /// un sens dans l'éditeur, et celui-ci les appliquerait le premier.
    /// Ne prend rien quand aucune liste n'est ouverte — Tab indente alors
    /// comme d'habitude, et les flèches déplacent le curseur.
    pub(crate) fn claim_completion_keys(&mut self, ui: &egui::Ui) {
        self.completion_keys = CompletionKeys::default();
        if self.suggestions.is_none() || !self.completion {
            return;
        }
        // `consume_key` retire l'événement : ce que la liste prend,
        // l'éditeur ne le verra pas.
        ui.input_mut(|i| {
            let vide = egui::Modifiers::NONE;
            self.completion_keys = CompletionKeys {
                accept: i.consume_key(vide, egui::Key::Tab),
                down: i.consume_key(vide, egui::Key::ArrowDown),
                up: i.consume_key(vide, egui::Key::ArrowUp),
                dismiss: i.consume_key(vide, egui::Key::Escape),
            };
        });
    }

    pub(crate) fn completion_ui(
        &mut self,
        ui: &mut egui::Ui,
        output: &egui::text_edit::TextEditOutput,
    ) {
        let s = strings(self.lang);
        let theme = self.theme;
        // Ni en mode Normal (on ne tape pas), ni si le réglage l'a
        // éteinte.
        let alt_pressed = self.alt_pressed;

        let Some(range) = output.cursor_range else {
            self.suggestions = None;
            return;
        };
        if !self.completion || self.editor_mode != EditorMode::Insert {
            self.suggestions = None;
            return;
        }

        let touches = self.completion_keys;
        // Échap referme sans rien écrire. La liste revient au caractère
        // suivant : ce n'est pas « plus jamais », c'est « pas celle-ci ».
        if touches.dismiss {
            self.suggestions = None;
            self.completion_dismissed = true;
            return;
        }

        let cursor = range.primary.index.0;
        let byte = char_to_byte(&self.source, cursor);
        let avant = self.suggestions.clone();
        self.suggestions = completion::suggest(&self.source, byte);
        // Une liste qu'on vient d'écarter ne se rouvre qu'une fois le mot
        // changé : la rouvrir sur le même mot rendrait Échap inopérant.
        if self.completion_dismissed {
            if self.suggestions.as_ref().map(|s| &s.prefix) == avant.as_ref().map(|s| &s.prefix) {
                self.suggestions = None;
                return;
            }
            self.completion_dismissed = false;
        }
        // Le choix repart du haut dès que la liste change : il désignait
        // une proposition qui n'y est peut-être plus.
        if self.suggestions.as_ref().map(|s| &s.prefix) != avant.as_ref().map(|s| &s.prefix) {
            self.completion_index = 0;
        }
        let Some(suggestions) = self.suggestions.clone() else {
            return;
        };

        // Les flèches choisissent. Sans elles, on ne pouvait accepter que
        // la première proposition, et la liste ne servait qu'à regarder.
        if touches.down || touches.up {
            self.completion_index = completion::next_index(
                self.completion_index,
                suggestions.candidates.len(),
                touches.down,
            );
        }

        // Tab accepte, comme partout ailleurs, et `Alt` aussi — celle-ci
        // était le seul moyen jusqu'ici, ce qui ne se devinait pas.
        if alt_pressed || touches.accept {
            // Pris : la correction de code ne verra pas cette frappe.
            self.alt_pressed = false;
            self.accept_completion(ui.ctx(), &suggestions);
            return;
        }

        // Sous le curseur, comme une bulle : la liste suit la frappe.
        let mut cliqué: Option<usize> = None;
        let cursor_rect = output.galley.pos_from_cursor(range.primary);
        let at = output.galley_pos + cursor_rect.left_bottom().to_vec2();
        egui::Area::new(egui::Id::unique("completion"))
            .order(egui::Order::Foreground)
            .fixed_pos(at + egui::vec2(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let choisi = self
                        .completion_index
                        .min(suggestions.candidates.len().saturating_sub(1));
                    for (index, candidate) in suggestions.candidates.iter().enumerate() {
                        let colour = candidate.kind.colour(theme);
                        let text = egui::RichText::new(&candidate.text).monospace();
                        // Celui qui sera pris, et non plus le premier :
                        // les flèches déplacent ce choix, et ne rien
                        // montrer bouger rendrait les flèches inutiles.
                        if index == choisi {
                            ui.horizontal(|ui| {
                                ui.colored_label(colour, text.strong());
                                ui.colored_label(theme.dim, s.completion_accept);
                            });
                        } else {
                            // Cliquable : la liste est sous le curseur,
                            // et viser une ligne à la souris est parfois
                            // plus court que de compter les flèches.
                            if cliquable(ui, egui::Label::new(text.color(theme.dim))).clicked() {
                                cliqué = Some(index);
                            }
                        }
                    }
                });
            });
        if let Some(index) = cliqué {
            self.completion_index = index;
            self.accept_completion(ui.ctx(), &suggestions);
        }
    }

    /// Écrit la proposition choisie à la place du mot commencé, et
    /// replace le curseur derrière.
    pub(crate) fn accept_completion(
        &mut self,
        ctx: &egui::Context,
        suggestions: &completion::Suggestions,
    ) {
        let Some(accepted) = suggestions.accepted_at(self.completion_index) else {
            return;
        };
        let accepted = accepted.to_owned();
        let accepted = accepted.as_str();
        // Le mot commencé est *remplacé*, et non complété : le candidat
        // porte sa propre casse, et une étiquette écrite `Boucle` doit
        // s'écrire `Boucle`.
        let range = suggestions.range();
        let end = range.start + accepted.len();
        self.source_mut().replace_range(range, accepted);

        // Le curseur d'egui compte en caractères, pas en octets.
        let after = byte_to_char(&self.source, end);
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id()) {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(after),
                )));
            state.store(ctx, editor_id());
        }
        self.suggestions = None;
        self.completion_index = 0;
    }

    /// La fiche de l'instruction survolée, en infobulle.
    ///
    /// F1 donne la fiche entière ; ceci répond à la question plus petite
    /// qu'on se pose en lisant — « celle-là, c'est quoi ? » — sans
    /// quitter la ligne des yeux ni presser quoi que ce soit. Soumis au
    /// réglage des infobulles : ceux qui les trouvent envahissantes n'en
    /// veulent pas ici non plus.
    pub(crate) fn hover_help_ui(
        &self,
        ui: &mut egui::Ui,
        géo: &Géométrie,
        réponse: &egui::Response,
    ) {
        if !self.show_tooltips || !self.hover_help {
            return;
        }
        let Some(pointer) = ui.ctx().pointer_hover_pos() else {
            return;
        };
        if !réponse.hovered() {
            return;
        }
        // Le listing occupe toute la hauteur de l'éditeur, bien au-delà
        // du code, et chaque ligne est bien plus courte que sa largeur :
        // sans ces deux vérifications, le pointeur posé dans le vide se
        // voyait attribuer le caractère le plus proche — le dernier du
        // fichier — et la fiche de cette instruction-là restait affichée
        // sans qu'on comprenne d'où elle venait.
        let Some(ligne) = géo.ligne_à(pointer.y) else {
            return;
        };
        let Some(rect) = géo.rect_de_ligne(ligne) else {
            return;
        };
        // Un demi-caractère de tolérance au bout de la ligne : viser une
        // lettre, pas son coin.
        let marge = rect.height() / 2.0;
        if pointer.x < rect.left() - marge || pointer.x > rect.right() + marge {
            return;
        }
        let Some(colonne) = géo.colonne_à(ligne, pointer.x) else {
            return;
        };
        // La position du pointeur dans le texte, puis le mot qui s'y
        // trouve : le même que celui sur lequel F1 répondrait.
        let Some(byte) = self.octet_de(ligne, colonne) else {
            return;
        };
        let word = word_at(&self.source, byte);
        let Some(entry) = namsdec_doc::lookup(word) else {
            return;
        };
        let lang = self.lang.code();
        let theme = self.theme;
        réponse.clone().show_tooltip_ui(|ui| {
            ui.set_max_width(420.0);
            ui.strong(entry.title.get(lang));
            ui.label(prose(ui, theme, entry.body.get(lang)));
            if !entry.flags.is_empty() {
                ui.colored_label(
                    theme.dim,
                    format!("{} {}", strings(self.lang).help_flags, entry.flags),
                );
            }
        });
    }
}
