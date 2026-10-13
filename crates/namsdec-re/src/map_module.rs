use crate::{ReContext, ReModule, ReSection, elf_scan, entropy};

/// La carte du fichier : ses segments et ses sections.
///
/// C'est la question qu'on pose juste après « qu'est-ce que c'est ? » :
/// *où est quoi ?* Un binaire n'est pas un bloc — il a du code, des
/// données en lecture seule, des données modifiables, des tables que le
/// chargeur lit et jette. Savoir à quel endroit on regarde change ce
/// qu'on y cherche : une entropie haute dans `.text` est normale, la
/// même dans `.rodata` annonce des données chiffrées.
///
/// **Deux tables et non une**, parce que ce sont deux découpages du même
/// fichier, faits par deux personnes pour deux usages :
///
/// - les **segments** sont ce que le noyau lit pour charger le
///   programme. Ils portent les droits — c'est là qu'on voit un `rwx`,
///   et c'est le seul découpage qui existe encore à l'exécution ;
/// - les **sections** sont ce que l'éditeur de liens a rangé. Elles
///   portent les noms, elles sont plus fines, et elles peuvent avoir
///   été retirées d'un binaire dépouillé sans que celui-ci cesse de
///   tourner.
#[derive(Default)]
pub struct MapModule {
    segments: Vec<Segment>,
    sections: Vec<Section>,
}

/// Un segment, prêt à être écrit.
struct Segment {
    kind: &'static str,
    droits: String,
    offset: u64,
    filesz: u64,
    vaddr: u64,
    memsz: u64,
}

/// Une section, avec ce qu'on a mesuré dedans.
struct Section {
    nom: String,
    addr: u64,
    offset: u64,
    taille: u64,
    droits: String,
    /// `None` pour une section qui n'occupe rien dans le fichier :
    /// mesurer l'entropie de `.bss` reviendrait à mesurer celle de son
    /// voisin, ce qui est pire qu'un blanc.
    entropie: Option<f64>,
}

impl ReModule for MapModule {
    fn section(&self) -> ReSection {
        ReSection::File
    }

