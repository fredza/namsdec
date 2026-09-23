# Journal des versions

Les dates sont celles des commits ; le format suit [Keep a
Changelog](https://keepachangelog.com/fr/1.1.0/) d'assez loin — une
entrée par version, et ce qui change pour qui s'en sert.

## 0.2.5-beta.3 — 23 septembre 2026

Ce qu'on avait sous les yeux sans le voir : une croix qui ne ressemblait
à rien d'autre dans l'application, un bouton gris qu'on prenait pour un
titre, un nom de fichier qui se déplaçait à chaque redimensionnement. Et
six fautes d'assembleur que namsdec sait désormais reconnaître.

**Corriger sans réécrire**
- namsdec regarde la ligne du curseur, dit en français ce qui cloche, et
  montre la ligne réécrite dans la barre d'état. **Alt** la pose — la
  même touche qui accepte une proposition de complétion, laquelle garde
  la priorité quand les deux s'offrent ensemble.
- Six règles, et six seulement : un crochet jamais refermé ; un
  mnémonique qui n'existe pas, à une inversion près d'un qui existe
  (`mvo` propose `mov`, `tralala` ne propose rien) ; `int 0x80` en code
  64 bits ; un saut vers une étiquette qui n'existe nulle part, casse
  comprise ; deux registres de tailles différentes, sauf `movzx` et
  `movsx` ; une écriture en mémoire dont rien ne dit la taille.
- Rien ne s'applique tout seul. Une correction dont on ne saurait pas
  dire pourquoi elle est juste apprendrait le faux, ce qui est pire que
  de se taire.

**Ce qu'on voit**
- Les quinze fenêtres internes se ferment par la **même pastille** que la
  fenêtre principale, au coin haut gauche. egui posait sa croix à lui,
  deux traits maigres à droite : deux grammaires pour le même geste, et
  c'est celle qu'on voyait le moins qui fermait le plus de choses.
- Le **nom du fichier** est aligné sur la fin du menu, à trois
  tabulations d'« Aide ». Centré, il se déplaçait à chaque panneau ouvert
  et à chaque redimensionnement : un repère qui bouge n'en est pas un.
- La **calculatrice** a son bouton juste à gauche de l'onglet Pile, là où
  le besoin naît : on lit une valeur, on veut la même dans une autre
  base. Elle était dans le menu Outils, à trois gestes de là.
- L'onglet **RE** ne porte plus qu'un grand bouton, rempli de la couleur
  d'accent, avec l'icône de Desdec — l'autre atelier de la maison. Les
  sept modules empilés dessous faisaient deux mille pixels à parcourir
  dans une colonne de trois cents ; la fenêtre d'analyse montre les mêmes
  en grand et tout dépliés. Deux fois la même chose, dont une mal.

**Réparé**
- Les icônes restent dans leur case, et un test le vérifie. Le chat de
  GitHub dépassait de la sienne d'un huitième, oreilles en haut et queue
  en bas — assez pour mordre le texte du lien voisin.
- `y y` et `g g` ne faisaient rien. egui marque la seconde frappe d'une
  même touche comme une répétition, et le collecteur écartait toutes les
  répétitions — la bonne règle pour tenir un raccourci enfoncé, la
  mauvaise pour doubler une touche.
- Cliquer la pastille des préférences ou d'« À propos » ne fermait rien :
  la barre d'action écrasait sèchement ce que la pastille venait de dire.
- Le dernier test instable du dépôt ne l'est plus. Le binaire produit n'y
  était pour rien : les tests tournent en parallèle dans le même
  processus, et le `fork` d'un voisin qui lance nasm hérite du
  descripteur ouvert en écriture. Tant qu'il n'a pas fait son `exec`, le
  noyau refuse d'exécuter le fichier — `ETXTBSY`.

**Le manuel**
- Un chapitre **« Comment une instruction est faite »** : les sept
  morceaux d'une instruction x86-64, l'octet ModR/M et ses trois champs,
  les quatre tailles d'un même registre, et pourquoi un décodeur linéaire
  se trompe. Tiré du cours de mini-désassembleur en C++ de l'auteur.
- Un chapitre **« Quatre séances, et ce qu'elles apprennent »** : quatre
  crackmes analysés avec namsdec — un XOR contre une constante, un sérial
  qui se calcule, une garde `ptrace`, un programme qui se déballe. Le
  dernier n'est pas résolu, et c'est aussi une leçon.
- Le guide refuse désormais de s'écrire s'il emploie un caractère que la
  police du PDF n'a pas. Il en employait trois, qui sortaient en carrés
  noirs — la même leçon que dans le programme, où un test vérifie chaque
  glyphe de l'interface.

---

## 0.2.5-beta.2 — 23 septembre 2026

La liste des améliorations, exécutée : sept entrées traitées, et deux
défauts trouvés en chemin.

**L'éditeur**
- Les déplacements par mot du mode Normal : `w`, `b`, `e`, `gg`. La
  notion de mot est celle de vim — dans `mov qword [rbp-0x18], rax`,
  `w` s'arrête sur le crochet puis sur `rbp`, au lieu de sauter la
  moitié de la ligne.

**Lire un binaire**
- **Nommer une adresse** et **commenter une ligne**. Le nom apparaît
  partout où l'adresse se montre — pile, adresses de retour, cibles de
  saut, graphe — et l'emporte sur le symbole du binaire.
- Rangé à côté du binaire, jamais dedans. La clé est le **contenu** du
  fichier, non son chemin : un binaire déplacé retrouve ses notes, un
  binaire recompilé n'hérite pas de celles d'un autre.
- Le graphe nomme la fonction qu'il montre, et garde son adresse à côté —
  c'est elle qu'on tape, et elle qui lève l'ambiguïté quand la fermeture
  par sauts se trompe.

**Les boîtes de dialogue**
- Une barre unique partout : bouton principal à droite en couleur
  d'accent, second de même largeur, Entrée et Échap. C'est la couleur qui
  dit lequel est le principal, pas la taille.
- « Annuler » ne défait rien dans les préférences, qui s'appliquent à
  mesure qu'on coche : c'est donc « Quitter », qui dit ce qu'il fait.

**Réparé**
- Entrée validait enfin la saisie du programme. Un champ de texte qui
  traite Entrée la retire de la file d'évènements, et la guetter après
  lui ne voyait rien.
- La zone d'attente se voit sans qu'on la cherche : liseré franc,
  pastille, et le geste écrit en toutes lettres.
- La rangée « Entrée » était coupée en deux, et le test ne le voyait pas
  — il vérifiait la position du texte, non son rectangle.
- Un diagnostic déjà affiché suit la langue quand on la change : il garde
  son motif et non sa phrase, laquelle se reforme au dessin.
- Échap ne déclenche plus l'action secondaire d'une fenêtre. La fenêtre
  d'installation y avait mis « Recommencer » : la touche qui veut dire
  « je m'en vais » lançait vingt minutes de compilation.
- Une séquence de touches en cours l'emporte sur un raccourci simple.
  Sans cette règle, toute lettre servant de deuxième touche quelque part
  serait interdite comme raccourci simple.

**Vérifié plutôt que supposé**
- Les paquets que les recettes annoncent existent : demandé au
  gestionnaire local, sans réseau ni droits.
- Les deux étiquettes de rizin et rz-ghidra existent encore : demandé à
  `git ls-remote` avant, et non découvert après dix minutes de
  compilation.
- La recette de rz-ghidra a été suivie en entier, de bout en bout : c'est
  la première fois qu'on peut dire qu'elle est juste.
- Les tests ne parlent plus du disque de leur auteur : ils cherchent les
  binaires d'épreuve dans `tests/binaires/`, que le dépôt ignore
  désormais.

---

## 0.2.5-beta.1 — 23 septembre 2026

Quatre chantiers : franchir l'anti-débogage, calculer sans sortir de
namsdec, reprendre son travail là où on l'a laissé, et un éditeur qui
sait où un mot revient.

**L'anti-débogage, en entier**
- `ptrace` répond « personne ne te regarde », `prctl` accepte, `fork` rend
  zéro — namsdec joue la branche du fils, celle qui travaille — et le
  `SIGSTOP` qu'un programme s'envoie à lui-même est encaissé. C'est le
  montage complet d'un crackme sérieux ; en rater un seul les arrête tous.
- Tout autre signal s'arrête et se nomme : avaler un `SIGKILL` ferait
  courir un programme que la vraie machine aurait tué.
- Ce que cela coûte, et c'est dit à l'écran comme ici : un programme qui
  se dédouble pour de bon ne s'exécutera pas comme sur la vraie machine.
  Une seule branche est jouée.
- `int3` ne se confond plus avec `hlt` : c'est un point d'arrêt que le
  programme se pose à lui-même, non une fin.

**Une calculatrice**
- Outils ▸ Calculatrice. Toutes les bases en même temps — hexadécimal,
  décimal signé et non signé, octal, binaire, ASCII, octets mémoire —,
  les opérateurs de bits avec la priorité du C, et les registres du
  programme en cours à toutes leurs largeurs (`rax`, `eax`, `ax`, `ah`).
- Dix-huit fonctions de rétro-ingénierie : `bswap`, `sext`, `popcnt`,
  `clz`, `ctz`, `rol`, `ror`, `align`, et les autres.
- Chaque erreur se nomme : « aucun programme ne tourne, ce registre n'a
  pas de valeur » plutôt que « erreur ».

**L'entropie, et une chronologie visible**
- Un module RE de plus, juste après les protections : il répond avant
  tout le reste à « y a-t-il quelque chose à lire ici ? ». Profil par
  tranches, échelle fixe de 0 à 8, et l'adresse du bloc le plus dense.
- La fenêtre Chronologie gagne une barre : une colonne par tranche de
  temps, colorée par ce qui s'y est passé de plus notable, et sur
  laquelle on clique pour rembobiner.

**Reprendre son travail**
- Le binaire ouvert revient au lancement suivant, avec ses points
  d'arrêt, ses zones surveillées, ses arguments et son onglet.
- Les repères ne reviennent que si le fichier n'a pas changé : sur un
  binaire recompilé, ils désigneraient d'autres instructions. Ils sont
  alors oubliés, et l'application le dit.

**L'éditeur**
- **Surligner les occurrences** — `Espace s s`, ou `Ctrl+F2`. En vert du
  thème. Il cherche des jetons et non du texte : le `rax` d'un
  commentaire n'est pas surligné, et la casse suit NASM (`MOV` = `mov`,
  mais `Boucle` ≠ `boucle`).
