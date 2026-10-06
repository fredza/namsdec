use crate::entropy::{self, Verdict, Window, seuils};
use crate::{ReContext, ReModule};

/// Le profil d'entropie du fichier, tranche par tranche.
///
/// Ce que ça répond, et qu'aucun autre module ne répond : « où est le
/// vrai code ? » Un binaire empaqueté montre un petit déchiffreur en
/// clair et un gros bloc illisible ; un binaire ordinaire montre une
/// courbe plate. On le voit en une image, sans rien lire.
///
/// Mesuré sur le **fichier**, et non sur le désassemblage : c'est le
/// fichier entier qui porte l'en-tête, les chaînes, le remplissage et les
/// données, et c'est leur voisinage qui raconte quelque chose.
#[derive(Default)]
pub struct EntropyModule {
    windows: Vec<Window>,
    whole: f64,
    size: usize,
    /// L'empaqueteur, quand il se nomme lui-même.
    ///
    /// Ici et non dans un panneau à part : c'est ce panneau qui dit
    /// « compressé, chiffré, ou empaqueté », et le nommer est le cran
    /// d'après — la réponse à la question que la mesure vient de poser.
    packers: Vec<crate::packer::Found>,
}

impl ReModule for EntropyModule {
    fn id(&self) -> &'static str {
        "entropy"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Entropie"
        } else {
            "Entropy"
        }
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        self.size = ctx.binary.len();
        self.whole = entropy::shannon(ctx.binary);
        // Soixante-quatre tranches : assez pour qu'une bosse se voie,
        // assez peu pour que chacune porte de quoi mesurer.
        self.windows = entropy::profile(ctx.binary, 64);
        self.packers = crate::packer::detect(ctx.binary);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.windows.is_empty() {
            ui.weak(ctx.t(
                "Rien à mesurer : aucun fichier chargé.",
                "Nothing to measure: no file loaded.",
            ));
            return;
        }

        let fr = ctx.lang.starts_with("fr");
        // Les deux mesures côte à côte, le profil à droite : la bande
        // pleine largeur prenait l'oeil avant le reste, alors que c'est
        // un indice parmi sept. Ce qu'on lit d'abord est le chiffre et
        // son verdict ; le dessin sert à savoir OÙ, non à savoir quoi.
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(ctx.t("Sur tout le fichier", "Whole file"));
                    ui.monospace(format!("{:.2}", self.whole));
                });
                ui.weak(Verdict::of(self.whole).label(fr));
            });
            ui.add_space(12.0);
            histogram(ui, &self.windows);
        });

        // L'empaqueteur quand il se nomme : la mesure dit « il y a
        // quelque chose d'illisible ici », le nom dit quoi en faire.
        // Avant la tranche la plus dense, parce que c'est la réponse et
        // non un détail de plus.
        for empaqueteur in &self.packers {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.strong(
                    egui::RichText::new(empaqueteur.name)
                        .color(egui::Color32::from_rgb(0xf4, 0x87, 0x71)),
                );
                // La preuve avec le nom, toujours : « c'est de l'UPX »
                // sans dire pourquoi demande de faire confiance.
                ui.monospace(empaqueteur.clue.label(ctx.lang));
            });
            ui.indent(empaqueteur.name, |ui| {
                ui.weak(empaqueteur.text(ctx.lang));
            });
        }

        ui.add_space(6.0);
        // La tranche la plus haute, nommée par son décalage : c'est
        // l'endroit où aller voir, et le donner en hexadécimal évite de
        // le convertir pour le chercher dans le vidage.
        if let Some(haute) = self
            .windows
            .iter()
            .max_by(|a, b| a.entropy.total_cmp(&b.entropy))
        {
            ui.horizontal_wrapped(|ui| {
                ui.label(ctx.t("Le plus dense", "Densest"));
                ui.monospace(format!(
                    "{:#x}..{:#x}",
                    haute.offset,
                    haute.offset + haute.len
                ));
                ui.monospace(format!("{:.2}", haute.entropy));
                ui.weak(Verdict::of(haute.entropy).label(fr));
            });
        }

        // Ce que la mesure ne dit pas. Une entropie haute sur un petit
        // bloc n'est pas une preuve, et laisser croire le contraire
        // ferait chercher un empaqueteur là où il n'y en a pas.
        ui.add_space(4.0);
        // Le chiffre vient de `entropy::seuils` : la phrase et le repère
        // tracé plus bas parlent du même réglage, et doivent donc le
        // lire au même endroit.
        ui.weak(if fr {
            format!(
                "Une piste, non un verdict : un bloc de code dense monte à {} sans être \
                 chiffré.",
                format!("{:.1}", seuils::DENSE_CODE).replace('.', ",")
            )
        } else {
            format!(
                "A hint, not a verdict: dense code reaches {:.1} without being encrypted.",
                seuils::DENSE_CODE
            )
        });
    }
}

