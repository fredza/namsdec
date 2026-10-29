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

use crate::vue::{Bloc, Fragment, Ligne, Ton, Vue};
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
