//! Les tests de l'interface.
//!
//! Ils dessinent l'application dans un contexte egui sans fenêtre et
//! regardent ce qui est *peint* : la largeur qu'un panneau prend, les
//! textes posés, les cadres des boutons. C'est ce qui permet de vérifier
//! une mise en page sans capture d'écran et sans clic simulé.
//!
//! Sortis de `main.rs`, qui portait autant de tests que de code.
//!
//! Découpés par thème le 22 octobre 2026 : cinq cent cinquante-quatre
//! contrôles et trente-trois mille lignes se suivaient ici d'un bloc, un
//! quart du code du dépôt dans un fichier. Ce qui reste est ce dont TOUS
//! les thèmes se servent — les scènes du banc, l'écran de mesure, les
//! binaires d'épreuve — et les trois contrôles qui regardent le dépôt
//! lui-même.
//!
//! ET LE MOTIF INVOQUÉ POUR CE DÉCOUPAGE NE TENAIT PAS. Il est écrit ici
//! plutôt qu'oublié : « chaque contrôle modifié recompile les trente et un
//! mille lignes ». Mesuré avant d'y toucher, une ligne ajoutée dans
//! `tests.rs` coûte 5,74 s ; la même ligne dans `timeline.rs`, 5,65 s.
//! L'unité de compilation est le *crate*, non le fichier, et rustc
//! s'arrête déjà aux fonctions qui ont changé. Le découpage ne fait rien
//! gagner à la compilation, et c'est tout ce que cette mesure dit.
//!
//! Il est fait pour la seule raison qui restait, et elle suffit : on
//! retrouve un contrôle, et surtout on voit ce qui n'en a pas. Vingt
//! fichiers nommés par ce qu'ils gardent disent d'un coup d'œil que le
//! châssis en porte dix et l'exécution quarante-sept.

use super::*;

// Le graphe vit dans son propre module depuis le 29 septembre 2026 ;
// deux de ses fonctions libres se mesurent ici.
use crate::flow::{flow_box_width, flow_ellipsis};

// L'éditeur aussi a son module ; le banc mesure trois de ses
// fonctions libres.
use crate::editor::{editor_width, layout_code, word_at};

// Les menus aussi ; le banc appelle la liste des commandes.
use crate::menus::palette_commands;

// L'affichage de la calculatrice a son module, distinct de son
// calcul ; le banc mesure son clavier.
use crate::calc_ui::{CALC_LARGEUR, TOUCHE_H, TOUCHE_L, écriture_de_touche};

// Le panneau latéral a ses propres fonctions : les tests les
// nomment sans préfixe, comme avant leur déménagement.
use crate::sidebar::*;

// Les thèmes. Ce sont des modules et non des fichiers indépendants :
// tout ce qui est partagé reste ci-dessous, et leur `use super::*` le
// leur donne — y compris les `use` de cette page.
mod aide;
mod analyse;
mod banc;
mod calculatrice;
mod chassis;
mod commandes;
mod console;
mod editeur;
mod execution;
mod exercices;
mod fenetres;
mod fichiers;
mod flux;
mod listing;
mod memoire;
mod outils;
mod panneau;
mod performance;
mod preferences;
mod processeur;
mod vues;

/// TOUT ce qui dessine l'interface, fichier par fichier.
///
/// Quelques contrôles lisent le code au lieu de le peindre — un bouton
/// posé sans cadre, deux styles de texte qui s'écrasent — parce que la
/// règle est dans le code et non dans les pixels. Ils lisaient
/// `main.rs`, ce qui était vrai tant que tout y vivait.
///
/// Le découpage de septembre 2026 a changé cela en silence : une
/// fonction sortie de `main.rs` sortait aussi de leur regard. Rien n'a
/// rougi — et c'est la pire façon dont un contrôle peut mentir, celle
/// qui ne se voit pas, puisqu'il passe *mieux* qu'avant.
///
/// La liste est donc écrite une fois, ici, et
/// [`la_liste_des_sources_ne_rate_aucun_module`] vérifie qu'elle suit
/// les `mod` de `main.rs`.
const SOURCES_DE_L_INTERFACE: &[(&str, &str)] = &[
    ("main.rs", include_str!("main.rs")),
    ("ansi.rs", include_str!("ansi.rs")),
    ("bugreport.rs", include_str!("bugreport.rs")),
    ("calc.rs", include_str!("calc.rs")),
    ("calc_ui.rs", include_str!("calc_ui.rs")),
    ("chrome.rs", include_str!("chrome.rs")),
    ("console.rs", include_str!("console.rs")),
    ("completion.rs", include_str!("completion.rs")),
    ("condition.rs", include_str!("condition.rs")),
    ("editor.rs", include_str!("editor.rs")),
    ("editing.rs", include_str!("editing.rs")),
    ("enums.rs", include_str!("enums.rs")),
    ("errors.rs", include_str!("errors.rs")),
    ("exercises.rs", include_str!("exercises.rs")),
    ("exports.rs", include_str!("exports.rs")),
    ("fixes.rs", include_str!("fixes.rs")),
    ("files.rs", include_str!("files.rs")),
    ("flow.rs", include_str!("flow.rs")),
    ("help.rs", include_str!("help.rs")),
    ("icons.rs", include_str!("icons.rs")),
    ("i18n.rs", include_str!("i18n.rs")),
    ("labels.rs", include_str!("labels.rs")),
    ("keybindings.rs", include_str!("keybindings.rs")),
    ("jobs.rs", include_str!("jobs.rs")),
    ("limites.rs", include_str!("limites.rs")),
    ("listing.rs", include_str!("listing.rs")),
    ("loader.rs", include_str!("loader.rs")),
    ("maximize.rs", include_str!("maximize.rs")),
    ("marks.rs", include_str!("marks.rs")),
    ("memfind.rs", include_str!("memfind.rs")),
    ("menus.rs", include_str!("menus.rs")),
    ("nasm_errors.rs", include_str!("nasm_errors.rs")),
    ("notes.rs", include_str!("notes.rs")),
    ("occurrences.rs", include_str!("occurrences.rs")),
    ("patching.rs", include_str!("patching.rs")),
    ("preferences.rs", include_str!("preferences.rs")),
    ("prevision.rs", include_str!("prevision.rs")),
    ("profiling.rs", include_str!("profiling.rs")),
    ("readings.rs", include_str!("readings.rs")),
    ("resources.rs", include_str!("resources.rs")),
    ("roles.rs", include_str!("roles.rs")),
    ("run.rs", include_str!("run.rs")),
    ("settings.rs", include_str!("settings.rs")),
    ("session.rs", include_str!("session.rs")),
    ("sidebar.rs", include_str!("sidebar.rs")),
    ("statusbar.rs", include_str!("statusbar.rs")),
    ("syntax.rs", include_str!("syntax.rs")),
    ("text.rs", include_str!("text.rs")),
    ("timeline.rs", include_str!("timeline.rs")),
    ("tools.rs", include_str!("tools.rs")),
    ("vue_listing.rs", include_str!("vue_listing.rs")),
    ("vues.rs", include_str!("vues.rs")),
    ("widgets.rs", include_str!("widgets.rs")),
    ("windows.rs", include_str!("windows.rs")),
];

/// TOUT ce qui porte des contrôles, fichier par fichier.
///
/// Trois contrôles lisent la source des contrôles eux-mêmes : la règle
/// qu'ils vérifient est une règle d'écriture, pas de pixels. Ils
/// faisaient `include_str!("tests.rs")`, ce qui était exact tant que
/// tout y vivait.
///
/// Le découpage du 22 octobre 2026 les aurait rendus muets sur
/// dix-neuf fichiers sur vingt — et muets en passant au VERT, qui est
/// la seule façon dont un contrôle peut mentir sans qu'on le voie. La
/// liste est donc écrite une fois, ici, et
/// [`la_liste_des_controles_ne_rate_aucun_theme`] vérifie qu'elle suit
/// les `mod` ci-dessus.
const SOURCES_DU_BANC: &[(&str, &str)] = &[
    ("tests.rs", include_str!("tests.rs")),
    ("tests/aide.rs", include_str!("tests/aide.rs")),
    ("tests/analyse.rs", include_str!("tests/analyse.rs")),
    ("tests/banc.rs", include_str!("tests/banc.rs")),
    (
        "tests/calculatrice.rs",
        include_str!("tests/calculatrice.rs"),
    ),
    ("tests/chassis.rs", include_str!("tests/chassis.rs")),
    ("tests/commandes.rs", include_str!("tests/commandes.rs")),
    ("tests/console.rs", include_str!("tests/console.rs")),
    ("tests/editeur.rs", include_str!("tests/editeur.rs")),
    ("tests/execution.rs", include_str!("tests/execution.rs")),
    ("tests/exercices.rs", include_str!("tests/exercices.rs")),
    ("tests/fenetres.rs", include_str!("tests/fenetres.rs")),
    ("tests/fichiers.rs", include_str!("tests/fichiers.rs")),
    ("tests/flux.rs", include_str!("tests/flux.rs")),
    ("tests/listing.rs", include_str!("tests/listing.rs")),
    ("tests/memoire.rs", include_str!("tests/memoire.rs")),
    ("tests/outils.rs", include_str!("tests/outils.rs")),
    ("tests/panneau.rs", include_str!("tests/panneau.rs")),
    ("tests/performance.rs", include_str!("tests/performance.rs")),
    ("tests/preferences.rs", include_str!("tests/preferences.rs")),
    ("tests/processeur.rs", include_str!("tests/processeur.rs")),
    ("tests/vues.rs", include_str!("tests/vues.rs")),
];

