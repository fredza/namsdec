//! Ce que le programme écrit, lu comme un terminal le lirait.
//!
//! Un programme qui s'adresse à un terminal ne lui envoie pas que des
//! lettres : il intercale des **séquences d'échappement** — efface
//! l'écran, remonte le curseur, passe en cyan, revient au normal. Un
//! crackme sur deux ouvre ainsi, par un logo en couleur.
//!
//! Montrées telles quelles, ces séquences se lisent « □[2J□[H□[36m » en
//! tête du logo, et le dessin qui suit paraît abîmé. C'est ce qu'on
//! voyait dans la console de namsdec comme dans la boîte de saisie.
//!
//! Ce module ne fait pas un terminal complet, et ne cherche pas à en
//! faire un : il lit ce qu'un programme de console écrit vraiment —
//! couleurs, effacement, retour en haut — et **jette proprement** le
//! reste plutôt que de l'afficher. Ce qui n'est pas reconnu disparaît
//! sans laisser de trace à l'écran ; ce qui l'est se voit comme sur un
//! vrai terminal.

/// Ce qu'une suite de caractères porte comme apparence.
///
/// Les seize couleurs d'un terminal, et rien d'autre : un programme qui
/// en demande deux cent cinquante-six se contente des huit de base dans
/// la quasi-totalité des cas, et les thèmes de namsdec ne portent pas
/// de palette de deux cent cinquante-six teintes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    /// L'encre, de 0 à 7, ou `None` pour celle du thème.
    pub couleur: Option<u8>,
    /// Vive : le « gras » d'un terminal, qui éclaircit la couleur au
    /// lieu d'épaissir le trait.
    pub vive: bool,
    /// Le fond, de 0 à 7, ou `None` pour celui du panneau.
    pub fond: Option<u8>,
    pub gras: bool,
    pub souligné: bool,
    /// L'encre et le fond échangés.
    pub inversé: bool,
}

/// Un morceau de texte et son apparence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Morceau {
    pub texte: String,
    pub style: Style,
}

/// L'écran tel qu'il faut le peindre : des lignes, faites de morceaux.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Écran {
    pub lignes: Vec<Vec<Morceau>>,
}

