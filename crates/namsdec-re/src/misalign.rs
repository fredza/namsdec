//! Quand le flot lit d'autres octets que la lecture ligne à ligne.
//!
//! Un désassemblage ordinaire lit **linéairement** : il décode une
//! instruction, avance de sa longueur, recommence. C'est ce que fait
//! namsdec, et c'est le bon choix pour un listing — il faut bien poser
//! une ligne par adresse, et une lecture qui suivrait le flot laisserait
//! des trous partout où elle ne sait pas aller.
//!
//! Mais le processeur, lui, ne lit pas linéairement : il va **où les
//! sauts l'envoient**. Les deux lectures coïncident dans un programme
//! ordinaire, parce qu'un compilateur fait commencer ses sauts sur des
//! frontières d'instruction. Elles divergent dès qu'on les fait diverger
//! exprès.
//!
//! ## La figure, et pourquoi elle marche
//!
//! ```text
//!     eb 01        jmp $+3       ← saute UN octet plus loin que la suite
//!     e8           db 0xE8       ← l'octet-piège : un opcode de `call`
//!     31 c0        xor eax,eax   ← le vrai code commence ici
//! ```
//!
//! La lecture linéaire tombe sur `e8` et le prend pour un `call` : cet
//! opcode avale quatre octets d'adresse, donc elle engloutit `31 c0 48
//! ff`, puis repart de travers et finit en `(bad)`. Le processeur, lui,
//! saute à l'octet d'après le piège et lit `xor eax,eax`. **Un seul
//! octet** a suffi pour que tout un désassemblage mente.
//!
//! C'est l'anti-désassemblage classique, et on le rencontre dans ce que
//! namsdec est fait pour lire. Le montrer sert deux fois : on comprend
//! pourquoi le listing paraissait incohérent, et l'on apprend du même
//! coup que *le découpage en instructions n'est pas une propriété des
//! octets* — c'est une propriété du point où l'on commence à lire.
//!
//! ## Ce que ce module fait, et ne fait pas
//!
//! Il **relève** les endroits où un saut atterrit au milieu d'une
//! instruction de la lecture linéaire, et donne les deux lectures côte à
//! côte. Il ne corrige pas le listing : refaire tout le désassemblage en
//! suivant le flot est un autre chantier, et un listing troué serait
//! pire. Nommer l'endroit et montrer les deux lectures suffit à ne plus
//! se faire avoir.
//!
//! Et il lit le listing tel qu'il est, sans le refaire : c'est la
//! condition pour que ce qu'il montre soit bien ce que l'utilisateur a
//! sous les yeux. La conséquence est qu'un saut que le listing n'a pas
//! décodé — parce qu'il est entré dans le code sur une frontière
//! quelconque, le listing partant du premier octet du *fichier* — ne
//! peut pas être relevé. Ce n'est pas une limite du relevé : c'est la
//! même figure, un cran plus tôt.

use namsdec_disasm::{DecodedInstruction, FlowControl};

/// Combien d'instructions on montre de la lecture du flot.
///
/// Assez pour voir que c'est du vrai code et non du bruit ; pas assez
/// pour refaire un listing dans un panneau qui n'est pas celui-là.
const SUITE: usize = 4;

/// À quelle distance du saut la figure doit se trouver, en octets.
///
/// Ce seuil est la différence entre un module utilisable et un module
/// qu'on n'ouvre plus. Mesuré sur `/bin/ls` : sans lui, trois
/// « désalignements » sont relevés, et aucun n'en est un. Ils viennent
/// tous du même endroit — le listing part du **premier octet du
/// fichier**, en-têtes compris, et les en-têtes décodés comme du code
/// rendent des instructions imaginaires (`add [rax],al`, qui est le
/// nom x86 de deux octets nuls) dont les « sauts » pointent n'importe
/// où. Un de ces faux sauts tombe forcément au milieu d'un autre faux
/// octet.
///
/// Or la figure vraie est **courte par nécessité** : elle cache un ou
/// deux octets-pièges derrière un saut qui les enjambe. Les trois faux
/// de `/bin/ls` sautaient de 71, 25 et 61 octets ; celui qu'on cherche
/// saute de un.
///
/// Ce que ce seuil coûte : un désalignement posé à cent octets de son
/// saut ne serait pas vu. C'est assumé — relever trois figures fausses
/// sur chaque binaire du monde coûte plus cher que d'en manquer une
/// rare, parce qu'un panneau qui crie au loup n'est plus lu du tout.
const PORTÉE: u64 = 16;

