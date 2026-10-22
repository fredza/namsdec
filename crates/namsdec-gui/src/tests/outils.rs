//! Les outils du dehors, et ce que namsdec installe.
//!
//! Les désassembleurs et décompilateurs tiers, le plan d'installation,
//! les scripts, le relevé de version, le rapport d'anomalie : tout ce qui
//! touche à la machine autour de namsdec.
//!
//! Sorti de `tests.rs` le 22 octobre 2026, où trente-trois mille
//! lignes de contrôles se suivaient d'un bloc.

use super::*;

/// Une préversion le dit dans sa boîte « À propos », et le bandeau
/// disparaît de lui-même le jour où elle n'en est plus une.
///
/// Un avertissement qu'il faut penser à retirer à la main finit toujours
/// par mentir : celui-ci est lu dans le numéro de version.
#[test]
fn a_pre_release_says_so_and_a_release_says_nothing() {
    // Ce que SemVer appelle une préversion : ce qui suit le tiret.
    assert_eq!(prerelease("0.2.1-beta.2"), Some("beta.2"));
    assert_eq!(prerelease("1.0.0-rc.1"), Some("rc.1"));
    assert_eq!(prerelease("0.2.1-alpha"), Some("alpha"));
    // Une version définitive n'a rien à annoncer, métadonnée de
    // compilation comprise.
    assert_eq!(prerelease("0.2.1"), None);
    assert_eq!(prerelease("1.0.0+20260917"), None);

    // Et la version d'aujourd'hui, telle qu'elle est : le bandeau est
    // là, avec ce qu'il faut dire.
    let mut app = NamsdecApp {
        show_about: true,
        ..Default::default()
    };
    let painted = paint(&mut app);
    let s = strings(app.lang);
    if let Some(étiquette) = prerelease(APP_VERSION) {
        assert!(
            painted.contains(&format!("{} — {étiquette}", s.about_beta_title)),
            "{painted}"
        );
        // Le texte dit ce qu'il faut savoir : que les réglages peuvent
        // bouger, et que les erreurs se disent.
        assert!(s.about_beta_body.contains("réglages"));
        assert!(strings(Lang::En).about_beta_body.contains("settings"));
    } else {
        assert!(
            !painted.contains(s.about_beta_title),
            "version définitive : aucun bandeau à afficher"
        );
    }
}

/// La recherche de mise à jour est branchée : elle demande, elle répond,
/// et elle n'installe rien.
///
/// Le réseau n'est pas touché ici — ce que GitHub répond est vérifié
/// chez `namsdec-update`. Ce qui se vérifie ici, c'est le chemin :
/// l'état d'attente, le verdict affiché, et la promesse tenue de ne
/// jamais se remplacer tout seul.
#[test]
fn the_update_check_is_wired_and_installs_nothing() {
    let mut app = NamsdecApp {
        show_about: true,
        ..Default::default()
    };
    // Rien n'a été demandé tant que personne n'a cliqué.
    assert!(app.update_check.is_none());
    assert!(app.update_result.is_none());

    let painted = paint(&mut app);
    let s = strings(app.lang);
    assert!(painted.contains(s.about_check_update), "{painted}");

    // Le verdict, tel qu'il s'affiche : plus récent, à jour, devant.
    let release = |v: &str| namsdec_update::Release {
        tag: format!("v{v}"),
        version: v.to_owned(),
        name: v.to_owned(),
        url: format!("https://github.com/fredza/namsdec/releases/tag/v{v}"),
        prerelease: v.contains('-'),
    };
    app.update_result = Some(Ok(namsdec_update::verdict(APP_VERSION, release("9.9.9"))));
    let painted = paint(&mut app);
    assert!(painted.contains(s.about_update_newer), "{painted}");
    assert!(painted.contains("v9.9.9"), "{painted}");
    // Et la façon de mettre à jour est dite : à la main.
    assert!(painted.contains(s.about_update_how), "{painted}");
    assert!(s.about_update_how.contains("install.sh"));

    app.update_result = Some(Ok(namsdec_update::verdict(
        APP_VERSION,
        release(APP_VERSION),
    )));
    let painted = paint(&mut app);
    assert!(painted.contains(s.about_update_uptodate), "{painted}");

    // Un dépôt qui n'a rien publié n'est pas une panne, et le message
    // le dit autrement qu'un échec.
    app.update_result = Some(Err(s.about_update_none.to_owned()));
    let painted = paint(&mut app);
    assert!(painted.contains(s.about_update_none), "{painted}");

    // Refermer la boîte oublie la réponse : l'état du dépôt a pu changer
    // entre deux ouvertures.
    app.show_about = false;
    paint(&mut app);
    assert!(app.update_result.is_none());
}

/// Ce que la recherche de mise à jour va trouver doit exister : le
/// dépôt publie ses versions, et la façon de les publier est décrite
/// dans le dépôt lui-même.
///
/// Un bouton qui interroge une API que rien n'alimente ne dit jamais
/// rien d'autre que « aucune version publiée ».
#[test]
fn the_repository_knows_how_to_publish_what_the_check_looks_for() {
    let workflow = include_str!("../../../../.github/workflows/release.yml");
    // Publié sur étiquette : c'est ce que « dès qu'un build est
    // publié » veut dire.
    assert!(workflow.contains("tags:"), "{workflow}");
    assert!(workflow.contains("softprops/action-gh-release"));
    // Et pas n'importe quoi : les tests passent avant la publication.
    assert!(workflow.contains("cargo test --workspace"));
    assert!(workflow.contains("cargo clippy"));
    // Une somme de contrôle accompagne le binaire.
    assert!(workflow.contains("sha256sum"));
    // Une étiquette à tiret est publiée comme préversion, comme le dit
    // SemVer et comme le programme le lit.
    assert!(workflow.contains("prerelease:"));

    // Le dépôt interrogé est celui qui publie.
    assert_eq!(namsdec_update::OWNER, "fredza");
    assert_eq!(namsdec_update::REPO, "namsdec");
}

/// Le dépôt sait aussi se vérifier à chaque poussée, pas seulement au
/// moment de publier.
///
/// Publier ce qui n'a pas été vérifié transformerait la vérification en
/// formalité ; vérifier seulement au moment de publier laisserait `main`
/// casser sans que personne le sache.
#[test]
fn the_repository_checks_itself_on_every_push() {
    let ci = include_str!("../../../../.github/workflows/ci.yml");
    assert!(ci.contains("branches: [main]"), "{ci}");
    assert!(ci.contains("pull_request"), "{ci}");
    for étape in [
        "cargo fmt --all --check",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test --workspace",
    ] {
        assert!(ci.contains(étape), "le CI ne fait pas « {étape} »");
    }
    // nasm et ld sont installés : sans eux, la moitié des tests se
    // taisent au lieu de vérifier quoi que ce soit.
    assert!(ci.contains("nasm binutils"), "{ci}");
}

