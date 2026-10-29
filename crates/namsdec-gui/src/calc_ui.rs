//! La calculatrice à l'écran.
//!
//! `calc.rs` dit ce que vaut une expression et ne connaît ni egui ni
//! pixel — c'est ce qui permet de le tester au caractère près. Ce
//! fichier-ci est l'autre moitié, celle qui montre : le clavier, les
//! bits qu'on retourne du doigt, les quatre bases côte à côte, les
//! registres du programme en cours.
//!
//! La séparation est celle qu'annonce l'en-tête de `calc.rs`, et elle
//! se voit maintenant dans l'arborescence.

use super::*;

impl NamsdecApp {
    /// La calculatrice : une expression, et toutes ses lectures.
    ///
    /// Elle répond pendant qu'on tape, sans bouton « égale ». C'est ce
    /// qui la rend utile : on essaie `0xff & 0x0f`, on voit, on corrige,
    /// on voit. Un bouton imposerait un aller-retour à chaque essai.
    ///
    /// Les registres du programme en cours sont des noms comme les
    /// autres : `rax & 0xff` se lit comme on le pense, et c'est très
    /// exactement la question qu'on se pose devant un `and eax, 0xff`.
    /// La calculatrice : un champ, un clavier, une grille de bits.
    ///
    /// Taillée comme celle de GNOME en mode Programmation, parce que
    /// c'est la disposition qu'on connaît : l'expression en haut, le
    /// résultat en grand, les soixante-quatre bits dessous, et un
    /// clavier qui écrit ce que l'évaluateur lit — `&`, `|`, `<<`, et
    /// non des symboles mathématiques qu'il faudrait retraduire pour les
    /// recopier dans du code.
    ///
    /// S'y ajoute ce qu'aucune calculatrice de bureau ne fait : les
    /// registres du programme en cours, et les dix-huit fonctions qu'on
    /// écrit à la main en lisant un désassemblage.
    pub(crate) fn calculator_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_calculator {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let lang = self.lang;
        let mut open = true;
        let mut fermer_ok = false;
        let regs = self.emulator.as_ref().map(|e| &e.cpu().regs);
        let résultat = calc::eval(&self.calc_input, regs);
        let mut à_copier: Option<String> = None;
        // Relevés dans la fenêtre, agis après : les fermetures empruntent
        // `self` en écriture, et la fenêtre le tient déjà.
        let mut frappe = Frappe::default();
        let mut bit_basculé: Option<u32> = None;
        // Le clavier appartient à l'expression : à l'ouverture, et à
        // nouveau dès qu'un clic dans la fenêtre l'a laissé à personne.
        // Voir `calc_reprendre_clavier`.
        let reprendre_le_clavier =
            !self.calculator_was_open || std::mem::take(&mut self.calc_reprendre_clavier);

        // Ce que l'écran laisse à une fenêtre : sa hauteur moins de quoi
        // voir son bandeau et son ombre. Prise sur l'ÉCRAN et non sur le
        // contenu — une dimension qu'on lit de son propre contenu se
        // reboucle d'une image à l'autre, et c'est le premier piège de
        // cette interface.
        let hauteur_libre = (ctx.viewport_rect().height() - 60.0).max(320.0);
        let fenêtre = egui::Window::new(s.calculator_title)
            .id(egui::Id::new("calculator_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            // Bornée en largeur : la rangée des dix-huit fonctions
            // poussait la fenêtre à douze cents pixels, où un clavier de
            // trois cent soixante-six flottait au milieu du vide.
            // Bornée, la rangée se replie et l'appareil garde sa forme.
            //
            // La fenêtre épouse la hauteur de son contenu. Une hauteur
            // figée laissait cent pixels de vide entre la dernière ligne
            // et le bouton — un appareil ne se termine pas par du vide.
            .default_width(500.0)
            .min_width(420.0)
            .max_width(560.0)
            // Jamais plus haute que l'écran : une fenêtre qui le dépasse
            // pose son bouton hors de vue, et l'on ne sait plus comment
            // la fermer.
            //
            // LA HAUTEUR D'OUVERTURE EST CELLE DE L'ÉCRAN, et c'est ce
            // qui rend la zone défilante inoffensive. Une fenêtre
            // s'agrandit parce que son contenu DÉBORDE de la place qu'on
            // lui laisse : egui l'y amène en quelques images. Une zone
            // qui défile ne déborde jamais — elle prend la place
            // disponible —, si bien qu'une fenêtre sans hauteur imposée
            // restait figée à la taille d'egui par défaut et coupait son
            // propre clavier. Partir de la hauteur de l'écran puis
            // laisser `auto_shrink` redescendre sur le contenu donne les
            // deux : rien n'est coupé, et rien ne se termine par du vide.
            .default_height(hauteur_libre)
            .max_height(hauteur_libre)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(default_window_pos(ctx, "calculator"));
        let rendue = self
            .maximizable(fenêtre, Self::CALCULATOR_WINDOW, ctx)
            .title_bar(false)
            .show(ctx, |ui| {
                if chrome::window_head(ui, self.theme, s, s.calculator_title) {
                    fermer_ok = true;
                }
                // La barre d'action est à la fin du corps, et non en
                // panneau bas : cette fenêtre-ci n'a pas de hauteur
                // imposée. Voir `ok_bar_inline`.

                // Une seule colonne, centrée une seule fois : tout ce qui
                // suit s'y range et partage donc le même bord. Centrer
                // chaque bloc de son côté les faisait diverger dès que la
                // fenêtre changeait de largeur — l'écran d'un côté, le
                // clavier de l'autre, à quarante pixels l'un de l'autre.
                //
                // ET DANS UNE ZONE QUI DÉFILE. Ce qui précède le disait
                // déjà — « la fenêtre épouse la hauteur de son contenu,
                // mais pas au-delà de l'écran » — et ce n'était qu'un
                // commentaire : aucune zone défilante n'existait. La
                // fenêtre borne sa hauteur par
                // `max_height`, mais `max_height` borne ce que la
                // poignée permet, non ce que le contenu prend. Un
                // contenu qui ne défile pas pousse le cadre au-delà :
                // mesuré sur l'écran le plus petit que l'application
                // accepte (800x520), l'appareil faisait 756 px de haut
                // là où il s'était promis 460, et son bandeau — qui
                // porte la croix — se posait 178 px AU-DESSUS du bord.
                // Hors de vue, la fenêtre ne se déplace plus et ne se
                // ferme plus à la souris.
                //
                // La barre « OK » reste HORS de la zone défilante : ce
                // qui ferme une fenêtre ne doit jamais avoir à être
                // amené à l'écran. Et la zone épouse son contenu quand
                // il tient (`auto_shrink` en hauteur), de sorte qu'aucune
                // barre ne paraît sur un écran ordinaire.
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        centered_row(ui, CALC_LARGEUR + 20.0, |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(CALC_LARGEUR + 20.0);
                                // Le résultat en grand, dans la base choisie et sur la
                                // largeur choisie. C'est la ligne qu'on regarde, elle a
                                // donc la taille qui va avec.
                                let valeur =
                                    résultat.as_ref().ok().map(|v| *v & self.calc_width.mask());
                                ui.add_space(8.0);
                                // Tout ce qui suit est centré : le résultat, les bits,
                                // les sélecteurs et le clavier. Alignés à gauche dans une
                                // fenêtre large, ils flottaient contre un bord avec du
                                // vide en face — une page, et non un appareil.
                                //
                                // Le résultat et les bits vivent dans un cadre à part,
                                // sur le fond de l'éditeur : c'est l'écran de
                                // l'appareil, et il se distingue des touches comme sur
                                // n'importe quelle calculatrice.
                                // La lunette de l'écran : dix pixels de chaque côté du
                                // clavier, et son contenu fait exactement la largeur de
                                // celui-ci.
                                egui::Frame::new()
                                    .fill(theme.bg)
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        mélange(theme.bg, theme.text, 0.18),
                                    ))
                                    .corner_radius(10.0)
                                    .inner_margin(egui::Margin::symmetric(10, 10))
                                    .show(ui, |ui| {
                                        ui.set_width(CALC_LARGEUR);
                                        écran_calculatrice(
                                            ui,
                                            theme,
                                            s,
                                            &mut self.calc_input,
                                            reprendre_le_clavier,
                                            &résultat,
                                            valeur,
                                            self.calc_base,
                                            self.calc_width,
                                            &mut à_copier,
                                            &mut bit_basculé,
                                            &mut frappe,
                                        );
                                    });
                                ui.add_space(6.0);
                                // Deux couples « étiquette + liste » de mesures
                                // identiques, de part et d'autre du filet.
                                //
                                // Ils ne l'étaient pas : chaque liste prenait la largeur
                                // de ce qu'elle affichait, si bien que « Binaire » et
                                // « 8 bits » donnaient deux boîtes de tailles
                                // différentes et deux flèches qui ne tombaient pas au
                                // même endroit. Deux réglages jumeaux doivent se lire
                                // comme jumeaux.
                                //
                                // Les largeurs sont mesurées sur les textes les plus
                                // longs — « Hexadécimal », « Largeur » — et non écrites
                                // en dur : les libellés changent de longueur avec la
                                // langue, et un chiffre posé à la main aligne le
                                // français en désalignant l'anglais.
                                let (étiquette, liste) = calculator_pick_widths(ui, s);
                                let couple = étiquette + 6.0 + liste;
                                let rangée = (couple * 2.0 + 20.0).min(CALC_LARGEUR);
                                // Chaque couple dans sa propre case, de mesures
                                // identiques : c'est ce qui les aligne vraiment.
                                //
                                // Tout dans une seule rangée ne suffisait pas. Une
                                // rangée horizontale d'egui centre chaque élément sur sa
                                // hauteur du moment, et ne recale pas ce qui est déjà
                                // posé quand un élément plus haut arrive ensuite — la
                                // liste est plus haute qu'une étiquette, si bien que
                                // « Base », posé avant la première liste, se retrouvait
                                // deux pixels au-dessus de « Largeur », posé après.
                                // Dans deux cases jumelles, les deux subissent la même
                                // chose, donc rien ne bouge.
                                // Une hauteur qui dépasse tout ce que la rangée porte, la
                                // liste comprise.
                                //
                                // C'est la clé, et elle a demandé trois essais. Une
                                // rangée horizontale d'egui centre chaque élément sur la
                                // hauteur qu'elle a *à cet instant*, et ne recale pas ce
                                // qui est déjà posé quand un élément plus haut arrive
                                // ensuite. La liste est plus haute qu'une étiquette :
                                // « Base », posé avant la première liste, se retrouvait
                                // deux pixels au-dessus de « Largeur », posé après. Si
                                // rien ne fait grandir la rangée, personne ne se décale.
                                let hauteur = ui.spacing().interact_size.y.max(30.0);
                                centered_row(ui, rangée, |ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(couple, hauteur),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(étiquette, hauteur),
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    // `set_min_width` en plus de la
                                                    // taille demandée : une case
                                                    // allouée avance sur ce qu'elle
                                                    // CONTIENT, non sur ce qu'on lui a
                                                    // donné. Sans cela, « Base », plus
                                                    // court que « Largeur », laissait
                                                    // sa liste remonter de dix-sept
                                                    // pixels — et les deux flèches ne
                                                    // tombaient pas au même endroit.
                                                    ui.set_min_width(étiquette);
                                                    ui.colored_label(
                                                        theme.dim,
                                                        s.calculator_base_label,
                                                    )
                                                },
                                            );
                                            let bases: Vec<(CalcBase, String)> = CalcBase::ALL
                                                .into_iter()
                                                .map(|b| (b, b.label(s).to_owned()))
                                                .collect();
                                            liste_déroulante(
                                                ui,
                                                theme,
                                                egui::Id::new("calculator_base"),
                                                liste,
                                                &mut self.calc_base,
                                                &bases,
                                                "",
                                            );
                                        },
                                    );
                                    ui.separator();
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(couple, hauteur),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            ui.allocate_ui_with_layout(
                                                egui::vec2(étiquette, hauteur),
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    ui.set_min_width(étiquette);
                                                    ui.colored_label(
                                                        theme.dim,
                                                        s.calculator_width_label,
                                                    )
                                                },
                                            );
                                            let largeurs: Vec<(CalcWidth, String)> = CalcWidth::ALL
                                                .into_iter()
                                                .map(|l| (l, format!("{} bits", l.bits())))
                                                .collect();
                                            liste_déroulante(
                                                ui,
                                                theme,
                                                egui::Id::new("calculator_width"),
                                                liste,
                                                &mut self.calc_width,
                                                &largeurs,
                                                "",
                                            );
                                        },
                                    );
                                });

                                ui.add_space(6.0);
                                calculator_keypad_ui(
                                    ui,
                                    theme,
                                    self.calc_base,
                                    &self.calc_input,
                                    &mut frappe,
                                );

                                // Pas de filet entre le clavier et les fonctions : les
                                // deux blocs se distinguent déjà par leur forme, et une
                                // barre de plus dans une fenêtre qui en a déjà deux
                                // (le bandeau, la barre d'action) fait un formulaire.
                                ui.add_space(8.0);
                                if regs.is_none() {
                                    ui.colored_label(theme.dim, s.calculator_no_registers);
                                }
                                calculator_functions_ui(
                                    ui,
                                    theme,
                                    s,
                                    lang,
                                    &mut self.calc_family,
                                    &mut frappe.écrire,
                                );

                                ui.add_space(6.0);
                                // Toutes les bases à la fois, en petit : c'est ce qu'on
                                // vient chercher quand on ne sait pas encore *laquelle*
                                // on veut. Chacune se copie d'un clic.
                                if let Some(valeur) = valeur {
                                    // Deux paires par rangée : sept lectures empilées
                                    // faisaient une colonne plus haute que le clavier,
                                    // et sur un écran de portable les dernières
                                    // passaient sous le bouton. Côte à côte, elles
                                    // tiennent en quatre lignes — et l'on compare
                                    // d'autant mieux deux bases qu'on les voit
                                    // ensemble.
                                    let lectures = readings::all_bases(lang, valeur);
                                    egui::Grid::new("calculator_bases")
                                        .num_columns(4)
                                        .min_col_width(0.0)
                                        .spacing([10.0, 3.0])
                                        .show(ui, |ui| {
                                            for paire in lectures.chunks(2) {
                                                for (nom, texte) in paire {
                                                    ui.colored_label(theme.dim, *nom);
                                                    let réponse = cliquable(
                                                        ui,
                                                        egui::Label::new(
                                                            egui::RichText::new(texte)
                                                                .monospace()
                                                                .size(11.5)
                                                                .color(theme.text),
                                                        ),
                                                    );
                                                    if réponse
                                                        .on_hover_text(s.calculator_copy)
                                                        .clicked()
                                                    {
                                                        à_copier = Some(texte.clone());
                                                    }
                                                }
                                                ui.end_row();
                                            }
                                        });
                                }
                            });
                        });
                    });
                if ok_bar_inline(ui, theme, s) {
                    fermer_ok = true;
                }
            });
        let cadre = rendue.as_ref().map(|r| r.response.rect);
        self.place_maximize_button(ctx, Self::CALCULATOR_WINDOW, rendue);

        // UN CLIC DANS LA FENÊTRE QUI N'A DONNÉ LE FOCUS À PERSONNE rend
        // le clavier à l'expression. C'est le cas d'une touche du
        // clavier, d'un bit, d'une pastille de fonction : des surfaces
        // qui se cliquent sans se focaliser, et après lesquelles egui ne
        // focalise plus rien. Le champ muet, Ctrl+C et Ctrl+V ne
        // servaient plus — sur une calculatrice, c'est-à-dire là où l'on
        // vient justement chercher une valeur pour la recoller ailleurs.
        //
        // La condition porte sur le CADRE de la fenêtre et non sur
        // l'application entière : en mode Normal, l'éditeur ne prend pas
        // le focus non plus, et une règle écrite sur « personne n'a le
        // focus » volerait la frappe à l'éditeur dès qu'on clique dans
        // le code avec la calculatrice ouverte.
        //
        // Les listes déroulantes « Base » et « Largeur » sont des
        // boutons : elles prennent le focus, donc elles le gardent, et
        // l'on peut choisir une base au clavier sans que l'expression le
        // reprenne sous les doigts.
        if let Some(cadre) = cadre
            && ctx.memory(|m| m.focused().is_none())
            && ctx.input(|i| {
                i.pointer.any_click() && i.pointer.interact_pos().is_some_and(|p| cadre.contains(p))
            })
        {
            self.calc_reprendre_clavier = true;
        }

        if frappe.effacer {
            self.calc_input.clear();
        }
        if frappe.reculer {
            self.calc_input.pop();
        }
        if let Some(morceau) = frappe.écrire {
            self.calc_input.push_str(&morceau);
        }
        if let Some(rang) = bit_basculé
            && let Ok(valeur) = résultat
        {
            // Le bit basculé devient la nouvelle expression, écrite dans
            // la base courante : on repart de ce qu'on vient de poser,
            // comme sur une calculatrice de bureau.
            let nouvelle = (valeur ^ (1u64 << rang)) & self.calc_width.mask();
            self.calc_input = self.calc_base.écrire(nouvelle);
        }
        if let Some(texte) = à_copier {
            ctx.copy_text(texte);
            self.status = Status::Note(|s| s.calculator_copied);
        }
        self.calculator_was_open = self.show_calculator;
        self.show_calculator = open && !fermer_ok;
        if !open {
            self.calculator_was_open = false;
        }
    }
}

