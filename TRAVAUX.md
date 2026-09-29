# namsdec — ce qui est fait, ce qui reste

Cette liste est l'**index** du travail. Le *pourquoi* de chaque entrée —
ce qui l'a fait apparaître, ce qu'elle coûte, ce qui a été essayé et
retiré — reste dans `AMELIORATIONS.txt`, qui ne bouge pas de rôle : une
seule source pour le raisonnement, une seule pour l'état. Les renvois
`(A 4.14)` pointent vers l'entrée correspondante de ce fichier-là.

Tenue à jour au fil des séances. Dernière mise à jour : **25 septembre
2026**, version 0.2.5-beta.3, 577 tests au vert.

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
- [x] DZF branché comme désassembleur (A 4.16)
- [x] `keymap.json` surveillé et exportable en entier (A 4.5)
- [x] Les raccourcis, repris par famille puis par le sens (A 1.23, 7.1)
- [x] **L'onglet Octets est éditable** — devant un binaire ouvert, un
      clic sur les octets d'une instruction ouvre un champ ; Entrée
      valide, Échap annule. Même chemin que les trois gestes du
      patcheur, donc la fenêtre Correctifs les liste et sait les
      défaire. La longueur ne change pas, et c'est refusé plutôt que
      rattrapé.
- [ ] **La provenance à travers la mémoire** — suivre une valeur qui
      transite par la pile, pas seulement par les registres

## 5. Vocation pédagogique

- [~] **Les échecs des exercices, expliqués** (A 7.10) — la moitié est
      faite : chaque condition manquée dit laquelle et avec quelle
      valeur. La seconde moitié demande 7.9.
- [ ] **Un mode « explique-moi cette fonction »** (A 7.9) — le profil
      d'exécution existe déjà ; ce qui manque est la mise en phrase.

## 6. Tests et robustesse

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
- [ ] **Pas de test de bout en bout sur l'installation** (A 3.3)
- [ ] **`egui_kittest` n'est pas utilisé** (A 3.5, 7.7) — à évaluer
      maintenant que le banc de mesure fait déjà l'essentiel

## 7. Dette technique

- [ ] **`main.rs` dépasse 19 600 lignes** (A 5.1, 7.2) — toute
      l'interface y est. Le découpage est le plus gros chantier restant.
- [ ] **Le listing entier est mis en page en une fois** (A 5.3, 7.8) —
      quinze mille lignes dans un binaire ouvert ; seule la gouttière
      virtualise aujourd'hui
- [~] **Les frames lentes** (A 5.2) — mesuré, et ce n'est pas ce qu'on
      croyait ; reste à ne plus crier « frame lente » pour la première
      image après un chargement
- [ ] **Ce qui pèse dans le binaire** (A 5.4) — mesuré le 18 septembre,
      rien décidé depuis
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
