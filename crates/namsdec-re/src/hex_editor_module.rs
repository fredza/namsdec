use crate::vue::{Bloc, Fragment, Vue};
use crate::{ReContext, ReModule, ReRequest, ReResult, ReSection, framed_button};

/// L'action qui change l'ordre de lecture de l'inspecteur : « le » ou
/// « be » suit.
const BOUTISME: &str = "hex:boutisme:";

/// Un éditeur binaire : le fichier chargé en grille hexadécimale, un octet
/// qu'on choisit, une valeur qu'on pose à la place.
///
/// C'est la suite naturelle de l'analyse : on a compris qu'un octet décide
/// d'un saut, d'une longueur, d'une constante, et l'on veut le voir
/// autrement — remplacer `75` (jne) par `74` (je), pour lire ce que le
/// programme ferait alors. Un geste qu'on apprend en le faisant, et qui
/// **se défait** : chaque écriture entre dans la liste des correctifs
/// (Outils › Correctifs), avec l'octet d'avant, prête à revenir en
/// arrière. Rien n'est écrit sur le disque tant qu'on ne l'enregistre pas
/// soi-même, vers un fichier qu'on choisit — jamais par-dessus l'original.
///
/// Première version : on navigue, on choisit un octet, on en pose un
/// autre, et ce qu'on a touché se repère au coup d'œil. L'édition de
/// plusieurs octets d'un coup et l'insertion viendront ensuite — insérer
/// décale tout ce qui suit, et chaque adresse du fichier deviendrait
/// fausse, donc elle demande sa propre prudence.
/// La colonne où l'on édite : les octets en hexadécimal, ou leur texte.
/// Comme dans 010 Editor, le curseur vit dans l'une ou l'autre, et la
/// frappe s'interprète selon celle qui est active — des chiffres hexa à
/// gauche, un caractère à droite.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Colonne {
    #[default]
    Hex,
    Texte,
}

#[derive(Default)]
pub struct HexEditorModule {
    /// L'octet qu'il faut AMENER SOUS LES YEUX à la prochaine image.
    ///
    /// Remplace la pagination. L'éditeur montrait le fichier par pages
    /// de 256 octets, avec « ‹ » et « › » pour tourner — ET une zone
    /// défilante par-dessus, parce que seize lignes ne tenaient pas
    /// dans la hauteur accordée. Deux façons de se déplacer dans la
    /// même chose, dont l'une s'arrête tous les 256 octets sans raison
    /// : un fichier n'a pas de pages. Il défile maintenant d'un bout à
    /// l'autre, et ce champ ne sert qu'aux sauts — « aller à », une
    /// trouvaille, une arrivée depuis un autre panneau, le curseur qui
    /// sort de l'écran.
    amener: Option<usize>,
    /// Les sections du fichier, rangées par position, relevées une fois
    /// au chargement.
    ///
    /// Dire `.text` plutôt que « 0x2050 » est ce qui change une adresse
    /// en lieu. Relevé une fois : relire les en-têtes à chaque image
    /// pour une ligne d'information coûterait plus que tout le reste du
    /// panneau.
    sections: Vec<(String, std::ops::Range<usize>, bool, bool)>,
    /// De combien le vidage est décalé horizontalement.
    ///
    /// La règle des colonnes vit HORS de la zone qui défile — elle doit
    /// rester en vue quand on descend — et doit pourtant suivre son
    /// défilement de côté, sans quoi elle désignerait la mauvaise
    /// colonne. Relevé après coup, employé à l'image suivante.
    décalage_x: f32,
    /// Combien d'octets par ligne à la dernière image.
    ///
    /// Calculé sur la largeur disponible, et retenu : le clavier s'en
    /// sert — flèche bas, page suivante — et il s'exécute APRÈS la
    /// grille, qui vient de le mesurer.
    par_ligne: usize,
    /// Le nombre d'octets par ligne suit-il la largeur offerte ?
    ///
    /// Vrai au départ : c'est ce qu'on attend d'une grille, et c'est ce
    /// qui emploie la place. Cliquer 8, 16 ou 32 le fige — on veut
    /// parfois comparer deux fichiers sur la même grille, ou retrouver
    /// la disposition de `xxd`.
    largeur_auto: bool,
    /// Ce qu'on a tapé dans « aller à ».
    aller: String,
    /// L'octet choisi, s'il y en a un. C'est le CURSEUR.
    choisi: Option<usize>,
    /// L'autre bout de la sélection, quand on en a tendu une.
    ///
    /// `None` : un seul octet, celui du curseur. Sinon la sélection va
    /// de l'un à l'autre, bornes comprises, et c'est elle que
    /// « Écrire » remplace et que « Remplir » recouvre.
    ///
    /// Un bout FIXE et un curseur qui bouge, et non deux bornes
    /// rangées : c'est ce qui permet d'étendre dans les deux sens avec
    /// Maj+flèches sans que la sélection se retourne sous les doigts.
    ancre: Option<usize>,
    /// La valeur qu'on pose à la place, en hexadécimal.
    valeur: String,
    /// Les octets qu'on a modifiés depuis l'ouverture de ce binaire, pour
    /// les montrer autrement. Remis à zéro au chargement d'un fichier.
    touchés: std::collections::BTreeSet<usize>,
    /// Ce qu'on cherche, et comment : en octets hexadécimaux, ou en texte.
    recherche: String,
    recherche_hex: bool,
    /// La frappe en place : le demi-octet de poids fort déjà tapé, en
    /// attente du second. `None` quand on n'a rien commencé.
    nibble: Option<u8>,
    /// La colonne active : où le curseur se trouve, et comment la frappe
    /// s'interprète.
    colonne: Colonne,
    /// L'inspecteur lit-il en gros-boutien ? Petit-boutien (x86) par
    /// défaut ; la bascule sert aux formats réseau et aux fichiers
    /// gros-boutiens.
    inspecteur_be: bool,
    /// La dernière trouvaille, surlignée dans la grille.
    trouvé: Option<std::ops::Range<usize>>,
    /// L'adresse d'où l'on arrive, quand on est venu d'un autre
    /// panneau. Écrite en tête : on vient de cliquer dessus, et la
    /// retrouver ici est ce qui dit qu'on est au bon endroit.
    venu_de: Option<u64>,
    /// Montrer l'adresse en mémoire à côté de la position du fichier.
    ///
    /// Les deux ne se valent pas : la position dit où l'octet est
    /// écrit, l'adresse où il sera lu. On patche à la première et l'on
    /// raisonne avec la seconde, et c'est pour avoir à les rapprocher
    /// de tête qu'on se trompe de ligne.
    ///
    /// Fausse au départ et réglée au chargement du fichier
    /// (`adresses_utiles`) : sans fichier, il n'y a pas d'adresse à
    /// montrer.
    adresses: bool,
    demande: Option<ReRequest>,
}

/// Les largeurs de ligne acceptées, en octets.
///
/// DES PUISSANCES DE DEUX, et c'est tout l'intérêt : chaque ligne
/// commence alors à une adresse ronde, et la règle des colonnes donne
/// directement le dernier chiffre de la position — `0x2050` se lit
/// « ligne 2050, colonne 0 ». À vingt-quatre octets par ligne on
/// remplirait mieux la fenêtre et l'on ne saurait plus lire une
/// adresse.
///
/// Seize est la forme de `xxd` et de tous les éditeurs hexadécimaux ;
/// trente-deux sert quand le panneau est large, et c'est là qu'un motif
/// qui se répète toutes les seize ou trente-deux positions se voit d'un
/// coup d'œil. Huit est pour les panneaux étroits, où seize se
/// couperait.
const LARGEURS: [usize; 3] = [8, 16, 32];

/// La largeur par défaut, et celle dont le clavier se sert tant que la
/// grille n'a pas encore été peinte.
const PAR_LIGNE: usize = 16;

/// Les octets se groupent par huit : l'œil retrouve la huitième colonne
/// sans la compter, et un mot de huit octets se lit d'un bloc.
const GROUPE: usize = 8;

/// La largeur d'une cellule d'octet, en caractères : deux chiffres et
/// ce qui les sépare du voisin.
///
/// SIX DIXIÈMES D'ESPACE, et non un entier. Mesuré : à un espace plein,
/// seize octets par ligne avec leurs deux colonnes d'adresses font 642
/// pixels dans un panneau qui en offre 622 — le vidage se dotait d'une
/// barre horizontale pour vingt pixels, sur la disposition PAR DÉFAUT.
/// Un appareil dont la vue ordinaire ne tient pas dans sa place n'a pas
/// la bonne taille ; les octets se resserrent, et le cran entre deux
/// groupes de huit suffit à ce que l'œil retrouve sa colonne.
const PAS_OCTET: f32 = 2.6;

/// Le moins de lignes du vidage qu'on accepte de montrer.
///
/// UN PLANCHER, et non une hauteur. Le vidage prend la place qui
/// reste ; ce nombre dit seulement à partir de quand il vaut mieux
/// déborder que montrer trois lignes. Huit, c'est de quoi voir un
/// motif se répéter.
///
/// Il y avait ici un nombre de lignes FIXE — vingt-deux — et la raison
/// écrite était la bonne crainte : lire la place qu'on occupe, dans
/// une fenêtre qui épouse son contenu, se reboucle. Mesuré le
/// 28 octobre 2026, ce n'est pas ce qui se passe : la zone qui porte
/// ce panneau est une `ScrollArea` à `auto_shrink` faux, donc elle
/// IMPOSE sa hauteur au lieu de l'épouser, et `available_height` est
/// stable dès la deuxième image — 365, 708 puis 806 pixels sur trois
/// tailles d'écran, sans jamais bouger ensuite.
///
/// Le nombre fixe, lui, était faux dans les deux sens : sur un grand
/// écran il restait 663 pixels sous la règle et le vidage n'en prenait
/// que 330, soit la moitié du panneau en blanc ; sur un petit il n'en
/// restait que 174, et le vidage débordait de 156, si bien qu'un
/// second ascenseur paraissait à côté du sien.
const LIGNES_MIN: usize = 8;

/// La largeur qu'il faut à l'inspecteur pour vivre dans une colonne à
/// droite du vidage plutôt que sous lui.
///
/// Sous le vidage, il lui prenait sa hauteur pendant que le quart droit
/// du panneau restait vide — et la grille ne gagne rien à être bornée à
/// une largeur de LECTURE, qui est faite pour de la prose. À côté, la
/// place perdue sert, et le vidage garde toute la hauteur.
///
/// Deux cent soixante pixels : de quoi poser « float64 LE » et
/// « 2.074285e-317 » sur une ligne, ce qui est la plus longue que
/// l'inspecteur ait à écrire depuis qu'un flottant illisible se dit en
/// notation scientifique. En dessous, il se replierait plus qu'il ne se
/// lirait, et il vaut mieux l'empiler.
///
/// Trois cents d'abord, et mesuré ensuite : dans l'onglet d'analyse, la
/// grille à trente-deux octets prend 1072 des 1346 pixels offerts, et
/// les 274 qui restent tombaient six pixels sous la barre. L'inspecteur
/// repassait sous la grille pour six pixels.
const INSPECTEUR_LARGEUR_MIN: f32 = 260.0;

