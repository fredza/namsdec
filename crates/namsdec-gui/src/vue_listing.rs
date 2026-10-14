//! Le listing peint **ligne à ligne**, et non d'un seul bloc.
//!
//! POURQUOI. Un `TextEdit` d'egui met en page tout son texte avant d'en
//! peindre la moindre ligne. Mesuré le 14 octobre 2026 : un listing d'un
//! million de lignes demande 2,9 secondes à la première image, et le
//! galley qui en sort pèse ce que pèsent ses glyphes. Pire, le prix ne
//! s'arrête pas là — epaint parcourt toutes les rangées à chaque
//! tesselation pour écarter les invisibles une par une, et le cache de
//! mise en page doit reconnaître le texte entier à chaque image.
//!
//! Or trente lignes sont à l'écran. Les mille autres n'ont ni à être
//! mises en page, ni à être gardées en mémoire.
//!
//! CE QU'ON PERD, ET POURQUOI C'EST ACCEPTABLE. Un champ de texte porte
//! la saisie, la sélection à la souris, le presse-papier, le caret. Rien
//! de tout cela n'existe en mode NORMAL : le champ y est déjà posé
//! `interactive(false)`, il ne fait que peindre. C'est donc là, et là
//! seulement, que cette vue le remplace — on la quitte en entrant en
//! Insertion, qui redonne le vrai champ avec tout ce qu'il porte.
//!
//! LA GÉOMÉTRIE. Police à chasse fixe, pas de repli : la ligne *n* est à
//! *n* pas de la première, et la colonne *c* est là où le galley de SA
//! ligne la pose. Les deux moitiés comptent — le pas se calcule (sans
//! quoi il faudrait le galley de tout le fichier pour le connaître), les
//! abscisses se lisent dans le galley de la ligne (sans quoi elles
//! dériveraient : egui cale chaque glyphe sur le pixel, et `x_offset(1)`
//! vaut 7 là où la largeur d'un caractère vaut 7,83).

use super::*;
use crate::editor::layout_code;

/// L'air entre le dernier chiffre de la gouttière et la première
/// colonne de code.
///
/// Il fait partie de la gouttière plutôt que d'être un espacement de
/// mise en page : l'espacement horizontal est mis à zéro pour que le
/// texte commence exactement où la gouttière s'arrête.
pub(crate) const GUTTER_PAD: f32 = 16.0;

/// La place que prennent les marques de la gouttière, à sa gauche : la
/// bande de chaleur, la pastille du point d'arrêt et la flèche de la
/// ligne courante.
pub(crate) const GUTTER_MARQUES: f32 = 26.0;

/// La largeur de la gouttière : de quoi écrire le plus grand numéro
/// SANS recouvrir les marques.
///
/// Elle était fixe — trente-huit pixels, cinq chiffres serrés —, et deux
/// choses s'y perdaient. À quatre chiffres, le numéro commençait sur la
/// pastille du point d'arrêt : une capture du 14 octobre 2026 montre
/// « ●012 » là où la ligne 4 012 porte un arrêt. Et un listing d'un
/// million de lignes en demande sept, qui ne tenaient pas du tout.
///
/// Elle suit donc le fichier : un programme de trente lignes garde une
/// gouttière étroite, un binaire d'un million de lignes la paie de ce
/// qu'elle vaut.
pub(crate) fn gutter_width(lignes: u32, largeur_caractère: f32) -> f32 {
    let chiffres = lignes.max(1).ilog10() + 1;
    GUTTER_MARQUES + chiffres as f32 * largeur_caractère + GUTTER_PAD
}

/// Le pas vertical d'une ligne à l'autre, tel qu'egui le pose vraiment.
///
/// `Fonts::row_height` rend la hauteur de la police — 15,125 ici —, et
/// les rangées d'un galley se suivent de 15,0 : egui cale chaque rangée
/// sur le pixel. Un huitième de pixel par ligne ne se voit pas sur
/// cinquante lignes et fait six pixels sur cinquante, puis cent vingt
/// sur mille. C'est la hauteur d'UNE ligne mise en page qui fait foi,
/// et c'est elle qu'on prend.
pub(crate) fn pas_de_ligne(ui: &egui::Ui, theme: &'static Theme) -> f32 {
    layout_code(ui, theme, "M").rect.height().max(1.0)
}

