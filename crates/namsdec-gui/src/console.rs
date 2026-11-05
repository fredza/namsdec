//! La console : le panneau du bas et ses cinq onglets.
//!
//! C'est là que le programme répond — ce qu'il écrit, ce que
//! l'assembleur reproche, où le temps est passé, quels modules sont
//! chargés — et là qu'on lui parle quand il attend une entrée.
//!
//! Un seul panneau, cinq onglets, parce qu'un débutant qui cherche « où
//! s'affiche mon texte » ne doit pas avoir à deviner lequel des cinq
//! coins de l'écran le porte.

use super::*;

/// Les marges du cadre qui entoure la fenêtre de sortie : huit en
/// haut, huit en bas.
///
/// Elles sont écrites ici parce que TROIS calculs doivent compter la
/// même chose — ce que la console demande, le plancher qu'elle se
/// donne, et le cadre lui-même. Elles ne figuraient dans aucun des
/// deux premiers, et une console qui demande moins que son propre
/// plancher arrive à l'écran coincée à son minimum.
pub(crate) const MARGES_DE_LA_SORTIE: f32 = 16.0;

/// Ce qu'une boîte SANS ASCENSEUR peut montrer de la fin d'une sortie :
/// autant de lignes qu'il en tient, et combien restent au-dessus.
///
/// Un ascenseur dans une boîte qui s'ouvre toute seule est un piège :
/// on y lit quatre lignes d'un logo qui en fait sept, on croit que le
/// programme n'a écrit que cela, et l'on cherche la suite dans une
/// console qu'on ne regardait pas. La boîte prend donc la place qu'il
/// faut — et quand le fichier dépasse l'écran, elle montre LA FIN,
/// c'est-à-dire l'invite, et dit en une ligne ce qu'elle a laissé
/// derrière.
///
/// `colonnes` et `lignes` sont ce que l'écran permet. Une ligne plus
/// large que `colonnes` se replie et compte donc pour plusieurs : c'est
/// ce qui fait qu'un logo de quatre-vingts colonnes dans une boîte qui
/// en montre soixante occupe le double de hauteur.
///
/// Prises par la FIN, parce que l'invite y est toujours : un programme
/// écrit « Enter the password: » en dernier, et c'est la seule ligne
/// dont on ne peut pas se passer.
pub(crate) fn lignes_qui_tiennent(largeurs: &[usize], colonnes: usize, lignes: usize) -> usize {
    let colonnes = colonnes.max(1);
    let lignes = lignes.max(1);
    let mut prises = 0;
    let mut hauteur_totale = 0;
    for largeur in largeurs.iter().rev() {
        let h = largeur.div_ceil(colonnes).max(1);
        // La première passe toujours, même seule et trop haute : une
        // boîte vide devant un programme qui attend n'apprend rien.
        if hauteur_totale + h > lignes && prises > 0 {
            break;
        }
        hauteur_totale += h;
        prises += 1;
    }
    prises
}

impl NamsdecApp {
    /// La console montre-t-elle sa ligne de saisie ?
    ///
    /// Dès qu'un programme est assemblé, et sans autre condition.
    ///
    /// Elle n'apparaissait avant que si namsdec avait *repéré* un `read`
    /// dans le code, au nom d'une économie : « un programme sans `read`
    /// n'attendra jamais rien, et lui tendre un champ de saisie, c'est
    /// trois lignes qui ne servent à rien ». Le raisonnement tombe
    /// précisément dans le cas qui compte le plus — celui de quelqu'un
    /// qui écrit son programme, a posé ses chaînes, et veut préparer ce
    /// qu'il lira avant d'avoir tapé le `read`. Il n'y avait alors nulle
    /// part où taper, et rien à l'écran n'expliquait pourquoi.
    ///
    /// Plus généralement : un champ qui n'apparaît qu'à une condition que
    /// l'utilisateur ne peut pas voir n'est pas une économie, c'est une
    /// disparition. Trois lignes toujours là se comprennent ; trois
    /// lignes qui vont et viennent se cherchent.
    pub(crate) fn console_shows_input(&self) -> bool {
        self.emulator.is_some()
    }

    /// Donne au programme la ligne saisie, ou ferme son entrée (`None`),
    /// puis relance l'exécution si c'est elle qui attendait.
    pub(crate) fn send_input(&mut self, line: Option<String>) {
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match line {
            Some(line) => emulator.provide_input(format!("{line}\n").as_bytes()),
            None => emulator.close_input(),
        }

        // Un `Exécuter` suspendu repart tout seul : c'est le programme qui
        // attendait, pas l'utilisateur. Un pas à pas, lui, garde la main à
        // l'utilisateur, qui décide quand avancer.
        if matches!(self.status, Status::WaitingForInput) {
            self.status = if self.resume_run_after_input {
                self.resume_run_after_input = false;
                self.running = true;
                Status::Running
            } else {
                Status::Idle
            };
        }
    }
    /// La hauteur que la console réclame pour ce qu'elle a à dire.
    ///
    /// Une console qui annonce « le programme n'a rien écrit » n'a pas
    /// besoin du quart de l'écran ; vingt lignes de sortie, si. On
    /// compte donc les lignes, on borne — au-delà d'une douzaine, c'est
    /// la barre de défilement qui prend le relais — et on ajoute ce qui
    /// entoure : les onglets, le séparateur, et le champ d'entrée quand
    /// un programme peut lire.
    pub(crate) fn console_wanted_height(&self, ui: &egui::Ui) -> f32 {
        let ligne = ui.text_style_height(&egui::TextStyle::Monospace);
        let lignes = match self.console_tab {
            ConsoleTab::Output => self
                .emulator
                .as_ref()
                .map(|emulator| {
                    String::from_utf8_lossy(emulator.output())
                        .lines()
                        .count()
                        .max(1)
                })
                .unwrap_or(1),
            // Trois lignes par simplification : son titre, et deux
            // pour l'explication repliée.
            ConsoleTab::Diagnostics => {
                (self.diagnostics.len() + self.combien_de_simplifications() * 3).max(1)
            }
            // Une ligne par ligne chaude, plus l'en-tête et le total.
            ConsoleTab::Profile => self.line_visits().len().max(1) + 2,
        };
        // Les onglets, le séparateur et les marges du panneau.
        //
        // LA RANGÉE D'ONGLETS MESURE `ONGLET_HAUTEUR` — 26 —, et non la
        // hauteur du texte d'un bouton — 15. La formule a compté la
        // seconde pendant des semaines, c'est-à-dire onze pixels de
        // moins que la console n'en occupe, et la conséquence se voyait
        // sur toutes les captures sans qu'aucune mesure ne la nomme :
        // **la phrase que la console affiche quand elle n'a rien à
        // montrer était tranchée par le milieu** — neuf pixels peints
        // sur quinze, sur l'écran du premier lancement comme sur
        // quarante-deux autres scènes.
        //
        // Le nombre juste avait été essayé le 28 octobre 2026 puis
        // défait : il faisait alors rogner une ligne de la sortie par le
        // bas. Remis le 4 novembre, et cette fois les soixante-deux
        // invariants du banc passent — la hauteur de la saisie se
        // mesure depuis sur son contenu, ce qui n'était pas le cas
        // alors. Retenu par
        // `aucune_phrase_ne_perd_de_hauteur_au_decoupage`, éprouvé en
        // remettant les quinze pixels : quarante-trois scènes rougissent.
        let chrome = crate::widgets::ONGLET_HAUTEUR + 28.0;
        // La zone de saisie, comptée ligne à ligne comme elle se dessine.
        //
        // Une seule ligne y était réservée. Elle en porte jusqu'à quatre
        // — la mention d'attente, les arguments, l'en-tête de
        // l'environnement, l'entrée elle-même —, plus les marges de son
        // cadre : le panneau était trop court, et le champ se faisait
        // rogner par le bas. On ne voyait plus où taper au moment précis
        // où le programme le demandait.
        // Ce que la zone de saisie réclame. La mesure de l'image
        // précédente quand elle est plus grande — l'environnement déplié
        // découvre un champ de deux lignes —, jamais quand elle est plus
        // petite : voir `console_input_estimate`.
        let entrée = self
            .console_input_estimate(ui)
            .max(self.console_input_height.unwrap_or(0.0));
        // La sortie, et les marges du cadre qui l'entoure : huit en
        // haut, huit en bas, qui ne figuraient nulle part. La hauteur
        // demandée tombait donc SOUS le plancher que la même console se
        // donne plus loin, et egui, la voyant à son minimum, changeait
        // le pointeur de sa poignée en flèche à sens unique. Deux
        // formules pour une même console doivent compter les mêmes
        // choses.
        let contenu = ligne * lignes.min(12) as f32 + MARGES_DE_LA_SORTIE;
        (chrome + entrée + contenu).max(CONSOLE_MIN_HEIGHT)
    }

