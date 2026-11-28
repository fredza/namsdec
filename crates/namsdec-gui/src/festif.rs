//! Le thème qui bouge un peu : quelques flocons, et de loin en loin une
//! boule.
//!
//! **Discret, et c'est la contrainte entière.** Un atelier où l'on
//! passe la journée à lire de l'assembleur n'a pas à scintiller : les
//! flocons sont une douzaine, gris pâle sur un fond presque noir, de un
//! à deux pixels ; les boules paraissent une à la fois, quelques
//! secondes toutes les trente, et s'effacent d'elles-mêmes. Un réglage
//! les éteint tous (Préférences › Général), et seuls les thèmes qui se
//! déclarent festifs en ont.
//!
//! **TOUT SE CALCULE À PARTIR DU TEMPS, et de rien d'autre.** Aucune
//! position n'est gardée d'une image à l'autre, aucun générateur
//! aléatoire n'a d'état : la place d'un flocon à l'instant `t` est une
//! fonction de `t` et de son numéro. Trois raisons, et la troisième est
//! la vraie :
//!
//! 1. rien à initialiser, rien à remettre à zéro quand la fenêtre
//!    change de taille ;
//! 2. l'animation ne dérive pas quand une image est sautée — elle
//!    dépend de l'horloge, pas du nombre d'images ;
//! 3. **les captures restent reproductibles.** Le banc rend chaque
//!    scène deux fois et compare les images ; une neige tirée au sort
//!    les ferait différer, et « une image qui bouge seule fait douter
//!    de toutes les autres ».
//!
//! Le prix, dit franchement : tant que l'animation tourne, la fenêtre
//! se redessine environ trente fois par seconde. C'est ce que coûte
//! toute animation, et c'est pourquoi le réglage existe.

use egui::{Color32, Pos2, Rect};
use namsdec_theme::Theme;

/// Combien de flocons. Douze, et non cent : on doit pouvoir les
/// compter du coin de l'œil sans que le regard s'y accroche.
pub(crate) const FLOCONS: usize = 12;

/// Combien de boules peuvent paraître — une à la fois le plus souvent,
/// puisque leurs périodes sont premières entre elles.
pub(crate) const BOULES: usize = 3;

/// Le temps entre deux images quand l'animation tourne.
///
/// Trente-trois millisecondes : trente images par seconde. Soixante ne
/// rendrait pas un flocon plus fluide à cette vitesse — ils tombent de
/// dix à vingt pixels par seconde — et doublerait la dépense.
pub(crate) const PAS: std::time::Duration = std::time::Duration::from_millis(33);

/// Un nombre entre 0 et 1, tiré d'un entier. Toujours le même pour le
/// même entier : c'est du hasard *figé*, ce qui est exactement ce qu'on
/// veut — des flocons dispersés, mais dispersés pareil à chaque
/// lancement.
fn tirage(graine: u32) -> f32 {
    // Mélange entier (xorshift multiplicatif), puis ramené dans [0, 1[.
    let mut x = graine.wrapping_mul(2_654_435_761).wrapping_add(1);
    x ^= x >> 15;
    x = x.wrapping_mul(2_246_822_519);
    x ^= x >> 13;
    x = x.wrapping_mul(3_266_489_917);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Entre deux bornes, à partir du même tirage.
fn entre(graine: u32, bas: f32, haut: f32) -> f32 {
    bas + tirage(graine) * (haut - bas)
}

/// Un flocon à l'instant `t` : où il est, sa taille, son encre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Flocon {
    pub place: Pos2,
    pub rayon: f32,
    pub encre: Color32,
}