/// La grille des bits, façon calculatrice de programmeur.
///
/// Deux rangées de trente-deux, groupées par quartets : c'est ainsi
/// qu'on lit un masque, et c'est ainsi que l'octet se retrouve du
/// premier coup d'œil. Les index encadrent chaque rangée — sans eux, on
/// compte les colonnes avec le doigt.
///
/// Chaque bit se clique pour le basculer. C'est le geste qui manque
/// partout ailleurs : on veut savoir ce que vaut `0x1f` avec le
/// quatrième bit en moins, et l'écrire à la main est plus long que de le
/// cliquer.
///
/// Les largeurs sont posées et l'espacement mis à zéro, ce qui permet de
/// calculer la largeur exacte d'une rangée — et donc de la centrer.
fn calculator_bits_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    valeur: u64,
    largeur: CalcWidth,
    actif: bool,
    basculé: &mut Option<u32>,
) {
    const PAR_RANGÉE: u32 = 32;
    /// La case d'un index, celle d'un bit, et le blanc entre deux
    /// quartets.
    const INDEX_L: f32 = 22.0;
    const BIT_L: f32 = 9.0;
    const QUARTET: f32 = 4.0;

    let bits = largeur.bits();
    let rangées = bits.div_ceil(PAR_RANGÉE);
    let par_rangée = bits.min(PAR_RANGÉE);
    let quartets = par_rangée / 4;
    let largeur_rangée = INDEX_L * 2.0
        + par_rangée as f32 * BIT_L
        + (quartets.saturating_sub(1)) as f32 * QUARTET
        + 6.0;
    // La MÊME police que les autres grilles de données de
    // l'application — registres, pile, vidage, octets — et non douze
    // points écrits ici. Un point d'écart est trop peu pour se lire
    // comme une intention et assez pour se voir ; et une taille écrite
    // en dur ne suit pas le thème quand celui-ci change d'échelle.
    //
    // La case d'un bit fait neuf pixels, quelle que soit la police :
    // rien ne bouge en largeur.
    let police = egui::TextStyle::Monospace.resolve(ui.style());

    for rangée in 0..rangées {
        // De haut en bas, du poids fort au poids faible : la rangée 0
        // porte les bits les plus hauts, comme un nombre s'écrit.
        let haut = bits - rangée * PAR_RANGÉE - 1;
        let bas = haut.saturating_sub(PAR_RANGÉE - 1);
        centered_row(ui, largeur_rangée, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_sized(
                [INDEX_L, 16.0],
                egui::Label::new(
                    egui::RichText::new(haut.to_string())
                        .monospace()
                        .size(10.0)
                        .color(theme.dim),
                ),
            );
            for rang in (bas..=haut).rev() {
                let (case, réponse) =
                    ui.allocate_exact_size(egui::vec2(BIT_L, 16.0), egui::Sense::click());
                let mis = valeur >> rang & 1 == 1;
                if actif && réponse.hovered() {
                    ui.painter().rect_filled(
                        case.expand(1.0),
                        2.0,
                        theme.accent.gamma_multiply(0.35),
                    );
                }
                ui.painter().text(
                    case.center(),
                    egui::Align2::CENTER_CENTER,
                    if mis { "1" } else { "0" },
                    police.clone(),
                    if !actif {
                        // Un mélange, et non une transparence :
                        // `gamma_multiply` ne touche que l'alpha, si
                        // bien que ce gris tombait à 2,1 de contraste
                        // sur le fond — on devinait les zéros plus
                        // qu'on ne les lisait. Mélangé au fond d'un
                        // quart, il reste discret et se lit (3,4).
                        mélange(theme.dim, theme.bg, 0.25)
                    } else if mis {
                        theme.accent
                    } else {
                        theme.dim
                    },
                );
                if actif && réponse.on_hover_text(s.calculator_bits_hint).clicked() {
                    *basculé = Some(rang);
                }
                // Un blanc tous les quatre bits : c'est la coupure du
                // quartet, celle qu'on lit en hexadécimal.
                if rang.is_multiple_of(4) && rang > bas {
                    ui.add_space(QUARTET);
                }
            }
            ui.add_space(6.0);
            ui.add_sized(
                [INDEX_L, 16.0],
                egui::Label::new(
                    egui::RichText::new(bas.to_string())
                        .monospace()
                        .size(10.0)
                        .color(theme.dim),
                ),
            );
        });
    }
}

