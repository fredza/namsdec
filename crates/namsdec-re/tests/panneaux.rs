//! Ce que les panneaux d'analyse **peignent** vraiment.
//!
//! egui ne garantit presque rien sur la mise en page : chaque image est
//! reconstruite de zéro, et rien n'empêche une mesure d'être reprise en
//! entrée de la mesure suivante. Un relevé du code ne voit donc pas ce
//! qu'on voit à l'écran — un texte qui s'alloue une ligne et s'en peint
//! deux, une étiquette qui recouvre sa voisine, une ligne coupée par le
//! bord.
//!
//! Ce contrôle relève les formes de texte que chaque panneau émet et les
//! mesure, avec les cinq précautions qui évitent les faux positifs :
//!
//! * **le rectangle, non la position** — comparer des positions laisse
//!   passer un texte qui déborde ;
//! * **la ligne, non la galley** — un texte de trois lignes a un cadre
//!   qui couvre le blanc au bout de la dernière, et deux de ces cadres se
//!   croisent sans qu'aucune lettre ne se touche ;
//! * **sans l'espace de tête** — dans une rangée qui se replie, egui pose
//!   la galley au début de la rangée et décale son premier mot à
//!   l'intérieur ;
//! * **ce qui se voit, non ce qui est peint** — on intersecte avec le
//!   rectangle de découpe, et c'est ainsi qu'on voit qu'un texte est
//!   coupé ;
//! * **deux largeurs** — étroite et large. La plupart des défauts de
//!   mise en page ne se montrent qu'à l'une des deux.

mod commun;

use commun::{BASE, germe_piégé};
use namsdec_core::{Cpu, Memory};
use namsdec_re::{ReContext, ReModule};

/// Le binaire d'épreuve des panneaux : le germe, plus ce qu'il faut pour
/// que les blocs ajoutés aient quelque chose à dire.
///
/// Un empaqueteur qui se nomme, un indicateur dans les données, et
/// l'entropie d'un gros bloc imprévisible. Sans eux, les panneaux
/// seraient mesurés vides — et c'est précisément avec du contenu que les
/// défauts de mise en page se révèlent.
fn binaire() -> Vec<u8> {
    // Le germe piégé : son `.text` porte la figure de
    // l'anti-désassemblage, ce qui donne au panneau des figures son bloc
    // des deux lectures.
    let mut f = germe_piégé();
    // Les indicateurs sont déjà dans le `.rodata` du germe : les ajouter
    // ici, après la table des sections, ne servirait à rien — le panneau
    // des chaînes ne lit que les sections de données, comme
    // `strings -d`, et c'est juste.
    f.extend_from_slice(b"UPX!");
    // Le code qui demande au noyau : posé DANS le fichier, et non à
    // côté, parce que le listing de namsdec part du premier octet du
    // fichier et que c'est ce listing-là qu'on mesure. Un `nop` devant
    // remet la lecture en phase après les zéros de la table des
    // sections. Huit et non un : une instruction fait au plus quinze
    // octets, et une entrée de travers peut en avaler quelques-uns —
    // après une file de `nop`, la lecture est forcément en phase.
    f.extend(std::iter::repeat_n(0x90u8, 8));
    f.extend_from_slice(&code());
    let mut graine = 0x2545_f491_4f6c_dd1du64;
    for _ in 0..16384 {
        graine ^= graine << 13;
        graine ^= graine >> 7;
        graine ^= graine << 17;
        f.push((graine >> 24) as u8);
    }
    f
}

/// Du code qui demande au noyau de quoi allumer plusieurs familles :
/// `socket`, `execve`, `ptrace`, `mprotect`, `open`, et un saut calculé.
fn code() -> Vec<u8> {
    let mut octets: Vec<u8> = Vec::new();
    for numéro in [41u32, 59, 101, 10, 2] {
        octets.push(0xB8); // mov eax, imm32
        octets.extend_from_slice(&numéro.to_le_bytes());
        octets.extend_from_slice(&[0x0F, 0x05]); // syscall
    }
    octets.extend_from_slice(&[0xFF, 0xE0]); // jmp rax — un saut calculé
    octets.push(0xC3); // ret
    octets
}

