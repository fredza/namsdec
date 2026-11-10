//! L'Établi : la disposition du mode simple.
//!
//! **Ce que le mode débutant était, et pourquoi cela ne suffisait pas.**
//! `Niveau::Débutant` FILTRAIT l'interface complète — neuf rubriques
//! d'analyse sur vingt-cinq, cinq onglets de vues sur neuf, un éditeur
//! sans modes — mais le squelette restait celui de l'analyste : deux
//! rails d'icônes, deux rangées d'onglets, une barre de menus, une
//! barre d'état, une console. Quelqu'un qui ouvre namsdec pour
//! apprendre voyait l'établi d'un professionnel avec des tiroirs vides,
//! ce qui est moins engageant qu'un établi plus petit.
//!
//! **Ce que l'Établi fait à la place.** Quatre bandes fixes, et c'est
//! tout : ce qu'on a écrit, ce que fait la machine, ce que fait la
//! ligne où l'on est, le résultat. Aucun menu déroulant, aucun rail,
//! aucune barre d'état. **Rien à ouvrir avant de pouvoir travailler**,
//! et rien qui se survole pour savoir ce que c'est : chaque bande porte
//! son titre, écrit à la première personne, parce que c'est le
//! programme de celui qui lit.
//!
//! **Ce que cela coûte, et c'est assumé** : la place est répartie
//! d'avance. Le code n'aura jamais toute la largeur, et la machine
//! jamais plus de [`MACHINE_LARGEUR`]. C'est le prix de « rien à
//! chercher », et il se paie surtout sur un petit écran.
//!
//! **Ce qui est gardé de l'interface complète, et pourquoi.** La rangée
//! des vues reste : au niveau débutant, elle porte le désassemblage, le
//! déroulement, l'analyse et les points d'arrêt — quatre façons de
//! regarder que ce même niveau juge compréhensibles sans rien savoir
//! d'avance (voir `VueCentrale::niveau`). Les retirer aurait fait un
//! mode simple plus pauvre que le mode débutant qu'il remplace, ce qui
//! n'était pas la demande. La barre des fichiers reste aussi, mais
//! **seulement quand il y a plus d'un fichier ouvert** : une rangée
//! d'onglets qui n'en porte qu'un ne dit rien que le nom déjà écrit
//! dans la barre du haut.
//!
//! Choisi le 9 novembre 2026 parmi trois maquettes
//! (`mockups/2026-11-09/`), avec la Scène gardée en bouton — voir
//! `scene.rs`.

use super::*;

/// La largeur VOULUE du volet de la machine.
///
/// Un souhait, pas une loi : la largeur réellement posée ne descend
/// jamais sous [`sidebar_min_width`], qui mesure ce qu'il faut pour
/// qu'une paire de registres tienne en entier. Fixée à 330 pixels sans
/// ce plancher, la première capture du banc montrait les noms coupés à
/// gauche — « X » pour RAX — et seize chiffres hexadécimaux qui
/// sortaient de l'écran à droite. Un registre dont il manque un chiffre
/// ne vaut rien, et c'est la règle de ce panneau depuis toujours : toute
/// la largeur, jamais de barre de défilement horizontale.
const MACHINE_LARGEUR: f32 = 330.0;

/// Ce que la bande du résultat prend au moins, même vide.
///
/// Elle ne doit pas changer de hauteur selon qu'elle a une ligne ou
/// trois à dire : la bande d'explication juste au-dessus se déplacerait
/// à chaque pas, et c'est elle qu'on lit.
const RESULTAT_MIN: f32 = 96.0;

/// Ce que la bande du résultat prend quand rien n'a encore été tiré.
///
/// Plus que son minimum : la sortie a son cadre, sa zone défilante et
/// sa rangée de saisie, et au minimum elle n'en montrait qu'une demie.
///
/// **Deux cents et non cent cinquante** (10 novembre 2026, retour
/// d'écran : « la console n'est pas assez visible »). À cent cinquante,
/// la sortie montrait deux lignes sous sa rangée de saisie, et le
/// « bonjour » qu'un premier programme écrit — ce qu'on attendait de
/// toute la séance — se lisait comme une note de bas de page.
const RESULTAT_DEFAUT: f32 = 200.0;

/// Ce que la bande du résultat prend au plus.
const RESULTAT_MAX: f32 = 320.0;

/// Ce que la bande de la ligne courante prend au plus.
///
/// Elle s'ajuste à ce qu'elle porte — deux lignes devant un `nop`, six
/// devant un saut conditionnel détaillé —, mais elle ne prend pas la
/// place du code : au-delà, c'est elle qui se serre.
const LIGNE_MAX: f32 = 190.0;

/// Ce que la bande de l'exercice prend quand elle paraît.
///
/// MESURÉE, et non choisie. À cent soixante-quatre pixels — le chiffre
/// qu'on avait écrit d'instinct —, la capture montrait la dernière
/// ligne coupée des DEUX formes d'exercice : le budget d'instructions
/// pour celui d'écriture, « exécutez pour voir » pour celui de lecture.
/// Le contenu demande jusqu'à 136 px sur un écran de 1600, et il faut y
/// ajouter le titre, les marges et l'air entre les deux : 190 les
/// couvre, avec de quoi ne pas se retrouver à un pixel près le jour où
/// une condition de plus s'ajoute.
///
/// Sur un écran étroit, l'énoncé prend plus de lignes et la bande
/// défile : c'est à cela que servent sa barre et sa poignée.
const EXERCICE_DEFAUT: f32 = 190.0;

/// Ce qu'elle prend au moins, quand on tire sa poignée vers le haut.
///
/// Son titre et une ligne : en dessous, on ne lit plus l'énoncé, on
/// devine qu'il y en a un.
const EXERCICE_MIN: f32 = 76.0;

/// Ce qu'elle prend au plus.
const EXERCICE_MAX: f32 = 320.0;

/// Ce qu'on garde à la sortie quand des problèmes la précèdent.
///
/// Son cadre, sa zone défilante et sa rangée de saisie : en dessous,
/// elle montre une demi-ligne, ce qui est pire que rien.
const SORTIE_MIN: f32 = 100.0;

impl NamsdecApp {
    /// Tout l'écran du mode simple.
    ///
    /// L'ordre des panneaux EST la mise en page : chacun prend la place
    /// qui reste au moment où il est posé. Les deux bandes du bas
    /// passent donc avant le volet de la machine — elles tiennent toute
    /// la largeur, et la machine s'arrête au-dessus d'elles, comme sur
    /// la maquette retenue.
    pub(crate) fn etabli_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;

        egui::Panel::top("etabli_barre")
            .frame(
                egui::Frame::new()
                    .fill(theme.panel)
                    .inner_margin(egui::Margin::symmetric(12, 7)),
            )
            .show(ui, |ui| self.etabli_barre_ui(ui));

