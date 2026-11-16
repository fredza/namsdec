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

/// Les indicateurs qu'on lit après une opération, dans l'ordre où le
/// processeur les range (par numéro de bit). Partagés par les pastilles
/// du bandeau et le récit du dernier pas : les deux doivent parler des
/// mêmes.
///
/// DF y figure, avec les six d'arithmétique : il ne change pas sur un
/// `add`, mais `std`/`cld` le posent et les instructions de chaîne
/// (`movsb`, `stosb`…) le lisent pour savoir dans quel sens avancer. Le
/// taire laissait croire qu'il n'existait pas — « tout doit s'afficher ».
const FLAG_BITS: [(&str, RFlags); 7] = [
    ("CF", RFlags::CF),
    ("PF", RFlags::PF),
    ("AF", RFlags::AF),
    ("ZF", RFlags::ZF),
    ("SF", RFlags::SF),
    ("DF", RFlags::DF),
    ("OF", RFlags::OF),
];

/// La hauteur de fenêtre en dessous de laquelle le détail de
/// l'instruction ne se déplie pas, au mode simple.
///
/// La bande de la ligne courante y est bornée (`LIGNE_MAX`) et ne
/// défile pas : ce qui dépasse se peint sur la zone du code. Mesuré à
/// 800x520 — le plus petit écran que l'application accepte —, où
/// quatre lignes de détail et le verdict d'un saut conditionnel font
/// tomber l'adresse de la prochaine instruction sur « CE QUE J'AI
/// ÉCRIT ».
///
/// Sept cents : au-dessus, les cinq bandes et le volet tiennent, et le
/// détail avec eux.
const DÉTAIL_MINIMUM: f32 = 700.0;

impl NamsdecApp {
    pub(crate) fn sidebar_ui(&mut self, ui: &mut egui::Ui) {
        // RE est masqué par défaut (activable dans Préférences > Modules) :
        // s'il vient d'être caché pendant qu'il était actif, on retombe
        // sur la pile plutôt que de laisser un panneau vide.
        if self.active_tab == SidebarTab::Re && !self.show_re_tab {
            self.active_tab = SidebarTab::Cpu;
        }

        // DEUX ZONES, ET NON TROIS. Le bandeau — ce qui vient, ce qui
        // vient de se passer — puis la vue choisie, qui prend tout le
        // reste.
        //
        // L'état du processeur occupait jusqu'ici une troisième zone,
        // ancrée entre les deux et hors de toute zone défilante, au nom
        // d'un principe juste : ce qu'on surveille entre deux pas ne
        // doit pas se chercher. Le prix en était un partage à trois qui
        // ne tombait jamais juste — trois correctifs successifs pour la
        // même famille de recouvrements, et au zoom de travail de Fred
        // les registres prenaient près de soixante pour cent du panneau
        // en écrasant la vue d'à côté. Ils ont maintenant leur onglet,
        // le premier, et chaque vue a toute la place.
        //
        // La part du bas est posée **avant** le bandeau : sans cela, le
        // bandeau prend ce qu'il veut et la vue hérite de ce qui reste —
        // deux lignes, quand il en faut vingt pour lire une analyse.
        // CE QUE LE PANNEAU DU BAS PREND, SU D'AVANCE.
        //
        // `ui.available_height()` ment après qu'un panneau s'est posé :
        // à 800×520, console ouverte sur Profil, le panneau du bas
        // occupe 84,4 px et le parent n'en défalque que 69,8. Le
        // bandeau croit donc avoir quinze pixels de plus qu'il n'en a,
        // les remplit, et sa dernière ligne se peint sur la rangée
        // d'onglets.
        //
        // La part du bas sait, elle, ce qu'elle a demandé : elle le
        // rend, et c'est ce chiffre-là qui borne le bandeau.
        let total = ui.available_height();
        let prise = self.sidebar_tab_panel_ui(ui);

        // Ce qui vient avant : la prochaine instruction, le dernier pas,
        // les conseils. Repliables, et c'est à eux de céder quand la
        // fenêtre est courte.
        // CE QU'IL RESTE, RELEVÉ AVANT. La part du bas est déjà posée ;
        // ce qui reste au-dessus est exactement ce que le bandeau peut
        // prendre, et il faut le lui DIRE. `auto_shrink` ne suffit pas
        // sur un panneau très court : relevé à 800×520, console
        // ouverte sur Profil, le bandeau s'arrêtait à 146 et peignait
        // « Détail » jusqu'à 161 — par-dessus la rangée d'onglets.
        let place = (total - prise).max(0.0);
        let bandeau = egui::ScrollArea::vertical()
            .id_salt("sidebar_banner")
            .max_height(place)
            // `false` AUSSI EN HAUTEUR : réglée sur « vrai », la zone
            // épouse son contenu, et sur un panneau très court elle
            // dépassait de quelques pixels la place qui lui restait —
            // la dernière ligne du bandeau se peignait alors par-dessus
            // ce qui suit. Mesuré : « Dernier pas » de 88 à 106 pour
            // une place qui s'arrêtait à 92.
            //
            // Bornée, elle prend exactement ce qui reste et fait
            // défiler le reste. Le partage, lui, ne s'en trouve pas
            // changé : il se calcule sur `content_size`, c'est-à-dire
            // sur ce que le bandeau VOUDRAIT, et non sur ce qu'il a.
            .auto_shrink([false, false])
            // Et sans plancher : une zone défilante refuse par défaut
            // de descendre sous soixante-quatre pixels (`min_scrolled_
            // height`). Sur un panneau très court, elle prenait donc
            // ces soixante-quatre pixels même quand il n'en restait que
            // quarante-trois — et les vingt et un autres se peignaient
            // sur la vue du dessous.
            .min_scrolled_height(0.0)
            .show(ui, |ui| self.sidebar_banner_ui(ui));
        // Ce que le bandeau mesurerait s'il avait toute la place : c'est
        // cette hauteur qui décide, à l'image suivante, de la part de
        // la vue — et le partage se stabilise dès que tout tient.
        self.sidebar_banner_height = bandeau.content_size.y;
    }

