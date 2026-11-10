//! La rangée des vues : ce que la zone centrale montre du programme.
//!
//! Une rangée plate, sous les onglets de fichiers, façon Cutter. Les
//! deux rangées ne disent pas la même chose et ne se dessinent donc pas
//! pareil : l'onglet de FICHIER est un rectangle plein, avec son liseré
//! en haut et sa croix — c'est un document qu'on ouvre et qu'on ferme ;
//! l'onglet de VUE est une étiquette soulignée — c'est une façon de
//! regarder, et elle ne se ferme pas.
//!
//! Une vue peut quitter la rangée pour une fenêtre à elle : c'est la
//! seule manière de lire la source et le désassemblage côte à côte, ce
//! qu'un onglet interdit par construction. Fermer la fenêtre la ramène
//! dans la rangée — une vue ne se perd pas, elle est quelque part.

use super::*;

/// La hauteur d'une ligne et la marge d'une étiquette viennent du corps
/// partagé (`rangée_d_onglets`) : les deux rangées d'onglets plats de
/// l'application — les vues de la zone centrale et les vues du panneau
/// latéral — se lisent pareil parce qu'elles se peignent pareil.
use crate::widgets::ONGLET_MARGE;

impl NamsdecApp {
    /// Cette vue est-elle sortie de la rangée ?
    ///
    /// Le booléen est celui de sa fenêtre, et c'est le même qu'avant le
    /// passage en onglets : `show_flow` dit « la fenêtre du déroulement
    /// est ouverte », ce qui veut dire exactement « le déroulement
    /// n'est plus dans la rangée ». Une seule vérité, lue de deux
    /// endroits — deux drapeaux auraient fini par se contredire.
    pub(crate) fn vue_détachée(&self, vue: VueCentrale) -> bool {
        match vue {
            VueCentrale::Source => false,
            VueCentrale::Disassembly => self.show_disassembly,
            VueCentrale::Flow => self.show_flow,
            VueCentrale::Decompiler => self.show_decompiler,
            VueCentrale::Analysis => self.show_analysis,
            VueCentrale::Timeline => self.show_timeline,
            VueCentrale::Breakpoints => self.show_breakpoints,
            VueCentrale::Vectors => self.show_vectors,
            VueCentrale::Patches => self.show_patches,
        }
    }

    /// Ouvre ou ferme la fenêtre d'une vue.
    ///
    /// `pub(crate)` pour le banc : fermer la fenêtre est ce que fait sa
    /// croix, et c'est le geste qu'un contrôle doit pouvoir refaire
    /// pour éprouver le retour dans la rangée.
    pub(crate) fn poser_fenêtre_de_vue(&mut self, vue: VueCentrale, ouverte: bool) {
        match vue {
            VueCentrale::Source => {}
            VueCentrale::Disassembly => self.show_disassembly = ouverte,
            VueCentrale::Flow => self.show_flow = ouverte,
            VueCentrale::Decompiler => self.show_decompiler = ouverte,
            VueCentrale::Analysis => self.show_analysis = ouverte,
            VueCentrale::Timeline => self.show_timeline = ouverte,
            VueCentrale::Breakpoints => self.show_breakpoints = ouverte,
            VueCentrale::Vectors => self.show_vectors = ouverte,
            VueCentrale::Patches => self.show_patches = ouverte,
        }
    }

    /// Cette vue existe-t-elle dans cette installation ?
    ///
    /// La décompilation est un module qu'on active : sans lui, elle n'a
    /// pas d'onglet. Un onglet qui ne mène à rien est pire qu'un onglet
    /// absent — c'est le même principe que pour une vue détachée.
    pub(crate) fn vue_disponible(&self, vue: VueCentrale) -> bool {
        match vue {
            VueCentrale::Decompiler => self.show_decompiler_module,
            _ => true,
        }
    }

    /// Cette vue est-elle dans la rangée, c'est-à-dire a-t-elle un
    /// onglet ?
    pub(crate) fn vue_dans_la_rangée(&self, vue: VueCentrale) -> bool {
        self.vue_disponible(vue) && !self.vue_détachée(vue)
    }

