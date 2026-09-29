//! Les sauts rendus à leur forme : `si`, `sinon`, `tant que`, `répéter`.
//!
//! `pseudo::of` traduit une instruction à la fois — `jne boucle` devient
//! « si différent, aller à boucle ». C'est déjà beaucoup, et ce n'est
//! pas ce qu'on lit quand on comprend un programme : personne ne pense
//! « aller à », on pense « tant que ». Le chaînon qui manque n'est pas
//! dans l'instruction, il est dans le GRAPHE — une arête qui remonte est
//! une boucle, deux branches qui se rejoignent sont un `si`.
//!
//! Ce module lit le déroulement construit par [`crate::cfg`] et rend un
//! pseudo-code indenté. Il ne devine rien : ce qu'il ne reconnaît pas,
//! il l'écrit tel quel, avec une étiquette et un « aller à ». Un `si`
//! inventé à la mauvaise place coûterait plus cher que le `goto` qu'il
//! remplace — on relit un `goto`, on ne relit pas ce qu'on croit avoir
//! compris.
//!
//! TROIS FORMES SONT RECONNUES, et rien d'autre :
//!
//! - **`si` / `sinon`** — un bloc à deux sorties dont les deux branches
//!   se rejoignent plus bas ;
//! - **`tant que`** — une tête de boucle qui décide, en haut, si l'on
//!   entre encore ;
//! - **`répéter … tant que`** — le test est en bas, donc le corps passe
//!   au moins une fois. La distinction n'est pas cosmétique : elle est
//!   la différence entre une boucle qui peut ne jamais s'exécuter et une
//!   qui s'exécute toujours au moins une fois, et c'est exactement ce
//!   qu'on cherche à savoir devant un `crackme`.
//!
//! La condition affichée est celle du PROGRAMME, non celle du saut. Un
//! compilateur écrit « si NON condition, sauter par-dessus » ; rendre
//! la structure demande de retourner la condition, faute de quoi chaque
//! `si` dirait le contraire de ce que fait le code.

use std::collections::{BTreeMap, BTreeSet};

use namsdec_disasm::{DecodedInstruction, Mnemonic};

use crate::cfg::{Cfg, Exit, Kind};

/// Une ligne de pseudo-code, avec ce qu'il faut pour la poser à l'écran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ligne {
    /// Le niveau d'indentation, en crans.
    pub profondeur: usize,
    pub texte: String,
    /// L'instruction d'où vient la ligne, quand il y en a une : de quoi
    /// remonter au listing, et de quoi surligner la ligne courante.
    pub adresse: Option<u64>,
    /// Cette ligne est-elle une étiquette de saut ?
    ///
    /// Dit ici et non deviné aux deux-points : « si égal : » en porte un
    /// aussi, et le premier jet effaçait donc tous les mots-clés de
    /// structure en croyant nettoyer les étiquettes inutiles.
    pub est_étiquette: bool,
}

impl Ligne {
    fn mot(profondeur: usize, texte: impl Into<String>) -> Self {
        Self {
            profondeur,
            texte: texte.into(),
            adresse: None,
            est_étiquette: false,
        }
    }

    fn étiquette(profondeur: usize, nom: &str) -> Self {
        Self {
            profondeur,
            texte: format!("{nom}:"),
            adresse: None,
            est_étiquette: true,
        }
    }
}

/// Le pseudo-code structuré d'une fonction.
///
/// `cfg` est le graphe de la fonction — celui que donne `subgraph` sur
/// `function_around` —, et l'entrée est son premier bloc. `dis` est le
/// désassemblage complet : on y retrouve chaque instruction par son
/// adresse pour la redire en clair.
pub fn pseudo_code(cfg: &Cfg, dis: &[DecodedInstruction], lang: &str) -> Vec<Ligne> {
    if cfg.blocks.is_empty() {
        return Vec::new();
    }
    let mut ctx = Contexte::new(cfg, dis, lang);
    let mut lignes = ctx.région(0, None, 0, 0);
    // Les étiquettes ne sont posées que si quelque chose y saute : une
    // étiquette sans renvoi est du bruit dans un texte qu'on veut lire.
    lignes.retain(|l| {
        !l.est_étiquette
            || ctx
                .étiquettes_utilisées
                .contains(l.texte.trim_end_matches(':'))
    });
    lignes
}

/// Le pseudo-code rendu en texte, une ligne par ligne, indenté.
pub fn en_texte(lignes: &[Ligne]) -> String {
    lignes
        .iter()
        .map(|l| format!("{}{}", "    ".repeat(l.profondeur), l.texte))
        .collect::<Vec<_>>()
        .join("\n")
}

