use crate::{ReContext, ReModule, ReSection, digest, strings};

/// Une règle YARA tirée du fichier chargé : de quoi reconnaître d'autres
/// échantillons de la même famille.
///
/// C'est le geste qui suit l'analyse : on a compris un binaire, on veut
/// retrouver ses frères dans un tas. YARA est le langage que lisent les
/// antivirus et les plateformes d'échantillons ; une règle dit « ce
/// fichier contient telles chaînes, commence par tels octets, pèse tant »,
/// et un moteur la passe sur des milliers de fichiers.
///
/// La règle est un **point de départ**, pas un verdict : namsdec prend les
/// chaînes les plus distinctives et pose un seuil prudent. L'analyste la
/// resserre (plus de chaînes exigées) ou l'élargit (motifs à la place de
/// chaînes exactes) selon ce qu'il cherche. Elle se calcule une fois, au
/// chargement, comme les empreintes.
#[derive(Default)]
pub struct YaraModule {
    rule: String,
}

impl ReModule for YaraModule {
    fn section(&self) -> ReSection {
        ReSection::Says
    }

    fn id(&self) -> &'static str {
        "yara"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Règles YARA"
        } else {
            "YARA rules"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.rule = build_rule(ctx.binary);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.rule.is_empty() {
            ui.weak(ctx.t(
                "Rien de chargé : il n'y a pas de fichier d'où tirer une règle.",
                "Nothing loaded: there is no file to derive a rule from.",
            ));
            return;
        }
        ui.weak(ctx.t(
            "Un point de départ pour reconnaître d'autres échantillons de la même \
             famille, pas un verdict. On la resserre (plus de chaînes exigées) ou on \
             l'élargit (motifs plutôt que chaînes exactes) selon ce qu'on cherche.",
            "A starting point for recognizing other samples of the same family, not a \
             verdict. Tighten it (require more strings) or widen it (patterns instead of \
             exact strings) depending on what you are after.",
        ));
        ui.add_space(6.0);

        // La règle entière, cliquable pour la copier : une règle YARA sert
        // à être collée dans un fichier `.yar`, pas recopiée à la main.
        if ui
            .add(
                egui::Label::new(egui::RichText::new(&self.rule).monospace())
                    .sense(egui::Sense::click()),
            )
            .on_hover_text(ctx.t("Cliquer pour copier la règle", "Click to copy the rule"))
            .clicked()
        {
            ui.ctx().copy_text(self.rule.clone());
        }
    }
}

/// Les chaînes communes à presque tous les binaires : les garder dans une
/// règle la ferait répondre « oui » sur n'importe quel ELF compilé par
/// GCC, ce qui n'identifie rien. On les écarte.
fn est_du_bruit(s: &str) -> bool {
    const BRUIT: [&str; 11] = [
        "GLIBC",
        "GLIBCXX",
        "GCC:",
        "/lib",
        ".so",
        "__libc",
        "__cxa",
        "__gmon",
        "stack_chk",
        "ITM_",
        "CXXABI",
    ];
    BRUIT.iter().any(|motif| s.contains(motif)) || s.starts_with("lib")
}

/// Les chaînes les plus distinctives du fichier : assez longues, porteuses
/// de lettres, ni communes à tous les binaires, ni dupliquées.
fn distinctive_strings(bytes: &[u8]) -> Vec<String> {
    let trouvées = strings::scan_elf(bytes).unwrap_or_else(|| strings::scan(bytes, 0));
    let mut vues = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for found in trouvées {
        let s = found.text.trim();
        // Trop courte, ce n'est pas distinctif ; sans quelques lettres,
        // c'est du bruit symbolique ; du bruit de bibliothèque, idem.
        if s.len() < 6 || s.chars().filter(|c| c.is_ascii_alphabetic()).count() < 3 {
            continue;
        }
        if est_du_bruit(s) {
            continue;
        }
        // Bornée : une chaîne de deux cents octets alourdit la règle sans
        // la rendre plus sûre — les soixante premiers suffisent à viser.
        let s: String = s.chars().take(60).collect();
        if vues.insert(s.clone()) {
            out.push(s);
        }
        if out.len() >= 10 {
            break;
        }
    }
    out
}

