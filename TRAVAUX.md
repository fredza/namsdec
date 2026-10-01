# namsdec — ce qui est fait, ce qui reste

Cette liste est l'**index** du travail. Le *pourquoi* de chaque entrée —
ce qui l'a fait apparaître, ce qu'elle coûte, ce qui a été essayé et
retiré — reste dans `AMELIORATIONS.txt`, qui ne bouge pas de rôle : une
seule source pour le raisonnement, une seule pour l'état. Les renvois
`(A 4.14)` pointent vers l'entrée correspondante de ce fichier-là.

Tenue à jour au fil des séances. Dernière mise à jour : **1er octobre
2026**, version 0.2.6-beta.1, 1 075 tests au vert.

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
- [ ] **Ce qui reste** : ne poser que les lignes visibles. Gain d'un facteur
      ~300 sur la première image, coût d'une vue en lecture seule pour les
      listings — sélection, recherche, curseur, marques. Pas à entreprendre
      à la légère.

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
- [ ] **La suite, non commencée** : assembler du code dans une zone libre,
      et détourner l'exécution vers lui puis la ramener. Passe par
      `patch.rs`, qui sait déjà défaire ce qu'il pose, et par
      `ReRequest::PatchBytes`, qui existe déjà. À faire pas à pas, chaque
      geste réversible et montré avant d'être posé — comme les correctifs.

## 4 octies. Découpage de main.rs (suite)

- [x] **main.rs : 6 643 → 5 453 lignes** (−1 190, −18 %), sans rien
      changer au comportement. Cinq modules neufs, chacun un thème net,
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
- [x] Les cinq ajoutés à `SOURCES_DE_L_INTERFACE` : les contrôles qui
      lisent le code source les voient désormais comme les autres.

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
