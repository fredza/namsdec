use crate::behaviour::{self, Family};
use crate::{
    ReContext, ReModule, ReSection, elf_scan, imports, strings, syscalls, toolchain, xrefs,
};

/// Le résumé : de quoi on parle, avant d'ouvrir les autres panneaux.
///
/// Six modules détaillent chacun une chose ; celui-ci répond à la
/// question qu'on pose en premier — qu'est-ce que c'est, et qu'est-ce
/// qu'il y a dedans ? Tout ce qu'il montre est mesuré, pas déclaré :
/// les fonctions viennent des `call` relevés, les chaînes des octets
/// lisibles, la nature du fichier de ses en-têtes, et ce qu'il *sait
/// faire* des fonctions qu'il emprunte et des appels système qu'il passe.
#[derive(Default)]
pub struct BinaryInfoModule {
    size: usize,
    entry: u64,
    instructions: usize,
    functions: usize,
    strings: usize,
    imports: usize,
    syscalls: usize,
    /// Le comportement, par famille, dans l'ordre de lecture — ce que le
    /// binaire sait faire, avec le nombre d'appels qui le tiennent.
    comportement: Vec<(Family, usize)>,
    /// Ce que l'en-tête ELF dit du fichier, quand on l'a : nature,
    /// liaison, protections. Gardé entier pour en tirer plusieurs lignes
    /// sans rescanner à chaque image.
    scan: Option<elf_scan::Scan>,
    /// Qui a produit ce fichier, et dans quel langage — avec le degré
    /// de certitude et les indices qui le portent.
    toolchain: toolchain::Toolchain,
}

impl ReModule for BinaryInfoModule {
    fn section(&self) -> ReSection {
        ReSection::File
    }