/// La hauteur que le bas du panneau se réserve quand un octet est
/// choisi.
///
/// Il la prenait TOUTE : quatre cents pixels de position, de boutons,
/// d'aide et de lectures, posés sous un vidage qui gardait ses
/// vingt-deux lignes — et les dernières lectures se faisaient couper
/// par le bord. Bornée, cette place est connue d'avance : le vidage
/// sait ce qui lui reste, et ce qui dépasse se fait défiler là où il
/// est, sans emmener le panneau avec lui.
const INSPECTEUR_MAX: f32 = 250.0;

/// Où chaque colonne du vidage commence, en pixels depuis le bord
/// gauche de la ligne.
///
/// UNE GÉOMÉTRIE CALCULÉE, et c'est ce qui permet la règle des
/// colonnes. Chaque octet était jusqu'ici une étiquette posée dans une
/// rangée horizontale : leurs abscisses dépendaient des largeurs de
/// glyphe et des écarts d'egui, personne ne les connaissait, et l'on ne
/// pouvait donc rien écrire AU-DESSUS qui tombe juste. On comptait les
/// colonnes avec le doigt — exactement ce que la grille de bits de la
/// calculatrice évite depuis toujours avec ses index.
///
/// Les mesures sont en caractères, puis multipliées par la chasse :
/// c'est la seule façon qu'une règle et un vidage s'alignent quelle que
/// soit la police.
struct Colonnes {
    chasse: f32,
    par_ligne: usize,
    /// Combien de chiffres hexadécimaux une position demande.
    chiffres: usize,
    x_adresse: f32,
    x_hex: f32,
    x_texte: f32,
    largeur: f32,
}

impl Colonnes {
    fn new(chasse: f32, par_ligne: usize, adresses: bool, chiffres: usize) -> Self {
        // `chiffres` de position, deux espaces ; autant pour l'adresse
        // quand elle est là ; trois caractères par octet (« xx ») plus
        // un de plus entre deux groupes ; deux espaces, puis le texte.
        //
        // LE NOMBRE DE CHIFFRES EST TAILLÉ SUR LE FICHIER, et non écrit
        // d'avance. Huit zéros devant chaque ligne d'un fichier de cent
        // cinquante kilooctets, c'est trois colonnes qui ne disent
        // jamais rien d'autre que « zéro » — et quatre caractères de
        // large, deux fois, soit de quoi faire déborder le vidage de la
        // place qu'on lui laisse.
        let après_position = chiffres as f32 + 2.0;
        let après_adresse = if adresses { chiffres as f32 + 2.0 } else { 0.0 };
        let x_hex = après_position + après_adresse;
        let groupes = par_ligne.div_ceil(GROUPE).saturating_sub(1) as f32;
        let x_texte = x_hex + par_ligne as f32 * PAS_OCTET + groupes + 1.0;
        Self {
            chasse,
            par_ligne,
            chiffres,
            x_adresse: après_position * chasse,
            x_hex: x_hex * chasse,
            x_texte: x_texte * chasse,
            largeur: (x_texte + par_ligne as f32) * chasse,
        }
    }

    /// Le bord gauche de la cellule hexadécimale de cette colonne.
    fn x_octet(&self, col: usize) -> f32 {
        self.x_hex + (col as f32 * PAS_OCTET + (col / GROUPE) as f32) * self.chasse
    }

    /// Le bord gauche du caractère de cette colonne.
    fn x_car(&self, col: usize) -> f32 {
        self.x_texte + col as f32 * self.chasse
    }

    /// Quelle cellule se trouve sous cette abscisse, comptée depuis le
    /// bord gauche de la ligne ?
    ///
    /// Les deux colonnes sont cliquables, et le clic doit dire DANS
    /// LAQUELLE il est tombé : c'est elle qui décide de ce que la
    /// frappe suivante voudra dire — des chiffres hexadécimaux d'un
    /// côté, un caractère de l'autre.
    fn sous(&self, dx: f32) -> Option<(usize, Colonne)> {
        if dx >= self.x_texte - self.chasse * 0.5 {
            let col = ((dx - self.x_texte) / self.chasse).floor();
            let col = if col < 0.0 { 0 } else { col as usize };
            return (col < self.par_ligne).then_some((col, Colonne::Texte));
        }
        (0..self.par_ligne).find_map(|col| {
            let gauche = self.x_octet(col);
            // La moitié de l'espace qui suit appartient à la cellule :
            // viser entre deux octets ne doit pas rater les deux.
            (dx >= gauche - self.chasse * 0.3 && dx < gauche + self.chasse * 2.3)
                .then_some((col, Colonne::Hex))
        })
    }
}

/// Les deux bornes d'une sélection, dans l'ordre.
fn étendue(curseur: usize, ancre: Option<usize>) -> std::ops::Range<usize> {
    match ancre {
        Some(a) if a < curseur => a..curseur + 1,
        Some(a) => curseur..a + 1,
        None => curseur..curseur + 1,
    }
}

/// Dans quelle section de l'ELF cet octet tombe-t-il ?
///
/// `.text`, `.rodata`, `.data` : un nom plutôt qu'un nombre. C'est ce
/// qui change une position en LIEU, et c'est la première question qu'on
/// se pose devant un octet qu'on s'apprête à changer — parce que la
/// réponse décide si l'on touche à du code ou à une constante.
fn sections_par_position(octets: &[u8]) -> Vec<(String, std::ops::Range<usize>, bool, bool)> {
    let Some(scan) = crate::elf_scan::scan(octets) else {
        return Vec::new();
    };
    scan.sections
        .iter()
        // `.bss` et ses semblables n'occupent RIEN dans le fichier :
        // leur position désigne ce qui suit, et les compter ferait
        // nommer le voisin.
        .filter(|s| !s.nobits && s.size > 0)
        .map(|s| {
            let début = s.offset as usize;
            let fin = (début + s.size as usize).min(octets.len());
            (s.name.clone(), début..fin, s.write, s.execute)
        })
        .filter(|(_, plage, _, _)| plage.start < plage.end)
        .collect()
}

/// La colonne des adresses apprend-elle quelque chose sur ce fichier ?
///
/// Elle ne vaut d'être montrée que si une adresse diffère de sa
/// position. Un exécutable indépendant de la position charge son
/// premier segment à l'adresse zéro : les deux colonnes y seraient
/// identiques, et deux colonnes identiques côte à côte ne se lisent
/// pas — on les prend pour une erreur d'affichage.
///
/// Mesuré sur les débuts de segments, qui suffisent : à l'intérieur
/// d'un segment, l'écart entre l'adresse et la position est constant.
fn adresses_utiles(octets: &[u8]) -> bool {
    // Un pas qui couvre le fichier sans le parcourir octet par octet :
    // un segment fait au moins une page, et quatre mille octets
    // retombent dans chacun.
    let pas = 0x1000;
    (0..octets.len())
        .step_by(pas)
        .filter_map(|offset| {
            crate::imports::virtual_address(octets, offset as u64).map(|addr| (offset, addr))
        })
        .any(|(offset, addr)| addr != offset as u64)
}

/// Lit un nombre écrit en hexadécimal (`0x…`) ou en décimal.
fn nombre(texte: &str) -> Option<usize> {
    let t = texte.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16).ok()
    } else {
        t.parse().ok()
    }
}

impl ReModule for HexEditorModule {
    /// Une grille, et non de la prose : voir `ReModule::pleine_largeur`.
    fn pleine_largeur(&self) -> bool {
        true
    }

    fn section(&self) -> ReSection {
        ReSection::Bytes
    }

