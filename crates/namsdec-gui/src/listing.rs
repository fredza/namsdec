//! Le rendu du listing désassemblé : la bande de chaleur, la ligne sous
//! le curseur, les boutons-liens d'adresse, les étiquettes de la table de
//! liaison, l'annotation d'une ligne et la cible d'un saut, le nom d'une
//! adresse.
//!
//! Des fonctions de dessin et de mise en forme, sans état propre. Sorties
//! de `main.rs` pour l'alléger.

use super::*;

/// Les valeurs que montre la légende, de la plus froide à la plus
/// chaude.
///
/// Réparties sur l'échelle logarithmique de la barre elle-même : entre
/// un passage et mille, les repères intéressants sont 1, 6, 32, 178,
/// 1000 — et non 200, 400, 600, 800.
pub(crate) fn heat_legend_steps(max: u64) -> Vec<u64> {
    let max = max.max(1);
    if max <= 5 {
        return (1..=max).collect();
    }
    let échelle = (max as f32).ln();
    (0..5)
        .map(|i| {
            let t = i as f32 / 4.0;
            (échelle * t).exp().round().max(1.0) as u64
        })
        // Deux repères qui tombent sur la même valeur ne se distinguent
        // pas : on n'en garde qu'un.
        .fold(Vec::new(), |mut out, valeur| {
            if out.last() != Some(&valeur) {
                out.push(valeur);
            }
            out
        })
}

/// La largeur de la bande de chaleur, à l'extrême gauche de la
/// gouttière. Assez pour se voir, assez peu pour ne pas se disputer la
/// place avec le point d'arrêt.
pub(crate) const HEAT_WIDTH: f32 = 4.0;

/// La couleur d'une ligne selon ce qu'elle a coûté.
///
/// L'échelle est logarithmique : entre une ligne parcourue une fois et
/// une autre parcourue mille fois, ce qui compte est l'ordre de
/// grandeur, pas la différence. Du vert d'une ligne traversée une fois
/// au rouge de celle où le programme s'attarde, en passant par le jaune
/// et l'orange : c'est la convention de toutes les cartes de chaleur,
/// et elle se lit sans légende — celle sous l'éditeur donne les
/// valeurs, l'onglet Profil le classement.
pub(crate) fn heat_colour(theme: &'static Theme, count: u64, max: u64) -> egui::Color32 {
    // L'échelle couvre l'intervalle observé : une ligne parcourue une
    // seule fois est au vert pur, la plus chaude au rouge pur. Compter
    // depuis zéro tassait tout le programme dans le milieu de la rampe.
    let échelle = (max.max(1) as f32).ln();
    let part = if échelle > 0.0 {
        ((count.max(1) as f32).ln() / échelle).clamp(0.0, 1.0)
    } else {
        0.0
    };
    // Vert, jaune, orange, rouge : la rampe qu'on lit sans légende,
    // celle des cartes de chaleur et des jauges de température. Le vert
    // dit « passé une fois, rien à voir ici », le rouge « c'est là que
    // le programme passe son temps ».
    //
    // Le jaune et l'orange sont posés en dur et non pris au thème : ce
    // sont des repères de lecture, pas des couleurs d'interface, et un
    // thème qui choisirait un « avertissement » violet casserait la
    // progression sans qu'on comprenne pourquoi.
    const JAUNE: egui::Color32 = egui::Color32::from_rgb(0xe3, 0xc5, 0x4a);
    const ORANGE: egui::Color32 = egui::Color32::from_rgb(0xe2, 0x88, 0x3c);
    let arrêts = [theme.good, JAUNE, ORANGE, theme.bad];
    // Trois segments entre quatre arrêts.
    let segments = (arrêts.len() - 1) as f32;
    let position = part * segments;
    let index = (position.floor() as usize).min(arrêts.len() - 2);
    arrêts[index].lerp_to_gamma(arrêts[index + 1], position - index as f32)
}

/// Les rangées du galley qui tombent dans `vue`, par dichotomie.
///
/// `origine` est le coin haut-gauche du galley à l'écran : les rangées
/// se situent par rapport à lui.
///
/// Les rangées sont posées de haut en bas, donc triées par ordonnée :
/// on trouve la tranche visible par recherche dichotomique au lieu de
/// parcourir toutes les autres. La différence n'est pas théorique —
/// mesurée le 14 octobre 2026 sur un listing d'un million de lignes, le
/// seul balayage de la gouttière coûtait NEUF millisecondes par image
/// pour trouver les soixante rangées à l'écran. Le temps d'une image
/// entière à soixante hertz, dépensé à ne rien peindre.
pub(crate) fn rangées_visibles(
    galley: &egui::Galley,
    origine: egui::Vec2,
    vue: egui::Rect,
) -> std::ops::Range<usize> {
    let haut = vue.top() - origine.y;
    let bas = vue.bottom() - origine.y;
    let début = galley
        .rows
        .partition_point(|row| row.pos.y + row.row.size.y < haut);
    let fin = galley.rows.partition_point(|row| row.pos.y <= bas);
    début..fin.max(début)
}

