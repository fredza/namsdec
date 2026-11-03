use crate::behaviour::{self, Family};
use crate::vue::{Bloc, Fragment, Ligne, Ton, Vue};
use crate::{ReContext, ReModule, ReSection};
use iced_x86::{Mnemonic, OpKind, Register};

/// Les appels système que le programme contient, avec leur numéro et leur
/// nom.
///
/// Un `syscall` tout seul ne dit rien : c'est le RAX qui le précède qui
/// dit ce qu'il demande au noyau. Le module remonte donc les quelques
/// instructions d'avant pour trouver ce numéro — exactement ce qu'on fait
/// à la main en lisant un désassemblage, et ce qui raconte en une liste
/// ce que le programme fait du monde extérieur.
#[derive(Default)]
pub struct SyscallsModule {
    found: Vec<Call>,
    /// L'appel dont la fiche est dépliée, par son numéro.
    ///
    /// Un seul à la fois : la fiche fait une dizaine de lignes, et deux
    /// ouvertes repousseraient la liste hors du panneau.
    ouvert: Option<u64>,
}

/// Un `syscall` du programme : où il est, et ce qu'il demande — quand
/// le numéro se lit d'ici.
pub struct Call {
    pub addr: u64,
    pub number: Option<u64>,
}

impl Call {
    /// La famille de comportement de cet appel, quand son numéro se lit
    /// et que namsdec sait ce qu'il fait.
    pub fn family(&self) -> Option<Family> {
        behaviour::family_of_syscall(self.number?)
    }
}

/// Tous les appels système d'un désassemblage, avec leur numéro quand il
/// est écrit en clair juste avant.
///
/// Sorti du module pour servir ailleurs : la console a besoin de savoir
/// si le programme peut lire quelque chose avant de proposer un champ de
/// saisie à un programme qui ne lira jamais.
pub fn scan(disasm: &[namsdec_disasm::DecodedInstruction]) -> Vec<Call> {
    disasm
        .iter()
        .enumerate()
        .filter(|(_, instr)| instr.text.trim() == "syscall")
        .map(|(index, instr)| Call {
            addr: instr.address,
            number: number_before(disasm, index),
        })
        .collect()
}

/// Le programme peut-il lire son entrée ?
///
/// Vrai s'il contient un `read` (appel numéro zéro), et vrai aussi si un
/// appel a un numéro qu'on ne sait pas lire d'ici : mieux vaut proposer
/// une saisie inutile que la refuser à un programme qui l'attend.
pub fn reads_input(disasm: &[namsdec_disasm::DecodedInstruction]) -> bool {
    scan(disasm)
        .iter()
        .any(|call| matches!(call.number, None | Some(0)))
}

/// Combien d'instructions on remonte pour trouver le numéro. Au-delà,
/// c'est que RAX vient d'ailleurs — d'un calcul, d'un argument — et
/// deviner serait mentir.
const LOOK_BACK: usize = 12;

use crate::table::{is_simulated, syscall, syscall_name};

/// Ce que demande un clic sur le nom d'un appel : « syscalls:fiche:1 ».
///
/// Un NOM et non la demande elle-même, comme partout dans la vue : le
/// numéro suit le préfixe, et c'est le module — lui seul — qui sait que
/// « fiche » veut dire déplier.
const ACTION_FICHE: &str = "syscalls:fiche:";

impl SyscallsModule {
    /// Ouvre ou referme la fiche que cette action désigne.
    ///
    /// À PART DE `ui()` POUR ÊTRE ÉPROUVÉE SANS PEINDRE : c'est un
    /// comportement, pas un dessin, et le vérifier en comptant des
    /// galères de texte revenait à mesurer la fenêtre pour savoir ce
    /// que le module retient.
    fn bascule_fiche(&mut self, action: &str) {
        let Some(numéro) = action.strip_prefix(ACTION_FICHE) else {
            return;
        };
        let Ok(numéro) = numéro.parse::<u64>() else {
            return;
        };
        // Un second clic referme : c'est le geste qu'on attend d'un
        // repli, et il évite d'avoir à chercher une croix. Et une seule
        // fiche à la fois — la fiche fait une dizaine de lignes, et
        // deux ouvertes repousseraient la liste hors du panneau.
        self.ouvert = (self.ouvert != Some(numéro)).then_some(numéro);
    }
}