- **Aller d'une occurrence à l'autre** — `n` et `N`, ou `F3`. Chacune
  arrive sélectionnée, donc prête à être remplacée.
- **Renommer toutes les occurrences** — `Espace s r`, ou `F2`. Ce à quoi
  le multicurseur sert neuf fois sur dix, et sans son défaut : les
  commentaires ne sont pas touchés.
- **Fermeture automatique des paires**, avec trois règles et pas une de
  plus : pas devant un mot, pas après une lettre pour l'apostrophe, et
  taper le fermant déjà là le traverse.
- **Duplication de ligne** — `Ctrl+Maj+D`, ou `y y`.
- L'autocomplétion se pilote enfin : Tab accepte, les flèches
  choisissent, Échap referme. Elle ne répondait qu'à `Alt`, et seulement
  pour sa première proposition.
- La coloration distingue `mov` de `mav` : la liste vient du décodeur
  qu'emploie namsdec, donc ce qui se décode se colore. Et une étiquette
  est peinte comme telle là où l'on y saute, pas seulement où on la
  définit.

**Réparé**
- Le champ de saisie disparaissait quand la console avait une hauteur
  fixée à la main : le plancher se calculait d'après une mesure
  elle-même tronquée.
- Un outil installé par namsdec est lancé par *son* chemin, et non par le
  programme de même nom trouvé dans le `PATH` — lequel pouvait être une
  version plus ancienne, sans le greffon attendu.
- La fenêtre du plan d'installation tient dans l'écran, se ferme par
  Échap, et ne propose plus de recommencer ce qui est déjà fait.

---

## 0.2.4-beta.1 — 18 septembre 2026

Trois chantiers : lire un binaire qu'on n'a pas écrit, ouvrir plusieurs
fichiers à la fois, et brancher les outils du métier.

**Un répertoire `plugins/`**
- Tout ce qui s'ajoute à namsdec sans le recompiler y est rangé en deux
  familles : les **thèmes**, qui ne sont que des couleurs, et les
  **fonctionnalités supplémentaires**, qui décrivent un programme à
  lancer. La distinction n'est pas un classement, c'est une question de
  confiance : un thème ne s'exécute pas.
- Chaque dossier porte son mode d'emploi — le format champ par champ, où
  les fichiers s'installent, et comment installer les décompilateurs :
  radare2, Rizin avec son greffon `rz-ghidra`, RetDec depuis ses versions
  publiées.
- Les descriptions d'outils y sont **engendrées** par le code qui les
  connaît : un argument ne peut pas diverger entre ce que namsdec lance
  et ce que le dépôt propose de copier.