    fn id(&self) -> &'static str {
        "hex_editor"
    }

    fn title(&self, lang: &str) -> &'static str {
        if lang.starts_with("fr") {
            "Éditeur binaire"
        } else {
            "Binary editor"
        }
    }

    fn take_request(&mut self) -> Option<ReRequest> {
        self.demande.take()
    }

    fn on_binary_loaded(&mut self, ctx: &ReContext) {
        // Un autre fichier : ce qu'on avait touché ne le concerne plus.
        self.touchés.clear();
        self.choisi = None;
        self.ancre = None;
        self.amener = Some(0);
        self.trouvé = None;
        self.venu_de = None;
        self.sections = sections_par_position(ctx.binary);
        if self.par_ligne == 0 {
            // `Default` rend faux, et ce n'est pas ce qu'on veut : une
            // grille suit la place qu'on lui donne tant qu'on ne lui a
            // rien imposé. Posé ici, où le module se découvre.
            self.largeur_auto = true;
            self.par_ligne = PAR_LIGNE;
        }
        // Et la colonne des adresses ne paraît que si elle apprend
        // quelque chose : dans un exécutable indépendant de la position,
        // le premier segment est chargé à l'adresse zéro, et elle
        // répéterait la position chiffre pour chiffre.
        self.adresses = adresses_utiles(ctx.binary);
    }

    /// On arrive d'ailleurs : un panneau a désigné une adresse et l'on
    /// a demandé à la voir ICI.
    ///
    /// La page est cadrée sur la position, l'octet choisi, et l'adresse
    /// d'où l'on vient écrite en tête — sans quoi rien ne distinguerait
    /// cette arrivée d'un éditeur qu'on vient d'ouvrir au hasard.
    fn deliver(&mut self, result: &ReResult) {
        if let ReResult::ShowBytesAt { offset, addr } = result {
            self.amener = Some(*offset);
            self.choisi = Some(*offset);
            self.venu_de = *addr;
            self.trouvé = None;
            self.nibble = None;
            // Le champ « aller à » dit où l'on est : on vient d'y
            // arriver autrement, et il montrait encore la précédente.
            self.aller = format!("{offset:#x}");
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &ReContext) {
        let octets = ctx.binary;
        if octets.is_empty() {
            ui.weak(ctx.t(
                "Rien de chargé : il n'y a pas de fichier à éditer.",
                "Nothing loaded: there is no file to edit.",
            ));
            return;
        }

        ui.weak(ctx.t(
            "Choisissez un octet — ou plusieurs, avec Maj —, posez-en d'autres à la place. \
             Chaque écriture se défait depuis Outils › Correctifs, et rien ne touche le \
             disque tant que vous n'enregistrez pas vous-même, vers un fichier que vous \
             choisissez.",
            "Pick a byte — or several, with Shift — and put others in its place. Each \
             write can be undone from Tools › Patches, and nothing touches the disk until \
             you save it yourself, to a file you choose.",
        ));
        // D'OÙ L'ON VIENT. Une position seule ne dit pas qu'on y a été
        // envoyé : on a cliqué une adresse dans un autre panneau, et
        // l'éditeur s'est ouvert là. L'écrire ferme la boucle.
        if let Some(addr) = self.venu_de {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    ui.visuals().hyperlink_color,
                    egui::RichText::new(format!("{addr:#010x}")).monospace(),
                );
                ui.weak(ctx.t(
                    "— l'adresse d'où vous venez, ici dans le fichier.",
                    "— the address you came from, here in the file.",
                ));
            });
        }
        ui.add_space(6.0);

        // La barre de navigation : aller à un décalage. Plus de « page
        // précédente / suivante » — le fichier défile d'un bout à
        // l'autre, et il n'a jamais eu de pages.
        if self.par_ligne == 0 {
            // `Default` rend faux, et ce n'est pas ce qu'on veut : une
            // grille suit la place qu'on lui donne tant qu'on ne lui a
            // rien imposé. Posé ici, où le module se découvre.
            self.largeur_auto = true;
            self.par_ligne = PAR_LIGNE;
        }
        ui.horizontal_wrapped(|ui| {
            ui.weak(ctx.t("Aller à", "Go to"));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut self.aller)
                    .hint_text("0x1000")
                    .font(egui::TextStyle::Monospace)
                    .desired_width(90.0),
            );
            let validé = champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (framed_button(ui, ctx.t("Aller", "Go")).clicked() || validé)
                && let Some(n) = nombre(&self.aller)
            {
                // UNE POSITION, OU UNE ADRESSE. On lit une adresse dans
                // un listing — `0x401228` —, on la tape ici, et elle
                // tombait sur la position 0x401228, à quatre mégaoctets
                // de là ou hors du fichier. Un nombre qui dépasse le
                // fichier mais que les segments portent est donc lu
                // comme une adresse ; on ne perd rien, puisqu'une
                // position hors du fichier ne menait nulle part.
                let cible = match crate::imports::file_offset(octets, n as u64) {
                    Some(offset) if n >= octets.len() => offset as usize,
                    _ => n,
                }
                .min(octets.len().saturating_sub(1));
                self.amener = Some(cible);
                self.choisi = Some(cible);
                self.venu_de = None;
            }
            ui.separator();
            ui.weak(format!("{} {}", octets.len(), ctx.t("octets", "bytes")));
            ui.separator();
            // COMBIEN D'OCTETS PAR LIGNE. Trente-deux remplit un panneau
            // large et fait sauter aux yeux un motif qui se répète ;
            // huit tient dans un panneau étroit.
            //
            // C'ÉTAIT UN RÉGLAGE À LA MAIN, seize par défaut, et la
            // raison écrite était la même crainte que pour la hauteur :
            // lire la largeur offerte serait lire une dimension que ce
            // panneau fixe lui-même. Mesuré le 28 octobre 2026 dans
            // l'onglet d'analyse : mille trois cent quarante-six pixels
            // offerts, six cent six pris — SEPT CENT QUARANTE de vide,
            // plus que la grille entière —, et la largeur relevée ne
            // bouge pas d'une image à l'autre, parce que la zone qui
            // porte ce panneau impose la sienne au lieu de l'épouser.
            //
            // Le réglage reste, et fige ce qu'on a choisi : on veut
            // parfois comparer deux fichiers sur la même grille.
            ui.weak(ctx.t("par ligne", "per row"));
            if ui
                .selectable_label(self.largeur_auto, ctx.t("auto", "auto"))
                .on_hover_text(ctx.t(
                    "Le plus d'octets par ligne que la largeur permet.",
                    "As many bytes per row as the width allows.",
                ))
                .clicked()
            {
                self.largeur_auto = true;
            }
            for largeur in LARGEURS {
                if ui
                    .selectable_label(
                        !self.largeur_auto && self.par_ligne == largeur,
                        largeur.to_string(),
                    )
                    .clicked()
                {
                    self.largeur_auto = false;
                    self.par_ligne = largeur;
                }
            }
            // La colonne des adresses, qu'on peut retirer : sur un
            // fichier qu'aucun segment ne porte — un tampon brut, une
            // archive —, elle ne montrerait que des tirets.
            ui.checkbox(&mut self.adresses, ctx.t("adresses", "addresses"))
                .on_hover_text(ctx.t(
                    "L'adresse à laquelle chaque ligne sera lue en mémoire, à côté de sa \
                     position dans le fichier.",
                    "The address each line will be read at in memory, next to its position \
                     in the file.",
                ));
        });

        // La recherche : un motif en octets ou en texte, trouvé en avant
        // ou en arrière, et la trouvaille surlignée.
        ui.add_space(4.0);
        let mut cherche: Option<bool> = None; // Some(précédent)
        ui.horizontal_wrapped(|ui| {
            ui.weak(ctx.t("Chercher", "Find"));
            let champ = ui.add(
                egui::TextEdit::singleline(&mut self.recherche)
                    .hint_text(if self.recherche_hex {
                        "7f 45 4c 46"
                    } else {
                        "ELF"
                    })
                    .font(egui::TextStyle::Monospace)
                    .desired_width(160.0),
            );
            let entrée = champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.selectable_value(&mut self.recherche_hex, false, ctx.t("texte", "text"));
            ui.selectable_value(&mut self.recherche_hex, true, "hex");
            // PRÉCÉDENT À GAUCHE, SUIVANT À DROITE, comme partout
            // ailleurs. Ils étaient dans l'autre ordre — « › ‹ » —, et
            // les deux flèches pointaient l'une vers l'autre : rien à
            // l'écran ne disait plus laquelle avance. L'ordre venait de
            // ce que « suivant » est l'action par défaut, celle
            // d'Entrée ; elle l'est restée, sans avoir à passer devant.
            if framed_button(ui, "\u{2039}")
                .on_hover_text(ctx.t("Précédent", "Previous"))
                .clicked()
            {
                cherche = Some(true);
            }
            if framed_button(ui, "\u{203a}")
                .on_hover_text(ctx.t("Suivant", "Next"))
                .clicked()
                || entrée
            {
                cherche = Some(false);
            }
        });
        if let Some(précédent) = cherche {
            match motif(&self.recherche, self.recherche_hex) {
                None => {
                    self.trouvé = None;
                    ui.weak(ctx.t(
                        "Rien à chercher : un texte, ou des octets hexadécimaux par paires.",
                        "Nothing to search: a text, or hexadecimal bytes in pairs.",
                    ));
                }
                Some(m) => {
                    // D'où l'on repart : après l'octet choisi pour « suivant »,
                    // depuis lui pour « précédent ».
                    let depuis = match self.choisi {
                        Some(c) if !précédent => c + 1,
                        Some(c) => c,
                        None => 0,
                    };
                    match trouver(octets, &m, depuis, précédent) {
                        Some(pos) => {
                            self.trouvé = Some(pos..pos + m.len());
                            self.choisi = Some(pos);
                            self.valeur = format!("{:02x}", octets[pos]);
                            self.amener = Some(pos);
                        }
                        None => {
                            self.trouvé = None;
                            ui.weak(ctx.t("Introuvable.", "Not found."));
                        }
                    }
                }
            }
        }

        ui.add_space(6.0);
        let adresses = self.adresses;
        let visuals = ui.visuals().clone();
        let fond_choisi = visuals.selection.bg_fill;
        let fond_touché = visuals.warn_fg_color.gamma_multiply(0.30);
        let fond_trouvé = visuals.hyperlink_color.gamma_multiply(0.35);
        let mut nouveau_choix: Option<usize> = None;
        let mut nouvelle_colonne: Colonne = self.colonne;
        // La sélection courante, relevée avant de peindre : la grille
        // l'emprunte en lecture seule, et le clic ne la change qu'après.
        let sélection = match self.choisi {
            Some(curseur) => étendue(curseur, self.ancre),
            None => 0..0,
        };
        let mut étendre = false;

        let police = egui::TextStyle::Monospace.resolve(ui.style());
        // La chasse, demandée à la police et non devinée : c'est elle
        // qui fait tomber la règle juste au-dessus des octets.
        // Mesurée sur HUIT caractères puis divisée : un seul glyphe
        // rend une largeur arrondie, et l'erreur, multipliée par les
        // quatre-vingt-dix colonnes d'une ligne, décalait la règle d'un
        // caractère entier au bout.
        let gabarit = ui.painter().layout_no_wrap(
            "00000000".to_owned(),
            police.clone(),
            egui::Color32::PLACEHOLDER,
        );
        let chasse = gabarit.size().x / 8.0;
        let hauteur = gabarit.size().y;
        let par_ligne = self.par_ligne.max(1);
        // Les chiffres que ce fichier-ci demande : six au minimum —
        // en dessous, la colonne est trop étroite pour se lire comme
        // une adresse — et deux de plus par tranche de seize fois plus
        // grand. La plus haute adresse compte autant que la plus haute
        // position : un segment peut être chargé bien au-delà de la
        // fin du fichier.
        let plus_haut = octets.len().max(
            crate::imports::virtual_address(octets, octets.len() as u64 / 2).unwrap_or(0) as usize,
        );
        let chiffres = format!("{plus_haut:x}").len().max(6).div_ceil(2) * 2;
        // LE PLUS D'OCTETS QUE LA LARGEUR PERMET, tant qu'on ne l'a pas
        // figé. On essaie du plus large au plus étroit et l'on garde le
        // premier qui tient ; si aucun ne tient, le plus étroit, et la
        // zone se dote d'une barre horizontale — ce qui est la bonne
        // réponse dans un panneau où rien ne tiendrait.
        let par_ligne = if self.largeur_auto {
            // Deux barres de moins, comme la zone : voir `zone_large`.
            let barre = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin;
            let offert = ui.available_width() - 2.0 * barre;
            LARGEURS
                .iter()
                .rev()
                .copied()
                .find(|n| Colonnes::new(chasse, *n, adresses, chiffres).largeur <= offert)
                .unwrap_or(LARGEURS[0])
        } else {
            par_ligne
        };
        self.par_ligne = par_ligne;
        let cols = Colonnes::new(chasse, par_ligne, adresses, chiffres);
        let lignes = octets.len().div_ceil(par_ligne);
        // LA LARGEUR VUE EST BORNÉE PAR CE QU'ON OFFRE, et c'est un
        // plafond, jamais un plancher : du contenu qui prend MOINS que
        // la place disponible ne peut pas faire grandir la fenêtre qui
        // la lui offre. C'est ce qui rend cette lecture sûre là où la
        // lecture symétrique — prendre toute la place — reboucle.
        //
        // Sans elle, la règle était allouée à la largeur du vidage, qui
        // dépassait le panneau : celui-ci se dotait d'une barre
        // horizontale, la zone du vidage d'une seconde juste en
        // dessous, et deux barres l'une sur l'autre pour trois pixels à
        // faire défiler.
        // L'ASCENSEUR VERTICAL SE POSE À L'INTÉRIEUR DE `max_width`, et
        // c'est tout le contraire de ce que le premier essai supposait.
        // Il rogne la vue de sa propre largeur : une zone bornée à la
        // largeur du vidage ne montrait que `largeur − barre`, le
        // contenu dépassait d'une dizaine de pixels, et une barre
        // HORIZONTALE paraissait sous un vidage qui tenait pourtant
        // dans la place offerte. Mesuré dans l'application elle-même —
        // ni le banc ni la capture ne pouvaient trancher : l'un voyait
        // une barre légitime, l'autre une barre de trop.
        //
        // La zone est donc bornée à « vidage + barre », et la règle à
        // la VUE, c'est-à-dire à ce qui reste une fois la barre posée.
        let barre = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin;
        // DEUX FOIS LA BARRE, et mesuré : egui la retranche une fois de
        // la largeur qu'on impose, et une fois encore pour le rectangle
        // intérieur. Bornée à « vidage + une barre », la zone n'offrait
        // au vidage que sa largeur MOINS quatorze pixels, et se dotait
        // d'une barre horizontale pour ces quatorze-là — sur la
        // disposition qui remplit le panneau, c'est-à-dire précisément
        // celle qu'on venait de choisir pour lui.
        let zone_large = (cols.largeur + 2.0 * barre).min(ui.available_width());
        let vue = (zone_large - 2.0 * barre).max(chasse * 20.0);

        // LA RÈGLE DES COLONNES, au-dessus du vidage et alignée au
        // pixel sur lui. Sans elle on compte les colonnes avec le
        // doigt pour savoir à quelle position un octet se trouve ;
        // avec elle, l'adresse se lit d'un trait — la ligne donne les
        // chiffres de poids fort, la colonne le dernier.
        //
        // Et les deux colonnes d'adresses portent enfin leur nom. Elles
        // se ressemblent assez pour être prises l'une pour l'autre —
        // sur une page où segment et fichier coïncident, elles sont
        // identiques chiffre pour chiffre —, et rien ne disait laquelle
        // on patche et laquelle on lit dans le listing.
        let (cadre, _) = ui.allocate_exact_size(egui::vec2(vue, hauteur), egui::Sense::hover());
        {
            // Rognée au cadre, et décalée comme le vidage : une règle
            // qui ne suivrait pas le défilement horizontal désignerait
            // la mauvaise colonne, ce qui est pire que pas de règle du
            // tout. Le décalage est celui de l'image PRÉCÉDENTE — il
            // n'existe qu'une fois la zone dessinée, et l'écart d'une
            // image ne se voit pas.
            let p = ui.painter().with_clip_rect(cadre);
            let cadre = cadre.translate(egui::vec2(-self.décalage_x, 0.0));
            let dim = visuals.weak_text_color();
            let écrire = |x: f32, texte: String, couleur: egui::Color32| {
                p.text(
                    egui::pos2(cadre.left() + x, cadre.center().y),
                    egui::Align2::LEFT_CENTER,
                    texte,
                    police.clone(),
                    couleur,
                );
            };
            // « fichier » et « mémoire », et non « position » et
            // « adresse ». Deux raisons, et la seconde est la bonne :
            // les mots tiennent dans la colonne — « position » fait
            // huit caractères pour un emplacement qui en a huit, et se
            // collait à l'étiquette suivante —, et surtout ils disent
            // la DIFFÉRENCE au lieu de la supposer connue. L'une dit où
            // l'octet est écrit, l'autre où il sera lu ; on patche à la
            // première et l'on raisonne avec la seconde, et c'est pour
            // avoir à les rapprocher de tête qu'on se trompe de ligne.
            écrire(0.0, ctx.t("fichier", "file").to_owned(), dim);
            if adresses {
                écrire(
                    cols.x_adresse,
                    ctx.t("mémoire", "memory").to_owned(),
                    visuals.hyperlink_color.gamma_multiply(0.75),
                );
            }
            for col in 0..par_ligne {
                écrire(cols.x_octet(col), format!("{col:02x}"), dim);
            }
            écrire(cols.x_texte, ctx.t("texte", "text").to_owned(), dim);
        }
        // Un filet sous la règle : c'est un en-tête, pas une ligne du
        // vidage, et rien d'autre ne le dirait.
        {
            let y = cadre.bottom() + 2.0;
            ui.painter().hline(
                cadre.left()..=cadre.right(),
                y,
                egui::Stroke::new(1.0, visuals.weak_text_color().gamma_multiply(0.5)),
            );
        }
        ui.add_space(4.0);

        // LE FICHIER ENTIER DÉFILE, d'un bout à l'autre. Il n'a pas de
        // pages : `show_rows` ne construit que les lignes visibles, si
        // bien qu'un fichier de plusieurs mégaoctets se parcourt sans
        // que rien ne soit peint hors de l'écran.
        // AUCUN ÉCART ENTRE LES LIGNES, et posé ICI, avant la zone.
        //
        // Il l'était à l'intérieur, et c'était trop tard : `show_rows`
        // compte ses lignes AVANT d'appeler ce qu'on lui donne, et il
        // les compte à `hauteur + item_spacing.y`. Relevé le 28 octobre
        // 2026 : une zone de six cent trente pixels, quarante-deux
        // lignes demandées, TRENTE ET UNE peintes — six cent trente
        // divisé par vingt, et non par quinze. Les cent soixante-cinq
        // pixels qui manquaient restaient blancs AU BAS DE LA ZONE, et
        // l'on cherchait le défaut dans la fenêtre.
        //
        // Le nombre de lignes figé d'avant portait le même écart sans
        // que personne le voie : « vingt-deux » en montrait seize.
        ui.spacing_mut().item_spacing.y = 0.0;
        // CE QUI RESTE, et non un nombre écrit d'avance : voir
        // `LIGNES_MIN`, qui porte la mesure.
        // À CÔTÉ PLUTÔT QUE DESSOUS, dès qu'il y a la largeur : le
        // vidage garde alors toute la hauteur, et l'inspecteur n'a plus
        // à se serrer dans deux cent cinquante pixels.
        // La largeur offerte, relevée AVANT que la grille ne la
        // consomme : c'est elle qui borne la colonne de droite. Prendre
        // `max_rect` à la place la laissait s'étendre au-delà de ce que
        // le panneau montre, et le panneau se dotait d'une barre
        // horizontale dont il n'avait pas l'usage.
        let largeur_offerte = ui.available_width();
        let bord_droit = ui.cursor().left() + largeur_offerte;
        let en_colonne = largeur_offerte >= zone_large + INSPECTEUR_LARGEUR_MIN + 12.0;
        let réserve = if self.choisi.is_some() && !en_colonne {
            INSPECTEUR_MAX + 16.0
        } else {
            hauteur + 12.0
        };
        let lignes_vues = (((ui.available_height() - réserve) / hauteur).floor() as isize)
            .max(LIGNES_MIN as isize) as usize;
        let mut zone = egui::ScrollArea::both()
            .id_salt("hex_dump")
            .max_height(hauteur * lignes_vues as f32)
            // LARGEUR : la zone épouse le vidage. Elle réclamait toute
            // la place offerte, ce qui posait son ascenseur à l'autre
            // bout du panneau — à quarante pixels de ce qu'il fait
            // défiler — et débordait d'assez peu pour que le panneau se
            // dote d'une barre horizontale dont il n'avait pas l'usage.
            //
            // HAUTEUR : imposée, et c'est l'inverse. Une hauteur qui
            // épouse son contenu dans une fenêtre qui épouse le sien se
            // reboucle, et ce panneau connaît la chanson. Imposée ne
            // veut pas dire figée : c'est la place offerte qui la
            // donne, et c'est tout l'objet de `LIGNES_MIN`.
            //
            // `both` plutôt que `vertical` : à trente-deux octets par
            // ligne dans un panneau étroit, le vidage dépasse, et ce
            // qu'on ne peut pas atteindre n'existe pas. La barre ne
            // paraît que s'il y a de quoi défiler.
            .max_width(zone_large)
            .auto_shrink([false, false]);
        // Un saut demandé : « aller à », une trouvaille, une arrivée
        // d'un autre panneau, ou le curseur sorti de l'écran. La ligne
        // visée est amenée au tiers haut de la zone — collée en haut,
        // on ne voit pas ce qui la précède, et c'est souvent là qu'est
        // la réponse.
        if let Some(off) = self.amener.take() {
            let y = (off / par_ligne) as f32 * hauteur;
            zone = zone.vertical_scroll_offset((y - hauteur * (lignes_vues as f32 / 3.0)).max(0.0));
        }
        let sortie = zone.show_rows(ui, hauteur, lignes, |ui, plage| {
            for index in plage.clone() {
                let début = index * par_ligne;
                let fin = (début + par_ligne).min(octets.len());
                let (rect, réponse) =
                    ui.allocate_exact_size(egui::vec2(cols.largeur, hauteur), egui::Sense::click());
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let p = ui.painter();
                // Les fonds d'abord, le texte ensuite : peints dans
                // l'autre ordre, un fond recouvrirait ses chiffres.
                for off in début..fin {
                    let col = off - début;
                    // TOUTE LA SÉLECTION, et non le seul curseur : ce
                    // qu'on va remplacer doit se voir avant qu'on le
                    // remplace. Le curseur garde un liseré, plus bas,
                    // pour qu'on sache lequel des trente octets la
                    // frappe touchera.
                    let fond = if sélection.contains(&off) {
                        Some(fond_choisi)
                    } else if self.trouvé.as_ref().is_some_and(|r| r.contains(&off)) {
                        Some(fond_trouvé)
                    } else if self.touchés.contains(&off) {
                        Some(fond_touché)
                    } else {
                        None
                    };
                    let Some(fond) = fond else { continue };
                    for (x, large) in [(cols.x_octet(col), chasse * 2.0), (cols.x_car(col), chasse)]
                    {
                        p.rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(rect.left() + x - 1.0, rect.top()),
                                egui::vec2(large + 2.0, hauteur),
                            ),
                            2.0,
                            fond,
                        );
                    }
                }
                // LE CURSEUR DANS LA SÉLECTION : un trait sous sa
                // cellule, peint APRÈS les fonds — dessiné avant, le
                // fond de la sélection le recouvrait, et l'on cherchait
                // un repère qui existait pourtant. Sans lui, une
                // sélection de trente octets ne dit pas lequel la
                // frappe suivante touche, ni de quel bout Maj+flèche
                // tirera.
                if sélection.len() > 1
                    && let Some(curseur) = self.choisi
                    && (début..fin).contains(&curseur)
                {
                    let x = rect.left() + cols.x_octet(curseur - début);
                    p.hline(
                        x..=x + chasse * 2.0,
                        rect.bottom() - 1.0,
                        egui::Stroke::new(1.5, visuals.text_color()),
                    );
                }
                let écrire = |x: f32, texte: String, couleur: egui::Color32| {
                    p.text(
                        egui::pos2(rect.left() + x, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        texte,
                        police.clone(),
                        couleur,
                    );
                };
                écrire(
                    0.0,
                    format!("{début:0larg$x}", larg = cols.chiffres),
                    visuals.weak_text_color(),
                );
                if adresses {
                    let texte = match crate::imports::virtual_address(octets, début as u64) {
                        Some(addr) => format!("{addr:0larg$x}", larg = cols.chiffres),
                        // Une ligne qu'aucun segment ne porte —
                        // l'en-tête, la table des symboles — : des
                        // tirets, et non un blanc, pour que la
                        // colonne reste une colonne.
                        None => "-".repeat(cols.chiffres),
                    };
                    écrire(
                        cols.x_adresse,
                        texte,
                        visuals.hyperlink_color.gamma_multiply(0.75),
                    );
                }
                for (col, &b) in octets[début..fin].iter().enumerate() {
                    écrire(cols.x_octet(col), format!("{b:02x}"), visuals.text_color());
                    let c = if (0x20..0x7f).contains(&b) {
                        b as char
                    } else {
                        '\u{b7}'
                    };
                    écrire(cols.x_car(col), c.to_string(), visuals.weak_text_color());
                }
                // Le clic est relevé sur la LIGNE et rendu à une
                // cellule par la géométrie : une étiquette par
                // octet coûtait trente-deux widgets par ligne, et
                // surtout ne disait pas où elle était.
                if réponse.clicked()
                    && let Some(pos) = réponse.interact_pointer_pos()
                    && let Some((col, colonne)) = cols.sous(pos.x - rect.left())
                    && début + col < fin
                {
                    nouveau_choix = Some(début + col);
                    nouvelle_colonne = colonne;
                    // Maj+clic ÉTEND au lieu de déplacer : le geste de
                    // toutes les listes, et le seul moyen de prendre
                    // trente octets sans les traverser à la flèche.
                    étendre = ui.input(|i| i.modifiers.shift);
                }
                if réponse.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
        });
        self.décalage_x = sortie.state.offset.x;
        let vues = sortie.inner_rect;
        // Ce que l'écran montre, en lignes : le clavier s'en sert pour
        // savoir quand le curseur en sort, et de combien « page
        // suivante » avance.
        let lignes_vues = ((vues.height() / hauteur).floor() as usize).max(1);

        if let Some(off) = nouveau_choix {
            if étendre {
                // Le bout fixe est celui d'où l'on vient, sauf si une
                // sélection était déjà tendue : on la prolonge alors
                // depuis SON ancre, et non depuis le curseur qu'on
                // vient de quitter.
                self.ancre = self.ancre.or(self.choisi);
            } else {
                self.ancre = None;
            }
            self.choisi = Some(off);
            self.valeur = format!("{:02x}", octets[off]);
            self.nibble = None;
            self.colonne = nouvelle_colonne;
            // Rien à faire du focus ici, et c'est MESURÉ : egui le rend
            // de lui-même dès qu'on clique un widget qui ne le prend
            // pas, et la grille retrouve donc le clavier sans qu'on le
            // demande. Un `surrender_focus` a été écrit ici le
            // 21 octobre 2026, puis retiré : les deux mesures — avec et
            // sans — donnaient « focus : None ». C'était un geste sans
            // effet, avec une explication qui sonnait juste.
        }

        // Le clavier, quand un octet est choisi et qu'aucun champ de texte
        // n'a le focus (sinon la frappe irait à « Chercher » ou « Aller
        // à »). Les flèches déplacent le curseur dans les deux colonnes ;
        // Tab passe de l'une à l'autre ; et la frappe s'interprète selon
        // la colonne active — chiffres hexa à gauche, caractère à droite.
        if let Some(off) = self.choisi
            && ui.memory(|m| m.focused()).is_none()
        {
            let dernier = octets.len().saturating_sub(1);
            // « Page suivante » avance d'un écran, et non d'un nombre
            // écrit d'avance : l'écran est ce qu'on voit, et c'est de
            // ça qu'on parle en disant « page ».
            let par_page = lignes_vues * par_ligne;
            let première_vue = (vues.top() / hauteur).floor().max(0.0) as usize;
            let dernière_vue = première_vue + lignes_vues;
            let mut bascule: Option<usize> = None;
            // Maj tenue : le déplacement TIRE la sélection au lieu de
            // la défaire. C'est le geste de tous les éditeurs, et sans
            // lui, prendre trente octets demandait de viser les deux
            // bouts à la souris.
            let mut étend_clavier = false;
            let mut chiffre: Option<u8> = None;
            let mut caractère: Option<u8> = None;
            let mut annule = false;
            let mut change_colonne = false;
            // Le presse-papiers : ce qu'on emporte, ce qu'on rapporte.
            let mut à_copier: Option<String> = None;
            let mut collé: Option<String> = None;
            let mut tout = false;
            let en_texte = self.colonne == Colonne::Texte;
            ui.input_mut(|i| {
                use egui::Key;
                let rien = egui::Modifiers::NONE;
                /// La touche est-elle tombée, et Maj était-elle tenue ?
                ///
                /// Les deux sont demandées d'un coup parce que ce sont
                /// la MÊME frappe : « droite » déplace, « Maj+droite »
                /// tire. Écrites en deux séries de `else if`, les huit
                /// déplacements auraient fait seize branches où l'on
                /// finit par en oublier une.
                fn tirer(i: &mut egui::InputState, k: egui::Key) -> Option<bool> {
                    // MAJ D'ABORD, et ce n'est pas une préférence :
                    // `consume_key` compare par `matches_logically`,
                    // qui exige les modificateurs du MOTIF et tolère
                    // ceux en trop dans l'évènement. `Modifiers::NONE`
                    // n'exige donc rien, et avalait les Maj+flèches
                    // avant que la ligne suivante ne les voie — le
                    // curseur se déplaçait sans rien sélectionner, et
                    // aucune de ces deux lignes n'avait l'air fausse.
                    if i.consume_key(egui::Modifiers::SHIFT, k) {
                        Some(true)
                    } else if i.consume_key(egui::Modifiers::NONE, k) {
                        Some(false)
                    } else {
                        None
                    }
                }
                // LE PRESSE-PAPIERS EN PREMIER : `Copie` et `Collage`
                // sont des évènements à part — egui les fabrique à
                // partir de Ctrl+C et Ctrl+V et n'en laisse pas la
                // touche —, et les lire avant les déplacements évite de
                // se demander lequel des deux a mangé la frappe.
                i.events.retain(|ev| match ev {
                    egui::Event::Copy | egui::Event::Cut => {
                        à_copier = Some(String::new());
                        false
                    }
                    egui::Event::Paste(texte) => {
                        collé = Some(texte.clone());
                        false
                    }
                    _ => true,
                });
                if let Some(maj) = tirer(i, Key::ArrowRight) {
                    bascule = Some((off + 1).min(dernier));
                    étend_clavier = maj;
                } else if let Some(maj) = tirer(i, Key::ArrowLeft) {
                    bascule = Some(off.saturating_sub(1));
                    étend_clavier = maj;
                } else if let Some(maj) = tirer(i, Key::ArrowDown) {
                    bascule = Some((off + par_ligne).min(dernier));
                    étend_clavier = maj;
                } else if let Some(maj) = tirer(i, Key::ArrowUp) {
                    bascule = Some(off.saturating_sub(par_ligne));
                    étend_clavier = maj;
                } else if let Some(maj) = tirer(i, Key::PageDown) {
                    bascule = Some((off + par_page).min(dernier));
                    étend_clavier = maj;
                } else if let Some(maj) = tirer(i, Key::PageUp) {
                    bascule = Some(off.saturating_sub(par_page));
                    étend_clavier = maj;
                } else if let Some(maj) = tirer(i, Key::Home) {
                    // Le début de la ligne, et Fin sa fin : les deux
                    // touches que tout éditeur de texte donne, et qui
                    // manquaient ici.
                    bascule = Some(off - off % par_ligne);
                    étend_clavier = maj;
                } else if let Some(maj) = tirer(i, Key::End) {
                    bascule = Some((off - off % par_ligne + par_ligne - 1).min(dernier));
                    étend_clavier = maj;
                } else if i.consume_key(rien, Key::Escape) {
                    annule = true;
                } else if i.consume_key(egui::Modifiers::COMMAND, Key::A) {
                    // Tout le fichier : le geste de toutes les
                    // fenêtres, et ici le seul moyen de dire « ces
                    // cent cinquante mille octets » sans les parcourir.
                    tout = true;
                } else if i.consume_key(rien, Key::Tab) {
                    change_colonne = true;
                } else if en_texte {
                    // Le premier caractère imprimable tapé devient l'octet.
                    for ev in &i.events {
                        if let egui::Event::Text(s) = ev
                            && let Some(c) = s.chars().next()
                            && c.is_ascii()
                            && !c.is_ascii_control()
                        {
                            caractère = Some(c as u8);
                            break;
                        }
                    }
                    // Consommés, pour ne pas les laisser filer ailleurs.
                    i.events.retain(|ev| !matches!(ev, egui::Event::Text(_)));
                } else {
                    // Mode hexadécimal : un chiffre ?
                    for (k, v) in [
                        (Key::Num0, 0u8),
                        (Key::Num1, 1),
                        (Key::Num2, 2),
                        (Key::Num3, 3),
                        (Key::Num4, 4),
                        (Key::Num5, 5),
                        (Key::Num6, 6),
                        (Key::Num7, 7),
                        (Key::Num8, 8),
                        (Key::Num9, 9),
                        (Key::A, 10),
                        (Key::B, 11),
                        (Key::C, 12),
                        (Key::D, 13),
                        (Key::E, 14),
                        (Key::F, 15),
                    ] {
                        if i.consume_key(rien, k) {
                            chiffre = Some(v);
                            break;
                        }
                    }
                }
            });
            if change_colonne {
                self.colonne = if en_texte {
                    Colonne::Hex
                } else {
                    Colonne::Texte
                };
                self.nibble = None;
            }
            if tout {
                self.ancre = Some(0);
                self.choisi = Some(dernier);
            }
            // CE QU'ON EMPORTE, dans l'écriture de la colonne active :
            // des octets hexadécimaux à gauche, le texte à droite.
            // C'est la même règle que pour la frappe, et c'est elle qui
            // rend le geste prévisible — on copie ce qu'on voit.
            if à_copier.is_some() {
                let plage = étendue(self.choisi.unwrap_or(off), self.ancre);
                if let Some(tranche) = octets.get(plage) {
                    let texte = if en_texte {
                        tranche
                            .iter()
                            .map(|b| {
                                if (0x20..0x7f).contains(b) {
                                    *b as char
                                } else {
                                    '.'
                                }
                            })
                            .collect()
                    } else {
                        tranche
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    };
                    ui.ctx().copy_text(texte);
                }
            }
            // CE QU'ON RAPPORTE VA DANS LE CHAMP, et n'écrit rien.
            //
            // C'est le seul endroit de ce panneau où la frappe
            // n'atteint pas le fichier, et c'est voulu : un collage
            // peut porter quatre mille octets venus d'on ne sait où.
            // Posé dans le champ, il se lit avant de partir, la ligne
            // qui dit ce que l'écriture ferait à la taille du fichier
            // s'affiche, et « Écrire » reste un geste qu'on fait.
            // Taper des chiffres, à côté, écrit UN octet qu'on vient de
            // voir : ce n'est pas le même risque.
            if let Some(texte) = collé {
                self.valeur = texte.trim().to_owned();
            }
            if annule {
                self.nibble = None;
                // Et la sélection avec : Échap défait ce qu'on vient de
                // tendre, comme il défait un quartet commencé.
                self.ancre = None;
            }
            if let Some(cible) = bascule {
                if étend_clavier {
                    self.ancre = self.ancre.or(Some(off));
                } else {
                    self.ancre = None;
                }
                self.choisi = Some(cible);
                self.valeur = format!("{:02x}", octets[cible]);
                self.nibble = None;
                // Suivre le curseur : s'il sort de la page, on tourne.
                // Suivre le curseur : s'il sort de ce qu'on voit, la
                // vue le rattrape.
                if cible / par_ligne < première_vue || cible / par_ligne >= dernière_vue {
                    self.amener = Some(cible);
                }
            }
            // Ce qu'on écrit, d'où qu'il vienne : un octet complet tapé en
            // hexa, ou un caractère tapé dans la colonne de texte.
            let à_écrire = if let Some(d) = chiffre {
                let (attente, complet) = frapper(self.nibble, d);
                self.nibble = attente;
                complet
            } else {
                caractère
            };
            if let Some(valeur) = à_écrire {
                if octets.get(off).copied() != Some(valeur) {
                    self.demande = Some(ReRequest::PatchBytes {
                        offset: off,
                        retirés: 1,
                        bytes: vec![valeur],
                    });
                    self.touchés.insert(off);
                }
                // On avance — la frappe continue toute seule, octet après
                // octet, dans l'une ou l'autre colonne.
                let suivant = (off + 1).min(dernier);
                self.choisi = Some(suivant);
                self.valeur = format!("{:02x}", octets.get(suivant).copied().unwrap_or(0));
                self.nibble = None;
                if suivant / par_ligne >= dernière_vue {
                    self.amener = Some(suivant);
                }
            }
        }

        if en_colonne {
            // LA PLACE À DROITE DU VIDAGE, celle qui restait vide. On
            // la prend sans imbriquer le vidage dans une colonne : il a
            // déjà dit où il s'arrête, et c'est tout ce qu'il faut
            // savoir pour poser l'inspecteur à côté de lui.
            let à_droite = egui::Rect::from_min_max(
                egui::pos2(vues.right() + 16.0, vues.top()),
                egui::pos2(bord_droit, vues.bottom()),
            );
            if à_droite.width() >= INSPECTEUR_LARGEUR_MIN {
                ui.painter().vline(
                    à_droite.left() - 8.0,
                    vues.y_range(),
                    egui::Stroke::new(1.0, visuals.weak_text_color().gamma_multiply(0.4)),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(à_droite), |ui| {
                    self.bas_du_panneau(ui, ctx, octets, &visuals, à_droite.height());
                });
            }
        } else {
            self.bas_du_panneau(ui, ctx, octets, &visuals, INSPECTEUR_MAX);
        }
    }
}

