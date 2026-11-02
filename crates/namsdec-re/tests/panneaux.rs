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
    relevé_sur(&egui::Context::default(), module, ctx, largeur)
}

/// Le même relevé, sur un contexte egui QU'ON GARDE.
///
/// Un contexte neuf oublie ce que l'interface retient entre deux
/// images — ce qui est ouvert, ce qui est tapé, ce qui a le clavier.
/// Pour mesurer un panneau APRÈS un clic, il faut donc le même contexte
/// qu'au clic ; `relevé` en fabrique un à chaque fois parce que ses
/// panneaux à lui sont mesurés au repos.
fn relevé_sur(
    egui_ctx: &egui::Context,
    module: &mut dyn ReModule,
    ctx: &ReContext,
    largeur: f32,
) -> Vec<Ligne> {
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
        patches: &[],
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
        // UPX : trois rangées dont deux portent un bouton ET la commande
        // en toutes lettres à côté. C'est « [Déballer vers…]
        // upx -d -o <choisi> <copie> » qui est à surveiller — un bouton
        // et un texte en largeur fixe sur la même ligne, dans deux
        // langues dont l'une est plus longue.
        ("upx", Box::new(namsdec_re::UpxModule::default())),
        ("figures", Box::new(namsdec_re::PatternsModule::default())),
        (
            "fonctions",
            Box::new(namsdec_re::FunctionsModule::default()),
        ),
        ("fragments", Box::new(namsdec_re::GadgetsModule::default())),
        // Le détour : trois rangées de commandes, une liste déroulante
        // et un champ multiligne. C'est la rangée « Détourner depuis …
        // [champ] [ici] » qui est à surveiller — trois éléments de
        // largeurs différentes sur une même ligne, dans deux langues
        // dont l'une est plus longue.
        ("détour", Box::new(namsdec_re::DetourModule::default())),
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
            patches: &[],
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
        patches: &[],
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
        // Le détour : ses TROIS commandes, et c'est ce qui prouve que le
        // panneau n'a pas pris son repli. Sans place libre exécutable il
        // écrit « aucune place libre » et s'arrête — le relevé serait
        // non vide, le contrôle de mise en page passerait, et l'on
        // n'aurait mesuré que la phrase d'excuse.
        (
            "détour",
            "fr",
            &[
                "Détourner depuis",
                "vers la place libre",
                "Code à ajouter",
                "Calculer le plan",
            ],
        ),
        (
            "détour",
            "en",
            &[
                "Detour from",
                "towards the slack",
                "Code to add",
                "Compute the plan",
            ],
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
            patches: &[],
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

/// Les sources des panneaux, pour les contrôles qui lisent le code.
///
/// Tenue à jour par `la_liste_des_panneaux_ne_rate_aucun_module`, qui la
/// confronte aux `pub mod` de `lib.rs` : un panneau neuf y entre sans
/// qu'on ait à y penser.
const SOURCES_DES_PANNEAUX: &[(&str, &str)] = &[
    ("lib.rs", include_str!("../src/lib.rs")),
    ("behaviour.rs", include_str!("../src/behaviour.rs")),
    ("cfg.rs", include_str!("../src/cfg.rs")),
    ("cyclic.rs", include_str!("../src/cyclic.rs")),
    ("detour_module.rs", include_str!("../src/detour_module.rs")),
    ("digest.rs", include_str!("../src/digest.rs")),
    ("dump_module.rs", include_str!("../src/dump_module.rs")),
    ("dzf.rs", include_str!("../src/dzf.rs")),
    ("elf_scan.rs", include_str!("../src/elf_scan.rs")),
    ("entropy.rs", include_str!("../src/entropy.rs")),
    ("explain.rs", include_str!("../src/explain.rs")),
    ("functions.rs", include_str!("../src/functions.rs")),
    ("gadgets.rs", include_str!("../src/gadgets.rs")),
    ("imports.rs", include_str!("../src/imports.rs")),
    ("indirect.rs", include_str!("../src/indirect.rs")),
    ("ioc.rs", include_str!("../src/ioc.rs")),
    ("libc.rs", include_str!("../src/libc.rs")),
    ("literals.rs", include_str!("../src/literals.rs")),
    ("misalign.rs", include_str!("../src/misalign.rs")),
    ("packer.rs", include_str!("../src/packer.rs")),
    ("patch.rs", include_str!("../src/patch.rs")),
    ("patterns.rs", include_str!("../src/patterns.rs")),
    ("provenance.rs", include_str!("../src/provenance.rs")),
    ("report.rs", include_str!("../src/report.rs")),
    ("route.rs", include_str!("../src/route.rs")),
    ("signals.rs", include_str!("../src/signals.rs")),
    ("slack.rs", include_str!("../src/slack.rs")),
    ("strings.rs", include_str!("../src/strings.rs")),
    ("structure.rs", include_str!("../src/structure.rs")),
    ("vue.rs", include_str!("../src/vue.rs")),
    ("vue_peinte.rs", include_str!("../src/vue_peinte.rs")),
    ("syscalls.rs", include_str!("../src/syscalls.rs")),
    ("table.rs", include_str!("../src/table.rs")),
    ("toolchain.rs", include_str!("../src/toolchain.rs")),
    ("xrefs.rs", include_str!("../src/xrefs.rs")),
];

/// Un bouton d'un panneau se voit **sans qu'on le survole**.
///
/// Le thème rend les boutons plats au repos — fond transparent, liseré
/// absent —, et c'est voulu dans la barre de menus et la barre d'outils,
/// où la platitude EST le dessin. Partout ailleurs, un bouton plat ne se
/// distingue d'une étiquette qu'au survol : on ignore qu'il y a quelque
/// chose à cliquer.
///
/// L'interface avait son contrôle depuis longtemps, mais il ne lisait
/// que les sources de `namsdec-gui`. Les panneaux d'analyse vivent ici,
/// et leurs trente-trois boutons étaient donc plats — dans la MÊME
/// fenêtre où ceux de l'interface portent un liseré. Vu à l'écran le
/// 15 octobre 2026 pendant l'audit du panneau « Détourner », et non par
/// une mesure : les mesures ne répondent qu'aux questions qu'on leur
/// pose.
///
/// `framed_button`, `framed_small_button` et `framed` posent le liseré.
/// Ce contrôle lit le source, parce que c'est là qu'est la règle.
#[test]
fn un_bouton_de_panneau_se_voit_sans_survol() {
    let mut plats: Vec<String> = Vec::new();
    for (fichier, source) in SOURCES_DES_PANNEAUX {
        // La fonction englobante : elle sert à nommer la SEULE exception,
        // et une exception qu'on ne peut pas nommer n'en est pas une.
        let mut fonction = "";
        for (n, ligne) in source.lines().enumerate() {
            if let Some(nom) = ligne
                .trim_start()
                .strip_prefix("pub fn ")
                .or_else(|| ligne.trim_start().strip_prefix("fn "))
            {
                fonction = nom.split('(').next().unwrap_or("").trim();
            }
            // `framed` EST l'endroit où le liseré se pose : c'est la
            // seule ligne du projet qui ait le droit d'appeler
            // `egui::Button::new` nue.
            if fonction == "framed" {
                continue;
            }
            let nu = ligne.trim_start();
            // Les deux écritures : sur une ligne, et coupée par rustfmt
            // dès qu'on ajoute `.on_hover_text(…)` —
            //
            //     if ui
            //         .button(…)
            //         .on_hover_text(…)
            //
            // La seconde est celle qui avait échappé au contrôle de
            // l'interface pendant des semaines ; on la cherche donc dès
            // le premier jour.
            let fautif = nu.contains("ui.button(")
                || nu.contains("ui.small_button(")
                || nu.starts_with(".button(")
                || nu.starts_with(".small_button(")
                // `egui::Button::new` sans liseré : c'est la forme
                // qu'emploie `add_enabled`, et elle compte aussi.
                || nu.contains("egui::Button::new(");
            if fautif {
                plats.push(format!("{fichier}:{} — {}", n + 1, nu.trim()));
            }
        }
    }
    assert!(
        plats.is_empty(),
        "{} bouton(s) de panneau sans liseré : on ne les distingue d'une \
         étiquette qu'au survol, donc on ne sait pas qu'on peut cliquer. \
         Employez `framed_button`, `framed_small_button` ou `framed` \
         (namsdec-re/src/lib.rs) :\n  {}",
        plats.len(),
        plats.join("\n  ")
    );
}

/// La liste ci-dessus n'oublie aucun panneau.
///
/// Sans ce contrôle, elle serait exacte le jour où elle est écrite et
/// fausse au panneau suivant — et sa fausseté ferait PASSER le contrôle
/// d'au-dessus, non échouer. C'est la pire sorte de test : vert pour la
/// mauvaise raison.
#[test]
fn la_liste_des_panneaux_ne_rate_aucun_module() {
    let déclarés: Vec<String> = include_str!("../src/lib.rs")
        .lines()
        .filter_map(|l| l.strip_prefix("pub mod ").and_then(|r| r.strip_suffix(';')))
        .map(|m| format!("{m}.rs"))
        .collect();
    let oubliés: Vec<&String> = déclarés
        .iter()
        .filter(|m| !SOURCES_DES_PANNEAUX.iter().any(|(nom, _)| nom == m))
        .collect();
    assert!(
        oubliés.is_empty(),
        "SOURCES_DES_PANNEAUX ignore {} module(s) — les contrôles qui \
         lisent le code ne les verraient pas :\n  {:?}",
        oubliés.len(),
        oubliés
    );
}

/// DANS LE GRAPHE D'APPELS, AUCUN TEXTE NE SORT DE SA BOÎTE.
///
/// Le graphe est dessiné à la main, sur un canevas : rien, dans egui,
/// ne garantit qu'un texte tienne dans le rectangle qu'on a peint pour
/// lui. C'est à la hauteur réservée de le garantir, et elle se calcule
/// — donc elle se trompe.
///
/// Elle s'est trompée le 20 octobre, en passant les boîtes au style
/// UML : le compte des lignes oubliait que le compartiment du bas en
/// porte DEUX, et la seconde — « 0 appelants · 13 appels » — se
/// peignait sous le trait du cadre, hors de la boîte. Vu sur capture.
///
/// Le contrôle apparie chaque texte du graphe avec le rectangle qui
/// devrait le contenir, et n'accepte pas qu'il n'y en ait aucun.
#[test]
fn no_text_of_the_call_graph_escapes_its_box() {
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x200], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    let mut module = namsdec_re::CallGraphModule::default();
    module.on_binary_loaded(&ctx);

    let egui_ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1000.0, 2400.0),
        )),
        ..Default::default()
    };
    let mut dehors: Vec<String> = Vec::new();
    let mut vus = 0usize;
    for _ in 0..2 {
        dehors.clear();
        vus = 0;
        let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, &ctx));
        // Les boîtes : des rectangles peints, assez larges et assez
        // hauts pour porter plusieurs lignes. Les filets de séparation,
        // plats, ne passent pas ce crible.
        let boîtes: Vec<egui::Rect> = sortie
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::epaint::Shape::Rect(r) => Some(r.rect),
                _ => None,
            })
            .filter(|r| r.width() > 60.0 && r.height() > 30.0)
            .collect();
        for clipped in &sortie.shapes {
            let egui::epaint::Shape::Text(t) = &clipped.shape else {
                continue;
            };
            // Le corps d'une boîte, et lui seul : c'est la ligne qui
            // débordait, et la seule dont on sache à coup sûr qu'elle
            // appartient à une boîte.
            let texte = t.galley.text();
            if !texte.contains("appelants") {
                continue;
            }
            vus += 1;
            let encre = egui::Rect::from_min_size(t.pos, t.galley.size());
            // Deux pixels de tolérance : les jambages d'une police
            // dépassent de la hauteur de ligne nominale.
            let dedans = boîtes.iter().any(|b| b.expand(2.0).contains_rect(encre));
            if !dedans {
                dehors.push(format!(
                    "« {texte} » en {:?}, aucune boîte ne le contient",
                    encre
                ));
            }
        }
        sortie.textures_delta.clear();
        let _ = module.take_request();
    }
    assert!(
        vus > 0,
        "le relevé n'a trouvé aucune boîte du graphe — il ne mesure rien"
    );
    assert!(
        dehors.is_empty(),
        "des textes du graphe sortent de leur boîte : {dehors:#?}"
    );
}