struct Contexte<'a> {
    cfg: &'a Cfg,
    /// Le désassemblage rangé par adresse : chaque bloc ne garde que le
    /// texte de ses instructions, et redire en clair demande les octets.
    par_adresse: BTreeMap<u64, &'a DecodedInstruction>,
    lang: &'a str,
    /// Les blocs déjà écrits. Un bloc n'est écrit qu'une fois : le
    /// rencontrer à nouveau donne un « aller à ».
    écrits: BTreeSet<usize>,
    étiquettes_utilisées: BTreeSet<String>,
    /// Les blocs par où l'on entre dans une boucle.
    têtes: BTreeSet<usize>,
    /// Les boucles dont on est en train d'écrire le corps.
    ///
    /// Une boucle dont le test est en bas fait réécrire sa tête — c'est
    /// par là que le corps commence. Sans cette liste, la tête rouvrirait
    /// sa propre boucle, et le pseudo-code descendrait en `répéter`
    /// emboîtés jusqu'à la profondeur maximale.
    ouvertes: BTreeSet<usize>,
    /// Le bloc qui ferme la boucle qu'on est en train d'écrire.
    ///
    /// Son saut de retour EST le `tant que` déjà posé : le réécrire en
    /// « si …, aller à » dirait deux fois la même chose, et la seconde
    /// donnerait à croire qu'il y a là un second test.
    ferme_la_boucle: Option<usize>,
}

/// La profondeur de récursion au-delà de laquelle on renonce.
///
/// Un graphe tordu — sauts croisés, branches qui rentrent au milieu
/// d'une autre — peut faire descendre indéfiniment. Renoncer produit du
/// « aller à », ce qui est laid mais vrai ; boucler produirait une pile
/// pleine, ce qui n'est rien.
const PROFONDEUR_MAX: usize = 24;

/// Par où l'on entre dans chaque boucle d'un graphe.
///
/// La cible d'une arête ARRIÈRE, c'est-à-dire d'une arête qui remonte
/// vers un bloc encore sur la pile du parcours en profondeur. Il y en a
/// une par boucle, et c'est celle par où l'on entre.
///
/// Exposé parce que le portrait d'une fonction compte ses boucles : les
/// recompter ailleurs, avec une autre définition, les ferait diverger —
/// et la première définition essayée ici était fausse.
pub fn têtes_de_boucle(cfg: &Cfg) -> BTreeSet<usize> {
    Contexte::têtes_de_boucle(cfg)
}

impl<'a> Contexte<'a> {
    fn new(cfg: &'a Cfg, dis: &'a [DecodedInstruction], lang: &'a str) -> Self {
        Self {
            cfg,
            par_adresse: dis.iter().map(|i| (i.address, i)).collect(),
            lang,
            écrits: BTreeSet::new(),
            étiquettes_utilisées: BTreeSet::new(),
            têtes: Self::têtes_de_boucle(cfg),
            ouvertes: BTreeSet::new(),
            ferme_la_boucle: None,
        }
    }

    /// Les successeurs d'un bloc par le lien demandé — les appels ne
    /// comptent pas : ils partent et reviennent, ils ne branchent pas.
    fn suivant(&self, bloc: usize, kind: Kind) -> Option<usize> {
        self.cfg
            .edges
            .iter()
            .find(|e| e.from == bloc && e.kind == kind)
            .map(|e| e.to)
    }

    /// La seule sortie d'un bloc qui n'a rien à décider.
    fn unique_suivant(&self, bloc: usize) -> Option<usize> {
        self.cfg
            .edges
            .iter()
            .find(|e| e.from == bloc && e.kind != Kind::Call)
            .map(|e| e.to)
    }

    /// Ce qu'on atteint depuis `départ` sans jamais repasser par
    /// `interdit`.
    fn atteignables(&self, départ: usize, interdit: usize) -> BTreeSet<usize> {
        let mut vus = BTreeSet::new();
        let mut à_voir = vec![départ];
        while let Some(index) = à_voir.pop() {
            if index == interdit || !vus.insert(index) {
                continue;
            }
            for edge in self
                .cfg
                .edges
                .iter()
                .filter(|e| e.from == index && e.kind != Kind::Call)
            {
                à_voir.push(edge.to);
            }
        }
        vus
    }

    /// Le bloc où deux branches se rejoignent, s'il existe.
    ///
    /// Le premier dans l'ordre des adresses qu'on atteint des deux
    /// côtés : c'est celui qui suit le `si` dans le texte, et prendre le
    /// plus bas ferait avaler tout le reste de la fonction dans une
    /// branche.
    fn convergence(&self, test: usize, vrai: usize, faux: usize) -> Option<usize> {
        let a = self.atteignables(vrai, test);
        let b = self.atteignables(faux, test);
        let mut communs: Vec<usize> = a.intersection(&b).copied().collect();
        communs.sort_by_key(|i| self.cfg.blocks[*i].start);
        communs.into_iter().next()
    }