    /// Le haut du bandeau : la prochaine instruction et le dernier pas.
    fn sidebar_banner_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        // Au-dessus de tout : c'est ce qu'on regarde entre deux pas, et
        // il ne doit jamais falloir aller le chercher.
        // Repliable comme l'état du processeur : sept lignes devant une
        // instruction qu'on a déjà comprise repoussent le reste du
        // panneau, et l'on passe son temps à faire défiler.
        let mut ouvert = self.next_instruction_open;
        let déplié = section_toggle(ui, self.theme, s.next_instruction, &mut ouvert);
        self.next_instruction_open = ouvert;
        if déplié {
            self.next_instruction_ui(ui);
        }
        self.last_step_ui(ui);
        self.advice_ui(ui);
    }

    /// La vue « Processeur » : les registres, le coût, et le lien vers
    /// les vectoriels.
    ///
    /// Un onglet depuis le 13 octobre 2026, et non plus un bandeau
    /// ancré : voir `SidebarTab` pour le pourquoi. Ce qu'elle montre
    /// n'a pas changé d'un pixel — seule sa place a changé, et avec
    /// elle la certitude qu'elle ne mange plus la vue d'à côté.
    fn cpu_tab_ui(&mut self, ui: &mut egui::Ui) {
        // `registers_ui` porte déjà l'état vide, et c'est le seul des
        // six à proposer le bouton « Assembler » : cette vue-ci est
        // celle qu'on a sous les yeux au premier lancement.
        self.registers_ui(ui);
        if self.emulator.is_none() {
            return;
        }
        // Les indicateurs ne sont pas ici : ils vivent dans la barre de
        // titre, toujours visible. Voir `flags_ui`.
        self.cost_ui(ui);
        // Les vectoriels non plus : seize registres SSE et huit
        // emplacements x87, c'est vingt-quatre lignes de plus dans une
        // vue qui en a déjà dix-huit. Ils ont une fenêtre à eux, où ils
        // tiennent en entier.
        self.vectors_link_ui(ui);
        // Le but de l'exercice n'est plus ici non plus : il vivait dans
        // le bandeau toujours visible, et restait donc affiché quand on
        // chargeait un binaire sans rapport. Il est dans la fenêtre des
        // exercices (Aide › Exercices). Voir `goal_ui`.
    }

    /// La part du panneau réservée au contenu de l'onglet.
    ///
    /// Un panneau à part, avec sa poignée : ce que la souris a choisi
    /// est retenu, et l'ajustement automatique s'arrête là — exactement
    /// comme la console. Tant que rien n'a été tiré, la part suit ce que
    /// le bandeau laisse, sans jamais tomber sous [`SIDEBAR_TAB_MIN`].
    ///
    /// Rend la hauteur qu'elle a demandée : le bandeau s'en sert pour
    /// se borner, `ui.available_height()` n'étant pas fiable après
    /// qu'un panneau s'est posé.
    fn sidebar_tab_panel_ui(&mut self, ui: &mut egui::Ui) -> f32 {
        let s = strings(self.lang);
        let total = ui.available_height();
        let haut_du_panneau = ui.max_rect();
        // Ce que la vue peut prendre au plus : il reste toujours au
        // bandeau de quoi montrer ce qui vient, et de quoi le replier —
        // un titre et son filet, sans quoi le repli n'aurait plus où se
        // poser.
        //
        // Le plancher était naguère calculé sur la hauteur de l'état du
        // processeur, qui occupait une troisième zone entre les deux.
        // Elle n'existe plus : les registres sont une vue comme les
        // autres, et le partage n'a plus que deux parts à faire.
        const TITRE_REPLIÉ: f32 = 44.0;
        // Ce qu'on garde au bandeau est ce qu'il DEMANDE, et non une
        // constante : replié, il tient en un titre et son filet, et
        // retenir cent vingt pixels pour lui en laissait soixante-dix
        // de vide au-dessus de la vue — visible sur capture, un trou
        // entre « Prochaine instruction » replié et le titre de la vue.
        // La constante ne sert plus que de plafond : au-delà, c'est au
        // bandeau de défiler.
        let plancher = self
            .sidebar_banner_height
            .clamp(TITRE_REPLIÉ, SIDEBAR_BANNER_MIN)
            .min(total * 0.5);
        // UNE GARDE ENTRE LES DEUX PARTS, et elle vient d'une mesure.
        //
        // À 800×520, console ouverte sur Profil, le panneau du bas
        // occupe 84,4 px et le parent n'en défalque que 69,8 : quatorze
        // pixels et demi d'écart — une hauteur de ligne — que le
        // bandeau croit libres et remplit. Sa dernière ligne, « Détail »,
        // se peignait alors sur la rangée d'onglets, relevé par
        // `panels_do_not_overlap_on_the_smallest_screen`.
        //
        // On ne demande donc jamais TOUT ce qui reste. Seize pixels sur
        // un panneau qui en fait mille ne se voient pas ; sur un
        // panneau qui en fait cent soixante-dix, ils séparent deux
        // zones au lieu de les laisser se chevaucher.
        const GARDE_ENTRE_LES_PARTS: f32 = 16.0;
        let plafond = (total - plancher - GARDE_ENTRE_LES_PARTS).max(0.0);
        // Ce que le bandeau laisse, mesuré à l'image précédente. Au
        // tout premier passage on ne sait rien : la moitié du panneau
        // est un départ honnête, et l'image suivante corrige.
        let laissée = if self.sidebar_banner_height > 0.0 {
            // Quelques pixels de garde : sans eux, le partage se cale
            // pile sur la hauteur du bandeau, et le moindre pixel de
            // plus — une ligne de séparation, un caractère plus haut —
            // fait apparaître une barre de défilement au milieu des
            // registres.
            total - self.sidebar_banner_height - 12.0
        } else {
            total * 0.5
        };
        // Sous le plancher, on prend quand même le plancher : un onglet
        // de deux lignes ne montre rien, et ce qui cède alors est le
        // détail de la prochaine instruction, qui se replie d'un clic.
        let voulue = laissée.clamp(SIDEBAR_TAB_MIN.min(plafond), plafond);
        // LA HAUTEUR RETENUE EST UN PLANCHER, PAS UN PLAFOND.
        //
        // C'était l'inverse, et cela rendait l'onglet Mémoire illisible
        // — le grief exact, et il se mesure : sur un panneau de
        // 2 460 px, « Prochaine instruction » replié demande une
        // cinquantaine de pixels, et l'onglet en recevait 307. Les deux
        // mille pixels du milieu étaient du vide. Ce que le partage
        // avait retenu d'un geste de poignée — ou d'un faux témoignage,
        // voir plus bas — gelait l'onglet pour toutes les séances
        // suivantes, puisqu'il est enregistré dans les réglages.
        //
        // Deux planchers, et le plus grand gagne : ce que la personne a
        // choisi à la poignée, et ce que le bandeau N'UTILISE PAS. On
        // peut donc toujours agrandir l'onglet au détriment du bandeau
        // (qui défile), mais plus le rétrécir pour laisser du vide — ce
        // qui n'est pas un réglage mais le défaut lui-même.
        let voulue = self
            .sidebar_tab_height
            .map_or(voulue, |gardée| gardée.max(voulue))
            .clamp(SIDEBAR_TAB_MIN.min(plafond), plafond);
        let id = egui::Id::new("sidebar_tab");
        // L'état du panneau est RÉÉCRIT tant qu'aucune poignée n'est
        // sous un doigt. Sans cela, egui garde ce qu'il avait à la
        // première image et le partage ne suit plus le bandeau : c'est
        // ce qui figeait l'onglet à sa hauteur d'il y a trois semaines.
        let tirée = ui.ctx().input(|i| i.pointer.primary_down());
        let actuelle = egui::containers::panel::PanelState::load(ui.ctx(), id)
            .map(|état| état.outer_rect.height());
        if !tirée && actuelle.is_none_or(|haut| (haut - voulue).abs() > 1.0) {
            // LE RECTANGLE EST POSÉ OÙ LE PANNEAU VIT, et non à
            // l'origine. C'était `Pos2::ZERO` : on ne voulait écrire
            // qu'une HAUTEUR, et egui lit aussi la position — il
            // plaçait alors le panneau quatorze pixels sous le bas de
            // son parent, et la rangée d'onglets se peignait sur le
            // bandeau. Mesuré à 800×520, console ouverte sur Profil :
            // l'état écrit valait [[429 0] – [792 84]] pour un parent
            // qui s'arrête à 230,8, et le panneau sortait à 245,4.
            //
            // Défaut invisible jusqu'ici faute de scène : les trois
            // onglets de la console n'étaient dans aucune, et c'est
            // leur hauteur qui rend le panneau assez court pour que le
            // décalage se voie.
            let parent = haut_du_panneau;
            let état = egui::containers::panel::PanelState {
                outer_rect: egui::Rect::from_min_max(
                    egui::pos2(parent.left(), parent.bottom() - voulue),
                    egui::pos2(parent.right(), parent.bottom()),
                ),
            };
            ui.ctx().data_mut(|data| data.insert_persisted(id, état));
        }
        egui::Panel::bottom(id)
            .resizable(true)
            .frame(egui::Frame::new())
            .default_size(voulue)
            .min_size(SIDEBAR_TAB_MIN.min(plafond))
            .max_size(plafond)
            .show(ui, |ui| {
                // LES ONGLETS, EN TÊTE DU PANNEAU, juste au-dessus de ce
                // qu'ils commandent.
                //
                // Ils vivaient dans la barre d'outils, en haut à droite,
                // et le panneau portait à leur place le TITRE de la vue
                // active. Deux défauts tenaient à cette distance : une
                // étiquette séparée de son contenu ne se lit plus comme
                // son étiquette — on cherchait en haut de l'écran de
                // quoi parlait une colonne de nombres —, et la rangée ne
                // tenait pas dans la barre en dessous de mille pixels,
                // où elle passait à la ligne et grandissait la barre de
                // trente-et-un pixels pour toute la séance.
                //
                // Le titre disparaît avec le déplacement : une rangée
                // qui souligne l'onglet actif EST le titre de ce qu'on
                // regarde, et le répéter en dessous ne faisait que
                // prendre une ligne de plus dans un panneau dense.
                //
                // La forme est celle de la rangée des vues, et c'est le
                // même corps qui la peint (`rangée_d_onglets`) : ce sont
                // la même chose — des façons de regarder un même objet.
                // MÊME RÈGLE QUE LA RANGÉE DES VUES : le niveau
                // allège, et ce qu'on regarde reste listé — un onglet
                // actif sans entrée est un endroit dont on ne sait plus
                // sortir.
                let niveau = self.niveau();
                let onglets: Vec<SidebarTab> = SidebarTab::ALL
                    .into_iter()
                    .filter(|tab| *tab != SidebarTab::Re || self.show_re_tab)
                    .filter(|tab| tab.niveau() <= niveau || *tab == self.active_tab)
                    .collect();
                let étiquettes: Vec<(&str, Option<&str>)> = onglets
                    .iter()
                    .map(|tab| {
                        let label = tab.label(s);
                        let titre = tab.title(s);
                        (label, (titre != label).then_some(titre))
                    })
                    .collect();
                let actif = onglets
                    .iter()
                    .position(|tab| *tab == self.active_tab)
                    .unwrap_or(0);
                // La réserve du bouton « cacher » : sa case et sa
                // gouttière, retenues sur la première ligne.
                let mut cacher = false;
                if let Some(rang) = rangée_d_onglets(
                    ui,
                    self.theme,
                    &étiquettes,
                    actif,
                    ONGLET_HAUTEUR + 4.0,
                    |ui| {
                        cacher = bouton_cacher(ui, self.theme, s.rail_sidebar_hide);
                    },
                ) {
                    self.active_tab = onglets[rang];
                }
                if cacher {
                    self.show_sidebar = false;
                }
                ui.add_space(2.0);
                // LE CONTENU EST BORNÉ À LA PLACE DU PANNEAU, et ce
                // n'est pas un confort : c'est ce qui empêche l'état du
                // processeur de se peindre par-dessus.
                //
                // Mesuré, parce que la cause est contre-intuitive.
                // Quand le contenu d'un panneau egui dépasse la place
                // qu'on lui a laissée, egui ramène bien la TAILLE du
                // rectangle au maximum demandé — mais en gardant le
                // bord du CONTENU, non celui de la place disponible.
                // Le rectangle glisse donc vers le bas, et le panneau
                // ne rend au parent que ce qui reste visible : relevé
                // sur un panneau de 490 px avec l'onglet Mémoire, le
                // panneau peignait de 361 à 539 et n'en rendait que de
                // 403 à 539. L'état du processeur, qui se place sur ce
                // que le parent a rendu, s'écrivait alors sur les
                // quarante-deux pixels de différence — « Instructions
                // exécutées » par-dessus le titre « Mémoire », et au
                // zoom de Fred tout le haut de l'onglet.
                //
                // Borné, le rectangle ne glisse plus : la vue défile
                // au lieu de déborder. Défiler ici est permis — ce qui
                // ne doit jamais défiler, c'est l'état du processeur,
                // et il est hors de cette zone.
                //
                // Chaque vue garde la sienne par-dessus : un vidage
                // mémoire défile dans les deux sens (seize octets et
                // leur transcription, c'est plus large que le panneau),
                // là où une commande — le champ Adresse — reste bornée
                // par le panneau. Une zone horizontale commune les
                // étirerait toutes les deux à la largeur du vidage.
                // LA VUE EST BORNÉE À LA PLACE DU PANNEAU, et elle y
                // défile. C'est ce qui empêche la zone d'à côté d'être
                // recouverte.
                //
                // Laisser la vue grandir à sa guise : quand son contenu
                // dépasse la place laissée, egui ramène la TAILLE du
                // rectangle du panneau au maximum demandé en gardant le
                // bord du CONTENU et non celui de la place. Le
                // rectangle glisse vers le bas et le panneau ne rend au
                // parent que ce qui reste visible — relevé : peint de
                // 361 à 539, rendu de 403 à 539, et le voisin s'écrit
                // dans les quarante-deux pixels d'écart.
                //
                // La hauteur est imposée DANS la zone défilante : sans
                // cela, le contenu d'une zone qui défile se croit une
                // hauteur sans borne, et les vues qui dimensionnent
                // leur propre liste dessus cessent de défiler elles-
                // mêmes. Imposée, chacune retrouve la hauteur vraie, et
                // la zone du dessus ne sert que lorsque tout ne tient
                // pas — un très petit écran, un zoom généreux.
                let hauteur = ui.available_height();
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_tab_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_height(hauteur);
                        match self.active_tab {
                            SidebarTab::Cpu => self.cpu_tab_ui(ui),
                            SidebarTab::Stack => self.stack_ui(ui),
                            SidebarTab::Memory => self.memory_ui(ui),
                            SidebarTab::Heap => self.heap_ui(ui),
                            SidebarTab::Bytes => self.bytes_ui(ui),
                            SidebarTab::Re => self.re_ui(ui),
                        }
                    });
            });
        // Ce que le panneau fait vraiment : s'il n'est pas à la hauteur
        // demandée ET qu'un bouton est enfoncé, c'est que la poignée a
        // été tirée.
        //
        // LE BOUTON ENFONCÉ EST LA CLÉ, et son absence a coûté cher. À
        // la première image, le partage ne connaît encore ni la hauteur
        // du bandeau ni celle de l'état du processeur : il demande la
        // moitié du panneau, egui en rend autre chose, et l'on concluait
        // « la poignée a été tirée ». La part de l'onglet était alors
        // gelée à son plancher pour toute la séance, et le bandeau
        // gardait le reste — quatre cent cinquante-six pixels de vide
        // sous « Prochaine instruction » sur l'écran d'accueil, que
        // trois tentatives de rééquilibrage n'avaient pas expliqués
        // parce qu'elles cherchaient un défaut de partage là où il y
        // avait un faux témoignage.
        //
        // Une poignée ne bouge que sous un doigt. C'est la seule
        // question à poser.
        if let Some(rendue) = egui::containers::panel::PanelState::load(ui.ctx(), id)
            .map(|état| état.outer_rect.height())
        {
            self.sidebar_tab_last = rendue;
            match self.sidebar_tab_height {
                Some(gardée) if tirée && (rendue - gardée).abs() > 1.0 => {
                    self.sidebar_tab_height = Some(rendue);
                }
                None if tirée && (rendue - voulue).abs() > 1.0 => {
                    self.sidebar_tab_height = Some(rendue);
                }
                _ => {}
            }
        }
        // CE QU'ELLE A VRAIMENT PRIS, et non ce qu'elle a demandé.
        //
        // Les deux coïncident presque toujours ; ils divergent pendant
        // qu'une poignée est sous un doigt, où c'est egui qui décide.
        // Rendre la demande bornerait alors le bandeau sur une hauteur
        // que personne n'occupe, et l'on reverrait le recouvrement
        // pendant le geste — c'est-à-dire au seul moment où l'on
        // regarde ces deux zones ensemble.
        // LA PLUS GRANDE DES DEUX : ce qui a été pris et ce qui a été
        // demandé.
        //
        // Elles coïncident presque toujours ; elles divergent pendant
        // qu'une poignée est sous un doigt, où c'est egui qui décide.
        // Borner le bandeau sur la plus petite le laisserait déborder
        // pendant le geste — c'est-à-dire au seul moment où l'on
        // regarde ces deux zones ensemble.
        self.sidebar_tab_last.max(voulue).clamp(0.0, total)
    }

    /// La question d'un exercice de LECTURE : on lit, on répond, puis on
    /// exécute.
    ///
    /// Rend `true` quand elle a pris la parole — l'appelant n'a alors
    /// rien d'autre à dire : ces exercices n'ont pas de but, et le bloc
    /// des buts se tairait de toute façon.
    ///
    /// **Le verdict ne révèle pas la réponse.** « Pas celle-là » et
    /// rien de plus : la bonne réponse s'obtient en exécutant le
    /// programme, ce que l'exercice demande de faire ENSUITE. La
    /// donner ici retirerait le seul geste qu'il enseigne — et c'est
    /// exactement le défaut que « prévoir avant d'avancer » corrigeait
    /// dans le bandeau, à l'échelle d'une instruction.
    fn question_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let s = strings(self.lang);
        let theme = self.theme;
        let Some((question, verdict)) = self.exercise_question() else {
            return false;
        };
        let énoncé = question.texte.get(self.lang.code()).to_owned();
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.question_label);
            ui.label(&énoncé);
        })
        .response
        .on_hover_text(s.question_hint);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            // ÉTROIT DANS LE PANNEAU, PLUS LARGE DANS LA BANDE.
            //
            // Une réponse est un nombre, pas une phrase, et cent dix
            // pixels suffisent dans un panneau de trois cents. Mais la
            // bande de l'Établi fait toute la largeur de la fenêtre, et
            // le même champ y paraissait minuscule à côté de sa
            // question — au point qu'on cherchait où répondre (relevé
            // sur `établi / un exercice de lecture`, 16 novembre 2026).
            // Il prend donc une part de ce qu'il a, bornée des deux
            // côtés.
            let large = (ui.available_width() * 0.25).clamp(110.0, 260.0);
            let champ = egui::TextEdit::singleline(&mut self.question_answer)
                .desired_width(large)
                .hint_text(s.question_answer)
                .font(egui::TextStyle::Monospace);
            ui.add(champ).on_hover_text(s.question_field_hint);
            use namsdec_exercises::reponse::Verdict;
            match verdict {
                // RIEN D'ÉCRIT N'EST PAS UNE ERREUR : « faux » devant un
                // champ vide dirait à quelqu'un qui réfléchit qu'il
                // s'est trompé, et c'est le moment où l'on abandonne.
                Verdict::Vide => {}
                Verdict::Juste => {
                    ui.colored_label(theme.good, s.question_right);
                }
                Verdict::Faux => {
                    ui.colored_label(theme.warn, s.question_wrong);
                }
            }
        });
        // Et ce qu'il reste à faire, dit une fois : exécuter. La machine
        // est l'arbitre, pas ce panneau.
        if !verdict.est_juste() {
            ui.colored_label(theme.dim, s.question_run_to_see);
        }
        true
    }

    /// Où en est le but de l'exercice, quand il y en a un.
    ///
    /// Un énoncé qui dit « RAX doit valoir 15 » laisse l'élève vérifier
    /// à l'œil, et se tromper de registre. La machine sait comparer :
    /// autant qu'elle le dise, là où l'on regarde déjà les valeurs.
    pub(crate) fn goal_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        // LA QUESTION D'ABORD, quand il y en a une : un exercice de
        // lecture donne un programme complet, et ce qu'on y fait est
        // répondre. Elle prend la place du but, qui n'existe pas pour
        // ces exercices-là — et qui, s'il existait, afficherait la
        // réponse sous la question.
        if self.question_ui(ui) {
            return;
        }
        // UNE DÉMONSTRATION LE DIT, au lieu de se taire. Sept exercices
        // n'ont pas de but — ce sont des programmes à regarder tourner
        // —, et le panneau sortait ici sans rien écrire : on cherchait
        // le but qu'on venait de lire dans l'énoncé, il n'y en avait
        // pas, et rien ne disait que c'était normal.
        if self.exercise_is_demo() {
            let fini = self
                .emulator
                .as_ref()
                .is_some_and(|e| e.executed() > 0 && e.cpu().halted);
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let (cadre, _) =
                    ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                let couleur = if fini { theme.good } else { theme.warn };
                if fini {
                    ui.painter().circle_filled(cadre.center(), 4.0, couleur);
                } else {
                    ui.painter().circle_stroke(
                        cadre.center(),
                        3.5,
                        egui::Stroke::new(1.5, couleur),
                    );
                }
                ui.colored_label(theme.dim, s.goal_demo);
                ui.colored_label(
                    couleur,
                    if fini {
                        s.goal_demo_done
                    } else {
                        s.goal_demo_watch
                    },
                );
            });
            return;
        }
        let Some((conditions, atteint)) = self.exercise_goal() else {
            // RIEN N'A TOURNÉ : ON MONTRE QUAND MÊME LA CIBLE.
            //
            // `exercise_goal` se tait tant qu'aucun programme ne
            // tourne, et c'est juste : on ne dit pas « pas atteint » à
            // qui n'a encore rien lancé. Mais se taire fait qu'on écrit
            // SANS SAVOIR OÙ L'ON VA — la cible ne se lisait qu'après
            // le premier essai, alors qu'elle est ce qui dirige ce
            // qu'on écrit.
            //
            // En gris et sans pastille : ce ne sont pas des conditions
            // qui tiennent ou qui manquent, ce sont des conditions
            // qu'on n'a pas encore éprouvées. Aucun verdict n'est posé.
            self.indice_ui(ui);
            if let Some(cibles) = self.exercise_targets() {
                ui.add_space(4.0);
                ui.colored_label(theme.dim, s.goal_ahead);
                let débutant = self.niveau() == namsdec_re::Niveau::Débutant;
                for cible in cibles {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(16.0);
                        ui.colored_label(
                            theme.dim,
                            egui::RichText::new(condition_lisible(&cible, débutant, self.lang))
                                .monospace(),
                        );
                    });
                }
            }
            return;
        };
        let débutant = self.niveau() == namsdec_re::Niveau::Débutant;
        self.indice_ui(ui);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.goal_label);
            ui.colored_label(
                if atteint { theme.good } else { theme.warn },
                if atteint {
                    s.goal_reached
                } else {
                    s.goal_not_yet
                },
            );
        })
        .response
        .on_hover_text(s.goal_hint);

        // Une ligne par condition, et son constat quand elle manque.
        //
        // « pas encore » ne dit rien à qui ne voit pas LAQUELLE manque :
        // un but à trois conditions se lisait comme un verdict global,
        // et l'on relançait au hasard. Écrite, la condition qui manque
        // dit quel registre regarder ; le constat dit ce qu'il porte à
        // la place. C'est le geste qu'on vient apprendre — comparer ce
        // qu'on voulait et ce qu'on a — et non une correction.
        for ligne in &conditions {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                // Un point plein pour ce qui tient, un cercle vide pour
                // ce qui manque : dessinés, parce qu'aucune coche ni
                // aucune croix n'existe dans la police de l'interface.
                let (cadre, _) =
                    ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                let couleur = if ligne.tenue { theme.good } else { theme.warn };
                if ligne.tenue {
                    ui.painter().circle_filled(cadre.center(), 4.0, couleur);
                } else {
                    ui.painter().circle_stroke(
                        cadre.center(),
                        3.5,
                        egui::Stroke::new(1.5, couleur),
                    );
                }
                let texte_condition = condition_lisible(&ligne.texte, débutant, self.lang);
                ui.colored_label(
                    if ligne.tenue { theme.good } else { theme.text },
                    egui::RichText::new(texte_condition.clone()).monospace(),
                );
                if let Some((gauche, valeur)) = &ligne.constat {
                    // LE CONSTAT SUIT LA MÊME RÈGLE QUE LE DERNIER
                    // PAS : sous le million, en décimal au niveau
                    // débutant. Vu à l'écran le 10 novembre 2026, sur
                    // la même image : la bande du bas disait déjà
                    // « RCX 3 -> 2 » pendant que le but, trois
                    // centimètres plus haut, disait « rax vaut 0x0 ».
                    // Deux façons d'écrire le même nombre sur un même
                    // écran, c'est une de trop.
                    //
                    // LA CONDITION, ELLE, RESTE TELLE QUE L'EXERCICE
                    // L'A ÉCRITE (`rax == 0xf`) : c'est son texte, pas
                    // le nôtre, et la réécrire ferait dire à l'énoncé
                    // autre chose que ce que son auteur a tapé.
                    // LE MÊME NOM QUE DANS LA CONDITION, à la même
                    // casse : celle-ci s'écrit « RAX vaut 15 » au
                    // niveau débutant, et le constat disait « rax vaut
                    // 0 » juste à côté. Deux écritures du même registre
                    // sur une seule ligne.
                    let nom = if débutant {
                        gauche.to_uppercase()
                    } else {
                        gauche.clone()
                    };
                    // DEUX PHRASES COLLÉES N'EN FONT QU'UNE, ET ELLE
                    // NE VEUT RIEN DIRE.
                    //
                    // Relevé sur la capture du 16 novembre 2026,
                    // `établi / un exercice d'écriture` : la ligne se
                    // lisait « RAX vaut 2 RAX vaut 1 ». La première
                    // moitié est ce qu'on veut, la seconde ce qu'on a,
                    // et rien ne le disait — pas même un blanc, les
                    // deux étant en chasse fixe et à six pixels l'une
                    // de l'autre. Un tiret sépare, et le registre ne
                    // se répète plus quand c'est le même : « RAX vaut
                    // 2 — il vaut 1 » se lit d'un trait.
                    let répété = texte_condition.starts_with(&nom);
                    let constat = if répété {
                        format!("— {} {}", s.goal_actual, nombre_lisible(*valeur, débutant))
                    } else {
                        format!(
                            "— {nom} {} {}",
                            s.goal_is,
                            nombre_lisible(*valeur, débutant)
                        )
                    };
                    ui.colored_label(theme.warn, egui::RichText::new(constat).monospace());
                }
            });
        }

        // CE QUE LE PROGRAMME A FAIT À LA PLACE, quand le but manque.
        //
        // C'est la moitié de l'explication qui ne se mesurait pas : la
        // condition dit ce que RAX vaut, elle ne dit pas pourquoi. « La
        // boucle a tourné quatre fois » le dit, et c'est le moment
        // précis où quelqu'un qui apprend abandonne ou comprend.
        //
        // Rien n'est déduit : ce sont des compteurs. Un saut dont la
        // suite a d'autres entrées ne dit rien du tout — voir
        // `ce_qui_a_tourné`, qui refuse de conclure là où plusieurs
        // chemins se rejoignent.
        if !atteint {
            let dit = self.ce_que_le_programme_a_fait();
            if !dit.is_empty() {
                // Une entrée en matière, parce que ces phrases parlent
                // d'autre chose que les conditions du dessus : celles-ci
                // disent ce qu'on voulait, celles-là ce qui s'est passé.
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(16.0);
                    ui.colored_label(theme.dim, s.goal_what_ran);
                });
            }
            for phrase in dit {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(24.0);
                    ui.colored_label(theme.text, phrase);
                });
            }
        }

        // Le coût, à côté du verdict : un programme juste peut être deux
        // fois plus court, et c'est en cherchant comment qu'on apprend
        // `lea`, les décalages, ou qu'une boucle de cinq tours se
        // remplace par une formule.
        let Some(id) = self.current_exercise.clone() else {
            return;
        };
        let Some(budget) = namsdec_exercises::find(&self.exercise_packs, &id).and_then(|e| e.par)
        else {
            return;
        };
        let exécutées = self.emulator.as_ref().map(|e| e.executed()).unwrap_or(0);
        let meilleur = self.exercises_best.get(&id).copied();
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.par_label);
            ui.colored_label(
                if atteint && exécutées < budget {
                    theme.good
                } else {
                    theme.text
                },
                egui::RichText::new(format!("{exécutées}")).monospace(),
            );
            ui.colored_label(theme.dim, format!("/ {budget}"));
            if let Some(meilleur) = meilleur {
                // Le point médian sépare, comme partout ailleurs dans
                // l'application : « 0 / 13 meilleur 13 » se lisait
                // comme une seule suite de nombres.
                ui.colored_label(theme.dim, format!("·  {} {meilleur}", s.par_best));
            }
        })
        .response
        .on_hover_text(s.par_hint);
    }

    /// L'indice : le bouton tant qu'on ne l'a pas demandé, son texte
    /// ensuite.
    ///
    /// DEUX PALIERS D'AIDE, ET C'ÉTAIT LE MANQUE : entre la consigne du
    /// trou et le programme entier, il n'y avait rien. On relisait la
    /// consigne, on ne trouvait pas, et le seul recours mettait fin à
    /// l'exercice. L'indice dit où chercher sans dire quoi écrire.
    ///
    /// LES INSTRUCTIONS SONT CLIQUABLES : chacune ouvre sa fiche. C'est
    /// le chemin le plus court entre une étape du parcours et les cent
    /// quatre-vingt-neuf fiches du manuel, qui existaient sans que rien
    /// n'y mène depuis un exercice.
    pub(crate) fn indice_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let Some((écrit, manquantes)) = self.exercise_hint() else {
            return;
        };
        // LE BOUTON EST AILLEURS — avec les autres gestes de l'étape,
        // dans la rangée du titre. Posé ici, il coûtait une ligne à la
        // bande, et le budget sortait par le bas : mesuré par
        // `la_bande_de_l_exercice_ne_coupe_pas_sa_derniere_ligne`. Ce
        // qui reste ici est l'indice lui-même, qui ne paraît qu'une
        // fois demandé — et la zone, elle, défile.
        if !self.indice_montré {
            return;
        }
        let mut fiche = None;
        ui.add_space(4.0);
        if let Some(écrit) = écrit {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.colored_label(theme.accent, s.exercise_hint);
                ui.colored_label(theme.text, écrit);
            });
        }
        if !manquantes.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.colored_label(theme.dim, s.exercise_hint_uses);
                for mot in &manquantes {
                    // PAR `cliquable` : le trait au repos et la main au
                    // survol viennent avec. Un texte qui se clique sans
                    // le dire ne sert qu'à celui qui savait déjà — et
                    // le banc le refuse, à juste titre.
                    if cliquable(ui, egui::Label::new(egui::RichText::new(*mot).monospace()))
                        .on_hover_text(s.help_open_sheet)
                        .clicked()
                    {
                        fiche = Some(*mot);
                    }
                }
                // CE QUE L'INDICE NE DIT PAS : il nomme ce que la
                // solution LIVRÉE emploie, et il y a mille façons
                // d'écrire une boucle. Le taire ferait passer un indice
                // pour un verdict.
                ui.colored_label(theme.dim, format!("({})", s.exercise_hint_yours));
            });
        }
        if let Some(mot) = fiche {
            self.open_help(mot);
            self.show_help = true;
        }
    }

    /// Combien d'instructions ont tourné depuis le départ.
    ///
    /// La mesure la plus simple de ce que coûte un programme, et la
    /// seule qui permette de comparer deux façons d'écrire la même
    /// chose : on réécrit sa boucle, on relance, et le nombre baisse —
    /// ou pas.
    fn cost_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let Some(emulator) = &self.emulator else {
            return;
        };
        let executed = emulator.executed();
        if executed == 0 {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.executed_label);
            ui.colored_label(
                ui.visuals().text_color(),
                egui::RichText::new(group_digits(executed)).monospace(),
            );
        })
        .response
        .on_hover_text(s.executed_hint);
    }

    /// Les huit registres de masque d'AVX-512, montrés en BINAIRE.
    ///
    /// En binaire et non en hexadécimal, parce que ce qu'on y lit est
    /// « quelles voies » : un bit par élément du vecteur, et c'est la
    /// forme même de la réponse. Le `0x0f` d'un `kmovd` ne dit pas
    /// qu'il s'agit des quatre premiers octets ; `00001111` le dit.
    ///
    /// Montrés seulement si l'un d'eux porte quelque chose : un
    /// programme qui n'emploie pas AVX-512 n'a pas à lire huit lignes
    /// de zéros, et celui qui l'emploie veut les voir apparaître.
    pub(crate) fn mask_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            return;
        };
        let theme = self.theme;
        let regs = &emulator.cpu().regs;
        if namsdec_core::RegMask::ALL
            .iter()
            .all(|k| regs.get_mask(*k) == 0)
        {
            return;
        }
        ui.add_space(10.0);
        ui.strong(s.mask_registers);
        ui.colored_label(theme.dim, s.mask_registers_hint);
        egui::Grid::new("mask_grid")
            .num_columns(2)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                for k in namsdec_core::RegMask::ALL {
                    let valeur = regs.get_mask(k);
                    ui.colored_label(theme.register, egui::RichText::new(k.name()).monospace());
                    // Trente-deux bits : la largeur d'un masque d'octets
                    // sur un registre de 256, et celle que `kmovd`
                    // déplace. Au-delà, on ne lirait plus.
                    let bits: String = (0..32)
                        .rev()
                        .map(|i| if valeur >> i & 1 == 1 { '1' } else { '0' })
                        .collect();
                    ui.colored_label(
                        ui.visuals().text_color(),
                        egui::RichText::new(bits).monospace(),
                    )
                    .on_hover_text(format!(
                        "{valeur:#x}\n{} {}",
                        valeur.count_ones(),
                        if self.lang == Lang::Fr {
                            "voie(s) retenue(s)"
                        } else {
                            "lane(s) selected"
                        }
                    ));
                    ui.end_row();
                }
            });
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
        let éclat = flash_fraction(self.values_changed_at);
        if éclat > 0.0 && !changed.is_empty() {
            ui.ctx().request_repaint();
        }

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
                // Les seize derniers n'existent qu'avec AVX-512 : on
                // ne les montre que s'ils portent quelque chose. Seize
                // lignes de zéros de plus pour un programme qui n'en
                // emploie aucun coûteraient la moitié de la fenêtre.
                let hauts_employés = XMM_REGISTERS[16..]
                    .iter()
                    .any(|(_, reg)| regs.get_zmm(*reg) != [0; 4]);
                let montrés = if hauts_employés { 32 } else { 16 };
                for (index, (name, reg)) in XMM_REGISTERS.into_iter().take(montrés).enumerate() {
                    let value = regs.get_ymm(reg);
                    let base = ui.visuals().text_color();
                    let flash = if changed.contains(&name) { éclat } else { 0.0 };
                    let color = mélange(base, theme.changed, flash);
                    let low = value[0] as u64;
                    let shown = vector_name(name, value);
                    ui.colored_label(color, egui::RichText::new(shown).monospace());
                    ui.colored_label(
                        color,
                        egui::RichText::new(format!("{low:016x}")).monospace(),
                    )
                    .on_hover_text(zmm_tooltip(
                        s,
                        regs.get_zmm(reg),
                        self.lang == Lang::Fr,
                    ));
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
    pub(crate) fn last_step_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let (Some(pas), Some(emulator)) = (&self.last_step, &self.emulator) else {
            return;
        };
        let (addr, text) = (pas.addr, pas.texte.as_str());
        let Some(before) = &self.regs_before else {
            return;
        };
        let now = &emulator.cpu().regs;
        // AU NIVEAU DÉBUTANT, CE QUI A CHANGÉ SE DIT EN TOUTES LETTRES.
        //
        // C'est ici qu'on apprend — « lire, prévoir, avancer,
        // CONSTATER » — et la ligne ne constatait rien de lisible à
        // qui débute : « RCX 0x3 -> 0x2 » demande de savoir lire
        // l'hexadécimal, et « ZF 0 -> 1 » de savoir ce qu'est ZF. Deux
        // choses qu'on vient précisément apprendre.
        //
        // Rien n'est ajouté au-dessus de ce niveau : l'hexadécimal est
        // la langue du métier, et une phrase par indicateur à chaque
        // pas serait du bruit devant quelqu'un qui n'a rien demandé.
        let débutant = self.niveau() == namsdec_re::Niveau::Débutant;
        let lang = self.lang;
        // RIP est écarté : il change à chaque instruction, le dire à
        // chaque pas noierait ce qui compte.
        let mut changes: Vec<String> = REGISTERS
            .iter()
            .filter(|&&(_, reg)| before.get64(reg) != now.get64(reg))
            .map(|&(name, reg)| {
                format!(
                    "{name} {} -> {}",
                    nombre_lisible(before.get64(reg), débutant),
                    nombre_lisible(now.get64(reg), débutant)
                )
            })
            .collect();
        // Les indicateurs, un par un : après un `cmp`, ce sont eux le
        // résultat — l'instruction ne touche aucun registre.
        if let Some(flags_before) = self.flags_before {
            let flags_now = emulator.cpu().flags;
            for (name, bit) in FLAG_BITS {
                if flags_before.contains(bit) != flags_now.contains(bit) {
                    let posé = flags_now.contains(bit);
                    let mut dit = format!("{name} {} -> {}", u8::from(!posé), u8::from(posé));
                    // LE RÔLE, ET SEULEMENT QUAND IL EST POSÉ. Un
                    // indicateur qui retombe ne dit pas l'inverse de ce
                    // qu'il disait — « le résultat n'est pas nul » est
                    // vrai mais sans intérêt —, et sept phrases à
                    // chaque `cmp` noieraient celle qui compte.
                    if débutant && posé {
                        let (_, rôle) = flag_words(name, lang);
                        if !rôle.is_empty() {
                            dit.push_str(" — ");
                            dit.push_str(rôle);
                        }
                    }
                    changes.push(dit);
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
        // OÙ L'ON EN EST DANS LE TEMPS DU PROGRAMME.
        //
        // La question que rien ne traitait au mode simple. Le mode
        // avancé a la chronologie et sa barre (`timeline.rs`) ; ici,
        // on avançait pas à pas sans jamais savoir combien de pas on
        // avait faits, ni combien de fois la boucle avait tourné —
        // alors que c'est la question même de plusieurs étapes du
        // parcours (« combien de tours, et que reste-t-il »).
        //
        // DEUX NOMBRES, ET AUCUN CALCUL : le compteur de pas de
        // l'émulateur, et le nombre de passages qu'il tient par
        // adresse — celui-là même qui peint la bande de chaleur dans
        // la gouttière. Le second ne paraît qu'à partir du deuxième
        // passage : devant du code droit, il dirait « passage 1 » à
        // chaque ligne, et une information qui ne varie jamais n'en
        // est pas une.
        //
        // LE PASSAGE EST CELUI DU PAS QU'ON VIENT DE FAIRE, à son
        // adresse à lui : le compte est alors exact, là où celui de
        // l'instruction À VENIR demanderait d'ajouter un.
        if débutant {
            let faits = emulator.executed();
            let passages = emulator.visits().get(&addr).copied().unwrap_or(0);
            let mut texte = s.last_step_count.replacen("{}", &faits.to_string(), 1);
            if passages > 1 {
                texte.push_str(&s.last_step_pass.replacen("{}", &passages.to_string(), 1));
            }
            ui.colored_label(theme.dim, egui::RichText::new(texte).size(12.0));
        }
        // CE QUE ÇA DISAIT, et seulement si on avait eu à le prévoir.
        //
        // C'est la troisième étape du chemin — lire, prévoir, avancer,
        // CONSTATER : l'annonce que le bandeau avait retenue se relit
        // ici, juste au-dessus de ce qui a vraiment changé. Les deux
        // côte à côte sont la seule façon de savoir si l'on avait
        // compris.
        //
        // Hors de ce mode, rien n'est ajouté : le panneau garde
        // exactement ce qu'il montrait, parce qu'une ligne de plus à
        // chaque pas devant quelqu'un qui n'a rien demandé est du
        // bruit.
        if self.prevoir
            && self.niveau() == namsdec_re::Niveau::Débutant
            && let Some(annonce) = &pas.annonce
        {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, format!("{} :", s.foresee_said));
                ui.colored_label(theme.good, egui::RichText::new(annonce).monospace());
            });
        }
        if changes.is_empty() {
            // Une instruction peut ne rien changer de visible — un saut,
            // un `nop`. Le dire vaut mieux que de laisser croire qu'on a
            // oublié de regarder.
            ui.colored_label(theme.dim, s.last_step_nothing);
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for change in changes {
                // CHASSE FIXE POUR UNE VALEUR, CHASSE ORDINAIRE POUR
                // UNE PHRASE. « ZF 0 -> 1 — le dernier résultat est
                // nul » en chasse fixe se lit comme du code, et c'est
                // justement ce qu'on cherchait à ne plus montrer. Le
                // tiret cadratin est le repère : il ne paraît que
                // lorsqu'une explication suit.
                let texte = egui::RichText::new(&change);
                let texte = if change.contains(" — ") {
                    texte
                } else {
                    texte.monospace()
                };
                ui.colored_label(theme.changed, texte);
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
                text.push_str(&format!(" -> {extrait}"));
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
        let Some(advice) = self.error_advice.clone() else {
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

    /// Les octets de l'instruction, colorés par rôle, chacun expliqué au
    /// survol.
    ///
    /// `48 89 d8` cesse d'être un bloc : un préfixe qui dit « 64 bits »,
    /// un opcode qui dit « mov », un octet d'adressage qui dit « de RBX
    /// vers RAX ».
    fn encoding_ui(&self, ui: &mut egui::Ui, instr: &namsdec_disasm::Decoded, window: &[u8]) {
        if namsdec_disasm::fields_of(instr, window).is_empty() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            self.encoding_items_ui(ui, instr, window);
        });
    }

    /// Les octets eux-mêmes, SANS leur rangée.
    ///
    /// À part depuis le 16 novembre 2026, parce qu'ils se peignent
    /// désormais à deux endroits : sur leur propre ligne dans le
    /// panneau latéral, qui est étroit, et au milieu d'une rangée dans
    /// la bande de l'Établi, qui fait toute la largeur de la fenêtre.
    /// Une fonction qui ouvre son propre layout ne peut pas être posée
    /// dans celui d'un autre.
    fn encoding_items_ui(&self, ui: &mut egui::Ui, instr: &namsdec_disasm::Decoded, window: &[u8]) {
        let s = strings(self.lang);
        let theme = self.theme;
        for (range, field) in namsdec_disasm::fields_of(instr, window) {
            let (couleur, explication) = match field {
                namsdec_disasm::Field::Prefix(prefix) => (theme.warn, prefix_hint(s, prefix)),
                namsdec_disasm::Field::OpCode => (theme.accent, s.field_opcode),
                namsdec_disasm::Field::Addressing => (theme.good, s.field_addressing),
                namsdec_disasm::Field::Immediate => (theme.changed, s.field_immediate),
            };
            let texte = window[range]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            ui.colored_label(couleur, egui::RichText::new(texte).monospace())
                .on_hover_text(explication);
        }
    }

    /// Le titre de la fiche de cette instruction, cliquable.
    ///
    /// Rend le mnémonique quand on a cliqué : la fiche s'ouvre une fois
    /// les emprunts du bandeau relâchés.
    fn fiche_item_ui(&self, ui: &mut egui::Ui, text: &str) -> Option<String> {
        let s = strings(self.lang);
        let theme = self.theme;
        let mnemonic = text.split([' ', '\t']).next().unwrap_or_default();
        let entry = namsdec_doc::lookup(mnemonic)?;
        // Et le titre mène à la fiche : on s'arrête sur une instruction
        // parce qu'on ne la comprend pas, et le curseur de l'éditeur,
        // lui, est resté où on l'avait laissé.
        let étiquette = egui::Label::new(
            egui::RichText::new(entry.title.get(self.lang.code())).color(theme.dim),
        );
        let titre = crate::cliquable(ui, étiquette).on_hover_text(s.help_open_sheet);
        titre.clicked().then(|| mnemonic.to_owned())
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
        let link = crate::cliquable(
            ui,
            egui::Label::new(egui::RichText::new(s.vectors_open).color(theme.dim)),
        );
        if link.on_hover_text(s.vectors_hint).clicked() {
            self.montrer_vue(VueCentrale::Vectors);
        }
        // De l'air sous le dernier élément de la zone.
        //
        // Sans lui, ce lien touche le bord du panneau, c'est-à-dire la
        // ligne qu'on tire pour partager le bandeau et l'onglet. Le
        // pointeur y disait « main » au lieu de « ça se tire » — mesuré :
        // `PointingHand` sur la poignée de l'onglet, `ResizeVertical`
        // sur celle de la console, qui n'a rien sous elle. Une poignée
        // qui ment est pire qu'une poignée muette.
        //
        // DEUX pixels et non quatre : à quatre, le bandeau coupait
        // « Détail » en deux à 1280x820 — un pixel sur quinze. Le compte
        // du panneau est serré à ce point-là, et l'invariant de la coupe
        // l'a dit tout de suite.
        ui.add_space(2.0);
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
    pub(crate) fn next_instruction_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let Some(emulator) = &self.emulator else {
            // La section est dépliée devant un source, et elle ne disait
            // rien tant qu'aucun programme n'était assemblé : un titre,
            // un filet, et deux cents pixels de blanc — entre deux
            // voisines qui, elles, annonçaient ce qu'elles montreraient.
            // Sa propre phrase, et non « Aucun binaire assemblé. » une
            // troisième fois : c'est ce que le panneau fera qui manque
            // ici, pas le constat, déjà dit deux fois plus bas.
            crate::empty_state(ui, theme, s.empty_next_instruction, None, None);
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
        // La même règle que le bloc « Dernier pas » : au niveau
        // débutant, la cible d'un saut porte le nom que la source lui
        // donne. Les deux blocs se lisent l'un sous l'autre dans la
        // bande du bas ; deux écritures de la même adresse à deux
        // lignes d'écart feraient croire à deux endroits.
        let text = self.instruction_lisible(&instr, namsdec_disasm::format_one(&instr));

        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                theme.accent,
                egui::RichText::new(format!("{rip:#x}")).monospace(),
            );
            ui.label(egui::RichText::new(&text).monospace().strong());
        });
        // PRÉVOIR AVANT D'AVANCER : au niveau débutant, et si on l'a
        // demandé, tout ce qui suit attend le pas.
        //
        // L'adresse et l'instruction viennent d'être peintes — c'est
        // l'énoncé, et il reste entier. Ce qui est retenu est la
        // RÉPONSE : le pseudo-code en clair, la fiche, les opérandes
        // résolus, et surtout le verdict d'un saut conditionnel, qui
        // est la réponse même. Voir `crate::prevision` pour le
        // pourquoi.
        if crate::prevision::réponse(self.prevoir, self.niveau(), rip, self.prevision_révélée)
            .est_retenue()
        {
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, s.foresee_question)
                    .on_hover_text(s.foresee_question_hint);
                // « Montrer » et non « Révéler » : on ne dévoile pas un
                // secret, on demande une aide — et la demander ne doit
                // pas coûter un aveu. Un clic, pas de confirmation, et
                // la réponse reste montrée pour cette instruction-là.
                // ENCADRÉ, ET PETIT : `small_button` pose un bouton PLAT,
                // qu'on ne voit pas tant qu'on ne passe pas dessus — et
                // un bouton qu'on ne voit pas, dans une rangée où l'on
                // attend une réponse, n'existe pas. Relevé par le banc
                // (`outside_the_menus_a_button_shows_itself_without_being_hovered`),
                // qui connaît les quatre écritures de bouton de
                // l'application.
                if crate::small_framed_button(ui, theme, s.foresee_reveal)
                    .on_hover_text(s.foresee_reveal_hint)
                    .clicked()
                {
                    self.prevision_révélée = Some(rip);
                }
            });
            // Le détail est refermé : rouvert tel quel au pas suivant,
            // il montrerait la réponse qu'on vient de retenir.
            //
            // `next_detail_addr` N'EST PAS POSÉE ICI, et ce n'est pas un
            // oubli : c'est elle qui permet au détail de s'ouvrir de
            // lui-même, une fois par instruction, devant un appel ou un
            // saut conditionnel. La poser maintenant ferait croire que
            // cela a déjà eu lieu, et « Montrer » ne révélerait alors
            // RIEN du cas le plus intéressant — le verdict d'un saut
            // vit précisément dans ce détail.
            self.next_detail_open = false;
            self.open_requested_sheet(None);
            ui.separator();
            return;
        }

        // ET LE RETOUR : « Cacher », quand c'est « Montrer » qui a
        // ouvert la réponse.
        //
        // Demandé à l'écran le 10 novembre 2026, capture à l'appui :
        // « mets un btn de cacher/afficher ». « Montrer » n'avait pas
        // de retour — la réponse donnée, elle restait jusqu'au pas
        // suivant, et l'on ne pouvait plus se reposer la question. Un
        // geste qui ne se défait pas se prend avec hésitation, et
        // l'hésitation est exactement ce que ce mode cherche à lever :
        // demander une aide ne doit pas coûter un aveu, ni un pas.
        //
        // SEULEMENT QUAND LA PRÉVISION EST EN JEU. Hors de ce réglage,
        // la réponse n'a jamais été cachée : un bouton « Cacher » y
        // proposerait de masquer ce que namsdec a toujours montré, ce
        // qui n'est pas une bascule mais une option de plus.
        if crate::prevision::réponse(self.prevoir, self.niveau(), rip, None).est_retenue()
            && self.prevision_révélée == Some(rip)
        {
            ui.horizontal_wrapped(|ui| {
                if crate::small_framed_button(ui, theme, s.foresee_hide)
                    .on_hover_text(s.foresee_hide_hint)
                    .clicked()
                {
                    self.prevision_révélée = None;
                }
            });
        }

        // L'instruction dite en clair, sous sa forme machine : c'est le
        // même moteur que le panneau de désassemblage, et c'est ici
        // qu'on en a le plus besoin — devant l'instruction qui va
        // s'exécuter.
        let clair = namsdec_disasm::pseudo::of(&instr, self.lang.code());
        if clair != text {
            ui.colored_label(theme.good, egui::RichText::new(clair).monospace());
        }

        // Deux lignes en tout jusqu'ici : où l'on est, et ce que
        // l'instruction dit en clair. Le reste — les octets découpés par
        // rôle, la fiche, les opérandes résolus, les arguments d'un
        // appel — est derrière un repli, qui s'ouvre de lui-même devant
        // ce qui mérite une explication.
        //
        // Les octets sont passés sous le repli, et c'est une place
        // gagnée qui se mesure : sur une fenêtre de 1280×820 avec la
        // console ouverte, le bandeau demandait plus que la place
        // disponible et la zone défilante coupait une ligne en deux.
        // Ils n'ont rien perdu de leur intérêt — ils découpent
        // l'instruction par champs, ce qu'aucun désassembleur ne montre
        // — mais on les regarde en s'arrêtant, pas à chaque pas.
        let mérite = matches!(
            instr.flow_control(),
            namsdec_disasm::FlowControl::Call
                | namsdec_disasm::FlowControl::IndirectCall
                | namsdec_disasm::FlowControl::ConditionalBranch
        ) || instr.mnemonic() == namsdec_disasm::Mnemonic::Syscall;
        if mérite && self.next_detail_addr != Some(rip) {
            self.next_detail_open = true;
        }
        // Retenu par instruction : sans cela, replier le détail devant
        // un `call` le rouvrirait à la frame suivante.
        self.next_detail_addr = Some(rip);
        let mut détail = self.next_detail_open;
        // LE DÉTAIL NE SE DÉPLIE PAS DANS UNE BANDE QUI NE PEUT PAS LE
        // PORTER.
        //
        // Dans l'Établi, cette section vit dans une bande bornée
        // (`LIGNE_MAX`) qui ne défile pas : ce qui dépasse se peint
        // PAR-DESSUS la zone du code. Mesuré à 800x520 le 16 novembre
        // 2026, l'adresse de la prochaine instruction tombait sur
        // « CE QUE J'AI ÉCRIT ».
        //
        // Les deux autres voies ont été essayées et rendues : une zone
        // défilante ne sait pas quelle hauteur demander dans un panneau
        // qui s'ajuste — elle montre la moitié de ce qu'elle a et le
        // bloc « Dernier pas » disparaît —, et borner le dessin au
        // rectangle du panneau ne borne rien, puisque le panneau se
        // donne la hauteur qu'il veut.
        //
        // L'ÉTAT MÉMORISÉ N'EST PAS TOUCHÉ : le repli se rouvre de
        // lui-même dès que la fenêtre a la place, et l'infobulle du
        // chevron dit pourquoi il refuse ici.
        //
        // LA MESURE EST CELLE DE LA FENÊTRE, et non `available_height`
        // : dans un panneau qui s'ajuste à son contenu, celle-ci rend
        // ce qui reste à l'instant où on la demande — c'est-à-dire
        // presque rien, puisque le contenu n'est pas encore posé. Le
        // premier jet repliait donc le détail sur un écran de 1600 de
        // haut comme sur un de 520, et la capture l'a montré tout de
        // suite.
        let à_l_étroit =
            détail && self.debutant && ui.ctx().viewport_rect().height() < DÉTAIL_MINIMUM;
        if à_l_étroit {
            détail = false;
        }
        // Le chevron dit déjà que la rangée se déplie ; la main et le
        // trait au survol le disent de la même façon que partout
        // ailleurs, ce qui évite d'avoir à connaître cette façon-là.
        let entête = ui
            .horizontal(|ui| {
                ui.colored_label(theme.dim, if détail { "⏷" } else { "⏵" });
                ui.colored_label(theme.dim, s.next_instruction_detail);
            })
            .response
            .interact(egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(if à_l_étroit {
                s.next_instruction_detail_cramped
            } else {
                s.next_instruction_detail_hint
            });
        souligne_au_survol(ui, &entête);
        // Et le clic ne ment pas : à l'étroit, il n'y a rien à ouvrir,
        // et le chevron l'a dit au survol.
        let ouvert = entête.clicked() && !à_l_étroit;
        if ouvert {
            détail = !détail;
            self.next_detail_open = détail;
        }
        let mut fiche = None;
        if !détail {
            self.open_requested_sheet(None);
            ui.separator();
            return;
        }

        // TROIS CHOSES COURTES, ET UNE SEULE LIGNE QUAND IL Y A LA
        // PLACE.
        //
        // Les octets, le titre de la fiche et les opérandes résolus se
        // peignaient l'un SOUS l'autre, chacun sur sa ligne. C'est la
        // bonne mise en page dans le panneau latéral du mode avancé,
        // qui fait trois cents pixels — et c'en est une mauvaise dans
        // la bande de l'Établi, qui fait toute la largeur de la
        // fenêtre : relevé à l'écran le 16 novembre 2026, une flèche
        // sur ce bloc, « pas moyen d'avoir tout sur une ligne ».
        // « b9 05 00 00 00 », « Copier une valeur » et
        // « ecx 0x00000000 » tenaient sur un quart de la largeur et
        // prenaient trois lignes d'une bande qui n'en a que quelques
        // unes — au point que son titre « Détail » était rogné par le
        // haut.
        //
        // LE SEUIL EST UNE LARGEUR, et non le niveau : c'est la place
        // qui décide, et le panneau du mode avancé peut être tiré large
        // lui aussi.
        let en_ligne = ui.available_width() >= 700.0;
        // Les opérandes résolus, relevés AVANT la rangée : ils en font
        // partie quand il y a la place, et gardent leur grille sinon.
        let lignes = namsdec_emu::explain::operands(cpu, &instr);
        let symbols = self
            .assemble_state
            .as_ref()
            .map(|state| state.output.symbols.as_slice())
            .unwrap_or_default();
        if en_ligne {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                self.encoding_items_ui(ui, &instr, window);
                if let Some(mnémonique) = self.fiche_item_ui(ui, &text) {
                    fiche = Some(mnémonique);
                }
                // CHAQUE OPÉRANDE EN UN SEUL MORCEAU : « ecx
                // 0x00000000 », et le nom de l'adresse quand elle en a
                // un. La grille sépare ces trois colonnes pour les
                // aligner d'une ligne à l'autre ; sur une rangée
                // unique, il n'y a rien à aligner, et trois étiquettes
                // par opérande se liraient comme six choses.
                for (rang, ligne) in lignes.iter().enumerate() {
                    // LES TROIS COULEURS DE LA GRILLE SONT GARDÉES.
                    //
                    // Premier jet : tout en une seule étiquette, d'une
                    // seule couleur — et « short 0000000000400021h
                    // 0x400021 boucle » se lisait comme un bloc, là où
                    // la grille distinguait l'opérande écrit, sa valeur
                    // et le nom qu'elle porte. La rangée perd les
                    // colonnes, elle n'a pas à perdre ce qu'elles
                    // disaient.
                    //
                    // Un blanc plus large entre deux opérandes qu'entre
                    // les morceaux d'un même : c'est ce blanc qui
                    // remplace la ligne de la grille.
                    if rang > 0 {
                        ui.add_space(6.0);
                    }
                    ui.colored_label(
                        theme.dim,
                        egui::RichText::new(ligne.text.clone()).monospace(),
                    );
                    ui.label(egui::RichText::new(ligne.value.clone()).monospace());
                    if let Some(nom) = ligne
                        .addr
                        .and_then(|addr| named_at(&self.notes, symbols, addr))
                    {
                        ui.colored_label(theme.accent, egui::RichText::new(nom).monospace());
                    }
                }
            });
        } else {
            // Les octets, découpés par rôle : un préfixe n'est pas un
            // opcode, et une instruction x86 se lit comme une suite de
            // champs. C'est ce qu'on apprend en lisant un manuel Intel,
            // et ce qu'aucun désassembleur ne montre.
            self.encoding_ui(ui, &instr, window);
            // La fiche de l'instruction, en une ligne. Le reste est sous
            // F1 — ici, c'est le rappel qui évite d'y aller.
            if let Some(mnémonique) = self.fiche_item_ui(ui, &text) {
                fiche = Some(mnémonique);
            }
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

        // Un appel : avec quoi ?
        //
        // La convention met les arguments dans six registres, et un
        // débutant qui lit `call strcpy@plt` ne sait pas lesquels. Quand
        // la fonction appelée est connue, ses paramètres portent leur
        // nom ; sinon on montre les registres, ce qui reste la bonne
        // réponse à « où sont les arguments ? ». C'est le `dumpargs` de
        // pwndbg, sous les yeux plutôt que sur commande.
        if instr.flow_control() == namsdec_disasm::FlowControl::Call {
            self.call_arguments_ui(ui, cpu, &instr);
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

        if lignes.is_empty() || en_ligne {
            // Rien à aligner — ou tout est déjà dit sur la rangée. La
            // fiche demandée s'ouvre quand même : elle ne dépend pas des
            // opérandes.
            ui.separator();
            self.open_requested_sheet(fiche);
            return;
        }
        // Les étiquettes du programme, pour nommer les adresses : le
        // désassembleur ne connaît que des nombres, et « tampon+2 » se
        // relit dans le source, pas « 0x401002 ».
        egui::Grid::new("next_instruction_grid")
            .num_columns(3)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                for line in lignes {
                    ui.colored_label(theme.dim, egui::RichText::new(line.text).monospace());
                    ui.label(egui::RichText::new(line.value).monospace());
                    let named = line
                        .addr
                        .and_then(|addr| named_at(&self.notes, symbols, addr));
                    ui.colored_label(
                        theme.accent,
                        egui::RichText::new(named.unwrap_or_default()).monospace(),
                    );
                    ui.end_row();
                }
            });
        ui.separator();
        self.open_requested_sheet(fiche);
    }

    /// Ouvre la fiche qu'un clic a demandée, une fois les emprunts du
    /// bandeau relâchés.
    pub(crate) fn open_requested_sheet(&mut self, mnemonic: Option<String>) {
        if let Some(mnemonic) = mnemonic {
            self.open_help(&mnemonic);
            self.show_help = true;
        }
    }

    /// Les arguments de l'appel sur le point d'être fait.
    ///
    /// Chaque valeur est suivie comme un pointeur : `0x402000` ne dit
    /// rien, `0x402000 -> "salut"` dit tout. Les paramètres portent leur
    /// nom quand la fonction appelée est de celles que namsdec connaît.
    fn call_arguments_ui(
        &self,
        ui: &mut egui::Ui,
        cpu: &namsdec_core::Cpu,
        instr: &namsdec_disasm::Decoded,
    ) {
        let s = strings(self.lang);
        let theme = self.theme;
        let symbols = self
            .assemble_state
            .as_ref()
            .map(|state| state.output.symbols.as_slice())
            .unwrap_or_default();
        // Le nom de la cible, quand l'appel en désigne une en clair : un
        // `call rax` ne dit pas où il va avant d'être exécuté.
        let cible = (instr.op0_kind() == namsdec_disasm::OpKind::NearBranch64)
            .then(|| named_at(&self.notes, symbols, instr.near_branch_target()))
            .flatten();
        let note = cible.as_deref().and_then(namsdec_re::libc::explain);
        let params = note.map(|note| note.params).unwrap_or_default();

        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.call_arguments);
            if let Some(nom) = &cible {
                // La signature, écrite comme une page de manuel :
                // `strcpy(dest, src)`.
                let signature = if params.is_empty() {
                    nom.clone()
                } else {
                    format!("{nom}({})", params.join(", "))
                };
                ui.colored_label(theme.mnemonic, egui::RichText::new(signature).monospace());
            }
        });
        // Autant de lignes que la fonction a de paramètres ; à défaut,
        // les quatre premiers registres — au-delà, on montrerait surtout
        // des restes du calcul précédent.
        let combien = if params.is_empty() {
            4
        } else {
            params.len().min(namsdec_emu::ARGUMENTS.len())
        };
        egui::Grid::new("call_arguments")
            .num_columns(3)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                for (index, (name, reg)) in namsdec_emu::ARGUMENTS.iter().enumerate().take(combien)
                {
                    let value = cpu.regs.get64(*reg);
                    ui.colored_label(
                        theme.register,
                        egui::RichText::new(match params.get(index) {
                            Some(param) if *param != "..." => format!("{param} ({name})"),
                            _ => (*name).to_owned(),
                        })
                        .monospace(),
                    );
                    ui.colored_label(
                        if value == 0 { theme.dim } else { theme.text },
                        egui::RichText::new(format!("{value:#x}")).monospace(),
                    );
                    // Ce que la valeur désigne : une chaîne, du code, un
                    // autre pointeur.
                    let mut suite = String::new();
                    match namsdec_emu::telescope::describe(&cpu.mem, value) {
                        namsdec_emu::telescope::What::Pointer => {
                            let links = namsdec_emu::telescope::chain(
                                &cpu.mem,
                                value,
                                namsdec_emu::telescope::LIMIT,
                            );
                            for link in &links {
                                suite.push_str(&format!("-> {:#x} ", link.value));
                            }
                            if let Some(fin) = chain_end(links.last()) {
                                suite.push_str(&fin);
                            }
                        }
                        autre => {
                            if let Some(fin) = chain_end(Some(&namsdec_emu::telescope::Link {
                                addr: value,
                                value,
                                what: autre,
                            })) {
                                suite.push_str(&fin);
                            }
                        }
                    }
                    ui.colored_label(theme.dim, egui::RichText::new(suite).monospace());
                    ui.end_row();
                }
            });
    }

    /// D'où vient la valeur d'un registre : le pas qui l'a écrite.
    ///
    /// Rendue en clair — « écrit il y a 12 pas, ligne 7 : add rax, rcx »
    /// — parce que c'est la phrase qu'on se dit en cherchant.
    pub(crate) fn last_write_line(&self, name: &str) -> Option<String> {
        let reg = register_by_name(name)?;
        let rang = self.emulator.as_ref()?.last_write_to(reg)?;
        self.step_description(rang)
    }

    /// La chaîne des écritures qui ont mené à ce registre, d'après
    /// l'historique.
    ///
    /// « D'où vient ce que RAX contient ? » est la question qu'on pose
    /// devant chaque comparaison d'un programme qu'on ne connaît pas, et
    /// une seule réponse ne suffit presque jamais : le pas qui a écrit
    /// RAX lisait RCX, et c'est RCX qu'il faut alors remonter. On le
    /// faisait à la main, un survol après l'autre.
    ///
    /// Elle s'appuie sur l'historique et non sur le code : celui-ci dit
    /// ce que le programme *pourrait* avoir fait, celui-là ce qu'il a
    /// fait. Devant plusieurs chemins qui mènent au même endroit, il n'y
    /// a que l'historique qui sache lequel a été pris.
    ///
    /// **Elle s'arrête à la mémoire, et le dit.** Une valeur qui vient
    /// d'un `[rbp-8]` ne se remonte pas par son registre de base : cela
    /// mènerait à la mise en place du cadre de pile, pas à la valeur.
    /// Mieux vaut s'arrêter en le disant que de conduire ailleurs avec
    /// assurance.
    pub(crate) fn provenance_chain(&self, name: &str, combien: usize) -> Vec<String> {
        let s = strings(self.lang);
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut cherché = name.to_owned();
        let mut borne = emulator.trace().len();
        for _ in 0..combien {
            let Some(reg) = register_by_name(&cherché) else {
                break;
            };
            let Some(rang) = emulator.last_write_to_before(reg, borne) else {
                break;
            };
            let Some(texte) = self.step_description(rang) else {
                break;
            };
            // Le registre expliqué en tête de ligne : sans lui, quatre
            // lignes de provenance ne disent plus laquelle explique
            // quoi.
            out.push(format!("{} — {texte}", cherché.to_uppercase()));

            // Ce que ce pas lisait, pour continuer.
            let suivant = emulator
                .trace()
                .get(rang)
                .and_then(|pas| {
                    state
                        .disasm
                        .iter()
                        .find(|instruction| instruction.address == pas.addr)
                })
                .and_then(|instruction| {
                    namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
                });
            let Some(décodée) = suivant else { break };
            if namsdec_re::provenance::reads_memory(&décodée) {
                out.push(s.provenance_from_memory.to_owned());
                break;
            }
            let (lus, _) = namsdec_re::provenance::registers(&décodée);
            // Un registre **autre** que celui qu'on explique, quand il y
            // en a un : dans `add rax, rcx`, RAX est l'accumulateur et
            // RCX l'ingrédient nouveau. C'est l'ingrédient qu'on veut
            // remonter — suivre l'accumulateur ne fait que redescendre
            // la même somme d'un cran. À défaut, on suit ce qu'il y a.
            let cible = cherché.to_ascii_uppercase();
            let source = lus
                .iter()
                .map(|r| format!("{r:?}").to_ascii_uppercase())
                .find(|nom| *nom != cible)
                .or_else(|| lus.first().map(|r| format!("{r:?}").to_ascii_uppercase()));
            let Some(nom) = source else { break };
            cherché = nom;
            borne = rang;
        }
        out
    }

    /// D'où vient cette valeur, **en lisant le code** et non
    /// l'historique.
    ///
    /// L'historique est la meilleure réponse quand il existe : il dit ce
    /// qui s'est passé. Mais devant un binaire qu'on ne peut pas
    /// exécuter — il appelle une bibliothèque qu'on n'a pas —, il est
    /// vide, et la seule voie est de remonter le code à l'envers. C'est
    /// ce qu'on fait à la main devant un listing ; autant le faire ici.
    pub(crate) fn static_provenance(&self, name: &str) -> Vec<namsdec_re::provenance::Step> {
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            return Vec::new();
        };
        namsdec_re::provenance::trace(&state.disasm, emulator.cpu().regs.rip, name)
    }

    /// D'où vient ce qui est en mémoire à cette adresse.
    pub(crate) fn last_memory_write_line(&self, addr: u64) -> Option<String> {
        let rang = self.emulator.as_ref()?.last_write_to_memory(addr)?;
        self.step_description(rang)
    }

    /// Le pas numéro `rang`, dit en clair : de combien on remonte, à
    /// quelle ligne, et quelle instruction.
    fn step_description(&self, rang: usize) -> Option<String> {
        let s = strings(self.lang);
        let emulator = self.emulator.as_ref()?;
        let pas = emulator.trace().get(rang)?;
        let recul = emulator.trace().len() - rang;
        let state = self.assemble_state.as_ref();
        let ligne = state.and_then(|state| state.addr_to_line.get(&pas.addr).copied());
        // Reformatée comme on l'écrit — `add rax, rbx` et non
        // `add rax,rbx` : c'est une phrase, pas une colonne de listing.
        let texte = state
            .and_then(|state| {
                state
                    .disasm
                    .iter()
                    .find(|instruction| instruction.address == pas.addr)
            })
            .and_then(|instruction| {
                namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
                    .map(|decoded| namsdec_disasm::format_one(&decoded))
            })
            .unwrap_or_default();
        Some(match ligne {
            Some(ligne) => format!(
                "{} -{recul}, {} {ligne} : {texte}",
                s.written_at, s.line_word
            ),
            None => format!("{} -{recul} : {texte}", s.written_at),
        })
    }

    pub(crate) fn registers_ui(&mut self, ui: &mut egui::Ui) {
        if self.emulator.is_none() {
            // Le seul des six panneaux vides à porter le bouton : la
            // vue du processeur est celle qu'on trouve en ouvrant, et
            // c'est elle qui répond à « et maintenant ? ». Le répéter
            // dans chaque onglet ferait deux boutons identiques à
            // l'écran dès qu'on en change.
            let s = strings(self.lang);
            let theme = self.theme;
            let peut = self.can_assemble();
            let mut lancer = false;
            ui.add_enabled_ui(peut, |ui| {
                lancer = empty_state(
                    ui,
                    theme,
                    s.no_binary_assembled,
                    Some(s.empty_assemble_hint),
                    Some(s.action_assemble),
                );
            });
            if lancer {
                self.dispatch(Action::Assemble);
            }
            return;
        }
        let Some(emulator) = &self.emulator else {
            return;
        };
        let theme = self.theme;
        // Une copie : la grille écrit dans `self` — le champ d'édition
        // d'un registre — et ne peut donc pas garder l'émulateur
        // emprunté pendant ce temps.
        let regs = emulator.cpu().regs.clone();
        let regs = &regs;
        let changed = changed_registers(self.regs_before.as_ref(), regs);
        // Le battement d'orange : une valeur qui change s'éclaire, puis
        // retombe. Tant qu'il brille, on redemande une frame — sans quoi
        // il resterait figé à la teinte de la dernière.
        let éclat = flash_fraction(self.values_changed_at);
        if éclat > 0.0 && !changed.is_empty() {
            ui.ctx().request_repaint();
        }
        // ON MONTRE CE QUE LE PROGRAMME TOUCHE.
        //
        // Relevé sur les trois captures de l'Établi du 16 novembre
        // 2026 : R8 à R15 y sont à zéro, alignés sur huit rangées,
        // c'est-à-dire LA MOITIÉ du volet — pendant que la pile, en
        // dessous, montre trois lignes et se défile de côté. Vingt
        // étapes du parcours sur cent dix-sept les emploient ; les
        // quatre-vingt-dix-sept autres paient huit lignes pour huit
        // zéros.
        //
        // LA RÈGLE SE DIT EN UNE PHRASE, et c'est ce qui la rend
        // tenable : les huit paraissent dès que l'un d'eux porte
        // quelque chose. Aucune mémoire à tenir, rien à remettre à
        // zéro entre deux courses, et la grille ne se réorganise qu'au
        // moment où un registre entre en jeu.
        //
        // LES HUIT ENSEMBLE, et non celui-là seul : un bloc qui
        // apparaîtrait registre par registre ferait danser la grille à
        // chaque pas, et c'est elle qu'on regarde en avançant.
        //
        // AU NIVEAU DÉBUTANT SEULEMENT, et la ligne sous la grille les
        // rend à la demande : celui qui travaille veut ses seize
        // registres à la même place d'une séance à l'autre, même
        // vides.
        let débutant = self.debutant;
        let hauts_en_jeu = REGISTERS[8..].iter().any(|(_, reg)| regs.get64(*reg) != 0)
            || self
                .regs_before
                .as_ref()
                .is_some_and(|avant| REGISTERS[8..].iter().any(|(_, reg)| avant.get64(*reg) != 0));
        let hauts_montrés = !débutant || hauts_en_jeu || self.registres_hauts;
        let visibles: &[(&'static str, Reg64)] = if hauts_montrés {
            &REGISTERS
        } else {
            &REGISTERS[..8]
        };
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
        let ligne = ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y;
        let (pairs, min_gap) = register_layout(
            available,
            pair,
            spacing,
            ui.available_height(),
            ligne,
            visibles.len(),
        );
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
        // Ce que le rendu a demandé : ouvrir un champ, poser une valeur,
        // ou refermer. Appliqué après la grille, qui emprunte `self`.
        let mut éditer: Option<&'static str> = None;
        let mut poser: Option<(&'static str, u64)> = None;
        let mut annuler = false;
        // Le champ qui vient de s'ouvrir a-t-il reçu le focus ?
        let mut focaliser = false;
        // Sortie de `self` le temps du rendu : la grille a besoin de
        // l'écrire, et elle lit `self` par ailleurs.
        let mut edition = self.editing_register.take();
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
                for (index, (name, value)) in visibles
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
                    } else {
                        let base = ui.visuals().text_color();
                        let flash = if changed.contains(&name) { éclat } else { 0.0 };
                        mélange(base, theme.changed, flash)
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
                    let response = crate::cliquable(ui, egui::Label::new(étiquette.color(color)));
                    // Le rôle du registre, d'abord : le processeur ne le
                    // connaît pas — RBX et R12 lui sont identiques —, et
                    // rien dans le code ne le rappelle. Puis ce que le
                    // clic ferait.
                    let provenance = self.last_write_line(name);
                    let response = response.on_hover_ui(|ui| {
                        ui.strong(name);
                        // Ce qu'il contient, avant ce à quoi il sert :
                        // on survole un registre pour savoir combien ça
                        // fait, et l'hexadécimal seul oblige à convertir
                        // de tête pour voir qu'un compteur vaut 10 ou
                        // qu'un retour vaut -1.
                        ui.label(egui::RichText::new(format!("{value:#018x}")).monospace());
                        ui.colored_label(theme.accent, readings::short(lang, value));
                        ui.add_space(4.0);
                        ui.label(roles::of(lang, name));
                        // D'où vient la valeur : le pas qui l'a écrite,
                        // son rang et sa ligne. « Qui a mis ça là ? » est
                        // la question qu'on pose devant un registre qui
                        // ne contient pas ce qu'on croyait.
                        if provenance.is_some() {
                            ui.add_space(4.0);
                            // La chaîne, et non le seul dernier pas : le
                            // pas qui a écrit RAX lisait RCX, et c'est
                            // RCX qu'il faut alors remonter. On le
                            // faisait à la main, un survol après
                            // l'autre.
                            for ligne in self.provenance_chain(name, 4) {
                                ui.colored_label(theme.accent, ligne);
                            }
                        }
                        // À défaut d'historique — un binaire qu'on ne
                        // peut pas exécuter n'en a pas —, la lecture du
                        // code à l'envers : la dernière écriture, puis
                        // ce qui l'a alimentée.
                        if provenance.is_none() {
                            let chaîne = self.static_provenance(name);
                            if !chaîne.is_empty() {
                                ui.add_space(4.0);
                                ui.colored_label(theme.dim, s.provenance_static);
                                for étape in chaîne.iter().take(5) {
                                    // La flèche marque le maillon qui
                                    // remplit une case mémoire : sans
                                    // elle, deux lignes se suivent sans
                                    // qu'on voie que la valeur a
                                    // traversé la pile entre les deux.
                                    let franchi = étape.writes.starts_with('[');
                                    // Le point médian, et non une
                                    // flèche coudée : `↳` n'est dans
                                    // aucune des polices embarquées, et
                                    // se peindrait en carré vide. Le
                                    // contrôle des glyphes l'a dit tout
                                    // de suite.
                                    let flèche = if franchi { "· " } else { "  " };
                                    ui.colored_label(
                                        theme.accent,
                                        egui::RichText::new(format!(
                                            "{flèche}{:#010x}  {}",
                                            étape.addr, étape.text
                                        ))
                                        .monospace(),
                                    );
                                }
                            }
                        }
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
                    // La valeur se lit *et* se pose : un clic ouvre un
                    // champ, et ce qu'on y écrit remplace le contenu du
                    // registre. « Et si RCX valait 1 ? » se répond alors
                    // en une seconde, sans réécrire le programme.
                    match &mut edition {
                        Some((nom, saisie)) if *nom == name => {
                            let champ = ui.add(
                                egui::TextEdit::singleline(saisie)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(pair * 0.62),
                            );
                            // LE FOCUS SE DEMANDE UNE FOIS, PAS À
                            // CHAQUE IMAGE.
                            //
                            // Redemandé à chaque frame, il est REPRIS
                            // aussitôt après avoir été perdu — et
                            // `lost_focus()`, qui est la condition de
                            // validation, n'arrive jamais. On tapait
                            // une valeur, on appuyait sur Entrée, et
                            // rien ne se passait : « la modification
                            // des registres n'est pas effective »
                            // (relevé à l'écran le 16 novembre 2026).
                            if !self.editing_register_focused {
                                champ.request_focus();
                                focaliser = true;
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                annuler = true;
                            } else if champ.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                poser = parse_register_value(saisie, débutant).map(|v| (name, v));
                                annuler = true;
                            }
                        }
                        _ => {
                            let flash = if changed.contains(&name) { éclat } else { 0.0 };
                            // Un FOND orange qui s'estompe, et non plus la
                            // seule teinte du texte : un bloc de couleur se
                            // voit d'un coup d'œil, là où quelques pixels
                            // d'encre orange se rataient. Réservé AVANT le
                            // texte pour passer derrière lui.
                            let fond = (flash > 0.0).then(|| ui.painter().add(egui::Shape::Noop));
                            // Sur ce fond, l'encre reste claire pour rester
                            // lisible pendant l'éclat ; sans fond, sa teinte
                            // habituelle.
                            let encre = if flash > 0.0 {
                                ui.visuals().text_color()
                            } else {
                                color
                            };
                            let job = valeur_job(ui, theme, value, encre, débutant);
                            let valeur =
                                crate::cliquable(ui, egui::Label::new(job)).on_hover_ui(|ui| {
                                    ui.label(readings::of(lang, value));
                                    ui.add_space(4.0);
                                    ui.weak(s.register_edit_hint);
                                });
                            if valeur.clicked() {
                                éditer = Some(name);
                            }
                            // Le fond est peint APRÈS, dans l'emplacement
                            // réservé plus haut : il passe donc derrière le
                            // texte, et la lecture du clic reste juste sous
                            // la réponse.
                            if let Some(fond) = fond {
                                let rect = valeur.rect.expand2(egui::vec2(3.0, 1.0));
                                ui.painter().set(
                                    fond,
                                    egui::Shape::rect_filled(
                                        rect,
                                        3.0,
                                        theme.changed.gamma_multiply(0.5 * flash),
                                    ),
                                );
                            }
                        }
                    }
                    in_row += 1;
                    if in_row == pairs {
                        ui.end_row();
                        in_row = 0;
                    } else if index != visibles.len() {
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

        // LA LIGNE QUI REND LES HUIT AUTRES.
        //
        // Un repli qui ne se défait pas est une amputation. Celui-ci se
        // défait d'un clic, DIT COMBIEN il cache, et son choix est un
        // réglage comme un autre : il vit dans `settings.json`, et la
        // séance suivante rouvre sur ce qu'on avait voulu voir.
        //
        // Elle ne paraît pas quand les huit sont déjà là d'eux-mêmes :
        // un repli qu'on ne peut plus demander n'aurait rien à offrir,
        // et la ligne dirait le contraire de ce que montre la grille.
        if débutant && !hauts_en_jeu {
            ui.add_space(2.0);
            let texte = if self.registres_hauts {
                s.registers_high_hide
            } else {
                s.registers_high_show
            };
            if crate::cliquable(
                ui,
                egui::Label::new(egui::RichText::new(texte).color(theme.dim).size(12.0)),
            )
            .on_hover_text(s.registers_high_hint)
            .clicked()
            {
                // L'enregistrement se fait tout seul : `settings.json`
                // est réécrit à la frame suivante par
                // `persist_settings_if_changed`, qui compare l'état
                // courant à celui du fichier.
                self.registres_hauts = !self.registres_hauts;
            }
        }

        if let Some(name) = clicked {
            if !self.watched_registers.remove(name) {
                self.watched_registers.insert(name);
            }
            self.sync_watchpoints();
        }
        self.editing_register = edition;
        if let Some(name) = éditer {
            let valeur = self
                .emulator
                .as_ref()
                .map(|e| register_value_by_name(e.cpu(), name))
                .unwrap_or(0);
            // LE CHAMP S'OUVRE DANS LA LANGUE DU NIVEAU.
            //
            // Il était toujours en hexadécimal, et sans préfixe : au
            // niveau débutant, où le volet écrit « 55 » en clair,
            // retaper « 55 » posait 0x55 — c'est-à-dire 85. Un champ
            // qui comprend autre chose que ce qu'il montre est pire
            // qu'un champ qui refuse.
            self.editing_register = Some((
                name,
                if self.debutant && valeur < 1_000_000 {
                    valeur.to_string()
                } else {
                    format!("{valeur:x}")
                },
            ));
            self.editing_register_focused = false;
        }
        if let Some((name, valeur)) = poser {
            self.set_register(name, valeur);
        }
        if focaliser {
            self.editing_register_focused = true;
        }
        if annuler {
            self.editing_register = None;
            self.editing_register_focused = false;
        }
    }

    /// Écrit des octets en mémoire, à la main.
    ///
    /// Sans regarder les droits : c'est ce que fait un débogueur, et
    /// c'est ce qui permet d'essayer une hypothèse sur du code ou sur
    /// une section en lecture seule. Le *programme*, lui, continue
    /// d'obéir aux règles — sans quoi la protection n'apprendrait plus
    /// rien.
    pub(crate) fn poke_memory(&mut self, addr: u64, bytes: &[u8]) {
        let s = strings(self.lang);
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        match emulator.cpu_mut().mem.poke(addr, bytes) {
            Ok(()) => {
                self.status = Status::Note(|s| s.poke_done);
                // Le vidage compare avec l'image d'avant : sans cela, ce
                // qu'on vient d'écrire ne ressortirait pas.
                self.mem_before = None;
            }
            Err(_) => self.status = Status::Error(s.address_out_of_range.to_owned()),
        }
    }

    /// Pose une valeur dans un registre, à la main.
    ///
    /// Le pas suivant partira de là : c'est une hypothèse qu'on teste,
    /// pas une triche. Revenir en arrière la défait comme le reste —
    /// l'annulation restaure les registres d'avant le pas.
    pub(crate) fn set_register(&mut self, name: &str, value: u64) {
        let Some(emulator) = &mut self.emulator else {
            return;
        };
        if name == "RIP" {
            emulator.cpu_mut().regs.rip = value;
        } else if let Some(reg) = register_by_name(name) {
            emulator.cpu_mut().regs.set64(reg, value);
        } else {
            return;
        }
        self.status = Status::Note(|s| s.register_set);
    }

    /// Les sept indicateurs, en pastilles, dans la barre de titre.
    ///
    /// Leur place : la barre de titre est toujours visible et ne défile
    /// jamais, là où le panneau d'état se replie et se fait pousser. Les
    /// y mettre, c'est les avoir sous les yeux à chaque pas, à côté du nom
    /// du fichier. Ce sont de simples témoins — on les lit, on ne les
    /// règle pas : ils disent ce que la dernière instruction a produit.
    /// Chacun clignote en orange quand il change, et se présente en entier
    /// au survol. (Les registres, eux, s'éditent à la main : voir
    /// `set_register`.)
    pub(crate) fn flags_ui(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        // Les sept s'affichent TOUJOURS, même avant d'assembler : au repos
        // ils valent tous zéro, grisés. Les cacher laissait une barre de
        // titre muette tant qu'aucun programme ne tournait — et « tout doit
        // s'afficher ». Ce sont de simples indicateurs : on les lit, on ne
        // les règle pas.
        let (f, vivant) = match &self.emulator {
            Some(emulator) => (emulator.cpu().flags, true),
            None => (RFlags::empty(), false),
        };
        let avant = self.flags_before;
        let éclat = flash_fraction(self.values_changed_at);
        let lang = self.lang;
        let mut encore = false;
        // `horizontal` se dimensionne à ses pastilles (et non à toute la
        // largeur restante) : dans la disposition droite-à-gauche de la
        // barre de titre, il se range donc au bout, à droite du nom. Mais
        // egui en inverse alors le contenu — mesuré : OF sortait en tête,
        // CF en queue. On parcourt FLAG_BITS À L'ENVERS pour que ce double
        // renversement redonne l'ordre du processeur, CF…OF, de gauche à
        // droite. L'espacement est aéré : les sept pastilles collées se
        // lisaient comme un bloc, on cherchait où finissait l'une et où
        // commençait l'autre.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            for &(name, bit) in FLAG_BITS.iter().rev() {
                let on = f.contains(bit);
                let changé = vivant && avant.is_some_and(|a| a.contains(bit) != on);
                let flash = if changé { éclat } else { 0.0 };
                encore |= flash > 0.0;
                // Le survol présente l'indicateur en entier : nom développé,
                // rôle, état posé ou retombé — « tous les états ».
                flag_chip(ui, theme, name, on, flash)
                    .on_hover_text(flag_description(name, on, lang));
            }
        });
        // Tant qu'un éclat n'est pas retombé, redessiner : sinon il
        // resterait figé à sa valeur de la dernière frame.
        if encore {
            ui.ctx().request_repaint();
        }
    }

    pub(crate) fn stack_ui(&self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            // Ce que l'onglet montrera : un panneau vide qui ne dit
            // pas à quoi il sert n'apprend rien à qui le découvre.
            empty_state(
                ui,
                self.theme,
                s.no_binary_assembled,
                Some(s.empty_stack),
                None,
            );
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
                // La pile défile aussi de côté : une ligne porte une
                // adresse, un mot de huit octets, ses repères et la
                // suite des pointeurs — c'est plus large qu'un panneau,
                // et le bord coupait les derniers chiffres d'une valeur
                // sans que rien ne le dise. Une valeur tronquée n'est
                // pas une valeur abrégée : c'est une valeur fausse.
                egui::ScrollArea::horizontal()
                    .id_salt("stack_words_scroll")
                    // Toute la largeur, mais seulement la hauteur de ses
                    // douze lignes : sans cela, la zone prenait le reste
                    // du panneau et repoussait hors de vue ce qui suit la
                    // pile — les adresses de retour, la chaîne des
                    // appels, la convention.
                    .auto_shrink([false, true])
                    .show(ui, |ui| self.stack_words_ui(ui, emulator.as_ref()));
                // LA LÉGENDE EST DEHORS, et c'est tout le sujet.
                //
                // Elle était la dernière ligne de `stack_words_ui`,
                // donc DANS la zone qui défile de côté — où rien ne se
                // replie. Une phrase de cent caractères y devenait une
                // ligne de cent caractères : elle allongeait la course
                // du défilement bien au-delà de ce que les mots de pile
                // demandent, et sur la capture du 10 novembre 2026 elle
                // était coupée au milieu d'un mot, lisible seulement en
                // poussant la barre. Une légende qu'il faut aller
                // chercher n'explique rien.
                //
                // Dehors, elle se replie sur la largeur du volet, comme
                // une phrase. Les valeurs, elles, gardent leur
                // défilement : une valeur coupée serait fausse, une
                // phrase repliée reste vraie.
                ui.add(egui::Label::new(
                    egui::RichText::new(s.stack_above_hint).weak(),
                ));
                ui.separator();
                self.return_addresses_ui(ui, emulator.as_ref());
                self.call_chain_ui(ui, emulator.as_ref());
                ui.separator();
                self.abi_ui(ui, emulator.as_ref());
            });
    }

    /// Toutes les adresses de retour que porte la pile, et pas seulement
    /// celles des douze mots affichés.
    ///
    /// C'est le `retaddr` de pwndbg. Après un débordement, la question
    /// est « laquelle a été écrasée » — et elle ne se pose que si l'on
    /// voit la liste entière.
    fn return_addresses_ui(&self, ui: &mut egui::Ui, emulator: &dyn Backend) {
        let s = strings(self.lang);
        let theme = self.theme;
        let symbols = self
            .assemble_state
            .as_ref()
            .map(|state| state.output.symbols.as_slice())
            .unwrap_or_default();
        let trouvées = return_addresses(emulator.cpu(), &self.notes, symbols, 32);
        if trouvées.is_empty() {
            return;
        }
        egui::CollapsingHeader::new(format!("{} ({})", s.stack_return_addresses, trouvées.len()))
            .id_salt("stack_retaddr")
            .show(ui, |ui| {
                ui.colored_label(theme.dim, s.stack_return_addresses_hint);
                egui::Grid::new("stack_retaddr_grid")
                    .num_columns(3)
                    .min_col_width(0.0)
                    .striped(true)
                    .show(ui, |ui| {
                        for (addr, value, nom) in &trouvées {
                            ui.colored_label(
                                theme.dim,
                                egui::RichText::new(format!("{addr:#010x}")).monospace(),
                            );
                            ui.colored_label(
                                theme.accent,
                                egui::RichText::new(format!("{value:#x}")).monospace(),
                            );
                            ui.colored_label(theme.text, egui::RichText::new(nom).monospace());
                            ui.end_row();
                        }
                    });
            });
        ui.separator();
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
        // Dans un panneau étroit, l'adresse absolue et le mot de huit
        // octets ne tiennent pas côte à côte, et c'est la valeur qui
        // perdait ses derniers chiffres — une valeur tronquée n'est pas
        // une valeur abrégée, c'est une valeur fausse. On passe alors à
        // l'écart depuis RSP, comme le fait pwndbg : plus court, et il
        // dit quelque chose de plus — à quelle profondeur on regarde.
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let largeur_de = |texte: &str| {
            ui.painter()
                .layout_no_wrap(texte.to_owned(), font.clone(), egui::Color32::WHITE)
                .rect
                .width()
        };
        let besoin = largeur_de("0x0040fef0")
            + largeur_de("0000000000000000")
            + 3.0 * ui.spacing().item_spacing.x;
        let compact = ui.available_width() < besoin;
        egui::Grid::new("stack_words")
            .num_columns(4)
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
                    let repère = if compact {
                        // L'écart depuis RSP, en octets : `+0x10` est le
                        // deuxième mot sous le sommet, `-0x08` celui
                        // qu'on vient de dépiler.
                        let écart = addr as i64 - rsp as i64;
                        let signe = if écart < 0 { '-' } else { '+' };
                        format!("{signe}{:#04x}", écart.unsigned_abs())
                    } else {
                        format!("{addr:#010x}")
                    };
                    ui.colored_label(colour, egui::RichText::new(repère).monospace())
                        .on_hover_text(format!("{addr:#018x}"));
                    // LA PILE SUIT LA MÊME RÈGLE QUE LES REGISTRES :
                    // ce qu'on y pousse est le plus souvent un petit
                    // nombre — un compteur sauvé, un argument —, et le
                    // lire en hexadécimal au niveau débutant obligeait
                    // à convertir ce que la grille d'à côté écrit déjà
                    // en clair.
                    ui.label(valeur_job(ui, theme, value, colour, self.debutant))
                        .on_hover_text(readings::of(lang, value));
                    // Le repère : d'abord ce que l'adresse *est* pour le
                    // processeur, puis ce que la valeur veut dire.
                    let mut note = String::new();
                    if addr == rsp {
                        note.push_str("<- RSP");
                    }
                    if addr == rbp && rbp != 0 {
                        if !note.is_empty() {
                            note.push_str(", ");
                        }
                        note.push_str("<- RBP");
                    }
                    if code.iter().any(|range| range.contains(&value)) {
                        if !note.is_empty() {
                            note.push_str(" · ");
                        }
                        note.push_str(s.stack_return_address);
                        if let Some(label) = named_at(&self.notes, symbols, value) {
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
                    // Et ce que la valeur désigne, suivi jusqu'au bout :
                    // un mot de pile n'est presque jamais un nombre, et
                    // savoir *quoi* demande sinon d'aller lire l'adresse
                    // à la main, puis la suivante.
                    let links = namsdec_emu::telescope::chain(
                        &emulator.cpu().mem,
                        addr,
                        namsdec_emu::telescope::LIMIT,
                    );
                    let mut suite = String::new();
                    for link in links.iter().skip(1) {
                        suite.push_str(&format!("-> {:#x} ", link.value));
                    }
                    if let Some(fin) = chain_end(links.last()) {
                        suite.push_str(&fin);
                    }
                    ui.colored_label(theme.dim, egui::RichText::new(suite).monospace());
                    ui.end_row();
                }
            });
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
            // Ce que l'onglet montrera : un panneau vide qui ne dit
            // pas à quoi il sert n'apprend rien à qui le découvre.
            empty_state(
                ui,
                self.theme,
                s.no_binary_assembled,
                Some(s.empty_heap),
                None,
            );
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
            // Ce que l'onglet montrera : un panneau vide qui ne dit
            // pas à quoi il sert n'apprend rien à qui le découvre.
            empty_state(
                ui,
                self.theme,
                s.no_binary_assembled,
                Some(s.empty_bytes),
                None,
            );
            return;
        };
        let rip = self.emulator.as_ref().map(|e| e.cpu().regs.rip);

        let total = state.octets_du_code();
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
        // `show_rows` plutôt que tout dessiner : un binaire ouvert fait
        // quinze mille instructions, et construire quinze mille rangées
        // pour en montrer trente coûtait une frame entière — près de
        // cent millisecondes. Toutes les rangées ayant la même hauteur,
        // egui sait lesquelles sont à l'écran et ne demande que
        // celles-là.
        // DEUX HAUTEURS, et les confondre décalait le suivi.
        //
        // `show_rows` veut la hauteur d'une ligne SANS l'espace qui la
        // sépare de la suivante : il l'ajoute lui-même. En lui donnant
        // les deux, on lui faisait croire à des rangées de vingt et un
        // pixels là où il en posait vingt-sept — la vue réservait donc
        // un quart de hauteur de trop, et le décalage calculé pour
        // centrer l'instruction courante tombait quelque chose comme
        // soixante-dix lignes trop haut. La fenêtre de décompilation
        // porte la même remarque, pour la même raison.
        //
        // Le pas complet, lui, sert à placer : c'est de ligne à ligne
        // qu'on compte pour centrer.
        let ligne_seule = ui.text_style_height(&egui::TextStyle::Monospace);
        let row_height = ligne_seule + ui.spacing().item_spacing.y;
        // La ligne à suivre, calculée avant : `show_rows` ne dessine pas
        // les autres, et `scroll_to_me` ne peut donc pas les atteindre.
        let suivie = (self.follow_rip)
            .then(|| {
                rip.and_then(|rip| {
                    state
                        .disasm
                        .iter()
                        .position(|instruction| instruction.address == rip)
                })
            })
            .flatten();
        let mut zone = egui::ScrollArea::vertical()
            .id_salt("bytes_tab")
            .auto_shrink([false, false]);
        let voulu = suivie.map(|index| {
            // Centrée : on veut voir ce qui précède et ce qui suit
            // l'instruction courante, pas seulement elle.
            let milieu = (index as f32 + 0.5) * row_height;
            // Et bornée à ce qu'il y a vraiment à faire défiler :
            // egui ne ramène pas dans ses bornes une position posée
            // à la main, si bien qu'un court listing descendait
            // quand même et cachait ses premières lignes sous le
            // bord, place libre en dessous. Même défaut que la vue
            // du désassemblage, même remède.
            let total = state.disasm.len() as f32 * row_height;
            let plafond = (total - ui.available_height()).max(0.0);
            (milieu - ui.available_height() / 2.0).max(0.0).min(plafond)
        });
        if let Some(voulu) = voulu {
            zone = zone.vertical_scroll_offset(voulu);
        }
        // Ce qu'on peut récrire : les octets d'un BINAIRE ouvert. Un
        // listing venu d'un source se réassemble — le récrire à la main
        // serait effacé au prochain Ctrl+Maj+B, et l'on ne propose pas
        // un geste qui ne tient pas.
        let éditable = state.from_binary.is_some();
        let changés: std::collections::BTreeSet<u64> =
            self.patches.iter().map(|patch| patch.addr).collect();
        let mut ouvrir = None;
        let mut valider = None;
        let mut annuler = false;
        let rendu_zone = zone.show_rows(ui, ligne_seule, state.disasm.len(), |ui, rows| {
            // Un panneau egui adopte la largeur de son contenu, et dix
            // octets en hexadécimal suivis d'un texte d'instruction en
            // font beaucoup : sans cette borne, l'onglet poussait le
            // panneau — et l'éditeur avec.
            let largeur = ui.available_width();
            ui.set_max_width(largeur);
            for instruction in &state.disasm[rows] {
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
                // Deux étiquettes seulement, et la seconde se tronque :
                // dans une rangée horizontale, chaque étiquette réclame
                // sa largeur entière, et quatre colonnes côte à côte
                // poussaient le panneau. Le préfixe — ligne, adresse,
                // longueur — a une largeur fixe ; tout le reste tient
                // dans ce qui subsiste.
                ui.horizontal(|ui| {
                    ui.set_max_width(largeur);
                    ui.colored_label(
                        if here { theme.accent } else { theme.dim },
                        egui::RichText::new(format!("{ligne} {:08x}", instruction.address))
                            .monospace(),
                    );
                    ui.colored_label(
                        theme.register,
                        egui::RichText::new(format!("{:2}", instruction.bytes.len())).monospace(),
                    )
                    .on_hover_text(s.bytes_length_hint);
                    if self.bytes_edit == Some(instruction.address) {
                        // Le champ prend la place des octets, l'instruction
                        // reste lisible à côté : on récrit en voyant ce
                        // qu'on récrit.
                        let champ = ui.add(
                            egui::TextEdit::singleline(&mut self.bytes_edit_input)
                                .font(egui::TextStyle::Monospace)
                                .desired_width((instruction.bytes.len() * 3) as f32 * 9.0 + 16.0),
                        );
                        champ.request_focus();
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            valider = Some((instruction.address, self.bytes_edit_input.clone()));
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            annuler = true;
                        }
                        ui.colored_label(
                            theme.dim,
                            egui::RichText::new(&instruction.text).monospace(),
                        );
                    } else {
                        // Les octets changés portent la couleur qui dit
                        // « ceci n'est plus ce qu'il y avait » — la même
                        // que partout ailleurs dans namsdec.
                        let encre = if changés.contains(&instruction.address) {
                            theme.changed
                        } else {
                            colour
                        };
                        let étiquette = egui::Label::new(
                            egui::RichText::new(format!("{octets}  {}", instruction.text))
                                .monospace()
                                .color(encre),
                        )
                        .truncate();
                        if éditable {
                            if crate::cliquable(ui, étiquette)
                                .on_hover_text(s.bytes_edit_hint)
                                .clicked()
                            {
                                ouvrir = Some((instruction.address, octets.clone()));
                            }
                        } else {
                            ui.add(étiquette).on_hover_text(&instruction.text);
                        }
                    }
                });
            }
        });
        if let Some((adresse, hexa)) = ouvrir {
            self.bytes_edit = Some(adresse);
            self.bytes_edit_input = hexa;
        }
        if annuler {
            self.bytes_edit = None;
        }
        if let Some((adresse, saisie)) = valider {
            self.bytes_edit = None;
            self.write_bytes_at(adresse, &saisie);
        }
        // ON CESSE DE SUIVRE QUAND ON Y EST, et non dès la première
        // image : egui applique un décalage demandé à l'image
        // SUIVANTE. Lâcher le suivi tout de suite laissait la vue une
        // page trop haut dès que la hauteur de la vue changeait entre
        // deux images — ce qui arrive à chaque fois que le partage du
        // panneau se cale, c'est-à-dire juste après l'assemblage.
        //
        // La comparaison porte sur le décalage BORNÉ : au bas d'un
        // listing, le décalage voulu dépasse ce qu'egui peut donner, et
        // l'on suivrait alors pour toujours.
        let borné = voulu.map(|voulu| {
            let maximum = (rendu_zone.content_size.y - rendu_zone.inner_rect.height()).max(0.0);
            voulu.min(maximum)
        });
        if borné.is_none_or(|voulu| (rendu_zone.state.offset.y - voulu).abs() <= 1.0) {
            self.follow_rip = false;
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
                if small_framed_button(ui, theme, "×")
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
            ui.colored_label(theme.accent, ">");
            ui.monospace(position(rip));
        });

        let frames = emulator.frames();
        if frames.is_empty() {
            ui.colored_label(theme.dim, s.call_chain_empty);
            return;
        }
        for frame in frames.iter().rev() {
            ui.horizontal(|ui| {
                ui.colored_label(theme.dim, "\\_");
                ui.monospace(position(frame.call_site));
            });
        }
    }

    fn memory_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let Some(emulator) = &self.emulator else {
            // Ce que l'onglet montrera : un panneau vide qui ne dit
            // pas à quoi il sert n'apprend rien à qui le découvre.
            empty_state(
                ui,
                self.theme,
                s.no_binary_assembled,
                Some(s.empty_memory),
                None,
            );
            return;
        };
        // Le champ est posé *avant* la zone de défilement, et non dedans :
        // à l'intérieur, il se serait étiré à la largeur du vidage.
        let theme = self.theme;
        let regs = emulator.cpu().regs.clone();
        let typed = eval_address(&self.mem_addr_input, Some(&regs));
        let mut go = false;
        let mut watch = None;
        // Relevé ici, agi plus bas : la liste des résultats emprunte
        // `self` en écriture, et l'adresse regardée en fait partie.
        let mut aller_à: Option<u64> = None;
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

        // Chercher une suite d'octets. C'est le premier réflexe devant un
        // programme qui compare quelque chose : on vient de taper un mot
        // de passe, on veut savoir dans quel tampon il a atterri.
        let motif = memfind::parse(&self.mem_find_input);
        let mut chercher = false;
        ui.horizontal(|ui| {
            ui.colored_label(theme.dim, s.memory_find_label);
            let réponse = ui
                .add(
                    egui::TextEdit::singleline(&mut self.mem_find_input)
                        .id_salt("memory_find")
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width() - 28.0),
                )
                .on_hover_text(s.memory_find_hint);
            // Le refus se voit sur le champ, comme pour l'adresse — mais
            // pas sur un champ vide : on n'a encore rien demandé.
            if matches!(&motif, Err(e) if !matches!(e, memfind::PatternError::Empty)) {
                ui.painter().rect_stroke(
                    réponse.rect,
                    ui.visuals().widgets.inactive.corner_radius,
                    egui::Stroke::new(1.0, theme.bad),
                    egui::StrokeKind::Inside,
                );
            }
            chercher |= réponse.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.add_enabled_ui(motif.is_ok(), |ui| {
                chercher |= icon_square_button(ui, chrome::icon_search)
                    .on_hover_text(s.memory_find_hint)
                    .clicked();
            });
        });
        // Ce que le motif mesure, jokers compris : c'est la première
        // chose qu'on veut vérifier après l'avoir tapé. « 414243 » fait
        // trois octets, « ?? 41 » en fait deux — et l'on s'aperçoit
        // ainsi d'un chiffre oublié avant de chercher, non après.
        if let Ok(motif) = &motif {
            ui.colored_label(
                theme.dim,
                s.memory_find_bytes
                    .replacen("{}", &motif.len().to_string(), 1),
            );
        }
        match &motif {
            Err(memfind::PatternError::Empty) => {}
            Err(e) => {
                ui.colored_label(theme.bad, pattern_error_text(s, e));
            }
            Ok(motif) => {
                if chercher {
                    self.mem_find_hits = search_memory(&emulator.cpu().mem, motif);
                    self.mem_find_done = true;
                }
            }
        }
        if self.mem_find_done && motif.is_ok() {
            if self.mem_find_hits.is_empty() {
                ui.colored_label(theme.dim, s.memory_find_none);
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        theme.dim,
                        s.memory_find_count.replacen(
                            "{}",
                            &self.mem_find_hits.len().to_string(),
                            1,
                        ),
                    );
                    if self.mem_find_hits.len() == MEM_FIND_MAX {
                        // Dire qu'on s'est arrêté : une liste tronquée
                        // qui ne le dit pas fait conclure à tort qu'il
                        // n'y en a pas davantage.
                        ui.colored_label(theme.warn, s.memory_find_capped);
                    }
                });
                // Une seule rangée qui se replie : dix adresses les unes
                // sous les autres prendraient la moitié du panneau, et
                // l'on est venu voir des octets.
                ui.horizontal_wrapped(|ui| {
                    for addr in &self.mem_find_hits {
                        let étiquette = egui::Label::new(
                            egui::RichText::new(format!("{addr:#x}"))
                                .monospace()
                                .color(theme.accent),
                        );
                        let lien = cliquable(ui, étiquette).on_hover_text(s.memory_go_hint);
                        if lien.clicked() {
                            aller_à = Some(*addr);
                        }
                    }
                });
            }
        }

        if let Some(addr) = aller_à {
            self.mem_addr = addr;
            self.mem_addr_input = format!("{addr:#x}");
        }

        // Ce qu'on suit et comment on lit : deux réglages du même geste.
        // Une seule rangée enroulée les tenait, pour ne pas prendre trois
        // lignes. Mais sur un panneau étroit elle débordait, et
        // `horizontal_wrapped` traitait mal l'élément qui passait à la
        // ligne : « Lire en : » se retrouvait avec une galère de 257 px
        // (pour neuf caractères) peinte à l'abscisse de départ, PAR-DESSUS
        // « Suivre : » et les boutons de suivi. Mesuré sur un binaire
        // chargé, barre latérale à ~480 px : cinq textes l'un sur l'autre.
        // Deux rangées franches — « Suivre » puis « Lire en » — et chacune
        // s'enroule proprement dans son coin. Deux lignes au lieu d'une,
        // mais jamais trois, et plus rien ne se chevauche.
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(theme.dim, s.memory_follow);
            for kind in MemFollow::ALL {
                if ui
                    .selectable_label(self.memory_follow == kind, kind.label(s))
                    .clicked()
                {
                    self.memory_follow = kind;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            // Le même endroit dit quatre choses différentes selon ce
            // qu'on y cherche — un tampon, une pile, une chaîne, une
            // suite de pointeurs. Dans une liste déroulante et non en
            // quatre boutons : « Octets Mots Texte Pointeurs » ne tient
            // pas sur la largeur d'un panneau, et repassait à la ligne
            // par-dessus le reste.
            ui.colored_label(theme.dim, s.memory_format_label);
            let formats: Vec<(MemoryFormat, String)> = MemoryFormat::ALL
                .into_iter()
                .map(|f| (f, f.label(s).to_owned()))
                .collect();
            crate::liste_déroulante(
                ui,
                theme,
                egui::Id::new("memory_format"),
                0.0,
                &mut self.memory_format,
                &formats,
                "",
            );
        });
        if let Some(addr) = self.memory_follow.address(&regs) {
            self.mem_addr = addr;
            self.mem_addr_input = format!("{addr:#x}");
        }

        // Le vidage est calculé *avant* la liste : celle-ci peut retirer
        // une zone, donc emprunter `self` en écriture, et l'émulateur
        // n'est plus disponible à ce moment-là.
        // Ce que l'adresse regardée *est* : quelle zone, avec quels
        // droits. La protection devient ainsi une chose qu'on lit avant
        // de se faire refuser un accès, et non seulement une punition.
        let mem = &emulator.cpu().mem;
        // L'étiquette d'abord, quand il y en a une : on cherche
        // « message », pas « 0x402000 », et c'est sous ce nom-là qu'on
        // l'a écrit dans le source.
        let symbols = self
            .assemble_state
            .as_ref()
            .map(|state| state.output.symbols.as_slice())
            .unwrap_or_default();
        let quartier = match named_at(&self.notes, symbols, self.mem_addr) {
            Some(label) => format!("{label} · {}", region_line(s, mem, self.mem_addr)),
            None => region_line(s, mem, self.mem_addr),
        };
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
            self.lang,
        );
        let watched: Vec<(u64, u64)> = emulator
            .watchpoints()
            .iter()
            .map(|point| (point.addr, point.value()))
            .collect();
        // Relevé avant tout affichage : la suite écrit dans `self`, et
        // l'émulateur ne peut pas rester emprunté pendant ce temps.
        let zones = mem.regions();
        // De même pour la recherche : elle lit toute la mémoire, et se
        // fait donc pendant que l'émulateur est encore là. Le résultat
        // date d'une frame, comme le vidage lui-même.
        let cherché = self.mem_search_kind.needle(&self.mem_search);
        let trouvés: Vec<u64> = cherché
            .as_ref()
            .map(|needle| namsdec_emu::telescope::search(mem, needle, 24))
            .unwrap_or_default();

        // Ce qu'on regarde, d'abord : l'étiquette, la zone, ses droits,
        // et qui a écrit là. C'est la réponse à « où suis-je », et elle
        // ne doit pas se mériter.
        self.watchpoints_ui(ui, &watched);
        ui.colored_label(theme.dim, quartier);
        if let Some(provenance) = self.last_memory_write_line(self.mem_addr) {
            ui.colored_label(theme.accent, provenance);
        }

        // Puis les outils, sous un seul repli : la carte, la recherche
        // et l'écriture. Trois en-têtes dépliables poussaient le vidage
        // hors du panneau, et c'est lui qu'on est venu voir.
        //
        // ET ILS S'OUVRENT D'UN COUP, SANS FONDU. Signalé sur capture le
        // 14 octobre 2026 : « Écrire ici » barrait la première ligne
        // d'octets du vidage, comme s'il s'écrivait par-dessus.
        //
        // Mesuré image par image, c'est le fondu de l'ouverture : egui
        // rogne le corps à sa hauteur du moment, si bien qu'un en-tête
        // est coupé en deux pendant que le vidage, lui, a déjà pris sa
        // nouvelle place. Les deux se croisent pendant TROIS images —
        // l'encre d'« Écrire ici » s'arrêtant à 601,8 là où le texte va
        // jusqu'à 615,2, le vidage reprenant à 608.
        //
        // Trois images ne se voient pas à soixante hertz ; elles se
        // voyaient très bien quand une image coûtait trois cents
        // millisecondes sur un gros binaire, et c'est de là que vient la
        // capture. Le remède tient en deux lignes : l'animation est
        // posée d'avance sur sa valeur d'arrivée, donc le corps est
        // entier dès la première image où il paraît. Ce qui disparaît
        // est un fondu de deux dixièmes de seconde dans un panneau
        // dense, où il n'apprenait rien.
        let mut aller = None;
        let mut écrire = None;
        // LE VIDAGE GARDE SON PLANCHER, et les outils dépliés ne
        // l'évincent plus. Mesuré le 26 octobre 2026 sur un binaire lié
        // dynamiquement, les trois replis ouverts : à 1280 x 820, la
        // boîte à outils prenait tout, et le vidage — ce qu'on est venu
        // voir — n'était pas peint DU TOUT. Pas rogné, pas repoussé sous
        // une barre : absent. La hauteur de l'onglet est imposée à son
        // contenu (voir `sidebar_ui`), si bien que ce qui ne tient pas
        // est coupé et que la zone défilante du panneau n'a rien à faire
        // défiler.
        //
        // La borne est CALCULÉE sur la place de l'onglet, jamais sur ce
        // que les outils mesurent : rien ne se reboucle. Au large elle
        // est plus haute que leur contenu, la zone épouse ce contenu
        // (`auto_shrink` en hauteur) et aucune barre ne paraît — l'écran
        // large ne change pas d'allure.
        let place_outils = (ui.available_height() - PLANCHER_DU_VIDAGE).max(90.0);
        egui::ScrollArea::vertical()
            .id_salt("memory_tools_scroll")
            .auto_shrink([false, true])
            .max_height(place_outils)
            .show(ui, |ui| {
                repli(
                    ui,
                    theme,
                    ui.make_persistent_id("memory_tools"),
                    s.memory_tools,
                    |ui| {
                        // La carte, comme `vmmap` : où commence le code, où
                        // finit la pile, ce que le tas occupe. Un refus d'accès
                        // n'apprend rien à qui ne sait pas où il se trouvait.
                        if !zones.is_empty() {
                            repli(
                                ui,
                                theme,
                                ui.make_persistent_id("memory_map"),
                                &format!("{} ({})", s.memory_map, zones.len()),
                                |ui| {
                                    // La carte défile dans sa propre fenêtre, bornée
                                    // en hauteur. Sans ça, un binaire aux régions
                                    // nombreuses — les sections d'un programme lié
                                    // dynamiquement, .init, .text, .fini… — dépliait
                                    // une carte si haute que le contenu de l'onglet
                                    // dépassait la place que le panneau latéral lui
                                    // réserve. Le partage à trois (bandeau, état du
                                    // processeur, onglet) divergeait alors : l'onglet
                                    // remontait par-dessus les registres — la carte se
                                    // peignait sur RAX…RIP — ou filait sous le bas de
                                    // la fenêtre. Mesuré sur les deux faces du défaut.
                                    // Bornée, la carte ne peut plus pousser le reste.
                                    egui::ScrollArea::vertical()
                                        .id_salt("memory_map_scroll")
                                        .max_height(180.0)
                                        .show(ui, |ui| {
                                            egui::Grid::new("memory_map_grid")
                                                .num_columns(3)
                                                .min_col_width(0.0)
                                                .striped(true)
                                                .show(ui, |ui| {
                                                    for zone in &zones {
                                                        let dedans =
                                                            zone.range.contains(&self.mem_addr);
                                                        let ligne = ui
                                                            .colored_label(
                                                                if dedans {
                                                                    theme.accent
                                                                } else {
                                                                    theme.text
                                                                },
                                                                egui::RichText::new(format!(
                                                                    "{:#x}-{:#x}",
                                                                    zone.range.start,
                                                                    zone.range.end
                                                                ))
                                                                .monospace(),
                                                            )
                                                            .on_hover_text(s.memory_map_hint);
                                                        ui.colored_label(
                                                            theme.register,
                                                            egui::RichText::new(perms_text(
                                                                zone.perms,
                                                            ))
                                                            .monospace(),
                                                        );
                                                        ui.colored_label(theme.dim, zone.name);
                                                        let rangée = ligne
                                                            .interact(egui::Sense::click())
                                                            .on_hover_cursor(
                                                                egui::CursorIcon::PointingHand,
                                                            );
                                                        souligne_au_survol(ui, &rangée);
                                                        if rangée.clicked() {
                                                            aller = Some(zone.range.start);
                                                        }
                                                        ui.end_row();
                                                    }
                                                });
                                        });
                                },
                            );
                        }
                        // Chercher : c'est le `search` de pwndbg. On a une chaîne, une
                        // valeur, une adresse — la question est « où ? », et la réponse
                        // ne s'obtient pas en faisant défiler un vidage.
                        repli(
                            ui,
                            theme,
                            ui.make_persistent_id("memory_search"),
                            s.search_title,
                            |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    for kind in SearchKind::ALL {
                                        if ui
                                            .selectable_label(
                                                self.mem_search_kind == kind,
                                                kind.label(s),
                                            )
                                            .clicked()
                                        {
                                            self.mem_search_kind = kind;
                                        }
                                    }
                                });
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.mem_search)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .hint_text(s.search_hint),
                                );
                                if cherché.is_none() {
                                    return;
                                }
                                // Vingt-quatre adresses : au-delà, la liste cesse
                                // d'être une réponse.
                                if trouvés.is_empty() {
                                    ui.colored_label(theme.dim, s.search_none);
                                    return;
                                }
                                ui.colored_label(
                                    theme.dim,
                                    format!("{} {} :", trouvés.len(), s.search_found),
                                );
                                for addr in &trouvés {
                                    let addr = *addr;
                                    let étiquette = egui::Label::new(
                                        egui::RichText::new(format!("{addr:#x}"))
                                            .monospace()
                                            .color(theme.accent),
                                    );
                                    let lien =
                                        cliquable(ui, étiquette).on_hover_text(s.search_goto);
                                    if lien.clicked() {
                                        aller = Some(addr);
                                    }
                                }
                            },
                        );
                        // Écrire ici, à la main : le pendant de l'édition d'un registre.
                        // Un débogueur écrit là où le programme n'a pas le droit — c'est
                        // ce qui permet d'essayer « et si ce tampon contenait ça ? » —,
                        // et le dire vaut mieux que de le cacher.
                        repli(
                            ui,
                            theme,
                            ui.make_persistent_id("memory_poke"),
                            s.poke_title,
                            |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    for kind in SearchKind::ALL {
                                        if ui
                                            .selectable_label(
                                                self.mem_search_kind == kind,
                                                kind.label(s),
                                            )
                                            .clicked()
                                        {
                                            self.mem_search_kind = kind;
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.mem_poke)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(180.0)
                                            .hint_text(s.poke_hint),
                                    );
                                    let octets = self.mem_search_kind.needle(&self.mem_poke);
                                    ui.add_enabled_ui(octets.is_some(), |ui| {
                                        if framed_button(ui, theme, s.poke_write)
                                            .on_hover_text(s.poke_write_hint)
                                            .clicked()
                                            && let Some(octets) = octets
                                        {
                                            écrire = Some(octets);
                                        }
                                    });
                                });
                                ui.colored_label(theme.dim, s.poke_explain);
                            },
                        );
                    },
                );
            });
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

        if let Some(octets) = écrire {
            self.poke_memory(self.mem_addr, &octets);
        }

        // Sauter à une zone de la carte : le vidage suit, et le champ
        // d'adresse avec lui — sans quoi le prochain « Go » ramènerait
        // là d'où l'on vient.
        if let Some(addr) = aller {
            self.mem_addr = addr;
            self.mem_addr_input = format!("{addr:#x}");
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

    /// L'onglet RE : un bouton, et rien d'autre.
    ///
    /// Il portait les sept modules empilés sous un bouton gris. Dans une
    /// colonne de trois cents pixels, cela donnait une page de deux
    /// mille pixels qu'on parcourait en faisant défiler pour retrouver
    /// un titre — et la fenêtre d'analyse, qui montre exactement les
    /// mêmes modules en grand et tout dépliés, était le vrai endroit
    /// pour les lire. Deux fois la même chose, dont une mal.
    ///
    /// Reste ce que l'onglet sait faire de mieux : conduire là-bas.
    fn re_ui(&mut self, ui: &mut egui::Ui) {
        let s = strings(self.lang);
        let theme = self.theme;
        let prêt = self.emulator.is_some() && self.assemble_state.is_some();

        // LA FICHE D'IDENTITÉ D'ABORD, le bouton ensuite.
        //
        // L'onglet ne portait que le bouton, et deux mille pixels de
        // vide sous lui : tout ce qu'il savait vivait derrière une
        // fenêtre qu'il fallait ouvrir. Un onglet dont le seul contenu
        // est « ouvrir une fenêtre » ne mérite pas sa place.
        // L'ÉTAT VIDE, QUI MANQUAIT. C'était le seul onglet du panneau
        // à n'en pas avoir : les six autres disent ce qu'ils attendent
        // et offrent le bouton qui y mène ; celui-ci montrait un bouton
        // gris, et pas un mot. Vu sur une capture, et nulle part
        // ailleurs — aucune mesure ne demandait « cet onglet
        // explique-t-il ce qu'il est ? ».
        //
        // Son détail lui est propre : les autres attendent un programme
        // assemblé, celui-ci attend un FICHIER, et il sert aussi bien
        // pour un binaire qu'on n'a pas écrit. C'est tout son objet.
        if !prêt {
            let peut = self.can_assemble();
            let mut lancer = false;
            ui.add_enabled_ui(peut, |ui| {
                lancer = empty_state(
                    ui,
                    theme,
                    s.no_binary_assembled,
                    Some(s.empty_re_hint),
                    Some(s.action_assemble),
                );
            });
            if lancer {
                self.dispatch(Action::Assemble);
            }
            return;
        }

        self.relever_la_fiche_re();
        if let Some((_, bref)) = self.re_bref.clone() {
            ui.add_space(6.0);
            self.peindre_la_fiche_re(ui, &bref);
            ui.add_space(8.0);
            ui.separator();
        }

        ui.add_space(10.0);
        let bouton = ui
            .add_enabled_ui(prêt, |ui| {
                big_action_button(ui, theme, chrome::icon_analysis, s.analysis_open)
            })
            .inner;
        // Deux infobulles et non une : egui ne montre `on_hover_text`
        // que sur un widget actif, et c'est justement quand le bouton
        // est éteint qu'il faut dire pourquoi.
        let bouton = bouton
            .on_hover_text(s.analysis_hint)
            .on_disabled_hover_text(s.no_binary_assembled);
        if bouton.clicked() {
            // Les rubriques d'outils apprennent lequel est là en
            // ouvrant la vue, et non ici : la question coûte un
            // parcours du PATH, parfois un lancement, et l'analyse
            // s'ouvre aussi par son onglet, par le menu et par la
            // palette. Posée dans `montrer_vue`, elle se pose une fois
            // pour les quatre chemins.
            self.montrer_vue(VueCentrale::Analysis);
        }
        ui.add_space(10.0);
    }

    /// Relève la fiche d'identité du binaire, si elle n'est pas déjà à
    /// jour.
    ///
    /// Une fois par fichier, et pas une de plus : les trois relevés
    /// qu'elle demande — le scan ELF, la reconnaissance de chaîne
    /// d'outils, le portrait des appels — lisent le fichier ENTIER.
    /// Les refaire par image coûterait une image par seconde sur un
    /// gros binaire, ce qui a déjà été mesuré ailleurs dans ce panneau.
    fn relever_la_fiche_re(&mut self) {
        let Some(state) = &self.assemble_state else {
            self.re_bref = None;
            return;
        };
        let fichier = &state.output.file;
        if fichier.is_empty() {
            self.re_bref = None;
            return;
        }
        let clé = (fichier.len(), state.output.load_addr);
        if self.re_bref.as_ref().is_some_and(|(vue, _)| *vue == clé) {
            return;
        }

        let scan = namsdec_re::elf_scan::scan(fichier);
        let symboles: Vec<String> = scan
            .as_ref()
            .map(|s| s.functions.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        let chaîne = namsdec_re::toolchain::detect(fichier, scan.as_ref(), &symboles);
        let fr = self.lang == Lang::Fr;
        let dit = |trouvé: &namsdec_re::toolchain::Finding| {
            (
                trouvé.name.clone(),
                trouvé
                    .confidence
                    .label(if fr { "fr" } else { "en" })
                    .to_owned(),
            )
        };

        let forme = match &scan {
            Some(scan) => format!(
                "ELF64 · {} · {}",
                if scan.pie() { "PIE" } else { "EXEC" },
                if scan.dynamic() { "dyn" } else { "static" }
            ),
            // Pas d'en-tête lisible : des octets bruts, et le dire vaut
            // mieux que de laisser la ligne vide.
            None => if fr { "octets bruts" } else { "raw bytes" }.to_owned(),
        };
        let protections = scan
            .as_ref()
            .map(|scan| {
                vec![
                    ("PIE", scan.pie()),
                    ("NX", scan.nx.unwrap_or(false)),
                    ("RELRO", scan.relro == namsdec_re::elf_scan::Relro::Full),
                    (if fr { "canari" } else { "canary" }, scan.canary),
                ]
            })
            .unwrap_or_default();

        // Ce qu'il sait faire : les fonctions empruntées ET les appels
        // système, comme « En bref ». Les deux, car un programme qui
        // veut passer inaperçu appelle le noyau par son numéro.
        let imports = namsdec_re::imports::scan(fichier);
        let appels = namsdec_re::syscalls::scan(&state.disasm);
        let comportement: Vec<(namsdec_re::behaviour::Family, usize)> = namsdec_re::behaviour::ALL
            .iter()
            .filter_map(|&famille| {
                let combien = imports
                    .iter()
                    .filter(|i| namsdec_re::behaviour::family_of(&i.name) == Some(famille))
                    .count()
                    + appels
                        .iter()
                        .filter(|c| c.family() == Some(famille))
                        .count();
                (combien > 0).then_some((famille, combien))
            })
            .collect();

        let plage = self
            .emulator
            .as_ref()
            .map(|e| e.cpu().mem.range())
            .unwrap_or(0..0);
        let fonctions = namsdec_re::xrefs::scan(&state.disasm, plage.clone())
            .functions()
            .len();
        let chaînes = namsdec_re::strings::scan_elf(fichier)
            .map(|v| v.len())
            .unwrap_or(0);

        self.re_bref = Some((
            clé,
            ReBref {
                taille: fichier.len(),
                forme,
                langage: chaîne.language.as_ref().map(dit),
                compilateur: chaîne.compiler.as_ref().map(dit),
                protections,
                comportement,
                chiffres: (fonctions, chaînes, imports.len(), appels.len()),
            },
        ));
    }

    /// Peint la fiche d'identité. Rien n'est calculé ici.
    ///
    /// SANS GRILLE, et c'est une correction mesurée. Une grille à deux
    /// colonnes dans un panneau étroit donne à la première tout ce que
    /// son étiquette la plus longue demande — ici « Fonctions · chaînes
    /// · emprunts · appels » — et ce qui reste à la seconde. Vu à
    /// l'écran : la taille du fichier coupée, « (GNU) 16.2.1 2026 »
    /// tronqué, le « canari » hors du bord, et la suite de la ligne du
    /// compilateur peinte PAR-DESSUS la ligne des protections.
    ///
    /// Une rangée qui s'enroule par ligne n'a pas ce défaut : chaque
    /// valeur prend la largeur du panneau et passe à la ligne, et deux
    /// lignes valent mieux qu'une phrase coupée.
    fn peindre_la_fiche_re(&mut self, ui: &mut egui::Ui, bref: &ReBref) {
        let s = strings(self.lang);
        let theme = self.theme;
        let ligne = |ui: &mut egui::Ui, étiquette: &str, peindre: &dyn Fn(&mut egui::Ui)| {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(theme.dim, étiquette);
                peindre(ui);
            });
        };

        let taille = format!("{} · {}", bref.forme, group_digits(bref.taille as u64));
        ligne(ui, s.re_bref_file, &|ui| {
            ui.monospace(&taille);
        });
        // Le langage et le compilateur portent leur CERTITUDE : « Rust »
        // sans elle apprend à croire plutôt qu'à lire.
        for (étiquette, trouvé) in [
            (s.re_bref_language, &bref.langage),
            (s.re_bref_compiler, &bref.compilateur),
        ] {
            let Some((nom, certitude)) = trouvé else {
                continue;
            };
            ligne(ui, étiquette, &|ui| {
                ui.monospace(nom);
                ui.weak(format!("({certitude})"));
            });
        }
        if !bref.protections.is_empty() {
            ligne(ui, s.re_bref_protections, &|ui| {
                for (nom, bon) in &bref.protections {
                    // « √ » et « × », et non « ✓ » et « ✗ » : ces
                    // deux-là n'existent pas dans les polices qu'egui
                    // embarque et se peignaient en carré vide. Voir le
                    // même choix dans `binary_info`.
                    ui.colored_label(
                        if *bon { theme.good } else { theme.bad },
                        format!("{} {nom}", if *bon { "\u{221a}" } else { "\u{00d7}" }),
                    );
                }
            });
        }
        let (fonctions, chaînes, empruntées, appels) = bref.chiffres;
        // L'étiquette dit ce que les quatre nombres comptent, et c'est
        // la plus longue de la fiche : elle est donc SUR SA PROPRE
        // LIGNE, au-dessus des nombres, plutôt qu'à leur gauche.
        ui.colored_label(theme.dim, egui::RichText::new(s.re_bref_counts).small())
            .on_hover_text(s.re_bref_counts_hint);
        ui.monospace(format!("{fonctions} · {chaînes} · {empruntées} · {appels}"))
            .on_hover_text(s.re_bref_counts_hint);
        if !bref.comportement.is_empty() {
            ui.add_space(4.0);
            ui.colored_label(theme.dim, s.re_bref_can_do);
            ui.horizontal_wrapped(|ui| {
                for (famille, combien) in &bref.comportement {
                    // La table des teintes vit du côté qui peint, depuis
                    // P7 : `behaviour.rs` relève les familles dans des
                    // noms de fonctions et ne connaît plus egui.
                    ui.colored_label(
                        namsdec_re::vue_peinte::couleur_de_famille(*famille),
                        format!("{} ({combien})", famille.label(self.lang.code())),
                    )
                    .on_hover_text(famille.why(self.lang.code()));
                }
            });
        }
    }

    /// Les modules de rétro-ingénierie.
    ///
    /// `seul` désigne celui qu'on veut, ou `None` pour les avoir tous à
    /// la suite. Les deux servent : la colonne de gauche de la fenêtre
    /// d'analyse en choisit un et lui donne toute la largeur — sept
    /// panneaux dépliés font une page de deux mille pixels où l'on
    /// cherche un titre en faisant défiler —, tandis que « Tout » les
    /// montre à la file, qui est la forme d'un compte rendu.
    ///
    /// Posés ici sans zone de défilement : l'appelant a la sienne, et
    /// deux zones imbriquées ne défilent bien ni l'une ni l'autre.
    pub(crate) fn re_modules_ui(&mut self, ui: &mut egui::Ui, seul: Option<&str>) {
        let s = strings(self.lang);
        let (Some(emulator), Some(state)) = (&self.emulator, &self.assemble_state) else {
            // Ce que l'onglet montrera : un panneau vide qui ne dit
            // pas à quoi il sert n'apprend rien à qui le découvre.
            empty_state(
                ui,
                self.theme,
                s.no_binary_assembled,
                Some(s.empty_re),
                None,
            );
            return;
        };
        let ctx = ReContext {
            cpu: emulator.cpu(),
            disasm: &state.disasm,
            source: &self.source,
            binary: &state.output.file,
            lang: self.lang.code(),
            names: Some(&self.notes.names),
            patches: &self.patches,
        };
        let théme = self.theme;
        // « TOUT » SUIT LE NIVEAU, LUI AUSSI. Une colonne allégée
        // au-dessus d'un « Tout » qui déroule les vingt-cinq panneaux
        // se contredit : on simplifie d'un côté, on rend tout de
        // l'autre, et le débutant tombe sur l'entropie et les
        // fragments ROP sans les avoir demandés. Le COMPTE RENDU, lui
        // — « Copier le rapport », « Enregistrer… » —, reste entier :
        // c'est un export, et un export tronqué serait un piège.
        let niveau = self.niveau();
        for module in self.re_registry.iter_mut() {
            if seul.is_none() && module.niveau() > niveau {
                continue;
            }
            match seul {
                // Un seul, choisi : il prend toute la place, sans cadre
                // ni flèche de repli. Il n'y a rien à replier quand il
                // n'y a rien d'autre.
                Some(voulu) if module.id() == voulu => {
                    ui.add_space(2.0);
                    // La MÊME taille que dans la liste. Le titre
                    // grossissait de deux points dès qu'on cliquait
                    // dessus : même texte, même fenêtre, même place, et
                    // un corps qui change parce qu'on l'a choisi. Un
                    // titre de section vaut treize partout ailleurs —
                    // « Composants », « Thèmes », « Registres
                    // vectoriels (SSE et AVX) ».
                    ui.label(egui::RichText::new(module.title(ctx.lang)).strong());
                    ui.add_space(6.0);
                    module.ui(ui, &ctx);
                    return;
                }
                Some(_) => continue,
                // Tous : chacun dans son cadre, tous dépliés. C'est la
                // forme qu'on relit et qu'on copie.
                None => {
                    egui::Frame::new()
                        .fill(théme.dim.gamma_multiply(0.06))
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .corner_radius(6.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            egui::CollapsingHeader::new(
                                egui::RichText::new(module.title(ctx.lang)).strong(),
                            )
                            .id_salt(module.id())
                            .default_open(true)
                            .show_unindented(ui, |ui| module.ui(ui, &ctx));
                        });
                    ui.add_space(4.0);
                }
            }
        }
    }
}