/// CLIQUER UN OCTET PUIS TAPER ÉCRIT, même après avoir employé
/// « Chercher ».
///
/// La grille n'écoute les touches que lorsque aucun champ de texte n'a
/// le focus — sinon « Chercher » recevrait les chiffres hexadécimaux
/// qu'on tape pour écrire un octet. Le parcours complet n'était éprouvé
/// nulle part : on ne savait pas si, après avoir cliqué dans un champ,
/// la grille reprenait la main.
///
/// Elle la reprend, et c'est egui qui s'en charge — il rend le focus
/// dès qu'on clique un widget qui ne le prend pas. Ce contrôle existe
/// pour que cela reste vrai : si egui changeait d'avis, ou si la grille
/// gagnait un champ de plus, la frappe deviendrait muette sans un mot.
///
/// Écrit le 21 octobre 2026 en cherchant à reproduire « dans l'éditeur
/// binaire on ne sait rien modifier ». Il n'y est pas parvenu : les
/// trois chemins — le bouton « Écrire », la frappe dans la grille, et
/// l'inscription du correctif — répondent tous.
#[test]
fn clicking_a_byte_takes_the_keyboard_back() {
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x140], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    let mut module = namsdec_re::HexEditorModule::default();
    module.on_binary_loaded(&ctx);

    let egui_ctx = egui::Context::default();
    let base = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 1600.0),
        )),
        ..Default::default()
    };

    // Deux images pour poser la grille, et le relevé de ce qu'il faut
    // cliquer : une cellule hexadécimale, et le champ « Chercher ».
    //
    // LA CELLULE EST CELLE DU BAS, et non la première venue. Depuis que
    // le vidage porte une RÈGLE DE COLONNES, « 00 », « 01 », … « 1f »
    // sont peints au-dessus de lui, en tête : deux chiffres
    // hexadécimaux comme les octets, et en premier dans la liste des
    // formes. Le relevé cliquait donc la règle, qui ne se clique pas,
    // et le contrôle échouait sans rien dire de l'éditeur. Prendre la
    // plus BASSE lève l'ambiguïté : la règle est toujours au-dessus.
    let mut cible: Option<egui::Pos2> = None;
    let mut champ = None;
    for _ in 0..2 {
        cible = None;
        champ = None;
        let mut sortie = egui_ctx.run_ui(base.clone(), |ui| module.ui(ui, &ctx));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let texte = t.galley.text();
                let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                let vu = rect.intersect(clipped.clip_rect);
                if !vu.is_positive() {
                    continue;
                }
                // Une cellule hexadécimale : deux chiffres, et rien
                // d'autre.
                if texte.len() == 2
                    && texte.chars().all(|c| c.is_ascii_hexdigit())
                    && cible.is_none_or(|déjà| vu.center().y > déjà.y)
                {
                    cible = Some(vu.center());
                }
                // Et le champ « Chercher », repéré par son exemple :
                // c'est LUI qui doit voler le clavier, parce qu'il
                // existe vraiment. Un identifiant jamais dessiné perd
                // son focus de lui-même à la fin de l'image, et le
                // contrôle ne reproduisait alors rien du tout.
                // Repéré par son ÉTIQUETTE et non par son exemple : le
                // texte d'exemple change avec la case « hexa », et le
                // relevé ne trouvait alors rien — il refusait de
                // mesurer, ce qui est au moins honnête.
                if texte == "Chercher" && champ.is_none() {
                    // Le champ est juste à droite de l'étiquette.
                    champ = Some(vu.center() + egui::vec2(vu.width() + 40.0, 0.0));
                }
            }
        }
        sortie.textures_delta.clear();
        let _ = module.take_request();
    }
    let (Some(cible), Some(champ)) = (cible, champ) else {
        panic!("le relevé n'a pas trouvé la grille et le champ — il ne mesure rien");
    };

    // On clique « Chercher » : il prend le clavier, comme dans la vraie
    // vie dès qu'on y a tapé une fois.
    let clic = |où: egui::Pos2, module: &mut dyn ReModule| {
        for pressé in [true, false] {
            let mut entrée = base.clone();
            if pressé {
                entrée.events.push(egui::Event::PointerMoved(où));
            }
            entrée.events.push(egui::Event::PointerButton {
                pos: où,
                button: egui::PointerButton::Primary,
                pressed: pressé,
                modifiers: egui::Modifiers::NONE,
            });
            let mut sortie = egui_ctx.run_ui(entrée, |ui| module.ui(ui, &ctx));
            sortie.textures_delta.clear();
            let _ = module.take_request();
        }
    };
    clic(champ, &mut module);
    assert!(
        egui_ctx.memory(|m| m.focused()).is_some(),
        "le champ « Chercher » doit avoir pris le clavier — sans quoi il n'y a rien à reprendre"
    );

    // Puis le clic sur l'octet.
    clic(cible, &mut module);

    // Le focus a bien été repris : c'est la correction même.
    assert!(
        egui_ctx.memory(|m| m.focused()).is_none(),
        "cliquer un octet doit reprendre le clavier au champ qui l'avait"
    );

    // Puis deux chiffres hexadécimaux : « 4 » puis « 2 » font 0x42.
    let mut demande = None;
    for key in [egui::Key::Num4, egui::Key::Num2] {
        let mut entrée = base.clone();
        entrée.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut sortie = egui_ctx.run_ui(entrée, |ui| module.ui(ui, &ctx));
        sortie.textures_delta.clear();
        if let Some(d) = module.take_request() {
            demande = Some(d);
        }
    }
    match demande {
        Some(namsdec_re::ReRequest::PatchBytes { bytes, .. }) => {
            assert_eq!(bytes, vec![0x42], "« 4 » puis « 2 » écrivent 0x42");
        }
        autre => panic!("taper deux chiffres doit demander une écriture : {autre:?}"),
    }
}