/// Les flocons de cette image.
///
/// Ils tombent lentement et dérivent de quelques pixels, chacun à son
/// rythme ; arrivés en bas, ils reparaissent en haut — le modulo s'en
/// charge, sans qu'on ait à les suivre.
pub(crate) fn flocons(cadre: Rect, t: f32, theme: &Theme) -> Vec<Flocon> {
    if cadre.width() < 1.0 || cadre.height() < 1.0 {
        return Vec::new();
    }
    // La neige d'un thème clair serait blanche sur blanc : c'est le
    // texte du thème qu'on prend, et il est sombre là où le fond est
    // clair.
    let base = theme.text;
    (0..FLOCONS as u32)
        .map(|i| {
            let vitesse = entre(i * 7 + 1, 9.0, 21.0);
            let hauteur = cadre.height() + 24.0;
            let départ = tirage(i * 13 + 3) * hauteur;
            let y = cadre.top() - 12.0 + (départ + vitesse * t) % hauteur;

            let dérive = entre(i * 17 + 5, 4.0, 16.0);
            let période = entre(i * 19 + 7, 5.0, 11.0);
            let phase = tirage(i * 23 + 11) * std::f32::consts::TAU;
            let x = cadre.left()
                + tirage(i * 29 + 13) * cadre.width()
                + dérive * (t * std::f32::consts::TAU / période + phase).sin();

            Flocon {
                place: Pos2::new(x, y),
                rayon: entre(i * 31 + 17, 1.0, 2.2),
                // Entre 10 % et 22 % : à peine posé sur le fond. Au-delà,
                // un flocon qui passe sur une ligne de code se lit comme
                // un point.
                encre: base.gamma_multiply(entre(i * 37 + 19, 0.10, 0.22)),
            }
        })
        .collect()
}

/// Une boule de Noël, le temps qu'elle paraît.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Boule {
    pub place: Pos2,
    pub rayon: f32,
    pub encre: Color32,
    /// De 0 à 1 : ce qui monte puis redescend, et qu'on multiplie à
    /// l'encre. Une boule qui apparaîtrait d'un coup serait un défaut
    /// d'affichage ; une qui enfle et s'efface est une décoration.
    pub éclat: f32,
}

/// Combien de temps une boule reste visible, en secondes.
const BOULE_DURÉE: f32 = 7.0;

/// Les boules de cette image — souvent aucune, parfois une.
///
/// Chaque emplacement a sa période, et elles sont choisies pour ne pas
/// retomber ensemble : vingt-trois, trente et un et quarante et une
/// secondes sont premiers entre eux, de sorte que les trois ne
/// coïncident qu'une fois toutes les huit heures.
pub(crate) fn boules(cadre: Rect, t: f32, theme: &Theme) -> Vec<Boule> {
    if cadre.width() < 1.0 || cadre.height() < 1.0 {
        return Vec::new();
    }
    const PÉRIODES: [f32; BOULES] = [23.0, 31.0, 41.0];
    // Rouge, or, vert : les trois couleurs d'un sapin, prises dans le
    // thème plutôt qu'écrites ici — un thème festif clair les donnera
    // sombres, et la boule restera visible.
    let couleurs = [theme.bad, theme.accent, theme.good];

    (0..BOULES)
        .filter_map(|j| {
            let période = PÉRIODES[j];
            let décalage = tirage(j as u32 * 101 + 3) * période;
            let avancement = (t + décalage) % période;
            if avancement > BOULE_DURÉE {
                return None;
            }
            // Le tour en cours : la place change à chaque fois qu'elle
            // revient, sans quoi ce serait toujours la même boule au
            // même endroit.
            let tour = ((t + décalage) / période).floor().max(0.0) as u32;
            let graine = j as u32 * 977 + tour * 31;
            // Un demi-cycle de sinus : zéro au début, un au milieu,
            // zéro à la fin.
            let éclat = (avancement / BOULE_DURÉE * std::f32::consts::PI).sin();
            // ÉCARTÉE DU MILIEU DE L'ÉCRAN. Le centre est l'éditeur,
            // c'est-à-dire le texte qu'on lit ; les boules vivent dans
            // le cinquième gauche ou droit, là où une barre, un bord ou
            // un panneau laissent du vide.
            let côté = if tirage(graine + 5) < 0.5 { 0.08 } else { 0.92 };
            let x = cadre.left() + cadre.width() * (côté + entre(graine + 7, -0.05, 0.05));
            let y = cadre.top() + cadre.height() * entre(graine + 11, 0.12, 0.88);
            Some(Boule {
                place: Pos2::new(x, y),
                rayon: entre(graine + 13, 5.0, 8.0),
                encre: couleurs[j % couleurs.len()],
                éclat,
            })
        })
        .collect()
}

