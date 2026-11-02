//! La vue d'un panneau, peinte en egui.
//!
//! Le seul fichier qui connaisse les deux côtés de la frontière : ce
//! qu'un module a à montrer ([`crate::vue`], qui ignore egui) et la
//! façon dont namsdec le peint aujourd'hui.
//!
//! Le découpage vaut par ce qu'il rend possible, et aussi par ce qu'il
//! empêche : un module qui décrit sa vue ne peut plus décider d'une
//! couleur, d'un espacement ou d'une largeur. Ces choix-là reviennent à
//! l'interface, qui est seule à savoir ce qu'il y a autour — et c'est
//! d'ailleurs ce que vingt entrées d'`AMELIORATIONS.txt` racontent, à
//! chaque fois qu'un panneau a volé la place de son voisin.
//!
//! Quand les vingt modules seront passés par ici, `namsdec-re` n'aura
//! plus besoin d'egui du tout et ce fichier déménagera dans
//! `namsdec-gui`. C'est la fin visée, pas une promesse pour demain.

use crate::entropy::{Verdict, seuils};
use crate::entropy_module::{octets, repères};
use crate::vue::{Bloc, Dessin, Fragment, Ligne, Ton, Vue};
use crate::{ReContext, ReRequest};

/// Peint la vue, et rend ce que l'utilisateur a demandé en route.
///
/// La demande est rendue **complète** — l'adresse et ce qu'on veut en
/// voir —, et non la seule cible : un module n'a plus qu'à la poser
/// dans son champ de demande. Rendre `GoTarget` seul obligeait chaque
/// module à retrouver de quelle adresse il s'agissait, ce qui est
/// précisément ce que le peintre vient d'apprendre en cliquant.
///
/// Une seule demande rendue, la dernière cliquée : on ne clique pas
/// deux adresses dans la même image.
pub fn peindre(ui: &mut egui::Ui, ctx: &ReContext, vue: &Vue) -> Option<ReRequest> {
    peindre_tout(ui, ctx, vue).demande
}

/// Ce que l'utilisateur a fait pendant qu'on peignait.
#[derive(Debug, Default)]
pub struct Gestes {
    /// Une adresse qu'il veut aller voir.
    pub demande: Option<ReRequest>,
    /// Les boutons pressés, par le nom que le module leur a donné.
    ///
    /// Un `Vec` et non un seul : rien n'interdit à un panneau d'en
    /// avoir deux pressés dans la même image, et laisser tomber le
    /// second serait une perte silencieuse.
    pub actions: Vec<String>,
}

/// La même peinture, pour un panneau qui a des boutons.
///
/// Deux fonctions et non une, parce que la plupart des panneaux n'ont
/// rien à presser : leur imposer de lire une liste vide à chaque image
/// serait du bruit dans cinq modules sur six.
pub fn peindre_tout(ui: &mut egui::Ui, ctx: &ReContext, vue: &Vue) -> Gestes {
    // L'ESPACE ENTRE DEUX BLOCS EST DÉCIDÉ ICI, et c'est voulu : un
    // module qui écrirait `add_space(4.0)` déciderait d'un pixel, ce qui
    // est le métier de l'interface et non le sien. Mais il en faut un —
    // sans lui, un paragraphe d'explication se colle au tableau qu'il
    // explique, et la capture le montre tout de suite.
    let mut gestes = Gestes::default();
    peindre_suite(ui, ctx, &vue.blocs, &mut gestes);
    gestes
}

/// Ce qui sépare deux blocs d'un panneau.
///
/// Quatre pixels : la valeur que les modules écrivaient à la main avant
/// P7, relevée sur `digest_module` et ses voisins. Un seul endroit la
/// tient désormais, et un thème plus aéré ne demandera qu'un chiffre.
const ESPACE_ENTRE_BLOCS: f32 = 4.0;