/// La règle des colonnes tombe juste au-dessus des octets.
///
/// C'est tout ce qu'on lui demande, et c'est ce qu'un vidage sans règle
/// ne peut pas offrir : savoir à quelle position un octet se trouve
/// sans compter les colonnes avec le doigt. Décalée d'un seul
/// caractère, elle est pire que rien — elle désigne la mauvaise.
///
/// Le contrôle compare les ABSCISSES, colonne par colonne, entre la
/// rangée d'en-tête et la première ligne du vidage. C'était impossible
/// tant que chaque octet était une étiquette posée dans une rangée
/// horizontale : personne ne connaissait son abscisse, pas même le code
/// qui la posait.
#[test]
fn the_column_ruler_lines_up_with_the_bytes() {
    let binaire = binaire();
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &[],
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    let mut module = namsdec_re::HexEditorModule::default();
    module.on_binary_loaded(&ctx);

    let egui_ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1200.0),
        )),
        ..Default::default()
    };

    // Deux images : la première pose la zone défilante, la seconde la
    // peint à sa taille.
    let mut cellules: Vec<(String, egui::Rect)> = Vec::new();
    for _ in 0..2 {
        cellules.clear();
        let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, &ctx));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let texte = t.galley.text();
                if texte.len() != 2 || !texte.chars().all(|c| c.is_ascii_hexdigit()) {
                    continue;
                }
                let vu =
                    egui::Rect::from_min_size(t.pos, t.galley.size()).intersect(clipped.clip_rect);
                if vu.is_positive() {
                    cellules.push((texte.to_owned(), vu));
                }
            }
        }
        sortie.textures_delta.clear();
        let _ = module.take_request();
    }
    assert!(
        cellules.len() > 32,
        "le relevé n'a trouvé que {} cellules : il ne mesure rien",
        cellules.len()
    );

    // Les rangées, par ordonnée : la première est la règle, la suivante
    // est la première ligne du vidage.
    let mut rangées: std::collections::BTreeMap<i32, Vec<(String, egui::Rect)>> =
        Default::default();
    for (texte, rect) in &cellules {
        rangées
            .entry(rect.top().round() as i32)
            .or_default()
            .push((texte.clone(), *rect));
    }
    // Seules les rangées PLEINES : « 16 » et « 32 », le choix du nombre
    // d'octets par ligne, sont aussi deux chiffres hexadécimaux, et
    // sont posés plus haut que la règle. Le relevé les prenait pour
    // elle et comparait deux boutons à une ligne de vidage. C'est la
    // deuxième façon de mentir du catalogue — « il cherche le premier »
    // —, et elle se corrige en disant ce qu'on cherche : une rangée
    // d'au moins huit colonnes.
    let mut rangées: Vec<Vec<(String, egui::Rect)>> = rangées
        .into_values()
        .filter(|rangée| rangée.len() >= 8)
        .collect();
    for rangée in &mut rangées {
        rangée.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    }
    assert!(
        rangées.len() >= 2,
        "il faut une règle et au moins une ligne"
    );
    let règle = &rangées[0];
    let première = &rangées[1];

    // La règle numérote ses colonnes à partir de zéro : c'est ce qui
    // permet de lire la position d'un octet en lisant sa colonne.
    let numéros: Vec<String> = règle.iter().map(|(t, _)| t.clone()).collect();
    let attendus: Vec<String> = (0..numéros.len()).map(|i| format!("{i:02x}")).collect();
    assert_eq!(
        numéros, attendus,
        "la règle doit numéroter les colonnes à partir de zéro"
    );

    // Et chaque numéro tombe sur la colonne qu'il désigne.
    assert!(
        première.len() >= règle.len(),
        "la première ligne du vidage porte moins d'octets que la règle n'a de colonnes"
    );
    for (col, ((_, haut), (_, bas))) in règle.iter().zip(première.iter()).enumerate() {
        let écart = (haut.left() - bas.left()).abs();
        assert!(
            écart < 1.0,
            "colonne {col} : la règle est à x={} et l'octet à x={} — {écart:.1} px d'écart",
            haut.left(),
            bas.left()
        );
    }
}