/// Peint l'animation et demande l'image suivante.
///
/// **Dans la couche du fond, et à la fin de l'image** : les panneaux y
/// ont déjà posé leurs fonds et leurs textes, donc la neige passe
/// par-dessus eux — c'est ce qu'on veut, de la neige devant une
/// fenêtre — mais elle reste SOUS tout ce qui est une `Window` : les
/// préférences, l'aide, la calculatrice, les menus et les infobulles
/// vivent dans des couches supérieures. Aucune boîte de dialogue n'est
/// donc jamais décorée contre son gré.
pub(crate) fn peindre(ctx: &egui::Context, theme: &Theme) {
    let cadre = ctx.viewport_rect();
    let t = ctx.input(|i| i.time) as f32;
    let peintre = ctx.layer_painter(egui::LayerId::background());

    for flocon in flocons(cadre, t, theme) {
        peintre.circle_filled(flocon.place, flocon.rayon, flocon.encre);
    }
    for boule in boules(cadre, t, theme) {
        // CREUSE, ET NON PLEINE. Peinte en disque, la première version
        // s'est posée sur « _start: » comme une tache brune : on ne
        // voyait pas une décoration mais un défaut d'affichage, et le
        // code dessous devenait illisible. Un cerceau laisse passer ce
        // qu'il y a derrière — c'est ce qui permet de la laisser
        // paraître n'importe où sans gêner la lecture.
        //
        // L'éclat module la transparence : elle naît, se montre, s'en
        // va.
        let encre = boule.encre.gamma_multiply(0.42 * boule.éclat);
        let attache = theme.dim.gamma_multiply(0.30 * boule.éclat);
        peintre.circle_stroke(boule.place, boule.rayon, egui::Stroke::new(1.0, encre));
        // Un souffle de couleur à l'intérieur, à peine : assez pour
        // qu'on voie une boule plutôt qu'un rond, pas assez pour
        // couvrir une lettre.
        peintre.circle_filled(
            boule.place,
            boule.rayon - 0.5,
            boule.encre.gamma_multiply(0.07 * boule.éclat),
        );
        // La calotte et le crochet, dessinés à la taille de la boule :
        // sans eux, c'est un rond ; avec eux, on voit ce que c'est.
        let haut = boule.place.y - boule.rayon;
        peintre.rect_filled(
            Rect::from_min_size(
                Pos2::new(boule.place.x - boule.rayon * 0.3, haut - boule.rayon * 0.35),
                egui::vec2(boule.rayon * 0.6, boule.rayon * 0.35),
            ),
            1,
            attache,
        );
        peintre.line_segment(
            [
                Pos2::new(boule.place.x, haut - boule.rayon * 0.35),
                Pos2::new(boule.place.x, haut - boule.rayon * 1.1),
            ],
            egui::Stroke::new(1.0, attache),
        );
    }

    // L'IMAGE SUIVANTE EST DEMANDÉE ICI, et nulle part ailleurs : c'est
    // la seule dépense de l'animation, et elle s'arrête d'elle-même dès
    // que le thème change ou que le réglage se décoche, puisque
    // personne n'appelle plus cette fonction.
    ctx.request_repaint_after(PAS);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cadre() -> Rect {
        Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 1000.0))
    }

    /// LA MÊME SECONDE DONNE LA MÊME IMAGE.
    ///
    /// C'est ce qui permet au banc de rendre une scène deux fois et de
    /// comparer les images. Une neige tirée au sort à chaque appel
    /// ferait échouer `aucune_capture_ne_depend_de_celle_d_avant` —
    /// ou, pire, le ferait échouer un jour sur dix.
    #[test]
    fn la_meme_seconde_donne_la_meme_neige() {
        let theme = &namsdec_theme::NOEL;
        for t in [0.0, 0.5, 3.0, 61.25] {
            assert_eq!(flocons(cadre(), t, theme), flocons(cadre(), t, theme));
            assert_eq!(boules(cadre(), t, theme), boules(cadre(), t, theme));
        }
        // Et deux instants différents ne donnent pas la même : une
        // neige figée ne serait pas une animation.
        assert_ne!(flocons(cadre(), 0.0, theme), flocons(cadre(), 1.0, theme));
    }

    /// Les flocons restent dans la fenêtre, et discrets.
    ///
    /// La dérive latérale peut sortir un flocon par le bord — egui le
    /// découpe —, mais la chute, elle, ne doit jamais le laisser
    /// loin au-dessus ou loin en dessous : un flocon à mille pixels
    /// sous la fenêtre est un flocon qu'on paie sans le voir.
    #[test]
    fn les_flocons_tiennent_dans_la_fenetre_et_restent_pales() {
        let theme = &namsdec_theme::NOEL;
        for pas in 0..400 {
            let t = pas as f32 * 0.25;
            let tous = flocons(cadre(), t, theme);
            assert_eq!(tous.len(), FLOCONS);
            for flocon in tous {
                assert!(
                    flocon.place.y >= cadre().top() - 13.0
                        && flocon.place.y <= cadre().bottom() + 13.0,
                    "à t={t}, un flocon est en {:?}",
                    flocon.place
                );
                assert!(
                    (1.0..=2.2).contains(&flocon.rayon),
                    "rayon {}",
                    flocon.rayon
                );
                // L'encre doit rester une suggestion : au-delà d'un
                // quart d'opacité, elle se lit comme un caractère posé
                // sur le code.
                assert!(
                    flocon.encre.a() <= 64,
                    "un flocon trop opaque : {}",
                    flocon.encre.a()
                );
            }
        }
    }

    /// Les boules sont rares, jamais au milieu, et s'effacent.
    #[test]
    fn les_boules_sont_rares_et_sur_les_cotes() {
        let theme = &namsdec_theme::NOEL;
        let mut vues = 0;
        let mut images = 0;
        for pas in 0..2000 {
            let t = pas as f32 * 0.25; // cinq cents secondes
            images += 1;
            for boule in boules(cadre(), t, theme) {
                vues += 1;
                let x = (boule.place.x - cadre().left()) / cadre().width();
                assert!(
                    !(0.2..=0.8).contains(&x),
                    "une boule au milieu de l'éditeur, à t={t} : x={x}"
                );
                assert!((0.0..=1.0).contains(&boule.éclat), "éclat {}", boule.éclat);
            }
        }
        // Rare : moins d'une image sur deux en porte une. Avec trois
        // emplacements de sept secondes sur des périodes de 23, 31 et
        // 41, l'espérance est d'environ 0,55 boule par image.
        assert!(
            vues < images,
            "{vues} boules sur {images} images : ce n'est plus discret"
        );
        assert!(vues > 0, "aucune boule en cinq cents secondes");
    }

    /// Une fenêtre qui n'a pas encore de taille ne fait rien peindre.
    ///
    /// C'est l'état de la première image, et un `% 0.0` y rendrait des
    /// positions `NaN` — qu'egui peint comme il peut.
    #[test]
    fn une_fenetre_sans_taille_ne_fait_rien_peindre() {
        let vide = Rect::from_min_size(Pos2::ZERO, egui::Vec2::ZERO);
        assert!(flocons(vide, 1.0, &namsdec_theme::NOEL).is_empty());
        assert!(boules(vide, 1.0, &namsdec_theme::NOEL).is_empty());
    }
}