/// Ce que le clavier a demandé pendant cette image.
///
/// Relevé et non agi sur place : la fenêtre emprunte `self` en écriture
/// pendant qu'elle se dessine, et le clavier ne peut donc pas toucher à
/// l'expression lui-même. Un seul paquet plutôt que trois sorties
/// séparées — c'est la même réponse, elle se transporte ensemble.
#[derive(Default)]
struct Frappe {
    écrire: Option<String>,
    effacer: bool,
    reculer: bool,
}

/// Pose une rangée de largeur connue, centrée dans la place qui reste.
///
/// `vertical_centered` ne centre que les enfants dont la largeur est
/// connue ; une rangée horizontale, elle, prend toute la largeur
/// disponible, si bien qu'elle ne se centre jamais — elle se contente
/// de remplir, et son contenu reste collé à gauche. C'est le piège
/// classique d'egui, et il s'évite en disant soi-même la largeur.
fn centered_row<R>(ui: &mut egui::Ui, largeur: f32, contenu: impl FnOnce(&mut egui::Ui) -> R) -> R {
    // `max_rect` et non `available_width` : la seconde part du curseur,
    // qui a déjà avancé si quelque chose a été posé sur la ligne. On
    // veut la largeur de la place *allouée*, qui ne bouge pas.
    let marge = ((ui.max_rect().width() - largeur) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(marge);
        contenu(ui)
    })
    .inner
}