    /// Ce bloc est-il la tête d'une boucle ?
    ///
    /// La question paraît simple et le premier jet s'y est trompé : « un
    /// bloc qu'on atteint et qui revient vers lui » désigne TOUS les
    /// blocs d'une boucle, puisqu'ils reviennent tous. Le structureur
    /// ouvrait alors une boucle à chaque bloc du corps, et descendait
    /// jusqu'à la profondeur maximale.
    ///
    /// La bonne définition passe par le parcours en profondeur : la tête
    /// est la cible d'une arête ARRIÈRE, c'est-à-dire d'une arête qui
    /// remonte vers un bloc encore sur la pile. Il n'y en a qu'une par
    /// boucle, et c'est celle par où l'on entre.
    fn tête_de_boucle(&self, bloc: usize) -> bool {
        self.têtes.contains(&bloc)
    }

    /// Les cibles des arêtes arrière, relevées une fois pour toutes.
    fn têtes_de_boucle(cfg: &Cfg) -> BTreeSet<usize> {
        let mut têtes = BTreeSet::new();
        let mut état = vec![0u8; cfg.blocks.len()]; // 0 neuf, 1 sur la pile, 2 fini
        // Une pile explicite : un graphe de quinze mille blocs déborde
        // la pile d'appels, et c'est un binaire ordinaire.
        let mut pile = vec![(0usize, 0usize)];
        if cfg.blocks.is_empty() {
            return têtes;
        }
        état[0] = 1;
        while let Some((bloc, rang)) = pile.pop() {
            let voisins: Vec<usize> = cfg
                .edges
                .iter()
                .filter(|e| e.from == bloc && e.kind != Kind::Call)
                .map(|e| e.to)
                .collect();
            if rang >= voisins.len() {
                état[bloc] = 2;
                continue;
            }
            pile.push((bloc, rang + 1));
            let voisin = voisins[rang];
            match état.get(voisin) {
                Some(0) => {
                    état[voisin] = 1;
                    pile.push((voisin, 0));
                }
                // Encore sur la pile : l'arête remonte, donc elle boucle.
                Some(1) => {
                    têtes.insert(voisin);
                }
                _ => {}
            }
        }
        têtes
    }

    /// L'étiquette d'un bloc, telle qu'on l'écrit dans le texte.
    fn étiquette(&self, bloc: usize) -> String {
        format!("L{:x}", self.cfg.blocks[bloc].start)
    }

    /// Les instructions d'un bloc, dites en clair.
    ///
    /// `sans_la_fin` retire la dernière : quand le saut devient une
    /// structure, le répéter en « aller à » dirait deux fois la même
    /// chose, et la seconde serait fausse.
    fn corps(&self, bloc: usize, profondeur: usize, sans_la_fin: usize) -> Vec<Ligne> {
        let b = &self.cfg.blocks[bloc];
        let mut adresse = b.start;
        let combien = b.instructions.len().saturating_sub(sans_la_fin);
        let mut lignes = Vec::with_capacity(combien);
        for (rang, texte) in b.instructions.iter().enumerate() {
            let décodée = self.par_adresse.get(&adresse).copied();
            if let Some(d) = décodée {
                adresse += d.bytes.len() as u64;
            }
            if rang >= combien {
                break;
            }
            let clair = décodée
                .and_then(|d| namsdec_disasm::decode_one(&d.bytes, d.address))
                .map(|i| namsdec_disasm::pseudo::of(&i, self.lang))
                .unwrap_or_else(|| texte.clone());
            lignes.push(Ligne {
                profondeur,
                texte: clair,
                adresse: décodée.map(|d| d.address),
                est_étiquette: false,
            });
        }
        lignes
    }

    /// La condition du saut qui termine ce bloc, dans les deux sens.
    ///
    /// Rend `(telle quelle, retournée)`. Sans les octets — un bloc venu
    /// d'ailleurs, une instruction qu'on n'a pas su redécoder — on rend
    /// le texte du saut : faux serait pire que brut.
    fn condition(&self, bloc: usize) -> (String, String) {
        let b = &self.cfg.blocks[bloc];
        let brut = b
            .instructions
            .last()
            .cloned()
            .unwrap_or_else(|| "?".to_owned());
        let Some(instr) = self
            .par_adresse
            .get(&b.last)
            .and_then(|d| namsdec_disasm::decode_one(&d.bytes, d.address))
        else {
            return (brut.clone(), brut);
        };
        let telle = namsdec_disasm::pseudo::condition_of(&instr, self.lang);
        let autre = namsdec_disasm::pseudo::condition_opposée(&instr, self.lang);
        match (telle, autre) {
            (Some(a), Some(b)) => (a.to_owned(), b.to_owned()),
            _ => (brut.clone(), brut),
        }
    }