**Aller chercher les outils, et les construire**
- namsdec sait maintenant installer **Rizin + rz-ghidra** et **RetDec**
  lui-même : le premier se compile dans son dossier, le second se
  télécharge depuis ses versions publiées. Un module à part, éteint par
  défaut, et qui ne s'allume que sous celui de la décompilation.
- Ce module ne se contente pas de lancer un programme : il télécharge du
  code et l'exécute. La fenêtre montre donc **la recette entière avant
  que rien ne parte** — d'où cela vient, ce que cela coûte, où cela sera
  posé, et chaque commande telle qu'elle sera lancée. Puis le journal
  s'écrit en direct, commande par commande, avec ce qu'elles répondent.
- Trois limites tenues par des tests : aucune étape n'appelle `sudo`,
  aucune ne passe par un interpréteur de commandes, et rien n'est produit
  hors de `~/.local/share/namsdec/tools/`.
- L'empreinte SHA-256 de ce qui est téléchargé est calculée et affichée,
  pour être comparée à celle que publie le projet — plutôt que confrontée
  à une constante que personne n'aurait vérifiée.
- L'adresse de l'archive de RetDec était **inventée** : le nom du fichier
  publié n'est pas celui que j'avais écrit, et `curl` rapportait un 404
  au lieu d'un décompilateur. Corrigé, vérifié de bout en bout — 180 Mo
  téléchargés, archive sans dossier racine, `retdec-decompiler` obtenu,
  puis lancé sur le crackme avec exactement les arguments que namsdec
  emploie : trente secondes, deux millions de caractères de C. Deux tests
  demandent désormais à GitHub si l'adresse et les dépôts existent
  vraiment, et se taisent sans réseau plutôt que d'échouer.
- La fenêtre d'installation **enseigne avant de proposer** : ce qu'un
  décompilateur reconstruit et ce qu'il ne rendra jamais, pourquoi c'est
  le greffon `rz-ghidra` qui décompile et non Rizin seul. Puis les faits,
  puis l'avertissement — posé contre le bouton, là où on le lit —, puis
  le plan dont chaque étape porte son état : faite, en cours, à venir.
- **Annuler annule vraiment** : l'étape en cours est tuée, au lieu
  d'attendre la fin d'un téléchargement de 180 Mo ou d'une compilation de
  quinze minutes. Et le journal s'écrit pendant que ça tourne — la jauge
  de `curl` comprise, qui réécrit sa ligne au lieu d'en empiler mille.
- **Le binaire n'en grossit pas** : 61 Ko, soit 0,34 %. Rien n'est
  embarqué — `curl` télécharge, `tar` extrait, `meson` et `cmake`
  construisent, comme `nasm` et `ld` assemblent déjà. Un test refuse
  qu'une bibliothèque de téléchargement ou d'archive rejoigne le
  programme.

**La page Outils des Préférences, relue**
- « absent » à côté d'un bouton « Retirer » : la page mélangeait deux
  choses qui n'ont rien à voir. Chaque outil a maintenant sa fiche, et
  elle dit les deux séparément — **le programme est là, ou non** ; et
  **namsdec s'en sert, ou non** — en deux pastilles côte à côte. Les
  boutons parlent le même langage : « S'en servir », « Ne plus s'en
  servir ».
- Ce à quoi sert l'outil se lit sans le survoler. La commande exacte et
  la façon d'obtenir ce qui manque se déplient — et sont dépliées
  d'office quand l'outil est absent, puisque c'est alors la seule chose
  qu'on cherche.
- Quand namsdec sait installer un outil mais que le module est éteint, il
  le **dit**, avec le bouton qui l'allume. Avant, le bouton « Installer »
  n'apparaissait pas et rien n'expliquait pourquoi.
- **Les boutons OK et Quitter ne sortent plus de la fenêtre.** Chaque page
  avait sa zone de défilement réglée pour prendre toute la hauteur
  offerte : la boîte s'étirait jusqu'en bas de l'écran pour cinq lignes de
  contenu. Une seule zone qui s'arrête avant la barre du bas, une boîte de
  780 × 760 posée au centre, et un test qui vérifie les cinq pages.

**Lire ce que la décompilation rend**
- RetDec rend deux millions de caractères pour un petit crackme. En un
  seul bloc de texte, egui remettait tout en page à chaque image : la
  fenêtre se figeait. On ne peint plus que les lignes qui tiennent à
  l'écran — soixante mille lignes se parcourent sans à-coup.
- Avec de quoi s'y retrouver : un **filtre** qui ne garde que les lignes
  cherchées en disant combien elles sont, une liste des **fonctions**
  repérées pour y sauter, et des numéros de ligne dans la marge.

**Un vrai chargeur**

Ouvrir un binaire s'arrêtait sur « rien n'est projeté à l'adresse … : ni
le programme, ni la pile, ni le tas » — et l'adresse fautive était son
propre point d'entrée. La cause tenait en une ligne ; ce qu'elle a
révélé tenait en un chantier. namsdec posait les octets d'un fichier en
mémoire ; il **charge** désormais un programme, comme le noyau le fait.

- **Le programme est projeté en entier.** Une zone mémoire était
  identifiée par son seul nom : un vrai exécutable a quatre sections de
  code — `.init`, `.text`, `.fini`, `.plt` — annoncées toutes les quatre
  comme « .text », et chacune effaçait la précédente. Il ne restait
  projeté que la dernière déclarée. Une zone se reconnaît maintenant à son
  nom **et** à son adresse de départ ; le tas continue de grandir sur
  place comme avant.
- **Le chargement se fait par segments**, comme celui du noyau, et non
  plus par sections : des pages entières, les droits que le fichier
  déclare, le `.bss` réservé même quand il ne pèse rien. Entre la fin
  d'une section et le début de la suivante, un vrai processus n'a pas de
  trou — et namsdec n'en a plus non plus. Un binaire **dépouillé**
  (`strip`), qui n'a plus une seule section, se charge désormais aussi
  bien qu'un autre.
- **Les relocations sont appliquées.** `DT_RELA`, `DT_JMPREL` et le
  format compact `DT_RELR` sont lus dans la table dynamique et posés
  avant la première instruction. Sans elles, un exécutable indépendant de
  la position démarre sur des tables de pointeurs entièrement nulles, et
  le premier appel indirect saute à zéro : 355 cases pour un petit
  crackme en Rust, 0 auparavant.
- **La mémoire locale au fil d'exécution existe**, et la base `FS` avec
  elle. Le bloc `PT_TLS` est recopié, le pointeur de fil posé dessus,
  `fs:0` pointe sur lui-même et `fs:0x28` porte un canari de pile —
  reconnaissable à l'œil dans un vidage, et toujours le même, pour qu'une
  exécution se rejoue à l'identique. `arch_prctl` permet au programme de
  la déplacer lui-même, comme sous Linux.