/// Les zones du listing qui portent du code exécutable.
///
/// Rendues pour les **deux** façons dont une adresse de section peut se
/// retrouver dans un listing, parce que namsdec emploie les deux :
///
/// * devant un ELF ouvert, l'application pose chaque section exécutable
///   à son **adresse virtuelle** — c'est ce que fait `executable_parts`,
///   et c'est pourquoi le listing d'un binaire montre du code dès sa
///   première ligne au lieu de commencer par l'en-tête ;
/// * un tampon désassemblé d'un bloc, lui, part de `base`, et une
///   section s'y trouve alors à `base + son décalage dans le fichier`.
///
/// Les deux intervalles sont rendus. Un intervalle de trop ne fait que
/// laisser passer ce que les autres filtres écarteront ; un intervalle
/// manquant, lui, aveugle le relevé — et c'est ce qui est arrivé : le
/// panneau ne montrait rien sur un binaire ouvert, parce qu'il cherchait
/// le code là où il n'était pas.
///
/// Rend une liste vide quand le fichier n'est pas un ELF lisible : cela
/// veut dire « on ne sait pas », et [`scan_in`] regarde alors partout.
pub fn code_zones(binary: &[u8], base: u64) -> Vec<std::ops::Range<u64>> {
    let Some(elf) = crate::elf_scan::scan(binary) else {
        return Vec::new();
    };
    let mut zones: Vec<std::ops::Range<u64>> = Vec::new();
    /// Ajoute un intervalle, s'il tient et s'il n'y est pas déjà.
    ///
    /// Fonction et non fermeture : une fermeture qui emprunte `zones`
    /// pour écrire ne peut pas le relire pour y chercher un doublon.
    fn ajoute(zones: &mut Vec<std::ops::Range<u64>>, début: Option<u64>, taille: u64) {
        if let Some(début) = début
            && let Some(fin) = début.checked_add(taille)
            && !zones.contains(&(début..fin))
        {
            zones.push(début..fin);
        }
    }
    for section in &elf.sections {
        if !section.execute || section.nobits || section.size == 0 {
            continue;
        }
        ajoute(
            &mut zones,
            Some(section.addr).filter(|a| *a != 0),
            section.size,
        );
        ajoute(&mut zones, base.checked_add(section.offset), section.size);
    }
    // Un binaire dépouillé n'a plus de table de sections : ses segments
    // restent, et c'est d'eux que l'application tire alors son listing.
    if zones.is_empty() {
        for segment in &elf.segments {
            if !segment.execute || segment.filesz == 0 {
                continue;
            }
            ajoute(
                &mut zones,
                Some(segment.vaddr).filter(|a| *a != 0),
                segment.filesz,
            );
            ajoute(&mut zones, base.checked_add(segment.offset), segment.filesz);
        }
    }
    zones
}

/// Une instruction telle que le flot la lit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lue {
    pub addr: u64,
    pub text: String,
}

/// Un endroit où les deux lectures divergent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Misalignment {
    /// Le saut qui envoie à côté.
    pub from: u64,
    /// Où il envoie : une adresse qui n'est le début d'aucune
    /// instruction de la lecture linéaire.
    pub target: u64,
    /// L'instruction linéaire qui contient cette adresse, et le texte
    /// qu'elle affiche — celui qui ment.
    pub inside: u64,
    pub linear: String,
    /// De combien d'octets on est à l'intérieur. Un, le plus souvent :
    /// c'est tout ce qu'il faut.
    pub offset: u64,
    /// Ce que le flot lit vraiment à partir de la cible.
    pub flow: Vec<Lue>,
}

/// Les endroits où un saut atterrit au milieu d'une instruction lue
/// linéairement.
///
/// `disasm` est la lecture linéaire, telle que le listing la montre.
/// Rendus par adresse de cible croissante, sans doublon : deux sauts qui
/// mènent au même octet désalignent la même chose, et la montrer deux
/// fois n'apprend rien de plus.
///
/// Rend une liste vide pour la quasi-totalité des programmes, et c'est
/// la réponse attendue : un compilateur ne produit pas cette figure.
pub fn scan(disasm: &[DecodedInstruction]) -> Vec<Misalignment> {
    scan_in(disasm, &[])
}

