# namsdec — ce qui est fait, ce qui reste

Cette liste est l'**index** du travail. Le *pourquoi* de chaque entrée —
ce qui l'a fait apparaître, ce qu'elle coûte, ce qui a été essayé et
retiré — reste dans `AMELIORATIONS.txt`, qui ne bouge pas de rôle : une
seule source pour le raisonnement, une seule pour l'état. Les renvois
`(A 4.14)` pointent vers l'entrée correspondante de ce fichier-là.

Tenue à jour au fil des séances. Dernière mise à jour : **16 novembre
2026**, version 0.4.0-beta.1.

- `[x]` fait et vérifié — un contrôle le retient
- `[~]` commencé, le reste est écrit sous l'entrée
- `[ ]` à faire

---

## Priorités pour Claude — revue du 28 octobre 2026

**À la demande de Fred : suivre cet ordre pour la prochaine passe de
consolidation.** Les trois premiers lots sont faits — voir le tableau
d'avancement plus bas ; les éléments déjà cochés dans le reste du
fichier restent l'historique des travaux réalisés. Avant de
modifier un comportement, vérifier son état actuel et consulter les
entrées existantes : un constat de cette revue peut avoir été corrigé
depuis. Ne pas refaire un travail déjà démontré comme terminé.

La revue de départ est conservée dans
[l'analyse du projet](mockups/2026-10-28/ANALYSE.md). Ce bloc en est le
plan d'action ; les investigations et décisions nouvelles continuent
d'aller dans `AMELIORATIONS.txt`, avec un renvoi ajouté ici. Les chemins
et symboles ci-dessous sont les points d'entrée, pas des numéros de ligne
à conserver malgré les modifications.

**Consigne de suivi pour Claude :** prendre un lot délimité, passer son
entrée à `[~]`, reproduire ou mesurer le problème, corriger, puis noter
les contrôles et leurs résultats. Ne passer à `[x]` qu'une fois les
critères de validation satisfaits. Distinguer les défauts reproduits,
les risques déduits du code et les choix de conception. Consolider ces
points avant d'ajouter de nouveaux modules sans rapport avec eux.

### Où en est ce plan — 29 octobre 2026

| Lot | État | Ce que la mesure a trouvé |
|-----|------|---------------------------|
| **P1** limites de l'émulation | `[x]` A 9.55 | Les trois constats étaient justes, et un quatrième les dépasse : le contrôle de précision se sépare en **trois**, dont deux réglages exactement simulés. |
| **P2** interface réactive | `[x]` A 9.56 | Le gel est **reproduit** : `times 2000000000 nop`, plus de 20 s. Et un second défaut, 40 Mo de listing lus en entier. |
| **P3** arrêt des outils | `[x]` A 9.57 | Reproduit et **pire que décrit** : arrêt demandé à 300 ms, main rendue à 30 001 ms. Plus 50 Mo avalés en 3 s sur une ligne sans fin. |
| **P3 bis** prototype RE web local | `[ ]` | Éprouver un espace RE fonctionnel sur de vrais ELF, y compris de gros binaires, avant de décider d'une migration de l'interface. |
| **P4** apprentissage / analyse | `[ ]` | Le seul lot qui demande un avis plutôt qu'une mesure : trois organisations à éprouver, pas une refonte validée. **Tauri a été envisagé puis écarté le 29 octobre** — voir P7. |
| **P5** sous-états de `NamsdecApp` | `[ ]` | — |
| **P6** commentaires historiques | `[ ]` | — |
| **P7** traitement / présentation | `[x]` | **Clos le 3 novembre 2026.** Les quatre étapes sont faites : **19 vues entières sur 22**, et trois panneaux partagés — l'éditeur binaire, le vidage et le détour gardent leur instrument peint et disent ce qu'ils comprennent. Un rapport complet, un export JSON et une analyse sans interface en sortent (`rapport.rs`). La frontière existe (`vue.rs`, sans egui), et ce qu'un panneau montre **se lit sans le peindre**. |

Ce que les trois premiers lots ont en commun, et qui vaut pour les
suivants : **chacun partait d'un risque déduit du code, et chacun s'est
révélé mesurable.** Les trois sont reproduits par une commande, et les
chiffres obtenus ont à chaque fois déplacé le sujet — ce n'est pas la
même chose de savoir qu'un gel est possible et de savoir qu'une ligne
d'assembleur suffit à le provoquer.

Un gain non prévu par le plan, et qui a servi aux trois : **le lanceur
de processus est désormais unique** (`namsdec-disasm::lanceur`). Il était
écrit trois fois — bien dans `plugin.rs`, mal dans `start_install`,
à moitié dans `run_upx_on`. Les deux copies ont disparu, et `nasm`,
`ld`, les sondes de démarrage et l'assemblage d'une instruction seule en
héritent. P3 n'a donc eu à ajouter qu'une chose : le journal progressif.

### P1 — Rendre explicites les limites de l'émulation

- [x] **Recenser et signaler les comportements simplifiés** (A 9.55,
      28 octobre 2026) — fait, et la mesure a déplacé le sujet. Les
      trois constats de la revue étaient justes ; le programme
      d'épreuve en a ajouté un quatrième, qui est le plus utile :
      **le contrôle de précision se sépare en trois**. À 53 bits,
      namsdec EST le matériel — c'est la double précision, celle qu'il
      calcule nativement. À 24 bits, il suffit d'arrondir chaque
      résultat au format simple. Seule la précision **étendue**, qui
      est le réglage par défaut, reste approximée : le matériel rend
      9,76e-19 là où namsdec rend zéro. Dire « le x87 est approximé »
      était une demi-vérité, et la moins utile des deux moitiés.
      Corrigé : `X87::control` est un vrai registre, `fldcw` écrit,
      `fnstcw` relit, `finit` remet 0x037f ; l'arrondi gouverne `fist`,
      `fistp` et `frndint`, `fisttp` s'en affranchit, et la précision
      s'applique aux cinq opérations de base.
      Recensé : `namsdec_emu::Simplification`, sept entrées, chacune un
      fait mesuré. L'émulateur nomme, l'interface fait la phrase dans
      les deux langues. Dites au moment où elles se produisent, une
      fois chacune : barre d'état, rubrique dans l'onglet Diagnostics
      (son compteur les ajoute à ceux de nasm), et la fiche
      **F1 « arrondi »** qui porte le recensement complet.
      **Validation :** `cargo run -p namsdec-emu --example oracle_x87`
      — vingt cas, un seul programme des deux côtés, écarts attendus
      déclarés d'avance — rend **18 identiques, 2 écarts attendus et
      annoncés, 0 défaut**. Cinq contrôles de régression portent les
      valeurs du matériel, tous éprouvés en remettant le défaut, plus
      une scène de banc « console / Diagnostics, simplifications ». Le
      banc a trouvé ce qu'aucun d'eux ne cherchait : la rubrique posée
      hors du défilement poussait le panneau sur l'éditeur à 800x520
      (A 9.37 mot pour mot).
      **Reste non mesuré**, et dit comme tel : les transcendantes ne
      sont pas comparées cas par cas au matériel — le manuel d'Intel ne
      garantit que l'ordre de grandeur de leur erreur —, et aucune
      exception flottante n'est levée. Les deux sont dans le
      recensement plutôt que passées sous silence.
      Le constat de départ de la revue, conservé : `Fldcw` était
      accepté sans effet, le mot de contrôle relu était constant et les
      valeurs 80 bits passent par `f64`.

  La consigne de départ, conservée telle qu'elle a été écrite — ce qui
  permet de relire ce qui était demandé et de voir ce qui a été fait
  d'autre :

  > **Recenser et signaler les comportements simplifiés**, en
  > commençant par x87 : `crates/namsdec-emu/src/x87.rs`, branches
  > `Fldcw`, `Fstcw` / `Fnstcw`, conversions `f80_to_f64` et
  > `read_fp_operand`. À la revue, `Fldcw` est accepté sans effet,
  > le mot de contrôle relu est constant et les valeurs 80 bits
  > passent par `f64`. C'est un constat de code ; aucun cas divergent
  > n'a été exécuté pendant cette revue.
  > **Action :** établir le contrat réellement simulé, corriger ce
  > qui est pris en charge et annoncer les approximations restantes
  > dans l'aide et au moment pertinent de l'exécution, en français
  > et en anglais, sans répéter le même avertissement à chaque pas.
  > Ne pas qualifier une instruction de « fidèle » sur la seule base
  > de son mnémonique : formes, précision, arrondis et exceptions
  > peuvent avoir des couvertures différentes.
  > **Validation :** un exemple minimal de changement d'arrondi et
  > un exemple sensible à la précision étendue ; comparaison à une
  > référence indépendante disponible, état avant/après et résultat
  > attendu documentés. Étendre l'oracle existant quand il convient,
  > garder les cas de régression dans les tests et rendre explicites
  > les vérifications non exécutables sur la machine. Un comportement
  > encore approximatif ne doit plus paraître pleinement simulé.

### P2 — Garder l'interface réactive pendant l'assemblage

- [x] **Sortir NASM et ld du fil de rendu** (A 9.56, 28 octobre 2026) —
      fait, et **le gel est reproduit** : le plan le donnait pour un
      risque déduit du code, une seule ligne valide suffit à le
      provoquer. `times 2000000000 nop` occupe nasm plus de vingt
      secondes, pendant lesquelles l'interface n'a ni image, ni
      clavier, ni Stop. Second défaut trouvé en chemin : `%rep 400000`
      produit quarante méga-octets de listing, que `read_to_string`
      lisait en entier.
      **Le lanceur n'a pas été réécrit, il a été partagé** — comme le
      plan le demandait. `plugin.rs` savait déjà mettre un outil dans
      son groupe de processus, vider ses tubes pendant qu'il écrit,
      tenir un délai et laisser une grâce aux lecteurs ; c'est extrait
      dans `namsdec-disasm::lanceur`, dont `namsdec-asm` dépend déjà.
      Aucun crate nouveau. Les 63 contrôles de `plugin.rs` sont passés
      au vert sans retouche.
      **Trois autres appels synchrones** recensés et passés au même
      lanceur : `assemble_one`, les deux sondes `--version` du
      démarrage, et `run_upx_on` — qui avait son propre guetteur, un
      délai tenu pour le processus mais pas pour ses descendants.
      **Le délai est mesuré, pas choisi** : sept lignes s'assemblent en
      4 ms, vingt-deux mille en 52 ms ; dix secondes laissent deux
      ordres de grandeur de marge. Le chiffre et sa mesure sont dans le
      manuel.
      **Le résultat est rattaché à sa révision** : le jeton est pris au
      lancement et comparé à l'arrivée ; un texte modifié ou un onglet
      changé entre-temps fait jeter le résultat plutôt que de faire
      pointer la gouttière sur des lignes qui ont bougé. Changer
      d'onglet arrête le travail en cours.
      **Stop arrête enfin quelque chose** : il était grisé tant que
      `running` était faux, c'est-à-dire pendant le seul travail dont
      on veuille vraiment sortir.
      **Validation :** treize contrôles — six sur le lanceur (délai
      tenu, arrêt à la demande, petit-fils qui ne retient pas la main,
      code de sortie gardé, sortie bornée sans blocage, fichier borné),
      sept sur l'interface (lancement qui rend la main, résultat
      périmé, arrêt, second assemblage, changement d'onglet, barre
      d'état dans les deux langues, et le panneau des diagnostics qui
      s'ouvre à la récolte). Trois sont éprouvés en remettant le
      défaut, dont un défaut que ce lot avait lui-même introduit : le
      panneau s'ouvrait avant que le résultat n'existe, et montrait
      donc les diagnostics de l'assemblage précédent. Le seuil de réactivité est un
      **rapport** et non un nombre de millisecondes, pour ne pas
      broncher sous charge (A 9.51).
      **Décision de conception, dite plutôt que cachée :**
      `start_assemble` pose le travail, `tick_assemble` le récolte, et
      `do_assemble` — marqué `#[cfg(test)]` — n'ajoute qu'une boucle
      d'attente par-dessus le même chemin, pour les contrôles qui ont
      besoin d'un état arrêté. Deux contrôles seulement ont dû changer,
      ceux qui passaient par le bouton.

  La consigne de départ, conservée :

  > **Sortir NASM et ld du fil de rendu**, sur le chemin
  > `crates/namsdec-gui/src/run.rs::assemble_once` vers
  > `crates/namsdec-asm/src/lib.rs::assemble`. À la revue, les appels
  > `Command::output()` y sont synchrones et sans échéance visible.
  > Le gel prolongé est un risque identifié, pas un incident reproduit
  > pendant la revue.
  > **Action :** lancer le travail en arrière-plan, montrer son état,
  > permettre l'annulation et imposer un délai explicite. Identifier
  > le document et sa révision au départ : un résultat tardif ne doit
  > pas remplacer un autre onglet ni être présenté comme correspondant
  > à un source modifié entre-temps. Examiner aussi `assemble_one`
  > et les autres appels externes synchrones atteignables depuis l'UI.
  > Partager les garanties de lancement avec P3 plutôt que créer un
  > nouveau lanceur indépendant.
  > **Validation :** outils factices lents, bloqués et en erreur ;
  > l'interface continue à traiter les interactions, l'annulation et
  > le délai rendent la main, les diagnostics restent attachés au bon
  > document, et un changement ou une fermeture d'onglet en cours de
  > travail ne corrompt aucun état. Un assemblage normal conserve ses
  > adresses, diagnostics et correspondances avec les lignes source.

### P3 — Unifier l'arrêt des outils et de leurs descendants

- [x] **Donner à l'installation les garanties du lanceur d'outils**
      (A 9.57, 29 octobre 2026) — fait, et **le risque est reproduit et
      chiffré** : en rejouant la logique de `start_install`, l'enfant
      meurt en **300 ms** et la main n'est rendue que **30 001 ms**
      plus tard — la durée du petit-fils, à la milliseconde près. Et
      sans que personne n'ait rien demandé : un parent qui sort tout de
      suite en laissant un descendant tenir le tube faisait croire
      l'étape en cours pendant trente secondes de plus.
      Troisième défaut, celui de la taille d'une ligne :
      `yes | tr -d '\n'` fait **50 Mo en 3 secondes** ; le garde-fou
      des deux mille lignes ne voyait rien passer, puisqu'il n'y avait
      jamais de ligne.
      **Rien de neuf n'a été écrit pour l'arrêt** : le lanceur partagé
      de P2 savait déjà tout cela. Il lui manquait de rendre la sortie
      au fur et à mesure, ce qu'une installation exige —
      `lancer_en_journal` a été ajouté à côté de `lancer`, les deux
      partageant `démarrer` et `surveiller`. `start_install` ne lance
      plus rien lui-même : quatre-vingts lignes de boucle ont disparu,
      et le troisième lanceur du projet avec.
      **Gagné au passage** : le drapeau d'arrêt atteint l'**étape en
      cours** et non plus seulement l'intervalle entre deux — il n'était
      donc jamais consulté pendant la compilation dont on voulait
      sortir. Son prix est dit : une étape coupée au milieu peut laisser
      un dossier à demi décompressé, d'où la vérification du résultat
      attendu à la fin.
      **Délais distingués**, comme demandé : trente minutes par étape
      d'installation contre deux pour une décompilation, avec la raison.
      **Validation :** onze contrôles — huit sur le lanceur (journal
      rempli pendant l'écriture, ligne sans fin coupée, fin du journal
      gardée, retour chariot, arrêt malgré un petit-fils, parent sorti,
      étape trop longue, étape en erreur) et trois sur l'installation
      (arrêt qui emporte la descendance, code de sortie exact, journal
      borné dans les deux sens).
      **Un contrôle a dû être refait**, et c'est la part utile : le
      premier jet passait SANS le groupe de processus — la grâce
      suffisait à rendre la main pendant que le descendant continuait
      sa vie. Il mesurait notre patience, pas la mort du petit-fils.
      Refait avec un **témoin** que le descendant pose s'il survit, puis
      éprouvé en retirant le groupe : le témoin apparaît, le contrôle
      tombe. Durées courtes à dessein — rien ne traîne plus de huit
      secondes, même si un contrôle échoue.
      **Limite dite plutôt que promise** : un descendant qui se détache
      par `setsid` échappe au signal, et rien ne le rattrape sans un
      cgroup. L'attente, elle, reste bornée dans tous les cas.
      **Précaution :** aucune recette d'épreuve ne va jusqu'au succès —
      une installation réussie appelle `declare_tool`, qui écrit dans
      les fichiers de l'utilisateur.

  La consigne de départ, conservée :

  > **Donner à l'installation les garanties du lanceur d'outils**.
  > Comparer `crates/namsdec-gui/src/jobs.rs::start_install` et
  > `crates/namsdec-disasm/src/plugin.rs` : le premier tue l'enfant
  > direct et joint les lecteurs sans échéance ; le second prévoit
  > un groupe de processus, des sorties bornées et un délai de grâce.
  > Le risque de descendants survivants reste à reproduire.
  > **Action :** mutualiser la gestion du cycle de vie, de
  > l'annulation, des délais et des sorties, tout en conservant le
  > journal progressif de l'installation. Distinguer le délai d'une
  > décompilation de celui d'une compilation d'outils : ne pas imposer
  > arbitrairement les mêmes deux minutes à toutes les tâches.
  > Borner aussi la mémoire du journal et la taille d'une ligne.
  > **Validation :** outil factice avec enfant et petit-enfant,
  > sortie abondante, ligne sans fin et tube maintenu ouvert après la
  > sortie du parent. Vérifier arrêt du groupe, attente bornée,
  > journal borné et statut exact sur succès, erreur, annulation et
  > expiration. Nettoyer les processus d'épreuve même en cas d'échec
  > du test ; documenter les limites des descendants qui quittent leur
  > groupe plutôt que promettre de les arrêter tous.

### P3 bis — Éprouver un espace RE web local sur de vrais ELF

- [ ] **Réaliser un premier espace RE fonctionnel : fonctions,
      listing, références croisées, pseudo-code et notes.** Partir de
      la [maquette RE](mockups/2026-10-29/re.html) et relier l'interface
      web locale aux moteurs Rust et aux outils locaux de Namsdec pour
      travailler sur un vrai ELF, au-delà du scénario fictif.
      **Validation :** vérifier la réactivité sur de gros binaires et
      le confort réel du parcours : ouvrir le fichier, retrouver une
      fonction, suivre une référence, consulter le pseudo-code et
      annoter une adresse. Mesurer les temps de chargement et de
      navigation, puis essayer l'interface dans un navigateur réel.
      **Décision :** cette étape doit établir si la facilité de
      modification des maquettes se retrouve dans une application
      fonctionnelle, avant de décider de migrer l'ensemble de Namsdec.
      Elle précède P4 ; la migration complète reste à évaluer à partir
      de ses résultats.

### P4 — Clarifier apprentissage et analyse avancée

- [ ] **Évaluer les trois organisations proposées** dans la
      [galerie des maquettes](mockups/2026-10-28/index.html) :
      **Atelier guidé** pour apprendre, **Observatoire** pour analyser,
      **Trajectoire** pour suivre le graphe et le temps.
      **Direction recommandée :** Atelier à l'accueil, Observatoire
      comme espace avancé, Trajectoire comme vue dédiée, avec la même
      session d'exécution. Mettre en avant « lire → prévoir → avancer →
      constater », notamment l'instruction suivante et ses effets.
      Ce sont des propositions à éprouver, pas une refonte validée par
      Fred ni une demande de remplacer egui par une interface web.
      **Action :** comparer les parcours sur un même programme, puis
      transposer une amélioration limitée dans l'interface existante.
      Conserver l'accès aux fonctions, raccourcis, thèmes et deux
      langues ; les outils avancés doivent rester faciles à retrouver.
      **Validation :** suivre `audit-ui-egui`, mesurer les scènes à
      vide et chargées, aux petites et grandes tailles, regarder les
      captures avec le vrai thème et essayer l'application en marche.
      Comparer débutant et utilisateur expérimenté quand des retours
      sont disponibles ; sans eux, garder le statut de proposition UX.
      Les contrôles géométriques seuls ne prouvent pas la compréhension.
      Les maquettes HTML n'ont pas été vérifiées dans un navigateur
      pendant la revue : les inspecter avant de les prendre pour modèle.

### P5 — Séparer les responsabilités de l'état de l'interface

- [ ] **Extraire progressivement des sous-états de `NamsdecApp`**,
      en commençant par une responsabilité rendue nécessaire par les
      lots précédents : document, exécution, analyse, travaux de fond ou
      présentation. Voir `crates/namsdec-gui/src/main.rs`, `session.rs`,
      `run.rs` et `jobs.rs`, ainsi que les entrées « Découpage de
      main.rs » plus bas dans ce fichier.
      **Action :** définir qui possède l'état et quelles opérations le
      modifient. Déplacer des méthodes dans un autre fichier ne suffit
      pas à réduire le couplage si elles modifient toujours toute
      l'application. Pas de réécriture générale ni d'objectif de nombre
      de lignes ou de gain de compilation non mesuré.
      **Validation :** préserver les parcours ouverture, fermeture et
      changement d'onglet, restauration de session, assemblage,
      exécution, retour arrière et achèvement d'un travail en fond.
      Réutiliser les contrôles existants et couvrir les frontières
      nouvelles ; maintenir les inventaires de sources utilisés par
      les tests qui inspectent le code.

### P6 — Garder le contrat dans le code, l'histoire dans le journal

- [ ] **Alléger les commentaires historiques au fil des zones touchées**.
      Garder près du code ses invariants, limites et raisons durables ;
      déplacer dans `AMELIORATIONS.txt` les récits de tentatives et de
      corrections qui masquent le contrat actuel, avec un renvoi utile.
      **Validation :** aucune justification nécessaire ne disparaît,
      aucun commentaire n'est rattaché au mauvais symbole et aucun
      changement de comportement ne se cache dans ce travail éditorial.
      Ne pas lancer une réécriture massive des commentaires.

### P7 — Séparer le traitement de la présentation (29 octobre 2026)

**Demandé par Fred, et c'est le vrai chantier d'architecture.** Les
modules de `namsdec-re` exposent leurs résultats par une méthode qui
*dessine* (`lib.rs`, trait `ReModule` : `fn ui(&mut self, ui: &mut
egui::Ui, ctx: &ReContext)`). Tant que c'est la seule sortie, ce que
namsdec comprend d'un binaire n'existe que peint à l'écran — ni
exportable, ni lisible par un autre programme, ni éprouvable autrement
qu'en mesurant des pixels.

**Tauri a été envisagé puis écarté** le 29 octobre : réécrire
88 747 lignes d'interface et perdre les ~780 contrôles qui mesurent ce
qui est peint coûte plus que ce que la mise en page CSS ferait gagner,
et WebKitGTK — la WebView de Tauri sous Linux — est justement la moins
sûre des trois. La séparation, elle, est à faire **dans tous les cas** :
elle se paie seule, et elle est le prérequis de toute alternative.

- [x] **Étape 0 — mesurer ce que les panneaux peignent vraiment**
      (29 octobre 2026). Avant de dessiner une frontière, savoir ce
      qu'elle doit laisser passer. Relevé sur les vingt
      `crates/namsdec-re/src/*_module.rs` :

      | ce qui est appelé | combien | ce que ça veut dire |
      |---|---|---|
      | `weak` + `label` + `monospace` + `strong` + `colored_label` | 230 | du **texte**, avec cinq emphases |
      | `egui::Grid` | 11 modules sur 20 | des **tableaux** |
      | `section(` | 18 | ~~des **sections titrées** — déjà une abstraction du projet~~ **FAUX, corrigé le 3 novembre 2026** : ce sont les dix-sept `fn section(&self) -> ReSection`, la méthode qui dit à quel GROUPE de panneaux un module appartient, plus une assertion de contrôle. Il n'existe aucune abstraction de section titrée dans le projet. `Bloc::Section`, né de cette ligne, n'a été employé par aucun module jusqu'à `binary_info`. La forme était réelle — cinq sections titrées écrites à la main — mais le chiffre ne la mesurait pas. |
      | `framed_button(` | 18 | des **actions** |
      | `clickable_address(` | 14 | des **adresses** qui mènent au listing |
      | `separator` / `barre` / `spinner` / `selectable_value` | 19 | séparateur, jauge, attente, choix |
      | `painter` | **4 modules** | du **dessin propre** |

      **Le résultat qui décide de la suite : quatre modules sur vingt
      peignent vraiment** — entropie, graphe d'appels, éditeur binaire,
      décompilation. Les seize autres font du texte, des tableaux et des
      adresses. La frontière est donc petite, et elle est réaliste.

      Et la moitié du travail est déjà faite, par construction : sur les
      28 293 lignes de `namsdec-re`, **18 000 ne connaissent pas egui**
      (`patch`, `explain`, `structure`, `elf_scan`, `cfg`, `entropy`,
      `ioc`, `imports`…) ; seules les ~10 000 de `*_module.rs`
      dessinent. `EntropyModule` garde déjà ses résultats dans des
      champs — `windows`, `ranges`, `whole`, `packers` — que `ui()` ne
      fait que peindre. Ils sont privés : c'est tout le défaut.
      `report.rs` (615 lignes, zéro egui) est déjà une sortie de
      données, mais partielle et en Markdown seulement.

- [x] **Étape 1 — le vocabulaire de la frontière** (29 octobre 2026).
      Deux fichiers neufs dans `namsdec-re` :
      `vue.rs` — `Vue`, `Bloc` (texte, tableau, jauge, trait, attente,
      section), `Ligne`, `Fragment` et `Ton`. **Il ne connaît pas egui,
      et ne doit jamais le connaître** : c'est ce qui en fait une
      frontière plutôt qu'une couche de plus. Sérialisable, et ce qui
      n'est pas dit ne pèse rien dans le JSON — sans quoi une vue de
      trois cents lignes porterait autant de `"copiable": false` que de
      contenu.
      `vue_peinte.rs` — le seul fichier qui connaisse les deux côtés.
      Il déménagera dans `namsdec-gui` quand les vingt modules seront
      passés, et `namsdec-re` perdra egui.
      Le trait porte `fn vue(&self, ctx) -> Option<Vue>`, `None` par
      défaut : un module qui n'a pas encore sa vue dessine comme avant.
      **Un gain non prévu :** `Ton::Bon`, `Attention`, `Mauvais` disent
      un **verdict**, pas une teinte. Deux des trois suivent désormais
      le thème (`warn_fg_color`, `error_fg_color`) là où les modules
      écrivaient `Color32::from_rgb` en dur — un thème pourra changer
      sans qu'aucun module bouge. Le troisième, le vert, n'a pas
      d'équivalent dans egui ; la constante est isolée à un seul endroit
      et le dit.
      **Module pilote : `digest_module`**, choisi pour sa forme (un
      tableau, du texte, une action « copier »). Son `ui()` tient
      maintenant en trois lignes et **dérive de sa vue** — une seule
      description du panneau, donc pas deux qui divergent.
      **Validation :** aller-retour JSON, texte nu d'une ligne, JSON
      sans champ inutile, et surtout
      `ce_que_le_panneau_montre_se_lit_sans_le_peindre` : ce que le
      panneau affiche se vérifie en lisant du **texte**, là où il
      fallait un contexte egui, une surface, un thème et la comparaison
      de rectangles. 1 441 contrôles verts, clippy muet.
      L'invariant `la_liste_des_panneaux_ne_rate_aucun_module` a
      réclamé son dû, comme prévu.
      **Et la capture a tranché deux choses que la mesure ne voyait
      pas.** Comparée au pixel avec celle d'avant le portage, la scène
      « analyse / digest » révélait un artefact : le peintre enveloppait
      une cellule à fragment unique dans une rangée repliable, et le
      trait que `clickable_label` peint sous un texte cliquable
      s'étirait alors bien au-delà du mot. Corrigé — un fragment seul
      ne crée plus de rangée.
      Reste **0,217 % des pixels**, sur une bande de 14 px de haut : le
      dernier paragraphe descend de deux pixels. Le module alternait
      `add_space(4.0)` puis `add_space(2.0)` entre ses trois blocs ; le
      peintre applique un rythme unique. C'est une incohérence corrigée,
      pas une régression — et c'est exactement ce que la frontière
      devait apporter : l'espacement est décidé à **un seul endroit**,
      et un thème plus aéré ne demandera qu'un chiffre.

- [x] **Étape 2 — les modules sans état d'interaction** (29 octobre
      2026). Six portés : `digest`, `slack`, `map`, `yara`, `patterns`,
      `upx`. Restent `xrefs` **et `imports`**, qui gardent eux-mêmes ce
      que l'utilisateur a déplié : ils rejoignent l'étape 3 avec les
      quatre qui dessinent.
      **`patterns` était passif, contrairement à ce que le recensement
      disait** : celui-ci comptait les affectations de
      `on_binary_loaded` comme des interactions. Son repli est un
      `CollapsingHeader` dont **egui garde l'état** — le module n'en
      retient rien, et c'est une qualité qu'il serait dommage de
      perdre. D'où deux ajouts au vocabulaire, structurels tous les
      deux, sans action ni couleur : `Bloc::Repliable`, dont le champ
      `ouvert` ne dit que la position de départ, et `Bloc::Retrait`,
      qui dit un rattachement — « ces lignes développent la
      précédente ».
      **`imports` a été regardé puis reporté, et la raison vaut d'être
      écrite** : il engage deux décisions que le vocabulaire ne porte
      pas encore. (1) Le nom d'une fonction empruntée est un **lien qui
      déplie sa fiche** — il faut une notion d'action, et elle vaudra
      pour les neuf modules interactifs. (2) Chaque entrée porte sa
      **famille de comportement**, peinte par `famille.color()` : une
      couleur arbitraire choisie par le module, exactement ce que la
      frontière doit lui retirer. La bonne réponse est probablement une
      étiquette typée dont l'interface choisit la teinte — comme les
      verdicts. Deux questions à trancher reposé, pas en fin de
      séance.
      **Ce que le recensement a corrigé** : « seize modules de texte »
      était optimiste. Sept seulement n'ont ni bouton, ni saisie, ni
      état d'ouverture ; les autres interagissent, et le vocabulaire de
      la vue ne porte pas encore l'action. C'est une mesure, pas une
      estimation — `grep` sur `framed_button`, `TextEdit`, `checkbox`,
      `selectable_value`, `clickable_label`.
      **Trois réglages de la frontière, trouvés en portant** — c'est à
      cela que sert un pilote :
      (1) le peintre rendait un `GoTarget`, les modules ont besoin de la
      **demande complète** (adresse et cible) ; sans quoi chacun devait
      retrouver de quelle adresse il s'agissait ;
      (2) il imposait le gras aux en-têtes de tableau : `map` les veut
      discrets, `slack` les veut francs. Le ton revient au module, les
      pixels à l'interface ;
      (3) il ignorait l'air entre les colonnes, que `map` écrivait
      lui-même — ses colonnes se touchaient. Quatorze pixels, décidés
      une fois pour tous les tableaux.
      **Et un trou du banc, trouvé en portant `patterns`.** Sa capture
      est revenue **identique au pixel** — mais la scène ne montre que
      « Aucune figure reconnue » : le programme d'épreuve de
      l'interface ne contient aucune figure, si bien que la
      comparaison ne prouvait rien de ce panneau-là. La frontière
      répond exactement à ce trou, et c'est sa première vraie
      justification : `le_panneau_plein_se_lit_sans_etre_peint` bâtit
      un code qui porte des figures et lit ce que le panneau
      montrerait. Il a d'ailleurs corrigé mon attente — **trois**
      familles et non deux, le `mov ecx, "HVUH"` étant lui-même une
      constante en texte, que le panneau a raison de nommer.

      **`upx` a imposé la notion d'action**, celle qui était reportée
      depuis `imports` : deux boutons qui demandent quelque chose à
      l'application. Tranchée au plus simple — un fragment porte le
      **nom** de ce qu'il demande (`Fragment::agit("upx:lister")`), pas
      la demande elle-même. La vue reste du texte et des noms,
      sérialisable et lisible par qui ne connaît pas namsdec ; c'est le
      module qui traduit, et lui seul sait traduire. `inactif` grise un
      bouton sans le cacher : un bouton absent fait chercher, un bouton
      grisé fait lire la raison.
      Deux fonctions de peinture et non une — `peindre` pour les
      panneaux sans bouton, `peindre_tout` qui rend aussi les actions.
      Imposer la seconde à tous aurait mis une liste vide à lire dans
      cinq modules sur six.
      **Et une infidélité que j'avais introduite sans la voir** : les
      lignes de sortie d'upx étaient **sélectionnables** à la souris, et
      le commentaire du code disait pourquoi — « c'est elle, la preuve,
      et c'est elle qu'on recopie dans une recherche » quand upx
      refuse. La vue n'a pas cette notion ; je les avais rendues
      ordinaires, c'est-à-dire inutilisables pour ce qu'elles servent.
      Corrigé en les rendant **copiables d'un clic**, ce qui vaut mieux
      qu'un glisser-déposer au pixel près. Les deux commandes écrites
      (`upx -l`, `upx -d -o`) le sont aussi : elles servent à être
      collées dans un terminal.

      **Validation :** les six panneaux sont comparés **au pixel**
      avec leur capture d'avant le portage. Après ajustement, `slack`
      est revenu identique ; les écarts restants sont des décalages de
      quelques pixels dus au rythme unique — un seul espacement entre
      blocs là où `digest` alternait 4 px et 2 px, un seul entre
      colonnes là où `map` était seul à l'écrire. Aucun contenu, aucune
      couleur, aucun alignement de colonne n'a changé. 1 441 contrôles
      verts.
      **Un gain au passage** : les droits `wx` d'un segment étaient
      peints par un `Color32::from_rgb` en dur dans `map_module`. Le
      module dit maintenant « mauvais » et l'interface choisit la
      teinte — un thème peut changer sans qu'on touche au module.
      **Une règle de manipulation, apprise en se trompant** : porter un
      module en remplaçant tout ce qui va de `fn ui` au marqueur
      suivant emporte ce qui vit APRÈS la méthode — sur `yara`, la
      fonction `build_rule` a disparu avec. On remplace la méthode
      jusqu'à sa propre accolade fermante, et l'on vérifie par un
      `diff` contre la version committée que rien d'autre n'a bougé.
      Le réflexe suivant — un `git checkout` du fichier — a été refusé
      par le garde-fou du projet, et c'était juste : la version
      committée se LIT (`git show HEAD:…`) sans toucher au dépôt.

- [~] **Étape 3 — ceux qui dessinent, et ceux qui retiennent.**
      Fait : **`imports`**, qui exigeait les trois nouveautés à lui
      seul — l'action, l'étiquette typée et le dépliage retenu par le
      module.
      **L'étiquette typée, tranchée** : un fragment porte le **nom**
      d'une catégorie (`behaviour::Family::clé()` : « réseau »,
      « crypto », « anti-debug ») et l'interface choisit la teinte. Les
      modules écrivaient `famille.color()` — ils choisissaient une
      couleur, ce que la frontière doit leur retirer. `Family::color()`
      reste pour `syscalls.rs`, dernier appelant de la version peinte,
      et quittera `behaviour.rs` — qui n'a rien à faire d'egui — quand
      il sera porté.
      **Le lien-qui-agit** : par défaut une action fait un **lien**,
      du texte qu'on clique ; `.bouton()` en fait un bouton encadré.
      Deux messages différents — « ceci va lancer quelque chose » pour
      `upx`, « ceci déplie » pour un nom de fonction.
      **`Ton::Repère`** pour le nom d'un registre dans une fiche : il
      n'est pas cliquable, mais il désigne ce que le bandeau du
      processeur montre. « Repère » et non « lien » : un lien se
      clique, celui-ci se regarde.
      **Validation :** capture identique au pixel — mais la scène ne
      montre, là encore, que le cas vide. Le contrôle qui compte est
      `une_fiche_se_deplie_par_son_action_et_se_referme_de_meme`, qui
      suit le chemin entier **sans peindre** : la vue propose
      l'action, la bascule l'applique, la vue suivante porte la fiche
      avec sa signature et le registre du premier argument, et la même
      action referme. Éprouvé en cassant la bascule. La logique a été
      sortie de `ui()` pour cela — c'est un comportement, pas un
      dessin, et le vérifier en comptant des galères de texte revenait
      à mesurer la fenêtre pour savoir ce que le module retient.
      **Un contrôle existant a maigri au passage** :
      `une_fonction_inconnue_dit_ce_que_son_nom_apprend` demandait un
      contexte egui, deux images et la collecte des formes de texte
      pour lire ce qu'une fiche dit. Il lit maintenant des blocs.
      L'invariant `aucun_commentaire_n_a_perdu_sa_fonction` a réclamé
      son dû : j'avais glissé `portrait_en_blocs` entre le doc-comment
      de `portrait` et sa signature.

      **`xrefs` suit**, et sans rien ajouter au vocabulaire : le
      signe « + » devient une action nommée comme la fiche
      d'`imports`, et `bascule` sort de `ui()` pour les mêmes raisons.
      Son contrôle est bâti sur **sept octets assemblés à la main** —
      un `call` et deux `ret` — plutôt que sur un fichier de la
      machine : il tourne partout, et il nomme exactement ce qu'il
      éprouve.

      **La décompilation suit**, premier des quatre « qui dessinent » —
      et celui qui montre que le mot était trompeur : ce qu'elle peint
      est du **texte**. D'où `Bloc::Code(String)` : le texte est la
      donnée, la coloration et les adresses qu'on y clique sont de la
      présentation. Un rapport en reçoit du code lisible, ce qui est
      exactement ce qu'il veut ; l'interface, elle, continue d'en faire
      des liens vers le listing. Les fonctions deviennent des
      `Repliable`, le bouton une action.

      **L'entropie suit, et avec elle la forme « dessin »** —
      `Bloc::Dessin(Dessin::Profil { valeurs, taille_tranche, total })`.
      Le module ne dit plus qu'une **valeur par tranche** ; les
      260 lignes qui décident « une colonne par pixel, et la plus haute
      des tranches qu'elle recouvre » ont déménagé dans le peintre,
      parce que cette décision n'a de sens qu'une fois la largeur
      connue. Les verdicts passent par `Ton` : le module dit
      « mauvais » selon les seuils d'`entropy::seuils`, l'interface
      choisit la teinte.
      **La capture a trouvé deux choses que les contrôles ne voyaient
      pas.** (1) Une vraie inexactitude : j'avais déduit la taille du
      fichier de `tranches × taille_tranche`, mais **la dernière
      tranche est plus courte** — la règle d'adresses sous le profil
      était graduée jusqu'à `0x2300` au lieu de `0x22c8`, soit
      cinquante-six octets qui n'existent pas. Le total est maintenant
      **dit** par le module, non déduit. (2) Les points médians de la
      ligne de résumé s'étaient collés à leurs voisins, parce que
      j'avais fondu trois étiquettes distinctes en un seul `format!`.
      Séparées de nouveau, la ligne est identique au pixel.
      **Les deux sont retenues par un contrôle**, et chacun a été
      éprouvé en remettant le défaut : le premier choisit une taille de
      `0x22c8` — que la tranche ne divise pas — et vérifie que la
      déduction se trompe VRAIMENT là, sans quoi il ne distinguerait
      pas les deux façons de compter ; le second compte les points
      médians comme fragments à part.

      **Le graphe d'appels suit — onze sur douze**, avec
      `Dessin::Graphe { nœuds, arêtes }` : des nœuds dans l'ordre où on
      les lit, leur distance à la racine, et les arêtes entre eux. Rien
      du placement n'est dit — ni pas vertical, ni indentation, ni
      largeur de boîte —, et les deux cents lignes qui en décident sont
      passées au peintre avec les tirets et la pointe ouverte d'UML. Ce
      que le module disait en deux drapeaux (`accent`, `renvoi`), il le
      dit en **`Ton`** : « fort » pour ce par quoi on entre dans le
      programme, « faible » pour ce qui est déjà déplié ailleurs.

      Trois formes nouvelles, chacune comptée avant d'être écrite :

      * **`Bloc::Champ`** — treize champs de saisie dans sept modules,
        relevés. Le texte en cours de frappe appartient à **celui qui
        peint**, comme l'état d'ouverture d'un repliable : le module ne
        l'apprend qu'à la validation, ce qui laisse `vue()` n'emprunter
        `self` qu'en lecture. La largeur se dit en **caractères** — le
        module sait qu'on y écrit une adresse, l'interface sait ce que
        mesurent ses caractères ; les sept largeurs en dur d'avant P7
        (90, 110, 120, 140, 160, 200, 220 pixels) étaient sept réglages
        pour une seule question.
      * **`vue::Nœud`** — les trois compartiments d'une classe UML, en
        texte : ce que la fonction EST, son nom, ce qu'on en mesure.
      * **`Fragment::à_la_main`** — « ceci vient de votre main », un
        FAIT ; le fond léger qui le distingue est choisi par
        l'interface.

      **Une mesure a décidé de la forme des fiches.** Le détail d'une
      fonction, c'est jusqu'à deux cents lignes de pseudo-code décodées
      une à une ; une vue se rebâtit à chaque image ; les quarante
      fonctions montrées coûtent **2,6 ms en release et 8,5 ms en
      debug** — sur un budget de 16,7 ms — pour du texte que personne
      ne regarde, puisque les fiches sont fermées au départ (mesuré sur
      `croackpocalypse`, 7 443 lignes). Le module retient donc **une
      fiche ouverte à la fois**, comme `imports`, et ne compose que
      celle-là. Un contrôle compte les tableaux de la vue : zéro quand
      rien n'est ouvert, un quand une fiche l'est.

      **Et quatre défauts trouvés en chemin, dont trois d'avant P7.**

      1. **Une action sans encadré se peignait en bouton.** La vue dit
         depuis le premier jour qu'une action fait un **lien** par
         défaut et qu'un encadré se demande (`Fragment::bouton`) ; le
         peintre encadrait tout, et le drapeau ne servait à rien. Les
         noms d'`imports` étaient des textes soulignés avant P7 — ils
         étaient devenus cinquante boutons dans une colonne.
      2. **Le titre d'un repliable perdait son ton.** Peint en texte
         nu, `Fragment::dit(nom).code().fort()` s'écrivait comme une
         phrase ordinaire : un nom de fonction en chasse
         proportionnelle, dans le panneau de décompilation.
      3. **Une fonction de 18 446 744 073 697 542 199 octets.** Vue sur
         la capture. Un `call` vers une adresse quelconque — ce que
         produit la moindre figure d'anti-désassemblage, et ce que
         porte le binaire d'épreuve — fabriquait une « fonction » là-bas,
         et la fonction d'AVANT allait jusqu'à elle. Sa fiche demandait
         les instructions d'un intervalle large comme la mémoire. Les
         débuts de fonction sont désormais bornés au code, comme les
         appels calculés l'étaient déjà ; l'appel, lui, reste compté.
      4. **Une cellule de tableau qui ne tient pas courait hors du
         panneau.** egui ne replie pas le texte dans une disposition
         horizontale, et une grille en est une. Les deux premières
         façons de corriger ont été **réfutées par la mesure**, et
         c'est ce que la séance a de plus utile à garder : *replier*
         réduisait l'en-tête « the listing reads » à sept pixels sur
         trois lignes, par-dessus sa voisine ; *élider à la largeur
         disponible* faisait boule de neige — une grille garde la
         largeur de ses colonnes d'une image sur l'autre, couper à
         cette largeur retire une lettre, qui rétrécit la colonne, qui
         recoupe le texte : « SHA-256 » devenait « … » en trois images.
         La borne se tire donc des largeurs **naturelles**, mesurées
         sans contrainte, et ne touche que la colonne la plus large.
         Tant qu'un tableau tient, **rien ne change, pas un pixel**.

      **111 captures sur 113 identiques au pixel.** Les deux autres :
      le graphe d'appels, et la poignée d'ascenseur de la vue
      d'ensemble — son contenu a changé de quelques pixels de haut.

      **Reste un module** :

      * **l'éditeur binaire** (`hex_editor_module`, 1 966 lignes, dont
        856 pour le seul `fn ui`). **Relevé fait avant d'y toucher**,
        comme au début de P7 — et il déplace la question.

        Ce qu'il emploie et que le vocabulaire n'a pas : une
        **case à cocher** (1 occurrence dans tout le crate, la
        sienne), des **étiquettes à bascule** (2, les siennes), une
        **zone bornée qui défile** (4 occurrences, 3 à lui et 1 au
        vidage), une **liste déroulante** (1, au détour). Autrement
        dit : **des formes à un seul usage**, et le fichier `vue.rs`
        dit en tête pourquoi c'est un signal d'arrêt — « une vue assez
        riche pour tout décrire serait un second HTML ».

        Et surtout le **clavier**. La grille prend le focus et consomme
        **vingt-six touches** : quatre flèches, page haut et bas, début
        et fin, Échap, Ctrl+A, Tab, les seize chiffres hexadécimaux,
        plus les événements de texte qu'elle retire de la file. Chacune
        déplace un curseur ou étend une sélection. Une vue, elle, ne
        connaît ni focus ni touche — et lui en apprendre reviendrait à
        refaire la gestion du clavier en protocole de chaînes, ce qui
        serait payer le prix d'une frontière sans en avoir le gain.

        **Ce que je propose, et la raison :** la frontière s'arrête
        ici, et c'est une conclusion de P7, pas un échec. Ce que la vue
        sert à faire sortir de namsdec, c'est **ce que namsdec
        comprend** d'un binaire — un profil d'entropie, un graphe
        d'appels, des figures, des empreintes. Une grille d'octets avec
        un curseur n'est pas une compréhension, c'est un
        **instrument** ; et ce qu'un rapport voudrait de ce panneau —
        la liste des octets changés — existe déjà ailleurs, dans les
        correctifs. Reste que **l'inspecteur** (« ce que cet octet vaut
        lu en int8, en float32, en date ») est, lui, de la
        compréhension pure : `inspecter()` rend déjà des paires
        nom/valeur sans toucher à egui, et c'est la part qui mérite de
        passer — seule, et sans emporter la grille avec elle.

        **Décidé le 3 novembre 2026 : l'inspecteur passe, seul.**
        `vue_de_l_inspecteur` dit ce que l'octet choisi vaut dans chaque
        format ; `ui()` le peint par le peintre commun ; la grille, son
        curseur et ses vingt-six touches restent peints sur place.

        Trois choses mesurées en route, et deux ont corrigé ce que
        j'allais écrire :

        1. **Une scène de banc manquait.** L'inspecteur n'existe que si
           un octet est choisi, et aucune scène n'en choisissait : ses
           seize lignes n'étaient mesurées par rien — ni le banc, ni les
           captures. La scène « analyse / éditeur binaire, un octet
           choisi » passe par `show_bytes_at`, le chemin réel, et met
           l'inspecteur sous les **vingt-cinq invariants** du banc.
        2. **La boucle que le code redoutait ne se reproduit plus.** Ce
           panneau portait écrit qu'aucune disposition imbriquée ne
           devait y entrer — l'inspecteur était une grille, la fenêtre
           d'analyse se dimensionnait sur son contenu, et la fenêtre
           gonflait jusqu'à 2878 pixels. Remis en grille et mesuré :
           tout reste vert, y compris l'invariant qui suit une fenêtre
           sur vingt images. La borne posée depuis sur le contenu de la
           fenêtre a fermé la boucle.
        3. **Et le rembourrage, lui, n'alignait plus rien.** La parade
           d'alors — le nom rembourré à onze caractères, pour aligner
           sans colonne — ne survit pas à la description en fragments :
           les blancs de fin disparaissent au rendu, et les valeurs
           partaient de 51, 56, 59 puis 68 pixels selon la longueur du
           nom. Mesuré, pas relu. D'où le tableau, qui aligne par
           construction.

        **Un défaut de vocabulaire trouvé en chemin, et c'est le plus
        utile :** `code` était un **ton**, au même rang que « faible »
        et « fort ». `.code().faible()` ne disait donc que « faible » —
        le second appel écrasait le premier, en silence. Trois endroits
        l'écrivaient, dont deux dans le panneau de décompilation : une
        signature C et un nom de fonction s'y peignaient en chasse
        **proportionnelle**. La chasse est maintenant un axe à part
        (`Fragment::chasse_fixe`), et un fragment peut demander les
        deux.

      **Et le compte était faux : 11 sur 12, non — 14 sur 22**
      (3 novembre 2026). Le « 12 » ne comptait que les modules nommés
      aux étapes 2 et 3. Vingt-deux implémentent `ReModule` ; onze
      avaient une vue. Trois de plus sont passés — **`syscalls`,
      `binary_info`, `protections`** —, choisis parce qu'ils ne
      demandaient rien de neuf au vocabulaire sauf les deux formes
      ci-dessous, parce qu'un rapport les veut d'abord (en-tête ELF,
      protections, appels système), et parce que `syscalls` était le
      **dernier appelant de `Family::color()`**.

      * **`behaviour.rs` ne connaît plus egui.** Il relève des familles
        de comportement dans des noms de fonctions et des numéros
        d'appels : il n'avait aucune raison de porter une table de
        couleurs. Elle vit dans `vue_peinte::couleur_de_famille`. La
        version peinte d'`imports_module::portrait`, gardée en double
        le temps que `syscalls` passe, a disparu — un seul portrait
        pour les deux panneaux, qui ne peuvent donc plus diverger.
      * **`Bloc::Verdict`**, compté avant d'être écrit : **seize lignes
        de verdict, trois marques différentes** pour la même donnée —
        une pastille ronde au `painter` dans Protections, un `√`/`×`
        dans « En bref », les mêmes glyphes dans un **autre vert** dans
        la fiche du panneau latéral. Le module dit le nom, l'état, le
        verdict et le pourquoi ; la marque, la teinte et la place du
        pourquoi sont décidées une fois. Le verdict est un **`Ton` et
        non un booléen**, et c'est ce qui corrige deux affirmations
        fausses : un RELRO « partiel » et une pile « non déclarée » se
        peignaient en rouge, au même rang qu'absent.
      * **`Fragment::renvoi`** — une adresse de page, et une seule dans
        tout le crate. Un usage unique est ailleurs un signal d'arrêt ;
        la différence est qu'une adresse de page n'apprend rien au
        vocabulaire sur la façon de peindre, et qu'un rapport la veut.

      **Trois défauts trouvés en chemin, et le premier est le plus
      grave.** (1) **« En bref » disait l'inverse de la vérité** : le
      panneau portait une note par protection, écrite pour le cas où
      celle-ci est PRÉSENTE, et la peignait quel que soit le verdict —
      « × PIE — chargé à une adresse au hasard ». Vu sur la capture ;
      aucun contrôle ne regardait cette phrase. (2) **Le contrôle écrit
      pour la retenir passait en remettant le défaut** : il lisait
      `crackme-01`, qui A du NX, donc la seule variante qu'il éprouvait
      était celle que le défaut rend. Refait en basculant l'en-tête
      sous le module. (3) Une assertion **fausse par construction**
      (`f.texte == "write" && f.texte.contains("non simulé")`), donc
      jamais rouge.

      **Et la scène du banc ne montre qu'une tranche des appels
      système** — `write` et `exit`, sans famille et tous deux simulés.
      Ni le portrait, ni « non simulé », ni la fiche n'y sont peints.
      Deux contrôles les lisent sur quatorze octets bâtis à la main.
      **Mesuré :** six captures sur 122 ont changé ; `syscalls` à
      0,066 %, soit le rythme unique et rien d'autre. 1 100 contrôles
      verts.

      **Le protocole, qui a servi treize fois et qu'il suffit de
      reprendre :** lire le `fn ui` entier ; remplacer **la méthode
      seule**, jusqu'à sa propre accolade — pas jusqu'au marqueur
      suivant, c'est ainsi que `build_rule` a disparu de `yara` ;
      sortir dans une méthode à part ce qui est un COMPORTEMENT (une
      bascule, un dépliage) pour pouvoir l'éprouver sans peindre ;
      comparer la capture **au pixel** avec celle d'avant ; et, quand
      la scène du banc ne montre que le cas vide — c'est arrivé pour
      `patterns`, `imports` et `digest` —, écrire un contrôle qui lit
      la vue sur des données bâties à la main.
      Les quatre qui peignent vraiment — entropie, graphe d'appels,
      éditeur binaire, décompilation —, plus `xrefs` et `imports`, qui
      gardent en propre ce que l'utilisateur a déplié.
      **La question qui reste ouverte, et qu'`imports` posera :** une
      entrée y porte sa famille de comportement, peinte par
      `famille.color()` — une couleur choisie par le module, exactement
      ce que la frontière doit lui retirer. La réponse est
      probablement une **étiquette typée** dont l'interface choisit la
      teinte, comme pour les verdicts `Bon`/`Attention`/`Mauvais`.
      Les quatre qui peignent vraiment. Ceux-là exposent leurs
      **données** — fenêtres d'entropie, nœuds et arêtes, octets, texte
      décompilé — et non un dessin. Le dessin reste dans l'interface,
      qui est son métier.

- [x] **Étape 4 — ce que la frontière permet** (3 novembre 2026), et
      qui la justifie. Tout est dans `rapport.rs`, et tout sort des
      mêmes vues que l'écran.

      * **Un rapport complet.** `report::markdown` est une prose écrite
        à la main, et elle reste en tête — mais elle ne couvrait que
        **six rubriques sur vingt-deux** : ni l'entropie, ni les
        chaînes, ni les appels système, ni les fragments ROP, ni le
        graphe d'appels n'en sortaient. Le rapport porte désormais, à la
        suite, **ce que chaque panneau montre**. Une seule source, donc
        un rapport qui ne peut plus prendre du retard sur l'écran.
      * **Un export JSON**, par la même porte : c'est **l'extension du
        fichier qu'on nomme** qui décide, plutôt qu'un bouton de plus
        dans une barre qui en porte déjà trois. L'aller-retour est
        éprouvé, et ce qui n'est pas dit ne pèse rien.
      * **Une analyse sans interface** : `rapport::analyse(octets,
        lang)` rend tout ce que namsdec comprend d'un fichier **sans
        rendre une seule image** — ni contexte egui, ni surface, ni
        thème, ni pixel. C'est le contrôle
        `un_binaire_s_analyse_sans_peindre`.

      **Deux décisions prises en chemin.**
      (1) **L'ordre des panneaux a déménagé** de `NamsdecApp::default`
      vers `ReModuleRegistry::standard` : c'est une propriété de
      l'ANALYSE — de quoi on parle, puis ce qui en ressort, puis le
      code, puis les octets —, et un rapport qui l'aurait recopié aurait
      fini par diverger. Cent quarante-huit lignes de moins dans
      `main.rs`, raisonnement de chaque rang compris.
      (2) **`dans_un_rapport`**, un fait de plus sur un module : les
      cinq panneaux d'outils extérieurs disent « radare2 n'est pas
      installé ici », ce qui est utile à l'écran et n'apprend rien à qui
      lira le rapport ailleurs. Dès qu'un outil a répondu, sa sortie y
      entre comme le reste.

      **Et un défaut vu en LISANT le rapport, non en relisant le
      code :** `Ligne::texte_nu` colle les fragments bout à bout, ce qui
      est juste — une ligne de vue n'est pas une phrase, c'est une suite
      de morceaux que la mise en page sépare. À l'écran, egui pose son
      espacement ; dans un fichier, personne ne le fait, et l'on lisait
      « Pas de canari de pile—aucun témoin posé ». Le rapport recolle
      donc avec un blanc, après avoir rogné chaque morceau — certains
      portent encore un espace de tête, du temps où le module posait
      lui-même son rythme.

      **Ce que cela ne fait pas, et il faut le dire :** `namsdec-re`
      dépend toujours d'egui, parce que le trait `ReModule` porte encore
      `fn ui(&mut self, ui: &mut egui::Ui, …)`. S'en défaire demanderait
      de déménager les vingt-deux `ui()` dans `namsdec-gui` — c'est un
      autre lot. Ici, « sans interface » veut dire : aucune image n'est
      rendue, et le résultat ne dépend d'aucun pixel.

### Référence de départ et clôture des lots

**Au départ (28 octobre, avant les lots).** La revue a exécuté
`cargo test --workspace --offline` : **1 405 tests réussis, 0 échec,
5 ignorés**. Le contrôle explicite `captures_de_toutes_les_scenes` a
réussi et produit **112 images** ; seules les vues source chargée et
analyse chargée ont été inspectées visuellement. L'oracle matériel et
clippy n'ont pas été exécutés lors de cette revue. Ces chiffres sont un
état daté, pas des seuils à figer.

**Après P1, P2, P3 et le début de P7 (29 octobre).**
`cargo test --workspace --offline` : **1 441 tests réussis, 0 échec,
5 ignorés** — trente-six de plus qu'au départ, tous écrits pour ces
lots. `cargo fmt --all --check` propre, `cargo
clippy --workspace --all-targets` **sans un avertissement**.
`captures_de_toutes_les_scenes` a produit **113 images**, dont la scène
nouvelle « console / Diagnostics, simplifications » — le seul état où la
rubrique des approximations existe. L'oracle x87
(`cargo run -p namsdec-emu --example oracle_x87`) rend **18 cas
identiques, 2 écarts attendus et annoncés, 0 défaut**.
Ce qui **reste non vérifié**, et qui est dit plutôt que passé sous
silence : l'oracle général (`--example oracle`) n'a pas été relancé lors
de cette séance, aucun lot n'ayant touché à la sémantique qu'il éprouve ;
les transcendantes du x87 ne sont pas comparées cas par cas au matériel ;
et un descendant qui se détache par `setsid` échappe toujours à l'arrêt
de groupe.

Pour chaque lot de code : contrôles ciblés, puis vérifications requises
par le projet (`cargo fmt --all --check`, clippy et tests du workspace).
Consigner exactement ce qui a été exécuté, ce qui a échoué ou reste
non vérifié. Pour une modification visible, appliquer aussi la règle du
manuel en fin de fichier. Conserver la version actuelle : cette liste
ne demande aucun changement de numéro ni aucune publication.

---

## Où en est l'interface

**Non, l'UX/UI n'est pas terminée.** Une passe complète a été faite le
25 septembre : seize scènes capturées sur profil neuf, deux relevés de
mesure sur les trente-trois scènes du banc, **vingt et un écarts corrigés,
dix-sept contrôles nouveaux et un élargi**. Tout ce qu'elle avait mesuré
et pas corrigé est fait depuis.

**Et il restait un relevé à refaire**, contrairement à ce qui était
écrit ici. La passe du 23 octobre — trois relevés menés en parallèle,
sur trois axes que le banc ne regardait pas — a trouvé **neuf écarts de
plus**, dont deux trous du banc lui-même : une scène qui ne montrait pas
l'onglet qu'elle nommait, et une quatrième écriture de bouton que le
contrôle de source ne connaissait pas. Voir la section 1 ci-dessous.

Et une leçon de méthode, payée d'une demi-journée le 25 octobre : il y a
**trois endroits où mesurer**, et non deux. Le banc, la capture, et
*l'application en marche* — six `eprintln` dans le code qui pose les
rectangles. Quand le banc dit « peint » et que l'image dit « vide », ni
l'un ni l'autre ne tranche ; le programme, lui, sait où il a posé ses
rectangles. Voir la section 0 ter.

La leçon qui se répète : ce qu'un relevé ne cherche pas, aucun contrôle
ne le retient. Les trois axes de cette passe — l'écran le plus petit,
l'inventaire des tailles, l'inventaire des commandes — n'étaient pas des
défauts suspectés, c'étaient des questions qu'on n'avait jamais posées.

---

## 0 trequadragies. Aucun survol ne déplace ce qui l'entoure (A 9.80)

Quatre captures : « des glitchs quand le curseur se positionne sur un
bouton ». Mesuré : la page des Outils descendait de **vingt-quatre
pixels** au survol.

- [x] **La largeur du contenu est imposée** avant la zone qui défile,
      barre comprise : elle ne dépend plus de ce que la barre décide.
- [x] **Aucun état n'ajoute de trait tout seul** — egui compte le trait
      dans la taille, et le survol en ajoutait un. Deux essais : donner
      au repos la largeur du survol a décalé l'entrée **sélectionnée**
      de la colonne d'analyse, peinte dans un cinquième état sans trait.
      Les quatre passent donc à zéro ; les cadres de namsdec sont
      demandés bouton par bouton et valent dans tous les états.
- [x] **Un contrôle qui survole**, et c'est ce qui manquait au banc : il
      ne mesurait que des images sans pointeur, c'est-à-dire la moitié
      de ce qu'une interface fait.

**J'ai accusé deux innocents avant de mesurer** : l'« expansion » des
widgets (nulle partout) et le cadre du bouton (159,7 px dans les deux
états). La cause était une boucle — contenu qui tient à trois pixels
près, barre qui paraît, rangée qui bascule, page qui grandit.

---

## 0 duoquadragies. L'éditeur du débutant n'a pas de modes (A 9.79)

- [x] **Taper écrit**, sans avoir à connaître `i`. L'application
      s'ouvrait en mode Normal, où le champ n'est pas interactif : on
      cliquait, on tapait, et rien ne se passait.
- [x] **La pastille de mode disparaît avec eux** — « NORMAL »
      nommerait une chose qui n'existe plus.
- [x] **Le niveau avancé garde ses modes**, et le contrôle le vérifie
      aussi : les retirer partout serait retirer à l'outil une de ses
      raisons d'être.
- [ ] **Le jeu de touches par défaut** — « Exécuter » et « Pas à pas »
      sont des séquences qui ne s'arment que hors de l'éditeur. Un
      débutant qui vient de taper ne les atteint pas au clavier. Le jeu
      « Éditeur ordinaire » (F5, F10, F11) marcherait pendant la frappe,
      mais changer le défaut d'une installation neuve touche à
      l'identité de l'outil : **à décider**, pas à glisser dans un lot.

---

## 0 unquadragies. L'accueil tient dans l'écran (A 9.78)

- [x] **Le blanc du haut se mesure sur la place** — posé à
      soixante-quatre pixels, il repoussait les récents sous le bord à
      800×520.
- [x] **Et une zone qui défile en filet**, pour les cas qu'on n'a pas
      prévus : dix récents, une police grossie.

Un accueil qui cache la moitié de ce qu'il propose est pire qu'un
éditeur vide : il montre qu'il y a quelque chose et ne le donne pas.

---

## 0 quadragies. Une rangée d'onglets tient sur une ligne (A 9.77)

Revue d'ergonomie : les cent vingt scènes rendues au plus petit écran
que l'application accepte (800×520), et regardées.

- [x] **Une seule ligne, et ce qui ne tient pas derrière « +5 »** — les
      dix vues prenaient trois lignes, quatre-vingt-seize pixels, et il
      restait quatre lignes de programme à l'écran.
- [x] **Ce qu'on regarde reste visible** : la fenêtre d'onglets glisse
      jusqu'à contenir l'actif.
- [x] **Le banc rend aussi le petit écran** (`captures_du_petit_ecran`)
      — c'est là que vivent les défauts d'ergonomie.
- [x] **Et la règle n'est pas générale** : au-dessus d'un contenu court
      (la rangée des vues) une ligne de plus coûte l'écran ; au-dessus
      d'un panneau haut (le panneau latéral) vingt-six pixels ne coûtent
      presque rien. Appliquée partout, elle cachait l'onglet RE derrière
      « +1 » — on allume le module, et rien ne change.

- [x] **Et un écart de 2,2 px** entre « +6 » et le disque du compteur,
      vu à la capture : rien ne se recouvrait, mais les deux se lisaient
      comme un seul objet cassé. **J'ai d'abord écrit le mauvais
      invariant** — « aucun dessin ne se peint sur un texte » —, qui
      passait même en remettant le défaut : une rangée alloue ses objets
      l'un après l'autre, le recouvrement y est impossible. C'est
      l'ÉCART qu'il fallait mesurer.

**Le repli était voulu, et un contrôle veillait à ce qu'il ne recouvre
rien.** Ce que personne n'avait mesuré, c'est ce qu'il coûte : un
invariant qui dit « rien ne se recouvre » est content d'un écran où il
ne reste rien à lire.

---

## 0 nonatricies. La barre d'icônes de la console (A 9.75)

Dernière des consignes du 4 novembre.

- [x] **Relancer, arrêter, effacer** au bout de la rangée d'onglets de
      la console — à côté de ce qu'elles changent.
- [x] **Effacer n'est pas réinitialiser** : l'écran se vide, le
      processeur ne bouge pas. Le contrôle clique pour de vrai.
- [x] **Éteinte, chacune dit pourquoi** — règle de la barre d'outils.
- [x] `clear_output` au moteur : la première méthode du trait qui ne
      change rien à l'exécution.

**Dans la rangée, et non en colonne à gauche** comme chez JetBrains : la
console garde ses quatre-vingts colonnes. Seul endroit où la consigne et
une règle du projet se contredisaient — la règle a une raison, la
ressemblance n'en est pas une.

---

## 0 octotricies. Les barres d'icônes des bords (A 9.74)

Capture annotée : trois rectangles sur les barres de CLion, « Afficher
l'explorateur », « cacher l'explorateur ».

- [x] **Barre de gauche** — l'explorateur et les exercices en haut, la
      console en bas, du côté et à la hauteur où le panneau s'ouvre.
- [x] **Barre de droite** — le panneau du processeur.
- [x] **L'infobulle dit le geste**, et le raccourci est lu dans le
      tableau.
- [x] **Un tiret au bout de chaque rangée d'onglets** : le panneau se
      range de là où il est.
- [x] **Affichage › Barres d'icônes** les éteint ; le réglage est gardé.

**Le défaut que seul le contrôle pouvait trouver** : l'icône de
l'explorateur ne pouvait pas le refermer. L'appui tombe hors de la
fenêtre flottante, qui se ferme ; le relâchement arrive sur l'icône d'un
panneau devenu fermé, et la rouvre. Un clic sur la barre n'est pas un
clic « ailleurs ».

**Et une mesure qui mesurait un trait** : le séparateur de la barre
tombe à son bord droit, à mi-hauteur, et se faisait compter pour une
icône. Quatrième fois cette semaine qu'un contrôle vert ne prouve rien.

---

## 0 septentricies. Le compte des problèmes (A 9.73)

Consigne annotée « Erreurs et warning », flèche sur le badge de CLion.

- [x] **Le compteur au bord droit de la rangée des vues** : disque rouge
      et nombre d'erreurs, triangle orange et nombre d'avertissements.
- [x] **Un clic mène à la liste**, sur le bon onglet ; les **chevrons**
      mènent au problème d'avant et à celui d'après, en bouclant.
- [x] **Rien à compter, rien à l'écran** — un compteur à zéro en
      permanence est un endroit qu'on apprend à ne plus regarder.
- [x] **Les chevrons sont dessinés** : « ↑ » et « ↓ » se peignaient en
      carrés vides, la police ne les portant pas.

**Et deux contrôles faibles, trouvés en les attaquant** : vérifier
l'absence d'un « 0 » qui ne s'écrit jamais, et mesurer sur une vue qui
n'appelle pas ses commandes. Un contrôle qui ne tombe pas quand on remet
le défaut ne contrôle rien.

---

## 0 sextricies. L'accueil (A 9.72)

« Quand je quitte, j'aimerais que tout soit fermé, et une fenêtre comme
la suivante apparaisse — celle comme Zed. »

- [x] **L'accueil à la place de l'éditeur** quand on a fermé le dernier
      fichier : le nom, ce par quoi on recommence, les récents.
- [x] **Un drapeau, et non la seule vue du tampon vide** — déduire
      l'accueil de « vide et sans fichier » le ferait paraître au milieu
      d'une frappe. L'accueil se demande, par un geste : fermer.
- [x] **Les raccourcis lus, non recopiés** : Ctrl+N réglé sur Ctrl+Alt+K
      se lit Ctrl+Alt+K. Les récents n'en affichent aucun — Zed leur en
      donne, mais il faudrait inventer cinq touches globales.
- [x] **Pas d'onglet fantôme** : « Nouveau fichier » depuis l'accueil
      n'empile pas le tampon qu'on vient de fermer.

**Et la même règle d'egui que ce matin** : un `Ui` hérite de la mise en
page de son parent. Les quatre entrées se peignaient les unes sur les
autres, côte à côte. Il faut dire `top_down` même quand « de haut en
bas » va de soi.

---

## 0 quintricies. La rangée « Moteur : » (A 9.71)

Deux consignes écrites sur une capture du 4 novembre.

- [x] **« Ça doit être aligné »** — l'étiquette était cinq pixels au-
      dessus de ses choix. Dans une rangée egui, chaque élément est
      centré sur la hauteur que la rangée a **à cet instant** : une
      étiquette qui ouvre la rangée se pose avant que les boutons ne
      l'aient fait grandir. Deux essais, le premier faux —
      `interact_size` n'est pas la hauteur d'un `selectable_value`.
- [x] **Trois moteurs au niveau débutant** — pseudo-code, NASM et
      `objdump` (Intel), celui qu'on a sur toute machine et dont la
      syntaxe est celle qu'on vient d'écrire. Et **ce qu'on regarde
      reste listé** : un moteur choisi au niveau avancé garde sa place.
- [x] **La scène manquait** — aucune ne montrait cette rangée au niveau
      débutant. Troisième fois de la semaine qu'une consigne venue d'une
      capture désigne un écran que le banc ne regardait pas.

**Et un écart que j'ai cru voir et qui n'existait pas** : trois
abscisses dans la même colonne (10, 12, 14). C'est l'encart du texte
dans sa boîte — un champ pose son texte à quatre pixels de son bord. Les
widgets commencent tous à 10. Un relevé qui compare des positions de
**texte** ne dit rien des bords.

---

## 0 quatertricies. La console : ce qu'on règle quitte ce qu'on lit (A 9.70)

Consigne écrite sur une capture : « la console et les autres champs ne
sont pas adaptés → inutilisable, s'inspirer des outils jetbrains ».

- [x] **La scène manquait** — aucune ne montrait une console à hauteur
      **automatique** devant un programme qui a écrit ; la seule qui
      charge un binaire fige sa hauteur. Ajoutée, elle sert de mesure
      avant/après.
- [x] **Les arguments et l'environnement quittent la console** — ils ne
      se tapent pas pendant qu'un programme tourne, ils sont posés sur
      la pile **avant** la première instruction. Deux rangées sur quatre
      rendues à la sortie. Bouton « Arguments… » à droite de la rangée
      d'onglets — la place des commandes dans cette application, et
      celle qu'elles ont dans les captures de référence. Réglage gardé
      dans `settings.json`.
- [x] **Ce qu'on tape reste** — la ligne « Entrée : » est toujours là :
      c'est une décision écrite, « un champ qui n'apparaît qu'à une
      condition que l'utilisateur ne peut pas voir n'est pas une
      économie, c'est une disparition ». Le contrôle le vérifie, pour
      qu'un lot suivant ne la retire pas par distraction.
- [x] **Une barre d'icônes de panneau** — relancer, arrêter, effacer :
      faite le 5 novembre, voir « 0 nonatricies » (A 9.75). Dans la
      rangée d'onglets et non en colonne à gauche : la console garde ses
      quatre-vingts colonnes.
- [x] **Les barres latérales à icônes** qui montrent et cachent les
      panneaux : faites le 5 novembre, voir « 0 octotricies » (A 9.74).

---

## 0 tertricies. « Restaurer » ne restaurait rien (A 9.69)

Signalé à l'écran, capture à l'appui : on agrandit une fenêtre, on
clique « Restaurer », elle reste pleine page. **Les sept fenêtres
agrandissables** étaient dans ce cas.

- [x] **La cause** — `fixed_pos` et `fixed_size` écrivent dans la
      mémoire d'egui ; au retour, les `default_*` qui les remplacent ne
      valent que la première fois. Personne ne gardait ce qu'il fallait
      rendre. On retient désormais, à chaque image, où chaque fenêtre se
      tient quand elle est normale — et « Restaurer » le lui réimpose le
      temps d'une image.
- [x] **Trois pièges, tous trouvés par la mesure** — deux identifiants
      pour une même fenêtre (la clé d'agrandissement n'est pas
      l'identifiant egui, et la première écriture du contrôle les a
      confondus) ; le bornage d'egui qui emploie la taille de l'image
      d'avant, donc celle de la pleine page, et colle la fenêtre au
      coin ; le pivot, qui est celui de la fenêtre et non le nôtre —
      relevé en même temps que le rectangle, sans quoi la fenêtre saute
      d'un demi-format à l'image suivante.
- [x] **Et la restauration se pose en dernier**, comme
      `maximizable` le dit déjà de l'agrandissement : posée avant, le
      `pivot` et le `default_size` du déroulement et du décompilateur
      l'écrasaient.
- [x] **Le contrôle** — `une_fenetre_restauree_retrouve_sa_taille`
      vérifie taille **et** position sur les sept fenêtres, et exige
      d'abord que l'agrandissement agrandisse. Éprouvé en retirant la
      restauration : quatorze écarts.

---

## 0 duotricies. L'écran que l'on voit en premier (A 9.68)

Le mode débutant devient celui par défaut : l'écran du premier
lancement est donc celui que tout le monde verra. Consigne : ne garder
que le nécessaire.

- [x] **La scène qui manquait** — le banc avait le mode débutant *en
      marche* et *avec un binaire chargé*, jamais l'écran d'un nouveau
      venu : programme d'accueil dans l'éditeur, rien d'assemblé, aucun
      module allumé. On regardait partout sauf là.
- [x] **Et la première image a montré un défaut qu'aucune mesure ne
      cherchait** — la phrase de la console, « la sortie du programme
      s'affichera ici », **tranchée par le milieu** : neuf pixels peints
      sur quinze, dans quarante-trois scènes. Cause : la formule de
      hauteur comptait la hauteur d'un bouton (15) là où la rangée
      d'onglets mesure 26. Le nombre juste avait été essayé le
      28 octobre puis défait ; remis, les soixante-deux invariants du
      banc passent. **Un commentaire qui explique pourquoi un nombre
      faux doit rester faux mérite d'être remesuré.**
- [x] **L'invariant qui reste** — `aucune_phrase_ne_perd_de_hauteur_au_decoupage`,
      à sa **troisième** formulation : les deux premières accusaient
      des listes qui défilent à bon droit. Ce qui ne s'excuse jamais
      est une zone dont l'**unique** ligne est coupée — elle montrait
      tout ce qu'elle avait, et à moitié. Éprouvé en remettant le
      défaut : 43 scènes rougissent.
- [x] **La console est fermée sur une installation neuve** — elle
      occupait soixante-douze pixels pour annoncer qu'elle est vide.
      Elle paraît d'elle-même dès qu'elle a quelque chose à montrer :
      une erreur de nasm (ce qui existait) ou le premier octet écrit
      (ce qui est neuf). Les deux moitiés vont ensemble, et un contrôle
      les mesure toutes les deux — fermer sans ouvrir cacherait la
      sortie d'un programme qui écrit.
- [x] **Une coupe essayée puis écartée par la mesure** — les sept
      indicateurs de la barre de titre, candidat le plus évident.
      L'exercice `drapeaux`, au palier **Découverte**, dit : « avancez
      pas à pas et regardez les pastilles du bandeau changer ». Les
      retirer rendrait la phrase fausse pour ceux à qui elle s'adresse.
      Ce qui ressemble à du jargon sur une capture est parfois le sujet
      de la leçon.

---

## 0 untricies. Le seul rouge du dépôt est éteint (A 9.67)

`what_this_machine_runs_can_be_explained` lit les soixante binaires du
système et exige une fiche pour chaque mnémonique rencontré. Douze
manquaient depuis le 3 novembre — et **un rouge installé finit par faire
douter de toute la suite**.

- [x] **Cinq fiches neuves** — le `cmov` du coprocesseur (`fcmov*`),
      l'environnement du x87 (`fnstenv`/`fldenv`, et le `fnstenv` qui
      donne l'adresse du code courant sans `call` — l'ancêtre du
      `lea rax, [rel $]`), tout l'état flottant d'un bloc (`fxsave`),
      le mot de contrôle de SSE (`stmxcsr`, et ce que `-ffast-math` y
      pose), la mémoire transactionnelle (`xtest` et les trois autres).
- [x] **Deux fiches élargies** — « La pile fantôme » gagne le
      **changement** de pile (`rstorssp`, `setssbsy`, `wrss`), et
      « Comparer dans un masque » gagne les quarante-quatre **alias**
      d'AVX-512 qu'elle décrivait déjà sans les nommer : `vpcmpltub`
      était expliqué dans le texte et introuvable par la recherche.
- [x] **Les alias sont passés à nasm, un par un** — soixante noms
      possibles, quarante-quatre acceptés, seize refusés (les prédicats
      « toujours faux » et « toujours vrai »). On ne documente que ce
      qui s'écrit, puisque la liste des fiches nourrit aussi
      l'autocomplétion ; et la fiche dit ce qu'un désassembleur écrit
      pour les deux autres, avec un contrôle qui empêche la liste et
      cette phrase de diverger.
- [x] **Ce que les fiches affirment est vérifié** — exemples assemblés
      par nasm puis relus par `objdump` : `vpcmpub …, 1` et `vpcmpltub`
      donnent les mêmes octets, et objdump écrit l'alias.
- [x] **Et le rouge en CACHAIT un autre** — `cargo test --workspace`
      s'arrête au premier paquet qui échoue : tant que le contrôle des
      mnémoniques était rouge, `namsdec-re/tests/panneaux.rs` ne
      tournait plus, et il y était rouge aussi.
      `SOURCES_DES_PANNEAUX` ignorait `comparaison.rs` (ajouté le matin
      même) et `rapport.rs` (depuis P7) — or c'est le genre de liste
      dont un trou fait **passer** le contrôle au lieu de l'arrêter.
      Lancer la suite avec `--no-fail-fast` dit la vérité ; sans lui, on
      mesure jusqu'au premier échec.
- [x] **Cinq fiches avouent ce que namsdec ne simule pas** — exigé par
      `every_documented_example_assembles_and_runs`, qui assemble ET
      exécute l'exemple de chaque fiche : une fiche qui décrit une
      instruction que l'interpréteur refuse doit le dire, sans quoi elle
      promet une exécution qui n'aura pas lieu. Ce n'est pas une
      formalité — « une transaction suppose plusieurs cœurs qui se
      marchent dessus, et namsdec n'en a qu'un » apprend quelque chose
      sur TSX, et pas seulement sur namsdec.
- [x] **Un contrôle déterministe en plus de celui qui lit la machine** —
      les deux ne font pas le même travail. L'un **cherche** ce qu'on
      n'a pas prévu mais dépend du poste ; l'autre est le même partout
      et vérifie chaque nom **avec sa famille**, parce qu'un mnémonique
      rangé sous la mauvaise fiche serait expliqué par un texte qui
      parle d'autre chose.

---

## 0 tricies. Six exercices qui demandent de lire (A 9.66)

Le travail que les deux lots précédents désignaient comme leur suite.
Les cent huit exercices faisaient tous **écrire** du code ; douze
disaient où regarder, mais leur but s'atteignait en exécutant le
programme livré — rien ne vérifiait qu'on avait lu.

- [x] **Six étapes qui donnent un programme complet et posent une
      question** — `prevoir_tous_les_bits`, `prevoir_decalage`,
      `prevoir_somme`, `prevoir_octet_choisi`, `prevoir_pile`,
      `prevoir_signe`. On lit, on répond, puis on exécute : c'est
      « prévoir avant d'avancer » à l'échelle d'un programme entier.
- [x] **La réponse s'écrit comme on la pense** — « 32 », `0x20`, `20h`
      et `0b100000` sont la même réponse ; `-1` vaut
      `0xffffffffffffffff`. On compare des nombres, pas des chaînes.
      Refuser trois écritures sur quatre n'enseignerait que notre
      notation. Six contrôles sur la seule logique
      (`namsdec_exercises::reponse`), sans rien peindre.
- [x] **Rien d'écrit n'est pas une erreur** — le champ vide ne vaut pas
      « faux » : dire « faux » à quelqu'un qui réfléchit encore est le
      moment où l'on abandonne.
- [x] **Le verdict ne donne pas la réponse** — « pas celle-là », et
      rien de plus : la bonne s'obtient en exécutant, c'est-à-dire en
      faisant ce que l'exercice enseigne. Contrôle :
      `la_question_se_pose_dans_le_panneau_et_le_verdict_ne_vend_pas_la_réponse`,
      qui mesure les trois états — rien d'écrit, faux, juste.
- [x] **La réponse écrite dans la série est celle que la machine
      donne** — un contrôle assemble chacun de ces programmes par le
      vrai nasm, l'exécute jusqu'à l'arrêt et vérifie. Il relève toutes
      les fautes d'un coup et dit ce que la machine a répondu à la
      place. Une réponse écrite à la main est une affirmation ; passée
      à l'épreuve de l'émulateur, c'est un fait.
- [x] **Et l'exercice ne se note pas fait en appuyant sur
      « Exécuter »** — c'est le défaut que ce lot devait s'éviter : ces
      exercices n'ont pas de but, donc la règle des démonstrations les
      aurait notés faits au premier lancement, alors que leur programme
      **marche déjà**. Contrôle des trois états.
- [x] **Les deux champs vont ensemble, ou sont ignorés** — une question
      sans réponse resterait posée pour toujours ; une réponse sans
      question n'a rien à trancher. Retenu au niveau du **format**, donc
      pour toute série qu'on écrit soi-même.
- [x] **Trois des quatre questions annoncées** (5 novembre) — l'appel
      système (ce que `write` rend dans RAX, qui n'est plus le numéro
      qu'on y avait mis), la convention d'appel (RDI, RSI, RDX), le
      canari (rangé à côté du canari qu'on écrit). Neuf questions de
      lecture au total, cent dix-sept étapes.
- [ ] **La table de sauts** reste à poser : `re_table_sauts` la MONTRE
      déjà au désassemblage, et une question de lecture sur un saut
      indirect demande d'abord de vérifier ce que l'émulateur fait d'un
      `jmp [table + rbx*8]`.

**Trois contrôles ont fait tomber leur première version**, chaque fois
pour la même raison — ils trouvaient ce qu'ils cherchaient au mauvais
endroit : `lire_` était déjà pris par `lire_nombre` (qui lit une
entrée) ; chercher l'hexadécimal nu de quinze revient à chercher la
lettre « f » dans une phrase française ; et chercher la question dans ce
qui est peint la trouvait dans l'éditeur, parce que le programme de
l'exercice redit sa question en commentaire.

---

## 0 novovicies. Prévoir avant d'avancer (A 9.64)

Le second des deux travaux retenus pour le 4 novembre, et la direction
que P4 recommandait : **lire → prévoir → avancer → constater**. Tous
les ingrédients existaient ; le défaut n'était pas un manque, c'était
un **excès** — namsdec donnait la réponse avant qu'on ait essayé.

- [x] **La réponse attend le pas** — au niveau débutant, l'adresse et
      l'instruction restent à l'écran (c'est l'énoncé), et le
      pseudo-code en clair, le détail et le verdict d'un saut
      conditionnel attendent. La question posée à leur place :
      « Qu'est-ce que ça va changer ? ». Contrôle :
      `la_réponse_attend_le_pas_au_niveau_débutant`, qui mesure les
      **trois** états — prévision allumée, éteinte, et niveau avancé.
      Un contrôle qui ne regarderait que le premier passerait aussi si
      la réponse avait disparu pour tout le monde.
- [x] **Et « Dernier pas » la révèle à côté de ce qui a changé** —
      « Ce que ça disait : EAX = 1 », puis « RAX 0x0 -> 0x1 ». Les deux
      l'une sous l'autre sont la seule façon de savoir si l'on avait
      prévu juste. L'annonce est prise **avant** le pas, comme le reste
      du bloc : après, l'instruction n'est plus sous le pointeur.
- [x] **« Montrer » donne la réponse sans attendre** — un clic, pas de
      confirmation, aucun compte de ce qu'on a demandé. Il retient
      l'**adresse** et non un drapeau : au pas suivant, la question se
      repose. Contrôle : `montrer_donne_la_réponse_sans_attendre_le_pas`,
      qui clique le vrai bouton.
- [x] **Rien ne s'impose** — le réglage ne vaut qu'au niveau débutant,
      vit dans `settings.json`, et se **grise** sous le niveau avancé en
      disant pourquoi. Un exercice imposé à qui se sert de l'outil tous
      les jours serait une taxe.
- [x] **Réinitialiser repose la question** — l'adresse révélée
      appartenait au programme qu'on regardait, et le point d'entrée est
      presque toujours le même d'un programme à l'autre. Sans remise à
      zéro, la prévision montrait la réponse d'une instruction jamais
      vue. Contrôle : `réinitialiser_repose_la_question`, éprouvé en
      retirant la remise à zéro.
- [ ] **Rien ne fait encore LIRE** — les cent huit exercices font tous
      **écrire** du code. Un exercice de lecture (« voici trois
      instructions, que vaut RAX après ? ») est ce que ce lot rend
      possible, et le lot d'après.

**Trois contrôles déjà en place ont trouvé trois choses**, et c'est leur
raison d'être : le banc a refusé un `small_button` plat (invisible au
repos, donc inexistant), la liste des sources a réclamé le module neuf
(sans quoi tous les contrôles qui lisent le code l'auraient ignoré), et
`every_setting_that_is_written_is_read_back` **refuse de compiler** tant
qu'un réglage neuf n'est pas dans son instantané.

---

## 0 octovicies. L'analyse sans la fenêtre : `namsdec-cli` (A 9.63)

Le lot que P7 avait rendu bon marché, et le premier des deux retenus
pour le 4 novembre. Il y a maintenant **deux binaires** : `namsdec`
ouvre une fenêtre, `namsdec-cli` écrit un rapport.

- [x] **Un rapport Markdown, celui de l'application** — la prose
      d'ouverture puis les rubriques, par le même chemin que l'export
      de l'interface. `rapport_complet` ne désassemble qu'UNE fois,
      sans quoi un rapport complet valait deux analyses du même
      fichier. Contrôle : `le_rapport_markdown_porte_la_prose_et_les_panneaux`,
      qui lance le vrai binaire.
- [x] **Une sortie JSON stable** — c'est la condition de tout le
      reste : une sortie qui bougerait d'un appel à l'autre ferait du
      diff un bruit. Contrôle : `le_json_est_le_même_à_chaque_appel`.
- [x] **Comparer deux versions d'un binaire** — la vraie raison de
      faire ce lot d'abord. Contrôle :
      `deux_versions_se_comparent_par_leur_analyse`, qui modifie une
      chaîne **en place** dans le binaire d'épreuve et vérifie les deux
      sens : la nouvelle chaîne paraît, l'ancienne disparaît, et ce qui
      n'a pas changé se lit pareil. **La première écriture de ce
      contrôle s'était trompée** : elle ajoutait la chaîne à la fin du
      fichier, où l'analyse ne la voit pas — et elle a raison, le
      panneau des chaînes lit les sections déclarées, pas le fichier
      entier.
- [x] **Un fichier vide ne passe plus pour un fichier analysé** — le
      défaut que la mesure a trouvé, et le plus instructif du lot.
      L'analyse de zéro octet rend dix-huit rubriques qui disent toutes
      « 0 octet » : avec un titre, des tableaux et des sections, **ce
      rapport a l'air d'une analyse**, et il sortait avec un code 0. En
      CI, un artefact tronqué passait donc pour analysé. Contrôle :
      `le_fichier_vide_s_arrête_sur_son_propre_code`, avec son témoin
      contraire — sans lui, il passerait aussi si la commande échouait
      sur tout.
- [x] **Quatre codes de retour, pour une chaîne d'intégration** — `0`
      écrit, `1` ligne de commande fautive, `2` fichier illisible, `3`
      rien à analyser. Chacun est éprouvé.
- [x] **Un tuyau qui se ferme ne fait pas paniquer** —
      `namsdec-cli programme | head -1` est ordinaire, et `print!`
      panique dessus : « failed printing to stdout » au bout d'un pipe
      parfaitement normal. Contrôle : `un_tuyau_fermé_ne_fait_pas_paniquer`,
      éprouvé en remettant `print!`.
- [x] **Deux fichiers sur la ligne s'arrêtent** — le second écrasait le
      premier en silence. Et **une rubrique mal nommée se plaint** au
      lieu de rendre un rapport vide, en disant combien le fichier en
      porte.
- [x] **Les rubriques se listent avant de se choisir** —
      `--rubriques` rend « identifiant <tab> titre », de quoi un
      `cut -f1`. Le contrôle vérifie que ces identifiants sont bien
      ceux que `--rubrique` accepte, et qu'ils ne se traduisent pas
      quand les titres, eux, changent de langue.
- [x] **Installée à côté de l'application** — `install.sh` la pose dans
      le PATH (`--no-cli` pour s'en passer), `uninstall.sh` la retire.
      `default-members` ne bouge pas, pour que `cargo run` sache encore
      quoi lancer ; les deux paquets sont nommés à la compilation.
- [x] **La comparaison elle-même** (A 9.65) — `--contre` dit ce qui a
      changé dans l'analyse, rubrique par rubrique, et **compte ce qui
      n'a pas changé** : « quinze rubriques disent la même chose » est ce
      qui permet de croire aux quatre qui ont bougé. Une ligne qui a
      seulement changé de **place** n'est pas un changement — comparer
      deux suites plutôt que deux sacs aurait annoncé trois cents
      changements pour un. Le code de retour porte la réponse (`4`,
      comme `diff` rend `1`), et `-s protections` est la forme qui sert
      en CI. Douze contrôles, dont le témoin contraire — comparer un
      fichier avec lui-même — sans lequel les autres ne prouveraient
      rien, et **le cas pour lequel tout ceci existe** : un seul bit du
      segment `PT_GNU_STACK` rend la pile exécutable, et l'analyse en
      tire trois phrases là où un `diff` d'octets dirait « un octet a
      changé ».
- [x] **Un trou de dix-huit espaces dans un message d'erreur** —
      `cargo fmt` a recollé une continuation de chaîne en gardant son
      retrait. Le programme compilait, les contrôles de la commande
      passaient, et personne ne lit un message d'erreur avant de
      l'avoir provoqué. C'est
      `aucune_phrase_ne_porte_l_indentation_du_source`, un contrôle de
      l'**interface** qui relit tous les `.rs` du dépôt, qui l'a nommé
      par son numéro de ligne.

**Ce qu'elle ne fait pas, et c'est voulu :** elle repart des octets,
donc elle ne porte rien d'une session (pas de nom donné à une fonction,
pas de détour calculé), et elle ne lance aucun outil extérieur — les
rubriques objdump, radare2, Ghidra et RetDec restent muettes. C'est le
prix de la reproductibilité. Et elle **lie egui sans l'employer** :
`namsdec-re` en dépend toujours par `ReModule::ui`, aucune image n'est
rendue, et la commande tourne sans écran.

---

## 0 septies. L'éditeur binaire : une règle, et plus de pages (A 9.22)

- [x] **Une règle de colonnes** — `00 01 02 … 0f` au-dessus des octets,
      alignée au pixel. C'était le défaut principal, et le plus bête :
      sans elle, on comptait les colonnes avec le doigt pour savoir à
      quelle position un octet tombe. La grille de bits de la
      calculatrice porte ses index pour exactement cette raison.
      Contrôle : `the_column_ruler_lines_up_with_the_bytes`, qui compare
      les abscisses colonne par colonne, éprouvé en décalant la règle
      d'une chasse.
- [x] **Une géométrie calculée** — chaque octet était une étiquette dans
      une rangée horizontale, dont personne ne connaissait l'abscisse,
      pas même le code qui la posait. Les colonnes se calculent
      maintenant en caractères puis se multiplient par la chasse,
      mesurée sur huit caractères et divisée — sur un seul, elle revient
      arrondie, et l'erreur décale la règle d'un caractère entier au bout
      de la ligne.
- [x] **Les deux colonnes d'adresses portent leur nom** — « fichier » et
      « mémoire ». Elles étaient sans titre et parfois identiques chiffre
      pour chiffre ; rien ne disait laquelle on patche.
- [x] **Le fichier défile d'un bout à l'autre** — plus de pages de 256
      octets avec « ‹ » et « › » PAR-DESSUS une zone défilante : deux
      façons de se déplacer dans la même chose, dont l'une s'arrêtait
      tous les 256 octets sans raison. Seules les lignes visibles sont
      construites.
- [x] **8, 16 ou 32 octets par ligne** — au choix, et des puissances de
      deux seulement pour que chaque ligne commence à une adresse ronde.
      Le choix ne se déduit PAS de la largeur disponible : c'est la
      boucle que ce fichier a déjà payée une fois, sur l'inspecteur.
- [x] **Deux barres de défilement l'une sur l'autre** — la règle était
      allouée à la largeur du vidage, qui dépassait le panneau. Les
      chiffres d'adresse sont désormais taillés sur le fichier, et la
      largeur vue est **bornée** par ce qu'on offre : un plafond, jamais
      un plancher — du contenu qui prend moins que la place disponible
      ne peut pas faire grandir la fenêtre qui la lui offre. La règle est
      rognée et **suit** le défilement horizontal. Contrôle :
      `the_dump_stays_inside_the_width_it_is_given`, à trois largeurs.
- [x] **Une barre horizontale de trop** — l'ascenseur vertical d'une
      zone egui se pose **à l'intérieur** de sa largeur, pas à côté : la
      vue faisait 633 px pour un contenu de 642, et une barre
      horizontale paraissait sous un vidage qui tenait. Ni le banc ni la
      capture ne pouvaient trancher — un vidage trop large DOIT défiler.
      C'est l'application en marche qui a répondu, pour la deuxième fois
      en deux séances. Contrôle :
      `the_hex_dump_adds_no_horizontal_scrollbar`, aux trois tailles dont
      la plus petite acceptée, éprouvé en remettant le défaut.
- [x] **L'octet choisi dit où il est** — sa section (`.text`,
      `.rodata`, `.data`) et ce que cela veut dire : du code, des données
      modifiables, des données en lecture seule. Il fallait aller le
      demander à la carte du fichier, dans un autre panneau.
- [x] **Plusieurs octets d'un coup, et l'insertion** — faits le
      27 octobre 2026, voir la section 0 nonies.

---

## 0 sexvicies. La checklist d'avant-commit (A 9.50)

- [x] **La fenêtre des exercices entre au banc** — la plus longue liste
      que namsdec peigne, et aucun invariant ne l'avait regardée. Tous
      passent sur elle.
- [x] **Le programme d'accueil annonçait quatre-vingt-seize exercices**
      — la première phrase qu'on lit en ouvrant namsdec, fausse depuis
      qu'il y en a cent huit. Trouvée en regardant la capture, pas par
      une mesure.
- [x] **L'invariant compare l'annoncé au réel** — ce qui demandait
      d'écrire les nombres en toutes lettres, avec leurs pièges :
      « vingt et un », « quatre-vingts », « soixante-onze ».
- [x] **`no_preferences_page_grows_by_itself` ne lit plus la machine**
      (A 9.51) — deux hypothèses réfutées par la mesure : le dossier de
      données déplacé en cours de route ne change pas la hauteur, et la
      page des outils fait 761 px qu'ils soient tous présents ou tous
      absents. Deux sources de non-déterminisme retirées quand même,
      chacune pour une règle écrite : l'état des outils est semé (le
      contrôle lançait un processus par outil), et l'horloge est posée
      (il mesurait une convergence sans fixer le temps). **La cause
      exacte n'a pas été reproduite** — dix passages verts, dont trois
      sous charge lourde, ne valent pas une reproduction.

---

## 0 quinvicies. Le parcours n'apprenait qu'à écrire (A 9.49)

- [x] **Douze exercices de rétro-ingénierie** — 96 étapes, pas une de
      lecture, dans une application dont la moitié des panneaux sert à
      l'analyse. Réparties sur trois paliers : lire un fichier se
      commence avant de savoir ce qu'est une pile.
- [x] **Chacun dit où regarder** — c'est ce qui les distingue d'un
      exercice d'écriture, et ce que l'invariant mesure. Éprouvé en
      retirant les renvois d'un seul.
- [x] **`re_derouler` demande un geste** — la clé est à trouver, à la
      main ou en lisant RDX. Les onze autres se regardent tourner.
- [x] **`re_entropie` réécrit** — 2855 instructions pour une table de
      256 compteurs ; la contrainte « mille pas » du projet l'a
      attrapé. 483 sans table, et l'exercice tient en une page.
- [x] **README, manuel et comptes** — 108 étapes, 101 avec un but
      vérifiable. Le manuel calcule ses chiffres tout seul.

---

## 0 quatervicies. L'éditeur binaire : la place, la largeur, et une phrase en deux morceaux (A 9.46, 9.47, 9.48)

- [x] **Le vidage prend la place qu'on lui donne** — vingt-deux lignes
      écrites d'avance : 663 pixels offerts, 330 pris sur un grand
      écran ; 174 offerts, 330 pris sur un petit, avec un second
      ascenseur. La crainte écrite — « lire la place se reboucle » —
      était la bonne, mais ne s'applique pas ici : mesuré stable à 365,
      708 et 806 px sur trois écrans.
- [x] **« Vingt-deux » en montrait seize** — `show_rows` compte à
      `hauteur + item_spacing.y`, et l'écart n'était remis à zéro qu'à
      l'intérieur de la closure. La zone réservait vingt pixels par
      ligne pour en peindre quinze, d'où du blanc au bas de la zone.
- [x] **L'inspecteur ne pousse plus le vidage dehors** — quatre cents
      pixels pris au bas du panneau, et ses dernières lectures coupées
      par le bord. Borné à 250 px, défilant chez lui.
- [x] **Vingt-six espaces dans une phrase** — le retrait du source
      entré dans la chaîne, qui s'affichait en deux blocs décalés.
      Dix-neuf chaînes dans tout le projet, `upx_module` en tête.
      Invariant posé, qui distingue la prose de l'alignement.
- [x] **Les deux flèches de la recherche** pointaient l'une vers
      l'autre : précédent à gauche, suivant à droite.
- [x] **La largeur ne se perd plus** — `ReModule::pleine_largeur`, faux
      par défaut : la règle reste la largeur de lecture, et l'éditeur
      binaire dit qu'il en veut plus. Fenêtre agrandie, l'inspecteur
      passe en colonne à droite et le vidage garde ses cinquante-deux
      lignes (A 9.47).
- [x] **`float64` s'étalait sur trois lignes de zéros** — trouvé par la
      colonne, qui ne pardonne pas trois cents caractères. Notation
      scientifique pour ce qui ne se lit pas.
- [x] **Autant d'octets par ligne que la largeur permet** — 1346 px
      offerts, 606 pris dans l'onglet d'analyse. `auto` par défaut,
      trente-deux dès qu'ils tiennent ; 8/16/32 figent. Troisième fois
      la même crainte de rebouclage, troisième fois réfutée par la
      mesure (A 9.48).
- [x] **Une barre horizontale pour quatorze pixels** — egui retranche
      la barre deux fois, la zone ne la comptait qu'une. Invisible à
      seize octets, flagrante à trente-deux.
- [x] **Le banc sait isoler une fenêtre agrandie** (A 9.52) — le tri se
      fait sur l'ORDRE des formes : ce qu'une fenêtre peint vient après
      le rectangle de son cadre. Deux mesures ont corrigé l'idée en
      chemin — le fond de l'écran coïncide mal, et une fenêtre agrandie
      ne peint aucun fond (alpha zéro). La scène retirée est revenue, et
      c'est elle qui prouve le correctif.

---

## 0 tervicies. Le `int3` qu'un programme se pose à lui-même (A 9.45)

- [x] **La cinquième pièce du montage anti-débogage** — `rt_sigaction`,
      `clone`, `ptrace`, `tgkill` étaient en place ; `int3` manquait, et
      sans lui les quatre autres ne servaient à rien. La trace des
      appels système a montré l'ordre exact sur `LeetCrackme3`.
- [x] **Un vrai cadre de signal** — `struct rt_sigframe`, `ucontext` de
      968 octets, `sigcontext` de 256, les registres dans l'ordre du
      noyau, la zone rouge enjambée, l'alignement d'après `call`. Un
      gestionnaire qui réécrit `uc_mcontext.rip` pour sauter par-dessus
      un leurre est obéi — c'est pour cela qu'il existe, et c'est ce que
      le second contrôle mesure.
- [x] **`rt_sigreturn`** — le trampoline posé par la bibliothèque C
      restaure tout depuis le cadre, RIP compris. Nommé dans la table.
- [x] **La note de la barre de statut ne dit plus « pas encore »** —
      elle dit ce qui est vrai : aucun gestionnaire installé, et sur une
      vraie machine ce signal tuerait le programme.
- [x] **Mesuré** — `LeetCrackme3` passe d'un arrêt à sa bannière ASCII
      et l'attente d'une entrée.

---

## 0 duovicies. Un chemin pris pour un descripteur, et `open` qui répond enfin (A 9.43, 9.44)

- [x] **`newfstatat` décrivait la sortie standard** — l'appel sert à
      deux choses selon que son chemin est vide ou non ; namsdec
      répondait d'après le descripteur dans les deux cas. Un programme
      qui demandait `/etc/passwd` recevait `-EBADF`, ou s'entendait
      décrire le terminal. Le chemin est lu, et un chemin non vide vaut
      `-ENOENT` — la réponse de `fopen`, pour la même raison.
- [x] **`open` répond comme `fopen`** — `veil`, lié statiquement,
      s'arrêtait sur « l'appel système 257 (openat) » ; le même
      programme lié dynamiquement suivait sa branche d'erreur.
      `open`, `openat`, `stat`, `lstat`, `access` et `faccessat`
      rendent `-ENOENT`, et `-EROFS` sur `O_CREAT`. La console
      l'annonce une fois. Décidé par le propriétaire du projet le
      28 octobre 2026 ; README, manuel et deux contrôles mis d'accord
      avec le code — ils avaient eu raison de refuser l'essai qui les
      contournait. `veil` tourne.

---

## 0 unvicies. `lseek`, et une aide qui nommait quatre appels sur quarante-huit (A 9.42)

- [x] **L'appel système 8** — un binaire C++ s'arrêtait sur « l'appel
      système 8 » après avoir écrit sa première ligne. C'est `lseek` :
      « où en suis-je dans ce flux ? ». Il n'y a ni fichier ni position
      derrière l'entrée et la sortie de namsdec, mais c'est une réponse
      que la bibliothèque attend, pas un arrêt — `-ESPIPE`, ce que rend
      un noyau pour un tube ou un terminal, et `-EBADF` au-delà du
      descripteur 2, comme `fstat`.
- [x] **La table et l'interpréteur se suivent** — `namsdec-re::table`
      marque `lseek` simulé ; le contrôle croisé
      `what_the_table_calls_simulated_really_runs` éprouve les deux
      listes l'une contre l'autre, il n'y avait rien à ajouter.
- [x] **Un appel refusé se nomme** — « l'appel système 8 » laisse
      chercher ; « l'appel système 8 (lseek) » donne la page de manuel à
      lire. Le numéro reste devant : c'est lui qu'on a mis dans RAX.
- [x] **L'aide nommait quatre appels sur quarante-huit** — « Les appels
      simulés sont `read`, `write`, `brk` et `exit` » : vrai le jour où
      la phrase fut écrite, faux depuis quarante-quatre appels. Elle dit
      désormais la ligne de partage — ce qui sert à tourner seul contre
      ce qui touche au dehors — et un invariant interdit l'entre-deux :
      ou bien l'aide n'énumère rien, ou bien elle les cite tous.
- [x] **Le manuel disait la même chose** — « les appels système qui
      touchent au monde — `mmap`, `open`, `fork` » : `mmap` et `fork`
      sont simulés depuis longtemps. Corrigé, PDF régénéré.
- [x] **Aucune scène ne montrait l'application en erreur** — la barre de
      statut en rouge, qui est pourtant l'écran qu'on photographie pour
      signaler quelque chose, n'avait jamais été regardée remplie. Scène
      de banc « barre de statut / un appel système refusé » ajoutée ; le
      message tient en entier.

---

## 0 vicies. La phrase sous « Entrée : », et deux rangées sous le fond du terminal (A 9.41)

- [x] **La phrase du dessous n'était comptée nulle part** — « Ce programme
      ne lit rien pour l'instant… » se peignait sur trois pixels de ses
      quinze, au ras de la barre de statut, dès que la hauteur de la
      console avait été figée à la poignée. `console_input_estimate`
      énumère ce que la zone de saisie prend, rangée par rangée ; cette
      ligne-là manquait, et celle du code de sortie aussi.
- [x] **La mesure ne pouvait pas rattraper l'oubli** — un panneau qui
      déborde se fait RAMENER par egui à la place disponible, bas
      conservé : la hauteur relevée à l'image suivante est celle qu'on
      lui a allouée, jamais celle qu'il lui fallait. Le panneau restait
      court parce qu'il était court. La hauteur se prend désormais sur
      `ui.min_rect()` du contenu.
- [x] **Deux rangées présentes, mesurables, invisibles** — « Arguments »
      et l'en-tête de l'environnement disparaissaient sous le fond de la
      fenêtre de sortie : une `ScrollArea` réclame soixante-quatre pixels
      quand on ne lui dit rien, et les prend même s'il n'en reste que
      sept. Aucune mesure de découpage ne bouge dans ce cas — il faut
      regarder l'ORDRE des formes. `min_scrolled_height(0.0)` : la sortie
      a un ascenseur, la rangée où l'on tape n'en a pas.
- [x] **Les seize pixels du cadre de la sortie** ne figuraient ni dans la
      hauteur demandée ni dans le plancher : la console demandait moins
      que son propre plancher et arrivait coincée à son minimum, poignée
      à sens unique. Une constante partagée par les trois calculs.
- [x] **Contrôle** : `nothing_of_the_input_area_is_ever_cut_off_or_covered`,
      vingt-quatre scènes, découpage et recouvrement, éprouvé en
      remettant chacun des deux défauts. Et une scène de banc de plus —
      « console / binaire chargé, hauteur figée » —, celle que le défaut
      occupait et qu'aucune capture ne montrait.

---

## 0 novodecies. Trois idiomes d'onglets, et cent sept captures qui ne montraient pas l'application (A 9.40)

- [x] **Les captures se rendaient SANS LE THÈME** — et c'est la trouvaille
      qui invalidait tout le reste. `captures_de_toutes_les_scenes`
      construisait son `Harness` sur un `NamsdecApp::default()` sans
      jamais appeler `apply_theme` : les 107 images montraient les
      `Visuals` d'egui — fond gris 27, boutons gris 60, espacements par
      défaut — et non namsdec. Le reste du banc passe par `chassis`, qui
      l'appelle depuis toujours ; ce test-là bâtissait son contexte à la
      main. Deux revues sur captures avaient jugé cette interface-là, qui
      n'existe nulle part. Le test pose maintenant le thème et **vérifie**
      que `panel_fill` est celui du thème, parce qu'un `build_ui` qui
      l'oublie se réécrit en une ligne.
- [x] **Trois idiomes d'onglets au lieu de deux** — `rangée_d_onglets`
      en prévenait en commentaire, et son inventaire ne les comptait pas :
      les préférences, la console et les familles de la calculatrice
      étaient des `selectable_label`, c'est-à-dire le bleu PLEIN de la
      sélection. Dans la fenêtre des préférences, l'onglet « Général » et
      le bouton « OK » se peignaient donc pareil — un sélecteur et une
      commande qu'on ne distingue plus. Les trois rangées passent par
      `rangée_d_onglets` ; le bleu plein reste à ce qui agit.
      Contrôle : `a_row_of_tabs_underlines_its_active_tab_and_never_fills_it`,
      sur les quatre rangées, et il demande **les deux moitiés** — pas de
      fond, et la PRÉSENCE du trait. Éprouvé en remettant le
      `selectable_value` : il nomme le rectangle fautif.
- [x] **Les sept indicateurs : un creux, et non un contour** — le trait
      qui cerclait CF…OF était de la couleur de leur propre texte : sept
      traits clairs dans la barre de titre, aussi marqués que les sept
      valeurs qu'ils encadrent, lesquelles sont à zéro la plupart du
      temps. Ils sont creusés dans le fond de l'éditeur ; celui qui est
      levé teinte son creux, et se voit à la tache avant qu'on ait lu le
      chiffre. Le texte estompé y gagne du contraste au passage — 4,52
      contre 4,16 sur la barre nue. Contrôle :
      `a_raised_flag_shows_as_a_patch_and_not_as_an_outline`, qui exige
      l'absence du contour **et** deux fonds différents selon l'état.
- [x] **Un bouton encadré commence par une capitale** — « sommaire » était
      le seul libellé de l'application à s'ouvrir en minuscule, à côté de
      « Copier » dans la même fenêtre. Contrôle :
      `a_framed_button_opens_its_label_with_a_capital`, mesuré sur le
      RENDU (un cadre de la couleur estompée EST un bouton encadré) et
      non sur `i18n` : un libellé nouveau est jugé sans qu'on ait à
      l'inscrire nulle part. Deux faux positifs écartés et nommés — les
      blocs du graphe de déroulement portent le même cadre, mais leur
      texte est en chasse fixe et ils en portent plusieurs.
- [x] **TREIZIÈME FAÇON DONT UN CONTRÔLE MENT : le fondu d'une fenêtre.**
      Rencontrée deux fois dans la même journée. egui fait APPARAÎTRE une
      `Area` en fondu, et tant qu'il dure, tout ce qu'elle peint est
      teinté de son opacité : le trait d'un onglet sort en `#1B4A7F` au
      lieu de `#3794FF`, le cadre d'un bouton en `dim` à moitié. Un
      relevé qui compare des couleurs EXACTES sur quatre ou six images ne
      reconnaît alors **rien** dans **aucune** fenêtre, et passe au vert
      en ne regardant rien. Douze images. Les trois autres contrôles du
      dépôt qui comparent une couleur exacte ont été relus : ils exigent
      tous un compte non nul, donc ils ne peuvent pas mentir ainsi.
- [x] **Une correction défaite SUR MESURE** — `console_wanted_height`
      compte sa rangée d'onglets comme la hauteur du texte d'un bouton
      (15) quand la rangée en fait 26. La corriger paraissait évident ;
      avec `ONGLET_HAUTEUR + 28`, une ligne de la sortie se fait rogner
      par le bas, vu sur la capture de « console / Output (en marche) ».
      Ce qui ressemble à une sous-estimation est ce qui tient : réclamer
      la hauteur exacte fait déborder le panneau. Le nombre est resté, et
      la mesure est écrite à côté de lui.
- [x] **Dix cartes, dix largeurs** — la page Modules des préférences
      empilait des `module_card` dont chacune épousait son texte : 355 px
      pour « nasm détecté — NASM version 3.02 compiled on Jul 14 2026 »
      à côté de 175, et un bord droit en dents de scie sur toute la page.
      Une largeur commune, relevée UNE FOIS avant la première carte et
      DITE à chacune — un groupe qui se règle sur la place que son propre
      contenu laisse est la boucle de mesure que ce dépôt a déjà payée.
      Contrôle : `the_cards_of_a_list_share_one_width`, éprouvé en
      retirant le `set_width` : il rend les neuf largeurs.
- Ce qui a été REGARDÉ et laissé : le trait sous les textes cliquables
  (choix du 20 octobre, découvrabilité), les touches bleues de la
  calculatrice (deux familles franches, décidées contre des mélanges
  timides), le 9 pt des états vides (dans le jeu de tailles déclaré).
  Et une observation qui n'est pas un défaut de forme : quatre boutons
  de la barre d'outils — « Step Into », « Step Over », « Step Back »,
  « Stop » — sont en anglais dans l'interface française, entre
  « Assembler » et « Réinitialiser ». C'est un choix de vocabulaire, pas
  une incohérence de peinture, et il revient à qui écrit le manuel.

---

## 0 octodecies. Un panneau qui déborde vole la place de son voisin (A 9.37)

- [x] **Les onglets de la console entrent au banc** — ils étaient à
      l'écran en permanence et dans aucune scène : ni mesurés, ni jamais
      vus. Trouvé par un recensement des états vides — on cherchait des
      panneaux mal présentés, on a trouvé des panneaux qu'on ne
      regardait pas du tout.
- [x] **Et la première scène a rougi** — à 800×520, le bandeau du
      panneau latéral peignait « Détail » par-dessus la rangée
      d'onglets.
      LA CAUSE EST GÉNÉRALE, et elle est dans egui : quand le contenu
      d'un panneau dépasse la place qu'on lui laisse, son rectangle
      GLISSE, et le parent n'avance son curseur que jusqu'à la partie
      restée visible (`cursor.max = visible_outer_rect.min`). Le voisin
      croit donc avoir une hauteur de ligne de plus qu'il n'en a, la
      remplit, et peint dessus. Mesuré : le panneau du bas occupe
      84,4 px, le parent n'en défalque que 69,8.
      Le remède : la part du bas REND ce qu'elle a demandé, et c'est ce
      chiffre-là qui borne le bandeau — `ui.available_height()` n'étant
      pas fiable après qu'un panneau s'est posé.
- [x] **Une liste qui tenait se repliait quand même** — « Chaînes
      lisibles » annonçait treize chaînes et en montrait onze et demie,
      avec deux cent cinquante pixels de vide dessous : on faisait
      défiler treize lignes dans un hublot, devant un écran libre.
      `max_height(220.0)` en dur, et le vidage du fichier avait le même
      avec 320. La place offerte, désormais, avec l'ancien chiffre pour
      plancher — ces modules se montrent aussi dans des cadres courts.
      Contrôle : `une_liste_qui_tient_ne_cache_aucune_ligne`, qui
      compare les adresses PEINTES au nombre que le module annonce
      lui-même — on ne recopie pas le chiffre, il dépend du binaire.
      Éprouvé : 10 sur 13.
      Le banc vérifiait qu'une liste ne déborde pas ; jamais qu'elle
      occupe ce qu'on lui donne.
- [x] **Une hypothèse essayée et rejetée** — le vide entre « Retrouver
      un décalage » et « OK », dans Motif cyclique, ressemblait au
      défaut que la fiche d'instruction a eu (« cent trois pixels de
      vide »). Retirer la hauteur imposée REND LA FENÊTRE PLUS HAUTE —
      560×420 au lieu de 560×400 — et le trou passe de 80 à 90 px.
      La différence : la fiche borne ses zones défilantes et se rétracte
      avec elles ; celle-ci n'en a pas, et son bouton est épinglé en bas
      par un panneau. Le vide n'est pas de la hauteur en trop, c'est la
      place que garde la réponse à venir. Mesure écrite dans le code,
      pour que personne ne refasse l'essai.
- [x] **Quatre autres hauteurs figées, toutes légitimes** —
      prévisualisation du rapport de bogue, deux journaux d'installation,
      carte mémoire. Chacune partage son volet avec des voisines, là où
      les deux corrigées l'occupaient seules. Recensement fait, aucune
      autre à changer.
- [x] **Un commentaire écrit deux fois** — le même paragraphe de douze
      lignes, dans `analysis_body_ui`, avec deux fins différentes.
      Retiré.
- [x] **« Les deux commandes lancées » avant tout assemblage** — le
      panneau des diagnostics les montre dès l'ouverture, et c'est tout
      son intérêt : « et si je veux le faire sans namsdec ? » est la
      bonne question. Il les annonçait au passé alors que rien n'avait
      tourné. Au futur tant que rien n'est assemblé.
      Vu sur une capture, et seulement parce que les onglets de la
      console venaient d'entrer au banc : l'écran existait, personne ne
      l'avait jamais regardé sans programme assemblé.
- [x] **L'éditeur peignait quatorze pixels sous sa zone** — troisième
      maillon de la même chaîne, et le plus simple une fois nommé.
      Relevé à 800×520, console ouverte sur Profil : zone de travail
      [0, 60]–[420, 205,8], légende de chaleur de 205,8 à 232,8, et le
      numéro de ligne « 4 » peint de 205,0 à 220,0.
      **Le compte des panneaux était juste** : ce n'était pas le défaut
      ci-dessus. L'indice était dans le code, à six lignes de là —
      toutes les vues de la rangée sont dans un `CentralPanel` « qui
      borne ce qu'il contient et le rogne », SAUF l'éditeur, parce
      qu'il « EST la zone centrale ». Vrai de sa place, faux de son
      découpage : un `Ui` ordinaire garde celui de son parent, et rien
      n'empêchait ce qu'on y peint de descendre sous le curseur.
      Un panneau sans cadre pour lui aussi, et la scène
      « console / Profile » rentre au banc.

---

## 0 septdecies. La décompilation se présente enfin (A 9.36)

- [x] **La vue de décompilation ouvrait sur une notice de
      configuration** — deux blocs de JSON en chasse fixe, collés en
      haut à gauche, sans un mot sur ce qu'est un décompilateur ni sur
      ce qu'il faut faire. Le même défaut que l'onglet RE, vu sur la
      même planche de captures, et dans l'axe que namsdec revendique.
      Elle se présente maintenant — ce que fait un décompilateur, que
      namsdec n'en embarque aucun, et où l'on branche le sien — avec le
      bouton qui mène à Préférences › Outils. La notice reste, sous son
      propre titre : on branche son outil avec, et c'est une bonne chose
      que namsdec l'enseigne. Elle n'est plus la première chose qu'on
      lit.
- [x] **Un état vide s'étirait sur toute la largeur de l'écran** — il
      prenait la place offerte, quelle qu'elle soit. Trois cents pixels
      dans le panneau : juste. Mille deux cents dans une vue de la
      rangée : la phrase courait d'un bord à l'autre sur une seule
      ligne. Borné à 560 px et centré dedans ; les panneaux étroits ne
      bougent pas, puisqu'ils offrent moins. Les dix-neuf états vides en
      profitent d'un coup.
      Contrôle : `un_etat_vide_tient_dans_une_largeur_de_lecture`,
      éprouvé en retirant la borne — 1 061 px.

**Deux pistes examinées et écartées**, parce qu'un faux positif coûte
plus cher qu'il n'informe :

- le vidage mémoire du panneau perd sa colonne de texte au bord droit.
  C'est **voulu** : `ScrollArea::both`, et le commentaire date du
  20 octobre — « seize octets et leur transcription ne doivent pas se
  replier à la largeur du panneau ». La décision est prise, elle est
  écrite, on ne la défait pas au vu d'une image.
- « NASM 3.02 » paraissait déborder la fenêtre de 53 px au relevé. Non :
  c'est la largeur de la GALLEY dans une rangée alignée à droite, pas
  celle de l'encre. L'image montre le texte entier. Un relevé qui
  mesure la galley au lieu de l'encre accuse à tort.

---

## 0 sexdecies. Cinq défauts que seule l'image pouvait montrer (A 9.35)

Première revue menée sur les captures, maintenant qu'elles existent.
Cinq écarts, et **aucun des cinq n'était atteignable par une mesure** :
tous relèvent de ce qu'on n'avait pas pensé à demander.

- [x] **Le désassemblage cachait le début du programme** — treize
      instructions, neuf peintes, six cents pixels de vide dessous.
      `_start` et les trois suivantes poussées hors du cadre, sur un
      programme qui tient trois fois dans la vue. egui ne ramène pas
      dans ses bornes une position de défilement posée à la main :
      « trois lignes de contexte au-dessus » est juste sur un long
      listing et absurde sur un court. Borné au contenu — quand tout
      tient, rien ne défile, et la demi-ligne coupée en haut disparaît
      avec. Même remède pour l'onglet Octets, qui avait le même calcul.
      Contrôle : `un_listing_qui_tient_en_entier_ne_cache_pas_son_debut`,
      qui compte les ADRESSES peintes — la seule mesure qui réponde à
      « voit-on le programme ? ». Éprouvé : 9 sur 13.
- [x] **L'onglet RE n'avait pas d'état vide** — le seul du panneau. Un
      bouton gris, et pas un mot, dans l'onglet qui porte tout le
      travail de rétro-ingénierie : celui qu'un nouveau venu ouvre par
      curiosité. Il dit maintenant ce qu'il attend, avec son détail à
      lui — les six autres attendent un programme assemblé, celui-ci
      attend un FICHIER, et il sert aussi bien pour un binaire qu'on n'a
      pas écrit.
- [x] **Quatre comptes à un portaient un pluriel** — « 1 fonctions
      retrouvées », « 1 boucles », « 1 étiquettes », « 1 adresses de
      données ». Le cas à UN est celui qu'on ne voit jamais en
      développant : on essaie toujours sur un binaire qui en a douze.
      `ModuleContext::compte` accorde, écrit une fois.
      Contrôle : `aucun_compte_a_un_ne_porte_un_pluriel`, qui balaie
      toutes les scènes du banc et lit ce qui est peint, avec la liste
      des mots invariables français.
- [x] **« 1 passages par ligne »** dans la légende de chaleur, sur le
      programme d'accueil. Le relevé général ne pouvait pas l'attraper :
      le chiffre et l'unité sont deux étiquettes peintes séparément, et
      il ne lit qu'un texte à la fois. L'unité s'accorde maintenant à
      son dernier repère.
- [x] **La fiche d'instruction s'ouvrait sur la barre d'état** — ancrée
      à 24 px du bas, la barre en fait 28 : « Lig 1, Col 1 » et l'état
      de nasm disparaissaient dessous, au moment précis où l'on regarde
      une instruction. `hauteur_des_barres_du_bas` est désormais écrite
      une fois et sert aux deux surfaces qui choisissent leur place en
      bas. Contrôle : `rien_ne_s_ouvre_par_dessus_la_barre_d_etat`.

**Et un faux positif, qui vaut d'être dit** : le détail d'un état vide
est peint en 9 points là où la prose voisine est en 13, et cela saute
aux yeux sur l'image. Ce n'est pas un écart — `every_text_size_belongs_
to_a_declared_set` déclare cette taille et sa raison. La capture pose
des questions ; c'est au code de répondre, et ici il répondait.

---

## 0 quindecies. Voir l'interface sans écran (A 9.34)

- [x] **La capture ne dépendait plus d'un bureau** — sous Wayland,
      XTEST n'atteint pas la fenêtre XWayland et GNOME ne donne pas le
      focus à une fenêtre qu'on vient de lancer : tout ce qui n'apparaît
      qu'au clavier était hors d'atteinte de l'œil. Le deuxième des
      trois lieux de mesure manquait, et pour une raison qui ne se
      répare pas côté namsdec.
      `captures_de_toutes_les_scenes` rend chaque scène du banc en PNG
      par `egui_kittest` et wgpu sur **lavapipe** — Vulkan logiciel.
      Ni écran, ni carte graphique, ni droits, et le même résultat
      partout : **101 images en 12 secondes**. Ignoré par défaut, en
      `dev-dependencies` seulement.
- [x] **Et cela a payé tout de suite** — le panneau des suites rangeait
      `/` entre `s` et `u`, parce que le tri portait sur `Key::name()`,
      où la barre oblique s'appelle « Slash ». Aucune mesure ne l'avait
      demandé ; l'image l'a montré en une seconde. Trié sur ce qui
      s'affiche, retenu par
      `les_suites_se_rangent_dans_l_ordre_ou_elles_s_ecrivent`.

---

## 0 quaterdecies. « Touche suivante attendue » — lesquelles ? (A 9.33)

- [x] **Le programme dit enfin quoi taper** — une séquence armée, une
      hésitation de quatre dixièmes de seconde, et un panneau paraît en
      bas à gauche : les touches qui restent, et l'action au bout de
      chacune. La barre d'état disait « touche suivante attendue… » sans
      dire lesquelles, et le seul recours était d'abandonner la séquence
      qu'on était en train d'apprendre pour ouvrir Préférences.
      Lu dans le jeu ACTIF, et pas de noms de familles : une seconde
      table mentirait dès le premier raccourci déplacé.
      Quatre contrôles. La partie pure — `continuations` et la règle du
      délai — se lit sans écran ni horloge. Le reste est de bout en
      bout : la frappe entre par les évènements, le dispatcher s'arme,
      et l'on mesure ce qui est peint.
- [x] **Une fenêtre, et non une surface nue** — egui ne découpe pas le
      contenu d'une `Area` : son découpage reste celui de l'écran. Le
      banc, qui isole une fenêtre en comparant le découpage de ses
      formes à son cadre, ne voyait donc RIEN du panneau — « n'écrit
      presque rien », alors qu'il écrivait vingt lignes. Une fenêtre se
      découpe à elle-même, et redevient mesurable.
      C'est le banc qui l'a dit, en refusant la scène qu'on venait de
      lui donner.
- [x] **Une grille, et non deux étiquettes à la suite** — les restes de
      séquence n'ont pas tous la même longueur, et un raccourci déplacé
      suffit à en laisser une d'une seule touche. Les noms d'actions
      partaient alors chacun où finissait sa touche : 15,66 px d'écart,
      mesurés. Une colonne qui ondule ne se descend pas des yeux, et
      c'est le seul service que ce panneau rende.
- [x] **Il ne recouvre jamais les barres du bas** — la hauteur de la
      barre de statut et de la console est relue à l'image. Un décalage
      en dur suffisait tant que la console restait fermée ; éprouvé en
      le remettant, le panneau descend 10 px sous le haut de la barre.

---

## 0 terdecies. La palette, les raccourcis, et une notation de trop (A 9.28 à 9.30)

- [x] **Le seul raccourci qu'on ne pouvait pas déplacer** — `Ctrl+Maj+P`,
      la palette de commandes, vivait dans un `consume_shortcut` posé au
      milieu de la boucle d'affichage. Il marchait ; c'est tout ce qui
      ENTOURE un raccourci dans ce programme qui lui manquait — absent
      des deux jeux, du tableau de Préférences, du fichier exporté, du
      chapitre du manuel qui lit `keybindings.rs`, et non déplaçable.
      Le seul geste qu'on ne pouvait pas régler était celui qui sert à
      trouver tous les autres.
      `Action::ToggleCommandPalette` le rejoint, avec sa bascule — la
      seconde du programme après la calculatrice, et pour la même
      raison. La frappe ne bouge pas : la rendre réglable ne doit pas
      faire payer l'amélioration à qui la connaît déjà.
      Contrôle : `aucun_raccourci_n_est_pose_en_dur_hors_de_la_table`,
      qui balaye tout le greffon et ne tolère une frappe fabriquée que
      dans `keybindings.rs` et `settings.rs`. Il ne parle pas de la
      palette, et c'est ce qui lui donne sa valeur. Éprouvé en remettant
      la ligne d'avant.
- [x] **La palette apprend le raccourci qu'on est venu y chercher** —
      chaque ligne porte sa frappe à droite, lue en direct dans le jeu
      actif. C'est l'endroit où l'on cherche une action dont on ne
      connaît pas la frappe : c'était donc le seul endroit où
      l'apprendre, et il ne disait rien.
      Deux contrôles, dont le second a été écrit deux fois : sa première
      version mesurait l'encre et passait au vert avec le défaut remis.
      Ce qui déborde n'est pas rogné — c'est la FENÊTRE qui cède, et la
      palette passe de 520 à 630 px selon le fichier ouvert. Le relevé
      porte donc sur le cadre, à programme nu puis avec une étiquette
      démesurée. Éprouvé : 630,2 contre 520.
- [x] **Deux notations pour la même frappe** — le manuel écrivait
      « Ctrl+Maj+B » et « Espace puis S puis I », l'écran
      « Ctrl+Shift+B » et « Space S I ». Une seule table maintenant,
      `keybindings::key_name`, et c'est le guide qui s'aligne sur
      l'écran pour le séparateur. Les lettres d'une séquence passent en
      minuscule : une séquence ignore les modificateurs.
      Le contrôle confronte les deux tables et a trouvé un écart du
      premier coup : egui rend U+2212 pour la touche « - », le clavier
      porte le trait d'union.
- [x] **Un chiffre qui mentait** — « les trente raccourcis » dans
      l'infobulle de l'export, quand il y en avait trente-deux. Compté
      sur `Action::ALL`, et un contrôle refuse qu'un nombre de
      raccourcis s'écrive en mots dans une chaîne de langue.
- [x] **Préférences a enfin une touche** — `Ctrl+,` dans le jeu
      ordinaire, `Espace v r` dans l'autre. C'était le dernier geste
      courant sans raccourci, et c'est la fenêtre où l'on règle les
      raccourcis. Au passage, `PaletteCommand::ShowPreferences`
      disparaît : deux routes vers la même chose, dont une seule
      figurait au tableau.
- [x] **Deux des quatre bascules d'affichage** — la console (`Ctrl+J`,
      `Espace v c`) et le panneau latéral (`Ctrl+Alt+B`, `Espace v p`).
      Pas les quatre : on règle la barre d'outils et la barre de statut
      une fois, et leur donner une frappe, ce serait la retirer à
      quelque chose qu'on fait tous les jours.
      La question était de dessin — `ui.checkbox` n'a pas de
      `shortcut_text`, et une infobulle en aurait fait les seules
      entrées de menu dont le raccourci ne se lise pas sans survoler.
      La réponse est dans les atomes d'egui 0.36 : un `Atom::grow` entre
      le libellé et la frappe.
      Deux choses à mesurer, et une seule saute aux yeux. Ce qu'on
      voit : les frappes forment une COLONNE — « à droite du libellé »
      ne suffit pas, sans l'atome chacune se colle à son texte et la
      colonne se disloque de 101 px. Ce qu'on ne voit pas : un enfant
      qui réclame toute la place dans un parent qui se dimensionne sur
      ses enfants est la boucle la plus connue d'egui ; vingt images,
      les huit dernières au même pixel.
      Contrôle : `les_frappes_du_menu_affichage_s_alignent_sans_elargir_le_menu`,
      éprouvé en retirant l'atome. Et
      `les_bascules_d_affichage_vont_et_reviennent` : une bascule qui ne
      rebascule pas est une porte à sens unique.
      Au passage, « Affichage: Console » devient « Afficher/masquer la
      console » dans la palette — elle était seule de son groupe à ne
      pas le dire ainsi — et deux `PaletteCommand` de plus disparaissent
      au profit des actions.

---

## 0 duodecies. Le panneau du détour se repliait sur 327 px (A 9.27)

Trouvé en écrivant l'invariant de la section précédente, et c'est l'ordre
habituel : le relevé cherche une chose et en montre une autre.

- [x] **Aucun refus ne se fait couper par le bord** — les neuf variantes
      de `DetourRefusé`, à 220, 420 et 900 px. Un refus coupé est pire
      qu'un refus muet : on lit la moitié d'une explication et l'on croit
      avoir compris. Contrôle :
      `no_refusal_is_cut_by_the_edge_of_the_panel`.
- [x] **La rangée de la place libre débordait de 107 px**, et un enfant
      qui déborde agrandit le `max_rect` de son parent : tout ce qui
      suivait se repliait sur 327 px là où le découpage restait à 220.
      Trois causes empilées, et aucune ne suffisait seule — `.width(230.0)`
      en dur devenu un plafond, `.truncate()` parce qu'une liste
      déroulante prend la largeur de son texte, et l'étiquette passée
      **au-dessus** parce que dans une rangée repliable
      `available_width` rend la largeur de la rangée et non ce qu'il en
      reste. Diagnostiqué par l'application en marche, pour la troisième
      séance de suite.
- [x] **Le texte tronqué se lit encore en entier** — survol sur la liste
      déroulante : l'adresse, la taille et la section du trou où le code
      atterrira ne s'obtiennent nulle part ailleurs.
- [x] **Et le contrôle a d'abord passé pour la mauvaise raison** — monté
      sans place libre, le panneau rendait la main avant de peindre le
      refus. Il exige maintenant d'avoir **vu** son sujet avant de le
      mesurer.

---

## 0 undecies. Un second détour écrasait le premier (A 9.26)

- [x] **`plan_detour` refuse d'écrire sur un correctif posé** — le plan se
      calcule sur le fichier déjà corrigé, mais des octets ne disent pas
      d'où ils viennent : le `jmp` d'un premier détour se décode comme
      n'importe quel `jmp`, se relocalise très bien, et le second plan
      sortait sans se plaindre. La fonction reçoit désormais la liste des
      correctifs, et refuse deux fois — à l'endroit visé avant de
      décoder, puis sur les deux écritures en entier. Contrôles :
      `a_second_detour_does_not_quietly_overwrite_the_first`,
      `overlapping_a_patch_by_one_byte_is_refused_too`,
      `writing_over_the_body_of_a_detour_is_refused`, tous éprouvés en
      éteignant la garde.
- [x] **Et un détour ailleurs passe encore** —
      `a_detour_elsewhere_is_still_allowed`. C'est lui qui donne leur
      sens aux trois autres : sans lui, ils passeraient au vert avec un
      refus inconditionnel, c'est-à-dire avec la fonctionnalité éteinte.
- [x] **Mesuré sur l'émulateur, et pire que prévu** — deux détours
      croisés ne privent pas seulement le premier de son effet : le
      programme ne s'arrête **plus du tout**, son `hlt` ayant été
      recouvert. Contrôle :
      `two_crossing_detours_break_the_program_and_are_refused`, qui
      montre le défaut puis la garde dans le même corps. Le site visé est
      le `nop` de complément et non le milieu du saut — celui-là est
      refusé par hasard, `0xFB 0x07` ne se décodant pas.
- [x] **La garde est rejouée là où les octets partent** — `write_detour`.
      Non pour réparer un trou de l'interface, qui jette son plan à
      chaque rechargement, mais parce que `ReRequest::PatchDetour` peut
      venir d'ailleurs. Mesuré sans elle : le second appel répond
      « Octets changés, binaire rechargé ». Contrôle :
      `posing_the_same_detour_twice_changes_nothing_the_second_time`.
- [x] **`ReContext` porte les correctifs posés** — un panneau qui
      s'apprête à écrire a besoin de savoir ce qui est déjà écrit. La
      garde se désarmant d'un caractère (`&[]` au lieu de
      `ctx.patches`), un contrôle de branchement la retient :
      `the_panel_refuses_a_site_that_a_patch_already_holds`, qui montre
      d'abord que le plan SORT quand rien n'est posé.
- [x] **Ce qui reste de A 4.17 : TRANCHÉ, et contre** — relocaliser un
      saut court en l'élargissant. La case laissait croire à une tâche
      en attente ; le relevé des 3 371 923 sites du corpus avait déjà
      décidé l'inverse, et il faut le lire avant de s'y remettre. Le
      saut court vaut **5,6 %** des refus ; la PLACE LIBRE en vaut
      **28,1 %**, cinq fois plus, et quatre binaires sur onze n'ont
      aucun trou exécutable. Élargir un `jmp rel8` en rel32 changerait
      sa longueur dans la zone recouverte, c'est-à-dire ce qui fait la
      sûreté du détour — on paierait cher le plus petit des deux
      gisements.
- [ ] **La vraie suite du détour : FABRIQUER de la place** — étendre un
      segment, ajouter une section. C'est là que sont les 28,1 %, et
      c'est un travail sur le fichier ELF, non sur les instructions.

---

## 0 decies. L'épingle du dock ne tenait pas (A 9.25)

- [x] **`install.sh` vérifie que l'épinglage a tenu** — GNOME Shell élague
      de lui-même les favoris dont il ne trouve pas le `.desktop`. Juste
      après une réinstallation, le lanceur vient d'être réécrit et le
      cache des applications n'est pas forcément relu : le dash retirait
      l'entrée qu'on venait de poser, sans rien dire — et l'on concluait
      que l'installation avait désinstallé l'icône. On relit, on laisse
      au bureau le temps de voir le fichier, on repose une fois, et si
      l'épingle ne tient toujours pas, **on le dit** plutôt que
      d'annoncer « épinglé ».
- [x] **La garde d'essai à blanc est là où l'écriture se fait** — elle
      vivait vingt lignes plus haut, dans un `elif` qui englobait tout :
      juste, et illisible. `a_dry_run_writes_nothing` a attrapé la
      première version de ce correctif, qui aurait écrit dans le dock
      malgré `--dry-run`.

---

## 0 octies. Les exercices : une première étape qu'on pouvait finir (A 9.23)

- [x] **Un exercice SANS but se termine quand on l'a regardé tourner** —
      sept des quatre-vingt-seize sont des démonstrations, et le tout
      premier du parcours en est une. `exercise_goal` rend `None` pour
      eux, et `note_goal_reached` sortait là sans rien noter : on allait
      jusqu'au `hlt`, la pastille restait creuse, « en cours » ne partait
      jamais. Contrôle :
      `a_demonstration_is_finished_once_it_has_been_watched_through`,
      éprouvé en remettant le défaut — il nomme `namsdec/registres`,
      c'est-à-dire l'étape 1.
- [x] **Le panneau dit que c'en est une** — il se taisait, et l'on
      cherchait le but qu'on venait de lire dans l'énoncé. Contrôle :
      `a_demonstration_says_that_it_is_one`.
- [x] **Une barre d'avancement par palier, et une pour le parcours** — un
      chiffre se lit, une barre se voit. Pas de flamme, pas de série de
      jours, pas de points : la seule distinction reste l'anneau du
      budget, qui se gagne en réécrivant.

---

## 0 nonies. L'éditeur binaire : plusieurs octets, et l'insertion (A 9.24)

- [x] **`patch::splice`** — la forme générale dont `apply` est le cas
      sûr : remplacer, insérer, supprimer. Le commentaire dit ce que
      chaque cas coûte.
- [x] **Défaire une insertion la RETIRE** — on remet les octets d'avant
      en retirant `after.len()`, ce qu'on a écrit, et non `before.len()`,
      ce qu'on a pris. Contrôle :
      `inserting_grows_the_file_and_undoing_gives_it_back_whole`, qui
      compare le fichier entier — sa taille seule aurait laissé passer la
      faute.
- [x] **Maj+clic et Maj+flèches tendent une sélection** — un bout fixe,
      un curseur qui bouge, et un trait sous le curseur pour savoir
      lequel des trente octets la frappe touchera. Piège trouvé :
      `matches_logically` tolère les modificateurs en trop, donc
      `consume_key(NONE, …)` avalait les Maj+flèches.
- [x] **« Écrire » remplace toute la sélection, « Remplir de 90 » la
      recouvre** — en UN correctif, donc un seul geste pour défaire.
      Contrôle : `shift_arrows_pick_several_bytes_and_fill_writes_them_at_once`.
- [x] **La sélection s'emporte** — Ctrl+A prend tout, Ctrl+C copie dans
      l'écriture de la colonne active, Ctrl+V rapporte **dans le champ**
      sans rien écrire : un collage peut porter quatre mille octets venus
      d'on ne sait où, et c'est le seul endroit du panneau où la frappe
      n'atteint pas le fichier. Contrôle :
      `the_selection_can_be_copied_and_the_clipboard_lands_in_the_field`,
      éprouvé en retirant la copie.
- [x] **Ce que l'écriture ferait à la taille est dit avant le clic** — le
      refus existait après coup ; apprendre après que tout s'est décalé
      n'est pas apprendre.

---

## 0 quinquies. La calculatrice : le presse-papiers, et l'épuration (A 9.20)

- [x] **Le clavier appartient à l'expression** — un clic sur une touche,
      un bit ou une fonction est une surface qui se clique sans se
      focaliser : egui retirait alors le focus au champ et ne le donnait
      à personne. Ctrl+C et Ctrl+V ne servaient plus dès la PREMIÈRE
      touche cliquée, c'est-à-dire au moment précis où l'on s'en sert.
      Le focus revient, et **le curseur avec lui** — posé avant que le
      champ ne soit construit, sans quoi « 0x » cliqué puis « ff » collé
      donnait `ff0x`. Contrôle :
      `the_keypad_gives_the_keyboard_back_to_the_expression`, qui fait
      le geste entier (cliquer, puis coller) et a échoué deux fois avant
      de passer.
- [x] **Les deux touches rouges quittent le clavier** — « Effacer tout »
      et le retour arrière tenaient un bandeau pleine largeur au-dessus
      des chiffres : la plus grande surface de l'appareil pour ce qu'on
      fait quand on s'est trompé. Elles sont au bout de la ligne
      d'expression, là où est ce qu'elles effacent, et disent pourquoi
      elles sont éteintes. Le clavier redevient un pavé régulier, et la
      famille de couleur « ce qui efface » s'en va — deux familles, deux
      teintes.
- [x] **Les six familles de fonctions tiennent sur une ligne** —
      « Calculer » passait seule à la suivante, où elle ne ressemblait
      plus à un onglet mais à l'intertitre de la rangée de pastilles
      qui la suit. Contrôle : `the_six_function_families_stay_on_one_row`,
      qui compare les ordonnées, éprouvé en remettant les écarts par
      défaut.
- [x] **`Espace c a` ouvre et referme la calculatrice** — une famille
      nouvelle, `Espace c…` (calculer), posée là où le reste de ce qui
      calcule viendra se ranger. Ctrl+Maj+A dans le jeu ordinaire. C'est
      la **seule bascule** du programme : elle s'ouvre au milieu d'autre
      chose, et la refermer ne doit pas demander de viser sa croix. Le
      bouton, le menu et la palette passent par la même action ;
      `PaletteCommand::ShowCalculator` disparaît.

---

## 0 sexies. Le pseudo-code : la comparaison fondue dans la condition (A 9.21)

- [x] **`comparer RCX et 0` + `tant que différent` deviennent
      `tant que RCX ≠ 0`** — c'était le dernier endroit où le pseudo-code
      restait du désassemblage traduit mot à mot. Le signe est dit des
      deux côtés : sur `0xFFFFFFFF`, « plus petit » est vrai en signé et
      faux en non signé.
- [x] **Une opération ordinaire prête sa condition sans disparaître** —
      `dec ecx` + `jnz` se lit « tant que ECX ≠ 0 », mais le `dec`
      décrémente pour de bon. Et seulement « nul » ou « non nul » : ZF
      est le seul indicateur que toutes ces instructions posent de la
      même façon, et un contrôle exige que `dec` + `jb` soit refusé.
- [x] **La ligne de structure garde l'adresse qu'elle a avalée** — sinon
      le processeur arrêté sur le `cmp` n'a plus de ligne à surligner.
      `Ligne` porte `est_structure`, au lieu de laisser déduire la
      couleur d'une adresse absente.
- [x] **`imul` à trois opérandes disait une phrase fausse** —
      `imul rax, rbx, 4` veut dire « RAX = RBX × 4 », et non
      « RAX = RAX × RBX ». Sur l'instruction que tout compilateur
      produit pour indexer un tableau.
- [x] **Les instructions de chaîne, `loop`, `jrcxz`, `xchg`, `cmpxchg`,
      `xadd`, `int3`** — `rep movsb` est `memcpy`, `movsb` est un pas,
      et le préfixe ne se voit pas dans le mnémonique. Reconnues à leurs
      opérandes : chez iced-x86, `Movsd` nomme deux instructions.

---

## 0. Des onglets de vues, façon Cutter (A 9.15, A 9.16)

Deux captures posées côte à côte — Cutter et Binary Ninja — et une
question : « ne serait-ce pas mieux ? » Oui pour Cutter, non pour Binary
Ninja, dont les onglets de documents existent déjà ici sous le nom de
barre des fichiers. Ce que Cutter désigne, c'est l'autre rangée : **les
vues d'un même objet**, que namsdec traitait en dix-neuf fenêtres
flottantes. **Les neuf vues y sont passées**, en deux temps et dans la
même journée.

- [x] **Une rangée de vues sous les onglets de fichiers** — `Source` et
      `Désassemblage` pour commencer. Deux rangées, deux rôles : l'une
      dit *quel* programme, l'autre *comment* on le regarde, et elles se
      distinguent à l'œil — rectangle plein marqué en haut et croix pour
      un fichier, étiquette soulignée en bas et pas de croix pour une
      vue. Contrôles : `tests/vues.rs`.
- [x] **Le désassemblage a quitté sa fenêtre** — il garde le chemin du
      retour. « Détacher » le sort dans une fenêtre pour le lire à côté
      de la source, ce qu'un onglet interdit par construction ; fermer
      la fenêtre le rend à la rangée. L'invariant : une vue est quelque
      part, jamais aux deux endroits ni nulle part. Un seul corps pour
      les deux lieux, retenu par
      `le_desassemblage_dit_la_meme_chose_dans_la_rangee_et_dans_sa_fenetre`.
- [x] **La barre des fichiers est sortie de l'éditeur** — elle vaut pour
      toutes les vues : le désassemblage est celui de *ce* fichier, et
      elle disparaissait avec l'éditeur.
- [x] **Ce que le passage d'une fenêtre à un panneau a révélé** — un
      contenu qui vivait dans une fenêtre ignore qu'il déborde, le cadre
      le rognait. À 800 x 520 le listing n'a que deux pixels, et ses
      adresses se peignaient sur l'onglet « Console ». Trouvé par le
      banc dès que les scènes `vue / source` et `vue / disassembly` y
      sont entrées. Deux remèdes : un `CentralPanel` qui borne et rogne,
      et la phrase du moteur qui s'efface quand la place manque **dans
      la rangée** — la hauteur seule ne distingue pas les deux lieux
      (143 px dans la fenêtre à vide, 150 dans l'onglet à l'étroit),
      c'est donc le lieu qui décide.

- [x] **Les sept autres vues ont suivi** (A 9.16) — la rangée en portait
      deux, elle en porte **neuf** : analyse, déroulement, décompilation,
      chronologie, points d'arrêt, registres étendus et correctifs ont
      quitté les fenêtres flottantes. Le menu Outils, la palette et
      « Espace v g » *choisissent l'onglet* au lieu d'ouvrir une
      fenêtre ; « Détacher » reste le chemin du côte à côte. L'onglet
      porte un nom court et dit son titre entier au survol — « Analyse »
      pour « Analyse du binaire » —, et la décompilation n'a d'onglet
      que si son module est allumé. Contrôles : cinq nouveaux, deux
      élargis, quatorze scènes de plus au banc.
- [x] **La rangée passe à la ligne plutôt que de raboter** — 840 px
      d'étiquettes pour 400 de zone centrale à 800 x 520 : trois lignes
      là, une seule à partir de 1280. Le premier contrôle passait au
      vert avec le défaut remis — « Correctifs » se peignait sur six
      pixels, rogné au bord : *un onglet repoussé hors du bord n'est pas
      absent, il est rogné*. Il compare maintenant chaque onglet à
      lui-même au large, où rien ne le rogne.
- [x] **Ce que la capture a vu, et que la mesure ne cherchait pas** —
      « Détacher » se retrouvait seul sur une deuxième ligne, à vingt
      pixels de l'onglet qu'il commande. Sa place est retenue d'avance
      en haut à droite : une commande se cherche à sa place.
- [x] **Un trou du banc** — la scène « vue / decompiler » ne montrait pas
      la décompilation : son module est éteint par défaut, et la zone
      centrale retombait en silence sur la source. Le même piège que
      l'onglet RE du panneau. Retenu par
      `chaque_scene_de_vue_montre_bien_sa_vue`, qui demande à
      l'application ce qu'elle montre **après** avoir dessiné.

- [x] **Deux retours sur la capture du bouton « Détacher »** — il
      *touchait le bord* (une disposition de droite à gauche n'en laisse
      rien : son liseré venait contre le trait du panneau, et deux
      cadres qui se touchent se lisent comme un seul) et il était *trop
      grand* — 27 px de haut dans une rangée de 26. Il garde sa
      gouttière, la même constante que les onglets, et passe en petit :
      17 px. Contrôle :
      `le_bouton_detacher_garde_sa_gouttiere_et_reste_petit`, éprouvé en
      remettant les deux défauts.
- [x] **Renommer une fonction inconnue marche depuis la rangée** — le
      geste tenait à un fil : un module ne renomme pas, il *demande*, et
      la rangée devait apprendre à relever sa demande après le dessin.
      Sans cela le champ s'ouvre, on tape, on valide, et il ne se passe
      rien. Le contrôle refait le geste en entier — clic, Ctrl+A,
      frappe, Entrée — et relit le nom dans les notes et à l'écran, sans
      écrire sur le disque.
- [x] **Le texte de l'analyse était coupé par son propre ascenseur** — la
      largeur de lecture se prenait sur toute la place disponible,
      ascenseur compris ; dans l'onglet, plus étroit que la largeur de
      lecture, les paragraphes se posaient dix pixels trop larges et se
      coupaient au milieu d'un mot. **Un contrôle le retient depuis le
      27 octobre 2026** : `the_analysis_text_is_not_cut_by_its_own_scrollbar`.
      Le premier essai cherchait le TEXTE COUPÉ et passait au vert avec
      le défaut remis — à vingt-six largeurs, avec un binaire chargé, à
      l'échelle de cet écran ; il avait été retiré plutôt que gardé pour
      l'ornement. Celui-ci cherche la BARRE HORIZONTALE, c'est-à-dire le
      mécanisme et non le symptôme : le contenu déborde la vue de la
      largeur exacte de l'ascenseur. Éprouvé en remettant le défaut —
      il rougit dès 1100 px.

Ce qui reste ouvert : l'**explorateur de fichiers**, flottant ici, docké
à gauche chez Cutter comme chez Binary Ninja — ce n'est pas un onglet,
c'est un panneau. La rangée des vues n'y est pour rien : la barre d'outils se
dessine avant elle, dans un panneau à part.

---

## 0 bis. La revue egui du 24 octobre (A 9.13, A 9.14)

Suite verte — 1 312 contrôles — et deux défauts trouvés en REGARDANT
l'écran. Les deux étaient hors d'atteinte d'une mesure : le premier
parce que le contrôle qui le couvrait posait la mauvaise question, le
second parce qu'aucune question ne pouvait le poser.

- [x] **Douze fenêtres rouvertes se posaient au même endroit** — dix-huit
      paires se recouvraient à cent pour cent sur cet écran, et la moitié
      des barres de titre était entièrement enterrée. Le contrôle en
      place passait : il mesurait l'écart entre CENTRES, et huit pixels
      lui suffisaient. L'éventail en diagonale devient une grille en
      quinconce, à pas proportionnel ; les barres de titre couvertes
      passent de 20 à 5, aux quatre tailles d'écran mesurées, et de 5 à 0
      au-dessus de 2560 x 1440. Contrôle :
      `no_window_that_comes_back_is_buried_under_the_others`.
- [x] **Une fenêtre qui remplit l'écran ne prend plus de cran** — trouvé
      par le banc, qui a rougi à la correction précédente : le graphe
      chargé s'ouvre à 92 % de la largeur, son cran le faisait sortir, et
      le numéro d'un bloc s'écrivait par-dessus la légende.
      `default_window_pos_sized` rend le centre au-delà des deux tiers de
      l'écran.
- [x] **Le chevron de repli était la lettre « v »** — et « > » replié,
      aux trois en-têtes du panneau. Un « v » gris devant un titre se lit
      comme une lettre orpheline. Les chevrons d'egui à la place, choisis
      après avoir DEMANDÉ à la police lesquels elle a : ⏷ et ⏵ oui, ▾ ▸
      ▼ ▶ ⌄ ˅ non. Contrôle :
      `a_fold_is_announced_by_a_chevron_never_by_a_letter`.

Ce qui reste ouvert : à 1280 x 800, trois fenêtres restent enterrées, et
ce n'est pas le placement qui en est cause — la fenêtre d'analyse mesure
1 100 x 1 000 et celle des exercices 920 x 720. **La taille d'ouverture
des grandes fenêtres sur petit écran** est le chantier suivant. La
calculatrice, qui occupe 95 % de la hauteur, couvre la barre de menu au
premier affichage : c'est la même cause.

---

## 0 quater. Les onglets du panneau en tête du panneau (A 9.19)

« Menus onglets en bas ? » Non pour le bas — la console y a déjà ses
onglets, et une rangée séparée de son contenu ne se lit plus. Mais la
question pointait juste : ces onglets vivaient dans la **barre
d'outils**, détachés du panneau qu'ils commandent.

- [x] **Ils sont en tête du panneau** — sous le bandeau, au-dessus de ce
      qu'ils commandent. Mesuré des deux côtés : la zone de travail
      commençait à y=91 sous mille pixels (la barre se dédoublait pour
      porter la rangée) et commence à **y=60 à toutes les largeurs**.
      Trente-et-un pixels rendus à l'établi sur un écran qui n'en a que
      520.
- [x] **Le titre de la vue disparaît** — une rangée qui souligne
      l'onglet actif *est* le titre de ce qu'on regarde ; « Pile » dans
      la barre et « Pile » en tête de la vue, c'était deux fois la même
      chose à trois cents pixels l'une de l'autre. `section_header` n'a
      plus d'emploi.
- [x] **Un seul corps pour les deux rangées d'onglets plats**
      (`rangée_d_onglets`) — les vues de l'établi et les vues du panneau
      sont la même chose, et doivent se lire pareil. Sans ce partage,
      trois idiomes d'onglets au lieu de deux.
- [x] **La calculatrice reste dans la barre** — panneau masqué, elle
      n'aurait plus de chemin visible. Une icône tient à toutes les
      largeurs, ce que six onglets ne faisaient pas.
- [x] **Un défaut introduit par la factorisation, attrapé dans la
      minute** — « Détacher » posé après les lignes avec `ui.put`, qui
      ramène le curseur : tout ce qui suivait repartait du bas de la
      *première* ligne, et à 800 x 520 la deuxième ligne d'onglets se
      peignait sur le « Moteur : » du désassemblage.
- [x] **Sept contrôles réancrés** — quatre cherchaient le titre disparu,
      un comptait le nom de la vue deux fois, un situait la calculatrice
      par rapport à l'onglet Pile, un prenait le titre comme repère du
      haut de la vue. Aucun affaibli : celui de la place est même devenu
      plus exigeant.

---

## 0 ter bis. Les outils mémoire évinçaient le vidage (A 9.18)

« Les textes se chevauchent, RE-Mémoire », sur capture. Le
chevauchement n'a pas été reproduit — binaire réel, mode Texte, replis
ouverts par de vrais clics, vingt-quatre tailles, molette comprise :
zéro recouvrement. Sa signature est celle de la veille : deux mises en
page superposées, dont une restée du tampon. Mais la mesure a trouvé le
voisin, et celui-là était réel.

- [x] **Le vidage garde un plancher** — à 1280 x 820, les trois replis
      ouverts, il n'était pas peint *du tout* : la hauteur de l'onglet
      est imposée à son contenu, donc ce qui déborde est coupé et la
      zone défilante du panneau n'a rien à faire défiler. On ouvrait la
      carte et l'on perdait la vue, sans qu'une barre ne le dise. Cent
      vingt pixels lui sont réservés et c'est la boîte à outils qui
      défile ; la borne se calcule sur la place de l'onglet, jamais sur
      ce que les outils mesurent. Au large, rien ne change — mêmes 208 à
      222 textes peints. Contrôle :
      `les_outils_depliés_n_evincent_pas_le_vidage`, éprouvé en
      remettant le défaut.

---

## 0 ter. La bande vide qui n'existait pas (A 9.17)

Une capture à 800 x 520 montrait une bande vide là où la barre d'outils
replie les onglets du panneau — « Processeur », « Pile »… — et le banc
disait l'inverse. L'affaire a été portée une demi-journée comme « l'un
des deux se trompe ».

- [x] **C'est le banc qui avait raison** — la mesure posée dans
      *l'application en marche* (six `eprintln` derrière une variable
      d'environnement) rend les rectangles : « onglet "cpu" : rect
      [[421.9 63.0] - [505.7 88.0]] clip [[0.0 30.0] - [800.0 91.0]] ».
      Posés, dans le cadre, non recouverts. Rien à corriger dans
      l'application.
- [x] **Ce qui mentait, c'était la capture** — l'application s'ouvre à
      1280 x 820 ; je la redimensionnais après coup à `wmctrl`, et la
      bande des onglets restait telle que le tampon l'avait laissée.
      Un repeint complet, et les cinq onglets paraissent. Une capture
      prise après un redimensionnement forcé n'est pas une capture de
      l'application : c'est une capture d'un tampon. **Il y a un
      troisième endroit où mesurer, et c'est le programme en marche.**
- [x] **L'invariant qui aurait tranché tout de suite** — le repli
      n'était tenu que par « ne rien recouvrir », qui est l'invariant
      d'un évitement : une rangée qui ne se peint pas du tout ne
      recouvre rien non plus.
      `the_sidebar_tabs_are_painted_whole_on_the_smallest_screen` exige
      la présence et compare chaque étiquette à elle-même au large.
      Éprouvé en remettant deux défauts, un par invariant.

---

## 1. La passe UX/UI du 23 octobre (A 7.11)

Trois relevés en parallèle — l'écran le plus petit, les tailles de
texte, les formes et les commandes — puis un seul arbitrage, parce que
les invariants se contredisent et qu'il faut une seule tête pour
trancher. Neuf écarts, **six contrôles nouveaux**, deux élargis.

- [x] **Trois fenêtres sortaient de l'écran par le haut** — à 800 x 520,
      le plus petit affichage que l'application accepte, la calculatrice
      dépassait de 236 px, « À propos » de 79 et les registres étendus
      de 68. Et c'est le HAUT qui manquait : une fenêtre trop haute pour
      l'écran déborde des deux côtés à la fois, bandeau et croix de
      fermeture compris — on ne pouvait plus ni la déplacer ni la fermer
      à la souris. La calculatrice portait même l'intention inverse en
      commentaire. Contrôle :
      `a_window_fits_the_smallest_screen_the_app_accepts`.
- [x] **Ce que la correction a appris sur egui**, et qui a coûté trois
      allers-retours : une fenêtre grandit parce que son contenu
      DÉBORDE de la place qu'on lui laisse. Une zone qui défile ne
      déborde jamais — elle prend ce qu'on lui donne —, si bien qu'une
      fenêtre sans hauteur d'ouverture reste figée à la taille d'egui
      par défaut et coupe son propre contenu. Et `default_size` n'est
      pas `default_width` + `default_height` : la première pose aussi la
      taille de l'aire, qui ne redescend plus sur le contenu. Les
      registres étendus sont restés 440 x 1040 avec 405 px de vide le
      temps qu'on le comprenne.
- [x] **La barre « OK » ancrée empêchait la fenêtre de rétrécir** — un
      panneau bas réclame toute la hauteur du cadre. Posée dans le flux,
      elle suit le contenu. Second contrôle :
      `a_window_of_known_content_does_not_scroll_when_there_is_room`,
      qui interdit l'inverse — une fenêtre qui se met à défiler sur un
      écran qui avait la place.
- [x] **Une scène du banc ne montrait pas l'onglet qu'elle nommait** —
      l'onglet RE est masqué par défaut et le panneau retombe en
      silence sur le processeur. « application / re » mesurait donc
      l'onglet Processeur une seconde fois, sous un autre nom, dans les
      deux moitiés du banc. Tous les invariants passaient : ils
      regardaient un onglet, simplement pas celui qu'ils nommaient.
      Trouvé en cherchant pourquoi le seul texte en 14 pt de
      l'application n'était peint nulle part. Contrôle :
      `chaque_scene_d_onglet_montre_bien_son_onglet`.
- [x] **Le thème ne posait ni l'alerte ni l'erreur d'egui** — dix sites
      de `namsdec-re` peignaient donc l'orange vif (#FF8F00) et le rouge
      pur d'egui au milieu de fenêtres où tout le reste suit la palette.
      Deux lignes dans `Theme::visuals()`, et les trois thèmes livrés
      sont justes d'un coup. Contrôle :
      `every_theme_paints_its_own_warning_and_its_own_error`.
- [x] **Six commandes sur douze étaient plus petites que leurs voisines
      dans la barre d'outils** — « Exécuter », les trois pas, « Stop »
      et « Réinitialiser » à 18 px quand « Assembler » et les cinq
      onglets de la même rangée font 25. Celles dont on se sert le plus
      étaient les plus petites, et six pixels sous la cible de 24 que
      l'on vise à la souris. `icon_button` demandait `interact_size.y`,
      la hauteur de ce qu'on n'a pas décrit : un bouton à icône n'est
      pas une autre sorte de bouton. Contrôle :
      `every_command_of_the_toolbar_has_the_same_height`.
- [x] **Une quatrième écriture de bouton, invisible du contrôle** —
      `ui.small_button(`, onze fois. « RAX », « RSP » et « RIP » du
      motif cyclique se lisaient comme trois étiquettes à soixante
      pixels de trois boutons encadrés. Le contrôle de source en
      connaissait trois sur quatre ; il en connaît quatre, et
      `small_framed_button` encadre sans rien épaissir.
- [x] **Le bouton principal de la boîte de saisie était deux pixels
      plus court que son voisin** — 110 x 25 contre 110 x 27, à huit
      pixels l'un de l'autre, faute du liseré invisible que `dialog_bar`
      pose déjà. L'invariant qui l'attrape existait depuis septembre ;
      il manquait la scène où le programme attend VRAIMENT une réponse.
      Une scène de plus, zéro contrôle neuf.
- [x] **Le même titre changeait de corps quand on cliquait dessus** —
      un module d'analyse s'écrivait en 13 pt dans la liste et en 15 pt
      une fois choisi : même texte, même fenêtre, même place. La page
      Outils des préférences ouvrait de même deux points au-dessus des
      quatre autres. Les deux avaient été relevés en septembre et
      rangés comme « inatteignables par le banc » : ils étaient peints
      cinquante-deux fois, dans vingt-cinq scènes. Ce qui manquait
      n'était pas une scène, c'était un inventaire. Contrôle :
      `every_text_size_belongs_to_a_declared_set`, qui aurait attrapé
      les deux d'un coup — et qui a trouvé en passant les onglets de la
      calculatrice en 11 pt, seuls de leur espèce.

## 1 bis. Ce que la capture a vu, et que la mesure ne demandait pas

Le troisième relevé du 23 octobre : seize scènes de plus capturées sur
profil neuf, et regardées. C'est le seul des trois qui ait trouvé des
défauts que personne n'avait pensé à mesurer.

- [x] **La barre d'outils et la rangée d'onglets se peignaient l'une
      sur l'autre** à 800x520 — « Step Back », « Stop » et
      « Réinitialiser » sous « Processeur », « Pile » et « Mémoire »,
      lettres sur lettres. Une disposition de droite à gauche ne recule
      pas devant le manque de place : elle part du bord droit et peint
      par-dessus. Les onglets passent maintenant à la ligne quand la
      rangée n'a plus la place, et la place se demande AVANT de poser,
      dans la même image. Contrôle :
      `panels_do_not_overlap_on_the_smallest_screen`.
- [x] **Un profil neuf ouvrait sur la pile** alors que `SidebarTab::Cpu`
      porte en commentaire « en tête, et vue par défaut ». Le premier
      écran d'un nouveau venu montrait donc l'onglet qui n'a rien à
      dire tant qu'on n'a pas assemblé, et le premier onglet de la
      rangée n'était pas celui qui était allumé.
- [x] **Une phrase des thèmes était rangée avec les exercices** — « Aucun
      thème installé pour l'instant » peinte en dernier, sous le titre
      « Séries d'exercices ». Et les deux titres de section de cette
      page, qui sont sœurs, n'avaient pas la même emphase.
- [x] **« Ce qu'il sait faire » s'écrivait deux fois**, à une ligne
      d'écart, parce que la ligne de pastilles des imports porte son
      propre libellé — elle est écrite pour vivre seule. Les familles
      qu'elle énumérait étaient réécrites juste en dessous, avec leur
      explication. Le contrôle vit dans `namsdec-re` : aucune scène du
      banc de l'interface n'a de binaire dont le comportement soit non
      vide, de sorte que ce chemin n'y est jamais peint.
- [x] **Les vingt-six modules d'analyse étaient énumérés deux fois** —
      une ligne de titres séparés par des virgules dans la carte, et le
      repli juste en dessous qui les redonne un par un. Le commentaire
      du code disait déjà « un par ligne PLUTÔT QU'en énumération dans
      la carte » : l'énumération était restée. Au passage, « 26
      module(s) » devant un nombre connu.
- [x] **Le contrôle qui l'aurait vu passait pour la mauvaise raison** :
      il cherchait chaque titre dans ce qui est peint, et la ligne de
      virgules — celle qu'il dit refuser — les contenait tous. Il ouvre
      désormais le repli d'un clic et regarde dedans.
- [x] **Le second bouton des préférences s'appelait « Quitter »**, le
      mot de Fichier > Quitter. Et il ne quitte pas : il REMET le thème,
      la langue, les onglets et les raccourcis tels qu'ils étaient à
      l'ouverture. Le commentaire qui justifiait ce nom — « les réglages
      s'appliquent à mesure, Annuler n'y défait rien » — datait d'avant
      l'instantané : le code avait changé sous lui. « Annuler », et un
      contrôle qui tient les deux bouts, ce que le bouton fait et ce
      qu'il dit.

**Mesuré, pas corrigé** (voir A 7.11) : une case à cocher décochée ne se
distingue pas du fond — fond `#3C3C3C`, trait de largeur zéro, rayon 6
sur un côté de 14, contraste **1,25:1** là où l'on demande 3. Elle se
lit comme un bouton radio, et comme allumée. La corriger demande notre
propre case, comme on a déjà fait notre propre liste déroulante : le
thème met tous les boutons à plat au repos, et c'est le même réglage qui
efface le bord de la case.

**Et une divergence entre l'interface et le manuel** : la barre d'outils
dit « Step Into », « Step Over », « Step Back », « Stop » en anglais
dans l'interface française, là où le manuel les nomme « Pas à pas
détaillé », « Pas à pas principal », « Pas en arrière », « Arrêter ».
Les mots français sont deux fois plus larges, et la rangée ne les tient
pas à 800 px : c'est une décision de produit avant d'être une
traduction.

## 17. Trois défauts vus à l'écran (A 9.1 à 9.3)

- [x] **Une parenthèse en posait cent** (A 9.1) — rien ne liait la pose
      d'une paire à une frappe : elle se refaisait à chaque image. Les
      contrôles éprouvaient la décision, jamais le geste.
- [x] **Le compteur comptait en Insertion** (A 9.2) — « INSERT ×8 »,
      trouvé sur la même capture en la lisant pour autre chose.
- [x] **Le don est réglable, et n'est plus fictif** (A 9.4) — l'adresse
      passe de constante à réglage, vide par défaut. Il reste à y poser
      la vraie : namsdec ne peut que la lire.
- [x] **Signaler une anomalie** (A 9.5) — le contexte rassemblé, montré
      en entier, et qui ne part qu'au clic. Ni le fichier ni son chemin.
- [x] **L'éditeur binaire : le refus était muet** (A 9.7) — trouvé avec
      crackme2. La grille marche de bout en bout ; c'est le rechargement
      qui refusait l'octet zéro, celui du nombre magique, et le
      correctif était jeté SANS UN MOT. Le refus s'explique, dit que le
      fichier n'a pas changé, et renvoie vers la copie à enregistrer.
      Trois relevés avaient conclu « tout va bien » avant (A 9.6) : ils
      ne pouvaient pas échouer.
- [x] **Ce qu'upx refuse, dit en français** (A 9.3) — le déballage
      fonctionnait ; c'est ce que le panneau en disait qui ne disait
      rien. Le contrôle qui le prouve avait lui aussi menti, en
      s'abstenant en silence.

---

## 16. Les frontières, avant les panneaux (A 8.1 à 8.6)

L'examen du 16 octobre disait la même chose de six façons : le périmètre
a grandi plus vite que les contrats de robustesse, et un atelier qui
ouvre des exécutables inconnus doit tenir ses frontières avant d'ajouter
un panneau de plus. Six points, six reproductions, six contrôles.

- [x] **Le plafond mémoire arrive avant l'allocation** (A 8.1) — un
      en-tête de 120 octets déclarant 512 Mio les obtenait pour se voir
      refuser juste après. Le plan (`load_plan`, `image_plan`) se calcule
      sur les seuls en-têtes et ne coûte rien ; `load_image(max)` et
      `image(max)` prennent le plafond en argument, de sorte qu'aucun
      appelant ne peut plus l'oublier. `try_reserve_exact` rend un refus
      du système affichable au lieu d'un arrêt.
- [x] **La sortie d'un outil est bornée pendant sa lecture** (A 8.2) —
      et non après. La coupe tombe sur une frontière UTF-8 (elle
      paniquait au milieu d'un « é »), stderr et le fichier `{out}`
      obéissent au même plafond, et l'incomplétude est dite.
- [x] **Le délai vaut pour toute la descendance** (A 8.3) — l'outil part
      dans son propre groupe de processus, c'est le groupe qu'on arrête,
      et la récolte des sorties a elle aussi une échéance. Un délai de
      100 ms rendait la main en cinq secondes.
- [x] **Le statut de sortie est gardé** (A 8.4) — `ToolOutput` porte le
      code, la provenance et la troncature. Un outil qui échoue le dit en
      tête de son panneau.
- [x] **Le format et l'architecture, dits explicitement** (A 8.5) —
      `e_machine` est lu, un ELF étranger est refusé en le nommant, et le
      type d'un fichier se décide à sa signature et non à la validité de
      son UTF-8.
- [x] **« En bref » dit qui a produit le binaire** (A 8.7) — le langage
      et le compilateur, chacun avec son degré de certitude et les
      indices qui le portent. L'essai sur /bin et /usr/bin a corrigé
      trois défauts invisibles à la lecture ; sur les quatre cents
      premiers fichiers, deux restent indéterminés.
- [x] **Ce qui se clique le dit au repos** (A 8.8) — le trait sous un
      texte cliquable est estompé au repos et franc au survol. Les
      rangées entières d'une liste, elles, gardent le survol seul.
- [x] **L'éditeur n'a plus de barre de défilement à demeure** (A 8.9) —
      et le contrôle qui le retient relève le bord de l'éditeur au lieu
      de l'estimer : le premier écrit passait au vert avec le défaut.
- [x] **La fiche d'un appel système, et son renvoi** (A 8.10) — la
      signature avec ses registres, ce que namsdec en fait, et `man 2`
      sur man7.org. Le renvoi va au noyau, pas à Intel.
- [x] **« Désactivé » ne voulait pas dire ce qu'on lisait** (A 8.11) —
      les pastilles d'un outil parlent de la liste des moteurs, et le
      disent. Un outil installé sert déjà, par son panneau RE.
- [x] **Le détour ne confond plus l'exemple et la valeur** (A 8.12) —
      champ prérempli, et un champ vide le dit au lieu de se déclarer
      illisible.
- [x] **Le menu d'une adresse dit où il mène** (A 8.13).
- [x] **Le numéro de ligne du Profil mène à sa ligne** (A 8.14).
- [x] **L'onglet Mémoire est lisible** (A 8.15) — le bandeau ne garde
      plus la place qu'il n'emploie pas (deux mille pixels de vide sur un
      grand écran), et les phrases du vidage se replient au lieu de
      partir hors de l'écran. Les deux contrôles passaient au vert avec
      les défauts remis : ils ne mesuraient rien, et c'est écrit.
- [x] **Un seul dossier pour les modules** (A 8.16) — `modules/`, et la
      fiche dit ce qu'elle rend par son champ `kind`. Les deux dossiers
      d'avant sont repris sans rien effacer.
- [x] **Rien n'écrit dans les fichiers de quelqu'un depuis un
      constructeur** (A 8.17) — la règle était tenue par l'habitude ;
      elle l'est maintenant par un contrôle, après avoir été enfreinte.
- [ ] **Une base de données ?** (A 8.18) — non pour la configuration, qui
      doit rester lisible et copiable. Oui, peut-être, pour les notes d'un
      binaire, l'histoire d'exécution et le cache des analyses : c'est là
      qu'on voudrait interroger sans tout relire.
- [x] **Plusieurs jeux de raccourcis** (A 8.19) — `~/.config/namsdec/
      keymaps/`, listés dans Préférences › Raccourcis, chargés d'un clic.
      namsdec ne fait que lire ce dossier.
- [x] **L'onglet RE porte une fiche d'identité** (A 8.20) — forme,
      langage, compilateur, protections, chiffres, portrait : ce qu'on
      veut savoir sans rien ouvrir. Il ne portait qu'un bouton.
- [ ] **egui 0.37** (A 8.21, puis 9.54) — repris AVEC
      `egui_system_font_provider`, comme 8.21 le demandait, et la mesure
      a bougé. La compilation coûte trois lignes, pas six erreurs. Les
      POLICES SONT RÉGLÉES : le fournisseur répare les deux invariants
      de glyphes, et le « il empire » ne tient plus. Mais vingt-deux
      contrôles tombent encore, et ils ne découlaient pas des glyphes.
      Le vrai chantier est nommé : `Id::new` est déprécié au profit
      d'`IdSalt`, `PanelState::load` ne retrouve plus les panneaux
      (« le panneau latéral est affiché » échoue), et un changement de
      style réel laisse deux boutons voisins à 21 et 26 px. Défait ;
      le dépôt est en 0.36, 773 contrôles verts.
- [x] **Le graphe d'appels en style UML** (A 8.22) — stéréotype,
      compartiments, angles droits, et des dépendances en tirets à pointe
      ouverte, qui est ce qu'un appel est. Le renommage répercuté
      existait déjà, et ses contrôles aussi.
- [x] **Soixante-quatorze fonctions de plus** (A 8.23) — 97 → 171,
      choisies par un relevé des imports du corpus et non par intuition.
      Les classes de caractères et les tables de `<ctype.h>`, `sscanf`,
      `strtok`, le temps, les signaux, et une couche fichiers qui dit
      honnêtement qu'il n'y a pas de système de fichiers.
- [x] **Les branchements calculés sont résolus** (A 8.24) — un sur dix
      était muet. Quatre chemins qui se nomment, plus les tables de
      sauts : 318 tables et 9 631 cas retrouvés sur le corpus. Ce qui
      n'entre dans aucun chemin reste inconnu, et le dit.
- [x] **Les cibles retrouvées nourrissent l'analyse** (A 8.25) — 994
      arêtes d'appel, pour une seule fonction de plus. Mesurer le compte
      de fonctions aurait fait conclure que ça ne sert à rien.
- [x] **Le fuzzing couvre le chargeur** (A 8.6) — et y a trouvé cinq
      débordements d'entier. La classe est traitée à la source : les
      lecteurs de champs composent base et déplacement de façon
      contrôlée. Quatre graines au lieu d'une, parce qu'une seule laissait
      trois de ces cinq défauts au vert.

---

## 15. AVX-512 : le morceau qui se découpait (A 7.21)

Le chantier paraissait d'un bloc. La mesure a montré que la grande
majorité des occurrences sont sur **ymm et xmm** : des alias EVEX
d'instructions déjà exécutées, écrites par un compilateur qui se sert
de l'encodage pour ses trente-deux registres, pas pour ses masques.

- [x] **`vmovdqu8/16/32/64`, `vmovdqa32/64`, `vpxord/q`, `vpandd/q`,
      `vpandnd/q`, `vpord/q`** rejoignent les bras qui les exécutaient
      déjà — plus de 1 200 occurrences pour une liste de noms.
- [x] **Une instruction masquée S'ARRÊTE et le dit** — avant même de
      regarder le mnémonique. L'ignorer donnerait un résultat faux sans
      le dire, et c'est pire que de refuser.
- [x] **Les registres `k0`–`k7`** (A 7.22) — huit entiers de 64 bits,
      `kmov*`, les combinaisons, `kortest*`/`ktest*`, et ce qui les
      remplit : `vptestm*`, `vptestnm*`, `vpcmp*` avec leur prédicat.
      Montrés en binaire dans la fenêtre des registres étendus, et
      seulement quand il y en a.
- [x] **Les registres `zmm`** (A 7.23) — trente-deux registres de 512
      bits, la règle du haut étendue (VEX efface, SSE préserve), et le
      coût de l'historique mesuré : 2 352 octets par jeu, 11 Mo pour
      cinq mille pas, borné par un contrôle.
- [x] **Le masquage des opérations** (A 7.24) — fusion et mise à zéro,
      posées dans `write_vec_op` : toutes les instructions y passent, et
      le masquage est une opération d'écriture. Ce n'était donc pas un
      travail sur chacune. Une famille dont on ne sait pas nommer
      l'élément s'arrête encore, plutôt que de décaler les voies.

---

## 14. Les instructions de chaîne de SSE4.2 (A 7.20)

Ce qui restait chiffré de 7.15, et le chemin suivant après `pmaxub` :
`pcmpistri` ouvre les boucles de `strlen`, `strchr`, `strcmp` et
`strstr` dans la bibliothèque C.

- [x] **Les quatre formes** — `pcmpistri`, `pcmpistrm`, `pcmpestri`,
      `pcmpestrm`, et leurs `v*`.
- [x] **Les quatre questions**, avec leurs règles de validité, qui ne
      sont pas les mêmes selon la question.
- [x] **Les quatre indicateurs** (CF, ZF, SF, OF) : c'est sur eux que
      la bibliothèque C branche.
- [x] **Plus une seule instruction manquante** dans les neuf binaires
      d'épreuve, hors AVX-512.

---

## 13. La sortie lue comme un terminal (A 7.19)

Le défaut signalé en bas de 7.17 et laissé ouvert : `□[2J□[H□[36m` se
lisait en tête du logo, dans la console comme dans la boîte.

- [x] **`ansi.rs`** — couleurs (16 + palettes étendues ramenées aux
      huit), effacement, fin de ligne, colonne du curseur, retour
      chariot, tabulation, retour arrière. Ce qui n'est pas reconnu
      disparaît au lieu de s'afficher.
- [x] **Peint en un seul bloc**, par un `LayoutJob` : une étiquette par
      morceau coûterait des milliers de widgets, et la sélection ne
      traverserait pas de l'une à l'autre.
- [x] **La boîte de saisie coupe sur les lignes de l'écran**, et non
      plus sur le texte brut — deux découpes seraient deux vérités.
- [x] **Dix contrôles**, dont un de bout en bout qui exécute un
      programme en couleur et mesure la couleur peinte dans la console.
- [ ] **Pas un terminal complet** : ni hauteur d'écran fixe, ni curseur
      qui remonte dans ce qui est déjà écrit. `ESC[A` ne fait donc rien.

---

## 12. La calculatrice compte en bits (A 7.18)

Demandé avec la calculatrice de GNOME en mode Programmation posée à côté
de la nôtre : le même style, plus de fonctions de bits.

- [x] **Vingt-neuf fonctions de plus**, quarante-sept en tout (A 7.18) —
      lire et changer un bit, extraire un champ (`bits(x, h, b)`),
      renverser l'ordre, les compléments, `parity` (celle du x86),
      `ffs`/`log2`/`ispow2`/`nextpow2`, les rotations sur 8/16/32 bits,
      `sar`, `smin`/`smax`, `mulhi`.
- [x] **Six familles, une à la fois** — onglets courts, phrase entière
      au survol. À plat, les quarante-sept poussaient le bas de la
      fenêtre hors de l'écran : mesuré à 1103 px pour un écran de 1100.
- [x] **La famille ouverte est un réglage** — elle revient au lancement
      suivant ; une famille inconnue retombe sur la première.
- [x] **Trois contrôles**, dont un qui refuse un argument de trop et un
      qui vérifie que ce qui n'est pas ouvert n'est pas peint.
- [x] **Le guide compte les fonctions dans le code** — il disait
      « dix-huit » longtemps après qu'il y en avait quarante-sept.

---

## 11. Une fenêtre complète, et ce qui se clique (A 7.17)

Deux griefs sur capture, le 15 octobre : la boîte de saisie montrait
quatre lignes d'un logo qui en fait sept, et rien ne disait quels textes
se cliquent.

- [x] **La boîte montre tout ce que le programme a écrit** (A 7.17) —
      largeur de la plus longue ligne, hauteur de toutes, bornées à
      l'écran ; au-delà c'est la fin qu'on voit, et une ligne dit ce qui
      reste au-dessus. Le calcul est dans `fin_qui_tient`, à part pour
      être éprouvé : une ligne trop large se replie et compte double.
- [x] **Deux signes pour tout ce qui se clique** — la main, et le trait
      au survol. Posés par `cliquable` (interface) et `clickable_label`
      (panneaux) ; une rangée entière se souligne entière.
- [x] **Un contrôle qui lit les sources des DEUX crates** — aucune
      étiquette cliquable hors des helpers. Il a trouvé deux oublis du
      premier coup.
- [x] **Un contrôle de mesure** — le trait est compté sur ce qui est
      peint, souris posée puis retirée ; éprouvé en le retirant.
- [x] **Les séquences ANSI sont interprétées** (A 7.19) — couleurs,
      effacement, retour chariot qui recouvre sa ligne, colonne du
      curseur ; le reste est jeté plutôt qu'affiché. Les huit couleurs
      d'un terminal sont des rôles : namsdec les prend dans son thème.

---

## 10. « Aller à cette adresse » : où, au juste ? (A 7.16)

Un grief de Fred sur capture : le clic sur une adresse menait toujours
au listing, alors qu'une chaîne, une table ou un bourrage se lisent dans
les octets. Et la fenêtre d'analyse devait retrouver son bouton
agrandir, en pastille à la macOS.

- [x] **Le clic demande où** (A 7.16) — deux entrées, « dans le code
      désassemblé » et « dans l'éditeur binaire ». La destination
      voyage avec la demande (`ReRequest::GoTo { addr, target }`) ;
      aller au code **ferme** la fenêtre d'analyse, aller aux octets la
      garde ouverte. Le choix ne se pose pas là où le contexte dit déjà
      de quoi il s'agit — boîte du graphe d'appels, adresse dans du
      pseudo-code.
- [x] **L'éditeur binaire sait où on l'envoie** — `ReResult::ShowBytesAt`
      cadre la page, choisit l'octet, et écrit l'adresse d'où l'on vient.
- [x] **Position ET adresse**, deux colonnes — la seconde seulement si
      elle apprend quelque chose (`adresses_utiles`), et « Aller à »
      accepte les deux.
- [x] **La pastille verte revient sur la fenêtre d'analyse** — et
      l'agrandissement s'arrête sous la barre de menus, qu'il
      recouvrait.
- [x] **Six contrôles**, dont deux au clic réel et un qui juge l'ordre
      des pastilles.

---

## 9. Le calcul vectoriel entier, au complet (A 7.15)

Un grief sur capture : FROGMAN s'arrête sur « l'interpréteur namsdec ne
gère pas l'instruction Pmaxub », juste après avoir demandé sa clé. Ce
n'est pas une curiosité : `pminub` est la pièce maîtresse du `strlen` de
la bibliothèque C, et tout binaire qui lit une chaîne passe par là.

- [x] **Soixante-dix mnémoniques de plus** (A 7.15) — min/max dans les
      deux signes et les trois largeurs, l'arithmétique saturée,
      `pshufb`, `palignr`, les `pack*`, les `pmovzx*`/`pmovsx*`, les
      produits et leurs deux moitiés, `pmaddwd`, les horizontales,
      `pavgb`, `psadbw`, `pabs*`, `pinsr*`/`pextr*`, `pcmpeqq`,
      `pcmpgtq`, `ptest`, les déplacements non temporels, et les
      `prefetch*` qui passent sans rien faire. Les formes AVX suivent :
      seul l'encodage change.
- [x] **Ce qui manquait a été MESURÉ, pas supposé** — les mnémoniques du
      moteur confrontés à ce qu'`objdump` trouve dans les neuf binaires
      d'épreuve, par fréquence : `palignr` 574, `pminub` 511, `movntps`
      160, `prefetch*` 444.
- [x] **Six contrôles de bout en bout**, assemblés par le vrai nasm :
      `the_vector_strlen_of_a_real_libc_runs`,
      `packed_min_and_max_know_their_sign_and_their_width` (bâti sur la
      dualité qui fait que `pmaxub` et `pminsb` rendent le même vecteur),
      `saturating_arithmetic_stops_at_the_edge`,
      `packing_shuffling_and_widening_move_the_bytes_where_they_belong`,
      `packed_multiplication_and_measures`,
      `one_lane_at_a_time_and_what_ptest_answers`,
      `multiply_add_and_horizontal_sums`.
- [x] **Le manuel ne peut plus être en retard sur l'interpréteur** —
      huit fiches disaient « namsdec ne les simule pas » de ce qui
      s'exécute maintenant. `the_manual_does_not_claim_an_instruction_is_unsupported_when_it_runs`
      assemble l'exemple de chaque fiche qui l'affirme et la dénonce si
      elle passe ; il compte ce qu'il a jugé et refuse de passer à vide.
      Éprouvé en semant une fiche menteuse.
- [x] **Sept fiches nouvelles**, le manuel passe de 164 à 171 — exigées
      par `everything_the_emulator_runs_has_a_sheet`, qui avait raison.
- [x] **La fenêtre d'aide ne garde plus sa taille quoi qu'elle montre**
      — trouvé par la checklist d'avant-commit : les scènes du banc
      ouvraient l'aide sur son sommaire et jamais sur une fiche. Une
      scène de plus (`pminub`, la fiche aux vingt-quatre mnémoniques), et
      `a_window_opens_at_the_size_of_what_it_shows` a nommé 103 px de
      vide au-dessus du bouton « OK ».
- [x] **`pcmpistri` et ses trois sœurs** (A 7.20) — les quatre
      questions de SSE4.2, leurs règles de validité, leurs quatre
      indicateurs, et le SEIZE qui fait sortir des boucles. Trois
      contrôles de bout en bout, écrits sur les usages réels : strlen,
      strchr, strstr, strcmp, une classe de caractères.
- [x] **AVX-512** : complet pour ce que namsdec rencontre (A 7.21 à 7.24) — les alias
      EVEX sur xmm/ymm des instructions déjà exécutées, soit plus de
      mille deux cents occurrences, et un refus net de toute
      instruction masquée. Restent les registres `k` (1 100
      occurrences) et les `zmm` (350).

---

## 8. Le listing ne se met plus en page d'un bloc (A 7.14)

Le gros chantier de performance laissé ouvert depuis le 1er octobre :
ne poser que les lignes visibles. Fait le 14 octobre 2026, en mode
NORMAL seulement — le champ de texte y est déjà non interactif, et rien
de ce qu'il porte (saisie, sélection, presse-papier) n'y existe.

- [x] **La vue ligne à ligne** (A 5.3, 7.8, 7.14) — `vue_listing.rs`.
      Sur un listing d'un million de lignes : première image 2 942 ms →
      **116 ms**, régime 11,3 ms → **1,86 ms**, et la mémoire
      **2 032 Mo → 268 Mo** (le galley d'un tel listing pesait 1,8 Go).
      Sur crackme-01 : 343 ms → 13,5 ms, et la recherche ouverte
      3 735 ms → 214 µs.
- [x] **Les deux vues posent le texte au même endroit** — retenu par
      `both_views_lay_the_text_in_the_same_place`, à un demi-pixel près,
      éprouvé en remettant le défaut. Le pas est PRIS sur une ligne mise
      en page (la hauteur de police dit 15,125 là où les rangées se
      suivent de 15,0) et les colonnes LUES dans le galley de leur ligne
      (egui cale chaque glyphe sur le pixel).
- [x] **Une seule gouttière, un seul surlignage** — `Géométrie` répond
      aux mêmes questions qu'elle lise le galley du champ ou qu'elle
      calcule. Deux peintures qui dévieraient se verraient au changement
      de mode.
- [x] **Le texte entier n'est relevé qu'une fois par texte** — où
      commence chaque ligne, quelle est la plus longue, ce que le fichier
      définit. Retenu par `the_whole_text_is_surveyed_once_per_text`.
- [x] **La révision du texte, et qui a le droit de l'écrire** — les poses
      du listing servaient de preuve qu'il avait changé ; la vue leur
      retire ce rôle. `tout_ce_qui_ecrit_le_texte_le_dit` lit les
      vingt-neuf fichiers de l'interface et a trouvé du premier coup un
      site oublié.
- [x] **Cliquer dans le code y pose le curseur** — impossible jusqu'ici
      en mode Normal, le champ y étant non interactif.
- [x] **La gouttière suit le fichier** — trouvé sur capture : la pastille
      d'un point d'arrêt recouvrait le numéro (« ●012 » pour la ligne
      4 012), et sept chiffres n'y tenaient pas du tout.
- [x] **Entrer en Insertion ne coûte plus trois secondes** (A 9.53) —
      mesuré : le pic n'était pas la coloration (437 ms) mais ce qu'elle
      PRODUIT, 1 200 001 sections qu'egui met 2,46 s à poser. Au-delà de
      400 Ko, le texte se pose d'un bloc, sans couleur : 2,95 s → 1,02 s
      sur deux cent mille lignes. Le mode Normal garde ses couleurs, il
      ne pose que ce qu'il montre.

## 7. Les gros binaires ne gèlent plus — passe du 14 octobre (A 7.13)

Quatre gels mesurés, quatre fois la même cause : un travail
proportionnel au listing entier, refait à chaque image, pour en montrer
trente lignes. Aucun ne se voit sur crackme-01 ; tous se voient sur un
binaire de quatre mégaoctets.

- [x] **La recherche ne gèle plus l'application** (A 7.13) — 3 735 ms
      par image barre ouverte sur crackme-01 (24 081 occurrences de
      « mov »), contre 0,8 ms fermée ; sur croackpocalypse, l'image ne
      venait plus. Les occurrences sont relevées une fois, portent leur
      ligne et leur colonne (`text::Place`), et seule la tranche visible
      se peint. Retenu par
      `the_search_hits_are_gathered_once_and_kept` : six images, un
      relevé.
- [x] **La fenêtre Désassemblage ne redécode plus tout** (A 7.13) — elle
      composait une ligne par instruction du programme à chaque image, un
      décodage et deux chaînes chacune, alors que sa zone en montre
      quarante. Retenu par
      `the_disassembly_window_decodes_once_per_question`.
- [x] **La gouttière trouve ses rangées par dichotomie** (A 7.13) — 8,96
      ms par image pour retenir 61 rangées sur 1 098 627. Retenu par
      `the_visible_rows_are_the_same_the_dichotomy_finds_them`, qui
      vérifie que la dichotomie désigne EXACTEMENT ce que le parcours
      désignait.
- [x] **Le texte n'est reconnu qu'une fois par image** (A 7.13) — le
      cache de mise en page compare le texte entier pour le reconnaître,
      1,5 ms sur seize mégaoctets, et c'était demandé deux fois par
      image. En mode Normal le champ n'est pas interactif : la mise en
      page lui est prêtée. Retenu par
      `the_listing_is_recognised_once_per_frame`.
- [x] **« Y a-t-il quelque chose à perdre ? » une fois par image** (A
      7.13) — la question comparait deux textes entiers, jusqu'à six fois
      par image. Relevée une fois pour tout ce qui AFFICHE ; ce qui
      DÉCIDE compare toujours sur le moment, et c'est
      `assembling_is_greyed_out_on_a_binary_listing` qui l'a imposé.
- [x] **Le chargement ne cherche plus les étiquettes en quadratique**
      (A 7.13) — `annotated_listing` relisait toute la table des symboles
      par instruction ; les deux listes étant triées, on avance dans les
      deux ensemble.
- [x] **Les compteurs de vim** (A 1.2, 7.13) — `5j`, `3w`, `12k`. Le
      compteur en attente se voit à côté de la pastille de mode
      (`a_pending_count_is_shown`), il est borné à quatre chiffres, et
      `move_cursor_times` ne découpe le texte qu'une fois pour toute la
      répétition — il le recopiait entièrement à chaque frappe de `j`.
- [x] **La ligne qu'on lisait revient** (A 4.11, 7.13) — sous la même
      garde que les points d'arrêt, curseur posé avant que l'éditeur se
      dessine, et sans changer de mode : on rouvre un binaire pour le
      lire. Retenu par `the_line_we_were_reading_comes_back`.
- [~] **La première image d'un gros listing** (A 5.3, 7.8, 7.13) — 2,9 s
      sur un million de lignes, et c'est toujours le même chantier :
      sortir le listing du `TextEdit`. Deux choses de plus à y mettre,
      mesurées : epaint parcourt lui aussi toutes les rangées du galley à
      chaque tesselation, et un galley d'un million de rangées pèse ce
      que pèsent ses glyphes.

**Un cinquième gel, d'une autre espèce, a été trouvé le 28 octobre**
(A 9.56) — et il n'était pas proportionnel au listing : il venait d'un
programme EXTÉRIEUR qu'on attendait. `nasm` sur `times 2000000000 nop`
occupe plus de vingt secondes, et `Command::output()` attendait sans
borne. Les quatre d'ici se voyaient sur un gros binaire ; celui-là se
voit sur une ligne. Voir P2 en tête de fichier.

## 6. Huit griefs sur captures — passe du 13 octobre (A 7.12)

Une liste de Fred, deux captures. Deux défauts se cachaient derrière les
autres, et c'est eux qui expliquaient la moitié de la liste.

- [x] **Le listing défile enfin jusqu'à l'adresse cliquée.** Le grief
      disait « dans figures de code et références croisées on ne sait
      pas aller aux adresses, pourtant l'infobulle dit aller à cette
      adresse ». Mesuré : le clic posait bien la demande, l'application
      la relevait, le curseur allait sur la bonne ligne — et la vue ne
      bougeait pas. egui ne défile vers le curseur d'un champ de texte
      que lorsque la sélection change PENDANT que le champ a le focus ;
      quand c'est l'application qui l'écrit, il n'a rien changé à
      l'image suivante. L'éditeur amène donc lui-même la ligne au
      centre. Contrôle : `going_to_a_line_really_scrolls_the_listing_to_it`.
- [x] **La fenêtre d'analyse s'efface au clic d'une adresse**, et la
      barre d'état dit où l'on arrive (`Status::WentTo`). Elle couvrait
      ce qu'on venait de demander à voir. Contrôle au CLIC, par
      kittest : `clicking_an_address_in_a_panel_really_lands_in_the_listing`
      — et il a attrapé la suite du défaut, l'état de la fenêtre étant
      écrit APRÈS le relevé des demandes, ce qui la rouvrait aussitôt.
- [x] **Plus de bouton agrandir/restaurer sur la fenêtre d'analyse**
      (demandé) : agrandie, elle recouvrait l'écran, et le listing
      défilait derrière un panneau opaque.
- [x] **La barre de chaleur « incomplète »** n'était pas incomplète :
      dans un binaire de soixante mille lignes, les lignes chaudes sont
      à soixante mille lignes du haut, et ni l'onglet Profil ni un
      panneau d'analyse n'y menaient. Réglé par le défilement ci-dessus.
- [x] **Les fonctions se renomment dans leur liste** — on clique le nom,
      on tape, Entrée ; la flèche de retour rend le `sub_xxxx`.
      L'adresse y mène au listing, comme ailleurs. Au repos le nom est
      une étiquette tronquée (quarante champs de saisie pèsent, et un
      nom long y serait coupé sans qu'on puisse le lire).
- [x] **Le renommage se répercute dans le listing** — il était écrit une
      fois à l'ouverture, à partir des symboles du fichier : le nom
      paraissait partout sauf là où l'on passe ses journées. Le listing
      se récrit, les points d'arrêt suivent leur adresse (une étiquette
      nouvelle décale tout ce qui suit), les symboles d'origine restent
      intacts, et un listing édité à la main n'est pas touché. Contrôle :
      `renaming_a_function_shows_up_in_the_listing`.
- [x] **Un panneau par outil extérieur** — Ghidra, RetDec, radare2,
      objdump, ndisasm —, chacun gardant sa sortie : c'est ce qui permet
      de comparer deux décompilateurs sur la même fonction. Générique
      (`ToolModule`), nourri par `known::all()` ; `ReRequest::RunTool` et
      `ReResult::{ToolOutput, ToolState}` ferment le canal. Rien ne
      s'exécute tout seul, la commande est écrite avant d'être lancée,
      un outil absent dit comment l'obtenir. Contrôle de bout en bout, au
      clic : `a_tool_panel_really_runs_its_tool_and_shows_the_output`.
- [x] **Un outil qui écrit beaucoup ne se bloque plus** — trouvé en
      branchant le précédent : `objdump -d /bin/ls` rend plusieurs
      méga-octets, le tube se remplit à soixante-quatre kilo-octets, et
      l'outil bloque en écriture pendant que namsdec attend sa fin. Deux
      fils lisent à mesure. Contrôles : `an_output_bigger_than_a_pipe_still_comes_back`,
      `a_tool_that_never_returns_is_stopped`.
- [x] **Fragments réutilisables (ROP) : une navigation** — deux mille
      fragments récoltés, quarante par page, « 41–80 sur 1 274 », et la
      recherche ramène en première page. La liste s'arrêtait à quarante
      sans dire qu'il y avait une suite.
- [x] **Entropie : compartimentée, graduée, et ses adresses** — la
      tranche suit la taille du binaire (256 o à 64 Ko, quelques
      centaines de tranches toujours) ; le profil prend toute la largeur
      et jamais plus de la moitié de la fenêtre ; une règle d'adresses
      court dessous ; l'échelle de couleurs est graduée de 0 à 8 avec
      neuf arrêts (cinq ne distinguaient pas un demi-bit dans la zone du
      code) ; et les **plages** donnent les adresses, cliquables, les
      miettes rejoignant leur voisine. Contrôles :
      `the_profile_never_takes_more_than_half_the_window`,
      `the_scale_tells_apart_what_half_a_bit_separates`,
      `the_panel_writes_its_ranges_and_where_they_are`,
      `the_window_grows_with_the_file_and_stays_round`,
      `the_ranges_join_what_says_the_same_thing`.
- [x] **Des carrés vides à la place de caractères** — trouvé en
      regardant la capture d'une nouvelle pagination : les polices
      qu'egui embarque n'ont ni `✓` ni `✗` ni les triangles ni les
      flèches. Six endroits les employaient, dont la liste des
      protections. Remplacés, et deux contrôles regardent désormais ce
      qui est PEINT, sur toutes les scènes :
      `tout_caractere_peint_existe_dans_la_police` (interface) et
      `tout_caractere_ecrit_existe_dans_la_police` (panneaux).
- [x] **La colonne de navigation tient ses vingt-quatre rubriques** —
      les noms y sont courts (« RetDec » et non « Décompilation par
      RetDec », qui passait sur deux lignes), les entrées serrées, et la
      fenêtre s'ouvre cent pixels plus haut. Nouveau `ReModule::nav_title`.
- [x] **Les fonctions empruntées sont des liens** (demandé) — survol :
      la phrase ; clic : la fiche, avec la signature et le registre qui
      porte chaque argument (`libc::ARGUMENT_REGISTERS`, désormais la
      seule source des noms de registres d'argument dans la crate —
      `explain.rs` en tirait une seconde liste). Quatre entrées
      nouvelles dans la table (`__libc_start_main`, `__cxa_finalize`,
      `__cxa_atexit`, `_Unwind_Resume`) : c'étaient les deux seules
      lignes du crackme de Fred, et elles étaient muettes. Et
      `libc::family_hint` dit ce que le PRÉFIXE d'un nom apprend sans
      rien deviner sur la fonction — `explain` garde son contrat.
      Contrôles : `le_nom_d_une_fonction_se_deplie_sur_sa_fiche` (qui a
      appris en chemin qu'egui ne peint pas ce qui sort de l'écran :
      la fiche se déplie sur la PREMIÈRE entrée affichée) et
      `une_fonction_inconnue_dit_ce_que_son_nom_apprend`.
- [x] **La fenêtre d'analyse ne grandit plus avec son contenu**
      (capture de Fred, rubrique « Chaînes lisibles ») — les chemins de
      source Rust, cent caractères avec leur empreinte de version,
      poussaient la fenêtre à 1 831 px de large contre 1 100 au départ,
      colonne de navigation sortie par la gauche. Et elle y restait :
      egui garde la taille d'une fenêtre. Deux bornes : le contenu
      défile désormais dans les DEUX sens (avec la largeur de lecture
      imposée avant la zone, sans quoi les panneaux qui se
      dimensionnent sur `available_width()` s'étireraient), et la
      fenêtre est bornée à ce qu'elle peut montrer — colonne plus
      largeur de lecture. Contrôle sur TOUTES les rubriques :
      `la_fenetre_d_analyse_ne_grandit_pas_avec_son_contenu`, éprouvé
      en remettant le défaut.
- [x] **Le processeur est un onglet** (demandé, option 1 sur trois
      proposées) — `SidebarTab::Cpu` en tête, vue par défaut, titre
      « État du processeur » dans le panneau et « Processeur » dans la
      rangée. Le panneau passe de trois zones à deux, et tout le
      calcul de partage lié à la hauteur des registres disparaît
      (`sidebar_cpu_height`, `cpu_state_open`, plancher de 280 px
      ramené à ce que le bandeau demande vraiment). La vue est bornée
      à la place du panneau — elle y défile plutôt que de déborder — et
      sa hauteur lui est imposée DANS la zone défilante, sans quoi les
      vues qui dimensionnent leur liste sur `available_height()`
      cessent de défiler elles-mêmes. Contrôle :
      `l_etat_du_processeur_est_un_onglet_et_la_vue_choisie_a_tout_le_panneau`.
- [x] **Les registres sur une colonne de plus quand le panneau est
      court** (demandé) — le blocage n'était pas la largeur des valeurs
      mais le BLANC entre colonnes (quatre espacements). Nouvelle
      fonction pure `register_layout` : elle rend le nombre de colonnes
      et le blanc, serre seulement si les registres ne tiennent pas en
      hauteur, et seulement si serrer gagne vraiment une colonne. Pas
      de boucle de mesure — la question posée ne dépend pas du nombre
      de colonnes choisi. Contrôles : les trois cas de
      `les_registres_gagnent_une_colonne_quand_le_panneau_est_court`,
      et `a_l_ecran_les_registres_se_rangent_sur_une_colonne_de_plus`
      qui compte les abscisses peintes. Limite mesurée et assumée : sur
      une fenêtre minimale (800×520), la largeur n'autorise que deux
      colonnes quoi qu'on fasse.
- [x] **L'onglet Octets suivait l'exécution de travers** — trouvé en
      corrigeant le reste : `show_rows` veut la hauteur d'une ligne
      SANS l'espace qui la sépare de la suivante, il l'ajoute lui-même.
      On lui donnait les deux : il croyait à des rangées de 21 px là où
      il en posait 27, et le décalage calculé pour centrer
      l'instruction courante tombait soixante-dix lignes trop haut sur
      un programme de quatre cents instructions. La fenêtre de
      décompilation portait déjà la remarque, pour la même raison.
- [x] **Le panneau latéral ne se recouvre plus, même à l'étroit**
      (capture de Fred, onglet Mémoire) — l'état du processeur
      s'écrivait sur le haut de la vue. Cause mesurée, et elle est dans
      egui : quand le contenu d'un panneau dépasse la place laissée,
      egui ramène la TAILLE du rectangle au maximum demandé en gardant
      le bord du CONTENU et non celui de la place — le rectangle glisse
      vers le bas, et le panneau ne rend au parent que ce qui reste
      visible (relevé : peint de 361 à 539, rendu de 403 à 539). Le
      contenu de la vue est borné à son panneau par une zone défilante,
      et le bandeau perd son plancher de 64 px (`min_scrolled_height`),
      qui le faisait déborder de vingt et un pixels sur un panneau
      court. Contrôle sur les cinq vues, panneau à l'étroit et poignée
      tirée : `le_panneau_lateral_ne_se_recouvre_jamais_meme_a_l_etroit`
      — éprouvé en remettant le défaut.
- [x] **Et elles sont rangées par familles** (demandé) — Le fichier, Le
      code, Décompiler, Ce qu'il dit, Ce qu'il emprunte, Les octets, dans
      l'ordre où l'on lit un binaire inconnu. Le module déclare la
      sienne (`ReModule::section`, `ReSection`), l'ordre
      d'enregistrement les tient groupées, et la colonne pose un
      intertitre quand la famille change. Deux rubriques ont déménagé
      pour rejoindre la leur : « Figures du code » et « Références
      croisées » étaient au milieu de ce que le binaire emprunte.
      Contrôle : `les_rubriques_d_une_meme_famille_se_suivent` — éprouvé
      en remettant le défaut, il nomme la rubrique fautive et sa
      famille.

## 5. Analyse et exercices — passe du 7 octobre (0.2.8-beta.1)

Cinq demandes de Fred, sur captures.

- [x] **Entropie en dégradé continu** — l'histogramme passait par quatre
      paliers de verdict ; il suit un dégradé vert → jaune → orange →
      rouge → violet sur l'échelle 0..8, couleurs fixes hors thème.
      Contrôle : `the_gradient_climbs_from_green_to_purple`.
- [x] **Flags plus espacés** dans la barre de titre (4 → 8 px) : collés,
      ils se lisaient comme un bloc.
- [x] **Un nom donné suit l'adresse partout** — `clickable_address`
      montre le nom à la main quand il y en a un (propagation à tous les
      panneaux), avec un léger fond en évidence, repris dans l'en-tête de
      l'arbre d'appels et la liste des fonctions. Pivot :
      `ReContext::user_name`.
- [x] **Le but et la progression d'un exercice** quittent le bandeau du
      processeur (toujours visible, d'où l'exercice resté affiché après
      chargement d'un binaire) pour la **fenêtre dédiée** (Aide ›
      Exercices). `goal_ui` passe `pub(crate)`.
- [x] **Rubrique YARA dans l'onglet RE** — nouveau `YaraModule` : tire une
      règle YARA du binaire (chaînes distinctives sans le bruit de
      bibliothèque, nombre magique, empreinte en méta, seuil prudent), à
      coller dans un `.yar`. Contrôles : `the_rule_is_well_formed` & co.

### Éditeur binaire (7 octobre)

- [x] **Début de l'éditeur binaire dans l'onglet RE** — nouveau module
      `HexEditorModule` (« Éditeur binaire »). Le fichier en grille
      hexadécimale (décalage · 16 octets · colonne de texte), navigable
      par page et par « Aller à ». On clique un octet, on en pose un
      autre : `75` (jne) → `74` (je) pour lire l'autre côté du saut. Ce
      qu'on a touché porte un fond. Chaque écriture passe par le journal
      des correctifs (réversible, Outils › Correctifs), et rien n'atteint
      le disque sans enregistrement explicite, vers un fichier choisi —
      jamais par-dessus l'original. Première version : l'édition multiple
      et l'insertion (qui décale les adresses) viendront ensuite.
      Contrôles : les trois de `hex_editor_module` +
      `l_editeur_binaire_dispose_le_fichier_en_grille`.
- [x] **Éditeur binaire — l'inspecteur de données (façon 010 Editor)** —
      sous l'octet choisi, les octets lus dans chaque format : int8…int64
      signés/non signés, flottants, caractère, bits, en petit-boutien.
      Chaque lecture se copie d'un clic. Contrôle :
      `l_inspecteur_lit_chaque_format`. Reste vers 010 Editor : frappe en
      place dans la grille, édition de la colonne de texte, curseur aux
flèches, bascule gros-boutien.
- [x] **Éditeur binaire — la recherche** — un motif en texte (« ELF ») ou
      en octets hexadécimaux (`7f 45 4c 46`), en avant ou en arrière, qui
      fait le tour, trouvaille surlignée. Fonctions pures `motif` et
      `trouver`. Contrôles : `un_motif_se_lit_en_hex_ou_en_texte`,
      `la_recherche_trouve_et_fait_le_tour`. (Étudié d'après 010 Editor
      16.0.4, dont on s'inspire des fonctions — jamais du code.)
- [x] **Éditeur binaire — clavier (façon 010 Editor)** — un octet choisi,
      les flèches déplacent le curseur (←→, ↑↓ d'une ligne, Page), et les
      chiffres hexa l'écrivent en place, quartet par quartet (7 puis 4 →
      74, avance tout seul). Échap annule un quartet. N'agit que si aucun
      champ de texte n'a le focus. Logique pure `frapper`, testée. Reste
      vers 010 Editor : édition de la colonne de texte, bascule
      gros-boutien.
- [x] **Éditeur binaire — la fenêtre n'enfle plus au clic d'un octet
      (8 octobre)** — l'inspecteur qui s'ouvre sous l'octet choisi était
      une `egui::Grid`. Dans la fenêtre d'analyse, qui s'ajuste à son
      contenu, toute rangée imbriquée (grille ou `ui.horizontal`) réserve
      la largeur disponible : la fenêtre grandit pour la contenir, ce qui
      agrandit la disponible, et l'on repart. Mesuré : 1100 px → 2878,
      puis deux de plus à chaque image, jusqu'à déborder de l'écran —
      centrée, sa moitié gauche passait hors champ et le contenu se
      coupait à gauche (capture de Fred). Bissecté jusqu'à la rangée.
      L'inspecteur est maintenant **un libellé par ligne**, une seule
      galère de texte (nom en chasse fixe atténué + valeur, via
      `LayoutJob`), qui ne lit aucune largeur. Contrôle :
      `choisir_un_octet_ne_fait_pas_enfler_la_fenetre`.
- [x] **Onglet Mémoire — « Lire en : » se peignait sur « Suivre : »
      (8 octobre)** — « Suivre : … » et « Lire en : … » tenaient sur une
      seule rangée `horizontal_wrapped`. Sur un panneau étroit elle
      débordait, et `horizontal_wrapped` donnait à l'élément reporté une
      galère démesurée (257 px pour les neuf caractères de « Lire en : »)
      peinte à l'abscisse de départ, par-dessus « Suivre : » et les
      boutons de suivi — cinq textes l'un sur l'autre, mesuré sur binaire
      chargé, barre latérale ~480 px, à tout facteur d'échelle. Deux
      rangées franches règlent l'affaire. Contrôle :
      `lire_en_ne_se_peint_pas_sur_suivre`.
- [x] **Onglet Mémoire — la carte dépliée se peignait sur les registres
      (8 octobre, le défaut de la capture de Fred)** — un binaire aux
      régions nombreuses (crackme-01 en a **34** ; les sections d'un
      programme lié dynamiquement) dépliait une carte si haute que le
      contenu de l'onglet dépassait la place que le panneau latéral lui
      réserve. Le partage à trois (bandeau, état du processeur, onglet)
      **divergeait** alors — même classe que le #8 : selon le chemin,
      l'onglet remontait par-dessus les registres (RAX…RIP recouverts par
      la carte, ce que Fred voyait) ou filait sous le bas de la fenêtre
      (mesuré : bas à 1646 px pour une fenêtre de 1183). La carte défile
      désormais dans sa propre fenêtre **bornée** (`max_height`) : trop
      haute, elle ne peut plus pousser le reste. Reproduit via les
      34 régions de crackme ; contrôle :
      `la_carte_depliee_ne_couvre_pas_les_registres`.
- [x] **Le nom du binaire s'affiche sur l'onglet (8 octobre)** — un
      binaire ouvert n'a pas de `path` (on ne le rouvre pas en
      l'enregistrant par-dessus l'original), et l'onglet restait « sans
      titre ». Son nom voyage pourtant dans `from_binary` : c'est lui qui
      s'écrit maintenant, sur l'onglet actif comme sur ceux rangés.
      Contrôle : `l_onglet_d_un_binaire_porte_son_nom`.
- [x] **Le chargement d'un gros binaire ne fige plus l'interface
      (8 octobre)** — ouvrir un binaire de 3 Mo bloquait le fil
      d'interface **quatre secondes** (bien plus en build debug), au point
      que le système proposait de « forcer à quitter ». Mesuré : tout le
      temps était dans `on_binary_loaded`, et dedans dans
      `functions::discover`, qui rebalayait les 400 000 instructions **pour
      chaque** fonction découverte — quadratique (le module « Fonctions »
      et le graphe d'appels l'appellent tous deux). Remplacé par une
      attribution dichotomique en un passage : `load_elf` passe de **4,0 s
      à 0,63 s** sur le même binaire. Même gain au Reset, qui refait la
      découverte. Garde-fou : `discovering_functions_stays_linear_on_a_big_binary`.
      Reste : la 1re image après chargement coûte ~1,4 s (rendu du listing
      de 400 000 lignes) — moins grave, autre sous-système.
- [x] **Commentaire périmé de `flags_ui` corrigé** — il disait encore
      « chacune se bascule d'un clic » depuis que les flags sont en
      lecture seule. (L'édition de registres, elle, est gardée — choix de
      Fred : c'est le `set \$reg` de gdb, réversible par Step Back.)
- [x] **Fausse alerte élucidée (8 octobre)** : je craignais que l'analyse
      reste périmée après un patch, parce que `load_elf` n'appelle pas
      `on_binary_loaded`. Vérifié empiriquement : elle se rafraîchit bien.
      `load_elf` appelle `rebuild_emulator`, qui dispatche vers
      `load_process` ou `rebuild_from_assembly` — et ces deux-là notifient
      les modules. Le SHA-256 change après une écriture. Verrouillé par
      `une_ecriture_rafraichit_l_analyse`, qui tombe si l'on casse la
      chaîne. Leçon : tracer jusqu'au bout avant d'inscrire un défaut.

- [x] **Vérifié (8 octobre) : les numéros de ligne ↔ les instructions** —
      demande de Fred. Alignés par construction : `layout_code` pose le
      texte à largeur de repli INFINIE, donc aucune ligne ne se replie,
      `galley.rows.len()` = nombre de lignes source, et le numéro `i+1`
      est la vraie ligne source ; la surbrillance d'exécution vient
      d'`addr_to_line[rip]`, la même ligne. Verrouillé par
      `les_numeros_de_la_gouttiere_suivent_les_lignes_source` (une ligne
      de 400 caractères en tête, numéros peints consécutifs à pas
      constant — un repli régressif les ferait sauter).

- [x] **Éditeur binaire — parité 010 Editor atteinte (8 octobre)** : deux
  dernières pièces. Édition dans la **colonne de texte** — Tab bascule de
  la colonne hexa à la colonne texte, où une frappe pose le caractère
  (via `Event::Text`), curseur partagé, même chemin d'écriture réversible.
  Et la **bascule gros-boutien** de l'inspecteur : le même quadruplet se
  relit dans l'autre sens, étiquettes LE/BE à l'appui. `inspecter(octets,
  be)` et l'enum `Colonne`. Contrôle `l_inspecteur_lit_chaque_format`
  étendu au gros-boutien.

### Pont DZF — le contrat JSON (8 octobre)

- [x] **Le contrat de décompilation DZF ↔ namsdec, figé.** DZF reçoit les
  octets et l'adresse, rend une décompilation en JSON ; namsdec la lit.
  Côté namsdec : `crates/namsdec-re/src/dzf.rs` — types serde tolérants
  (`DzfOutput`, `DzfFunction`) + `parse`, la source de vérité. Pour Astra :
  `docs/contrat-dzf.md`, la spec en français (appel du plugin, forme du
  JSON champ par champ, règles de tolérance). Principes : adresses
  virtuelles u64, `address` seule exigence dure, champs inconnus ignorés
  (DZF grandit sans casser namsdec), `version` pour les changements de
  sémantique, vide valide. Cinq contrôles (`un_exemple_complet_se_lit`,
  `le_minimum_suffit`, `un_champ_inconnu_est_ignore`,
  `une_fonction_sans_adresse_est_refusee`, `le_vide_se_lit`).
- [x] **Le panneau décompilateur DZF** (`DzfDecompilerModule`,
  « Décompilation DZF ») — rend le JSON du contrat : chaque fonction en
  pseudo-C dans un repli, sa signature, son adresse cliquable. Branché sur
  les **noms partagés** : une fonction rebaptisée dans l'arbre d'appels
  porte son nom namsdec ici aussi (prouvé :
  `le_nom_de_namsdec_l_emporte_sur_celui_de_dzf`). État vide clair tant que
  DZF n'a rien rendu. `set_decompilation` reçoit la sortie ;
  `take_request` relaie l'aller-à. Enregistré après le graphe d'appels.
- [x] **Câblage de bout en bout (8 octobre)** : le bouton « Décompiler »
  du panneau émet `ReRequest::Decompile` ; le GUI lance le décompilateur
  actif (`start_decompile`, en tâche de fond), et `poll_decompile` tente de
  lire sa sortie comme du JSON DZF (`ingest_dzf_json`) pour la livrer au
  panneau via le nouveau canal retour `ReResult` (`deliver`). Une sortie de
  texte (rizin/retdec) n'est pas livrée ; une demande DZF restée sans JSON
  se dit. Symétrie posée : `ReRequest` (module→app), `ReResult` (app→
  module). Éprouvé avec un « faux DZF » (JSON fourni à la main) :
  `le_json_de_dzf_livre_au_panneau_s_y_affiche`,
  `une_sortie_non_dzf_demandee_le_dit`.
- [x] **Adresses cliquables dans le pseudo-code (8 octobre)** —
  `0x401030`, `sub_401030`, `loc_401030` mènent au listing, à l'endroit
  exact visé. Tout le pseudo-code est peint en **une seule galère**
  (aucun `ui.horizontal` imbriqué, qui ferait gonfler la fenêtre
  d'analyse — cf. le #8) : les adresses sont colorées en lien, et la
  position du clic est testée contre les plages de caractères pour sauter
  à la bonne. Fonction pure `adresses_cliquables`, trois contrôles.
  Compromis : le pseudo-code n'est plus sélectionnable d'un bloc (un
  libellé cliquable n'est pas un libellé sélectionnable) — à revoir si le
  copier-coller manque.
- [ ] **À suivre** : éprouver avec le vrai DZF quand Astra l'aura fait
  émettre du conforme.

### Revue d'interface — l'éditeur binaire et les modules (8 octobre)

Audit egui (skill audit-ui-egui), concentré sur la surface neuve :
l'éditeur binaire (grille, inspecteur, recherche, clavier) et YARA.

- **Mesuré, propre.** Contraste du texte des octets sur les trois fonds de
  surlignage : 5,7–6,3 (le bleu de sélection #264F78 est assez sombre).
  Recouvrement : 0 sur 330 textes de l'éditeur. Découpage : 0. Les faux
  positifs écartés : le bloc « octet choisi » non capturé était SOUS le
  pli (joignable au défilement de la fenêtre d'analyse), pas découpé.
- [x] **Vrai manque trouvé et comblé : les scènes n'éprouvaient que le
  module d'analyse PAR DÉFAUT.** `fenêtres_ouvrables` ouvrait la fenêtre
  d'analyse sans choisir de module ; l'éditeur binaire, YARA, le graphe
  d'appels et la plupart des autres passaient au travers des SEPT
  invariants de scène (recouvrement, dérive, balisage, doublons, hauteur
  des boutons, panneau vide, taille d'ouverture). `scènes()` ajoute
  désormais une scène PAR MODULE (en marche) — tous éprouvés, un module
  neuf couvert sans qu'on y pense. Les sept passent sur les dix-sept
  modules.

### Passe de vérification (7 octobre)

Relecture à la recherche d'erreurs. Ce qui tenait : les analyseurs ne
paniquent sur aucun en-tête ELF incohérent (relevé jeté sur quinze points
d'entrée), le parcours du graphe d'appels reste fini, la détection d'un
outil installé balaie bien tous les sous-dossiers, les 791 chaînes des
deux langues sont traduites, et les deux séries d'exercices sont
exécutées par un contrôle. Ce qui ne tenait pas :

- [x] **Le manuel nommait six panneaux autrement que l'écran** —
      « Gadgets » pour *Fragments réutilisables (ROP)*, « Fonctions
      empruntées » pour *Fonctions importées*, « Chaînes » pour *Chaînes
      lisibles*, et trois noms tronqués (*Figures du code*, *Protections
      du binaire*, *Place libre (bourrage)*). On cherchait dans
      l'interface un nom qui n'y était pas. Neuf mentions corrigées, et
      un garde-fou qui lit les titres **dans le registre** :
      `le_manuel_nomme_les_panneaux_comme_l_interface`.
- [x] **Le journal des versions oubliait la moitié de 0.2.8** — l'entrée
      ne couvrait que les cinq dernières demandes et taisait tout ce qui
      était entré depuis 0.2.6 : le graphe d'appels, le renommage, les
      adresses cliquables, les signaux, la place libre, le désaccord de
      lecture, « En bref », le sommaire des 674 mnémoniques, DZF
      installable, la panique sur ELF hostile. Entrée complétée.
- [x] **L'en-tête de ce fichier annonçait 1 087 tests** pour 1 099.
- [x] **Quatre réglages que l'interface bascule n'étaient pas gardés** —
      l'explorateur de fichiers (alors que la console, la barre latérale
      et la barre d'état, ses frères, se souvenaient), le filtre du graphe
      de flot, celui du déroulement, et le repli des variables
      d'environnement. Le manuel promettait pourtant « tout réglage est
      enregistré » : c'est le code qui a été mis à la promesse. Garde-fou
      qui lit le code qui bascule et exige la correspondance dans les deux
      sens : `tout_reglage_bascule_par_l_interface_est_garde`.
- [x] **Les fichiers que namsdec réécrit n'étaient pas remplacés d'un
      bloc** — `fs::write` tronque puis écrit. Une coupure entre les deux
      laissait un `settings.json` vide, que la lecture suivante remplaçait
      par les défauts, en silence. Même piège pour les raccourcis, les
      **notes par binaire** (les noms donnés aux fonctions, parfois des
      heures de lecture) et le source de l'utilisateur. Écriture à côté,
      `sync`, puis renommage atomique : `write_atomically` dans
      `resources.rs`. Contrôle : `un_fichier_reecrit_est_remplace_d_un_bloc`.

---

## 1. Interface — corrigé

- [x] Les fenêtres reviennent ouvertes d'une séance à l'autre, en
      escalier (A 7.11)
- [x] Les trois zones du panneau de droite ont le même en-tête et le même bord
- [x] Un titre de zone se souligne, il ne s'encadre pas
- [x] Sept boutons sans liseré encadrés — et le contrôle qui les
      manquait, réparé
- [x] Deux boutons voisins ont la même hauteur
- [x] Les points de suspension sont des points de suspension (dix libellés)
- [x] Plus aucun écran ne montre son balisage (`**gras**` rendu)
- [x] Ce qui se dit « chasse fixe » l'est vraiment (quatre textes)
- [x] Préférences › Général garde un seul rythme
- [x] Une section dépliée dit toujours quelque chose
- [x] Deux mots identiques ne se touchent plus (« 100 % » → « Taille réelle »)
- [x] L'état du processeur ne défile jamais, à toute taille (A 7.11)
- [x] Le bandeau ne coupe plus de ligne en deux (A 7.11)
- [x] « Prochaine instruction » part repliée devant un binaire (A 7.11)
- [x] Chaque panneau vide dit ce qu'il montrera, dans ses propres mots
- [x] Les icônes ne se peignent plus en bouillie
- [x] Tout texte se détache de son fond (contraste mesuré, tous les thèmes)
- [x] Les boîtes de dialogue parlent la même langue (A 1.24)
- [x] Les fenêtres internes disent leur rang (A 2.1)
- [x] Six fenêtres s'agrandissent (A 2.9)
- [x] **Une commande grisée dit pourquoi elle l'est** — `on_hover_text`
      ne s'affiche pas sur un widget éteint, si bien que cinq boutons de
      la barre d'outils et six entrées de menu ne disaient rien au
      survol. C'est l'écran qu'on a sous les yeux tant qu'on n'a pas
      assemblé.
- [x] Un changement d'échelle de l'écran redemande une image
- [x] **On voit qu'on peut ouvrir plusieurs fichiers** — la barre
      d'onglets est là **dès le premier écran**, tampon « sans titre »
      compris, avec un « + » au bout. Elle n'apparaissait qu'à partir du
      second fichier : rien, jamais, ne disait que le mécanisme
      existait. Premier remède insuffisant — « dès le premier fichier
      OUVERT » — parce que l'écran d'accueil n'en a pas.
- [x] **La barre d'onglets ressemble à une barre d'onglets** — des
      rectangles francs collés à l'éditeur, liseré d'accent sur l'actif,
      croix à l'intérieur, rond du non-enregistré à sa place. C'étaient
      trois objets qui flottaient : une pastille arrondie, une croix
      détachée, un plus.
- [x] **Le « + » crée un programme neuf** — il menait au sélecteur de
      fichiers, puis à un menu de deux entrées ; les deux fois c'était
      trop. Et « Nouveau programme » ne créait PAS d'onglet quand le
      tampon courant était vierge : le geste n'avait aucun effet
      visible.
- [x] **Les tampons sans titre se numérotent** — trois programmes neufs
      s'appelaient tous « sans titre ».
- [x] **« Fermer le fichier » ne disparaît plus du menu** — elle
      s'effaçait quand aucun fichier n'était ouvert, c'est-à-dire au
      moment où on la cherche. Grisée, avec sa raison.
- [x] **namsdec répond à sa ligne de commande** — `namsdec fichier.asm`
      ouvre le fichier, `--version` et `--help` répondent sans ouvrir de
      fenêtre. Le lanceur déclarait `Exec=namsdec %f` et
      `MimeType=text/x-asm` : « Ouvrir avec namsdec » ouvrait une fenêtre
      vide, ce qui est pire que de ne rien proposer.
- [x] **Remettre une fenêtre devant marche aussi au premier plan** — les
      trois fenêtres qui y vivent (plan d'installation, « À propos »,
      saisie) recevaient un ordre adressé au rang ordinaire, donc à un
      calque vide portant leur nom.
- [x] « Touche simple… » et « Séquence… » sont des boutons encadrés — et
      le contrôle voit maintenant la troisième façon d'écrire un bouton,
      `egui::Button::new` posé à la main
- [x] Le plancher du bandeau suit la hauteur réelle de l'état du
      processeur (335 px) au lieu d'une constante de 280
- [x] La zone du bandeau prend la place qui lui reste et fait défiler ce
      qui n'y tient pas, au lieu d'épouser son contenu et de déborder
- [x] **Fichier > Fermer le fichier** marche aussi sur le dernier
      fichier ouvert, et l'entrée ne disparaît plus du menu

## 2. Interface — ce que la revue a mesuré et pas corrigé

Détail chiffré dans `AMELIORATIONS.txt` 7.11, section « REVUE DU
25 SEPTEMBRE — CE QUI RESTE ».

- [x] **Huit fenêtres plus grandes que ce qu'elles montrent** — une
      fenêtre s'ouvre maintenant à la taille de ce qu'elle a à montrer,
      et le contrôle mesure le plus grand vide entre deux bandes de
      contenu. Le graphe est passé de 863 px de trou à moins de 80.
- [x] **« Registres étendus » est trop grande même pleine** — 440 x 480
      pleine au lieu de 520 x 620, et 540 x 220 à vide.
- [x] **Les états vides ont deux formes** — une seule maintenant : le
      constat, puis l'explication, centrés l'un sur l'autre. Cinq
      fenêtres converties. Exception nommée : « Aucun décompilateur
      installé. » est un mode d'emploi, pas un état vide.
- [x] **Le même fait s'écrit de quatre façons** — deux maintenant :
      « Aucun binaire assemblé. » pour le bandeau et ses onglets,
      « Aucun programme lié. » pour les fenêtres qui flottent par-dessus.
      Deux, et non une : les trois pourraient être à l'écran en même
      temps, et le contrôle des répétitions veille.
- [x] **Le listing de désassemblage perd ses colonnes de 54,8 px** — la
      colonne des étiquettes a une largeur fixe, prise sur le plus long
      symbole et bornée à vingt signes.
- [x] **La barre de saisie de la console** : 4,5 px d'amplitude sur une
      seule rangée — l'étiquette venait en premier et se centrait sur sa
      propre hauteur. Le seul remède est que le premier élément soit
      déjà le plus haut : trente-quatre pixels, mesurés, et un contrôle
      les retient.
- [x] **Étiquette contre liste déroulante** : 3,5 px, partout où les
      deux se suivent. Six façons d'entourer la liste d'egui ont été
      essayées, l'écart n'a pas bougé d'un pixel — il est DANS
      `ComboBox`, dont le cadre part de `available_rect_before_wrap()`.
      Nous avons donc la nôtre : un cadre alloué et peint, le texte
      aligné à gauche, le chevron dessiné à droite. Elle se centre comme
      tout le reste, elle a le liseré des autres commandes, et un
      contrôle exige que l'étiquette et sa valeur se lisent sur la même
      ligne.
- [x] **Deux marges gauches dans le châssis**, 16 et 18 px — examiné,
      ce n'est pas un défaut : l'onglet « Console » est une pastille
      cliquable, son fond part à 16 comme le corps et son texte à 18
      parce qu'une pastille a un rembourrage. Le relevé comparait un
      texte nu à un texte encadré.
- [x] **La grille de bits de la calculatrice est en 12 pt** — elle prend
      maintenant le style Monospace du thème, comme les registres, la
      pile et le vidage. La case d'un bit fait neuf pixels quelle que
      soit la police : rien n'a bougé en largeur.
- [x] **Le graphe écrit ses instructions en 11 pt**, et « le programme
      est ici » y est en chasse fixe — examiné, gardé : dans une boîte
      où tout est en colonnes, l'annotation est cohérente avec son
      contexte, et la largeur des boîtes se calcule en colonnes de
      caractères. Changer la police casserait ce calcul.
- [x] **Le vide sous « Prochaine instruction »** — la cause n'était pas
      dans le partage, où trois tentatives l'avaient cherchée. Le panneau
      croyait que la poignée avait été tirée dès que la première image ne
      rendait pas la hauteur demandée, ce qui arrive toujours : cette
      image-là ne connaît encore ni la hauteur du bandeau ni celle de
      l'état du processeur. La part de l'onglet était gelée à son
      plancher pour toute la séance, et le bandeau gardait le reste. Une
      poignée ne bouge que sous un doigt.

## 2 bis. Le chevauchement des captures — élucidé

- [x] **Ce n'était pas un défaut de mise en page.** Les deux captures du
      25 septembre au soir montraient l'état du processeur et le haut de
      l'onglet Mémoire sur les mêmes pixels. Mesuré SUR L'IMAGE : « RIP »
      et « Carte de la mémoire » font 33 et 31 pixels de haut, les
      textes superposés à eux en font **dix-huit**. Deux échelles sur
      une même image — egui n'en dessine qu'une. C'est une bande restée
      d'un rendu précédent, plus petit, que rien n'a repeinte : le
      facteur d'échelle de l'écran avait changé entre les deux.
      Le banc ne pouvait pas le retrouver : il dessine à une échelle et
      une seule. Quatre hypothèses de mise en page ont été essayées
      avant celle-là ; deux ont trouvé de vrais défauts au passage (le
      plancher du bandeau, la zone qui débordait au lieu de défiler),
      deux ont été réfutées par la mesure.
      **Fait depuis** : un changement d'échelle redemande une image, ce
      qui évite d'attendre que le compositeur se réveille. On ne répare
      pas le compositeur, on ne l'attend pas non plus.

## 2 ter. Découvrabilité — première passe

- [x] **Toute action a un chemin qui se voit** — huit actions sur trente
      n'avaient que le clavier : dupliquer une ligne, surligner les
      occurrences, aller à la suivante et à la précédente, les renommer
      toutes, nommer une adresse, la commenter, revenir au fichier
      précédent. Elles sont dans les menus Édition et Outils, chacune
      avec son raccourci. Un contrôle le retient — c'est le pendant, du
      côté de l'œil, de « toute action a un raccourci dans les deux
      jeux ».
- [x] **Les quatorze fenêtres ont toutes un chemin** — vérifié : chacune
      s'ouvre depuis un menu ou un bouton, aucune n'est réservée à la
      palette.
- [x] **Ce qui se clique change le pointeur** — douze étiquettes
      cliquables, aucune ne le disait. Les infobulles annonçaient déjà ce
      qu'un clic ferait, mais il faut s'être arrêté dessus pour les lire,
      et l'on ne s'arrête que sur ce qu'on soupçonne d'être cliquable.
      egui réserve la main aux liens ; `cliquable` la pose, et un
      contrôle l'exige.
- [x] **Les poignées disent qu'elles se tirent** — egui pose le
      pointeur de redimensionnement, mais le lien « Registres étendus »
      touchait le bord du panneau et réclamait la main : le pointeur
      disait « cliquable » là où il fallait tirer. Une poignée qui ment
      est pire qu'une poignée muette. Deux pixels d'air le règlent — pas
      quatre, qui coupaient « Détail » à 1280x820.

## 3. L'éditeur

- [x] Fermeture automatique des paires, réglable (A 4.14)
- [x] L'autocomplétion se pilote — Tab, flèches, Échap, clic (A 4.14)
- [x] Duplication de ligne (A 4.14)
- [x] Surligner les occurrences, par jetons et non par texte (A 4.14)
- [x] Aller d'une occurrence à l'autre, sélection prête à remplacer (A 4.14)
- [x] Renommer toutes les occurrences (A 4.14)
- [x] Coller insère au curseur au lieu de tout remplacer
- [x] La gouttière montre au survol ce qu'un clic y ferait
- [x] La ligne où le programme est arrêté porte une bande
- [x] Les compteurs de vim — `5j`, `3w` (A 1.2, 7.13)
- [ ] **Le multicurseur proprement dit** (A 4.14) — vérifié dans egui
      0.36 : `TextEditState` n'a qu'un seul champ `cursor`. En tenir
      plusieurs demande de remplacer le widget de texte, ou d'écrire une
      couche qui rejoue chaque frappe à chaque position. Ce qui précède
      couvre son usage réel.

## 4. Fonctions

- [x] Patcher un binaire — inverser, forcer, neutraliser (A 4.1, 7.3)
- [x] Nommer les fonctions et les adresses (A 4.2)
- [x] Commentaires sur le listing (A 4.3)
- [x] Suivre une valeur à rebours (A 4.4, 7.5)
- [x] Points d'arrêt conditionnels sur expression (A 4.6)
- [x] Le graphe s'exporte en DOT (A 4.7)
- [x] Vue hexadécimale éditable (A 4.8)
- [x] Chercher dans la mémoire (A 4.9, 7.4)
- [x] Calculatrice multi-bases (A 4.10)
- [x] Reprendre le travail après extinction (A 4.11)
- [x] Chronologie de l'exécution (A 4.12)
- [x] Entropie dans l'onglet RE (A 4.13)
- [x] DZF branché comme désassembleur (A 4.16) — et sa limite levée le
      29 septembre : `{text}` lui donne les octets exécutables seuls,
      `{base}` leur adresse. Il ne désassemble plus l'en-tête ELF comme
      du code, sans avoir rien eu à changer. Le contrat vaut pour tout
      outil qui ne lit pas l'ELF, ce qui est le cas du premier
      désassembleur que quiconque écrit.
- [x] `keymap.json` surveillé et exportable en entier (A 4.5)
- [x] Les raccourcis, repris par famille puis par le sens (A 1.23, 7.1)
- [x] **L'onglet Octets est éditable** — devant un binaire ouvert, un
      clic sur les octets d'une instruction ouvre un champ ; Entrée
      valide, Échap annule. Même chemin que les trois gestes du
      patcheur, donc la fenêtre Correctifs les liste et sait les
      défaire. La longueur ne change pas, et c'est refusé plutôt que
      rattrapé.
- [x] **La provenance à travers la mémoire** — une valeur rangée sur la
      pile puis relue se remonte jusqu'à elle. C'est le chemin ordinaire
      de tout argument dans du code compilé sans optimisation, et la
      remontée s'y arrêtait net. Deux pistes sont suivies, non une :
      QUELLE case (les registres de l'adresse) et QUOI dedans (la
      dernière écriture). Une écriture vers une case qu'on ne sait pas
      distinguer de la nôtre coupe la piste — `[rax]` vaut peut-être
      `[rbp-8]`, et rien dans le code ne le dit.

## 4 bis. Les mots de l'interface

- [x] **Un mot par question, pour les outils** — trois questions se
      mélangeaient en autant de vocabulaires (« allumer », « s'en
      servir », « programme détecté »). Elles sont nommées séparément :
      le programme est **installé sur la machine**, il est **activé dans
      namsdec**, et le module d'installation est activé. Les trois sont
      indépendantes, et c'est tout le sujet.
- [x] **« Recommencer » retiré d'un outil qui répond** — une offre sans
      motif : vingt minutes de compilation pour un résultat identique, et
      l'on finissait par douter que l'installation ait abouti. À la place,
      **« Réparer »** dans le seul cas où relancer sert : l'outil là mais
      incomplet, c'est-à-dire rizin sans `rz-ghidra`.
- [x] **Six fiches ne tiennent pas dans la fenêtre** — trouvé par la CI,
      que ma machine cachait : elle a tout d'installé, le coureur n'a rien.
      Une fiche absente écrit une ligne de plus, et les fiches se
      dépliaient d'office — RetDec et DZF passaient sous le bord sans
      qu'aucune barre ne le dise. « Pour l'obtenir » sort du repli (c'est
      une panne, pas un détail), le repli reste fermé, et le nombre
      d'outils connus est écrit à côté du titre. Le contrôle sème
      désormais le pire cas — **rien** d'installé — et vérifie que les six
      sont atteignables en déroulant.
- [x] **DZF dans la liste**, bien qu'en cours d'écriture. Deux contrôles
      le refusaient : l'un exigeait `{file}` alors que DZF reçoit
      `{text}`, l'autre lisait le `-S` de `cmake -S .` comme celui de
      `pacman -S` et allait demander aux dépôts s'ils connaissent un
      paquet nommé « . ».
- [x] **Le dialogue qui montrait une sortie sous un titre d'entrée** —
      « Entrée du programme » gardait ce titre après la réponse. Il
      s'intitule « Ce que le programme a écrit » dès qu'il ne recueille
      plus rien.
- [x] **Le profil dit combien de lignes sont classées** — le défilement
      marchait, et c'est mesuré : douze tours de molette atteignent la
      dernière rangée. Le grief « je ne sais pas me déplacer » était de
      la découvrabilité, pas de la mécanique : rien ne disait qu'il y
      avait une suite, et l'on ne cherche pas à dérouler ce qu'on croit
      avoir vu en entier.
- [x] **Les scripts annoncent avant d'agir** — `install.sh` et
      `uninstall.sh` montrent en tête les réglages retenus, avec l'option
      qui annule chacun. On découvrait après coup qu'ils avaient touché au
      fichier du shell ou épinglé au dock. Cadre, barre d'avancement et
      bilan, et un contrôle qui vérifie l'alignement du cadre — un titre
      accentué le décalait, parce que `printf '%-*s'` compte les octets.

## 4 ter. Les contrôles qui lisaient la machine

- [x] **Quatre CI rouges pour un seul contrôle** (et non pour la mise en
      forme : `cargo fmt --all --check` passait). Il semait l'état des
      outils, mais ouvrir la fenêtre d'installation jette ce relevé par
      conception — on a pu installer l'outil à la main dans un terminal
      —, et l'image suivante le refaisait en lançant vraiment rizin :
      installé ici, absent du coureur. Il sème maintenant **après la
      première image**, et couvre les trois états au lieu d'un.
- [x] **L'onglet Outils** avait le même défaut, révélé en y ajoutant
      DZF : il lisait la machine pour savoir s'il devait vérifier quoi que
      ce soit, et ne vérifiait donc rien sur un poste bien équipé.
- [x] **Toute la classe, et non les instances** (A 3.1) — le défaut
      était dans **quatre** contrôles, pas un. Trois autres lisaient
      l'état des outils pour choisir leur assertion : le plan
      d'installation, le panneau des outils, et celui du greffon
      manquant. Ce dernier n'éprouvait **jamais** le cas « incomplet »,
      le seul qui l'intéressait, parce que rizin fonctionne sur cette
      machine. Un contrôle qui s'adapte ne signale pas qu'il a cessé de
      voir.
- [x] **La règle de A 7.6, enfin tenue** — 58 tests peignaient sans poser
      le thème, dont 11 mesuraient une géométrie sous des espacements qui
      n'existent nulle part. Les 11 posent le style, `paint` le pose pour
      tous, et `a_control_that_measures_geometry_sets_the_style_first`
      refuse qu'un test lise une position, une taille de galley ou une
      forme peinte sans l'avoir posé. Les 47 autres ne cherchent qu'un mot
      : l'exiger d'eux serait du bruit, et la règle le dit.
      Relecture faite aussi des 44 lectures de `.pos` : elles combinent
      presque toutes position et taille, donc un rectangle. Rien à
      réécrire — c'est un résultat, pas un renoncement.
- [x] **`no_control_asks_the_machine_what_it_should_be_checking`** — le
      contrôle qui empêche la récidive. Il lit la source des contrôles —
      `SOURCES_DU_BANC` depuis le découpage du 22 octobre 2026, `tests.rs`
      avant lui —, découpe par
      `#[test]`, et refuse `availability_in`, `.availability()` et
      `tool_availability` dans un corps de test. Deux exceptions nommées
      avec leur raison — vérifier une relation vraie partout, vérifier la
      mécanique du relevé —, et la liste blanche est elle-même vérifiée :
      un nom qu'on y laisse après avoir renommé le test ferait croire à
      une exception qui n'existe plus. Éprouvé en remettant le défaut.
- [x] **Le guide que le README annonce** — le lien pointait vers un PDF
      que trois montées de version avaient emporté. Un contrôle exige
      maintenant qu'il porte la version courante, que le fichier existe,
      et qu'aucun autre ne traîne à côté.

## 4 quater. La mise en page du listing, mesurée

- [x] **Les 280 ms, chiffrées et rangées** (A 5.3) — sur 15 000 lignes :
      coloration 40 ms, **pose des glyphes 285 ms**, total 325. La pose est
      87 % du coût. Sur le crackme (62 386 lignes) : première image 561 ms,
      puis 2,4 ms en régime, et **une seule pose** pour tout le chargement.
      Le cache fait son travail ; le prix est celui d'un chargement.
- [x] **`opening_a_binary_lays_out_its_listing_once`** — retient le « une
      fois ». Une régression qui reposerait à chaque image ne se verrait pas
      sur un source de vingt lignes et rendrait l'application inutilisable
      sur un binaire : une demi-seconde par image.
- [x] **Piste fermée : fusionner les sections de même format.** Aucun gain
      — `egui::LayoutJob::append` le fait déjà (epaint 0.36.2). Les 120 000
      sections mesurées sont déjà le résultat de sa fusion, huit par ligne
      au lieu de dix. Le refaire coûtait 22 ms de plus. Retiré, et le
      commentaire de `append` le dit pour la prochaine fois.
- [x] **Ce qui restait — ne poser que les lignes visibles — est fait**
      (A 7.14, section 8) : le 14 octobre 2026, en mode Normal, où le
      champ de texte était déjà non interactif. Ce que cette entrée
      craignait de perdre (sélection, recherche, curseur, marques)
      n'existait pas dans ce mode-là.
- [x] **Et ce qui se payait à CHAQUE image, lui, est corrigé** — voir la
      section 7 : le régime sur un listing d'un million de lignes passe de
      11,3 ms à moins de 2 ms. Le relevé de 5.2 ne l'avait pas vu parce
      qu'il ne mesurait que crackme-01, le plus petit binaire du dossier
      d'épreuve.

## 4 sexies. Robustesse sur entrée hostile

- [x] **Panique corrigée dans `imports::scan`** — un fuzzing (40 000 ELF
      malformés) a trouvé deux paniques : `sym * sym_entry` débordait,
      `sym` venant d'un champ quelconque du fichier. Corrigé par
      arithmétique vérifiée, comme le reste du module. C'est la première
      exigence d'un outil qui reçoit des fichiers malveillants : ils sont
      délibérément malformés. Invariant tenu par
      `a_hostile_elf_never_panics_the_scan`. Reconfirmé : 0/40 000.
- [x] **L'invariant étendu à TOUS les analyseurs** (C2) — un contrôle
      d'intégration, `tests/hostile.rs`, fait passer une vingtaine
      d'analyseurs, le désassembleur et les deux rapports sur des ELF
      abîmés de trois façons : octets changés, troncature, et valeurs
      extrêmes posées sur huit octets alignés. Le germe est **fabriqué**
      — en-tête, segments, sections, symboles, relocations, entrées de
      liaison — et non lu sur le disque : le contrôle d'origine partait
      de `/bin/true` et passait sans rien prouver sur une machine qui
      n'en a pas.
      **Quatre paniques trouvées et corrigées** : `imports::file_offset`
      (deux sommes d'adresses), `slack` (le nom de section et l'adresse
      virtuelle d'un trou), et surtout les **trois lecteurs** `u16_at`,
      `u32_at`, `u64_at` d'`elf_scan` et d'`imports` — `at + 4` déborde
      quand `at` vient d'un champ du fichier, et toutes les additions de
      décalage de champ sont désormais saturantes. C'était une classe de
      défauts, non un défaut.

## 4 septies. Lire un échantillon : ce qui ressort, et ce qu'on en recopie

La séance du 1er octobre, menée sur le relevé `septembre26.md`. Tout
s'appuie sur du déjà-calculé : aucun de ces panneaux ne relit le fichier
pour son compte.

- [x] **Les familles de comportement** (A1) — `behaviour.rs` range les
      noms en six familles : réseau, exécution, anti-débogage, mémoire,
      chiffrement, fichiers. Une seule table pour les imports **et** les
      appels système, parce qu'un `socket` est un `socket` qu'il vienne
      de la table de liaison ou du numéro 41 dans RAX — et qu'un
      programme qui veut passer inaperçu emploie justement le second.
      Une table de noms exacts et non des règles : « tout ce qui commence
      par `exec` » attraperait `execute_query`. Le portrait se dessine en
      tête des deux panneaux, avec les mêmes couleurs.
      Ce qui n'y est **pas** : `raise`, `alarm`, `sigaltstack`,
      `rt_sigaction`. La capture d'écran a montré `/bin/ls` allumant
      « Anti-débogage » sur `raise` — un tableau de bord qui crie sur `ls`
      n'est plus lu.
- [x] **Le panneau « Signaux »** (A2) — la synthèse, en seconde position
      dans la colonne. Il n'analyse rien de neuf : empaqueteur reconnu,
      segment W+X, entropie haute sur une **grande** tranche, protections
      absentes, familles présentes, code auto-modifiant possible,
      indicateurs. Chaque ligne dit un fait, ce que ce fait change, et le
      panneau qui l'explique. **Rien n'est additionné en score** : un
      score donnerait l'illusion d'un verdict.
      Un contrôle garde le panneau utilisable : sur `/bin/true` et
      `/bin/ls`, aucune ligne « Remarquable ».
      La tranche d'entropie y est mesurée sur 4 Kio au moins, et non sur
      les soixante-quatrièmes du panneau Entropie : la queue de
      `/bin/true` fait 505 octets et mesure 7,61 sans rien cacher.
- [x] **Les indicateurs dans les chaînes** (A3) — `ioc.rs` : adresses
      web, adresses IP, noms de domaine, courriels, chemins absolus, clés
      de registre. Ce sont les seules choses d'un binaire qu'on puisse
      aller vérifier **ailleurs**. La règle est écrite pour se taire
      plutôt que se tromper : une liste de domaines de premier niveau
      connus, et non « tout ce qui contient un point » — sans quoi
      `libc.so.6` passerait pour un domaine et la liste serait
      inutilisable. Chaque rubrique se copie d'un clic.
- [x] **L'empaqueteur nommé** (A4) — `packer.rs` : UPX, PyInstaller,
      AppImage, par nom de section ou nombre magique, avec **la preuve**
      et ce qu'on en fait (`upx -d`, `pyinstxtractor`). Le nombre magique
      d'AppImage n'est cherché qu'à sa place, au décalage 8 : trois
      octets se rencontrent par hasard partout.
- [x] **Les seuils d'entropie rassemblés** (C3) — `entropy::seuils`, avec
      la mesure qui justifie chacun. Ils étaient écrits en clair dans un
      `match` et recopiés dans le panneau.
- [x] **Le rapport porte tout cela** — signaux, portrait, familles sur
      chaque import, indicateurs par nature. C'est la **même** fonction
      qui produit les signaux du rapport et ceux du panneau.
- [x] **Le relevé de ce qui est peint** — `tests/panneaux.rs` mesure les
      cinq panneaux touchés, à deux largeurs et dans les deux langues :
      rien ne court au-delà d'un bord, rien ne se recouvre, rien ne bouge
      tout seul, et aucune phrase n'est trouée d'espaces. Ce dernier
      contrôle vient d'un vrai défaut : un littéral Rust coupé sur
      plusieurs lignes garde ses espaces d'indentation quand le `\` de
      continuation disparaît.
- [x] **Le désalignement volontaire** (A5) — vérifié d'abord, mesuré :
      le désassemblage de namsdec est **linéaire**, et c'est le bon choix
      pour un listing. Sur la figure classique (`jmp $+3` / `db 0xE8` /
      `xor eax,eax`), il rend `call 0FFFF…h` puis `(bad)` là où le
      processeur lit `xor eax,eax ; inc rax ; ret` — et le graphe de flot
      perdait l'arête sans rien dire.
      `misalign.rs` relève les endroits où un saut atterrit **au milieu**
      d'une instruction du listing, et le panneau « Figures du code »
      donne les **deux lectures côte à côte**. Il ne corrige pas le
      listing : un listing troué serait pire que trompeur, et nommer
      l'endroit suffit à ne plus se faire avoir.
      Deux bornes, chacune venue d'une mesure sur `/bin/ls` : seules les
      sections **exécutables** comptent (une chaîne de format décodée
      comme du code rend des sauts qui tombent au hasard), et le saut doit
      tomber à moins de seize octets (la figure cache un ou deux octets,
      pas soixante). Sans elles, cinq figures relevées sur `/bin/ls`, et
      aucune vraie.
      Relevé aussi dans les signaux et dans le rapport, avec les deux
      lectures.

## 4 octies. Circuler : d'un panneau d'analyse au listing

- [x] **Le compte d'appels des imports, réparé** — le panneau annonçait
      105 fonctions importées et **zéro** appel vers chacune sur un
      binaire ouvert. Le comptage convertissait l'adresse d'une entrée de
      liaison en position dans le fichier, parce que le commentaire
      d'`imports_module` dit que « le désassemblage part du premier octet
      du fichier ». C'est vrai d'un tampon brut et **faux** d'un ELF
      ouvert : `load_elf` ne désassemble que les parties exécutables,
      chacune à son adresse virtuelle. Les deux conventions sont essayées
      maintenant ; mesuré sur `/bin/ls` : 719 appels au lieu de 0.
      Un compte faux se lit comme un compte vrai : c'est pourquoi le
      défaut a tenu. Le contrôle qui le retient charge le binaire **par
      l'application**, et non en reconstruisant le listing à sa façon.
- [x] **Les adresses des panneaux sont cliquables** — une entrée de
      liaison, une zone libre, une figure, un fragment, un désalignement :
      cliquer l'adresse amène le listing dessus, par
      `ReRequest::GoTo`. Le suivi de RIP s'arrête au passage, sans quoi
      le pas suivant ramènerait le listing là où l'exécution en est.
      Une adresse entre deux lignes mène à la ligne juste avant — c'est
      le cas d'un désalignement volontaire ; une adresse dans aucune
      partie désassemblée le dit plutôt que de ne rien faire.
      Le contrôle **clique pour de vrai** : il peint le panneau, cherche
      où l'adresse a été posée, et envoie un appui puis un relâchement.
      Vérifier que le code « poserait une demande si on cliquait » ne
      prouverait pas qu'on peut cliquer.

## 4 nonies. Les raccourcis, et ce que le banc ne regardait pas (A 7.1)

- [x] **Les deux jeux côte à côte** — l'onglet n'en montrait qu'un, et
      c'est précisément ce qui empêchait de voir les trous et les
      doublons : on ne compare pas de mémoire deux listes de trente
      lignes. Le jeu actif en clair, l'autre à côté ; « la même » quand
      les deux disent la même chose, parce que c'est ce qui DIFFÈRE qu'on
      vient lire ; un raccourci déplacé soi-même est marqué et dit au
      survol ce que le jeu proposait ; deux actions qui répondent à la
      même frappe se voient en rouge.
      Le rangement par famille (`Espace p…`, `Espace v…`) était déjà fait
      ; c'est ce tableau qui manquait pour en juger.
- [x] **Les séquences s'écrivent `Space P I`** et non `Space , P , I`.
      Les deux formes côte à côte sur trente lignes ont tranché : la
      virgule entre deux espaces n'ajoute rien. Le projet écrit d'ailleurs
      « Espace p i » partout dans sa prose.
- [x] **Les cinq pages des préférences entrent au banc** — seule la
      première y était, la seule qui tienne sans défiler. Les quatre
      autres n'avaient jamais été mesurées, et deux défauts y
      dormaient.
- [x] **La boîte de préférences grandissait d'un pixel par image, sans
      fin** — défaut antérieur, trouvé par le banc dès que les autres
      pages y sont entrées. La zone qui défile prenait sa hauteur sur la
      fenêtre ; la fenêtre prenait la sienne sur son contenu. Trois
      essais : borner la zone sur le viewport coupe la boucle mais fait
      sortir « OK » par le bas ; borner la fenêtre par le haut laisse
      deux cent quarante images de reptation ; **imposer** la hauteur de
      la fenêtre répond aux deux. Un contrôle garde la cause, page par
      page, et il échoue sur l'ancien code.
- [x] **Un contrôle qui lisait l'horloge** — « ouvrir un binaire ne pose
      son listing qu'une fois » gardait un budget de 40 ms par image. Il
      a échoué deux fois sous une suite complète alors qu'il passait
      seul : une machine chargée mesure mal. Il compare maintenant un
      RAPPORT — les images d'après coûtent moins de la moitié de la
      première — ce qui ne dépend plus de la charge, les deux mesures la
      subissant ensemble. Le compte de poses, lui, était déjà
      déterministe et ne change pas.
- [x] **Une phrase pour deux rôles** — « Aucun binaire assemblé. » était
      à la fois l'état vide d'un panneau (un bloc centré) et l'état d'une
      fiche de module (une ligne à gauche). C'est la même maladie que
      deux formes pour un rôle. La fiche dit maintenant « rien à
      désassembler pour l'instant ».

## 4 quinquies. Modifier un binaire — premier morceau

Fred a rappelé la vocation : l'onglet RE apprend la rétro-ingénierie,
comme Ghidra ou un CTF. La fonctionnalité annoncée — modifier le binaire
ouvert pour y ajouter ou retirer quelque chose — a un mur : tout ce que
`patch.rs` sait faire garde la longueur exacte, donc on peut retirer,
jamais ajouter. Il faut de la place, et d'abord savoir où elle est.

- [x] **`slack` : la place libre d'un binaire** — lecture seule. Parcourt
      les segments chargeables et repère les longues suites d'un même
      octet de bourrage (0x00, 0x90 nop, 0xcc int3) que l'éditeur de liens
      laisse pour aligner. Robuste sur un binaire dépouillé : le bourrage
      se reconnaît à lui-même, sans en-têtes de section. Borné par
      `filesz`, jamais `memsz` — `.bss` n'occupe rien sur le disque.
- [x] **Le panneau `slack`** — dans la grappe pédagogique, juste après les
      fragments ROP : là où ceux-ci montrent qu'un binaire contient déjà du
      code réutilisable, celui-ci montre qu'il contient de la place
      inutilisée. L'octet de bourrage est NOMMÉ (« 0xcc (int3) ») et non
      seulement chiffré. Vérifié sur `/bin/true` et `/bin/ls` : 1 906 et
      6 280 octets libres.
- [x] **La section qui porte chaque zone** — `.text`, `.plt.sec`, `.rodata`…
      C'est elle qui raconte l'histoire : 50 octets libres dans `.plt.sec`
      n'est pas 245 en fin de `.rodata`. `None` sur un binaire dépouillé ou
      pour un trou entre deux sections. `.bss` (qui n'occupe rien) ne nomme
      jamais un trou, bien que son adresse chevauche celle d'une vraie
      section.
- [x] **Le détour est bâti, et il S'EXÉCUTE** (A 9.12) — l'entrée
      ci-dessus se donnait pour « non commencée », et elle avait dérivé :
      le module `DetourModule` est complet (saisie en assembleur ou en
      octets, choix de la place libre exécutable, aperçu du plan, refus
      nommés), `plan_detour` rend deux `Patch` réversibles sous un même
      groupe, et `write_detour` les pose par le chemin ordinaire des
      correctifs. Une douzaine de contrôles l'éprouvaient.
      CE QUI MANQUAIT : aucun ne faisait TOURNER ce que le plan fabrique.
      Ils vérifiaient les octets et l'arithmétique des sauts — justes —,
      mais jamais que l'idée s'exécute. `a_planned_detour_runs_in_the_emulator`
      monte un détour À LA MAIN (sans nasm, donc sans abstention),
      l'applique, et lance l'émulateur : le code ajouté tourne (rbx), ce
      qu'on a recouvert est rejoué, et — le site étant `mov eax, [rip+…]`
      — sa relocalisation est prouvée À L'EXÉCUTION, eax lisant la même
      globale qu'avant le détour. Éprouvé en corrompant le déplacement
      relocalisé : eax lit alors ailleurs, et l'oracle rougit.

## 4 sexdecies. DZF : namsdec sait l'installer lui-même

- [x] **Recette d'installation de DZF** (7 octobre 2026) — DZF vit dans
      son propre dépôt (`github.com/fredza/DZF`), jamais en dépendance.
      `recipe_for("dzf")` le clone, le configure et le construit en
      C++23 avec `cmake` (`DZF_BUILD_TESTS=OFF`), puis copie
      `x64-disassembler` et `dzf-plugin-engine` dans `<tools>/dzf/bin/` —
      `cmake` n'installe que la bibliothèque, pas les exécutables. Rien
      hors du dossier de namsdec, aucun `sudo`, chaque commande montrée
      avant d'être lancée. Même patron que Rizin et RetDec.
- [x] **Le bouton Installer… apparaît pour DZF** — dès qu'une recette
      existe (`recette_existe`), la fiche de DZF porte le bouton, comme
      les autres. La chaîne d'installation manuelle est corrigée (vrai
      dépôt, bon chemin de build), `dzf.json` régénéré, manuel et README
      des fonctionnalités à jour.

## 4 septdecies. L'onglet RE : graphe d'appels, aller-à, code et pseudo-code

Fred a demandé trois choses pour l'onglet RE : aller à une adresse, voir
les appels de fonctions en graphe (style UML), et des rubriques montrant
le code binaire et le pseudo-code. Un seul module neuf les réunit.

- [x] **Module « Graphe d'appels »** (`call_graph_module.rs`) — greffé
      juste après « Fonctions », dont il prolonge le travail :
      `functions::discover` donne déjà `calls`, c'est-à-dire les arêtes du
      graphe. Rien de neuf à calculer.
- [x] **L'arbre des appels, à la façon UML** — déplié depuis le point
      d'entrée (ou `main`), une boîte cliquable par fonction, un trait de
      liaison peint entre parent et enfant. Une fonction déjà dépliée
      devient un renvoi (`↩`) : l'arbre reste fini même sur du code
      récursif. Borné (40 fonctions, profondeur 8, 120 nœuds) — au-delà,
      un graphe n'est plus une carte.
- [x] **Aller à une adresse** — un champ qui lit l'hexadécimal (`0x…`) ou
      le décimal, et émet `ReRequest::GoTo` ; chaque boîte de l'arbre est
      aussi un raccourci. S'appuie sur le canal `GoTo` qui existait déjà.
- [x] **Rubriques code binaire + pseudo-code** — sous l'arbre, chaque
      fonction se déplie sur ses octets bruts (le code binaire) et leur
      traduction en clair (le pseudo-code, moteur `pseudo::of`), côte à
      côte. Instructions prises dans la plage de la fonction par recherche
      dichotomique, bornées à 200.
- [x] **Vérifié** — trois tests du module (lecture d'adresse, arbre fini
      sur récursion, instructions bornées à la plage) ; les invariants du
      banc (non-recouvrement, découpe, contraste) couvrent le panneau via
      la scène « Analyse ».

### Retour de Fred (7 octobre, après essai) — corrigé

- [x] **Le graphe se voyait comme une liste, pas comme un graphe.** La
      première version posait des boutons empilés et traçait des traits
      par-dessus ; aux couleurs discrètes du thème (fond `faint_bg`,
      traits `weak_text`), rien ne se lisait comme un diagramme. Réécrit
      en vrai canevas peint (`allocate_painter`, comme le déroulement CFG
      de `flow.rs`) : boîtes à fond et cadre francs, connecteurs fléchés
      parent→enfant, racine/`main` en accent. Clic sur une boîte → on y
      va. Mesuré : le panneau peint bien des boîtes, et les invariants du
      banc le couvrent.
- [x] **Cliquer une référence croisée ne menait nulle part.** Le module
      `xrefs` ne faisait que déplier les appelants — il n'émettait aucun
      `GoTo`. Désormais l'adresse est cliquable (y aller), le triangle
      déplie (d'où c'est appelé), et les adresses « appelé depuis » et
      « bouclée depuis » mènent aussi. `take_request` branché.
- [x] **« En bref » manquait de détails.** Le résumé dit maintenant ce
      que le binaire **emprunte** (fonctions + appels système), **ce
      qu'il sait faire** — le portrait en six familles, dont le réseau :
      la réponse à « a-t-il accès à Internet ? » se lit d'un coup d'œil,
      qu'il passe par la table de liaison ou par un numéro d'appel
      système —, et les **protections** qui décident de tout (PIE, NX,
      RELRO, canari), cochées ou barrées. Tout mesuré, rien déclaré.

- [x] **Renommer les fonctions** (demandé par Fred) — chaque fonction,
      dans le détail du graphe d'appels, porte un champ **Nom** : on
      remplace `sub_10450` par « vérifie_clé », « Défaut » rend le nom
      d'origine. Pas une table neuve : ça écrit dans les **notes** du
      binaire (`notes.names`, `ReRequest::Rename` → `set_name` + `save`),
      la même que le listing et le déroulement emploient déjà. Le nom se
      lit donc partout — arbre, liste des fonctions, listing — et il est
      **gardé d'une séance à l'autre** (persisté par empreinte du
      binaire). `ReContext` porte la table (`names`), les 29 littéraux
      complétés. Un test : un nom donné prend la place du défaut.
- [x] **L'éditeur binaire est fait** (A 9.6, 9.7) — la grille
      hexadécimale de la fenêtre d'analyse écrit les octets à la main,
      par `patch.rs` et `ReRequest::PatchBytes`, réversible comme les
      correctifs. Le défaut signalé le 21 octobre — « on ne sait rien
      modifier » — était le refus muet de l'octet du nombre magique,
      corrigé en 9.7 ; le refus se dit désormais et ne change rien.

## 4 octies. Découpage de main.rs (suite)

- [x] **main.rs : 6 643 → 4 231 lignes** (−2 412, −36 %), sans rien
      changer au comportement. Neuf modules neufs, chacun un thème net,
      chacun vérifié par compilation puis par la suite complète :
      - `icons.rs` (217) — les dix icônes dessinées à la main (peintre,
        rectangle, couleur ; dessins purs, testés au pixel).
      - `widgets.rs` (667) — barres de validation et de dialogue, boutons
        (texte, action, icône), cartes de thème et de module, `Answer`.
      - `labels.rs` (104) — les étiquettes du source (fonctions pures sur
        du texte, sans écran).
      - `resources.rs` (169) — dossiers de données et ressources
        installées (greffons, thèmes), et les deux libellés de région.
      - `profiling.rs` (82) — le diagnostic des images lentes : mesurer
        chaque étape d'une frame, et écrire celle qui a gelé l'interface.
      - `listing.rs` (~280) — le rendu du listing désassemblé : bande de
        chaleur, ligne sous le curseur, boutons-liens, étiquettes de
        liaison, annotation d'une ligne, cible d'un saut, nom d'adresse.
      - `enums.rs` (~530) — les onze énumérations d'état de l'interface
        (onglets, modes, formats, statut) et leurs libellés.
      - `patching.rs` (~285) — les méthodes de `NamsdecApp` qui modifient
        le binaire : prévoir un correctif, l'écrire (octets ou instruction
        assemblée), le défaire. Premier groupe de MÉTHODES extrait, sur le
        patron des modules frères (`impl NamsdecApp` + `use super::*`).
      - `exports.rs` (~155) — le texte que les vues produisent pour la
        copie ou l'enregistrement : contexte d'une ligne, listing entier,
        rapport d'analyse.
- [x] Les neuf ajoutés à `SOURCES_DE_L_INTERFACE` : les contrôles qui
      lisent le code source les voient désormais comme les autres.
- [x] **Reprise du 6 octobre : les méthodes des outils vers `tools.rs`**
      (−96 lignes, 4 272 → 4 176). `refresh_tool_availability`,
      `tool_availability`, `declare_tool`, `undeclare_tool` rejoignent le
      module où vit déjà toute la gestion des outils extérieurs. Même
      patron (`impl NamsdecApp` + `use super::*`), passées `pub(crate)`
      pour les frères qui les appellent (jobs, tests). Comportement
      inchangé, suite complète au vert.
- [x] **Puis la géométrie de la console vers `console.rs`** (−118,
      4 176 → 4 058). `console_wanted_height` et `console_input_estimate`
      — la hauteur que le panneau du bas et sa zone de saisie réclament —
      rejoignent le module de la console. Même patron, `pub(crate)` pour
      `draw` qui les appelle.
- [x] **Et l'entrée de la console : `console_shows_input` + `send_input`**
      (−46, 4 058 → 4 012). Donner au programme la ligne saisie (ou fermer
      son entrée), et dire si le champ doit paraître : les deux rejoignent
      `console.rs`. (`send_input` ne faisait que ~20 lignes, non ~440 — une
      borne mal lue la séance d'avant, corrigée ici.) La console tient
      désormais toute sa logique dans son module.
- [x] **Le cycle des préférences vers `preferences.rs`** (−41,
      4 012 → 3 971, main.rs passe sous 4 000). `open_preferences`
      (mémorise l'état des réglages pour que « Quitter » ait où revenir) et
      `close_preferences` (OK garde, Quitter restaure) rejoignent le module
      des préférences. `pub(crate)` pour menus et palette qui les appellent.
- [x] **Le curseur de l'éditeur vers `editor.rs`** (−56, 3 971 → 3 915).
      `current_line` (la ligne où le programme est arrêté), `apply_goto_line`
      (poser le curseur au début d'une ligne et y amener la vue) et
      `apply_pending_cursor` (replacer le curseur après une modification)
      rejoignent le module de l'éditeur. `pub(crate)` pour la barre de
      statut et l'éditeur qui les appellent.

## 4 nonies. Toutes les instructions nasm, visibles

- [x] **Le sommaire de l'aide montrait 113 têtes de famille pour 674
      mnémoniques** — `jmp` se voyait, mais `je`/`jne`/`jz`… étaient dans
      la doc sans être à l'écran : 561 instructions nasm qu'il fallait
      connaître pour les chercher. Trouvé sur les captures de Fred (2-3
      octobre). Le sommaire des instructions affiche désormais **tous les
      noms** de chaque fiche, chacun cliquable (nom surligné à
      l'ouverture), avec un compte en tête. Contrôle :
      `the_instruction_index_shows_every_mnemonic_not_just_the_family_head`.
      Manuel et PDF à jour.

## 4 quindecies. Gestion plugins/modules : première passe

- [x] **Onglet Modules clarifié** — le mot « module » couvrait deux choses :
      les COMPOSANTS (crates, santé, non allumables) et les FONCTIONS
      optionnelles (RE, exercices, décompilation, installation). Deux
      sous-titres les séparent désormais.
- [x] **Carte d'outil incomplet corrigée** — pour rizin « installé, mais
      incomplet », « Pour l'obtenir : sudo dnf install rizin » se
      contredisait (la base est là) et visait le mauvais paquet (ce qui
      manque, rz-ghidra, n'est pas un paquet dnf). La ligne n'apparaît plus
      que si l'outil est ABSENT ; pour l'incomplet, « il lui manque… » + le
      module d'installation (qui construit ce qui manque) suffisent.
      Contrôle : assertion ajoutée à `a_working_tool_is_not_announced…`.
- [x] **La décompilation éteinte est signalée dans l'onglet Outils** — les
      deux réglages d'une même fonction tombaient sur deux onglets (activer
      un outil dans Outils, allumer la décompilation dans Modules). L'onglet
      Outils le dit en tête quand elle est éteinte, avec un bouton pour
      l'allumer sur place. Contrôle : `the_tools_tab_says_when_decompilation_is_off`.
- [x] **Localiser un binaire bâti hors du PATH** — DZF compilé dans son dépôt
      restait « absent » : namsdec ne cherche que dans le PATH et son dossier
      d'outils, et un binaire laissé dans `DZF/x64-disassembler/build/` n'est
      ni l'un ni l'autre. La fiche d'un outil absent propose maintenant
      **Localiser le binaire…** : on désigne le fichier une fois, il est
      retenu (dans `settings.json`, clé `located_tools`), employé pour la
      détection (prêt si le fichier existe) et comme commande à la déclaration.
      Vaut pour tout outil rangé hors des chemins habituels, pas que DZF.
      Contrôle : `a_located_binary_makes_a_tool_ready`.
- [x] **L'éditeur suit l'exécution** — si l'on avait défilé vers la droite
      (pour lire le bout d'une longue ligne de commentaire), un pas gardait
      cette vue : ligne courante visible en hauteur, mais instructions et
      gouttière sorties à gauche, on ne voyait plus que les commentaires.
      Chaque pas remet le défilement horizontal à zéro, tenu quelques images
      pour absorber l'élan d'une molette lancée. Vu en vidéo (exj4.asm).
      Contrôle : `stepping_brings_the_code_back_into_view`.
- [x] **Les flags redeviennent de simples indicateurs** — ils étaient
      cliquables (un clic basculait l'indicateur quand un programme
      tournait), ce qui brouillait leur lecture et n'était possible que par
      intermittence. Désormais on les lit, on ne les règle plus : `Sense`
      au survol (l'infobulle explique), plus de clic. `toggle_flag` et la
      note « Indicateur basculé à la main » retirés ; le test
      `a_register_is_set_by_hand` ne couvre plus que le registre.
- [x] **Enchaînement Activer / Réparer d'un outil incomplet** — activer un
      outil incomplet (rizin sans rz-ghidra) déclarait un décompilateur qui
      échoue au premier essai (`pdg` n'existe pas). « Activer » est désormais
      **grisé** tant que l'outil n'est pas prêt (infobulle qui l'explique),
      et son bouton de complétion devient **« Réparer »** en accent (et non
      « Installer… », mot d'un outil absent). On ne peut activer qu'un outil
      prêt. Contrôle : `an_incomplete_tool_points_to_repair_not_enable`.
- [x] **Revue plugins/modules close** : frontières nettes (Composants/
      Fonctions sous Modules, Thèmes/Exercices sous Plugins, programmes sous
      Outils), fiche d'outil incomplet corrigée, décompilation éteinte
      signalée, binaire hors PATH localisable, et l'enchaînement
      Activer/Réparer réglé.

## 4 quaterdecies. Flags toujours affichés, éclat tenu plus longtemps

- [x] **Les sept flags s'affichent en permanence** dans la barre de titre,
      même avant d'assembler (tous à zéro, grisés, non cliquables tant
      qu'il n'y a pas d'émulateur). La barre restait muette au repos ;
      « tout doit s'afficher ».
- [x] **L'éclat orange dure ~3 s avec un maintien** — il reste à pleine
      couleur ~45 % du temps puis fond, au lieu d'une décroissance dès le
      premier instant. « Clignoter longuement. » `FLASH_SECONDS` 1,5 → 3,0,
      `FLASH_HOLD` = 0.45.

## 4 terdecies. Flags à droite du nom, et éclat plus long

- [x] **Les indicateurs à droite du nom du fichier** (et non à gauche) —
      au bout de la barre de titre, via une disposition droite-à-gauche.
      egui inversait alors l'ordre des pastilles ; on parcourt FLAG_BITS à
      l'envers pour que le double renversement redonne CF…OF. Mesuré :
      nom à ~470, pastilles de ~1110 à ~1358 sur une fenêtre de 1400.
- [x] **L'éclat orange dure 1,5 s** (au lieu de 0,6) — trop court, disait
      Fred. `FLASH_SECONDS` porté à 1.5.

## 4 duodecies. Les flags dans la barre de titre, le changement bien visible

- [x] **Les sept indicateurs déménagent dans la barre de titre** — c'est
      là que Fred les voulait (par « barre de titre »), pas sous un en-tête
      dans le panneau. La barre de titre est toujours visible et ne se
      replie jamais : on les a sous les yeux à chaque pas, à côté du nom du
      fichier. Clic pour basculer, survol qui développe nom/rôle/état,
      clignotement conservés. Retirés du panneau d'état (qui en ressort
      plus court — le repli libère ~81 px, seuil du test ajusté).
- [x] **Le changement d'un registre, bien plus visible** — un fond orange
      qui s'estompe derrière la valeur, à la place de la seule teinte de
      texte (trop discrète, dit Fred). Un bloc de couleur se voit d'un coup
      d'œil ; le texte reste clair dessus pour rester lisible pendant
      l'éclat. Manuel et PDF à jour.

## 4 undecies. Les indicateurs : une barre de titre, et chacun se présente

- [x] **Barre de titre « Indicateurs »** devant la rangée des pastilles —
      elle flottait sous les registres sans dire ce qu'elle était. Le titre
      et les sept pastilles tiennent sur une seule rangée : l'espacement
      resserré (4 px) et des pastilles compactes (padding 6×2) laissent la
      place, sans que l'état du processeur ne défile jamais — mesuré de
      800×520 à 1920×1230.
- [x] **Chaque indicateur se présente en entier au survol** — nom développé
      (CF → retenue, ZF → zéro, DF → direction…), rôle, et état posé ou
      retombé. Le survol ne disait que « cliquez pour basculer » ; on lisait
      `CF` sans savoir ce que c'était. `flag_toggle_hint` fondu dedans.
      Contrôle : `the_flags_carry_a_title_and_describe_each_state`.

## 4 decies. Les indicateurs : tous présents, et qui clignotent

- [x] **DF ajouté aux indicateurs** — le bandeau n'en montrait que six
      (CF PF AF ZF SF OF). DF manquait, alors que `std`/`cld` le posent et
      les instructions de chaîne (`movsb`, `stosb`…) le lisent. Ordre du
      processeur conservé : CF PF AF ZF SF DF OF. « Tout doit s'afficher. »
- [x] **Clignotement orange à chaque changement** — un registre ou un
      indicateur qui bascule s'éclaire en orange, puis retombe en 0,6 s.
      Une valeur qui change sans bouger à l'écran se ratait ; l'éclat la
      désigne sans peser sur la lecture d'après. L'instant du changement
      est posé en un point unique (`notify_step`), la couleur fond de
      l'orange vers la normale (`flash_fraction`), et l'on redessine tant
      qu'elle brille. Appliqué aux registres (généraux et XMM) et aux
      indicateurs. Contrôles : `the_flash_fades_over_real_time`,
      `every_flag_is_shown_including_df`, `a_changed_value_flashes_orange`.
      Manuel et PDF à jour.

## 5. Vocation pédagogique

- [x] **L'Atelier, seconde série d'exercices** — dix-huit exercices de
      plus, livrés en greffon (`plugins/exercices/atelier.json`) et posés
      par le même `./install.sh` : huit au palier Structures (matrice,
      longueur d'une chaîne, tri par insertion, récursion, cadre de pile,
      liste chaînée, recherche d'octet, table de sauts), dix au palier
      Maîtrise (`paddd`, `pcmpeqb`/`pmovmskb`, appel système, débordement
      signé contre non signé, produit sur 128 bits, division par
      décalage, ordre des octets, compte de bits, canari de pile, code
      auto-modifiant). Les dix-huit **atteignent leur but tels qu'ils
      sont livrés** — un contrôle les assemble et les exécute, et il a
      déjà attrapé deux énoncés faux de ma main. Les budgets sont
      MESURÉS, non devinés. Le JSON se régénère depuis
      `plugins/exercices/ecrire_atelier.py` : on corrige le Python, pas
      le JSON.
- [x] **Les échecs des exercices, expliqués** (A 7.10) — la seconde
      moitié est faite : sous un but manqué, namsdec dit ce que le
      programme a FAIT — « la boucle de la ligne 6 a tourné 4 fois »,
      « le saut de la ligne 9 n'a jamais été pris ». Des compteurs, non
      des déductions ; un saut dont la suite a d'autres entrées ne dit
      rien du tout.
- [x] **Un mode « explique-moi cette fonction »** (A 7.9) — les deux
      crans sont faits. Le bloc : cinq reconnaisseurs, chacun certain ou
      muet — appel système dont le numéro est dans le bloc, boucle avec
      son nombre de tours, comparaison suivie de son saut, bloc de
      `nop`, préparation de constantes. La FONCTION : un portrait en une
      ligne — les arguments lus avant d'être écrits, la valeur rendue
      quand tous les retours s'accordent, les appels système. Il se tait
      quand il n'aurait que ce que le dessin montre déjà.
- [x] **Les sauts rendus à leur forme** (A 7.9) — `namsdec-re/structure`
      lit le graphe d'une fonction et en écrit le pseudo-code indenté :
      `si` / `sinon` pour deux branches qui se rejoignent, `tant que`
      quand le test est en haut, `répéter … tant que` quand il est en
      bas. La condition affichée est celle du PROGRAMME, non celle du
      saut. Ce qui ne se reconnaît pas garde son « aller à ». La case
      « Pseudo-code » de la fenêtre du déroulement bascule entre les
      deux vues, et le choix part dans `settings.json`.

## 6. Tests et robustesse

- [x] **L'intégration continue passe sur le coureur GitHub** — verte
      pour la première fois le 29 septembre 2026. Elle échouait sur un
      contrôle qui lit les binaires de la machine : seize instructions
      présentes sur Ubuntu et absentes ici manquaient au manuel. Ce
      contrôle est le seul du dépôt à lire la machine sciemment, parce
      que ce qu'il trouve n'est jamais un faux positif — un échec y est
      une liste de fiches à écrire, non un défaut de l'application.

- [x] **Les contrôles ne dépendent plus d'un corpus que la CI n'a pas**
      (A 7.15, « en fond ») — `tests/binaires/` n'est pas versionné, et
      cette décision tient : un dépôt n'embarque pas d'exécutables. Mais
      elle laissait **37 contrôles sur 555 s'abstenir en silence** là où
      le dossier est vide, donc sur la CI — verts sans rien ouvrir.
      Le chiffre a été mesuré avant d'y toucher, en faisant CRIER le
      dossier absent : 40 contrôles l'appelaient, 37 s'abstenaient, 3
      repliaient sur `/bin/ls` — dont un qui demandait `crackme-010`,
      une faute de frappe qui lui faisait éprouver `/bin/ls` depuis
      toujours sans que rien ne le dise.
      Un binaire d'épreuve est maintenant FABRIQUÉ par nasm et ld au
      moment du contrôle, le même sur toute machine : deux sections, des
      chaînes, un appel et son retour, une boucle de comparaison, un
      `read` qui attend vraiment. Et une variante LONGUE — douze mille
      instructions — pour les cinq contrôles qui ne mesurent quelque
      chose que sur un gros listing. 32 des 37 ont tenu dessus sans une
      ligne de changement ; les cinq autres demandaient soit l'invite
      exacte du crackme, soit la taille — la fabrication leur donne les
      deux.
      Le corpus de chacun garde son contrôle, qui est sa raison d'être :
      `every_binary_here_loads_and_runs`, désormais son SEUL lecteur.
      LA CLEF DE VOÛTE est un contrôle neuf : si la fabrication se
      rompait, les trente-sept redeviendraient muets en silence. Il exige
      donc que les deux binaires soient des ELF, se chargent, se
      désassemblent, avancent d'un pas, que le long porte ses dix mille
      « mov » et que le court demande vraiment son mot de passe.
      Mesuré : la suite rend 704 contrôles au vert AVEC le corpus et 704
      SANS — c'est-à-dire que la CI vérifie enfin ce qu'elle croyait
      vérifier.
      Un défaut trouvé en chemin, dans la règle qui lit la source des
      contrôles : son découpage courait jusqu'au `#[test]` suivant, donc
      un test s'appropriait les fonctions libres posées sous lui. Elle
      s'arrête à sa propre accolade. C'est ce qui l'a fait accuser le
      contrôle neuf d'un `clip_rect` écrit trente lignes plus bas.

- [~] **Des tests dépendent encore de la machine** (A 3.1) — un dossier
      `tests/binaires/` existe, le reste passe encore par nasm et ld.
      Deux fuites trouvées et bouchées le 25-26 septembre : le
      `keymap.json` de l'utilisateur, lu par dix-sept tests, et la liste
      des outils installés, lue par le test du plan d'installation —
      lequel a échoué le jour où namsdec a été installé sur la machine.
- [x] `the_assembled_program_can_leave_the_workbench` stabilisé (A 3.4)
- [x] Le rendu hors écran est mesuré, et les captures sont outillées (A 3.2)
- [x] **Toute abstention nomme sa raison** (A 7.6, clos) — 7.6
      demandait « un contrôle qui dit ce qui n'est pas testé ». Il tient
      à l'envers de ce qu'on croirait : non pas compter ce qui manque,
      mais **refuser qu'un silence n'ait pas de nom**. Un `return;` nu
      est un contrôle qui rend la main au lieu de juger, et qui passe au
      vert en le faisant.
      `every_abstention_names_its_reason` relève les 225 abstentions des
      556 contrôles et exige que chacune porte, dans ses trente lignes
      d'amont, l'une des neuf raisons nommées — l'assembleur, le
      compilateur C, le corpus, un binaire du système, rien d'assemblé,
      pas de dossier de données, un outil non installé, bash, un réglage
      hors du dépôt. Allonger la liste demande une phrase écrite, comme
      la liste blanche de 7.4.
      **SIX DÉFAUTS TROUVÉS EN L'ÉCRIVANT**, et le pire d'abord :
      `the_install_button_brings_its_plan_to_the_front` ne vérifiait
      RIEN, et sur aucune machine. Il semait l'état des outils dans le
      littéral de sa structure, or ouvrir les préférences JETTE le relevé
      par conception et l'image suivante interroge la machine — ce que le
      message d'erreur de 7.4 dit mot pour mot. rizin-ghidra se
      présentait donc « installé », aucun bouton « Réparer » n'était
      peint, et le contrôle s'abstenait. Il sème maintenant APRÈS la
      première image, fait défiler la liste pour amener le bouton dans sa
      zone visible — les trois outils qui ont une recette sont les
      derniers des six —, et crie s'il ne le trouve pas.
      Quatre s'abstenaient sur l'absence d'un fichier DU DÉPÔT
      (`docs/guide.py`, `src/`, `install.sh` deux fois) : ce n'est pas une
      machine mal outillée, c'est le dépôt qui a bougé. Elles échouent.
      Trois autres s'abstenaient faute de trouver ce qu'elles venaient
      cliquer — un en-tête, un numéro de ligne du profil, une cellule de
      la grille hexadécimale. Mesuré : les trois tournent ici, donc leur
      garde-fou ne protégeait que leur propre silence le jour où le
      panneau changerait. Elles crient, et l'une d'elles rendait son
      silence par un `println!` resté de la chasse d'octobre.
      Éprouvée en semant une abstention anonyme : elle rougit et la
      nomme.

- [x] **Un chemin de ma machine dans deux contrôles** (A 7.6) — relevé
      en comptant les abstentions : **212 contrôles sur 556 peuvent se
      taire**, et la plupart pour une bonne raison (132 demandent nasm et
      ld, 24 un binaire du système — tous présents sur la CI). Onze
      s'abstenaient sans raison nommée, et deux d'entre eux lisaient
      `/home/fred/CrackMe/crackme2`, le fichier qui avait servi à
      signaler le défaut de l'éditeur binaire. L'un des deux était la
      RÉTENTION de ce défaut (A 9.7) : elle ne retenait rien sur aucune
      autre machine, tout en passant au vert.
      Les deux travaillent maintenant sur le binaire fabriqué, et le
      premier retient enfin quelque chose — il n'assertait rien, il
      IMPRIMAIT : le refus de l'octet zéro doit se dire, ne rien
      inscrire, et ne pas changer le fichier d'un octet.
      Un troisième lisait le corpus par son chemin, sans passer par
      l'aide commune, et échappait donc au relevé de la veille.
      `no_control_reads_a_path_from_someones_home` refuse désormais
      `/home/`, `/root/` et `/Users/` dans un corps de contrôle. Un
      chemin INVENTÉ pour l'exemple — le rapport d'anomalie qui ne doit
      pas trahir le chemin d'un client — s'écrit sous `/home/quelquun/`,
      et la convention se lit d'un coup d'œil. Éprouvé en semant le
      chemin : il rougit et le nomme.

- [~] **Un contrôle qui dit ce qui n'est pas testé** (A 7.6) — commencé ;
      reste à relire les tests d'affichage existants
- [x] **Un test de bout en bout sur l'installation** (A 3.3) — il LANCE
      `install.sh`, regarde ce qui est posé sur le disque, puis lance
      `uninstall.sh` et vérifie ce qui part. Il n'écrit rien dans le
      profil : `--prefix` et `--bin-dir` l'envoient dans un dossier
      temporaire, `--no-dock` et `--no-path` lui interdisent le bureau
      et le shell, `--no-build` évite de rappeler cargo depuis cargo.
      Ces options n'existaient pas — c'est pour cela que ce test est
      resté ouvert des semaines.
- [x] **`egui_kittest` évalué, et gardé pour une seule chose** (A 3.5,
      7.7) — le mandat était « si elle ne raccourcit pas visiblement,
      elle sort ». Le même contrôle écrit des deux façons fait la même
      longueur : sur ce critère, elle sortait.
      Mais elle CLIQUE, et le banc ne sait pas le faire — les captures
      non plus, les clics simulés n'atteignant pas l'application sous
      Wayland. Or tous nos contrôles d'interface fabriquent l'état
      (`flow_pseudo: true`) puis mesurent ce qui est peint : ils
      vérifient que la vue est juste, jamais que le geste y mène.
      Débrancher la case « Pseudo-code » ne faisait rougir aucun test ;
      elle en fait rougir un maintenant.
      L'emploi est borné : kittest pour les GESTES, le banc pour la mise
      en page — il mesure des pixels, là où l'arbre d'accessibilité ne
      connaît que des noms.

## 7. Dette technique

- [x] **Le découpage de `tests.rs`** (A 7.15 §3) — 33 032 lignes et 554
      contrôles dans un fichier, un quart du code du dépôt. Vingt
      modules de thème dans `src/tests/`, nommés par ce qu'ils gardent :
      `execution` en porte 47, `banc` 39, `chassis` 10. Les 574
      fonctions sont retrouvées une à une, et un contrôle de relecture
      compare chaque corps à ce qu'il était : **quatre seulement ont
      changé**, les trois qu'il fallait changer et un artefact de la
      comparaison. Rien ne s'est perdu — 703 contrôles au vert avant,
      703 après, plus un neuf.
      LE MOTIF INVOQUÉ NE TENAIT PAS, et c'est écrit en tête du fichier
      plutôt qu'oublié : « chaque contrôle modifié recompile les 31
      kloc ». Mesuré AVANT d'y toucher — une ligne ajoutée dans
      `tests.rs`, 5,74 s ; la même dans `timeline.rs`, 5,65 s. L'unité
      de compilation est le crate, non le fichier. Le découpage ne fait
      rien gagner à la compilation ; il est fait pour la seule raison
      qui restait : on retrouve un contrôle, et surtout on voit ce qui
      n'en a pas.
      LE PIÈGE ÉTAIT AILLEURS. Trois contrôles lisent la source des
      contrôles — la règle qu'ils vérifient est une règle d'écriture.
      Ils faisaient `include_str!("tests.rs")` : après le découpage ils
      auraient regardé un vingtième du banc, EN PASSANT AU VERT. Ils
      lisent maintenant `SOURCES_DU_BANC`, qu'un contrôle neuf confronte
      aux `mod` déclarés. Les trois ont été éprouvés en semant une faute
      dans un AUTRE fichier qu'eux — chacun rougit, et nomme le fichier.
      Et le relevé qui lit tout le dépôt exclut `src/tests/` comme il
      excluait `tests.rs`, sans cesser de regarder les essais
      d'intégration : semé, mesuré.

- [~] **Le découpage de `main.rs`** (A 5.1, 7.2) — 20 641 lignes au
      départ, 6 440 aujourd'hui. Dix-neuf modules en sont sortis :
      `flow`, `tools`, `windows`, `editor`, `menus`, `calc_ui`,
      `console`, `help`, `preferences`, `statusbar`, `run`, `files`,
      `jobs`, `marks`, `editing`, `maximize`, `session`, `exercises`,
      `text`. Ce qui reste est la structure `NamsdecApp`, ses
      énumérations, le routeur d'actions, les fonctions de dessin
      partagées et la boucle de rendu — c'est-à-dire ce qui a le plus
      de raisons d'y rester.
      Le patron est celui de `sidebar.rs` : `impl NamsdecApp` dans le
      module, `use super::*` en tête, le texte déplacé sans une virgule
      de différence, et `pub(crate)` seulement sur ce qu'un module frère
      appelle.
- [x] **Aucun commentaire n'a perdu sa fonction** — vingt-cinq blocs de
      `///` documentaient autre chose que ce qu'ils précédaient, dont
      vingt-deux avant le découpage. Rust laisse deux blocs se suivre et
      les concatène : rien ne le signalait. Six recollés, cinq effacés,
      un invariant pose la règle.
- [x] **Les contrôles qui lisent le code voient tous les modules**
      (A 7.11) — quatre d'entre eux ne regardaient que `main.rs` et se
      vidaient à chaque extraction, sans rougir. `SOURCES_DE_L_INTERFACE`
      les branche sur les vingt-neuf fichiers de l'interface, et un
      contrôle vérifie que cette liste suit les `mod`.
- [~] **Le listing entier est mis en page en une fois** (A 5.3, 7.8) —
      MESURÉ le 30 septembre, et ce n'était pas où l'on croyait.
      Les trois cents millisecondes de la mise en page sont payées UNE
      fois : le cache tenait déjà. Ce qui se payait à chaque frame était
      la CLÉ de ce cache — un condensé des six cent mille octets, 1,2 ms,
      deux fois par frame. Le texte lui-même sert de clé maintenant :
      11 µs, cent treize fois moins, et plus sûr.
      Ce qui reste : les ~280 ms de la première pose, à l'ouverture d'un
      binaire. Les retirer demande de ne poser que les lignes visibles,
      donc de sortir du `TextEdit` d'egui — qui porte le curseur, la
      sélection, la recherche, les points d'arrêt et la complétion.
      C'est un vrai chantier, non un réglage.
- [x] **Les frames lentes** (A 5.2) — le message ne confond plus les
      deux causes. La distinction se devinait en comparant la longueur
      du texte d'une frame à l'autre, ce qui marche quand l'éditeur pose
      dans la même frame et se trompe exactement quand il ne faut pas :
      un fichier venu d'un dialogue arrive parfois après le dessin, et
      c'est la frame SUIVANTE qui paie. Le cache de mise en page compte
      ses poses — il sait, lui, s'il a payé.
- [x] **Ce qui pèse dans le binaire** (A 5.4) — remesuré le 30
      septembre, et tranché. La pile TLS de la recherche de mise à jour
      pèse 1,6 Mo sur 16, autant que tout le code de namsdec réuni, pour
      une fonction employée une fois par mois : elle passe derrière une
      option de compilation, activée par défaut. Un empaqueteur bâtit
      avec `--no-default-features` et obtient 14,4 Mo.
      `panic = "abort"` gagnerait autant et n'est PAS pris : une panique
      dans l'un des cinq threads de fond tuerait alors tout le processus
      au lieu du seul thread, et namsdec en lance pour installer,
      décompiler et chercher une version. Un mégaoctet et demi ne vaut
      pas le travail en cours de quelqu'un.
- [x] `egui_extras` retiré des dépendances (A 5.5)
- [x] **Dépendances à jour** (7 octobre 2026) — contrôle complet :
      toutes les dépendances directes étaient déjà sur leur dernière
      version majeure (egui/eframe 0.36.2, iced-x86 1.21, rfd 0.17.2,
      arboard 3.6.1, ureq 3.4.2, egui_code_editor 0.4.1) — aucun saut
      majeur à faire. `cargo update` a tiré les correctifs compatibles
      (thiserror 2.0.21, libc, rustix, zerocopy, wasm-bindgen, cc…).
      `tokio` apparaît dans le lock via `wasm-bindgen-futures` mais
      seulement sur la cible wasm32 : il n'entre pas dans le binaire
      natif. Compilation propre, clippy muet, 1 087 tests au vert.
- [x] **Le détour** (A 4.17, 15 octobre 2026) — la première brique de
      « modifier le code du binaire ouvert », et la seule qui demandait
      une idée neuve. Tout ce que `patch` faisait gardait la longueur
      exacte, ce qui interdit d'ajouter quoi que ce soit : un octet de
      plus décale tout le fichier. Le détour ne combat pas la règle, il
      la contourne — `jmp` vers la place libre que `slack` connaît déjà,
      code ajouté là-bas, instructions recouvertes réécrites à la suite,
      `jmp` de retour. Le fichier garde sa taille au bit près.
      `jmp_rel32`, `plan_detour`, `Detour`, `DetourRefusé` dans
      `namsdec-re/patch`, et deux `Patch` ordinaires en sortie — donc le
      détour se défait par la fenêtre Correctifs, sans rien de spécial.
      Ce qui a demandé le plus de soin est le REFUS : une instruction
      relative à `rip` déplacée lit la mauvaise donnée sans rien dire.
      namsdec décode le site, refuse, et nomme l'instruction.
      18 tests, et deux oracles extérieurs parce qu'aucun test unitaire
      ne dit si le binaire TOURNE : Ghidra 12.1.4 (par REA) relit le
      fichier corrigé et y voit la structure exacte ; Frida 17.23.3
      confirme que le corps du détour est atteint à l'exécution, l'octet
      visé valant encore 'A' à cet instant. Le programme affiche 'B'.
      Deux pièges consignés dans AMELIORATIONS : Frida ne peut pas
      instrumenter un statique sans libc, et un crochet Frida sur `main`
      écrase le détour — il pose cinq octets là où `push rbp` n'en fait
      qu'un.
      Pas encore dans l'interface : l'onglet RE vient ensuite, et le
      manuel avec lui. Rien de visible n'a changé pour l'instant.
- [x] **Relocaliser plutôt que refuser** (A 4.17, 15 octobre 2026) — le
      détour refusait presque tout code compilé, donc ne servait à rien :
      `lea rsi, [rip+0x2b8a]` est la forme normale d'un accès à une
      globale. `relocate` recalcule l'écart pour que la somme retombe
      sur la même cible, sur les deux champs qui s'y prêtent — le
      déplacement d'un accès relatif à `rip`, l'immédiat d'un saut ou
      d'un appel relatif. `Decoder::get_constant_offsets` dit où ils
      sont ; l'instruction garde sa longueur.
      Pas d'encodeur ajouté : iced-x86 reste en `decoder` seul, et le
      champ est réécrit sur place. Le poids du binaire est surveillé.
      Le piège évité est silencieux : `mov dword [rip+0x10], 0x42` porte
      un déplacement à recalculer ET un immédiat qui est une donnée.
      On ne touche l'immédiat que s'il porte une cible — sinon on
      changerait la valeur écrite sans que rien le montre.
      Reste refusé, et c'est honnête : un `jmp rel8` ne porte qu'à
      127 octets. L'élargir changerait sa longueur. Un test montre que
      de près il passe, pour qu'on n'ait pas codé « rel8 = refus » sans
      le savoir.
      25 tests, formulés sur la SOMME et non sur les octets : l'adresse
      visée, recalculée depuis la nouvelle position, doit être celle
      d'avant. Et Ghidra, par REA, relit le site refusé une heure plus
      tôt : `0x40049a: LEA RSI, [0x40300c]` — même cible, et le
      programme affiche bien son message.
- [x] **L'oracle différentiel** (A 3.6, 15 octobre 2026) — l'émulateur
      n'avait rien pour dire s'il correspond au vrai processeur, et
      c'était le seul endroit du projet où une erreur pouvait vivre
      longtemps sans rien dire. `namsdec-emu/examples/oracle.rs` sème un
      état, le fait exécuter par `gdb` sur le matériel et par
      `namsdec-emu` sur les mêmes octets, et compare. Aucune réponse
      n'est écrite à la main : on ne dit jamais ce que `adc` devrait
      donner, on le demande au processeur.
      La moitié du travail est la table des INDÉFINIS. Le manuel d'Intel
      laisse certains indicateurs à la discrétion du processeur, et les
      comparer reviendrait à reprocher à namsdec de n'être pas ce
      modèle-ci. La première exécution signalait onze instructions ; dix
      étaient de ce genre.
      La onzième était réelle : `bzhi` ne posait pas CF et effaçait SF,
      parce qu'il partageait sa branche avec `bextr`, qui n'a pas les
      mêmes règles. Corrigé, et retenu par un test écrit dans
      `namsdec-emu/src/tests.rs` — pas dans l'oracle, qui demande nasm,
      ld, gdb et un x86-64 et ne tourne donc pas sur la CI. Le contrôle
      a été éprouvé en remettant le défaut.
      Bilan : 91 instructions, 910 cas, zéro écart. Toute l'arithmétique
      entière (AF et PF compris), les décalages, les rotations, les bits,
      et seize instructions AVX2.
      Ce qu'il ne peut PAS dire ici : cette machine n'a pas AVX-512
      (Lunar Lake), donc le travail le plus récent de l'émulateur est
      précisément celui qu'on ne peut pas confronter au matériel. Il
      faudrait Intel SDE ou QEMU — à brancher à la place de `gdb` dans
      `sous_le_matériel`, seule fonction qui connaisse l'outil.
      Trois manques signalés au passage : `vpblendw`, `pext`, `pdep`.
- [x] **Les boutons des panneaux se voient** (A 3.7, 15 octobre 2026) —
      le thème les rend plats au repos, et le contrôle qui l'interdit
      hors des menus ne lisait que les sources de `namsdec-gui`. Les
      panneaux vivent dans `namsdec-re` : leurs trente-trois boutons
      étaient donc nus, dans la même fenêtre où ceux de l'interface
      portent un liseré.
      Vu À L'ÉCRAN pendant l'audit du panneau « Détourner », pas par une
      mesure — le banc vérifie que rien ne déborde, ne se recouvre et
      n'est coupé, et un bouton invisible satisfait les trois.
      `framed`, `framed_button`, `framed_small_button` posés dans
      `namsdec-re/src/lib.rs` ; le liseré vient des `visuals`, donc
      aucune dépendance nouvelle et le thème clair est servi aussi.
      L'invariant reste : `un_bouton_de_panneau_se_voit_sans_survol`,
      avec sa liste de sources qui se tient à jour sur les `pub mod` de
      `lib.rs`, et une seule exception nommée — `framed`, qui pose le
      liseré.
- [x] **pext, pdep, vpblendw** (A 3.8, 15 octobre 2026) — les trois que
      l'oracle avait signalées non pas fausses mais ABSENTES. Écrites bit
      à bit comme le manuel les décrit : un émulateur qui sert à
      apprendre doit pouvoir se lire, l'astuce irait dans un
      compilateur. Vérifiées contre le matériel dans la foulée — 220 cas,
      zéro écart —, ce qui est tout l'intérêt d'avoir l'oracle.
      Le contrôle de pext/pdep est un aller-retour : `pdep` du résultat
      de `pext`, même masque, remet chaque bit en place. Confondre
      « tasser » et « éparpiller » ne passerait pas.
      Deux fiches écrites, parce qu'un invariant l'exige : toute
      instruction qui s'exécute doit pouvoir s'expliquer. Deux autres ont
      réclamé que l'interface et le README annoncent 176 et non 174.
- [x] **AVX-512 confronté au matériel** (A 3.10, 15 octobre 2026) — le
      trou que 3.6 avait identifié est comblé : Intel SDE 10.13.1 émule
      le jeu complet, et `--sde` le met devant le programme d'épreuve.
      Empreinte vérifiée contre celle d'Intel avant extraction.
      Refonte au passage, et elle valait mieux que le branchement : le
      programme relève SON PROPRE état — un `mov` vers la mémoire ne
      touche aucun indicateur, c'est toute l'astuce. `gdb` disparaît :
      une dépendance de moins, bien plus rapide, et un émulateur se
      glisse devant sans rien changer d'autre.
      Cinq défauts trouvés, que personne ne pouvait voir. Quatre
      comparaisons à masque ne lisaient que les 128 bits bas d'un `zmm`
      (le calcul des voies s'écrivait « ymm ou pas ymm »), et `vpermq
      zmm` laissait la moitié haute à zéro. Corrigés, vérifiés sous SDE,
      retenus par deux contrôles éprouvés en remettant les défauts.
      La bonne nouvelle : le cœur d'AVX-512 est juste. Masques `{k1}`,
      zéroing `{z}`, formes VL, les huit opérations entre masques. 127
      instructions, 762 cas, zéro écart.
      Onze instructions manquent encore, et l'oracle les nomme —
      `vpternlogd` en tête, qui n'a d'équivalent nulle part ailleurs.
- [x] **Les onze d'AVX-512, et quatorze de plus** (A 3.11, 15 octobre
      2026) — `vpternlogd`, `vpermd`, `valignd`, `vshufi32x4`,
      `vpsrav`/`vpsllv`/`vpsrlv`, `vplzcnt`, `vpopcnt`, `vpconflict`, et
      les comparaisons EVEX à destination de masque. Chacune écrite puis
      vérifiée sous SDE dans la minute : implémenter une instruction
      vectorielle n'est plus un pari sur une lecture du manuel.
      Deux trouvailles de conception en route. `vpermq` porte DEUX
      instructions sous un nom — immédiat figé par voie de 256, ou index
      variable sur tout le registre —, et namsdec ne connaissait que la
      première. Et `vpcmpeqd k1, …` n'est pas `vpcmpeqd xmm0, …` : seul
      le type de la destination les sépare.
      L'oracle passe 138 instructions, 1 104 cas, zéro écart, zéro
      écartée.
      Sept fiches écrites (183 au lieu de 176). Cinq invariants ont
      réclamé leur dû, dont un qui a cédé à raison : son exemple
      d'instruction « ni simulée ni documentée » venait d'être les deux.
- [x] **QEMU ne sert pas d'oracle pour AVX-512** (A 3.9) — piste
      fermée par l'expérience, pas par la documentation. `qemu-x86_64
      -cpu help` annonce `avx512f` et les autres, mais ce sont des noms
      de drapeaux, pas des instructions exécutables : un `vpaddd zmm0,
      zmm1, zmm1` sous `-cpu max` rend un signal 4. Reste Intel SDE.
- [x] **Un vidage mémoire était dans le dépôt** (A 7.15, 15 octobre
      2026) — `qemu_essai512_*.core`, 8,4 Mo, commité par mégarde avec
      c2896a0 pendant les essais QEMU. Retiré du suivi, `*.core` posé
      dans `.gitignore`, le fichier laissé sur le disque.
      Le blob reste dans l'historique et la branche est poussée : l'en
      sortir demande un rebase des sept commits et un force-push, à
      faire AVANT la fusion dans `main`. En attente d'un accord, parce
      que c'est irréversible.
- [x] **Le taux de refus du détour, mesuré** (A 7.15 point 2, 15
      octobre 2026) — la crainte à lever avant d'aller plus loin : le
      détour refuse les instructions qu'il ne sait pas relocaliser, et
      `mov rax, [rip+0x2e4f]` est la forme NORMALE d'un accès à une
      globale en code compilé. L'interface dit pourquoi elle refuse UN
      site ; rien ne disait combien elle en refuse.
      `namsdec-re/examples/taux_detour.rs` planifie un détour à chaque
      adresse d'instruction de chaque plage exécutable et compte, par
      motif et par mnémonique. Le seuil était posé D'AVANCE par le plan,
      ce qui est tout ce qui rend une mesure honnête.
      **3 371 923 sites, 66,0 % acceptent un détour.** Et le motif
      craint ne paraît pas une fois : `CodeDéplacéRelatif`, zéro sur
      3,4 millions. La relocalisation de 4.17 couvre entièrement les
      deux formes courantes — 547 997 sites recouvrent du code relatif,
      54,5 % passent, là où tout était refusé en bloc avant.
      Le seul refus de PRINCIPE est le saut court : 5,6 %, un site sur
      dix-huit, dont `je`/`jmp`/`jne` en rel8 font 81 %. Les 28,1 %
      restants ne viennent pas des instructions mais de la PLACE —
      `croackpocalypse` n'a qu'un trou, trop petit ; `crackme`,
      `TermBreaker` et `thisismebtw` n'en ont aucun.
      DÉCISION : la relocalisation à longueur variable n'est pas la
      suite ; fabriquer de la place l'est, et c'est un travail sur le
      fichier ELF. Décidé sur un chiffre, comme partout ici.
      Un défaut trouvé dans le banc lui-même, et c'est la part utile de
      l'essai : il ne lisait que les SECTIONS, donc deux binaires
      dépouillés rendaient « 0 site » — ce qui se lit « rien à mesurer »
      au lieu de « rien de mesuré ». Avec le recours aux segments, ces
      deux-là rendent 1 458 363 sites. Ce que le banc ne sait pas dire
      est écrit en tête du fichier : le balayage est linéaire, une table
      de données enchâssée est comptée comme du code.
      Le chiffre est passé dans le manuel, à l'endroit où le détour
      explique ses refus : un outil qui sert à apprendre doit dire
      l'étendue de ce qu'il sait faire, et pas seulement la raison d'un
      non.
- [x] **Le plan de la séance suivante** (A 7.15, arrêté le 15 octobre
      2026) — les cinq points sont faits : (1) [FAIT] 7.9, expliquer un
      ENSEMBLE et non plus une instruction, puis la moitié manquante de
      7.10 — voir les deux entrées de la section 5 ; (2) [FAIT] mesurer
      le taux de refus du détour sur les
      onze binaires d'épreuve — 66 %, et la relocalisation n'est pas la
      condition : la place libre l'est ; (3) [FAIT] découper `tests.rs`,
      33 032 lignes et 554 contrôles d'un bloc — vingt thèmes, et le
      motif invoqué ne tenait pas (voir la section 7) ;
      (4) [FAIT] réconcilier l'analyse d'AMELIORATIONS avec son journal —
      5.3 et 7.8 se donnaient encore pour le gros chantier restant, 3.5
      et 7.7 pour une dépendance inutile ; (5) [FAIT] le ménage
      ci-dessus.
      Ce qui motive l'ordre : les cinq dernières séances ont servi la
      justesse — invisible pour qui apprend —, et l'émulateur porte
      désormais de quoi tenir 7.9 honnêtement.

- [x] **Un mode débutant, et une interface qui se gradue** (3 novembre
      2026) — demandé le jour même : *« ce sera à usage pédagogique
      pour apprendre l'assembleur et le RE ; les fonctions avancées
      pourront être activées/désactivées. L'interface débutant doit
      être plus simple, la version avancée identique à l'actuelle. »*

      **Le mécanisme, et il tient en trois pièces.** `ReModule::niveau()`
      — déclaré par le panneau, comme sa famille, parce que c'est lui
      qui sait ce qu'il demande de SAVOIR. `Settings::debutant`, qui va
      dans `settings.json` comme tout réglage. Et la colonne d'analyse,
      qui s'y borne. Un panneau neuf est **avancé par défaut** : le
      silence range du bon côté, et personne ne s'invite devant
      quelqu'un qui apprend sans qu'on l'ait décidé.

      **Neuf rubriques sur vingt-cinq**, et chacune porte la raison de
      sa présence : « En bref » (qu'est-ce que c'est ?), « Carte du
      fichier » (où sont les choses), « Fonctions » et « Graphe
      d'appels » (par où lire le code), « Références croisées » (« qui
      se sert de ceci ? », la question fondatrice), « Chaînes lisibles »
      (ce qu'il dit en clair), « Appels système » et « Fonctions
      importées » (ce qu'il demande au dehors), « Éditeur binaire »
      (changer un octet et voir le programme faire autre chose —
      l'exercice par lequel tout le monde commence).

      **Trois décisions prises contre ma première idée, chacune sur un
      défaut observé :**

      1. **Une rubrique ouverte ne disparaît pas sous les pieds.** Ma
         première règle refermait sur « Tout » ce qui passait au-dessus
         du niveau — et sept contrôles d'analyse sont tombés d'un coup
         pour le dire : un lien d'un autre panneau (« aller voir cette
         adresse dans les octets ») mène à un panneau avancé, et c'est
         une DEMANDE. L'escamoter laisse un panneau à l'écran sans
         entrée dans la colonne : on ne sait plus où l'on est.
      2. **« Tout » suit le niveau, le compte rendu non.** Une colonne
         allégée au-dessus d'un « Tout » qui déroule les vingt-cinq
         panneaux se contredit. Mais « Copier le rapport » reste
         entier : c'est un export, et un export tronqué serait un piège.
      3. **Le défaut vit dans le RÉGLAGE, pas dans le code des
         contrôles.** `Settings::default()` dit vrai — une installation
         neuve s'ouvre sur ce qui s'apprend — et la migration v2→v3 dit
         faux pour une configuration qui existait déjà : son
         propriétaire se sert de l'outil entier depuis toujours. Le
         `Default` de l'application, lui, reste avancé, parce que le
         banc et les contrôles doivent mesurer l'interface ENTIÈRE —
         sinon seize rubriques sortaient d'un coup de toute mesure.

      **Et une scène de banc pour le mode débutant**, qui a trouvé un
      défaut dès la première image : « Fonctions **1 (cibles de
      `call`)** ». Un outil qui sert à apprendre ne peut pas écrire
      cela. L'accord en nombre est posé sur les quatre comptes de « En
      bref » — un programme d'une ligne les met tous à un.

      Retenu par trois contrôles : la colonne raccourcit vraiment et
      garde ce qui se lit sans rien savoir ; la rubrique ouverte reste
      listée ; le réglage traverse l'enregistrement et la migration.

      **LA BARRE D'OUTILS SUIT** (même jour, découpage des neuf
      rubriques approuvé). `Action::niveau()`, sur le modèle de
      `ReModule::niveau()`. Sept commandes deviennent **cinq** :
      Assembler, Exécuter, Pas à pas, Stop, Réinitialiser.

      Ce qui part demande de savoir quelque chose d'avance — *par-dessus
      quoi ?* pour « Step Over », et qu'une machine puisse garder son
      passé pour « Step Back ». De ces deux-là, **le second est le plus
      discutable** : défaire un pas est une des plus belles choses que
      namsdec sache faire, et cela s'explique en une phrase. S'il doit
      revenir, c'est une ligne dans `Action::niveau`.

      **Et le pas change de nom.** Les boutons s'appelaient « Step
      Into », « Step Over », « Step Back » — l'anglais de gdb et de
      pwndbg, qui est le bon pour qui s'en sert déjà, puisque les
      raccourcis de namsdec sont les leurs. Dans une interface française
      qui sert à apprendre, ce sont trois murs. En mode débutant le pas
      est seul : il n'a rien à distinguer, et s'appelle **« Pas à pas »**
      — exactement le mot que le manuel employait depuis toujours pour
      le décrire, pendant que le bouton disait autre chose.

      Rien n'est désinstallé : les raccourcis marchent, la palette
      propose encore les deux commandes. **C'est la barre qui se tait.**
      Retenu par un contrôle qui relève les deux barres et compare, et
      par une scène de banc — l'écran principal en mode débutant.

      **LES DEUX RANGÉES D'ONGLETS SUIVENT.** `VueCentrale::niveau()`
      et `SidebarTab::niveau()`, même forme que les deux précédents.

      Au centre, **quatre sur neuf** : ce qu'on a écrit, ce que la
      machine en a reçu, la forme que le programme prend, et ce que le
      fichier dit de lui-même. Partent la décompilation (un module
      qu'on allume déjà), la chronologie, les points d'arrêt, les
      registres vectoriels et les correctifs. **« Points d'arrêt » est
      la plus discutable** — poser un arrêt s'apprend tôt —, mais le
      clic dans la gouttière marche à tous les niveaux : c'est la
      LISTE, avec ses conditions `(rax & 0xff) == 'A'`, qui demande
      d'en savoir plus.

      Dans le panneau, **quatre sur six** : registres, pile, mémoire,
      octets — les quatre endroits où l'on voit la machine faire, et le
      dernier est le plus pédagogique de tous, qui met la source et le
      code machine côte à côte. Le tas suppose qu'on alloue.

      **La règle « ce qu'on regarde reste listé » est devenue la règle
      du lot**, et elle est éprouvée : on envoie un débutant sur les
      correctifs — ce que fait « Outils › Patcher le binaire » —, son
      onglet paraît, et la rangée n'en ramène aucun autre. Sans elle,
      une vue s'afficherait sans entrée dans sa rangée : on ne saurait
      plus ni où l'on est, ni comment en sortir.

      **Un choix assumé, en revanche :** le lien « Registres étendus
      (SSE, AVX, x87)… » reste au bas de l'onglet Processeur, même en
      mode débutant. C'est la porte vers le cran d'après, et la vue
      qu'elle ouvre récupère son onglet en s'ouvrant. Effacer toute
      mention de ce qui existe serait cacher l'outil, non l'alléger.

      **LES MENUS FERMENT LE LOT.** Le menu Outils ne propose plus ce
      que la rangée ne montre pas — chaque entrée qui ouvre une vue
      suit le niveau de SA vue, ce qui rend la cohérence automatique —
      et s'allège en plus de `Step Over`, des trois « jusqu'au prochain
      appel / retour / appel système » (les `nextcall`, `nextret` et
      `nextsyscall` de pwndbg) et du **motif cyclique**, qui est un
      outil d'exploitation : on s'en sert pour retrouver de combien
      d'octets on a dépassé dans un débordement de tampon, question qui
      ne se pose pas avant d'avoir appris ce qu'est une pile. Le menu
      Affichage ne liste que les onglets de panneau qui existent au
      niveau.

      **Une distinction qui a demandé à être nommée :** un onglet dit
      *où l'on est*, une entrée de menu est une *porte*. D'où deux
      règles et non une — `vue_au_niveau` garde la vue active dans la
      rangée, `montre_vue` ne l'ajoute pas au menu. Une porte qui ne
      paraît qu'une fois qu'on est passé ne sert à rien.

      **Et la palette garde tout.** C'est une recherche, pas une
      vitrine : y cacher une commande la rendrait introuvable à qui la
      connaît, alors qu'une barre allégée ne fait que ne pas la
      proposer.

      **Le défaut que la mesure a trouvé, et il est le plus instructif
      du lot :** la barre d'outils disait « Pas à pas » pendant que le
      menu Outils disait encore « Step Into » — deux noms pour un même
      geste, dans la même image, une heure après la traduction de la
      barre seule. Tout ce qui nomme une commande passe désormais par
      `action_label_pour`, la palette comprise. Retenu par un contrôle
      qui ouvre vraiment le menu et compare les deux niveaux.

      **DEUX RETOURS, DEMANDÉS APRÈS COUP** (3 novembre 2026) — les
      deux que j'avais signalés comme les plus discutables de ce que
      j'écartais, et la décision est allée contre mon découpage :

      * **« Step Back » revient au débutant.** Je l'avais écarté parce
        qu'il suppose qu'une machine garde son passé, ce qui n'est vrai
        d'aucun débogueur ordinaire. Mais c'est exactement ce qui en
        fait une des plus belles choses que namsdec sache offrir à qui
        apprend : on avance d'un pas, on n'a pas vu ce qui a changé, on
        revient. Cela s'explique en une phrase et s'essaie sans risque.
      * **« Points d'arrêt » revient.** Je l'avais écarté pour ses
        CONDITIONS (« (rax & 0xff) == 'A' »). Mais poser un arrêt
        s'apprend le deuxième jour, et cette vue est **le seul endroit
        où l'on voit ce qu'on a posé** : sans elle, un arrêt oublié
        trois fichiers plus loin devient un programme qui s'arrête sans
        raison — la pire chose qui puisse arriver à qui apprend. Le
        champ de condition attend qu'on s'en serve ; il n'oblige à rien.

      **Et une conséquence qu'il a fallu traiter dans la minute :**
      « Step Back » devenait le seul mot anglais d'une barre qui dit
      « Pas à pas ». Les deux pas vont par paire, et s'appellent
      désormais **« Pas à pas »** et **« Pas en arrière »** au niveau
      débutant — par le même `action_label_pour`, donc dans la barre
      comme dans la palette.

      **ET UN DÉFAUT SIGNALÉ À L'ÉCRAN, QU'AUCUNE MESURE N'AURAIT
      POSÉ** (3 novembre 2026) : *« je ne sais pas basculer entre les
      modes débutant et avancé. »* Le réglage n'avait de chemin que par
      Fichier › Préférences › Modules — trois gestes, et au seul
      endroit où quelqu'un qui débute n'ira pas chercher. Un réglage
      qui décide de TOUT ce que l'application montre ne peut pas être
      le plus enfoui de tous.

      Il est maintenant **en tête du menu Affichage**, qui ne contient
      que des bascules décidant de ce qu'on voit — la barre d'outils,
      le panneau, la barre d'état, la console — et celle-ci les
      gouverne toutes. La case des Préférences reste : on y RÈGLE, on
      ne bascule pas ; les deux touchent le même champ, donc elles ne
      peuvent pas diverger. Retenu par un contrôle qui ouvre vraiment
      le menu et le lit, aux deux niveaux.

      **Le lot est clos** : la colonne d'analyse (9 rubriques sur 25),
      « Tout », la barre d'outils (6 commandes sur 7), les onglets du
      centre (5 sur 9) et du panneau (4 sur 6), les menus et la
      palette parlent le même niveau, et un seul endroit le dit
      (`NamsdecApp::niveau`).

---

## 8. Le mode simple, interface à part

- `[~]` **Trois dispositions en maquette, à départager** — 9 novembre
  2026 (A 9.81). Aujourd'hui, `Niveau::Débutant` FILTRE l'interface
  complète : neuf rubriques d'analyse au lieu de vingt-cinq, cinq
  onglets de vues sur neuf, un éditeur sans modes. Le squelette reste
  celui du mode avancé — rails, onglets, menus, barre d'état. La
  demande est une interface **à elle**, simplifiée au maximum mais
  complète.

  Trois directions sont posées dans `mockups/2026-11-09/`, jouables
  depuis le disque, sur un scénario commun (somme de 1 à 5 dans une
  fonction appelée par `call` : 10 lignes, 22 instructions,
  `exit(15)`) :

  * **L'Établi** — quatre bandes fixes : le code, la machine, ce que
    fait la ligne courante, le résultat. Aucun onglet, aucun rail,
    aucun menu déroulant. La plus proche de ce que l'application sait
    déjà faire ; le travail serait surtout de RETIRER.
  * **Le Carnet** — une colonne, le programme lu comme un texte, et
    l'état de la machine écrit EN MARGE de la ligne courante. La
    question « avant d'avancer » y est le sujet de la page, et le
    verdict attend le pas (règle de `prevision.rs`).
  * **La Scène** — deux temps exclusifs, « Écrire » et « Regarder ».
    Dans le second, le code n'est plus qu'un ruban et la machine prend
    toute la place : trois grands chiffres, la pile dessinée, la fiche
    de l'instruction en cours. Le seul des trois qui se lise d'un
    mètre.

  **Tranché le 9 novembre 2026 : l'Établi, et la Scène gardée en
  bouton.** Les deux sont faits — voir l'entrée suivante.

  **Les trois tiennent la même charge** : écrire, assembler, exécuter,
  lire ; l'écran d'une faute de frappe ; registres, pile, indicateurs,
  octets de chaque ligne et adresse ; le retour au mode avancé à la
  même place. Le mode simple montre MOINS, pas autre chose.

  **Deux défauts trouvés par la mesure, pas par la relecture** — les
  maquettes ont été capturées et comparées au pixel (`chromium-browser
  --headless`, deux rendus identiques) : un bouton éteint qui gardait
  son fond d'accent, et le pupitre du Carnet qui, flottant, mangeait la
  dernière réponse de la question. Voir A 9.81 pour le détail, et pour
  la raison qui a rendu ces captures reproductibles.

- `[x]` **L'Établi est la disposition du mode simple** — 9 novembre
  2026 (A 9.82), `crates/namsdec-gui/src/etabli.rs`. `Niveau::Débutant`
  ne filtre plus l'interface complète : **il choisit une mise en page**.
  Quatre bandes fixes — ce qu'on a écrit, ce que fait la machine, ce que
  fait la ligne où l'on est, le résultat —, aucun menu, aucun rail,
  aucune barre d'état. La rangée des vues reste (cinq façons de regarder
  de niveau débutant), la barre des fichiers ne paraît qu'à partir de
  deux fichiers ouverts.

  Retenu par : les scènes `établi`, `établi (en marche)` et
  `établi / l'assemblage a échoué` du banc, qui passent par **tous** ses
  invariants ; `la_barre_d_outils_se_gradue`,
  `on_peut_toujours_changer_de_niveau` et
  `les_menus_se_graduent_sans_se_contredire`, réécrits pour mesurer la
  nouvelle règle.

  **Quatre défauts trouvés par la mesure, pas par la relecture** :
  registres coupés à droite (largeur du volet), recouvrement de la barre
  à 800 px, deux hauteurs de bouton côte à côte, et une phrase de la
  bande du résultat tranchée par le milieu. Le détail est en A 9.82.

- `[x]` **La Scène est gardée en bouton** — même jour,
  `crates/namsdec-gui/src/scene.rs`. Une fenêtre, aux **deux** niveaux :
  un ruban d'instructions, jusqu'à trois registres en gros chiffres —
  ceux que le dernier pas a changés —, la fiche de l'instruction qui
  vient, et les commandes. Trois portes : le bouton de la frise au mode
  simple, Outils › Regarder tourner… au mode avancé, et la palette.

  **Pourquoi une fenêtre et non une dixième vue** : elle s'ouvre
  par-dessus sans rien déranger, elle existe aux deux niveaux, et elle
  peut prendre tout l'écran — c'est la seule façon de montrer un pas à
  quelqu'un qui n'est pas devant le clavier.

- `[x]` **Ce qui ne tient pas dans une bande sort dans une fenêtre** —
  10 novembre 2026 (A 9.83). « Ce que fait la machine » et
  « Résultat » portent un bouton **Détacher**, comme la rangée des
  vues. Trouvé à l'écran sur un ELF lié dynamiquement : la pile
  défilait horizontalement, le résultat était tranché au milieu d'une
  ligne, et la ligne d'état avait **avalé RIP et le bouton de la
  Scène** — posés après le message, ils prenaient « ce qui reste », et
  il ne restait rien.

- `[ ]` **Une scène du banc devant un vrai binaire** — le trou que
  A 9.83 a mis au jour : les trois scènes de l'Établi tournent sur le
  programme du banc (quinze lignes, registres à zéro), donc **aucun
  invariant ne peut voir un contenu long**. Une pile qui porte
  l'environnement, une note de quatre lignes, un listing de mille
  instructions : rien de tout cela n'est mesuré aujourd'hui.

- `[x]` **Quatre manques de l'Établi, vus à l'écran** — 10 novembre
  2026 (A 9.84). Les **trois pastilles** de fenêtre (il n'y avait plus
  de quoi fermer l'application), l'**engrenage** des préférences, la
  **calculatrice**, et une console enfin visible : plus haute, sur le
  fond des bandes, et une pastille verte dans son titre dès que le
  programme a écrit.

  **La leçon dépasse ces quatre-là** : en retirant la barre de menus,
  on a retiré ce qu'elle PORTAIT, sans en faire l'inventaire. Les
  exercices avaient déjà été rattrapés ainsi. Reste à passer le menu
  Outils et le menu Fichier en revue, entrée par entrée, pour voir ce
  qui n'a plus de porte au mode simple.

- `[x]` **Trois commandes qui mentent, sur une seule capture** — 10
  novembre 2026 (A 9.85). Une flèche verte sur le bouton
  « Détacher » de la rangée des vues, la source active : « ne sert à
  rien ». Il se peignait alors que la source ne se détache pas, et
  `détacher_vue` refusait en silence. Deux voisins du même tonneau
  étaient sur la même image : le message d'état de l'Établi, qui
  passait **par dessus** le bouton de la Scène et RIP jusqu'à
  x = 2824 sur un écran de 1600 ; et la légende de la pile, peinte
  **dans** la zone qui défile de côté, donc coupée au milieu d'un
  mot.

  **Chacun demandait une condition que le banc n'avait jamais
  réunie** : un voisin pour que le bouton se peigne seulement, un
  état LONG dans la frise, une pile LARGE sous la légende. C'est la
  leçon : un contrôle ne voit un défaut de place que si la place
  manque vraiment. Les trois contrôles neufs sèment donc ce qu'il
  faut, et une scène de banc est ajoutée pour la frise.

  **Un quatrième défaut est sorti en regardant l'image** : le code de
  l'éditeur y était en charabia, et ce n'était pas l'application —
  l'atlas de glyphes grandit à la dernière image, et le premier rendu
  emploie encore l'ancienne texture. Les trois relevés d'images
  rendent maintenant **deux fois**, et jettent la première.

- `[x]` **Le parcours se consultait au lieu de se faire** — 10 novembre
  2026 (A 9.86). « Les exercices devraient être interactifs ». Au mode
  avancé, l'énoncé et le but vivent dans le panneau latéral, qui est
  toujours là ; l'Établi n'en a pas. On chargeait une étape, on
  refermait la fenêtre, et l'on se retrouvait seul devant un programme
  à trous. **Une cinquième bande** porte maintenant le rang, l'énoncé
  et le but — condition par condition, **qui s'allument pendant qu'on
  avance** —, avec la barre du parcours, « Choisir… » et « Étape
  suivante ». Rien n'est réécrit : `goal_ui` peignait déjà tout cela,
  il manquait l'endroit.

  **Et un défaut de mesure trouvé en chemin, qui vaut plus que la
  bande** : `Rect::from_min_size(t.pos, t.galley.size())` est faux
  pour toute galley alignée à droite — `pos` y est le bord droit.
  L'erreur vaut la largeur du texte et va toujours vers la droite,
  c'est-à-dire vers le bord où un recouvrement se produit. Dix-sept
  relevés du banc l'employaient ; `cadre_du_texte` translate
  `galley.rect`, qui porte déjà le décalage.

  **Au plus petit écran, la bande prend la place du code** : mesuré en
  lignes de programme entièrement visibles, six deviennent **zéro**.
  Donner à la bande une part de ce qui reste plutôt que ses 190 pixels
  en rend deux. **Deux reste peu**, et deux autres pistes ont été
  mesurées puis retirées : rapetisser encore la bande ne donne rien
  (le résultat reprend ce qu'elle rend), et borner la bande de la
  ligne courante gagnait une troisième ligne mais faisait **se
  recouvrir deux panneaux**. Le seul levier qui reste est la part du
  résultat, qui vient d'une demande explicite (A 9.84) : arbitrage à
  prendre avec Fred, pas sans lui.

- `[x]` **L'interface montrait l'état sans le nommer** — 10 novembre
  2026 (A 9.87). « L'interface n'est pas assez pédagogique ». Le volet
  de la machine montrait dix-sept noms de trois lettres et sept sigles
  de deux, sans un mot — dans le mode dont la règle est que chaque
  bande se présente elle-même. La pile avait pourtant sa légende
  depuis toujours.

  Deux corrections, au niveau débutant seulement. **Le dernier pas se
  dit en toutes lettres** : `RCX 3 → 2` au lieu de `RCX 0x3 → 0x2`
  sous le million, et l'indicateur qui se pose dit ce qu'il raconte.
  C'est le moment où l'on apprend, et il exigeait deux traductions.
  **Le volet dit ce qu'il montre** : une phrase au-dessus des
  registres, une sous les indicateurs — pas dans l'infobulle, qui
  demande de savoir qu'il faut survoler.

- `[x]` **Le programme qu'on lit n'était expliqué nulle part** — 10
  novembre 2026 (A 9.88). Le premier écran porte quinze lignes et sept
  mots qu'un débutant ne connaît pas ; la bande du bas répondait
  « revenez quand vous aurez assemblé », et la seule porte vers les
  189 fiches était F1, une touche dont rien ne parle. **La bande
  explique maintenant la ligne où est le curseur**, tant que rien ne
  tourne : le mot, ce qu'il veut dire, la fiche entière à un clic. On
  lit un programme ligne à ligne avant de l'exécuter — le premier des
  quatre verbes, et le seul qui n'avait aucun support.

  **Et un défaut vieux de toujours en est sorti** : `prose` ne rendait
  pas les `*emphases*` simples, que cinquante passages des fiches
  emploient. La fenêtre d'aide montrait ses astérisques depuis le
  début. L'invariant du balisage ne regardait que les doubles.

  **Ce que la bande coûte, mesuré deux fois.** La fiche complète
  ramenait de six à une les lignes de programme visibles à 800×520 :
  on n'en garde que la première phrase. Puis le cas qu'on n'avait pas
  regardé — un exercice chargé — faisait paraître **deux** bandes
  d'explication et ne laissait **aucune** ligne. D'où la règle :
  **une seule bande explique à la fois**, et pendant un exercice c'est
  la sienne. Ce que cela coûte est dit : pendant un exercice, la ligne
  du curseur ne s'explique plus toute seule.

- `[x]` **Trois gestes qui ne se défaisaient pas** — 10 novembre 2026
  (A 9.89). « Montrer » donnait la réponse sans retour possible, le
  compteur de problèmes ouvrait la console sans la refermer, et un
  trait de « ce qui se clique » soulignait des notes qui ne mènent
  nulle part (« pourquoi ces lignes »). Les trois sont réparés, et la
  troisième est la même famille que le bouton « Détacher » inerte du
  matin : un signe qui annonce un geste que rien ne rend.

  **Deux défauts vus sur les mêmes images** : l'énoncé de l'exercice
  montrait ses accents graves, et le constat du but parlait encore en
  hexadécimal pendant que la bande du bas disait « RCX 3 → 2 ».

  **Et pour la convivialité** : la cible d'un exercice se lit
  maintenant **avant** le premier essai — en gris, sans verdict. On
  écrivait sans savoir où l'on allait.

  **Deux remarques d'une quatrième capture, que j'avais manquée** :
  le nom du fichier s'abrège par le milieu dans la barre du haut (en
  gardant son extension), et la rangée où l'on répond au programme
  n'est plus poussée sous le bord de la fenêtre — la bande et la zone
  des problèmes réservent maintenant ce que les rangées du bas
  réclament, par la mesure que la console du mode avancé se donnait
  déjà.

- `[x]` **Le titre ne disait pas ce qu'on lisait** — 11 novembre 2026
  (A 9.90). Une capture : cent vingt lignes de désassemblage d'un
  programme chargé, et au-dessus « namsdec sans titre ». Un binaire
  ouvert n'a pas de `current_file` — exprès : un Ctrl+S distrait n'a pas
  à écraser l'exécutable par son propre désassemblage — et les quatre
  endroits qui nomment le tampon lisaient ce champ-là seul. Ils passent
  tous par `nom_du_tampon` et `chemin_du_tampon` désormais, les deux
  seules sources de l'application.

  **Le survol donne le chemin complet**, partout où le nom s'écrit :
  deux `crackme` rangés dans deux dossiers portent le même nom. Dans la
  barre de titre, l'infobulle ne se montre que sur les lettres — la
  bande, elle, court jusqu'au bord de la fenêtre. **Et le titre du
  système porte le nom** : l'alt-tab ne montrait que « Namsdec ».

  **Le contrôle du survol a échoué trois fois** pour une raison qui se
  reverra : un `PointerMoved` renvoyé à chaque image, fût-ce au même
  endroit, fait croire à egui que la souris bouge encore — et une souris
  qui bouge n'ouvre aucune infobulle.

- `[x]` **La séance précédente se redemande** — 11 novembre 2026
  (A 9.91). namsdec rouvrait d'autorité le fichier et le binaire de la
  veille. Au lancement, il pose maintenant la question : le nom de ce
  qu'il rouvrirait, son **chemin complet** dessous, et combien d'autres
  fichiers reviendraient avec. **Reprendre** ou **Repartir à neuf**, et
  une case **Ne plus demander** qui retient la réponse qu'on vient de
  donner. Le réglage — *Demander*, *Rouvrir*, *Ne pas rouvrir* — se
  reprend aux Préférences, page Général.

  **Le contrôle qui clique a trouvé ce que l'autre n'aurait pas vu** :
  la case « ne plus demander » était une variable locale de la fenêtre,
  recalculée à chaque image — on la cochait, elle se décochait toute
  seule à l'image suivante. Un contrôle appelant la méthode de décision
  serait passé au vert ; celui qui clique par `egui_kittest` est tombé
  dessus du premier coup.

- `[x]` **Quatre-vingt-dix-sept exercices ne demandaient rien** —
  11 novembre 2026 (A 9.92). Mesuré en exécutant les cent dix-sept
  étapes : sur les 101 qui posent un but, **97 l'atteignaient déjà
  telles qu'elles étaient livrées**. On ouvrait l'étape, on cliquait
  « Exécuter », la pastille passait au vert — sans avoir écrit une
  ligne.

  Chacune a maintenant un **trou** (« ; À FAIRE : … ») et une
  **solution** rangée à côté dans la série. Un invariant exécute les
  deux : le sujet s'assemble, rend la main et n'atteint pas le but ; la
  solution l'atteint, dans le budget annoncé. **Trois buts étaient
  faux** — « rax == 0x3c » mesuré juste après `mov rax, 60` — et un
  quatrième était vrai avant le premier pas.

  **L'interface** : le curseur se pose sur le trou, la bande dit ce
  qu'il reste à écrire et l'efface à mesure, et la solution se demande
  sans rien effacer ni coûter de score. **Et le but se lit en toutes
  lettres au mode débutant** — « RAX vaut 15 » et non « rax == 0xf »,
  comme la bande du bas qui dit déjà « RCX 3 → 2 ».

  **La leçon de forme** : un trou dans une boucle emporte la boucle
  entière, saut de retour compris. Huit squelettes partaient sinon en
  boucle infinie — et c'est un contrôle plus ancien, plus exigeant que
  le mien, qui l'a dit.

- `[x]` **Un indice, entre la consigne et la solution** — 12 novembre
  2026 (A 9.93). Après les trous, l'aide restait tout ou rien : la
  consigne, puis le programme entier — qui met fin à l'exercice.
  L'indice est le palier qui manquait. **Il se calcule** pour 80 étapes
  (les instructions que la solution emploie et que le tampon n'a pas),
  **s'affine** à mesure qu'on écrit, et **mène aux fiches** : chaque
  instruction nommée ouvre la sienne. Les 21 étapes dont le trou ne
  demande aucune instruction nouvelle ont un indice écrit, et un
  contrôle vérifie qu'aucun ne recopie une ligne de la solution.

  **Mesuré puis laissé** : les énoncés. Médiane de 334 caractères,
  aucun trop court, tous reliés à une fiche — il n'y avait rien à
  corriger, et réécrire au jugé aurait coûté la clarté qu'ils ont.

- `[x]` **Le parcours va dans les deux sens** — 12 novembre 2026
  (A 9.94). « Étape suivante » existait des deux côtés, et rien en
  face : pour revoir l'étape qu'on venait de faire, il fallait rouvrir
  la liste. Un chevron `‹` dans la bande, « Étape précédente » en
  toutes lettres dans la fenêtre. **La précédente tout court**, et non
  « la précédente non réussie » : on avance vers ce qu'on n'a pas fait,
  on recule vers ce qu'on vient de faire.

- `[x]` **Revenir à une étape rend ce qu'on y avait écrit** — 12
  novembre 2026 (A 9.95). Le chevron ne tenait sa promesse qu'à une
  condition tacite : que l'onglet de l'étape soit encore ouvert. Fermé,
  il emportait le programme, et l'étape rouvrait sur son code de départ
  — précisément ce qu'on ne cherchait pas en revenant en arrière.
  **Un brouillon par étape, gardé en mémoire** (`exercise_drafts`),
  rangé quand l'onglet se ferme et **relu sur demande** : au moment où
  l'on redemande cette étape-là, et pas avant. La séance seulement, et
  non le disque — ce n'est pas un réglage. Un brouillon qui ne diffère
  pas du code de départ n'en est pas un : il n'est pas gardé, sans quoi
  une correction d'énoncé n'arriverait plus jusqu'à l'éditeur.
  **Deux contrôles**, tous deux éprouvés en remettant le défaut :
  `a_closed_step_keeps_what_was_written` et
  `an_untouched_step_leaves_no_draft`.

- `[x]` **Cinq choses que l'écran disait de travers** — 16 novembre 2026
  (A 9.96). Les dix scènes « établi » du banc rendues puis regardées,
  et cinq contradictions relevées : le volet de la machine écrivait
  `0000000000000037` pendant qu'un exercice demandait « Que vaut RAX à
  l'arrêt ? » (réponse : 55) ; le bloc « Dernier pas » annonçait une
  instruction **jamais exécutée**, lue dans les octets nuls qui suivent
  le `hlt` ; le but d'un exercice collait deux phrases en une (« RAX
  vaut 2 RAX vaut 1 ») ; R8 à R15, tous à zéro, prenaient la moitié du
  volet ; et la dernière explication d'erreur était tranchée par le
  milieu au bas de sa bande.

  Retenu par `at_beginner_level_registers_are_written_plainly`,
  `a_finished_program_does_not_invent_one_more_step`,
  `the_high_registers_show_up_when_the_program_uses_them`,
  `la_bande_des_problemes_dit_la_suite_et_propose_la_correction`, et
  les deux contrôles d'exercices qui portaient l'ancien constat.

  **Le réglage neuf** : `registres_hauts` dans `settings.json`, pour
  celui qui veut ses seize registres même vides.

- `[x]` **Quatre portes qui manquaient** — 16 novembre 2026 (A 9.97).
  Le parcours était **éteint au premier lancement** — bouton grisé, et
  la seule indication était un commentaire dans le programme de
  bienvenue : il est allumé pour une installation neuve, et le bouton
  l'allume d'un clic. `mov rzx, 1` n'avait **aucun diagnostic** : une
  septième règle de `fixes` nomme le registre qui n'existe pas et ses
  voisins, sans jamais en choisir un quand plusieurs sont à égalité.
  Les corrections de `fixes`, qui n'avaient de porte que par Alt+Entrée
  et la barre d'état, **se montrent sous l'erreur** avec un bouton. Et
  la bande dit enfin **où l'on en est** : le nom que la source donne à
  la cible d'un saut (`jne boucle` au lieu de seize chiffres), le
  compte des pas, et le numéro de passage par la ligne courante.

  Retenu par `a_jump_target_is_named_as_the_source_named_it`,
  `the_beginner_bench_says_how_far_the_program_has_got`,
  `le_bouton_du_parcours_allume_le_module_qu_il_demande`, quatre
  contrôles neufs dans `fixes` et le test de migration des réglages.

- `[x]` **Le parcours répond, et il n'avance que sur ce qu'il a
  montré** — 16 novembre 2026 (A 9.98). Deux demandes en une, et la
  mesure les a séparées.

  **La progressivité était presque acquise** : sur les cent dix-sept
  étapes, vingt introduisent une notion dans leur trou, dix-huit la
  nomment, une la nomme par sa famille (« `setcc` »), et **une seule**
  ne la nommait nulle part — `rep`, à l'étape 43, qui a reçu son
  indice. Ce qui manquait était la garantie que cela le reste :
  `aucune_etape_ne_demande_une_notion_jamais_vue_ni_annoncee`.

  **L'interactivité, non.** Savoir si l'on avait réussi demandait trois
  gestes et un savoir — assembler, exécuter, remarquer une pastille.
  Un bouton **Vérifier** fait les trois et conclut en toutes lettres,
  sans jamais juger une course qu'on n'a pas demandée. Une étape de
  lecture se grise tant qu'on n'a pas répondu : l'exécuter donnerait la
  réponse.

  **Le carnet des notions** dit ce que les étapes réussies ont fait
  écrire ou lire, par fiche, chacune cliquable : « 12/117 » disait le
  chemin et rien de ce qu'on y avait appris.

  Retenu par `verifier_assemble_execute_et_rend_un_verdict`,
  `une_question_ne_se_verifie_pas_avant_d_avoir_repondu`,
  `le_carnet_des_notions_suit_les_etapes_reussies` et l'invariant de
  progression. Et un défaut de mesure corrigé :
  `la_bande_de_l_exercice_ne_coupe_pas_sa_derniere_ligne` posait un
  `hlt` seul, donc un but sans constat — il mesurait deux lignes de
  moins que ce qu'on voit.

---

## Comment cette liste se tient

Une entrée passe à `[x]` quand **un contrôle la retient** — pas quand le
code est écrit. C'est la règle qui a le plus servi cette année : trois
défauts corrigés sans invariant sont revenus, et un contrôle écrit sans
être éprouvé en remettant le défaut est passé au vert pendant des
semaines en ne regardant rien.

La séance du 29 octobre en a donné un exemple de plus, et il vaut d'être
gardé : le contrôle qui devait prouver qu'un arrêt emporte la
descendance d'une commande **passait sans le groupe de processus**. Il
vérifiait que la main revenait vite — ce qu'un abandon donne aussi bien
qu'un arrêt. Il mesurait notre patience, pas la mort du descendant.
Refait avec un témoin que celui-ci pose s'il survit, il tombe dès qu'on
retire le groupe. **Un contrôle qui ne distingue pas les deux états
qu'il nomme ne regarde rien**, même quand il est vert pour la bonne
raison.

Toute modification visible met aussi à jour `docs/guide.py` et régénère
le PDF du manuel.