- **La pile initiale est celle d'un processus** : `argc`, `argv` (le nom
  du fichier ouvert), l'environnement, puis le vecteur auxiliaire —
  `AT_PHDR`, `AT_ENTRY`, `AT_PAGESZ`, `AT_RANDOM` et les autres —, le
  tout aligné sur seize octets comme l'exige la convention d'appel. Le
  programme démarrait jusqu'ici sur une pile de zéros, où `argc` valait
  zéro.
- **Un binaire C lié statiquement s'exécute entièrement**, bibliothèque C
  comprise : cent trente-sept mille instructions de démarrage de la
  glibc, puis `main`, puis `printf`, puis la sortie avec son code. Un
  test le vérifie en compilant un programme et en le faisant tourner
  jusqu'au bout.
- **Un appel qui sort du programme dit qui il appelait.** Chaque fonction
  qu'une bibliothèque aurait dû fournir reçoit une borne : sauter dessus
  s'arrête sur « appel à une fonction de bibliothèque : `__libc_start_main` »
  au lieu de « on ne saute pas dans des données », qui envoyait chercher
  une faute de pointeur là où il n'y en avait pas.
- **Ce que le chargeur a fait est dit** quand il n'a pas tout pu faire :
  combien de segments, combien de relocations, combien d'octets de TLS,
  et les fonctions manquantes nommées.
- La carte mémoire **nomme les sections telles qu'elles sont** : `.got`,
  `.plt`, `.dynamic`, `.data.rel.ro`, `.eh_frame`… Savoir qu'on est tombé
  dans la table de liaison plutôt que dans « des données » est souvent
  toute la réponse à la question qu'on se pose.
- **Les appels système du démarrage d'une bibliothèque C** sont simulés :
  `mmap` (avec sa propre arène, hors du tas), `mprotect` — appliqué pour
  de vrai, ce qui rend RELRO observable —, `munmap`, `fstat`, `ioctl`,
  `writev`, `readv`, `uname`, `prlimit64`, `getrlimit`, `arch_prctl`,
  `set_tid_address`, `set_robust_list`, `rseq`, `futex`, `clock_gettime`,
  `readlink`. Il suffit d'un seul manquant pour qu'un programme n'arrive
  jamais à son `main`.
- **Six instructions de plus** : `movlpd`/`movhpd` et leurs cousines, que
  toute routine de chaînes emploie ; `pshuflw`/`pshufhw` ; et les
  instructions de pile fantôme (`rdssp`, `incssp`…), qui ne font rien sur
  une machine sans le dispositif — ce qui est précisément ainsi qu'une
  bibliothèque teste sa présence. Une fiche les explique toutes.
- Un binaire **lié dynamiquement** se lit, s'analyse et se désassemble
  comme avant ; il s'exécute maintenant jusqu'à son premier appel à la
  bibliothèque, et s'arrête en le nommant.

**Une bibliothèque C, écrite en Rust**

Un exercice de rétro-ingénierie est presque toujours lié dynamiquement :
il affiche une invite, lit une réponse, la compare. S'arrêter sur
`__libc_start_main` revenait à ne jamais le voir tourner.

- namsdec **fournit** une centaine de fonctions de bibliothèque, écrites
  en Rust : `printf` et ses cousines (`sprintf`, `snprintf`, `fprintf`,
  avec les conversions, la largeur, la précision et les flottants),
  `puts`, `fgets`, `gets`, `scanf`, toutes les fonctions de chaînes et de
  mémoire, `malloc` et sa famille, et les enveloppes des appels système.
  `__libc_start_main` appelle `main` pour de vrai, et son retour devient
  le code de sortie.
- Elles obéissent aux règles de la maison : les **droits** de la mémoire
  simulée s'appliquent — un `strcpy` vers `.rodata` échoue ici comme sur
  la vraie machine —, `malloc` prend sa mémoire **dans le tas**, où la
  carte la montre, chaque appel compte pour **un pas**, et ce qu'il écrit
  se défait avec « pas en arrière ».
- Rien n'est approché : une fonction qui n'est pas fournie **s'arrête en
  se nommant**, avec ce qui marche malgré tout. Le compte se lit au
  chargement — « namsdec fournit 46 des 59 fonctions empruntées ; il ne
  fournit pas : … » —, ce qui répond avant de commencer à la seule
  question qui se pose : est-ce que ce programme-là va tourner ?
- Résultat, sur un crackme ordinaire : l'invite s'affiche, la saisie est
  lue, la comparaison a lieu, le verdict s'écrit. Deux tests le
  vérifient en compilant un programme C, lié dynamiquement puis
  statiquement, et en le faisant tourner jusqu'à son code de sortie.
- Une fiche **F1** explique tout ce qui se passe avant la première
  instruction : segments, relocations, TLS, pile de processus, et où
  s'arrête namsdec.
- La mémoire simulée ne fait plus soixante-quatre kilo-octets : elle
  s'adapte au programme chargé. Un binaire de trois cent mille octets n'y
  entrait pas, namsdec retombait sur un désassemblage brut tassé à
  `0x400000`, et l'on croyait lire le fichier ouvert alors que toutes les
  adresses étaient décalées. Un exercice, lui, retrouve exactement la
  fenêtre d'avant — même base, même taille, même sommet de pile.
- Un binaire **lié dynamiquement** le dit dès son ouverture : il se lit,
  se désassemble et s'analyse, mais ne s'exécute pas ici — les fonctions
  qu'il emprunte ne sont pas là. Mieux vaut l'apprendre au chargement que
  sur un « rien n'est projeté à cette adresse » au premier appel.
- **Réassembler le listing d'un binaire** est refusé, en le disant : cela
  donnait un programme sans ses données ni sa table de liaison, qui
  partait dans le décor à la première lecture.
- Les outils d'analyse ont **leur propre onglet** dans les préférences,
  et une ligne chacun : ils étaient au bas de la page des plugins, sur
  quatre lignes, et seul le premier entrait dans la fenêtre — Rizin et
  RetDec étaient là sans que personne puisse les voir.

**Un module de décompilation, chargé à la demande**
- Cinq outils connus d'avance, avec les arguments qui marchent :
  `objdump`, `ndisasm`, le pseudo-code de `radare2`, **Rizin + rz-ghidra**
  (`pdg`) et **RetDec**. Préférences ▸ Plugins dit lesquels sont présents
  sur la machine, lesquels manquent, et ce qu'il faut taper pour les
  obtenir — puis les déclare d'un clic, ce qui écrit un fichier et rien
  d'autre.
