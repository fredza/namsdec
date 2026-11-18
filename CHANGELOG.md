# Journal des versions

Les dates sont celles des commits ; le format suit [Keep a
Changelog](https://keepachangelog.com/fr/1.1.0/) d'assez loin — une
entrée par version, et ce qui change pour qui s'en sert.

## 0.4.2 — 18 novembre 2026

Ce qui justifie le chiffre, du point de vue de qui s'en sert : **chaque
exercice explique maintenant la mécanique sur laquelle il repose**, et
**l'installateur met à jour sans rien fermer avant d'avoir fini**.

**Une fiche de théorie pour chacun des 117 exercices**
- Ni la consigne, ni l'indice. La consigne dit ce qu'on demande,
  l'indice dit où chercher quand on bloque ; la **théorie** dit comment
  la machine s'y prend — ce qu'est une boucle quand le processeur n'en
  a pas, pourquoi écrire `eax` efface le haut de `rax`, pourquoi une
  division réquisitionne deux registres sans les nommer.
- Écrite **pour qui débute** : une idée par paragraphe, des phrases
  courtes, un exemple avec de vrais nombres, et le piège à la fin.
- Elle s'ouvre dans **sa propre fenêtre**, par le bouton *La théorie*,
  à côté de *Un indice* et *La solution* — l'ordre dans lequel on y
  vient. Les mots qu'elle cite mènent aux fiches, instructions comme
  notions.
- Cinq contrôles la tiennent : chacune existe dans les deux langues,
  ne recopie pas sa consigne, cite au moins une fiche qui existe, n'est
  écrite qu'une fois, et se lit d'un trait.

**L'installateur met à jour**
- Le relancer est une **mise à jour**, pas une réinstallation : rien
  n'est retiré, et il le dit en tête.
- Chaque fichier est écrit à côté puis **renommé**. L'icône ne
  disparaît plus du dock pendant l'installation — le lanceur ne passe
  plus par un état vide.
- **Si namsdec est ouvert, il le reste** pendant toute la compilation,
  et n'est refermé qu'à la fin, proprement, avant d'être rouvert sur la
  version neuve. `--no-restart` l'en empêche. Une fenêtre qui ne part
  pas n'est jamais forcée.

**Ce que l'écran disait de travers**
- Six captures sur cent trente-sept montraient autre chose que l'écran :
  un cache de mise en page gardait un texte posé pour un atlas de
  polices qui n'existait plus. Dans l'application, cela serait arrivé au
  bout d'une séance.
- La bande de la ligne courante perdait sa première ligne par le haut et
  sa dernière par le bas, sans rien pour les atteindre. Elle se donne
  maintenant une hauteur exacte, et ce qui dépasse défile.
- Le bouton de don d'« À propos » annonçait qu'il était éteint et se
  peignait en pleine couleur.

## 0.4.0-beta.1 — 10 au 16 novembre 2026

Ce qui justifie le chiffre du milieu, du point de vue de qui s'en sert :
**le parcours demande enfin quelque chose, et le mode débutant est une
interface à part entière**. Sur les 101 étapes qui posent un but, **97
l'atteignaient telles qu'elles étaient livrées** — on ouvrait, on
cliquait « Exécuter », la pastille passait au vert sans qu'on ait écrit
une ligne. Chacune a maintenant un trou, un indice, une solution, et un
chemin dans les deux sens.

**Les exercices demandent d'écrire**
- Chaque étape a son **trou** — « ; À FAIRE : … » —, et une **solution**
  rangée à côté dans la série. Un invariant exécute les deux : le sujet
  s'assemble, rend la main et n'atteint pas le but ; la solution
  l'atteint, dans le budget annoncé.
- Le curseur se pose sur le trou à l'ouverture, la bande dit ce qu'il
  reste à écrire et **l'efface à mesure** qu'on le remplace.
- La solution se demande sans rien effacer ni coûter de score.
- **Trois buts étaient faux**, et un quatrième était vrai avant le
  premier pas. Corrigés.
- Au mode débutant, le but se lit en toutes lettres : « RAX vaut 15 »
  et non « rax == 0xf ».

**Un indice, entre la consigne et la solution**
- L'aide était tout ou rien : la consigne, puis le programme entier —
  qui met fin à l'exercice. L'indice est le palier qui manquait.
- Il se **calcule** pour 80 étapes — les instructions que la solution
  emploie et que le tampon n'a pas —, **s'affine** à mesure qu'on
  écrit, et **mène aux fiches** : chaque instruction nommée ouvre la
  sienne.
- Il ne se montre pas tout seul : un indice qu'on n'a pas réclamé est
  une réponse soufflée.

**Le parcours va dans les deux sens, et garde ce qu'on y a écrit**
- Un chevron `‹` dans la bande, « Étape précédente » en toutes lettres
  dans la fenêtre. Il fallait auparavant rouvrir la liste et y retrouver
  son rang.
- **La précédente tout court**, et non « la précédente non réussie » :
  on avance vers ce qu'on n'a pas fait, on recule vers ce qu'on vient de
  faire.
- **Revenir à une étape rend le programme qu'on y avait laissé**, même
  si son onglet a été fermé : un brouillon par étape, gardé le temps de
  la séance et relu au moment où l'on redemande l'étape.

**Le mode simple est une disposition, et non un mode avancé amputé**
- L'**Établi** est la disposition du débutant ; la **Scène** — un ruban
  d'instructions, trois registres en gros chiffres, la fiche de
  l'instruction qui vient — s'ouvre en fenêtre, aux deux niveaux.
- Ce qui ne tient pas dans une bande **se détache en fenêtre** :
  « Ce que fait la machine » et « Résultat » ont leur bouton.
- Les trois pastilles de fenêtre, l'engrenage des préférences, la
  calculatrice et une console visible sont revenus : la barre de menus
  retirée avait emporté ce qu'elle portait.
- **La ligne où est le curseur s'explique**, tant que rien ne tourne :
  le mot, ce qu'il veut dire, la fiche entière à un clic. Une seule
  bande explique à la fois — pendant un exercice, c'est la sienne.

**Le titre dit ce qu'on lit**
- Cent vingt lignes de désassemblage s'affichaient sous « namsdec sans
  titre » : un binaire ouvert n'a pas de fichier d'enregistrement, et
  les quatre endroits qui nomment le tampon lisaient ce champ-là seul.
- Le survol donne le **chemin complet**, partout où le nom s'écrit, et
  le titre du système porte le nom — l'alt-tab ne montrait que
  « Namsdec ».

**La séance précédente se redemande**
- namsdec rouvrait d'autorité le fichier et le binaire de la veille. Il
  pose maintenant la question au lancement — **Reprendre** ou
  **Repartir à neuf** —, avec une case « Ne plus demander ».
- Le réglage se reprend aux Préférences, page Général : *Demander*,
  *Rouvrir*, *Ne pas rouvrir*.

**Trois gestes qui ne se défaisaient pas**
- « Montrer » donnait la réponse sans retour possible, le compteur de
  problèmes ouvrait la console sans la refermer, et un trait soulignait
  des notes qui ne mènent nulle part.
- La cible d'un exercice se lit **avant** le premier essai, en gris et
  sans verdict : on écrivait sans savoir où l'on allait.
- Le nom du fichier s'abrège par le milieu dans la barre du haut, en
  gardant son extension, et la rangée où l'on répond au programme n'est
  plus poussée sous le bord de la fenêtre.

**Le mode débutant, revu sur ses propres captures** (16 novembre)
- Le volet de la machine **écrit les nombres en clair** : un exercice
  demandait « Que vaut RAX à l'arrêt ? » — réponse 55 — pendant que le
  volet affichait `0000000000000037`. Les adresses gardent
  l'hexadécimal, et la légende le dit.
- **R8 à R15 ne prennent plus la moitié du volet** quand ils sont vides :
  ils paraissent dès que le programme en emploie un, et une ligne les
  rend à la demande.
- Un pas de plus après la fin du programme **n'invente plus
  d'instruction** : le bloc « Dernier pas » annonçait `add [rax], al`,
  lu dans les octets nuls qui suivent le `hlt`.
- Le but d'un exercice **sépare ce qu'on veut de ce qu'on a** :
  « RAX vaut 2 — il vaut 1 », là où les deux phrases se lisaient comme
  une seule.
- La liste des problèmes **dit quand elle continue plus bas**, au lieu
  de couper sa dernière phrase en deux.
- Le **parcours est allumé** pour une installation neuve, et son bouton
  ne se grise plus : un clic allume le module et ouvre la fenêtre.
- `mov rzx, 1` reçoit enfin son diagnostic : **il n'y a pas de registre
  `rzx`**, et les quatre qui en sont proches sont nommés — sans qu'aucun
  soit choisi d'office.
- Les **corrections proposées se montrent sous l'erreur**, avec un
  bouton : elles n'avaient de porte que par Alt+Entrée et la barre
  d'état, que le mode simple n'a pas.
- La cible d'un saut porte **le nom que la source lui donne** :
  « jne boucle » au lieu de `jne short 0000000000400021h`.
- La bande dit **où l'on en est** : le compte des pas depuis le début,
  et le numéro de passage par la ligne courante dès le deuxième tour.

**Les exercices répondent** (16 novembre)
- Un bouton **Vérifier** assemble, exécute et conclut — « Réussi » ou
  « Pas encore » : il fallait trois gestes et savoir qu'il les fallait.
- Le verdict ne paraît **que si on l'a demandé** : on exécute aussi pour
  regarder tourner, et un jugement qu'on n'a pas demandé en est un.
- Une **étape de lecture ne se vérifie pas avant d'avoir répondu** :
  l'exécuter donnerait la réponse. Le champ de réponse n'est plus
  minuscule dans la bande.
- **Le carnet des notions** : ce que les étapes réussies ont fait écrire
  ou lire, chaque instruction ouvrant sa fiche. « 12/117 » disait le
  chemin parcouru et rien de ce qu'on y avait appris.
- **Une seule règle de réussite** entre la pastille et le verdict : deux
  écritures auraient fini par donner un bouton qui félicite et une
  pastille qui refuse.
- Le parcours ne demande plus **aucune notion qu'il n'a ni montrée ni
  annoncée** — mesuré sur les 117 étapes, une seule manquait à l'appel
  (`rep`), et un contrôle le garantit désormais.

**Le détail de l'instruction tient sur une ligne** (16 novembre)
- Les octets, la fiche et les opérandes résolus s'empilaient sur trois
  lignes pour un quart de la largeur, dans une bande qui fait celle de
  la fenêtre — et le titre du repli finissait rogné. Ils tiennent sur
  une rangée dès que la place le permet ; le panneau latéral, étroit,
  garde ses lignes et sa grille.
- Sur un petit écran, le repli **reste fermé** plutôt que de peindre
  par-dessus le code, et le chevron dit pourquoi.

**Sept retours d'écran, en une passe** (16 novembre)
- **Poser un registre à la main marche enfin** : le champ reprenait le
  focus à chaque image, si bien qu'Entrée ne validait jamais. Et au mode
  débutant il s'ouvre désormais en décimal, comme le volet l'écrit —
  taper « 55 » posait 0x55.
- **L'indice se cache** : le bouton bascule au lieu de disparaître.
- **Recommencer** remet une étape dans l'état où on l'a trouvée, sans
  retirer la réussite ni le meilleur score.
- **Tout remettre à zéro** efface réussites, scores et brouillons du
  parcours — avec une question avant.
- **Le nom du fichier copie son chemin** d'un clic.
- **L'énoncé mène aux fiches** : les instructions qu'il cite s'ouvrent
  d'un clic, et chaque fiche renvoie au manuel Intel en ligne.
- **Les points d'arrêt disent ce qu'ils valent** : les registres de leur
  ligne, et le nombre de passages.
- **« Jusqu'au curseur »** arrive au mode simple, sur la frise.

## 0.3.0-beta.1 — 4 et 5 novembre 2026

Ce qui justifie le chiffre du milieu, du point de vue de qui s'en sert :
**l'établi a changé de forme**. Des barres d'icônes montrent et cachent
les panneaux sur les quatre bords, un accueil remplace l'éditeur quand
on a tout fermé, le compte des problèmes se lit sans ouvrir la liste, et
l'analyse se lance **sans fenêtre du tout** — `namsdec-cli` compare deux
versions d'un binaire rubrique par rubrique. Le parcours, lui, demande
maintenant de **prévoir** avant d'avancer.

**Plus de saut sous le curseur**
- Au survol d'un bouton des Préférences, la page descendait de
  vingt-quatre pixels : le contenu tient à trois pixels près, la barre
  de défilement paraissait, une rangée basculait à la ligne.
- Un bouton ne bouge plus d'un pixel quand le pointeur arrive.

**L'éditeur du débutant n'a pas de modes**
- On clique dans le programme, on tape, ça s'écrit. L'application
  s'ouvrait en mode NORMAL, où le champ de texte n'est pas interactif :
  il fallait savoir appuyer sur `i` d'abord.
- La **pastille de mode** disparaît avec eux : « NORMAL » nommerait une
  chose qui n'existe plus.
- **Au niveau avancé, rien ne change** — les modes sont une habitude de
  vim, et une bonne pour qui l'a déjà.


**L'accueil tient dans l'écran**
- Le blanc du haut se mesure sur la place disponible, et ce qui ne tient
  pas encore se rejoint en descendant. À 800×520, les récents tombaient
  sous le bord sans rien pour y aller.

**Les rangées d'onglets tiennent sur une ligne**
- Ce qui ne rentre pas passe derrière un bouton qui dit combien (`+5`),
  et **la vue qu'on regarde reste toujours visible**.
- À 800×520, les dix vues prenaient trois lignes — quatre-vingt-seize
  pixels — et il restait quatre lignes de programme à l'écran.

**Trois questions de lecture de plus** (cent dix-sept étapes)
- **L'appel système** : ce que `write` rend dans RAX — et le numéro
  qu'on y avait mis n'y est plus.
- **La convention d'appel** : RDI, RSI, RDX.
- **Le canari**, rangé à côté du canari qu'on écrit.

**La console a ses trois commandes**
- **Relancer**, **arrêter**, **effacer** au bout de sa rangée d'onglets,
  à côté de ce qu'elles changent. Éteinte, chacune dit pourquoi.
- **Effacer n'est pas réinitialiser** : l'écran se vide, le processeur
  ne bouge pas d'une instruction.

**Des barres d'icônes aux bords**
- À gauche l'explorateur et les exercices, en bas à gauche la console, à
  droite le panneau du processeur — chaque icône du côté et à la hauteur
  où son panneau s'ouvre.
- Survolée, elle dit **ce qu'un clic fera** : « Afficher l'explorateur »,
  « Cacher l'explorateur », avec le raccourci tel qu'il est réglé.
- Chaque panneau porte un **tiret** au bout de sa rangée d'onglets, qui
  le range.
- **Affichage › Barres d'icônes** les éteint ; le réglage est gardé.
- **L'icône de l'explorateur le referme** : l'appui le fermait et le
  relâchement le rouvrait aussitôt.

**Le compte des problèmes, sans ouvrir la liste**
- Au bord droit de la rangée des vues : un disque rouge et le nombre
  d'erreurs, un triangle orange et le nombre d'avertissements.
- Un clic ouvre la liste sur le bon onglet ; les deux **chevrons**
  mènent au problème d'avant et à celui d'après, en bouclant.
- Il ne paraît que s'il y a quelque chose à compter.

**L'accueil, quand on a fermé le dernier fichier**
- Le nom de l'atelier, puis ce par quoi on recommence — nouveau fichier,
  ouvrir, palette de commandes, exercices — et les **récents**, qu'un
  clic rouvre.
- Chaque raccourci montré est **celui du tableau des raccourcis**, lu et
  non recopié.
- L'accueil se **demande**, par un geste : fermer. Il ne paraît jamais
  parce qu'on a effacé son texte.

**Le choix du moteur de désassemblage**
- La rangée **« Moteur : »** est alignée : l'étiquette se posait cinq
  pixels au-dessus de ses choix.
- **En mode débutant, trois moteurs** au lieu de six : le pseudo-code,
  NASM et `objdump` en syntaxe Intel — celui qu'on a sur toute machine,
  et dont la syntaxe est celle qu'on vient d'apprendre en écrivant du
  nasm. Un moteur choisi au niveau avancé **garde sa place** si l'on
  revient au niveau débutant : on ne retire pas ce qu'on est en train de
  lire.

**La console rend sa place à la sortie**
- Les **arguments** et les **variables d'environnement** quittent la
  console : ils ne se tapent pas pendant qu'un programme tourne — ils
  sont posés avant la première instruction. Ils prenaient deux rangées
  sur quatre, et la sortie n'avait plus qu'une demi-ligne. Le bouton
  **« Arguments… »**, à droite de la rangée d'onglets, les rouvre quand
  on en a besoin, et le réglage est gardé.
- La ligne **« Entrée : »**, elle, ne bouge pas : on retire ce qu'on
  règle, pas ce qu'on tape.

**« Restaurer » restaure vraiment**
- Une fenêtre agrandie puis restaurée retrouve **sa taille et sa place**.
  Elle restait pleine page : le bouton basculait bien, mais personne ne
  gardait la géométrie d'avant — et ce que namsdec impose à une fenêtre
  agrandie reste dans la mémoire d'egui. Les sept fenêtres
  agrandissables étaient dans ce cas.

**L'écran du premier lancement, allégé**
- **La console est fermée au départ** sur une installation neuve : avant
  d'avoir assemblé, elle occupait soixante-douze pixels pour annoncer
  qu'elle est vide. Elle **paraît d'elle-même** dès qu'elle a quelque
  chose à montrer — une erreur de nasm, ou le premier octet que le
  programme écrit. Refermée en pleine course, elle le reste. Votre
  réglage, lui, ne change pas.
- **Et la phrase de la console n'est plus coupée** : elle se peignait
  tranchée par le milieu — neuf pixels sur quinze — juste au-dessus de
  la barre d'état, parce que sa hauteur comptait onze pixels de moins
  que la rangée d'onglets n'en occupe. Vu sur une capture, pas par une
  mesure ; un invariant le retient désormais.

**Douze instructions de plus savent s'expliquer**
- Un contrôle lit les binaires du système et exige une fiche pour chaque
  instruction rencontrée. Douze manquaient : **cinq fiches neuves** — le
  `cmov` du coprocesseur, l'environnement du x87, tout l'état flottant
  d'un bloc (`fxsave`), le mot de contrôle de SSE (`stmxcsr`, et ce que
  `-ffast-math` y pose), la mémoire transactionnelle (`xtest`) — et
  **deux élargies** : la pile fantôme gagne le changement de pile,
  « Comparer dans un masque » gagne les quarante-quatre alias d'AVX-512
  qu'elle décrivait déjà sans les nommer. Cent quatre-vingt-neuf fiches
  en tout.