    /// Le niveau d'interface en vigueur.
    ///
    /// Un seul endroit le dit, et tout le reste le lui demande : la
    /// colonne d'analyse, la barre d'outils, les deux rangées
    /// d'onglets, les menus. Une copie de plus serait une occasion de
    /// diverger — et ce réglage décide de ce que l'application montre
    /// d'elle-même à quelqu'un qui l'ouvre pour apprendre.
    pub(crate) fn niveau(&self) -> namsdec_re::Niveau {
        if self.debutant {
            namsdec_re::Niveau::Débutant
        } else {
            namsdec_re::Niveau::Avancé
        }
    }

    /// Cette vue a-t-elle un onglet AU NIVEAU EN VIGUEUR ?
    ///
    /// À part de [`Self::vue_disponible`], et la distinction compte :
    /// « disponible » dit ce que cette installation PORTE — la
    /// décompilation n'existe pas sans son module —, le niveau ne dit
    /// que ce que la rangée MONTRE. Une vue au-dessus du niveau existe
    /// toujours : le menu Outils y mène, la palette l'ouvre, un lien
    /// d'un panneau la demande, et `montrer_vue` la pose comme vue
    /// active — ce qui suffit à lui rendre son onglet, par la règle
    /// ci-dessous.
    ///
    /// CE QU'ON REGARDE RESTE LISTÉ, toujours. C'est la même leçon que
    /// pour la colonne d'analyse : une vue à l'écran sans onglet est
    /// une vue dont on ne peut plus sortir, et dont on ne sait même
    /// plus le nom.
    pub(crate) fn vue_au_niveau(&self, vue: VueCentrale) -> bool {
        vue.niveau() <= self.niveau() || self.vue_centrale == vue
    }

    /// Sort la vue de la rangée, dans sa fenêtre.
    ///
    /// Celle qu'on détache ne peut plus être la vue active : la zone
    /// centrale retombe donc sur la source, qui est toujours là.
    pub(crate) fn détacher_vue(&mut self, vue: VueCentrale) {
        if !vue.détachable() || !self.vue_disponible(vue) {
            return;
        }
        self.poser_fenêtre_de_vue(vue, true);
        if self.vue_centrale == vue {
            self.vue_centrale = VueCentrale::Source;
        }
    }

    /// Montre cette vue — par son onglet, ou en la laissant dans la
    /// fenêtre où elle est déjà.
    ///
    /// C'est ce qu'appellent le menu Outils et la palette : « montre
    /// le désassemblage » veut dire « que je le voie », et il est déjà
    /// visible quand il est détaché. Le rattacher d'autorité défairait
    /// le côte à côte qu'on venait d'installer.
    pub(crate) fn montrer_vue(&mut self, vue: VueCentrale) {
        if !self.vue_dans_la_rangée(vue) {
            return;
        }
        self.vue_centrale = vue;
        // CE QUE L'ANALYSE A BESOIN DE SAVOIR EN S'OUVRANT : quels
        // outils extérieurs sont installés. La question coûte un
        // parcours du PATH, parfois un lancement, et ne se pose donc
        // qu'à l'ouverture. Elle était posée par le bouton du panneau
        // latéral, c'est-à-dire par un seul des quatre chemins qui
        // mènent à cette vue — l'onglet, le menu et la palette
        // montraient « outil absent » faute d'avoir demandé.
        if vue == VueCentrale::Analysis {
            self.deliver_tool_states();
        }
    }

    /// Ôte cette vue de devant les yeux, où qu'elle soit.
    ///
    /// Un seul geste le demande, et il vaut la peine d'être nommé :
    /// « montre-moi cette adresse dans le listing », depuis l'analyse.
    /// Ce qu'on va voir est DERRIÈRE la vue qui le dit — la fenêtre le
    /// recouvre, l'onglet occupe sa place —, et un clic qui défile un
    /// listing invisible passe pour un clic qui ne fait rien. C'était
    /// déjà vrai de la fenêtre, et l'onglet n'y change qu'une chose :
    /// on la referme en revenant à la source.
    pub(crate) fn masquer_vue(&mut self, vue: VueCentrale) {
        if self.vue_détachée(vue) {
            self.poser_fenêtre_de_vue(vue, false);
        } else if self.vue_centrale == vue {
            self.vue_centrale = VueCentrale::Source;
        }
    }