/// Le numéro chargé dans RAX juste avant `index`, s'il est écrit en clair.
fn number_before(disasm: &[namsdec_disasm::DecodedInstruction], index: usize) -> Option<u64> {
    let first = index.saturating_sub(LOOK_BACK);
    for earlier in disasm[first..index].iter().rev() {
        let instr = namsdec_disasm::decode_one(&earlier.bytes, earlier.address)?;
        let writes_rax = instr.op_count() > 0
            && instr.op_kind(0) == OpKind::Register
            && matches!(
                instr.op_register(0),
                Register::RAX | Register::EAX | Register::AX
            );
        if !writes_rax {
            continue;
        }
        // La dernière écriture de RAX avant l'appel décide. Si ce n'est
        // pas un immédiat — un calcul, une lecture mémoire —, le numéro
        // n'est pas lisible ici, et on le dit plutôt que de remonter plus
        // haut vers une valeur périmée.
        if instr.mnemonic() == Mnemonic::Mov && instr.op_count() > 1 {
            return match instr.op_kind(1) {
                OpKind::Immediate8
                | OpKind::Immediate16
                | OpKind::Immediate32
                | OpKind::Immediate64
                | OpKind::Immediate8to16
                | OpKind::Immediate8to32
                | OpKind::Immediate8to64
                | OpKind::Immediate32to64 => Some(instr.immediate(1)),
                _ => None,
            };
        }
        // `xor rax, rax` : l'idiome pour l'appel numéro zéro.
        if instr.mnemonic() == Mnemonic::Xor
            && instr.op_count() > 1
            && instr.op_kind(1) == OpKind::Register
            && instr.op_register(0) == instr.op_register(1)
        {
            return Some(0);
        }
        return None;
    }
    None
}

impl ReModule for SyscallsModule {
    fn section(&self) -> ReSection {
        ReSection::Borrows
    }

    /// Ce que le programme demande au noyau. Apprendre
    /// l'assembleur, c'est d'abord apprendre à demander.
    fn niveau(&self) -> crate::Niveau {
        crate::Niveau::Débutant
    }

