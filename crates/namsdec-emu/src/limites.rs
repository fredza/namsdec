//! Ce que namsdec simplifie, nommé une fois pour toutes.
//!
//! Un émulateur pédagogique a le droit de simplifier ; il n'a pas le
//! droit de le taire. Un programme qui tourne ici et donne un résultat
//! un peu différent du vrai processeur enseigne quelque chose de faux —
//! sauf si l'écart est annoncé, auquel cas il enseigne au contraire la
//! chose la plus utile du lot : que la virgule flottante a des réglages,
//! et que deux machines correctes peuvent ne pas répondre pareil.
//!
//! D'où ce recensement. Chaque entrée est un fait **vérifié** contre le
//! matériel, pas une précaution rédigée au cas où : voir l'entrée 9.55
//! d'`AMELIORATIONS.txt` pour le programme d'épreuve et les seize
//! mesures qui l'ont établi.
//!
//! Le texte n'est pas ici. L'émulateur nomme, `namsdec-gui` rend la
//! phrase dans la langue de l'utilisateur — la même règle que pour
//! `EmuError`, et pour la même raison : une phrase figée en français au
//! fond du moteur ne se traduit plus.

/// Un comportement que namsdec simule plus simplement que le
/// processeur.
///
/// L'ordre de `TOUTES` est celui dans lequel l'aide les présente : du
/// plus courant au plus rare.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Simplification {
    /// Les opérations de base calculent sur les 53 bits de mantisse du
    /// double, là où le réglage par défaut du x87 en demande 64.
    X87PrecisionEtendue,
    /// `fstp tword` écrit un 80 bits reconstruit depuis un double : ses
    /// onze derniers bits de mantisse sont nuls. Un `fld tword` qui
    /// relit un vrai 80 bits les perd de même.
    X87FormatEtenduEnMemoire,
    /// Les transcendantes passent par la bibliothèque de Rust et non
    /// par le polynôme du coprocesseur : les derniers bits diffèrent.
    X87Transcendantes,
    /// Avec le contrôle de précision à 24 bits, l'arrondi au format
    /// simple suit le plus proche même quand le mode réglé est dirigé.
    X87ArrondiSimpleAuPlusProche,
    /// Aucune exception flottante n'est levée ni signalée : démasquer
    /// les bits 0 à 5 du mot de contrôle n'a pas d'effet.
    X87ExceptionsMasquees,
    /// `fstsw` rend les bits de condition et le numéro du sommet, pas
    /// les six bits d'exception ni le drapeau de pile.
    X87MotDEtatPartiel,
    /// `fprem` et `fprem1` rendent le reste complet du premier coup, là
    /// où le processeur peut s'arrêter en chemin et le dire par C2.
    X87FpremEnUnPas,
}

/// La famille à laquelle une simplification appartient — ce qui permet
/// à l'aide de les regrouper sans que l'interface connaisse la liste.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Famille {
    /// Le coprocesseur 8087.
    X87,
}

impl Simplification {
    /// Le recensement complet, dans l'ordre où l'aide le présente.
    pub const TOUTES: &'static [Simplification] = &[
        Simplification::X87PrecisionEtendue,
        Simplification::X87FormatEtenduEnMemoire,
        Simplification::X87Transcendantes,
        Simplification::X87ArrondiSimpleAuPlusProche,
        Simplification::X87ExceptionsMasquees,
        Simplification::X87MotDEtatPartiel,
        Simplification::X87FpremEnUnPas,
    ];

    /// La clé stable sous laquelle l'interface retrouve les deux
    /// phrases. Stable : elle est écrite dans les tests et dans le
    /// manuel, la renommer casse les deux — ce qui est l'intention.
    pub fn cle(self) -> &'static str {
        match self {
            Simplification::X87PrecisionEtendue => "x87_precision_etendue",
            Simplification::X87FormatEtenduEnMemoire => "x87_format_etendu_memoire",
            Simplification::X87Transcendantes => "x87_transcendantes",
            Simplification::X87ArrondiSimpleAuPlusProche => "x87_arrondi_simple",
            Simplification::X87ExceptionsMasquees => "x87_exceptions",
            Simplification::X87MotDEtatPartiel => "x87_mot_etat",
            Simplification::X87FpremEnUnPas => "x87_fprem",
        }
    }

    pub fn famille(self) -> Famille {
        match self {
            Simplification::X87PrecisionEtendue
            | Simplification::X87FormatEtenduEnMemoire
            | Simplification::X87Transcendantes
            | Simplification::X87ArrondiSimpleAuPlusProche
            | Simplification::X87ExceptionsMasquees
            | Simplification::X87MotDEtatPartiel
            | Simplification::X87FpremEnUnPas => Famille::X87,
        }
    }
}

impl Famille {
    pub const TOUTES: &'static [Famille] = &[Famille::X87];

    pub fn cle(self) -> &'static str {
        match self {
            Famille::X87 => "x87",
        }
    }
}

use std::collections::BTreeSet;

use namsdec_core::Precision;

use crate::Emulator;

impl Emulator {
    /// Note qu'une simplification vient d'être touchée. Idempotent :
    /// une instruction dans une boucle ne remplit pas la liste.
    pub(crate) fn note_simplification(&mut self, quoi: Simplification) {
        if self.simplifications.insert(quoi) {
            self.a_annoncer.push(quoi);
        }
    }

    /// Ce que cette exécution a touché, pour un inventaire — l'aide,
    /// elle, montre `Simplification::TOUTES`.
    pub fn simplifications(&self) -> &BTreeSet<Simplification> {
        &self.simplifications
    }

    /// Ce qui reste à dire, et qu'on ne redira pas. L'interface vide
    /// cette file après chaque pas : annoncer au moment où le
    /// comportement se produit vaut mieux qu'un avertissement au
    /// démarrage, que personne ne relie à ce qu'il voit.
    pub fn simplifications_a_annoncer(&mut self) -> Vec<Simplification> {
        std::mem::take(&mut self.a_annoncer)
    }

    /// Le résultat d'une opération de base du x87, ramené à la
    /// précision réglée — et la simplification notée au passage, parce
    /// que c'est ici, et nulle part ailleurs, qu'on sait laquelle
    /// s'applique.
    pub(crate) fn shorten_x87(&mut self, value: f64) -> f64 {
        let x87 = self.cpu.regs.x87;
        match x87.precision() {
            Precision::Extended => self.note_simplification(Simplification::X87PrecisionEtendue),
            Precision::Single if x87.rounding() != namsdec_core::Rounding::Nearest => {
                self.note_simplification(Simplification::X87ArrondiSimpleAuPlusProche)
            }
            _ => {}
        }
        x87.shorten(value)
    }
}
