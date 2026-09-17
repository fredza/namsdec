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

**Vérifier ce qu'on a fait**
- Onze exercices portent un but en conditions (`rax == 0xf`), vérifié
  sur les registres : le bandeau dit « atteint » ou « pas encore ». Une
  série installée peut en porter aussi (champ `goal`).

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
  l'échelle est logarithmique, le compte exact vient au survol, et la
  colonne du compteur disparaît avec ses quarante-quatre pixels.
- La fenêtre porte ses trois pastilles au bout de la barre de menus, qui
  fait office de barre de titre : une rangée de moins. Réduire à gauche,
  fermer à droite, comme partout ailleurs sous Linux. Préférences ▸
  Général rend la barre du système à qui la préfère.
- La boîte « À propos » mène quelque part : namsdec.io, le Discord, le
  dépôt.
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