/// Une suite de blocs, au même rythme que la vue entière.
fn peindre_suite(ui: &mut egui::Ui, ctx: &ReContext, blocs: &[Bloc], gestes: &mut Gestes) {
    for (rang, bloc) in blocs.iter().enumerate() {
        if rang > 0 {
            ui.add_space(ESPACE_ENTRE_BLOCS);
        }
        peindre_bloc(ui, ctx, bloc, gestes);
    }
}

/// Le premier texte qu'on trouve dans une suite de blocs, pour en faire
/// un identifiant stable.
fn premier_texte(blocs: &[Bloc]) -> String {
    blocs
        .iter()
        .find_map(|bloc| match bloc {
            Bloc::Texte(ligne) | Bloc::Attente(ligne) => Some(ligne.texte_nu()),
            Bloc::Section { titre, .. } | Bloc::Repliable { titre, .. } => Some(titre.texte_nu()),
            Bloc::Jauge { texte, .. } => Some(texte.texte_nu()),
            Bloc::Tableau { rangées, .. } => {
                rangées.first().and_then(|r| r.first()).map(Ligne::texte_nu)
            }
            Bloc::Retrait(dedans) => Some(premier_texte(dedans)),
            Bloc::Code(texte) => Some(texte.clone()),
            Bloc::Dessin(_) => None,
            Bloc::Trait => None,
        })
        .unwrap_or_default()
}

