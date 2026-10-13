use crate::entropy::{self, Range, Verdict, Window, seuils};
use crate::{ReContext, ReModule, ReRequest, clickable_address};

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

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        if self.windows.is_empty() {
            ui.weak(ctx.t(
                "Rien à mesurer : aucun fichier chargé.",
                "Nothing to measure: no file loaded.",
            ));
            return;
        }

        let fr = ctx.lang.starts_with("fr");
        // Ce qu'on lit d'abord : le chiffre et son verdict. Puis le
        // découpage, parce qu'un profil ne veut rien dire sans la
        // taille de ses tranches — une bosse sur 256 octets et une
        // bosse sur 64 kilo-octets ne disent pas la même chose.
        ui.horizontal_wrapped(|ui| {
            ui.label(ctx.t("Sur tout le fichier", "Whole file"));
            ui.monospace(format!("{:.2}", self.whole));
            ui.weak(Verdict::of(self.whole).label(fr));
            ui.weak("·");
            ui.weak(format!(
                "{} {} {} {}",
                self.windows.len(),
                ctx.t("tranches de", "windows of"),
                octets(self.window),
                ctx.t("chacune", "each"),
            ));
            ui.weak("·");
            ui.weak(format!(
                "{} {}",
                octets(self.size),
                ctx.t("en tout", "in all")
            ));
        });

        ui.add_space(6.0);
        if let Some(offset) = profil(ui, ctx, &self.windows, self.window) {
            // Un clic dans le profil mène à l'adresse — quand cette
            // position du fichier est projetée quelque part.
            if let Some(addr) = self.adresse_de(offset) {
                self.demande = Some(ReRequest::GoTo { addr });
            }
        }
        ui.add_space(8.0);
        échelle(ui, ctx);

        // L'empaqueteur quand il se nomme : la mesure dit « il y a
        // quelque chose d'illisible ici », le nom dit quoi en faire.
        // Avant les plages, parce que c'est la réponse et non un détail
        // de plus.
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

        ui.add_space(8.0);
        // LES PLAGES, avec leurs adresses. Le dessin dit « il se passe
        // quelque chose vers le tiers du fichier » ; cette table dit
        // où, en positions ET en adresses, et le clic y emmène. Sans
        // elle, il fallait relever le décalage au survol puis le
        // convertir de tête.
        ui.strong(format!(
            "{} ({})",
            ctx.t("Plages", "Ranges"),
            self.ranges.len()
        ));
        ui.weak(ctx.t(
            "Les tranches voisines qui disent la même chose, réunies. L'adresse mène \
             au listing ; la position, elle, se retrouve dans le vidage et dans \
             l'éditeur binaire.",
            "Neighbouring windows that say the same thing, joined. The address leads \
             to the listing; the file position is what the dump and the binary \
             editor use.",
        ));
        ui.add_space(4.0);
        let mut demande: Option<ReRequest> = None;
        let plus_dense = self
            .ranges
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.haute.total_cmp(&b.1.haute))
            .map(|(rang, _)| rang);
        egui::Grid::new("entropy_ranges")
            .num_columns(5)
            .min_col_width(0.0)
            .striped(true)
            .show(ui, |ui| {
                ui.weak(ctx.t("position", "position"));
                ui.weak(ctx.t("taille", "size"));
                ui.weak(ctx.t("adresse", "address"));
                ui.weak(ctx.t("entropie", "entropy"));
                ui.weak(ctx.t("ce que c'est", "what it is"));
                ui.end_row();
                for (rang, plage) in self.ranges.iter().enumerate().take(PLAGES_MONTRÉES) {
                    ui.monospace(format!("{:#x}..{:#x}", plage.offset, plage.end()));
                    ui.monospace(octets(plage.len));
                    match self.addresses.get(rang).copied().flatten() {
                        Some(addr) => {
                            if clickable_address(ui, ctx, addr) {
                                demande = Some(ReRequest::GoTo { addr });
                            }
                        }
                        // Une position que rien ne projette en mémoire :
                        // l'en-tête, la table des symboles, le
                        // remplissage de fin. Le dire plutôt que de
                        // laisser une case vide.
                        None => {
                            ui.weak(ctx.t("hors mémoire", "not mapped"));
                        }
                    }
                    let couleur = entropy_color(plage.entropy());
                    ui.monospace(
                        egui::RichText::new(format!("{:.2}", plage.entropy())).color(couleur),
                    );
                    // Le verdict en un mot : la phrase entière, dans
                    // une table à cinq colonnes, court au-delà du bord
                    // d'un panneau étroit. Elle est sous la légende,
                    // où elle a la place de s'expliquer.
                    let mot = plage.verdict.short(fr);
                    if plus_dense == Some(rang) {
                        ui.strong(egui::RichText::new(mot).color(couleur));
                    } else {
                        ui.weak(mot);
                    }
                    ui.end_row();
                }
            });
        if self.ranges.len() > PLAGES_MONTRÉES {
            ui.weak(format!(
                "… {} {}",
                self.ranges.len() - PLAGES_MONTRÉES,
                ctx.t("plages de plus", "more ranges")
            ));
        }
        if demande.is_some() {
            self.demande = demande;
        }

        // Ce que la mesure ne dit pas. Une entropie haute sur un petit
        // bloc n'est pas une preuve, et laisser croire le contraire
        // ferait chercher un empaqueteur là où il n'y en a pas.
        ui.add_space(6.0);
        // Le chiffre vient de `entropy::seuils` : la phrase, les
        // couleurs et le repère tracé en travers du profil parlent du
        // même réglage, et doivent donc le lire au même endroit.
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
fn octets(n: usize) -> String {
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

/// La couleur d'une tranche selon son entropie, en dégradé continu.
///
/// Du vert (régulier, inoffensif) au violet (maximal — chiffré ou
/// compressé), en passant par le vert-jaune, le jaune, l'orangé,
/// l'orange, le rouge et le magenta. **Neuf arrêts et non cinq** : avec
/// cinq, tout l'intervalle du code ordinaire — 5,5 à 6,5, celui où se
/// joue la lecture d'un vrai binaire — tenait dans une seule teinte, et
/// deux blocs qu'un demi-bit sépare paraissaient identiques.
///
/// Les arrêts sont posés là où le sens bascule : le texte vers 4-5, le
/// code dense vers 6, le soupçon d'empaquetage vers 7, l'aléa pur à 8.
///
/// Couleurs fixes, et non tirées du thème : c'est une échelle de mesure,
/// elle doit dire la même chose d'un thème à l'autre.
fn entropy_color(entropy: f64) -> egui::Color32 {
    const ARRÊTS: [(f32, egui::Color32); 9] = [
        (0.0, egui::Color32::from_rgb(0x3f, 0xb9, 0x50)), // vert
        (3.0, egui::Color32::from_rgb(0x8b, 0xc3, 0x4a)), // vert-jaune
        (4.0, egui::Color32::from_rgb(0xd2, 0xa8, 0x06)), // jaune
        (5.0, egui::Color32::from_rgb(0xdd, 0x8a, 0x0c)), // jaune orangé
        (6.0, egui::Color32::from_rgb(0xe0, 0x7b, 0x1f)), // orange
        (6.5, egui::Color32::from_rgb(0xec, 0x63, 0x34)), // orange rouge
        (7.0, egui::Color32::from_rgb(0xf8, 0x51, 0x49)), // rouge
        (7.5, egui::Color32::from_rgb(0xd5, 0x5a, 0xb0)), // magenta
        (8.0, egui::Color32::from_rgb(0xa3, 0x71, 0xf7)), // violet
    ];
    let e = entropy as f32;
    for paire in ARRÊTS.windows(2) {
        let (v0, c0) = paire[0];
        let (v1, c1) = paire[1];
        if e <= v1 {
            let t = ((e - v0) / (v1 - v0)).clamp(0.0, 1.0);
            let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
            return egui::Color32::from_rgb(
                m(c0.r(), c1.r()),
                m(c0.g(), c1.g()),
                m(c0.b(), c1.b()),
            );
        }
    }
    ARRÊTS[ARRÊTS.len() - 1].1
}

/// La hauteur du profil : ample, mais jamais plus que la moitié de la
/// fenêtre.
///
/// Trente-six pixels de haut sur cinq cents de large écrasaient tout :
/// entre 5,8 et 6,6 — l'écart qui sépare du code d'un bloc suspect —,
/// il y avait trois pixels, et l'on ne voyait rien. Il faut de la
/// hauteur pour lire une courbe.
///
/// La borne est celle que l'usage a demandée : **jamais plus de la
/// moitié du viewport**. Un dessin qui prend tout l'écran n'est plus un
/// indice parmi d'autres, c'est le panneau tout entier — et il reste
/// quatre rubriques à lire dessous.
pub(crate) fn hauteur_du_profil(viewport: f32) -> f32 {
    const VOULUE: f32 = 240.0;
    const MINIMUM: f32 = 60.0;
    VOULUE.min(viewport * 0.5).max(MINIMUM.min(viewport * 0.5))
}

/// Combien de repères d'adresse la règle porte, pour cette largeur.
///
/// Un repère tous les cent soixante pixels environ : assez pour situer
/// une bosse sans que les nombres se touchent — un décalage en
/// hexadécimal fait huit caractères.
pub(crate) fn repères(largeur: f32) -> usize {
    ((largeur / 160.0).floor() as usize).clamp(2, 10)
}

/// Le profil : une colonne par tranche, du bas vers le haut, l'échelle
/// allant de 0 à 8 — **toujours**, et non de la plus basse à la plus
/// haute valeur vue : une échelle qui s'ajuste ferait paraître
/// spectaculaire l'écart entre 5,9 et 6,1, et l'oeil y verrait une
/// bosse qui n'existe pas.
///
/// Rend la position du fichier sur laquelle on a cliqué, s'il y en a
/// une : le dessin désigne des endroits, et la seule chose qu'on veuille
/// en faire est d'aller voir.
fn profil(
    ui: &mut egui::Ui,
    ctx: &ReContext,
    windows: &[Window],
    taille_tranche: usize,
) -> Option<usize> {
    let hauteur = hauteur_du_profil(ui.ctx().viewport_rect().height());
    // Pleine largeur : c'est une carte du fichier, et une carte se lit
    // d'autant mieux qu'elle est large. La borne de 520 pixels d'avant
    // tassait trois cents tranches dans un timbre-poste.
    let largeur = ui.available_width().max(120.0);
    let (rect, réponse) = ui.allocate_exact_size(
        egui::vec2(largeur, hauteur),
        egui::Sense::click().union(egui::Sense::hover()),
    );
    let peintre = ui.painter_at(rect);
    let visuels = ui.visuals().clone();
    peintre.rect_filled(rect, 2.0, visuels.extreme_bg_color);

    let y_de = |bits: f64| rect.bottom() - (bits / 8.0).clamp(0.0, 1.0) as f32 * hauteur;

    // Les lignes de niveau, graduées en bits par octet : sans elles,
    // les hauteurs ne se comparent qu'entre elles, et l'on ne sait pas
    // si une barre est haute dans l'absolu.
    let police = egui::TextStyle::Small.resolve(ui.style());
    for bits in [2.0, 4.0, 6.0] {
        let y = y_de(bits);
        peintre.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, visuels.weak_text_color().gamma_multiply(0.35)),
        );
    }

    // Une colonne par pixel, et la PLUS HAUTE des tranches qu'elle
    // recouvre : sur un fichier qui porte plus de tranches que le
    // dessin n'a de pixels, prendre la moyenne effacerait exactement ce
    // qu'on cherche — le petit bloc dense au milieu du calme.
    let colonnes = largeur.floor().max(1.0) as usize;
    let n = windows.len();
    for colonne in 0..colonnes {
        let début = colonne * n / colonnes;
        let fin = (((colonne + 1) * n).div_ceil(colonnes))
            .min(n)
            .max(début + 1);
        let Some(haute) = windows.get(début..fin).and_then(|tranche| {
            tranche
                .iter()
                .map(|f| f.entropy)
                .fold(None, |acc: Option<f64>, e| {
                    Some(acc.map_or(e, |a: f64| a.max(e)))
                })
        }) else {
            continue;
        };
        let x = rect.left() + colonne as f32;
        let barre = egui::Rect::from_min_max(
            egui::pos2(x, y_de(haute)),
            egui::pos2(x + 1.0, rect.bottom()),
        );
        // La couleur suit l'entropie en dégradé continu : on lit la
        // montée sans avoir à comparer des hauteurs entre elles, et
        // l'extrême — le chiffré, le compressé — se détache en violet.
        peintre.rect_filled(barre, 0.0, entropy_color(haute));
    }

    // Les graduations APRÈS les barres : peintes avant, le « 2 » et le
    // « 4 » disparaissaient sous la courbe dès que celle-ci montait, et
    // l'échelle ne se lisait plus qu'à moitié.
    for bits in [2.0, 4.0, 6.0] {
        let y = y_de(bits);
        let chiffre = format!("{bits:.0}");
        let place =
            peintre.layout_no_wrap(chiffre.clone(), police.clone(), visuels.weak_text_color());
        let fond = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, y - place.size().y - 1.0),
            place.size() + egui::vec2(4.0, 2.0),
        );
        peintre.rect_filled(fond, 2.0, visuels.extreme_bg_color.gamma_multiply(0.8));
        peintre.text(
            egui::pos2(rect.left() + 4.0, y - 1.0),
            egui::Align2::LEFT_BOTTOM,
            chiffre,
            police.clone(),
            visuels.weak_text_color(),
        );
    }

    // Le repère d'empaquetage : la ligne au-dessus de laquelle il n'y a
    // plus de structure à voir. La valeur vient de `entropy::seuils`,
    // comme celle qui décide des couleurs.
    let seuil = y_de(seuils::EMPAQUETÉ);
    peintre.line_segment(
        [
            egui::pos2(rect.left(), seuil),
            egui::pos2(rect.right(), seuil),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(0xf8, 0x51, 0x49)),
    );
    peintre.text(
        egui::pos2(rect.right() - 4.0, seuil - 2.0),
        egui::Align2::RIGHT_BOTTOM,
        format!(
            "{} {}",
            format!("{:.1}", seuils::EMPAQUETÉ).replace('.', ","),
            ctx.t("empaqueté", "packed")
        ),
        police.clone(),
        egui::Color32::from_rgb(0xf8, 0x51, 0x49),
    );

    // LA RÈGLE DES ADRESSES, sous le dessin : sans elle, une bosse « au
    // tiers du fichier » ne se retrouve nulle part. Les positions sont
    // celles du FICHIER — c'est lui qu'on mesure —, et la table des
    // plages donne l'adresse en mémoire en regard.
    let (règle, _) = ui.allocate_exact_size(egui::vec2(largeur, 16.0), egui::Sense::hover());
    let peintre = ui.painter_at(règle.expand(2.0));
    let total = windows.last().map_or(0, |f| f.offset + f.len);
    let combien = repères(largeur);
    for repère in 0..=combien {
        let part = repère as f32 / combien as f32;
        let x = rect.left() + part * largeur;
        peintre.line_segment(
            [egui::pos2(x, règle.top()), egui::pos2(x, règle.top() + 4.0)],
            egui::Stroke::new(1.0, visuels.weak_text_color()),
        );
        let position = (total as f64 * part as f64) as usize;
        let ancrage = if repère == 0 {
            egui::Align2::LEFT_TOP
        } else if repère == combien {
            egui::Align2::RIGHT_TOP
        } else {
            egui::Align2::CENTER_TOP
        };
        peintre.text(
            egui::pos2(x, règle.top() + 5.0),
            ancrage,
            format!("{position:#x}"),
            police.clone(),
            visuels.weak_text_color(),
        );
    }

    // Au survol, la tranche visée se dit : sa plage, sa mesure, son
    // verdict. Et ce qu'un clic ferait — personne ne clique sur ce qui
    // n'a pas l'air cliquable.
    let mut cliqué = None;
    if let Some(position) = réponse.hover_pos() {
        let part = ((position.x - rect.left()) / largeur).clamp(0.0, 0.999_9);
        let index = ((part * n as f32) as usize).min(n.saturating_sub(1));
        if let Some(fenêtre) = windows.get(index) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            réponse.clone().on_hover_text(format!(
                "{:#x}..{:#x}  ({})\n{:.2} — {}\n{}",
                fenêtre.offset,
                fenêtre.offset + fenêtre.len,
                octets(taille_tranche),
                fenêtre.entropy,
                Verdict::of(fenêtre.entropy).label(ctx.lang.starts_with("fr")),
                ctx.t(
                    "Cliquer : aller à cette adresse dans le listing",
                    "Click: go to this address in the listing"
                ),
            ));
            if réponse.clicked() {
                cliqué = Some(fenêtre.offset);
            }
        }
    }
    cliqué
}