/// Une ligne relevée : ce qu'elle dit, l'encre qu'elle occupe, et si elle
/// a perdu quelque chose au découpage.
struct Ligne {
    texte: String,
    encre: egui::Rect,
    clip: egui::Rect,
    /// L'encre a perdu de la **largeur** au découpage : du texte court
    /// au-delà d'un bord.
    rognée: bool,
    élidée: bool,
}

/// Ce qu'un panneau peint, à une largeur donnée.
///
/// Deux images avant de relever : la première pose les identifiants et
/// les tailles qu'egui garde d'une image sur l'autre, et c'est la
/// seconde qui ressemble à ce qu'on voit.
fn relevé(module: &mut dyn ReModule, ctx: &ReContext, largeur: f32) -> Vec<Ligne> {
    let egui_ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(largeur, 2400.0),
        )),
        ..Default::default()
    };
    let mut lignes: Vec<Ligne> = Vec::new();
    for _ in 0..2 {
        lignes.clear();
        let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, ctx));
        for clipped in &sortie.shapes {
            let egui::epaint::Shape::Text(t) = &clipped.shape else {
                continue;
            };
            for ligne in &t.galley.rows {
                if ligne.glyphs.is_empty() {
                    continue;
                }
                let posée = ligne
                    .rect_without_leading_space()
                    .translate(t.pos.to_vec2());
                let encre = posée.intersect(clipped.clip_rect);
                if encre.width() <= 0.0 || encre.height() <= 0.0 {
                    continue;
                }
                lignes.push(Ligne {
                    texte: t.galley.text().to_owned(),
                    encre,
                    clip: clipped.clip_rect,
                    // De la **largeur** perdue, et non de la hauteur.
                    //
                    // Une hauteur perdue est normale et voulue dès qu'une
                    // zone défilante est bornée : sa dernière ligne
                    // visible est coupée par le bas, et c'est à cela que
                    // sert l'ascenseur. La mesurer ferait échouer le
                    // contrôle sur ce que les panneaux font exprès — la
                    // liste des chaînes est bornée à deux cent vingt
                    // pixels, et le dit.
                    //
                    // Une largeur perdue, en revanche, n'est jamais
                    // voulue ici : rien ne défile horizontalement dans
                    // ces panneaux, donc tout texte qui en perd court
                    // au-delà d'un bord.
                    rognée: posée.width() - encre.width() > 0.5,
                    élidée: t.galley.elided,
                });
            }
        }
        sortie.textures_delta.clear();
    }
    lignes
}