impl Écran {
    /// Le texte seul, sans les couleurs — pour chercher dedans, le
    /// copier, ou le mesurer.
    pub fn texte(&self) -> String {
        self.lignes
            .iter()
            .map(|ligne| {
                ligne
                    .iter()
                    .map(|m| m.texte.as_str())
                    .collect::<Vec<_>>()
                    .concat()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Combien de lignes.
    pub fn hauteur(&self) -> usize {
        self.lignes.len()
    }
}

/// L'état de l'analyseur, ligne par ligne.
#[derive(Default)]
struct Composition {
    lignes: Vec<Vec<Morceau>>,
    /// La ligne en cours, et la colonne où le curseur s'y trouve —
    /// comptée en caractères, parce qu'un retour chariot y ramène.
    courante: Vec<Morceau>,
    colonne: usize,
    style: Style,
}

impl Composition {
    /// Ajoute du texte à la position du curseur.
    ///
    /// À la position, et non à la fin : un `\r` ramène le curseur au
    /// début de la ligne, et ce qui suit RECOUVRE ce qui y était. C'est
    /// ainsi qu'un programme anime une barre de progression — sans
    /// cela, on lirait cinquante barres les unes sous les autres.
    fn écrire(&mut self, texte: &str) {
        if texte.is_empty() {
            return;
        }
        let déjà: usize = self.courante.iter().map(|m| m.texte.chars().count()).sum();
        if self.colonne >= déjà {
            // Au bout : on complète d'espaces si le curseur a été posé
            // plus loin que ce qui est écrit.
            let trou = self.colonne - déjà;
            if trou > 0 {
                self.courante.push(Morceau {
                    texte: " ".repeat(trou),
                    style: Style::default(),
                });
            }
            self.pousser(texte);
        } else {
            // Au milieu : on recompose la ligne en gardant ce qui est
            // avant le curseur et ce qui dépasse derrière le texte posé.
            let large = texte.chars().count();
            let avant = self.tranche(0, self.colonne);
            let après = self.tranche(self.colonne + large, usize::MAX);
            self.courante = avant;
            self.pousser(texte);
            for morceau in après {
                self.courante.push(morceau);
            }
        }
        self.colonne += texte.chars().count();
    }

    /// Ajoute à la fin, en prolongeant le dernier morceau quand il porte
    /// déjà le même style : une ligne de quatre-vingts caractères ne
    /// doit pas faire quatre-vingts morceaux.
    fn pousser(&mut self, texte: &str) {
        match self.courante.last_mut() {
            Some(dernier) if dernier.style == self.style => dernier.texte.push_str(texte),
            _ => self.courante.push(Morceau {
                texte: texte.to_owned(),
                style: self.style,
            }),
        }
    }

    /// Les morceaux de la ligne courante entre deux colonnes.
    fn tranche(&self, début: usize, fin: usize) -> Vec<Morceau> {
        let mut out: Vec<Morceau> = Vec::new();
        let mut colonne = 0usize;
        for morceau in &self.courante {
            let large = morceau.texte.chars().count();
            let (d, f) = (colonne, colonne + large);
            colonne = f;
            if f <= début || d >= fin {
                continue;
            }
            let de = début.saturating_sub(d);
            let à = (fin.min(f) - d).min(large);
            let texte: String = morceau.texte.chars().skip(de).take(à - de).collect();
            if !texte.is_empty() {
                out.push(Morceau {
                    texte,
                    style: morceau.style,
                });
            }
        }
        out
    }

    /// Termine la ligne courante et passe à la suivante.
    fn saut(&mut self) {
        let ligne = std::mem::take(&mut self.courante);
        self.lignes.push(ligne);
        self.colonne = 0;
    }

    /// Efface de la position du curseur jusqu'au bout de la ligne.
    fn effacer_fin_de_ligne(&mut self) {
        self.courante = self.tranche(0, self.colonne);
    }

    /// Tout ce qui précède disparaît : le programme a demandé un écran
    /// net.
    fn effacer_tout(&mut self) {
        self.lignes.clear();
        self.courante.clear();
        self.colonne = 0;
    }
}

/// Lit une sortie brute et rend l'écran qu'elle décrit.
///
/// Les octets qui ne forment pas de l'UTF-8 valide passent en caractère
/// de remplacement plutôt que de faire échouer la lecture : un
/// programme peut écrire n'importe quoi, et namsdec doit le montrer
/// quand même.
pub fn écran(sortie: &[u8]) -> Écran {
    let texte = String::from_utf8_lossy(sortie);
    let mut c = Composition::default();
    let mut chars = texte.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\n' => c.saut(),
            // Le retour chariot seul ramène au début de la ligne. Suivi
            // d'un saut, il ne compte pas deux fois — d'où la lecture
            // du caractère suivant.
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                    c.saut();
                } else {
                    c.colonne = 0;
                }
            }
            // La tabulation : le prochain multiple de huit, comme
            // partout.
            '\t' => {
                let jusqu_à = (c.colonne / 8 + 1) * 8;
                let large = jusqu_à - c.colonne;
                c.écrire(&" ".repeat(large));
            }
            // Le retour arrière recule d'une colonne sans rien effacer :
            // ce qui suivra le recouvrira.
            '\u{8}' => c.colonne = c.colonne.saturating_sub(1),
            '\u{1b}' => lire_échappement(&mut chars, &mut c),
            // Les autres caractères de commande ne s'écrivent pas : la
            // sonnerie, le saut de page, le caractère nul.
            ch if (ch as u32) < 0x20 => {}
            ch => {
                let mut tampon = [0u8; 4];
                c.écrire(ch.encode_utf8(&mut tampon));
            }
        }
    }
    // La ligne en cours compte, même sans saut final : c'est elle qui
    // porte l'invite — « Enter the password: » n'est suivi d'aucun
    // retour, puisque le programme attend sur la même ligne.
    if !c.courante.is_empty() {
        c.saut();
    }
    Écran { lignes: c.lignes }
}