/// Lit ce qu'on cherche : une suite d'octets hexadécimaux (« 7f 45 4c »
/// ou « 7f454c »), ou un texte pris tel quel. Rien pour une saisie vide
/// ou un hexadécimal impair — on ne devine pas un demi-octet.
fn motif(query: &str, hex: bool) -> Option<Vec<u8>> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
    if !hex {
        return Some(q.as_bytes().to_vec());
    }
    let nettoyé: String = q.chars().filter(|c| !c.is_whitespace()).collect();
    if nettoyé.is_empty() || !nettoyé.len().is_multiple_of(2) {
        return None;
    }
    let octets = nettoyé.as_bytes();
    let mut out = Vec::with_capacity(octets.len() / 2);
    let mut i = 0;
    while i < octets.len() {
        let paire = std::str::from_utf8(&octets[i..i + 2]).ok()?;
        out.push(u8::from_str_radix(paire, 16).ok()?);
        i += 2;
    }
    Some(out)
}

/// Cherche le motif dans les octets, à partir de `depuis`, dans un sens ou
/// l'autre. Revient au début (ou à la fin) quand il n'y a plus rien devant
/// — chercher à nouveau fait le tour, comme dans un éditeur.
fn trouver(octets: &[u8], motif: &[u8], depuis: usize, précédent: bool) -> Option<usize> {
    if motif.is_empty() || motif.len() > octets.len() {
        return None;
    }
    let dernier = octets.len() - motif.len();
    let occupe = |i: usize| &octets[i..i + motif.len()] == motif;
    if précédent {
        (0..=dernier)
            .rev()
            .find(|&i| i < depuis && occupe(i))
            .or_else(|| (0..=dernier).rev().find(|&i| occupe(i)))
    } else {
        (depuis..=dernier)
            .find(|&i| occupe(i))
            .or_else(|| (0..=dernier).find(|&i| occupe(i)))
    }
}