    /// Le bloc ne fait-il que préparer sa condition ?
    ///
    /// C'est le cas ordinaire d'un `tant que` : `cmp rcx, 0` puis `je`.
    /// Un bloc qui fait autre chose avant de tester ne se résume pas à
    /// une condition — sa boucle s'écrit alors avec une sortie au milieu,
    /// ce qui est la vérité.
    fn ne_fait_que_tester(&self, bloc: usize) -> bool {
        let b = &self.cfg.blocks[bloc];
        if b.instructions.len() > 2 {
            return false;
        }
        let mut adresse = b.start;
        for _ in 0..b.instructions.len().saturating_sub(1) {
            let Some(d) = self.par_adresse.get(&adresse) else {
                return false;
            };
            let Some(instr) = namsdec_disasm::decode_one(&d.bytes, d.address) else {
                return false;
            };
            if !matches!(instr.mnemonic(), Mnemonic::Cmp | Mnemonic::Test) {
                return false;
            }
            adresse += d.bytes.len() as u64;
        }
        true
    }

    /// Écrit une région : à partir de `entrée`, jusqu'à `stop` exclu.
    fn région(
        &mut self,
        entrée: usize,
        stop: Option<usize>,
        profondeur: usize,
        récursion: usize,
    ) -> Vec<Ligne> {
        let mut lignes = Vec::new();
        let mut courant = entrée;
        loop {
            if Some(courant) == stop || courant >= self.cfg.blocks.len() {
                return lignes;
            }
            if !self.écrits.insert(courant) {
                // Déjà écrit ailleurs : on renvoie vers lui plutôt que
                // de le recopier — un texte qui répète un bloc laisse
                // croire qu'il s'exécute deux fois.
                let nom = self.étiquette(courant);
                self.étiquettes_utilisées.insert(nom.clone());
                lignes.push(Ligne::mot(profondeur, format!("aller à {nom}")));
                return lignes;
            }
            let nom = self.étiquette(courant);
            lignes.push(Ligne::étiquette(profondeur, &nom));

            if récursion >= PROFONDEUR_MAX {
                lignes.extend(self.corps(courant, profondeur, 0));
                return lignes;
            }

            if self.ferme_la_boucle == Some(courant) {
                lignes.extend(self.corps(courant, profondeur, 1));
                return lignes;
            }

            if self.tête_de_boucle(courant)
                && !self.ouvertes.contains(&courant)
                && let Some(suite) = self.boucle(courant, &mut lignes, profondeur, récursion)
            {
                match suite {
                    Some(bloc) => {
                        courant = bloc;
                        continue;
                    }
                    None => return lignes,
                }
            }

            let b = &self.cfg.blocks[courant];
            match b.exit {
                Exit::Branch => {
                    let (Some(vrai), Some(faux)) = (
                        self.suivant(courant, Kind::Taken),
                        self.suivant(courant, Kind::NotTaken),
                    ) else {
                        lignes.extend(self.corps(courant, profondeur, 0));
                        return lignes;
                    };
                    lignes.extend(self.corps(courant, profondeur, 1));
                    let (telle, retournée) = self.condition(courant);
                    let joint = self.convergence(courant, vrai, faux);
                    match joint {
                        // Le saut passe par-dessus une branche : c'est le
                        // `si` sans `sinon` qu'écrivent tous les
                        // compilateurs, et la condition s'y retourne.
                        Some(j) if j == vrai => {
                            lignes.push(Ligne::mot(profondeur, format!("si {retournée} :")));
                            lignes.extend(self.région(
                                faux,
                                Some(j),
                                profondeur + 1,
                                récursion + 1,
                            ));
                            courant = j;
                        }
                        Some(j) if j == faux => {
                            lignes.push(Ligne::mot(profondeur, format!("si {telle} :")));
                            lignes.extend(self.région(
                                vrai,
                                Some(j),
                                profondeur + 1,
                                récursion + 1,
                            ));
                            courant = j;
                        }
                        Some(j) => {
                            lignes.push(Ligne::mot(profondeur, format!("si {retournée} :")));
                            lignes.extend(self.région(
                                faux,
                                Some(j),
                                profondeur + 1,
                                récursion + 1,
                            ));
                            lignes.push(Ligne::mot(profondeur, "sinon :"));
                            lignes.extend(self.région(
                                vrai,
                                Some(j),
                                profondeur + 1,
                                récursion + 1,
                            ));
                            courant = j;
                        }
                        // Les deux branches ne se rejoignent pas : un
                        // retour dans chacune, par exemple. On le dit
                        // tel quel plutôt que d'en faire un `si` dont on
                        // ne saurait pas fermer l'accolade.
                        None => {
                            let nom = self.étiquette(vrai);
                            self.étiquettes_utilisées.insert(nom.clone());
                            lignes
                                .push(Ligne::mot(profondeur, format!("si {telle}, aller à {nom}")));
                            courant = faux;
                        }
                    }
                }
                Exit::Jump => {
                    lignes.extend(self.corps(courant, profondeur, 1));
                    // Par la seule arête qui sort, et non par son nom :
                    // un `jmp` inconditionnel n'a rien à décider, et le
                    // graphe l'étiquette « suite » plutôt que « pris ».
                    let Some(cible) = self.unique_suivant(courant) else {
                        return lignes;
                    };
                    courant = cible;
                }
                Exit::Fallthrough | Exit::Call => {
                    lignes.extend(self.corps(courant, profondeur, 0));
                    let Some(cible) = self.suivant(courant, Kind::Next) else {
                        return lignes;
                    };
                    courant = cible;
                }
                Exit::Return | Exit::Stop | Exit::Indirect => {
                    lignes.extend(self.corps(courant, profondeur, 0));
                    return lignes;
                }
            }
        }
    }