/// La liste ci-dessus n'oublie aucun thème.
///
/// Même raison que [`la_liste_des_sources_ne_rate_aucun_module`], et
/// même conséquence si elle dérive : un thème absent de la liste n'est
/// pas un thème mal contrôlé, c'est un thème que plus personne ne
/// regarde.
#[test]
fn la_liste_des_controles_ne_rate_aucun_theme() {
    let déclarés: Vec<String> = include_str!("tests.rs")
        .lines()
        .filter_map(|l| l.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')))
        .map(|m| format!("tests/{m}.rs"))
        .collect();
    assert!(
        déclarés.len() > 10,
        "le relevé ne trouve presque aucun thème ({}) — il lit au mauvais \
         endroit",
        déclarés.len()
    );
    let oubliés: Vec<&String> = déclarés
        .iter()
        .filter(|m| !SOURCES_DU_BANC.iter().any(|(nom, _)| nom == m))
        .collect();
    assert!(
        oubliés.is_empty(),
        "SOURCES_DU_BANC ignore {} thème(s) — les contrôles qui lisent la \
         source des contrôles ne les verraient pas, et passeraient au vert \
         sans rien regarder :\n  {:?}",
        oubliés.len(),
        oubliés
    );
}

/// Aucun commentaire de documentation n'a perdu sa fonction.
///
/// Rust laisse deux blocs de `///` se suivre : ils se concatènent sur
/// l'item d'après. Donc quand un item disparaît — renommé, supprimé,
/// déplacé sans son commentaire —, le sien reste, se colle au suivant,
/// et **plus rien ne le signale**. Ni le compilateur, ni clippy, ni les
/// cinq cent quatre-vingts contrôles de ce fichier.
///
/// Vingt-cinq blocs étaient dans ce cas, dont vingt-deux antérieurs au
/// découpage de `main.rs`. `module_card` était annoncé par le
/// commentaire de la barre de statut ; `flag_chip` n'en avait plus ;
/// `can_assemble` portait celui de `wait_for_input`. Un lecteur qui
/// ouvre ce dépôt pour apprendre lit donc la description d'autre chose
/// que ce qu'il a sous les yeux — c'est le contraire de ce qu'on a
/// voulu écrire.
///
/// La règle tient à la forme des blocs de ce dépôt : un résumé, une
/// ligne `///` vide, l'explication. Un bloc qui RECOMMENCE au milieu
/// d'un autre — une phrase finie en bout de ligne, puis une majuscule
/// qui ouvre la ligne suivante, puis un `///` vide juste après — est un
/// bloc collé, donc un commentaire qui a perdu sa fonction.
///
/// Le remède, quand ce contrôle rougit, n'est pas de couper le texte :
/// c'est de retrouver ce que le bloc documentait. `git log -S` sur sa
/// première ligne le dit en une commande.
#[test]
fn aucun_commentaire_n_a_perdu_sa_fonction() {
    // TOUT le dépôt, et non la seule interface : le premier cas trouvé
    // hors de ce crate était dans `cfg.rs`, où la description des
    // couches du graphe annonçait l'export Graphviz. Les fichiers sont
    // lus sur le disque parce qu'ils sont trop nombreux pour
    // `include_str!` — et c'est le dépôt qu'on lit, non la machine.
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut sources: Vec<std::path::PathBuf> = Vec::new();
    let mut à_voir = vec![racine];
    while let Some(dossier) = à_voir.pop() {
        let Ok(entrées) = std::fs::read_dir(&dossier) else {
            continue;
        };
        for entrée in entrées.flatten() {
            let chemin = entrée.path();
            if chemin.is_dir() {
                à_voir.push(chemin);
            } else if chemin.extension().is_some_and(|e| e == "rs")
                // `tests.rs` porte des récits en plusieurs paragraphes —
                // la forme y est trop libre pour que la règle tienne.
                && chemin.file_name().is_some_and(|n| n != "tests.rs")
                // Depuis le 22 octobre 2026, ses thèmes sont dans
                // `src/tests/` : c'est le même fichier éclaté, et la même
                // liberté de forme. Un dossier `tests/` qui n'est PAS sous
                // un `src/` est un essai d'intégration, et reste regardé.
                && !chemin.parent().is_some_and(|p| {
                    p.file_name().is_some_and(|n| n == "tests")
                        && p.parent()
                            .is_some_and(|g| g.file_name().is_some_and(|n| n == "src"))
                })
            {
                sources.push(chemin);
            }
        }
    }
    sources.sort();
    assert!(
        sources.len() > 40,
        "le relevé ne trouve presque aucun fichier ({}) — il regarde au \
         mauvais endroit",
        sources.len()
    );

    let mut collés = Vec::new();
    for chemin in &sources {
        let fichier = chemin
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(chemin)
            .display()
            .to_string();
        let Ok(source) = std::fs::read_to_string(chemin) else {
            continue;
        };
        let lignes: Vec<&str> = source.lines().collect();
        if lignes.len() < 5 {
            continue;
        }
        for i in 1..lignes.len().saturating_sub(3) {
            let (avant, ici) = (lignes[i - 1].trim(), lignes[i].trim());
            let (Some(a), Some(b)) = (avant.strip_prefix("///"), ici.strip_prefix("///")) else {
                continue;
            };
            let (a, b) = (a.trim(), b.trim());
            // Une phrase finie en bout de ligne, une majuscule qui ouvre
            // la suivante. Une puce n'en est pas une : une énumération
            // se poursuit ligne à ligne sans séparation.
            if a.is_empty()
                || b.is_empty()
                || !a.ends_with('.')
                || !b.starts_with(char::is_uppercase)
                || b.starts_with("- ")
                || b.starts_with("* ")
            {
                continue;
            }
            // Et un nouveau bloc commence vraiment : son résumé est
            // suivi d'une ligne `///` vide, alors que le bloc d'avant
            // n'en avait pas.
            let suivantes = (1..4).map(|k| lignes[i + k].trim());
            let précédentes = (1..4).map(|k| lignes[i - 1 - k].trim());
            if suivantes.clone().any(|l| l == "///") && !précédentes.clone().any(|l| l == "///") {
                collés.push(format!("{fichier}:{}\n      …{a}\n      {b}", i + 1));
            }
        }
    }
    assert!(
        collés.is_empty(),
        "{} commentaire(s) collé(s) au suivant — leur fonction a disparu \
         entre les deux, cherchez-la avec `git log -S` :\n  {}",
        collés.len(),
        collés.join("\n  ")
    );
}

/// La liste ci-dessus n'oublie aucun module.
///
/// Sans ce contrôle, la liste serait exacte le jour où elle est écrite
/// et fausse au module suivant — et sa fausseté ferait passer les
/// autres contrôles, non échouer.
#[test]
fn la_liste_des_sources_ne_rate_aucun_module() {
    let déclarés: Vec<String> = include_str!("main.rs")
        .lines()
        .filter_map(|l| l.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')))
        .filter(|m| *m != "tests")
        .map(|m| format!("{m}.rs"))
        .collect();
    let oubliés: Vec<&String> = déclarés
        .iter()
        .filter(|m| !SOURCES_DE_L_INTERFACE.iter().any(|(nom, _)| nom == m))
        .collect();
    assert!(
        oubliés.is_empty(),
        "SOURCES_DE_L_INTERFACE ignore {} module(s) — les contrôles qui \
         lisent le code ne les verraient pas :\n  {:?}",
        oubliés.len(),
        oubliés
    );
}

/// La série livrée, lue dans le dépôt.
///
/// Elle n'est plus compilée dans le programme : c'est un fichier de
/// données. Les tests lisent donc celui **du dépôt** — jamais celui qui
/// est installé sur la machine, dont l'état ne prouverait rien.
fn builtin_pack() -> namsdec_exercises::Pack {
    let packs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../namsdec-exercises/packs");
    namsdec_exercises::builtin_pack_in(&packs).expect("la série livrée est lisible")
}

/// Une application qui connaît la série livrée.
///
/// `NamsdecApp::default()` n'en charge aucune — c'est le démarrage qui
/// lit les fichiers —, donc un test qui parle d'exercices la lui donne.
fn app_with_exercises() -> NamsdecApp {
    NamsdecApp {
        exercises: true,
        exercise_packs: vec![builtin_pack()],
        ..Default::default()
    }
}

/// Le fichier d'exemple, ou une copie minimale quand le plugin n'est
/// pas sur cette machine : le test ne dépend pas de ce qui traîne
/// hors du dépôt.
fn return_sample() -> String {
    include_str!("../../namsdec-theme/tests/abyss.json").to_owned()
}

/// Le dossier de données est une variable d'environnement, donc partagée
/// par tous les tests du binaire. Ceux qui y touchent prennent ce verrou :
/// sans lui, deux tests en parallèle se volent le dossier de l'autre.
static DATA_HOME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Détourne le dossier de données vers `dir` le temps du test, pour que
/// rien ne soit écrit chez l'utilisateur.
fn with_data_home<T>(dir: &std::path::Path, body: impl FnOnce() -> T) -> T {
    let _guard = DATA_HOME.lock().unwrap_or_else(|e| e.into_inner());
    let previous = std::env::var_os("XDG_DATA_HOME");
    unsafe { std::env::set_var("XDG_DATA_HOME", dir) };
    let result = body();
    match previous {
        Some(value) => unsafe { std::env::set_var("XDG_DATA_HOME", value) },
        None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
    }
    result
}

/// Ce que l'interface écrit vraiment à l'écran, deux images durant —
/// la première sert à egui pour mesurer, la seconde montre le résultat.
fn paint(app: &mut NamsdecApp) -> String {
    let ctx = egui::Context::default();
    // Le thème de l'application, et non celui d'egui. Sans lui, on
    // mesure des espacements et des polices qui n'existent nulle part —
    // c'est l'un des quatre défauts qui sont passés sous des tests qui
    // paraissaient les couvrir (A 7.6).
    app.apply_theme(&ctx);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };
    // Trois images, et non deux : une fenêtre dont la hauteur épouse son
    // contenu ne connaît sa taille qu'après l'avoir disposé, et ne se
    // pose donc qu'à la troisième. Deux suffisaient tant que toutes les
    // fenêtres avaient une taille imposée ; ce n'est plus le cas.
    let mut painted = String::new();
    for _ in 0..3 {
        painted.clear();
        let mut output = ctx.run_ui(input.clone(), |ui| app.draw(ui));
        for clipped in &output.shapes {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                painted.push_str(text.galley.text());
                painted.push('|');
            }
        }
        output.textures_delta.clear();
    }
    painted
}

