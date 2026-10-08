use crate::{ReContext, ReModule, ReRequest, ReResult, clickable_address, dzf};

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
    aller: Option<u64>,
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
    fn id(&self) -> &'static str {
        "dzf_decompiler"
    }

    fn take_request(&mut self) -> Option<ReRequest> {
        if std::mem::take(&mut self.demande_decompile) {
            return Some(ReRequest::Decompile);
        }
        self.aller.take().map(|addr| ReRequest::GoTo { addr })
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

        let mut aller: Option<u64> = None;
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
                    if clickable_address(ui, ctx, f.address) {
                        aller = Some(f.address);
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
                    } else {
                        // Le pseudo-code, sélectionnable pour être copié.
                        ui.add(
                            egui::Label::new(egui::RichText::new(&f.pseudocode).monospace())
                                .selectable(true),
                        );
                    }
                });
        }
        if let Some(addr) = aller {
            self.aller = Some(addr);
        }
    }
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