/// Ce que la vue doit savoir du texte ENTIER, et qu'elle ne peut pas
/// apprendre des seules lignes visibles.
///
/// Trois choses, une passe : OÙ COMMENCE CHAQUE LIGNE, quelle est la
/// plus longue, et ce que le fichier définit lui-même (ses macros et ses
/// étiquettes, sans quoi la coloration d'une ligne dépendrait de
/// l'endroit d'où on la regarde).
///
/// L'index des débuts de ligne est ce qui rend la vue vraiment
/// indépendante de la taille du fichier : sans lui, atteindre la ligne
/// 900 000 demanderait de relire les 899 999 premières à chaque image —
/// `split('\n').skip(n)` ne sait pas faire autrement. Il coûte quatre
/// octets par ligne, quatre mégaoctets pour un million de lignes, là où
/// le galley qu'on remplace en pesait plusieurs centaines.
///
/// Relevé une fois par texte, et gardé : c'est le seul endroit où cette
/// vue lit le fichier en entier.
pub(crate) struct ReleveDuTexte {
    /// L'octet où commence chaque ligne. Jamais vide : un texte vide a
    /// une ligne, vide elle aussi.
    pub débuts: Vec<u32>,
    /// La plus longue, en caractères.
    pub colonnes: usize,
    /// Ce que le fichier définit : macros et étiquettes.
    pub macros: syntax::Macros,
}

impl ReleveDuTexte {
    /// Relève le texte entier.
    pub(crate) fn de(texte: &str) -> Self {
        let mut débuts = vec![0u32];
        let mut colonnes = 0usize;
        let mut début = 0usize;
        for (at, _) in texte.match_indices('\n') {
            colonnes = colonnes.max(texte[début..at].chars().count());
            début = at + 1;
            débuts.push(début as u32);
        }
        colonnes = colonnes.max(texte[début..].chars().count());
        Self {
            débuts,
            colonnes,
            macros: syntax::Macros::of(texte),
        }
    }

    /// Combien de lignes.
    pub(crate) fn lignes(&self) -> u32 {
        self.débuts.len() as u32
    }

    /// Le texte de la ligne `n`, comptée depuis 1.
    pub(crate) fn ligne<'a>(&self, texte: &'a str, n: u32) -> Option<&'a str> {
        let début = *self.débuts.get(n.checked_sub(1)? as usize)? as usize;
        let fin = match self.débuts.get(n as usize) {
            // Le retour à la ligne ne fait pas partie de la ligne.
            Some(suivant) => (*suivant as usize).saturating_sub(1),
            None => texte.len(),
        };
        texte.get(début..fin)
    }

    /// L'octet où tombe la colonne `colonne` (en caractères) de la ligne
    /// `n`.
    ///
    /// Au-delà de la fin de la ligne, c'est sa fin : le curseur peut
    /// s'y poser, et le mot qu'on y cherche est le dernier.
    pub(crate) fn octet(&self, texte: &str, n: u32, colonne: usize) -> Option<usize> {
        let début = *self.débuts.get(n.checked_sub(1)? as usize)? as usize;
        let ligne = self.ligne(texte, n)?;
        let dans_la_ligne = ligne
            .char_indices()
            .nth(colonne)
            .map_or(ligne.len(), |(at, _)| at);
        Some(début + dans_la_ligne)
    }
}

/// Les lignes visibles, mises en page — et rien d'autre.
pub(crate) struct LignesPosées {
    /// La première ligne posée, comptée depuis 1 comme partout ailleurs.
    pub première: u32,
    /// Le coin haut-gauche de la première ligne, à l'écran.
    pub origine: egui::Pos2,
    /// Le pas d'une ligne à la suivante.
    pub pas: f32,
    /// Un galley par ligne posée, dans l'ordre.
    pub galleys: Vec<std::sync::Arc<egui::Galley>>,
}

impl LignesPosées {
    /// Le galley de cette ligne, si elle est posée.
    fn galley(&self, ligne: u32) -> Option<&egui::Galley> {
        let rang = ligne.checked_sub(self.première)? as usize;
        self.galleys.get(rang).map(|g| &**g)
    }
}