    /// La rangée elle-même.
    pub(crate) fn views_bar_ui(&mut self, ui: &mut egui::Ui) {
        // La vue active a pu partir en fenêtre depuis la dernière image
        // — ou l'être déjà au chargement des réglages, ou son module
        // avoir été éteint dans les préférences. On ne peint pas une
        // rangée dont l'onglet choisi n'est plus là.
        if !self.vue_dans_la_rangée(self.vue_centrale) {
            self.vue_centrale = VueCentrale::Source;
        }
        let theme = self.theme;
        let s = strings(self.lang);
        let active = self.vue_centrale;

        // Les vues présentes, dans l'ordre, avec le titre entier en
        // infobulle quand l'onglet a dû raccourcir son nom.
        let présentes: Vec<VueCentrale> = VueCentrale::ALL
            .into_iter()
            .filter(|vue| self.vue_dans_la_rangée(*vue) && self.vue_au_niveau(*vue))
            .collect();
        let étiquettes: Vec<(&str, Option<&str>)> = présentes
            .iter()
            .map(|vue| {
                let label = vue.label(s);
                let titre = vue.titre(s);
                (label, (titre != label).then_some(titre))
            })
            .collect();
        let actif = présentes.iter().position(|vue| *vue == active).unwrap_or(0);

        // « Détacher » ne s'adresse qu'à la vue active : sa largeur est
        // nulle quand il n'y a rien à détacher, et la rangée en dispose
        // d'autant.
        //
        // LA GOUTTIÈRE EST COMPTÉE AVEC LUI. Sans elle, une disposition
        // de droite à gauche le colle au bord de la zone centrale : sur
        // la capture, son liseré touchait le trait du panneau latéral,
        // et deux cadres qui se touchent se lisent comme un seul.
        //
        // Et sa place est RETENUE D'AVANCE, sur toutes les lignes : la
        // règle précédente — « au bout de la dernière ligne s'il y
        // tient » — se lisait mal dès qu'il n'y tenait pas, la capture
        // montrant un bouton seul sur une deuxième ligne, à vingt
        // pixels de l'onglet qu'il commande. Une commande se cherche à
        // sa place.
        let largeur_bouton = if active.détachable() {
            let galette = ui.painter().layout_no_wrap(
                s.view_detach.to_owned(),
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::PLACEHOLDER,
            );
            galette.size().x + ui.spacing().button_padding.x * 2.0 + 4.0 + ONGLET_MARGE * 2.0
        } else {
            0.0
        };

        // LE COMPTEUR DE PROBLÈMES, à gauche de « Détacher ».
        //
        // Demandé à l'écran, capture à l'appui : le badge de CLion,
        // « erreurs et warning », en haut à droite de l'éditeur. Ici la
        // rangée des vues EST le haut de l'éditeur, et le coin droit
        // est déjà pris par « Détacher » — un bouton qu'on sait trouver
        // là. Le compteur se pose à sa gauche plutôt que de le
        // déplacer : on ne range pas ailleurs ce que la main connaît.
        let (erreurs, avertissements) = self.compte_des_problemes();
        let largeur_compteur = if erreurs + avertissements > 0 {
            // Trois chiffres au plus par nombre, deux chevrons, les
            // marges : une réserve large plutôt que juste, le trop-plein
            // d'une rangée d'onglets se payant d'une deuxième ligne.
            let chiffres = ONGLET_HAUTEUR * 0.68 + 3.0 + 18.0 + 6.0;
            chiffres * (usize::from(erreurs > 0) + usize::from(avertissements > 0)) as f32
                + ONGLET_HAUTEUR * 1.8
                + 10.0
        } else {
            0.0
        };

        let mut détacher = false;
        let mut problèmes = None;
        let cliqué = rangée_d_onglets_repliée(
            ui,
            theme,
            &étiquettes,
            actif,
            largeur_bouton + largeur_compteur,
            // UNE SEULE LIGNE : c'est ici que le repli coûtait l'écran.
            Repli::UneLigne,
            // PETIT, et c'est une question de rang : un bouton de pleine
            // taille fait vingt-sept pixels dans une rangée qui en fait
            // vingt-six — il la remplit d'un bord à l'autre et pèse plus
            // que les onglets qu'il accompagne. La rangée dit ce qu'on
            // regarde ; « Détacher » n'est qu'un déménagement.
            |ui| {
                // ET SEULEMENT S'IL Y A QUELQUE CHOSE À DÉTACHER.
                //
                // La place du bouton était déjà mise à zéro dans ce
                // cas (`largeur_bouton` ci-dessus) ; il se peignait
                // quand même, et `détacher_vue` refusait en silence.
                // Relevé à l'écran le 10 novembre 2026, sur l'Établi,
                // la source active : « ne sert à rien ». Un bouton
                // qu'on clique sans rien obtenir est pire qu'absent —
                // on le reclique, puis on se demande ce qu'on a mal
                // fait. La source ne se détache pas (voir
                // `VueCentrale::détachable`) : elle est ce qui reste
                // quand une autre part.
                if active.détachable()
                    && small_framed_button(ui, theme, s.view_detach)
                        .on_hover_text(s.view_detach_hint)
                        .clicked()
                {
                    détacher = true;
                }
                problèmes = compteur_de_problemes(
                    ui,
                    theme,
                    erreurs,
                    avertissements,
                    (s.problems_list, s.problems_prev, s.problems_next),
                );
            },
        );
        let choisie = cliqué.map(|rang| présentes[rang]);
        if let Some(vue) = choisie {
            self.vue_centrale = vue;
        }
        if détacher {
            self.détacher_vue(active);
        }
        if let Some(quoi) = problèmes {
            self.aller_au_probleme(quoi);
        }
    }