- Chaque alias a été passé à nasm un par un : on ne documente que ce qui
  s'écrit, puisque les fiches nourrissent aussi l'autocomplétion. Et ce
  que les fiches affirment a été relu par `objdump`.
- **Cinq de ces fiches disent ce que namsdec ne simule pas**, et c'est
  souvent leur information la plus utile : « une transaction suppose
  plusieurs cœurs qui se marchent dessus, et namsdec n'en a qu'un ».
  Une fiche qui décrirait une instruction que l'interpréteur refuse sans
  le dire promettrait une exécution qui n'aura pas lieu.

**Six exercices qui demandent de lire, et non d'écrire**
- Les cent huit exercices faisaient tous **écrire** du code. Six de plus
  donnent un programme **complet** et posent une question — « que vaut
  RAX à l'arrêt ? » : on lit, on répond, puis on exécute, et la machine
  tranche. C'est « prévoir avant d'avancer » à l'échelle d'un programme
  entier. Le parcours compte **cent quatorze** étapes.
- **La réponse s'écrit comme on la pense** : « 32 », `0x20`, `20h` et
  `0b100000` sont la même réponse, et `-1` vaut `0xffffffffffffffff`.
  Refuser trois écritures sur quatre n'enseignerait que notre notation.
- **Le verdict ne donne pas la bonne réponse** — « pas celle-là », et
  rien de plus : elle s'obtient en exécutant le programme, c'est-à-dire
  en faisant ce que l'exercice enseigne. Et un champ vide ne vaut pas
  « faux ».
- **Et la réponse écrite dans la série est celle que la machine donne** :
  un contrôle assemble chacun de ces programmes, l'exécute jusqu'à
  l'arrêt et vérifie. Une série que vous écrivez porte ces exercices de
  la même façon — champs `question` et `answer`.

**Prévoir avant d'avancer**
- Devant une instruction pas encore exécutée, namsdec disait déjà tout :
  ce qu'elle fait en clair, ses opérandes résolus, et si un saut sera
  pris. On appuyait sur « Pas à pas » pour vérifier une réponse qu'on
  venait de lire — et l'on n'avait jamais l'occasion de se tromper,
  alors que se tromper est la seule façon de savoir qu'on n'avait pas
  compris.
- **En mode débutant**, l'adresse et l'instruction restent à l'écran,
  la réponse attend le pas, et **« Dernier pas » la révèle à côté de ce
  qui a vraiment changé** : « Ce que ça disait : EAX = 1 », puis
  « RAX 0x0 -> 0x1 ».
- **« Montrer »** donne la réponse sans attendre, d'un clic — demander
  une aide ne doit pas coûter un aveu — et seulement pour
  l'instruction qu'on regardait : au pas suivant, la question se
  repose.
- **Rien n'est imposé** : le réglage (Préférences › Modules, sous le
  mode débutant) ne vaut qu'avec lui, et se grise en le disant. Rien
  n'est calculé de neuf non plus — ce que namsdec disait déjà change
  seulement de moment.

**`namsdec-cli` : l'analyse sans ouvrir de fenêtre**
- Une **seconde commande**, posée dans le PATH à côté de l'application :
  `namsdec-cli programme` écrit tout ce que les panneaux d'analyse
  montrent, en Markdown ou en JSON, sans interface. C'est le même fond
  par le même chemin — l'analyse est sortie de l'écran, et rien n'y est
  peint.
- **Ce qu'elle rend possible, et qui est sa raison d'être : comparer
  deux versions d'un binaire.** `namsdec-cli prog.v2 --contre prog.v1`
  dit ce qui a changé dans l'**analyse** — une protection qui disparaît,
  une chaîne qui apparaît —, rubrique par rubrique, là où un `diff`
  d'octets répond « le fichier a changé à l'offset 0x15604 ». Il
  **compte aussi ce qui n'a pas changé** : « quinze rubriques disent la
  même chose » est ce qui permet de croire aux quatre qui ont bougé. Une
  ligne qui a seulement changé de place n'est pas un changement.
- En intégration continue, `-s protections` pose la seule question qui
  compte — les empreintes changent à chaque compilation — et **le code
  de retour porte la réponse** : `4` quand les deux analyses diffèrent,
  comme `diff` rend `1`. Le rapport est reproductible : deux appels sur
  le même fichier rendent le même texte.
- **Faite pour une chaîne d'intégration** : quatre codes de retour, dont
  un qui dit « il se lit, mais il n'y a rien à analyser ». Un fichier
  vide rendait dix-huit rubriques annonçant toutes « 0 octet », avec un
  titre et des tableaux : ce rapport **avait l'air d'une analyse**, et un
  artefact tronqué passait donc pour analysé.
- `--rubriques` liste ce dont un fichier a de quoi parler, `-s` n'en
  écrit qu'une, `--aide` dit le reste — en français ou en anglais.
- **Ce qu'elle ne fait pas, et c'est voulu** : elle repart des octets,
  donc elle ne porte rien d'une session (pas de nom donné à une
  fonction, pas de détour calculé), et elle ne lance aucun outil
  extérieur — objdump, radare2, Ghidra et RetDec restent muets, pour
  qu'un rapport ne parle jamais de la machine qui l'a produit.
- `./install.sh` la pose, `--no-cli` s'en passe, `./uninstall.sh` la
  retire.

## 0.2.9-beta.3 — 3 novembre 2026

**Une interface qui se gradue : débutant, ou avancée**
- namsdec sert à apprendre l'assembleur et la rétro-ingénierie, et
  vingt-cinq rubriques d'analyse devant quelqu'un qui ouvre son premier
  binaire lui demandent de savoir ce qu'il cherche avant d'avoir appris
  à chercher. Une case des **Préférences › Modules** décide : en **mode
  débutant**, l'interface ne montre que ce qui se comprend sans rien
  savoir d'avance.
- **Ce que cela change à l'écran** : six commandes dans la barre au lieu
  de sept, cinq onglets au centre au lieu de neuf, quatre dans le
  panneau au lieu de six, neuf rubriques d'analyse au lieu de
  vingt-cinq. Les menus suivent ; la palette de commandes, elle, garde
  tout — c'est une recherche, pas une vitrine.
- **Les deux pas s'appellent par ce qu'ils font** : « Pas à pas » et
  « Pas en arrière » plutôt que « Step Into » et « Step Back ». En mode
  avancé, les mots de gdb et de pwndbg reviennent — ce sont ceux des
  raccourcis.
- **Rien n'est désinstallé.** Les raccourcis marchent toujours, les
  panneaux cachés continuent de travailler, et **la vue où l'on vous
  envoie garde son onglet** quel que soit le niveau : on ne vous retire
  pas ce que vous avez demandé.
- **Une installation neuve s'ouvre en mode débutant** ; une
  configuration qui existait avant garde l'interface entière. On hérite
  d'un défaut, on ne se fait pas retirer ce dont on se servait.

**Ce que namsdec comprend d'un binaire sort encore un peu plus de
l'écran**
- Trois panneaux de plus décrivent ce qu'ils montrent au lieu de le
  dessiner : l'**entropie**, le **graphe d'appels**, et l'**inspecteur**
  de l'éditeur binaire. Onze des douze panneaux visés sont passés.
- La grille d'octets, elle, **restera peinte sur place**, et c'est une
  décision : une grille avec un curseur et vingt-six touches est un
  instrument, pas une compréhension.

**Ce que les mesures ont trouvé en chemin**
- **Une fonction de 18 446 744 073 697 542 199 octets** dans le graphe
  d'appels. Un `call` vers une adresse quelconque — ce que produit la
  moindre figure d'anti-désassemblage — fabriquait une « fonction »
  là-bas, et celle d'avant allait jusqu'à elle.
- Le **profil d'entropie** se graduait cinquante-six octets au-delà de
  la fin du fichier : la dernière tranche est plus courte que les
  autres, et la taille en était déduite.
- Les **noms de fonctions importées** étaient redevenus des boutons
  encadrés ; ce sont de nouveau des liens. Une **signature C** et un
  **nom de fonction** du panneau de décompilation se peignaient en
  chasse proportionnelle.
- **Une cellule de tableau qui ne tient pas s'arrête maintenant sur
  « … »**, et le survol rend ce qui manque — elle courait hors du
  panneau. C'est la colonne la plus large qui cède, jamais les
  étroites.
- « Fonctions : **1 (cibles de `call`)** » — l'accord en nombre manquait
  sur les quatre comptes de « En bref », qu'un programme d'une ligne met
  tous à un.

**Le détail d'une fonction ne se compose plus pour rien**
- Dans le graphe d'appels, une seule fiche s'ouvre à la fois. Les
  quarante coûtaient 2,6 ms par image — sur un budget de 16,7 — pour du
  texte que personne ne regarde, puisqu'elles sont fermées au départ.

## 0.2.9-beta.2 — 28 octobre 2026

**Ce que namsdec comprend d'un binaire ne vit plus seulement à l'écran**
- Les panneaux d'analyse **décrivent** ce qu'ils montrent au lieu de le
  dessiner directement : neuf des douze sont passés par cette frontière.
  Rien ne change à l'écran — c'est vérifié capture par capture, au pixel
  — mais ce qu'un panneau montre peut désormais être lu, exporté ou
  repris par autre chose que la fenêtre.
- Au passage, trois incohérences corrigées : l'espacement entre blocs
  était décidé panneau par panneau (4 px ici, 2 px là), les couleurs de
  verdict étaient écrites en dur au lieu de suivre le thème, et les
  colonnes d'un tableau n'avaient pas le même air d'un panneau à
  l'autre.
