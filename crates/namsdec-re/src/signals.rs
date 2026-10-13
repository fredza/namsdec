//! Le panneau « Signaux » : ce qui ressort, et où aller le lire.
//!
//! Dix panneaux d'analyse répondent chacun à une question, et c'est bien
//! — mais devant un échantillon inconnu, la première question est
//! « par où commencer ». Celui-ci n'analyse **rien** de nouveau : il
//! rassemble ce que les autres ont déjà trouvé, garde ce qui sort de
//! l'ordinaire, et dit chaque fois quel panneau l'explique.
//!
//! ## Il ne conclut pas
//!
//! C'est la règle du module, et elle tient tout le reste. Un segment
//! inscriptible et exécutable n'est pas une preuve ; `mprotect` dans un
//! binaire n'est pas un déballeur ; une entropie de 7,5 n'est pas du
//! chiffrement. Chaque ligne dit donc **un fait** — l'adresse, le nom,
//! la mesure —, **ce que ce fait change** pour qui lit, et **où** le
//! vérifier. Rien n'est noté, rien n'est additionné en score : un score
//! donnerait l'illusion d'un verdict, et c'est le chercheur qui conclut.
//!
//! ## Un silence veut dire quelque chose
//!
//! Quand rien ne ressort, le panneau le dit en une phrase au lieu de
//! rester vide. « Rien de remarquable » est une information ; un panneau
//! blanc ressemble à un panneau cassé.

use crate::behaviour::{self, Family};
use crate::entropy::{self, seuils};
use crate::{ReContext, ReModule, ReSection};

/// Combien un signal mérite qu'on s'y arrête d'abord.
///
/// Deux degrés et pas trois : au-delà, on classe des choses qui ne se
/// comparent pas. « Remarquable » veut dire *rare* — peu de programmes
/// ordinaires le font ; « à noter » veut dire *utile à savoir*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weight {
    /// Rare dans un programme ordinaire.
    Remarkable,
    /// Courant, mais bon à savoir avant de lire la suite.
    Worth,
}

/// Un signal relevé : le fait, ce qu'il change, et où le vérifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub weight: Weight,
    /// Le titre, dans les deux langues.
    pub fr: &'static str,
    pub en: &'static str,
    /// Le fait, tel qu'on peut le vérifier : une adresse, un nom, une
    /// mesure. Jamais une appréciation.
    pub detail: String,
    /// Ce que ce fait change pour qui lit le binaire.
    pub why_fr: &'static str,
    pub why_en: &'static str,
    /// Le panneau qui l'explique en entier.
    pub panel_fr: &'static str,
    pub panel_en: &'static str,
}

impl Signal {
    pub fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.fr
        } else {
            self.en
        }
    }

    pub fn why(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.why_fr
        } else {
            self.why_en
        }
    }

    /// Le panneau où aller lire la suite.
    pub fn panel(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            self.panel_fr
        } else {
            self.panel_en
        }
    }
}