        egui::Panel::top("etabli_frise")
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(14, 5)))
            .show(ui, |ui| self.etabli_frise_ui(ui));

        // LA BANDE DE L'EXERCICE, quand il y en a un en cours.
        //
        // Elle n'existait pas, et c'est le trou que ce mode avait : au
        // mode avancé, l'énoncé et le but vivent dans le panneau
        // latéral, qui est TOUJOURS LÀ. L'Établi n'a pas de panneau
        // latéral. On ouvrait donc « Exercices… », on chargeait une
        // étape, on refermait la fenêtre — et l'on se retrouvait seul
        // devant un programme à trous, sans l'énoncé, sans le but, sans
        // savoir si l'on avait réussi. Le parcours se lisait comme un
        // catalogue qu'on consulte, au lieu de se faire.
        //
        // EN HAUT ET SUR TOUTE LA LARGEUR : c'est la consigne, elle se
        // lit avant le code, et un énoncé de quatre cents caractères
        // tombe sur six lignes dans la colonne du code quand il en fait
        // deux ici. Elle passe donc avant le volet de la machine, comme
        // les deux bandes du bas.
        //
        // RÉGLABLE, comme la bande du résultat : un énoncé court n'a
        // pas besoin de cent soixante pixels, et l'on veut parfois voir
        // le but pendant qu'on lit beaucoup de code.
        if self.exercices_en_bande() {
            egui::Panel::top("etabli_exercice")
                .resizable(true)
                // UNE PART DE CE QUI RESTE, ET NON UN NOMBRE DE
                // PIXELS. C'est la différence entre lire son programme
                // et ne pas le lire.
                //
                // Cent quatre-vingt-dix pixels sont un sixième d'un
                // écran de 1100 et plus d'un tiers d'un écran de 520.
                // Compté en LIGNES DE PROGRAMME entièrement visibles à
                // 800x520, le plus petit écran que l'application
                // accepte : avec les 190 fixes, ZÉRO. Avec la part,
                // deux. Au-dessus de 860 pixels de place disponible
                // elle ne mord plus et la bande prend ses 190.
                //
                // DESCENDRE PLUS BAS NE DONNE RIEN — un septième au
                // lieu d'un cinquième, toujours deux lignes —, et la
                // raison est quinze lignes plus bas : la bande du
                // résultat prend « 45 % de ce qui reste », une part
                // elle aussi, et reprend ce que celle-ci rend avant
                // que le code n'en voie la couleur. Le seul levier qui
                // compte ensuite est cette part-là, dont la hauteur
                // vient d'une demande explicite (A 9.84, « la console
                // n'est pas assez visible ») : la toucher est un
                // arbitrage pour l'utilisateur, pas pour nous.
                //
                // Retenu par
                // `au_plus_petit_ecran_l_etabli_montre_encore_du_code`,
                // qui compte des lignes et non des pixels.
                .default_size(
                    EXERCICE_DEFAUT
                        .min(ui.available_height() * 0.22)
                        .max(EXERCICE_MIN),
                )
                .min_size(EXERCICE_MIN)
                .max_size(EXERCICE_MAX.min((ui.available_height() * 0.4).max(EXERCICE_MIN)))
                .frame(
                    egui::Frame::new()
                        .fill(theme.panel)
                        .inner_margin(egui::Margin::symmetric(14, 8)),
                )
                .show(ui, |ui| self.etabli_exercice_ui(ui));
        }

        // La bande du résultat, tout en bas : elle est la dernière
        // chose qu'on lit, et elle est à la place où on la cherche.
        //
        // Sauf si on l'a détachée : elle est alors dans une fenêtre à
        // elle, et ce qui reste ici irait sous une bande vide.
        if !self.etabli_resultat_detache {
            // CE QUE LA RANGÉE DE SAISIE RÉCLAME, ET QUI NE SE NÉGOCIE
            // PAS.
            //
            // Relevé à l'écran le 10 novembre 2026, une flèche verte
            // sur « Variables d'environnement » : « innaccesible ». La
            // bande portait, de haut en bas, les notes du binaire, la
            // sortie du programme, puis les rangées du bas — les
            // arguments, l'environnement, et LE CHAMP OU L'ON REPOND AU
            // PROGRAMME. Ces rangées-la sont dans un panneau ancré au
            // bas de la sortie : quand la bande est trop courte pour
            // tout porter, elles passent sous son bord, c'est-à-dire
            // sous le bord de la fenêtre. Le programme demandait
            // « Enter your name: » et il n'y avait plus nulle part où
            // le lui dire.
            //
            // La console du mode avancé se donne déjà ce plancher
            // (`console_input_estimate`, voir son appel dans
            // `console.rs`) ; la bande, non. Elle le prend maintenant,
            // et par la même mesure : deux formules pour une même
            // rangee finiraient par compter deux choses.
            //
            // LES DEUX MESURES, et la plus grande : l'estimation dit ce
            // que les rangées valent AVANT de les peindre, le relevé de
            // l'image précédente dit ce qu'elles ont vraiment pris —
            // l'environnement déplié, par exemple.
            let saisie = self
                .console_input_estimate(ui)
                .max(self.console_input_height.unwrap_or(0.0));
            // ET CE PLANCHER NE PASSE PAS AVANT LE RESTE DE L'ÉCRAN.
            //
            // Premier jet : le plancher l'emportait sur la part de
            // 45 %. À 800x520 il la dépassait de trente-trois pixels,
            // pris à la bande de la ligne courante — qui, elle, ne
            // défile pas : son contenu passait PAR-DESSUS la rangée
            // des vues, « Le programme est terminé. » peint sur
            // « Source ». Attrapé par
            // `panels_do_not_overlap_on_the_smallest_screen`.
            //
            // La garantie vaut donc là où il y a la place, et c'est
            // honnête : à 800x520, cinq bandes et un volet ne tiennent
            // pas, et la rangée de saisie s'y coupe comme le reste.
            // Mieux vaut une promesse tenue sur les écrans ordinaires
            // qu'une promesse qui casse la mise en page du plus petit.
            let plafond_naturel =
                RESULTAT_MAX.min((ui.available_height() * 0.45).max(RESULTAT_MIN));
            let plancher = RESULTAT_MIN.max(saisie + SORTIE_MIN).min(plafond_naturel);
            // Le plafond ne descend jamais sous le plancher : une borne
            // haute inférieure à la borne basse laisse egui choisir, et
            // ce qu'il choisit n'est pas ce qu'on voulait.
            let plafond = plafond_naturel.max(plancher);
            egui::Panel::bottom("etabli_resultat")
                .resizable(true)
                .default_size(RESULTAT_DEFAUT.max(plancher))
                .min_size(plancher)
                .max_size(plafond)
                .frame(
                    egui::Frame::new()
                        .fill(theme.bg)
                        .inner_margin(egui::Margin::symmetric(14, 8)),
                )
                .show(ui, |ui| self.etabli_resultat_ui(ui));
        }

        // Ce que fait la ligne où l'on est : entre le code et le
        // résultat, parce que c'est là qu'on regarde en avançant.
        // Bornée, et SANS zone défilante à l'intérieur.
        //
        // Une `ScrollArea` bornée à 150 pixels y avait été posée pour
        // la retenir — et elle empêchait purement et simplement le bloc
        // « Dernier pas » de se peindre : le contrôle de la prévision
        // (`la_réponse_attend_le_pas_au_niveau_débutant`) ne le
        // trouvait plus à l'écran, alors que l'application le tenait
        // bien en mémoire. Mesurée dans un panneau qui s'ajuste à son
        // contenu, une zone défilante ne sait pas quelle hauteur
        // demander, et elle montrait la première moitié de ce qu'elle
        // avait.
        //
        // Le panneau, lui, sait se borner : il prend la hauteur de ce
        // qu'il porte, jusqu'à [`LIGNE_MAX`].
        // SA BORNE RESTE UN NOMBRE DE PIXELS, et ce n'est pas un
        // oubli. On a essayé de la mettre en part de ce qui reste,
        // comme ses deux sœurs, pour rendre au code une ligne de plus
        // à 800x520 — et le banc a refusé : à cette taille, la bande
        // du résultat se retrouvait posée SUR celle-ci, « RÉSULTAT »
        // peint par-dessus le numéro de ligne 13. Une ligne de code
        // gagnée ne paie pas deux panneaux qui se recouvrent, et l'on
        // ne garde pas une borne dont on ne sait pas expliquer l'effet.
        // Voir `panels_do_not_overlap_on_the_smallest_screen`.
        egui::Panel::bottom("etabli_ligne")
            .max_size(LIGNE_MAX)
            .frame(
                egui::Frame::new()
                    .fill(theme.panel)
                    .inner_margin(egui::Margin::symmetric(14, 8)),
            )
            .show(ui, |ui| self.etabli_ligne_ui(ui));

        if !self.etabli_machine_detachee {
            // Le volet de la machine : assez large pour les registres, et
            // tiré à la main si l'on en veut plus. `min_size` ne borne que
            // la poignée — une largeur retenue d'une session précédente
            // passerait dessous sans être corrigée —, donc on la relève
            // nous-mêmes, comme le fait le panneau latéral du mode avancé.
            let machine_minimum = sidebar_min_width(ui);
            let id_machine = egui::Id::new("etabli_machine");
            if let Some(état) = egui::containers::panel::PanelState::load(ui.ctx(), id_machine)
                && état.outer_rect.width() < machine_minimum
            {
                let corrigé = egui::containers::panel::PanelState {
                    outer_rect: egui::Rect::from_min_size(
                        état.outer_rect.min,
                        egui::vec2(machine_minimum, état.outer_rect.height()),
                    ),
                };
                ui.ctx()
                    .data_mut(|data| data.insert_persisted(id_machine, corrigé));
            }
            egui::Panel::right(id_machine)
                .resizable(true)
                .default_size(MACHINE_LARGEUR.max(machine_minimum))
                .min_size(machine_minimum)
                .max_size((ui.available_width() * 0.5).max(machine_minimum))
                .frame(
                    egui::Frame::new()
                        .fill(theme.panel)
                        .inner_margin(egui::Margin::symmetric(10, 8)),
                )
                .show(ui, |ui| self.etabli_machine_ui(ui));
        }

        // La zone de travail, relevée ICI comme au mode avancé : les
        // bandes sont posées, et ce qui reste est la place du code.
        // C'est elle qu'une fenêtre agrandie occupe.
        self.zone_de_travail = ui.available_rect_before_wrap();

        // La barre des fichiers, seulement s'il y en a plusieurs.
        if self.tabs.len() > 1 {
            egui::Panel::top("etabli_fichiers")
                .frame(egui::Frame::new().fill(theme.bg))
                .show(ui, |ui| self.tabs_bar_ui(ui));
        }
        egui::Panel::top("etabli_vues")
            .frame(egui::Frame::new().fill(theme.bg))
            .show(ui, |ui| self.views_bar_ui(ui));

        // Le titre de la bande du code est posé au-dessus du listing et
        // non dedans : il ne doit pas défiler avec lui.
        egui::Panel::top("etabli_titre_code")
            .frame(egui::Frame::new().inner_margin(egui::Margin {
                left: 14,
                right: 14,
                top: 4,
                bottom: 4,
            }))
            .show(ui, |ui| {
                titre_de_bande(ui, theme, s.etabli_code);
            });

        self.vue_centrale_ui(ui);
    }

    /// La barre du haut : le fichier, les cinq gestes, et de quoi
    /// sortir.
    ///
    /// UNE RANGÉE OU DEUX, selon la place. Sur un écran large, tout
    /// tient sur une ligne : les gestes à gauche, ce qui sort du mode
    /// à droite. À 800 pixels de large, le banc a montré « Pas à pas »
    /// peint SUR « Ouvrir un fichier… » — un alignement à droite prend
    /// ce qui reste, et quand il ne reste rien il passe par-dessus.
    /// La barre se coupe donc en deux rangées, et rien ne se recouvre.
    fn etabli_barre_ui(&mut self, ui: &mut egui::Ui) {
        // Le seuil est mesuré, non deviné : à 800 de large les deux
        // moitiés se chevauchent de vingt-cinq pixels, et la marge
        // qu'on prend ici est celle d'un libellé plus long dans une
        // autre langue.
        // Mille cent, et non neuf cents : la barre a gagné les trois
        // pastilles de fenêtre à gauche, la calculatrice et
        // l'engrenage à droite (A 9.84), et le banc l'a vue se
        // recouvrir de nouveau à 800 px. Le seuil suit ce que la barre
        // PORTE, et il est à remonter chaque fois qu'on y ajoute
        // quelque chose — ce que `panels_do_not_overlap` dit tout seul.
        const UNE_SEULE_RANGÉE: f32 = 1100.0;
        if ui.available_width() >= UNE_SEULE_RANGÉE {
            ui.horizontal(|ui| {
                self.etabli_identite_ui(ui);
                ui.add_space(10.0);
                self.etabli_gestes_ui(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.etabli_sorties_ui(ui);
                });
            });
            return;
        }
        // TROIS RANGÉES, UN GROUPE PAR RANGÉE.
        //
        // Deux rangées ne suffisaient plus : l'identité et les sept
        // portes se partageaient la première, et à 800 px « sans
        // titre » se peignait sur « Ouvrir un fichier… ». Chaque
        // groupe a donc sa ligne — c'est trente pixels de plus, et
        // seulement sur une fenêtre où la barre ne tenait de toute
        // façon pas.
        ui.horizontal(|ui| self.etabli_identite_ui(ui));
        ui.add_space(5.0);
        ui.horizontal(|ui| self.etabli_gestes_ui(ui));
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.etabli_sorties_ui(ui);
            });
        });
    }

    /// Les pastilles de fenêtre, le nom de l'application, celui du
    /// fichier.
    ///
    /// LES PASTILLES ÉTAIENT ABSENTES, et c'est un oubli franc : le
    /// mode simple n'a pas de barre de menus, or c'est elle qui les
    /// portait. On ne pouvait donc plus fermer la fenêtre depuis
    /// l'application — seulement par le gestionnaire de fenêtres, que
    /// namsdec a justement cessé d'afficher. Elles sont ici, au coin
    /// haut gauche, exactement comme au mode avancé : la fermeture
    /// occupe l'angle qu'une souris lancée vers le haut à gauche
    /// atteint sans viser.
    fn etabli_identite_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        if !self.system_titlebar {
            let maximisée = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            if let Some(bouton) = chrome::window_buttons_ui(ui, s, maximisée) {
                self.window_button(bouton);
            }
            ui.add_space(6.0);
        }
        ui.label(
            egui::RichText::new("namsdec")
                .color(theme.text)
                .size(14.0)
                .strong(),
        );
        // Le nom du fichier, et son point quand il est modifié. C'est
        // la seule chose que la barre de titre disait et que cette
        // barre-ci doit redire : au mode simple, elle n'est plus là.
        let nom = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| s.untitled_buffer.to_owned());
        // ABRÉGÉ PAR LA FIN QUAND IL EST LONG.
        //
        // Demandé à l'écran le 10 novembre 2026, une flèche verte sur
        // ce nom : « si le nom est trop long le concatener ou premiers
        // caractères puis ... ». Un nom de fichier n'a pas de limite,
        // et celui-ci ouvre une rangée qui porte ensuite CINQ
        // commandes : laissé entier, il les repousse — et au-delà, il
        // pousse la barre sur deux lignes ou passe par-dessus ce qui
        // s'aligne à droite. C'est le defaut qu'on a déjà corrigé deux
        // fois aujourd'hui, à la frise et au titre de l'exercice.
        //
        // ON COUPE LE NOM, PAS L'EXTENSION ni la marque de
        // modification : l'extension dit de quoi il s'agit, et le point
        // dit qu'on n'a pas enregistré — deux choses qu'on perdrait en
        // coupant bêtement par la droite.
        //
        // RELISIBLE AU SURVOL, comme partout ailleurs.
        let abrégé = nom_abrégé(&nom, NOM_MAX);
        let marque = if self.modifié_vu {
            format!("{abrégé} •")
        } else {
            abrégé.clone()
        };
        let étiquette = ui.label(
            egui::RichText::new(marque)
                .color(theme.dim)
                .monospace()
                .size(13.0),
        );
        if abrégé != nom {
            étiquette.on_hover_text(&nom);
        }
    }

    /// Ce qui mène ailleurs : ouvrir, enregistrer, l'aide, et la sortie
    /// du mode simple.
    ///
    /// Posé de droite à gauche, donc écrit dans cet ordre-là.
    fn etabli_sorties_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // TOUS LES QUATRE SONT ENCADRÉS, et de la même façon : hors
        // d'un menu, un bouton doit se voir sans qu'on le survole
        // (« outside_the_menus_a_button_shows_itself »). Ils font donc
        // vingt-sept pixels quand les gestes en font vingt-cinq — ce
        // qui est permis, et même voulu : les deux groupes sont
        // séparés par toute la largeur de la barre, ou par une rangée
        // entière quand elle se coupe en deux. Ce qu'on refuse, c'est
        // deux hauteurs CÔTE À CÔTE.
        let theme = self.theme;
        if framed_button(ui, theme, s.etabli_advanced)
            .on_hover_text(s.etabli_advanced_hint)
            .clicked()
        {
            self.debutant = false;
        }
        // L'ENGRENAGE DES RÉGLAGES. Une icône et non un mot : c'est le
        // seul bouton de cette barre qui ne mène pas à un geste du
        // travail en cours, et le signe est reconnu partout. Carré de
        // dix-huit pixels, comme les autres icônes de l'application.
        if icon_square_button(ui, icon_gear)
            .on_hover_text(s.preferences_title)
            .clicked()
        {
            self.show_preferences = true;
        }
        if framed_button(ui, theme, s.menu_help).clicked() {
            self.show_help = true;
        }
        // LA CALCULATRICE. Convertir entre décimal, hexadécimal et
        // binaire est le geste le plus fréquent du premier mois —
        // « combien fait 0x3c ? » —, et elle n'avait de chemin que par
        // le menu Outils, qui n'existe pas ici.
        if framed_button(ui, theme, s.calculator_open).clicked() {
            self.show_calculator = true;
        }
        // LES EXERCICES SONT DANS LA BARRE, et non derrière un menu
        // qui n'existe plus ici.
        //
        // C'est le parcours d'apprentissage du projet — cent quatorze
        // étapes —, et le mode simple est exactement celui de ceux à
        // qui il s'adresse. Sans ce bouton, il n'aurait plus de porte
        // du tout au niveau où il sert le plus : la palette de
        // commandes y mène encore, mais personne ne cherche une palette
        // le premier jour.
        //
        // Grisé quand le module n'est pas là, et il dit où l'installer :
        // une porte qu'on ne voit pas n'existe pas.
        if self.exercises {
            if framed_button(ui, theme, s.exercises_open).clicked() {
                self.show_exercises = true;
            }
        } else {
            ui.add_enabled_ui(false, |ui| {
                framed_button(ui, theme, s.exercises_open)
                    .on_disabled_hover_text(s.exercises_where);
            });
        }
        if framed_button(ui, theme, s.action_save).clicked() {
            self.dispatch(Action::Save);
        }
        if framed_button(ui, theme, s.file_open_file).clicked() {
            self.dispatch(Action::OpenFile);
        }
    }

    /// Les cinq gestes du mode simple, dans l'ordre où on les apprend :
    /// assembler, lancer, avancer d'un pas, revenir, reprendre.
    ///
    /// Aucun n'est caché selon l'état : ils se grisent, et disent
    /// pourquoi. Une commande éteinte qui se tait passe pour cassée ;
    /// celle qui dit ce qui manque enseigne l'ordre des gestes — c'est
    /// la règle déjà suivie par la barre d'outils du mode avancé.
    fn etabli_gestes_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let pourquoi_pas = if self.emulator.is_none() {
            s.why_no_program
        } else {
            s.why_program_finished
        };
        // ASSEMBLER PORTE UNE ICÔNE, et ce n'est pas un ornement : un
        // bouton encadré fait vingt-sept pixels de haut, un bouton à
        // icône vingt-cinq, et le banc refuse deux hauteurs côte à côte
        // (« two_buttons_side_by_side_share_a_height »). Les cinq
        // gestes sont une seule rangée de commandes : ils se peignent
        // donc pareil, tous les cinq.
        ui.add_enabled_ui(self.can_assemble(), |ui| {
            if icon_button(ui, icon_assemble, s.action_assemble)
                .on_disabled_hover_text(s.binary_no_reassemble_hint)
                .clicked()
            {
                self.dispatch(Action::Assemble);
            }
        });
        let en_marche = matches!(self.status, Status::Running);
        ui.add_enabled_ui(self.can_execute(), |ui| {
            // Lancer et arrêter sont le même bouton : le programme
            // tourne ou il ne tourne pas, et deux boutons dont l'un est
            // toujours éteint prennent la place pour ne rien dire.
            let (icône, libellé, action): (
                fn(&egui::Painter, egui::Rect, egui::Color32),
                &str,
                Action,
            ) = if en_marche {
                (icon_stop, s.action_stop, Action::Stop)
            } else {
                (icon_run, s.action_run, Action::Run)
            };
            if icon_button(ui, icône, libellé)
                .on_disabled_hover_text(pourquoi_pas)
                .clicked()
            {
                self.dispatch(action);
            }
            let nom_du_pas = action_label_pour(s, Action::StepInto, self.niveau());
            if icon_button(ui, icon_step_into, nom_du_pas)
                .on_disabled_hover_text(pourquoi_pas)
                .clicked()
            {
                self.dispatch(Action::StepInto);
            }
        });
        let peut_revenir = self.emulator.as_ref().is_some_and(|e| e.can_step_back());
        ui.add_enabled_ui(peut_revenir, |ui| {
            let nom = action_label_pour(s, Action::StepBack, self.niveau());
            if icon_button(ui, icon_step_back, nom)
                .on_disabled_hover_text(s.why_nothing_to_undo)
                .clicked()
            {
                self.dispatch(Action::StepBack);
            }
        });
        ui.add_enabled_ui(self.emulator.is_some(), |ui| {
            if icon_button(ui, icon_reset, s.action_reset)
                .on_disabled_hover_text(s.why_no_program)
                .clicked()
            {
                self.dispatch(Action::Reset);
            }
        });
    }

    /// Où l'on en est : une pastille, un mot, et l'adresse courante.
    ///
    /// C'est ce que disait la barre d'état, ramené en haut et réduit à
    /// ce qui change : le mode simple n'a pas de barre d'état, et le
    /// bas de l'écran est pris par le résultat.
    fn etabli_frise_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        ui.horizontal(|ui| {
            // LA DROITE D'ABORD, ET C'EST TOUT LE SUJET.
            //
            // Posée après le message, elle n'avait plus de place dès
            // que celui-ci était long : relevé à l'écran sur un ELF lié
            // dynamiquement, dont la note fait quatre lignes, RIP et le
            // bouton de la Scène étaient purement et simplement
            // absents de la ligne. Un alignement à droite prend ce qui
            // reste ; quand il ne reste rien, il ne reste rien.
            //
            // En la posant la première, elle réserve sa place, et c'est
            // le message qui se serre — lui sait le dire, voir
            // ci-dessous.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(emulator) = self.emulator.as_ref() {
                    let rip = emulator.cpu().regs.rip;
                    ui.label(
                        egui::RichText::new(format!("RIP {rip:#x}"))
                            .color(theme.dim)
                            .monospace()
                            .size(12.0),
                    );
                    ui.add_space(10.0);
                    // LA SCÈNE EST ICI, ET NON DANS LA BARRE.
                    //
                    // C'est une façon de regarder l'exécution, pas une
                    // commande du fichier : sa place est sur la ligne
                    // qui dit où l'on en est. Et elle ne paraît que
                    // devant un programme — un bouton qui n'a rien à
                    // montrer est un bouton qui déçoit.
                    if small_framed_button(ui, theme, s.scene_open)
                        .on_hover_text(s.scene_hint)
                        .clicked()
                    {
                        self.show_scene = true;
                    }
                }
                // Le message dans ce qui reste, et aligné à gauche :
                // `status_message_ui` mesure ce qu'il voulait, et pose
                // une infobulle qui le donne en entier dès qu'il a été
                // coupé. Une seule écriture de cette règle, partagée
                // avec la barre d'état du mode avancé.
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    self.status_message_ui(ui);
                });
            });
        });
    }

    /// Le volet de la machine : les registres, les indicateurs, la pile.
    ///
    /// Les trois sont posés l'un sous l'autre et sans onglet : au mode
    /// simple, choisir entre « registres » et « pile » est déjà un
    /// geste de trop, et ce sont justement les deux endroits qu'on
    /// regarde ENSEMBLE en avançant d'un pas.
    fn etabli_machine_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        if titre_detachable(ui, theme, s, s.etabli_machine) {
            self.etabli_machine_detachee = true;
        }
        ui.add_space(4.0);
        self.etabli_machine_corps_ui(ui, false);
    }

    /// Le contenu du volet de la machine, sans son titre.
    ///
    /// À part, parce qu'il se peint à deux endroits : dans le volet de
    /// droite, et dans la fenêtre où on l'a sorti. Les deux montrent
    /// exactement la même chose — une fenêtre n'est pas une autre vue,
    /// c'est la même vue ailleurs.
    pub(crate) fn etabli_machine_corps_ui(&mut self, ui: &mut egui::Ui, détachée: bool) {
        let s = strings(self.lang);
        let theme = self.theme;
        egui::ScrollArea::vertical()
            .id_salt("etabli_machine")
            // DANS LE VOLET, la zone prend toute la hauteur qu'on lui a
            // donnée ; DANS SA FENÊTRE, c'est elle qui donne sa hauteur
            // à la fenêtre. Réglée sur « ne pas se rétracter » des deux
            // côtés, la fenêtre s'ouvrait trop courte et coupait la
            // pile — ce pour quoi on l'avait détachée.
            .auto_shrink([false, !détachée])
            .show(ui, |ui| {
                // CE QUE CE SONT, DIT EN MOTS, ET À SA PLACE.
                //
                // La pile avait sa légende depuis toujours — « en
                // grisé, au-dessus de RSP : déjà dépilé ». Les
                // registres et les indicateurs n'en avaient aucune :
                // dix-sept noms de trois lettres, sept sigles de deux,
                // et pas un mot pour dire ce que c'est. Dans le mode
                // dont la règle est que chaque bande se présente
                // elle-même et que rien ne se survole pour savoir ce
                // que c'est (voir l'en-tête de ce fichier), c'était le
                // seul endroit qui montrait de l'état brut sans le
                // nommer.
                //
                // ICI ET PAS DANS L'INFOBULLE : les sept indicateurs
                // ont déjà la leur, complète, et elle demande de savoir
                // qu'il faut survoler — c'est-à-dire de savoir que ces
                // sigles cachent quelque chose. Celui qui l'ignore est
                // exactement celui à qui cela s'adresse.
                //
                // SEULEMENT AU MODE SIMPLE, et c'est là qu'est la
                // différence avec la légende de la pile : le panneau
                // latéral du mode avancé est déjà serré, et ces deux
                // phrases y seraient du bruit devant quelqu'un qui sait.
                //
                // AVANT LA LISTE, ET NON APRÈS. La première capture la
                // posait sous les dix-sept registres : on lisait donc
                // dix-sept lignes incompréhensibles, PUIS ce qu'elles
                // étaient. Une légende se lit avant ce qu'elle explique
                // — c'est l'ordre de la bande de l'exercice, « ce qu'on
                // demande » puis l'énoncé.
                //
                // Celle des indicateurs, elle, reste APRÈS sa rangée :
                // la rangée tient sur une ligne, on l'a déjà vue en
                // entier quand on arrive à la phrase.
                legende_de_volet(ui, theme, s.etabli_regs_legend);
                ui.add_space(3.0);
                self.registers_ui(ui);
                if self.emulator.is_none() {
                    return;
                }
                ui.add_space(8.0);
                self.flags_ui(ui);
                legende_de_volet(ui, theme, s.etabli_flags_legend);
                ui.add_space(8.0);
                titre_de_bande(ui, theme, s.etabli_stack);
                ui.add_space(2.0);
                self.stack_ui(ui);
            });
    }

    /// Y a-t-il un exercice à montrer en bande ?
    ///
    /// Les deux conditions comptent. Le MODULE peut être éteint dans
    /// les préférences — les séries restent alors chargées, et la bande
    /// paraîtrait pour un parcours auquel plus rien ne mène. Et il faut
    /// une étape EN COURS : une bande vide qui dirait « aucun exercice »
    /// prendrait cent soixante pixels pour l'annoncer à quelqu'un qui
    /// écrit son propre programme.
    pub(crate) fn exercices_en_bande(&self) -> bool {
        self.exercices_en_cours().is_some()
    }

    /// L'exercice en cours, s'il y en a un et si le module est allumé.
    fn exercices_en_cours(&self) -> Option<&str> {
        if !self.exercises {
            return None;
        }
        let id = self.current_exercise.as_deref()?;
        namsdec_exercises::find(&self.exercise_packs, id).map(|_| id)
    }

    /// La bande de l'exercice : ce qu'on demande, et où l'on en est.
    ///
    /// TROIS CHOSES, ET DANS CET ORDRE. Le titre dit laquelle des cent
    /// dix-sept étapes on fait et combien sont faites — on travaille
    /// dans un parcours, pas sur un devoir isolé. L'énoncé dit ce qu'on
    /// demande. Le but, en dessous, s'allume tout seul pendant qu'on
    /// avance : c'est lui qui rend l'exercice INTERACTIF plutôt que lu.
    ///
    /// LE BUT N'EST PAS RÉÉCRIT ICI. `goal_ui` le peint déjà, condition
    /// par condition, avec le constat de ce qui manque et le budget
    /// d'instructions — et il porte aussi la question des exercices de
    /// lecture et le cas des démonstrations. Le redire autrement aurait
    /// fait deux vérités à tenir d'accord ; ce qui manquait n'était pas
    /// le texte, c'était l'endroit où le peindre.
    fn etabli_exercice_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let Some(id) = self.exercices_en_cours().map(str::to_owned) else {
            return;
        };
        // TOUT CE QU'ON VA PEINDRE EST RELEVÉ D'ABORD. Les séries sont
        // empruntées pour le lire, et la bande écrit ensuite dans
        // `show_exercises` ou charge l'étape suivante : garder
        // l'emprunt ouvert interdirait les deux.
        let mut relevé = None;
        for pack in &self.exercise_packs {
            let Some(index) = pack
                .exercises
                .iter()
                .position(|autre| namsdec_exercises::full_id(pack, autre) == id)
            else {
                continue;
            };
            let exercise = &pack.exercises[index];
            let faits = pack
                .exercises
                .iter()
                .filter(|autre| {
                    self.exercises_done
                        .contains(&namsdec_exercises::full_id(pack, autre))
                })
                .count();
            relevé = Some((
                index + 1,
                pack.exercises.len(),
                faits,
                exercise.title.get(lang).to_owned(),
                exercise.description.get(lang).to_owned(),
                exercise.level.label(lang),
            ));
            break;
        }
        let Some((rang, total, faits, titre, énoncé, palier)) = relevé else {
            return;
        };
        let fait = self.exercises_done.contains(&id);
        let suivant = self.exercise_after(&id);

        let mut choisir = false;
        let mut aller: Option<String> = None;
        // LE TITRE RESTE HORS DE LA ZONE QUI DÉFILE. Les deux gestes
        // qu'il porte — choisir une autre étape, passer à la suivante —
        // ne doivent pas descendre hors de vue quand l'énoncé est long :
        // c'est précisément l'étape longue qu'on veut pouvoir quitter.
        ui.horizontal(|ui| {
            // LA DROITE D'ABORD, et c'est la même règle que la frise :
            // posée après le titre, elle n'a plus de place dès que
            // celui-ci est long, et un alignement à droite qui ne
            // trouve rien ne laisse rien.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((suivant_id, titre_suivant)) = &suivant
                    && small_framed_button(ui, theme, s.exercises_next)
                        .on_hover_text(titre_suivant)
                        .clicked()
                {
                    aller = Some(suivant_id.clone());
                }
                if small_framed_button(ui, theme, s.etabli_exercise_pick)
                    .on_hover_text(s.exercises_title)
                    .clicked()
                {
                    choisir = true;
                }
                ui.add_space(8.0);
                // CE QUI EST FAIT DU PARCOURS, et non de cette étape :
                // la barre avance d'un cran à chaque réussite, et c'est
                // la seule chose de cet écran qui dise qu'on va quelque
                // part.
                //
                // LA BARRE SEULE, ET LE COMPTE AU SURVOL. Elle portait
                // « 1/117 » à côté d'elle, et le titre de la bande
                // porte « 2/117 » trois centimètres plus à gauche :
                // deux fractions de même forme et de même dénominateur
                // sur une seule ligne, l'une disant le RANG et l'autre
                // les RÉUSSIS. Sur la capture du 10 novembre 2026, on
                // les lisait l'une pour l'autre. Le rang reste écrit —
                // c'est lui qu'on cherche —, le reste se survole.
                crate::help::barre_progression(ui, theme, faits, total, 80.0)
                    .on_hover_text(format!("{} : {faits} / {total}", s.exercises_title));
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // La pastille verte des étapes déjà réussies : la
                    // même qu'au titre du résultat, et elle dit la même
                    // chose — c'est fait, on peut passer.
                    if fait {
                        status_dot(ui, theme.good);
                    }
                    titre_de_bande(ui, theme, s.etabli_exercise);
                    ui.colored_label(theme.accent, format!("{rang}/{total}"));
                    ui.colored_label(theme.dim, palier);
                    // TRONQUÉ, et relisible au survol : un titre long
                    // dans une rangée d'egui s'allonge au lieu de se
                    // couper, et passerait par-dessus les deux boutons
                    // qu'on vient de poser à droite.
                    ui.add(egui::Label::new(egui::RichText::new(&titre).strong()).truncate())
                        .on_hover_text(&titre);
                });
            });
        });
        ui.add_space(2.0);
        // L'ÉNONCÉ ET LE BUT DÉFILENT ENSEMBLE, dans ce qui reste de la
        // bande. La hauteur est celle du panneau — réglée, non devinée
        // d'après le contenu —, et c'est ce qui permet à une zone
        // défilante de savoir ce qu'elle a : montée dans un panneau qui
        // s'ajuste à ce qu'il porte, elle n'en montrait que la moitié
        // (voir la bande de la ligne courante).
        egui::ScrollArea::vertical()
            .id_salt("etabli_exercice")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.colored_label(theme.dim, s.etabli_exercise_statement);
                // `prose` ET NON UNE ÉTIQUETTE NUE. Les énoncés citent
                // les instructions dont ils parlent entre accents
                // graves — « `loop` décrémente RCX » —, et la bande
                // les montrait tels quels. Vu à l'écran le 10 novembre
                // 2026 sur l'étape 5. Même traitement que la fiche de
                // la bande du bas, et pour la même raison : un texte
                // qui montre son balisage apprend à ne plus le lire.
                ui.label(prose(ui, theme, &énoncé));
                self.goal_ui(ui);
            });

        if choisir {
            self.show_exercises = true;
        }
        if let Some(suivant_id) = aller {
            self.load_exercise(&suivant_id);
        }
    }

    /// Ce que fait la ligne où l'on est.
    ///
    /// Les deux blocs que le panneau latéral portait dans son bandeau,
    /// remis à plat : ce qui vient, et ce qui vient de se passer. Ils
    /// ne sont pas repliables ici — c'est la bande qu'on lit en
    /// avançant, et une bande qu'on peut replier finit repliée.
    fn etabli_ligne_ui(&mut self, ui: &mut egui::Ui) {
        // RIEN NE TOURNE : ON EXPLIQUE LA LIGNE QU'ON LIT.
        //
        // C'est l'état du premier écran, et il durait tout le temps
        // qu'on passe à LIRE un programme avant de l'exécuter —
        // c'est-à-dire la première moitié de l'apprentissage. La bande
        // y disait « ici s'afficheront l'adresse de la prochaine
        // instruction, ce qu'elle fait et ses octets », autrement dit :
        // revenez quand vous aurez assemblé.
        //
        // Pendant ce temps, l'écran portait six instructions qu'un
        // débutant ne connaît pas, et la seule porte vers les cent
        // quatre-vingt-neuf fiches était une touche dont rien ne parle.
        // Elles sont écrites ; il manquait l'endroit et le moment.
        // UNE SEULE BANDE D'EXPLICATION À LA FOIS, et c'est la mesure
        // qui a tranché.
        //
        // La bande de l'exercice porte déjà la consigne, et elle est en
        // haut. Les deux ensemble, à 800x520 — le plus petit écran que
        // l'application accepte —, ne laissaient PLUS AUCUNE ligne de
        // programme visible : deux avant cette bande, zéro après. Un
        // écran où l'on ne voit pas une ligne du programme qu'on
        // demande d'écrire n'est pas un écran serré, c'est un écran
        // cassé.
        //
        // Ce n'est pas une règle de taille mais de RÔLE : expliquer ce
        // qu'on a sous les yeux est un seul métier, et deux bandes qui
        // le font en même temps se disputent la même place. Quand un
        // exercice est en cours, c'est lui qui l'exerce.
        //
        // CE QUE CELA COÛTE, ET C'EST DIT : pendant un exercice, la
        // ligne où est le curseur ne s'explique plus toute seule — il
        // reste F1 et le survol. Le vrai levier serait ailleurs (la
        // bande du résultat prend 45 % de ce qui reste), et il
        // appartient à l'utilisateur : voir A 9.86.
        if self.emulator.is_none() && !self.exercices_en_bande() {
            if self.etabli_ligne_du_curseur_ui(ui) {
                return;
            }
            // RIEN À DIRE DE CETTE LIGNE-CI — une ligne vide, une
            // étiquette seule, un commentaire. C'est l'état du PREMIER
            // ÉCRAN : le curseur y est au premier caractère, qui ouvre
            // une ligne de commentaire, et la bande restait donc muette
            // au moment précis où elle avait le plus à apprendre.
            //
            // On garde ce que la bande annonçait — ce qui s'y affichera
            // une fois le programme assemblé —, et l'on dit comment la
            // faire parler tout de suite. La même phrase sert aux deux
            // endroits : sous une fiche elle dit « il y en a d'autres »,
            // ici elle dit « essayez ». Deux phrases presque identiques
            // seraient deux phrases à tenir d'accord.
            self.next_instruction_ui(ui);
            let s = strings(self.lang);
            let theme = self.theme;
            ui.add_space(4.0);
            ui.vertical_centered(|ui| {
                ui.add(egui::Label::new(
                    egui::RichText::new(s.etabli_cursor_hint)
                        .color(theme.dim)
                        .size(12.0),
                ));
            });
            return;
        }
        self.next_instruction_ui(ui);
        self.last_step_ui(ui);
    }

    /// Ce que dit la ligne où est le curseur, tant que rien ne tourne.
    ///
    /// Rend `false` quand il n'y a rien à dire — une ligne vide, une
    /// étiquette seule, un mot qui n'a pas de fiche. La bande reprend
    /// alors ce qu'elle disait : ce qui s'y affichera une fois le
    /// programme assemblé. Ne jamais rien dire est pire que de dire ce
    /// qui viendra.
    fn etabli_ligne_du_curseur_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang.code();
        let Some((mnémonique, fiche)) = self.fiche_de_la_ligne_du_curseur() else {
            return false;
        };
        let mut ouvrir = false;
        ui.horizontal(|ui| {
            titre_de_bande(ui, theme, s.etabli_cursor_line);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if small_framed_button(ui, theme, s.etabli_cursor_sheet)
                    .on_hover_text(s.etabli_cursor_hint)
                    .clicked()
                {
                    ouvrir = true;
                }
            });
        });
        ui.add_space(2.0);
        // LE MOT, PUIS CE QU'IL VEUT DIRE, puis le détail : c'est la
        // forme qu'a déjà la Scène pour l'instruction qui vient, et
        // deux formes pour la même chose se liraient comme deux
        // fonctions différentes.
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(
                egui::RichText::new(&mnémonique)
                    .monospace()
                    .strong()
                    .color(theme.mnemonic)
                    .size(14.0),
            );
            ui.label(
                egui::RichText::new(fiche.title.get(lang))
                    .color(theme.text)
                    .size(13.0),
            );
        });
        ui.add_space(2.0);
        // `prose` ET NON UNE ÉTIQUETTE NUE : une fiche cite entre
        // accents graves les mots dont elle parle — `cmp`, `RSP` —, et
        // c'est la fenêtre d'aide qui les met en chasse fixe. Peinte
        // telle quelle, la même phrase montrait ses accents graves.
        // Une seule mise en forme pour les deux endroits : la fiche
        // est la même, elle doit se lire pareil.
        ui.label(prose(ui, theme, premiere_phrase(fiche.body.get(lang))));
        // ET COMMENT ON S'EN SERT, une fois : sans cette ligne, rien ne
        // dit que la bande SUIT le curseur, et l'on croit lire une note
        // posée là pour cette ligne-là.
        ui.add_space(2.0);
        ui.add(egui::Label::new(
            egui::RichText::new(s.etabli_cursor_hint)
                .color(theme.dim)
                .size(12.0),
        ));
        if ouvrir {
            self.open_requested_sheet(Some(mnémonique));
        }
        true
    }

    /// La fiche du mot qui ouvre la ligne où est le curseur.
    ///
    /// LE PREMIER MOT, ET NON LE MOT SOUS LE CURSEUR — c'est la
    /// différence avec F1, et elle est voulue. F1 répond à une question
    /// qu'on pose : on a mis le curseur sur `rep`, c'est de `rep` qu'on
    /// parle. Ici personne n'a rien demandé : la bande suit le curseur
    /// de ligne en ligne, et ce qu'on veut savoir d'une ligne est ce
    /// qu'elle FAIT, c'est-à-dire son premier mot.
    ///
    /// L'étiquette et le commentaire sont retirés : `boucle: add rax,
    /// rcx  ; la somme` parle de `add`.
    fn fiche_de_la_ligne_du_curseur(&self) -> Option<(String, &'static namsdec_doc::Entry)> {
        let ligne = crate::text::ligne_autour(&self.source, self.cursor_byte);
        let sans_commentaire = ligne.split(';').next().unwrap_or("");
        let sans_étiquette = match sans_commentaire.split_once(':') {
            Some((_, reste)) => reste,
            None => sans_commentaire,
        };
        let mot = sans_étiquette.split_whitespace().next()?;
        let mnémonique = mot.to_lowercase();
        namsdec_doc::lookup(&mnémonique).map(|fiche| (mnémonique, fiche))
    }

    /// Le résultat : ce que le programme a écrit, ou ce qui l'en a
    /// empêché.
    ///
    /// Une seule bande pour les deux, et c'est voulu : au mode simple,
    /// « ça n'a pas marché » et « voilà ce que ça donne » sont la même
    /// question, posée au même endroit. Les trois onglets de la console
    /// restent à un clic.
    fn etabli_resultat_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        // CE QUE LE PROGRAMME A ÉCRIT SE SIGNALE DANS LE TITRE.
        //
        // Une pastille verte et le titre en clair : on voit d'un coup
        // d'œil que le programme a parlé, sans avoir à lire la bande.
        // Sans cela, la sortie d'un premier programme — « bonjour » sur
        // une ligne — passait inaperçue au milieu d'un écran qui
        // change à chaque pas.
        let a_parlé = self
            .emulator
            .as_ref()
            .is_some_and(|e| !e.output().is_empty());
        if titre_du_resultat(ui, theme, s, a_parlé) {
            self.etabli_resultat_detache = true;
        }
        ui.add_space(2.0);
        self.etabli_resultat_corps_ui(ui);
    }

    /// Le contenu de la bande du résultat, sans son titre.
    ///
    /// À part pour la même raison que celui de la machine : il se peint
    /// dans la bande, et dans la fenêtre où on l'a sorti.
    pub(crate) fn etabli_resultat_corps_ui(&mut self, ui: &mut egui::Ui) {
        // LES DEUX, ET DANS CET ORDRE, quand il y a les deux.
        //
        // Un avertissement de nasm et une sortie de programme peuvent
        // coexister, et choisir l'un des deux reviendrait à cacher
        // l'autre sans le dire.
        let des_problèmes = !self.diagnostics.is_empty() || self.combien_de_simplifications() > 0;
        // LA SORTIE NE SE PEINT PAS QUAND IL N'Y A RIEN EU À EXÉCUTER.
        //
        // Quand l'assemblage a échoué il n'y a pas d'émulateur, et « La
        // sortie du programme s'affichera ici » n'apprend rien qu'on ne
        // sache déjà — mais elle prend sa place, et le banc l'a trouvée
        // COUPÉE PAR LE MILIEU sous la liste des erreurs : quatre
        // pixels peints sur quinze. Une zone dont l'unique ligne est
        // tronquée a montré tout ce qu'elle avait, et l'a montré à
        // moitié.
        let avec_sortie = !(des_problèmes && self.emulator.is_none());

        if des_problèmes {
            // Bornée : un programme qui compte vingt erreurs ne doit
            // pas chasser la sortie hors de la bande — on lit les
            // premières, on défile pour les suivantes.
            // CE QU'ON GARDE POUR LA SUITE, ET LA SUITE A TROIS
            // RANGÉES.
            //
            // `SORTIE_MIN` seul ne gardait que le cadre de la sortie —
            // sa zone défilante et ses marges. Il oubliait ce qui vient
            // APRÈS elle : les arguments, l'environnement, et le champ
            // où l'on répond au programme. Devant un binaire ouvert,
            // ces trois rangées font près de cent pixels de plus, et
            // elles passaient sous le bord de la bande : « Variables
            // d'environnement » coupé en deux, et plus de champ de
            // saisie du tout (relevé à l'écran le 10 novembre 2026,
            // « innaccesible »).
            //
            // La mesure est celle de la console du mode avancé : deux
            // formules pour une même rangée finiraient par compter deux
            // choses.
            let saisie = self
                .console_input_estimate(ui)
                .max(self.console_input_height.unwrap_or(0.0));
            let garde = if avec_sortie {
                SORTIE_MIN + saisie
            } else {
                0.0
            };
            let place = (ui.available_height() - garde).max(40.0);
            let aller = egui::ScrollArea::vertical()
                .id_salt("etabli_problemes")
                .max_height(place)
                .auto_shrink([false, true])
                .min_scrolled_height(0.0)
                .show(ui, |ui| self.problemes_ui(ui))
                .inner;
            if let Some(ligne) = aller {
                self.goto_line = Some(ligne);
            }
            if avec_sortie {
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);
            }
        }
        // `output_ui` porte déjà tous ses états : pas de programme,
        // programme en cours, code de sortie, et le champ de saisie
        // quand le programme attend de quoi lire. Le mode simple n'a
        // rien à redire par-dessus — et il ne l'enveloppe PAS dans une
        // zone défilante : elle a la sienne, qui prend toute la
        // hauteur, et deux zones l'une dans l'autre laissaient soixante
        // pixels de vide sous le titre.
        if avec_sortie {
            self.output_ui(ui);
        }
    }
}

