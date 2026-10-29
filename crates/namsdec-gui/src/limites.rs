//! Les simplifications de l'émulateur, mises en phrases.
//!
//! L'émulateur NOMME ce qu'il simplifie — `namsdec_emu::Simplification`
//! — et ne dit rien de plus : une phrase figée en français au fond du
//! moteur ne se traduirait plus, c'est la même règle que pour
//! `EmuError`. C'est ici qu'elle devient française ou anglaise.
//!
//! À part de `i18n.rs` pour la raison inverse de `errors.rs` : ces
//! textes n'interpolent rien, mais ils vont par paire — un titre court
//! pour la barre d'état, une explication pour le panneau — et les
//! accrocher à la grande table des chaînes y ajouterait quatorze champs
//! que rien d'autre ne lit.

use namsdec_emu::Simplification;

use crate::i18n::Lang;

/// Le titre court : ce qui passe dans la barre d'état au moment où le
/// programme touche la simplification.
pub(crate) fn titre(lang: Lang, quoi: Simplification) -> &'static str {
    let fr = lang == Lang::Fr;
    match quoi {
        Simplification::X87PrecisionEtendue => {
            if fr {
                "x87 : la précision étendue est calculée en double précision"
            } else {
                "x87: extended precision is computed in double precision"
            }
        }
        Simplification::X87FormatEtenduEnMemoire => {
            if fr {
                "x87 : un 80 bits passe par un double, ses onze derniers bits sont perdus"
            } else {
                "x87: an 80-bit value goes through a double, its last eleven bits are lost"
            }
        }
        Simplification::X87Transcendantes => {
            if fr {
                "x87 : les transcendantes viennent de la bibliothèque de Rust"
            } else {
                "x87: transcendentals come from Rust's library"
            }
        }
        Simplification::X87ArrondiSimpleAuPlusProche => {
            if fr {
                "x87 : à 24 bits, l'arrondi suit le plus proche même en mode dirigé"
            } else {
                "x87: at 24 bits, rounding follows nearest even in a directed mode"
            }
        }
        Simplification::X87ExceptionsMasquees => {
            if fr {
                "x87 : démasquer une exception flottante ne change rien ici"
            } else {
                "x87: unmasking a floating-point exception has no effect here"
            }
        }
        Simplification::X87MotDEtatPartiel => {
            if fr {
                "x87 : le mot d'état ne porte pas les bits d'exception"
            } else {
                "x87: the status word carries no exception bits"
            }
        }
        Simplification::X87FpremEnUnPas => {
            if fr {
                "x87 : fprem rend le reste complet du premier coup"
            } else {
                "x87: fprem returns the complete remainder on the first try"
            }
        }
    }
}