/// Tout caractère qu'un panneau peint existe dans la police.
///
/// Un caractère absent ne manque pas discrètement : egui peint un
/// **carré vide** à sa place. Les polices qu'il embarque — Ubuntu et
/// deux polices d'émojis — n'ont ni les petits triangles
/// (`\u{25b8}`, `\u{25be}`), ni la coche `\u{2713}`, ni la croix
/// `\u{2717}`, ni la flèche de retour `\u{21a9}`. Les quatre étaient
/// employés : la liste des protections montrait deux carrés au lieu
/// d'une coche et d'une croix, et le triangle qui déplie les
/// références croisées n'existait pas à l'écran.
///
/// Ce contrôle les attrape tous d'un coup, et attrapera le prochain :
/// il ne lit aucune liste de caractères écrite d'avance — il regarde ce
/// qui est **peint**.
///
/// Ce qui existe et qu'on peut employer : `\u{2039}` `\u{203a}`
/// `\u{00ab}` `\u{00bb}` `+` `\u{2212}` `\u{00d7}` `\u{221a}`
/// `\u{2022}` `\u{25cb}` `\u{2026}` `\u{21ba}`.
#[test]
fn tout_caractere_ecrit_existe_dans_la_police() {
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire, BASE);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
    };
    let mut absents: Vec<(String, char)> = Vec::new();
    // TOUS les panneaux, et non les neuf du relevé de mise en page :
    // un carré vide se voit partout, et la question ne coûte rien.
    let mut tous = panneaux();
    tous.extend([
        (
            "en bref",
            Box::new(namsdec_re::BinaryInfoModule::default()) as Box<dyn ReModule>,
        ),
        ("empreintes", Box::new(namsdec_re::DigestModule::default())),
        ("carte", Box::new(namsdec_re::MapModule::default())),
        (
            "protections",
            Box::new(namsdec_re::ProtectionsModule::default()),
        ),
        (
            "graphe d'appels",
            Box::new(namsdec_re::CallGraphModule::default()),
        ),
        ("yara", Box::new(namsdec_re::YaraModule::default())),
        ("place libre", Box::new(namsdec_re::SlackModule::default())),
        ("vidage", Box::new(namsdec_re::DumpModule::default())),
        (
            "éditeur binaire",
            Box::new(namsdec_re::HexEditorModule::default()),
        ),
    ]);
    for (nom, mut module) in tous {
        module.on_binary_loaded(&ctx);
        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 2400.0),
            )),
            ..Default::default()
        };
        let mut vus: Vec<String> = Vec::new();
        for _ in 0..2 {
            vus.clear();
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, &ctx));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    vus.push(t.galley.text().to_owned());
                }
            }
            sortie.textures_delta.clear();
        }
        // La police PROPORTIONNELLE, quelle que soit celle du texte, et
        // c'est mesuré : interrogée sur la famille à chasse fixe,
        // `has_glyph` répond « non » pour tout, jusqu'au signe plus.
        // La question reste la bonne — la chaîne de repli de la chasse
        // fixe contient celle de la proportionnelle —, elle est
        // seulement un peu sévère.
        let police = egui::FontId::proportional(13.0);
        for texte in &vus {
            for c in texte.chars() {
                if c.is_ascii() || c.is_whitespace() {
                    continue;
                }
                if !egui_ctx.fonts_mut(|f| f.has_glyph(&police, c)) {
                    absents.push((nom.to_owned(), c));
                }
            }
        }
    }
    absents.sort();
    absents.dedup();
    assert!(
        absents.is_empty(),
        "des caractères se peignent en carré vide — la police ne les a pas : {}",
        absents
            .iter()
            .map(|(nom, c)| format!("{nom} : U+{:04X} {c}", *c as u32))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Les panneaux à contrôler, chacun avec le nom qu'un échec doit citer.
fn panneaux() -> Vec<(&'static str, Box<dyn ReModule>)> {
    vec![
        ("signaux", Box::new(namsdec_re::SignalsModule::default())),
        ("imports", Box::new(namsdec_re::ImportsModule::default())),
        (
            "appels système",
            Box::new(namsdec_re::SyscallsModule::default()),
        ),
        ("chaînes", Box::new(namsdec_re::StringsModule::default())),
        ("entropie", Box::new(namsdec_re::EntropyModule::default())),
        ("figures", Box::new(namsdec_re::PatternsModule::default())),
        (
            "fonctions",
            Box::new(namsdec_re::FunctionsModule::default()),
        ),
        ("fragments", Box::new(namsdec_re::GadgetsModule::default())),
        (
            "outil extérieur",
            Box::new(
                namsdec_re::ToolModule::connu("objdump", "objdump", "objdump", "objdump")
                    .expect("objdump est un outil connu"),
            ),
        ),
    ]
}

/// Les cinq panneaux touchés, à deux largeurs, dans les deux langues.
///
/// Trois invariants d'un coup, parce qu'ils se mesurent sur le même
/// relevé : rien ne se recouvre, rien n'est coupé, et un texte tronqué
/// l'est volontairement.
#[test]
fn no_analysis_panel_overlaps_or_cuts_its_own_text() {
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire, BASE);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));

    for lang in ["fr", "en"] {
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &binaire,
            lang,
            names: None,
        };
        // Étroite comme la colonne d'un écran partagé, large comme un
        // écran d'aujourd'hui : un texte qui déborde ne déborde souvent
        // qu'à l'une des deux.
        for largeur in [420.0f32, 1200.0] {
            for (nom, mut module) in panneaux() {
                module.on_binary_loaded(&ctx);
                let lignes = relevé(module.as_mut(), &ctx, largeur);
                assert!(
                    !lignes.is_empty(),
                    "{nom} / {lang} / {largeur} : le panneau ne peint rien"
                );

                // Aucun texte ne court au-delà d'un bord.
                //
                // Une étiquette *volontairement* tronquée est exclue :
                // `truncate()` élide le texte et promet l'infobulle, ce
                // qui est un choix et non un défaut.
                let rognées: Vec<(&str, f32)> = lignes
                    .iter()
                    .filter(|l| l.rognée && !l.élidée)
                    .map(|l| (l.texte.as_str(), l.clip.right()))
                    .collect();
                assert!(
                    rognées.is_empty(),
                    "{nom} / {lang} / largeur {largeur} : du texte court au-delà de son \
                     bord — {rognées:?}"
                );

                // Et rien ne dépasse la largeur qu'on a donnée : un
                // texte peint hors du cadre ne serait pas *rogné* par
                // son propre découpage, mais il serait invisible tout de
                // même.
                let dehors: Vec<(&str, f32)> = lignes
                    .iter()
                    .filter(|l| l.encre.right() > largeur + 0.5)
                    .map(|l| (l.texte.as_str(), l.encre.right()))
                    .collect();
                assert!(
                    dehors.is_empty(),
                    "{nom} / {lang} / largeur {largeur} : du texte est posé hors du \
                     cadre — {dehors:?}"
                );

                // Aucun texte ne porte une suite d'espaces.
                //
                // Ce contrôle vient d'un vrai défaut, trouvé ici : un
                // littéral Rust coupé sur plusieurs lignes garde ses
                // espaces d'indentation si le `\` de continuation
                // disparaît, et la phrase s'affiche trouée. Rien dans
                // ces cinq panneaux n'aligne quoi que ce soit à
                // l'espace — un panneau qui le ferait (un vidage
                // hexadécimal, par exemple) n'est pas dans la liste.
                let trouées: Vec<&str> = lignes
                    .iter()
                    .filter(|l| l.texte.contains("  "))
                    .map(|l| l.texte.as_str())
                    .collect();
                assert!(
                    trouées.is_empty(),
                    "{nom} / {lang} : une phrase est trouée d'espaces — {trouées:?}"
                );

                // Deux textes d'une même couche ne se recouvrent jamais.
                // Deux pixels de tolérance : les jambages d'une ligne
                // frôlent les hampes de la suivante.
                for (i, une) in lignes.iter().enumerate() {
                    for autre in &lignes[i + 1..] {
                        if une.clip != autre.clip {
                            continue;
                        }
                        let croisement = une.encre.shrink(1.0).intersect(autre.encre.shrink(1.0));
                        assert!(
                            croisement.width() <= 0.0 || croisement.height() <= 0.0,
                            "{nom} / {lang} / largeur {largeur} : « {} » {:?} recouvre \
                             « {} » {:?}",
                            une.texte,
                            une.encre,
                            autre.texte,
                            autre.encre
                        );
                    }
                }
            }
        }
    }
}