impl NamsdecApp {
    /// Les deux bandes qu'on a sorties de l'Établi.
    ///
    /// Posées avec les autres fenêtres, donc APRÈS les panneaux : une
    /// fenêtre se pose par-dessus, et c'est tout l'intérêt.
    ///
    /// **Pourquoi elles existent.** Les quatre bandes de l'Établi ont
    /// une place répartie d'avance — c'est le prix de « rien à
    /// chercher » —, et deux d'entre elles portent du contenu dont on
    /// ne connaît pas la taille : la pile d'un vrai binaire montre des
    /// chaînes d'environnement longues de quarante caractères, et une
    /// explication de namsdec peut faire cinq lignes. Relevé à
    /// l'écran, sur un ELF lié dynamiquement : la pile défilait
    /// horizontalement et le texte du résultat était coupé en son
    /// milieu. Tirer la poignée aide ; sortir la bande dans une
    /// fenêtre règle la question.
    pub(crate) fn etabli_fenetres_ui(&mut self, ctx: &egui::Context) {
        if !self.debutant {
            // Au mode avancé, ces deux bandes n'existent pas : leur
            // contenu est dans le panneau latéral et dans la console.
            // Une fenêtre restée ouverte en changeant de niveau
            // montrerait un morceau d'une interface qu'on a quittée.
            self.etabli_machine_detachee = false;
            self.etabli_resultat_detache = false;
            return;
        }
        let s = strings(self.lang);
        if self.etabli_machine_detachee {
            let mut ouverte = true;
            let mut fermer = false;
            egui::Window::new(s.etabli_machine)
                .id(egui::Id::new("etabli_machine_window"))
                .open(&mut ouverte)
                .resizable(true)
                .default_width(MACHINE_LARGEUR + 120.0)
                // Assez haute pour les seize registres, les sept
                // indicateurs ET les douze cases de pile : c'est pour
                // la pile qu'on détache, et une fenêtre qui la coupe
                // encore n'aurait rien réglé.
                .default_height(600.0)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(default_window_pos(ctx, "etabli_machine"))
                .title_bar(false)
                .show(ctx, |ui| {
                    if chrome::window_head(ui, self.theme, s, s.etabli_machine) {
                        fermer = true;
                    }
                    self.etabli_machine_corps_ui(ui, true);
                });
            self.etabli_machine_detachee = ouverte && !fermer;
        }
        if self.etabli_resultat_detache {
            let mut ouverte = true;
            let mut fermer = false;
            egui::Window::new(s.etabli_result)
                .id(egui::Id::new("etabli_resultat_window"))
                .open(&mut ouverte)
                .resizable(true)
                .default_width(720.0)
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(default_window_pos(ctx, "etabli_resultat"))
                .title_bar(false)
                .show(ctx, |ui| {
                    if chrome::window_head(ui, self.theme, s, s.etabli_result) {
                        fermer = true;
                    }
                    self.etabli_resultat_corps_ui(ui);
                });
            self.etabli_resultat_detache = ouverte && !fermer;
        }
    }
}