/// Comment ranger les registres : combien de colonnes, et quel blanc
/// entre elles.
///
/// DEUX BLANCS POSSIBLES, et c'est la hauteur qui choisit.
///
/// Au large, on respire : deux espacements entre deux paires, plus les
/// deux que la grille pose elle-même, sans quoi seize chiffres suivis
/// de trois lettres se lisent comme un seul mot (« …003c RBX »). Mais
/// sur un panneau court, ces blancs-là coûtent parfois une COLONNE
/// entière — et une colonne de plus, ce sont deux rangées de moins,
/// c'est-à-dire la différence entre lire les registres d'un coup d'œil
/// et les faire défiler.
///
/// On ne serre donc **que lorsque le compte ne tombe pas** : tant que
/// les registres tiennent en hauteur, rien ne change. Et serrer ne sert
/// que si cela gagne vraiment une colonne — sinon l'on aurait rétréci
/// les blancs pour rien.
///
/// Pas de boucle de mesure possible : la question posée — « deux
/// colonnes tiendraient-elles dans la hauteur disponible ? » — ne
/// dépend pas du nombre de colonnes choisi, mais de la place laissée,
/// qui est la même dans les deux cas.
pub(crate) fn register_layout(
    width: f32,
    pair: f32,
    spacing: f32,
    height: f32,
    line: f32,
    registers: usize,
) -> (usize, f32) {
    let confortable_gap = spacing * 4.0;
    let serré_gap = spacing / 2.0;
    let confortable = register_columns(width, pair, confortable_gap);
    let serré = register_columns(width, pair, serré_gap);
    // Une rangée par paire, plus celle de RIP, plus le compte
    // d'instructions et le lien des vectoriels : trois lignes qui
    // suivent la grille et comptent dans ce qui doit tenir.
    let tient = |colonnes: usize| {
        let rangées = registers.div_ceil(colonnes.max(1)) + 3;
        rangées as f32 * line <= height
    };
    if tient(confortable) || serré <= confortable {
        (confortable, spacing * 2.0)
    } else {
        (serré, serré_gap)
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

/// La valeur d'un registre nommé, RIP compris.
pub(crate) fn register_value_by_name(cpu: &namsdec_core::Cpu, name: &str) -> u64 {
    if name == "RIP" {
        return cpu.regs.rip;
    }
    register_by_name(name)
        .map(|reg| cpu.regs.get64(reg))
        .unwrap_or(0)
}

/// Ce qu'on tape dans le champ d'un registre.
///
/// **Tout est hexadécimal**, comme le champ d'adresse et comme la valeur
/// affichée : on recopie ce qu'on voit. Un `-` devant donne le
/// complément à deux, parce que « mets -1 » est la façon dont on le
/// pense.
pub(crate) fn parse_register_value(saisie: &str, décimal: bool) -> Option<u64> {
    let saisie = saisie.trim().to_ascii_lowercase();
    let (négatif, chiffres) = match saisie.strip_prefix('-') {
        Some(reste) => (true, reste),
        None => (false, saisie.as_str()),
    };
    // `0x` TRANCHE, QUEL QUE SOIT LE NIVEAU : c'est le préfixe qui dit
    // la base, et celui qui l'écrit sait ce qu'il demande.
    let (base, chiffres) = match chiffres.strip_prefix("0x") {
        Some(reste) => (16, reste),
        // AU NIVEAU DÉBUTANT, CE QU'ON TAPE EST LU COMME CE QU'ON LIT :
        // le volet y écrit « 55 » en clair, et un champ qui rendrait
        // 0x55 comprendrait autre chose que ce qu'il montre.
        None if décimal => (10, chiffres),
        None => (16, chiffres),
    };
    let chiffres = chiffres.replace('_', "");
    if chiffres.is_empty() {
        return None;
    }
    let value = u64::from_str_radix(&chiffres, base).ok()?;
    Some(if négatif {
        value.wrapping_neg()
    } else {
        value
    })
}

/// Les adresses de retour que porte la pile, de RSP vers le haut.
///
/// C'est le `retaddr` de pwndbg : tout mot de la pile qui pointe du code
/// exécutable. Après un débordement, la question est « laquelle a été
/// écrasée », et elle ne se pose que devant la liste entière.
pub(crate) fn return_addresses(
    cpu: &namsdec_core::Cpu,
    notes: &crate::notes::Notes,
    symbols: &[namsdec_asm::elf::Symbol],
    limit: usize,
) -> Vec<(u64, u64, String)> {
    let rsp = cpu.regs.get64(Reg64::Rsp);
    // De RSP jusqu'au haut de sa zone : en dessous, c'est ce qui n'a pas
    // encore été empilé.
    let sommet = cpu
        .mem
        .regions()
        .into_iter()
        .find(|region| region.range.contains(&rsp))
        .map(|region| region.range.end)
        .unwrap_or_else(|| cpu.mem.range().end);
    let mut trouvées = Vec::new();
    let mut addr = rsp;
    while addr + 8 <= sommet && trouvées.len() < limit {
        if let Ok(value) = cpu.mem.read_u64(addr)
            && let namsdec_emu::telescope::What::Code(texte) =
                namsdec_emu::telescope::describe(&cpu.mem, value)
        {
            let nom = named_at(notes, symbols, value).unwrap_or(texte);
            trouvées.push((addr, value, nom));
        }
        addr += 8;
    }
    trouvées
}

/// La valeur d'un registre, écrite comme le niveau la lit.
///
/// **Le défaut qu'elle corrige tenait sur une seule capture.**
/// `établi / un exercice de lecture`, 16 novembre 2026 : l'énoncé
/// demande « Que vaut RAX à l'arrêt ? », la bonne réponse est **55**,
/// et le volet de la machine affichait `0000000000000037` — pendant
/// que la bande du bas, quinze centimètres plus bas, disait déjà
/// « RCX 3 → 2 » en clair. L'exercice portait donc sa propre réponse à
/// l'écran, dans une écriture que celui qui apprend ne sait pas encore
/// lire.
///
/// **La règle est celle de [`nombre_lisible`]**, et c'est tout
/// l'intérêt : sous le million et au niveau débutant, le nombre en
/// clair ; au-delà, les seize chiffres hexadécimaux. Deux endroits qui
/// écrivent le même nombre l'écrivent pareil, ou l'un des deux ment.
///
/// **Aligné à droite sur la largeur de l'hexadécimal**, et non posé à
/// gauche : les colonnes de la grille ne bougent pas quand une valeur
/// passe de 9 à 10, et des nombres se lisent alignés par la droite,
/// unités sous unités. Le remplissage est fait d'espaces, donc
/// invisible — là où l'hexadécimal, lui, garde ses zéros estompés.
///
/// Ce qui reste en hexadécimal au niveau débutant se reconnaît alors à
/// ces zéros de tête : ce sont les adresses — RSP, RIP, un pointeur —,
/// et la légende du volet le dit en toutes lettres.
pub(crate) fn valeur_job(
    ui: &egui::Ui,
    theme: &'static Theme,
    value: u64,
    colour: egui::Color32,
    débutant: bool,
) -> egui::text::LayoutJob {
    if !débutant || value >= 1_000_000 {
        return hex_job(ui, theme, value, colour);
    }
    let police = egui::TextStyle::Monospace.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &format!("{value:>16}"),
        0.0,
        egui::TextFormat {
            font_id: police,
            color: colour,
            ..Default::default()
        },
    );
    job
}

/// Seize chiffres hexadécimaux, les zéros de tête effacés.
///
/// `0000000000040ff00` : sur seize chiffres, la valeur en occupe cinq et
/// le remplissage onze. L'œil doit alors compter les zéros pour trouver
/// où commence le nombre. Ils restent — aligner les colonnes vaut mieux
/// que de les faire danser — mais dans une teinte qui les fait passer
/// derrière, et ce qu'on lit en premier est ce qui compte.
pub(crate) fn hex_job(
    ui: &egui::Ui,
    theme: &'static Theme,
    value: u64,
    colour: egui::Color32,
) -> egui::text::LayoutJob {
    let texte = format!("{value:016x}");
    let police = egui::TextStyle::Monospace.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    // Une valeur nulle s'efface en entier : un zéro clair au bout de
    // quinze zéros sombres attire l'œil sur le seul registre qui n'a
    // rien à dire.
    let significatifs = if value == 0 {
        0
    } else {
        texte.len() - texte.chars().take_while(|c| *c == '0').count()
    };
    let (zéros, reste) = texte.split_at(texte.len() - significatifs);
    if !zéros.is_empty() {
        job.append(
            zéros,
            0.0,
            egui::TextFormat {
                font_id: police.clone(),
                color: theme.dim.gamma_multiply(0.45),
                ..Default::default()
            },
        );
    }
    if !reste.is_empty() {
        job.append(
            reste,
            0.0,
            egui::TextFormat {
                font_id: police,
                color: colour,
                ..Default::default()
            },
        );
    }
    job
}

/// Le même titre, mais qui se replie.
///
/// Rend `true` quand la section est ouverte. Le triangle et le titre
/// forment une seule cible : viser un triangle de six pixels de côté
/// n'est pas un geste, c'est une épreuve.
/// Un repli SANS FONDU : le titre, et ce qu'il cache.
///
/// egui en a un (`CollapsingHeader`), et il déplie en fondu. Le fondu
/// rogne le corps à sa hauteur du moment, si bien qu'un en-tête est
/// coupé en deux pendant que ce qui suit a DÉJÀ pris sa place —
/// signalé sur capture le 14 octobre 2026, où « Écrire ici » barrait la
/// première ligne du vidage mémoire. Mesuré : trois images, qui se
/// voient très bien quand une image coûte trois cents millisecondes sur
/// un gros binaire.
///
/// On ne peut pas désamorcer ce fondu de l'extérieur : l'animation est
/// gardée par le contexte, et la poser d'avance sur sa valeur d'arrivée
/// n'efface pas l'instant où elle a commencé. D'où ce repli-ci, qui
/// n'anime rien — dans un panneau d'outils dense, un fondu de deux
/// dixièmes de seconde n'apprend rien.
///
/// L'état est gardé par egui, comme celui d'un `CollapsingHeader` : il
/// survit donc au redémarrage, et les deux triangles de la vue mémoire
/// se retrouvent repliés comme on les avait laissés.
/// Ce que le vidage de la mémoire garde pour lui, quoi qu'on déplie
/// au-dessus.
///
/// Six lignes de seize octets, ou quatre lignes de texte : de quoi lire
/// quelque chose. En dessous, l'onglet ne montre plus ce qu'il promet,
/// et l'on déplie les outils sans savoir qu'on perd la vue.
const PLANCHER_DU_VIDAGE: f32 = 120.0;

pub(crate) fn repli(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    id: egui::Id,
    titre: &str,
    corps: impl FnOnce(&mut egui::Ui),
) {
    let mut ouvert = ui
        .ctx()
        .data_mut(|données| données.get_persisted::<bool>(id))
        .unwrap_or(false);
    let bande = ui
        .horizontal(|ui| {
            ui.colored_label(theme.dim, if ouvert { "⏷" } else { "⏵" });
            ui.label(egui::RichText::new(titre).color(theme.text));
        })
        .response
        .rect;
    // Un identifiant à nous, et non celui de la rangée : celui-là est
    // tiré du compteur de widgets, et il bouge dès qu'une ligne paraît
    // au-dessus — le clic se perdait alors sans rien dire.
    let réponse = ui.interact(bande, id.with("entête"), egui::Sense::click());
    if réponse.clicked() {
        ouvert = !ouvert;
        ui.ctx()
            .data_mut(|données| données.insert_persisted(id, ouvert));
    }
    if ouvert {
        ui.indent(id, corps);
    }
}

pub(crate) fn section_toggle(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    titre: &str,
    ouvert: &mut bool,
) -> bool {
    ui.add_space(2.0);
    let réponse = ui
        .horizontal(|ui| {
            // LES CHEVRONS D'EGUI, et non les lettres « v » et « > » :
            // en gris pâle à côté d'un titre, un « v » minuscule se lit
            // comme une lettre orpheline, et l'on ne devine pas qu'il
            // annonce un repli. La police les a — demandé à
            // `has_glyph`, qui refuse en revanche ▾ et ▼.
            ui.colored_label(theme.dim, if *ouvert { "⏷" } else { "⏵" });
            ui.label(egui::RichText::new(titre).strong().color(theme.text));
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    souligne_au_survol(ui, &réponse);
    if réponse.clicked() {
        *ouvert = !*ouvert;
    }
    rule(ui, theme);
    *ouvert
}

/// Le titre d'une section du panneau : un mot en gras, un filet sous
/// lui, et de l'air autour.
///
/// Toutes les sections du panneau portent le même : sans une facture
/// commune, un titre en gris clair ressemble à une explication, et l'on
/// ne sait plus ce qui annonce quoi.
/// Un filet pleine largeur, du ton des titres de section.
///
/// Les trois zones du panneau — ce qui vient, l'état du processeur, la
/// vue choisie — ont chacune leur place fixe depuis qu'elles se la
/// partagent. Un trait le dit : sans lui, l'œil cherche où finit l'une
/// et où commence l'autre à chaque fois que leur contenu change.
pub(crate) fn rule(ui: &mut egui::Ui, theme: &'static Theme) {
    ui.add_space(4.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, theme.dim.gamma_multiply(0.35));
    ui.add_space(4.0);
}

/// Les droits d'une zone, à la façon d'un `vmmap` : `rwx`, un tiret pour
/// ce qui est refusé. Trois caractères se comparent d'un coup d'œil, là
/// où trois mots demandent de lire.
pub(crate) fn perms_text(perms: namsdec_core::Perms) -> String {
    let mut out = String::new();
    out.push(if perms.read { 'r' } else { '-' });
    out.push(if perms.write { 'w' } else { '-' });
    out.push(if perms.execute { 'x' } else { '-' });
    out
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

/// Ce qu'un registre vectoriel porte, dit en toutes lettres.
///
/// `haut` est la moitié supérieure des 512 bits — celle que seul
/// AVX-512 atteint. Montrée seulement quand elle n'est pas nulle : un
/// registre resté en 256 bits n'a pas à traîner trente-deux zéros.
pub(crate) fn zmm_tooltip(s: &Strings, value: [u128; 4], fr: bool) -> String {
    let bas = xmm_tooltip(s, [value[0], value[1]]);
    if value[2] == 0 && value[3] == 0 {
        return bas;
    }
    format!(
        "{bas}\n{} {:032x} {:032x}",
        if fr {
            "bits 256-511 :"
        } else {
            "bits 256-511:"
        },
        value[3],
        value[2]
    )
}

/// Tous les bits d'un registre vectoriel, puis leur lecture en
/// flottants.
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

/// Une pastille de flag. Allumée, elle prend la couleur « bon » du thème ;
/// éteinte, elle reste un contour discret. Le 0/1 est gardé à côté du
/// nom : c'est la valeur du flag, et c'est elle qu'on apprend à lire.
///
/// Peinte à la main sur une place réservée d'avance, et non montée en
/// `Frame` : dans une rangée qui se replie, egui décide du passage à la
/// ligne d'après la taille annoncée par l'élément. Un `Frame` ne l'annonce
/// qu'une fois son contenu posé — les six pastilles restaient donc sur une
/// seule ligne, qui dépassait du panneau.
/// La durée du battement d'orange, en secondes : assez long pour qu'on le
/// voie sans avoir à guetter — l'éclat reste plein un moment, puis fond.
pub(crate) const FLASH_SECONDS: f32 = 3.0;

/// La part de la durée où l'éclat reste à PLEIN avant de fondre. Clignoter
/// longuement, c'est d'abord tenir la couleur, puis la laisser partir :
/// une décroissance qui commence au premier instant se voit moins qu'un
/// maintien franc suivi d'un estompage.
const FLASH_HOLD: f32 = 0.45;

/// Où en est le battement : 1 juste après le changement, 0 une fois
/// retombé. Sert à fondre la couleur de l'orange vers la normale.
///
/// Le temps réel, et non le temps d'egui : un pas, puis la frame qui le
/// dessine, puis les suivantes — l'éclat doit décroître à l'horloge, pas
/// au rythme des images.
pub(crate) fn flash_fraction(since: Option<std::time::Instant>) -> f32 {
    let Some(t) = since else { return 0.0 };
    let écoulé = t.elapsed().as_secs_f32();
    if écoulé >= FLASH_SECONDS {
        return 0.0;
    }
    let maintien = FLASH_SECONDS * FLASH_HOLD;
    if écoulé <= maintien {
        // Pleine couleur : on tient l'orange, le temps de le voir.
        1.0
    } else {
        // Puis l'estompage, sur le reste de la durée.
        (1.0 - (écoulé - maintien) / (FLASH_SECONDS - maintien)).clamp(0.0, 1.0)
    }
}

/// Ce qu'un indicateur veut dire, nom développé et rôle, et où il en est.
///
/// Le survol d'une pastille ne disait que « cliquez pour basculer » : on
/// y lisait `CF` sans savoir que c'est la retenue, ni ce qu'elle raconte.
/// Ici chaque indicateur se présente en entier — nom, rôle, état posé ou
/// retombé —, pour que les sept se lisent clairement, pas seulement se
/// voient.
pub(crate) fn flag_description(name: &str, on: bool, lang: Lang) -> String {
    let fr = lang == Lang::Fr;
    let (plein, rôle) = flag_words(name, lang);
    let état = match (on, fr) {
        (true, true) => "posé (1)",
        (true, false) => "set (1)",
        (false, true) => "retombé (0)",
        (false, false) => "clear (0)",
    };
    format!("{name} — {plein} · {état}\n{rôle}")
}

/// Un nombre écrit comme celui qui le lit l'écrirait.
///
/// AU NIVEAU DÉBUTANT, UN PETIT NOMBRE S'ÉCRIT EN DÉCIMAL. Le
/// programme qu'on vient de taper dit `mov rcx, 3` ; la bande du
/// dernier pas répondait `RCX 0x3 -> 0x2`, et il fallait traduire deux
/// fois — une fois pour se reconnaître, une fois pour comprendre. Sous
/// le million, le décimal dit la même chose sans rien demander.
///
/// AU-DESSUS, L'HEXADÉCIMAL RESTE. Ce ne sont plus des quantités mais
/// des ADRESSES — `0x401000` se reconnaît d'un coup d'œil dans le
/// listing, `4198400` ne se reconnaît nulle part. Le million est donc
/// choisi pour cela et non pour sa rondeur : la plus basse adresse
/// qu'un ELF ordinaire emploie est 0x400000, soit 4 194 304.
///
/// Au-dessus du niveau débutant, rien ne change : l'hexadécimal est la
/// langue du métier.
pub(crate) fn nombre_lisible(valeur: u64, débutant: bool) -> String {
    if débutant && valeur < 1_000_000 {
        valeur.to_string()
    } else {
        format!("{valeur:#x}")
    }
}

/// Une condition de but, écrite pour être LUE au niveau débutant.
///
/// « rax == 0xf » devient « RAX vaut 15 ». Trois choses changent, et
/// chacune parce que le reste de l'interface la dit déjà ainsi :
///
/// - le registre en MAJUSCULES, comme le bandeau, la bande du dernier
///   pas et toutes les infobulles ;
/// - l'opérateur en toutes lettres, comme « ZF veut dire “c'était
///   zéro” » ;
/// - le nombre en décimal sous le million, comme « RCX 3 → 2 » de la
///   bande du bas (A 9.87).
///
/// Relevé à l'écran le 11 novembre 2026 : la bande du bas disait
/// « RCX 3 → 2 » pendant que le but, trois centimètres plus haut,
/// disait « rax == 0xf ». Deux façons d'écrire la même chose sur un
/// même écran, et c'est le mode qui se veut le plus clair qui portait
/// la plus obscure.
///
/// AU MODE AVANCÉ, RIEN NE CHANGE : la condition s'y lit telle que
/// l'exercice l'a écrite, dans la grammaire des points d'arrêt — c'est
/// celle qu'on tape soi-même dans le champ d'un point d'arrêt
/// conditionnel, et la réécrire ferait croire à une autre syntaxe.
pub(crate) fn condition_lisible(texte: &str, débutant: bool, lang: Lang) -> String {
    if !débutant {
        return texte.to_owned();
    }
    let Some(condition) = condition::parse(texte) else {
        return texte.to_owned();
    };
    let (gauche, comparaison, droite) = condition.parts();
    let fr = lang == Lang::Fr;
    let mot = match (comparaison, fr) {
        (condition::Comparison::Equal, true) => "vaut",
        (condition::Comparison::Equal, false) => "is",
        (condition::Comparison::NotEqual, true) => "n'est pas",
        (condition::Comparison::NotEqual, false) => "is not",
        (condition::Comparison::Less, true) => "est plus petit que",
        (condition::Comparison::Less, false) => "is less than",
        (condition::Comparison::LessOrEqual, true) => "vaut au plus",
        (condition::Comparison::LessOrEqual, false) => "is at most",
        (condition::Comparison::Greater, true) => "est plus grand que",
        (condition::Comparison::Greater, false) => "is greater than",
        (condition::Comparison::GreaterOrEqual, true) => "vaut au moins",
        (condition::Comparison::GreaterOrEqual, false) => "is at least",
    };
    // Le côté droit est réécrit en décimal quand c'est un NOMBRE écrit
    // en hexadécimal. Une expression — « rbx + 1 », « [rsp] » — est
    // laissée telle quelle : la traduire demanderait de la calculer, et
    // ce que le but annonce doit rester ce que l'auteur a écrit.
    let droite_lisible = match crate::calc::eval_with(droite, None, None) {
        Ok(valeur) if droite.starts_with("0x") || droite.starts_with("0b") => {
            nombre_lisible(valeur, true)
        }
        _ => droite.to_owned(),
    };
    format!("{} {mot} {droite_lisible}", gauche.to_uppercase())
}

/// Le nom plein d'un indicateur, et ce qu'il raconte.
///
/// À PART DE [`flag_description`] depuis le 10 novembre 2026, et pour
/// une raison qui tient à l'enseignement : l'infobulle n'est pas le
/// seul endroit qui en a besoin. Au niveau débutant, un indicateur qui
/// vient de changer est dit EN TOUTES LETTRES dans la bande du dernier
/// pas — « ZF 0 → 1 — le dernier résultat est nul » —, parce que
/// « ZF 0 → 1 » tout seul n'apprend rien à qui ne sait pas ce qu'est ZF.
///
/// Une seule table pour les deux : deux phrases pour le même
/// indicateur seraient deux vérités à tenir d'accord, et c'est ainsi
/// qu'une finit par être fausse.
pub(crate) fn flag_words(name: &str, lang: Lang) -> (&'static str, &'static str) {
    let fr = lang == Lang::Fr;
    match name {
        "CF" => (
            if fr { "retenue" } else { "carry" },
            if fr {
                "une opération non signée a débordé"
            } else {
                "an unsigned operation overflowed"
            },
        ),
        "PF" => (
            if fr { "parité" } else { "parity" },
            if fr {
                "l'octet de poids faible du résultat a un nombre pair de bits à un"
            } else {
                "the low byte of the result has an even number of one bits"
            },
        ),
        "AF" => (
            if fr {
                "retenue auxiliaire"
            } else {
                "auxiliary carry"
            },
            if fr {
                "retenue entre le quartet 3 et le quartet 4 — le calcul décimal codé binaire la lit"
            } else {
                "carry between nibble 3 and 4 — binary-coded decimal reads it"
            },
        ),
        "ZF" => (
            if fr { "zéro" } else { "zero" },
            if fr {
                "le dernier résultat est nul — c'est lui qu'un `je`/`jz` lit"
            } else {
                "the last result was zero — a `je`/`jz` reads it"
            },
        ),
        "SF" => (
            if fr { "signe" } else { "sign" },
            if fr {
                "le dernier résultat est négatif (bit de poids fort à un)"
            } else {
                "the last result is negative (top bit set)"
            },
        ),
        "DF" => (
            "direction",
            if fr {
                "les instructions de chaîne avancent vers le bas ; `std` le pose, `cld` le retire"
            } else {
                "string instructions move downward; `std` sets it, `cld` clears it"
            },
        ),
        "OF" => (
            if fr {
                "débordement signé"
            } else {
                "signed overflow"
            },
            if fr {
                "une opération signée a débordé — à ne pas confondre avec la retenue"
            } else {
                "a signed operation overflowed — not to be confused with carry"
            },
        ),
        _ => ("", ""),
    }
}

pub(crate) fn flag_chip(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    name: &str,
    on: bool,
    flash: f32,
) -> egui::Response {
    // Au repos, vert si l'indicateur est levé, grisé sinon ; le temps
    // d'un battement après un changement, l'orange par-dessus.
    let base = if on { theme.good } else { theme.dim };
    let color = mélange(base, theme.changed, flash);
    let galley = ui.painter().layout_no_wrap(
        format!("{name} {}", u8::from(on)),
        egui::TextStyle::Monospace.resolve(ui.style()),
        color,
    );
    // De simples indicateurs : ils se survolent (l'infobulle les explique)
    // mais ne se cliquent pas. Les basculer à la main brouillait ce qu'on
    // vient de lire — l'état posé par la dernière instruction.
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + egui::vec2(10.0, 4.0), egui::Sense::hover());
    // UN CREUX, ET NON UN CONTOUR. Le trait était de la couleur du
    // texte : sept traits clairs côte à côte dans la barre de titre,
    // aussi marqués que ce qu'ils encadrent, pour sept témoins qui
    // valent zéro la plupart du temps. Le cadre criait, le contenu
    // chuchotait.
    //
    // Le fond de l'éditeur sert de creux : plus SOMBRE que la barre, il
    // sépare les sept sans rien ajouter de clair, et le texte estompé y
    // gagne du contraste au lieu d'en perdre — 4,52 contre 4,16 sur la
    // barre nue, mesuré sur VS Code Dark+. Un indicateur levé teinte ce
    // creux de sa couleur : à l'écran, ce qui est à un se voit
    // maintenant à la tache, et non au seul trait.
    let fond = if on || flash > 0.0 {
        mélange(theme.bg, color, 0.22)
    } else {
        theme.bg
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::from(4), fond);
    let pos = rect.center() - 0.5 * galley.size();
    ui.painter().galley(pos, galley, color);
    response
}

pub(crate) fn hexdump_ui(ui: &mut egui::Ui, id: &str, dump: &str) {
    egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // LES OCTETS S'ÉTENDENT, LES PHRASES SE REPLIENT.
            //
            // Tout le bloc était en `Extend`, ce qui est juste pour une
            // ligne de vidage — seize octets et leur transcription ne
            // doivent pas se replier à la largeur du panneau — et faux
            // pour les phrases qui l'accompagnent. En lecture « Texte »,
            // deux explications sur trois partaient hors de l'écran :
            // « 1 octet, puis un zéro : c'est là que `write` s… ». Vu à
            // l'écran le 20 octobre 2026, à l'origine du grief sur cet
            // onglet.
            //
            // La règle est la plus simple qui tienne : une ligne qui
            // commence par une adresse EST du vidage ; tout le reste est
            // de la prose, et la prose se replie.
            for ligne in dump.lines() {
                let données = ligne.trim_start().starts_with("0x");
                ui.add(
                    egui::Label::new(egui::RichText::new(ligne).monospace()).wrap_mode(
                        if données {
                            egui::TextWrapMode::Extend
                        } else {
                            egui::TextWrapMode::Wrap
                        },
                    ),
                );
            }
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

/// La mémoire lue comme du texte : la chaîne qui commence ici, puis ce
/// que la fenêtre contient de lisible.
///
/// Deux questions se posent devant un tampon, et elles n'ont pas la même
/// réponse. « Qu'est-ce que `write` va envoyer ? » : la chaîne qui
/// commence à cette adresse et s'arrête au premier octet nul, comme le
/// C la définit. « Où est donc mon message ? » : les suites lisibles de
/// la fenêtre, avec leur adresse, comme le fait `strings`.
///
/// Les octets illisibles sont montrés entre chevrons plutôt que
/// remplacés par un point : leur valeur est justement ce qu'on vient
/// vérifier. Mais quand il n'y a rien de lisible du tout — on regarde du
/// code, et `<b8><01>` n'apprend rien à personne — on le dit avec des
/// mots.
pub(crate) fn text_dump(bytes: &[u8], addr: u64, lang: Lang) -> String {
    let fr = lang == Lang::Fr;
    let fin = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let chaîne = &bytes[..fin];
    let lisibles = chaîne.iter().filter(|b| est_lisible(**b)).count();
    let mut out = String::new();

    if fin == 0 {
        out.push_str(if fr {
            "Chaîne vide : le premier octet est un zéro.
"
        } else {
            "Empty string: the first byte is a zero.
"
        });
    } else {
        out.push_str(&format!(
            "{addr:#010x}  {}
",
            échapper(chaîne)
        ));
        let unité = if fr { "octet" } else { "byte" };
        let pluriel = if fin > 1 { "s" } else { "" };
        if fin < bytes.len() {
            out.push_str(&if fr {
                format!(
                    "{fin} {unité}{pluriel}, puis un zéro : c'est là que `write` s'arrêterait.
"
                )
            } else {
                format!(
                    "{fin} {unité}{pluriel}, then a zero: that is where `write` would stop.
"
                )
            });
        }
        // Le diagnostic qui manquait : dire que ce n'est pas du texte,
        // plutôt que de laisser lire `<b8><01>` et se demander pourquoi.
        if lisibles * 2 < fin {
            out.push_str(if fr {
                "Ces octets ne forment pas du texte — c'est du code ou des nombres. Lisez-les en Octets.
"
            } else {
                "These bytes are not text — they are code or numbers. Read them as Bytes.
"
            });
        }
    }

    // Et ce que la fenêtre contient de lisible, où qu'il soit : c'est la
    // réponse à « où est donc mon message ? », qu'on se pose dès que le
    // tampon regardé n'est pas le bon.
    let trouvées = suites_lisibles(bytes, addr, 4);
    if !trouvées.is_empty() && !(trouvées.len() == 1 && trouvées[0].0 == addr) {
        out.push('\n');
        out.push_str(if fr {
            "Texte lisible dans les octets qui suivent :\n"
        } else {
            "Readable text in the bytes that follow:\n"
        });
        for (at, texte) in trouvées {
            out.push_str(&format!("{at:#010x}  {texte}\n"));
        }
    }
    out
}

/// Un octet qu'on peut lire à l'écran : l'ASCII imprimable, plus les deux
/// blancs qui structurent un texte.
fn est_lisible(b: u8) -> bool {
    matches!(b, 0x20..=0x7e | b'\n' | b'\t')
}

/// Les octets d'une chaîne, avec les illisibles entre chevrons.
fn échapper(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        match b {
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(b as char),
            other => out.push_str(&format!("<{other:02x}>")),
        }
    }
    out
}

/// Les suites d'au moins `minimum` caractères lisibles, avec leur
/// adresse — le `strings` de la fenêtre qu'on regarde.
fn suites_lisibles(bytes: &[u8], base: u64, minimum: usize) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    let mut début = 0usize;
    let mut courante = 0usize;
    for (index, &b) in bytes.iter().enumerate() {
        if est_lisible(b) {
            if courante == 0 {
                début = index;
            }
            courante += 1;
            continue;
        }
        if courante >= minimum {
            out.push((base + début as u64, échapper(&bytes[début..index])));
        }
        courante = 0;
    }
    if courante >= minimum {
        out.push((base + début as u64, échapper(&bytes[début..])));
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
    lang: Lang,
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
                Ok(bytes) => text_dump(bytes, addr, lang),
                Err(_) => out_of_range.to_string(),
            }
        }
        MemoryFormat::Chain => chain_dump(mem, addr),
    }
}