/// Le dossier des binaires d'épreuve : `tests/binaires/` du dépôt.
///
/// Voir son LISEZMOI. `NAMSDEC_TEST_BINARIES` le déplace ailleurs, pour
/// qui garde ses programmes hors du dépôt.
///
/// Un seul contrôle le lit désormais, et c'est sa raison d'être :
/// `every_binary_here_loads_and_runs`, qui passe en revue ce que
/// chacun y dépose. Tout le reste travaille sur
/// [`binaire_fabriqué`], le même partout.
fn dossier_dépreuve() -> Option<std::path::PathBuf> {
    if let Some(ailleurs) = std::env::var_os("NAMSDEC_TEST_BINARIES") {
        let chemin = std::path::PathBuf::from(ailleurs);
        return chemin.is_dir().then_some(chemin);
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/binaires")
        .canonicalize()
        .ok()
}

/// Le programme du binaire d'épreuve FABRIQUÉ : il demande un mot de
/// passe, le lit sur son entrée, le compare octet par octet et répond.
///
/// Écrit pour porter ce dont les contrôles ont besoin d'un vrai
/// programme, et rien de plus : deux sections, des chaînes dans les
/// données, un appel et son retour, une boucle de comparaison, deux
/// branches qui se rejoignent, un `read` qui attend vraiment, et un
/// code de sortie qui dit laquelle des deux branches a été prise.
const PROGRAMME_FABRIQUÉ: &str = "\
global _start

section .data
invite      db \"Enter the password: \"
invite_len  equ $ - invite
secret      db \"ouvre-toi\"
secret_len  equ $ - secret
gagne       db \"Correct password\", 10
gagne_len   equ $ - gagne
perdu       db \"Incorrect password\", 10
perdu_len   equ $ - perdu
saisie      times 64 db 0

section .text
_start:
    mov rsi, invite
    mov rdx, invite_len
    call ecrire

    xor eax, eax
    xor edi, edi
    mov rsi, saisie
    mov rdx, 64
    syscall
    mov r8, rax

    mov rcx, secret_len
    mov rsi, secret
    mov rdi, saisie
compare:
    mov al, [rsi]
    cmp al, [rdi]
    jne echec
    inc rsi
    inc rdi
    dec rcx
    jnz compare

    mov rsi, gagne
    mov rdx, gagne_len
    call ecrire
    xor edi, edi
    jmp sortir

echec:
    mov rsi, perdu
    mov rdx, perdu_len
    call ecrire
    mov edi, 1

sortir:
    mov eax, 60
    syscall

ecrire:
    mov eax, 1
    mov edi, 1
    syscall
    ret
";

/// Un vrai binaire, le MÊME sur toute machine, bâti par nasm et ld au
/// moment du contrôle.
///
/// `tests/binaires/` n'est pas versionné, et cette décision tient : un
/// dépôt n'embarque pas d'exécutables. Mais elle laissait trente-sept
/// contrôles s'abstenir EN SILENCE là où le dossier est vide — donc sur
/// la CI, où ils passaient au vert sans rien ouvrir. Mesuré le 22
/// octobre 2026 en faisant crier le dossier absent.
///
/// Un binaire fabriqué vaut mieux ici qu'un binaire trouvé : il est le
/// même partout, donc le contrôle dit la même chose partout. Le corpus
/// de chacun garde son propre contrôle —
/// `every_binary_here_loads_and_runs` —, qui est sa raison d'être :
/// éprouver namsdec sur ce que personne n'avait prévu.
///
/// `None` seulement quand nasm ou ld manquent : se taire alors n'est pas
/// s'adapter, c'est l'absence d'un outil.
fn binaire_fabriqué() -> Option<Vec<u8>> {
    static OCTETS: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    OCTETS
        .get_or_init(|| {
            if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
                return None;
            }
            // Le chemin réel de namsdec, non un appel direct à nasm :
            // ce qui est éprouvé derrière est aussi la chaîne que
            // l'application emploie pour livrer un programme.
            let dossier = tempfile::tempdir().ok()?;
            let chemin = dossier.path().join("crackme-fabrique");
            let mut app = NamsdecApp {
                source: PROGRAMME_FABRIQUÉ.to_owned(),
                ..Default::default()
            };
            app.do_assemble();
            app.export_binary(&chemin);
            std::fs::read(&chemin).ok()
        })
        .clone()
}

/// Le nom sous lequel le binaire fabriqué se charge.
///
/// Les contrôles le passaient en clair — `"crackme-01"` —, ce qui était
/// vrai quand c'était ce fichier-là. Un nom en clair sur des octets
/// fabriqués serait faux, et ce qui se lit à l'écran le répéterait.
const NOM_FABRIQUÉ: &str = "crackme-fabrique";

/// Combien d'instructions porte le binaire fabriqué LONG.
///
/// Trois contrôles ne mesurent quelque chose que sur un gros listing :
/// ne pas mettre en page le fichier entier, ne relever les occurrences
/// qu'une fois, retrouver la ligne qu'on lisait au millier. Le seuil
/// Deux seuils sont exigés, et c'est le plus haut qui commande : dix
/// mille occurrences de « mov » pour le relevé de recherche, et plus de
/// vingt mille LIGNES pour le profil d'un gros binaire. Vingt-quatre
/// mille instructions donnent les deux avec de la marge, et coûtent une
/// fraction de seconde de nasm, une fois par processus.
const INSTRUCTIONS_DU_LONG: usize = 24_000;

/// Le même binaire, mais long : de quoi mesurer ce qui ne se voit que
/// sur un gros fichier.
///
/// Douze mille `mov` d'affilée ne ressemblent pas à du vrai code, et
/// c'est assumé : ce que les trois contrôles concernés mesurent est un
/// COMPTE de poses et de relevés, qui ne dépend pas de la variété des
/// instructions. Ce qui dépendrait d'elle est mesuré ailleurs, sur le
/// corpus de chacun.
fn binaire_fabriqué_long() -> Option<Vec<u8>> {
    static OCTETS: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    OCTETS
        .get_or_init(|| {
            if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
                return None;
            }
            let mut source = String::from("global _start\n\nsection .text\n_start:\n");
            for i in 0..INSTRUCTIONS_DU_LONG {
                // Deux formes qui alternent : un listing d'une seule
                // ligne répétée se mettrait en page comme une ligne.
                if i % 2 == 0 {
                    source.push_str("    mov rax, rbx\n");
                } else {
                    source.push_str("    mov rcx, rdx\n");
                }
            }
            source.push_str("    mov eax, 60\n    xor edi, edi\n    syscall\n");
            let dossier = tempfile::tempdir().ok()?;
            let chemin = dossier.path().join("crackme-fabrique-long");
            let mut app = NamsdecApp {
                source,
                ..Default::default()
            };
            app.do_assemble();
            app.export_binary(&chemin);
            std::fs::read(&chemin).ok()
        })
        .clone()
}

