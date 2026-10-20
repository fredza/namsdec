//! Signaler une anomalie, sans rien envoyer dans le dos de personne.
//!
//! ## Ce que cette fenêtre est, et ce qu'elle n'est pas
//!
//! Ce n'est pas un client Bugzilla, et namsdec n'héberge aucun serveur.
//! C'est l'autre moitié du travail, celle qui coûte du temps à qui
//! signale : **rassembler le contexte**. Un rapport utile dit la
//! version, le système, les outils trouvés, ce que le programme faisait
//! et ce qu'il a répondu — et personne n'a envie de recopier tout cela
//! à la main, ce qui est la vraie raison pour laquelle les anomalies ne
//! se signalent pas.
//!
//! Le rapport est donc composé ici, **montré en entier**, et il part
//! par l'un des deux gestes de l'utilisateur :
//!
//! - **copier** : le texte va dans le presse-papiers, et il en fait ce
//!   qu'il veut — un courriel, un fil Discord, une issue ;
//! - **ouvrir le formulaire** : le navigateur s'ouvre sur les issues du
//!   dépôt, le titre et le corps déjà remplis. C'est le plus proche
//!   d'un Bugzilla qu'un programme sans serveur puisse offrir, et cela
//!   ne demande aucun compte à namsdec.
//!
//! ## Trois règles, et elles sont le sujet
//!
//! 1. **Rien ne part tout seul.** Aucune requête n'est faite à
//!    l'ouverture de la fenêtre. Le texte reste sur la machine tant
//!    qu'on n'a pas cliqué.
//! 2. **Ce qui part est montré AVANT.** Le rapport est dans un champ
//!    qu'on peut lire, corriger, vider. Un rapport qu'on n'a pas lu est
//!    un rapport qu'on n'a pas consenti à envoyer.
//! 3. **Ni le fichier, ni son chemin.** Un binaire analysé est souvent
//!    ce qu'on a de plus privé — un échantillon, un travail en cours,
//!    le code de quelqu'un d'autre. Le rapport dit SA FORME — « un ELF
//!    x86-64 de 84 548 octets, lié dynamiquement » — et jamais son nom,
//!    son chemin ni son contenu. La case « joindre le chemin du
//!    fichier » existe, décochée, pour qui le veut vraiment.

use super::*;

/// Où mènent les signalements.
///
/// Les issues du dépôt : namsdec n'héberge pas de serveur, et un
/// tracker à soi demanderait qu'on le tienne. Celui-ci existe déjà, il
/// est public, et ce qu'on y écrit reste lisible par tous — ce qui est
/// la raison même de montrer le rapport avant de l'ouvrir.
const ISSUES_URL: &str = "https://github.com/fredza/namsdec/issues/new";

/// Ce qu'on signale, tel que l'utilisateur le remplit.
#[derive(Default, Clone)]
pub(crate) struct BugReport {
    /// Le titre, une ligne.
    pub titre: String,
    /// Ce qu'on faisait, ce qu'on attendait, ce qui est arrivé.
    pub récit: String,
    /// Joindre le chemin du fichier ouvert. DÉCOCHÉ par défaut : un
    /// chemin porte souvent un nom de projet, de client, de personne.
    pub avec_chemin: bool,
}