- Dans **UPX**, les lignes de sortie et les deux commandes se copient
  maintenant d'un clic : c'est la ligne d'origine qu'on recopie dans une
  recherche quand upx refuse.

**Arrêter une installation l'arrête vraiment**
- Le bouton **Arrêter** rend la main en une demi-seconde, et **pendant
  l'étape en cours** — pas à la fin de la compilation dont on voulait
  précisément sortir. Mesuré avant : l'enfant mourait en 300 ms, la main
  revenait 30 secondes plus tard, le temps qu'un programme lancé par la
  commande veuille bien fermer sa sortie.
- La commande **emporte sa descendance** : ce qu'elle avait elle-même
  lancé s'arrête avec elle. Ce qui échappe est dit plutôt que promis —
  un descendant détaché par `setsid` ne reçoit pas le signal.
- Même sans rien demander : une étape finie n'est plus annoncée « en
  cours » pendant que l'un de ses descendants vit sa vie.
- Le journal est **borné dans les deux sens**. Il l'était en nombre de
  lignes ; il ne l'était pas en taille de ligne, et une sortie sans
  aucun retour à la ligne avalait 50 Mo en trois secondes.
- **Trente minutes par étape**, et non les deux de la décompilation :
  une compilation d'outil peut légitimement durer un quart d'heure.

**L'assemblage ne gèle plus la fenêtre**
- `nasm` et `ld` travaillent **dans un fil à part** : la fenêtre continue
  de répondre, la barre d'état dit depuis combien de temps ça dure, et
  **Stop arrête l'assembleur** — ce bouton était grisé pendant le seul
  travail dont on veuille vraiment sortir.
- **Dix secondes au plus**, après quoi l'outil est arrêté et namsdec le
  dit. Une seule ligne valide suffit à sortir de l'échelle ordinaire :
  `times 2000000000 nop` occupe nasm bien au-delà de vingt secondes,
  pendant lesquelles il n'y avait plus ni image ni clavier. Le chiffre
  vient d'une mesure : sept lignes s'assemblent en 4 ms, vingt-deux
  mille en 52 ms.
- **Un résultat tardif ne remplace plus ce qui est à l'écran.** Si le
  texte a changé — ou si l'on a changé d'onglet — pendant que nasm
  travaillait, le listing qui arrive décrit un autre programme : il est
  jeté, au lieu de faire pointer la gouttière et les points d'arrêt sur
  des lignes qui ont bougé.
- Le listing de nasm est lu **borné** : `%rep 400000` en produit
  quarante méga-octets, lus jusqu'ici en entier.
- Même traitement pour les autres appels extérieurs : l'assemblage d'une
  instruction seule, les deux sondes `--version` du démarrage, et `upx`,
  qui avait son propre guetteur — un délai tenu pour le processus mais
  pas pour ses descendants.

**Le coprocesseur obéit enfin à son mot de contrôle**
- `fldcw` **compte** : l'arrondi réglé gouverne `fist`, `fistp` et
  `frndint`. `fistp` de 3,5 rend 4 au plus proche, 3 par troncature, 4
  vers le haut et 3 vers le bas — et `fistp` de -3,5 vers le bas rend
  -4, parce que « vers le bas » n'est pas « vers zéro ». `fisttp`
  tranche quoi qu'on règle, c'est sa raison d'être.
- `fnstcw` rend **ce qu'on y a écrit** et non plus une constante. Le
  prologue de beaucoup de routines de conversion fait exactement cette
  danse : garder l'ancien mot, tronquer, remettre. `finit` remet 0x037f.
- Le **contrôle de précision** est simulé : à 24 bits, chaque opération
  de base s'arrondit au format simple (racine de 2 rend
  1,4142135381698608, comme le matériel) ; à 53 bits, namsdec **est** le
  matériel, puisque c'est la double précision qu'il calcule.
- Et ce qui reste approximé **se dit**, au moment où le programme le
  touche : la barre d'état le nomme, l'onglet Diagnostics le garde avec
  ce que ça change, et la fiche **F1 « arrondi »** porte le recensement
  des sept points. Le principal : la précision étendue — le réglage par
  défaut — est approximée par la double, et (1+1e-18)-1 rend 9,76e-19
  sur le matériel contre zéro ici. Posez `fldcw` à 0x027f et les deux
  redeviennent identiques.

**Ce qu'un programme demande au noyau**
- `lseek` répond au lieu d'arrêter le programme : il n'y a ni fichier ni
  position derrière l'entrée et la sortie de namsdec, et `-ESPIPE` est
  ce que rend un vrai noyau pour un tube ou un terminal. Un binaire C++
  ordinaire mourait dessus après avoir écrit sa première ligne.
- **`open`, `openat`, `stat`, `lstat`, `access` répondent « introuvable »**
  au lieu de s'arrêter, comme `fopen` le fait depuis longtemps : le
  programme suit alors sa branche d'erreur, qui est souvent la moitié
  intéressante. La console le dit une fois. Ouvrir pour créer rend
  `-EROFS` — ce qui manque alors n'est pas le fichier mais l'endroit où
  l'écrire. La même question recevait jusqu'ici deux réponses opposées
  selon que le binaire était lié dynamiquement ou statiquement.
- `newfstatat` lit enfin son chemin : il décrivait la sortie standard à
  un programme qui demandait `/etc/passwd`.
- Un appel refusé se **nomme** : « l'appel système 8 (lseek) » plutôt
  que le seul numéro. Et l'aide qui l'accompagne n'énumère plus quatre
  appels sur quarante-huit — elle dit la ligne de partage, qui ne se
  périme pas.

**Le point d'arrêt qu'un programme se pose à lui-même**
- `int3` va au gestionnaire de `SIGTRAP` que le programme a installé,
  avec un **vrai cadre de signal** : un gestionnaire qui réécrit
  l'adresse de reprise pour sauter par-dessus un leurre est obéi. C'est
  la dernière pièce du montage anti-débogage, après `ptrace`, `clone` et
  `tgkill`. Deux binaires du corpus d'épreuve passent de l'arrêt sec à
  leur écran d'accueil.
- Sans gestionnaire installé, l'exécution s'arrête en le disant : sur
  une vraie machine, ce signal tuerait le programme.

**L'éditeur binaire**
- Le vidage **prend la place qu'on lui donne** — en hauteur comme en
  largeur. Le nombre de lignes se calcule sur la hauteur offerte ; le
  nombre d'octets par ligne suit la largeur (`auto`, et 8/16/32 pour
  figer). Dans l'onglet d'analyse, la grille passe de 606 à 1072 pixels.
- L'inspecteur se pose **à côté** du vidage dès qu'il y a la largeur, au
  lieu de lui prendre quatre cents pixels de hauteur et de se faire
  couper par le bas.
- Un flottant illisible se dit en notation scientifique : `float64`
  s'étalait sur trois lignes de zéros.
- Les deux flèches de la recherche ne pointent plus l'une vers l'autre.

**La console**
- La phrase sous la rangée « Entrée : » ne se fait plus couper, et le
  fond de la fenêtre de sortie ne recouvre plus les rangées
  « Arguments » et « Variables d'environnement ».

**Les textes**
- Dix-neuf phrases portaient le retrait de leur source — jusqu'à
  vingt-six espaces au milieu d'un mot —, et s'affichaient en deux blocs
  qu'on lisait dans le désordre.

**Le parcours**
- Le programme d'accueil annonçait « quatre-vingt-seize exercices »
  alors qu'il y en a cent huit — la première phrase qu'on lit en
  ouvrant namsdec.
- **Douze exercices de rétro-ingénierie**, répartis sur trois paliers :
  la signature d'un fichier, ce qu'un `strings` laisse voir, la
  comparaison et le saut qui en dépend, un aiguillage vu du
  désassemblage, les chaînes cachées par XOR, la garde `ptrace`, `int3`
  et son gestionnaire, l'entropie d'une zone, un gadget ROP, et l'octet
  qui change tout. Le parcours compte **cent huit étapes**, dont cent
  une avec un but vérifiable.

## 0.2.9-beta.1 — 15 octobre 2026

**Le masque s'applique**
- `vmovdqu32 ymm0{k1}, [rsi]` et `vpaddd ymm3{k1}, …` s'exécutent : les
  voies retenues reçoivent le résultat, les autres gardent leur valeur
  (fusion) ou tombent à zéro (`{z}`). AVX-512 est complet pour ce que
  namsdec rencontre.
- Une instruction dont namsdec ne sait pas nommer l'élément s'arrête
  encore en le disant : se tromper d'élément décalerait les voies et
  rendrait un résultat faux sans avertissement.

**Les registres de 512 bits**
- Le fichier de registres vectoriels passe de seize à **trente-deux**
  registres, et de 256 bits à **512**. `xmm5`, `ymm5` et `zmm5` sont
  trois fenêtres sur les mêmes bits ; c'est l'instruction qui dit
  combien elle en lit. LeetCrackmeLin64 en compte trois cent
  cinquante-cinq, qui s'exécutent désormais.
- La règle du haut vaut jusqu'en 512 : une instruction AVX qui écrit
  `ymm1` **efface** les bits 256 à 511, une instruction SSE n'y touche
  pas. Les deux cohabitent dans le même programme.
- La fenêtre des registres étendus montre les seize nouveaux, et les
  bits hauts dans l'infobulle — seulement quand ils portent quelque
  chose.

**Les registres de masque d'AVX-512**
- `k0` à `k7` existent, et avec eux `kmov*` — l'instruction AVX-512 la
  plus fréquente d'un vrai binaire —, les combinaisons, `kortest*`, et
  ce qui les remplit : `vptestm*` et les `vpcmp*` avec leur prédicat.
  C'est la grande idée du jeu : un bit par voie, rangé à part, là où
  SSE écrivait un vecteur de zéros et de uns.
- La fenêtre des registres étendus les montre **en binaire** — ce qu'on
  y lit est « quelles voies » —, et seulement quand il y en a.

**Un premier morceau d'AVX-512**
- Les **alias EVEX** des instructions déjà exécutées — `vmovdqu64`,
  `vmovdqa64`, `vpxorq` et leurs semblables — s'exécutent sur xmm et
  ymm. Le chiffre final ne dit ni la taille du transfert ni celle du
  registre : il dit ce qu'est un élément pour le masque qui *peut* les
  accompagner. Sans masque, ce sont les mêmes instructions, et cela
  fait plus de 1 200 occurrences dans les binaires d'épreuve.
- **Une instruction masquée s'arrête et le dit**, plutôt que d'ignorer
  son masque : un résultat faux sans avertissement est pire qu'un arrêt.

**Les instructions de chaîne de SSE4.2 s'exécutent**
- `pcmpistri` et ses trois sœurs font en une instruction ce qu'une
  boucle fait en vingt, et c'est ainsi que la bibliothèque C écrit
  `strlen`, `strchr`, `strcmp` et `strstr`. C'était le chemin suivant
  après `pmaxub` : un binaire qui lit une chaîne passait par l'une,
  puis butait sur l'autre.
- Il ne reste plus une seule instruction manquante dans les neuf
  binaires d'épreuve, hors AVX-512.