/// Lit ce qui suit un ESC, et l'applique.
///
/// Seules les séquences CSI (`ESC [ … lettre`) portent quelque chose
/// qui se voit ici. Les autres — un OSC qui renomme la fenêtre du
/// terminal, un jeu de caractères — sont lues jusqu'à leur fin et
/// jetées : il n'y a pas de fenêtre à renommer.
fn lire_échappement(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, c: &mut Composition) {
    match chars.next() {
        Some('[') => {
            let mut paramètres = String::new();
            for ch in chars.by_ref() {
                if ch.is_ascii_digit() || ch == ';' || ch == '?' {
                    paramètres.push(ch);
                } else {
                    appliquer_csi(&paramètres, ch, c);
                    return;
                }
            }
        }
        // Un OSC se termine par une sonnerie ou par `ESC \`.
        Some(']') => {
            while let Some(ch) = chars.next() {
                if ch == '\u{7}' {
                    return;
                }
                if ch == '\u{1b}' {
                    chars.next();
                    return;
                }
            }
        }
        // `ESC ( B` et les autres choix de jeu de caractères : un
        // caractère de plus, et rien à faire.
        Some('(') | Some(')') => {
            chars.next();
        }
        _ => {}
    }
}

/// Applique une séquence CSI dont on a les paramètres et la lettre.
fn appliquer_csi(paramètres: &str, lettre: char, c: &mut Composition) {
    // `ESC[?25l` — cacher le curseur — et ses semblables : il n'y a pas
    // de curseur à cacher dans une console qu'on lit.
    if paramètres.starts_with('?') {
        return;
    }
    let nombres: Vec<u32> = paramètres
        .split(';')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let premier = nombres.first().copied().unwrap_or(0);
    match lettre {
        // Effacer : 2 (tout), 0 (jusqu'en bas), 1 (jusqu'en haut). Les
        // trois reviennent ici au même, faute d'un écran de taille
        // fixe — et le cas qui compte, celui qu'un programme écrit en
        // ouvrant, est le 2.
        'J' => c.effacer_tout(),
        // Effacer dans la ligne : 0 (jusqu'au bout) est le cas courant.
        'K' => c.effacer_fin_de_ligne(),
        // Poser le curseur. La ligne est ignorée — l'écran n'a pas de
        // hauteur fixe ici — mais la COLONNE compte : c'est ainsi qu'un
        // programme écrit en tableau.
        'H' | 'f' => {
            let colonne = nombres.get(1).copied().unwrap_or(1);
            c.colonne = colonne.saturating_sub(1) as usize;
            // Aller en haut à gauche après un effacement est le geste
            // d'ouverture le plus courant ; sans écran de hauteur fixe,
            // il n'a rien d'autre à faire.
        }
        // Déplacer le curseur dans la ligne.
        'C' => c.colonne += premier.max(1) as usize,
        'D' => c.colonne = c.colonne.saturating_sub(premier.max(1) as usize),
        'G' => c.colonne = premier.saturating_sub(1) as usize,
        // Monter, descendre : sans écran de hauteur fixe, on ne peut
        // pas remonter dans ce qui est déjà écrit. On ne fait donc
        // rien, plutôt que de mélanger des lignes.
        'A' | 'B' => {}
        // L'apparence.
        'm' => appliquer_sgr(&nombres, &mut c.style),
        _ => {}
    }
}