/// Le binaire fabriqué EST un vrai binaire — et ce contrôle est la
/// clef de voûte des trente-sept autres.
///
/// Si la fabrication se rompait — nasm qui change d'avis, une directive
/// refusée, `export_binary` qui n'écrit plus —, `binaire_fabriqué`
/// rendrait `None` et les trente-sept contrôles qui s'en servent
/// s'abstiendraient EN SILENCE, exactement comme avant lui. C'est
/// précisément le défaut qu'on vient de retirer ; il faut donc quelqu'un
/// pour le guetter.
#[test]
fn le_binaire_fabrique_est_un_vrai_binaire() {
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    for (quoi, octets) in [
        ("le court", binaire_fabriqué()),
        ("le long", binaire_fabriqué_long()),
    ] {
        let octets = octets.unwrap_or_else(|| {
            panic!(
                "{quoi} ne se fabrique plus — trente-sept contrôles viennent \
                    de devenir muets sans rien dire"
            )
        });
        assert!(octets.starts_with(b"\x7fELF"), "{quoi} est un ELF");
        let mut app = NamsdecApp::default();
        assert!(app.load_elf(&octets, NOM_FABRIQUÉ), "{quoi} se charge");
        let lignes = app.source.lines().count();
        assert!(lignes > 10, "{quoi} se désassemble : {lignes} lignes");
        app.do_step();
        assert!(
            app.emulator.as_ref().is_some_and(|e| e.cpu().regs.rip != 0),
            "{quoi} avance d'un pas"
        );
    }

    // Et le long est vraiment long : c'est tout ce que les trois
    // contrôles de relevés lui demandent.
    let long = binaire_fabriqué_long().expect("le long se fabrique");
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&long, NOM_FABRIQUÉ));
    let movs = app.source.matches("mov").count();
    assert!(movs > 10_000, "{movs} occurrences de « mov »");
    // Et le second seuil, celui du profil d'un gros binaire : les deux
    // sont écrits ici, là où la longueur est décidée, et non répartis
    // chez ceux qui en dépendent.
    let lignes = app.source.lines().count();
    assert!(lignes > 20_000, "{lignes} lignes de listing");

    // Le court, lui, demande vraiment une entrée : c'est ce que les
    // contrôles de la console éprouvent sur lui.
    let court = binaire_fabriqué().expect("le court se fabrique");
    let mut app = NamsdecApp::default();
    assert!(app.load_elf(&court, NOM_FABRIQUÉ));
    app.do_run();
    for _ in 0..4000 {
        if !app.running {
            break;
        }
        app.tick_run(&egui::Context::default());
    }
    assert!(
        matches!(app.status, Status::WaitingForInput),
        "il attend son mot de passe : {:?}",
        app.status
    );
    let écrit =
        String::from_utf8_lossy(app.emulator.as_ref().expect("un émulateur").output()).to_string();
    assert!(
        écrit.contains("Enter the password:"),
        "et il a demandé : {écrit:?}"
    );
}

/// Lance un binaire qu'on vient d'écrire, en attendant qu'il se libère.
///
/// Ce test était le seul instable du dépôt : il tombait une fois sur
/// vingt, et seulement dans un `cargo test --workspace` complet. La
/// cause n'est pas dans le binaire produit, elle est chez les voisins.
/// Des dizaines de tests tournent en parallèle **dans le même
/// processus**, et plusieurs lancent `nasm` ou `ld`. Quand l'un d'eux
/// fait son `fork` pendant qu'on tient ici un fichier ouvert en
/// écriture, le fils hérite d'une copie du descripteur ; tant qu'il n'a
/// pas fait son `exec` — quelques microsecondes —, le noyau compte
/// encore une écriture ouverte sur notre fichier et refuse de
/// l'exécuter : `ETXTBSY`. Le programme est bon, c'est le voisin qui
/// tient encore la porte.
///
/// On attend donc, et seulement pour cette raison-là : toute autre
/// erreur remonte au premier tour, sans délai ajouté. Une seconde de
/// patience au total, ce qu'aucune de ces courses n'a jamais approché.
fn lance_ce_qui_vient_d_etre_ecrit(chemin: &std::path::Path) -> std::process::ExitStatus {
    let mut attente = std::time::Duration::from_millis(1);
    for _ in 0..12 {
        match std::process::Command::new(chemin).status() {
            Ok(statut) => return statut,
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(attente);
                attente *= 2;
            }
            Err(e) => panic!("le binaire exporté se lance : {e}"),
        }
    }
    panic!("le binaire exporté reste « occupé » après plusieurs secondes");
}