**La sortie du programme se lit comme dans un terminal**
- **Les séquences d'échappement ne s'affichent plus en toutes lettres.**
  Un crackme sur deux ouvre par un logo en couleur, et l'on lisait
  « □[2J□[H□[36m » avant le dessin. namsdec les interprète : les
  couleurs se voient, le retour chariot recouvre sa ligne — c'est ainsi
  qu'une barre de progression s'anime —, l'effacement efface, et ce qui
  n'est pas reconnu disparaît au lieu de s'afficher.
- Les huit couleurs d'un terminal sont prises **dans le thème** : un
  logo s'accorde au reste de la fenêtre au lieu de jurer avec.

**La calculatrice compte en bits**
- **Vingt-neuf fonctions de plus**, quarante-sept en tout. Ce qui
  manquait tenait en une phrase : on ne pouvait ni lire un bit, ni
  extraire un champ, ni tourner sur une autre largeur que soixante-
  quatre. S'ajoutent donc `bit`, `setbit`, `clrbit`, `flipbit`,
  `bits(x, h, b)` — de quoi décoder un ModR/M —, `mask`, `byte`,
  `bitrev` et ses largeurs, `twos`, `ones`, `abs`, `parity` (celle du
  x86 : l'octet de bas, et 1 quand le compte est pair), `ffs`, `log2`,
  `ispow2`, `nextpow2`, les rotations sur 8, 16 et 32 bits, `sar` — le
  décalage arithmétique —, `smin`, `smax`, et `mulhi`, ce que `mul`
  laisse dans RDX.
- **Rangées en six familles** qu'on ouvre une à une : Bits, Ordre,
  Signe, Compter, Tourner, Calculer. À plat, les quarante-sept
  poussaient le bas de la fenêtre hors de l'écran. La famille ouverte
  revient au lancement suivant.

## 0.2.8-beta.3 — 14 octobre 2026

Trois passes depuis la beta.1 : les griefs relevés sur captures le 13
octobre, une passe de **mesure** sur les gros binaires — quatre gels,
tous dus au même défaut de forme, un travail proportionnel au listing
entier refait à chaque image pour en montrer trente lignes —, puis le
gros chantier qui en découlait : **ne plus mettre le listing en page
d'un bloc**.

**Ce que le programme écrit, en entier — et ce qui se clique**
- **La boîte de saisie montre toute la sortie, sans ascenseur.** Elle en
  montrait les quatre dernières lignes dans une fenêtre large de 480
  pixels fixes : un logo de sept lignes sur quatre-vingts colonnes y
  perdait ses trois premières et sa moitié droite, sans que rien ne dise
  que c'en était un fragment. Quand la sortie dépasse l'écran, c'est la
  fin qu'on voit — l'invite est toujours en bas — et une ligne dit ce
  qui reste au-dessus.
- **Un texte qui se clique le dit** : le pointeur devient une main et le
  texte se souligne dessous. Partout, du sommaire du manuel aux
  empreintes, aux adresses, aux lignes du désassemblage. Une rangée
  entière cliquable se souligne entière.

**Aller voir une adresse : dans le code, ou dans les octets**
- **Cliquer une adresse demande où aller.** Elle menait toujours au
  listing — la bonne réponse pour une fonction, la mauvaise pour une
  chaîne, une table ou un bourrage, qu'un désassembleur lit comme des
  instructions faute de savoir faire autrement. Deux entrées, et les
  deux chemins ne se ressemblent pas : aller au code ferme la fenêtre
  d'analyse, aller aux octets la garde ouverte.
- **L'éditeur binaire montre l'adresse à côté de la position**, accepte
  l'une ou l'autre dans « Aller à », et dit d'où l'on vient quand on y
  arrive depuis un autre panneau.
- **La fenêtre d'analyse retrouve son bouton agrandir** — la pastille
  verte à côté de la rouge —, et agrandie elle ne recouvre plus la barre
  de menus.

**Le calcul vectoriel entier s'exécute**
- **Un binaire ne s'arrête plus sur `pmaxub`.** On peut n'avoir jamais
  écrit une ligne de SSE et buter dessus au premier pas : la bibliothèque
  C ne cherche pas la fin d'une chaîne octet par octet, elle en compare
  seize d'un coup au vecteur nul. Soixante-dix mnémoniques de plus —
  minimum et maximum dans les deux signes et les trois largeurs,
  l'arithmétique qui **sature** au lieu de boucler, `pshufb`, `palignr`,
  l'emballage et l'élargissement, les produits et leurs deux moitiés, la
  moyenne, l'écart absolu, `ptest`, et les versions AVX de tout cela.
- Les `prefetch*` passent désormais sans rien faire, ce qui est fidèle :
  elles n'ont aucun effet observable sur la vraie machine non plus. On
  s'arrêtait dessus.
- **La fenêtre d'aide s'ajuste à ce qu'elle montre** : une fiche courte
  laissait cent trois pixels de vide au-dessus du bouton « OK ».
- **Sept fiches nouvelles**, et huit mises à jour : elles annonçaient
  « namsdec ne les simule pas » de ce qui s'exécute maintenant. Le manuel
  compte 171 fiches, et un contrôle l'empêche désormais de prendre du
  retard sur l'interpréteur.

**Le listing ne se met plus en page d'un bloc**
- **Ouvrir un gros binaire est devenu immédiat.** namsdec mettait tout le
  listing en page avant d'en peindre la moindre ligne : 2,9 secondes et
  **1,8 gigaoctet de mémoire** pour un million de lignes, alors que
  trente sont à l'écran. En mode NORMAL — celui où l'on lit —, seules les
  lignes visibles sont désormais posées. Première image **2 942 ms →
  116 ms**, mémoire **2 032 Mo → 268 Mo**, et une image en régime
  **11,3 ms → 1,9 ms**. Sur un binaire plus ordinaire (62 386 lignes) :
  343 ms → 13,5 ms.
- La saisie n'y perd rien : entrer en Insertion redonne le vrai champ de
  texte, avec la sélection, le presse-papier et le reste. C'est la
  frontière que l'éditeur portait déjà — le champ y était posé non
  interactif.
- **Cliquer dans le code y pose le curseur** : le champ de texte ne le
  permettait pas en mode Normal, et l'on voyait un curseur qu'aucun geste
  de souris n'atteignait.
- **Aller à une adresse ne fait plus basculer en saisie.** Cliquer une
  adresse dans un panneau d'analyse ouvrait le mode Insertion sur un
  listing de binaire : on venait lire, on se retrouvait à éditer.
- **La gouttière suit le fichier** : sa largeur était fixe, et la
  pastille d'un point d'arrêt recouvrait le numéro dès quatre chiffres —
  un listing d'un million de lignes en demande sept.

**Aller voir une adresse — et la passe du 13 octobre**
- **Cliquer une adresse emmène vraiment au listing.** Le canal marchait
  déjà, mais rien ne se voyait : poser le curseur sur une ligne ne
  déplace pas la vue, et la fenêtre d'analyse restait devant. Le listing
  **défile et centre** la ligne, la fenêtre s'efface (un clic la rouvre
  à la même rubrique), et la barre d'état dit où l'on arrive. C'est
  aussi ce qui rend la **barre de chaleur** atteignable : dans un
  binaire de soixante mille lignes, les lignes chaudes sont loin du
  haut, et l'onglet Profil n'y menait pas davantage.
- La fenêtre d'analyse n'a plus de bouton **agrandir/restaurer** :
  agrandie, elle recouvrait l'écran, et l'adresse cliquée emmenait le
  listing derrière un panneau opaque.
- **Les fonctions se renomment dans leur propre liste** : on clique le
  nom, on tape, Entrée. Et le nom donné paraît maintenant **dans le
  listing lui-même** — l'étiquette s'y écrit, les points d'arrêt suivent
  leur adresse plutôt que leur numéro de ligne, et les symboles
  d'origine restent intacts pour qu'on puisse se dédire.
- **Un panneau par outil extérieur** — Ghidra, RetDec, radare2, objdump,
  ndisasm — dans la fenêtre d'analyse, chacun gardant sa sortie : c'est
  ce qui permet de comparer ce que deux décompilateurs disent d'une même
  fonction. Rien ne s'exécute tout seul, la commande est écrite avant
  d'être lancée, et un outil absent dit comment l'obtenir.
- **Un outil qui écrit beaucoup ne se bloque plus.** `objdump -d` rend
  plusieurs méga-octets ; namsdec attendait sa fin avant de le lire,
  pendant qu'il attendait d'être lu — le tube plein. Il était déclaré
  « arrêté au bout de cent vingt secondes » alors qu'il avait fini en
  une fraction de seconde.
- **Fragments réutilisables (ROP)** : la liste s'arrêtait à quarante
  sans dire qu'il y avait une suite. Deux mille fragments, parcourus
  page par page, avec « 41–80 sur 1 274 ».
- **Entropie** : le découpage suit la taille du binaire (des tranches de
  256 octets à 64 Ko, quelques centaines toujours), le profil prend
  toute la largeur sans jamais dépasser la moitié de la fenêtre, une
  règle d'adresses court dessous, l'échelle de couleurs est graduée de 0
  à 8 avec neuf arrêts, et les **plages** donnent enfin les adresses —
  cliquables.
- **Les fonctions empruntées sont des liens** (demandé). Le survol
  donne la phrase, le clic déplie la fiche : la signature comme une
  page de manuel l'écrit, et le registre qui porte chaque argument.
  Quatre fonctions du support d'exécution entrent dans la table —
  `__libc_start_main`, `__cxa_finalize`, `__cxa_atexit`,
  `_Unwind_Resume` —, celles que tout binaire C++ ou Rust emprunte et
  que la liste laissait muettes. Et ce que namsdec ne connaît pas le
  dit, en donnant ce que le nom apprend : `__cxa_` est le support C++,
  `_Unwind_` le dérouleur de pile, `_ZN` un nom encodé.
- **La fenêtre d'analyse ne grandit plus avec son contenu.** Sur la
  rubrique « Chaînes lisibles », les chemins de cent caractères la
  poussaient à la largeur de l'écran — la colonne de navigation sortait
  par la gauche — et elle y restait : revenue sur « En bref », elle
  gardait sa taille d'écran. Ce qui dépasse se fait maintenant défiler,
  et la fenêtre est bornée à ce qu'elle peut montrer.
- **Le processeur est devenu un onglet** (demandé). Les registres
  occupaient une zone ancrée au milieu du panneau latéral ; le panneau
  se partageait donc en trois, et au zoom de travail les registres
  prenaient près des trois cinquièmes de la place en écrasant la vue
  d'à côté. « Processeur » est maintenant la première vue, et celle
  qu'on trouve en ouvrant — une zone à la fois, chacune avec toute la
  place. Le bandeau ne garde par ailleurs que la hauteur qu'il demande,
  au lieu des 280 px qu'il réservait du temps où il la partageait avec
  les registres.
- **Les registres prennent une colonne de plus quand le panneau est
  court** (demandé). Ce qui bloquait une colonne n'était pas la largeur
  des valeurs — seize chiffres, c'est irréductible — mais le blanc
  entre les colonnes : sur un panneau court il coûtait une colonne,
  c'est-à-dire deux rangées. On le rend là où cela gagne vraiment une
  colonne, et nulle part ailleurs.
- **L'onglet Octets suivait l'exécution de travers.** Sa liste annonce
  à egui des rangées d'une hauteur, et en pose d'une autre — l'espace
  entre deux lignes était compté deux fois. Le calcul qui centre
  l'instruction courante tombait donc d'autant plus loin que le
  programme est long.
- **Le panneau latéral ne se recouvre plus.** Dans l'onglet Mémoire,
  l'état du processeur s'écrivait par-dessus le haut de la vue —
  « Adresse », « Chercher », « Suivre » et « Lire en » sous les
  registres, illisible. La cause tient en une ligne d'egui : quand le
  contenu d'un panneau dépasse la place qu'on lui a laissée, le
  rectangle du panneau glisse vers le bas, et le panneau ne rend au
  parent que ce qui reste visible. Le contenu de la vue est désormais
  borné à son panneau — il défile au lieu de déborder —, et l'état du
  processeur garde sa place : c'est ce qu'on regarde entre deux pas.
- **La colonne de la fenêtre d'analyse est rangée par familles** — Le
  fichier, Le code, Décompiler, Ce qu'il dit, Ce qu'il emprunte, Les
  octets —, dans l'ordre où l'on lit un binaire inconnu. Vingt-quatre
  rubriques à plat se parcouraient une à une ; on sait maintenant où
  regarder avant d'avoir lu.
- **Des carrés vides à la place de caractères.** La coche, la croix, les
  triangles, les flèches : ces caractères n'existent pas dans les
  polices qu'egui embarque, et se peignaient en carré. Remplacés partout
  par des caractères que la police porte, et un contrôle regarde
  désormais ce qui est peint plutôt qu'une liste écrite d'avance.

**Les gros binaires cessent de geler namsdec**
- **Chercher dans un listing de binaire** ne gèle plus. Sur crackme-01
  — 62 386 lignes, 24 081 fois « mov » —, l'image prenait **3,7
  secondes** barre de recherche ouverte, contre 0,8 ms fermée ; sur un
  binaire de quatre mégaoctets, l'image ne venait plus du tout. Les
  occurrences sont désormais relevées **une fois**, portent leur ligne
  et leur colonne, et seules celles qui sont à l'écran se peignent.
- **La fenêtre Désassemblage** redécodait le binaire entier à chaque
  image — un million d'instructions pour quarante rangées visibles. Elle
  ne compose plus que les lignes qu'elle montre.
- **La gouttière** ne peignait déjà que les lignes visibles, mais elle
  écartait les autres une par une : 9 ms par image sur un listing d'un
  million de lignes, pour en retenir soixante. La tranche visible se
  trouve maintenant par dichotomie.
- Au total, sur un listing d'un million de lignes, une image en régime
  tombe à **5,6 ms** ; les morceaux retirés en coûtaient onze de plus,
  mesurés dans la même course. L'ouverture d'un gros binaire, elle,
  coûte toujours sa mise en page — c'est un autre chantier, et il est
  écrit.
- **Les compteurs de vim** : `5j` descend de cinq lignes, `3w` avance de
  trois mots. Le compteur en attente s'affiche à côté de la pastille de
  mode — un état qui change ce que la touche suivante fera doit se voir.
- **La ligne qu'on lisait revient** avec le binaire, sous la même garde
  que les points d'arrêt : même fichier, même date. Retrouver ses
  repères mais rouvrir à la ligne 1 laissait à refaire le plus long.

## 0.2.8-beta.1 — 7 octobre 2026

L'onglet RE prend sa forme : on y circule, on y nomme ce qu'on comprend,
et on en ressort une règle. La gestion des outils se resserre, l'éditeur
suit enfin l'exécution, et les indicateurs redeviennent ce qu'ils sont —
des témoins qu'on lit.

**L'onglet RE : circuler, nommer, ressortir quelque chose**
- Un **graphe d'appels** : l'arbre depuis le point d'entrée, et sous lui
  chaque fonction qui se déplie sur son **code binaire** et son
  **pseudo-code**, la même ligne lue deux fois — telle que la machine la
  reçoit, puis telle qu'on la comprend. Une fonction déjà dépliée devient
  un renvoi : l'arbre reste fini même sur du code récursif.
- **Renommer une fonction** : <code>sub_10450</code> devient
  « vérifie_clé », et « Défaut » lui rend son nom d'origine. Le nom donné
  **suit l'adresse partout** où elle paraît — liste des fonctions,
  références croisées, listing — avec un léger fond qui distingue ce
  qu'on a déjà nommé. Il est gardé d'une séance à l'autre.
- **Les adresses se cliquent** : d'un panneau au listing, sans relever
  l'adresse à la main pour la retaper dans « aller à ».
- **Signaux** — par où commencer devant un échantillon inconnu : ce que
  le binaire sait faire, rangé par familles, et les indicateurs qu'on
  peut aller vérifier ailleurs.
- **Place libre (bourrage)** — le remplissage que l'éditeur de liens
  laisse entre deux fonctions, nommé par sa section. En lecture seule.
- **Quand le listing ment** — les deux lectures du même octet : un
  désassemblage qui démarre à contretemps produit des instructions qui
  n'existent pas, et le panneau le signale.
- **« En bref » enrichi** : taille du code, point d'entrée, fonctions,
  chaînes, et ce que le fichier dit de lui-même.
- **Règles YARA** — une règle prête à coller dans un
  <code>.yar</code> pour retrouver les frères de l'échantillon.
- **Entropie en dégradé continu** — vert, jaune, orange, rouge, violet
  pour l'extrême : la nature d'un bloc se lit à sa couleur.

**L'aide**
- Le sommaire des instructions montre les **674 mnémoniques**, et non
  plus les 113 têtes de famille : <code>je</code>, <code>jne</code>,
  <code>jz</code> étaient dans la documentation sans être à l'écran.

**Les exercices**
- Le **but et la progression** quittent le bandeau du processeur, où ils
  restaient affichés même après le chargement d'un binaire sans rapport,
  pour la fenêtre dédiée aux exercices.

**Solidité**
- Une **panique sur ELF hostile** corrigée, trouvée par un relevé qui
  jette des en-têtes incohérents aux analyseurs.
- Dépendances à jour (correctifs compatibles).

**Les outils d'analyse**
- **DZF s'installe depuis namsdec** : la recette clone le dépôt, le
  construit en C++23 et pose ses binaires dans le dossier de namsdec —
  rien hors de chez lui.
- Un outil **bâti soi-même hors du PATH** — DZF compilé dans son dépôt —
  restait « absent » sans recours : namsdec ne cherche que dans le PATH et
  son dossier d'outils. La fiche d'un outil absent propose maintenant
  **Localiser le binaire…** : on le désigne une fois, il est retenu et
  employé tel quel.
- Un outil **incomplet** — rizin sans son greffon rz-ghidra — ne s'active
  plus : l'activer déclarait un décompilateur qui échoue au premier essai.
  « Activer » est grisé (l'infobulle dit pourquoi), et **Réparer**, en
  vif, mène la marche. On n'active qu'un outil prêt.
- L'onglet Outils **signale quand la décompilation est éteinte**, avec de
  quoi l'allumer sur place — les deux réglages d'une même fonction ne sont
  plus éparpillés sur deux onglets. Et l'onglet Modules sépare les
  composants (les crates, leur santé) des fonctions optionnelles.

**L'éditeur et les indicateurs**
- **Avancer d'un pas ramène le code sous les yeux.** Si l'on avait fait
  défiler vers la droite pour lire un long commentaire, le pas suivant
  gardait cette vue : la ligne courante restait visible en hauteur, mais
  les instructions étaient sorties à gauche. L'éditeur suit désormais
  l'exécution.
- Les **sept indicateurs** (CF, PF, AF, ZF, SF, DF, OF) de la barre de
  titre sont de **simples témoins** : on les lit, on ne les règle plus.
  Les cliquer brouillait ce qu'ils disent — l'état que la dernière
  instruction a produit.

## 0.2.6-beta.1 — 30 septembre 2026

Une instruction se récrit en assembleur, le désassemblage se lit dans la
syntaxe du système, dix-huit exercices de plus, le vocabulaire des outils
cesse de mélanger trois questions — et la fenêtre d'analyse répond enfin
à la première question qu'on se pose devant un échantillon inconnu : par
où commencer.

**Les raccourcis : les deux jeux côte à côte**
- Préférences › Raccourcis montrait **un** jeu, et c'est précisément ce
  qui empêchait de voir les trous et les doublons — on ne compare pas de
  mémoire deux listes de trente lignes. Les deux y sont désormais, action
  par action : l'actif en clair, l'autre à côté.
- Quand les deux disent la même chose — une frappe sur trois : `Ctrl+S`,
  `Ctrl+O`, `Maj+F5` —, la seconde colonne écrit **« la même »** au lieu
  de recopier. L'œil ne s'arrête alors que sur les écarts, qui sont la
  seule raison de regarder deux jeux à la fois.
- Un raccourci **déplacé soi-même** est marqué, et le survol dit ce que le
  jeu proposait. Deux actions qui répondraient à la **même frappe** se
  voient en rouge, et le survol nomme l'autre — les jeux livrés n'en ont
  aucune, mais un raccourci qu'on déplace peut en écraser un autre, et
  rien ne le disait.
- Les séquences s'écrivent **`Space P I`** et non `Space , P , I`. Les
  deux formes côte à côte sur trente lignes ont tranché.

**La boîte de préférences grandissait toute seule**
- Elle gagnait **un pixel par image, sans fin**, sur les quatre pages plus
  hautes qu'elle : la zone qui défile prenait sa hauteur sur la fenêtre, et
  la fenêtre prenait la sienne sur son contenu. Sa hauteur est désormais
  imposée, ce qui coupe la boucle sans faire sortir « OK » et « Quitter »
  par le bas.
- Le banc de mise en page n'ouvrait que la **première** page des
  préférences — la seule qui tienne sans défiler. Les cinq y sont, et une
  page nouvelle s'y trouvera mesurée sans qu'on y pense.
- « Aucun binaire assemblé. » servait à la fois d'état vide et d'état
  d'une fiche de module : deux rôles pour une phrase. La fiche dit
  maintenant « rien à désassembler pour l'instant ».

**Circuler : d'un panneau d'analyse au listing**
- **Les adresses des panneaux sont cliquables** — une entrée de liaison,
  une zone libre, une figure, un fragment, un désalignement : un clic
  amène le listing dessus. C'est la seule chose qu'on veuille faire d'un
  endroit qu'un panneau désigne, et il fallait jusqu'ici relever
  l'adresse à la main pour la retaper dans « aller à ».
- Le suivi de RIP s'arrête au passage — sans quoi le pas suivant
  ramènerait le listing là où l'exécution en est, et le clic paraîtrait
  n'avoir rien fait. Une adresse entre deux lignes mène à la ligne juste
  avant ; une adresse dans aucune partie désassemblée le dit.

**Le compte d'appels des imports valait zéro**
- Le panneau annonçait « 105 fonctions importées » et **zéro appel** vers
  chacune, sur tout binaire ouvert. Il convertissait l'adresse d'une
  entrée de liaison en position dans le fichier, alors que le listing d'un
  ELF ouvert est déjà en **adresses virtuelles** — namsdec n'y désassemble
  que les parties exécutables, chacune à son adresse.
- Mesuré sur `/bin/ls` : **719 appels au lieu de 0**, et le classement
  « ce qui sert partout d'abord », qui est toute la raison d'être de ce
  panneau, fonctionne enfin.
- Un compte faux se lit comme un compte vrai : c'est pourquoi le défaut a
  tenu si longtemps. Le contrôle qui le retient charge désormais le
  binaire **par l'application**, au lieu de reconstruire le listing à sa
  façon.

**Quand le listing ment**
- Un désassemblage lit **linéairement** ; le processeur, lui, va **où les
  sauts l'envoient**. Un seul octet-piège derrière un `jmp` qui l'enjambe,
  et toute la lecture ligne à ligne part de travers : c'est
  l'anti-désassemblage classique, et on le rencontre dans ce que namsdec
  est fait pour lire.
- Le panneau **Figures du code** donne désormais les **deux lectures côte
  à côte** — ce que le listing lit, ce que le processeur lit — et l'octet
  qui les sépare. Le listing n'est pas corrigé : refaire le désassemblage
  en suivant le flot laisserait des trous partout où l'on ne sait pas
  aller, et un listing troué serait pire que trompeur.
- Ce qu'on apprend au passage vaut plus que la figure : **le découpage en
  instructions n'est pas une propriété des octets**, c'est une propriété
  du point où l'on commence à lire.
- Le relevé ne regarde que les sections exécutables, et que les sauts qui
  tombent à moins de seize octets. Sans ces deux bornes, cinq figures
  étaient relevées sur `/bin/ls` et aucune n'en était une — des chaînes de
  format décodées comme du code.

**« Signaux » : ce qui ressort, et où le lire**
- Un panneau de synthèse, en seconde position : listing trompé,
  empaqueteur reconnu, segment inscriptible **et** exécutable, entropie qui monte, protections
  absentes, anti-débogage, code qui peut se déballer, indicateurs dans
  les chaînes. Il **n'analyse rien de neuf** — c'est ce que les autres
  panneaux ont déjà trouvé, rassemblé.
- Chaque ligne dit un **fait** vérifiable (une adresse, un nom, une
  mesure), **ce que ce fait change**, et **le panneau** qui l'explique.
- **Rien n'est additionné en score.** Un score donnerait l'illusion d'un
  verdict ; aucune de ces lignes n'est une preuve, et c'est le lecteur
  qui conclut.
- Quand rien ne ressort, le panneau le dit — et dit aussi que cela ne
  veut pas dire inoffensif, seulement que ce binaire ne porte aucune des
  marques que namsdec sait reconnaître.

**Ce que le binaire sait faire, en six familles**
- Les fonctions importées et les appels système sont désormais **rangés**
  autant que listés : réseau, exécution, anti-débogage, mémoire,
  chiffrement, fichiers. Trois familles allumées sont déjà un portrait.
- La même table sert aux deux panneaux, avec les mêmes couleurs : un
  `socket` est un `socket` qu'il arrive par la table de liaison ou par le
  numéro 41 dans RAX — et un programme qui veut passer inaperçu emploie
  justement le second.

**Les indicateurs, tirés des chaînes**
- Adresses web, adresses IP, noms de domaine, courriels, chemins absolus,
  clés de registre : les seules choses d'un binaire qu'on puisse aller
  vérifier **ailleurs**, dans un journal réseau ou sur un autre poste.
  Chaque rubrique se copie d'un clic, une par ligne.
- La règle est écrite pour **se taire plutôt que se tromper** : une liste
  de domaines de premier niveau connus, et non « tout ce qui contient un
  point » — sans quoi `libc.so.6` passerait pour un nom de domaine.

**L'empaqueteur nommé, avec sa preuve**
- UPX, PyInstaller, AppImage se reconnaissent à un nom de section ou à un
  nombre magique. Le panneau Entropie les nomme, **dit où est la marque**,
  et dit quoi en faire : `upx -d` rend l'original, `pyinstxtractor` sort
  l'archive. « Compressé, chiffré ou empaqueté » est juste et frustrant.

**Le rapport porte tout cela**
- Signaux, portrait par familles, famille de chaque import, indicateurs
  rangés par nature. C'est la **même fonction** qui produit les signaux du
  rapport et ceux du panneau.

**Quatre paniques de plus sur ELF hostile**
- L'invariant « aucune entrée ne fait paniquer un analyseur » couvre
  maintenant **tous** les analyseurs et le désassembleur, sur un ELF
  d'épreuve **fabriqué** plutôt que lu sur le disque — le contrôle tourne
  donc partout de la même façon.
- Il a trouvé quatre paniques : deux sommes d'adresses dans
  `imports::file_offset`, deux dans la recherche de place libre, et
  surtout les trois lecteurs d'entiers d'`elf_scan` et d'`imports` —
  `at + 4` déborde dès que `at` vient d'un champ du fichier. Ce n'était
  pas un défaut mais une classe de défauts : quarante additions de
  décalage sont désormais saturantes.

**Modifier le code, en assembleur**
- La fenêtre des correctifs accepte désormais **une instruction** là où
  elle n'acceptait que des octets : `jmp 0x401050` plutôt que
  `e9 f6 00 00 00`. C'est nasm qui assemble, à l'adresse réelle — un
  saut est relatif, et le même texte ne donne pas les mêmes octets à deux
  endroits. Si nasm refuse, c'est son diagnostic qui s'affiche, pas le
  mien.
- Le désassemblage se lit au choix en **syntaxe NASM ou dans celle du
  système** (AT&T, celle de `objdump` et de gdb). Le même octet, deux
  écritures : `mov rax, rbx` et `movq %rbx, %rax`. L'ordre des opérandes
  y est inversé, et c'est la première chose qui déroute en passant d'un
  outil à l'autre.

**L'Atelier : une seconde série d'exercices**
- Dix-huit exercices livrés en greffon, posés par le même
  `./install.sh` : huit au palier Structures, dix au palier Maîtrise.
- Les seconds sortent du calcul entier — quatre additions en une
  instruction (`paddd`), seize octets comparés d'un coup (`pcmpeqb` puis
  `pmovmskb`), un appel système qui écrit vraiment, le débordement signé
  et le non signé qui ne répondent pas à la même question, un produit qui
  ne tient pas dans un registre, un canari de pile monté à la main.
- Les dix-huit **atteignent leur but tels qu'ils sont livrés**, et un
  contrôle l'exécute à chaque fois : il a attrapé deux énoncés faux —
  « Bonjour tout » fait douze caractères, et `0b1011011011` porte sept
  bits allumés.

**Les outils : un mot par question**
- Trois questions se mélangeaient en autant de vocabulaires. Elles sont
  maintenant nommées séparément : le programme est **installé sur la
  machine** ou non, il est **activé dans namsdec** ou non, et le module
  d'installation est activé ou non. « Allumer », « s'en servir » et
  « programme détecté » ont disparu.
- **« Recommencer » est retiré** d'un outil qui répond : c'était une
  offre sans motif — vingt minutes de compilation pour un résultat
  identique —, et elle faisait douter que l'installation ait abouti.
- À la place, **« Réparer »** dans le seul cas où relancer sert : l'outil
  là mais incomplet. C'est rizin sans `rz-ghidra`, où `pdg` n'existe pas
  et où rien ne disait quoi faire.
- La fiche d'un outil absent dit **comment l'obtenir sans rien déplier**.
  Les fiches s'ouvraient d'office quand l'outil manquait : sur une machine
  neuve, où aucun ne l'est, les deux derniers outils passaient sous le
  bord de la fenêtre. C'est la CI qui l'a trouvé — sa machine n'a rien
  d'installé, la mienne a tout.
- **DZF figure dans la liste**, bien que ses fonctions soient encore
  minimales : un outil qu'on développe se reconnaît d'abord à ce qu'il
  est absent, et la fiche dit où le prendre.

**Deux textes qui mentaient**
- Le dialogue intitulé « Entrée du programme » gardait ce titre après la
  réponse, quand il ne montre plus qu'une **sortie**. Il s'intitule
  maintenant « Ce que le programme a écrit ».
- L'onglet Profil dit **combien de lignes sont classées**. Le défilement
  marchait — mesuré —, mais rien ne disait qu'il y avait une suite : on
  ne cherche pas à dérouler ce qu'on croit avoir vu en entier.

**Ce qui rendait la CI rouge**
- Quatre poussées d'affilée ont échoué sur un seul contrôle, et pas sur
  la mise en forme : il **lisait la machine**. Ouvrir la fenêtre
  d'installation jette le relevé des outils par conception, et l'image
  suivante le refait en lançant vraiment rizin — installé ici, absent du
  coureur. Il sème maintenant l'état, après la première image, et couvre
  les trois cas au lieu d'un.

## 0.2.5-beta.11 — 29 septembre 2026

Les sauts rendus à leur forme, un `main.rs` qui a maigri des deux tiers,
et vingt-cinq commentaires qui décrivaient autre chose que ce qu'ils
précédaient.

**Le pseudo-code lit enfin les sauts**
- La fenêtre du déroulement montre, au choix, le graphe ou le
  **pseudo-code structuré** de la fonction. Un `jne` qui enjambe deux
  instructions devient « si égal : … » ; une arête qui remonte devient
  « tant que » ou « répéter … tant que ».
- La distinction entre les deux formes de boucle n'est pas cosmétique :
  l'une peut ne jamais tourner, l'autre passe toujours au moins une
  fois. C'est précisément ce qu'on cherche à savoir devant un binaire
  qu'on n'a pas écrit.
- La condition affichée est celle du **programme**, non celle du saut.
  Un compilateur écrit « si NON condition, sauter par-dessus » ;
  recopier le mnémonique dirait l'inverse de ce que fait le code, et se
  lirait très bien.
- Ce qui ne se reconnaît pas garde son « aller à » et son étiquette. Un
  `si` inventé à la mauvaise place coûte plus cher que le `goto` qu'il
  remplace : on relit un `goto`, on ne relit pas ce qu'on croit avoir
  compris.
- Le choix de la vue se garde dans `settings.json`.
- Les commandes d'échelle sont **grisées** dans cette vue — un texte n'a
  pas d'échelle — et disent pourquoi au survol plutôt que de disparaître.
  **Copier le pseudo-code** prend leur relais : le texte s'emporte comme
  le graphe, indentation comprise.
- Une fonction sans aucun saut le dit en une ligne, au lieu de laisser
  chercher un « si » qui n'y est pas.

**Un binaire qu'on peut alléger de dix pour cent**
- La recherche de mise à jour traîne une pile TLS de 1,6 Mo — autant que
  tout le code de namsdec — pour une fonction employée une fois par
  mois. Elle passe derrière une option de compilation, **activée par
  défaut** : l'application livrée ne change pas.
- `cargo build --release --no-default-features` donne 14,4 Mo au lieu de
  16,0. C'est ce que veut un empaqueteur de distribution.

**Un gros listing ne coûte plus rien par frame**
- Un binaire ouvert fait quinze mille lignes. La clé du cache de mise en
  page était un condensé des six cent mille octets, recalculé deux fois
  par frame — 2,4 ms, un huitième du budget d'une image, dépensés à
  apprendre que rien n'avait changé.
- Le texte lui-même sert de clé : 11 µs au lieu de 1 224, et une égalité
  exacte ne peut pas collisionner là où un condensé le pourrait.

**La provenance traverse la mémoire**
- « D'où vient cette valeur ? » s'arrêtait à la première lecture en
  mémoire — c'est-à-dire exactement là où la question devient
  intéressante. Une valeur rangée sur la pile puis relue se remonte
  maintenant jusqu'à elle : `mov [rbp-8], rdi` en haut, `mov rax,
  [rbp-8]` trente lignes plus bas.
- Deux pistes sont suivies : **quelle** case (les registres de
  l'adresse — `[rsi+8]` avec RSI reçu en argument, c'est le champ +8 de
  l'objet passé) et **quoi** dedans (la dernière écriture).
- La piste **se coupe** devant une écriture qu'on ne sait pas distinguer
  de la nôtre : `mov [rax], rcx` vise peut-être `[rbp-8]`. Une écriture
  voisine dont on prouve qu'elle ne touche pas la nôtre ne coupe rien.

**L'installation se laisse conduire**
- `./install.sh --help` décrit onze options. Les deux qui servent le
  plus : `--dry-run` dit ce qui serait fait sans rien écrire, `--debug`
  installe la version non optimisée, prête en quelques secondes.
- Aussi : `--prefix`, `--bin-dir`, `--no-desktop`, `--no-dock`,
  `--no-path`, `--no-exercises`, `--no-strip`, `--no-build`, `--quiet`,
  `--no-color`. `./uninstall.sh` en reprend cinq.
- Les couleurs s'éteignent hors d'un terminal et quand `NO_COLOR` est
  posé. Une option inconnue est refusée en disant laquelle.

**Le déroulement dit ce que fait la fonction entière**
- Quand « Fonction courante » est coché, une ligne en tête résume ce que
  namsdec sait de la fonction : « Lit RDI et RSI, rend 0 ou 1. »
- Les arguments sont ceux que le bloc d'entrée lit avant de les écrire ;
  la valeur rendue n'est dite que si **tous** les retours posent une
  constante — un seul chemin muet suffit à se taire.
- La ligne ne répète pas le dessin : « contient une boucle » devant un
  graphe où la boucle se voit n'apprend rien, et namsdec préfère ne rien
  dire.

**Un outil extérieur reçoit du code, non un fichier**
- Deux marqueurs de plus dans la fiche d'un désassembleur : `{text}` reçoit
  les octets **exécutables seuls**, `{base}` l'adresse où ils sont chargés.
- Sans eux, un outil qui ne lit pas l'ELF — et c'est le cas du premier
  désassembleur que quiconque écrit — commençait par désassembler
  l'en-tête comme du code, et adressait tout son listing à partir de
  zéro. C'est namsdec qui sait lire l'ELF ; il n'y a pas de raison de le
  faire réapprendre à chaque outil.
- **DZF** en profite sans avoir rien eu à changer : c'est sa fiche qui dit
  ce qu'il veut recevoir. Le `0x401000` qu'elle portait en dur disparaît
  au passage — il n'était juste que pour les binaires produits ici.
- Une fiche qui demande `{text}` alors que namsdec n'a pas su isoler le
  code reçoit le fichier entier, c'est-à-dire ce qu'elle recevait avant.

**Neuf fiches de plus au manuel** (155 → 164)
- Additionner sans déborder (les formes saturantes), multiplier en
  parallèle, multiplier et additionner d'un coup, additionner voisin à
  voisin, élargir chaque élément, charger seize octets mal alignés,
  prévenir le cache, demander ce que ce processeur sait faire, parler à
  l'hyperviseur.
- Chacune dit si namsdec sait l'exécuter. Décrire ce qu'on ne simule pas
  sans le dire serait une promesse en l'air.

**Ce qui ne se voit pas, et qui compte**
- `main.rs` passe de 20 641 à 6 440 lignes : dix-neuf modules en sont
  sortis, le texte déplacé sans une virgule de différence.
- Vingt-cinq commentaires de documentation décrivaient autre chose que
  ce qu'ils précédaient — Rust laisse deux blocs de `///` se suivre, et
  rien ne le signalait. Six sont revenus à leur fonction, cinq étaient
  d'anciennes rédactions déjà remplacées.
- Le bouton « Écrire » du panneau mémoire est encadré comme les autres :
  il était indiscernable d'une étiquette tant qu'on ne le survolait pas.
- L'intégration continue passe pour la première fois.

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

**Les points d'arrêt savent regarder dans la mémoire**
- Les deux côtés d'une condition sont désormais des expressions de la
  **calculatrice** : parenthèses, opérateurs binaires, dix-huit
  fonctions. `(rax & 0xff) == 'A'` s'écrit tel quel.
- Et la calculatrice a gagné la **lecture mémoire**, entre crochets et
  avec la grammaire de nasm : `[rsp+8]`, `byte [rdi]`,
  `dword [rbx + rcx*4]`. C'est la condition qu'on veut devant un
  contrôle de mot de passe — l'intéressant n'est pas dans un registre,
  il est à l'adresse qu'un registre désigne.
- La lecture est un coup d'œil, non une instruction : elle ne déclenche
  aucune surveillance et ne compte pas comme un accès du programme.
  Regarder ne doit pas changer ce qu'on regarde.
- Le champ ne refuse que ce qui est **mal écrit**, non ce qu'il ne sait
  pas encore calculer. Une condition s'écrit avant de lancer — c'est
  même le seul moment où l'on en écrit une —, et à ce moment-là il n'y a
  ni registre ni mémoire à lire.

**La calculatrice, comme celle d'un programmeur**
- L'expression en haut, le résultat en grand, les **soixante-quatre
  bits** dessous — groupés par quartets, avec leurs index —, puis un
  clavier. Cliquer un bit le bascule, et l'expression devient la
  nouvelle valeur.
- Le clavier écrit **ce que l'évaluateur lit** : `&`, `|`, `^`, `<<`, et
  non des symboles mathématiques qu'il faudrait retraduire pour les
  recopier dans du code.
- La **base** gouverne les deux sens : le résultat est écrit dedans, et
  un chiffre tapé y entre avec son préfixe — en hexadécimal, `1` puis
  `F` donnent `0x1F`. Les chiffres qui n'existent pas dans la base sont
  éteints ; un clavier qui propose `9` en binaire ment sur ce qu'on peut
  taper. La **largeur** (8, 16, 32, 64) borne la lecture, non le calcul.
- Les dix-huit fonctions sont en boutons, et **survoler l'une d'elles
  écrit ce qu'elle fait** juste en dessous. L'infobulle seule ne
  suffisait pas : elle se mérite après une demi-seconde d'immobilité,
  alors qu'on parcourt dix-huit noms des yeux.
- Base et largeur sont des réglages : on les retrouve au retour.
- **Les touches sont de vrais boutons**, avec leur fond, leur liseré et
  leur couleur de famille : gris pour les chiffres, bleu pour les
  opérateurs, rouge pour ce qui efface. Au repos, le thème rend le fond
  des boutons transparent et le liseré invisible — le clavier n'était
  qu'une trentaine de caractères posés dans du vide, qu'on ne pensait
  même pas à cliquer.
- Les touches sont peintes en couleurs **opaques et franches** : les
  chiffres sur un gris net, les opérateurs sur la couleur d'accent, ce
  qui efface sur du rouge. Des mélanges timides donnaient six nuances du
  même bleu sombre, où il fallait lire chaque touche pour la trouver.
  L'encre est celle des deux couleurs du thème qui contraste le plus
  avec le fond — la règle vaut pour un thème clair comme pour un sombre.
  (`gamma_multiply` ne touche que l'alpha, et une touche à vingt pour
  cent sur un panneau sombre est une touche sombre.)
- La fenêtre **épouse la hauteur de son contenu**. Une hauteur figée
  laissait cent pixels de vide entre la dernière ligne et le bouton : un
  appareil ne se termine pas par du vide.
- **Aucun filet horizontal** dans les fenêtres : ni sous le titre, ni
  au-dessus du bouton. Une fenêtre qui en porte deux se lit comme un
  formulaire — trois bandes empilées —, et c'est l'espace qui sépare
  aussi bien. Un test les compte, et exige zéro.
- Les sept lectures du résultat sont rangées **deux par rangée** : elles
  tiennent en quatre lignes au lieu de sept, et l'on compare d'autant
  mieux deux bases qu'on les voit ensemble. Empilées, les dernières
  passaient sous le bouton sur un écran de portable.
- Les filets internes du clavier sont partis, et les espacements
  resserrés.
  Le clavier et les fonctions se distinguent déjà par leur forme ; une
  barre de plus dans une fenêtre qui en a déjà deux fait un formulaire. Elles s'éclaircissent au survol et
  s'assombrissent sous le clic — un bouton auquel on impose un fond
  garde le même dans tous les états, et le clavier avait l'air d'une
  image.
- L'**écran** — l'expression, le résultat et les bits — a son cadre, sur
  le fond de l'éditeur : ce que l'appareil montre se distingue de ce sur
  quoi on appuie. Les deux blocs ont la même largeur au pixel.
- **On tape dans l'écran**, sur sa première ligne et en dix-neuf points,
  sans cadre ni étiquette. Un champ encadré posé au-dessus avec son
  libellé « Expression » faisait un formulaire ; sur une calculatrice,
  la ligne du haut de l'écran *est* le champ.
- Les **dix opérateurs et les dix-huit fonctions** sont désormais
  croisés avec la sémantique de Rust sur seize valeurs limites — zéro,
  un, les puissances de deux, les bords de chaque largeur, `u64::MAX` :
  deux mille cinq cents comparaisons pour les opérateurs, autant pour
  les fonctions, et quatorze expressions confrontées à leur forme
  entièrement parenthésée. Une priorité inversée donne des résultats
  *plausibles*, ce qui la rend dangereuse : `1 | 2 & 3` vaut 3 dans un
  sens et 2 dans l'autre, et rien à l'écran ne dit lequel on a obtenu.
- La fenêtre est bornée en largeur. La rangée des dix-huit fonctions la
  poussait à douze cents pixels, où un clavier de trois cent
  soixante-six flottait au milieu du vide.
- **Tout est centré** : le résultat, la grille de bits, les sélecteurs et
  le clavier. `vertical_centered` ne centre que les enfants de largeur
  connue, et une rangée horizontale prend toute la largeur : elle ne se
  centrait donc jamais. Les largeurs sont maintenant dites.

**L'empreinte du fichier**
- Un module de plus : la taille exacte, le **SHA-256** et le **CRC-32**
  du fichier entier, comme `sha256sum` les donne. C'est la première
  ligne d'un compte rendu — de quel octet parle-t-on ? —, et les deux se
  copient d'un clic.
- Écrites à la main, sans dépendance : SHA-256 tient en soixante lignes,
  CRC-32 en quinze, et les vecteurs d'essai publics disent si c'est
  juste. Ils l'ont dit : le rembourrage s'arrêtait un bloc trop tôt
  quand le message faisait de cinquante-six à soixante-trois octets.
  Les dix longueurs autour de la bascule sont maintenant gravées dans un
  test.
- Ni MD5 ni SHA-1 : ce sont des empreintes cassées, et les afficher à
  côté d'une bonne sans le dire apprendrait le faux.
- Le rapport Markdown la porte aussi, en tête : « de quel fichier
  parle-t-on ? » est la question qu'un compte rendu doit trancher en
  premier.

**Patcher un binaire**
- Outils ▸ *Patcher le binaire…*, sur la ligne du curseur. **Inverser ce
  saut** — `je` devient `jne`, le programme prend l'autre branche — et
  **neutraliser** — l'instruction devient des `nop`, un appel de
  vérification disparaît.
- Les deux gardent la **longueur exacte** : un octet de plus ou de moins
  décalerait tout ce qui suit, et chaque adresse du fichier deviendrait
  fausse.
- La fenêtre écrit les octets d'avant et ceux d'après **avant** qu'on
  clique — on ne change pas les octets de quelqu'un à l'aveugle — et
  chaque changement se défait.
- L'inversion tient en un bit, et c'est le processeur qui l'a voulu
  ainsi : les conditions vont par paires, et le bit de poids faible de
  l'opcode dit laquelle des deux. Aucune table à tenir à jour, et
  l'inverse de l'inverse retombe sur l'original.
- Le module **Vidage du fichier** laisse écrire : on choisit la tranche,
  on tape les octets, et le changement rejoint la liste des patchs —
  donc il se défait, et l'export le porte. Un module ne connaît ni le
  disque ni l'historique des changements : il décrit ce qu'il voudrait,
  l'application relève et décide.
- Trouvé en chemin : la conversion adresse → position dans le fichier ne
  regardait que les **sections**, donc rien sur un binaire dépouillé —
  précisément le cas des crackmes. Elle se rabat sur les segments, que
  le noyau lit toujours.

**Où est quoi, et ce que ces octets-là sont**
- **Carte du fichier** : les segments, ce que le noyau charge, avec
  leurs droits — un `rwx` se voit en rouge — ; et les sections, ce que
  l'éditeur de liens a rangé, avec leur place dans le fichier, leur
  adresse et leur entropie. `.bss` n'a pas d'entropie et le dit : elle
  n'occupe rien dans le fichier, et mesurer là reviendrait à raconter le
  voisin.
- **Vidage du fichier** : les octets tels qu'ils sont sur le disque —
  l'onglet Mémoire montre ceux du programme qui tourne, ce n'est pas la
  même chose — et, pour la tranche qu'on choisit, sa taille, son
  SHA-256, son CRC-32 et son entropie. Cliquer un octet pose le début.
  Seules les lignes visibles sont disposées : un fichier de trois cents
  kilo-octets fait vingt mille lignes.

**La fenêtre d'analyse se lit en largeur**
- Une colonne à gauche nomme les modules, celui qu'on choisit prend tout
  le reste. Les sept empilés n'employaient que la hauteur : on relisait
  une page de deux mille pixels dans une fenêtre large de mille, en
  cherchant un titre à l'ascenseur.
- « Tout » les remet à la file, qui est la forme d'un compte rendu —
  c'est cette vue-là qu'on copie. Le module choisi est un réglage.
- Les chaînes d'un binaire s'alignent à gauche, quelle que soit leur
  longueur. Elles flottaient au milieu du panneau, chacune à une
  abscisse différente : `add_sized` centre son contenu, et dire à
  l'étiquette de s'aligner à gauche ne suffisait pas à l'en empêcher. On
  cherchait chaque ligne au lieu de descendre la colonne.
- Le contenu ne s'étire plus au-delà d'une largeur de lecture. Sur un
  écran large, une table à deux colonnes posait la seconde à mille
  pixels de la première, et l'on suivait la ligne du doigt.
- Le **vidage s'enregistre** au format de `xxd` — celui qu'un `diff` sait
  lire —, et se copie d'un bouton. Un vidage sert à être comparé : à
  celui qu'on a sous les yeux, à celui d'un autre outil, à la même
  analyse faite la semaine dernière.
- La colonne est **alignée à gauche** et les deux zones ont leurs
  marges. Centrés, treize libellés de longueurs différentes n'ont plus
  de bord commun : l'œil cherche chacun au lieu de descendre une liste.
  Et un tableau collé au trait de séparation donne l'impression d'être
  coupé.

**D'où vient ce que ce registre contient ?**
- Survoler un registre rend la **chaîne** des écritures, et non le seul
  dernier pas : RAX vient de ce pas, qui lisait RCX, lequel vient de
  celui-là. Jusqu'à quatre maillons, chacun avec son rang et sa ligne.
  On le faisait à la main, un survol après l'autre.
- Elle s'appuie sur l'**historique** et non sur le code : celui-ci dit
  ce que le programme *pourrait* avoir fait, celui-là ce qu'il a fait.
  Devant plusieurs chemins qui mènent au même endroit, seul l'historique
  sait lequel a été pris.
- Elle suit l'ingrédient **nouveau** : dans `add rax, rcx`, RAX est
  l'accumulateur et RCX ce qu'on ajoute — suivre l'accumulateur ne
  ferait que redescendre la même somme d'un cran.
- Et elle **s'arrête à la mémoire en le disant**. Une valeur qui vient
  d'un `[rbp-8]` ne se remonte pas par son registre de base : cela
  mènerait à la mise en place du cadre de pile, pas à la valeur. Mieux
  vaut s'arrêter en le disant que de conduire ailleurs avec assurance.

**Chercher dans la mémoire**
- Un champ dans l'onglet Mémoire, et trois écritures qui se mélangent
  dans la même case : `"pass"` pour un texte, `41 42 43` ou `414243`
  pour des octets, `??` pour « n'importe lequel ». `"pass" 00` cherche
  le mot suivi de son zéro final, `48 89 ?? 45` une instruction dont un
  octet varie.
- Le panneau dit combien d'octets le motif mesure **avant** de chercher :
  c'est là qu'on s'aperçoit d'un chiffre oublié.
- Les adresses trouvées sont cliquables, et la liste s'arrête à
  soixante-quatre en le disant. Un motif d'un seul octet dans un
  mégaoctet trouve des milliers de réponses, et une liste tronquée qui
  se tait fait conclure qu'il n'y en a pas davantage.

**Une boîte pour répondre au programme**
- Quand le programme se met à attendre, une boîte s'ouvre d'elle-même,
  au premier plan, le curseur déjà dans le champ. Elle montre **ce que
  le programme vient d'écrire** — neuf fois sur dix c'est l'invite, et
  c'est la seule chose qui dise ce qu'on attend de nous.
- Entrée envoie, et la boîte **reste ouverte** : elle montre alors ce que
  le programme a répondu — « Incorrect password » — avant de s'arrêter.
  La refermer aussitôt renvoyait devant une console souvent repliée, où
  l'on ne voyait rien de ce qu'on venait de provoquer, et l'on croyait
  qu'il ne s'était rien passé. Échap ou OK la ferment ; relancer aussi,
  puisqu'elle parlerait de la course précédente.
- « Fin d'entrée » ferme l'entrée du programme. La ligne de la console
  reste, pour qui préfère.
- Elle sert précisément quand la console est repliée ou couverte par une
  fenêtre d'analyse : on restait sinon devant un programme qui ne répond
  plus, sans savoir qu'il attendait *nous*.
- Son champ est **distinct** de celui de la console : deux widgets qui
  écrivaient la même chaîne se disputaient le clavier, et ce qu'on
  tapait dans l'un disparaissait de l'autre.

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

**« Base » et « Largeur » se lisent comme deux jumeaux**
- Les deux listes prenaient la largeur de ce qu'elles affichaient :
  « Binaire » et « 8 bits » donnaient deux boîtes de tailles
  différentes, et deux flèches qui ne tombaient pas au même endroit.
- Les largeurs sont maintenant mesurées sur les textes les plus longs —
  « Hexadécimal », « Largeur » — et non écrites en dur : un chiffre posé
  à la main alignerait le français en désalignant l'anglais.
- Trois pièges d'egui traversés en chemin, et chacun vaut d'être su :
  une rangée horizontale centre chaque élément sur la hauteur qu'elle a
  *à cet instant* et ne recale pas ce qui est déjà posé, d'où « Base »
  deux pixels au-dessus de « Largeur » ; une case allouée avance sur ce
  qu'elle **contient**, non sur la taille demandée, d'où une colonne
  d'étiquettes qui ne réservait rien ; et une liste prend la plus grande
  de sa largeur demandée et de celle de son texte, d'où six pixels
  d'écart quand la marge devinée est trop courte de quatre.
- Le contrôle mesure les deux étiquettes, les deux valeurs et les deux
  flèches, dans les deux langues.

**L'état du processeur défilait**
- Une **barre de défilement** coupait les registres sur une fenêtre de
  1280×820 — la taille par défaut. C'est ce qu'on regarde entre deux
  pas : devoir le faire défiler pour voir R15 ou ZF coupe la lecture.
- La cause : l'état du processeur partageait sa zone défilante avec la
  prochaine instruction, laquelle est **au-dessus** — et une zone qui
  défile coupe par le bas. Le compte ne tombait juste que sur une
  fenêtre haute.
- L'état du processeur est maintenant ancré au bas du bandeau, hors de
  toute zone défilante : il a toujours sa hauteur entière. Ce qui cède,
  quand la fenêtre est courte, c'est la prochaine instruction — qui se
  replie d'un clic, contrairement aux registres.
- Le contrôle d'alors regardait les **colonnes**, pas la barre. Le
  nouveau mesure ce que chaque nom de registre laisse voir, à quatre
  tailles de fenêtre, et refuse le moindre pixel rogné. Vu à l'écran
  avant d'être mesuré : c'est la première fois de cette série qu'une
  capture d'écran trouve ce que les tests laissaient passer.

**Dix-sept boutons qu'on ne voyait pas**
- Le thème rend les boutons **plats au repos** — fond transparent,
  liseré absent — et c'est voulu : sans cela la barre de menus et la
  barre d'outils se ressemblent trop. Mais la règle valait pour *tous*
  les boutons, y compris ceux des fenêtres et des boîtes de dialogue,
  qui ne se distinguaient donc d'une étiquette qu'au survol.
- Dix-sept l'étaient : « Recharger » et « Garder » après une
  modification sur le disque, « Copier le rapport », « Envoyer » de la
  console, « Tout » du désassemblage, « Lancer », les flèches de la
  recherche, les croix de fermeture des champs… Tous encadrés.
- Les vingt-quatre des menus restent plats : là, la platitude est le
  dessin. Un contrôle nomme l'exception et interdit les autres.

**La palette tient sa promesse**
- Un test exige qu'elle propose **les trente actions**, et qu'aucune
  entrée n'ait de libellé vide. Le code le promettait déjà en
  commentaire — « une commande que la palette ne propose pas n'existe
  que pour qui connaît déjà son raccourci » — sans que rien ne
  l'impose. Passé du premier coup ; une action ajoutée demain n'y
  entrerait pas d'elle-même.

**namsdec relançait `rizin` soixante fois par seconde**
- Savoir si un outil est là **et complet** demande de le lancer :
  `rizin` sans `rz-ghidra` a le programme et pas la commande, et namsdec
  le lui demande plutôt que de le supposer. Cette réponse était
  redemandée **à chaque image**, pour chaque outil connu — ouvrir
  Préférences › Outils saturait donc un cœur et faisait hésiter
  l'interface.
- Le relevé est maintenant gardé. Il est refait quand quelque chose a pu
  changer : une installation qui s'achève, la fenêtre qui s'ouvre (on a
  pu installer l'outil à la main dans un terminal), ou le bouton
  **Revérifier**, qui est la porte de sortie explicite.
- Trouvé par un **test instable** : il échouait au hasard dans la suite
  complète et jamais seul, parce qu'il posait la question une fois et
  que la fenêtre la reposait en se dessinant. Un test qui échoue au
  hasard apprend à ignorer la suite ; celui-ci disait quelque chose.

**Le bandeau ne coupe plus de ligne en deux**
- Sur une fenêtre de 1280×820 — la taille par défaut — avec la console
  ouverte, la zone défilante du bandeau tranchait « EAX = 0 » au milieu.
  Une zone qui défile se coupe, et c'est son métier ; mais dans une
  liste, où l'on fait défiler exprès. Dans le bandeau du haut, une
  demi-ligne ne se lit pas comme un choix.
- Deux changements, et il fallait les deux. **La ligne d'octets passe
  sous le repli « Détail »** : elle découpe l'instruction par champs —
  préfixe, opcode, ModR/M — ce qu'aucun désassembleur ne montre, mais on
  la regarde en s'arrêtant, pas à chaque pas. Elle libère 24 px. Et le
  **plancher de l'onglet passe de 220 à 180** : les deux ensemble
  suffisent, aucun des deux seul.
- Mesuré plutôt que deviné : en faisant varier le plancher de 220 à 150
  sans toucher au reste, la coupe ne disparaissait jamais — elle se
  déplaçait, de « EAX = 0 » à la ligne d'octets puis à « Détail ».
- Un contrôle interdit désormais au bandeau de couper une ligne en deux,
  à trois tailles de fenêtre et pour les deux sortes de séance — un
  source qu'on assemble, un binaire qu'on ouvre. Il ne regarde que ce
  qui est au-dessus de l'onglet : en dessous, la pile et la mémoire sont
  des listes, et leur dernière ligne a le droit d'être coupée.
- Et **« Prochaine instruction » se replie sur un binaire**, pas sur un
  source. Une instruction de binaire porte deux lignes de plus —
  adresse absolue et pseudo-code — et à 1280×820 avec la console, le
  bandeau n'avait plus la place de finir : la coupe ne se réglait par
  aucun partage, elle se déplaçait. Repliée, la section laisse son
  titre, qu'un clic rouvre — et qui se rouvre de lui-même devant un
  appel, un saut conditionnel ou un appel système. Devant un binaire on
  lit d'abord le listing et les registres ; devant son propre
  programme, l'instruction qui vient.

**La même phrase trois fois sur un écran**
- Au démarrage, « Aucun binaire assemblé. » s'affichait **trois fois en
  même temps** : le bandeau des registres, l'onglet du panneau, et la
  console. La première dit ce qui manque et porte le bouton ; les deux
  autres ne faisaient que la répéter.
- La console dit maintenant ce qu'elle montrera — « la sortie du
  programme s'affichera ici : ce qu'il écrit, et ce que vous lui
  répondez » — et la fenêtre des registres étendus de même.
- Un contrôle interdit à une phrase de plus de vingt caractères d'être
  écrite trois fois sur le même écran. Deux fois passe : un constat et
  son écho là où l'on regarde. Trois est du remplissage.

**Un panneau vide dit quoi faire**
- Six panneaux du bandeau latéral disaient « Aucun binaire assemblé. »
  et s'arrêtaient là — un constat, pas une porte. Or c'est exactement
  l'écran qu'on a sous les yeux la première fois qu'on ouvre namsdec.
- Le bandeau des registres porte maintenant **le bouton** qui lance
  l'assemblage, avec la phrase qui dit ce que l'assemblage montrera.
  Un seul bouton : le répéter dans chaque onglet en mettrait deux
  identiques à l'écran en même temps.
- Et **chaque onglet dit ce qu'il affichera** — la pile et la chaîne des
  appels, les pages de mémoire et leurs droits, les blocs alloués, les
  octets de chaque ligne, les analyses du binaire, les instructions
  telles que le processeur les lit. Un panneau vide qui ne dit pas à
  quoi il sert n'apprend rien à qui le découvre.
- La règle existait déjà dans le code, écrite dans l'explorateur de
  fichiers — « un panneau simplement vide se lit comme cassé » — et
  n'était appliquée que là.

**« Coller » collait par-dessus tout le programme**
- L'entrée du menu Édition remplaçait le **programme entier** par le
  presse-papier. Elle s'appelait « Coller ». On cherchait à recopier
  trois lignes d'un cours, on perdait son travail, et rien ne prévenait
  — pas même un intitulé qui l'aurait laissé deviner.
- « Coller » colle maintenant **à l'endroit du curseur**, en remplaçant
  la sélection s'il y en a une, comme partout ailleurs.
- Le remplacement total existe toujours, sous son propre nom :
  **Remplacer tout le code…**. C'est le geste de qui reprend un
  programme entier d'un cours ; ce qui ne servait personne, c'est qu'il
  se cache derrière le mot « Coller ».
- Les deux refusent sur un listing venu d'un binaire, comme la
  duplication de ligne : y insérer du texte décalerait les
  correspondances ligne ↔ adresse dont dépendent la gouttière, les
  points d'arrêt et le suivi de RIP.

**Les icônes, regardées à la taille qu'elles ont vraiment**
- **L'icône de la calculatrice ne ressemblait à rien**, et c'était
  exact : sur un bouton de dix-huit pixels, cinq de marge n'en
  laissaient que huit au dessin, où se superposaient deux rectangles
  tracés au trait de 1,2 px. Le contour du boîtier en mangeait deux et
  demi, l'écran intérieur se remplissait de son propre contour, et les
  quatre touches débordaient. Redessinée : un boîtier, un écran **plein**,
  et six touches en trois colonnes — la grille est ce qui fait
  reconnaître une calculatrice.
- **L'icône d'analyse** superposait la clé et l'engrenage de Desdec : la
  clé tenait dans le moyeu, ce qui donnait une rayure en travers d'un
  rouage. Les deux sont maintenant croisés, chacun sur sa diagonale et
  avec la moitié de la place.
- Un test interdit à une icône de **se peindre en bouillie**, aux trois
  tailles auxquelles elles sont réellement posées : un rectangle tracé
  garde un intérieur au moins aussi épais que son trait, et deux points
  de même taille ne se touchent pas. C'est en petit que les dessins se
  cassent, et c'est en petit qu'on ne les regarde jamais.

**Forcer un saut**
- Le troisième geste de la fenêtre des patchs, qui manquait : `je`
  devient `jmp`, et la branche est prise quoi qu'il arrive. Inverser et
  forcer ne répondent pas à la même question — inverser demande « et si
  la condition était l'autre ? », forcer « et si on y allait de toute
  façon ? ». C'est le second qu'on veut devant un contrôle qu'on n'a pas
  encore compris.
- La longueur est gardée, comme pour les deux autres. Le saut long y
  gagne un `nop` : `jmp` long fait un octet de moins que `0f 8x`, et le
  déplacement est corrigé d'autant — sans quoi le saut arriverait un
  octet trop tôt.

**Ce qui était écrit et jamais relu**
- **Les points d'arrêt d'un fichier source revenaient de nulle part.**
  Ils étaient écrits dans `settings.json` depuis toujours, et relus
  seulement quand la séance portait sur un *binaire* : celui qui écrit
  son propre programme — le cas ordinaire — retrouvait son fichier, ses
  onglets, ses arguments, et pas un seul de ses points d'arrêt. Les
  zones surveillées et les registres suivis non plus.
- Ils ne reviennent que si le fichier n'a pas bougé : les réglages
  gardent maintenant sa date, comme ils gardaient celle du binaire. Un
  fichier réécrit ailleurs entre deux séances ferait désigner d'autres
  lignes, et la barre d'état le dit au lieu de les laisser disparaître
  en silence.
- Un test compare les deux structures **champ par champ** : un réglage
  ajouté demain sans être relu le fera échouer sans qu'on ait pensé à lui.

**Ce qui manquait pour s'en servir vraiment**
- **`keymap.json` est surveillé.** Modifié pendant que namsdec tourne, il
  était repris au lancement suivant et rien ne le disait : on éditait, on
  essayait la touche, il ne se passait rien. Il est maintenant repris dans
  la seconde. Une capture de raccourci en cours suspend la reprise.
- **Le jeu de raccourcis s'exporte en entier**, à plat : les trente
  actions écrites une par une. Le fichier de réglages ne garde que les
  écarts — c'est ce qu'on veut d'un réglage — mais un fichier de deux
  lignes ne dit pas ce que namsdec sait faire.
- **Le graphe s'enregistre dans un fichier**, et pas seulement dans le
  presse-papier, qui ne survit pas à la copie suivante. C'est le graphe
  qu'on regarde qui part : la fonction seule ou le programme entier,
  selon la case cochée.

**Un but manqué dit ce qui manque**
- Le bandeau des registres écrit maintenant **une ligne par condition**
  du but, avec un point plein pour ce qui tient et un cercle vide pour
  ce qui manque. « Pas encore » tout seul est un verdict, non une
  explication : sur un but à plusieurs conditions, on ne savait pas
  laquelle regarder.
- Et sous la condition qui manque, **ce que l'expression vaut à la
  place** — `rax == 0xf` suivi de `rax vaut 0xc`. C'est le geste qu'on
  vient apprendre : comparer ce qu'on voulait et ce qu'on a.
- Rien n'est écrit sous une condition qui tient (il n'y a rien à
  expliquer), ni sous une condition qu'on ne sait pas encore calculer —
  une adresse hors des pages chargées ne se devine pas, et une valeur
  inventée serait pire que le silence.
- Comment y arriver, namsdec continue de se taire. Corriger à la place
  de l'utilisateur lui retirerait ce qu'il est venu apprendre.

**Réparé**
- Les fenêtres **montaient toutes seules** — et d'autant plus vite qu'on
  remuait la souris, puisque chaque mouvement vaut une image de plus.
  La barre « OK » en était la cause : sa rangée de boutons, posée en
  `Align::Center`, prenait toute la hauteur restante du panneau bas,
  laquelle était justement mesurée sur cette rangée. Chaque image lui
  rendait la hauteur lue, augmentée des marges qui l'entourent, et la
  fenêtre d'analyse perdait cinq pixels de contenu par image jusqu'à
  n'avoir plus rien à montrer. La rangée reçoit maintenant une hauteur,
  et ne lit plus rien de son parent.
- La calculatrice écrivait « Survolez une fonction… » **à travers son
  bouton OK**. Un panneau bas s'ancre au bas du cadre de la fenêtre ;
  quand ce cadre est mesuré sur le contenu — c'est le cas d'une fenêtre
  sans hauteur imposée —, les deux se poursuivent jusqu'à un équilibre
  où la dernière ligne passe sous le bouton. Ce n'était pas un réglage
  à ajuster mais la géométrie du montage : la barre d'action est
  maintenant posée dans le flux, en dernier.
- Un test demande qu'**une fenêtre qu'on ne touche pas ne bouge pas** :
  quatorze fenêtres, une douzaine d'images chacune, et les six
  dernières doivent être identiques. C'est la règle la plus simple
  d'une interface en mode immédiat, et justement celle qu'elle ne
  garantit pas.
- **La pastille du mode** (NORMAL / INSERTION) portait un noir écrit en
  dur, « pour le contraste ». Il l'avait sur le bleu vif d'un thème
  sombre, et pas du tout sur le bleu profond du thème clair : 2,9, où le
  blanc en donne 6,3. L'encre est maintenant choisie d'après le fond.
- **Tokyo Night** écrivait son gris-bleu `#565f89` — le nom de l'onglet,
  les libellés secondaires, les commentaires du programme — à 2,35 de
  contraste. Éclairci, à un écart assumé d'avec le thème d'origine : un
  commentaire qui explique et qu'on doit deviner ne sert à rien.
- **Le texte sélectionné se lisait mal** — 2,07 de contraste, c'est-à-dire
  du gris clair sur un bleu vif. Le thème employait `focusBorder` comme
  couleur de sélection ; VS Code Dark+ emploie `editor.selectionBackground`,
  un bleu sombre, qui en donne 5,7. Corrigé de même pour Light+ (2,65 → 11,0).
- **Les touches de la calculatrice portaient l'encre la moins lisible des
  deux.** `encre_sur` comparait des clartés brutes, ce qui revient à
  mesurer ce que l'écran reçoit et non ce que l'œil en fait ; elle compare
  maintenant de vrais contrastes. Les opérateurs passent de 3,09 à 3,64,
  « Effacer tout » de 2,85 à 3,95.
- **Les zéros de la calculatrice**, quand rien n'est encore tapé,
  tombaient à 2,10 : ils étaient obtenus par `gamma_multiply`, qui ne
  touche que la transparence. Un mélange opaque les rend lisibles (3,4)
  sans les rendre bavards.
- **L'indication du champ « retrouver »** du motif cyclique était coupée
  en deux. Une indication s'affiche avant qu'on ait tapé quoi que ce
  soit — c'est le moment où l'on en a besoin. Le champ est plus large.
- Et un **banc de mise en page** : une liste de fenêtres déclarée une
  fois, des invariants qui valent pour toutes. Une fenêtre nouvelle est
  contrôlée sans qu'on y pense ; un invariant nouveau s'applique
  aussitôt aux quatorze. Deux pour l'instant — rien ne bouge tout seul,
  deux textes ne se recouvrent jamais, un texte tronqué se lit encore en
  entier au survol, et tout texte se détache de son fond.
- Chaque scène du banc est regardée **deux fois** : à vide, puis avec un
  programme assemblé et avancé de huit pas. Une application qu'on vient
  d'ouvrir n'affiche presque rien, et les défauts de mise en page
  viennent avec le contenu.
- Le banc ne regarde plus seulement les fenêtres : il passe aussi sur
  **l'application nue**, barre d'outils, panneau latéral et barre
  d'état comprises, et le contraste y est mesuré dans **chacun des
  trois thèmes**. C'est là qu'on passe ses journées, et c'est là que
  vivent les couleurs d'un thème — qu'on ne relit pas en le regardant.
- Le plancher de contraste est **3 pour un texte actif** et **2 pour
  une commande inactive** : un bouton grisé doit être moins lisible,
  c'est ce que « grisé » veut dire, mais il doit rester visible. Au-delà
  de 3, la nuance entre `text` et `dim` est une décision de thème, qui
  ne regarde pas un test.
- Les quatre invariants ont trouvé un défaut réel le jour de leur
  écriture — huit en tout, dont aucun n'était cherché.
- La fenêtre du décompilateur était **illisible** : chaque bloc se
  posait une ligne trop haut et recouvrait le précédent. Une commande de
  cent cinquante caractères — celle du pont DZF — s'allouait la hauteur
  d'une ligne et s'en peignait deux, parce que sans largeur bornée la
  première passe de mise en page dispose sur une largeur infinie. La
  fenêtre a maintenant une borne, et son contenu une largeur de retour à
  la ligne dite explicitement.
- Et la phrase « Assemblez d'abord » est sortie de la rangée de
  boutons : une rangée qui se replie place son dernier élément à la
  ligne suivante sans savoir de combien descendre, et le texte se posait
  sur le bouton.
- Un test interdit désormais à **deux textes d'une même fenêtre de se
  recouvrir**. C'est un défaut qui se voit tout de suite et se
  diagnostique mal ; mesuré, il se dit en une ligne.
- **La ligne de saisie est toujours là** dès qu'un programme est
  assemblé. Elle n'apparaissait que si namsdec avait repéré un `read`
  dans le code — donc jamais pour qui écrit son programme, a posé ses
  chaînes, et veut préparer ce qu'il lira avant d'avoir tapé le `read`.
  Il n'y avait alors nulle part où taper, et rien à l'écran n'expliquait
  pourquoi. Une phrase dit maintenant ce que la saisie deviendra quand
  rien ne la lit encore. Un champ qui n'apparaît qu'à une condition
  invisible n'est pas une économie, c'est une disparition.
- Un message d'état tronqué se lit **en entier au survol**, et la
  troncature est mesurée au lieu d'être devinée : la règle d'avant
  (« plus de quatre-vingt-dix caractères ») ne valait que pour deux
  états sur dix-sept, et la place dépend de la fenêtre, de la police et
  du zoom. Une seule fonction compose la phrase — la barre la peint,
  l'infobulle la répète, et les deux ne peuvent plus diverger.
- `Suggestions::accepted` et `about_window_id` n'étaient employées que
  par les tests, et la compilation release les signalait comme mortes.
  Elles sont marquées `#[cfg(test)]`, ce qui dit ce qui est : un
  avertissement qu'on apprend à ignorer finit par en cacher un vrai.
- Le caractère `←` n'existe pas dans la police d'egui. Le test qui
  vérifie chaque glyphe de l'interface l'a attrapé avant qu'il n'arrive
  à l'écran sous forme de carré vide ; le retour arrière est une icône
  dessinée.
- Une fenêtre qui vient de s'ouvrir passe devant. Le plan d'installation
  s'ouvre depuis les préférences et se posait derrière elles : on
  cliquait « Installer… » et il ne se passait rien de visible. La règle
  vaut pour toutes, et un test vérifie qu'aucune fenêtre ne manque à la
  table — il en a trouvé deux sans identifiant explicite, les
  préférences et le motif cyclique, qui perdaient leur position en
  passant à l'anglais.
- « frame lente » ne se crie plus devant un simple chargement. Ouvrir un
  binaire de soixante mille lignes demande une demi-seconde de mise en
  page, **une fois** ; le message le dit maintenant, au lieu de faire
  chercher une fuite là où il n'y a qu'un fichier qui arrive.
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