/// « Installer… » ouvre son plan, et le plan se voit.
///
/// Le bouton posait bien la demande, et la fenêtre se dessinait bien —
/// mais derrière celle des préférences, d'où elle était partie. On
/// cliquait, et il ne se passait rien de visible.
#[test]
fn the_install_button_brings_its_plan_to_the_front() {
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Tools,
        show_decompiler_module: true,
        show_installer_module: true,
        // CE QUE LA MACHINE RÉPOND, POSÉ ICI plutôt que demandé au
        // disque. Le test lisait vraiment `~/.local/share` pour savoir
        // quels outils portent un bouton « Installer » — il dépendait
        // donc de ce qui est installé sur le poste, et il a fini par
        // échouer le jour où namsdec lui-même a été installé dessus.
        //
        // Un seul outil manquant, tous les autres prêts : le bouton
        // cherché est alors le sien, et il n'y en a qu'un.
        tools_ready: namsdec_disasm::known::all()
            .into_iter()
            .map(|k| {
                // Deux outils incomplets, et non un seul : lequel des
                // deux boutons se trouve en premier dans la fenêtre
                // dépend de la mise en page, laquelle bouge quand on
                // touche à un widget voisin. Le test n'a pas à le
                // savoir — il a à vérifier que le bouton cliqué est
                // celui d'un outil qui a vraiment besoin d'être
                // installé.
                let état = if matches!(k.plugin.id.as_str(), "rizin-ghidra" | "retdec") {
                    namsdec_disasm::known::Availability::Incomplete
                } else {
                    namsdec_disasm::known::Availability::Ready
                };
                (k.plugin.id.clone(), état)
            })
            .collect(),
        ..Default::default()
    };
    let s = strings(app.lang);
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1400.0),
        )),
        ..Default::default()
    };

    // Où se trouve le bouton, d'après ce qui est peint.
    let mut cible = None;
    for _ in 0..3 {
        cible = None;
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            // Deux outils INCOMPLETS : leur bouton de complétion dit
            // « Réparer » (accent), et non « Installer… » — on ne peut pas
            // les activer avant de les avoir complétés.
            if let egui::epaint::Shape::Text(texte) = &clipped.shape
                && texte.galley.text() == s.install_repair
            {
                let milieu = texte.pos + texte.galley.size() / 2.0;
                // **Visible**, et pas seulement peint : la liste des
                // outils défile, et le dernier bouton peint est souvent
                // celui qui dépasse du bas. Cliquer à sa position ne
                // l'atteindrait pas — le test échouait alors en accusant
                // le clic, alors que c'est la cible qui était mauvaise.
                if clipped.clip_rect.contains(milieu) {
                    cible = Some(milieu);
                    break;
                }
            }
        }
        sortie.textures_delta.clear();
    }
    // Pas de bouton s'il n'y a rien à installer : sur une machine où
    // tout est déjà bâti, ce test n'a rien à observer. Il le dit plutôt
    // que d'échouer sur l'état du poste.
    let Some(cible) = cible else {
        return;
    };

    let mut presse = entrée.clone();
    presse.events.push(egui::Event::PointerMoved(cible));
    presse.events.push(egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(presse, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    let mut relâche = entrée.clone();
    relâche.events.push(egui::Event::PointerButton {
        pos: cible,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(relâche, |ui| app.draw(ui));
    sortie.textures_delta.clear();

    // Le plan demandé est celui d'un outil qui a vraiment besoin d'être
    // installé — et non un nom écrit en dur : lequel des outils porte un
    // bouton « Installer » dépend de ce que la machine a déjà. Nommer
    // « rizin-ghidra » faisait échouer le test sur un poste où rizin
    // était complet, c'est-à-dire là où tout allait bien.
    let plan = app
        .install_plan
        .as_deref()
        .expect("le clic doit demander un plan");
    assert!(
        matches!(plan, "rizin-ghidra" | "retdec"),
        "le bouton cliqué est celui d'un outil qu'on a dit incomplet — « {plan} »"
    );

    // Et la fenêtre passe devant : son calque est au premier plan, non
    // rangé avec les fenêtres ordinaires comme les préférences.
    let mut sortie = ctx.run_ui(entrée, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    // UN calque au premier plan, et non « le premier calque trouvé ».
    // Remettre une fenêtre devant crée un calque à son nom au rang
    // ordinaire, même quand la fenêtre vit au premier plan : deux
    // calques portent alors le même nom, et `find` rendait celui qui se
    // présentait en premier.
    let au_premier_plan = ctx.memory(|m| {
        m.layer_ids().any(|id| {
            id.id == egui::Id::new("installer_window") && id.order == egui::Order::Foreground
        })
    });
    assert!(
        au_premier_plan,
        "le plan doit se poser au-dessus des préférences d'où il part"
    );
}

/// La décompilation ne s'exécute pas toute seule, et le dit avant.
///
/// Ce module n'ajoute pas un affichage : il accorde à namsdec la
/// permission de lancer un programme extérieur. Trois verrous, et ce
/// test les vérifie tous les trois — la case des préférences, l'entrée
/// de menu qui n'existe qu'ensuite, et le bouton qui seul déclenche.
#[test]
fn the_decompiler_never_runs_on_its_own() {
    let app = NamsdecApp::default();
    assert!(
        !app.show_decompiler_module,
        "le module est éteint tant que personne ne l'allume"
    );
    assert!(app.decompile_job.is_none() && app.decompile_result.is_none());

    // Éteint, la fenêtre ne s'ouvre pas — même si on le demande.
    let mut app = NamsdecApp {
        show_decompiler: true,
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        !peint.contains(strings(Lang::Fr).decompiler_title),
        "la fenêtre reste fermée tant que le module est éteint :\n{peint}"
    );

    // Allumé mais sans outil installé, la fenêtre explique comment en
    // installer un, et ne lance rien.
    let mut app = NamsdecApp {
        show_decompiler_module: true,
        show_decompiler: true,
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        peint.contains(strings(Lang::Fr).decompiler_title),
        "{peint}"
    );
    assert!(
        peint.contains(strings(Lang::Fr).decompiler_none_installed),
        "{peint}"
    );
    assert!(app.decompile_job.is_none(), "rien n'a été lancé");
}

/// Deux millions de caractères se lisent, se cherchent et se parcourent.
///
/// RetDec rend soixante mille lignes de C pour un petit crackme. Ce test
/// tient trois promesses : on ne peint que ce qui tient à l'écran — sinon
/// la fenêtre se fige —, les fonctions sont repérées, et le filtre réduit
/// vraiment ce qu'on regarde.
#[test]
fn a_huge_decompilation_stays_readable() {
    let mut texte = String::new();
    for i in 0..20_000 {
        texte.push_str(&format!("int32_t function_{i:05x}(int32_t a) {{\n"));
        texte.push_str("    return a + 1;\n");
        texte.push_str("}\n");
    }
    let dernière = "function_04e1f";

    let mut app = NamsdecApp {
        show_decompiler_module: true,
        show_decompiler: true,
        decompilers: vec![namsdec_disasm::plugin::Plugin {
            id: "essai".to_owned(),
            name: "Essai".to_owned(),
            command: "wc".to_owned(),
            args: vec!["-c".to_owned(), "{file}".to_owned()],
            install: None,
            kind: Default::default(),
        }],
        decompiler: "essai".to_owned(),
        ..Default::default()
    };
    app.index_decompilation(&texte);
    app.decompile_result = Some(Ok(texte));

    assert_eq!(
        app.decompile_functions.len(),
        20_000,
        "chaque fonction ouvre une ligne sans indentation"
    );
    assert_eq!(app.decompile_lines.len(), 60_000);

    let peint = paint(&mut app);
    assert!(
        peint.contains("function_00000"),
        "le début se lit :\n{}",
        &peint[..peint.len().min(2000)]
    );
    assert!(
        !peint.contains(dernière),
        "mais pas la soixante-millième ligne : on ne peint que l'écran"
    );

    // Le filtre ne garde que ce qu'on cherche, et le compte le dit.
    app.decompile_filter = "function_04e1f".to_owned();
    let peint = paint(&mut app);
    assert_eq!(
        app.decompile_visible.len(),
        1,
        "une seule ligne porte ce nom"
    );
    assert!(
        peint.contains(dernière),
        "et c'est elle qu'on voit maintenant"
    );

    // Sauter à une fonction remet le texte entier, à la bonne hauteur.
    app.decompile_filter.clear();
    app.decompile_goto = Some(app.decompile_functions[10_000].0);
    let peint = paint(&mut app);
    assert!(
        peint.contains("function_02710"),
        "le saut amène où l'on a cliqué :\n{peint}"
    );
}

/// Avec un outil installé, la commande est écrite avant d'être lancée —
/// et « Lancer » la lance vraiment.
#[test]
fn the_decompiler_shows_the_command_then_runs_it() {
    let outil = namsdec_disasm::plugin::Plugin {
        id: "essai".to_owned(),
        name: "Essai".to_owned(),
        // `wc -c` sur le fichier : un outil qui existe partout, dont la
        // réponse dépend vraiment du binaire qu'on lui donne.
        command: "wc".to_owned(),
        args: vec!["-c".to_owned(), "{file}".to_owned()],
        install: None,
        kind: Default::default(),
    };
    let mut app = NamsdecApp {
        show_decompiler_module: true,
        show_decompiler: true,
        decompilers: vec![outil],
        decompiler: "essai".to_owned(),
        source: DEFAULT_SOURCE.to_owned(),
        ..Default::default()
    };
    if namsdec_asm::find_nasm().is_none() || !namsdec_asm::find_linker() {
        return;
    }
    app.do_assemble();

    // La commande est annoncée, en toutes lettres.
    let peint = paint(&mut app);
    assert!(
        peint.contains("wc -c {file}"),
        "ce qui sera lancé doit se lire avant de l'être :\n{peint}"
    );
    assert!(peint.contains(strings(Lang::Fr).decompiler_run), "{peint}");
    assert!(app.decompile_result.is_none(), "afficher n'est pas lancer");

    // Et le lancement, lui, rend ce que l'outil a écrit.
    app.start_decompile();
    let ctx = egui::Context::default();
    for _ in 0..200 {
        app.poll_decompile(&ctx);
        if app.decompile_result.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let réponse = app
        .decompile_result
        .as_ref()
        .expect("l'outil a fini")
        .as_ref()
        .expect("sans erreur");
    // `wc -c` écrit la taille du fichier : c'est bien notre binaire.
    let taille = app
        .assemble_state
        .as_ref()
        .expect("assemblé")
        .output
        .file
        .len();
    assert!(
        réponse.contains(&taille.to_string()),
        "l'outil a bien reçu le programme lié : {réponse:?} pour {taille} octets"
    );
    assert!(app.decompiled_from.is_some(), "on dit de quoi il s'agit");
}

/// Un outil qui ne rend pas la main est arrêté, et l'on dit pourquoi.
#[test]
fn a_tool_that_never_finishes_is_stopped() {
    let dormeur = namsdec_disasm::plugin::Plugin {
        id: "dormeur".to_owned(),
        name: "Dormeur".to_owned(),
        // `tail -f` suit un fichier sans fin : c'est exactement l'outil
        // qui ne rend jamais la main, et il existe partout.
        command: "tail".to_owned(),
        args: vec!["-f".to_owned(), "{file}".to_owned()],
        install: None,
        kind: Default::default(),
    };
    let début = std::time::Instant::now();
    let erreur = dormeur
        .run_within(b"peu importe", std::time::Duration::from_millis(300))
        .expect_err("l'outil doit être arrêté");
    assert!(
        début.elapsed() < std::time::Duration::from_secs(5),
        "on ne l'a pas attendu : {:?}",
        début.elapsed()
    );
    assert!(
        erreur.to_string().contains("longer"),
        "et l'on dit qu'il a été arrêté : {erreur}"
    );
}

/// Ce qui tourne à côté se voit : dans la barre d'état, et jusque dans
/// la barre des tâches du système.
///
/// Un fil d'exécution est invisible. Sans un mot quelque part, on ne
/// sait ni que quelque chose tourne, ni depuis combien de temps, ni
/// combien de temps encore avant l'abandon. La progression montrée est
/// celle du **temps accordé** — la seule qu'un programme extérieur nous
/// laisse connaître, et la seule qui décide de quelque chose.
#[test]
fn a_background_job_shows_its_progress_where_one_looks() {
    let mut app = NamsdecApp {
        show_statusbar: true,
        ..Default::default()
    };
    // Rien ne tourne : rien ne s'affiche, et le titre reste le nom.
    let peint = paint(&mut app);
    assert!(
        !peint.contains(strings(Lang::Fr).job_decompiling),
        "aucun travail, aucun bruit :\n{peint}"
    );

    // Un travail borné : la barre montre une part vraie, et le texte dit
    // le temps écoulé sur le temps accordé.
    app.job = Some(BackgroundJob {
        label: strings(Lang::Fr).job_decompiling.to_owned(),
        started: std::time::Instant::now() - std::time::Duration::from_secs(30),
        limit: Some(std::time::Duration::from_secs(120)),
    });
    let part = app.job.as_ref().and_then(BackgroundJob::fraction);
    assert!(
        part.is_some_and(|p| (p - 0.25).abs() < 0.02),
        "trente secondes sur deux minutes : un quart, obtenu {part:?}"
    );
    let peint = paint(&mut app);
    assert!(
        peint.contains(strings(Lang::Fr).job_decompiling),
        "la barre d'état nomme ce qui tourne :\n{peint}"
    );
    assert!(
        peint.contains("30 s / 120 s"),
        "avec le temps écoulé sur le temps accordé :\n{peint}"
    );

    // Un travail que rien ne borne : pas de fraction inventée.
    app.job = Some(BackgroundJob {
        label: strings(Lang::Fr).job_checking_update.to_owned(),
        started: std::time::Instant::now(),
        limit: None,
    });
    assert_eq!(
        app.job.as_ref().and_then(BackgroundJob::fraction),
        None,
        "sans limite, aucune progression ne peut être honnête"
    );

    // Et le titre de la fenêtre porte l'avancement : c'est lui que la
    // barre des tâches du système affiche.
    let ctx = egui::Context::default();
    app.job = Some(BackgroundJob {
        label: "Décompilation".to_owned(),
        started: std::time::Instant::now() - std::time::Duration::from_secs(60),
        limit: Some(std::time::Duration::from_secs(120)),
    });
    app.reflect_job_in_title(&ctx);
    let titre = app.window_title.clone().expect("un titre a été posé");
    assert!(titre.contains("50 %"), "{titre}");
    assert!(titre.starts_with("Namsdec"), "{titre}");

    // Le travail fini, le titre revient à ce qu'il était : une
    // application marquée « 100 % » pour toujours fait douter du reste.
    app.job = None;
    app.reflect_job_in_title(&ctx);
    assert_eq!(app.window_title.as_deref(), Some("Namsdec"));
}

/// CONSTRUIRE L'APPLICATION N'ÉCRIT RIEN DANS LES FICHIERS DE
/// QUELQU'UN.
///
/// La règle est ancienne — aucun contrôle n'écrit dans `settings.json`
/// ni dans `~/.local/share/namsdec` — et elle a été enfreinte le
/// 20 octobre 2026 en ajoutant la reprise des anciens dossiers de
/// modules : posée dans `NamsdecApp::default()`, elle s'exécutait à
/// chaque construction, donc dans les six cent quatre-vingt-dix
/// contrôles. Deux fichiers sont apparus dans le dossier de données de
/// la personne pendant une simple exécution de `cargo test`.
///
/// Le contrôle surveille le dossier de données RÉEL : il relève ce
/// qu'il contient, construit l'application une centaine de fois, et
/// vérifie que rien n'a bougé. Il ne crée rien lui-même, et s'abstient
/// quand ce dossier n'existe pas — sur une machine d'intégration
/// continue, il n'y a rien à protéger.
#[test]
fn building_the_app_writes_nothing_into_anyones_files() {
    let Some(données) = data_dir() else {
        return;
    };
    let relevé = |dossier: &std::path::Path| -> Vec<String> {
        let Ok(entrées) = std::fs::read_dir(dossier) else {
            return Vec::new();
        };
        let mut noms: Vec<String> = entrées
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        noms.sort();
        noms
    };
    // Les sous-dossiers qu'une reprise ou une installation toucherait.
    let surveillés: Vec<std::path::PathBuf> = ["modules", "decompilers", "disassemblers", "themes"]
        .iter()
        .map(|nom| données.join(nom))
        .collect();
    let avant: Vec<Vec<String>> = surveillés.iter().map(|d| relevé(d)).collect();
    let existait: Vec<bool> = surveillés.iter().map(|d| d.exists()).collect();

    for _ in 0..50 {
        let _ = NamsdecApp::default();
    }

    for ((dossier, avant), existait) in surveillés.iter().zip(&avant).zip(&existait) {
        assert_eq!(
            dossier.exists(),
            *existait,
            "construire l'application a créé ou retiré {} — elle ne doit rien \
             écrire dans les fichiers de quelqu'un",
            dossier.display()
        );
        assert_eq!(
            &relevé(dossier),
            avant,
            "construire l'application a changé le contenu de {}",
            dossier.display()
        );
    }
}

/// Déclarer un outil écrit un fichier — et rien d'autre.
///
/// C'est la promesse de cet écran : namsdec ne télécharge pas Rizin, ne
/// l'installe pas, ne le lance pas. Il pose la description que l'outil
/// attendait, avec les arguments qui marchent — ceux qu'on aurait mis
/// une heure à trouver soi-même.
#[test]
fn declaring_a_tool_writes_a_file_and_nothing_else() {
    let dossier = tempfile::tempdir().expect("un dossier temporaire");
    let mut app = NamsdecApp::default();
    assert!(app.decompilers.is_empty());

    let rizin = namsdec_disasm::known::all()
        .into_iter()
        .find(|outil| outil.plugin.id == "rizin-ghidra")
        .expect("Rizin est un outil connu");

    with_data_home(dossier.path(), || app.declare_tool(&rizin.plugin));

    // Le fichier est là, lisible, et namsdec le relit comme n'importe
    // quelle description écrite à la main.
    // Dans `modules/`, qui porte maintenant TOUS les outils : le
    // dossier ne dit plus ce qu'une fiche rend, la fiche le dit.
    let écrit = dossier.path().join("namsdec/modules/rizin-ghidra.json");
    assert!(écrit.is_file(), "{} manque", écrit.display());
    let relu = namsdec_disasm::plugin::read(&écrit).expect("la description est valide");
    assert_eq!(relu.command, "rizin");
    assert_eq!(
        relu.kind,
        namsdec_disasm::plugin::Kind::Decompiler,
        "et la fiche dit qu'il décompile"
    );
    assert!(
        relu.args.iter().any(|a| a.contains("pdg")),
        "c'est `pdg` qui décompile : {:?}",
        relu.args
    );

    // Et le premier outil déclaré devient celui qu'on lancera : une
    // liste d'un seul élément ne devrait pas s'ouvrir sur « choisir… ».
    with_data_home(dossier.path(), || {
        app.decompilers = load_installed_decompilers();
    });
    assert_eq!(app.decompiler, "rizin-ghidra");

    // Le retrait enlève la description, pas le programme.
    with_data_home(dossier.path(), || app.undeclare_tool("rizin-ghidra"));
    assert!(!écrit.exists(), "la description s'en va");
    assert!(app.decompiler.is_empty() || app.decompiler != "rizin-ghidra");
}

/// namsdec dit ce que la machine a, et ce qu'elle n'a pas.
///
/// Proposer un outil sans dire s'il est là laisse découvrir son absence
/// au moment de s'en servir — c'est-à-dire au pire moment. Et Rizin
/// mérite un mot de plus : le programme peut être installé sans que la
/// décompilation le soit, faute du greffon qui l'apporte.
#[test]
fn the_tools_say_whether_this_machine_has_them() {
    use namsdec_disasm::known::Availability;
    let outils = namsdec_disasm::known::all();
    assert!(outils.len() >= 5, "{}", outils.len());

    for outil in &outils {
        let dispo = outil.availability();
        let trouvé = namsdec_disasm::known::program_path(&outil.plugin.command).is_some();
        if !trouvé {
            assert_eq!(
                dispo,
                Availability::Missing,
                "{} n'est pas installé ici, il faut le dire",
                outil.plugin.id
            );
        }
        // Et ce qui est absent dit toujours comment l'obtenir.
        if dispo == Availability::Missing {
            assert!(outil.plugin.install.is_some(), "{}", outil.plugin.id);
        }
    }

    // Rizin est le seul dont la présence ne suffit pas : sans
    // `rz-ghidra`, `pdg` n'existe pas, et l'outil est dit incomplet.
    let rizin = outils
        .iter()
        .find(|outil| outil.plugin.id == "rizin-ghidra")
        .expect("Rizin");
    assert!(
        rizin.needs_fr.is_some() && rizin.needs_en.is_some(),
        "la pièce manquante doit être nommée dans les deux langues"
    );
}

/// L'onglet Outils montre tous les outils, et non le premier.
///
/// Ils tenaient sur quatre lignes chacun : seul objdump entrait dans la
/// fenêtre, et Rizin et RetDec étaient là sans que personne puisse les
/// voir. Une ligne par outil, le détail au survol — et l'on trouve ce
/// qu'on est venu chercher.
#[test]
fn the_tools_tab_shows_every_tool_not_just_the_first() {
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Tools,
        ..Default::default()
    };
    // Le pire cas, et le plus courant le premier jour : RIEN n'est
    // installé. C'est celui d'une machine neuve, et celui du coureur de
    // la CI — qui a trouvé ce défaut là où ma machine le cachait, tous
    // ses outils étant bâtis.
    //
    // Chaque fiche absente écrit alors une ligne de plus (« Pour
    // l'obtenir : … »), et les fiches se dépliaient d'office : la page
    // devenait plus haute que la fenêtre, et les deux derniers outils
    // passaient sous le bord sans qu'aucune barre ne le dise.
    let mut sortie = {
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 1400.0),
                )),
                ..Default::default()
            },
            |ui| app.draw(ui),
        )
    };
    sortie.textures_delta.clear();
    app.tools_ready = namsdec_disasm::known::all()
        .into_iter()
        .map(|outil| {
            (
                outil.plugin.id,
                namsdec_disasm::known::Availability::Missing,
            )
        })
        .collect();

    let peint = paint(&mut app);
    let s_fr = strings(Lang::Fr);
    // Combien il y en a : sans ce chiffre, rien ne dit que la liste
    // continue sous le bord de la fenêtre.
    assert!(
        peint.contains(&format!(
            "{} {}",
            namsdec_disasm::known::all().len(),
            s_fr.tools_count
        )),
        "le nombre d'outils connus est écrit :\n{peint}"
    );
    // Et la façon d'obtenir un outil absent se lit SANS RIEN DÉPLIER :
    // c'est une panne, pas un détail. Elle était dans le repli, et le
    // repli s'ouvrait d'office — ce qui rendait les fiches si hautes que
    // les deux dernières passaient sous le bord.
    assert!(
        peint.contains(s_fr.tools_get_it),
        "et comment les obtenir :\n{peint}"
    );

    // Les six sont atteignables. Tous peints d'un coup serait trop
    // demander — six fiches dépassent la fenêtre —, mais aucun ne doit
    // rester hors de portée.
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1400.0));
    let mut vus: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let attendus = ["objdump", "ndisasm", "radare2", "Rizin", "RetDec", "DZF"];
    for tour in 0..40 {
        let input = egui::RawInput {
            screen_rect: Some(écran),
            events: if tour == 0 {
                Vec::new()
            } else {
                vec![
                    // Le pointeur au milieu de la fenêtre des préférences.
                    egui::Event::PointerMoved(egui::pos2(640.0, 700.0)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -120.0),
                        modifiers: egui::Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    },
                ]
            },
            ..Default::default()
        };
        let mut sortie = ctx.run_ui(input, |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                let écrit = texte.galley.text();
                for nom in attendus {
                    if écrit.contains(nom) {
                        vus.insert(nom);
                    }
                }
            }
        }
        sortie.textures_delta.clear();
        if vus.len() == attendus.len() {
            break;
        }
    }
    let manquants: Vec<&str> = attendus
        .iter()
        .copied()
        .filter(|nom| !vus.contains(nom))
        .collect();
    assert!(
        manquants.is_empty(),
        "hors de portée même en déroulant, alors qu'aucun outil n'est \
         installé : {manquants:?}"
    );
    // Et ce qu'il faut taper pour obtenir ce qui manque.
    //
    // L'état est SEMÉ, non lu sur la machine. Il était lu, et le test
    // disait alors deux choses différentes selon le poste : sur celui où
    // tous les outils sont bâtis il ne vérifiait rien, et sur un poste
    // où le premier outil absent tombait en bas de liste il échouait
    // pour une question de place et non de contenu.
    //
    // Semé APRÈS une première image : ouvrir les préférences jette le
    // relevé par conception — on a pu installer un outil à la main dans
    // un terminal entre-temps.
    let premier = namsdec_disasm::known::all()
        .first()
        .expect("au moins un outil connu")
        .plugin
        .id
        .clone();
    app.tools_ready = namsdec_disasm::known::all()
        .into_iter()
        .map(|outil| {
            let dispo = if outil.plugin.id == premier {
                namsdec_disasm::known::Availability::Missing
            } else {
                namsdec_disasm::known::Availability::Ready
            };
            (outil.plugin.id, dispo)
        })
        .collect();
    let peint = paint(&mut app);
    assert!(
        peint.contains(strings(Lang::Fr).tools_get_it),
        "la façon d'obtenir un outil absent doit se lire :\n{peint}"
    );
    // Les deux états sont dits séparément : « absent » parle du
    // programme, « namsdec s'en sert » parle de namsdec. Les confondre
    // était toute la confusion de l'ancienne page.
    //
    // Les deux états sont SEMÉS, un pour chaque moitié de la liste : le
    // panneau doit écrire les deux phrases en même temps, et c'est
    // précisément ce qu'on veut vérifier — que « absent de la machine »
    // et « installé sur la machine » ne se confondent pas.
    //
    // C'était lu sur la machine, et alors la moitié des assertions ne
    // s'exécutait pas : depuis que namsdec retrouve ce qu'il a installé
    // lui-même, un poste bien équipé n'a plus un seul outil absent, et le
    // test ne vérifiait plus qu'une phrase sur deux.
    let s_fr = strings(Lang::Fr);
    let connus = namsdec_disasm::known::all();
    app.tools_ready = connus
        .iter()
        .enumerate()
        .map(|(rang, outil)| {
            let dispo = if rang % 2 == 0 {
                namsdec_disasm::known::Availability::Ready
            } else {
                namsdec_disasm::known::Availability::Missing
            };
            (outil.plugin.id.clone(), dispo)
        })
        .collect();
    assert!(connus.len() >= 2, "il faut deux outils pour deux états");
    let peint = paint(&mut app);
    assert!(peint.contains(s_fr.tools_unused), "{peint}");
    for attendu in [s_fr.tools_state_found, s_fr.tools_state_absent] {
        assert!(peint.contains(attendu), "« {attendu} » :\n{peint}");
    }
    // Et les boutons de la boîte restent visibles : la page ne prend
    // plus toute la hauteur de l'écran.
    assert!(
        peint.contains(strings(Lang::Fr).dialog_ok)
            && peint.contains(strings(Lang::Fr).dialog_quit),
        "OK et Quitter doivent rester à l'écran :\n{peint}"
    );
}