/// Le titre de la bande du résultat : sa pastille, son nom, son bouton.
///
/// À part de [`titre_detachable`] pour une seule raison : lui seul a
/// quelque chose à annoncer. Rend `true` quand on a cliqué sur
/// « Détacher ».
fn titre_du_resultat(
    ui: &mut egui::Ui, theme: &'static Theme, s: &Strings, a_parlé: bool
) -> bool {
    let mut détacher = false;
    ui.horizontal(|ui| {
        if a_parlé {
            status_dot(ui, theme.good);
        }
        ui.label(
            egui::RichText::new(s.etabli_result.to_uppercase())
                .color(if a_parlé { theme.text } else { theme.dim })
                .size(9.0)
                .strong(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if small_framed_button(ui, theme, s.view_detach)
                .on_hover_text(s.etabli_detach_hint)
                .clicked()
            {
                détacher = true;
            }
        });
    });
    détacher
}

/// Le titre d'une bande, avec le bouton qui la sort dans une fenêtre.
///
/// Rend `true` quand on a cliqué. Le bouton est au bord droit, petit et
/// encadré : c'est la place et la forme qu'a déjà « Détacher » sur la
/// rangée des vues, et il fait la même chose — un déménagement, pas un
/// changement de ce qu'on regarde.
fn titre_detachable(ui: &mut egui::Ui, theme: &'static Theme, s: &Strings, texte: &str) -> bool {
    let mut détacher = false;
    ui.horizontal(|ui| {
        titre_de_bande(ui, theme, texte);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if small_framed_button(ui, theme, s.view_detach)
                .on_hover_text(s.etabli_detach_hint)
                .clicked()
            {
                détacher = true;
            }
        });
    });
    détacher
}