fn peindre_bloc(ui: &mut egui::Ui, ctx: &ReContext, bloc: &Bloc, gestes: &mut Gestes) {
    match bloc {
        Bloc::Texte(ligne) => peindre_ligne(ui, ctx, ligne, gestes),
        Bloc::Trait => {
            ui.separator();
        }
        Bloc::Attente(ligne) => {
            ui.horizontal(|ui| {
                ui.spinner();
                peindre_ligne(ui, ctx, ligne, gestes);
            });
        }
        Bloc::Jauge { part, texte } => {
            ui.add(
                egui::ProgressBar::new(part.clamp(0.0, 1.0))
                    .text(texte.texte_nu())
                    .desired_width(ui.available_width().min(320.0)),
            );
        }
        Bloc::Section { titre, blocs } => {
            ui.add_space(ESPACE_ENTRE_BLOCS);
            // Le titre d'une section est fort, quoi que le module en
            // dise : c'est la place qui le rend fort, pas le ton.
            ui.strong(titre.texte_nu());
            ui.add_space(2.0);
            // En retrait, pour qu'on voie d'un coup d'œil où une
            // section commence et finit — un intertitre seul se perd
            // dans une suite de lignes.
            let id = ("vue_section", titre.texte_nu());
            ui.indent(id, |ui| {
                peindre_suite(ui, ctx, blocs, gestes);
            });
        }
        Bloc::Dessin(Dessin::Profil {
            valeurs,
            taille_tranche,
            total,
        }) => {
            if let Some((position, cible)) = profil(ui, ctx, valeurs, *taille_tranche, *total) {
                // Une POSITION DU FICHIER, et non une adresse : seul
                // le module sait ce que cette position devient en
                // mémoire — il en tient la table, relevée une fois au
                // chargement. L'action la lui rend telle quelle.
                gestes.actions.push(format!(
                    "{PROFIL_ALLER}{position}:{}",
                    match cible {
                        crate::GoTarget::Listing => "listing",
                        crate::GoTarget::Bytes => "octets",
                    }
                ));
            }
            ui.add_space(ESPACE_ENTRE_BLOCS);
            échelle(ui, ctx);
        }
        Bloc::Code(texte) => {
            // Par le rendu du pseudo-code du projet : il colore les
            // mots-clés et fait des adresses des liens vers le listing.
            // Un module qui décrit sa vue n'a pas à redire tout cela —
            // il dit « ceci est du code ».
            if let Some(adresse) =
                crate::dzf_decompiler_module::pseudocode_cliquable(ui, texte, f32::INFINITY)
            {
                // Vers le CODE, sans demander : une adresse du
                // pseudo-code y désigne un appel ou un saut, et le
                // choix ne se pose que pour une adresse dont on ignore
                // ce qu'elle porte.
                gestes.demande = Some(ReRequest::GoTo {
                    addr: adresse,
                    target: crate::GoTarget::Listing,
                });
            }
        }
        Bloc::Retrait(blocs) => {
            // L'identifiant vient du contenu : deux retraits voisins
            // doivent pouvoir vivre côte à côte.
            let id = ("vue_retrait", premier_texte(blocs));
            ui.indent(id, |ui| {
                peindre_suite(ui, ctx, blocs, gestes);
            });
        }
        Bloc::Repliable {
            titre,
            ouvert,
            blocs,
        } => {
            egui::CollapsingHeader::new(titre.texte_nu())
                .id_salt(titre.texte_nu())
                .default_open(*ouvert)
                .show(ui, |ui| {
                    peindre_suite(ui, ctx, blocs, gestes);
                });
        }
        Bloc::Tableau {
            colonnes,
            rangées,
            rayé,
        } => {
            let colonnes_max = rangées
                .iter()
                .map(Vec::len)
                .chain(std::iter::once(colonnes.len()))
                .max()
                .unwrap_or(0)
                .max(1);
            // L'identifiant vient du contenu de l'en-tête, ou du premier
            // texte : deux grilles d'un même panneau ne doivent pas
            // partager leur état de colonnes.
            let id = colonnes
                .iter()
                .map(Ligne::texte_nu)
                .collect::<Vec<_>>()
                .join("|");
            let id = if id.is_empty() {
                rangées
                    .first()
                    .and_then(|r| r.first())
                    .map(Ligne::texte_nu)
                    .unwrap_or_default()
            } else {
                id
            };
            egui::Grid::new(("vue_tableau", id))
                .num_columns(colonnes_max)
                .min_col_width(0.0)
                // L'AIR ENTRE LES COLONNES, décidé ici pour tous les
                // tableaux. `map_module` l'écrivait lui-même — quatorze
                // pixels — et ses colonnes se touchaient dès qu'on le
                // lui retirait : « 0x0 + 0x1031 0x3ff000 » se lit mal
                // quand rien ne sépare les nombres. Les autres panneaux
                // s'en remettaient au défaut d'egui, plus serré.
                // Un seul rythme vaut mieux que deux, et c'est le plus
                // lisible des deux qu'on garde.
                .spacing([14.0, 3.0])
                .striped(*rayé)
                .show(ui, |ui| {
                    if !colonnes.is_empty() {
                        // AU TON QUE LE MODULE A DIT, et non en gras
                        // d'office : `map` veut des en-têtes discrets
                        // au-dessus de sa carte, `slack` les veut
                        // francs au-dessus de sa liste. Imposer ici le
                        // gras aurait fait de la frontière un endroit
                        // qui décide à la place des modules — soit
                        // exactement ce qu'on lui reproche quand elle
                        // est trop riche.
                        for titre in colonnes {
                            peindre_ligne(ui, ctx, titre, gestes);
                        }
                        ui.end_row();
                    }
                    for rangée in rangées {
                        for cellule in rangée {
                            peindre_ligne(ui, ctx, cellule, gestes);
                        }
                        ui.end_row();
                    }
                });
        }
    }
}

/// Une ligne, fragment par fragment, sur une seule rangée qui se replie.
fn peindre_ligne(ui: &mut egui::Ui, ctx: &ReContext, ligne: &Ligne, gestes: &mut Gestes) {
    // UN SEUL FRAGMENT : pas de rangée. C'est le cas de la grande
    // majorité des lignes, et ce n'est pas qu'une économie — une rangée
    // repliable laisse son contenu prendre la largeur disponible, et le
    // trait que `clickable_label` peint sous un texte cliquable
    // s'étirait alors bien au-delà du mot. Vu sur la capture du panneau
    // Empreintes, en comparant avec l'avant.
    if let [seul] = ligne.fragments.as_slice() {
        peindre_fragment(ui, ctx, seul, gestes);
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for fragment in &ligne.fragments {
            peindre_fragment(ui, ctx, fragment, gestes);
        }
    });
}

