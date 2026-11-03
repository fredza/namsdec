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
use crate::vue::{Bloc, Champ, Dessin, Fragment, Ligne, Nœud, Ton, Vue};
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
            Bloc::Verdict { nom, .. } => Some(nom.texte_nu()),
            Bloc::Champ(champ) => Some(champ.action.clone()),
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
            // UN TRAIT AU-DESSUS DU TITRE, et c'est l'interface qui en
            // décide pour tout le monde. Trois des cinq sections
            // titrées écrites à la main dans le crate en traçaient un
            // (« Ce qui l'a produit », « Ce qu'il sait faire »,
            // « Protections », dans « En bref »), deux non
            // (« Bibliothèques réclamées », « Segments », dans
            // Protections) : la même forme se peignait de deux façons
            // selon le module qui l'écrivait. Mesuré sur la capture :
            // sans le trait, trois sections d'affilée se lisent comme
            // une seule liste à intertitres, et l'on cherche où
            // commence la suivante.
            ui.separator();
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
            // BORNÉ À LA LARGEUR DU PANNEAU, et c'est l'interface qui
            // le décide : une ligne d'`objdump` fait parfois deux
            // cents caractères, et peinte sans borne elle court hors
            // du panneau — où rien ne la rattrape, puisque la fenêtre
            // d'analyse ne défile pas en largeur. Le panneau des
            // outils l'écrivait lui-même avant P7 ; celui de la
            // décompilation ne l'écrivait nulle part, et débordait
            // donc sans que personne le mesure.
            let largeur = ui.available_width().max(80.0);
            if let Some(adresse) =
                crate::dzf_decompiler_module::pseudocode_cliquable(ui, texte, largeur)
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
        Bloc::Verdict {
            nom,
            état,
            verdict,
            pourquoi,
        } => peindre_verdict(ui, ctx, nom, état, *verdict, pourquoi.as_deref(), gestes),
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
            // LE TITRE AU TON QU'IL PORTE, et non en texte nu : le
            // détail d'une fonction s'écrit en chasse fixe, et un nom
            // donné à la main se distingue par un fond léger. Peint nu,
            // le repliable perdait les deux.
            let en_tête = match titre.fragments.as_slice() {
                [seul] => riche(ui, seul),
                _ => egui::RichText::new(titre.texte_nu()),
            };
            egui::CollapsingHeader::new(en_tête)
                .id_salt(titre.texte_nu())
                .default_open(*ouvert)
                .show(ui, |ui| {
                    peindre_suite(ui, ctx, blocs, gestes);
                });
        }
        Bloc::Champ(champ) => peindre_champ(ui, ctx, champ, gestes),
        Bloc::Dessin(Dessin::Graphe { nœuds, arêtes }) => {
            if let Some(addr) = graphe(ui, nœuds, arêtes) {
                // Vers le CODE, sans demander : une boîte de ce graphe
                // est une fonction, et une fonction se lit
                // désassemblée. Le choix se pose pour une adresse
                // écrite — une chaîne, une table, un bourrage peuvent
                // être l'un ou l'autre —, pas pour celle-ci.
                gestes.demande = Some(ReRequest::GoTo {
                    addr,
                    target: crate::GoTarget::Listing,
                });
            }
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
            // CE QUI DÉBORDERAIT EST BORNÉ AVANT D'ÊTRE PEINT. Voir
            // `plafonds_des_colonnes` : dans le cas ordinaire, tout
            // vaut `None` et rien ne change.
            const AIR_ENTRE_COLONNES: f32 = 14.0;
            let plafonds =
                plafonds_des_colonnes(ui, colonnes, rangées, colonnes_max, AIR_ENTRE_COLONNES);
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
                .spacing([AIR_ENTRE_COLONNES, 3.0])
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
                        for (rang, titre) in colonnes.iter().enumerate() {
                            peindre_ligne_bornée(ui, ctx, titre, plafonds[rang], gestes);
                        }
                        ui.end_row();
                    }
                    for rangée in rangées {
                        for (rang, cellule) in rangée.iter().enumerate() {
                            peindre_ligne_bornée(ui, ctx, cellule, plafonds[rang], gestes);
                        }
                        ui.end_row();
                    }
                });
        }
    }
}

