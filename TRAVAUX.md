# namsdec — ce qui est fait, ce qui reste

Cette liste est l'**index** du travail. Le *pourquoi* de chaque entrée —
ce qui l'a fait apparaître, ce qu'elle coûte, ce qui a été essayé et
retiré — reste dans `AMELIORATIONS.txt`, qui ne bouge pas de rôle : une
seule source pour le raisonnement, une seule pour l'état. Les renvois
`(A 4.14)` pointent vers l'entrée correspondante de ce fichier-là.

Tenue à jour au fil des séances. Dernière mise à jour : **25 septembre
2026**, version 0.2.5-beta.3, 562 tests au vert.

- `[x]` fait et vérifié — un contrôle le retient
- `[~]` commencé, le reste est écrit sous l'entrée
- `[ ]` à faire

---

## Où en est l'interface

**Non, l'UX/UI n'est pas terminée.** Une passe complète a été faite le
25 septembre : seize scènes capturées sur profil neuf, deux relevés de
mesure sur les trente-trois scènes du banc, **quinze écarts corrigés,
quatorze contrôles nouveaux et un élargi**. Ce que cette passe a mesuré
et **pas** corrigé est en bas de la section suivante, chiffré. C'est du
travail identifié, pas de l'exploration : il n'y a plus de relevé à
refaire.

Ce qui manque encore n'est pas de la finition mais du fond : huit
fenêtres s'ouvrent plus grandes que ce qu'elles montrent, et les états
vides parlent avec deux formes et quatre formulations différentes.

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

## 2. Interface — ce que la revue a mesuré et pas corrigé

Détail chiffré dans `AMELIORATIONS.txt` 7.11, section « REVUE DU
25 SEPTEMBRE — CE QUI RESTE ».

- [ ] **Huit fenêtres plus grandes que ce qu'elles montrent** — de 138 à
      863 px de trou, quand aucune fenêtre saine ne dépasse 59. Cinq se
      remplissent une fois le programme chargé : c'est l'état vide qui
      est en cause, donc l'écran du premier lancement.
- [ ] **« Registres étendus » est trop grande même pleine** — seize XMM
      et huit ST, contenu de taille fixe et connue, dans une fenêtre qui
      lui laisse 197 px de trop en hauteur et 146 en largeur. Le plus net
      des huit.
- [ ] **Les états vides ont deux formes** — neuf blocs centrés à trois
      étages, huit phrases seules alignées à gauche. Aucune n'est
      majoritaire au point de servir de norme.
- [ ] **Le même fait s'écrit de quatre façons** — « Aucun binaire
      assemblé. », « … pour l'instant. », « Assemblez d'abord : … »,
      « Aucun programme chargé : … ».
- [ ] **Le listing de désassemblage perd ses colonnes de 54,8 px** sur
      les lignes qui portent une étiquette.
- [ ] **La barre de saisie de la console** : 4,5 px d'amplitude sur une
      seule rangée.
- [ ] **Étiquette contre liste déroulante** : 3,5 px, deux fois. Avec un
      `DragValue` l'écart tombe à 0,5 — c'est le couple qui est mal
      centré, pas une fatalité d'egui.
- [ ] **Deux marges gauches dans le châssis**, 16 et 18 px.
- [ ] **La grille de bits de la calculatrice est en 12 pt** quand toute
      autre grille de données est en 13. Attention : remonter peut faire
      déborder les soixante-quatre colonnes.
- [ ] **Le graphe écrit ses instructions en 11 pt à 100 %**, et « le
      programme est ici » y est de la prose posée en chasse fixe.
- [ ] **Le vide sous « Prochaine instruction »** fait encore ~260 px.
      Même nœud que le vide de 56 px : l'état du processeur est ancré.

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
- [ ] **L'onglet Octets n'est pas éditable** alors que la vue mémoire l'est
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
      `tests/binaires/` existe, le reste passe encore par nasm et ld
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