impl NamsdecApp {
    /// La fenêtre « Signaler une anomalie ».
    pub(crate) fn bug_report_ui(&mut self, ctx: &egui::Context) {
        if !self.show_bug_report {
            return;
        }
        let s = strings(self.lang);
        let theme = self.theme;
        let mut ouvert = self.show_bug_report;
        let mut fermer = false;
        let mut copier = false;
        let mut ouvrir = false;

        egui::Window::new(s.bug_title)
            // Un identifiant explicite : sans lui, egui le dérive du
            // TITRE, qui change avec la langue — passer en anglais
            // ferait perdre à la fenêtre sa position et sa taille.
            .id(egui::Id::new("bug_report"))
            .open(&mut ouvert)
            .collapsible(false)
            .resizable(true)
            .default_width(680.0)
            .show(ctx, |ui| {
                ui.weak(s.bug_intro);
                ui.add_space(8.0);

                ui.label(s.bug_what_title);
                ui.add(
                    egui::TextEdit::singleline(&mut self.bug_report.titre)
                        .desired_width(f32::INFINITY)
                        .hint_text(s.bug_what_title_hint),
                );
                ui.add_space(6.0);
                ui.label(s.bug_what_happened);
                ui.add(
                    egui::TextEdit::multiline(&mut self.bug_report.récit)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY)
                        .hint_text(s.bug_what_happened_hint),
                );

                ui.add_space(8.0);
                ui.checkbox(&mut self.bug_report.avec_chemin, s.bug_include_path);
                ui.weak(s.bug_include_path_hint);

                ui.add_space(8.0);
                ui.separator();
                // CE QUI PARTIRA, EN ENTIER ET AVANT DE PARTIR.
                //
                // Un rapport qu'on n'a pas lu est un rapport qu'on n'a
                // pas consenti à envoyer. Il est dans un champ, donc
                // lisible, corrigible, et vidable.
                ui.strong(s.bug_preview);
                ui.weak(s.bug_preview_hint);
                ui.add_space(4.0);
                let mut rapport = self.bug_report_text();
                egui::ScrollArea::vertical()
                    .id_salt("bug_preview")
                    .max_height(260.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut rapport)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace)
                                .interactive(false),
                        );
                    });

                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if framed_button(ui, theme, s.bug_copy)
                        .on_hover_text(s.bug_copy_hint)
                        .clicked()
                    {
                        copier = true;
                    }
                    if framed_button(ui, theme, s.bug_open_form)
                        .on_hover_text(s.bug_open_form_hint)
                        .clicked()
                    {
                        ouvrir = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if framed_button(ui, theme, s.dialog_ok).clicked() {
                            fermer = true;
                        }
                    });
                });
            });

        if copier {
            ctx.copy_text(self.bug_report_text());
            self.status = Status::Note(|s| s.bug_copied);
        }
        if ouvrir {
            // Le formulaire s'ouvre PRÉREMPLI : c'est tout l'écart avec
            // un lien vers la page des issues. Ce que namsdec met dans
            // l'adresse est exactement ce que la fenêtre montrait.
            let adresse = format!(
                "{ISSUES_URL}?title={}&body={}",
                encode_url(self.bug_report.titre.trim()),
                encode_url(&self.bug_report_text())
            );
            ctx.open_url(egui::OpenUrl::new_tab(adresse));
        }
        self.show_bug_report = ouvert && !fermer;
    }

    /// Le rapport, composé.
    ///
    /// Rassemblé ICI et nulle part ailleurs : c'est la seule façon que
    /// ce qui est montré soit exactement ce qui est copié et ce qui est
    /// mis dans l'adresse du formulaire. Trois textes différents pour
    /// trois chemins seraient trois occasions de mentir.
    pub(crate) fn bug_report_text(&self) -> String {
        let s = strings(self.lang);
        let mut out = String::new();
        let récit = self.bug_report.récit.trim();
        out.push_str(s.bug_section_story);
        out.push('\n');
        out.push_str(if récit.is_empty() {
            s.bug_section_story_empty
        } else {
            récit
        });
        out.push_str("\n\n");

        out.push_str(s.bug_section_context);
        out.push('\n');
        out.push_str(&format!("- namsdec {}\n", env!("CARGO_PKG_VERSION")));
        out.push_str(&format!(
            "- {} {}\n",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        out.push_str(&format!(
            "- {} : {}\n",
            s.bug_field_language,
            self.lang.code()
        ));
        // Les outils : ce qui est là décide de ce que namsdec peut
        // faire, et la moitié des anomalies vient de leur absence.
        out.push_str(&format!(
            "- nasm : {}\n",
            self.nasm_info
                .as_ref()
                .map(|info| info.version.clone())
                .unwrap_or_else(|| s.bug_absent.to_owned())
        ));
        out.push_str(&format!(
            "- ld : {}\n",
            if namsdec_asm::find_linker() {
                s.bug_present
            } else {
                s.bug_absent
            }
        ));

        out.push('\n');
        out.push_str(s.bug_section_file);
        out.push('\n');
        // LA FORME DU FICHIER, JAMAIS SON CONTENU NI SON NOM.
        //
        // Un binaire analysé est souvent ce qu'on a de plus privé : un
        // échantillon, un travail en cours, le code de quelqu'un
        // d'autre. Ce qui aide à reproduire est sa FORME.
        match self.assemble_state.as_ref() {
            Some(state) if !state.output.file.is_empty() => {
                let taille = state.output.file.len();
                let forme = namsdec_re::elf_scan::scan(&state.output.file)
                    .map(|scan| {
                        format!(
                            "ELF64 {} · {} · {} {}",
                            scan.machine,
                            if scan.pie() { "PIE" } else { "EXEC" },
                            if scan.dynamic() { "dyn" } else { "static" },
                            if scan.stripped {
                                s.bug_stripped
                            } else {
                                s.bug_not_stripped
                            }
                        )
                    })
                    .unwrap_or_else(|| s.bug_not_elf.to_owned());
                out.push_str(&format!("- {forme}, {taille} {}\n", s.bug_bytes));
                out.push_str(&format!(
                    "- {} : {}\n",
                    s.bug_field_instructions,
                    state.disasm.len()
                ));
            }
            _ => {
                out.push_str(&format!("- {}\n", s.bug_no_file));
            }
        }
        if self.bug_report.avec_chemin
            && let Some(chemin) = self.binary_path.as_ref().or(self.current_file.as_ref())
        {
            out.push_str(&format!("- {} : {}\n", s.bug_field_path, chemin.display()));
        }

        // Ce que namsdec disait au moment où l'on a cliqué : c'est
        // souvent la seule chose qui nomme l'anomalie.
        if let Status::Error(message) = &self.status {
            out.push('\n');
            out.push_str(s.bug_section_last_error);
            out.push('\n');
            out.push_str(message);
            out.push('\n');
        }
        out
    }
}

/// Le peu d'encodage d'URL dont une adresse de formulaire a besoin.
///
/// Écrit ici plutôt que pris d'une dépendance : trois lignes contre un
/// crate de plus, pour un besoin qui ne grandira pas. La règle est
/// celle de `application/x-www-form-urlencoded` restreinte au strict
/// nécessaire — tout ce qui n'est pas un caractère sûr passe en `%XX`.
fn encode_url(texte: &str) -> String {
    let mut out = String::with_capacity(texte.len() * 2);
    for octet in texte.as_bytes() {
        match octet {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*octet as char)
            }
            _ => out.push_str(&format!("%{octet:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'encodage ne laisse passer que ce qui est sûr.
    ///
    /// Un rapport porte des retours à la ligne, des accents, des
    /// esperluettes — et une esperluette non encodée coupe l'adresse en
    /// deux, si bien que la moitié du rapport disparaît sans un mot.
    #[test]
    fn the_url_encoding_lets_nothing_dangerous_through() {
        assert_eq!(encode_url("abc-123_~."), "abc-123_~.");
        assert_eq!(encode_url("a b"), "a%20b");
        assert_eq!(encode_url("a&b=c"), "a%26b%3Dc");
        assert_eq!(encode_url("\n"), "%0A");
        // Un accent fait deux octets en UTF-8, et les deux sont encodés.
        assert_eq!(encode_url("é"), "%C3%A9");
    }
}