/// Le vidage ne déborde pas la place qu'on lui offre.
///
/// Il l'a débordée : la règle était allouée à la largeur du vidage, qui
/// dépassait le panneau. Celui-ci se dotait d'une barre horizontale, la
/// zone du vidage d'une seconde juste en dessous, et l'on voyait deux
/// barres l'une sur l'autre pour trois pixels à faire défiler.
#[test]
fn the_dump_stays_inside_the_width_it_is_given() {
    let binaire = binaire();
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &[],
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    // Trois largeurs, dont une trop étroite pour seize octets par
    // ligne : c'est là que le débordement se produisait.
    for largeur in [520.0_f32, 900.0, 1400.0] {
        let mut module = namsdec_re::HexEditorModule::default();
        module.on_binary_loaded(&ctx);
        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, 1200.0),
            )),
            ..Default::default()
        };
        let mut débords: Vec<String> = Vec::new();
        for _ in 0..3 {
            débords.clear();
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, &ctx));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                    // L'encre VUE, et non l'encre posée : une zone qui
                    // défile peint ce qu'elle rogne, et le rognage
                    // n'est pas un débord.
                    let vu = rect.intersect(clipped.clip_rect);
                    if vu.is_positive() && vu.right() > largeur + 1.0 {
                        débords.push(format!(
                            "« {} » jusqu'à x={:.0}",
                            t.galley.text(),
                            vu.right()
                        ));
                    }
                }
            }
            sortie.textures_delta.clear();
            let _ = module.take_request();
        }
        assert!(
            débords.is_empty(),
            "à {largeur} px de large, le vidage déborde : {débords:?}"
        );
    }
}