/// Ce mot est-il un nom de registre, sous n'importe quelle largeur ?
///
/// Les seize généraux et leurs moitiés, les vectoriels, la pile x87. Ils
/// n'ont pas chacun leur fiche — c'est la notion « registre » qui les
/// explique —, et un test de couverture du manuel n'a pas à les
/// réclamer.
fn est_un_registre(mot: &str) -> bool {
    const CLASSIQUES: [&str; 8] = ["ax", "bx", "cx", "dx", "si", "di", "bp", "sp"];
    let mot = mot.to_ascii_lowercase();
    if let Some(reste) = mot.strip_prefix('r') {
        // r8 … r15, avec leurs suffixes d, w, b.
        let chiffres = reste.trim_end_matches(['d', 'w', 'b']);
        if !chiffres.is_empty() && chiffres.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    for base in CLASSIQUES {
        for forme in [
            format!("r{base}"),
            format!("e{base}"),
            base.to_owned(),
            format!("{}l", &base[..1]),
            format!("{}h", &base[..1]),
            format!("{base}l"),
        ] {
            if mot == forme {
                return true;
            }
        }
    }
    (mot.starts_with("xmm") || mot.starts_with("ymm") || mot.starts_with("st"))
        && mot.chars().any(|c| c.is_ascii_digit())
}

/// Compile un petit programme C, et rend son binaire.
///
/// `None` quand la machine n'a pas de quoi — pas de `gcc`, pas de
/// bibliothèque C statique. Ce n'est pas un échec de namsdec, et le test
/// qui s'en sert se tait alors plutôt que d'échouer ailleurs.
fn compile_c(source: &str, options: &[&str]) -> Option<(tempfile::TempDir, std::path::PathBuf)> {
    let dir = tempfile::tempdir().ok()?;
    let chemin = dir.path().join("essai.c");
    std::fs::write(&chemin, source).ok()?;
    let binaire = dir.path().join("essai");
    let sortie = std::process::Command::new("gcc")
        .args(options)
        .arg("-O1")
        .arg("-o")
        .arg(&binaire)
        .arg(&chemin)
        .output()
        .ok()?;
    if !sortie.status.success() {
        eprintln!(
            "gcc n'a pas pu compiler ({}) : test ignoré",
            String::from_utf8_lossy(&sortie.stderr)
                .lines()
                .next()
                .unwrap_or("")
        );
        return None;
    }
    Some((dir, binaire))
}

// ============================================================
//  Le banc de mise en page
// ============================================================
//
// Quatre défauts de suite ont été trouvés par l'œil et non par la suite
// de tests : des textes qui se recouvraient, une fenêtre qui montait
// toute seule, un champ de saisie absent, une colonne centrée au lieu
// d'être alignée. Aucun n'était subtil ; tous étaient visibles à la
// première capture d'écran. Ce qui manquait n'était pas des tests — il
// y en a des centaines — mais des tests qui regardent la *mise en
// page* au lieu de regarder un détail choisi d'avance.
//
// D'où ce banc : une liste de fenêtres, déclarée une fois, et des
// invariants qui valent pour toutes. Ajouter une fenêtre à la liste la
// soumet d'un coup à tout ce qui suit ; et un invariant nouveau
// s'applique aussitôt aux quatorze.

/// Comment ouvrir une fenêtre : son nom dans l'échec, son identifiant
/// egui, et ce qu'il faut poser dans l'application pour qu'elle
/// s'affiche.
type Ouverture = (&'static str, &'static str, fn(&mut NamsdecApp));

/// Les fenêtres qu'un démarrage nu sait ouvrir.
///
/// Une seule liste, et non une par test : c'est ce qui fait qu'une
/// fenêtre nouvelle est contrôlée sans qu'on y pense. Celles qui
/// demandent un binaire chargé ou une exécution en cours n'y sont pas —
/// elles se contrôlent séparément, avec ce qu'il leur faut.
fn fenêtres_ouvrables() -> Vec<Ouverture> {
    vec![
        // Les cinq pages des préférences, et non la seule première :
        // seul l'onglet Général s'ouvrait, et les quatre autres
        // n'avaient jamais été mesurées. Le tableau des raccourcis, qui
        // montre les deux jeux côte à côte, est la plus large grille de
        // l'application — exactement le genre de chose que le banc est
        // fait pour attraper.
        ("préférences", "preferences_window", |app| {
            app.show_preferences = true;
        }),
        ("à propos", ABOUT_WINDOW, |app| app.show_about = true),
        ("calculatrice", "calculator_window", |app| {
            app.show_calculator = true;
        }),
        ("aide", "instruction_help", |app| app.show_help = true),
        ("registres étendus", "vectors_window", |app| {
            app.show_vectors = true;
        }),
        ("points d'arrêt", "breakpoints_window", |app| {
            app.show_breakpoints = true;
        }),
        ("chronologie", "timeline_window", |app| {
            app.show_timeline = true;
        }),
        ("motif cyclique", "cyclic_window", |app| {
            app.show_cyclic = true
        }),
        ("analyse", "analysis_window", |app| app.show_analysis = true),
        ("désassemblage", "disassembly_window", |app| {
            app.show_disassembly = true;
        }),
        ("graphe", "flow_window", |app| app.show_flow = true),
        ("décompilateur", "decompiler_window", |app| {
            app.show_decompiler_module = true;
            app.show_decompiler = true;
        }),
        ("saisie du programme", INPUT_DIALOG, |app| {
            app.show_input_dialog = true;
        }),
        ("correctifs", PATCHES_WINDOW, |app| app.show_patches = true),
        // LA FENÊTRE DES EXERCICES N'Y ÉTAIT PAS, et c'est la plus
        // longue liste que namsdec sache peindre : cent huit titres,
        // leur énoncé, leur palier, leurs fiches utiles. Aucun
        // invariant ne l'avait jamais regardée — ni recouvrement, ni
        // découpage, ni taille —, et le parcours vient d'y gagner douze
        // entrées.
        ("exercices", "exercises_window_v2", |app| {
            app.exercises = true;
            app.exercise_packs = vec![builtin_pack()];
            app.show_exercises = true;
        }),
        // LA PALETTE N'ÉTAIT PAS AU BANC, et c'est une fenêtre comme
        // les autres : elle a une largeur fixe, une liste qui défile et
        // un champ de saisie. Elle y entre le jour où ses lignes se
        // mettent à porter DEUX textes — le libellé et la frappe —,
        // c'est-à-dire le jour où elles peuvent se marcher dessus.
        ("palette de commandes", "command_palette", |app| {
            app.command_palette = Some(CommandPalette::default());
        }),
        // Les suites possibles d'une séquence armée : vingt lignes, et
        // la seule surface du programme qui n'apparaisse qu'au clavier.
        // La date d'armement est reculée d'une seconde pleine — le
        // panneau ne se montre qu'après une hésitation, et une scène ne
        // doit pas dépendre de la vitesse de la machine.
        ("suites d'une séquence", "leader_hints", |app| {
            app.dispatcher.arm(&[egui::Key::Space]);
            app.leader_since = Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
        }),
    ]
}

/// Une scène à contrôler : son nom, la fenêtre où regarder (`None`
/// pour l'application entière), et ce qu'il faut poser pour l'obtenir.
///
/// Les fenêtres ne sont pas tout : la barre d'outils, le panneau
/// latéral et la barre d'état sont sous les yeux en permanence, et
/// c'est là qu'on passe ses journées. Une règle qui ne vaudrait que
/// pour les fenêtres laisserait l'essentiel hors contrôle.
type Scène = (String, Option<&'static str>, Box<dyn Fn(&mut NamsdecApp)>);

/// Le programme du banc : court, et il touche à tout ce que
/// l'interface sait montrer.
///
/// Deux sections, un appel système, une pile qu'on remue, une boucle.
/// Assemblé et avancé de quelques pas, il remplit les registres, la
/// pile, le listing, le graphe et la chronologie — c'est-à-dire tout
/// ce qui reste vide dans une application qu'on vient d'ouvrir, et
/// donc tout ce que le banc ne regardait pas.
const PROGRAMME_DU_BANC: &str = "\
section .data
message db \"bonjour\", 10
longueur equ $ - message

section .text
global _start
_start:
    mov rax, 1
    mov rdi, 1
    mov rsi, message
    mov rdx, longueur
    syscall
    push rax
    mov rcx, 3
boucle:
    dec rcx
    jnz boucle
    pop rax
    mov rax, 60
    xor rdi, rdi
    syscall
";

/// Un programme qui touche quatre des sept simplifications du
/// coprocesseur : la précision étendue (toute addition y suffit), le
/// format 80 bits en mémoire, une transcendante, et le mot d'état.
///
/// Quatre et non une, parce que c'est la rubrique DÉPLIÉE qu'on veut
/// regarder : une ligne tient partout, quatre titres suivis de quatre
/// explications repliées sont ce qui déborde.
const PROGRAMME_QUI_SIMPLIFIE: &str = "\
section .data
angle  dq 1.0
sortie dq 0.0
etendu dt 0.0

section .text
global _start
_start:
    fld qword [angle]
    fadd qword [angle]
    fstp tword [etendu]
    fld tword [etendu]
    fsin
    fstsw ax
    fstp qword [sortie]
    hlt
";

/// Tout ce que le banc regarde : les quatorze fenêtres et les cinq
/// onglets du panneau, à vide puis programme en marche.
///
/// « En marche » n'est pas un raffinement : une application qu'on vient
/// d'ouvrir n'affiche presque rien, et les défauts de mise en page se
/// révèlent avec le contenu. Cette moitié-là du banc demande nasm ;
/// faute de lui, elle est simplement absente, et le reste tient.
fn scènes() -> Vec<Scène> {
    let mut scènes: Vec<Scène> = Vec::new();
    let en_marche = namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker();
    for marche in [false, true] {
        if marche && !en_marche {
            continue;
        }
        let suffixe = if marche { " (en marche)" } else { "" };
        /// Pose le programme du banc et avance de quelques pas.
        fn démarre(app: &mut NamsdecApp) {
            app.source = PROGRAMME_DU_BANC.to_owned();
            app.do_assemble();
            for _ in 0..8 {
                app.do_step();
            }
        }
        for (nom, id, ouvre) in fenêtres_ouvrables() {
            scènes.push((
                format!("{nom}{suffixe}"),
                Some(id),
                Box::new(move |app| {
                    if marche {
                        démarre(app);
                    }
                    ouvre(app);
                }),
            ));
        }
        // Les pages des préférences, une par une.
        for onglet in PreferencesTab::ALL {
            scènes.push((
                format!("préférences / {}{suffixe}", onglet.id()),
                Some("preferences_window"),
                Box::new(move |app| {
                    if marche {
                        démarre(app);
                    }
                    app.show_preferences = true;
                    app.preferences_tab = onglet;
                }),
            ));
        }
        for onglet in SidebarTab::ALL {
            scènes.push((
                format!("application / {}{suffixe}", onglet.id()),
                None,
                Box::new(move |app| {
                    if marche {
                        démarre(app);
                    }
                    app.show_sidebar = true;
                    // L'ONGLET RE EST MASQUÉ PAR DÉFAUT, et le panneau
                    // retombe en silence sur le processeur quand on le
                    // demande sans l'avoir allumé (`sidebar_ui`, en
                    // tête). La scène « application / re » mesurait donc
                    // l'onglet Processeur une seconde fois, sous un autre
                    // nom, dans les deux moitiés du banc : un onglet
                    // entier hors de contrôle depuis que l'option
                    // existe, et rien ne rougissait.
                    //
                    // C'est la pire espèce de trou — un contrôle vert
                    // qui ne regarde pas ce qu'il nomme. Il est retenu
                    // par `chaque_scene_d_onglet_montre_bien_son_onglet`.
                    app.show_re_tab = onglet == SidebarTab::Re || app.show_re_tab;
                    app.active_tab = onglet;
                }),
            ));
        }
        // LES TROIS ONGLETS DE LA CONSOLE, qui n'étaient dans aucune
        // scène. Ils sont à l'écran en permanence — « Sortie »,
        // « Diagnostics », « Profil » — et rien ne les mesurait : ni le
        // banc, ni les captures, puisque la console s'ouvre sur le
        // premier et que les deux autres n'apparaissent qu'au clic.
        //
        // C'est un recensement des états vides qui a révélé le trou : on
        // cherchait des panneaux mal présentés, on a trouvé des panneaux
        // qu'on ne regardait pas du tout.
        for onglet in ConsoleTab::ALL {
            scènes.push((
                format!("console / {onglet:?}{suffixe}"),
                None,
                Box::new(move |app| {
                    if marche {
                        démarre(app);
                    }
                    app.show_console = true;
                    app.console_tab = onglet;
                }),
            ));
        }
        // L'ONGLET DIAGNOSTICS AVEC SA RUBRIQUE DES SIMPLIFICATIONS.
        // La scène « console / Diagnostics » ci-dessus ne la montre
        // pas : le programme du banc ne touche pas le coprocesseur, et
        // la rubrique ne paraît que si l'exécution l'a touchée — ce qui
        // est voulu, mais laisse sans mesure le seul état où elle existe.
        // Celui-ci exécute un `fsin`, et la rubrique se peint avec ses
        // titres et ses explications sur plusieurs lignes : c'est l'état
        // où un contenu neuf vole la place des diagnostics de nasm.
        if marche {
            scènes.push((
                "console / Diagnostics, simplifications".to_owned(),
                None,
                Box::new(|app| {
                    app.source = PROGRAMME_QUI_SIMPLIFIE.to_owned();
                    app.do_assemble();
                    for _ in 0..40 {
                        app.do_step();
                    }
                    app.show_console = true;
                    app.console_tab = ConsoleTab::Diagnostics;
                }),
            ));
        }
        // Les vues de la zone centrale, une par une.
        //
        // Les neuf vues ont quitté leurs fenêtres pour la rangée : elles
        // se peignent désormais dans la place de l'éditeur, qui n'a ni
        // cadre ni marge de fenêtre pour les tenir. Les invariants du
        // banc valent là comme ailleurs — et c'est précisément en
        // passant d'une fenêtre à un panneau qu'un contenu découvre
        // qu'il touchait le bord.
        for vue in VueCentrale::ALL {
            scènes.push((
                format!("vue / {}{suffixe}", vue.id()),
                None,
                Box::new(move |app| {
                    if marche {
                        démarre(app);
                    }
                    // LA DÉCOMPILATION EST UN MODULE QU'ON ALLUME, et
                    // sans lui elle n'a pas d'onglet : `montrer_vue`
                    // refuse, et la scène retomberait en silence sur la
                    // source — elle nommerait une vue et en mesurerait
                    // une autre, exactement comme l'onglet RE du
                    // panneau l'a fait pendant des semaines. Retenu par
                    // `chaque_scene_de_vue_montre_bien_sa_vue`.
                    if vue == VueCentrale::Decompiler {
                        app.show_decompiler_module = true;
                    }
                    app.montrer_vue(vue);
                }),
            ));
        }
        // La fenêtre d'analyse, UN MODULE À LA FOIS. Ouverte par
        // `fenêtres_ouvrables`, elle ne montrait que son module par défaut :
        // l'éditeur binaire, YARA, le graphe d'appels et les autres
        // passaient au travers des invariants de scène. On les éprouve
        // tous, et un module neuf y entre sans qu'on y pense. Seulement « en
        // marche » : sans binaire, un module d'analyse n'écrit presque rien.
        if marche {
            let ids: Vec<String> = NamsdecApp::default()
                .re_registry
                .entries("fr")
                .iter()
                .map(|(id, _)| (*id).to_owned())
                .collect();
            for id in ids {
                scènes.push((
                    format!("analyse / {id}{suffixe}"),
                    Some("analysis_window"),
                    Box::new(move |app| {
                        démarre(app);
                        app.show_analysis = true;
                        app.analysis_module = id.clone();
                    }),
                ));
            }
        }
    }
    // L'aide sur une FICHE, et la plus chargée du manuel : vingt-quatre
    // mnémoniques en tête, deux paragraphes dessous. La scène « aide »
    // n'ouvrait que le sommaire, si bien qu'aucun invariant ne regardait
    // jamais une fiche peinte — alors que c'est ce qu'on lit sous F1.
    scènes.push((
        "aide / une fiche chargée".to_owned(),
        Some("instruction_help"),
        Box::new(|app| {
            app.show_help = true;
            app.help = Some(("pminub".to_owned(), namsdec_doc::lookup("pminub")));
        }),
    ));
    // Un compteur en attente : `5` tapé, le mouvement pas encore venu.
    // C'est une forme de plus dans la rangée la plus serrée de l'écran —
    // la barre d'état —, et les invariants de scène sont ce qui dit
    // qu'elle ne pousse ni ne recouvre rien. Quatre chiffres, qui est le
    // plus large qu'elle puisse porter.
    // LA BOÎTE DE SAISIE QUAND LE PROGRAMME ATTEND VRAIMENT.
    //
    // La scène « saisie du programme » ouvre la boîte, mais sur un
    // programme qui n'attend rien : elle montre alors UN bouton, et la
    // rangée des trois — « Fin d'entrée », « Plus tard », « Envoyer » —
    // n'était peinte par aucune scène. C'est là que
    // `two_buttons_side_by_side_share_a_height` a trouvé un 110x25
    // contre un 110x27, posés à huit pixels l'un de l'autre : le bouton
    // principal, le seul sans liseré, était deux pixels plus court que
    // son voisin.
    //
    // Un invariant déjà écrit, un défaut déjà là, et rien entre les
    // deux qu'une scène manquante.
    scènes.push((
        "saisie du programme / le programme attend".to_owned(),
        Some(INPUT_DIALOG),
        Box::new(|app| {
            app.status = Status::WaitingForInput;
            app.show_input_dialog = true;
        }),
    ));
    scènes.push((
        "application / compteur en attente".to_owned(),
        None,
        Box::new(|app| {
            app.show_statusbar = true;
            app.count_pending = Some(4242);
        }),
    ));
    // LA CONSOLE D'UN BINAIRE CHARGÉ, HAUTEUR FIGÉE À LA POIGNÉE.
    //
    // C'est la console la plus chargée que l'application sache peindre :
    // un binaire ouvert y ajoute les arguments et l'environnement, et
    // l'environnement déplié y ajoute encore une aide et un champ de
    // deux lignes. Les trois scènes « console / … » la regardaient sur
    // un programme assemblé, qui n'a ni l'un ni l'autre, et toujours en
    // hauteur automatique — celle que namsdec calcule lui-même.
    //
    // Les deux manques se cumulaient précisément là où la phrase du
    // dessous se faisait couper : hauteur figée, environnement déplié,
    // il n'en restait que trois pixels sur quinze.
    if let Some(octets) = binaire_fabriqué() {
        scènes.push((
            "console / binaire chargé, hauteur figée".to_owned(),
            None,
            Box::new(move |app| {
                app.load_elf(&octets, NOM_FABRIQUÉ);
                app.show_console = true;
                app.console_tab = ConsoleTab::Output;
                app.console_env_open = true;
                // Une hauteur qu'on a tirée soi-même : c'est là que la
                // console cesse de suivre ce qu'elle a à dire, et que
                // le compte de la zone de saisie doit être juste.
                app.console_height = Some(200.0);
            }),
        ));
    }
    // L'ÉDITEUR BINAIRE EN FENÊTRE AGRANDIE : la seule disposition où
    // son inspecteur se pose à côté du vidage, donc celle qu'on veut
    // voir. Elle avait dû être retirée le 28 octobre 2026 — le banc
    // triait les formes d'une fenêtre sur la LARGEUR de leur découpe,
    // et une fenêtre agrandie est aussi large que l'écran, si bien que
    // les textes de l'éditeur de code passaient le tri. Le tri se fait
    // maintenant sur l'ORDRE des formes, et la scène est revenue.
    if let Some(octets) = binaire_fabriqué() {
        scènes.push((
            "analyse / éditeur binaire, fenêtre agrandie".to_owned(),
            Some("analysis_window"),
            Box::new(move |app| {
                app.load_elf(&octets, NOM_FABRIQUÉ);
                app.show_analysis = true;
                app.analysis_module = "hex_editor".to_owned();
                app.toggle_maximized(NamsdecApp::ANALYSIS_WINDOW);
            }),
        ));
    }
    // LE MODE DÉBUTANT, VU DE L'ÉCRAN PRINCIPAL : c'est la barre
    // d'outils qu'on vient y regarder — cinq commandes au lieu de
    // sept, et le pas nommé par ce qu'il fait plutôt qu'en anglais de
    // débogueur. Programme assemblé et quelques pas faits, sans quoi
    // toutes les commandes seraient grises.
    if namsdec_asm::find_nasm().is_some() && namsdec_asm::find_linker() {
        scènes.push((
            "application / mode débutant (en marche)".to_owned(),
            None,
            Box::new(|app| {
                app.source = PROGRAMME_DU_BANC.to_owned();
                app.do_assemble();
                for _ in 0..8 {
                    app.do_step();
                }
                app.debutant = true;
                app.show_sidebar = true;
            }),
        ));
    }
    // LE MODE DÉBUTANT, qui est ce qu'une installation neuve montre.
    //
    // Sans scène, l'interface que verra quelqu'un qui ouvre namsdec
    // pour la première fois ne serait mesurée par rien — ni le banc,
    // ni les captures —, alors que c'est elle qui décide s'il reste.
    if let Some(octets) = binaire_fabriqué() {
        scènes.push((
            "analyse / mode débutant".to_owned(),
            Some("analysis_window"),
            Box::new(move |app| {
                app.load_elf(&octets, NOM_FABRIQUÉ);
                app.debutant = true;
                app.show_analysis = true;
                app.analysis_module.clear();
            }),
        ));
    }
    // L'ÉDITEUR BINAIRE AVEC UN OCTET CHOISI, et c'est le seul état où
    // son INSPECTEUR existe : « ce que cet octet vaut lu en int8, en
    // float64, en caractère ». Sans octet choisi, le panneau dit
    // « Cliquez un octet » et seize lignes de lecture n'étaient
    // mesurées par rien — ni le banc, ni les captures.
    //
    // La scène passe par `show_bytes_at`, c'est-à-dire par le chemin
    // réel : c'est ce qui arrive quand on clique une adresse dans un
    // autre panneau et qu'on demande à la voir dans les octets.
    if let Some(octets) = binaire_fabriqué() {
        scènes.push((
            "analyse / éditeur binaire, un octet choisi".to_owned(),
            Some("analysis_window"),
            Box::new(move |app| {
                app.load_elf(&octets, NOM_FABRIQUÉ);
                app.show_analysis = true;
                app.analysis_module = "hex_editor".to_owned();
                app.toggle_maximized(NamsdecApp::ANALYSIS_WINDOW);
                // L'entrée du programme : une position qui existe à
                // coup sûr, et celle qu'on va voir en premier.
                if let Some(elf) = namsdec_re::elf_scan::scan(&octets) {
                    app.show_bytes_at(elf.entry);
                }
            }),
        ));
    }
    // Un appel système refusé : la barre de statut en rouge, le message
    // le plus long que namsdec sache y mettre, et la fenêtre qui redit
    // ce que le programme avait écrit avant de s'arrêter.
    //
    // AUCUNE scène ne montrait l'application EN ERREUR, et c'est
    // pourtant l'écran sur lequel on tombe quand quelque chose ne va
    // pas — celui qu'on photographie pour le signaler. Les invariants
    // n'avaient jamais regardé cette barre-là remplie.
    scènes.push((
        "barre de statut / un appel système refusé".to_owned(),
        None,
        Box::new(|app| {
            app.status = Status::Error(crate::errors::emu(
                app.lang,
                &namsdec_emu::EmuError::UnsupportedSyscall {
                    backend: "namsdec",
                    number: 2,
                },
            ));
            app.show_console = true;
            app.console_tab = ConsoleTab::Output;
        }),
    ));
    scènes
}

/// L'écran du banc : assez grand pour qu'aucune fenêtre n'ait à se
/// serrer, et fixe, pour que deux exécutions disent la même chose.
fn écran_du_banc() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    }
}