/// Un bouton de lien : une marque dessinée, puis son nom.
///
/// Fait à la main plutôt qu'avec `Button::image_and_text` : ces marques
/// sont peintes, pas chargées, et elles doivent prendre la couleur du
/// texte du bouton — celle qui change au survol.
pub(crate) fn link_button_ui(
    ui: &mut egui::Ui,
    theme: &'static Theme,
    label: &str,
    dessin: fn(&egui::Painter, egui::Rect, egui::Color32),
) -> egui::Response {
    let police = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), police, theme.text);
    const MARQUE: f32 = 16.0;
    const AIR: f32 = 8.0;
    let taille = egui::vec2(
        MARQUE + AIR + galley.size().x + 24.0,
        galley.size().y.max(MARQUE) + 12.0,
    );
    let (rect, réponse) = ui.allocate_exact_size(taille, egui::Sense::click());
    let visuals = ui.style().interact(&réponse);
    ui.painter().rect_filled(rect, 6.0, visuals.weak_bg_fill);
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, visuals.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let marque = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 12.0 + MARQUE * 0.5, rect.center().y),
        egui::vec2(MARQUE, MARQUE),
    );
    dessin(ui.painter(), marque, visuals.text_color());
    ui.painter().galley(
        egui::pos2(
            marque.right() + AIR,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        visuals.text_color(),
    );
    réponse
}

/// Les entrées de la table de liaison, en étiquettes du listing.
///
/// Un `call 0x4010a0` ne dit rien ; le même appel, quand l'adresse porte
/// le nom `malloc@plt`, dit tout. Les entrées deviennent donc des
/// symboles comme les autres : le listing les écrit, le bandeau les
/// nomme, et « Aller à une étiquette » les propose.
///
/// `place` dit où l'adresse d'une entrée tombe dans ce qui est
/// désassemblé : telle quelle pour une image chargée à ses adresses
/// virtuelles, décalée quand c'est le fichier brut qu'on lit.
pub(crate) fn plt_labels(
    file: &[u8],
    existants: &[namsdec_asm::elf::Symbol],
    place: impl Fn(u64) -> Option<u64>,
) -> Vec<namsdec_asm::elf::Symbol> {
    namsdec_re::imports::scan(file)
        .into_iter()
        .filter_map(|import| {
            let addr = place(import.plt)?;
            // Un nom donné par la table des symboles prime : il vient du
            // binaire, celui-ci est déduit.
            if existants.iter().any(|symbol| symbol.addr == addr) {
                return None;
            }
            Some(namsdec_asm::elf::Symbol {
                name: format!("{}@plt", import.name),
                addr,
                size: 0,
            })
        })
        .collect()
}

/// Le listing d'un binaire ouvert : les instructions, avec les étiquettes
/// que sa table des symboles porte encore.
///
/// Un désassemblage brut est une colonne d'instructions sans début ni
/// fin : on ne voit pas où une fonction commence, et un `call 401030h`
/// ne dit pas *qui* est appelé. Les étiquettes rendent au binaire la
/// structure que le source avait — c'est la première chose qu'on
/// reconstitue à la main en lisant du code qu'on n'a pas écrit.
///
/// Rend le texte et la table ligne → adresse : les lignes d'étiquette
/// n'y figurent pas, elles ne portent aucune instruction.
pub(crate) fn annotated_listing(
    disasm: &[DecodedInstruction],
    symbols: &[namsdec_asm::elf::Symbol],
) -> (String, BTreeMap<u32, u64>) {
    // La place à peu près juste, demandée d'un coup : une quarantaine
    // d'octets par instruction. Un million d'instructions, c'est sinon
    // une vingtaine de recopies du listing entier pendant qu'il grandit.
    let mut listing = String::with_capacity(disasm.len() * 40);
    let mut line_to_addr = BTreeMap::new();
    let mut line = 1u32;
    // Les deux listes sont triées par adresse : on avance dans les
    // symboles AVEC les instructions, au lieu de relire la table pour
    // chacune. Le filtre d'avant était quadratique — dix mille symboles
    // devant un million d'instructions font dix milliards de
    // comparaisons, et c'était la moitié du temps d'ouverture d'un gros
    // binaire.
    let mut prochain = 0usize;
    for instruction in disasm {
        while symbols
            .get(prochain)
            .is_some_and(|s| s.addr < instruction.address)
        {
            prochain += 1;
        }
        // Une étiquette qui commence ici : elle prend sa ligne, précédée
        // d'une ligne vide — sauf tout en haut, où elle ferait un trou.
        while let Some(symbol) = symbols
            .get(prochain)
            .filter(|s| s.addr == instruction.address)
        {
            if line > 1 {
                listing.push('\n');
                line += 1;
            }
            listing.push_str(&symbol.name);
            listing.push_str(":\n");
            line += 1;
            prochain += 1;
        }
        line_to_addr.insert(line, instruction.address);
        listing.push_str(&instruction.text);
        // La cible d'un saut ou d'un appel, nommée en commentaire : le
        // nombre reste — c'est lui qui est dans les octets — mais on
        // sait enfin où il mène.
        if let Some(target) = branch_target(&instruction.text)
            && let Some(label) = symbol_at(symbols, target)
        {
            listing.push_str("    ; -> ");
            listing.push_str(&label);
        }
        listing.push('\n');
        line += 1;
    }
    (listing, line_to_addr)
}

