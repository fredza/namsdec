# Journal des versions

Les dates sont celles des commits ; le format suit [Keep a
Changelog](https://keepachangelog.com/fr/1.1.0/) d'assez loin — une
entrée par version, et ce qui change pour qui s'en sert.

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
- Un onglet **Profil**, dans la console, classe les lignes par temps
  passé dedans : le nombre de passages, la part du total, la ligne
  elle-même, et un clic qui y emmène. La bande de chaleur dit où ça
  chauffe ; le profil dit de combien, et se copie en texte. Quand un
  exercice impose un budget d'instructions, le profil le rappelle et,
  s'il est dépassé, désigne la ligne par où reprendre. Chaque ligne y
  est nommée par son bloc — « boucle: dec rcx » — pour n'avoir pas à
  retourner dans le programme.
- La fenêtre porte ses trois pastilles au bout de la barre de menus, qui
  fait office de barre de titre : une rangée de moins. Réduire à gauche,
  fermer à droite, comme partout ailleurs sous Linux. Préférences ▸
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