- Rizin a droit à une nuance que personne d'autre ne fait : le programme
  peut être installé sans que la décompilation le soit, faute du greffon
  `rz-ghidra`. namsdec le lui demande et répond « incomplet » plutôt que
  de laisser essayer.
- Le format de description accepte `{out}` : un outil qui écrit son
  résultat dans un fichier — RetDec — est désormais lisible, là où
  namsdec n'attendait que la sortie standard.
- Un outil extérieur — `objdump`, `ndisasm`, RetDec, le vôtre — décrit
  par un fichier JSON dans `~/.local/share/namsdec/decompilers/`, choisi
  dans une liste et lancé sur le programme lié.
- Éteint par défaut, et pour une raison qui est dite : ce module
  n'affiche pas, il **exécute**. La case des préférences, l'entrée de
  menu qui n'apparaît qu'ensuite, et le bouton « Lancer » font trois
  verrous — et la commande complète est écrite avant d'être lancée.
- Le travail se fait dans un fil à part : l'interface reste utilisable, et
  un outil qui ne rend pas la main au bout de deux minutes est réellement
  arrêté, pas attendu.
- Ce qui tourne se voit à deux endroits : une **barre de progression dans
  la barre d'état**, et le **titre de la fenêtre** — donc la barre des
  tâches du système, où l'on regarde quand on a changé d'application. La
  progression est celle du temps accordé, seule chose qu'un programme
  extérieur nous laisse savoir ; quand rien ne borne, la barre avance
  toute seule plutôt que d'inventer un pourcentage.

**Lire un binaire qu'on n'a pas écrit**
- Un module **Fonctions** : le `main` d'un binaire dépouillé, retrouvé
  par la convention de démarrage, puis les fonctions découpées sur les
  cibles d'appel, avec leurs appelants.
- Les chaînes des langages sans zéro final — Rust, Go, Zig — sont
  découpées **par le code**, où l'adresse voyage avec sa longueur. Un
  scan des octets lisait « Error flushing bufferError reading
  inputIncorrect password » d'un seul tenant.
- Un module **Figures** : constantes qui sont du texte, contrôles de
  longueur, comparaisons déguisées en XOR, comparaisons de chaîne,
  tables de sauts. Rangées de la plus rare à la plus commune — c'est ce
  classement qui fait sortir une piste d'un binaire de soixante mille
  instructions.
- **La route vers une adresse** : dans le déroulement, un champ répond à
  « comment arrive-t-on là ? » en listant les conditions à satisfaire
  dans l'ordre, et met en avant les blocs du chemin. La plus courte —
  namsdec dit quand il en existe d'autres.
- **La provenance lue dans le code** : à défaut d'historique, namsdec
  remonte les instructions à l'envers jusqu'à la source d'une valeur, et
  s'arrête au premier saut plutôt que de deviner.

**Un guide de l'utilisateur**
- Un PDF de neuf pages accompagne la version : installation, prise en
  main, chaque panneau, l'exécution, la mémoire, le manuel, le
  déroulement, le parcours, la rétro-ingénierie, les raccourcis, les
  fichiers et les limites connues.
- Il est produit depuis le dépôt : version, nombre d'exercices, nombre de
  fiches, nombre de tests et table des raccourcis sont lus dans les
  sources. Un test refuse qu'une seule de ces valeurs y soit recopiée.

**Le déroulement du programme**
- Une fenêtre, ouverte par `Espace g`, dessine le graphe des blocs : un
  listing se lit de haut en bas, un programme non. Chaque boîte est une
  suite d'instructions qu'on ne peut qu'entrer par le haut et quitter par
  le bas ; les flèches disent le saut pris, le saut non pris et l'appel ;
  une boucle se voit à ce qu'elle remonte, en passant par le côté.
- Le bloc où le programme se trouve est mis en avant à chaque pas, un
  clic y emmène dans l'éditeur, et Ctrl+molette recule pour voir la forme
  d'ensemble.
- Un saut dont la cible se calcule — table de sauts, pointeur de
  fonction — est signalé comme tel : aucune flèche n'est inventée vers
  une adresse qu'on ne connaît pas.

**Le parcours sort du binaire, et passe à soixante et onze étapes**
- Les exercices ne sont plus compilés dans le programme : c'est un
  fichier de données, `namsdec.json`, installé à côté des thèmes et lu
  au démarrage comme n'importe quelle série. Corriger un énoncé ne
  demande plus de recompiler, et ce qu'on écrit soi-même se range
  exactement comme ce qui est livré.
- Vingt-cinq étapes encore, portées à **96** : ce que l'assembleur
  calcule pour vous, les rotations, `setcc`, `not` et `neg`, la division
  signée et le `cqo` qu'elle exige, ce que coûte un immédiat ; les
  tableaux d'octets, les zones qui se chevauchent (`memmove` contre
  `memcpy`), `rep stosb`, la recherche de bit, l'addition sur cent
  vingt-huit bits, la table de conversion, les trous d'une structure ;
  la concaténation, l'extraction d'un mot, la récursion terminale qui
  devient une boucle, le tri par insertion à comparer au tri à bulles
  dans le Profil, la recherche binaire et son calcul de milieu qui
  déborde, la liste chaînée, la transposition ; la somme vectorielle et
  son repliement final, tronquer contre arrondir, la table ASCII en un
  seul `write`, l'affichage d'un négatif et le nombre sur lequel la
  valeur absolue ment, et la lecture jusqu'à la fin.
- Vingt-quatre étapes de plus avant elles, portées alors à 71 : les trois écritures
  d'un nombre, `inc` contre `add` et la retenue qu'il épargne, `test`
  qui regarde sans écrire, `xchg` et `bswap`, les masques de bits ; le
  minimum et le maximum en un passage, la parité en un bit, les
  occurrences, le retournement sur place, le bornage sans branchement,
  les puissances de deux et leur limite, la somme et la moyenne ;
  `strlen` par `repne scasb`, le mot à l'envers, le compte de mots, la
  pile qui retourne ce qu'on lui donne, les arguments au-delà de six,
  le pointeur de fonction, l'alignement de pile sur seize ; l'affichage
  hexadécimal, le chiffrement XOR et sa faiblesse, la somme de
  contrôle, le canari posé à la main, et le choix vectoriel par masque.
- Le test qui vérifiait les budgets exécute désormais **tous** les
  exercices : un exercice qui n'atteint pas son propre but est faux,
  budget ou non. Il a trouvé deux énoncés qui comptaient mal.
- Le binaire passe de 25,1 à **17,8 Mo** — l'édition de liens voit tout
  le programme d'un coup et jette ce qui n'est appelé nulle part. Rien
  n'est effacé pour autant : les symboles restent, un rapport de
  plantage doit pouvoir se lire.

