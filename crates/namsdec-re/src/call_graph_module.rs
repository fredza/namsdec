//! Le module « Graphe d'appels » : qui appelle qui, et ce qu'il y a dedans.
//!
//! Les autres modules disent *où* sont les fonctions (le module
//! « Fonctions ») et *ce qu'elles font de remarquable* (figures, appels
//! système). Celui-ci montre leur **structure** : à partir du point
//! d'entrée, l'arbre des appels, à la façon d'un diagramme UML — une
//! boîte par fonction, un trait vers chacune de celles qu'elle appelle.
//! C'est la carte qu'on lit avant de descendre dans une fonction.
//!
//! Trois choses s'y font :
//!
//! - **Aller à une adresse.** On tape une adresse, on y va dans le
//!   listing — et chaque boîte de l'arbre est elle-même un raccourci.
//! - **Voir l'arbre des appels.** Déplié depuis `_start`/`main`, chaque
//!   fonction menant à celles qu'elle appelle. Une fonction déjà vue
//!   ailleurs est marquée d'un renvoi plutôt que redéployée : c'est un
//!   arbre, pas une boucle infinie.
//! - **Lire le détail d'une fonction.** Sous l'arbre, chaque fonction se
//!   déplie sur son **code binaire** (les octets, bruts) et son
//!   **pseudo-code** (ce que chaque instruction fait, en clair).

use crate::functions::{Function, Origin, by_importance, discover_with};
use crate::vue::{Bloc, Champ, Dessin, Fragment, Ton, Vue};
use crate::{ReContext, ReModule, ReRequest, ReSection};
use std::collections::HashMap;

/// L'adresse tapée dans « aller à » ; la valeur suit l'action.
const ALLER: &str = "call_graph:aller";

/// La fiche d'une fonction s'ouvre ou se referme ; son adresse suit.
const FICHE: &str = "call_graph:fiche:";

/// Le nom qu'on donne à une fonction : son adresse, puis le nom.
const RENOMMER: &str = "call_graph:renommer:";

/// Rendre à une fonction son nom d'origine ; son adresse suit.
const DÉFAUT: &str = "call_graph:défaut:";

/// Combien de fonctions l'arbre et le détail montrent, au plus.
///
/// Un arbre de trois cents boîtes n'est plus une carte, c'est un mur. On
/// garde les plus employées — celles par où l'on entre dans un binaire —
/// et le compte total dit ce qu'on ne montre pas.
const MAX_FONCTIONS: usize = 40;

/// Jusqu'où l'arbre descend. Au-delà, on marque « … » : une pile d'appels
/// de douze niveaux ne se lit plus, et la profondeur seule n'apprend rien.
const MAX_PROFONDEUR: usize = 8;

/// Combien de nœuds l'arbre dessine en tout, renvois compris.
const MAX_NŒUDS: usize = 120;

/// Combien d'instructions une rubrique de détail montre avant de couper.
const MAX_INSTRUCTIONS: usize = 200;

/// Un nœud de l'arbre, aplati dans l'ordre où on le dessine.
struct Nœud {
    /// Rang de la fonction dans `self.functions`.
    fonction: usize,
    /// Profondeur dans l'arbre (0 = la racine).
    profondeur: usize,
    /// Le nœud parent, dans cette liste aplatie.
    parent: Option<usize>,
    /// Un renvoi : la fonction est déjà dépliée ailleurs. On la montre,
    /// mais on ne redéploie pas ses appels.
    renvoi: bool,
}

#[derive(Default)]
pub struct CallGraphModule {
    /// Ce que le panneau demande à l'application. Relevé après le dessin.
    demande: Option<ReRequest>,
    functions: Vec<Function>,
    /// De l'adresse de début d'une fonction à son rang dans `functions`.
    par_adresse: HashMap<u64, usize>,
    entry: u64,
    main: Option<u64>,
    /// Les rangs des fonctions montrées, les plus employées d'abord.
    montrées: Vec<usize>,
    /// Dit quand l'adresse tapée ne se lit pas, pour l'écrire sous le champ.
    aller_à_erreur: bool,
    /// La fonction dont la fiche est ouverte, s'il y en a une.
    ///
    /// Une seule à la fois, et le module la retient lui-même plutôt que
    /// de la laisser à l'interface : le détail d'une fonction se
    /// compose de son pseudo-code, et bâtir les quarante à chaque image
    /// coûte 2,6 ms en release pour du texte que personne ne regarde.
    /// Voir [`CallGraphModule::fiche`] pour la mesure.
    ouverte: Option<u64>,
}