/// À quelle famille une touche appartient, et donc de quelle couleur
/// elle est.
///
/// DEUX familles : les chiffres, et ce qu'on en fait. C'est ce qui
/// permet de viser une touche sans la lire — on va vers la zone grise
/// pour un chiffre, vers la bleue pour une opération.
///
/// Il y en avait trois : « ce qui efface » portait un rouge franc, sur
/// les deux plus grandes touches de l'appareil. Ces deux-là ont quitté
/// le clavier pour la ligne d'expression (voir `touche_défaire`), et la
/// famille est partie avec elles — un clavier de programmeur n'a pas de
/// touche rouge, il a des chiffres et des opérations.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TouchePour {
    Chiffre,
    Opérateur,
}

impl TouchePour {
    /// Le fond et l'encre de cette famille, dans le thème courant.
    ///
    /// Des couleurs opaques et **franches** : les chiffres sur un gris
    /// net, les opérateurs sur la couleur d'accent. Deux familles, deux
    /// teintes, et l'on vise une touche sans la lire. Des mélanges
    /// timides donnaient six nuances du même bleu sombre, où il fallait
    /// lire chaque touche pour la trouver.
    fn couleurs(self, theme: &Theme) -> (egui::Color32, egui::Color32) {
        let fond = match self {
            TouchePour::Chiffre => mélange(theme.surface, theme.text, 0.30),
            TouchePour::Opérateur => mélange(theme.surface, theme.accent, 0.72),
        };
        (fond, encre_sur(fond, theme))
    }
}