    fn id(&self) -> &'static str {
        "syscalls"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Appels système"
        } else {
            "System calls"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.found = scan(ctx.disasm);
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        if self.found.is_empty() {
            return Some(Vue::neuve().texte(ctx.t(
                "Ce programme ne demande rien au système.",
                "This program asks nothing of the system.",
            )));
        }
        // Le portrait d'abord, comme pour les imports : la liste répond
        // « quels appels », le portrait répond « pour quoi faire ». Les
        // deux panneaux emploient la même ligne et les mêmes couleurs —
        // un `socket` est un `socket`, qu'il vienne du noyau ou de la
        // table de liaison. Et c'est maintenant LA MÊME FONCTION qui
        // les dit : `portrait_en_blocs` existait depuis le portage
        // d'`imports`, en attendant celui-ci.
        let comptes: Vec<(Family, usize)> = behaviour::ALL
            .into_iter()
            .filter_map(|famille| {
                let combien = self
                    .found
                    .iter()
                    .filter(|call| call.family() == Some(famille))
                    .count();
                (combien > 0).then_some((famille, combien))
            })
            .collect();
        let mut blocs = crate::imports_module::portrait_en_blocs(ctx, &comptes);
        for call in &self.found {
            let mut ligne = vec![Fragment::dit(format!("{:08x}", call.addr)).code()];
            match call.number {
                None => ligne.push(Fragment::dit(ctx.t(
                    "numéro calculé — illisible d'ici",
                    "number computed — not readable from here",
                ))),
                Some(number) => {
                    ligne.push(Fragment::dit(format!("{number}")).code());
                    // LE NOM EST UN LIEN : on le clique, sa fiche se
                    // déplie.
                    //
                    // « 00400016 1 write » est une ligne juste et
                    // muette. Elle suppose qu'on sache ce que `write`
                    // attend, dans quels registres, et ce qu'il rend —
                    // ce que personne ne sait au moment où il en a
                    // besoin. La fiche le dit, et le renvoi mène à la
                    // page de manuel qui fait autorité.
                    let nom = syscall_name(number).unwrap_or(ctx.t("inconnu", "unknown"));
                    ligne.push(
                        Fragment::dit(nom)
                            .fort()
                            .agit(format!("{ACTION_FICHE}{number}"))
                            .survol(ctx.t(
                                "Cliquer pour la fiche : ce qu'il attend, dans quel registre, \
                                 et ce qu'il rend",
                                "Click for the entry: what it expects, in which register, \
                                 and what it returns",
                            )),
                    );
                    if !is_simulated(number) {
                        ligne
                            .push(Fragment::dit(ctx.t("(non simulé)", "(not simulated)")).faible());
                    }
                    if let Some(famille) = call.family() {
                        ligne.push(
                            Fragment::dit(famille.label(ctx.lang))
                                .étiquette(famille.clé())
                                .survol(famille.why(ctx.lang)),
                        );
                    }
                }
            }
            blocs.push(Bloc::Texte(ligne.into()));
            if let Some(number) = call.number
                && self.ouvert == Some(number)
            {
                // EN RETRAIT, ET NON DANS UN CADRE. Le cadre de groupe
                // était le seul du crate : une forme à un seul usage,
                // et le vocabulaire s'arrête devant celles-là. Le
                // retrait dit la même chose — « ces lignes développent
                // la précédente » — et c'est déjà ce que la fiche d'une
                // fonction empruntée emploie, dans le panneau voisin
                // qui montre la même sorte de chose.
                blocs.push(Bloc::Retrait(fiche_en_blocs(ctx, number)));
            }
        }
        Some(Vue { blocs })
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let Some(vue) = self.vue(ctx) else {
            return;
        };
        let gestes = crate::vue_peinte::peindre_tout(ui, ctx, &vue);
        // Relevé pendant le dessin, agi après : la vue emprunte `self`,
        // et le module ne peut donc pas s'écrire dedans.
        for action in &gestes.actions {
            self.bascule_fiche(action);
        }
    }
}