/// Le même relevé, borné aux zones qui portent vraiment du code.
///
/// C'est la forme qu'emploient les panneaux, et elle change tout sur un
/// vrai fichier. Le listing de namsdec part du **premier octet du
/// fichier** : les en-têtes, les tables et surtout les chaînes y sont
/// décodés comme du code, et une chaîne de format comme `%d%% - ` rend
/// des « instructions » parfaitement valides dont les « sauts » tombent
/// au hasard. Mesuré sur `/bin/ls` : deux figures relevées, toutes deux
/// dans `.rodata`, toutes deux faites de texte.
///
/// `zones` sont des intervalles d'adresses **du listing** — c'est à
/// l'appelant de convertir, lui seul sachant où le fichier est posé. Une
/// liste vide veut dire « on ne sait pas » et non « nulle part » : on
/// regarde alors partout, ce qui est le mieux qu'on puisse faire devant
/// un tampon brut.
pub fn scan_in(disasm: &[DecodedInstruction], zones: &[std::ops::Range<u64>]) -> Vec<Misalignment> {
    if disasm.is_empty() {
        return Vec::new();
    }
    let dans_le_code = |addr: u64| zones.is_empty() || zones.iter().any(|z| z.contains(&addr));
    // Où commence chaque instruction de la lecture linéaire, et jusqu'où
    // elle va. Trié par construction : le désassemblage l'est.
    let débuts: Vec<(u64, usize)> = disasm.iter().map(|i| (i.address, i.bytes.len())).collect();

    let mut trouvés: Vec<Misalignment> = Vec::new();
    for (index, instruction) in disasm.iter().enumerate() {
        let Some(décodée) = namsdec_disasm::decode_one(&instruction.bytes, instruction.address)
        else {
            continue;
        };
        // Les sauts et les appels à cible écrite en clair. Un saut
        // calculé ne dit pas où il va, et deviner mentirait.
        if !matches!(
            décodée.flow_control(),
            FlowControl::UnconditionalBranch | FlowControl::ConditionalBranch | FlowControl::Call
        ) {
            continue;
        }
        if !matches!(
            décodée.op0_kind(),
            namsdec_disasm::OpKind::NearBranch16
                | namsdec_disasm::OpKind::NearBranch32
                | namsdec_disasm::OpKind::NearBranch64
        ) {
            continue;
        }
        let cible = décodée.near_branch_target();
        if !dans_le_code(instruction.address) || !dans_le_code(cible) {
            continue;
        }

        // La cible est-elle le début d'une instruction ? Alors tout va
        // bien, et c'est le cas de presque tous les sauts du monde.
        let Err(rang) = débuts.binary_search_by_key(&cible, |(addr, _)| *addr) else {
            continue;
        };
        // Sinon : tombe-t-elle à l'intérieur de celle qui précède ?
        // Hors du désassemblage — une cible dans une autre section, un
        // saut vers une bibliothèque — n'est pas un désalignement, c'est
        // simplement ailleurs.
        if rang == 0 {
            continue;
        }
        let (début, longueur) = débuts[rang - 1];
        let fin = début.saturating_add(longueur as u64);
        if cible >= fin {
            continue;
        }
        // Du texte lu comme du code n'est pas du code.
        //
        // Le garde-fou de secours, pour quand on ne sait pas où sont les
        // zones de code — un tampon brut, un fichier qui n'est pas un
        // ELF. Une « instruction » faite entièrement de caractères
        // imprimables est presque toujours un morceau de chaîne : c'est
        // ainsi que `%d%% - ` devenait `sub eax,25206425h`, avec un faux
        // saut à la clé.
        if disasm[rang - 1]
            .bytes
            .iter()
            .all(|b| (0x20..0x7f).contains(b) || matches!(b, b'\n' | b'\r' | b'\t'))
        {
            continue;
        }
        if trouvés.iter().any(|déjà| déjà.target == cible) {
            continue;
        }
        // La figure est courte : le saut enjambe les octets-pièges, qui
        // se comptent sur une main. Voir [`PORTÉE`] pour ce que ce
        // seuil écarte, et ce qu'il coûte.
        if cible.abs_diff(instruction.address) > PORTÉE {
            continue;
        }

        // Et ce que le flot y lit doit être du code. Le désassembleur
        // écrit `(bad)` quand les octets ne forment aucune instruction :
        // une « cible » qui mène à cela n'est pas du code caché, c'est
        // du hasard. On relève la figure seulement si les deux lectures
        // sont toutes deux plausibles — c'est leur DÉSACCORD qui est
        // l'information, et deux lectures dont l'une est du bruit ne
        // sont pas en désaccord.
        let flow = lit_depuis(disasm, rang - 1, cible);
        if flow.is_empty() || flow.iter().any(|lue| lue.text.contains("(bad)")) {
            continue;
        }

        trouvés.push(Misalignment {
            from: instruction.address,
            target: cible,
            inside: début,
            linear: disasm[rang - 1].text.clone(),
            offset: cible - début,
            flow,
        });
        let _ = index;
    }
    trouvés.sort_by_key(|m| m.target);
    trouvés
}