/// Ce qu'une fenêtre écrit, ligne par ligne, et l'encre que chaque
/// ligne occupe vraiment.
///
/// Trois précautions, et chacune vient d'une erreur commise :
///
/// * **Le rectangle, non la position.** Un test qui compare des
///   positions laisse passer un texte qui déborde — c'est ce qui est
///   arrivé au champ de saisie coupé.
/// * **La ligne, non la galley.** Un texte de trois lignes a un cadre
///   qui couvre toute sa largeur, y compris le blanc au bout de la
///   dernière ligne ; deux de ces cadres se croisent sans qu'aucune
///   lettre ne se touche.
/// * **Sans l'espace de tête.** Dans une rangée qui se replie, egui
///   pose la galley au *début* de la rangée et décale son premier mot
///   à l'intérieur. Mesurée depuis son origine, une étiquette paraît
///   alors recouvrir celle qui la précède, alors qu'à l'écran les deux
///   se suivent. On demande donc à egui l'encre, et non le cadre.
///
/// `images` dit combien de fois dessiner avant de relever — une fenêtre
/// dont la hauteur épouse son contenu ne se pose qu'à la troisième.
fn ce_qui_est_écrit(
    ctx: &egui::Context,
    app: &mut NamsdecApp,
    id: Option<&str>,
    images: usize,
) -> (Option<egui::Rect>, Vec<(String, egui::Rect)>) {
    ce_qui_est_écrit_sur(ctx, app, id, images, écran_du_banc())
}