/// La première phrase d'un texte, et rien de plus.
///
/// LE CORPS ENTIER NE TIENT PAS DANS UNE BANDE, et c'est mon propre
/// contrôle du plus petit écran qui l'a dit : à 800x520, la fiche
/// complète ramenait de six à UNE le nombre de lignes de programme
/// visibles. Une explication qui chasse le programme qu'elle explique
/// ne vaut pas mieux que pas d'explication — c'est la même leçon que
/// les légendes du volet de la machine, trois jours plus tôt dans la
/// même séance.
///
/// La première phrase porte l'essentiel — « L'addition et la
/// soustraction, sur 8 à 64 bits. » —, le reste est derrière « La
/// fiche entière ». Une bande n'est pas une page de manuel : elle dit
/// de quoi il s'agit et montre la porte.
///
/// Le découpage est fait sur « . » suivi d'un blanc, ce qui laisse
/// intacts les `0.5`, les `st0.` et les abréviations sans espace. Un
/// texte d'une seule phrase revient entier.
fn premiere_phrase(texte: &str) -> &str {
    match texte.find(". ") {
        Some(at) => &texte[..at + 1],
        None => texte,
    }
}

/// Ce qu'un nom de fichier garde au plus dans la barre du haut.
///
/// Trente caractères : de quoi reconnaître un nom de travail sans
/// entamer la place des cinq commandes qui suivent. Compté en
/// CARACTÈRES et non en pixels, parce que le nom est en chasse fixe —
/// trente caractères y font toujours la même largeur.
pub(crate) const NOM_MAX: usize = 30;