/// Jusqu'où chaque colonne a le droit d'aller, quand le tableau ne
/// tient pas dans la largeur qu'on lui donne.
///
/// `None` partout dans le cas ordinaire — et c'est voulu : tant que le
/// tableau tient, rien ne change, pas un pixel. Une table qui déborde
/// voit en revanche sa colonne LA PLUS LARGE bornée à ce qui reste ;
/// les étroites ne sont jamais touchées, parce que rogner « adresse »
/// pour faire tenir du pseudo-code ne ferait que perdre les deux.
///
/// La borne est tirée des largeurs NATURELLES, mesurées sans
/// contrainte. C'est ce qui la rend sûre : une grille garde la largeur
/// de ses colonnes d'une image sur l'autre, et une borne tirée de
/// `available_width()` serait donc tirée du dessin précédent — à la
/// deuxième image « SHA-256 » devenait « … », à la troisième il ne
/// restait rien. Vu sur la capture du panneau des empreintes, et c'est
/// le piège même d'egui : une mesure reprise en entrée de la suivante.
fn plafonds_des_colonnes(
    ui: &egui::Ui,
    colonnes: &[Ligne],
    rangées: &[Vec<Ligne>],
    combien: usize,
    espacement: f32,
) -> Vec<Option<f32>> {
    let mut naturelles = vec![0.0f32; combien];
    for rangée in std::iter::once(colonnes).chain(rangées.iter().map(Vec::as_slice)) {
        for (rang, cellule) in rangée.iter().enumerate().take(combien) {
            naturelles[rang] = naturelles[rang].max(largeur_naturelle(ui, cellule));
        }
    }
    let total: f32 = naturelles.iter().sum::<f32>() + espacement * (combien.max(1) - 1) as f32;
    let dispo = ui.available_width();
    if total <= dispo || combien == 0 {
        return vec![None; combien];
    }
    let (plus_large, _) = naturelles
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .expect("au moins une colonne");
    let autres: f32 = naturelles
        .iter()
        .enumerate()
        .filter(|(rang, _)| *rang != plus_large)
        .map(|(_, l)| *l)
        .sum();
    // Jamais moins de quarante pixels : une colonne réduite à trois
    // points ne dit plus rien, et mieux vaut alors déborder un peu que
    // de ne rien montrer du tout.
    let borne = (dispo - espacement * (combien - 1) as f32 - autres).max(40.0);
    let mut plafonds = vec![None; combien];
    plafonds[plus_large] = Some(borne);
    plafonds
}

/// La largeur qu'une ligne prendrait si rien ne la contraignait.
fn largeur_naturelle(ui: &egui::Ui, ligne: &Ligne) -> f32 {
    ligne
        .fragments
        .iter()
        .map(|fragment| {
            let galley = egui::WidgetText::from(riche(ui, fragment)).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Body,
            );
            // Un bouton encadré porte sa marge : la mesurer au texte
            // seul ferait croire qu'il tient là où il déborde.
            galley.size().x + if fragment.action.is_some() { 16.0 } else { 0.0 }
        })
        .sum()
}

/// Une ligne, fragment par fragment, sur une seule rangée qui se replie.
fn peindre_ligne(ui: &mut egui::Ui, ctx: &ReContext, ligne: &Ligne, gestes: &mut Gestes) {
    peindre_ligne_bornée(ui, ctx, ligne, None, gestes);
}

/// La même, avec une largeur à ne pas dépasser.
///
/// Le plafond ne vient que des tableaux, et il est `None` partout
/// ailleurs — voir `plafonds_des_colonnes`.
fn peindre_ligne_bornée(
    ui: &mut egui::Ui,
    ctx: &ReContext,
    ligne: &Ligne,
    plafond: Option<f32>,
    gestes: &mut Gestes,
) {
    // UN SEUL FRAGMENT : pas de rangée. C'est le cas de la grande
    // majorité des lignes, et ce n'est pas qu'une économie — une rangée
    // repliable laisse son contenu prendre la largeur disponible, et le
    // trait que `clickable_label` peint sous un texte cliquable
    // s'étirait alors bien au-delà du mot. Vu sur la capture du panneau
    // Empreintes, en comparant avec l'avant.
    if let [seul] = ligne.fragments.as_slice() {
        peindre_fragment_borné(ui, ctx, seul, plafond, gestes);
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for fragment in &ligne.fragments {
            peindre_fragment_borné(ui, ctx, fragment, plafond, gestes);
        }
    });
}

fn peindre_fragment(ui: &mut egui::Ui, ctx: &ReContext, fragment: &Fragment, gestes: &mut Gestes) {
    peindre_fragment_borné(ui, ctx, fragment, None, gestes);
}

