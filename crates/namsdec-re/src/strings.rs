use crate::{ReContext, ReModule};

/// Les chaînes lisibles trouvées dans la mémoire du programme.
///
/// C'est la première chose qu'on cherche dans un binaire qu'on n'a pas
/// écrit : un message d'erreur, un nom de fichier, une clé. Elles disent
/// souvent ce que le code fait avant qu'on ait lu une seule instruction.
#[derive(Default)]
pub struct StringsModule {
    found: Vec<Found>,
    /// Qui désigne quoi : c'est ce qui fait la différence entre une
    /// liste de chaînes et une piste à suivre.
    xrefs: crate::xrefs::Xrefs,
    /// Ce qu'on cherche parmi elles. Deux cents chaînes se parcourent
    /// mal ; on sait en général ce qu'on veut y trouver.
    query: String,
    /// N'afficher que celles que le code désigne.
    ///
    /// Une chaîne à laquelle personne ne fait référence est souvent du
    /// texte mort — un reste de bibliothèque, un nom de section. Celles
    /// qu'une instruction va chercher sont, elles, des pistes.
    pointed_only: bool,
}

/// Une chaîne trouvée : où elle commence, et ce qu'elle dit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub addr: u64,
    pub text: String,
}

/// En dessous de quatre caractères, tout ressemble à une chaîne : deux
/// octets imprimables au milieu d'un nombre suffiraient.
const MIN_LEN: usize = 4;

/// De quoi remplir un panneau sans le noyer. Un binaire entier en contient
/// des milliers, et ce n'est pas une liste de mille entrées qu'on lit.
const MAX_FOUND: usize = 200;

/// Les chaînes d'un ELF, cherchées **là où elles se trouvent**.
///
/// Un binaire commence par du code, et du code contient à foison des
/// suites d'octets imprimables qui ne veulent rien dire : `[A\A]A^A_]`
/// n'est pas une chaîne, c'est une fin de fonction. Chercher depuis le
/// premier octet remplissait donc le panneau de bruit et s'arrêtait
/// avant d'atteindre `.rodata`, là où sont les vrais messages.
///
/// On ne lit donc que les sections qui portent des données : ni le code,
/// ni celles qui n'occupent rien dans le fichier. C'est ce que fait
/// `strings -d`, et pour la même raison.
///
/// Rend `None` quand ce n'est pas un ELF : l'appelant retombe alors sur
/// [`scan`], qui lit tout.
pub fn scan_elf(bytes: &[u8]) -> Option<Vec<Found>> {
    let elf = crate::elf_scan::scan(bytes)?;
    let mut out = Vec::new();
    for section in &elf.sections {
        if section.execute || section.nobits || section.size == 0 {
            continue;
        }
        let début = section.offset as usize;
        let fin = début.checked_add(section.size as usize)?;
        let Some(contenu) = bytes.get(début..fin) else {
            continue;
        };
        // L'adresse virtuelle quand la section en a une ; sinon sa
        // position dans le fichier, qui reste un repère utilisable.
        let base = if section.addr != 0 {
            section.addr
        } else {
            section.offset
        };
        for found in scan(contenu, base) {
            out.push(found);
            if out.len() >= MAX_FOUND {
                return Some(out);
            }
        }
    }
    Some(out)
}

/// Cherche les suites d'octets imprimables dans `bytes`, avec l'adresse
/// où chacune commence.
///
/// Fonction libre et non méthode : c'est elle qu'on teste, et elle n'a
/// besoin de rien d'autre que d'octets.
pub fn scan(bytes: &[u8], base: u64) -> Vec<Found> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut start = base;
    for (offset, byte) in bytes.iter().enumerate() {
        // Imprimable au sens de l'ASCII, plus la tabulation : le reste
        // couperait les chaînes en deux ou les remplirait de carrés.
        let printable = (0x20..0x7f).contains(byte) || *byte == b'\t';
        if printable {
            if current.is_empty() {
                start = base + offset as u64;
            }
            current.push(*byte as char);
            continue;
        }
        if current.len() >= MIN_LEN {
            out.push(Found {
                addr: start,
                text: std::mem::take(&mut current),
            });
            if out.len() >= MAX_FOUND {
                return out;
            }
        } else {
            current.clear();
        }
    }
    if current.len() >= MIN_LEN {
        out.push(Found {
            addr: start,
            text: current,
        });
    }
    out
}

