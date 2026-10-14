use crate::{ReContext, ReModule, ReRequest, ReResult, ReSection, clickable_address, dzf};

/// Le panneau de décompilation DZF : les fonctions que DZF a rendues, en
/// pseudo-C, dans l'onglet RE.
///
/// DZF (le moteur maison) décompile et rend un JSON ; namsdec le lit (voir
/// `dzf`) et l'affiche ici. Le panneau ne lance pas DZF lui-même — c'est
/// l'application qui le fait et lui passe le résultat par
/// [`set_decompilation`] —, il montre ce qu'on lui donne.
///
/// Le lien qui compte : le nom d'une fonction est celui que **namsdec**
/// porte (`display_name`), pas celui de DZF. Une fonction rebaptisée dans
/// l'arbre d'appels l'est donc aussi dans la décompilation, sans que DZF
/// en sache rien.
#[derive(Default)]
pub struct DzfDecompilerModule {
    decompilation: Option<dzf::DzfOutput>,
    /// L'adresse qu'on veut aller voir dans le listing, relevée par
    /// l'application au prochain tour comme pour les autres panneaux.
    aller: Option<(u64, crate::GoTarget)>,
    /// On a demandé une décompilation ; l'application la lancera.
    demande_decompile: bool,
    /// Le dernier échec de décompilation, s'il y en a eu un.
    erreur: Option<String>,
}

impl DzfDecompilerModule {
    /// Reçoit une décompilation que l'application vient d'obtenir de DZF.
    pub fn set_decompilation(&mut self, sortie: dzf::DzfOutput) {
        self.decompilation = Some(sortie);
    }
}

impl ReModule for DzfDecompilerModule {
    fn section(&self) -> ReSection {
        ReSection::Decompile
    }