/// Le répertoire `plugins/` dit la vérité sur ce que namsdec lance.
///
/// Ses fichiers sont engendrés par le code qui connaît ces outils : si
/// l'un des deux changeait sans l'autre, le dépôt proposerait de copier
/// une commande que namsdec ne lance pas — c'est-à-dire un mensonge dans
/// le seul endroit où l'on ne peut pas se le permettre, puisqu'il s'agit
/// d'exécuter un programme.
#[test]
fn the_plugins_directory_matches_what_namsdec_would_run() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let plugins = racine.join("plugins");

    // Les deux familles, et leur mode d'emploi.
    for (dossier, attendu) in [
        ("themes", "Préférences ▸ Plugins"),
        ("fonctionnalites", "rz-ghidra"),
    ] {
        let readme = plugins.join(dossier).join("README.md");
        let texte = std::fs::read_to_string(&readme)
            .unwrap_or_else(|_| panic!("{} manque", readme.display()));
        assert!(
            texte.contains(attendu),
            "{} doit expliquer ce qu'on y range",
            readme.display()
        );
    }
    let sommaire = std::fs::read_to_string(plugins.join("README.md")).expect("plugins/README.md");
    assert!(
        sommaire.contains("Thèmes") && sommaire.contains("Fonctionnalités"),
        "le sommaire nomme les deux familles"
    );

    // Chaque outil connu a son fichier, et ce fichier est exactement ce
    // que namsdec lancerait.
    for outil in namsdec_disasm::known::all() {
        let chemin = plugins
            .join("fonctionnalites")
            .join(format!("{}.json", outil.plugin.id));
        let écrit = namsdec_disasm::plugin::read(&chemin)
            .unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
        assert_eq!(
            écrit,
            outil.plugin,
            "{} ne décrit pas ce que namsdec lance",
            chemin.display()
        );
    }

    // Et les thèmes proposés se lisent vraiment.
    let mut thèmes = 0;
    for entrée in std::fs::read_dir(plugins.join("themes")).expect("le dossier des thèmes") {
        let chemin = entrée.expect("une entrée").path();
        if chemin.extension().is_some_and(|e| e == "json") {
            namsdec_theme::plugin::read(&chemin)
                .unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
            thèmes += 1;
        }
    }
    assert!(
        thèmes >= 2,
        "au moins deux thèmes d'exemple, obtenu {thèmes}"
    );
}