/// La largeur d'une touche, sa hauteur, et l'écart entre deux.
///
/// Des tailles posées et non proportionnelles à la fenêtre : un clavier
/// dont les touches s'étirent n'a plus de forme qu'on reconnaisse, et
/// l'on rate celle qu'on visait après un redimensionnement. C'est le
/// bloc entier qui se centre, pas chaque touche qui s'allonge.
pub(crate) const TOUCHE_H: f32 = 40.0;

const TOUCHE_ÉCART: f32 = 7.0;

const COLONNES: usize = 6;

/// La largeur d'une étiquette et celle d'une liste, dans la rangée
/// « Base » / « Largeur » de la calculatrice.
///
/// Mesurées sur les textes les plus longs de chaque colonne, dans la
/// langue courante : c'est ce qui fait que les deux couples ont la même
/// taille et que les deux flèches tombent au même endroit. Un chiffre
/// écrit en dur alignerait le français en désalignant l'anglais.
fn calculator_pick_widths(ui: &egui::Ui, s: &Strings) -> (f32, f32) {
    let police = egui::TextStyle::Body.resolve(ui.style());
    let mesure = |texte: String| {
        ui.painter()
            .layout_no_wrap(texte, police.clone(), egui::Color32::PLACEHOLDER)
            .size()
            .x
    };
    let étiquette =
        mesure(s.calculator_base_label.to_owned()).max(mesure(s.calculator_width_label.to_owned()));
    let choix = CalcBase::ALL
        .iter()
        .map(|base| mesure(base.label(s).to_owned()))
        .chain(
            CalcWidth::ALL
                .iter()
                .map(|largeur| mesure(format!("{} bits", largeur.bits()))),
        )
        .fold(0.0_f32, f32::max);
    // Ce qu'une liste ajoute autour de son texte, demandé au style et
    // non deviné : l'écart avant la flèche, la flèche, et les deux
    // marges du bouton.
    //
    // Deviné, il l'était de quatre pixels trop peu — et c'est tout ce
    // qu'il fallait pour que la largeur demandée ne suffise plus au plus
    // long des choix. egui prend alors la plus grande des deux, si bien
    // que la liste des bases était six pixels plus large que celle des
    // largeurs, et les deux flèches ne tombaient pas au même endroit.
    let autour =
        ui.spacing().icon_spacing + ui.spacing().icon_width + ui.spacing().button_padding.x * 2.0;
    (étiquette.ceil(), (choix + autour).ceil() + 1.0)
}

/// L'identifiant du champ où l'expression se tape.
///
/// Posé plutôt que laissé à egui : c'est à lui qu'on rend le clavier
/// après un clic sur une touche, et c'est lui qu'un contrôle interroge
/// pour savoir si le collage ira au bon endroit.
pub(crate) fn calc_expression_id() -> egui::Id {
    egui::Id::new("calculator_expression")
}

/// La largeur commune de l'écran et du clavier.
///
/// Les deux blocs sont alignés au pixel, et ce n'est pas une
/// coquetterie : un appareil dont l'écran et le clavier n'ont pas la
/// même largeur a l'air monté de deux morceaux. La valeur vient de la
/// grille de bits, qui est celle des deux qu'on ne peut pas resserrer —
/// trente-deux colonnes lisibles.
pub(crate) const CALC_LARGEUR: f32 = 366.0;

/// La largeur d'une touche, déduite de celle du bloc.
pub(crate) const TOUCHE_L: f32 =
    (CALC_LARGEUR - (COLONNES as f32 - 1.0) * TOUCHE_ÉCART) / COLONNES as f32;

/// Cinq rangées de six touches.
///
/// Les chiffres hexadécimaux sur leur propre rangée, comme sur une
/// calculatrice de programmeur : on les cherche ensemble, on les trouve
/// ensemble. Puis le pavé décimal à gauche et les opérateurs à droite,
/// dans la disposition que tout le monde a dans les doigts.
const TOUCHES: [[&str; COLONNES]; 5] = [
    ["A", "B", "C", "D", "E", "F"],
    ["7", "8", "9", "/", "&", "<<"],
    ["4", "5", "6", "*", "|", ">>"],
    ["1", "2", "3", "-", "^", "~"],
    ["0", "0x", "(", ")", "+", "%"],
];

/// Ce qu'une touche désigne.
fn famille_de(touche: &str) -> TouchePour {
    let chiffre = touche.len() == 1
        && touche
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric());
    if chiffre {
        TouchePour::Chiffre
    } else {
        TouchePour::Opérateur
    }
}