/// La courbe, dessinée à la main.
///
/// Une barre par tranche, du bas vers le haut, l'échelle allant de 0 à 8
/// — **toujours**, et non de la plus basse à la plus haute valeur vue :
/// une échelle qui s'ajuste ferait paraître spectaculaire l'écart entre
/// 5,9 et 6,1, et l'oeil y verrait une bosse qui n'existe pas.
fn histogram(ui: &mut egui::Ui, windows: &[Window]) {
    // Assez haut pour qu'une bosse se voie, assez bas pour ne pas
    // occuper le tiers de la fenêtre d'analyse. Soixante-quatre pixels
    // en travers de toute la largeur faisaient une bande qui prenait
    // l'oeil avant le reste — or c'est un indice parmi sept, non le
    // sujet.
    const HAUTEUR: f32 = 36.0;
    // Bornée aussi en largeur : étirée sur une fenêtre de mille pixels,
    // chaque colonne devenait un trait isolé au milieu du vide, et la
    // forme du profil se perdait au lieu de se lire.
    let largeur = ui.available_width().clamp(64.0, 520.0);
    let (rect, réponse) =
        ui.allocate_exact_size(egui::vec2(largeur, HAUTEUR), egui::Sense::hover());
    let peintre = ui.painter_at(rect);
    let visuels = ui.visuals().clone();

    peintre.rect_filled(rect, 2.0, visuels.extreme_bg_color);
    let pas = largeur / windows.len() as f32;

    for (i, fenêtre) in windows.iter().enumerate() {
        let part = (fenêtre.entropy / 8.0).clamp(0.0, 1.0) as f32;
        let haut = rect.bottom() - part * HAUTEUR;
        let barre = egui::Rect::from_min_max(
            egui::pos2(rect.left() + i as f32 * pas, haut),
            egui::pos2(rect.left() + (i as f32 + 1.0) * pas, rect.bottom()),
        );
        // La couleur suit le verdict : on lit la nature du bloc sans
        // avoir à comparer des hauteurs entre elles.
        let couleur = match Verdict::of(fenêtre.entropy) {
            Verdict::Flat => visuels.weak_text_color(),
            Verdict::Text | Verdict::Code => visuels.selection.bg_fill,
            Verdict::Mixed => visuels.warn_fg_color,
            Verdict::Random => visuels.error_fg_color,
        };
        peintre.rect_filled(barre, 0.0, couleur);
    }

    // Le repère d'empaquetage : la ligne au-dessus de laquelle il n'y a
    // plus de structure à voir. Sans elle, les hauteurs ne se comparent
    // qu'entre elles, et l'on ne sait pas si une barre est haute dans
    // l'absolu. La valeur vient de `entropy::seuils`, comme celle qui
    // décide des couleurs juste au-dessus.
    let seuil = rect.bottom() - (seuils::EMPAQUETÉ / 8.0) as f32 * HAUTEUR;
    peintre.line_segment(
        [
            egui::pos2(rect.left(), seuil),
            egui::pos2(rect.right(), seuil),
        ],
        egui::Stroke::new(1.0, visuels.weak_text_color()),
    );

    // Au survol, la tranche visée se dit : son décalage et sa mesure.
    if let Some(position) = réponse.hover_pos() {
        let index = (((position.x - rect.left()) / pas) as usize).min(windows.len() - 1);
        let fenêtre = windows[index];
        réponse.on_hover_text(format!(
            "{:#x}..{:#x}\n{:.2}",
            fenêtre.offset,
            fenêtre.offset + fenêtre.len,
            fenêtre.entropy
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use namsdec_core::{Cpu, Memory};

    fn contexte<'a>(binaire: &'a [u8], cpu: &'a Cpu) -> ReContext<'a> {
        ReContext {
            cpu,
            disasm: &[],
            source: "",
            binary: binaire,
            lang: "fr",
            names: None,
        }
    }

    /// Le module mesure ce qu'on lui donne, et nomme le bloc le plus
    /// dense.
    #[test]
    fn the_module_finds_the_densest_block() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let mut binaire = vec![0u8; 4096];
        let mut graine = 0x2545f4914f6cdd1du64;
        for _ in 0..4096 {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            binaire.push((graine >> 24) as u8);
        }

        let mut module = EntropyModule::default();
        module.on_binary_loaded(&contexte(&binaire, &cpu));

        assert!(!module.windows.is_empty(), "des tranches sont mesurées");
        let haute = module
            .windows
            .iter()
            .max_by(|a, b| a.entropy.total_cmp(&b.entropy))
            .unwrap();
        assert!(haute.offset >= 4096, "la bosse est dans la seconde moitié");
        assert_eq!(Verdict::of(haute.entropy), Verdict::Random);
    }

    /// Un fichier vide ne fait rien tomber, et le dit.
    #[test]
    fn an_empty_file_says_so_instead_of_breaking() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let mut module = EntropyModule::default();
        module.on_binary_loaded(&contexte(&[], &cpu));
        assert!(module.windows.is_empty());
        assert_eq!(module.whole, 0.0);
    }

    /// Un empaqueteur qui se nomme est nommé, avec sa preuve.
    ///
    /// C'est le cran d'après la mesure : « compressé, chiffré, ou
    /// empaqueté » est juste et frustrant ; « UPX, et `upx -d` le rendra »
    /// est actionnable.
    #[test]
    fn a_packer_that_names_itself_is_named() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let mut binaire = vec![0u8; 512];
        binaire.extend_from_slice(b"UPX!");
        binaire.extend(std::iter::repeat_n(0x90u8, 1024));

        let mut module = EntropyModule::default();
        module.on_binary_loaded(&contexte(&binaire, &cpu));
        assert_eq!(module.packers.len(), 1, "{:?}", module.packers);
        assert_eq!(module.packers[0].name, "UPX");

        // Et il est écrit dans le panneau, preuve comprise.
        let egui_ctx = egui::Context::default();
        let ctx = contexte(&binaire, &cpu);
        let mut textes: Vec<String> = Vec::new();
        for _ in 0..2 {
            textes.clear();
            let mut sortie = egui_ctx.run_ui(Default::default(), |ui| module.ui(ui, &ctx));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    textes.push(texte.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        let tout = textes.join(" | ");
        assert!(tout.contains("UPX"), "{tout}");
        assert!(tout.contains("0x200"), "la preuve se vérifie : {tout}");
    }

    /// Le titre suit la langue, comme celui des autres modules.
    #[test]
    fn the_title_follows_the_language() {
        let module = EntropyModule::default();
        assert_eq!(module.title("fr"), "Entropie");
        assert_eq!(module.title("en"), "Entropy");
    }
}