/// Où tombent les lignes du listing, à l'écran.
///
/// Deux réponses pour une même question, selon qui a mis le texte en
/// page — et c'est tout l'intérêt : la gouttière, les surlignages, les
/// commentaires et le curseur sont peints par le MÊME code dans les deux
/// cas. Deux peintures qui diraient des choses différentes seraient
/// pires qu'une.
pub(crate) enum Géométrie<'a> {
    /// Celle du champ de texte : son galley porte toutes les lignes.
    Champ {
        galley: &'a egui::Galley,
        origine: egui::Pos2,
    },
    /// Celle de la vue ligne à ligne : seules les lignes visibles sont
    /// posées, et le pas donne les autres.
    Lignes(&'a LignesPosées),
}

impl Géométrie<'_> {
    /// Le rectangle de la ligne, à l'écran — `None` si elle n'existe pas
    /// ou n'est pas posée.
    pub(crate) fn rect_de_ligne(&self, ligne: u32) -> Option<egui::Rect> {
        match self {
            Géométrie::Champ { galley, origine } => {
                let rangée = galley.rows.get(ligne.checked_sub(1)? as usize)?;
                Some(rangée.rect().translate(origine.to_vec2()))
            }
            Géométrie::Lignes(posées) => {
                let rang = ligne.checked_sub(posées.première)?;
                let galley = posées.galleys.get(rang as usize)?;
                let haut = posées.origine.y + rang as f32 * posées.pas;
                Some(egui::Rect::from_min_size(
                    egui::pos2(posées.origine.x, haut),
                    egui::vec2(galley.rect.width(), posées.pas),
                ))
            }
        }
    }

    /// L'abscisse d'une colonne de cette ligne, à l'écran.
    ///
    /// La colonne se compte en CARACTÈRES depuis le début de la ligne, et
    /// une colonne au-delà de la fin rend le bout de la ligne : c'est ce
    /// qu'il faut pour peindre un curseur en fin de ligne.
    pub(crate) fn x_de_colonne(&self, ligne: u32, colonne: usize) -> Option<f32> {
        match self {
            Géométrie::Champ { galley, origine } => {
                let rangée = galley.rows.get(ligne.checked_sub(1)? as usize)?;
                Some(origine.x + rangée.pos.x + rangée.row.x_offset(egui::text::CharIndex(colonne)))
            }
            Géométrie::Lignes(posées) => {
                let galley = posées.galley(ligne)?;
                let rangée = galley.rows.first()?;
                Some(posées.origine.x + rangée.row.x_offset(egui::text::CharIndex(colonne)))
            }
        }
    }

    /// Le rectangle d'une occurrence, à l'écran.
    ///
    /// Sa ligne et sa colonne sont déjà connues (voir `text::Place`) :
    /// il ne reste qu'à demander deux abscisses et la hauteur de la
    /// ligne.
    pub(crate) fn rect_de_place(&self, place: &Place) -> Option<egui::Rect> {
        let rect = self.rect_de_ligne(place.ligne)?;
        let gauche = self.x_de_colonne(place.ligne, place.colonne)?;
        let droite = self.x_de_colonne(place.ligne, place.colonne + place.largeur)?;
        Some(egui::Rect::from_min_max(
            egui::pos2(gauche, rect.top()),
            egui::pos2(droite, rect.bottom()),
        ))
    }

    /// La colonne qu'une abscisse d'écran désigne sur cette ligne.
    ///
    /// Comptée en caractères, et au plus proche : on vise une lettre, et
    /// la moitié franchie compte pour la suivante — c'est la règle d'un
    /// curseur de texte, et celle que `cursor_from_pos` applique aussi.
    /// La différence est qu'elle ne parcourt pas toutes les rangées
    /// depuis la première pour arriver ici.
    pub(crate) fn colonne_à(&self, ligne: u32, x: f32) -> Option<usize> {
        let (rangée, origine_x) = match self {
            Géométrie::Champ { galley, origine } => {
                let rangée = galley.rows.get(ligne.checked_sub(1)? as usize)?;
                (&rangée.row, origine.x + rangée.pos.x)
            }
            Géométrie::Lignes(posées) => {
                let galley = posées.galley(ligne)?;
                (&galley.rows.first()?.row, posées.origine.x)
            }
        };
        let relatif = x - origine_x;
        Some(
            rangée
                .glyphs
                .partition_point(|glyphe| glyphe.pos.x + glyphe.advance_width * 0.5 < relatif),
        )
    }

    /// Les lignes qui tombent dans `vue`, et elles seules.
    ///
    /// Pour le champ de texte, c'est une dichotomie sur les rangées du
    /// galley ; pour la vue ligne à ligne, ce sont les lignes posées,
    /// qui sont déjà celles qu'on voit.
    pub(crate) fn lignes_dans(&self, vue: egui::Rect) -> std::ops::RangeInclusive<u32> {
        match self {
            Géométrie::Champ { galley, origine } => {
                let tranche = rangées_visibles(galley, origine.to_vec2(), vue);
                // Une tranche vide se dit par un intervalle dont le début
                // dépasse la fin, et `1..=0` en est un — mais clippy le
                // prend pour une faute de frappe, à raison : on l'écrit
                // donc à partir de la tranche elle-même.
                let début = tranche.start as u32 + 1;
                début..=début + (tranche.len() as u32).saturating_sub(1)
            }
            Géométrie::Lignes(_) => self.lignes(),
        }
    }

    /// Les lignes posées, celles dont on peut demander la géométrie.
    pub(crate) fn lignes(&self) -> std::ops::RangeInclusive<u32> {
        match self {
            Géométrie::Champ { galley, .. } => 1..=galley.rows.len() as u32,
            Géométrie::Lignes(posées) => {
                posées.première..=posées.première + posées.galleys.len().saturating_sub(1) as u32
            }
        }
    }

    /// La ligne qu'une ordonnée d'écran désigne, si elle en désigne une.
    pub(crate) fn ligne_à(&self, y: f32) -> Option<u32> {
        match self {
            Géométrie::Champ { galley, origine } => {
                let vue = egui::Rect::from_min_max(
                    egui::pos2(origine.x, y),
                    egui::pos2(origine.x + 1.0, y),
                );
                let tranche = rangées_visibles(galley, origine.to_vec2(), vue);
                galley.rows[tranche.clone()]
                    .iter()
                    .position(|row| {
                        row.rect()
                            .translate(origine.to_vec2())
                            .y_range()
                            .contains(y)
                    })
                    .map(|rang| (tranche.start + rang) as u32 + 1)
            }
            Géométrie::Lignes(posées) => {
                let rang = ((y - posées.origine.y) / posées.pas).floor();
                if rang < 0.0 || rang >= posées.galleys.len() as f32 {
                    return None;
                }
                Some(posées.première + rang as u32)
            }
        }
    }
}