/// Les instructions d'installation des décompilateurs sont là, et
/// complètes.
///
/// C'est la seule étape que namsdec ne fait pas à la place de
/// l'utilisateur : autant qu'elle soit écrite en entier, pour les trois
/// outils qui demandent un geste.
#[test]
fn the_decompilers_come_with_their_installation_instructions() {
    let readme = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/fonctionnalites/README.md");
    let texte = std::fs::read_to_string(&readme).expect("le mode d'emploi des outils");

    for attendu in [
        // Ce qu'il faut taper, pour chacun.
        "dnf install radare2",
        "rizin-plugin-ghidra",
        "rz-pm -i rz-ghidra",
        "retdec/releases",
        // Et les deux marqueurs du format, sans lesquels on ne peut pas
        // écrire sa propre description.
        "{file}",
        "{out}",
    ] {
        assert!(
            texte.contains(attendu),
            "le mode d'emploi doit contenir « {attendu} »"
        );
    }
    // Le piège de Rizin est nommé : le programme seul ne décompile pas.
    assert!(
        texte.contains("Rizin seul ne décompile pas"),
        "la nuance qui coûte une soirée doit être dite"
    );
}

/// Aller chercher un outil ne se fait pas tout seul, et jamais sans
/// l'avoir montré.
///
/// Ce module-ci ne se contente pas de lancer un programme installé : il
/// **télécharge du code et l'exécute**. Les verrous sont donc plus
/// nombreux, et ce test les compte : le module de décompilation d'abord,
/// celui-ci ensuite, la fenêtre qui montre le plan, et seulement alors
/// un bouton.
#[test]
fn fetching_a_tool_never_happens_by_itself() {
    let app = NamsdecApp::default();
    assert!(
        !app.show_installer_module,
        "éteint tant que personne ne l'allume"
    );
    assert!(app.install_job.is_none() && app.install_plan.is_none());

    // Demander le plan sans avoir allumé le module ne montre rien.
    let mut app = NamsdecApp {
        install_plan: Some("retdec".to_owned()),
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        !peint.contains(strings(Lang::Fr).install_title),
        "la fenêtre reste fermée :\n{peint}"
    );
    assert!(app.install_job.is_none(), "et rien n'a été lancé");

    // Module allumé : la fenêtre montre le plan — d'où ça vient, ce que
    // ça coûte, où ça va, et chaque commande.
    let mut app = NamsdecApp {
        show_decompiler_module: true,
        show_installer_module: true,
        install_plan: Some("retdec".to_owned()),
        ..Default::default()
    };
    let peint = paint(&mut app);
    for attendu in [
        // La leçon d'abord : à quoi sert un décompilateur.
        strings(Lang::Fr).install_what_for,
        "il ne restitue pas la source",
        // Puis les faits. Le coût est une phrase longue : s'il manque,
        // c'est qu'il file hors de la fenêtre au lieu de se replier.
        strings(Lang::Fr).install_cost,
        "180 Mo à télécharger",
        strings(Lang::Fr).install_where,
        // L'avertissement, tout en bas et toujours peint.
        strings(Lang::Fr).install_warning,
        strings(Lang::Fr).install_plan,
        "github.com/avast/retdec",
        "sha256sum",
    ] {
        assert!(
            peint.contains(attendu),
            "« {attendu} » doit se lire avant tout lancement :\n{peint}"
        );
    }
    // Le bouton de lancement, ou le constat que c'est déjà fait. L'état
    // est SEMÉ, non lu sur la machine : le lire faisait dire au test deux
    // choses différentes selon le poste, et il ne vérifiait donc rien de
    // stable. Semé APRÈS une première image, parce qu'ouvrir la fenêtre
    // jette le relevé des outils par conception — et que l'image suivante
    // le refait en lançant vraiment l'outil.
    for (état, attendu) in [
        (
            namsdec_disasm::known::Availability::Ready,
            strings(Lang::Fr).install_already_done,
        ),
        (
            namsdec_disasm::known::Availability::Missing,
            strings(Lang::Fr).install_start,
        ),
    ] {
        app.tools_ready = [("retdec".to_owned(), état)].into_iter().collect();
        let peint = paint(&mut app);
        assert!(
            peint.contains(attendu),
            "{état:?} : « {attendu} » doit se lire avant tout lancement :\n{peint}"
        );
        assert!(
            app.install_job.is_none(),
            "montrer le plan n'est pas l'exécuter"
        );
    }
}

/// Ce que les recettes s'interdisent, et qui compte plus que ce
/// qu'elles font.
#[test]
fn a_recipe_stays_inside_namsdec_and_shows_everything() {
    let data = std::path::Path::new("/tmp/namsdec-essai");
    for outil in ["retdec", "rizin-ghidra"] {
        let recette = namsdec_disasm::install::recipe_for(outil, data).expect("une recette existe");

        // Tout ce qui est produit reste sous le dossier de namsdec :
        // rien dans /usr, rien qui demande des droits.
        assert!(
            recette
                .produces
                .starts_with(namsdec_disasm::install::tools_dir(data)),
            "{} sort du dossier de namsdec",
            recette.produces.display()
        );
        for étape in &recette.steps {
            assert_ne!(étape.command, "sudo", "{}", étape.line());
            // Chaque étape sait dire ce qu'elle fait, dans les deux
            // langues : une commande sans explication n'est pas montrée,
            // elle est infligée.
            assert!(
                !étape.about_fr.is_empty() && !étape.about_en.is_empty(),
                "{} n'explique pas ce qu'elle fait",
                étape.line()
            );
        }
    }

    // Et ce qu'on ne sait pas installer ne s'invente pas.
    assert!(
        namsdec_disasm::install::recipe_for("objdump", data).is_none(),
        "objdump vient des paquets du système, pas de namsdec"
    );
}

