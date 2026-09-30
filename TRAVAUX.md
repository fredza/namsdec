# namsdec — ce qui est fait, ce qui reste

Cette liste est l'**index** du travail. Le *pourquoi* de chaque entrée —
ce qui l'a fait apparaître, ce qu'elle coûte, ce qui a été essayé et
retiré — reste dans `AMELIORATIONS.txt`, qui ne bouge pas de rôle : une
seule source pour le raisonnement, une seule pour l'état. Les renvois
`(A 4.14)` pointent vers l'entrée correspondante de ce fichier-là.

Tenue à jour au fil des séances. Dernière mise à jour : **30 septembre
2026**, version 0.2.6-beta.1, 612 tests au vert.

- `[x]` fait et vérifié — un contrôle le retient
- `[~]` commencé, le reste est écrit sous l'entrée
- `[ ]` à faire

---

## Où en est l'interface

**Non, l'UX/UI n'est pas terminée.** Une passe complète a été faite le
25 septembre : seize scènes capturées sur profil neuf, deux relevés de
mesure sur les trente-trois scènes du banc, **vingt et un écarts corrigés,
dix-sept contrôles nouveaux et un élargi**. Ce que cette passe a mesuré
et **pas** corrigé est en bas de la section suivante, chiffré. C'est du
travail identifié, pas de l'exploration : il n'y a plus de relevé à
refaire.

Ce qui reste est de la finition mesurée : des alignements de quelques
pixels, deux tailles de police, et le vide sous « Prochaine
instruction », qui tient au même nœud que le vide de 56 px — l'état du
processeur est ancré.

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
- [x] **`no_control_asks_the_machine_what_it_should_be_checking`** — le
      contrôle qui empêche la récidive. Il lit `tests.rs`, découpe par
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

- [~] **Des tests dépendent encore de la machine** (A 3.1) — un dossier
      `tests/binaires/` existe, le reste passe encore par nasm et ld.
      Deux fuites trouvées et bouchées le 25-26 septembre : le
      `keymap.json` de l'utilisateur, lu par dix-sept tests, et la liste
      des outils installés, lue par le test du plan d'installation —
      lequel a échoué le jour où namsdec a été installé sur la machine.
- [x] `the_assembled_program_can_leave_the_workbench` stabilisé (A 3.4)
- [x] Le rendu hors écran est mesuré, et les captures sont outillées (A 3.2)
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

---

## Comment cette liste se tient

Une entrée passe à `[x]` quand **un contrôle la retient** — pas quand le
code est écrit. C'est la règle qui a le plus servi cette année : trois
défauts corrigés sans invariant sont revenus, et un contrôle écrit sans
être éprouvé en remettant le défaut est passé au vert pendant des
semaines en ne regardant rien.

Toute modification visible met aussi à jour `docs/guide.py` et régénère
le PDF du manuel.
