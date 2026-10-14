//! Le panneau des fonctions empruntées : ce que le binaire appelle
//! ailleurs que chez lui.
//!
//! Le premier réflexe devant un binaire inconnu est de regarder ce qu'il
//! demande au système et à la bibliothèque C : qui ouvre des fichiers,
//! qui alloue, qui écrit. Les appels sont classés par nombre
//! d'appelants, comme les références croisées — ce qui sert partout se
//! lit en premier.

use crate::behaviour::{self, Family};
use crate::imports::{self, Import};
use crate::libc;
use crate::{ReContext, ReModule, ReRequest, ReSection, clickable_address};

#[derive(Default)]
pub struct ImportsModule {
    /// Ce que le panneau demande à l'application : aller voir une
    /// adresse. Relevé après le dessin, jamais pendant.
    demande: Option<ReRequest>,
    imports: Vec<Import>,
    /// Combien de `call` du code visent chaque entrée.
    appels: Vec<usize>,
    /// La fonction dont la fiche est dépliée, s'il y en a une.
    ///
    /// Une seule à la fois : une liste de cent entrées toutes dépliées
    /// n'est plus une liste, et ce qu'on cherche — la fonction qu'on ne
    /// connaît pas — se perd dedans.
    ouverte: Option<String>,
}

impl ImportsModule {
    /// Les fonctions importées, pour qui veut les lire sans l'interface.
    pub fn imports(&self) -> &[Import] {
        &self.imports
    }

    /// Combien d'appels mènent à chacune, dans le même ordre.
    pub fn calls(&self) -> &[usize] {
        &self.appels
    }

    /// Les familles de comportement présentes, dans l'ordre où on les
    /// regarde — le portrait du binaire en six cases.
    pub fn families(&self) -> Vec<Family> {
        behaviour::families_of(self.imports.iter().map(|i| i.name.as_str()))
    }
}

/// La ligne de familles : ce que le binaire sait faire, d'un coup d'œil.
///
/// Dessinée avant la liste, parce que c'est la question qu'on pose
/// d'abord. Chaque case porte le nombre de fonctions qui l'ont nommée —
/// une famille tenue par un seul appel ne dit pas la même chose qu'une
/// famille tenue par quinze.
pub(crate) fn portrait(ui: &mut egui::Ui, ctx: &ReContext, comptes: &[(Family, usize)]) {
    if comptes.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(ctx.t("Ce qu'il sait faire", "What it can do"));
        for (famille, combien) in comptes {
            let texte = format!("{} ({combien})", famille.label(ctx.lang));
            ui.label(egui::RichText::new(texte).color(famille.color()).strong())
                .on_hover_text(famille.why(ctx.lang));
        }
    });
}