/// L'octet choisi dit dans quelle section il tombe.
///
/// `.text`, `.rodata`, `.data` : un nom plutôt qu'un nombre. C'est la
/// première question qu'on se pose devant un octet qu'on s'apprête à
/// changer, parce que la réponse décide de tout le reste — on ne touche
/// pas à du code comme à une constante. Il fallait aller la poser à la
/// carte du fichier, dans un autre panneau.
#[test]
fn the_picked_byte_says_which_section_it_falls_in() {
    let binaire = binaire();
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &[],
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    // La section qu'on vise, demandée au même lecteur d'ELF que le
    // panneau : écrire « .text » ici supposerait un binaire d'épreuve
    // qu'on ne contrôle pas d'ici.
    let scan = namsdec_re::elf_scan::scan(&binaire).expect("le germe est un ELF");
    let section = scan
        .sections
        .iter()
        .find(|s| !s.nobits && s.size > 4 && s.name.starts_with('.'))
        .expect("le germe porte au moins une section nommée");
    let dedans = section.offset as usize + 1;

    let mut module = namsdec_re::HexEditorModule::default();
    module.on_binary_loaded(&ctx);
    module.deliver(&namsdec_re::ReResult::ShowBytesAt {
        offset: dedans,
        addr: None,
    });

    let egui_ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1400.0),
        )),
        ..Default::default()
    };
    let mut écrits: Vec<String> = Vec::new();
    for _ in 0..2 {
        écrits.clear();
        let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| module.ui(ui, &ctx));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let vu =
                    egui::Rect::from_min_size(t.pos, t.galley.size()).intersect(clipped.clip_rect);
                if vu.is_positive() {
                    écrits.push(t.galley.text().to_owned());
                }
            }
        }
        sortie.textures_delta.clear();
        let _ = module.take_request();
    }
    assert!(
        écrits.iter().any(|t| t == &section.name),
        "« {} » doit être écrit quand l'octet {dedans:#x} est choisi",
        section.name
    );
}

/// Maj+flèches choisit plusieurs octets, et « Remplir de 90 » les
/// recouvre d'un seul correctif.
///
/// C'est le geste le plus fréquent devant un binaire — neutraliser une
/// instruction entière —, et on le faisait octet par octet, en tapant
/// « 9 0 » autant de fois qu'il le fallait. Un correctif par octet, donc
/// autant de gestes pour défaire.
#[test]
fn shift_arrows_pick_several_bytes_and_fill_writes_them_at_once() {
    let binaire = binaire();
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &[],
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    let mut module = namsdec_re::HexEditorModule::default();
    module.on_binary_loaded(&ctx);
    module.deliver(&namsdec_re::ReResult::ShowBytesAt {
        offset: 0x100,
        addr: None,
    });

    let egui_ctx = egui::Context::default();
    let base = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1600.0),
        )),
        ..Default::default()
    };
    let image = |module: &mut dyn ReModule, events: Vec<egui::Event>| {
        let mut entrée = base.clone();
        entrée.events = events;
        let mut sortie = egui_ctx.run_ui(entrée, |ui| module.ui(ui, &ctx));
        sortie.textures_delta.clear();
    };
    image(&mut module, Vec::new());
    let _ = module.take_request();

    // Deux fois Maj+droite : trois octets choisis, de 0x100 à 0x102.
    for _ in 0..2 {
        image(
            &mut module,
            vec![egui::Event::Key {
                key: egui::Key::ArrowRight,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
        );
        let _ = module.take_request();
    }

    // Ce que le panneau dit de la sélection : trois octets, et leurs
    // bornes. Mesuré sur ce qui est PEINT — un champ privé qui dirait
    // « trois » sans que rien ne s'affiche ne servirait à personne.
    let mut écrits: Vec<(String, egui::Rect)> = Vec::new();
    for _ in 0..2 {
        écrits.clear();
        let mut sortie = egui_ctx.run_ui(base.clone(), |ui| module.ui(ui, &ctx));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                let vu =
                    egui::Rect::from_min_size(t.pos, t.galley.size()).intersect(clipped.clip_rect);
                if vu.is_positive() {
                    écrits.push((t.galley.text().to_owned(), vu));
                }
            }
        }
        sortie.textures_delta.clear();
        let _ = module.take_request();
    }
    assert!(
        écrits
            .iter()
            .any(|(t, _)| t.contains("3 octets") && t.contains("0x100")),
        "le panneau doit dire ce qui est choisi : {:?}",
        écrits.iter().map(|(t, _)| t).collect::<Vec<_>>()
    );

    // Et « Remplir de 90 » demande UN correctif de trois octets.
    let bouton = écrits
        .iter()
        .find(|(t, _)| t.starts_with("Remplir de"))
        .map(|(_, r)| r.center())
        .expect("le bouton « Remplir de 90 » est peint");
    let mut demande = None;
    for pressé in [true, false] {
        let mut events = Vec::new();
        if pressé {
            events.push(egui::Event::PointerMoved(bouton));
        }
        events.push(egui::Event::PointerButton {
            pos: bouton,
            button: egui::PointerButton::Primary,
            pressed: pressé,
            modifiers: egui::Modifiers::NONE,
        });
        image(&mut module, events);
        if let Some(d) = module.take_request() {
            demande = Some(d);
        }
    }
    match demande {
        Some(namsdec_re::ReRequest::PatchBytes {
            offset,
            retirés,
            bytes,
        }) => {
            assert_eq!(offset, 0x100);
            assert_eq!(retirés, 3, "les trois octets choisis sont remplacés");
            assert_eq!(bytes, vec![0x90, 0x90, 0x90], "par autant de `nop`");
        }
        autre => panic!("« Remplir » doit demander une écriture de trois octets : {autre:?}"),
    }
}

