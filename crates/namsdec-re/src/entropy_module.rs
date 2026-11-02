use crate::entropy::{self, Range, Verdict, Window, seuils};
use crate::vue::{Bloc, Dessin, Fragment, Ligne, Ton, Vue};
use crate::{ReContext, ReModule, ReRequest, ReSection};

/// Le verdict d'une mesure d'entropie, dit en TON et non en couleur.
///
/// La table des teintes vit dans le peintre, avec son dégradé continu ;
/// ici on ne dit que ce que la valeur VAUT. Les seuils sont ceux de
/// `entropy::seuils`, comme partout ailleurs — la phrase du bas du
/// panneau, les couleurs du profil et ce verdict parlent du même
/// réglage, et doivent donc le lire au même endroit.
fn teinte(entropie: f64) -> Ton {
    if entropie >= seuils::EMPAQUETÉ {
        Ton::Mauvais
    } else if entropie >= seuils::DENSE_CODE {
        Ton::Attention
    } else {
        Ton::Normal
    }
}

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
    /// La taille d'une tranche, en octets. Elle suit la taille du
    /// fichier — voir [`entropy::window_size`].
    window: usize,
    /// Les tranches voisines de même verdict, réunies : c'est la forme
    /// qui porte les adresses, et celle qu'on lit.
    ranges: Vec<Range>,
    /// L'adresse virtuelle du début de chaque plage, quand cette
    /// position du fichier est projetée quelque part.
    addresses: Vec<Option<u64>>,
    whole: f64,
    size: usize,
    /// Ce que le panneau demande à l'application : aller voir une
    /// adresse. Relevé après le dessin, jamais pendant.
    demande: Option<ReRequest>,
    /// L'empaqueteur, quand il se nomme lui-même.
    ///
    /// Ici et non dans un panneau à part : c'est ce panneau qui dit
    /// « compressé, chiffré, ou empaqueté », et le nommer est le cran
    /// d'après — la réponse à la question que la mesure vient de poser.
    packers: Vec<crate::packer::Found>,
}

/// Combien de plages la liste montre avant de s'arrêter.
const PLAGES_MONTRÉES: usize = 40;

