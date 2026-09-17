# Journal des versions

Les dates sont celles des commits ; le format suit [Keep a
Changelog](https://keepachangelog.com/fr/1.1.0/) d'assez loin — une
entrée par version, et ce qui change pour qui s'en sert.

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

**Ouvrir un vrai binaire**
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