**Plusieurs fichiers à la fois**
- Une barre d'onglets au-dessus de l'éditeur : un onglet par fichier, le
  nom raccourci par le milieu quand il est long, un point sur ce qui
  n'est pas enregistré, une croix pour fermer. Elle n'apparaît qu'à
  partir du deuxième fichier.
- Chaque onglet garde ce qu'on y faisait : le texte, le curseur, les
  points d'arrêt et leurs conditions, l'exercice en cours, et
  l'exécution arrêtée au pas où on l'avait laissée. Y revenir, c'est
  reprendre la phrase au milieu.
- Ouvrir un fichier ne demande donc plus si l'on veut perdre le
  précédent : il attend dans son onglet. Ctrl+Tab passe au suivant,
  Ctrl+Maj+Tab au précédent, Ctrl+W ferme.
- Un fichier changé sur le disque pendant qu'on regardait ailleurs — un
  `git checkout`, un autre éditeur — ne passe plus inaperçu : rien de
  tapé de notre côté, il est relu et namsdec le dit ; modifié des deux
  côtés, un bandeau propose de relire ou de garder. La vérification a
  lieu au retour dans la fenêtre et en changeant d'onglet, jamais à
  chaque image.
- Chaque exercice a son onglet, et porte son titre : on ouvre le suivant
  sans abandonner le précédent, et l'on y revient avec ce qu'on y avait
  écrit. Trois onglets « sans titre » côte à côte ne disaient rien.
- Fermer un onglet dont le texte n'est pas enregistré demande d'abord, et
  quitter regarde **tous** les fichiers ouverts : la fenêtre dit combien
  sont en danger, et « Enregistrer » passe au suivant au lieu
  d'emporter le reste.
- Les fichiers ouverts reviennent au lancement suivant — leurs chemins,
  jamais leur contenu, comme pour le reste.

- La mémoire lue « en Texte » s'explique : `<b8><01>` suivi de `\0` ne
  disait ni que le zéro arrêtait la lecture, ni que ces octets n'étaient
  pas du texte, ni où se trouvait le message cherché. Elle dit maintenant
  les trois — et liste, avec leurs adresses, les suites lisibles de la
  fenêtre, comme `strings`.
- La pile ne coupe plus ses valeurs au bord du panneau : elle défile de
  côté, et sur un panneau étroit l'adresse laisse la place à l'écart
  depuis RSP (`+0x10`, `-0x08`) — plus court, et il dit quelque chose de
  plus : à quelle profondeur on regarde. L'adresse complète reste au
  survol. Le panneau, lui, ne se resserre plus en deçà d'une valeur de
  seize chiffres, barre de défilement comprise.
- L'onglet Mémoire lit son format dans une liste déroulante : « Octets
  Mots Texte Pointeurs » ne tenait pas sur la largeur d'un panneau et
  repassait à la ligne par-dessus le reste.
- Un filet sépare les trois zones du panneau latéral, et Affichage ▸
  « Panneau latéral partagé tout seul » rend le partage automatique à qui
  a tiré la poignée une fois.
- Le panneau latéral partage sa hauteur au lieu de la donner au premier
  arrivé : l'état du processeur garde la sienne, entière et sans barre
  de défilement, et le contenu de l'onglet — Pile, Mémoire, Tas, Octets,
  RE — a la sienne, avec une poignée pour la régler et un plancher en
  dessous duquel il ne descend plus. Il sortait jusque-là par le bas du
  panneau : l'onglet RE se réduisait à son titre et à un champ de
  recherche coupé en deux. Ce qui cède, désormais, c'est le détail de la
  prochaine instruction, qui se replie d'un clic. L'onglet RE mène en
  outre à la fenêtre d'analyse, où tout se lit déplié.
- Un appel système que namsdec ne simule pas — `mmap`, `open`, `fork` —
  est annoncé dès l'assemblage, avec sa ligne : le programme s'arrêtait
  dessus sans que rien ne l'ait laissé prévoir.
- La console ne tend plus un champ de saisie à un programme qui ne lit
  rien : elle relit son code, et la saisie n'apparaît que s'il contient
  un `read` — ou dès qu'il en réclame une. Vide, elle dit aussi
  *pourquoi* : « ce programme n'écrit rien » quand il ne contient aucun
  `write`, « rien pour l'instant » tant qu'il n'a pas tourné, et « le
  programme n'a rien écrit » seulement quand c'est vrai.
- La console prend la hauteur de ce qu'elle a à dire : « le programme n'a
  rien écrit » occupait le quart de l'écran, c'est maintenant trois
  lignes. Tirer la poignée reprend la main, et la hauteur choisie est
  retenue ; Affichage ▸ « Console à la hauteur de son contenu » rend
  l'ajustement.

## 0.2.1-beta.2 — 17 septembre 2026

La version qui emprunte ses gestes à [pwndbg](https://pwndbg.re/) : ce
qu'on fait devant un binaire porte ici les mêmes noms qu'ailleurs.

**Lire la mémoire**
- Les pointeurs se suivent jusqu'au bout (`telescope`) : dans l'onglet
  Mémoire, dans chaque mot de la pile, devant chaque argument d'appel.
- La carte des zones et leurs droits (`vmmap`), la recherche d'un texte,
  d'octets ou d'une valeur (`search`), et l'écriture à la main (`patch`)
  — le tout sous « Outils mémoire ».
- L'onglet suit RSP ou RIP pas à pas, si on le lui demande.

**Exécuter**
- « Jusqu'au prochain appel, retour, appel système » (`nextcall`,
  `nextret`, `nextsyscall`).
- Un registre se pose d'un clic, un indicateur se bascule d'un clic — et
  la valeur posée entre dans l'histoire du programme, que le retour en
  arrière défait comme le reste.
- Les appels système qui ne font que renseigner répondent enfin :
  `getpid`, `getuid`, `geteuid`, `sched_yield`, `nanosleep`,
  `getrandom` (octets reproductibles, et namsdec le dit).

**Comprendre un binaire**
- Chaque entrée de la table de liaison retrouve son nom : 105 fonctions
  sur `/bin/ls`, avec le nombre d'appels qui y mènent, une phrase par
  fonction pour les cinquante plus courantes, et un repère sur celles
  qui écrivent sans borne. Ces noms deviennent des étiquettes du listing
  (`malloc@plt:`).
- Les adresses de retour de la pile se listent (`retaddr`).
- Les chaînes sont cherchées dans les sections de données, comme
  `strings -d` : le panneau montrait jusque-là des fins de fonctions et
  s'arrêtait avant d'atteindre les vrais messages. Le rapport en reprend
  les plus parlantes.