/// L'adresse que vise un saut ou un appel, lue dans le texte que le
/// désassembleur a écrit (`call 401030h`, `jne 400012h`).
///
/// Par le texte plutôt qu'en redécodant : c'est ce texte-là qui est
/// affiché, et l'étiquette doit nommer ce qu'il montre.
pub(crate) fn branch_target(text: &str) -> Option<u64> {
    let (mnemonic, operand) = text.split_once(' ')?;
    if !(mnemonic.starts_with('j') || mnemonic == "call" || mnemonic.starts_with("loop")) {
        return None;
    }
    let operand = operand.trim();
    let hex = operand.strip_suffix('h')?;
    u64::from_str_radix(hex, 16).ok()
}

/// Le nom d'une adresse : celui qu'on a posé, sinon celui du binaire.
///
/// Le nom posé à la main l'emporte, et c'est voulu : quelqu'un qui
/// renomme `sub_1d491` en `vérifie_mdp` veut voir le second — c'est même
/// toute la raison de l'avoir écrit. Retiré, on retombe sur le symbole,
/// et non sur rien.
///
/// Fonction libre plutôt que méthode : elle sert dans la barre latérale,
/// dans le graphe, dans le listing, à des endroits qui tiennent déjà un
/// emprunt sur l'application. En faire une méthode obligeait à la
/// contourner là où elle servait le plus.
pub(crate) fn named_at(
    notes: &notes::Notes,
    symbols: &[namsdec_asm::elf::Symbol],
    addr: u64,
) -> Option<String> {
    if let Some(nom) = notes.name(addr) {
        return Some(nom.to_owned());
    }
    symbol_at(symbols, addr)
}

/// Les symboles du binaire, avec par-dessus les noms qu'on a donnés.
///
/// Le listing est écrit **une fois**, au chargement, à partir des
/// symboles de l'ELF : rebaptiser une fonction ne s'y voyait donc pas,
/// alors que le nom apparaissait partout ailleurs. On ne touche pas aux
/// symboles d'origine pour autant — ils restent ce que le fichier dit,
/// et c'est ce qui permet de se dédire : un nom retiré rend le symbole,
/// non le vide.
///
/// Un nom posé sur une adresse qui n'en portait aucune devient une
/// étiquette à elle seule : c'est le cas ordinaire d'un binaire
/// dépouillé, où l'on nomme `sub_1d491`.
pub(crate) fn symbols_with_names(
    symbols: &[namsdec_asm::elf::Symbol],
    notes: &notes::Notes,
) -> Vec<namsdec_asm::elf::Symbol> {
    let mut sortie: Vec<namsdec_asm::elf::Symbol> = symbols.to_vec();
    for (addr, nom) in &notes.names {
        match sortie.iter_mut().find(|s| s.addr == *addr) {
            Some(symbole) => symbole.name = nom.clone(),
            None => sortie.push(namsdec_asm::elf::Symbol {
                name: nom.clone(),
                addr: *addr,
                size: 0,
            }),
        }
    }
    sortie.sort_by_key(|symbole| symbole.addr);
    sortie
}

/// L'étiquette qui nomme `addr`, telle qu'on l'a écrite dans le source :
/// `tampon`, ou `tampon+2` deux octets plus loin.
///
/// Les symboles sont triés par adresse : on prend le dernier qui commence
/// avant, et on ne le propose que s'il est assez proche pour dire encore
/// quelque chose — sa taille déclarée quand nasm en donne une, sinon la
/// distance jusqu'au symbole suivant.
pub(crate) fn symbol_at(symbols: &[namsdec_asm::elf::Symbol], addr: u64) -> Option<String> {
    let index = symbols.partition_point(|s| s.addr <= addr).checked_sub(1)?;
    let symbol = &symbols[index];
    let offset = addr - symbol.addr;
    let reach = if symbol.size > 0 {
        symbol.size
    } else {
        symbols
            .get(index + 1)
            .map(|next| next.addr - symbol.addr)
            .unwrap_or(1)
    };
    if offset >= reach {
        return None;
    }
    if offset == 0 {
        Some(symbol.name.clone())
    } else {
        Some(format!("{}+{offset}", symbol.name))
    }
}