fn peindre_fragment(ui: &mut egui::Ui, ctx: &ReContext, fragment: &Fragment, gestes: &mut Gestes) {
    if let Some(quoi) = &fragment.action {
        // Par le bouton encadré du projet : il se voit SANS qu'on le
        // survole, ce que le thème plat ne donne pas tout seul.
        let bouton = ui.add_enabled(!fragment.inactif, crate::framed(ui, fragment.texte.clone()));
        let bouton = match &fragment.survol {
            // Au survol ET quand il est grisé : un bouton éteint qui ne
            // dit pas pourquoi fait chercher la raison ailleurs.
            Some(texte) => bouton.on_hover_text(texte).on_disabled_hover_text(texte),
            None => bouton,
        };
        if bouton.clicked() {
            gestes.actions.push(quoi.clone());
        }
        return;
    }
    if let Some(adresse) = fragment.mène_à {
        // Par l'aide du projet : elle porte le nom que l'utilisateur a
        // donné à l'adresse, et le menu « aller voir » qui va avec. Un
        // module n'a pas à redire tout cela.
        if let Some(cible) = crate::clickable_address(ui, ctx, adresse) {
            gestes.demande = Some(ReRequest::GoTo {
                addr: adresse,
                target: cible,
            });
        }
        return;
    }
    if fragment.copiable {
        let réponse = crate::clickable_label(ui, egui::Label::new(riche(ui, fragment)));
        let réponse = match &fragment.survol {
            Some(texte) => réponse.on_hover_text(texte),
            None => réponse.on_hover_text(ctx.t("Cliquer pour copier", "Click to copy")),
        };
        if réponse.clicked() {
            ui.ctx().copy_text(fragment.texte.clone());
        }
        return;
    }
    let réponse = étiquette(ui, fragment);
    if let Some(texte) = &fragment.survol {
        réponse.on_hover_text(texte);
    }
}

fn étiquette(ui: &mut egui::Ui, fragment: &Fragment) -> egui::Response {
    let texte = riche(ui, fragment);
    ui.label(texte)
}

/// Le ton, traduit dans les couleurs du thème en vigueur.
///
/// C'est ici, et nulle part ailleurs, qu'un verdict devient une
/// couleur : un module dit « mauvais », l'interface dit laquelle de ses
/// teintes porte ce mot-là. Un thème peut donc changer sans qu'aucun
/// module bouge.
fn riche(ui: &egui::Ui, fragment: &Fragment) -> egui::RichText {
    let texte = egui::RichText::new(&fragment.texte);
    let visuals = ui.visuals();
    // UNE CATÉGORIE L'EMPORTE SUR LE TON : quand un module range un
    // fragment dans une famille de comportement — « réseau »,
    // « crypto » —, c'est cette famille qu'on vient lire, et c'est elle
    // qui donne la couleur. La table vit ici, et non dans le module :
    // celui-ci n'a dit qu'un nom.
    if let Some(clé) = &fragment.étiquette
        && let Some(famille) = crate::behaviour::Family::de_clé(clé)
    {
        return texte.color(famille.color());
    }
    match fragment.ton {
        Ton::Normal => texte,
        Ton::Faible => texte.color(visuals.weak_text_color()),
        Ton::Fort => texte.color(visuals.strong_text_color()),
        Ton::Code => texte.monospace(),
        // Deux des trois verdicts suivent le thème, parce qu'egui porte
        // leurs couleurs et que `namsdec-theme` les y installe —
        // `visuals.warn_fg_color` et `visuals.error_fg_color` sont déjà
        // lus par une dizaine d'endroits de ce crate.
        Ton::Attention => texte.color(visuals.warn_fg_color),
        Ton::Mauvais => texte.color(visuals.error_fg_color),
        // Le troisième n'a pas d'équivalent : egui ne connaît pas la
        // réussite. D'où cette constante, qui est le vert employé
        // partout ailleurs dans namsdec — celui des pastilles de
        // protection. Le jour où le thème sera accessible d'ici, elle
        // s'en ira ; en attendant, mieux vaut un vert qui s'accorde au
        // reste qu'une teinte inventée sur place.
        Ton::Bon => texte.color(VERT),
        Ton::Repère => texte.color(visuals.hyperlink_color),
    }
}

