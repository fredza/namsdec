//! Lancer un programme extérieur, et reprendre la main quoi qu'il
//! arrive.
//!
//! Trois choses qu'un `Command::output()` ne donne pas, et dont chacune
//! a coûté un défaut à ce projet :
//!
//! * **Un délai tenu.** `output()` attend aussi longtemps que l'outil
//!   veut. Une seule ligne d'assembleur — `times 2000000000 nop` — fait
//!   réfléchir `nasm` plus de vingt secondes, pendant lesquelles
//!   l'interface ne répond à rien.
//! * **Une sortie bornée ET vidée.** Un tube a une soixantaine de
//!   kilo-octets de tampon ; plein, il BLOQUE celui qui écrit. Lire
//!   après coup, c'est donc attendre un outil qui attend qu'on le lise.
//!   Les deux tubes sont lus pendant que l'outil écrit, on garde au
//!   plus la borne demandée, et l'on continue de vider le reste.
//! * **Un arrêt qui emporte la descendance.** Tuer l'enfant direct
//!   laisse un petit-fils tenir les tubes ouverts : le délai était
//!   alors tenu pour le processus et pas pour l'attente. L'enfant est
//!   mis dans son propre groupe, et c'est le groupe qu'on arrête.
//!
//! Un seul lanceur pour tout le monde — les désassembleurs extérieurs
//! de `plugin`, `nasm` et `ld` dans `namsdec-asm` —, parce que ces
//! garanties-là ne se redécouvrent pas une fois par appelant. Ce
//! module est né de la mise en commun décidée par P2 du plan du
//! 28 octobre 2026 : le lanceur des outils avait raison, l'assembleur
//! n'en avait aucun.

use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Au-delà, ce n'est plus un listing qu'on lit, c'est un vidage.
pub const MAX_SORTIE: usize = 4 * 1024 * 1024;

/// Le délai de grâce accordé aux fils de lecture après l'arrêt de
/// l'outil : le temps que les tubes se ferment, et pas davantage.
const GRÂCE: Duration = Duration::from_millis(500);

/// Ce qu'on accorde à un programme extérieur.
///
/// Le délai est par TÂCHE et non par programme : une compilation
/// d'outil a le droit de durer un quart d'heure, un assemblage de
/// quelques milliers de lignes se compte en dizaines de millisecondes.
/// Leur imposer le même chiffre reviendrait à ne surveiller ni l'un ni
/// l'autre.
#[derive(Debug, Clone)]
pub struct Bornes {
    pub délai: Duration,
    /// Combien d'octets on garde de chaque sortie.
    pub max_sortie: usize,
    /// De quoi arrêter avant la fin, quand quelqu'un le demande.
    ///
    /// `None` pour un appel qu'on ne peut pas annuler — un lancement
    /// qui doit de toute façon rendre dans la seconde.
    pub arrêt: Option<Arc<AtomicBool>>,
}

impl Bornes {
    /// Un délai, et le reste par défaut.
    pub fn en(délai: Duration) -> Self {
        Self {
            délai,
            max_sortie: MAX_SORTIE,
            arrêt: None,
        }
    }

    /// La même chose, arrêtable.
    pub fn arrêtable(mut self, arrêt: Arc<AtomicBool>) -> Self {
        self.arrêt = Some(arrêt);
        self
    }

    fn arrêt_demandé(&self) -> bool {
        self.arrêt
            .as_ref()
            .is_some_and(|drapeau| drapeau.load(Ordering::Relaxed))
    }
}

/// Comment un programme extérieur s'est terminé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fin {
    /// Il a fini de lui-même, avec ce code — `None` s'il a été arrêté
    /// par un signal.
    Terminé(Option<i32>),
    /// Le délai a été atteint, et il a été tué.
    TropLong,
    /// Quelqu'un a demandé l'arrêt, et il a été tué.
    Arrêté,
}

impl Fin {
    /// A-t-il fini de lui-même, sans erreur ?
    pub fn réussi(self) -> bool {
        self == Fin::Terminé(Some(0))
    }
}

/// Ce qu'un programme extérieur a rendu.
#[derive(Debug, Clone, Default)]
pub struct Sortie {
    pub ordinaire: Récolte,
    pub erreur: Récolte,
    pub fin: Option<Fin>,
}

impl Sortie {
    pub fn fin(&self) -> Fin {
        self.fin.unwrap_or(Fin::Terminé(None))
    }
}