/// La sélection s'emporte, et le presse-papiers revient dans le champ.
///
/// Deux gestes qui manquaient dès que la sélection a existé : on
/// choisit seize octets et l'on n'a aucun moyen de les sortir de là,
/// ni d'y mettre ceux qu'on a relevés ailleurs.
///
/// Ctrl+C copie dans l'écriture de la COLONNE ACTIVE — des octets
/// hexadécimaux à gauche, le texte à droite —, la même règle que pour
/// la frappe : on copie ce qu'on voit.
///
/// Ctrl+V, lui, remplit le CHAMP et n'écrit rien. C'est le seul endroit
/// du panneau où la frappe n'atteint pas le fichier, et c'est voulu :
/// un collage peut porter quatre mille octets venus d'on ne sait où.
#[test]
fn the_selection_can_be_copied_and_the_clipboard_lands_in_the_field() {
    let binaire = binaire();
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &[],
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    let mut module = namsdec_re::HexEditorModule::default();
    module.on_binary_loaded(&ctx);
    module.deliver(&namsdec_re::ReResult::ShowBytesAt {
        offset: 0x100,
        addr: None,
    });

    let egui_ctx = egui::Context::default();
    let base = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 1600.0),
        )),
        ..Default::default()
    };
    let image = |module: &mut dyn ReModule, events: Vec<egui::Event>| -> egui::PlatformOutput {
        let mut entrée = base.clone();
        entrée.events = events;
        let mut sortie = egui_ctx.run_ui(entrée, |ui| module.ui(ui, &ctx));
        sortie.textures_delta.clear();
        sortie.platform_output
    };
    image(&mut module, Vec::new());
    let _ = module.take_request();

    // Trois octets choisis : 0x100, 0x101, 0x102.
    for _ in 0..2 {
        image(
            &mut module,
            vec![egui::Event::Key {
                key: egui::Key::ArrowRight,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
        );
        let _ = module.take_request();
    }

    // Ctrl+C : egui en fait un évènement `Copy`, et ce qui part se lit
    // dans ce que la plateforme reçoit.
    let sortie = image(&mut module, vec![egui::Event::Copy]);
    let attendu = binaire[0x100..0x103]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    let copié: Vec<String> = sortie
        .commands
        .iter()
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        copié,
        vec![attendu.clone()],
        "la sélection part en octets hexadécimaux"
    );

    // Ctrl+V : le champ se remplit, et RIEN n'est écrit.
    image(&mut module, vec![egui::Event::Paste("de ad be ef".into())]);
    assert!(
        module.take_request().is_none(),
        "un collage n'écrit pas tout seul : il se lit d'abord"
    );
    let mut écrits: Vec<String> = Vec::new();
    for _ in 0..2 {
        écrits.clear();
        let mut sortie = egui_ctx.run_ui(base.clone(), |ui| module.ui(ui, &ctx));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                écrits.push(t.galley.text().to_owned());
            }
        }
        sortie.textures_delta.clear();
        let _ = module.take_request();
    }
    assert!(
        écrits.iter().any(|t| t == "de ad be ef"),
        "le collage doit se lire dans le champ avant d'être écrit"
    );
}

/// UNE ACTION SANS ENCADRÉ SE PEINT EN LIEN, ET NON EN BOUTON.
///
/// Le vocabulaire des vues le dit depuis le premier jour :
/// « par défaut une action fait un lien — du texte qu'on clique, comme
/// une adresse —, parce que c'est la forme la plus fréquente et la plus
/// discrète. Un bouton encadré dit « ceci va LANCER quelque chose ».
///
/// Le peintre, lui, encadrait TOUT, et le drapeau `bouton` ne servait à
/// rien.
///
/// Ce n'était pas une subtilité : les noms de fonctions importées
/// étaient des textes soulignés avant P7 — `clickable_label`, la seule
/// grammaire du cliquable dans ce projet —, et la migration les a
/// changés en cinquante boutons encadrés dans une colonne.
///
/// Le contrôle mesure ce qui est peint : un encadré pose un rectangle
/// autour de son texte, un lien pose un trait dessous.
#[test]
fn une_action_sans_encadre_se_peint_en_lien() {
    use namsdec_re::vue::{Bloc, Fragment, Vue};
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x200], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    let compte = |vue: &Vue| {
        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 400.0),
            )),
            ..Default::default()
        };
        let mut cadres = 0usize;
        let mut traits = 0usize;
        for _ in 0..2 {
            cadres = 0;
            traits = 0;
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| {
                namsdec_re::vue_peinte::peindre_tout(ui, &ctx, vue);
            });
            for clipped in &sortie.shapes {
                match &clipped.shape {
                    // Le cadre d'un bouton : un rectangle à coins
                    // arrondis, assez large pour porter son texte.
                    egui::epaint::Shape::Rect(r) if r.rect.width() > 20.0 => cadres += 1,
                    // Le trait du cliquable : `clickable_label` le pose
                    // sous le texte, sur toute sa largeur.
                    egui::epaint::Shape::LineSegment { points, .. }
                        if (points[0].y - points[1].y).abs() < 0.5
                            && (points[1].x - points[0].x).abs() > 10.0 =>
                    {
                        traits += 1
                    }
                    _ => {}
                }
            }
            sortie.textures_delta.clear();
        }
        (cadres, traits)
    };

    // Un lien : un trait, pas de cadre.
    let (cadres, traits) =
        compte(&Vue::neuve().texte(Fragment::dit("printf").agit("fiche:printf")));
    assert_eq!(cadres, 0, "une action sans encadré ne pose pas de cadre");
    assert_eq!(traits, 1, "elle pose le trait du cliquable");

    // Un bouton encadré : un cadre, et pas de trait.
    let (cadres, traits) =
        compte(&Vue::neuve().texte(Fragment::dit("Identifier").bouton("upx:lister")));
    assert_eq!(cadres, 1, "un encadré se voit sans qu'on le survole");
    assert_eq!(traits, 0, "et ne porte pas le trait d'un lien");

    // Et les deux formes coexistent dans la même vue, chacune la sienne.
    let (cadres, traits) = compte(
        &Vue::neuve().bloc(Bloc::Texte(
            vec![
                Fragment::dit("printf").agit("fiche:printf"),
                Fragment::dit("Identifier").bouton("upx:lister"),
            ]
            .into(),
        )),
    );
    assert_eq!((cadres, traits), (1, 1));
}

