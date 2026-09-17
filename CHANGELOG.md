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
  mise à jour reste un geste.
- Le dépôt sait publier : une étiquette `v…` déclenche la compilation, les
  vérifications et la publication du binaire avec sa somme SHA-256.

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
