//! Le panneau des références croisées : par où entrer dans un binaire.
//!
//! On y trouve les fonctions — les adresses qu'un `call` désigne, que
//! leurs noms aient été retirés ou non — classées par nombre
//! d'appelants, et pour chacune d'où elle est appelée. C'est le chemin
//! qu'on suit en lisant du code qu'on n'a pas écrit : commencer par ce
//! qui sert partout, puis remonter.

use crate::vue::{Bloc, Fragment, Vue};
use crate::xrefs::{self, Kind, Xrefs};
use crate::{ReContext, ReModule, ReRequest, ReSection};

/// Le préfixe de l'action qui déplie une cible : son adresse en
/// hexadécimal suit. Une constante, pour que l'écriture et la relecture
/// ne puissent pas diverger en silence.
const ACTION_DÉPLIER: &str = "xrefs:déplier:";

#[derive(Default)]
pub struct XrefsModule {
    xrefs: Xrefs,
    /// La cible dépliée, s'il y en a une.
    ouverte: Option<u64>,
    /// Ce que le panneau demande à l'application : aller voir une adresse.
    demande: Option<ReRequest>,
}

impl XrefsModule {
    /// Ouvre ou referme la cible que cette action désigne.
    ///
    /// À part de `ui()` pour être éprouvée sans peindre : c'est le
    /// comportement, pas le dessin.
    fn bascule(&mut self, action: &str) {
        let Some(hexa) = action.strip_prefix(ACTION_DÉPLIER) else {
            return;
        };
        let Ok(addr) = u64::from_str_radix(hexa, 16) else {
            return;
        };
        // Un second clic referme, et une seule à la fois : c'est ce que
        // `Option` dit déjà, et ce que le panneau faisait avant la
        // frontière.
        self.ouverte = (self.ouverte != Some(addr)).then_some(addr);
    }

    /// Les références relevées, pour qui veut les lire sans l'interface.
    pub fn xrefs(&self) -> &Xrefs {
        &self.xrefs
    }
}