impl CallGraphModule {
    /// L'arbre des appels, aplati dans l'ordre de dessin (pré-ordre).
    ///
    /// Depuis la racine — le point d'entrée, ou `main` à défaut —, on
    /// descend par les appels. Une fonction déjà posée devient un renvoi :
    /// l'arbre reste fini même quand le code boucle sur lui-même.
    fn arbre(&self) -> Vec<Nœud> {
        let mut nœuds = Vec::new();
        let Some(&racine) = self
            .main
            .and_then(|m| self.par_adresse.get(&m))
            .or_else(|| self.entry_index())
            .or_else(|| self.montrées.first())
        else {
            return nœuds;
        };
        let mut vus = std::collections::HashSet::new();
        self.descendre(racine, 0, None, &mut nœuds, &mut vus);
        nœuds
    }

    fn entry_index(&self) -> Option<&usize> {
        self.par_adresse.get(&self.entry)
    }

    fn descendre(
        &self,
        fonction: usize,
        profondeur: usize,
        parent: Option<usize>,
        nœuds: &mut Vec<Nœud>,
        vus: &mut std::collections::HashSet<usize>,
    ) {
        if nœuds.len() >= MAX_NŒUDS {
            return;
        }
        let renvoi = !vus.insert(fonction);
        let moi = nœuds.len();
        nœuds.push(Nœud {
            fonction,
            profondeur,
            parent,
            renvoi,
        });
        // Un renvoi ne se redéploie pas, et l'on s'arrête à la profondeur
        // où l'arbre cesse d'apprendre.
        if renvoi || profondeur >= MAX_PROFONDEUR {
            return;
        }
        for cible in self.functions[fonction].calls.clone() {
            if let Some(&rang) = self.par_adresse.get(&cible) {
                self.descendre(rang, profondeur + 1, Some(moi), nœuds, vus);
            }
        }
    }

    /// Les instructions d'une fonction, prises dans le désassemblage trié.
    fn instructions<'a>(
        &self,
        ctx: &'a ReContext,
        f: &Function,
    ) -> &'a [namsdec_disasm::DecodedInstruction] {
        let début = ctx.disasm.partition_point(|i| i.address < f.start);
        let fin = ctx.disasm.partition_point(|i| i.address <= f.last);
        ctx.disasm.get(début..fin).unwrap_or(&[])
    }
}

