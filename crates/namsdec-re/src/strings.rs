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
}

/// Une chaîne trouvée : où elle commence, et ce qu'elle dit.
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
        self.found = match ctx.cpu.mem.peek_bytes(range.start, len) {
            Ok(bytes) => scan(bytes, range.start),
            Err(_) => Vec::new(),
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
            if !self.query.is_empty() && ui.small_button("✕").clicked() {
                self.query.clear();
            }
        });
        let query = self.query.to_lowercase();
        let montrées: Vec<&Found> = self
            .found
            .iter()
            .filter(|found| query.is_empty() || found.text.to_lowercase().contains(&query))
            .collect();
        ui.label(if query.is_empty() {
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
            .show(ui, |ui| {
                for found in &montrées {
                    // Qui la désigne ? Une chaîne à laquelle personne ne
                    // fait référence est souvent du texte mort — un
                    // reste de bibliothèque, un nom de section.
                    let vers = self.xrefs.to(found.addr);
                    ui.horizontal(|ui| {
                        ui.monospace(format!("{:08x}", found.addr));
                        if let Some(première) = vers.first() {
                            ui.weak(format!("← {:#x}", première.from)).on_hover_text(
                                if vers.len() > 1 {
                                    format!("{} {}", vers.len(), ctx.t("références", "references"))
                                } else {
                                    ctx.t("désignée ici", "pointed at here").to_owned()
                                },
                            );
                        }
                        // Tronquée, et entière dans l'infobulle : une
                        // chaîne peut faire trois cents caractères, et le
                        // panneau prendrait sa largeur.
                        ui.add(
                            egui::Label::new(egui::RichText::new(&found.text).monospace())
                                .truncate(),
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