/// LE TITRE D'UN REPLIABLE GARDE LE TON QUE LE MODULE A DIT.
///
/// Peint en texte nu, un titre perdait sa chasse fixe et son gras : le
/// panneau de décompilation demande `Fragment::dit(nom).code().fort()`
/// pour ses fonctions, et les voyait s'écrire comme une phrase
/// ordinaire. Un nom de fonction en chasse proportionnelle n'est pas
/// qu'une question de goût — c'est ce qui distingue `sub_401136` d'un
/// mot de la langue.
#[test]
fn le_titre_d_un_repliable_garde_son_ton() {
    use namsdec_re::vue::{Bloc, Fragment, Vue};
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x200], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    let vue = Vue::neuve().bloc(Bloc::Repliable {
        titre: Fragment::dit("sub_401136").code().into(),
        ouvert: false,
        blocs: vec![Bloc::Texte("dedans".into())],
    });
    let egui_ctx = egui::Context::default();
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(600.0, 400.0),
        )),
        ..Default::default()
    };
    let mut chasses: Vec<egui::FontFamily> = Vec::new();
    for _ in 0..2 {
        chasses.clear();
        let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| {
            namsdec_re::vue_peinte::peindre_tout(ui, &ctx, &vue);
        });
        for clipped in &sortie.shapes {
            let egui::epaint::Shape::Text(t) = &clipped.shape else {
                continue;
            };
            if t.galley.text() != "sub_401136" {
                continue;
            }
            chasses.extend(
                t.galley
                    .job
                    .sections
                    .iter()
                    .map(|s| s.format.font_id.family.clone()),
            );
        }
        sortie.textures_delta.clear();
    }
    assert!(
        !chasses.is_empty(),
        "le titre du repliable n'a pas été relevé — le contrôle ne mesure rien"
    );
    assert!(
        chasses.iter().all(|f| *f == egui::FontFamily::Monospace),
        "un titre dit `code` s'écrit en chasse fixe : {chasses:?}"
    );
}

/// OUVRIR UNE FICHE DU GRAPHE D'APPELS : ELLE S'OUVRE, ET RIEN N'Y
/// DÉBORDE.
///
/// Le banc ne montre ce panneau qu'au repos, toutes fiches fermées —
/// c'est-à-dire l'état où il n'y a presque rien à mesurer. Le contenu
/// d'une fiche (un nom qu'on change, l'adresse, et jusqu'à deux cents
/// lignes d'octets et de pseudo-code en trois colonnes) n'était donc
/// mesuré nulle part.
///
/// Le contrôle clique vraiment le nom de la fonction, comme on le
/// ferait, puis mesure ce qui est peint : la table paraît, rien ne
/// court au-delà du bord, et deux textes ne se recouvrent pas.
#[test]
fn ouvrir_une_fiche_du_graphe_montre_son_code_sans_deborder() {
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x200], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    // Étroite comme une colonne partagée : c'est la largeur où trois
    // colonnes d'une table de code se marchent dessus.
    const LARGEUR: f32 = 560.0;
    let mut module = namsdec_re::CallGraphModule::default();
    module.on_binary_loaded(&ctx);
    let egui_ctx = egui::Context::default();

    // Le titre de la fiche : il porte l'adresse et « octets », et se
    // trouve SOUS l'arbre — les boîtes de l'arbre, elles, sont peintes
    // sur un canevas et portent les mêmes mots autrement.
    let lignes = relevé_sur(&egui_ctx, &mut module, &ctx, LARGEUR);
    let titre = lignes
        .iter()
        .filter(|l| l.texte.contains("octets") && l.texte.contains("0x"))
        .max_by(|a, b| a.encre.top().total_cmp(&b.encre.top()))
        .map(|l| l.encre.center())
        .expect("le nom d'une fonction doit se lire sous « Le détail des fonctions »");
    let avant = lignes.len();

    let clic = |où: egui::Pos2, module: &mut dyn ReModule| {
        for pressé in [true, false] {
            let mut entrée = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(LARGEUR, 2400.0),
                )),
                ..Default::default()
            };
            if pressé {
                entrée.events.push(egui::Event::PointerMoved(où));
            }
            entrée.events.push(egui::Event::PointerButton {
                pos: où,
                button: egui::PointerButton::Primary,
                pressed: pressé,
                modifiers: egui::Modifiers::NONE,
            });
            let mut sortie = egui_ctx.run_ui(entrée, |ui| module.ui(ui, &ctx));
            sortie.textures_delta.clear();
            let _ = module.take_request();
        }
    };
    clic(titre, &mut module);

    let ouvertes = relevé_sur(&egui_ctx, &mut module, &ctx, LARGEUR);
    let dit = |quoi: &str| ouvertes.iter().any(|l| l.texte.contains(quoi));
    assert!(
        ouvertes.len() > avant,
        "la fiche ouverte doit peindre plus que la fermée : {} puis {}",
        avant,
        ouvertes.len()
    );
    assert!(dit("pseudo-code"), "la table de code paraît");
    assert!(dit("Nom :"), "et le champ qui renomme");

    // Rien ne court au-delà d'un bord, et deux textes ne se recouvrent
    // pas : les trois colonnes d'une table de code sont exactement ce
    // qui se marche dessus dans une colonne étroite.
    let rognées: Vec<&str> = ouvertes
        .iter()
        .filter(|l| l.rognée && !l.élidée)
        .map(|l| l.texte.as_str())
        .collect();
    assert!(
        rognées.is_empty(),
        "du texte court au-delà de son bord : {rognées:?}"
    );
    for (i, une) in ouvertes.iter().enumerate() {
        for autre in &ouvertes[i + 1..] {
            if une.clip != autre.clip {
                continue;
            }
            let croisement = une.encre.shrink(1.0).intersect(autre.encre.shrink(1.0));
            assert!(
                croisement.width() <= 0.0 || croisement.height() <= 0.0,
                "« {} » {:?} recouvre « {} » {:?}",
                une.texte,
                une.encre,
                autre.texte,
                autre.encre
            );
        }
    }

    // Et le même clic referme : la fiche est une bascule, comme
    // partout ailleurs.
    clic(titre, &mut module);
    let refermée = relevé_sur(&egui_ctx, &mut module, &ctx, LARGEUR);
    assert!(
        !refermée.iter().any(|l| l.texte.contains("pseudo-code")),
        "un second clic referme la fiche"
    );
}