/// Échappe ce qu'une chaîne YARA entre guillemets ne supporte pas tel quel.
fn yara_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Bâtit la règle YARA du fichier : méta (empreinte, taille), chaînes
/// distinctives, et une condition qui combine le nombre magique, la taille
/// et un seuil de chaînes.
///
/// Fonction libre, pour l'éprouver sans interface : elle ne tient qu'aux
/// octets.
pub fn build_rule(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let sha = digest::hex(&digest::sha256(bytes));
    let crc = digest::crc32(bytes);
    let taille = bytes.len();
    let chaînes = distinctive_strings(bytes);

    // Le nombre magique du format, quand on le reconnaît : il écarte
    // d'emblée tout ce qui n'est pas du même type de fichier.
    let magie = if bytes.starts_with(b"\x7fELF") {
        Some("uint32(0) == 0x464c457f") // ELF
    } else if bytes.starts_with(b"MZ") {
        Some("uint16(0) == 0x5a4d") // PE/MZ
    } else {
        None
    };

    let mut r = String::new();
    r.push_str(&format!("rule namsdec_{crc:08x}\n{{\n"));
    r.push_str("    meta:\n");
    r.push_str("        description = \"Généré par namsdec\"\n");
    r.push_str(&format!("        sha256 = \"{sha}\"\n"));
    r.push_str(&format!("        size = {taille}\n"));

    if !chaînes.is_empty() {
        r.push_str("    strings:\n");
        for (i, s) in chaînes.iter().enumerate() {
            r.push_str(&format!("        $s{i} = \"{}\" ascii\n", yara_escape(s)));
        }
    }

    r.push_str("    condition:\n");
    let mut conditions = Vec::new();
    if let Some(m) = magie {
        conditions.push(m.to_owned());
    }
    conditions.push(format!(
        "filesize < {}",
        taille.saturating_mul(2).max(taille + 1)
    ));
    if !chaînes.is_empty() {
        // Un seuil prudent : exiger TOUTES les chaînes casse la règle au
        // moindre écart entre deux variantes ; un sous-ensemble tient la
        // famille sans la perdre au premier changement.
        let seuil = chaînes.len().clamp(1, 4);
        if chaînes.len() > seuil {
            conditions.push(format!("{seuil} of them"));
        } else {
            conditions.push("all of them".to_owned());
        }
    }
    r.push_str(&format!("        {}\n", conditions.join(" and\n        ")));
    r.push_str("}\n");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La règle porte un nom valide, l'empreinte du fichier, et se termine
    /// sur une condition — c'est une règle YARA qu'un moteur accepte.
    #[test]
    fn the_rule_is_well_formed() {
        let mut bytes = b"\x7fELF".to_vec();
        bytes.extend_from_slice(b"motif_distinctif_de_la_famille_X");
        bytes.extend(std::iter::repeat_n(0u8, 200));
        let r = build_rule(&bytes);
        assert!(r.starts_with("rule namsdec_"), "{r}");
        assert!(r.contains("meta:") && r.contains("condition:"), "{r}");
        assert!(r.contains(&digest::hex(&digest::sha256(&bytes))), "{r}");
        // Le nombre magique de l'ELF est dans la condition.
        assert!(r.contains("uint32(0) == 0x464c457f"), "{r}");
        // La chaîne distinctive y est, échappée en chaîne YARA.
        assert!(r.contains("motif_distinctif_de_la_famille_X"), "{r}");
        assert!(r.trim_end().ends_with('}'), "{r}");
    }

    /// Le bruit de bibliothèque est écarté : une règle qui exige « GLIBC »
    /// répondrait « oui » sur tout ELF compilé par GCC.
    #[test]
    fn library_noise_is_left_out() {
        assert!(est_du_bruit("GLIBC_2.2.5"));
        assert!(est_du_bruit("libpthread.so.0"));
        assert!(est_du_bruit("__cxa_finalize"));
        assert!(!est_du_bruit("telecharge_charge_utile"));
    }

    /// Les guillemets et les antislash sont échappés : sans quoi la règle
    /// produite ne compilerait pas.
    #[test]
    fn quotes_and_backslashes_are_escaped() {
        assert_eq!(yara_escape(r#"C:\a "b""#), r#"C:\\a \"b\""#);
    }

    /// Rien de chargé : pas de règle, et le module le dira plutôt que de
    /// montrer la règle du vide.
    #[test]
    fn nothing_loaded_has_no_rule() {
        assert!(build_rule(&[]).is_empty());
    }
}