/// Une touche du clavier, posée et peinte. Rend sa réponse.
///
/// Peinte à la main plutôt que confiée à `egui::Button` : dès qu'on
/// impose un fond à un bouton, egui le garde tel quel dans tous les
/// états, et la touche ne répond plus au survol ni au clic. Un clavier
/// qui ne bouge pas sous le doigt a l'air d'une image.
fn calculator_key(
    ui: &mut egui::Ui,
    theme: &Theme,
    texte: &str,
    famille: TouchePour,
    largeur: f32,
    permis: bool,
) -> egui::Response {
    let (rect, réponse) =
        ui.allocate_exact_size(egui::vec2(largeur, TOUCHE_H), egui::Sense::click());
    let (fond, encre) = famille.couleurs(theme);

    // Éteinte, et non cachée : savoir qu'une touche existe mais ne sert
    // pas dans cette base en apprend plus que de la voir disparaître.
    let (fond, encre) = if !permis {
        (
            mélange(theme.surface, fond, 0.35),
            mélange(theme.surface, encre, 0.45),
        )
    } else if réponse.is_pointer_button_down_on() {
        // Enfoncée : elle s'assombrit, comme une vraie touche qui
        // descend.
        (mélange(fond, theme.bg, 0.30), encre)
    } else if réponse.hovered() {
        (mélange(fond, theme.text, 0.14), encre)
    } else {
        (fond, encre)
    };

    ui.painter().rect(
        rect,
        8.0,
        fond,
        egui::Stroke::new(1.0, mélange(fond, theme.text, 0.18)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        texte,
        egui::FontId::monospace(16.0),
        encre,
    );
    réponse
}

/// Le côté d'un des deux boutons qui défont, au bout de la ligne
/// d'expression.
///
/// Carré, et à peine plus grand qu'une ligne de texte : ce sont des
/// commandes de champ, pas des touches de clavier. Leur ancienne taille
/// — la moitié de la largeur de l'appareil, en rouge — disait qu'effacer
/// était l'opération principale.
const DÉFAIRE: f32 = 26.0;

/// Un des deux boutons qui défont. Rend `true` s'il vient d'être cliqué.
///
/// Discret au repos, rouge au survol : il ne doit pas tirer l'œil avant
/// qu'on le cherche, et doit dire ce qu'il fait au moment où on le
/// vise. Éteint, il reste visible — une commande qui disparaît quand
/// elle ne sert pas se cherche en vain quand elle sert.
fn touche_défaire(
    ui: &mut egui::Ui,
    theme: &Theme,
    dessin: fn(&egui::Painter, egui::Rect, egui::Color32),
    permis: bool,
    infobulle: &str,
) -> bool {
    let (rect, réponse) = ui.allocate_exact_size(
        egui::vec2(DÉFAIRE, DÉFAIRE),
        if permis {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let (fond, encre) = if !permis {
        (
            egui::Color32::TRANSPARENT,
            mélange(theme.bg, theme.dim, 0.45),
        )
    } else if réponse.is_pointer_button_down_on() {
        (
            mélange(theme.bg, theme.bad, 0.75),
            encre_sur(theme.bad, theme),
        )
    } else if réponse.hovered() {
        (
            mélange(theme.bg, theme.bad, 0.55),
            encre_sur(theme.bad, theme),
        )
    } else {
        (egui::Color32::TRANSPARENT, theme.dim)
    };
    if fond != egui::Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 6.0, fond);
    }
    dessin(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 13.0)),
        encre,
    );
    // L'infobulle est posée DANS LES DEUX CAS, et c'est tout l'objet
    // d'un bouton peint à la main : `on_disabled_hover_text` n'apparaît
    // que sur un widget qu'egui sait éteint, et celui-ci ne l'est pas —
    // il est seulement peint comme tel. Éteint, il dit pourquoi ; sans
    // cette ligne, on le survolait, rien ne venait, et l'on croyait
    // l'interface en panne plutôt que l'expression vide.
    let réponse = réponse.on_hover_text(infobulle);
    permis && réponse.clicked()
}