/// Le vert des verdicts favorables. Voir `riche` pour le pourquoi.
const VERT: egui::Color32 = egui::Color32::from_rgb(0x4e, 0xc9, 0xb0);

/// L'action qu'un clic dans le profil engendre : la position du
/// fichier, puis où l'on veut aller. Le module la décode — lui seul
/// sait traduire une position en adresse.
pub const PROFIL_ALLER: &str = "profil:aller:";

// --- Le profil d'entropie, et son échelle -----------------------------
//
// Deux cent soixante lignes de dessin, qui vivaient dans
// `entropy_module`. Elles décident « une colonne par pixel, et la plus
// haute des tranches qu'elle recouvre » — une décision qui n'a de sens
// qu'une fois la largeur connue, c'est-à-dire ici. Le module, lui, n'a
// plus qu'une chose à dire : une valeur par tranche.

/// La hauteur du profil : ample, mais jamais plus que la moitié de la
/// fenêtre.
///
/// Trente-six pixels de haut sur cinq cents de large écrasaient tout :
/// entre 5,8 et 6,6 — l'écart qui sépare du code d'un bloc suspect —,
/// il y avait trois pixels, et l'on ne voyait rien. Il faut de la
/// hauteur pour lire une courbe.
///
/// La borne est celle que l'usage a demandée : **jamais plus de la
/// moitié du viewport**. Un dessin qui prend tout l'écran n'est plus un
/// indice parmi d'autres, c'est le panneau tout entier — et il reste
/// quatre rubriques à lire dessous.
pub(crate) fn hauteur_du_profil(viewport: f32) -> f32 {
    const VOULUE: f32 = 240.0;
    const MINIMUM: f32 = 60.0;
    VOULUE.min(viewport * 0.5).max(MINIMUM.min(viewport * 0.5))
}

/// La couleur d'une tranche selon son entropie, en dégradé continu.
///
/// Du vert (régulier, inoffensif) au violet (maximal — chiffré ou
/// compressé), en passant par le vert-jaune, le jaune, l'orangé,
/// l'orange, le rouge et le magenta. **Neuf arrêts et non cinq** : avec
/// cinq, tout l'intervalle du code ordinaire — 5,5 à 6,5, celui où se
/// joue la lecture d'un vrai binaire — tenait dans une seule teinte, et
/// deux blocs qu'un demi-bit sépare paraissaient identiques.
///
/// Les arrêts sont posés là où le sens bascule : le texte vers 4-5, le
/// code dense vers 6, le soupçon d'empaquetage vers 7, l'aléa pur à 8.
///
/// Couleurs fixes, et non tirées du thème : c'est une échelle de mesure,
/// elle doit dire la même chose d'un thème à l'autre.
pub(crate) fn entropy_color(entropy: f64) -> egui::Color32 {
    const ARRÊTS: [(f32, egui::Color32); 9] = [
        (0.0, egui::Color32::from_rgb(0x3f, 0xb9, 0x50)), // vert
        (3.0, egui::Color32::from_rgb(0x8b, 0xc3, 0x4a)), // vert-jaune
        (4.0, egui::Color32::from_rgb(0xd2, 0xa8, 0x06)), // jaune
        (5.0, egui::Color32::from_rgb(0xdd, 0x8a, 0x0c)), // jaune orangé
        (6.0, egui::Color32::from_rgb(0xe0, 0x7b, 0x1f)), // orange
        (6.5, egui::Color32::from_rgb(0xec, 0x63, 0x34)), // orange rouge
        (7.0, egui::Color32::from_rgb(0xf8, 0x51, 0x49)), // rouge
        (7.5, egui::Color32::from_rgb(0xd5, 0x5a, 0xb0)), // magenta
        (8.0, egui::Color32::from_rgb(0xa3, 0x71, 0xf7)), // violet
    ];
    let e = entropy as f32;
    for paire in ARRÊTS.windows(2) {
        let (v0, c0) = paire[0];
        let (v1, c1) = paire[1];
        if e <= v1 {
            let t = ((e - v0) / (v1 - v0)).clamp(0.0, 1.0);
            let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
            return egui::Color32::from_rgb(
                m(c0.r(), c1.r()),
                m(c0.g(), c1.g()),
                m(c0.b(), c1.b()),
            );
        }
    }
    ARRÊTS[ARRÊTS.len() - 1].1
}