/// UN TABLEAU QUI TIENT NE S'ÉLIDE JAMAIS — et celui qui déborde
/// n'élide QUE sa colonne la plus large.
///
/// Une grille egui garde la largeur de ses colonnes d'une image sur
/// l'autre. Borner une cellule à « ce qui reste » revient donc à la
/// borner à ce qu'elle mesurait à l'image précédente : on y perd une
/// lettre, la colonne rétrécit d'autant, et l'on recommence. Mesuré sur
/// le panneau des empreintes : à la deuxième image « SHA-256 » devenait
/// « … », et il n'en restait plus rien à la troisième.
///
/// D'où la règle : la borne se tire des largeurs NATURELLES, mesurées
/// sans contrainte, et jamais de ce qui a été peint.
#[test]
fn un_tableau_qui_tient_ne_s_elide_pas() {
    use namsdec_re::vue::{Bloc, Fragment, Vue};
    let binaire = binaire();
    let disasm = namsdec_disasm::disassemble(&binaire[0x100..0x200], BASE + 0x100);
    let cpu = Cpu::new(Memory::new(BASE, 0x1000));
    let ctx = ReContext {
        cpu: &cpu,
        disasm: &disasm,
        source: "",
        binary: &binaire,
        lang: "fr",
        names: None,
        patches: &[],
    };
    // Deux colonnes « nom / valeur », comme le panneau des empreintes.
    let vue = Vue::neuve().bloc(Bloc::Tableau {
        colonnes: vec![],
        rangées: vec![
            vec!["Taille".into(), Fragment::dit("8904 octets").code().into()],
            vec![
                "SHA-256".into(),
                Fragment::dit("d76a20e1f02468fdf0ef7394028a62a2f4a61a232c7a0f86486f9572444d8ff3")
                    .code()
                    .into(),
            ],
            vec!["CRC-32".into(), Fragment::dit("4f44292f").code().into()],
        ],
        rayé: false,
    });
    let peint = |largeur: f32| {
        let egui_ctx = egui::Context::default();
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(largeur, 400.0),
            )),
            ..Default::default()
        };
        // Le texte ET son élision : une galée élidée garde son texte
        // entier — c'est la LIGNE peinte qui s'arrête sur « … ». Lire
        // le texte seul ferait passer le contrôle sans rien regarder.
        let mut textes: Vec<(String, bool)> = Vec::new();
        // QUATRE images, et c'est le cœur du contrôle : le défaut ne se
        // voyait qu'à partir de la deuxième, et s'aggravait ensuite.
        for _ in 0..4 {
            textes.clear();
            let mut sortie = egui_ctx.run_ui(entrée.clone(), |ui| {
                namsdec_re::vue_peinte::peindre_tout(ui, &ctx, &vue);
            });
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(t) = &clipped.shape {
                    textes.push((t.galley.text().to_owned(), t.galley.elided));
                }
            }
            sortie.textures_delta.clear();
        }
        textes
    };

    // Large : tout tient, donc rien n'est coupé.
    let larges = peint(900.0);
    for attendu in ["Taille", "SHA-256", "CRC-32", "8904 octets"] {
        assert!(
            larges.iter().any(|(t, _)| t == attendu),
            "« {attendu} » doit être peint : {larges:?}"
        );
    }
    let coupés: Vec<&str> = larges
        .iter()
        .filter(|(_, élidée)| *élidée)
        .map(|(t, _)| t.as_str())
        .collect();
    assert!(
        coupés.is_empty(),
        "un tableau qui tient ne coupe rien : {coupés:?}"
    );

    // Étroit : l'empreinte — la colonne la plus large — est coupée, et
    // les étiquettes de gauche ne le sont pas. Rogner « SHA-256 » pour
    // faire tenir une empreinte ferait perdre les deux.
    let étroits = peint(260.0);
    for attendu in ["Taille", "SHA-256", "CRC-32"] {
        assert!(
            étroits.iter().any(|(t, élidée)| t == attendu && !élidée),
            "« {attendu} » reste entier même à l'étroit : {étroits:?}"
        );
    }
    assert!(
        étroits
            .iter()
            .any(|(t, élidée)| t.starts_with("d76a20e1") && *élidée),
        "l'empreinte, elle, s'élide : {étroits:?}"
    );
}