- Le motif cyclique de De Bruijn (`cyclic`), identique octet pour octet
  à celui de pwntools, avec envoi direct dans l'entrée du programme et
  décalage relu depuis un registre, des caractères ou un nombre.
- Le rapport d'analyse s'enregistre dans un fichier ; l'état du
  processeur se copie en texte (`context`).

**Apprendre**
- Le manuel se parcourt : sommaire par famille, renvois déduits du texte
  des fiches, retour en arrière, export complet en Markdown. 144 fiches.
- Le parcours passe à seize étapes : division, ordre des octets, signé
  ou non, et « ce qu'un tampon trop petit laisse faire » — mesuré par le
  motif cyclique.
- Chaque exercice montre les fiches que son propre code emploie.
- F1 répond aussi sur une fonction empruntée.

**Le parcours passe à quarante-sept étapes, rangées par difficulté**
- Trois de plus : « D'un texte à un nombre » (`atoi`, et ce qu'`atoi` ne
  rend pas — le texte était-il un nombre ?), « Un enregistrement, et ses
  champs » (`struc`, et le calcul base + indice × taille + champ que
  fait un compilateur derrière `gens[i].age`), et « Trier, et voir ce
  que ça coûte » (le tri à bulles, dont l'onglet Profil montre où part
  le temps — O(n²) se lit au lieu de se croire sur parole).

- Huit de plus : les quatre indicateurs provoqués un par un, le
  complément à deux vu de près, `popcnt` contre la boucle qui compte les
  bits, la matrice et pourquoi on la parcourt ligne par ligne, `cmov` et
  la prédiction de branchement, Fibonacci sans un seul `call`, le tableau
  local sur la pile, et la mise en majuscules — un seul bit d'écart dans
  la table ASCII.

- Six de plus : « Mettre à zéro, trois façons » (sept octets contre deux,
  et pourquoi les binaires sont pleins de `xor`), « La boucle que le
  processeur connaît » (`loop`, et pourquoi elle a disparu), « La même
  adresse, quatre lectures » (byte/word/dword/qword), « Chercher dans un
  tableau » (et la sortie qu'on oublie), « Deux chaînes sont-elles les
  mêmes ? » (`repe cmpsb`), « Ce qu'une fonction a le droit d'abîmer »
  (la convention d'appel, avec l'entorse que namsdec relève).

- Quatre paliers, nommés par ce qu'on y fait : **Découverte** (ce que le
  processeur est), **Pratique** (ce qu'il manipule), **Structures** (ce
  qu'on construit avec), **Maîtrise** (ce qui l'entoure). La liste les
  annonce en tête de chaque groupe, et un test vérifie que le parcours ne
  redescend jamais d'un palier.