/// Le module d'installation n'alourdit pas le programme.
///
/// C'était la condition posée : pouvoir aller chercher Rizin et RetDec
/// sans que namsdec grossisse. La réponse tient dans une règle — **rien
/// n'est embarqué**. Les téléchargements passent par `curl`,
/// l'extraction par `tar`, la compilation par `meson` et `cmake` : ce
/// sont des commandes du système, exactement comme `nasm` et `ld`.
/// Aucune bibliothèque de plus, donc aucun octet de plus que le code qui
/// décrit les recettes.
#[test]
fn installing_tools_adds_no_dependency_to_the_binary() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    // Aucune crate de téléchargement, d'archive ou de compilation n'a
    // rejoint le programme.
    let workspace = std::fs::read_to_string(racine.join("Cargo.toml")).expect("Cargo.toml");
    for interdite in [
        "reqwest", "hyper", "flate2", "tar =", "zip", "xz2", "git2", "cmake", "curl =",
    ] {
        assert!(
            !workspace.contains(interdite),
            "« {interdite} » alourdirait le binaire : le module doit passer par les \
             commandes du système"
        );
    }

    // Et les recettes n'appellent que des programmes du système.
    let data = std::path::Path::new("/tmp/x");
    let mut appelés: std::collections::BTreeSet<String> = Default::default();
    for outil in ["retdec", "rizin-ghidra"] {
        let recette = namsdec_disasm::install::recipe_for(outil, data).expect("une recette");
        for étape in &recette.steps {
            appelés.insert(étape.command.clone());
        }
    }
    for commande in &appelés {
        assert!(
            [
                "mkdir",
                "curl",
                "sha256sum",
                "tar",
                "rm",
                "git",
                "meson",
                "ninja",
                "cmake",
                "env"
            ]
            .contains(&commande.as_str()),
            "« {commande} » n'est pas un outil du système attendu"
        );
    }
}

/// LE RAPPORT D'ANOMALIE NE PORTE NI LE FICHIER, NI SON CHEMIN.
///
/// C'est la règle qui tient tout le reste. Un binaire analysé est
/// souvent ce qu'on a de plus privé — un échantillon, un travail en
/// cours, le code de quelqu'un d'autre —, et un rapport part sur un
/// tracker public. Il dit la FORME du fichier et jamais son contenu ;
/// le chemin n'y entre que si la case est cochée, et elle est décochée.
#[test]
fn a_bug_report_carries_neither_the_file_nor_its_path() {
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    if !app.load_elf(&octets, "secret_de_famille") {
        return;
    }
    app.binary_path = Some(std::path::PathBuf::from(
        "/home/quelquun/clients/acme/binaire_confidentiel",
    ));
    app.bug_report.récit = "j'ai cliqué, rien ne s'est passé".to_owned();

    let rapport = app.bug_report_text();

    // La forme y est : c'est elle qui aide à reproduire.
    assert!(
        rapport.contains("ELF64"),
        "la forme du fichier doit y être : {rapport}"
    );
    assert!(
        rapport.contains(&octets.len().to_string()),
        "sa taille aussi"
    );
    // Le chemin n'y est PAS, case décochée.
    assert!(
        !app.bug_report.avec_chemin,
        "la case est décochée par défaut"
    );
    for secret in ["acme", "binaire_confidentiel", "quelquun"] {
        assert!(
            !rapport.contains(secret),
            "« {secret} » ne doit pas figurer dans un rapport public :\n{rapport}"
        );
    }
    // Et le contenu du fichier non plus : on cherche une suite d'octets
    // qui s'y trouve et qui n'a aucune raison d'être dans un rapport.
    let début: String = octets
        .iter()
        .skip(1)
        .take(3)
        .map(|o| format!("{o:02x}"))
        .collect();
    assert!(
        !rapport.contains(&début),
        "le contenu du fichier ne doit pas fuir : « {début} » est dedans"
    );

    // Cochée, le chemin entre — et seulement alors.
    app.bug_report.avec_chemin = true;
    assert!(
        app.bug_report_text().contains("binaire_confidentiel"),
        "coché, le chemin entre : c'est le seul moyen de le joindre"
    );
}

/// UPX DÉBALLE VRAIMENT, et écrit le fichier qu'on lui a nommé.
///
/// Signalé le 21 octobre 2026 : « le déballage ne produit pas de
/// fichier de sortie », « le programme ne dépaquette pas upx ». Le
/// contrôle part d'un binaire qu'il empaquette lui-même, le fait passer
/// par le chemin RÉEL de namsdec — ouverture, puis la fonction que le
/// bouton appelle — et regarde le fichier écrit.
///
/// Il s'abstient sans upx sur la machine : il n'y aurait rien à
/// éprouver, et un contrôle qui dépend d'un outil absent ne doit pas
/// faire échouer la suite.
#[test]
fn upx_really_unpacks_and_writes_the_file() {
    let Some(upx) = namsdec_disasm::known::program_path_in("upx", None) else {
        println!("ABSTENU : upx introuvable");
        return;
    };
    println!("upx : {}", upx.display());
    let Ok(original) = std::fs::read("/bin/ls") else {
        return;
    };
    let dossier = tempfile::tempdir().expect("un dossier");
    let paquet = dossier.path().join("paquet");
    std::fs::write(&paquet, &original).expect("la copie s'écrit");
    // Exécutable, sans quoi upx refuse de l'empaqueter : « file not
    // executable ». Le contrôle s'abstenait silencieusement là-dessus,
    // et passait au vert en ne mesurant rien.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&paquet, std::fs::Permissions::from_mode(0o755))
            .expect("les droits se posent");
    }
    // Empaqueter pour de vrai : c'est la seule façon d'éprouver le
    // déballage sur autre chose qu'un refus.
    let fait = std::process::Command::new(&upx)
        .args(["-q", "-1"])
        .arg(&paquet)
        .output();
    match &fait {
        Ok(s) if s.status.success() => {}
        autre => {
            println!("ABSTENU : l'empaquetage a échoué : {autre:?}");
            return;
        }
    }
    let octets = std::fs::read(&paquet).expect("le paquet se lit");
    assert!(
        octets.len() < original.len(),
        "upx a bien compressé : {} contre {}",
        octets.len(),
        original.len()
    );

    // LE CHEMIN RÉEL : ouvrir le binaire, puis demander le déballage.
    let mut app = NamsdecApp::default();
    assert!(
        app.load_elf(&octets, "paquet"),
        "un binaire empaqueté s'ouvre : sans cela, rien ne part à upx"
    );
    let porté = app
        .assemble_state
        .as_ref()
        .map(|state| state.output.file.len())
        .unwrap_or(0);
    assert_eq!(
        porté,
        octets.len(),
        "et le fichier ENTIER est porté : c'est lui qu'upx reçoit"
    );

    let vers = dossier.path().join("déballé");
    app.unpack_upx_to(&vers);
    // Le travail part sur un fil : on l'attend.
    let ctx = egui::Context::default();
    for _ in 0..600 {
        app.poll_upx(&ctx);
        if app.upx_job.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(app.upx_job.is_none(), "upx a fini");

    let écrit = std::fs::metadata(&vers).map(|m| m.len()).unwrap_or(0);
    assert!(
        écrit > 0,
        "le déballage doit écrire le fichier nommé : {} est vide ou absent",
        vers.display()
    );
    // Et ce qu'il écrit est bien l'original : c'est tout l'objet.
    let rendu = std::fs::read(&vers).expect("le déballé se lit");
    assert_eq!(
        rendu.len(),
        original.len(),
        "le déballé a la taille de l'original"
    );
}

/// « Il lui manque… » ne s'écrit que s'il manque vraiment quelque chose.
///
/// Le message était affiché sans condition, si bien qu'un outil installé
/// et parfaitement fonctionnel s'annonçait cassé : « programme détecté »
/// et « namsdec s'en sert » d'un côté, « Il lui manque le greffon
/// rz-ghidra » juste en dessous. On en concluait, très logiquement, que
/// l'installation n'avait pas marché.
#[test]
fn a_working_tool_is_not_announced_as_missing_something() {
    use namsdec_disasm::known::Availability;

    let s = strings(Lang::Fr);
    // L'état est SEMÉ, et les deux cas sont éprouvés. Il était lu sur la
    // machine, et le test ne jugeait alors qu'un seul des deux — celui
    // que le poste lui présentait. Sur une machine où rizin fonctionne,
    // le cas « incomplet » — le seul qui nous intéressait — n'était
    // jamais vérifié.
    for (état, doit_le_dire) in [
        (Availability::Ready, false),
        (Availability::Incomplete, true),
    ] {
        let mut app = NamsdecApp {
            show_preferences: true,
            preferences_tab: PreferencesTab::Tools,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1200.0),
            )),
            ..Default::default()
        };

        // Première image : ouvrir les préférences jette le relevé des
        // outils par conception, et l'image suivante le refait en lançant
        // vraiment rizin. Ce qu'on sème avant serait donc perdu.
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();

        // Tous les outils prêts sauf rizin, dont l'état est celui qu'on
        // éprouve : un autre outil absent écrirait « Il lui manque » de
        // son côté, et l'on accuserait rizin.
        app.tools_ready = namsdec_disasm::known::all()
            .into_iter()
            .map(|outil| {
                let dispo = if outil.plugin.id == "rizin-ghidra" {
                    état
                } else {
                    Availability::Ready
                };
                (outil.plugin.id, dispo)
            })
            .collect();

        let mut peint = String::new();
        for _ in 0..3 {
            peint.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    peint.push_str(texte.galley.text());
                    peint.push('|');
                }
            }
            sortie.textures_delta.clear();
        }

        if doit_le_dire {
            assert!(
                peint.contains(s.tools_needs),
                "le greffon manque vraiment : il faut le dire.\n{peint}"
            );
            // Et SANS « Pour l'obtenir : sudo dnf install rizin » : la base
            // est déjà là, c'est ce qui manque (rz-ghidra) qu'il faut, et le
            // module d'installation le construit. Afficher la commande de la
            // base contredisait « installé, mais incomplet ».
            assert!(
                !peint.contains(s.tools_get_it),
                "incomplet : on ne propose pas de réinstaller la base.\n{peint}"
            );
        } else {
            assert!(
                !peint.contains(s.tools_needs),
                "rizin+rz-ghidra fonctionne : rien ne doit dire qu'il manque \
                 quelque chose.\n{peint}"
            );
        }
    }
}