    /// Ce que la zone centrale peint : la vue choisie dans la rangée.
    pub(crate) fn vue_centrale_ui(&mut self, ui: &mut egui::Ui) {
        if self.vue_centrale == VueCentrale::Source {
            // UN PANNEAU POUR LUI AUSSI, et sans cadre.
            //
            // Il « EST la zone centrale », disait-on, et c'est vrai de
            // sa PLACE ; ce n'était pas vrai de son DÉCOUPAGE. Un Ui
            // ordinaire garde le découpage de son parent : le curseur
            // s'arrête bien au-dessus de la légende de chaleur, mais
            // rien n'empêche ce qu'on y peint de descendre dessous.
            //
            // Relevé à 800×520, console ouverte sur Profil : zone de
            // travail [0, 60]–[420, 205,8], légende de 205,8 à 232,8,
            // et le numéro de ligne « 4 » peint de 205,0 à 220,0 —
            // quatorze pixels par-dessus la légende.
            //
            // `Frame::NONE` : la marge des autres vues n'a pas lieu
            // d'être ici, l'éditeur ayant sa gouttière pour écarter le
            // texte du bord.
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| self.editor_ui(ui));
            return;
        }
        // UN PANNEAU, et non un simple cadre : un panneau borne ce
        // qu'il contient à la place qui reste et le rogne. Sans cela,
        // une vue trop à l'étroit peint par-dessus la console — c'est
        // ce que faisait le listing à 800×520, où il ne lui restait que
        // deux pixels. Dans sa fenêtre, le cadre le rognait sans qu'on
        // ait à y penser ; dans la rangée, il faut le dire.
        //
        // La marge est posée ICI, et non dans la vue : détachée,
        // celle-ci est dans une fenêtre qui a déjà la sienne, et les
        // deux se seraient ajoutées. Sans elle, « Moteur : » touchait
        // le bord gauche de l'écran — l'éditeur, lui, a sa gouttière
        // pour l'en écarter.
        let vue = self.vue_centrale;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 4)))
            .show(ui, |ui| match vue {
                // L'éditeur est traité plus haut : il n'est pas dans un
                // panneau, il EST la zone centrale.
                VueCentrale::Source => {}
                VueCentrale::Disassembly => self.disassembly_view_ui(ui),
                VueCentrale::Flow => self.flow_body_ui(ui),
                VueCentrale::Decompiler => self.decompiler_body_ui(ui),
                VueCentrale::Analysis => self.analysis_body_ui(ui),
                VueCentrale::Timeline => self.timeline_body_ui(ui),
                VueCentrale::Breakpoints => self.breakpoints_body_ui(ui),
                // Les registres étendus sont bornés par la place du
                // panneau, relevée AVANT qu'ils ne peignent : c'est la
                // hauteur du panneau, que son contenu ne décide pas, et
                // non la place qu'il en reste — celle-là se reboucle.
                VueCentrale::Vectors => {
                    let place = ui.available_height();
                    self.vectors_body_ui(ui, place)
                }
                VueCentrale::Patches => self.patches_body_ui(ui),
            });
        // LES DEMANDES DES MODULES D'ANALYSE, relevées APRÈS le dessin,
        // comme le fait la fenêtre : un module dit « montre-moi cette
        // adresse dans le listing », ce qui fait partir la vue —
        // l'appliquer pendant qu'elle se peint serait l'appliquer à
        // une vue qui tient encore l'écran.
        //
        // Personne d'autre ne les relève : les modules ne sont peints
        // qu'ici et dans la fenêtre, et une demande qu'on ne relève pas
        // est un clic qui ne fait rien.
        if vue == VueCentrale::Analysis {
            self.apply_re_requests();
        }
    }
}