/// Lance la commande et la surveille jusqu'au bout — ou jusqu'à la
/// borne.
///
/// La commande arrive déjà garnie de ses arguments et de son répertoire
/// de travail : ce lanceur ne décide de rien de ce que l'outil fait,
/// seulement de ce qu'on lui accorde. Les flux sont posés ici, parce
/// qu'ils font partie des garanties — une entrée standard laissée
/// ouverte est un outil qui attend une réponse que personne ne donnera.
pub fn lancer(commande: &mut Command, bornes: &Bornes) -> std::io::Result<Sortie> {
    let mut enfant = démarrer(commande)?;
    let sortie_std = enfant
        .stdout
        .take()
        .map(|flux| Lecteur::lancer(flux, bornes.max_sortie));
    let sortie_err = enfant
        .stderr
        .take()
        .map(|flux| Lecteur::lancer(flux, bornes.max_sortie));
    let fin = surveiller(&mut enfant, bornes);
    // Les fils rejoints DANS TOUS LES CAS, y compris après un arrêt —
    // mais avec une échéance : un descendant qui aurait échappé au
    // groupe (un `setsid`, par exemple) tiendrait sinon le tube ouvert
    // pour toujours. Passé la grâce, on prend ce qui a été lu jusque-là
    // et l'on s'en va ; le tampon est partagé, donc rien n'est perdu de
    // ce qui était arrivé.
    Ok(Sortie {
        ordinaire: sortie_std
            .map(|lecteur| lecteur.récolter(GRÂCE))
            .unwrap_or_default(),
        erreur: sortie_err
            .map(|lecteur| lecteur.récolter(GRÂCE))
            .unwrap_or_default(),
        fin: Some(fin),
    })
}

/// Le même lancement, mais la sortie va **ligne à ligne dans un
/// journal** pendant que l'outil écrit, au lieu d'être rendue à la fin.
///
/// C'est ce dont une installation a besoin : un téléchargement de
/// 180 Mo ou une compilation d'un quart d'heure doivent s'écrire au fur
/// et à mesure, sinon la fenêtre reste muette pendant tout ce temps et
/// passe pour cassée. Les deux sorties vont dans le même journal, dans
/// l'ordre où elles arrivent — c'est ce qu'on lit dans un terminal, et
/// séparer les deux rendrait les messages d'erreur illisibles, détachés
/// de l'étape qui les a provoqués.
pub fn lancer_en_journal(
    commande: &mut Command,
    bornes: &Bornes,
    journal: &Journal,
) -> std::io::Result<Fin> {
    let mut enfant = démarrer(commande)?;
    let mut lecteurs = Vec::new();
    if let Some(flux) = enfant.stdout.take() {
        lecteurs.push(LecteurDeLignes::lancer(flux, journal.clone()));
    }
    if let Some(flux) = enfant.stderr.take() {
        lecteurs.push(LecteurDeLignes::lancer(flux, journal.clone()));
    }
    let fin = surveiller(&mut enfant, bornes);
    // La même grâce, et pour la même raison : c'est elle qui fait que
    // « arrêter » rend la main. Sans elle, un `join` sans échéance
    // attendait que le dernier descendant veuille bien fermer le tube —
    // mesuré à trente secondes pour un arrêt demandé au bout de trois
    // cents millisecondes.
    for lecteur in lecteurs {
        lecteur.attendre(GRÂCE);
    }
    Ok(fin)
}

/// Prépare et lance : flux fermés ou canalisés, et son propre groupe.
fn démarrer(commande: &mut Command) -> std::io::Result<std::process::Child> {
    commande
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        commande.process_group(0);
    }
    commande.spawn()
}