/// L'inspecteur de données : comment lire les octets à partir du curseur,
/// dans les formats qu'un désassembleur rencontre. C'est l'outil phare de
/// 010 Editor, et le plus pédagogique — « ces quatre octets, c'est ce
/// nombre-là ». En petit-boutien (`LE`), celui du x86 : l'octet de poids
/// faible d'abord, ce qui déroute tant qu'on ne l'a pas vu une fois.
///
/// Fonction libre, pour l'éprouver sans interface.
/// Un flottant écrit pour être LU.
///
/// `to_string` d'un `f64` rend tous les chiffres qu'il faut pour le
/// relire exactement — et pour un motif d'octets quelconque, cela
/// donne « 0.000…0002074285 » sur trois cents caractères. Relevé sur
/// la capture du 28 octobre 2026 : trois lignes pleines qui sortaient
/// de la colonne de l'inspecteur.
///
/// Ce n'est pas un nombre qu'on lit, c'est une forme qu'on reconnaît :
/// la notation scientifique dit en quinze signes ce que trois cents ne
/// disaient pas. Les valeurs ordinaires, elles, gardent leur écriture
/// ordinaire — un `1.5` ne gagne rien à devenir `1.5e0`.
fn flottant(valeur: f64) -> String {
    if valeur == 0.0 || !valeur.is_finite() {
        return valeur.to_string();
    }
    let ampleur = valeur.abs();
    if (1e-4..1e16).contains(&ampleur) {
        let écrit = valeur.to_string();
        if écrit.len() <= 24 {
            return écrit;
        }
    }
    format!("{valeur:.6e}")
}