impl NamsdecApp {
    /// Ce que l'assembleur a relevé : les erreurs, les avertissements.
    ///
    /// Comptés et non devinés : le panneau Diagnostics montre les mêmes
    /// lignes, et deux comptes différents à l'écran pour la même chose
    /// seraient pires qu'aucun.
    pub(crate) fn compte_des_problemes(&self) -> (usize, usize) {
        let erreurs = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == namsdec_asm::Severity::Error)
            .count();
        (erreurs, self.diagnostics.len() - erreurs)
    }

    /// Le compteur a été cliqué : montrer la liste, ou sauter d'un
    /// problème au suivant.
    pub(crate) fn aller_au_probleme(&mut self, quoi: Problèmes) {
        if quoi == Problèmes::Liste {
            // La liste est un onglet de la console : on l'ouvre, et on
            // s'y installe. Ouvrir la console sans changer d'onglet
            // montrerait la sortie du programme à qui vient de cliquer
            // sur un compte d'erreurs.
            //
            // ET C'EST UNE BASCULE, demandé à l'écran le 10 novembre
            // 2026, une flèche rouge sur le compteur : « ça devrait
            // être une bascule ». Le compte ouvrait la console et rien
            // ne la refermait par là — on cliquait une deuxième fois,
            // il ne se passait rien, et il fallait aller chercher
            // ailleurs ce qu'on venait d'ouvrir ici.
            //
            // REFERMER NE VAUT QUE SI L'ON EST DÉJÀ SUR LA LISTE : la
            // console ouverte sur la sortie du programme, un clic sur
            // le compte doit montrer les problèmes, pas tout fermer.
            // C'est le même geste dans les deux cas — « montre-moi les
            // problèmes » —, et il ne se renverse qu'une fois rendu.
            if self.show_console && self.console_tab == ConsoleTab::Diagnostics {
                self.show_console = false;
                return;
            }
            self.show_console = true;
            self.console_tab = ConsoleTab::Diagnostics;
            return;
        }
        // DES LIGNES, ET NON DES DIAGNOSTICS : on saute d'un endroit du
        // texte à un autre, et deux diagnostics sur la même ligne
        // désignent le même endroit. Triées pour que « d'après » ait un
        // sens, dédoublonnées pour que la liste dise des endroits.
        let mut lignes: Vec<u32> = self
            .diagnostics
            .iter()
            .map(|d| d.line)
            .filter(|ligne| *ligne > 0)
            .collect();
        lignes.sort_unstable();
        lignes.dedup();
        if lignes.is_empty() {
            return;
        }
        let ici = self.cursor_pos.0 as u32;
        // ON BOUCLE, plutôt que de buter : le dernier problème mène au
        // premier. C'est la règle des onglets, et pour la même raison —
        // on parcourt trois erreurs avec la même flèche, sans avoir à
        // se demander où l'on est dans la file.
        let cible = match quoi {
            Problèmes::Suivant => lignes
                .iter()
                .find(|ligne| **ligne > ici)
                .copied()
                .unwrap_or(lignes[0]),
            _ => lignes
                .iter()
                .rev()
                .find(|ligne| **ligne < ici)
                .copied()
                .unwrap_or_else(|| *lignes.last().expect("non vide")),
        };
        self.goto_line = Some(cible);
    }
}