/// Le même relevé, sur un écran qu'on choisit.
///
/// Le banc dessine sur 1600x1100, et c'est là que tout a été éprouvé.
/// Mais l'application accepte de descendre à `MIN_SCREEN`, et ce qui
/// tient au large ne tient pas forcément à l'étroit : à 800x520, la
/// barre d'outils et la rangée d'onglets du panneau se peignaient l'une
/// sur l'autre — « Step Back » sous « Processeur », dans la même
/// couche. Une règle qui ne vaut qu'à une taille ne vaut pas.
fn ce_qui_est_écrit_sur(
    ctx: &egui::Context,
    app: &mut NamsdecApp,
    id: Option<&str>,
    images: usize,
    entrée: egui::RawInput,
) -> (Option<egui::Rect>, Vec<(String, egui::Rect)>) {
    let mut lignes: Vec<(String, egui::Rect)> = Vec::new();
    for _ in 0..images {
        lignes.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        let cadre =
            id.and_then(|id| egui::AreaState::load(ctx, egui::Id::new(id)).map(|a| a.rect()));
        // SON PROPRE FOND, et l'ordre des formes. Une fenêtre peint
        // d'abord le rectangle opaque de son cadre, puis ce qu'elle a
        // à dire : tout ce qui vient APRÈS ce rectangle est à elle.
        //
        // C'est le tri le plus sûr, et il a remplacé celui qui se
        // faisait sur la LARGEUR de la découpe — une fenêtre découpe
        // son contenu à sa propre largeur, un panneau à celle de
        // l'écran. Juste, tant que la fenêtre est plus étroite que
        // l'écran. AGRANDIE, elle l'est autant, et les textes de
        // l'éditeur de code derrière elle passaient le tri : les
        // invariants accusaient alors la fenêtre de faire son métier,
        // qui est de couvrir. Une scène a dû être retirée pour cela le
        // 28 octobre 2026.
        let fond = cadre.and_then(|c| {
            sortie.shapes.iter().position(|clipped| {
                // Il COÏNCIDE avec le cadre, il ne fait pas que le
                // contenir : le fond de l'écran contient aussi toutes
                // les fenêtres, et le prendre pour l'une d'elles
                // revenait à ne plus trier du tout.
                // SON REMPLISSAGE NE COMPTE PAS, seulement sa place.
                // Une fenêtre ordinaire peint un fond plein ; une
                // fenêtre AGRANDIE dans la zone de travail n'en peint
                // aucun — c'est le panneau derrière qui le fournit —,
                // et il ne reste que le trait de son cadre, en alpha
                // zéro. Il est peint au même moment, avant le contenu,
                // et c'est tout ce qu'on lui demande.
                matches!(&clipped.shape, egui::epaint::Shape::Rect(r)
                    if r.rect.contains_rect(c.shrink(2.0))
                        && c.expand(8.0).contains_rect(r.rect))
            })
        });
        for (rang, clipped) in sortie.shapes.iter().enumerate() {
            // Ce que la fenêtre peint *elle-même*, et non ce qui se
            // trouve sous elle. Sans ce tri, un registre caché derrière
            // la fenêtre d'aide passait pour un recouvrement — et cela
            // n'arrivait qu'une fois un programme chargé, c'est-à-dire
            // jamais dans les tests d'alors.
            match fond {
                Some(fond) if rang <= fond => continue,
                // Pas de fond trouvé — une surface qui n'en peint pas,
                // ou un cadre qu'on ne sait pas situer : on retombe sur
                // la largeur de la découpe, qui vaut pour tout ce qui
                // n'est pas agrandi.
                None if cadre.is_some_and(|c| {
                    let large = c.x_range().expand(2.0);
                    !large.contains(clipped.clip_rect.left())
                        || !large.contains(clipped.clip_rect.right())
                }) =>
                {
                    continue;
                }
                _ => {}
            }
            if let egui::epaint::Shape::Text(t) = &clipped.shape {
                for ligne in &t.galley.rows {
                    if ligne.glyphs.is_empty() {
                        continue;
                    }
                    // Ce qui se voit, et non ce qui est peint : une
                    // ligne sortie d'un ascenseur est bien dans la
                    // liste des formes, mais rognée à néant.
                    let encre = ligne
                        .rect_without_leading_space()
                        .translate(t.pos.to_vec2())
                        .intersect(clipped.clip_rect);
                    if encre.width() <= 0.0 || encre.height() <= 0.0 {
                        continue;
                    }
                    lignes.push((t.galley.text().to_owned(), encre));
                }
            }
        }
        sortie.textures_delta.clear();
    }
    let cadre = id.map(|id| {
        egui::AreaState::load(ctx, egui::Id::new(id))
            .unwrap_or_else(|| panic!("la fenêtre « {id} » est affichée"))
            .rect()
    });
    (cadre, lignes)
}