fn inspecter(octets: &[u8], be: bool) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let Some(&b) = octets.first() else {
        return out;
    };
    // Un octet seul ne dépend pas de l'ordre des octets.
    out.push(("int8", (b as i8).to_string()));
    out.push(("uint8", b.to_string()));
    out.push(("binaire", format!("{b:08b}")));
    out.push((
        "caractère",
        if (0x20..0x7f).contains(&b) {
            (b as char).to_string()
        } else {
            "·".to_owned()
        },
    ));
    if octets.len() >= 2 {
        let a = <[u8; 2]>::try_from(&octets[..2]).unwrap();
        let (i, u) = if be {
            (
                i16::from_be_bytes(a).to_string(),
                u16::from_be_bytes(a).to_string(),
            )
        } else {
            (
                i16::from_le_bytes(a).to_string(),
                u16::from_le_bytes(a).to_string(),
            )
        };
        out.push((if be { "int16 BE" } else { "int16 LE" }, i));
        out.push((if be { "uint16 BE" } else { "uint16 LE" }, u));
    }
    if octets.len() >= 4 {
        let a = <[u8; 4]>::try_from(&octets[..4]).unwrap();
        let (i, u, f) = if be {
            (
                i32::from_be_bytes(a).to_string(),
                u32::from_be_bytes(a).to_string(),
                flottant(f32::from_be_bytes(a) as f64),
            )
        } else {
            (
                i32::from_le_bytes(a).to_string(),
                u32::from_le_bytes(a).to_string(),
                flottant(f32::from_le_bytes(a) as f64),
            )
        };
        out.push((if be { "int32 BE" } else { "int32 LE" }, i));
        out.push((if be { "uint32 BE" } else { "uint32 LE" }, u));
        out.push((if be { "float32 BE" } else { "float32 LE" }, f));
    }
    if octets.len() >= 8 {
        let a = <[u8; 8]>::try_from(&octets[..8]).unwrap();
        let (i, u, f) = if be {
            (
                i64::from_be_bytes(a).to_string(),
                u64::from_be_bytes(a).to_string(),
                flottant(f64::from_be_bytes(a)),
            )
        } else {
            (
                i64::from_le_bytes(a).to_string(),
                u64::from_le_bytes(a).to_string(),
                flottant(f64::from_le_bytes(a)),
            )
        };
        out.push((if be { "int64 BE" } else { "int64 LE" }, i));
        out.push((if be { "uint64 BE" } else { "uint64 LE" }, u));
        out.push((if be { "float64 BE" } else { "float64 LE" }, f));
    }
    out
}

/// CE QUE VAUT L'OCTET CHOISI, lu dans chaque format — dit sans savoir
/// comment on le peint.
///
/// **La seule part de ce panneau qui passe la frontière de P7**, et
/// c'est une décision prise sur un relevé (voir `TRAVAUX.md`). Le reste
/// — la grille, son curseur, ses vingt-six touches — est un
/// INSTRUMENT : on s'en sert, il ne se raconte pas. L'inspecteur, lui,
/// est une compréhension pure : « ces huit octets valent 1 en entier
/// 64 bits, 4,94 e-324 en flottant, et `SOH` en caractère » est une
/// phrase qu'un rapport veut écrire, qu'un contrôle veut lire, et
/// qu'une autre interface voudra peindre autrement.
///
/// `octets` est la suite du fichier À PARTIR de l'octet choisi, comme
/// pour [`inspecter`] : c'est le même point de vue, et le dire deux
/// fois serait deux occasions de diverger.
pub fn vue_de_l_inspecteur(ctx: &ReContext, octets: &[u8], be: bool) -> Vue {
    let lectures = inspecter(octets, be);
    if lectures.is_empty() {
        return Vue::neuve();
    }
    // L'ORDRE DE LECTURE, et le choix de l'autre.
    //
    // Celui qui est en vigueur s'écrit en clair, l'autre est un lien :
    // le vocabulaire des vues n'a pas d'état « choisi », et plutôt que
    // d'en inventer un pour un seul usage — c'est ce que le relevé de
    // l'éditeur binaire a reproché à ses propres bascules —, on dit la
    // chose comme elle se lit. Un mot franc, un lien à côté ; presser
    // ce qui est déjà choisi n'a jamais rien fait de toute façon.
    let (ici, ailleurs, action) = if be {
        (
            ctx.t("gros-boutien", "big-endian"),
            ctx.t("petit-boutien", "little-endian"),
            "le",
        )
    } else {
        (
            ctx.t("petit-boutien", "little-endian"),
            ctx.t("gros-boutien", "big-endian"),
            "be",
        )
    };
    let mut vue = Vue::neuve().texte(vec![
        Fragment::dit(ctx.t("Lu à partir d'ici :", "Read from here:")).faible(),
        Fragment::dit(ici).fort(),
        Fragment::dit(ailleurs)
            .agit(format!("{BOUTISME}{action}"))
            .survol(ctx.t(
                "Lire les mêmes octets dans l'autre ordre",
                "Read the same bytes the other way round",
            )),
    ]);
    // LES LECTURES EN TABLEAU, et c'est une mesure qui l'a décidé
    // contre ce que le code d'avant redoutait.
    //
    // Ce panneau portait écrit qu'il ne fallait « AUCUNE disposition
    // imbriquée » ici : l'inspecteur était une grille, la fenêtre
    // d'analyse se dimensionnait sur son contenu, et un enfant qui
    // réclame toute la place offerte dans un parent qui se dimensionne
    // sur ses enfants est la boucle la plus connue d'egui — 1100 px,
    // puis 2878, puis deux de plus à chaque image. Le nom était donc
    // rembourré à onze caractères pour aligner les valeurs sans
    // colonne.
    //
    // Deux mesures ont tranché. (1) La boucle NE SE REPRODUIT PLUS :
    // scène du banc avec un octet choisi, vingt-cinq invariants dont
    // celui qui suit une fenêtre sur vingt images, tout reste vert
    // avec une grille — la fenêtre d'analyse borne son contenu depuis,
    // et c'est cette borne qui a fermé la boucle. (2) Le rembourrage,
    // lui, N'ALIGNE PLUS une fois les lectures décrites en fragments :
    // les blancs de fin ne survivent pas au rendu, et les valeurs
    // partaient de 51, 56, 59, 68 pixels selon la longueur du nom.
    // Une colonne qui ne colonne rien ne vaut pas qu'on la garde.
    vue = vue.bloc(Bloc::Tableau {
        colonnes: vec![],
        rangées: lectures
            .into_iter()
            .map(|(nom, valeur)| {
                vec![
                    Fragment::dit(nom).code().faible().into(),
                    // La valeur se copie d'un clic : on l'a sous les
                    // yeux pour la porter ailleurs — dans une
                    // calculatrice, dans un message, dans un autre
                    // outil.
                    Fragment::dit(valeur).code().copiable().into(),
                ]
            })
            .collect(),
        // Seize lignes courtes : le rayage les hacherait sans aider.
        rayé: false,
    });
    vue
}