fn peindre_fragment_borné(
    ui: &mut egui::Ui,
    ctx: &ReContext,
    fragment: &Fragment,
    plafond: Option<f32>,
    gestes: &mut Gestes,
) {
    if let Some(quoi) = &fragment.action {
        if fragment.bouton {
            // Par le bouton encadré du projet : il se voit SANS qu'on
            // le survole, ce que le thème plat ne donne pas tout seul.
            let bouton =
                ui.add_enabled(!fragment.inactif, crate::framed(ui, fragment.texte.clone()));
            let bouton = match &fragment.survol {
                // Au survol ET quand il est grisé : un bouton éteint qui
                // ne dit pas pourquoi fait chercher la raison ailleurs.
                Some(texte) => bouton.on_hover_text(texte).on_disabled_hover_text(texte),
                None => bouton,
            };
            if bouton.clicked() {
                gestes.actions.push(quoi.clone());
            }
            return;
        }
        // UN LIEN, ET NON UN BOUTON ENCADRÉ, et c'est la correction
        // d'une divergence : la vue dit depuis le début qu'une action
        // fait un lien par défaut et qu'un encadré se demande
        // (`Fragment::bouton`), mais le peintre encadrait TOUT. Les
        // noms d'`imports` étaient des textes soulignés avant P7 ; ils
        // sont devenus cinquante boutons dans une colonne, ce qui est
        // exactement ce que le vocabulaire voulait éviter.
        //
        // La couleur de lien est posée ici : le module a dit un ton —
        // « fort », « code » —, et « ceci se clique » est de la
        // présentation.
        if fragment.inactif {
            let réponse = ui.add_enabled(false, egui::Label::new(riche(ui, fragment)));
            if let Some(texte) = &fragment.survol {
                réponse.on_disabled_hover_text(texte);
            }
            return;
        }
        let couleur = ui.visuals().hyperlink_color;
        let réponse =
            crate::clickable_label(ui, egui::Label::new(riche(ui, fragment).color(couleur)));
        let réponse = match &fragment.survol {
            Some(texte) => réponse.on_hover_text(texte),
            None => réponse,
        };
        if réponse.clicked() {
            gestes.actions.push(quoi.clone());
        }
        return;
    }
    if let Some(url) = &fragment.renvoi {
        // UN RENVOI MÈNE HORS DE NAMSDEC, et c'est pourquoi il se peint
        // autrement qu'une adresse : celle-ci se clique pour aller voir
        // le listing, celui-là ouvre un navigateur. L'adresse de la
        // page se lit au survol faute d'autre indication — un lien dont
        // on ne sait pas où il mène ne se clique pas de bon cœur.
        let réponse = ui.hyperlink_to(fragment.texte.clone(), url);
        réponse.on_hover_text(fragment.survol.clone().unwrap_or_else(|| url.clone()));
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
    let réponse = étiquette(ui, fragment, plafond);
    if let Some(texte) = &fragment.survol {
        réponse.on_hover_text(texte);
    }
}

fn étiquette(ui: &mut egui::Ui, fragment: &Fragment, plafond: Option<f32>) -> egui::Response {
    let texte = riche(ui, fragment);
    // SANS PLAFOND, RIEN NE CHANGE : c'est le cas partout hors des
    // tableaux, et des tableaux qui tiennent — c'est-à-dire presque
    // partout.
    let Some(borne) = plafond else {
        return ui.label(texte);
    };
    let entière = egui::WidgetText::from(texte.clone()).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Body,
    );
    if entière.size().x <= borne + 0.5 {
        return ui.add(egui::Label::new(entière));
    }
    let coupée = egui::WidgetText::from(texte).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        borne,
        egui::TextStyle::Body,
    );
    // Ce qui manque se lit au survol : une élision sans recours est une
    // information perdue.
    ui.add(egui::Label::new(coupée))
        .on_hover_text(fragment.texte.clone())
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
        return texte.color(couleur_de_famille(famille));
    }
    // CE QUI VIENT DE LA MAIN DE L'UTILISATEUR PORTE UN FOND LÉGER, et
    // garde par-dessus le ton que le module a dit : un nom donné à la
    // main s'écrit en chasse fixe comme les autres, il se repère en
    // plus. D'où un fond posé avant le ton, et non à sa place.
    let texte = if fragment.à_la_main {
        texte.background_color(visuals.selection.bg_fill.gamma_multiply(0.25))
    } else {
        texte
    };
    // LA CHASSE D'ABORD, LE TON ENSUITE : ce sont deux axes, et un
    // fragment peut demander les deux — une signature de fonction est
    // en chasse fixe et atténuée.
    let texte = if fragment.chasse_fixe {
        texte.monospace()
    } else {
        texte
    };
    match fragment.ton {
        Ton::Normal => texte,
        Ton::Faible => texte.color(visuals.weak_text_color()),
        Ton::Fort => texte.color(visuals.strong_text_color()),
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

/// Une chose vérifiée, son état, et ce que cela change.
///
/// **TROIS DÉCISIONS DE PRÉSENTATION VIVENT ICI**, et chacune était
/// prise trois fois ailleurs avant P7 :
///
/// 1. **la marque.** `√` et `×`, et non `✓` et `✗` : ces deux-là
///    n'existent pas dans les polices qu'egui embarque et se peignaient
///    donc en carré vide — mesuré, voir
///    `tout_caractere_ecrit_existe_dans_la_police`. Une coche absente
///    ne dit pas « protection absente », elle ne dit rien du tout. Le
///    glyphe plutôt que la pastille ronde du panneau Protections, parce
///    qu'il porte l'information SANS la couleur : qui distingue mal le
///    rouge du vert lit encore la croix.
/// 2. **sa couleur**, qui vient du ton et donc du thème.
/// 3. **où se lit le pourquoi** : sous la ligne quand le verdict n'est
///    pas favorable, en infobulle quand il l'est. C'était la règle
///    écrite dans `protections`, et c'est la bonne — un point faible se
///    lit sans avoir à le chercher, et la liste tient malgré tout d'un
///    coup d'œil. « En bref » montrait au contraire ses quatre notes en
///    permanence, y compris pour ce qui va bien : quatre lignes de
///    commentaire dans un panneau qui s'appelle « En bref ».
fn peindre_verdict(
    ui: &mut egui::Ui,
    ctx: &ReContext,
    nom: &Ligne,
    état: &Ligne,
    verdict: Ton,
    pourquoi: Option<&str>,
    gestes: &mut Gestes,
) {
    let marque = match verdict {
        Ton::Mauvais => "\u{00d7}",
        // Rien à marquer pour ce qui n'est ni bon ni mauvais : un
        // RELRO partiel n'a pas de glyphe qui le dise, et en inventer
        // un ferait lire un verdict là où il n'y en a pas.
        Ton::Attention => "\u{2022}",
        _ => "\u{221a}",
    };
    let favorable = verdict == Ton::Bon;
    let ligne = ui
        .horizontal_wrapped(|ui| {
            peindre_fragment(
                ui,
                ctx,
                &Fragment::dit(format!("{marque} {}", nom.texte_nu())).ton(verdict),
                gestes,
            );
            for fragment in &état.fragments {
                peindre_fragment(ui, ctx, &fragment.clone().ton(verdict), gestes);
            }
        })
        .response;
    match (pourquoi, favorable) {
        (Some(texte), true) => {
            ligne.on_hover_text(texte);
        }
        (Some(texte), false) => {
            // En retrait sous la ligne : ces mots développent la
            // précédente, et c'est le même rattachement que
            // `Bloc::Retrait` dit dans les données.
            ui.indent(("vue_verdict", nom.texte_nu()), |ui| {
                ui.weak(texte);
            });
        }
        (None, _) => {}
    }
}

/// Le vert des verdicts favorables. Voir `riche` pour le pourquoi.
const VERT: egui::Color32 = egui::Color32::from_rgb(0x4e, 0xc9, 0xb0);

/// De quelle teinte se peint une famille de comportement.
///
/// **Cette table vivait dans `behaviour.rs`**, qui relève les familles
/// à partir des noms de fonctions et des numéros d'appels système — et
/// qui n'a rien à faire d'egui. Elle y est restée le temps que le
/// dernier module peignant `famille.color()` passe la frontière ; c'est
/// fait, et `behaviour.rs` ne connaît plus l'interface. Un module dit
/// une CLÉ — « réseau », « crypto » —, et c'est ici, du côté qui sait
/// peindre, qu'elle devient une couleur.
///
/// Les six teintes sont celles de VS Code Dark+ : le bleu des types
/// pour le réseau, le mauve des mots-clés de contrôle pour le
/// chiffrement, le rouge des erreurs pour l'anti-débogage.
pub fn couleur_de_famille(famille: crate::behaviour::Family) -> egui::Color32 {
    use crate::behaviour::Family;
    match famille {
        Family::AntiDebug => egui::Color32::from_rgb(0xf4, 0x87, 0x71),
        Family::Execution => egui::Color32::from_rgb(0xce, 0x91, 0x78),
        Family::Network => egui::Color32::from_rgb(0x56, 0x9c, 0xd6),
        Family::Memory => egui::Color32::from_rgb(0xd7, 0xba, 0x7d),
        Family::Crypto => egui::Color32::from_rgb(0xc5, 0x86, 0xc0),
        Family::Files => egui::Color32::from_rgb(0x4e, 0xc9, 0xb0),
    }
}

/// Ce que l'interface retient d'un champ entre deux images.
///
/// Deux textes et non un : ce qui est TAPÉ, et ce que le module
/// PROPOSAIT la dernière fois. Sans le second, on ne saurait pas
/// distinguer « le module a changé d'avis » de « l'utilisateur est en
/// train d'écrire », et l'un des deux écraserait l'autre à chaque
/// image — soit le champ se vide sous les doigts, soit « Défaut » ne
/// rend plus rien.
#[derive(Clone)]
struct Frappe {
    proposée: String,
    tapée: String,
}

/// Un champ de saisie, son étiquette et ses boutons, sur une rangée.
///
/// LE TAMPON DE FRAPPE VIT ICI, dans la mémoire d'egui, et non dans le
/// module : c'est le même partage que l'état d'ouverture d'un
/// repliable. Le module n'apprend ce qui est écrit qu'au moment où l'on
/// valide — par la touche Entrée, ou par le bouton qui porte la même
/// action que le champ.
fn peindre_champ(ui: &mut egui::Ui, ctx: &ReContext, champ: &Champ, gestes: &mut Gestes) {
    let id = ui.id().with(("vue_champ", &champ.action));
    let mut frappe = ui.data(|d| d.get_temp::<Frappe>(id)).unwrap_or(Frappe {
        proposée: champ.valeur.clone(),
        tapée: champ.valeur.clone(),
    });
    // Le module a changé d'avis : sa valeur l'emporte sur ce qui est
    // tapé. C'est ainsi que « Défaut » rend son nom d'origine à une
    // fonction sans que le module tienne le tampon.
    if frappe.proposée != champ.valeur {
        frappe.proposée = champ.valeur.clone();
        frappe.tapée = champ.valeur.clone();
    }
    // UNE LARGEUR EN CARACTÈRES, convertie ici et nulle part ailleurs :
    // le module dit « vingt caractères », l'interface sait ce que
    // mesurent les siens.
    let style = if champ.code {
        egui::TextStyle::Monospace
    } else {
        egui::TextStyle::Body
    };
    let police = style.resolve(ui.style());
    let glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(&police, '0')).max(1.0);
    // Plus la marge intérieure du champ, sans quoi le dernier caractère
    // écrit touche le cadre.
    let largeur = champ.caractères as f32 * glyphe + 8.0;

    let mut validé = false;
    let mut locaux = Gestes::default();
    ui.horizontal(|ui| {
        if !champ.étiquette.fragments.is_empty() {
            peindre_ligne(ui, ctx, &champ.étiquette, &mut locaux);
        }
        let mut édition = egui::TextEdit::singleline(&mut frappe.tapée).desired_width(largeur);
        if champ.code {
            édition = édition.font(egui::TextStyle::Monospace);
        }
        if let Some(indice) = &champ.indice {
            édition = édition.hint_text(indice.clone());
        }
        let réponse = ui.add(édition);
        // Entrée valide, comme partout ailleurs dans namsdec : un champ
        // qui demande d'aller chercher le bouton à la souris fait taper
        // l'adresse deux fois sur trois.
        validé = réponse.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        for bouton in &champ.boutons {
            peindre_fragment(ui, ctx, bouton, &mut locaux);
        }
    });
    // Le bouton qui porte la MÊME action que le champ le valide ; les
    // autres demandent ce qu'ils disent.
    for action in locaux.actions.drain(..) {
        if action == champ.action {
            validé = true;
        } else {
            gestes.actions.push(action);
        }
    }
    if let Some(demande) = locaux.demande {
        gestes.demande = Some(demande);
    }
    if validé {
        gestes
            .actions
            .push(format!("{}:{}", champ.action, frappe.tapée));
    }
    ui.data_mut(|d| d.insert_temp(id, frappe));
}