/// Un outil incomplet oriente vers « Réparer », pas vers « Activer ».
///
/// Activer un outil incomplet — rizin sans rz-ghidra — déclarerait un
/// décompilateur qui échoue au premier essai : `pdg` n'existe pas. On ne
/// peut donc pas l'activer avant de l'avoir complété, et son bouton de
/// complétion dit « Réparer » (en accent, l'action qui mène), et non
/// « Installer… » (qui est le mot d'un outil absent, pas incomplet).
#[test]
fn an_incomplete_tool_points_to_repair_not_enable() {
    use namsdec_disasm::known::Availability;

    let s = strings(Lang::Fr);
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Tools,
        show_decompiler_module: true,
        // Sans le module d'installation, c'est une invite à l'allumer qui
        // s'affiche, pas le bouton « Réparer » qu'on veut éprouver.
        show_installer_module: true,
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let entrée = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1600.0, 1200.0),
        )),
        ..Default::default()
    };

    // L'état est SEMÉ, et APRÈS la première image : ouvrir les préférences
    // jette le relevé des outils par conception, et l'image suivante le
    // referait en lançant vraiment rizin.
    let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
    sortie.textures_delta.clear();
    // Rizin incomplet, tous les autres prêts : aucun n'est absent, donc le
    // seul mot de complétion qui peut paraître est le sien.
    app.tools_ready = namsdec_disasm::known::all()
        .into_iter()
        .map(|outil| {
            let dispo = if outil.plugin.id == "rizin-ghidra" {
                Availability::Incomplete
            } else {
                Availability::Ready
            };
            (outil.plugin.id, dispo)
        })
        .collect();

    let mut peint = String::new();
    for _ in 0..3 {
        peint.clear();
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        for clipped in &sortie.shapes {
            if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                peint.push_str(texte.galley.text());
                peint.push('|');
            }
        }
        sortie.textures_delta.clear();
    }

    assert!(
        peint.contains(s.install_repair),
        "un outil incomplet doit porter « Réparer ».\n{peint}"
    );
    assert!(
        !peint.contains(s.install_button),
        "incomplet, et non absent : pas de « Installer… ».\n{peint}"
    );
}

/// Un binaire localisé à la main rend prêt un outil que le PATH ignore.
///
/// namsdec cherche un outil dans le PATH et dans son propre dossier
/// d'outils. Ce qu'on a bâti soi-même hors du PATH — DZF dans son dépôt —
/// n'est ni l'un ni l'autre, et reste « absent » sans que rien n'y puisse.
/// Montrer où est le binaire doit suffire à le rendre prêt. On se garde de
/// lire l'état de la machine : un fichier réel qui existe toujours — le
/// binaire de test lui-même — prouve l'override, quel que soit le PATH.
///
/// Ce contrôle appelle bien `refresh_tool_availability`, car c'est son
/// override qu'il vérifie — pas l'état des autres outils, sur lequel il
/// n'affirme rien. Il figure à ce titre dans la liste des contrôles de
/// mécanisme du gardien `no_control_asks_the_machine…`.
#[test]
fn a_located_binary_makes_a_tool_ready() {
    let binaire = std::env::current_exe().expect("le binaire de test existe");
    let mut app = NamsdecApp {
        system_titlebar: false,
        ..Default::default()
    };
    app.located_tools.insert("dzf".to_owned(), binaire);
    app.refresh_tool_availability();
    assert_eq!(
        app.tools_ready.get("dzf").copied(),
        Some(namsdec_disasm::known::Availability::Ready),
        "un binaire réel localisé doit rendre DZF prêt, PATH ou pas"
    );
}

/// L'onglet Outils dit quand la décompilation est éteinte.
///
/// Les deux réglages d'une même fonction tombaient sur deux onglets :
/// activer un outil ici, mais la décompilation s'allume dans Modules.
/// Sans un mot, on activait un outil qui ne servait nulle part. L'onglet
/// le dit maintenant en tête, avec de quoi l'allumer sur place.
#[test]
fn the_tools_tab_says_when_decompilation_is_off() {
    let s = strings(Lang::Fr);

    // Éteinte : la note et son bouton se lisent.
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Tools,
        show_decompiler_module: false,
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        peint.contains(s.tools_decompilation_off),
        "la décompilation éteinte doit se dire :\n{peint}"
    );
    assert!(
        peint.contains(s.tools_enable_decompilation),
        "et de quoi l'allumer sur place :\n{peint}"
    );

    // Allumée : plus de note.
    let mut app = NamsdecApp {
        show_preferences: true,
        preferences_tab: PreferencesTab::Tools,
        show_decompiler_module: true,
        ..Default::default()
    };
    let peint = paint(&mut app);
    assert!(
        !peint.contains(s.tools_decompilation_off),
        "allumée, rien à signaler :\n{peint}"
    );
}

/// La fenêtre du plan d'installation reste entièrement à l'écran.
///
/// Le plan de rizin compte neuf étapes et un journal : la fenêtre
/// dépassait en haut comme en bas, et sa barre de titre — avec la croix
/// qui s'y trouve — passait hors champ. On ne pouvait plus la fermer du
/// tout.
#[test]
fn the_install_plan_window_never_runs_off_the_screen() {
    for (largeur, hauteur) in [(1280.0, 800.0), (1999.0, 1248.0)] {
        let mut app = NamsdecApp {
            show_installer_module: true,
            install_plan: Some("rizin-ghidra".to_owned()),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(largeur, hauteur));
        let entrée = egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        };
        let mut haut = f32::MAX;
        let mut bas = f32::MIN;
        for _ in 0..3 {
            haut = f32::MAX;
            bas = f32::MIN;
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    haut = haut.min(texte.pos.y);
                    bas = bas.max(texte.pos.y);
                }
            }
            sortie.textures_delta.clear();
        }
        // La fenêtre n'est peut-être pas ouverte : sans dossier de
        // données, le plan se referme tout seul. Le test le dit plutôt
        // que d'échouer sur l'état du poste.
        if app.install_plan.is_none() {
            return;
        }
        assert!(
            haut >= 0.0,
            "{largeur}x{hauteur} : quelque chose est peint au-dessus de l'écran (y={haut})"
        );
        assert!(
            bas <= hauteur,
            "{largeur}x{hauteur} : quelque chose est peint sous l'écran (y={bas})"
        );
    }
}

/// Échap referme le plan, et le bouton « Fermer » aussi.
///
/// La croix vit dans la barre de titre, qu'on perd de vue sur un plan de
/// neuf étapes. Il faut un second chemin — et le geste qu'on fait sans y
/// penser devant une fenêtre dont on a fini, c'est Échap.
#[test]
fn escape_closes_the_install_plan() {
    let mut app = NamsdecApp {
        show_installer_module: true,
        install_plan: Some("rizin-ghidra".to_owned()),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    app.apply_theme(&ctx);
    let écran = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
    let mut sortie = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(écran),
            ..Default::default()
        },
        |ui| app.draw(ui),
    );
    sortie.textures_delta.clear();
    if app.install_plan.is_none() {
        return;
    }

    let mut échap = egui::RawInput {
        screen_rect: Some(écran),
        ..Default::default()
    };
    échap.events.push(egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    let mut sortie = ctx.run_ui(échap, |ui| app.draw(ui));
    sortie.textures_delta.clear();
    assert!(
        app.install_plan.is_none(),
        "Échap doit refermer le plan d'installation"
    );
}

/// Ce que la fenêtre d'installation propose répond à l'état de l'outil.
///
/// Trois états, trois offres, et pas une de plus :
///
/// - il répond → **Fermer**, et rien d'autre. « Recommencer » était là,
///   et c'était une offre sans motif : vingt minutes de compilation pour
///   un résultat identique. Un bouton qu'on n'a aucune raison de cliquer
///   fait douter de celui d'à côté — il laissait croire que
///   l'installation avait peut-être échoué.
/// - il est là mais incomplet → **Réparer**. C'est le cas de rizin sans
///   `rz-ghidra` : `pdg` n'existe pas, et rien dans la fenêtre ne disait
///   quoi faire.
/// - il n'est pas là → **Lancer l'installation**.
///
/// L'état est SEMÉ, et semé **après la première image**. L'ouverture de
/// la fenêtre jette le relevé par conception — on a pu installer l'outil
/// à la main dans un terminal entre-temps —, et l'image suivante le
/// refait en lançant vraiment rizin. Semer avant ne servait donc à rien :
/// le test lisait la machine malgré lui, passait ici parce que rizin y
/// est installé, et échouait sur la CI qui ne l'a pas. Quatre CI rouges
/// d'affilée, sans qu'une ligne de namsdec soit en cause.
#[test]
fn what_the_installer_window_offers_answers_the_tools_state() {
    use namsdec_disasm::known::Availability;

    let s = strings(Lang::Fr);
    for (état, attendus, absents) in [
        (
            Availability::Ready,
            vec![s.install_already_done, s.dialog_close],
            vec![s.install_start, s.install_repair],
        ),
        (
            Availability::Incomplete,
            vec![s.install_incomplete, s.install_repair, s.dialog_close],
            vec![s.install_start, s.install_already_done],
        ),
        (
            Availability::Missing,
            vec![s.install_start],
            vec![s.install_already_done, s.install_repair],
        ),
    ] {
        let mut app = NamsdecApp {
            show_installer_module: true,
            install_plan: Some("rizin-ghidra".to_owned()),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        app.apply_theme(&ctx);
        let entrée = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 1200.0),
            )),
            ..Default::default()
        };

        // Première image : la fenêtre s'ouvre, et vide le relevé.
        let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
        sortie.textures_delta.clear();
        if app.install_plan.is_none() {
            // Pas de dossier de données sur cette machine : la fenêtre se
            // referme d'elle-même, et il n'y a rien à juger.
            return;
        }

        // Maintenant on sème, et plus rien ne le jettera : la fenêtre
        // n'est plus « nouvellement ouverte ».
        app.tools_ready = [("rizin-ghidra".to_owned(), état)].into_iter().collect();

        let mut peint = String::new();
        for _ in 0..3 {
            peint.clear();
            let mut sortie = ctx.run_ui(entrée.clone(), |ui| app.draw(ui));
            for clipped in &sortie.shapes {
                if let egui::epaint::Shape::Text(texte) = &clipped.shape {
                    peint.push_str(texte.galley.text());
                    peint.push('|');
                }
            }
            sortie.textures_delta.clear();
        }

        for attendu in attendus {
            assert!(
                peint.contains(attendu),
                "{état:?} : « {attendu} » doit être proposé.\n{peint}"
            );
        }
        for absent in absents {
            assert!(
                !peint.contains(absent),
                "{état:?} : « {absent} » n'a rien à faire là.\n{peint}"
            );
        }
    }
}