/// Applique un chiffre hexadécimal à la frappe en place.
///
/// Rend le nouvel état d'attente et, quand l'octet est complet, la valeur
/// à écrire. Premier quartet : on le garde (poids fort), rien à écrire.
/// Second : l'octet est `haut<<4 | bas`, et l'attente retombe.
fn frapper(nibble: Option<u8>, chiffre: u8) -> (Option<u8>, Option<u8>) {
    match nibble {
        None => (Some(chiffre), None),
        Some(haut) => (None, Some((haut << 4) | chiffre)),
    }
}

impl HexEditorModule {
    /// Le bas du panneau : l'octet choisi, ce qu'il vaut lu dans
    /// chaque format, et de quoi le remplacer.
    ///
    /// À PART, parce qu'il se pose à deux endroits : sous le vidage
    /// quand le panneau est étroit, et DANS UNE COLONNE À DROITE dès
    /// qu'il y a la largeur. Sous le vidage, il lui prenait quatre
    /// cents pixels de hauteur et se faisait couper par le bas, dans
    /// un panneau dont le quart droit était vide.
    fn bas_du_panneau(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &ReContext,
        octets: &[u8],
        visuals: &egui::Visuals,
        hauteur_max: f32,
    ) {
        // L'octet choisi, et de quoi le remplacer.
        let Some(off) = self.choisi else {
            ui.add_space(4.0);
            ui.weak(ctx.t(
                "Cliquez un octet pour le choisir.",
                "Click a byte to pick it.",
            ));
            return;
        };
        // BORNÉ, ET DÉFILANT CHEZ LUI. Ce bloc fait quatre cents pixels
        // quand toutes les lectures tiennent ; posé sans borne sous un
        // vidage qui prenait déjà sa place, il sortait par le bas et
        // l'on allait chercher `int64` en faisant défiler la fenêtre
        // entière — alors qu'il y avait du vide juste au-dessus.
        egui::ScrollArea::vertical()
            .id_salt("hex_inspecteur")
            .max_height(hauteur_max)
            .auto_shrink([false, false])
            .show(ui, |ui| {
            {
            ui.add_space(8.0);
            ui.separator();
            // OÙ L'ON EST, avant ce qu'on y voit. Une position seule ne
            // dit rien : `0x2050` est un nombre, `.text` est un lieu —
            // et c'est le lieu qui décide si l'octet qu'on s'apprête à
            // changer est du code, une constante ou un nom de symbole.
            // La question se pose à chaque fois, et il fallait jusqu'ici
            // aller la poser à la carte du fichier, dans un autre
            // panneau.
            ui.horizontal_wrapped(|ui| {
                ui.monospace(
                    egui::RichText::new(format!("{} {off:#x}", ctx.t("position", "offset")))
                        .strong(),
                );
                // Et son adresse : c'est elle qu'on lit dans le
                // listing, et c'est par elle qu'on revient.
                if let Some(addr) = crate::imports::virtual_address(octets, off as u64) {
                    ui.weak("·");
                    ui.monospace(
                        egui::RichText::new(format!("{addr:#010x}")).color(visuals.hyperlink_color),
                    );
                }
                if let Some((nom, _, write, execute)) =
                    self.sections.iter().find(|(_, p, _, _)| p.contains(&off))
                {
                    ui.weak("·");
                    let quoi = match (execute, write) {
                        (true, _) => ctx.t("du code", "code"),
                        (_, true) => ctx.t("des données modifiables", "writable data"),
                        _ => ctx.t("des données en lecture seule", "read-only data"),
                    };
                    ui.monospace(egui::RichText::new(nom).color(visuals.warn_fg_color))
                        .on_hover_text(quoi);
                    ui.weak(quoi);
                }
            });
            // LA SÉLECTION, ET CE QU'ON EN FAIT.
            //
            // Elle remplace le « un octet, une valeur » du premier jet.
            // Trois gestes manquaient, et ce sont les trois qu'on fait
            // devant un binaire : recouvrir une instruction entière de
            // `nop`, écrire une chaîne à la place d'une autre, et
            // ajouter ce qui n'y était pas.
            let plage = étendue(off, self.ancre);
            let combien = plage.len();
            let écrits = motif(&self.valeur, true);
            ui.horizontal_wrapped(|ui| {
                // Ce qu'on a sous la main, en toutes lettres : « 3
                // octets, 0x2050 … 0x2052 ». Une sélection qu'on ne
                // compte pas se compte à l'œil, et l'on se trompe d'un.
                ui.monospace(
                    egui::RichText::new(if combien == 1 {
                        format!("{:02x}", octets.get(off).copied().unwrap_or(0))
                    } else {
                        format!(
                            "{combien} {} {:#x} … {:#x}",
                            ctx.t("octets", "bytes"),
                            plage.start,
                            plage.end - 1
                        )
                    })
                    .strong(),
                );
                // « -> » et non « → » : la flèche n'existe pas dans
                // les polices qu'egui embarque, et se peignait en
                // carré vide.
                ui.weak(ctx.t("->", "->"));
                let champ = ui.add(
                    egui::TextEdit::singleline(&mut self.valeur)
                        .hint_text("90 90 90")
                        .font(egui::TextStyle::Monospace)
                        .desired_width(120.0),
                );
                let change = écrits
                    .as_ref()
                    .is_some_and(|b| !b.is_empty() && octets.get(plage.clone()) != Some(&b[..]));
                let validé = champ.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.add_enabled_ui(change, |ui| {
                    if (framed_button(ui, ctx.t("Écrire", "Write")).clicked() || (validé && change))
                        && let Some(bytes) = écrits.clone()
                    {
                        for o in plage.start..plage.start + bytes.len() {
                            self.touchés.insert(o);
                        }
                        self.demande = Some(ReRequest::PatchBytes {
                            offset: plage.start,
                            retirés: combien,
                            bytes,
                        });
                    }
                });
                // RECOUVRIR DE `nop` : le geste le plus fréquent de
                // tous, et celui qu'on faisait jusqu'ici octet par
                // octet en tapant « 9 0 » autant de fois qu'il le
                // fallait. Il garde la longueur, donc il ne décale
                // rien.
                if framed_button(ui, format!("{} 90", ctx.t("Remplir de", "Fill with")))
                    .on_hover_text(ctx.t(
                        "Recouvre la sélection de `nop`. La longueur ne change pas : \
                         rien ne se décale.",
                        "Covers the selection with `nop`s. The length does not change: \
                         nothing shifts.",
                    ))
                    .clicked()
                {
                    for o in plage.clone() {
                        self.touchés.insert(o);
                    }
                    self.demande = Some(ReRequest::PatchBytes {
                        offset: plage.start,
                        retirés: combien,
                        bytes: vec![0x90; combien],
                    });
                }
                // INSÉRER : le geste qui sort du cadre, et qui le dit.
                ui.add_enabled_ui(écrits.as_ref().is_some_and(|b| !b.is_empty()), |ui| {
                    if framed_button(ui, ctx.t("Insérer", "Insert"))
                        .on_hover_text(ctx.t(
                            "Ajoute les octets AVANT la sélection, sans rien retirer. Tout ce \
                             qui suit se décale, et un exécutable n'y survit presque jamais : \
                             ses en-têtes désignent alors autre chose. Refusé si le fichier \
                             ne se relit plus, et rien n'est perdu.",
                            "Adds the bytes BEFORE the selection, removing nothing. Everything \
                             after shifts, and an executable hardly ever survives it: its \
                             headers then point elsewhere. Refused if the file no longer \
                             loads, and nothing is lost.",
                        ))
                        .clicked()
                        && let Some(bytes) = écrits.clone()
                    {
                        self.demande = Some(ReRequest::PatchBytes {
                            offset: plage.start,
                            retirés: 0,
                            bytes,
                        });
                    }
                });
            });
            if !self.valeur.trim().is_empty() && écrits.is_none() {
                ui.weak(ctx.t(
                    "Des octets en hexadécimal, par paires : « 90 » ou « 48 31 c0 ».",
                    "Bytes in hexadecimal, in pairs: \"90\" or \"48 31 c0\".",
                ));
            }
            // CE QUE L'ÉCRITURE FERAIT À LA TAILLE DU FICHIER, dit
            // AVANT de cliquer. Le refus existe déjà après coup — le
            // fichier illisible n'est pas chargé —, mais apprendre
            // après que tout s'est décalé n'est pas apprendre : c'est
            // essayer.
            if let Some(bytes) = &écrits
                && !bytes.is_empty()
                && bytes.len() != combien
            {
                let delta = bytes.len() as isize - combien as isize;
                ui.colored_label(
                    visuals.warn_fg_color,
                    if delta > 0 {
                        ctx.t(
                            "« Écrire » ferait grandir le fichier : tout ce qui suit se décale.",
                            "\"Write\" would grow the file: everything after it shifts.",
                        )
                    } else {
                        ctx.t(
                            "« Écrire » ferait rétrécir le fichier : tout ce qui suit se décale.",
                            "\"Write\" would shrink the file: everything after it shifts.",
                        )
                    },
                );
            }
            // La frappe en place : le quartet déjà tapé, en attente du
            // second, et l'indice pour qui ne connaît pas le geste.
            if let Some(haut) = self.nibble {
                ui.weak(
                    egui::RichText::new(format!("{}{haut:x}_", ctx.t("frappe : ", "typing: ")))
                        .monospace(),
                );
            } else {
                let colonne = match self.colonne {
                    Colonne::Hex => ctx.t("colonne hexa", "hex column"),
                    Colonne::Texte => ctx.t("colonne texte", "text column"),
                };
                let geste = match self.colonne {
                    Colonne::Hex => ctx.t("chiffres hexa pour écrire", "hex digits to type"),
                    Colonne::Texte => ctx.t("un caractère pour écrire", "a character to type"),
                };
                ui.weak(format!(
                    "{colonne} — {} ; {geste}, Tab {}",
                    ctx.t(
                        "flèches pour se déplacer, Maj+flèches pour choisir plusieurs octets, Ctrl+A pour tout prendre, Ctrl+C pour emporter, Ctrl+V pour rapporter dans le champ",
                        "arrows to move, Shift+arrows to pick several bytes, Ctrl+A to take everything, Ctrl+C to copy, Ctrl+V to bring back into the field",
                    ),
                    ctx.t("pour changer de colonne", "to switch column"),
                ));
            }

            // L'INSPECTEUR, et il est le seul morceau de ce panneau
            // qui soit DÉCRIT plutôt que peint sur place : sa vue dit
            // ce que l'octet vaut, le peintre commun s'occupe du
            // reste. Voir `vue_de_l_inspecteur`, et `TRAVAUX.md` pour
            // la raison de s'arrêter là.
            let reste = &octets[off..];
            let vue = vue_de_l_inspecteur(ctx, reste, self.inspecteur_be);
            if !vue.est_vide() {
                ui.add_space(6.0);
                // DEUX PIXELS ENTRE LES LECTURES, et c'est l'interface
                // qui le dit : seize lignes au rythme ordinaire
                // allongent la colonne d'un tiers pour rien.
                ui.spacing_mut().item_spacing.y = 2.0;
                let gestes = crate::vue_peinte::peindre_tout(ui, ctx, &vue);
                for action in &gestes.actions {
                    if let Some(quoi) = action.strip_prefix(BOUTISME) {
                        self.inspecteur_be = quoi == "be";
                    }
                }
            }
        }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_nombre_se_lit_en_hex_ou_en_decimal() {
        assert_eq!(nombre("0x10"), Some(16));
        assert_eq!(nombre("16"), Some(16));
        assert_eq!(nombre("  0X1f "), Some(31));
        assert_eq!(nombre("zz"), None);
        assert_eq!(nombre(""), None);
    }

    /// Changer d'octet oublie ce qu'on avait touché : un nouveau fichier
    /// n'hérite pas des marques de l'ancien.
    /// L'inspecteur lit les octets dans chaque format, en petit-boutien,
    /// et ne propose que les largeurs qui tiennent dans ce qui reste.
    /// La frappe en place assemble l'octet quartet par quartet.
    #[test]
    fn la_frappe_assemble_l_octet_par_quartets() {
        // Rien tapé : le premier chiffre devient le quartet de poids fort,
        // rien n'est encore écrit.
        assert_eq!(frapper(None, 0x9), (Some(0x9), None));
        // Un quartet en attente : le second complète l'octet, l'attente
        // retombe. 0x9 puis 0x0 -> 0x90.
        assert_eq!(frapper(Some(0x9), 0x0), (None, Some(0x90)));
        // 0x7 puis 0x4 -> 0x74 (je).
        assert_eq!(frapper(Some(0x7), 0x4), (None, Some(0x74)));
        // 0x0 puis 0xf -> 0x0f.
        assert_eq!(frapper(Some(0x0), 0xf), (None, Some(0x0f)));
    }

    /// La colonne des adresses ne paraît que si elle apprend quelque
    /// chose.
    ///
    /// Deux colonnes identiques côte à côte ne se lisent pas : on les
    /// prend pour une erreur d'affichage. Le cas n'est pas rare — c'est
    /// celui de tout exécutable indépendant de la position, dont le
    /// premier segment est chargé à l'adresse zéro.
    #[test]
    fn la_colonne_des_adresses_ne_parait_que_si_elle_apprend_quelque_chose() {
        // Rien du tout : pas d'adresse, pas de colonne.
        assert!(!adresses_utiles(&[]));
        assert!(!adresses_utiles(b"ceci n'est pas un ELF"));

        // Un ELF minimal, un segment chargeable, et l'adresse qu'on lui
        // donne. Chargé à zéro, la colonne répéterait la position ;
        // chargé à 0x400000, elle dit autre chose.
        let elf = |vaddr: u64| -> Vec<u8> {
            let mut f = vec![0u8; 0x1000];
            f[..4].copy_from_slice(b"\x7fELF");
            // e_phoff = 0x40, e_phentsize = 56, e_phnum = 1
            f[0x20..0x28].copy_from_slice(&64u64.to_le_bytes());
            f[0x36..0x38].copy_from_slice(&56u16.to_le_bytes());
            f[0x38..0x3a].copy_from_slice(&1u16.to_le_bytes());
            // Le segment : PT_LOAD, offset 0, vaddr, filesz 0x1000.
            f[64..68].copy_from_slice(&1u32.to_le_bytes());
            f[72..80].copy_from_slice(&0u64.to_le_bytes());
            f[80..88].copy_from_slice(&vaddr.to_le_bytes());
            f[96..104].copy_from_slice(&0x1000u64.to_le_bytes());
            f
        };
        assert!(
            !adresses_utiles(&elf(0)),
            "chargé à zéro, l'adresse EST la position"
        );
        assert!(
            adresses_utiles(&elf(0x400000)),
            "chargé ailleurs, la colonne apprend quelque chose"
        );
    }

    /// Un flottant se lit, ou se dit en notation scientifique.
    ///
    /// `to_string` d'un `f64` rend tous les chiffres nécessaires pour
    /// le relire exactement : sur un motif d'octets quelconque, trois
    /// cents caractères de zéros qui sortaient de la colonne.
    #[test]
    fn un_flottant_ne_deborde_jamais_de_sa_ligne() {
        // Ce qui se lit garde son écriture ordinaire.
        assert_eq!(flottant(1.5), "1.5");
        assert_eq!(flottant(0.0), "0");
        assert_eq!(flottant(-42.0), "-42");
        // Ce qui ne se lit pas se dit autrement — et tient.
        let minuscule = f64::from_le_bytes([0x88, 0xf0, 0x3f, 0, 0, 0, 0, 0]);
        let écrit = flottant(minuscule);
        assert!(
            écrit.len() <= 24,
            "un flottant doit tenir sur sa ligne : {écrit:?} ({} signes)",
            écrit.len()
        );
        assert!(écrit.contains('e'), "en notation scientifique : {écrit:?}");
        // Et l'on ne perd rien de ce qui compte : l'ordre de grandeur.
        assert!(flottant(f64::INFINITY).contains("inf"), "l'infini se nomme");
    }

    #[test]
    fn un_motif_se_lit_en_hex_ou_en_texte() {
        assert_eq!(motif("ELF", false), Some(b"ELF".to_vec()));
        assert_eq!(
            motif("7f 45 4c 46", true),
            Some(vec![0x7f, 0x45, 0x4c, 0x46])
        );
        assert_eq!(motif("7f454c46", true), Some(vec![0x7f, 0x45, 0x4c, 0x46]));
        // Un demi-octet ne se devine pas.
        assert_eq!(motif("7f4", true), None);
        assert_eq!(motif("zz", true), None);
        assert_eq!(motif("   ", false), None);
    }

    /// La recherche va en avant, en arrière, et fait le tour quand il n'y
    /// a plus rien devant — comme dans un éditeur.
    #[test]
    fn la_recherche_trouve_et_fait_le_tour() {
        let octets = b"ab_ab_ab"; // 'ab' en 0, 3, 6
        let m = b"ab";
        // En avant depuis 0 : la première.
        assert_eq!(trouver(octets, m, 0, false), Some(0));
        // En avant depuis 1 : la suivante.
        assert_eq!(trouver(octets, m, 1, false), Some(3));
        // En avant après la dernière : on fait le tour, retour à 0.
        assert_eq!(trouver(octets, m, 7, false), Some(0));
        // En arrière depuis 6 : la précédente.
        assert_eq!(trouver(octets, m, 6, true), Some(3));
        // En arrière depuis 0 : le tour, la dernière.
        assert_eq!(trouver(octets, m, 0, true), Some(6));
        // Un motif plus long que le fichier : rien.
        assert_eq!(trouver(b"x", m, 0, false), None);
        // Absent.
        assert_eq!(trouver(octets, b"zz", 0, false), None);
    }

    #[test]
    fn l_inspecteur_lit_chaque_format() {
        let trouve = |lectures: &[(&str, String)], nom: &str| {
            lectures
                .iter()
                .find(|(n, _)| *n == nom)
                .map(|(_, v)| v.clone())
        };

        // 0x01 suivi de zéros : 1 dans toutes les largeurs non signées.
        let l = inspecter(&[0x01, 0, 0, 0, 0, 0, 0, 0], false);
        assert_eq!(trouve(&l, "uint8").as_deref(), Some("1"));
        assert_eq!(trouve(&l, "uint16 LE").as_deref(), Some("1"));
        assert_eq!(trouve(&l, "uint32 LE").as_deref(), Some("1"));
        assert_eq!(trouve(&l, "uint64 LE").as_deref(), Some("1"));

        // 0xff seul : -1 signé, 255 non signé, et les bits.
        let l = inspecter(&[0xff], false);
        assert_eq!(trouve(&l, "int8").as_deref(), Some("-1"));
        assert_eq!(trouve(&l, "uint8").as_deref(), Some("255"));
        assert_eq!(trouve(&l, "binaire").as_deref(), Some("11111111"));
        // Un seul octet : pas de largeur de 16 bits ou plus.
        assert!(trouve(&l, "int16 LE").is_none());
        assert!(trouve(&l, "uint32 LE").is_none());

        // Petit-boutien : 0x00 0x01 = 256, et non 1.
        let l = inspecter(&[0x00, 0x01], false);
        assert_eq!(trouve(&l, "uint16 LE").as_deref(), Some("256"));

        // Rien à lire sur rien.
        assert!(inspecter(&[], false).is_empty());

        // Gros-boutien : 0x00 0x01 se lit 1, et non 256 — l'inverse du
        // petit-boutien, et les étiquettes le disent.
        let be = inspecter(&[0x00, 0x01], true);
        assert_eq!(trouve(&be, "uint16 BE").as_deref(), Some("1"));
        assert!(
            trouve(&be, "uint16 LE").is_none(),
            "en gros-boutien, pas d'étiquette LE"
        );
    }

    #[test]
    fn un_nouveau_binaire_efface_les_marques() {
        use namsdec_core::{Cpu, Memory};
        let cpu = Cpu::new(Memory::new(0x400000, 0x1000));
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &[],
            source: "",
            binary: &[1, 2, 3],
            lang: "fr",
            names: None,
            patches: &[],
        };
        let mut module = HexEditorModule::default();
        module.touchés.insert(2);
        module.choisi = Some(2);
        module.amener = None;
        module.on_binary_loaded(&ctx);
        assert!(module.touchés.is_empty());
        assert_eq!(module.choisi, None);
        assert_eq!(
            module.amener,
            Some(0),
            "un fichier neuf se regarde par son début"
        );
    }
}