    fn id(&self) -> &'static str {
        "dzf_decompiler"
    }

    fn take_request(&mut self) -> Option<ReRequest> {
        if std::mem::take(&mut self.demande_decompile) {
            return Some(ReRequest::Decompile);
        }
        self.aller
            .take()
            .map(|(addr, target)| ReRequest::GoTo { addr, target })
    }

    fn deliver(&mut self, result: &ReResult) {
        match result {
            ReResult::Decompilation(sortie) => {
                self.decompilation = Some(sortie.clone());
                self.erreur = None;
            }
            ReResult::DecompilationError(message) => {
                self.erreur = Some(message.clone());
            }
            // Ce qu'un outil extérieur rend ne passe pas par ici : il a
            // sa propre rubrique, et sa sortie n'est pas du JSON DZF.
            ReResult::ToolOutput { .. }
            | ReResult::ToolState { .. }
            | ReResult::ShowBytesAt { .. } => {}
        }
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Décompilation DZF"
        } else {
            "DZF decompilation"
        }
    }

    fn on_binary_loaded(&mut self, _ctx: &ReContext) {
        // Un autre binaire : la décompilation d'avant ne le concerne plus.
        self.decompilation = None;
        self.erreur = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let Some(sortie) = &self.decompilation else {
            // Pas encore de décompilation : on dit ce que ce panneau
            // montrera, et comment l'obtenir. Un panneau vide qui ne dit
            // rien se lit comme cassé.
            ui.weak(ctx.t(
                "La décompilation que DZF produit apparaîtra ici : chaque fonction en \
                 pseudo-C, nommée comme vous l'avez nommée ailleurs.",
                "The decompilation DZF produces will appear here: each function in \
                 pseudo-C, named the way you named it elsewhere.",
            ));
            ui.add_space(4.0);
            ui.weak(ctx.t(
                "DZF est le désassembleur-décompilateur maison, en cours d'écriture. \
                 Activez-le dans Outils, et il s'affichera ici dès qu'il aura rendu.",
                "DZF is the home-grown disassembler-decompiler, under development. Enable \
                 it in Tools, and it will show here as soon as it has produced output.",
            ));
            ui.add_space(8.0);
            if ui
                .button(ctx.t("Décompiler", "Decompile"))
                .on_hover_text(ctx.t(
                    "Lancer le décompilateur actif et lire sa sortie ici.",
                    "Run the active decompiler and read its output here.",
                ))
                .clicked()
            {
                self.demande_decompile = true;
            }
            if let Some(message) = &self.erreur {
                ui.add_space(4.0);
                ui.colored_label(ui.visuals().error_fg_color, message);
            }
            return;
        };

        if sortie.functions.is_empty() {
            ui.weak(ctx.t(
                "DZF n'a rendu aucune fonction pour ce binaire.",
                "DZF returned no function for this binary.",
            ));
        }

        // Ce que DZF n'a pas su faire, en tête : une fonction manquante
        // n'est pas un vide, c'est un renoncement qu'il faut dire.
        if !sortie.diagnostics.is_empty() {
            for note in &sortie.diagnostics {
                ui.colored_label(ui.visuals().warn_fg_color, note);
            }
            ui.add_space(4.0);
        }

        let mut aller: Option<(u64, crate::GoTarget)> = None;
        for f in &sortie.functions {
            // Le nom : celui que NAMSDEC porte pour cette adresse, sinon
            // celui de DZF, sinon un sub_… par défaut.
            let défaut = f
                .name
                .clone()
                .unwrap_or_else(|| format!("sub_{:x}", f.address));
            let nom = ctx.display_name(f.address, &défaut);

            egui::CollapsingHeader::new(egui::RichText::new(&nom).monospace().strong())
                .id_salt(("dzf_fn", f.address))
                .default_open(sortie.functions.len() == 1)
                .show(ui, |ui| {
                    if let Some(cible) = clickable_address(ui, ctx, f.address) {
                        aller = Some((f.address, cible));
                    }
                    if let Some(sig) = &f.signature {
                        ui.add_space(2.0);
                        ui.monospace(
                            egui::RichText::new(sig).color(ui.visuals().weak_text_color()),
                        );
                    }
                    ui.add_space(4.0);
                    if f.pseudocode.is_empty() {
                        ui.weak(ctx.t(
                            "DZF n'a donné que la signature de cette fonction.",
                            "DZF gave only this function's signature.",
                        ));
                    } else if let Some(addr) =
                        pseudocode_cliquable(ui, &f.pseudocode, f32::INFINITY)
                    {
                        // Une adresse du pseudo-code a été cliquée :
                        // vers le CODE, sans demander. Elle y désigne un
                        // appel ou un saut, et le choix ne se pose que
                        // pour une adresse dont on ignore ce qu'elle
                        // porte.
                        aller = Some((addr, crate::GoTarget::Listing));
                    }
                });
        }
        if let Some(addr) = aller {
            self.aller = Some(addr);
        }
    }
}

/// Une frontière de mot à gauche de `i` : début de ligne, ou un
/// caractère qui n'est ni lettre, ni chiffre, ni `_`. Sans ça,
/// `var0x10` ferait prendre `0x10` pour une adresse.
fn frontière(chars: &[char], i: usize) -> bool {
    i == 0 || (!chars[i - 1].is_ascii_alphanumeric() && chars[i - 1] != '_')
}

fn commence_par(chars: &[char], i: usize, motif: &str) -> bool {
    let motif: Vec<char> = motif.chars().collect();
    i + motif.len() <= chars.len() && chars[i..i + motif.len()] == motif[..]
}