/// Applique un SGR — « Select Graphic Rendition », l'apparence.
fn appliquer_sgr(nombres: &[u32], style: &mut Style) {
    // `ESC[m` sans paramètre vaut `ESC[0m` : tout revient au normal.
    if nombres.is_empty() {
        *style = Style::default();
        return;
    }
    let mut reste = nombres.iter().copied().peekable();
    while let Some(n) = reste.next() {
        match n {
            0 => *style = Style::default(),
            1 => style.gras = true,
            4 => style.souligné = true,
            7 => style.inversé = true,
            22 => style.gras = false,
            24 => style.souligné = false,
            27 => style.inversé = false,
            30..=37 => {
                style.couleur = Some((n - 30) as u8);
                style.vive = false;
            }
            90..=97 => {
                style.couleur = Some((n - 90) as u8);
                style.vive = true;
            }
            39 => style.couleur = None,
            40..=47 => style.fond = Some((n - 40) as u8),
            100..=107 => style.fond = Some((n - 100) as u8),
            49 => style.fond = None,
            // Les couleurs étendues : `38;5;n` et `38;2;r;g;b`. On lit
            // leurs paramètres pour ne pas les prendre pour des codes,
            // et l'on ramène au plus proche des huit.
            38 | 48 => {
                let sur_le_fond = n == 48;
                match reste.next() {
                    Some(5) => {
                        let index = reste.next().unwrap_or(0);
                        let (couleur, vive) = parmi_les_huit(index);
                        if sur_le_fond {
                            style.fond = Some(couleur);
                        } else {
                            style.couleur = Some(couleur);
                            style.vive = vive;
                        }
                    }
                    Some(2) => {
                        let r = reste.next().unwrap_or(0);
                        let v = reste.next().unwrap_or(0);
                        let b = reste.next().unwrap_or(0);
                        let couleur = depuis_rvb(r, v, b);
                        if sur_le_fond {
                            style.fond = Some(couleur);
                        } else {
                            style.couleur = Some(couleur);
                            style.vive = r.max(v).max(b) > 160;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// Une couleur de la palette de 256, ramenée aux huit de base.
fn parmi_les_huit(index: u32) -> (u8, bool) {
    match index {
        0..=7 => (index as u8, false),
        8..=15 => ((index - 8) as u8, true),
        // Le cube de 6×6×6, puis les vingt-quatre gris : on retombe sur
        // la teinte la plus proche en comparant ses trois composantes.
        16..=231 => {
            let n = index - 16;
            let (r, v, b) = (n / 36, (n / 6) % 6, n % 6);
            let fort = r.max(v).max(b);
            (depuis_rvb(r * 51, v * 51, b * 51), fort >= 4)
        }
        _ => (7, index >= 244),
    }
}

/// Le plus proche des huit, par ses composantes : chacune compte pour un
/// bit, comme dans le codage d'origine des couleurs de terminal.
fn depuis_rvb(r: u32, v: u32, b: u32) -> u8 {
    let seuil = 96;
    let bit = |x: u32, rang: u8| if x > seuil { 1 << rang } else { 0 };
    bit(r, 0) | bit(v, 1) | bit(b, 2)
}

/// La couleur d'un terminal, rendue dans les teintes du thème.
///
/// Les huit couleurs d'un terminal ne sont pas des valeurs : ce sont
/// des RÔLES, et chaque terminal les peint à sa façon. namsdec les
/// prend donc dans son thème — le rouge est celui de ses erreurs, le
/// vert celui de ses réussites —, ce qui fait qu'un logo de crackme
/// s'accorde au reste de la fenêtre au lieu de jurer avec.
///
/// `vive` éclaircit : c'est ce que le « gras » d'un terminal fait aux
/// couleurs, et ce que font les codes 90 à 97.
pub fn couleur(index: u8, vive: bool, theme: &namsdec_theme::Theme) -> egui::Color32 {
    let base = match index {
        0 => theme.dim,
        1 => theme.bad,
        2 => theme.good,
        3 => theme.warn,
        4 => theme.accent,
        5 => theme.directive,
        6 => theme.register,
        _ => theme.text,
    };
    if vive { éclaircir(base) } else { base }
}

/// Une teinte rapprochée du blanc, sans la délaver : c'est ce qu'un
/// terminal fait d'une couleur « vive ».
fn éclaircir(c: egui::Color32) -> egui::Color32 {
    let mêler = |x: u8| (u16::from(x) + (255 - u16::from(x)) * 2 / 5) as u8;
    egui::Color32::from_rgb(mêler(c.r()), mêler(c.g()), mêler(c.b()))
}

/// Un écran posé dans un `LayoutJob` : le texte, ses couleurs, ses
/// soulignements, prêt à peindre d'un seul bloc.
///
/// D'un seul bloc, et c'est ce qui compte : une étiquette par morceau
/// coûterait des milliers de widgets sur une sortie un peu longue, et
/// la sélection ne traverserait pas de l'un à l'autre.
pub fn mise_en_page(
    écran: &Écran,
    police: egui::FontId,
    theme: &namsdec_theme::Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for (rang, ligne) in écran.lignes.iter().enumerate() {
        if rang > 0 {
            job.append("\n", 0.0, format_de(&Style::default(), &police, theme));
        }
        for morceau in ligne {
            job.append(
                &morceau.texte,
                0.0,
                format_de(&morceau.style, &police, theme),
            );
        }
    }
    job
}

/// Le format egui d'un style de terminal.
fn format_de(
    style: &Style,
    police: &egui::FontId,
    theme: &namsdec_theme::Theme,
) -> egui::TextFormat {
    let encre = style
        .couleur
        .map(|c| couleur(c, style.vive || style.gras, theme))
        .unwrap_or(theme.text);
    let fond = style.fond.map(|c| couleur(c, false, theme));
    // L'inverse : l'encre devient le fond, et le fond l'encre. Sans
    // fond nommé, c'est celui du panneau qui sert — sinon le texte
    // inversé serait invisible.
    let (encre, fond) = if style.inversé {
        (fond.unwrap_or(theme.bg), Some(encre))
    } else {
        (encre, fond)
    };
    egui::TextFormat {
        font_id: police.clone(),
        color: encre,
        background: fond.unwrap_or(egui::Color32::TRANSPARENT),
        underline: if style.souligné {
            egui::Stroke::new(1.0, encre)
        } else {
            egui::Stroke::NONE
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texte(sortie: &str) -> String {
        écran(sortie.as_bytes()).texte()
    }

    /// Les séquences ne se lisent plus en toutes lettres.
    ///
    /// C'est le grief, et il se voyait sur deux captures : le logo de
    /// FROGMAN ouvre par « ESC[2J ESC[H ESC[36m », et l'on lisait ces
    /// trois-là avant le dessin.
    #[test]
    fn les_sequences_ne_se_lisent_plus() {
        assert_eq!(
            texte("\u{1b}[2J\u{1b}[H\u{1b}[36mFROGMAN\u{1b}[0m"),
            "FROGMAN"
        );
        assert_eq!(
            texte("\u{1b}[1;32mvert\u{1b}[m et \u{1b}[31mrouge"),
            "vert et rouge"
        );
        // Ce qu'on ne reconnaît pas disparaît aussi : un terminal ne
        // l'afficherait pas non plus.
        assert_eq!(texte("a\u{1b}[?25lb\u{1b}]0;titre\u{7}c"), "abc");
        assert_eq!(texte("a\u{1b}(Bb"), "ab");
    }

    /// La couleur arrive jusqu'aux morceaux, et le reste du texte n'en
    /// prend pas.
    #[test]
    fn la_couleur_se_pose_sur_ce_quelle_couvre() {
        let é = écran("normal\u{1b}[36mcyan\u{1b}[0mnormal".as_bytes());
        assert_eq!(é.lignes.len(), 1);
        let ligne = &é.lignes[0];
        assert_eq!(ligne.len(), 3, "trois morceaux : {ligne:?}");
        assert_eq!(ligne[0].style.couleur, None);
        assert_eq!(ligne[1].texte, "cyan");
        assert_eq!(ligne[1].style.couleur, Some(6), "le cyan est le 6");
        assert_eq!(
            ligne[2].style.couleur, None,
            "le reset rend l'encre du thème"
        );

        // Le gras d'un terminal éclaircit : `1;31` est un rouge vif.
        let é = écran("\u{1b}[1;31mvif".as_bytes());
        assert!(é.lignes[0][0].style.gras);
        assert_eq!(é.lignes[0][0].style.couleur, Some(1));
        // Et `91` dit la même chose en un seul code.
        let é = écran("\u{1b}[91mvif".as_bytes());
        assert!(é.lignes[0][0].style.vive);
        assert_eq!(é.lignes[0][0].style.couleur, Some(1));

        // Une même couleur de bout en bout ne fait qu'UN morceau : une
        // ligne de quatre-vingts caractères n'en vaut pas quatre-vingts.
        let é = écran("\u{1b}[32mabcdefgh".as_bytes());
        assert_eq!(é.lignes[0].len(), 1);
    }

    /// Effacer l'écran efface ce qui précède, et rien d'autre.
    #[test]
    fn effacer_lecran_efface_ce_qui_precede() {
        let é = écran("vieux\nencore\n\u{1b}[2Jneuf".as_bytes());
        assert_eq!(é.texte(), "neuf");
        assert_eq!(é.hauteur(), 1);
    }

    /// Le retour chariot recouvre la ligne : c'est ainsi qu'une barre de
    /// progression s'anime, et sans lui on en lirait cinquante.
    #[test]
    fn le_retour_chariot_recouvre_la_ligne() {
        assert_eq!(texte("10%\r20%\r100%"), "100%");
        assert_eq!(texte("abcdef\rXY"), "XYcdef");
        // `\r\n` reste un seul saut de ligne.
        assert_eq!(texte("une\r\ndeux"), "une\ndeux");
        // Effacer la fin de ligne, après un retour : ce qui dépassait
        // s'en va.
        assert_eq!(texte("abcdef\rXY\u{1b}[K"), "XY");
    }

    /// Le curseur se pose où on le lui dit, dans la ligne.
    #[test]
    fn le_curseur_se_pose_en_colonne() {
        // `ESC[10G` : dixième colonne. Le trou se comble d'espaces.
        assert_eq!(texte("ab\u{1b}[10Gfin"), "ab       fin");
        // `ESC[3C` : trois à droite ; `ESC[2D` : deux à gauche.
        assert_eq!(texte("ab\u{1b}[3Ccd"), "ab   cd");
        assert_eq!(texte("abcd\u{1b}[2DXY"), "abXY");
    }

    /// La tabulation s'arrête au prochain multiple de huit.
    #[test]
    fn la_tabulation_va_au_multiple_de_huit() {
        assert_eq!(texte("a\tb"), "a       b");
        assert_eq!(texte("abcdefgh\tx"), "abcdefgh        x");
    }

    /// L'invite sans saut de ligne final compte quand même : c'est elle
    /// qui dit ce qu'on attend de nous.
    #[test]
    fn linvite_sans_saut_final_est_une_ligne() {
        let é = écran(b"Enter the password: ");
        assert_eq!(é.hauteur(), 1);
        assert_eq!(é.texte(), "Enter the password: ");
    }

    /// Ce qui n'est pas de l'UTF-8 ne fait pas échouer la lecture.
    #[test]
    fn des_octets_quelconques_passent() {
        let é = écran(&[0xff, b'a', 0xfe, b'b']);
        assert!(é.texte().contains('a') && é.texte().contains('b'));
    }

    /// Les couleurs étendues se ramènent aux huit, et surtout leurs
    /// paramètres ne se lisent pas comme des codes.
    #[test]
    fn les_couleurs_etendues_ne_debordent_pas() {
        // `38;5;196` est un rouge vif de la palette de 256. Sans la
        // lecture de ses paramètres, le « 5 » passerait pour un
        // clignotement et le « 196 » pour rien.
        let é = écran("\u{1b}[38;5;196mrouge".as_bytes());
        assert_eq!(é.lignes[0][0].style.couleur, Some(1));
        let é = écran("\u{1b}[38;2;0;0;255mbleu".as_bytes());
        assert_eq!(é.lignes[0][0].style.couleur, Some(4));
        // Et le texte reste le texte.
        assert_eq!(texte("\u{1b}[38;5;21mx"), "x");
    }
}