// --- Le graphe d'appels ----------------------------------------------
//
// Deux cents lignes de dessin, qui vivaient dans `call_graph_module`.
// Elles décident d'un pas vertical, d'une indentation, d'une largeur de
// boîte en caractères — c'est-à-dire des pixels, et les pixels sont le
// métier de l'interface. Le module, lui, dit la structure : des nœuds,
// leur distance à la racine, et les arêtes entre eux.

/// Le graphe, dessiné comme un vrai diagramme : une boîte peinte et
/// cliquable par nœud, reliée à son parent par un connecteur fléché.
///
/// Peint sur un canevas unique (`allocate_painter`) plutôt qu'empilé en
/// widgets : c'est ce qui en fait un graphe et non une liste. Une
/// première version posait des boutons dans une colonne et traçait des
/// traits par-dessus ; aux couleurs discrètes du thème, cela ne se
/// lisait pas comme un graphe. Ici les boîtes ont un fond et un cadre
/// francs, et les liens portent une flèche — on voit qui appelle qui.
///
/// Rend l'adresse de la boîte cliquée, s'il y en a une.
fn graphe(ui: &mut egui::Ui, nœuds: &[Nœud], arêtes: &[(usize, usize)]) -> Option<u64> {
    if nœuds.is_empty() {
        return None;
    }
    // Les couleurs, relevées du thème avant d'emprunter `ui` pour le
    // canevas. Celles d'un graphe doivent se voir : on ne prend pas les
    // fonds estompés qui servent aux fiches.
    let v = ui.visuals();
    let couleur_texte = v.text_color();
    let couleur_faible = v.weak_text_color();
    let couleur_accent = v.hyperlink_color;
    let couleur_cadre = v.widgets.noninteractive.fg_stroke.color;
    let fond_boîte = v.extreme_bg_color;
    let fond_accent = v.selection.bg_fill.gamma_multiply(0.45);
    let fond_survol = v.widgets.hovered.bg_fill;
    let couleur_trait = couleur_texte.gamma_multiply(0.55);

    let police = egui::FontId::monospace(12.0);
    let hauteur_ligne = ui.ctx().fonts_mut(|f| f.row_height(&police));
    let glyphe = ui.ctx().fonts_mut(|f| f.glyph_width(&police, '0')).max(1.0);
    const PAD_X: f32 = 8.0;
    const PAD_Y: f32 = 4.0;
    const INDENT_X: f32 = 30.0;
    /// Combien de lignes le compartiment du bas porte, au plus.
    ///
    /// Le pas doit valoir la PLUS HAUTE des boîtes — celle qui porte un
    /// stéréotype ET le plus de lignes de corps —, sinon elle mord sur
    /// sa voisine du dessous.
    const LIGNES_DE_CORPS: usize = 2;
    let lignes_de_corps = nœuds
        .iter()
        .map(|n| n.corps.len())
        .max()
        .unwrap_or(LIGNES_DE_CORPS)
        .max(LIGNES_DE_CORPS);
    let pas_vertical = (1 + lignes_de_corps + 1) as f32 * hauteur_ligne + PAD_Y * 4.0 + 10.0;

    // UNE BOÎTE À COMPARTIMENTS, COMME UNE CLASSE EN UML.
    //
    // C'était une suite de rectangles d'une ligne — « nom 0x401136 » —,
    // ce qui tient de la liste indentée plus que du diagramme. Demandé
    // le 20 octobre : « pour le graphe du programme un style UML et non
    // une suite de boîtes ».
    //
    // La grammaire d'UML est reprise là où elle dit quelque chose de
    // vrai sur un graphe d'appels, et nulle part ailleurs :
    //
    // - un **stéréotype** entre guillemets français pour ce que le nœud
    //   EST — « entrée », « main », « renvoi » ;
    // - un **compartiment de titre**, le nom, séparé par un filet ;
    // - un **compartiment de corps**, ce qu'on mesure ;
    // - une **dépendance** entre les boîtes, qui en UML se trace en
    //   TIRETS avec une pointe OUVERTE. C'est exactement ce qu'un appel
    //   est : « A dépend de B ». Un trait plein à pointe pleine dirait
    //   une association, qui ne veut rien dire ici.
    struct Boîte {
        rect: egui::Rect,
        accent: bool,
        renvoi: bool,
    }
    let mut boîtes: Vec<Boîte> = Vec::with_capacity(nœuds.len());
    let mut largeur_max = 0.0f32;
    for (i, nœud) in nœuds.iter().enumerate() {
        let plus_large = nœud
            .corps
            .iter()
            .map(|l| l.chars().count())
            .chain(std::iter::once(nœud.titre.chars().count()))
            // Le stéréotype porte ses guillemets : quatre caractères de
            // plus, et c'est l'interface qui les ajoute.
            .chain(nœud.stéréotype.iter().map(|s| s.chars().count() + 4))
            .max()
            .unwrap_or(0);
        let largeur = plus_large as f32 * glyphe + PAD_X * 2.0;
        // UNE LIGNE PAR LIGNE PEINTE, et c'est la correction d'un
        // défaut vu à l'écran : le compte oubliait que le compartiment
        // du bas en porte DEUX, et la seconde — « 0 appelants · 13
        // appels » — se peignait sous le trait du cadre, hors de la
        // boîte.
        let lignes = 1 + nœud.corps.len() + usize::from(nœud.stéréotype.is_some());
        let hauteur = lignes as f32 * hauteur_ligne + PAD_Y * 4.0;
        let x = nœud.profondeur as f32 * INDENT_X;
        let y = i as f32 * pas_vertical;
        largeur_max = largeur_max.max(x + largeur);
        boîtes.push(Boîte {
            rect: egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(largeur, hauteur)),
            // UN TON, ET NON DEUX DRAPEAUX : le module dit « fort » de
            // ce par quoi on entre dans le programme et « faible » de
            // ce qui est déjà déplié ailleurs ; l'interface en fait un
            // cadre d'accent ou un texte estompé.
            accent: nœud.ton == Ton::Fort,
            renvoi: nœud.ton == Ton::Faible,
        });
    }
    let hauteur_totale = nœuds.len() as f32 * pas_vertical;

    let (réponse, peintre) = ui.allocate_painter(
        egui::vec2(largeur_max.max(120.0), hauteur_totale.max(40.0)),
        egui::Sense::click(),
    );
    let origine = réponse.rect.min.to_vec2();
    let décalé = |r: egui::Rect| r.translate(origine);
    let pointeur = réponse.hover_pos();

    // Les connecteurs d'abord : ils passent sous les boîtes.
    //
    // EN TIRETS, ET À POINTE OUVERTE : c'est ainsi qu'UML trace une
    // DÉPENDANCE, et une dépendance est exactement ce qu'un appel est —
    // « celle-ci a besoin de celle-là ». Un trait plein à pointe pleine
    // dirait une association, c'est-à-dire un lien durable entre deux
    // objets, ce qui ne veut rien dire ici.
    let trait_ = egui::Stroke::new(1.5, couleur_trait);
    for &(de, à) in arêtes {
        let (Some(parent), Some(enfant)) = (boîtes.get(de), boîtes.get(à)) else {
            continue;
        };
        let rp = décalé(parent.rect);
        let rn = décalé(enfant.rect);
        let x_rail = rp.left() + 13.0;
        let y_enfant = rn.center().y;
        tirets(
            &peintre,
            egui::pos2(x_rail, rp.bottom()),
            egui::pos2(x_rail, y_enfant),
            trait_,
        );
        tirets(
            &peintre,
            egui::pos2(x_rail, y_enfant),
            egui::pos2(rn.left(), y_enfant),
            trait_,
        );
        flèche(&peintre, egui::pos2(rn.left(), y_enfant), couleur_trait);
    }

    // Puis les boîtes, et le clic qui mène à l'adresse.
    let mut cliquée = None;
    let mut sur_une_boîte = false;
    for (nœud, boîte) in nœuds.iter().zip(&boîtes) {
        let rect = décalé(boîte.rect);
        let survol = pointeur.is_some_and(|p| rect.contains(p));
        sur_une_boîte |= survol && nœud.mène_à.is_some();
        let fond = if survol {
            fond_survol
        } else if boîte.accent {
            fond_accent
        } else {
            fond_boîte
        };
        // UN RECTANGLE À ANGLES DROITS : une classe UML n'a pas de
        // coins arrondis, et c'est le premier signe qui dit qu'on lit un
        // diagramme et non une liste de pastilles.
        peintre.rect_filled(rect, 0.0, fond);
        peintre.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(
                if boîte.accent { 2.0 } else { 1.0 },
                if boîte.accent {
                    couleur_accent
                } else {
                    couleur_cadre
                },
            ),
            egui::StrokeKind::Inside,
        );
        let couleur = if boîte.renvoi {
            couleur_faible
        } else {
            couleur_texte
        };
        let mut y = rect.top() + PAD_Y;
        // Le stéréotype, entre guillemets français comme UML l'écrit :
        // « entrée », « main », « renvoi ».
        if let Some(stéréotype) = &nœud.stéréotype {
            peintre.text(
                egui::pos2(rect.center().x, y),
                egui::Align2::CENTER_TOP,
                format!("\u{ab}{stéréotype}\u{bb}"),
                police.clone(),
                couleur_faible,
            );
            y += hauteur_ligne;
        }
        // Le compartiment de titre, centré comme un nom de classe.
        peintre.text(
            egui::pos2(rect.center().x, y),
            egui::Align2::CENTER_TOP,
            &nœud.titre,
            police.clone(),
            couleur,
        );
        y += hauteur_ligne + PAD_Y;
        // LE FILET QUI SÉPARE LES DEUX COMPARTIMENTS. C'est lui qui
        // fait une classe plutôt qu'une étiquette encadrée.
        peintre.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, couleur_cadre),
        );
        y += PAD_Y;
        for ligne in &nœud.corps {
            peintre.text(
                egui::pos2(rect.left() + PAD_X, y),
                egui::Align2::LEFT_TOP,
                ligne,
                police.clone(),
                couleur_faible,
            );
            y += hauteur_ligne;
        }
        if survol && réponse.clicked() {
            cliquée = nœud.mène_à;
        }
    }

    // Le curseur dit que les boîtes mènent quelque part.
    if sur_une_boîte {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    cliquée
}