/// Les adresses qu'on peut cliquer dans du pseudo-code, chacune avec sa
/// plage **en caractères** et sa valeur : `0x401030`, `sub_401030`,
/// `loc_401030`. L'indice en caractères est celui que la galère d'egui
/// rend au clic, ce qui permet de sauter à l'adresse exacte visée et non
/// à la première de la ligne.
pub(crate) fn adresses_cliquables(texte: &str) -> Vec<(std::ops::Range<usize>, u64)> {
    let chars: Vec<char> = texte.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        // Le début de la partie hexadécimale, selon le préfixe reconnu.
        let hex_start = if frontière(&chars, i)
            && chars[i] == '0'
            && i + 1 < chars.len()
            && (chars[i + 1] == 'x' || chars[i + 1] == 'X')
        {
            Some(i + 2)
        } else if frontière(&chars, i)
            && (commence_par(&chars, i, "sub_") || commence_par(&chars, i, "loc_"))
        {
            Some(i + 4)
        } else {
            None
        };
        if let Some(hex_start) = hex_start {
            let mut j = hex_start;
            while j < chars.len() && chars[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > hex_start {
                let hex: String = chars[hex_start..j].iter().collect();
                if let Ok(valeur) = u64::from_str_radix(&hex, 16) {
                    out.push((i..j, valeur));
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Peint le pseudo-code en une seule galère — aucun `ui.horizontal`
/// imbriqué, qui ferait gonfler la fenêtre d'analyse (voir l'inspecteur
/// de l'éditeur binaire) — avec les adresses en couleur de lien. Rend
/// l'adresse cliquée, s'il y en a une sous le pointeur.
///
/// Un seul widget, dont on teste la position du clic contre les plages
/// de caractères : on saute donc à l'adresse exacte visée.
///
/// `largeur` borne le retour à la ligne : infinie pour du pseudo-code,
/// qu'on lit ligne à ligne et qui reste court, bornée pour la sortie
/// d'un outil extérieur — une ligne d'`objdump` fait parfois deux cents
/// caractères, et peinte sans borne elle court hors du panneau.
pub(crate) fn pseudocode_cliquable(ui: &mut egui::Ui, texte: &str, largeur: f32) -> Option<u64> {
    let adresses = adresses_cliquables(texte);
    let police = egui::TextStyle::Monospace.resolve(ui.style());
    let normal = ui.visuals().text_color();
    let lien = ui.visuals().hyperlink_color;
    let format = |couleur: egui::Color32| egui::TextFormat {
        font_id: police.clone(),
        color: couleur,
        ..Default::default()
    };

    let chars: Vec<char> = texte.chars().collect();
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = largeur;
    let mut pos = 0usize;
    for (plage, _) in &adresses {
        if plage.start > pos {
            job.append(
                &chars[pos..plage.start].iter().collect::<String>(),
                0.0,
                format(normal),
            );
        }
        job.append(
            &chars[plage.clone()].iter().collect::<String>(),
            0.0,
            format(lien),
        );
        pos = plage.end;
    }
    if pos < chars.len() {
        job.append(
            &chars[pos..].iter().collect::<String>(),
            0.0,
            format(normal),
        );
    }

    let galley = ui.painter().layout_job(job);
    let (rect, réponse) = ui.allocate_exact_size(galley.size(), egui::Sense::click());
    // L'adresse sous une position locale, s'il y en a une là.
    let adresse_à = |locale: egui::Vec2| -> Option<u64> {
        let indice = galley.cursor_from_pos(locale).index.0;
        adresses
            .iter()
            .find(|(plage, _)| plage.contains(&indice))
            .map(|(_, valeur)| *valeur)
    };
    // Le doigt change au survol d'une adresse : c'est ce qui dit qu'on
    // peut cliquer.
    if let Some(p) = réponse.hover_pos()
        && adresse_à(p - rect.min).is_some()
    {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    ui.painter().galley(rect.min, galley.clone(), normal);
    réponse
        .clicked()
        .then(|| réponse.interact_pointer_pos())
        .flatten()
        .and_then(|p| adresse_à(p - rect.min))
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn rendre(module: &mut DzfDecompilerModule, ctx: &ReContext) -> String {
        let egctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        let mut peint = String::new();
        for _ in 0..3 {
            peint.clear();
            let mut out = egctx.run_ui(entrée.clone(), |ui| module.ui(ui, ctx));
            for clipped in &out.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    peint.push_str(t.galley.text());
                    peint.push('|');
                }
            }
            out.textures_delta.clear();
        }
        peint
    }

    fn fonction(address: u64, name: &str, pseudocode: &str) -> dzf::DzfFunction {
        dzf::DzfFunction {
            address,
            name: Some(name.to_owned()),
            signature: None,
            pseudocode: pseudocode.to_owned(),
            size: None,
            calls: Vec::new(),
        }
    }

    /// Les trois écritures d'adresse se repèrent, avec leur valeur.
    #[test]
    fn les_adresses_du_pseudo_code_se_reperent() {
        let cas = [
            ("x = 0x401030;", vec![(4usize..12usize, 0x401030u64)]),
            ("goto loc_401016;", vec![(5..15, 0x401016)]),
            ("call sub_401020();", vec![(5..15, 0x401020)]),
            (
                "f = sub_1000 + 0x2000;",
                vec![(4..12, 0x1000), (15..21, 0x2000)],
            ),
        ];
        for (texte, attendu) in cas {
            assert_eq!(adresses_cliquables(texte), attendu, "pour « {texte} »");
        }
    }

    /// Un hexa collé à un identifiant n'est pas une adresse : `var0x10`
    /// reste un nom, et `0xtexte` n'a pas de chiffres à lire.
    #[test]
    fn un_hexa_colle_a_un_nom_n_est_pas_une_adresse() {
        assert!(adresses_cliquables("var0x10 = 1;").is_empty());
        assert!(adresses_cliquables("sub_total = 3;").is_empty());
    }

    /// Les plages sont en caractères sur TOUT le texte, sauts de ligne
    /// compris : c'est l'indice que la galère rend au clic.
    #[test]
    fn les_plages_portent_sur_tout_le_texte() {
        let texte = "a;\ngoto 0x10;";
        // « 0x10 » commence après « a;\ngoto » = 3 + 5 = 8 caractères.
        assert_eq!(adresses_cliquables(texte), vec![(8..12, 0x10)]);
    }

    /// Le nom qu'on a donné dans namsdec l'emporte sur celui de DZF : une
    /// fonction rebaptisée l'est aussi dans la décompilation.
    #[test]
    fn le_nom_de_namsdec_l_emporte_sur_celui_de_dzf() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let mut noms = std::collections::BTreeMap::new();
        noms.insert(0x401020u64, "verifie_cle".to_owned());
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &[],
            lang: "fr",
            names: Some(&noms),
        };
        let mut module = DzfDecompilerModule::default();
        let mut sortie = dzf::DzfOutput::default();
        sortie
            .functions
            .push(fonction(0x401020, "sub_401020", "return saisie == 42;"));
        module.set_decompilation(sortie);

        let peint = rendre(&mut module, &ctx);
        assert!(
            peint.contains("verifie_cle"),
            "le nom de namsdec :\n{peint}"
        );
        assert!(
            !peint.contains("sub_401020"),
            "pas le nom par défaut de DZF :\n{peint}"
        );
        assert!(
            peint.contains("return saisie == 42;"),
            "le pseudo-code (une seule fonction, dépliée) :\n{peint}"
        );
    }

    /// Sans décompilation, le panneau dit ce qu'il montrera — pas un vide
    /// qu'on lit comme cassé.
    #[test]
    fn rien_de_decompile_se_dit() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };
        let mut module = DzfDecompilerModule::default();
        let peint = rendre(&mut module, &ctx);
        assert!(peint.contains("DZF"), "le panneau se présente :\n{peint}");
    }

    /// Un autre binaire efface la décompilation d'avant.
    #[test]
    fn un_nouveau_binaire_efface_la_decompilation() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &[],
            lang: "fr",
            names: None,
        };
        let mut module = DzfDecompilerModule::default();
        let mut sortie = dzf::DzfOutput::default();
        sortie.functions.push(fonction(0x1000, "f", "x"));
        module.set_decompilation(sortie);
        assert!(module.decompilation.is_some());
        module.on_binary_loaded(&ctx);
        assert!(module.decompilation.is_none());
    }
}