- Quatre exercices de plus : « Échanger deux registres » (`xchg` contre
  les trois `xor`), « Multiplier sans multiplier » (`shl`/`shr`/`sar`, et
  ce que le signe change), « Choisir parmi plusieurs » (la table de saut,
  et pourquoi rien n'y vérifie l'indice), « Quatre additions d'un coup »
  (`paddd`, et l'alignement que `movdqa` exige).

- « Déclarer des données » : `db`/`dw`/`dd`/`dq`, `times`, `equ`, `$` — et
  l'onglet Octets pour voir ce que chaque directive a écrit.
- « Calculer une adresse » : `lea` contre `mov`, et la multiplication par
  cinq la moins chère du jeu d'instructions.
- « Avant l'assemblage » : `%define`, `%macro`, `%rep` — une macro n'est
  pas un appel, et les octets le prouvent.
- « Une fonction qui s'appelle elle-même » : la récursion, la chaîne des
  appels qui s'allonge, et d'où vient le débordement de pile.
- « Lire ce qu'on vous donne » : `read` rend ce qu'il a pu lire, jamais
  ce qu'on lui a demandé.

- « Copier sans boucle » : `rep movsb`, et d'où RSI et RDI tirent leur
  nom.
- « Un cadre de pile » : `push rbp` / `mov rbp, rsp` / `sub rsp, 16`, les
  variables locales lues depuis RBP, et `leave`.
- « Demander de la mémoire » : `brk`, sur quoi repose tout `malloc`.
- « Calculer à virgule » : XMM0, `movsd`, `addsd`, et pourquoi le `s`.
- « Afficher un nombre » : la synthèse — une boucle, `div` et son RDX, la
  pile, l'arithmétique des caractères et un appel système.

**Un budget par exercice**
- Chaque exercice mesuré annonce le nombre d'instructions qu'exécute la
  solution livrée. Le bandeau montre ce que coûte la vôtre, le budget, et
  votre meilleur score ; un anneau marque les exercices passés en
  dessous, et la série en tient le compte.
- Un test vérifie que chaque budget est tenable, solution à l'appui.

**Vérifier ce qu'on a fait**
- Un exercice réussi le reste : pastille dans la liste, compte en tête de
  la série, retenus sur le disque. Repartir du programme de départ ne le
  défait pas — on ne retire pas ce qu'on a compris.
- Onze exercices portent un but en conditions (`rax == 0xf`), vérifié
  sur les registres : le bandeau dit « atteint » ou « pas encore ». Une
  série installée peut en porter aussi (champ `goal`).

**Sortir de l'atelier**
- Fichier ▸ Exporter le binaire : l'ELF que `ld` a produit, rendu
  exécutable, celui qu'on lance dans un terminal ou qu'on lit avec
  `objdump`.
- L'onglet Diagnostics affiche les deux commandes réellement lancées,
  prêtes à copier.

**Mises à jour**
- « Rechercher une mise à jour » est branché sur les versions publiées du
  dépôt : comparaison SemVer complète (préversions comprises), et un lien
  vers la version trouvée. Rien n'est envoyé, rien n'est installé — la
  mise à jour reste un geste. La liste des versions est interrogée, et
  non `releases/latest`, qui **ignore les préversions** et répond 404 à
  un dépôt qui n'a publié que des bêtas.
- Le dépôt sait publier : une étiquette `v…` déclenche la compilation, les
  vérifications et la publication du binaire avec sa somme SHA-256.

**Corrigé**
- Ouvrir un fichier objet `.o` se plaignait de la mémoire — « le binaire
  dépasse la fenêtre simulée » devant cinq cents octets. La vraie raison
  est que `ld` n'est pas passé : il se lit et s'analyse, mais ne
  s'exécute pas, et le message le dit avec la commande qui manque.
- Aucun lien ne s'ouvrait : `open_url` d'eframe est derrière la
  fonctionnalité `links`, que le projet n'activait pas. Les boutons de
  la boîte « À propos », le lien de la licence et celui d'une version
  publiée partaient dans le vide.
- Les références aux données relatives à RIP étaient toutes perdues :
  iced résout déjà l'adresse, et y rajouter `next_ip` l'envoyait hors du
  programme. « Qui lit cette chaîne ? » ne répondait rien pour aucun
  programme moderne. L'avertissement du lint sur les écritures en
  lecture seule ratait le même cas.
- Ouvrir un binaire montrait son en-tête ELF décodé comme des
  instructions : le désassemblage part maintenant des sections
  exécutables que le fichier désigne.

**Place à l'écran**
- Une légende sous l'éditeur dit ce que valent les couleurs de la
  gouttière, avec des repères pris sur l'échelle réelle du programme.
  Préférences ▸ Général l'éteint.
- Le panneau latéral nomme la vue qu'il montre : depuis que les onglets
  sont dans la barre d'outils, une colonne d'adresses n'annonçait plus
  qu'elle était une pile. Même facture de titre pour « Prochaine
  instruction ».
- L'onglet Mémoire tient « Suivre » et les quatre lectures sur une seule
  ligne, au lieu de trois rangées avant le premier octet.
- Les zéros de tête des valeurs 64 bits passent au second plan :
  `0000000000` puis `40ff00` en clair. Les colonnes restent alignées, mais
  l'œil attrape la valeur au lieu de compter le remplissage.
- Le bloc de la prochaine instruction se replie, et son détail — fiche,
  opérandes résolus, arguments — est derrière un repli qui s'ouvre de
  lui-même devant un appel, un saut conditionnel ou un appel système.
- Une chaîne de l'onglet RE se tronque au bord du panneau, et non à la
  largeur idéale de son texte : les lignes s'arrêtaient au milieu d'un
  panneau à moitié vide.
- Les modules de rétro-ingénierie sont dans des cadres, et seul le résumé
  est ouvert dans le panneau latéral. La fenêtre d'analyse, elle, se lit
  en entier et les déplie tous.
- Les onglets du panneau latéral — Pile, Mémoire, Tas, Octets, RE — sont
  passés à droite de la barre d'outils : ils ne montent plus et ne
  descendent plus avec le bandeau, et le panneau gagne leur hauteur.
- La gouttière peint une bande de chaleur au lieu d'écrire « ×1203 » :
  l'échelle est logarithmique — vert, jaune, orange, rouge, du parcouru
  une fois au plus chaud du programme —, le compte exact vient au
  survol, et la colonne du compteur disparaît avec ses quarante-quatre
  pixels. Une légende sous l'éditeur donne les valeurs, et Préférences ▸
  Affichage la retire.
- Les colonnes du profil tombent sous leur en-tête : « ligne passages
  part » était écrit d'un seul trait, avec des espaces qui ne mesuraient
  rien. Une grille les aligne, les nombres sont calés à droite, et les
  pourcentages s'écrivent comme la langue les écrit — « 30,0 % » en
  français.
- Un onglet **Profil**, dans la console, classe les lignes par temps
  passé dedans : le nombre de passages, la part du total, la ligne
  elle-même, et un clic qui y emmène. La bande de chaleur dit où ça
  chauffe ; le profil dit de combien, et se copie en texte. Quand un
  exercice impose un budget d'instructions, le profil le rappelle et,
  s'il est dépassé, désigne la ligne par où reprendre. Chaque ligne y
  est nommée par son bloc — « boucle: dec rcx » — pour n'avoir pas à
  retourner dans le programme.
- La fenêtre porte ses trois pastilles **en tête** de la barre de menus,
  qui fait office de barre de titre : une rangée de moins. Fermer,
  réduire, agrandir — l'ordre de macOS, qui va avec le côté gauche : la
  fermeture occupe le coin de l'écran, l'angle qu'une souris lancée sans
  viser atteint, et les deux autres sont derrière elle. Préférences ▸
  Général rend la barre du système à qui la préfère.
- La boîte « À propos » mène quelque part : namsdec.io, le Discord, le
  dépôt.
- L'interpréteur exécute les comparaisons flottantes en masque —
  `cmpps`, `cmppd`, `cmpss`, `cmpsd` et leurs formes nommées —, y compris
  ce que les prédicats en `n` font d'un NaN : `cmpnltsd` n'est pas
  `cmpgesd`. Une 153ᵉ fiche les explique, et le test qui relit les
  binaires du système passe à **soixante** programmes avec la plus forte
  exigence possible : plus une seule instruction, même vue une fois, que
  le manuel ne sache expliquer.
- La palette de commandes mène aux **étiquettes du programme** : on tape
  `affiche_nombre`, on y est. Sur deux cents lignes, c'est le « ouvrir
  par son nom » des éditeurs, sans rien de nouveau à retenir.
- L'état du processeur ne se peint plus par-dessus la vue du dessous.
  Il vivait dans un panneau hors zone de défilement : un tel panneau ne
  rogne pas ce qui dépasse, il le laisse déborder — les indicateurs, le
  coût et le lien des registres étendus se superposaient à l'onglet, les
  deux illisibles. Le bandeau tient maintenant dans une seule zone qui
  coupe net, et la barre n'apparaît que sur une fenêtre trop courte.
**Tenue**
- Un bandeau dit, dans « À propos », que la version est une préversion.
  Il est lu dans le numéro de version et disparaîtra de lui-même.
- 476 tests, dont : les deux listes d'appels système tenues l'une à
  l'autre, vingt binaires de `/usr/bin` lus sans broncher, la palette
  qui doit proposer toutes les commandes, le README vérifié par le
  programme lui-même.

## 0.1.99-beta.2 — 17 septembre 2026

Chronologie du temps passé (un clic sur un pas y revient, mémoire et
registres compris), points d'arrêt conditionnels, provenance d'une
valeur (« qui a écrit ici ? »), panneau de désassemblage à moteurs
interchangeables — pseudo-code natif d'abord —, fragments ROP, chaînes
et leurs références, couverture complète des directives du manuel NASM.

## 0.1.99-beta.1 — 17 septembre 2026

Rétro-ingénierie : protections d'un binaire (PIE, NX, RELRO, canari,
FORTIFY, IBT), segments, sections, symboles, références croisées,
boucles, appels système repérés dans le code. Le manuel devient
consultable sous F1 et au survol.

## 0.1.1 — 15 septembre 2026

Console, bandeau de l'état du processeur, réglages persistés dans
`settings.json`, thèmes et exercices installables, explorateur de
fichiers.

## 0.0.2 — 14 septembre 2026

Le premier atelier complet : assembler avec le vrai `nasm`, exécuter pas
à pas dans un interpréteur x86-64 écrit à la main, revenir en arrière.
Licence MIT.