/// Met en page les lignes `lignes` du texte, et rien d'autre.
///
/// `macros` vient du fichier ENTIER : une macro définie en bas s'emploie
/// en haut, et une étiquette définie plus loin se colore ici. Les relever
/// par ligne colorerait le même mot de deux façons selon l'endroit d'où
/// on le regarde.
pub(crate) fn poser_les_lignes(
    ui: &egui::Ui,
    theme: &'static Theme,
    texte: &str,
    relevé: &ReleveDuTexte,
    lignes: std::ops::Range<u32>,
    origine: egui::Pos2,
    pas: f32,
) -> LignesPosées {
    let mut galleys = Vec::with_capacity(lignes.len());
    for n in lignes.clone() {
        // Par l'index : une ligne se prend à son octet, et non en
        // relisant tout ce qui la précède.
        let ligne = relevé.ligne(texte, n).unwrap_or("");
        let job = syntax::highlight_line(ui, theme, ligne, &relevé.macros);
        galleys.push(ui.fonts_mut(|f| f.layout_job(job)));
    }
    LignesPosées {
        première: lignes.start.max(1),
        origine,
        pas,
        galleys,
    }
}

/// Peint les lignes posées.
pub(crate) fn peindre(painter: &egui::Painter, posées: &LignesPosées) {
    for (rang, galley) in posées.galleys.iter().enumerate() {
        let à = egui::pos2(
            posées.origine.x,
            posées.origine.y + rang as f32 * posées.pas,
        );
        // La couleur est dans le galley, section par section : celle
        // qu'on passe ici ne sert qu'aux sections qui n'en portent pas,
        // et il n'y en a aucune.
        painter.galley(à, galley.clone(), egui::Color32::WHITE);
    }
}