impl ReModule for EntropyModule {
    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn section(&self) -> ReSection {
        ReSection::File
    }

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
        // La tranche suit la taille du fichier : soixante-quatre
        // tranches quoi qu'il arrive noyaient un déchiffreur de deux
        // cents octets dans une moyenne de seize kilo-octets.
        self.window = entropy::window_size(ctx.binary.len());
        self.windows = entropy::profile_by(ctx.binary, self.window);
        self.ranges = entropy::ranges(&self.windows);
        self.addresses = self
            .ranges
            .iter()
            .map(|plage| crate::imports::virtual_address(ctx.binary, plage.offset as u64))
            .collect();
        self.packers = crate::packer::detect(ctx.binary);
    }

    fn vue(&self, ctx: &ReContext) -> Option<Vue> {
        if self.windows.is_empty() {
            return Some(
                Vue::neuve().texte(
                    Fragment::dit(ctx.t(
                        "Rien à mesurer : aucun fichier chargé.",
                        "Nothing to measure: no file loaded.",
                    ))
                    .faible(),
                ),
            );
        }
        let fr = ctx.lang.starts_with("fr");
        // Ce qu'on lit d'abord : le chiffre et son verdict. Puis le
        // découpage, parce qu'un profil ne veut rien dire sans la
        // taille de ses tranches — une bosse sur 256 octets et une
        // bosse sur 64 kilo-octets ne disent pas la même chose.
        let mut blocs = vec![
            Bloc::Texte(
                vec![
                    Fragment::dit(ctx.t("Sur tout le fichier", "Whole file")),
                    Fragment::dit(format!("{:.2}", self.whole)).code(),
                    Fragment::dit(Verdict::of(self.whole).label(fr)).faible(),
                    // Des fragments SÉPARÉS, et non une seule chaîne :
                    // chacun est une étiquette à part, et l'interface
                    // les espace comme elle espace des mots voisins.
                    // Fondus en un seul `format!`, les points médians
                    // se collaient à leurs voisins — vu sur la capture.
                    Fragment::dit("·").faible(),
                    Fragment::dit(format!(
                        "{} {} {} {}",
                        self.windows.len(),
                        ctx.t("tranches de", "windows of"),
                        octets(self.window),
                        ctx.t("chacune", "each"),
                    ))
                    .faible(),
                    Fragment::dit("·").faible(),
                    Fragment::dit(format!(
                        "{} {}",
                        octets(self.size),
                        ctx.t("en tout", "in all")
                    ))
                    .faible(),
                ]
                .into(),
            ),
            // LE PROFIL : des valeurs, et rien d'autre. Comment on les
            // peint — une colonne par pixel, la plus haute de celles
            // qu'elle recouvre — appartient à qui connaît la largeur.
            Bloc::Dessin(Dessin::Profil {
                valeurs: self.windows.iter().map(|f| f.entropy).collect(),
                taille_tranche: self.window,
                total: self.size,
            }),
        ];

        // L'empaqueteur quand il se nomme : la mesure dit « il y a
        // quelque chose d'illisible ici », le nom dit quoi en faire.
        // Avant les plages, parce que c'est la réponse et non un détail
        // de plus.
        for empaqueteur in &self.packers {
            blocs.push(Bloc::Texte(
                vec![
                    Fragment::dit(empaqueteur.name).ton(Ton::Mauvais),
                    // La preuve avec le nom, toujours : « c'est de
                    // l'UPX » sans dire pourquoi demande de faire
                    // confiance.
                    Fragment::dit(format!(" {}", empaqueteur.clue.label(ctx.lang))).code(),
                ]
                .into(),
            ));
            blocs.push(Bloc::Retrait(vec![Bloc::Texte(
                Fragment::dit(empaqueteur.text(ctx.lang)).faible().into(),
            )]));
        }

        // LES PLAGES, avec leurs adresses. Le dessin dit « il se passe
        // quelque chose vers le tiers du fichier » ; cette table dit
        // où, en positions ET en adresses, et le clic y emmène.
        blocs.push(Bloc::Texte(
            Fragment::dit(format!(
                "{} ({})",
                ctx.t("Plages", "Ranges"),
                self.ranges.len()
            ))
            .fort()
            .into(),
        ));
        blocs.push(Bloc::Texte(
            Fragment::dit(ctx.t(
                "Les tranches voisines qui disent la même chose, réunies. L'adresse mène \
                 au listing ; la position, elle, se retrouve dans le vidage et dans \
                 l'éditeur binaire.",
                "Neighbouring windows that say the same thing, joined. The address leads \
                 to the listing; the file position is what the dump and the binary \
                 editor use.",
            ))
            .faible()
            .into(),
        ));
        let plus_dense = self
            .ranges
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.haute.total_cmp(&b.1.haute))
            .map(|(rang, _)| rang);
        let entête = |texte: &str| -> Ligne { Fragment::dit(texte).faible().into() };
        blocs.push(Bloc::Tableau {
            colonnes: vec![
                entête(ctx.t("position", "position")),
                entête(ctx.t("taille", "size")),
                entête(ctx.t("adresse", "address")),
                entête(ctx.t("entropie", "entropy")),
                entête(ctx.t("ce que c'est", "what it is")),
            ],
            rangées: self
                .ranges
                .iter()
                .enumerate()
                .take(PLAGES_MONTRÉES)
                .map(|(rang, plage)| {
                    // La mesure dit le VERDICT, pas la couleur : c'est
                    // la règle de P7, et c'est ce qui laisse un thème
                    // changer sans qu'on touche ici.
                    let verdict = teinte(plage.entropy());
                    vec![
                        Fragment::dit(format!("{:#x}..{:#x}", plage.offset, plage.end()))
                            .code()
                            .into(),
                        Fragment::dit(octets(plage.len)).code().into(),
                        match self.addresses.get(rang).copied().flatten() {
                            Some(addr) => {
                                Fragment::dit(format!("{addr:#010x}")).mène_à(addr).into()
                            }
                            // Une position que rien ne projette en
                            // mémoire : l'en-tête, la table des
                            // symboles, le remplissage de fin. Le dire
                            // plutôt que de laisser une case vide.
                            None => Fragment::dit(ctx.t("hors mémoire", "not mapped"))
                                .faible()
                                .into(),
                        },
                        Fragment::dit(format!("{:.2}", plage.entropy()))
                            .code()
                            .ton(verdict)
                            .into(),
                        // Le verdict en un mot : la phrase entière,
                        // dans une table à cinq colonnes, court au-delà
                        // du bord d'un panneau étroit. Elle est sous la
                        // légende, où elle a la place de s'expliquer.
                        if plus_dense == Some(rang) {
                            Fragment::dit(plage.verdict.short(fr)).ton(verdict).into()
                        } else {
                            Fragment::dit(plage.verdict.short(fr)).faible().into()
                        },
                    ]
                })
                .collect(),
            rayé: true,
        });
        if self.ranges.len() > PLAGES_MONTRÉES {
            blocs.push(Bloc::Texte(
                Fragment::dit(format!(
                    "… {} {}",
                    self.ranges.len() - PLAGES_MONTRÉES,
                    ctx.t("plages de plus", "more ranges")
                ))
                .faible()
                .into(),
            ));
        }

        // Ce que la mesure ne dit pas. Une entropie haute sur un petit
        // bloc n'est pas une preuve, et laisser croire le contraire
        // ferait chercher un empaqueteur là où il n'y en a pas.
        //
        // Le chiffre vient de `entropy::seuils` : la phrase, les
        // couleurs et le repère tracé en travers du profil parlent du
        // même réglage, et doivent donc le lire au même endroit.
        blocs.push(Bloc::Texte(
            Fragment::dit(if fr {
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
            })
            .faible()
            .into(),
        ));
        Some(Vue { blocs })
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let Some(vue) = self.vue(ctx) else {
            return;
        };
        let gestes = crate::vue_peinte::peindre_tout(ui, ctx, &vue);
        // Relevé pendant le dessin, posé après : la vue emprunte
        // `self`, et le module ne peut donc pas s'écrire dedans.
        if gestes.demande.is_some() {
            self.demande = gestes.demande;
        }
        for action in &gestes.actions {
            // Un clic dans le profil mène à l'adresse — quand cette
            // position du fichier est projetée quelque part. Le peintre
            // ne connaît que la position ; la table des adresses est
            // ici.
            let Some(reste) = action.strip_prefix(crate::vue_peinte::PROFIL_ALLER) else {
                continue;
            };
            let Some((position, cible)) = reste.split_once(':') else {
                continue;
            };
            let Ok(position) = position.parse::<usize>() else {
                continue;
            };
            let target = match cible {
                "octets" => crate::GoTarget::Bytes,
                _ => crate::GoTarget::Listing,
            };
            if let Some(addr) = self.adresse_de(position) {
                self.demande = Some(ReRequest::GoTo { addr, target });
            }
        }
    }
}

