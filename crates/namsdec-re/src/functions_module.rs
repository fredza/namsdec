//! Le module « Fonctions » : par où commencer à lire.
//!
//! Devant un binaire dépouillé, la première question n'est pas « que
//! fait cette instruction » mais « où est le programme ». Deux cent
//! quarante kilo-octets de `.text` dont la quasi-totalité appartient à
//! la bibliothèque standard : ce qu'on cherche tient en quelques
//! centaines d'octets, et rien ne dit lesquels.
//!
//! Ce module répond en trois temps : le `main`, retrouvé par la
//! convention de démarrage ; les fonctions qu'on sait nommer ; et le
//! nombre d'appels qui mènent à chacune, parce qu'une fonction appelée
//! de quarante endroits est un utilitaire, et que celle qui n'est
//! appelée qu'une fois est le programme.

use crate::functions::{Function, by_importance, discover_with};
use crate::vue::{Bloc, Champ, Fragment, Vue};
use crate::{ReContext, ReModule, ReRequest, ReSection};

/// Ce que le panneau demande, par son nom.
const ACTION_OUVRIR: &str = "fonctions:nommer:";
const ACTION_RENOMMER: &str = "fonctions:nom:";
const ACTION_DÉFAIRE: &str = "fonctions:défaire:";

/// La largeur du champ du nom, EN CARACTÈRES.
///
/// Le module calculait des pixels : `(place - 340).clamp(70, 220)`,
/// avec une branche « étroit » qui retranchait quatre-vingts de plus.
/// Trois nombres pour une seule question — combien de lettres doit-on
/// pouvoir lire ? Vingt-six : `__libc_csu_init` en fait quinze,
/// `vérifie_le_mot_de_passe` vingt-trois, et au-delà l'infobulle rend
/// le reste. L'interface sait ce que mesurent ses caractères.
const NOM_EN_CARACTÈRES: usize = 26;

#[derive(Default)]
pub struct FunctionsModule {
    found: Vec<Function>,
    main: Option<u64>,
    entry: u64,
    /// Ce que le panneau demande à l'application : aller voir une
    /// adresse, ou rebaptiser une fonction. Relevé après le dessin.
    demande: Option<ReRequest>,
    /// La fonction dont on est en train de taper le nom.
    ///
    /// Une seule à la fois : une liste de quarante champs de saisie
    /// pèse, et un nom long y serait coupé sans qu'on puisse le lire.
    /// Au repos, le nom est une étiquette qu'on clique pour la changer.
    en_édition: Option<u64>,
}

/// Combien de fonctions la liste montre avant de s'arrêter.
///
/// Trois cents lignes ne se lisent pas : on montre les plus employées,
/// et le compte total dit ce qu'on ne montre pas.
const MONTRÉES: usize = 40;

impl FunctionsModule {
    /// Ce que le panneau fait de ce qu'on lui demande.
    ///
    /// À PART DU DESSIN, et c'est ce que le portage a dégagé : ouvrir
    /// la saisie, poser un nom et se dédire sont trois comportements,
    /// qui s'éprouvent maintenant sans peindre une image.
    fn agit(&mut self, action: &str, ctx: &ReContext) {
        if let Some(addr) = action
            .strip_prefix(ACTION_OUVRIR)
            .and_then(|a| a.parse::<u64>().ok())
        {
            self.en_édition = Some(addr);
            return;
        }
        if let Some(addr) = action
            .strip_prefix(ACTION_DÉFAIRE)
            .and_then(|a| a.parse::<u64>().ok())
        {
            // Un nom vide rend son `sub_xxxx` : c'est ainsi que
            // l'application efface une note.
            self.demande = Some(ReRequest::Rename {
                addr,
                name: String::new(),
            });
            self.en_édition = None;
            return;
        }
        let Some(reste) = action.strip_prefix(ACTION_RENOMMER) else {
            return;
        };
        // « fonctions:nom:4198400:vérifie_mdp » : l'adresse, puis ce
        // qui a été tapé. Un nom peut contenir un deux-points, d'où la
        // coupe à la PREMIÈRE occurrence seulement.
        let Some((adresse, nom)) = reste.split_once(':') else {
            return;
        };
        let Ok(addr) = adresse.parse::<u64>() else {
            return;
        };
        self.en_édition = None;
        // Rien à poser si le nom n'a pas bougé : sans cette garde,
        // ouvrir puis valider sans rien changer écrirait une note qui
        // répète le nom d'origine, et la fonction passerait pour
        // rebaptisée.
        let défaut = self
            .found
            .iter()
            .find(|f| f.start == addr)
            .map(|f| f.name())
            .unwrap_or_default();
        if nom == ctx.display_name(addr, &défaut) {
            return;
        }
        self.demande = Some(ReRequest::Rename {
            addr,
            name: nom.to_owned(),
        });
    }
}