/// L'écran de la calculatrice : le résultat, et les bits.
///
/// Dans son cadre, sur le fond de l'éditeur : c'est ce qui distingue ce
/// que l'appareil *montre* de ce sur quoi on *appuie*. Sans cette
/// séparation, le résultat n'était qu'une ligne de texte parmi d'autres,
/// et l'on cherchait des yeux la réponse qu'on venait de demander.
#[allow(clippy::too_many_arguments)]
fn écran_calculatrice(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    expression: &mut String,
    rendre_le_clavier: bool,
    résultat: &Result<u64, calc::CalcError>,
    valeur: Option<u64>,
    base: CalcBase,
    largeur: CalcWidth,
    à_copier: &mut Option<String>,
    bit_basculé: &mut Option<u32>,
    frappe: &mut Frappe,
) {
    // L'expression **dans** l'écran, en grand : c'est la ligne du haut
    // d'une calculatrice, celle où l'on tape. Un champ de saisie
    // encadré, posé au-dessus avec son étiquette « Expression »,
    // faisait un formulaire.
    //
    // Alignée à gauche, et le résultat aussi. Une calculatrice de
    // bureau cale à droite parce qu'on y entre un nombre chiffre par
    // chiffre ; ici on écrit une *expression*, qui se lit de gauche à
    // droite comme la ligne de code d'où elle sort.
    //
    // Sans cadre et sans fond : le cadre de l'écran est déjà là, et un
    // second liseré à l'intérieur ferait deux boîtes imbriquées. C'est
    // le curseur qui dit que la ligne s'écrit.
    // LE CLAVIER EST REPRIS AVANT que le champ ne soit posé, et le
    // curseur avec lui. Fait après, le focus arrivait bien mais le
    // curseur restait à zéro : une touche cliquée écrivait à la fin de
    // l'expression, et le collage qui suivait se posait au DÉBUT —
    // « 0x » cliqué puis « ff » collé donnait `ff0x`. Le champ lit son
    // état en entrant ; c'est donc avant d'y entrer qu'il faut l'écrire.
    if rendre_le_clavier {
        ui.ctx()
            .memory_mut(|m| m.request_focus(calc_expression_id()));
        if let Some(mut état) = egui::TextEdit::load_state(ui.ctx(), calc_expression_id()) {
            let fin = egui::text::CCursor::new(expression.chars().count());
            état
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(fin)));
            egui::TextEdit::store_state(ui.ctx(), calc_expression_id(), état);
        }
    }
    // AU BOUT DE LA LIGNE, les deux gestes qui défont : le retour
    // arrière et la croix. Ils tenaient un bandeau rouge pleine largeur
    // au-dessus du clavier — la plus grande surface de l'appareil, et
    // la plus criarde, pour ce qu'on fait quand on s'est trompé. Ici,
    // ils sont là où est ce qu'ils effacent, comme la croix d'un champ
    // de recherche, et le clavier redevient un pavé régulier.
    //
    // LA RANGÉE A UNE HAUTEUR IMPOSÉE. Une rangée horizontale d'egui
    // centre chaque élément sur la hauteur qu'elle a à cet instant et
    // ne recale pas ce qui est déjà posé : le champ, posé avant des
    // boutons plus hauts que lui, se serait retrouvé deux pixels trop
    // haut. C'est le piège qui a déjà coûté trois essais à la rangée
    // « Base / Largeur », quelques lignes plus bas.
    let hauteur = DÉFAIRE.max(ui.spacing().interact_size.y);
    let champ = ui
        .allocate_ui_with_layout(
            egui::vec2(ui.available_width(), hauteur),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let réservé = DÉFAIRE * 2.0 + 6.0 + 6.0;
                let champ = ui.add(
                    egui::TextEdit::singleline(expression)
                        // Un identifiant posé et non dérivé de la place du
                        // widget : c'est par lui qu'on sait, de l'extérieur,
                        // si le clavier est bien à l'expression — et un
                        // contrôle qui ne peut pas nommer ce qu'il mesure ne
                        // mesure rien.
                        .id(calc_expression_id())
                        .hint_text(s.calculator_hint_text)
                        .font(egui::FontId::monospace(19.0))
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .desired_width((ui.available_width() - réservé).max(60.0)),
                );
                // Éteints sur une expression vide, et non cachés : savoir
                // qu'ils existent avant d'en avoir besoin vaut mieux que
                // de les voir apparaître.
                let vivants = !expression.is_empty();
                let dire = |actif: &'static str| {
                    if vivants {
                        actif
                    } else {
                        s.calculator_nothing_to_erase
                    }
                };
                frappe.reculer |= touche_défaire(
                    ui,
                    theme,
                    chrome::icon_backspace,
                    vivants,
                    dire(s.calculator_backspace),
                );
                frappe.effacer |= touche_défaire(
                    ui,
                    theme,
                    chrome::icon_clear,
                    vivants,
                    dire(s.calculator_clear),
                );
                champ
            },
        )
        .inner;
    // La demande a été faite plus haut ; ce qui reste ici est le cas de
    // la toute première image, où le champ n'avait pas encore d'état à
    // charger.
    if rendre_le_clavier && !champ.has_focus() {
        champ.request_focus();
    }
    ui.add_space(2.0);

    match (résultat, valeur) {
        (Ok(_), Some(valeur)) => {
            let texte = base.écrire(valeur);
            let réponse = cliquable(
                ui,
                egui::Label::new(
                    egui::RichText::new(&texte)
                        .monospace()
                        .size(28.0)
                        .color(theme.accent),
                ),
            );
            if réponse.on_hover_text(s.calculator_copy).clicked() {
                *à_copier = Some(texte);
            }
        }
        (Err(erreur), _) => {
            // Vide, on ne reproche rien : la fenêtre vient de s'ouvrir,
            // il n'y a pas d'erreur à signaler. La place reste prise,
            // sinon tout ce qui suit sauterait d'une ligne à la première
            // frappe.
            if matches!(erreur, calc::CalcError::Empty) {
                ui.add_space(34.0);
            } else {
                ui.colored_label(theme.bad, calc_error_text(s, erreur));
            }
        }
        _ => {}
    }
    ui.add_space(8.0);
    calculator_bits_ui(
        ui,
        theme,
        s,
        valeur.unwrap_or(0),
        largeur,
        valeur.is_some(),
        bit_basculé,
    );
}

/// Le clavier de la calculatrice.
///
/// Il écrit ce que l'évaluateur lit — `&`, `|`, `^`, `<<` — et non des
/// symboles mathématiques qu'il faudrait retraduire pour les recopier
/// dans du code. C'est la même raison qui fait écrire `qword [rsp]`
/// plutôt qu'une notation à nous : namsdec sert à lire du code, et ce
/// qu'on y tape doit ressembler à ce qu'on y lit.
///
/// Les chiffres qui n'ont pas de sens dans la base courante sont
/// éteints : un clavier qui propose `9` en binaire ment sur ce qu'on
/// peut taper.
///
/// **Chaque touche est un vrai bouton**, avec son fond, son liseré et sa
/// couleur de famille. Au repos, le thème rend le fond des boutons
/// transparent et le liseré invisible : le clavier n'était alors qu'une
/// trentaine de caractères posés dans du vide, qu'on ne pensait même pas
/// à cliquer.
fn calculator_keypad_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    base: CalcBase,
    courant: &str,
    frappe: &mut Frappe,
) {
    let total = CALC_LARGEUR;

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(TOUCHE_ÉCART, TOUCHE_ÉCART);

        // Rien au-dessus des chiffres : « Effacer tout » et le retour
        // arrière sont remontés sur la ligne d'expression, là où est ce
        // qu'ils effacent. Voir `écran_calculatrice`.
        for rangée in TOUCHES {
            centered_row(ui, total, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(TOUCHE_ÉCART, TOUCHE_ÉCART);
                for étiquette in rangée {
                    let famille = famille_de(étiquette);
                    let permis = famille != TouchePour::Chiffre
                        || étiquette.chars().all(|c| base.accepte(c));
                    if calculator_key(ui, theme, étiquette, famille, TOUCHE_L, permis).clicked() {
                        frappe.écrire = Some(écriture_de_touche(étiquette, base, courant));
                    }
                }
            });
        }
    });
}

/// Ce qu'une touche du clavier écrit dans l'expression.
///
/// Un chiffre qui **commence un nombre** entre avec le préfixe de la
/// base : en hexadécimal, `1` puis `f` doit donner `0x1f` et non `1f`,
/// que l'évaluateur refuserait. Un chiffre qui continue un nombre entre
/// seul — sans quoi on écrirait `0x10x5`.
///
/// « Commence un nombre » se reconnaît au dernier caractère déjà là :
/// une lettre, un chiffre ou un point le continuent, tout le reste
/// l'ouvre. C'est grossier, et c'est juste dans tous les cas qu'on
/// rencontre.
pub(crate) fn écriture_de_touche(touche: &str, base: CalcBase, courant: &str) -> String {
    let est_un_chiffre = touche.len() == 1
        && touche
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric());
    if !est_un_chiffre {
        return touche.to_owned();
    }
    let continue_un_nombre = courant
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_alphanumeric());
    if continue_un_nombre {
        touche.to_owned()
    } else {
        format!("{}{touche}", base.prefix())
    }
}