/// Un segment en TIRETS, parce qu'egui n'en trace pas tout seul.
///
/// Le motif est fixe — six pixels de trait, quatre de vide — et non
/// proportionnel à la longueur : deux dépendances de longueurs
/// différentes doivent se ressembler, sinon le tireté se lit comme une
/// information qu'il ne porte pas.
fn tirets(peintre: &egui::Painter, de: egui::Pos2, à: egui::Pos2, trait_: egui::Stroke) {
    const TRAIT: f32 = 6.0;
    const VIDE: f32 = 4.0;
    let total = de.distance(à);
    if total <= f32::EPSILON {
        return;
    }
    let pas = (à - de) / total;
    let mut parcouru = 0.0;
    while parcouru < total {
        let fin = (parcouru + TRAIT).min(total);
        peintre.line_segment([de + pas * parcouru, de + pas * fin], trait_);
        parcouru = fin + VIDE;
    }
}

/// La pointe d'une DÉPENDANCE UML : deux traits, et rien entre eux.
///
/// Elle était pleine — un triangle rempli —, ce qui en UML désigne une
/// autre relation. Une pointe ouverte, deux segments en V, est celle
/// d'une dépendance ou d'une association simple, et c'est ce qu'un
/// appel est. La forme porte donc une information juste, au lieu d'en
/// porter une fausse.
fn flèche(peintre: &egui::Painter, bout: egui::Pos2, couleur: egui::Color32) {
    const T: f32 = 7.0;
    let trait_ = egui::Stroke::new(1.5, couleur);
    let arrière = bout - egui::vec2(T, 0.0);
    peintre.line_segment([bout, arrière + egui::vec2(0.0, T * 0.55)], trait_);
    peintre.line_segment([bout, arrière - egui::vec2(0.0, T * 0.55)], trait_);
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::{ALL, Family};

    /// Chaque famille de comportement garde sa teinte propre.
    ///
    /// **Ce contrôle vivait dans `behaviour.rs`**, et il en est parti
    /// avec la table : ce fichier-là relève des familles dans des noms
    /// de fonctions, et ne connaît plus egui. Deux familles de la même
    /// couleur rendraient le portrait d'un binaire illisible — c'est
    /// par la teinte qu'on relie une ligne de la liste à sa case du
    /// portrait, et non par sa place.
    #[test]
    fn une_famille_garde_sa_teinte_propre() {
        let couleurs: Vec<_> = ALL
            .into_iter()
            .map(|famille| couleur_de_famille(famille).to_array())
            .collect();
        let uniques: std::collections::BTreeSet<_> = couleurs.iter().collect();
        assert_eq!(uniques.len(), ALL.len(), "une couleur par famille");
    }

    /// La clé fait le tour complet : un module la dit, l'interface
    /// remonte à la famille et en tire la teinte.
    ///
    /// C'est le chemin entier de la frontière pour une étiquette, et il
    /// se casse en silence si une clé change d'un côté seulement — le
    /// mot se peindrait alors sans couleur, ce qu'aucune mesure de
    /// pixels n'irait chercher.
    #[test]
    fn la_clé_d_une_famille_mène_à_sa_teinte() {
        let fragment = crate::vue::Fragment::dit("réseau").étiquette(Family::Network.clé());
        let clé = fragment.étiquette.as_deref().expect("la clé est dite");
        let retrouvée = Family::de_clé(clé).expect("la clé se relit");
        assert_eq!(retrouvée, Family::Network);
        assert_eq!(
            couleur_de_famille(retrouvée),
            couleur_de_famille(Family::Network)
        );
    }
}