    /// Écrit la boucle dont `tête` est l'entrée.
    ///
    /// Rend `None` si la forme n'est pas reconnue — l'appelant traite
    /// alors le bloc comme un bloc ordinaire —, `Some(Some(suite))` avec
    /// le bloc qui vient après la boucle, ou `Some(None)` quand la
    /// boucle ne se quitte pas d'elle-même.
    fn boucle(
        &mut self,
        tête: usize,
        lignes: &mut Vec<Ligne>,
        profondeur: usize,
        récursion: usize,
    ) -> Option<Option<usize>> {
        let dedans = self.corps_de_boucle(tête);
        let exit = self.cfg.blocks[tête].exit;

        // Le test est en HAUT : la tête décide, avant chaque tour, si
        // l'on entre encore. Sa sortie quitte donc la boucle.
        if exit == Exit::Branch {
            let vrai = self.suivant(tête, Kind::Taken)?;
            let faux = self.suivant(tête, Kind::NotTaken)?;
            // Un bloc qui revient sur lui-même n'a pas de « dedans » :
            // il EST la boucle, et son test est en bas puisqu'il vient
            // après ses propres instructions.
            // Les deux côtés restent dedans, ou aucun : la tête ne
            // décide pas de la sortie, et la boucle se ferme plus bas.
            // On ne renonce pas pour autant — c'est le cas d'un `si`
            // posé en tête d'un `répéter`.
            let en_haut = match (dedans.contains(&vrai), dedans.contains(&faux)) {
                _ if vrai == tête || faux == tête => None,
                (true, false) => Some((vrai, faux, self.condition(tête).0)),
                (false, true) => Some((faux, vrai, self.condition(tête).1)),
                _ => None,
            };
            if let Some((entrer, sortir, condition)) = en_haut {
                if self.ne_fait_que_tester(tête) {
                    let mot = self.mot_tant_que();
                    lignes.push(Ligne::mot(profondeur, format!("{mot} {condition} :")));
                    let corps = self.région(entrer, Some(tête), profondeur + 1, récursion + 1);
                    lignes.extend(corps);
                } else {
                    // La tête fait autre chose que tester : on ne peut
                    // pas la résumer à une condition d'entrée, alors on
                    // écrit la boucle telle qu'elle tourne, sortie au
                    // milieu. C'est plus long et c'est vrai.
                    lignes.push(Ligne::mot(profondeur, self.mot_boucler()));
                    lignes.extend(self.corps(tête, profondeur + 1, 1));
                    let (_, retournée) = self.condition(tête);
                    lignes.push(Ligne::mot(profondeur + 1, self.mot_sortir(&retournée)));
                    let corps = self.région(entrer, Some(tête), profondeur + 1, récursion + 1);
                    lignes.extend(corps);
                }
                return Some(Some(sortir));
            }
        }

        // Le test est en BAS : le corps passe au moins une fois. La tête
        // compte parmi les fins possibles — `dec ecx` suivi d'un `jnz`
        // qui revient sur le `dec` est une boucle entière dans un seul
        // bloc, et c'est la plus courante qui soit.
        let fin = self
            .cfg
            .edges
            .iter()
            .filter(|e| {
                e.to == tête && e.kind != Kind::Call && (e.from == tête || dedans.contains(&e.from))
            })
            .map(|e| e.from)
            .next_back()?;

        match self.cfg.blocks[fin].exit {
            Exit::Branch => {
                let reste = self.suivant(fin, Kind::Taken) == Some(tête);
                let (telle, retournée) = self.condition(fin);
                let condition = if reste { telle } else { retournée };
                let sortie = if reste {
                    self.suivant(fin, Kind::NotTaken)
                } else {
                    self.suivant(fin, Kind::Taken)
                };
                lignes.push(Ligne::mot(profondeur, self.mot_répéter()));
                lignes.extend(self.corps_de_la_boucle(tête, fin, profondeur + 1, récursion));
                lignes.push(Ligne::mot(
                    profondeur,
                    format!("{} {condition}", self.mot_tant_que()),
                ));
                Some(sortie)
            }
            Exit::Jump => {
                lignes.push(Ligne::mot(profondeur, self.mot_sans_fin()));
                lignes.extend(self.corps_de_la_boucle(tête, fin, profondeur + 1, récursion));
                Some(None)
            }
            _ => None,
        }
    }