    /// La hauteur que la zone de saisie réclame, comptée rangée par
    /// rangée. Zéro quand il n'y a pas de saisie à montrer.
    ///
    /// **Calculée, et non mesurée** — et c'est tout le point. On se
    /// servait de la hauteur que la zone avait prise à l'image
    /// précédente, ce qui paraît plus juste puisqu'elle seule connaît sa
    /// vraie taille. Mais cette hauteur est celle qu'on lui a *allouée* :
    /// trop courte, la zone est tronquée, la mesure rend la hauteur
    /// tronquée, et le plancher qu'elle sert à calculer reste sous ce
    /// qu'il faut. Le panneau restait court parce qu'il était court.
    ///
    /// C'est ce qui faisait disparaître la rangée « Entrée : » — avec
    /// ses deux boutons — sur une console dont la hauteur avait été
    /// fixée à la main à 128 px : le programme demandait un mot de
    /// passe, et il n'y avait nulle part où le taper.
    pub(crate) fn console_input_estimate(&self, ui: &egui::Ui) -> f32 {
        if !self.console_shows_input() {
            return 0.0;
        }
        let rangée = ui
            .spacing()
            .interact_size
            .y
            .max(ui.text_style_height(&egui::TextStyle::Body))
            + ui.spacing().item_spacing.y;
        // L'entrée, toujours : c'est la rangée qui porte le champ et ses
        // deux boutons.
        let mut hauteur = rangée;
        if matches!(self.status, Status::WaitingForInput) {
            hauteur += rangée;
        }
        if self
            .assemble_state
            .as_ref()
            .is_some_and(|état| état.from_binary.is_some())
            && self.console_args_open
        {
            // Les arguments, et l'en-tête de l'environnement.
            hauteur += rangée * 2.0;
            if self.console_env_open {
                // Déplié : une ligne d'aide et un champ de deux lignes.
                hauteur += rangée * 3.0;
            }
        }
        // LA PHRASE DU DESSOUS, qui n'était comptée nulle part.
        //
        // Sous la rangée « Entrée : », la zone dit encore une chose :
        // combien d'octets attendent d'être lus, ou — quand rien dans
        // le programme ne les lira — ce que la saisie deviendra. Cette
        // ligne-là manquait au compte, et c'est elle qu'on perdait :
        // console figée à la poignée et environnement déplié, il n'en
        // restait que trois pixels de haut sur quinze, un filet de
        // sommets de lettres au ras de la barre de statut.
        //
        // Le mensonge est en deux temps. La zone réclame ce que cette
        // fonction annonce ; quand elle en veut plus, egui la RAMÈNE à
        // la place disponible en gardant son bas, si bien que la
        // hauteur relevée à l'image suivante est celle qu'on lui a
        // allouée — jamais celle qu'il lui fallait. Le `max` avec cette
        // mesure ne rattrape donc rien : c'est ici, et nulle part
        // ailleurs, que le compte doit être juste.
        let dit_quelque_chose_dessous = self
            .emulator
            .as_ref()
            .is_some_and(|emulator| emulator.pending_input() > 0)
            || (!matches!(self.status, Status::WaitingForInput) && !self.program_reads);
        if dit_quelque_chose_dessous {
            hauteur += rangée;
        }
        // Le code de sortie, annoncé au-dessus de la saisie dès que le
        // programme s'est terminé : son trait de séparation et sa ligne.
        if self
            .emulator
            .as_ref()
            .is_some_and(|emulator| emulator.exit_code().is_some())
        {
            hauteur += rangée + 6.0 + ui.spacing().item_spacing.y * 2.0;
        }
        // Ce qui n'est pas une rangée, compté terme à terme plutôt
        // qu'arrondi : un forfait de 24 px paraissait large et manquait
        // de neuf — juste assez pour couper la rangée « Entrée » en
        // deux, dont on ne voyait plus que le haut, champ et boutons
        // sous le bord de la fenêtre.
        let espacement = ui.spacing().item_spacing.y;
        // Le trait de séparation, et le blanc de part et d'autre.
        let séparateur = 6.0 + espacement * 2.0;
        // Les marges hautes et basses du cadre qui entoure la saisie,
        // plus son liseré quand il y en a un — deux pixels en haut, deux
        // en bas. Les oublier suffisait à couper « Envoyer » d'un
        // cheveu : le cadre a été épaissi pour se voir, et il fallait
        // que la place suive.
        let marges_du_cadre = if matches!(self.status, Status::WaitingForInput) {
            12.0 + 4.0
        } else {
            12.0
        };
        // La ligne d'attente porte une pastille, qui prend la hauteur
        // d'une rangée déjà comptée — rien à ajouter pour elle.
        // Le `add_space` posé avant la dernière rangée.
        let respiration = 2.0;
        // Et de quoi ne pas jouer au pixel près : les hauteurs de
        // police varient d'un thème à l'autre, et deux pixels de trop ne
        // coûtent rien quand deux de moins coupent la ligne où l'on tape.
        let sûreté = 8.0;
        hauteur + séparateur + marges_du_cadre + respiration + sûreté
    }
    /// Le panneau bas : la console du programme et les diagnostics de
    /// nasm, sur toute la largeur de la fenêtre.
    ///
    /// La console y a la largeur d'un terminal — un programme écrit en 80
    /// colonnes, le panneau latéral n'en offrait que trente-quatre — et
    /// les diagnostics y trouvent enfin de quoi être lus : ils étaient
    /// jusqu'ici entassés sous la barre de statut, sur deux lignes
    /// hautes de rien.
    pub(crate) fn console_panel_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // LE MÊME IDIOME D'ONGLETS QUE PARTOUT AILLEURS. Ces trois-là
        // étaient des `selectable_label` : l'onglet choisi prenait le
        // bleu plein de la sélection, c'est-à-dire l'apparence d'un
        // bouton d'action — dans la fenêtre des préférences, l'onglet
        // « Général » et le bouton « OK » se peignaient pareil. Un
        // sélecteur et une commande ne doivent pas se ressembler.
        //
        // `rangée_d_onglets` en prévenait : « le jour où l'une des deux
        // divergerait, l'application aurait trois idiomes d'onglets au
        // lieu de deux ». Elle en avait trois, et son inventaire ne les
        // comptait pas — la console et les préférences n'étaient pas
        // passées par elle.
        //
        // Le compte des diagnostics se lit sur l'onglet : c'est ce qui
        // fait remarquer un avertissement qu'on n'était pas allé
        // chercher.
        let libellés: Vec<String> = ConsoleTab::ALL
            .iter()
            .map(|tab| {
                // Les simplifications comptent avec les diagnostics :
                // elles sont dans le même onglet, et un compteur qui
                // n'annonce qu'une partie de ce qu'il y a à lire est
                // pire que pas de compteur.
                if *tab == ConsoleTab::Diagnostics
                    && self.diagnostics.len() + self.combien_de_simplifications() > 0
                {
                    format!(
                        "{} ({})",
                        tab.label(s),
                        self.diagnostics.len() + self.combien_de_simplifications()
                    )
                } else {
                    tab.label(s).to_owned()
                }
            })
            .collect();
        let étiquettes: Vec<(&str, Option<&str>)> =
            libellés.iter().map(|l| (l.as_str(), None)).collect();
        let actif = ConsoleTab::ALL
            .iter()
            .position(|tab| *tab == self.console_tab)
            .unwrap_or(0);
        // LE BOUTON DES ARGUMENTS, à droite de la rangée d'onglets.
        //
        // C'est la place des commandes d'un panneau dans cette
        // application — « Détacher » se pose ainsi sur la rangée des
        // vues —, et c'est aussi celle qu'ont les outils dont Fred a
        // envoyé les captures : la fenêtre « Run » d'un JetBrains porte
        // sa barre d'icônes au bord, et laisse TOUT le reste à la
        // sortie.
        //
        // Il ne paraît que devant un binaire ouvert, comme la rangée
        // qu'il commande : un programme qu'on vient d'assembler n'a pas
        // d'arguments à recevoir.
        let avec_arguments = self
            .assemble_state
            .as_ref()
            .is_some_and(|état| état.from_binary.is_some());
        let largeur_bouton = if avec_arguments {
            let galette = ui.painter().layout_no_wrap(
                s.console_args_toggle.to_owned(),
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::PLACEHOLDER,
            );
            galette.size().x + ui.spacing().button_padding.x * 2.0 + 4.0 + ONGLET_MARGE * 2.0
        } else {
            0.0
        };
        let mut bascule_arguments = false;
        let mut cacher = false;
        if let Some(rang) = rangée_d_onglets(
            ui,
            self.theme,
            &étiquettes,
            actif,
            largeur_bouton + ONGLET_HAUTEUR + 4.0,
            |ui| {
                // Le tiret d'abord : on part du bord droit, et c'est au
                // bord que se tient ce qui range le panneau — comme
                // dans la fenêtre Run de JetBrains, d'où vient la
                // consigne.
                cacher = bouton_cacher(ui, self.theme, s.rail_console_hide);
                if avec_arguments
                    && small_framed_button(ui, self.theme, s.console_args_toggle)
                        .on_hover_text(s.console_args_toggle_hint)
                        .clicked()
                {
                    bascule_arguments = true;
                }
            },
        ) {
            self.console_tab = ConsoleTab::ALL[rang];
        }
        if bascule_arguments {
            self.console_args_open = !self.console_args_open;
        }
        if cacher {
            self.show_console = false;
        }
        ui.separator();
        match self.console_tab {
            ConsoleTab::Output => self.output_ui(ui),
            ConsoleTab::Diagnostics => self.diagnostics_ui(ui),
            ConsoleTab::Profile => self.profile_ui(ui),
        }
    }

    /// Le profil : où le programme a passé son temps.
    ///
    /// La bande de chaleur dans la gouttière répond à « par où ça
    /// chauffe » d'un coup d'œil, mais elle ne classe pas : deux rouges
    /// voisins ne disent pas lequel coûte le double de l'autre. Ce
    /// tableau le dit — le nombre de passages, la part du total, et la
    /// ligne elle-même — et y emmène d'un clic. C'est ce qu'un
    /// profileur montre d'un programme, à la seule échelle qui vaille
    /// ici : la ligne de source.
    pub(crate) fn profile_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let passages = self.line_visits();
        if passages.is_empty() {
            ui.colored_label(theme.dim, s.profile_empty);
            return;
        }
        // Le plus chaud d'abord ; à égalité, la ligne la plus haute,
        // pour que deux exécutions de suite donnent le même ordre.
        let mut classement: Vec<(u32, u64)> = passages.iter().map(|(l, c)| (*l, *c)).collect();
        classement.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let total: u64 = classement.iter().map(|(_, c)| *c).sum();
        let plus_chaude = classement.first().map_or(1, |(_, c)| *c).max(1);

        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                theme.dim,
                format!("{} {}", sidebar::group_digits(total), s.profile_total),
            );
            // Combien de lignes sont classées, et non seulement combien
            // d'instructions ont tourné. La console en montre une
            // douzaine ; sans ce chiffre, rien ne disait que la liste
            // continue sous le bord — ni de combien. « Je ne sais pas me
            // déplacer dans profil » venait de là : on ne cherche pas à
            // dérouler ce qu'on croit avoir vu en entier.
            ui.colored_label(theme.dim, "·");
            ui.colored_label(
                theme.dim,
                format!(
                    "{} {}",
                    sidebar::group_digits(classement.len() as u64),
                    s.profile_lines_ranked
                ),
            )
            // Et comment y aller. Le défilement marchait — mesuré : la
            // molette déroule bien la liste —, mais rien ne disait qu'il
            // y avait une suite, ni qu'on pouvait tirer le bord haut de
            // la console pour en voir plus d'un coup. « Je ne sais pas me
            // déplacer dans profil » était un grief de découvrabilité, et
            // c'est le chiffre au-dessus qui le règle : on ne cherche pas
            // à dérouler ce qu'on croit avoir vu en entier.
            .on_hover_text(s.profile_scroll_hint);
            if small_framed_button(ui, self.theme, s.profile_copy).clicked() {
                ui.ctx().copy_text(self.profile_text(&classement, total));
                self.status = Status::Note(|s| s.profile_copied);
            }
        });
        // Le budget de l'exercice, quand il y en a un : c'est ici qu'on
        // vient quand on le dépasse, et la réponse — quelle ligne
        // reprendre — est juste en dessous.
        if let Some(budget) = self
            .current_exercise
            .as_deref()
            .and_then(|id| namsdec_exercises::find(&self.exercise_packs, id))
            .and_then(|exercice| exercice.par)
        {
            let exécutées = self.emulator.as_ref().map_or(0, |e| e.executed());
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, s.par_label);
                ui.colored_label(
                    if exécutées > budget {
                        theme.warn
                    } else {
                        theme.good
                    },
                    egui::RichText::new(format!("{exécutées} / {budget}")).monospace(),
                );
                if exécutées > budget {
                    ui.colored_label(theme.dim, s.profile_over_budget);
                }
            });
        }
        ui.colored_label(theme.dim, s.profile_hint);
        ui.separator();

        // Le source découpé UNE fois, et ses étiquettes relevées une
        // fois, au lieu d'être reparcourus pour chacune des cinq cents
        // lignes du classement. C'était dix-sept secondes par image sur
        // un binaire d'un million d'octets — et le pire cas était le
        // plus courant : un binaire sans symboles n'a aucune étiquette,
        // donc la recherche remontait tout le listing à chaque ligne.
        let lignes_du_source: Vec<&str> = self.source.lines().collect();
        let étiquettes = source_labels(&self.source);
        let mut goto = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Une grille, et non des rangées posées à la main : les
                // colonnes d'une rangée tombaient sous les mots d'une
                // autre, et l'en-tête — un seul texte dont les espaces ne
                // mesuraient rien — sous aucune.
                egui::Grid::new("profile_rows")
                    .num_columns(6)
                    .min_col_width(0.0)
                    .striped(true)
                    .show(ui, |ui| {
                        // L'en-tête : sans lui, « 12 » et « 4,2 % » se
                        // lisent, mais « 1 204 » se prend pour une
                        // adresse.
                        ui.label("");
                        // Les titres au même gabarit que les nombres
                        // qu'ils surmontent : dans une chasse fixe, c'est
                        // le remplissage qui aligne, et la grille fait le
                        // reste. Une cellule de grille ne prend que la
                        // largeur de son contenu — un calage à droite n'y
                        // change donc rien.
                        for (titre, largeur) in [
                            (s.line_word, COL_LIGNE),
                            (s.profile_count, COL_PASSAGES),
                            (s.profile_share, COL_PART),
                        ] {
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(caler_à_droite(titre, largeur)).monospace(),
                            );
                        }
                        ui.label("");
                        ui.label("");
                        ui.end_row();

                        for (ligne, compte) in &classement {
                            let part = *compte as f32 / total.max(1) as f32;
                            let teinte = heat_colour(theme, *compte, plus_chaude);
                            // La pastille reprend exactement la teinte de
                            // la gouttière : la même ligne a la même
                            // couleur aux deux endroits, sinon le lien se
                            // perd.
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(8.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 1.0, teinte);

                            // LE NUMÉRO DE LIGNE EST CLIQUABLE, et c'est
                            // celui qu'on vise.
                            //
                            // Seul le TEXTE de la ligne menait quelque
                            // part — tout à droite de la rangée, et
                            // tronqué. On clique « 60749 », qui est la
                            // réponse à « où ça ? », et il ne se passait
                            // rien : le classement passait pour mort.
                            let numéro = cliquable(
                                ui,
                                egui::Label::new(
                                    egui::RichText::new(caler_à_droite(
                                        &format!("{ligne}"),
                                        COL_LIGNE,
                                    ))
                                    .monospace()
                                    .color(ui.visuals().hyperlink_color),
                                ),
                            );
                            if numéro.on_hover_text(s.profile_goto).clicked() {
                                goto = Some(*ligne);
                            }
                            // Les deux autres nombres, calés à droite de
                            // leur colonne : c'est ainsi que des chiffres
                            // se comparent d'une ligne à l'autre.
                            for (texte, largeur, couleur) in [
                                (sidebar::group_digits(*compte), COL_PASSAGES, theme.text),
                                (pourcentage(part, self.lang), COL_PART, theme.dim),
                            ] {
                                ui.colored_label(
                                    couleur,
                                    egui::RichText::new(caler_à_droite(&texte, largeur))
                                        .monospace(),
                                );
                            }

                            // Une barre proportionnelle : les pourcentages
                            // se comparent bien plus vite en longueur
                            // qu'en chiffres.
                            let (barre, _) = ui
                                .allocate_exact_size(egui::vec2(60.0, 10.0), egui::Sense::hover());
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    barre.min,
                                    egui::vec2(barre.width() * part, barre.height()),
                                ),
                                1.0,
                                teinte,
                            );

                            // Le bloc et la ligne elle-même : sans eux, le
                            // classement renvoie à un numéro qu'il faut
                            // aller chercher.
                            ui.horizontal(|ui| {
                                if let Some(étiquette) = enclosing_label_at(&étiquettes, *ligne) {
                                    ui.colored_label(
                                        theme.label,
                                        egui::RichText::new(format!("{étiquette}:")).monospace(),
                                    );
                                }
                                let réponse = cliquable(
                                    ui,
                                    egui::Label::new(
                                        egui::RichText::new(source_line_in(
                                            &lignes_du_source,
                                            *ligne,
                                        ))
                                        .monospace()
                                        .color(theme.text),
                                    )
                                    .truncate(),
                                );
                                if réponse.on_hover_text(s.profile_goto).clicked() {
                                    goto = Some(*ligne);
                                }
                            });
                            ui.end_row();
                        }
                    });
            });
        if let Some(ligne) = goto {
            self.goto_line = Some(ligne);
        }
    }

    /// Combien de simplifications l'exécution en cours a touchées.
    ///
    /// Zéro quand aucun programme ne tourne : le recensement complet
    /// est dans le manuel, l'onglet ne compte que ce qui a eu lieu.
    pub(crate) fn combien_de_simplifications(&self) -> usize {
        self.emulator
            .as_ref()
            .map(|emulator| emulator.touched_simplifications().len())
            .unwrap_or(0)
    }

    /// Les comportements que l'exécution en cours a simplifiés, chacun
    /// avec ce qu'il change.
    ///
    /// Rien quand il n'y a rien : une rubrique « aucune approximation »
    /// affichée en permanence devient du décor, et ce n'est pas le but.
    /// Ce qui est montré ici est ce que CE programme a touché — le
    /// recensement complet est dans le manuel, sous « arrondi ».
    pub(crate) fn simplifications_ui(&mut self, ui: &mut egui::Ui) {
        let (lang, theme) = (self.lang, self.theme);
        let fr = lang == Lang::Fr;
        let Some(emulator) = &self.emulator else {
            return;
        };
        let vues = emulator.touched_simplifications();
        if vues.is_empty() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                theme.warn,
                if fr {
                    "Ce que cette exécution a simplifié :"
                } else {
                    "What this run simplified:"
                },
            );
            // Vers la fiche qui tient le recensement entier, et dit
            // aussi ce qui N'EST PAS simplifié — la précision double,
            // par exemple, où namsdec répond bit pour bit.
            if small_framed_button(
                ui,
                theme,
                if fr {
                    "Lire la fiche « arrondi »"
                } else {
                    "Read the \"rounding\" entry"
                },
            )
            .clicked()
            {
                self.help = Some(("arrondi".to_owned(), namsdec_doc::lookup("arrondi")));
                self.help_back.clear();
                self.help_query.clear();
                self.show_help = true;
            }
        });
        for quoi in vues {
            ui.colored_label(
                theme.warn,
                egui::RichText::new(format!("• {}", limites::titre(lang, quoi))).monospace(),
            );
            ui.indent(("simplification", quoi.cle()), |ui| {
                // `prose` et non un label simple : les explications
                // citent des instructions entre accents graves, et un
                // accent grave peint tel quel est un caractère parasite
                // qui dit au lecteur que personne n'a regardé.
                let texte = limites::explication(lang, quoi);
                ui.label(prose(ui, theme, texte));
            });
        }
        ui.separator();
    }

    /// Ce que nasm a dit de la source, une ligne par diagnostic.
    pub(crate) fn diagnostics_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let (lang, theme) = (self.lang, self.theme);
        let mut goto = None;
        // Les deux commandes réellement lancées. « Et si je veux le
        // faire sans namsdec ? » est la bonne question, et un atelier
        // d'apprentissage y répond au lieu de garder son tour de main
        // pour lui.
        let fichier = self
            .current_file
            .as_ref()
            .and_then(|path| path.file_stem())
            .map(|nom| nom.to_string_lossy().into_owned())
            .unwrap_or_else(|| "programme".to_owned());
        let commandes = namsdec_asm::build_commands(&format!("{fichier}.asm"), &fichier, BASE_ADDR);
        // AU PASSÉ SEULEMENT SI ELLES L'ONT ÉTÉ. Le panneau les montre
        // dès l'ouverture, et c'est tout son intérêt ; mais « lancées »
        // avant le premier assemblage est une petite fausseté, et c'est
        // le dernier endroit où en laisser une — on vient y lire ce que
        // la machine a vraiment fait.
        let annonce = match self.assemble_state {
            Some(_) => s.assemble_commands,
            None => s.assemble_commands_future,
        };
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, annonce);
            if small_framed_button(ui, theme, s.assemble_commands_copy)
                .on_hover_text(s.assemble_commands_hint)
                .clicked()
            {
                ui.ctx().copy_text(commandes.join("\n"));
                self.status = Status::Note(|s| s.assemble_commands_copied);
            }
        });
        for commande in &commandes {
            ui.colored_label(theme.dim, egui::RichText::new(commande).monospace());
        }
        ui.separator();
        // TOUT CE QUI GRANDIT VA DANS LA ZONE DÉFILANTE — la rubrique
        // des simplifications comprise. Posée au-dessus d'elle, elle
        // poussait le panneau : à 800x520, la rangée d'onglets de la
        // console se peignait sur les numéros de ligne de l'éditeur.
        // C'est le défaut d'A 9.37, « un panneau qui déborde vole la
        // place de son voisin », et il revient par tout contenu qu'on
        // ajoute en dehors du défilement. Seules l'annonce et les deux
        // commandes restent fixes : leur hauteur, elle, ne dépend de
        // rien.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Ce que l'exécution a simplifié, s'il y a lieu, et en
                // TÊTE : c'est l'information que la barre d'état vient
                // de nommer, et celui qui vient la lire ne doit pas
                // avoir à défiler. Le compteur de l'onglet la compte
                // avec les diagnostics de nasm, sans quoi il
                // promettrait « (1) » et montrerait autre chose.
                //
                // Ici et non dans la sortie du programme : ce n'est pas
                // le programme qui parle, c'est namsdec, et mélanger
                // les deux voix dans un même flux est le plus sûr moyen
                // de faire douter de ce qu'on lit.
                self.simplifications_ui(ui);
                if self.diagnostics.is_empty() {
                    ui.colored_label(theme.dim, s.no_diagnostics);
                }
                for d in &self.diagnostics {
                    let color = if d.severity == Severity::Error {
                        theme.bad
                    } else {
                        theme.warn
                    };
                    // La phrase est reformée ici quand le diagnostic
                    // porte son motif : c'est ce qui la fait suivre la
                    // langue du moment. Sans cela, ceux qui étaient déjà
                    // dans la liste gardaient celle où ils étaient nés,
                    // et basculer de langue laissait du français dans une
                    // interface anglaise — les retraduire aurait demandé
                    // de réassembler.
                    let phrase = match &d.kind {
                        Some(kind) => kind.message(lang == Lang::Fr),
                        None => d.message.clone(),
                    };
                    // Un diagnostic sans ligne — ceux de l'éditeur de
                    // liens, qui parlent du programme entier — ne
                    // s'annonce pas par « ligne 0 ».
                    let text = match (d.line, lang) {
                        (0, _) => phrase,
                        (line, Lang::Fr) => format!("ligne {line} — {phrase}"),
                        (line, _) => format!("line {line} — {phrase}"),
                    };
                    // Cliquable quand nasm a nommé une ligne : lire le
                    // message et retrouver l'endroit sont deux gestes, et
                    // le second n'a aucune raison d'être manuel.
                    let response = cliquable(
                        ui,
                        egui::Label::new(egui::RichText::new(text).monospace().color(color)),
                    );
                    if d.line > 0 && response.on_hover_text(s.diagnostic_goto).clicked() {
                        goto = Some(d.line);
                    }
                    // Le message brut reste — c'est lui qu'on colle dans
                    // une recherche —, et l'explication vient dessous.
                    if let Some(explication) = nasm_errors::explain(lang, &d.message) {
                        ui.label(prose(ui, theme, explication));
                        ui.add_space(6.0);
                    }
                }
            });
        if let Some(line) = goto {
            self.goto_line = Some(line);
        }
    }

    /// La console du programme : ce qu'il a écrit, et de quoi lui donner
    /// ce qu'il veut lire.
    ///
    /// Les octets de sortie sont rendus en UTF-8 « au mieux » : un
    /// programme en assembleur écrit ce qu'il veut, y compris des octets
    /// qui ne forment aucun texte, et ce n'est pas une raison pour ne rien
    /// afficher.
    ///
    /// La sortie a sa fenêtre à elle — encadrée, qui garde sa place même
    /// vide et défile pour son compte — plutôt que d'être une suite de
    /// lignes posées au-dessus du champ de saisie : un programme bavard
    /// repoussait ce champ hors de vue, et il fallait faire défiler pour
    /// lui répondre.
    pub(crate) fn output_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // La fenêtre de sortie est là dans tous les cas : sans programme
        // chargé elle le dit, plutôt que de laisser l'onglet vide.
        // LA SORTIE, LUE COMME UN TERMINAL LA LIRAIT. Un programme
        // n'écrit pas que des lettres : il efface l'écran, remonte le
        // curseur, passe en cyan. Montrées telles quelles, ces
        // séquences se lisaient « □[2J□[H□[36m » en tête du logo, et le
        // dessin qui suivait paraissait abîmé.
        let (écran, exit_code, pending) = match &self.emulator {
            Some(emulator) => (
                ansi::écran(emulator.output()),
                emulator.exit_code(),
                emulator.pending_input(),
            ),
            None => (ansi::Écran::default(), None, 0),
        };
        let output = écran.texte();

        // Posée en bas du panneau, la saisie ne bouge plus quoi qu'écrive
        // le programme.
        if self.console_shows_input() {
            let zone = egui::Panel::bottom("console_input")
                .resizable(false)
                .frame(egui::Frame::NONE)
                .show(ui, |ui| {
                    if let Some(code) = exit_code {
                        ui.separator();
                        let color = if code == 0 {
                            self.theme.good
                        } else {
                            self.theme.bad
                        };
                        ui.colored_label(
                            color,
                            if self.lang == Lang::Fr {
                                format!("Le programme s'est terminé avec le code {code}.")
                            } else {
                                format!("The program exited with code {code}.")
                            },
                        );
                    }
                    self.console_input_ui(ui, pending);
                    // CE QUE LE CONTENU VOULAIT, ET NON CE QU'ON LUI A
                    // DONNÉ.
                    //
                    // La hauteur se relevait sur l'état du panneau,
                    // c'est-à-dire sur son rectangle — et egui RAMÈNE ce
                    // rectangle à la place disponible quand le contenu
                    // déborde, en gardant son bas. La mesure rendait
                    // donc la place allouée, jamais la hauteur voulue :
                    // le panneau restait court parce qu'il était court,
                    // et le `max` qu'on en faisait plus loin ne
                    // rattrapait rien.
                    //
                    // `min_rect` est l'autre mesure, celle du CONTENU :
                    // ce que les rangées ont réellement pris, qu'il y
                    // ait eu la place ou non. C'est elle qui dit à
                    // l'image suivante combien réserver, et c'est elle
                    // qui a fait réapparaître les deux rangées du haut —
                    // les arguments et l'en-tête de l'environnement —
                    // que le fond de la sortie recouvrait.
                    ui.min_rect().height()
                });
            self.console_input_height = Some(zone.inner).filter(|hauteur| *hauteur > 1.0);
        } else {
            self.console_input_height = None;
        }

        // Le reste de la hauteur revient à la sortie, sur le fond sombre
        // d'un terminal.
        //
        // Elle se contente de ce qui reste, fût-ce une demi-ligne :
        // essayé de lui donner des lignes entières en lui imposant une
        // hauteur, et REFUSÉ sur mesure — le « jamais moins d'une
        // ligne » qu'il fallait pour une console serrée la faisait
        // déborder sur ses voisins, et quatre invariants l'ont dit dans
        // la minute. Une zone qui défile a le droit de couper ; ce
        // qu'on protège, c'est la rangée où l'on tape, qui ne défile
        // pas.
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .inner_margin(egui::Margin::same((MARGES_DE_LA_SORTIE / 2.0) as i8))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    // UNE ZONE DÉFILANTE RÉCLAME SOIXANTE-QUATRE PIXELS
                    // SI ON NE LUI DIT RIEN, et les prend même quand il
                    // n'en reste que sept — elle est peinte AVANT la
                    // saisie dans l'ordre des formes, mais son fond est
                    // opaque et couvre donc ce qui a été peint avant
                    // elle. Console figée à la poignée, la rangée des
                    // arguments et l'en-tête de l'environnement
                    // disparaissaient ainsi sous le fond du terminal :
                    // présents, mesurables, invisibles.
                    //
                    // Zéro, et non une ligne : la sortie a un
                    // ascenseur, elle sait donc n'en montrer aucune:
                    // tandis que la rangée où l'on tape, elle, ne sait
                    // pas se replier. Qui se serre est celui qui peut.
                    //
                    // C'est aussi ce qui laisse à la poignée de quoi
                    // être tirée : une console dont le plancher vaut
                    // exactement ce qu'elle demande est coincée à son
                    // minimum, et egui change alors son pointeur en
                    // flèche à sens unique.
                    .min_scrolled_height(0.0)
                    // Ce que le programme vient d'écrire est ce qu'on veut
                    // lire : la vue suit le bas, comme un terminal.
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.emulator.is_none() {
                            // Sa phrase à elle, et non le constat qui
                            // est déjà écrit deux fois plus haut. Au
                            // démarrage, « Aucun binaire assemblé. »
                            // s'affichait trois fois sur le même écran
                            // — le bandeau, l'onglet, et ici — sans que
                            // la troisième apprenne rien de plus. Une
                            // console dit ce qu'elle montrera.
                            ui.colored_label(self.theme.dim, s.console_before_run);
                        } else if output.is_empty() {
                            // Dire *pourquoi* c'est vide. « Le programme
                            // n'a rien écrit » devant un programme qui
                            // n'écrit jamais rien laisse chercher une
                            // panne qui n'existe pas ; devant un
                            // programme qu'on n'a pas lancé, c'est
                            // simplement faux.
                            let exécuté = self.emulator.as_ref().is_some_and(|e| e.executed() > 0);
                            ui.colored_label(
                                self.theme.dim,
                                match (self.program_writes, exécuté) {
                                    (false, _) => s.output_never,
                                    (true, false) => s.output_not_yet,
                                    (true, true) => s.output_empty,
                                },
                            );
                        } else {
                            // Sélectionnable : la sortie d'un programme se
                            // recopie ailleurs (un rapport, un test).
                            //
                            // En UN SEUL bloc, couleurs comprises : une
                            // étiquette par morceau coûterait des
                            // milliers de widgets sur une sortie un peu
                            // longue, et la sélection ne traverserait
                            // pas de l'un à l'autre.
                            let job = ansi::mise_en_page(
                                &écran,
                                egui::TextStyle::Monospace.resolve(ui.style()),
                                self.theme,
                            );
                            ui.add(egui::Label::new(job).selectable(true));
                        }
                    });
            });
    }

    /// La boîte de saisie : là où l'on répond au programme.
    ///
    /// La ligne de la console suffit quand on la voit. Elle ne suffit pas
    /// quand une fenêtre d'analyse couvre l'écran, quand la console est
    /// repliée, ou simplement quand on ne sait pas encore qu'elle existe
    /// — et l'on reste alors devant un programme qui ne répond plus, sans
    /// savoir qu'il attend *nous*.
    ///
    /// La boîte s'ouvre donc d'elle-même au moment où le programme se met
    /// à attendre, au premier plan, le curseur déjà dans le champ. Elle
    /// montre **ce que le programme vient d'écrire** : neuf fois sur dix
    /// c'est l'invite — « Enter the password: » —, et c'est la seule
    /// chose qui dise ce qu'on attend de nous.
    pub(crate) fn input_dialog_ui(&mut self, ctx: &egui::Context) {
        // Le passage à l'attente ouvre la boîte ; l'attente seule ne la
        // rouvre pas, sans quoi la refermer serait impossible.
        let attend = matches!(self.status, Status::WaitingForInput);
        if attend && !self.was_waiting_for_input {
            self.show_input_dialog = true;
            self.input_dialog_focus = true;
        }
        self.was_waiting_for_input = attend;
        if !self.show_input_dialog {
            return;
        }

        let s = strings(self.lang);
        let theme = self.theme;
        // Les dernières lignes écrites : l'invite, et ce qui la précède.
        // La sortie LUE COMME UN TERMINAL : sans cela, l'invite
        // s'ouvrait sur « □[2J□[H□[36m » et le logo paraissait abîmé.
        let terminal = self
            .emulator
            .as_ref()
            .map(|e| ansi::écran(e.output()))
            .unwrap_or_default();
        // CE QUE LE PROGRAMME A ÉCRIT, EN ENTIER, ET SANS ASCENSEUR.
        //
        // Quatre lignes et une largeur fixe de 480 pixels : un logo de
        // sept lignes sur quatre-vingts colonnes — ce par quoi commence
        // la moitié des crackmes — y perdait ses trois premières lignes
        // et sa moitié droite. On lisait un fragment, et rien ne disait
        // que c'en était un.
        //
        // La boîte prend donc la place qu'il faut : la largeur de la
        // plus longue ligne, la hauteur de toutes. Bornée à l'écran,
        // parce qu'un programme peut écrire mille lignes — et dans ce
        // cas c'est LA FIN qu'on montre, l'invite étant toujours en
        // bas, avec une ligne qui dit ce qui reste au-dessus.
        let police = egui::TextStyle::Monospace.resolve(&ctx.style_of(ctx.theme()));
        let chasse = ctx.fonts_mut(|f| f.glyph_width(&police, '0')).max(1.0);
        let interligne = ctx.fonts_mut(|f| f.row_height(&police)).max(1.0);
        let écran = ctx.viewport_rect();
        // Ce que le cadre, les marges et la fenêtre prennent autour du
        // texte : mesuré sur la boîte, et non deviné.
        const AUTOUR_X: f32 = 60.0;
        const AUTOUR_Y: f32 = 280.0;
        let colonnes = (((écran.width() * 0.9 - AUTOUR_X) / chasse) as usize).max(20);
        let lignes_tenables = (((écran.height() - AUTOUR_Y) / interligne) as usize).max(3);
        // Coupé sur les LIGNES DE L'ÉCRAN, couleurs comprises : couper
        // le texte d'un côté et peindre l'écran de l'autre ferait deux
        // découpes à tenir d'accord.
        let largeurs: Vec<usize> = terminal
            .lignes
            .iter()
            .map(|ligne| ligne.iter().map(|m| m.texte.chars().count()).sum())
            .collect();
        let prises = lignes_qui_tiennent(&largeurs, colonnes, lignes_tenables);
        let au_dessus = terminal.hauteur() - prises;
        let vue = ansi::Écran {
            lignes: terminal.lignes[au_dessus..].to_vec(),
        };
        let invite = vue.texte();
        // La largeur voulue : celle du texte, jamais moins que la boîte
        // d'avant — un dialogue qui rétrécit à chaque réponse se lit
        // comme s'il se refermait.
        let plus_longue = largeurs[au_dessus..].iter().copied().max().unwrap_or(0);
        let largeur =
            (plus_longue as f32 * chasse + AUTOUR_X).clamp(480.0, (écran.width() * 0.9).max(480.0));

        let mut envoyer = false;
        let mut fin = false;
        let mut fermer = false;
        // Le titre suit l'état, parce que la boîte change de rôle.
        // Tant que le programme attend, on y tape : « Entrée du
        // programme » est juste. Une fois qu'il a repris, elle ne
        // recueille plus rien — elle montre ce qu'il a écrit, et garder
        // le premier titre faisait lire une sortie sous l'étiquette
        // d'une entrée.
        let titre = if attend {
            s.input_dialog_title
        } else {
            s.input_dialog_title_done
        };
        egui::Window::new(titre)
            .id(egui::Id::new(INPUT_DIALOG))
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .current_pos(ctx.viewport_rect().center())
            .default_width(largeur)
            .min_width(largeur)
            .max_width(largeur)
            .show(ctx, |ui| {
                if chrome::window_head(ui, theme, s, titre) {
                    fermer = true;
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(10.0, ui.text_style_height(&egui::TextStyle::Body)),
                        egui::Sense::hover(),
                    );
                    let (couleur, phrase) = if attend {
                        (theme.accent, s.console_waiting.to_owned())
                    } else {
                        // Le programme a repris et n'attend plus : la
                        // boîte reste pour montrer ce qu'il a répondu,
                        // et dit où il en est.
                        let (texte, couleur, _) = self.status_line();
                        (couleur, texte)
                    };
                    ui.painter().circle_filled(rect.center(), 4.0, couleur);
                    ui.colored_label(couleur, egui::RichText::new(phrase).strong());
                });

                ui.add_space(8.0);
                ui.colored_label(
                    theme.dim,
                    if attend {
                        s.input_dialog_asked
                    } else {
                        s.input_dialog_answered
                    },
                );
                ui.add_space(2.0);
                // Ce que le programme vient d'écrire, sur le fond d'un
                // terminal : c'est sa voix, elle ne doit pas se confondre
                // avec celle de l'application.
                egui::Frame::new()
                    .fill(theme.bg)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .corner_radius(4)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        if invite.trim().is_empty() {
                            ui.colored_label(theme.dim, s.input_dialog_nothing_asked);
                        } else {
                            // Ce qui reste au-dessus, s'il en reste :
                            // une boîte qui montre la fin sans le dire
                            // laisse croire que le programme n'a écrit
                            // que ça.
                            if au_dessus > 0 {
                                ui.colored_label(
                                    theme.dim,
                                    format!(
                                        "{} {au_dessus} {}",
                                        s.input_dialog_above, s.decompiler_lines
                                    ),
                                );
                            }
                            // Les couleurs du programme, et non son
                            // texte brut : c'est ce qu'il montre.
                            let job = ansi::mise_en_page(
                                &vue,
                                egui::TextStyle::Monospace.resolve(ui.style()),
                                theme,
                            );
                            ui.label(job);
                        }
                    });

                // Les deux touches relevées avant le champ : un
                // `TextEdit` qui traite Entrée la retire de la file, si
                // bien que la guetter après lui ne verrait plus rien.
                let entrée_frappée = ui.input(|i| i.key_pressed(egui::Key::Enter));
                let échap_frappé = ui.input(|i| i.key_pressed(egui::Key::Escape));
                fermer |= échap_frappé;

                if attend {
                    ui.add_space(10.0);
                    let champ = ui.add(
                        egui::TextEdit::singleline(&mut self.input_dialog_text)
                            .hint_text(s.console_input_hint)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width()),
                    );
                    // Le curseur y va à l'ouverture, une fois : la boîte
                    // s'est ouverte parce qu'on nous demande quelque
                    // chose, et la seule chose à faire est d'écrire. Le
                    // redemander à chaque image empêcherait Échap de
                    // rendre le clavier.
                    if self.input_dialog_focus {
                        champ.request_focus();
                        self.input_dialog_focus = false;
                    }
                    envoyer |= entrée_frappée && (champ.has_focus() || champ.lost_focus());
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if attend {
                        // « Fin d'entrée » à gauche, loin du bouton
                        // qu'on vise : fermer l'entrée d'un programme
                        // est sans retour pour lui, et ce n'est pas ce
                        // qu'on venait faire.
                        if framed_button(ui, self.theme, s.console_eof)
                            .on_hover_text(s.console_eof_hint)
                            .clicked()
                        {
                            fin = true;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if attend {
                            // LE LISERÉ EST DE LA COULEUR DU FOND : on
                            // ne le voit pas, et il sert quand même.
                            // egui ajoute la largeur du trait à la
                            // taille d'un bouton tracé — sans lui,
                            // « Envoyer » faisait 25 px de haut et
                            // « Plus tard », posé huit pixels à sa
                            // gauche, 27. C'est le remède que
                            // `dialog_bar` porte déjà et que cette
                            // boîte-ci, qui refait sa barre d'action à
                            // la main, n'avait jamais reçu.
                            let envoi = egui::Button::new(
                                egui::RichText::new(s.console_send).color(theme.bg),
                            )
                            .fill(theme.accent)
                            .stroke(egui::Stroke::new(1.0, theme.accent))
                            .min_size(egui::vec2(110.0, 0.0));
                            envoyer |= ui.add(envoi).clicked();
                            if ui
                                .add(
                                    egui::Button::new(s.input_dialog_later)
                                        .stroke(egui::Stroke::new(1.0, theme.dim))
                                        .min_size(egui::vec2(110.0, 0.0)),
                                )
                                .on_hover_text(s.input_dialog_later_hint)
                                .clicked()
                            {
                                fermer = true;
                            }
                        } else {
                            // Plus rien à répondre : un seul bouton, et
                            // Entrée le presse.
                            // Même liseré invisible que ci-dessus :
                            // seul à l'écran aujourd'hui, ce bouton
                            // partagerait sa rangée demain, et deux
                            // pixels d'écart ne se voient que sur une
                            // mesure.
                            let ok =
                                egui::Button::new(egui::RichText::new(s.dialog_ok).color(theme.bg))
                                    .fill(theme.accent)
                                    .stroke(egui::Stroke::new(1.0, theme.accent))
                                    .min_size(egui::vec2(110.0, 0.0));
                            fermer |= ui.add(ok).clicked() || entrée_frappée;
                        }
                    });
                });
            });

        // Envoyer ne referme pas : c'est tout le point. Le programme
        // reprend, écrit sa réponse — « Incorrect password » —, et la
        // boîte la montre. Refermer aussitôt renvoyait devant une
        // console souvent repliée, où l'on ne voyait rien de ce qu'on
        // venait de provoquer.
        if envoyer {
            let ligne = std::mem::take(&mut self.input_dialog_text);
            self.send_input(Some(ligne));
        }
        if fin {
            self.send_input(None);
        }
        if fermer {
            self.show_input_dialog = false;
        }
    }

    /// Le côté « entrée » de la console.
    ///
    /// Une ligne validée part avec son saut de ligne, parce que c'est ce
    /// qu'un programme attend d'un terminal : il lit jusqu'au `\n`, et une
    /// ligne sans fin de ligne le laisserait croire qu'elle n'est pas
    /// terminée.
    pub(crate) fn console_input_ui(&mut self, ui: &mut egui::Ui, pending: usize) {
        let s = strings(self.lang);
        let waiting = matches!(self.status, Status::WaitingForInput);

        ui.separator();

        // Quand le programme attend, la ligne de saisie se distingue du
        // reste : elle est en bas d'un panneau lui-même en bas, sur un
        // fond de terminal, et l'on ne la trouvait pas — on cherchait
        // « où taper » en regardant la sortie.
        // Quand le programme attend, la zone doit se voir sans qu'on la
        // cherche : c'est le seul moment où l'application demande
        // quelque chose, et le manquer, c'est croire qu'elle a planté.
        // Un liseré d'un pixel et un fond à dix pour cent ne suffisaient
        // pas — sur un panneau déjà sombre, cela passait pour une nuance.
        let cadre = if waiting {
            egui::Frame::new()
                .fill(self.theme.accent.gamma_multiply(0.18))
                .stroke(egui::Stroke::new(2.0, self.theme.accent))
                .corner_radius(4)
                .inner_margin(egui::Margin::symmetric(8, 6))
        } else {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 2))
        };

        let mut send = false;
        let mut relancer = false;
        cadre.show(ui, |ui| {
            if waiting {
                ui.horizontal(|ui| {
                    // Une pastille pleine devant la phrase : l'oeil s'y pose
                    // avant de lire, et c'est ce qu'on veut d'un état qui ne
                    // dure que le temps d'une réponse.
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(10.0, ui.text_style_height(&egui::TextStyle::Body)),
                        egui::Sense::hover(),
                    );
                    ui.painter()
                        .circle_filled(rect.center(), 4.0, self.theme.accent);
                    ui.colored_label(
                        self.theme.accent,
                        egui::RichText::new(s.console_waiting).strong(),
                    );
                    ui.colored_label(self.theme.dim, s.console_waiting_how);
                });
            }

            // Les deux lignes — les arguments, puis l'entrée — sont bâties
            // pareil : un libellé de largeur commune, un champ qui prend ce
            // qui reste, des boutons calés à droite. Sans cette largeur
            // commune les deux champs commençaient à des abscisses
            // différentes ; sans le calage à droite, « Recharger » sortait
            // du cadre.
            let libellés = [s.console_args, s.console_input_label];
            let colonne = libellés
                .iter()
                .map(|texte| {
                    ui.painter()
                        .layout_no_wrap(
                            (*texte).to_owned(),
                            egui::TextStyle::Body.resolve(ui.style()),
                            egui::Color32::PLACEHOLDER,
                        )
                        .rect
                        .width()
                })
                .fold(0.0_f32, f32::max);

            // Les arguments : l'autre chose qu'un programme attend, et la
            // première pour un crackme, qui lit souvent son mot de passe
            // dans `argv[1]` plutôt que sur l'entrée standard. Ils ne se
            // tapent pas pendant qu'il tourne — ils sont posés sur la pile
            // avant la première instruction —, d'où le bouton qui recharge.
            // DÉPLIÉE SEULEMENT SI ON LA DEMANDE, par le bouton
            // « Arguments… » de la rangée d'onglets. Voir
            // `Settings::console_args_open` : toujours affichées, ces
            // deux rangées ne laissaient qu'une demi-ligne à la sortie.
            if self.console_args_open
                && self
                    .assemble_state
                    .as_ref()
                    .is_some_and(|état| état.from_binary.is_some())
            {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(colonne, ui.spacing().interact_size.y),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.colored_label(self.theme.dim, s.console_args);
                        },
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let appliquer = framed_button(ui, self.theme, s.console_args_apply)
                            .on_hover_text(s.console_args_apply_hint)
                            .clicked();
                        let réponse = ui.add(
                            egui::TextEdit::singleline(&mut self.program_args)
                                .hint_text(s.console_args_hint)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(ui.available_width()),
                        );
                        let entrée =
                            réponse.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        relancer = appliquer || entrée;
                    });
                });
                // L'environnement, replié : on s'en sert moins souvent que
                // des arguments, et une ligne de plus toujours ouverte
                // repousserait la saisie vers le bas.
                let ouvert = egui::CollapsingHeader::new(s.console_env)
                    .id_salt("console_env")
                    .open(Some(self.console_env_open))
                    .show(ui, |ui| {
                        ui.weak(s.console_env_hint);
                        let réponse = ui.add(
                            egui::TextEdit::multiline(&mut self.program_env)
                                .hint_text("SECRET=sesame")
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(2)
                                .desired_width(f32::INFINITY),
                        );
                        if réponse.lost_focus() {
                            relancer = true;
                        }
                    });
                if ouvert.header_response.clicked() {
                    self.console_env_open = !self.console_env_open;
                }
                ui.add_space(2.0);
            }
            ui.horizontal(|ui| {
                // La hauteur de la case DÉPASSE tout ce que la rangée porte.
                //
                // « Entrée : » se posait quatre pixels et demi au-dessus du
                // champ et des deux boutons : une rangée horizontale centre
                // chaque élément sur la hauteur qu'elle a À CET INSTANT et ne
                // recale pas ce qui est déjà posé. L'étiquette venant en
                // premier, elle se centrait sur elle-même. Imposer sa propre
                // hauteur n'y changeait rien — c'est la hauteur de la RANGÉE
                // qu'il faut fixer, et le seul moyen est que le premier
                // élément soit déjà le plus haut.
                //
                // TRENTE-QUATRE PIXELS, et c'est une mesure, non un calcul :
                // c'est la hauteur que la rangée prend d'elle-même une fois
                // le champ et les deux boutons posés. Essayé 27, 30 et 34 —
                // à 27 l'étiquette reste 3,5 px trop haut, à 30 il en reste
                // 2, à 34 elle tombe pile.
                //
                // Un nombre mesuré n'est acceptable que si quelque chose le
                // retient : l'invariant « une rangée de commandes se lit sur
                // une seule ligne » le fait, et dira le jour où il cessera
                // d'être juste.
                let rangée = ui.spacing().interact_size.y + 9.0;
                ui.allocate_ui_with_layout(
                    egui::vec2(colonne, rangée),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_height(rangée);
                        // En accent et en gras pendant l'attente : c'est la
                        // ligne où il faut taper, et rien d'autre à l'écran
                        // ne le dit.
                        if waiting {
                            ui.colored_label(
                                self.theme.accent,
                                egui::RichText::new(s.console_input_label).strong(),
                            );
                        } else {
                            ui.colored_label(self.theme.dim, s.console_input_label);
                        }
                    },
                );
                // Les boutons sont posés en premier, calés à droite, et le
                // champ prend ce qui reste. Deviner sa largeur (« la place
                // disponible moins 120 px ») la surestimait de quelques
                // pixels : la ligne débordait du panneau latéral, et ce
                // débordement était repris comme sa nouvelle largeur à la
                // frame suivante. L'onglet Console élargissait donc le
                // panneau sans fin, jusqu'à ce qu'il occupe la fenêtre.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Les deux touches relevées **avant** le champ, et sans
                    // les consommer : un `TextEdit` qui traite Entrée la
                    // retire de la file, si bien que la guetter après lui ne
                    // voyait plus rien. Le champ gardait le texte, le
                    // programme continuait d'attendre, et il fallait cliquer
                    // « Envoyer » — ce qui est exactement ce qu'on voulait
                    // éviter.
                    let entrée_frappée = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let échap_frappé = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    // Fin d'entrée : un programme qui lit jusqu'au bout de son
                    // entrée attendrait indéfiniment sans ce bouton.
                    if framed_button(ui, self.theme, s.console_eof)
                        .on_hover_text(s.console_eof_hint)
                        .clicked()
                    {
                        self.send_input(None);
                    }
                    send = framed_button(ui, self.theme, s.console_send).clicked();

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.console_input)
                            .hint_text(s.console_input_hint)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width()),
                    );
                    // Le programme réclame de quoi lire : le curseur va là où
                    // l'utilisateur doit taper, sans qu'il ait à le chercher.
                    // Échap le lui reprend — sans cette porte, le champ
                    // gardait toutes les touches tant que durait l'attente,
                    // et l'éditeur ne répondait plus : « i » s'écrivait ici
                    // au lieu de passer en Insertion.
                    //
                    // `lost_focus` et non `has_focus` : egui rend déjà le
                    // focus du champ sur Échap. Le guetter ici sur le focus
                    // encore détenu ne voyait donc rien, et la reprise plus
                    // bas le récupérait dans la même frame — la touche
                    // n'avait aucun effet visible.
                    if response.lost_focus() && échap_frappé {
                        self.console_input_released = true;
                        // Personne ne prend le focus derrière : c'est
                        // l'absence de champ focalisé qui réarme les
                        // commandes de l'éditeur.
                        self.editor_mode = EditorMode::Normal;
                    } else if response.has_focus() {
                        // Revenu dedans de son plein gré : l'attente suivante
                        // pourra de nouveau l'y amener.
                        self.console_input_released = false;
                    } else if waiting && !self.console_input_released && !self.show_input_dialog {
                        // Pas tant que la boîte est ouverte : c'est elle qui
                        // demande, et deux champs qui réclament le clavier
                        // dans la même image se l'arrachent.
                        response.request_focus();
                    }
                    // Le champ a le focus et l'on vient de frapper Entrée :
                    // c'est un envoi. On ne demande pas `lost_focus`, qui
                    // dépend de ce qu'egui a décidé de faire du focus, ni
                    // `key_pressed` après coup, qui ne voit plus rien.
                    send |= entrée_frappée && (response.has_focus() || response.lost_focus());
                });
            });

            if pending > 0 {
                ui.colored_label(
                    self.theme.dim,
                    if self.lang == Lang::Fr {
                        format!("{pending} octet(s) en attente d'être lus.")
                    } else {
                        format!("{pending} byte(s) waiting to be read.")
                    },
                );
            } else if !waiting && !self.program_reads {
                // Le champ est là, mais rien ne le lira tel que le programme
                // est écrit. Le dire plutôt que de faire disparaître le
                // champ : celui qui prépare son entrée avant d'avoir tapé
                // son `read` doit pouvoir la taper, et comprendre pourquoi
                // elle attend.
                ui.colored_label(self.theme.dim, s.console_no_read);
            }
        });

        // Reposer les arguments demande de refaire la pile : c'est un
        // rechargement, et le dire vaut mieux que de le faire en
        // silence sous un programme à demi exécuté.
        if relancer {
            self.do_reset();
        }
        if send {
            let line = std::mem::take(&mut self.console_input);
            self.send_input(Some(line));
        }
    }

    /// Vue d'ensemble des crates moteur (core/asm/disasm/emu/re), un peu
    /// comme un gestionnaire de plugins — namsdec-exercises n'y figure pas :
    /// il deviendra un module séparé, ajoutable indépendamment.
    pub(crate) fn modules_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let fr = self.lang == Lang::Fr;
        // La largeur que les dix cartes partageront, relevée avant la
        // première : voir `module_card_width`.
        let large = module_card_width(ui);

        module_card(
            ui,
            "namsdec-core",
            namsdec_core::VERSION,
            true,
            theme.good,
            s.module_core_detail,
            large,
        );

        let (asm_ok, asm_status) = match &self.nasm_info {
            Some(info) => (
                true,
                if fr {
                    format!("nasm détecté — {}", info.version)
                } else {
                    format!("nasm detected — {}", info.version)
                },
            ),
            None => (
                false,
                if fr {
                    "nasm introuvable dans le PATH".to_string()
                } else {
                    "nasm not found in PATH".to_string()
                },
            ),
        };
        module_card(
            ui,
            "namsdec-asm",
            namsdec_asm::VERSION,
            asm_ok,
            if asm_ok { theme.good } else { theme.bad },
            &asm_status,
            large,
        );

        let disasm_status = match &self.assemble_state {
            Some(state) => {
                if fr {
                    format!(
                        "{} instructions désassemblées (vérification)",
                        state.disasm.len()
                    )
                } else {
                    format!(
                        "{} instructions disassembled (verification)",
                        state.disasm.len()
                    )
                }
            }
            None => s.module_disasm_idle.to_string(),
        };
        module_card(
            ui,
            "namsdec-disasm",
            namsdec_disasm::VERSION,
            self.assemble_state.is_some(),
            theme.accent,
            &disasm_status,
            large,
        );

        let emu_status = match &self.emulator {
            Some(emulator) => {
                let state_word = match (emulator.cpu().halted, fr) {
                    (true, true) => "arrêté (hlt)",
                    (true, false) => "halted (hlt)",
                    (false, true) => "en cours",
                    (false, false) => "running",
                };
                let bp_word = if fr {
                    "point(s) d'arrêt"
                } else {
                    "breakpoint(s)"
                };
                format!(
                    "RIP={:#x} · {} {bp_word} · {state_word}",
                    emulator.cpu().regs.rip,
                    emulator.breakpoints().len(),
                )
            }
            None => s.no_program_loaded.to_string(),
        };
        module_card(
            ui,
            "namsdec-emu",
            namsdec_emu::VERSION,
            self.emulator.is_some(),
            theme.accent,
            &emu_status,
            large,
        );

        let titles = self.re_registry.titles(self.lang.code());
        // LE COMPTE, PAS LA LISTE — le repli juste en dessous la
        // donne, une ligne par module, et le commentaire qui l'y a mise
        // dit déjà « un par ligne PLUTÔT QU'en énumération dans la
        // carte ». L'énumération était restée : la carte écrivait les
        // vingt-six titres séparés par des virgules, et le repli les
        // réécrivait aussitôt. Vu sur une capture.
        //
        // Et le pluriel se décide, puisqu'on compte : « module(s) »
        // devant un nombre connu est une parenthèse qu'on peut fermer.
        let re_status = if titles.is_empty() {
            s.no_re_module_registered.to_string()
        } else {
            let pluriel = if titles.len() > 1 { "s" } else { "" };
            if fr {
                format!("{} module{pluriel} d'analyse", titles.len())
            } else {
                format!("{} analysis module{pluriel}", titles.len())
            }
        };
        module_card(
            ui,
            "namsdec-re",
            namsdec_re::VERSION,
            true,
            theme.good,
            &re_status,
            large,
        );

        // Les modules d'analyse, un par ligne plutôt qu'en énumération
        // dans la carte : à neuf titres séparés par des virgules, on ne
        // lisait plus lequel était là ni ce qu'il fait.
        if !titles.is_empty() {
            ui.add_space(2.0);
            // Repliés : neuf lignes de plus allongeaient la page d'autant,
            // et l'on vient d'abord y voir l'état des modules, non la
            // liste de ce que l'analyse sait faire.
            egui::CollapsingHeader::new(format!("{} ({})", s.modules_re_detail, titles.len()))
                .id_salt("modules_re_detail")
                .show(ui, |ui| {
                    for titre in &titles {
                        ui.horizontal(|ui| {
                            status_dot(ui, theme.good);
                            ui.label(*titre);
                        });
                    }
                });
            ui.add_space(6.0);
        }

        // Ce que namsdec porte en propre, et qui n'avait aucune carte :
        // l'explorateur, la documentation, les exercices, les thèmes et
        // la recherche de mise à jour. L'onglet annonçait les modules de
        // namsdec et en montrait la moitié.
        module_card(
            ui,
            "namsdec-explorer",
            namsdec_explorer::VERSION,
            self.explorer.root.is_some(),
            theme.accent,
            &match self.explorer.root.as_ref() {
                Some(racine) => format!("{} {}", s.modules_explorer_root, racine.display()),
                None => s.modules_explorer_none.to_string(),
            },
            large,
        );

        module_card(
            ui,
            "namsdec-doc",
            namsdec_doc::VERSION,
            true,
            theme.good,
            &if fr {
                format!("{} fiches d'instructions", namsdec_doc::count())
            } else {
                format!("{} instruction entries", namsdec_doc::count())
            },
            large,
        );

        let exercices = self
            .exercise_packs
            .iter()
            .map(|pack| pack.exercises.len())
            .sum::<usize>();
        module_card(
            ui,
            "namsdec-exercises",
            namsdec_exercises::VERSION,
            self.exercises,
            theme.accent,
            &if self.exercises {
                if fr {
                    format!(
                        "{} série(s), {exercices} exercice(s)",
                        self.exercise_packs.len()
                    )
                } else {
                    format!(
                        "{} pack(s), {exercices} exercise(s)",
                        self.exercise_packs.len()
                    )
                }
            } else {
                s.modules_inactive.to_string()
            },
            large,
        );

        let thèmes = namsdec_theme::all().len();
        module_card(
            ui,
            "namsdec-theme",
            namsdec_theme::VERSION,
            true,
            theme.good,
            &if fr {
                format!(
                    "{thèmes} thème(s), dont {} venu(s) de fichiers",
                    self.theme_files.len()
                )
            } else {
                format!(
                    "{thèmes} theme(s), {} of them from files",
                    self.theme_files.len()
                )
            },
            large,
        );

        let (maj_ok, maj_couleur, maj_texte) = match &self.update_result {
            Some(Ok(verdict)) => (true, theme.good, format!("{verdict:?}")),
            Some(Err(raison)) => (false, theme.bad, raison.clone()),
            None => (false, theme.dim, s.modules_update_idle.to_string()),
        };
        module_card(
            ui,
            "namsdec-update",
            namsdec_update::VERSION,
            maj_ok,
            maj_couleur,
            &maj_texte,
            large,
        );

        // Et ce qui vient de fichiers : thèmes et outils déposés dans le
        // dossier des greffons. Ils ne figuraient que dans l'onglet
        // Plugins, alors qu'ils sont chargés au même titre que le reste.
        ui.add_space(4.0);
        ui.strong(s.modules_plugins);
        ui.weak(s.modules_plugins_hint);
        ui.add_space(2.0);
        let mut venus_de_fichiers = false;
        for (id, chemin) in &self.theme_files {
            venus_de_fichiers = true;
            ui.horizontal(|ui| {
                status_dot(ui, theme.accent);
                ui.label(id);
                ui.weak(chemin.display().to_string());
            });
        }
        for outil in &self.decompilers {
            venus_de_fichiers = true;
            ui.horizontal(|ui| {
                status_dot(ui, theme.accent);
                ui.label(&outil.name);
                ui.weak(&outil.command);
            });
        }
        if !venus_de_fichiers {
            ui.weak(s.modules_none_loaded);
        }
    }
}