    fn id(&self) -> &'static str {
        "map"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Carte du fichier"
        } else {
            "File map"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.segments.clear();
        self.sections.clear();
        let Some(scan) = elf_scan::scan(ctx.binary) else {
            return;
        };

        for segment in &scan.segments {
            self.segments.push(Segment {
                kind: segment.kind,
                droits: segment.rights(),
                offset: segment.offset,
                filesz: segment.filesz,
                vaddr: segment.vaddr,
                memsz: segment.memsz,
            });
        }

        for section in &scan.sections {
            // Les droits d'une section tels que l'en-tête les donne. Le
            // « r » est implicite : une section qu'on ne peut pas lire
            // n'aurait aucun sens dans un fichier.
            let droits = format!(
                "r{}{}",
                if section.write { 'w' } else { '-' },
                if section.execute { 'x' } else { '-' }
            );
            let entropie = (!section.nobits && section.size > 0)
                .then(|| {
                    let début = usize::try_from(section.offset).ok()?;
                    let fin = début.checked_add(usize::try_from(section.size).ok()?)?;
                    ctx.binary.get(début..fin).map(entropy::shannon)
                })
                .flatten();
            self.sections.push(Section {
                nom: section.name.clone(),
                addr: section.addr,
                offset: section.offset,
                taille: section.size,
                droits,
                entropie,
            });
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.segments.is_empty() && self.sections.is_empty() {
            ui.weak(ctx.t(
                "Pas d'en-tête ELF : il n'y a ni segment ni section à cartographier.",
                "No ELF header: there is neither segment nor section to map.",
            ));
            return;
        }

        ui.label(
            egui::RichText::new(ctx.t(
                "Segments — ce que le noyau charge",
                "Segments — what the kernel loads",
            ))
            .strong(),
        );
        ui.add_space(2.0);
        egui::Grid::new("map_segments")
            .num_columns(5)
            .striped(true)
            .min_col_width(0.0)
            .spacing([14.0, 3.0])
            .show(ui, |ui| {
                for entête in [
                    ctx.t("Type", "Type"),
                    ctx.t("Droits", "Rights"),
                    ctx.t("Dans le fichier", "In the file"),
                    ctx.t("En mémoire", "In memory"),
                    ctx.t("Taille", "Size"),
                ] {
                    ui.weak(entête);
                }
                ui.end_row();
                for segment in &self.segments {
                    ui.monospace(segment.kind);
                    // Les droits en couleur : `wx` sur le même segment
                    // est ce qu'on cherche du regard, et le lire dans
                    // une colonne grise ne le donne pas.
                    let inquiétant = segment.droits.contains('w') && segment.droits.contains('x');
                    ui.monospace(egui::RichText::new(&segment.droits).color(if inquiétant {
                        egui::Color32::from_rgb(0xe0, 0x6c, 0x5e)
                    } else {
                        ui.visuals().text_color()
                    }));
                    ui.monospace(format!("{:#x} + {:#x}", segment.offset, segment.filesz));
                    ui.monospace(format!("{:#x}", segment.vaddr));
                    ui.monospace(format!("{:#x}", segment.memsz));
                    ui.end_row();
                }
            });

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(ctx.t(
                "Sections — ce que l'éditeur de liens a rangé",
                "Sections — what the linker laid out",
            ))
            .strong(),
        );
        ui.add_space(2.0);
        if self.sections.is_empty() {
            ui.weak(ctx.t(
                "Aucune : le binaire a été dépouillé de sa table de sections. Il tourne \
                 quand même — le noyau ne lit que les segments.",
                "None: the binary was stripped of its section table. It still runs — the \
                 kernel only reads the segments.",
            ));
            return;
        }
        egui::Grid::new("map_sections")
            .num_columns(6)
            .striped(true)
            .min_col_width(0.0)
            .spacing([14.0, 3.0])
            .show(ui, |ui| {
                for entête in [
                    ctx.t("Nom", "Name"),
                    ctx.t("Droits", "Rights"),
                    ctx.t("Adresse", "Address"),
                    ctx.t("Dans le fichier", "In the file"),
                    ctx.t("Taille", "Size"),
                    ctx.t("Entropie", "Entropy"),
                ] {
                    ui.weak(entête);
                }
                ui.end_row();
                for section in &self.sections {
                    ui.monospace(&section.nom);
                    ui.monospace(&section.droits);
                    ui.monospace(format!("{:#x}", section.addr));
                    ui.monospace(format!("{:#x}", section.offset));
                    ui.monospace(format!("{:#x}", section.taille));
                    match section.entropie {
                        Some(valeur) => {
                            let verdict = entropy::Verdict::of(valeur);
                            ui.monospace(format!(
                                "{valeur:.2} — {}",
                                verdict.label(ctx.lang.starts_with("fr"))
                            ));
                        }
                        // Un blanc dit mieux qu'un zéro : `.bss`
                        // n'occupe rien dans le fichier, et lire là
                        // reviendrait à raconter le contenu du voisin.
                        None => {
                            ui.weak("—");
                        }
                    }
                    ui.end_row();
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn module_sur(binaire: &[u8]) -> MapModule {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: binaire,
            lang: "fr",
            names: None,
        };
        let mut module = MapModule::default();
        module.on_binary_loaded(&ctx);
        module
    }

    /// Ce qui n'est pas un ELF ne rend ni segment ni section — et ne
    /// fait rien tomber.
    #[test]
    fn what_is_not_an_elf_maps_to_nothing() {
        let module = module_sur(b"pas un elf du tout");
        assert!(module.segments.is_empty());
        assert!(module.sections.is_empty());
        let module = module_sur(&[]);
        assert!(module.segments.is_empty());
    }

    /// Sur un vrai binaire, la carte dit où est quoi.
    ///
    /// Le test se tait s'il n'y a pas de binaire d'épreuve sous la
    /// main : il vérifie une lecture, pas la présence d'un fichier.
    #[test]
    fn a_real_binary_has_its_map() {
        let chemin = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/binaires/crackme-01");
        let Ok(octets) = std::fs::read(&chemin) else {
            return;
        };
        let module = module_sur(&octets);

        assert!(!module.segments.is_empty(), "un ELF a des segments");
        // Le segment chargeable du code : lisible, exécutable, et il
        // occupe quelque chose dans le fichier.
        assert!(
            module
                .segments
                .iter()
                .any(|s| s.droits.contains('x') && s.filesz > 0),
            "un segment exécutable"
        );

        // Les sections nommées, avec leur entropie mesurée là où elles
        // occupent des octets.
        let text = module.sections.iter().find(|s| s.nom == ".text");
        if let Some(text) = text {
            assert!(text.droits.contains('x'), "`.text` s'exécute");
            let entropie = text.entropie.expect("`.text` occupe des octets");
            assert!(
                (4.0..8.0).contains(&entropie),
                "du code a l'entropie du code : {entropie}"
            );
        }
        // `.bss` n'occupe rien dans le fichier : pas d'entropie, un
        // blanc. Mesurer là reviendrait à raconter le voisin.
        if let Some(bss) = module.sections.iter().find(|s| s.nom == ".bss") {
            assert!(bss.entropie.is_none(), "`.bss` n'a rien à mesurer");
        }
    }
}
