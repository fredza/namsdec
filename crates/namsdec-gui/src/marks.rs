//! Ce qu'on pose sur le code pour s'y retrouver.
//!
//! Surligner toutes les occurrences d'un mot, les renommer d'un coup,
//! nommer une adresse, commenter une ligne : quatre gestes qui font la
//! différence entre lire un désassemblage et le comprendre.
//!
//! Les marques d'un binaire vivent à côté de lui et lui survivent :
//! rouvrir le même fichier, c'est retrouver ce qu'on avait compris la
//! dernière fois. C'est la raison d'être de ce module — sans quoi
//! chaque séance recommencerait à zéro.

use super::*;

impl NamsdecApp {
    /// L'autocomplétion : ce qu'on pourrait écrire là, et `Alt` pour
    /// l'accepter.
    ///
    /// Une liste flottante sous le curseur plutôt qu'un texte fantôme
    /// dans le champ : le tampon reste exactement ce que l'utilisateur a
    /// tapé, et rien de ce qu'il voit n'est écrit avant qu'il l'accepte.
    /// Va à l'occurrence suivante — ou précédente — et la sélectionne.
    ///
    /// Rien de surligné ? On surligne d'abord ce qui est sous le curseur.
    /// C'est le `*` puis `n` de vim en un seul geste, et cela évite de
    /// répondre « il n'y a rien » à quelqu'un qui a le mot sous les yeux.
    ///
    /// La boucle est circulaire : après la dernière vient la première.
    /// Buter contre la fin obligerait à remonter à la main pour repasser
    /// sur ce qu'on vient de dépasser.
    pub(crate) fn go_to_match(&mut self, avant: bool) {
        if self.tracked.is_none() {
            self.highlight_matches();
            if self.tracked.is_none() {
                return;
            }
        }
        self.refresh_matches();
        if self.tracked_spans.is_empty() {
            return;
        }
        let ici = self.cursor_byte;
        // La suivante est la première qui commence après le curseur ; à
        // défaut, on repart du début.
        let rang = if avant {
            self.tracked_spans
                .iter()
                .position(|é| é.start > ici)
                .unwrap_or(0)
        } else {
            self.tracked_spans
                .iter()
                .rposition(|é| é.end < ici)
                .unwrap_or(self.tracked_spans.len() - 1)
        };
        let cible = self.tracked_spans[rang].clone();
        self.cursor_byte = cible.start;
        self.pending_selection = Some(cible);
        // Sélectionner, c'est pouvoir remplacer tout de suite : on passe
        // donc en insertion, comme après un `i`.
        self.editor_mode = EditorMode::Insert;
        self.goto_line = None;
        self.status = Status::Counted(rang + 1, |s| s.matches_at);
    }

    /// Ouvre la saisie du nouveau nom.
    ///
    /// Il faut avoir surligné d'abord : renommer sans savoir *quoi*
    /// n'aurait pas de sens, et deviner le mot sous le curseur à ce
    /// moment-là ferait renommer autre chose que ce qu'on voyait.
    pub(crate) fn start_rename(&mut self) {
        if self.tracked.is_none() {
            // Un geste rendu : on surligne, et l'on pourra renommer.
            self.highlight_matches();
            if self.tracked.is_none() {
                self.status = Status::Note(|s| s.rename_nothing);
                return;
            }
        }
        self.rename = Some(
            self.tracked
                .as_ref()
                .map(|suivi| suivi.word.clone())
                .unwrap_or_default(),
        );
        self.rename_just_opened = true;
    }

    /// Applique le nouveau nom à toutes les occurrences surlignées.
    pub(crate) fn apply_rename(&mut self, nouveau: &str) {
        self.refresh_matches();
        let combien = self.tracked_spans.len();
        if combien == 0 || nouveau.is_empty() {
            self.rename = None;
            return;
        }
        let remplacé = occurrences::replace_all(&self.source, &self.tracked_spans, nouveau);
        self.set_source(remplacé);
        // Le surlignage suit le nouveau nom : on vient de le poser
        // partout, et le voir confirme que rien n'a été oublié.
        if let Some(suivi) = &mut self.tracked {
            suivi.word = nouveau.to_owned();
        }
        self.tracked_source_len = 0;
        self.refresh_matches();
        self.rename = None;
        self.status = Status::Counted(combien, |s| s.rename_done);
    }