impl ReModule for CallGraphModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Code
    }

    /// La structure du programme avant ses instructions — et un
    /// arbre se lit sans rien savoir.
    fn niveau(&self) -> crate::Niveau {
        crate::Niveau::Débutant
    }

    fn id(&self) -> &'static str {
        "call_graph"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Graphe d'appels"
        } else {
            "Call graph"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.entry = crate::elf_scan::scan(ctx.binary)
            .map(|elf| elf.entry)
            .or_else(|| ctx.disasm.first().map(|i| i.address))
            .unwrap_or(0);
        // Les appels dont la destination se CALCULE comptent comme les
        // autres, quand on a su la retrouver : chacun était une fonction
        // que la découverte ne voyait pas, et une arête perdue.
        let imports = crate::imports::scan(ctx.binary);
        let résolus: Vec<(u64, u64)> = crate::indirect::resolve(ctx.disasm, ctx.binary, &imports)
            .into_iter()
            .filter(|r| r.appel)
            .filter_map(|r| Some((r.at, r.target?)))
            .collect();
        self.functions = discover_with(ctx.disasm, self.entry, &résolus);
        self.main = crate::functions::main_of(ctx.disasm, self.entry);
        self.par_adresse = self
            .functions
            .iter()
            .enumerate()
            .map(|(rang, f)| (f.start, rang))
            .collect();
        // Les plus employées d'abord : ce sont les utilitaires et les
        // points d'entrée, par où l'on commence à lire.
        let ordre = by_importance(&self.functions);
        self.montrées = ordre
            .into_iter()
            .take(MAX_FONCTIONS)
            .filter_map(|f| self.par_adresse.get(&f.start).copied())
            .collect();
        // La fiche ouverte parlait d'une fonction d'un autre binaire :
        // ses adresses ne sont pas celles-ci.
        self.ouverte = None;
        self.aller_à_erreur = false;
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        let mut blocs = vec![Bloc::Texte(
            Fragment::dit(ctx.t(
                "Un programme n'est pas une liste d'instructions mais un réseau de \
                 fonctions qui s'appellent. Voici ce réseau, déplié depuis le point \
                 d'entrée : chaque boîte est une fonction, chaque trait un appel. On \
                 clique une boîte pour aller la voir dans le listing ; on déplie une \
                 fonction, plus bas, pour lire ses octets et ce qu'ils font.",
                "A program is not a list of instructions but a web of functions that \
                 call one another. Here is that web, unfolded from the entry point: \
                 each box is a function, each line a call. Click a box to go to it in \
                 the listing; unfold a function, below, to read its bytes and what \
                 they do.",
            ))
            .faible()
            .into(),
        )];

        if self.functions.is_empty() {
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Rien à tracer : aucun code n'est chargé.",
                    "Nothing to trace: no code is loaded.",
                ))
                .faible()
                .into(),
            ));
            return Some(Vue { blocs });
        }

        // « ALLER À » : une adresse tapée, et on y va dans le listing.
        //
        // Le texte en cours de frappe appartient à l'interface — voir
        // `vue::Champ` : le module ne l'apprend qu'à la validation, ce
        // qui lui permet de décrire son panneau sans emprunt mutable.
        blocs.push(Bloc::Champ(
            Champ::pour(ALLER)
                .étiquette(ctx.t("Aller à :", "Go to:"))
                .indice("0x401000")
                .code()
                .bouton(Fragment::dit(ctx.t("Y aller", "Go")).bouton(ALLER)),
        ));
        if self.aller_à_erreur {
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Cette adresse ne se lit pas : attendez un nombre, en hexadécimal \
                     (0x401000) ou en décimal.",
                    "That address does not read: a number is expected, in hex \
                     (0x401000) or decimal.",
                ))
                .faible()
                .into(),
            ));
        }

        blocs.push(Bloc::Trait);
        blocs.push(Bloc::Texte(
            Fragment::dit(ctx.t("L'arbre des appels", "The call tree"))
                .fort()
                .into(),
        ));
        let (nœuds, arêtes) = self.graphe(ctx);
        if nœuds.is_empty() {
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Pas de racine d'où partir : ni point d'entrée ni `main` connus.",
                    "No root to start from: neither entry point nor `main` is known.",
                ))
                .faible()
                .into(),
            ));
        } else {
            blocs.push(Bloc::Dessin(Dessin::Graphe { nœuds, arêtes }));
            if self.functions.len() > self.montrées.len() {
                blocs.push(Bloc::Texte(
                    Fragment::dit(format!(
                        "… {} {}",
                        self.functions.len() - self.montrées.len(),
                        ctx.t(
                            "fonctions de plus, moins employées, non tracées",
                            "more functions, less used, not traced"
                        )
                    ))
                    .faible()
                    .into(),
                ));
            }
        }

        blocs.push(Bloc::Trait);
        blocs.push(Bloc::Texte(
            Fragment::dit(ctx.t("Le détail des fonctions", "Function details"))
                .fort()
                .into(),
        ));
        blocs.push(Bloc::Texte(
            Fragment::dit(ctx.t(
                "Pour chaque fonction, ses octets et leur traduction. Un nom se \
                 clique pour ouvrir sa fiche.",
                "For each function, its bytes and their translation. Click a name to \
                 open its card.",
            ))
            .faible()
            .into(),
        ));
        for rang in &self.montrées {
            blocs.extend(self.fiche(ctx, *rang));
        }
        Some(Vue { blocs })
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let Some(vue) = self.vue(ctx) else {
            return;
        };
        let gestes = crate::vue_peinte::peindre_tout(ui, ctx, &vue);
        // Relevé pendant le dessin, posé après : la vue emprunte
        // `self`, et le module ne peut donc pas s'écrire dedans.
        if gestes.demande.is_some() {
            self.demande = gestes.demande;
        }
        for action in &gestes.actions {
            self.agir(action);
        }
    }
}