/// Pousse la date de dernière écriture d'un fichier d'une seconde, pour
/// que le changement soit visible d'un test qui écrit deux fois dans la
/// même seconde.
fn filetime_plus_tard(chemin: &std::path::Path) {
    let plus_tard = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
    let f = std::fs::File::options().write(true).open(chemin).unwrap();
    f.set_modified(plus_tard).unwrap();
}

/// Toutes les icônes dessinées, avec la taille qu'elles reçoivent
/// vraiment.
///
/// Le bouton carré du panneau donne douze pixels, la barre d'outils
/// quatorze, le gros bouton d'analyse dix-huit. Une icône qui n'a été
/// regardée qu'en grand ment sur ce qu'elle sera.
type Icône = (&'static str, fn(&egui::Painter, egui::Rect, egui::Color32));

fn icônes_du_banc() -> Vec<Icône> {
    vec![
        ("calculatrice", chrome::icon_calculator),
        ("analyse", chrome::icon_analysis),
        ("recherche", chrome::icon_search),
        ("globe", chrome::icon_globe),
        ("github", chrome::icon_github),
        ("discord", chrome::icon_discord),
        ("retour arrière", chrome::icon_backspace),
        ("tout effacer", chrome::icon_clear),
        ("œil", icon_watch),
        ("aller", icon_go),
        ("exécuter", icon_run),
        ("arrêter", icon_stop),
        ("pas dans", icon_step_into),
        ("pas sur", icon_step_over),
        ("pas arrière", icon_step_back),
        ("réinitialiser", icon_reset),
        ("agrandir", icon_maximize),
        ("restaurer", icon_restore),
    ]
}

/// Les abscisses des flèches de liste posées sur cette ligne.
///
/// egui dessine la flèche d'une `ComboBox` en triangle fermé de trois
/// points, calée à droite de la boîte. C'est le repère le plus proche du
/// bord droit de la liste, et donc le meilleur pour dire si deux listes
/// ont la même largeur.
fn flèches_de_la_rangée(ctx: &egui::Context, app: &mut NamsdecApp, ligne: f32) -> Vec<f32> {
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1100.0),
        )),
        ..Default::default()
    };
    let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
    let mut x = Vec::new();
    for clipped in &sortie.shapes {
        if let egui::epaint::Shape::Path(chemin) = &clipped.shape
            && chemin.closed
            && chemin.points.len() == 3
        {
            let boîte = egui::Rect::from_points(&chemin.points);
            if (boîte.center().y - ligne).abs() < 12.0 {
                x.push(boîte.left());
            }
        }
    }
    sortie.textures_delta.clear();
    x
}

/// Le plus grand vide vertical d'une fenêtre, toutes formes comptées.
///
/// Les textes ne suffisent pas : l'image du bouton de don, l'histogramme
/// d'analyse et le tracé de la chronologie sont du contenu, et les
/// compter pour rien accuserait des fenêtres pleines.
///
/// Les fonds de la fenêtre elle-même, eux, ne comptent pas : un
/// rectangle qui couvre tout le cadre remplirait le vide à lui seul.
fn plus_grand_vide(ctx: &egui::Context, app: &mut NamsdecApp, id: &str) -> (f32, egui::Rect) {
    let entrée = écran_du_banc();
    let mut bandes: Vec<(f32, f32)> = Vec::new();
    let mut cadre = egui::Rect::NOTHING;
    for _ in 0..5 {
        bandes.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        cadre = egui::AreaState::load(ctx, egui::Id::new(id))
            .map(|a| a.rect())
            .unwrap_or(egui::Rect::NOTHING);
        for clipped in &sortie.shapes {
            let large = cadre.x_range().expand(2.0);
            if !large.contains(clipped.clip_rect.left())
                || !large.contains(clipped.clip_rect.right())
            {
                continue;
            }
            let vu = clipped
                .shape
                .visual_bounding_rect()
                .intersect(clipped.clip_rect);
            if !vu.is_positive() {
                continue;
            }
            // Un fond de fenêtre n'est pas du contenu.
            if vu.width() > cadre.width() * 0.8 && vu.height() > cadre.height() * 0.8 {
                continue;
            }
            bandes.push((vu.top(), vu.bottom()));
        }
        sortie.textures_delta.clear();
    }
    bandes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut fusion: Vec<(f32, f32)> = Vec::new();
    for (haut, bas) in bandes {
        match fusion.last_mut() {
            Some(dernière) if haut <= dernière.1 => dernière.1 = dernière.1.max(bas),
            _ => fusion.push((haut, bas)),
        }
    }
    let trou = fusion
        .windows(2)
        .map(|p| p[1].0 - p[0].1)
        .fold(0.0f32, f32::max);
    (trou, cadre)
}

/// Les séquences « Espace x y » d'une page, relevées à la main.
///
/// À la main parce qu'une seule expression régulière servirait ici, et
/// qu'une dépendance pour trois lignes se paierait sur chaque
/// compilation.
fn regex_maison(texte: &str) -> Vec<String> {
    let mut trouvées = Vec::new();
    let nettoyé = texte.replace("**", "").replace(" puis ", " ");
    for (position, _) in nettoyé.match_indices("Espace ") {
        let suite = &nettoyé[position..];
        let mots: Vec<&str> = suite.split_whitespace().take(3).collect();
        if mots.len() < 3 {
            continue;
        }
        // Deux lettres ou signes courts après « Espace » : au-delà,
        // c'est une phrase, non une séquence.
        // Une lettre ASCII ou une barre oblique. Sans cette borne, le
        // relevé prenait le « … » de la phrase qui EXPLIQUE ce qu'est une
        // séquence, et accusait le manuel de promettre « Espace … ».
        let bon = |m: &str| {
            let m = m.trim_end_matches([',', '.', ':', ')', '`', '<', '»']);
            m.chars().count() == 1 && m.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
        };
        if bon(mots[1]) && bon(mots[2]) {
            let taille = |m: &str| {
                m.trim_end_matches([',', '.', ':', ')', '`', '<', '»'])
                    .to_owned()
            };
            trouvées.push(format!("Espace {} {}", taille(mots[1]), taille(mots[2])));
        }
    }
    trouvées.sort();
    trouvées.dedup();
    trouvées
}

/// UN ELF MINIMAL, fabriqué ici.
///
/// Les contrôles voisins partent de `/bin/ls` ou de nasm, et s'abstiennent
/// quand ils manquent — ce qui les rend muets sur une machine
/// d'intégration. Celui-ci ne demande rien : un en-tête, un segment
/// chargeable, un peu de code.
fn elf_minimal() -> Vec<u8> {
    const BASE: u64 = 0x40_0000;
    const DÉPART: u64 = 0x78;
    let code: &[u8] = &[
        0x48, 0xc7, 0xc0, 0x3c, 0x00, 0x00, 0x00, // mov rax, 60
        0x48, 0x31, 0xff, // xor rdi, rdi
        0x0f, 0x05, // syscall
    ];
    let mut f = vec![0u8; DÉPART as usize];
    f[..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
    f[4] = 2; // ELFCLASS64
    f[5] = 1; // ELFDATA2LSB
    f[6] = 1; // EV_CURRENT
    f[16..18].copy_from_slice(&2u16.to_le_bytes()); // ET_EXEC
    f[18..20].copy_from_slice(&62u16.to_le_bytes()); // EM_X86_64
    f[20..24].copy_from_slice(&1u32.to_le_bytes());
    f[24..32].copy_from_slice(&(BASE + DÉPART).to_le_bytes()); // e_entry
    f[32..40].copy_from_slice(&64u64.to_le_bytes()); // e_phoff
    f[52..54].copy_from_slice(&64u16.to_le_bytes()); // e_ehsize
    f[54..56].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
    f[56..58].copy_from_slice(&1u16.to_le_bytes()); // e_phnum
    // Le segment : tout le fichier, chargeable et exécutable.
    let total = DÉPART + code.len() as u64;
    let p = 64usize;
    f[p..p + 4].copy_from_slice(&1u32.to_le_bytes()); // PT_LOAD
    f[p + 4..p + 8].copy_from_slice(&5u32.to_le_bytes()); // R+X
    f[p + 16..p + 24].copy_from_slice(&BASE.to_le_bytes()); // p_vaddr
    f[p + 24..p + 32].copy_from_slice(&BASE.to_le_bytes()); // p_paddr
    f[p + 32..p + 40].copy_from_slice(&total.to_le_bytes()); // p_filesz
    f[p + 40..p + 48].copy_from_slice(&total.to_le_bytes()); // p_memsz
    f[p + 48..p + 56].copy_from_slice(&0x1000u64.to_le_bytes()); // p_align
    f.extend_from_slice(code);
    f
}