impl ReModule for ImportsModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::Borrows
    }

    fn id(&self) -> &'static str {
        "imports"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Fonctions importées"
        } else {
            "Imported functions"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.imports = imports::scan(ctx.binary);
        // Un autre binaire : la fiche ouverte parlait d'une fonction
        // qui n'est peut-être plus là.
        self.ouverte = None;
        // Un appel par entrée : c'est ce qui distingue la fonction dont
        // tout dépend de celle qui est liée sans jamais servir.
        //
        // Les adresses ne se comparent pas toujours telles quelles, et
        // c'est là qu'était le défaut. Le nom d'un import vient des
        // tables du fichier : c'est une **adresse virtuelle**. Le
        // listing, lui, se place de deux façons selon ce qu'on lui
        // donne :
        //
        // * un ELF **ouvert** : l'application ne désassemble que les
        //   parties exécutables, chacune à son adresse virtuelle. Les
        //   deux adresses sont alors les mêmes, et il n'y a rien à
        //   convertir ;
        // * un **tampon brut** désassemblé d'un bloc : il part de son
        //   premier octet, et une adresse virtuelle s'y retrouve à
        //   `base + sa position dans le fichier`.
        //
        // Seule la seconde était traitée. Sur un binaire ouvert —
        // c'est-à-dire dans l'usage même du panneau — aucun compte ne
        // tombait juste : mesuré sur `/bin/ls`, 105 imports et **zéro**
        // appel, sans que rien ne le signale. Un compte faux se lit
        // comme un compte vrai.
        //
        // Les deux façons sont donc essayées. Une entrée comptée deux
        // fois est impossible : les deux adresses ne coïncident que si
        // la conversion est l'identité.
        let base = ctx.disasm.first().map(|first| first.address);
        let cibles: Vec<u64> = ctx
            .disasm
            .iter()
            .filter_map(|instruction| {
                namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
            })
            .filter(|decoded| decoded.flow_control() == iced_x86::FlowControl::Call)
            .map(|decoded| decoded.near_branch_target())
            .collect();
        self.appels = self
            .imports
            .iter()
            .map(|import| {
                // L'adresse de l'entrée dans un listing qui part du
                // premier octet du fichier, quand on sait la calculer.
                let posée = base.and_then(|base| {
                    imports::file_offset(ctx.binary, import.plt)
                        .and_then(|offset| base.checked_add(offset))
                });
                cibles
                    .iter()
                    .filter(|cible| **cible == import.plt || Some(**cible) == posée)
                    .count()
            })
            .collect();
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.imports.is_empty() {
            ui.weak(ctx.t(
                "Aucune : ce programme n'emprunte rien: il est statique, ou il ne parle \
                 qu'au noyau.",
                "None: this program borrows nothing — it is static, or it only talks to \
                 the kernel.",
            ));
            return;
        }
        ui.strong(format!(
            "{} {}",
            self.imports.len(),
            ctx.t("fonctions importées", "imported functions")
        ));
        ui.weak(ctx.t(
            "Chaque entrée est une petite fonction qui saute là où l'éditeur de liens \
             aura écrit la vraie adresse. C'est pour cela qu'un `call` vers la \
             bibliothèque C passe par le programme lui-même.",
            "Each entry is a small function jumping to wherever the linker will have \
             written the real address. That is why a call into the C library goes \
             through the program itself.",
        ));
        // Le portrait avant la liste : « qu'est-ce que ce binaire fait »
        // se répond en six cases, là où cinquante noms demandent de les
        // connaître un par un.
        let comptes: Vec<(Family, usize)> = behaviour::ALL
            .into_iter()
            .filter_map(|famille| {
                let combien = self
                    .imports
                    .iter()
                    .filter(|i| behaviour::family_of(&i.name) == Some(famille))
                    .count();
                (combien > 0).then_some((famille, combien))
            })
            .collect();
        if !comptes.is_empty() {
            ui.add_space(4.0);
            portrait(ui, ctx, &comptes);
        }
        ui.add_space(4.0);

        // Par nombre d'appels : ce qui sert partout d'abord, le reste
        // par ordre alphabétique pour s'y retrouver.
        let mut rangs: Vec<usize> = (0..self.imports.len()).collect();
        rangs.sort_by(|&a, &b| {
            self.appels[b]
                .cmp(&self.appels[a])
                .then_with(|| self.imports[a].name.cmp(&self.imports[b].name))
        });
        // Relevé pendant le dessin, posé après : la liste emprunte
        // `self`, et le module ne peut donc pas s'écrire dedans.
        let mut demande: Option<ReRequest> = None;
        let mut bascule: Option<String> = None;
        for rang in rangs {
            let import = &self.imports[rang];
            let appels = self.appels[rang];
            let note = libc::explain(&import.name);
            let ouverte = self.ouverte.as_deref() == Some(import.name.as_str());
            ui.horizontal_wrapped(|ui| {
                if let Some(cible) = clickable_address(ui, ctx, import.plt) {
                    demande = Some(ReRequest::GoTo {
                        addr: import.plt,
                        target: cible,
                    });
                }
                // LE NOM EST UN LIEN : on le clique, sa fiche se
                // déplie.
                //
                // Une liste de noms suppose qu'on les connaisse — c'est
                // vrai de `malloc`, beaucoup moins de `__cxa_finalize`
                // ou de `_Unwind_Resume`, qui sont pourtant les deux
                // seules entrées d'un binaire Rust ordinaire. La phrase
                // existait déjà pour les plus courantes, posée sous la
                // ligne ; elle manquait partout ailleurs, et rien ne
                // disait qu'il y avait quelque chose à lire.
                //
                // Le survol donne la phrase, le clic donne la fiche :
                // la signature, le registre qui porte chaque argument,
                // et ce qu'il faut surveiller.
                let couleur = ui.visuals().hyperlink_color;
                let réponse = ui
                    .add(
                        egui::Label::new(egui::RichText::new(&import.name).strong().color(couleur))
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(match note {
                        Some(note) => note.text(ctx.lang).to_owned(),
                        None => match libc::family_hint(&import.name) {
                            Some((fr, en)) => {
                                if ctx.lang.starts_with("fr") {
                                    fr.to_owned()
                                } else {
                                    en.to_owned()
                                }
                            }
                            None => ctx
                                .t(
                                    "Cliquer pour ce que namsdec en sait",
                                    "Click for what namsdec knows about it",
                                )
                                .to_owned(),
                        },
                    });
                if réponse.clicked() {
                    bascule = Some(import.name.clone());
                }
                if appels > 0 {
                    ui.weak(format!(
                        "{appels} {}",
                        if appels > 1 {
                            ctx.t("appels", "calls")
                        } else {
                            ctx.t("appel", "call")
                        }
                    ));
                }
                // Ce qu'on cherche vraiment en lisant la liste : par où
                // une entrée peut-elle déborder ? Ce n'est pas un
                // verdict sur le programme, c'est l'endroit qu'on
                // regarde en premier.
                if note.is_some_and(|note| note.watch) {
                    ui.label(
                        egui::RichText::new(ctx.t("à regarder", "worth a look"))
                            .color(egui::Color32::from_rgb(0xd7, 0xba, 0x7d)),
                    );
                }
                // Et sa famille de comportement, quand elle en a une :
                // c'est ce qui relie une ligne au portrait du dessus.
                if let Some(famille) = behaviour::family_of(&import.name) {
                    ui.label(egui::RichText::new(famille.label(ctx.lang)).color(famille.color()))
                        .on_hover_text(famille.why(ctx.lang));
                }
            });
            // La phrase reste sous la ligne, dépliée ou non : c'est
            // elle qu'on lit en parcourant, et la cacher derrière un
            // clic serait reprendre d'une main ce qu'on donne de
            // l'autre.
            if let Some(note) = note {
                ui.indent(&import.name, |ui| ui.weak(note.text(ctx.lang)));
            }
            if ouverte {
                ui.indent(("fiche", &import.name), |ui| {
                    fiche(ui, ctx, &import.name, note);
                });
            }
        }
        if demande.is_some() {
            self.demande = demande;
        }
        if let Some(nom) = bascule {
            // Un second clic referme : c'est la même touche qui ouvre
            // et qui ferme, comme partout ailleurs.
            self.ouverte = (self.ouverte.as_deref() != Some(nom.as_str())).then_some(nom);
        }
    }
}