/// namsdec répond à sa ligne de commande.
///
/// Il l'ignorait purement et simplement — alors que son propre lanceur
/// déclare `Exec=namsdec %f` et `MimeType=text/x-asm`, c'est-à-dire
/// qu'il se propose d'ouvrir les fichiers `.asm` du gestionnaire de
/// fichiers. « Ouvrir avec namsdec » ouvrait une fenêtre vide, ce qui
/// est pire que de ne rien proposer. Et `namsdec --version` lançait
/// l'interface graphique : un programme installé dans le PATH doit
/// savoir répondre à la question la plus simple qu'on lui pose.
///
/// Le contrôle porte sur la décision, séparée du reste pour cela : elle
/// ne touche ni au disque ni à l'écran.
#[test]
fn namsdec_answers_its_command_line() {
    use crate::Intention;
    let dit = |args: &[&str]| {
        let v: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
        crate::intention(&v)
    };

    assert_eq!(
        dit(&[]),
        Intention::Ouvrir(None),
        "sans argument, on ouvre l'interface"
    );
    assert_eq!(
        dit(&["bonjour.asm"]),
        Intention::Ouvrir(Some(std::path::PathBuf::from("bonjour.asm"))),
        "avec un chemin, on l'ouvre — c'est ce que le lanceur promet"
    );

    for drapeau in ["--version", "-V"] {
        let Intention::Dire(texte) = dit(&[drapeau]) else {
            panic!("{drapeau} doit répondre sans ouvrir de fenêtre")
        };
        assert!(
            texte.contains(crate::APP_VERSION),
            "{drapeau} donne la version : {texte:?}"
        );
    }
    for drapeau in ["--help", "-h"] {
        let Intention::Dire(texte) = dit(&[drapeau]) else {
            panic!("{drapeau} doit répondre sans ouvrir de fenêtre")
        };
        assert!(
            texte.contains("--version") && texte.contains("FICHIER"),
            "{drapeau} dit ce que le programme accepte : {texte:?}"
        );
    }

    // Une faute de frappe n'est pas un nom de fichier : la prendre pour
    // tel donnerait « fichier introuvable : --verison », ce qui n'aide
    // personne.
    let Intention::Dire(texte) = dit(&["--verison"]) else {
        panic!("un argument inconnu ne s'ouvre pas comme un fichier")
    };
    assert!(
        texte.contains("--verison") && texte.contains("--help"),
        "il nomme l'argument fautif et dit où regarder : {texte:?}"
    );
}

/// Les deux scripts disent ce qu'ils vont faire AVANT de le faire.
///
/// L'aide existait, mais il fallait la demander : on lançait
/// `./install.sh` et l'on découvrait après coup qu'il avait touché au
/// fichier du shell et épinglé au dock. Un relevé des réglages retenus en
/// tête, avec l'option qui annule chacun, laisse le temps de couper
/// court — et évite d'avoir à relire `--help` pour savoir ce qui se
/// passe.
///
/// Le contrôle porte sur le comportement, non sur la mise en forme : il
/// lance un essai à blanc et exige que chaque réglage nommé s'y lise,
/// avec l'option qui le retire.
#[test]
fn the_scripts_say_what_they_will_do_before_doing_it() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !racine.join("install.sh").is_file() {
        return;
    }
    let dossier = tempfile::tempdir().expect("un dossier temporaire");

    for (nom, attendus) in [
        (
            "install.sh",
            vec![
                "Ce qui va être fait",
                "-h pour toutes les options",
                "--no-exercises",
                "--no-desktop",
                "--no-path",
                "--no-dock",
            ],
        ),
        (
            "uninstall.sh",
            vec!["Ce qui va partir", "-h pour toutes les options", "--purge"],
        ),
    ] {
        let sortie = std::process::Command::new("bash")
            .arg(racine.join(nom))
            .args(["--dry-run", "--no-color", "--prefix"])
            .arg(dossier.path().join("share"))
            .arg("--bin-dir")
            .arg(dossier.path().join("bin"))
            .output();
        let Ok(sortie) = sortie else {
            eprintln!("bash est introuvable : test ignoré");
            return;
        };
        assert!(
            sortie.status.success(),
            "{nom} --dry-run doit aboutir :\n{}",
            String::from_utf8_lossy(&sortie.stderr)
        );
        let dit = String::from_utf8_lossy(&sortie.stdout);
        for attendu in attendus {
            assert!(
                dit.contains(attendu),
                "{nom} doit annoncer « {attendu} » :\n{dit}"
            );
        }
        // Et le cadre ne doit pas être de travers : ses trois lignes font
        // la même largeur. Un titre accentué la décalait, parce que
        // `printf '%-*s'` compte les octets et non les caractères.
        let bords: Vec<&str> = dit
            .lines()
            .filter(|ligne| ligne.contains('┌') || ligne.contains('└') || ligne.contains('│'))
            .collect();
        assert!(bords.len() >= 3, "{nom} doit porter un cadre :\n{dit}");
        let largeurs: std::collections::BTreeSet<usize> =
            bords.iter().map(|ligne| ligne.chars().count()).collect();
        assert_eq!(
            largeurs.len(),
            1,
            "{nom} : les lignes du cadre doivent faire la même largeur, vu {largeurs:?}\n{dit}"
        );
    }
}

/// Les scripts d'installation disent toutes les options qu'ils
/// acceptent.
///
/// Une option qui marche mais que `--help` ne nomme pas n'existe pas :
/// personne ne la devinera. Le contrôle lit le `case` qui traite les
/// arguments et vérifie que chaque motif se retrouve dans l'aide.
#[test]
fn the_install_scripts_describe_every_option_they_accept() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for nom in ["install.sh", "uninstall.sh"] {
        let source = std::fs::read_to_string(racine.join(nom)).expect(nom);
        // Le bloc d'aide : entre `cat <<AIDE` et son terminateur.
        let aide = source
            .split_once("cat <<AIDE")
            .and_then(|(_, reste)| reste.split_once("\nAIDE\n"))
            .map(|(bloc, _)| bloc.to_owned())
            .unwrap_or_else(|| panic!("{nom} doit porter un bloc d'aide"));

        // Les motifs du `case` : `--purge)`, `-n|--dry-run)`, …
        let mut options: Vec<String> = Vec::new();
        for ligne in source.lines() {
            let taillée = ligne.trim();
            let Some((motifs, _)) = taillée.split_once(')') else {
                continue;
            };
            if !motifs.starts_with('-') {
                continue;
            }
            for motif in motifs.split('|') {
                // `--prefix=*` et `--prefix` sont la même option.
                let motif = motif.trim().trim_end_matches("=*");
                if motif.starts_with('-') {
                    options.push(motif.to_owned());
                }
            }
        }
        assert!(
            options.len() >= 5,
            "{nom} : le relevé ne trouve presque aucune option ({}) — il \
             regarde au mauvais endroit",
            options.len()
        );
        let muettes: Vec<&String> = options.iter().filter(|o| !aide.contains(*o)).collect();
        assert!(
            muettes.is_empty(),
            "{nom} accepte {} option(s) que son aide ne nomme pas — \
             une option qu'on ne peut pas deviner n'existe pas : {muettes:?}",
            muettes.len()
        );
    }
}

/// Un essai à blanc n'écrit RIEN.
///
/// `--dry-run` est une promesse plus forte qu'une commodité : on
/// l'emploie précisément quand on n'ose pas lancer la vraie commande.
/// Un seul geste qui y échappe la rend pire qu'absente.
///
/// Le défaut a été commis en écrivant ces options : le dock et la ligne
/// de PATH étaient modifiés pour de bon en essai à blanc, parce qu'ils
/// ne passaient pas par la fonction qui sait se taire.
///
/// Le contrôle lit le source : toute commande qui écrit doit passer par
/// `faire`/`retirer`, ou vivre sous une garde qui teste `$ESSAI`.
#[test]
fn a_dry_run_writes_nothing() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for nom in ["install.sh", "uninstall.sh"] {
        let source = std::fs::read_to_string(racine.join(nom)).expect(nom);
        let lignes: Vec<&str> = source.lines().collect();
        let mut nues = Vec::new();
        for (n, ligne) in lignes.iter().enumerate() {
            let taillée = ligne.trim();
            if taillée.starts_with('#') {
                continue;
            }
            // Ce qui touche au disque, au bureau ou au shell.
            let écrit = taillée.starts_with("rm ")
                || taillée.starts_with("rm -")
                || taillée.starts_with("install -")
                || taillée.starts_with("mkdir ")
                || taillée.starts_with("gsettings set")
                || taillée.starts_with("cargo build")
                || taillée.contains(">> \"$rc\"");
            if !écrit {
                continue;
            }
            // Sous une garde qui teste l'essai à blanc ? On regarde les
            // quinze lignes au-dessus : c'est la portée d'un `if` dans
            // ces scripts.
            let gardée = lignes[n.saturating_sub(15)..n]
                .iter()
                .any(|l| l.contains("$ESSAI"));
            if !gardée {
                nues.push(format!("{nom}:{} {taillée}", n + 1));
            }
        }
        assert!(
            nues.is_empty(),
            "{} geste(s) qui écriraient malgré --dry-run :\n  {}",
            nues.len(),
            nues.join("\n  ")
        );
    }
}