    /// Peint les occurrences suivies.
    ///
    /// En vert, parce que c'est la couleur que le thème réserve à « ça va
    /// bien » — et qu'elle est verte dans tous les thèmes livrés comme
    /// dans ceux qu'on écrit : chacun choisit la sienne, donc chacun
    /// garde son contraste. Un vert en dur serait illisible sur un thème
    /// clair.
    ///
    /// Un fond translucide et un liseré plein : le fond seul disparaît
    /// sur les thèmes très contrastés, le liseré seul se perd dans une
    /// ligne dense.
    pub(crate) fn paint_matches(
        &mut self,
        ui: &egui::Ui,
        géo: &Géométrie,
        lignes: &std::ops::RangeInclusive<u32>,
    ) {
        self.refresh_matches();
        if self.tracked_places.is_empty() {
            return;
        }
        let peintre = ui.painter();
        // Celles qui sont à l'écran, et elles seules. Un `rax` surligné
        // dans un listing revient vingt mille fois : les situer toutes
        // demandait deux parcours complets du texte chacune, pour en
        // peindre soixante (voir `text::Place`).
        let vues = places_entre(&self.tracked_places, *lignes.start(), *lignes.end());
        for place in &self.tracked_places[vues] {
            // Une occurrence qui ne tient pas sur une rangée n'est pas
            // peinte : un surlignage faux est pire qu'un surlignage
            // manquant.
            let Some(rect) = géo.rect_de_place(place) else {
                continue;
            };
            if rect.width() <= 0.0 {
                continue;
            }
            peintre.rect_filled(rect, 2.0, self.theme.good.gamma_multiply(0.28));
            peintre.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, self.theme.good),
                egui::StrokeKind::Inside,
            );
        }
    }

    /// Ouvre la saisie d'un nom ou d'un commentaire pour la ligne où
    /// l'on est.
    ///
    /// Il faut une adresse : sur une ligne qui n'en a pas — une
    /// étiquette, un blanc — il n'y a rien à nommer, et le dire vaut
    /// mieux que d'ouvrir un champ qui n'écrira nulle part.
    pub(crate) fn start_note(&mut self, genre: NoteKind) {
        let ligne = self.cursor_pos.0;
        let adresse = self
            .assemble_state
            .as_ref()
            .and_then(|état| état.output.line_to_addr.get(&(ligne as u32)).copied());
        let Some(adresse) = adresse else {
            self.status = Status::Note(|s| s.notes_no_address);
            return;
        };
        // Le champ s'ouvre sur ce qui est déjà là : on corrige plus
        // souvent qu'on n'écrit de zéro.
        let actuel = match genre {
            NoteKind::Name => self.notes.name(adresse),
            NoteKind::Comment => self.notes.comment(adresse),
        }
        .unwrap_or_default()
        .to_owned();
        self.note_edit = Some((genre, adresse, actuel));
        self.note_just_opened = true;
    }

    /// Retrouve les notes de ce binaire, s'il en a.
    ///
    /// Appelé à l'ouverture. L'empreinte suit le contenu : un fichier
    /// déplacé ou renommé retrouve ses notes, et un binaire recompilé
    /// n'hérite pas de celles d'un autre.
    pub(crate) fn load_notes_for(&mut self, bytes: &[u8]) {
        let empreinte = notes::fingerprint(bytes);
        self.notes = match notes_dir() {
            Some(dossier) => notes::load(&dossier, &empreinte),
            None => notes::Notes::default(),
        };
        self.notes_fingerprint = Some(empreinte);
    }

    /// Écrit les notes du binaire ouvert.
    ///
    /// Tout de suite après chaque changement : un nom qu'on vient de
    /// trouver et qui disparaît au prochain lancement se cherche deux
    /// fois.
    pub(crate) fn save_notes(&mut self) {
        let (Some(empreinte), Some(dossier)) = (&self.notes_fingerprint, notes_dir()) else {
            return;
        };
        if let Err(e) = notes::save(&dossier, empreinte, &self.notes) {
            self.status = Status::Error(format!("{}: {e}", strings(self.lang).notes_not_saved));
        }
    }

    /// Récrit les étiquettes du listing après un renommage.
    ///
    /// **Répercuter, c'est aussi ici.** Un nom donné paraissait partout
    /// — arbre d'appels, références croisées, « prochaine instruction »,
    /// déroulement — sauf dans le listing lui-même, qui est pourtant ce
    /// qu'on a sous les yeux toute la journée. La raison : le listing
    /// est du texte, écrit une fois au chargement. On le récrit donc,
    /// et seulement lui : les octets du binaire ne bougent pas d'un
    /// bit.
    ///
    /// Trois précautions :
    ///
    /// * **Rien si le texte a été touché.** Devant un listing qu'on a
    ///   édité à la main, le récrire effacerait ce travail. On ne le
    ///   fait que tant qu'il est exactement celui qu'on a produit.
    /// * **Les points d'arrêt suivent leur adresse**, non leur numéro :
    ///   un nom nouveau insère une ligne d'étiquette, et tout ce qui
    ///   suit descend d'un cran.
    /// * **Les symboles d'origine restent intacts** : c'est ce qui
    ///   permet de se dédire.
    pub(crate) fn refresh_listing_names(&mut self) {
        let Some(state) = &self.assemble_state else {
            return;
        };
        if state.from_binary.is_none() || self.source != self.saved_source {
            return;
        }
        let symboles = symbols_with_names(&state.output.symbols, &self.notes);
        let (listing, line_to_addr) = annotated_listing(&state.disasm, &symboles);
        if listing == self.source {
            return;
        }
        let addr_to_line: BTreeMap<u64, u32> = line_to_addr.iter().map(|(l, a)| (*a, *l)).collect();
        // Les adresses des points d'arrêt AVANT de changer la table.
        let arrêts: Vec<u64> = self
            .breakpoint_lines
            .iter()
            .filter_map(|ligne| state.output.line_to_addr.get(ligne).copied())
            .collect();
        // La ligne que l'on regarde, elle aussi, suit son adresse.
        let ici = state
            .output
            .line_to_addr
            .get(&(self.cursor_pos.0 as u32))
            .copied();

        let Some(state) = &mut self.assemble_state else {
            return;
        };
        state.output.line_to_addr = line_to_addr;
        state.addr_to_line = addr_to_line.clone();
        self.set_source(listing.clone());
        self.saved_source = listing;
        self.breakpoint_lines = arrêts
            .iter()
            .filter_map(|addr| addr_to_line.get(addr).copied())
            .collect();
        self.sync_breakpoints();
        // Le texte a changé sous le curseur : les surlignages d'une
        // recherche ne désignent plus les mêmes octets.
        self.forget_matches();
        if let Some(ligne) = ici.and_then(|addr| addr_to_line.get(&addr).copied()) {
            self.goto_line = Some(ligne);
        }
    }

    /// Peint les commentaires à droite de leur ligne.
    ///
    /// À droite du code, et non dans un panneau à côté : un commentaire
    /// qu'il faut mettre en face de sa ligne de mémoire ne se lit pas.
    /// En couleur de commentaire, comme ceux qu'on écrit dans une source
    /// — ce sont les mêmes, à ceci près qu'ils ne sont pas dans le
    /// fichier.
    pub(crate) fn paint_notes(
        &self,
        ui: &egui::Ui,
        géo: &Géométrie,
        lignes: &std::ops::RangeInclusive<u32>,
    ) {
        if self.notes.comments.is_empty() {
            return;
        }
        let Some(état) = self.assemble_state.as_ref() else {
            return;
        };
        let peintre = ui.painter();
        let police = egui::TextStyle::Monospace.resolve(ui.style());
        // Les lignes à l'écran seulement : un binaire commenté en compte
        // un million, et chercher la note de chacune coûtait autant que
        // de les peindre.
        for ligne in lignes.clone() {
            let Some(adresse) = état.output.line_to_addr.get(&ligne) else {
                continue;
            };
            let Some(texte) = self.notes.comment(*adresse) else {
                continue;
            };
            let Some(rect) = géo.rect_de_ligne(ligne) else {
                continue;
            };
            // Après la fin du texte de la ligne, avec un blanc : collé,
            // on ne saurait plus où finit le code.
            let à = egui::pos2(rect.right() + 24.0, rect.top());
            peintre.text(
                à,
                egui::Align2::LEFT_TOP,
                format!("; {texte}"),
                police.clone(),
                self.theme.comment,
            );
        }
    }

    /// Relève ce qui est sous le curseur et montre où ça revient.
    ///
    /// Une sélection l'emporte sur le curseur : quelqu'un qui a
    /// sélectionné `ax` demande les `ax`, et lui rendre les `rax` serait
    /// répondre à côté.
    ///
    /// Refait sur le même mot, cela efface : c'est la même touche qui
    /// allume et qui éteint, comme un interrupteur — et l'on n'a pas à
    /// se souvenir d'un second raccourci pour revenir en arrière.
    pub(crate) fn highlight_matches(&mut self) {
        let suivi = match self.editor_selection.clone() {
            Some(étendue) if étendue.end > étendue.start => {
                occurrences::tracked_selection(&self.source, étendue)
            }
            _ => occurrences::tracked_at(&self.source, self.cursor_byte),
        };
        let Some(suivi) = suivi else {
            self.status = Status::Note(|s| s.matches_none);
            return;
        };
        // Le même que la dernière fois : on éteint.
        if self
            .tracked
            .as_ref()
            .is_some_and(|actuel| actuel.word == suivi.word && actuel.kind == suivi.kind)
        {
            self.clear_matches();
            return;
        }
        self.tracked_spans = occurrences::find_all(&self.source, &suivi);
        self.tracked_places = place_spans(&self.source, &self.tracked_spans);
        let combien = self.tracked_spans.len();
        self.tracked = Some(suivi);
        self.tracked_source_len = self.source.len();
        self.status = Status::Counted(combien, |s| s.matches_found);
    }

    /// Éteint le surlignage, et le dit.
    ///
    /// À n'employer que lorsque c'est le geste demandé — refaire la
    /// commande de surlignage, qui fonctionne en interrupteur. Éteindre
    /// est visible à l'écran : les marques disparaissent. L'annoncer
    /// quand personne ne l'a demandé écrase un message plus utile.
    pub(crate) fn clear_matches(&mut self) {
        if self.tracked.is_none() {
            return;
        }
        self.forget_matches();
        self.status = Status::Note(|s| s.matches_cleared);
    }

    /// Éteint le surlignage sans rien dire.
    ///
    /// Pour les effets de bord : Échap sert d'abord à quitter le mode
    /// Insertion, et annoncer « Surlignage effacé » à ce moment-là
    /// répond à côté de ce qu'on vient de faire.
    pub(crate) fn forget_matches(&mut self) {
        self.tracked = None;
        self.tracked_spans.clear();
        self.tracked_places.clear();
    }

    /// Remet les étendues à jour quand la source a changé.
    ///
    /// Les positions sont en octets : taper une lettre plus haut décale
    /// tout ce qui suit, et garder les anciennes surlignerait le milieu
    /// des mots. On recalcule plutôt que de deviner le décalage — c'est
    /// une recherche sur un fichier qu'on a déjà en mémoire, et cela
    /// coûte moins que de se tromper.
    pub(crate) fn refresh_matches(&mut self) {
        let Some(suivi) = self.tracked.clone() else {
            return;
        };
        if self.source.len() == self.tracked_source_len {
            return;
        }
        self.tracked_spans = occurrences::find_all(&self.source, &suivi);
        self.tracked_places = place_spans(&self.source, &self.tracked_spans);
        self.tracked_source_len = self.source.len();
    }

    /// Va à l'occurrence `current` : la ligne sous les yeux, le curseur
    /// dessus.
    pub(crate) fn goto_match(&mut self) {
        // La requête vient peut-être de changer — une frappe, un clic
        // sur « > » — : le relevé se remet à jour s'il le faut, et ne
        // coûte rien sinon.
        self.refresh_find_hits();
        let Some(find) = &self.find else { return };
        let Some(place) = self.find_hits.get(find.current) else {
            return;
        };
        self.goto_line = Some(place.ligne);
    }
}