    /// Le corps d'une boucle dont le test est en bas, tête comprise.
    ///
    /// La tête a déjà été notée comme écrite par l'appelant — sans quoi
    /// il ne saurait pas qu'il est dans une boucle. On la libère le
    /// temps d'écrire le corps, et l'on tait le saut de retour du bloc
    /// qui ferme : il est déjà dit par le `tant que`.
    fn corps_de_la_boucle(
        &mut self,
        tête: usize,
        fin: usize,
        profondeur: usize,
        récursion: usize,
    ) -> Vec<Ligne> {
        self.écrits.remove(&tête);
        self.ouvertes.insert(tête);
        let gardé = self.ferme_la_boucle.replace(fin);
        let lignes = self.région(tête, None, profondeur, récursion + 1);
        self.ferme_la_boucle = gardé;
        self.ouvertes.remove(&tête);
        lignes
    }

    fn mot_répéter(&self) -> &'static str {
        if self.lang.starts_with("fr") {
            "répéter :"
        } else {
            "repeat:"
        }
    }

    fn mot_tant_que(&self) -> &'static str {
        if self.lang.starts_with("fr") {
            "tant que"
        } else {
            "while"
        }
    }

    fn mot_boucler(&self) -> &'static str {
        if self.lang.starts_with("fr") {
            "boucler :"
        } else {
            "loop:"
        }
    }

    fn mot_sans_fin(&self) -> &'static str {
        if self.lang.starts_with("fr") {
            "boucler sans fin :"
        } else {
            "loop forever:"
        }
    }

    fn mot_sortir(&self, condition: &str) -> String {
        if self.lang.starts_with("fr") {
            format!("si {condition} : sortir de la boucle")
        } else {
            format!("if {condition}: leave the loop")
        }
    }

    /// Les blocs qui appartiennent à la boucle de cette tête : ceux
    /// qu'on atteint depuis elle et qui savent y revenir.
    fn corps_de_boucle(&self, tête: usize) -> BTreeSet<usize> {
        let depuis = self.atteignables(tête, usize::MAX);
        depuis
            .iter()
            .copied()
            .filter(|bloc| *bloc != tête && self.atteignables(*bloc, usize::MAX).contains(&tête))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le pseudo-code d'une suite d'octets, en une ligne.
    fn dire(bytes: &[u8], lang: &str) -> String {
        let dis = namsdec_disasm::disassemble(bytes, 0x400000);
        let cfg = crate::cfg::build(&dis);
        en_texte(&pseudo_code(&cfg, &dis, lang))
    }

    /// Une boucle dont le test est en bas se dit « répéter … tant que ».
    ///
    /// `dec ecx` puis `jnz` qui revient sur le `dec` : la boucle tient
    /// dans un seul bloc, et c'est la forme la plus courante qui soit.
    /// Le corps passe au moins une fois — c'est toute la différence avec
    /// un `tant que`, et c'est ce qu'on veut savoir devant un binaire.
    #[test]
    fn a_loop_that_tests_at_the_bottom_repeats_first() {
        let bytes: &[u8] = &[0xB9, 0x03, 0x00, 0x00, 0x00, 0xFF, 0xC9, 0x75, 0xFC, 0xF4];
        let texte = dire(bytes, "fr");
        assert!(texte.contains("répéter :"), "{texte}");
        assert!(texte.contains("tant que différent"), "{texte}");
        // Et le saut de retour n'est pas redit : il EST le « tant que ».
        assert!(!texte.contains("aller à"), "{texte}");
        // Le corps est bien à l'intérieur.
        let lignes: Vec<&str> = texte.lines().collect();
        let corps = lignes
            .iter()
            .position(|l| l.contains("ECX = ECX"))
            .expect("le corps de la boucle est écrit");
        assert!(lignes[corps].starts_with("    "), "{texte}");
    }

    /// Une boucle dont le test est en haut se dit « tant que ».
    ///
    /// Le corps peut ne jamais tourner, et le pseudo-code doit le dire :
    /// confondre les deux formes, c'est affirmer qu'un code s'exécute
    /// quand il peut ne pas s'exécuter.
    #[test]
    fn a_loop_that_tests_at_the_top_may_never_run() {
        // jmp vers le test / dec eax / test eax,eax / jnz vers le dec / hlt
        let bytes: &[u8] = &[0xEB, 0x02, 0xFF, 0xC8, 0x85, 0xC0, 0x75, 0xFA, 0xF4];
        let texte = dire(bytes, "fr");
        assert!(texte.contains("tant que différent :"), "{texte}");
        assert!(!texte.contains("répéter"), "{texte}");
        assert!(!texte.contains("aller à"), "{texte}");
    }

    /// La condition écrite est celle du PROGRAMME, non celle du saut.
    ///
    /// Un compilateur écrit « si NON condition, sauter par-dessus ».
    /// `cmp eax, 5` puis `jne` qui enjambe un `inc` veut dire « si eax
    /// vaut 5, incrémente » — et le pseudo-code qui recopierait le
    /// mnémonique dirait exactement le contraire. C'est l'erreur qui
    /// coûte le plus cher : elle se lit bien, et elle est fausse.
    #[test]
    fn the_condition_is_the_programs_not_the_jumps() {
        let bytes: &[u8] = &[0x83, 0xF8, 0x05, 0x75, 0x02, 0xFF, 0xC0, 0xF4];
        let texte = dire(bytes, "fr");
        assert!(texte.contains("si égal :"), "{texte}");
        assert!(!texte.contains("différent"), "{texte}");
        let lignes: Vec<&str> = texte.lines().collect();
        let sous = lignes
            .iter()
            .position(|l| l.contains("EAX = EAX + 1"))
            .expect("la branche est écrite");
        assert!(lignes[sous].starts_with("    "), "{texte}");
    }

    /// Deux branches qui se rejoignent donnent un « si … sinon ».
    #[test]
    fn two_branches_that_meet_again_give_an_else() {
        // cmp eax,5 / jne .autre / inc eax / jmp .fin / .autre: dec eax / .fin: hlt
        let bytes: &[u8] = &[
            0x83, 0xF8, 0x05, 0x75, 0x04, 0xFF, 0xC0, 0xEB, 0x02, 0xFF, 0xC8, 0xF4,
        ];
        let texte = dire(bytes, "fr");
        assert!(texte.contains("si égal :"), "{texte}");
        assert!(texte.contains("sinon :"), "{texte}");
        // Les deux branches sont en retrait, la suite ne l'est pas.
        let lignes: Vec<&str> = texte.lines().collect();
        for attendu in ["EAX = EAX + 1", "EAX = EAX − 1"] {
            let rang = lignes
                .iter()
                .position(|l| l.contains(attendu))
                .unwrap_or_else(|| panic!("{attendu} manque :\n{texte}"));
            assert!(lignes[rang].starts_with("    "), "{attendu} :\n{texte}");
        }
        assert!(
            lignes.iter().any(|l| l == &"arrêter la machine"),
            "la suite revient au ras de la marge :\n{texte}"
        );
    }

    /// Un `si` posé dans une boucle reste dans la boucle.
    ///
    /// C'est le cas où le premier jet se perdait : la tête de boucle
    /// teste quelque chose qui ne SORT PAS de la boucle, et il ne faut
    /// ni y voir un « tant que » ni renoncer — la boucle se ferme plus
    /// bas, et le test du haut est un `si` ordinaire.
    #[test]
    fn a_test_inside_a_loop_stays_inside() {
        let bytes: &[u8] = &[
            0xB9, 0x05, 0x00, 0x00, 0x00, // mov ecx, 5
            0x83, 0xF9, 0x02, // cmp ecx, 2
            0x74, 0x02, // je  -> dec
            0xFF, 0xC0, // inc eax
            0xFF, 0xC9, // dec ecx
            0x75, 0xF5, // jnz -> cmp
            0xF4, // hlt
        ];
        let texte = dire(bytes, "fr");
        let lignes: Vec<&str> = texte.lines().collect();
        assert!(texte.contains("répéter :"), "{texte}");
        assert!(texte.contains("si différent :"), "{texte}");
        // Le `si` est DANS la boucle, son corps encore plus profond.
        let si = lignes
            .iter()
            .position(|l| l.contains("si différent"))
            .expect("le si");
        let inc = lignes
            .iter()
            .position(|l| l.contains("EAX = EAX + 1"))
            .expect("le corps du si");
        assert!(lignes[si].starts_with("    "), "{texte}");
        assert!(lignes[inc].starts_with("        "), "{texte}");
    }

    /// Une boucle ne s'emboîte pas dans elle-même.
    ///
    /// Le premier jet appelait « tête de boucle » tout bloc d'un cycle —
    /// ils y reviennent tous — et ouvrait donc une boucle par bloc du
    /// corps, jusqu'à la profondeur maximale. Soixante-seize lignes pour
    /// un programme de neuf octets.
    #[test]
    fn a_loop_does_not_nest_inside_itself() {
        for bytes in [
            &[0xEB, 0x02, 0xFF, 0xC8, 0x85, 0xC0, 0x75, 0xFA, 0xF4][..],
            &[0xB9, 0x03, 0x00, 0x00, 0x00, 0xFF, 0xC9, 0x75, 0xFC, 0xF4][..],
        ] {
            let dis = namsdec_disasm::disassemble(bytes, 0x400000);
            let cfg = crate::cfg::build(&dis);
            let lignes = pseudo_code(&cfg, &dis, "fr");
            assert!(
                lignes.len() <= cfg.blocks.len() + 6,
                "{} lignes pour {} blocs :\n{}",
                lignes.len(),
                cfg.blocks.len(),
                en_texte(&lignes)
            );
            assert!(
                lignes.iter().all(|l| l.profondeur < 4),
                "l'indentation s'emballe :\n{}",
                en_texte(&lignes)
            );
        }
    }

    /// Ce qu'on ne sait pas structurer est écrit tel quel.
    ///
    /// Un `jmp` sur lui-même ne se quitte pas : on le dit, plutôt que
    /// d'inventer une condition de sortie qui n'existe pas.
    #[test]
    fn what_cannot_be_structured_is_said_plainly() {
        let texte = dire(&[0xEB, 0xFE], "fr");
        assert!(texte.contains("boucler sans fin"), "{texte}");
    }

    /// Une étiquette n'est écrite que si quelque chose y saute.
    ///
    /// Un texte semé d'étiquettes qu'on ne cite jamais se lit moins bien
    /// que le listing dont il devait dispenser.
    #[test]
    fn a_label_appears_only_when_something_jumps_to_it() {
        // Trois instructions et pas un saut : aucune étiquette.
        let texte = dire(&[0xFF, 0xC0, 0xFF, 0xC8, 0xF4], "fr");
        assert!(!texte.contains("L4000"), "{texte}");
        assert_eq!(texte.lines().count(), 3, "{texte}");
    }

    /// Les mots de structure suivent la langue.
    #[test]
    fn the_keywords_follow_the_language() {
        let bytes: &[u8] = &[0xB9, 0x03, 0x00, 0x00, 0x00, 0xFF, 0xC9, 0x75, 0xFC, 0xF4];
        let anglais = dire(bytes, "en");
        assert!(anglais.contains("repeat:"), "{anglais}");
        assert!(anglais.contains("while "), "{anglais}");
        assert!(!anglais.contains("répéter"), "{anglais}");
    }

    /// Chaque ligne venue d'une instruction sait d'où elle vient.
    ///
    /// C'est ce qui permet de surligner, dans le pseudo-code, la ligne
    /// où le processeur se trouve — sans quoi le pseudo-code serait un
    /// texte mort à côté d'un listing vivant.
    #[test]
    fn every_line_from_an_instruction_knows_its_address() {
        let dis = namsdec_disasm::disassemble(&[0xFF, 0xC0, 0xFF, 0xC8, 0xF4], 0x400000);
        let cfg = crate::cfg::build(&dis);
        let lignes = pseudo_code(&cfg, &dis, "fr");
        let adresses: Vec<u64> = lignes.iter().filter_map(|l| l.adresse).collect();
        assert_eq!(adresses, vec![0x400000, 0x400002, 0x400004], "{lignes:?}");
    }

    /// Un graphe vide ne fait rien tomber.
    #[test]
    fn an_empty_graph_says_nothing() {
        assert!(pseudo_code(&Cfg::default(), &[], "fr").is_empty());
    }
}