/// Huit mots, chacun suivi de pointeur en pointeur.
///
/// C'est `telescope` : à chaque ligne, l'adresse, ce qu'elle contient, et
/// ce que cela désigne — une chaîne, une instruction, un autre pointeur —
/// jusqu'à ce qu'il n'y ait plus rien à suivre.
pub(crate) fn chain_dump(mem: &namsdec_core::Memory, addr: u64) -> String {
    let mut out = String::new();
    for index in 0..8u64 {
        let at = addr + index * 8;
        let links = namsdec_emu::telescope::chain(mem, at, namsdec_emu::telescope::LIMIT);
        let Some(first) = links.first() else { break };
        out.push_str(&format!("{at:#010x}  {:016x}", first.value));
        for link in &links[1..] {
            out.push_str(&format!("  -> {:#x}", link.value));
        }
        if let Some(fin) = chain_end(links.last()) {
            out.push_str("  ");
            out.push_str(&fin);
        }
        out.push('\n');
    }
    out
}

/// Ce qu'il y a au bout de la chaîne, dit en clair.
pub(crate) fn chain_end(last: Option<&namsdec_emu::telescope::Link>) -> Option<String> {
    use namsdec_emu::telescope::What;
    match &last?.what {
        // Une instruction : c'est ainsi qu'on reconnaît une adresse de
        // retour au milieu de nombres.
        What::Code(text) => Some(format!("({text})")),
        What::Text(text) => Some(format!("\"{text}\"")),
        What::Pointer | What::Number => None,
        What::Unreadable => Some("?".to_owned()),
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
/// Cherche un motif dans toute la mémoire simulée.
///
/// D'un seul tenant, et non par tranches : la mémoire tient en quelques
/// mégaoctets, et découper manquerait ce qui est à cheval sur deux
/// morceaux. Rendre une liste vide quand la mémoire n'est pas lisible
/// est le bon silence : il n'y a alors rien à chercher dedans.
pub(crate) fn search_memory(mem: &namsdec_core::Memory, motif: &memfind::Pattern) -> Vec<u64> {
    let étendue = mem.range();
    let taille = (étendue.end - étendue.start) as usize;
    mem.peek_bytes(étendue.start, taille)
        .map(|octets| motif.find_all(octets, étendue.start, MEM_FIND_MAX))
        .unwrap_or_default()
}

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

/// L'explication d'un préfixe, dans la langue de l'interface.
fn prefix_hint(s: &Strings, prefix: namsdec_disasm::Prefix) -> &'static str {
    use namsdec_disasm::Prefix as P;
    match prefix {
        P::RexW => s.prefix_rex_w,
        P::Rex => s.prefix_rex,
        P::OperandSize => s.prefix_operand_size,
        P::AddressSize => s.prefix_address_size,
        P::Rep => s.prefix_rep,
        P::Repne => s.prefix_repne,
        P::Lock => s.prefix_lock,
        P::Segment => s.prefix_segment,
        P::Vex => s.prefix_vex,
    }
}

/// Un nombre lisible : les milliers séparés par une espace fine.
///
/// « 1234567 » ne se lit pas, « 1 234 567 » si — et c'est un nombre
/// qu'on compare d'un coup d'œil à celui d'avant.
pub(crate) fn group_digits(value: u64) -> String {
    let chiffres = value.to_string();
    let mut out = String::with_capacity(chiffres.len() + chiffres.len() / 3);
    for (index, c) in chiffres.chars().enumerate() {
        if index > 0 && (chiffres.len() - index).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    out
}