impl ReModule for StringsModule {
    fn id(&self) -> &'static str {
        "strings"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Chaînes lisibles"
        } else {
            "Readable strings"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        let range = ctx.cpu.mem.range();
        let len = (range.end - range.start) as usize;
        // Le fichier d'abord, quand on l'a : ses sections disent où sont
        // les données, et chercher ailleurs ne rapporte que du code qui
        // ressemble à du texte. À défaut — un tampon brut, un programme
        // qu'on vient d'assembler — on lit la mémoire telle quelle.
        self.found = match scan_elf(ctx.binary) {
            Some(trouvées) => {
                // Les adresses viennent des sections — des adresses
                // virtuelles. Le listing, lui, part parfois du premier
                // octet du fichier : il faut alors les ramener là où le
                // code les désigne, sans quoi aucune référence croisée
                // ne tomberait juste.
                let base = ctx.disasm.first().map(|first| first.address);
                trouvées
                    .into_iter()
                    .map(|found| match base {
                        Some(base) if !range.contains(&found.addr) => {
                            match crate::imports::file_offset(ctx.binary, found.addr) {
                                Some(offset) => Found {
                                    addr: base + offset,
                                    ..found
                                },
                                None => found,
                            }
                        }
                        _ => found,
                    })
                    .collect()
            }
            None => match ctx.cpu.mem.peek_bytes(range.start, len) {
                Ok(bytes) => scan(bytes, range.start),
                Err(_) => Vec::new(),
            },
        };
        // « Mot de passe » quelque part dans les données ne dit rien ;
        // « mot de passe, lu ici » dit par où entrer.
        self.xrefs = crate::xrefs::scan(ctx.disasm, range);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.found.is_empty() {
            ui.label(ctx.t("Aucune chaîne trouvée.", "No string found."));
            return;
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(ctx.t("chercher…", "search…"))
                    .desired_width(140.0),
            );
            if !self.query.is_empty() && ui.small_button("x").clicked() {
                self.query.clear();
            }
            ui.checkbox(
                &mut self.pointed_only,
                ctx.t("désignées seulement", "pointed at only"),
            )
            .on_hover_text(ctx.t(
                "Une chaîne que personne ne va chercher est souvent du texte mort.",
                "A string nobody goes looking for is usually dead text.",
            ));
        });
        let query = self.query.to_lowercase();
        let montrées: Vec<&Found> = self
            .found
            .iter()
            .filter(|found| query.is_empty() || found.text.to_lowercase().contains(&query))
            .filter(|found| !self.pointed_only || !self.xrefs.to(found.addr).is_empty())
            .collect();
        ui.label(if query.is_empty() && !self.pointed_only {
            format!(
                "{} {}",
                self.found.len(),
                ctx.t(
                    "chaînes d'au moins quatre caractères",
                    "strings of at least four characters"
                )
            )
        } else {
            format!(
                "{} / {} {}",
                montrées.len(),
                self.found.len(),
                ctx.t("chaînes", "strings")
            )
        });
        egui::ScrollArea::vertical()
            .id_salt("re_strings")
            .max_height(220.0)
            // Toute la largeur du panneau : sans cela, la zone se réduit
            // à la plus longue ligne *déjà* affichée, et les chaînes
            // suivantes se tronquent bien avant le bord.
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for found in &montrées {
                    // Qui la désigne ? Une chaîne à laquelle personne ne
                    // fait référence est souvent du texte mort — un
                    // reste de bibliothèque, un nom de section.
                    let vers = self.xrefs.to(found.addr);
                    ui.horizontal(|ui| {
                        ui.monospace(format!("{:08x}", found.addr));
                        if let Some(première) = vers.first() {
                            ui.weak(format!("<- {:#x}", première.from)).on_hover_text(
                                if vers.len() > 1 {
                                    format!("{} {}", vers.len(), ctx.t("références", "references"))
                                } else {
                                    ctx.t("désignée ici", "pointed at here").to_owned()
                                },
                            );
                        }
                        // Tronquée à la largeur qui *reste*, et entière
                        // dans l'infobulle : une chaîne peut faire trois
                        // cents caractères, et le panneau prendrait sa
                        // largeur. Dimensionnée explicitement, sinon
                        // `truncate` coupe à la largeur idéale du texte
                        // et laisse le reste de la ligne vide.
                        let reste = ui.available_width().max(24.0);
                        ui.add_sized(
                            [reste, ui.spacing().interact_size.y],
                            egui::Label::new(egui::RichText::new(&found.text).monospace())
                                .truncate()
                                .halign(egui::Align::LEFT),
                        )
                        .on_hover_text(&found.text);
                    });
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sur un vrai binaire, les chaînes cherchées dans les données
    /// valent celles qu'on lirait avec `strings -d` — et non les
    /// fragments de code qui leur ressemblent.
    ///
    /// C'est ce qui manquait : en partant du premier octet du fichier,
    /// les deux cents premières « chaînes » étaient des fins de
    /// fonctions, et la recherche s'arrêtait avant d'atteindre les vrais
    /// messages.
    #[test]
    fn a_real_binary_yields_its_data_not_its_code() {
        let Ok(bytes) = std::fs::read("/bin/ls") else {
            eprintln!("/bin/ls introuvable : test ignoré");
            return;
        };
        let trouvées = scan_elf(&bytes).expect("un ELF");
        assert!(!trouvées.is_empty());
        // L'interpréteur dynamique est une chaîne, et elle est dans une
        // section de données : c'est le repère le plus sûr qui soit.
        assert!(
            trouvées.iter().any(|found| found.text.contains("ld-linux")),
            "les vraies chaînes ne sont pas atteintes"
        );
        // Et les fragments de code ne sont plus là : aucune adresse
        // trouvée ne tombe dans une section exécutable.
        let elf = crate::elf_scan::scan(&bytes).expect("un ELF");
        for found in &trouvées {
            for section in elf.sections.iter().filter(|s| s.execute) {
                let fin = section.addr + section.size;
                assert!(
                    !(section.addr..fin).contains(&found.addr),
                    "« {} » vient de {}, qui est du code",
                    found.text,
                    section.name
                );
            }
        }
    }

    /// Le filtre « désignées seulement » sépare les pistes du texte
    /// mort : une chaîne que personne ne va chercher n'apprend rien.
    #[test]
    fn showing_only_the_strings_the_code_points_at() {
        // lea rax, [rel 0x401000] ; hlt — le code désigne la première
        // chaîne, pas la seconde.
        let code = [0x48, 0x8d, 0x05, 0xf9, 0x0f, 0x00, 0x00, 0xf4];
        let disasm = namsdec_disasm::disassemble(&code, 0x400000);
        let xrefs = crate::xrefs::scan(&disasm, 0x400000..0x402000);

        let trouvées = vec![
            Found {
                addr: 0x401000,
                text: "désignée".to_owned(),
            },
            Found {
                addr: 0x401100,
                text: "orpheline".to_owned(),
            },
        ];
        let désignées: Vec<&Found> = trouvées
            .iter()
            .filter(|found| !xrefs.to(found.addr).is_empty())
            .collect();
        assert_eq!(désignées.len(), 1, "{désignées:?}");
        assert_eq!(désignées[0].text, "désignée");
    }

    /// Une chaîne occupe la largeur qu'on lui donne, et se tronque au
    /// bord du panneau — pas trois centimètres avant.
    ///
    /// `truncate` seul coupe à la largeur *idéale* du texte et laisse le
    /// reste de la ligne vide : dans un panneau de quatre cents pixels,
    /// les chaînes s'arrêtaient au milieu.
    #[test]
    fn a_string_uses_the_width_it_is_given() {
        let mut module = StringsModule {
            found: vec![
                Found {
                    addr: 0x401000,
                    text: "abcd".to_owned(),
                },
                Found {
                    addr: 0x401010,
                    text: "une chaîne bien plus longue que la précédente, pour voir".to_owned(),
                },
            ],
            ..Default::default()
        };
        let mem = namsdec_core::Memory::new(0x400000, 0x100);
        let cpu = namsdec_core::Cpu::new(mem);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 600.0),
            )),
            ..Default::default()
        };
        let mut largeurs: Vec<(String, f32)> = Vec::new();
        for _ in 0..2 {
            largeurs.clear();
            let mut output = ctx.run_ui(input.clone(), |ui| {
                let re = ReContext {
                    cpu: &cpu,
                    disasm: &[],
                    source: "",
                    binary: &[],
                    lang: "fr",
                };
                module.ui(ui, &re);
            });
            for clipped in &output.shapes {
                if let egui::epaint::Shape::Text(forme) = &clipped.shape {
                    let texte = forme.galley.text().to_owned();
                    if texte.starts_with("abcd") || texte.starts_with("une chaîne") {
                        largeurs.push((texte, forme.galley.size().x));
                    }
                }
            }
            output.textures_delta.clear();
        }
        let longue = largeurs
            .iter()
            .find(|(texte, _)| texte.starts_with("une chaîne"))
            .unwrap_or_else(|| panic!("la longue chaîne n'est pas peinte : {largeurs:?}"));
        // Elle doit occuper l'essentiel des quatre cents pixels, moins
        // l'adresse et les marges.
        assert!(
            longue.1 > 200.0,
            "la chaîne s'arrête à {} pixels sur 400",
            longue.1
        );
    }

    /// Ce qui n'est pas un ELF se lit quand même, du premier octet au
    /// dernier : un tampon brut n'a pas de sections à consulter.
    #[test]
    fn a_raw_buffer_is_still_read_whole() {
        assert!(scan_elf(b"pas un ELF, juste des octets").is_none());
        let trouvées = scan(b"\x00\x01salut tout le monde\x00", 0x1000);
        assert_eq!(trouvées.len(), 1);
        assert_eq!(trouvées[0].text, "salut tout le monde");
        assert_eq!(trouvées[0].addr, 0x1002);
    }
    use super::*;

    #[test]
    fn a_run_of_printable_bytes_is_a_string() {
        let bytes = b"\x00\x01salut, monde\n\x00xy\x00mot de passe";
        let found = scan(bytes, 0x400000);
        let texts: Vec<&str> = found.iter().map(|f| f.text.as_str()).collect();
        // « xy » est trop court pour compter ; le saut de ligne coupe.
        assert_eq!(texts, ["salut, monde", "mot de passe"]);
        assert_eq!(
            found[0].addr, 0x400002,
            "l'adresse est celle du premier octet"
        );
    }

    #[test]
    fn a_program_full_of_text_does_not_fill_the_panel_for_ever() {
        let mut bytes = Vec::new();
        for _ in 0..(MAX_FOUND * 2) {
            bytes.extend_from_slice(b"abcd\x00");
        }
        assert_eq!(scan(&bytes, 0).len(), MAX_FOUND);
    }
}