/// Ce qui est peint ne bouge plus quand on ne touche à rien.
///
/// Vingt images, les huit dernières identiques : egui *amène* une taille
/// au lieu de l'y poser, et une mise en place qui converge n'est pas une
/// dérive. Ce qu'on refuse, c'est ce qui ne s'arrête jamais — un panneau
/// qui lit sa largeur de son contenu et dont le contenu lit sa largeur
/// grandit de quelques pixels par image, indéfiniment.
#[test]
fn an_analysis_panel_left_alone_stays_where_it_is() {
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire, BASE);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
    };

    for (nom, mut module) in panneaux() {
        module.on_binary_loaded(&ctx);
        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 2400.0),
            )),
            ..Default::default()
        };
        let mut images: Vec<Vec<(String, egui::Pos2)>> = Vec::new();
        for _ in 0..20 {
            let mut posés: Vec<(String, egui::Pos2)> = Vec::new();
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, &ctx));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    posés.push((t.galley.text().to_owned(), t.pos));
                }
            }
            sortie.textures_delta.clear();
            images.push(posés);
        }
        for paire in images[12..].windows(2) {
            assert_eq!(
                paire[0].len(),
                paire[1].len(),
                "{nom} : le nombre de textes change d'une image à l'autre"
            );
            for (avant, après) in paire[0].iter().zip(&paire[1]) {
                assert_eq!(avant.0, après.0, "{nom} : l'ordre des textes change");
                assert!(
                    (avant.1 - après.1).length() < 0.01,
                    "{nom} : « {} » bouge toute seule, de {:?} à {:?}",
                    avant.0,
                    avant.1,
                    après.1
                );
            }
        }
    }
}