/// Le profil : une colonne par tranche, du bas vers le haut, l'échelle
/// allant de 0 à 8 — **toujours**, et non de la plus basse à la plus
/// haute valeur vue : une échelle qui s'ajuste ferait paraître
/// spectaculaire l'écart entre 5,9 et 6,1, et l'oeil y verrait une
/// bosse qui n'existe pas.
///
/// Rend la position du fichier sur laquelle on a cliqué, s'il y en a
/// une : le dessin désigne des endroits, et la seule chose qu'on veuille
/// en faire est d'aller voir.
fn profil(
    ui: &mut egui::Ui,
    ctx: &ReContext,
    valeurs: &[f64],
    taille_tranche: usize,
    total: usize,
) -> Option<(usize, crate::GoTarget)> {
    let hauteur = hauteur_du_profil(ui.ctx().viewport_rect().height());
    // Pleine largeur : c'est une carte du fichier, et une carte se lit
    // d'autant mieux qu'elle est large. La borne de 520 pixels d'avant
    // tassait trois cents tranches dans un timbre-poste.
    let largeur = ui.available_width().max(120.0);
    let (rect, réponse) = ui.allocate_exact_size(
        egui::vec2(largeur, hauteur),
        egui::Sense::click().union(egui::Sense::hover()),
    );
    let peintre = ui.painter_at(rect);
    let visuels = ui.visuals().clone();
    peintre.rect_filled(rect, 2.0, visuels.extreme_bg_color);

    let y_de = |bits: f64| rect.bottom() - (bits / 8.0).clamp(0.0, 1.0) as f32 * hauteur;

    // Les lignes de niveau, graduées en bits par octet : sans elles,
    // les hauteurs ne se comparent qu'entre elles, et l'on ne sait pas
    // si une barre est haute dans l'absolu.
    let police = egui::TextStyle::Small.resolve(ui.style());
    for bits in [2.0, 4.0, 6.0] {
        let y = y_de(bits);
        peintre.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, visuels.weak_text_color().gamma_multiply(0.35)),
        );
    }

    // Une colonne par pixel, et la PLUS HAUTE des tranches qu'elle
    // recouvre : sur un fichier qui porte plus de tranches que le
    // dessin n'a de pixels, prendre la moyenne effacerait exactement ce
    // qu'on cherche — le petit bloc dense au milieu du calme.
    let colonnes = largeur.floor().max(1.0) as usize;
    let n = valeurs.len();
    for colonne in 0..colonnes {
        let début = colonne * n / colonnes;
        let fin = (((colonne + 1) * n).div_ceil(colonnes))
            .min(n)
            .max(début + 1);
        let Some(haute) = valeurs.get(début..fin).and_then(|tranche| {
            tranche.iter().copied().fold(None, |acc: Option<f64>, e| {
                Some(acc.map_or(e, |a: f64| a.max(e)))
            })
        }) else {
            continue;
        };
        let x = rect.left() + colonne as f32;
        let barre = egui::Rect::from_min_max(
            egui::pos2(x, y_de(haute)),
            egui::pos2(x + 1.0, rect.bottom()),
        );
        // La couleur suit l'entropie en dégradé continu : on lit la
        // montée sans avoir à comparer des hauteurs entre elles, et
        // l'extrême — le chiffré, le compressé — se détache en violet.
        peintre.rect_filled(barre, 0.0, entropy_color(haute));
    }

    // Les graduations APRÈS les barres : peintes avant, le « 2 » et le
    // « 4 » disparaissaient sous la courbe dès que celle-ci montait, et
    // l'échelle ne se lisait plus qu'à moitié.
    for bits in [2.0, 4.0, 6.0] {
        let y = y_de(bits);
        let chiffre = format!("{bits:.0}");
        let place =
            peintre.layout_no_wrap(chiffre.clone(), police.clone(), visuels.weak_text_color());
        let fond = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, y - place.size().y - 1.0),
            place.size() + egui::vec2(4.0, 2.0),
        );
        peintre.rect_filled(fond, 2.0, visuels.extreme_bg_color.gamma_multiply(0.8));
        peintre.text(
            egui::pos2(rect.left() + 4.0, y - 1.0),
            egui::Align2::LEFT_BOTTOM,
            chiffre,
            police.clone(),
            visuels.weak_text_color(),
        );
    }

    // Le repère d'empaquetage : la ligne au-dessus de laquelle il n'y a
    // plus de structure à voir. La valeur vient de `entropy::seuils`,
    // comme celle qui décide des couleurs.
    let seuil = y_de(seuils::EMPAQUETÉ);
    peintre.line_segment(
        [
            egui::pos2(rect.left(), seuil),
            egui::pos2(rect.right(), seuil),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(0xf8, 0x51, 0x49)),
    );
    peintre.text(
        egui::pos2(rect.right() - 4.0, seuil - 2.0),
        egui::Align2::RIGHT_BOTTOM,
        format!(
            "{} {}",
            format!("{:.1}", seuils::EMPAQUETÉ).replace('.', ","),
            ctx.t("empaqueté", "packed")
        ),
        police.clone(),
        egui::Color32::from_rgb(0xf8, 0x51, 0x49),
    );

    // LA RÈGLE DES ADRESSES, sous le dessin : sans elle, une bosse « au
    // tiers du fichier » ne se retrouve nulle part. Les positions sont
    // celles du FICHIER — c'est lui qu'on mesure —, et la table des
    // plages donne l'adresse en mémoire en regard.
    let (règle, _) = ui.allocate_exact_size(egui::vec2(largeur, 16.0), egui::Sense::hover());
    let peintre = ui.painter_at(règle.expand(2.0));
    let combien = repères(largeur);
    for repère in 0..=combien {
        let part = repère as f32 / combien as f32;
        let x = rect.left() + part * largeur;
        peintre.line_segment(
            [egui::pos2(x, règle.top()), egui::pos2(x, règle.top() + 4.0)],
            egui::Stroke::new(1.0, visuels.weak_text_color()),
        );
        let position = (total as f64 * part as f64) as usize;
        let ancrage = if repère == 0 {
            egui::Align2::LEFT_TOP
        } else if repère == combien {
            egui::Align2::RIGHT_TOP
        } else {
            egui::Align2::CENTER_TOP
        };
        peintre.text(
            egui::pos2(x, règle.top() + 5.0),
            ancrage,
            format!("{position:#x}"),
            police.clone(),
            visuels.weak_text_color(),
        );
    }

    // Au survol, la tranche visée se dit : sa plage, sa mesure, son
    // verdict. Et ce qu'un clic ferait — personne ne clique sur ce qui
    // n'a pas l'air cliquable.
    let mut cliqué = None;
    if let Some(position) = réponse.hover_pos() {
        let part = ((position.x - rect.left()) / largeur).clamp(0.0, 0.999_9);
        let index = ((part * n as f32) as usize).min(n.saturating_sub(1));
        if let Some(valeur) = valeurs.get(index).copied() {
            // La position vient du RANG : une tranche fait toujours la
            // même taille, et la dernière n'est plus courte que du
            // reste. Le module donnait ces offsets ; les redonner
            // aurait été dire deux fois la même chose.
            let position = index * taille_tranche;
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            réponse.clone().on_hover_text(format!(
                "{:#x}..{:#x}  ({})\n{:.2} — {}\n{}",
                position,
                (position + taille_tranche).min(total),
                octets(taille_tranche),
                valeur,
                Verdict::of(valeur).label(ctx.lang.starts_with("fr")),
                ctx.t(
                    "Cliquer : aller voir cette tranche — dans le code, ou dans les octets",
                    "Click: go and see this slice — in the code, or in the bytes"
                ),
            ));
            if réponse.clicked() {
                cliqué = Some(position);
            }
        }
    }
    // LA TRANCHE VISÉE EST RETENUE, et le menu s'ouvre à l'image
    // suivante : entre le clic et le choix, la souris a quitté la
    // tranche pour aller sur l'entrée du menu, et la relire sous le
    // pointeur mènerait ailleurs qu'où l'on a cliqué.
    let visée = ui.id().with("tranche_visée");
    if let Some(offset) = cliqué {
        ui.data_mut(|d| d.insert_temp(visée, offset));
    }
    let mut choix = None;
    egui::Popup::menu(&réponse).show(|ui| {
        if crate::framed_button(ui, ctx.t("Dans le code désassemblé", "In the disassembly"))
            .clicked()
        {
            choix = Some(crate::GoTarget::Listing);
            ui.close();
        }
        if crate::framed_button(ui, ctx.t("Dans l'éditeur binaire", "In the binary editor"))
            .clicked()
        {
            choix = Some(crate::GoTarget::Bytes);
            ui.close();
        }
    });
    let cible = choix?;
    let offset = ui.data(|d| d.get_temp::<usize>(visée))?;
    Some((offset, cible))
}