/// La fiche d'une fonction empruntée : ce que namsdec en sait, et ce
/// qu'il ne sait pas.
///
/// Trois choses, et la troisième compte autant que les deux autres :
///
/// * **la signature**, avec le registre qui porte chaque argument —
///   c'est ce qui transforme « RDI vaut 0x402000 » en « dest vaut
///   0x402000 », et c'est la moitié du travail quand on lit un appel ;
/// * **ce qu'il faut surveiller**, quand la fonction écrit sans
///   connaître la taille de sa destination ;
/// * **l'aveu**, quand la fonction n'est pas dans la table. Un panneau
///   qui se tait laisse croire qu'il n'y a rien à dire ; ici il dit ce
///   que le NOM apprend — un préfixe appartient à une convention — et
///   s'arrête là plutôt que de deviner.
fn fiche(ui: &mut egui::Ui, ctx: &ReContext, nom: &str, note: Option<libc::Note>) {
    let fr = ctx.lang.starts_with("fr");
    if let Some(note) = note {
        if !note.params.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.monospace(format!("{nom}({})", note.params.join(", ")));
            });
            ui.add_space(2.0);
            egui::Grid::new(("fiche_params", nom))
                .num_columns(2)
                .min_col_width(0.0)
                .striped(true)
                .show(ui, |ui| {
                    for (rang, param) in note.params.iter().enumerate() {
                        // Au-delà du sixième, la convention passe par la
                        // pile : on ne nomme pas un registre qui ne
                        // porte rien.
                        let Some(registre) = libc::ARGUMENT_REGISTERS.get(rang) else {
                            break;
                        };
                        if *param == "..." {
                            ui.weak(ctx.t("… et la suite, variable", "… and the rest, variable"));
                            ui.weak(ctx.t(
                                "le nombre d'arguments dépend du format",
                                "how many arguments depends on the format",
                            ));
                            ui.end_row();
                            continue;
                        }
                        ui.monospace(*param);
                        ui.monospace(
                            egui::RichText::new(*registre).color(ui.visuals().hyperlink_color),
                        );
                        ui.end_row();
                    }
                });
            ui.add_space(2.0);
        }
        if note.watch {
            ui.colored_label(
                egui::Color32::from_rgb(0xd7, 0xba, 0x7d),
                ctx.t(
                    "À regarder : elle écrit sans connaître la taille de la destination, ou \
                     exécute ce qu'on lui donne. Ce n'est pas un verdict sur le programme — \
                     c'est l'endroit qu'on regarde en premier.",
                    "Worth a look: it writes without knowing the size of the destination, or \
                     runs what it is given. That is not a verdict on the program — it is \
                     where one looks first.",
                ),
            );
        }
        return;
    }
    // Pas de fiche : on dit ce que le nom apprend, et rien de plus.
    match libc::family_hint(nom) {
        Some((texte_fr, texte_en)) => {
            ui.weak(if fr { texte_fr } else { texte_en });
            ui.add_space(2.0);
            ui.weak(ctx.t(
                "namsdec n'a pas de fiche pour cette fonction-là : ce qui précède se lit \
                 dans son nom, pas dans son code.",
                "namsdec has no entry for this particular function: what precedes is read \
                 from its name, not from its code.",
            ));
        }
        None => {
            ui.weak(ctx.t(
                "namsdec n'a pas de fiche pour celle-ci. La table ne porte que les \
                 fonctions qu'on rencontre partout et celles dont l'usage demande de \
                 l'attention : ce qui n'y figure pas n'est pas deviné. `man` et la page \
                 de la bibliothèque le diront mieux.",
                "namsdec has no entry for this one. The table only holds the functions one \
                 meets everywhere and those whose use calls for care: what is not in it is \
                 not guessed. `man` and the library's own page will say it better.",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    /// Ce que le panneau peint, une galère par ligne.
    fn textes(module: &mut ImportsModule, ctx: &ReContext) -> Vec<String> {
        let egui_ctx = egui::Context::default();
        let mut vus = Vec::new();
        for _ in 0..2 {
            vus.clear();
            let mut sortie = egui_ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 1400.0),
                    )),
                    ..Default::default()
                },
                |ui| module.ui(ui, ctx),
            );
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    vus.push(texte.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        vus
    }

    /// Le nom d'une fonction empruntée se déplie sur sa fiche.
    ///
    /// Demandé par Fred : « les appels pourraient être des hyperliens
    /// montrant la description de chaque appel ». Une liste de noms
    /// suppose qu'on les connaisse — c'est vrai de `malloc`, beaucoup
    /// moins de `__cxa_finalize`, qui est pourtant l'une des deux
    /// seules entrées d'un binaire Rust ordinaire.
    ///
    /// La fiche porte ce qu'une page de manuel donne en premier : la
    /// signature, et le registre par lequel arrive chaque argument.
    #[test]
    fn le_nom_d_une_fonction_se_deplie_sur_sa_fiche() {
        let Ok(octets) = std::fs::read("/bin/ls") else {
            return;
        };
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &octets,
            lang: "fr",
            names: None,
        };
        let mut module = ImportsModule::default();
        module.on_binary_loaded(&ctx);
        // La PREMIÈRE dans l'ordre d'affichage, et c'est nécessaire :
        // egui ne peint pas ce qui tombe hors de l'écran, et /bin/ls
        // emprunte cent cinq fonctions. Une fiche dépliée au
        // soixantième rang ne serait pas peinte, et le contrôle
        // conclurait à tort qu'elle manque.
        //
        // Sans désassemblage, tous les comptes d'appels valent zéro :
        // l'ordre d'affichage est alors alphabétique.
        let mut noms: Vec<String> = module.imports.iter().map(|i| i.name.clone()).collect();
        noms.sort();
        let Some(connue) = noms
            .into_iter()
            .find(|nom| libc::explain(nom).is_some_and(|note| !note.params.is_empty()))
        else {
            return; // aucun import connu ici : rien à prouver
        };

        eprintln!("SONDE connue = {connue}");
        // Repliée, la fiche n'est pas là.
        let avant = textes(&mut module, &ctx).join(" | ");
        assert!(avant.contains(&connue), "le nom est peint : {avant}");
        assert!(
            !avant.contains("RDI"),
            "tant qu'on n'a rien ouvert, aucune fiche : {avant}"
        );

        // Dépliée, elle donne la signature et les registres.
        module.ouverte = Some(connue.clone());
        let après = textes(&mut module, &ctx).join(" | ");
        let note = libc::explain(&connue).expect("une fiche");
        assert!(
            après.contains(&format!("{connue}({})", note.params.join(", "))),
            "la signature est écrite : {après}"
        );
        assert!(
            après.contains("RDI"),
            "et le registre qui porte le premier argument : {après}"
        );
        for param in note.params.iter().filter(|p| **p != "...") {
            assert!(après.contains(*param), "le paramètre {param} est nommé");
        }
    }

    /// Une fonction que la table ne connaît pas dit ce que son NOM
    /// apprend — et dit aussi qu'elle ne la connaît pas.
    ///
    /// C'est le cas des deux seules entrées du crackme de Fred :
    /// `_Unwind_Resume` et `__cxa_finalize`. La première a maintenant
    /// sa fiche ; pour ce que la table ignore, le préfixe appartient à
    /// une convention et le dire n'est pas deviner.
    #[test]
    fn une_fonction_inconnue_dit_ce_que_son_nom_apprend() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };
        let egui_ctx = egui::Context::default();
        let mut vus: Vec<String> = Vec::new();
        for _ in 0..2 {
            vus.clear();
            let mut sortie = egui_ctx.run_ui(Default::default(), |ui| {
                fiche(ui, &ctx, "__cxa_guard_acquire", None)
            });
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    vus.push(texte.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        let tout = vus.join(" | ");
        assert!(
            tout.contains("C++"),
            "la famille se lit dans le nom : {tout}"
        );
        assert!(
            tout.contains("pas de fiche"),
            "et l'aveu est dit, pas tu : {tout}"
        );

        // Les deux du crackme : l'une a sa fiche, l'autre son préfixe.
        assert!(libc::explain("_Unwind_Resume").is_some());
        assert!(libc::explain("__cxa_finalize").is_some());
    }

    /// Sur un vrai binaire, le module nomme ce qui est emprunté *et*
    /// compte les appels qui y mènent.
    ///
    /// Le comptage est la partie fragile : les noms viennent des
    /// sections — des adresses virtuelles — et le désassemblage part du
    /// premier octet du fichier. Sans le rapprochement par position dans
    /// le fichier, tous les comptes resteraient à zéro sans que rien ne
    /// le signale.
    #[test]
    fn a_system_binary_shows_what_it_calls_and_how_often() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        // Comme l'interface : le fichier entier, désassemblé depuis son
        // premier octet, posé à l'adresse de travail de namsdec.
        let taille = bytes.len().min(0x10000);
        let disasm = namsdec_disasm::disassemble(&bytes[..taille], 0x400000);
        let mem = Memory::new(0x400000, 0x1000);
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &bytes,
            lang: "fr",
            names: None,
        };

        let mut module = ImportsModule::default();
        module.on_binary_loaded(&ctx);

        let noms: Vec<&str> = module
            .imports()
            .iter()
            .map(|import| import.name.as_str())
            .collect();
        assert!(noms.contains(&"malloc"), "{noms:?}");
        assert!(noms.contains(&"free"), "{noms:?}");
        assert_eq!(module.calls().len(), module.imports().len());
        assert!(
            module.calls().iter().sum::<usize>() > 50,
            "un binaire de cette taille appelle ses imports : {:?}",
            module.calls()
        );
    }

    /// Et le compte tombe juste quand le listing est posé aux adresses
    /// **virtuelles** — c'est-à-dire dans l'application.
    ///
    /// Le contrôle d'au-dessus désassemble le fichier d'un bloc depuis
    /// son premier octet ; `load_elf` ne désassemble que les parties
    /// exécutables, chacune à son adresse virtuelle. Seule la première
    /// façon était traitée, et sur un binaire ouvert — l'usage même du
    /// panneau — tous les comptes restaient à zéro sans que rien ne le
    /// signale.
    #[test]
    fn the_count_is_right_when_the_listing_sits_at_virtual_addresses() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        let Some(elf) = crate::elf_scan::scan(&bytes) else {
            return;
        };
        // Comme `load_elf` : chaque section exécutable à son adresse.
        let mut disasm = Vec::new();
        for section in &elf.sections {
            if !section.execute || section.nobits || section.size == 0 {
                continue;
            }
            let début = section.offset as usize;
            let Some(contenu) = bytes.get(début..début + section.size as usize) else {
                continue;
            };
            disasm.extend(namsdec_disasm::disassemble(contenu, section.addr));
        }
        disasm.sort_by_key(|instruction| instruction.address);
        assert!(!disasm.is_empty(), "il y a du code exécutable");

        let mem = Memory::new(0x400000, 0x1000);
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &bytes,
            lang: "fr",
            names: None,
        };
        let mut module = ImportsModule::default();
        module.on_binary_loaded(&ctx);

        assert!(!module.imports().is_empty(), "des imports sont nommés");
        assert!(
            module.calls().iter().sum::<usize>() > 50,
            "un binaire de cette taille appelle ses imports — zéro veut dire que la \
             conversion d'adresse est fausse : {:?}",
            module.calls()
        );
    }

    /// Un programme qui n'emprunte rien ne se voit pas reprocher de
    /// tables manquantes : la liste est vide, et c'est tout.
    #[test]
    fn a_program_that_borrows_nothing_shows_nothing() {
        let bytes: &[u8] = &[0x90, 0xc3];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: bytes,
            lang: "fr",
            names: None,
        };
        let mut module = ImportsModule::default();
        module.on_binary_loaded(&ctx);
        assert!(module.imports().is_empty());
    }
}