/// Tout ce qui ressort de ce binaire, du plus rare au plus banal.
///
/// Fonction libre, et qui ne demande pas de `ReContext` : le fichier, son
/// désassemblage, la langue. Rien d'autre ne sert ici — et c'est ce qui
/// permet au **rapport** d'appeler la même fonction que le panneau, sans
/// avoir à fabriquer un processeur simulé pour l'occasion. Deux
/// synthèses qui diraient des choses différentes seraient pires qu'une.
///
/// Aucune analyse n'est refaite ici : chaque ligne vient d'un module qui
/// la calculait déjà.
pub fn gather(
    binary: &[u8],
    disasm: &[namsdec_disasm::DecodedInstruction],
    lang: &str,
) -> Vec<Signal> {
    /// Le petit traducteur local, le temps de cette fonction : les
    /// détails factuels se disent dans la langue de l'interface.
    fn t<'a>(lang: &str, fr: &'a str, en: &'a str) -> &'a str {
        if lang.starts_with("fr") { fr } else { en }
    }
    let mut out: Vec<Signal> = Vec::new();

    // --- Ce que le fichier dit de lui-même --------------------------
    for empaqueteur in crate::packer::detect(binary) {
        out.push(Signal {
            weight: Weight::Remarkable,
            fr: "Empaqueteur reconnu",
            en: "Packer identified",
            detail: format!("{} — {}", empaqueteur.name, empaqueteur.clue.label(lang)),
            why_fr: "Ce qu'on lit ici n'est pas le programme : c'est son enveloppe. \
                     Tant qu'elle n'est pas défaite, les autres panneaux décrivent le \
                     déballeur.",
            why_en: "What one reads here is not the program: it is its wrapper. Until it \
                     is undone, the other panels describe the unpacker.",
            panel_fr: "Entropie",
            panel_en: "Entropy",
        });
    }

    if let Some(elf) = crate::elf_scan::scan(binary) {
        // --- Les droits de la mémoire ------------------------------
        for segment in elf.writable_executable() {
            out.push(Signal {
                weight: Weight::Remarkable,
                fr: "Segment inscriptible et exécutable",
                en: "Writable and executable segment",
                detail: format!("{:#x} ({})", segment.vaddr, segment.rights()),
                why_fr: "On peut écrire du code dans cette zone et sauter dedans. \
                         Presque aucun programme ordinaire n'en a besoin ; un programme \
                         qui se déballe lui-même, si.",
                why_en: "Code can be written into this area and jumped into. Almost no \
                         ordinary program needs this; a self-unpacking one does.",
                panel_fr: "Protections du binaire",
                panel_en: "Binary protections",
            });
        }

        // --- Les protections absentes ------------------------------
        //
        // Une par ligne et non « protections faibles » en bloc : ce
        // n'est pas la même information qu'il manque NX ou qu'il manque
        // PIE, et les regrouper obligerait à ouvrir l'autre panneau
        // pour savoir laquelle.
        // `Some(false)` et non `None` : un fichier qui ne déclare rien
        // laisse le noyau décider, et on ne reproche pas au binaire un
        // choix qui n'est pas le sien.
        if elf.nx == Some(false) {
            out.push(Signal {
                weight: Weight::Remarkable,
                fr: "NX absent",
                en: "NX missing",
                detail: t(lang, "la pile est exécutable", "the stack is executable").to_owned(),
                why_fr: "Des octets écrits sur la pile peuvent s'exécuter. C'est la \
                         protection la plus ancienne, et il est rare qu'elle manque sans \
                         raison.",
                why_en: "Bytes written onto the stack can execute. This is the oldest \
                         protection, and it rarely goes missing without a reason.",
                panel_fr: "Protections du binaire",
                panel_en: "Binary protections",
            });
        }
        if elf.dynamic() && !elf.canary {
            out.push(Signal {
                weight: Weight::Worth,
                fr: "Pas de canari de pile",
                en: "No stack canary",
                detail: t(
                    lang,
                    "aucun témoin posé avant l'adresse de retour",
                    "no witness placed before the return address",
                )
                .to_owned(),
                why_fr: "Un débordement de tampon local peut atteindre l'adresse de \
                         retour sans que rien ne s'en aperçoive.",
                why_en: "A local buffer overflow can reach the return address with \
                         nothing noticing.",
                panel_fr: "Protections du binaire",
                panel_en: "Binary protections",
            });
        }

        // --- Le fichier est-il lisible du tout ? -------------------
        if elf.stripped {
            out.push(Signal {
                weight: Weight::Worth,
                fr: "Binaire dépouillé",
                en: "Stripped binary",
                detail: t(
                    lang,
                    "aucun nom de fonction dans le fichier",
                    "no function name in the file",
                )
                .to_owned(),
                why_fr: "Les fonctions n'ont plus que leur adresse : c'est le panneau \
                         « Fonctions » qui les retrouvera, et les noms qu'on posera \
                         soi-même qui tiendront lieu de mémoire.",
                why_en: "Functions have nothing left but their address: the “Functions” \
                         panel will recover them, and the names one writes oneself will \
                         serve as memory.",
                panel_fr: "Fonctions",
                panel_en: "Functions",
            });
        }
    }

    // --- L'entropie, et où elle monte ------------------------------
    //
    // La tranche la plus dense, et non la moyenne : un bloc chiffré rangé
    // en queue de fichier ne déplace pas la moyenne d'un fichier de deux
    // mégaoctets.
    //
    // Mais la tranche doit être **grande**, et c'est là que ce panneau
    // diffère du panneau Entropie. Celui-ci découpe en soixante-quatre
    // tranches pour dessiner une forme ; sur un petit binaire, chacune
    // fait quatre ou cinq cents octets — et cinq cents octets variés
    // atteignent 7,6 bits sans rien cacher du tout, simplement parce
    // qu'il n'y a pas de quoi répéter quoi que ce soit. Mesuré sur
    // `/bin/true` : la queue du fichier, 505 octets, 7,61. Une forme
    // dessinée peut se permettre cette imprécision ; une ligne qui dit
    // « remarquable », non.
    const FENÊTRE: usize = 4096;
    let tranches = entropy::profile(binary, (binary.len() / FENÊTRE).max(1));
    if let Some(haute) = tranches
        .iter()
        .max_by(|a, b| a.entropy.total_cmp(&b.entropy))
        && haute.entropy >= seuils::EMPAQUETÉ
        && haute.len >= FENÊTRE
    {
        {
            // Où dans le fichier ? Une bosse en queue est la forme d'un
            // programme empaqueté ; la même en tête peut n'être qu'une
            // table de symboles compressée.
            let part = haute.offset as f64 / binary.len().max(1) as f64;
            out.push(Signal {
                weight: Weight::Remarkable,
                fr: "Entropie élevée sur une tranche",
                en: "High entropy over one slice",
                detail: format!(
                    "{:#x}..{:#x} — {:.2} {} ({} {:.0} %)",
                    haute.offset,
                    haute.offset + haute.len,
                    haute.entropy,
                    if lang.starts_with("fr") {
                        "bits/octet"
                    } else {
                        "bits/byte"
                    },
                    t(lang, "à", "at"),
                    part * 100.0
                ),
                why_fr: "Aucune habitude dans ces octets : compressés, chiffrés, ou \
                         empaquetés. C'est une piste et non un verdict — un petit bloc \
                         de code très dense atteint la même valeur.",
                why_en: "No habit in these bytes: compressed, encrypted, or packed. A \
                         hint and not a verdict — a small block of very dense code \
                         reaches the same value.",
                panel_fr: "Entropie",
                panel_en: "Entropy",
            });
        }
    }

    // --- Ce que le binaire sait faire ------------------------------
    //
    // Les deux sources ensemble : une fonction empruntée et un appel
    // système direct disent la même chose, et un programme qui veut
    // passer inaperçu emploie justement le second.
    let empruntées = crate::imports::scan(binary);
    let appels = crate::syscalls::scan(disasm);
    let mut familles: Vec<(Family, String)> = Vec::new();
    for famille in behaviour::ALL {
        let noms: Vec<String> = empruntées
            .iter()
            .filter(|i| behaviour::family_of(&i.name) == Some(famille))
            .map(|i| i.name.clone())
            .chain(
                appels
                    .iter()
                    .filter(|c| c.family() == Some(famille))
                    .filter_map(|c| c.number)
                    .filter_map(crate::table::syscall)
                    // Dit « par le noyau » et non « () » : un nom suivi
                    // de parenthèses se lit comme une fonction, et c'est
                    // justement ce que ce n'est pas. Le même nom peut
                    // apparaître deux fois — emprunté *et* demandé
                    // directement —, et ce n'est pas un doublon : c'est
                    // une information.
                    .map(|s| {
                        format!(
                            "{} ({})",
                            s.name,
                            if lang.starts_with("fr") {
                                "noyau"
                            } else {
                                "kernel"
                            }
                        )
                    }),
            )
            .collect();
        if noms.is_empty() {
            continue;
        }
        // Trois noms suffisent à dire ce qui tient la famille ; la liste
        // entière est dans les deux panneaux qui la portent.
        let mut vus: Vec<String> = Vec::new();
        for nom in noms {
            if !vus.contains(&nom) {
                vus.push(nom);
            }
        }
        let combien = vus.len();
        let début = vus.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
        familles.push((
            famille,
            if combien > 3 {
                format!("{début}, … ({combien})")
            } else {
                début
            },
        ));
    }
    for (famille, détail) in familles {
        // Les familles rares pèsent plus : ouvrir un socket distingue un
        // programme, ouvrir un fichier ne distingue personne.
        let poids = match famille {
            Family::AntiDebug | Family::Execution | Family::Network => Weight::Remarkable,
            Family::Memory | Family::Crypto | Family::Files => Weight::Worth,
        };
        out.push(Signal {
            weight: poids,
            fr: match famille {
                Family::AntiDebug => "Anti-débogage",
                Family::Execution => "Lance d'autres programmes",
                Family::Network => "Parle au réseau",
                Family::Memory => "Change les droits de sa mémoire",
                Family::Crypto => "Chiffre ou brouille",
                Family::Files => "Touche aux fichiers",
            },
            en: match famille {
                Family::AntiDebug => "Anti-debugging",
                Family::Execution => "Launches other programs",
                Family::Network => "Talks to the network",
                Family::Memory => "Changes its memory rights",
                Family::Crypto => "Encrypts or scrambles",
                Family::Files => "Touches files",
            },
            detail: détail,
            why_fr: famille.why("fr"),
            why_en: famille.why("en"),
            panel_fr: "Fonctions importées",
            panel_en: "Imported functions",
        });
    }

    // --- Du code qui se déballe ? ----------------------------------
    //
    // La famille « mémoire » dit que le programme peut changer les droits
    // d'une page ; qu'un *saut indirect* existe par ailleurs dit qu'il
    // peut aller s'exécuter là où il vient d'écrire. Les deux ensemble,
    // c'est la forme d'un déballeur — et séparément, ce n'est rien.
    let touche_la_mémoire = empruntées
        .iter()
        .any(|i| behaviour::family_of(&i.name) == Some(Family::Memory))
        || appels.iter().any(|c| c.family() == Some(Family::Memory));
    if touche_la_mémoire {
        let sauts_indirects = disasm
            .iter()
            .filter_map(|instr| namsdec_disasm::decode_one(&instr.bytes, instr.address))
            .filter(|decoded| {
                matches!(
                    decoded.flow_control(),
                    iced_x86::FlowControl::IndirectBranch | iced_x86::FlowControl::IndirectCall
                )
            })
            .count();
        if sauts_indirects > 0 {
            out.push(Signal {
                weight: Weight::Worth,
                fr: "Code auto-modifiant possible",
                en: "Self-modifying code possible",
                detail: format!(
                    "{sauts_indirects} {}",
                    match (lang.starts_with("fr"), sauts_indirects) {
                        // Un seul saut calculé est le cas le plus
                        // parlant — et « 1 sauts » se lit mal.
                        (true, 1) => "saut calculé, et les droits mémoire sont modifiables",
                        (true, _) => "sauts calculés, et les droits mémoire sont modifiables",
                        (false, 1) => "computed jump, and memory rights are changeable",
                        (false, _) => "computed jumps, and memory rights are changeable",
                    }
                ),
                why_fr: "Les deux morceaux d'un déballeur sont là : de quoi rendre une \
                         page exécutable, et de quoi sauter à une adresse calculée. \
                         Séparément, chacun est banal — c'est leur rencontre qui vaut \
                         d'être regardée.",
                why_en: "Both halves of an unpacker are present: a way to make a page \
                         executable, and a way to jump to a computed address. Separately \
                         each is ordinary — their meeting is what deserves a look.",
                panel_fr: "Appels système",
                panel_en: "System calls",
            });
        }
    }

    // --- Le listing dit-il la vérité ? -----------------------------
    //
    // La seule ligne de ce panneau qui mette en cause ce que namsdec
    // AFFICHE. Elle passe donc avant les indicateurs : tant qu'on ne
    // sait pas que le listing ment, on lit de travers tout le reste.
    let base = disasm.first().map(|i| i.address).unwrap_or(0);
    let désalignés = crate::misalign::scan_in(disasm, &crate::misalign::code_zones(binary, base));
    if !désalignés.is_empty() {
        let premier = &désalignés[0];
        out.push(Signal {
            weight: Weight::Remarkable,
            fr: "Le listing est trompé",
            en: "The listing is being fooled",
            // « -> » et non « → » : la flèche n'existe pas dans les
            // polices qu'egui embarque, et se peignait en carré vide.
            detail: format!(
                "{:#x} -> {:#x} ({} {})",
                premier.from,
                premier.target,
                désalignés.len(),
                t(lang, "endroits", "places")
            ),
            why_fr: "Un saut atterrit au milieu d'une instruction du listing : le processeur lit d'autres octets que ce qui est affiché. C'est de l'anti-désassemblage, et c'est délibéré — aucun compilateur ne produit cette figure.",
            why_en: "A jump lands in the middle of an instruction of the listing: the processor reads other bytes than what is shown. This is anti-disassembly, and it is deliberate — no compiler produces this shape.",
            panel_fr: "Figures du code",
            panel_en: "Code shapes",
        });
    }

    // --- Ce qu'on peut aller vérifier ailleurs ---------------------
    let chaînes = crate::strings::scan_elf(binary).unwrap_or_default();
    let indicateurs = crate::ioc::from_strings(&chaînes);
    if !indicateurs.is_empty() {
        let premiers = indicateurs
            .iter()
            .take(2)
            .map(|i| i.value.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        out.push(Signal {
            weight: Weight::Worth,
            fr: "Indicateurs dans les chaînes",
            en: "Indicators in the strings",
            detail: if indicateurs.len() > 2 {
                format!("{premiers}, … ({})", indicateurs.len())
            } else {
                premiers
            },
            why_fr: "Des adresses, des chemins, des clés : les seules choses du binaire \
                     qu'on puisse aller vérifier ailleurs.",
            why_en: "Addresses, paths, keys: the only things in the binary one can go and \
                     check elsewhere.",
            panel_fr: "Chaînes lisibles",
            panel_en: "Readable strings",
        });
    }

    // Du plus rare au plus banal, et l'ordre d'ajout conservé à poids
    // égal : c'est l'ordre dans lequel on a envie de lire.
    out.sort_by_key(|signal| signal.weight);
    out
}

/// Le panneau de synthèse.
#[derive(Default)]
pub struct SignalsModule {
    signals: Vec<Signal>,
    /// Le fichier portait-il quelque chose à lire ? Sans fichier, ce
    /// n'est pas « rien de remarquable », c'est « rien du tout ».
    loaded: bool,
}

impl SignalsModule {
    /// Les signaux relevés, pour qui veut les lire sans l'interface.
    pub fn signals(&self) -> &[Signal] {
        &self.signals
    }
}

impl ReModule for SignalsModule {
    fn section(&self) -> ReSection {
        ReSection::File
    }

    fn id(&self) -> &'static str {
        "signals"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Signaux"
        } else {
            "Signals"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.loaded = !ctx.binary.is_empty() || !ctx.disasm.is_empty();
        self.signals = gather(ctx.binary, ctx.disasm, ctx.lang);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if !self.loaded {
            ui.weak(ctx.t(
                "Rien à relever : aucun fichier chargé.",
                "Nothing to report: no file loaded.",
            ));
            return;
        }
        if self.signals.is_empty() {
            ui.label(ctx.t(
                "Rien de remarquable : pas de segment inscriptible et exécutable, pas \
                 d'entropie qui monte, rien d'emprunté qui sorte de l'ordinaire.",
                "Nothing remarkable: no writable and executable segment, no rising \
                 entropy, nothing borrowed that stands out.",
            ));
            ui.weak(ctx.t(
                "Ce qui ne veut pas dire inoffensif — seulement que ce binaire ne porte \
                 aucune des marques que namsdec sait reconnaître.",
                "Which does not mean harmless — only that this binary carries none of the \
                 marks namsdec knows how to recognise.",
            ));
            return;
        }

        ui.weak(ctx.t(
            "Ce que les autres panneaux ont trouvé, rassemblé. Aucune de ces lignes n'est \
             une preuve, et rien n'est additionné en score : chacune dit un fait, ce que \
             ce fait change, et où aller le vérifier.",
            "What the other panels found, gathered together. None of these lines is \
             proof, and nothing is added up into a score: each says a fact, what that \
             fact changes, and where to go and check it.",
        ));
        ui.add_space(6.0);

        let mut poids_précédent: Option<Weight> = None;
        for signal in &self.signals {
            // Un intertitre quand on passe du rare au banal : sans lui,
            // la liste se lit comme si tout pesait pareil.
            if poids_précédent != Some(signal.weight) {
                if poids_précédent.is_some() {
                    ui.add_space(8.0);
                }
                ui.strong(match signal.weight {
                    Weight::Remarkable => ctx.t("Remarquable", "Remarkable"),
                    Weight::Worth => ctx.t("À savoir", "Worth knowing"),
                });
                poids_précédent = Some(signal.weight);
            }
            let couleur = match signal.weight {
                Weight::Remarkable => egui::Color32::from_rgb(0xf4, 0x87, 0x71),
                Weight::Worth => egui::Color32::from_rgb(0xd7, 0xba, 0x7d),
            };
            ui.horizontal_wrapped(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 4.0, couleur);
                ui.strong(signal.title(ctx.lang));
                // Un tiret entre le titre et le fait. Sans lui, les deux
                // se lisaient d'un trait — « Binaire dépouillé aucun nom
                // de fonction dans le fichier » — et l'on cherchait le
                // verbe d'une phrase qui n'en a pas.
                ui.weak("—");
                ui.monospace(&signal.detail);
            });
            ui.indent(signal.title("en"), |ui| {
                ui.weak(signal.why(ctx.lang));
                ui.weak(format!(
                    "{} {}",
                    ctx.t("à lire dans", "read it in"),
                    signal.panel(ctx.lang)
                ));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// Rien ne ressort d'un fichier vide, et rien ne tombe non plus.
    #[test]
    fn nothing_stands_out_of_nothing() {
        let mut module = SignalsModule::default();
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        module.on_binary_loaded(&ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        });
        assert!(module.signals().is_empty());
        assert!(!module.loaded, "sans fichier, ce n'est pas « rien à dire »");
    }

    /// Un empaqueteur reconnu ressort en premier, avec sa preuve.
    #[test]
    fn a_named_packer_comes_first_with_its_evidence() {
        let mut fichier = vec![0u8; 512];
        fichier.extend_from_slice(b"UPX!");
        // Et un gros bloc imprévisible, comme en laisse un empaqueteur.
        let mut graine = 0x2545f4914f6cdd1du64;
        for _ in 0..8192 {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            fichier.push((graine >> 24) as u8);
        }

        let signaux = gather(&fichier, &[], "fr");
        assert_eq!(signaux[0].fr, "Empaqueteur reconnu", "{signaux:?}");
        assert!(signaux[0].detail.contains("UPX"));
        // La preuve se vérifie : l'adresse du nombre magique y est.
        assert!(signaux[0].detail.contains("0x200"), "{:?}", signaux[0]);
        // Et l'entropie du bloc ressort aussi, séparément.
        assert!(
            signaux
                .iter()
                .any(|s| s.fr == "Entropie élevée sur une tranche"),
            "{signaux:?}"
        );
        // Chaque ligne renvoie à un panneau : c'est la promesse du
        // module.
        assert!(signaux.iter().all(|s| !s.panel("fr").is_empty()));
    }

    /// Un `ptrace` appelé par numéro ressort comme anti-débogage.
    ///
    /// Par numéro et non par import : un programme qui veut passer
    /// inaperçu n'emprunte rien, il parle au noyau directement. Si le
    /// panneau ne regardait que la table de liaison, il ne verrait rien
    /// de ce programme-là.
    #[test]
    fn a_raw_ptrace_syscall_shows_up_as_anti_debugging() {
        // mov eax, 101 (ptrace) / syscall
        let octets: &[u8] = &[0xB8, 0x65, 0x00, 0x00, 0x00, 0x0F, 0x05];
        let disasm = namsdec_disasm::disassemble(octets, 0x400000);

        let signaux = gather(&[], &disasm, "fr");
        let anti = signaux
            .iter()
            .find(|s| s.fr == "Anti-débogage")
            .unwrap_or_else(|| panic!("{signaux:?}"));
        assert_eq!(anti.weight, Weight::Remarkable);
        assert!(anti.detail.contains("ptrace (noyau)"), "{anti:?}");
        assert_eq!(anti.panel("en"), "Imported functions");
    }

    /// Les lignes rares d'abord : c'est l'ordre dans lequel on lit.
    #[test]
    fn the_rare_lines_come_before_the_ordinary_ones() {
        // `open` (2) puis `socket` (41) : l'un est banal, l'autre non.
        let octets: &[u8] = &[
            0xB8, 0x02, 0x00, 0x00, 0x00, 0x0F, 0x05, 0xB8, 0x29, 0x00, 0x00, 0x00, 0x0F, 0x05,
        ];
        let disasm = namsdec_disasm::disassemble(octets, 0x400000);
        let signaux = gather(&[], &disasm, "fr");

        let rang = |titre: &str| {
            signaux
                .iter()
                .position(|s| s.fr == titre)
                .unwrap_or_else(|| panic!("« {titre} » attendu dans {signaux:?}"))
        };
        assert!(
            rang("Parle au réseau") < rang("Touche aux fichiers"),
            "{signaux:?}"
        );
    }

    /// Un listing trompé ressort, et renvoie au panneau qui le montre.
    ///
    /// C'est la seule ligne du panneau qui mette en cause ce que namsdec
    /// **affiche** : tant qu'on ne sait pas que le listing ment, on lit
    /// de travers tout le reste.
    #[test]
    fn a_fooled_listing_stands_out() {
        //   jmp $+3 / db 0xE8 (l'octet-piège) / xor eax,eax / inc rax / ret
        let octets: &[u8] = &[0xEB, 0x01, 0xE8, 0x31, 0xC0, 0x48, 0xFF, 0xC0, 0xC3];
        let disasm = namsdec_disasm::disassemble(octets, 0x400000);
        let signaux = gather(&[], &disasm, "fr");
        let ligne = signaux
            .iter()
            .find(|s| s.fr == "Le listing est trompé")
            .unwrap_or_else(|| panic!("{signaux:?}"));
        assert_eq!(ligne.weight, Weight::Remarkable);
        assert!(ligne.detail.contains("0x400003"), "{ligne:?}");
        assert_eq!(ligne.panel("fr"), "Figures du code");
    }

    /// Une petite tranche dense ne suffit pas à crier à l'empaquetage.
    ///
    /// Le cas qui l'a imposé : la queue de `/bin/true`, 505 octets à
    /// 7,61 bits. Rien n'y est caché — il n'y a simplement pas de quoi
    /// répéter un octet dans cinq cents. Le panneau Entropie peut
    /// dessiner cette bosse ; une ligne « remarquable » mentirait.
    #[test]
    fn a_small_dense_slice_is_not_enough_to_cry_packing() {
        // Un petit fichier entièrement imprévisible : chaque tranche y
        // est dense, et aucune n'est assez grande pour en conclure.
        let mut graine = 0x9e3779b97f4a7c15u64;
        let petit: Vec<u8> = (0..1024)
            .map(|_| {
                graine ^= graine << 13;
                graine ^= graine >> 7;
                graine ^= graine << 17;
                (graine >> 24) as u8
            })
            .collect();
        let signaux = gather(&petit, &[], "fr");
        assert!(
            !signaux
                .iter()
                .any(|s| s.fr == "Entropie élevée sur une tranche"),
            "mille octets ne prouvent rien : {signaux:?}"
        );

        // Le même bruit sur seize kibioctets, lui, ressort.
        let mut gros = petit.clone();
        while gros.len() < 16 * 1024 {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            gros.push((graine >> 24) as u8);
        }
        let signaux = gather(&gros, &[], "fr");
        assert!(
            signaux
                .iter()
                .any(|s| s.fr == "Entropie élevée sur une tranche"),
            "seize kibioctets de bruit, c'est autre chose : {signaux:?}"
        );
    }

    /// Sur un vrai binaire du système, rien de remarquable ne ressort.
    ///
    /// C'est le contrôle qui garde le panneau utilisable : s'il allumait
    /// trois lignes rouges sur `/bin/true`, on cesserait de le lire.
    #[test]
    fn an_ordinary_system_binary_raises_no_alarm() {
        // Deux binaires et non un : `/bin/true` ne fait presque rien, et
        // c'est `/bin/ls` — qui ouvre des dossiers, lit des liens, pose
        // un gestionnaire de signaux — qui a montré le premier faux
        // positif. Un contrôle qui ne regarde que le cas pauvre ne
        // protège pas du cas qu'on rencontre.
        let mut lus = 0;
        for chemin in ["/bin/true", "/bin/ls"] {
            let Ok(octets) = std::fs::read(chemin) else {
                continue;
            };
            lus += 1;
            let signaux = gather(&octets, &[], "fr");
            for signal in &signaux {
                assert_ne!(
                    signal.weight,
                    Weight::Remarkable,
                    "{chemin} ne devrait rien avoir de remarquable : {signal:?}"
                );
            }
        }
        if lus == 0 {
            eprintln!("aucun binaire du système trouvé : contrôle ignoré");
        }
    }

    /// Le titre suit la langue, comme celui des autres modules.
    #[test]
    fn the_title_follows_the_language() {
        let module = SignalsModule::default();
        assert_eq!(module.title("fr"), "Signaux");
        assert_eq!(module.title("en"), "Signals");
    }
}