impl CallGraphModule {
    /// L'arbre, dit en nœuds et en arêtes : des VALEURS, et pas un
    /// dessin.
    ///
    /// Le placement n'est pas ici — ni le pas vertical, ni
    /// l'indentation, ni la largeur d'une boîte. Ce sont des pixels, et
    /// seule l'interface sait de quelle surface elle dispose et ce que
    /// mesure un de ses caractères. Ce que le module dit est la
    /// structure : qui est là, à quelle distance de la racine, et qui
    /// appelle qui.
    fn graphe(&self, ctx: &ReContext) -> (Vec<crate::vue::Nœud>, Vec<(usize, usize)>) {
        let arbre = self.arbre();
        let mut nœuds = Vec::with_capacity(arbre.len());
        let mut arêtes = Vec::new();
        for (rang, nœud) in arbre.iter().enumerate() {
            let f = &self.functions[nœud.fonction];
            nœuds.push(crate::vue::Nœud {
                stéréotype: match (nœud.renvoi, f.origin) {
                    // Le renvoi l'emporte : c'est ce qu'il faut savoir
                    // de cette boîte-là, déjà dépliée ailleurs.
                    (true, _) => Some(ctx.t("renvoi", "repeat").to_owned()),
                    (false, Origin::Entry) => Some(ctx.t("entrée", "entry").to_owned()),
                    (false, Origin::Main) => Some("main".to_owned()),
                    (false, Origin::Called) => None,
                },
                titre: ctx.display_name(f.start, &f.name()),
                corps: vec![
                    format!("{:#x} · {} {}", f.start, f.size(), ctx.t("octets", "bytes")),
                    format!(
                        "{} {} · {} {}",
                        f.callers,
                        ctx.t("appelants", "callers"),
                        f.calls.len(),
                        ctx.t("appels", "calls")
                    ),
                ],
                mène_à: Some(f.start),
                profondeur: nœud.profondeur,
                // UN TON, et non deux drapeaux : « fort » pour ce par
                // quoi on entre dans le programme, « faible » pour ce
                // qui est déjà déplié ailleurs. L'interface en fait un
                // cadre d'accent ou un texte estompé — c'est elle qui
                // connaît ses teintes.
                ton: match (nœud.renvoi, f.origin) {
                    (true, _) => Ton::Faible,
                    (false, Origin::Entry | Origin::Main) => Ton::Fort,
                    (false, Origin::Called) => Ton::Normal,
                },
            });
            if let Some(parent) = nœud.parent {
                arêtes.push((parent, rang));
            }
        }
        (nœuds, arêtes)
    }