/// Et ce qu'on a ajouté est bien à l'écran, dans les deux langues.
///
/// Un panneau qui ne panique pas et ne déborde pas peut très bien ne rien
/// montrer : il faut aussi dire ce qu'on attend d'y lire. La liste est
/// courte, et chaque entrée est la raison d'être d'un des ajouts.
#[test]
fn what_was_added_is_really_on_screen() {
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire, BASE);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));

    let attendus: &[(&str, &str, &[&str])] = &[
        // Le panneau de synthèse : un fait, et le panneau où le lire.
        (
            "signaux",
            "fr",
            // Le titre du panneau est peint par la fenêtre qui le
            // contient, non par le module : ce qu'on cherche ici est ce
            // que le module écrit lui-même.
            &["Remarquable", "Empaqueteur reconnu", "à lire dans"],
        ),
        (
            "signaux",
            "en",
            &["Remarkable", "Packer identified", "read it in"],
        ),
        // Le portrait par familles, dans les deux panneaux qui le
        // portent, et avec les mêmes mots.
        ("imports", "fr", &["Ce qu'il sait faire", "Réseau"]),
        ("imports", "en", &["What it can do", "Network"]),
        (
            "appels système",
            "fr",
            &["Ce qu'il sait faire", "Anti-débogage", "Exécution"],
        ),
        (
            "appels système",
            "en",
            &["What it can do", "Anti-debugging"],
        ),
        // Les indicateurs, rangés par nature.
        (
            "chaînes",
            "fr",
            &["Adresse web", "Chemin", "Clé de registre", "copier"],
        ),
        ("chaînes", "en", &["URL", "Path", "Registry key", "copy"]),
        // Et l'empaqueteur nommé, avec sa preuve.
        ("entropie", "fr", &["UPX"]),
        // Les deux lectures du même octet, côte à côte.
        (
            "figures",
            "fr",
            &[
                "Le listing est trompé",
                "le listing lit",
                "le processeur lit",
            ],
        ),
        (
            "figures",
            "en",
            &[
                "The listing is being fooled",
                "the listing reads",
                "the processor reads",
            ],
        ),
    ];

    for (nom, lang, mots) in attendus {
        let ctx = ReContext {
            cpu: &cpu,
            disasm: &disasm,
            source: "",
            binary: &binaire,
            lang,
            names: None,
        };
        let (_, mut module) = panneaux()
            .into_iter()
            .find(|(id, _)| id == nom)
            .unwrap_or_else(|| panic!("le panneau « {nom} » existe"));
        module.on_binary_loaded(&ctx);
        let lignes = relevé(module.as_mut(), &ctx, 1000.0);
        let tout: Vec<&str> = lignes.iter().map(|l| l.texte.as_str()).collect();
        for mot in *mots {
            assert!(
                tout.iter().any(|texte| texte.contains(mot)),
                "{nom} / {lang} : « {mot} » devrait être peint — {tout:?}"
            );
        }
    }
}