/// Abrège un nom de fichier par le MILIEU, en gardant son extension.
///
/// Le début dit de quel fichier il s'agit, l'extension dit de quoi il
/// est fait. Couper par la droite perdrait la seconde, et deux
/// fichiers d'un même projet ne se distinguent souvent que par leur
/// début.
///
/// Rend le nom tel quel quand il tient — et quand l'abréger ne
/// gagnerait rien : un nom de trente-et-un caractères dont l'extension
/// en fait vingt ne se raccourcit pas en y ajoutant des points de
/// suspension.
pub(crate) fn nom_abrégé(nom: &str, maximum: usize) -> String {
    if nom.chars().count() <= maximum {
        return nom.to_owned();
    }
    // L'extension avec son point, quand elle est courte : au-delà, ce
    // n'est plus une extension mais la fin du nom.
    let extension = nom
        .rfind('.')
        .map(|at| &nom[at..])
        .filter(|ext| ext.chars().count() <= 6)
        .unwrap_or("");
    let garde = maximum.saturating_sub(extension.chars().count() + 1);
    if garde < 4 {
        return nom.chars().take(maximum).collect();
    }
    let début: String = nom.chars().take(garde).collect();
    format!("{début}…{extension}")
}

/// Une phrase qui dit ce que la ligne du dessus montrait.
///
/// Plus petite que le texte courant et dans la couleur discrète : elle
/// se lit une fois, puis on l'oublie — ce qui est exactement ce qu'on
/// veut d'une légende. Posée comme une étiquette ordinaire, et donc
/// repliée sur la largeur du volet : une phrase qui défilerait de côté
/// ne serait lue par personne (voir la légende de la pile, qui l'a
/// appris à ses dépens).
fn legende_de_volet(ui: &mut egui::Ui, theme: &'static Theme, texte: &str) {
    ui.add_space(3.0);
    ui.add(egui::Label::new(
        egui::RichText::new(texte).color(theme.dim).size(12.0),
    ));
}

/// Le titre d'une bande : petit, en capitales, dans la couleur
/// discrète.
///
/// Le même à quatre endroits, et d'un seul tenant : quatre bandes dont
/// les titres ne se ressembleraient pas se liraient comme quatre
/// applications.
fn titre_de_bande(ui: &mut egui::Ui, theme: &'static Theme, texte: &str) {
    ui.label(
        egui::RichText::new(texte.to_uppercase())
            .color(theme.dim)
            .size(9.0)
            .strong(),
    );
}