    /// La fiche d'une fonction : son nom qu'on clique, et ce qu'elle
    /// porte quand elle est ouverte.
    ///
    /// UNE SEULE FICHE À LA FOIS, et c'est une mesure qui l'a décidé :
    /// le détail d'une fonction se compose de son pseudo-code, soit
    /// jusqu'à deux cents lignes décodées une à une. Les quarante
    /// fonctions montrées, bâties à chaque image, coûtent 2,6 ms en
    /// release et 8,5 ms en debug — sur un budget de 16,7 ms — pour du
    /// texte que personne ne regarde, puisque les fiches sont fermées
    /// au départ. Mesuré sur `croackpocalypse` : 7 443 lignes.
    ///
    /// C'est le même partage que dans `imports`, et pour une raison de
    /// lecture en plus : une liste de quarante fonctions toutes
    /// dépliées ne se parcourt pas.
    fn fiche(&self, ctx: &ReContext, rang: usize) -> Vec<Bloc> {
        let f = &self.functions[rang];
        let nom = ctx.display_name(f.start, &f.name());
        let ouverte = self.ouverte == Some(f.start);
        // Le nom, l'adresse et la taille : de quoi reconnaître la
        // fonction sans ouvrir sa fiche.
        let titre = Fragment::dit(format!(
            "{}   {:#010x}   {} {}",
            nom,
            f.start,
            f.size(),
            ctx.t("octets", "bytes"),
        ))
        .code()
        .agit(format!("{FICHE}{:#x}", f.start))
        .survol(if ouverte {
            ctx.t("Refermer cette fiche", "Close this card")
        } else {
            ctx.t(
                "Ouvrir : les octets de cette fonction, et ce qu'ils font",
                "Open: this function's bytes, and what they do",
            )
        });
        // Rebaptisée à la main : elle se distingue des fonctions encore
        // anonymes, dans une liste qui peut en compter des centaines.
        // Le FAIT est dit ici, la teinte est choisie par l'interface.
        let titre = if ctx.user_name(f.start).is_some() {
            titre.à_la_main()
        } else {
            titre
        };
        let mut blocs = vec![Bloc::Texte(titre.into())];
        if !ouverte {
            return blocs;
        }

        let mut dedans = vec![
            // Renommer : on comprend une fonction, on lui donne un nom.
            // « Défaut » rend son `sub_xxxx` — se dédire fait partie de
            // nommer.
            Bloc::Champ(
                Champ::pour(format!("{RENOMMER}{:#x}", f.start))
                    .étiquette(ctx.t("Nom :", "Name:"))
                    .valeur(nom)
                    .code()
                    .caractères(28)
                    .bouton(
                        Fragment::dit(ctx.t("Renommer", "Rename"))
                            .bouton(format!("{RENOMMER}{:#x}", f.start)),
                    )
                    .bouton(
                        Fragment::dit(ctx.t("Défaut", "Default"))
                            .bouton(format!("{DÉFAUT}{:#x}", f.start))
                            .survol(ctx.t("Rendre son nom d'origine", "Restore its original name")),
                    ),
            ),
            Bloc::Texte(
                Fragment::dit(format!("{:#010x}", f.start))
                    .code()
                    .mène_à(f.start)
                    .into(),
            ),
        ];
        let instrs = self.instructions(ctx, f);
        if instrs.is_empty() {
            dedans.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Aucune instruction désassemblée dans cette plage.",
                    "No disassembled instruction in this range.",
                ))
                .faible()
                .into(),
            ));
        } else {
            dedans.push(table_des_instructions(ctx, instrs));
            if instrs.len() > MAX_INSTRUCTIONS {
                dedans.push(Bloc::Texte(
                    Fragment::dit(format!(
                        "… {} {}",
                        instrs.len() - MAX_INSTRUCTIONS,
                        ctx.t("instructions de plus", "more instructions")
                    ))
                    .faible()
                    .into(),
                ));
            }
        }
        blocs.push(Bloc::Retrait(dedans));
        blocs
    }

    /// Ce que le panneau fait d'une action, sans rien peindre.
    ///
    /// À part de `ui()` pour être éprouvé : c'est le comportement, et
    /// le vérifier en relevant des galées de texte revenait à mesurer
    /// la fenêtre pour savoir ce que le module retient.
    fn agir(&mut self, action: &str) {
        if let Some(adresse) = action.strip_prefix(ALLER).and_then(|r| r.strip_prefix(':')) {
            match lire_adresse(adresse) {
                // Une adresse TAPÉE dans ce champ, et non une adresse
                // cliquée : on la cherche dans le code, c'est l'objet
                // de ce panneau.
                Some(addr) => {
                    self.demande = Some(ReRequest::GoTo {
                        addr,
                        target: crate::GoTarget::Listing,
                    });
                    self.aller_à_erreur = false;
                }
                None => self.aller_à_erreur = !adresse.trim().is_empty(),
            }
            return;
        }
        if let Some(adresse) = action.strip_prefix(FICHE) {
            let Some(addr) = lire_adresse(adresse) else {
                return;
            };
            // Un second clic referme : c'est la même touche qui ouvre
            // et qui ferme, comme dans les autres panneaux.
            self.ouverte = (self.ouverte != Some(addr)).then_some(addr);
            return;
        }
        if let Some(reste) = action.strip_prefix(RENOMMER) {
            // L'adresse, puis le nom — et le nom garde ses deux-points
            // s'il en porte : on ne coupe qu'une fois.
            let Some((adresse, nom)) = reste.split_once(':') else {
                return;
            };
            if let Some(addr) = lire_adresse(adresse) {
                self.demande = Some(ReRequest::Rename {
                    addr,
                    name: nom.to_owned(),
                });
            }
            return;
        }
        if let Some(adresse) = action.strip_prefix(DÉFAUT)
            && let Some(addr) = lire_adresse(adresse)
        {
            // Un nom vide EST la demande de rendre le nom d'origine.
            self.demande = Some(ReRequest::Rename {
                addr,
                name: String::new(),
            });
        }
    }
}

/// La table de code d'une fonction : par instruction, les octets puis
/// le pseudo-code.
///
/// Trois colonnes : à gauche l'adresse et les octets bruts — le **code
/// binaire**, ce qui est vraiment dans le fichier —, à droite ce que
/// l'instruction fait, en clair. C'est la même ligne lue deux fois :
/// telle que la machine la reçoit, puis telle qu'on la comprend.
fn table_des_instructions(ctx: &ReContext, instrs: &[namsdec_disasm::DecodedInstruction]) -> Bloc {
    Bloc::Tableau {
        colonnes: vec![
            Fragment::dit(ctx.t("adresse", "address")).faible().into(),
            Fragment::dit(ctx.t("octets", "bytes")).faible().into(),
            Fragment::dit(ctx.t("pseudo-code", "pseudo-code"))
                .faible()
                .into(),
        ],
        rangées: instrs
            .iter()
            .take(MAX_INSTRUCTIONS)
            .map(|instr| {
                // Le pseudo-code quand on sait le rendre, l'instruction
                // elle-même sinon : mieux vaut le NASM que rien.
                let pseudo = namsdec_disasm::decode_one(&instr.bytes, instr.address)
                    .map(|d| namsdec_disasm::pseudo::of(&d, ctx.lang))
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(|| instr.text.clone());
                vec![
                    Fragment::dit(format!("{:#010x}", instr.address))
                        .code()
                        .into(),
                    Fragment::dit(octets_hex(&instr.bytes)).code().into(),
                    Fragment::dit(pseudo).code().into(),
                ]
            })
            .collect(),
        rayé: true,
    }
}