/// Attend l'outil, et reprend la main au délai ou à la demande.
///
/// On surveille à petits pas plutôt que d'attendre : c'est la seule
/// façon, sans bibliothèque supplémentaire, de reprendre la main sur un
/// outil qui ne la rend pas.
fn surveiller(enfant: &mut std::process::Child, bornes: &Bornes) -> Fin {
    let début = std::time::Instant::now();
    loop {
        match enfant.try_wait() {
            // Le statut est GARDÉ. Jeté, il laissait un outil qui écrit
            // « tool failed » et sort avec le code 7 rendre un succès
            // indiscernable d'un vrai.
            Ok(Some(fini)) => return Fin::Terminé(fini.code()),
            Err(_) => return Fin::Terminé(None),
            Ok(None) if bornes.arrêt_demandé() => {
                tuer_le_groupe(enfant);
                let _ = enfant.wait();
                return Fin::Arrêté;
            }
            Ok(None) if début.elapsed() >= bornes.délai => {
                tuer_le_groupe(enfant);
                let _ = enfant.wait();
                return Fin::TropLong;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

/// Arrête l'outil **et sa descendance**.
///
/// Sous Unix, l'enfant a été placé dans son propre groupe : un signal
/// envoyé au groupe (un PID négatif) atteint tout ce qu'il a lancé.
/// Ailleurs, on fait ce qu'on peut — l'enfant direct.
///
/// Ce qui échappe quand même : un descendant qui s'est donné un
/// nouveau groupe par `setsid`. Rien ne le rattrape sans un cgroup, et
/// le dire vaut mieux que de promettre de tous les arrêter.
pub fn tuer_le_groupe(enfant: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let pid = enfant.id() as i32;
        // SAFETY: `kill(2)` sur un groupe dont nous sommes le parent.
        // Le processus n'a pas encore été attendu, donc son PID est
        // toujours le sien et ne peut pas avoir été réattribué.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    let _ = enfant.kill();
}

/// Ce qu'un fil de lecture a récolté sur un tube ou dans un fichier.
#[derive(Debug, Default, Clone)]
pub struct Récolte {
    /// Les octets gardés : au plus la borne demandée.
    pub octets: Vec<u8>,
    /// Vrai si l'outil en a écrit davantage.
    pub tronqué: bool,
}

impl Récolte {
    /// Le contenu d'un fichier, borné de la même façon qu'un tube.
    ///
    /// Un listing de nasm peut faire quarante méga-octets — `%rep
    /// 400000` suffit —, et le lire en entier pour en garder une partie
    /// revient à ne pas avoir de borne.
    pub fn lire_fichier(path: &std::path::Path, max: usize) -> std::io::Result<Self> {
        use std::io::Read;
        let mut octets = Vec::new();
        std::fs::File::open(path)?
            .take(max as u64 + 1)
            .read_to_end(&mut octets)?;
        let tronqué = octets.len() > max;
        octets.truncate(max);
        Ok(Self { octets, tronqué })
    }

    /// Le texte, coupé sur une frontière UTF-8 valide.
    ///
    /// C'est ici que tenait la panique : `String::truncate(4 Mio)` sur
    /// un texte dont le quatre-millionième octet tombait au milieu d'un
    /// « é » interrompait le fil, et le résultat du travail était
    /// perdu. On coupe les OCTETS, et seulement là où une séquence se
    /// termine.
    pub fn texte(&self) -> String {
        let mut texte = String::from_utf8_lossy(frontière(&self.octets)).into_owned();
        if self.tronqué {
            texte.push_str("\n… (sortie coupée à 4 Mio)");
        }
        texte
    }

    /// Le texte sans la mention de coupure : pour qui n'affiche pas
    /// cette sortie mais l'analyse — les diagnostics de nasm, par
    /// exemple, où une ligne ajoutée deviendrait un faux avertissement.
    pub fn texte_brut(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(frontière(&self.octets))
    }
}

/// Les octets jusqu'à la dernière séquence UTF-8 complète.
///
/// Une séquence tronquée en fin de tampon est écartée ; une vraie
/// erreur au milieu est laissée à `from_utf8_lossy`, qui la remplace
/// sans rien interrompre.
fn frontière(octets: &[u8]) -> &[u8] {
    match std::str::from_utf8(octets) {
        Ok(_) => octets,
        Err(e) if e.error_len().is_none() => &octets[..e.valid_up_to()],
        Err(_) => octets,
    }
}

/// Ce qu'on garde d'un journal d'exécution, et de chacune de ses
/// lignes.
///
/// Deux bornes, et la seconde manquait. Le nombre de lignes était tenu
/// — un journal de compilation en produit des dizaines de milliers, on
/// garde la fin, qui est ce qu'on lit quand ça casse. Mais **la taille
/// d'UNE ligne** ne l'était pas : une sortie sans aucun retour à la
/// ligne — `yes | tr -d '\n'`, ou n'importe quel outil qui écrit une
/// barre de progression sans `\r` — faisait grossir le tampon de la
/// ligne en cours sans que rien ne l'arrête. Mesuré : cinquante
/// méga-octets en trois secondes, et le garde-fou des deux mille lignes
/// ne voyait rien passer, puisqu'il n'y avait jamais de ligne.
#[derive(Debug, Clone, Copy)]
pub struct BornesJournal {
    /// Au-delà, on retire les plus anciennes.
    pub max_lignes: usize,
    /// Combien on en retire d'un coup : rogner une par une ferait
    /// recopier tout le journal à chaque ligne reçue.
    pub a_retirer: usize,
    /// La taille d'une ligne, au-delà de laquelle on coupe et on le
    /// dit.
    pub max_ligne: usize,
}

impl Default for BornesJournal {
    fn default() -> Self {
        Self {
            max_lignes: 2000,
            a_retirer: 500,
            max_ligne: 64 * 1024,
        }
    }
}

/// Un journal que le lanceur remplit pendant que l'outil écrit.
///
/// Partagé et bon marché à cloner : l'interface en garde un exemplaire
/// pour le peindre à chaque image, les fils de lecture le remplissent.
#[derive(Debug, Clone)]
pub struct Journal {
    lignes: Arc<std::sync::Mutex<Vec<String>>>,
    bornes: BornesJournal,
}

impl Default for Journal {
    fn default() -> Self {
        Self::neuf(BornesJournal::default())
    }
}

impl Journal {
    pub fn neuf(bornes: BornesJournal) -> Self {
        Self {
            lignes: Arc::new(std::sync::Mutex::new(Vec::new())),
            bornes,
        }
    }

    /// Ajoute une ligne, et tient les bornes.
    pub fn écrire(&self, ligne: impl Into<String>) {
        self.poser(ligne.into(), false);
    }

    /// Ajoute une ligne, ou REMPLACE la dernière.
    ///
    /// Le remplacement est ce que demande un retour chariot : `curl` et
    /// `git` réécrivent leur ligne de progression des dizaines de
    /// milliers de fois, et les empiler ferait un journal illisible de
    /// pourcentages.
    fn poser(&self, ligne: String, remplacer: bool) {
        let Ok(mut lignes) = self.lignes.lock() else {
            return;
        };
        match lignes.last_mut() {
            Some(dernière) if remplacer => *dernière = ligne,
            _ => lignes.push(ligne),
        }
        if lignes.len() > self.bornes.max_lignes {
            lignes.drain(..self.bornes.a_retirer);
        }
    }

    /// Ce que le journal contient, copié pour être peint.
    pub fn copie(&self) -> Vec<String> {
        self.lignes
            .lock()
            .map(|lignes| lignes.clone())
            .unwrap_or_default()
    }

    /// Combien de lignes, sans tout recopier.
    pub fn combien(&self) -> usize {
        self.lignes.lock().map(|lignes| lignes.len()).unwrap_or(0)
    }

    pub fn est_vide(&self) -> bool {
        self.combien() == 0
    }
}

/// Un fil qui découpe un tube en lignes et les pose dans un journal.
struct LecteurDeLignes {
    fini: std::sync::mpsc::Receiver<()>,
}

impl LecteurDeLignes {
    fn lancer(mut flux: impl std::io::Read + Send + 'static, journal: Journal) -> Self {
        let (dit_fini, fini) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut ligne: Vec<u8> = Vec::new();
            let mut bloc = [0u8; 4096];
            let mut remplacer = false;
            // Vrai dès qu'une ligne a dépassé sa borne : ce qui suit
            // est jeté jusqu'au prochain retour à la ligne, mais on
            // CONTINUE DE LIRE — s'arrêter remplirait le tube et
            // bloquerait l'outil sur son écriture suivante.
            let mut déborde = false;
            loop {
                let lu = match flux.read(&mut bloc) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                for &octet in &bloc[..lu] {
                    if octet != b'\n' && octet != b'\r' {
                        if ligne.len() < journal.bornes.max_ligne {
                            ligne.push(octet);
                        } else if !déborde {
                            déborde = true;
                            ligne.extend_from_slice(
                                format!(" … (ligne coupée à {} octets)", journal.bornes.max_ligne)
                                    .as_bytes(),
                            );
                        }
                        continue;
                    }
                    let texte = String::from_utf8_lossy(frontière(&ligne))
                        .trim_end()
                        .to_owned();
                    ligne.clear();
                    déborde = false;
                    if !texte.is_empty() {
                        journal.poser(texte, remplacer);
                    }
                    remplacer = octet == b'\r';
                }
            }
            // Ce qui restait sans retour à la ligne final : un outil qui
            // écrit son dernier mot sans `\n` n'a pas à le perdre.
            if !ligne.is_empty() {
                let texte = String::from_utf8_lossy(frontière(&ligne))
                    .trim_end()
                    .to_owned();
                if !texte.is_empty() {
                    journal.poser(texte, remplacer);
                }
            }
            let _ = dit_fini.send(());
        });
        Self { fini }
    }

    /// Laisse au fil au plus `grâce` pour finir, puis s'en va.
    fn attendre(self, grâce: Duration) {
        let _ = self.fini.recv_timeout(grâce);
    }
}

/// Un fil qui vide un tube dans un tampon partagé.
///
/// Partagé, et c'est la raison d'être du type : le fil peut être
/// abandonné sans que ce qu'il avait déjà lu disparaisse avec lui.
struct Lecteur {
    tampon: Arc<std::sync::Mutex<Récolte>>,
    fini: std::sync::mpsc::Receiver<()>,
}

impl Lecteur {
    fn lancer(mut flux: impl std::io::Read + Send + 'static, max: usize) -> Self {
        let tampon = Arc::new(std::sync::Mutex::new(Récolte::default()));
        let (dit_fini, fini) = std::sync::mpsc::channel();
        let à_remplir = Arc::clone(&tampon);
        std::thread::spawn(move || {
            let mut bloc = [0u8; 64 * 1024];
            loop {
                let lu = match flux.read(&mut bloc) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                let Ok(mut récolte) = à_remplir.lock() else {
                    break;
                };
                let place = max.saturating_sub(récolte.octets.len());
                let prend = lu.min(place);
                récolte.octets.extend_from_slice(&bloc[..prend]);
                if prend < lu {
                    // Au-delà de la borne on CONTINUE DE LIRE sans
                    // garder : s'arrêter remplirait le tube, et l'outil
                    // resterait bloqué sur son écriture suivante.
                    récolte.tronqué = true;
                }
            }
            let _ = dit_fini.send(());
        });
        Self { tampon, fini }
    }

    /// Ce que le fil a lu, en lui laissant au plus `grâce` pour finir.
    fn récolter(self, grâce: Duration) -> Récolte {
        let _ = self.fini.recv_timeout(grâce);
        self.tampon
            .lock()
            .map(|récolte| récolte.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn sh_existe() -> bool {
        crate::known::program_path("sh").is_some()
    }

    /// Un outil qui ne finit pas est arrêté au délai, et le délai est
    /// TENU — pas « à peu près tenu ».
    ///
    /// La marge mesurée ici est large à dessein : ce qu'on vérifie
    /// n'est pas la précision de l'horloge, c'est qu'on ne reste pas
    /// accroché aux cinq secondes du dormeur.
    #[test]
    fn un_outil_qui_ne_finit_pas_est_arrete_au_delai() {
        if !sh_existe() {
            return;
        }
        let début = std::time::Instant::now();
        let mut commande = Command::new("sh");
        commande.args(["-c", "sleep 5"]);
        let rendu = lancer(&mut commande, &Bornes::en(Duration::from_millis(200)))
            .expect("l'outil se lance");
        assert_eq!(rendu.fin(), Fin::TropLong);
        assert!(
            début.elapsed() < Duration::from_secs(3),
            "la main est rendue sans attendre le dormeur : {:?}",
            début.elapsed()
        );
    }

    /// Et un arrêt demandé rend la main AVANT le délai.
    ///
    /// C'est la garantie que P2 ajoute au lanceur : le bouton Stop d'un
    /// assemblage ne peut pas attendre les dix secondes accordées à
    /// nasm pour avoir un effet.
    #[test]
    fn un_arret_demande_rend_la_main_avant_le_delai() {
        if !sh_existe() {
            return;
        }
        let arrêt = Arc::new(AtomicBool::new(false));
        let drapeau = Arc::clone(&arrêt);
        // Levé après un court instant : le temps que le lanceur ait
        // commencé à surveiller.
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            drapeau.store(true, Ordering::Relaxed);
        });
        let début = std::time::Instant::now();
        let mut commande = Command::new("sh");
        commande.args(["-c", "sleep 30"]);
        let bornes = Bornes::en(Duration::from_secs(30)).arrêtable(arrêt);
        let rendu = lancer(&mut commande, &bornes).expect("l'outil se lance");
        assert_eq!(rendu.fin(), Fin::Arrêté);
        assert!(
            début.elapsed() < Duration::from_secs(5),
            "arrêté à la demande, et non au bout des trente secondes : {:?}",
            début.elapsed()
        );
    }

    /// Un descendant qui garde les tubes ouverts n'allonge pas
    /// l'attente : l'enfant est dans son propre groupe, et c'est le
    /// groupe qu'on arrête.
    #[test]
    fn un_petit_fils_ne_retient_pas_la_main() {
        if !sh_existe() {
            return;
        }
        let début = std::time::Instant::now();
        let mut commande = Command::new("sh");
        // Le `sleep` hérite des tubes et survit à son parent : c'est
        // exactement ce que font les scripts d'installation.
        commande.args(["-c", "sleep 20 & sleep 20"]);
        let rendu = lancer(&mut commande, &Bornes::en(Duration::from_millis(200)))
            .expect("l'outil se lance");
        assert_eq!(rendu.fin(), Fin::TropLong);
        assert!(
            début.elapsed() < Duration::from_secs(3),
            "le petit-fils est parti avec le groupe : {:?}",
            début.elapsed()
        );
    }

    /// Le code de sortie est gardé : un outil qui échoue en le disant
    /// ne doit pas être confondu avec un outil qui réussit.
    #[test]
    fn le_code_de_sortie_est_garde() {
        if !sh_existe() {
            return;
        }
        let mut commande = Command::new("sh");
        commande.args(["-c", "echo refus >&2; exit 7"]);
        let rendu =
            lancer(&mut commande, &Bornes::en(Duration::from_secs(5))).expect("l'outil se lance");
        assert_eq!(rendu.fin(), Fin::Terminé(Some(7)));
        assert!(!rendu.fin().réussi());
        assert_eq!(rendu.erreur.texte_brut().trim(), "refus");
    }

    /// Une sortie plus grande que la borne est coupée, et l'outil n'est
    /// pas bloqué pour autant : on continue de vider le tube.
    #[test]
    fn une_grosse_sortie_est_bornee_sans_bloquer_l_outil() {
        if !sh_existe() {
            return;
        }
        let mut commande = Command::new("sh");
        // Deux méga-octets, là où l'on n'en garde que mille : bien
        // au-delà du tampon d'un tube, donc l'outil resterait bloqué si
        // personne ne lisait.
        commande.args(["-c", "printf 'A%.0s' $(seq 1 2000000)"]);
        let bornes = Bornes {
            délai: Duration::from_secs(20),
            max_sortie: 1000,
            arrêt: None,
        };
        let rendu = lancer(&mut commande, &bornes).expect("l'outil se lance");
        assert_eq!(rendu.fin(), Fin::Terminé(Some(0)), "il a fini de lui-même");
        assert_eq!(
            rendu.ordinaire.octets.len(),
            1000,
            "et on n'en garde que mille"
        );
        assert!(rendu.ordinaire.tronqué, "la coupe est dite");
    }

    /// Un fichier est lu borné, comme un tube.
    #[test]
    fn un_fichier_est_lu_borne() {
        let dir = tempfile::tempdir().expect("un dossier");
        let chemin = dir.path().join("gros");
        std::fs::write(&chemin, vec![b'x'; 5000]).expect("l'écriture");
        let récolte = Récolte::lire_fichier(&chemin, 1000).expect("la lecture");
        assert_eq!(récolte.octets.len(), 1000);
        assert!(récolte.tronqué);
        let entier = Récolte::lire_fichier(&chemin, 100_000).expect("la lecture");
        assert_eq!(entier.octets.len(), 5000);
        assert!(!entier.tronqué, "rien à couper, rien n'est dit");
    }
}

#[cfg(test)]
mod journal_tests {
    use super::*;
    use std::process::Command;

    fn sh_existe() -> bool {
        crate::known::program_path("sh").is_some()
    }

    /// Un journal se remplit PENDANT que l'outil écrit, et pas après.
    ///
    /// C'est ce qui distingue ce mode de l'autre, et c'est ce dont une
    /// installation a besoin : une compilation d'un quart d'heure qui
    /// n'écrit rien avant la fin passe pour un programme bloqué.
    #[test]
    fn le_journal_se_remplit_pendant_que_l_outil_ecrit() {
        if !sh_existe() {
            return;
        }
        let journal = Journal::default();
        let vu = journal.clone();
        let (dit, su) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // On regarde le journal pendant que l'outil tourne : il
            // doit avoir quelque chose bien avant la fin.
            for _ in 0..100 {
                if !vu.est_vide() {
                    let _ = dit.send(true);
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = dit.send(false);
        });
        let mut commande = Command::new("sh");
        commande.args(["-c", "echo début; sleep 1; echo fin"]);
        let fin = lancer_en_journal(
            &mut commande,
            &Bornes::en(Duration::from_secs(10)),
            &journal,
        )
        .expect("l'outil se lance");
        assert_eq!(fin, Fin::Terminé(Some(0)));
        assert!(
            su.recv_timeout(Duration::from_secs(5)).unwrap_or(false),
            "la première ligne est arrivée avant la fin de l'outil"
        );
        assert_eq!(journal.copie(), vec!["début", "fin"]);
    }

    /// Une ligne sans fin est coupée, et la coupe est dite.
    ///
    /// LE DÉFAUT REPRODUIT, puis retenu : une sortie sans aucun retour
    /// à la ligne faisait grossir le tampon sans borne — cinquante
    /// méga-octets en trois secondes, mesuré. Le garde-fou du nombre de
    /// lignes ne voyait rien passer, puisqu'il n'y avait jamais de
    /// ligne.
    #[test]
    fn une_ligne_sans_fin_est_coupee() {
        if !sh_existe() {
            return;
        }
        let journal = Journal::neuf(BornesJournal {
            max_ligne: 500,
            ..BornesJournal::default()
        });
        let mut commande = Command::new("sh");
        // Cent mille octets sans un seul retour à la ligne.
        commande.args(["-c", "printf 'A%.0s' $(seq 1 100000)"]);
        let fin = lancer_en_journal(
            &mut commande,
            &Bornes::en(Duration::from_secs(20)),
            &journal,
        )
        .expect("l'outil se lance");
        assert_eq!(fin, Fin::Terminé(Some(0)));
        let lignes = journal.copie();
        assert_eq!(lignes.len(), 1, "une seule ligne : {lignes:?}");
        assert!(
            lignes[0].len() < 600,
            "coupée à la borne, pas gardée en entier : {} octets",
            lignes[0].len()
        );
        assert!(
            lignes[0].contains("ligne coupée"),
            "et la coupe est dite : {}",
            &lignes[0][lignes[0].len().saturating_sub(40)..]
        );
    }

    /// Le nombre de lignes est borné, et c'est la FIN qu'on garde —
    /// celle qu'on lit quand ça casse.
    #[test]
    fn le_journal_garde_la_fin_et_non_le_debut() {
        let journal = Journal::neuf(BornesJournal {
            max_lignes: 100,
            a_retirer: 30,
            max_ligne: 1000,
        });
        for i in 0..500 {
            journal.écrire(format!("ligne {i}"));
        }
        let lignes = journal.copie();
        assert!(lignes.len() <= 100, "borné : {}", lignes.len());
        assert_eq!(
            lignes.last().map(String::as_str),
            Some("ligne 499"),
            "la dernière est là"
        );
        assert!(
            !lignes.iter().any(|l| l == "ligne 0"),
            "et la première est partie"
        );
    }

    /// Un retour chariot REMPLACE la dernière ligne au lieu d'en
    /// ajouter une : c'est ce que fait une barre de progression, et
    /// empiler ses dizaines de milliers de pourcentages rendrait le
    /// journal illisible.
    #[test]
    fn un_retour_chariot_reecrit_la_ligne() {
        if !sh_existe() {
            return;
        }
        let journal = Journal::default();
        let mut commande = Command::new("sh");
        commande.args(["-c", r"printf 'a\r10%%\r50%%\r100%%\ndone\n'"]);
        lancer_en_journal(
            &mut commande,
            &Bornes::en(Duration::from_secs(10)),
            &journal,
        )
        .expect("l'outil se lance");
        let lignes = journal.copie();
        assert_eq!(
            lignes,
            vec!["100%".to_owned(), "done".to_owned()],
            "seule la dernière version de la ligne reste : {lignes:?}"
        );
    }

    /// Un arrêt demandé rend la main **même si un petit-fils tient le
    /// tube ouvert**.
    ///
    /// LE DÉFAUT PRINCIPAL DE P3, reproduit avant d'être corrigé :
    /// l'installation tuait l'enfant direct puis joignait ses lecteurs
    /// SANS ÉCHÉANCE. Mesuré sur cette forme exacte — `sleep 30 & sleep
    /// 30` — l'enfant mourait en trois cents millisecondes et la main
    /// n'était rendue que trente secondes plus tard, à la seconde près :
    /// la durée du petit-fils.
    ///
    /// Deux corrections y répondent, et il faut les deux : le groupe de
    /// processus emporte la descendance, et la grâce borne l'attente de
    /// ceux qui y auraient échappé.
    #[test]
    fn un_arret_rend_la_main_malgre_un_petit_fils() {
        if !sh_existe() {
            return;
        }
        let dossier = tempfile::tempdir().expect("un dossier");
        let témoin = dossier.path().join("le-petit-fils-a-survecu");
        let journal = Journal::default();
        let arrêt = Arc::new(AtomicBool::new(false));
        let drapeau = Arc::clone(&arrêt);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drapeau.store(true, Ordering::Relaxed);
        });
        let début = std::time::Instant::now();
        let mut commande = Command::new("sh");
        // Le petit-fils attend une seconde et demie, puis POSE UN
        // TÉMOIN. C'est ce témoin qui distingue « arrêté » de
        // « abandonné » : sans lui, le contrôle passait aussi bien sans
        // groupe de processus, la grâce suffisant à rendre la main
        // pendant que le descendant continuait tranquillement sa vie.
        //
        // Les durées sont courtes à dessein : même si ce contrôle
        // échoue, rien ne traîne sur la machine plus de huit secondes.
        commande.args([
            "-c",
            &format!("(sleep 1.5; touch {}) & sleep 8", témoin.to_string_lossy()),
        ]);
        let bornes = Bornes::en(Duration::from_secs(60)).arrêtable(arrêt);
        let fin = lancer_en_journal(&mut commande, &bornes, &journal).expect("l'outil se lance");
        assert_eq!(fin, Fin::Arrêté);
        assert!(
            début.elapsed() < Duration::from_secs(3),
            "la main est rendue sans attendre le petit-fils : {:?}",
            début.elapsed()
        );
        // Et maintenant la vraie question : est-il mort, ou seulement
        // perdu de vue ?
        std::thread::sleep(Duration::from_secs(3));
        assert!(
            !témoin.exists(),
            "le petit-fils a survécu à l'arrêt : il a posé {}",
            témoin.display()
        );
    }

    /// Et le cas où le parent SORT TOUT DE SUITE en laissant un
    /// descendant tenir le tube : sans la grâce, l'étape passait pour
    /// « en cours » pendant toute la vie du descendant, alors qu'elle
    /// était finie.
    #[test]
    fn un_parent_sorti_ne_laisse_pas_l_etape_en_cours() {
        if !sh_existe() {
            return;
        }
        let journal = Journal::default();
        let début = std::time::Instant::now();
        let mut commande = Command::new("sh");
        commande.args(["-c", "sleep 30 & echo parti; exit 0"]);
        let fin = lancer_en_journal(
            &mut commande,
            &Bornes::en(Duration::from_secs(60)),
            &journal,
        )
        .expect("l'outil se lance");
        assert_eq!(fin, Fin::Terminé(Some(0)));
        assert!(
            début.elapsed() < Duration::from_secs(5),
            "l'étape est finie quand le parent est fini : {:?}",
            début.elapsed()
        );
        assert_eq!(journal.copie(), vec!["parti"], "et sa sortie est là");
    }

    /// Une étape qui dépasse son délai est arrêtée, et le dit.
    #[test]
    fn une_etape_trop_longue_est_arretee() {
        if !sh_existe() {
            return;
        }
        let journal = Journal::default();
        let mut commande = Command::new("sh");
        commande.args(["-c", "echo je commence; sleep 30"]);
        let début = std::time::Instant::now();
        let fin = lancer_en_journal(
            &mut commande,
            &Bornes::en(Duration::from_millis(300)),
            &journal,
        )
        .expect("l'outil se lance");
        assert_eq!(fin, Fin::TropLong);
        assert!(
            début.elapsed() < Duration::from_secs(5),
            "{:?}",
            début.elapsed()
        );
        assert_eq!(
            journal.copie(),
            vec!["je commence"],
            "ce qu'il avait écrit avant d'être arrêté n'est pas perdu"
        );
    }

    /// Le statut d'une étape qui échoue est exact : le code, et non
    /// « ça n'a pas marché ».
    #[test]
    fn une_etape_en_erreur_rend_son_code() {
        if !sh_existe() {
            return;
        }
        let journal = Journal::default();
        let mut commande = Command::new("sh");
        commande.args(["-c", "echo avant; echo le refus >&2; exit 3"]);
        let fin = lancer_en_journal(
            &mut commande,
            &Bornes::en(Duration::from_secs(10)),
            &journal,
        )
        .expect("l'outil se lance");
        assert_eq!(fin, Fin::Terminé(Some(3)));
        let lignes = journal.copie();
        assert!(
            lignes.contains(&"avant".to_owned()) && lignes.contains(&"le refus".to_owned()),
            "les deux sorties vont au même journal, comme dans un terminal : {lignes:?}"
        );
    }
}