    fn id(&self) -> &'static str {
        "binary_info"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "En bref"
        } else {
            "At a glance"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.size = ctx.disasm.iter().map(|i| i.bytes.len()).sum();
        self.entry = ctx.disasm.first().map(|i| i.address).unwrap_or(0);
        self.instructions = ctx.disasm.len();

        let range = ctx.cpu.mem.range();
        self.functions = xrefs::scan(ctx.disasm, range.clone()).functions().len();
        self.strings = (range.end - range.start)
            .try_into()
            .ok()
            .and_then(|len: usize| ctx.cpu.mem.peek_bytes(range.start, len).ok())
            .map(|bytes| strings::scan(bytes, range.start).len())
            .unwrap_or(0);

        // Ce que le binaire sait faire : relevé de ses fonctions
        // empruntées ET de ses appels système, rangé par famille. Les
        // deux, car un programme qui veut passer inaperçu appelle le
        // noyau par son numéro plutôt que par la table de liaison.
        let imports = imports::scan(ctx.binary);
        self.imports = imports.len();
        let appels = syscalls::scan(ctx.disasm);
        self.syscalls = appels.len();
        self.comportement = behaviour::ALL
            .iter()
            .map(|&famille| {
                let n = imports
                    .iter()
                    .filter(|i| behaviour::family_of(&i.name) == Some(famille))
                    .count()
                    + appels
                        .iter()
                        .filter(|c| c.family() == Some(famille))
                        .count();
                (famille, n)
            })
            .collect();

        self.scan = elf_scan::scan(ctx.binary);
        // La chaîne d'outils : relevée une fois au chargement, comme le
        // reste. Elle lit des sections entières et parcourt les chaînes
        // du fichier — ce n'est pas un calcul à refaire par image.
        let symboles: Vec<String> = self
            .scan
            .as_ref()
            .map(|scan| scan.functions.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        self.toolchain = toolchain::detect(ctx.binary, self.scan.as_ref(), &symboles);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        egui::Grid::new("binary_info_grid")
            .num_columns(2)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                let mut ligne = |label: &str, valeur: String| {
                    ui.label(label);
                    ui.monospace(valeur);
                    ui.end_row();
                };
                ligne(
                    ctx.t("Code", "Code"),
                    format!(
                        "{} {} · {} {}",
                        self.size,
                        ctx.t("octets", "bytes"),
                        self.instructions,
                        ctx.t("instructions", "instructions")
                    ),
                );
                ligne(
                    ctx.t("Point d'entrée", "Entry point"),
                    format!("{:#x}", self.entry),
                );
                ligne(
                    ctx.t("Fonctions", "Functions"),
                    format!(
                        "{} ({})",
                        self.functions,
                        ctx.t("cibles de `call`", "`call` targets")
                    ),
                );
                ligne(ctx.t("Chaînes", "Strings"), format!("{}", self.strings));
                ligne(
                    ctx.t("Emprunte", "Borrows"),
                    format!(
                        "{} {} · {} {}",
                        self.imports,
                        ctx.t("fonctions", "functions"),
                        self.syscalls,
                        ctx.t("appels système", "system calls")
                    ),
                );
                if let Some(scan) = &self.scan {
                    ligne(
                        ctx.t("Fichier", "File"),
                        format!(
                            "{} · {}",
                            if scan.pie() { "PIE" } else { "EXEC" },
                            if scan.dynamic() { "dyn" } else { "static" }
                        ),
                    );
                }
            });

        // CE QUI L'A PRODUIT.
        //
        // La première question qu'on pose devant un fichier inconnu, et
        // la plus rentable : un binaire Go ne se lit pas comme un
        // binaire C, et savoir qu'on a du C++ devant soi change la
        // façon dont on lit un `call` indirect — c'est peut-être une
        // table virtuelle.
        //
        // Chaque réponse porte SON DEGRÉ DE CERTITUDE, et le détail des
        // indices se déplie. Un outil qui affiche « compilateur : GCC »
        // sans dire d'où il le tient apprend à croire plutôt qu'à lire.
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.strong(ctx.t("Ce qui l'a produit", "What produced it"));
        let certitude = |ui: &mut egui::Ui, trouvé: &toolchain::Finding| {
            ui.monospace(&trouvé.name);
            ui.weak(format!("({})", trouvé.confidence.label(ctx.lang)));
        };
        if self.toolchain.language.is_none() && self.toolchain.compiler.is_none() {
            ui.weak(ctx.t(
                "Indéterminé : ni section .comment, ni runtime nommé, ni décor de \
                 symboles reconnu. Un binaire dépouillé peut n'en porter aucun.",
                "Undetermined: no .comment section, no named runtime, no recognised \
                 symbol mangling. A stripped binary may carry none of these.",
            ));
        } else {
            egui::Grid::new("binary_info_toolchain")
                .num_columns(3)
                .min_col_width(0.0)
                .striped(true)
                .show(ui, |ui| {
                    if let Some(langage) = &self.toolchain.language {
                        ui.label(ctx.t("Langage", "Language"));
                        certitude(ui, langage);
                        ui.end_row();
                    }
                    if let Some(compilateur) = &self.toolchain.compiler {
                        ui.label(ctx.t("Compilateur", "Compiler"));
                        certitude(ui, compilateur);
                        ui.end_row();
                    }
                });
        }
        if !self.toolchain.evidence.is_empty() {
            egui::CollapsingHeader::new(ctx.t("Ce qui le dit", "What says so"))
                .id_salt("binary_info_evidence")
                .show(ui, |ui| {
                    for indice in &self.toolchain.evidence {
                        ui.horizontal_wrapped(|ui| {
                            ui.weak(format!("[{}]", indice.where_));
                            ui.monospace(&indice.what);
                            ui.weak(format!("— {}", indice.means));
                        });
                    }
                    ui.add_space(4.0);
                    // CE QUE CELA NE PROUVE PAS, et ce n'est pas une
                    // formule de prudence : `.comment` s'écrit, les
                    // symboles s'effacent, les chaînes se remplacent. Un
                    // lecteur qui l'ignore tirerait de « Go 1.22 » une
                    // confiance qu'aucun octet ne justifie.
                    ui.weak(ctx.t(
                        "Tout cela se falsifie : la section .comment s'écrit, les symboles \
                     s'effacent, les chaînes se remplacent. Ces indices disent ce que le \
                     fichier MONTRE, non ce qu'il est.",
                        "All of this can be forged: the .comment section can be written, symbols \
                     stripped, strings replaced. These clues say what the file SHOWS, not \
                     what it is.",
                    ));
                });
        }

        // Ce qu'il sait faire : le portrait, puis une ligne par famille
        // présente. C'est la réponse à « a-t-il accès au réseau ? » et aux
        // cinq autres questions du même ordre — lue d'un coup d'œil, et
        // expliquée juste en dessous.
        let présentes: Vec<(Family, usize)> = self
            .comportement
            .iter()
            .copied()
            .filter(|(_, n)| *n > 0)
            .collect();
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.strong(ctx.t("Ce qu'il sait faire", "What it can do"));
        if présentes.is_empty() {
            ui.weak(ctx.t(
                "Rien de remarquable : ni réseau, ni exécution, ni chiffrement, ni accès \
                 fichier repéré. Un programme peut n'avoir besoin de rien de tout cela.",
                "Nothing notable: no network, execution, cryptography or file access spotted. \
                 A program may need none of these.",
            ));
        } else {
            // SANS LE PORTRAIT. `portrait` est la ligne de pastilles du
            // panneau « Fonctions importées », et il porte son propre
            // libellé — « Ce qu'il sait faire » —, écrit pour vivre
            // seul. Appelé ici, sous un titre qui dit déjà la même
            // phrase, il la redisait à la ligne suivante, puis
            // réénumérait les familles que les lignes ci-dessous
            // détaillent. Trois lignes, deux répétitions :
            //
            //     Ce qu'il sait faire
            //     Ce qu'il sait faire   Fichiers (4)
            //     Fichiers (4)   Le programme touche au système de…
            //
            // Vu sur une capture. On garde les lignes expliquées, qui
            // disent la même chose et de plus disent pourquoi.
            for (famille, combien) in &présentes {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        famille.color(),
                        format!("{} ({combien})", famille.label(ctx.lang)),
                    );
                    ui.weak(famille.why(ctx.lang));
                });
            }
        }

        // Les protections, en un coup d'œil : le détail complet reste au
        // panneau « Protections », mais les quatre qui décident de tout se
        // lisent ici.
        if let Some(scan) = &self.scan {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.strong(ctx.t("Protections", "Protections"));
            // « √ » et « × », et non « ✓ » et « ✗ » : ces deux-là
            // n'existent pas dans les polices qu'egui embarque, et se
            // peignaient donc en carré vide — mesuré, voir
            // `tout_caractere_ecrit_existe_dans_la_police`. Une coche
            // absente ne dit pas « protection absente » : elle ne dit
            // rien du tout.
            let oui_non = |bon: bool| {
                if bon {
                    ("\u{221a}", egui::Color32::from_rgb(0x4e, 0xc9, 0xb0))
                } else {
                    ("\u{00d7}", egui::Color32::from_rgb(0xf4, 0x87, 0x71))
                }
            };
            egui::Grid::new("binary_info_prot")
                .num_columns(2)
                .min_col_width(0.0)
                .striped(true)
                .show(ui, |ui| {
                    let prot = |ui: &mut egui::Ui, label: &str, bon: bool, note: String| {
                        let (marque, couleur) = oui_non(bon);
                        ui.colored_label(couleur, format!("{marque} {label}"));
                        ui.weak(note);
                        ui.end_row();
                    };
                    prot(
                        ui,
                        ctx.t("PIE", "PIE"),
                        scan.pie(),
                        ctx.t(
                            "chargé à une adresse au hasard",
                            "loaded at a random address",
                        )
                        .to_owned(),
                    );
                    prot(
                        ui,
                        ctx.t("NX", "NX"),
                        scan.nx.unwrap_or(false),
                        ctx.t("la pile ne s'exécute pas", "the stack does not execute")
                            .to_owned(),
                    );
                    prot(
                        ui,
                        ctx.t("RELRO", "RELRO"),
                        scan.relro == elf_scan::Relro::Full,
                        match scan.relro {
                            elf_scan::Relro::Full => ctx.t("complet", "full"),
                            elf_scan::Relro::Partial => ctx.t("partiel", "partial"),
                            elf_scan::Relro::None => ctx.t("absent", "none"),
                        }
                        .to_owned(),
                    );
                    prot(
                        ui,
                        ctx.t("Canari", "Canary"),
                        scan.canary,
                        ctx.t(
                            "sentinelle contre l'écrasement de pile",
                            "sentinel against stack smashing",
                        )
                        .to_owned(),
                    );
                });
        } else {
            ui.add_space(6.0);
            ui.weak(ctx.t(
                "Pas d'en-tête ELF : ce sont des octets bruts, désassemblés depuis le premier.",
                "No ELF header: these are raw bytes, disassembled from the first one.",
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    #[test]
    fn the_summary_counts_what_it_sees() {
        let bytes: &[u8] = &[
            0xe8, 0x01, 0x00, 0x00, 0x00, // call +1
            0x90, // nop  (la cible)
            0xf4, // hlt
        ];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };

        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.size, 7);
        assert_eq!(module.entry, 0x400000);
        assert_eq!(module.instructions, 3);
        // Le `call` désigne une adresse : c'est une fonction.
        assert_eq!(module.functions, 1);
        // Pas d'en-tête : on ne prétend rien sur la nature du fichier.
        assert!(module.scan.is_none());
    }

    /// Le portrait compte ce que le code DEMANDE au système, même par
    /// numéro : `mov eax, 41 ; syscall` est un `socket`, donc du réseau.
    /// « Ce qu'il sait faire » ne s'écrit qu'une fois.
    ///
    /// Cette phrase existe à DEUX endroits par construction : c'est le
    /// titre de la section ici, et c'est aussi le libellé que
    /// `imports_module::portrait` porte en tête de sa ligne de
    /// pastilles — écrit pour vivre seul dans le panneau « Fonctions
    /// importées ». L'un appelé sous l'autre donnait trois lignes dont
    /// deux redites :
    ///
    /// ```text
    /// Ce qu'il sait faire
    /// Ce qu'il sait faire   Réseau (1)
    /// Réseau (1)   Le programme ouvre ou écoute une connexion…
    /// ```
    ///
    /// Vu sur une capture du 23 octobre, et nulle part ailleurs :
    /// aucune scène du banc de l'interface n'a de binaire dont le
    /// comportement soit non vide, de sorte que ce chemin n'y est
    /// jamais peint. Le contrôle vit donc ici, où l'état se construit
    /// en sept octets.
    #[test]
    fn what_it_can_do_is_written_only_once() {
        // Le même appel système que ci-dessous : il suffit à remplir le
        // comportement, qui est la condition pour que la section
        // s'affiche.
        let bytes: &[u8] = &[0xb8, 0x29, 0x00, 0x00, 0x00, 0x0f, 0x05];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);
        assert!(
            !module.comportement.is_empty(),
            "sans comportement, la section ne se peint pas et le contrôle \
             ne prouverait rien"
        );

        let egui_ctx = egui::Context::default();
        let mut combien = 0;
        let mut sortie = egui_ctx.run_ui(Default::default(), |ui| module.ui(ui, &ctx));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape
                && texte.galley.text().trim() == "Ce qu'il sait faire"
            {
                combien += 1;
            }
        }
        sortie.textures_delta.clear();
        assert_eq!(
            combien, 1,
            "« Ce qu'il sait faire » est peint {combien} fois : le titre de la \
             section et le libellé de la ligne de pastilles disent la même chose"
        );
    }

    #[test]
    fn the_portrait_counts_a_network_syscall() {
        // b8 29 00 00 00 = mov eax, 41 (socket) ; 0f 05 = syscall.
        let bytes: &[u8] = &[0xb8, 0x29, 0x00, 0x00, 0x00, 0x0f, 0x05];
        let disasm = namsdec_disasm::disassemble(bytes, 0x400000);
        let mut mem = Memory::new(0x400000, 0x100);
        mem.load(0x400000, bytes).unwrap();
        let cpu = Cpu::new(mem);
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = BinaryInfoModule::default();
        module.on_binary_loaded(&ctx);

        assert_eq!(module.syscalls, 1, "un appel système relevé");
        let réseau = module
            .comportement
            .iter()
            .find(|(f, _)| *f == Family::Network)
            .map(|(_, n)| *n)
            .unwrap_or(0);
        assert_eq!(
            réseau, 1,
            "le socket compte comme du réseau : {:?}",
            module.comportement
        );
    }
}