/// L'échelle de couleurs, graduée.
///
/// Une rampe sans graduation ne se lit pas : on voit « du rouge » sans
/// savoir si c'est 6,9 ou 7,8 — or c'est exactement ce qui sépare du
/// code dense d'un bloc chiffré. La barre porte donc ses huit
/// graduations, et les cinq verdicts sont écrits sous la zone qui leur
/// correspond.
fn échelle(ui: &mut egui::Ui, ctx: &ReContext) {
    let fr = ctx.lang.starts_with("fr");
    let largeur = ui.available_width().clamp(160.0, 560.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(largeur, 12.0), egui::Sense::hover());
    let peintre = ui.painter_at(rect);
    // Un pixel par colonne : la rampe est continue, et c'est ce qu'elle
    // doit montrer.
    let colonnes = largeur.floor().max(1.0) as usize;
    for colonne in 0..colonnes {
        let bits = 8.0 * colonne as f64 / colonnes as f64;
        let x = rect.left() + colonne as f32;
        peintre.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x, rect.top()),
                egui::pos2(x + 1.0, rect.bottom()),
            ),
            0.0,
            entropy_color(bits),
        );
    }
    // Les graduations : zéro à huit bits par octet.
    let police = egui::TextStyle::Small.resolve(ui.style());
    let (règle, _) = ui.allocate_exact_size(egui::vec2(largeur, 14.0), egui::Sense::hover());
    let peintre = ui.painter_at(règle.expand(2.0));
    let couleur = ui.visuals().weak_text_color();
    for bits in 0..=8 {
        let part = bits as f32 / 8.0;
        let x = rect.left() + part * largeur;
        peintre.line_segment(
            [egui::pos2(x, règle.top()), egui::pos2(x, règle.top() + 3.0)],
            egui::Stroke::new(1.0, couleur),
        );
        let ancrage = match bits {
            0 => egui::Align2::LEFT_TOP,
            8 => egui::Align2::RIGHT_TOP,
            _ => egui::Align2::CENTER_TOP,
        };
        peintre.text(
            egui::pos2(x, règle.top() + 4.0),
            ancrage,
            format!("{bits}"),
            police.clone(),
            couleur,
        );
    }
    // Et ce que chaque zone veut dire, dans sa couleur.
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.weak(ctx.t("bits par octet :", "bits per byte:"));
        for (milieu, verdict) in [
            (0.75, Verdict::Flat),
            (3.25, Verdict::Text),
            (5.9, Verdict::Code),
            (7.0, Verdict::Mixed),
            (7.9, Verdict::Random),
        ] {
            ui.label(egui::RichText::new(verdict.label(fr)).color(entropy_color(milieu)));
            ui.weak("·");
        }
    });
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

    /// Le dégradé va du vert au violet, en montant avec l'entropie.
    ///
    /// Les deux bouts sont les couleurs posées ; entre eux, la teinte
    /// change de façon monotone — le rouge monte sans discontinuité. C'est
    /// ce qui permet de lire la nature d'un bloc à sa couleur seule.
    #[test]
    fn the_gradient_climbs_from_green_to_purple() {
        // Les extrêmes sont exactement les arrêts annoncés.
        assert_eq!(
            entropy_color(0.0),
            egui::Color32::from_rgb(0x3f, 0xb9, 0x50)
        );
        assert_eq!(
            entropy_color(8.0),
            egui::Color32::from_rgb(0xa3, 0x71, 0xf7)
        );
        // Au-delà de l'échelle, on reste au violet, jamais rien de plus.
        assert_eq!(
            entropy_color(9.0),
            egui::Color32::from_rgb(0xa3, 0x71, 0xf7)
        );
        // Le bas est vert (vert dominant), le haut vire au violet (bleu
        // revenu). Entre les deux, le rouge croît sans jamais redescendre.
        let bas = entropy_color(1.0);
        assert!(
            bas.g() > bas.r() && bas.g() > bas.b(),
            "le bas doit être vert : {bas:?}"
        );
        let mut rouge_avant = 0u8;
        for pas in 0..=80 {
            let r = entropy_color(pas as f64 / 10.0).r();
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
            let hauteur = hauteur_du_profil(fenêtre);
            assert!(
                hauteur <= fenêtre * 0.5 + 0.01,
                "{fenêtre} px de haut : le profil en prend {hauteur}"
            );
            assert!(hauteur > 0.0, "et il reste visible : {hauteur}");
        }
        // Sur un grand écran, la hauteur est celle qu'on veut, et non
        // la moitié de l'écran : une bande de huit cents pixels de haut
        // ne se lit pas mieux qu'une de deux cent quarante.
        assert_eq!(hauteur_du_profil(2160.0), hauteur_du_profil(1080.0));
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
            let (x, y) = (entropy_color(a), entropy_color(b));
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
}