impl ReModule for XrefsModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Code
    }

    fn id(&self) -> &'static str {
        "xrefs"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Références croisées"
        } else {
            "Cross-references"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        // La plage du programme : ce qui tombe dedans est une adresse,
        // le reste est un nombre.
        let range = match (ctx.disasm.first(), ctx.disasm.last()) {
            (Some(first), Some(last)) => {
                first.address..last.address + last.bytes.len() as u64 + 0x2000
            }
            _ => 0..0,
        };
        self.xrefs = xrefs::scan(ctx.disasm, range);
        self.ouverte = None;
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        if self.xrefs.is_empty() {
            return Some(
                Vue::neuve().texte(
                    Fragment::dit(ctx.t(
                        "Aucune référence : le programme n'appelle rien et ne pointe nulle part.",
                        "No references: this program calls nothing and points nowhere.",
                    ))
                    .faible(),
                ),
            );
        }
        let mut blocs = Vec::new();

        let fonctions = self.xrefs.functions();
        if !fonctions.is_empty() {
            blocs.push(Bloc::Texte(
                Fragment::dit(format!(
                    "{} ({})",
                    ctx.t("Fonctions appelées", "Called functions"),
                    fonctions.len()
                ))
                .fort()
                .into(),
            ));
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Une adresse qu'un `call` désigne est une fonction, même sans son nom. \
                     Cliquez l'adresse pour y aller ; le signe déplie d'où elle est appelée.",
                    "An address a `call` points at is a function, even without its name. \
                     Click the address to go there; the sign unfolds where it is called from.",
                ))
                .faible()
                .into(),
            ));
            for (addr, appels) in &fonctions {
                let ouverte = self.ouverte == Some(*addr);
                // Le signe déplie ; l'adresse mène. Deux gestes
                // distincts, car ce sont deux intentions : lire d'où
                // une fonction est appelée, ou aller la voir.
                // « − » et « + », et non deux triangles : ceux-ci
                // n'existent pas dans les polices qu'egui embarque et
                // se peignaient en carré vide, si bien que rien ne
                // disait que la ligne se dépliait.
                blocs.push(Bloc::Texte(
                    vec![
                        Fragment::dit(if ouverte { "\u{2212}" } else { "+" })
                            .agit(format!("{ACTION_DÉPLIER}{addr:x}"))
                            .survol(ctx.t("D'où est-ce appelé ?", "Called from where?")),
                        Fragment::dit(format!(" {addr:#010x}")).mène_à(*addr),
                        Fragment::dit(format!(
                            " {appels} {}",
                            if *appels > 1 {
                                ctx.t("appels", "calls")
                            } else {
                                ctx.t("appel", "call")
                            }
                        ))
                        .faible(),
                    ]
                    .into(),
                ));
                if ouverte {
                    blocs.push(Bloc::Retrait(
                        self.xrefs
                            .to(*addr)
                            .iter()
                            .map(|reference| {
                                Bloc::Texte(
                                    vec![
                                        Fragment::dit(match reference.kind {
                                            Kind::Call => ctx.t("appelé depuis ", "called from "),
                                            Kind::Jump => ctx.t("saut depuis ", "jumped from "),
                                            Kind::Data => ctx.t("lu depuis ", "read from "),
                                            Kind::Address => {
                                                ctx.t("adresse prise en ", "address taken at ")
                                            }
                                        })
                                        .faible(),
                                        // D'où vient l'appel est aussi
                                        // une adresse où l'on veut
                                        // aller : la lire sans pouvoir
                                        // l'atteindre serait s'arrêter
                                        // au milieu.
                                        Fragment::dit(format!("{:#010x}", reference.from))
                                            .mène_à(reference.from),
                                    ]
                                    .into(),
                                )
                            })
                            .collect(),
                    ));
                }
            }
        }

        // Les boucles : ce qui explique qu'un programme de dix lignes
        // exécute des milliers d'instructions.
        let boucles = self.xrefs.loops();
        if !boucles.is_empty() {
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.compte(boucles.len(), ("boucle", "boucles"), ("loop", "loops")))
                    .fort()
                    .into(),
            ));
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.t(
                    "Un saut qui revient en arrière ferme un chemin. Combien de fois on y \
                     tourne ne se sait qu'en exécutant : la gouttière le compte.",
                    "A jump going backwards closes a path. How many times one goes round is \
                     only known by running it: the gutter counts that.",
                ))
                .faible()
                .into(),
            ));
            blocs.extend(boucles.iter().map(|(cible, depuis)| {
                Bloc::Texte(
                    vec![
                        Fragment::dit(format!("{cible:#010x}")).mène_à(*cible),
                        Fragment::dit(ctx.t(" bouclée depuis ", " looped from ")).faible(),
                        Fragment::dit(format!("{depuis:#010x}")).mène_à(*depuis),
                    ]
                    .into(),
                )
            }));
        }

        // Les étiquettes : ce qu'on saute sans y entrer. Leur nombre
        // dit combien de boucles et de branches porte le programme.
        let étiquettes = self.xrefs.labels().len();
        if étiquettes > 0 {
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.compte(
                    étiquettes,
                    (
                        "étiquette — début de boucle ou de branche",
                        "étiquettes — débuts de boucle et de branche",
                    ),
                    (
                        "label — start of a loop or a branch",
                        "labels — starts of loops and branches",
                    ),
                ))
                .faible()
                .into(),
            ));
        }

        // Ce que le programme désigne sans l'appeler : ses données. Le
        // compte suffit ici — le détail est dans le panneau des
        // chaînes, qui les montre avec leur contenu.
        let données = self
            .xrefs
            .targets()
            .filter(|(_, refs)| {
                refs.iter()
                    .any(|r| matches!(r.kind, Kind::Data | Kind::Address))
            })
            .count();
        if données > 0 {
            blocs.push(Bloc::Texte(
                Fragment::dit(ctx.compte(
                    données,
                    (
                        "adresse de données désignée par le code",
                        "adresses de données désignées par le code",
                    ),
                    (
                        "data address pointed at by the code",
                        "data addresses pointed at by the code",
                    ),
                ))
                .faible()
                .into(),
            ));
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
            self.bascule(action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// Le dépliage d'une cible, éprouvé **sans peindre**.
    ///
    /// Le signe « + » était un `clickable_label` qui écrivait dans
    /// `self.ouverte` pendant le dessin ; il est devenu une action
    /// nommée que le module relit après coup. Ce contrôle suit le
    /// chemin entier : la vue propose l'action, la bascule l'applique,
    /// la vue suivante porte les appelants — et la même action referme.
    #[test]
    fn une_cible_se_deplie_par_son_action_et_se_referme_de_meme() {
        // Un appel et un retour : de quoi avoir une fonction appelée,
        // donc une cible dépliable. Assemblé ici en octets pour que ce
        // contrôle ne dépende d'aucun fichier de la machine.
        let octets = [
            0xe8, 0x01, 0x00, 0x00, 0x00, // call +1
            0xc3, // ret
            0xc3, // ret (la cible)
        ];
        let disasm = namsdec_disasm::disassemble(&octets, 0x400000);
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = XrefsModule::default();
        module.on_binary_loaded(&ctx);
        let Some((cible, _)) = module.xrefs.functions().first().copied() else {
            return; // rien d'appelé ici : il n'y a rien à déplier
        };

        // 1. La vue propose l'action, et le signe dit « fermé ».
        let action = format!("{ACTION_DÉPLIER}{cible:x}");
        assert!(
            textes(&module, &ctx).iter().any(|t| t == "+"),
            "le signe dit que la ligne se déplie : {:?}",
            textes(&module, &ctx)
        );

        // 2. La bascule l'applique : les appelants paraissent.
        module.bascule(&action);
        assert_eq!(module.ouverte, Some(cible));
        let ouverte = textes(&module, &ctx).join(" | ");
        assert!(
            ouverte.contains("appelé depuis"),
            "d'où la fonction est appelée : {ouverte}"
        );
        assert!(
            ouverte.contains('\u{2212}'),
            "et le signe dit maintenant « ouvert » : {ouverte}"
        );

        // 3. La même action referme.
        module.bascule(&action);
        assert_eq!(module.ouverte, None);
        assert!(
            !textes(&module, &ctx).join(" | ").contains("appelé depuis"),
            "un second clic referme"
        );
    }

    /// Tous les textes de la vue, dans l'ordre.
    fn textes(module: &XrefsModule, ctx: &ReContext) -> Vec<String> {
        fn dedans(blocs: &[Bloc], vus: &mut Vec<String>) {
            for bloc in blocs {
                match bloc {
                    Bloc::Texte(ligne) | Bloc::Attente(ligne) => {
                        vus.extend(ligne.fragments.iter().map(|f| f.texte.clone()))
                    }
                    Bloc::Retrait(dedans_là) => dedans(dedans_là, vus),
                    Bloc::Section { blocs, .. } | Bloc::Repliable { blocs, .. } => {
                        dedans(blocs, vus)
                    }
                    Bloc::Tableau { rangées, .. } => vus.extend(
                        rangées
                            .iter()
                            .flatten()
                            .flat_map(|l| &l.fragments)
                            .map(|f| f.texte.clone()),
                    ),
                    Bloc::Jauge { texte, .. } => {
                        vus.extend(texte.fragments.iter().map(|f| f.texte.clone()))
                    }
                    // Ni le code ni un dessin ne portent de fragments :
                    // leur contenu est la donnée, d'un bloc.
                    Bloc::Code(_) | Bloc::Dessin(_) | Bloc::Trait => {}
                }
            }
        }
        let mut vus = Vec::new();
        dedans(&module.vue(ctx).expect("une vue").blocs, &mut vus);
        vus
    }
}