/// Les fonctions de rétro-ingénierie, en boutons.
///
/// Une grille qui se replie à la largeur de la fenêtre, et **une ligne
/// sous elle** qui dit ce que fait celle qu'on survole. L'infobulle
/// seule ne suffisait pas : elle se mérite après une demi-seconde
/// d'immobilité, alors qu'on parcourt les noms des yeux pour trouver
/// le bon.
///
/// UNE FAMILLE À LA FOIS, choisie par une rangée d'onglets. Les
/// quarante-sept fonctions tenaient sur douze lignes, et douze lignes
/// de plus poussaient le bas de la fenêtre — les bases, le bouton —
/// hors de l'écran. Mesuré : la fenêtre dépassait l'écran de trois
/// pixels sur un banc de 1100, et débordait pour de bon à l'usage.
///
/// Rangées plutôt que tronquées : on ne cherche pas « une fonction »,
/// on cherche à lire un bit, à changer l'ordre, à compter. L'onglet
/// pose la question avant qu'on ait lu les noms.
fn calculator_functions_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    s: &Strings,
    lang: Lang,
    famille_active: &mut String,
    à_écrire: &mut Option<String>,
) {
    let mut survolée: Option<&'static str> = None;
    // RANGÉES PAR FAMILLE, et non à plat : quarante-sept pastilles les
    // unes derrière les autres se parcourent une à une, et l'on finit
    // par ne plus voir celles du bas. Avec un intertitre par question —
    // lire un bit, changer l'ordre, compter —, on sait où regarder
    // avant d'avoir lu.
    // Les onglets : courts, et dans l'ordre où l'on apprend — lire un
    // bit avant de les compter, les compter avant de les faire tourner.
    //
    // À LA TAILLE DES AUTRES ONGLETS. Ceux-ci étaient en onze quand les
    // cinq du panneau latéral — Octets, Tas, Mémoire, Pile, Processeur —
    // sont en treize : même rôle, deux corps, et onze ne paraissait
    // nulle part ailleurs en proportionnelle. La rangée se replie si
    // elle manque de place, ce qu'elle savait déjà faire.
    //
    // ET DE LA MÊME FORME QU'ELLES, depuis le 28 octobre 2026 : elle
    // passe par `rangée_d_onglets`, comme les vues, le panneau latéral,
    // les préférences et la console. Au bleu plein de `Button::selectable`,
    // la famille choisie avait l'apparence du bouton « OK » posé deux
    // rangées plus bas, et celle des pastilles de fonctions juste
    // dessous — trois rôles, une seule peinture. Le repli dont parle la
    // note qui suit est désormais celui de la rangée, qui mesure ses
    // étiquettes avant de les poser au lieu de découvrir en les peignant
    // qu'elles débordent.
    let étiquettes: Vec<(&str, Option<&str>)> = calc::FAMILIES
        .iter()
        .map(|(_, court, long)| (*court, Some(*long)))
        .collect();
    let actif = calc::FAMILIES
        .iter()
        .position(|(famille, _, _)| famille_active == famille)
        .unwrap_or(0);
    if let Some(rang) = rangée_d_onglets(ui, theme, &étiquettes, actif, 0.0, |_| {}) {
        *famille_active = calc::FAMILIES[rang].0.to_owned();
    }
    {
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            for (nom, arité, fam, fr, en) in calc::FUNCTIONS {
                if fam != famille_active {
                    continue;
                }
                // LE NOM SEUL sur la pastille, l'appel complet dans
                // l'infobulle : écrits dessus, les « (x, n) » doublent
                // la largeur de chaque pastille, et les quarante-sept
                // poussaient le bas de la fenêtre — les bases, et
                // jusqu'au message d'erreur — hors de l'écran.
                let appel = match *arité {
                    3 => format!("{nom}(x, h, b)"),
                    2 => format!("{nom}(x, n)"),
                    _ => format!("{nom}(x)"),
                };
                let fond = mélange(theme.surface, theme.text, 0.14);
                let réponse = ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(*nom)
                                .monospace()
                                .size(12.0)
                                .color(encre_sur(fond, theme)),
                        )
                        .fill(fond)
                        .corner_radius(6.0),
                    )
                    .on_hover_text(&appel);
                if réponse.hovered() {
                    survolée = Some(if lang == Lang::Fr { *fr } else { *en });
                }
                if réponse.clicked() {
                    // Le nom et sa parenthèse : on cherche ce qu'une
                    // fonction fait, on la clique, on complète.
                    *à_écrire = Some(format!("{nom}("));
                }
            }
        });
    }
    ui.add_space(2.0);
    // La place de la ligne est réservée même quand rien n'est survolé :
    // sinon la grille sautait d'une ligne sous le pointeur.
    ui.colored_label(
        if survolée.is_some() {
            theme.text
        } else {
            theme.dim
        },
        survolée.unwrap_or(s.calculator_pick_function),
    );
}

fn calc_error_text(s: &Strings, erreur: &calc::CalcError) -> String {
    use calc::CalcError as E;
    match erreur {
        E::Empty => String::new(),
        E::BadChar(c) => format!("{} {c}", s.calc_bad_char),
        E::Unknown(mot) => format!("{} {mot}", s.calc_unknown),
        E::Unbalanced => s.calc_unbalanced.to_owned(),
        E::Missing => s.calc_missing.to_owned(),
        E::Arity { name, wanted, got } => {
            format!(
                "{name} {} {wanted}, {} {got}",
                s.calc_arity_wants, s.calc_arity_got
            )
        }
        E::NoRegisters(nom) => format!("{nom} : {}", s.calc_no_program),
        E::DivideByZero => s.calc_divide_by_zero.to_owned(),
        E::Trailing(reste) => format!("{} {reste}", s.calc_trailing),
        E::NoMemory => s.calc_no_memory.to_owned(),
        E::Unreadable(addr) => format!("{} {addr:#x}", s.calc_unreadable),
    }
}