/// Ce que namsdec sait de cet appel système, et où lire le reste.
///
/// Trois choses, et la troisième est celle qui manquait le plus :
///
/// 1. la **signature**, registres compris — `write(fd: RDI, buf: RSI,
///    count: RDX)`. En lisant un désassemblage, ce qui manque n'est pas
///    le nom des arguments mais le registre qui porte chacun ;
/// 2. ce que namsdec en **fait** — l'exécute vraiment, ou s'arrête ;
/// 3. le **renvoi** à la page de manuel qui fait autorité. Pas une
///    reformulation de plus : `man 2 write` est complet, à jour, et
///    namsdec n'a aucune raison de le recopier à moitié.
///
/// Dit et non peint, depuis P7 : un rapport reçoit la signature, les
/// registres et l'adresse de la page, ce qui est exactement ce qu'il
/// veut — et le lecteur n'a pas à croire namsdec sur parole.
fn fiche_en_blocs(ctx: &ReContext, number: u64) -> Vec<Bloc> {
    let Some(appel) = syscall(number) else {
        return vec![Bloc::Texte(
            Fragment::dit(ctx.t(
                "namsdec n'a pas de fiche pour ce numéro. La table ne porte que les appels \
                 qu'un programme d'apprentissage emploie et ceux qu'on croise en lisant un \
                 binaire : ce qui n'y figure pas n'est pas deviné.",
                "namsdec has no entry for this number. The table only holds the calls a \
                 learning program uses and those one meets while reading a binary: what is \
                 not in it is not guessed.",
            ))
            .faible()
            .into(),
        )];
    };
    let mut blocs = vec![Bloc::Texte(Fragment::dit(appel.signature()).code().into())];
    if !appel.args.is_empty() {
        let mut rangées: Vec<Vec<Ligne>> = Vec::new();
        for (rang, argument) in appel.args.iter().enumerate() {
            // Au-delà du sixième, il n'y a plus de registre :
            // `syscall` n'en passe pas davantage, et aucun appel de la
            // table n'en demande sept.
            let Some(registre) = crate::table::ARGUMENT_REGISTERS.get(rang) else {
                break;
            };
            rangées.push(vec![
                Fragment::dit(*argument).code().into(),
                // UN REPÈRE, et non un lien : le nom d'un registre ne
                // se clique pas, il désigne ce que le bandeau du
                // processeur montre au même instant.
                Fragment::dit(*registre).code().ton(Ton::Repère).into(),
            ]);
        }
        // Le retour passe par RAX, toujours, et c'est la moitié de la
        // convention : savoir où l'on met les arguments sans savoir où
        // lire la réponse ne sert qu'à moitié.
        rangées.push(vec![
            Fragment::dit(ctx.t("ce qu'il rend", "what it returns"))
                .code()
                .into(),
            Fragment::dit("RAX").code().ton(Ton::Repère).into(),
        ]);
        blocs.push(Bloc::Tableau {
            colonnes: Vec::new(),
            rangées,
            rayé: true,
        });
    }
    // Ce que namsdec en fait — c'est ce qui décide si l'exécution ira
    // plus loin que cette ligne.
    blocs.push(Bloc::Texte(if appel.simulated {
        Fragment::dit(ctx.t(
            "namsdec exécute celui-ci : l'exécution le traverse.",
            "namsdec runs this one: execution goes through it.",
        ))
        .faible()
        .into()
    } else {
        // UN VERDICT, ET NON UNE TEINTE. Ce module écrivait ici un
        // `Color32::from_rgb(0xd7, 0xba, 0x7d)` — un jaune choisi sur
        // place, que le thème ne connaît pas : sous le thème clair il
        // se lisait pâle sur blanc. Le module dit « attention »,
        // l'interface dit laquelle de ses couleurs porte ce mot.
        Fragment::dit(ctx.t(
            "namsdec ne simule PAS celui-ci : l'exécution s'arrêtera ici. Il n'y a pas \
             de vrai noyau derrière — ce qui est simulé l'est à la main, un par un.",
            "namsdec does NOT simulate this one: execution will stop here. There is no \
             real kernel behind it — what is simulated is simulated by hand, one at a \
             time.",
        ))
        .ton(Ton::Attention)
        .into()
    }));
    // LE RENVOI, et il mène à la page du NOYAU.
    //
    // La référence d'un appel système est `man-pages`, pas le manuel du
    // processeur : Intel décrit l'instruction `syscall`, qui ne dit rien
    // de ce que le numéro 1 demande. Pour une INSTRUCTION, c'est l'autre
    // renvoi qu'il faut, et il est dans la fiche d'instruction.
    blocs.push(Bloc::Texte(
        Fragment::dit(format!("man 2 {}", appel.name))
            .renvoi(appel.manual_url())
            .into(),
    ));
    blocs
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn context_of(bytes: &[u8]) -> Vec<namsdec_disasm::DecodedInstruction> {
        namsdec_disasm::disassemble(bytes, 0x400000)
    }

    #[test]
    fn the_number_comes_from_the_rax_just_before() {
        // mov eax, 1 / mov edi, 1 / syscall   puis   xor eax, eax / syscall
        let bytes: &[u8] = &[
            0xB8, 0x01, 0x00, 0x00, 0x00, 0xBF, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x05, 0x31, 0xC0,
            0x0F, 0x05,
        ];
        let disasm = context_of(bytes);
        let cpu = Cpu::new(Memory::new(0x400000, 0x10));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = SyscallsModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.found.len(), 2);
        assert_eq!(module.found[0].number, Some(1), "write");
        assert_eq!(module.found[1].number, Some(0), "xor rax, rax vaut zéro");
        assert_eq!(syscall_name(1), Some("write"));
        assert!(is_simulated(12) && !is_simulated(59));
    }

    /// Tous les blocs d'une vue, à plat.
    fn à_plat(blocs: &[Bloc]) -> Vec<&Bloc> {
        let mut vus = Vec::new();
        for bloc in blocs {
            vus.push(bloc);
            match bloc {
                Bloc::Section { blocs, .. }
                | Bloc::Repliable { blocs, .. }
                | Bloc::Retrait(blocs) => vus.extend(à_plat(blocs)),
                _ => {}
            }
        }
        vus
    }

    /// Tous les fragments d'une vue, dans l'ordre.
    fn fragments(vue: &Vue) -> Vec<&Fragment> {
        let mut vus = Vec::new();
        for bloc in à_plat(&vue.blocs) {
            match bloc {
                Bloc::Texte(ligne) | Bloc::Attente(ligne) => vus.extend(&ligne.fragments),
                Bloc::Tableau { rangées, .. } => {
                    vus.extend(rangées.iter().flatten().flat_map(|l| &l.fragments))
                }
                _ => {}
            }
        }
        vus
    }

    /// Le programme du banc, et deux appels qu'il faut pour tout voir.
    ///
    /// `mov eax, 41 ; syscall` est un `socket` — du réseau, donc une
    /// famille, donc un portrait — et namsdec ne le simule pas, donc
    /// l'avertissement. `mov eax, 1 ; syscall` est un `write`, qu'il
    /// simule. Sept octets suffisent, et ils tournent partout : voir la
    /// règle du contrôle d'`xrefs`, bâti de la même façon.
    fn deux_appels() -> Vec<namsdec_disasm::DecodedInstruction> {
        context_of(&[
            0xB8, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x05, // mov eax, 1 ; syscall
            0xB8, 0x29, 0x00, 0x00, 0x00, 0x0F, 0x05, // mov eax, 41 ; syscall
        ])
    }

    /// Le panneau plein se lit sans être peint.
    ///
    /// **LA SCÈNE DU BANC NE MONTRE QU'UNE TRANCHE DE CE PANNEAU**, et
    /// la capture le dit : le programme d'épreuve passe `write` et
    /// `exit`, qui n'appartiennent à aucune famille de comportement et
    /// que namsdec simule tous les deux. Ni le portrait, ni
    /// l'avertissement « non simulé », ni la fiche n'y sont jamais
    /// peints — exactement le trou trouvé en portant `patterns`. On
    /// bâtit donc les octets qui les font paraître, et l'on LIT.
    #[test]
    fn le_panneau_plein_se_lit_sans_etre_peint() {
        let disasm = deux_appels();
        let cpu = Cpu::new(Memory::new(0x400000, 0x10));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = SyscallsModule::default();
        module.on_binary_loaded(&ctx);
        let vue = module.vue(&ctx).expect("le panneau dit quelque chose");
        let vus = fragments(&vue);

        // LE PORTRAIT, et sa famille rangée par CLÉ — c'est le dernier
        // endroit du crate qui choisissait une couleur.
        assert!(
            vus.iter().any(|f| f.texte == "Ce qu'il sait faire"),
            "le portrait est en tête"
        );
        let réseau = vus
            .iter()
            .find(|f| f.étiquette.as_deref() == Some("network"))
            .expect("le socket range le binaire dans le réseau");
        assert!(
            réseau.texte.contains("(1)"),
            "une famille tenue par un seul appel le dit : {}",
            réseau.texte
        );

        // L'AVERTISSEMENT, qui ne paraît que sur ce que namsdec ne sait
        // pas exécuter.
        assert!(
            vus.iter().any(|f| f.texte.contains("non simulé")),
            "`socket` n'est pas simulé, et le panneau le dit"
        );
        // UNE FOIS, ET NON DEUX : `write` est simulé, lui, et ne doit
        // pas porter l'avertissement. Une première version comparait
        // `f.texte == "write" && f.texte.contains("non simulé")` — faux
        // par construction, donc un contrôle qui ne regarde rien.
        assert_eq!(
            vus.iter()
                .filter(|f| f.texte.contains("non simulé"))
                .count(),
            1,
            "deux appels, un seul non simulé : {:?}",
            vus.iter().map(|f| &f.texte).collect::<Vec<_>>()
        );
    }

    /// La fiche d'un appel se déplie par son action, et se referme de même.
    ///
    /// Le chemin entier, SANS PEINDRE : la vue propose une action, la
    /// bascule l'applique, la vue suivante porte la fiche avec sa
    /// signature, le registre de chaque argument et le renvoi au
    /// manuel — et la même action referme.
    #[test]
    fn une_fiche_se_deplie_par_son_action_et_se_referme_de_meme() {
        let disasm = deux_appels();
        let cpu = Cpu::new(Memory::new(0x400000, 0x10));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = SyscallsModule::default();
        module.on_binary_loaded(&ctx);

        // Rien n'est déplié : aucun tableau dans la vue.
        let tableaux = |module: &SyscallsModule| {
            let vue = module.vue(&ctx).expect("le panneau dit quelque chose");
            à_plat(&vue.blocs)
                .into_iter()
                .filter(|bloc| matches!(bloc, Bloc::Tableau { .. }))
                .count()
        };
        assert_eq!(tableaux(&module), 0, "aucune fiche au départ");

        // L'action est PROPOSÉE par la vue, et non inventée par le
        // contrôle : c'est ce qui le fait tomber si le nom change.
        let vue = module.vue(&ctx).expect("le panneau dit quelque chose");
        let action = fragments(&vue)
            .into_iter()
            .find(|f| f.texte == "write")
            .and_then(|f| f.action.clone())
            .expect("le nom de l'appel propose une action");
        module.bascule_fiche(&action);
        assert_eq!(module.ouvert, Some(1), "la fiche de `write` est ouverte");

        let vue = module.vue(&ctx).expect("le panneau dit quelque chose");
        let vus = fragments(&vue);
        assert_eq!(tableaux(&module), 1, "une fiche, une seule");
        assert!(
            // « RAX=1 write(fd: RDI, buf: RSI, count: RDX) » : le
            // numéro d'abord, parce que c'est par lui qu'on reconnaît
            // l'appel en lisant un désassemblage.
            vus.iter()
                .any(|f| f.texte.contains("write(") && f.texte.contains("RAX=1")),
            "la signature est dite : {:?}",
            vus.iter().map(|f| &f.texte).collect::<Vec<_>>()
        );
        // LE REGISTRE DE CHAQUE ARGUMENT : c'est ce qui manque en
        // lisant un désassemblage, et c'est la raison d'être de la
        // fiche.
        for registre in ["RDI", "RSI", "RDX", "RAX"] {
            assert!(
                vus.iter().any(|f| f.texte == registre),
                "{registre} est nommé : {:?}",
                vus.iter().map(|f| &f.texte).collect::<Vec<_>>()
            );
        }
        // LE RENVOI, qui mène hors de namsdec. C'est une donnée, et
        // c'est pour cela qu'elle est dans la vue : un rapport la
        // garde, et le lecteur n'a pas à croire namsdec sur parole.
        let renvoi = vus
            .iter()
            .find_map(|f| f.renvoi.as_deref())
            .expect("la fiche renvoie au manuel");
        assert!(
            renvoi.starts_with("https://") && renvoi.contains("write"),
            "le renvoi mène à la page de `write` : {renvoi}"
        );

        // Et la même action referme.
        module.bascule_fiche(&action);
        assert_eq!(module.ouvert, None, "un second clic referme");
        assert_eq!(tableaux(&module), 0);
    }

    /// Un numéro qui vient d'un calcul ne se devine pas : le module le dit
    /// plutôt que de remonter jusqu'à une valeur qui n'a plus cours.
    #[test]
    fn a_computed_number_is_reported_as_unreadable() {
        // mov eax, 1 / add eax, ebx / syscall
        let bytes: &[u8] = &[0xB8, 0x01, 0x00, 0x00, 0x00, 0x01, 0xD8, 0x0F, 0x05];
        let disasm = context_of(bytes);
        let cpu = Cpu::new(Memory::new(0x400000, 0x10));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = SyscallsModule::default();
        module.on_binary_loaded(&ctx);
        assert_eq!(module.found[0].number, None);
    }
}