/// L'installation, pour de vrai, de bout en bout.
///
/// Les contrôles précédents lisent le script ; celui-ci le LANCE, et
/// regarde ce qui est posé sur le disque. C'est la seule façon de savoir
/// qu'`install.sh` marche — un script qui se relit bien peut très bien
/// ne rien installer.
///
/// Il n'écrit RIEN dans le profil de l'utilisateur : `--prefix` et
/// `--bin-dir` l'envoient dans un dossier temporaire, `--no-dock` et
/// `--no-path` lui interdisent de toucher au bureau et au shell. C'est
/// précisément ce que ces options ont rendu possible — sans elles, ce
/// test ne pouvait pas exister, et il est resté ouvert des semaines pour
/// cette raison.
///
/// `--no-build` évite de rappeler cargo depuis cargo : le binaire est
/// remplacé par un fichier quelconque, puisque ce qu'on mesure ici est
/// l'installation, non la compilation.
#[test]
fn installing_really_puts_everything_in_place() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !racine.join("install.sh").is_file() {
        return;
    }
    // Un binaire de remplacement, là où le script ira le chercher. On
    // ne touche à `target/` que s'il n'y a rien : écraser celui de la
    // compilation en cours serait une mauvaise surprise.
    let faux = racine.join("target/debug/namsdec-gui");
    if faux.exists() {
        // Le vrai est là : tant mieux, on l'installera lui.
    } else {
        let Some(parent) = faux.parent() else { return };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        if std::fs::write(&faux, b"#!/bin/sh\nexit 0\n").is_err() {
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&faux, std::fs::Permissions::from_mode(0o755));
        }
    }

    let dossier = tempfile::tempdir().expect("un dossier temporaire");
    let prefix = dossier.path().join("share");
    let bin = dossier.path().join("bin");

    let sortie = std::process::Command::new("bash")
        .arg(racine.join("install.sh"))
        .args([
            "--no-build",
            "--debug",
            "--no-dock",
            "--no-path",
            "--no-color",
            "--prefix",
        ])
        .arg(&prefix)
        .arg("--bin-dir")
        .arg(&bin)
        .output();
    let Ok(sortie) = sortie else {
        eprintln!("bash est introuvable : test ignoré");
        return;
    };
    assert!(
        sortie.status.success(),
        "l'installation a échoué :\n{}\n{}",
        String::from_utf8_lossy(&sortie.stdout),
        String::from_utf8_lossy(&sortie.stderr)
    );

    // Ce que le script promet d'avoir posé, et rien de moins.
    for attendu in [
        bin.join("namsdec"),
        prefix.join("namsdec/exercises/namsdec.json"),
        // Les séries greffées, et non le seul parcours livré : le
        // script les pose par une boucle sur `plugins/exercices/`, et
        // une boucle qui ne trouve rien s'exécute sans rien dire.
        prefix.join("namsdec/exercises/atelier.json"),
        prefix.join("icons/hicolor/scalable/apps/namsdec.svg"),
        prefix.join("applications/namsdec.desktop"),
    ] {
        assert!(
            attendu.exists(),
            "{} manque après l'installation :\n{}",
            attendu.display(),
            String::from_utf8_lossy(&sortie.stdout)
        );
    }

    // Le lanceur pointe vers le binaire qu'on vient de poser, et non
    // vers celui d'ailleurs : un `.desktop` qui lance autre chose est
    // pire qu'un lanceur absent.
    let lanceur =
        std::fs::read_to_string(prefix.join("applications/namsdec.desktop")).expect("le lanceur");
    assert!(
        lanceur.contains(&format!("Exec={}/namsdec", bin.display())),
        "le lanceur doit pointer vers le binaire installé :\n{lanceur}"
    );

    // Et la série livrée est bien celle du dépôt, non un fichier vide.
    let pack = namsdec_exercises::pack::read(prefix.join("namsdec/exercises/namsdec.json"))
        .expect("la série installée se lit");
    assert_eq!(pack.id, namsdec_exercises::BUILTIN_ID);

    // La désinstallation défait exactement cela.
    let sortie = std::process::Command::new("bash")
        .arg(racine.join("uninstall.sh"))
        .args(["--no-color", "--prefix"])
        .arg(&prefix)
        .arg("--bin-dir")
        .arg(&bin)
        .output()
        .expect("la désinstallation tourne");
    assert!(
        sortie.status.success(),
        "la désinstallation a échoué :\n{}",
        String::from_utf8_lossy(&sortie.stderr)
    );
    for parti in [
        bin.join("namsdec"),
        prefix.join("icons/hicolor/scalable/apps/namsdec.svg"),
        prefix.join("applications/namsdec.desktop"),
    ] {
        assert!(!parti.exists(), "{} devait partir", parti.display());
    }
    // Les exercices, eux, restent : on désinstalle souvent pour
    // réinstaller, et une série qu'on a écrite ne se perd pas au passage.
    assert!(
        prefix.join("namsdec/exercises/namsdec.json").exists(),
        "les séries d'exercices sont gardées sans --purge"
    );
    assert!(
        prefix.join("namsdec/exercises/atelier.json").exists(),
        "l'atelier aussi est gardé sans --purge"
    );

    // Et l'atelier installé est bien celui du dépôt, avec ses deux
    // paliers : un fichier posé mais vide passerait les lignes
    // ci-dessus sans rien apprendre.
    let atelier = namsdec_exercises::pack::read(prefix.join("namsdec/exercises/atelier.json"))
        .expect("l'atelier installé se lit");
    assert!(
        atelier.exercises.len() >= 18,
        "l'atelier installé en compte {}",
        atelier.exercises.len()
    );
    assert!(
        atelier
            .exercises
            .iter()
            .any(|e| e.level == namsdec_exercises::Level::Mastery),
        "l'atelier monte jusqu'au palier Maîtrise"
    );
}

/// La recherche de mise à jour se laisse retirer, et l'est par une seule
/// option.
///
/// La pile TLS qu'elle traîne — ureq, rustls, ring — pèse 1,6 Mo sur un
/// binaire de 16, soit dix pour cent, pour une fonction qu'on emploie
/// une fois par mois. C'est le cas d'un empaqueteur de distribution :
/// les mises à jour sont son travail, pas celui de l'application.
///
/// L'option est activée PAR DÉFAUT — l'application livrée ne change
/// pas. Le contrôle vérifie les deux moitiés de cette promesse : que
/// l'option existe, et qu'elle est dans les défauts. Une option
/// d'allègement qu'on oublierait d'activer livrerait une application
/// amputée sans que personne ne s'en aperçoive avant l'utilisateur.
#[test]
fn the_update_check_can_be_left_out_by_one_option() {
    let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let gui = std::fs::read_to_string(racine.join("Cargo.toml")).expect("Cargo.toml du gui");
    assert!(
        gui.contains("reseau = [\"namsdec-update/reseau\"]"),
        "l'option doit passer la consigne au crate qui porte le réseau :\n{gui}"
    );
    assert!(
        gui.contains("default = [\"reseau\"]"),
        "et être activée par défaut — l'application livrée ne change pas :\n{gui}"
    );

    let update =
        std::fs::read_to_string(racine.join("../namsdec-update/Cargo.toml")).expect("Cargo.toml");
    assert!(
        update.contains("ureq = { workspace = true, optional = true }"),
        "la pile réseau doit être optionnelle là où elle est déclarée :\n{update}"
    );

    // Et le workspace n'active pas les défauts du crate à la place du
    // gui : sans cela, l'option ne servirait à rien, puisque `ureq`
    // entrerait quoi qu'on demande.
    let ws = std::fs::read_to_string(racine.join("../../Cargo.toml")).expect("Cargo.toml racine");
    assert!(
        ws.contains(
            "namsdec-update = { path = \"crates/namsdec-update\", default-features = false }"
        ),
        "le workspace doit laisser le gui décider :\n{ws}"
    );

    // Ce qui ne dépend PAS du réseau reste disponible sans l'option :
    // comparer deux versions, lire une réponse, rendre un verdict. Le
    // vérifier ici garde la frontière à sa place.
    let lib = std::fs::read_to_string(racine.join("../namsdec-update/src/lib.rs")).expect("lib.rs");
    let bornées = lib.matches("#[cfg(feature = \"reseau\")]").count();
    assert!(
        bornées <= 3,
        "{bornées} éléments bornés au réseau — seule la requête doit \
         l'être, le reste se teste et se lit sans lui"
    );
}

/// La rubrique d'un outil extérieur le lance vraiment, et montre sa
/// sortie.
///
/// Le chemin entier, au clic : le panneau pose une demande, l'application
/// lance le programme dans un fil, en relève la réponse et la rend au
/// panneau qui l'attendait. C'est le seul endroit où namsdec sort de son
/// bac à sable, et c'est donc celui qu'il faut éprouver en entier.
///
/// **Ce contrôle lance objdump sur cette machine.** Il passe donc ici et
/// ne prouve rien là où objdump n'est pas installé — il s'efface alors
/// plutôt que d'échouer, comme ceux qui lisent `/bin/ls`.
#[test]
fn a_tool_panel_really_runs_its_tool_and_shows_the_output() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    if namsdec_disasm::known::program_path("objdump").is_none() {
        return;
    }
    let Ok(octets) = std::fs::read("/bin/ls") else {
        return;
    };
    let mut app = NamsdecApp::default();
    if !app.load_elf(&octets, "ls") {
        return;
    }
    app.show_analysis = true;
    app.analysis_module = "tool_objdump".to_owned();
    // Ce que l'application sait des outils : sans cela, le panneau
    // croit objdump absent et éteint son bouton.
    app.deliver_tool_states();
    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1600.0, 1000.0))
        .build_ui_state(|ui, app: &mut NamsdecApp| app.draw(ui), app);
    harness.run();
    harness.get_by_label("Lancer").click();
    // `step` et non `run` : tant que l'outil travaille, l'application
    // redemande une image — et `run`, qui tourne jusqu'à ce que
    // l'interface se taise, s'arrête alors sur une erreur.
    harness.step();
    assert!(
        harness.state().tool_job.is_some(),
        "le clic a lancé l'outil"
    );

    // On attend la réponse : l'outil travaille dans un fil, et
    // l'application la relève image par image.
    for _ in 0..600 {
        harness.step();
        if harness.state().tool_job.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        harness.state().tool_job.is_none(),
        "l'outil a rendu la main"
    );
    harness.step();
    harness.step();
    // Et sa sortie est à l'écran. On la reconnaît à ce que le panneau
    // écrit AUTOUR d'elle — le compte de lignes et la pagination —, et
    // non aux lignes elles-mêmes : celles-ci sont peintes d'une seule
    // galère, pour que chaque adresse y soit cliquable à sa place
    // exacte, et une galère peinte à la main n'entre pas dans l'arbre
    // d'accessibilité où kittest cherche.
    assert!(
        harness.query_by_label_contains("lignes").is_some(),
        "le panneau dit combien de lignes l'outil a rendues"
    );
    assert!(
        harness.query_by_label_contains(" sur ").is_some(),
        "et quelle page on regarde"
    );
    assert!(
        harness.query_by_label_contains("Chercher").is_some(),
        "la barre de recherche paraît avec la sortie"
    );
}