impl EntropyModule {
    /// L'adresse virtuelle d'une position du fichier, telle qu'on l'a
    /// relevée plage par plage au chargement.
    ///
    /// On ne relit pas le binaire ici : le panneau se dessine soixante
    /// fois par seconde, et relire les en-têtes ELF à chaque image pour
    /// une adresse qu'on connaît déjà serait du travail refait pour
    /// rien.
    fn adresse_de(&self, offset: usize) -> Option<u64> {
        let rang = self
            .ranges
            .iter()
            .rposition(|plage| plage.offset <= offset)?;
        let plage = &self.ranges[rang];
        let base = self.addresses.get(rang).copied().flatten()?;
        base.checked_add((offset - plage.offset) as u64)
    }
}

/// Une taille, écrite comme on la lit : en octets, en Ko, en Mo.
pub(crate) fn octets(n: usize) -> String {
    const KO: usize = 1024;
    const MO: usize = 1024 * KO;
    if n >= MO {
        format!("{:.1} Mo", n as f64 / MO as f64)
    } else if n >= KO {
        format!("{} Ko", n / KO)
    } else {
        format!("{n} o")
    }
}

/// Combien de repères d'adresse la règle porte, pour cette largeur.
///
/// Un repère tous les cent soixante pixels environ : assez pour situer
/// une bosse sans que les nombres se touchent — un décalage en
/// hexadécimal fait huit caractères.
pub(crate) fn repères(largeur: f32) -> usize {
    ((largeur / 160.0).floor() as usize).clamp(2, 10)
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
            patches: &[],
        }
    }

    /// Le dégradé va du vert au violet, en montant avec l'entropie.
    ///
    /// Le dégradé lui-même a suivi le dessin dans `vue_peinte` — c'est
    /// une décision de présentation, et le module ne dit plus que des
    /// valeurs. Le contrôle, lui, reste ici : ce qu'il éprouve est une
    /// propriété de l'ENTROPIE, pas du peintre.
    ///
    /// Les deux bouts sont les couleurs posées ; entre eux, la teinte
    /// change de façon monotone — le rouge monte sans discontinuité. C'est
    /// ce qui permet de lire la nature d'un bloc à sa couleur seule.
    #[test]
    fn the_gradient_climbs_from_green_to_purple() {
        // Les extrêmes sont exactement les arrêts annoncés.
        assert_eq!(
            crate::vue_peinte::entropy_color(0.0),
            egui::Color32::from_rgb(0x3f, 0xb9, 0x50)
        );
        assert_eq!(
            crate::vue_peinte::entropy_color(8.0),
            egui::Color32::from_rgb(0xa3, 0x71, 0xf7)
        );
        // Au-delà de l'échelle, on reste au violet, jamais rien de plus.
        assert_eq!(
            crate::vue_peinte::entropy_color(9.0),
            egui::Color32::from_rgb(0xa3, 0x71, 0xf7)
        );
        // Le bas est vert (vert dominant), le haut vire au violet (bleu
        // revenu). Entre les deux, le rouge croît sans jamais redescendre.
        let bas = crate::vue_peinte::entropy_color(1.0);
        assert!(
            bas.g() > bas.r() && bas.g() > bas.b(),
            "le bas doit être vert : {bas:?}"
        );
        let mut rouge_avant = 0u8;
        for pas in 0..=80 {
            let r = crate::vue_peinte::entropy_color(pas as f64 / 10.0).r();
            // De 0 à 7 (rouge au sommet), le rouge ne décroît pas.
            if pas <= 70 {
                assert!(
                    r >= rouge_avant,
                    "le rouge ne doit pas redescendre avant 7 (pas {pas})"
                );
                rouge_avant = r;
            }
        }
    }

    /// Le profil ne prend jamais plus de la moitié de la fenêtre.
    ///
    /// Un dessin qui occupe tout l'écran n'est plus un indice parmi
    /// d'autres : il devient le panneau, et les quatre rubriques qui le
    /// suivent — les empaqueteurs, les plages, leurs adresses, la
    /// phrase de prudence — passent sous le bord.
    #[test]
    fn the_profile_never_takes_more_than_half_the_window() {
        for fenêtre in [120.0f32, 300.0, 600.0, 820.0, 1100.0, 2160.0] {
            let hauteur = crate::vue_peinte::hauteur_du_profil(fenêtre);
            assert!(
                hauteur <= fenêtre * 0.5 + 0.01,
                "{fenêtre} px de haut : le profil en prend {hauteur}"
            );
            assert!(hauteur > 0.0, "et il reste visible : {hauteur}");
        }
        // Sur un grand écran, la hauteur est celle qu'on veut, et non
        // la moitié de l'écran : une bande de huit cents pixels de haut
        // ne se lit pas mieux qu'une de deux cent quarante.
        assert_eq!(
            crate::vue_peinte::hauteur_du_profil(2160.0),
            crate::vue_peinte::hauteur_du_profil(1080.0)
        );
    }

    /// L'échelle distingue ce qu'un demi-bit sépare.
    ///
    /// C'était le grief : « une échelle graduée en couleur beaucoup
    /// plus détaillée ». Avec cinq arrêts, tout l'intervalle du code
    /// ordinaire tenait dans une teinte, et deux blocs qu'un demi-bit
    /// sépare — c'est-à-dire du code dense et un bloc chiffré —
    /// paraissaient identiques.
    #[test]
    fn the_scale_tells_apart_what_half_a_bit_separates() {
        let écart = |a: f64, b: f64| -> i32 {
            let (x, y) = (
                crate::vue_peinte::entropy_color(a),
                crate::vue_peinte::entropy_color(b),
            );
            (x.r() as i32 - y.r() as i32).abs()
                + (x.g() as i32 - y.g() as i32).abs()
                + (x.b() as i32 - y.b() as i32).abs()
        };
        let mut pas = 0.0f64;
        while pas < 8.0 {
            assert!(
                écart(pas, pas + 0.5) >= 12,
                "{pas} et {} doivent se distinguer : {} de différence",
                pas + 0.5,
                écart(pas, pas + 0.5)
            );
            pas += 0.5;
        }
        // Et là où tout se joue — entre du code dense et un bloc
        // chiffré —, la différence saute aux yeux.
        assert!(écart(6.0, 7.0) >= 60, "{}", écart(6.0, 7.0));
    }

    /// Le panneau écrit ses plages, avec leurs positions.
    ///
    /// Le dessin dit « il se passe quelque chose vers le tiers du
    /// fichier » ; sans les plages, il fallait relever le décalage au
    /// survol, puis le convertir de tête pour le retrouver ailleurs.
    #[test]
    fn the_panel_writes_its_ranges_and_where_they_are() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        // Du calme, puis du bruit : deux plages, et la seconde est
        // celle qu'on cherche.
        let mut binaire = vec![0u8; 8192];
        let mut graine = 0x2545f4914f6cdd1du64;
        for _ in 0..8192 {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            binaire.push((graine >> 24) as u8);
        }

        let mut module = EntropyModule::default();
        let ctx = contexte(&binaire, &cpu);
        module.on_binary_loaded(&ctx);
        assert_eq!(module.window, entropy::MIN_WINDOW);
        assert_eq!(module.windows.len(), 64, "{} octets", binaire.len());
        assert!(module.ranges.len() >= 2, "{:?}", module.ranges);

        let egui_ctx = egui::Context::default();
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
        // Le découpage est annoncé : une bosse sur 256 octets et une
        // bosse sur 64 Ko ne disent pas la même chose.
        assert!(tout.contains("256 o"), "la taille des tranches : {tout}");
        // La plage de bruit est nommée par sa position dans le fichier.
        assert!(
            tout.contains("0x2000..0x4000"),
            "la plage dense et ses bornes : {tout}"
        );
        // Rien ne projette ces octets en mémoire — ce n'est pas un ELF —
        // et le panneau le dit plutôt que de laisser une case vide.
        assert!(tout.contains("hors mémoire"), "{tout}");
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
    /// LA TAILLE DU FICHIER EST DITE, ET NON DÉDUITE DES TRANCHES.
    ///
    /// C'est le défaut que la capture a trouvé : la dernière tranche
    /// est plus COURTE que les autres, et `tranches × taille_tranche`
    /// place donc la fin du fichier au-delà de sa fin réelle. La règle
    /// d'adresses sous le profil se graduait jusqu'à `0x2300` sur un
    /// fichier qui s'arrête à `0x22c8` — cinquante-six octets qui
    /// n'existent pas, et un clic au bout menait hors du fichier.
    ///
    /// Le contrôle choisit exprès une taille qui n'est pas un multiple
    /// de la tranche : c'est le seul cas où la déduction se trompe, et
    /// c'est le cas ordinaire.
    #[test]
    fn the_profile_says_the_file_size_instead_of_deducing_it() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        // 0x22c8 octets : la taille vue sur la capture, et surtout un
        // nombre que la tranche ne divise pas.
        let binaire = vec![0x41u8; 0x22c8];
        let mut module = EntropyModule::default();
        let ctx = contexte(&binaire, &cpu);
        module.on_binary_loaded(&ctx);
        let vue = module.vue(&ctx).expect("le module rend une vue");
        let Some(Dessin::Profil {
            valeurs,
            taille_tranche,
            total,
        }) = vue.blocs.iter().find_map(|bloc| match bloc {
            Bloc::Dessin(dessin) => Some(dessin.clone()),
            _ => None,
        })
        else {
            panic!("le profil est dans la vue : {:?}", vue.blocs);
        };
        assert_eq!(total, binaire.len(), "la taille mesurée, et pas une autre");
        // Et la déduction se trompe VRAIMENT ici : sans quoi le
        // contrôle passerait en ne regardant rien.
        let déduite = valeurs.len() * taille_tranche;
        assert!(
            déduite > total,
            "la dernière tranche doit être plus courte, sinon ce contrôle \
             ne distingue pas les deux façons de compter : {déduite} vs {total}"
        );
    }

    /// LES POINTS MÉDIANS DE LA LIGNE DE RÉSUMÉ SONT DES FRAGMENTS À
    /// PART.
    ///
    /// Second écart vu sur la capture : fondus dans un `format!` avec
    /// le texte qui les suit, ils se collaient à leurs voisins — c'est
    /// l'interface qui espace des étiquettes voisines, et elle ne peut
    /// pas le faire à l'intérieur d'une chaîne.
    #[test]
    fn the_middots_of_the_summary_are_fragments_of_their_own() {
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let binaire = vec![0x41u8; 0x2000];
        let mut module = EntropyModule::default();
        let ctx = contexte(&binaire, &cpu);
        module.on_binary_loaded(&ctx);
        let vue = module.vue(&ctx).expect("le module rend une vue");
        let Some(Bloc::Texte(ligne)) = vue.blocs.first() else {
            panic!("la première ligne est le résumé : {:?}", vue.blocs);
        };
        let collés: Vec<&str> = ligne
            .fragments
            .iter()
            .map(|f| f.texte.as_str())
            .filter(|t| t.contains('\u{b7}') && t.trim() != "\u{b7}")
            .collect();
        assert!(
            collés.is_empty(),
            "un point médian collé à son voisin : {collés:?}"
        );
        assert_eq!(
            ligne
                .fragments
                .iter()
                .filter(|f| f.texte.trim() == "\u{b7}")
                .count(),
            2,
            "deux séparateurs, un par étiquette ajoutée"
        );
    }
}