/// L'échelle de couleurs, graduée.
///
/// Une rampe sans graduation ne se lit pas : on voit « du rouge » sans
/// savoir si c'est 6,9 ou 7,8 — or c'est exactement ce qui sépare du
/// code dense d'un bloc chiffré. La barre porte donc ses huit
/// graduations, et les cinq verdicts sont écrits sous la zone qui leur
/// correspond.
fn échelle(ui: &mut egui::Ui, ctx: &ReContext) {
    let fr = ctx.lang.starts_with("fr");
    let largeur = ui.available_width().clamp(160.0, 560.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(largeur, 12.0), egui::Sense::hover());
    let peintre = ui.painter_at(rect);
    // Un pixel par colonne : la rampe est continue, et c'est ce qu'elle
    // doit montrer.
    let colonnes = largeur.floor().max(1.0) as usize;
    for colonne in 0..colonnes {
        let bits = 8.0 * colonne as f64 / colonnes as f64;
        let x = rect.left() + colonne as f32;
        peintre.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x, rect.top()),
                egui::pos2(x + 1.0, rect.bottom()),
            ),
            0.0,
            entropy_color(bits),
        );
    }
    // Les graduations : zéro à huit bits par octet.
    let police = egui::TextStyle::Small.resolve(ui.style());
    let (règle, _) = ui.allocate_exact_size(egui::vec2(largeur, 14.0), egui::Sense::hover());
    let peintre = ui.painter_at(règle.expand(2.0));
    let couleur = ui.visuals().weak_text_color();
    for bits in 0..=8 {
        let part = bits as f32 / 8.0;
        let x = rect.left() + part * largeur;
        peintre.line_segment(
            [egui::pos2(x, règle.top()), egui::pos2(x, règle.top() + 3.0)],
            egui::Stroke::new(1.0, couleur),
        );
        let ancrage = match bits {
            0 => egui::Align2::LEFT_TOP,
            8 => egui::Align2::RIGHT_TOP,
            _ => egui::Align2::CENTER_TOP,
        };
        peintre.text(
            egui::pos2(x, règle.top() + 4.0),
            ancrage,
            format!("{bits}"),
            police.clone(),
            couleur,
        );
    }
    // Et ce que chaque zone veut dire, dans sa couleur.
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.weak(ctx.t("bits par octet :", "bits per byte:"));
        for (milieu, verdict) in [
            (0.75, Verdict::Flat),
            (3.25, Verdict::Text),
            (5.9, Verdict::Code),
            (7.0, Verdict::Mixed),
            (7.9, Verdict::Random),
        ] {
            ui.label(egui::RichText::new(verdict.label(fr)).color(entropy_color(milieu)));
            ui.weak("·");
        }
    });
}