/// L'explication : ce qui diffère du vrai processeur, et ce que ça
/// change pour le programme qu'on regarde. Une phrase qui dit « c'est
/// approximatif » sans dire de combien n'apprend rien.
pub(crate) fn explication(lang: Lang, quoi: Simplification) -> &'static str {
    let fr = lang == Lang::Fr;
    match quoi {
        Simplification::X87PrecisionEtendue => {
            if fr {
                "Le mot de contrôle demande 64 bits de mantisse — c'est son réglage par \
                 défaut — et namsdec calcule sur les 53 du double. Sur le programme \
                 d'épreuve, (1 + 1e-18) - 1 rend 9,76e-19 sur le matériel et zéro \
                 ici. Posez `fldcw` à 0x027f (53 bits) et les deux redeviennent \
                 identiques : à cette précision, namsdec EST le processeur."
            } else {
                "The control word asks for 64 mantissa bits — its default setting — and \
                 namsdec computes on the double's 53. On the test program, (1 + 1e-18) - 1 \
                 gives 9.76e-19 on the hardware and zero here. Set `fldcw` to 0x027f \
                 (53 bits) and the two match again: at that precision, namsdec IS the \
                 processor."
            }
        }
        Simplification::X87FormatEtenduEnMemoire => {
            if fr {
                "Un `fstp tword` écrit bien dix octets, mais reconstruits depuis un \
                 double : les onze derniers bits de mantisse sont nuls. Un `fld tword` \
                 qui relit un vrai 80 bits les arrondit de même. L'aller-retour par la \
                 mémoire reste exact pour toute valeur qui tient dans un double."
            } else {
                "An `fstp tword` does write ten bytes, but rebuilt from a double: the last \
                 eleven mantissa bits are zero. An `fld tword` reading a true 80-bit value \
                 rounds it the same way. The round trip through memory stays exact for any \
                 value that fits in a double."
            }
        }
        Simplification::X87Transcendantes => {
            if fr {
                "`fsin`, `fcos`, `fptan`, `fpatan`, `fyl2x` et `f2xm1` passent par la \
                 bibliothèque mathématique de Rust et non par le polynôme du \
                 coprocesseur. Les derniers bits diffèrent — et ils diffèrent déjà entre \
                 deux générations de processeurs : le manuel d'Intel ne garantit que \
                 l'ordre de grandeur de l'erreur."
            } else {
                "`fsin`, `fcos`, `fptan`, `fpatan`, `fyl2x` and `f2xm1` go through Rust's \
                 math library rather than the coprocessor's polynomial. The last bits \
                 differ — and they already differ between two processor generations: \
                 Intel's manual only guarantees the order of magnitude of the error."
            }
        }
        Simplification::X87ArrondiSimpleAuPlusProche => {
            if fr {
                "Le contrôle de précision à 24 bits est simulé en arrondissant chaque \
                 opération de base au format simple, et cet arrondi-là suit le plus \
                 proche. Le mode dirigé que vous avez réglé gouverne bien les \
                 conversions vers l'entier ; c'est le seul endroit où il ne s'applique \
                 pas."
            } else {
                "The 24-bit precision control is simulated by rounding every basic \
                 operation to single format, and that rounding follows nearest even. The \
                 directed mode you set does govern integer conversions; this is the only \
                 place where it does not apply."
            }
        }
        Simplification::X87ExceptionsMasquees => {
            if fr {
                "Les six bits bas du mot de contrôle masquent les exceptions flottantes \
                 — division par zéro, débordement, opération invalide. namsdec n'en lève \
                 aucune : les démasquer n'appellera pas de gestionnaire, et le calcul \
                 continue avec un infini ou un NaN, comme si elles étaient restées \
                 masquées."
            } else {
                "The low six bits of the control word mask the floating-point exceptions — \
                 divide by zero, overflow, invalid operation. namsdec raises none of them: \
                 unmasking will not call a handler, and the computation goes on with an \
                 infinity or a NaN, as though they had stayed masked."
            }
        }
        Simplification::X87MotDEtatPartiel => {
            if fr {
                "`fstsw` rend les quatre bits de condition — ceux que `fcom` écrit — et \
                 le numéro du sommet de pile, ce qui suffit au `fstsw ax` / `sahf` du \
                 code d'époque. Les six bits d'exception et le drapeau de synthèse ES \
                 restent à zéro, puisque rien ne les lève."
            } else {
                "`fstsw` returns the four condition bits — those `fcom` writes — and the \
                 top-of-stack index, which is all the old `fstsw ax` / `sahf` idiom needs. \
                 The six exception bits and the ES summary flag stay zero, since nothing \
                 raises them."
            }
        }
        Simplification::X87FpremEnUnPas => {
            if fr {
                "Le vrai `fprem` peut s'arrêter en chemin quand les deux exposants sont \
                 très éloignés, et le dit en laissant C2 à un : le programme doit alors \
                 recommencer en boucle. Ici le reste est complet du premier coup et C2 \
                 reste à zéro — la boucle tourne une fois, ce qui donne le bon résultat \
                 mais pas le bon nombre de tours."
            } else {
                "A real `fprem` may stop partway when the two exponents are far apart, and \
                 says so by leaving C2 set: the program then has to loop. Here the \
                 remainder is complete on the first try and C2 stays clear — the loop runs \
                 once, which gives the right result but not the right number of turns."
            }
        }
    }
}