/// Ce que le flot lit à partir de `cible`, en recollant les octets des
/// instructions linéaires qui suivent.
///
/// Recoller plutôt que relire le fichier : le module ne reçoit que le
/// désassemblage, et c'est voulu — il doit pouvoir travailler sur un
/// tampon brut aussi bien que sur un ELF.
fn lit_depuis(disasm: &[DecodedInstruction], à_partir_de: usize, cible: u64) -> Vec<Lue> {
    let mut octets: Vec<u8> = Vec::new();
    // De quoi décoder quelques instructions : la plus longue en x86-64
    // fait quinze octets.
    for instruction in disasm.iter().skip(à_partir_de).take(SUITE + 4) {
        octets.extend_from_slice(&instruction.bytes);
    }
    let base = disasm[à_partir_de].address;
    let Ok(saut) = usize::try_from(cible.saturating_sub(base)) else {
        return Vec::new();
    };
    if saut >= octets.len() {
        return Vec::new();
    }
    namsdec_disasm::disassemble(&octets[saut..], cible)
        .into_iter()
        .take(SUITE)
        .map(|i| Lue {
            addr: i.address,
            text: i.text,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La figure classique : un octet-piège, et tout un désassemblage qui
    /// ment.
    ///
    /// C'est le cas d'école de l'anti-désassemblage, et celui qu'on
    /// rencontre. Mesuré avant d'écrire une ligne de ce module : la
    /// lecture linéaire rend `call 0FFFFFFFFFF88C038h` puis `(bad)`,
    /// quand le processeur lit `xor eax,eax ; inc rax ; ret`.
    #[test]
    fn the_classic_trap_byte_is_caught_and_both_readings_given() {
        let octets: &[u8] = &[
            0xEB, 0x01, // jmp $+3 — un octet plus loin que la suite
            0xE8, // l'octet-piège : un opcode de `call`
            0x31, 0xC0, // xor eax,eax   ← ce que le flot lit
            0x48, 0xFF, 0xC0, // inc rax
            0xC3, // ret
        ];
        let linéaire = namsdec_disasm::disassemble(octets, 0x400000);
        // La prémisse du contrôle : la lecture linéaire se fait bien
        // avoir. Si elle cessait de se faire avoir, ce test ne
        // prouverait plus rien, et il doit le dire.
        assert!(
            linéaire[1].text.starts_with("call"),
            "la lecture linéaire prend le piège pour un call : {:?}",
            linéaire[1].text
        );

        let trouvés = scan(&linéaire);
        assert_eq!(trouvés.len(), 1, "{trouvés:?}");
        let m = &trouvés[0];
        assert_eq!(m.from, 0x400000, "le saut");
        assert_eq!(m.target, 0x400003, "où il envoie");
        assert_eq!(m.inside, 0x400002, "l'instruction qui l'avale");
        assert_eq!(m.offset, 1, "un seul octet à l'intérieur");
        assert!(m.linear.starts_with("call"), "{}", m.linear);

        // Et les deux lectures sont là, lisibles.
        let flot: Vec<&str> = m.flow.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(flot, vec!["xor eax,eax", "inc rax", "ret"], "{flot:?}");
        assert_eq!(m.flow[0].addr, 0x400003);

        // Et la figure se voit toujours quand on borne aux zones de
        // code : le filtre qui écarte le bruit de `.rodata` ne doit pas
        // écarter ce qu'on cherche.
        assert_eq!(
            scan_in(&linéaire, std::slice::from_ref(&(0x400000..0x400100))),
            trouvés,
            "bornée au code, la figure est la même"
        );
        // Bornée ailleurs, en revanche, elle n'est plus de notre
        // ressort : ce n'est pas du code qu'on regarde.
        assert!(scan_in(&linéaire, std::slice::from_ref(&(0x500000..0x500100))).is_empty());
    }

    /// Un programme ordinaire ne relève rien.
    ///
    /// C'est la moitié qui décide de l'utilité du module : une figure
    /// signalée dans tous les binaires du monde ne signale rien.
    #[test]
    fn an_ordinary_program_reports_nothing() {
        // mov eax,0 / mov ecx,5 / add rax,rcx / dec rcx / jnz / hlt
        let octets: &[u8] = &[
            0xB8, 0x00, 0x00, 0x00, 0x00, 0xB9, 0x05, 0x00, 0x00, 0x00, 0x48, 0x01, 0xC8, 0x48,
            0xFF, 0xC9, 0x75, 0xF8, 0xF4,
        ];
        let linéaire = namsdec_disasm::disassemble(octets, 0x400000);
        assert!(scan(&linéaire).is_empty(), "{:?}", scan(&linéaire));
        assert!(scan(&[]).is_empty());

        // Et sur de vrais binaires du système : un compilateur ne
        // produit pas cette figure, et un panneau qui l'annoncerait sur
        // `/bin/ls` ne serait plus jamais lu.
        //
        // Appelé comme les panneaux l'appellent : avec les zones de
        // code. Sans elles, le relevé trouvait deux figures dans
        // `.rodata` — des chaînes de format décodées comme des
        // instructions, avec des « sauts » qui tombent au hasard.
        for chemin in ["/bin/true", "/bin/ls"] {
            let Ok(octets) = std::fs::read(chemin) else {
                continue;
            };
            let base = 0x400000;
            let linéaire = namsdec_disasm::disassemble(&octets[..octets.len().min(0x20000)], base);
            let zones = code_zones(&octets, base);
            assert!(!zones.is_empty(), "{chemin} a des sections exécutables");
            let trouvés = scan_in(&linéaire, &zones);
            assert!(
                trouvés.is_empty(),
                "{chemin} ne désaligne rien : {:?}",
                &trouvés[..trouvés.len().min(3)]
            );
        }
    }

    /// Un saut qui sort du désassemblage n'est pas un désalignement.
    ///
    /// Une cible dans une autre section, un appel vers une bibliothèque :
    /// elle n'est le début d'aucune instruction d'ici, et ce n'est pas
    /// pour autant qu'on lit de travers. Confondre les deux remplirait
    /// le panneau de bruit sur le premier binaire venu.
    #[test]
    fn a_target_outside_the_listing_is_not_a_misalignment() {
        // jmp très loin en avant, puis ret.
        let octets: &[u8] = &[0xE9, 0x00, 0x10, 0x00, 0x00, 0xC3];
        let linéaire = namsdec_disasm::disassemble(octets, 0x400000);
        assert!(scan(&linéaire).is_empty(), "{:?}", scan(&linéaire));

        // Et en arrière, avant le début : même raison.
        let octets: &[u8] = &[0xE9, 0x00, 0xF0, 0xFF, 0xFF, 0xC3];
        let linéaire = namsdec_disasm::disassemble(octets, 0x400000);
        assert!(scan(&linéaire).is_empty(), "{:?}", scan(&linéaire));
    }

    /// Deux sauts vers le même octet ne le disent qu'une fois.
    #[test]
    fn two_jumps_to_the_same_byte_are_reported_once() {
        let octets: &[u8] = &[
            0xEB, 0x03, // jmp $+5  → 0x400005
            0xEB, 0x01, // jmp $+3  → 0x400005
            0xE8, // le piège
            0x31, 0xC0, // xor eax,eax
            0xC3, // ret
        ];
        let linéaire = namsdec_disasm::disassemble(octets, 0x400000);
        let trouvés = scan(&linéaire);
        assert_eq!(trouvés.len(), 1, "{trouvés:?}");
        assert_eq!(trouvés[0].target, 0x400005);
    }

    /// Un saut calculé ne dit pas où il va, et on ne le devine pas.
    #[test]
    fn a_computed_jump_is_not_guessed() {
        // jmp rax, puis le même piège derrière.
        let octets: &[u8] = &[0xFF, 0xE0, 0xE8, 0x31, 0xC0, 0xC3];
        let linéaire = namsdec_disasm::disassemble(octets, 0x400000);
        assert!(scan(&linéaire).is_empty(), "{:?}", scan(&linéaire));
    }
}