impl ReModule for FunctionsModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Code
    }

    /// Par où commencer à lire du code : la liste des fonctions
    /// et leur importance.
    fn niveau(&self) -> crate::Niveau {
        crate::Niveau::Débutant
    }

    fn id(&self) -> &'static str {
        "functions"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Fonctions"
        } else {
            "Functions"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        // Le point d'entrée déclaré par le fichier, quand on l'a : c'est
        // de lui qu'on remonte jusqu'à `main`.
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
        self.found = discover_with(ctx.disasm, self.entry, &résolus);
        self.main = crate::functions::main_of(ctx.disasm, self.entry);
        // Un autre binaire, d'autres fonctions : les noms en cours de
        // frappe ne parlent plus de rien.
        self.en_édition = None;
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        if self.found.is_empty() {
            return Some(Vue::neuve().texte(ctx.t(
                "Rien à découper : aucun code n'est chargé.",
                "Nothing to carve up: no code is loaded.",
            )));
        }

        let mut blocs = match self.main {
            Some(main) => vec![
                Bloc::Texte(
                    vec![
                        Fragment::dit(ctx.t("Le programme commence à", "The program starts at")),
                        Fragment::dit(format!("{main:#010x}")).code(),
                    ]
                    .into(),
                ),
                Bloc::Texte(
                    Fragment::dit(ctx.t(
                        "Trouvé par la convention de démarrage : le code d'amorçage passe \
                         l'adresse de `main` à `__libc_start_main`, dans RDI. C'est une \
                         convention, pas une loi — un programme peut s'en passer.",
                        "Found through the startup convention: the bootstrap code hands the \
                         address of `main` to `__libc_start_main`, in RDI. That is a \
                         convention, not a law — a program may do without it.",
                    ))
                    .faible()
                    .into(),
                ),
            ],
            None => vec![Bloc::Texte(
                ctx.t(
                    "Le `main` n'a pas été retrouvé : ce programme ne suit pas la \
                     convention de démarrage de la bibliothèque C.",
                    "No `main` was found: this program does not follow the C library's \
                     startup convention.",
                )
                .into(),
            )],
        };
        blocs.push(Bloc::Texte(
            Fragment::dit(ctx.compte(
                self.found.len(),
                (
                    "fonction retrouvée — les cibles d'appel, et rien d'autre : ce qu'on \
                     n'appelle que par pointeur échappe à ce relevé.",
                    "fonctions retrouvées — les cibles d'appel, et rien d'autre : ce qu'on \
                     n'appelle que par pointeur échappe à ce relevé.",
                ),
                (
                    "function recovered — call targets, and nothing else: what is only \
                     called through a pointer escapes this survey.",
                    "functions recovered — call targets, and nothing else: what is only \
                     called through a pointer escapes this survey.",
                ),
            ))
            .faible()
            .into(),
        ));

        // RENOMMER SE FAIT ICI, et le nom donné vaut partout : c'est la
        // même table que le listing, l'arbre d'appels et les références
        // croisées — les « notes » du binaire, gardées d'une séance à
        // l'autre. Comprendre ce que fait `sub_1d491` et ne pas pouvoir
        // l'appeler `vérifie_mdp` dans la liste même des fonctions
        // était le défaut : il fallait passer par l'arbre d'appels pour
        // nommer ce qu'on lisait ici.
        blocs.push(Bloc::Texte(
            Fragment::dit(ctx.t(
                "Cliquez un nom pour le changer, et validez par Entrée. Il vaut partout — \
                 listing, arbre d'appels, références croisées — et survit à la fermeture ; \
                 la flèche de retour rend son nom d'origine.",
                "Click a name to change it, and press Enter. It holds everywhere — \
                 listing, call tree, cross-references — and survives closing; the back \
                 arrow restores the original one.",
            ))
            .faible()
            .into(),
        ));

        // UNE RANGÉE PAR FONCTION, ET NON UN TABLEAU, et la raison tient
        // toujours : dans une grille, egui donne à chaque cellule ce qui
        // reste de la largeur une fois les autres colonnes posées — le
        // champ du nom se retrouvait rogné à vingt-quatre pixels dès que
        // la fenêtre d'analyse était étroite. Une rangée, elle, donne à
        // chacun la largeur qu'on lui dit — et la vue la dit maintenant
        // en CARACTÈRES.
        for fonction in by_importance(&self.found).into_iter().take(MONTRÉES) {
            let défaut = fonction.name();
            let nom = ctx.display_name(fonction.start, &défaut);
            let rebaptisée = ctx.user_name(fonction.start).is_some();
            // Se dédire fait partie de nommer : la flèche rend son
            // `sub_xxxx` à la fonction. Éteinte quand il n'y a rien à
            // défaire.
            let défaire = Fragment::dit("\u{21ba}")
                .bouton(format!("{ACTION_DÉFAIRE}{}", fonction.start))
                .inactif_si(!rebaptisée)
                .survol(ctx.t("Rendre son nom d'origine", "Restore its original name"));
            // L'adresse mène au listing, comme partout ailleurs : lire
            // « 0x1d491 » sans pouvoir y aller était s'arrêter au milieu
            // du geste.
            let adresse = Fragment::dit(format!("{:#x}", fonction.start))
                .code()
                .mène_à(fonction.start);
            let taille =
                Fragment::dit(format!("{} {}", fonction.size(), ctx.t("octets", "bytes"))).faible();
            let appels = Fragment::dit(format!(
                "{} {}",
                fonction.callers,
                if fonction.callers > 1 {
                    ctx.t("appels", "calls")
                } else {
                    ctx.t("appel", "call")
                }
            ))
            .faible();

            if self.en_édition == Some(fonction.start) {
                blocs.push(Bloc::Champ(
                    Champ::pour(format!("{ACTION_RENOMMER}{}", fonction.start))
                        .valeur(&nom)
                        .code()
                        .caractères(NOM_EN_CARACTÈRES)
                        // Il vient de paraître parce qu'on a cliqué le
                        // nom : taper doit marcher tout de suite.
                        .clavier()
                        .bouton(défaire)
                        .bouton(adresse)
                        .bouton(taille)
                        .bouton(appels),
                ));
                continue;
            }
            // Au repos, le nom est une étiquette qu'on clique : quarante
            // champs de saisie dans une liste pèsent, et un nom long y
            // serait coupé sans qu'on puisse le lire.
            let mut étiquette = Fragment::dit(&nom)
                .code()
                .agit(format!("{ACTION_OUVRIR}{}", fonction.start))
                .survol(format!(
                    "{nom}\n{}",
                    ctx.t(
                        "Cliquez pour renommer — le nom vaut partout, et survit à la \
                         fermeture",
                        "Click to rename — the name holds everywhere, and survives closing",
                    )
                ));
            if rebaptisée {
                étiquette = étiquette.à_la_main();
            }
            blocs.push(Bloc::Texte(
                vec![étiquette, défaire, adresse, taille, appels].into(),
            ));
        }
        if self.found.len() > MONTRÉES {
            blocs.push(Bloc::Texte(
                Fragment::dit(format!(
                    "… {} {}",
                    self.found.len() - MONTRÉES,
                    ctx.t("de plus", "more")
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
        if gestes.demande.is_some() {
            self.demande = gestes.demande;
        }
        for action in &gestes.actions {
            self.agit(action, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// Renommer suit le chemin entier, et sans peindre une image.
    ///
    /// **C'EST CE QUE LE PORTAGE A DÉGAGÉ.** Ouvrir la saisie, poser
    /// un nom et se dédire vivaient dans `ui()`, mêlés au tampon de
    /// frappe, au focus et à trois calculs de pixels : on ne pouvait
    /// les éprouver qu'en rendant une image et en comptant des formes
    /// de texte. Ce sont des comportements, et ils se lisent
    /// maintenant dans la vue.
    ///
    /// LES ACTIONS SONT PROPOSÉES PAR LA VUE, jamais inventées ici :
    /// le contrôle tombe si un nom change d'un seul côté.
    #[test]
    fn renommer_se_fait_sans_peindre() {
        // deux `call` : deux fonctions, et de quoi avoir une liste.
        let bytes: &[u8] = &[
            0xE8, 0x06, 0x00, 0x00, 0x00, // call +6
            0xE8, 0x06, 0x00, 0x00, 0x00, // call +6
            0xC3, // ret
            0x90, 0xC3, // nop ; ret   (première cible)
            0x90, 0x90, 0xC3, // seconde cible
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = FunctionsModule::default();
        module.on_binary_loaded(&ctx);
        assert!(!module.found.is_empty(), "il y a des fonctions à nommer");

        /// Les champs de la vue, et les fragments qui agissent.
        fn champs(module: &FunctionsModule, ctx: &ReContext) -> Vec<Champ> {
            module
                .vue(ctx)
                .expect("le panneau dit quelque chose")
                .blocs
                .iter()
                .filter_map(|bloc| match bloc {
                    Bloc::Champ(champ) => Some(champ.clone()),
                    _ => None,
                })
                .collect()
        }
        fn action_du_nom(module: &FunctionsModule, ctx: &ReContext, nom: &str) -> String {
            module
                .vue(ctx)
                .expect("le panneau dit quelque chose")
                .blocs
                .iter()
                .filter_map(|bloc| match bloc {
                    Bloc::Texte(ligne) => Some(ligne),
                    _ => None,
                })
                .flat_map(|ligne| &ligne.fragments)
                .find(|f| f.texte == nom)
                .and_then(|f| f.action.clone())
                .unwrap_or_else(|| panic!("« {nom} » se clique"))
        }

        // AU REPOS, AUCUN CHAMP : quarante champs de saisie dans une
        // liste pèsent, et un nom long y serait coupé.
        assert!(champs(&module, &ctx).is_empty(), "rien n'est en saisie");

        let première = module.found[0].start;
        let défaut = module.found[0].name();
        let ouvrir = action_du_nom(&module, &ctx, &défaut);
        module.agit(&ouvrir, &ctx);

        // UN SEUL CHAMP, et il prend le clavier : on vient de cliquer
        // pour écrire, taper doit marcher tout de suite.
        let ouverts = champs(&module, &ctx);
        assert_eq!(ouverts.len(), 1, "une seule saisie à la fois");
        assert!(ouverts[0].clavier, "le champ prend le clavier");
        assert_eq!(ouverts[0].valeur, défaut, "il part du nom en place");
        // LA RANGÉE RESTE ENTIÈRE : l'adresse mène toujours au listing
        // pendant qu'on tape, et la flèche qui se dédit est là.
        assert!(
            ouverts[0]
                .boutons
                .iter()
                .any(|f| f.mène_à == Some(première)),
            "l'adresse reste sur la rangée en saisie"
        );

        // VALIDER POSE LE NOM.
        module.agit(&format!("{}:vérifie_mdp", ouverts[0].action), &ctx);
        assert!(champs(&module, &ctx).is_empty(), "la saisie se referme");
        match module.take_request() {
            Some(ReRequest::Rename { addr, name }) => {
                assert_eq!(addr, première);
                assert_eq!(name, "vérifie_mdp");
            }
            autre => panic!("un renommage était attendu : {autre:?}"),
        }

        // ET VALIDER SANS RIEN CHANGER NE POSE RIEN. C'est ce qui fait
        // d'Échap un cas particulier de « valider » : la touche rend au
        // champ le nom que le module proposait, et la validation qui
        // suit n'a plus rien à écrire. Sans cette garde, ouvrir puis
        // renoncer marquerait la fonction comme rebaptisée.
        module.agit(&ouvrir, &ctx);
        let ouverts = champs(&module, &ctx);
        module.agit(&format!("{}:{défaut}", ouverts[0].action), &ctx);
        assert!(
            module.take_request().is_none(),
            "un nom inchangé n'est pas une note"
        );

        // SE DÉDIRE EFFACE LA NOTE : un nom vide rend son `sub_xxxx`.
        let défaire = module
            .vue(&ctx)
            .expect("le panneau dit quelque chose")
            .blocs
            .iter()
            .filter_map(|bloc| match bloc {
                Bloc::Texte(ligne) => Some(ligne),
                _ => None,
            })
            .flat_map(|ligne| &ligne.fragments)
            .find(|f| f.texte == "\u{21ba}")
            .and_then(|f| f.action.clone())
            .expect("la flèche se demande");
        module.agit(&défaire, &ctx);
        match module.take_request() {
            Some(ReRequest::Rename { name, .. }) => {
                assert!(name.is_empty(), "un nom vide rend celui d'origine")
            }
            autre => panic!("un effacement était attendu : {autre:?}"),
        }
    }

    /// Le module nomme le point de départ, et ne prétend rien quand la
    /// convention n'est pas suivie.
    #[test]
    fn the_module_names_where_to_start_or_says_it_cannot() {
        // Un démarrage en règle : lea rdi, [rip+0x10] puis call.
        let bytes: &[u8] = &[
            0xF3, 0x0F, 0x1E, 0xFA, 0x48, 0x8D, 0x3D, 0x10, 0x00, 0x00, 0x00, 0xFF, 0x15, 0x00,
            0x20, 0x00, 0x00, 0xF4,
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x1000);
        let cpu = Cpu::new(Memory::new(0x1000, 0x100));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = FunctionsModule::default();
        module.on_binary_loaded(&ctx);
        // `lea` à 0x1004, longue de sept octets : 0x100b + 0x10.
        assert_eq!(module.main, Some(0x101b));
        assert!(!module.found.is_empty());
    }
}