/// Les octets d'une instruction, en hexadécimal, espacés par deux.
fn octets_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lit une adresse tapée à la main : hexadécimale (`0x…`) ou décimale.
fn lire_adresse(texte: &str) -> Option<u64> {
    let t = texte.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        // Un nombre nu : décimal d'abord, car c'est ce qu'on tape quand on
        // en écrit un ; hexadécimal en repli, car une adresse l'est
        // souvent même sans le `0x`.
        t.parse::<u64>()
            .ok()
            .or_else(|| u64::from_str_radix(t, 16).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn contexte<'a>(
        disasm: &'a [namsdec_disasm::DecodedInstruction],
        cpu: &'a Cpu,
    ) -> ReContext<'a> {
        ReContext {
            cpu,
            disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        }
    }

    /// Une adresse se lit en hexadécimal avec ou sans `0x`, et en décimal.
    #[test]
    fn an_address_reads_in_hex_or_decimal() {
        assert_eq!(lire_adresse("0x401000"), Some(0x401000));
        assert_eq!(lire_adresse("  0X10 "), Some(0x10));
        assert_eq!(lire_adresse("4096"), Some(4096));
        // Sans `0x`, un nombre qui n'est pas décimal se relit en hexa.
        assert_eq!(lire_adresse("ff"), Some(0xff));
        assert_eq!(lire_adresse("zzz"), None);
        assert_eq!(lire_adresse(""), None);
    }

    /// L'arbre part de la racine, descend par les appels, et reste fini
    /// même quand une fonction s'appelle elle-même.
    #[test]
    fn the_tree_unfolds_from_the_root_and_stays_finite() {
        // Deux fonctions : à 0x1000, un `call` vers 0x1000 lui-même
        // (récursion), puis `ret`. L'arbre ne doit pas boucler.
        // e8 .. .. .. .. = call rel32 ; c3 = ret.
        let bytes: &[u8] = &[0xE8, 0xFB, 0xFF, 0xFF, 0xFF, 0xC3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = contexte(&disasm, &cpu);
        let mut module = CallGraphModule::default();
        module.on_binary_loaded(&ctx);
        let arbre = module.arbre();
        assert!(!arbre.is_empty(), "la racine au moins doit être là");
        // La récursion est marquée d'un renvoi, pas redéployée.
        assert!(
            arbre.iter().any(|n| n.renvoi),
            "un appel récursif devient un renvoi"
        );
        assert!(arbre.len() <= MAX_NŒUDS);
    }

    /// Un nom donné par l'utilisateur prend la place du nom par défaut,
    /// et l'absence de table rend le nom par défaut.
    #[test]
    fn a_user_name_replaces_the_default() {
        let mut noms = std::collections::BTreeMap::new();
        noms.insert(0x401000u64, "vérifie_clé".to_owned());
        let bytes: &[u8] = &[0x90, 0xc3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x401000);
        let cpu = Cpu::new(Memory::new(0x401000, 0x100));
        let avec = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: Some(&noms),
            patches: &[],
        };
        assert_eq!(avec.display_name(0x401000, "sub_401000"), "vérifie_clé");
        // Une adresse non nommée garde son défaut, même quand la table existe.
        assert_eq!(avec.display_name(0x402000, "sub_402000"), "sub_402000");
        // `user_name` dit s'il y a un nom donné — c'est ce qui sert à le
        // montrer partout (via `clickable_address`) et à le mettre en
        // évidence. Nommé : Some ; anonyme ou sans table : None.
        assert_eq!(avec.user_name(0x401000), Some("vérifie_clé"));
        assert_eq!(avec.user_name(0x402000), None);
        // Sans table, le défaut partout.
        let sans = ReContext {
            names: None,
            ..avec
        };
        assert_eq!(sans.display_name(0x401000, "sub_401000"), "sub_401000");
        assert_eq!(sans.user_name(0x401000), None);
    }

    /// Les instructions d'une fonction sont prises dans sa plage, et pas
    /// au-delà.
    #[test]
    fn a_functions_instructions_are_bounded_to_its_range() {
        let bytes: &[u8] = &[0x90, 0x90, 0xC3]; // nop ; nop ; ret
        let disasm = namsdec_disasm::disassemble(bytes, 0x2000);
        let cpu = Cpu::new(Memory::new(0x2000, 0x100));
        let ctx = contexte(&disasm, &cpu);
        let module = CallGraphModule::default();
        let f = Function {
            start: 0x2000,
            last: 0x2001,
            callers: 0,
            calls: vec![],
            origin: Origin::Called,
        };
        let instrs = module.instructions(&ctx, &f);
        // 0x2000 et 0x2001 appartiennent à la fonction, 0x2002 (ret) non.
        assert_eq!(instrs.len(), 2, "deux instructions dans [0x2000, 0x2001]");
        assert_eq!(instrs[0].address, 0x2000);
    }
    /// UNE FICHE FERMÉE NE PORTE AUCUNE LIGNE DE PSEUDO-CODE.
    ///
    /// C'est l'invariant qui retient la mesure : bâtir le détail des
    /// quarante fonctions montrées à chaque image coûte 2,6 ms en
    /// release et 8,5 ms en debug — sur un budget de 16,7 ms —, et les
    /// fiches sont fermées au départ. Le panneau ne compose donc que
    /// celle qui est ouverte.
    ///
    /// Le contrôle compte les TABLEAUX de la vue : il y en a un par
    /// fiche ouverte, et zéro sinon.
    #[test]
    fn a_closed_card_carries_no_pseudo_code() {
        // Un `call` vers soi-même puis `ret` : une fonction trouvée,
        // donc une fiche à ouvrir.
        let bytes: &[u8] = &[0xE8, 0xFB, 0xFF, 0xFF, 0xFF, 0xC3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = contexte(&disasm, &cpu);
        let mut module = CallGraphModule::default();
        module.on_binary_loaded(&ctx);
        assert!(!module.montrées.is_empty(), "une fonction au moins");

        let tableaux = |module: &CallGraphModule| {
            fn compte(blocs: &[Bloc]) -> usize {
                blocs
                    .iter()
                    .map(|bloc| match bloc {
                        Bloc::Tableau { .. } => 1,
                        Bloc::Retrait(dedans) => compte(dedans),
                        Bloc::Section { blocs, .. } | Bloc::Repliable { blocs, .. } => {
                            compte(blocs)
                        }
                        _ => 0,
                    })
                    .sum()
            }
            compte(&module.vue(&ctx).expect("une vue").blocs)
        };
        assert_eq!(tableaux(&module), 0, "rien d'ouvert, rien de composé");

        // Ouverte : un tableau, et un seul.
        let adresse = module.functions[module.montrées[0]].start;
        module.agir(&format!("{FICHE}{adresse:#x}"));
        assert_eq!(module.ouverte, Some(adresse));
        assert_eq!(tableaux(&module), 1);

        // Le même clic referme : c'est la même touche qui ouvre et qui
        // ferme.
        module.agir(&format!("{FICHE}{adresse:#x}"));
        assert_eq!(module.ouverte, None);
        assert_eq!(tableaux(&module), 0);
    }

    /// Le champ « aller à » dit où il mène, et dit quand il ne sait pas
    /// lire.
    ///
    /// Le texte tapé vient de l'interface, collé à l'action : c'est
    /// tout ce que le module en voit, et il doit en tirer les deux cas.
    #[test]
    fn the_go_to_field_reads_an_address_or_says_it_cannot() {
        let mut module = CallGraphModule::default();
        module.agir(&format!("{ALLER}:0x401136"));
        assert!(matches!(
            module.take_request(),
            Some(ReRequest::GoTo {
                addr: 0x401136,
                target: crate::GoTarget::Listing
            })
        ));
        assert!(!module.aller_à_erreur);

        module.agir(&format!("{ALLER}:pas une adresse"));
        assert!(module.take_request().is_none(), "rien à demander");
        assert!(module.aller_à_erreur, "et la raison s'écrit sous le champ");

        // Un champ vide n'est pas une erreur : on n'a rien demandé.
        module.agir(&format!("{ALLER}:   "));
        assert!(!module.aller_à_erreur);
    }

    /// Renommer porte le nom tapé ; « Défaut » porte le nom vide.
    ///
    /// Le nom arrive derrière l'adresse, séparé par un deux-points — et
    /// il garde les SIENS : on ne coupe qu'une fois, sans quoi un nom
    /// comme `std::vector::push` arriverait tronqué.
    #[test]
    fn renaming_carries_the_typed_name_and_default_carries_none() {
        let mut module = CallGraphModule::default();
        module.agir(&format!("{RENOMMER}0x401136:vérifie_clé"));
        match module.take_request() {
            Some(ReRequest::Rename { addr, name }) => {
                assert_eq!(addr, 0x401136);
                assert_eq!(name, "vérifie_clé");
            }
            autre => panic!("une demande de renommage, pas {autre:?}"),
        }

        module.agir(&format!("{RENOMMER}0x401136:std::vector::push"));
        match module.take_request() {
            Some(ReRequest::Rename { name, .. }) => assert_eq!(name, "std::vector::push"),
            autre => panic!("le nom garde ses deux-points, pas {autre:?}"),
        }

        // « Défaut » : un nom VIDE est la demande de rendre l'original.
        module.agir(&format!("{DÉFAUT}0x401136"));
        match module.take_request() {
            Some(ReRequest::Rename { addr, name }) => {
                assert_eq!(addr, 0x401136);
                assert!(name.is_empty());
            }
            autre => panic!("un renommage à nom vide, pas {autre:?}"),
        }
    }

    /// Le graphe dit des nœuds, des arêtes et des TONS — pas un dessin.
    ///
    /// La racine se lit en premier (« fort »), un renvoi s'estompe
    /// (« faible »), et une arête relie un rang de parent à un rang
    /// d'enfant. Rien de tout cela ne mentionne un pixel : c'est ce qui
    /// permet au même graphe d'être peint, exporté ou lu.
    #[test]
    fn the_graph_says_nodes_edges_and_tones() {
        let bytes: &[u8] = &[0xE8, 0xFB, 0xFF, 0xFF, 0xFF, 0xC3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = contexte(&disasm, &cpu);
        let mut module = CallGraphModule::default();
        module.on_binary_loaded(&ctx);
        let (nœuds, arêtes) = module.graphe(&ctx);
        assert!(nœuds.len() >= 2, "la racine et son renvoi : {nœuds:?}");
        assert_eq!(nœuds[0].profondeur, 0);
        assert_eq!(nœuds[0].ton, Ton::Fort, "on entre par là");
        assert_eq!(
            nœuds[0].mène_à,
            Some(0x1000),
            "une boîte mène à sa fonction"
        );
        // Deux lignes de corps : l'adresse et la taille, puis les
        // appelants et les appels. C'est ce que la boîte mesure.
        assert_eq!(nœuds[0].corps.len(), 2);
        let renvoi = nœuds
            .iter()
            .find(|n| n.stéréotype.as_deref() == Some("renvoi"))
            .expect("l'appel récursif devient un renvoi");
        assert_eq!(renvoi.ton, Ton::Faible, "déjà déplié ailleurs");
        assert_eq!(arêtes, vec![(0, 1)], "la racine appelle son renvoi");
    }

    /// Le nom qu'on a donné soi-même est dit comme tel.
    ///
    /// Un FAIT dans la vue — « ceci vient de votre main » — et non une
    /// couleur : c'est l'interface qui en fait un fond léger. Sans lui,
    /// une fonction rebaptisée se perd dans une liste de quarante.
    #[test]
    fn a_hand_given_name_says_it_comes_from_the_hand() {
        let bytes: &[u8] = &[0x90, 0xC3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x401000);
        let cpu = Cpu::new(Memory::new(0x401000, 0x100));
        let mut noms = std::collections::BTreeMap::new();
        noms.insert(0x401000u64, "vérifie_clé".to_owned());
        let cpu2 = Cpu::new(Memory::new(0x401000, 0x100));
        let sans = contexte(&disasm, &cpu);
        let avec = ReContext {
            names: Some(&noms),
            cpu: &cpu2,
            ..contexte(&disasm, &cpu2)
        };
        let mut module = CallGraphModule::default();
        module.on_binary_loaded(&sans);

        let à_la_main = |ctx: &ReContext| {
            module
                .vue(ctx)
                .expect("une vue")
                .blocs
                .iter()
                .filter_map(|bloc| match bloc {
                    Bloc::Texte(ligne) => Some(ligne.fragments.iter().any(|f| f.à_la_main)),
                    _ => None,
                })
                .any(|oui| oui)
        };
        assert!(!à_la_main(&sans), "aucun nom donné : rien à distinguer");
        assert!(à_la_main(&avec), "le nom donné se distingue");
    }
}
